# Deck builder

`fill_deck.py` patches the compiled React presentation bundle in place. It is kept here
because it previously lived only in a session scratch directory, and that directory was
cleaned — taking its build assets with it.

## It does not currently run

The script needs two directories that are **no longer on disk**:

- `shots/b64.json` — the screenshots, base64-encoded
- `targets/*.txt` — the exact source strings each patch searches for

Without them it fails at `json.load(open("shots/b64.json"))`. The last successfully built
deck is `~/Downloads/HouseCheck-Presentation-filled.html`, which is current as of the
ingest-truncation correction: it carries 26,306 violations, the 134,837-row finding, the
69.5 → 63.0 fleet mean, and the 72-of-250 band change.

One edit is staged in the source and **not** in that built file — a sentence on the
provenance slide noting that the database now carries its own provenance and that the card
prints its class I exclusion. It will apply the next time the assets exist.

## Two traps, both hit more than once

**Precompiled Tailwind.** The bundle ships a fixed set of ~899 classes. A class that is not
in that set fails *silently* — no error, no visible style. Validate every class in a changed
slide region against the bundle after each build.

**Patch anchors are exact strings.** Editing the source text that a `sub()` call searches for
breaks that patch, and the build reports `MISS` rather than failing. If you change text that
a patch targets, change the patch's replacement instead — the anchor has to stay byte-identical.
