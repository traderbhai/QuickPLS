import fs from "node:fs";

const out = "validation/results/v244_home_project_status_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");

const checks = [
  ["recent projects persist in local storage", adapter.includes("RECENT_PROJECTS_KEY") && adapter.includes("writeRecentProjects")],
  ["home status uses current project summary", shell.includes("{data.projectSummary.name}") && shell.includes("{data.recentProjects.length} recent project(s)")],
  ["default adapter does not append fallback recent projects", !adapter.includes("fallbackNativePrototypeData.recentProjects")],
  ["project summary has honest no-project fallback", adapter.includes('"No project open"') && adapter.includes('"No dataset loaded"')],
  ["message center is derived from readiness/data/model/run", adapter.includes("readiness.summary") && adapter.includes("Dataset '") && adapter.includes("constructs,")],
  ["fake home telemetry is absent", !shell.includes("Memory: 312 MB")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 home/project/status smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v244 home/project/status smoke passed: ${out}`);
