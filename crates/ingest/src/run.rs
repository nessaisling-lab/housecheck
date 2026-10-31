use anyhow::{Context, Result};
use model::{Building, Violation};
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::geo::{nearest_ada_m, Station};
use crate::sources::{
    bbl_block, census_url, complaints_311_by_zip_query, county_fips, dobnow_date,
    excluded_classes_note, fetch_foil_cos, fetch_rent_stab, homes_by_zip_query, hpd_excluded_class,
    hpd_row_bbl, hpd_where_blocks, is_new_building_job, later_exemption, note_co, nta_query,
    parse_census_medians, parse_dob_has_elevator, parse_hpd_violation, parse_nta, parse_pluto,
    parse_zip_counts, pluto_coords, pluto_query, soda_date, zip_rates, LatestCo, DOF_SUBSIDY_CODES,
    HPD_SELECT, PLUTO_SELECT, SODA,
};

/// Blocking HTTP client with a UA (Socrata throttles anonymous no-UA traffic harder) and a
/// generous timeout for the larger pulls. If `NYC_APP_TOKEN` is set, it is sent as the Socrata
/// `X-App-Token` header on every request, lifting the anonymous rate limit (harmless on the
/// Census/MTA hosts, which ignore an unknown token).
///
/// The `gzip` feature makes every request ask for a compressed body and decompress it on
/// arrival. Measured on a 50,000-row violation page: 17.7 MB plain, 1.29 MB gzipped.
fn client() -> Client {
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(token) = std::env::var("NYC_APP_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(token) {
                headers.insert(reqwest::header::HeaderName::from_static("x-app-token"), val);
                println!("Socrata: using NYC_APP_TOKEN (raised rate limit)");
            }
        }
    }
    Client::builder()
        .user_agent("housecheck-ingest/0.1")
        .default_headers(headers)
        .timeout(Duration::from_secs(300))
        .build()
        .expect("build reqwest client")
}

