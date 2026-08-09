# Chapter 11 — Accessibility as a Correctness Property

> **The question this chapter answers:** What did WCAG 2.2 AA actually require of
> this build, which failures were measurable, and why were the contrast bugs the
> easy half?

---

## 1. The surface

Accessibility attributes in **app code only** — excluding the 53-file shadcn
scaffold from Chapter 10, which contributes 89 more that nothing imports:

```
aria-label 26   aria-hidden 20   :focus-visible 6   sr-only 5
prefers-reduced-motion 3   aria-busy 3   role="img" 3   aria-pressed 3
aria-live 2   aria-modal 2   role="log" 1   role="dialog" 1   role="status" 1
```

Small, and every item is load-bearing. There is no `aria-label` sprayed on a `div`
to silence a linter — a pattern worth checking for, and absent here.

The criteria the code names by number: **1.3.1** (info and relationships),
**1.4.3** (contrast minimum), **1.4.4** (resize text), **2.1.2** (no keyboard
trap), **2.4.3** (focus order), **4.1.3** (status messages). Citing the criterion
in the comment that satisfies it is the practice that makes this chapter possible
at all — it converts "we did some accessibility work" into a set of falsifiable
claims.

So let me falsify one.

## 2. The measurable half, done properly

`frontend/src/index.css:49-58` carries the best comment in the repository:

> **WCAG 2.2 AA 1.4.3.** Every token below is small text (status pills,
> "Unverified" labels, the 11px source line, statute links), so all of them need
> 4.5:1 — not the 3:1 large-text allowance. Measured against `--hc-sunken`
> (`#48484A`), the lighter of the two surfaces they sit on; anything passing there
> also passes on `--hc-card` (`#3A3A3C`). Hue and saturation are unchanged,
> lightness raised to the minimum that clears 4.5:1, so the palette keeps its
> identity. Previous values and their ratios on sunken: ink-3 0.44 = 3.19,
> strong `#4CC66A` = 4.17, concern `#F0743E` = 3.16, critical `#F0595C` = 2.73
> (below even the 3:1 non-text floor), unverified `#A0A0A6` = 3.51.

Four things right in one comment. It names the criterion. It justifies the
threshold rather than assuming it — these are 11px labels, so the large-text
allowance does not apply. It states the measurement surface *and why that surface*.
And it records the failing values it replaced, which is how a reader can check the
work instead of trusting it.

So I checked it. Recomputing every band token independently, sRGB relative
luminance, against both surfaces:

| token | on `--hc-card` | on `--hc-sunken` |
|---|---:|---:|
| `--hc-ink` | 10.42:1 | 8.38:1 |
| `--hc-strong` | 5.61:1 | **4.51:1** |
| `--hc-solid` | 6.70:1 | 5.39:1 |
| `--hc-mixed` | 5.61:1 | **4.51:1** |
| `--hc-concern` | 5.64:1 | **4.53:1** |
| `--hc-critical` | 5.62:1 | **4.52:1** |
| `--hc-unverified` | 5.61:1 | **4.51:1** |

Every one passes, and look at the column: 4.51, 4.51, 4.53, 4.52, 4.51. That is the
fingerprint of "lightness raised to the minimum that clears 4.5:1" — the comment
describes the method and the numbers confirm it was applied. The old `#F0595C` at
2.73:1 really was below even the non-text floor. The work was real.

The margin is **0.01**.

## 3. What a 0.01 margin does not survive

`--hc-card` is the token. It is not the background.

```css
.glass-card, .hc-card {
  background: linear-gradient(170deg,
    rgba(64, 64, 66, 0.94),
    rgba(52, 52, 54, 0.94));
  backdrop-filter: blur(22px) saturate(160%);
}
```

The rendered surface behind that small text is a gradient at **0.94 alpha**, whose
top stop is `rgb(64,64,66)` — lighter than the `#3A3A3C` token — composited over
the blurred page canvas `rgb(215,215,217)`.

Compositing it out (blur over a uniform canvas returns that canvas, and `saturate()`
is near-identity on a neutral grey, so the arithmetic is exact here):

```
0.94 × 64 + 0.06 × 215  =  73
```

The effective background at the top of every card is `rgb(73,73,75)` — **one unit
lighter than `--hc-sunken`**, the surface the palette was tuned against. And 0.01
of margin does not absorb that:

| token | on `--hc-sunken` | composited, card top | verdict |
|---|---:|---:|---|
| `--hc-strong` | 4.51:1 | **4.44:1** | fails 1.4.3 |
| `--hc-mixed` | 4.51:1 | **4.44:1** | fails 1.4.3 |
| `--hc-unverified` | 4.51:1 | **4.44:1** | fails 1.4.3 |
| `--hc-critical` | 4.52:1 | **4.44:1** | fails 1.4.3 |
| `--hc-concern` | 4.53:1 | **4.46:1** | fails 1.4.3 |

All five band colours — the status pills that say *strong*, *mixed*, *concern* —
fall below AA in the default rendering.

**Scope it honestly**, because the failure is not everywhere:

- It is the **top** of the 170° gradient. The bottom stop composites to
  `rgb(62,62,64)` and `--hc-strong` reaches **5.30:1** there, comfortably passing.
  The shortfall is a band across the upper region of each card.
- Over a lighter backdrop the gap widens — composited against white the same tokens
  land at **4.27–4.29:1**.
- And under `prefers-reduced-transparency: reduce` (`index.css:249`) the rules set
  `background: var(--hc-card)` with `backdrop-filter: none`. In that mode the
  measurement is exact and everything passes. **The only rendering path where the
  contrast claim is true is the accessibility fallback**, which was written for a
  different preference entirely.

