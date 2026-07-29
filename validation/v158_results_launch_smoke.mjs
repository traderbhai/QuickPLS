import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultsDir = path.join(root, "validation", "results");
fs.mkdirSync(resultsDir, { recursive: true });

const runHistory = fs.readFileSync(path.join(root, "src", "components", "RunHistory.tsx"), "utf8");
const interpretation = fs.readFileSync(path.join(root, "src", "domain", "resultInterpretation.ts"), "utf8");
const styles = fs.readFileSync(path.join(root, "src", "styles.css"), "utf8");

const checks = [
  ["results workbench shell exists", runHistory.includes("results-workbench-shell")],
  ["section navigation tiles exist", runHistory.includes("results-section-nav") && runHistory.includes("resultTabHint")],
  ["result tools are grouped", runHistory.includes("results-tool-stack") && runHistory.includes("results-tool-group")],
  ["finding triage exists", runHistory.includes("finding-triage-row") && runHistory.includes("displayFindings")],
  ["mediation table is split", runHistory.includes("Mediation effects summary") && runHistory.includes("Mediation inference") && runHistory.includes("Mediation classification")],
  ["wide table affordance exists", runHistory.includes("result-table-affordance") && runHistory.includes("Wide table: first column stays pinned")],
  ["HTMT uses unique upper-triangle pairs", interpretation.includes("cell.rowIndex < cell.columnIndex")],
  ["results CSS shell exists", styles.includes(".results-workbench-shell") && styles.includes(".results-section-nav")],
  ["findings CSS triage exists", styles.includes(".finding-triage-row") && styles.includes(".finding-more-note")],
  ["table CSS affordance exists", styles.includes(".result-section-meta") && styles.includes(".result-table-affordance")],
  ["no Results mojibake remains", !/[Rr]Â²|fÂ²|QÂ²|Î²/.test(`${runHistory}\n${interpretation}`)],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const payload = {
  passed: failures.length === 0,
  milestone: "v1_5_8_results_workspace_launch_redesign",
  checked_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
  boundary: "frontend-only results presentation; no statistical engine or result payload changes",
};

fs.writeFileSync(path.join(resultsDir, "v158_results_launch_smoke.json"), JSON.stringify(payload, null, 2) + "\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log("v1.5.8 results launch smoke passed");
