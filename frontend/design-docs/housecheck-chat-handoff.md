# HouseCheck — Project Handoff (paste this into the new chat to continue)

## Who I am & the project
I'm the frontend designer/developer on a 3-person student team (Pursuit fellowship, NYC) building **HouseCheck** in ~10 days: "Carfax for NYC apartments." A renter types a Brooklyn address and gets a **Building Health Card** — a 0–100 score from free public NYC data (HPD violations, Census rent, DOB/MTA accessibility, 311 complaints). Every number links to its public source. Tone: honest, non-overclaiming ("unverified," "a signal, not a legal ruling"). MVP coverage: ~250 buildings in Bed-Stuy, Brooklyn. I'm a strong designer, near-beginner coder, React or HTML/CSS/JS.

**Backend (live, built by teammate, Rust/Axum REST API):**
- `GET /search?address=` → { bbl, label, in_curated_set }
- `GET /building/{bbl}` → full Health Card: building facts (address, year_built, floors, units_res, has_elevator, near_ada_subway_m, complaints_311, lat/long), score total + 4 sub-scores (condition, legal, neighborhood, accessibility), open_violations (Class A/B/C, C = hazardous), access_likelihood (Higher/Mixed/Lower), stabilization (likely/none_on_record/unverified + hedged message), good_cause (bool)
- `GET /buildings` → all ~250 with { bbl, address, lat, long, score }
- `POST /rent-fairness` → { user_rent, tract_median, pct_vs_median, verdict, hud_fmr }
- `GET /compare?bbls=a,b,c` → up to 4 cards
- `POST /summary` → AI plain-language summary
Third teammate builds the AI agent feature.

## Design decisions locked in (in order)
1. **Visual style: Whoop app structure × Apple Liquid Glass, LIGHT-first.** Governing rule (Apple HIG): glass only for floating chrome (nav, sheets, overlays, agent orb); scores/data on near-solid light surfaces — never body text on glass, never glass-on-glass. Figma has a native Glass Effect (built with Apple) + official iOS 26 Figma kit — use those, not glassmorphism hacks.
2. **Reference screens:** I uploaded 9 Whoop screenshots; key patterns adopted: ① bottom chrome = TWO detached floating elements — capsule tab bar + SEPARATE circular agent orb (agent is NOT a tab); ② sticky collapsed header (sub-score mini-rings pin to top on scroll); ③ thin ring gauges, big tabular numbers, ALL-CAPS tracked labels × large sentence-case section headers; ④ "Pace of Aging" tick spectrum → our rent-fairness track; ⑤ status pills ("21% above median ▲"); ⑥ agent sheet anatomy (logo pill, history/tips icons, "＋" + input capsule + mic).
3. **Nav capsule (final): Search · Saved · Compare · More + agent orb.** Map tab CUT (250 buildings in one neighborhood = thin value, expensive build). Community DEFERRED post-MVP (needs accounts/moderation, legal exposure, cold-start). Saved/Recent = localStorage, feeds Compare.
4. **Score bands (light-mode accessible):** 80–100 #248A3D "Strong record" · 60–79 #7DA629 "Generally solid" · 40–59 #B7791F "Mixed signals" · 20–39 #D04A1E "Real concerns" · 0–19 #C7272B "Serious red flags" · unverified #8E8E93 (designed state, never an error). Canvas #F6F6F8 (never pure white — glass needs something to refract); top 40% of Health Card gets 8% gradient wash of the band color.
5. **Type:** SF Pro/Inter stack, all numerals tabular-nums. Hero 72px Semibold, sub-score 28px, section headers 28px Regular sentence-case, eyebrows 13px Semibold ALL-CAPS +10% tracking, body 17px, source lines 12px.
6. **Health Card layout (3 tiers):** Tier 1 hero ring + band label + one-line disclaimer → Tier 2 four equal sub-score rows (mini-ring + name + one-line status + number + chevron) → Tier 3 four identical section widgets (icon chip + title + status pill / 2–4 label:value rows / one plain sentence / divider / source line with agency + date + link). Widgets: Rent fairness (tick spectrum, Census/HUD), Building condition (A/B/C violations, HPD/311), Legal protections (stabilization + Good Cause, DHCR), Accessibility (elevator, ADA subway, DOB/MTA). Depth budget: max 3 taps score→source. Equal sub-score weights (backend averages the four).

## Deliverables produced so far (files in /mnt/agents/output/)
- `housecheck-health-card-design.md` — 24-reference annotated design library (US: Walk Score, Yuka, Oura, Whoop, Credit Karma, NerdWallet; Swiss/Nordic: SBB, Swiss Confederation DS, MitID, Helsenorge, Lunar, HDS Helsinki, Entur, Klarna, 1177; German/Dutch: N26, Trade Republic, DB, NS, NL Design System, bunq; Japan: PayPay, Yahoo Transit) + 7 shared design principles
- `housecheck-design-strategy.md` — v1.1 full token system (colors, type, glass CSS recipes w/ fallbacks, radii, spacing) + component specs + "Whoop screen autopsy" (8 measured patterns)
- `housecheck-task-flows.md` — 6 flows w/ edge states + screen inventory (nav updated to Search·Saved·Compare·More)
- `housecheck-figma-make-prompt.md` — condensed prompt-optimized spec (Figma Make attempt FAILED — abandoned)
- `wireframes/` — 11 precise grayscale wireframe PNGs built programmatically (real text, 8px grid): 01 splash, 02 home, 03 health card full scroll (real data: 460 Macon St, score 57 Fair), 04 scrolled w/ sticky strip, 05 section detail sheet, 06 saved, 07 compare 3-up, 08 agent sheet, 09 out-of-coverage, 10 loading skeleton, 11 not-found — plus `housecheck-wireframes.zip`

## Current state / where we are
Wireframes DONE. Next step: I recreate them hi-fi in Figma myself using the strategy tokens + Apple's iOS 26 kit, starting with the Building Health Card (frame 03 is the full-scroll master). After each screen I can send it back for design review against the spec. Then: React build mapping tokens → CSS variables, components: HeroRing, SubScoreRow, SectionCard, SourceLine, StatusPill, SpectrumTrack, NavCapsule, AgentOrb, StickyStrip.

## Open questions parked
- Methodology page content (weights explainer: "each pillar counts equally")
- Whether rent fairness lives only inside the Health Card (current assumption: yes)
- Agent orb behavior when API/agent is unavailable

## How to continue
When I paste this, confirm you've absorbed the context, then ask me which screen I'm designing first in Figma or whether I want to start the React component architecture.
