# HouseCheck — Frontend

"Carfax for NYC apartments" — mobile-first React app implementing the locked design system
(Whoop structure × Apple Liquid Glass, light-first). See `design-docs/` for the spec and
`wireframes/` for the 11 reference frames this build follows.

## Stack

React 19 + TypeScript + Vite + Tailwind (shadcn/ui base). No backend account system —
Saved / Recent / Compare tray live in `localStorage`.

## Run

```bash
npm install
npm run dev        # http://localhost:3000
```

## Connecting the backend (Rust/Axum)

The app calls the live backend at `https://housecheck-nessa.fly.dev` by default
(endpoints at root — no `/api` prefix). For local backend development, copy
`.env.example` → `.env.local` and set `VITE_API_URL=http://localhost:8080`.

Expected endpoints (see `design-docs/housecheck-chat-handoff.md`):

| Method | Path | Used by |
|---|---|---|
| GET | `/search?address=` | Home autocomplete |
| GET | `/building/{bbl}` | Health Card |
| GET | `/buildings` | More → covered buildings list |
| POST | `/rent-fairness` | Rent check inside Health Card (`{ bbl, monthly_rent }`) |
| GET | `/compare?bbls=a,b,c` | Compare screen |
| POST | `/summary` | Agent sheet opening summary (`{ bbl }`) |

**Demo-data fallback:** if the backend is unreachable, the app serves bundled sample data
(460 Macon St, 548 Gates Ave, 1230 Bedford Ave, 921 Fulton St) and labels it "demo data"
so demos never break. `src/lib/api.ts` → `normalizeBuilding()` tolerates minor field-name
variants from the backend (`class_a`/`a`, `violations`/`open_violations`, etc.).

## Structure

```
src/
  lib/        api client, mock fallback, localStorage store, score bands, agent context
  components/ ScoreRing, SubScoreRow, SectionCard, StatusPill, SpectrumTrack,
              SourceLine, NavChrome (capsule + orb), StickyStrip, Sheet, AgentSheet, Splash
  pages/      Home (search) · HealthCard · Saved · Compare · More (coverage + methodology)
```

## Design tokens

All colors/type/glass recipes come from `design-docs/housecheck-design-strategy.md` and live
as CSS variables in `src/index.css` (`--hc-*`). Band colors appear only where they encode
state; glass only on floating chrome (nav capsule, orb, sheets) — never under body text.

## Notable behaviors

- Splash ring draws <1s on launch
- Health Card: score-band wash behind hero, sticky mini-ring strip on scroll, anchor jumps
- Rent fairness check runs inside the card (no navigation), marker animates onto the spectrum
- Compare: 2–4 columns, best-in-row bold, max 4 enforced with toast
- States as first-class citizens: loading skeleton (min 400ms), not-found, out-of-coverage
  sheet, error/retry card, unverified chips, `prefers-reduced-motion/transparency` honored
- First-launch onboarding (optional, skippable): "What matters most to you?" — pick up to 2
  priorities; chosen Health Card sections float to the top with a subtle "Your priority" badge
  (reorder only, never hidden). Picks + local aggregate counts persist in localStorage
  (`hc.onboarding.v1`, `hc.priorityCounts.v1`)
