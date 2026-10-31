# Design: serve the relevant passages of the law, keep the full text verified

**Dated:** 2026-10-31 (New York time). **Status:** proposal for Aisling's review. Nothing here is
built.

**The question:** Phase B made the law table complete enough to be slow. `lookup_law` hands the
model every current provision on a topic, whole. How should HouseCheck give the model, and the
reader, only what bears on the question, without ever weakening the guarantee that every word
is the official text?

**Answer:** keep the full official text as the single source of truth, cut it into passages at
its own subdivisions when the database is built, serve the passages that match the question, and
give the reader the full verified text on demand: to read, download or attach.

---

## Why now (measured on `claude/gallant-cohen-fb2f37`, 32 provisions)

What one `lookup_law` call sends today, by topic:

| Topic | Provisions | Characters |
|---|---|---|
| `eviction` | 9 | 49,999 |
| `repairs` | 9 | 29,659 |
| `security_deposit` | 3 | 21,667 |
| `lead_paint` | 2 | 11,227 |
| `mold` | 3 | 7,502 |
| `heat`, `hot_water` | 4 each | about 5,800 |

Live run 6 (2026-10-31, $0.2279) is the first live run of the four-part answer, and 11 of its 13
failures are answers that skipped parts 3 and 4. The model wrote its own headings after thousands
of words of statute. RPL § 231-c alone is 18,674 characters, most of it a notice form, and any
eviction question carries it. A smaller, sharper context is the most likely fix, and it is also
cheaper and faster.

## Aisling's requirements (2026-10-31)

1. Serve only the relevant passages.
2. The reader can get the full text: read it, download it, or attach it to an email or a text.
3. A verbosity toggle the reader controls.
4. "No illusions": the full text is always verified against the source, in testing and as the
   product runs. A passage is never a paraphrase and never drifts from the full text.

## Design

### 1. Passages are cut from the full text, never written by hand

`data/law.json` keeps exactly what it has now: each provision's full verbatim text and its
SHA-256. Ingest splits the text at the provision's own subdivision markers, a line that opens with
`a.`, `a-1.`, `1.`, `(1)`, `(a)` or `* 2.`, and stores each piece with its position:

- table `law_passage (provision_id, seq, label, text)`, `label` being the marker (`a.`, `(3)`).
- **Lossless by construction.** The pieces joined in order must equal the full text, byte for
  byte, or ingest refuses the provision. Measured on the 32 current provisions: lossless for all
  32, 212 passages, median 393 characters, 90% under 1,125, largest 5,972 (inside § 231-c's
  notice form).
- Every fact's quote must fall inside one passage, so a served fact always comes with the words
  it was taken from. Measured: all 15 do.
- When the passage before a served one ends with a colon (a lead-in such as "…shall constitute a
  hazardous violation if:"), the lead-in is served with it, so `(a)` never arrives without the
  sentence it completes.

Nobody edits passages. Change the full text (by the Phase C process) and the passages follow.

### 2. Which passages are served

`lookup_law` keeps its topic and gains one optional argument, `about`: a few words for what the
reader asked. Within the topic's provisions:

1. Rank passages with SQLite's FTS5 full-text index (compiled into the bundled SQLite:
   `libsqlite3-sys` 0.30.1 builds with `SQLITE_ENABLE_FTS5`). The query is the reader's last
   message plus `about`, with the Porter stemmer so "increase" matches "increases".
2. Always include each passage that holds a fact's quote for the topic.
3. Add passages by rank until a budget of **6,000 characters**, never splitting a passage.
4. Each provision appears at most once, with its heading, citation, official link, verified date,
   and its passages in their original order, labelled ("§ 27-2017.3 a.4").
5. With no match, serve each provision's opening passage, which states what the section covers.

For eviction this goes from 49,999 characters to at most 6,000, an 88% cut, before any tuning.

### 3. The full text, for the reader

- **`GET /law/{id}`**: the provision as JSON: citation, title, full text, official link, retrieved
  and verified dates, who verified it, SHA-256.
- **`GET /law/{id}/text`**: the same as a plain-text download (`HouseCheck-NY-RPL-231-C.txt`), with
  a header: citation, official link, verified on, verified by, SHA-256, and "Verified copy of the
  official text. Compare it with the official link before relying on it."
- The chat API returns the provisions it served (id, citation, official link) beside `citations`.
  **The app draws the links, not the model:** under an answer, "Laws used", each with "Full text",
  "Download", and "Share" (the phone's own share sheet through the browser's Web Share API, which
  can attach the file to an email or a text; plain download where it can't). The model never has
  to write a link, so it can't write a wrong one.

### 4. The verbosity toggle

In the chat sheet: **Law detail: Brief | Full**, remembered in this browser (no accounts, as
decided). Sent with each chat request as `law_detail`.

- **Brief** (default): passages as above.
- **Full**: whole provisions, as today, up to 40,000 characters; anything over that is served as
  passages with a note pointing at the full text link.

### 5. Verification, at every step

| Where | Check |
|---|---|
| Ingest | full text hashed; passages rejoin to the full text exactly; each fact quote inside one passage |
| CI | the above over the real `law.json`; every served passage is a substring of its provision's full text; `/law/{id}/text` returns the full text whose SHA-256 matches the stored one |
| Evals | the existing checks (numbers, temperatures, sections, links) run against what was actually served, so a claim from a passage that wasn't served fails |
| Weekly (Phase C) | the full text, not the passages, is compared with the official source |

### 6. Bundled with this change (from live run 6)

- **The answer format in the tool result.** Every tool result that serves law carries the four
  headings, so they sit next to the law instead of only in the system prompt.
- **Number words.** The number check also reads numbers written as words in what was served
  ("twenty-one days", "nineteen hundred seventy-eight"), so a faithful "21 days" passes. A
  planted "22 days" must still fail.

`about` changes the tool schema and the format note changes what the model reads, so
`agent_version` changes. One billed re-record covers it all.

## What to measure

Before (live run 6) and after, per conversation: tool-result characters (p50, p95), cost, time,
and how many answers carry all four parts. Target: Brief serves at most 6,000 characters a
lookup, and at least 19 of 21 live conversations pass.

## Not in this change

- Passages in the PDF and JSON exports (`/building/{bbl}/export`). Later, once the reader-facing
  shape has settled.
- Semantic (embedding) search. FTS5 is exact, explainable and already compiled in; revisit only if
  measured relevance is poor.

## Questions for Aisling

1. **Budget:** 6,000 characters a lookup in Brief (about 1,000 words)?
2. **Default:** Brief, with Full one tap away?
3. **Download format:** plain text with the verification header now, PDF later?
4. **Toggle place:** in the chat sheet, remembered in this browser?
5. **Full-mode cap:** 40,000 characters, falling back to passages above that?