/// Borrow the array inside a JSON value, or an empty slice (never panics on a scalar/null).
fn arr(v: &Value) -> &[Value] {
    v.as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

/// A URL with its query string removed, for error messages and logs.
///
/// `census_url` embeds `CENSUS_API_KEY` as a query parameter, and the Census fetch below is
/// non-fatal — it prints `{e:#}`, the whole anyhow context chain. Interpolating the raw URL
/// into that context would print the caller's API key to stdout on any Census outage, and from
/// there into CI logs and terminal scrollback. The host and path are all a reader needs.
/// `reqwest`'s own errors carry the full URL too, which is why every error below goes through
/// `without_url()` first.
fn redacted(url: &str) -> &str {
    url.split('?').next().unwrap_or(url)
}

/// Up to three tries, 2 s then 4 s apart. A citywide pull is a few hundred requests over most
/// of an hour, and one transient 5xx should not throw the run away.
///
/// ponytail: retries every error, a malformed query's 400 included; that one just fails 6 s
/// later than it would have.
fn with_retry<T>(mut f: impl FnMut() -> Result<T>) -> Result<T> {
    let mut attempt = 1;
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) if attempt < 3 => {
                println!("  retry {attempt}/2 after: {e:#}");
                std::thread::sleep(Duration::from_secs(2 << (attempt - 1)));
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

/// GET a whole URL (Census / MTA — no raw spaces to encode) and decode JSON.
fn get_json(c: &Client, url: &str) -> Result<Value> {
    let safe = redacted(url);
    with_retry(|| {
        c.get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json())
            .map_err(|e| e.without_url())
            .with_context(|| format!("GET {safe}"))
    })
}

/// GET a base URL with query params (Socrata SoQL). `reqwest` percent-encodes the `$where`
/// clause's spaces and `>` — the whole point of sending components, not a baked URL string.
///
/// Raw fetch, no completeness check: use this only where `$limit` is a deliberate sample cap
/// (the pilot's PLUTO pull, which is *meant* to stop at `--limit`). Anywhere the query is
/// supposed to return everything that matches, use `get_json_complete` or `for_each_page`.
fn get_json_query(c: &Client, base: &str, params: &[(String, String)]) -> Result<Value> {
    with_retry(|| {
        c.get(base)
            .query(params)
            .send()
            .and_then(|r| r.error_for_status())
            .and_then(|r| r.json())
            .map_err(|e| e.without_url())
            .with_context(|| format!("GET {base}"))
    })
}

/// Short name for log lines: the Socrata dataset id out of the URL.
fn dataset_of(base: &str) -> &str {
    base.rsplit('/').next().unwrap_or(base)
}

/// GET a query that is expected to return every matching row in one request, and **fail loudly
/// if it did not**.
///
/// A bare `$limit` is a silent truncation. Socrata returns the first N rows in an unspecified
/// order and tells you nothing about how many matched, so a query that quietly drops rows looks
/// exactly like one that found few. Row count equal to the limit is the signal, and it is the
/// only signal there is.
fn get_json_complete(
    c: &Client,
    (base, params): &(String, Vec<(String, String)>),
) -> Result<Value> {
    let rows = get_json_query(c, base, params)?;
    if let Some(limit) = params
        .iter()
        .find(|(k, _)| k == "$limit")
        .and_then(|(_, v)| v.parse::<usize>().ok())
    {
        let n = arr(&rows).len();
        anyhow::ensure!(
            n < limit,
            "{}: returned {n} rows against a $limit of {limit} — the result is truncated and \
             the ingest would silently store incomplete data. Page this query or raise the limit.",
            dataset_of(base)
        );
    }
    Ok(rows)
}

/// Rows per Socrata page.
const PAGE: usize = 50_000;

/// How many rows Socrata says match `where_`.
fn count_rows(c: &Client, dataset: &str, where_: &str) -> Result<usize> {
    let base = format!("{SODA}/{dataset}.json");
    let mut params = vec![("$select".to_string(), "count(1) AS n".to_string())];
    if !where_.is_empty() {
        params.push(("$where".to_string(), where_.to_string()));
    }
    let v = get_json_query(c, &base, &params)?;
    arr(&v)
        .first()
        .and_then(|r| r.get("n"))
        .and_then(|n| n.as_str())
        .and_then(|n| n.parse().ok())
        .with_context(|| format!("{dataset}: no count in {v}"))
}

/// GET **every** row matching `where_` (every row at all when it is empty), handing each page
/// to `f` and keeping none of them.
///
/// Memory stays at one page whatever the dataset's size: the old paged fetch collected every
/// row as JSON first, which is about 10 GB for the city's 10.5M violations.
///
/// Pages are keyed on Socrata's `:id` (`:id > last`, ordered by `:id`), not on `$offset`. An
/// offset makes the server count past every earlier row on every request, which slows with
/// depth, and the violation pull is ~211 pages deep. The order is still the total order that
/// made offset paging well-defined (the half-dropped violation history it fixed is in the git
/// log under `get_json_paged`).
///
/// **A short page is not proof the pull is complete**, so every caller gets the same check:
/// Socrata's own count before and after, which the rows fetched must land between
/// (`check_complete`). And a full page whose last `:id` is the one already seen means Socrata
/// ignored the cursor, so that stops the run rather than looping on the same page.
fn for_each_page(
    c: &Client,
    dataset: &str,
    select: &str,
    where_: &str,
    mut f: impl FnMut(&[Value]) -> Result<()>,
) -> Result<usize> {
    let base = format!("{SODA}/{dataset}.json");
    let before = count_rows(c, dataset, where_)?;
    let mut last: Option<String> = None;
    let (mut n, mut pages) = (0usize, 0usize);
    loop {
        let w = match (&last, where_.is_empty()) {
            (Some(id), true) => format!(":id > '{id}'"),
            (Some(id), false) => format!("({where_}) AND :id > '{id}'"),
            (None, _) => where_.to_string(),
        };
        let mut params = vec![
            ("$select".to_string(), format!(":id,{select}")),
            ("$order".to_string(), ":id".to_string()),
            ("$limit".to_string(), PAGE.to_string()),
        ];
        if !w.is_empty() {
            params.push(("$where".to_string(), w));
        }
        let rows = get_json_query(c, &base, &params)?;
        let page = arr(&rows);
        f(page)?;
        n += page.len();
        pages += 1;
        if pages % 20 == 0 {
            println!("  {dataset}: {n} rows so far");
        }
        if page.len() < PAGE {
            break;
        }
        let next = page
            .last()
            .and_then(|v| v.get(":id"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        anyhow::ensure!(
            next.is_some(),
            "{dataset}: a full page with no :id to continue from"
        );
        anyhow::ensure!(
            next != last,
            "{dataset}: the :id cursor did not advance past {last:?}; Socrata returned the same \
             page again, so the filter is being ignored"
        );
        last = next;
    }
    println!("  {dataset}: {n} rows over {pages} requests");
    let after = count_rows(c, dataset, where_)?;
    check_complete(dataset, n, before, after)?;
    Ok(n)
}

/// Refuse a pull whose row count falls outside Socrata's own counts taken before and after it.
/// A dataset can change while a long pull runs (HPD changes ~157,000 rows a week), so the
/// fetched count must land between the two rather than equal either.
fn check_complete(dataset: &str, fetched: usize, before: usize, after: usize) -> Result<()> {
    anyhow::ensure!(
        (before.min(after)..=before.max(after)).contains(&fetched),
        "{dataset}: fetched {fetched} rows but Socrata counted {before} before the pull and \
         {after} after. Rows went missing; refusing to build a thinner city."
    );
    Ok(())
}

/// Refuse the neighborhood figures when either source looks broken, since each failure scores
/// buildings without saying so:
/// - more than 1% of buildings without a rate (an empty or malformed PLUTO count), which puts
///   them on the 70 fallback;
/// - more than 10% of ZIPs at zero complaints (an empty or malformed 311 count). That gives
///   every ZIP a real-looking 0 and every building a 100. Measured 2026-10-30: none of the
///   184 ZIPs with homes is at zero; the quietest, 11005, has 99 per 1,000 homes.
fn check_zip_rates(
    zips: usize,
    zero_rate_zips: usize,
    without_rate: usize,
    buildings: usize,
) -> Result<()> {
    anyhow::ensure!(
        zips > 0 && zero_rate_zips * 10 <= zips && without_rate * 100 <= buildings,
        "311 ZIP rates: {zips} ZIPs ({zero_rate_zips} at zero complaints), {without_rate} of \
         {buildings} buildings without a rate. The 311 or PLUTO count came back empty or \
         malformed; refusing to score neighborhoods from it."
    );
    Ok(())
}

/// Stream every HPD violation matching `where_` into the DuckDB stage; `for_each_page` refuses
/// the run if rows went missing. Returns the rows staged, and the rows skipped for an
/// unmodelled class on our own buildings, by letter.
///
/// Every class is fetched so the skip can be counted per building, as it always was: the
/// count is printed on the card ("excluded: class I (753 records ...)"), and counting a whole
/// tax block's class I rows instead put 12,079 there for the pilot.
fn stage_violations(
    c: &Client,
    stage: &duckdb::Connection,
    where_: &str,
    bbl_set: &HashSet<String>,
) -> Result<(usize, BTreeMap<String, usize>)> {
    stage.execute_batch(
        "DROP TABLE IF EXISTS hpd_violation;
         CREATE TABLE hpd_violation (
            fetch_seq BIGINT,     -- arrival order, which is HPD's :id order
            sid VARCHAR,          -- Socrata :id, the key a weekly refresh will upsert on
            bbl BIGINT,
            class VARCHAR,
            open BOOLEAN,
            year INTEGER,
            issued_on VARCHAR,
            closed_on VARCHAR,
            description VARCHAR
         );",
    )?;
    let mut app = stage.appender("hpd_violation")?;
    let mut seq: i64 = 0;
    let mut excluded: BTreeMap<String, usize> = BTreeMap::new();
    for_each_page(c, "wvxf-dwi5", HPD_SELECT, where_, |page| {
        for v in page {
            let Some(viol) = parse_hpd_violation(v) else {
                if let Some(letter) = hpd_excluded_class(v) {
                    if hpd_row_bbl(v).is_some_and(|b| bbl_set.contains(&b)) {
                        *excluded.entry(letter).or_default() += 1;
                    }
                }
                continue;
            };
            app.append_row(duckdb::params![
                seq,
                v.get(":id").and_then(Value::as_str),
                hpd_row_bbl(v).and_then(|b| b.parse::<i64>().ok()),
                viol.class.as_str(),
                viol.open,
                viol.year,
                viol.issued_on,
                viol.closed_on,
                viol.description,
            ])?;
            seq += 1;
        }
        Ok(())
    })?;
    app.flush()?;
    drop(app);
    Ok((seq as usize, excluded))
}

/// Write the staged violations of every building in `bbls` into the serving artifact, one
/// building at a time through `store`'s writer, so the description compression is the tested
/// one. Returns (rows written, staged rows on lots outside the building set).
fn compose_violations(
    stage: &duckdb::Connection,
    conn: &rusqlite::Connection,
    bbls: &[i64],
) -> Result<(usize, usize)> {
    stage.execute_batch("DROP TABLE IF EXISTS bset; CREATE TABLE bset (bbl BIGINT);")?;
    {
        let mut app = stage.appender("bset")?;
        for b in bbls {
            app.append_row([b])?;
        }
    }
    // Lots HPD cites that PLUTO lists with no homes (or not at all). Counted, not dropped
    // silently.
    let outside: i64 = stage.query_row(
        "SELECT count(*) FROM hpd_violation
          WHERE bbl IS NULL OR bbl NOT IN (SELECT bbl FROM bset)",
        [],
        |r| r.get(0),
    )?;
    let mut writer = store::ViolationWriter::new(conn)?;
    let mut stmt = stage.prepare(
        "SELECT bbl, class, open, year, issued_on, closed_on, description
           FROM hpd_violation
          WHERE bbl IN (SELECT bbl FROM bset)
          ORDER BY bbl, fetch_seq",
    )?;
    let mut rows = stmt.query([])?;
    let (mut cur, mut vs, mut written) = (None::<i64>, Vec::<Violation>::new(), 0usize);
    while let Some(r) = rows.next()? {
        let bbl: i64 = r.get(0)?;
        if cur != Some(bbl) {
            if let Some(b) = cur {
                writer.insert(conn, &b.to_string(), &vs)?;
                written += vs.len();
                vs.clear();
            }
            cur = Some(bbl);
        }
        vs.push(Violation {
            class: r.get::<_, String>(1)?.parse()?,
            open: r.get(2)?,
            year: r.get(3)?,
            issued_on: r.get(4)?,
            closed_on: r.get(5)?,
            description: r.get(6)?,
        });
    }
    if let Some(b) = cur {
        writer.insert(conn, &b.to_string(), &vs)?;
        written += vs.len();
    }
    Ok((written, outside as usize))
}

/// Up to 100,000 staged descriptions, sampled the same way every run, to train the
/// description dictionary on.
fn description_samples(stage: &duckdb::Connection) -> Result<Vec<String>> {
    let mut stmt = stage.prepare(
        "SELECT description FROM (
            SELECT description FROM hpd_violation WHERE description <> ''
         ) USING SAMPLE reservoir(100000 ROWS) REPEATABLE (42)",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        out.push(r.get(0)?);
    }
    Ok(out)
}

/// The pilot's buildings: a BLEND so both stories show, ~60% pulled largest-first (the
/// pre-war, multi-unit, stabilization-eligible buildings) and the rest from the neighborhood's
/// natural mix of small rowhouses.
fn pilot_buildings(c: &Client, cfg: &Config) -> Result<Vec<Value>> {
    let big_n = (cfg.limit * 3) / 5;
    let (b1, p1) = pluto_query(cfg.community_district, big_n, Some("unitsres DESC"));
    let (b2, p2) = pluto_query(cfg.community_district, cfg.limit, None);
    let mut rows: Vec<Value> = arr(&get_json_query(c, &b1, &p1)?).to_vec();
    rows.extend(arr(&get_json_query(c, &b2, &p2)?).iter().cloned());
    Ok(rows)
}

/// What `fetch_good_cause` found: housing program by BBL, certificates of occupancy by BBL,
/// and (dataset, rows read, note) for the provenance table.
type GoodCauseFound = (
    HashMap<String, &'static str>,
    HashMap<String, LatestCo>,
    Vec<(&'static str, usize, String)>,
);

/// Good Cause exemptions beyond the building class: DOF tax exemptions for public or
/// subsidized housing, and certificates of occupancy dated 2009 or later from three sources
/// (DOB's two open datasets, and the FOIL list for the gap before 2012-07-12).
///
/// Fatal, like the 311 counts: a source that failed would show "No exemption found" on
/// buildings that have one, which is a wrong answer rather than a missing one.
fn fetch_good_cause(c: &Client, bbl_set: &HashSet<String>, until: &str) -> Result<GoodCauseFound> {
    let str_of = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);

    // The latest tax year only, so an exemption that has ended doesn't count.
    let latest = get_json_query(
        c,
        &format!("{SODA}/muvi-b6kx.json"),
        &[("$select".into(), "max(year) AS y".into())],
    )?;
    let year = arr(&latest)
        .first()
        .and_then(|r| str_of(r, "y"))
        .context("muvi-b6kx: no latest year")?;
    let codes = DOF_SUBSIDY_CODES
        .iter()
        .map(|(code, _)| format!("'{code}'"))
        .collect::<Vec<_>>()
        .join(",");
    let mut programs: HashMap<String, &'static str> = HashMap::new();
    let dof_rows = for_each_page(
        c,
        "muvi-b6kx",
        "parid,exmp_code",
        &format!("year='{year}' AND exmp_code in({codes})"),
        |page| {
            for v in page {
                let (Some(bbl), Some(code)) = (str_of(v, "parid"), str_of(v, "exmp_code")) else {
                    continue;
                };
                let program = DOF_SUBSIDY_CODES.iter().find(|(k, _)| *k == code);
                if let (Some((_, p)), true) = (program, bbl_set.contains(&bbl)) {
                    programs.entry(bbl).or_insert(p);
                }
            }
            Ok(())
        },
    )?;

    let mut cos: HashMap<String, LatestCo> = HashMap::new();
    let mut note = |v: &Value, date: Option<String>| {
        if let (Some(bbl), Some(job), Some(d)) = (str_of(v, "bbl"), str_of(v, "job_type"), date) {
            if bbl_set.contains(&bbl) {
                note_co(&mut cos, &bbl, is_new_building_job(&job), &d, until);
            }
        }
    };
    let bis_rows = for_each_page(
        c,
        "bs8b-p36w",
        "bbl,job_type,c_o_issue_date",
        "bbl IS NOT NULL AND c_o_issue_date >= '2009-01-01'",
        |page| {
            for v in page {
                note(
                    v,
                    str_of(v, "c_o_issue_date").as_deref().and_then(soda_date),
                );
            }
            Ok(())
        },
    )?;
    let now_rows = for_each_page(
        c,
        "pkdm-hqz6",
        "bbl,job_type,c_of_o_issuance_date",
        "bbl IS NOT NULL",
        |page| {
            for v in page {
                note(
                    v,
                    str_of(v, "c_of_o_issuance_date")
                        .as_deref()
                        .and_then(dobnow_date),
                );
            }
            Ok(())
        },
    )?;
    let foil_rows = fetch_foil_cos(c, bbl_set, until, &mut cos)?;

    let provenance = vec![
        (
            "muvi-b6kx",
            dof_rows,
            format!("DOF exemptions for public or subsidized housing, tax year {year}"),
        ),
        (
            "bs8b-p36w",
            bis_rows,
            "DOB certificates of occupancy (BIS), 2012-07-12 on".to_string(),
        ),
        (
            "pkdm-hqz6",
            now_rows,
            "DOB NOW certificates of occupancy".to_string(),
        ),
        (
            "justfix-foil-co-2000-2012",
            foil_rows,
            "DOB certificates of occupancy to 2012-07-11, FOIL 2024-810-4230, via JustFix"
                .to_string(),
        ),
    ];
    Ok((programs, cos, provenance))
}

/// Deletes the previous artifact so the next one starts from an empty schema. A file that
/// is still there afterwards means something has it open (on Windows, a running API), and
/// writing into it would mix the old schema with the new rows: a pilot rebuild failed with
/// "table buildings has no column named zip" that way.
pub(crate) fn remove_old(path: &str) -> Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e).with_context(|| {
            format!("could not replace {path}; stop the API (or anything else using it) and rerun")
        }),
        _ => Ok(()),
    }
}

