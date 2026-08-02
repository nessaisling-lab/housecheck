# Chapter 8 — The Ingest Nobody Runs Twice

> **The question this chapter answers:** What does a 1,199-line pipeline against
> eight public datasets actually guarantee, and what does the published score look
> like when you check it against the source?

---

## 1. The pipeline

```
crates/ingest/src/sources.rs   643    query builders + parsers
crates/ingest/src/run.rs       350    orchestration
crates/ingest/src/geo.rs       101    haversine, radius counts
crates/ingest/src/config.rs     82    CLI
crates/ingest/src/main.rs       23
```

Blocking `reqwest`, no async, no runtime. For a batch job that runs once on a
laptop this is the right call — there is nothing to overlap, and `tokio` would buy
complexity and no throughput.

Eight external sources:

| source | what it provides |
|---|---|
| Socrata `64uk-42ks` (PLUTO) | the building set, coordinates, year, floors, units |
| Socrata `wvxf-dwi5` (HPD) | violations, by class and status |
| Socrata `e5aq-a4j2` (DOB) | elevator devices |
| Socrata `erm2-nwe9` (311) | complaint points |
| Socrata `43nn-pn8j` (DOHMH) | restaurant grades |
| Socrata (MTA) | ADA-accessible stations |
| `api.census.gov/data/2023/acs/acs5` — note the pinned vintage | B25064 tract rent medians |
| JustFix S3 CSV | DOF rent-stabilization unit counts |

Note the `2023` in the Census URL. That query is **pinned to a dataset vintage**,
so it returns the same rows in 2026 that it did in 2025. It is the one external
call in this pipeline that is reproducible by construction, and it happens to be
the one against an API that versions its releases. Everything else queries a live
dataset with no as-of parameter.

The fetch helper is honest and simple (`run.rs:74-85`):

```rust
let resp = c.get(base).query(params).send()
    .with_context(|| format!("GET {base}"))?;
let resp = resp.error_for_status()
    .with_context(|| format!("bad status for {base}"))?;
resp.json().with_context(|| format!("decode json from {base}"))
```

No retries, no backoff. Any transient failure aborts the run with a message naming
the URL. For a batch job that is defensible — partial data written to an artifact
is worse than no artifact — and the `with_context` chain means a failure tells you
which of the eight sources broke.

What it does not do is check whether the response was complete.

## 2. Five limits

`$limit` appears at five production call sites:

```
sources.rs:67    PLUTO             limit passed in
sources.rs:144   HPD violations    50,000 hardcoded
sources.rs:185   bbl_in_query      50,000 hardcoded   (DOB elevators)
sources.rs:254   311               limit passed in    (called with 50,000)
sources.rs:290   restaurants       limit passed in
```

Chapter 4 took apart the 311 one: 219,199 rows match the `$where`, 50,000 are
requested, no `$order`, and the counts in the artifact carry the fingerprint of the
truncation. The neighborhood pillar it feeds is weighted **0.15**.

This chapter is about the query on line 144, which feeds the pillar weighted
**0.45**.

## 3. One chunk, one query

```rust
for chunk in blocks.chunks(500) {
    let (base, params) = hpd_block_query(boroid, chunk);
```

The curated set spans **122 Brooklyn tax blocks**. `chunks(500)` therefore yields
exactly one chunk, so the entire violation history for the entire product is
fetched in **a single request**, with `$limit` of 50,000 and no `$order`.

I asked HPD how many rows that `$where` matches:

```
$where: boroid=3 AND block in(<122 blocks>)
[{"count_1":"134837"}]
```

**134,837 matching. 50,000 requested.**

The chunking exists — someone thought about batch size — and 500 is large enough
that it never engages. A chunk size of 100 would have produced two queries and
still truncated; the problem is not the chunk size, it is that `$limit` is treated
as a generous ceiling rather than as a value that must be checked.

## 4. What's missing is not a random half

Filtering HPD's records back to the 250 curated BBLs, restricted to the A/B/C
classes the ingest keeps (Chapter 5), and comparing against the shipped artifact:

