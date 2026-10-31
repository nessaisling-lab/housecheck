// The highlight library and frame renderer for HouseCheck PR visuals (README.md in this folder).
//
// One fixed vocabulary for every before/after and diagram:
//   color = what kind of thing it is, border = how directly it applies, shape = what it points at.
// Every note starts with its kind's word, so nothing depends on color alone.

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const LIBRARY_VERSION = "v1.1 (v1 + Redacted)";

/**
 * Kinds. PRs use the team set (wrong, fixed, action, new, gap, context) plus redacted.
 * `was` and `improved` are the client-safe pair for portfolio run-throughs only, when
 * Aisling asks for them: they never say "Wrong".
 */
export const KINDS = {
  wrong: { color: "#D92D20", soft: "rgba(217,45,32,.20)", word: "Wrong", means: "what was wrong (before)" },
  fixed: { color: "#12965A", soft: "rgba(18,150,90,.20)", word: "Fixed", means: "the fix (after)" },
  action: { color: "#2563EB", soft: "rgba(37,99,235,.18)", word: "Do", means: "what you do in this step (click, type, upload)" },
  new: { color: "#7C3AED", soft: "rgba(124,58,237,.18)", word: "New", means: "new: it didn't exist before" },
  gap: { color: "#D99A00", soft: "rgba(217,154,0,.22)", word: "Gap", means: "still open, or watch out" },
  context: { color: "#5F6672", soft: "rgba(95,102,114,.16)", word: "Note", means: "unchanged, shown for reference" },
  redacted: { color: "#111114", soft: "rgba(17,17,20,.22)", word: "Redacted", means: "hidden on purpose: private or sensitive, blanked in the page before the capture" },
  was: { color: "#5F6672", soft: "rgba(95,102,114,0)", word: "Before", means: "how it was" },
  improved: { color: "#12965A", soft: "rgba(18,150,90,.20)", word: "Improved", means: "a change that was asked for" },
};

export const SCOPES = {
  exact: { css: "solid", glow: true, means: "the exact thing this note is about" },
  affected: { css: "dashed", glow: false, means: "also affected by the same cause, elsewhere" },
  related: { css: "dotted", glow: false, means: "related, unchanged, for orientation" },
  added: { css: "double", glow: true, means: "a new element" },
  open: { css: "solid", glow: false, hatch: true, name: "hatched", means: "not resolved yet" },
};

export const SHAPES = ["box", "ring", "underline", "arrow"];

const FONT = "Poppins,'Segoe UI',system-ui,sans-serif";

