# Chapter 5 — The Type That Refuses to Be Wrong

> **The question this chapter answers:** What does `model` encode, what does it
> deliberately not encode, and where is a `String` doing a job the compiler should
> have been given?

---

## 1. The crate that does almost nothing

```toml
[dependencies]
serde.workspace = true
```

192 lines. One dependency. Nine structs, two `impl` blocks, one test.

No `Result`. No `async`. No database handle, no HTTP client, no error enum. Chapter 1
established that `model` and `scoring` are the two crates with no I/O in them at
all; this is what that buys. `Building` is a struct that can be constructed in a
test in four lines and cannot fail to be constructed. Every crate in the workspace
depends on it and it depends on nothing that can break.

That shape is a refusal. A type that owns a connection can be wrong about the
database. A type that returns `Result` can be wrong about what failure means. This
crate declines both jobs, and the discipline holds for 192 lines.

Then it hands three of its most important invariants to `String`.

## 2. The three-state type whose three states are wrong

This is the centrepiece, and it is live in production right now.

```rust
/// Honest, three-state rent-stabilization signal for the Health Card. Public stabilization
/// lists are incomplete and never a legal ruling, so the wording is deliberately hedged.
pub struct Stabilization {
    /// "on_record" | "not_found" | "unverified" — machine-readable state for the frontend.
    pub status: String,
    /// Human wording shown to tenants.
    pub message: String,
}
```

Read the doc comment on `status` (`crates/model/src/lib.rs:71`). It enumerates
three values: **`on_record`**, **`not_found`**, **`unverified`**. It even says what
they are for — *machine-readable state for the frontend*. That is a type
declaration written in a comment.

Now read the only function that constructs the type (`:82-103`):

```rust
Some(true)  => status: "likely".into(),
Some(false) => status: "none_on_record".into(),
None        => status: "unverified".into(),
```

**`likely`. `none_on_record`. `unverified`.**

Two of the three documented values do not exist. `on_record` is never produced by
any code path in the workspace. Neither is `not_found`. The doc comment describes
an API that was never shipped, sitting directly above the field it describes, in
the crate whose entire job is to be the shared vocabulary between the backend and
the frontend.

Nothing can catch this. A doc comment is not checked against the function forty
lines below it. This is the Chapter 2 stale-comment failure and the Chapter 3
published-description failure in their purest form — same root cause, minimum
distance. The comment and the code are in the same file, ten screens apart, and
they disagree.

## 3. How the frontend got it right anyway

Here is the part that is genuinely interesting, because the product works.

`frontend/src/pages/HealthCard.tsx` contains **ten comparisons** against this field:

```
5 ×  stabilization === "likely"
5 ×  stabilization === "unverified"
0 ×  stabilization === "on_record"
0 ×  stabilization === "not_found"
```

The frontend is correct. It branches on the values the Rust code actually emits,
across five separate expressions — the label, the colour token, the screen-reader
summary, the detail row, the advisory line.

It is correct **because whoever wrote it ignored the documentation and read the
JSON.** The doc comment is the one artifact in the repository that claims to
specify this contract, and the consumer of the contract did not use it. They
opened the network tab.

That is worth being precise about, because it is easy to read as a criticism of
the frontend and it is the opposite. Reading the wire format is the *correct*
engineering response to a stringly-typed contract, because the wire format is the
only thing that is actually true. The cost is that the knowledge now lives in
someone's memory and in ten string literals in a `.tsx` file, with nothing
connecting them back to the Rust that produces them.

## 4. The fix that would break production

Now combine §2 and §3, and consider the most reasonable thing a future maintainer
could do.

They open `model/src/lib.rs`, see the doc comment on line 71, see the constructor
emitting `"likely"` instead of the documented `"on_record"`, and conclude — correctly,
by every normal reading — that the constructor has drifted from its spec. They fix
the constructor. Two-line change. It is the *right* instinct, it makes the file
internally consistent, and it is the change a code reviewer would approve on sight.

Every one of the ten frontend comparisons then fails to match.

Follow it through `HealthCard.tsx:235`:

