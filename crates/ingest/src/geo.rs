// Pure geo helpers consumed by the live orchestration in `run::run_real` (Plan 2 Task 5):
// great-circle distance and nearest ADA-accessible subway station. Exercised both by the
// unit tests below and by the real ingest path.

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

/// How many of `points` lie within `radius_m` metres of (lat, lon). A cheap lat/long
/// bounding-box pre-filter skips the exact haversine for far-away points, keeping the
/// O(buildings × points) sweep over a large 311 pull fast. The box is widened past the raw
/// radius (longitude by 1/cos(lat)) so it never clips a true in-radius point — the haversine
/// is the exact gate.
pub fn count_within_m(lat: f64, lon: f64, points: &[(f64, f64)], radius_m: f64) -> usize {
    let lat_deg = radius_m / 111_320.0 + 1e-4;
    let lon_deg = radius_m / (111_320.0 * lat.to_radians().cos().abs().max(1e-6)) + 1e-4;
    points
        .iter()
        .filter(|(plat, plon)| (plat - lat).abs() <= lat_deg && (plon - lon).abs() <= lon_deg)
        .filter(|(plat, plon)| haversine_m(lat, lon, *plat, *plon) <= radius_m)
        .count()
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

    #[test]
    fn count_within_m_counts_only_points_inside_radius() {
        let (lat, lon) = (40.6829, -73.9251);
        let points = vec![
            (40.6829, -73.9251), // 0 m — the building itself
            (40.6835, -73.9251), // ~67 m north (0.0006 deg lat)
            (40.6829, -73.9245), // ~51 m east
            (40.70, -73.95),     // ~1.9 km away — outside
        ];
        assert_eq!(count_within_m(lat, lon, &points, 150.0), 3);
        assert_eq!(count_within_m(lat, lon, &points, 60.0), 2); // drops the 67 m point
        assert_eq!(count_within_m(lat, lon, &[], 150.0), 0);
    }
}
