pub mod export;
pub mod law;
pub mod urgency;

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
    /// A Good Cause Eviction exemption public records show for the whole building, chosen at
    /// ingest. `None` means none was found, which is not the same as covered.
    pub good_cause_exemption: Option<GoodCauseExemption>,
    /// What backs `good_cause_exemption`: the certificate of occupancy's date (YYYY-MM-DD) or
    /// the housing program's name. `None` for the exemptions read from the building class.
    pub good_cause_detail: Option<String>,
    pub has_elevator: bool,
    pub near_ada_subway_m: Option<i32>,
    /// ZIP code (PLUTO `zipcode`): the area the neighborhood figures below are counted over.
    pub zip: Option<String>,
    /// Neighborhood name: the 2020 NTA the building's census tract falls in (DCP `hm78-6dwm`).
    /// A label only; nothing is computed from it.
    pub neighborhood: Option<String>,
    /// 311 complaints since 2024-01-01 in this building's ZIP, per 1,000 homes in that ZIP
    /// (PLUTO `unitsres`). The same figure for every building in the ZIP: it describes the
    /// area, not the block. `None` when the ZIP is unknown or has no homes on record.
    pub complaints_311_per_1k_homes: Option<i32>,
    /// Building centroid (from PLUTO), stored so the frontend map can plot the curated set.
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// HPD's hazard classes, as the city defines them.
///
/// This was a `String` with `// "A" | "B" | "C"` beside it, and that comment was the only thing
/// enforcing the rule. Every dispatch site consequently needed a fallthrough arm: `scoring`
/// gave an unrecognised class a penalty of **0** and `ViolationCounts` counted it as nothing,
/// so a class the code did not know about made a building look *better* — silently, and in the
/// direction that understates hazard. Those arms are gone. Adding a variant here now fails the
/// build at every site that has to decide what it means.
///
/// **The wire strings are load-bearing twice.** `HealthCard.tsx` renders them (`Class {d.class}`),
/// and `export::canonical` hashes them into the signed payload — so changing them would not merely
/// break the frontend, it would invalidate every export already issued. `wire_form_is_the_letter`
/// pins them against what serde actually emits, the same way `wire_form_matches_serde` does for
/// [`StabilizationStatus`], and `the_typed_class_signs_the_same_bytes_as_the_string_it_replaced`
/// pins the signed bytes against `main` from before this type existed.
///
/// Class I is deliberately absent. HPD issues it, `ingest` skips it and counts it by letter (753
/// on the 250-building CD 303 pilot), and the count is stated on the card and in `/meta` — so a
/// value that never reaches the database is not a state this type should be able to represent.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Default,
)]
pub enum ViolationClass {
    /// Non-hazardous.
    ///
    /// The default only because `Violation::default()` is a test construct and every caller
    /// sets the class explicitly. Neither direction is harmless: defaulting to `C` would make
    /// fixtures look hazardous and hide real ones.
    #[default]
    A,
    /// Hazardous.
    B,
    /// Immediately hazardous — no heat, no hot water, lead paint. Shortest correction deadline.
    C,
}

impl ViolationClass {
    /// Every class. Lets a test iterate the whole set rather than a hand-copied list that can
    /// fall behind the enum.
    pub const ALL: [Self; 3] = [Self::A, Self::B, Self::C];

    /// The single letter HPD uses. Must equal what serde emits — `wire_form_is_the_letter`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

impl std::fmt::Display for ViolationClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A class letter that is not one HPD issues.
///
/// Carries the offending value: "the artifact holds a class we do not model" is only actionable
/// if it says which one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownViolationClass(pub String);

impl std::fmt::Display for UnknownViolationClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not an HPD violation class: {:?}", self.0)
    }
}

impl std::error::Error for UnknownViolationClass {}

impl std::str::FromStr for ViolationClass {
    type Err = UnknownViolationClass;

