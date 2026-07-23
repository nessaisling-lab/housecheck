# HouseCheck — Design Spec (v1)

**Date:** 2026-07-21
**Project folder:** `D:\L2 Cycle 4\Housecheck Antonin Idea`
**Team:** 2–3 person Pursuit capstone team; Aisling = primary builder (Rust core)
**Timeline:** 10-day sprint (curated MVP), full-NYC as stretch
**One-liner:** *Carfax for apartments.* Type an NYC address → an instant Building Health Card (0–100) covering building condition, legal protections, neighborhood context, and (user-supplied) rent fairness — every number linked to a government source.

> This spec supersedes the original `Housecheck Proposal.md` where they differ. Two deliberate overrides from that proposal: (1) a **real Rust backend** instead of static JSON, because we're treating this as a product (robustness, security, fast data processing); (2) all statistics replaced with **independently verified** figures (see Appendix A). Fabricated claims from the proposal have been removed.

---

## 1. Problem (verified figures only)

Renting in NYC means committing ~$40k/year on almost no information about the building.

- **51.6%** of NYC renter households are rent-burdened (30%+ of income); **28.8%** are severely burdened (50%+). Bronx 37.1% → Manhattan 24.2%. — *NYC Rent Guidelines Board, 2026 Income & Affordability Study.*
- Median **citywide** asking rent Q1 2026 ≈ **$3,616/mo**, **+6.2% YoY**, **+28% vs pre-pandemic**; Manhattan ≈ $4,878. — *Realtor.com Q1 2026 NYC Rental Report.*
- In-place-vs-market "stay-vs-move" gap ≈ **$1,761/mo**. — *same report.*
- **~11.1M** HPD housing-maintenance-code violation records on NYC Open Data. — *dataset `wvxf-dwi5`.*
- Renters manually cross-reference OpenIgloo + HPD Online + Who Owns What. No single tool combines condition + legal + rent context in a fast, mobile-first flow.

**Why now:** FARE Act (broker-fee ban, Jun 11 2025) and Good Cause Eviction (Apr 20 2024) both increase renters' need for building/rent data to exercise new rights.

## 2. Users

- **Primary:** NYC renter mid-search, evaluating a specific building before signing.
- **Secondary:** current tenant checking whether their rent/unit is fair or stabilized.
- **Non-user (v1):** landlords. B2B is a later phase, out of MVP scope.

## 3. MVP scope

**In (curated demo set):** ~50–200 pre-vetted real buildings in **Brooklyn** (chosen — highest violation volume makes the condition score pop). Every building guaranteed to resolve address→BBL and have real data. This is the demo-safe core. Plus an **accessibility indicator** (elevator-on-record + build-era inference + nearby ADA transit) — data-backed "access-likelihood," never a certification.

**Stretch (post-curated):** one-borough live, then full-NYC live.

**Out:** landlord/B2B tooling, payments, accounts/auth, real-time crime, building-level LGBTQ+ safety score (no data exists — reframed as legal-protection + community-resource surfacing).

## 4. Architecture

```
[Ingest — offline, run once + on data refresh]        [Serve]                     [Client]
NYC Open Data CSVs ─┐                                                             
  HPD violations    │                                                            
  HPD registrations ├─▶ DuckDB ETL ──▶ SQLite + SpatiaLite ──▶ Rust + Axum API ──▶ React + Vite +
  311 (2020+)       │   (filter to      (buildings+geom,        (SQLx/rusqlite)     Tailwind + Mapbox
  DOHMH restaurants │    curated set,    violations, 311,        GET /search         - address search
Census ACS B25064   │    resolve BBL,    restaurants,            GET /building/{bbl}  - Building Health Card
PLUTO (BBL+geom)    │    join tract,     acs_rent_by_tract,      POST /rent-fairness  - "enter your rent"
DHCR / WOW rent-stab┘    precompute      rent_stab, scores)      tower: CORS,         - map + rent heatmap
                          scores)                                rate-limit, tracing
Deploy: backend → Shuttle.dev or Fly.io · frontend → Vercel
```

### 4a. Data layer — SQLite + SpatiaLite (user's choice)
- **DuckDB** does heavy CSV crunching at ingest (311 alone is ~21.9M rows), then exports clean tables.
- **SQLite + SpatiaLite** is the serving DB: embedded, sub-ms reads, ships with the binary, trivial deploy. SpatiaLite provides the geospatial ops (point-in-tract, nearby-restaurants) we'd otherwise hand-roll.
- Rust access via `rusqlite` (bundled + SpatiaLite extension) or `sqlx` sqlite. Read-mostly reference data → SQLite's write-concurrency limits don't bite.
- **Tradeoff accepted:** less production-heft than Postgres+PostGIS, but far simpler deploy and fastest reads for read-only data. Revisit only if we go full-NYC-live with heavy concurrent writes.

