# HouseCheck — Backlog

**The live to-do list.** `ROADMAP.md` and `TASKS.md` are historical capstone artifacts and are
not maintained; this is what is actually open.

**Rule for this file:** every item says *why*, and anything derived rather than measured says so.
An item with no reason is a wish, not a task.

**Last updated:** 2026-08-12.

---

## Live defects — measured against production on 2026-08-11

Found by measuring the deployed product rather than reading the repo. **The repo is not the demo.**

- [x] **The Fly backend is one commit behind `main`, and it breaks two of the three export
      destinations.** *Fixed by deploy, 2026-08-11.* Re-measured after: `?format=text` returns
      `text/plain; charset=utf-8`, 6,905 bytes, the real transcript. Copy and Print work again.
      Measured before: `GET /building/3016440063/export?format=text` returned
      `content-type: application/json`. The frontend asks for `format=text` for both Copy and
      Print (`frontend/src/lib/api.ts:241`, `pages/HealthCard.tsx:203`), so **Copy puts ~13 KB of
      raw JSON on the clipboard and Print renders that JSON at 11px monospace.** Download is
      unaffected — JSON is what it is meant to produce. The missing commit is `6617151`. Fixed by
      a `flyctl deploy` from the owning account, not by code. *This also invalidates the premise
      of `design/pdf-export.md` §1, which describes the browser print path as working.*
- [x] **Production exports are unsigned.** *Resolved 2026-08-11 — key set, and a second hole
      found and closed in the same pass.* Live exports now carry `signature` and `public_key`,
      and `POST /verify` returns `signed_and_intact` for a genuine document and
      `tampered {row: 0, what: "row content"}` for one altered character.
      **But signing alone was not enough.** Verified from outside with an independent Python
      implementation: a forger who rewrites a row, **recomputes the whole chain**, and signs it
      with their own keypair produces a document that verifies as `SIGNED AND INTACT` — it is
      internally consistent, so every check inside it passes. A row rewritten to *"NO VIOLATIONS
      OF ANY KIND AT THIS ADDRESS"* passed cleanly. The only defence is comparing the embedded
      key against one published independently, and **nothing published it** — although
      `ExportDocument::public_key`'s doc comment had said "the published one" since it was
      written. Closed in `d24a028`: `/meta` now serves `export_public_key`, and the transcript
      instructs the comparison. Re-verified after deploy: the same forgery is now
      `REJECTED: signed by an unpublished key`, and the genuine document reads
      `SIGNED AND INTACT, key matches /meta`.
      *Original measurement:* `signature: null`, `public_key: null`,
      chain intact. The fail-closed path is behaving correctly — an absent
      `HOUSECHECK_EXPORT_SIGNING_KEY` produces an unsigned-but-chained document rather than a
      signature-shaped lie. But the live product currently proves *non-alteration* and not
      *authorship*, so "signed, verifiable record" overstates what a visitor can actually get.
      Whether the cause is an unset secret or the stale binary is **not** established; nobody has
      run `flyctl secrets list -a housecheck-nessa`. **Update, 2026-08-11:** now established.
      The backend was redeployed at `main`, the signing code is present, and the export is
      still unsigned — so the cause is an **unset secret**, not a stale binary. Setting it is a
      key-handling step for the repo owner alone.
- [x] **Ambiguous addresses resolve to an arbitrary borough, presented as fact.** *Fixed in
      `8aacc48`.* GeoSearch is now asked for five candidates; every one carrying a BBL is
      returned with its borough in plain words, covered buildings first via a stable sort.
      A feature with no BBL is skipped rather than fatal. Because the curated path short-circuits
      before the geocoder — and must, since it is **2.7 ms** against the geocoder's **5-7 s** —
      the second half needed `?scope=city`, surfaced as *"Not this one? Search all five
      boroughs"*. Verified in a browser against a local backend: the Manhattan building comes
      back in 4.0 s, flagged outside the pilot. Original measurement below.
      `search_handler`
      asks GeoSearch for `size=1` (`crates/api/src/main.rs:887`). Measured against GeoSearch with
      `size=5`, the correct borough is the **second** result in all three cases tried, at
      *identical* confidence 0.8:

      | typed | result 1 (what we show) | result 2 (also 0.8) |
      |---|---|---|
      | `350 5 Avenue` | 350 5 AVENUE, **Brooklyn** | 350 5 AVENUE, **Manhattan** |
      | `869 Park Avenue` | 869 PARK AVENUE, **Brooklyn** | 869 PARK AVENUE, **Manhattan** |
      | `1 Court Square` | 1 COURT SQUARE, **Brooklyn** | 1 COURT SQUARE, **Queens** |

      We are not being misled by the geocoder. **We ask for one answer to a question that has
      several equally-ranked ones, then print the arbitrary pick with no borough label.**
      `869 Park Avenue` additionally returns `in_curated_set: true` in 147 ms — one tap from a
      full Health Card for a building in the wrong borough. Fix is `size=5`, show the candidates,
      let the person choose. No schema change, no re-ingest.
