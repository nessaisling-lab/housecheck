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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Violation {
    pub class: String, // "A" | "B" | "C"
    pub open: bool,
    pub year: i32,
    /// HPD's own notice text, verbatim — `§ 27-2005 ADM CODE … ABATE THE NUISANCE …`.
    ///
    /// This is the difference between a count and a condition: a petition has to plead
    /// what is wrong, not how many things are. Stored as the city wrote it, because that
    /// wording is what goes in a filing. Rewriting it into plain English is a separate
    /// job that needs a housing lawyer to validate, and is deliberately not done here.
    ///
    /// `None` where the record carries no text.
    #[serde(default)]
    pub description: Option<String>,
    /// ISO date the notice was issued (`novissueddate`), for "open for N days".
    ///
    /// **Not always present.** Populated on 10,345,990 of 11,156,924 citywide rows —
    /// 92.7% — so roughly one violation in fourteen has no issue date at all. Those must
    /// render as an unknown age rather than as zero days, which would read as "just
    /// raised" for a violation that may be a decade old.
    #[serde(default)]
    pub issued_on: Option<String>,
    /// ISO date the status last changed (`currentstatusdate`). For a closed violation
    /// this is when it closed, which is what time-to-close is computed from.
    #[serde(default)]
    pub closed_on: Option<String>,
}

impl Violation {
    /// Whole days between issue and close, or between issue and `today` if still open.
    ///
    /// `None` when the issue date is missing, which is the honest answer for the ~7% of
    /// records that have none — the alternative is a confident zero.
    ///
    /// `today` is passed in rather than read from the clock so the value is a pure
    /// function of the record and can be tested without freezing time.
    pub fn days_open(&self, today: &str) -> Option<i64> {
        let start = civil_days(self.issued_on.as_deref()?)?;
        let end = match self.closed_on.as_deref() {
            Some(d) if !self.open => civil_days(d)?,
            _ => civil_days(today)?,
        };
        Some((end - start).max(0))
    }
}

/// Days since 1970-01-01 for a `YYYY-MM-DD…` string. Howard Hinnant's civil-days
/// algorithm; it is correct across leap years and needs no date dependency.
fn civil_days(s: &str) -> Option<i64> {
    let y: i64 = s.get(0..4)?.parse().ok()?;
    let m: i64 = s.get(5..7)?.parse().ok()?;
    let d: i64 = s.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
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

/// Machine-readable rent-stabilization state.
///
/// This was a `String` with a doc comment listing `"on_record" | "not_found" | "unverified"`.
/// The constructor below emitted `"likely" | "none_on_record" | "unverified"` — two of the
/// three documented values existed nowhere in the workspace. Nothing could catch that, because
/// a doc comment is not checked against the function forty lines under it, and the frontend
/// got it right only by reading the JSON instead of the type.
///
/// The variants are the documentation now, so there is nothing left to drift from. The
/// `rename_all` is the single place the wire strings exist; `serializes_to_the_wire_contract`
/// pins them, because ten comparisons in `HealthCard.tsx` depend on these exact bytes and
/// changing them would fail silently rather than loudly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilizationStatus {
    /// A DOF record shows stabilized units.
    Likely,
    /// A DOF record exists and shows zero stabilized units.
    NoneOnRecord,
    /// No DOF record was found for this building. Not evidence either way.
    Unverified,
}

impl StabilizationStatus {
    /// Whether a DOF record actually backed this state — i.e. whether citing one is honest.
    /// `Unverified` means the lookup found nothing, which is not a source.
    pub fn has_dof_record(self) -> bool {
        matches!(self, Self::Likely | Self::NoneOnRecord)
    }

    /// The wire string, for prose that needs to name the state (the agent's grounding block).
    ///
    /// This is a second place the strings appear, which is exactly the shape of the original
    /// defect — so `wire_form_matches_serde` asserts it against what serde actually emits.
    /// Two hand-written copies of a fact are fine when a test makes them one fact.
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Likely => "likely",
            Self::NoneOnRecord => "none_on_record",
            Self::Unverified => "unverified",
        }
    }
}

impl std::fmt::Display for StabilizationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_wire())
    }
}

