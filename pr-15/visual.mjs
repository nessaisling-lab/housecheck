// PR #15's visual: the law-check workflow with its Monday schedule switched on.
//   node visual.mjs      (run from anywhere inside the repo, with scripts/pr-visuals present)
import { execFileSync } from "node:child_process";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const lib = (f) => import(pathToFileURL(join(root, "scripts", "pr-visuals", f)).href);
const { boxOf, diagramSvg, renderDiagram } = await lib("diagram.mjs");
const here = fileURLToPath(new URL(".", import.meta.url));

const W = 1340, H = 460;
const n = (id, x, y, title, sub, kind = "context", scope = "related", h = 90) => ({ id, x, y, w: 230, h, title, sub, kind, scope });
const [c1, c2, c3, c4, c5] = [20, 290, 560, 830, 1090];
const [r1, r2, r3] = [30, 180, 350];
const nodes = [
  n("run", c1, r1, "Run workflow", "Actions tab: still there,\na dry run by default"),
  n("cron", c1, r3, "Monday 07:17 New York", "on: first run Nov 2, the\nfirst Monday, so a full pass", "new", "added"),
  n("state", c2, r1, "Last good run's state", "Oct 31 dry runs: full pass,\nthen one from its state", "context", "exact"),
  n("key", c2, r3 + 15, "NYSENATE_API_KEY", "", "redacted", "exact", 60),
  n("ingest", c3, r2, "ingest --law-check", "PR #10's check, unchanged"),
  n("summary", c4, r1, "Run summary", "law-check.md, with the\nrequests and KB used"),
  n("diffs", c4, r2, "Word diffs", "whole sections only"),
  n("artifacts", c4, r3, "Artifacts, 90 days", "report, texts and diffs;\nthe state feeds the next run"),
  n("issue", c5, r1, "law-check issue", "scheduled runs are never\ndry runs: findings land here", "new", "affected"),
  n("repo", c5, r3, "The repo", "contents: read; the run\nnever edits law.json"),
];
const edges = [
  { from: "cron", to: "run", kind: "new", fromSide: "t", toSide: "b", packets: 1 },
  { from: "run", to: "state", kind: "context", packets: 1 },
  { from: "state", to: "ingest", kind: "context", packets: 1, toAt: 0.3 },
  { from: "key", to: "ingest", kind: "redacted", scope: "dashed", toAt: 0.7 },
  { from: "ingest", to: "summary", kind: "context", fromAt: 0.25 },
  { from: "ingest", to: "diffs", kind: "context" },
  { from: "ingest", to: "artifacts", kind: "context", fromAt: 0.75 },
  { from: "diffs", to: "artifacts", kind: "context", fromSide: "b", toSide: "t" },
  { from: "summary", to: "issue", kind: "new", packets: 1 },
];
const svg = diagramSvg({ width: W, height: H, nodes, edges });
const files = await renderDiagram({
  pr: "#15",
  heading: "Law Phase C: the law-check schedule is on",
  cols: [{ tag: "only", label: ".github/workflows/law-check.yml · this PR", shot: { svg, w: W, h: H }, marks: [
    { n: 1, kind: "new", badgeOnly: true, box: boxOf(nodes, "cron"), html: "<b>Weekly, on New York time.</b> Mondays at 07:17, after the Senate's Friday batch. The first run is Monday, Nov 2: the first Monday of the month, so every source is compared." },
    { n: 2, kind: "new", scope: "affected", badgeOnly: true, box: boxOf(nodes, "issue"), html: "<b>Findings now reach the issue.</b> A scheduled run has no inputs, so it is never a dry run. With nothing found, the issue is left alone." },
    { n: 3, kind: "context", badgeOnly: true, box: boxOf(nodes, "state"), html: "<b>Proved on Oct 31.</b> A full pass with no saved state: 35 unchanged, 26 requests, 2,088 KB. Then a pass from its state: all quiet, 17 requests, 662 KB." },
    { n: 4, kind: "redacted", badgeOnly: true, box: boxOf(nodes, "key"), html: "<b>The key stayed hidden.</b> Both logs show it only as <code>***</code> in the step's list of environment variables." },
    { n: 5, kind: "context", scope: "related", badgeOnly: true, box: boxOf(nodes, "run"), html: "<b>Unchanged:</b> the run by hand, the check, the diffs, the artifacts and the read-only repo." },
  ] }],
  why: [
    ["What happened", "The last part of rollout step 5: switch on the schedule that PR #14 left commented out."],
    ["When found", "Both dry runs passed on Oct 31, 2026; Aisling said to turn it on Nov 1."],
    ["How chosen", "As the design says: Monday 07:17 New York, minute 17 to dodge top-of-hour queues. The daily bill and Council run waits for those watches."],
    ["Gap still open", "A dispatch run can't prove GitHub honours timezone: the Nov 2 run should start soon after 07:17 New York, not UTC. GitHub pauses schedules after 60 days with no repo activity."],
  ],
  when: "PR #15 · Nov 1, 2026 (934ea99)",
}, { out: here, name: "1-law-check-schedule" });