pub fn run_real(cfg: &Config) -> Result<()> {
    let census_key = std::env::var("CENSUS_API_KEY")
        .context("set CENSUS_API_KEY (https://api.census.gov/data/key_signup.html)")?;
    let c = client();
    let started = Instant::now();
    let lap = |what: &str| {
        println!(
            "[{:>6.1} min] {what}",
            started.elapsed().as_secs_f64() / 60.0
        )
    };

    // 1. Buildings from PLUTO: every residential lot, or the pilot's blend.
    let mut coords: HashMap<String, (f64, f64)> = HashMap::new();
    let mut buildings: Vec<Building> = Vec::new();
    let mut take = |v: &Value| {
        if let Some(b) = parse_pluto(v) {
            if let Some((bbl, lat, lon)) = pluto_coords(v) {
                coords.insert(bbl, (lat, lon));
            }
            buildings.push(b);
        }
    };
    let scope = if cfg.citywide {
        for_each_page(&c, "64uk-42ks", PLUTO_SELECT, "unitsres>0", |page| {
            page.iter().for_each(&mut take);
            Ok(())
        })?;
        "city".to_string()
    } else {
        pilot_buildings(&c, cfg)?.iter().for_each(&mut take);
        format!("community district {}", cfg.community_district)
    };
    // Dedup in pull order (the pilot blend's two pulls overlap) and cap the pilot at --limit
    // BEFORE sorting: sorting first would keep the lowest BBLs instead of the blend. Then sort
    // by BBL so the serving table fills in key order.
    let mut seen: HashSet<String> = HashSet::new();
    buildings.retain(|b| seen.insert(b.bbl.clone()));
    if !cfg.citywide {
        buildings.truncate(cfg.limit as usize);
    }
    buildings.sort_by(|a, b| a.bbl.cmp(&b.bbl));
    let bbl_set: HashSet<String> = buildings.iter().map(|b| b.bbl.clone()).collect();
    lap(&format!(
        "PLUTO: {} residential buildings ({scope})",
        buildings.len()
    ));

    // 2. HPD violations, streamed into the DuckDB stage. Citywide that is the whole dataset;
    //    the pilot keeps its borough + tax-block filter.
    let hpd_scope = if cfg.citywide {
        String::new()
    } else {
        let blocks: Vec<u32> = buildings
            .iter()
            .filter_map(|b| bbl_block(&b.bbl).map(|(_, blk)| blk))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        hpd_where_blocks(cfg.community_district / 100, &blocks)
    };
    let stage = duckdb::Connection::open(cfg.stage_path())
        .with_context(|| format!("open stage {}", cfg.stage_path()))?;
    let (staged, excluded) = stage_violations(&c, &stage, &hpd_scope, &bbl_set)?;
    lap(&format!(
        "HPD: {staged} A/B/C violations staged in {}; skipped on our buildings: {}",
        cfg.stage_path(),
        excluded_classes_note(&excluded)
    ));

    // 3. DOB elevators: the whole dataset is ~120,000 devices, three pages, cheaper than
    //    asking about 767,563 BBLs 200 at a time.
    let mut has_elevator: HashSet<String> = HashSet::new();
    for_each_page(
        &c,
        "e5aq-a4j2",
        "bbl,device_type,device_status",
        "bbl IS NOT NULL",
        |page| {
            for v in page.iter().filter(|v| parse_dob_has_elevator(v)) {
                if let Some(bbl) = v.get("bbl").and_then(|x| x.as_str()) {
                    if bbl_set.contains(bbl) {
                        has_elevator.insert(bbl.to_string());
                    }
                }
            }
            Ok(())
        },
    )?;

    // 4. Census tract rent medians, one request per borough's county. Non-fatal: rent medians
    //    only feed /rent-fairness, not the building Health Card, so a Census outage or invalid
    //    key must not abort the whole ingest — warn and continue.
    let mut medians: HashMap<String, i32> = HashMap::new();
    for county in (1..=5).filter_map(county_fips) {
        match get_json(&c, &census_url(&census_key, county)) {
            Ok(v) => medians.extend(parse_census_medians(&v)),
            Err(e) => println!("warning: Census county {county} skipped ({e:#})"),
        }
    }
    println!("Census: {} tract medians loaded", medians.len());

    // 5. ADA subway stations (all of NYC; nearest ADA-accessible one per building).
    let mta = get_json(
        &c,
        "https://data.ny.gov/resource/39hk-dx4f.json?$limit=1000",
    )?;
    let stations: Vec<Station> = arr(&mta)
        .iter()
        .filter_map(|v| {
            Some(Station {
                lat: v.get("gtfs_latitude")?.as_str()?.trim().parse().ok()?,
                lon: v.get("gtfs_longitude")?.as_str()?.trim().parse().ok()?,
                ada: v
                    .get("ada")
                    .and_then(|x| x.as_str())
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0),
            })
        })
        .collect();

    // 6. Neighborhood context by ZIP, counted by Socrata: 311 requests since 2024 per ZIP over
    //    homes per ZIP. Fatal, unlike Census: without it every building's neighborhood pillar
    //    would fall back to the same 70, which is a different score, not a missing one.
    let complaints = parse_zip_counts(
        &get_json_complete(&c, &complaints_311_by_zip_query())?,
        "incident_zip",
    );
    let homes = parse_zip_counts(&get_json_complete(&c, &homes_by_zip_query())?, "zipcode");
    let rates = zip_rates(&complaints, &homes);
    let complaints_counted: i64 = rates.keys().filter_map(|z| complaints.get(z)).sum();
    // A 200 with an empty body or an error object parses to no rates, and every building would
    // score 70 without a word. Measured 2026-10-30: 213 of 767,563 buildings (0.03%) have no
    // ZIP rate, all for want of a ZIP in PLUTO; more than 1% means the source broke.
    let without_zip_rate = buildings
        .iter()
        .filter(|b| b.zip.as_ref().and_then(|z| rates.get(z)).is_none())
        .count();
    check_zip_rates(
        rates.len(),
        rates.values().filter(|&&r| r == 0).count(),
        without_zip_rate,
        buildings.len(),
    )?;
    // Neighborhood names are a label only, so a failure here leaves them blank and carries on.
    let ntas = get_json_complete(&c, &nta_query())
        .map(|v| parse_nta(&v))
        .unwrap_or_else(|e| {
            println!("warning: neighborhood names skipped ({e:#})");
            HashMap::new()
        });
    lap(&format!(
        "context: {} DOB elevators, {} stations, {} ZIP rates, {} neighborhood tracts",
        has_elevator.len(),
        stations.len(),
        rates.len(),
        ntas.len()
    ));

    // 7. Rent-stabilization unit counts. Source: JustFix.org (nyc-doffer), derived from NYC DOF
    //    Statement of Account records; latest year 2024. Streams the ~25 MB static CSV and keeps
    //    only rows whose BBL is in our set. Non-fatal: a download/parse failure must not abort
    //    the whole ingest — warn and continue with an empty map (buildings read "unverified").
    let rent_stab: HashMap<String, i32> = match fetch_rent_stab(&c, &bbl_set) {
        Ok(m) => m,
        Err(e) => {
            println!(
                "warning: rent-stabilization source skipped ({e:#}); rent_stabilized left null"
            );
            HashMap::new()
        }
    };

    // Scoring recency reads this snapshot year, not the wall clock, so runs are reproducible.
    let snapshot_year: i32 = std::env::var("SNAPSHOT_YEAR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2026);

    // 7b. Good Cause exemptions: subsidized housing and certificates of occupancy. The building
    //     class ones were read with PLUTO in step 1.
    let (programs, cos, good_cause_provenance) =
        fetch_good_cause(&c, &bbl_set, &format!("{snapshot_year}-12-31"))?;

    // 8. The serving artifact. Rebuilt from scratch every run, so a crash means rerun: no
    //    rollback journal and no syncs, which is most of the cost of 10.5M inserts.
    remove_old(&cfg.out)?;
    let conn = store::open_db(&cfg.out)?;
    conn.execute_batch("PRAGMA journal_mode = OFF; PRAGMA synchronous = OFF;")?;
    store::migrate(&conn)?;
    let provisions = store::load_law(&conn)?;
    println!("law: {provisions} provisions from data/law.json");
    store::set_snapshot_year(&conn, snapshot_year)?;
    match store::train_desc_dict(&description_samples(&stage)?) {
        Some(dict) => {
            store::set_desc_dict(&conn, &dict)?;
            lap(&format!(
                "description dictionary trained: {} KB",
                dict.len() / 1024
            ));
        }
        None => println!("too little description text to train a dictionary; plain zstd"),
    }

    let tx = conn.unchecked_transaction()?;
    let mut tracts_written: HashSet<String> = HashSet::new();
    let mut stabilized_count = 0usize;
    let mut exemptions: BTreeMap<&str, usize> = BTreeMap::new();
    for b in buildings.iter_mut() {
        if b.good_cause_exemption.is_none() {
            (b.good_cause_exemption, b.good_cause_detail) =
                later_exemption(programs.get(&b.bbl).copied(), cos.get(&b.bbl));
        }
        if let Some(e) = b.good_cause_exemption {
            *exemptions.entry(e.as_wire()).or_default() += 1;
        }
        b.has_elevator = has_elevator.contains(&b.bbl);
        if let Some((lat, lon)) = coords.get(&b.bbl) {
            b.latitude = Some(*lat);
            b.longitude = Some(*lon);
            b.near_ada_subway_m = nearest_ada_m(*lat, *lon, &stations).map(|d| d as i32);
        }
        b.neighborhood = ntas.get(&b.tract_geoid).cloned();
        b.complaints_311_per_1k_homes = b.zip.as_ref().and_then(|z| rates.get(z)).copied();
        // Rent-stabilization tri-state from the JustFix map: units>0 → stabilized, 0 → on
        // record with none, absent → left unknown (None) so the card reads "unverified".
        (b.rent_stabilized, b.rent_stab_units) = match rent_stab.get(&b.bbl) {
            Some(&units) if units > 0 => {
                stabilized_count += 1;
                (Some(true), Some(units))
            }
            Some(_) => (Some(false), Some(0)),
            None => (None, None),
        };
        store::upsert_building(&tx, b)?;
        if let Some(m) = medians.get(&b.tract_geoid) {
            if tracts_written.insert(b.tract_geoid.clone()) {
                store::upsert_tract_median(&tx, &b.tract_geoid, *m)?;
            }
        }
    }
    lap(&format!(
        "buildings written; rent-stab {stabilized_count}/{} stabilized (JustFix nyc-doffer 2024); \
         Good Cause exemptions {exemptions:?}",
        buildings.len()
    ));
    let bbls: Vec<i64> = buildings
        .iter()
        .filter_map(|b| b.bbl.parse().ok())
        .collect();
    let (stored_violations, outside) = compose_violations(&stage, &tx, &bbls)?;
    tx.commit()?;
    lap(&format!(
        "violations written: {stored_violations} on our buildings, {outside} staged rows on \
         lots outside the set (no homes in PLUTO, or no BBL)"
    ));

    // 9. Provenance. These rows travel with the database into the image, so the running
    //    container states its own provenance.
    //
    //    Unix seconds rather than a formatted date, deliberately: `chrono` is not a
    //    dependency of this workspace and adding one here to print a string would put a
    //    clock in reach of the scoring path. The API formats it.
    let ingested_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    for (k, v) in [
        ("ingested_at_unix", ingested_at.to_string()),
        ("coverage", scope.clone()),
        ("buildings", buildings.len().to_string()),
        ("violations", stored_violations.to_string()),
        ("violations_outside_buildings", outside.to_string()),
        ("tract_medians", tracts_written.len().to_string()),
        ("zip_rates", rates.len().to_string()),
        ("buildings_without_zip_rate", without_zip_rate.to_string()),
        // From the enum, so the published list cannot fall behind the classes actually kept.
        (
            "violation_classes",
            model::ViolationClass::ALL.map(|c| c.as_str()).join(","),
        ),
        (
            "violation_classes_excluded",
            excluded_classes_note(&excluded),
        ),
        (
            "sources",
            "HPD wvxf-dwi5 · 311 erm2-nwe9 (per ZIP) · PLUTO 64uk-42ks · DOB e5aq-a4j2 · \
             DCP NTA hm78-6dwm · MTA 39hk-dx4f · Census ACS5 2023 B25064 · JustFix nyc-doffer 2024 · \
             DOF exemptions muvi-b6kx · DOB CO bs8b-p36w, pkdm-hqz6 · DOB CO 2009 to 2012-07-11 \
             (FOIL 2024-810-4230, via JustFix)"
                .to_string(),
        ),
    ] {
        store::set_meta(&conn, k, &v)?;
    }

    // Per-source provenance: what each dataset gave us, and when.
    //
    // This is what lets an exported record attest to a fact rather than to a file. A hash
    // chain proves nobody edited our output; only the dataset id and retrieval time make
    // the document about HPD's data at a stated moment. Recorded per dataset because this
    // artifact is one full snapshot -- see the schema comment in `store` for why that
    // assumption breaks under incremental refresh.
    let hpd_note = format!("HPD violations, classes A/B/C, {scope}");
    let pluto_note = format!("PLUTO lots with homes, {scope}");
    for (dataset, rows, note) in [
        ("wvxf-dwi5", stored_violations as i64, hpd_note.as_str()),
        (
            "erm2-nwe9",
            complaints_counted,
            "311 requests since 2024-01-01, counted per ZIP by Socrata",
        ),
        ("64uk-42ks", buildings.len() as i64, pluto_note.as_str()),
        (
            "e5aq-a4j2",
            has_elevator.len() as i64,
            "DOB devices: active elevators on our buildings",
        ),
        (
            "hm78-6dwm",
            ntas.len() as i64,
            "DCP 2020 tract to neighborhood (NTA) names",
        ),
    ]
    .into_iter()
    .chain(
        good_cause_provenance
            .iter()
            .map(|(d, rows, note)| (*d, *rows as i64, note.as_str())),
    ) {
        store::set_source_provenance(&conn, dataset, ingested_at as i64, rows, Some(note))?;
    }

    conn.execute_batch("VACUUM")?;
    drop(conn);
    let size = std::fs::metadata(&cfg.out).map(|m| m.len()).unwrap_or(0);
    lap(&format!(
        "wrote {} ({:.1} MB): {} buildings, {stored_violations} violations, ingested_at_unix={ingested_at}",
        cfg.out,
        size as f64 / 1e6,
        buildings.len()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No old artifact is fine; one that cannot be deleted stops the run with what to do.
    #[test]
    fn remove_old_refuses_a_file_still_in_use() {
        let path = std::env::temp_dir().join(format!("hc-remove-old-{}.db", std::process::id()));
        let path = path.to_str().unwrap();
        remove_old(path).expect("nothing there");
        std::fs::write(path, b"old").unwrap();
        remove_old(path).expect("not in use");
        assert!(!std::path::Path::new(path).exists());
        // Windows refuses to delete a file another handle has open without share-delete, as
        // SQLite opens it; other systems allow it.
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            std::fs::write(path, b"old").unwrap();
            let held = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(1 | 2) // FILE_SHARE_READ | FILE_SHARE_WRITE, no delete
                .open(path)
                .unwrap();
            let err = remove_old(path).expect_err("in use");
            assert!(format!("{err:#}").contains("stop the API"), "{err:#}");
            drop(held);
            remove_old(path).unwrap();
        }
    }

    /// The two guards that keep a broken source from becoming a quietly thinner or flatter
    /// city: a pull outside Socrata's counts, and ZIP rates missing for many buildings.
    #[test]
    fn incomplete_sources_stop_the_run() {
        // Rows added during the pull: anything between the two counts is complete.
        assert!(check_complete("x", 105, 100, 110).is_ok());
        assert!(check_complete("x", 100, 100, 100).is_ok());
        // A short page that ended the pull early, or a cursor that skipped rows.
        assert!(check_complete("x", 50_000, 767_563, 767_563).is_err());

        // Citywide on 2026-10-30: 184 ZIPs, none at zero, 213 of 767,563 buildings without a rate.
        assert!(check_zip_rates(184, 0, 213, 767_563).is_ok());
        // An empty PLUTO count: no ZIPs at all.
        assert!(check_zip_rates(0, 0, 767_563, 767_563).is_err());
        // Rates for some ZIPs but missing for most buildings.
        assert!(check_zip_rates(3, 0, 700_000, 767_563).is_err());
        // An empty 311 count: every ZIP with homes at a real-looking zero, every building 100.
        let empty_311 = crate::sources::zip_rates(
            &HashMap::new(),
            &HashMap::from([("11221".to_string(), 20_000), ("10466".to_string(), 30_000)]),
        );
        let zero = empty_311.values().filter(|&&r| r == 0).count();
        assert!(check_zip_rates(empty_311.len(), zero, 0, 767_563).is_err());
    }

    /// The stage → artifact path, without the network: rows staged out of order and across
    /// buildings must come out grouped per building, in HPD's order, with dates and
    /// descriptions intact, and rows on lots outside the set must be counted, not written.
    #[test]
    fn compose_groups_staged_rows_by_building_in_fetch_order() -> Result<()> {
        let stage = duckdb::Connection::open_in_memory()?;
        stage.execute_batch(
            "CREATE TABLE hpd_violation (fetch_seq BIGINT, sid VARCHAR, bbl BIGINT,
               class VARCHAR, open BOOLEAN, year INTEGER, issued_on VARCHAR,
               closed_on VARCHAR, description VARCHAR);
             INSERT INTO hpd_violation VALUES
               (0, 'a', 3000020002, 'B', true,  2025, '2025-03-01', '2025-03-01', 'SECOND BLDG'),
               (1, 'b', 3000010001, 'C', true,  2026, '2026-01-05', '2026-01-05', 'FIRST, FIRST'),
               (2, 'c', 9999999999, 'A', true,  2026, '2026-01-05', '2026-01-05', 'NOT OURS'),
               (3, 'd', 3000010001, 'A', false, 2023, '2023-02-01', '2023-03-03', NULL),
               (4, 'e', 3000010001, 'B', true,  2024, NULL,         NULL,         'FIRST, LAST'),
               (5, 'f', NULL,       'B', true,  2024, NULL,         NULL,         'NO BBL');",
        )?;
        let conn = store::open_db(":memory:")?;
        store::migrate(&conn)?;
        let (written, outside) = compose_violations(&stage, &conn, &[3000010001, 3000020002])?;
        assert_eq!((written, outside), (4, 2));

        let open = store::get_open_violations(&conn, "3000010001")?;
        let got: Vec<_> = open
            .iter()
            .map(|v| {
                (
                    v.class.as_str(),
                    v.issued_on.as_deref(),
                    v.description.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                ("C", Some("2026-01-05"), Some("FIRST, FIRST")),
                ("B", None, Some("FIRST, LAST")),
            ]
        );
        // The closed one is there too: issued 2023-02-01, closed 2023-03-03.
        assert_eq!(
            store::closed_violation_durations(&conn, "3000010001", 2023)?,
            [30]
        );
        assert_eq!(store::get_open_violations(&conn, "3000020002")?.len(), 1);
        Ok(())
    }
}
