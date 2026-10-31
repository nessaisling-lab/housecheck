# PR visuals

Every HouseCheck pull request is reviewed through pictures, not a wall of text:

- **A change you can see on screen** gets a marked-up **before and after**.
- **A backend or tooling change** gets a **diagram** of what changed.

Both use one highlight library, so every image reads the same way.

## The highlight library (v1.1)

**Color = what kind of thing it is.** Every note starts with its kind's word, so nothing depends
on color alone.

| Kind | Word | Means |
|---|---|---|
| `wrong` | **Wrong** (red) | what was wrong (before) |
| `fixed` | **Fixed** (green) | the fix (after) |
| `action` | **Do** (blue) | what you do in this step |
| `new` | **New** (purple) | it didn't exist before |
| `gap` | **Gap** (amber) | still open, or watch out |
| `context` | **Note** (gray) | unchanged, shown for reference |
| `redacted` | **Redacted** (black, striped) | hidden on purpose: private or sensitive |

**Border = how directly a note applies:**
- **solid with a glow:** the exact thing;
- **dashed:** also affected;
- **dotted:** related, unchanged;
- **double:** a new element;
- **hatched:** not resolved yet.

**Shape = what it points at:**
- **box:** an area;
- **ring:** a small target;
- **underline:** a wording change;
- **arrow:** where something goes next.

PRs use the words above. For a portfolio run-through, and only when Aisling asks, use the
client-safe pair instead: `was` (gray **Before**) and `improved` (green **Improved**), with `new`.
They never say "Wrong".

## Redaction

Anything private, such as a key, an email address or a real tenant's details, is **blanked in the
page before the capture**: `capture(page, { redactions: [locator] })`. The text becomes block
characters and images are hidden, so no saved file holds the value. The frame then marks the spot
**Redacted**. Never blur or cover a value after the capture; the raw file would still hold it.

## Making the visuals for a PR

One-time setup, in `scripts/pr-visuals`:

```bash
npm ci
node setup.mjs
```

`setup.mjs` installs Playwright's headless browser into HouseCheck's own labelled folder:
`%LOCALAPPDATA%\ms-playwright\housecheck` on Windows, `~/.cache/ms-playwright/housecheck` on
Linux. The shared `ms-playwright` folder belongs to other tools (gstack, career-ops, global and
Python Playwright) and is left alone. `PLAYWRIGHT_BROWSERS_PATH` overrides the folder.

Edge and Chrome as installed on the development PC exit at once in headless mode, so the kit uses
Playwright's own headless browser. Set `PR_VISUALS_BROWSER=msedge` or `chrome` to try an
installed one.

Write the PR's script in `target/pr-visuals/pr-<n>/visual.mjs` (git-ignored). Start from an
example:

- `examples/before-after.mjs`: PR #9's 311 row.
  - `app.mjs` builds the frontend as it was at the before commit and at the after commit.
  - Both builds talk to a local API on the pilot database.
  - The script captures the same spot on each and renders one frame.
- `examples/diagram.mjs`: this kit's own diagram, with boxes, right-angled lines, moving dashes
  and a GIF.

Each frame carries:
- the before and after (or the diagram);
- a numbered note per highlight;
- four boxes: **What happened**, **When found**, **How chosen**, **Gap still open**;
- a legend of only the kinds it uses, and a line saying where the images came from.

A change that takes several steps gets one frame per step, plus a GIF of them (needs ffmpeg).

## Publishing

```bash
node scripts/pr-visuals/pr-media.mjs <pr-number> target/pr-visuals/pr-<n> --push
```

- **What it publishes:** the frames, GIFs and the script that made them, into `pr-<n>/` on the
  `pr-media` branch. That branch holds images only and is never merged.
- **What it leaves behind:** raw captures (`before-*`, `after-*`) stay local.
- **What it prints:** the Markdown to paste into the PR; the images show inline from there.

Then drag the same images into the PR comment by hand as a backup.

## Files

| File | What it does |
|---|---|
| `kit.mjs` | the library: kinds, highlights, notes, legend, frame, browser, redaction, capture, render to PNG and GIF |
| `diagram.mjs` | backend diagrams in the same library: boxes, right-angled lines, moving dashes and packets |
| `app.mjs` | the app at any commit: build the frontend there, serve it, run the local API |
| `pr-media.mjs` | publish to the `pr-media` branch and print the Markdown |
| `setup.mjs` | one-time install of the headless browser into `ms-playwright/housecheck` |
| `kit.test.mjs` | `npm test`: the library's rules, line routing, what gets published, and redaction in a real browser |
