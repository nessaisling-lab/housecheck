# HouseCheck Real Brooklyn Ingest Implementation Plan (Plan 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Replace the fixture DB with a real one built from live NYC Open Data + Census for a curated Brooklyn community district, producing the exact same SQLite schema the backend already serves — so `api`/`scoring` need no changes.

**Architecture:** A `--real` mode is added to the existing `crates/ingest` binary. It uses `reqwest::blocking` to pull from the Socrata SODA API (HPD, DOB, 311, DOHMH) and the Census API (B25064), transforms records with pure, unit-tested functions, computes accessibility + nearby-context via haversine, and writes rows through new `store` insert functions. No DuckDB, no geospatial engine: PLUTO provides the census tract per BBL directly.

**Tech Stack:** Rust, `reqwest` (blocking, json, rustls-tls), `serde`/`serde_json`, existing `rusqlite`/`store`/`model`.

**Scope:** Real ingest for one configurable Brooklyn Community District (default CD 303, Bed-Stuy). NOT in scope: full-NYC live (DuckDB, stretch), `/search` geocoding (Plan 3 / separate), frontend (Plan 3).

## Prerequisites (human, one-time)
- Free **Census API key** → https://api.census.gov/data/key_signup.html → set env `CENSUS_API_KEY`.
- Network access to `data.cityofnewyork.us`, `data.ny.gov`, `api.census.gov` at execution time.

## Data sources (verified 2026-07-21; Socrata `/resource/<id>.json?$where=...&$limit=...`)
| Purpose | Dataset | Key fields | Filter |
|---|---|---|---|
| Buildings + tract + geom | PLUTO `64uk-42ks` | `bbl,address,yearbuilt,numfloors,unitsres,bldgclass,latitude,longitude,ct2020,cd` | `borough='BK' AND cd=<CD> AND unitsres>0` |
| Violations | HPD `wvxf-dwi5` | `bbl,class,currentstatusid/currentstatus,novissueddate` | `bbl in (...)` (chunked) |
| Elevators | DOB `e5aq-a4j2` | `bbl,device_type,device_status` | `bbl in (...)` |
| 311 (nearby) | `erm2-nwe9` | `latitude,longitude,created_date` | `borough='BROOKLYN'` bbox around CD |
| Restaurants (nearby) | DOHMH `43nn-pn8j` | `boro,latitude,longitude,grade` | `boro='Brooklyn'` bbox |
| Rent median by tract | Census `B25064_001E` | tract median | `state:36 county:047 tract:*` |
| ADA subway | MTA `39hk-dx4f` (data.ny.gov) | `gtfs_latitude,gtfs_longitude,ada` | all (496 rows) |

---

### Task 1: `ingest` gains a `--real` mode + config

**Files:**
- Modify: `crates/ingest/Cargo.toml`
- Modify: `crates/ingest/src/main.rs`
- Create: `crates/ingest/src/config.rs`

- [ ] **Step 1: Add HTTP + serde deps**

`crates/ingest/Cargo.toml` — add under `[dependencies]`:
```toml
reqwest = { version = "0.12", features = ["blocking", "json", "rustls-tls"], default-features = false }
serde = { workspace = true }
serde_json = { workspace = true }
model = { path = "../model" }
```

- [ ] **Step 2: Write the config with a failing test**

