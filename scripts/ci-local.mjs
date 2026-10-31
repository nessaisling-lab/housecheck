#!/usr/bin/env node
// Local CI: the checks .github/workflows/{ci,security,smoke}.yml run, on this machine, so a
// failure shows up before a push instead of after.
//
//   node scripts/ci-local.mjs                every check
//   node scripts/ci-local.mjs fmt secrets    only these
//   node scripts/ci-local.mjs --list         what each check mirrors
//   node scripts/ci-local.mjs --linux        also fmt, clippy and test in a Linux container
//   node scripts/ci-local.mjs --push         push once everything passed (clean tree only)
//
// Exit code 0 only when every selected check passed. A check whose tool is missing is
// NOT RUN, which also fails the run: a green result here has to mean what green means on
// GitHub. CodeQL is GitHub-only and is listed, never run.
//
// Cargo builds go to target/ci-local with RUSTFLAGS="-D warnings", as in ci.yml, so they don't
// throw away the normal dev build's cache. Each check's full output is in
// target/ci-local/logs/<check>.log.

import { spawn, spawnSync } from "node:child_process";
import { createWriteStream, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

const GITLEAKS_IMAGE = "ghcr.io/gitleaks/gitleaks:v8.24.3"; // the version security.yml runs
const LINUX_IMAGE = "rust:1";
const SMOKE_PORT = "18787"; // not 8787, which a dev API is usually holding
const KNOWN_BBL = "3000010001"; // in the fixture DB, as in smoke.yml
const TIMEOUT_MIN = 45; // a run left while the PC sleeps should end, not hang for hours

const IS_WIN = process.platform === "win32";

/** Each check, the GitHub job it mirrors, and how to run it. */
export const CHECKS = [
  { name: "fmt", mirrors: "ci / rust: Format check" },
  { name: "secrets", mirrors: "security / secret scan (gitleaks, this branch's new commits)" },
  { name: "clippy", mirrors: "ci / rust: Clippy (-D warnings)" },
  { name: "build", mirrors: "ci / rust: Build --locked" },
  { name: "test", mirrors: "ci / rust: Test --locked (includes the agent eval replays)" },
  { name: "smoke", mirrors: "smoke / smoke + stability (fixture DB, API answers, 50 requests)" },
  { name: "frontend", mirrors: "ci / frontend: npm ci, lint, build, test" },
  { name: "npm-audit", mirrors: "security / frontend deps: npm audit --audit-level=high" },
  { name: "cargo-audit", mirrors: "security / rust deps: cargo-audit" },
  { name: "cargo-deny", mirrors: "security / rust deps: cargo deny check advisories bans sources" },
  { name: "linux", mirrors: "ci / rust (ubuntu-latest): fmt, clippy, test in a Linux container", optIn: true },
];
export const GITHUB_ONLY = ["security / CodeQL (js/ts)", "ci / rust and frontend on macOS"];

export function parseArgs(argv) {
  const flags = new Set(argv.filter((a) => a.startsWith("--")));
  const names = argv.filter((a) => !a.startsWith("--"));
  const known = new Set(CHECKS.map((c) => c.name));
  const unknown = names.filter((n) => !known.has(n));
  if (unknown.length) throw new Error(`unknown check: ${unknown.join(", ")} (see --list)`);
  const selected = names.length
    ? names
    : CHECKS.filter((c) => !c.optIn || flags.has(`--${c.name}`)).map((c) => c.name);
  if (flags.has("--linux") && !selected.includes("linux")) selected.push("linux");
  return { selected, list: flags.has("--list"), push: flags.has("--push") };
}

/**
 * The `git push` arguments for `--push`. The upstream is used only when it is this branch's
 * namesake: a branch cut from origin/main tracks origin/main, and a plain push would refuse
 * (or, configured differently, go to main).
 */
export function pushArgs(branch, upstream) {
  return upstream === `origin/${branch}` ? ["push"] : ["push", "-u", "origin", "HEAD"];
}

/** The number gitleaks reports in "N commits scanned.", or null when it says nothing. */
export function commitsScanned(output) {
  const m = /(\d+) commits? scanned/.exec(output);
  return m ? Number(m[1]) : null;
}

/**
 * Whether a gitleaks run passed. A worktree mounted into the container once gave "0 commits
 * scanned" with exit 0, a false pass; so when the branch has commits, a scan that read none
 * fails too.
 */
export function secretVerdict({ code, output, expected }) {
  if (code !== 0) return { ok: false, why: /leaks found/.test(output) ? "leaks found" : `gitleaks exited ${code}` };
  const scanned = commitsScanned(output);
  if (expected > 0 && !scanned) return { ok: false, why: `scanned ${scanned ?? "no"} commits of ${expected}` };
  return { ok: true, why: `${scanned ?? 0} commits scanned, no leaks` };
}

function git(...args) {
  const r = spawnSync("git", args, { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr.trim()}`);
  return r.stdout.trim();
}

function has(cmd, args = ["--version"]) {
  return spawnSync(cmd, args, { encoding: "utf8" }).status === 0;
}

/** Run one command, teeing its output to the log. Resolves to its exit code. */
function run(log, cmd, args, opts = {}) {
  log.write(`\n$ ${cmd} ${args.join(" ")}\n`);
  return new Promise((done) => {
    const spawnOpts = { cwd: opts.cwd, env: { ...process.env, ...opts.env } };
    // npm is a .cmd shim on Windows, which Node runs only through a shell; hand the shell one
    // command line (these arguments have no spaces or quotes).
    const child = opts.shell
      ? spawn([cmd, ...args].join(" "), { ...spawnOpts, shell: true })
      : spawn(cmd, args, spawnOpts);
    const timer = setTimeout(() => {
      log.write(`\n(timed out after ${TIMEOUT_MIN} minutes)\n`);
      kill(child);
    }, TIMEOUT_MIN * 60_000);
    child.stdout.on("data", (d) => log.write(d));
    child.stderr.on("data", (d) => log.write(d));
    child.on("error", (e) => {
      log.write(`${e.message}\n`);
      clearTimeout(timer);
      done(127);
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      done(code ?? 1);
    });
  });
}

function kill(child) {
  if (IS_WIN) spawnSync("taskkill", ["/pid", String(child.pid), "/t", "/f"]);
  else child.kill("SIGKILL");
}

async function steps(log, list) {
  for (const [cmd, args, opts] of list) {
    const code = await run(log, cmd, args, opts);
    if (code !== 0) return { ok: false, why: `${cmd} ${args[0] ?? ""} exited ${code}` };
  }
  return { ok: true };
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** Boot the API on the fixture DB and hold it to what smoke.yml asserts. */
async function smoke(log, root, cargoEnv) {
  const built = await steps(log, [
    ["cargo", ["build", "-p", "api", "-p", "ingest", "--locked"], { cwd: root, env: cargoEnv }],
  ]);
  if (!built.ok) return built;
  const bin = join(root, "target", "ci-local", "debug");
  const db = join(root, "target", "ci-local", "smoke.db");
  rmSync(db, { force: true });
  const exe = (n) => join(bin, IS_WIN ? `${n}.exe` : n);
  if ((await run(log, exe("ingest"), ["--fixture", "--out", db], { cwd: root })) !== 0)
    return { ok: false, why: "fixture DB build failed" };
  const api = spawn(exe("housecheck-api"), [], {
    cwd: root,
    env: { ...process.env, HOUSECHECK_DB: db, HOST: "127.0.0.1", PORT: SMOKE_PORT },
  });
  api.stdout.on("data", (d) => log.write(d));
  api.stderr.on("data", (d) => log.write(d));
  const base = `http://127.0.0.1:${SMOKE_PORT}`;
  try {
    let up = false;
    for (let i = 0; i < 30 && !up; i++) {
      up = await fetch(`${base}/health`).then((r) => r.ok, () => false);
      if (!up) await sleep(1000);
    }
    if (!up) return { ok: false, why: "API never became healthy" };
    const card = await (await fetch(`${base}/building/${KNOWN_BBL}`)).json();
    const score = card?.score?.total;
    log.write(`score.total = ${score}\n`);
    if (!(score >= 0 && score <= 100)) return { ok: false, why: `score ${score} out of range` };
    const rent = await fetch(`${base}/rent-fairness`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ bbl: KNOWN_BBL, monthly_rent: 3000 }),
    }).then((r) => r.json());
    if (!("pct_vs_median" in rent)) return { ok: false, why: "rent-fairness has no pct_vs_median" };
    let bad = 0;
    for (let i = 0; i < 50; i++) {
      const r = await fetch(`${base}/building/${KNOWN_BBL}`);
      await r.arrayBuffer();
      if (r.status !== 200) bad++;
    }
    if (bad) return { ok: false, why: `${bad} of 50 requests failed` };
    return { ok: true, why: `score ${score}, 50 of 50 answered` };
  } finally {
    kill(api);
  }
}

