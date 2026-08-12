//! HouseCheck as an MCP server: another agent can look up a building and get the same
//! card the website serves.
//!
//! **Step 1 of `docs/mcp-ui.md`, deliberately text-only.** No `ui://` resource yet. This
//! step proves the transport and is useful on its own, and it keeps the security question
//! that comes with embedding -- a framed card page is a surface we own -- as a separate
//! decision rather than something smuggled in with the first commit.
//!
//! Two rules the HTTP API already follows and this one inherits, because an agent is a
//! *worse* audience for a bare number than a person is:
//!
//! 1. **No figure without its source.** Every response carries the artifact's build date
//!    and the coverage limit, so a model cannot restate a score as citywide fact.
//! 2. **Absent stays absent.** Where the record cannot support a claim -- repair speed on
//!    a building that has closed nothing -- the tool says so in words rather than
//!    emitting a zero an agent would read as "fast".
//!
//! Run it against the artifact:
//! ```text
//! HOUSECHECK_DB=data/housecheck.db cargo run -p mcp
//! ```

use anyhow::Result;
use rmcp::{
    handler::server::wrapper::Parameters, tool, tool_handler, tool_router, ServerHandler,
    ServiceExt,
};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

/// Read-only, single-process, and behind a mutex: rusqlite `Connection` is not `Sync`,
/// and an MCP server over stdio handles one request at a time anyway. A pool here would
/// be ceremony -- the artifact is a file baked into the image, not a database server.
#[derive(Clone)]
struct HouseCheck {
    conn: Arc<Mutex<Connection>>,
    snapshot_year: i32,
    /// All four read from the artifact's own `meta` table rather than written here, so the
    /// provenance an agent is handed cannot drift from the data it describes.
    data_month: String,
    sources: String,
    excluded: String,
    building_count: i64,
    // Read by the code `#[tool_handler]` generates, which rustc's dead-code pass cannot
    // see through. The field name is part of the macro's contract, not incidental.
    #[allow(dead_code)]
    tool_router: rmcp::handler::server::router::tool::ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Part of a street address, e.g. "603 Putnam". Matched against covered buildings only.
    query: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct BblParams {
    /// A 10-digit NYC Borough-Block-Lot identifier, e.g. "3016440063".
    bbl: String,
}

#[tool_router]
impl HouseCheck {
    /// The line every response ends with.
    ///
    /// An agent will happily present "27 of 100" as a fact about New York. It is a fact
    /// about 250 buildings in one community district, from a dated snapshot, excluding one
    /// violation class. Saying so costs one line and is the difference between a citation
    /// and a rumour.
    fn provenance(&self) -> String {
        format!(
            "\n---\nSources: {}\nSnapshot {} · data from {}. Coverage: {} buildings in one \
             Brooklyn community district (~0.1% of the city). Excluded: {}. \
             A signal, not a legal ruling, and not legal advice.",
            self.sources, self.snapshot_year, self.data_month, self.building_count, self.excluded
        )
    }

