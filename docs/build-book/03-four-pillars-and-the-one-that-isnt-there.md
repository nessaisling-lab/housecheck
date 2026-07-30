# Chapter 3 — Four Pillars, and the One That Isn't There

> **The question this chapter answers:** What is the headline number, precisely —
> and what does the public documentation say it is?

---

## 1. The number, in full

Here is every line of code that produces the integer a tenant sees
(`crates/scoring/src/lib.rs:78-85`):

```rust
/// Weighted 0–100 total. Weights: condition .45, legal .20, neighborhood .15, accessibility .20.
pub fn total_score(condition: u8, legal: u8, neighborhood: u8, accessibility: u8) -> u8 {
    let t = condition as f64 * 0.45
        + legal as f64 * 0.20
        + neighborhood as f64 * 0.15
        + accessibility as f64 * 0.20;
    t.round().clamp(0.0, 100.0) as u8
}
```

Eight lines. The weights sum to exactly 1.0, so the result is a true weighted
mean and inherits the 0–100 range of its inputs without needing the clamp — the
clamp is belt-and-braces, and Chapter 2 covered why this crate reaches for those.

The four pillars are **condition**, **legal**, **neighborhood**, and
**accessibility**. Hold on to that list. It matters in §4.

## 2. Where the weights come from

Nowhere.

I mean that literally, and I checked rather than assumed. A grep for `0.45`,
`0.20` and `0.15` across every `.rs`, `.md`, `.ts` and `.tsx` file in the
repository returns exactly two locations: `crates/scoring/src/lib.rs:80-83`,
and `docs/superpowers/plans/2026-07-21-housecheck-backend-core.md:447-449` —
which is the implementation plan, containing the same function body verbatim,
with no accompanying justification.

So the provenance chain for the most consequential four numbers in the product
is: they appear in the plan, and then they appear in the code. There is no
source, no sensitivity analysis, no reference to housing research, no note
explaining why condition should be worth three times neighborhood.

**Compare that to the pillar one function below it.** `neighborhood_score` gets
fifteen lines of rationale (`:35-52`) justifying every single constant — why
`ln`, why `1 +`, why `- 4.0`, why `* 20.0`, why the clamp, plus three reference
points. Chapter 2 found two errors in that block. It is still, by a wide margin,
the most carefully documented code in the repository.

The four weights that multiply that pillar's output get a one-line restatement of
what the code already says.

That asymmetry is the interesting part. The author clearly knew how to document a
constant — did it thoroughly, one function away. The weights didn't get the same
treatment because they never felt like a *choice*; they felt like a
specification. They came down from the plan, and code that implements a plan
faithfully doesn't feel like it's making a decision.

It was making a decision. `0.45` is an assertion that a building's violation
history matters two and a quarter times as much to a renter as its rent context.
That might be right. Nothing in this repository argues it.

## 3. The test that lets almost anything through

There is exactly one test that exercises `total_score`
(`crates/scoring/src/lib.rs:220-223`):

```rust
fn total_is_weighted_sum_rounded() {
    // 80,60,100,90 -> 80*.45+60*.20+100*.15+90*.20 = 36+12+15+18 = 81
    assert_eq!(total_score(80, 60, 100, 90), 81);
}
```

One input vector, one expected output. There is no test asserting the weights sum
to 1.0.

I wanted to know how much that single assertion actually constrains, so I
enumerated every four-weight vector on a 0.05 grid that sums to 1.0, and counted
how many also produce 81 from `(80, 60, 100, 90)`.

**Forty-six of them do.**

Among them:

```
(0.00, 0.30, 0.00, 0.70)
(0.00, 0.45, 0.45, 0.10)
(0.05, 0.45, 0.50, 0.00)
```

Read the first one. A weight vector that assigns **zero weight to building
condition** — the pillar the entire product is about, the one fed by 13,253 HPD
violation records — passes the only test guarding the scoring formula. So does
one that zeroes out accessibility. So do forty-four others.

The test is not worthless; it pins the rounding behaviour and catches a
transposed operator. But it does not defend the weights, and its comment — which
restates the arithmetic longhand — creates a strong impression that it does. A
reader skimming for coverage sees a weighted-sum test and moves on.

A three-line addition fixes the gap:

```rust
assert_eq!(total_score(100, 0, 0, 0), 45);
assert_eq!(total_score(0, 100, 0, 0), 20);
assert_eq!(total_score(0, 0, 100, 0), 15);
assert_eq!(total_score(0, 0, 0, 100), 20);
```

Four basis vectors pin all four weights exactly and make the intent unambiguous
to the next reader. It costs nothing and it is not there.

## 4. The pillar that isn't there

Now the part that is not a code-quality observation.

`docs/CASE-STUDY.md:26` tells the public what the score is made of:

> a single 0–100 score across four plain-language axes — **building condition**
> (HPD violations), **legal protections** (rent-stabilization, Good Cause),
> **rent fairness** (your rent vs the neighborhood median + HUD FMR), and
> **accessibility** (elevator-on-record + build-era)

The same framing opens the document at `:3`.

Compare with `total_score`. The four arguments are `condition`, `legal`,
**`neighborhood`**, `accessibility`.

**Rent fairness is not one of the four axes of the score. It is not in
`total_score` at all.**

