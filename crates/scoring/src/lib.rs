use model::Violation;

/// 0–100 building-condition score. Deterministic: `current_year` is passed in,
/// never read from the clock, so scores are testable and reproducible.
/// Open violations only. Severity: C=15, B=7, A=3. Recency (<=2 yrs) doubles it.
pub fn condition_score(violations: &[Violation], current_year: i32) -> u8 {
    let mut penalty: i32 = 0;
    for v in violations.iter().filter(|v| v.open) {
        let base = match v.class.as_str() {
            "C" => 15,
            "B" => 7,
            "A" => 3,
            _ => 0,
        };
        let recency = if current_year - v.year <= 2 { 2 } else { 1 };
        penalty += base * recency;
    }
    (100 - penalty).clamp(0, 100) as u8
}

use model::Building;

/// 0–100 legal-protection score. Base 60; stabilized +25; Good Cause +15.
pub fn legal_score(b: &Building) -> u8 {
    let mut s: i32 = 60;
    if b.rent_stabilized == Some(true) {
        s += 25;
    }
    if b.good_cause {
        s += 15;
    }
    s.clamp(0, 100) as u8
}

/// 0–100 neighborhood score from 311 complaint count. Each complaint -2, capped -60.
pub fn neighborhood_score(complaints_311: i32) -> u8 {
    (100 - (complaints_311 * 2).min(60)).clamp(0, 100) as u8
}

/// Accessibility likelihood as (score, label). Elevator-on-record is the strongest
/// signal; otherwise infer from floors and FHA build-era. NOT a certification.
pub fn access_likelihood(b: &Building) -> (u8, String) {
    if b.has_elevator {
        return (90, "Higher".to_string());
    }
    if b.num_floors <= 2 {
        return (75, "Higher".to_string());
    }
    let fha_era = b.year_built >= 1992 && b.units_res >= 4;
    if fha_era {
        (55, "Mixed".to_string())
    } else {
        (30, "Lower".to_string())
    }
}

/// Weighted 0–100 total. Weights: condition .45, legal .20, neighborhood .15, accessibility .20.
pub fn total_score(condition: u8, legal: u8, neighborhood: u8, accessibility: u8) -> u8 {
    let t = condition as f64 * 0.45
        + legal as f64 * 0.20
        + neighborhood as f64 * 0.15
        + accessibility as f64 * 0.20;
    t.round().clamp(0.0, 100.0) as u8
}

/// Rent vs tract median: (pct difference, human verdict). >5% above, <-5% below, else "about at".
pub fn rent_fairness(user_rent: i32, tract_median: i32) -> (f64, String) {
    // Defense-in-depth: Census B25064 ships suppressed tracts as 0 or a sentinel
    // negative (e.g. -666666666). A non-positive median is meaningless — never divide
    // by it, or the flagship feature would print a confident, fabricated number.
    if tract_median <= 0 {
        return (0.0, "no reliable neighborhood median available".to_string());
    }
    let pct = (user_rent - tract_median) as f64 / tract_median as f64 * 100.0;
    let verdict = if pct > 5.0 {
        format!("{:.0}% above neighborhood median", pct)
    } else if pct < -5.0 {
        format!("{:.0}% below neighborhood median", pct.abs())
    } else {
        "about at the neighborhood median".to_string()
    };
    (pct, verdict)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_recent_open_class_c_costs_30_points() {
        let vs = vec![Violation {
            class: "C".into(),
            open: true,
            year: 2026,
        }];
        // recent (within 2 yrs of 2026) C = 15 * 2 = 30 penalty -> 70
        assert_eq!(condition_score(&vs, 2026), 70);
    }

    #[test]
    fn closed_violations_are_ignored() {
        let vs = vec![Violation {
            class: "C".into(),
            open: false,
            year: 2026,
        }];
        assert_eq!(condition_score(&vs, 2026), 100);
    }

    #[test]
    fn penalty_clamps_at_zero() {
        let vs: Vec<Violation> = (0..20)
            .map(|_| Violation {
                class: "C".into(),
                open: true,
                year: 2026,
            })
            .collect();
        assert_eq!(condition_score(&vs, 2026), 0);
    }

    use model::Building;

    fn base_building() -> Building {
        Building {
            bbl: "3000010001".into(),
            address: "1 Test St, Brooklyn".into(),
            year_built: 1960,
            num_floors: 5,
            units_res: 20,
            tract_geoid: "36047000100".into(),
            rent_stabilized: None,
            good_cause: false,
            has_elevator: false,
            near_ada_subway_m: None,
            complaints_311: 0,
        }
    }

    #[test]
    fn legal_rewards_stabilized_and_good_cause() {
        let mut b = base_building();
        assert_eq!(legal_score(&b), 60);
        b.rent_stabilized = Some(true);
        b.good_cause = true;
        assert_eq!(legal_score(&b), 100);
    }

    #[test]
    fn neighborhood_penalizes_311_density() {
        assert_eq!(neighborhood_score(0), 100);
        assert_eq!(neighborhood_score(10), 80);
        assert_eq!(neighborhood_score(100), 40); // capped penalty at 60
    }

    #[test]
    fn accessibility_elevator_is_higher() {
        let mut b = base_building();
        b.has_elevator = true;
        assert_eq!(access_likelihood(&b), (90, "Higher".to_string()));
    }

    #[test]
    fn accessibility_no_elevator_fha_era_is_mixed() {
        let mut b = base_building();
        b.has_elevator = false;
        b.num_floors = 5;
        b.year_built = 2000;
        b.units_res = 12;
        assert_eq!(access_likelihood(&b), (55, "Mixed".to_string()));
    }

    #[test]
    fn accessibility_walkup_lowrise_is_higher() {
        let mut b = base_building();
        b.has_elevator = false;
        b.num_floors = 2;
        assert_eq!(access_likelihood(&b), (75, "Higher".to_string()));
    }

    #[test]
    fn total_is_weighted_sum_rounded() {
        // 80,60,100,90 -> 80*.45+60*.20+100*.15+90*.20 = 36+12+15+18 = 81
        assert_eq!(total_score(80, 60, 100, 90), 81);
    }

    #[test]
    fn rent_fairness_flags_above_market() {
        let (pct, verdict) = rent_fairness(3000, 2500);
        assert_eq!(pct.round() as i32, 20);
        assert_eq!(verdict, "20% above neighborhood median");
    }

    #[test]
    fn rent_fairness_guards_nonpositive_median() {
        // A suppressed/sentinel Census median must not divide-by-zero or print garbage.
        for bad in [0, -666666666] {
            let (pct, verdict) = rent_fairness(3000, bad);
            assert!(pct.is_finite());
            assert_eq!(pct, 0.0);
            assert!(verdict.contains("no reliable"));
        }
    }
}
