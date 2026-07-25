/**
 * Live API smoke test — run with:
 *   npx esbuild scripts/live-test.mts --bundle --format=esm --alias:@=./src --outfile=node_modules/.cache/live-test.mjs && node node_modules/.cache/live-test.mjs
 * Hits https://housecheck-nessa.fly.dev (or VITE_API_URL) through the real client code.
 */
import {
  searchAddress,
  getBuilding,
  listBuildings,
  checkRentFairness,
  compareBuildings,
} from "@/lib/api";

let failures = 0;
function check(name: string, cond: boolean, detail?: unknown) {
  console.log(`${cond ? "PASS" : "FAIL"}  ${name}${detail !== undefined ? "  →  " + JSON.stringify(detail)?.slice(0, 160) : ""}`);
  if (!cond) failures++;
}

// 1. search (single-object response normalized to array)
const search = await searchAddress("1024 Gates");
check("searchAddress returns array", Array.isArray(search.data) && search.data.length > 0, search.data[0]);
check("search source is live", search.source === "live", search.source);

// 2. listBuildings (latitude/longitude mapping)
const list = await listBuildings();
check("listBuildings returns buildings", list.data.length > 50, list.data.length);
check("listBuildings maps lat/long + score", typeof list.data[0]?.lat === "number" && typeof list.data[0]?.score === "number", list.data[0]);

// 3. getBuilding (nested building + score.total + sub-scores)
const bbl = list.data[0].bbl;
const card = await getBuilding(bbl);
check("getBuilding source is live", card.source === "live", card.source);
check("getBuilding maps score.total", typeof card.data.score === "number", card.data.score);
check(
  "getBuilding maps sub-scores from raw.score",
  [card.data.sub_scores.condition, card.data.sub_scores.legal, card.data.sub_scores.neighborhood, card.data.sub_scores.accessibility].every((v) => typeof v === "number"),
  card.data.sub_scores
);
check("getBuilding maps floors from num_floors", typeof card.data.floors === "number", card.data.floors);
check("getBuilding maps stabilization object", typeof card.data.stabilization === "string" && typeof card.data.stabilization_message === "string", card.data.stabilization);
check("getBuilding maps violations", typeof card.data.open_violations.c === "number", card.data.open_violations);

// 4. checkRentFairness (monthly_rent body, hud_fmr object)
const rf = await checkRentFairness(bbl, 2900);
check("rentFairness source is live", rf.source === "live", rf.source);
check("rentFairness pct_vs_median is number", typeof rf.data.pct_vs_median === "number", rf.data.pct_vs_median);
check("rentFairness hud_fmr is object with two_br", typeof rf.data.hud_fmr?.two_br === "number", rf.data.hud_fmr);
check("rentFairness verdict string", typeof rf.data.verdict === "string" && rf.data.verdict.length > 0, rf.data.verdict);

// 5. compareBuildings ({buildings:[...]} wrapper)
const cmp = await compareBuildings([list.data[0].bbl, list.data[1].bbl]);
check("compare returns 2 normalized cards", cmp.data.length === 2 && typeof cmp.data[0].score === "number", cmp.data.map((c) => [c.address, c.score]));

console.log(failures === 0 ? "\nALL CHECKS PASSED" : `\n${failures} CHECK(S) FAILED`);
process.exit(failures === 0 ? 0 : 1);
