# Design Session — The Storage Layer at City Scale

**The question:** HouseCheck covers one community district. Citywide coverage plus violation
descriptions was believed to break the architecture. What replaces it, and can it stay free?

**Dated:** 2026-08-09. **Status:** recommendation, not yet built.

---

## The headline, up front

**The earlier conclusion was wrong, and by roughly an order of magnitude.** The "coverage cliff
at ~14,500 buildings" assumed descriptions would be stored as raw text. Stored in blocks they
compress **6.6×**, because every one begins with a housing-code statute reference. (The blocking
matters and is not a detail — see the addendum.)

Measured, not estimated:

| Input | Value | How it was obtained |
|---|---:|---|
| Total violation records citywide | 11,156,924 | `count(1)` on HPD `wvxf-dwi5` |
| **Open** violations — what the card lists | **2,858,719** | `count(1) where violationstatus='Open'` |
| Distinct buildings with any violation | 222,433 | `count(distinct bbl)` |
| Description length | 175 chars mean | 1,600 rows sampled across 8 offsets |
| Description after zlib -9, blob | 17.7 bytes/row (9.9×) | compressed the same 1,600 rows — **not randomly accessible, see addendum** |
| Description, **blocks of 128 rows** | **26.6 bytes/row** (6.6×) | the figure the design actually uses |
| Storage per violation row today | **48 bytes** all-in | 26,306 rows in a 1.21 MB artifact |

**Only 25.6% of violations are open.** The product lists open violations, so the working set is
2.86M rows rather than 11.2M — the first thing that makes this tractable.

### What citywide actually costs

| Component | Uncompressed | Block-compressed |
|---|---:|---:|
| Violation rows (2.86M × ~43 B) | 123 MB | 123 MB |
| Descriptions (2.86M × 175 B vs 26.6 B) | 500 MB | **76 MB** |
| Buildings (222,433 × ~300 B) | 67 MB | 67 MB |
| **Total** | **~690 MB** | **~266 MB** |

At ~266 MB, **a citywide artifact still fits a small read-only machine.** The design does not have
to be replaced. That is a materially different situation from the one assumed in the solution
sprint, and it was one measurement away the whole time.

*Confidence note: per-row cost is measured on a Bed-Stuy district where violation density is
above the city average, so the row estimate is more likely high than low. The compression ratio
is measured on a spread sample with 128 distinct statute prefixes and 96% unique full texts, so
it is not an artifact of a clustered sample.*

---

## Options

### A. Compress descriptions in the existing baked artifact ✅ recommended

Store descriptions in compressed blocks of ~128 rows and decompress the block a card needs.
Measured at 31 microseconds per block in Python zlib — under 3% of the 2.2 ms card budget, and
several times cheaper in Rust with zstd. Everything else stays exactly as it is.

- **Cost:** ~266 MB artifact. A 512 MB Fly machine is roughly $3–4/month.
- **Keeps:** read-only file, no database server, no write path, no secrets, the provenance stamp,
  and the whole "the database is the deployment" property.
- **Costs:** descriptions are no longer greppable by raw SQL; a dictionary must be versioned with
  the artifact.
- **Change size:** compress at ingest, decompress at read. Two functions, contained.

### B. Shard by borough or community district

One artifact per district, served by the district a lookup resolves to.

- **Keeps** everything tiny and free.
- **Breaks the thing the product is moving toward.** Landlord portfolio and median-time-to-close
  are cross-building queries, and a landlord's buildings do not respect district lines.
- Rejected on those grounds, not on cost.

### C. SQLite over HTTP range reads from object storage

Keep one large SQLite file on R2 or S3 and read only the pages a query touches, over HTTP range
requests, instead of shipping the file inside the image.

- **Keeps** the read-only, no-server property. Storage is effectively free (~$0.015/GB/month).
- **Scales past any size we will ever have.**
- **Costs** a custom VFS in Rust, and per-query latency becomes 2–3 network round trips instead
  of a page-cache hit, turning a 2.2 ms card into tens of milliseconds.
- **Verdict: the documented escape hatch.** Correct if the artifact ever passes ~800 MB. Wrong
  now, because it trades a measured 2.2 ms for complexity we do not yet need.

### D. Postgres

- **Solves** growth permanently and enables per-user state later.
- **Deletes the property the product sells:** "no database server to breach and no write path to
  abuse" is on a slide, in the PRD, and in the build book. It is also a monthly bill whether
  anyone visits or not, which is in direct tension with staying free.
- **Verdict: not for coverage.** Revisit only when per-user state is actually being built, since
  that is the requirement that genuinely needs a writable database.

### E. Pre-rendered static JSON per building on a CDN

222,433 files, no server at all.

- **Cheapest possible** and effectively infinite scale.
- **Breaks** search-as-you-type, any aggregate, and the agent's ability to query across
  buildings. Rebuilding 222k files per ingest is its own problem.
- Rejected.

---

## Recommendation

**Option A now. Option C documented as the escape hatch.**

