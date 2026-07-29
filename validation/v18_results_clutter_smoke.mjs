import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const src = fs.readFileSync(path.join(root, "src", "components", "RunHistory.tsx"), "utf8");
const css = fs.readFileSync(path.join(root, "src", "styles.css"), "utf8");
const checks = [
  ["grouped View menu", src.includes('name="view"')],
  ["grouped Table menu", src.includes('name="table"')],
  ["grouped Export menu", src.includes('name="export"')],
  ["grouped Interpretation menu", src.includes('name="interpretation"')],
  ["sticky run context", src.includes("results-run-context-sticky") && css.includes(".results-run-context-sticky")],
  ["method confidence drawer", src.includes("Why trust this result?")],
  ["finding lanes", src.includes("finding-lane-grid") && css.includes(".finding-lane-grid")],
];
const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const out = {
  passed: failed.length === 0,
  milestone: "v1_8_results_report_refinement_real_user_testing",
  checks: Object.fromEntries(checks),
  failed,
};
const outPath = path.join(root, "validation", "results", "v18_results_clutter_smoke.json");
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
if (failed.length) {
  console.error(`v1.8 Results clutter smoke failed: ${failed.join(", ")}`);
  process.exit(1);
}
console.log(`wrote ${outPath}`);
