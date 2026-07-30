# Chapter 1 — The Constraint Was Never Latency

> **The question this chapter answers:** What is HouseCheck actually optimising
> for, and why does that make this a book about types rather than a book about
> throughput?

---

## 1. The product, in one page

HouseCheck takes a New York City address and returns a single integer between 0
and 100.

Behind that integer are eight municipal feeds — HPD housing-maintenance
violations (`wvxf-dwi5`), 311 service requests (`erm2-nwe9`), PLUTO tax-lot
records (`64uk-42ks`), DOB elevator devices (`e5aq-a4j2`), DOHMH restaurant
inspections (`43nn-pn8j`), DHCR rent-stabilisation registrations (`39hk-dx4f`),
JustFix's DOF-derived rent-stabilised unit counts, and Census ACS B25064 median
gross rent. Four of those feed weighted pillars. The weights are 0.45 condition,
0.20 legal, 0.15 neighborhood, 0.20 accessibility
(`crates/scoring/src/lib.rs:78-84`).

The whole serving corpus is 250 buildings and 13,253 violation records in a
SQLite file of **671,744 bytes**. A person could hold the entire dataset in a
spreadsheet. Nothing about this system is large.

Nothing about it is fast, either, and that is the point of this book. There was
never a latency problem to solve. The 250 rows fit in L2 cache. Every interesting
decision in this codebase was forced by a different constraint, and this chapter
names it.

## 2. The failure mode that actually matters

A renter opens the app standing in an apartment, fifteen minutes into a viewing,
deciding whether to commit a year of their life and roughly forty thousand
dollars. They see a number.

If that number arrives in 900ms instead of 200ms, nothing happens. They wait.

If that number is **confidently wrong**, the product has not degraded — it has
inverted. A tool that exists to stop people signing bad leases has just helped
someone sign one, with more confidence than they would have had without it. The
failure is not that HouseCheck was unhelpful. It is that HouseCheck was worse
than nothing.

That asymmetry is the constraint. And it is not a value the author imposed on
the code after the fact — the code says it out loud. Here is the flagship
rent-comparison function in full
(`crates/scoring/src/lib.rs:88-94`):

```rust
pub fn rent_fairness(user_rent: i32, tract_median: i32) -> (f64, String) {
    // Defense-in-depth: Census B25064 ships suppressed tracts as 0 or a sentinel
    // negative (e.g. -666666666). A non-positive median is meaningless — never divide
    // by it, or the flagship feature would print a confident, fabricated number.
    if tract_median <= 0 {
        return (0.0, "no reliable neighborhood median available".to_string());
    }
```

Read what that guard prevents. The US Census suppresses median rent for tracts
with too few responses, and signals suppression with `-666666666`. That value is
a perfectly good `i32`. It divides without complaint. A tenant asking "is my rent
fair?" in a suppressed tract would have received a fluent, specific,
authoritative sentence containing a number derived from a value that means *we
do not know*.

No compiler catches that. No test catches it unless someone thinks to write one.
The type is `i32` and `-666666666` is a valid `i32`. The only thing standing
between that sentinel and a tenant is four lines of hand-written defence and the
comment explaining why they exist.

**That is the book in miniature.** The interesting question in this codebase is
never "how fast?" It is "what can this type express that is false?"

## 3. The workspace has the shape of that answer

Given the constraint, look at what the two crates that compute the number are
allowed to touch.

`crates/model` is 192 lines. Its entire dependency list is:

```toml
[dependencies]
serde.workspace = true
```

`crates/scoring` is 242 lines. Its entire dependency list is:

```toml
[dependencies]
model = { path = "../model" }
```

Seven-line manifests, both of them. And a grep across both crates returns zero
hits for every one of the following: `Result`, `unsafe`, `async`, `.await`,
`use std::`, `reqwest`, `tokio`, `rusqlite`.

Not "few." Zero.

The scoring core is a pure function library over plain data. It cannot perform
I/O, cannot fail, cannot block, and cannot observe anything outside its
arguments. Given the same inputs it returns the same outputs, forever, with no
ambient state to explain a discrepancy.

That is not an aesthetic preference. It is what makes the number *auditable*. A
tenant, a journalist, or an opposing lawyer can be handed four integers and
recompute the score by hand. There is nowhere for a discrepancy to hide, because
there is nowhere for the code to have looked.

Meanwhile `crates/api/src/main.rs` is 3,028 lines and contains all the mess — the
router, the state, nine handlers, the LLM transport, a hand-rolled rate limiter.
The mess is real and later chapters are unkind to it. But it is *quarantined*,
and the boundary is enforced by the dependency graph rather than by discipline.