/** Gitleaks over this branch's commits not on origin/main, as the PR scan reads them. */
async function secrets(log, root) {
  if (!has("docker", ["info"])) return { missing: "Docker Desktop, running" };
  const head = git("rev-parse", "HEAD");
  const base = git("merge-base", "origin/main", "HEAD");
  const expected = Number(git("rev-list", "--count", "--no-merges", "--first-parent", `${base}..${head}`));
  if (expected === 0) return { ok: true, why: "no commits beyond origin/main" };
  // Mount the repository that owns the git directory, not the worktree: a worktree's .git is a
  // file naming a Windows path the container cannot follow. Commits and refs are shared, so
  // the SHA range reads the same either way.
  const common = resolve(git("rev-parse", "--path-format=absolute", "--git-common-dir"), "..");
  const args = ["run", "--rm", "-v", `${common.replaceAll("\\", "/")}:/repo`];
  const ignore = join(root, ".gitleaksignore");
  if (existsSync(ignore)) args.push("-v", `${ignore.replaceAll("\\", "/")}:/ignore/.gitleaksignore:ro`);
  args.push(GITLEAKS_IMAGE, "git", "/repo", "--redact", "--no-banner", "-v");
  // This branch's ignore file, not the main checkout's: a branch that records a known
  // non-secret must be judged by its own list, as on GitHub.
  if (existsSync(ignore)) args.push("--gitleaks-ignore-path", "/ignore/.gitleaksignore");
  args.push(`--log-opts=--no-merges --first-parent ${base}..${head}`);
  let output = "";
  const capture = { write: (d) => ((output += d), log.write(d)) };
  const code = await run(capture, "docker", args, { env: { MSYS_NO_PATHCONV: "1" } });
  return secretVerdict({ code, output, expected });
}