/// Honest, three-state rent-stabilization signal for the Health Card. Public stabilization
/// lists are incomplete and never a legal ruling, so the wording is deliberately hedged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stabilization {
    pub status: StabilizationStatus,
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
                status: StabilizationStatus::Likely,
                message: format!(
                    "Likely rent-stabilized — {} units on the latest NYC DOF record (2024). \
                     A signal, not a legal ruling; confirm with DHCR.",
                    rent_stab_units.unwrap_or(0)
                ),
            },
            Some(false) => Stabilization {
                status: StabilizationStatus::NoneOnRecord,
                message: "No stabilized units on the latest DOF record (2024) — public data \
                          lags, so not proof it is market-rate."
                    .into(),
            },
            None => Stabilization {
                status: StabilizationStatus::Unverified,
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

    /// The wire contract. `HealthCard.tsx` compares `stabilization` against these exact
    /// strings in ten places, and `types/building.ts` declares them as a closed union — so
    /// renaming a variant without updating both would not fail a build, it would silently
    /// send every card down the frontend's else-branch. A building with twelve stabilized
    /// units would render "Rent stabilized: No".
    ///
    /// This is the test the old `String` field could not have. There was nothing to pin:
    /// the value was constructed inline at three call sites and described, wrongly, in a
    /// doc comment.
    #[test]
    fn serializes_to_the_wire_contract() {
        let json = |s: StabilizationStatus| serde_json::to_string(&s).unwrap();
        assert_eq!(json(StabilizationStatus::Likely), "\"likely\"");
        assert_eq!(json(StabilizationStatus::NoneOnRecord), "\"none_on_record\"");
        assert_eq!(json(StabilizationStatus::Unverified), "\"unverified\"");

        // And round-trips, so a stored/cached card still deserializes.
        for s in [
            StabilizationStatus::Likely,
            StabilizationStatus::NoneOnRecord,
            StabilizationStatus::Unverified,
        ] {
            let back: StabilizationStatus = serde_json::from_str(&json(s)).unwrap();
            assert_eq!(back, s);
        }
    }

    /// `as_wire` and `rename_all` are two hand-maintained spellings of the same fact, which is
    /// the defect this whole enum exists to remove. This is what makes them one fact.
    #[test]
    fn wire_form_matches_serde() {
        for s in [
            StabilizationStatus::Likely,
            StabilizationStatus::NoneOnRecord,
            StabilizationStatus::Unverified,
        ] {
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.as_wire()),
                "as_wire disagrees with serde for {s:?}"
            );
            assert_eq!(s.to_string(), s.as_wire(), "Display disagrees with as_wire");
        }
    }

    #[test]
    fn only_a_real_dof_record_counts_as_a_source() {
        assert!(StabilizationStatus::Likely.has_dof_record());
        assert!(StabilizationStatus::NoneOnRecord.has_dof_record());
        // "unverified" means the lookup found nothing. Citing DHCR here is the defect that
        // over-claimed a source on 163 of 250 shipped buildings.
        assert!(!StabilizationStatus::Unverified.has_dof_record());
    }

    #[test]
    fn from_units_maps_the_tri_state() {
        assert_eq!(
            Stabilization::from_units(Some(true), Some(12)).status,
            StabilizationStatus::Likely
        );
        assert_eq!(
            Stabilization::from_units(Some(false), Some(0)).status,
            StabilizationStatus::NoneOnRecord
        );
        assert_eq!(
            Stabilization::from_units(None, None).status,
            StabilizationStatus::Unverified
        );
    }

    /// `days_open` decides how old a violation looks, so its failure modes matter more
    /// than its happy path. The one that would do damage is a missing issue date
    /// silently becoming zero: 7.3% of citywide rows have no `novissueddate`, and a
    /// decade-old violation rendering as "raised today" is worse than rendering as
    /// unknown.
    #[test]
    fn days_open_is_none_rather_than_zero_when_the_date_is_missing() {
        let no_date = Violation {
            class: "C".into(),
            open: true,
            ..Default::default()
        };
        assert_eq!(no_date.days_open("2026-08-09"), None);

        // Still open: measured against today.
        let open = Violation {
            class: "C".into(),
            open: true,
            issued_on: Some("2026-03-14".into()),
            ..Default::default()
        };
        assert_eq!(open.days_open("2026-08-09"), Some(148));

        // Closed: measured to the close date, not to today.
        let closed = Violation {
            class: "C".into(),
            open: false,
            issued_on: Some("2014-01-06".into()),
            closed_on: Some("2014-01-25".into()),
            ..Default::default()
        };
        assert_eq!(closed.days_open("2026-08-09"), Some(19));

        // A close date on an OPEN violation is a status change, not a closure, so the
        // age still runs to today.
        let reopened = Violation {
            class: "B".into(),
            open: true,
            issued_on: Some("2026-08-01".into()),
            closed_on: Some("2026-08-03".into()),
            ..Default::default()
        };
        assert_eq!(reopened.days_open("2026-08-09"), Some(8));

        // Leap day is real: 2024 had one, 2100 will not.
        let leap = Violation {
            class: "A".into(),
            open: true,
            issued_on: Some("2024-02-28".into()),
            ..Default::default()
        };
        assert_eq!(leap.days_open("2024-03-01"), Some(2));

        // Dirty data must not produce a negative age.
        let backwards = Violation {
            class: "A".into(),
            open: false,
            issued_on: Some("2020-06-01".into()),
            closed_on: Some("2019-01-01".into()),
            ..Default::default()
        };
        assert_eq!(backwards.days_open("2026-08-09"), Some(0));

        // Unparseable dates are missing dates, not a panic.
        let junk = Violation {
            class: "A".into(),
            open: true,
            issued_on: Some("not-a-date".into()),
            ..Default::default()
        };
        assert_eq!(junk.days_open("2026-08-09"), None);
    }

    #[test]
    fn counts_only_open_violations_by_class() {
        let vs = vec![
            Violation {
                class: "C".into(),
                open: true,
                year: 2025,
                ..Default::default()
            },
            Violation {
                class: "C".into(),
                open: false,
                year: 2020,
                ..Default::default()
            },
            Violation {
                class: "A".into(),
                open: true,
                year: 2024,
                ..Default::default()
            },
        ];
        let counts = ViolationCounts::open_from(&vs);
        assert_eq!(counts, ViolationCounts { a: 1, b: 0, c: 1 });
    }
}