`crates/ingest/src/config.rs`:
```rust
/// Ingest run configuration, parsed from CLI args.
#[derive(Debug, PartialEq)]
pub struct Config {
    pub mode: Mode,
    pub out: String,
    pub community_district: u32, // e.g. 303 = Brooklyn CD3 (Bed-Stuy)
    pub limit: u32,              // cap building count for a demo-sized set
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    Fixture,
    Real,
}

impl Config {
    /// Parse from an arg list (excluding program name). Errors are human strings.
    pub fn parse(args: &[String]) -> Result<Config, String> {
        let flag = |name: &str| args.iter().position(|a| a == name).map(|i| args.get(i + 1));
        let mode = if args.iter().any(|a| a == "--real") {
            Mode::Real
        } else if args.iter().any(|a| a == "--fixture") {
            Mode::Fixture
        } else {
            return Err("must pass --fixture or --real".into());
        };
        let out = flag("--out")
            .flatten()
            .cloned()
            .ok_or("missing --out <path>")?;
        let community_district = flag("--cd")
            .flatten()
            .map(|s| s.parse().map_err(|_| "bad --cd".to_string()))
            .transpose()?
            .unwrap_or(303);
        let limit = flag("--limit")
            .flatten()
            .map(|s| s.parse().map_err(|_| "bad --limit".to_string()))
            .transpose()?
            .unwrap_or(200);
        Ok(Config { mode, out, community_district, limit })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_mode_with_defaults() {
        let args = ["--real", "--out", "data/hc.db"].map(String::from);
        let c = Config::parse(&args).unwrap();
        assert_eq!(c, Config { mode: Mode::Real, out: "data/hc.db".into(), community_district: 303, limit: 200 });
    }

    #[test]
    fn requires_a_mode() {
        let args = ["--out", "x".to_string()].map(String::from);
        assert!(Config::parse(&args).is_err());
    }

    #[test]
    fn overrides_cd_and_limit() {
        let args = ["--real", "--out", "x", "--cd", "301", "--limit", "50"].map(String::from);
        let c = Config::parse(&args).unwrap();
        assert_eq!(c.community_district, 301);
        assert_eq!(c.limit, 50);
    }
}
```

- [ ] **Step 3: Run tests, expect FAIL then wire the module**

Run: `cargo test -p ingest config` → FAIL (module not declared).
Add `mod config;` to the top of `crates/ingest/src/main.rs` and re-run → 3 pass.

- [ ] **Step 4: Route `main` through Config (keep fixture behavior identical)**

Replace the body of `main()` in `crates/ingest/src/main.rs` with:
```rust
mod config;
mod sources;
mod geo;
mod run;

use anyhow::Result;
use config::{Config, Mode};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cfg = Config::parse(&args).map_err(|e| anyhow::anyhow!(e))?;
    match cfg.mode {
        Mode::Fixture => {
            let _ = std::fs::remove_file(&cfg.out);
            let conn = store::open_db(&cfg.out)?;
            store::migrate(&conn)?;
            store::insert_fixture(&conn)?;
            println!("built fixture DB at {}", cfg.out);
        }
        Mode::Real => run::run_real(&cfg)?,
    }
    Ok(())
}
```
(You will create `sources`, `geo`, `run` in later tasks. To compile Task 1 alone, temporarily comment out `mod sources; mod geo; mod run;` and the `Mode::Real` arm, or implement stubs — but the cleanest path is to do Tasks 2–5 before final `cargo build`. If you split the commit, make Task 1's commit compile by stubbing `run_real` to `unimplemented!()` behind the real modules.)

- [ ] **Step 5: Commit**
```bash
git add crates/ingest
git commit -m "feat(ingest): --real mode config parsing

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: `sources` — pure API URL builders + JSON parsers (TDD)

**Files:**
- Create: `crates/ingest/src/sources.rs`

These functions are pure: they build request URLs and parse `serde_json::Value` responses into typed rows. No network in tests — feed them sample JSON literals captured from the real APIs.

- [ ] **Step 1: Write failing tests with real-shaped sample JSON**

`crates/ingest/src/sources.rs`:
```rust
use model::{Building, Violation};
use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pluto_url_filters_by_cd_and_residential() {
        let url = pluto_url(303, 200);
        assert!(url.contains("64uk-42ks.json"));
        assert!(url.contains("cd=303"));
        assert!(url.contains("unitsres%3E0") || url.contains("unitsres>0"));
        assert!(url.contains("$limit=200"));
    }

    #[test]
    fn parses_pluto_record_into_building() {
        let v = json!({
            "bbl": "3018420001",
            "address": "123 MACON STREET",
            "yearbuilt": "1910",
            "numfloors": "3",
            "unitsres": "6",
            "bldgclass": "C0",
            "latitude": "40.6829",
            "longitude": "-73.9251",
            "ct2020": "025300"
        });
        let b = parse_pluto(&v).expect("valid record");
        assert_eq!(b.bbl, "3018420001");
        assert_eq!(b.year_built, 1910);
        assert_eq!(b.num_floors, 3);
        assert_eq!(b.units_res, 6);
        // tract_geoid = state(36)+county(047)+ct2020(6-digit)
        assert_eq!(b.tract_geoid, "36047025300");
        // defaults for fields PLUTO doesn't carry:
        assert_eq!(b.has_elevator, false);
        assert_eq!(b.rent_stabilized, None);
    }

    #[test]
    fn parses_hpd_violation_open_and_class() {
        let v = json!({"bbl": "3018420001", "class": "C", "currentstatus": "Open", "novissueddate": "2025-06-01T00:00:00.000"});
        let viol = parse_hpd_violation(&v).expect("valid");
        assert_eq!(viol.class, "C");
        assert!(viol.open);
        assert_eq!(viol.year, 2025);
    }

    #[test]
    fn hpd_violation_closed_status_is_not_open() {
        let v = json!({"bbl": "3018420001", "class": "A", "currentstatus": "Close", "novissueddate": "2018-01-01T00:00:00.000"});
        assert!(!parse_hpd_violation(&v).unwrap().open);
    }

    #[test]
    fn dob_record_flags_active_passenger_elevator() {
        assert!(parse_dob_has_elevator(&json!({"device_type": "Elevator", "device_status": "ACTIVE"})));
        assert!(!parse_dob_has_elevator(&json!({"device_type": "Escalator", "device_status": "ACTIVE"})));
        assert!(!parse_dob_has_elevator(&json!({"device_type": "Elevator", "device_status": "DISMANTLED"})));
    }

    #[test]
    fn parses_census_median_and_rejects_sentinels() {
        // Census returns arrays: [ [ "B25064_001E", "state","county","tract" ], [ "1850","36","047","025300" ], ... ]
        let v = json!([["B25064_001E","state","county","tract"],
                       ["1850","36","047","025300"],
                       ["-666666666","36","047","025400"]]);
        let map = parse_census_medians(&v);
        assert_eq!(map.get("36047025300"), Some(&1850));
        assert_eq!(map.get("36047025400"), None); // sentinel dropped
    }
}
```

- [ ] **Step 2: Run to verify FAIL**

Run: `cargo test -p ingest sources` → FAIL (functions undefined).

- [ ] **Step 3: Implement the pure functions**

Add above the test module in `crates/ingest/src/sources.rs`:
```rust
const SODA: &str = "https://data.cityofnewyork.us/resource";

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
}
fn num(v: &Value, k: &str) -> i32 {
    s(v, k).and_then(|x| x.trim().parse::<f64>().ok()).map(|f| f as i32).unwrap_or(0)
}

