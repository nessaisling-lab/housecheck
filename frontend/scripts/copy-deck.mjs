// Copy the presentation into public/ so Vercel serves it alongside the app.
//
// Runs automatically as npm `prebuild`. The deck lives in docs/deck/ as the single source
// of truth -- it is a living document, edited in place, and committed there. Copying at
// build time means it is served without a second 16 MB copy sitting in git.
//
// FAIL SOFT, DELIBERATELY. Vercel builds with a Root Directory setting, and depending on
// how that is configured the repo's docs/ may or may not be present in the build container.
// If the deck cannot be found this warns and exits 0: a missing slide deck must never be
// the reason a deploy of the actual product fails.
import { existsSync, mkdirSync, readdirSync, copyFileSync, statSync, writeFileSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));      // frontend/scripts
const frontend = resolve(here, "..");                       // frontend
const OUT = join(frontend, "public", "deck");

// Tried in order, so this works whether the build runs from the repo root or from frontend/.
const CANDIDATES = [
  resolve(frontend, "..", "docs", "deck"),
  resolve(frontend, "docs", "deck"),
  resolve(frontend, "..", "..", "docs", "deck"),
];

const src = CANDIDATES.find((p) => existsSync(p) && statSync(p).isDirectory());

if (!src) {
  console.warn(
    "[copy-deck] docs/deck not found in any of:\n  " +
      CANDIDATES.join("\n  ") +
      "\n[copy-deck] skipping. The app builds fine without it; /deck/ will 404.",
  );
  process.exit(0);
}

const html = readdirSync(src).filter((f) => f.toLowerCase().endsWith(".html"));

if (html.length === 0) {
  console.warn(`[copy-deck] no .html in ${src} -- skipping.`);
  process.exit(0);
}

mkdirSync(OUT, { recursive: true });

let bytes = 0;
for (const name of html) {
  const from = join(src, name);
  copyFileSync(from, join(OUT, name));
  bytes += statSync(from).size;
  console.log(`[copy-deck] ${name}`);
}

// Stable short entry point, because a URL someone types into a phone should be /deck.
// A REDIRECT rather than a second copy: the deck is 16.5 MB, and duplicating it here
// would double the deploy payload to alias one URL.
const primary = html.find((f) => /presentation/i.test(f)) ?? html[0];
const target = encodeURIComponent(primary);
writeFileSync(
  join(OUT, "index.html"),
  `<!doctype html>
<meta charset="utf-8">
<title>HouseCheck — the deck</title>
<meta http-equiv="refresh" content="0; url=./${target}">
<link rel="canonical" href="./${target}">
<p>Redirecting to <a href="./${target}">the HouseCheck deck</a>.</p>
`,
  "utf-8",
);

console.log(
  `[copy-deck] ${html.length} file(s), ${(bytes / 1048576).toFixed(1)} MB -> public/deck/ ` +
    `(/deck/ redirects to ${primary})`,
);
