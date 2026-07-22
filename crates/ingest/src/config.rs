/// Ingest run configuration, parsed from CLI args.
#[derive(Debug, PartialEq)]
pub struct Config {
    pub mode: Mode,
    pub out: String,
    pub community_district: u32, // e.g. 303 = Brooklyn CD3 (Bed-Stuy)
    pub limit: u32,              // cap building count for a demo-sized set
}

#[derive(Debug, PartialEq)]
pub enum Mode {
    Fixture,
    Real,
}

impl Config {
    /// Parse from an arg list (excluding program name). Errors are human strings.
    pub fn parse(args: &[String]) -> Result<Config, String> {
        let flag = |name: &str| args.iter().position(|a| a == name).map(|i| args.get(i + 1));
        let mode = if args.iter().any(|a| a == "--real") {
            Mode::Real
        } else if args.iter().any(|a| a == "--fixture") {
            Mode::Fixture
        } else {
            return Err("must pass --fixture or --real".into());
        };
        let out = flag("--out")
            .flatten()
            .cloned()
            .ok_or("missing --out <path>")?;
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
        })
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
                limit: 200
            }
        );
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
