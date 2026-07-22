// This module is a pure library of geo helpers consumed by the live orchestration in
// `run::run_real` (Plan 2 Task 5, out of scope here — see crates/ingest/src/run.rs, which
// is currently a stub). Everything here is exercised by the unit tests below but stays
// unreachable from `main` until Task 5 lands, so silence dead_code at the module level
// rather than deleting tested, spec'd logic.
#![allow(dead_code)]

pub struct Station {
    pub lat: f64,
    pub lon: f64,
    pub ada: i32, // 0 none, 1 full, 2 partial
}

/// Great-circle distance in metres.
pub fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6_371_000.0_f64;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * r * a.sqrt().asin()
}

/// Metres to the nearest fully/partially ADA-accessible station (ada != 0), or None.
pub fn nearest_ada_m(lat: f64, lon: f64, stations: &[Station]) -> Option<f64> {
    stations
        .iter()
        .filter(|s| s.ada != 0)
        .map(|s| haversine_m(lat, lon, s.lat, s.lon))
        .fold(None, |acc, d| match acc {
            Some(m) if m <= d => Some(m),
            _ => Some(d),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haversine_known_distance() {
        // ~1.11 km between 0.01 deg latitude apart at NYC.
        let d = haversine_m(40.68, -73.95, 40.69, -73.95);
        assert!((d - 1112.0).abs() < 20.0, "got {d}");
    }

    #[test]
    fn nearest_ada_returns_closest_ada_only() {
        let stations = vec![
            Station {
                lat: 40.70,
                lon: -73.95,
                ada: 0,
            }, // closest but not ADA
            Station {
                lat: 40.685,
                lon: -73.951,
                ada: 1,
            }, // ADA, a bit further
        ];
        let d = nearest_ada_m(40.68, -73.95, &stations).unwrap();
        // distance to the ADA station, not the non-ADA one
        assert!(d > 500.0 && d < 700.0, "got {d}");
    }

    #[test]
    fn nearest_ada_none_when_no_ada_stations() {
        let stations = vec![Station {
            lat: 40.70,
            lon: -73.95,
            ada: 0,
        }];
        assert!(nearest_ada_m(40.68, -73.95, &stations).is_none());
    }
}
