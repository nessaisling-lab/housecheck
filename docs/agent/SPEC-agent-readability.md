# Spec — Agent response readability and accessibility

**For:** Anthony · **Written:** 2026-07-27 · **Scope:** `frontend/src/components/AgentSheet.tsx` and one backend prompt string

Read `docs/agent/LLM-RULES.md` before starting if you're using an AI assistant on this.

---

## 0. Pull first — main moved a lot

Main is **20 commits ahead** of your last push (`771c38a`), and two of them touch `AgentSheet.tsx`:

- `3b6c494` — fixed four defects in the sheet: it was discarding the demo/live `source` flag and rendering mock text under a real source line; the "Attach" button had no handler and was removed; the agent now receives the rent-fairness result through context.
- `c2d5560` — **replaced the `setTimeout` stub with a real API call.** `send()` is now async and posts to `POST /agent/chat`. `answerChip` survives as the offline path, labelled `· offline answer`.

There's also a new `sendChat()` in `frontend/src/lib/api.ts` and a 70s LLM timeout. Pull before you touch anything or you'll rebuild work that exists.

## 1. The problem

Live screenshot from a real answer:

```
**The law you need to know** | Statute | What it says ||---------|---------|
| **NY Real Property Law § 235-b** (Warranty of Habitability) | Every
residential lease carries an implied warranty… **Cannot be waived by lease.**
```

The model is producing *good* markdown — headings, a table, numbered lists. `AgentSheet.tsx:241` renders it with `{m.text}`, a plain text node, and there is **no markdown library in the project at all**. So every answer arrives as one unbroken block with literal `**` in it.

Two consequences, and the second is the one that matters more:

1. It's hard to read.
2. **A screen reader gets no structure whatsoever.** It reads `asterisk asterisk N Y Real Property Law asterisk asterisk pipe pipe dash dash dash` — no headings to jump between, no list semantics. Rendering the markdown emits real `<h3>`, `<ul>`, `<strong>`, which fixes both problems in one change. Do this task first.

## 2. Task 1 — render the markdown

**Add:** `react-markdown` + `remark-gfm` (~30 KB gzipped).

**Use `react-markdown`, not `marked` + `dangerouslySetInnerHTML`.** It doesn't inject raw HTML by default. That matters here: agent output is shaped by whatever the user typed *and* by web-search results from the `search_law` tool, which pulls from external sites. Treat it as untrusted even though we generated it.

Render only for `role === "agent"`. User messages stay plain text — they never contain markdown and shouldn't be parsed.

Pass a `components` map so output inherits our tokens rather than browser defaults. Available: `--hc-ink`, `--hc-ink-2`, `--hc-ink-3`, `--hc-sunken`, `--hc-card`, `--hc-strong`, `--hc-concern`, `--hc-critical`, `--hc-unverified`.

Rough shape:

| Element | Treatment |
|---|---|
| `h2` / `h3` | The uppercase eyebrow style already used for section labels — 11px, letter-spaced, `--hc-ink-3` |
| `strong` | `--hc-ink`, weight 600. It carries the key term in almost every answer |
| `p` | `--hc-ink-2`, `leading-relaxed`, ~10px bottom margin |
| `ul` / `ol` | Tighter than browser default; the answers are list-heavy |
| `a` | `--hc-strong`, underline offset, external-link affordance. Statute links must be obviously tappable |
| `table` | **Stack it** — see task 2 |
| `hr` | Hairline in `--hc-sunken`, or drop it entirely |

**Do not** style the source line through the markdown map — `m.source` is separate and already handled.

## 3. Task 2 — no tables on a phone

A markdown table needs ~600px. The sheet is ~340px. Two halves:

**Backend (not yours — coordinate with Aisling).** `AGENT_SYSTEM_PROMPT` in `crates/api/src/main.rs` gains a line telling the model answers render in a narrow mobile sheet, never to use tables, and to prefer short sections with a bold lead-in and bullets.

**Frontend (yours).** Belt and braces: map `table` in the components map to stacked blocks — one bordered row per table row, label above value — so a table that slips through degrades instead of forcing a horizontal scroll. The prompt will fail eventually; the renderer shouldn't.

## 4. Task 3 — accessibility

Four real WCAG 2.2 AA gaps. Currently `AgentSheet.tsx` has **zero** `aria-live` regions.

| # | Criterion | What's wrong | Fix |
|---|---|---|---|
| 3a | **4.1.3 Status messages** | An agent reply appears silently. A screen-reader user has no idea it arrived. | `aria-live="polite"` + `aria-atomic="false"` on the message list container. Set `aria-busy="true"` while `busy` |
| 3b | **1.3.1 Info and relationships** | No semantic structure | Free once task 1 lands |
| 3c | **2.4.3 Focus order** | Opening the sheet doesn't move focus; closing doesn't restore it | Focus the input on open, trap focus inside while open, return focus to the orb on close. Escape already closes — keep that |
| 3d | **1.4.4 Resize text** | 7 hardcoded `px` sizes (lines 206, 220, 240, 244, 252, 275, 297) ignore the user's browser text-size setting | Convert to rem, then task 4 |

Also measure `--hc-ink-3` on `--hc-sunken` against **1.4.3** — 4.5:1 for body text. That pairing is used for the source line and hints; I have not verified it passes.

**Testing:** axe DevTools for the mechanical checks, then actually tab through the sheet with the keyboard only, and run VoiceOver (⌘F5) or NVDA over one legal answer. The keyboard pass will find the focus-trap bugs that axe won't.

## 5. Task 4 — text-size control

A four-step control — Small / Default / Large / Larger — in the **More** tab, scaling root font-size (roughly 0.875 / 1 / 1.125 / 1.25). Persist to `localStorage` next to the existing prefs in `frontend/src/lib/store.ts`.

Depends on 3d: it only works once sizes are rem-based. This is why 3d comes first.

Not strictly required for AA — browser zoom technically satisfies 1.4.4 — but an in-app control is the difference between compliant and usable, and this app's users may be reading a violation history on a phone in a hallway.

## 6. Acceptance

- [ ] A legal answer renders with visible headings, bullets, and bold — no literal `**` anywhere
- [ ] No horizontal scroll at 320px width
- [ ] Statute links are tappable and open correctly
- [ ] Screen reader announces a new agent reply without the user hunting for it
- [ ] Full keyboard round trip: open sheet → type → send → read reply → Escape → focus back on the orb
- [ ] Text-size control persists across reload
- [ ] `npm run lint` and `npm run build` both clean

## 7. Don't break these

- **The offline path.** `answerChip` renders when the agent is unavailable and is labelled `· offline answer`. It must keep working and keep its label — an unlabelled canned answer looks like a live one, which is the exact bug `3b6c494` fixed.
- **Citations come from the server.** `m.source` is built from the API's `citations[]`. Don't hardcode a source line; that was the original defect.
- **User messages stay unparsed.** Rendering markdown in user text is an injection surface for no benefit.

## 8. Effort

| Task | Estimate |
|---|---|
| 1 — markdown rendering | ~1h |
| 2 — table stacking | ~30m |
| 3 — a11y | ~1h |
| 4 — text-size control | ~2h |

Tasks 1 and 3 are the ones worth doing regardless. 4 is a genuine feature.