The fix is trivial either way. Dropping the card alpha from `.94` to `1.0` puts
`--hc-strong` at **5.12:1**. Or keep the glass and raise each band token by two
units of lightness — `#5ECC79 → #60CE7B` — for 4.53:1 composited.

## 4. Why no checker catches this

This is the part worth generalising.

Automated tools — axe, Lighthouse, the browser's own contrast inspector — read
**computed styles**. `getComputedStyle` reports `background-image` as a gradient
string and `backdrop-filter` as a filter string. Neither tool composites them.
Faced with a translucent gradient over a blurred backdrop, a checker either skips
the element as indeterminate or falls back to the nearest opaque ancestor's
`background-color` — which is `--hc-card`, where every token passes at 5.6:1.

So the one contrast failure that survived is invisible to precisely the tools
everyone assumes make contrast a solved, automatable criterion. It survived a
careful manual audit too, because the audit measured the token — the thing the CSS
declares — rather than the pixel, the thing the user sees. The comment even says
so: *"Measured against `--hc-sunken`… the lighter of the two surfaces they sit
on."* That sentence is true about the tokens and false about the rendering, and it
is the reason the gap is findable at all. A less rigorous file would have left
nothing to check.

## 5. The half nobody can automate

Meanwhile the criteria that are genuinely hard were done correctly, by hand, and no
tool can confirm it.

**The focus trap** (`Sheet.tsx:23-27`):

> WCAG 2.2 AA 2.4.3 (focus order) + 2.1.2 (no keyboard trap outside the dialog).
> `aria-modal` alone hides the page from a screen reader but does nothing for the
> Tab key, so a keyboard user could tab out of an open [sheet].

That distinction is the whole thing. `aria-modal="true"` satisfies a checker; it
does not stop Tab. The implementation moves focus in, cycles at both ends, pulls
focus back if it escapes, handles Escape, and restores focus to the opener — with
an `opener?.isConnected` guard for the case where the element that opened the sheet
has since unmounted. That guard is the kind of detail that only appears after
someone hit it.

There is also a bug recorded in the comment: focus was originally set inside a
`requestAnimationFrame`, and *"focus silently never moved whenever rAF was
throttled — a background [tab]."* A screen-reader user opening a sheet in a
backgrounded tab would have been left with focus on the page behind it, silently.
No checker reports that. It is a race condition in an accessibility feature.

**The live region** (`AgentSheet.tsx:422-426`):

```tsx
role="log" aria-live="polite" aria-atomic="false"
aria-relevant="additions" aria-busy={busy}
```

Four attributes that only work as a set. `aria-atomic="false"` plus
`aria-relevant="additions"` means a screen reader announces the **new** message
rather than re-reading the entire conversation on every turn — which is what the
default gives you and what makes most chat UIs unusable non-visually. `aria-busy`
covers the wait. And `sr-only` speaker labels — *"Agent said:"*, *"You said:"* —
carry information that the visual layout encodes in position and colour, and that
a screen reader otherwise loses entirely.

An automated checker verifies that `role="log"` exists. It cannot tell you whether
the announcement cadence is right, whether focus cycles, or whether the label is
read before the message. Those require a person and a screen reader, and this
codebase did that work.

---

## The hardest question a reader can ask of this chapter

> *"The build claims WCAG 2.2 AA. You just showed five tokens fail 1.4.3 as
> rendered. Does it conform or not?"*

**Not fully, as rendered, and the honest statement has to be that specific.** Five
band tokens sit at 4.44–4.46:1 against the composited top region of `.hc-card`,
against a 4.5:1 requirement. That is a real failure of a real success criterion.
Naming it as anything softer would be the thing this whole book argues against.

Three qualifications that are true and none of which are a defence:

1. It is confined to the upper band of cards; the same text passes at 5.30:1 lower
   down the same gradient.
2. It is 0.06 short — a shortfall you cannot see, on a criterion that exists
   precisely because "I can see it fine" is not evidence.
3. Under `prefers-reduced-transparency` it passes exactly.

What actually deserves defending is the process, and it holds up better than the
result. Six criteria were cited by number in the code that satisfies them. The
contrast comment recorded its threshold, its justification, its measurement
surface, and the five failing values it replaced. That is why an outsider could
recompute the palette in an afternoon and find a 0.06 gap: **the claim was specific
enough to be wrong.** A file that said "colors updated for accessibility" would
have been unfalsifiable and, in practice, worse.

Ordered:

1. **Set card alpha to `1.0`.** One character. `--hc-strong` goes to 5.12:1 and all
   five pass. The glass effect on cards is doing very little at 0.94 anyway.
2. **Re-measure against the composited surface, not the token**, and record *that*
   in the comment — including the arithmetic, so the next reader can check it the
   way I checked this one.
3. **Add the composite to the note.** The next person to adjust the gradient or the
   canvas needs to know these tokens have no margin.
4. **Leave the behavioural work alone.** The focus trap and the live region are the
   parts that actually determine whether the product is usable without sight or
   without a mouse, and they are correct.

The inversion is the lesson. Contrast is the criterion everyone treats as solved
because a tool reports it, and it is where the one surviving defect lives — because
the tool measures a declaration and a person measures a pixel. Focus order and
status messages are the criteria nobody can automate, and they were done right.
Automation moved the failure to the place automation cannot look.

---

*Next: **Chapter 12 — What the Tests Actually Test.** 130 tests across the
workspace, what they pin, what they permit, and the four assertions that would have
caught this book.*
