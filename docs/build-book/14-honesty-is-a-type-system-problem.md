# Chapter 14 — Honesty Is a Type-System Problem

> **The question this chapter answers:** Is the title true? And if you get exactly
> one change, which one?

---

## 1. The thesis, and the evidence against it

The argument this book set out to prosecute:

> Every load-bearing decision in this codebase was forced by a trust constraint,
> not a performance one — and the places where the code is weakest are precisely
> the places where a trust constraint was expressed in prose, a comment, or a
> `String` instead of in a type.

Thirteen chapters in, the first half held completely. Nothing in HouseCheck is fast
because nothing needed to be: a 671 KB artifact, 2.2 ms cards, 167 ms cold start,
a 0.11-second test suite. Every architectural decision — the injected snapshot
year, the pure scoring crate, the baked database, the domain allowlist, the curated
legal directory — was made to be believed rather than to be quick.

The second half is where the book has to be honest about itself.

**The single largest defect found is not a type-system problem, and no type would
have caught it.**

`$limit` is `50000`. That is a `u32`. It is a perfectly good `u32`. The rows that
came back are well-typed `Violation`s, correctly parsed by well-tested parsers. A
`Vec<Violation>` holding 13,253 elements is, at the type level, indistinguishable
from one holding 26,306. There is no signature, no newtype, no enum, no lifetime
that expresses *"this HTTP response was complete."* Rust cannot help. Nor could
Haskell, nor Idris.

And that defect is the one that matters. It moves the mean total score 6.5 points,
moves 70 of 250 buildings into a band they do not belong in, and causes the Health
Card to state — affirmatively, in prose a person reads — that a building with seven
open immediately-hazardous violations has *"a clean hazardous-violation record."*

Every type-level defect in this book is smaller than that. Combined.

## 2. The distinction the title was missing

The thesis is not wrong. It is **scoped**, and I did not know the scope when I
wrote it.

There are two kinds of honesty in a system like this, and they fail differently.

**Internal honesty — does the code agree with itself?** This is squarely a
type-system problem, and the book found it repeatedly:

- `Stabilization.status` documents `"on_record" | "not_found" | "unverified"` and
  emits `"likely" | "none_on_record" | "unverified"`. Two of three wrong, live,
  with ten frontend comparisons written against the runtime values by someone who
  read the JSON instead of the type (Ch. 5).
- `citations_for` branches on `status != "none"` — a value produced nowhere in the
  workspace — so 163 of 250 buildings claim a DHCR source that returned nothing
  (Ch. 9).
- Four weights with no source and a single test that 46 alternative vectors
  satisfy, including one that zeroes building condition entirely (Ch. 3).
- Three hand-maintained copies of the tool list, with the schema-to-dispatch pair
  unchecked (Ch. 6).

Every one is a claim written down twice, in two places that cannot see each other,
which then drifted. An enum, a newtype, or an exhaustive match collapses each pair
to one site. **For this class, the title is exactly right.**

**External honesty — does the code agree with the world?** This is not a
type-system problem at all, and it is where the damage is:

- The 311 query returns 50,000 of 219,199 matching rows, unordered (Ch. 4).
- The HPD query returns 50,000 of 134,837, dropping half the violations and 77% of
  those inside the recency-doubled window (Ch. 8).
- The API opens a missing database, creates it, and serves `/health` → `ok` with no
  data (Ch. 7).
- Contrast tokens measured at 4.51:1 against a surface that is not what renders,
  landing at 4.44:1 composited (Ch. 11).

Different shape entirely. In each case the program asked the world a question,
received an answer, and had no mechanism for asking *"is that all of it?"* The types
were fine. The values were fine. **What was missing was an assertion at a boundary.**

So the honest restatement, which is what I would defend under questioning:

> **Types make a system's internal claims checkable. Only boundary assertions make
> its external claims falsifiable. This codebase did the first competently and the
> second not at all.**

