import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const exists = (relative) => fs.existsSync(path.join(root, relative));

const protocol = read("docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md");
const active = read("docs/V2_ACTIVE_MILESTONE.md");
const template = JSON.parse(read("validation/templates/real_dataset_issue_register_template.json"));

const checks = [
  ["protocol doc exists", exists("docs/V2_12_0_REAL_DATASET_REVIEW_PROTOCOL.md")],
  ["release notes exist", exists("docs/RELEASE_NOTES_V2_12_0.md")],
  ["template exists", exists("validation/templates/real_dataset_issue_register_template.json")],
  ["active tracker points to v2.12", active.includes("v2_12_0_real_dataset_review_protocol")],
  ["no private data rule is explicit", protocol.includes("No-Private-Data Rule") && protocol.includes("Do not commit")],
  ["manual checklist covers key screens", ["Data Workspace", "Setup Workspace", "Results Workspace", "Report Workspace"].every((token) => protocol.includes(token))],
  ["automated inputs stay fixture based", protocol.includes("Automated Gate Inputs") && protocol.includes("generated synthetic metadata")],
  ["template has privacy booleans", template.privacy?.raw_data_committed === false && template.privacy?.private_project_committed === false && template.privacy?.value_revealing_screenshots_committed === false],
  ["template separates issue types", template.issues?.[0]?.type?.includes("statistical_evidence_gap") && template.issues?.[0]?.type?.includes("method_guidance_gap")],
  ["template covers target screens", ["data", "setup", "results", "report"].every((screen) => template.reviewed_screens?.[screen])],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_12_0_real_dataset_review_protocol",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v2120_real_dataset_protocol_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.12 real dataset protocol smoke passed");
