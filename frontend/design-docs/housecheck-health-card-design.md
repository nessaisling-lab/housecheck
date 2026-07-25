# HouseCheck — Building Health Card
## Design research + layout specification

*Prepared for the frontend designer/developer. Covers: (1) a mobile-first layout hierarchy for the Building Health Card screen, (2) an annotated reference library of products that solve the "score + breakdown + sources" pattern — US analogues plus Swiss, Nordic, German, Dutch, and Japanese examples — and (3) the design principles they share.*

---

# Part 1 — The Building Health Card layout spec

## The core idea (steal this framing)

Every great score product uses the same three-tier structure:

> **Tier 1 — The verdict.** One big number + a color band + a 2–4-word plain-language label. Answers "should I care?" in under a second.
> **Tier 2 — The breakdown.** The four sub-scores as equal, scannable rows. Answers "why?"
> **Tier 3 — The evidence.** Detail sections, each a self-contained chunk with its data and its source. Answers "says who?"

Your job is to keep these tiers visually *unambiguous* — the user should always know which tier they're looking at. Most cluttered data apps fail because tier-3 data leaks up into tier 1.

## Screen structure, top to bottom (mobile-first)

### 0. Header strip (compact)
- Building address (one line, bold) + borough/neighborhood ("Bed-Stuy, Brooklyn").
- Optional: tiny "last updated" or coverage badge. Nothing else. No score here.

### 1. HERO — The Building Health Score (Tier 1)
The single most dominant element on the screen. Nothing else competes with it.

- **One huge number** (0–100) — oversized tabular numeral, ~4–6× body text size. (Klarna/Oura: the number IS the screen.)
- **Color band, not decoration**: the number sits inside a large circular badge or on a full-width color wash mapped to fixed bands, e.g.:
  - 80–100 green "Strong record"
  - 60–79 yellow-green "Generally solid"
  - 40–59 amber "Mixed signals"
  - 20–39 orange "Real concerns"
  - 0–19 red "Serious red flags"
- **Plain-language label under the number** — never the number alone (Walk Score: "92 — Walker's Paradise"). HouseCheck's honest tone means labels like "Mixed signals — a signal, not a legal ruling."
- **One-line disclaimer under the hero**, muted, small: "Built from public NYC data. A screening signal, not a legal ruling." This is your honesty contract — put it where everyone sees it once, not on every section.
- Optional, if easy: tiny sparkline or "vs. Bed-Stuy median" chip for context.

### 2. SUB-SCORE GRID — the four pillars (Tier 2)
Four equal rows or a 2×2 grid of cards, directly under the hero. Each shows:
- **Name** (Condition / Legal / Neighborhood / Accessibility)
- **Its score** (medium-size number, same color grammar as hero)
- **One-line status** ("12 open violations" / "Good Cause eviction applies")
- **Chevron →** taps through to the matching detail section below (anchor scroll) or its own screen.

