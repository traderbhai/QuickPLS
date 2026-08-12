import fs from "node:fs";

const out = "validation/results/v244_production_binding_audit.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");
const packageJson = fs.readFileSync("package.json", "utf8");
const registry = fs.readFileSync("validation/development_slices.json", "utf8");
const matrix = fs.existsSync("docs/V2_NATIVE_FRONTEND_WIRING_MATRIX.md") ? fs.readFileSync("docs/V2_NATIVE_FRONTEND_WIRING_MATRIX.md", "utf8") : "";
const doc = fs.existsSync("docs/V2_44_0_NATIVE_UI_PRODUCTION_BINDING_COMPLETION.md") ? fs.readFileSync("docs/V2_44_0_NATIVE_UI_PRODUCTION_BINDING_COMPLETION.md", "utf8") : "";

const forbiddenDefaultShellStrings = [
  "Memory: 312 MB",
  "Memory: 182 MB",
  "CPU: 18%",
  "Dataset: TechAdoption.sav",
  "GreenMobility Study.qpls",
  "Run_20240515_1023",
  "Validated Scope: Full Model",
];

const checks = [
  ["package scripts include v2.44 production binding", packageJson.includes("qpls:v244:production-binding")],
  ["registry contains v2.44 milestone", registry.includes("v2_44_0_native_ui_production_binding_completion")],
  ["roadmap/doc matrix updated for v2.44", matrix.includes("v2_44_0_native_ui_production_binding_completion") && doc.includes("v2_44_0_native_ui_production_binding_completion")],
  ["fallback data is only returned by explicit mockup parity route", adapter.includes("if (mockupParity)") && !adapter.includes("fallbackNativePrototypeData.resultRows") && !adapter.includes("fallbackNativePrototypeData.resultSummary")],
  ["no forbidden static default-shell strings remain", forbiddenDefaultShellStrings.every((token) => !shell.includes(token))],
  ["no SmartPLS equivalence claim", !/SmartPLS equivalen/i.test(shell + adapter + matrix + doc)],
  ["no mojibake RÂ²", !/RÂ²/.test(shell + adapter + matrix + doc)],
  ["run/results/report production data path is present", ["runMonitor", "latestRun", "runExportTables", "buildResultInterpretation"].every((token) => adapter.includes(token))],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 production binding audit failed: ${out}`);
  process.exit(1);
}
console.log(`v244 production binding audit passed: ${out}`);
