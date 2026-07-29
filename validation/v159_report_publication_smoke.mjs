import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultsDir = path.join(root, "validation", "results");
fs.mkdirSync(resultsDir, { recursive: true });

const report = fs.readFileSync(path.join(root, "src", "components", "ReportsWorkspace.tsx"), "utf8");
const styles = fs.readFileSync(path.join(root, "src", "styles.css"), "utf8");
const svg = fs.readFileSync(path.join(root, "src", "domain", "publicationDiagram.ts"), "utf8");

const checks = [
  ["report preset cards replace passive strip", report.includes("report-preset-panel") && report.includes("report-preset-card")],
  ["settings grouped by task", report.includes("report-settings-shell") && report.includes("<h3>Figure</h3>") && report.includes("<h3>Statistics</h3>") && report.includes("<h3>Tables</h3>") && report.includes("<h3>Notes</h3>")],
  ["export actions are explicit buttons", report.includes("report-export-actions") && report.includes("ExportAction") && report.includes("CSV tables") && report.includes("XLSX workbook") && report.includes("Model diagram SVG")],
  ["export disabled reasons are action specific", report.includes("Run an available method before exporting XLSX.") && report.includes("XLSX export requires the native desktop runtime.") && report.includes("Create a model diagram before exporting SVG.")],
  ["preview frame has layout risk guidance", report.includes("preview-guidance") && report.includes("switch to Tidy publication before export") && styles.includes(".publication-preview-frame")],
  ["comparison moved to Results", report.includes("openResultsComparison") && report.includes('selectedTab: "comparison"') && report.includes("Open Results Comparison")],
  ["report no longer renders comparison table inline", !report.includes('aria-label="Run comparison table"')],
  ["new report CSS exists", styles.includes(".report-settings-shell") && styles.includes(".report-export-action") && styles.includes(".report-comparison-link")],
  ["publication SVG protects construct labels", svg.includes("smartpls-latent-label-bg") && svg.includes("labelWidth")],
  ["publication SVG offsets structural labels", svg.includes("automaticLabelOffset") && svg.includes("Math.hypot")],
  ["no report mojibake remains", !/[Rr]Ã‚Â²|fÃ‚Â²|QÃ‚Â²|ÃŽÂ²/.test(`${report}\n${svg}`)],
];

const failures = checks.filter(([, passed]) => !passed).map(([name]) => name);
const payload = {
  passed: failures.length === 0,
  milestone: "v1_5_9_report_publication_workflow_redesign",
  checked_at: new Date().toISOString(),
  checks: Object.fromEntries(checks.map(([name, passed]) => [name, Boolean(passed)])),
  failures,
  boundary: "frontend-only report and publication workflow redesign; no statistical engine or result payload changes",
};

fs.writeFileSync(path.join(resultsDir, "v159_report_publication_smoke.json"), JSON.stringify(payload, null, 2) + "\n");
if (!payload.passed) {
  console.error(JSON.stringify(payload, null, 2));
  process.exit(1);
}
console.log("v1.5.9 report publication smoke passed");
