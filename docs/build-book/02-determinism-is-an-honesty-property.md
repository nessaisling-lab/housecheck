# Chapter 2 — Determinism Is an Honesty Property, Not a Performance One

> **The question this chapter answers:** What does "deterministic scoring"
> actually guarantee a tenant, and exactly where does the guarantee leak?

---

## 1. The claim, at the top of the crate

The first doc comment in the scoring crate is not about scoring. It is about
time (`crates/scoring/src/lib.rs:3-5`):

```rust
/// 0–100 building-condition score. Deterministic: `current_year` is passed in,
/// never read from the clock, so scores are testable and reproducible.
/// Open violations only. Severity: C=15, B=7, A=3. Recency (<=2 yrs) doubles it.
pub fn condition_score(violations: &[Violation], current_year: i32) -> u8 {
```

The reason a year is in that signature at all is the recency rule on the next
few lines (`:15`):

```rust
let recency = if current_year - v.year <= 2 { 2 } else { 1 };
```

A violation from the last two years costs double. Which means the condition
score is a function of *when you ask* — unless the "when" is an input rather
than an ambient fact.

Most codebases would call `Utc::now().year()` there. It is one line, it is
obviously correct, and it is what the domain means. This one takes a parameter
instead, and threads it through five call sites to do so.

## 2. What that actually buys, traced end to end

The claim in a doc comment is worth nothing on its own — this book's whole
thesis is that prose drifts. So follow the parameter.

**The value originates at ingest**, from an environment variable, on a
developer's laptop (`crates/ingest/src/run.rs:298-303`):

```rust
// Scoring recency reads this snapshot year, not the wall clock, so runs are reproducible.
let snapshot_year: i32 = std::env::var("SNAPSHOT_YEAR")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(2026);
store::set_snapshot_year(&conn, snapshot_year)?;
```

**It is written into the artifact** as a row in the `meta` table. In the shipped
database that row is, verbatim:

```
[('snapshot_year', '2026')]
```

**It is read once at process start** into `AppState`
(`crates/api/src/main.rs:206`), not per request:

```rust
let snapshot_year = get_snapshot_year(&conn)?.unwrap_or(DEFAULT_SNAPSHOT_YEAR);
```

**And it is passed explicitly** into every scoring call — `card_for` at `:317`,
`buildings_handler` at `:423`, and onward through `/compare`, `/rank`,
`/summary` and the agent's tool dispatch, all reading `state.snapshot_year`.

Then the fact that closes the argument: **`chrono` is not a dependency of this
workspace.** Not of `model`, not of `scoring`, not of `api`, not of `store`, not
of `ingest`. A grep across every `Cargo.toml` in the repository returns nothing.
There is no clock to read in the scoring path, because the crate that would let
you read one was never added.

## 3. The guarantee is stronger than the doc comment claims

The doc says scores are "testable and reproducible." That undersells it.

Because the year lives *in the artifact* rather than in the process, the same
`housecheck.db` returns the same score in 2026 and in 2030. Determinism here is
not merely across runs — it is **across time**. A screenshot a tenant took last
March still matches what the API returns today, from that artifact.

That property is what makes the number arguable. If a landlord disputes a score,
you can hand over four integers and a snapshot year and let them recompute it.
There is no "well, it was different when you looked" — not because anyone
promised, but because there is no mechanism by which it could have been.

Note what this costs, since Chapter 1 committed to naming costs: it means the
recency window is **frozen at 2026 until someone re-runs ingest.** A violation
issued in 2024 counts as recent forever, in an artifact nobody rebuilt. The
system cannot silently drift, and it also cannot silently keep up. Those are the
same property viewed from two sides, and the code chose the side where being
wrong is at least *stable and inspectable*.

## 4. Where the guarantee leaks

Three places. None fatal; all real.

### Leak 1 — the fallback is silent

```rust
get_snapshot_year(&conn)?.unwrap_or(DEFAULT_SNAPSHOT_YEAR)
```

`DEFAULT_SNAPSHOT_YEAR` is `2026` (`crates/api/src/main.rs:31`). If the `meta`
row is absent — a hand-built database, a partial ingest, a future migration that
forgets it — the API does not fail, does not warn, and does not log. It scores
every building against a hardcoded constant and serves the results with full
confidence.

Today that constant happens to equal the row's value, so the failure is
invisible *and* harmless. Those two facts are independent, and only one of them
is durable. The honest description is: this is a `unwrap_or` where the
alternative branch is a silent, plausible, wrong answer — which is precisely the
failure class the rest of the crate is organised to prevent.

### Leak 2 — `f64::ln` is the one call into libm

The neighborhood pillar is the only place in either crate that touches
transcendental math (`crates/scoring/src/lib.rs:55`):

```rust
let penalty = (((1.0 + c).ln() - 4.0).max(0.0) * 20.0)
    .round()
    .clamp(0.0, 60.0);
```

IEEE 754 pins `+`, `-`, `*`, `/` and `sqrt` to correctly-rounded results. It does
**not** pin `ln`. Rust's `f64::ln` delegates to the platform math library, and
different libm implementations are permitted to differ in the last ulp.

