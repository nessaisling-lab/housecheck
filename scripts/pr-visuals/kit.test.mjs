// npm test (in scripts/pr-visuals)
import assert from "node:assert/strict";
import { test } from "node:test";
import { route } from "./diagram.mjs";
import { frameHtml, hl, KINDS, launch, legend, note, redact } from "./kit.mjs";
import { markdown, published } from "./pr-media.mjs";

test("every kind has a word and a meaning; Redacted is one of them", () => {
  for (const [name, k] of Object.entries(KINDS)) assert.ok(k.word && k.means && k.color, name);
  assert.equal(KINDS.redacted.word, "Redacted");
});

test("notes start with their kind's word, so nothing depends on color", () => {
  assert.match(note({ kind: "fixed", n: 1, html: "the row" }), />Fixed<\/span> the row/);
  assert.throws(() => note({ kind: "oops", n: 1, html: "" }), /unknown kind "oops"/);
});

test("a highlight is placed at its box times the frame's scale", () => {
  const h = hl({ kind: "wrong", box: { x: 100, y: 50, w: 200, h: 20 }, n: 1, scale: 0.5 });
  assert.match(h, /left:44px;top:19px;width:112px;height:22px/); // 6px padding around the scaled box
  assert.match(hl({ kind: "redacted", box: { x: 0, y: 0, w: 10, h: 10 }, n: 2 }), /Redacted/);
});

test("the legend lists only the kinds a frame uses", () => {
  const l = legend([["fixed", "exact"], ["gap", "open"]], "PR #1");
  assert.match(l, /Fixed/);
  assert.match(l, /hatched = not resolved yet/);
  assert.doesNotMatch(l, /Wrong/);
  const f = frameHtml({ pr: "#1", heading: "h", step: 1, of: 1, when: "w",
    cols: [{ tag: "after", shot: { src: "a.png", w: 100, h: 50 }, marks: [{ n: 1, kind: "new", box: { x: 1, y: 1, w: 5, h: 5 }, html: "x" }] }] });
  assert.match(f, /New<\/b>/);
  assert.doesNotMatch(f, /Step<b>/, "a single frame has no step counter");
});

test("diagram lines turn at right angles", () => {
  const a = { x: 0, y: 0, w: 100, h: 40 }, b = { x: 300, y: 200, w: 100, h: 40 };
  assert.equal(route(a, b).d, "M100,20 H200 V220 H300");
  assert.equal(route(a, b, "b", "t").d, "M50,40 V120 H350 V200");
  assert.equal(route(a, b, "r", "t").d, "M100,20 H350 V200", "across into a top: one turn");
  assert.equal(route(a, b, "b", "l", 0.25).d, "M25,40 V220 H300", "down into a side, off-centre start");
});

test("pr-media publishes frames, GIFs and the script, never raw captures", () => {
  assert.deepEqual(["step1.png", "visual.gif", "visual.mjs", "before-row.png", "after-row.png", "_frame.html"].filter(published),
    ["step1.png", "visual.gif", "visual.mjs"]);
  assert.deepEqual(markdown("https://github.com/nessaisling-lab/housecheck.git", 9, ["visual.mjs", "step1.png"]),
    ["![step1](https://raw.githubusercontent.com/nessaisling-lab/housecheck/pr-media/pr-9/step1.png)"]);
});

test("redaction removes the private text from the page before anything is captured", async (t) => {
  let browser;
  try {
    browser = await launch();
  } catch (e) {
    t.skip(`no headless browser here: ${e.message}`);
    return;
  }
  try {
    const page = await browser.newPage();
    await page.setContent(`<p id="ssn">SSN 123-45-6789, rent $2,450</p><input id="phone" value="212-555-0199"><p id="ok">Shown as is</p>`);
    const boxes = await redact([page.locator("#ssn"), page.locator("#phone")]);
    assert.equal(boxes.length, 2);
    const text = await page.evaluate(() => document.body.innerText + document.querySelector("#phone").value);
    assert.doesNotMatch(text, /\d/, "no digit of the private values is left in the page");
    assert.match(text, /Shown as is/, "text that is not redacted stays");
  } finally {
    await browser.close();
  }
});