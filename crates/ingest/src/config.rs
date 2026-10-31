/// Ingest run configuration, parsed from CLI args.
#[derive(Debug, PartialEq)]
pub struct Config {
    pub mode: Mode,
    pub out: String,
    pub community_district: u32, // e.g. 303 = Brooklyn CD3 (Bed-Stuy)
    pub limit: u32,              // cap building count for a demo-sized set
    /// Every residential lot in all five boroughs. Ignores `--cd` and `--limit`; explicit so
    /// an hour-long run never starts by accident.
    pub citywide: bool,
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    Fixture,
    Real,
    /// Reload only the law tables of an existing artifact, from `data/law.json`. A law fix
    /// should not need a fresh pull of every building.
    Law,
    /// Re-read every law source and report what changed. Writes a report, never the artifact,
    /// so it takes no `--out`. Its own flags are in `lawcheck::Opts`.
    LawCheck,
}

impl Config {
    /// Parse from an arg list (excluding program name). Errors are human strings.
    pub fn parse(args: &[String]) -> Result<Config, String> {
        let flag = |name: &str| args.iter().position(|a| a == name).map(|i| args.get(i + 1));
        let mode = if args.iter().any(|a| a == "--real") {
            Mode::Real
        } else if args.iter().any(|a| a == "--fixture") {
            Mode::Fixture
        } else if args.iter().any(|a| a == "--law") {
            Mode::Law
        } else if args.iter().any(|a| a == "--law-check") {
            Mode::LawCheck
        } else {
            return Err("must pass --fixture, --real, --law or --law-check".into());
        };
        let out = match flag("--out").flatten().cloned() {
            Some(out) => out,
            None if mode == Mode::LawCheck => String::new(),
            None => return Err("missing --out <path>".into()),
        };
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
        Ok(Config {
            mode,
            out,
            community_district,
            limit,
            citywide: args.iter().any(|a| a == "--citywide"),
        })
    }

    /// The DuckDB staging file, kept next to the artifact and between runs.
    pub fn stage_path(&self) -> String {
        format!("{}.stage.duckdb", self.out.trim_end_matches(".db"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_mode_with_defaults() {
        let args = ["--real", "--out", "data/hc.db"].map(String::from);
        let c = Config::parse(&args).unwrap();
        assert_eq!(
            c,
            Config {
                mode: Mode::Real,
                out: "data/hc.db".into(),
                community_district: 303,
                limit: 200,
                citywide: false,
            }
        );
        assert_eq!(c.stage_path(), "data/hc.stage.duckdb");
    }

    #[test]
    fn parses_citywide() {
        let args = ["--real", "--citywide", "--out", "x.db"].map(String::from);
        assert!(Config::parse(&args).unwrap().citywide);
    }

    #[test]
    fn parses_law_mode() {
        let args = ["--law", "--out", "data/hc.db"].map(String::from);
        assert_eq!(Config::parse(&args).unwrap().mode, Mode::Law);
    }

    #[test]
    fn parses_law_check_mode_without_out() {
        let args = ["--law-check", "--full"].map(String::from);
        assert_eq!(Config::parse(&args).unwrap().mode, Mode::LawCheck);
    }

    #[test]
    fn requires_a_mode() {
        let args = ["--out", "x"].map(String::from);
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