export const CSS = `
*{box-sizing:border-box} body{margin:0;background:#fff;font-family:${FONT};color:#111114}
.frame{width:1400px;padding:26px 30px 22px;display:grid;gap:16px;background:#fff}
.top{display:flex;align-items:baseline;justify-content:space-between;gap:20px}
.top h1{margin:0;font-size:24px;font-weight:800} .top h1 span{color:#1F5F8B}
.step{font-size:14px;font-weight:700;color:#45454E;white-space:nowrap}
.step b{display:inline-block;background:#111114;color:#fff;border-radius:50px;padding:2px 12px;margin-left:6px}
.title{font-size:19px;font-weight:700;margin:-4px 0 0}
.cols{display:grid;grid-template-columns:640px 640px;gap:60px;justify-content:center}
.cols.one{grid-template-columns:1340px}
.col{display:grid;gap:12px;align-content:start}
.tag{justify-self:start;font-size:13px;font-weight:800;letter-spacing:.06em;text-transform:uppercase;padding:4px 14px;border-radius:50px;border:2px solid}
.tag.before{color:#B42318;background:#FDECEA;border-color:#B42318} .tag.after{color:#1E6B3E;background:#E6F3EB;border-color:#1E6B3E}
.tag.only{color:#1F3A5F;background:#E8EEF6;border-color:#1F3A5F}
.shot{position:relative;border:1px solid #D9D9E0;background:#F4F4F6;justify-self:center}
.shot img,.shot svg{display:block}
.why{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:14px}
.why div{border:1px solid #E1E1E6;padding:12px 14px;font-size:14px;line-height:1.45;color:#2B2B31}
.why h3{margin:0 0 6px;font-size:12px;letter-spacing:.07em;text-transform:uppercase;color:#111114}
.why .gap{border-color:#D99A00;background:repeating-linear-gradient(135deg,rgba(217,154,0,.10) 0 6px,transparent 6px 12px)} .why .gap h3{color:#8A6200}
.hl{position:absolute;pointer-events:none}
.hl.box{border-radius:7px} .hl.ring{border-radius:50%}
.hl .num{position:absolute;top:-17px;left:-17px;min-width:32px;height:32px;padding:0 6px;border-radius:16px;color:#fff;
  font:800 14px/32px ${FONT};text-align:center;white-space:nowrap;box-shadow:0 1px 3px rgba(0,0,0,.3)}
.hl.underline{border-left:0!important;border-right:0!important;border-top:0!important;border-radius:0}
.hl.underline .num{top:auto;bottom:-14px;left:auto;right:-30px}
.hl.tr .num{left:auto;right:-17px}
.hl.redacted{background:repeating-linear-gradient(135deg,#111114 0 8px,#2B2B31 8px 16px)}
.hl.redacted .lbl{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;color:#fff;
  font:800 11px/1 ${FONT};letter-spacing:.12em;text-transform:uppercase}
.note{display:grid;grid-template-columns:auto 1fr;gap:10px;font-size:15px;line-height:1.45;color:#2B2B31;align-items:start}
.note .n{min-width:30px;height:28px;padding:0 7px;border-radius:14px;color:#fff;font:800 13px/28px ${FONT};text-align:center;white-space:nowrap}
.note .w{font-weight:800;text-transform:uppercase;letter-spacing:.04em;font-size:13px;margin-right:4px}
.legend{display:flex;flex-wrap:wrap;gap:8px 22px;align-items:center;font-size:13px;color:#45454E;border-top:1px solid #E1E1E6;padding-top:12px}
.legend .sw{display:inline-block;width:30px;height:16px;border-radius:4px;vertical-align:-3px;margin-right:7px}
.legend .when{margin-left:auto;font-weight:600;color:#111114}
`;

const hatch = (c) => `repeating-linear-gradient(135deg, ${c} 0 6px, transparent 6px 12px)`;
const alpha = (soft, a) => soft.replace(/[\d.]+\)$/, `${a})`);

function kindOf(name) {
  const k = KINDS[name];
  if (!k) throw new Error(`unknown kind "${name}"; one of ${Object.keys(KINDS).join(", ")}`);
  return k;
}

/** One highlight over a screenshot. `box` is in the shot's own pixels; `scale` fits it to the frame. */
export function hl({ kind, scope = "exact", shape = "box", box, n, scale = 1, badgeRight = false, badgeOnly = false }) {
  const k = kindOf(kind);
  const s = SCOPES[scope];
  const x = box.x * scale, y = box.y * scale, w = box.w * scale, h = box.h * scale;
  // On a diagram the box already carries its kind's border: only the number goes on top.
  if (badgeOnly) {
    return `<div class="hl" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px"><span class="num" style="background:${k.color}">${n}</span></div>`;
  }
  if (kind === "redacted") {
    return `<div class="hl box redacted" style="left:${x}px;top:${y}px;width:${w}px;height:${h}px">` +
      `<span class="lbl">Redacted</span><span class="num" style="background:${k.color}">${n}</span></div>`;
  }
  const pad = shape === "underline" ? 0 : 6;
  const bw = shape === "underline" ? 5 : scope === "added" ? 7 : 4;
  let style = `left:${x - pad}px;top:${shape === "underline" ? y + h + 1 : y - pad}px;width:${w + pad * 2}px;` +
    `height:${shape === "underline" ? 6 : h + pad * 2}px;border:${bw}px ${s.css} ${k.color};`;
  if (s.glow) style += `box-shadow:0 0 0 6px ${k.soft},0 0 22px ${alpha(k.soft, ".55")};`;
  if (s.hatch) style += `background:${hatch(alpha(k.soft, ".30"))};`;
  return `<div class="hl ${shape}${badgeRight ? " tr" : ""}" style="${style}"><span class="num" style="background:${k.color}">${n}</span></div>`;
}

