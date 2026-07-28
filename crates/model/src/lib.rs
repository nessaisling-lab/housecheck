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
    /// Count of rent-stabilized units on the latest NYC DOF Statement-of-Account record
    /// (JustFix nyc-doffer, latest year 2024). `Some(n>0)` pairs with `rent_stabilized =
    /// Some(true)`; `Some(0)` with `Some(false)`; `None` when the building has no DOF record.
    pub rent_stab_units: Option<i32>,
    pub good_cause: bool,
    pub has_elevator: bool,
    pub near_ada_subway_m: Option<i32>,
    pub complaints_311: i32,
    /// Building centroid (from PLUTO), stored so the frontend map can plot the curated set.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// Letter grade of the nearest DOHMH-graded restaurant within ~200 m. Neighborhood
    /// context only — display, never folded into any score.
    pub restaurant_grade: Option<String>,
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

/// Honest, three-state rent-stabilization signal for the Health Card. Public stabilization
/// lists are incomplete and never a legal ruling, so the wording is deliberately hedged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stabilization {
    /// "on_record" | "not_found" | "unverified" — machine-readable state for the frontend.
    pub status: String,
    /// Human wording shown to tenants.
    pub message: String,
}

impl Stabilization {
    /// Map the stored `rent_stabilized` tri-state plus its unit count into the honest display
    /// wording. Backed by JustFix nyc-doffer (NYC DOF Statement-of-Account records, latest year
    /// 2024): `Some(true)` carries the unit count `n`, `Some(false)` means zero units on the
    /// latest record, `None` means no DOF record was found for the building.
    pub fn from_units(rent_stabilized: Option<bool>, rent_stab_units: Option<i32>) -> Self {
        match rent_stabilized {
            Some(true) => Stabilization {
                status: "likely".into(),
                message: format!(
                    "Likely rent-stabilized — {} units on the latest NYC DOF record (2024). \
                     A signal, not a legal ruling; confirm with DHCR.",
                    rent_stab_units.unwrap_or(0)
                ),
            },
            Some(false) => Stabilization {
                status: "none_on_record".into(),
                message: "No stabilized units on the latest DOF record (2024) — public data \
                          lags, so not proof it is market-rate."
                    .into(),
            },
            None => Stabilization {
                status: "unverified".into(),
                message: "Unverified — no DOF stabilization record found for this building.".into(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCard {
    pub building: Building,
    pub score: ScoreBreakdown,
    pub open_violations: ViolationCounts,
    pub access_likelihood: String, // "Higher" | "Mixed" | "Lower"
    pub stabilization: Stabilization,
}

/// Current HUD Fair Market Rents by bedroom count for the building's metro area. Second
/// comparator alongside the Census tract median in `/rent-fairness`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudFmr {
    pub area: String,
    pub fiscal_year: i32,
    pub studio: i32,
    pub one_br: i32,
    pub two_br: i32,
    pub three_br: i32,
}

impl HudFmr {
    /// FY2026 HUD Fair Market Rents for the New York, NY HUD Metro FMR Area (covers Kings
    /// County / Brooklyn, our curated set), effective Oct 1, 2025 through Sep 30, 2026.
    /// Source: HUD USER FY2026 Fair Market Rent Documentation System
    /// (https://www.huduser.gov/portal/datasets/fmr.html). No HUD API key required — the four
    /// area-wide figures are embedded as constants. FY2025 was 2233/2330/2580/3215; the FY2026
    /// step up is consistent with HUD's published revision.
    pub fn ny_metro_fy2026() -> Self {
        HudFmr {
            area: "New York, NY HUD Metro FMR Area".into(),
            fiscal_year: 2026,
            studio: 2529,
            one_br: 2655,
            two_br: 2910,
            three_br: 3644,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RentFairness {
    pub bbl: String,
    pub user_rent: i32,
    pub tract_median: i32,
    pub pct_vs_median: f64,
    pub verdict: String,
    pub hud_fmr: HudFmr,
}

/// Compact building row for the `GET /buildings` list/map endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildingListItem {
    pub bbl: String,
    pub address: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub score: u8,
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
