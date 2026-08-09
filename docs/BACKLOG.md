# HouseCheck — Backlog

**The live to-do list.** `ROADMAP.md` and `TASKS.md` are historical capstone artifacts and are
not maintained; this is what is actually open.

**Rule for this file:** every item says *why*, and anything derived rather than measured says so.
An item with no reason is a wish, not a task.

**Last updated:** 2026-08-09.

---

## Committed — the MVP

From `classwork/solution-design-sprint.md`. The single core feature:

> A tenant lawyer opens a building, sees every open violation in the notice's own words with how
> long each has been open, and exports it as a file a stranger can independently verify was not
> altered after retrieval.

- [ ] **Call a Legal Aid housing attorney or paralegal — before any code.**
      Two questions: how long the manual HPD Online pass actually takes, and whether an exported
      file fits how a case is really built. If the count is sufficient, the whole MVP is aimed at
      nothing. Cheapest possible way to be wrong. Also closes open question 1 in
      `classwork/problem-definition-notes.md`.
- [ ] **Ingest: fetch `novdescription`.**
      Measured 100% populated across 800 sampled rows, mean 120 chars. One column on the SoQL
      select in `crates/ingest/src/run.rs`.
- [ ] **Ingest: fetch violation open and close dates.**
      Required for days-open and time-to-close. Free — same rows.
- [ ] **Ingest: stamp dataset version and retrieval timestamp per row.**
      Not bookkeeping. Without it the export's signature attests to a file rather than to a fact,
      which is security theatre. This is what makes the export honest.
- [ ] **Model: extend `Violation`.**
      Currently `{ class, open, year }` in `crates/model/src/lib.rs` — there is nowhere for a
      description to go, so this is a schema change, not just a fetch.
- [ ] **Run one real ingest on the 250 and read the actual artifact size.**
      Arithmetic says ~3.2 MB of text against a 1.3 MB artifact — roughly 3.4×, moving the 256 MB
      ceiling from ~40,000 buildings to ~14,500. That is *derived*. Confirm before it drives a
      decision.
- [ ] **Card: render open violations** — class, raw notice text, days open.
- [ ] **Derived: median days-to-close per landlord**, computed from HPD dates alone.
      The single best value-per-effort item found in the sprint: it gives an operator-level
      credibility signal with no landlord participation, no identity verification and no legal
      exposure.
- [ ] **Export: document plus hash chain, signed Ed25519.**
      Reuses SiteAssure's `entry_hash = sha256(prev_hash + payload_hash)` and Resona's
      `verify_license_with` signing path. Both already exist and both were worked on this cycle.
- [ ] **Public verifier** that takes an exported file and answers valid / tampered.
      The demo moment: hand someone the file, they change one character, verification fails.

---

## Known defects and stated limitations

- [ ] **A count without its meaning can be read backwards.**
      `603 Putnam Avenue` scores `condition 0` with 11 Class A, 22 Class B and **zero** Class C,
      so the card reads "no hazardous violations" next to a floor-level score. Both numbers are
      correct; the sentence reconciling them — *33 open violations, none of them Class C* — cannot
      be rendered because descriptions are not ingested. **Closed by the MVP above.** The deck now
      explains it in the interim.
- [ ] **Class I violations excluded.** 753 records skipped on the curated set. Stated on the card
      and in `/meta`, but not scored.
- [ ] **Multi-source ingest and weekly self-refresh.** See `design/database-layer.md` addendum 2.
      One module per source with its own incremental key, cadence and schema contract; stage to
      Parquet; compose with DuckDB; build then verify then deploy, so a bad ingest never
      publishes. Measured: 35.7M source rows collapse to ~7.0M stored rows, ~303 MB for the whole
      city including 311, litigation, evictions and owner linkage.
- [ ] **No scheduled refresh.** The artifact is a point-in-time snapshot; nothing re-ingests. The
      card states its build date rather than hiding it, but it is still a limitation.
- [ ] **Coverage is 250 buildings, one community district (CD 303).**
      Disqualifying for a daily professional user — a tool covering 0.1% of a caseload does not
      get adopted.

---

## Address resolution — from `design/address-to-bbl.md`

- [ ] **Fix `normalize_address` to expand by position, not presence.** It currently corrupts real
      NYC street names: `ST NICHOLAS AVENUE` becomes `STREET NICHOLAS AVENUE` (**167** PLUTO lots),
      `AVENUE W` becomes `AVENUE WEST` (**403**), `AVENUE N` becomes `AVENUE NORTH` (**744**).
      Street types should expand only in final position and never first; directionals only in
      leading position. Wrong at any scale, so it does not wait for citywide.
- [ ] **Replace the linear scan with an index built at ingest.** `search_curated` loads every
      building and normalises every stored address *per query*. Invisible at 250; roughly two
      orders of magnitude past the 2.2 ms card budget at 222,433. Normalise once at ingest, store
      it, and use FTS5 for prefix/type-ahead.
- [ ] **Put borough in the index key.** 30,035 of 858,602 PLUTO lots (**3.5%**) share an address
      string with another lot. Substring matching compounds it — `FULTON STREET` matches every
      building on Fulton Street in every borough.
- [ ] **Treat an address with no house number as unresolvable.** Already biting at 250 buildings:
      our curated set has 250 buildings and **249** distinct addresses, because `FULTON STREET`
      appears twice against two different BBLs. PLUTO's address field is sometimes just a street.
