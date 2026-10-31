#!/usr/bin/env node
// Publish a PR's visuals to the `pr-media` branch and print the Markdown to paste into the PR.
//
//   node scripts/pr-visuals/pr-media.mjs <pr-number> <folder> [--push]
//
// `pr-media` holds images only and is never merged. Copied from <folder>: the frames and GIFs
// (*.png, *.gif) and the script that made them (*.mjs), into pr-<number>/. Raw captures named
// before-* or after-* stay local: the frames already show them, marked. Without --push the commit
// stays local, to look at first. Afterwards, drag the same images into the PR comment by hand as a
// backup.

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

/** The Markdown image lines for files on `pr-media`, from the origin URL. */
export function markdown(originUrl, pr, files) {
  const m = /github\.com[/:]([^/]+)\/([^/.]+)(\.git)?$/.exec(originUrl.trim());
  if (!m) throw new Error(`not a GitHub remote: ${originUrl}`);
  return files
    .filter((f) => /\.(png|gif)$/i.test(f))
    .map((f) => `![${f.replace(/\.(png|gif)$/i, "")}](https://raw.githubusercontent.com/${m[1]}/${m[2]}/pr-media/pr-${pr}/${f})`);
}

/** Whether a file in a visuals folder is published. */
export const published = (f) => /\.(png|gif|mjs)$/i.test(f) && !/^(before|after)-/i.test(f) && !f.startsWith("_");

function main() {
  const args = process.argv.slice(2);
  const push = args.includes("--push");
  const [pr, folder] = args.filter((a) => !a.startsWith("--"));
  if (!/^\d+$/.test(pr ?? "") || !folder || !existsSync(folder)) {
    console.error("usage: node scripts/pr-visuals/pr-media.mjs <pr-number> <folder> [--push]");
    process.exit(2);
  }
  const git = (...a) => execFileSync("git", a, { encoding: "utf8" }).trim();
  const root = git("rev-parse", "--show-toplevel");
  const wt = join(root, "target", "pr-visuals", "pr-media-worktree");
  const has = (ref) => {
    try {
      git("rev-parse", "--verify", "--quiet", ref);
      return true;
    } catch {
      return false;
    }
  };
  try {
    git("fetch", "--quiet", "origin", "pr-media");
  } catch {
    // no pr-media on origin yet
  }
  if (existsSync(wt)) git("worktree", "remove", "--force", wt);
  if (has("refs/remotes/origin/pr-media")) git("worktree", "add", "-B", "pr-media", wt, "origin/pr-media");
  else if (has("refs/heads/pr-media")) git("worktree", "add", wt, "pr-media");
  else git("worktree", "add", "--orphan", "-b", "pr-media", wt);
  try {
    const readme = join(wt, "README.md");
    if (!existsSync(readme)) {
      writeFileSync(readme, "# pr-media\n\nImages for HouseCheck pull requests, one folder per PR. Never merged.\n" +
        "Made with `scripts/pr-visuals/` on `main`.\n");
    }
    const dest = join(wt, `pr-${pr}`);
    mkdirSync(dest, { recursive: true });
    const files = readdirSync(folder).filter(published);
    if (!files.length) throw new Error(`nothing to publish in ${folder}`);
    for (const f of files) copyFileSync(join(folder, f), join(dest, f));
    execFileSync("git", ["-C", wt, "add", "README.md", `pr-${pr}`]);
    const staged = execFileSync("git", ["-C", wt, "diff", "--cached", "--name-only"], { encoding: "utf8" }).trim();
    if (staged) execFileSync("git", ["-C", wt, "commit", "-q", "-m", `PR #${pr} visuals`]);
    if (push) execFileSync("git", ["-C", wt, "push", "-q", "origin", "pr-media"], { stdio: "inherit" });
    console.log(staged ? `Committed ${files.length} files to pr-media/pr-${pr}/` : "No changes to commit.");
    console.log(push ? "Pushed. Paste into the PR:\n" : "Not pushed (add --push). The PR will show these once pushed:\n");
    console.log(markdown(git("remote", "get-url", "origin"), pr, files.sort()).join("\n\n"));
    console.log(`\nBackup: drag the same images from ${resolve(folder)} into the PR comment by hand.`);
  } finally {
    git("worktree", "remove", "--force", wt);
  }
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main();