Chapter 5 got half of this and stopped one step short: *"Types stop the drift
between things you wrote down twice. They say nothing about what you decided not to
write down at all."* The completion is that the thing nobody wrote down was never a
type. It was a question — *did we get everything?* — and it belongs in an `if`.

## 3. What the type system did do

It would be a bad book that spent thirteen chapters on defects and closed without
naming what held. Every one of these worked, and worked because something was
enforcing it rather than describing it:

- **`Option<bool>` for `rent_stabilized`** produced three deliberately hedged
  sentences instead of a guessed boolean, and the tri-state survived all the way to
  the card copy.
- **`std::sync::Mutex`** made the guard `!Send`, so holding the database lock across
  an `.await` does not compile. The comment says "drop the guard before any await";
  the compiler is what guarantees it (Ch. 6).
- **The `> 0` sentinel guard** on `rent_fairness` — because Census B25064 ships
  suppressed tracts as `0` or `-666666666`, and without it the flagship feature
  divides by a sentinel.
- **`include_domains`** made the legal web search a capability boundary rather than
  an instruction. The agent *cannot* reach a hostile page (Ch. 9).
- **`assert_ne!(neighborhood_score(100), neighborhood_score(500))`** encodes a
  failure mode instead of a behaviour — the one test in the workspace that asks
  "what would make this useless?" (Ch. 4).

And one worth recording precisely because it did *not* fire. The outline predicted a
live defect: `Some(true)` paired with a `None` unit count would render *"Likely
rent-stabilized — 0 units on the latest NYC DOF record (2024)"* via `unwrap_or(0)`.
I checked all 250 buildings. **All 87 stabilized buildings carry a real, non-zero
unit count.** The type permits that sentence; the data does not contain it. A latent
hole, not a live one — and saying so is the difference between an audit and a
prosecution.

## 4. The ledger

Every change this book argues for, ranked by **what a tenant sees differently**,
not by elegance.

| # | change | ch. | cost | what changes for a user |
|---|---|---|---|---|
| 1 | **Truncation guard** — `bail!` when `rows.len() == limit`, all five sites | 4, 8 | 1 line | Mean score −6.5. 70 of 250 leave the wrong band. The "clean record" sentence stops being false. |
| 2 | **Re-ingest and republish** after (1), recalibrating `−4.0` | 4, 8 | a run | The published numbers become true. Must land *with* (1): fixing the query alone pushes buildings into the floor. |
| 3 | **`number \| null` through the violation path** + a "no data" render | 10 | ~4 lines | Absence stops being displayed as a clean record. |
| 4 | **Read-only open + non-empty assert at startup** | 2, 7 | ~3 lines | An empty deploy fails instead of serving 404s under a green health check. |
| 5 | **Fill `meta`** — ingest date, coverage, class I exclusion, checksum — and surface it on `/health`, in `grounding_block`, and as `DATA_MONTH` | 5, 7, 9, 10 | ~20 lines | The product can state what it does and does not contain. Four chapters ended here. |
| 6 | **`enum Stabilization` + generated TS union**; fix `citations_for` | 5, 9 | ~15 lines | Removes the 65% false DHCR citation and the trap where fixing the doc comment silently breaks ten frontend branches. |
| 7 | **Basis-vector weight tests** | 3 | 4 lines | Nothing today. Stops the weights drifting silently tomorrow. |
| 8 | **Card alpha `.94` → `1.0`** | 11 | 1 char | Five band tokens go from 4.44:1 to 5.12:1, clearing AA. |
| 9 | **Census link `ACSDT1Y2023` → `ACSDT5Y2023`** | 10 | 1 char | The "check our source" link reaches a table that contains the number. |
| 10 | **`enum ViolationClass`**, schema↔dispatch test, extract `crates/agent`, delete `components/ui` | 5, 6, 10 | ~1 day | Nothing a user sees. Makes classes of defect unrepresentable. |

