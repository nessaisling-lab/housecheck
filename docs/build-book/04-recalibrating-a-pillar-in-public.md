# Chapter 4 — Recalibrating a Pillar in Public

> **The question this chapter answers:** How do you find out that a scoring rule
> is lying, and what does fixing it actually cost?

---

## 1. The rule that was there first

One line (`crates/scoring/src/lib.rs`, before commit `2bc7851`):

```rust
(100 - (complaints_311 * 2).min(60)).clamp(0, 100) as u8
```

Two points off per 311 complaint within 150 metres, penalty capped at 60. It is
readable, it is obviously bounded, and every reviewer's eye passes straight over
it. It is also the worst function that has ever been in this repository, and the
reason is arithmetic: the cap binds at **30 complaints**.

Thirty. In a dense American city, on a block, over a two-year window.

## 2. What it did to the actual data

The shipped artifact has 250 buildings. Their 311 counts:

```
min = 67    median = 440    mean = 445    max = 977
```

The **minimum** is 67. The cap binds at 30.

So under the old rule, the neighborhood pillar produced this many distinct values
across the entire dataset:

**One.**

Every building scored exactly 40. Not "most" — all 250. Fifteen percent of the
headline number was a constant that no input could move, wearing the interface of
a measurement. The pillar had a name, a doc comment, a data source, a Socrata
dataset ID, an ingest step that computed haversine distances for every complaint
in an 8.6 km² box, and a card on the front end. It carried none of that
information into the score. It was `+6.0` for everybody.

This is worth sitting with, because it is not a bug in the sense of a crash or a
wrong branch. Every line did exactly what it said. The failure was that nobody
multiplied `2 × 30` and compared it to the data they were about to feed it.

## 3. The test was green the whole time

Here is the part that should bother a reviewer more than the rule itself. The old
test suite contained this assertion:

```rust
assert_eq!(neighborhood_score(100), 40); // capped penalty at 60
```

Read the comment. It is not wrong. It is not stale. It correctly describes the
intended behaviour of the code, it was written deliberately, and it passed.

A test that pins a saturated output *documents* the saturation. It converts the
defect into a specification. Any future change that fixed the pillar would have
broken that assertion, and a developer looking at a red test with the comment
"capped penalty at 60" would very reasonably have concluded they had broken
something.

That is the failure mode: **the test was not insufficient, it was load-bearing in
the wrong direction.** Chapter 3 found a test that constrains too little
(forty-six weight vectors satisfy it). This one constrains the wrong thing with
full precision.

The general form — and this is the transferable lesson — is that a test written
by asking "what does this code do?" will pass forever and tell you nothing. A test
has to be written by asking "what would make this number useless?" The two
questions produce identical-looking assertions and completely different suites.

## 4. The fix

Commit `2bc7851` replaced it (`:53-59`):

```rust
pub fn neighborhood_score(complaints_311: i32) -> u8 {
    let c = complaints_311.max(0) as f64;
    let penalty = (((1.0 + c).ln() - 4.0).max(0.0) * 20.0)
        .round()
        .clamp(0.0, 60.0);
    (100.0 - penalty) as u8
}
```

Four lines of body, fifteen lines of rationale above it. The log is the right
instinct: complaint counts are heavy-tailed, and a linear penalty on a heavy-tailed
input will always saturate. `- 4.0` is a free allowance so an ordinary busy block
isn't punished. `* 20.0` is the slope.

Measured on the same 250 buildings:

| | old rule | new rule |
|---|---|---|
| distinct values produced | **1** | **38** |
| buildings at the floor | 250 (100%) | 0 (0%) |
| score range | 40..40 | 42..96 |
| contribution to the headline | 6.0 flat | 6.3 .. 14.4 |

The pillar went from contributing zero bits of information to spreading eight
points of the headline score. As a fix, it worked. It is the single largest
improvement in the product's accuracy that anyone made, and it was four lines.

And a regression test was added that encodes the *failure*, not the behaviour:

```rust
fn neighborhood_discriminates_dense_blocks() {
    assert_ne!(neighborhood_score(100), neighborhood_score(500));
```

`assert_ne!`. That is the corrected question from §3, written down. Whatever the
curve does later, it must never again map two very different blocks to the same
number. That test would have failed on the old rule on day one.

## 5. What the fix did not touch

Now the part that is not in any commit message.

The recalibration changed the curve. It did not look at the input. So follow
`complaints_311` back to where it comes from (`crates/ingest/src/run.rs:313`):

```rust
b.complaints_311 = count_within_m(*lat, *lon, &points_311, 150.0) as i32;
```

A 150-metre radius, real haversine distance, with a bounding-box prefilter and a
test that checks a point at 67 m is inside and a point at 1.9 km is outside
(`crates/ingest/src/geo.rs:89-100`). That code is correct and well-tested.

`points_311` is where it goes wrong (`crates/ingest/src/sources.rs:236-256`):

```rust
("$where", "created_date > '2024-01-01' AND latitude >= .. AND longitude <= .."),
("$limit", limit.to_string()),        // called with 50_000
```

Two things are missing from that query and one is fatal.

**There is no `$order`.** And the limit is 50,000.

I asked NYC how many rows that `$where` actually matches, using the same bounding
box as the shipped artifact (lat 40.6773–40.6989, lon −73.9602 to −73.9179):

```
[{"count_1":"219199"}]
```

**219,199 matching rows. The ingest asks for 50,000.**

So every `complaints_311` value in the shipped database was computed from
**23% of the data**, and *which* 23% is whatever Socrata felt like returning,
because no ordering was specified. The `$limit` is silently doing the work of a
sampling strategy that nobody designed.

**This is confirmed by the data itself**, independent of the API. The bounding box
is 8.64 km². 219,199 points over that area is 25,370 per km². A 150-metre circle
covers 0.0707 km², so a building should see about **1,793** complaints if the full
window were loaded. The observed mean across the 250 buildings is **445**.

```
observed / expected  = 445 / 1793 = 0.248
truncation ratio     = 50000 / 219199 = 0.228
```

Those two numbers agree to within a couple of points. The truncation is not
hypothetical — its fingerprint is in the shipped artifact.

**And the subset moves.** Querying the same `$where` today, row 1 in Socrata's
default order has `created_date` 2026-04-19, row 50,000 has 2026-04-27, and row
50,001 jumps to 2026-05-26. The ordering is an internal implementation detail that
shifts as records arrive. Re-run ingest tomorrow and you get different counts, and
nothing in the pipeline compares them or notices.

The ingest even prints the number:

```rust
println!("311: {} complaint points loaded", points_311.len());
```

It prints it and never compares it to the limit. `50000 == limit` is the classic
signal that a query was truncated, it was one `if` away, and it scrolled past in a
build log.

**Chapter 2 argued the determinism guarantee is real, and it still is** — once the
artifact exists, it is frozen, and the same DB gives the same answer forever. What
this shows is where that guarantee stops. It is determinism of *scoring*, not of
*ingest*. The artifact is reproducible; the process that builds the artifact is
not. Those get conflated constantly, and the distinction is the whole difference
between "you can check my number" and "you can check my number *and* rebuild it."

## 6. The margin nobody is watching

The old rule saturated at 30. The new one saturates at **c = 1069** — not 3209, as
the doc comment's choice of reference point implies (Chapter 2, §5).

The worst building in the shipped set has **977**.

```
worst observed:  c = 977  -> score 42
saturation:      c = 1069 -> score 40
margin: 92 complaints — 91% of the way there
```

The fix has 9% headroom on the current data, and that data is a 23% sample. The
pillar is not saturated today. It is one denser block, or one less aggressive
truncation, away from the top of the range starting to compress again.

That is not a prediction that it will break. It is the observation that **the fix
was validated against exactly the same dataset that exposed the original bug**,
and no one asked how much margin it bought. Four lines of arithmetic, done once,
would have surfaced the 91% figure at review time.

