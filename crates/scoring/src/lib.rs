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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_recent_open_class_c_costs_30_points() {
        let vs = vec![Violation { class: "C".into(), open: true, year: 2026 }];
        // recent (within 2 yrs of 2026) C = 15 * 2 = 30 penalty -> 70
        assert_eq!(condition_score(&vs, 2026), 70);
    }

    #[test]
    fn closed_violations_are_ignored() {
        let vs = vec![Violation { class: "C".into(), open: false, year: 2026 }];
        assert_eq!(condition_score(&vs, 2026), 100);
    }

    #[test]
    fn penalty_clamps_at_zero() {
        let vs: Vec<Violation> = (0..20)
            .map(|_| Violation { class: "C".into(), open: true, year: 2026 })
            .collect();
        assert_eq!(condition_score(&vs, 2026), 0);
    }
}
