// Backend diagrams in the system-map style: boxes joined by right-angled lines, with moving dashes
// and packets showing what flows where. Boxes and lines take their color and border from the
// highlight library, so a diagram reads with the same legend as a before/after.

import { execFileSync } from "node:child_process";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { frameHtml, hasFfmpeg, KINDS, launch, SCOPES } from "./kit.mjs";

/** A point on a node's side; `at` is how far along it, 0 to 1 (the middle by default). */
const SIDES = {
  r: (n, at) => [n.x + n.w, n.y + n.h * at],
  l: (n, at) => [n.x, n.y + n.h * at],
  t: (n, at) => [n.x + n.w * at, n.y],
  b: (n, at) => [n.x + n.w * at, n.y + n.h],
};
const across = (s) => s === "r" || s === "l";

/**
 * A right-angled path between two nodes' sides. Across to across turns twice at the middle;
 * across to up/down, or up/down to across, turns once.
 */
export function route(a, b, from = "r", to = "l", fromAt = 0.5, toAt = 0.5) {
  const [x1, y1] = SIDES[from](a, fromAt), [x2, y2] = SIDES[to](b, toAt);
  if (across(from) && across(to)) {
    const mx = Math.round((x1 + x2) / 2);
    return { d: `M${x1},${y1} H${mx} V${y2} H${x2}`, mid: [mx, (y1 + y2) / 2] };
  }
  if (!across(from) && !across(to)) {
    const my = Math.round((y1 + y2) / 2);
    return { d: `M${x1},${y1} V${my} H${x2} V${y2}`, mid: [(x1 + x2) / 2, my] };
  }
  return across(from)
    ? { d: `M${x1},${y1} H${x2} V${y2}`, mid: [(x1 + x2) / 2, y1] }
    : { d: `M${x1},${y1} V${y2} H${x2}`, mid: [x1, (y1 + y2) / 2] };
}

const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;");
const DASH = { dashed: 'stroke-dasharray="10 6"', dotted: 'stroke-dasharray="2 6" stroke-linecap="round"' };

function node(n) {
  const k = KINDS[n.kind ?? "context"], s = SCOPES[n.scope ?? "exact"];
  if (n.kind === "redacted") {
    return `<g><rect x="${n.x}" y="${n.y}" width="${n.w}" height="${n.h}" rx="8" fill="url(#redact)"/>` +
      `<text x="${n.x + n.w / 2}" y="${n.y + n.h / 2 + 5}" text-anchor="middle" class="rl">REDACTED</text></g>`;
  }
  const lines = String(n.sub ?? "").split("\n").filter(Boolean);
  return `<g>` +
    `<rect x="${n.x}" y="${n.y}" width="${n.w}" height="${n.h}" rx="8" fill="#fff" stroke="${k.color}" stroke-width="3" ${DASH[s.css] ?? ""}${s.glow ? ` filter="url(#glow-${n.kind ?? "context"})"` : ""}/>` +
    (s.css === "double" ? `<rect x="${n.x + 5}" y="${n.y + 5}" width="${n.w - 10}" height="${n.h - 10}" rx="5" fill="none" stroke="${k.color}" stroke-width="1.5"/>` : "") +
    `<text x="${n.x + 14}" y="${n.y + 26}" class="t">${esc(n.title)}</text>` +
    lines.map((l, i) => `<text x="${n.x + 14}" y="${n.y + 46 + i * 18}" class="s">${esc(l)}</text>`).join("") +
    `</g>`;
}

function edge(e, byId, animate) {
  const k = KINDS[e.kind ?? "context"];
  const { d, mid } = route(byId[e.from], byId[e.to], e.fromSide, e.toSide, e.fromAt, e.toAt);
  const packets = animate ? Array.from({ length: e.packets ?? 0 }, (_, j) =>
    `<circle r="5" fill="${k.color}"><animateMotion dur="${e.dur ?? 2.4}s" begin="-${(j * (e.dur ?? 2.4)) / (e.packets ?? 1)}s" repeatCount="indefinite" path="${d}"/></circle>`).join("") : "";
  const [lx, ly] = e.labelAt ?? mid;
  return `<path d="${d}" fill="none" stroke="${k.color}" stroke-width="2.5" marker-end="url(#arrow-${e.kind ?? "context"})"` +
    `${e.scope === "dashed" ? ' stroke-dasharray="10 6"' : animate ? ' class="flow"' : ""}/>` + packets +
    (e.label ? `<text x="${lx}" y="${ly - 8}" text-anchor="middle" class="el" fill="${k.color}">${esc(e.label)}</text>` : "");
}

