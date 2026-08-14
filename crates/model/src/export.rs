//! A building's record as a document a stranger can check.
//!
//! # Why this exists
//!
//! A tenant lawyer can already read a building's violations on the card. What they cannot do
//! is put that reading in front of a court: a printout is unverifiable, and a count they
//! hand-copied is hearsay about hearsay. This turns the record into something opposing
//! counsel can re-check without trusting us.
//!
//! # What it proves, and what it does not
//!
//! Two separate claims, and conflating them would be the whole failure:
//!
//! - **The hash chain proves the document was not altered after we produced it.**
//!   `entry_hash[i] = sha256(entry_hash[i-1] ++ payload_hash[i])`, the same append-only
//!   construction SiteAssure uses for OSHA logs. Change one character of one violation and
//!   every hash from that row onward changes.
//! - **The provenance block is what makes it about the world rather than about the file.**
//!   The chain alone attests that nobody edited *our own output*. It says nothing about
//!   whether that output matched HPD. So each source's dataset id and retrieval timestamp
//!   travel inside the signed region — without them this is an exhibit about itself.
//!
//! And what it cannot prove: that HPD's data was correct, or that the building is safe. It
//! attests to what a named public dataset said at a stated time. That is the honest claim,
//! and it is the one a court can act on.
//!
//! # Signing
//!
//! The chain is always computed. The signature is only added when a signing key is
//! configured, exactly as Resona's licence verification does it — an absent key produces an
//! **unsigned but still chained** document rather than a fake signature. A verifier reports
//! three distinct outcomes: tampered, intact-but-unsigned, intact-and-signed-by-<key>.
//! Collapsing the middle case into "valid" would let an unsigned document pass as an
//! authenticated one.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Building, ViolationDetail};

/// Domain separator, so a chain built here can never be replayed as one built elsewhere.
const GENESIS: &[u8] = b"housecheck.export.v1";

/// One source, and when we read it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceStamp {
    /// The dataset's own identifier, e.g. `wvxf-dwi5` — checkable against NYC Open Data.
    pub dataset: String,
    pub retrieved_at_unix: i64,
    pub row_count: i64,
    /// The query that produced the rows, where one applies.
    pub note: Option<String>,
}

/// One row of the exported record. Kept flat and explicit so the canonical bytes below are
/// obvious rather than emergent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportRow {
    pub class: String,
    pub description: Option<String>,
    pub issued_on: Option<String>,
    pub days_open: Option<i64>,
    /// `sha256` of this row's canonical bytes, hex.
    pub payload_hash: String,
    /// `sha256(prev_entry_hash ++ payload_hash)`, hex. The chain link.
    pub entry_hash: String,
}

/// The document handed to a court, a landlord, or opposing counsel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportDocument {
    pub format: String,
    pub bbl: String,
    pub address: String,
    /// When this document was produced — distinct from when the data was retrieved.
    pub exported_at_unix: i64,
    pub sources: Vec<SourceStamp>,
    pub open_violation_total: u32,
    pub rows: Vec<ExportRow>,
    /// The last `entry_hash`, or the genesis digest for an empty record. This is the value
    /// a signature covers, so one 32-byte comparison settles the whole document.
    pub chain_head: String,
    /// Hex Ed25519 signature over `chain_head`'s bytes, when a key was configured.
    pub signature: Option<String>,
    /// Hex public key the signature verifies against, so a reader knows *which* key to
    /// compare with the published one. Present only alongside a signature.
    pub public_key: Option<String>,
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Canonical bytes for one row.
///
/// Hand-built with an explicit separator rather than serialising to JSON, because a hash is
/// only as stable as its input: JSON key order, whitespace and unicode escaping are all free
/// to change under a serialiser upgrade, and every historical signature would break with no
/// code change. `\x1f` (unit separator) cannot appear in any of these fields.
fn canonical(class: &str, description: Option<&str>, issued: Option<&str>, days: Option<i64>) -> Vec<u8> {
    let mut out = Vec::new();
    for part in [
        class,
        description.unwrap_or(""),
        issued.unwrap_or(""),
        &days.map(|d| d.to_string()).unwrap_or_default(),
    ] {
        out.extend_from_slice(part.as_bytes());
        out.push(0x1f);
    }
    out
}

