import fs from "node:fs";

const out = "validation/results/v244_run_results_report_binding_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");

const checks = [
  ["run summary uses run monitor lifecycle", adapter.includes("runMonitor.status") && adapter.includes("runMonitor.logs")],
  ["run output rows do not fall back to mock result rows", !adapter.includes("return fallbackNativePrototypeData.resultRows")],
  ["no-run result summary is honest", adapter.includes("noRunResultSummary") && !adapter.includes("return fallbackNativePrototypeData.resultSummary")],
  ["results workbook uses selected real run summary", shell.includes("data.resultSummary.runName") && shell.includes("data.resultSummary.pathRows")],
  ["report summary uses runExportTables and selected/latest run", adapter.includes("runExportTables(latestRun)") && adapter.includes("exportReady: Boolean(latestRun?.result)")],
  ["run log save is wired to actual log rows", shell.includes("saveRunLogFromShell(data.runSummary.logs)")],
  ["fake run telemetry is absent", !shell.includes("CPU: 18%") && !shell.includes("Memory: 32%")],
  ["stale report selected run text is absent", !shell.includes("Run_20240515_1023")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 run/results/report binding smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v244 run/results/report binding smoke passed: ${out}`);
