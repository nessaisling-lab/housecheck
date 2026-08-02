# Chapter 7 — The Database Is the Deployment

> **The question this chapter answers:** Why is a 671 KB SQLite file baked into
> the image the decision everything else is downstream of, and what does it cost?

---

## 1. The artifact

```
data/housecheck.db      671,744 bytes
page_size                     4,096
page_count                      164
freelist_count                    0
journal_mode                 delete
```

Inside it:

| table | rows |
|---|---:|
| `buildings` | 250 |
| `violations` | 13,253 |
| `acs_rent_by_tract` | 41 |
| `meta` | **1** |

One non-primary-key index in the entire schema:

```sql
CREATE INDEX idx_violations_bbl ON violations(bbl)
```

That is exactly right, and it is worth pausing on because it is the kind of thing
that usually goes wrong in both directions. `buildings` and `acs_rent_by_tract` are
looked up by their primary keys, which already have implicit indexes. `violations`
is looked up by `bbl`, which is not its primary key — so it gets one index. There
are no speculative indexes on `year`, on `class`, on `open`. Zero freelist pages
means no churn: the file was written once and never updated in place.

The `Dockerfile` states the design in its first two lines:

```dockerfile
# HouseCheck API — multi-stage build. The serving DB is a read-only artifact baked into
# the image, so the running container needs NO secrets (ingest is done ahead of time).
```

Everything in this chapter follows from that sentence being *mostly* true.

## 2. What the decision buys

`fly.toml` sets `min_machines_running = 0` with `auto_stop_machines = 'stop'`, on a
256 MB shared CPU. The app scales to zero and cold-starts on demand. That is only
viable because of what startup does *not* have to do.

Measured on the real artifact:

```
cold start (exec → "listening")     167 ms
GET /building/{bbl}, warm           2.2 ms
GET /buildings (250 cards, fully scored, warm)   21 ms
```

No connection pool to establish. No external database to reach over a network. No
migration to run against a live schema. No credential to fetch before the first
query. The process opens a file and binds a port.

And the sizing is quietly perfect: SQLite's default page cache is 2 MB. The entire
database is 671 KB. **The whole dataset fits in the default cache**, so after the
first few reads there is no filesystem I/O in the serving path at all. Nobody
appears to have tuned that — it is what happens when the data is small enough, and
the 21 ms for 250 fully-scored buildings is the consequence. Every one of those 250
cards runs `condition_score` over that building's violations, plus three more
scoring functions and a weighted sum.

The security property is the one the Dockerfile leads with, and it is real. The
image contains a binary and a data file. There is no database URL, no password, no
service account. Compromising the image gets you public NYC data that anyone can
download from Socrata.

One correction to the comment, though: *"the running container needs NO secrets"*
is true of the container and false of the product. `/summary` and `/agent/chat`
need `OPENROUTER_API_KEY`, supplied at runtime as a Fly secret. The container runs
fine without it — those two routes return 501 and everything else works — so the
sentence is defensible as written. It just describes the *scoring* service, which
is the part the sentence is really about.

## 3. The decision it forces

`.gitignore:4`:

```
/data/*.db
```

`.dockerignore:12`:

```
# keep data/housecheck.db — it is baked into the image
```

Two ignore files that deliberately disagree, and the disagreement is commented. The
671 KB binary stays out of git history; it must be in the Docker build context.
That is a considered call, and for keeping a repository clean it is the right one.

The consequence is unavoidable and large: **a fresh clone of this repository cannot
build the image.** The `COPY data/housecheck.db` line refers to a file that does
not exist until someone runs the command in the Dockerfile's third comment line:

```
cargo run -p ingest -- --real --cd 303 --limit 250 --out data/housecheck.db
```

The recipe is written down, which is more than most projects manage. But Chapter 4
established what running it actually does: the 311 query asks for 50,000 of 219,199
matching rows with no `$order`, and the subset Socrata returns shifts over time. So
the recipe does not reproduce *the* artifact. It produces *an* artifact, with
different `complaints_311` values and therefore different neighborhood scores and
different totals.

Which means the deployed database is the one object in this system that is neither
in version control nor regenerable. Chapter 2's determinism argument — hand someone
four integers and a snapshot year and let them recompute — holds completely *given
the artifact*. This is where that qualifier lives. You can check the number. You
cannot rebuild the thing the number came from.

## 4. The failure the system cannot see

Now the part that is not a tradeoff.

`AppState::from_path` is four lines (`crates/api/src/main.rs:203-206`):

```rust
let conn = store::open_db(path)?;
store::migrate(&conn)?;
let snapshot_year = get_snapshot_year(&conn)?.unwrap_or(DEFAULT_SNAPSHOT_YEAR);
```

And `store::open_db` (`crates/store/src/lib.rs:7-17`):

```rust
if path != ":memory:" {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
}
let conn = Connection::open(path)?;
```

`Connection::open` is read-write with create-if-missing. Not `SQLITE_OPEN_READ_ONLY`.
So walk what happens when `HOUSECHECK_DB` points at a file that is not there:

1. `create_dir_all` creates the directory.
2. `Connection::open` creates an empty SQLite database.
3. `migrate` runs `CREATE TABLE IF NOT EXISTS` and builds the full schema — empty.
4. `get_snapshot_year` finds no `meta` row, returns `None`, and `unwrap_or` supplies
   `2026`. This is Chapter 2's Leak 1, and here is where it gets its consequence.
5. The server binds and serves.

I ran it rather than reasoning about it. Pointing the release binary at a
nonexistent path:

```
$ curl /health
ok

$ curl /buildings
[]

$ curl /building/3000010001
building not found [HTTP 404]
```

