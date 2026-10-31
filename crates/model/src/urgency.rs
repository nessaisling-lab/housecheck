//! Whether a reader's question is time-sensitive (docs/design/law-passages.md §7).
//!
//! Rules, not the model: the reader's words are read for danger, emergency conditions and
//! deadlines, and a date next to a deadline is counted in days from `today` (New York). The
//! model may raise the level later; it never computes a date.

use serde::{Deserialize, Serialize};

/// Ordered: a later variant is more urgent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// 15 to 30 days.
    ComingUp,
    /// 4 to 14 days.
    TimeSensitive,
    /// 3 days or less, a date already passed, a marshal or lockout, danger, or an emergency
    /// condition.
    Urgent,
}

impl Level {
    /// The level a deadline this many days away calls for. A date already passed is urgent.
    pub fn for_days(days: i64) -> Option<Level> {
        match days {
            ..=3 => Some(Level::Urgent),
            4..=14 => Some(Level::TimeSensitive),
            15..=30 => Some(Level::ComingUp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Urgency {
    /// `None` when a deadline was named but no date: the agent asks for it first.
    pub level: Option<Level>,
    /// In plain words: "court date", "gas smell".
    pub reason: String,
    /// Immediate danger: the answer opens with 911.
    pub danger: bool,
    /// The deadline's date, `YYYY-MM-DD`, when the reader gave one.
    pub date: Option<String>,
    /// Days from `counted_on` to `date`; negative when it has passed.
    pub days_left: Option<i64>,
    /// A deadline named with no date.
    pub needs_date: bool,
    /// The New York date the days were counted from.
    pub counted_on: String,
}

const DANGER: &[(&str, &str)] = &[
    ("smell gas", "gas smell"),
    ("smells like gas", "gas smell"),
    ("gas smell", "gas smell"),
    ("gas leak", "gas leak"),
    ("carbon monoxide", "carbon monoxide"),
    ("co alarm", "carbon monoxide alarm"),
    ("smoke", "smoke"),
    ("fire", "fire"),
    ("sparking", "sparking wires"),
    ("exposed wire", "exposed wires"),
    ("ceiling collapsed", "collapsed ceiling"),
    ("ceiling is collapsing", "collapsing ceiling"),
];

const EMERGENCY: &[(&str, &str)] = &[
    ("no heat", "no heat"),
    ("heat is off", "no heat"),
    ("heat isn't working", "no heat"),
    ("heat is not working", "no heat"),
    ("without heat", "no heat"),
    ("no hot water", "no hot water"),
    ("no running water", "no water"),
    ("no water", "no water"),
    ("locked out", "lockout"),
    ("locked me out", "lockout"),
    ("lockout", "lockout"),
    ("changed the locks", "lockout"),
    ("marshal", "marshal's notice"),
];

const DEADLINE: &[(&str, &str)] = &[
    ("court date", "court date"),
    ("hearing", "hearing date"),
    ("in court", "court date"),
    ("go to court", "court date"),
    ("answer the petition", "court papers"),
    ("answered the petition", "court papers"),
    ("got a petition", "court papers"),
    ("received a petition", "court papers"),
    ("notice of petition", "court papers"),
    ("court papers", "court papers"),
    ("served", "court papers"),
    ("eviction notice", "eviction notice"),
    ("notice to vacate", "notice to vacate"),
    ("evicting me", "eviction"),
    ("evict me", "eviction"),
    ("being evicted", "eviction"),
    ("eviction case", "eviction"),
    ("rent demand", "rent demand"),
    ("14-day", "rent demand"),
    ("deadline", "deadline"),
    ("due date", "deadline"),
    ("due on", "deadline"),
    ("due by", "deadline"),
];

const CHILD: &[&str] = &[
    "child", "baby", "kid", "toddler", "infant", "son", "daughter",
];

const MONTHS: &[&str] = &[
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const WEEKDAYS: &[&str] = &[
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
];
const SMALL: &[&str] = &[
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen",
];

/// Days since 1970-01-01. Howard Hinnant's algorithm, as in `Violation::days_open`.
pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 {
            yoe + era * 400 + 1
        } else {
            yoe + era * 400
        },
        m,
        d,
    )
}

fn parse_iso(s: &str) -> Option<i64> {
    let (y, rest) = s.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    valid(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
}

fn valid(y: i64, m: i64, d: i64) -> Option<i64> {
    let dim = [
        31,
        28 + i64::from(y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)),
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    ((1..=12).contains(&m) && d >= 1 && d <= dim[(m - 1) as usize])
        .then(|| days_from_civil(y, m, d))
}

fn month_of(tok: &str) -> Option<i64> {
    let t = tok.trim_end_matches('.');
    if t.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| {
            *m == t || (t.len() >= 3 && m.starts_with(t) && (t.len() == 3 || t == "sept"))
        })
        .map(|i| i as i64 + 1)
}

fn day_of(tok: &str) -> Option<i64> {
    let t = tok.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let d: i64 = t.parse().ok()?;
    (t.len() <= 2 && (1..=31).contains(&d)).then_some(d)
}

fn count_of(tok: &str) -> Option<i64> {
    tok.parse::<i64>()
        .ok()
        .or_else(|| SMALL.iter().position(|w| *w == tok).map(|i| i as i64))
}

/// Every date the text names, as days since 1970-01-01. A date with no year is this year's,
/// or next year's when this year's passed more than two weeks ago.
fn dates_in(text: &str, today: i64) -> Vec<i64> {
    let (ty, _, _) = civil_from_days(today);
    let toks: Vec<&str> = text
        .split_whitespace()
        .map(|t| {
            t.trim_matches(|c: char| ",;:!?()\"'".contains(c))
                .trim_end_matches('.')
        })
        .filter(|t| !t.is_empty())
        .collect();
    let year_for = |m: i64, d: i64, y: Option<i64>| -> Option<i64> {
        match y {
            Some(y) => valid(y, m, d),
            None => {
                let this = valid(ty, m, d)?;
                if this < today - 14 {
                    valid(ty + 1, m, d)
                } else {
                    Some(this)
                }
            }
        }
    };
    let year_tok = |t: Option<&&str>| -> Option<i64> {
        t.and_then(|t| t.parse::<i64>().ok())
            .filter(|y| (2000..=2100).contains(y))
    };
    let mut out = Vec::new();
    for (i, &t) in toks.iter().enumerate() {
        let prev = |k: usize| i.checked_sub(k).map(|j| toks[j]);
        match t {
            "today" | "tonight" => out.push(today),
            "tomorrow" if prev(1) == Some("after") && prev(2) == Some("day") => out.push(today + 2),
            "tomorrow" => out.push(today + 1),
            "days" | "day" | "weeks" | "week" => {
                if let (Some(n), Some("in")) = (prev(1).and_then(count_of), prev(2)) {
                    out.push(today + if t.starts_with("week") { 7 * n } else { n });
                }
            }
            _ => {}
        }
        if let Some(w) = WEEKDAYS.iter().position(|w| *w == t) {
            let dow = (today + 4).rem_euclid(7); // 1970-01-01 was a Thursday; 0 = Sunday
            let ahead = (w as i64 - dow).rem_euclid(7);
            out.push(today + if ahead == 0 { 7 } else { ahead });
        }
        if let Some(m) = month_of(t) {
            // "November 5", "Nov 5th, 2026"
            if let Some(d) = toks.get(i + 1).and_then(|n| day_of(n)) {
                out.extend(year_for(m, d, year_tok(toks.get(i + 2))));
            // "5 November", "the 5th of November"
            } else if let Some(d) = prev(1).and_then(day_of).or_else(|| {
                (prev(1) == Some("of"))
                    .then(|| prev(2).and_then(day_of))
                    .flatten()
            }) {
                out.extend(year_for(m, d, year_tok(toks.get(i + 1))));
            }
        }
        if t.contains('/') {
            let p: Vec<&str> = t.split('/').collect();
            if (2..=3).contains(&p.len())
                && p.iter()
                    .all(|x| !x.is_empty() && x.bytes().all(|c| c.is_ascii_digit()))
            {
                let (m, d) = (p[0].parse().unwrap_or(0), p[1].parse().unwrap_or(0));
                let y = p.get(2).map(|y| {
                    let y: i64 = y.parse().unwrap_or(0);
                    if y < 100 {
                        2000 + y
                    } else {
                        y
                    }
                });
                out.extend(year_for(m, d, y));
            }
        } else if t.len() == 10 && t.as_bytes()[4] == b'-' {
            out.extend(parse_iso(t));
        }
    }
    out
}

fn first_match<'a>(text: &str, table: &'a [(&str, &str)]) -> Option<&'a str> {
    table
        .iter()
        .find(|(k, _)| text.contains(k))
        .map(|(_, r)| *r)
}

/// How urgent the reader's messages make this conversation, or `None` if nothing in them is
/// time-sensitive. `today` is `YYYY-MM-DD`, New York time.
pub fn assess(user_messages: &[&str], today: &str) -> Option<Urgency> {
    let today_n = parse_iso(today)?;
    let text = user_messages.join(" \n ").to_lowercase();
    let mut out = Urgency {
        level: None,
        reason: String::new(),
        danger: false,
        date: None,
        days_left: None,
        needs_date: false,
        counted_on: today.to_string(),
    };
    let raise = |out: &mut Urgency, level: Level, reason: &str| {
        if out.level.is_none_or(|l| level > l) {
            out.level = Some(level);
            out.reason = reason.to_string();
        }
    };
    if let Some(r) = first_match(&text, DANGER) {
        out.danger = true;
        raise(&mut out, Level::Urgent, r);
    }
    if let Some(r) = first_match(&text, EMERGENCY) {
        raise(&mut out, Level::Urgent, r);
    }
    if (text.contains("peeling paint") || text.contains("lead paint"))
        && CHILD.iter().any(|c| text.contains(c))
    {
        raise(
            &mut out,
            Level::Urgent,
            "peeling paint with a child at home",
        );
    }
    if let Some(r) = first_match(&text, DEADLINE)
        .or_else(|| (out.reason == "marshal's notice" || out.reason == "lockout").then_some(""))
    {
        // The deadline the reader is closest to: the nearest date not more than two weeks gone.
        let date = dates_in(&text, today_n)
            .into_iter()
            .filter(|d| *d >= today_n - 14)
            .min_by_key(|d| (d - today_n).abs());
        match date {
            Some(d) => {
                let days = d - today_n;
                let (y, m, dd) = civil_from_days(d);
                out.date = Some(format!("{y:04}-{m:02}-{dd:02}"));
                out.days_left = Some(days);
                if let Some(l) = Level::for_days(days) {
                    raise(&mut out, l, if r.is_empty() { "deadline" } else { r });
                }
            }
            None if !r.is_empty() => {
                out.needs_date = true;
                if out.reason.is_empty() {
                    out.reason = r.to_string();
                }
            }
            None => {}
        }
    }
    (out.level.is_some() || out.needs_date).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-10-31"; // a Saturday

    fn level(msg: &str) -> Option<Level> {
        assess(&[msg], TODAY).and_then(|u| u.level)
    }

    #[test]
    fn days_to_a_deadline_set_the_level() {
        let u = assess(&["My court date is tomorrow"], TODAY).unwrap();
        assert_eq!((u.level, u.days_left), (Some(Level::Urgent), Some(1)));
        assert_eq!(u.date.as_deref(), Some("2026-11-01"));
        assert_eq!(level("I have a hearing on 11/2"), Some(Level::Urgent));
        assert_eq!(level("The hearing is Nov 10th"), Some(Level::TimeSensitive));
        assert_eq!(level("court date November 20, 2026"), Some(Level::ComingUp));
        assert_eq!(level("court date is in 3 weeks"), Some(Level::ComingUp));
        assert_eq!(level("court date on 2026-12-15"), None, "45 days away");
        assert_eq!(
            level("I got a rent demand, due by Tuesday"),
            Some(Level::Urgent)
        );
        assert_eq!(
            level("the deadline is the 5th of November"),
            Some(Level::TimeSensitive)
        );
        let passed = assess(&["I missed my court date on October 29"], TODAY).unwrap();
        assert_eq!(
            (passed.level, passed.days_left),
            (Some(Level::Urgent), Some(-2))
        );
        // A date with no year that passed long ago is next year's, and 66 days is not urgent.
        let today = parse_iso(TODAY).unwrap();
        assert_eq!(
            dates_in("court date january 5", today),
            [parse_iso("2027-01-05").unwrap()]
        );
        assert!(assess(&["court date January 5"], TODAY).is_none());
    }

    #[test]
    fn a_deadline_with_no_date_asks_for_it() {
        let u = assess(&["My court date is soon, what do I do?"], TODAY).unwrap();
        assert!(u.needs_date && u.level.is_none());
        assert_eq!(u.reason, "court date");
        // The date arrives in a later message: counted then.
        let u = assess(&["My court date is soon", "It's on November 3"], TODAY).unwrap();
        assert_eq!((u.level, u.needs_date), (Some(Level::Urgent), false));
    }

    #[test]
    fn danger_and_emergencies_are_urgent_without_a_date() {
        let u = assess(&["I smell gas in the hallway"], TODAY).unwrap();
        assert!(u.danger);
        assert_eq!(u.level, Some(Level::Urgent));
        assert_eq!(
            level("There's no heat in my apartment"),
            Some(Level::Urgent)
        );
        assert_eq!(level("The marshal left a notice"), Some(Level::Urgent));
        assert_eq!(
            level("peeling paint and my baby crawls there"),
            Some(Level::Urgent)
        );
        assert!(!assess(&["no heat"], TODAY).unwrap().danger);
    }

    #[test]
    fn ordinary_questions_are_not_urgent() {
        for msg in [
            "Is this building safe?",
            "It was built in 1975, any problems?",
            "I'm free tomorrow to see the apartment",
            "What does the law say about peeling paint?",
            "When does heat season start?",
            "What does Good Cause Eviction say about rent increases in NYC?",
            "Write my HP Action petition for me so I can file it tomorrow.",
        ] {
            assert!(assess(&[msg], TODAY).is_none(), "{msg}");
        }
    }

    #[test]
    fn calendar_arithmetic_round_trips() {
        for d in [-1, 0, 59, 365, 20_391, 20_757] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
        assert_eq!(parse_iso("2026-02-29"), None);
        assert!(parse_iso("2028-02-29").is_some());
    }
}
