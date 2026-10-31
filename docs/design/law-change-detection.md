# Design Session — Law Phase C: Weekly Change Detection

**The question:** every row in `data/law.json` is a verbatim excerpt of an official page, checked
on one date. Laws get amended and pages move. How does HouseCheck find out that a row it serves
no longer matches its source, without letting a script decide what the law says?

**Answer: a weekly GitHub Actions job that re-reads each source, compares, and opens an issue
for Aisling to verify. It reports. It never changes served law.**

**Dated:** 2026-10-30, revised 2026-10-31 (New York time). **Status:** reviewed by Aisling on
2026-10-30. The shape is approved; the parts listed under "Revised after review" are being
redesigned from research before final approval. Nothing here is built.

---

## Goal

Within a week of an official source changing, Aisling has a list of exactly which provisions
changed, moved, died or are overdue for a hand re-check, with the evidence and the URLs.

## Non-goals

- **No auto-apply.** The job never edits `data/law.json`, the DB or a release. A finding is a
  question for a person.
- **No bypassing bot protection.** No browser user agents, no headless browsers, no unblocking
  services, no retrying with different headers. A protected site is a manual route.
- **No LLM.** Free GitHub Actions minutes on a public repo and free APIs, unless Aisling chooses a
  paid law-data service (she is open to one if it fits; see "Revised after review").
- **Not a check of what Fly serves.** It checks `main`'s `data/law.json`. The deployed artifact
  can be ahead of or behind that until the next deploy.
- **Not a search for new law.** It checks rows that exist. Adding rows stays a hand process.

## Sources and routes

The route is picked from the host of each provision's `url`. A host with no route fails the job
(and a unit test, below), so a new source cannot go unchecked by accident.

| Host | Rows on `main` (8d60d6d) | Route | Signal |
|---|---|---|---|
| `www.nysenate.gov` | `NY-RPL-235-B` (GOL §§ 7-107, 7-108 and RPL §§ 211, 216 captured 2026-10-30, not seeded) | Open Legislation API: `legislation.nysenate.gov/api/3/laws/{lawId}/{location}`, e.g. `RPP/235-B`, `GOB/7-108`, with Aisling's free key | revision date if exposed, then hash of the section text |
| `hcr.ny.gov` | `NYS-HCR-ORA`, `NYS-HCR-TENANT-RESOURCES` | plain GET; PDFs via `pdftotext -enc UTF-8` | passages present |
| `portal.311.nyc.gov` | `NYC311-KA-01036` | plain GET | passages present |
| `data.cityofnewyork.us` | `NYC-OPENDATA-WVXF-DWI5` | plain GET; exact endpoint unverified (see below) | passages present |
| `rules.cityofnewyork.us` | none yet | plain GET | passages present |
| `codelibrary.amlegal.com` | none yet (12 Admin Code sections captured 2026-10-30, not seeded) | manual | `verified_on` age |
| `nycourts.gov` | none yet (the moved HP case pages, captured 2026-10-30, not seeded) | manual | `verified_on` age |
| `nyc.gov` | none | manual (refuses scripted requests, per the `verified_by` comment in `law.rs`) | `verified_on` age |

Phase B adds 17 sources: 12 Admin Code sections, 4 state statutes and the NY Courts HP page,
which has moved and is now two pages, so 18 texts in all.

Every request sends `HouseCheck-law-check/0.2 (+https://github.com/nessaisling-lab/housecheck)`,
at most one request a second per host.

Notes per route:

- **NY Senate API.** The survey got `401` without a key; nothing has been fetched with one yet.
  The nysenate.gov section pages show a "most recent revision" date (seen: GOL § 7-107
  2025-11-21, § 7-108 2022-03-04, RPL §§ 211 and 216 2024-05-03). If the API exposes it, store
  it per provision as a cheap first check: a newer date is a finding on its own and dates the
  amendment. Whether the API has that field, and its name, is **unverified**; confirm against the
  API docs, don't assume it. `NY-RPL-235-B` is a hand copy of the web page, so the first run also
  shows whether the API's text normalizes to the same hash.
- **hcr.ny.gov PDFs.** Phase B's two PDF rows (unmerged branch) need `poppler-utils` on the
  runner; whether `ubuntu-latest` has it is unverified.
- **data.cityofnewyork.us.** The stored text joins the dataset description and one column
  description; the URL Phase A read them from isn't recorded. The Socrata metadata endpoint
  `/api/views/wvxf-dwi5.json` is the likely candidate; unverified.
