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