pub fn pluto_url(cd: u32, limit: u32) -> String {
    // $where is URL-encoded by reqwest when we pass it as a query param, but we build the
    // full string here for transparency/tests; encode the comparison operator.
    format!(
        "{SODA}/64uk-42ks.json?$select=bbl,address,yearbuilt,numfloors,unitsres,bldgclass,latitude,longitude,ct2020&$where=borough='BK' AND cd={cd} AND unitsres>0&$limit={limit}"
    )
}

pub fn parse_pluto(v: &Value) -> Option<Building> {
    let bbl = s(v, "bbl")?;
    let ct = s(v, "ct2020").unwrap_or_default();
    // ct2020 is a 6-digit tract code; GEOID = 36 (NY) + 047 (Kings) + ct.
    let tract_geoid = if ct.len() == 6 { format!("36047{ct}") } else { String::new() };
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
        complaints_311: 0,     // filled from 311
    })
}

pub fn bbl_in_url(id: &str, select: &str, bbls: &[String]) -> String {
    let list = bbls.iter().map(|b| format!("'{b}'")).collect::<Vec<_>>().join(",");
    format!("{SODA}/{id}.json?$select={select}&$where=bbl in({list})&$limit=50000")
}

pub fn parse_hpd_violation(v: &Value) -> Option<Violation> {
    let class = s(v, "class")?.to_uppercase();
    let status = s(v, "currentstatus").unwrap_or_default().to_lowercase();
    let open = !status.starts_with("close");
    let year = s(v, "novissueddate")
        .and_then(|d| d.get(0..4).map(|y| y.to_string()))
        .and_then(|y| y.parse::<i32>().ok())
        .unwrap_or(0);
    Some(Violation { class, open, year })
}

pub fn parse_dob_has_elevator(v: &Value) -> bool {
    let is_elevator = s(v, "device_type").unwrap_or_default().eq_ignore_ascii_case("Elevator");
    let active = s(v, "device_status").unwrap_or_default().eq_ignore_ascii_case("ACTIVE");
    is_elevator && active
}