/// The public half of a signing key, for publishing.
///
/// # Why this has to exist
///
/// A signature inside a document proves nothing on its own. An attacker who wants a building
/// to look clean does not forge our signature — they rewrite a row, **recompute the whole
/// chain**, sign the new head with a keypair they generated, and embed their own public key.
/// Every check inside the document then passes, because it is internally consistent. Verified
/// against the document alone on 2026-08-11: a row rewritten to "NO VIOLATIONS OF ANY KIND AT
/// THIS ADDRESS" verified as **signed and intact**.
///
/// The only thing that stops it is a reader comparing the embedded `public_key` against one
/// published somewhere they already trust. `ExportDocument::public_key`'s own doc comment says
/// "the published one" — this is the function that makes that phrase true.
///
/// Takes the secret and returns only the public half, so the caller never has to touch key
/// material to publish it. An unusable key yields `None` rather than a placeholder: publishing
/// a key that verifies nothing is worse than publishing none.
pub fn public_key_for(secret_key_hex: &str) -> Option<String> {
    let bytes = hex::decode(secret_key_hex.trim()).ok()?;
    let key_bytes: [u8; 32] = bytes.try_into().ok()?;
    let signing = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
    Some(hex::encode(signing.verifying_key().as_bytes()))
}

impl ExportDocument {
    /// Build the document and its chain. Rows are exported in the order given, and that
    /// order is part of what is signed — reordering them changes every hash after the swap.
    pub fn build(
        building: &Building,
        details: &[ViolationDetail],
        open_violation_total: u32,
        sources: Vec<SourceStamp>,
        exported_at_unix: i64,
    ) -> Self {
        let mut prev = hex_digest(GENESIS);
        let mut rows = Vec::with_capacity(details.len());
        for d in details {
            let payload_hash = hex_digest(&canonical(
                &d.class,
                d.description.as_deref(),
                d.issued_on.as_deref(),
                d.days_open,
            ));
            let mut linked = Vec::new();
            linked.extend_from_slice(prev.as_bytes());
            linked.extend_from_slice(payload_hash.as_bytes());
            let entry_hash = hex_digest(&linked);
            rows.push(ExportRow {
                class: d.class.clone(),
                description: d.description.clone(),
                issued_on: d.issued_on.clone(),
                days_open: d.days_open,
                payload_hash,
                entry_hash: entry_hash.clone(),
            });
            prev = entry_hash;
        }
        ExportDocument {
            format: GENESIS_FORMAT.to_string(),
            bbl: building.bbl.clone(),
            address: building.address.clone(),
            exported_at_unix,
            sources,
            open_violation_total,
            rows,
            chain_head: prev,
            signature: None,
            public_key: None,
        }
    }

    /// Sign `chain_head` with a hex Ed25519 secret key.
    ///
    /// An empty or malformed key leaves the document **unsigned rather than falsely signed**.
    /// That is the same fail-closed choice Resona's licence check makes: producing a
    /// signature-shaped value from a missing key would be worse than having none.
    pub fn sign_with(&mut self, secret_key_hex: &str) -> bool {
        let Some(bytes) = hex::decode(secret_key_hex.trim()).ok() else {
            return false;
        };
        let Ok(key_bytes): Result<[u8; 32], _> = bytes.try_into() else {
            return false;
        };
        let signing = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        let sig = ed25519_dalek::Signer::sign(&signing, self.chain_head.as_bytes());
        self.signature = Some(hex::encode(sig.to_bytes()));
        self.public_key = Some(hex::encode(signing.verifying_key().as_bytes()));
        true
    }

