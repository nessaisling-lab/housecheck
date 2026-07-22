# HouseCheck — 10-Day Roadmap

Borough: **Brooklyn** · MVP: **curated ~50–200 buildings** · Stretch: one-borough live.
Team: **A** = Aisling (Rust backend lead) · **B** = Frontend · **C** = Data/Design/QA.
Each day ends at a **gate** — do not advance until it's met.

## Milestones

| M | Name | Days | Exit gate |
|---|------|------|-----------|
| M0 | Foundation | D1 | Repo + workspace scaffold; curated DB builds from real Brooklyn data |
| M1 | Core engine | D2 | `/building/{bbl}` returns a real, correct Health Card JSON |
| M2 | Product UI | D3–D5 | Address search → animated card on mobile, with source links |
| M3 | Depth features | D6–D7 | Rent-fairness flow + map/heatmap + accessibility indicator live |
| M4 | Harden | D8 | CI green on Mac+Windows+Linux; security + smoke + stability pass; user-tested |
| M5 | Ship | D9–D10 | Deployed URL, case study, demo video, rehearsed pitch |

## Day-by-day

**D1 — Foundation (A leads; B/C parallel)**
- A: `cargo new` workspace (`ingest`/`scoring`/`api`); `.gitignore` (with `.env`, `data/`); `rusqlite` + SpatiaLite wired; **git init + first commit**.
- C: pick + list the curated Brooklyn buildings; download HPD/311/DOHMH/PLUTO/ACS subsets; register Census API key.
- B: `frontend/` Vite+React+TS+Tailwind scaffold; import brand tokens; deploy empty shell to Vercel.
- Gate: `cargo run -p ingest` builds `data/housecheck.db` from real data.

**D2 — Core engine (A)**
- `scoring` crate: weighted 0–100 + sub-scores, unit-tested with fixtures.
- `api`: `/health`, `/search`, `/building/{bbl}` reading precomputed card.
- Gate: three known Brooklyn BBLs return correct, hand-verified cards.

**D3–D4 — Health Card UI (B; A supports)**
- Card component, animated gauge, violation timeline, legal badges, "show the math" panel.
- Wire to API; loading/error/empty states; **mobile-first** at 375px.
- Gate: real card renders on a phone from a typed address.

**D5 — Search + polish (B)**
- Address autocomplete over curated set; per-number source links; dark theme pass.
- Gate: full search→card flow with no dead ends.

**D6 — Rent fairness + accessibility (A + B)**
- `POST /rent-fairness` (tract median B25064 + HUD FMR); "enter your rent" UI.
- Accessibility indicator surfaced (data-backed — see PRD §Appendix).
- Gate: user rent → correct % vs median; accessibility badge shows with source.

**D7 — Map (B; A data)**
- Mapbox map, rent heatmap, nearby restaurant grades.
- Gate: map loads curated buildings; heatmap reads correctly.

**D8 — Harden (all)**
- CI matrix green (Mac/Windows/Linux); `security.yml` + `smoke.yml` + stability pass.
- User testing: 5 students + 2 community; log + fix top issues.
- Gate: green pipeline + zero P0 bugs.

**D9 — Story (C lead)**
- Case study (problem/solution/data/decisions); record demo video.
- Gate: video + case study drafted.

**D10 — Ship (all)**
- Deploy backend (Shuttle/Fly) + frontend (Vercel); phone test; rehearse 5-min pitch.
- Gate: live URL works end-to-end on a phone.

## Cross-cutting tracks (run continuously)
- **Data integrity:** every displayed number links to source; no fabricated stats (see spec Appendix A).
- **CI from D1:** workflows exist D1, must stay green as code lands.
- **Design:** brand tokens D1; anti-slop polish through M2–M3.

## Risk burndown (top 3)
1. **SpatiaLite on Windows** — use `rusqlite` bundled + bundled SpatiaLite, verify in CI D1, not D8.
2. **Scope creep to live-NYC** — locked stretch-only; curated set is the demo.
3. **Score credibility** — transparent weights + "show the math" shipped in M2, not bolted on late.

## Stretch (only after M5 green)
One-borough live ingest of all Brooklyn; then multi-borough. Requires GeoSearch live path + larger DuckDB ingest.
