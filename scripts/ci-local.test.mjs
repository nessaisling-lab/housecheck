// node --test scripts/ci-local.test.mjs
import assert from "node:assert/strict";
import { test } from "node:test";
import { commitsScanned, parseArgs, secretVerdict } from "./ci-local.mjs";

test("every default check runs; linux only when asked", () => {
  const all = parseArgs([]).selected;
  assert.ok(all.includes("secrets") && all.includes("smoke") && !all.includes("linux"));
  assert.ok(parseArgs(["--linux"]).selected.includes("linux"));
  assert.deepEqual(parseArgs(["fmt", "secrets"]).selected, ["fmt", "secrets"]);
  assert.throws(() => parseArgs(["fmtt"]), /unknown check: fmtt/);
  assert.equal(parseArgs(["--push"]).push, true);
});

test("gitleaks' scanned-commit count is read from its output", () => {
  assert.equal(commitsScanned("12:00PM INF 3 commits scanned.\n12:00PM INF no leaks found"), 3);
  assert.equal(commitsScanned("INF 1 commit scanned."), 1);
  assert.equal(commitsScanned("nothing useful"), null);
});

test("a scan that read no commits is a failure, not a pass", () => {
  // What a worktree mounted into the container produced: exit 0, nothing read.
  const empty = secretVerdict({ code: 0, output: "INF 0 commits scanned.\nINF no leaks found", expected: 3 });
  assert.equal(empty.ok, false);
  assert.match(empty.why, /scanned 0 commits of 3/);
  assert.equal(secretVerdict({ code: 0, output: "INF 3 commits scanned.", expected: 3 }).ok, true);
  assert.equal(secretVerdict({ code: 1, output: "WRN leaks found: 4", expected: 3 }).why, "leaks found");
  assert.equal(secretVerdict({ code: 0, output: "", expected: 0 }).ok, true);
});
