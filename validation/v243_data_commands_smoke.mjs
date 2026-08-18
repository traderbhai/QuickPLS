import fs from "node:fs";

const out = "validation/results/v243_data_commands_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const data = fs.readFileSync("src/components/DataWorkspace.tsx", "utf8");
const versionedActions = fs.readFileSync("src/components/dataWorkspaceVersionedActions.ts", "utf8");

const dialogs = ["data_transform", "data_add_column", "data_recode", "data_missing_values", "data_filter", "data_sort"];
const versionedRoutes = [
  'runVersionedAction({ kind: "sort", column: detail?.column ?? (selectedColumn || null), direction })',
  'runVersionedAction({ kind: "add-column", name, value })',
  'runVersionedAction({ kind: "recode", column: recodeColumn, from, to })',
  'runVersionedAction({ kind: "missing-values", column: detail?.column ?? null, markers: detail?.markers ?? "" })',
  'runVersionedAction({ kind: "z-score", column: detail?.column ?? (selectedColumn || null), outputName: detail?.outputName ?? "" })',
];
const removedUnsafeRewrites = ["[...dataset.rows].sort(", "dataset.rows.map((row)", "next[column] = null"];
const checks = [
  ...dialogs.map((dialog) => [`${dialog} dialog exists`, shell.includes(dialog)]),
  ["filter uses structured query", data.includes("detail?.query")],
  ["all five legacy mutators route through the immutable action gate", versionedRoutes.every((token) => data.includes(token))],
  ["native recode activates only a returned dataset version", data.includes("recodeNativeDatasetColumn") && data.includes("commitVersion: commitDatasetVersion")],
  ["native recode response must prove child-version provenance", versionedActions.includes("assertRecodeVersion") && versionedActions.includes('version.parentDatasetId !== sourceDatasetId')],
  ["one-or-many-column missing handling uses one immutable transformation version", data.includes("applyNativeDatasetTransformation") && versionedActions.includes("assertTransformationVersion") && versionedActions.includes('kind: "missing_markers"') && versionedActions.includes("sourceColumns.map")],
  ["row sorting is presentation-only and preserves source order", versionedActions.includes('kind: "view-only"') && data.includes("sortDataWorkspaceViewRows") && data.includes("source order is unchanged")],
  ["z-score creates one immutable standardize transformation version", versionedActions.includes('kind: "standardize"') && versionedActions.includes('denominator: "sample_n_minus_one"') && versionedActions.includes("assertTransformationVersion")],
  ["add-column creates one immutable zero-input transformation version", versionedActions.includes('kind: "add_column"') && versionedActions.includes("target_column: targetColumn") && versionedActions.includes("assertTransformationVersion")],
  ["browser mutation attempts fail closed without changing the active dataset", versionedActions.includes("Browser preview cannot create immutable dataset versions") && versionedActions.includes('kind: "blocked"')],
  ["legacy direct row rewrites are absent", removedUnsafeRewrites.every((token) => !data.includes(token))],
  ["fail-closed transformation status is accessible", data.includes('role="status" aria-live="polite"')],
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
