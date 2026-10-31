//! `ingest --law-check`: re-reads the source of every current provision and every legal-help
//! entry in `data/law.json`, and writes a report of what changed, moved, died or is due for a
//! hand re-check. It reports; it never edits `law.json`, the database or a release. A finding
//! is a question for a person (docs/design/law-change-detection.md).
//!
//! Change-only: each source keeps a cursor or an HTTP validator in the state file, so a quiet
//! week costs one small request per source. Only sources that signal a change are compared,
//! except on a full pass (`--full`, no state yet, or the first Monday of the month), which
//! compares everything whatever the cursors say.

use crate::lawnet::{Fetch, Net, Resp};
use model::law::{self, Compared, LawFact, LawSeed};
use model::urgency::{civil_ymd, days_from_civil, new_york_ymd};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Read;

const SENATE_API: &str = "https://legislation.nysenate.gov/api/3/laws";
const ADMIN_ZIP: &str = "https://files.amlegal.com/pdffiles/NewYorkCity/Admin/XML.zip";
/// The bulk file's entries holding Title 27 Chapter 2 (the Housing Maintenance Code) and the
/// introduction with the "Current through" line, as named in the build of 2026-10-30.
///
/// ponytail: named, not discovered. If amlegal renumbers them, every Admin Code row reports
/// "could not compare" and these names get updated.
const HMC_ENTRY: &str = "0-0-0-60027.xml";
const INTRO_ENTRY: &str = "0-0-0-1.xml";
/// Pages a script may not read are re-checked by a person this often.
const MANUAL_DAYS: i64 = 30;
/// GitHub's issue body limit is 65,536 characters.
const ISSUE_LIMIT: usize = 60_000;

pub struct Opts {
    pub law: String,
    pub report: String,
    pub texts: String,
    pub state: String,
    pub full: bool,
}

impl Opts {
    pub fn parse(args: &[String]) -> Opts {
        let flag = |name: &str, default: &str| {
            args.iter()
                .position(|a| a == name)
                .and_then(|i| args.get(i + 1))
                .cloned()
                .unwrap_or_else(|| default.to_string())
        };
        Opts {
            law: flag("--law-file", "data/law.json"),
            report: flag("--report", "law-check.md"),
            texts: flag("--texts", "law-check"),
            state: flag("--state", "law-check-state.json"),
            full: args.iter().any(|a| a == "--full"),
        }
    }
}

/// How a source is re-read.
#[derive(Debug, Clone, PartialEq)]
pub enum Route {
    /// NY Senate Open Legislation API, whole section: law id and section, e.g. `RPP`, `235-B`.
    Senate { law: String, loc: String },
    /// A page that answers a plain request; its excerpt is looked for in the page text.
    Page,
    /// An NYC Open Data dataset, read through its metadata API, not the script-built page.
    OpenData { view: String },
    /// An NYC Administrative Code section, from amlegal's official bulk XML, never the reader
    /// site (which answers scripts with 403).
    Amlegal { section: String },
    /// Bot-protected or refusing scripted requests: a person re-checks it in a browser.
    Manual,
}

/// The route for a source, by host. An unknown host is an error, so a row from a new site
/// cannot go unchecked by accident: someone picks its route here first.
pub fn route(id: &str, url: &str) -> Result<Route, String> {
    let u = reqwest::Url::parse(url).map_err(|_| format!("{id}: bad URL"))?;
    let host = u.host_str().unwrap_or_default();
    let segs: Vec<&str> = u
        .path_segments()
        .map(|s| s.filter(|x| !x.is_empty()).collect())
        .unwrap_or_default();
    Ok(match host {
        "www.nysenate.gov" => match segs.as_slice() {
            ["legislation", "laws", law, loc] => Route::Senate {
                law: law.to_string(),
                loc: loc.to_string(),
            },
            _ => return Err(format!("{id}: not a /legislation/laws/LAW/SECTION URL")),
        },
        "codelibrary.amlegal.com" => Route::Amlegal {
            section: id
                .strip_prefix("NYC-ADMIN-")
                .ok_or(format!("{id}: an amlegal row needs an NYC-ADMIN- id"))?
                .to_string(),
        },
        "data.cityofnewyork.us" => Route::OpenData {
            view: segs
                .last()
                .ok_or(format!("{id}: no dataset id"))?
                .to_string(),
        },
        // Answered plain requests on 2026-10-31; their robots.txt allows these paths.
        "portal.311.nyc.gov"
        | "hcr.ny.gov"
        | "housingcourtanswers.org"
        | "www.metcouncilonhousing.org"
        | "legalaidnyc.org"
        | "www.lawhelpny.org" => Route::Page,
        // nycourts.gov challenges every path; nyc.gov refuses scripted requests.
        "www.nycourts.gov" | "nycourts.gov" | "www.nyc.gov" | "www1.nyc.gov" | "nyc.gov" => {
            Route::Manual
        }
        _ => {
            return Err(format!(
                "{id}: no law-check route for {host}; pick one in lawcheck::route"
            ))
        }
    })
}

/// What the check found for one source.
#[derive(Debug, Clone, PartialEq)]
pub enum Finding {
    Unchanged,
    /// No change signal since the last check (cursor or validator), so not fetched.
    Quiet,
    Changed {
        note: String,
        /// Stored sentences no longer there.
        missing: Vec<String>,
        /// Facts whose quote is no longer there, as "`key` (value)".
        facts: Vec<String>,
    },
    /// Nothing stored is there at all: a different page or a broken extractor.
    Mismatch {
        note: String,
    },
    Moved {
        to: String,
        permanent: bool,
        /// Whether the stored text is on the page it moved to; `None` when it went to another
        /// site, which is not followed.
        same_text: Option<bool>,
    },
    Dead {
        status: u16,
        /// The last good copy in the Wayback Machine.
        archived: Option<String>,
        /// Where the Wayback Machine last saw the page redirect, and whether the stored text is
        /// there.
        candidate: Option<String>,
    },
    Blocked,
    Unreachable(String),
    NotChecked(String),
    /// A manual source last verified more than `MANUAL_DAYS` ago.
    Due,
}

impl Finding {
    /// Whether the next run must check this source in full, whatever its cursor says. Without
    /// this, a change reported once would fall silent the week after, because its validator
    /// already moved on.
    fn open(&self) -> bool {
        !matches!(self, Finding::Unchanged | Finding::Quiet | Finding::Due)
    }
}

pub struct Checked {
    pub id: String,
    pub url: String,
    pub help: bool,
    pub verified_on: String,
    pub finding: Finding,
    pub stored: String,
    pub fetched: Option<String>,
}

/// What a run leaves for the next one: cursors, validators and the open findings. Losing it
/// costs one full pass, nothing else.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub senate_cursor: Option<String>,
    #[serde(default)]
    pub amlegal_etag: Option<String>,
    #[serde(default)]
    pub amlegal_crc: Option<u32>,
    #[serde(default)]
    pub current_through: Option<String>,
    #[serde(default)]
    pub pages: BTreeMap<String, Validators>,
    #[serde(default)]
    pub open: BTreeSet<String>,
    #[serde(default)]
    pub last_full: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Validators {
    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub view_last_modified: Option<i64>,
}

/// One source to re-read: a current provision or a legal-help entry.
struct Item<'a> {
    id: String,
    url: &'a str,
    text: &'a str,
    facts: &'a [LawFact],
    verified_on: &'a str,
    help: bool,
}

fn items(seed: &LawSeed) -> Vec<Item<'_>> {
    let provisions = seed
        .provisions
        .iter()
        .filter(|p| p.status == "current")
        .map(|p| Item {
            id: p.id.clone(),
            url: &p.url,
            text: &p.text,
            facts: &p.facts,
            verified_on: &p.verified_on,
            help: false,
        });
    let help = seed.legal_help.iter().map(|h| Item {
        id: format!("legal help: {}", h.name),
        url: &h.url,
        text: &h.excerpt,
        facts: &[],
        verified_on: &h.verified_on,
        help: true,
    });
    provisions.chain(help).collect()
}