### 4b. Backend — Rust + Axum
- Endpoints: `GET /search?address=` (geocode → BBL), `GET /building/{bbl}` (full card), `POST /rent-fairness {bbl, monthly_rent}` (% vs tract median + HUD FMR).
- Middleware (tower/tower-http): CORS, rate-limiting, structured tracing.
- Reference (not copy): reuse Cargo/Axum/SQLx patterns from the Launch Radar scaffold.

### 4c. Geocoding — address → BBL  ⚠️ corrected
- **GeoClient is no longer usable** (v1 dead Oct 2025; v2 keyed). Use **GeoSearch** (`geosearch.planninglabs.nyc`, keyless) for live lookups, or bundle **PLUTO/Geosupport** offline. For the curated set, BBLs are pre-resolved at ingest — zero runtime geocoding risk.

### 4d. Scoring engine (Rust module)
0–100 Building Health Card from transparent weighted sub-scores:
- **Building condition** — violation count × severity (Class C/B/A) × recency decay.
- **Rent fairness** — user rent vs tract median (only when user supplies rent).
- **Legal protections** — stabilized? Good-Cause-covered? (boosts score / adds badges).
- **Neighborhood signal** — 311 complaint density, nearby restaurant grades.
- Weights documented and surfaced in-app ("show the math") to defuse "score feels arbitrary."

### 4e. Frontend — React + Vite + Tailwind + Mapbox GL JS
Mobile-first, dark theme, animated score gauge. Design tokens pulled from the Nisaba/Ziqpu brand kits. Sections: search → Health Card → rent-fairness input → map/heatmap.

## 5. Building Health Card — data sources (verified IDs)

| Section | Shows | Source | ID / endpoint | Notes |
|---|---|---|---|---|
| Building condition | Violation counts A/B/C + timeline | NYC HPD Open Data | `wvxf-dwi5` (~11.1M rows, daily) | Class C = immediately hazardous |
| Owner / registration | Owner + managing agent | HPD Open Data | `tesw-yqqr` + `feu5-w2e2` | Two-table join on RegistrationID → BBL |
| Legal protections | Rent-stabilized? Good Cause? | DHCR list + JustFix Who Owns What (nycdb) | whoownswhat.justfix.org | **Incomplete** — label as "best-available," not authoritative |
| Rent fairness | User rent vs tract median + HUD FMR | US Census ACS | `B25064_001E`, **5-year**, tract-level (needs free API key) | User supplies their rent (numerator has no public source) |
| Neighborhood | 311 density, restaurant grades | NYC Open Data / DOHMH | `erm2-nwe9` (2020+, ~21.9M) · `43nn-pn8j` (~294k) | 311 is 2020→present only |

**Total data cost: $0.** All free/public.

## 6. Rent fairness (the flagship, fixed)

No free dataset gives a specific unit's asking rent. So: **the app asks the user for their rent**, then compares it to (a) the Census tract median gross rent (B25064) and (b) HUD Fair Market Rent by bedroom count. Output: "Your rent is X% above/below the neighborhood median." Fully data-backed and honest; costs one input field. This keeps rent fairness as a headline feature without scraping StreetEasy (ToS/fragility/trust risk).

## 7. Security & data policy (adapt from SiteAssure DATA_POLICY.md)
- **Collect minimal:** address queried + optionally the rent the user types. No accounts, no PII, no tracking in v1.
- **Government data only** = the trust model. Every number links to its source dataset.
- Parameterized SQL (no injection), input validation on address/rent, HTTPS, rate-limiting, secrets server-side only.
- "Data from [date]" labels prominent — this is a prototype over snapshotted open data.

## 8. Non-goals / don't-promise
Building-level LGBTQ+ safety score, real-time crime by address, authoritative rent-stabilization status (data incomplete). Reframe discrimination angle as "legal protections + community resources," which *are* data-backed.

## 9. Risks & mitigations (updated)

| Risk | Mitigation |
|---|---|
| Address→BBL fails | Curated set pre-resolves BBLs at ingest; GeoSearch (not GeoClient) for live |
| Rust backend inflates 10-day scope | Keep curated scope; teammates take frontend/design/data-gathering; reuse known Cargo/Axum patterns |
| Score feels arbitrary | Transparent weights, "show the math," user-adjustable |
| Rent-stab / RS data incomplete | Label "best-available," never authoritative |
| Data staleness | Prominent "Data from [date]"; batch-refresh via DuckDB ingest |
| 311 pre-2020 missing | Scope UI copy to "since 2020"; add historical dataset only if needed |

## 10. 10-day build plan (Rust-backend adjusted)

