// Pure library of Socrata/Census query builders + JSON parsers consumed by the live
// orchestration in `run::run_real` (Plan 2 Task 5). URL builders return request COMPONENTS
// (a base URL + a list of `(key, value)` query params) rather than a pre-baked full-URL
// string, so the caller hands them to `reqwest`'s `.query()` and lets it percent-encode the
// SoQL `$where` clause (which contains spaces and `>` — invalid raw in a URL).
//
// Field names here reflect the datasets' *actual* live schemas, which differ from the original
// plan sketch: PLUTO ships `bbl` as a float string ("3015990007.00000000") and carries the 2020
// tract in `bct2020` (borough digit + 6-digit tract), and HPD violations (wvxf-dwi5) carry an
// `Open`/`Close` `violationstatus`. HPD gained a `bbl` column after 2026-07-22, but the BBL is
// still rebuilt from `boroid`/`block`/`lot` (see `hpd_row_bbl` for the building that decided
// it). The parsers below normalize all of that.

use anyhow::{Context, Result};
use model::{Building, Violation, ViolationClass};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader};

pub const SODA: &str = "https://data.cityofnewyork.us/resource";
pub const PLUTO_SELECT: &str =
    "bbl,address,yearbuilt,numfloors,unitsres,bldgclass,latitude,longitude,bct2020,zipcode";

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

/// PLUTO's two-letter borough code for a borough digit (the first digit of a BBL or of a
/// community district number: 303 is Brooklyn's CD 3).
pub fn pluto_borough(boro: u32) -> Option<&'static str> {
    ["MN", "BX", "BK", "QN", "SI"]
        .get((boro as usize).checked_sub(1)?)
        .copied()
}

/// The county FIPS code for a borough digit. Census tract GEOIDs are state + county + tract,
/// and each borough is its own county: Manhattan is New York County (061), the Bronx 005,
/// Brooklyn Kings (047), Queens 081, Staten Island Richmond (085).
pub fn county_fips(boro: u32) -> Option<&'static str> {
    ["061", "005", "047", "081", "085"]
        .get((boro as usize).checked_sub(1)?)
        .copied()
}

/// PLUTO residential buildings for one Community District. `$where` carries spaces and a
/// `>` operator, so it MUST be sent through `reqwest`'s `.query()` — never interpolated raw.
pub fn pluto_query(cd: u32, limit: u32, order: Option<&str>) -> Query {
    let base = format!("{SODA}/64uk-42ks.json");
    let borough = pluto_borough(cd / 100).unwrap_or("??");
    let mut params = vec![
        ("$select".to_string(), PLUTO_SELECT.to_string()),
        (
            "$where".to_string(),
            format!("borough='{borough}' AND cd={cd} AND unitsres>0"),
        ),
        ("$limit".to_string(), limit.to_string()),
    ];
    // Optional sort. The ingest blends two pulls: one by `unitsres DESC` (the largest, pre-war,
    // stabilization-eligible buildings) and one unordered (the neighborhood's small-rowhouse
    // mix), so both stories show instead of only one end of the size distribution.
    if let Some(o) = order {
        params.push(("$order".to_string(), o.to_string()));
    }
    (base, params)
}

/// A ZIP code as its five digits, or None for a blank, `N/A`, or anything else malformed.
/// 311's `incident_zip` carries ZIP+4 and junk alongside the real thing.
pub fn zip5(raw: &str) -> Option<String> {
    let z = raw.trim().get(0..5)?;
    z.bytes().all(|b| b.is_ascii_digit()).then(|| z.to_string())
}