- [x] **A failed lookup left the previous address's results on screen.** *Fixed in `33f3686`.*
      **Observed on production, and the prior guess about it was wrong.** `Home.tsx` awaited
      `searchAddress` with no catch, so a 404 threw past `setResults`. This was assumed to hang
      the spinner; it does not. The spinner clears and the dropdown silently keeps the *last*
      query's buildings. Measured live: input reading `Joe's Pizza`, list still offering five
      `869 PARK AVENUE` results, one tap from a Health Card for a building nobody searched for —
      the same confident-wrong-answer failure as the borough bug, reached another way.
      Now: every exit clears prior results; a 404 and a transport failure say different things
      (verified by taking the backend down and typing `464 Madison Street`, a **real covered
      address**, which reports *"we couldn't reach the address service"* and not *"not found"*);
      and a monotonic request epoch drops a slow older response, because debouncing narrows that
      window without closing it — the geocoder path takes seconds and the local path
      milliseconds. `More.tsx` had the same unguarded promise, rendering `Showing 0 of 250` over
      nothing, which reads as a fact about the pilot rather than a failure to load it.
- [x] **On failure the agent answered a different question.** *Fixed in `0a54446`.* The catch
      pushed `offlineAnswer(t)`, which for a typed question falls through to
      `answerChip(CHIPS[0], …)` — the canned score explanation — whatever was asked. Measured
      while the upstream was slow: **two of three runs** of "there is no heat in my apartment,
      what should I do" returned 502, so a score paragraph tagged *"offline answer"* was the
      common reply to a question about heat. The failure is now stated first and the canned
      material offered second. Verified on a production build with only `/agent/chat` forced to
      fail, so the branch under test was the only thing exercised.
- [x] **`/summary` fails every time, so the agent panel opens on an error message.** *Fixed in
      `8b55a72`* — the hardcoded `20` became `LLM_CALL_TIMEOUT_SECS` (30), so one attempt plus
      its retry is 61 s, the same arithmetic already const-asserted to fit the client's 70 s.
      **Still needs the model diagnosed**: `OPENROUTER_MODEL` *is* set as a Fly secret, so the
      `claude-haiku-4.5` default is being overridden, and a `:free` slug would explain a
      >20 s generation exactly as `main.rs:79-84` warns. Read it from the startup log after the
      next deploy (`flyctl logs -a housecheck-nessa`, look for `LLM: enabled`), because Fly
      secrets are write-only and cannot be read back.
      *Original measurement:* measured on
      production 2026-08-11: **HTTP 502 on 2 of 2 runs, both at 40.9 s**. That decomposes as
      `20 s + 0.7 s pause + 20 s` against the per-call timeout at `crates/api/src/main.rs:2520`,
      so both attempts timed out. The panel then renders *"The agent couldn't summarize this
      building — the raw data on the card is still your best source."* — **the first thing every
      visitor sees when they open the agent.** Confirmed in a browser on production.
      This is deterministic, not intermittent, which makes it worse than the chat 502s and puts
      it ahead of them. Note the per-call timeouts are inconsistent across call sites: **20 s**
      for summary (`:2520`), **25 s** for the law lookup (`:1559`), **30 s** in the agent loop
      (`:2337`) — the tightest budget is on the call that runs first and unprompted.
      **Diagnose before changing code.** The model is not in `fly.toml`, so it is a Fly secret or
      the `anthropic/claude-haiku-4.5` default. `main.rs:79-84` already warns that a `:free` slug
      drops long generations at ~22 s, which matches this signature exactly. Check with
      `flyctl secrets list -a housecheck-nessa` (names and digests only, never values).
- [x] **RESOLVED, and my diagnosis was wrong twice.** *2026-08-11.* `OPENROUTER_MODEL` was
      `nvidia/nemotron-3-ultra-550b-a55b:free`. Changing it to `anthropic/claude-haiku-4.5`
      fixed everything below at once:

      | | on `nemotron:free` | on `claude-haiku-4.5` |
      |---|---|---|
      | `/summary` | **502 × 2** at 40.9 s | **200 × 3**, 2.3-4.3 s |
      | "what is the condition score" | 4.4-6.8 s | **2.3 s** |
      | "what does NYC law say about heat season" | 18.5-49.2 s | **6.6 s** |
      | "there is no heat in my apartment…" | **502 on 2 of 3**, 25-60 s | **200 × 3**, 7.3-7.9 s |

      **Correction 1:** I wrote "the cause is not a slow model — it is the sequential tool loop."
      Wrong. The loop is real, but its per-round cost fell from 12-15 s to about 2 s, so the
      same two-round question now finishes in under 8 seconds.
      **Correction 2:** I read the 40.9 s failures as two attempts timing out against a 20 s
      budget, and "fixed" the timeout. The timeout was a symptom. The `:free` endpoint was
      returning `ResourceExhausted: Worker local total request limit reached (32/32)` and
      dropping bodies; `main.rs:79-84` had warned about exactly this since it was written.
      **Streaming is no longer justified.** It was the "final item" for four rounds on the
      argument that a 25-60 s wait needs progressive rendering and that >30 s generations
      hard-fail. At a 7.9 s worst case, neither holds. Re-open only if measurements move.
      **Bonus:** the model is now paid, so OpenRouter no longer logs prompts — the privacy
      caveat in `main.rs:79-84` is closed too.
- [x] **The 30 s per-call timeout is now the binding constraint, and the upstream exceeds it.**
      *Resolved 2026-08-15 — deployed, then measured on production.*
      Measured 2026-08-11 after the budget fix: the same question returned **200 at 27.4 s**,
      then **502 at 59.2 s** and **502 at 60.5 s** — which decomposes exactly as
      `30 s + 0.7 s pause + 30 s`, i.e. one generation exceeding the per-attempt timeout twice.
      **The old code called `openrouter_post(.., 30)` for every round too, so these runs would
      have failed identically before any of the deadline work** — this is upstream slowness, not
      a regression. It is also the strongest argument for streaming: a response that emits its
      first token in ~3 s never trips a no-response timeout, so streaming converts this entire
      hard-failure class into a slow-but-successful answer.
      **Deployed and re-measured 2026-08-15.** Streaming shipped in `f13845c` + `9f09757`.
      Across all five questions in the table below, run against production: **no 502s, no
      timeouts, worst total 7.0 s.** The `30 s + 0.7 s + 30 s` double-timeout signature is
      gone, exactly as predicted — a stream that emits its first token under a second never
      trips a no-response timeout, so the hard-failure class no longer has a way to occur.
- [x] **The agent takes 25-67 seconds on the questions people actually ask, and shows nothing
      while it works.** *Resolved 2026-08-15.* Measured on production 2026-08-11. The cause is not a slow model — it is
      the sequential tool loop, and the citation count is a clean proxy for how many rounds ran
      (`citations_for` seeds 4; each tool that runs adds one):

      | question | citations | rounds | latency |
      |---|---|---|---|
      | "what is the condition score" | 4 | 0 | **2.8 s** |
      | "how many open violations" | 4 | 0 | **5.8 s** |
      | "what does NYC law say about heat season" | 5 | 1 | **18.5 s** |
      | "is this building safe" | 6 | 2 | **25.8 s** |
      | "there is no heat in my apartment, what should I do" | 6 | 2 | **34.8 s** |

      One round trip is 3-6 s, so the model is fine. Each extra tool round costs ~12-15 s, and
      **the questions a tenant actually asks are the ones that trigger rounds** — the fast ones
      are the ones nobody needs an agent for. Worst observed: **66.7 s**, which is 3.3 s from
      the client's `LLM_TIMEOUT_MS = 70000` abort.
      Two fixes, in order. **Stream the answer** — `main.rs:116` records the deliberate choice
      not to (`"we do not stream"`), which was right when the loop was one call and is now the
      difference between 3 s to first token and 35 s of blank. **And align the deadlines:**
      `MAX_TOOL_ITERATIONS = 5` at a 30 s per-call timeout is a **150 s** server ceiling against
      a 70 s client abort, so the server can keep working — and billing — for 80 s after the
      reader has gone.
      **Both halves shipped, then re-measured on production 2026-08-15** — the same five
      questions, same building (`3016440063`), against the deployed stream:

      | question | was | first token | total |
      |---|---|---|---|
      | "what is the condition score" | 2.8 s | **1.0 s** | 2.3 s |
      | "how many open violations" | 5.8 s | **0.6 s** | 2.0 s |
      | "what does NYC law say about heat season" | 18.5 s | **0.8 s** | 5.1 s |
      | "is this building safe" | 25.8 s | **0.8 s** | 4.5 s |
      | "there is no heat in my apartment, what should I do" | 34.8 s | **0.8 s** | 7.0 s |

      The defect in the title was *blank screen*, and that is the number that closed:
      **34.8 s of nothing became 0.8 s.** First token never exceeded 1.0 s on any question,
      and the status line arrives at 0.0-0.1 s before that. Worst observed total is 7.0 s
      against a previous worst of 66.7 s, so the 70 s client abort is no longer anywhere near
      binding.

      **Do not attribute the total-time drop to streaming.** Streaming shows generation
      sooner; it does not make generation faster. Totals fell 34.8 s → 7.0 s, which is more
      than streaming can explain, so some of it is the deadline work and some is upstream
      conditions differing from 2026-08-11. **First-token latency is the structural win and
      is the only figure here that is safe to claim as ours.** Tool rounds also came in lower
      than the 2026-08-11 run ("is this building safe" showed one status line, not two), which
      is model behaviour and may not reproduce.

      Deadlines aligned in the same work: `AGENT_TOTAL_BUDGET_SECS` is the binding cap with
      `MAX_TOOL_ITERATIONS` demoted to a backstop, and a test parses `LLM_TIMEOUT_MS` out of
      `frontend/src/lib/api.ts` so the coupling cannot drift silently.
- [x] **Five of 250 buildings have an address with no house number, and one is the empty string.**
      *Fixed `7d0e414` (Rust) and `ff153d6` (frontend).*
      Measured on live `/buildings`: `3015097501` (`""`), `3016840001` and `3017030009` (both
      `FULTON STREET`), `3017790022` (`DEKALB AVE`), `3018110070` (`GATES AVENUE`). The empty one
      can never be reached by search — an empty haystack never contains a non-empty needle — and
      renders an empty heading.

      **What fixing it found.** `3015097501` — the unreachable one — has **96 open violations,
      12 of them Class C (immediately hazardous)**. The worst-documented building in the pilot
      set was the one nobody could look up. That reframes this from a cosmetic defect to a
      coverage failure: the search silently excluded the building that most needed finding.

      `model::export::display_address` states the gap rather than rendering blank
      ("Address not recorded" / "FULTON STREET (no house number on record)"), and the MCP
      search now also matches on BBL, since the identifier always exists even when the
      address does not. Three tests. Frontend fixed in `ff153d6`: verified in a browser that the heading now reads
      "Address not recorded" instead of nothing.
- [x] **HPD ships `0x1A` inside violation text, and it renders as nothing.** *Fixed `7d0e414`
      (Rust) and `ff153d6` (frontend).* Verified in a browser on BBL 3019380001: the card
      renders `DESCRIBED ON HPD'S WEBSITE` with zero substitute characters on the page.
      **Re-measured across the whole artifact 2026-08-12 and it is far wider than first
      recorded: 890 occurrences in 169 of 202 description blocks — 84% of covered buildings,
      not one.** Every instance is a possessive: `HPD'S` (640), `AGENCY'S` (158), `TENANTS'`
      (72), `BUILDING'S` (19). Confirmed identical in HPD's own `wvxf-dwi5`, so the ingest is
      faithful and this is the city's data.

      `model::export::for_display` substitutes the apostrophe and drops other C0 controls
      (a stray control byte is an invisible no-op in HTML and is not one in a PDF text
      stream). Applied at the render boundary only — the chain still hashes HPD's bytes
      exactly as retrieved, because normalising at ingest would convert a faithful record
      into a tidied one. A test asserts precisely that: the transcript reads `HPD'S` while
      the stored description keeps `U+001A`. The transcript now says the substitution
      happened, so a reader comparing it against the JSON is told why one byte per
      apostrophe differs rather than discovering it and wondering what else was adjusted.
      **This unblocks the PDF work.**

