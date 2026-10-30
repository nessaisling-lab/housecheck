# Design — Citywide ingest

**Dated:** 2026-10-30. **Status:** proposed, not built. Follows `database-layer.md`.

**Decided by Aisling, 2026-10-30:** keep **every** A/B/C violation, not only open ones, so
repair-speed history stays complete. Build and run locally first; hosting is a later
conversation that starts from the measured artifact size.

---

## Measured inputs (Socrata, 2026-10-30)

| Input | Value |
|---|---:|
| HPD violations, class A/B/C, all statuses | **10,547,697** |
| — of which open | 2,655,915 |
| — of which closed since 2023 (what repair speed reads today) | 2,922,785 |
| PLUTO residential lots (`unitsres > 0`, today's filter minus the district) | **767,563** |
| — with 3+ units (HPD's multiple-dwelling threshold) | 172,485 |
| — in CD 303 alone (the pilot ships 250 of these) | 15,237 |
| 311 requests with coordinates since 2024-01-01 (today's window) | 10,236,190 |
| DOHMH graded inspection rows | 127,151 (24,805 restaurants) |
| DOB elevator devices, whole dataset | 120,688 |
| One HPD page, 50,000 rows, 8 columns | **4.7 s, 18.7 MB** |

From the last row: the full violation pull is ~211 pages, **~17 minutes and ~4 GB** of JSON.
D: has 317 GB free.

## What breaks at city scale today

Each of these is fine at 250 buildings and fails at 767,563:

1. **Brooklyn is hardcoded in four places.** PLUTO `borough='BK' AND cd=…`
   (`sources.rs:65`), tract GEOID prefix `36047` (`sources.rs:84`), Census `county:047`
   (`sources.rs:223`), and violations fetched by `boroid` + block list (`run.rs:237`).
2. **Every violation is held in memory as JSON** before anything is written
   (`get_json_paged` returns `Vec<Value>`). At an estimated ~1 KB per parsed row, 10.5M rows
   is on the order of 10 GB;
   `MAX_PAGED_ROWS = 2,000,000` would also abort the pull.
3. **No write transaction.** `store::insert_violations` runs one autocommit `INSERT` per
   row. 10.5M commits, each one a disk sync, is hours.
4. **Geo joins are buildings × points.** 311 density compares every building to every
   complaint: 767,563 × 10.2M ≈ 7.8 × 10¹² distance calls. Restaurant grades are the same
   shape at 127k points.
5. **DOB elevators are fetched 200 BBLs at a time**: ~3,840 requests where 3 pages would do.
6. **`$offset` paging gets slower the deeper it goes** on Socrata, and 211 pages is deep.

## Proposed ingest

One new flag, `--citywide`. The pilot path (`--cd 303 --limit 250`) keeps working and keeps
its tests.

1. **Buildings:** every PLUTO lot with `unitsres > 0`, all five boroughs, paged. No blend,
   no limit.
2. **Violations, streamed:** page `wvxf-dwi5` (class A/B/C) by key (`:id > last`), not by
   offset. Each page goes straight into a staging table in a scratch SQLite file, so memory
   stays flat. Then read the stage `ORDER BY bbl` and write each building through the
   existing `store::insert_violations`, so the block compression stays as tested. Rows on
   lots outside the building set are counted and reported, not silently dropped.
3. **Completeness check:** `count(1)` at the start, compared with rows staged at the end. A
   mismatch aborts. Replaces the 2M cap for this path.
4. **One transaction** around the bulk writes, with `synchronous=OFF` and
   `journal_mode=OFF` on the output file. It is rebuilt from scratch each run, so a crash
   means rerun, not repair.
5. **Geo joins on a grid:** bucket points into ~150 m cells and compare each building only
   with its own and neighbouring cells. Same answers as today; nine cells cover ~0.2 km² of
   the city's ~780 km², so a few thousand times less work. A test checks grid and brute force
   agree on the pilot fixture.
6. **Census:** five requests, one per county (005, 047, 061, 081, 085), and the tract GEOID
   takes its county from the BBL's borough digit.
7. **DOB elevators:** pull the whole dataset once (3 pages), filter locally.
8. **Unchanged:** rent-stabilization CSV (already filters by BBL set), MTA stations (already
   citywide), law load, provenance stamp. `community_district` in `meta` becomes `city`.

Staging is SQLite, not DuckDB or Parquet as `database-layer.md` suggested: it is already a
dependency, and the stage is read once in key order, which SQLite does well.
*ponytail: add DuckDB only if the compose step measures slow.*

**Output stays out of git**, on D:, e.g. `D:\housecheck-data\stage.db` and
`D:\housecheck-data\housecheck-city.db`.

## What it measures

The run prints, and the doc records afterwards: rows per source, wall clock per stage, peak
memory, and the **artifact size on disk**: total, and split into buildings, violation rows
and description blocks. That number opens the hosting conversation.

## Outside the ingest: needed before the API can serve it

The ingest alone gives a correct artifact, but the API isn't ready for one this size. These
live in `crates/api/src/main.rs` and the frontend, so they're sequenced after the artifact
and coordinated, not folded into this change:

- **`GET /buildings`** scores every building on every request (one violation query per
  building). At 767,563 that is a multi-second response with a huge body. Needs a bounding
  box or paging.
- **`search_curated`** loads every building into memory on every search. Needs an indexed
  lookup: a normalized-address column with an index, or FTS5 with the borough in the key.
- **The coverage map** (`frontend/src/lib/coverage-points.ts`, generated from the shipped
  DB) cannot hold 767k points.
- **Coverage wording:** the card and agent describe "the pilot" and one district.

## Not in this change

The datasets sized in `database-layer.md` addendum 2 that the ingest doesn't read yet
(litigation, evictions, registrations, owner linkage). Citywide here means the same card,
every borough.

## Open questions for Aisling

1. **Which lots count as "the whole city"?** All 767,563 residential lots (recommended; same
   filter as today and covers 1–2 family rentals), or the 172,485 with 3+ units.
2. **Where on D: should the output live?**
