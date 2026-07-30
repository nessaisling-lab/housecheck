# HouseCheck: A Book Outline
### Working title — *Confident, Fabricated Numbers: Building a Tenant-Facing Score in Rust*

---

## The argument (the spine, stated once so every chapter can point at it)

HouseCheck computes a 0–100 Building Health Card for NYC tenants from eight municipal feeds. It serves 250 buildings out of a 671 KB SQLite file. Nothing in it is fast, because nothing in it needed to be. **Every load-bearing technical decision in the codebase was forced by a trust constraint, not a performance one** — and the places where the code is weakest are precisely the places where a trust constraint was expressed in prose, a comment, or a `String` instead of in a type.

The book's method is to prosecute that thesis in both directions: the decisions where the constraint was encoded in the type system or in a test and therefore held, and the decisions where it was encoded in a doc comment and therefore drifted. The reader should finish able to say which category any given line falls into.

**Evidence status, stated in the front matter:** the domain/scoring chapters (2–7, 13) rest on an independently re-verified map — 29 of 34 checkable assertions confirmed verbatim at cited lines, five refuted and carried below as corrections. The storage/ingest chapters (8–12) rest on a map that WAS independently verified — 24 assertions confirmed, 6 refuted — but the synthesis step that produced this outline received a truncated copy of that verdict, so chapters 8–12 should be re-read against the full storage verdict before drafting. Say this in the book. An audience that grills hard rewards a stated confidence level and punishes a uniform one.

---

## PART I — THE CONSTRAINT

### Chapter 1 — The Constraint Was Never Latency
**Question it answers:** What is HouseCheck actually optimizing for, and why does that make this a book about types rather than a book about throughput?

**Beats**
1. The product in one page: eight municipal feeds, four weighted pillars, one integer shown to a tenant deciding whether to sign a lease.
2. The failure mode that matters. A slow card is an annoyance; a confident wrong card is the whole product failing. Name the canonical example up front — the Census B25064 suppression sentinel `-666666666`, which divides into a plausible-looking percentage (`crates/scoring/src/lib.rs:89-91`).
3. The shape of the workspace as a consequence of that: `model` depends only on `serde`; `scoring` depends only on `model`; both are 7-line `Cargo.toml`s. No async, no I/O, no state, no `unsafe`, no `Result` in either crate (grep-confirmed, zero hits).
4. The architectural bet: all I/O, all network, all geometry, and all failure happens once, in a batch binary, on a developer's laptop. The serving path touches an immutable file.
5. Statement of thesis and of the book's evidentiary rule: every claim is cited to a file and line, and the corrections appendix exists because the first pass over this codebase got five things wrong.
6. What this book is not: it is not an argument that the design is good. Four of the fourteen chapters are prosecutions.

**Evidence:** both maps, top-level; `crates/model/Cargo.toml:7`, `crates/scoring/Cargo.toml:7`; `.github/workflows/ci.yml:54-56`.