- **amlegal and nycourts.gov.** amlegal's landing page states "Current through Local Law
  YYYY/NNN, enacted <date>"; a human re-check records that line for the next one to compare.
  Whether anything on either host (a feed, a sitemap named in `robots.txt`, a `HEAD`) answers a
  script without a challenge is **untested**. If nothing does, they are manual only.
- **robots.txt.** Before the first scheduled run, confirm each fetched host's `robots.txt` allows
  the paths used. A disallowed path moves to the manual route.

## Change signal

`model::law::normalize` collapses whitespace, and `text_sha256` hashes the result. The job calls
those same functions, so its idea of "same text" is ingest's by construction.

A stored `text` is not always the whole page. `law.rs` says passages that are not adjacent on the
page are joined by a line holding only `…`. So there are two comparisons:

1. **Whole section** (NY Senate API): `text_sha256(fetched) == text_sha256(stored)`.
2. **Excerpt** (fetched pages): split the stored text on `…` lines, normalize each passage, and
   require each to appear in the normalized page text. This is the rule `insert_law` already
   applies to fact quotes.

Outcomes, one per current provision (superseded rows are never served, so never checked):

| Outcome | Meaning |
|---|---|
| `unchanged` | hash equal, or every passage found |
| `changed` | hash differs, or some passages are missing |
| `page_mismatch` | no passage found at all: more likely a different page or a broken extractor than a new law |
| `moved` | `3xx`; redirects are not followed, and the `Location` is reported |
| `dead` | `404` or `410` |
| `blocked` | a bot-protection page came back; not parsed |
| `unreachable` | timeout, `5xx` or `429` after one retry |
| `not_checked` | manual route, or the API key is missing |
| `due_for_recheck` | manual route and `verified_on` older than 90 days |

The HTML-to-text step strips tags, drops `script` and `style`, and decodes entities. `normalize`
does not fold curly quotes or entities (`New York City’s` in the HPD row), so the extractor must
produce the same characters the original copy had. Unverified until the dry run.

## Job flow

`.github/workflows/law-check.yml`, in the style of `security.yml`:

```yaml
on:
  schedule:
    - cron: "0 7 * * 1"   # weekly Monday 07:00 UTC, after security.yml's 06:00
  workflow_dispatch:
    inputs:
      dry_run: { type: boolean, default: true }
permissions:
  contents: read
  issues: write
```

1. Checkout, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`.
2. `cargo run -p ingest --locked -- --law-check --report law-check.md --texts law-check/`, a new
   `Mode` next to `Law`. `ingest` already has `reqwest` (blocking, rustls); no new HTTP dependency.
   The comparison lives in `model::law`, beside `normalize`, so it is testable with no network.
3. For each current provision: route, fetch, classify, compare. Then list manual rows that are due.
4. Write `law-check.md` (the issue body) and one stored and one fetched text per changed row.
   Exit `0` when it ran, findings or not. Exit non-zero only when the checker itself is broken:
   `law.json` doesn't parse, or a host has no route.
5. For each `changed` whole-section row, `git diff --no-index --word-diff=plain` of the two texts,
   one sentence per line, so a reflow can't show as a change. Git is on the runner already.
6. Upload `law-check/` with `actions/upload-artifact`, and write the report to
   `$GITHUB_STEP_SUMMARY`.
7. If there are findings and it is not a dry run (scheduled runs never are): edit the body of the
   open issue labelled `law-check`, or open one if none is open. One issue at a time.

## Output

An issue, not a PR. An issue changes nothing in the repo, and a bot-written PR invites merging
fetched text without the check this job exists to prompt. Aisling's fix for a real change is an
ordinary PR that edits `data/law.json` (see "Applying a confirmed change"). Illustrative body: the "Changed" item is
what the planted fixture in Testing must produce; the rest shows the format, with placeholders.

```markdown
## Law check 2026-11-02: 1 changed, 0 dead or moved, 1 due for re-check

Nothing here has changed what HouseCheck serves. Each item needs a person to verify it.

### Changed
**NYC311-KA-01036**: https://portal.311.nyc.gov/article/?kanumber=KA-01036
1 of 4 passages not found. Its sentence that is not on the page:
> Between 10 PM and 6 AM, the inside temperature must be at least 62 degrees Fahrenheit at all times.
Fact `heat.night.min_f` (62): its quote is no longer on the page.
Fetched text: workflow artifact `law-check`, file `NYC311-KA-01036.fetched.txt`.

### Dead or moved
(none)