Compress descriptions in place, raise the machine from 256 MB to 512 MB, and keep the
read-only baked artifact. This holds every integrity property the product argues for, keeps the
cost at a few dollars a month, and is a contained change rather than a migration.

**Reject the premise that citywide requires abandoning the design.** It required one
measurement.

### Sequenced

1. Compress descriptions at ingest; decompress on read. Verify the artifact size on the current
   250 buildings first.
2. Ingest one full borough. Measure the real artifact rather than trusting the table above.
3. If a borough extrapolates under ~400 MB citywide, go citywide on a 512 MB machine.
4. If it lands above ~800 MB, build Option C rather than reaching for Postgres.

### What would change this answer

- **Descriptions compressing worse than ~4×** in blocks on the full corpus. Sampled at 6.6×, so
  the margin is real but not vast, and the sample is 1,600 of 2.86M rows.
- **Wanting closed violations as rows** rather than as per-landlord aggregates. That is 11.2M
  rows instead of 2.86M and roughly quadruples everything. Median-time-to-close should therefore
  be computed **at ingest** and stored as a summary — a design constraint that falls directly out
  of this analysis.
- **Building per-user state.** That needs a writable database and re-opens Option D on its own
  merits, not for coverage.

### Open items

- Ingest time and Socrata rate limits at 2.86M rows (~58 paged requests at 50k) are unmeasured.
- Docker image size and deploy time with a ~266 MB artifact are unmeasured.
- Whether a 512 MB machine holds the working set in page cache under real traffic — the current
  2.2 ms per card depends on the database being small enough to sit in cache, and that is the
  number most likely to move.


---

## Addendum — how the compression is done, and should DuckDB replace SQLite?

Added after a second measurement pass. The first version of this document quoted 9.9× and
17.7 bytes per row. **That figure was measured on all 1,600 descriptions concatenated into a
single blob, and a blob cannot be randomly accessed.** The serving path needs one building's
~30 violations, not the whole corpus.

### Compression only survives in blocks

| Scheme | Ratio | Bytes/row | Randomly accessible |
|---|---:|---:|---|
| Whole blob | 9.9× | 17.7 | **No** — the original, unusable figure |
| Per row, no dictionary | **1.3×** | 136.5 | Yes |
| Per row + trained dictionary | 2.6× | 72.2 | Yes |
| **Blocks of 128 rows** | **6.6×** | **26.6** | Yes — one block decompress per lookup |
| Blocks of 64 rows | 5.4× | 32.1 | Yes |

Per-row compression is close to worthless here, because the ratio lives entirely in the
*repetition between* descriptions — every one opens with a housing-code statute reference. Alone,
a 175-byte string has nothing to reference.

So: **store descriptions in blocks of ~128, compressed together, and decompress the block on
read.** Measured cost of that decompress is **31 microseconds in Python zlib**; a card touches
one or two blocks, so 31–63 µs against a 2,200 µs budget — under 3%, using the slower of the two
available languages and the weaker of the two codecs. In Rust with zstd it is several times
cheaper. **Decompression speed is not a constraint on this design.**

