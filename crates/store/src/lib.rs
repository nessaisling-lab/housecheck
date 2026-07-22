use anyhow::Result;
use rusqlite::Connection;

/// Open a bundled-SQLite connection (":memory:" or a file path).
/// Creates the parent directory for a file path if it doesn't exist — SQLite
/// error 14 ("unable to open the database file") otherwise on a fresh checkout.
pub fn open_db(path: &str) -> Result<Connection> {
    if path != ":memory:" {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
    }
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
         );
         CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );",
    )?;
    Ok(())
}

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
        rent_stabilized: row
            .get::<_, Option<i64>>("rent_stabilized")?
            .map(|v| v != 0),
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
    let mut stmt =
        conn.prepare("SELECT class, open, year FROM violations WHERE bbl = ?1 AND open = 1")?;
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
    // `median_gross_rent > 0` filters out suppressed/sentinel ACS values (0 or negative
    // jam-values like -666666666) so they surface as "no data" rather than bad math.
    let mut stmt = conn.prepare(
        "SELECT median_gross_rent FROM acs_rent_by_tract
         WHERE tract_geoid = ?1 AND median_gross_rent > 0",
    )?;
    let mut rows = stmt.query_map([tract_geoid], |row| row.get::<_, i32>(0))?;
    match rows.next() {
        Some(v) => Ok(Some(v?)),
        None => Ok(None),
    }
}

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
        assert!(b.has_elevator);
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

    #[test]
    fn tract_median_ignores_suppressed_sentinel_values() -> Result<()> {
        let conn = seeded()?;
        // Census suppressed/sentinel medians must read as "no data", not a real number.
        conn.execute(
            "INSERT INTO acs_rent_by_tract (tract_geoid, median_gross_rent) VALUES ('36047999900', -666666666)",
            [],
        )?;
        conn.execute(
            "INSERT INTO acs_rent_by_tract (tract_geoid, median_gross_rent) VALUES ('36047999901', 0)",
            [],
        )?;
        assert_eq!(get_tract_median(&conn, "36047999900")?, None);
        assert_eq!(get_tract_median(&conn, "36047999901")?, None);
        Ok(())
    }

    #[test]
    fn upsert_building_and_reload() -> Result<()> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        let b = Building {
            bbl: "3018420001".into(),
            address: "123 Macon St".into(),
            year_built: 1910,
            num_floors: 3,
            units_res: 6,
            tract_geoid: "36047025300".into(),
            rent_stabilized: None,
            good_cause: false,
            has_elevator: true,
            near_ada_subway_m: Some(420),
            complaints_311: 7,
        };
        upsert_building(&conn, &b)?;
        assert_eq!(get_building(&conn, "3018420001")?.unwrap(), b);
        Ok(())
    }

    #[test]
    fn insert_violation_and_median_roundtrip() -> Result<()> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        insert_violation(
            &conn,
            "3018420001",
            &Violation {
                class: "C".into(),
                open: true,
                year: 2025,
            },
        )?;
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
}