    /// Recompute the chain and check the signature. Everything a verifier needs, offline.
    pub fn verify(&self) -> VerifyOutcome {
        let mut prev = hex_digest(GENESIS);
        for (i, row) in self.rows.iter().enumerate() {
            let payload_hash = hex_digest(&canonical(
                &row.class,
                row.description.as_deref(),
                row.issued_on.as_deref(),
                row.days_open,
            ));
            if payload_hash != row.payload_hash {
                return VerifyOutcome::Tampered { row: Some(i), what: "row content" };
            }
            let mut linked = Vec::new();
            linked.extend_from_slice(prev.as_bytes());
            linked.extend_from_slice(payload_hash.as_bytes());
            let entry_hash = hex_digest(&linked);
            if entry_hash != row.entry_hash {
                return VerifyOutcome::Tampered { row: Some(i), what: "chain link" };
            }
            prev = entry_hash;
        }
        if prev != self.chain_head {
            return VerifyOutcome::Tampered { row: None, what: "chain head" };
        }
        match (self.signature.as_deref(), self.public_key.as_deref()) {
            (Some(sig), Some(pk)) => {
                let ok = (|| {
                    let sig: [u8; 64] = hex::decode(sig).ok()?.try_into().ok()?;
                    let pk_bytes: [u8; 32] = hex::decode(pk).ok()?.try_into().ok()?;
                    let vk = ed25519_dalek::VerifyingKey::from_bytes(&pk_bytes).ok()?;
                    let signature = ed25519_dalek::Signature::from_bytes(&sig);
                    ed25519_dalek::Verifier::verify(&vk, self.chain_head.as_bytes(), &signature).ok()
                })()
                .is_some();
                if ok {
                    VerifyOutcome::SignedAndIntact { public_key: pk.to_string() }
                } else {
                    VerifyOutcome::Tampered { row: None, what: "signature" }
                }
            }
            // Intact but unsigned is its own answer. Reporting it as "valid" would let an
            // unsigned document pass for an authenticated one.
            _ => VerifyOutcome::IntactUnsigned,
        }
    }
}


