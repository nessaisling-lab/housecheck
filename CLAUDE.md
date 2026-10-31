# HouseCheck: notes for Claude

## Pull requests are reviewed through pictures

Every PR carries a visual, made with `scripts/pr-visuals/` (see its README):

- **On-screen change:** a before and after, marked with the highlight library.
  - Use the full team words: Wrong, Fixed, Do, New, Gap, Note, Redacted.
  - Use one frame per step, and a GIF when there are several.
  - Each frame has the notes, the four boxes (What happened, When found, How chosen, Gap still
    open) and a legend.
- **Backend, data or tooling change:** a diagram of what changed, in the same library.
- **Anything private** (keys, email addresses, a real tenant's details) is **blanked in the page
  before the capture** and marked Redacted. Never cover it afterwards.
- **Publishing:** `node scripts/pr-visuals/pr-media.mjs <pr> <folder> --push` puts the images on
  the `pr-media` branch (never merged) and prints the Markdown for the PR. Then remind Aisling to
  drag the same images into the PR by hand as a backup.
- **Client-safe words** (Before, Improved, New, never Wrong) are only for portfolio run-throughs,
  and only when Aisling asks.

Use `.github/pull_request_template.md` for the rest of the description.
