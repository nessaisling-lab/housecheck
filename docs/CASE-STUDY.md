# HouseCheck — Case Study

> **Carfax for apartments.** Type any NYC address, get an instant Building Health Card — condition, legal protections, rent fairness, and accessibility — every number linked to a government source.

**Team:** Aisling Leiva-Davila (backend + data), Anthony Lesov (frontend), Jagger (agent), + DB analyst · Pursuit NYC Fellowship, L2 Cycle 4
**Live:** https://housecheck-nessa.fly.dev · **Repo:** https://github.com/nessaisling-lab/housecheck

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

**1. We fact-checked our own pitch first.** Before writing code, we ran every statistic in the original proposal through independent verification. Several were **fabricated** — a "761,352 buildings" figure and an "11% Class C" stat attributed to a REBNY report that contains neither; an investor and revenue figure for a competitor that no database supports. We pulled them. That set the rule for the whole project: *data-backed, full stop.*

**2. A deliberately simple, robust stack.** Rust + Axum + bundled SQLite. All geospatial work happens once at ingest, so the serving database is a **read-only artifact baked into the Docker image** — meaning the deployed API needs *zero secrets*. Every data source is free: NYC Open Data (Socrata), US Census, NYC GeoSearch. **Ingest cost: $0. Hosting: ~$0** (Fly.io, scale-to-zero).

**3. Real data fought back — and we won.** Plumbing eight live datasets (PLUTO, HPD, DOB elevators, 311, DOHMH, MTA, Census, JustFix) surfaced problems the plan didn't anticipate: HPD's violation table has **no BBL column** (we query by borough + tax block and reconstruct it); PLUTO ships the BBL as a float-string; the census tract lives in a different field than documented. We verified each dataset against the live API and fixed the pipeline building-by-building.

**4. We refused to fake the hard part.** There is **no official, per-building rent-stabilization list** — DHCR publishes only an incomplete PDF. Rather than guess, we sourced JustFix's DOF-tax-derived dataset and label it honestly: *"Likely rent-stabilized — 192 units on the latest DOF record. A signal, not a legal ruling."* When real 311 volumes made every dense-block score saturate at the same floor, we recalibrated the neighborhood score to a log scale so it actually discriminates.

## Results

**It's live and serving real data**, worldwide, right now:

| Building | Score | What the card shows |
|---|---|---|
| **61 Stuyvesant Ave** | **24 / 100** | 65 open violations (A:21 B:32 C:12) — a genuinely hazardous walk-up |
| **443A Monroe St** | **78 / 100** | zero open violations — clean, well-kept |
| **510 Quincy St** | — | **192 rent-stabilized units** on the 2024 DOF record |

Two real buildings a few blocks apart score **24 vs 78** — that spread *is* the product. The curated set of **250 real Bed-Stuy buildings** blends large regulated buildings (87 sourced rent-stabilized) with small rowhouses, ranging 1–1,624 units.

- **Endpoints live:** `/building/{bbl}`, `/buildings` (map-ready), `/rent-fairness`, `/search`, `/compare`, `/summary`
- **~76 tests**, clippy-clean, **green CI on macOS + Windows + Linux**, independently code-reviewed and hardened
- **$0 data cost**, well within the project's $20–50 budget

## What we learned

- **The hardest part wasn't code — it was honest data.** Sourcing a defensible rent-stabilization signal took more judgment than building the entire scoring engine.
- **Intellectual honesty is a feature.** A confidently-wrong number on a legal-rights tool is worse than an honest "unverified." We shipped "unverified" where the data didn't support a claim — and the product is more trustworthy for it.
- **> _[Your reflection here — your specific role, the moment it clicked, or what you'd do differently. This is the part only you can write; tell me and I'll weave it in._]**

## What's next

A **Dioxus (Rust→WASM)** frontend rendering the card against the live API; a MapLibre + Protomaps map layer; and a path to a real business — a free consumer tool feeding a B2B2C model in the $3.6B property-data adjacent market.

---

*HouseCheck turns a blind $40,000 decision into an informed one — built in ~10 days on nothing but free, public, honestly-sourced data.*
