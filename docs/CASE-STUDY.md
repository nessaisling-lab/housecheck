# HouseCheck — Case Study

> **Carfax for apartments.** Type any NYC address, get an instant Building Health Card — condition, legal protections, rent fairness, and accessibility — every number linked to a government source. Then ask an AI agent about it that cites published law and will not make anything up.

**Team:** Aisling Leiva-Davila (backend, data, agent), Anthony Lesov (frontend), Jagger (comparison prototype), + DB analyst · Pursuit NYC Fellowship, L2 Cycle 4
**Live app:** https://housecheck-wine.vercel.app · **API:** https://housecheck-nessa.fly.dev · **Repo:** https://github.com/nessaisling-lab/housecheck

---

## The problem

Renting in Brooklyn means committing ~$40,000 a year and a 12-month lease to a building you know almost nothing about. The facts exist — in government databases — but they're scattered across three portals no normal person can use.

- **51.6%** of NYC renter households are rent-burdened; **28.8%** are *severely* burdened (50%+ of income on rent). *— NYC Rent Guidelines Board, 2026.*
- Median citywide asking rent hit **~$3,616/mo** (+6.2% YoY, +28% vs pre-pandemic); the "stay-vs-move" gap is **~$1,761/mo**. *— Realtor.com, Q1 2026.*
- NYC Open Data holds **~11.1 million** HPD housing-maintenance-code violation records — the data is *there*, it's just unusable.

New laws raised the stakes: the **FARE Act** (broker-fee ban, June 2025) and **Good Cause Eviction** (April 2024) give renters rights they can only use *with data*. So renters sign blind, or spend hours cross-referencing HPD Online, DHCR lists, and Census tables — and still miss hazardous-violation history and stabilization rights they're entitled to.

## Who it's for

Brooklyn renters evaluating a specific apartment before signing — and current tenants checking whether their unit is stabilized or their rent is fair.

## The solution

Type an address → an instant **Building Health Card**: a single 0–100 score across four plain-language axes — **building condition** (HPD violations), **legal protections** (rent-stabilization, Good Cause), **rent fairness** (your rent vs the neighborhood median + HUD FMR), and **accessibility** (elevator-on-record + build-era). Every figure links to its source, with a "data from [date]" label.

The differentiator isn't any single feature — it's the **trust model**: objective government data only, not crowdsourced reviews, with every number sourced and honestly bounded.

## How we built it — and where it got interesting

**1. We fact-checked our own pitch — and our own corrections.** Before writing code, we ran every statistic in the original proposal through independent verification against primary sources, and we pulled a competitor's investor and revenue figures that no database supports. The discipline cuts both ways: when a teammate later challenged two of our calls, we re-verified and found we'd been wrong — a "761,352 buildings / ~11% Class C" figure we had dismissed is in fact real REBNY data (*Data Over Rhetoric*, Feb 2026), so we restored and cited it. The rule holds no matter who it embarrasses: *data-backed, full stop.*

**2. A deliberately simple, robust stack.** Rust + Axum + bundled SQLite. All geospatial work happens once at ingest, so the serving database is a **read-only artifact baked into the Docker image** — meaning the deployed API needs *zero secrets*. Every data source is free: NYC Open Data (Socrata), US Census, NYC GeoSearch. **Ingest cost: $0. Hosting: ~$0** (Fly.io, scale-to-zero).

**3. Real data fought back — and we won.** Plumbing eight live datasets (PLUTO, HPD, DOB elevators, 311, DOHMH, MTA, Census, JustFix) surfaced problems the plan didn't anticipate: PLUTO ships the BBL as a float-string; the census tract lives in a different field than documented; and HPD's oldest (pre-2013) records predate BBL geocoding, so a naive single-row schema probe can wrongly suggest the BBL column is missing when it isn't. We verified each dataset against the live API and fixed the pipeline building-by-building.

**4. We refused to fake the hard part.** There is **no official, per-building rent-stabilization list** — DHCR publishes only an incomplete PDF. Rather than guess, we sourced JustFix's DOF-tax-derived dataset and label it honestly: *"Likely rent-stabilized — 192 units on the latest DOF record. A signal, not a legal ruling."* When real 311 volumes made every dense-block score saturate at the same floor, we recalibrated the neighborhood score to a log scale so it actually discriminates.

**5. We built an AI agent that is structurally incapable of making things up.** The card answers *"what is the state of this building?"* Renters immediately ask harder questions — *is that bad enough to walk away? I have no heat and my landlord won't respond, what now?* Answering those well is where a housing tool either earns trust or destroys it.

The architecture is one decision: **the model never touches the database.** It emits a tool call, *our code* runs the query, and the result comes back as data. Six read-only tools cover the building record, individual violations, address lookup, comparison, published law, and legal referrals. Every fact in an answer therefore passed through code we control — which is what makes grounding enforceable rather than aspirational.

