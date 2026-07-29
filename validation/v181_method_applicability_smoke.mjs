import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), "utf8");
const outDir = path.join(root, "validation", "results");
fs.mkdirSync(outDir, { recursive: true });

const applicability = read("src/domain/methodApplicability.ts");
const setup = read("src/components/AnalysisCatalog.tsx");
const topbar = read("src/components/TopBar.tsx");
const data = read("src/components/DataWorkspace.tsx");
const model = read("src/components/Explorer.tsx");
const readiness = read("src/domain/analysisReadiness.ts");
const tests = read("src/domain/methodApplicability.test.ts");

const checks = [
  ["applicability states exist", ["recommended", "available", "needs_setup", "not_applicable", "unsupported", "experimental"].every((token) => applicability.includes(token))],
  ["method categories exist", ["Core model estimation", "Inference add-on", "Assessment and diagnostics", "Prediction and segmentation", "Standalone analysis", "Workflow analysis"].every((token) => applicability.includes(token))],
  ["exact action labels exist", ["Import raw dataset", "Assign indicators", "Choose binary outcome", "Select group column", "Use reflective constructs only", "Add at least four indicators", "Choose positive numeric weight column"].every((token) => applicability.includes(token) || tests.includes(token))],
  ["setup recommendation sections exist", ["Recommended for this project", "Available after setup", "Advanced diagnostics", "Standalone analyses", "Show all methods"].every((token) => setup.includes(token))],
  ["bootstrap is an add-on", setup.includes("Bootstrap<input") && applicability.includes("inference_add_on") && tests.includes("keeps bootstrap out of the primary top-bar method list")],
  ["top bar points to setup for broader catalog", topbar.includes("More methods in Setup") && topbar.includes("topBarMethods")],
  ["data guidance panel exists", data.includes("What can I do with this data?") && data.includes("dataGuidance")],
  ["model guidance panel exists", model.includes("What can I do with this model?") && model.includes("modelGuidance")],
  ["readiness uses applicability blocker", readiness.includes("methodApplicabilityFor") && readiness.includes("failedCheck.actionLabel")],
  ["unit scenarios are covered", ["formative", "MICOM/MGA", "logistic", "WPLS", "PCA", "NCA"].every((token) => tests.includes(token))],
  ["no R2 mojibake", ![applicability, setup, topbar, data, model, readiness].some((source) => source.includes("RÂ²") || source.includes("RÃ‚Â²"))],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v1_8_1_method_applicability_guided_setup",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v181_method_applicability_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v1.8.1 method applicability smoke passed");
