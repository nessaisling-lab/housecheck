// PR #14's visual: the law-check workflow around PR #10's `ingest --law-check`.
//   node visual.mjs      (run from anywhere inside the repo, with scripts/pr-visuals present)
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const lib = (f) => import(pathToFileURL(join(root, "scripts", "pr-visuals", f)).href);
const { boxOf, diagramSvg, renderDiagram } = await lib("diagram.mjs");
const here = fileURLToPath(new URL(".", import.meta.url));

const W = 1340, H = 460;
const n = (id, x, y, title, sub, kind = "new", scope = "added", h = 90) => ({ id, x, y, w: 230, h, title, sub, kind, scope });
const [c1, c2, c3, c4, c5] = [20, 290, 560, 830, 1090];
const [r1, r2, r3] = [30, 180, 350];
const nodes = [
  n("run", c1, r1, "Run workflow", "Actions tab: dry run ticked,\nfull pass unticked", "action", "exact"),
  n("cron", c1, r3, "Monday 07:17 New York", "in the file, commented out;\non after one dry run", "gap", "open"),
  n("state", c2, r1, "Last good run's state", "artifact law-check-state;\nnone or expired: full pass"),
  n("key", c2, r3 + 15, "NYSENATE_API_KEY", "", "redacted", "exact", 60),
  n("ingest", c3, r2, "ingest --law-check", "PR #10's check, unchanged;\n--full when ticked", "context", "exact"),
  n("summary", c4, r1, "Run summary", "law-check.md, with the\nrequests and KB used"),
  n("diffs", c4, r2, "Word diffs", "whole sections: a word per\nline, so reflow is ignored"),
  n("artifacts", c4, r3, "Artifacts, 90 days", "report, texts and diffs;\nthe state feeds the next run"),
  n("issue", c5, r1, "law-check issue", "one rolling issue: edit the\nopen one, or open one"),
  n("repo", c5, r3, "The repo", "contents: read; the run\nnever edits law.json", "context", "related"),
];
const edges = [
  { from: "cron", to: "run", kind: "gap", fromSide: "t", toSide: "b", scope: "dashed" },
  { from: "run", to: "state", kind: "action", packets: 1 },
  { from: "state", to: "ingest", kind: "new", packets: 1, toAt: 0.3 },
  { from: "key", to: "ingest", kind: "redacted", scope: "dashed", toAt: 0.7 },
  { from: "ingest", to: "summary", kind: "new", packets: 1, fromAt: 0.25 },
  { from: "ingest", to: "diffs", kind: "new", packets: 1 },
  { from: "ingest", to: "artifacts", kind: "new", packets: 1, fromAt: 0.75 },
  { from: "diffs", to: "artifacts", kind: "new", fromSide: "b", toSide: "t", packets: 1 },
  { from: "summary", to: "issue", kind: "new", packets: 1 },
];
const svg = diagramSvg({ width: W, height: H, nodes, edges });
const files = await renderDiagram({
  pr: "#14",
  heading: "Law Phase C: the law-check workflow",
  cols: [{ tag: "only", label: ".github/workflows/law-check.yml · this PR", shot: { svg, w: W, h: H }, marks: [
    { n: 1, kind: "action", badgeOnly: true, box: boxOf(nodes, "run"), html: "<b>You start it.</b> Dispatch-only for now, and a dry run unless you untick it. A dry run writes the summary and artifacts, never an issue." },
    { n: 2, kind: "new", badgeOnly: true, box: boxOf(nodes, "state"), html: "<b>Change-only between runs.</b> The last good run's state rides forward as an artifact. None, or expired after 90 days, means a full pass." },
    { n: 3, kind: "redacted", badgeOnly: true, box: boxOf(nodes, "key"), html: "<b>The key stays hidden.</b> A repo secret, passed in the environment only and masked in the log. Never on a command line." },
    { n: 4, kind: "new", badgeOnly: true, box: boxOf(nodes, "diffs"), html: "<b>Word diffs for whole sections.</b> One word per line, so a reflowed paragraph shows only the words that changed." },
    { n: 5, kind: "new", badgeOnly: true, box: boxOf(nodes, "issue"), html: "<b>One rolling issue.</b> Opened only when a report line starts with a bold id; a test holds the report to that. A missing key opens it too, so nothing fails quietly." },
    { n: 6, kind: "gap", badgeOnly: true, box: boxOf(nodes, "cron"), html: "<b>No schedule yet.</b> Uncommenting one block turns on Mondays at 07:17 New York time, after one dry run passes and with your OK." },
    { n: 7, kind: "context", scope: "related", badgeOnly: true, box: boxOf(nodes, "repo"), html: "<b>Reports only.</b> The workflow reads the repo and never writes to it; the law HouseCheck serves stays as a person verified it." },
  ] }],
  why: [
    ["What happened", "Rollout step 5 of the law-change design: run PR #10's check on GitHub and keep its findings in one issue for a person to verify."],
    ["When found", "Oct 31, 2026, after the keyed run (fixed in a2cadbc) and the robots.txt check: all 25 URLs allowed on all 10 hosts."],
    ["How chosen", "Dispatch-only with a dry run first, as the design says. State kept as an artifact, not a commit, so the run never writes to the repo."],
    ["Gap still open", "Set the NYSENATE_API_KEY secret, one dry run, then the schedule. The daily bill and Council watches and supersede plus changelog are not built."],
  ],
  when: "PR #14 · Oct 31, 2026 (89f26c1)",
}, { out: here, name: "law-check-workflow" });
console.log(files.join("\n"));
