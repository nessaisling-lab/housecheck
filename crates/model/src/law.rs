//! The law the agent is allowed to state, as it is written in `data/law.json`.
//!
//! Every provision is a verbatim excerpt of an official page somebody actually read, with the
//! URL and the date. The live evals showed why: asked when heat season starts, the agent
//! answered from memory and applied the 55°F outdoor condition to the 62°F night rule too. A
//! claim is only as good as what the agent is handed, so this is what it is handed, and the
//! evals check its answers against it.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Topics `lookup_law` accepts. The tool schema's enum is built from this list and ingest
/// refuses a provision tagged with anything else, so the two cannot drift apart.
pub const TOPICS: [&str; 12] = [
    "heat",
    "hot_water",
    "repairs",
    "habitability",
    "mold",
    "pests",
    "lead_paint",
    "rent_stabilization",
    "security_deposit",
    "eviction",
    "housing_court",
    "general",
];

/// `data/law.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct LawSeed {
    pub sources: Vec<LawSource>,
    pub provisions: Vec<LawProvision>,
    pub legal_help: Vec<LegalHelp>,
}

/// A free tenant legal-services organisation or hotline that `find_legal_help` names.
///
/// Curated rather than web-searched, deliberately: someone asking this question is often in a
/// housing crisis, and an open search for "tenant lawyer" surfaces lead-generation sites and
/// operations that target exactly that desperation. A hallucinated firm is worse than no
/// answer. Every entry is an established nonprofit or a government service, checked against
/// its own published page, never a third-party listing: on 2026-07-26 that caught LawHelpNY
/// listing Housing Court Answers as Tue/Wed/Thu (its own page says Monday to Friday), and on
/// 2026-10-30 the Met Council hotline page HouseCheck linked had become a 404.
///
/// What this does NOT prove: that someone picks up. Nobody dialled these. A stale hotline
/// number for a person with no heat is a real harm, not a broken link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegalHelp {
    pub name: String,
    pub what: String,
    pub phone: Option<String>,
    /// As the page words it, so ingest can find it in `excerpt`.
    pub hours: Option<String>,
    pub url: String,
    pub free: bool,
    /// Verbatim from `url`. Ingest refuses an entry whose phone numbers or hours are not in
    /// it, so a number can't be changed in one place and not the other.
    #[serde(skip_serializing)]
    pub excerpt: String,
    #[serde(skip_serializing)]
    pub retrieved_at: String,
    #[serde(skip_serializing)]
    pub verified_by: String,
    pub verified_on: String,
}

/// The provision id `data/law.json` gives an NYC Administrative Code section.
pub fn admin_code_id(section: &str) -> String {
    format!("NYC-ADMIN-{section}")
}

/// The Administrative Code sections an HPD notice cites in its opening citation:
/// `§ 27-2005, 2007 ADM CODE …` gives `["27-2005", "27-2007"]`, `HMC ADM CODE: § 27-2017.4 …`
/// gives `["27-2017.4"]`. A bare number continues the section before it only after a comma.
///
/// Only titles 26 and 27, the housing titles: notices also cite HPD's rules (`§ 12-06 … RCNY`,
/// `28 RCNY §25-171`) and the Multiple Dwelling Law (`§ 329, M/D LAW`), which are not Admin
/// Code sections.
///
/// ponytail: reads the first 120 characters, where the notices sampled on 2026-10-30 put their
/// citation (50,000 recent open violations, citywide); the order text after it can hold
/// numbers that are not sections.
pub fn hpd_cited_sections(description: &str) -> Vec<String> {
    let head: String = description.chars().take(120).collect();
    let is_num = |s: &str| {
        s.len() >= 3
            && s.starts_with(|c: char| c.is_ascii_digit())
            && s.bytes().all(|c| c.is_ascii_digit() || c == b'.')
    };
    let mut out: Vec<String> = Vec::new();
    let mut title = "";
    let mut continues = false;
    for raw in head.split(|c: char| c.is_whitespace() || c == '§' || c == '(') {
        let comma = raw.ends_with(',');
        let t = raw.trim_end_matches([',', '.', ':', ';']);
        match t.split_once('-') {
            Some((a, b)) if (a == "26" || a == "27") && is_num(b) => {
                title = a;
                out.push(t.to_string());
            }
            None if continues && is_num(t) => out.push(format!("{title}-{t}")),
            _ => {
                continues = false;
                continue;
            }
        }
        continues = comma;
    }
    out.dedup();
    out
}

/// `s` without the separators a phone number is written with, so "(212) 962-4795" and
/// "212-962-4795" both read `2129624795`.
pub fn phone_digits(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, ' ' | '-' | '(' | ')' | '.'))
        .collect()
}

