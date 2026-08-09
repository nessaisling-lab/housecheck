# Market Framing Notes — HouseCheck

**Thinking past one user:** who else has this problem, roughly how many, and what they do
instead today.

**Started:** 2026-08-09. Living document, same standard as the Research Notes — every figure
carries its source, and where a number is derived rather than sourced it says so.

---

## Question 1 — Where I'm starting from

I have run into this distinction, and not as theory. It happened twice in this cycle, in the
last week, and both times it changed an answer I had already written down.

### The first time: the denominator collapsed by a factor of 75

While mapping the industry I wrote down who HouseCheck serves and reached for the obvious
number: **~2.5M NYC renters.** It is true, it is sourced, and it is the number anyone would
put on a slide.

Then a harder question got asked — *does a renter actually use this at the moment that
matters?* — and that is a behavioural question, not a market-size one. A tool used at the
moment of decision does not serve a population. It serves a **moment**.

| Framing | Count |
|---|---:|
| NYC renters — the population that *has* the problem | ~2,500,000 |
| Units actually available at a point in time — the people who can *act* on it | **33,210** |

**About 1.3%. Roughly 1 in 75.**

Two independent routes agree, which is why I trust it: the sourced vacancy figure (1.41%,
NYC HPD, 2023) says 33,210 units are on the market at once, and an 8–12% turnover model
against 2,357,000 housing units implies 21,800–32,600 households searching in any six-week
window. Different methods, same order of magnitude.

**What that taught me concretely:** "how many people have this problem" and "how many people
can use this product" are different questions with answers two orders of magnitude apart, and
the second one is the one that governs. It also explained something I could not explain
before — Openigloo reached 3M+ NYC renters and still pivoted into brokerage. Reach was never
their constraint. Only about one in seventy-five of the people they reached could act at any
given time, and the ones who could were standing next to a transaction.

### The second time: one building versus 28% of them

Separately, auditing our own build, I found a single building — 689 Myrtle Avenue — whose
Health Card said it had *"a clean hazardous-violation record"* while HPD held **seven open
immediately-hazardous violations** against it.

One building is an anecdote. It is a bug report, and someone can reasonably ask whether it
matters. So I recomputed all 250:

- mean score **69.5 → 63.0**
- **72 of 250** buildings changed band
- **69 of the 72 moved down**, and none could move up, because the score starts at 100 and
  subtracts — a violation you fail to fetch can only ever make a building look *better*

That third line is the point. At one building it looks like a data glitch. At 250 it is a
**structural bias with a direction**: a tool built to give tenants leverage against landlords
was systematically flattering landlords. Same defect, completely different severity, and the
difference is only visible in aggregate.

### The third time, and it is a constraint rather than an insight

Coverage. HouseCheck serves 250 buildings. I measured what the current architecture — a
SQLite file baked into the container image — actually supports:

| Coverage | Buildings | Artifact size | Verdict |
|---|---:|---:|---|
| One community district (today) | 250 | 1.3 MB | fits the 2 MB page cache |
| One borough subset | 5,000 | 25 MB | fits the 256 MB VM |
| All NYC community districts | 48,000 | 244 MB | **exceeds the VM** |
| All HPD-registered multifamily | 180,000 | 914 MB | **exceeds the VM** |

Scale is not an ambition here. It is a number with a cliff in it at roughly 40,000 buildings,
and it tells me which decisions are cheap now and which ones require replacing the design.

### What I already believed, and what changed

**What I brought in:** that a product should work for a real person before it works for a
market, and that starting from one concrete user is the honest way to build. I still believe
that — the whole product was designed around one person standing in a hallway with fifteen
minutes and a lease in front of them.

**What changed:** I used to treat "and lots of people have this problem" as a supporting
argument you add afterwards. This cycle it stopped being supporting and started being
*load-bearing* — it changed the denominator by 75×, it turned a bug into a bias, and it put a
hard ceiling on the architecture. None of those were visible from the single-user view.