---

## Committed — the MVP

From `classwork/solution-design-sprint.md`. The single core feature:

> A tenant lawyer opens a building, sees every open violation in the notice's own words with how
> long each has been open, and exports it as a file a stranger can independently verify was not
> altered after retrieval.

- [ ] **Call a Legal Aid housing attorney or paralegal — before any more export work.**
      Cheapest possible way to be wrong. Also closes open question 1 in
      `classwork/problem-definition-notes.md`.

      **Prepped and ready to make: `docs/legal-aid-call.md`** — script, capture sheet, and
      the kill conditions below restated so they are decided before the call rather than
      after. Two things that came out of preparing it:
      *(a)* **do not use the intake line.** 212-577-3300 and Met Council's hotline are for
      tenants in crisis; a product-research call takes a slot from someone with no heat.
      Better routes, ranked, are in that doc — JustFix by email is first and warmest.
      *(b)* **prior art exists for the deferred owner-linkage item.** JustFix's
      `who-owns-what` already links NYC buildings to a common owner. It is GPL-3.0, so its
      code cannot be borrowed here, but building that feature without reading it first
      would be redoing solved work badly.

      **The assumption under test:** that the expensive part of a tenant lawyer's job here is
      producing a *trustworthy* record of what HPD says, and that a portable independently
      checkable file is a form they can use. Two separate claims, and either can fail alone —
      the manual pass may not cost enough time to matter, and the provenance of an HPD printout
      may never be challenged in the first place.

      **Ask about the workflow, never about the product.** "Would you use this" gets a polite
      yes and teaches nothing. Ask instead: walk me through the last time you needed a
      building's violation history — what did you do, how long did it take, what did you do
      with the output, did anyone ever question where it came from? And one artifact question:
      would you rather have a PDF, or a document you can edit into a filing?

      **Kill conditions, pre-registered so they cannot be argued away afterwards:**
      - *"Nobody has ever challenged where an HPD printout came from."* → the hash chain
        solves a problem that does not exist. It stays (it is built and costs nothing to keep)
        but stops being the headline; the product's value becomes speed and legibility, and
        `design/pdf-export.md` becomes **more** important rather than less.
      - *"We would need a certified copy from HPD / a sworn declaration."* → the mechanism is
        right and the packaging is wrong. Add a declaration page and find out what the accepted
        authentication route actually is. Do not defend the chain.
      - *"The lookup takes two minutes and we already have a way."* → the MVP is aimed at
        nothing, and the honest response is to change the primary user rather than the feature.
        That pivot is not free: the renter-at-the-moment-of-decision is the other candidate and
        is itself an open question below.

      **What a bad call does not invalidate:** ingest, scoring, the card, the agent, address
      resolution and the provenance stamp are all user-agnostic. The export is one route plus
      one module. That is the whole reason this call is cheap.