```tsx
building.stabilization === "likely" ? "Yes"
  : building.stabilization === "unverified" ? "Unverified"
  : "No";
```

`"on_record"` matches neither branch, so it falls to the else. A building with
**twelve rent-stabilized units on the latest DOF record** would display:

> **Rent stabilized: No**

Not "unknown." Not blank. A confident, wrong, negative answer on the pillar the
product's own doc comment calls *"a signal, not a legal ruling"* — the one a tenant
is most likely to act on, and the one where being wrong in this direction costs
them a protection they actually have.

The Rust test suite would not catch it. Two tests pin these strings
(`crates/api/src/main.rs:2201, :2207`):

```rust
assert_eq!(card.stabilization.status, "likely");
assert_eq!(card2.stabilization.status, "unverified");
```

Those would go red, which is good — but they would go red in a way that reads as
*"the tests were pinning the old value, update them."* That is exactly what a
maintainer fixing a doc/code mismatch would expect to see, and updating them is a
one-character-per-line change. Nothing in the repository says the frontend is
listening. And `"none_on_record"` is pinned by no test at all.

**This is the strongest argument in the book for the enum.** Not aesthetics, not
type-safety-as-a-value. A three-variant enum with `#[serde(rename_all = "snake_case")]`
makes the doc comment unnecessary because the variants *are* the documentation,
makes the drift impossible because there is nothing to drift from, and — with a
generated or hand-maintained TS union — turns the silent frontend breakage into a
compile error at the exact ten sites that matter.

## 5. The class that isn't there

Second `String` doing a type's job (`:29-33`):

```rust
pub struct Violation {
    pub class: String, // "A" | "B" | "C"
    pub open: bool,
    pub year: i32,
}
```

Same pattern: a union type, written in a comment, where the compiler cannot see it.
And both consumers handle the unlisted case by ignoring it —
`ViolationCounts::open_from` with `_ => {}` (`:51`), `condition_score` with
`_ => 0` (`crates/scoring/src/lib.rs:13`).

So an unrecognised class scores **zero penalty** and appears in **no bucket** of
the counts. Invisible twice: it does not hurt the score, and it is not displayed
either. A tenant sees "3 open violations" and the number is complete as far as
anything on screen can tell.

That matters here because **HPD does not have three classes.** Asking the source
dataset directly:

```
class B   5,221,987
class C   2,563,707
class A   2,549,294
class I     805,526      <- 7.2% of all HPD violations
```

There is a fourth. And on the 122 Brooklyn tax blocks the curated set sits on,
HPD holds **11,944 class I violations**. The shipped artifact contains **zero**.

## 6. Where the boundary held, and where it stopped

That zero is not the `_ => {}` silently eating them. Chapter 4 found a truncated
query that nobody noticed; this is the opposite, and credit belongs where it is due
(`crates/ingest/src/run.rs:154-166`):

```rust
if !matches!(viol.class.as_str(), "A" | "B" | "C") {
    unknown_classes += 1;
    continue;
}
...
if unknown_classes > 0 {
    println!("note: {unknown_classes} violations had non-A/B/C classes (skipped)");
}
```

Explicit filter. Counted. Reported. This is the `if` that Chapter 4 wanted on the
311 query and did not get, written by the same person in the same file. The
boundary was defended deliberately and the decision was surfaced.

**The problem is that the boundary is the last place it is mentioned.**

For the 250 curated buildings specifically, I matched HPD's class I records back to
their BBLs:

| | |
|---|---|
| class I violations on the 250 buildings | **753** |
| of which **open** | **187** |
| buildings affected | **134 of 250 (54%)** |
| shipped DB, open violations | 2,553 |
| class I would add | **+7% open violations** |

More than half the buildings in the product have at least one class I violation on
record that the Health Card does not know exists. The count reached a build log
that has since scrolled away, and past `run.rs:155` nothing carries it: not `model`,
not `scoring`, not the API response, not the card, not the case study, not the deck.
The tenant-facing number is "open violations," unqualified.

