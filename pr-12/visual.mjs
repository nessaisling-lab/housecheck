// PR #12's visual, made after it merged: what `node scripts/ci-local.mjs --push` runs before a push.
//   node visual.mjs      (run from anywhere inside the repo, with scripts/pr-visuals present)
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const lib = (f) => import(pathToFileURL(join(root, "scripts", "pr-visuals", f)).href);
const { boxOf, diagramSvg, renderDiagram } = await lib("diagram.mjs");
const here = fileURLToPath(new URL(".", import.meta.url));

const W = 1340, H = 520;
const n = (id, x, y, title, sub, kind = "new", scope = "added", h = 80) => ({ id, x, y, w: 230, h, title, sub, kind, scope });
const nodes = [
  n("branch", 20, 210, "Your branch", "committed, clean tree\n(--push refuses otherwise)", "context", "exact", 90),
  n("ci", 270, 210, "scripts/ci-local.mjs", "each check mirrors a\nnamed GitHub job", "new", "added", 90),
  n("rust", 540, 20, "fmt, clippy, build, test", "-D warnings, --locked;\nbuilds in target/ci-local"),
  n("secrets", 540, 120, "secret scan", "gitleaks 8.24.3 in Docker;\nthe branch's new commits"),
  n("smoke", 540, 220, "smoke + stability", "API on the fixture DB,\nport 18787, 50 requests"),
  n("frontend", 540, 320, "frontend + npm audit", "npm ci, lint, build, test;\naudit at high"),
  n("deps", 540, 420, "cargo audit + cargo deny", "advisories, bans, sources;\na missing tool fails the run"),
  n("gate", 810, 210, "10 of 10 passed?", "any FAIL or NOT RUN:\nlast 30 lines + its log", "new", "added", 90),
  n("push", 1090, 110, "git push", "to the branch's own name;\nnever to main"),
  n("github", 1090, 310, "GitHub CI", "the same checks again,\nplus CodeQL and macOS", "context", "exact", 90),
];
const checks = ["rust", "secrets", "smoke", "frontend", "deps"];
const edges = [
  { from: "branch", to: "ci", kind: "context", packets: 1 },
  ...checks.map((c) => ({ from: "ci", to: c, kind: "new", packets: 1, dur: 3 })),
  ...checks.map((c) => ({ from: c, to: "gate", kind: "new", packets: 1, dur: 3 })),
  { from: "gate", to: "push", kind: "fixed", packets: 1, label: "--push" },
  { from: "push", to: "github", kind: "context", fromSide: "b", toSide: "t", packets: 1 },
];
const svg = diagramSvg({ width: W, height: H, nodes, edges });
const files = await renderDiagram({
  pr: "#12",
  heading: "Local CI: GitHub's checks before a push",
  cols: [{ tag: "only", label: "What node scripts/ci-local.mjs --push runs · merged", shot: { svg, w: W, h: H }, marks: [
    { n: 1, kind: "new", badgeOnly: true, box: boxOf(nodes, "secrets"), html: "<b>Would have caught #10's failure.</b> Replayed on the old commit, it fails with the same 4 findings GitHub reported. From a worktree it reads the branch's commits; a scan that read none fails, the false pass the earlier build hit." },
    { n: 2, kind: "new", badgeOnly: true, box: boxOf(nodes, "ci"), html: "<b>Green here means green there.</b> A tool that is missing reports NOT RUN and fails the run, with the command to install it (cargo-deny, Oct 31)." },
    { n: 3, kind: "fixed", badgeOnly: true, box: boxOf(nodes, "push"), html: "<b>--push found its own bug on first use:</b> a branch cut from main tracks main, and git refused. It now pushes to the branch's own name and refuses to run on main." },
    { n: 4, kind: "gap", badgeOnly: true, box: boxOf(nodes, "github"), html: "<b>Still GitHub-only:</b> CodeQL and macOS. <code>--linux</code> (the Rust checks in a Linux container) is written but not yet run." },
  ] }],
  why: [
    ["What happened", "PR #10's secret scan failed on GitHub, and nothing on this PC could run that scan first."],
    ["When found", "Oct 31, on PR #10. Built the same day from the gala build's ci-local, with its two lessons: the worktree scan and a time limit per check."],
    ["How chosen", "No fast-lane skip: Actions minutes are free on a public repo, so GitHub still runs everything. A first run builds for about 12 minutes; a warm run takes about one."],
    ["Gap still open", "CodeQL and macOS stay on GitHub. --linux needs a first run (it downloads rust:1 and builds from scratch)."],
  ],
  when: "PR #12 · merged Oct 31, 2026 (5b61f9c)",
}, { out: here, name: "local-ci" });
console.log(files.join("\n"));