Startup log, in full:

```
INFO housecheck_api: LLM: enabled model=anthropic/claude-haiku-4.5
INFO housecheck_api: listening on 127.0.0.1:8791
INFO housecheck_api: CORS: permissive (local dev); set CORS_ALLOWED_ORIGIN to restrict
```

Three lines. **Not one of them mentions the database.** No warning, no error, no row
count. And on disk afterwards:

```
-rw-r--r--  36864  nope.db
```

It created a 36 KB empty database and served from it.

`/health` returns `ok`, so Fly's health check passes, the machine is marked healthy,
and the deploy is green. Every building returns 404. The map renders empty. A
tenant searching their address is told, with a normal-looking response, that their
building is not covered.

**The system cannot distinguish "deployed correctly" from "deployed with no data."**
For an architecture whose entire premise is that the database *is* the deployment,
that is the load-bearing failure.

## 5. Why every step of that is individually correct

It would be easy to read §4 as carelessness. It is not, and the reason is worth
extracting because it generalises.

`open_db` and `migrate` are shared by two callers with opposite needs.

**`ingest` must create.** It runs against a path that does not exist yet — that is
the point. `create_dir_all` is there because of a real bug, and the doc comment says
so: *"SQLite error 14 ('unable to open the database file') otherwise on a fresh
checkout."* Someone hit that error and fixed it correctly. `migrate` must build the
schema, because nothing else will.

**`api` must not create.** For the reader, a missing file is a fatal deployment
error, and a missing schema means a corrupt artifact.

One function, two contracts, and the writer's contract won — because the writer is
the one that failed loudly during development. The convenience that made `ingest`
work is the same convenience that makes `api` silently wrong, and nothing in the
type signature distinguishes them.

This is the same shape as the `unwrap_or` in Chapter 2 and the `_ => 0` in Chapter 5:
a fallback that is correct for one caller, reached by another, producing a plausible
answer instead of an error.

## 6. The table that holds one row

`meta` has a single row:

```
snapshot_year    2026
```

Chapter 2 traced that row end to end and showed the mechanism works — a fact written
into the artifact at ingest survives into every response, forever, without a clock.
It is the best provenance mechanism in the system.

It carries one fact. It does not carry:

- **when ingest ran** — so nothing can compute how stale the data is
- **that the 311 query was truncated** to 50,000 of 219,199 rows (Chapter 4)
- **that class I violations were excluded** — 187 open ones across 134 of the 250
  buildings (Chapter 5)
- **row counts**, so nothing can assert the artifact is non-empty
- **a checksum**, so nothing can say *which* artifact is running

Every one of those is a `meta` insert at ingest and a read at startup, using a
mechanism that already exists and is already proven. Chapters 4 and 5 each ended
with a remediation that turned out to be "put it in `meta`." This is the chapter
where that stops being a coincidence: the provenance gap is not three separate
oversights, it is one table that was built for exactly this and then used once.

---

## The hardest question a reader can ask of this chapter

> *"The whole architecture rests on the artifact being correct, and you have just
> demonstrated the running service cannot tell whether it has the right artifact —
> or any artifact. Why should anyone trust the number?"*

The demonstration is mine and I am not going to soften it. A deployment with no data
returns `ok` from its health check and 404 from every real query, and nothing
anywhere logs a complaint.

What survives the objection, precisely: **the artifact is honest about what it
contains, and dishonest only about what it lacks.** Every building in that file has
real HPD violations, real DOB elevator filings, real Census medians, real 311
counts, and a score that anyone can recompute from four integers. There is no
fabricated row. The failure mode is not "wrong data" — it is "no data, presented as
complete coverage," and it is the *empty* case that goes undetected, not the wrong
one.

That distinction matters because it tells you the fix is cheap. Four changes, in
order of how much they buy:

1. **Open read-only in the API path.** One line — `OpenFlags::SQLITE_OPEN_READ_ONLY`.
   I tested it against the real artifact before recommending it: reads work, and
   `migrate`'s `CREATE TABLE IF NOT EXISTS` is a successful no-op on existing
   tables, so nothing else has to move. Against a missing file it fails immediately
   with *"unable to open database file"*, which propagates out of `main` and the
   container never starts. This is the Chapter 5 lesson applied to a file handle:
   make the wrong state unrepresentable rather than detecting it. `ingest` keeps the
   read-write open it needs — the two callers get two functions, which is what they
   should have had.
2. **Assert non-empty at startup.** `if building_count == 0 { bail!(...) }`. Catches
   the corrupt-but-present artifact that (1) does not, and turns a green deploy
   serving nothing into a failed deploy.
3. **Fill `meta`.** Ingest timestamp, row counts, dataset IDs, the 311 truncation
   flag, the class I exclusion. Then log them at startup and expose them on
   `/health`. Suddenly a human looking at the running service can see what it is
   serving, and Chapters 4 and 5 get their remediation for free.
4. **Checksum the artifact and log it.** Two lines that answer "which database is
   production actually running?" — a question that currently has no answer at all.

(1) and (2) together are under ten lines and convert the failure this chapter
demonstrated from silent to impossible.

The honest summary of the architecture: baking the database into the image is the
right call for this product, and the measurements support it — 167 ms cold start,
2.2 ms cards, no secrets in the image, scale-to-zero that actually works. What it
trades away is the ability to tell the difference between a good deployment and an
empty one, and that trade was not deliberate. It fell out of one `open_db` serving
both the writer and the reader.

---

*Next: **Chapter 8 — The Ingest Nobody Runs Twice.** What a 700-line pipeline
against nine public datasets actually guarantees, and why "it worked once" is the
strongest claim available.*
