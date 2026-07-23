// Pure library of Socrata/Census query builders + JSON parsers consumed by the live
// orchestration in `run::run_real` (Plan 2 Task 5). URL builders return request COMPONENTS
// (a base URL + a list of `(key, value)` query params) rather than a pre-baked full-URL
// string, so the caller hands them to `reqwest`'s `.query()` and lets it percent-encode the
// SoQL `$where` clause (which contains spaces and `>` — invalid raw in a URL).
//
// Field names here reflect the datasets' *actual* live schemas (verified 2026-07-22), which
// differ from the original plan sketch: PLUTO ships `bbl` as a float string
// ("3015990007.00000000") and carries the 2020 tract in `bct2020` (borough digit + 6-digit
// tract), and HPD violations (wvxf-dwi5) have no `bbl` column at all — only `boroid`/`block`/
// `lot`, plus an `Open`/`Close` `violationstatus`. The parsers below normalize all of that.

use anyhow::{Context, Result};
use model::{Building, Violation};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};

const SODA: &str = "https://data.cityofnewyork.us/resource";
const PLUTO_SELECT: &str =
    "bbl,address,yearbuilt,numfloors,unitsres,bldgclass,latitude,longitude,bct2020";

/// A request as (base URL, query param pairs). Pass params straight to `reqwest`'s
/// `RequestBuilder::query`, which URL-encodes keys and values correctly.
pub type Query = (String, Vec<(String, String)>);

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
}
fn num(v: &Value, k: &str) -> i32 {
    s(v, k)
        .and_then(|x| x.trim().parse::<f64>().ok())
        .map(|f| f as i32)
        .unwrap_or(0)
}
/// Parse a numeric field that Socrata may ship as either a JSON string or a JSON number
/// (311's `latitude`/`longitude` come back as numbers in some slices, strings in others).
fn fnum(v: &Value, k: &str) -> Option<f64> {
    match v.get(k) {
        Some(Value::String(s)) => s.trim().parse().ok(),
        Some(Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

/// Normalize a BBL to its canonical 10-digit form. PLUTO ships it as a float string
/// ("3015990007.00000000"); DOB/our own records use the clean integer. Both round-trip
/// through f64 exactly (a BBL is < 2^53). Returns None for a missing/zero/garbage value.
fn norm_bbl(raw: &str) -> Option<String> {
    let n = raw.trim().parse::<f64>().ok()?;
    if n <= 0.0 {
        return None;
    }
    Some((n as u64).to_string())
}

/// PLUTO residential buildings for one Community District. `$where` carries spaces and a
/// `>` operator, so it MUST be sent through `reqwest`'s `.query()` — never interpolated raw.
pub fn pluto_query(cd: u32, limit: u32) -> Query {
    let base = format!("{SODA}/64uk-42ks.json");
    let params = vec![
        ("$select".to_string(), PLUTO_SELECT.to_string()),
        (
            "$where".to_string(),
            format!("borough='BK' AND cd={cd} AND unitsres>0"),
        ),
        // Largest residential buildings first: pre-war multi-unit buildings are the
        // stabilization-eligible ones, so ordering by unit count surfaces real rent-stabilized
        // buildings in the curated slice instead of the small rowhouses a natural-order scan hits.
        ("$order".to_string(), "unitsres DESC".to_string()),
        ("$limit".to_string(), limit.to_string()),
    ];
    (base, params)
}

pub fn parse_pluto(v: &Value) -> Option<Building> {
    let bbl = norm_bbl(&s(v, "bbl")?)?;
    // bct2020 = borough digit + 6-digit 2020 census tract, e.g. "3025300". The tract GEOID
    // is state(36) + county(047 = Kings) + the 6-digit tract (bct2020 minus its boro digit).
    let bct = s(v, "bct2020").unwrap_or_default();
    let tract_geoid = if bct.len() == 7 {
        format!("36047{}", &bct[1..])
    } else {
        String::new()
    };
    Some(Building {
        bbl,
        address: s(v, "address").unwrap_or_default(),
        year_built: num(v, "yearbuilt"),
        num_floors: num(v, "numfloors"),
        units_res: num(v, "unitsres"),
        tract_geoid,
        rent_stabilized: None, // filled later from the JustFix nyc-doffer CSV; None = unknown
        rent_stab_units: None, // filled later from the JustFix nyc-doffer CSV
        good_cause: false,     // filled later
        has_elevator: false,   // filled from DOB
        near_ada_subway_m: None, // filled from MTA
        complaints_311: 0,     // filled from 311 (Task 6)
        latitude: None,        // filled from the PLUTO coords map in run_real
        longitude: None,       // filled from the PLUTO coords map in run_real
        restaurant_grade: None, // filled from DOHMH nearest-graded-restaurant in run_real
    })
}

/// Normalized BBL + coordinates for a PLUTO row, for geo joins keyed by the same BBL the
/// `Building` uses. Returns None if BBL or either coordinate is missing/unparseable.
pub fn pluto_coords(v: &Value) -> Option<(String, f64, f64)> {
    let bbl = norm_bbl(&s(v, "bbl")?)?;
    let lat = s(v, "latitude")?.trim().parse().ok()?;
    let lon = s(v, "longitude")?.trim().parse().ok()?;
    Some((bbl, lat, lon))
}

/// Borough code (leading digit) and tax block (next 5 digits) of a 10-digit BBL.
pub fn bbl_block(bbl: &str) -> Option<(u32, u32)> {
    if bbl.len() != 10 {
        return None;
    }
    let boro = bbl.get(0..1)?.parse().ok()?;
    let block = bbl.get(1..6)?.parse().ok()?;
    Some((boro, block))
}

/// HPD violations for a borough + a set of tax blocks. wvxf-dwi5 has no BBL column, so we
/// query by `boroid` + `block in(...)` and reconstruct the BBL per row via `hpd_bbl`.
pub fn hpd_block_query(boroid: u32, blocks: &[u32]) -> Query {
    let list = blocks
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let base = format!("{SODA}/wvxf-dwi5.json");
    let params = vec![
        (
            "$select".to_string(),
            "boroid,block,lot,class,violationstatus,novissueddate".to_string(),
        ),
        (
            "$where".to_string(),
            format!("boroid={boroid} AND block in({list})"),
        ),
        ("$limit".to_string(), "50000".to_string()),
    ];
    (base, params)
}

/// Reconstruct the 10-digit BBL from HPD's separate borough/block/lot fields:
/// boro(1) + block(5, zero-padded) + lot(4, zero-padded).
pub fn hpd_bbl(v: &Value) -> Option<String> {
    let boro = s(v, "boroid")?.trim().parse::<u32>().ok()?;
    let block = s(v, "block")?.trim().parse::<u32>().ok()?;
    let lot = s(v, "lot")?.trim().parse::<u32>().ok()?;
    if boro == 0 {
        return None;
    }
    Some(format!("{boro}{block:05}{lot:04}"))
}

pub fn parse_hpd_violation(v: &Value) -> Option<Violation> {
    let class = s(v, "class")?.to_uppercase();
    // wvxf-dwi5 `violationstatus` is exactly "Open" / "Close".
    let status = s(v, "violationstatus").unwrap_or_default().to_lowercase();
    let open = status.starts_with("open");
    let year = s(v, "novissueddate")
        .and_then(|d| d.get(0..4).map(|y| y.to_string()))
        .and_then(|y| y.parse::<i32>().ok())
        .unwrap_or(0);
    Some(Violation { class, open, year })
}

/// A `bbl in (...)` Socrata query (used for DOB, whose `bbl` is a clean 10-digit column).
/// `$where` holds a space, so it goes through `.query()`. Chunk `bbls` upstream.
pub fn bbl_in_query(id: &str, select: &str, bbls: &[String]) -> Query {
    let list = bbls
        .iter()
        .map(|b| format!("'{b}'"))
        .collect::<Vec<_>>()
        .join(",");
    let base = format!("{SODA}/{id}.json");
    let params = vec![
        ("$select".to_string(), select.to_string()),
        ("$where".to_string(), format!("bbl in({list})")),
        ("$limit".to_string(), "50000".to_string()),
    ];
    (base, params)
}

pub fn parse_dob_has_elevator(v: &Value) -> bool {
    let is_elevator = s(v, "device_type")
        .unwrap_or_default()
        .eq_ignore_ascii_case("Elevator");
    let active = s(v, "device_status")
        .unwrap_or_default()
        .eq_ignore_ascii_case("ACTIVE");
    is_elevator && active
}

/// Full Census ACS5 URL for all Kings County (Brooklyn) tract rent medians. Contains no
/// spaces or `>` — only the `for=tract:*` / `in=state:36` colons the Census API expects
/// literally — so it is safe to send as a whole URL string.
pub fn census_url(key: &str) -> String {
    format!("https://api.census.gov/data/2023/acs/acs5?get=B25064_001E&for=tract:*&in=state:36&in=county:047&key={key}")
}

pub fn parse_census_medians(v: &Value) -> std::collections::HashMap<String, i32> {
    let mut out = std::collections::HashMap::new();
    let rows = match v.as_array() {
        Some(r) => r,
        None => return out,
    };
    for row in rows.iter().skip(1) {
        // row = [ median, state, county, tract ]
        let cols: Vec<&str> = row
            .as_array()
            .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        if cols.len() < 4 {
            continue;
        }
        let median: i32 = cols[0].parse().unwrap_or(-1);
        if median <= 0 {
            continue;
        } // drop suppressed/sentinel (e.g. -666666666)
        let geoid = format!("{}{}{}", cols[1], cols[2], cols[3]);
        out.insert(geoid, median);
    }
    out
}

/// 311 service requests (erm2-nwe9) inside a lat/long bounding box and since a recent cutoff.
/// Bounding to the curated set's box + recent complaints keeps a single request (with a
/// tens-of-thousands `$limit`) enough to cover the slice. `$where` carries spaces and `>`, so
/// it goes through `reqwest`'s `.query()` for encoding, same as the other Socrata builders.
pub fn complaints_311_query(
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
    limit: u32,
) -> Query {
    let base = format!("{SODA}/erm2-nwe9.json");
    let params = vec![
        ("$select".to_string(), "latitude,longitude".to_string()),
        (
            "$where".to_string(),
            format!(
                "created_date > '2024-01-01' \
                 AND latitude >= {min_lat} AND latitude <= {max_lat} \
                 AND longitude >= {min_lon} AND longitude <= {max_lon}"
            ),
        ),
        ("$limit".to_string(), limit.to_string()),
    ];
    (base, params)
}

/// (latitude, longitude) of a 311 record, or None if either coordinate is missing/unparseable.
pub fn parse_311_point(v: &Value) -> Option<(f64, f64)> {
    Some((fnum(v, "latitude")?, fnum(v, "longitude")?))
}

/// DOHMH restaurant inspections (43nn-pn8j) with a real letter grade inside a lat/long box.
/// The box (built around the curated set) is what restricts this to Brooklyn. `$where` carries
/// spaces + `>=`, so it goes through `reqwest`'s `.query()` for encoding like the other Socrata
/// builders. `$order` newest-first so the most recent grade wins a nearest-point tie downstream.
pub fn restaurant_grades_query(
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
    limit: u32,
) -> Query {
    let base = format!("{SODA}/43nn-pn8j.json");
    let params = vec![
        (
            "$select".to_string(),
            "grade,latitude,longitude,inspection_date".to_string(),
        ),
        (
            "$where".to_string(),
            format!(
                "grade in('A','B','C') \
                 AND latitude >= {min_lat} AND latitude <= {max_lat} \
                 AND longitude >= {min_lon} AND longitude <= {max_lon}"
            ),
        ),
        ("$order".to_string(), "inspection_date DESC".to_string()),
        ("$limit".to_string(), limit.to_string()),
    ];
    (base, params)
}

/// (grade, latitude, longitude) of a graded restaurant record, or None if the grade is empty or
/// either coordinate is missing/unparseable. Grades are the DOHMH letters A/B/C.
pub fn parse_restaurant_grade(v: &Value) -> Option<(String, f64, f64)> {
    let grade = s(v, "grade")?;
    let grade = grade.trim();
    if grade.is_empty() {
        return None;
    }
    Some((
        grade.to_string(),
        fnum(v, "latitude")?,
        fnum(v, "longitude")?,
    ))
}

// --- Rent stabilization (JustFix nyc-doffer) ---------------------------------------------------
// Source: JustFix.org (nyc-doffer), derived from NYC DOF Statement of Account records; latest
// year 2024. A single ~25 MB static CSV, keyless, one row per tax lot (BBL). Header:
//   ucbbl,uc2018,pdfsoa2018,uc2019,pdfsoa2019,...,uc2024,pdfsoa2024
// `ucbbl` is the 10-digit BBL; each `uc<year>` is that year's rent-stabilized unit count and may
// be blank or "NA" (a scrape gap, not a real change). The most recent numeric value is the count.

/// The keyless JustFix `rentstab_counts_from_doffer` static CSV (latest year 2024).
pub const RENTSTAB_URL: &str =
    "https://s3.amazonaws.com/justfix-data/rentstab_counts_from_doffer_2024.csv";

/// Column indices of the `uc<year>` unit counts, newest-first (2024,2023,…,2018). The header is
/// `ucbbl,uc2018,pdfsoa2018,…` so the counts land on odd indices 1..=13; scanning newest-first
/// yields the most recent reported value.
const UC_YEAR_COLS_NEWEST_FIRST: [usize; 7] = [13, 11, 9, 7, 5, 3, 1];

/// Parse one CSV row into `(bbl, latest_units)`, where `latest_units` is the most recent
/// non-blank / non-`NA` `uc<year>` value (scanning 2024 → 2018). Fields never contain embedded
/// commas, so a plain split is safe. Returns `None` for the header, a malformed BBL, or a row
/// with no numeric year at all. A row whose most recent numeric value is `0` (units dropped in a
/// later year) correctly yields `latest_units = 0`.
pub fn parse_rentstab_row(line: &str) -> Option<(String, i32)> {
    let cols: Vec<&str> = line.split(',').collect();
    if cols.len() < 15 {
        return None;
    }
    let bbl = cols[0].trim();
    // 10-digit numeric BBL only — this also drops the header row (`ucbbl`).
    if bbl.len() != 10 || !bbl.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    for &i in &UC_YEAR_COLS_NEWEST_FIRST {
        let cell = cols[i].trim();
        if cell.is_empty() || cell.eq_ignore_ascii_case("NA") {
            continue;
        }
        if let Ok(n) = cell.parse::<i32>() {
            return Some((bbl.to_string(), n));
        }
    }
    None
}

/// Download the JustFix nyc-doffer rent-stabilization CSV and return `bbl -> latest_units` for
/// only the BBLs in `bbl_set`. Streams the response line-by-line through a `BufReader` so the
/// ~25 MB file never fully lands in memory. Reuses the caller's blocking `reqwest` client.
pub fn fetch_rent_stab(
    client: &reqwest::blocking::Client,
    bbl_set: &HashSet<String>,
) -> Result<HashMap<String, i32>> {
    let resp = client
        .get(RENTSTAB_URL)
        .send()
        .with_context(|| format!("GET {RENTSTAB_URL}"))?
        .error_for_status()
        .with_context(|| format!("bad status for {RENTSTAB_URL}"))?;
    let reader = BufReader::new(resp);
    let mut out = HashMap::new();
    for line in reader.lines() {
        let line = line.context("read rentstab CSV line")?;
        if let Some((bbl, units)) = parse_rentstab_row(&line) {
            if bbl_set.contains(&bbl) {
                out.insert(bbl, units);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn param<'a>(params: &'a [(String, String)], key: &str) -> &'a str {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .unwrap_or_else(|| panic!("missing param {key}"))
    }

    #[test]
    fn pluto_query_filters_by_cd_and_residential() {
        let (base, params) = pluto_query(303, 200);
        assert!(base.ends_with("/64uk-42ks.json"), "base was {base}");
        // The where clause carries the raw operators; reqwest encodes them at send time.
        let where_clause = param(&params, "$where");
        assert!(where_clause.contains("borough='BK'"));
        assert!(where_clause.contains("cd=303"));
        assert!(where_clause.contains("unitsres>0"));
        assert_eq!(param(&params, "$limit"), "200");
        assert_eq!(param(&params, "$order"), "unitsres DESC");
        assert!(param(&params, "$select").contains("bbl"));
        assert!(param(&params, "$select").contains("bct2020"));
    }

    #[test]
    fn bbl_in_query_builds_where_in_clause() {
        let bbls = ["3018420001".to_string(), "3018420002".to_string()];
        let (base, params) = bbl_in_query("e5aq-a4j2", "bbl,device_type,device_status", &bbls);
        assert!(base.ends_with("/e5aq-a4j2.json"), "base was {base}");
        assert_eq!(param(&params, "$select"), "bbl,device_type,device_status");
        assert_eq!(
            param(&params, "$where"),
            "bbl in('3018420001','3018420002')"
        );
        assert_eq!(param(&params, "$limit"), "50000");
    }

    #[test]
    fn hpd_block_query_filters_by_boro_and_blocks() {
        let (base, params) = hpd_block_query(3, &[1599, 1970]);
        assert!(base.ends_with("/wvxf-dwi5.json"), "base was {base}");
        assert_eq!(param(&params, "$where"), "boroid=3 AND block in(1599,1970)");
        assert!(param(&params, "$select").contains("violationstatus"));
    }

    #[test]
    fn census_url_targets_kings_county_rent_median() {
        let url = census_url("SECRET");
        assert!(url.contains("B25064_001E"));
        assert!(url.contains("in=state:36"));
        assert!(url.contains("in=county:047"));
        assert!(url.contains("key=SECRET"));
    }

    #[test]
    fn parses_pluto_record_into_building() {
        // PLUTO ships bbl as a float string and the 2020 tract in bct2020 (boro + 6-digit).
        let v = json!({
            "bbl": "3018420001.00000000",
            "address": "123 MACON STREET",
            "yearbuilt": "1910",
            "numfloors": "3.0000000",
            "unitsres": "6",
            "bldgclass": "C0",
            "latitude": "40.6829",
            "longitude": "-73.9251",
            "bct2020": "3025300"
        });
        let b = parse_pluto(&v).expect("valid record");
        assert_eq!(b.bbl, "3018420001"); // float form normalized to canonical 10 digits
        assert_eq!(b.year_built, 1910);
        assert_eq!(b.num_floors, 3);
        assert_eq!(b.units_res, 6);
        // tract_geoid = state(36) + county(047) + tract(bct2020 minus boro digit)
        assert_eq!(b.tract_geoid, "36047025300");
        // defaults for fields PLUTO doesn't carry:
        assert!(!b.has_elevator);
        assert_eq!(b.rent_stabilized, None);
    }

    #[test]
    fn pluto_coords_uses_normalized_bbl() {
        let v =
            json!({"bbl": "3018420001.00000000", "latitude": "40.6829", "longitude": "-73.9251"});
        let (bbl, lat, lon) = pluto_coords(&v).expect("coords");
        assert_eq!(bbl, "3018420001");
        assert!((lat - 40.6829).abs() < 1e-6);
        assert!((lon + 73.9251).abs() < 1e-6);
    }

    #[test]
    fn bbl_block_splits_boro_and_block() {
        assert_eq!(bbl_block("3015990007"), Some((3, 1599)));
        assert_eq!(bbl_block("bad"), None);
    }

    #[test]
    fn hpd_bbl_reconstructs_from_boro_block_lot() {
        let v = json!({"boroid": "3", "block": "1599", "lot": "7"});
        assert_eq!(hpd_bbl(&v).as_deref(), Some("3015990007"));
        // padding: block/lot zero-filled to 5/4 digits
        let v2 = json!({"boroid": "3", "block": "1599", "lot": "26"});
        assert_eq!(hpd_bbl(&v2).as_deref(), Some("3015990026"));
    }

    #[test]
    fn parses_hpd_violation_open_and_class() {
        let v = json!({"class": "C", "violationstatus": "Open", "novissueddate": "2025-06-01T00:00:00.000"});
        let viol = parse_hpd_violation(&v).expect("valid");
        assert_eq!(viol.class, "C");
        assert!(viol.open);
        assert_eq!(viol.year, 2025);
    }

    #[test]
    fn hpd_violation_closed_status_is_not_open() {
        let v = json!({"class": "A", "violationstatus": "Close", "novissueddate": "2018-01-01T00:00:00.000"});
        assert!(!parse_hpd_violation(&v).unwrap().open);
    }

    #[test]
    fn dob_record_flags_active_passenger_elevator() {
        // Live DOB status casing is "Active"; parser is case-insensitive.
        assert!(parse_dob_has_elevator(
            &json!({"device_type": "Elevator", "device_status": "Active"})
        ));
        assert!(!parse_dob_has_elevator(
            &json!({"device_type": "Escalator", "device_status": "Active"})
        ));
        assert!(!parse_dob_has_elevator(
            &json!({"device_type": "Elevator", "device_status": "Work in Progress"})
        ));
    }

    #[test]
    fn parses_census_median_and_rejects_sentinels() {
        // Census returns arrays: [ [ "B25064_001E", "state","county","tract" ], [ "1850","36","047","025300" ], ... ]
        let v = json!([
            ["B25064_001E", "state", "county", "tract"],
            ["1850", "36", "047", "025300"],
            ["-666666666", "36", "047", "025400"]
        ]);
        let map = parse_census_medians(&v);
        assert_eq!(map.get("36047025300"), Some(&1850));
        assert_eq!(map.get("36047025400"), None); // sentinel dropped
    }

    #[test]
    fn complaints_311_query_bounds_bbox_and_recent_date() {
        let (base, params) = complaints_311_query(40.68, -74.0, 40.70, -73.90, 50000);
        assert!(base.ends_with("/erm2-nwe9.json"), "base was {base}");
        let where_clause = param(&params, "$where");
        assert!(where_clause.contains("created_date > '2024-01-01'"));
        assert!(where_clause.contains("latitude >= 40.68"));
        assert!(where_clause.contains("latitude <= 40.7"));
        assert!(where_clause.contains("longitude >= -74"));
        assert!(where_clause.contains("longitude <= -73.9"));
        assert_eq!(param(&params, "$select"), "latitude,longitude");
        assert_eq!(param(&params, "$limit"), "50000");
    }

    #[test]
    fn parses_311_point_from_string_or_number() {
        // Socrata returns the coords as strings in some slices, JSON numbers in others.
        let (lat, lon) =
            parse_311_point(&json!({"latitude": "40.6829", "longitude": "-73.9251"})).unwrap();
        assert!((lat - 40.6829).abs() < 1e-6);
        assert!((lon + 73.9251).abs() < 1e-6);
        assert!(parse_311_point(&json!({"latitude": 40.6829, "longitude": -73.9251})).is_some());
        // A record missing a coordinate is dropped, not defaulted to (0,0).
        assert!(parse_311_point(&json!({"latitude": "40.6829"})).is_none());
    }

    #[test]
    fn restaurant_grades_query_bounds_bbox_and_letter_grades() {
        let (base, params) = restaurant_grades_query(40.68, -74.0, 40.70, -73.90, 20000);
        assert!(base.ends_with("/43nn-pn8j.json"), "base was {base}");
        let where_clause = param(&params, "$where");
        assert!(where_clause.contains("grade in('A','B','C')"));
        assert!(where_clause.contains("latitude >= 40.68"));
        assert!(where_clause.contains("longitude <= -73.9"));
        assert_eq!(param(&params, "$order"), "inspection_date DESC");
        assert_eq!(param(&params, "$limit"), "20000");
    }

    #[test]
    fn parses_restaurant_grade_and_drops_empty() {
        // Socrata ships the coords as strings in some slices, numbers in others.
        let (g, lat, lon) = parse_restaurant_grade(
            &json!({"grade": "A", "latitude": "40.6829", "longitude": "-73.9251"}),
        )
        .expect("graded record");
        assert_eq!(g, "A");
        assert!((lat - 40.6829).abs() < 1e-6);
        assert!((lon + 73.9251).abs() < 1e-6);
        assert!(parse_restaurant_grade(
            &json!({"grade": "B", "latitude": 40.6, "longitude": -73.9})
        )
        .is_some());
        // Empty grade or missing coordinate → dropped.
        assert!(parse_restaurant_grade(
            &json!({"grade": "", "latitude": 40.6, "longitude": -73.9})
        )
        .is_none());
        assert!(parse_restaurant_grade(&json!({"grade": "A", "latitude": 40.6})).is_none());
    }

    // Column layout: ucbbl,uc2018,pdfsoa2018,uc2019,pdfsoa2019,uc2020,pdfsoa2020,uc2021,
    // pdfsoa2021,uc2022,pdfsoa2022,uc2023,pdfsoa2023,uc2024,pdfsoa2024 (15 fields). The pdf
    // columns are left blank in these fixtures — only the uc counts matter.
    #[test]
    fn parse_rentstab_row_takes_most_recent_numeric() {
        // 2024 is an "NA" scrape gap, so the latest real value is 2023's 20 → stabilized.
        let line = "3018420001,15,,16,,18,,19,,20,,20,,NA,";
        let (bbl, units) = parse_rentstab_row(line).expect("valid row");
        assert_eq!(bbl, "3018420001");
        assert_eq!(units, 20);
        assert!(units > 0); // → rent_stabilized = Some(true)
    }

    #[test]
    fn parse_rentstab_row_trailing_zero_is_not_stabilized() {
        // Building carried stabilized units through 2023 but dropped to 0 in 2024 (the latest
        // numeric value) → latest_units 0, not stabilized.
        let line = "3018420002,8,,8,,6,,4,,2,,1,,0,";
        let (bbl, units) = parse_rentstab_row(line).expect("valid row");
        assert_eq!(bbl, "3018420002");
        assert_eq!(units, 0);
        assert!(units == 0); // → rent_stabilized = Some(false)
    }

    #[test]
    fn parse_rentstab_row_uses_2024_when_present() {
        let line = "3018420004,10,,20,,30,,35,,38,,40,,42,";
        assert_eq!(
            parse_rentstab_row(line),
            Some(("3018420004".to_string(), 42))
        );
    }

    #[test]
    fn parse_rentstab_row_all_na_yields_none() {
        // Every year blank or NA → no numeric value → dropped (building stays "unverified").
        let line = "3018420003,NA,,NA,,NA,,NA,,NA,,NA,,NA,";
        assert_eq!(parse_rentstab_row(line), None);
    }

    #[test]
    fn parse_rentstab_row_skips_header_and_malformed() {
        // The CSV header (ucbbl is non-numeric) is naturally skipped.
        let header = "ucbbl,uc2018,pdfsoa2018,uc2019,pdfsoa2019,uc2020,pdfsoa2020,uc2021,pdfsoa2021,uc2022,pdfsoa2022,uc2023,pdfsoa2023,uc2024,pdfsoa2024";
        assert_eq!(parse_rentstab_row(header), None);
        // Too few columns.
        assert_eq!(parse_rentstab_row("3018420001,5"), None);
        // Non-10-digit BBL.
        assert_eq!(parse_rentstab_row("999,1,,2,,3,,4,,5,,6,,7,"), None);
    }
}
