# Design Session — The Storage Layer at City Scale

**The question:** HouseCheck covers one community district. Citywide coverage plus violation
descriptions was believed to break the architecture. What replaces it, and can it stay free?

**Dated:** 2026-08-09. **Status:** recommendation, not yet built.

---

## The headline, up front

**The earlier conclusion was wrong, and by roughly an order of magnitude.** The "coverage cliff
at ~14,500 buildings" assumed descriptions would be stored as raw text. They compress about
**10×**, because every one of them begins with a housing-code statute reference.

Measured, not estimated:

| Input | Value | How it was obtained |
|---|---:|---|
| Total violation records citywide | 11,156,924 | `count(1)` on HPD `wvxf-dwi5` |
| **Open** violations — what the card lists | **2,858,719** | `count(1) where violationstatus='Open'` |
| Distinct buildings with any violation | 222,433 | `count(distinct bbl)` |
| Description length | 175 chars mean | 1,600 rows sampled across 8 offsets |
| Description after zlib -9 | **17.7 bytes/row** (9.9×) | compressed the same 1,600 rows |
| Storage per violation row today | **48 bytes** all-in | 26,306 rows in a 1.21 MB artifact |

**Only 25.6% of violations are open.** The product lists open violations, so the working set is
2.86M rows rather than 11.2M — the first thing that makes this tractable.

### What citywide actually costs

| Component | Uncompressed | Compressed |
|---|---:|---:|
| Violation rows (2.86M × ~43 B) | 123 MB | 123 MB |
| Descriptions (2.86M × 175 B vs 17.7 B) | 500 MB | **51 MB** |
| Buildings (222,433 × ~300 B) | 67 MB | 67 MB |
| **Total** | **~690 MB** | **~240 MB** |

At 240 MB, **a citywide artifact still fits a small read-only machine.** The design does not have
to be replaced. That is a materially different situation from the one assumed in the solution
sprint, and it was one measurement away the whole time.

*Confidence note: per-row cost is measured on a Bed-Stuy district where violation density is
above the city average, so the row estimate is more likely high than low. The compression ratio
is measured on a spread sample with 128 distinct statute prefixes and 96% unique full texts, so
it is not an artifact of a clustered sample.*

---

## Options

### A. Compress descriptions in the existing baked artifact ✅ recommended

Store each description as a compressed blob. Decompress only the rows a user actually expands —
around 30 per card — which is microseconds. Everything else stays exactly as it is.

- **Cost:** ~240 MB artifact. A 512 MB Fly machine is roughly $3–4/month.
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

- **Descriptions compressing worse than ~5×** on the full corpus. Sampled at 9.9×, so there is
  a wide margin, but the sample is 1,600 of 2.86M rows.
- **Wanting closed violations as rows** rather than as per-landlord aggregates. That is 11.2M
  rows instead of 2.86M and roughly quadruples everything. Median-time-to-close should therefore
  be computed **at ingest** and stored as a summary — a design constraint that falls directly out
  of this analysis.
- **Building per-user state.** That needs a writable database and re-opens Option D on its own
  merits, not for coverage.

### Open items

- Ingest time and Socrata rate limits at 2.86M rows (~58 paged requests at 50k) are unmeasured.
- Docker image size and deploy time with a 240 MB artifact are unmeasured.
- Whether a 512 MB machine holds the working set in page cache under real traffic — the current
  2.2 ms per card depends on the database being small enough to sit in cache, and that is the
  number most likely to move.