## 4. The architectural bet

Every expensive or fallible thing happens exactly once, in a batch binary, on a
developer's laptop: eight network pulls, all geocoding, all distance
calculations, all joins across three incompatible building-identifier formats.
The output is a single file.

That file is then baked into a Docker image and opened read-only. The serving
path performs no network I/O, holds no credentials for any upstream, and cannot
be made to write.

The usual framing for this is "it's fast" or "it's cheap." Both are true and
neither is the reason. The reason is that **an immutable artifact cannot drift**.
Two identical requests a week apart return identical numbers because they are
reading the same bytes, and there is no upstream that can quietly change beneath
them mid-session. The deployed image also carries no application secrets, because
there is nothing for it to authenticate to.

The costs are real and this book does not hide them. Chapter 8 deals with the
fact that this artifact is gitignored, built on one machine, and reproducible
only in the loosest sense. Chapter 9 deals with what happens to a hand-rolled
schema that has no version number. The bet has a bill, and it comes due later.

## 5. The thesis, and the rule this book follows

> **Every load-bearing decision in this codebase was forced by a trust
> constraint rather than a performance one — and the places where the code is
> weakest are precisely the places where a trust constraint was expressed in
> prose, a comment, or a `String` instead of in a type.**

The book prosecutes that claim in both directions, which is the only way it is
worth anything. Where the constraint reached the type system or a test, it held.
Where it stayed in a doc comment, it drifted — and Chapters 5 and 6 show exactly
where, including a doc comment in `crates/model` that describes a version of the
code that no longer ships.

Two rules follow, and they are stated here so the reader can hold the book to
them:

**Every claim is cited to a file and a line.** If a claim in these pages has no
citation, treat it as opinion.

**There is an errata appendix**, because the first analytical pass over this
codebase got five things wrong — including the length of `main.rs` and a
confident assertion that "every scoring function ends in a defensive clamp,"
which is false in two of six cases. Those errors were caught by re-reading the
source rather than by anyone objecting. A book whose thesis is that unverified
claims drift, and which then presents its own unverified claims, would be
refuting itself in public.

## 6. What this book is not

It is not a claim that HouseCheck is well-built. Several chapters are hostile to
it. Chapter 3 establishes that the four pillar weights have **no citation
anywhere in the repository** — no source, no sensitivity analysis, no test that
they sum to 1.0 — and that the public case study describes the four axes
incorrectly. Chapter 6 shows that a corrupted HPD feed would silently deflate
every condition score in the product with no error and no log line.

It is also not a Rust tutorial. It assumes the reader knows what `Option` is and
is interested in the harder question of when a codebase declines to use it.

What it is: a record of what happened when a small team's principal engineering
constraint was that the output had to be *believable*, and an honest account of
where the type system carried that weight and where it was left to prose.

---

## The hardest question a reader can ask of this chapter

> *"'Honesty was the constraint' is the kind of thing you say after the fact.
> Name one decision where honesty measurably cost you performance or
> convenience."*

`GET /search`, and the cost is on every keystroke.

The obvious implementation is to forward the query to NYC GeoSearch and return
what it says. One network call, no local work. That is what the code did
originally.

It was replaced because GeoSearch is not deterministic. Five consecutive calls
for the string `464 Madison Street` — a building HouseCheck holds a full record
for — returned the correct building three times, HTTP 502 once, and a *different
building on the same street* once. The upstream held a veto over whether our own
data existed.

The replacement (`crates/api/src/main.rs:558+`) searches the curated set first,
and it does so by loading every building and normalising every address on every
query:

```rust
let mut scored: Vec<(u8, String, SearchResult)> = get_all_buildings(conn)?
    .into_iter()
    .filter_map(|b| {
        let hay = normalize_address(&b.address);
        ...
```

That is a full table read plus 250 string normalisations per search, against a
debounced input that fires while the user types. An index and a prepared
statement would beat it comfortably. A trigram index would beat it badly.

It was chosen anyway, because the property that matters is not speed — it is that
the same query returns the same building every time, and that no upstream outage
can make a building we hold data for report as outside our coverage. The
measurable cost is real; at 250 rows it is also affordable, which is the honest
qualifier. At 91,918 buildings it would not be, and Chapter 10 addresses what
breaks first.

---

*Next: **Chapter 2 — Determinism Is an Honesty Property, Not a Performance
One.** What deterministic scoring actually guarantees a tenant, and the exact
line where the guarantee starts leaking.*
