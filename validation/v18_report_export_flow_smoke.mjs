import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const src = fs.readFileSync(path.join(root, "src", "components", "ReportsWorkspace.tsx"), "utf8");
const css = fs.readFileSync(path.join(root, "src", "styles.css"), "utf8");
const checks = [
  ["four-step select run", src.includes("Select run")],
  ["four-step choose preset", src.includes("Choose preset")],
  ["four-step review preview", src.includes("Review figure and table preview")],
  ["four-step export", src.includes("<span>Export</span>")],
  ["export review section", src.includes("report-export-review") && css.includes(".report-export-review")],
  ["export status feedback", src.includes("lastExportMessage") && css.includes(".export-status-feedback")],
  ["reviewer pack interpretation default", src.includes('initialReportPreset === "reviewer_pack"')],
  ["explicit SVG export status", src.includes("SVG diagram")],
];
const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const out = {
  passed: failed.length === 0,
  milestone: "v1_8_results_report_refinement_real_user_testing",
  checks: Object.fromEntries(checks),
  failed,
};
const outPath = path.join(root, "validation", "results", "v18_report_export_flow_smoke.json");
fs.mkdirSync(path.dirname(outPath), { recursive: true });
fs.writeFileSync(outPath, JSON.stringify(out, null, 2));
if (failed.length) {
  console.error(`v1.8 report export flow smoke failed: ${failed.join(", ")}`);
  process.exit(1);
}
console.log(`wrote ${outPath}`);
