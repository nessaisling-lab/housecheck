# Design — Citywide ingest and storage

**Dated:** 2026-10-30. **Status:** proposed, not built. Follows `database-layer.md`.

## Decided (Aisling, 2026-10-30)

- **Keep every A/B/C violation**, open and closed, so repair-speed history stays whole.
- **Local first.** Build and run on her machine; hosting is sized later from the measured file.
- **All five boroughs.** The Brooklyn hardcodes come out.
- **Stage page by page into a scratch file**, then write each building through the existing
  compression code.
- **SQLite and DuckDB together, in Rust:** DuckDB does the ingest work, SQLite serves.
- **Neighborhood context by ZIP code and neighborhood name**, not by distance from the
  building.

---

## How big is it all together

Measured on real rows, 2026-10-30, then multiplied out to the city (10,547,697 violations,
767,563 residential lots).

| Piece | Today's layout | Proposed |
|---|---:|---:|
| Violation rows + descriptions | 1.11 GB | **0.38 GB** |
| Buildings | 79 MB | 63 MB |
| Address search index, borough in the key | none (search scans every row) | 40 MB (B-tree; FTS5 if substring search is needed, not measured) |
| ZIP and neighborhood tables | none | under 1 MB |
| **Serving SQLite file** | **~1.19 GB** | **~0.48 GB** |
| DuckDB stage file, kept between runs | — | ~0.75 GB (violations 0.72 measured, the rest estimated) |
| Parquet snapshot, one per week if kept | — | 0.20 GB each |
| Full download, first build | ~5 GB of JSON | **~0.36 GB** over the wire with gzip |
| Weekly download after that | — | ~157,000 changed rows |

**On disk locally: about 1.2 GB**, plus 0.2 GB a week if snapshots are kept. D: has 317 GB
free. Zipping the whole serving file with zstd shrinks it a further 2.34× (~0.2 GB), which
matters later for moving it, not for running it.

**How it was measured.** Violations: every A/B/C row on tax blocks 1000–1099 in all five
boroughs, **115,273 rows over 3,605 whole buildings**, stored each way and measured on disk
after `VACUUM`, minus an empty file with the same schema. The sample averages 32 rows per
building against ~47 citywide, and per-building compression improves with more rows, so the
city figure is more likely high than low. Buildings: one 50,000-row PLUTO page. Weekly
change: rows with Socrata `:updated_at` in the last 7 days, **157,231 (1.5%)**. Tools:
SQLite 3.45.1, DuckDB 2.0.0, zstandard 0.25.0, in a throwaway script outside the repo.

---

## Where SQLite and where DuckDB

| | Ingest (once a week, on the laptop) | Serving (every request) |
|---|---|---|
| Work | scan 10.5M rows, group by ZIP, find what changed | one building, ~30 rows |
| Tool | **DuckDB** | **SQLite** |
| Why | columnar, fast group-by, compresses on its own, writes Parquet | B-tree point lookup; the measured 2.2 ms card comes from this |
| Ships in the API image | **no** | yes, as it does today |

The flow:

1. **Download.** Rust asks Socrata for gzip pages, keyed by `:id`, not by offset.
2. **Stage.** Each page goes straight into a DuckDB file on D:. Memory stays flat, and this
   is the scratch file you approved.
3. **Aggregate.** DuckDB computes the ZIP rates (most of these Socrata can return already
   grouped, see below).
4. **Compose.** Rust reads the stage `ORDER BY bbl` and writes each building into the
   SQLite serving file through `store::insert_violations`, inside one transaction.
5. **Verify, then swap.** Row counts match the server's count, scores are in range, known
   addresses spot-check. The previous file stays until the new one passes.

**The stage file is kept between runs**, and that is the point of it. The first build pulls
everything. After that, a weekly run asks only for rows changed since the last run
(`:updated_at > last_run`), about 157,000 a week, upserts them into the stage, and
recomposes the serving file. That's the weekly update in the agent-answer design, without
re-downloading the city each time.