- [x] **Ingest: fetch `novdescription`.**
      Measured 100% populated across 800 sampled rows, mean 120 chars. One column on the SoQL
      select in `crates/ingest/src/run.rs`.
- [x] **Ingest: fetch violation open and close dates.**
      Required for days-open and time-to-close. Free — same rows.
- [x] **Ingest: stamp dataset version and retrieval timestamp per row.**
      Not bookkeeping. Without it the export's signature attests to a file rather than to a fact,
      which is security theatre. This is what makes the export honest.
- [x] **Model: extend `Violation`.**
      Currently `{ class, open, year }` in `crates/model/src/lib.rs` — there is nowhere for a
      description to go, so this is a schema change, not just a fetch.
- [x] **Run one real ingest on the 250 and read the actual artifact size.**
      Arithmetic says ~3.2 MB of text against a 1.3 MB artifact — roughly 3.4×, moving the 256 MB
      ceiling from ~40,000 buildings to ~14,500. That is *derived*. Confirm before it drives a
      decision.
- [x] **Card: render open violations** — class, raw notice text, days open.
- [x] **Derived: median days-to-close — per *building*, not per landlord.** *Shipped `76b2992`.*
      **The sprint's framing was not computable.** There is no owner column in the artifact and
      owner linkage lives in an HPD registration dataset that has never been ingested, so the
      feature is named for what it measures. One landlord's twelve buildings stay twelve
      unrelated records until that dataset lands — see the deferred owner/portfolio item.
      Three states, because two were wrong: a median, `nothing_closed`, or absent. The middle
      one exists because 603 Putnam (33 open, one closure ever, dated **2017-10-18**) rendered
      blank under two states — the building that fixes nothing looked emptier than one that
      fixes things slowly. **26 of 250 pilot buildings** have ≥5 open and zero closures in the
      window. Window and floor are judgment calls with the measurements recorded beside them:
      median-of-medians barely moves (121 d all-time / 118 since 2023) while the range collapses
      from **0–4,951 d** to **25–1,676**.