Rules:
- All four get *equal visual weight* — no pillar is "more hero" than another (bunq V3's cautionary tale: never hide one pillar behind a menu).
- Strict grid alignment — this is the Swiss/Yahoo-Transit lesson: multiple data points in a row are fine *if the grid is rigid*.
- Neutral ink by default; color appears only where it means something (DB Navigator grammar: black = normal, red = exception).

### 3. DETAIL SECTIONS (Tier 3) — four chunks, one template
This is where scannability is won or lost. **Every section uses the identical card template** (the MitID lesson: a rigid repeatable frame is itself a trust signal):

```
┌─────────────────────────────────────┐
│ SECTION TITLE          status chip  │
│ 2–4 key data points (not all of them)│
│ "What this means" — 1 plain sentence │
│ ─────────────────────────────────── │
│ Source: NYC HPD · updated Jul 2026 → │
└─────────────────────────────────────┘
```

The **source line is a designed component**, identical on every section: agency name + freshness + link icon (the Swiss federal design system makes institutional attribution a mandatory, standardized component; NL Design System does the same for Dutch government). Not a footnote — a component.

**Section A — Rent fairness**
- Contents: tract median rent, % vs. median (from POST /rent-fairness), verdict chip ("12% above tract median"), HUD FMR reference.
- Visual: a simple horizontal position marker on a low→median→high track — Yuka's "visual spectrum" trick beats a bare number.
- Interaction: rent input field lives here (or as an inline prompt: "Paying $2,400? Check →"), so the section is both display and tool.
- Source: US Census ACS / HUD.

**Section B — Building condition (violations)**
- Contents: open Class A / B / C counts, with **C (hazardous) visually foregrounded** — a "3 hazardous open violations" line in red ink matters more than the A-count. (DB Navigator: exceptions get color, the rest stays neutral.)
- Supporting: complaints_311 count, restaurant-grade-style context only if meaningful.
- One sentence of interpretation in your honest voice: "Open violations are a snapshot, not a history."
- Source: NYC HPD, NYC 311.

**Section C — Legal protections**
- Contents: stabilization status ("Likely rent stabilized" / "None on record" / **"Unverified"** — with the hedged message displayed, never hidden), Good Cause eviction true/false.
- Visual: this section should look *different in kind* — it's about rights, not conditions. A simple "protected / unverified / none on record" status chip trio works.
- The "unverified" state is a first-class visual state, styled deliberately (gray, with a short explanation of why it can't be verified). This is your brand's honesty made visible.
- Source: NYS DHCR / NYC data.

**Section D — Accessibility**
- Contents: access_likelihood ("Higher / Mixed / Lower") as a labeled chip, has_elevator, distance to nearest ADA subway (near_ada_subway_m), building facts (year built, floors, units) can live here or in a small "About this building" strip.
- Source: NYC DOB / MTA.

### 4. Secondary destinations (bottom of screen)
- "Compare this building" → adds to comparison tray.
- "See plain-language summary" → the AI agent feature (POST /summary) — also reachable from the floating bottom nav.
- Collapsed "How we calculate this score" → methodology page (weights, data sources, caveats). Yuka and Walk Score both publish this; it's your credibility engine.

## Visual priority summary

| Rank | Element | Treatment |
|---|---|---|
| 1 | Hero score + label + color | Largest type on screen; only saturated color block |
| 2 | Four sub-scores | Equal rows, medium numbers, strict grid |
| 3 | Status chips & exceptions (Class C, "unverified") | Semantic color, small |
| 4 | Detail data points | Neutral ink, label–value pairs |
| 5 | Sources | Muted, identical component on every section |
| 6 | Methodology / secondary actions | Collapsed, bottom |

Depth budget: **score → sub-score → evidence item → source = max 3 taps** (Yuka/Whoop both stay within this).

---

# Part 2 — Reference library

## A. US analogues (baseline patterns)

### 1. Walk Score — US (Seattle) ★ closest analog
The single most relevant reference: it literally scores an *address* 0–100 and is embedded on NYC apartment listings.
- **Steal:** Hero circular badge with number + color band + 2–4-word label ("Walker's Paradise — daily errands do not require a car"), then all detail disclosed *by scroll* on one long page: map → amenities by category → transit lines → neighborhood rank. Published score-band table and a methodology page naming every data source (Google, OpenStreetMap, US Census…).
- **Flow:** One big address search field → score page in 1 step → scroll = all detail, zero extra taps.
- **See it:** Live address page: https://www.walkscore.com/score/350-5th-ave-new-york-ny · Methodology: https://www.walkscore.com/methodology.shtm · Band tables: https://www.walkscore.com/how-it-works/

### 2. Yuka — France (US-popular)
Barcode scanner rating food 0–100; the gold standard for source transparency in a consumer score.
- **Steal:** Positives/Negatives summary lists with colored dots so users get the gist without reading; expandable sections per weighted dimension (weights published: nutrition 60% / additives 30% / organic 10%); tapping a nutrient opens a *visual scale* showing where the product sits on a green→red spectrum; each additive links to actual scientific sources; a disclosed rule (high-risk additive caps the score at 49) shows honesty builds trust.
- **Flow:** Scan → product card immediately → tap any section for detail → source page. 2–3 taps to deepest layer.
- **See it:** Methodology: https://help.yuka.io/l/en/article/ijzgfvi1jq-how-are-food-products-scored · Teardown with screenshots: https://screensdesign.com/showcase/yuka-food-cosmetic-scanner · Flow video: https://pageflows.com/post/ios/scanning-products/yuka/ · Mobbin (account needed): https://mobbin.com/explore/screens/d384625a-9e23-43b3-a400-9f7835e5639c

### 3. Oura — Finland (US market)
The cleanest "score + contributors" model.
- **Steal:** Three-tier disclosure — (1) Today tab with score pills, (2) tap a pill → score detail with band label (Optimal 85+ / Good / Fair / Pay Attention), 7-day trend, and a **contributors list where each row has its own mini-score and status**, (3) tap a contributor → raw metric graphs. Your sub-score rows should behave exactly like Oura contributor rows.
- **Flow:** Open app → scores at 0 taps → breakdown in 1 tap → raw data in 2.
- **See it:** Official Sleep Score explainer: https://support.ouraring.com/hc/en-us/articles/360025445574-Sleep-Score · Review with many app screenshots: https://www.dcrainmaker.com/2026/07/oura-ring-5-in-depth-review-comparison.html · Mobbin: https://mobbin.com/explore/screens/fb49ee69-838d-4018-a6e4-8eac8a95b7c7

### 4. Whoop — US (Boston)
Textbook three-tier progressive disclosure and disciplined color.
- **Steal:** Three dials at top (big number in semicircular gauge); Recovery color-coded green/yellow/red with *published thresholds*; dark UI where a strict three-color vocabulary does all the work; every tile is a doorway to a deeper view — nothing decorative.
- **Flow:** Home = 3 scores (0 taps) → tap dial = contributors + week trend (1) → swipe = raw graphs (2).
- **See it:** Design breakdown (screenshot-illustrated): https://www.925studios.co/blog/whoop-design-breakdown · Official home-screen post: https://www.whoop.com/us/en/thelocker/the-all-new-whoop-home-screen/ · Mobbin: https://mobbin.com/explore/screens/6a012278-d01c-473e-b860-3c08e35311ed

### 5. Credit Karma — US
- **Steal:** Semicircular dial with color band red→green and band label; "credit factors" list with per-factor status color and impact weight; scores labeled by bureau and model (VantageScore 3.0) — transparency by *naming the provider*.
- **Flow:** Dashboard (scores visible immediately) → tap score → history chart → factor rows → factor detail. ~2 taps deep.
- **See it:** Mobbin dashboard: https://mobbin.com/explore/screens/a74c7c40-04d7-4203-bc98-0dd899704644 · Annotated review: https://www.thewaystowealth.com/reviews/credit-karma-review/

### 6. NerdWallet — US
- **Steal:** Factor list *ranked from highest to lowest impact*, each with a status rating and a plain-English "why"; explainer content interleaved as education rather than hidden in a help center. Deliberately less cluttered dashboard than competitors.
- **Flow:** Dashboard → Credit Score tab → factor detail (1–2 taps) → report/simulator.
- **See it:** Flow recordings: https://pageflows.com/web/products/nerd-wallet/ · Redesign case study: https://www.solvd.com/cases/fintech-ui-ux-design-for-nerdwallet

## B. Switzerland — clarity, grids, institutional trust

### 7. SBB Mobile + SBB Digital Design System — Switzerland
Swiss Federal Railways' app; the canonical example of Swiss information design on dense real-time data.
- **Steal:** *Context-adaptive hierarchy* — the screen re-orders itself around the one thing that matters now and pushes everything else below the fold; near-monochrome palette (one brand red + grays) with a rigorous pictogram system.
- **Flow:** Home (current context) → search → connection list → expandable journey detail.
- **See it:** Design system: https://digital.sbb.ch/en/ · Redesign write-up by the UX lead: https://www.unic.com/en/magazine/sbb-mobile-app-design-principles

### 8. Swiss Confederation Design System (admin.ch) — Switzerland
Binding design rules for all Swiss federal websites; a direct descendant of the International Typographic Style (Frutiger is mandatory).
- **Steal:** *Mandatory attribution as a component* — every federal page must carry a standardized authority header so users always know who published the data; palette discipline of exactly 3 hues. This is the strongest model for your per-section "Source:" component.
- **See it:** Repo + Storybook: https://github.com/swiss/designsystem · Styleguide (color/type rules): https://swiss.github.io/styleguide/en/general.html

### 9. Interactive Things (Zürich) — data-viz studio
Their Cancer Monitoring Dashboard for the Swiss Federal Statistical Office is the closest existing analog to a "Building Health Card": a public-data civic scorecard.
- **See it:** https://www.interactivethings.com/work/ · Benchmark pool: Swiss Viz Awards https://www.swissviz.org/

## C. Nordics — humane civic UX

### 10. MitID — Denmark (national digital identity)
~5.6M users, 89% trust — remarkable for a mandatory government product.
- **Steal:** *The single recognizable frame.* Usability tests showed custom per-provider branding made users feel unsafe; one identical calm frame everywhere became the trust signal. Your identical section template = the same effect.
- **See it:** Case study by the design director: https://jesperbentzen.com/work/mitid

### 11. Helsenorge — Norway (national health portal)
- **Steal:** *Source attribution as trust architecture* — content explicitly labeled "quality-assured" with the providing authority named (Directorate of Health, Helfo…) and a stated review cadence. Provenance + freshness date: exactly your per-section citation pattern.
- **See it:** App case study: https://www.apps.no/project/helsenorge · Attribution model: https://www.helsenorge.no/en/about-helsenorge/

### 12. Lunar 5.0 — Denmark (challenger bank)
Post-redesign: −41% navigation support calls, rating 4.2→4.7.
- **Steal:** *"Daily vs. situational" IA* — everyday data on the main screen, situational detail structurally separated; theming held at the design-token layer.
- **See it:** Case study by Lunar's Head of Product Design: https://www.iamtomnewton.com/work/lunar

### 13. Helsinki Design System (HDS) — Finland
Open, accessibility-first, token-driven civic design system.
- **Steal:** Design tokens with contrast rules baked in — systematize your red→amber→green sub-score scale with guaranteed accessible contrast instead of hand-picking colors.
- **See it:** https://hds.hel.fi/getting-started/

### 14. Entur "Linje" — Norway (transit data platform)
- **Steal:** Color-coded "TravelTag" badges give each transport mode an instantly scannable identity — the same trick can give your four sub-scores distinct, consistent identities.
- **See it:** https://linje.entur.no/ · https://github.com/entur/design-system

### 15. Klarna — Sweden
- **Steal:** Hero-number typography: oversized tabular numeral, small muted label beneath, one soft accent per state, card-stack where each card carries a single dominant datum.
- **See it:** Mobbin (search "Klarna"): https://mobbin.com/explore

### 16. 1177.se Vårdguiden — Sweden (public healthcare guide)
- **Steal:** Search → filter/compare → structured provider profile, with accessibility prioritized over decoration — maps directly to your search → compare → Health Card flow.
- **See it:** Redesign case study: https://juliahildingsson.se/1177-vardguiden/ · Live: https://www.1177.se

## D. Germany & Netherlands — precision and plain language

### 17. N26 — Germany (Berlin fintech)
- **Steal:** Progressive disclosure anchored to a clean hero: the summary row shows only what the user's mental model needs; everything else (status, legal entity, tags) lives one tap deeper.
- **See it:** Case study by an N26 product designer: https://www.jonnyczar.com/project/n26 · Mobbin: https://mobbin.com/

### 18. Trade Republic — Germany
- **Steal:** *Color restraint as hierarchy* — near-monochrome canvas, accent color reserved strictly for meaning (gain/loss) and one primary action.
- **See it:** Product teardown: https://nextsprints.com/guide/trade-republic-product-teardown-analysis

### 19. DB Navigator + DB UX Design System — Germany
- **Steal:** *Status grammar for dense data* — black = on schedule, red = delay (with scheduled vs. expected shown side by side), tap through for the reason. Neutral by default; color only for exceptions. Public, open-source design system to crib component specs from.
- **See it:** https://design-system.deutschebahn.com/core-web/review/main/ · https://db-ui.github.io/ · App walkthrough: https://nomadepicureans.com/europe/step-by-step-guide-to-using-the-db-navigator-app-in-germany/

### 20. NS — Netherlands (railway)
- **Steal:** *Chunking* — their disruption-info redesign replaced text walls with small self-contained chunks (label → value → status → progress), each with clear hierarchy. Your detail sections should be chunks, never paragraphs.
- **See it:** Agency case study: https://www.designrebels.nl/projects/ns · Feature case study: https://www.moontaxi.co/work/nederlandse-spoorwegen

### 21. NL Design System — Netherlands (Dutch government, incl. DigiD ecosystem)
- **Steal:** Trust as a *system*: every claim gets visible provenance and plain-language explanation; accessibility-tested components with a "Hall of Fame" tier. Nothing on screen is unverifiable.
- **See it:** https://nldesignsystem.nl/ · EU profile: https://accessible-eu-centre.ec.europa.eu/nl-design-system-nlds-netherlands_en

### 22. bunq — Netherlands (neobank) — cautionary tale included
- **Steal (positive):** Dual-encoding: impact data shown both graphically and numerically (show the score AND its visual position).
- **Avoid (negative):** bunq V3 hid features behind a '+' button with no visible hierarchy and users revolted. Never tuck a sub-score behind a generic menu.
- **See it:** UX deep-dive: https://ergomania.eu/innovative-ux-solutions-from-bunq-neobank-digital-banking-in-the-netherlands-part-1/

## E. Japan — elegant density

### 23. PayPay — Japan (payments super-app)
- **Steal:** *Engineered density via layering* — hero content stays dominant; secondary data appears progressively on zoom/tap; monochromatic base layers with contrast reserved for what matters. Density is managed through disclosure, not deletion.
- **See it:** "Design at PayPay" (English): https://insideout.paypay.ne.jp/en/2024/06/12/design-at-paypay-vol4-en/

### 24. Yahoo!乗換案内 (Yahoo Transit) — Japan
Japan's most-downloaded transit app; the archetype of functional high-density UI.
- **Steal:** *Multi-criteria triage on strict columnar rows* — each result row packs departure, arrival, line, platform, fare, and distance yet stays scannable through rigid grid discipline; instant filtering by 早/楽/安 (fastest/easiest/cheapest). Proof that a row can carry score + status + source if the grid is strict.
- **See it:** Review with real screenshots: https://app-liv.jp/maps/routes/2830/

### Context reading (Japanese density philosophy)
- "The lies, myths, and secrets of Japanese UI design": https://www.disruptingjapan.com/the-lies-myths-and-secrets-of-japanese-ui-design/
- "What can we learn from Japan for UI Design" (Ma / negative space): https://uxplanet.org/what-can-we-learn-from-japan-for-ui-design-2f6ff8c0b3a2
- Key insight: Japanese users equate *visible detail with trustworthiness* — which aligns with your source-citation requirement; density is fine if structure carries it.

---

# Part 3 — Seven design principles the best examples share

1. **One hero, one glance.** A single dominant number with a color band *and* a 2–4-word plain-language label. Number + color + words together — never a bare number (Walk Score, Yuka, Oura, Klarna). Everything else on the screen is definitionally subordinate.

2. **Fixed bands with published meaning.** The color bands are a legend users learn once and trust everywhere ("Optimal 85+", "Walker's Paradise 90–100"). Ship a visible "what the score means" table and use the identical band system on the hero, the sub-scores, and every status chip (Oura, Walk Score, Whoop).

3. **Color is grammar, not decoration.** Neutral ink by default; color only signals state/exception (Trade Republic, DB Navigator, SBB). On your card: Class C violations and "unverified" states get color; everything else stays quiet. If everything is colored, nothing is.

4. **One rigid, repeatable section template.** Identical chunk structure for every data section — title, key values, one plain sentence, source line (MitID's "single recognizable frame"; NS chunking; Swiss federal attribution component). Consistency itself is the trust signal, and it makes "unverified" look deliberate rather than broken.

5. **Provenance is a component, not a footnote.** Every number carries its source in a standardized, designed element — agency name + freshness + link (Swiss Confederation DS, NL Design System, Helsenorge, Walk Score methodology, Yuka's scientific sources). Plus one permanent "How we calculate this" page with weights and caveats — your credibility engine, and perfectly aligned with HouseCheck's non-overclaiming voice.

6. **Progressive disclosure within a 3-tap budget.** Score → sub-score → evidence → source, never deeper; detail is revealed on tap, never deleted (Yuka, Oura, Whoop, N26, PayPay). The summary view shows 2–4 key data points per section, not all of them.

7. **Design tokens enforce the discipline.** Colors, type scale, spacing, and status chips exist as named tokens with accessibility contrast baked in (HDS Helsinki, Entur Linje, DB UX). For a near-beginner coder this is also the practical path: define ~10 tokens once, and the whole card stays consistent by construction.

---

## Practical next steps for the 10-day build
1. Define tokens first: 5 score-band colors (accessible), 2 text grays, 1 accent, type scale (hero / sub-score / body / source).
2. Build the section card component once with slots (title, status chip, data rows, "what this means", source) — reuse it 4×.
3. Build the hero badge as a standalone component (it's also your embeddable "widget" asset, like Walk Score's badge).
4. Treat "unverified," "out of coverage," and "not found" as designed states of the same components, not error pages.
5. Mobbin note: full flow libraries require a (free) account — budget 20 minutes to sign up and screenshot Yuka, Oura, Whoop, Klarna, N26 flows before you wireframe.
