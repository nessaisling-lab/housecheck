use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags, OptionalExtension};

use model::{UnknownViolationClass, ViolationClass};

/// Read a `class` column into [`ViolationClass`].
///
/// `model` deliberately depends on exactly one crate and must not learn about SQLite, so the
/// conversion lives here — at the edge that actually touches a database — rather than as a
/// `FromSql` impl on the type.
///
/// An unmodelled letter becomes a rusqlite error carrying the offending value. That is louder
/// than the behaviour it replaces, where an unknown class flowed through as a `String` and was
/// then priced at zero by `scoring` and counted as nothing by `ViolationCounts`.
fn parse_class(row: &rusqlite::Row<'_>, column: &str) -> rusqlite::Result<ViolationClass> {
    let raw: String = row.get(column)?;
    raw.parse().map_err(|e: UnknownViolationClass| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

/// Open the serving artifact **read-only**, for the API.
///
/// `open_db` below is read-write with create-if-missing, which is correct for `ingest` — it
/// has to create the file — and wrong for the server, which must never create or modify one.
/// Pointing the API at a missing path used to succeed: it created the directory, created an
/// empty database, built the schema, and then served `/health` → `ok` with a 404 for every
/// building. A green deploy with no data, and nothing logged.
///
/// Read-only makes that state unrepresentable rather than merely detectable. A missing file
/// fails here, once, at startup, instead of silently at every request forever after.
pub fn open_db_readonly(path: &str) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
    .with_context(|| format!("open {path} read-only (the serving artifact must already exist)"))
}

/// Number of buildings in the artifact. The API asserts this is non-zero at startup: a
/// present-but-empty database is the one bad state `open_db_readonly` cannot rule out.
pub fn building_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM buildings", [], |r| r.get(0))?)
}