- [ ] **Median days-to-close per *landlord*** — needs the HPD registration dataset for owner
      linkage. The per-building version above is the same arithmetic; only the grouping key is
      missing. Blocked on the owner/portfolio item below, not on effort.
      **Update 2026-08-12: the grouping key is reachable.** `docs/owner-linkage.md` establishes
      the two Socrata datasets and the matching rules. The honest name for the result is
      *median days-to-close per registered contact*, not per landlord — the data cannot
      support the stronger word.
      The single best value-per-effort item found in the sprint: it gives an operator-level
      credibility signal with no landlord participation, no identity verification and no legal
      exposure.
- [x] **Export: document plus hash chain, signed Ed25519.**
      Reuses SiteAssure's `entry_hash = sha256(prev_hash + payload_hash)` and Resona's
      `verify_license_with` signing path. Both already exist and both were worked on this cycle.
- [x] **Public verifier** that takes an exported file and answers valid / tampered.
      The demo moment: hand someone the file, they change one character, verification fails.

---

## Toward a real product, not a demo

Raised 10 August 2026. These are the gaps between "the capstone works" and "a stranger can
rely on it", and several are accessibility issues rather than features.

- [x] **Remove the demo-data fallback from the shipping build.** *Shipped `ee3802d`.* Gated behind
      `DEMO_DATA_ALLOWED` (`import.meta.env.DEV || VITE_ALLOW_DEMO_DATA`). Verified with a
      production build against an unreachable API: the card now says "We couldn't reach the data
      service" instead of rendering a fabricated building. `getBuilding` also stopped reporting a
      backend outage as a 404 — an outage is a 503 and says so.
      *Original reasoning, kept because it is the argument:* a person checking a real address
      could be shown a **fabricated building** and the only signal was two small words.
- [ ] **Real PDF generation, server-side.** **Scoped 2026-08-11 — see `design/pdf-export.md`.**
      Printing currently hands off to the browser dialog, which works but puts the output under
      the reader's control and produces nothing an agent can attach. Decision: `printpdf` 0.12.5
      (**MIT**, released 29 July 2026, ~995k recent downloads) in a new `crates/render`, with
      `default-features = false`. Rejected `pdf-writer` (MIT/Apache-2.0, but every glyph width and
      line break would be hand-written) and `genpdf` (unmaintained). **Three gates before it
      ships:** the stripped feature set must build clean and not blow up the artifact; no C
      toolchain may enter the graph; and every link pattern must be fetched and return 200 before
      it is written into a document. The design rule is that the PDF is a *rendering* — it carries
      the record hash and says the JSON is the checkable artifact, because a typeset document that
      implies verification it cannot carry is the exact failure the export exists to prevent.
- [ ] **Let the agent hand over documents, not just prose.** When the agent cites a statute
      or a dataset it should be able to offer the source itself — a link the user can open, or
      a PDF of the relevant page — so someone who wants to read the law can, without knowing
      how to find it. Pairs with the PDF work above: the same links belong in the exported
      document.
- [x] **Save a conversation.** *Shipped `d69b701`.* Downloads a plain-text transcript; the
      product has no accounts by design, so the reader's own machine is the only honest place
      to keep one. The header carries the building, the BBL, the date, and a line stating it is
      an assistant transcript and not legal advice. Verified live: a 996-byte file headed
      `Building: 603 PUTNAM AVENUE (BBL 3016440063)`.
