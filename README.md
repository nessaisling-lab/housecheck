# HouseCheck 🏙️

> **Carfax for apartments.** Type any NYC address, get an instant Building Health Card — condition, legal protections, rent fairness, and accessibility — with every number linked to a government record.

**Live app:** https://housecheck-wine.vercel.app · **Live API:** https://housecheck-nessa.fly.dev
**Case study:** [pursuit-l2-portfolio / cycle-4](https://github.com/nessaisling-lab/pursuit-l2-portfolio/tree/main/cycle-4-housecheck) · Pursuit NYC Fellowship, L2 Cycle 4

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

## The export — a record that survives leaving us

A tenant lawyer can read a building's violations on any city website. What they cannot do
is put that reading in front of a court, because a printout is unverifiable.

Every card exports as a document carrying an **append-only hash chain** —
`entry_hash[i] = sha256(entry_hash[i-1] ++ payload_hash[i])` — with the chain head signed
**Ed25519**. Change one character of one violation and every hash from that row onward
changes. Each source's dataset id and retrieval timestamp travel *inside* the signed
region, so the document attests to a fact rather than merely to itself.

**Three verification outcomes, not two:** signed-and-intact, intact-but-unsigned, tampered.
Collapsing the middle one would let an unsigned document pass as an authenticated one.

**Verify it yourself, without trusting us.** The public key is published separately from
the documents it signs:

```bash
curl -s https://housecheck-nessa.fly.dev/meta | grep -o '"export_public_key":"[^"]*"'
```

Export a record from the live app and check that the key embedded in it matches. If it
does not, the document did not come from this system however intact its own chain looks.

That check exists because signing alone was not enough. A forger who rewrites a row,
recomputes the whole chain and signs it with their own keypair produces a document that
verifies as intact — every check inside it passes, because it is internally consistent. A
row rewritten to *"NO VIOLATIONS OF ANY KIND AT THIS ADDRESS"* passed cleanly. Publishing
the key is what rejects it.

**Repair speed** is on the card for the same reason: median days from a violation being
issued to being closed — the only measure describing landlord *behaviour* rather than
state. Three states again (a median / "nothing closed since 2023" / no data), because one
pilot building has 33 open violations and closed exactly one, in October 2017, and under
two states it rendered blank — making the landlord who fixes nothing look emptier than one
who fixes things slowly.

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

## Configuration and secrets

**There is deliberately no `.env` in this repo, and its absence is the design rather than
an oversight.** Fly's secret store is invisible by inspection, so it is written down here.

- **Runtime secrets live in Fly.io's encrypted secret store**, set with
  `flyctl secrets set NAME=value -a housecheck-nessa` and injected as environment variables
  at runtime. They are never written to disk in this repository. Full commands in
  **[docs/DEPLOY.md](docs/DEPLOY.md)**.
- `flyctl secrets list` returns **names and digests only, never values** — which is how the
  deployed configuration is audited without exposing it.
- **The core API needs no secrets at all.** The SQLite artifact is read-only and baked into
  the Docker image, so there is no database URL, no connection string and no password to
  leak. Secrets are only required by optional features.
- `.env` is gitignored and none is tracked. No key material appears in source.

| Variable | Required | Purpose |
|---|---|---|
| `HOUSECHECK_DB` | no (defaults to the baked artifact) | path to the SQLite artifact |
| `HOST` · `PORT` | no | bind address; Fly sets these |
| `CORS_ALLOWED_ORIGIN` | production | pinned to the deployed frontend origin |
| `HOUSECHECK_EXPORT_SIGNING_KEY` | for signed exports | Ed25519 secret key; its public half is served at `/meta` |
| `OPENROUTER_API_KEY` · `OPENROUTER_MODEL` · `OPENROUTER_SEARCH_MODEL` | for `/summary` | the optional grounded assistant |
| `CENSUS_API_KEY` · `NYC_APP_TOKEN` | ingest only | free keys; raise rate limits on public APIs |
| `SNAPSHOT_YEAR` | ingest only | pins the ACS vintage |

Ingest-only keys are never needed to *run* the API — the data is already in the artifact.

## Stack

Rust · Axum · bundled SQLite (read-only, baked into the Docker image → the deployed API needs **zero secrets**) · `reqwest` ingest over free NYC Open Data + Census APIs. Deployed on Fly.io (scale-to-zero). Frontend: React + Vite + Tailwind + shadcn/ui (mobile-first), wired to the live API.

- **Deploy:** [docs/DEPLOY.md](docs/DEPLOY.md) · **Design spec:** [docs/superpowers/specs/](docs/superpowers/specs) · **PRD:** [HouseCheck_PRD.docx](HouseCheck_PRD.docx)
- **CI:** build + test on macOS/Windows/Linux, security scan, smoke + stability — green.

## Team & branches

| Person | Branch | Area |
|---|---|---|
| Aisling Leiva-Davila | `aisling-backend` | Backend + data (lead) |
| Anthony Lesov | `anthony-frontend` | React frontend |
| Jagger | `jagger-agent` | Agent |
| — | `db-analyst` | Data |

`main` is the **shared team branch** — everyone's work merges here. The frontend has landed
and is deployed at the live app link above. `post-capstone` is **Aisling's personal branch**
for evolving the backend past the capstone; the rest of the team doesn't work in it.

## Data & honesty

Every displayed number links to its source. Where public data can't support a claim (e.g. definitive rent-stabilization), the card says so — *"a signal, not a legal ruling."* Rent-stabilization data is derived from public NYC DOF tax records via [JustFix](https://github.com/JustFixNYC); no data is fabricated. *Research/educational capstone — not legal or financial advice.*
