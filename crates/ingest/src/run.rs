use crate::config::Config;
use anyhow::Result;

/// Real ingest — implemented in Plan 2 Task 5 (needs Census API key + network).
pub fn run_real(_cfg: &Config) -> Result<()> {
    anyhow::bail!("--real ingest not yet wired (Plan 2 Task 5); use --fixture for now")
}