- [x] **Copy from the agent.** *Shipped `d69b701`.* A copy button per answer, treated as an
      accessibility defect rather than a convenience: the OS can already select text, but a
      selection handle on a phone is exactly the interaction an elderly tenant or a person with
      low vision cannot reliably perform. **Falls back to a hidden textarea + `execCommand`**
      where `navigator.clipboard` is unavailable (older Safari, any non-HTTPS origin) — failing
      silently there would break the fix precisely on the browsers least-served users have.
      **Zero new dependencies:** `navigator.clipboard`, `Blob` and `localStorage` are platform
      APIs, so neither of these two items carries a licence.

---

## Known defects and stated limitations

- [x] **A count without its meaning can be read backwards.** *Confirmed closed 2026-08-15 —
      this box was stale, its own text already said so.*
      `603 Putnam Avenue` scores `condition 0` with 11 Class A, 22 Class B and **zero** Class C,
      so the card reads "no hazardous violations" next to a floor-level score. Both numbers are
      correct; the sentence reconciling them — *33 open violations, none of them Class C* — cannot
      be rendered because descriptions are not ingested. **Closed by the MVP above.** The deck now
      explains it in the interim.

      Verified on production `GET /building/3016440063`: the card carries
      `open_violation_total: 33` next to `open_violations {a: 11, b: 22, c: 0}`, and
      `open_violation_details` renders each notice in HPD's own words. The reconciling number
      is on the card, so the count can no longer be read alone.
- [ ] **Class I violations excluded.** 753 records skipped on the curated set. Stated on the card
      and in `/meta`, but not scored.
- [ ] **Multi-source ingest and weekly self-refresh.** See `design/database-layer.md` addendum 2.
      One module per source with its own incremental key, cadence and schema contract; stage to
      Parquet; compose with DuckDB; build then verify then deploy, so a bad ingest never
      publishes. Measured: 35.7M source rows collapse to ~7.0M stored rows, ~303 MB for the whole
      city including 311, litigation, evictions and owner linkage.
- [ ] **No scheduled refresh.** The artifact is a point-in-time snapshot; nothing re-ingests. The
      card states its build date rather than hiding it, but it is still a limitation.
- [ ] **Coverage is 250 buildings, one community district (CD 303).**
      Disqualifying for a daily professional user — a tool covering 0.1% of a caseload does not
      get adopted.

---

## Address resolution — from `design/address-to-bbl.md`

- [x] **Fix `normalize_address` to expand by position, not presence.** *Shipped `c18a1dc`* —
      this box was stale. Verified in `crates/api/src/main.rs:751-808`: `ST`/`DR` expand only in
      final position and never first, so `ST NICHOLAS AVENUE` (**167** lots) and `100 ST JOHNS PL`
      both survive; directionals expand only when *not* last, so `AVENUE W` (**403**) and
      `AVENUE N` (**744**) survive.
      **Correction to `design/address-to-bbl.md:44-49`:** that doc prescribes "directionals only
      in leading position." The rule that actually landed is the opposite and is the correct one —
      it expands a directional *unless* it is last, so `AVE W` and `AVENUE W` reduce to the same
      string. **Following the doc would reintroduce the bug.** The doc is wrong, not the code.
- [ ] **Replace the linear scan with an index built at ingest.** `search_curated` loads every
      building and normalises every stored address *per query*. Invisible at 250; roughly two
      orders of magnitude past the 2.2 ms card budget at 222,433. Normalise once at ingest, store
      it, and use FTS5 for prefix/type-ahead.
- [ ] **Put borough in the index key.** 30,035 of 858,602 PLUTO lots (**3.5%**) share an address
      string with another lot. Substring matching compounds it — `FULTON STREET` matches every
      building on Fulton Street in every borough.
- [x] **Treat an address with no house number as unresolvable.** *Resolved 2026-08-15 — and the
      prescription in this box was wrong.* Already biting at 250 buildings: our curated set has
      250 buildings and **249** distinct addresses, because `FULTON STREET` appears twice against
      two different BBLs. PLUTO's address field is sometimes just a street.

      **Making them unresolvable would have deleted coverage.** Measured live,
      `/search?address=FULTON%20STREET` returned `3016840001` and `3017030009` as two rows both
      reading `FULTON STREET`, same borough. The defect is that a reader cannot tell them apart
      — not that the rows should be withheld. Both are real buildings with real violations, and
      hiding them repeats the failure the empty-address bug already taught. What shipped instead:
      search labels come from `model::export::display_address`, so a row reads
      `FULTON STREET (no house number on record)` — the same string the card shows.

      **Fixing it found the unfinished half of `7d0e414`.** That commit added BBL matching to
      `crates/mcp` only. `search_curated` — the path the website uses — never got it, so measured
      against production 2026-08-15 `/search?address=3015097501` returned **404**. The building
      with **96 open violations, 12 of them Class C** was findable by an agent and not by a
      tenant, which is the same coverage failure as before wearing a different surface.
      `search_curated` now also matches a BBL-shaped query: six to ten digits, so a house number
      like `603` stays an address search rather than colliding with every BBL containing it.
      Four tests, each confirmed to fail when the fix is mutated out.