pub fn census_url(key: &str) -> String {
    format!("https://api.census.gov/data/2023/acs/acs5?get=B25064_001E&for=tract:*&in=state:36&in=county:047&key={key}")
}

pub fn parse_census_medians(v: &Value) -> std::collections::HashMap<String, i32> {
    let mut out = std::collections::HashMap::new();
    let rows = match v.as_array() { Some(r) => r, None => return out };
    for row in rows.iter().skip(1) {
        // row = [ median, state, county, tract ]
        let cols: Vec<&str> = row.as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        if cols.len() < 4 { continue; }
        let median: i32 = cols[0].parse().unwrap_or(-1);
        if median <= 0 { continue; } // drop suppressed/sentinel (e.g. -666666666)
        let geoid = format!("{}{}{}", cols[1], cols[2], cols[3]);
        out.insert(geoid, median);
    }
    out
}
```

- [ ] **Step 4: Run to verify PASS**

Run: `cargo test -p ingest sources` → all pass.

- [ ] **Step 5: Commit**
```bash
git add crates/ingest
git commit -m "feat(ingest): pure Socrata/Census url builders + parsers

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: `geo` — haversine + nearest ADA station (TDD)

**Files:**
- Create: `crates/ingest/src/geo.rs`

- [ ] **Step 1: Failing tests**

`crates/ingest/src/geo.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_known_distance() {
        // ~1.11 km between 0.01 deg latitude apart at NYC.
        let d = haversine_m(40.68, -73.95, 40.69, -73.95);
        assert!((d - 1112.0).abs() < 20.0, "got {d}");
    }

    #[test]
    fn nearest_ada_returns_closest_ada_only() {
        let stations = vec![
            Station { lat: 40.70, lon: -73.95, ada: 0 }, // closest but not ADA
            Station { lat: 40.685, lon: -73.951, ada: 1 }, // ADA, a bit further
        ];
        let d = nearest_ada_m(40.68, -73.95, &stations).unwrap();
        // distance to the ADA station, not the non-ADA one
        assert!(d > 500.0 && d < 700.0, "got {d}");
    }

    #[test]
    fn nearest_ada_none_when_no_ada_stations() {
        let stations = vec![Station { lat: 40.70, lon: -73.95, ada: 0 }];
        assert!(nearest_ada_m(40.68, -73.95, &stations).is_none());
    }
}
```

- [ ] **Step 2: Run → FAIL**, then implement above the tests:
```rust
pub struct Station {
    pub lat: f64,
    pub lon: f64,
    pub ada: i32, // 0 none, 1 full, 2 partial
}

/// Great-circle distance in metres.
pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0_f64;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Metres to the nearest fully/partially ADA-accessible station (ada != 0), or None.
pub fn nearest_ada_m(lat: f64, lon: f64, stations: &[Station]) -> Option<f64> {
    stations
        .iter()
        .filter(|s| s.ada != 0)
        .map(|s| haversine_m(lat, lon, s.lat, s.lon))
        .fold(None, |acc, d| match acc {
            Some(m) if m <= d => Some(m),
            _ => Some(d),
        })
}
```

- [ ] **Step 3: Run → PASS.** Commit:
```bash
git add crates/ingest
git commit -m "feat(ingest): haversine + nearest-ADA-station geo helpers

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: `store` — real insert functions + snapshot meta (TDD)

**Files:**
- Modify: `crates/store/src/lib.rs`

- [ ] **Step 1: Failing tests** (add to `store` test module):
```rust
    #[test]
    fn upsert_building_and_reload() -> Result<()> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        let b = Building {
            bbl: "3018420001".into(), address: "123 Macon St".into(), year_built: 1910,
            num_floors: 3, units_res: 6, tract_geoid: "36047025300".into(),
            rent_stabilized: None, good_cause: false, has_elevator: true,
            near_ada_subway_m: Some(420), complaints_311: 7,
        };
        upsert_building(&conn, &b)?;
        assert_eq!(get_building(&conn, "3018420001")?.unwrap(), b);
        Ok(())
    }

    #[test]
    fn insert_violation_and_median_roundtrip() -> Result<()> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        insert_violation(&conn, "3018420001", &Violation { class: "C".into(), open: true, year: 2025 })?;
        upsert_tract_median(&conn, "36047025300", 1850)?;
        assert_eq!(get_tract_median(&conn, "36047025300")?, Some(1850));
        assert_eq!(get_open_violations(&conn, "3018420001")?.len(), 1);
        Ok(())
    }

    #[test]
    fn snapshot_date_roundtrip() -> Result<()> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        set_snapshot_year(&conn, 2026)?;
        assert_eq!(get_snapshot_year(&conn)?, Some(2026));
        Ok(())
    }