    #[tool(
        description = "Find covered NYC buildings by partial street address. Returns BBL \
                       identifiers to pass to get_building_card. Only searches the 250 \
                       buildings in the covered community district."
    )]
    fn search_building(&self, Parameters(SearchParams { query }): Parameters<SearchParams>) -> String {
        let needle = query.trim().to_lowercase();
        if needle.len() < 3 {
            return "Give at least three characters of a street address.".into();
        }
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(e) => return format!("Could not read the artifact: {e}"),
        };
        let buildings = match store::get_all_buildings(&conn) {
            Ok(b) => b,
            Err(e) => return format!("Could not read the artifact: {e}"),
        };
        let hits: Vec<_> = buildings
            .iter()
            .filter(|b| b.address.to_lowercase().contains(&needle))
            .take(10)
            .collect();

        if hits.is_empty() {
            // Deliberately distinguished from "this building does not exist". Telling
            // someone their home is not real, when the truth is that we cover 0.1% of the
            // city, is the failure this wording exists to avoid.
            return format!(
                "No covered building matches \"{query}\". That means it is outside the \
                 covered district, not that the building or its violations do not exist.{}",
                self.provenance()
            );
        }

        let mut out = format!("{} covered building(s) matching \"{query}\":\n", hits.len());
        for b in hits {
            out.push_str(&format!("- {} · BBL {}\n", b.address, b.bbl));
        }
        out.push_str(&self.provenance());
        out
    }

    #[tool(
        description = "Get the Building Health Card for a BBL: a 0-100 score across four \
                       pillars, open violation counts by class, and how long this building \
                       actually takes to fix things."
    )]
    fn get_building_card(&self, Parameters(BblParams { bbl }): Parameters<BblParams>) -> String {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(e) => return format!("Could not read the artifact: {e}"),
        };
        let today = today_iso();
        let card = match card::card_for(&conn, self.snapshot_year, bbl.trim(), &today) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return format!(
                    "BBL {bbl} is not in the covered district. That is a coverage limit, \
                     not a statement about the building.{}",
                    self.provenance()
                )
            }
            Err(e) => return format!("Could not build the card: {e}"),
        };

        let s = &card.score;
        let v = &card.open_violations;
        let mut out = format!(
            "{}\nBBL {} · built {}\n\nScore {}/100 \
             (condition {}, legal {}, neighborhood {}, accessibility {})\n\
             Open violations: {} class A, {} class B, {} class C ({} total)\n\
             Accessibility likelihood: {}\n",
            card.building.address,
            card.building.bbl,
            card.building.year_built,
            s.total,
            s.condition,
            s.legal,
            s.neighborhood,
            s.accessibility,
            v.a,
            v.b,
            v.c,
            card.open_violation_total,
            card.access_likelihood,
        );

        // The three-state metric, stated in words. An agent handed a bare 0 would report
        // the landlord who fixes nothing as the fastest one in the district.
        out.push_str(&match &card.repair_speed {
            Some(model::RepairSpeed::Median {
                median_days,
                sample,
                since_year,
            }) => format!(
                "Repair speed: median {median_days} days to close, from {sample} \
                 violations closed since {since_year}.\n"
            ),
            Some(model::RepairSpeed::NothingClosed { open, since_year }) => format!(
                "Repair speed: nothing closed since {since_year}, with {open} still open. \
                 This is not a fast building -- it is one with no closures on record.\n"
            ),
            None => "Repair speed: not enough closed violations on record to state one. \
                     Absent, not zero.\n"
                .to_string(),
        });

        out.push_str(&self.provenance());
        out
    }
}

#[tool_handler(
    name = "housecheck",
    version = "0.1.0",
    instructions = "Look up NYC building conditions from public city data. Coverage is 250 \
                    buildings in one Brooklyn community district. Every figure comes with \
                    its source and its limits; present them together or not at all. This is \
                    a signal, not a legal ruling, and it is not legal advice."
)]
impl ServerHandler for HouseCheck {}

/// Today as `YYYY-MM-DD`. Mirrors the API's own helper; days-open arithmetic needs a date
/// the model can parse, not a timestamp.
fn today_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = civil_ymd(secs.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Civil date from days since the epoch. Howard Hinnant's algorithm.
fn civil_ymd(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[tokio::main]
async fn main() -> Result<()> {
    // stderr, never stdout: stdout is the MCP transport, and a stray log line there is a
    // protocol error rather than noise.
    tracing_subscriber::fmt().with_writer(std::io::stderr).init();

    let db = std::env::var("HOUSECHECK_DB").unwrap_or_else(|_| "data/housecheck.db".to_string());
    let conn = store::open_db_readonly(&db)?;
    let snapshot_year = store::get_snapshot_year(&conn)?.unwrap_or(2026);
    // Derived from the ingest timestamp exactly as `/meta` does it, rather than stored --
    // one fewer field that can be updated in one place and not the other.
    let data_month = store::get_meta(&conn, "ingested_at_unix")?
        .and_then(|s| s.parse::<i64>().ok())
        .map(|secs| {
            let (y, m, _) = civil_ymd(secs.div_euclid(86_400));
            format!("{} {}", MONTHS[(m as usize - 1).min(11)], y)
        })
        .unwrap_or_else(|| "an unrecorded date".into());
    let sources = store::get_meta(&conn, "sources")?.unwrap_or_else(|| "NYC open data".into());
    let excluded = store::get_meta(&conn, "violation_classes_excluded")?
        .unwrap_or_else(|| "see the repository".into());
    let building_count = store::building_count(&conn)?;
    tracing::info!(%db, snapshot_year, building_count, "housecheck-mcp ready");

    let server = HouseCheck {
        conn: Arc::new(Mutex::new(conn)),
        snapshot_year,
        data_month,
        sources,
        excluded,
        building_count,
        tool_router: HouseCheck::tool_router(),
    };

    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
