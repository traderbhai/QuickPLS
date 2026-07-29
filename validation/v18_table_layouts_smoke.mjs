import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const src = fs.readFileSync(path.join(root, "src", "components", "RunHistory.tsx"), "utf8");
const css = fs.readFileSync(path.join(root, "src", "styles.css"), "utf8");
const checks = [
  ["bootstrap estimates section", src.includes('title="Bootstrap estimates"')],
  ["percentile CI section", src.includes('title="Percentile confidence intervals"')],
  ["BCa CI section", src.includes('title="BCa confidence intervals"')],
  ["bootstrap-t CI section", src.includes('title="Bootstrap-t confidence intervals"')],
  ["CI zero status helper", src.includes("function ciZeroStatus")],
  ["HTMT pair table", src.includes("construct pairs") && src.includes("Show full {label} matrix")],
  ["HTMT pair status styling", css.includes(".result-matrix-details td.issue")],
  ["sticky table shell retained", css.includes("position: sticky") && src.includes("sticky")],
];
const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const out = {
  passed: failed.length === 0,
  milestone: "v1_8_results_report_refinement_real_user_testing",
  checks: Object.fromEntries(checks),
  failed,
};
const outPath = path.join(root, "validation", "results", "v18_table_layouts_smoke.json");
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
if (failed.length) {
  console.error(`v1.8 table layout smoke failed: ${failed.join(", ")}`);
  process.exit(1);
}
console.log(`wrote ${outPath}`);