/// Each phone number in `s`: a run of ten or more digits once separators are dropped. "311"
/// alone is too short to count.
pub fn phone_numbers(s: &str) -> Vec<String> {
    phone_digits(s)
        .split(|c: char| !c.is_ascii_digit())
        .filter(|d| d.len() >= 10)
        .map(String::from)
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
pub struct LawSource {
    pub id: String,
    pub publisher: String,
    /// `ny_state`, `nyc` or `federal`. Checked by the schema.
    pub jurisdiction: String,
    /// `statute`, `regulation` or `agency_guidance`. Checked by the schema.
    pub kind: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LawProvision {
    /// Stable id, e.g. `NY-RPL-235-B`.
    pub id: String,
    pub source: String,
    /// As written, e.g. `NY Real Property Law § 235-b`.
    pub citation: String,
    pub title: String,
    /// Verbatim from `url`. Passages that are not adjacent on the page are joined by a line
    /// holding only `…`.
    pub text: String,
    pub url: String,
    pub retrieved_at: String,
    /// Who compared `text` against the official page: `fetch` when a script copied it from a
    /// page that answered a plain request, or the name of the person who checked it by hand
    /// (nyc.gov, nysenate.gov and amlegal refuse scripted requests, and are never bypassed).
    pub verified_by: String,
    pub verified_on: String,
    /// `current` or `superseded`. Only `current` is ever served.
    pub status: String,
    /// `residential`, `commercial` or `both`.
    pub occupancy: String,
    pub topics: Vec<String>,
    #[serde(default)]
    pub facts: Vec<LawFact>,
}

/// One checkable number or condition, and the sentence it was taken from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LawFact {
    /// e.g. `heat.night.min_f`. A key ending in `_f` is a temperature in °F, and the evals
    /// hold every temperature in an answer to one of these.
    pub key: String,
    pub value: String,
    pub condition: String,
    /// Verbatim; must appear in the provision's `text`.
    pub quote: String,
}

/// A provision as `lookup_law` hands it to the model.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServedProvision {
    pub id: String,
    pub citation: String,
    pub title: String,
    pub text: String,
    pub url: String,
    pub publisher: String,
    pub kind: String,
    pub status: String,
    pub retrieved_at: String,
    pub verified_on: String,
    /// Whether `text` is the whole provision. When it is not, `text` holds the passages served,
    /// in order, with a line holding only `…` where passages were left out.
    pub complete: bool,
    /// Characters in the whole provision, so the reader can be told what the full text costs.
    pub full_chars: usize,
}

/// A fact as `lookup_law` hands it to the model, with the provision it came from.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ServedFact {
    pub provision: String,
    #[serde(flatten)]
    pub fact: LawFact,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LawLookup {
    pub provisions: Vec<ServedProvision>,
    pub facts: Vec<ServedFact>,
}

/// Whitespace collapsed to single spaces, trimmed. What the hash and the quote check compare,
/// so a reflowed line on the publisher's page is not mistaken for a changed law.
pub fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// sha256 of the normalized text, hex. Stored per provision so a later fetch can tell a
/// changed page from an unchanged one without a person rereading it.
pub fn text_sha256(text: &str) -> String {
    hex::encode(Sha256::digest(normalize(text).as_bytes()))
}

/// One piece of a provision's text, cut at the provision's own subdivision markers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Passage {
    /// The marker that opens it (`a.`, `(3)`, `* 2.`), or `""` for the opening piece.
    pub label: String,
    pub text: String,
}

/// The subdivision marker a line opens with, if any: `a.`, `a-1.`, `1.`, `1-a.`, `(1)`, `(a)`,
/// `* 2.`, after any indentation, and followed by a space or tab.
fn opening_marker(line: &str) -> Option<&str> {
    let s = line.trim_start_matches([' ', '\t']);
    let body = s
        .strip_prefix('*')
        .map_or(s, |r| r.trim_start_matches([' ', '\t']));
    let b = body.as_bytes();
    let end = if b.first() == Some(&b'(') {
        let close = body.find(')')?;
        let inner = &body[1..close];
        if !((1..=4).contains(&inner.len())
            && inner
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
        {
            return None;
        }
        close + 1
    } else {
        let dot = body.find('.')?;
        let head = &body[..dot];
        let ok = match head.split_once('-') {
            None => {
                (head.len() == 1 && head.as_bytes()[0].is_ascii_lowercase())
                    || (!head.is_empty() && head.bytes().all(|c| c.is_ascii_digit()))
            }
            Some((l, d)) if l.len() == 1 && l.as_bytes()[0].is_ascii_lowercase() => {
                !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit())
            }
            Some((d, l)) if !d.is_empty() && d.bytes().all(|c| c.is_ascii_digit()) => {
                l.len() == 1 && l.as_bytes()[0].is_ascii_lowercase()
            }
            _ => false,
        };
        if !ok {
            return None;
        }
        dot + 1
    };
    matches!(b.get(end), Some(b' ' | b'\t')).then(|| &s[..s.len() - body.len() + end])
}

