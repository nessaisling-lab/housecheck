# HouseCheck Backend Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the HouseCheck Rust backend that turns a Brooklyn BBL into a scored Building Health Card and a rent-fairness verdict, served over HTTP, proven with tests on a deterministic fixture dataset.

**Architecture:** A Cargo workspace of five focused crates — `model` (shared types), `scoring` (pure score functions), `store` (SQLite persistence), `ingest` (builds the DB), `api` (Axum HTTP). The serving DB is plain **bundled SQLite** (cross-platform, no native SpatiaLite); all geospatial joins are precomputed upstream (DuckDB ingest — a separate plan), so runtime queries are simple keyed lookups. Scoring is pure and deterministic (current year passed in, never read from the clock) so every score is unit-testable to an exact number.

**Tech Stack:** Rust (stable), `rusqlite` (bundled SQLite), `axum` 0.8 + `tokio`, `serde`/`serde_json`, `tower-http` (CORS/trace), `axum-test` for HTTP tests.

**Scope:** Backend core only. NOT in this plan: DuckDB ingest of real NYC Open Data (Plan 2), React frontend (Plan 3). This plan produces a running API answering from fixtures — the contract the frontend and real ingest both target.

---

## File Structure

```
housecheck/
├─ Cargo.toml                      # workspace manifest
├─ .gitignore
├─ crates/
│  ├─ model/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs                # Building, Violation, ScoreBreakdown, HealthCard, RentFairness
│  ├─ scoring/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs                # pure fns: condition/legal/neighborhood/accessibility/total/rent_fairness
│  ├─ store/
│  │  ├─ Cargo.toml
│  │  └─ src/lib.rs                # open_db, migrate, insert_fixture, get_building, get_open_violations, get_tract_median
│  ├─ ingest/
│  │  ├─ Cargo.toml
│  │  └─ src/main.rs               # `ingest --fixture --out <path>` builds a fixture DB
│  └─ api/
│     ├─ Cargo.toml
│     └─ src/main.rs               # Axum app: /health, /building/{bbl}, POST /rent-fairness
```

Boundaries: `scoring` has zero IO (trivial to test). `store` owns all SQL. `api` composes `store` + `scoring`. `model` is the shared vocabulary every crate speaks.

---

### Task 1: Workspace scaffold + git

**Files:**
- Create: `Cargo.toml`
- Create: `.gitignore`

- [ ] **Step 1: Create the workspace manifest**

`Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = ["crates/model", "crates/scoring", "crates/store", "crates/ingest", "crates/api"]

[workspace.package]
edition = "2021"
license = "MIT"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.32", features = ["bundled"] }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = "0.3"
axum-test = "17"
anyhow = "1"
```

- [ ] **Step 2: Create `.gitignore`**

`.gitignore`:
```gitignore
/target
**/*.rs.bk
.env
/data/*.db
/data/raw/
```

- [ ] **Step 3: Initialize git and commit**