/// Civil date from days since the epoch (Howard Hinnant). Local to this module so the
/// document can stamp itself without the API having to pre-format anything.
fn ymd(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

fn iso(unix: i64) -> String {
    let (y, m, d) = ymd(unix.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

/// Render HPD's stored text for a human, without touching the bytes a signature covers.
///
/// HPD publishes `U+001A` (SUBSTITUTE) where an apostrophe belongs. Measured across the
/// whole artifact on 2026-08-12: **890 occurrences in 169 of 202 description blocks** — 84%
/// of covered buildings, not the single building first recorded. Every observed instance is
/// a possessive: `HPD'S` (640), `AGENCY'S` (158), `TENANTS'` (72), `BUILDING'S` (19).
/// Confirmed identical in HPD's own `wvxf-dwi5` API, so the ingest is faithful and this is
/// the city's data.
///
/// **Why this is not fixed at ingest.** The chain hashes the description exactly as it was
/// retrieved. Normalising on the way in would make the signed bytes stop matching the
/// source, which converts a faithful record into a tidied one — and the entire value of the
/// export is that it is not tidied. So the substitution happens here, at the boundary where
/// text is shown to a person, and the transcript says that it happened.
///
/// Other C0 control characters are dropped rather than substituted. In HTML a stray control
/// byte is an invisible no-op; in a PDF text stream it is not, which is why this blocks the
/// PDF work until it exists.
/// An address a person can act on, or a statement that there isn't one.
///
/// Measured on the live artifact 2026-08-12: **5 of 250 buildings** have an address with no
/// house number, and `3015097501` has the empty string. PLUTO records the lot; it does not
/// always record a street number for it.
///
/// The empty one rendered as an empty heading — a card with a blank where the building's
/// name goes, which reads as a rendering bug rather than as missing data. It is also
/// unreachable by address search, because an empty haystack never contains a non-empty
/// needle. Both are the same underlying fact and it should be stated, not papered over.
/// Callers append the BBL themselves — every surface that shows an address already shows
/// the identifier next to it, and folding it in here produced "· BBL x · BBL x".
pub fn display_address(address: &str) -> std::borrow::Cow<'_, str> {
    let a = address.trim();
    if a.is_empty() {
        std::borrow::Cow::Borrowed("Address not recorded")
    } else if !a.starts_with(|c: char| c.is_ascii_digit()) {
        // A street with no number: real, locatable to a block, not to a door.
        std::borrow::Cow::Owned(format!("{a} (no house number on record)"))
    } else {
        std::borrow::Cow::Borrowed(a)
    }
}

pub fn for_display(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.bytes().any(|b| b < 0x20 && b != b'\t' && b != b'\n') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\u{1A}' => out.push('\''),
            '\t' | '\n' => out.push(ch),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    std::borrow::Cow::Owned(out)
}

impl ExportDocument {
    /// A transcript for pasting into a filing.
    ///
    /// **This text is not the verifiable artifact and says so in its own footer.** The JSON
    /// document is what `verify` recomputes; a paragraph of prose cannot carry a hash chain
    /// through a copy-paste without becoming unreadable. What the transcript carries instead
    /// is the record hash, so a reader holding both can confirm they describe the same
    /// export. Claiming more than that would be exactly the kind of unearned assurance this
    /// feature exists to avoid.
    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("HOUSECHECK RECORD - {}
", self.address));
        out.push_str(&format!("BBL {} - exported {}

", self.bbl, iso(self.exported_at_unix)));

        let hazardous = self.rows.iter().filter(|r| r.class.eq_ignore_ascii_case("C")).count();
        out.push_str(&format!(
            "{} open HPD violation{} on record. {} immediately hazardous (Class C).

",
            self.open_violation_total,
            if self.open_violation_total == 1 { "" } else { "s" },
            hazardous
        ));

        for (i, r) in self.rows.iter().enumerate() {
            let age = match r.days_open {
                Some(d) => format!("open {d} day{}", if d == 1 { "" } else { "s" }),
                None => "age unknown (no issue date on record)".to_string(),
            };
            out.push_str(&format!(
                "{:>3}. Class {} - issued {} - {}
     {}
",
                i + 1,
                r.class,
                r.issued_on.as_deref().unwrap_or("date not recorded"),
                age,
                r.description
                    .as_deref()
                    .map(for_display)
                    .unwrap_or(std::borrow::Cow::Borrowed("(HPD recorded no description)"))
            ));
        }
        if (self.rows.len() as u32) < self.open_violation_total {
            out.push_str(&format!(
                "
     ... showing {} of {} open violations.
",
                self.rows.len(),
                self.open_violation_total
            ));
        }

        // Say it when it happened, and only then. A reader comparing this transcript with
        // the JSON will find one byte per apostrophe that does not match, and the honest
        // move is to tell them why rather than let them discover it and wonder what else
        // was quietly adjusted.
        if self
            .rows
            .iter()
            .filter_map(|r| r.description.as_deref())
            .any(|d| d.contains('\u{1A}'))
        {
            out.push_str(
                "
  Note: HPD publishes a control character where an apostrophe belongs. It is shown
  above as an apostrophe. The signed document keeps HPD's bytes exactly as retrieved,
  so this transcript and the JSON differ by that one character per occurrence.
",
            );
        }

        out.push_str("
SOURCES
");
        for s in &self.sources {
            out.push_str(&format!(
                "  {:<12} {:>9} rows  retrieved {}
",
                s.dataset,
                s.row_count,
                iso(s.retrieved_at_unix)
            ));
        }

        out.push_str("
VERIFICATION
");
        out.push_str(&format!("  Record hash: {}
", self.chain_head));
        match self.public_key.as_deref() {
            Some(pk) => {
                out.push_str(&format!("  Signed by:   {pk}\n"));
                // Without this instruction the signature is decorative. A forger rewrites a
                // row, recomputes the chain and signs it with their own keypair; the document
                // then verifies perfectly against the key it carries. Comparing that key with
                // one published independently is the only step that catches it, so the
                // document has to ask for it in the same breath as it makes the claim.
                out.push_str(
                    "  Check that key against the one published at the issuer's /meta\n\
                     \x20 endpoint before relying on this signature. A document signed by an\n\
                     \x20 unknown key proves only that it is consistent with itself.\n",
                );
            }
            None => out.push_str("  Unsigned (hash-chained, but not attributed to an issuer).\n"),
        }
        out.push_str("  This text is a transcript. The verifiable document is the JSON
");
        out.push_str("  export of the same record, which anyone can check offline without
");
        out.push_str("  contacting HouseCheck.
");
        out
    }
}

const GENESIS_FORMAT: &str = "housecheck.export.v1";

/// Three outcomes, deliberately not two.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum VerifyOutcome {
    /// Chain recomputes and the signature checks out against the embedded key.
    SignedAndIntact { public_key: String },
    /// Chain recomputes; no signature was attached.
    IntactUnsigned,
    /// Something does not recompute. `row` locates it where a row is at fault.
    Tampered { row: Option<usize>, what: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn building() -> Building {
        Building {
            bbl: "3016440063".into(),
            address: "603 PUTNAM AVENUE".into(),
            year_built: 1899,
            num_floors: 3,
            units_res: 3,
            tract_geoid: "36047027700".into(),
            rent_stabilized: None,
            rent_stab_units: None,
            good_cause: false,
            has_elevator: false,
            near_ada_subway_m: Some(897),
            complaints_311: 1779,
            latitude: None,
            longitude: None,
            restaurant_grade: None,
        }
    }

    fn details() -> Vec<ViolationDetail> {
        vec![
            ViolationDetail {
                class: "C".into(),
                description: Some("§ 27-2033 ADM CODE PROVIDE ADEQUATE HEAT".into()),
                issued_on: Some("2026-03-14".into()),
                days_open: Some(148),
            },
            ViolationDetail {
                class: "B".into(),
                description: None,
                issued_on: None,
                days_open: None,
            },
        ]
    }

    fn doc() -> ExportDocument {
        ExportDocument::build(
            &building(),
            &details(),
            33,
            vec![SourceStamp {
                dataset: "wvxf-dwi5".into(),
                retrieved_at_unix: 1_786_321_428,
                row_count: 26_343,
                note: None,
            }],
            1_786_400_000,
        )
    }

    /// A key derived from a secret must match what signing actually produces.
    ///
    /// Two code paths reach `ed25519_dalek` — `sign_with` embeds the public half in the
    /// document, `public_key_for` hands it to `/meta` for publishing — and if they ever
    /// disagree, every reader who follows the transcript's instruction to compare them sees a
    /// mismatch and concludes the record is forged. A false alarm on a tamper check is not a
    /// smaller failure than a missed one; it destroys the same trust.
    #[test]
    fn the_published_key_is_the_one_the_signature_carries() {
        // A fixed key, so this asserts agreement rather than merely self-consistency.
        let secret = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
        let mut d = doc();
        assert!(d.sign_with(secret), "fixture key must be usable");
        assert_eq!(
            d.public_key.as_deref(),
            public_key_for(secret).as_deref(),
            "the key published at /meta and the key inside the document have diverged"
        );
        // RFC 8032 test vector 1: this exact secret has this exact public key. Pins the pair
        // against a value from outside this codebase, so an ed25519 upgrade that changed the
        // derivation could not pass by agreeing with itself.
        assert_eq!(
            public_key_for(secret).as_deref(),
            Some("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a")
        );
    }

    #[test]
    fn an_unusable_secret_publishes_no_key_rather_than_a_placeholder() {
        assert_eq!(public_key_for(""), None);
        assert_eq!(public_key_for("not hex"), None);
        assert_eq!(public_key_for("aabb"), None, "wrong length must not be padded");
    }

    /// The transcript is the version most likely to be read by someone who never sees the
    /// JSON, so its honesty matters more than its formatting. It must carry the record hash,
    /// must not claim to be the verifiable artifact, and must never print a confident zero
    /// for an age it does not know.
    #[test]
    fn the_transcript_is_honest_about_what_it_is() {
        let t = doc().to_plain_text();

        assert!(t.contains("603 PUTNAM AVENUE"));
        assert!(t.contains("3016440063"));
        // The hash is what ties this paper to the checkable document.
        assert!(t.contains(&doc().chain_head), "record hash must appear");
        assert!(t.contains("transcript"), "must not pass itself off as the verifiable artifact");
        assert!(t.contains("wvxf-dwi5"), "sources must travel with the record");

        // Row two has no issue date; it must read as unknown, never as zero days.
        assert!(t.contains("age unknown"), "missing age must say so");
        assert!(!t.contains("open 0 days"));

        // Unsigned documents must say they are unsigned rather than staying quiet.
        assert!(t.contains("Unsigned"));

        let mut signed = doc();
        signed.sign_with(&hex::encode([7u8; 32]));
        assert!(signed.to_plain_text().contains("Signed by:"));
    }

    /// A capped list must say what it is a slice of, in the transcript too — the paper copy
    /// is exactly where a truncated list would otherwise read as the whole story.
    #[test]
    fn the_transcript_states_when_it_is_truncated() {
        let mut d = doc();
        d.open_violation_total = 754;
        let t = d.to_plain_text();
        assert!(t.contains("754"), "the true total must appear");
        assert!(t.contains("showing 2 of 754"));
    }

    #[test]
    fn a_clean_document_verifies() {
        assert_eq!(doc().verify(), VerifyOutcome::IntactUnsigned);
    }

    /// The demo moment, and the whole point of the feature: change one character and the
    /// document stops verifying.
    #[test]
    fn one_edited_character_is_detected() {
        let mut d = doc();
        d.rows[0].description = Some("§ 27-2033 ADM CODE PROVIDE ADEQUATE HEXT".into());
        assert!(matches!(d.verify(), VerifyOutcome::Tampered { row: Some(0), .. }));
    }

    /// Recomputing the hashes to match the edit must not rescue it: the chain head still
    /// disagrees. This is what makes it a chain rather than a checksum.
    #[test]
    fn recomputing_a_rows_own_hash_does_not_rescue_it() {
        let mut d = doc();
        d.rows[0].description = Some("ALTERED".into());
        d.rows[0].payload_hash = hex_digest(&canonical("C", Some("ALTERED"), Some("2026-03-14"), Some(148)));
        // Row 0 now self-consistent, but its entry_hash — and everything after — is not.
        assert!(matches!(d.verify(), VerifyOutcome::Tampered { .. }));
    }

    /// Reordering rows changes the record's meaning, so it must break the chain too.
    #[test]
    fn reordering_rows_breaks_the_chain() {
        let mut d = doc();
        d.rows.swap(0, 1);
        assert!(matches!(d.verify(), VerifyOutcome::Tampered { .. }));
    }

    /// Deleting the inconvenient violation is the most likely real-world tampering.
    #[test]
    fn deleting_a_row_breaks_the_chain() {
        let mut d = doc();
        d.rows.remove(0);
        assert!(matches!(d.verify(), VerifyOutcome::Tampered { .. }));
    }

    #[test]
    fn signing_round_trips_and_a_wrong_key_fails() {
        let mut d = doc();
        let secret = hex::encode([7u8; 32]);
        assert!(d.sign_with(&secret));
        match d.verify() {
            VerifyOutcome::SignedAndIntact { public_key } => {
                assert_eq!(Some(public_key.as_str()), d.public_key.as_deref());
            }
            other => panic!("expected a signed document, got {other:?}"),
        }

        // A signature over a different chain head must not verify.
        let mut e = doc();
        e.sign_with(&secret);
        e.chain_head = hex_digest(b"something else");
        assert!(matches!(e.verify(), VerifyOutcome::Tampered { .. }));
    }

    /// An absent or malformed key must leave the document unsigned, never falsely signed.
    #[test]
    fn a_missing_key_produces_no_signature_rather_than_a_fake_one() {
        for bad in ["", "not-hex", "00ff"] {
            let mut d = doc();
            assert!(!d.sign_with(bad), "{bad:?} should not sign");
            assert_eq!(d.signature, None);
            assert_eq!(d.verify(), VerifyOutcome::IntactUnsigned);
        }
    }

    /// Two exports of the same record must produce the same chain, or a lawyer comparing
    /// two copies would see a difference that means nothing.
    #[test]
    fn the_chain_is_deterministic() {
        assert_eq!(doc().chain_head, doc().chain_head);
        assert_eq!(doc().rows[0].entry_hash, doc().rows[0].entry_hash);
    }

    /// HPD's substitute character stands in for an apostrophe. Measured 2026-08-12: 890
    /// occurrences across 169 of 202 description blocks.
    #[test]
    fn hpds_control_character_renders_as_the_apostrophe_it_stands_for() {
        assert_eq!(for_display("DESCRIBED ON HPD\u{1A}S WEBSITE"), "DESCRIBED ON HPD'S WEBSITE");
        assert_eq!(for_display("THE AGENCY\u{1A}S HOUSING INFO"), "THE AGENCY'S HOUSING INFO");
    }

    /// Measured on the artifact 2026-08-12: 5 of 250 have no house number, and
    /// `3015097501` has the empty string, which rendered as an empty heading.
    #[test]
    fn a_building_with_no_address_says_so_rather_than_rendering_blank() {
        assert_eq!(
            display_address(""), "Address not recorded"
        );
    }

    /// A street with no number is real and locatable to a block, not to a door. Saying
    /// "FULTON STREET" alone would imply a precision the record does not have.
    #[test]
    fn a_street_without_a_number_is_labelled_rather_than_shown_bare() {
        assert_eq!(
            display_address("FULTON STREET"), "FULTON STREET (no house number on record)"
        );
    }

    #[test]
    fn a_normal_address_passes_through_untouched() {
        assert_eq!(display_address("603 PUTNAM AVENUE"), "603 PUTNAM AVENUE");
    }

    /// A stray control byte is an invisible no-op in HTML and is not one in a PDF text
    /// stream, which is why it is removed rather than passed through.
    #[test]
    fn other_control_characters_are_dropped_but_tabs_and_newlines_survive() {
        assert_eq!(for_display("A\u{0}B\u{7}C"), "ABC");
        assert_eq!(for_display("A\tB\nC"), "A\tB\nC");
    }

    /// The common case must not allocate — 33 of 202 blocks are already clean.
    #[test]
    fn clean_text_is_borrowed_rather_than_copied() {
        assert!(matches!(
            for_display("MEND THE BROKEN PLASTERED SURFACES"),
            std::borrow::Cow::Borrowed(_)
        ));
    }

    /// **The constraint this whole fix exists under.** Display cleaning must never reach
    /// the bytes a signature covers, or a faithful record silently becomes a tidied one.
    #[test]
    fn cleaning_for_display_does_not_change_what_was_signed() {
        let mut d = doc();
        d.rows[0].description = Some("DESCRIBED ON HPD\u{1A}S WEBSITE".into());
        let rebuilt = ExportDocument::build(
            &building(),
            &[ViolationDetail {
                class: d.rows[0].class.clone(),
                description: Some("DESCRIBED ON HPD\u{1A}S WEBSITE".into()),
                issued_on: d.rows[0].issued_on.clone(),
                days_open: d.rows[0].days_open,
            }],
            1,
            vec![],
            0,
        );
        let transcript = rebuilt.to_plain_text();
        assert!(transcript.contains("HPD'S WEBSITE"), "transcript should read cleanly");
        assert!(
            rebuilt.rows[0].description.as_deref().unwrap().contains('\u{1A}'),
            "the stored description must keep HPD's byte"
        );
        assert!(transcript.contains("keeps HPD's bytes exactly as retrieved"));
    }

    /// An empty record still has a well-defined head, so a building with nothing open
    /// produces a document that verifies rather than one that cannot be checked.
    #[test]
    fn an_empty_record_still_chains() {
        let d = ExportDocument::build(&building(), &[], 0, vec![], 1);
        assert_eq!(d.chain_head, hex_digest(GENESIS));
        assert_eq!(d.verify(), VerifyOutcome::IntactUnsigned);
    }
}
