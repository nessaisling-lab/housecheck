// PR #10's visual, made after it merged: how `ingest --law-check` re-reads every law source.
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
  n("law", 20, 210, "data/law.json", "37 sources: 32 laws\nand 5 legal-help entries", "context", "exact", 90),
  n("key", 270, 20, "NYSENATE_API_KEY", "", "redacted", "exact", 60),
  n("route", 270, 210, "lawcheck::route", "a route per host; an\nunknown host fails the run", "new", "added", 90),
  n("senate", 540, 20, "NY Senate API", "10 statutes: update feed,\nthen only the listed sections"),
  n("amlegal", 540, 120, "amlegal bulk XML", "12 Admin Code sections:\nETag, zip CRC, one entry"),
  n("opendata", 540, 220, "Open Data metadata", "1 dataset:\nviewLastModified"),
  n("pages", 540, 320, "HCR, 311, legal help", "11 pages: ETag or Last-\nModified; PDFs: pdftotext"),
  n("manual", 540, 420, "nycourts.gov, nyc.gov", "2 pages: a person re-checks\nevery 30 days", "context", "exact"),
  n("compare", 810, 210, "model::law compare", "passages or whole section;\nfacts whose quote is gone", "new", "added", 90),
  n("report", 1090, 260, "law-check.md", "changed, dead or moved,\ndue, not checked, evals"),
  n("state", 1090, 110, "law-check-state.json", "cursors, validators; a\nfinding stays open"),
  n("issue", 1090, 410, "Weekly issue", "the workflow and rolling\nissue: next PR", "gap", "open"),
];
const toSources = ["senate", "amlegal", "opendata", "pages", "manual"];
const edges = [
  { from: "law", to: "route", kind: "context", packets: 1 },
  { from: "key", to: "senate", kind: "redacted", scope: "dashed" },
  ...toSources.map((s) => ({ from: "route", to: s, kind: s === "manual" ? "context" : "new", packets: s === "manual" ? 0 : 1, dur: 3 })),
  ...toSources.filter((s) => s !== "manual").map((s) => ({ from: s, to: "compare", kind: "new", packets: 1, dur: 3 })),
  { from: "compare", to: "report", kind: "new", packets: 1 },
  { from: "compare", to: "state", kind: "new", packets: 1 },
  { from: "report", to: "issue", kind: "gap", fromSide: "b", toSide: "t", fromAt: 0.25, toAt: 0.25, scope: "dashed" },
];
const svg = diagramSvg({ width: W, height: H, nodes, edges });
const files = await renderDiagram({
  pr: "#10",
  heading: "Law Phase C: the weekly text check",
  cols: [{ tag: "only", label: "How ingest --law-check works · merged", shot: { svg, w: W, h: H }, marks: [
    { n: 1, kind: "new", badgeOnly: true, box: boxOf(nodes, "route"), html: "<b>Every source has a route.</b> A row from a new site fails the run and a test until someone picks how to check it." },
    { n: 2, kind: "new", badgeOnly: true, box: boxOf(nodes, "amlegal"), html: "<b>Change-only, as you asked.</b> A quiet week costs 14 requests and 661 KB. HCR, the bulk file and Met Council answer 304; a full pass runs on the first Monday." },
    { n: 3, kind: "fixed", badgeOnly: true, box: boxOf(nodes, "senate"), html: "<b>Found by your keyed run.</b> The API sends line breaks as a backslash and an n; all 10 statutes looked changed. Fixed in this PR: all 10 match." },
    { n: 4, kind: "redacted", badgeOnly: true, box: boxOf(nodes, "key"), html: "<b>The key never shows.</b> It rides only inside request URLs; errors print without URLs. Your run's outputs held it nowhere." },
    { n: 5, kind: "gap", badgeOnly: true, box: boxOf(nodes, "issue"), html: "<b>Not built yet:</b> the weekly workflow and rolling issue, the daily bill and Council watches with the \"may have changed\" warning, supersede plus changelog." },
  ] }],
  why: [
    ["What happened", "Rollout step 2 of the approved design: re-read every law source and report what changed. It never edits the law HouseCheck serves."],
    ["When found", "Design approved Oct 31; built and run the same day. Without the key, 25 of 25 sources were unchanged once one stored row got its missing \"…\" lines."],
    ["How chosen", "One cursor or validator per source, so only a source that signals a change is compared. A finding stays open until it comes back unchanged."],
    ["Gap still open", "The workflow (dispatch-only, dry run first), the daily watches, the warning in answers, and supersede plus changelog."],
  ],
  when: "PR #10 · merged Oct 31, 2026 (372f5ad)",
}, { out: here, name: "law-check" });
console.log(files.join("\n"));
