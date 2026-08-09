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

### Where this leaves a gap I should be honest about

Earlier cycles are not covered here. This document draws only on Cycle 4, because that is
where the evidence lives and where the numbers can be checked. If this distinction came up in
Cycle 1 or Cycle 2 work, that belongs in this section and I should add it — I am not going to
reconstruct it from memory and present it as evidence, given the standard the rest of these
notes are held to.

---

## Question 2 — *(pending)*

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