**The distinction I would now draw:** solving for one person tells you whether the product is
*right*. Thinking at scale tells you whether it is *viable*, and — the part I did not expect —
whether a defect is *serious*. Those are three different jobs and only the first one is
answerable by talking to one user.

### Where I'd actually run into this before: Cycles 1 through 3

I went back and read the code rather than trusting memory. Four projects, ~24,000 lines of
first-party code, one developer.

| | Project | What it is | First-party code |
|---|---|---|---|
| **L2 C1** | **Resona** | Local voice-to-text desktop app. Grew out of a Whisper clone | 736 Rust / 458 TS |
| **L2 C2** | **SiteAssure** | OSHA compliance app, built on Resona's engine | 551 Rust own (+5,184 vendored) |
| **L2 C3** | **Ziqpu** | Astrology agent, descended from my L1 Cycle 4 capstone | 6,718 Rust across 7 crates |
| **L2 C4** | **HouseCheck** | This one. NYC tenant building-health scores | 5,591 Rust / 4,303 TS |

The honest finding is not a clean upward arc, and it is not flattering:

> **In four cycles I never once built an authentication system, a user record, or any
> server-side state belonging to a specific person.** A grep for `login`, `jwt`, `session`,
> `user_id`, `tenant` returns no implementation hits in any of the four.

What actually changed was not *how many users* the code served. It was **which non-author
human I was designing against** — and that moved, with a reversal in the middle.

**Cycle 1 — Resona had the most multi-user *intent* and the least multi-user *code*.** A
`Tier` enum, an entitlements struct checked at every command, a shipped paywall modal with a
7-row feature matrix, and a PRD specifying a genuinely multi-tenant Team plan at $25/user/mo
with shared workspaces and admin controls, measured by DAU and free→paid conversion.
Underneath: one global `AppState`, one process, one person. `start_dictation` tears down any
prior session, so concurrency is *impossible by construction*. `validate_license` is
`key.starts_with("PRO-")`, marked DEMO ONLY. Zero network calls anywhere in the shipping app.

**Cycle 2 — SiteAssure went backwards, on purpose.** "Single device, single user, offline"
appears verbatim in the README, the kickoff and the build plan. The schema literally comments
`-- author id (single user in v1)`. The Team tier and the conversion metrics are gone.

But it introduced a genuinely new non-author reader: **the verifier.** The SHA-256 hash chain
— `entry_hash = sha256(prev_hash + payload_hash)`, append-only, re-walked by `verify()` —
exists so an OSHA inspector or an insurer can *disbelieve me and check*. That is an audience
expansion along a completely different axis than headcount.

**Cycle 3 — Ziqpu scaled installs and contributors, not users.** Twelve release tags, a 3-OS
CI matrix, a CLA, DCO enforcement, CODEOWNERS, an MCP server so third-party hosts can drive
it. The *item* axis genuinely scales: Postgres 16 with a pooled 5 connections, 5,271 tickers,
a 69,458-city gazetteer. The *user* axis does not: no auth, no rate limits, no pagination, one
API key per process. The tell is a table keyed
`PRIMARY KEY (user_chart_hash, choice_ticker)` — the only real multi-user data structure in
three cycles — that **no code ever writes to.**

**Cycle 4 — HouseCheck is the first project whose request path was written against
strangers.** And the proof is in the comments, which say so out loud: the rate limiter's note
reads *"/agent/chat is the first endpoint here that costs real money per request, so an
unlimited public endpoint is a way for a stranger to run up the bill."* **The first endpoint
here** — I knew it was a first. `ConcurrencyLimitLayer(64)` answers a question that does not
arise for one user. The startup guard refuses to boot on an empty database *"rather than serve
a 404 for every address under a green health check"* — an operator worrying about other
people's requests. And correctness finally got evaluated at population scale: the paging bug
was measured across all 250 buildings, not spot-checked on one.

