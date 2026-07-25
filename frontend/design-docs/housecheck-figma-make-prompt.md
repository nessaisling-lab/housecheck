# HouseCheck — Figma Make Prompt Spec
Paste this entire document into Figma Make. Generate the "Building Health Card" screen first, then the remaining screens one at a time, referencing the Design Tokens and Components sections each time.

---

## 1. Product summary
HouseCheck is a mobile-first web app: a renter enters an NYC address and receives a "Building Health Card" — a 0–100 score with four sub-scores (Condition, Legal, Neighborhood, Accessibility) and detail sections, each citing a public government data source. Tone: honest, calm, civic. Visual style: Apple iOS 26 Liquid Glass (light mode) for all floating chrome; Whoop fitness app's data structure (big ring gauges, tabular numerals, ALL-CAPS tracked labels) for all content.

## 2. Canvas and global rules
- Frame: iPhone, 390 × 844 px. Mobile-first.
- App background: #F6F6F8. Never pure white.
- All screens use the floating bottom chrome (Component C1) unless noted.
- Content sits on white cards; glass is used ONLY for floating chrome (bottom nav, agent orb, sticky header strip, sheets' input bars, overlays). Never place body text or scores directly on glass.
- No glass element overlaps another glass element.
- Corner radius system (concentric): cards 20px, rows/inner elements 14px, sheets 24px top corners, pills/chips/nav fully rounded (999px).
- Spacing: 8px grid. Screen side margins 20px. Card padding 20px. Gap between cards 16px.
- Elevation: single soft shadow style — 0 8px 32px rgba(0,0,0,0.08). No hard borders on cards.

## 3. Design tokens

### Colors
- Background: #F6F6F8 · Card surface: #FFFFFF · Inset/track surface: #EFEFF2
- Text primary: #1C1C1E · Text secondary: #3C3C43 at 60% · Text tertiary: #3C3C43 at 38% · Text on tinted: #FFFFFF
- Score bands (use for hero ring, sub-score numbers, status pills, map dots — the only saturated colors in the app):
  - Strong (80–100): #248A3D, label "Strong record"
  - Solid (60–79): #7DA629, label "Generally solid"
  - Mixed (40–59): #B7791F, label "Mixed signals"
  - Concern (20–39): #D04A1E, label "Real concerns"
  - Critical (0–19): #C7272B, label "Serious red flags"
  - Unverified/no data: #8E8E93, label "Unverified"
- Score wash: on the Health Card, the top 40% of the screen has a vertical gradient of the current score-band color at 8% opacity fading to transparent.

### Typography (SF Pro / Inter; all numbers tabular-nums)
- Hero number: 72px Semibold, −2% tracking
- Sub-score number: 28px Semibold
- Section header (sentence case, light): 28px Regular — e.g. "Building details"
- Section/card title (sentence case): 20px Regular
- Eyebrow/label (ALL-CAPS, +10% tracking): 13px Semibold, text-secondary
- Body: 17px Regular
- Data value: 15px Regular
- Status chip text: 13px Semibold
- Source line: 12px Regular, text-tertiary

### Glass styles (apply via Figma Glass Effect; fallback values in parentheses)
- glass/nav: white 55% fill, heavy background blur, high saturation, 1px white top border at 70%, shadow 0 8px 32px rgba(0,0,0,0.10), inner top highlight (Figma Glass Effect: moderate Frost, light Refraction)
- glass/sheet: white 72% fill, heavy blur — more opaque because sheets contain text
- glass/agent-orb: white 55% fill + blur, tinted with score-band or accent color at 25%, subtle outer glow ring

---

## 4. Components

**C1 — Bottom chrome (two detached floating elements, both 16px above screen bottom):**
- Left: capsule tab bar, height 64px, glass/nav, 4 equal tabs — Search (magnifier icon), Map (map icon), Compare (two-columns icon), More (three-lines icon). Each tab: 24px line icon + 11px label below. Active tab icon in text-primary, inactive at 40%.
- Right: circular agent orb, 56px, glass/agent-orb, centered sparkle icon 24px in white. This is the only tinted element in the app. It is NOT part of the tab bar — it floats 10px to the right of the capsule as its own element.

**C2 — Hero score gauge:** ring 184px diameter, stroke 12px rounded caps, track #EFEFF2, arc = score percentage in score-band color. Centered inside ring: score number (72px Semibold) with "/100" 15px text-tertiary beside baseline. Below ring: band label 15px Semibold in band color (e.g. "Mixed signals"). Below that: disclaimer line 12px text-tertiary: "Built from public NYC data. A screening signal, not a legal ruling."

**C3 — Sub-score row:** white row, height 64px, radius 14px. Left: mini ring 28px (stroke 4px) in sub-score's band color. Then: name 17px Semibold + one-line status 13px text-secondary beneath. Right: sub-score number 28px Semibold in band color + chevron-right icon. Four rows stacked with 8px gaps inside one white card (padding 6px, radius 20px).

**C4 — Section card (one template, used 4× on Health Card):** white card, radius 20px, padding 20px. Top row: section icon chip (44px rounded-square, band color at 12% bg, 22px glyph in band color) + section title 20px Regular + status pill right-aligned (C5). Middle: 2–4 label/value rows (eyebrow label left, 15px value right). Then one 17px sentence in text-secondary ("what this means"). Divider. Footer source line: 12px text-tertiary "Source: NYC HPD · Updated Jul 2026" + external-link icon.

**C5 — Status pill:** capsule, band color at 15% bg, text 13px Semibold full-strength band color, optional ▲▼ triangle glyph. Examples: "3 hazardous ▲", "Likely stabilized", "12% above median ▲", "Unverified" (gray #8E8E93).

**C6 — Rent spectrum track:** inset track #EFEFF2, height 48px, radius 14px, containing a ruler of 2px vertical tick marks (varying heights). Centered end labels below: "Below median" / "Above median" 12px text-tertiary. User marker: 4px-wide rounded bar, full track height, in verdict band color, with value label above ("$2,400").

**C7 — Sticky mini-ring strip:** appears pinned to top when hero scrolls away. Height 48px, glass/nav. Four groups evenly spaced: mini ring 20px + number 15px Semibold. Tapping jumps to section.

**C8 — AI summary card:** same white card template as C4 but with sparkle icon chip; contains 2–3 sentences of 17px plain-language summary; footer button "Ask the agent →".

**C9 — Skeleton loading:** gray #EFEFF2 pulsing shapes mirroring the layout: ring placeholder 184px, four row placeholders, three card placeholders.

**C10 — Chip/tag:** capsule, #EFEFF2 bg, 13px Semibold text-primary. Used for facts: "Elevator", "Built 1928", "6 floors".

---

## 5. Screens

### S1 — Home / Search
Top: wordmark "HouseCheck" 20px Semibold, left. Center-top headline 28px Regular, two lines: "Know the building before you sign." Below: search field, height 56px, radius 999px, white, soft shadow, magnifier icon left, placeholder "Enter a Brooklyn address". Below field: 3 example address chips (C10). Bottom third: quiet illustration area with 3 small stat chips: "250 buildings · Bed-Stuy · 6 public data sources". Bottom chrome C1. State variant: with suggestions — dropdown card under field listing 4 address rows (15px, divider-separated). State variant: inline message under field, text 13px #C7272B: "Address not found — try house number + street."

### S2 — Loading
Same layout as S3 but rendered as skeleton C9. Show for Building Health Card loads.

### S3 — Building Health Card (generate this screen FIRST)
Apply score wash gradient (Mixed #B7791F at 8%) to top 40%. Header: address "548 Gates Ave" 20px Semibold + "Bed-Stuy, Brooklyn" 13px text-secondary, left-aligned, margin-top 56px. Hero C2 centered, score 58 (Mixed). Below hero: AI summary card C8. Below: sub-score card containing 4× C3 rows — Condition 41 (Concern), Legal 72 (Solid), Neighborhood 68 (Solid), Accessibility 55 (Mixed). Then section header "Building details" (28px). Then 4× C4 section cards in order:
1. Rent fairness — icon banknote, pill "12% above median ▲" (#B7791F). Rows: "Your rent $2,400" / "Tract median $2,140" / "HUD FMR $2,320". Spectrum track C6 with marker right-of-center. Sentence: "Asking rent is above the neighborhood median, but within HUD fair-market range." Source: "US Census ACS · HUD · 2023".
2. Building condition — icon wrench, pill "3 hazardous ▲" (#D04A1E). Rows: "Class C (hazardous) 3" / "Class B 11" / "Class A 24" / "311 complaints (12 mo) 17". Sentence: "Open violations are a snapshot of today, not the building's full history." Source: "NYC HPD · NYC 311 · Jul 2026".
3. Legal protections — icon shield, pill "Likely stabilized" (#248A3D). Rows: "Rent stabilization Likely" / "Good Cause eviction Applies". Sentence: "Records suggest stabilization, but confirm with NYS DHCR before signing." Source: "NYS DHCR · NYC OpenData".
4. Accessibility — icon accessibility figure, pill "Mixed" (#B7791F). Rows: "Elevator Yes" / "Nearest ADA subway 850 m" / "Access likelihood Mixed". Chips row: "Built 1928" "6 floors" "42 units". Sentence: "Step-free access is possible but not verified." Source: "NYC DOB · MTA".
Bottom: two text buttons stacked, 15px Semibold: "Compare this building ＋" and "How we calculate this score →". Bottom chrome C1. Sticky variant: show C7 pinned at top with hero scrolled off.

### S4 — Section detail sheet (make 1, note 4 variants)
Sheet slides up, 90% screen height, 24px top radius, glass/sheet header band. Grabber handle centered. Header: section title 20px + close X. Body: full data table (8–10 label/value rows), then "What this means" paragraph (17px), then "Why this might be unverified" note card (#EFEFF2) for Legal variant, then large source card: agency name 17px Semibold + "Opens official source" + external-link icon. Footer: button "Ask the agent about this →" 15px Semibold. Variants: Rent, Condition, Legal, Accessibility — same layout, content from S3.

### S5 — Methodology ("How we calculate")
Header title "How scores work". Intro paragraph. Four band-legend rows: colored dot + band name + range + one-line meaning. Sub-score weights card: 4 rows with horizontal proportion bars. Data sources list: 6 rows (agency name + description + link icon). Disclaimer card at bottom.

### S6 — Out-of-coverage sheet
Sheet, 60% height. Title "We're not there yet" 28px. Body: "HouseCheck currently covers ~250 buildings in Bed-Stuy, Brooklyn for our pilot." Small map thumbnail with coverage dots. Primary button (capsule, #1C1C1E fill, white text): "Explore covered buildings". Secondary text button: "Get notified when we expand".

### S7 — Map / List explore
Top segmented control (glass/nav capsule, two segments: Map | List). Map variant: muted beige-gray map filling screen, 12–16 dots in score-band colors, one enlarged selected dot with score label. List variant: rows — address 17px Semibold + score pill right (C5 style). Bottom sheet mini-card (collapsed, 180px): address, small hero ring 64px with score, two stat chips ("3 Class C" "Likely stabilized"), buttons: "View full card" (primary capsule) + "＋ Compare" (text). Bottom chrome C1.

### S8 — Compare
Header "Compare" 28px + subtitle "3 buildings". Horizontally scrolling columns, one per building, column width 260px. Column header: mini ring 44px + score + address (2 lines). Below, aligned row groups with row labels in left frozen column (eyebrow style): Total score, Condition, Legal, Neighborhood, Accessibility, Class C violations, Stabilization, Elevator, ADA subway. Best value per row in its band color, others text-primary. Top-right of each column: small X to remove. Empty-state variant: centered illustration + "Add buildings from search or the map." Toast variant: bottom capsule toast "Compare holds 4 buildings — remove one first."

### S9 — AI agent sheet
Full-screen sheet. Subtle vertical gradient #F6F6F8 → faint tint of current building's band color. Top bar: agent logo pill (sparkle icon + "HouseCheck Agent" 13px Semibold capsule, glass/nav) left; history (clock) and tips (lightbulb) icons right. Context card when opened from a building: "Asking about 548 Gates Ave" with mini ring 28px. Conversation: agent messages in white cards (17px), user messages right-aligned in #EFEFF2 capsules. First agent message includes 3 suggested-prompt chips (glass/nav capsules): "Explain this score" · "Is it rent stabilized?" · "Compare to nearby buildings". Every data claim in agent text is followed by a 12px source line. Footer input bar: "＋" button (44px circle, #EFEFF2), input capsule (glass/nav, placeholder "Ask about this building…"), mic icon right. Loading variant: three-dot typing indicator in white bubble. Fallback variant: message "I couldn't answer that — here's the raw data" + button "View Health Card".

### S10 — States library (one frame, small components in a grid)
Unverified pill (gray), error/retry card ("Something went wrong — public data is unavailable right now" + "Try again" button), empty compare illustration, toast capsule, skeleton group, offline banner.

---

## 6. Generation order
1. S3 Building Health Card (defines hero, rows, section cards)
2. S1 Home/Search 3. S7 Map/List 4. S9 Agent sheet 5. S8 Compare 6. S4 Section detail 7. S6 Out-of-coverage 8. S5 Methodology 9. S2 Loading 10. S10 States
