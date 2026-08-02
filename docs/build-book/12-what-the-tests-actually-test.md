# Chapter 12 — What the Tests Actually Test

> **The question this chapter answers:** 111 tests pass in 0.11 seconds. What do
> they pin, what do they permit, and which four assertions would have caught this
> book?

---

## 1. The numbers

```
  crate     tests  asserts   src lines   test lines
  model         1        1        193      28 (15%)
  scoring      12       22        243     138 (57%)
  store        11       20        413     168 (41%)
  ingest       28       88      1,204     355 (29%)
  api          59      171      3,029     947 (31%)
  TOTAL       111      302
```

Frontend test files: **0**. There is no `test` script in `package.json` and no
runner configured. 4,297 lines of application TypeScript — including
`normalizeBuilding`, `bandFor`, and the `score.ts` helpers this book has quoted
repeatedly — have never been executed by anything except a browser.

The whole Rust suite runs in **0.11 seconds**. Not one test opens a socket — I
grepped for it. Every database test uses `:memory:`. The suite is completely
hermetic, completely deterministic, and fast enough to run on every save.

Two distributions are worth noting before anything else. `scoring` is 57% test
lines, the highest in the workspace, which is the right instinct: it is where the
product's arithmetic lives. And `model` has **one test with one assertion for 193
lines** — the crate every other crate depends on, whose three stringly-typed
fields produced live defects in Chapters 5 and 9. The crate with the least test
coverage is the one with the widest blast radius.

## 2. Twenty-eight ingest tests, and what they are

Chapter 8 found the defect that matters most in `ingest`. So it is worth naming
exactly what its 28 tests cover:

- **Five query builders** — `bbl_in_query`, `hpd_block_query`, `census_url`,
  `complaints_311_query`, `restaurant_grades_query`
- **About fifteen parsers** — PLUTO records, BBL reconstruction, HPD violations,
  DOB elevator devices, Census medians, 311 points, restaurant grades, and five
  separate tests for the JustFix rent-stabilization CSV rows
- **Geometry** — haversine, radius counting, nearest ADA station
- **Config** — CLI argument parsing

Every one is a pure function. The parser tests are genuinely good: `parse_rentstab_row`
alone has five, covering most-recent-numeric, trailing zero, the 2024 column,
all-NA, and malformed headers. That is someone who found real dirty data and pinned
each shape.

What is not there: **any test that a response was complete.** Not one of the 28
concerns itself with how much data came back.

## 3. Two truncations, two opposite non-catches

This is the centrepiece, and it is better than either simple version of the story.

Chapter 4's truncation is on the 311 query. Its test:

```rust
fn complaints_311_query_bounds_bbox_and_recent_date() {
    let (base, params) = complaints_311_query(40.68, -74.0, 40.70, -73.90, 50000);
    …
    assert_eq!(param(&params, "$select"), "latitude,longitude");
    assert_eq!(param(&params, "$limit"), "50000");
}
```

It asserts the limit. **The truncation is pinned as intended behaviour** — the same
failure mode as Chapter 4's `assert_eq!(neighborhood_score(100), 40)`. Page the
query and this test goes red, with a name saying it verifies the query is properly
*bounded*. Which it does. That is the problem.

Chapter 8's truncation — the one that costs half the violation history and 6.5
points of mean score — is on `hpd_block_query`. Its test:

```rust
fn hpd_block_query_filters_by_boro_and_blocks() {
    let (base, params) = hpd_block_query(3, &[1599, 1970]);
    assert!(base.ends_with("/wvxf-dwi5.json"), "base was {base}");
    assert_eq!(param(&params, "$where"), "boroid=3 AND block in(1599,1970)");
    assert!(param(&params, "$select").contains("violationstatus"));
}
```

**It never mentions `$limit`.** The 50,000 that drops 63% of the rows is invisible
to the test that covers that exact function.

So the two most consequential defects in the system sit under tests that fail to
catch them for opposite reasons. One pins the wrong value; the other does not look
at it. Four `$limit` assertions exist in the suite — `"200"`, `"50000"`, `"50000"`,
`"20000"` — and the query where the value does the most damage is not among them.
Nobody decided that. It is just inconsistency, and inconsistency is what
distributes a bug's visibility at random.

Both test names are accurate. Both are green.

## 4. The shape of the blind spot

The general form is worth stating plainly, because it is the transferable lesson:

**A query-builder test can only ever confirm that you built the query you meant to
build.** It has no access to the question of whether the query you meant was right.
`complaints_311_query_bounds_bbox_and_recent_date` verifies the bbox is in the
`$where`, the date cutoff is present, the `$select` is minimal, and the limit is
50,000. All true. All checked. And it tells you nothing about whether 50,000 is
enough, because the number of matching rows lives on a server this test has
deliberately never contacted.

The same shape recurs across the workspace:

| chapter | defect | what the test suite did |
|---|---|---|
| 2 | silent `snapshot_year` fallback | nothing tests the missing-`meta` path |
| 3 | four weights unpinned | one test; 46 alternative vectors satisfy it |
| 4 | 311 truncation | **asserts the truncating limit** |
| 6 | schema ↔ dispatch unchecked | pins schema↔test, not schema↔dispatch |
| 7 | empty DB serves 200 OK | nothing constructs a missing artifact |
| 8 | HPD truncation | test omits `$limit` entirely |
| 9 | dead `!= "none"` branch | tests the Census branch only |
| 10 | `?? 0` renders "clean record" | **no frontend tests exist** |