/// `text` cut at every line that opens a subdivision. Lossless: the pieces joined in order are
/// `text`, byte for byte, and ingest refuses a provision for which that fails.
///
/// ponytail: a wrapped line that happens to start with "a. " or "(2) " mid-sentence is cut
/// too. Harmless (the pieces still rejoin exactly); if it splits a sentence that matters,
/// mark subdivisions at capture instead of inferring them.
pub fn passages(text: &str) -> Vec<Passage> {
    let mut starts = vec![0];
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if at > 0 && opening_marker(line).is_some() {
            starts.push(at);
        }
        at += line.len();
    }
    starts.push(text.len());
    starts
        .windows(2)
        .map(|w| {
            let piece = &text[w[0]..w[1]];
            let label = if w[0] == 0 && opening_marker(piece).is_none() {
                ""
            } else {
                opening_marker(piece).unwrap_or("")
            };
            Passage {
                label: label.trim().to_string(),
                text: piece.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passages_rejoin_exactly_and_label_their_markers() {
        let t = "§ 1 Title.\n\na. The owner shall:\n\n1. paint; and\n(2) repair.\n  * 3. Note.\nb.\tEnd.\n";
        let p = passages(t);
        assert_eq!(p.iter().map(|x| x.text.as_str()).collect::<String>(), t);
        let labels: Vec<&str> = p.iter().map(|x| x.label.as_str()).collect();
        assert_eq!(labels, ["", "a.", "1.", "(2)", "* 3.", "b."]);
        // Not markers: a time, a decimal section number, a word.
        for line in [
            "6 a.m. to midnight",
            "27-2005. text",
            "ab. no",
            "(longer) no",
            "a.no",
        ] {
            assert!(opening_marker(line).is_none(), "{line}");
        }
    }

    /// The real table: every provision splits back into exactly its own text.
    #[test]
    fn every_real_provision_splits_losslessly() {
        let seed: LawSeed =
            serde_json::from_str(include_str!("../../../data/law.json")).expect("law.json");
        for p in &seed.provisions {
            let parts = passages(&p.text);
            assert_eq!(
                parts.iter().map(|x| x.text.as_str()).collect::<String>(),
                p.text,
                "{}",
                p.id
            );
        }
    }

    #[test]
    fn hash_ignores_reflow_but_not_words() {
        let a = text_sha256("at least 62 degrees\nFahrenheit  at all times.");
        assert_eq!(
            a,
            text_sha256(" at least 62 degrees Fahrenheit at all times. ")
        );
        assert_ne!(
            a,
            text_sha256("at least 68 degrees Fahrenheit at all times.")
        );
    }

    /// Openings of real open HPD violations (wvxf-dwi5), read 2026-10-30.
    #[test]
    fn hpd_citations_are_read_as_sections() {
        let cases: [(&str, &[&str]); 7] = [
            (
                "§ 27-2005 ADM CODE PROPERLY REPAIR THE BROKEN OR DEFECTIVE INTERCOM",
                &["27-2005"],
            ),
            (
                "§ 27-2005, 27-2007, 27-2041.1 HMC, §238, § 309; § 107 (2) ( C) MDL AND 28 RCNY §25-171: REPLACE",
                &["27-2005", "27-2007", "27-2041.1"],
            ),
            (
                "§ 27-2026, 2027 HMC: PROPERLY REPAIR THE SOURCE AND ABATE THE EVIDENCE OF A WATER LEAK",
                &["27-2026", "27-2027"],
            ),
            (
                "HMC ADM CODE: § 27-2017.4 ABATE THE INFESTATION CONSISTING OF MICE",
                &["27-2017.4"],
            ),
            (
                "§ 27-2045(B)(1)(B) HMC, § 12-06, § 12-07, § 12-09 RCNY REPAIR, REPLACE OR PROVIDE",
                &["27-2045"],
            ),
            (
                "§ 329, M/D LAW AND DEPT. RULES AND REGS. PROVIDE A COMPLETED CERTIFICATE",
                &[],
            ),
            (
                "§ 27-2010, 2011, 2012 ADM CODE CLEANSE AND DISINFECT",
                &["27-2010", "27-2011", "27-2012"],
            ),
        ];
        for (desc, want) in cases {
            assert_eq!(hpd_cited_sections(desc), want, "{desc}");
        }
    }

    #[test]
    fn phone_numbers_read_through_formatting() {
        assert_eq!(
            phone_numbers("Central 212-577-3300 · Manhattan (212) 426-3000"),
            ["2125773300", "2124263000"]
        );
        assert_eq!(
            phone_numbers("311, or 212-639-9675 from outside NYC"),
            ["2126399675"]
        );
        assert!(phone_numbers("Call 311").is_empty());
    }
}