- **D1** Repo + Cargo/Vite scaffold; download CSVs; DuckDB ingest of curated set → SQLite/SpatiaLite.
- **D2** Scoring engine + `/building/{bbl}` returning real card JSON.
- **D3–4** Health Card UI, rent gauge, violation timeline (Tailwind, mobile-first).
- **D5** Address search + `/search`; loading/error states; source links.
- **D6** Rent-fairness input flow + `/rent-fairness` (Census + HUD FMR).
- **D7** Mapbox map + rent heatmap + restaurant grades.
- **D8** User testing (5 students + 2 community); fix.
- **D9** Case study + demo video.
- **D10** Deploy (backend Shuttle/Fly, frontend Vercel); rehearse pitch; phone test.

## 11. Success criteria
**Must:** address (from curated set) → Health Card with 0–100 color-coded score, rent gauge, violation timeline; works on mobile. **Nice:** quiz onboarding, map, building comparison. **Portfolio gold:** all + case study + demo video + live URL.

---

## Appendix A — Data-integrity ledger (do NOT re-cite the fabricated claims)

| Original proposal claim | Status | Correct version |
|---|---|---|
| 51.6% / 28.8% rent-burdened (RGB) | ✅ confirmed | keep as-is |
| $3,616 / +6.2% (metro) | 🟡 mislabeled | it's **citywide**; metro = $2,968 / +1.7% |
| 2.91M / 38.3% "NYC metro" | 🟡 mislabeled | that's **NY State, all households**; NYC-metro renters ≈ 52.4% |
| "1M+ HPD violations" | 🟡 understated | ~**11.1M** records |
| "761,352 buildings (REBNY 2026)" | 🔴 fabricated source | drop; cite **PLUTO** (~870k lots) |
| "11% buildings Class C (REBNY 2026)" | 🔴 fabricated | drop; ~450k active Class C violations |
| OpenIgloo "Index Ventures" | 🔴 false | Gutter Capital, MetroCap, Trend Forward |
| OpenIgloo "$1.37M rev / $4.4M val" | 🔴 unverifiable | drop both |
| OpenIgloo "100K users" | 🟡 stale | now 1.5–3M+ |
| FARE Act Jun 2025 / Good Cause Apr 2024 | ✅ confirmed | Jun 11 2025 / Apr 20 2024 |
| GeoClient for address→BBL | 🔴 broken | **GeoSearch** / Geosupport |
| 311 "30M+ / 2010+" | 🟡 stale | ~21.9M, **2020→present** |
| Property mgmt SW $3.6B | ✅ confirmed | Grand View, $3.61B (2025) |
| Tenant screening $2.3B | 🟡 wide variance | present as **range $1.5–6.8B** |

*Full source URLs retained in the fact-check report; port them into the PRD's citations.*

## Appendix B — Reuse map
- **SiteAssure** `DATA_POLICY.md` + PRD → adapt for §7 + Housecheck PRD.
- **Launch Radar** roadmap structure + Axum/SQLx Cargo patterns → PRD template + backend reference.
- **Nisaba-Brand-Kit / Ziqpu-Design** → frontend design tokens.
- All copied into this folder as templates; originals untouched.

## Appendix C — Resolved decisions & current status (2026-07-23, PRD approved)

These supersede earlier pre-decision mentions in the body (React/Tailwind, Mapbox, Shuttle, SpatiaLite).

- **Curated set:** 250 buildings = all residential buildings in Brooklyn CD3 (Bed-Stuy), capped at 250 (`--cd 303 --limit 250`); scales to the full ~2,000-building CD3 post-grade. Framed as a neighborhood slice, not a hand-picked list.
- **Rent-stabilization wording:** 3-state honest labels — "Likely rent-stabilized (signal, not a ruling)" / "No record found (lists are incomplete)" / "Unverified (ask for the DHCR rent history)" + footnote "a signal, not a verdict."
- **Backend hosting:** Fly.io — the read-only SQLite DB baked into the Docker image (no DB service/volume), scale-to-zero. Frontend on Vercel.
- **Frontend framework:** **Dioxus 0.6** (Rust→WASM over the Axum API), NOT React/Tailwind. Topcoat evaluated, deferred (too early; full-stack).
- **Serving DB:** plain **bundled SQLite** (not SpatiaLite); geospatial handled at ingest. DuckDB reserved for a *future full-NYC bulk-CSV/Parquet ingest*, never the serving DB.
- **Map:** **MapLibre GL + Protomaps pmtiles** (free, no key), NOT Mapbox. Geocoding via NYC GeoSearch (keyless).
- **Cost:** data APIs $0 · hosting < $10/mo — within the $20–50 budget. Secrets: `CENSUS_API_KEY` (required) + `NYC_APP_TOKEN` (free Socrata app token, recommended).
- **Build status:** backend LIVE with real Bed-Stuy data (PLUTO buildings, HPD violations, 311, DOB elevators, MTA ADA, Census medians). Rent-fairness + neighborhood axes are real. Repo: github.com/nessaisling-lab/housecheck (public, CI green). Plan 2 Tasks 5–6 done. **Live:** https://housecheck-nessa.fly.dev (Fly.io, deployed 2026-07-23, verified serving real data).
