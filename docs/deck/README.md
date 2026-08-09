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

## Responsive behaviour

The deck was built as a fixed 16:9 stage — every slide is `w-screen h-screen overflow-hidden`
with no transform anywhere — so on a narrow screen content was not scaled down, it was
**clipped**. `mobile_layer.py` injects a CSS layer with three tiers:

| Width | Behaviour |
|---|---|
| **≤ 860px** | Reflows. Grids and rows stack to one column, type steps down, the two 64px edge overlays become a bottom **Back / Next** bar, dots go fixed with enlarged hit areas. |
| **861–1180px** | Keeps the multi-column design — four-across is still right on an iPad — but the stage grows and the page scrolls instead of clipping. |
| **> 1180px, height ≤ 700px** | Same anti-clip treatment, for a desktop browser with toolbars eating the height. |
| **Everything else** | Untouched. The media queries simply do not match. |

**Why reflow and not a scale transform.** Scaling the 1280×720 stage to fit preserves the design
exactly and can never clip, and it is wrong here: at 390px that is roughly a 30% scale and body
text lands near 5px. This is going to be hosted for people to *read*.

**Why raw CSS rather than Tailwind classes.** The bundle ships a fixed precompiled set, so an
absent class fails silently — which has bitten this deck twice. Hand-written CSS overrides the
utilities by specificity and does not care what Tailwind compiled.

**Navigation on a phone.** There is no swipe handler in the bundle; the `touchstart`/`touchend`
hits in the source are React's internal event registry, not app code. So the existing edge
overlays are re-anchored into a real bottom bar — same click handlers, no JS change — and the
6px dots get an invisible `::after` hit area, because 6px is not a thumb target.

`mobile_layer.py` is idempotent: it strips its previous block before injecting, so it can be
re-run while iterating.

## Change log

- **2026-08-09** — Mobile support added (`mobile_layer.py`, 8.2 KB of CSS). Verified at
  **360×740, 390×844, 1024×768, 1280×720 and 1440×900**: no horizontal scroll at any size, no
  unreachable content, no text under 11px, all 18 links intact, Back/Next and dots both
  navigating, and desktop provably unchanged (media query inactive, stage still `720px` /
  `overflow:hidden`).

  One regression caught during the work and worth recording: a blanket
  `svg { height:auto; max-width:100% }` collapsed a 30px inline icon to **0×0**, because an SVG
  sized by `width`/`height` attributes does not survive being told its height is auto. The rule
  was removed — the only wide decorative SVG is the 180px closing wordmark, which already fits a
  390px screen, so it was guarding against nothing.


- **2026-08-09** — Slides 8 and 20 trimmed to fit **1280×720**, via `trim_slides.py`. Both
  clipped before any of this session's work; neither was one of the rebuilt slides. **No
  sentence was removed** — the brief was to fill the deck out, so the fix is spacing and
  decorative scale only.

  | | Before | After |
  |---|---:|---:|
  | 8 · We Show Our Work | 71px over | fits |
  | 20 · close | 89px over | fits |

  Two things worth remembering from this:

  - Slide 8's screenshot carried `width:100%` with **no height cap**, so its intrinsic aspect
    ratio set the row height and nothing bounded it. Capped at 250px with `objectFit:contain`.
  - Slide 20 is a `justify-center` stack, so **trimming its padding did nothing** — once content
    is taller than the viewport it simply overflows both ends equally. Only reducing real
    content height moved it, which is why the wordmark went 320 → 180px.

  Verified at both 1280×720 and 1440×900: zero clipped text on all 20 slides, 18 links intact,
  no horizontal scroll, ArrowRight walks 1→20 and ends on the closing slide.


- **2026-08-09** — Seven sparsest slides rebuilt with real detail and citations, via
  `densify_slides.py`. Word counts roughly tripled:

  | Slide | Before | After |
  |---|---:|---:|
  | 4 · The Change | 70 | 231 |
  | 5 · HouseCheck | 77 | 224 |
  | 6 · The Evidence | 85 | 177 |
  | 10 · Under the Hood | 81 | 245 |
  | 12 · Guardrails | 76 | 239 |
  | 14 · Live Now | **45** | 227 |
  | 15 · What It Makes Possible | 88 | 234 |

  **18 outbound links added**, all `https`, all opening in a new tab. Only URLs verified in
  `industry-research-notes.md` or checked live this session are used; `data.cityofnewyork.us/d/<id>`
  is Socrata's stable short form. Statutes are cited by section without a link rather than
  guessing a URL.

  **Links use inline styles, not classes.** The bundle contains zero `<a>` elements, so no anchor
  styling is compiled and any link class would silently do nothing.

  Method: new components, registry re-pointed at them. The originals stay as dead code, so a
  revert is a one-line registry change and no neighbouring minified function can be corrupted.

  Verified at 1440×900: zero clipped text across all 20 slides, 18/18 links well-formed and
  underlined, no horizontal scroll, nav still walks 1→20. At 1280×720 three slides clip —
  8, 9 and 20 — **all pre-existing and none of them rebuilt here.**


- **2026-08-09** — Three audience slides added at positions **16–18**, between *What It Makes
  Possible* and *Market*, taking the deck from 17 slides to 20. The deck covered the renter and
  said nothing about the professional user the product is now designed for.
  - **Who It's For** — both audiences side by side, on the axis that separates them: the renter
    acts once every few years (33,210 units in market, ~1 in 75), the professional twice a day.
  - **The Daily User** — the housing attorney in detail: who, when, what breaks, what they do
    instead. Carries the committed problem statement.
  - **The Bet** — why design for the group with the least money, including the part that does
    not work yet (they are grant-funded and cannot pay).

  Added with `scratchpad/add_slides.py`, which **extracts every class it is about to emit and
  checks each against the compiled CSS before writing a byte.** That guard caught the real trap
  immediately: `grid-cols-2` and `grid-cols-3` are **not in this bundle** — only `grid-cols-4`
  is — so anything that is not four-across uses `flex-1`, the way the Integrity slide does.
  Using a two-column grid here would have produced a stacked, unstyled slide and no error.

  Verified rendered: 20 dots, correct order, no console errors, accent resolving to
  `rgb(29,106,83)` (= `cD`, so the styles are live rather than silently dropped), zero clipped
  cards, no page or horizontal overflow.

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
