// Rendering NYC's data for a person, without pretending it is cleaner than it is.
//
// Mirrors `model::export::{for_display, display_address}` in the Rust crate. Both exist
// because the same two defects show up wherever this data is displayed, and the fix has to
// be at the render boundary: the export's hash chain covers HPD's bytes exactly as
// retrieved, so normalising anything earlier would turn a faithful record into a tidied
// one — and being untidied is the whole point of the export.
//
// If you change the rules here, change them in `crates/model/src/export.rs` too. The two
// are deliberately duplicated across a language boundary rather than shared, because
// shipping WASM to fix two string functions costs more than it saves.
//
// Escape sequences throughout, never literal control bytes — a literal U+001A in a source
// file is invisible in every editor and survives exactly one careless copy-paste.

/** HPD's SUBSTITUTE character, which stands in for an apostrophe. */
const SUB = "\u001A";

/** C0 controls worth removing: everything below 0x20 except tab and newline. */
// eslint-disable-next-line no-control-regex
const C0 = /[\u0000-\u0008\u000B-\u001F]/g;

/**
 * Render HPD violation text.
 *
 * HPD publishes `U+001A` where an apostrophe belongs, so `HPD'S WEBSITE` arrives as
 * `HPDS WEBSITE` and renders in HTML as `HPDS WEBSITE` — the character is invisible,
 * so the text silently loses punctuation rather than showing a placeholder.
 *
 * Measured across the whole artifact on 2026-08-12: **890 occurrences in 169 of 202
 * description blocks**, 84% of covered buildings. Every instance is a possessive:
 * `HPD'S` (640), `AGENCY'S` (158), `TENANTS'` (72), `BUILDING'S` (19). Confirmed identical
 * in HPD's own `wvxf-dwi5` API — the ingest is faithful and this is the city's data.
 *
 * Other C0 controls are dropped rather than substituted. They are invisible no-ops in HTML
 * but not in a PDF text stream, and this text gets copied out of the page.
 */
export function forDisplay(s: string): string {
  if (!s.includes(SUB) && !C0.test(s)) {
    C0.lastIndex = 0; // `g` regexes are stateful across .test() calls
    return s;
  }
  C0.lastIndex = 0;
  return s.split(SUB).join("'").replace(C0, "");
}

/**
 * An address a person can act on, or a statement that there isn't one.
 *
 * Measured on the live artifact 2026-08-12: **5 of 250 buildings** have an address with no
 * house number, and `3015097501` has the empty string. PLUTO records the lot; it does not
 * always record a street number for it.
 *
 * The empty one rendered as a blank heading — which reads as a broken page rather than as
 * missing data. That building has **96 open violations, 12 of them Class C**, so the
 * worst-documented building in the pilot set was the one that looked like a rendering bug.
 *
 * The caller appends the BBL. Every surface that shows an address already shows the
 * identifier beside it.
 */
export function displayAddress(address: string | null | undefined): string {
  const a = (address ?? "").trim();
  if (a === "") return "Address not recorded";
  // A street with no number is real and locatable to a block, not to a door. Showing it
  // bare would imply a precision the record does not have.
  if (!/^\d/.test(a)) return `${a} (no house number on record)`;
  return a;
}