```
                          HPD holds    artifact    coverage
  A/B/C violations           26,306      13,253      50.4%
  of which open               5,261       2,553      48.5%
```

Half. But the interesting part is the distribution by issue year:

| year | HPD holds | artifact | coverage |
|---:|---:|---:|---:|
| 2026 | 1,377 | 253 | **18.4%** |
| 2025 | 2,041 | 521 | **25.5%** |
| 2024 | 2,102 | 1,651 | 78.5% |
| 2023 | 2,137 | 1,681 | 78.7% |
| 2022 | 1,569 | 516 | 32.9% |
| 2021 | 2,009 | 680 | 33.8% |
| 2019 | 2,322 | 835 | 36.0% |
| 2018 | 1,610 | 455 | 28.3% |
| 2017 | 1,666 | 1,297 | 77.9% |
| 2016 | 1,226 | 1,194 | **97.4%** |
| 2015 | 1,352 | 1,310 | **96.9%** |

That is not a date cutoff. It is not a uniform sample. 2016 is nearly complete,
2018 is a quarter, 2023 is four-fifths, 2026 is a fifth. It is whatever slab
Socrata's internal ordering happened to return, and it is arbitrary.

Now recall `condition_score` (`crates/scoring/src/lib.rs:15`):

```rust
let recency = if current_year - v.year <= 2 { 2 } else { 1 };
```

A violation from the last two years counts **double**. So group the table by that
boundary:

```
  recency-doubled window (2025-26):   22.6% coverage
  everything older (<= 2024):         54.5% coverage
```

The truncation preferentially dropped the violations the scoring rule weights most
heavily. Not by design — by accident of Socrata's row ordering. But the effect is
that the pillar carrying 45% of the headline number is computed on under a quarter
of its highest-weighted input.

## 5. What the published scores actually are

The scoring functions are pure and take their inputs as arguments (Chapter 1), so
this is directly computable. I recomputed `condition_score` for all 250 buildings
from HPD's complete A/B/C records — same severity constants, same recency rule,
same `snapshot_year = 2026` — then fed the result through `total_score` with each
building's shipped legal, neighborhood and accessibility values unchanged.

```
  mean condition score    published 73.6   ->   actual 59.3    (-14.3)
  mean TOTAL score        published 69.5   ->   actual 63.0    ( -6.5)

  condition floored at 0  published   44   ->   actual   75
```

And through the product's own band function (`frontend/src/lib/score.ts:39-46`,
thresholds 80 / 60 / 40 / 20):

```
  buildings that change band:  70 of 250  (28%)

    solid   -> mixed     22        band distribution   published  actual
    strong  -> solid     20          strong               103        75
    mixed   -> concern   10          solid                 86        76
    solid   -> concern    9          mixed                 38        53
    strong  -> concern    4          concern               23        46
    strong  -> mixed      4
    mixed   -> solid      1
```

**Sixty-nine of the seventy move down.** The five worst:

```
  84 -> 39    689 MYRTLE AVENUE
  84 -> 39    21 KANE PLACE
  81 -> 36    536A MONROE STREET
  84 -> 39    75 LEWIS AVENUE
  72 -> 27    522 MONROE STREET
```

A building displayed to a prospective tenant as **84, "strong"** is, on HPD's
complete record, **39, "concern."**

## 6. The error has a direction

This is the part that makes it more than a data-quality bug.

Missing violations can only ever *remove* penalty. `condition_score` starts at 100
and subtracts. There is no mechanism by which an incomplete violation history
produces a score that is too low. Every one of the 250 buildings is scored at or
above its true condition, and 69 of them are displayed in a better band than they
belong in.

A product whose stated purpose is to give tenants leverage against landlords has a
systematic bias toward landlords, and the bias is structural rather than
accidental — it follows from subtracting penalties for records you have.

The one building that moved *up* (mixed → solid) is the exception that confirms the
mechanism: violations that were open at ingest have since been closed, which is the
only direction in which elapsed time improves a score.

**Caveats, stated plainly, because this is the strongest claim in the book:**

