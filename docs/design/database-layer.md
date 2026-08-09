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