- [ ] **Verify the Property Address Directory (`bc8t-ecyu`).** PLUTO holds one address per lot, so
      corner buildings and alternate entrances will never match what a user types. PAD is the
      authoritative all-addresses-per-BBL source, but it did not respond on the Socrata tabular
      endpoint and is probably a file download. **Confirm before planning around it** — two
      dataset ids have already turned out not to resolve.
- [ ] **Keep the two failure messages distinct.** "We could not find that address" and "that
      building is outside our coverage" are different, and the current handler already separates
      them. A silent geocoding gap is indistinguishable from a clean building, which is the
      failure mode that makes this the biggest citywide risk.

---

## Engineering debt carried from the audit

- [ ] **Basis-vector weight tests** (audit ledger item 7).
- [ ] **`enum ViolationClass`** to replace stringly-typed classes, plus a schema↔dispatch test
      (ledger item 10).
- [ ] **Extract `crates/agent`** out of the API crate (ledger item 10).
- [x] **Delete unused `components/ui`** (ledger item 10). *Done 2026-08-15.* 53 shadcn files,
      **6,083 lines**, and nothing outside the directory imported any of them. `eslint.config.js`
      had them in `globalIgnores`, so they were not even linted — dead code that was also
      exempt from the standards applied to everything else. Removing them let that ignore go,
      so every file under `src/` is now linted.

      **The dependencies mattered more than the files.** Enumerating every bare specifier
      imported under `src/` returns exactly five: `react`, `react-dom`, `react-markdown`,
      `react-router`, `remark-gfm`. The other **41 of 46** dependencies — 27 `@radix-ui/*`
      packages plus `cmdk`, `vaul`, `recharts`, `zod`, `react-hook-form` and the rest — arrived
      with the template and were reachable from nothing. Also removed `src/lib/utils.ts`, the
      `cn` helper, orphaned once `ui/` went.

      **Evidence it was safe: the built bundle is byte-identical.** `index-D0UGomiJ.js`,
      506.08 kB, same content hash before and after — those 41 packages shipped nothing, they
      were install-time and supply-chain surface only. `tsc`, `eslint` and `vite build` all
      clean. The `--radix-accordion-content-height` string in `tailwind.config.js` is a CSS
      variable name, not an import, so it needed no package. `npm audit` also went from **2
      high-severity advisories to 0**, since both sat in transitive tooling the trim reached.

---

## Deferred, with the reason

Not "someday" — each has a specific blocker.

- [ ] **Plain-English violation text.** Blocked on validation, not effort. A wrong rendering on a
      legal-rights tool is exactly what the Guardrails work exists to prevent, so it needs a
      housing lawyer to check a code→condition table. Ships raw until then.
- [ ] **Per-user state — accounts, saved buildings, history.** The gap in all four L2 cycles: a
      grep for `login`, `jwt`, `session`, `user_id` returns no implementation in any of them. On a
      lawyer's tool a saved caseload is client-adjacent data, so getting it wrong is worse than
      not having it.
- [ ] **Citywide coverage.** **The ~266 MB figure below is wrong, and the error is in the plan, not
      the arithmetic.** `design/database-layer.md:36-39` sizes the city assuming *open* violations
      only (2,858,719 rows). `crates/ingest/src/run.rs:255-261` stores **every** A/B/C violation
      regardless of status — visible in the shipped artifact, which holds 26,343 violations for
      250 buildings of which only 5,168 are open. Citywide that is 10,352,768 rows, roughly 3.6×
      the planned figure, and the artifact is 2.51 MB rather than the 1.21 MB the doc records.
      **Either the ingest starts filtering to open, or the citywide size estimate is off by more
      than the 256 MB machine can absorb.** That decision comes before any borough ingest.
      *Original entry, re-scoped 9 August 2026 — see `design/database-layer.md`:* The
      "cliff at ~14,500 buildings" assumed descriptions stored as raw text; stored in blocks of
      ~128 rows they compress **6.6x** (per-row compression manages only 1.3x — the ratio lives
      in the repetition between rows). Measured citywide: 2,858,719 *open* violations (only 25.6%
      of the 11.2M total), 222,433 buildings, and **~266 MB** against ~690 MB raw. **The baked-artifact design does not have to be replaced** — it needs
      compression and a 512 MB machine, roughly $3-4/month. Sequenced in the design doc.
