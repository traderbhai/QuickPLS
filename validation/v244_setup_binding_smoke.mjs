import fs from "node:fs";

const out = "validation/results/v244_setup_binding_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");

const checks = [
  ["method cards use real applicability engine", adapter.includes("evaluateMethodApplicability") && adapter.includes("adaptMethodCards")],
  ["method applicability rows use real settings/model/data", adapter.includes("methodApplicabilityRows(dataset, nodes, edges, analysisSettings)")],
  ["calculation setup dialog writes real analysis settings", shell.includes("setAnalysisSettings") && shell.includes("settings.bootstrapSamples")],
  ["bootstrap is an add-on setting in calculation dialog", shell.includes("Enable bootstrap inference")],
  ["setup status bar uses current project state", shell.includes("Data: {data.projectSummary.variables ?") && shell.includes("Model: {data.projectSummary.constructs ?")],
  ["stale GreenMobility title is not hardcoded", !shell.includes("GreenMobility Study.qpls")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 setup binding smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v244 setup binding smoke passed: ${out}`);
