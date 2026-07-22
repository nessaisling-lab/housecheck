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
