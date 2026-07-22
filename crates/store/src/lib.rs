use anyhow::Result;
use rusqlite::Connection;

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
