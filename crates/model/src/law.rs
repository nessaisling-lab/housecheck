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
pub const TOPICS: [&str; 8] = [
    "heat",
    "hot_water",
    "repairs",
    "habitability",
    "mold",
    "pests",
    "rent_stabilization",
    "general",
];

/// `data/law.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct LawSeed {
    pub sources: Vec<LawSource>,
    pub provisions: Vec<LawProvision>,
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
