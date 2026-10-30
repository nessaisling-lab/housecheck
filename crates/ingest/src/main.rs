mod config;
mod geo;
mod run;
mod sources;

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
        Mode::Law => {
            // Never create one: an artifact with law and no buildings would fail at startup
            // anyway, but a typo in --out should say so here.
            anyhow::ensure!(
                std::path::Path::new(&cfg.out).exists(),
                "{} does not exist; build it with --real or --fixture first",
                cfg.out
            );
            let conn = store::open_db(&cfg.out)?;
            store::migrate(&conn)?;
            let n = store::load_law(&conn)?;
            println!("loaded {n} law provisions into {}", cfg.out);
        }
    }
    Ok(())
}
