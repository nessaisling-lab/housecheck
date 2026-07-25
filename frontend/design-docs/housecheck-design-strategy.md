# HouseCheck Design Strategy
## Whoop structure × Apple Liquid Glass · light-first · v1

*This is the single source of truth before wireframes. Every Figma screen and every CSS decision references these tokens. Numbers marked ※ are community approximations (Apple publishes no px values for Liquid Glass); everything else follows Apple's official HIG.*

---

## 0. The fusion concept

**Whoop's skeleton, Apple's skin.**
- From **Whoop**: score-first hierarchy, giant tabular numerals, circular gauge language, a strict 3-color semantic vocabulary, 3-tier progressive disclosure (glance → breakdown → evidence), "every element is a doorway."
- From **Liquid Glass**: a light, airy canvas; translucent floating chrome (nav, sheets, overlays); concentric rounded shapes; content that softly scrolls beneath floating glass.

**The governing rule (Apple's own): glass is for chrome, never for content.**
Liquid Glass lives on the floating bottom nav, modals/sheets, overlays, and the primary CTA. Scores, data, and section cards sit on near-solid light surfaces where text contrast is guaranteed. This is not just aesthetics — NN/g measured serious legibility failures when text sits directly on glass (beta screens as low as 1.5:1 contrast vs. the 4.5:1 WCAG minimum). Our data is the product; it stays crisp.

---

## 1. Canvas & color tokens

### 1.1 Canvas (light-first)
Glass needs something to refract — over flat white the effect dies. So the content layer is never pure white:

| Token | Value | Use |
|---|---|---|
| `canvas/base` | `#F6F6F8` | App background |
| `canvas/wash-score` | subtle vertical gradient derived from the hero score band color at 6–10% opacity → transparent | Top 40% of the Health Card screen, behind the hero. Gives the glass nav something to lens, and tints the whole screen to the building's health — Whoop's "the score IS the screen" feeling, in light mode |
| `surface/card` | `#FFFFFF` | Section cards, sub-score rows |
| `surface/sunken` | `#EFEFF2` | Inset tracks (rent spectrum bar), code-ish data wells |

### 1.2 Ink
| Token | Value | Use |
|---|---|---|
| `ink/primary` | `#1C1C1E` | Numbers, titles |
| `ink/secondary` | `rgba(60,60,67,0.6)` (Apple secondaryLabel) | Labels, "what this means" sentences |
| `ink/tertiary` | `rgba(60,60,67,0.38)` | Source lines, timestamps |
| `ink/on-tint` | `#FFFFFF` | Text on colored chips/CTA |

### 1.3 Score bands (semantic — the ONLY saturated colors in the app)
Five bands, tuned for ≥4.5:1 contrast of their text on white, reused identically on hero, sub-scores, chips, map pins:

| Band | Range | Token | Hex | Label examples |
|---|---|---|---|---|
| Strong | 80–100 | `score/strong` | `#248A3D` | "Strong record" |
| Solid | 60–79 | `score/solid` | `#7DA629` | "Generally solid" |
| Mixed | 40–59 | `score/mixed` | `#B7791F` | "Mixed signals" |
| Concern | 20–39 | `score/concern` | `#D04A1E` | "Real concerns" |
| Critical | 0–19 | `score/critical` | `#C7272B` | "Serious red flags" |
| — | no data | `score/unverified` | `#8E8E93` | "Unverified" — a designed state, never an error |

**Color grammar (DB Navigator rule):** ink is neutral by default; band colors appear only where they encode state. Tinted glass (accent color) is reserved for exactly one thing per screen — usually the AI-agent button in the nav or the primary CTA. Tint everything = tint nothing (Apple).

---

## 2. Typography

Stack: `-apple-system, "SF Pro", "Inter", system-ui, sans-serif`. All numerals `font-variant-numeric: tabular-nums` (non-negotiable — Whoop's calm comes from numbers that don't jitter).

| Token | Size/weight | Use |
|---|---|---|
| `display/hero` | 72 / Semibold, −2% tracking | THE Building Health Score. Only one per screen, ever |
| `display/score-md` | 28 / Semibold | Sub-score numbers |
| `title/1` | 28 / Regular | Screen titles (rare — the score is the title) |
| `title/3` | 20 / Regular | Section titles ("Building condition") |
| `headline` | 17 / Semibold | Card titles, row labels |
| `body` | 17 / Regular | "What this means" sentences (17pt = legibility floor) |
| `subhead` | 15 / Regular | Data values in rows |
| `footnote` | 13 / Regular | Status chips, band labels |
| `caption` | 12 / Regular | Source lines ("Source: NYC HPD · Jul 2026") |

Hero number + footnote-size band label underneath, always paired (Walk Score rule: never a bare number).

---

## 3. Liquid Glass recipes

### 3.1 Figma (use the NATIVE Glass Effect — do not fake glassmorphism)
Figma shipped a native **Glass Effect built with Apple** (Jul 2025) with Refraction / Depth / Frost / Dispersion controls. Start from Apple's official **iOS 26 Figma kit** and tune:
- Nav bar glass: moderate Frost, light Refraction, subtle edge highlight.
- Apple's kit: figma.com/community/file/1527721578857867021 · Playground: /file/1522715486231239473/glass-effect-playground

### 3.2 CSS tokens ※ (for the React build; design Figma to match visually)
```
glass/nav (light):  bg rgba(255,255,255,0.55)
                    backdrop-filter: blur(20px) saturate(180%)
                    border-top: 1px rgba(255,255,255,0.7)
                    shadow: 0 8px 32px rgba(0,0,0,0.10), inset 0 1px 0 rgba(255,255,255,0.7)
glass/sheet:        bg rgba(255,255,255,0.72) + blur(24px) — more opaque: sheets carry text
glass/tinted-cta:   same recipe, tinted score-band/accent color at 20–30%
fallback:           @supports not (backdrop-filter) → rgba(255,255,255,0.88), no blur
a11y:               honor prefers-reduced-transparency (→ fallback) and prefers-reduced-motion
```
Apple's hard rules we adopt verbatim: no glass-on-glass stacking · no critical text on unmodified glass · "clear" variant only over rich media (we essentially never use clear) · dimming layer 35% if glass ever overlays bright imagery.

---

## 4. Shape, spacing, elevation

- **Concentric radii** (Apple): `radius/child = radius/parent − padding`.
  - `radius/card` 20 · `radius/row` 14 (inside 6px-padded cards) · `radius/sheet-top` 24 · pills/chips/nav = capsule (999).
- **Spacing**: 8pt grid, 4pt subdivisions. Card padding 16–20; section gap 16; hero block vertical padding 32+.
- **Elevation**: one shadow language only — soft, diffuse, low opacity (`0 8px 32px rgba(0,0,0,0.08)`). No hard borders on cards; separation comes from shadow + `#F6F6F8` canvas.

---

## 5. Component notes (Whoop → HouseCheck mapping)

| Whoop | HouseCheck | Spec |
|---|---|---|
| Recovery dial (hero) | **Health Score hero**: 72pt tabular number, concentric thin ring gauge in band color (track `surface/sunken`, arc = score %), band label + disclaimer line beneath, `canvas/wash-score` behind | Ring 168–192px ※, stroke 10–12px, rounded caps |
| 3 dials row | **4 sub-score rows** in a white card: mini 28px ring OR 8px band dot + `display/score-md` number + name + one-line status + chevron. Strict grid, equal weight | Row height 56–64px |
| Deep-dive screen | **Section cards** (identical template): title + status chip → 2–4 label/value rows → one plain sentence → divider → `caption` source line with link icon | Source line is a component, not text |
| Sleep/Strain Coach | **Rent fairness module**: input field → verdict chip + marker on `surface/sunken` spectrum track | Marker = band-colored dot with `% vs median` label |
| — | **Floating bottom chrome = TWO detached elements** (verified from Whoop screenshots): ① a capsule tab bar (`glass/nav`, 4 tabs, icon + 11px label) and ② a **separate circular agent orb** floating beside it — the AI agent is NOT a tab. Orb gets the only tinted glass + subtle glow (`glass/tinted-cta`) | Nav capsule height ~64px; orb ~56px; both float 12–16px above screen bottom; gap 8–10px ※ |
| Sticky score strip | **Collapsed sticky header**: when the hero scrolls away, a slim glass strip pins to top with 4 mini-rings + sub-score numbers (Whoop shrinks Sleep/Recovery/Strain rings into exactly this) | Strip height ~48px, mini-rings 20px, `glass/nav` material |
| Coaching card ("Strain Target Reached") | **AI summary card** on the Health Card: plain-language paragraph + small icon badge top-right; tap → full agent sheet | Same card template as sections, but with agent icon chip |
| "Pace of Aging" tick spectrum | **Rent fairness track**: a ruler of vertical ticks on `surface/sunken`, marker = user's rent positioned between "below median" / "above median" end labels, status pill above-right ("12% above median ▲") | Marker = 4px rounded bar in verdict band color; ticks 2px, varying heights ※ |
| 3-color vocabulary | 5-band vocabulary + gray "unverified" — same discipline, slightly wider | Legend lives on methodology page |

## 5b. Whoop screen autopsy (from the designer's 9 reference screenshots, IMG_7564–7572)

Measured/observed patterns now baked into the spec:

1. **Nav anatomy (adopted):** capsule tab bar + detached agent orb. Whoop's orb carries the brand monogram ("W") with a soft blue-violet glow ring; HouseCheck's orb = sparkle/agent glyph with our one allowed tint. The orb is the single most visually "alive" element on screen — everything else stays quiet.
2. **Ring gauge geometry:** thin ring stroke (~8–10px), number centered in light-weight display type, unit/suffix small, then ALL-CAPS wide-tracked label + chevron *below* the ring (not beside it). Hero ring for HouseCheck: same construction at ~180px.
3. **Sticky collapse:** full rings → mini-ring strip on scroll (adopted above).
4. **Label typography:** ALL-CAPS, letterspaced (+8–12% tracking), 13–15px, semibold — used for every row label and section eyebrow. Section *headers* ("My Day", "My Plan") are the opposite: large, sentence-case, light weight. Adopt both: eyebrows = `footnote` caps+tracking; section headers = `title/1` sentence case. This caps/sentence contrast is a big part of the Whoop feel.
5. **Card anatomy:** colored icon chip (rounded square, ~44px, tinted bg + glyph) + ALL-CAPS name + value → right side: stacked secondary values + 3px status edge-bar. Our section cards use this with band-color icon chips (Condition = wrench, Legal = shield, Neighborhood = buildings, Accessibility = person figure).
6. **Status pills:** small capsule chips with triangle glyph ("▼ slower vs. last week") in semantic color at 15% opacity bg + full-strength text. Use for verdicts ("12% above median ▲", "Likely stabilized").
7. **Agent sheet (light-mode translation):** full-screen sheet, subtle vertical gradient (`canvas/base` → faint tint), agent logo + label pill top-left, history + tips icons top-right, conversation in card bubbles, footer = "＋" attach button + input capsule ("Ask about this building…") + mic icon. Glass input capsule, not a boxed field.
8. **What we do NOT copy:** promotional carousels, shop/community tabs, streak gamification. Whoop's monetization clutter is the anti-pattern; our card stays civic-clean.

**States to design as first-class citizens:** loading skeleton (hero ring as pulsing gray ring), not-found, out-of-coverage (map + "we cover ~250 Bed-Stuy buildings for the MVP"), unverified, empty compare tray.

---

## 6. Accessibility floor
- All text ≥4.5:1 on its surface; band-colored text only on white/near-white.
- Tap targets ≥44×44pt.
- `prefers-reduced-transparency` and `prefers-reduced-motion` honored.
- The hero must survive a screenshot-without-color test: number + label carry meaning, color only reinforces.

## 7. References
- Apple HIG Materials: developer.apple.com/design/human-interface-guidelines/materials
- WWDC25 "Meet Liquid Glass": developer.apple.com/videos/play/wwdc2025/219/
- Whoop teardown: 925studios.co/blog/whoop-design-breakdown
- NN/g critique (our caution): nngroup.com/articles/liquid-glass/
- CSS recipe source: blog.logrocket.com/how-create-liquid-glass-effects-css-and-svg/
- Full annotated reference library: `housecheck-health-card-design.md`
