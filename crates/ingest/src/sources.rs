use model::{Building, Violation};
use serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pluto_url_filters_by_cd_and_residential() {
        let url = pluto_url(303, 200);
        assert!(url.contains("64uk-42ks.json"));
        assert!(url.contains("cd=303"));
        assert!(url.contains("unitsres%3E0") || url.contains("unitsres>0"));
        assert!(url.contains("$limit=200"));
    }

    #[test]
    fn parses_pluto_record_into_building() {
        let v = json!({
            "bbl": "3018420001",
            "address": "123 MACON STREET",
            "yearbuilt": "1910",
            "numfloors": "3",
            "unitsres": "6",
            "bldgclass": "C0",
            "latitude": "40.6829",
            "longitude": "-73.9251",
            "ct2020": "025300"
        });
        let b = parse_pluto(&v).expect("valid record");
        assert_eq!(b.bbl, "3018420001");
        assert_eq!(b.year_built, 1910);
        assert_eq!(b.num_floors, 3);
        assert_eq!(b.units_res, 6);
        // tract_geoid = state(36)+county(047)+ct2020(6-digit)
        assert_eq!(b.tract_geoid, "36047025300");
        // defaults for fields PLUTO doesn't carry:
        assert_eq!(b.has_elevator, false);
        assert_eq!(b.rent_stabilized, None);
    }

    #[test]
    fn parses_hpd_violation_open_and_class() {
        let v = json!({"bbl": "3018420001", "class": "C", "currentstatus": "Open", "novissueddate": "2025-06-01T00:00:00.000"});
        let viol = parse_hpd_violation(&v).expect("valid");
        assert_eq!(viol.class, "C");
        assert!(viol.open);
        assert_eq!(viol.year, 2025);
    }

    #[test]
    fn hpd_violation_closed_status_is_not_open() {
        let v = json!({"bbl": "3018420001", "class": "A", "currentstatus": "Close", "novissueddate": "2018-01-01T00:00:00.000"});
        assert!(!parse_hpd_violation(&v).unwrap().open);
    }

    #[test]
    fn dob_record_flags_active_passenger_elevator() {
        assert!(parse_dob_has_elevator(&json!({"device_type": "Elevator", "device_status": "ACTIVE"})));
        assert!(!parse_dob_has_elevator(&json!({"device_type": "Escalator", "device_status": "ACTIVE"})));
        assert!(!parse_dob_has_elevator(&json!({"device_type": "Elevator", "device_status": "DISMANTLED"})));
    }

    #[test]
    fn parses_census_median_and_rejects_sentinels() {
        // Census returns arrays: [ [ "B25064_001E", "state","county","tract" ], [ "1850","36","047","025300" ], ... ]
        let v = json!([["B25064_001E","state","county","tract"],
                       ["1850","36","047","025300"],
                       ["-666666666","36","047","025400"]]);
        let map = parse_census_medians(&v);
        assert_eq!(map.get("36047025300"), Some(&1850));
        assert_eq!(map.get("36047025400"), None); // sentinel dropped
    }
}

const SODA: &str = "https://data.cityofnewyork.us/resource";

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|x| x.to_string())
}
fn num(v: &Value, k: &str) -> i32 {
    s(v, k).and_then(|x| x.trim().parse::<f64>().ok()).map(|f| f as i32).unwrap_or(0)
}

pub fn pluto_url(cd: u32, limit: u32) -> String {
    // $where is URL-encoded by reqwest when we pass it as a query param, but we build the
    // full string here for transparency/tests; encode the comparison operator.
    format!(
        "{SODA}/64uk-42ks.json?$select=bbl,address,yearbuilt,numfloors,unitsres,bldgclass,latitude,longitude,ct2020&$where=borough='BK' AND cd={cd} AND unitsres>0&$limit={limit}"
    )
}

pub fn parse_pluto(v: &Value) -> Option<Building> {
    let bbl = s(v, "bbl")?;
    let ct = s(v, "ct2020").unwrap_or_default();
    // ct2020 is a 6-digit tract code; GEOID = 36 (NY) + 047 (Kings) + ct.
    let tract_geoid = if ct.len() == 6 { format!("36047{ct}") } else { String::new() };
    Some(Building {
        bbl,
        address: s(v, "address").unwrap_or_default(),
        year_built: num(v, "yearbuilt"),
        num_floors: num(v, "numfloors"),
        units_res: num(v, "unitsres"),
        tract_geoid,
        rent_stabilized: None, // filled later from DHCR/WOW if available; None = unknown
        good_cause: false,     // filled later
        has_elevator: false,   // filled from DOB
        near_ada_subway_m: None, // filled from MTA
        complaints_311: 0,     // filled from 311
    })
}

pub fn bbl_in_url(id: &str, select: &str, bbls: &[String]) -> String {
    let list = bbls.iter().map(|b| format!("'{b}'")).collect::<Vec<_>>().join(",");
    format!("{SODA}/{id}.json?$select={select}&$where=bbl in({list})&$limit=50000")
}

pub fn parse_hpd_violation(v: &Value) -> Option<Violation> {
    let class = s(v, "class")?.to_uppercase();
    let status = s(v, "currentstatus").unwrap_or_default().to_lowercase();
    let open = !status.starts_with("close");
    let year = s(v, "novissueddate")
        .and_then(|d| d.get(0..4).map(|y| y.to_string()))
        .and_then(|y| y.parse::<i32>().ok())
        .unwrap_or(0);
    Some(Violation { class, open, year })
}

pub fn parse_dob_has_elevator(v: &Value) -> bool {
    let is_elevator = s(v, "device_type").unwrap_or_default().eq_ignore_ascii_case("Elevator");
    let active = s(v, "device_status").unwrap_or_default().eq_ignore_ascii_case("ACTIVE");
    is_elevator && active
}

pub fn census_url(key: &str) -> String {
    format!("https://api.census.gov/data/2023/acs/acs5?get=B25064_001E&for=tract:*&in=state:36&in=county:047&key={key}")
}

pub fn parse_census_medians(v: &Value) -> std::collections::HashMap<String, i32> {
    let mut out = std::collections::HashMap::new();
    let rows = match v.as_array() { Some(r) => r, None => return out };
    for row in rows.iter().skip(1) {
        // row = [ median, state, county, tract ]
        let cols: Vec<&str> = row.as_array().map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        if cols.len() < 4 { continue; }
        let median: i32 = cols[0].parse().unwrap_or(-1);
        if median <= 0 { continue; } // drop suppressed/sentinel (e.g. -666666666)
        let geoid = format!("{}{}{}", cols[1], cols[2], cols[3]);
        out.insert(geoid, median);
    }
    out
}