### Due for manual re-check (open in a browser)
**<id>**: <url>. Last verified <verified_on>. Last recorded: "Current through Local Law <...>".

### Not checked this week
(none)

### Evals
- NYC311-KA-01036: a fact quote is gone. If the fact value changes, recordings that state it
  fail replay. **Re-record likely needed** after the fix (billed; Aisling runs it).
```

For an excerpt row, a word diff of one passage against a whole page is noise. So the body narrows
a missing passage to its sentences that are not on the page, and points at the fetched text. A
body over GitHub's size limit is truncated with a pointer to the artifact.

**Dead links.** On 2026-10-30 (New York time; 2026-10-31 UTC) the NY Courts HP Actions URL
`https://www.nycourts.gov/courts/nyc/housing/hpactions.shtml` had become a `404` (moved under
`/new-york-city-housing-court/`). A person found it: the row isn't seeded yet and nycourts.gov is
manual. The job reports `404`s and redirects on fetched hosts; on protected hosts the re-check
list carries the exact URL for a person.

**Evals line, worked out per finding** (the job checks each fact's quote against the fetched text,
so it can tell these apart). `agent_version` hashes the system prompt, the tool schemas and the
model (`crates/api/src/main.rs`), and `lookup_law`'s topic enum is `TOPICS`. So a fix that adds or
renames a topic stales every recording: **re-record needed**. A changed fact value leaves
`agent_version` alone, but the fixture DB loads the real `law.json`, so a recorded answer stating
the old value fails replay: **re-record likely needed**. Changed wording with every fact quote
intact still replays, against turns written for the old wording: **her call**.

## Secrets handling

- One secret, `NYSENATE_API_KEY`, a repository Actions secret. Aisling sets it in the repo
  settings, or with `gh secret set NYSENATE_API_KEY`, which prompts for the value.
- It reaches only the check step, via `env:`. It is never a command-line argument, so it is not in
  a process list or a logged command. The binary reads it from the environment.
- The API takes the key as a query parameter, so URLs carrying it are never printed. The step
  starts with `echo "::add-mask::$NYSENATE_API_KEY"` on top of GitHub's own secret masking. No
  `set -x`.
- The binary logs provision id, host and path only. `reqwest`'s error text includes the request
  URL, so errors are printed via `reqwest::Error::without_url`.
- Triggers are `schedule` and `workflow_dispatch` only, so a fork's PR never runs with the secret.
- A missing key makes the NY Senate rows `not_checked`; it doesn't fail the run. Artifacts hold
  public law text only: no headers, no URLs with queries.

## Failure modes

| Failure | Behaviour |
|---|---|
| API or site down | one retry, then `unreachable`; listed under "Not checked", never as unchanged |
| Rate limit (`429`) | wait once, honouring `Retry-After` up to 60 s, then `unreachable`. The API's limits are unverified; the load is one request per fetched row a week, a few dozen at most |
| `404` / `410` | `dead`, with the URL for a browser re-check |
| Redirect | `moved`, with the `Location`; not followed |
| Cloudflare page instead of content | `blocked`; not parsed, not retried with other headers. Markers to confirm against a real response: `server: cloudflare` with `403`/`503`, a `cf-mitigated` header, a "Just a moment..." title. A protected host may serve the challenge in place of a `404`, so a dead page there is found only by the human re-check |
| Extractor drift (site redesign, new `pdftotext`) | shows as `page_mismatch`, not `changed` |
| Schedule silently stops | GitHub disables scheduled workflows in a public repo after 60 days with no repository activity. No issue then means no run, not no change. Re-enable it from the Actions tab |

## Testing

House rule: every fix ships with a check that fails without it.

- **Comparison unit tests** in `model::law`, like `hash_ignores_reflow_but_not_words`: reflowed
  text is `unchanged`; `62` to `64` is `changed`; one passage altered is `changed` and names that
  passage; no passage found is `page_mismatch`.
- **Planted altered fixture.** A saved copy of the KA-01036 page and a copy with `62` changed to
  `64`, under `crates/ingest/tests/fixtures/`. On the altered copy the check must flag
  `NYC311-KA-01036`, name the 62°F sentence and `heat.night.min_f`. On the original it must flag
  nothing. Offline, in normal CI.
- **Blocked page.** A minimal synthetic challenge response (labelled synthetic) must classify as
  `blocked` and never reach the comparison.
- **Every row has a route.** A test maps each `url` in `data/law.json` to a route and fails on an
  unknown host. A row from a new site fails CI until someone picks its route.
