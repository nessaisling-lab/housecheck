#!/usr/bin/env node
// One-time setup: install Playwright's headless browser into HouseCheck's own labelled folder
// (kit.browsersPath(): `ms-playwright/housecheck`), not the folder other tools share.
//   cd scripts/pr-visuals && npm ci && node setup.mjs
import { execSync } from "node:child_process";
import { browsersPath } from "./kit.mjs";

const path = browsersPath();
console.log(`Installing chromium-headless-shell into ${path}`);
execSync("npx playwright-core install chromium-headless-shell", {
  stdio: "inherit",
  env: { ...process.env, PLAYWRIGHT_BROWSERS_PATH: path },
});