- [ ] **Verify the Property Address Directory (`bc8t-ecyu`).** PLUTO holds one address per lot, so
      corner buildings and alternate entrances will never match what a user types. PAD is the
      authoritative all-addresses-per-BBL source, but it did not respond on the Socrata tabular
      endpoint and is probably a file download. **Confirm before planning around it** — two
      dataset ids have already turned out not to resolve.
- [ ] **Keep the two failure messages distinct.** "We could not find that address" and "that
      building is outside our coverage" are different, and the current handler already separates
      them. A silent geocoding gap is indistinguishable from a clean building, which is the
      failure mode that makes this the biggest citywide risk.

---

## Engineering debt carried from the audit

- [ ] **Basis-vector weight tests** (audit ledger item 7).
- [ ] **`enum ViolationClass`** to replace stringly-typed classes, plus a schema↔dispatch test
      (ledger item 10).
- [ ] **Extract `crates/agent`** out of the API crate (ledger item 10).
- [ ] **Delete unused `components/ui`** (ledger item 10).

---

## Deferred, with the reason

Not "someday" — each has a specific blocker.

- [ ] **Plain-English violation text.** Blocked on validation, not effort. A wrong rendering on a
      legal-rights tool is exactly what the Guardrails work exists to prevent, so it needs a
      housing lawyer to check a code→condition table. Ships raw until then.
- [ ] **Per-user state — accounts, saved buildings, history.** The gap in all four L2 cycles: a
      grep for `login`, `jwt`, `session`, `user_id` returns no implementation in any of them. On a
      lawyer's tool a saved caseload is client-adjacent data, so getting it wrong is worse than
      not having it.
- [ ] **Citywide coverage.** **Re-scoped 9 August 2026 — see `design/database-layer.md`.** The
      "cliff at ~14,500 buildings" assumed descriptions stored as raw text; stored in blocks of
      ~128 rows they compress **6.6x** (per-row compression manages only 1.3x — the ratio lives
      in the repetition between rows). Measured citywide: 2,858,719 *open* violations (only 25.6%
      of the 11.2M total), 222,433 buildings, and **~266 MB** against ~690 MB raw. **The baked-artifact design does not have to be replaced** — it needs
      compression and a 512 MB machine, roughly $3-4/month. Sequenced in the design doc.
- [ ] **Owner / portfolio dimension.** The record is published per building; owner linkage lives
      in a separate HPD registration dataset, so one landlord's twelve buildings are twelve
      unrelated records. Recorded as a gap in the research notes §4.6; the workflow behind it is
      inferred rather than observed.
- [ ] **MCP server**, so another agent can call HouseCheck as a tool. Pattern already implemented
      in Ziqpu.
- [ ] **The landlord ledger** (Sketch 3). Sell the right of reply, publish the silence. **Blocked
      on legal review — a hard gate, not a caveat.** Also needs identity verification and citywide
      coverage. Parked deliberately, not abandoned.

---

## Open questions

- [ ] **How long does the manual HPD Online pass actually take?** The single most useful number
      not held. One phone call.
- [ ] **Do attorneys already have a private workaround?** If so, Problem 1 is much weaker than it
      looks from outside.
- [ ] **Would a renter act on a bad score at the moment of decision, or sign anyway?**
      The load-bearing unknown of the whole project. Openigloo reached 3M+ renters and still
      pivoted to brokerage, which is at least consistent with "they sign anyway."
- [ ] **Reconcile the rent-burden figures.** The case study cites **51.6%** (RGB 2026); the 2023
      NYCHVS reports **29.5%**. Very likely different populations or thresholds rather than a
      contradiction — but the distinction has to be established before either is cited in front of
      someone who knows the other.
- [ ] **Is the habitability-risk white space actually empty**, or just not visible from public
      search? Absence of evidence is not evidence of absence.
- [ ] **Does HPD's description text need grouping before display?** At 83% distinct, near
      duplicates differ only by room, so 33 violations could render as a wall of near-identical
      lines.

---

## Deck and docs

- [ ] **Look at the deck on a real phone.** Every responsive check so far is DOM geometry —
      no screenshot has ever rendered. Particularly slides 5, 10, 12 and 14, which stack four
      cards each and become long scrolls. If that reads as a slog, collapse the small grey
      sub-note lines behind the main text rather than cutting content.
- [ ] **Confirm `/deck/` serves on Vercel.** `frontend/vercel.json` rewrites every path to
      `index.html` for the SPA. Vercel checks the filesystem before applying rewrites, so a static
      file should win — but it is worth one click after the first deploy.
- [ ] **Set the Vercel Root Directory to `frontend`.** Dashboard-only; cannot be done from here.
- [ ] **Link statutes in the deck.** NY Judiciary Law §§ 478/484 and *FTC v. DoNotPay* are cited
      by name with no link, because their URLs are not verified. Verify, then link.

---

## Related projects

Tracked here because they came out of the same work, not because they are HouseCheck.

- [ ] **SiteAssure — cut a signed installer.** The app is feature-complete (18 commands, all
      wired; zero stubs) and uninstallable. CI is green on Windows and macOS at
      `nessaisling-lab/L2-C2-Solution`.
- [ ] **SiteAssure — confirm push events trigger Actions on the fork.** Manual dispatch works;
      GitHub does not replay push events from before Actions was enabled.
- [ ] **Resona — cut a release build.** Same wall: finished and uninstallable.