/** The diagram as an SVG string. `nodes` are boxes in SVG pixels; `edges` join them by id. */
export function diagramSvg({ width, height, nodes, edges = [], animate = true }) {
  const byId = Object.fromEntries(nodes.map((n) => [n.id, n]));
  const kinds = [...new Set([...nodes, ...edges].map((x) => x.kind ?? "context"))];
  const defs = kinds.map((kind) => {
    const k = KINDS[kind];
    return `<marker id="arrow-${kind}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="${k.color}"/></marker>` +
      `<filter id="glow-${kind}" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="0" stdDeviation="5" flood-color="${k.color}" flood-opacity=".35"/></filter>`;
  }).join("");
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">` +
    `<style>.t{font:700 15px Poppins,'Segoe UI',sans-serif;fill:#111114}.s{font:400 12.5px Poppins,'Segoe UI',sans-serif;fill:#45454E}` +
    `.el{font:700 12px Poppins,'Segoe UI',sans-serif;paint-order:stroke;stroke:#F4F4F6;stroke-width:5px}` +
    `.rl{font:800 12px Poppins,sans-serif;letter-spacing:.14em;fill:#fff}` +
    `.flow{stroke-dasharray:9 7;animation:flow 1.2s linear infinite}@keyframes flow{to{stroke-dashoffset:-32}}</style>` +
    `<defs>${defs}<pattern id="redact" width="16" height="16" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="16" height="16" fill="#111114"/><rect width="8" height="16" fill="#2B2B31"/></pattern></defs>` +
    `<rect width="${width}" height="${height}" fill="#F4F4F6"/>` +
    edges.map((e) => edge(e, byId, animate)).join("") + nodes.map(node).join("") + `</svg>`;
}

/** A node's box, for a numbered mark over it in the frame. */
export const boxOf = (nodes, id) => {
  const n = nodes.find((x) => x.id === id);
  return { x: n.x, y: n.y, w: n.w, h: n.h };
};

/**
 * Render one diagram frame as a still (`<name>.png`) and, with ffmpeg, an animated GIF of the
 * moving lines (`<name>.gif`). `frame` is what kit.frameHtml takes, with one column whose shot
 * is `{ svg, w, h }`.
 */
export async function renderDiagram(frame, { out, name = "diagram", stills = 16, fps = 8 }) {
  out = resolve(out);
  mkdirSync(out, { recursive: true });
  const html = join(out, "_diagram.html");
  writeFileSync(html, frameHtml({ step: 1, of: 1, ...frame }));
  const browser = await launch();
  const files = [];
  try {
    const page = await browser.newPage({ viewport: { width: 1400, height: 1000 }, deviceScaleFactor: 1 });
    await page.goto(pathToFileURL(html).href);
    await page.evaluate(() => document.fonts.ready);
    const el = page.locator("#f");
    const png = join(out, `${name}.png`);
    await el.screenshot({ path: png, animations: "disabled" });
    files.push(png);
    if (hasFfmpeg()) {
      const dir = join(out, `_${name}-frames`);
      mkdirSync(dir, { recursive: true });
      for (let i = 0; i < stills; i++) {
        await el.screenshot({ path: join(dir, `f${String(i).padStart(3, "0")}.png`) });
        await page.waitForTimeout(1000 / fps);
      }
      const gif = join(out, `${name}.gif`);
      execFileSync("ffmpeg", ["-y", "-loglevel", "error", "-framerate", String(fps), "-i", join(dir, "f%03d.png"),
        "-vf", "split[a][b];[a]palettegen[p];[b][p]paletteuse", "-loop", "0", gif]);
      rmSync(dir, { recursive: true, force: true });
      files.push(gif);
    }
  } finally {
    await browser.close();
    rmSync(html, { force: true });
  }
  return files;
}
