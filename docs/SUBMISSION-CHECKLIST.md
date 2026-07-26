# HouseCheck — Capstone Submission Checklist

Maps every Day-29 / Day-30 deliverable to the artifact you already have + the action left. Most are **done** — what remains is mostly *submitting* through Pursuit's forms/Pathfinder.

## Day 29 — Discover the problem

| Deliverable | Have it? | Artifact | Action |
|---|---|---|---|
| **Industry research notes** | ✅ | PRD §1 + design spec (players: OpenIgloo/StreetSmart/JustFix; gap; who's served) | Copy into the Google Doc if a standalone one is required |
| **Market framing** | ✅ | PRD §1a Opportunity + verified market stats | — |
| **Problem statement (with numbers)** | ✅ | PRD §1 (51.6% burdened, $1,761 gap, 11.1M violations — all sourced) | ☐ **Paste into Pathfinder** |

## Day 30 — Design + build

| Deliverable | Have it? | Artifact | Action |
|---|---|---|---|
| **Solution Design Sprint → MVP scope** | ✅ | PRD §2c Goals/Non-Goals + spec (3 options considered, 1 chosen) | — |
| **Capstone PRD (submitted)** | ✅ **APPROVED** | `HouseCheck_PRD.docx` (all teacher notes resolved in Appendix E/F) | ☐ **Submit as the Google Doc** (upload/convert the .docx) |
| **Repo + first P0 feature running end-to-end** | ✅ | Public repo + **live**: address → scored Building Health Card | ☐ **Submit the GitHub repo link** |

## Portfolio gold ("what success looks like")

| Item | Status |
|---|---|
| Type an address → Building Health Card, 0–100 color-coded | ✅ live |
| Rent-fairness · violations + timeline · works on real data | ✅ live |
| **Live deployed app** (show this one) | ✅ https://housecheck-wine.vercel.app |
| **Live API** | ✅ https://housecheck-nessa.fly.dev |
| **Case study** | ✅ `docs/CASE-STUDY.md` + polished portfolio page |
| **Demo video** | ☐ record — script in `docs/DEMO-SCRIPT.md` |
| 5-min pitch rehearsed | ☐ outline in `docs/PITCH.md` |

## What's genuinely left for you (Aisling)

1. ☐ Add your **reflection** to the case study (3–4 sentences → I'll republish).
2. ☐ **Submit**: problem statement (Pathfinder) · PRD (Google Doc) · repo link — per Pursuit's forms.
3. ☐ **Record the demo** (script ready) and **rehearse the pitch** (outline ready).

## Engineering status (2026-07-26)

Nothing outstanding. For the record, so nobody re-checks:

| | |
|---|---|
| CI | ✅ `ci` · `security` · `smoke` all green, three OSes, 15/15 jobs |
| Security | ✅ 0 npm vulnerabilities · gitleaks, cargo-audit, cargo-deny, CodeQL all pass |
| Frontend | ✅ deployed, deep links work, react-router 8 |
| Backend | ✅ live, real data, CORS locked to the Vercel origin |
| Deploys | manual and documented — see `docs/DEPLOY.md`. There is deliberately no CD; the reason is written up there rather than left as a gap. |
| Anthony | ✅ accepted and actively contributing (the whole React frontend is his) |
| Fly billing | ✅ card added; app runs scale-to-zero |

**`/summary` (the LLM agent) is intentionally disabled** — it returns `501` until `OPENROUTER_API_KEY` is set, and the UI degrades honestly to grounded canned answers. It stays off until a paid zero-data-retention model is configured, because prompts contain a building address and the user's rent. Not required for submission. Full plan in `docs/agent/PRD-AGENT.md`.

## Not yours

Frontend (React), map layer, theme pick → **Anthony**. Agent feature → tracked in `docs/agent/`. Demo/pitch delivery → **team**.
