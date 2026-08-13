import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const packageJson = JSON.parse(read("package.json"));
const runHistory = read("src/components/RunHistory.tsx");
const styles = read("src/styles.css");
const topbar = read("src/components/TopBar.tsx");

const requiredRunHistory = [
  "ResultsV2LensPanel",
  "resultsTabSummary",
  "results-v2-nav-header",
  "Selected run",
  "Active results view",
  "results-v2-lens-panel",
  "results-v2-table-header",
  "results-v2-table-meta",
  "Wide table: first column stays pinned while scrolling",
  "Scope checked",
];

const requiredStyles = [
  ".results-v2-nav-header",
  ".results-v2-lens-panel",
  ".results-v2-lens-copy",
  ".results-v2-lens-metrics",
  ".results-v2-table-header",
  ".results-v2-table-title",
  ".results-v2-table-meta",
  ".results-v2-selected-run-card .run-status",
];

const checks = [
  ["package version is 2.0.4", packageJson.version === "2.0.4"],
  ["artifact label is v2.0.4", packageJson.scripts["qpls:release:artifacts"].includes("v2_0_4_results_table_interpretation_redesign")],
  ["top bar shows v2.0.4 milestone", topbar.includes("v2.0.4 results table redesign")],
  ["Results component contains v2.0.4 workbook/lens/table contract", requiredRunHistory.every((token) => runHistory.includes(token))],
  ["Results styles contain v2.0.4 lens and table classes", requiredStyles.every((token) => styles.includes(token))],
  ["Results tabs use Overview naming", runHistory.includes('{ id: "overview", label: "Overview" }')],
  ["Results tables show row and column metadata", runHistory.includes("displayRows.length") && runHistory.includes("visibleColumnEntries.length")],
  ["Matrix tables show construct and column metadata", runHistory.includes("constructs.length} constructs") && runHistory.includes("constructs.length + 1} columns")],
  ["No user-facing mojibake markers in Results v2 sources", ![runHistory, styles, topbar].join("\n").includes("RÃ")],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_0_4_results_table_interpretation_redesign",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v204_results_redesign_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.0.4 Results redesign smoke passed");
