import fs from "node:fs";

const out = "validation/results/v243_run_control_smoke.json";
const source = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");

const checks = [
  ["Run command dispatches production run event", source.includes('dispatchQuickPlsCommand("run-analysis")')],
  ["Cancel Run dispatches production cancel event", source.includes('dispatchQuickPlsCommand("cancel-analysis")')],
  ["Default native run command bar does not expose Pause", !source.includes('label: "Pause"')],
  ["Run screen does not expose fake Pause button", !source.includes(">Pause</button>")],
  ["Calculation setup dialog still launches run", source.includes('dialog === "calculation_setup" ? "Run" : "OK"')],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_43_0_full_native_frontend_backend_wiring", passed, checks: results, note: "Pause/resume is intentionally absent; QuickPLS exposes cancellation only in the native shell." }, null, 2));
if (!passed) {
  console.error(`v243 run control smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v243 run control smoke passed: ${out}`);