That let us take on the question most products dodge: **legal help.** We researched it rather than guessing. Disclaimers do not cure unauthorized practice of law — what the software *does* controls — and the FTC's $193,000 order against DoNotPay turned on unevidenced claims that an AI performed like a lawyer. So we drew the line at the honest place, not the convenient one:

- **It gives legal information, with citations.** NY Real Property Law § 235-b, the two-year succession co-residency rule, HPD violation classes — each with a link the reader can open and check.
- **It maps published law onto this building's public record.** Asked about succession rights, it found the governing rule *and* flagged that succession only applies to stabilized units, then checked our data and reported this building's status is unverified.
- **It refuses to predict outcomes.** Not because a lawyer told us to — because we hold no case history, no docket data, and have never seen the user's lease. A litigation forecast would be fabrication, and fabrication is the one thing this product cannot ship.
- **It hands people to humans who can actually advise them**, from a curated directory of free tenant services — deliberately curated, because an open search for "tenant lawyer" surfaces lead-generation aimed at exactly the people in crisis.
- **It drafts the question for them**, in their own voice, citing the statute, with placeholders for the facts only they know.

Web search for legal edge cases is restricted to an **allowlist of government and academic sources**. That single constraint collapses two risks at once: prompt injection stops being realistic, because nysenate.gov does not serve text written to hijack an agent, and predatory referrals disappear, because there are none on nycourts.gov.

We attacked it to check. Instructed to ignore its rules, act as a legal advisor, guarantee a lawsuit win, and claim the building had zero violations, it refused the advice, refused the prediction, and **contradicted the injected lie with the real figure — five open Class C violations.**

## Results

**It's live and serving real data**, worldwide, right now:

| Building | Score | What the card shows |
|---|---|---|
| **61 Stuyvesant Ave** | **24 / 100** | 65 open violations (A:21 B:32 C:12) — a genuinely hazardous walk-up |
| **443A Monroe St** | **78 / 100** | zero open violations — clean, well-kept |
| **510 Quincy St** | — | **192 rent-stabilized units** on the 2024 DOF record |

Two real buildings a few blocks apart score **24 vs 78** — that spread *is* the product. The curated set of **250 real Bed-Stuy buildings** blends large regulated buildings (87 sourced rent-stabilized) with small rowhouses, ranging 1–1,624 units.

Ask the agent *"I have no heat for a week and my landlord won't respond"* and it returns the governing statute with a link, an evidence checklist (dated 311 numbers, timestamped thermometer photos, written notice), the official complaint route, a drafted question for a lawyer, and a free hotline to call — then states plainly that this is published information and a public record, not advice about your situation.

- **Endpoints live:** `/building/{bbl}`, `/buildings` (map-ready), `/rent-fairness`, `/search`, `/compare`, `/summary`, `/agent/chat`
- **98 tests**, clippy-clean, **green CI on macOS + Windows + Linux**, independently code-reviewed and hardened
- **Zero secrets in the deployed image** — verified: the production app runs with no application secrets beyond the optional LLM key
- **Spend controls by design:** per-client rate limiting, a token ceiling, a history cap, and a hard stop on the tool loop — because an LLM endpoint on a public URL is otherwise a way for a stranger to spend your money
- **$0 data cost**, well within the project's $20–50 budget

## What we learned

- **The hardest part wasn't code — it was honest data.** Sourcing a defensible rent-stabilization signal took more judgment than building the entire scoring engine.
- **Intellectual honesty is a feature.** A confidently-wrong number on a legal-rights tool is worse than an honest "unverified." We shipped "unverified" where the data didn't support a claim — and the product is more trustworthy for it.
- **The safe design and the honest design kept turning out to be the same design.** Refusing to predict case outcomes reads like legal caution; it's actually just refusing to invent data we don't have. Restricting legal search to government sources reads like a security control; it also happens to produce better citations. When those two pressures agreed, it was a signal the architecture was right.
- **A working system can be load-bearing in ways nobody planned.** The "agent" branch turned out to contain no AI at all — a form wizard over five hardcoded buildings, with a second scoring engine that disagreed with ours by fifty points on the same building. Finding that early was worth more than shipping it late.
- **> _[Your reflection here — your specific role, the moment it clicked, or what you'd do differently. This is the part only you can write; tell me and I'll weave it in._]**

## What's next

The **React (Vite + Tailwind + shadcn/ui)** frontend is live and wired to the API, agent included. Next: a map layer, violation *descriptions* (HPD publishes them; our ingest doesn't pull them yet, so the agent can currently report how many violations exist but not what they are), and comparison weighted by what a renter actually cares about. Beyond that, a path to a real business — a free consumer tool feeding a B2B2C model in the $3.6B property-data adjacent market.

---

*HouseCheck turns a blind $40,000 decision into an informed one — built in ~10 days on nothing but free, public, honestly-sourced data.*