A trained-dictionary approach (2.6× measured with zlib's `zdict`) would do considerably better
with real zstd dictionaries, which are built for exactly this shape of data. Worth testing if
block granularity ever becomes awkward, but blocks are simpler and already sufficient.

### SQLite or DuckDB?

The honest answer is **both, at different stages** — and the split falls exactly along the line
the architecture already draws.

| | Serving path | Ingest path |
|---|---|---|
| Shape | point lookup: one BBL, ~30 rows | full scan: 11.2M rows, group by landlord |
| Frequency | every request | once per ingest |
| Wins | **SQLite** | **DuckDB** |

**Keep SQLite for serving.** The hot path is a B-tree point lookup, which is precisely what
SQLite is best at and what the measured 2.2 ms comes from. DuckDB is columnar and vectorised —
built for scanning and aggregating, and comparatively weak at single-row lookups. It also carries
a much larger library and a heavier runtime memory profile, which matters on a 512 MB machine
where the whole point is that the working set sits in page cache. Swapping a measured, working
2.2 ms for a slower path with a bigger footprint would be a downgrade.

**Use DuckDB (or Polars) at ingest.** Computing median-time-to-close per landlord means scanning
all 11.2M violations and grouping — an analytical query, and the exact workload DuckDB is built
for. It also reads Parquet directly and has **FSST** string compression, which targets short
repetitive strings natively. Doing this in SQLite or hand-rolled Rust loops is the wrong tool.

That split reinforces the principle the project already runs on: *everything expensive happens
once, at ingest, on a laptop.* DuckDB is an ingest-time tool that never ships in the image.

### On Rust and compression speed

The instinct is right, the mechanism is slightly different. Compression throughput is set by the
**codec**, not the language — most Rust compression crates bind the same C library the rest of
the world uses (`zstd-rs` wraps libzstd). What Rust actually contributes here is that it can
decompress straight into a reusable buffer with no allocation churn and no garbage-collector
pause, and it can hold a decompressed block borrowed rather than copied. That matters for tail
latency and for a 512 MB memory ceiling — it just is not the reason the bytes shrink.

The codec choice is the decision that matters: **zstd over zlib**, for roughly comparable ratio
at several times the decompression speed, with dictionary support if blocks stop being enough.


---

## Addendum 2 — the other datasets, and how a self-updating ingest is shaped

The first pass sized **one** source. The real product joins many, is meant to cover every
apartment in the city rather than only vacant ones, and is meant to refresh itself weekly without
a human. Sizing every source as stored rows gives a frightening number and the wrong answer.

### Measured inventory

| Source | Rows | Role |
|---|---:|---|
| 311 service requests (`erm2-nwe9`) | **22,080,927** | derived |
| HPD violations (`wvxf-dwi5`) | 11,156,924 | stored (open only) + derived |
| PLUTO tax lots (`64uk-42ks`) | 858,602 | reference |
| HPD registration contacts (`feu5-w2e2`) | 782,024 | reference |
| DOHMH inspections (`43nn-pn8j`) | 294,746 | derived |
| HPD housing litigation (`59kj-x8nc`) | 240,163 | stored |
| HPD registrations (`tesw-yqqr`) | 203,236 | reference |
| Marshal evictions (`6z8x-wfk4`) | 131,913 | stored |

*HPD complaints and complaint-problems were not resolvable at the dataset ids tried, so they are
absent from this table rather than estimated. Their real ids need looking up before anything is
planned around them.*

### The distinction that makes this tractable

**Most sources are inputs to a derived value, not rows a user ever reads.** 311 is the largest
dataset by a wide margin and contributes exactly one number per building: complaint density on a
log curve. Twenty-two million rows collapse to 222,433 values, about 1.7 MB.

| Component | Stored rows | MB |
|---|---:|---:|
| Buildings (PLUTO + registrations) | 222,433 | 63.6 |
| Open violations | 2,858,719 | 117.2 |
| — descriptions, block-compressed | 2,858,719 | 72.5 |
| Housing litigation | 240,163 | 25.2 |
| Evictions | 131,913 | 6.3 |
| Owner linkage | 222,433 | 12.7 |
| 311 density *(derived from 22.1M rows)* | 222,433 | 1.7 |
| Landlord aggregates *(derived from 11.2M)* | ~60,000 | 2.7 |
| Restaurant grade *(derived from 295k)* | 222,433 | 0.8 |
| **Total** | **~7.0M** | **~303 MB** |

**35.7 million source rows become 7.0 million stored rows — a 5x collapse — and the whole city,
every apartment, still fits a 512 MB machine.**

The rule worth keeping: *a source earns per-row storage only if a user reads those rows
individually.* Violations, litigation and evictions do. 311 and restaurant inspections do not.

### Ingest becomes several layers, deliberately

One module per source, each independently runnable, each declaring:

- its dataset id and **incremental key** (`created_date`, `violationid`, and so on)
- its **refresh cadence** — 311 and violations weekly, registrations monthly, PLUTO is annual and
  does not need a weekly pull
- its **schema contract**, so a renamed or dropped column fails the build rather than silently
  producing nulls
- its own provenance: dataset version, retrieval timestamp, row count

Sources stage to Parquet; one compose step reads all staged Parquet, computes the derived values,
and emits the serving SQLite. **This is where DuckDB earns its place** — grouping 22.1M 311 rows
and 11.2M violations is exactly its workload — and where it stays, since it never ships in the
image.

### What "set and forget" actually requires

The scheduling is the easy part. The hard part is that **an automated pipeline which can publish
bad data is worse than a manual one**, so the ordering is build, verify, deploy — never deploy
then discover:

- **Completeness check on every paged fetch.** Already built, after an unchecked `$limit` silently
  dropped half the violations once.
- **Schema drift is a hard failure.** A missing column aborts; it does not null-fill.
- **Row-count sanity band.** A source returning far fewer rows than last week aborts rather than
  publishing a thinner city.
- **Verify the artifact before it ships:** non-zero buildings, scores in range, provenance
  stamped, spot-check known addresses.
- **Keep the previous artifact.** A failed ingest must leave last week's good data serving rather
  than taking the site down. The API already refuses to boot on an empty database.
- **The card keeps stating its own build date**, so a stalled pipeline is visible to users instead
  of silently serving stale data as fresh.

### Risks not yet resolved

- **Socrata throttling** at this volume. An app token raises the limits; it belongs in the CI
  secret store, added by the repo owner, and never in the repository.
- **Weekly wall-clock is unmeasured.** Deltas should keep it modest, but the first full backfill
  of 35.7M rows will not be quick.
- **Address to BBL resolution citywide** is harder than within one district, and a geocoding
  failure becomes a coverage gap rather than a visible error.
- **The two HPD complaints datasets** need their real ids resolved.
