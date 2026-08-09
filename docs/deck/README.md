# The deck

`HouseCheck-Presentation.html` is the presentation, and **it is the artifact of record** —
a living document held to the same standard as the case study and the classwork notes. It is
committed here rather than living in a Downloads folder so that a claim on a slide can be
diffed, corrected and traced like any other claim in this repository.

15 slides, self-contained, no network. Open it directly, or serve the folder:

```bash
python -m http.server 8931 --directory docs/deck
```

## Edit the HTML directly. The builder cannot regenerate it.

`fill_deck.py` is kept for reference only. It needs two directories that are **no longer on
disk** — `shots/b64.json` (screenshots, base64-encoded) and `targets/*.txt` (the exact source
strings each patch searches for) — so it fails at `json.load(open("shots/b64.json"))`.

That makes the compiled HTML the source of truth, not a build output. Edits are made against
it in place, with a uniqueness assertion on the search string:

```python
assert s.count(old) == 1, "expected exactly one occurrence, found %d" % s.count(old)
```

One edit remains staged in `fill_deck.py` and **not** in the built file — a sentence on the
provenance slide about the database carrying its own provenance and the card printing its
class I exclusion. It cannot be applied until the assets exist.

## Change log

- **2026-08-09** — Condition source card on *We Show Our Work* now reads: *"The score counts
  every class, so a building can show no hazardous violations and still score low on volume
  alone."* Added because the card beside it shows `Condition 1` next to *"No hazardous
  violations"*, which reads as a contradiction and is not one. Verified against the live API:
  603 Putnam Avenue scores `condition 0` with 11 Class A, 22 Class B and **zero** Class C.
  Both numbers were always correct; the sentence reconciling them was missing. See
  `docs/classwork/problem-definition-notes.md`.

## Two traps, both hit more than once

## Two traps, both hit more than once

**Precompiled Tailwind.** The bundle ships a fixed set of ~899 classes. A class that is not
in that set fails *silently* — no error, no visible style. Validate every class in a changed
slide region against the bundle after each build.

**Patch anchors are exact strings.** Editing the source text that a `sub()` call searches for
breaks that patch, and the build reports `MISS` rather than failing. If you change text that
a patch targets, change the patch's replacement instead — the anchor has to stay byte-identical.