- [ ] **Owner / portfolio dimension.** The record is published per building; owner linkage lives
      in a separate HPD registration dataset, so one landlord's twelve buildings are twelve
      unrelated records. Recorded as a gap in the research notes §4.6; the workflow behind it is
      inferred rather than observed.

      **No longer blocked — see `docs/owner-linkage.md`.** The claim that the registration
      dataset "has never been ingested" was true; the implication that it was unavailable was
      not. Both halves are public on Socrata on the same path the ingest already uses, and
      were queried live on 2026-08-12: **Multiple Dwelling Registrations `tesw-yqqr`
      (203,236 rows)** and **Registration Contacts `feu5-w2e2` (782,024 rows)**, carrying
      every column the linkage needs.

      **And it is largely solved already.** JustFix's `who-owns-what` (GPL-3.0) builds a graph
      over HPD contacts, joining on exact business address (high confidence) and on exact name
      corroborated by a >0.9 trigram address match (low confidence), then splits any component
      over 300 BBLs with Louvain. Their code cannot come into this tree without relicensing;
      the public datasets underneath it are City open data and are fair game. At our scale —
      250 buildings, one district — the fuzzy path and the splitting are both unnecessary.

      **Naming discipline, if built:** the data supports *registered contact*, never *owner*.
      Three states — `Linked` / `RegisteredAlone` / absent — because a lapsed registration and
      a genuinely single-building landlord must not render the same.
- [x] **MCP server**, so another agent can call HouseCheck as a tool. *Shipped `b8bd93b`
      (tools) and `f90de12` (the `ui://` resource).* `crates/mcp` on `rmcp`, all Rust.
      Card assembly moved to `crates/card` first so the agent and the website cannot drift
      apart; the API calls it through a wrapper, all ten call sites untouched. Verified over
      stdio against the real artifact rather than by unit test alone: 603 Putnam returns the
      same 27/100 and the same 33 open violations the site shows, an out-of-district BBL
      returns a coverage limit rather than a verdict, and `resources/read` returns a
      sandboxed iframe onto the deployed card. **Still open:** rmcp does not expose `_meta`,
      which is how MCP Apps links a tool to its UI, so the resource URI is named in the tool
      response instead. And `verify_export` — step 3, and the one worth demonstrating, since
      an agent that can *check* a document beats one that describes it.

      **Scoped in `docs/mcp-ui.md`.** The item assumed the answer is text; **MCP Apps** lets a
      tool return a UI resource the host renders. That matters here specifically: the card
      conveys a score, four pillars, three-state repair speed and a source per number, and an
      agent handed prose will paraphrase — the exact failure the export exists to prevent.
      All-Rust and licence-clean: `rmcp` (Apache-2.0, 19.8M downloads) plus a `ui://` resource
      whose shape we emit directly, since the published SDKs are TS/Ruby/Python and only
      assemble JSON. Sequenced text-first so step 1 is useful before any UI exists.
      **Open question, not a detail:** embedding makes the card page a framed surface, so
      `frame-ancestors` joins `CORS_ALLOWED_ORIGIN` as something to decide rather than widen.
- [ ] **The landlord ledger** (Sketch 3). Sell the right of reply, publish the silence. **Blocked
      on legal review — a hard gate, not a caveat.** Also needs identity verification and citywide
      coverage. Parked deliberately, not abandoned.

---

## Open questions

- [ ] **How long does the manual HPD Online pass actually take?** The single most useful number
      not held. One phone call.
- [ ] **Do attorneys already have a private workaround?** If so, Problem 1 is much weaker than it
      looks from outside.
- [ ] **Would a renter act on a bad score at the moment of decision, or sign anyway?**
      The load-bearing unknown of the whole project. Openigloo reached 3M+ renters and still
      pivoted to brokerage, which is at least consistent with "they sign anyway."
- [ ] **Reconcile the rent-burden figures.** The case study cites **51.6%** (RGB 2026); the 2023
      NYCHVS reports **29.5%**. Very likely different populations or thresholds rather than a
      contradiction — but the distinction has to be established before either is cited in front of
      someone who knows the other.
- [ ] **Is the habitability-risk white space actually empty**, or just not visible from public
      search? Absence of evidence is not evidence of absence.
- [ ] **Does HPD's description text need grouping before display?** At 83% distinct, near
      duplicates differ only by room, so 33 violations could render as a wall of near-identical
      lines.

---

## Deck and docs

- [ ] **Look at the deck on a real phone.** Every responsive check so far is DOM geometry —
      no screenshot has ever rendered. Particularly slides 5, 10, 12 and 14, which stack four
      cards each and become long scrolls. If that reads as a slog, collapse the small grey
      sub-note lines behind the main text rather than cutting content.
- [ ] **Confirm `/deck/` serves on Vercel.** `frontend/vercel.json` rewrites every path to
      `index.html` for the SPA. Vercel checks the filesystem before applying rewrites, so a static
      file should win — but it is worth one click after the first deploy.
- [ ] **Set the Vercel Root Directory to `frontend`.** Dashboard-only; cannot be done from here.
- [ ] **Link statutes in the deck.** NY Judiciary Law §§ 478/484 and *FTC v. DoNotPay* are cited
      by name with no link, because their URLs are not verified. Verify, then link.

---

## Related projects

Tracked here because they came out of the same work, not because they are HouseCheck.

- [ ] **SiteAssure — cut a signed installer.** The app is feature-complete (18 commands, all
      wired; zero stubs) and uninstallable. CI is green on Windows and macOS at
      `nessaisling-lab/L2-C2-Solution`.
- [ ] **SiteAssure — confirm push events trigger Actions on the fork.** Manual dispatch works;
      GitHub does not replay push events from before Actions was enabled.
- [ ] **Resona — cut a release build.** Same wall: finished and uninstallable.
