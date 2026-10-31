# Design Session — Law Phase C: Weekly Change Detection

**The question:** every row in `data/law.json` is a verbatim excerpt of an official page, checked
on one date. Laws get amended and pages move. How does HouseCheck find out that a row it serves
no longer matches its source, without letting a script decide what the law says?

**Answer: a weekly GitHub Actions job that re-reads each source, compares, and opens an issue
for Aisling to verify. It reports. It never changes served law.**

**Dated:** 2026-10-30, revised 2026-10-31 (New York time). **Status: approved by Aisling on
2026-10-31**, with her answers recorded at the end. Nothing here is built; building starts after
law Phase B lands.

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
| `www.nysenate.gov` | `NY-RPL-235-B` (GOL §§ 7-107, 7-108 and RPL §§ 211, 216 captured 2026-10-30, not seeded) | Open Legislation API with Aisling's free key: `laws/{lawId}/updates/{cursor}` once per law, then `laws/{lawId}/{location}` (e.g. `RPP/235-B`) only for sections it lists | `activeDate`, then hash of the section text |
| `hcr.ny.gov` | `NYS-HCR-ORA`, `NYS-HCR-TENANT-RESOURCES` | plain GET; PDFs via `pdftotext -enc UTF-8` | passages present |
| `portal.311.nyc.gov` | `NYC311-KA-01036` | plain GET | passages present |
| `data.cityofnewyork.us` | `NYC-OPENDATA-WVXF-DWI5` | `/api/views/wvxf-dwi5.json`, compare `viewLastModified` first | the description |
| `rules.cityofnewyork.us` | none yet | RSS `/feed/` with `If-None-Match`, HPD items only; codified rule text from amlegal's Rules bulk XML | new or changed HPD rule |
| `codelibrary.amlegal.com` | none yet (12 Admin Code sections captured 2026-10-30, not seeded) | official bulk XML at `files.amlegal.com` (below), never the reader pages | `ETag`, then the entry's CRC, then each section's `version` and text |
| `nycourts.gov` | none yet (the moved HP case pages, captured 2026-10-30, not seeded) | manual, monthly (Cloudflare challenges every path, `HEAD` and `robots.txt` included) | `verified_on` age |
| `nyc.gov` | none | manual, monthly (refuses scripted requests, per the `verified_by` comment in `law.rs`) | `verified_on` age |

Phase B adds 17 sources: 12 Admin Code sections, 4 state statutes and the NY Courts HP page,
which has moved and is now two pages, so 18 texts in all.

Every request sends `HouseCheck-law-check/0.2 (+https://github.com/nessaisling-lab/housecheck)`,
at most one request a second per host.

Notes per route:

- **NY Senate API.** The survey got `401` without a key; nothing has been fetched with one yet.
  Each section carries `activeDate`, "the date that particular document was updated"
  ([Laws API](https://legislation.nysenate.gov/static/docs/html/laws.html)); the first keyed
  call confirms it equals the web pages' "most recent revision" (seen: GOL § 7-107 2025-11-21,
  § 7-108 2022-03-04, RPL §§ 211 and 216 2024-05-03). Law text arrives in a weekly batch, so a
  Monday run sees the previous week. The docs state no rate limit and no `ETag`; the update
  routes are the cheap signal instead. `NY-RPL-235-B` is a hand copy of the web page, so the
  first run also shows whether the API's text normalizes to the same hash.
- **hcr.ny.gov.** `If-Modified-Since` gets a `304`, but `Last-Modified` there is a cache date,
  so a `200` only means "compare the passages". Phase B's two PDF rows (unmerged branch) need
  `pdftotext`; the ubuntu-24.04 runner lacks `poppler-utils`, so the job installs it.
- **data.cityofnewyork.us.** The stored text joins the dataset description and one column
  description; the URL Phase A read them from isn't recorded. The Socrata metadata endpoint
  `/api/views/wvxf-dwi5.json` (about 85 KB, no `ETag`) carries the description and a
  `viewLastModified` date for the metadata; whether its text matches the stored row is checked
  on the first run.
- **amlegal.** The reader site (`codelibrary.amlegal.com`) answers scripts with `403`, and the
  job never touches it. The same publisher posts the official bulk XML of the Admin Code, Charter
  and Rules at `files.amlegal.com/pdffiles/NewYorkCity/{Admin,Charter,Rules}/XML.zip`, plain
  nginx with no challenge. Admin Code § 7-111 requires the city to publish these codes for bulk
  download in machine-readable form and refresh them within four weeks of a change
  ([LL 2020/82](https://intro.nyc/local-laws/2020-82)). The Admin zip is 65.5 MB, but the job
  reads only pieces of it (see "Change-only checks"). Checked 2026-10-31: `If-None-Match` with
  the last `ETag` returns `304` and 0 bytes.
- **nycourts.gov.** Every path, including `robots.txt`, `sitemap.xml` and a `HEAD`, returns
  `403` with `cf-mitigated: challenge`. A dead page and a live one look the same to a script, so
  these rows are manual only.
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
| `moved` | `3xx`; the `Location` is reported and followed up to 5 hops on the same site (see "Dead or moved pages") |
| `dead` | `404` or `410` |
| `blocked` | a bot-protection page came back; not parsed |
| `unreachable` | timeout, `5xx` or `429` after one retry |
| `not_checked` | manual route, or the API key is missing |
| `due_for_recheck` | manual route and `verified_on` older than 30 days, or a watch named the row |
| `pending` | a bill or local law that amends this provision has advanced (passed a house, approved by the Council) |
| `imminent` | it was delivered to the Governor or Mayor, or signed with a later effective date |

The HTML-to-text step strips tags, drops `script` and `style`, and decodes entities. `normalize`
does not fold curly quotes or entities (`New York City’s` in the HPD row), so the extractor must
produce the same characters the original copy had. Unverified until the dry run.

## Job flow

`.github/workflows/law-check.yml`, in the style of `security.yml`:

```yaml
on:
  schedule:
    - cron: "17 6 * * *"   # daily 06:17 New York: bill and local-law watch
      timezone: "America/New_York"
    - cron: "17 7 * * 1"   # Monday 07:17 New York: text check after the Senate's Friday batch
      timezone: "America/New_York"
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
| Rate limit (`429`) | wait once, honouring `Retry-After` up to 60 s, then `unreachable`. The Senate API documents no limit; the load is one request per fetched row a week, a few dozen at most |
| `404` / `410` | `dead`, with the URL for a browser re-check |
| Redirect | `moved`, with the `Location`; followed on the same site only, then the passage check |
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
- **Cursor miss.** A planted change that the cursor path skips must be reported by the monthly
  full pass.
- **amlegal pieces.** A small fixture zip: the job reads its directory and one entry by byte
  range, finds a changed section `version`, and never reads the rest of the file.
- **New York dates.** A run at 01:30 UTC on a summer night is dated the previous day.
- **The key never leaks.** A request with key `SECRET-TEST` to an unroutable address. The printed
  error must not contain it.
- **Dry run.** `workflow_dispatch` with `dry_run: true` writes the body to the step summary and
  opens nothing.

## Rollout steps

1. ~~Aisling approves the revised doc.~~ Approved 2026-10-31.
2. Build `--law-check` and its tests. One PR, CI green, no workflow yet.
3. Aisling runs it locally once, free, with the key read the way she reads her other keys. On
   today's seed the expected result is no `changed` rows. Anything else is an extractor bug,
   fixed before step 5.
4. Check `robots.txt` on each fetched host. On the first keyed call, confirm `activeDate`
   matches the revision dates in the captures. (The bulk XML's 12 Admin Code sections were
   already compared with the captures on 2026-10-31: identical; answer 5 below.)
5. She adds `NYSENATE_API_KEY` (and the Legistar token, if she requests one). Merge the workflow
   with `workflow_dispatch` only, run it once with `dry_run: true`, and confirm the log shows ids
   and hosts only, the `timezone` key is accepted, and the measured traffic. Then turn on the
   schedules.
6. After four weekly runs, review false positives and adjust the extractor, not the comparison.

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

## Revised after review: the research (2026-10-31)

Three questions went back for research after her review. Sources are linked inline. Figures
marked "measured" were read from live responses on 2026-10-31; the rest come from the sources'
own docs.

### Cadence: watch bills daily, check text weekly

**The shortest gap between a law passing and taking effect is zero days, at both levels.** The
2019 Housing Stability and Tenant Protection Act and the 2024 Good Cause Eviction law took effect
on signing; so did NYC Local Laws 144/2025 and 23/2026. The official text lags behind the law:
nysenate.gov publishes statute text in a weekly batch, 6 to 14 days after each change took
effect, and on 2026-10-30 amlegal was about 6 days and 7 local laws behind the Council. So a check
of law text can never warn ahead of a change. **Warning has to come from watching bills.**

| Watch | Cadence | Signal |
|---|---|---|
| NY bills that amend a tracked section ([bills API](https://legislation.nysenate.gov/static/docs/html/bills.html), `bills/updates/{cursor}`) | daily | passed a house → `pending`; delivered to the Governor or signed with a later date → `imminent`; signed with immediate effect → "changed, verify" |
| NYC Council matters whose text cites Title 27 Chapter 2 or a tracked section ([Legistar Web API](https://council.nyc.gov/legislation/api), free token) | daily | approved → `pending`; enacted → `imminent` or "changed, verify" |
| NY statute text (Open Legislation `laws/updates`) | weekly, Monday | `activeDate` and hash |
| NYC Admin Code and Rules text (amlegal bulk XML), HPD rules feed, HCR, 311 and Open Data pages | weekly, Monday | see "Change-only checks" |
| nycourts.gov and nyc.gov (manual) | every 30 days, and whenever a watch names the row | a person opens the URL |
| Calendar | fixed dates | re-check the named rows before Oct 1 (heat season, rent-stabilized leases under the new Rent Guidelines Board order), before Aug 1 (the Good Cause rent figure) and before Jan 1 (lead-paint and window-guard notices) |

The 90-day manual interval becomes 30 days, plus an event trigger from the daily watches.

**Worst case for a tenant.** Her paperwork lag is about 10 calendar days (3 days to gather, 5
business days to process). Assuming she verifies a finding within 3 days:

| Kind of change | Warning before it takes effect |
|---|---|
| State law with a delayed date (shortest found: the 30th day after signing) | at least 26 days |
| City law with a delayed date (shortest found: 180 days) | months |
| Law effective on signing | none; flagged the next morning. That leaves 13 days on a 14-day clock (rent demand, marshal's notice) and 9 days on the 10-day clock to answer a petition, about one day short |
| The same, with a weekly signal instead of daily | 7 days left on a 14-day clock, which fails |

The tenant deadlines behind these numbers include HPD's correction periods (class A 90 days,
B 30, C 24 hours, but 21 days for lead paint, window guards, mold and pests; heat and hot water
at once) ([HPD](https://www.nyc.gov/site/hpd/services-and-information/clear-violations.page)),
the 14-day deposit statement (GOL § 7-108 (1-a)(e); § 7-107 (6) for rent-stabilized leases from
2025-11-15), the 14-day rent demand ([RPAPL § 711](https://www.nysenate.gov/legislation/laws/RPA/711)),
10 days to answer a petition ([RPAPL § 732](https://www.nysenate.gov/legislation/laws/RPA/732))
and the 14-day marshal's notice ([RPAPL § 749](https://www.nysenate.gov/legislation/laws/RPA/749)).

### Change-only checks: one cursor per source

Each source keeps a cursor: its own change feed where it has one, otherwise the HTTP validator
from the last answer. A quiet week costs one small request per source, not one page per row. Only
rows whose source reports a change are fetched and compared.

| Source | Cursor | Quiet week | On a change |
|---|---|---|---|
| NY statutes | `processedDateTime` from `GET /api/3/laws/{GOB,RPP}/updates/{cursor}?type=processed` | 2 small calls | fetch only the listed tracked sections; compare `activeDate`, then the hash |
| NY bills | last `bills/updates` time | 1 call a day | read the bill's law sections; set `pending` or `imminent` |
| NYC Admin Code | the Admin zip's `ETag` | 1 conditional `HEAD`; measured `304`, 0 bytes | one `Range` request for the zip's directory (about 130 KB) gives a CRC per entry. Read the 944-byte entry holding "Current through Local Law …". If the Housing Maintenance Code entry's CRC changed, range-fetch that one entry (178 KB) and compare each tracked section's `version` and text. Never the 65.5 MB file |
| NYC Council | last `MatterLastModifiedUtc` | 1 call a day | read the matter's text for Title 27 Chapter 2 and tracked sections |
| HPD rules | the feed's `ETag` | 1 conditional GET; `304` measured | keep items with `agency_name` HPD; codified text from the Rules zip, as above |
| hcr.ny.gov | `Last-Modified` | 1 conditional GET per page | compare passages (the date is a cache date, so a `200` alone proves nothing) |
| portal.311.nyc.gov | none (no validators) | 1 GET, about 195 KB | compare passages |
| data.cityofnewyork.us | `viewLastModified` from `/api/views/wvxf-dwi5.json` | 1 GET, about 85 KB | compare the description |

The amlegal entries embed a per-build token, so an entry's CRC may change on every rebuild with
no text change. The CRC therefore only says "look"; the decision is the section's `version` and
text. `version` is set per paragraph record, so a section carries several; whether a record's
`version` rises only when its text changes is confirmed once two builds can be compared.

**The safety net.** On the first Monday of each month the job compares every fetchable row,
whatever the cursors say. A test plants a change that the cursor path misses, and requires the
monthly pass to report it, the same way a broken cursor must fail its test in a delta sync.

**Where the cursors live.** In a small `law-check-state.json`, uploaded as a workflow artifact
each run and downloaded by the next with `gh run download` (artifacts are kept 90 days). The job
writes nothing to the repo, so it keeps `contents: read`. If the state is lost, the next run is a
full pass, the way a stale cursor gets everything again.

**Traffic rules.**

1. Ask before fetching: a cursor or validator first, a body only on change.
2. Byte ranges for big files.
3. One request a second per host; one retry; honour `Retry-After` up to 60 seconds.
4. Redirects: at most 5 hops, same site only.
5. The comparison uses `normalize` and `text_sha256`, as ingest does.
6. Expected load on a quiet week: under 0.5 MB, most of it the two pages that send no validators.
   This is an estimate until the dry run measures it.

### Dead or moved pages: find the replacement

The row keeps being served until Aisling changes it; the job attaches evidence.

1. Fetch with redirects off. Check for a bot challenge first: a challenge is `blocked`
   (unknown), never `dead`, and is not retried.
2. On a `3xx`, record the `Location` and whether the move is permanent. Follow up to 5 hops on
   the same site and run the passage check on the end page. A match is reported as "moved, same
   text"; a miss as "moved, text differs".
3. On a `404` or `410`, look for the replacement: the last good capture and any captured
   redirect in the Wayback Machine's
   [CDX API](https://github.com/internetarchive/wayback/tree/master/wayback-cdx-server), and the
   new entries in the host's `sitemap.xml` since last week where one exists (hcr.ny.gov,
   rules.cityofnewyork.us). Each candidate is scored with the same passage check.
4. When a fetchable row comes back `unchanged`, archive it with the Wayback Machine's
   Save Page Now (free without a key, a few requests a minute), so a dead page later has a dated
   copy. Protected hosts are left out: the archive stores their challenge page, not the text.

A person confirms the replacement (same publisher, same provision, the text in a browser) and
changes the URL by PR. Paid permanent-link services (Perma.cc) are not needed for this.

### New York time

- **Schedule.** GitHub Actions takes an IANA time zone next to `cron` since March 2026
  ([changelog](https://github.blog/changelog/2026-03-26-github-actions-late-march-2026-updates),
  [syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax)),
  as in "Job flow" above. Minute 17 avoids the top-of-hour queue delays GitHub warns about. The
  dry run confirms the key works before the schedule is turned on.
- **Dates the job writes** (the issue, the changelog, `verified_on`): `YYYY-MM-DD` in New York.
  In Rust, [`jiff`](https://docs.rs/jiff) 0.2 (MIT or Unlicense, current tz data, bundled on
  Windows, system data on the Linux runner), in `ingest` only. `chrono-tz` was rejected: no
  release since 2025-07, so its zone data is stale. The API's own dates (its UTC `today_iso()`)
  are a separate fix.

### Paid law services: not needed

Bill trackers (LegiScan, Plural, FiscalNote, Quorum) cover bills, which Open Legislation and
Legistar give free. Only Lexis, Westlaw and vLex sell codified New York text, all by sales
contact, with terms aimed at research rather than republishing, and none confirmed to carry the
NYC Admin Code. The free official sources above give a change signal for every row. Revisit only
for other states or case law.

### Compression: not needed for law text

The law corpus is about 56 KB (14 KB gzipped). Compression pays elsewhere: the citywide
branch already stores violation descriptions as zstd blocks with a trained dictionary (2.3 µs to
read one building's block), and the release download is 160.9 MB zstd'd against 479.1 MB.

### Licence: questions for a lawyer

Statute text is likely not copyrightable
([*Georgia v. Public.Resource.Org*](https://www.law.cornell.edu/supremecourt/text/18-1150)).
legislation.nysenate.gov's footer says CC BY-NC-ND 3.0; nysenate.gov's
[copyright policy](https://www.nysenate.gov/copyright-policy) adds "CC+" permissions that allow
derivatives and commercial use except political fundraising. HouseCheck is a nonprofit funded by
donations, and verbatim excerpts fit "NoDerivatives". Questions:

1. Does CC BY-NC-ND bind statute text fetched through Open Legislation, or only the Senate's own
   material and compilation?
2. Does the CC+ grant cover the API's data?
3. Apache-2.0 lets others use the project commercially. Does the law database need its own data
   licence notice?
4. amlegal's history notes are privately written. Can they be stored, or only the code text?

### Her answers to the research items (2026-10-31)

| # | Item | Answer |
|---|---|---|
| 1 | "May have changed" warning in answers | **Yes.** While a row is `pending` or `imminent`, the agent says "this may have changed on <date> (<bill>); confirm with Legal Aid" instead of stating the old rule flatly. It changes the prompt and `agent_version`, so its billed re-record is batched with Phase B's prompt changes |
| 2 | Legistar token | **Yes**, she requests it; the Council watch uses the official API |
| 3 | Her turnaround | **Within 3 days, the same day for a law effective on signing.** The issue lists those first, marked urgent |
| 4 | More tracked sections | **Add all five** to Phase B: RPAPL §§ 711, 732, 749 and RPL §§ 226-c, 231-c. All are state statutes, so the Senate API checks them |
| 5 | Which "current through" line | **Settled 2026-10-31.** The 12 captured sections were compared with the Housing Maintenance Code entry of the bulk XML (built 2026-10-30, "Current through Local Law 2026/160"), fetched by byte range: 443 KB of the 65.5 MB file. All 12 are identical once whitespace is normalized, a TAB read as a space, and amlegal's `[ALP S-nnn]` supplement markers (editorial, not shown on the reader page) dropped. The reader page's "2026/153" line was older than the bulk file; the job records the bulk file's line. `version` is per paragraph record, so a section has several (27-2045 has six); a section's signature is its set of record versions plus its text |
| 6 | Hot water hours | **Serve both, labelled.** § 27-2031's text is the rule (120°F "at all times between the hours of six a.m. and midnight"; 110°F with mixing valves). The 311 line ("365 days per year") is labelled city guidance. The agent never states round-the-clock hot water as law, and adds a question for Legal Aid about hours |
| — | `verified_by` for the captured texts | **"Aisling"**, set as she reads each one against its source |
| — | The design | **Approved** 2026-10-31. Building starts after Phase B lands |