```

- [ ] **Step 2: Run → FAIL.** Add a `meta` table to `migrate`'s batch (append before the closing `");`):
```sql
         CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );
```
Then add implementations above the test module:
```rust
pub fn upsert_building(conn: &Connection, b: &Building) -> Result<()> {
    conn.execute(
        "INSERT INTO buildings
          (bbl,address,year_built,num_floors,units_res,tract_geoid,rent_stabilized,good_cause,has_elevator,near_ada_subway_m,complaints_311)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(bbl) DO UPDATE SET
          address=excluded.address, year_built=excluded.year_built, num_floors=excluded.num_floors,
          units_res=excluded.units_res, tract_geoid=excluded.tract_geoid, rent_stabilized=excluded.rent_stabilized,
          good_cause=excluded.good_cause, has_elevator=excluded.has_elevator,
          near_ada_subway_m=excluded.near_ada_subway_m, complaints_311=excluded.complaints_311",
        rusqlite::params![
            b.bbl, b.address, b.year_built, b.num_floors, b.units_res, b.tract_geoid,
            b.rent_stabilized.map(|v| v as i64), b.good_cause as i64, b.has_elevator as i64,
            b.near_ada_subway_m, b.complaints_311
        ],
    )?;
    Ok(())
}

pub fn insert_violation(conn: &Connection, bbl: &str, v: &Violation) -> Result<()> {
    conn.execute(
        "INSERT INTO violations (bbl,class,open,year) VALUES (?1,?2,?3,?4)",
        rusqlite::params![bbl, v.class, v.open as i64, v.year],
    )?;
    Ok(())
}

pub fn upsert_tract_median(conn: &Connection, tract_geoid: &str, median: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO acs_rent_by_tract (tract_geoid, median_gross_rent) VALUES (?1,?2)
         ON CONFLICT(tract_geoid) DO UPDATE SET median_gross_rent=excluded.median_gross_rent",
        rusqlite::params![tract_geoid, median],
    )?;
    Ok(())
}

pub fn set_snapshot_year(conn: &Connection, year: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key,value) VALUES ('snapshot_year', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![year.to_string()],
    )?;
    Ok(())
}

pub fn get_snapshot_year(conn: &Connection) -> Result<Option<i32>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key='snapshot_year'")?;
    let mut rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    match rows.next() {
        Some(v) => Ok(v?.parse::<i32>().ok()),
        None => Ok(None),
    }
}
```

- [ ] **Step 3: Run `cargo test -p store` → all pass.** Commit:
```bash
git add crates/store
git commit -m "feat(store): real upsert inserts + snapshot-year meta

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: `run` — live orchestration (integration; verified by running)

**Files:**
- Create: `crates/ingest/src/run.rs`

This wires the pure pieces to the network. It is verified by *running it against the live APIs*, not by unit tests (network + real data are non-deterministic). Keep all parsing/aggregation delegated to the tested `sources`/`geo`/`store` functions so `run.rs` stays thin.

- [ ] **Step 1: Implement the orchestration**

`crates/ingest/src/run.rs`:
```rust
use anyhow::{Context, Result};
use std::collections::HashMap;

use crate::config::Config;
use crate::geo::{haversine_m, nearest_ada_m, Station};
use crate::sources::*;

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("housecheck-ingest/0.1")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("client")
}

fn get_json(c: &reqwest::blocking::Client, url: &str) -> Result<serde_json::Value> {
    let resp = c.get(url).send().with_context(|| format!("GET {url}"))?;
    let resp = resp.error_for_status().with_context(|| format!("status for {url}"))?;
    Ok(resp.json()?)
}

