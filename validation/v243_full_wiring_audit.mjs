import fs from "node:fs";

const out = "validation/results/v243_full_wiring_audit.json";
const files = {
  shell: fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8"),
  data: fs.readFileSync("src/components/DataWorkspace.tsx", "utf8"),
  store: fs.readFileSync("src/store.ts", "utf8"),
  packageJson: fs.readFileSync("package.json", "utf8"),
  tauri: fs.readFileSync("src-tauri/tauri.conf.json", "utf8"),
  cargo: fs.readFileSync("Cargo.toml", "utf8"),
  matrix: fs.existsSync("docs/V2_NATIVE_FRONTEND_WIRING_MATRIX.md") ? fs.readFileSync("docs/V2_NATIVE_FRONTEND_WIRING_MATRIX.md", "utf8") : "",
};

const checks = [
  ["package version is 2.43.0", files.packageJson.includes('"version": "2.43.0"')],
  ["Tauri version is 2.43.0", files.tauri.includes('"version": "2.43.0"')],
  ["Cargo workspace version is 2.43.0", files.cargo.includes('version = "2.43.0"')],
  ["artifact label is v2.43.0", files.packageJson.includes("v2_43_0_full_native_frontend_backend_wiring")],
  ["no default Pause command is exposed", !files.shell.includes('label: "Pause"')],
  ["native close project command is present", files.shell.includes("closeProjectNow") && files.store.includes("closeProject")],
  ["native data dialogs are present", ["DataTransformDialog", "DataAddColumnDialog", "DataRecodeDialog", "DataMissingValuesDialog", "DataFilterDialog", "DataSortDialog"].every((token) => files.shell.includes(token))],
  ["data workspace consumes structured command payloads", ["detail?.query", "detail?.column", "detail?.outputName", "targetColumns"].every((token) => files.data.includes(token))],
  ["layout commands are real handlers", ["saveLayout", "resetLayout", "closePane", "restorePane", "toggleStatusBar"].every((token) => files.shell.includes(token))],
  ["help documentation dialog is local/offline", files.shell.includes("QuickPLS Documentation") && files.shell.includes("no internet") === false],
  ["release integrity details are shown", files.shell.includes("ReleaseIntegrityDialog")],
  ["wiring matrix documents v2.43", files.matrix.includes("v2_43_0_full_native_frontend_backend_wiring")],
  ["no SmartPLS equivalence claim", !/SmartPLS equivalen/i.test(files.shell + files.matrix)],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_43_0_full_native_frontend_backend_wiring", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v243 full wiring audit failed: ${out}`);
  process.exit(1);
}
console.log(`v243 full wiring audit passed: ${out}`);