## 7. What the recalibration cost

Blunt accounting, because the chapter promised it:

- **The code change:** four lines, plus fifteen lines of rationale. Under an hour.
- **The rationale:** contains one arithmetic claim that does not close
  (`4.1 × 20 = 82`, not 60) and a reference point that hides the real floor by
  1.5 orders of magnitude. Both found by recomputing the curve, not by reading it.
- **The test:** correctly rewritten to assert discrimination. Genuinely good.
- **The margin:** never measured. 9%.
- **The input:** never examined. Still a 23% unordered sample.

The lesson is not "the fix was bad." The fix was the highest-value four lines in
the codebase. The lesson is that fixing a *symptom you can see in the output*
feels so much like fixing the problem that nobody walks one step upstream. The
saturation was visible in the scores. The truncation was visible only in a
`println!` that nobody compared against a constant.

---

## The hardest question a reader can ask of this chapter

> *"You moved saturation right by 1.5 orders of magnitude and shipped it as a
> fix. Given the input is an unnormalized raw count from a truncated query, what
> is this pillar actually measuring?"*

Three parts, and one of them cuts against me.

**On truncation — the objection lands completely.** The absolute magnitude of
`complaints_311` has no defined meaning. It is not "complaints near this building
since 2024." It is "complaints near this building in an unspecified 23% of the
records matching that window," and the free-allowance constant (`- 4.0`, ≈ 54
complaints) was calibrated against those sampled numbers. Against full data the
same allowance corresponds to roughly 235 real complaints. The curve is tuned to
an artifact of a `$limit`.

**On normalization — I expected the objection to land and it does not.** The
natural criticism is that a raw count within a fixed radius measures how many
people live near you as much as how bad your block is: a 150-metre circle around a
tower contains far more complainants than one around a row of walk-ups. That is a
good argument. It is testable, so I tested it on the 250 buildings:

```
correlation of complaints_311 with units_res    r = -0.196
                                  num_floors    r = -0.011
                                  year_built    r = -0.013
```

Essentially no correlation with building size, and the sign on residential units
is *negative*. Within this curated Brooklyn slice, complaint density is not a
proxy for how many units are nearby. I went looking for a defect and the data said
no. Normalizing by units would have made the pillar worse, not better.

**What it is therefore measuring** is a genuine spatial signal — 311 activity
within 150 metres — expressed on a scale whose zero point is arbitrary and whose
absolute values are a sampling artifact. That is more useful than it sounds:
because every building was sampled by the same truncated query, the *ordering* is
probably sound, and ordering is most of what a comparative score needs. What it
cannot support is the sentence a tenant would naturally read off it — "there were
440 complaints near this building."

Ranked by what a reviewer should insist on:

1. **Page the query.** Loop `$offset` with an explicit `$order=created_date`, or
   pre-aggregate server-side with `$group`. Turns an undefined 23% into all of it,
   and makes ingest reproducible. This is the one that matters.
2. **Fail loudly on truncation.** `if rows.len() as u32 == limit { bail!(..) }`.
   One line. It would have caught this at the first ingest run.
3. **Recalibrate `- 4.0` after (1).** The allowance is currently tuned to sampled
   counts. Full data means roughly 4× the counts, and the curve has 9% headroom.
   Fixing the input without re-checking the constants would push real buildings
   straight into the floor — the exact bug this chapter is about, reintroduced by
   its own fix.
4. **Assert the margin in a test.** `assert!(neighborhood_score(max_observed) > 40)`
   against the shipped artifact. Makes the headroom a property the suite defends
   rather than something someone computes once in a book.

Note the ordering trap in (1) and (3): doing the obviously-correct thing to the
query, alone, makes the product worse. That is the honest shape of this defect,
and it is why it is worth a chapter rather than a line in a changelog.

---

*Next: **Chapter 5 — The Type That Refuses to Be Wrong.** What `model` actually
encodes, what it deliberately does not, and the three places a `String` is doing a
job an enum should have.*