- **The key never leaks.** A request with key `SECRET-TEST` to an unroutable address. The printed
  error must not contain it.
- **Dry run.** `workflow_dispatch` with `dry_run: true` writes the body to the step summary and
  opens nothing.

## Rollout steps

1. Aisling approves the revised doc (her first answers are below).
2. Build `--law-check` and its tests. One PR, CI green, no workflow yet.
3. Aisling runs it locally once, free, with the key read the way she reads her other keys. On
   today's seed the expected result is no `changed` rows. Anything else is an extractor bug,
   fixed before step 5.
4. Check `robots.txt` on each fetched host, and the API docs for a revision-date field.
5. She adds `NYSENATE_API_KEY`. Merge the workflow with `workflow_dispatch` only, run it once
   with `dry_run: true`, and confirm the log shows ids and hosts only. Then turn on the cron.
6. After four runs, review false positives and adjust the extractor, not the comparison.

## Aisling's answers (2026-10-30)

HouseCheck is a nonprofit, donation-supported project. Every date the job writes, and its
schedule, is in New York time (America/New_York).

| # | Question | Answer |
|---|---|---|
| — | Report, never auto-apply | Approved |
| — | NY statutes through the Senate API, key kept out of logs | Approved, with the least data and traffic possible (see "Revised after review") |
| — | Plain re-reads of pages that answer simple requests | Approved |
| — | amlegal and nycourts.gov as manual re-checks | Acceptable for testing only. Wanted: change-only detection that asks "what changed since last time?" instead of re-reading pages |
| — | Dead and moved pages reported | Not enough: the job must look for the replacement or newer version, never just drop a source |
| 1 | Issue or PR? | One rolling issue |
| 2 | Re-check every 90 days? | **No.** The cadence must come from how fast these laws take effect and how fast tenants must act, allowing for paperwork lag (a few days to gather, about five business days to process) |
| 3 | Edit in place or supersede? | Add a new row and mark the old one superseded. Keep **at most two** superseded versions per provision, so a change can be reverted one or two steps. Keep a **permanent changelog** of every change |
| 4 | Serve the API's statute text? | Yes. Text that visibly comes from the official source is part of the proof |
| 5 | Revision date and amlegal "current through" fields | Yes |
| 6 | Re-check the legal-help numbers and hours weekly | Yes |
| 7 | Monday 07:00 UTC | A fixed New York time instead |

## Applying a confirmed change

A change Aisling confirms is an ordinary PR that edits `data/law.json`:

1. The new text becomes a new provision row with its own `verified_on`, and its facts move to it.
2. The old row gets `superseded_by` and stays in the file, unserved, so a revert is a one-line
   change. `law_fact.key` is a primary key across all provisions, so the old row's facts are
   dropped from the current set rather than kept under the same keys.
3. A provision keeps at most two superseded versions. Adding a third removes the oldest from
   `law.json`; its text stays in git history and its entry stays in the changelog.
4. `data/law-changelog.json` is append-only and never pruned. One entry per change: provision
   id, old and new `text_sha256`, the source's revision date or "current through" line, the
   date it was confirmed (New York time), who confirmed it, and the check-issue link. A test
   fails if a superseded row has no changelog entry.

## Revised after review

Research under way, results in this section before final approval:

1. **Cadence.** Set from the enactment-to-effect lag of NYC local laws, NY state laws and agency
   rules, and from the tenant deadlines in the stored law, with the worst-case warning a tenant
   gets after paperwork lag.
2. **Change-only checks, in the style of a cursor-based delta sync.** Each source keeps a cursor:
   the source's own change feed where one exists, otherwise HTTP `ETag` / `Last-Modified`. A
   quiet week costs one small request per source, not one page per row. Only rows whose source
   reports a change are fetched and compared. A periodic full pass compares everything, and a
   planted change the cursor path misses must be caught by it, the way a broken cursor fails
   its test.
3. **Dead or moved pages.** Follow the redirect or find the replacement page, record the last
   good archived copy, and report both for a person to confirm. A challenge page means
   "unknown", never "dead".
4. **Paid full-text services.** Whether any service sells the relevant law as a downloadable
   corpus with change tracking, at a price that fits a nonprofit.
5. **New York time** for the schedule and every date the job writes.
6. **Licence.** Open Legislation's CC BY-NC-ND 3.0 notice, read for a nonprofit, donation-funded,
   Apache-2.0 project. A question for a lawyer; the design only records it.
