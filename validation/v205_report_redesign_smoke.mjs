import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const packageJson = JSON.parse(read("package.json"));
const report = read("src/components/ReportsWorkspace.tsx");
const styles = read("src/styles.css");
const topbar = read("src/components/TopBar.tsx");

const requiredReportTokens = [
  "report-v2-workspace",
  "report-v2-hero",
  "report-v2-command-center",
  "Choose export preset",
  "Report package",
  "ready export output",
  "Report export readiness",
  "qpls2-panel",
  "report-v2-preview-shell",
  "report-v2-export-actions",
];

const requiredStyleTokens = [
  ".report-v2-workspace",
  ".report-v2-hero",
  ".report-v2-hero-metrics",
  ".report-v2-command-center",
  ".report-v2-preview-shell",
  ".report-v2-export-actions",
];

const checks = [
  ["package version is 2.0.5", packageJson.version === "2.0.5"],
  ["artifact label is v2.0.5", packageJson.scripts["qpls:release:artifacts"].includes("v2_0_5_report_export_flow_redesign")],
  ["top bar shows v2.0.5 milestone", topbar.includes("v2.0.5 report export redesign")],
  ["Report workspace uses v2 shell classes", report.includes("qpls2-workspace") && report.includes("report-v2-workspace")],
  ["Report component contains v2.0.5 report contract", requiredReportTokens.every((token) => report.includes(token))],
  ["Report styles contain v2.0.5 classes", requiredStyleTokens.every((token) => styles.includes(token))],
  ["Report preserves export actions", ["CSV tables", "HTML report", "XLSX workbook", "Print / PDF", "Model diagram SVG"].every((token) => report.includes(token))],
  ["Report keeps interpretation opt-in", report.includes("Include interpretation notes") && report.includes("numeric by default")],
  ["No user-facing mojibake markers in Report v2 sources", ![report, styles, topbar].join("\n").includes("RÃ")],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const output = {
  milestone: "v2_0_5_report_export_flow_redesign",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v205_report_redesign_smoke.json"), JSON.stringify(output, null, 2));
if (!output.passed) {
  console.error(JSON.stringify(output, null, 2));
  process.exit(1);
}
console.log("v2.0.5 Report redesign smoke passed");