/** The numbered note under a screenshot, starting with its kind's word. */
export function note({ kind, n, html }) {
  const k = kindOf(kind);
  return `<div class="note"><span class="n" style="background:${k.color}">${n}</span><div><span class="w" style="color:${k.color}">${k.word}</span> ${html}</div></div>`;
}

export function swatch(kind, scope = "exact") {
  const k = kindOf(kind), s = SCOPES[scope];
  if (kind === "redacted") return `<span class="sw" style="background:repeating-linear-gradient(135deg,#111114 0 5px,#2B2B31 5px 10px)"></span>`;
  return `<span class="sw" style="border:3px ${s.css} ${k.color};${s.hatch ? `background:${hatch(k.soft)};` : ""}${s.glow ? `box-shadow:0 0 0 3px ${k.soft};` : ""}"></span>`;
}

/** The legend for one frame: only the kind and scope pairs it uses, then the provenance line. */
export function legend(used, when) {
  const byKind = new Map();
  for (const [k, s] of used) {
    if (!byKind.has(k)) byKind.set(k, []);
    if (!byKind.get(k).includes(s)) byKind.get(k).push(s);
  }
  const items = [...byKind].map(([k, scopes]) => {
    const extra = scopes.filter((s) => s !== "exact" && k !== "redacted")
      .map((s) => `${SCOPES[s].name || SCOPES[s].css} = ${SCOPES[s].means}`);
    return `<span>${scopes.slice(0, k === "redacted" ? 1 : undefined).map((s) => swatch(k, s)).join("")}` +
      `<b style="color:${KINDS[k].color}">${KINDS[k].word}</b> · ${KINDS[k].means}${extra.length ? `; ${extra.join("; ")}` : ""}</span>`;
  });
  return `<div class="legend">${items.join("")}<span class="when">${when}</span></div>`;
}

const TAGS = { before: "Before", after: "After", only: "" };

/**
 * One column: a captured shot with its marks and notes.
 * shot: { src, w, h } from `capture`; marks: [{ n, kind, scope?, shape?, box, html }].
 */
function column({ tag, label, shot, marks = [] }, width) {
  const shown = Math.min(width, shot.w);
  const scale = shown / shot.w;
  const img = shot.svg
    ? `<div style="width:${shown}px">${shot.svg}</div>`
    : `<img src="${shot.src}" style="width:${shown}px">`;
  return `<div class="col"><span class="tag ${tag}">${label ?? TAGS[tag]}</span>` +
    `<div class="shot" style="width:${shown}px">${img}` +
    // Same number, same spot on both sides; a mark can move its badge right with `badgeRight`.
    marks.map((m) => hl({ ...m, scale })).join("") +
    `</div>${marks.map(note).join("")}</div>`;
}

/**
 * One frame's HTML. `cols` is one or two columns; `why` is up to four [heading, text] boxes, and
 * a heading that starts with "Gap" gets the hatched gap style.
 */
export function frameHtml({ pr, heading, step, of, title, cols, why = [], when }) {
  const width = cols.length === 1 ? 1340 : 640;
  const used = cols.flatMap((c) => (c.marks ?? []).map((m) => [m.kind, m.scope ?? "exact"]));
  return `<!doctype html><meta charset=utf-8>` +
    `<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Poppins:wght@400;600;700;800&display=swap">` +
    `<style>${CSS}</style><div class="frame" id="f">` +
    `<div class="top"><h1><span>${pr}</span> · ${heading}</h1>${of > 1 ? `<div class="step">Step<b>${step} of ${of}</b></div>` : ""}</div>` +
    (title ? `<p class="title">${title}</p>` : "") +
    `<div class="cols${cols.length === 1 ? " one" : ""}">${cols.map((c) => column(c, width)).join("")}</div>` +
    (why.length ? `<div class="why">${why.map(([h, t]) => `<div${/^gap/i.test(h) ? ' class="gap"' : ""}><h3>${h}</h3>${t}</div>`).join("")}</div>` : "") +
    legend(used, `${when} · highlight library ${LIBRARY_VERSION}`) +
    `</div>`;
}