- HPD has grown since ingest ran, roughly a month. That inflates the 2026 row
  somewhat. It cannot explain 2025 at 25.5%, and it cannot explain 2018 at 28.3%
  against 2016 at 97.4%.
- The "actual" totals still use each building's shipped `neighborhood` score, which
  Chapter 4 showed comes from its own truncated query. So −6.5 is a **floor** on the
  error, not an estimate of it.
- The band thresholds are the product's, read out of `score.ts`, not chosen by me.

## 7. What the pipeline gets right

Three things, because the chapter is otherwise one-sided and the code is not.

**The class filter is exemplary** (Chapter 5): explicit `matches!`, a counter, and a
`println!` reporting how many records were skipped. That is precisely the guard this
chapter says the `$limit` needed, written by the same person in the same file, sixty
lines away. Whoever wrote `run.rs` knew this pattern. It was applied to the
categorical boundary and not the volumetric one.

**Nine warn-and-continue sites** handle optional sources degrading rather than
aborting — if the JustFix rent-stabilization CSV fails, buildings read "unverified"
and the run completes. The doc comment at `run.rs:276` says so explicitly. Optional
data degrades; required data aborts. That distinction is drawn deliberately.

**The Census query is version-pinned.** `data/2023/acs/acs5` returns the same rows
forever. It is the only reproducible external call in the pipeline, and it is
reproducible because someone wrote a vintage into the URL.

---

## The hardest question a reader can ask of this chapter

> *"You have shown the published scores are wrong by 6.5 points on average and that
> 28% of buildings are in the wrong band. Is the product salvageable, or is the
> whole thing invalid?"*

Salvageable, and the reason is the architecture the earlier chapters described.

**The scoring is not what is broken.** `condition_score` computed the correct answer
for the violations it was given — I verified that by reimplementing it and matching
the artifact exactly before changing anything. The severity weights, the recency
rule, the clamps, the determinism, the injected `snapshot_year`: all of it worked.
The failure is entirely upstream, in one unchecked integer on one line of a query
builder. A system where the defect is isolated to the ingest boundary is a system
where the fix is a re-ingest, not a rewrite.

**And the defect was findable because of the design choices in Chapters 1 and 2.**
Scoring is pure, so I could recompute 250 buildings outside the process. The
snapshot year is an argument, not a clock, so my recomputation used the same year
the artifact did. The database is a file, so I could diff it against the source.
None of that is available in a system that reads the clock and hides its scoring
behind an ORM. The property this codebase argued for — *hand someone the inputs and
let them recompute* — is exactly the property that let an outsider prove it wrong.
That is what auditability is for. It is supposed to produce findings like this one.

Ordered by what each buys:

1. **Page the HPD query.** `$order=novissueddate` plus `$offset` looping, or
   `$group` server-side. Same fix as Chapter 4's 311 query, same three lines,
   and this one moves 45% of the score instead of 15%.
2. **Fail on truncation, everywhere.** `if rows.len() as u32 == limit { bail!(..) }`
   at all five `$limit` sites. One line each. It would have caught both this and
   Chapter 4's on the first run, in July, before any of it shipped.
3. **Re-ingest and republish.** The mean total drops 6.5 points and a quarter of the
   buildings change band. That is a visible, explainable correction — and per
   Chapter 7 it also needs the `meta` provenance rows so the next version says what
   it contains.
4. **Assert the artifact against the source in CI.** Count rows at the source, count
   rows in the DB, fail if they diverge beyond a threshold. This is the check that
   makes "it worked once" into a claim with an expiry date.

Until (1) and (3) land, the honest statement about the live product is narrow and
worth writing down exactly: *the scores are computed correctly from an incomplete
violation history, they are biased upward, and the size of the bias has been
measured at 6.5 points on the total and 14.3 on the condition pillar.*

That sentence is not a good look. It is considerably better than not knowing.

---

*Next: **Chapter 9 — Eight Tools and a System Prompt.** What the agent can actually
do, how it is prevented from giving legal advice, and whether the grounding holds.*