---

## Dependencies

| Crate | Version (crates.io, 2026-10-30) | Where | Status | For |
|---|---|---|---|---|
| `rusqlite` (`bundled`) | 0.32, already pinned | store, ingest, api | have | the serving file |
| `duckdb` (`bundled`) | 1.20000.1 | **ingest only** | **add** | stage, aggregates, Parquet |
| `zstd` | 0.14.3 | store (written at ingest, read by api and mcp) | **add** | description blocks with a trained dictionary |
| `reqwest` | 0.12, already pinned | ingest | have; **add the `gzip` feature** | 13.7× smaller downloads |
| `serde_json`, `anyhow` | already pinned | everywhere | have | — |
| `flate2` | 1, already pinned | store | **remove** once zstd is in | — |

**Costs, said plainly:**

- `duckdb` compiles DuckDB's C++ source on the first build. That's slow, but only the
  ingest binary carries it. The build time isn't measured yet.
- `zstd` binds the C library. `flate2` was picked for being pure Rust; that matters less now
  that `rusqlite` already compiles C.

**Deliberately not added:**

- `arrow` and `parquet`: DuckDB writes Parquet itself with `COPY … (FORMAT parquet)`.
- `polars`: DuckDB covers the same work.
- `geo` / `rstar` spatial indexes: ZIP grouping removes the distance math.
- The `sqlite-zstd` extension: it would compress whole rows at the cost of loading an
  extension.

---

## Compression, layer by layer

| Layer | Technology | Measured | Use it? |
|---|---|---|---|
| Download | gzip (HTTP `Accept-Encoding`) | 17.7 MB page → 1.29 MB, **13.7×**, and faster | **yes** |
| Stage | DuckDB's built-in compression (dictionary, FSST for strings, RLE, bit-packing), automatic | 68 B/row | automatic, nothing to set |
| Archive | Parquet + zstd | **18.5 B/row**, 0.20 GB for the city | optional weekly snapshot |
| Serving rows | integer BBL and dates (days since 1970), table clustered by building (`WITHOUT ROWID`), no separate BBL index, drop the `year` column (it's in the issue date) | 105 → **58 B/row** | **yes** |
| Serving text | zstd per building + a **trained dictionary** | descriptions 10.4 B/row; all-in **36 B/row** | **yes** |
| Whole file | zstd -19 on the `.db` | 2.34× | only for moving the file |

**zstd alone is not the win.** Per building block it measured 57.6 B/row against zlib's
58.4: the same. The gain comes from the **dictionary**: every HPD description opens with a
housing-code reference, and a small building's block is too short to learn that on its
own. The dictionary is 112 KB, stored once in the file, and versioned with it. It was
trained on half the sample buildings and scored on all of them, so it isn't just memorising
its own test.

**Reading stays fast.** Decompressing one building's block takes **2.3 µs**, measured in
Python, against a 2.2 ms card. Rust is faster still.

**One thing to protect:** dates move from ISO text to integer days, so the read path formats
them back to ISO. The signed export hashes its rows, so a test must prove an export from
the new layout is byte-identical to one from the old.

**Rejected:**

- **ZIPVFS:** a commercial SQLite license.
- **Serving from DuckDB or Parquet:** slower point lookups and a heavier runtime, for no
  gain on a ~30-row read.

---

## Neighborhood by ZIP and name, instead of distance

Every source carries ZIP as a plain column, so no geometry is needed:

| Dataset | ZIP | Neighborhood (NTA) | BBL |
|---|---|---|---|
| HPD violations `wvxf-dwi5` | `zip` | `nta` | `bbl`, missing on 9,757 of 10.5M |
| 311 `erm2-nwe9` (since 2024: 10,415,878 rows) | `incident_zip`, missing on **0.9%** | none | missing on 11.3% |
| PLUTO `64uk-42ks` | `zipcode`, missing on 213 lots; 184 ZIPs | none | yes |
| DOHMH `43nn-pn8j` | `zipcode` | `nta` | yes |

**So: ZIP for the numbers, neighborhood name for the label.**

- **311 by ZIP.** Socrata does the counting: `$select=incident_zip, count(1)`
  `$group=incident_zip` returns about 500 rows instead of 10.4M points (tested). It's
  divided by residential units in that ZIP (PLUTO `unitsres`), because ZIPs differ a lot in
  size. The busiest, 10466, has 278,042 complaints since 2024.
- **Restaurant grades by ZIP.** The share of A grades, grouped by Socrata the same way.
  Display only, as today.
- **Neighborhood name for every building.** PLUTO's 2020 census tract → DCP's
  tract-to-NTA table **`hm78-6dwm`** → e.g. "Bedford-Stuyvesant (East)".
- **Subway stays a distance.** ZIP can't express "nearest accessible station". That's every
  station × 767k buildings, a few seconds by brute force (496 stations, fewer once filtered
  to accessible ones).

**What this changes, honestly:**

- Every building in a ZIP gets the same neighborhood number. A bad block inside a good ZIP
  no longer shows.
- `scoring::neighborhood_score` is fitted to 150 m counts, so it has to be refitted to
  rates. **Every score moves**, including the golden 603 Putnam 27/100, so tests and the
  recorded agent evals that quote scores change with it.
- An add-on for later, not in this proposal: count 311 at the building's own BBL (88.7% of
  rows carry one) for "complaints at this address". Exact, and also grouped by Socrata.