Run:
```bash
git init
git add Cargo.toml .gitignore
git commit -m "chore: cargo workspace scaffold"
```
Expected: a repo with one commit. (`cargo build` fails until crates exist — that's fine, next task adds one.)

---

### Task 2: `model` crate — shared types

**Files:**
- Create: `crates/model/Cargo.toml`
- Create: `crates/model/src/lib.rs`

- [ ] **Step 1: Write the crate manifest**

`crates/model/Cargo.toml`:
```toml
[package]
name = "model"
version = "0.1.0"
edition.workspace = true

[dependencies]
serde.workspace = true
```

- [ ] **Step 2: Write the failing test (types + a helper)**

`crates/model/src/lib.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub bbl: String,
    pub address: String,
    pub year_built: i32,
    pub num_floors: i32,
    pub units_res: i32,
    pub tract_geoid: String,
    pub rent_stabilized: Option<bool>,
    pub good_cause: bool,
    pub has_elevator: bool,
    pub near_ada_subway_m: Option<i32>,
    pub complaints_311: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub class: String, // "A" | "B" | "C"
    pub open: bool,
    pub year: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViolationCounts {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl ViolationCounts {
    /// Count only OPEN violations by class.
    pub fn open_from(violations: &[Violation]) -> Self {
        let mut counts = ViolationCounts { a: 0, b: 0, c: 0 };
        for v in violations.iter().filter(|v| v.open) {
            match v.class.as_str() {
                "A" => counts.a += 1,
                "B" => counts.b += 1,
                "C" => counts.c += 1,
                _ => {}
            }
        }
        counts
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub total: u8,
    pub condition: u8,
    pub legal: u8,
    pub neighborhood: u8,
    pub accessibility: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCard {
    pub building: Building,
    pub score: ScoreBreakdown,
    pub open_violations: ViolationCounts,
    pub access_likelihood: String, // "Higher" | "Mixed" | "Lower"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RentFairness {
    pub bbl: String,
    pub user_rent: i32,
    pub tract_median: i32,
    pub pct_vs_median: f64,
    pub verdict: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_open_violations_by_class() {
        let vs = vec![
            Violation { class: "C".into(), open: true, year: 2025 },
            Violation { class: "C".into(), open: false, year: 2020 },
            Violation { class: "A".into(), open: true, year: 2024 },
        ];
        let counts = ViolationCounts::open_from(&vs);
        assert_eq!(counts, ViolationCounts { a: 1, b: 0, c: 1 });
    }
}
```

- [ ] **Step 3: Run the test — expect PASS**

Run: `cargo test -p model`
Expected: 1 passed. (The test and impl land together here because the type definitions are the unit.)

- [ ] **Step 4: Commit**

```bash
git add crates/model
git commit -m "feat(model): shared building/violation/score/card types"
```

---

### Task 3: `scoring` — condition sub-score (TDD)

**Files:**
- Create: `crates/scoring/Cargo.toml`
- Create: `crates/scoring/src/lib.rs`

- [ ] **Step 1: Write the crate manifest**

`crates/scoring/Cargo.toml`:
```toml
[package]
name = "scoring"
version = "0.1.0"
edition.workspace = true

[dependencies]
model = { path = "../model" }
```

- [ ] **Step 2: Write the failing test**

`crates/scoring/src/lib.rs`:
```rust
use model::Violation;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_recent_open_class_c_costs_30_points() {
        let vs = vec![Violation { class: "C".into(), open: true, year: 2026 }];
        // recent (within 2 yrs of 2026) C = 15 * 2 = 30 penalty -> 70
        assert_eq!(condition_score(&vs, 2026), 70);
    }

    #[test]
    fn closed_violations_are_ignored() {
        let vs = vec![Violation { class: "C".into(), open: false, year: 2026 }];
        assert_eq!(condition_score(&vs, 2026), 100);
    }

    #[test]
    fn penalty_clamps_at_zero() {
        let vs: Vec<Violation> = (0..20)
            .map(|_| Violation { class: "C".into(), open: true, year: 2026 })
            .collect();
        assert_eq!(condition_score(&vs, 2026), 0);
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p scoring condition`
Expected: FAIL — `cannot find function 'condition_score'`.

- [ ] **Step 4: Write minimal implementation**

Add above the `#[cfg(test)]` block in `crates/scoring/src/lib.rs`:
```rust
/// 0–100 building-condition score. Deterministic: `current_year` is passed in,
/// never read from the clock, so scores are testable and reproducible.
/// Open violations only. Severity: C=15, B=7, A=3. Recency (<=2 yrs) doubles it.
pub fn condition_score(violations: &[Violation], current_year: i32) -> u8 {
    let mut penalty: i32 = 0;
    for v in violations.iter().filter(|v| v.open) {
        let base = match v.class.as_str() {
            "C" => 15,
            "B" => 7,
            "A" => 3,
            _ => 0,
        };
        let recency = if current_year - v.year <= 2 { 2 } else { 1 };
        penalty += base * recency;
    }
    (100 - penalty).clamp(0, 100) as u8
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p scoring condition`
Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/scoring
git commit -m "feat(scoring): condition sub-score"
```

---

### Task 4: `scoring` — legal, neighborhood, accessibility, total, rent-fairness (TDD)

**Files:**
- Modify: `crates/scoring/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `mod tests` block in `crates/scoring/src/lib.rs`:
```rust
    use model::Building;

    fn base_building() -> Building {
        Building {
            bbl: "3000010001".into(),
            address: "1 Test St, Brooklyn".into(),
            year_built: 1960,
            num_floors: 5,
            units_res: 20,
            tract_geoid: "36047000100".into(),
            rent_stabilized: None,
            good_cause: false,
            has_elevator: false,
            near_ada_subway_m: None,
            complaints_311: 0,
        }
    }

    #[test]
    fn legal_rewards_stabilized_and_good_cause() {
        let mut b = base_building();
        assert_eq!(legal_score(&b), 60);
        b.rent_stabilized = Some(true);
        b.good_cause = true;
        assert_eq!(legal_score(&b), 100);
    }

    #[test]
    fn neighborhood_penalizes_311_density() {
        assert_eq!(neighborhood_score(0), 100);
        assert_eq!(neighborhood_score(10), 80);
        assert_eq!(neighborhood_score(100), 40); // capped penalty at 60
    }

    #[test]
    fn accessibility_elevator_is_higher() {
        let mut b = base_building();
        b.has_elevator = true;
        assert_eq!(access_likelihood(&b), (90, "Higher".to_string()));
    }

    #[test]
    fn accessibility_no_elevator_fha_era_is_mixed() {
        let mut b = base_building();
        b.has_elevator = false;
        b.num_floors = 5;
        b.year_built = 2000;
        b.units_res = 12;
        assert_eq!(access_likelihood(&b), (55, "Mixed".to_string()));
    }

    #[test]
    fn accessibility_walkup_lowrise_is_higher() {
        let mut b = base_building();
        b.has_elevator = false;
        b.num_floors = 2;
        assert_eq!(access_likelihood(&b), (75, "Higher".to_string()));
    }

    #[test]
    fn total_is_weighted_sum_rounded() {
        // 80,60,100,90 -> 80*.45+60*.20+100*.15+90*.20 = 36+12+15+18 = 81
        assert_eq!(total_score(80, 60, 100, 90), 81);
    }

    #[test]
    fn rent_fairness_flags_above_market() {
        let (pct, verdict) = rent_fairness(3000, 2500);
        assert_eq!(pct.round() as i32, 20);
        assert_eq!(verdict, "20% above neighborhood median");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p scoring`
Expected: FAIL — `legal_score`, `neighborhood_score`, `access_likelihood`, `total_score`, `rent_fairness` not found.

- [ ] **Step 3: Write minimal implementations**

Add above the `#[cfg(test)]` block in `crates/scoring/src/lib.rs`:
```rust
use model::Building;

/// 0–100 legal-protection score. Base 60; stabilized +25; Good Cause +15.
pub fn legal_score(b: &Building) -> u8 {
    let mut s: i32 = 60;
    if b.rent_stabilized == Some(true) {
        s += 25;
    }
    if b.good_cause {
        s += 15;
    }
    s.clamp(0, 100) as u8
}

/// 0–100 neighborhood score from 311 complaint count. Each complaint -2, capped -60.
pub fn neighborhood_score(complaints_311: i32) -> u8 {
    (100 - (complaints_311 * 2).min(60)).clamp(0, 100) as u8
}

/// Accessibility likelihood as (score, label). Elevator-on-record is the strongest
/// signal; otherwise infer from floors and FHA build-era. NOT a certification.
pub fn access_likelihood(b: &Building) -> (u8, String) {
    if b.has_elevator {
        return (90, "Higher".to_string());
    }
    if b.num_floors <= 2 {
        return (75, "Higher".to_string());
    }
    let fha_era = b.year_built >= 1992 && b.units_res >= 4;
    if fha_era {
        (55, "Mixed".to_string())
    } else {
        (30, "Lower".to_string())
    }
}

/// Weighted 0–100 total. Weights: condition .45, legal .20, neighborhood .15, accessibility .20.
pub fn total_score(condition: u8, legal: u8, neighborhood: u8, accessibility: u8) -> u8 {
    let t = condition as f64 * 0.45
        + legal as f64 * 0.20
        + neighborhood as f64 * 0.15
        + accessibility as f64 * 0.20;
    t.round().clamp(0.0, 100.0) as u8
}

/// Rent vs tract median: (pct difference, human verdict). >5% above, <-5% below, else "about at".
pub fn rent_fairness(user_rent: i32, tract_median: i32) -> (f64, String) {
    let pct = (user_rent - tract_median) as f64 / tract_median as f64 * 100.0;
    let verdict = if pct > 5.0 {
        format!("{:.0}% above neighborhood median", pct)
    } else if pct < -5.0 {
        format!("{:.0}% below neighborhood median", pct.abs())
    } else {
        "about at the neighborhood median".to_string()
    };
    (pct, verdict)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p scoring`
Expected: all passed (10 total).

- [ ] **Step 5: Commit**

```bash
git add crates/scoring
git commit -m "feat(scoring): legal, neighborhood, accessibility, total, rent-fairness"
```

---

### Task 5: `store` — open + schema migration (TDD)

**Files:**
- Create: `crates/store/Cargo.toml`
- Create: `crates/store/src/lib.rs`

- [ ] **Step 1: Write the crate manifest**

`crates/store/Cargo.toml`:
```toml
[package]
name = "store"
version = "0.1.0"
edition.workspace = true

[dependencies]
model = { path = "../model" }
rusqlite.workspace = true
anyhow.workspace = true
```

- [ ] **Step 2: Write the failing test**

`crates/store/src/lib.rs`:
```rust
use anyhow::Result;
use rusqlite::Connection;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrate_creates_expected_tables() -> Result<()> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        let count: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table'
             AND name IN ('buildings','violations','acs_rent_by_tract')",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 3);
        Ok(())
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p store migrate`
Expected: FAIL — `open_db` / `migrate` not found.

- [ ] **Step 4: Write minimal implementation**

Add above the `#[cfg(test)]` block in `crates/store/src/lib.rs`:
```rust
/// Open a bundled-SQLite connection (":memory:" or a file path).
pub fn open_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    Ok(conn)
}

/// Create the serving schema. Idempotent (IF NOT EXISTS).
pub fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS buildings (
            bbl TEXT PRIMARY KEY,
            address TEXT NOT NULL,
            year_built INTEGER NOT NULL,
            num_floors INTEGER NOT NULL,
            units_res INTEGER NOT NULL,
            tract_geoid TEXT NOT NULL,
            rent_stabilized INTEGER,          -- NULL unknown / 0 no / 1 yes
            good_cause INTEGER NOT NULL,
            has_elevator INTEGER NOT NULL,
            near_ada_subway_m INTEGER,
            complaints_311 INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS violations (
            id INTEGER PRIMARY KEY,
            bbl TEXT NOT NULL,
            class TEXT NOT NULL,
            open INTEGER NOT NULL,
            year INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_violations_bbl ON violations(bbl);
         CREATE TABLE IF NOT EXISTS acs_rent_by_tract (
            tract_geoid TEXT PRIMARY KEY,
            median_gross_rent INTEGER NOT NULL
         );",
    )?;
    Ok(())
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test -p store migrate`
Expected: 1 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/store
git commit -m "feat(store): open + schema migration"
```

---

### Task 6: `store` — fixtures + typed loaders (TDD)

**Files:**
- Modify: `crates/store/src/lib.rs`

- [ ] **Step 1: Write the failing tests**

Add inside the existing `mod tests` block in `crates/store/src/lib.rs`:
```rust
    use model::Building;

    fn seeded() -> Result<Connection> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        insert_fixture(&conn)?;
        Ok(conn)
    }

    #[test]
    fn fixture_building_loads_by_bbl() -> Result<()> {
        let conn = seeded()?;
        let b: Building = get_building(&conn, "3000010001")?.expect("building exists");
        assert_eq!(b.address, "1 Fixture Ave, Brooklyn");
        assert_eq!(b.has_elevator, true);
        Ok(())
    }

    #[test]
    fn missing_bbl_returns_none() -> Result<()> {
        let conn = seeded()?;
        assert!(get_building(&conn, "9999999999")?.is_none());
        Ok(())
    }

    #[test]
    fn open_violations_load_for_building() -> Result<()> {
        let conn = seeded()?;
        let vs = get_open_violations(&conn, "3000020002")?;
        assert!(vs.iter().all(|v| v.open));
        assert!(!vs.is_empty());
        Ok(())
    }

    #[test]
    fn tract_median_loads() -> Result<()> {
        let conn = seeded()?;
        assert_eq!(get_tract_median(&conn, "36047000100")?, Some(2500));
        Ok(())
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p store`
Expected: FAIL — `insert_fixture`, `get_building`, `get_open_violations`, `get_tract_median` not found.

- [ ] **Step 3: Write minimal implementation**

Add above the `#[cfg(test)]` block in `crates/store/src/lib.rs`:
```rust
use model::{Building, Violation};

/// Seed a small, deterministic fixture set (2 Brooklyn buildings + violations + tract rent).
/// Mirrors the shape the real DuckDB ingest will produce.
pub fn insert_fixture(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO acs_rent_by_tract (tract_geoid, median_gross_rent) VALUES ('36047000100', 2500)",
        [],
    )?;
    // Building 1: elevator, well-kept, stabilized.
    conn.execute(
        "INSERT INTO buildings VALUES ('3000010001','1 Fixture Ave, Brooklyn',1975,8,40,'36047000100',1,1,1,300,5)",
        [],
    )?;
    // Building 2: walk-up, open violations, no protections.
    conn.execute(
        "INSERT INTO buildings VALUES ('3000020002','2 Fixture Ave, Brooklyn',1930,4,8,'36047000100',NULL,0,0,NULL,40)",
        [],
    )?;
    conn.execute(
        "INSERT INTO violations (bbl,class,open,year) VALUES ('3000020002','C',1,2026)",
        [],
    )?;
    conn.execute(
        "INSERT INTO violations (bbl,class,open,year) VALUES ('3000020002','B',1,2025)",
        [],
    )?;
    conn.execute(
        "INSERT INTO violations (bbl,class,open,year) VALUES ('3000020002','A',0,2019)",
        [],
    )?;
    Ok(())
}

fn row_to_building(row: &rusqlite::Row) -> rusqlite::Result<Building> {
    Ok(Building {
        bbl: row.get("bbl")?,
        address: row.get("address")?,
        year_built: row.get("year_built")?,
        num_floors: row.get("num_floors")?,
        units_res: row.get("units_res")?,
        tract_geoid: row.get("tract_geoid")?,
        rent_stabilized: row.get::<_, Option<i64>>("rent_stabilized")?.map(|v| v != 0),
        good_cause: row.get::<_, i64>("good_cause")? != 0,
        has_elevator: row.get::<_, i64>("has_elevator")? != 0,
        near_ada_subway_m: row.get("near_ada_subway_m")?,
        complaints_311: row.get("complaints_311")?,
    })
}

pub fn get_building(conn: &Connection, bbl: &str) -> Result<Option<Building>> {
    let mut stmt = conn.prepare("SELECT * FROM buildings WHERE bbl = ?1")?;
    let mut rows = stmt.query_map([bbl], row_to_building)?;
    match rows.next() {
        Some(b) => Ok(Some(b?)),
        None => Ok(None),
    }
}

pub fn get_open_violations(conn: &Connection, bbl: &str) -> Result<Vec<Violation>> {
    let mut stmt = conn.prepare(
        "SELECT class, open, year FROM violations WHERE bbl = ?1 AND open = 1",
    )?;
    let rows = stmt.query_map([bbl], |row| {
        Ok(Violation {
            class: row.get("class")?,
            open: row.get::<_, i64>("open")? != 0,
            year: row.get("year")?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_tract_median(conn: &Connection, tract_geoid: &str) -> Result<Option<i32>> {
    let mut stmt =
        conn.prepare("SELECT median_gross_rent FROM acs_rent_by_tract WHERE tract_geoid = ?1")?;
    let mut rows = stmt.query_map([tract_geoid], |row| row.get::<_, i32>(0))?;
    match rows.next() {
        Some(v) => Ok(Some(v?)),
        None => Ok(None),
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p store`
Expected: all passed (5 total).

- [ ] **Step 5: Commit**

```bash
git add crates/store
git commit -m "feat(store): fixtures + typed loaders"
```

---

### Task 7: `ingest` binary — build a fixture DB file

**Files:**
- Create: `crates/ingest/Cargo.toml`
- Create: `crates/ingest/src/main.rs`

- [ ] **Step 1: Write the crate manifest**

`crates/ingest/Cargo.toml`:
```toml
[package]
name = "ingest"
version = "0.1.0"
edition.workspace = true

[dependencies]
store = { path = "../store" }
anyhow.workspace = true
```

- [ ] **Step 2: Write the binary**

`crates/ingest/src/main.rs`:
```rust
use anyhow::{bail, Result};

/// Usage: ingest --fixture --out <path>
/// Builds a serving DB. Only fixture mode exists in this plan; real DuckDB ingest is Plan 2.
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let fixture = args.iter().any(|a| a == "--fixture");
    let out = match args.iter().position(|a| a == "--out") {
        Some(i) => args.get(i + 1).cloned(),
        None => None,
    };
    let out = match out {
        Some(p) => p,
        None => bail!("missing --out <path>"),
    };
    if !fixture {
        bail!("only --fixture mode is implemented (see Plan 2 for real ingest)");
    }

    // Fresh file each run so builds are reproducible.
    let _ = std::fs::remove_file(&out);
    let conn = store::open_db(&out)?;
    store::migrate(&conn)?;
    store::insert_fixture(&conn)?;
    println!("built fixture DB at {out}");
    Ok(())
}
```

- [ ] **Step 3: Build and run it**

Run:
```bash
mkdir -p data
cargo run -p ingest -- --fixture --out data/housecheck.test.db
```
Expected: prints `built fixture DB at data/housecheck.test.db` and the file exists.

- [ ] **Step 4: Verify the DB is queryable**

Run:
```bash
cargo run -p ingest -- --fixture --out data/housecheck.test.db && echo OK
```
Expected: re-runs cleanly (removes + rebuilds), prints `OK`.

- [ ] **Step 5: Commit**

```bash
git add crates/ingest
git commit -m "feat(ingest): fixture DB builder"
```

---

### Task 8: `api` — `/health` (TDD)

**Files:**
- Create: `crates/api/Cargo.toml`
- Create: `crates/api/src/main.rs`

- [ ] **Step 1: Write the crate manifest**

`crates/api/Cargo.toml`:
```toml
[package]
name = "api"
version = "0.1.0"
edition.workspace = true

[[bin]]
name = "housecheck-api"
path = "src/main.rs"

[dependencies]
model = { path = "../model" }
scoring = { path = "../scoring" }
store = { path = "../store" }
axum.workspace = true
tokio.workspace = true
serde.workspace = true
serde_json.workspace = true
tower-http.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow.workspace = true

[dev-dependencies]
axum-test.workspace = true
```

- [ ] **Step 2: Write the failing test**

`crates/api/src/main.rs`:
```rust
use axum::{routing::get, Router};

/// Build the router. Takes a DB path so tests can point at an in-memory/fixture DB.
pub fn app() -> Router {
    Router::new().route("/health", get(|| async { "ok" }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;

    #[tokio::test]
    async fn health_returns_ok() {
        let server = TestServer::new(app()).unwrap();
        let res = server.get("/health").await;
        res.assert_status_ok();
        res.assert_text("ok");
    }
}
```

- [ ] **Step 3: Run test to verify it passes**

Run: `cargo test -p api health`
Expected: 1 passed. (Health is trivial enough that test+impl land together; the next tasks are strict red→green.)

- [ ] **Step 4: Commit**

```bash
git add crates/api
git commit -m "feat(api): health endpoint + app router"
```

---

### Task 9: `api` — `GET /building/{bbl}` returns a Health Card (TDD)

**Files:**
- Modify: `crates/api/src/main.rs`

- [ ] **Step 1: Write the failing test**

In `crates/api/src/main.rs`, replace the `#[cfg(test)] mod tests { ... }` block with:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum_test::TestServer;
    use model::HealthCard;

    fn test_server() -> TestServer {
        // Shared in-memory DB seeded with fixtures, wrapped in the app state.
        let state = AppState::in_memory_fixture().unwrap();
        TestServer::new(app_with_state(state)).unwrap()
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let server = test_server();
        let res = server.get("/health").await;
        res.assert_status_ok();
        res.assert_text("ok");
    }

    #[tokio::test]
    async fn building_returns_scored_card() {
        let server = test_server();
        let res = server.get("/building/3000020002").await;
        res.assert_status_ok();
        let card: HealthCard = res.json();
        assert_eq!(card.building.bbl, "3000020002");
        assert!(card.score.total <= 100);
        // walk-up with open C+B violations -> some open violations present
        assert!(card.open_violations.c >= 1);
        assert_eq!(card.access_likelihood, "Lower"); // 1930 walk-up, 4 floors, pre-FHA
    }

    #[tokio::test]
    async fn unknown_building_is_404() {
        let server = test_server();
        let res = server.get("/building/9999999999").await;
        res.assert_status_not_found();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p api building`
Expected: FAIL — `AppState`, `app_with_state` not found.

- [ ] **Step 3: Write minimal implementation**

Replace the top of `crates/api/src/main.rs` (everything above the `#[cfg(test)]` block) with:
```rust
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use std::sync::{Arc, Mutex};

use model::{HealthCard, ScoreBreakdown, ViolationCounts};
use store::{get_building, get_open_violations, get_tract_median};

/// Shared app state: a single SQLite connection behind a mutex.
/// (Read-mostly reference data + a curated set → a single connection is fine for the MVP.)
#[derive(Clone)]
pub struct AppState {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl AppState {
    pub fn from_path(path: &str) -> anyhow::Result<Self> {
        let conn = store::open_db(path)?;
        store::migrate(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }

    /// In-memory DB seeded with fixtures — used by tests.
    pub fn in_memory_fixture() -> anyhow::Result<Self> {
        let conn = store::open_db(":memory:")?;
        store::migrate(&conn)?;
        store::insert_fixture(&conn)?;
        Ok(Self { conn: Arc::new(Mutex::new(conn)) })
    }
}

/// Year used for recency in scoring. Centralized so it's the single place to bump.
const SCORING_YEAR: i32 = 2026;

pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/building/{bbl}", get(building_handler))
        .with_state(state)
}

/// Back-compat helper for the `main` fn / simplest tests.
pub fn app() -> Router {
    let state = AppState::in_memory_fixture().expect("fixture state");
    app_with_state(state)
}

async fn building_handler(
    State(state): State<AppState>,
    Path(bbl): Path<String>,
) -> impl IntoResponse {
    let conn = state.conn.lock().unwrap();

    let building = match get_building(&conn, &bbl) {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "building not found").into_response(),
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };
    let violations = match get_open_violations(&conn, &bbl) {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };

    let condition = scoring::condition_score(&violations, SCORING_YEAR);
    let legal = scoring::legal_score(&building);
    let neighborhood = scoring::neighborhood_score(building.complaints_311);
    let (accessibility, access_likelihood) = scoring::access_likelihood(&building);
    let total = scoring::total_score(condition, legal, neighborhood, accessibility);

    let card = HealthCard {
        open_violations: ViolationCounts::open_from(&violations),
        score: ScoreBreakdown { total, condition, legal, neighborhood, accessibility },
        access_likelihood,
        building,
    };
    (StatusCode::OK, Json(card)).into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let db = std::env::var("HOUSECHECK_DB").unwrap_or_else(|_| "data/housecheck.db".to_string());
    let state = AppState::from_path(&db)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app_with_state(state)).await?;
    Ok(())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p api`
Expected: all passed (health + building_returns_scored_card + unknown_building_is_404).

- [ ] **Step 5: Commit**

```bash
git add crates/api
git commit -m "feat(api): GET /building/{bbl} returns scored Health Card"
```

---

### Task 10: `api` — `POST /rent-fairness` (TDD)

**Files:**
- Modify: `crates/api/src/main.rs`

- [ ] **Step 1: Write the failing test**

Add inside `mod tests` in `crates/api/src/main.rs`:
```rust
    use model::RentFairness;
    use serde_json::json;

    #[tokio::test]
    async fn rent_fairness_returns_pct_vs_median() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "3000010001", "monthly_rent": 3000}))
            .await;
        res.assert_status_ok();
        let rf: RentFairness = res.json();
        assert_eq!(rf.tract_median, 2500);
        assert_eq!(rf.pct_vs_median.round() as i32, 20);
        assert!(rf.verdict.contains("above"));
    }

    #[tokio::test]
    async fn rent_fairness_rejects_nonpositive_rent() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "3000010001", "monthly_rent": 0}))
            .await;
        res.assert_status_bad_request();
    }

    #[tokio::test]
    async fn rent_fairness_unknown_bbl_is_404() {
        let server = test_server();
        let res = server
            .post("/rent-fairness")
            .json(&json!({"bbl": "9999999999", "monthly_rent": 3000}))
            .await;
        res.assert_status_not_found();
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p api rent_fairness`
Expected: FAIL — no `/rent-fairness` route (404 where 200/400 expected, and handler missing).

- [ ] **Step 3: Write minimal implementation**

In `crates/api/src/main.rs`, add `Deserialize` to imports and the route + handler.

Change the serde import line to:
```rust
use serde::Deserialize;
```
Add `.route("/rent-fairness", axum::routing::post(rent_fairness_handler))` to `app_with_state` (before `.with_state(state)`):
```rust
pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/building/{bbl}", get(building_handler))
        .route("/rent-fairness", axum::routing::post(rent_fairness_handler))
        .with_state(state)
}
```
Add the request type and handler above the `#[tokio::main]` fn:
```rust
#[derive(Deserialize)]
struct RentFairnessReq {
    bbl: String,
    monthly_rent: i32,
}

async fn rent_fairness_handler(
    State(state): State<AppState>,
    Json(req): Json<RentFairnessReq>,
) -> impl IntoResponse {
    if req.monthly_rent <= 0 {
        return (StatusCode::BAD_REQUEST, "monthly_rent must be positive").into_response();
    }
    let conn = state.conn.lock().unwrap();
    let building = match get_building(&conn, &req.bbl) {
        Ok(Some(b)) => b,
        Ok(None) => return (StatusCode::NOT_FOUND, "building not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let median = match get_tract_median(&conn, &building.tract_geoid) {
        Ok(Some(m)) => m,
        Ok(None) => return (StatusCode::NOT_FOUND, "no rent data for tract").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let (pct, verdict) = scoring::rent_fairness(req.monthly_rent, median);
    let body = model::RentFairness {
        bbl: req.bbl,
        user_rent: req.monthly_rent,
        tract_median: median,
        pct_vs_median: pct,
        verdict,
    };
    (StatusCode::OK, Json(body)).into_response()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p api`
Expected: all passed (6 total).

- [ ] **Step 5: Commit**

```bash
git add crates/api
git commit -m "feat(api): POST /rent-fairness"
```

---

### Task 11: Middleware + end-to-end smoke

**Files:**
- Modify: `crates/api/src/main.rs`

- [ ] **Step 1: Add CORS + tracing middleware**

In `crates/api/src/main.rs`, add imports:
```rust
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
```
Extend `app_with_state` to attach the layers (after the routes, before/around `.with_state`):
```rust
pub fn app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/building/{bbl}", get(building_handler))
        .route("/rent-fairness", axum::routing::post(rent_fairness_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive()) // MVP: tighten to the Vercel origin before launch
        .with_state(state)
}
```

- [ ] **Step 2: Verify all tests still pass with middleware**

Run: `cargo test --workspace`
Expected: all crates green (model, scoring, store, api).

- [ ] **Step 3: Manual end-to-end smoke**

Run (two terminals, or background the server):
```bash
cargo run -p ingest -- --fixture --out data/housecheck.db
HOUSECHECK_DB=data/housecheck.db cargo run -p api &
sleep 2
curl -s http://127.0.0.1:8787/health
curl -s http://127.0.0.1:8787/building/3000020002
curl -s -X POST http://127.0.0.1:8787/rent-fairness \
  -H 'content-type: application/json' \
  -d '{"bbl":"3000010001","monthly_rent":3000}'
```
Expected: `ok`; a Health Card JSON with a `score.total`; a rent-fairness JSON with `"pct_vs_median": 20.0`.

- [ ] **Step 4: Confirm formatting + lint are clean**

Run:
```bash
cargo fmt --all
cargo clippy --workspace --all-targets
```
Expected: no changes needed / no warnings. Fix any clippy findings, re-run.

- [ ] **Step 5: Commit**

```bash
git add crates/api
git commit -m "feat(api): CORS + tracing middleware; backend core complete"
```

---

## Self-Review

**Spec coverage** (against `docs/superpowers/specs/2026-07-21-housecheck-design.md`):
- 0–100 weighted score with sub-scores → Tasks 3–4, 9. ✅
- Violation condition scoring (A/B/C, recency) → Task 3. ✅
- Legal protections (stabilized, Good Cause) → Task 4, in card via Task 9. ✅
- Rent fairness (user rent vs tract median) → Task 4 + Task 10. ✅ (HUD FMR layer deferred — noted below.)
- Accessibility likelihood (elevator + build-era) → Task 4, in card via Task 9. ✅
- Neighborhood (311 density) → Task 4/9. ✅
- Cross-platform serving (plain SQLite) → Task 1/5. ✅
- `/health`, `/building/{bbl}`, `/rent-fairness` API contract → Tasks 8–10. ✅ (matches `smoke.yml`.)

**Deferred to later plans (intentional, not gaps):**
- Real DuckDB ingest of NYC Open Data (HPD/311/DOHMH/PLUTO/ACS/DOB elevators) → **Plan 2**. This plan uses fixtures with the identical schema, so `api`/`scoring` need no change when real data lands.
- HUD Fair Market Rent as a second rent comparator → add alongside `get_tract_median` in Plan 2.
- `/search?address=` + GeoSearch live fallback → Plan 2 (curated set resolves by BBL directly for now).
- Frontend → **Plan 3**.

**Placeholder scan:** none — every code step is complete and runnable.

**Type consistency:** `Building`, `Violation`, `ViolationCounts`, `ScoreBreakdown`, `HealthCard`, `RentFairness` defined once in `model` (Task 2) and used verbatim in `store` (6), `scoring` (3–4), `api` (9–10). Function names stable: `condition_score`, `legal_score`, `neighborhood_score`, `access_likelihood`, `total_score`, `rent_fairness`, `get_building`, `get_open_violations`, `get_tract_median`. The API `SCORING_YEAR` (2026) matches the recency tests' `current_year`.