/**
 * A headless browser: Playwright's own (`npx playwright-core install chromium-headless-shell`, once),
 * or an installed one named in PR_VISUALS_BROWSER (`msedge`, `chrome`). Installed Edge and Chrome
 * exit at once in headless mode on the machine this was built on (Oct 31, 2026), so they are not
 * the default.
 */
export async function launch() {
  const { chromium } = await import("playwright-core");
  const channel = process.env.PR_VISUALS_BROWSER || undefined;
  try {
    return await chromium.launch(channel ? { channel } : {});
  } catch (e) {
    const fix = channel ? "unset PR_VISUALS_BROWSER to use Playwright's own browser" : "run `npx playwright-core install chromium-headless-shell` in scripts/pr-visuals";
    throw new Error(`could not start the browser (${fix}): ${e.message.split("\n")[0]}`);
  }
}

/**
 * Blank `locators` in the page before a capture: each element's text becomes block characters
 * of the same length and its images are hidden, so no captured file holds the value.
 * Returns their boxes relative to `clip`, for the frame's Redacted marks.
 */
export async function redact(locators, clip = { x: 0, y: 0 }) {
  const boxes = [];
  for (const loc of locators) {
    for (const el of await loc.all()) {
      await el.evaluate((node) => {
        const blank = (s) => s.replace(/\S/g, "█");
        const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
        for (let t = walker.nextNode(); t; t = walker.nextNode()) t.nodeValue = blank(t.nodeValue);
        node.querySelectorAll?.("img,svg,canvas,video").forEach((m) => (m.style.visibility = "hidden"));
        if ("value" in node && typeof node.value === "string") node.value = blank(node.value);
        node.style.background = "#111114";
        node.style.color = "#111114";
      });
      const b = await el.boundingBox();
      if (b) boxes.push({ x: b.x - clip.x, y: b.y - clip.y, w: b.width, h: b.height });
    }
  }
  return boxes;
}

/** Capture `clip` of `page` to `path`, with the boxes of `marks` (named locators) in the shot's pixels. */
export async function capture(page, { path, clip, marks = {}, redactions = [] }) {
  const redacted = await redact(redactions, clip);
  await page.waitForTimeout(300);
  await page.screenshot({ path, clip });
  const boxes = {};
  for (const [k, loc] of Object.entries(marks)) {
    const b = await loc.first().boundingBox();
    if (b) boxes[k] = { x: b.x - clip.x, y: b.y - clip.y, w: b.width, h: b.height };
  }
  return { src: path.replace(/\\/g, "/").split("/").pop(), w: clip.width, h: clip.height, boxes, redacted };
}

/**
 * Render frames to step1.png… in `out`, and a GIF of them when there is more than one and ffmpeg
 * is installed. Returns the files written.
 */
export async function render(frames, { out, gif = "visual.gif", seconds = 4 }) {
  out = resolve(out);
  mkdirSync(out, { recursive: true });
  const browser = await launch();
  const page = await browser.newPage({ viewport: { width: 1400, height: 1000 }, deviceScaleFactor: 1 });
  const files = [], heights = [];
  try {
    for (const [i, f] of frames.entries()) {
      const html = join(out, "_frame.html");
      writeFileSync(html, frameHtml({ ...f, step: i + 1, of: frames.length }));
      await page.goto(pathToFileURL(html).href);
      await page.evaluate(() => document.fonts.ready);
      const el = page.locator("#f");
      heights.push((await el.boundingBox()).height);
      const png = join(out, `step${i + 1}.png`);
      await el.screenshot({ path: png });
      files.push(png);
    }
  } finally {
    await browser.close();
    rmSync(join(out, "_frame.html"), { force: true });
  }
  if (frames.length > 1 && hasFfmpeg()) {
    const H = Math.ceil(Math.max(...heights) / 2) * 2;
    const target = join(out, gif);
    execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-framerate", `1/${seconds}`, "-i", join(out, "step%d.png"),
      "-vf", `pad=1400:${H}:0:0:white,split[a][b];[a]palettegen[p];[b][p]paletteuse`, "-loop", "0", target]);
    files.push(target);
  }
  return files;
}

export function hasFfmpeg() {
  try {
    execFileSync("ffmpeg", ["-version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

