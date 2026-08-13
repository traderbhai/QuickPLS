import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const resultPath = path.join(root, "validation", "results", "v2190_report_trust_settings_smoke.json");

const checks = [];
function check(name, passed, detail) {
  checks.push({ name, passed: Boolean(passed), detail });
}

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

const report = read("src/components/ReportsWorkspace.tsx");
const trust = read("src/components/TrustCenterWorkspace.tsx");
const settings = read("src/components/SettingsWorkspace.tsx");
const styles = read("src/styles.css");

check(
  "Report screen marker",
  report.includes('data-v219-mockup-screen="report"') && report.includes("report-v219-workspace"),
  "Report workspace has the v2.19 mockup marker and alignment class."
);
check(
  "Trust Center screen marker",
  trust.includes('data-v219-mockup-screen="trust"') && trust.includes("trust-v219-workspace"),
  "Trust Center workspace has the v2.19 mockup marker and alignment class."
);
check(
  "Settings screen marker",
  settings.includes('data-v219-mockup-screen="settings"') && settings.includes("settings-v219-workspace"),
  "Settings workspace has the v2.19 mockup marker and alignment class."
);
check(
  "v2.19 CSS hooks",
  [".report-v219-workspace", ".trust-v219-workspace", ".settings-v219-workspace"].every((selector) => styles.includes(selector)),
  "Styles include Report, Trust Center, and Settings mockup alignment hooks."
);
check(
  "Compact report export flow",
  styles.includes(".report-v219-workspace .report-v213-flow") && styles.includes(".report-v219-workspace .report-v213-export-actions"),
  "Report step flow and export actions use v2.19 compact desktop styling."
);
check(
  "Compact trust evidence layout",
  styles.includes(".trust-v219-workspace .trust-v2-confidence-grid") && styles.includes(".trust-v219-workspace .trust-v2-panel"),
  "Trust Center evidence panels use v2.19 compact desktop styling."
);
check(
  "Compact settings forms",
  styles.includes(".settings-v219-workspace .qpls2-settings-grid") && styles.includes(".settings-v219-workspace .qpls2-design-system-grid"),
  "Settings grouped forms use v2.19 compact desktop styling."
);
check(
  "No R squared mojibake",
  ![report, trust, settings, styles].some((source) => source.includes("RÃ") || source.includes("RÂ")),
  "Touched UI sources do not contain R-squared mojibake."
);
check(
  "No SmartPLS equivalence claim",
  ![report, trust, settings, styles].some((source) => /SmartPLS[- ]equivalent|equivalent to SmartPLS/i.test(source)),
  "QuickPLS keeps visual inspiration separate from equivalence claims."
);

const passed = checks.every((entry) => entry.passed);
fs.mkdirSync(path.dirname(resultPath), { recursive: true });
fs.writeFileSync(resultPath, JSON.stringify({ passed, milestone: "v2_19_0_report_trust_settings_mockup_alignment", generatedAt: new Date().toISOString(), checks }, null, 2));

if (!passed) {
  console.error(JSON.stringify({ passed, failed: checks.filter((entry) => !entry.passed) }, null, 2));
  process.exit(1);
}

console.log(`v2.19 Report/Trust/Settings smoke passed: ${resultPath}`);
