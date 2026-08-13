import fs from "node:fs";

const out = "validation/results/v244_data_binding_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");
const dataWorkspace = fs.readFileSync("src/components/DataWorkspace.tsx", "utf8");

const checks = [
  ["data view rows are adapted from active dataset", adapter.includes("dataset.rows.slice") && adapter.includes("dataset.columns.slice")],
  ["empty dataset does not use fallback quality cards", adapter.includes("emptyDataQuality") && !adapter.includes("return fallbackNativePrototypeData.dataQuality")],
  ["selected variable empty state is honest", adapter.includes("noSelectedVariable") && !adapter.includes("return fallbackNativePrototypeData.selectedVariable")],
  ["import dialog receives adapter data", shell.includes("<ImportDataWizard data={data} />")],
  ["import dialog preview uses adapter rows and headers", shell.includes("data.dataHeaders.slice") && shell.includes("data.dataRows.slice")],
  ["data command handlers consume structured payloads", ["detail?.query", "detail?.column", "detail?.outputName", "targetColumns"].every((token) => dataWorkspace.includes(token))],
  ["fake data telemetry is absent", !shell.includes("Dataset: TechAdoption.sav") && !shell.includes("Memory: 182 MB")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 data binding smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v244 data binding smoke passed: ${out}`);