async function check(name, log, root) {
  const cargoEnv = { RUSTFLAGS: "-D warnings", CARGO_TARGET_DIR: join(root, "target", "ci-local") };
  const fe = join(root, "frontend");
  const npm = (args) => ["npm", args, { cwd: fe, shell: IS_WIN }];
  switch (name) {
    case "fmt":
      return steps(log, [["cargo", ["fmt", "--all", "--check"], { cwd: root }]]);
    case "secrets":
      return secrets(log, root);
    case "clippy":
      return steps(log, [["cargo", ["clippy", "--workspace", "--all-targets"], { cwd: root, env: cargoEnv }]]);
    case "build":
      return steps(log, [["cargo", ["build", "--workspace", "--locked"], { cwd: root, env: cargoEnv }]]);
    case "test":
      return steps(log, [["cargo", ["test", "--workspace", "--locked"], { cwd: root, env: cargoEnv }]]);
    case "smoke":
      return smoke(log, root, cargoEnv);
    case "frontend":
      if (!existsSync(join(fe, "package.json"))) return { ok: true, why: "no frontend" };
      return steps(log, [
        npm(["ci", "--no-audit", "--no-fund"]),
        npm(["run", "lint", "--if-present"]),
        npm(["run", "build"]),
        npm(["run", "test", "--if-present"]),
      ]);
    case "npm-audit":
      if (!existsSync(join(fe, "package.json"))) return { ok: true, why: "no frontend" };
      return steps(log, [npm(["audit", "--audit-level=high"])]);
    case "cargo-audit":
      if (!has("cargo", ["audit", "--version"])) return { missing: "cargo install --locked cargo-audit" };
      return steps(log, [["cargo", ["audit"], { cwd: root }]]);
    case "cargo-deny":
      if (!has("cargo", ["deny", "--version"])) return { missing: "cargo install --locked cargo-deny" };
      return steps(log, [["cargo", ["deny", "check", "advisories", "bans", "sources"], { cwd: root }]]);
    case "linux": {
      if (!has("docker", ["info"])) return { missing: "Docker Desktop, running" };
      // Named volumes keep the registry and the build between runs; the first run builds
      // everything (DuckDB's C++ included) and takes a while.
      const script = "cargo fmt --all --check && cargo clippy --workspace --all-targets && cargo test --workspace --locked";
      return steps(log, [
        [
          "docker",
          [
            "run", "--rm",
            "-v", `${root.replaceAll("\\", "/")}:/src`,
            "-v", "housecheck-ci-cargo:/usr/local/cargo/registry",
            "-v", "housecheck-ci-target:/target",
            "-e", "CARGO_TARGET_DIR=/target", "-e", "RUSTFLAGS=-D warnings",
            "-w", "/src", LINUX_IMAGE,
            "sh", "-c", `rustup component add rustfmt clippy >/dev/null && ${script}`,
          ],
          { env: { MSYS_NO_PATHCONV: "1" } },
        ],
      ]);
    }
  }
  throw new Error(`no check named ${name}`);
}

