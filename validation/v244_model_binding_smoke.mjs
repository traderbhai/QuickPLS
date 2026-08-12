import fs from "node:fs";

const out = "validation/results/v244_model_binding_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");

const checks = [
  ["model constructs come from React Flow nodes", adapter.includes("normalizedConstructPositions(nodes)")],
  ["default model does not use fallback constructs", !adapter.includes("fallbackNativePrototypeData.constructs")],
  ["default model does not use fallback paths", !adapter.includes("fallbackNativePrototypeData.paths")],
  ["model workbench embeds production canvas", shell.includes("<ModelCanvas />")],
  ["model workbench embeds production explorer and inspector", shell.includes("<Explorer />") && shell.includes("<Inspector />")],
  ["add indicator command uses active dataset/nodes", shell.includes("assignIndicator(construct.id, candidate)")],
  ["focus/layout pane state is real shell state", shell.includes("paneClosed") && shell.includes("restorePane")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_44_0_native_ui_production_binding_completion", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v244 model binding smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v244 model binding smoke passed: ${out}`);
