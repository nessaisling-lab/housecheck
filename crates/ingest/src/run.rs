use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::geo::{count_within_m, nearest_ada_m, Station};
use crate::sources::{
    bbl_block, bbl_in_query, census_url, complaints_311_query, hpd_bbl, hpd_block_query,
    parse_311_point, parse_census_medians, parse_dob_has_elevator, parse_hpd_violation,
    parse_pluto, pluto_coords, pluto_query,
};

/// Blocking HTTP client with a UA (Socrata throttles anonymous no-UA traffic harder) and a
/// generous timeout for the larger tract/station pulls. If `NYC_APP_TOKEN` is set, it is sent
/// as the Socrata `X-App-Token` header on every request, lifting the anonymous rate limit
/// (harmless on the Census/MTA hosts, which ignore an unknown token).
fn client() -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(token) = std::env::var("NYC_APP_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(token) {
                headers.insert(reqwest::header::HeaderName::from_static("x-app-token"), val);
                println!("Socrata: using NYC_APP_TOKEN (raised rate limit)");
            }
        }
    }
    Client::builder()
        .user_agent("housecheck-ingest/0.1")
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .expect("build reqwest client")
}

/// Borrow the array inside a JSON value, or an empty slice (never panics on a scalar/null).
fn arr(v: &Value) -> &[Value] {
    v.as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

/// GET a whole URL (Census / MTA — no raw spaces to encode) and decode JSON.
fn get_json(c: &Client, url: &str) -> Result<Value> {
    let resp = c.get(url).send().with_context(|| format!("GET {url}"))?;
    let resp = resp
        .error_for_status()
        .with_context(|| format!("bad status for {url}"))?;
    resp.json()
        .with_context(|| format!("decode json from {url}"))
}

/// GET a base URL with query params (Socrata SoQL). `reqwest` percent-encodes the `$where`
/// clause's spaces and `>` — the whole point of sending components, not a baked URL string.
fn get_json_query(c: &Client, base: &str, params: &[(String, String)]) -> Result<Value> {
    let resp = c
        .get(base)
        .query(params)
        .send()
        .with_context(|| format!("GET {base}"))?;
    let resp = resp
        .error_for_status()
        .with_context(|| format!("bad status for {base}"))?;
    resp.json()
        .with_context(|| format!("decode json from {base}"))
}

pub fn run_real(cfg: &Config) -> Result<()> {
    let census_key = std::env::var("CENSUS_API_KEY")
        .context("set CENSUS_API_KEY (https://api.census.gov/data/key_signup.html)")?;
    let c = client();

    // 1. Buildings from PLUTO for the community district.
    let (pluto_base, pluto_params) = pluto_query(cfg.community_district, cfg.limit);
    let pluto = get_json_query(&c, &pluto_base, &pluto_params)?;
    let mut buildings: Vec<model::Building> = arr(&pluto)
        .iter()
        .filter_map(parse_pluto)
        .filter(|b| !b.bbl.is_empty())
        .collect();
    // Coordinates for geo joins, keyed by the same normalized BBL the buildings use.
    let coords: HashMap<String, (f64, f64)> = arr(&pluto)
        .iter()
        .filter_map(|v| pluto_coords(v).map(|(bbl, lat, lon)| (bbl, (lat, lon))))
        .collect();
    println!(
        "PLUTO: {} residential buildings in CD {}",
        buildings.len(),
        cfg.community_district
    );
    let bbl_set: HashSet<String> = buildings.iter().map(|b| b.bbl.clone()).collect();
    let bbls: Vec<String> = buildings.iter().map(|b| b.bbl.clone()).collect();

    // 2. HPD violations. wvxf-dwi5 has no BBL column, so query by borough + tax blocks, then
    //    reconstruct each row's BBL and keep only those matching our building set.
    let boroid = cfg.community_district / 100; // 303 -> Brooklyn (3)
    let blocks: Vec<u32> = {
        let set: HashSet<u32> = buildings
            .iter()
            .filter_map(|b| bbl_block(&b.bbl).map(|(_, blk)| blk))
            .collect();
        set.into_iter().collect()
    };
    let mut violations: HashMap<String, Vec<model::Violation>> = HashMap::new();
    let mut unknown_classes = 0usize;
    for chunk in blocks.chunks(500) {
        let (base, params) = hpd_block_query(boroid, chunk);
        let rows = get_json_query(&c, &base, &params)?;
        for v in arr(&rows) {
            let Some(bbl) = hpd_bbl(v) else { continue };
            if !bbl_set.contains(&bbl) {
                continue; // a neighbor on the same block, not one of our buildings
            }
            if let Some(viol) = parse_hpd_violation(v) {
                if !matches!(viol.class.as_str(), "A" | "B" | "C") {
                    unknown_classes += 1;
                    continue;
                }
                violations.entry(bbl).or_default().push(viol);
            }
        }
    }
    if unknown_classes > 0 {
        println!("note: {unknown_classes} violations had non-A/B/C classes (skipped)");
    }

    // 3. DOB elevators (active passenger elevator on record). DOB `bbl` is a clean column.
    let mut has_elevator: HashMap<String, bool> = HashMap::new();
    for chunk in bbls.chunks(200) {
        let (base, params) = bbl_in_query("e5aq-a4j2", "bbl,device_type,device_status", chunk);
        let rows = get_json_query(&c, &base, &params)?;
        for v in arr(&rows) {
            if parse_dob_has_elevator(v) {
                if let Some(bbl) = v.get("bbl").and_then(|x| x.as_str()) {
                    has_elevator.insert(bbl.to_string(), true);
                }
            }
        }
    }

    // 4. Census tract rent medians for all of Brooklyn (looked up per building by GEOID).
    //    Non-fatal: rent medians only feed /rent-fairness, not the building Health Card, so a
    //    Census outage or invalid key must not abort the whole ingest — warn and continue.
    let medians = match get_json(&c, &census_url(&census_key)) {
        Ok(v) => {
            let m = parse_census_medians(&v);
            println!("Census: {} tract medians loaded", m.len());
            m
        }
        Err(e) => {
            println!("warning: Census rent medians skipped ({e:#}); acs_rent_by_tract left empty");
            HashMap::new()
        }
    };

    // 5. ADA subway stations (all of NYC; nearest ADA-accessible one per building).
    let mta = get_json(
        &c,
        "https://data.ny.gov/resource/39hk-dx4f.json?$limit=1000",
    )?;
    let stations: Vec<Station> = arr(&mta)
        .iter()
        .filter_map(|v| {
            Some(Station {
                lat: v.get("gtfs_latitude")?.as_str()?.trim().parse().ok()?,
                lon: v.get("gtfs_longitude")?.as_str()?.trim().parse().ok()?,
                ada: v
                    .get("ada")
                    .and_then(|x| x.as_str())
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0),
            })
        })
        .collect();

    // 5b. 311 complaints for the nearby-context density (count within 150 m of each building).
    //     Bound the pull to the curated set's lat/long bbox and to recent complaints so a single
    //     request with a tens-of-thousands `$limit` covers the slice. No geocoded buildings
    //     (empty coords) → skip the call rather than fetch all of Brooklyn.
    let points_311: Vec<(f64, f64)> = if coords.is_empty() {
        Vec::new()
    } else {
        let (min_lat, max_lat) = coords
            .values()
            .map(|(lat, _)| *lat)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                (lo.min(x), hi.max(x))
            });
        let (min_lon, max_lon) = coords
            .values()
            .map(|(_, lon)| *lon)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
                (lo.min(x), hi.max(x))
            });
        let (base, params) = complaints_311_query(min_lat, min_lon, max_lat, max_lon, 50_000);
        let rows = get_json_query(&c, &base, &params)?;
        arr(&rows).iter().filter_map(parse_311_point).collect()
    };
    println!("311: {} complaint points loaded", points_311.len());

    // 6. Enrich each building and write everything through the tested `store` inserts.
    let _ = std::fs::remove_file(&cfg.out);
    let conn = store::open_db(&cfg.out)?;
    store::migrate(&conn)?;
    // Scoring recency reads this snapshot year, not the wall clock, so runs are reproducible.
    let snapshot_year: i32 = std::env::var("SNAPSHOT_YEAR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2026);
    store::set_snapshot_year(&conn, snapshot_year)?;

    let mut tracts_written: HashSet<String> = HashSet::new();
    for b in buildings.iter_mut() {
        b.has_elevator = has_elevator.get(&b.bbl).copied().unwrap_or(false);
        if let Some((lat, lon)) = coords.get(&b.bbl) {
            b.near_ada_subway_m = nearest_ada_m(*lat, *lon, &stations).map(|d| d as i32);
            b.complaints_311 = count_within_m(*lat, *lon, &points_311, 150.0) as i32;
        }
        store::upsert_building(&conn, b)?;
        for v in violations.remove(&b.bbl).unwrap_or_default() {
            store::insert_violation(&conn, &b.bbl, &v)?;
        }
        if let Some(m) = medians.get(&b.tract_geoid) {
            if tracts_written.insert(b.tract_geoid.clone()) {
                store::upsert_tract_median(&conn, &b.tract_geoid, *m)?;
            }
        }
    }
    println!("wrote {} buildings to {}", buildings.len(), cfg.out);
    Ok(())
}
