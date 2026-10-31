// The PR visuals kit's own diagram: how a before/after gets made and published.
//   node scripts/pr-visuals/examples/diagram.mjs
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const lib = (f) => import(pathToFileURL(join(root, "scripts", "pr-visuals", f)).href);
const { boxOf, diagramSvg, renderDiagram } = await lib("diagram.mjs");
// Output goes to target/pr-visuals/kit/ (git-ignored); publish it with pr-media.mjs.
const here = join(root, "target", "pr-visuals", "kit") + "/";
mkdirSync(here, { recursive: true });

const W = 1340, H = 400;
const n = (id, x, y, title, sub, kind = "new", scope = "added") => ({ id, x, y, w: 230, h: 92, title, sub, kind, scope });
const [c1, c2, c3, c4, c5] = [20, 290, 560, 830, 1090];
const [r1, r2] = [50, 270];
const nodes = [
  n("before", c1, r1, "Before commit", "e.g. main before the PR;\ngit archive gives its frontend", "context", "exact"),
  n("after", c1, r2, "After commit", "this PR's branch, or the\nworking tree as it is now", "context", "exact"),
  n("build", c2, r1, "app.mjs", "builds each frontend at its\ncommit and serves it locally"),
  n("api", c2, r2, "Local API", "real pilot data, so the\nshots show the real bug", "context", "exact"),
  n("capture", c3, r1, "kit.mjs capture", "Playwright opens both apps;\nprivate text blanked in page"),
  n("raw", c3, r2, "Raw shots, local only", "before-*.png and after-*.png;\nnever published", "context", "related"),
  n("frame", c4, r1, "Frame + legend", "highlights, numbered notes,\nWhat / When / How / Gap"),
  n("pr", c4, r2, "The PR", "Markdown links show them\ninline; you also drag them in", "context", "exact"),
  n("png", c5, r1, "step1.png + GIF", "a PNG per step; a GIF when\nthere is more than one"),
  n("media", c5, r2, "pr-media branch", "pr-media.mjs commits the\nimages; never merged"),
];
const edges = [
  { from: "before", to: "build", kind: "new", packets: 1 },
  { from: "after", to: "build", kind: "new", packets: 1, fromSide: "t", toSide: "b", toAt: 0.3 },
  { from: "build", to: "capture", kind: "new", packets: 2 },
  { from: "api", to: "capture", kind: "context", fromSide: "t", toSide: "b", fromAt: 0.7, toAt: 0.3, label: "data", labelAt: [520, 200] },
  { from: "capture", to: "raw", kind: "context", fromSide: "b", toSide: "t", fromAt: 0.7, toAt: 0.7, scope: "dashed" },
  { from: "capture", to: "frame", kind: "new", packets: 2 },
  { from: "frame", to: "png", kind: "new", packets: 2 },
  { from: "png", to: "media", kind: "new", fromSide: "b", toSide: "t", packets: 1, label: "--push", labelAt: [1240, 228] },
  { from: "media", to: "pr", kind: "new", fromSide: "l", toSide: "r", packets: 1 },
];
const svg = diagramSvg({ width: W, height: H, nodes, edges });
const files = await renderDiagram({
  pr: "PR visuals",
  heading: "How a before/after gets made and published",
  cols: [{ tag: "only", label: "The kit · scripts/pr-visuals", shot: { svg, w: W, h: H }, marks: [
    { n: 1, kind: "new", badgeOnly: true, box: boxOf(nodes, "build"), html: "<b>Honest before/after.</b> Each side is the frontend as it was at its own commit, not a mock-up." },
    { n: 2, kind: "redacted", badgeOnly: true, box: boxOf(nodes, "capture"), html: "<b>Private text never leaves the page.</b> It is blanked in the page before the capture, so no saved file holds it." },
    { n: 3, kind: "new", badgeOnly: true, box: boxOf(nodes, "media"), html: "<b>Images live on <code>pr-media</code>,</b> one folder per PR, linked from the PR so they show inline." },
  ] }],
  why: [
    ["What happened", "PRs were a wall of text. Aisling reviews changes faster from marked-up before/afters, as on the gala guest hub build."],
    ["When found", "Oct 31, 2026: the rule moves to HouseCheck, for this build only for now."],
    ["How chosen", "The highlight library v1 as it is, plus Redacted. Playwright's own headless browser: installed Edge and Chrome exit at once in headless mode here."],
    ["Gap still open", "Portfolio run-throughs use the client-safe words (Before, Improved, New) when Aisling asks. Not yet a rule across all her projects."],
  ],
  when: "PR visuals kit · Oct 31, 2026",
}, { out: here, name: "diagram" });
console.log(files.map((f) => f.replace(here, "")).join("\n"));