Eight findings. In every case the tested surface is the pure, in-process,
deterministic part, and the defect is one step outside it — at a network boundary,
a filesystem boundary, a rendering boundary, or a hand-maintained correspondence
between two lists.

**The suite's greatest virtue is precisely what blinds it.** Hermetic, no sockets,
`:memory:` only, 0.11 seconds. Those properties are why it is worth running, and
they are achieved by drawing a line around everything that could disagree with the
code. Every defect in this book lives on the far side of that line.

That is not an argument for integration tests everywhere. It is an argument for
knowing which line you drew.

## 5. The four assertions

Ranked by how many of this book's findings each would have caught.

**One — the truncation guard. Catches Chapters 4 and 8.**

```rust
if rows.len() as u32 == limit {
    anyhow::bail!("{base}: hit the {limit}-row limit — result is truncated");
}
```

One statement in `get_json_query`, applying to all five `$limit` sites at once. It
would have fired on the first real ingest run in July, before anything shipped,
naming the URL. This is the single highest-value line of code not in this
repository — it prevents a 6.5-point score bias and a 50% data loss, and it is
shorter than the comment explaining it.

**Two — a real artifact at startup. Catches Chapters 2 and 7.**

```rust
let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
let n: i64 = conn.query_row("SELECT count(*) FROM buildings", [], |r| r.get(0))?;
anyhow::ensure!(n > 0, "artifact at {path} has no buildings");
```

Chapter 7 demonstrated the current behaviour: point the binary at a missing file
and it creates an empty database, serves `/health` → `ok`, and returns 404 for
every building with no warning. Read-only open makes the missing case impossible
rather than detected; the count makes the empty case fatal. It also closes
Chapter 2's silent `unwrap_or(DEFAULT_SNAPSHOT_YEAR)`, because an artifact without
a `meta` row is exactly the artifact this rejects.

**Three — any frontend test at all. Catches Chapter 10.**

```ts
it("distinguishes absent violation data from zero", () => {
  expect(normalizeBuilding({ building: {} }).open_violations.c).toBeNull();
});
```

There is no test runner in `package.json`. The function that converts a missing
field into `0`, which the card then renders as *"a clean hazardous-violation
record"* for a building with seven open class-C violations, has never been asserted
against. Writing this test forces the `number | null` change that fixes it — the
test cannot be written honestly without fixing the type.

**Four — basis vectors for the weights. Catches Chapter 3.**

```rust
assert_eq!(total_score(100, 0, 0, 0), 45);
assert_eq!(total_score(0, 100, 0, 0), 20);
assert_eq!(total_score(0, 0, 100, 0), 15);
assert_eq!(total_score(0, 0, 0, 100), 20);
```

Chapter 3 enumerated every four-weight vector on a 0.05 grid summing to 1.0 and
found **46** that satisfy the single existing test — including `(0.00, 0.30, 0.00,
0.70)`, which assigns zero weight to building condition. Four lines pin all four
weights exactly.

Two more that are cheap and close real defects: the schema-to-dispatch loop from
Chapter 6, and extending `citations_only_claim_sources_that_were_actually_used` to
the stabilization branch it does not currently touch — the one whose dead
`!= "none"` comparison over-claims a DHCR source on 65% of buildings.

Total: roughly **fifteen lines of assertion** covering six of this book's ten
findings-chapters, including both of the two that actually change what a tenant is
told.

---

## The hardest question a reader can ask of this chapter

> *"One hundred and eleven passing tests and you found ten chapters of defects.
> What is the test suite for?"*

It is for the thing it is good at, and the honest answer requires separating two
claims that get conflated.

**The suite has caught real bugs and still prevents them.** Five separate tests pin
`parse_rentstab_row` behaviours that come from actually dirty CSV data. The
`rent_fairness_guards_nonpositive_median` test pins the Census sentinel guard —
without which, per that function's own comment, *"the flagship feature would print a
confident, fabricated number."* `neighborhood_discriminates_dense_blocks` uses
`assert_ne!` to encode a failure mode rather than a behaviour. `hpd_violation_closed_status_is_not_open`
catches an inversion that would silently double every condition score. These are
not ceremonial. They are load-bearing, and they run in a tenth of a second.

**What it does not do — and was never built to do — is doubt the world.** Every
test in this workspace answers "does this function do what I wrote it to do?" Not
one answers "was what I wrote the right thing to ask for?" Those are different
questions, and only the first is answerable without leaving the process.

So the fair statement is: this is a good unit test suite, and the system's failures
are not unit-level. Chapter 8's finding required querying NYC's API and diffing it
against the artifact. Chapter 11's required compositing a translucent gradient by
hand. Chapter 7's required launching the binary against a path that does not exist.
None of that is a test any reasonable person writes at the start of a project, and
all of it is the kind of check that should exist by the time something is published
with real people's housing decisions attached.

The distinction I would defend under questioning: **111 tests is not the wrong
number. Zero of them crossing a boundary is the wrong shape.** The four assertions
above are the minimum crossing. Three of the four are single statements, one is a
test file that does not exist yet, and together they cover the defects that changed
what a user is told.

---

*Next: **Chapter 13 — Thirteen Agents and an Adversarial Verifier.** How this book
was researched, what the workflow got wrong, and why 27 findings were refuted.*