pub fn parse_pluto(v: &Value) -> Option<Building> {
    let bbl = norm_bbl(&s(v, "bbl")?)?;
    // bct2020 = borough digit + 6-digit 2020 census tract, e.g. "3025300". The tract GEOID is
    // state(36) + the borough's county + the 6-digit tract (bct2020 minus its boro digit).
    let bct = s(v, "bct2020").unwrap_or_default();
    let tract_geoid = bct
        .get(0..1)
        .and_then(|b| b.parse().ok())
        .and_then(county_fips)
        .filter(|_| bct.len() == 7)
        .map(|county| format!("36{county}{}", &bct[1..]))
        .unwrap_or_default();
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
        zip: s(v, "zipcode").as_deref().and_then(zip5),
        neighborhood: None, // filled from the tract-to-NTA table in run_real
        complaints_311_per_1k_homes: None, // filled from the ZIP rates in run_real
        latitude: None,     // filled from the PLUTO coords map in run_real
        longitude: None,    // filled from the PLUTO coords map in run_real
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

/// HPD violation columns the ingest reads. novdescription is the notice's own wording -- the
/// difference between a count and a condition. currentstatusdate is when the status last
/// changed, which for a closed violation is when it closed, and is what time-to-close is
/// computed from.
pub const HPD_SELECT: &str =
    "bbl,boroid,block,lot,class,violationstatus,novissueddate,novdescription,currentstatusdate";

/// The letter of a row whose class `ViolationClass` does not model, or `None` for a modelled
/// class or a row with no class field.
///
/// Exists so `run` can count skipped rows by the letter they carried. That count is published
/// on the card and in `/meta`, and until 2026-10-30 every skipped letter was printed there as
/// class I: citywide, a new letter or a junk value would have been reported as one HPD issues.
pub fn hpd_excluded_class(v: &Value) -> Option<String> {
    let letter = s(v, "class")?.trim().to_uppercase();
    if letter.parse::<ViolationClass>().is_ok() {
        return None;
    }
    // An empty field is still a skipped row; name it rather than print a blank.
    Some(if letter.is_empty() {
        "(blank)".into()
    } else {
        letter
    })
}

/// The `/meta` line for skipped classes: `I (753 records skipped at ingest)`, one entry per
/// letter. The card prints it after "excluded: class", so the single-letter form is unchanged.
pub fn excluded_classes_note(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts
        .iter()
        .map(|(c, n)| format!("{c} ({n} records skipped at ingest)"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The pilot's violation filter: one borough's tax blocks, matching the block/lot that
/// `hpd_row_bbl` attributes rows by.
pub fn hpd_where_blocks(boroid: u32, blocks: &[u32]) -> String {
    let list = blocks
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("boroid={boroid} AND block in({list})")
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

/// A violation's BBL: rebuilt from HPD's borough/block/lot, as the ingest always has, with
/// HPD's `bbl` column only where those fields are blank.
///
/// **Not the `bbl` column first, and the reason is a building.** HPD rewrites that column to
/// the current tax lot after a merger. 88 Tompkins Ave was built in 2019 on lot 30, merged
/// from lots 30, 33 and 131; preferring the column handed it 55 violations from the demolished
/// buildings on lots 33 and 131 (different HPD `buildingid`s), issued in 2007 and closed
/// administratively in 2025. Its card then read "median 6,533 days to close" for a
/// six-year-old building. Block/lot keeps each row on the lot it was written against.
///
/// ponytail: misses violations re-lotted with no change of building (21 Prescott Pl has 9 rows
/// whose `bbl` names its pre-condo lot); joining on HPD's `buildingid` would catch those
/// without importing a predecessor's history.
pub fn hpd_row_bbl(v: &Value) -> Option<String> {
    hpd_bbl(v).or_else(|| {
        s(v, "bbl")
            .as_deref()
            .and_then(norm_bbl)
            .filter(|b| b.len() == 10)
    })
}

pub fn parse_hpd_violation(v: &Value) -> Option<Violation> {
    // A class outside A/B/C now makes the row unusable rather than merely unusual, because
    // every downstream site requires a modelled class. HPD also issues `I` (753 on the CD 303
    // pilot), and that skip is the exclusion stated on the card and in `/meta` — so it has to
    // stay countable, not silent. `run` counts it by letter through `hpd_excluded_class`.
    let class: ViolationClass = s(v, "class")?.trim().parse().ok()?;
    // wvxf-dwi5 `violationstatus` is exactly "Open" / "Close".
    let status = s(v, "violationstatus").unwrap_or_default().to_lowercase();
    let open = status.starts_with("open");
    let year = s(v, "novissueddate")
        .and_then(|d| d.get(0..4).map(|y| y.to_string()))
        .and_then(|y| y.parse::<i32>().ok())
        .unwrap_or(0);
    // Dates arrive as "2014-01-06T00:00:00.000"; keep the ISO date and drop the time,
    // which is always midnight and carries no information.
    let date = |k: &str| s(v, k).map(|d| d.chars().take(10).collect::<String>());
    Some(Violation {
        class,
        open,
        year,
        description: s(v, "novdescription"),
        issued_on: date("novissueddate"),
        closed_on: date("currentstatusdate"),
    })
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

/// Full Census ACS5 URL for one county's tract rent medians. Contains no spaces or `>` — only
/// the `for=tract:*` / `in=state:36` colons the Census API expects literally — so it is safe to
/// send as a whole URL string. One request per county keeps to a URL shape already known to
/// work.
pub fn census_url(key: &str, county: &str) -> String {
    format!("https://api.census.gov/data/2023/acs/acs5?get=B25064_001E&for=tract:*&in=state:36&in=county:{county}&key={key}")
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

/// 311 requests since 2024 per ZIP, counted by Socrata. One request returns ~500 rows
/// instead of downloading 10.4M complaint points.
pub fn complaints_311_by_zip_query() -> Query {
    (
        format!("{SODA}/erm2-nwe9.json"),
        vec![
            (
                "$select".to_string(),
                "incident_zip, count(1) AS n".to_string(),
            ),
            (
                "$where".to_string(),
                "created_date > '2024-01-01'".to_string(),
            ),
            ("$group".to_string(), "incident_zip".to_string()),
            ("$limit".to_string(), "50000".to_string()),
        ],
    )
}

/// Homes (PLUTO `unitsres`) per ZIP, summed by Socrata: the denominator for the 311 rate.
pub fn homes_by_zip_query() -> Query {
    (
        format!("{SODA}/64uk-42ks.json"),
        vec![
            (
                "$select".to_string(),
                "zipcode, sum(unitsres) AS n".to_string(),
            ),
            ("$where".to_string(), "unitsres>0".to_string()),
            ("$group".to_string(), "zipcode".to_string()),
            ("$limit".to_string(), "50000".to_string()),
        ],
    )
}

/// `zip -> n` from a grouped Socrata result, merging rows whose ZIPs normalize to the same
/// five digits (`11221` and `11221-1234`) and dropping the malformed ones.
pub fn parse_zip_counts(rows: &Value, zip_field: &str) -> HashMap<String, i64> {
    // Socrata sends grouped values as strings, but a JSON number must count too: dropping it
    // would leave ZIPs without a rate and their buildings on the 70 fallback.
    let text = |r: &Value, k: &str| match r.get(k) {
        Some(Value::String(x)) => Some(x.clone()),
        Some(Value::Number(x)) => Some(x.to_string()),
        _ => None,
    };
    let mut out = HashMap::new();
    for r in rows.as_array().map(Vec::as_slice).unwrap_or_default() {
        let (Some(z), Some(n)) = (
            text(r, zip_field).as_deref().and_then(zip5),
            text(r, "n").and_then(|n| n.trim().parse::<f64>().ok()),
        ) else {
            continue;
        };
        *out.entry(z).or_insert(0) += n as i64;
    }
    out
}

/// 311 complaints per 1,000 homes for every ZIP with homes on record. A ZIP with homes and no
/// complaints gets 0; a ZIP with no homes gets nothing, since a rate per zero homes is not a
/// number anyone should read.
pub fn zip_rates(
    complaints: &HashMap<String, i64>,
    homes: &HashMap<String, i64>,
) -> HashMap<String, i32> {
    homes
        .iter()
        .filter(|(_, &h)| h > 0)
        .map(|(z, &h)| {
            let c = complaints.get(z).copied().unwrap_or(0) as f64;
            (z.clone(), (c * 1000.0 / h as f64).round() as i32)
        })
        .collect()
}

/// DCP's 2020 census tract to Neighborhood Tabulation Area table: 2,327 rows, one request.
pub fn nta_query() -> Query {
    (
        format!("{SODA}/hm78-6dwm.json"),
        vec![
            ("$select".to_string(), "geoid,ntaname".to_string()),
            ("$limit".to_string(), "50000".to_string()),
        ],
    )
}

/// `tract GEOID -> neighborhood name`. The GEOID is the same 11-digit form `parse_pluto`
/// builds, so a building's neighborhood is one lookup.
pub fn parse_nta(rows: &Value) -> HashMap<String, String> {
    rows.as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|r| Some((s(r, "geoid")?, s(r, "ntaname")?)))
        .collect()
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
        let (base, params) = pluto_query(303, 200, Some("unitsres DESC"));
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
        assert!(param(&params, "$select").contains("zipcode"));
        // No order → no $order param (the unordered neighborhood-mix pull).
        let (_, unordered) = pluto_query(303, 200, None);
        assert!(!unordered.iter().any(|(k, _)| k == "$order"));
    }

    #[test]
    fn pluto_query_takes_the_borough_from_the_district() {
        // Brooklyn is no longer hardcoded: the district's leading digit names the borough.
        let borough = |cd| param(&pluto_query(cd, 1, None).1, "$where").to_string();
        assert!(borough(101).contains("borough='MN' AND cd=101"));
        assert!(borough(212).contains("borough='BX'"));
        assert!(borough(407).contains("borough='QN'"));
        assert!(borough(503).contains("borough='SI'"));
    }

    #[test]
    fn every_borough_has_its_county() {
        assert_eq!(
            (1..=5).map(county_fips).collect::<Vec<_>>(),
            ["061", "005", "047", "081", "085"].map(Some)
        );
        assert_eq!(county_fips(0), None);
        assert_eq!(county_fips(6), None);
    }

    #[test]
    fn hpd_where_blocks_filters_by_boro_and_blocks() {
        assert_eq!(
            hpd_where_blocks(3, &[1599, 1970]),
            "boroid=3 AND block in(1599,1970)"
        );
    }

    #[test]
    fn census_url_targets_one_county_rent_median() {
        let url = census_url("SECRET", "081");
        assert!(url.contains("B25064_001E"));
        assert!(url.contains("in=state:36"));
        assert!(url.contains("in=county:081"));
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
            "bct2020": "3025300",
            "zipcode": "11233"
        });
        let b = parse_pluto(&v).expect("valid record");
        assert_eq!(b.bbl, "3018420001"); // float form normalized to canonical 10 digits
        assert_eq!(b.year_built, 1910);
        assert_eq!(b.num_floors, 3);
        assert_eq!(b.units_res, 6);
        // tract_geoid = state(36) + county(047) + tract(bct2020 minus boro digit)
        assert_eq!(b.tract_geoid, "36047025300");
        assert_eq!(b.zip.as_deref(), Some("11233"));
        // defaults for fields PLUTO doesn't carry:
        assert!(!b.has_elevator);
        assert_eq!(b.rent_stabilized, None);
    }

    #[test]
    fn pluto_tract_takes_its_county_from_the_borough() {
        let tract = |bct: &str| {
            parse_pluto(&json!({"bbl": "1000010001", "bct2020": bct}))
                .unwrap()
                .tract_geoid
        };
        assert_eq!(tract("1001901"), "36061001901"); // Manhattan
        assert_eq!(tract("2001901"), "36005001901"); // the Bronx
        assert_eq!(tract("4001901"), "36081001901"); // Queens
        assert_eq!(tract("5001901"), "36085001901"); // Staten Island
        assert_eq!(tract("9001901"), ""); // not a borough: no tract rather than a wrong one
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
    fn hpd_row_bbl_keeps_a_row_on_the_lot_it_was_written_against() {
        // A demolished building's violation on old lot 33, which HPD's bbl column has since
        // rewritten to the merged lot 30: it must not land on the 2019 building now there.
        let merged = json!({"bbl": "3017470030", "boroid": "3", "block": "1747", "lot": "33"});
        assert_eq!(hpd_row_bbl(&merged).as_deref(), Some("3017470033"));
        // Block/lot blank: the column is all there is.
        let column_only = json!({"bbl": "3015990007"});
        assert_eq!(hpd_row_bbl(&column_only).as_deref(), Some("3015990007"));
        assert_eq!(hpd_row_bbl(&json!({})), None);
    }

    #[test]
    fn parses_hpd_violation_open_and_class() {
        let v = json!({"class": "C", "violationstatus": "Open", "novissueddate": "2025-06-01T00:00:00.000"});
        let viol = parse_hpd_violation(&v).expect("valid");
        assert_eq!(viol.class, ViolationClass::C);
        assert!(viol.open);
        assert_eq!(viol.year, 2025);
    }

    /// REVIEW-NOTES M4 at ingest: a skipped class is counted under its own letter, never
    /// folded into class I, and never stored.
    #[test]
    fn excluded_classes_are_counted_by_the_letter_they_carried() {
        // Modelled classes, in any case or padding, and rows with no class are not exclusions.
        assert_eq!(hpd_excluded_class(&json!({"class": "C"})), None);
        assert_eq!(hpd_excluded_class(&json!({"class": "c "})), None);
        assert_eq!(hpd_excluded_class(&json!({})), None);
        // HPD's class I, and anything else, keep their own letter.
        assert_eq!(
            hpd_excluded_class(&json!({"class": " i"})).as_deref(),
            Some("I")
        );
        assert_eq!(
            hpd_excluded_class(&json!({"class": "X"})).as_deref(),
            Some("X")
        );
        assert_eq!(
            hpd_excluded_class(&json!({"class": ""})).as_deref(),
            Some("(blank)")
        );
        assert!(parse_hpd_violation(&json!({"class": "X", "violationstatus": "Open"})).is_none());

        // The published line is unchanged while I is the only exclusion...
        let mut counts = BTreeMap::new();
        counts.insert("I".to_string(), 753);
        assert_eq!(
            excluded_classes_note(&counts),
            "I (753 records skipped at ingest)"
        );
        // ...and a second letter is named, not reported as class I.
        counts.insert("X".to_string(), 2);
        assert_eq!(
            excluded_classes_note(&counts),
            "I (753 records skipped at ingest), X (2 records skipped at ingest)"
        );
        assert_eq!(excluded_classes_note(&BTreeMap::new()), "none");
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
    fn zip5_keeps_five_digits_and_drops_junk() {
        assert_eq!(zip5("11221").as_deref(), Some("11221"));
        assert_eq!(zip5("11221-1234").as_deref(), Some("11221"));
        assert_eq!(zip5(" 10466 ").as_deref(), Some("10466"));
        assert_eq!(zip5("N/A"), None);
        assert_eq!(zip5("1122"), None);
    }

    #[test]
    fn zip_counts_merge_variants_and_rates_divide_by_homes() {
        let c311 = parse_zip_counts(
            &json!([
                {"incident_zip": "11221", "n": "300"},
                {"incident_zip": "11221-1234", "n": 60},
                {"incident_zip": 11221, "n": "40"},
                {"incident_zip": "N/A", "n": "999"},
                {"n": "5"}
            ]),
            "incident_zip",
        );
        // A count or a ZIP sent as a JSON number counts like a string one.
        assert_eq!(c311, HashMap::from([("11221".to_string(), 400)]));
        let homes = parse_zip_counts(
            &json!([
                {"zipcode": "11221", "n": "200"},
                {"zipcode": "10001", "n": "50"},
                {"zipcode": "10002", "n": "0"}
            ]),
            "zipcode",
        );
        let rates = zip_rates(&c311, &homes);
        assert_eq!(rates.get("11221"), Some(&2000)); // 400 complaints / 200 homes * 1000
        assert_eq!(rates.get("10001"), Some(&0)); // homes, no complaints: a real zero
        assert_eq!(rates.get("10002"), None); // no homes: no rate
    }

    #[test]
    fn grouped_queries_ask_socrata_to_count() {
        let (base, p) = complaints_311_by_zip_query();
        assert!(base.ends_with("/erm2-nwe9.json"));
        assert_eq!(param(&p, "$group"), "incident_zip");
        assert_eq!(param(&p, "$where"), "created_date > '2024-01-01'");
        let (base, p) = homes_by_zip_query();
        assert!(base.ends_with("/64uk-42ks.json"));
        assert_eq!(param(&p, "$group"), "zipcode");
    }

    #[test]
    fn nta_maps_tract_geoid_to_name() {
        let m = parse_nta(&json!([
            {"geoid": "36005001901", "ntaname": "Mott Haven-Port Morris"},
            {"geoid": "36047025300"}
        ]));
        assert_eq!(
            m.get("36005001901").map(String::as_str),
            Some("Mott Haven-Port Morris")
        );
        assert_eq!(m.len(), 1);
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
