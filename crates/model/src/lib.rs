use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Building {
    pub bbl: String,
    pub address: String,
    pub year_built: i32,
    pub num_floors: i32,
    pub units_res: i32,
    pub tract_geoid: String,
    pub rent_stabilized: Option<bool>,
    pub good_cause: bool,
    pub has_elevator: bool,
    pub near_ada_subway_m: Option<i32>,
    pub complaints_311: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub class: String, // "A" | "B" | "C"
    pub open: bool,
    pub year: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViolationCounts {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl ViolationCounts {
    /// Count only OPEN violations by class.
    pub fn open_from(violations: &[Violation]) -> Self {
        let mut counts = ViolationCounts { a: 0, b: 0, c: 0 };
        for v in violations.iter().filter(|v| v.open) {
            match v.class.as_str() {
                "A" => counts.a += 1,
                "B" => counts.b += 1,
                "C" => counts.c += 1,
                _ => {}
            }
        }
        counts
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    pub total: u8,
    pub condition: u8,
    pub legal: u8,
    pub neighborhood: u8,
    pub accessibility: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCard {
    pub building: Building,
    pub score: ScoreBreakdown,
    pub open_violations: ViolationCounts,
    pub access_likelihood: String, // "Higher" | "Mixed" | "Lower"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RentFairness {
    pub bbl: String,
    pub user_rent: i32,
    pub tract_median: i32,
    pub pct_vs_median: f64,
    pub verdict: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_only_open_violations_by_class() {
        let vs = vec![
            Violation {
                class: "C".into(),
                open: true,
                year: 2025,
            },
            Violation {
                class: "C".into(),
                open: false,
                year: 2020,
            },
            Violation {
                class: "A".into(),
                open: true,
                year: 2024,
            },
        ];
        let counts = ViolationCounts::open_from(&vs);
        assert_eq!(counts, ViolationCounts { a: 1, b: 0, c: 1 });
    }
}