Items 1 through 5 are roughly **thirty lines and one ingest run**, and they account
for every finding in this book that changes what a person is told.

Items 6 through 10 are the type-system work the title promised. They are real, they
are cheap, and they are *below* a one-line `if` in the ordering. That ranking is the
book's actual conclusion.

## 5. The change to refuse

The outline named **per-unit normalization of `complaints_311`** as *"the largest
correctness win and the largest amount of work."*

Refuse it.

The reasoning is intuitive and wrong: a 150-metre circle around a tower holds more
people than one around a row of walk-ups, so a raw complaint count must be partly a
population proxy, so divide by units. I believed it too, and wrote it into a chapter
before testing it. Measured across all 250 buildings (Ch. 4):

```
complaints_311 vs units_res     r = -0.196
              vs num_floors     r = -0.011
              vs year_built     r = -0.013
```

No correlation, and the sign on residential units is **negative**. Dividing by
units would inject a relationship the data does not contain and degrade a pillar
that currently works. It is a substantial piece of work whose measured effect is to
make the product worse.

That is the one to refuse, and the argument for refusing it is the same argument
this whole book makes: somebody checked.

---

## The hardest question a reader can ask of this book

> *"You have listed ten changes. If you get exactly one, which — and why does it
> beat the other nine?"*

**The truncation guard.** One statement in `get_json_query`:

```rust
if rows.len() as u32 == limit {
    anyhow::bail!("{base}: hit the {limit}-row limit — result is truncated");
}
```

The argument, in the order it should be made:

**It is the only change on the list that makes the published numbers true.** Items
3 through 10 improve how the product handles, displays, or types its data. This one
fixes the data. Mean condition score is overstated by 14.3 points and the total by
6.5; 70 of 250 buildings sit in a band above the one they belong in; and every one
of those errors runs in the same direction, because `condition_score` starts at 100
and subtracts, so a violation you failed to fetch can only ever make a landlord's
building look better. A tenant-facing product with a structural pro-landlord bias
has failed at the only thing it was for.

**It covers two chapters at once, and they are the two biggest.** The same line
fixes the 311 query behind the 0.15-weight pillar and the HPD query behind the
0.45-weight one. Five `$limit` sites, one guard.

**It would have fired in July, before anything shipped**, naming the URL. Every
other item on the ledger is a fix; this one is the check that would have made the
fix unnecessary.

**And it is one line.** Shorter than the comment explaining it. The codebase already
contains this exact pattern, sixty lines from where it was needed — the class filter
at `run.rs:155` counts what it skips and prints the count. Somebody knew. It got
applied to the categorical boundary and not the volumetric one.

If I get a second: **item 3**, `number | null`, because it is the only change that
removes a false sentence from the screen rather than correcting a number on it.

---

## Coda

The best sentence in this codebase is not in any chapter's argument. It is a comment
someone left above a two-line guard (`crates/scoring/src/lib.rs:89-91`):

> Census B25064 ships suppressed tracts as 0 or a sentinel negative (e.g.
> `-666666666`). A non-positive median is meaningless — never divide by it, or the
> flagship feature would print **a confident, fabricated number**.

That is the whole book, and it was already in the source before the book existed.
Someone understood the failure mode exactly, named it precisely, and wrote code that
made it impossible — for that one field, on that one path.

The finding of these fourteen chapters is that the same person, in the same
codebase, in the same month, did not ask the same question about a `$limit`. Not
from carelessness — the ingest is careful, the tests are real, the accessibility
work is measured, the agent's guardrails are structural rather than aspirational.
The question simply looks different when the value in front of you is `50000`
instead of `-666666666`. One of them announces itself as a lie. The other looks like
a generous ceiling.

A confident, fabricated number does not usually arrive as a sentinel. It arrives as
a plausible integer, correctly typed, faithfully parsed, honestly rendered, and
half of what it should have been.

---

*End of book. Corrections, method, and this book's own error rate: **Chapter 13**.*
