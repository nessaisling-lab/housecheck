# HouseCheck — Delegated Build Task List

Owners: **A** = Aisling (Rust backend) · **B** = Frontend · **C** = Data/Design/QA.
Priority: P0 = must-ship · P1 = important · P2 = nice-to-have.
Status: ☐ todo · ◐ in-progress · ☑ done. Update in PRs.

## Workstream A — Rust backend (Aisling)

| ID | Task | Day | Pri | Depends | Done when |
|----|------|-----|-----|---------|-----------|
| BE-1 | Cargo workspace scaffold (`ingest`/`scoring`/`api`); `git init`; `.gitignore` incl `.env`,`data/` | D1 | P0 | — | `cargo build --workspace` passes |
| BE-2 | Wire `rusqlite` + **bundled SpatiaLite**; open/query DB from Rust | D1 | P0 | BE-1 | test query returns a row + a spatial op works |
| BE-3 | `ingest`: DuckDB ETL → curated Brooklyn tables → `data/housecheck.db` | D1 | P0 | DATA-1 | DB builds from real CSVs, reproducible |
| BE-4 | Spatial join building→census tract (`ST_Contains`) | D1 | P0 | BE-2,BE-3 | every building has a `tract_geoid` |
| BE-5 | `scoring` crate: weighted 0–100 + sub-scores; unit tests | D2 | P0 | BE-3 | fixture buildings score as hand-computed |
| BE-6 | `api`: `/health`, `/search?address=`, `/building/{bbl}` | D2 | P0 | BE-5 | 3 known BBLs return verified cards |
| BE-7 | tower middleware: CORS, rate-limit, tracing | D2 | P1 | BE-6 | limits + structured logs verified |
| BE-8 | `POST /rent-fairness` (tract median + HUD FMR) | D6 | P0 | BE-6 | correct % for a test rent |
| BE-9 | Accessibility fields in ingest + `/building` payload | D6 | P1 | ACC-1 | badge data returned with source |
| BE-10 | GeoSearch live fallback for addresses outside curated set | D6 | P2 | BE-6 | live Brooklyn address resolves to BBL |
| BE-11 | Author `ci.yml`/`security.yml`/`smoke.yml`; keep green | D1→D8 | P0 | BE-1 | pipeline green on all 3 OSes |

## Workstream B — Frontend

| ID | Task | Day | Pri | Depends | Done when |
|----|------|-----|-----|---------|-----------|
| FE-1 | Vite+React+TS+Tailwind scaffold; brand tokens; deploy shell to Vercel | D1 | P0 | — | shell live on Vercel |
| FE-2 | API client + types; env config for backend URL | D2 | P0 | FE-1,BE-6 | fetches `/building` in dev |
| FE-3 | Building Health Card: gauge, sections, legal badges | D3 | P0 | FE-2 | real card renders |
| FE-4 | Violation timeline component | D3 | P1 | FE-3 | A/B/C over time |
| FE-5 | "Show the math" score-breakdown panel | D4 | P1 | FE-3 | weights visible + adjustable |
| FE-6 | Address search + autocomplete over curated set | D5 | P0 | FE-2 | typed address → card |
| FE-7 | Loading / error / empty states; mobile-first 375px | D5 | P0 | FE-3 | no dead ends on phone |
| FE-8 | Rent-fairness input flow + result viz | D6 | P0 | BE-8 | % vs median shown |
| FE-9 | Accessibility badge/section UI | D6 | P1 | BE-9 | badge + source link |
| FE-10 | Mapbox map + rent heatmap + restaurant grades | D7 | P1 | FE-2 | map loads curated set |
| FE-11 | Dark-theme + anti-slop polish pass | D5→D8 | P1 | FE-3 | design-review clean |
| FE-12 | Quiz onboarding ("what's your priority?") | D6 | P2 | FE-3 | personalizes card weights |

## Workstream C — Data / Design / QA

| ID | Task | Day | Pri | Depends | Done when |
|----|------|-----|-----|---------|-----------|
| DATA-1 | Select curated Brooklyn buildings; pull HPD/311/DOHMH/PLUTO/ACS subsets; Census API key | D1 | P0 | — | clean CSVs in `data/raw/` + building list |
| DATA-2 | Verify every displayed stat has a real source (no fabricated claims) | D1→D9 | P0 | — | data-integrity ledger current |
| ACC-1 | Assemble accessibility sources (DOB elevators, MTA ADA, DOT ramps) for curated set | D6 | P1 | DATA-1 | accessibility table populated |
| DES-1 | Brand tokens (color/type/icons) from Nisaba/Ziqpu kits → Tailwind config | D1 | P0 | — | tokens in `frontend` |
| DES-2 | Health Card visual spec + gauge animation direction | D3 | P1 | DES-1 | B has a design to build to |
| QA-1 | Write smoke + stability test assertions | D8 | P0 | BE-6 | smoke.yml asserts real behavior |
| QA-2 | User testing: 5 students + 2 community; log issues | D8 | P0 | M2 | prioritized bug list |
| QA-3 | Case study (problem/solution/data/decisions) | D9 | P0 | — | draft complete |
| QA-4 | Demo video record + edit | D9 | P0 | M3 | 2–3 min demo |

## Critical path
`DATA-1 → BE-3 → BE-4 → BE-5 → BE-6 → FE-3 → FE-6 → (D8 harden) → deploy`. Anything off this path is parallelizable; protect the path.

## Definition of Done (every task)
Code compiles on Mac+Windows+Linux · tests pass · no secret committed (gitleaks) · any user-facing number links to its source · PR reviewed by one other owner.