// Frame 2: the test that started failing on Nov 1, and the fixed test day.
const W2 = 1340, H2 = 300;
const m = (id, x, y, title, sub, kind, scope = "exact") => ({ id, x, y, w: 250, h: 90, title, sub, kind, scope });
const nodes2 = [
  m("rec", 20, 105, "Recorded Oct 31", "the model's words: \"tomorrow\n(2026-11-01), 1 day away\"", "context"),
  m("real", 330, 20, "Replayed Nov 1", "server counts from the real\nday: tomorrow is 2026-11-02", "wrong"),
  m("check", 640, 20, "Number check", "\"01\" is in the answer and in\nnothing the model was shown", "wrong"),
  m("pin", 330, 190, "TEST_TODAY 2026-10-31", "under cfg(test), today_iso()\nreturns the recording day", "fixed"),
  m("pass", 640, 190, "Replay agrees again", "tomorrow is 2026-11-01;\n94 of 94 API tests pass", "fixed"),
  m("clock", 1070, 190, "clock_today_iso()", "the release build still\nreads the real clock", "context", "related"),
];
const edges2 = [
  { from: "rec", to: "real", kind: "wrong" },
  { from: "real", to: "check", kind: "wrong", packets: 1 },
  { from: "rec", to: "pin", kind: "fixed" },
  { from: "pin", to: "pass", kind: "fixed", packets: 1 },
];
const svg2 = diagramSvg({ width: W2, height: H2, nodes: nodes2, edges: edges2 });
files.push(...await renderDiagram({
  pr: "#15",
  heading: "Also: the API's tests run on a fixed day",
  cols: [{ tag: "only", label: "crates/api/src/main.rs · this PR", shot: { svg: svg2, w: W2, h: H2 }, marks: [
    { n: 1, kind: "wrong", badgeOnly: true, box: boxOf(nodes2, "check"), html: "<b>Failed on Nov 1 with no code change.</b> <code>replayed_conversations_hold_on_both_endpoints</code>, on <code>urgent-court-date-tomorrow</code>: the recorded words say 2026-11-01; the server now said 2026-11-02." },
    { n: 2, kind: "fixed", badgeOnly: true, box: boxOf(nodes2, "pin"), html: "<b>Tests run on the recording day.</b> A replay counts from the day its words were recorded. Live re-records are tests too, so they count from it as well." },
    { n: 3, kind: "context", scope: "related", badgeOnly: true, box: boxOf(nodes2, "clock"), html: "<b>Production unchanged.</b> The test that once caught <code>2026-00-08</code> now checks <code>clock_today_iso()</code>, the path the release build uses." },
  ] }],
  why: [
    ["What happened", "Local CI for this PR failed one agent eval that passed on Oct 31. Main and every PR would have failed from Nov 1."],
    ["When found", "Nov 1, 2026, running local CI on the schedule change."],
    ["How chosen", "Pin the day for tests rather than re-record: a re-record is a billed live call and would break again the next morning."],
    ["Gap still open", "Recorded answers are now tied to Oct 31. Moving TEST_TODAY later means re-recording the conversations whose answers carry dates."],
  ],
  when: "PR #15 · Nov 1, 2026 (ca4a539)",
}, { out: here, name: "2-eval-test-day" }));
console.log(files.join("\n"));
