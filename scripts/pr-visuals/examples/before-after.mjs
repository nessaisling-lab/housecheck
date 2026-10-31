// PR #9's visual: the Health Card's 311 row before (main at fa7e965) and after (02c9de9), both
// built from those commits and talking to a local API on the 250-building pilot database.
//   node scripts/pr-visuals/examples/before-after.mjs <path to housecheck.db>
import { execFileSync } from "node:child_process";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const root = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const lib = (f) => import(pathToFileURL(join(root, "scripts", "pr-visuals", f)).href);
const { api, buildFrontend, serve } = await lib("app.mjs");
const { capture, launch, render } = await lib("kit.mjs");
// Output goes to target/pr-visuals/pr-9/ (git-ignored); publish it with pr-media.mjs.
const here = join(root, "target", "pr-visuals", "pr-9") + "/";
mkdirSync(here, { recursive: true });

const BBL = "3017380001"; // 111 Nostrand Avenue, ZIP 11206
const db = process.argv[2] ?? join(root, "data", "housecheck.db");
const server = await api({ db, port: 18788 });
const browser = await launch();
const shots = {};
try {
  for (const [side, ref, port] of [["before", "fa7e965", 18790], ["after", "02c9de9", 18791]]) {
    const site = await serve(buildFrontend(ref, server.url), port);
    const page = await browser.newPage({ viewport: { width: 1280, height: 1100 }, deviceScaleFactor: 1 });
    await page.addInitScript(() =>
      localStorage.setItem("hc.onboarding.v1", JSON.stringify({ done: true, skipped: true, priorities: [] })));
    await page.goto(`${site.url}/building/${BBL}`);
    await page.getByRole("button", { name: "Building condition details" }).click();
    // The row first, then the list it sits in: the page has other lists with the same labels.
    const row = side === "before"
      ? page.getByText("311 complaints (12 mo)").locator("..")
      : page.getByText(/311 requests per 1,000 homes/).locator("..");
    await row.scrollIntoViewIfNeeded();
    const list = row.locator("xpath=ancestor::dl[1]");
    const b = await list.boundingBox();
    const clip = { x: b.x - 16, y: b.y - 12, width: b.width + 32, height: b.height + 24 };
    shots[side] = await capture(page, { path: join(here, `${side}-row.png`), clip, marks: { row } });
    await page.close();
    site.close();
  }
} finally {
  await browser.close();
  server.close();
}

const when = "PR #9 · the frontends of fa7e965 and 02c9de9, a local API on the 250-building pilot data (Oct 31)";
const files = await render([{
  pr: "#9",
  heading: "The Health Card's 311 row",
  title: "111 Nostrand Avenue · Building condition",
  cols: [
    { tag: "before", label: "Before · main at fa7e965", shot: shots.before, marks: [
      { n: 1, kind: "wrong", box: shots.before.boxes.row,
        html: "<b>311 complaints (12 mo): —</b> on every building. The app read <code>complaints_311</code>, which the API stopped sending when coverage went citywide." },
    ] },
    { tag: "after", label: "After · this PR", shot: shots.after, marks: [
      { n: 1, kind: "fixed", box: shots.after.boxes.row,
        html: "<b>311 requests per 1,000 homes · ZIP 11206: 2,139</b>, with the neighborhood, and a line saying it is the same for every building in the ZIP." },
    ] },
  ],
  why: [
    ["What happened", "PR #6 replaced the nearby-311 count with a ZIP rate per 1,000 homes. The API changed; the Health Card still asked for the old field, so the row went blank."],
    ["When found", "Oct 31, reading the citywide handoff after #6 merged: the row was blank on main."],
    ["How chosen", "The citywide session's label, word for word, so a reader can't take an area figure for this building's own record."],
    ["Gap still open", "The rate is one number for a whole ZIP. 311 by category and at the building level is a later task (TASKS 5)."],
  ],
  when,
}], { out: here, gif: "visual.gif" });
console.log(files.map((f) => f.replace(here, "")).join("\n"));
