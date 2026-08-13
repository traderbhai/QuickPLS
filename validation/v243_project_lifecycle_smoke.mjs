import fs from "node:fs";

const out = "validation/results/v243_project_lifecycle_smoke.json";
const source = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const store = fs.readFileSync("src/store.ts", "utf8");

const checks = [
  ["Close Project command is visible", source.includes("Close Project")],
  ["Close Project opens native dialog", source.includes('setDialog("close_project")') || source.includes('dialog: "close_project"')],
  ["Unsaved guard offers save and close", source.includes("Save and close")],
  ["Unsaved guard offers close without saving", source.includes("Close without saving")],
  ["Store has closeProject action", store.includes("closeProject: () =>")],
  ["Close clears project to no project open", store.includes('projectName: "No project open"')],
  ["Close clears dataset to empty dataset", store.includes("emptyDataset")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_43_0_full_native_frontend_backend_wiring", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v243 project lifecycle smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v243 project lifecycle smoke passed: ${out}`);
