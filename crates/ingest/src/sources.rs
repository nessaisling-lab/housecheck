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

use model::{Building, Violation};
use serde_json::Value;

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
        rent_stabilized: None, // filled later from DHCR/WOW if available; None = unknown
        good_cause: false,     // filled later
        has_elevator: false,   // filled from DOB
        near_ada_subway_m: None, // filled from MTA
        complaints_311: 0,     // filled from 311 (Task 6)
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
}
