import fs from "node:fs";

const out = "validation/results/v243_export_trust_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const reports = fs.readFileSync("src/components/ReportsWorkspace.tsx", "utf8");
const trust = fs.readFileSync("src/components/TrustCenterWorkspace.tsx", "utf8");

const checks = [
  ["Report SVG export event wired", reports.includes("quickpls:report-export-svg")],
  ["Report table export event wired", reports.includes("quickpls:report-export-tables")],
  ["Report workbook export event wired", reports.includes("quickpls:report-export-workbook")],
  ["Report print event wired", reports.includes("quickpls:report-print")],
  ["Open Folder uses native service", shell.includes("openDefaultExportFolderFromShell")],
  ["Trust refresh evidence event wired", trust.includes("quickpls:trust-refresh-evidence")],
  ["Trust open method doc event wired", trust.includes("quickpls:trust-open-method-doc")],
  ["Trust export evidence index event wired", trust.includes("quickpls:trust-export-evidence-index")],
  ["Checksum detail dialog wired", shell.includes("release_integrity") && shell.includes("ReleaseIntegrityDialog")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_43_0_full_native_frontend_backend_wiring", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v243 export/trust smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v243 export/trust smoke passed: ${out}`);
