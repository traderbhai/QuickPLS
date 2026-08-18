import fs from "node:fs";

const out = "validation/results/v244_data_binding_smoke.json";
const shell = fs.readFileSync("src/v2/NativePrototypeApp.tsx", "utf8");
const adapter = fs.readFileSync("src/v2/nativePrototypeAdapters.ts", "utf8");
const dataWorkspace = fs.readFileSync("src/components/DataWorkspace.tsx", "utf8");
const versionedActions = fs.readFileSync("src/components/dataWorkspaceVersionedActions.ts", "utf8");

const versionedRoutes = [
  'runVersionedAction({ kind: "sort", column: detail?.column ?? (selectedColumn || null), direction })',
  'runVersionedAction({ kind: "add-column", name, value })',
  'runVersionedAction({ kind: "recode", column: recodeColumn, from, to })',
  'runVersionedAction({ kind: "missing-values", column: detail?.column ?? null, markers: detail?.markers ?? "" })',
  'runVersionedAction({ kind: "z-score", column: detail?.column ?? (selectedColumn || null), outputName: detail?.outputName ?? "" })',
];
const removedUnsafeRewrites = ["[...dataset.rows].sort(", "dataset.rows.map((row)", "next[column] = null"];

const checks = [
  ["data view rows are adapted from active dataset", adapter.includes("dataset.rows.slice") && adapter.includes("dataset.columns.slice")],
  ["empty dataset does not use fallback quality cards", adapter.includes("emptyDataQuality") && !adapter.includes("return fallbackNativePrototypeData.dataQuality")],
  ["selected variable empty state is honest", adapter.includes("noSelectedVariable") && !adapter.includes("return fallbackNativePrototypeData.selectedVariable")],
  ["import dialog receives adapter data", shell.includes("<ImportDataWizard data={data} />")],
  ["import dialog preview uses adapter rows and headers", shell.includes("data.dataHeaders.slice") && shell.includes("data.dataRows.slice")],
  ["data commands route every legacy mutator through the immutable gate", versionedRoutes.every((token) => dataWorkspace.includes(token))],
  ["supported recode activates a distinct version with provenance", dataWorkspace.includes("recodeNativeDatasetColumn") && dataWorkspace.includes("commitVersion: commitDatasetVersion") && versionedActions.includes("assertRecodeVersion")],
  ["one-or-many-column missing handling activates one provenance-bound transform version", dataWorkspace.includes("applyNativeDatasetTransformation") && versionedActions.includes("assertTransformationVersion") && versionedActions.includes('kind: "missing_markers"') && versionedActions.includes("sourceColumns.map")],
  ["sort changes only copied preview order", versionedActions.includes('kind: "view-only"') && dataWorkspace.includes("sortDataWorkspaceViewRows") && dataWorkspace.includes('aria-sort={activeSort}')],
  ["z-score activates one exact immutable standardize version", versionedActions.includes('kind: "standardize"') && versionedActions.includes('denominator: "sample_n_minus_one"') && dataWorkspace.includes("applyNativeDatasetTransformation")],
  ["add-column activates one zero-input immutable transform version", versionedActions.includes('kind: "add_column"') && versionedActions.includes("target_column: targetColumn") && versionedActions.includes("assertTransformationVersion")],
  ["browser mutations fail closed and direct row rewrites stay absent", versionedActions.includes("Browser preview cannot create immutable dataset versions") && versionedActions.includes('kind: "blocked"') && removedUnsafeRewrites.every((token) => !dataWorkspace.includes(token))],
  ["fail-closed mutation status is exposed accessibly", dataWorkspace.includes('role="status" aria-live="polite"')],
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
