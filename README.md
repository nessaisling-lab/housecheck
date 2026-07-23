# HouseCheck 🏙️

> **Carfax for apartments.** Type any NYC address, get an instant Building Health Card — condition, legal protections, rent fairness, and accessibility — with every number linked to a government record.

**Live API:** https://housecheck-nessa.fly.dev · **Case study:** *(portfolio link)* · Pursuit NYC Fellowship, L2 Cycle 4

---

## Why

Renting in NYC means committing ~$40,000 a year to a building you know almost nothing about. The facts exist in government databases but are scattered across three unusable portals. HouseCheck combines them into one honest 0–100 score — objective public data only, every number sourced.

**The product working, in one line:** two real Bed-Stuy buildings a few blocks apart score **24 vs 78** — one a 65-violation walk-up, the other spotless. That spread comes straight from public HPD records.

## What it scores

| Axis | Source |
|---|---|
| Building condition (violations A/B/C, severity × recency) | NYC HPD |
| Legal protections (rent-stabilization, Good Cause) | JustFix/DOF · NY HCR |
| Rent fairness (your rent vs tract median + HUD FMR) | Census B25064 · HUD |
| Accessibility (elevator-on-record + build-era) | NYC DOB · MTA |
| Neighborhood (311 density, restaurant grades) | NYC 311 · DOHMH |

## Quickstart

```bash
git clone https://github.com/nessaisling-lab/housecheck.git
cd housecheck

# run against the built-in fixture data (no keys, no network)
cargo run -p ingest -- --fixture --out data/housecheck.db
HOUSECHECK_DB=data/housecheck.db cargo run -p api
curl http://127.0.0.1:8787/building/3000010001
```

Real data (needs a free [Census key](https://api.census.gov/data/key_signup.html) + optional NYC app token in env):
```bash
cargo run -p ingest -- --real --cd 303 --limit 250 --out data/housecheck.db
```

## API

`GET /health` · `GET /building/{bbl}` · `GET /buildings` · `POST /rent-fairness` · `GET /search?address=` · `GET /compare?bbls=` · `POST /summary`
Full contract with request/response examples: **[docs/API.md](docs/API.md)**.

## Stack

Rust · Axum · bundled SQLite (read-only, baked into the Docker image → the deployed API needs **zero secrets**) · `reqwest` ingest over free NYC Open Data + Census APIs. Deployed on Fly.io (scale-to-zero). Frontend: Dioxus (Rust→WASM) — in progress.

- **Deploy:** [docs/DEPLOY.md](docs/DEPLOY.md) · **Design spec:** [docs/superpowers/specs/](docs/superpowers/specs) · **PRD:** [HouseCheck_PRD.docx](HouseCheck_PRD.docx)
- **CI:** build + test on macOS/Windows/Linux, security scan, smoke + stability — green.

## Team & branches

| Person | Branch | Area |
|---|---|---|
| Aisling Leiva-Davila | `aisling-backend` | Backend + data (lead) |
| Anthony Lesov | `anthony-frontend` | Dioxus frontend |
| Jagger | `jagger-agent` | Agent |
| — | `db-analyst` | Data |

`main` is the **shared team branch** — everyone's work (including Anthony's frontend) merges here; `main` isn't complete until the frontend lands. `post-capstone` is **Aisling's personal branch** for evolving the backend past the capstone — the rest of the team doesn't work in it.

## Data & honesty

Every displayed number links to its source. Where public data can't support a claim (e.g. definitive rent-stabilization), the card says so — *"a signal, not a legal ruling."* Rent-stabilization data is derived from public NYC DOF tax records via [JustFix](https://github.com/JustFixNYC); no data is fabricated. *Research/educational capstone — not legal or financial advice.*