pub fn run_real(cfg: &Config) -> Result<()> {
    let census_key = std::env::var("CENSUS_API_KEY")
        .context("set CENSUS_API_KEY (https://api.census.gov/data/key_signup.html)")?;
    let c = client();

    // 1. Buildings from PLUTO for the community district.
    let pluto = get_json(&c, &pluto_url(cfg.community_district, cfg.limit))?;
    let mut buildings: Vec<model::Building> = pluto.as_array().unwrap_or(&vec![])
        .iter().filter_map(parse_pluto).filter(|b| !b.bbl.is_empty()).collect();
    // capture lat/long for geo joins (parallel to buildings, by index)
    let coords: Vec<(f64, f64)> = pluto.as_array().unwrap_or(&vec![]).iter()
        .filter_map(|v| Some((v.get("latitude")?.as_str()?.parse().ok()?, v.get("longitude")?.as_str()?.parse().ok()?)))
        .collect();
    println!("PLUTO: {} residential buildings in CD {}", buildings.len(), cfg.community_district);
    let bbls: Vec<String> = buildings.iter().map(|b| b.bbl.clone()).collect();

    // 2. Violations (chunk bbls to keep URLs sane).
    let mut violations: HashMap<String, Vec<model::Violation>> = HashMap::new();
    let mut unknown_classes = 0usize;
    for chunk in bbls.chunks(200) {
        let url = bbl_in_url("wvxf-dwi5", "bbl,class,currentstatus,novissueddate", chunk);
        for v in get_json(&c, &url)?.as_array().unwrap_or(&vec![]) {
            if let (Some(bbl), Some(viol)) = (v.get("bbl").and_then(|x| x.as_str()), parse_hpd_violation(v)) {
                if !matches!(viol.class.as_str(), "A" | "B" | "C") { unknown_classes += 1; continue; }
                violations.entry(bbl.to_string()).or_default().push(viol);
            }
        }
    }
    if unknown_classes > 0 { println!("note: {unknown_classes} violations had non-A/B/C classes (skipped)"); }

    // 3. Elevators.
    let mut has_elevator: HashMap<String, bool> = HashMap::new();
    for chunk in bbls.chunks(200) {
        let url = bbl_in_url("e5aq-a4j2", "bbl,device_type,device_status", chunk);
        for v in get_json(&c, &url)?.as_array().unwrap_or(&vec![]) {
            if parse_dob_has_elevator(v) {
                if let Some(bbl) = v.get("bbl").and_then(|x| x.as_str()) {
                    has_elevator.insert(bbl.to_string(), true);
                }
            }
        }
    }

    // 4. Census tract medians (all Brooklyn, then looked up by tract).
    let medians = parse_census_medians(&get_json(&c, &census_url(&census_key))?);

    // 5. ADA subway stations (all, then nearest per building).
    let mta = get_json(&c, "https://data.ny.gov/resource/39hk-dx4f.json?$limit=1000")?;
    let stations: Vec<Station> = mta.as_array().unwrap_or(&vec![]).iter().filter_map(|v| Some(Station {
        lat: v.get("gtfs_latitude")?.as_str()?.parse().ok()?,
        lon: v.get("gtfs_longitude")?.as_str()?.parse().ok()?,
        ada: v.get("ada").and_then(|x| x.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0),
    })).collect();

    // 6. Enrich + write.
    let _ = std::fs::remove_file(&cfg.out);
    let conn = store::open_db(&cfg.out)?;
    store::migrate(&conn)?;
    // SCORING_YEAR source of truth = ingest year. Pass a fixed year via env to stay reproducible.
    let snapshot_year: i32 = std::env::var("SNAPSHOT_YEAR").ok().and_then(|s| s.parse().ok()).unwrap_or(2026);
    store::set_snapshot_year(&conn, snapshot_year)?;

    for (i, b) in buildings.iter_mut().enumerate() {
        b.has_elevator = *has_elevator.get(&b.bbl).unwrap_or(&false);
        if let Some((lat, lon)) = coords.get(i) {
            b.near_ada_subway_m = nearest_ada_m(*lat, *lon, &stations).map(|d| d as i32);
        }
        store::upsert_building(&conn, b)?;
        for v in violations.remove(&b.bbl).unwrap_or_default() {
            store::insert_violation(&conn, &b.bbl, &v)?;
        }
        if let Some(m) = medians.get(&b.tract_geoid) {
            store::upsert_tract_median(&conn, &b.tract_geoid, *m)?;
        }
    }
    println!("wrote {} buildings to {}", buildings.len(), cfg.out);
    Ok(())
}
```
> Note: `complaints_311` and nearby restaurant grades are **P1** and deliberately left at their PLUTO defaults (0) in this task — wiring the 311/DOHMH radius counts is Task 6. The score still computes; neighborhood just reflects 0 complaints until Task 6 lands.

- [ ] **Step 2: Compile the whole workspace**

Run: `cargo build --workspace`. Fix any compile errors (the earlier stubs from Task 1 should now resolve). Expected: clean build.

- [ ] **Step 3: Run the real ingest against live APIs**

Run (bash):
```bash
export CENSUS_API_KEY=<your key>
cargo run -p ingest -- --real --cd 303 --limit 100 --out data/housecheck.db
```
Expected: prints PLUTO building count, any unknown-class note, and `wrote N buildings to data/housecheck.db` with N ≈ 100.

- [ ] **Step 4: Verify with the API (end-to-end)**

```bash
HOUSECHECK_DB=data/housecheck.db cargo run -p api &
sleep 2
# pick a real bbl printed during ingest or query the db:
sqlite3 data/housecheck.db "select bbl,address,has_elevator from buildings limit 3;"
curl -s http://127.0.0.1:8787/building/<a-real-bbl-from-above>
```
Expected: a Health Card JSON for a real Bed-Stuy building, with a real address, real violation counts, and a plausible score. Stop the server.

- [ ] **Step 5: Commit**
```bash
git add crates/ingest
git commit -m "feat(ingest): live Brooklyn ingest orchestration (--real)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: 311 + restaurant nearby context (P1) and snapshot-year wiring in the API

