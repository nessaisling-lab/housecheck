# Design: serve the relevant passages of the law, keep the full text verified

**Dated:** 2026-10-31 (New York time). **Status:** reviewed by Aisling on 2026-10-31; her answers
are at the end. Nothing here is built.

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
5. Time-sensitive questions get an urgency warning, a dated question for the lawyer, and the full
   law ready to hand over (section 7), in a form an elderly, non-technical reader can use (section 8).

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

In the chat sheet: **Law detail: Brief | More | Full**, remembered in this browser (no accounts,
as decided). Sent with each chat request as `law_detail`.

- **Brief** (default): passages as above, up to 6,000 characters a lookup.
- **More**: the same ranking, up to 10,000 characters.
- **Full**: whole provisions, the complete text. When a lookup's full text is over 40,000
  characters, the first answer uses a 40,000-character preview (passages in rank order) and the
  app shows a warning with the size: the full text makes the answer slower. The reader has to
  confirm explicitly ("Use the full text") before the question is asked again with every word.
  The server enforces it: without the confirmation flag, it never sends more than 40,000.

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

### 7. Time-sensitive questions: urgency, a date first, and a bundle for the lawyer

Aisling's requirement (2026-10-31): when a question shows a deadline or a danger, the answer says
so plainly, tells the reader how much time is left, and hands them everything a lawyer needs,
with the full law, so nothing is missing that could work against them. Her answers to the
questionnaire are in the table at the end.

**Levels.** Each is a word, an icon and a colour together, never colour alone (section 8):

| Level | When |
|---|---|
| **Urgent** | 3 days or less; a marshal's notice, a lockout, a court date today or tomorrow; or danger |
| **Time-sensitive** | 4 to 14 days |
| **Coming up** | 15 to 30 days |

**Detection: rules first, the model may only raise it.**

1. The server reads the reader's messages for dates ("Nov 5", "11/5", "2026-11-05", "Monday",
   "tomorrow", "in 3 days") and counts the days left itself, in New York time. The model never
   does date arithmetic (the evals already forbid arithmetic by the model).
2. It reads them for deadline words (court date, hearing, petition, served, marshal, eviction
   notice, rent demand, locked out) and condition words (no heat, no hot water, no water, peeling
   paint with a child, gas smell, smoke, fire, carbon monoxide).
3. The result, `urgency: {level, reason, days_left, date}`, goes to the model with the building
   facts, and to the app with the answer.
4. The model may raise the level, with a reason, through a small `mark_urgent` tool. The server
   accepts a higher level only, never a lower one.

**Danger comes first.** Anything that sounds like immediate danger (gas, smoke, fire, carbon
monoxide) opens with "Call 911 now" before anything else, with no questions asked.

**A deadline with no date: ask first.** If the reader names a deadline but no date ("my court
date is soon"), the agent answers with one question: the exact date, and what paper they
received. It says why: a lawyer at a free service can act fastest on a complete, dated request,
and those services are stretched thin. Aisling's words: nothing should be vague here. Once the
date is given, the full answer follows.

**What the urgent answer says.** Legal information, never advice:

- At the top: the level, the deadline as the served law words it, and the days left
  ("Urgent: your court date is in 2 days").
- The four parts, with part 4's question carrying the date and the days left, so the lawyer
  sees the urgency in the first line.
- "Bring these": HouseCheck's checklist for the topic, with the record and the laws used.
- Call a named free legal-help organisation today, with its phone number and hours.
- Never which legal step to take, or how it will go.

**The bundle for the lawyer.** One plain-text file, offered first and prominently: "Download
everything for your lawyer". It holds:

1. The urgency summary (level, date, days left, counted on date X in New York time).
2. The question for the lawyer, labelled as drafted by HouseCheck's assistant.
3. The checklist of what to bring.
4. The building's record with sources.
5. The **full text** of every law used, each with its official link, verified date and SHA-256.

The answer itself still uses passages, so it comes back fast when time is short; the full text
travels in the file. A secondary option, offered after the first: a `.zip` of separate files
plus the signed building export (`/building/{bbl}/export`). Built later; the `.txt` comes first.

### 8. Accessibility: built for someone in a crisis, on a phone, maybe elderly

Aisling's requirement: a person who is not comfortable with technology, possibly elderly, with
low vision or other access needs, may use this in an emergency. So:

- **Never colour alone** (WCAG 2.2, 1.4.1): every level shows its word and an icon (Urgent: a
  warning triangle; Time-sensitive: a clock; Coming up: a calendar) beside the colour.
- **Contrast** of at least 4.5:1 for text (1.4.3), and 3:1 for the icon and the banner's edge
  (1.4.11). The "Coming up" yellow uses dark text on a pale background, never yellow text.
- **Announced, not only shown:** the Urgent banner is `role="alert"`, so a screen reader reads it
  first; the others are `role="status"`.
- **Big, plain targets:** the download and call buttons are at least 44 by 44 pixels (2.5.8 asks
  for 24; 44 is the platform guideline) and say exactly what they do ("Download everything for
  your lawyer", "Call Housing Court Answers"). Phone numbers are `tel:` links.
- **Plain language:** short sentences, no legal jargon in HouseCheck's own words, dates written
  out ("Tuesday, November 4"), days left as a number.
- **Text scales** to 200% without losing anything (1.4.4), and nothing depends on hover.
- **One clear next step** at the top of an urgent answer, not a menu.

These are checked in the build with an automated accessibility scan of the chat sheet and a
manual pass with a screen reader and 200% zoom.

## What to measure

Before (live run 6) and after, per conversation: tool-result characters (p50, p95), cost, time,
and how many answers carry all four parts. Target: Brief serves at most 6,000 characters a
lookup, and at least 19 of 21 live conversations pass.

## Not in this change

- Passages in the PDF and JSON exports (`/building/{bbl}/export`). Later, once the reader-facing
  shape has settled.
- Semantic (embedding) search. FTS5 is exact, explainable and already compiled in; revisit only if
  measured relevance is poor.

## Aisling's answers (2026-10-31)

| Question | Answer |
|---|---|
| Budget | Three levels: Brief 6,000 characters, More 10,000, Full the complete text |
| Default and place | Brief by default; the toggle in the chat sheet, remembered in this browser |
| Download format | Plain text with the verification header now, PDF later |
| Full-mode cap | 40,000 characters is a preview and a warning; the full text only after the reader explicitly confirms |
| Urgency detection | Rules first (dates and words, days counted by the server in New York time); the model may raise the level, never lower it |
| Levels | Three: Urgent (3 days or less), Time-sensitive (14), Coming up (30) |
| Full text when urgent | A bundle for the lawyer with the full text of every law used; the answer stays on passages |
| Emergencies | Safety emergencies count; anything that sounds like danger leads with 911 |
| Warning wording | The deadline as the law words it, the days left, what to bring, and to call free legal help today; never a legal step or an outcome |
| Bundle | One `.txt` first, offered prominently; a `.zip` with the signed export as a secondary option later |
| No date given | Ask for the date first, and explain why: a complete, dated request lets a stretched free service act fast. Nothing vague |
| Level names | Words with colours and icons together, accessible to an elderly, non-technical reader in a crisis (section 8) |