**Hardest question:** *"'Honesty was the constraint' is the kind of thing you say after the fact. Name one decision where honesty measurably cost you performance or convenience — and one where you took the convenient option anyway."* (Both answers are in the book: the HPD ingest deliberately over-fetches entire tax blocks and discards the neighbors client-side, because HPD's feed has no BBL column to filter on. And `Violation.class` is a `String` because the SQLite column is TEXT and a `FromSql` impl was more typing.)

---

### Chapter 2 — Determinism Is an Honesty Property, Not a Performance One
**Question it answers:** What does "deterministic scoring" actually guarantee a tenant, and exactly where does the guarantee leak?

**Beats**
1. `current_year` is a parameter, never a clock read, and the reason is in the code: *"so scores are testable and reproducible"* (`crates/scoring/src/lib.rs:3-4`). Verified: no time call anywhere in either crate.
2. What a clock read would have done — the condition pillar silently drifting as the calendar advances, tests that fail on January 1st, and no way to reproduce a score a tenant screenshotted.
3. Where the year actually comes from: materialized into a `meta` table at ingest (`crates/ingest/src/run.rs:298-303`), read once at API startup (`crates/api/src/main.rs:206`), defaulting to a hardcoded `2026` and overridable by `SNAPSHOT_YEAR`. The shipped DB holds exactly one row: `snapshot_year|2026`.
4. The provenance gap: the snapshot year is *not* the ingest date, and `meta` records nothing else — no source URLs, no pull timestamps, no row counts. The most auditable-looking table in the schema audits nothing.
5. The float leak. `neighborhood_score` calls `f64::ln` (`scoring:55`). IEEE 754 does not require correctly-rounded transcendentals; last-ulp results are not guaranteed identical across libm versions, platforms, or opt levels. If `(ln(1+c) - 4.0) * 20.0` lands within an ulp of a `.5`, `.round()` flips and the score moves a point.
6. Determinism vs. reproducibility vs. bit-reproducibility — three claims a reader will conflate because the doc comment invites them to. Only the first is actually asserted by the code.
7. The cheap fix nobody made: the output space is 61 values. A 60-entry integer breakpoint table is genuinely bit-reproducible and is not a large ask.

**Evidence:** domain-and-scoring (determinism claim, ln claim, grill risk #6); storage-and-ingest (snapshot year, `meta` contents).

**Hardest question:** *"You advertise determinism at the top of the crate and then call libm. Which of the two claims is the doc comment making, and what would you have to change to make the stronger one literally true?"*

---

## PART II — THE NUMBER

### Chapter 3 — Four Pillars, and the One That Isn't There
**Question it answers:** What is the headline number, precisely — and what does the public documentation say it is?

**Beats**
1. The expression, quoted verbatim: `condition * 0.45 + legal * 0.20 + neighborhood * 0.15 + accessibility * 0.20`, then `t.round().clamp(0.0, 100.0) as u8` (`scoring:79-85`). Weights sum to 1.0, so the total is a true weighted mean and inherits the 0–100 range.
2. There is no orchestrator in `scoring`. Six free functions, no trait, no struct, no builder. The API wires the pillars itself.
3. And wires them **twice** — `card_for` (`api/src/main.rs:317-321`) and `buildings_handler` (`423-427`) each contain their own copy of the five-call sequence, deliberately, per the comment at `408-409`. Two independent copies of the definition of the score.
4. The pillars in one paragraph each, deferring the deep dives: condition (Ch. 4's sibling), legal (Ch. 5), neighborhood (Ch. 4), accessibility (Ch. 11's punchline).
5. **Rent fairness is not a pillar.** It has its own function, its own endpoint, and its output never reaches `total_score`. But `docs/CASE-STUDY.md:26` describes the four public axes as condition, legal, *rent fairness*, and accessibility. The fourth pillar is `neighborhood`. The public description of the product does not match the product.
6. The weights have no citation anywhere. Grep of `docs/` and `README.md` returns nothing; the only doc line is descriptive restatement. Contrast with `neighborhood_score`, where every constant gets its own rationale line — a fifteen-line doc block for a fifteen-point pillar.
7. The honesty inversion that organizes the rest of Part II: the most public number in the system is the least documented, and the least public is the most.

**Evidence:** domain-and-scoring, confirmed claims 1–2 and grill risk #4; the verifier's note #8 on the duplicated wiring.

**Hardest question:** *"Where do 0.45, 0.20, 0.15, and 0.20 come from?"* (Honest answer: nowhere citable. There is not even a test asserting they sum to 1.0 — `total_is_weighted_sum_rounded` pins one point, `(80,60,100,90) → 81`, which passes under several wrong weight vectors. And accessibility is weighted equal to legal protections while being computed from three PLUTO columns containing no accessibility data.)

---

### Chapter 4 — Recalibrating a Pillar in Public
**Question it answers:** How do you find out that a scoring rule is lying, and what does fixing it actually cost?

**Beats**
1. The original rule: −2 per 311 complaint, capped at −60. It floored at 30 complaints. In dense NYC that meant every building returned an identical 40 and the pillar carried zero information.
2. The archaeology: `git log -S` finds commit `2bc7851` ("311 rescale"). The pre-change tests pinned `neighborhood_score(10) == 80` and `(100) == 40` — the linear rule, preserved in the test suite exactly as described.
3. The replacement, with each constant's stated rationale: `ln(1+c)` (heavy tails; keeps `c=0` defined), `−4.0` (free allowance, roughly `c ≤ 54`, so a busy-but-normal block isn't dinged), `.max(0.0)` (stops low counts pushing the score above 100), `×20.0` (slope), `.clamp(0.0, 60.0)` (floors the score at 40).
4. The verified curve, recomputed independently outside Rust: c≤54 → 100, 55 → 99, 100 → 88, 262 → 69, 500 → 56, 1000 → 42, **1069 → 40**. Six of those seven points are test-pinned; c=55, c=1000, and c=1069 are not covered by any test.
5. **Saturation was not eliminated, it was moved.** c=1068 → 41; c=1069, c=3209, and c=100000 all → 40. The doc's chosen reference point (`c=3209 → 40`) is true but frames saturation as far-tail. *Correction to state plainly: the doc never claims 3209 is the onset — that inference belongs to the analysis, not to the code.*
6. The doc's own arithmetic doesn't close: it says ×20 maps a "~0→4.1 usable log range" onto a 0→60 band, but 4.1 × 20 = 82. The 0–60 band comes from the clamp on the next line, not from the slope. A carefully-reasoned comment that is still wrong is a specific and instructive failure.
7. The defect the curve cannot fix: `complaints_311` is a raw count, unnormalized by units, block area, or tract population. A 200-unit tower and a 6-unit walk-up on the same block get the same count and the same score. The log transform is doing cosmetic work over an uncorrected denominator.

**Evidence:** domain-and-scoring, neighborhood claims + verdict (curve independently recomputed, `git log -S` confirmed) + grill risk #5 + verifier notes #3 and #4.

**Hardest question:** *"You moved saturation right by 1.5 orders of magnitude and shipped it as a fix. Given the input is an unnormalized raw count, what is this pillar actually measuring — neighborhood conditions, or building size?"*

---

### Chapter 5 — "Unverified" Is a Type, Until It Isn't
**Question it answers:** Can the type system carry "we don't know," and where in this system does it stop carrying it?

**Beats**
1. In the data model, unverified *is* `Option`. Six of `Building`'s fifteen fields: `rent_stabilized`, `rent_stab_units`, `near_ada_subway_m`, `latitude`, `longitude`, `restaurant_grade`.
2. In the score model, unverified does not exist. `ScoreBreakdown` is five bare `u8`s (`model:58-65`); `BuildingListItem.score` is a bare `u8` (`model:163`). No `Option<u8>`, no sentinel, no variant. The backend cannot express "no data for this pillar" even if it wanted to.
3. Where the collapse happens, exactly: `if b.rent_stabilized == Some(true) { s += 25; }` (`scoring:26`). No `None` arm. `Some(false)` — confirmed zero stabilized units — and `None` — no DOF record found at all — both yield 60. Pinned by the test at `scoring:168`.
4. The tri-state survives on the display path. `Stabilization::from_units` exhaustively matches into three deliberately hedged statuses (`"likely"` / `"none_on_record"` / `"unverified"`), and the `none_on_record` copy explicitly tells the reader that public data lags.
5. So the same `Option` produces three distinct sentences and one indistinguishable integer. This is the book's central image and deserves a full page.
6. The frontend claims "an unverified pillar is never scored as a zero." The claim is *true* and *misleading*: unverified is not a zero, it is numerically identical to a confirmed negative, and the score type cannot tell you which you're looking at.
7. What would make the marketing claim enforceable: `enum Pillar { Scored(u8), Unverified }`, or `ScoreBreakdown { legal: Option<u8>, .. }`. Cost it honestly — it changes the JSON wire shape, which is why it didn't happen.
8. **Correction carried from verification:** the earlier framing of "two independent `Option`s give six representable combinations" is wrong. `Option<bool> × Option<i32>` is 3 × (2³²+1) at the type level; under the doc's own three-way bucketing of the unit count it is 3 × 3 = 9, of which three are declared legal. Six is only reachable by collapsing `Some(n>0)` and `Some(0)` — which is the exact distinction the invariant exists to protect. The point survives; the arithmetic did not.

**Evidence:** domain-and-scoring claims on `Option`, `ScoreBreakdown`, `from_units`, grill risk #3, refutation #2.

**Hardest question:** *"Your UI makes a guarantee about how unverified data is scored. Does the type system enforce it — yes or no?"* (No.)

---

### Chapter 6 — The Enum You Wrote in a Comment
**Question it answers:** What does stringly-typing cost when your input is a municipal data feed you don't control?

**Beats**
1. Exhibit A: `pub class: String, // "A" | "B" | "C"` (`model:30`). The enum exists — in a comment.
2. The two swallow sites: `_ => {}` in `ViolationCounts::open_from` (`model:51`) and `_ => 0` in `condition_score` (`scoring:13`). A `Violation { class: "Q".into(), .. }` compiles, penalizes nothing, and lands in no bucket.
3. The failure trace: HPD changes or corrupts a class code; every condition score silently inflates; no error, no log line, no test. The condition pillar is weighted 0.45. This is the highest-blast-radius defect in the codebase and it is invisible by construction.
4. The pattern repeats three more times — `Stabilization.status` (`model:71`), `HealthCard.access_likelihood` (`model:111`), and the `(u8, String)` return of `access_likelihood` (`scoring:63`). Four sites, one habit.
5. The proof that the habit costs something: the doc comment on `Stabilization.status` advertises `"on_record" | "not_found" | "unverified"`; the constructor emits `"likely" | "none_on_record" | "unverified"`. The doc is describing a previous version of the code and nothing could tell it otherwise. *Correction: that doc comment is at `model:71`, not `:72` — line 72 is the field itself.*
6. Why it isn't a live bug: the frontend union was written against the implementation. Why that's luck, not design: `frontend/src/types/building.ts:24-28` ends the union with `| string`, which widens the whole thing back to `string`. The three literals are documentation at both ends of the wire.
7. The fix and its real price: `#[derive(Serialize, Deserialize)] enum ViolationClass { A, B, C }` plus a `FromSql` impl for the TEXT column. Roughly six lines and one trait impl, and all four `_ =>` arms become unreachable-by-construction.
8. The related cast smell: `access_likelihood` returns `(u8, String)` and `buildings_handler` destructures it as `let (accessibility, _) = ...` — one heap allocation per building, immediately dropped. `&'static str` is free; an enum with a `score()` method would additionally stop a future edit from returning `(30, "Higher")`.

**Evidence:** domain-and-scoring grill risks #1, #2, #10; refutation #4; verifier note #6.

**Hardest question:** *"A class code you don't recognize starts arriving from HPD tomorrow. Walk me through how you find out."* (You don't. There is no log, no metric, no test on the fallthrough, and the score moves in the flattering direction.)

---

### Chapter 7 — Saying "No Data" in a Crate With No `Result`
**Question it answers:** If neither crate contains a single `Result`, how does the flagship feature report that it had nothing to work with?

**Beats**
1. The hazard, named in the code: Census B25064 ships suppressed tracts as `0` or `-666666666`, and dividing by it "would print a confident, fabricated number" (`scoring:89-91`). This is the best comment in the repository.
2. The guard, and its test — `rent_fairness_guards_nonpositive_median` exercises both `0` and the sentinel (`scoring:232-241`).
3. The mechanism, and its flaw: the guard returns an in-band tuple `(0.0, "no reliable neighborhood median available")`. The numeric channel is now ambiguous — `pct == 0.0` means either "suppressed tract" or "your rent is exactly at the median," and the only way to disambiguate is to string-match the verdict. The API forwards `pct_vs_median: pct` unchanged.
4. **The twist:** that branch is unreachable from HTTP. `crates/store/src/lib.rs:176-180` filters `AND median_gross_rent > 0` with a comment saying why, and `rent_fairness_handler` returns 404 "no rent data for tract" (`api:462`) before scoring is ever called. The guard is defense-in-depth, as its own comment says. Its only live callers are its two unit tests.
5. So the actual honesty mechanism is a *pipeline* of three independent filters, not the guard: `parse_census_medians` drops `median <= 0` at ingest parse time (`crates/ingest/src/sources.rs:222-225`), the store's read query filters `> 0`, and the scoring guard backstops both. Three layers for one sentinel, in a codebase with no retry logic anywhere. That ratio *is* the thesis.
6. Is defense-in-depth against an unreachable case good engineering or dead code? Argue it: the guard is the only thing standing between a future second call site and a fabricated number, and the function is pure and free. Then argue the other side: unreachable code is untested-in-production code, and the map's own summary described a path the API cannot take.
7. What the type should have been: `Option<(f64, String)>`, or a small `Verdict` enum with a `NoData` variant. The reasoning was excellent and then discarded at the type boundary — the recurring shape of this codebase's mistakes.
8. Thresholds and formatting as a footnote in honesty: ±5%, `{:.0}`, and `pct.abs()` on the below-median branch so the sign never prints twice.

**Evidence:** domain-and-scoring rent-fairness claims + grill risk #7 + verifier note #5; store `lib.rs:176-180` and ingest `sources.rs:222-225`, both re-verified for this outline.

**Hardest question:** *"You wrote a guard that cannot fire, tested it twice, and shipped it. Defend that as anything other than a comment with a return statement attached."*

---

## PART III — THE ARTIFACT

> Drafting note: chapters 8–12 rest on the storage/ingest map, which has not been through the adversarial verification pass that Part II's evidence has, and which arrived truncated. Re-verify every line citation in this part before drafting. The one claim that was truncated has been completed and confirmed above (Ch. 7, beat 5).

### Chapter 8 — One Writer, One File, Read-Only Forever
**Question it answers:** Why is the serving layer a 671 KB SQLite file baked into a Docker image, and what does that buy that a database server wouldn't?

**Beats**
1. The shape: a batch ETL binary writes the file once; the API opens it read-only; there is exactly one writer and it is never running at the same time as a reader.
2. The whole "DB into the image" mechanism is three lines — stage 2 `COPY data/housecheck.db`, `ENV HOUSECHECK_DB` (`Dockerfile`, 23 lines), and a `.dockerignore` that drops the `-wal`/`-shm` sidecars while explicitly keeping the `.db` so it survives the context filter.
3. Full rebuild, never incremental: `let _ = std::fs::remove_file(&cfg.out);` then `open_db` then `migrate` (`crates/ingest/src/run.rs:295-297`). The delete's error is discarded. Same pattern in fixture mode.
4. Why that's a trust decision: the artifact is a *snapshot*. It has one consistent view of eight feeds pulled minutes apart, it cannot half-update, and the deployed image is the provenance record — you can pull an old image and reproduce an old card exactly.
5. The contents, as shipped: 250 buildings, 13,253 violations, 41 tracts, 671 KB. The performance story is one sentence long and this chapter does not tell it.
6. **The hole in the middle of the argument:** `data/housecheck.db` is gitignored (`.gitignore:4`) and is produced only on the developer's machine. The immutable, auditable artifact at the center of a determinism story exists in exactly one place and is not in version control.
7. What would close it: commit a checksum, and add a manifest to `meta` recording each source URL, its pull timestamp, and its row count. The table already exists and holds one row.

**Evidence:** storage-and-ingest — Dockerfile, `.dockerignore`, `run.rs:295-297`, artifact stats, `.gitignore:4`.

**Hardest question:** *"The serving artifact is gitignored and built on one laptop from eight live upstream feeds with no retry logic. What, concretely, is reproducible about your reproducible build?"*

---

### Chapter 9 — Migrations Without a Version Number
**Question it answers:** What happens to a hand-rolled schema after v1, and when does "it's just a rebuild" stop being an excuse?

**Beats**
1. The schema is a Rust string literal. Four tables and one index in a single `execute_batch`. No `.sql` files, no migrations directory anywhere in the repo.
2. Post-v1 columns are `ALTER`s guarded by a `PRAGMA table_info` scan, because SQLite has no `ADD COLUMN IF NOT EXISTS`. Four columns arrived this way: `latitude`, `longitude`, `restaurant_grade`, `rent_stab_units`.
3. There is no version tracking of any kind — `PRAGMA user_version` appears nowhere in the workspace, no `schema_migrations` table. Idempotency is asserted by a test (`migrate_is_idempotent_on_existing_db`) rather than by a number.
4. The scar is visible in the shipped artifact: `.schema` shows the v1 comment line, then the four later columns appended after the original closing paren. The migration history is legible in the DDL and nowhere else.
5. Nullability as a semantic channel: `rent_stabilized INTEGER, -- NULL unknown / 0 no / 1 yes` carries the tri-state from Chapter 5 all the way down to the column. But `good_cause` and `has_elevator` are `NOT NULL` and therefore *cannot* express unknown — the storage layer has already made a decision the domain layer pretends is open.
6. `violations` has a synthetic rowid `id`, a bare `bbl TEXT NOT NULL` with no foreign key to `buildings`, and no unique constraint. Row identity is `(bbl, class, open, year)` by convention only. Under a full-rebuild model, that's defensible. Under any incremental model, it's a duplication bug waiting.
7. Where "just rebuild it" stops working: the moment one DB outlives one ingest binary. Nothing in the schema can tell you which binary wrote the file you're serving.

**Evidence:** storage-and-ingest schema/migration claims; `crates/store/src/lib.rs:21-51, 52-60, 65-75, 352-371`.

**Hardest question:** *"You have no version number, an `ALTER`-based migration path, and a nullable column whose NULL is load-bearing product semantics. How does the next engineer determine whether a DB they found is safe to serve?"*

---

### Chapter 10 — The Join Key Is the Product
**Question it answers:** What happens when the identity of a building arrives in three incompatible formats from three agencies of the same city?

**Beats**
1. BBL — borough, block, lot — is the join key for everything. Every claim on a card is glued together by a ten-character string.
2. Three inbound formats, three treatments. PLUTO ships BBL as a float string and is normalized through `f64` (`norm_bbl`, `sources.rs:49-55`). HPD's violations feed has **no BBL column at all**, so it's reconstructed from `boroid`/`block`/`lot` with zero-padding (`format!("{boro}{block:05}{lot:04}")`). DOB's BBL is consumed raw, with no normalization call.
3. That asymmetry is the chapter's live wire. Round-tripping a 10-digit identifier through `f64` happens to be safe (well under 2⁵³), but "happens to be safe" is doing load-bearing work, and the unnormalized DOB path is the one with no argument at all.
4. The over-fetch, and why it's an honesty cost rather than a performance one: you cannot filter a feed by a key the feed doesn't have. So the ingest queries by borough plus a chunked set of tax blocks (chunks of 500) and then discards every reconstructed BBL not in the curated set — *"a neighbor on the same block, not one of our buildings."*
5. Brooklyn is baked into the parser. The tract GEOID is derived arithmetically from PLUTO's `bct2020` by prefixing a hardcoded `"36047"` (Kings County), and the PLUTO `$where` hardcodes `borough='BK'` — while `--cd` is a configurable CLI flag. The configuration surface advertises a generality the parser doesn't have.
6. Where the trust actually lives: roughly half of `sources.rs`'s 643 lines are unit tests of the query builders and parsers. The most heavily tested code in the repository is the code that decides what a building *is*. That is the correct allocation and the book should say so.
7. Hand-rolled CLI parsing, no `clap`, 82 lines, defaults `--cd 303` and `--limit 200` — a one-paragraph aside on where dependency minimalism is fine and where it isn't.

**Evidence:** storage-and-ingest BBL/tract/over-fetch claims; `sources.rs:49-55, 62-65, 82-87, 128-159`; `run.rs:146-152, 174`; `config.rs`.

**Hardest question:** *"You normalize PLUTO's BBL through a float, reconstruct HPD's by string formatting, and take DOB's raw. If one of those three disagrees by a leading zero, what does the tenant see?"* (Not an error — a card with silently missing violations and a flatteringly high condition score.)

---

### Chapter 11 — Geometry Once, Never at Request Time
**Question it answers:** What do you give up by resolving every spatial question at ingest and shipping only scalars?

**Beats**
1. The rule, stated in CI: *"Geospatial joins run at ingest time, not at runtime."* Three joins are materialized per building — nearest ADA subway in metres, 311 complaints within 150 m, nearest graded restaurant within 200 m (`run.rs:312-314`).
2. Consequence: the serving DB has no spatial extension, no R-tree, no geometry. `latitude`/`longitude` exist only so the frontend can plot the curated set.
3. The primitives, hand-written in 101 lines: haversine with a hardcoded spherical earth radius of 6,371,000 m, and `count_within_m` with a bounding-box pre-filter before exact distance.
4. The pre-filter is deliberately *widened* — longitude by `1/cos(lat)`, plus `1e-4` — so it can never clip a true in-radius point. A correctness-first optimization: it is allowed to be slow, it is not allowed to be wrong. Hold this up next to the `_ => 0` from Chapter 6.
5. One bbox, folded from the curated set's coordinates, bounds both the 311 and the DOHMH restaurant pulls. If nothing geocoded, both pulls are **skipped entirely** — no citywide fallback. Honest emptiness over cheap coverage.
6. The radii and cutoffs — 150 m, 200 m, and the FHA-era `year_built >= 1992 && units_res >= 4` — are policy decisions frozen into a binary artifact. Changing a policy requires a rebuild and a redeploy, which is either excellent governance or an unshippable hotfix path, depending on the day.
7. **The punchline, and the reason this chapter sits here:** `near_ada_subway_m` is ingested, stored, and rendered in three places in the frontend — and is read by no scoring function. `access_likelihood` uses only `has_elevator`, `num_floors`, `year_built`, `units_res`. A pillar worth 20% of the headline number is computed from an elevator flag, a floor count, and a build year, while the only genuinely ADA-relevant datum in the model is display-only. (`docs/REVIEW-NOTES.md:28` records this as intentional per the PRD.)
8. Same shape for `restaurant_grade`, which at least says so: *"Neighborhood context only — display, never folded into any score."*

**Evidence:** storage-and-ingest geo claims + `crates/ingest/src/geo.rs`; domain-and-scoring `near_ada_subway_m` and `restaurant_grade` confirmations and grill risk #12.

**Hardest question:** *"You computed a real ADA-accessible-subway distance, stored it, and put it on the card next to an accessibility score it did not influence. How is that not the exact thing this book says the codebase is trying to avoid?"*

---

### Chapter 12 — Failure Policy Is Editorial Policy
**Question it answers:** Which upstream failures are allowed to change what a tenant sees, and who decided?

**Beats**
1. The split, made by hand, source by source. Fatal (`?`, aborts the run): PLUTO, HPD, DOB, MTA, 311. Non-fatal (warn and degrade to empty): Census, DOHMH restaurant grades, the JustFix rent-stabilization CSV.
2. The editorial logic that split implies: a building's identity and violations are the product, so a partial card is worse than no card. Rent medians and restaurant grades are context, so a card without them is still true.
3. What degradation looks like from the tenant's side — and this is the chapter's payoff. `"warning: rent-stabilization source skipped ({e:#}); rent_stabilized left null"` becomes `None`, which becomes `Stabilization { status: "unverified" }` from Chapter 5, which becomes hedged copy on the card. **The failure policy is legible to the end user.** That is a genuinely good design and it happened because the tri-state existed.
4. The inconsistency: `CENSUS_API_KEY` is a hard precondition that aborts before any network call (`run.rs:88-89`), even though the Census pull itself is handled as non-fatal 100 lines later. Two different opinions about the same source, 100 lines apart.
5. What isn't there: no retry, no backoff, no rate limiting anywhere in the ingest. The concessions to upstream throttling are a User-Agent, an optional Socrata `X-App-Token` from `NYC_APP_TOKEN`, and a 90-second timeout. A single 429 on a fatal source kills the run.
6. Defensible under a one-shot batch model — a human is watching, and rerunning is free. Indefensible the moment it goes on a schedule. Name the exact line where that changes.
7. The observability gap: warnings go to stdout, unstructured, on a developer's laptop. If the JustFix source quietly 404s, 250 cards flip to "unverified" and nothing anywhere records that it happened.

**Evidence:** storage-and-ingest error-policy claims; `run.rs:38-55, 88-89, 184-194, 262-267, 286-291`.

**Hardest question:** *"Your non-fatal path converts an upstream outage into the word 'unverified' on every card in the product. That's the honest output — but how does anyone find out it was an outage rather than the truth?"*

---

## PART IV — THE LEDGER

### Chapter 13 — What Thirteen Tests Buy
**Question it answers:** Is example-based testing enough for a number you put in front of a tenant, and if not, which specific test is missing?

**Beats**
1. The inventory: 13 example-based tests in the scored core (1 in `model`, 12 in `scoring`), all passing, plus 10 in `store` and roughly half of `sources.rs`. Zero property tests, zero doc-tests, workspace-wide.
2. What's pinned, and pinned well: the log-curve reference points, the linear-rule regression test named for the bug it prevents (`neighborhood_discriminates_dense_blocks`), both Census sentinels, `closed_violations_are_ignored`, `migrate_is_idempotent_on_existing_db`. These tests document intent better than the doc comments do — and unlike the doc comments, they cannot go stale silently.
3. What isn't pinned: that the weights sum to 1.0; that `neighborhood_score` is monotonically non-increasing in `c`; that `total_score ∈ [0,100]` for arbitrary `u8` inputs; three of the seven curve points; future-dated violation years; and **the `_ =>` fallthrough on an unknown violation class** — which is precisely the silent-corruption path from Chapter 6.
4. The recency cliff, untested and probably unnoticed: `if current_year - v.year <= 2 { 2 } else { 1 }` means a class-C violation dated 2 years ago costs 30 condition points and the same violation dated 3 years ago costs 15 — a 6.75-point swing on the headline number triggered by which side of a January 1st a year-granular field fell on.
5. Same line, other direction: `v.year > current_year` gives a negative difference, which is `<= 2`, so future-dated violations are treated as recent. Probably the right default; nothing says so and nothing tests it.
6. Integer discipline, stated correctly. **Correction:** the claim that `u32` buckets and an `i32` accumulator are "two different integer widths for the same quantity" is wrong twice — they are the same width and differ in signedness, and they are not the same quantity (buckets count violations; `penalty` accumulates weighted severity and is signed so `100 - penalty` can go negative before clamping). The real observation is that `penalty += base * recency` is plain arithmetic, not `saturating_add`.
7. **Correction:** "every scoring function ends in a defensive clamp" is false. Two of the six have no clamp and no cast at all (`access_likelihood`, `rent_fairness`). There are four `.clamp` calls; the one at `:57` bounds the *penalty*, not the score, and the cast on the next line — `(100.0 - penalty) as u8` — is an unguarded truncating float-to-int cast, correct only because the previous line rounded. It is the one place the crate breaks its own idiom.
8. `proptest` is one dev-dependency line and would cover more input space than the twelve hand-written cases combined. Say what each of four properties would have caught.

**Evidence:** domain-and-scoring test claims, grill risks #8, #9, #11; refutations #3 and #5; storage-and-ingest test counts.

**Hardest question:** *"Name the single missing test whose absence could let a wrong number reach a tenant with no error anywhere."* (The unknown-violation-class fallthrough. It is one `assert_eq!`.)

---

### Chapter 14 — Honesty Is a Type-System Problem
**Question it answers:** If the thesis is right, what is the ordered list of changes — and which one would you refuse to make?

**Beats**
1. Restate the thesis against the evidence now on the table. Every place this codebase told the truth reliably, a type, a guard, or a test was enforcing it: `Option<bool>` → three hedged sentences; the `> 0` filters → no fabricated percentage; the widened bbox → no clipped point; the regression test → no silent return of the linear rule. Every place it drifted, a `String`, a bare `u8`, or a prose invariant was carrying the meaning: the stale status doc, the collapsed legal tri-state, the uncited weights, the unenforced `rent_stabilized`/`rent_stab_units` pairing papered over with `unwrap_or(0)` — which renders, verbatim, *"Likely rent-stabilized — 0 units on the latest NYC DOF record (2024)."*
2. The ledger, ordered by trust bought per line changed:
   - `enum ViolationClass` + `FromSql` — kills four `_ =>` arms and makes the stale-doc class of bug impossible. ~6 lines.
   - One test on the unknown-class fallthrough. ~3 lines.
   - `Score(u8)` newtype with a smart constructor — collapses four clamps into one and makes `ScoreBreakdown { total: 200, .. }` unconstructible, which today it is not.
   - `Option<(f64, String)>` or a `Verdict` enum for `rent_fairness`.
   - `&'static str` (or an enum with `score()`) for `access_likelihood`.
   - `PRAGMA user_version` + a provenance manifest in `meta`.
   - `proptest` on monotonicity, range, and weight-sum.
   - Per-unit normalization for `complaints_311` — the largest correctness win and the largest amount of work.
   - `enum Pillar { Scored(u8), Unverified }` — the only change that makes the frontend's existing claim true.
3. The one to refuse, argued rather than asserted: `Pillar` changes the JSON wire shape and the frontend contract for a distinction that today exists in the copy. Make the case both ways and commit to an answer.
4. The meta-lesson for the reader, generalized off HouseCheck: in a system whose job is to be believed, the type system is the only documentation that cannot lie, and the test suite is the only comment that cannot go stale.
5. Close on the sentence the codebase already wrote for itself, at `scoring:89-91`: the reason the guard exists is that without it the flagship feature *"would print a confident, fabricated number."* That is the whole book, and it was already in the source.

**Hardest question:** *"You've listed nine changes. If you get exactly one, which, and what is your argument that it beats the other eight?"*

---

## Appendix — Corrections and Method
Short, and load-bearing for credibility with this audience. State the verification method (both mapped files read in full, cross-referenced files opened, repo-wide greps, the log curve recomputed independently outside Rust, `cargo test` run, `git log -S` for the 311 recalibration), then list the five corrections the first analysis pass got wrong, each in one line: `api/src/main.rs` is 3028 lines not 2600; the "six combinations" arithmetic; "every function clamps"; the `model:72`/`:71` off-by-one; the "different integer widths" characterization. Then the four imprecisions carried as caveats in-chapter: `access_likelihood` has four outcomes not three branches; the recency rule is one-sided; the doc's slope arithmetic doesn't close; the sentinel guard is unreachable from HTTP. A book that opens by claiming honesty is the organizing constraint and then hides its own errata will be read exactly as hypocritically as it deserves.

---

## Diagrams — placement and exact content

**If only three ship, ship D1, D3, and D4.** They carry the thesis.

**D1 — Chapter 1, "The trust boundary map."** Left-to-right dataflow: 8 upstream sources → `ingest` (one-shot, developer laptop) → `data/housecheck.db` (immutable artifact) → Docker image layer → `api` (read-only open) → frontend. Annotate every arrow with *what can be unknown at that point* and *whether the type crossing that arrow can express it*. Mark two red X's: `legal_score`'s `== Some(true)`, where `Option<bool>` collapses to a `u8`; and the `ScoreBreakdown` boundary, where five bare `u8`s leave for the wire. Everything else in the book is a zoom into one of those two X's or into the arrow that prevented a third.

**D2 — Chapter 3, "Pillar assembly."** Call graph, not a box diagram. `scoring` exports six free functions and no orchestrator, so draw the six as loose nodes. Two call-site boxes — `card_for` (`api:317-321`) and `buildings_handler` (`api:423-427`) — each drawing its own five arrows into its own `ScoreBreakdown`, to make the duplication visible. `rent_fairness` sits off to the side, wired only to `/rent-fairness`, with a dashed arrow labeled *"described to the public as pillar #3 — `docs/CASE-STUDY.md:26` — not in `total_score`."* Weights on the four scored edges; a small annotation that the weights have no citation.

**D3 — Chapter 4, "Two curves." The book's money figure.** One chart, x-axis log-scaled 0→10,000 complaints, y-axis 0→100 score. Plot the old linear rule (−2/complaint, floor 40 at c=30) and the new log rule. Both flatline at 40; annotate the two saturation onsets, **c=30** and **c=1069**, and shade the gap as "the improvement." Filled dots for the six test-pinned points (0, 10, 100, 262, 500, 3209); hollow dots for c=55, c=1000, c=1069, captioned "verified by independent computation; not covered by the suite." Caption line: `penalty = clamp(round((ln(1+c) − 4.0).max(0) × 20.0), 0, 60)`.

**D4 — Chapter 5, "Where the tri-state dies."** Three horizontal lanes entering from the left — `Some(true)`, `Some(false)`, `None`. Top path through `Stabilization::from_units`: three lanes stay separate and exit as three distinct tenant-facing sentences. Bottom path through `legal_score`: `Some(false)` and `None` merge at the `== Some(true)` check and exit as a single `60`. Both paths end at the same rendered card. This one picture proves "unverified is not scored as a zero — it is scored identically to a confirmed negative," and it should be reproduced at reduced size in Chapter 14.

**D5 — Chapter 7, "The guard that cannot fire."** Reachability diagram of the `rent_fairness` path. Three sequential filters drawn as gates: `parse_census_medians` dropping `median <= 0` at ingest; the store's `WHERE ... AND median_gross_rent > 0`; the handler's 404 at `api:462`. The `tract_median <= 0` branch inside `rent_fairness` is drawn downstream of all three and shaded as unreachable-from-HTTP, with its only two live inbound edges labeled "unit test (0)" and "unit test (−666666666)."

**D6 — Chapter 8, "Artifact lifecycle."** Vertical pipeline: developer machine → `cargo run -p ingest -- --real` → `remove_file` → `open_db` → `migrate` → 8 pulls → `data/housecheck.db` → docker build context (annotate `.dockerignore`: drops `-wal`/`-shm`, explicitly keeps `.db`) → stage 2 `COPY` → `ENV HOUSECHECK_DB` → running container. One step boxed in red: the artifact is gitignored and exists on exactly one machine. That red box is the chapter's hardest question, drawn.

**D7 — Chapter 9, "Schema over time."** The four tables, with v1 columns in one shade and the four `ALTER`-added columns (`latitude`, `longitude`, `restaurant_grade`, `rent_stab_units`) in another, positioned after the closing paren exactly as the shipped `.schema` renders them. Tag each column NOT NULL / nullable, and flag the two whose NULL is load-bearing product semantics against `good_cause`/`has_elevator`, which are NOT NULL and therefore cannot say "unknown." Draw the absent `violations.bbl → buildings.bbl` foreign key as a dashed non-edge labeled "not enforced."

**D8 — Chapter 10, "BBL identity funnel."** Top half: three inbound formats converging on one 10-digit TEXT key — PLUTO's float string via `norm_bbl` (label the edge `f64` round-trip), HPD's `boroid`/`block`/`lot` triple via `format!("{boro}{block:05}{lot:04}")`, and DOB's raw string on an edge labeled "no normalization." Bottom half: the over-fetch funnel — borough + block chunks of 500 → all rows on those blocks → `bbl_set` membership filter → kept rows, with the discarded neighbors drawn falling out the side.

**D9 — Chapter 11, "One building's geospatial fan."** A single building's lat/lon at center. Show the widened bounding box (`1/cos(lat) + 1e-4`) drawn *outside* the true 150 m circle so the over-inclusion is visually obvious. Three arrows out to the three materialized scalar columns. Then the punchline as a fourth arrow: `near_ada_subway_m` exits to the frontend card and **does not** enter `access_likelihood`, whose four actual inputs are boxed separately.

**D10 — Chapter 12, "Failure policy matrix."** A table with a flow element, not a pure table: 8 sources down the side; columns for fatal/non-fatal, the code line, and — the column that makes it worth drawing — *what the tenant sees when this source fails*. The three non-fatal rows each get an arrow tracing the degradation all the way to the rendered string ("unverified", grade omitted, rent comparison absent). The five fatal rows all point at one terminal box: "no artifact; nothing deploys."