/// Open a bundled-SQLite connection (":memory:" or a file path) for **writing**.
/// Creates the parent directory for a file path if it doesn't exist — SQLite
/// error 14 ("unable to open the database file") otherwise on a fresh checkout.
/// Used by `ingest` and by the in-memory test fixtures; the API uses
/// [`open_db_readonly`].
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
        "-- BBLs are stored as integers: ten digits as text cost 11 bytes a row, as an
         -- integer 6, and the citywide artifact repeats them 10.5M times. Callers still pass
         -- and receive the 10-digit string; SQLite's integer affinity converts on the way in.
         CREATE TABLE IF NOT EXISTS buildings (
            bbl INTEGER PRIMARY KEY,
            address TEXT NOT NULL,
            year_built INTEGER NOT NULL,
            num_floors INTEGER NOT NULL,
            units_res INTEGER NOT NULL,
            tract_geoid TEXT NOT NULL,
            rent_stabilized INTEGER,          -- NULL unknown / 0 no / 1 yes
            -- Rent-stabilized units on the latest NYC DOF Statement-of-Account record
            -- (JustFix nyc-doffer 2024). NULL = no DOF record found for the building.
            rent_stab_units INTEGER,
            good_cause INTEGER NOT NULL,
            has_elevator INTEGER NOT NULL,
            near_ada_subway_m INTEGER,
            zip TEXT,
            neighborhood TEXT,
            complaints_311_per_1k_homes INTEGER,
            latitude REAL,
            longitude REAL
         );
         -- Clustered by building (WITHOUT ROWID, keyed on bbl then seq), so one building's
         -- rows sit together on disk and no separate bbl index is needed. Measured on 115,273
         -- real rows with the description blocks below: 105 bytes a row in the old layout
         -- (text dates, rowid, bbl index, zlib), 36 in this one.
         CREATE TABLE IF NOT EXISTS violations (
            bbl INTEGER NOT NULL,
            seq INTEGER NOT NULL,             -- order within the building, as HPD listed them
            class TEXT NOT NULL,
            open INTEGER NOT NULL,
            year INTEGER NOT NULL,
            -- Index into this building's compressed description block, or NULL when the
            -- record carries no text. The text itself lives in violation_desc.
            desc_ord INTEGER,
            -- Days since 1970-01-01, converted by SQLite on the way in and back to ISO dates
            -- on the way out, so callers only ever see `YYYY-MM-DD`. issued_on is absent on
            -- ~7% of citywide rows, so a violation's age is genuinely unknown for those
            -- rather than zero.
            issued_on INTEGER,
            closed_on INTEGER,
            PRIMARY KEY (bbl, seq)
         ) WITHOUT ROWID;
         -- Violation descriptions, compressed one block per building.
         --
         -- Stored this way because the redundancy is BETWEEN notices -- every one opens
         -- with a housing-code statute reference -- not inside a single one. Measured on
         -- the pilot's 5,354 real open violations: 1.3x compressing each row alone, 7.0x
         -- compressing a building's together. The block is the building because a card
         -- reads exactly one building, so a lookup decompresses exactly what it needs and
         -- no block index is required.
         --
         -- zstd against a dictionary trained on HPD's own wording. Alone, zstd measured the
         -- same as zlib here (57.6 vs 58.4 bytes a row): a small building's block is too
         -- short to learn the statute prefixes every notice repeats. The dictionary already
         -- knows them, which took descriptions to 10.4 bytes a row.
         CREATE TABLE IF NOT EXISTS violation_desc (
            bbl INTEGER PRIMARY KEY,
            z BLOB NOT NULL
         );
         -- The dictionary every violation_desc block was compressed with, versioned with
         -- the artifact by living inside it. Absent when there was too little text to train
         -- one (the fixture), in which case blocks are plain zstd.
         CREATE TABLE IF NOT EXISTS desc_dict (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            dict BLOB NOT NULL
         );
         -- What each source actually gave us, and when.
         --
         -- This is what makes a signed export attest to a FACT rather than to a FILE. A
         -- hash chain proves nobody edited our output; it says nothing about whether the
         -- output matched HPD at the time. Without the dataset id and the retrieval
         -- timestamp, an exhibit is only evidence about itself.
         --
         -- Per dataset rather than per row, deliberately: this artifact is one full
         -- snapshot, so every row from a dataset shares a retrieval time and 2.8M copies
         -- of one timestamp would be waste. Incremental refresh (see
         -- docs/design/database-layer.md) breaks that assumption and will need per-batch
         -- stamping, which is why `note` carries the query that produced the rows.
         CREATE TABLE IF NOT EXISTS source_provenance (
            dataset TEXT PRIMARY KEY,
            retrieved_at_unix INTEGER NOT NULL,
            row_count INTEGER NOT NULL,
            note TEXT
         );
         CREATE TABLE IF NOT EXISTS acs_rent_by_tract (
            tract_geoid TEXT PRIMARY KEY,
            median_gross_rent INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
         );
         -- The law the agent may state, loaded from data/law.json by `load_law`. The CHECKs
         -- are the vocabulary: a typo in the seed fails the build instead of shipping a
         -- provision no query will ever match.
         CREATE TABLE IF NOT EXISTS law_source (
            id TEXT PRIMARY KEY,
            publisher TEXT NOT NULL,
            jurisdiction TEXT NOT NULL CHECK (jurisdiction IN ('ny_state','nyc','federal')),
            kind TEXT NOT NULL CHECK (kind IN ('statute','regulation','agency_guidance')),
            base_url TEXT NOT NULL CHECK (base_url LIKE 'https://%')
         );
         CREATE TABLE IF NOT EXISTS law_provision (
            id TEXT PRIMARY KEY,
            source_id TEXT NOT NULL REFERENCES law_source(id),
            citation TEXT NOT NULL,
            title TEXT NOT NULL,
            text TEXT NOT NULL,
            url TEXT NOT NULL CHECK (url LIKE 'https://%'),
            retrieved_at TEXT NOT NULL CHECK (retrieved_at GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            text_sha256 TEXT NOT NULL,
            verified_by TEXT NOT NULL CHECK (length(verified_by) > 0),
            verified_on TEXT NOT NULL CHECK (verified_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            status TEXT NOT NULL CHECK (status IN ('current','superseded')),
            occupancy TEXT NOT NULL CHECK (occupancy IN ('residential','commercial','both'))
         );
         CREATE TABLE IF NOT EXISTS law_topic (
            provision_id TEXT NOT NULL REFERENCES law_provision(id),
            topic TEXT NOT NULL,
            PRIMARY KEY (provision_id, topic)
         );
         CREATE TABLE IF NOT EXISTS law_fact (
            key TEXT PRIMARY KEY,
            provision_id TEXT NOT NULL REFERENCES law_provision(id),
            value TEXT NOT NULL,
            condition TEXT NOT NULL,
            quote TEXT NOT NULL
         );
         -- A provision's text cut at its own subdivisions (`model::law::passages`). Derived at
         -- load, never edited: joined in seq order the rows are the provision's text exactly.
         CREATE TABLE IF NOT EXISTS law_passage (
            provision_id TEXT NOT NULL REFERENCES law_provision(id),
            seq INTEGER NOT NULL,
            label TEXT NOT NULL,
            text TEXT NOT NULL,
            PRIMARY KEY (provision_id, seq)
         );
         CREATE VIRTUAL TABLE IF NOT EXISTS law_passage_fts USING fts5(
            text, provision_id UNINDEXED, seq UNINDEXED, tokenize = 'porter unicode61'
         );
         -- find_legal_help's directory, served in rowid order. Only free services belong
         -- here: a paid referral to someone in a housing crisis is the harm it exists to avoid.
         CREATE TABLE IF NOT EXISTS legal_help (
            name TEXT PRIMARY KEY,
            what TEXT NOT NULL,
            phone TEXT,
            hours TEXT,
            url TEXT NOT NULL CHECK (url LIKE 'https://%'),
            free INTEGER NOT NULL CHECK (free = 1),
            excerpt TEXT NOT NULL,
            retrieved_at TEXT NOT NULL CHECK (retrieved_at GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'),
            verified_by TEXT NOT NULL CHECK (length(verified_by) > 0),
            verified_on TEXT NOT NULL CHECK (verified_on GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]')
         );",
    )?;
    // ponytail: no column-by-column upgrades. Every artifact is rebuilt from scratch by
    // `ingest`, so an older file is replaced rather than altered.
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
    // Building 1: elevator, well-kept, stabilized. Both fixture ZIP rates sit under the curve's
    // zero-penalty line (677), quieter than any real ZIP, so the fixture's neighborhood score
    // stays 100 as it was under the old 311 count and its totals do not move.
    conn.execute(
        "INSERT INTO buildings
          (bbl,address,year_built,num_floors,units_res,tract_geoid,rent_stabilized,good_cause,has_elevator,near_ada_subway_m,zip,neighborhood,complaints_311_per_1k_homes,latitude,longitude,rent_stab_units)
         VALUES ('3000010001','1 Fixture Ave, Brooklyn',1975,8,40,'36047000100',1,1,1,300,'11221','Bedford-Stuyvesant (East)',600,40.6829,-73.9251,12)",
        [],
    )?;
    // Building 2: walk-up, open violations, no protections.
    conn.execute(
        "INSERT INTO buildings
          (bbl,address,year_built,num_floors,units_res,tract_geoid,rent_stabilized,good_cause,has_elevator,near_ada_subway_m,zip,neighborhood,complaints_311_per_1k_homes,latitude,longitude,rent_stab_units)
         VALUES ('3000020002','2 Fixture Ave, Brooklyn',1930,4,8,'36047000100',NULL,0,0,NULL,'11221','Bedford-Stuyvesant (East)',600,40.6835,-73.9240,NULL)",
        [],
    )?;
    // HPD's own notice wording, copied from real CD 303 notices in the 2026-10-30 pilot ingest
    // and cut before the apartment location, so `cited_law` runs end to end on the fixture.
    let notice = |class, year, text: &str| Violation {
        class,
        open: true,
        year,
        description: Some(text.to_string()),
        issued_on: None,
        closed_on: None,
    };
    insert_violations(
        conn,
        "3000020002",
        &[
            notice(
                model::ViolationClass::C,
                2026,
                "§ 27-2056.6 ADM CODE - CORRECT THE LEAD-BASED PAINT HAZARD - PRESUMED LEAD PAINT \
                 THAT IS PEELING OR ON A DETERIORATED SUBSURFACE USING WORK PRACTICES SET FORTH IN \
                 28 RCNY §11-06(B)(2) CEILING IN THE 1st ROOM FROM NORTH",
            ),
            notice(
                model::ViolationClass::B,
                2025,
                "§ 27-2005 HMC:TRACE AND REPAIR THE SOURCE AND ABATE THE NUISANCE CONSISTING OF \
                 MOLD ... AT SOUTH WALL APPROX 10 SQ. FT. IN THE KITCHEN",
            ),
        ],
    )?;
    conn.execute(
        "INSERT INTO violations (bbl,seq,class,open,year) VALUES ('3000020002',2,'A',0,2019)",
        [],
    )?;
    // The real law, not a fixture of it: the evals check answers against these rows.
    load_law(conn)?;
    Ok(())
}

fn row_to_building(row: &rusqlite::Row) -> rusqlite::Result<Building> {
    Ok(Building {
        bbl: row.get::<_, i64>("bbl")?.to_string(),
        address: row.get("address")?,
        year_built: row.get("year_built")?,
        num_floors: row.get("num_floors")?,
        units_res: row.get("units_res")?,
        tract_geoid: row.get("tract_geoid")?,
        rent_stabilized: row
            .get::<_, Option<i64>>("rent_stabilized")?
            .map(|v| v != 0),
        rent_stab_units: row.get("rent_stab_units")?,
        good_cause: row.get::<_, i64>("good_cause")? != 0,
        has_elevator: row.get::<_, i64>("has_elevator")? != 0,
        near_ada_subway_m: row.get("near_ada_subway_m")?,
        zip: row.get("zip")?,
        neighborhood: row.get("neighborhood")?,
        complaints_311_per_1k_homes: row.get("complaints_311_per_1k_homes")?,
        latitude: row.get("latitude")?,
        longitude: row.get("longitude")?,
    })
}

/// Every building, for the `/buildings` list/map endpoint. Ordered by BBL for a stable list.
pub fn get_all_buildings(conn: &Connection) -> Result<Vec<Building>> {
    let mut stmt = conn.prepare("SELECT * FROM buildings ORDER BY bbl")?;
    let rows = stmt.query_map([], row_to_building)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_building(conn: &Connection, bbl: &str) -> Result<Option<Building>> {
    let mut stmt = conn.prepare("SELECT * FROM buildings WHERE bbl = ?1")?;
    let mut rows = stmt.query_map([bbl], row_to_building)?;
    match rows.next() {
        Some(b) => Ok(Some(b?)),
        None => Ok(None),
    }
}

/// Days from issue to close for this building's recently-closed violations.
///
/// Returns the raw durations rather than a median, so the statistic stays in `model` where it
/// is a pure function and can be tested without a database. This function's only job is to ask
/// the right rows for them.
///
/// Dates are stored as days since 1970, so a duration is a subtraction and the cutoff is the
/// same day number SQLite computes for `YYYY-01-01`. Rows missing either date are excluded:
/// measured on the pilot, 21,173 of 21,175 closed violations carry both, so this discards two
/// rows rather than skewing anything.
pub fn closed_violation_durations(
    conn: &Connection,
    bbl: &str,
    since_year: i32,
) -> Result<Vec<i64>> {
    let cutoff = format!("{since_year:04}-01-01");
    let mut stmt = conn.prepare_cached(
        "SELECT closed_on - issued_on
           FROM violations
          WHERE bbl = ?1
            AND open = 0
            AND issued_on IS NOT NULL
            AND closed_on IS NOT NULL
            AND closed_on >= unixepoch(?2) / 86400",
    )?;
    let rows = stmt.query_map(rusqlite::params![bbl, cutoff], |r| r.get::<_, i64>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn get_open_violations(conn: &Connection, bbl: &str) -> Result<Vec<Violation>> {
    // `date(n * 86400, 'unixepoch')` turns the stored day number back into the exact
    // `YYYY-MM-DD` string the ingest was given, so nothing above the store sees the change.
    let mut stmt = conn.prepare_cached(
        "SELECT class, open, year, desc_ord,
                date(issued_on * 86400, 'unixepoch') AS issued_on,
                date(closed_on * 86400, 'unixepoch') AS closed_on
           FROM violations WHERE bbl = ?1 AND open = 1 ORDER BY seq",
    )?;
    // One decompress for the whole building, before the row loop rather than inside it.
    let descriptions = read_descriptions(conn, bbl)?;
    let rows = stmt.query_map([bbl], |row| {
        let ord: Option<i64> = row.get("desc_ord")?;
        Ok(Violation {
            // The schema is `class TEXT NOT NULL` with no CHECK, so the column can hold any
            // string; the invariant is upheld by `ingest`, which counts and skips anything
            // outside A/B/C. Parsing here is where that promise is checked rather than assumed.
            // A failure means the artifact was built by something that did not uphold it, which
            // is worth surfacing as an error instead of quietly scoring the row at zero — the
            // behaviour this whole change exists to remove.
            class: parse_class(row, "class")?,
            open: row.get::<_, i64>("open")? != 0,
            year: row.get("year")?,
            description: ord
                .and_then(|o| usize::try_from(o).ok())
                .and_then(|o| descriptions.get(o).cloned()),
            issued_on: row.get("issued_on")?,
            closed_on: row.get("closed_on")?,
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
          (bbl,address,year_built,num_floors,units_res,tract_geoid,rent_stabilized,good_cause,has_elevator,near_ada_subway_m,zip,neighborhood,complaints_311_per_1k_homes,latitude,longitude,rent_stab_units)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)
         ON CONFLICT(bbl) DO UPDATE SET
          address=excluded.address, year_built=excluded.year_built, num_floors=excluded.num_floors,
          units_res=excluded.units_res, tract_geoid=excluded.tract_geoid, rent_stabilized=excluded.rent_stabilized,
          good_cause=excluded.good_cause, has_elevator=excluded.has_elevator,
          near_ada_subway_m=excluded.near_ada_subway_m, zip=excluded.zip,
          neighborhood=excluded.neighborhood,
          complaints_311_per_1k_homes=excluded.complaints_311_per_1k_homes,
          latitude=excluded.latitude, longitude=excluded.longitude,
          rent_stab_units=excluded.rent_stab_units",
        rusqlite::params![
            b.bbl, b.address, b.year_built, b.num_floors, b.units_res, b.tract_geoid,
            b.rent_stabilized.map(|v| v as i64), b.good_cause as i64, b.has_elevator as i64,
            b.near_ada_subway_m, b.zip, b.neighborhood, b.complaints_311_per_1k_homes,
            b.latitude, b.longitude, b.rent_stab_units
        ],
    )?;
    Ok(())
}

/// Descriptions are joined with a byte that cannot appear in HPD's text, so splitting is
/// unambiguous. A newline would not be safe -- a notice may contain one.
const RECORD_SEP: char = '\u{1e}'; // ASCII record separator

/// The artifact's description dictionary, if it has one.
fn desc_dict(conn: &Connection) -> Result<Option<Vec<u8>>> {
    use rusqlite::OptionalExtension;
    Ok(conn
        .query_row("SELECT dict FROM desc_dict WHERE id = 1", [], |r| r.get(0))
        .optional()?)
}

/// Descriptions for one building, in `desc_ord` order. Empty when the building has none.
///
/// ponytail: reads and loads the 112 KB dictionary on every call. Decompressing a building's
/// block measured 2.3 µs; if loading the dictionary ever shows in card latency, keep one
/// `zstd::dict::DecoderDictionary` per connection instead.
fn read_descriptions(conn: &Connection, bbl: &str) -> Result<Vec<String>> {
    let z: Option<Vec<u8>> = conn
        .query_row("SELECT z FROM violation_desc WHERE bbl = ?1", [bbl], |r| {
            r.get(0)
        })
        .optional()?;
    let Some(z) = z else {
        return Ok(Vec::new());
    };
    let dict = desc_dict(conn)?.unwrap_or_default();
    let mut out = String::new();
    std::io::Read::read_to_string(
        &mut zstd::stream::read::Decoder::with_dictionary(&z[..], &dict)?,
        &mut out,
    )?;
    // split, not lines(): desc_ord indexes into this vector, so positions must be exact.
    Ok(out.split(RECORD_SEP).map(str::to_string).collect())
}

/// zstd level for description blocks. Ingest-time cost only; reading is the same speed at
/// any level.
const DESC_LEVEL: i32 = 19;

/// Train the description dictionary from sample notices. `None` when there is too little text
/// to train one, which leaves blocks as plain zstd.
pub fn train_desc_dict(samples: &[String]) -> Option<Vec<u8>> {
    zstd::dict::from_samples(samples, 112 * 1024).ok()
}

/// Store the dictionary. Must happen before any [`ViolationWriter`] is made, since blocks are
/// only readable with the dictionary they were written with.
pub fn set_desc_dict(conn: &Connection, dict: &[u8]) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO desc_dict (id, dict) VALUES (1, ?1)",
        [dict],
    )?;
    Ok(())
}

/// Writes violations, holding one zstd compressor with the artifact's dictionary loaded.
/// Loading a dictionary is the slow part of compressing a small block, so the ingest makes
/// one of these and reuses it for every building.
pub struct ViolationWriter {
    z: zstd::bulk::Compressor<'static>,
}

impl ViolationWriter {
    pub fn new(conn: &Connection) -> Result<Self> {
        let z = match desc_dict(conn)? {
            Some(d) => zstd::bulk::Compressor::with_dictionary(DESC_LEVEL, &d)?,
            None => zstd::bulk::Compressor::new(DESC_LEVEL)?,
        };
        Ok(Self { z })
    }

    /// Write one building's violations, with their descriptions as one compressed block.
    ///
    /// Per-building rather than per-row because the redundancy is BETWEEN notices -- every
    /// one opens with a housing-code statute reference -- not inside a single one. The block
    /// is the building because a card reads exactly one building, so a lookup decompresses
    /// exactly what it needs and needs no block index.
    pub fn insert(&mut self, conn: &Connection, bbl: &str, vs: &[Violation]) -> Result<()> {
        let mut texts: Vec<&str> = Vec::new();
        let mut stmt = conn.prepare_cached(
            "INSERT INTO violations (bbl,seq,class,open,year,desc_ord,issued_on,closed_on) \
             VALUES (?1,?2,?3,?4,?5,?6,unixepoch(?7) / 86400,unixepoch(?8) / 86400)",
        )?;
        for (seq, v) in vs.iter().enumerate() {
            let ord = match v.description.as_deref() {
                Some(d) if !d.is_empty() => {
                    texts.push(d);
                    Some(texts.len() as i64 - 1)
                }
                _ => None,
            };
            stmt.execute(rusqlite::params![
                bbl,
                seq as i64,
                // `as_str`, so the column keeps holding exactly the letter HPD issued.
                v.class.as_str(),
                v.open as i64,
                v.year,
                ord,
                v.issued_on,
                v.closed_on
            ])?;
        }
        if !texts.is_empty() {
            let z = self
                .z
                .compress(texts.join(&RECORD_SEP.to_string()).as_bytes())?;
            conn.prepare_cached("INSERT OR REPLACE INTO violation_desc (bbl,z) VALUES (?1,?2)")?
                .execute(rusqlite::params![bbl, z])?;
        }
        Ok(())
    }
}

/// Write one building's violations with a fresh [`ViolationWriter`]: the fixture's and the
/// tests' path. The ingest keeps one writer for the whole run instead.
pub fn insert_violations(conn: &Connection, bbl: &str, vs: &[Violation]) -> Result<()> {
    ViolationWriter::new(conn)?.insert(conn, bbl, vs)
}

/// Single-row insert, kept for the fixture seed and for tests.
pub fn insert_violation(conn: &Connection, bbl: &str, v: &Violation) -> Result<()> {
    insert_violations(conn, bbl, std::slice::from_ref(v))
}

pub fn upsert_tract_median(conn: &Connection, tract_geoid: &str, median: i32) -> Result<()> {
    conn.execute(
        "INSERT INTO acs_rent_by_tract (tract_geoid, median_gross_rent) VALUES (?1,?2)
         ON CONFLICT(tract_geoid) DO UPDATE SET median_gross_rent=excluded.median_gross_rent",
        rusqlite::params![tract_geoid, median],
    )?;
    Ok(())
}

/// Write one provenance fact into `meta`. Idempotent per key.
///
/// `meta` already carried `snapshot_year` and nothing else, which meant the artifact could
/// not answer the two questions anyone actually asks of it: when was this gathered, and what
/// is missing from it. Both were knowable at ingest and thrown away. Everything written here
/// travels with the database into the image, so a running container can state its own
/// provenance instead of the frontend asserting it from a hardcoded literal.
/// Record what one source returned, and when. Idempotent per dataset.
pub fn set_source_provenance(
    conn: &Connection,
    dataset: &str,
    retrieved_at_unix: i64,
    row_count: i64,
    note: Option<&str>,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO source_provenance (dataset,retrieved_at_unix,row_count,note)          VALUES (?1,?2,?3,?4)",
        rusqlite::params![dataset, retrieved_at_unix, row_count, note],
    )?;
    Ok(())
}

/// One source's provenance row: dataset id, retrieval time, row count, and the query note.
pub type SourceProvenanceRow = (String, i64, i64, Option<String>);

/// Every source's provenance, dataset order, for the export and the `/meta` endpoint.
pub fn all_source_provenance(conn: &Connection) -> Result<Vec<SourceProvenanceRow>> {
    let mut stmt = conn.prepare(
        "SELECT dataset, retrieved_at_unix, row_count, note FROM source_provenance          ORDER BY dataset",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta (key,value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// One provenance fact, or `None` if this artifact predates it.
pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key=?1")?;
    let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
    Ok(match rows.next() {
        Some(v) => Some(v?),
        None => None,
    })
}

/// Every provenance row, key-ordered, for `/health` and startup logging.
pub fn all_meta(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, value FROM meta ORDER BY key")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn set_snapshot_year(conn: &Connection, year: i32) -> Result<()> {
    set_meta(conn, "snapshot_year", &year.to_string())
}

pub fn get_snapshot_year(conn: &Connection) -> Result<Option<i32>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key='snapshot_year'")?;
    let mut rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    match rows.next() {
        Some(v) => Ok(v?.parse::<i32>().ok()),
        None => Ok(None),
    }
}

/// `data/law.json`, compiled in, so the artifact, the fixture and the tests all read the same
/// reviewed file and no run can pick up a stray copy.
const LAW_JSON: &str = include_str!("../../../data/law.json");

/// Replace the law tables with `data/law.json`. Returns the number of provisions.
///
/// Refuses the whole file on the first problem: an unknown topic, a fact whose quote is not in
/// its provision's text, a broken reference, or anything the schema's CHECKs reject. A wrong
/// statute is worse than a missing one, so a bad row stops the build rather than being skipped.
pub fn load_law(conn: &Connection) -> Result<usize> {
    insert_law(conn, &law_seed()?)
}

/// `data/law.json` as compiled in. The evals read the directory from here to recognise a
/// referral by name or number.
pub fn law_seed() -> Result<model::law::LawSeed> {
    serde_json::from_str(LAW_JSON).context("data/law.json does not parse")
}

fn insert_law(conn: &Connection, seed: &model::law::LawSeed) -> Result<usize> {
    use model::law::{normalize, phone_digits, phone_numbers, text_sha256, TOPICS};
    // Off by default in SQLite, and a no-op once a transaction is open.
    conn.execute_batch("PRAGMA foreign_keys = ON")?;
    let tx = conn.unchecked_transaction()?;
    tx.execute_batch(
        "DELETE FROM legal_help; DELETE FROM law_fact; DELETE FROM law_topic; DELETE FROM law_passage_fts;
         DELETE FROM law_passage; DELETE FROM law_provision; DELETE FROM law_source;",
    )?;
    for o in &seed.legal_help {
        let excerpt = phone_digits(&o.excerpt);
        for n in phone_numbers(o.phone.as_deref().unwrap_or("")) {
            anyhow::ensure!(
                excerpt.contains(&n),
                "legal help {}: phone {n} is not in its excerpt",
                o.name
            );
        }
        if let Some(h) = &o.hours {
            anyhow::ensure!(
                normalize(&o.excerpt.to_lowercase()).contains(&normalize(&h.to_lowercase())),
                "legal help {}: hours {h:?} are not in its excerpt",
                o.name
            );
        }
        tx.execute(
            "INSERT INTO legal_help (name,what,phone,hours,url,free,excerpt,retrieved_at,verified_by,verified_on)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            rusqlite::params![
                o.name,
                o.what,
                o.phone,
                o.hours,
                o.url,
                o.free,
                o.excerpt,
                o.retrieved_at,
                o.verified_by,
                o.verified_on
            ],
        )
        .with_context(|| format!("legal help {}", o.name))?;
    }
    for s in &seed.sources {
        tx.execute(
            "INSERT INTO law_source (id,publisher,jurisdiction,kind,base_url) VALUES (?1,?2,?3,?4,?5)",
            rusqlite::params![s.id, s.publisher, s.jurisdiction, s.kind, s.base_url],
        )
        .with_context(|| format!("law source {}", s.id))?;
    }
    for p in &seed.provisions {
        tx.execute(
            "INSERT INTO law_provision (id,source_id,citation,title,text,url,retrieved_at,
               text_sha256,verified_by,verified_on,status,occupancy)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            rusqlite::params![
                p.id,
                p.source,
                p.citation,
                p.title,
                p.text,
                p.url,
                p.retrieved_at,
                text_sha256(&p.text),
                p.verified_by,
                p.verified_on,
                p.status,
                p.occupancy
            ],
        )
        .with_context(|| format!("law provision {}", p.id))?;
        anyhow::ensure!(!p.topics.is_empty(), "{}: no topics", p.id);
        for t in &p.topics {
            anyhow::ensure!(
                TOPICS.contains(&t.as_str()),
                "{}: topic {t:?} is not one lookup_law accepts",
                p.id
            );
            tx.execute(
                "INSERT INTO law_topic (provision_id, topic) VALUES (?1, ?2)",
                rusqlite::params![p.id, t],
            )?;
        }
        let parts = model::law::passages(&p.text);
        anyhow::ensure!(
            parts.iter().map(|x| x.text.as_str()).collect::<String>() == p.text,
            "{}: its passages do not rejoin to its text",
            p.id
        );
        for (seq, x) in parts.iter().enumerate() {
            tx.execute(
                "INSERT INTO law_passage (provision_id, seq, label, text) VALUES (?1,?2,?3,?4)",
                rusqlite::params![p.id, seq as i64, x.label, x.text],
            )?;
            tx.execute(
                "INSERT INTO law_passage_fts (text, provision_id, seq) VALUES (?1,?2,?3)",
                rusqlite::params![x.text, p.id, seq as i64],
            )?;
        }
        let text = normalize(&p.text);
        for f in &p.facts {
            anyhow::ensure!(
                text.contains(&normalize(&f.quote)),
                "{}: the quote for {} is not in the provision's text",
                p.id,
                f.key
            );
            // A served fact must come with the words it was taken from, and passages are
            // served whole, so its quote has to sit inside one of them.
            anyhow::ensure!(
                parts
                    .iter()
                    .any(|x| normalize(&x.text).contains(&normalize(&f.quote))),
                "{}: the quote for {} crosses a passage boundary",
                p.id,
                f.key
            );
            anyhow::ensure!(
                f.quote.contains(&f.value),
                "{}: {} = {:?} is not in its quote",
                p.id,
                f.key,
                f.value
            );
            tx.execute(
                "INSERT INTO law_fact (key,provision_id,value,condition,quote) VALUES (?1,?2,?3,?4,?5)",
                rusqlite::params![f.key, p.id, f.value, f.condition, f.quote],
            )
            .with_context(|| format!("law fact {}", f.key))?;
        }
    }
    tx.commit()?;
    Ok(seed.provisions.len())
}

/// Provisions in the artifact. The API refuses to start on zero, as it does for buildings: an
/// agent with no law to look up would answer every legal question with "I couldn't verify it".
pub fn law_provision_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM law_provision", [], |r| r.get(0))?)
}

/// The legal-help directory, in the order `data/law.json` lists it.
pub fn legal_help(conn: &Connection) -> Result<Vec<model::law::LegalHelp>> {
    let mut stmt = conn.prepare(
        "SELECT name, what, phone, hours, url, free, excerpt, retrieved_at, verified_by, verified_on
         FROM legal_help ORDER BY rowid",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(model::law::LegalHelp {
                name: r.get(0)?,
                what: r.get(1)?,
                phone: r.get(2)?,
                hours: r.get(3)?,
                url: r.get(4)?,
                free: r.get(5)?,
                excerpt: r.get(6)?,
                retrieved_at: r.get(7)?,
                verified_by: r.get(8)?,
                verified_on: r.get(9)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

/// Columns `p.id, p.citation, p.title, p.text, p.url, s.publisher, s.kind, p.status,
/// p.retrieved_at, p.verified_on`, in that order.
fn served_provision(r: &rusqlite::Row) -> rusqlite::Result<model::law::ServedProvision> {
    let text: String = r.get(3)?;
    Ok(model::law::ServedProvision {
        id: r.get(0)?,
        citation: r.get(1)?,
        title: r.get(2)?,
        full_chars: text.chars().count(),
        complete: true,
        text,
        url: r.get(4)?,
        publisher: r.get(5)?,
        kind: r.get(6)?,
        status: r.get(7)?,
        retrieved_at: r.get(8)?,
        verified_on: r.get(9)?,
    })
}

/// One current provision by id, whole: what `/law/{id}` serves.
pub fn law_provision(conn: &Connection, id: &str) -> Result<Option<model::law::ServedProvision>> {
    Ok(conn
        .query_row(
            "SELECT p.id, p.citation, p.title, p.text, p.url, s.publisher, s.kind, p.status,
                    p.retrieved_at, p.verified_on
             FROM law_provision p JOIN law_source s ON s.id = p.source_id
             WHERE p.id = ?1",
            [id],
            served_provision,
        )
        .optional()?)
}

/// Words worth searching for in a question: letters and digits, three or more, minus the
/// filler every question shares. At most 16, so one long message can't make a huge query.
fn search_terms(query: &str) -> Vec<String> {
    const SKIP: &[&str] = &[
        "the", "and", "for", "are", "but", "not", "you", "your", "with", "this", "that", "what",
        "when", "how", "can", "does", "have", "has", "was", "will", "would", "should", "could",
        "there", "here", "about", "from", "they", "them", "their", "into", "any", "who", "why",
        "which", "our", "out", "get", "got", "just", "like", "need", "want", "know", "tell",
    ];
    let mut out: Vec<String> = Vec::new();
    for w in query
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() >= 3 && !SKIP.contains(w))
    {
        if !out.iter().any(|o| o == w) {
            out.push(w.to_string());
        }
        if out.len() == 16 {
            break;
        }
    }
    out
}

/// Cut `provisions` down to the passages that bear on `query`, within `budget` characters.
///
/// Passages holding a fact's quote come first, then the passages FTS5 ranks highest for the
/// query, each with the lead-in sentence of any list it sits in (the nearest passage above it
/// ending with a colon, past its sibling items). Passages are served whole or not at all. With nothing matched, each
/// provision's opening passage is served, since it says what the section covers. Provisions
/// keep their order; inside one, passages keep theirs, and a gap is a line holding only `…`.
pub fn select_passages(
    conn: &Connection,
    provisions: Vec<model::law::ServedProvision>,
    facts: &[model::law::ServedFact],
    query: &str,
    budget: usize,
) -> Result<Vec<model::law::ServedProvision>> {
    use model::law::normalize;
    let mut stmt =
        conn.prepare("SELECT seq, text FROM law_passage WHERE provision_id = ?1 ORDER BY seq")?;
    let mut parts: Vec<Vec<String>> = Vec::new();
    for p in &provisions {
        parts.push(
            stmt.query_map([&p.id], |r| r.get::<_, String>(1))?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        );
    }
    let index = |id: &str| provisions.iter().position(|p| p.id == id);

    // Candidates in priority order: (provision index, seq).
    let mut wanted: Vec<(usize, usize)> = Vec::new();
    for f in facts {
        if let Some(i) = index(&f.provision) {
            let q = normalize(&f.fact.quote);
            if let Some(s) = parts[i].iter().position(|t| normalize(t).contains(&q)) {
                wanted.push((i, s));
            }
        }
    }
    let terms = search_terms(query);
    if !terms.is_empty() {
        let fts = terms
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut stmt = conn.prepare(
            "SELECT provision_id, seq FROM law_passage_fts WHERE law_passage_fts MATCH ?1
             ORDER BY bm25(law_passage_fts)",
        )?;
        let hits = stmt
            .query_map([fts], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (id, seq) in hits {
            if let Some(i) = index(&id) {
                wanted.push((i, seq as usize));
            }
        }
    }
    let matched = !wanted.is_empty();
    if !matched {
        // A heading on its own line ("§ 27-2005 Duties of owner.") says nothing; the opening
        // is the heading and the first subdivision after it.
        for (i, p) in parts.iter().enumerate() {
            wanted.push((i, 0));
            if p.len() > 1 && p[0].trim().chars().count() < 200 {
                wanted.push((i, 1));
            }
        }
    }

    let mut chosen: Vec<Vec<bool>> = parts.iter().map(|v| vec![false; v.len()]).collect();
    let mut used = 0usize;
    for (i, s) in wanted {
        // The passage and the lead-ins of the lists it sits in: walking back over its
        // siblings (items ending ";", "; or", "; and") to each sentence ending with a colon.
        let mut group = vec![s];
        let mut k = s;
        while k > 0 {
            let prev = parts[i][k - 1].trim_end();
            if prev.ends_with(':') {
                group.push(k - 1);
            } else if !(prev.ends_with(';') || prev.ends_with("; or") || prev.ends_with("; and")) {
                break;
            }
            k -= 1;
        }
        let cost: usize = group
            .iter()
            .filter(|&&g| !chosen[i][g])
            .map(|&g| parts[i][g].chars().count())
            .sum();
        if cost == 0 || used + cost > budget {
            continue;
        }
        used += cost;
        for g in group {
            chosen[i][g] = true;
        }
    }

    let mut out = Vec::new();
    for (i, mut p) in provisions.into_iter().enumerate() {
        if !chosen[i].iter().any(|&c| c) {
            continue;
        }
        let mut text = String::new();
        let mut prev: Option<usize> = None;
        for (s, piece) in parts[i].iter().enumerate().filter(|(s, _)| chosen[i][*s]) {
            if prev.is_some_and(|q| q + 1 != s) {
                text.push_str("\n…\n");
            }
            text.push_str(piece);
            prev = Some(s);
        }
        p.complete = chosen[i].iter().all(|&c| c);
        p.text = text;
        out.push(p);
    }
    Ok(out)
}

/// The current text of the Admin Code sections HPD cites in these notices, in order of first
/// citation. A section the table does not hold is left out: HouseCheck has not verified it.
pub fn law_cited_by(
    conn: &Connection,
    descriptions: &[&str],
) -> Result<Vec<model::law::ServedProvision>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.citation, p.title, p.text, p.url, s.publisher, s.kind, p.status,
                p.retrieved_at, p.verified_on
         FROM law_provision p JOIN law_source s ON s.id = p.source_id
         WHERE p.id = ?1 AND p.status = 'current'",
    )?;
    let mut ids: Vec<String> = Vec::new();
    for d in descriptions {
        for s in model::law::hpd_cited_sections(d) {
            let id = model::law::admin_code_id(&s);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    let mut out = Vec::new();
    for id in ids {
        if let Some(p) = stmt.query_map([&id], served_provision)?.next() {
            out.push(p?);
        }
    }
    Ok(out)
}

/// Current provisions tagged `topic`, statutes before regulations before agency guidance, and
/// the facts taken from them.
pub fn law_for_topic(conn: &Connection, topic: &str) -> Result<model::law::LawLookup> {
    use model::law::{LawFact, ServedFact};
    let mut stmt = conn.prepare(
        "SELECT p.id, p.citation, p.title, p.text, p.url, s.publisher, s.kind, p.status,
                p.retrieved_at, p.verified_on
         FROM law_provision p
         JOIN law_topic t ON t.provision_id = p.id
         JOIN law_source s ON s.id = p.source_id
         WHERE t.topic = ?1 AND p.status = 'current'
         ORDER BY CASE s.kind WHEN 'statute' THEN 0 WHEN 'regulation' THEN 1 ELSE 2 END, p.id",
    )?;
    let provisions = stmt
        .query_map([topic], served_provision)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut stmt = conn.prepare(
        "SELECT f.provision_id, f.key, f.value, f.condition, f.quote
         FROM law_fact f
         JOIN law_topic t ON t.provision_id = f.provision_id
         JOIN law_provision p ON p.id = f.provision_id
         WHERE t.topic = ?1 AND p.status = 'current'
         ORDER BY f.key",
    )?;
    let facts = stmt
        .query_map([topic], |r| {
            Ok(ServedFact {
                provision: r.get(0)?,
                fact: LawFact {
                    key: r.get(1)?,
                    value: r.get(2)?,
                    condition: r.get(3)?,
                    quote: r.get(4)?,
                },
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(model::law::LawLookup { provisions, facts })
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
             AND name IN ('buildings','violations','acs_rent_by_tract',
                          'law_source','law_provision','law_topic','law_fact','legal_help')",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 8);
        Ok(())
    }

    /// The shipped seed loads, and serves only what is current.
    #[test]
    fn law_seed_loads_and_superseded_rows_are_never_served() -> Result<()> {
        let conn = seeded()?;
        assert!(law_provision_count(&conn)? > 0);
        let heat = law_for_topic(&conn, "heat")?;
        let night = heat
            .facts
            .iter()
            .find(|f| f.fact.key == "heat.night.min_f")
            .expect("the night rule the live eval got wrong");
        assert_eq!(night.fact.value, "62");

        conn.execute(
            "UPDATE law_provision SET status = 'superseded' WHERE id = ?1",
            [&night.provision],
        )?;
        let after = law_for_topic(&conn, "heat")?;
        assert!(after.provisions.iter().all(|p| p.id != night.provision));
        assert!(after.facts.iter().all(|f| f.provision != night.provision));
        Ok(())
    }

    /// Each way a seed can be wrong stops the load, and says which row.
    #[test]
    fn law_seed_refuses_a_bad_row() -> Result<()> {
        let good: serde_json::Value = serde_json::from_str(LAW_JSON)?;
        // Each edit is made to the heat page, the provision that has facts.
        let refuses = |edit: &dyn Fn(&mut serde_json::Value), needle: &str| -> Result<()> {
            let mut v = good.clone();
            let heat = v["provisions"]
                .as_array_mut()
                .and_then(|ps| ps.iter_mut().find(|p| p["id"] == "NYC311-KA-01036"))
                .expect("the heat provision");
            edit(heat);
            let conn = open_db(":memory:")?;
            migrate(&conn)?;
            let err = insert_law(&conn, &serde_json::from_value(v)?)
                .expect_err("a bad seed must not load");
            assert!(format!("{err:#}").contains(needle), "{needle}: {err:#}");
            Ok(())
        };
        refuses(
            &|p| p["topics"] = serde_json::json!(["heating"]),
            "is not one lookup_law accepts",
        )?;
        refuses(
            &|p| p["facts"][0]["quote"] = "heat begins September 15".into(),
            "is not in the provision's text",
        )?;
        refuses(
            // facts[4] is heat.night.min_f, "62".
            &|p| p["facts"][4]["value"] = "64".into(),
            "is not in its quote",
        )?;
        refuses(&|p| p["status"] = "draft".into(), "law provision")?;
        refuses(&|p| p["source"] = "nobody".into(), "law provision")?;
        refuses(&|p| p["verified_on"] = "Oct 30".into(), "law provision")?;
        Ok(())
    }

    /// A section HPD cites comes back as the provision the table holds for it, once, and only
    /// while current. Sections the table does not hold, and the Multiple Dwelling Law, do not.
    #[test]
    fn violations_join_to_the_sections_they_cite() -> Result<()> {
        // The seed holds the real § 27-2005 (copied from amlegal, 2026-10-30) and no § 27-2007.
        let conn = seeded()?;
        let notices = [
            "§ 27-2005 ADM CODE PROPERLY REPAIR THE BROKEN OR DEFECTIVE INTERCOM",
            "§ 27-2005, 27-2007 ADM CODE REPLACE",
            "§ 329, M/D LAW AND DEPT. RULES AND REGS.",
        ];
        let law = law_cited_by(&conn, &notices)?;
        let ids: Vec<&str> = law.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            ["NYC-ADMIN-27-2005"],
            "27-2007 is not held; § 329 is not Admin Code"
        );

        conn.execute_batch(
            "UPDATE law_provision SET status = 'superseded' WHERE id = 'NYC-ADMIN-27-2005'",
        )?;
        assert!(law_cited_by(&conn, &notices)?.is_empty());
        Ok(())
    }

    /// A directory entry has to agree with the page it was copied from, and be free.
    #[test]
    fn legal_help_refuses_what_its_page_does_not_say() -> Result<()> {
        let good: serde_json::Value = serde_json::from_str(LAW_JSON)?;
        let refuses = |edit: &dyn Fn(&mut serde_json::Value), needle: &str| -> Result<()> {
            let mut v = good.clone();
            edit(&mut v["legal_help"][0]);
            let conn = open_db(":memory:")?;
            migrate(&conn)?;
            let err = insert_law(&conn, &serde_json::from_value(v)?)
                .expect_err("a bad directory entry must not load");
            assert!(format!("{err:#}").contains(needle), "{needle}: {err:#}");
            Ok(())
        };
        // One digit off.
        refuses(
            &|o| o["phone"] = "212-962-4796".into(),
            "is not in its excerpt",
        )?;
        refuses(
            &|o| o["hours"] = "Tuesday — Thursday 9 am to 5 pm".into(),
            "are not in its excerpt",
        )?;
        refuses(&|o| o["free"] = false.into(), "legal help")?;

        let conn = seeded()?;
        let dir = legal_help(&conn)?;
        assert_eq!(dir[0].name, "Housing Court Answers", "served in seed order");
        assert!(dir.iter().any(|o| o.phone.is_some()));
        Ok(())
    }

    use model::Building;

    fn seeded() -> Result<Connection> {
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        insert_fixture(&conn)?;
        Ok(conn)
    }

    /// Each served piece between `…` lines is word for word in its provision's full text.
    fn assert_verbatim(conn: &Connection, served: &[model::law::ServedProvision]) -> Result<()> {
        for p in served {
            let full = law_provision(conn, &p.id)?
                .expect("served provision exists")
                .text;
            for piece in p.text.split("\n…\n") {
                assert!(
                    full.contains(piece),
                    "{}: a served piece is not in the full text",
                    p.id
                );
            }
        }
        Ok(())
    }

    /// Ingest stores every provision as passages that rejoin to its text exactly, and indexes
    /// them for search.
    #[test]
    fn passages_rejoin_to_every_provision() -> Result<()> {
        let conn = seeded()?;
        let mut stmt = conn.prepare("SELECT id, text FROM law_provision")?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        assert!(!rows.is_empty());
        for (id, text) in rows {
            let joined: String = conn
                .prepare("SELECT text FROM law_passage WHERE provision_id = ?1 ORDER BY seq")?
                .query_map([&id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
                .concat();
            assert_eq!(joined, text, "{id}");
        }
        let fts: i64 = conn.query_row("SELECT count(*) FROM law_passage_fts", [], |r| r.get(0))?;
        let all: i64 = conn.query_row("SELECT count(*) FROM law_passage", [], |r| r.get(0))?;
        assert_eq!(fts, all);
        Ok(())
    }

    /// Eviction is the topic that sent 49,999 characters. A question about a rent increase gets
    /// passages within the budget, every one verbatim, the fact passages among them.
    #[test]
    fn passages_fit_the_budget_and_stay_verbatim() -> Result<()> {
        let conn = seeded()?;
        let law = law_for_topic(&conn, "eviction")?;
        let full: usize = law.provisions.iter().map(|p| p.full_chars).sum();
        assert!(full > 40_000, "the eviction topic is the big one ({full})");
        let served = select_passages(
            &conn,
            law.provisions,
            &law.facts,
            "My landlord says my rent increase notice is legal. Is it?",
            6_000,
        )?;
        let gaps = served
            .iter()
            .map(|p| p.text.matches("\n…\n").count() * 3)
            .sum::<usize>();
        let sent: usize = served.iter().map(|p| p.text.chars().count()).sum();
        assert!(sent <= 6_000 + gaps, "{sent} characters");
        assert!(served.iter().any(|p| !p.complete));
        assert_verbatim(&conn, &served)?;
        for f in &law.facts {
            let p = served
                .iter()
                .find(|p| p.id == f.provision)
                .expect("fact provision served");
            assert!(
                model::law::normalize(&p.text).contains(&model::law::normalize(&f.fact.quote)),
                "{}: its quote must be served",
                f.fact.key
            );
        }
        Ok(())
    }

    /// A passage that completes a sentence arrives with the sentence: asking about a false
    /// certification of a mold violation serves § 27-2017.3's "(b)" with its "3." lead-in.
    #[test]
    fn a_passage_comes_with_its_lead_in() -> Result<()> {
        let conn = seeded()?;
        let law = law_for_topic(&conn, "mold")?;
        let served = select_passages(
            &conn,
            law.provisions,
            &law.facts,
            "owner submitted a false certification of correction",
            1_500,
        )?;
        let mold = served
            .iter()
            .find(|p| p.id == "NYC-ADMIN-27-2017.3")
            .expect("27-2017.3 served");
        assert!(mold.text.contains("false certification"));
        assert!(mold
            .text
            .contains("shall constitute a hazardous violation if:"));
        assert_verbatim(&conn, &served)
    }

    /// Nothing matched: each provision's opening passage, which says what it covers.
    #[test]
    fn no_match_serves_opening_passages() -> Result<()> {
        let conn = seeded()?;
        let law = law_for_topic(&conn, "lead_paint")?;
        let n = law.provisions.len();
        let served = select_passages(&conn, law.provisions, &[], "zzzz qqqq", 6_000)?;
        assert_eq!(served.len(), n);
        assert!(served.iter().all(|p| p.text.starts_with("§ 27-2056")));
        assert_verbatim(&conn, &served)
    }

    #[test]
    fn fixture_building_loads_by_bbl() -> Result<()> {
        let conn = seeded()?;
        let b: Building = get_building(&conn, "3000010001")?.expect("building exists");
        assert_eq!(b.address, "1 Fixture Ave, Brooklyn");
        assert!(b.has_elevator);
        // Stabilized fixture carries its DOF unit count alongside the boolean flag.
        assert_eq!(b.rent_stabilized, Some(true));
        assert_eq!(b.rent_stab_units, Some(12));
        Ok(())
    }

    #[test]
    fn missing_bbl_returns_none() -> Result<()> {
        let conn = seeded()?;
        assert!(get_building(&conn, "9999999999")?.is_none());
        Ok(())
    }

    /// Descriptions are compressed as one blob per building, so the round trip has to
    /// preserve order, gaps and awkward bytes exactly. Silent corruption here would put the
    /// wrong condition next to the right violation, which is worse than showing none.
    #[test]
    fn descriptions_round_trip_through_compression() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        let v = |class: ViolationClass, d: Option<&str>| Violation {
            class,
            open: true,
            year: 2026,
            description: d.map(str::to_string),
            ..Default::default()
        };
        let vs = vec![
            v(
                ViolationClass::C,
                Some("§ 27-2005 ADM CODE ABATE THE NUISANCE AT KITCHEN"),
            ),
            // A gap in the middle: desc_ord must skip, not shift everything after it.
            v(ViolationClass::A, None),
            // A notice containing a newline is exactly why records are joined with
            // \x1e rather than \n.
            v(ViolationClass::B, Some("LINE ONE\nLINE TWO")),
            v(
                ViolationClass::C,
                Some("§ 27-2005 ADM CODE ABATE THE NUISANCE AT BATHROOM"),
            ),
        ];
        insert_violations(&conn, "3000010001", &vs)?;

        let got = get_open_violations(&conn, "3000010001")?;
        assert_eq!(got.len(), 4);
        assert_eq!(got[0].description.as_deref(), vs[0].description.as_deref());
        assert_eq!(got[1].description, None, "a gap must stay a gap");
        assert_eq!(got[2].description.as_deref(), Some("LINE ONE\nLINE TWO"));
        assert_eq!(got[3].description.as_deref(), vs[3].description.as_deref());

        // A building with no descriptions stores no block and still reads back clean.
        insert_violations(&conn, "3000010002", &[v(ViolationClass::A, None)])?;
        let none = get_open_violations(&conn, "3000010002")?;
        assert_eq!(none.len(), 1);
        assert_eq!(none[0].description, None);

        // And the blob is genuinely smaller than the text it holds.
        let raw: usize = vs
            .iter()
            .filter_map(|x| x.description.as_ref())
            .map(|d| d.len())
            .sum();
        let z = conn.query_row(
            "SELECT length(z) FROM violation_desc WHERE bbl = '3000010001'",
            [],
            |r| r.get::<_, i64>(0),
        )? as usize;
        assert!(z < raw, "compressed {z} should be under raw {raw}");
        Ok(())
    }

    /// The same round trip with a trained dictionary, which is what the real artifact uses.
    /// A block written with the dictionary must come back exactly, and must be smaller than
    /// the same block without it, or the dictionary is dead weight.
    #[test]
    fn descriptions_round_trip_with_a_trained_dictionary() -> Result<()> {
        let notice = |i: usize| {
            format!(
                "§ 27-20{:02} ADM CODE ABATE THE NUISANCE CONSISTING OF {} AT APT {}{} \
                 LOCATED AT {} STORY",
                i % 40,
                ["BROKEN PLASTER", "MICE", "ROACHES", "WATER LEAK", "NO HEAT"][i % 5],
                i % 9 + 1,
                ["A", "B", "C", "D"][i % 4],
                ["1ST", "2ND", "3RD"][i % 3]
            )
        };
        let samples: Vec<String> = (0..2000).map(notice).collect();
        let dict = train_desc_dict(&samples).expect("enough text to train a dictionary");

        let vs: Vec<Violation> = (5000..5003)
            .map(|i| Violation {
                class: ViolationClass::B,
                open: true,
                year: 2026,
                description: Some(notice(i)),
                ..Default::default()
            })
            .collect();
        let size_with = |dict: Option<&[u8]>| -> Result<(Connection, i64)> {
            let conn = Connection::open_in_memory()?;
            migrate(&conn)?;
            if let Some(d) = dict {
                set_desc_dict(&conn, d)?;
            }
            insert_violations(&conn, "3000010001", &vs)?;
            let n = conn.query_row("SELECT length(z) FROM violation_desc", [], |r| r.get(0))?;
            Ok((conn, n))
        };
        let (conn, with) = size_with(Some(&dict))?;
        let (_, without) = size_with(None)?;
        let got = get_open_violations(&conn, "3000010001")?;
        for (g, v) in got.iter().zip(&vs) {
            assert_eq!(g.description, v.description);
        }
        assert!(with < without, "dictionary {with} bytes vs plain {without}");
        Ok(())
    }

    /// Dates are stored as day numbers and must come back as the exact strings that went in,
    /// including before 1970 and across a leap day, because the export hashes them.
    #[test]
    fn dates_round_trip_as_iso_strings() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        let dates = ["1965-07-04", "2000-02-29", "2014-01-06", "2026-10-30"];
        let vs: Vec<Violation> = dates
            .iter()
            .map(|d| Violation {
                class: ViolationClass::A,
                open: true,
                year: d[..4].parse().unwrap(),
                issued_on: Some(d.to_string()),
                closed_on: Some(d.to_string()),
                ..Default::default()
            })
            .collect();
        insert_violations(&conn, "3000010001", &vs)?;
        let got = get_open_violations(&conn, "3000010001")?;
        let issued: Vec<_> = got.iter().map(|v| v.issued_on.as_deref()).collect();
        assert_eq!(issued, dates.map(Some));
        Ok(())
    }

    /// Repair speed reads day differences off the stored day numbers. Same answers as the old
    /// `julianday` over text, including the cutoff boundary.
    #[test]
    fn closed_durations_are_day_differences_since_the_cutoff() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        let closed = |issued: &str, closed: &str| Violation {
            class: ViolationClass::B,
            open: false,
            year: issued[..4].parse().unwrap(),
            issued_on: Some(issued.into()),
            closed_on: Some(closed.into()),
            ..Default::default()
        };
        insert_violations(
            &conn,
            "3000010001",
            &[
                closed("2022-12-01", "2023-01-01"), // closed on the cutoff day: counts
                closed("2022-06-01", "2022-12-31"), // closed the day before: does not
                closed("2024-02-28", "2024-03-01"), // across a leap day: 2 days
            ],
        )?;
        let mut d = closed_violation_durations(&conn, "3000010001", 2023)?;
        d.sort();
        assert_eq!(d, vec![2, 31]);
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
            rent_stabilized: Some(true),
            rent_stab_units: Some(9),
            good_cause: false,
            has_elevator: true,
            near_ada_subway_m: Some(420),
            zip: Some("11216".into()),
            neighborhood: Some("Bedford-Stuyvesant (West)".into()),
            complaints_311_per_1k_homes: Some(2677),
            latitude: Some(40.6829),
            longitude: Some(-73.9251),
        };
        upsert_building(&conn, &b)?;
        assert_eq!(get_building(&conn, "3018420001")?.unwrap(), b);
        Ok(())
    }

    #[test]
    fn migrate_is_idempotent_on_existing_db() -> Result<()> {
        // `law --out` runs migrate against an artifact that already has every table.
        let conn = open_db(":memory:")?;
        migrate(&conn)?;
        migrate(&conn)?;
        Ok(())
    }

    #[test]
    fn get_all_buildings_returns_fixture_set() -> Result<()> {
        let conn = seeded()?;
        let all = get_all_buildings(&conn)?;
        assert_eq!(all.len(), 2);
        // Ordered by BBL; the first fixture carries stored coordinates and its ZIP context.
        assert_eq!(all[0].bbl, "3000010001");
        assert_eq!(all[0].zip.as_deref(), Some("11221"));
        assert_eq!(all[0].complaints_311_per_1k_homes, Some(600));
        assert!(all[0].latitude.is_some() && all[0].longitude.is_some());
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
                class: ViolationClass::C,
                open: true,
                year: 2025,
                ..Default::default()
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

    /// REVIEW-NOTES M4 at the read. Before `ViolationClass`, a stored class this code did not
    /// model came back as a `String` and `scoring` priced it at 0. Measured on `main` at
    /// `03dfae6`: a building whose only open violation was class "X" scored condition 100, the
    /// same as a building with none. `ingest` can no longer write such a row, so one here means
    /// an artifact built by something else, and the read refuses it by name.
    #[test]
    fn an_unmodelled_class_is_an_error_not_a_clean_building() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        migrate(&conn)?;
        conn.execute(
            "INSERT INTO violations (bbl,seq,class,open,year) VALUES ('3000010009',0,'X',1,2026)",
            [],
        )?;
        let err = get_open_violations(&conn, "3000010009")
            .expect_err("an unmodelled class must not read back as a violation");
        assert!(
            format!("{err:#}").contains("\"X\""),
            "the error should name the letter: {err:#}"
        );
        Ok(())
    }
}
