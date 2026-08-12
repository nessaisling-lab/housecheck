//! Assembles a Building Health Card from the artifact.
//!
//! Extracted from the HTTP API so that the MCP server answers with the *same* card
//! rather than a second implementation of it. Two card builders would drift, and the
//! failure would be the quiet kind: an agent and the website reporting different scores
//! for the same building, each internally consistent.
//!
//! `today` is a parameter rather than a call to the clock, so the caller owns the time
//! source and the days-open arithmetic is testable without waiting a day.

use anyhow::Result;
use model::{HealthCard, ScoreBreakdown, Stabilization, ViolationCounts};
use rusqlite::Connection;
use store::{get_building, get_open_violations};

pub fn card_for(
    conn: &Connection,
    snapshot_year: i32,
    bbl: &str,
    today: &str,
) -> Result<Option<HealthCard>> {
    let building = match get_building(conn, bbl)? {
        Some(b) => b,
        None => return Ok(None),
    };
    let violations = get_open_violations(conn, bbl)?;

    let condition = scoring::condition_score(&violations, snapshot_year);
    let legal = scoring::legal_score(&building);
    let neighborhood = scoring::neighborhood_score(building.complaints_311);
    let (accessibility, access_likelihood) = scoring::access_likelihood(&building);
    let total = scoring::total_score(condition, legal, neighborhood, accessibility);

    let (open_violation_details, open_violation_total) =
        model::ViolationDetail::from_open(&violations, today);

    // Behaviour, not state. Absent rather than zero when the record cannot support the
    // claim -- 84 of the 250 pilot buildings have no closed violations at all, and a bold
    // "0 days" on those would read as the fastest landlord in Brooklyn.
    let repair_speed = model::RepairSpeed::classify(
        store::closed_violation_durations(conn, bbl, model::REPAIR_SPEED_SINCE_YEAR)?,
        open_violation_total,
        model::REPAIR_SPEED_SINCE_YEAR,
    );

    Ok(Some(HealthCard {
        repair_speed,
        open_violations: ViolationCounts::open_from(&violations),
        open_violation_details,
        open_violation_total,
        score: ScoreBreakdown {
            total,
            condition,
            legal,
            neighborhood,
            accessibility,
        },
        access_likelihood,
        // Honest three-state signal derived from the stored rent-stabilization data
        // (JustFix nyc-doffer, from NYC DOF Statement-of-Account records, latest year
        // 2024). Carries the unit count for the "likely" wording; the message never
        // overstates a match.
        stabilization: Stabilization::from_units(building.rent_stabilized, building.rent_stab_units),
        building,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Connection {
        let conn = store::open_db(":memory:").expect("open");
        store::migrate(&conn).expect("migrate");
        store::insert_fixture(&conn).expect("fixture");
        conn
    }

    /// A BBL we do not cover must be `None`, not an empty card. An empty card renders as
    /// a building with nothing wrong with it, which is the opposite of "we have no data".
    #[test]
    fn an_uncovered_bbl_is_none_rather_than_a_blank_card() {
        let conn = fixture();
        assert!(card_for(&conn, 2026, "9999999999", "2026-08-12")
            .expect("query")
            .is_none());
    }

    /// The card exists to be assembled once. This is the smoke test that the extraction
    /// from the HTTP API kept it working at all.
    #[test]
    fn a_covered_bbl_produces_a_scored_card() {
        let conn = fixture();
        let first = store::get_all_buildings(&conn).expect("buildings");
        let bbl = &first.first().expect("at least one fixture building").bbl;
        let card = card_for(&conn, 2026, bbl, "2026-08-12")
            .expect("query")
            .expect("covered");
        assert_eq!(&card.building.bbl, bbl);
        assert!(card.score.total <= 100);
    }

    /// `today` is a parameter precisely so days-open does not depend on the wall clock.
    /// If this ever starts failing, someone has reached for `SystemTime::now` in here.
    #[test]
    fn the_same_inputs_and_the_same_day_give_the_same_card() {
        let conn = fixture();
        let bbl = store::get_all_buildings(&conn).expect("buildings")[0].bbl.clone();
        let a = card_for(&conn, 2026, &bbl, "2026-08-12").unwrap().unwrap();
        let b = card_for(&conn, 2026, &bbl, "2026-08-12").unwrap().unwrap();
        assert_eq!(a.score.total, b.score.total);
        assert_eq!(a.open_violation_total, b.open_violation_total);
    }
}