/// `YYYY-MM-DD` for days since the epoch.
fn ymd(days: i64) -> String {
    let (y, m, d) = civil_ymd(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_of(iso: &str) -> Option<i64> {
    let mut p = iso.get(..10)?.split('-').map(|x| x.parse::<i64>().ok());
    Some(days_from_civil(p.next()??, p.next()??, p.next()??))
}

/// Whether today's run compares everything: asked for, no state to go on, or the first Monday
/// of the month, the safety net under a cursor that misses a change.
pub fn full_pass(asked: bool, state_found: bool, today: i64) -> bool {
    let (_, _, d) = civil_ymd(today);
    let monday = (today + 3).rem_euclid(7) == 0; // 1970-01-01 was a Thursday
    asked || !state_found || (monday && d <= 7)
}

/// Re-read every source in `seed` and say what was found. `today` is days since the epoch in
/// New York. Fails only when the checker itself is broken: a host with no route.
pub fn check(
    seed: &LawSeed,
    net: &mut dyn Fetch,
    state: &mut State,
    today: i64,
    full: bool,
    senate_key: Option<&str>,
) -> Result<Vec<Checked>, String> {
    let items = items(seed);
    let routes = items
        .iter()
        .map(|it| route(&it.id, it.url))
        .collect::<Result<Vec<_>, _>>()?;
    let force: Vec<bool> = items
        .iter()
        .map(|it| full || state.open.contains(&it.id))
        .collect();
    let mut found: Vec<Option<(Finding, Option<String>)>> = vec![None; items.len()];

    let senate: Vec<usize> = (0..items.len())
        .filter(|&i| matches!(routes[i], Route::Senate { .. }))
        .collect();
    for (i, f) in check_senate(
        net, state, &items, &routes, &senate, &force, full, today, senate_key,
    ) {
        found[i] = Some(f);
    }
    let amlegal: Vec<usize> = (0..items.len())
        .filter(|&i| matches!(routes[i], Route::Amlegal { .. }))
        .collect();
    for (i, f) in check_amlegal(net, state, &items, &routes, &amlegal, &force) {
        found[i] = Some(f);
    }
    for (i, it) in items.iter().enumerate() {
        let f = match &routes[i] {
            Route::Page => check_page(net, state, it, force[i]),
            Route::OpenData { view } => check_open_data(net, state, it, view, force[i]),
            Route::Manual => {
                let age = days_of(it.verified_on).map_or(i64::MAX, |v| today - v);
                (
                    if age > MANUAL_DAYS {
                        Finding::Due
                    } else {
                        Finding::Quiet
                    },
                    None,
                )
            }
            Route::Senate { .. } | Route::Amlegal { .. } => continue,
        };
        found[i] = Some(f);
    }

    let checked: Vec<Checked> = items
        .iter()
        .zip(found)
        .map(|(it, f)| {
            let (finding, fetched) = f.expect("every route sets a finding");
            Checked {
                id: it.id.clone(),
                url: it.url.to_string(),
                help: it.help,
                verified_on: it.verified_on.to_string(),
                finding,
                stored: it.text.to_string(),
                fetched,
            }
        })
        .collect();
    state.open = checked
        .iter()
        .filter(|c| c.finding.open())
        .map(|c| c.id.clone())
        .collect();
    if full {
        state.last_full = Some(ymd(today));
    }
    Ok(checked)
}

/// A comparison as a finding, with the facts whose quote is gone.
fn judge(it: &Item, fetched: &str, compared: Compared, note: String) -> Finding {
    match compared {
        Compared::Unchanged => Finding::Unchanged,
        Compared::PageMismatch => Finding::Mismatch {
            note: "not one run of eight stored words is there: a different page or a broken \
                   extractor is more likely than a new law"
                .into(),
        },
        Compared::Changed { missing } => Finding::Changed {
            note,
            missing,
            facts: law::facts_missing(it.facts, fetched)
                .iter()
                .map(|f| format!("`{}` ({})", f.key, f.value))
                .collect(),
        },
    }
}

fn excerpt_note(stored: &str, page: &str) -> String {
    let page = law::normalize(page);
    let parts = law::excerpt_parts(stored);
    let gone = parts
        .iter()
        .filter(|p| !page.contains(&law::normalize(p)))
        .count();
    format!("{gone} of {} passages not found", parts.len())
}

/// A request that must come back with `want`, or the finding that explains why not.
fn fetch_ok(
    net: &mut dyn Fetch,
    url: &str,
    headers: &[(&str, String)],
    want: u16,
) -> Result<Resp, Finding> {
    match net.get(url, headers, false) {
        Err(e) => Err(Finding::Unreachable(e)),
        Ok(r) if r.blocked() => Err(Finding::Blocked),
        Ok(r) if r.status == 404 || r.status == 410 => Err(Finding::Dead {
            status: r.status,
            archived: None,
            candidate: None,
        }),
        Ok(r) if r.status != want => Err(Finding::Unreachable(format!("HTTP {}", r.status))),
        Ok(r) => Ok(r),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_senate(
    net: &mut dyn Fetch,
    state: &mut State,
    items: &[Item],
    routes: &[Route],
    rows: &[usize],
    force: &[bool],
    full: bool,
    today: i64,
    key: Option<&str>,
) -> Vec<(usize, (Finding, Option<String>))> {
    let Some(key) = key.filter(|k| !k.is_empty()) else {
        let why = "NYSENATE_API_KEY is not set".to_string();
        return rows
            .iter()
            .map(|&i| (i, (Finding::NotChecked(why.clone()), None)))
            .collect();
    };
    let loc_of = |i: usize| match &routes[i] {
        Route::Senate { law, loc } => (law.clone(), loc.clone()),
        _ => unreachable!(),
    };
    // Sections each law's update feed lists since the cursor. A law missing here (no cursor,
    // a full pass, or a failed call) has every tracked section fetched.
    let mut listed: HashMap<String, HashSet<String>> = HashMap::new();
    let mut feeds_ok = true;
    if let (false, Some(from)) = (full, state.senate_cursor.clone()) {
        let laws: BTreeSet<String> = rows.iter().map(|&i| loc_of(i).0).collect();
        for law in laws {
            let url = format!(
                "{SENATE_API}/{law}/updates/{from}/{}T00:00:00?type=processed&detail=true&limit=1000&key={key}",
                ymd(today + 2)
            );
            let feed = net
                .get(&url, &[], false)
                .ok()
                .filter(|r| r.status == 200)
                .and_then(|r| serde_json::from_slice::<Value>(&r.body).ok())
                .filter(|v| v["success"] == true);
            match feed {
                Some(v) => {
                    let set = v["result"]["items"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|it| it["id"]["locationId"].as_str())
                        .map(str::to_uppercase)
                        .collect();
                    listed.insert(law, set);
                }
                None => feeds_ok = false,
            }
        }
    }
    let mut out = Vec::new();
    for &i in rows {
        let it = &items[i];
        let (law, loc) = loc_of(i);
        if !force[i]
            && listed
                .get(&law)
                .is_some_and(|s| !s.contains(&loc.to_uppercase()))
        {
            out.push((i, (Finding::Quiet, None)));
            continue;
        }
        let url = format!("{SENATE_API}/{law}/{loc}?key={key}");
        let f = match net.get(&url, &[], false) {
            Err(e) => (Finding::Unreachable(e), None),
            Ok(r) if r.status == 401 || r.status == 403 => (
                Finding::NotChecked("the Open Legislation API refused the key".into()),
                None,
            ),
            Ok(r) if r.status != 200 => (Finding::Unreachable(format!("HTTP {}", r.status)), None),
            Ok(r) => match serde_json::from_slice::<Value>(&r.body) {
                Ok(v) if v["success"] == true => {
                    // The API's text carries its line breaks as a backslash and an `n`, not as
                    // new lines; Phase B's seed turned them into new lines, so this does too.
                    // Left as they were, they glued words together, and the first keyed run
                    // reported all 10 statutes changed.
                    let text = v["result"]["text"]
                        .as_str()
                        .unwrap_or_default()
                        .replace("\\n", "\n");
                    let active = v["result"]["activeDate"].as_str().unwrap_or("unknown");
                    let note = format!(
                        "the section differs from the stored copy (Open Legislation activeDate \
                         {active})"
                    );
                    (
                        judge(it, &text, law::compare_whole(it.text, &text), note),
                        Some(text),
                    )
                }
                // "The requested law document was not found"
                Ok(v) if v["errorCode"] == 21 => (
                    Finding::Dead {
                        status: 404,
                        archived: None,
                        candidate: None,
                    },
                    None,
                ),
                _ => (
                    Finding::Unreachable("an unreadable API response".into()),
                    None,
                ),
            },
        };
        out.push((i, f));
    }
    // A day of overlap: the feed's times are the API's own clock, and seeing an update twice
    // only costs one comparison.
    if feeds_ok {
        state.senate_cursor = Some(format!("{}T00:00:00", ymd(today - 1)));
    }
    out
}

struct ZipEntry {
    method: u16,
    crc: u32,
    csize: u64,
    size: u64,
    off: u64,
}

fn u16le(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn broken(what: &str) -> Finding {
    Finding::Mismatch {
        note: format!("the bulk file could not be read: {what}"),
    }
}

/// The zip's directory, read from its last 256 KB (and the directory's own range when it
/// starts earlier), never the whole file.
fn zip_directory(
    net: &mut dyn Fetch,
    url: &str,
) -> Result<(Option<String>, HashMap<String, ZipEntry>), Finding> {
    let tail = fetch_ok(net, url, &[("range", "bytes=-262144".into())], 206)?;
    let etag = tail.header("etag").map(String::from);
    let total: u64 = tail
        .header("content-range")
        .and_then(|c| c.rsplit('/').next())
        .and_then(|t| t.parse().ok())
        .ok_or_else(|| broken("no Content-Range"))?;
    let b = &tail.body;
    let eocd = b
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .ok_or_else(|| broken("no end of central directory"))?;
    let cd_size = u32le(b, eocd + 12).ok_or_else(|| broken("short EOCD"))? as u64;
    let cd_off = u32le(b, eocd + 16).ok_or_else(|| broken("short EOCD"))? as u64;
    let tail_start = total - b.len() as u64;
    let cd = if cd_off >= tail_start {
        let at = (cd_off - tail_start) as usize;
        b.get(at..at + cd_size as usize)
            .ok_or_else(|| broken("directory out of range"))?
            .to_vec()
    } else {
        let range = format!("bytes={cd_off}-{}", cd_off + cd_size - 1);
        fetch_ok(net, url, &[("range", range)], 206)?.body
    };
    let mut entries = HashMap::new();
    let mut p = 0;
    while cd.get(p..p + 4) == Some(b"PK\x01\x02") {
        let field = |at| u32le(&cd, p + at).map(u64::from);
        let (
            Some(method),
            Some(crc),
            Some(csize),
            Some(size),
            Some(nlen),
            Some(xlen),
            Some(clen),
            Some(off),
        ) = (
            u16le(&cd, p + 10),
            u32le(&cd, p + 16),
            field(20),
            field(24),
            u16le(&cd, p + 28),
            u16le(&cd, p + 30),
            u16le(&cd, p + 32),
            field(42),
        )
        else {
            return Err(broken("short directory entry"));
        };
        let name_at = p + 46;
        let name = cd
            .get(name_at..name_at + nlen as usize)
            .map(|n| String::from_utf8_lossy(n).replace('\\', "/"))
            .ok_or_else(|| broken("short directory entry"))?;
        entries.insert(
            name,
            ZipEntry {
                method,
                crc,
                csize,
                size,
                off,
            },
        );
        p = name_at + nlen as usize + xlen as usize + clen as usize;
    }
    Ok((etag, entries))
}

/// One entry of the zip, by byte range, checked against its CRC.
fn zip_entry(net: &mut dyn Fetch, url: &str, e: &ZipEntry) -> Result<Vec<u8>, Finding> {
    // The local header is 30 bytes plus a name and extra field; 1 KB covers both.
    let range = format!("bytes={}-{}", e.off, e.off + 30 + 1024 + e.csize - 1);
    let r = fetch_ok(net, url, &[("range", range)], 206)?;
    let b = &r.body;
    if b.get(..4) != Some(b"PK\x03\x04") {
        return Err(broken("no local header"));
    }
    let start = 30
        + u16le(b, 26).ok_or_else(|| broken("short header"))? as usize
        + u16le(b, 28).ok_or_else(|| broken("short header"))? as usize;
    let data = b
        .get(start..start + e.csize as usize)
        .ok_or_else(|| broken("short entry"))?;
    let raw = match e.method {
        0 => data.to_vec(),
        8 => {
            let mut out = Vec::with_capacity(e.size as usize);
            flate2::read::DeflateDecoder::new(data)
                .read_to_end(&mut out)
                .map_err(|_| broken("bad deflate data"))?;
            out
        }
        _ => return Err(broken("unknown compression")),
    };
    let mut crc = flate2::Crc::new();
    crc.update(&raw);
    if crc.sum() != e.crc || raw.len() as u64 != e.size {
        return Err(broken("CRC or size mismatch"));
    }
    Ok(raw)
}

/// Rendered text of an amlegal XML element: a TAB is a space, a line break a new line, and the
/// `[ALP S-nnn]` supplement markers are dropped (editorial, not on the reader page). The rule
/// the 12 captured sections matched on 2026-10-31.
fn xml_text(n: roxmltree::Node) -> String {
    let mut s = String::new();
    for c in n.children() {
        if c.is_text() {
            s.push_str(c.text().unwrap_or_default());
        } else if c.has_tag_name("TAB") {
            s.push(' ');
        } else if c.has_tag_name("LINEBRK") {
            s.push('\n');
        } else if c.has_tag_name("HIGHLIGHTER") && c.attribute("name") == Some("ALP") {
        } else if c.is_element() {
            s.push_str(&xml_text(c));
        }
    }
    s
}

/// `27-2005` from a heading like `§ 27-2005 Duties of owner.`
fn section_number(heading: &str) -> Option<String> {
    heading.match_indices("27-").find_map(|(i, _)| {
        let rest = heading[i + 3..].trim_start();
        let mut num = String::new();
        let mut chars = rest.chars().peekable();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() {
                num.push(c);
                chars.next();
            } else if c == '.' && !num.is_empty() {
                let mut ahead = chars.clone();
                ahead.next();
                if ahead.peek().is_some_and(char::is_ascii_digit) {
                    num.push(c);
                    chars.next();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        (!num.is_empty()).then(|| format!("27-{num}"))
    })
}

/// Each section in an amlegal chapter: its text (paragraphs joined by new lines) and the
/// `version` of each of its records.
fn amlegal_sections(xml: &str) -> Result<HashMap<String, (String, BTreeSet<u32>)>, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let mut out = HashMap::new();
    for lvl in doc
        .descendants()
        .filter(|n| n.has_tag_name("LEVEL") && n.attribute("style-name") == Some("Section"))
    {
        let heading: String = lvl
            .children()
            .find(|n| n.has_tag_name("RECORD"))
            .and_then(|r| r.children().find(|n| n.has_tag_name("HEADING")))
            .map(|h| h.descendants().filter_map(|n| n.text()).collect())
            .unwrap_or_default();
        let Some(sec) = section_number(&heading) else {
            continue;
        };
        let paras: Vec<String> = lvl
            .descendants()
            .filter(|n| n.has_tag_name("PARA"))
            .map(xml_text)
            .collect();
        let versions = lvl
            .descendants()
            .filter(|n| n.has_tag_name("RECORD"))
            .filter_map(|r| r.attribute("version")?.parse().ok())
            .collect();
        out.insert(sec, (paras.join("\n"), versions));
    }
    Ok(out)
}

fn current_through(xml: &str) -> Option<String> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    doc.descendants()
        .filter(|n| n.has_tag_name("PARA"))
        .map(|p| law::normalize(&xml_text(p)))
        .find(|t| t.starts_with("Current through"))
}

fn check_amlegal(
    net: &mut dyn Fetch,
    state: &mut State,
    items: &[Item],
    routes: &[Route],
    rows: &[usize],
    force: &[bool],
) -> Vec<(usize, (Finding, Option<String>))> {
    if rows.is_empty() {
        return Vec::new();
    }
    let all = |f: Finding| -> Vec<(usize, (Finding, Option<String>))> {
        rows.iter().map(|&i| (i, (f.clone(), None))).collect()
    };
    let forced = rows.iter().any(|&i| force[i]);
    if let (false, Some(etag)) = (forced, state.amlegal_etag.clone()) {
        match net.get(ADMIN_ZIP, &[("if-none-match", etag)], true) {
            Ok(r) if r.status == 304 => return all(Finding::Quiet),
            Ok(r) if r.blocked() => return all(Finding::Blocked),
            _ => {}
        }
    }
    let (etag, dir) = match zip_directory(net, ADMIN_ZIP) {
        Ok(d) => d,
        Err(f) => return all(f),
    };
    let Some(hmc) = dir
        .iter()
        .find(|(n, _)| n.ends_with(HMC_ENTRY))
        .map(|(_, e)| e)
    else {
        return all(broken("the Housing Maintenance Code entry is not in it"));
    };
    if !forced && state.amlegal_crc == Some(hmc.crc) {
        state.amlegal_etag = etag;
        return all(Finding::Quiet);
    }
    if let Some(intro) = dir
        .iter()
        .find(|(n, _)| n.ends_with(INTRO_ENTRY))
        .map(|(_, e)| e)
    {
        if let Ok(x) = zip_entry(net, ADMIN_ZIP, intro) {
            state.current_through = current_through(&String::from_utf8_lossy(&x));
        }
    }
    let sections = match zip_entry(net, ADMIN_ZIP, hmc)
        .and_then(|x| amlegal_sections(&String::from_utf8_lossy(&x)).map_err(|e| broken(&e)))
    {
        Ok(s) => s,
        Err(f) => return all(f),
    };
    state.amlegal_etag = etag;
    state.amlegal_crc = Some(hmc.crc);
    let through = state.current_through.clone().unwrap_or_default();
    rows.iter()
        .map(|&i| {
            let it = &items[i];
            let Route::Amlegal { section } = &routes[i] else {
                unreachable!()
            };
            let f = match sections.get(section) {
                None => (
                    Finding::Mismatch {
                        note: format!("§ {section} is not in the bulk file's chapter"),
                    },
                    None,
                ),
                Some((text, versions)) => {
                    let note = format!(
                        "the section differs from the stored copy (amlegal bulk XML, {through}; \
                         record versions {versions:?})"
                    );
                    (
                        judge(it, text, law::compare_whole(it.text, text), note),
                        Some(text.clone()),
                    )
                }
            };
            (i, f)
        })
        .collect()
}

/// Tags that start a new block of text on the page.
const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "br",
    "button",
    "caption",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "label",
    "li",
    "main",
    "nav",
    "ol",
    "option",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

/// The text of an HTML page: tags stripped, `script` and `style` dropped, entities decoded, a
/// new line wherever a block starts. Not a renderer: enough to find a copied passage, and a
/// page it reads wrong shows as "could not compare", never as a changed law.
pub fn html_text(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let mut out = String::with_capacity(html.len() / 2);
    let mut at = 0;
    while let Some(rel) = html[at..].find('<') {
        let lt = at + rel;
        out.push_str(&html[at..lt]);
        let next = html[lt + 1..].chars().next().unwrap_or(' ');
        if !(next.is_ascii_alphabetic() || next == '/' || next == '!' || next == '?') {
            out.push('<');
            at = lt + 1;
            continue;
        }
        if lower[lt..].starts_with("<!--") {
            at = lower[lt..].find("-->").map_or(html.len(), |e| lt + e + 3);
            continue;
        }
        let Some(gt) = html[lt..].find('>').map(|g| lt + g) else {
            at = html.len();
            break;
        };
        let tag = &lower[lt + 1..gt];
        let closing = tag.starts_with('/');
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        at = gt + 1;
        if !closing
            && matches!(
                name.as_str(),
                "script" | "style" | "noscript" | "template" | "svg"
            )
        {
            let close = format!("</{name}");
            at = lower[at..]
                .find(&close)
                .and_then(|c| lower[at + c..].find('>').map(|g| at + c + g + 1))
                .unwrap_or(html.len());
            out.push('\n');
        } else if BLOCK_TAGS.contains(&name.as_str()) {
            out.push('\n');
        }
    }
    out.push_str(&html[at..]);
    decode_entities(&out)
}

/// Character references: numeric, and the named ones these pages use.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let decoded = rest[1..].find(';').filter(|&e| e <= 10).and_then(|e| {
            let name = &rest[1..1 + e];
            let c = if let Some(n) = name.strip_prefix("#x").or(name.strip_prefix("#X")) {
                u32::from_str_radix(n, 16).ok().and_then(char::from_u32)
            } else if let Some(n) = name.strip_prefix('#') {
                n.parse().ok().and_then(char::from_u32)
            } else {
                Some(match name {
                    "amp" => '&',
                    "lt" => '<',
                    "gt" => '>',
                    "quot" => '"',
                    "apos" => '\'',
                    "nbsp" => '\u{a0}',
                    "rsquo" => '’',
                    "lsquo" => '‘',
                    "rdquo" => '”',
                    "ldquo" => '“',
                    "ndash" => '–',
                    "mdash" => '—',
                    "hellip" => '…',
                    "sect" => '§',
                    "para" => '¶',
                    "deg" => '°',
                    "copy" => '©',
                    "reg" => '®',
                    "trade" => '™',
                    "bull" => '•',
                    "middot" => '·',
                    "frac12" => '½',
                    "times" => '×',
                    "aacute" => 'á',
                    "eacute" => 'é',
                    "iacute" => 'í',
                    "oacute" => 'ó',
                    "uacute" => 'ú',
                    "agrave" => 'à',
                    "egrave" => 'è',
                    "ograve" => 'ò',
                    "ntilde" => 'ñ',
                    "ccedil" => 'ç',
                    "uuml" => 'ü',
                    _ => return None,
                })
            };
            c.map(|c| (c, e + 2))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The text of a PDF, by `pdftotext -enc UTF-8` (poppler-utils on the runner), with the
/// "PAGE 2 OF 2" footers dropped: the stored copy of DHCR Fact Sheet 9 runs across one.
fn pdf_text(pdf: &[u8]) -> Result<String, String> {
    let path = std::env::temp_dir().join(format!("hc-law-check-{}.pdf", std::process::id()));
    std::fs::write(&path, pdf).map_err(|e| e.to_string())?;
    let out = std::process::Command::new("pdftotext")
        .args(["-enc", "UTF-8"])
        .arg(&path)
        .arg("-")
        .output();
    let _ = std::fs::remove_file(&path);
    let out = out.map_err(|_| "pdftotext is not installed".to_string())?;
    if !out.status.success() {
        return Err("pdftotext could not read it".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !is_page_footer(l))
        .collect::<Vec<_>>()
        .join("\n"))
}

fn is_page_footer(line: &str) -> bool {
    let w: Vec<&str> = line.trim().trim_matches('\u{c}').split(' ').collect();
    matches!(w.as_slice(), [p, n, o, m] if p.eq_ignore_ascii_case("page")
        && o.eq_ignore_ascii_case("of")
        && n.parse::<u32>().is_ok()
        && m.parse::<u32>().is_ok())
}

/// Same site: the same host once a leading `www.` is dropped.
fn same_site(a: &str, b: &str) -> bool {
    let host = |u: &str| {
        reqwest::Url::parse(u).ok().and_then(|u| {
            u.host_str()
                .map(|h| h.trim_start_matches("www.").to_string())
        })
    };
    host(a).is_some() && host(a) == host(b)
}

/// GET `url`, following up to five redirects on the same site. Returns the last response and,
/// when it moved, the first `Location` and whether the move was permanent.
fn follow(
    net: &mut dyn Fetch,
    url: &str,
    headers: &[(&str, String)],
) -> Result<(Resp, Option<(String, bool)>), String> {
    let mut at = url.to_string();
    let mut moved = None;
    for hop in 0..=5 {
        let r = net.get(&at, if hop == 0 { headers } else { &[] }, false)?;
        let location = r
            .header("location")
            .filter(|_| (300..400).contains(&r.status) && r.status != 304)
            .and_then(|l| reqwest::Url::parse(&at).ok()?.join(l).ok())
            .map(|u| u.to_string());
        let Some(next) = location else {
            return Ok((r, moved));
        };
        moved.get_or_insert((next.clone(), matches!(r.status, 301 | 308)));
        if !same_site(url, &next) || hop == 5 {
            return Ok((r, moved));
        }
        at = next;
    }
    unreachable!()
}

fn page_text(r: &Resp) -> Result<String, String> {
    if r.body.starts_with(b"%PDF") {
        pdf_text(&r.body)
    } else {
        Ok(html_text(&String::from_utf8_lossy(&r.body)))
    }
}

fn check_page(
    net: &mut dyn Fetch,
    state: &mut State,
    it: &Item,
    force: bool,
) -> (Finding, Option<String>) {
    let mut headers = Vec::new();
    if !force {
        if let Some(v) = state.pages.get(it.url) {
            if let Some(e) = &v.etag {
                headers.push(("if-none-match", e.clone()));
            }
            if let Some(m) = &v.last_modified {
                headers.push(("if-modified-since", m.clone()));
            }
        }
    }
    let (r, moved) = match follow(net, it.url, &headers) {
        Ok(x) => x,
        Err(e) => return (Finding::Unreachable(e), None),
    };
    if r.blocked() {
        return (Finding::Blocked, None);
    }
    match r.status {
        304 => (Finding::Quiet, None),
        404 | 410 => (dead(net, it, r.status), None),
        200 => {
            let text = match page_text(&r) {
                Ok(t) => t,
                Err(e) => return (Finding::NotChecked(e), None),
            };
            let compared = law::compare_excerpt(it.text, &text);
            if let Some((to, permanent)) = moved {
                let same_text = Some(compared == Compared::Unchanged);
                return (
                    Finding::Moved {
                        to,
                        permanent,
                        same_text,
                    },
                    Some(text),
                );
            }
            state.pages.insert(
                it.url.to_string(),
                Validators {
                    etag: r.header("etag").map(String::from),
                    last_modified: r.header("last-modified").map(String::from),
                    view_last_modified: None,
                },
            );
            let note = excerpt_note(it.text, &text);
            (judge(it, &text, compared, note), Some(text))
        }
        s if (300..400).contains(&s) => {
            let (to, permanent) = moved.unwrap_or_default();
            (
                Finding::Moved {
                    to,
                    permanent,
                    same_text: None,
                },
                None,
            )
        }
        s => (Finding::Unreachable(format!("HTTP {s}")), None),
    }
}

/// The dataset's description and column descriptions, in the form the stored row was copied in:
/// the description, then one `Name (column description): …` line per described column.
fn open_data_text(v: &Value) -> String {
    let mut s = v["description"].as_str().unwrap_or_default().to_string();
    for c in v["columns"].as_array().into_iter().flatten() {
        if let (Some(n), Some(d)) = (c["name"].as_str(), c["description"].as_str()) {
            s.push_str(&format!("\n{n} (column description): {d}"));
        }
    }
    s
}

fn check_open_data(
    net: &mut dyn Fetch,
    state: &mut State,
    it: &Item,
    view: &str,
    force: bool,
) -> (Finding, Option<String>) {
    let url = format!("https://data.cityofnewyork.us/api/views/{view}.json");
    let r = match fetch_ok(net, &url, &[], 200) {
        Ok(r) => r,
        Err(Finding::Dead { status, .. }) => return (dead(net, it, status), None),
        Err(f) => return (f, None),
    };
    let Ok(v) = serde_json::from_slice::<Value>(&r.body) else {
        return (
            Finding::Unreachable("an unreadable metadata response".into()),
            None,
        );
    };
    let modified = v["viewLastModified"].as_i64();
    let seen = state.pages.get(it.url).and_then(|p| p.view_last_modified);
    if !force && modified.is_some() && modified == seen {
        return (Finding::Quiet, None);
    }
    state.pages.insert(
        it.url.to_string(),
        Validators {
            view_last_modified: modified,
            ..Default::default()
        },
    );
    let text = open_data_text(&v);
    let note = excerpt_note(it.text, &text);
    (
        judge(it, &text, law::compare_excerpt(it.text, &text), note),
        Some(text),
    )
}

/// A dead page, with where to look for its replacement: the last good copy in the Wayback
/// Machine, and the last redirect it recorded after that, scored by whether the stored text is
/// on the page it points to.
///
/// ponytail: Wayback only. New entries in the host's sitemap since last week are the other
/// lead the design names; add them when a dead page here goes unexplained.
fn dead(net: &mut dyn Fetch, it: &Item, status: u16) -> Finding {
    let mut archived = None;
    let mut candidate = None;
    let cdx = reqwest::Url::parse_with_params(
        "https://web.archive.org/cdx/search/cdx",
        &[
            ("url", it.url),
            ("output", "json"),
            ("fl", "timestamp,statuscode"),
            ("limit", "-25"),
        ],
    );
    let rows: Vec<(String, String)> = cdx
        .ok()
        .and_then(|u| net.get(u.as_str(), &[], false).ok())
        .filter(|r| r.status == 200)
        .and_then(|r| serde_json::from_slice::<Vec<Vec<String>>>(&r.body).ok())
        .unwrap_or_default()
        .into_iter()
        .skip(1)
        .filter_map(|row| Some((row.first()?.clone(), row.get(1)?.clone())))
        .collect();
    let good = rows
        .iter()
        .rev()
        .find(|(_, s)| s == "200")
        .map(|(t, _)| t.clone());
    if let Some(t) = &good {
        archived = Some(format!("https://web.archive.org/web/{t}/{}", it.url));
    }
    let redirect = rows
        .iter()
        .rev()
        .find(|(t, s)| s.starts_with('3') && good.as_ref().is_none_or(|g| t > g));
    if let Some((t, _)) = redirect {
        let snap = format!("https://web.archive.org/web/{t}id_/{}", it.url);
        let target = net
            .get(&snap, &[], false)
            .ok()
            .and_then(|r| r.header("location").map(String::from))
            .and_then(|l| {
                l.split_once("id_/")
                    .map(|(_, rest)| rest.to_string())
                    .or(Some(l))
            });
        if let Some(target) = target {
            let verdict = if same_site(it.url, &target) {
                match net.get(&target, &[], false) {
                    Ok(r) if r.status == 200 && !r.blocked() => match page_text(&r) {
                        Ok(t) if law::compare_excerpt(it.text, &t) == Compared::Unchanged => {
                            "the stored text is there"
                        }
                        Ok(_) => "the stored text is not all there",
                        Err(_) => "could not read it",
                    },
                    _ => "it did not answer",
                }
            } else {
                "another site, not fetched"
            };
            candidate = Some(format!("{target} ({verdict})"));
        }
    }
    Finding::Dead {
        status,
        archived,
        candidate,
    }
}

fn file_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// The issue body: findings a person must verify, most urgent kinds first. Every item starts
/// with its bold id, and nothing else does: `law-check.yml` opens the issue only when a line
/// starts with `**`.
pub fn report(
    date: &str,
    checked: &[Checked],
    state: &State,
    full: bool,
    traffic: (u32, u64),
) -> String {
    let count = |f: fn(&Finding) -> bool| checked.iter().filter(|c| f(&c.finding)).count();
    let changed = count(|f| matches!(f, Finding::Changed { .. } | Finding::Mismatch { .. }));
    let gone = count(|f| matches!(f, Finding::Moved { .. } | Finding::Dead { .. }));
    let due = count(|f| matches!(f, Finding::Due));
    let mut s = format!(
        "## Law check {date}: {changed} changed, {gone} dead or moved, {due} due for re-check\n\n\
         Nothing here has changed what HouseCheck serves. Each item needs a person to verify it.\n"
    );
    let section = |s: &mut String, title: &str, lines: Vec<String>| {
        s.push_str(&format!("\n### {title}\n"));
        if lines.is_empty() {
            s.push_str("(none)\n");
        }
        for l in lines {
            s.push_str(&l);
        }
    };
    let fetched_line = |c: &Checked| {
        if c.fetched.is_some() {
            format!(
                "Fetched text: workflow artifact `law-check`, file `{}.fetched.txt`.\n",
                file_id(&c.id)
            )
        } else {
            String::new()
        }
    };

    let mut lines = Vec::new();
    for c in checked {
        match &c.finding {
            Finding::Changed {
                note,
                missing,
                facts,
            } => {
                let mut l = format!("**{}**: {}\n{note}.", c.id, c.url);
                if missing.is_empty() {
                    l.push_str(" Nothing stored is missing; words were added.\n");
                } else {
                    l.push_str(" Stored sentences that are not there:\n");
                    for m in missing.iter().take(5) {
                        l.push_str(&format!("> {m}\n"));
                    }
                    if missing.len() > 5 {
                        l.push_str(&format!("> (and {} more)\n", missing.len() - 5));
                    }
                }
                for f in facts {
                    l.push_str(&format!("Fact {f}: its quote is no longer there.\n"));
                }
                l.push_str(&fetched_line(c));
                lines.push(l + "\n");
            }
            Finding::Mismatch { note } => {
                lines.push(format!(
                    "**{}**: {}\nCould not compare: {note}.\n{}\n",
                    c.id,
                    c.url,
                    fetched_line(c)
                ));
            }
            _ => {}
        }
    }
    section(&mut s, "Changed", lines);

    let mut lines = Vec::new();
    for c in checked {
        match &c.finding {
            Finding::Moved {
                to,
                permanent,
                same_text,
            } => {
                let how = if *permanent {
                    "permanently"
                } else {
                    "temporarily"
                };
                let text = match same_text {
                    Some(true) => "the stored text is there",
                    Some(false) => "the stored text is not all there",
                    None => "another site, not followed",
                };
                lines.push(format!(
                    "**{}**: {}\nMoved {how} to {to}: {text}. Change the URL by PR once a person \
                     has checked it.\n\n",
                    c.id, c.url
                ));
            }
            Finding::Dead {
                status,
                archived,
                candidate,
            } => {
                let mut l = format!(
                    "**{}**: {}\nHTTP {status}: the page is gone. Still served until changed.\n",
                    c.id, c.url
                );
                l.push_str(&match archived {
                    Some(a) => format!("Last good copy: {a}\n"),
                    None => "No good copy in the Wayback Machine.\n".into(),
                });
                if let Some(cand) = candidate {
                    l.push_str(&format!("Possible replacement: {cand}\n"));
                }
                lines.push(l + "\n");
            }
            _ => {}
        }
    }
    section(&mut s, "Dead or moved", lines);

    let lines = checked
        .iter()
        .filter(|c| c.finding == Finding::Due)
        .map(|c| {
            format!(
                "**{}**: {}. Last verified {}.\n",
                c.id, c.url, c.verified_on
            )
        })
        .collect();
    section(&mut s, "Due for manual re-check (open in a browser)", lines);

    let lines = checked
        .iter()
        .filter_map(|c| {
            let why = match &c.finding {
                Finding::Blocked => "a bot check answered instead of the page".to_string(),
                Finding::Unreachable(e) => format!("unreachable ({e})"),
                Finding::NotChecked(e) => format!("not checked: {e}"),
                _ => return None,
            };
            Some(format!("**{}**: {}. {why}.\n", c.id, c.url))
        })
        .collect();
    section(&mut s, "Not checked this time", lines);

    let lines = checked
        .iter()
        .filter_map(|c| match &c.finding {
            Finding::Changed { .. } if c.help => Some(format!(
                "- {}: `find_legal_help` hands the model this entry, so recorded answers that \
                 quote its number or hours fail replay. **Re-record likely needed** after the fix \
                 (billed; Aisling runs it).\n",
                c.id
            )),
            Finding::Changed { facts, .. } if !facts.is_empty() => Some(format!(
                "- {}: a fact quote is gone. If the fact's value changes, recordings that state \
                 it fail replay. **Re-record likely needed** after the fix (billed; Aisling runs \
                 it).\n",
                c.id
            )),
            Finding::Changed { .. } => Some(format!(
                "- {}: the wording changed and every fact quote is still there. Recordings still \
                 replay, against turns written for the old wording: your call.\n",
                c.id
            )),
            _ => None,
        })
        .collect();
    section(&mut s, "Evals", lines);

    let unchanged = count(|f| *f == Finding::Unchanged);
    let quiet = count(|f| *f == Finding::Quiet);
    s.push_str(&format!(
        "\n---\nChecked {} sources: {unchanged} unchanged, {quiet} not compared (no change signal \
         since the last check, or a manual re-check not yet due){}. {} requests, {} KB.{}\n",
        checked.len(),
        if full {
            " (full pass: every source compared)"
        } else {
            ""
        },
        traffic.0,
        traffic.1 / 1024,
        state
            .current_through
            .as_ref()
            .map(|t| format!(" NYC Admin Code bulk file: {t}"))
            .unwrap_or_default()
    ));
    if s.len() > ISSUE_LIMIT {
        let mut cut = ISSUE_LIMIT;
        while !s.is_char_boundary(cut) {
            cut -= 1;
        }
        s.truncate(cut);
        s.push_str("\n\n(Cut at GitHub's size limit: the whole report is `law-check.md` in the workflow artifact.)\n");
    }
    s
}

/// `ingest --law-check`.
pub fn run(o: &Opts) -> anyhow::Result<()> {
    let seed: LawSeed = serde_json::from_str(&std::fs::read_to_string(&o.law)?)
        .map_err(|e| anyhow::anyhow!("{}: {e}", o.law))?;
    let saved = std::fs::read_to_string(&o.state).ok();
    let mut state: State = saved
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let (y, m, d) = new_york_ymd(secs);
    let today = days_from_civil(y, m.into(), d.into());
    let full = full_pass(o.full, saved.is_some(), today);
    let key = std::env::var("NYSENATE_API_KEY").ok();
    let mut net = Net::new()?;
    let checked = check(&seed, &mut net, &mut state, today, full, key.as_deref())
        .map_err(|e| anyhow::anyhow!(e))?;

    std::fs::create_dir_all(&o.texts)?;
    for c in &checked {
        println!("{:<34} {}", c.id, label(&c.finding));
        if let (
            Finding::Changed { .. } | Finding::Mismatch { .. } | Finding::Moved { .. },
            Some(f),
        ) = (&c.finding, &c.fetched)
        {
            let dir = std::path::Path::new(&o.texts);
            std::fs::write(
                dir.join(format!("{}.stored.txt", file_id(&c.id))),
                &c.stored,
            )?;
            std::fs::write(dir.join(format!("{}.fetched.txt", file_id(&c.id))), f)?;
        }
    }
    let body = report(
        &ymd(today),
        &checked,
        &state,
        full,
        (net.requests(), net.bytes()),
    );
    std::fs::write(&o.report, &body)?;
    std::fs::write(&o.state, serde_json::to_string_pretty(&state)? + "\n")?;
    println!(
        "law check {}: {} sources, {} open findings, {} requests, {} KB; report at {}",
        ymd(today),
        checked.len(),
        state.open.len(),
        net.requests(),
        net.bytes() / 1024,
        o.report
    );
    Ok(())
}

fn label(f: &Finding) -> &'static str {
    match f {
        Finding::Unchanged => "unchanged",
        Finding::Quiet => "quiet",
        Finding::Changed { .. } => "CHANGED",
        Finding::Mismatch { .. } => "COULD NOT COMPARE",
        Finding::Moved { .. } => "MOVED",
        Finding::Dead { .. } => "DEAD",
        Finding::Blocked => "blocked",
        Finding::Unreachable(_) => "unreachable",
        Finding::NotChecked(_) => "not checked",
        Finding::Due => "due for re-check",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    /// The network as a function of the request, counting what it would have sent.
    struct Fake<F> {
        f: F,
        requests: u32,
        bytes: u64,
    }

    impl<F: FnMut(&str, &HashMap<String, String>, bool) -> Resp> Fetch for Fake<F> {
        fn get(
            &mut self,
            url: &str,
            headers: &[(&str, String)],
            head: bool,
        ) -> Result<Resp, String> {
            self.requests += 1;
            let h = headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect();
            let r = (self.f)(url, &h, head);
            self.bytes += r.body.len() as u64;
            Ok(r)
        }
        fn bytes(&self) -> u64 {
            self.bytes
        }
        fn requests(&self) -> u32 {
            self.requests
        }
    }

    fn fake<F: FnMut(&str, &HashMap<String, String>, bool) -> Resp>(f: F) -> Fake<F> {
        Fake {
            f,
            requests: 0,
            bytes: 0,
        }
    }

    fn ok(body: impl Into<Vec<u8>>) -> Resp {
        Resp {
            status: 200,
            body: body.into(),
            ..Default::default()
        }
    }

    fn status(s: u16) -> Resp {
        Resp {
            status: s,
            ..Default::default()
        }
    }

    fn redirect(s: u16, to: &str) -> Resp {
        let mut r = status(s);
        r.headers.insert("location".into(), to.into());
        r
    }

    fn today() -> i64 {
        days_from_civil(2026, 10, 31)
    }

    /// A saved copy of https://portal.311.nyc.gov/article/?kanumber=KA-01036, read 2026-10-31.
    const KA: &str = include_str!("../tests/fixtures/nyc311-ka-01036.html");

    fn real_seed() -> LawSeed {
        serde_json::from_str(include_str!("../../../data/law.json")).expect("law.json")
    }

    fn only(ids: &[&str]) -> LawSeed {
        let mut s = real_seed();
        s.provisions.retain(|p| ids.contains(&p.id.as_str()));
        s.legal_help.clear();
        s
    }

    fn seed_of(id: &str, url: &str, text: &str) -> LawSeed {
        serde_json::from_value(json!({ "sources": [], "legal_help": [], "provisions": [{
            "id": id, "source": "s", "citation": "c", "title": "t", "text": text, "url": url,
            "retrieved_at": "2026-10-31", "verified_by": "fetch", "verified_on": "2026-10-31",
            "status": "current", "occupancy": "residential", "topics": ["heat"], "facts": []
        }] }))
        .unwrap()
    }

    #[test]
    fn every_source_in_law_json_has_a_route() {
        for it in items(&real_seed()) {
            route(&it.id, it.url).unwrap_or_else(|e| panic!("{e}"));
        }
        assert!(route("X", "https://example.org/a").is_err());
    }

    /// The design's planted fixture: 62 changed to 64 on the real 311 page must be flagged,
    /// naming the 62°F sentence and `heat.night.min_f`; the page as saved must flag nothing.
    #[test]
    fn a_changed_number_on_the_311_page_is_flagged_with_its_fact() {
        let seed = only(&["NYC311-KA-01036"]);
        let mut state = State::default();
        let c = check(
            &seed,
            &mut fake(|_, _, _| ok(KA)),
            &mut state,
            today(),
            true,
            None,
        )
        .unwrap();
        assert_eq!(c[0].finding, Finding::Unchanged);

        let altered = KA.replace("at least 62 degrees", "at least 64 degrees");
        assert_ne!(altered, KA);
        let c = check(
            &seed,
            &mut fake(|_, _, _| ok(altered.clone())),
            &mut state,
            today(),
            true,
            None,
        )
        .unwrap();
        let Finding::Changed {
            note,
            missing,
            facts,
        } = &c[0].finding
        else {
            panic!("{:?}", c[0].finding)
        };
        assert_eq!(note, "1 of 4 passages not found");
        assert_eq!(
            missing,
            &[
                "Between 10 PM and 6 AM, the inside temperature must be at least 62 degrees \
               Fahrenheit at all times."
            ]
        );
        assert_eq!(facts, &["`heat.night.min_f` (62)"]);
        let body = report("2026-10-31", &c, &state, true, (1, 1));
        assert!(body.contains("### Changed\n**NYC311-KA-01036**"), "{body}");
        assert!(body.contains("Fact `heat.night.min_f` (62)"), "{body}");
        assert!(body.contains("**Re-record likely needed**"), "{body}");
    }

    /// law-check.yml decides "any findings?" by a line starting with `**`.
    #[test]
    fn report_items_and_only_items_start_bold() {
        let row = |id: &str, finding: Finding| Checked {
            id: id.into(),
            url: "https://example.org/a".into(),
            help: false,
            verified_on: "2026-09-01".into(),
            finding,
            stored: "s".into(),
            fetched: Some("f".into()),
        };
        let bold = |c: &[Checked]| {
            report("2026-10-31", c, &State::default(), false, (1, 1))
                .lines()
                .filter(|l| l.starts_with("**"))
                .count()
        };
        let quiet = [row("A", Finding::Unchanged), row("B", Finding::Quiet)];
        assert_eq!(bold(&quiet), 0);
        let found = [
            Finding::Changed {
                note: "n".into(),
                missing: vec!["m".into()],
                facts: vec!["`k` (1)".into()],
            },
            Finding::Mismatch { note: "n".into() },
            Finding::Moved {
                to: "https://example.org/b".into(),
                permanent: true,
                same_text: Some(true),
            },
            Finding::Dead {
                status: 404,
                archived: None,
                candidate: None,
            },
            Finding::Blocked,
            Finding::Unreachable("e".into()),
            Finding::NotChecked("no key".into()),
            Finding::Due,
        ];
        for f in found {
            assert_eq!(bold(&[row("X", f.clone())]), 1, "{f:?}");
        }
    }

    #[test]
    fn a_challenge_page_is_blocked_and_never_compared() {
        let seed = only(&["NYC311-KA-01036"]);
        let mut net = fake(|_, _, _| {
            // Synthetic: the shape of a Cloudflare challenge.
            let mut r = status(403);
            r.headers.insert("server".into(), "cloudflare".into());
            r.body = b"<title>Just a moment...</title> Between 10 PM and 6 AM".to_vec();
            r
        });
        let c = check(&seed, &mut net, &mut State::default(), today(), true, None).unwrap();
        assert_eq!(c[0].finding, Finding::Blocked);
        assert_eq!(net.requests, 1);
    }

    /// A server that answers 304 to its old validator, whatever the page now says.
    fn stale_validator(page: String) -> impl FnMut(&str, &HashMap<String, String>, bool) -> Resp {
        move |_: &str, h: &HashMap<String, String>, _: bool| {
            if h.contains_key("if-none-match") {
                status(304)
            } else {
                ok(page.clone())
            }
        }
    }

    /// The cursor path can miss a change (here a validator that did not move); the monthly full
    /// pass must report it, and it stays reported until it is fixed.
    #[test]
    fn a_change_the_cursor_misses_is_found_by_the_monthly_full_pass() {
        let seed = only(&["NYC311-KA-01036"]);
        let mut state = State::default();
        state.pages.insert(
            seed.provisions[0].url.clone(),
            Validators {
                etag: Some("\"v1\"".into()),
                ..Default::default()
            },
        );
        let altered = KA.replace("at least 62 degrees", "at least 64 degrees");
        let c = check(
            &seed,
            &mut fake(stale_validator(altered.clone())),
            &mut state,
            today(),
            false,
            None,
        )
        .unwrap();
        assert_eq!(c[0].finding, Finding::Quiet);

        let monday = days_from_civil(2026, 11, 2);
        assert!(full_pass(false, true, monday), "first Monday of the month");
        assert!(!full_pass(false, true, monday + 7));
        assert!(full_pass(false, false, today()), "no state yet");
        let c = check(
            &seed,
            &mut fake(stale_validator(altered.clone())),
            &mut state,
            monday,
            true,
            None,
        )
        .unwrap();
        assert!(matches!(c[0].finding, Finding::Changed { .. }));
        let c = check(
            &seed,
            &mut fake(stale_validator(altered)),
            &mut state,
            monday + 7,
            false,
            None,
        )
        .unwrap();
        assert!(
            matches!(c[0].finding, Finding::Changed { .. }),
            "an open finding is re-checked"
        );
    }

    #[test]
    fn moved_and_dead_pages_come_with_where_they_went() {
        let seed = only(&["NYC311-KA-01036"]);
        let url = seed.provisions[0].url.clone();
        let mut net = fake(|u, _, _| {
            if u.ends_with("KA-01036") {
                redirect(301, "/article/?kanumber=KA-01036-new")
            } else {
                ok(KA)
            }
        });
        let c = check(&seed, &mut net, &mut State::default(), today(), true, None).unwrap();
        assert_eq!(
            c[0].finding,
            Finding::Moved {
                to: "https://portal.311.nyc.gov/article/?kanumber=KA-01036-new".into(),
                permanent: true,
                same_text: Some(true),
            }
        );

        let mut net = fake(|u, _, _| {
            if u.starts_with("https://web.archive.org/cdx/") {
                ok(
                    r#"[["timestamp","statuscode"],["20260101000000","200"],["20261020000000","301"]]"#,
                )
            } else if u.starts_with("https://web.archive.org/web/20261020000000id_/") {
                redirect(
                    301,
                    "https://web.archive.org/web/20261020000000id_/https://portal.311.nyc.gov/article/?kanumber=KA-09999",
                )
            } else if u.ends_with("KA-09999") {
                ok(KA)
            } else {
                status(404)
            }
        });
        let c = check(&seed, &mut net, &mut State::default(), today(), true, None).unwrap();
        assert_eq!(
            c[0].finding,
            Finding::Dead {
                status: 404,
                archived: Some(format!("https://web.archive.org/web/20260101000000/{url}")),
                candidate: Some(
                    "https://portal.311.nyc.gov/article/?kanumber=KA-09999 (the stored text is there)"
                        .into()
                ),
            }
        );
    }

    #[test]
    fn senate_sections_are_fetched_only_when_the_update_feed_lists_them() {
        let text = "§ 235-b. Warranty of habitability. 1. In every written or oral lease.";
        let seed = seed_of(
            "NY-RPL-235-B",
            "https://www.nysenate.gov/legislation/laws/RPP/235-B",
            text,
        );
        let doc = |t: &str| {
            ok(
                json!({"success": true, "result": {"text": t, "activeDate": "2024-05-03"}})
                    .to_string(),
            )
        };
        let feed = |locs: &[&str]| {
            let items: Vec<Value> = locs
                .iter()
                .map(|l| json!({"id": {"lawId": "RPP", "locationId": l}}))
                .collect();
            ok(json!({"success": true, "result": {"items": items}}).to_string())
        };
        let mut state = State::default();
        let mut net = fake(|u, _, _| {
            assert!(u.ends_with("/RPP/235-B?key=K"), "{u}");
            // Line breaks as the API sends them: a backslash and an `n`.
            doc(&text.replace(' ', "\\n"))
        });
        let c = check(&seed, &mut net, &mut state, today(), true, Some("K")).unwrap();
        assert_eq!(c[0].finding, Finding::Unchanged);
        assert_eq!(state.senate_cursor.as_deref(), Some("2026-10-30T00:00:00"));

        // The feed lists another section: one call, nothing fetched.
        let mut net = fake(|u, _, _| {
            assert!(
                u.contains("/RPP/updates/2026-10-30T00:00:00/2026-11-09T00:00:00?"),
                "{u}"
            );
            feed(&["211"])
        });
        let c = check(&seed, &mut net, &mut state, today() + 7, false, Some("K")).unwrap();
        assert_eq!((c[0].finding.clone(), net.requests), (Finding::Quiet, 1));

        // Listed and changed: fetched and compared.
        let mut net = fake(|u, _, _| {
            if u.contains("/updates/") {
                feed(&["235-B"])
            } else {
                doc("§ 235-b. Warranty of habitability. 1. In every lease.")
            }
        });
        let c = check(&seed, &mut net, &mut state, today() + 14, false, Some("K")).unwrap();
        assert!(
            matches!(&c[0].finding, Finding::Changed { note, .. } if note.contains("activeDate 2024-05-03")),
            "{:?}",
            c[0].finding
        );

        let mut net = fake(|_, _, _| panic!("no request without a key"));
        let c = check(&seed, &mut net, &mut State::default(), today(), true, None).unwrap();
        assert!(matches!(c[0].finding, Finding::NotChecked(_)));
    }

    /// A zip with deflated entries, in the layout amlegal's bulk file uses.
    fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let (mut out, mut cd) = (Vec::new(), Vec::new());
        for (name, data) in entries {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data).unwrap();
            let comp = enc.finish().unwrap();
            let mut crc = flate2::Crc::new();
            crc.update(data);
            let sizes = [
                crc.sum().to_le_bytes(),
                (comp.len() as u32).to_le_bytes(),
                (data.len() as u32).to_le_bytes(),
            ]
            .concat();
            let off = out.len() as u32;
            out.extend(b"PK\x03\x04\x14\0\0\0\x08\0\0\0\0\0");
            out.extend(&sizes);
            out.extend((name.len() as u16).to_le_bytes());
            out.extend([0, 0]);
            out.extend(name.as_bytes());
            out.extend(&comp);
            cd.extend(b"PK\x01\x02\x14\0\x14\0\0\0\x08\0\0\0\0\0");
            cd.extend(&sizes);
            cd.extend((name.len() as u16).to_le_bytes());
            cd.extend([0; 12]);
            cd.extend(off.to_le_bytes());
            cd.extend(name.as_bytes());
        }
        let cd_off = out.len() as u32;
        out.extend(&cd);
        out.extend(b"PK\x05\x06\0\0\0\0");
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((cd.len() as u32).to_le_bytes());
        out.extend(cd_off.to_le_bytes());
        out.extend([0, 0]);
        out
    }

    /// The bulk file's server: ETags, HEAD, and byte ranges. Asking for the whole file fails.
    fn serve_zip(
        zip: Vec<u8>,
        etag: &str,
    ) -> impl FnMut(&str, &HashMap<String, String>, bool) -> Resp {
        let etag = etag.to_string();
        move |url: &str, h: &HashMap<String, String>, head: bool| {
            assert_eq!(url, ADMIN_ZIP);
            let mut r = status(200);
            r.headers.insert("etag".into(), etag.clone());
            if h.get("if-none-match") == Some(&etag) {
                r.status = 304;
                return r;
            }
            if head {
                return r;
            }
            let range = h.get("range").expect("the whole file was requested");
            let n = zip.len();
            let (a, b) = match range
                .strip_prefix("bytes=")
                .unwrap()
                .split_once('-')
                .unwrap()
            {
                ("", k) => (n - k.parse::<usize>().unwrap().min(n), n - 1),
                (a, b) => (a.parse().unwrap(), b.parse::<usize>().unwrap().min(n - 1)),
            };
            r.status = 206;
            r.headers
                .insert("content-range".into(), format!("bytes {a}-{b}/{n}"));
            r.body = zip[a..=b].to_vec();
            r
        }
    }

    #[test]
    fn amlegal_is_read_in_pieces_and_is_quiet_while_the_etag_holds() {
        let chapter = |n: &str| {
            format!(
                r#"<DOCUMENT><LEVEL style-name="Section"><RECORD version="7"><HEADING>§ 27-2029 Minimum temperatures.</HEADING><PARA>a. From October first<TAB/>through May thirty-first <HIGHLIGHTER name="ALP">[ALP S-1]</HIGHLIGHTER>at least {n} degrees.</PARA></RECORD></LEVEL></DOCUMENT>"#
            )
        };
        let intro = r#"<DOCUMENT><PARA>Current through Local Law 2026/160,<LINEBRK/>and so on.</PARA></DOCUMENT>"#;
        // Bytes that do not compress, so the file is far bigger than the pieces read.
        let mut x = 0x2545_f491_4f6c_dd1du64;
        let filler: Vec<u8> = (0..1_000_000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x as u8
            })
            .collect();
        let build = |n: &str| {
            zip(&[
                ("XML/filler.bin", &filler),
                (&format!("XML/{INTRO_ENTRY}"), intro.as_bytes()),
                (&format!("XML/{HMC_ENTRY}"), chapter(n).as_bytes()),
            ])
        };
        let seed = seed_of(
            "NYC-ADMIN-27-2029",
            "https://codelibrary.amlegal.com/codes/newyorkcity/latest/NYCadmin/0-0-0-60027",
            "a. From October first through May thirty-first at least 62 degrees.",
        );
        let v1 = build("62");
        assert!(v1.len() > 900_000);
        let mut state = State::default();
        let mut net = fake(serve_zip(v1.clone(), "\"e1\""));
        let c = check(&seed, &mut net, &mut state, today(), true, None).unwrap();
        assert_eq!(c[0].finding, Finding::Unchanged);
        assert!(
            net.bytes < 300_000,
            "read {} of {} bytes",
            net.bytes,
            v1.len()
        );
        assert_eq!(
            state.current_through.as_deref(),
            Some("Current through Local Law 2026/160, and so on.")
        );

        // Same ETag: one HEAD answered 304, nothing read.
        let mut net = fake(serve_zip(v1, "\"e1\""));
        let c = check(&seed, &mut net, &mut state, today() + 7, false, None).unwrap();
        assert_eq!(
            (c[0].finding.clone(), net.requests, net.bytes),
            (Finding::Quiet, 1, 0)
        );

        // A new build with the number changed.
        let mut net = fake(serve_zip(build("64"), "\"e2\""));
        let c = check(&seed, &mut net, &mut state, today() + 14, false, None).unwrap();
        assert!(
            matches!(&c[0].finding, Finding::Changed { note, .. } if note.contains("record versions {7}")),
            "{:?}",
            c[0].finding
        );
    }

    #[test]
    fn html_text_reads_what_a_reader_sees() {
        let h = "<html><head><style>p{}</style><script>var a = '<p>no</p>';</script></head>\
                 <body><p>New York City&rsquo;s rule:</p><div>at&nbsp;least&#32;62&#x20;degrees \
                 &amp; more <b>bold</b>text</div><!-- <p>hidden</p> --> 3 < 4 &unknown;</body>";
        assert_eq!(
            law::normalize(&html_text(h)),
            "New York City’s rule: at least 62 degrees & more boldtext 3 < 4 &unknown;"
        );
        assert!(is_page_footer("PAGE 2 OF 2"));
        assert!(is_page_footer("\u{c}Page 10 of 12"));
        assert!(!is_page_footer("page two of the lease"));
    }
}