function tail(path, n = 30) {
  const lines = readFileSync(path, "utf8").replace(/\r\n?/g, "\n").trimEnd().split("\n");
  return lines.slice(-n).join("\n") + "\n";
}

async function main() {
  let opts;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (e) {
    console.error(e.message);
    process.exit(2);
  }
  if (opts.list) {
    for (const c of CHECKS) console.log(`${c.name.padEnd(12)} ${c.mirrors}${c.optIn ? "  (opt-in)" : ""}`);
    for (const g of GITHUB_ONLY) console.log(`${"(github)".padEnd(12)} ${g}: not run locally`);
    return;
  }
  const root = git("rev-parse", "--show-toplevel");
  const dirty = git("status", "--porcelain") !== "";
  if (opts.push && dirty) {
    console.error("--push needs a clean tree: commit first, so what passed is what goes up.");
    process.exit(2);
  }
  if (dirty) console.log("Note: uncommitted changes are included in this run but not in a push.\n");
  const logs = join(root, "target", "ci-local", "logs");
  mkdirSync(logs, { recursive: true });

  const results = [];
  for (const name of opts.selected) {
    const path = join(logs, `${name}.log`);
    const log = createWriteStream(path);
    process.stdout.write(`${name.padEnd(12)} ...`);
    const t0 = Date.now();
    let r;
    try {
      r = await check(name, log, root);
    } catch (e) {
      r = { ok: false, why: e.message };
    }
    await new Promise((d) => log.end(d));
    const secs = ((Date.now() - t0) / 1000).toFixed(0);
    const status = r.missing ? "NOT RUN" : r.ok ? "pass" : "FAIL";
    const why = r.missing ? `needs ${r.missing}` : r.why ?? "";
    process.stdout.write(`\r${name.padEnd(12)} ${status.padEnd(8)} ${secs.padStart(4)}s  ${why}\n`);
    if (status === "FAIL") process.stdout.write(tail(path).replace(/^/gm, "    | ") + `    (full log: ${path})\n`);
    results.push(status);
  }
  const passed = results.filter((s) => s === "pass").length;
  const green = passed === results.length;
  console.log(`\n${passed} of ${results.length} passed${green ? "" : " -- not green"}. GitHub-only: ${GITHUB_ONLY.join("; ")}.`);
  if (!green) process.exit(1);
  if (opts.push) {
    const up = spawnSync("git", ["rev-parse", "--abbrev-ref", "@{u}"], { encoding: "utf8" });
    const branch = git("branch", "--show-current");
    if (!branch || branch === "main") {
      console.error("--push is for a branch of your own, not main or a detached HEAD.");
      process.exit(2);
    }
    const r = spawnSync("git", pushArgs(branch, up.status === 0 ? up.stdout.trim() : null), { stdio: "inherit" });
    process.exit(r.status ?? 1);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) await main();
