import fs from "node:fs";

const out = "validation/results/v243_layout_docs_smoke.json";
const source = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");

const checks = [
  ["Reset Layout command wired", source.includes("resetLayout") && source.includes("Workbench layout reset to default")],
  ["Save Layout command wired", source.includes("saveLayout") && source.includes("quickpls.native.layout")],
  ["Close Pane command wired", source.includes("closePane") && source.includes("np-pane-closed")],
  ["Restore Pane command wired", source.includes("restorePane")],
  ["Status Bar toggle persists preference", source.includes("quickpls.native.statusBar") && source.includes("toggleStatusBar")],
  ["Documentation dialog exists", source.includes('documentation: "QuickPLS Documentation"')],
  ["Help menu opens documentation dialog", source.includes('dialog: "documentation"')],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_43_0_full_native_frontend_backend_wiring", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v243 layout/docs smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v243 layout/docs smoke passed: ${out}`);
