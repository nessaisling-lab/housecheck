// The app at any commit, for before/after captures: the frontend built as it was at that commit,
// served from a folder, and a local API it talks to.

import { execFileSync, execSync, spawn } from "node:child_process";
import { createReadStream, existsSync, mkdirSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";

export const ROOT = execFileSync("git", ["rev-parse", "--show-toplevel"], { encoding: "utf8" }).trim();
const CACHE = join(ROOT, "target", "pr-visuals");
const EXE = process.platform === "win32" ? ".exe" : "";

const git = (...a) => execFileSync("git", ["-C", ROOT, ...a], { encoding: "utf8" }).trim();

/**
 * The frontend built as it was at `ref`, or as this checkout is now when `ref` is "WORKTREE",
 * with `apiUrl` as its API. A commit's build is kept and reused. Returns the built folder.
 */
export function buildFrontend(ref, apiUrl) {
  const key = ref === "WORKTREE" ? "worktree" : git("rev-parse", "--short=12", ref);
  const dir = join(CACHE, `frontend-${key}`);
  const dist = join(dir, `dist-${apiUrl.replace(/[^a-z0-9]/gi, "_")}`);
  if (ref !== "WORKTREE" && existsSync(join(dist, "index.html"))) return dist;
  let src = join(ROOT, "frontend");
  if (ref !== "WORKTREE") {
    src = join(dir, "frontend");
    if (!existsSync(join(src, "package.json"))) {
      mkdirSync(dir, { recursive: true });
      git("archive", "--format=tar", "-o", join(dir, "src.tar"), key, "frontend");
      execFileSync("tar", ["-xf", join(dir, "src.tar"), "-C", dir]);
    }
    if (!existsSync(join(src, "node_modules"))) {
      execSync("npm ci --no-audit --no-fund", { cwd: src, stdio: "inherit" });
    }
  }
  execSync(`npx vite build --outDir "${dist}" --emptyOutDir --logLevel warn`, {
    cwd: src,
    stdio: "inherit",
    env: { ...process.env, VITE_API_URL: apiUrl, VITE_ALLOW_DEMO_DATA: "false" },
  });
  return dist;
}

const MIME = {
  ".html": "text/html; charset=utf-8", ".js": "text/javascript", ".css": "text/css", ".json": "application/json",
  ".svg": "image/svg+xml", ".png": "image/png", ".jpg": "image/jpeg", ".woff2": "font/woff2", ".ico": "image/x-icon",
};

/** Serve a built frontend on `port`; any path that is not a file gets index.html, as the app's routes need. */
export function serve(dist, port) {
  const root = resolve(dist);
  const server = createServer((req, res) => {
    let file = normalize(join(root, decodeURIComponent(new URL(req.url, "http://x").pathname)));
    if (!file.startsWith(root)) return res.writeHead(403).end();
    if (!existsSync(file) || statSync(file).isDirectory()) file = join(root, "index.html");
    res.writeHead(200, { "content-type": MIME[extname(file)] ?? "application/octet-stream" });
    createReadStream(file).pipe(res);
  });
  return new Promise((done) =>
    server.listen(port, "127.0.0.1", () => done({ url: `http://127.0.0.1:${port}`, close: () => server.close() })),
  );
}

/**
 * The API on `db` and `port`: this checkout's debug build, built if missing, or the binary named
 * in PR_VISUALS_API_EXE. Resolves once /health answers.
 */
export async function api({ db, port = 18788 }) {
  let exe = process.env.PR_VISUALS_API_EXE || join(ROOT, "target", "debug", `housecheck-api${EXE}`);
  if (!existsSync(exe)) execFileSync("cargo", ["build", "-p", "api"], { cwd: ROOT, stdio: "inherit" });
  const child = spawn(exe, [], {
    env: { ...process.env, HOUSECHECK_DB: resolve(db), HOST: "127.0.0.1", PORT: String(port) },
    stdio: "ignore",
  });
  const url = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 60; i++) {
    if (await fetch(`${url}/health`).then((r) => r.ok, () => false)) {
      return { url, close: () => child.kill() };
    }
    await new Promise((r) => setTimeout(r, 500));
  }
  child.kill();
  throw new Error(`the API on ${db} never answered /health`);
}
