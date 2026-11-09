// PR #16's visual: smoke, stability and law-check build without debug info.
//   node visual.mjs      (run from anywhere inside a checkout with scripts/pr-visuals installed)
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const lib = (f) => import(pathToFileURL(join(root, "scripts", "pr-visuals", f)).href);
const { boxOf, diagramSvg, renderDiagram } = await lib("diagram.mjs");
const here = fileURLToPath(new URL(".", import.meta.url));

const W = 1290, H = 440;
const n = (id, x, y, title, sub, kind, scope = "exact") => ({ id, x, y, w: 260, h: 90, title, sub, kind, scope });
const [c1, c2, c3, c4] = [20, 350, 680, 1010];
const [r1, r2, r3] = [20, 175, 330];
const nodes = [
  n("cache", c1, r1, "Shared cache, 2.9 GB", "debug info for DuckDB's\nC++ and for the Rust", "wrong"),
  n("unpack", c2, r1, "Stability job unpacks it", "main, Nov 4; the same cache\nfit on the PR two days before", "wrong"),
  n("full", c3, r1, "No space left on device", "run 41922128527, before\nthe build even started", "wrong"),
  n("env", c1, r2, "Debug info turned off", "CARGO_PROFILE_DEV_DEBUG=0 in\nsmoke.yml and law-check.yml", "fixed"),
  n("duck", c2, r2, "DuckDB's build output", "5.1 GB down to 1.9 GB,\nmeasured on Windows", "fixed", "affected"),
  n("cache2", c3, r2, "Smaller shared cache", "size on Linux: shown by\nthis PR's CI", "gap", "open"),
  n("jobs", c4, r2, "smoke, stability, law-check", "same line, same cache key;\nlocal check 50 of 50", "fixed", "affected"),
  n("ndebug", c1, r3, "The cost, these jobs only", "DuckDB builds with NDEBUG;\nno line numbers in a crash", "context", "related"),
  n("ci", c2, r3, "ci.yml tests", "unchanged: keep debug info\nand DuckDB's assertions", "context", "related"),
];
const edges = [
  { from: "cache", to: "unpack", kind: "wrong" },
  { from: "unpack", to: "full", kind: "wrong" },
  { from: "env", to: "duck", kind: "fixed", packets: 1 },
  { from: "duck", to: "cache2", kind: "fixed", packets: 1 },
  { from: "cache2", to: "jobs", kind: "fixed", packets: 1 },
  { from: "env", to: "ndebug", kind: "context", fromSide: "b", toSide: "t" },
];
const svg = diagramSvg({ width: W, height: H, nodes, edges });
await renderDiagram({
  pr: "#16",
  heading: "CI: smoke, stability and law-check build without debug info",
  cols: [{ tag: "only", label: ".github/workflows/smoke.yml and law-check.yml · this PR", shot: { svg, w: W, h: H }, marks: [
    { n: 1, kind: "wrong", badgeOnly: true, box: boxOf(nodes, "full"), html: "<b>Out of disk on main.</b> Unpacking the cache left no room, before any build step. It fit two days earlier, so free space varies by runner." },
    { n: 2, kind: "fixed", badgeOnly: true, box: boxOf(nodes, "env"), html: "<b>No debug info.</b> One line in each workflow. DuckDB's bundled C++ follows Cargo's setting, so its debug info goes too." },
    { n: 3, kind: "fixed", scope: "affected", badgeOnly: true, box: boxOf(nodes, "duck"), html: "<b>5.1 GB down to 1.9 GB.</b> DuckDB's build output on Windows, the largest single part of the build." },
    { n: 4, kind: "gap", scope: "open", badgeOnly: true, box: boxOf(nodes, "cache2"), html: "<b>Linux size not measured yet.</b> This PR's CI shows the new cache size, to compare with 2.9 GB." },
    { n: 5, kind: "fixed", scope: "affected", badgeOnly: true, box: boxOf(nodes, "jobs"), html: "<b>All three share it.</b> law-check restores the same cache, so it had the same risk. The stability check passed 50 of 50 against this build." },
    { n: 6, kind: "context", scope: "related", badgeOnly: true, box: boxOf(nodes, "ndebug"), html: "<b>What it costs.</b> DuckDB's assertions don't run in these three jobs. The tests in <code>ci.yml</code> keep them." },
  ] }],
  why: [
    ["What happened", "The stability job on main ran out of disk unpacking the shared cache from PR #15. law-check restores that cache too."],
    ["When found", "Nov 9, 2026, in the checks on the merge of #15 (Nov 4)."],
    ["How chosen", "One env line, not a step that deletes the runner's preinstalled tools. It must match in both files, because it is part of the cache key."],
    ["Gap still open", "The Linux size waits for this PR's CI. Free disk varies, so this widens the margin without guaranteeing it. ci.yml keeps its debug info."],
  ],
  when: "PR #16 · Nov 9, 2026 (24222df)",
}, { out: here, name: "1-ci-disk" });
