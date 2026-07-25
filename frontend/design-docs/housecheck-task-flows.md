# HouseCheck — Task Flows
## v1 · to be validated BEFORE wireframes

*Notation: [SCREEN] = a screen · (decision) = branch · → = user action · ⚡ = API call. Every flow lists its edge states — these become required Figma frames.*

---

## Global frame (applies to all flows)
- Persistent floating bottom chrome = **TWO detached elements** (Whoop pattern, confirmed from reference screenshots): ① glass capsule tab bar: **Search · Saved · Compare · More** (v2 decision: Map tab cut — 250 buildings in one neighborhood = thin value, high build cost; Community deferred to post-MVP — needs accounts/moderation; Saved/Recent is localStorage-only and feeds Compare) and ② a **separate circular agent orb** floating beside it — the AI agent is NOT a tab in the capsule. Orb is the only tinted element.
- Chrome is visible on all top-level screens; hides on scroll down, returns on scroll up (iOS convention).
- Agent orb opens the agent sheet from ANY screen, carrying current building context when available.
- **Sticky collapse:** on the Health Card, scrolling past the hero pins a slim glass strip of 4 mini sub-score rings to the top (Whoop's Sleep/Recovery/Strain collapse pattern); tapping a mini-ring jumps to that section.

---

## Flow 1 — Search → Building Health Card (the money flow)
```
[Home/Search]
  hero search field: "Enter a Brooklyn address"
  → types address, autocomplete suggestions ⚡GET /search?address=
  (in_curated_set = true?)  ── yes ──→ [Loading skeleton] ⚡GET /building/{bbl}
       │                                        │
       no                                       ▼
       ▼                              [BUILDING HEALTH CARD]
  [Out-of-coverage sheet]               hero score → sub-scores → sections
  "We're covering ~250 Bed-Stuy           → tap sub-score row → anchor-scroll
   buildings for the MVP"                  to its section
  → "Explore covered buildings"           → tap source line → external gov source
    jumps to [Map/List]                   → "Compare" → adds to compare tray
```
**Edge states:** (a) no autocomplete match → inline "Address not found — try street + house number" (not an error page); (b) loading skeleton = gray pulsing ring + ghost rows, min 400ms to avoid flash; (c) API failure → retry card with honest copy.

**Screens/frames needed:** Home, Home-with-suggestions, Loading skeleton, Health Card, Out-of-coverage sheet.

---

## Flow 2 — Health Card → Section detail → Source (the trust flow)
```
[HEALTH CARD] → scroll to section (e.g. Building condition)
  section card shows 2–4 key values + status chip + 1 plain sentence
  → tap "Source: NYC HPD · Jul 2026 ↗" → external source (new tab)
  → tap section → [SECTION DETAIL sheet] (full data: all violation classes,
    history note, hedged interpretation, methodology link)
  → "How we calculate this" → [Methodology page]
```
**Edge states:** unverified section (stabilization = "unverified") → gray chip + why-unverified explainer inside detail sheet; zero violations → positive empty state, still cites source.

**Frames:** Section detail sheet (×4 variants: rent, condition, legal, accessibility), Methodology.

---

## Flow 3 — Rent fairness check (the interactive flow)
```
Entry A: [HEALTH CARD] → rent fairness section → "Paying rent? Check →"
Entry B: nav → [Rent fairness] (after a building was searched, pre-filled tract)
  → input: monthly rent (numeric keypad, $ prefix)
  ⚡POST /rent-fairness { user_rent, tract context }
  → result INSIDE the same card (no navigation):
    verdict chip + "X% vs tract median" + marker animates onto spectrum track
    + HUD FMR reference line + source line
  → "What counts as fair?" → methodology anchor
```
**Edge states:** rent = 0/empty → disabled CTA; absurd value (>2× FMR+) → results still shown but with "double-check this number" note; no tract data → unverified state.

**Frames:** Rent section (empty / filled / result / unverified).

---

## Flow 4 — Map / List explore (the coverage flow)
```
[Map/List]  ⚡GET /buildings (~250)
  map: dots colored by score band; list: address + score chip rows
  → toggle map/list (segmented control in glass bar)
  → tap dot/row → [BUILDING MINI-CARD] (bottom sheet, collapsed):
    address + hero score ring (small) + top 2 stats
    → "View full card" → [HEALTH CARD]
    → "+ Compare" → adds to tray, tray pill appears above nav
```
**Edge states:** dot density → cluster with count; list sort (score asc/desc, A–Z); empty compare tray hint.

**Frames:** Map view, List view, Mini-card sheet, Compare tray pill.

---

## Flow 5 — Compare (the decision flow)
```
[Compare tray] holds 2–4 buildings (from Flow 1 or 4)
  → tray pill → [COMPARE screen] ⚡GET /compare?bbls=a,b,c
  columns: one per building; rows: total score, 4 sub-scores,
    key facts (violations C, stabilization, elevator, ADA distance)
  best-in-row value subtly highlighted in band color
  → tap column header → full [HEALTH CARD]
  → swipe to remove a building → slot empties (max 4 enforced at add-time)
```
**Edge states:** <2 buildings → empty state with "add from search or map"; 4-building max → toast "Compare holds 4 — remove one first."

**Frames:** Compare (2-up, 3-up, 4-up), empty state, max-toast.

---

## Flow 6 — AI agent (the assistant flow)
```
[Any screen] → agent orb (separate from nav capsule, tinted glass)
  → [AI AGENT sheet] rises; if on a Health Card, context attached
    ("Ask about 548 Gates Ave…") ⚡POST /summary or agent endpoint
  → suggested prompts as glass chips ("Is this building rent stabilized?",
    "Explain the score", "Compare to neighborhood")
  → answers render in card style with the same source-line component
  → "View sources" → section detail / methodology
```
**Edge states:** no building context → general mode ("Search a building first for specific answers"); loading = typing indicator in glass bubble; error → honest "The agent couldn't answer — here's the raw data" fallback linking to the card.

**Frames:** Agent sheet (with/without context), conversation state, loading, fallback.

---

## Screen inventory (for Figma page setup)
1. Home/Search (+suggestions, +not-found)
2. Loading skeleton
3. Building Health Card (hero + 4 sub-scores + 4 section cards) ← *design this one FIRST*
4. Section detail sheets ×4
5. Methodology
6. Out-of-coverage sheet
7. Map view / List view / Mini-card
8. Compare (2/3/4-up + empty)
9. AI agent sheet
10. States library: unverified, error/retry, toasts, skeletons

## Open questions to resolve before/during wireframes
1. 4th nav tab: Saved buildings? About/coverage? (Recommend: **About/Coverage** for MVP — Saved needs accounts.)
2. Does rent fairness live ONLY inside the Health Card, or also standalone in nav? (Recommend: inside card only — one less screen.)
3. Compare rows: fixed set (~8 rows) or user-toggleable? (Recommend fixed for MVP.)
