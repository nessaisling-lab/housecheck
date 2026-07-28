use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::geo::{count_within_m, haversine_m, nearest_ada_m, Station};
use crate::sources::{
    bbl_block, bbl_in_query, census_url, complaints_311_query, fetch_rent_stab, hpd_bbl,
    hpd_block_query, parse_311_point, parse_census_medians, parse_dob_has_elevator,
    parse_hpd_violation, parse_pluto, parse_restaurant_grade, pluto_coords, pluto_query,
    restaurant_grades_query,
};

/// Grade of the nearest DOHMH-graded restaurant within `radius_m` metres of (lat, lon), or
/// None. Reuses `geo::haversine_m` for the exact distance. The restaurant list is ordered
/// newest-inspection-first, and ties keep the first-seen row, so a tie prefers the fresher grade.
fn nearest_grade_within(
    lat: f64,
    lon: f64,
    restaurants: &[(String, f64, f64)],
    radius_m: f64,
) -> Option<String> {
    let mut best: Option<(f64, &str)> = None;
    for (grade, rlat, rlon) in restaurants {
        let d = haversine_m(lat, lon, *rlat, *rlon);
        if d <= radius_m && best.map(|(bd, _)| d < bd).unwrap_or(true) {
            best = Some((d, grade.as_str()));
        }
    }
    best.map(|(_, g)| g.to_string())
}

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

    // 1. Buildings from PLUTO — a BLEND so both stories show: ~60% pulled largest-first (the
    //    pre-war, multi-unit, stabilization-eligible buildings) and the rest from the
    //    neighborhood's natural mix of small rowhouses. Merge, dedup by BBL, cap at the limit.
    let big_n = (cfg.limit * 3) / 5;
    let (b1, p1) = pluto_query(cfg.community_district, big_n, Some("unitsres DESC"));
    let (b2, p2) = pluto_query(cfg.community_district, cfg.limit, None);
    let big = get_json_query(&c, &b1, &p1)?;
    let mix = get_json_query(&c, &b2, &p2)?;
    let mut seen: HashSet<String> = HashSet::new();
    let mut buildings: Vec<model::Building> = Vec::new();
    let mut coords: HashMap<String, (f64, f64)> = HashMap::new();
    let mut big_taken = 0usize;
    for (is_big, v) in arr(&big)
        .iter()
        .map(|v| (true, v))
        .chain(arr(&mix).iter().map(|v| (false, v)))
    {
        let Some(b) = parse_pluto(v) else { continue };
        if b.bbl.is_empty() || !seen.insert(b.bbl.clone()) {
            continue;
        }
        if let Some((bbl, lat, lon)) = pluto_coords(v) {
            coords.insert(bbl, (lat, lon));
        }
        if is_big {
            big_taken += 1;
        }
        buildings.push(b);
        if buildings.len() >= cfg.limit as usize {
            break;
        }
    }
    println!(
        "PLUTO: {} residential buildings in CD {} (blend: {} large + {} neighborhood mix)",
        buildings.len(),
        cfg.community_district,
        big_taken,
        buildings.len() - big_taken
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

    // Bounding box around the curated set's coordinates, reused to bound both the 311 pull
    // (below) and the DOHMH restaurant pull (5c). No geocoded buildings → no box → skip both.
    let bbox: Option<(f64, f64, f64, f64)> = if coords.is_empty() {
        None
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
        Some((min_lat, min_lon, max_lat, max_lon))
    };

    // 5b. 311 complaints for the nearby-context density (count within 150 m of each building).
    //     Bound the pull to the curated set's lat/long bbox and to recent complaints so a single
    //     request with a tens-of-thousands `$limit` covers the slice.
    let points_311: Vec<(f64, f64)> = match bbox {
        None => Vec::new(),
        Some((min_lat, min_lon, max_lat, max_lon)) => {
            let (base, params) = complaints_311_query(min_lat, min_lon, max_lat, max_lon, 50_000);
            let rows = get_json_query(&c, &base, &params)?;
            arr(&rows).iter().filter_map(parse_311_point).collect()
        }
    };
    println!("311: {} complaint points loaded", points_311.len());

    // 5c. DOHMH restaurant grades within the same box. Each building gets the grade of the
    //     nearest graded restaurant within ~200 m (neighborhood context, display only — never
    //     folded into any score). Non-fatal: a DOHMH outage must not abort the whole ingest.
    let restaurants: Vec<(String, f64, f64)> = match bbox {
        None => Vec::new(),
        Some((min_lat, min_lon, max_lat, max_lon)) => {
            let (base, params) =
                restaurant_grades_query(min_lat, min_lon, max_lat, max_lon, 20_000);
            match get_json_query(&c, &base, &params) {
                Ok(rows) => arr(&rows)
                    .iter()
                    .filter_map(parse_restaurant_grade)
                    .collect(),
                Err(e) => {
                    println!(
                        "warning: DOHMH restaurant grades skipped ({e:#}); restaurant_grade left null"
                    );
                    Vec::new()
                }
            }
        }
    };
    println!("DOHMH: {} graded restaurants loaded", restaurants.len());

    // 5d. Rent-stabilization unit counts. Source: JustFix.org (nyc-doffer), derived from NYC DOF
    //     Statement of Account records; latest year 2024. Streams the ~25 MB static CSV and keeps
    //     only rows whose BBL is in our curated set. Non-fatal: a download/parse failure must not
    //     abort the whole ingest — warn and continue with an empty map (buildings read "unverified").
    let rent_stab: HashMap<String, i32> = match fetch_rent_stab(&c, &bbl_set) {
        Ok(m) => {
            println!(
                "JustFix nyc-doffer: {} of {} buildings matched in rent-stab CSV",
                m.len(),
                buildings.len()
            );
            m
        }
        Err(e) => {
            println!(
                "warning: rent-stabilization source skipped ({e:#}); rent_stabilized left null"
            );
            HashMap::new()
        }
    };

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
    let mut stabilized_count = 0usize;
    for b in buildings.iter_mut() {
        b.has_elevator = has_elevator.get(&b.bbl).copied().unwrap_or(false);
        if let Some((lat, lon)) = coords.get(&b.bbl) {
            b.latitude = Some(*lat);
            b.longitude = Some(*lon);
            b.near_ada_subway_m = nearest_ada_m(*lat, *lon, &stations).map(|d| d as i32);
            b.complaints_311 = count_within_m(*lat, *lon, &points_311, 150.0) as i32;
            b.restaurant_grade = nearest_grade_within(*lat, *lon, &restaurants, 200.0);
        }
        // Rent-stabilization tri-state from the JustFix map: units>0 → stabilized, 0 → on
        // record with none, absent → left unknown (None) so the card reads "unverified".
        match rent_stab.get(&b.bbl) {
            Some(&units) if units > 0 => {
                b.rent_stabilized = Some(true);
                b.rent_stab_units = Some(units);
                stabilized_count += 1;
            }
            Some(_) => {
                b.rent_stabilized = Some(false);
                b.rent_stab_units = Some(0);
            }
            None => {
                b.rent_stabilized = None;
                b.rent_stab_units = None;
            }
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
    println!(
        "rent-stab: {}/{} buildings stabilized (source: JustFix nyc-doffer 2024)",
        stabilized_count,
        buildings.len()
    );
    println!("wrote {} buildings to {}", buildings.len(), cfg.out);
    Ok(())
}