`scoring::rent_fairness` has exactly two call sites in the entire API
(`crates/api/src/main.rs`): the `POST /rent-fairness` handler at `:465`, and the
agent's `check_rent_fairness` tool at `:1611`. It is never called from
`card_for`. It cannot be — it requires the user's own monthly rent as an
argument, which the server does not know at card-construction time and has no
business storing.

The pillar that *is* in the score, `neighborhood`, is computed from
`complaints_311` — the count of 311 service requests near the building. Rent
does not enter it.

So the published description is wrong in both directions at once. It names an
axis that isn't in the number, and omits the one that is.

**Why it happened is easy to see and worth stating plainly.** Rent fairness is the
better story. It is the feature a renter cares about, it is the one the agent
demos, and "your rent vs the neighborhood median" is a far more compelling
sentence than "311 complaint density on your block." The description drifted
toward the strongest version of the product. Nobody was lying; the marketing copy
and the arithmetic were written at different times by people looking at different
artifacts, and nothing in between them could disagree.

That is the same failure mode as the stale doc comment in Chapter 2 and the
stringly-typed enum in Chapter 6, moved up a layer. A `String` can't be checked
against the code that produces it. Neither can a paragraph in a case study.

**This one is live.** The case study is published. It should be corrected to name
the four actual pillars, and to describe rent fairness accurately as what it is —
a separate on-demand comparison, arguably the best feature in the product, which
does not feed the headline score.

## 5. The pillar weighted equally with legal protections

While auditing the four, one more thing is hard to defend.

`access_likelihood` (`:63-76`) is weighted **0.20** — identical to legal
protections, and higher than neighborhood. Its inputs, in full:

- `b.has_elevator` — from DOB elevator device filings (`e5aq-a4j2`)
- `b.num_floors`
- `b.units_res`
- `b.year_built`

One real signal and three proxies. A building scores 55 rather than 30 because it
was built after 1992 and has four or more units — an FHA-era inference, not an
observation. The function's own doc comment is honest about this: *"NOT a
certification."*

But "not a certification" and "one fifth of the headline number" are an awkward
pair. A build-era heuristic carries the same weight in the score as whether a
tenant has rent-stabilisation protection, which is a matter of public record.
Chapter 14 puts re-weighting on the ledger; the point here is that the weight was
never argued, so there is nothing to defend it with when someone asks.

## 6. There is no orchestrator

A structural note that costs the codebase later.

`scoring` exports six free functions and nothing that composes them. Assembling a
score is the API's job, and it does it in two places:

```rust
// crates/api/src/main.rs:317-321  (card_for)
let condition = scoring::condition_score(&violations, snapshot_year);
let legal = scoring::legal_score(&building);
let neighborhood = scoring::neighborhood_score(building.complaints_311);
let (accessibility, access_likelihood) = scoring::access_likelihood(&building);
let total = scoring::total_score(condition, legal, neighborhood, accessibility);
```

```rust
// crates/api/src/main.rs:423-427  (buildings_handler)
let condition = scoring::condition_score(&violations, snapshot_year);
let legal = scoring::legal_score(b);
let neighborhood = scoring::neighborhood_score(b.complaints_311);
let (accessibility, _) = scoring::access_likelihood(b);
let total = scoring::total_score(condition, legal, neighborhood, accessibility);
```

Near-identical, with one real difference: `card_for` keeps the accessibility
label, the list handler discards it with `_`.

Two copies of the assembly order is two places to forget a pillar. Add a fifth
tomorrow and the compiler will demand you update `total_score`'s signature — but
it will say nothing about the second call site computing its inputs. That is a
`scoring::score_building(&Building, &[Violation], i32) -> ScoreBreakdown` waiting
to be written, and it would move the composition into the crate that is already
pure, tested, and dependency-free.

---

## The hardest question a reader can ask of this chapter

> *"Where do 0.45, 0.20, 0.15, and 0.20 come from?"*

They come from the implementation plan, and the plan does not say where they came
from. That is the complete honest answer. There is no citation, no research
reference, no sensitivity analysis, and no test that would notice if they were
wrong — forty-six alternative vectors, including one that zeroes condition
entirely, satisfy the only test.

The three things that would change that answer, in order of cost:

1. **Pin them with basis-vector tests.** Four asserts. Converts the weights from
   a floating assumption into a stated, enforced intent. Does not justify them,
   but stops them drifting silently.
2. **Publish a sensitivity table.** Recompute all 250 buildings under several
   plausible weight vectors and report how many change band. If the answer is
   "almost none," the weights barely matter and the whole objection deflates —
   that would be a genuinely strong result. If the answer is "half," then the
   weighting *is* the product and it needs a defensible source.
3. **Source them, or stop claiming four equal-status axes.** Housing research
   exists on what predicts habitability complaints. Failing that, the honest
   framing is that condition dominates and the rest are modifiers — which is what
   `0.45` versus `0.15` actually encodes.

Number 2 is the one to do first. It is a script, not a study, and it converts
"we picked these" into a measured statement about how much the choice matters.
Given this codebase's stated posture, that is exactly the kind of question it
should have already answered about itself.

---

*Next: **Chapter 4 — Recalibrating a Pillar in Public.** How the neighborhood
rule was found to be lying, what the fix cost, and why the fixed version has the
same defect moved 1.5 orders of magnitude to the right.*