**Files:**
- Modify: `crates/ingest/src/run.rs`
- Modify: `crates/api/src/main.rs`

- [ ] **Step 1: 311 radius count.** In `run.rs`, after loading coords, fetch a Brooklyn 311 slice bounded to the CD's bounding box (min/max lat/lon of `coords`), parse `latitude`/`longitude`, and for each building set `complaints_311 = count of 311 points within 150 m` (use `haversine_m`). Cap the 311 pull with `$limit` and a recent date `$where` (e.g. `created_date > '2024-01-01'`) to keep it bounded. Print the total 311 points loaded.

- [ ] **Step 2: Restaurant grades (optional display).** Same pattern with DOHMH `43nn-pn8j`; store nearest-restaurant grade only if you add a column — otherwise skip and leave for the frontend to fetch. (If skipping, say so in the commit message; do not silently drop.)

- [ ] **Step 3: API reads snapshot year from the DB (fixes M6).** In `crates/api/src/main.rs`, replace the hardcoded `const SCORING_YEAR: i32 = 2026;` usage: load it once at startup from `store::get_snapshot_year(&conn)` (fall back to 2026 for the fixture DB, which has no `meta` row), store it in `AppState`, and pass it into `condition_score`. Add a test that a fixture-backed server still scores (snapshot defaults to 2026).

- [ ] **Step 4:** `cargo test --workspace` green; re-run the real ingest and confirm `complaints_311` is now non-zero for buildings near hotspots. Commit:
```bash
git add crates/ingest crates/api
git commit -m "feat(ingest): 311 nearby-context counts; api reads snapshot year

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage:** violations (HPD) ✅ Task 5; rent median (Census, sentinel-safe) ✅ Tasks 2/5; legal/stabilization — **deferred** (DHCR/WOW join is its own sub-task; `rent_stabilized` stays `None`/unknown until then — noted, not silent); accessibility elevator+build-era ✅ (elevator Task 5, build-era already in `scoring`); ADA-transit chip ✅ Task 5; 311 context ✅ Task 6; restaurants — optional/deferred with a note (Task 6 Step 2). Curated Brooklyn set ✅ (CD-filtered PLUTO).

**Deferred (intentional, noted in-plan):** DHCR/WOW rent-stabilization join; DOHMH restaurant grades display; full-NYC DuckDB scale.

**Placeholder scan:** none — pure functions have complete code + tests; live orchestration has exact URLs and run/verify commands. The one judgement call (`complaints_311` defaulting to 0 until Task 6) is explicitly flagged, not hidden.

**Type consistency:** `parse_pluto`→`model::Building`, `parse_hpd_violation`→`model::Violation`, and the new `store::upsert_building`/`insert_violation`/`upsert_tract_median`/`set_snapshot_year`/`get_snapshot_year` match the existing `store` signatures and `model` types used by `api`/`scoring`. No schema change beyond the additive `meta` table (migrate stays `IF NOT EXISTS`, so the fixture DB and CI smoke keep working).