In practice this is almost certainly harmless here, and the reason is the
`.round()` on the very next line: an error of one ulp in the natural log has to
survive multiplication by 20 and then land exactly on a `.5` boundary to change
an integer. But "almost certainly harmless" is a different claim from
"deterministic," and the crate's own top-line doc comment makes the stronger one.

The one place it could bite is a cross-platform reproduction argument — the same
artifact scored on Linux glibc, on macOS, and in a musl container. Nobody has run
that comparison. It is one test.

### Leak 3 — the crate's only unguarded cast

```rust
(100.0 - penalty) as u8
```

Every other numeric exit in the crate clamps *then* casts. This one clamps the
**penalty** (`:57`), then does arithmetic, then casts the result unguarded
(`:58`).

It is safe. `penalty` is clamped to `0.0..=60.0`, so `100.0 - penalty` is
`40.0..=100.0`, comfortably inside `u8`. And since Rust 1.45 float-to-int `as`
casts saturate rather than wrap, with `NaN` mapping to `0` — so even a pathological
input degrades to a bounded number rather than to nonsense.

But it is safe *by consequence of a clamp three lines up*, not by construction.
Change the clamp bound to `70.0` and this line silently starts producing scores of
30 while every reader's eye slides past it. It is the one place in the crate that
breaks the crate's own idiom, and it is worth naming precisely because the idiom
everywhere else is so consistent.

## 5. The doc block that doesn't survive arithmetic

The neighborhood function carries a fifteen-line rationale — by a distance the
best-documented code in the repository (`:35-52`). Chapter 4 covers why it was
rewritten. Here, only whether it is *true*.

I recomputed the curve independently, outside Rust, from the formula as written:

```
penalty = clamp(round((ln(1 + c) − 4.0).max(0) × 20.0), 0, 60)
score   = 100 − penalty
```

**What checks out.** All three reference points in the doc — `c≈54 → 100`,
`c=262 → 69`, `c=3209 → 40` — are exactly right. So are all six points pinned by
the tests at `:177-191`. The free-allowance claim is right too: penalty first
becomes non-zero at `c = 55`, and the doc says "≈ c ≤ 54."

**What doesn't.** Two things.

First, the slope rationale (`:49`):

> `* 20.0` — slope: converts the ~0→4.1 usable log range into the 0→60 penalty band.

4.1 × 20 = **82**, not 60. For a slope of 20 to land on a 60-point band the
usable range must be exactly 3.0. The sentence is describing a mapping the
constants do not perform.

Second, and more consequential — the doc's own choice of reference point conceals
where the curve dies. It offers `c=3209 → 40` as the bottom of the range. But the
score first reaches 40 at **c = 1069**, and every value above that is identical:
`c=1069`, `c=3209` and `c=100000` all score 40.

Chapter 4 makes the argument about what that means for the pillar. The point *here*
is narrower and about method: the constants were documented with more care than
anything else in the codebase, and the documentation still contains an arithmetic
claim that does not close and a reference point that flatters the curve. Prose
drifts even when it is written by someone actively trying to be rigorous. That is
the thesis, demonstrated on the best-intentioned code in the repository.

---

## The hardest question a reader can ask of this chapter

> *"You advertise determinism at the top of the crate and then call libm. Which
> of the two claims is that doc comment making, and what would you have to
> change to make the stronger one true?"*

The doc comment is making the **weaker** claim, and it is correct.

Read it precisely: *"`current_year` is passed in, never read from the clock, so
scores are testable and reproducible."* Every clause is about the clock. It is a
statement that the function has no hidden temporal input — and that is verified,
end to end, all the way back to an environment variable at ingest, with `chrono`
absent from the workspace entirely.

It is not a claim of bit-level reproducibility across platforms. It does not
mention floats. Reading it as an libm guarantee is reading a stronger claim than
the sentence makes.

But the stronger claim is the one that matters for the property the chapter
opened with — handing four integers to a hostile party and inviting them to
recompute. To actually earn it:

1. **Replace `ln` with a fixed-point or integer approximation**, or precompute the
   curve as a lookup table over the realistic complaint range. The function is
   already a step function after `.round()`; the table would be a few hundred
   `u8`s and would remove libm from the scoring path entirely. This is the real
   fix, and it is small.
2. **Add a cross-platform reproduction test** — score the shipped artifact on
   glibc, musl and macOS and diff. Cheap, and converts an assumption into a
   result.
3. **Make the snapshot-year fallback loud.** `unwrap_or` becomes an error at
   startup, or at minimum a warning. An artifact without provenance should not
   serve scores silently.
4. **Clamp the score, not the penalty**, at `:57-58`, so the last unguarded cast
   in the crate stops depending on a bound three lines away.

Of those, (1) and (3) are the ones a reviewer should insist on. (3) is four lines
and closes a silent-wrong-answer path. (1) is the difference between a claim that
is true in practice and a claim that is true by construction — which, given this
codebase's entire argument, is the distinction it should care about most.

---

*Next: **Chapter 3 — Four Pillars, and the One That Isn't There.** What the
headline number is, precisely, where its four weights come from, and what the
published case study says it is instead.*