    /// Case-insensitive, because `export` already compared with `eq_ignore_ascii_case` and
    /// narrowing that at the same time as introducing the type would be two changes wearing
    /// one commit.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "A" | "a" => Ok(Self::A),
            "B" | "b" => Ok(Self::B),
            "C" | "c" => Ok(Self::C),
            other => Err(UnknownViolationClass(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Violation {
    pub class: ViolationClass,
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
            // Exhaustive on purpose: no fallthrough arm, so a new class cannot be silently
            // counted as nothing. Adding a variant breaks this match, which is the point.
            match v.class {
                ViolationClass::A => counts.a += 1,
                ViolationClass::B => counts.b += 1,
                ViolationClass::C => counts.c += 1,
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

/// A Good Cause Eviction exemption (RPL § 214) that public records show for a whole building.
///
/// The building-class ones follow JustFix's Good Cause screener (PLUTO `bldgclass`), as
/// Aisling decided on 2026-10-31, as does counting condo rentals (RR) as condos and counting a
/// certificate of occupancy of any type. A renovation's certificate is kept apart from a new
/// building's so the card can say it is the less certain case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoodCauseExemption {
    /// § 214(7). PLUTO class R*.
    Condo,
    /// § 214(7). PLUTO C6, C8, CC, D0, DC, D4.
    Coop,
    /// § 214(12). PLUTO H* except H8.
    Hotel,
    /// § 214(13). PLUTO H8.
    Dormitory,
    /// § 214(10). PLUTO I*.
    CareFacility,
    /// § 214(14). PLUTO M*.
    Religious,
    /// PLUTO W*: school and college buildings, which JustFix treats as exempt.
    School,
    /// § 214(11). PLUTO CM.
    ManufacturedHomes,
    /// § 214(5)/(6): public or subsidized housing, from a DOF tax exemption.
    Subsidized,
    /// § 214(8): a new building's certificate of occupancy dated 2009-01-01 or later.
    NewBuilding,
    /// § 214(8), the less certain case: a certificate after a renovation (DOB alteration job).
    Renovation,
}

impl GoodCauseExemption {
    pub const ALL: [Self; 11] = [
        Self::Condo,
        Self::Coop,
        Self::Hotel,
        Self::Dormitory,
        Self::CareFacility,
        Self::Religious,
        Self::School,
        Self::ManufacturedHomes,
        Self::Subsidized,
        Self::NewBuilding,
        Self::Renovation,
    ];

    /// The stored and wire string. `wire_form_matches_serde` pins it against serde.
    pub fn as_wire(self) -> &'static str {
        match self {
            Self::Condo => "condo",
            Self::Coop => "coop",
            Self::Hotel => "hotel",
            Self::Dormitory => "dormitory",
            Self::CareFacility => "care_facility",
            Self::Religious => "religious",
            Self::School => "school",
            Self::ManufacturedHomes => "manufactured_homes",
            Self::Subsidized => "subsidized",
            Self::NewBuilding => "new_building",
            Self::Renovation => "renovation",
        }
    }

    pub fn from_wire(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.as_wire() == s)
    }
}

/// How the Health Card states Good Cause coverage. Never "covered": coverage is per apartment
/// and turns on facts no public record holds (the landlord's units statewide, whether they live
/// there, the rent), so the most public data can say is that no exemption showed up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoodCauseStatus {
    /// A whole-building exemption, or every apartment stabilized.
    LikelyNotCovered,
    /// Some apartments stabilized (those are exempt, § 214(5)), the others unknown.
    PartlyStabilized,
    /// 10 or fewer apartments: the small-landlord and owner-occupied exemptions can't be seen.
    MightBeCovered,
    /// 11 or more apartments and nothing in the records.
    NoExemptionFound,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoodCause {
    pub status: GoodCauseStatus,
    /// Human wording shown to tenants.
    pub message: String,
}

impl GoodCause {
    /// Whole-building exemptions first, then stabilization, then the unit count.
    ///
    /// ponytail: one reason per building. A building with a renovation certificate and some
    /// stabilized apartments shows the certificate; the stabilization section still shows the
    /// apartments.
    pub fn from_building(b: &Building) -> Self {
        use GoodCauseStatus::*;
        let detail = b.good_cause_detail.as_deref().unwrap_or("");
        let (status, message) = match b.good_cause_exemption {
            Some(e) => (
                LikelyNotCovered,
                match e {
                    GoodCauseExemption::Condo => {
                        "Likely not covered: this is a condo building. Good Cause doesn't cover \
                         condos or co-ops."
                            .to_string()
                    }
                    GoodCauseExemption::Coop => {
                        "Likely not covered: this is a co-op building. Good Cause doesn't cover \
                         condos or co-ops."
                            .to_string()
                    }
                    GoodCauseExemption::Hotel => {
                        "Likely not covered: city tax records list this as a hotel or rooming \
                         house."
                            .to_string()
                    }
                    GoodCauseExemption::Dormitory => {
                        "Likely not covered: city tax records list this as a dormitory.".to_string()
                    }
                    GoodCauseExemption::CareFacility => {
                        "Likely not covered: city tax records list this as a hospital, nursing \
                         home or care facility."
                            .to_string()
                    }
                    GoodCauseExemption::Religious => {
                        "Likely not covered: city tax records list this as a religious building."
                            .to_string()
                    }
                    GoodCauseExemption::School => {
                        "Likely not covered: city tax records list this as a school or college \
                         building."
                            .to_string()
                    }
                    GoodCauseExemption::ManufacturedHomes => {
                        "Likely not covered: this is a manufactured home park.".to_string()
                    }
                    GoodCauseExemption::Subsidized => format!(
                        "Likely not covered: public or subsidized housing ({detail}). Rents here \
                         are set under a government program."
                    ),
                    GoodCauseExemption::NewBuilding => format!(
                        "Likely not covered: newer building. Its certificate of occupancy was \
                         issued {detail}, and Good Cause doesn't cover buildings with one from \
                         2009 or later."
                    ),
                    GoodCauseExemption::Renovation => format!(
                        "Likely not covered: the building got a certificate of occupancy after a \
                         renovation, issued {detail}. Whether a renovation certificate counts is \
                         less certain, so check the Good Cause notice on your lease."
                    ),
                },
            ),
            None => match (b.rent_stab_units, b.units_res) {
                (Some(stab), units) if stab > 0 && stab >= units => (
                    LikelyNotCovered,
                    "Likely not covered: rent-stabilized. Stabilized apartments have their own, \
                     stronger protection."
                        .to_string(),
                ),
                (Some(stab), _) if stab > 0 => (
                    PartlyStabilized,
                    "Rent-stabilized apartments here are not covered (they have stronger \
                     protection). Other apartments might be."
                        .to_string(),
                ),
                (_, units) if units <= 10 => (
                    MightBeCovered,
                    "Might be covered: it depends on your landlord. Good Cause doesn't cover \
                     landlords who own 10 or fewer apartments in New York State, or who live in \
                     a building with 10 or fewer apartments."
                        .to_string(),
                ),
                _ => (
                    NoExemptionFound,
                    "No exemption found in public records. Good Cause may cover apartments \
                     here; your rent and your lease's Good Cause notice decide."
                        .to_string(),
                ),
            },
        };
        GoodCause { status, message }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthCard {
    pub building: Building,
    pub score: ScoreBreakdown,
    pub open_violations: ViolationCounts,
    /// The conditions behind the counts, newest first, capped at [`OPEN_DETAIL_CAP`].
    ///
    /// `open_violations` says how many. This says *what* — which is the difference between
    /// a number and an argument, and the reason a count can be read backwards: a building
    /// can truthfully show "no hazardous violations" beside a floor-level score when it
    /// has thirty-three non-hazardous ones.
    pub open_violation_details: Vec<ViolationDetail>,
    /// How many open violations exist in total, so a truncated list can say so rather than
    /// implying the capped list is all of them. One pilot building has 754.
    pub open_violation_total: u32,
    pub access_likelihood: String, // "Higher" | "Mixed" | "Lower"
    pub stabilization: Stabilization,
    /// What public records suggest about Good Cause Eviction. Not part of any score.
    pub good_cause: GoodCause,
    /// How long this building's violations actually take to get fixed. `None` when the
    /// record cannot support the claim — see [`RepairSpeed`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_speed: Option<RepairSpeed>,
}

/// Median days from a violation being issued to it being closed, for one building.
///
/// # Why this is the best value-per-effort signal in the project
///
/// Every other measure here describes a building's *state*: how many violations are open, how
/// bad they are, how long they have festered. This is the only one that describes **behaviour**
/// — whether whoever is responsible for this building fixes things or waits. Two buildings can
/// show identical open counts while one closes its violations in three weeks and the other in
/// three years, and a renter deciding whether to sign has no way to tell them apart from a
/// snapshot.
///
/// It costs no landlord participation, no identity verification and no legal exposure: it is
/// arithmetic over dates HPD already publishes.
///
/// # Why it is per building and not per landlord
///
/// The sprint specified "per landlord" and **the artifact cannot support that**. There is no
/// owner column in `buildings` or anywhere else; owner linkage lives in a separate HPD
/// registration dataset that has never been ingested. Rather than silently compute something
/// narrower and label it with the word the sprint used, this is named for what it measures.
/// One landlord's twelve buildings remain twelve unrelated records until that dataset lands.
///
/// # Why it carries its own basis
///
/// A median with no sample size is a number a reader cannot argue with, and a number nobody
/// can argue with does not belong on a page that also says "a signal, not a legal ruling".
/// `sample` and `since_year` travel with it so the figure can never be quoted without them.
/// # Three states, and the third one is the point
///
/// The first version of this returned a median or nothing, and that was wrong in a way the
/// measurements caught immediately. **603 Putnam Avenue has 33 open violations, has closed one
/// in its entire record, and that closure was in October 2017.** With two states it rendered as
/// blank — so the building that fixes nothing showed *no data* while a building that fixes
/// things in 100 days showed a number, and the worse landlord looked emptier rather than worse.
///
/// That is the same defect as "no hazardous violations" printing beside a floor-level score:
/// an absence that reads as reassurance. It is not rare either — **26 of the 250 pilot
/// buildings** have five or more open violations and *zero* closures in the window.
///
/// So `NothingClosed` is its own answer, exactly as `IntactUnsigned` is its own answer in the
/// export. Collapsing it into "no data" would be the same mistake in a different file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RepairSpeed {
    /// Enough closures to state a typical repair time.
    Median {
        /// Median days from issue to close. Median rather than mean because the distribution
        /// is violently skewed — measured on the pilot, one building's closures range from 6
        /// days to 1,075, and a mean would be dragged around by the tail.
        median_days: i64,
        /// How many closed violations the median is computed over. Never below
        /// [`REPAIR_SPEED_MIN_SAMPLE`].
        sample: u32,
        /// Only violations closed on or after 1 January of this year were counted.
        since_year: i32,
    },
    /// Open violations on the record and **nothing closed** in the window. The absence is the
    /// finding, so it is reported rather than omitted.
    NothingClosed {
        /// How many are sitting open, so the statement has a size.
        open: u32,
        since_year: i32,
    },
}

/// Fewest closed violations a median may be computed from.
///
/// A median over one or two closures is noise wearing a number's clothes, and this figure is
/// meant to be read as a claim about a building's behaviour. Measured on the pilot: at five,
/// **106 of 250** buildings qualify.
pub const REPAIR_SPEED_MIN_SAMPLE: u32 = 5;

/// Only count violations closed on or after this year.
///
/// **A judgment call, and the measurements are why.** Across the pilot the central tendency is
/// almost unmoved by the window — median-of-medians is 121 days all-time, 118 since 2023, 119
/// since 2024 — but the *range* collapses from **0–4,951 days** to **25–1,676**. All-time
/// includes a building whose median closure is 13.5 years, which is a fact about a dormant
/// record rather than about anyone managing the place today.
///
/// Three years keeps the most buildings (106, against 95 at two years) while discarding the
/// absurd tail. A tenant lawyer building a pattern-of-neglect argument might legitimately want
/// all-time; a renter deciding this week wants recent behaviour. This picks the renter.
pub const REPAIR_SPEED_SINCE_YEAR: i32 = 2023;

impl RepairSpeed {
    /// Classify a building's repair record.
    ///
    /// Takes the durations rather than a connection so the statistic is a pure function that
    /// can be tested without a database — the same reason `days_open` takes `today` instead of
    /// reading a clock.
    ///
    /// `None` is reserved for buildings that genuinely have nothing to say: too few closures to
    /// median *and* too few open violations for the silence to mean anything. A building with
    /// no violations at all is not a slow one.
    pub fn classify(mut durations: Vec<i64>, open: u32, since_year: i32) -> Option<Self> {
        // A close date before the issue date is a contradiction in the source, not a fast
        // repair. Measured on the pilot: zero such rows, so this is a guard rather than a
        // filter -- but a citywide ingest will meet one eventually, and a negative duration
        // would pull a median *down*, making a bad building look good.
        durations.retain(|&d| d >= 0);

        if (durations.len() as u32) >= REPAIR_SPEED_MIN_SAMPLE {
            durations.sort_unstable();
            let n = durations.len();
            // Even-length medians average the middle pair, so "half the repairs took longer
            // than this" stays true rather than approximately true.
            let median_days = if n % 2 == 1 {
                durations[n / 2]
            } else {
                (durations[n / 2 - 1] + durations[n / 2]) / 2
            };
            return Some(Self::Median {
                median_days,
                sample: n as u32,
                since_year,
            });
        }

        // Nothing closed, and enough sitting open that the silence is a finding rather than a
        // small sample. The same threshold as the median's, so one number governs when this
        // card is willing to make a claim at all.
        if durations.is_empty() && open >= REPAIR_SPEED_MIN_SAMPLE {
            return Some(Self::NothingClosed { open, since_year });
        }

        None
    }
}

/// The most detail any one card returns. A building in the pilot has 754 open violations,
/// and serialising every one would make a single card response larger than the entire
/// database was two commits ago. The total travels alongside so nothing is hidden.
pub const OPEN_DETAIL_CAP: usize = 50;

/// One open violation, as the card shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViolationDetail {
    pub class: ViolationClass,
    /// HPD's own wording. `None` where the record carries no text.
    pub description: Option<String>,
    pub issued_on: Option<String>,
    /// `None` when the issue date is missing — 7.3% of citywide rows — because an unknown
    /// age has to read as unknown rather than as zero days.
    pub days_open: Option<i64>,
}

impl ViolationDetail {
    /// Newest first, capped. Violations with no issue date sort last: they are the ones
    /// whose age is unknown, and leading with them would bury the recent ones.
    pub fn from_open(violations: &[Violation], today: &str) -> (Vec<Self>, u32) {
        let mut open: Vec<&Violation> = violations.iter().filter(|v| v.open).collect();
        let total = open.len() as u32;
        open.sort_by(|a, b| b.issued_on.cmp(&a.issued_on));
        let details = open
            .into_iter()
            .take(OPEN_DETAIL_CAP)
            .map(|v| ViolationDetail {
                class: v.class,
                description: v.description.clone(),
                issued_on: v.issued_on.clone(),
                days_open: v.days_open(today),
            })
            .collect();
        (details, total)
    }
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
    /// FY2027 HUD Fair Market Rents for the New York, NY HUD Metro FMR Area
    /// (METRO35620MM5600, which covers all five boroughs), effective
    /// Oct 1, 2026 (91 FR 56156); New York is not on HUD's FY2027 reevaluation list.
    /// Source: HUD USER FMR Documentation System
    /// (https://www.huduser.gov/portal/datasets/fmr.html). No HUD API key required — the four
    /// area-wide figures are embedded as constants. FY2026 was 2529/2655/2910/3644.
    pub fn ny_metro_fy2027() -> Self {
        HudFmr {
            area: "New York, NY HUD Metro FMR Area".into(),
            fiscal_year: 2027,
            studio: 2593,
            one_br: 2729,
            two_br: 2971,
            three_br: 3760,
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
        assert_eq!(
            json(StabilizationStatus::NoneOnRecord),
            "\"none_on_record\""
        );
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

    /// `as_str`, `Display`, `FromStr` and serde are four spellings of one letter. The card
    /// renders it and the export chain hashes it, so all four must agree for every class.
    #[test]
    fn wire_form_is_the_letter() {
        for c in ViolationClass::ALL {
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.as_str()),
                "as_str disagrees with serde for {c:?}"
            );
            assert_eq!(c.to_string(), c.as_str(), "Display disagrees with as_str");
            assert_eq!(c.as_str().parse::<ViolationClass>(), Ok(c));
            let back: ViolationClass = serde_json::from_str(&format!("\"{c}\"")).unwrap();
            assert_eq!(back, c);
        }
        // HPD's fourth class is not one this type models: it must fail to parse, not
        // become A.
        assert_eq!(
            "I".parse::<ViolationClass>(),
            Err(UnknownViolationClass("I".into()))
        );
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

    #[test]
    fn good_cause_wire_form_matches_serde() {
        for e in GoodCauseExemption::ALL {
            assert_eq!(
                serde_json::to_string(&e).unwrap(),
                format!("\"{}\"", e.as_wire())
            );
            assert_eq!(GoodCauseExemption::from_wire(e.as_wire()), Some(e));
        }
        assert_eq!(GoodCauseExemption::from_wire("covered"), None);
        assert_eq!(
            serde_json::to_string(&GoodCauseStatus::PartlyStabilized).unwrap(),
            "\"partly_stabilized\""
        );
    }

    /// One building per branch, in the order `from_building` tries them.
    #[test]
    fn good_cause_never_says_covered() {
        let b = |units: i32, stab: Option<i32>, e: Option<GoodCauseExemption>, d: Option<&str>| {
            GoodCause::from_building(&Building {
                bbl: "3000010001".into(),
                address: String::new(),
                year_built: 1920,
                num_floors: 4,
                units_res: units,
                tract_geoid: String::new(),
                rent_stabilized: stab.map(|n| n > 0),
                rent_stab_units: stab,
                good_cause_exemption: e,
                good_cause_detail: d.map(str::to_string),
                has_elevator: false,
                near_ada_subway_m: None,
                zip: None,
                neighborhood: None,
                complaints_311_per_1k_homes: None,
                latitude: None,
                longitude: None,
            })
        };
        use GoodCauseExemption as E;
        use GoodCauseStatus as S;

        // A whole-building exemption wins over stabilization and the unit count.
        let new = b(40, Some(12), Some(E::NewBuilding), Some("2015-03-02"));
        assert_eq!(new.status, S::LikelyNotCovered);
        assert!(new.message.contains("2015-03-02"), "{}", new.message);
        let reno = b(40, None, Some(E::Renovation), Some("2019-06-01"));
        assert!(reno.message.contains("less certain"), "{}", reno.message);
        let ml = b(200, None, Some(E::Subsidized), Some("Mitchell-Lama"));
        assert!(ml.message.contains("Mitchell-Lama"), "{}", ml.message);

        assert_eq!(b(12, Some(12), None, None).status, S::LikelyNotCovered);
        assert_eq!(b(40, Some(12), None, None).status, S::PartlyStabilized);
        assert_eq!(b(8, Some(0), None, None).status, S::MightBeCovered);
        assert_eq!(b(10, None, None, None).status, S::MightBeCovered);
        assert_eq!(b(11, None, None, None).status, S::NoExemptionFound);

        for e in E::ALL {
            let m = b(40, None, Some(e), Some("x")).message;
            assert!(m.starts_with("Likely not covered"), "{e:?}: {m}");
        }
    }

    /// `days_open` decides how old a violation looks, so its failure modes matter more
    /// than its happy path. The one that would do damage is a missing issue date
    /// silently becoming zero: 7.3% of citywide rows have no `novissueddate`, and a
    /// decade-old violation rendering as "raised today" is worse than rendering as
    /// unknown.
    /// One pilot building has 754 open violations, so the card caps the detail it returns.
    /// The cap is only safe if the total travels with it — otherwise a truncated list
    /// reads as the whole story, which is the same defect as reporting a count with no
    /// meaning, just in the other direction.
    #[test]
    fn violation_detail_caps_the_list_but_never_the_count() {
        let v = |issued: Option<&str>, open: bool| Violation {
            class: ViolationClass::C,
            open,
            year: 2026,
            description: Some("ABATE THE NUISANCE".into()),
            issued_on: issued.map(str::to_string),
            ..Default::default()
        };
        // 60 real dates: 2025-01-01 through 2025-02-29 would not exist, so walk months.
        let mut vs: Vec<Violation> = (0..60)
            .map(|i| {
                v(
                    Some(&format!("2025-{:02}-{:02}", i / 28 + 1, i % 28 + 1)),
                    true,
                )
            })
            .collect();
        let newest = vs
            .iter()
            .filter_map(|x| x.issued_on.clone())
            .max()
            .expect("dates");
        vs.push(v(None, true)); // no issue date
        vs.push(v(Some("2026-06-01"), false)); // closed: must not appear at all

        let (details, total) = ViolationDetail::from_open(&vs, "2026-08-09");
        assert_eq!(
            total, 61,
            "total counts every OPEN violation, not the capped list"
        );
        assert_eq!(details.len(), OPEN_DETAIL_CAP);

        // Newest first, so the freshest violation is also the fewest days open.
        assert_eq!(details[0].issued_on.as_deref(), Some(newest.as_str()));
        assert!(details[0].days_open.unwrap() < details[1].days_open.unwrap());

        // The undated one sorts last, so it cannot displace a recent violation.
        let (all, _) = ViolationDetail::from_open(&vs[58..], "2026-08-09");
        assert_eq!(all.last().unwrap().issued_on, None);
        assert_eq!(all.last().unwrap().days_open, None);

        // Closed violations are absent, since the card lists what is still wrong.
        assert!(details
            .iter()
            .all(|d| d.issued_on.as_deref() != Some("2026-06-01")));
    }

    #[test]
    fn days_open_is_none_rather_than_zero_when_the_date_is_missing() {
        let no_date = Violation {
            class: ViolationClass::C,
            open: true,
            ..Default::default()
        };
        assert_eq!(no_date.days_open("2026-08-09"), None);

        // Still open: measured against today.
        let open = Violation {
            class: ViolationClass::C,
            open: true,
            issued_on: Some("2026-03-14".into()),
            ..Default::default()
        };
        assert_eq!(open.days_open("2026-08-09"), Some(148));

        // Closed: measured to the close date, not to today.
        let closed = Violation {
            class: ViolationClass::C,
            open: false,
            issued_on: Some("2014-01-06".into()),
            closed_on: Some("2014-01-25".into()),
            ..Default::default()
        };
        assert_eq!(closed.days_open("2026-08-09"), Some(19));

        // A close date on an OPEN violation is a status change, not a closure, so the
        // age still runs to today.
        let reopened = Violation {
            class: ViolationClass::B,
            open: true,
            issued_on: Some("2026-08-01".into()),
            closed_on: Some("2026-08-03".into()),
            ..Default::default()
        };
        assert_eq!(reopened.days_open("2026-08-09"), Some(8));

        // Leap day is real: 2024 had one, 2100 will not.
        let leap = Violation {
            class: ViolationClass::A,
            open: true,
            issued_on: Some("2024-02-28".into()),
            ..Default::default()
        };
        assert_eq!(leap.days_open("2024-03-01"), Some(2));

        // Dirty data must not produce a negative age.
        let backwards = Violation {
            class: ViolationClass::A,
            open: false,
            issued_on: Some("2020-06-01".into()),
            closed_on: Some("2019-01-01".into()),
            ..Default::default()
        };
        assert_eq!(backwards.days_open("2026-08-09"), Some(0));

        // Unparseable dates are missing dates, not a panic.
        let junk = Violation {
            class: ViolationClass::A,
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
                class: ViolationClass::C,
                open: true,
                year: 2025,
                ..Default::default()
            },
            Violation {
                class: ViolationClass::C,
                open: false,
                year: 2020,
                ..Default::default()
            },
            Violation {
                class: ViolationClass::A,
                open: true,
                year: 2024,
                ..Default::default()
            },
        ];
        let counts = ViolationCounts::open_from(&vs);
        assert_eq!(counts, ViolationCounts { a: 1, b: 0, c: 1 });
    }

    /// Helper: unwrap a `Median` or fail loudly, so tests read as assertions not matches.
    fn median_of(durations: Vec<i64>) -> (i64, u32) {
        match RepairSpeed::classify(durations, 0, 2023) {
            Some(RepairSpeed::Median {
                median_days,
                sample,
                ..
            }) => (median_days, sample),
            other => panic!("expected a median, got {other:?}"),
        }
    }

    /// The median must not be dragged by the tail — that is the whole reason it is a median.
    ///
    /// Real shape, from pilot building 3016910012: closures of 6, 580 and 1,075 days. A mean
    /// over this row set is 340 days and describes none of them; the median is 30.
    #[test]
    fn one_ancient_closure_does_not_move_the_median() {
        assert_eq!(
            median_of(vec![6, 12, 30, 580, 1075]),
            (30, 5),
            "a mean would report 340"
        );
    }

    /// An even-length sample averages the middle pair, so "half took longer" stays exactly true.
    #[test]
    fn an_even_sample_averages_the_middle_pair() {
        assert_eq!(median_of(vec![10, 20, 30, 40, 50, 60]).0, 35);
    }

    /// A close date before the issue date is a contradiction in the source, not a fast repair.
    ///
    /// It matters which way this fails: a negative duration pulls the median **down**, so a
    /// building that never fixes anything could be made to look responsive by one malformed
    /// row. Dropping them can only ever make a building look worse, which is the safe
    /// direction on a page a renter uses to decide.
    #[test]
    fn a_closure_dated_before_its_issue_is_discarded_not_counted() {
        assert_eq!(median_of(vec![-900, 10, 20, 30, 40, 50]), (30, 5));
    }

    /// **The case that forced the third state.**
    ///
    /// 603 Putnam Avenue: 33 open violations, one closure in the entire record, dated October
    /// 2017. With two states this rendered blank, so the building that fixes nothing looked
    /// emptier than one that fixes things slowly. The absence is the finding.
    #[test]
    fn a_building_that_closes_nothing_says_so_rather_than_going_blank() {
        match RepairSpeed::classify(vec![], 33, 2023) {
            Some(RepairSpeed::NothingClosed { open, since_year }) => {
                assert_eq!(open, 33);
                assert_eq!(since_year, 2023);
            }
            other => panic!("33 open and nothing closed must be reported, got {other:?}"),
        }
    }

    /// But silence only means something when there is something to be silent about.
    #[test]
    fn a_building_with_almost_no_violations_makes_no_claim() {
        assert!(
            RepairSpeed::classify(vec![], 0, 2023).is_none(),
            "no violations is not slow"
        );
        assert!(
            RepairSpeed::classify(vec![], 4, 2023).is_none(),
            "4 open is too few to judge"
        );
        assert!(
            RepairSpeed::classify(vec![10, 20], 100, 2023).is_none(),
            "some closures but too few to median, and not silent either -- no claim"
        );
    }

    /// Below the sample floor there is no median, not a small one.
    #[test]
    fn too_few_closures_produce_no_median() {
        assert!(RepairSpeed::classify(vec![1, 2, 3, 4], 0, 2023).is_none());
        assert!(matches!(
            RepairSpeed::classify(vec![1, 2, 3, 4, 5], 0, 2023),
            Some(RepairSpeed::Median { .. })
        ));
    }

    /// The wire shape must distinguish the three states without a client guessing.
    #[test]
    fn each_state_is_distinguishable_on_the_wire() {
        let m = serde_json::to_string(&RepairSpeed::classify(vec![7; 9], 0, 2023)).unwrap();
        assert!(m.contains(r#""kind":"median""#), "got {m}");
        assert!(m.contains(r#""sample":9"#));

        let n = serde_json::to_string(&RepairSpeed::classify(vec![], 33, 2023)).unwrap();
        assert!(n.contains(r#""kind":"nothing_closed""#), "got {n}");
        assert!(
            !n.contains("median_days"),
            "must not imply a duration it does not have"
        );

        // And absent is absent -- HealthCard's skip_serializing_if keeps it off the wire, so a
        // client can never read a missing history as a fast one.
        assert_eq!(
            serde_json::to_string(&RepairSpeed::classify(vec![], 0, 2023)).unwrap(),
            "null"
        );
    }
}