### The constant, which is the part I did not expect

In all four projects the multi-user affordance is **modeled, annotated, and left inert** — and
I was right about it every time. Resona's licensing file says client-side gating is bypassable
and that revenue needs a signed server entitlement, then defers it. SiteAssure carves `status`
and `role` into the schema labeled "v2 hook" and nothing reads them. Ziqpu builds the
`user_chart_hash` key and never writes the table. HouseCheck's store says "no accounts —
design decision #3."

Paired with zero telemetry in all four, the pattern is:

> **The first non-author human I model is always an auditor, an attacker, or a cost risk. The
> customer only ever appears in a document.**

The market research is consistently more populous than the code — Resona's Team tier,
HouseCheck's 91,918-building ARR table.

Said plainly: four cycles, four correct diagnoses of the same missing piece, zero
implementations. The capstone reached *many concurrent anonymous clients*, which is a real
step and a well-engineered one. But a read-only database baked into an image cannot hold a
user, and localStorage is not an account. **I got to no-user-at-scale, not to multi-user.**

That is the honest place I am starting from for this block.

---

## Question 2 — Who else has this problem, how many, and what they do now

### The broader group

The problem is not "renters cannot read HPD data." It is one level up:

> **A building's condition is a matter of public record, and the people whose safety, money
> or case depends on that record cannot use it at the moment they need it.**

Framed that way the group is much wider than renters, and it splits cleanly by *when* they
need it.

**At a decision point — episodic, high stakes, once every few years**

| Group | How many | Source |
|---|---:|---|
| NYC households in-market for an apartment | **33,210** at any moment | NYC HPD, 1.41% vacancy 2023 |
| — the same, annually | 188,000–283,000 | 8–12% turnover on 2,357,000 units |
| Rent-stabilised/controlled tenants with rights they must know to use | ~1M units, ~2.5M people | NYCHVS-derived |

**In daily practice — repeated, professional, low stakes per query**

| Group | Rough size | How often |
|---|---:|---|
| Tenant lawyers and Legal Aid housing staff | Hundreds citywide | **Several times a day** |
| Tenant organisers | Hundreds | Weekly |
| Housing Court staff | Hundreds | Per case |
| HPD inspectors | Low hundreds *(needs a source before quoting)* | Daily |
| Journalists and researchers | Dozens | Per story |
| Small property managers tracking their own portfolio | Thousands | Ongoing |

**The observation that matters more than either headcount:** the people who need this most
often are not the people there are most of. A renter needs it **twice in a decade**. A tenant
lawyer needs it **twice a day**.

### What they do instead today

Nobody is doing nothing. Everyone has a workaround, and every workaround is bad in a different
way.

| Group | What they do now | Why it fails |
|---|---|---|
| Renters | Cross-reference HPD Online, DHCR lists and Census tables by hand — or, far more often, sign without checking | Three portals, none designed for a layperson, at the exact moment there is no time. 11.1M violation records, public and illegible |
| Tenant lawyers | Pull records case by case from the same portals, manually, every time | Works, but it is per-client manual labour that never compounds |
| Organisers | Build spreadsheets of buildings and landlords by hand | Goes stale immediately; no one else can reuse it |
| Journalists | FOIL requests and bulk Open Data downloads, analysed ad hoc | Fine for a story, useless as a lookup |
| Landlords/managers | Yardi, RealPage and similar | Shows *their* buildings, not a comparative read; priced for portfolios |
| Everyone | Word of mouth, a walk-through, the broker's description | The broker is paid to close the lease |

**The honest read on the alternatives:** the data being free and public is precisely why nobody
has built the good version. There is no paywall to break — only an interpretation problem, and
interpretation is hard to charge for.

### Which group I would pick as primary — and it is not the obvious one

The question is renters (volume) or lawyers and organisers (frequency). **I would design for
the professionals and reach for the renters**, and the argument is not sentimental.

