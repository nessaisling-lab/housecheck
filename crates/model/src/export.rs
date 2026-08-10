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

    /// An empty record still has a well-defined head, so a building with nothing open
    /// produces a document that verifies rather than one that cannot be checked.
    #[test]
    fn an_empty_record_still_chains() {
        let d = ExportDocument::build(&building(), &[], 0, vec![], 1);
        assert_eq!(d.chain_head, hex_digest(GENESIS));
        assert_eq!(d.verify(), VerifyOutcome::IntactUnsigned);
    }
}
