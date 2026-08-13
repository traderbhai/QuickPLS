import fs from "node:fs";

const out = "validation/results/v243_data_commands_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const data = fs.readFileSync("src/components/DataWorkspace.tsx", "utf8");

const dialogs = ["data_transform", "data_add_column", "data_recode", "data_missing_values", "data_filter", "data_sort"];
const handlers = [
  ["filter uses structured query", "detail?.query"],
  ["sort uses structured column", "detail?.column ?? selectedColumn"],
  ["add column uses structured name/value", "detail?.name"],
  ["recode uses structured column/from/to", "detail?.from"],
  ["missing values supports selected column", "targetColumns"],
  ["transform supports output name", "detail?.outputName"],
];
const checks = [
  ...dialogs.map((dialog) => [`${dialog} dialog exists`, shell.includes(dialog)]),
  ...handlers.map(([name, token]) => [name, data.includes(token)]),
  ["Data command dialogs dispatch real data events", shell.includes("dispatchNativeDataCommand")],
];

const results = checks.map(([name, passed]) => ({ name, passed: Boolean(passed) }));
const passed = results.every((check) => check.passed);
fs.mkdirSync("validation/results", { recursive: true });
fs.writeFileSync(out, JSON.stringify({ target: "v2_43_0_full_native_frontend_backend_wiring", passed, checks: results }, null, 2));
if (!passed) {
  console.error(`v243 data command smoke failed: ${out}`);
  process.exit(1);
}
console.log(`v243 data command smoke passed: ${out}`);