**Renters are not addressable.** 33,210 people are in-market at any moment, they do not know
they need this until the week they need it, and there is no channel that reaches them at that
moment which is not simply buying attention. Openigloo proved the negative case here: they
reached 3M+ NYC renters and still had to pivot into brokerage, because reach was never the
constraint — **timing** was. Spending to acquire someone who needs the product for six weeks
every four years is the hardest possible acquisition maths.

**The professionals are findable.** There are hundreds of them, at a countable number of named
organisations — Legal Aid, Housing Court Answers, Met Council, the tenant unions. You can
reach essentially the entire population with a list and a phone. That is not a market, but it
is a *distribution channel that exists*, which is more than the renter side has.

**And frequency drives quality.** A user who opens the tool twice a day finds every gap in a
month. The daily user is the one who will say "seven open Class C" is not enough — they need to
know it is *no heat, twice, unresolved since March*. That is the biggest product gap in the
whole landscape and it is invisible from the renter side, because a renter does not know what
to ask for. **Building for the professional makes the product good enough that the renter
version is worth having.**

**What this is not:** it is not a claim that lawyers are the revenue. They are grant-funded and
close to broke — they sit at the very top of the alignment gradient in §6, which is exactly why
they will not pay. Primary market here means *primary design target and first distribution
channel*, and the honest structure is:

- **Design for** the daily professional user — they make it correct.
- **Reach** renters — they are the mission and the volume.
- **Charge** neither, yet. §6 is unresolved and picking a payer before resolving it is how both
  comparables ended up somewhere other than where they started.

### One constraint this decision does *not* have

Capacity. The read path was measured at 2.2 ms per Health Card, which is **400,000–1,300,000
daily users on a single $2/month VM**. Whichever group is picked, the machine is not the reason.
The binding constraints are coverage — 250 buildings today, a hard architectural cliff at
~40,000 — and the per-request cost of the LLM endpoints. Full working in
`docs/reflection/capacity-and-ceilings.md`.

## Question 3 — *(pending)*

---

## Working material for the questions ahead

Collected while answering Q1, so it is here when the later questions need it.

### Who else has this problem

| Group | Rough size | Frequency of need |
|---|---:|---|
| NYC renters signing a lease | ~33,210 in market at once; 188K–283K/yr | Once every few years |
| Rent-stabilised / controlled tenants | ~1M units, ~2.5M tenants | Ongoing, situational |
| Tenant lawyers and Legal Aid staff | Hundreds citywide | **Daily, per client** |
| Tenant organisers | Hundreds | Weekly |
| Housing Court staff | Hundreds | Per case |
| HPD inspectors | ~400 (needs a source before use) | Daily |
| Journalists and researchers | Dozens | Per story |

The pattern already visible: **the people who need it most often are not the people there are
most of.** A renter needs this twice in a decade; a tenant lawyer needs it twice a day.

### What they do instead today

- **Renters:** cross-reference HPD Online, DHCR stabilisation lists and Census tables by hand —
  or, far more commonly, sign without checking. NYC Open Data holds **11.1M** HPD violation
  records; they are public and effectively unreadable at the moment of decision.
- **Tenant lawyers and organisers:** pull records case by case from the same portals, manually,
  every time.
- **Landlords and managers:** commercial portfolio tools (Yardi, RealPage) that surface their
  own buildings, not a comparative read.
- **Everyone else:** word of mouth, a walk-through, and the broker's description.

### Numbers already verified elsewhere in these notes

- 91,918 multifamily buildings in NYC — REBNY, *Data Over Rhetoric*, Feb 2026
- 2,357,000 total housing units — 2023 NYCHVS
- 1.41% rental vacancy, 33,210 units available — NYC HPD, 2023
- 11.1M HPD violation records — NYC Open Data
- Our own coverage: 250 buildings, 26,306 violations, in one community district