---

## Ingest steps

One new flag, `--citywide`. The pilot path (`--cd 303 --limit 250`) keeps working and keeps
its tests.

1. **Brooklyn comes out:**
   - the PLUTO `borough='BK' AND cd=…` filter;
   - the tract prefix `36047`, which now takes its county from the BBL's borough digit
     (061, 005, 047, 081, 085);
   - Census `county:047`, now five requests, one per county;
   - the borough-plus-block violation query, replaced by HPD's own `bbl` column, with the
     old rebuild from borough/block/lot kept for the 9,757 rows that lack one.
2. **Buildings:** PLUTO residential lots, paged.
3. **Violations into the DuckDB stage:** keyset paging, gzip. A `count(1)` taken at the
   start must equal the rows staged, or the run aborts.
4. **Compose into SQLite:** one transaction, `synchronous=OFF` and `journal_mode=OFF` on the
   file being built (it's rebuilt from scratch, so a crash means rerun). Compact schema,
   zstd with the dictionary.
5. **Context:**
   - ZIP rates for 311 and restaurants;
   - neighborhood names from `hm78-6dwm`;
   - DOB elevators pulled whole (3 pages, not ~3,840 requests);
   - MTA stations and the rent-stabilization CSV unchanged.
6. **Provenance:** `meta` says `coverage = city`, plus the per-source retrieval stamps it
   carries today.
7. **Weekly:** pull rows with `:updated_at` after the last run into the stage, recompose,
   verify, swap.

## Outside the ingest: the API and frontend

These live in `crates/api/src/main.rs` and the frontend. They're sequenced after the file
exists and coordinated with the session that owns that code.

- **`GET /buildings`** scores every building per request; it needs a bounding box or paging.
- **`search_curated`** loads every building per search; it switches to the address index
  above.
- **The coverage map** (`frontend/src/lib/coverage-points.ts`) can't hold 767k points.
- **Wording** across the card and agent still says "the pilot" and one district.

## Open questions for Aisling

1. **All 767,563 residential lots, or the 172,485 with 3+ units?** All is recommended: it's
   today's filter, and it covers 1–2 family rentals.
2. **Is it OK that every score moves** when the neighborhood pillar switches to ZIP rates?
3. **Keep a weekly Parquet snapshot** (0.2 GB each) as history and provenance?
4. **Output folder on D:** — default `D:\housecheck-data\`.
