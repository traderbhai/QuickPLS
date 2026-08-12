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
const tests = read("src/domain/methodApplicability.test.ts");

const checks = [
  ["applicability states exist", ["recommended", "available", "needs_setup", "not_applicable", "unsupported", "experimental"].every((token) => applicability.includes(token))],
  ["method categories exist", ["Core model estimation", "Inference add-on", "Assessment and diagnostics", "Prediction and segmentation", "Standalone analysis", "Workflow analysis"].every((token) => applicability.includes(token))],
  ["setup v2.11 marker exists", setup.includes('data-method-applicability-polish="v2.11.0"') && setup.includes("setup-v2110-workspace")],
  ["setup availability summary exists", setup.includes("Method availability") && setup.includes("Recommended</dt>") && setup.includes("Blocked or scoped")],
  ["method cards expose status data", ["data-method-id", "data-method-status", "data-method-category", "method-guidance-needed"].every((token) => setup.includes(token))],
  ["why not available copy exists", setup.includes("Why not available yet") && setup.includes("All required checks for the selected method are satisfied")],
  ["bootstrap remains add-on", setup.includes("Bootstrap<input") && applicability.includes("inference_add_on") && tests.includes("keeps bootstrap out of the primary top-bar method list")],
  ["top bar is conservative", topbar.includes("More methods in Setup") && topbar.includes("topBarMethods") && topbar.includes("data-method-applicability-status")],
  ["top bar version label updated", topbar.includes("v2.11.0 method setup guidance")],
  ["data guidance marker exists", data.includes("What can I do with this data?") && data.includes('data-method-applicability-polish="v2.11.0"')],
  ["model guidance marker exists", model.includes("What can I do with this model?") && model.includes('data-method-applicability-polish="v2.11.0"')],
  ["unit coverage includes guidance and R2 text", tests.includes("uses clean R2 text") && tests.includes("dataGuidance") && tests.includes("modelGuidance")],
  ["no R2 mojibake in targeted sources", ![applicability, setup, topbar, data, model].some((source) => source.includes("RÂ²") || source.includes("RÃ"))],
];

const failed = checks.filter(([, passed]) => !passed).map(([name]) => name);
const report = {
  milestone: "v2_11_0_method_applicability_setup_polish",
  passed: failed.length === 0,
  checks: Object.fromEntries(checks),
  failed,
};

fs.writeFileSync(path.join(outDir, "v2110_method_setup_applicability_smoke.json"), JSON.stringify(report, null, 2));
if (!report.passed) {
  console.error(JSON.stringify(report, null, 2));
  process.exit(1);
}
console.log("v2.11 method setup applicability smoke passed");
