import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const data = read("src/components/DataWorkspace.tsx");
const styles = read("src/styles.css");

const checks = [
  ["v2.24 marker", data.includes("data-v224-data-workbench") && data.includes("data-v224-workbench")],
  ["workbench tabs", ["Data View", "Variable View", "Import History", "Data Quality", "Notes"].every((token) => data.includes(token))],
  ["variable view table", data.includes("function VariableView") && data.includes("Variable metadata table")],
  ["data quality workbench", data.includes("function DataQualityWorkbench") && data.includes("Variable issues")],
  ["method applicability remains in data", data.includes("What can I do with this data?") && data.includes("dataGuidance")],
  ["import modes remain explicit", ["Raw data", "Covariance matrix", "Correlation matrix", "Sample size"].every((token) => data.includes(token))],
  ["data grid remains primary", data.includes("data-v224-data-view") && data.includes("Data preview: first")],
  ["tab styling exists", styles.includes(".data-v224-tabs") && styles.includes(".data-v224-variable-grid") && styles.includes(".data-v224-quality-grid")],
  ["no schema/backend changes in data component", !/F_ml|qpls-estimation|numerical fingerprint|invoke<AnalysisResultEnvelope>/.test(data)],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const result = {
  passed: failures.length === 0,
  milestone: "v2_24_0_data_workbench_redesign",
  generated_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
};

fs.writeFileSync(path.join(outDir, "v2240_data_workbench_smoke.json"), JSON.stringify(result, null, 2));
if (!result.passed) {
  console.error(JSON.stringify(result, null, 2));
  process.exit(1);
}
console.log("v2.24 data workbench smoke passed");