To be fair to the decision: excluding a class you have not calibrated a penalty for
is more defensible than assigning it an invented weight. `condition_score` has
severity constants for A, B and C that someone chose; there is no fourth constant,
and inventing one to avoid an exclusion would be worse. The defect is not the
filter. The defect is that a deliberate, well-executed data decision stopped being
visible one line after it was made, and the product now states a violation count
that is complete only with respect to a filter nobody downstream can see.

Two fixes, both cheap. Carry the exclusion into the artifact — a `meta` row, the
same mechanism `snapshot_year` already uses, which Chapter 2 showed makes a
provenance fact survive into every response. And say it on the card: *"A, B and C
violations. Class I excluded."* One clause.

## 7. The fourth value the frontend invented

Third one, quickly, because it is the same shape and it inverts (`:111`):

```rust
pub access_likelihood: String, // "Higher" | "Mixed" | "Lower"
```

Three values, produced at three `return` sites in `access_likelihood`
(`crates/scoring/src/lib.rs:63-76`), and this comment is accurate. Then the
frontend, twice (`HealthCard.tsx:332, :571`):

```tsx
building.access_likelihood ?? "Unverified"
```

A fourth value, invented on the consumer side to cover `null`. The Rust type cannot
produce it — `access_likelihood` returns `(u8, String)`, never an `Option` — but
`normalizeBuilding` coerces a missing field to `null` (`api.ts:113`), so the
frontend needs a case the backend does not have.

Which is defensive and fine. The point is the direction of travel: the backend
publishes three values in a comment, the frontend needs four, and there is no place
where those two facts are written down together. `Option<AccessLikelihood>` on the
Rust side would state the same thing once, in a form both ends could read.

---

## The hardest question a reader can ask of this chapter

> *"Enums serialise to strings over the wire anyway. What does the compiler
> actually buy you here that a doc comment doesn't?"*

The objection is sharp and half of it is correct. `#[serde(rename_all)]` on an enum
emits exactly the same JSON. The TypeScript on the other side still receives a
string. Nothing about an enum crosses the network boundary — the wire format is
identical, byte for byte.

What changes is **where the string literals live and how many of them there are.**

With `String`, the value `"likely"` is authored in three places that cannot see each
other: the constructor in `model`, two assertions in the API tests, and ten
comparisons in `HealthCard.tsx`. Fifteen independent copies of a fact, and the
sixteenth — the doc comment — is already wrong and has been shipping wrong.

With an enum, the constructor and the tests reference `Stabilization::Likely`, and
the string exists once, in the `rename_all` attribute. Changing it becomes a
single-site edit that a grep can complete. The doc comment on line 71 stops being
necessary, which is the real win: **it cannot be wrong if it does not need to
exist.** Every defect in this chapter is a comment that fell out of sync with code
it could not be checked against.

Concretely, the three things an enum buys, in order of value:

1. **The rename attribute becomes the single source of truth.** `on_record` versus
   `likely` stops being a question anyone can answer two ways, because there is one
   place to look and it is machine-readable.
2. **Exhaustiveness at the match sites.** Add a fourth stabilization state and every
   Rust `match` fails to compile. Today, adding one means finding ten `===` in a
   `.tsx` file by memory.
3. **A generatable TS union.** `ts-rs` or `schemars` emits
   `type Stabilization = "likely" | "none_on_record" | "unverified"` from the Rust
   definition, and the ten comparisons become type-checked. This is the one that
   turns the §4 production break from silent into a build failure.

And the honest limit: **none of that would have caught the class I exclusion.** An
enum over `A | B | C` makes the unhandled case explicit at the match, which is
better than `_ => 0` — but the filter at `run.rs:155` is deliberate and correct, and
no type system tells you that a category you chose to exclude is 7% of the open
violations on 54% of your buildings. That is a provenance problem, and it needs the
`meta` row and the clause on the card, not a type.

Types stop the drift between things you wrote down twice. They say nothing about
what you decided not to write down at all.

---

*Next: **Chapter 6 — Three Thousand Lines of `main.rs`.** What the API layer
accumulated, which parts genuinely belong there, and the two extractions that would
pay for themselves.*
