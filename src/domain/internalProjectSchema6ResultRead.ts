import {
  canonicalResultDocumentJson,
  validateCanonicalResultDocumentV2,
  type CanonicalResultDocumentV2,
  type CanonicalResultCell,
  type CanonicalResultTable,
} from "./canonicalResultDocumentV2";
import {
  cbsemCfaScoreLmChiSquare1PValueV1,
  cbsemCfaScoreLmNumbersCloseV1,
} from "./internalRecipeV4CbsemExecution";
import { compareUtf8StringsV1 } from "./semModelV4";

const OK_OUTCOME_KEYS = ["status", "value"] as const;
const BLOCKED_OUTCOME_KEYS = ["status", "diagnostic"] as const;
const DIAGNOSTIC_KEYS = ["code", "message", "correctiveAction"] as const;
const SNAPSHOT_KEYS = [
  "schemaVersion",
  "projectId",
  "archivePath",
  "sourceDocumentSha256",
  "canonicalResultDocumentCount",
  "documents",
  "sourceRecheckedUnchanged",
] as const;
const ENTRY_KEYS = [
  "documentId",
  "runId",
  "canonicalDocumentSha256",
  "immutable",
  "canonicalDocumentJson",
  "canonicalDocument",
] as const;
const SHA256_HEX = /^[a-f0-9]{64}$/;
const DATA_FINGERPRINT_V2 = /^v2:([a-f0-9]{64})$/;

export interface InternalProjectSchema6ResultReadRequestV1 {
  surface: "internal_labs" | "standard_exact_cbsem";
  experimentalLabsEnabled: boolean;
  archivePath: string;
  expectedSourceSha256: string;
}

export interface InternalProjectSchema6CanonicalResultEntryV1 {
  documentId: string;
  runId: string;
  canonicalDocumentSha256: string;
  immutable: true;
  canonicalDocumentJson: string;
  canonicalDocument: CanonicalResultDocumentV2;
}

export interface InternalProjectSchema6ResultReadSnapshotV1 {
  schemaVersion: 1;
  projectId: string;
  archivePath: string;
  sourceDocumentSha256: string;
  canonicalResultDocumentCount: number;
  documents: InternalProjectSchema6CanonicalResultEntryV1[];
  sourceRecheckedUnchanged: true;
}

export type InternalProjectSchema6ResultReadOutcomeV1 =
  | { status: "ok"; value: InternalProjectSchema6ResultReadSnapshotV1 }
  | {
    status: "blocked";
    diagnostic: { code: string; message: string; correctiveAction: string };
  };

function fail(path: string, message: string): never {
  throw new Error(`Internal schema-6 canonical-result read ${path}: ${message}`);
}

function recordAt(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail(path, "must be an object");
  }
  return value as Record<string, unknown>;
}

function requireExactKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
  path: string,
): void {
  const actual = Object.keys(value).sort();
  const required = [...expected].sort();
  if (actual.length !== required.length || actual.some((key, index) => key !== required[index])) {
    fail(path, `must contain exactly ${required.join(", ")}`);
  }
}

function nonemptyStringAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) fail(path, "must be a nonempty string");
  return value;
}

function stringAt(value: unknown, path: string): string {
  if (typeof value !== "string") fail(path, "must be a string");
  return value;
}

function sha256At(value: unknown, path: string): string {
  const digest = nonemptyStringAt(value, path);
  if (!SHA256_HEX.test(digest)) fail(path, "must be a lowercase SHA-256 value");
  return digest;
}

function dataFingerprintAt(
  value: unknown,
  path: string,
): { value: string; recordedSha256: string } {
  const fingerprint = nonemptyStringAt(value, path);
  if (SHA256_HEX.test(fingerprint)) {
    return { value: fingerprint, recordedSha256: fingerprint };
  }
  const version2 = DATA_FINGERPRINT_V2.exec(fingerprint);
  if (version2) {
    return { value: fingerprint, recordedSha256: version2[1] };
  }
  fail(path, "must be a bare lowercase SHA-256 or v2:<lowercase SHA-256>");
}

function countAt(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    fail(path, "must be a nonnegative safe integer");
  }
  return value as number;
}

function arrayAt(value: unknown, path: string): unknown[] {
  if (!Array.isArray(value)) fail(path, "must be an array");
  return value;
}

function finiteNumberAt(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(path, "must be a finite number");
  return value;
}

function booleanAt(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") fail(path, "must be a boolean");
  return value;
}

function enumAt<T extends string>(value: unknown, allowed: readonly T[], path: string): T {
  const parsed = nonemptyStringAt(value, path);
  if (!allowed.includes(parsed as T)) fail(path, `must equal one of ${allowed.join(", ")}`);
  return parsed as T;
}

function strictRecordAt(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): Record<string, unknown> {
  const record = recordAt(value, path);
  const allowed = new Set([...required, ...optional]);
  const unknown = Object.keys(record).filter((key) => !allowed.has(key)).sort();
  if (unknown.length > 0) fail(path, `contains unknown keys: ${unknown.join(", ")}`);
  const missing = required.filter((key) => !Object.prototype.hasOwnProperty.call(record, key));
  if (missing.length > 0) fail(path, `is missing required keys: ${missing.join(", ")}`);
  return record;
}

function stringArrayAt(value: unknown, path: string): void {
  arrayAt(value, path).forEach((item, index) => nonemptyStringAt(item, `${path}[${index}]`));
}

function validateCapabilityReferenceShape(value: unknown, path: string): void {
  const reference = strictRecordAt(
    value,
    ["registry_schema_version", "capability_id", "cell_id", "capability_version"],
    [],
    path,
  );
  if (reference.registry_schema_version !== 2) {
    fail(`${path}.registry_schema_version`, "must equal 2");
  }
  nonemptyStringAt(reference.capability_id, `${path}.capability_id`);
  nonemptyStringAt(reference.cell_id, `${path}.cell_id`);
  nonemptyStringAt(reference.capability_version, `${path}.capability_version`);
}

function validateCapabilityReferencesShape(value: unknown, path: string): void {
  arrayAt(value, path).forEach((reference, index) => (
    validateCapabilityReferenceShape(reference, `${path}[${index}]`)
  ));
}

function validateChartDisplayShape(value: unknown, path: string): void {
  const display = strictRecordAt(
    value,
    [],
    ["palette", "show_legend", "show_values", "x_axis_label", "y_axis_label"],
    path,
  );
  if ("palette" in display) stringAt(display.palette, `${path}.palette`);
  if ("show_legend" in display) booleanAt(display.show_legend, `${path}.show_legend`);
  if ("show_values" in display) booleanAt(display.show_values, `${path}.show_values`);
  if ("x_axis_label" in display) stringAt(display.x_axis_label, `${path}.x_axis_label`);
  if ("y_axis_label" in display) stringAt(display.y_axis_label, `${path}.y_axis_label`);
}

function validateCanonicalCellShape(value: unknown, path: string): void {
  const tagged = recordAt(value, path);
  const kind = enumAt(tagged.kind, ["number", "text", "boolean", "missing"] as const, `${path}.kind`);
  if (kind === "number") {
    const cell = strictRecordAt(tagged, ["kind", "value"], ["display"], path);
    finiteNumberAt(cell.value, `${path}.value`);
    if ("display" in cell) stringAt(cell.display, `${path}.display`);
  } else if (kind === "text") {
    const cell = strictRecordAt(tagged, ["kind", "value"], [], path);
    if (typeof cell.value !== "string") fail(`${path}.value`, "must be a string");
  } else if (kind === "boolean") {
    const cell = strictRecordAt(tagged, ["kind", "value"], [], path);
    booleanAt(cell.value, `${path}.value`);
  } else {
    const cell = strictRecordAt(tagged, ["kind", "reason"], ["display"], path);
    enumAt(
      cell.reason,
      ["not_applicable", "not_estimated", "undefined", "withheld"] as const,
      `${path}.reason`,
    );
    if ("display" in cell) stringAt(cell.display, `${path}.display`);
  }
}

function validateCanonicalDocumentWireShape(value: unknown, path: string): void {
  const document = strictRecordAt(
    value,
    [
      "schema_version", "document_id", "title", "provenance", "sections", "tables",
      "charts", "notices", "exclusions", "footnotes", "presentation",
    ],
    ["capability_cells", "general_sem_results"],
    path,
  );
  if (document.schema_version !== 2) fail(`${path}.schema_version`, "must equal 2");
  nonemptyStringAt(document.document_id, `${path}.document_id`);
  nonemptyStringAt(document.title, `${path}.title`);
  if ("capability_cells" in document) {
    validateCapabilityReferencesShape(document.capability_cells, `${path}.capability_cells`);
  }

  const provenance = strictRecordAt(
    document.provenance,
    [
      "run_id", "project_id", "model_id", "model_digest", "dataset_id",
      "dataset_fingerprint", "recipe_id", "recipe_digest", "capability_cell",
      "method_version", "engine_version", "seed", "workers", "started_at", "completed_at",
    ],
    [],
    `${path}.provenance`,
  );
  for (const key of [
    "run_id", "project_id", "model_id", "dataset_id", "recipe_id", "method_version",
    "engine_version", "started_at", "completed_at",
  ]) nonemptyStringAt(provenance[key], `${path}.provenance.${key}`);
  for (const key of ["model_digest", "recipe_digest"]) {
    sha256At(provenance[key], `${path}.provenance.${key}`);
  }
  dataFingerprintAt(
    provenance.dataset_fingerprint,
    `${path}.provenance.dataset_fingerprint`,
  );
  validateCapabilityReferenceShape(
    provenance.capability_cell,
    `${path}.provenance.capability_cell`,
  );
  if (provenance.seed !== null) countAt(provenance.seed, `${path}.provenance.seed`);
  const workers = countAt(provenance.workers, `${path}.provenance.workers`);
  if (workers < 1) fail(`${path}.provenance.workers`, "must be positive");

  arrayAt(document.sections, `${path}.sections`).forEach((value, index) => {
    const sectionPath = `${path}.sections[${index}]`;
    const section = strictRecordAt(
      value,
      ["id", "title", "table_ids", "chart_ids"],
      ["description", "capability_cells"],
      sectionPath,
    );
    nonemptyStringAt(section.id, `${sectionPath}.id`);
    nonemptyStringAt(section.title, `${sectionPath}.title`);
    stringArrayAt(section.table_ids, `${sectionPath}.table_ids`);
    stringArrayAt(section.chart_ids, `${sectionPath}.chart_ids`);
    if ("description" in section) stringAt(section.description, `${sectionPath}.description`);
    if ("capability_cells" in section) validateCapabilityReferencesShape(section.capability_cells, `${sectionPath}.capability_cells`);
  });

  arrayAt(document.tables, `${path}.tables`).forEach((value, index) => {
    const tablePath = `${path}.tables[${index}]`;
    const table = strictRecordAt(
      value,
      ["id", "title", "columns", "rows", "footnote_ids"],
      ["description", "capability_cells"],
      tablePath,
    );
    nonemptyStringAt(table.id, `${tablePath}.id`);
    nonemptyStringAt(table.title, `${tablePath}.title`);
    stringArrayAt(table.footnote_ids, `${tablePath}.footnote_ids`);
    if ("description" in table) stringAt(table.description, `${tablePath}.description`);
    if ("capability_cells" in table) validateCapabilityReferencesShape(table.capability_cells, `${tablePath}.capability_cells`);
    arrayAt(table.columns, `${tablePath}.columns`).forEach((columnValue, columnIndex) => {
      const columnPath = `${tablePath}.columns[${columnIndex}]`;
      const column = strictRecordAt(
        columnValue,
        ["id", "label", "data_type", "description"],
        ["role", "unit", "default_precision"],
        columnPath,
      );
      nonemptyStringAt(column.id, `${columnPath}.id`);
      nonemptyStringAt(column.label, `${columnPath}.label`);
      nonemptyStringAt(column.description, `${columnPath}.description`);
      enumAt(column.data_type, ["number", "text", "boolean"] as const, `${columnPath}.data_type`);
      if ("role" in column) enumAt(column.role, ["label", "estimate", "uncertainty", "decision", "diagnostic", "provenance"] as const, `${columnPath}.role`);
      if ("unit" in column) stringAt(column.unit, `${columnPath}.unit`);
      if ("default_precision" in column) countAt(column.default_precision, `${columnPath}.default_precision`);
    });
    arrayAt(table.rows, `${tablePath}.rows`).forEach((rowValue, rowIndex) => {
      const rowPath = `${tablePath}.rows[${rowIndex}]`;
      const row = strictRecordAt(rowValue, ["id", "cells"], [], rowPath);
      nonemptyStringAt(row.id, `${rowPath}.id`);
      arrayAt(row.cells, `${rowPath}.cells`).forEach((cell, cellIndex) => (
        validateCanonicalCellShape(cell, `${rowPath}.cells[${cellIndex}]`)
      ));
    });
  });

  arrayAt(document.charts, `${path}.charts`).forEach((value, index) => {
    const chartPath = `${path}.charts[${index}]`;
    const chart = strictRecordAt(
      value,
      ["id", "title", "description", "kind", "series", "display"],
      ["source_table_id"],
      chartPath,
    );
    nonemptyStringAt(chart.id, `${chartPath}.id`);
    nonemptyStringAt(chart.title, `${chartPath}.title`);
    nonemptyStringAt(chart.description, `${chartPath}.description`);
    enumAt(chart.kind, ["line", "bar", "scatter", "interval", "heatmap"] as const, `${chartPath}.kind`);
    if ("source_table_id" in chart) stringAt(chart.source_table_id, `${chartPath}.source_table_id`);
    validateChartDisplayShape(chart.display, `${chartPath}.display`);
    arrayAt(chart.series, `${chartPath}.series`).forEach((seriesValue, seriesIndex) => {
      const seriesPath = `${chartPath}.series[${seriesIndex}]`;
      const series = strictRecordAt(seriesValue, ["id", "label", "points"], ["group"], seriesPath);
      nonemptyStringAt(series.id, `${seriesPath}.id`);
      nonemptyStringAt(series.label, `${seriesPath}.label`);
      if ("group" in series) stringAt(series.group, `${seriesPath}.group`);
      arrayAt(series.points, `${seriesPath}.points`).forEach((pointValue, pointIndex) => {
        const pointPath = `${seriesPath}.points[${pointIndex}]`;
        const point = strictRecordAt(pointValue, ["x", "y"], ["lower", "upper", "label"], pointPath);
        if (typeof point.x !== "string") finiteNumberAt(point.x, `${pointPath}.x`);
        finiteNumberAt(point.y, `${pointPath}.y`);
        if ("lower" in point) finiteNumberAt(point.lower, `${pointPath}.lower`);
        if ("upper" in point) finiteNumberAt(point.upper, `${pointPath}.upper`);
        if ("label" in point) stringAt(point.label, `${pointPath}.label`);
      });
    });
  });

  arrayAt(document.notices, `${path}.notices`).forEach((value, index) => {
    const noticePath = `${path}.notices[${index}]`;
    const notice = strictRecordAt(value, ["id", "code", "severity", "message", "section_ids", "table_ids"], [], noticePath);
    nonemptyStringAt(notice.id, `${noticePath}.id`);
    nonemptyStringAt(notice.code, `${noticePath}.code`);
    nonemptyStringAt(notice.message, `${noticePath}.message`);
    enumAt(notice.severity, ["information", "warning", "error"] as const, `${noticePath}.severity`);
    stringArrayAt(notice.section_ids, `${noticePath}.section_ids`);
    stringArrayAt(notice.table_ids, `${noticePath}.table_ids`);
  });

  arrayAt(document.exclusions, `${path}.exclusions`).forEach((value, index) => {
    const exclusionPath = `${path}.exclusions[${index}]`;
    const exclusion = strictRecordAt(value, ["id", "title", "reason"], ["capability_cell"], exclusionPath);
    nonemptyStringAt(exclusion.id, `${exclusionPath}.id`);
    nonemptyStringAt(exclusion.title, `${exclusionPath}.title`);
    nonemptyStringAt(exclusion.reason, `${exclusionPath}.reason`);
    if ("capability_cell" in exclusion) validateCapabilityReferenceShape(exclusion.capability_cell, `${exclusionPath}.capability_cell`);
  });

  arrayAt(document.footnotes, `${path}.footnotes`).forEach((value, index) => {
    const footnotePath = `${path}.footnotes[${index}]`;
    const footnote = strictRecordAt(value, ["id", "text"], ["reference"], footnotePath);
    nonemptyStringAt(footnote.id, `${footnotePath}.id`);
    nonemptyStringAt(footnote.text, `${footnotePath}.text`);
    if ("reference" in footnote) stringAt(footnote.reference, `${footnotePath}.reference`);
  });

  const presentation = strictRecordAt(
    document.presentation,
    ["default_section_id", "default_table_id", "precision", "missing_value_label", "chart_defaults"],
    [],
    `${path}.presentation`,
  );
  if (presentation.default_section_id !== null) nonemptyStringAt(presentation.default_section_id, `${path}.presentation.default_section_id`);
  if (presentation.default_table_id !== null) nonemptyStringAt(presentation.default_table_id, `${path}.presentation.default_table_id`);
  countAt(presentation.precision, `${path}.presentation.precision`);
  nonemptyStringAt(presentation.missing_value_label, `${path}.presentation.missing_value_label`);
  validateChartDisplayShape(presentation.chart_defaults, `${path}.presentation.chart_defaults`);
}

async function sha256Hex(value: string, path: string): Promise<string> {
  return sha256BytesHex(new TextEncoder().encode(value), path);
}

async function sha256BytesHex(value: Uint8Array, path: string): Promise<string> {
  if (!globalThis.crypto?.subtle) fail(path, "cannot be verified on this runtime");
  const digest = await globalThis.crypto.subtle.digest("SHA-256", value);
  return Array.from(
    new Uint8Array(digest),
    (byte) => byte.toString(16).padStart(2, "0"),
  ).join("");
}

const PLS_SCORE_SUMMARY_COLUMNS = [
  "contract_version",
  "maximum_iterations",
  "stop_criterion",
  "estimated_block_count",
  "fixed_block_count",
  "performed_iterations",
  "estimated_block_updates",
] as const;
const PLS_SCORE_WEIGHT_COLUMNS = [
  "construct_id",
  "indicator_id",
  "block_kind",
  "estimated_mode",
  "requested_initialization",
  "normalization",
  "requested_weight",
  "resolved_initial_or_fixed_weight",
  "final_outer_weight",
] as const;
const PLS_FIXED_SCORE_SCALE_COLUMNS = [
  "contract_version",
  "construct_id",
  "indicator_id",
  "pre_standardization_center",
  "pre_standardization_scale",
  "resolved_scoring_coefficient",
  "effective_unit_score_weight",
] as const;
const PLS_POINT_ATTRIBUTION_COLUMNS = [
  "contract_version",
  "preprocessing",
  "indicator_centering",
  "indicator_scaling",
  "outer_weights",
  "outer_loadings",
  "construct_scores",
  "structural_paths",
  "effects",
] as const;
const PLS_ALGORITHM_CONVERGENCE_COLUMNS = [
  "contract_version",
  "weighting_scheme",
  "maximum_iterations",
  "stop_criterion",
  "comparison",
  "performed_iterations",
  "estimated_block_updates",
  "termination_reason",
  "final_max_outer_weight_change",
] as const;
const PLS_ALGORITHM_BLOCK_COLUMNS = [
  "block_ordinal",
  "construct_id",
  "indicator_ordinal",
  "indicator_id",
  "update_rule",
  "initialization",
] as const;
const PLS_NONLINEAR_METHOD_VERSION_V1 = "pls_quadratic_nonlinear_effects_v1";
const PLS_NONLINEAR_ADAPTER_VERSION_V7 = "compiled_recipe_v4_pls_plan_v2_execution_v7";
const PLS_NONLINEAR_TERM_V1 = "centered_squared_construct_score_v1";
const PLS_NONLINEAR_ENGINE_WARNING_V1 = "Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms.";
const PLS_BASE_CAPABILITY_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.pls_algorithm",
  cell_id: "qpls3.pls.algorithm",
  capability_version: "pls_pm_v1",
} as const;
const PLS_NONLINEAR_CAPABILITY_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.nonlinear_relationships",
  cell_id: "qpls3.pls.nonlinear_quadratic",
  capability_version: "pls_quadratic_nonlinear_effects_v1",
} as const;
const PLS_NONLINEAR_TABLE_IDS = [
  "nonlinear_quadratic_diagnostics",
  "nonlinear_equation_fit",
  "nonlinear_method_scope",
] as const;
const PLS_NONLINEAR_DIAGNOSTIC_COLUMNS = [
  "source", "target", "linear_coefficient", "quadratic_coefficient",
  "standard_error", "t_statistic", "p_value_two_sided", "warning",
] as const;
const PLS_NONLINEAR_EQUATION_COLUMNS = [
  "target", "linear_r_squared", "augmented_r_squared", "delta_r_squared",
] as const;
const PLS_NONLINEAR_SCOPE_COLUMNS = ["method_version", "term", "warning"] as const;

interface ArchivedFixedScoreWeight {
  constructId: string;
  indicatorId: string;
  resolved: number;
}
const MISSING_DATA_EXECUTION_COLUMNS = [
  "method_version",
  "policy",
  "archive_validation_scope",
  "raw_replay_performed",
  "source_dataset_id",
  "source_dataset_fingerprint",
  "source_row_count",
  "retained_row_count",
  "omitted_row_count",
  "modeled_variable_count",
  "imputed_cell_count",
  "affected_case_count",
  "variable_warning_threshold",
  "high_missingness_threshold",
  "missingness_sha256",
  "completed_matrix_sha256",
  "receipt_sha256",
] as const;
const MEAN_REPLACEMENT_VARIABLE_COLUMNS = [
  "variable_order",
  "variable_id",
  "source_column",
  "canonical_missing_markers_json",
  "observed_count",
  "missing_count",
  "replacement_mean",
  "missing_fraction",
  "warning_level",
] as const;
const MEAN_REPLACEMENT_CELL_COLUMNS = [
  "row_index_zero_based",
  "variable_order",
  "variable_id",
  "source_column",
  "replacement_mean",
  "case_missing_fraction",
  "high_missingness_warning",
] as const;
const CBSEM_HISTORICAL_FIT_COLUMNS = [
  "chi_square",
  "degrees_of_freedom",
  "p_value",
  "cfi",
  "tli",
  "rmsea",
  "srmr",
  "aic",
  "bic",
] as const;
const CBSEM_RMSEA_INTERVAL_METHOD_VERSION =
  "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1";
const CBSEM_RMSEA_INTERVAL_COLUMNS = [
  "fit_method_version",
  "chi_square",
  "degrees_of_freedom",
  "p_value",
  "cfi",
  "tli",
  "rmsea",
  "rmsea_interval_method_version",
  "rmsea_interval_confidence_level",
  "rmsea_ci_lower",
  "rmsea_ci_upper",
  "srmr",
  "aic",
  "bic",
] as const;
const CBSEM_SCORE_LM_COLUMNS = [
  "method_version", "scope", "parameter_id", "kind", "lhs", "rhs", "status",
  "score", "efficient_score", "candidate_information", "efficient_information",
  "modification_index", "expected_parameter_change", "degrees_of_freedom", "p_value",
  "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_ADAPTER_V9 = "compiled_recipe_v4_cbsem_plan_v2_execution_v9";
const CBSEM_EXACT_BOOTSTRAP_ADAPTER_V10 = "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
const CBSEM_EXACT_BOOTSTRAP_ADAPTER_V11 = "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
const CBSEM_EXACT_BOOTSTRAP_ADAPTER_V12 = "compiled_recipe_v4_cbsem_plan_v2_execution_v12";
const CBSEM_EXACT_BOOTSTRAP_METHOD_V1 = "cbsem_exact_case_bootstrap_v1";
const CBSEM_EXACT_BOOTSTRAP_STREAM_V1 = "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1";
const CBSEM_EXACT_BOOTSTRAP_SCHEDULE_DIGEST_V1 =
  "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1";
const CBSEM_EXACT_BOOTSTRAP_TABLE_IDS = [
  "exact_case_bootstrap_summary",
  "exact_case_bootstrap_parameter_intervals",
  "exact_case_bootstrap_successful_refits",
  "exact_case_bootstrap_failures",
] as const;
const CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS = [
  "method_version", "estimator_method_version", "source_dataset_id",
  "source_dataset_fingerprint", "outer_recipe_analytical_identity_sha256",
  "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256",
  "model_scientific_sha256", "complete_case_sample_size",
  "complete_case_universe_digest_method", "complete_case_universe_sha256",
  "covariance_denominator", "sample_indices_digest_method",
  "sampling_positions_digest_method", "interval_method", "confidence_level",
  "requested_replicates", "attempted_refits", "usable_replicates", "failed_replicates",
  "minimum_usable_fraction", "minimum_usable_replicates", "seed_decimal", "stream_token",
  "retry_policy", "max_attempts_per_replicate", "parameter_ids_json", "inference_status",
  "unavailable_reason_code", "unavailable_message", "archive_validation_scope",
] as const;
const CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS = [
  "parameter_id", "original", "bootstrap_mean", "bias", "standard_error",
  "percentile_lower", "percentile_upper", "usable_replicates",
] as const;
const CBSEM_EXACT_BOOTSTRAP_SUCCESS_COLUMNS = [
  "replicate_index", "sampling_positions_sha256", "sample_indices_sha256",
  "parameter_estimates_json", "iterations", "objective", "gradient_norm",
] as const;
const CBSEM_EXACT_BOOTSTRAP_FAILURE_COLUMNS = [
  "replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "kind", "message",
] as const;
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID = "exact_case_bootstrap_hypothesis_tests";
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID = "bootstrap_hypothesis_tests";
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS = [
  "method_version", "null_hypothesis", "statistic", "tie_policy", "probability_method",
  "decision_rule", "selected_test_tail", "null_value", "significance_level",
  "usable_replicates", "inference_status", "global_unavailable_reason_code",
  "global_unavailable_message", "parameter_id", "parameter_status", "point_estimate",
  "two_sided_exceedances", "greater_or_equal_exceedances", "less_or_equal_exceedances",
  "p_value_two_sided", "p_value_greater", "p_value_less", "selected_exceedances",
  "selected_p_value", "reject_null", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID = "bootstrap_studentized_inference";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS = [
  "exact_case_bootstrap_studentized_summary",
  "exact_case_bootstrap_studentized_point_standard_errors",
  "exact_case_bootstrap_studentized_parameter_intervals",
  "exact_case_bootstrap_studentized_refit_standard_errors",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS = [
  "method_version", "standard_error_method_version", "expected_information_method", "pivot_method",
  "quantile_method", "interval_method", "archive_validation_scope", "confidence_level",
  "minimum_usable_fraction", "minimum_usable_replicates", "studentized_usable_replicates",
  "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS = [
  "method_version", "parameter_id", "status", "information_method", "standard_error", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS = [
  "parameter_id", "status", "point_estimate", "point_standard_error", "lower_pivot_quantile",
  "upper_pivot_quantile", "interval_lower", "interval_upper", "usable_replicates", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS = [
  "replicate_index", "status", "information_method", "standard_errors_json", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_METHOD_V1 =
  "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1";
const CBSEM_EXACT_BOOTSTRAP_STANDARD_ERROR_METHOD_V1 =
  "cbsem_exact_case_bootstrap_refit_standard_errors_v1";
const CBSEM_EXACT_BOOTSTRAP_EXPECTED_INFORMATION_METHOD_V1 =
  "cbsem_ml_expected_information_delta_method_v1";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1 =
  "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1 = "percentile_type7_v1";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1 =
  "reversed_type7_studentized_pivot_v1";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_ARCHIVE_SCOPE_V1 =
  "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1";
const CBSEM_EXACT_BOOTSTRAP_STANDARD_ERROR_UNAVAILABLE_REASONS = new Set([
  "singular_information", "information_not_positive_definite",
  "invalid_information_variance_or_standard_error", "derivative_unavailable",
  "numerical_information_failure",
]);
const CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID = "bootstrap_bca_inference";
const CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS = [
  "exact_case_bootstrap_bca_summary",
  "exact_case_bootstrap_bca_parameter_intervals",
  "exact_case_bootstrap_bca_successful_delete_one_refits",
  "exact_case_bootstrap_bca_failures",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS = [
  "method_version", "base_bootstrap_method_version", "outer_recipe_analytical_identity_sha256",
  "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256",
  "model_scientific_sha256", "delete_one_refit_method_version",
  "delete_one_sampling_positions_digest_method", "delete_one_sample_indices_digest_method",
  "bias_correction_method", "acceleration_method", "adjusted_probability_method", "quantile_method",
  "retry_policy", "archive_validation_scope", "confidence_level", "bootstrap_usable_replicates",
  "minimum_bootstrap_usable_replicates", "delete_one_case_count", "successful_delete_one_refits",
  "failed_delete_one_refits", "parameter_ids_json", "inference_status", "unavailable_reason_code",
  "unavailable_message",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS = [
  "parameter_id", "status", "point_estimate", "bias_correction", "acceleration",
  "adjusted_lower_probability", "adjusted_upper_probability", "interval_lower", "interval_upper",
  "usable_replicates", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS = [
  "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
  "retained_sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS = [
  "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
  "retained_sample_indices_sha256", "kind", "message",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE =
  "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1";
const CBSEM_EXACT_BOOTSTRAP_BCA_UNAVAILABLE_REASONS = new Set([
  "base_inference_unavailable", "incomplete_delete_one_ledger",
  "bias_correction_probability_at_boundary", "degenerate_jackknife_acceleration",
  "nonfinite_jackknife_arithmetic", "singular_acceleration_adjustment",
  "invalid_adjusted_probability", "adjusted_probability_order_invalid",
  "nonfinite_or_reversed_interval",
]);

function exactTableColumns(
  table: CanonicalResultTable,
  expected: readonly string[],
  path: string,
): void {
  if (
    table.columns.length !== expected.length
    || table.columns.some((column, index) => column.id !== expected[index])
    || table.rows.some((row) => row.cells.length !== expected.length)
  ) {
    fail(path, "has a drifted column contract");
  }
}

function canonicalTextCell(cell: CanonicalResultCell, path: string): string {
  if (cell.kind !== "text") fail(path, "must be a text cell");
  return cell.value;
}

function canonicalOptionalTextCell(cell: CanonicalResultCell, path: string): string | null {
  if (cell.kind === "text") return cell.value;
  if (cell.kind === "missing" && cell.reason === "not_applicable" && cell.display === undefined) {
    return null;
  }
  fail(path, "must be text or an undisplayed not_applicable cell");
}

function canonicalNumberCell(cell: CanonicalResultCell, path: string): number {
  if (cell.kind !== "number" || !Number.isFinite(cell.value)) {
    fail(path, "must be a finite number cell");
  }
  return cell.value;
}

function canonicalCountCell(cell: CanonicalResultCell, path: string): number {
  const value = canonicalNumberCell(cell, path);
  if (!Number.isSafeInteger(value) || value < 0) fail(path, "must be a nonnegative safe integer");
  return value;
}

function canonicalOptionalNumberCell(cell: CanonicalResultCell, path: string): number | null {
  if (cell.kind === "number" && Number.isFinite(cell.value)) return cell.value;
  if (cell.kind === "missing" && cell.reason === "not_estimated" && cell.display === undefined) {
    return null;
  }
  fail(path, "must be a finite number or an undisplayed not_estimated cell");
}

function canonicalBooleanCell(cell: CanonicalResultCell, path: string): boolean {
  if (cell.kind !== "boolean") fail(path, "must be a boolean cell");
  return cell.value;
}

function positiveCanonicalScalar(weights: ReadonlyArray<readonly [number, number]>): boolean {
  const anchor = weights.find(([requested]) => requested !== 0);
  if (!anchor) return false;
  const scale = anchor[1] / anchor[0];
  return Number.isFinite(scale) && scale > 0 && weights.every(([requested, resolved]) => {
    const expected = requested * scale;
    return Math.abs(resolved - expected)
      <= 1e-12 * Math.max(Math.abs(resolved), Math.abs(expected), 1);
  });
}

function canonicalWeightsMatchNormalization(
  normalization: string | null,
  weights: ReadonlyArray<readonly [number, number]>,
): boolean {
  if (normalization === "none") {
    return weights.every(([requested, resolved]) => Object.is(requested, resolved));
  }
  if (normalization === "sum_to_one") {
    const sum = weights.reduce((total, [requested]) => total + requested, 0);
    return Number.isFinite(sum) && sum !== 0 && weights.every(([requested, resolved]) => (
      Object.is(requested / sum, resolved)
    ));
  }
  return normalization === "unit_variance" || normalization === null
    ? positiveCanonicalScalar(weights)
    : false;
}

function validateArchivedFixedScoreScaleReceipt(
  document: CanonicalResultDocumentV2,
  fixedWeights: readonly ArchivedFixedScoreWeight[],
  currentAdapter: boolean,
  path: string,
): void {
  const table = document.tables.find((candidate) => candidate.id === "fixed_score_scale_receipt");
  if (fixedWeights.length === 0) {
    if (table) fail(path, "fixed_score_scale_receipt exists without fixed score rows");
    return;
  }
  if (!table) {
    if (currentAdapter) fail(path, "current fixed-score adapter omitted fixed_score_scale_receipt");
    return;
  }
  const references = document.sections.flatMap((section) => section.table_ids
    .filter((tableId) => tableId === "fixed_score_scale_receipt")
    .map(() => section.id));
  if (references.length !== 1 || references[0] !== "run_details") {
    fail(path, "fixed_score_scale_receipt must belong exactly once to run_details");
  }
  exactTableColumns(table, PLS_FIXED_SCORE_SCALE_COLUMNS, `${path}.fixed_score_scale_receipt`);
  if (table.rows.length !== fixedWeights.length) {
    fail(`${path}.fixed_score_scale_receipt`, "must exactly cover fixed score rows");
  }
  const blockScales = new Map<string, readonly [number, number]>();
  table.rows.forEach((row, index) => {
    const rowPath = `${path}.fixed_score_scale_receipt.rows[${index}]`;
    const expected = fixedWeights[index];
    if (row.id !== `fixed_score_scale_${index.toString().padStart(4, "0")}`) {
      fail(`${rowPath}.id`, "is non-canonical");
    }
    if (
      canonicalTextCell(row.cells[0], `${rowPath}.contract_version`)
        !== "pls_fixed_score_scale_receipt_v1"
      || canonicalTextCell(row.cells[1], `${rowPath}.construct_id`) !== expected.constructId
      || canonicalTextCell(row.cells[2], `${rowPath}.indicator_id`) !== expected.indicatorId
    ) {
      fail(rowPath, "differs from fixed score-execution order or identity");
    }
    const center = canonicalNumberCell(row.cells[3], `${rowPath}.pre_standardization_center`);
    const scale = canonicalNumberCell(row.cells[4], `${rowPath}.pre_standardization_scale`);
    const coefficient = canonicalNumberCell(row.cells[5], `${rowPath}.resolved_scoring_coefficient`);
    const effective = canonicalNumberCell(row.cells[6], `${rowPath}.effective_unit_score_weight`);
    const prior = blockScales.get(expected.constructId);
    if (
      scale <= Number.EPSILON
      || (prior && (!Object.is(prior[0], center) || !Object.is(prior[1], scale)))
      || !Object.is(coefficient, expected.resolved)
      || !Object.is(effective, coefficient / scale)
    ) {
      fail(rowPath, "has tampered center, scale, coefficient, or effective weight arithmetic");
    }
    blockScales.set(expected.constructId, [center, scale]);
  });
}

function exactRunDetailsReference(
  document: CanonicalResultDocumentV2,
  tableId: string,
  path: string,
): void {
  const references = document.sections.flatMap((section) => section.table_ids
    .filter((id) => id === tableId)
    .map(() => section.id));
  if (references.length !== 1 || references[0] !== "run_details") {
    fail(path, `${tableId} must belong exactly once to run_details`);
  }
}

function validateArchivedPointEstimateAttribution(
  document: CanonicalResultDocumentV2,
  currentAdapter: boolean,
  path: string,
): void {
  const tables = document.tables.filter((table) => table.id === "point_estimate_attribution");
  if (tables.length === 0 && !currentAdapter) return;
  if (tables.length !== 1) fail(path, "point_estimate_attribution must occur exactly once");
  exactRunDetailsReference(document, "point_estimate_attribution", path);
  const table = tables[0];
  exactTableColumns(table, PLS_POINT_ATTRIBUTION_COLUMNS, `${path}.point_estimate_attribution`);
  if (table.rows.length !== 1 || table.rows[0].id !== "attribution") {
    fail(`${path}.point_estimate_attribution`, "must contain exactly its canonical row");
  }
  const values = table.rows[0].cells.map((cell, index) => (
    canonicalTextCell(cell, `${path}.point_estimate_attribution.cells[${index}]`)
  ));
  const preprocessing = values[1];
  const mapping: Record<string, readonly [string, string]> = {
    standardized: ["sample_mean", "sample_standard_deviation"],
    mean_centered: ["sample_mean", "unit_scale"],
    unstandardized: ["no_centering", "unit_scale"],
  };
  const expected = mapping[preprocessing];
  if (
    values[0] !== "pls_point_estimate_attribution_v1"
    || !expected
    || values[2] !== expected[0]
    || values[3] !== expected[1]
    || values[4] !== "preprocessed_indicator_to_unit_variance_construct_score"
    || values[5] !== "indicator_construct_score_correlation"
    || values[6] !== "zero_mean_unit_variance_construct_score"
    || values[7] !== "standardized_construct_score_regression"
    || values[8] !== "standardized_structural_path_decomposition"
  ) fail(`${path}.point_estimate_attribution`, "has drifted preprocessing or scale attribution");
}

function validateArchivedAlgorithmConvergenceReceipt(
  document: CanonicalResultDocumentV2,
  currentAdapter: boolean,
  path: string,
): void {
  const summaries = document.tables.filter((table) => table.id === "algorithm_convergence_receipt");
  const blockTables = document.tables.filter((table) => table.id === "algorithm_block_order");
  if (summaries.length === 0 && blockTables.length === 0 && !currentAdapter) return;
  if (summaries.length !== 1 || blockTables.length !== 1) {
    fail(path, "algorithm convergence tables must occur as one exact family");
  }
  exactRunDetailsReference(document, "algorithm_convergence_receipt", path);
  exactRunDetailsReference(document, "algorithm_block_order", path);
  const summary = summaries[0];
  const blocks = blockTables[0];
  exactTableColumns(summary, PLS_ALGORITHM_CONVERGENCE_COLUMNS, `${path}.algorithm_convergence_receipt`);
  exactTableColumns(blocks, PLS_ALGORITHM_BLOCK_COLUMNS, `${path}.algorithm_block_order`);
  if (summary.rows.length !== 1 || summary.rows[0].id !== "convergence") {
    fail(`${path}.algorithm_convergence_receipt`, "must contain exactly its canonical row");
  }
  const cells = summary.rows[0].cells;
  const maximum = canonicalCountCell(cells[2], `${path}.algorithm_convergence_receipt.maximum_iterations`);
  const criterion = canonicalNumberCell(cells[3], `${path}.algorithm_convergence_receipt.stop_criterion`);
  const performed = canonicalCountCell(cells[5], `${path}.algorithm_convergence_receipt.performed_iterations`);
  const updates = canonicalCountCell(cells[6], `${path}.algorithm_convergence_receipt.estimated_block_updates`);
  if (
    canonicalTextCell(cells[0], `${path}.algorithm_convergence_receipt.contract_version`)
      !== "pls_algorithm_convergence_receipt_v1"
    || !["path", "factor"].includes(canonicalTextCell(cells[1], `${path}.algorithm_convergence_receipt.weighting_scheme`))
    || maximum !== 3_000
    || criterion !== 1e-7
    || canonicalTextCell(cells[4], `${path}.algorithm_convergence_receipt.comparison`) !== "less_than_or_equal"
  ) fail(`${path}.algorithm_convergence_receipt`, "has drifted settings or comparison");

  const byOrdinal = new Map<number, {
    construct: string;
    update: string;
    initialization: string;
    nextIndicator: number;
  }>();
  let lastBlockOrdinal = -1;
  blocks.rows.forEach((row, rowIndex) => {
    const rowPath = `${path}.algorithm_block_order.rows[${rowIndex}]`;
    const blockOrdinal = canonicalCountCell(row.cells[0], `${rowPath}.block_ordinal`);
    const construct = canonicalTextCell(row.cells[1], `${rowPath}.construct_id`);
    const indicatorOrdinal = canonicalCountCell(row.cells[2], `${rowPath}.indicator_ordinal`);
    canonicalTextCell(row.cells[3], `${rowPath}.indicator_id`);
    const update = canonicalTextCell(row.cells[4], `${rowPath}.update_rule`);
    const initialization = canonicalTextCell(row.cells[5], `${rowPath}.initialization`);
    const prior = byOrdinal.get(blockOrdinal);
    const coherent = (update === "mode_a_covariance" || update === "mode_b_ols")
      ? initialization === "standard_unit_weights" || initialization === "individual_requested_weights"
      : update === "fixed_no_update"
        && (initialization === "fixed_unit_weights" || initialization === "fixed_custom_weights");
    if (
      row.id !== `algorithm_block_${blockOrdinal.toString().padStart(4, "0")}_indicator_${indicatorOrdinal.toString().padStart(4, "0")}`
      || !coherent
      || (blockOrdinal !== lastBlockOrdinal && blockOrdinal !== lastBlockOrdinal + 1)
      || (prior && (prior.construct !== construct
        || prior.update !== update
        || prior.initialization !== initialization
        || prior.nextIndicator !== indicatorOrdinal))
      || (!prior && indicatorOrdinal !== 0)
    ) fail(rowPath, "has a non-canonical block/indicator order or algorithm token");
    byOrdinal.set(blockOrdinal, {
      construct,
      update,
      initialization,
      nextIndicator: indicatorOrdinal + 1,
    });
    lastBlockOrdinal = blockOrdinal;
  });
  if (byOrdinal.size === 0 || new Set([...byOrdinal.values()].map((block) => block.construct)).size !== byOrdinal.size) {
    fail(`${path}.algorithm_block_order`, "must cover unique nonempty blocks");
  }
  const estimatedBlocks = [...byOrdinal.values()].filter((block) => block.update !== "fixed_no_update").length;
  const scoreWeights = document.tables.find((table) => table.id === "score_execution_weights");
  if (scoreWeights) {
    const positions = Object.fromEntries(scoreWeights.columns.map((column, index) => [column.id, index]));
    for (const id of ["construct_id", "block_kind", "estimated_mode", "requested_initialization"]) {
      if (!Number.isInteger(positions[id])) fail(`${path}.score_execution_weights`, `omits ${id}`);
    }
    const scoreBlocks = new Map<string, { kind: string; mode: string | null; initialization: string | null; count: number }>();
    scoreWeights.rows.forEach((row, index) => {
      const rowPath = `${path}.score_execution_weights.rows[${index}]`;
      const construct = canonicalTextCell(row.cells[positions.construct_id], `${rowPath}.construct_id`);
      const observed = {
        kind: canonicalTextCell(row.cells[positions.block_kind], `${rowPath}.block_kind`),
        mode: canonicalOptionalTextCell(row.cells[positions.estimated_mode], `${rowPath}.estimated_mode`),
        initialization: canonicalOptionalTextCell(row.cells[positions.requested_initialization], `${rowPath}.requested_initialization`),
        count: 1,
      };
      const prior = scoreBlocks.get(construct);
      if (prior) {
        if (prior.kind !== observed.kind || prior.mode !== observed.mode || prior.initialization !== observed.initialization) {
          fail(rowPath, "changes score-block semantics");
        }
        prior.count += 1;
      } else scoreBlocks.set(construct, observed);
    });
    const orderedScoreBlocks = [...scoreBlocks.entries()];
    if (orderedScoreBlocks.length !== byOrdinal.size) fail(`${path}.algorithm_block_order`, "differs from score-block count");
    [...byOrdinal.values()].forEach((block, index) => {
      const [construct, scoring] = orderedScoreBlocks[index];
      const expected = scoring.kind === "estimated"
        ? {
            update: scoring.mode === "mode_a" ? "mode_a_covariance" : "mode_b_ols",
            initialization: scoring.initialization === "standard"
              ? "standard_unit_weights"
              : "individual_requested_weights",
          }
        : {
            update: "fixed_no_update",
            initialization: scoring.kind === "fixed_unit" ? "fixed_unit_weights" : "fixed_custom_weights",
          };
      if (
        block.construct !== construct
        || block.nextIndicator !== scoring.count
        || block.update !== expected.update
        || block.initialization !== expected.initialization
      ) fail(`${path}.algorithm_block_order`, "differs from score-execution block order or semantics");
    });
  }
  if (updates !== performed * estimatedBlocks) {
    fail(`${path}.algorithm_convergence_receipt`, "has incoherent iteration/update accounting");
  }
  const termination = canonicalTextCell(cells[7], `${path}.algorithm_convergence_receipt.termination_reason`);
  if (estimatedBlocks === 0) {
    if (
      termination !== "all_blocks_fixed"
      || performed !== 0
      || cells[8].kind !== "missing"
      || cells[8].reason !== "not_applicable"
      || cells[8].display !== undefined
    ) fail(`${path}.algorithm_convergence_receipt`, "has an incoherent all-fixed termination");
  } else {
    const finalChange = canonicalNumberCell(cells[8], `${path}.algorithm_convergence_receipt.final_max_outer_weight_change`);
    if (termination !== "converged_tolerance" || performed < 1 || performed > maximum || finalChange < 0 || finalChange > criterion) {
      fail(`${path}.algorithm_convergence_receipt`, "has an incoherent converged termination");
    }
  }
  const estimationSummary = document.tables.find((table) => table.id === "estimation_summary");
  if (!estimationSummary || estimationSummary.rows.length !== 1) fail(path, "omits estimation_summary");
  const iterationIndex = estimationSummary.columns.findIndex((column) => column.id === "iterations");
  if (
    iterationIndex < 0
    || canonicalCountCell(estimationSummary.rows[0].cells[iterationIndex], `${path}.estimation_summary.iterations`) !== performed
  ) fail(`${path}.algorithm_convergence_receipt`, "differs from estimation-summary iterations");
}

function capabilityCellEquals(
  actual: CanonicalResultDocumentV2["provenance"]["capability_cell"],
  expected: CanonicalResultDocumentV2["provenance"]["capability_cell"],
): boolean {
  return actual.registry_schema_version === expected.registry_schema_version
    && actual.capability_id === expected.capability_id
    && actual.cell_id === expected.cell_id
    && actual.capability_version === expected.capability_version;
}

function capabilityCellArrayEquals(
  actual: CanonicalResultDocumentV2["capability_cells"] | undefined,
  expected: readonly CanonicalResultDocumentV2["provenance"]["capability_cell"][],
): boolean {
  return actual?.length === expected.length
    && actual.every((cell, index) => capabilityCellEquals(cell, expected[index]));
}

function canonicalNonlinearWarning(cell: CanonicalResultCell, path: string): string | null {
  if (cell.kind === "text" && cell.value.trim()) return cell.value;
  if (cell.kind === "missing" && cell.reason === "not_estimated" && cell.display === undefined) return null;
  fail(path, "must be nonempty text or an undisplayed not_estimated cell");
}

function findExactlyOneTable(
  document: CanonicalResultDocumentV2,
  tableId: string,
  path: string,
): CanonicalResultTable {
  const matches = document.tables.filter((table) => table.id === tableId);
  if (matches.length !== 1) fail(path, `must contain exactly one ${tableId} table`);
  return matches[0];
}

/** Exact archive reader for the Recipe-v4 v7 fixed-score quadratic diagnostic lane. */
export function validateArchivedPlsNonlinearEffectsV1(
  document: CanonicalResultDocumentV2,
  path = "canonicalDocument",
): void {
  if (!capabilityCellEquals(document.provenance.capability_cell, PLS_NONLINEAR_CAPABILITY_CELL)) return;
  if (
    document.provenance.method_version !== PLS_NONLINEAR_METHOD_VERSION_V1
    || document.provenance.engine_version !== PLS_NONLINEAR_ADAPTER_VERSION_V7
    || document.title !== "PLS nonlinear quadratic diagnostics"
    || document.presentation.default_section_id !== "nonlinear_relationships"
    || document.presentation.default_table_id !== "nonlinear_quadratic_diagnostics"
  ) fail(path, "has a drifted nonlinear method, adapter, title, or default view");
  if (document.charts.length !== 0 || document.sections.some((section) => section.chart_ids.length !== 0)) {
    fail(path, "nonlinear v7 must not contain or reference charts");
  }
  if (!capabilityCellArrayEquals(document.capability_cells, [PLS_NONLINEAR_CAPABILITY_CELL, PLS_BASE_CAPABILITY_CELL])) {
    fail(path, "nonlinear capability_cells must be ordered [primary nonlinear, base PLS]");
  }
  if (JSON.stringify(document.sections.map((section) => section.id)) !== JSON.stringify([
    "run_details", "measurement_model", "structural_model", "nonlinear_relationships",
  ])) fail(path, "has a drifted nonlinear section order or ownership boundary");
  document.sections.forEach((section, index) => {
    const expected = index === 3 ? PLS_NONLINEAR_CAPABILITY_CELL : PLS_BASE_CAPABILITY_CELL;
    if (!capabilityCellArrayEquals(section.capability_cells, [expected])) {
      fail(`${path}.sections[${index}]`, "has a drifted capability owner");
    }
  });
  const hasControls = document.tables.some((table) => table.id === "control_estimates");
  const expectedStructural = ["structural_paths", "effects", "r_squared"];
  if (hasControls) expectedStructural.push("control_estimates");
  if (
    JSON.stringify(document.sections[0].table_ids)
      !== JSON.stringify(["estimation_summary", "point_estimate_attribution"])
    || JSON.stringify(document.sections[1].table_ids) !== JSON.stringify(["outer_model"])
    || JSON.stringify(document.sections[2].table_ids) !== JSON.stringify(expectedStructural)
  ) fail(path, "has drifted nonlinear base PLS section table membership or order");
  if (JSON.stringify(document.sections[3].table_ids) !== JSON.stringify(PLS_NONLINEAR_TABLE_IDS)) {
    fail(`${path}.nonlinear_relationships`, "has a drifted table order");
  }
  PLS_NONLINEAR_TABLE_IDS.forEach((tableId) => {
    const references = document.sections.reduce(
      (total, section) => total + section.table_ids.filter((id) => id === tableId).length,
      0,
    );
    if (references !== 1) fail(path, `${tableId} must belong exactly once to nonlinear_relationships`);
  });
  if (document.tables.some((table) => [
    "score_execution_summary", "score_execution_weights", "fixed_score_scale_receipt",
    "algorithm_convergence_receipt", "algorithm_block_order",
  ].includes(table.id))) fail(path, "v7 nonlinear document must not mix score, fixed-scale, or convergence tables");
  const nonlinearTables = document.tables.filter((table) => (
    (PLS_NONLINEAR_TABLE_IDS as readonly string[]).includes(table.id)
  ));
  const expectedTableIds = [
    "estimation_summary", "outer_model", "structural_paths", "effects", "r_squared",
  ];
  if (hasControls) expectedTableIds.push("control_estimates");
  expectedTableIds.push("point_estimate_attribution", ...PLS_NONLINEAR_TABLE_IDS);
  if (
    nonlinearTables.length !== PLS_NONLINEAR_TABLE_IDS.length
    || nonlinearTables.some((table, index) => table.id !== PLS_NONLINEAR_TABLE_IDS[index])
    || JSON.stringify(document.tables.map((table) => table.id)) !== JSON.stringify(expectedTableIds)
  ) fail(path, "nonlinear tables must occur exactly once at the canonical table tail");
  document.tables.forEach((table, index) => {
    const expected = (PLS_NONLINEAR_TABLE_IDS as readonly string[]).includes(table.id)
      ? PLS_NONLINEAR_CAPABILITY_CELL
      : PLS_BASE_CAPABILITY_CELL;
    if (!capabilityCellArrayEquals(table.capability_cells, [expected])) {
      fail(`${path}.tables[${index}]`, "has a drifted capability owner");
    }
  });
  validateArchivedPointEstimateAttribution(document, true, path);

  const diagnostics = findExactlyOneTable(document, "nonlinear_quadratic_diagnostics", path);
  const equations = findExactlyOneTable(document, "nonlinear_equation_fit", path);
  const scope = findExactlyOneTable(document, "nonlinear_method_scope", path);
  exactTableColumns(diagnostics, PLS_NONLINEAR_DIAGNOSTIC_COLUMNS, `${path}.nonlinear_quadratic_diagnostics`);
  exactTableColumns(equations, PLS_NONLINEAR_EQUATION_COLUMNS, `${path}.nonlinear_equation_fit`);
  exactTableColumns(scope, PLS_NONLINEAR_SCOPE_COLUMNS, `${path}.nonlinear_method_scope`);
  if (diagnostics.rows.length === 0) fail(path, "nonlinear_quadratic_diagnostics must not be empty");

  const structural = findExactlyOneTable(document, "structural_paths", path);
  exactTableColumns(structural, ["source", "target", "coefficient"], `${path}.structural_paths`);
  const structuralCoefficients = new Map<string, number>();
  structural.rows.forEach((row, index) => {
    const source = canonicalTextCell(row.cells[0], `${path}.structural_paths.rows[${index}].source`);
    const target = canonicalTextCell(row.cells[1], `${path}.structural_paths.rows[${index}].target`);
    const identity = `${target}\u0000${source}`;
    if (structuralCoefficients.has(identity)) fail(path, "structural_paths contains duplicate endpoints");
    structuralCoefficients.set(identity, canonicalNumberCell(row.cells[2], `${path}.structural_paths.rows[${index}].coefficient`));
  });

  const diagnosticIdentities = new Set<string>();
  const targets = new Set<string>();
  let priorTarget = "";
  let priorSource = "";
  diagnostics.rows.forEach((row, index) => {
    const rowPath = `${path}.nonlinear_quadratic_diagnostics.rows[${index}]`;
    if (row.id !== `nonlinear_quadratic_diagnostic_${index.toString().padStart(4, "0")}`) fail(`${rowPath}.id`, "is non-canonical");
    const source = canonicalTextCell(row.cells[0], `${rowPath}.source`);
    const target = canonicalTextCell(row.cells[1], `${rowPath}.target`);
    if (index > 0 && (
      compareUtf8StringsV1(priorTarget, target) > 0
      || (priorTarget === target && compareUtf8StringsV1(priorSource, source) >= 0)
    )) fail(rowPath, "is not strictly ordered by (target, source)");
    const identity = `${target}\u0000${source}`;
    const linear = canonicalNumberCell(row.cells[2], `${rowPath}.linear_coefficient`);
    const quadratic = canonicalNumberCell(row.cells[3], `${rowPath}.quadratic_coefficient`);
    const standardError = canonicalNumberCell(row.cells[4], `${rowPath}.standard_error`);
    const tStatistic = canonicalNumberCell(row.cells[5], `${rowPath}.t_statistic`);
    const pValue = canonicalNumberCell(row.cells[6], `${rowPath}.p_value_two_sided`);
    canonicalNonlinearWarning(row.cells[7], `${rowPath}.warning`);
    if (
      diagnosticIdentities.has(identity)
      || !structuralCoefficients.has(identity)
      || !Object.is(structuralCoefficients.get(identity), linear)
      || standardError <= 0
      || !Object.is(tStatistic, quadratic / standardError)
      || pValue < 0 || pValue > 1
    ) fail(rowPath, "has drifted numerical or structural invariants");
    diagnosticIdentities.add(identity);
    targets.add(target);
    priorTarget = target;
    priorSource = source;
  });
  if (
    diagnosticIdentities.size !== structuralCoefficients.size
    || [...structuralCoefficients.keys()].some((identity) => !diagnosticIdentities.has(identity))
  ) fail(path, "nonlinear diagnostic endpoints differ from structural_paths");

  const orderedTargets = [...targets].sort(compareUtf8StringsV1);
  if (equations.rows.length !== orderedTargets.length) fail(path, "nonlinear equation-fit targets differ from diagnostics");
  equations.rows.forEach((row, index) => {
    const rowPath = `${path}.nonlinear_equation_fit.rows[${index}]`;
    const target = canonicalTextCell(row.cells[0], `${rowPath}.target`);
    const linear = canonicalNumberCell(row.cells[1], `${rowPath}.linear_r_squared`);
    const augmented = canonicalNumberCell(row.cells[2], `${rowPath}.augmented_r_squared`);
    const delta = canonicalNumberCell(row.cells[3], `${rowPath}.delta_r_squared`);
    if (
      row.id !== `nonlinear_equation_fit_${index.toString().padStart(4, "0")}`
      || target !== orderedTargets[index]
      || linear < 0 || linear > 1 || augmented < 0 || augmented > 1
      || !Object.is(delta, Math.max(augmented - linear, 0))
    ) fail(rowPath, "has drifted row identity, order, or R-squared arithmetic");
  });
  const scopeRow = scope.rows[0];
  if (
    scope.rows.length !== 1 || scopeRow.id !== "nonlinear_method_scope"
    || canonicalTextCell(scopeRow.cells[0], `${path}.nonlinear_method_scope.method_version`) !== PLS_NONLINEAR_METHOD_VERSION_V1
    || canonicalTextCell(scopeRow.cells[1], `${path}.nonlinear_method_scope.term`) !== PLS_NONLINEAR_TERM_V1
    || canonicalTextCell(scopeRow.cells[2], `${path}.nonlinear_method_scope.warning`) !== PLS_NONLINEAR_ENGINE_WARNING_V1
  ) fail(path, "nonlinear_method_scope differs from the exact method-v1 contract");
}

/** Fail-closed self-consistency reader for archived PLS score-execution tables. */
export function validateArchivedPlsScoreExecutionV2(
  document: CanonicalResultDocumentV2,
  path = "canonicalDocument",
): void {
  const cell = document.provenance.capability_cell;
  const isBase = capabilityCellEquals(cell, PLS_BASE_CAPABILITY_CELL);
  const isNonlinear = capabilityCellEquals(cell, PLS_NONLINEAR_CAPABILITY_CELL);
  const hasNonlinearArtifact = document.provenance.method_version === PLS_NONLINEAR_METHOD_VERSION_V1
    || document.provenance.engine_version === PLS_NONLINEAR_ADAPTER_VERSION_V7
    || document.sections.some((section) => (
    section.id === "nonlinear_relationships"
    || section.capability_cells?.some((owner) => capabilityCellEquals(owner, PLS_NONLINEAR_CAPABILITY_CELL))
  )) || document.tables.some((table) => (
    (PLS_NONLINEAR_TABLE_IDS as readonly string[]).includes(table.id)
    || table.capability_cells?.some((owner) => capabilityCellEquals(owner, PLS_NONLINEAR_CAPABILITY_CELL))
  )) || document.capability_cells?.some((owner) => capabilityCellEquals(owner, PLS_NONLINEAR_CAPABILITY_CELL));
  if (isNonlinear) {
    validateArchivedPlsNonlinearEffectsV1(document, path);
    return;
  }
  if (hasNonlinearArtifact) {
    fail(path, "archived Recipe-v4 PLS v3-v6 document contains injected nonlinear artifacts");
  }
  if (!isBase) return;

  const summary = document.tables.find((table) => table.id === "score_execution_summary");
  const weights = document.tables.find((table) => table.id === "score_execution_weights");
  const tableIds = new Set(document.tables.map((table) => table.id));
  const controlReferences = document.sections.flatMap((section) => section.table_ids
    .filter((tableId) => tableId === "control_estimates")
    .map(() => section.id));
  if (
    tableIds.has("control_estimates")
    && (controlReferences.length !== 1 || controlReferences[0] !== "structural_model")
  ) {
    fail(path, "control_estimates must belong exactly once to structural_model");
  }
  const engine = document.provenance.engine_version;
  if (document.provenance.method_version === "pls_pm_v1") {
    const legacy = engine === "compiled_recipe_v4_pls_plan_v2_execution_v3";
    const current = engine === "compiled_recipe_v4_pls_plan_v2_execution_v5";
    if (summary || weights || (!legacy && !current)) {
      fail(path, "pls_pm_v1 has a non-allowlisted adapter generation or typed table identity");
    }
    validateArchivedPointEstimateAttribution(document, current, path);
    validateArchivedAlgorithmConvergenceReceipt(document, current, path);
    return;
  }
  const legacyScore = engine === "compiled_recipe_v4_pls_plan_v2_execution_v4";
  const currentScore = engine === "compiled_recipe_v4_pls_plan_v2_execution_v6";
  if (
    document.provenance.method_version !== "pls_score_execution_v2"
    || (!legacyScore && !currentScore)
    || !summary
    || !weights
  ) {
    fail(path, "has a non-allowlisted PLS score adapter generation or typed table identity");
  }
  validateArchivedPointEstimateAttribution(document, currentScore, path);
  validateArchivedAlgorithmConvergenceReceipt(document, currentScore, path);
  exactTableColumns(summary, PLS_SCORE_SUMMARY_COLUMNS, `${path}.score_execution_summary`);
  exactTableColumns(weights, PLS_SCORE_WEIGHT_COLUMNS, `${path}.score_execution_weights`);
  if (summary.rows.length !== 1 || summary.rows[0].id !== "execution") {
    fail(`${path}.score_execution_summary`, "must contain exactly the execution row");
  }
  const summaryCells = summary.rows[0].cells;
  if (canonicalTextCell(summaryCells[0], `${path}.score_execution_summary.contract_version`) !== "pls_score_execution_v2") {
    fail(`${path}.score_execution_summary.contract_version`, "must equal pls_score_execution_v2");
  }
  const maximumIterations = canonicalCountCell(summaryCells[1], `${path}.score_execution_summary.maximum_iterations`);
  const stopCriterion = canonicalNumberCell(summaryCells[2], `${path}.score_execution_summary.stop_criterion`);
  const estimatedBlockCount = canonicalCountCell(summaryCells[3], `${path}.score_execution_summary.estimated_block_count`);
  const fixedBlockCount = canonicalCountCell(summaryCells[4], `${path}.score_execution_summary.fixed_block_count`);
  const performedIterations = canonicalCountCell(summaryCells[5], `${path}.score_execution_summary.performed_iterations`);
  const estimatedBlockUpdates = canonicalCountCell(summaryCells[6], `${path}.score_execution_summary.estimated_block_updates`);

  const blockKinds = new Map<string, string>();
  const resolvedByBlock = new Map<string, {
    normalization: string | null;
    weights: Array<readonly [number, number]>;
  }>();
  const fixedScaleWeights: ArchivedFixedScoreWeight[] = [];
  const weightIdentities = new Set<string>();
  weights.rows.forEach((row, index) => {
    const rowPath = `${path}.score_execution_weights.rows[${index}]`;
    if (row.id !== `score_weight_${index.toString().padStart(4, "0")}`) {
      fail(`${rowPath}.id`, "is non-canonical");
    }
    const constructId = canonicalTextCell(row.cells[0], `${rowPath}.construct_id`);
    const indicatorId = canonicalTextCell(row.cells[1], `${rowPath}.indicator_id`);
    const identity = `${constructId}\u0000${indicatorId}`;
    if (weightIdentities.has(identity)) fail(rowPath, "duplicates a stable weight identity");
    weightIdentities.add(identity);
    const blockKind = canonicalTextCell(row.cells[2], `${rowPath}.block_kind`);
    const estimatedMode = canonicalOptionalTextCell(row.cells[3], `${rowPath}.estimated_mode`);
    const initialization = canonicalOptionalTextCell(row.cells[4], `${rowPath}.requested_initialization`);
    const normalization = canonicalOptionalTextCell(row.cells[5], `${rowPath}.normalization`);
    if (
      (blockKind === "estimated"
        && !(["mode_a", "mode_b"].includes(estimatedMode ?? "")))
      || (blockKind === "estimated"
        && (!(["standard", "individual"].includes(initialization ?? "")) || normalization !== null))
      || ((blockKind === "fixed_unit" || blockKind === "fixed_custom")
        && (estimatedMode !== null
          || initialization !== null
          || !["none", "sum_to_one", "unit_variance"].includes(normalization ?? "")))
      || !(["estimated", "fixed_unit", "fixed_custom"].includes(blockKind))
    ) {
      fail(rowPath, "has incoherent estimated/fixed score semantics");
    }
    const priorKind = blockKinds.get(constructId);
    if (priorKind && priorKind !== blockKind) fail(rowPath, "changes block kind within a construct");
    blockKinds.set(constructId, blockKind);
    const requested = canonicalNumberCell(row.cells[6], `${rowPath}.requested_weight`);
    const resolved = canonicalNumberCell(row.cells[7], `${rowPath}.resolved_initial_or_fixed_weight`);
    const finalOuter = canonicalNumberCell(row.cells[8], `${rowPath}.final_outer_weight`);
    if ((initialization === "standard" || blockKind === "fixed_unit") && requested !== 1) {
      fail(`${rowPath}.requested_weight`, "must equal exact +1 for Standard/Unit semantics");
    }
    if (blockKind !== "estimated" && !Object.is(resolved, finalOuter)) {
      fail(`${rowPath}.final_outer_weight`, "fixed scoring changed after resolution");
    }
    if (blockKind !== "estimated") {
      fixedScaleWeights.push({ constructId, indicatorId, resolved });
    }
    const constructWeights = resolvedByBlock.get(constructId) ?? {
      normalization,
      weights: [],
    };
    if (constructWeights.normalization !== normalization) {
      fail(rowPath, "changes normalization within a construct");
    }
    constructWeights.weights.push([requested, resolved]);
    resolvedByBlock.set(constructId, constructWeights);
  });
  if ([...resolvedByBlock.values()].some(({ normalization, weights: blockWeights }) => (
    !canonicalWeightsMatchNormalization(normalization, blockWeights)
  ))) {
    fail(`${path}.score_execution_weights`, "resolved weights violate the normalization contract");
  }
  validateArchivedFixedScoreScaleReceipt(
    document,
    fixedScaleWeights,
    currentScore,
    path,
  );
  const observedEstimated = [...blockKinds.values()].filter((kind) => kind === "estimated").length;
  const observedFixed = blockKinds.size - observedEstimated;
  if (
    maximumIterations !== 3_000
    || stopCriterion !== 1e-7
    || estimatedBlockCount !== observedEstimated
    || fixedBlockCount !== observedFixed
    || estimatedBlockUpdates !== performedIterations * estimatedBlockCount
    || (estimatedBlockCount === 0 && performedIterations !== 0)
    || (estimatedBlockCount > 0 && (performedIterations < 1 || performedIterations > 3_000))
  ) {
    fail(`${path}.score_execution_summary`, "has drifted iteration accounting");
  }
  const estimationSummary = document.tables.find((table) => table.id === "estimation_summary");
  if (!estimationSummary) fail(path, "omits estimation_summary");
  exactTableColumns(
    estimationSummary,
    ["converged", "iterations", "used_observations", "omitted_observations"],
    `${path}.estimation_summary`,
  );
  if (
    estimationSummary.rows.length !== 1
    || estimationSummary.rows[0].id !== "run"
    || estimationSummary.rows[0].cells[0].kind !== "boolean"
    || estimationSummary.rows[0].cells[0].value !== true
    || canonicalCountCell(
      estimationSummary.rows[0].cells[1],
      `${path}.estimation_summary.iterations`,
    ) !== performedIterations
  ) {
    fail(`${path}.estimation_summary`, "differs from score-execution accounting");
  }
}

function isArchivedExactCbsemCapabilityV1(
  capability: CanonicalResultDocumentV2["provenance"]["capability_cell"],
): boolean {
  return capability.registry_schema_version === 2 && (
    (capability.capability_id === "smartpls.cbsem"
      && capability.cell_id === "qpls3.cbsem.ml"
      && capability.capability_version === "cbsem_ml_v1")
    || (capability.capability_id === "smartpls.cbsem_bootstrapping"
      && capability.cell_id === "qpls3.cbsem.bootstrap"
      && capability.capability_version === "cbsem_exact_case_bootstrap_v1")
  );
}

/** Fail-closed reader for genuine CFA score/LM inference introduced by CB-SEM adapter v8. */
export function validateArchivedCbsemCfaScoreLmV1(
  document: CanonicalResultDocumentV2,
  path = "canonicalDocument",
): void {
  const capability = document.provenance.capability_cell;
  if (!isArchivedExactCbsemCapabilityV1(capability)) return;
  const scoreArtifacts = document.tables.filter((table) => table.id === "modification_index_score_tests" || table.id === "modification_indices");
  const tables = scoreArtifacts.filter((table) => table.id === "modification_index_score_tests");
  const sections = document.sections.filter((section) => section.id === "modification_indices");
  const current = document.provenance.engine_version === "compiled_recipe_v4_cbsem_plan_v2_execution_v8"
    || document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V9
    || document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V10
    || document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V11
    || document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V12;
  if (!current) {
    if (scoreArtifacts.length || sections.length) fail(path, "pre-v8 CB-SEM result carries genuine score/LM inference or a masquerading heuristic table");
    return;
  }
  if (document.provenance.method_version !== "cbsem_ml_exact_parameter_table_v3"
    || scoreArtifacts.length !== 1 || tables.length !== 1 || sections.length !== 1) {
    fail(path, "current score/LM adapter requires exactly one genuine score/LM table and section");
  }
  const section = sections[0];
  if (section.table_ids.length !== 1 || section.table_ids[0] !== "modification_index_score_tests"
    || section.chart_ids.length !== 0) fail(`${path}.modification_indices`, "has drifted table ownership or order");
  const table = tables[0];
  exactTableColumns(table, CBSEM_SCORE_LM_COLUMNS, `${path}.modification_index_score_tests`);
  let previousParameterId: string | null = null;
  table.rows.forEach((row, index) => {
    const rowPath = `${path}.modification_index_score_tests.rows[${index}]`;
    const cells = row.cells;
    if (row.id !== `score_lm_${String(index).padStart(4, "0")}`) fail(rowPath, "has a drifted stable row identity");
    if (canonicalTextCell(cells[0], `${rowPath}.method_version`) !== "cbsem_cfa_score_lm_v1"
      || canonicalTextCell(cells[1], `${rowPath}.scope`) !== "covariance_only_declared_zero_residual_covariances") {
      fail(rowPath, "has drifted score/LM method or scope");
    }
    const parameterId = canonicalTextCell(cells[2], `${rowPath}.parameter_id`);
    if (!parameterId || (previousParameterId !== null && compareUtf8StringsV1(previousParameterId, parameterId) >= 0)) {
      fail(`${path}.modification_index_score_tests`, "must be in stable parameter-id order without duplicates");
    }
    previousParameterId = parameterId;
    if (canonicalTextCell(cells[3], `${rowPath}.kind`) !== "residual_covariance") fail(`${rowPath}.kind`, "must equal residual_covariance");
    const lhs = canonicalTextCell(cells[4], `${rowPath}.lhs`);
    const rhs = canonicalTextCell(cells[5], `${rowPath}.rhs`);
    if (!lhs || !rhs || lhs === rhs) fail(rowPath, "must identify an off-diagonal residual covariance");
    const status = canonicalTextCell(cells[6], `${rowPath}.status`);
    if (status === "available") {
      const values = cells.slice(7, 15).map((cell, valueIndex) => canonicalNumberCell(cell, `${rowPath}.${CBSEM_SCORE_LM_COLUMNS[valueIndex + 7]}`));
      if (values.some((value) => Object.is(value, -0))) fail(rowPath, "must use canonical positive zero");
      const [score, efficientScore, candidateInformation, efficientInformation, modificationIndex, expectedParameterChange, degreesOfFreedom, pValue] = values;
      if (candidateInformation <= 0 || efficientInformation <= 0 || modificationIndex < 0
        || degreesOfFreedom !== 1 || pValue < 0 || pValue > 1
        || !Object.is(modificationIndex, efficientScore * efficientScore / efficientInformation)
        || !Object.is(expectedParameterChange, efficientScore / efficientInformation)
        || !cbsemCfaScoreLmNumbersCloseV1(pValue, cbsemCfaScoreLmChiSquare1PValueV1(modificationIndex))) {
        fail(rowPath, "has incoherent score/LM arithmetic, df, or chi-square(1) probability");
      }
      void score;
      if (canonicalOptionalTextCell(cells[15], `${rowPath}.unavailable_reason`) !== null) fail(rowPath, "available row must omit unavailable_reason");
    } else if (status === "unavailable") {
      for (let cellIndex = 7; cellIndex <= 14; cellIndex += 1) {
        if (canonicalOptionalNumberCell(cells[cellIndex], `${rowPath}.${CBSEM_SCORE_LM_COLUMNS[cellIndex]}`) !== null) fail(rowPath, "unavailable row must omit every numeric inference cell");
      }
      const reason = canonicalTextCell(cells[15], `${rowPath}.unavailable_reason`);
      if (reason !== "nuisance_information_unavailable" && reason !== "efficient_information_non_positive" && reason !== "non_finite_computation") fail(`${rowPath}.unavailable_reason`, "has an unknown typed reason");
    } else fail(`${rowPath}.status`, "must equal available or unavailable");
  });
}

function exactBootstrapFiniteNumber(cell: CanonicalResultCell, path: string): number {
  const value = canonicalNumberCell(cell, path);
  if (Object.is(value, -0)) fail(path, "must not use negative zero");
  return value;
}

function exactBootstrapCount(cell: CanonicalResultCell, path: string): number {
  const value = exactBootstrapFiniteNumber(cell, path);
  if (!Number.isSafeInteger(value) || value < 0) fail(path, "must be a nonnegative safe integer");
  return value;
}

function exactBootstrapCompactStringArray(value: string, path: string): string[] {
  let parsed: unknown;
  try { parsed = JSON.parse(value); } catch { fail(path, "must be compact canonical JSON"); }
  if (!Array.isArray(parsed) || !parsed.length || parsed.some((entry) => typeof entry !== "string" || !entry)) {
    fail(path, "must contain a nonempty string array");
  }
  if (JSON.stringify(parsed) !== value || new Set(parsed).size !== parsed.length) {
    fail(path, "must be compact canonical JSON without duplicate IDs");
  }
  return parsed as string[];
}

function exactBootstrapCompactNumberArray(value: string, expected: number, path: string): number[] {
  let parsed: unknown;
  try { parsed = JSON.parse(value); } catch { fail(path, "must be compact canonical JSON"); }
  if (!Array.isArray(parsed) || parsed.length !== expected
    || parsed.some((entry) => typeof entry !== "number" || !Number.isFinite(entry) || Object.is(entry, -0))
    || JSON.stringify(parsed) !== value) {
    fail(path, "must be a finite, signed-zero-safe compact canonical JSON vector");
  }
  return parsed as number[];
}

function exactBootstrapType7(sorted: readonly number[], probability: number): number {
  const position = probability * (sorted.length - 1);
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  return lower === upper
    ? sorted[lower]
    : sorted[lower] + (position - lower) * (sorted[upper] - sorted[lower]);
}

function exactBootstrapMissingNotApplicable(cell: CanonicalResultCell, path: string): void {
  if (cell.kind !== "missing" || cell.reason !== "not_applicable" || cell.display !== undefined) {
    fail(path, "must be an undisplayed not_applicable cell");
  }
}

function validateArchivedCbsemExactCaseBootstrapStudentizedV1(
  document: CanonicalResultDocumentV2,
  path: string,
  parameterIds: readonly string[],
  freeParameters: readonly { id: string; estimate: number }[],
  successfulRefitIndices: readonly number[],
  estimateVectors: readonly number[][],
  confidenceLevel: number,
  minimumUsableFraction: number,
  minimumUsableReplicates: number,
): void {
  const tableMatches = CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS.map((id) => (
    document.tables.filter((table) => table.id === id)
  ));
  const sections = document.sections.filter((section) => (
    section.id === CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID
  ));
  if (tableMatches.some((matches) => matches.length !== 1) || sections.length !== 1) {
    fail(path, "adapter v11 requires exactly one complete studentized bootstrap table family and section");
  }
  const section = sections[0];
  if (section.chart_ids.length !== 0
    || JSON.stringify(section.table_ids) !== JSON.stringify(CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS)) {
    fail(`${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID}`, "has drifted table ownership or order");
  }
  const [summary, point, intervals, refits] = tableMatches.map((matches) => matches[0]);
  exactTableColumns(
    summary,
    CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS,
    `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[0]}`,
  );
  exactTableColumns(
    point,
    CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS,
    `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[1]}`,
  );
  exactTableColumns(
    intervals,
    CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS,
    `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[2]}`,
  );
  exactTableColumns(
    refits,
    CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS,
    `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[3]}`,
  );

  const summaryPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[0]}`;
  if (summary.rows.length !== 1 || summary.rows[0].id !== "bootstrap_studentized") {
    fail(summaryPath, "must contain exactly the bootstrap_studentized row");
  }
  const summaryCells = summary.rows[0].cells;
  const summaryText = (index: number) => canonicalTextCell(
    summaryCells[index],
    `${summaryPath}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS[index]}`,
  );
  const summaryNumber = (index: number) => exactBootstrapFiniteNumber(
    summaryCells[index],
    `${summaryPath}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS[index]}`,
  );
  const summaryCount = (index: number) => exactBootstrapCount(
    summaryCells[index],
    `${summaryPath}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS[index]}`,
  );
  if (summaryText(0) !== CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_METHOD_V1
    || summaryText(1) !== CBSEM_EXACT_BOOTSTRAP_STANDARD_ERROR_METHOD_V1
    || summaryText(2) !== CBSEM_EXACT_BOOTSTRAP_EXPECTED_INFORMATION_METHOD_V1
    || summaryText(3) !== CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1
    || summaryText(4) !== CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1
    || summaryText(5) !== CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1
    || summaryText(6) !== CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_ARCHIVE_SCOPE_V1
    || !Object.is(summaryNumber(7), confidenceLevel)
    || !Object.is(summaryNumber(8), minimumUsableFraction)
    || summaryCount(9) !== minimumUsableReplicates
    || JSON.stringify(exactBootstrapCompactStringArray(summaryText(11), `${summaryPath}.parameter_ids_json`))
      !== JSON.stringify(parameterIds)) {
    fail(summaryPath, "has drifted studentized method, threshold, archive scope, or base ownership");
  }

  const pointPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[1]}`;
  if (point.rows.length !== parameterIds.length) {
    fail(pointPath, "must contain exactly one point standard-error row per parameter");
  }
  let pointStatus: "available" | "unavailable" | null = null;
  let pointUnavailableReason: string | null = null;
  const pointStandardErrors: number[] = [];
  point.rows.forEach((row, parameterIndex) => {
    const rowPath = `${pointPath}.rows[${parameterIndex}]`;
    const cells = row.cells;
    if (row.id !== `bootstrap_studentized_point_standard_error_${String(parameterIndex).padStart(4, "0")}`
      || canonicalTextCell(cells[0], `${rowPath}.method_version`) !== CBSEM_EXACT_BOOTSTRAP_STANDARD_ERROR_METHOD_V1
      || canonicalTextCell(cells[1], `${rowPath}.parameter_id`) !== parameterIds[parameterIndex]) {
      fail(rowPath, "has drifted point standard-error identity or parameter order");
    }
    const status = canonicalTextCell(cells[2], `${rowPath}.status`);
    if (status !== "available" && status !== "unavailable") fail(`${rowPath}.status`, "must equal available or unavailable");
    if (pointStatus === null) pointStatus = status;
    else if (status !== pointStatus) fail(pointPath, "must not mix point standard-error statuses");
    if (status === "available") {
      if (canonicalTextCell(cells[3], `${rowPath}.information_method`)
        !== CBSEM_EXACT_BOOTSTRAP_EXPECTED_INFORMATION_METHOD_V1) {
        fail(rowPath, "has a drifted expected-information method");
      }
      const standardError = exactBootstrapFiniteNumber(cells[4], `${rowPath}.standard_error`);
      if (standardError <= 0) fail(`${rowPath}.standard_error`, "must be positive");
      pointStandardErrors.push(standardError);
      exactBootstrapMissingNotApplicable(cells[5], `${rowPath}.unavailable_reason`);
    } else {
      exactBootstrapMissingNotApplicable(cells[3], `${rowPath}.information_method`);
      exactBootstrapMissingNotApplicable(cells[4], `${rowPath}.standard_error`);
      const reason = canonicalTextCell(cells[5], `${rowPath}.unavailable_reason`);
      if (!CBSEM_EXACT_BOOTSTRAP_STANDARD_ERROR_UNAVAILABLE_REASONS.has(reason)) {
        fail(`${rowPath}.unavailable_reason`, "has an unknown typed analytical standard-error reason");
      }
      if (pointUnavailableReason === null) pointUnavailableReason = reason;
      else if (reason !== pointUnavailableReason) fail(pointPath, "must repeat one whole-vector unavailable reason");
    }
  });

  const refitPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[3]}`;
  if (refits.rows.length !== successfulRefitIndices.length) {
    fail(refitPath, "must contain exactly one standard-error receipt per successful base refit");
  }
  const usableRefits: Array<{ estimates: readonly number[]; standardErrors: readonly number[] }> = [];
  refits.rows.forEach((row, refitIndex) => {
    const rowPath = `${refitPath}.rows[${refitIndex}]`;
    const cells = row.cells;
    const replicateIndex = exactBootstrapCount(cells[0], `${rowPath}.replicate_index`);
    if (replicateIndex !== successfulRefitIndices[refitIndex]
      || row.id !== `bootstrap_studentized_refit_standard_error_${String(replicateIndex).padStart(5, "0")}`) {
      fail(rowPath, "has drifted identity or successful-refit ledger order");
    }
    const status = canonicalTextCell(cells[1], `${rowPath}.status`);
    if (status === "available") {
      if (canonicalTextCell(cells[2], `${rowPath}.information_method`)
        !== CBSEM_EXACT_BOOTSTRAP_EXPECTED_INFORMATION_METHOD_V1) {
        fail(rowPath, "has a drifted expected-information method");
      }
      const standardErrors = exactBootstrapCompactNumberArray(
        canonicalTextCell(cells[3], `${rowPath}.standard_errors_json`),
        parameterIds.length,
        `${rowPath}.standard_errors_json`,
      );
      if (standardErrors.some((standardError) => standardError <= 0)) {
        fail(`${rowPath}.standard_errors_json`, "must contain only positive standard errors");
      }
      exactBootstrapMissingNotApplicable(cells[4], `${rowPath}.unavailable_reason`);
      usableRefits.push({ estimates: estimateVectors[refitIndex], standardErrors });
    } else if (status === "unavailable") {
      exactBootstrapMissingNotApplicable(cells[2], `${rowPath}.information_method`);
      exactBootstrapMissingNotApplicable(cells[3], `${rowPath}.standard_errors_json`);
      const reason = canonicalTextCell(cells[4], `${rowPath}.unavailable_reason`);
      if (!CBSEM_EXACT_BOOTSTRAP_STANDARD_ERROR_UNAVAILABLE_REASONS.has(reason)) {
        fail(`${rowPath}.unavailable_reason`, "has an unknown typed analytical standard-error reason");
      }
    } else fail(`${rowPath}.status`, "must equal available or unavailable");
  });

  const studentizedUsable = summaryCount(10);
  if (studentizedUsable !== usableRefits.length) {
    fail(`${summaryPath}.studentized_usable_replicates`, "differs from the compact refit standard-error partition");
  }
  const unavailableReason = pointStatus === "unavailable"
    ? "point_standard_errors_unavailable"
    : studentizedUsable < minimumUsableReplicates
      ? "insufficient_studentized_usable_replicates"
      : null;
  const inferenceStatus = summaryText(12);
  if (unavailableReason === null) {
    if (inferenceStatus !== "available") fail(`${summaryPath}.inference_status`, "must be available above the whole-vector usable threshold");
    exactBootstrapMissingNotApplicable(summaryCells[13], `${summaryPath}.unavailable_reason_code`);
    exactBootstrapMissingNotApplicable(summaryCells[14], `${summaryPath}.unavailable_message`);
  } else {
    const expectedMessage = unavailableReason === "point_standard_errors_unavailable"
      ? "Analytically studentized inference is unavailable because the point estimate has no whole-vector analytical standard-error receipt."
      : `Analytically studentized inference is unavailable because ${studentizedUsable} whole-vector usable refits are below the required ${minimumUsableReplicates}.`;
    if (inferenceStatus !== "unavailable"
      || canonicalTextCell(summaryCells[13], `${summaryPath}.unavailable_reason_code`) !== unavailableReason
      || canonicalTextCell(summaryCells[14], `${summaryPath}.unavailable_message`) !== expectedMessage) {
      fail(summaryPath, "does not exactly bind the analytical standard-error availability state");
    }
  }

  const intervalPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS[2]}`;
  if (intervals.rows.length !== parameterIds.length) {
    fail(intervalPath, "must contain exactly one typed interval outcome per parameter");
  }
  const lowerProbability = (1 - confidenceLevel) / 2;
  const upperProbability = 1 - lowerProbability;
  intervals.rows.forEach((row, parameterIndex) => {
    const rowPath = `${intervalPath}.rows[${parameterIndex}]`;
    const cells = row.cells;
    if (row.id !== `bootstrap_studentized_interval_${String(parameterIndex).padStart(4, "0")}`
      || canonicalTextCell(cells[0], `${rowPath}.parameter_id`) !== parameterIds[parameterIndex]) {
      fail(rowPath, "has drifted studentized interval identity or parameter order");
    }
    const status = canonicalTextCell(cells[1], `${rowPath}.status`);
    if (unavailableReason !== null) {
      if (status !== "unavailable") fail(`${rowPath}.status`, "must follow the global studentized availability state");
      for (let cellIndex = 2; cellIndex <= 8; cellIndex += 1) {
        exactBootstrapMissingNotApplicable(
          cells[cellIndex],
          `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS[cellIndex]}`,
        );
      }
      if (canonicalTextCell(cells[9], `${rowPath}.unavailable_reason`) !== unavailableReason) {
        fail(`${rowPath}.unavailable_reason`, "differs from the global studentized unavailable reason");
      }
      return;
    }
    if (status !== "available") fail(`${rowPath}.status`, "must be available under the global studentized state");
    const pointEstimate = freeParameters[parameterIndex].estimate;
    const pointStandardError = pointStandardErrors[parameterIndex];
    const pivots = usableRefits.map((refit) => (
      (refit.estimates[parameterIndex] - pointEstimate) / refit.standardErrors[parameterIndex]
    ));
    if (pivots.some((pivot) => !Number.isFinite(pivot))) fail(rowPath, "has a nonfinite studentized pivot");
    pivots.sort((left, right) => left - right);
    const lowerPivot = exactBootstrapType7(pivots, lowerProbability);
    const upperPivot = exactBootstrapType7(pivots, upperProbability);
    const expected = [
      pointEstimate,
      pointStandardError,
      lowerPivot,
      upperPivot,
      pointEstimate - upperPivot * pointStandardError,
      pointEstimate - lowerPivot * pointStandardError,
    ];
    const observed = cells.slice(2, 8).map((cell, offset) => exactBootstrapFiniteNumber(
      cell,
      `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS[offset + 2]}`,
    ));
    if (observed.some((value, index) => !Object.is(value, expected[index]))
      || exactBootstrapCount(cells[8], `${rowPath}.usable_replicates`) !== studentizedUsable
      || observed[4] > observed[5]) {
      fail(rowPath, "has drifted outer-SE pivots or reversed Type-7 interval arithmetic");
    }
    exactBootstrapMissingNotApplicable(cells[9], `${rowPath}.unavailable_reason`);
  });
}

function validateArchivedCbsemExactCaseBootstrapBcaV1(
  document: CanonicalResultDocumentV2,
  path: string,
  baseSummaryCells: readonly CanonicalResultCell[],
  parameterIds: readonly string[],
  freeParameters: readonly { id: string; estimate: number }[],
  estimateVectors: readonly number[][],
  confidenceLevel: number,
  usableReplicates: number,
  minimumUsableReplicates: number,
  completeCases: number,
  baseAvailable: boolean,
): void {
  const matches = CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS.map((id) => document.tables.filter((table) => table.id === id));
  const sections = document.sections.filter((section) => section.id === CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID);
  if (matches.some((tables) => tables.length !== 1) || sections.length !== 1) {
    fail(path, "adapter v12 requires exactly one complete BCa bootstrap table family and section");
  }
  const section = sections[0];
  if (section.chart_ids.length !== 0
    || JSON.stringify(section.table_ids) !== JSON.stringify(CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS)) {
    fail(`${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID}`, "has drifted table ownership or order");
  }
  const [summary, intervals, successes, failures] = matches.map((tables) => tables[0]);
  exactTableColumns(summary, CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS, `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[0]}`);
  exactTableColumns(intervals, CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS, `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[1]}`);
  exactTableColumns(successes, CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS, `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[2]}`);
  exactTableColumns(failures, CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS, `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[3]}`);
  const summaryPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[0]}`;
  if (summary.rows.length !== 1 || summary.rows[0].id !== "bootstrap_bca") {
    fail(summaryPath, "must contain exactly the bootstrap_bca row");
  }
  const cells = summary.rows[0].cells;
  const text = (index: number) => canonicalTextCell(cells[index], `${summaryPath}.${CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS[index]}`);
  const number = (index: number) => exactBootstrapFiniteNumber(cells[index], `${summaryPath}.${CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS[index]}`);
  const count = (index: number) => exactBootstrapCount(cells[index], `${summaryPath}.${CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS[index]}`);
  const baseText = (index: number) => canonicalTextCell(baseSummaryCells[index], `${path}.exact_case_bootstrap_summary.${CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS[index]}`);
  if (text(0) !== "cbsem_exact_case_bootstrap_bca_interval_v1"
    || text(1) !== baseText(0)
    || text(2) !== baseText(4) || text(3) !== baseText(5) || text(4) !== baseText(6)
    || text(5) !== baseText(7) || text(6) !== baseText(8)
    || text(7) !== "cbsem_exact_case_bootstrap_delete_one_refit_v1"
    || text(8) !== "sha256_complete_case_n_and_ordered_sampling_positions_v1"
    || text(9) !== "sha256_source_fingerprint_and_ordered_u64_indices_v1"
    || text(10) !== "midrank_less_plus_half_ties_no_clamp_v1"
    || text(11) !== "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2"
    || text(12) !== "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2"
    || text(13) !== "percentile_type7_v1"
    || text(14) !== "no_retry_exactly_one_fit_per_omitted_case_v1"
    || text(15) !== CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE
    || !Object.is(number(16), confidenceLevel)
    || count(17) !== usableReplicates || count(18) !== minimumUsableReplicates
    || count(19) !== completeCases
    || text(22) !== baseText(27)
    || JSON.stringify(exactBootstrapCompactStringArray(text(22), `${summaryPath}.parameter_ids_json`))
      !== JSON.stringify(parameterIds)) {
    fail(summaryPath, "has drifted BCa method, base authority, threshold, digest, or archive scope");
  }
  if (count(20) !== successes.rows.length || count(21) !== failures.rows.length
    || successes.rows.length + failures.rows.length !== completeCases) {
    fail(summaryPath, "has drifted delete-one accounting");
  }

  const omissions: Array<{ position: number; sourceRow: number }> = [];
  successes.rows.forEach((row, index) => {
    const rowPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[2]}.rows[${index}]`;
    const position = exactBootstrapCount(row.cells[0], `${rowPath}.omitted_complete_case_position`);
    const sourceRow = exactBootstrapCount(row.cells[1], `${rowPath}.omitted_source_row_index`);
    if (row.id !== `bootstrap_bca_delete_one_refit_${String(position).padStart(5, "0")}`
      || position >= completeCases
      || (index > 0 && position <= exactBootstrapCount(successes.rows[index - 1].cells[0], `${rowPath}.previous_position`))) {
      fail(rowPath, "has drifted delete-one identity or order");
    }
    if (!SHA256_HEX.test(canonicalTextCell(row.cells[2], `${rowPath}.retained_sampling_positions_sha256`))
      || !SHA256_HEX.test(canonicalTextCell(row.cells[3], `${rowPath}.retained_sample_indices_sha256`))) {
      fail(rowPath, "has a malformed delete-one digest");
    }
    exactBootstrapCompactNumberArray(canonicalTextCell(row.cells[4], `${rowPath}.parameter_estimates_json`), parameterIds.length, `${rowPath}.parameter_estimates_json`);
    if (exactBootstrapCount(row.cells[5], `${rowPath}.iterations`) === 0
      || exactBootstrapFiniteNumber(row.cells[6], `${rowPath}.objective`) < 0
      || exactBootstrapFiniteNumber(row.cells[7], `${rowPath}.gradient_norm`) < 0) {
      fail(rowPath, "has invalid delete-one convergence values");
    }
    omissions.push({ position, sourceRow });
  });
  failures.rows.forEach((row, index) => {
    const rowPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[3]}.rows[${index}]`;
    const position = exactBootstrapCount(row.cells[0], `${rowPath}.omitted_complete_case_position`);
    const sourceRow = exactBootstrapCount(row.cells[1], `${rowPath}.omitted_source_row_index`);
    if (row.id !== `bootstrap_bca_delete_one_failure_${String(position).padStart(5, "0")}`
      || position >= completeCases
      || (index > 0 && position <= exactBootstrapCount(failures.rows[index - 1].cells[0], `${rowPath}.previous_position`))) {
      fail(rowPath, "has drifted failed delete-one identity or order");
    }
    if (!SHA256_HEX.test(canonicalTextCell(row.cells[2], `${rowPath}.retained_sampling_positions_sha256`))
      || !SHA256_HEX.test(canonicalTextCell(row.cells[3], `${rowPath}.retained_sample_indices_sha256`))
      || !["moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure"]
        .includes(canonicalTextCell(row.cells[4], `${rowPath}.kind`))
      || !canonicalTextCell(row.cells[5], `${rowPath}.message`).trim()) {
      fail(rowPath, "has an invalid delete-one failure payload");
    }
    omissions.push({ position, sourceRow });
  });
  omissions.sort((left, right) => left.position - right.position);
  if (omissions.length !== completeCases || omissions.some((row, index) => row.position !== index)
    || omissions.some((row, index) => index > 0 && omissions[index - 1].sourceRow >= row.sourceRow)) {
    fail(path, "does not form the complete, source-ordered delete-one omission partition");
  }

  const globalReason = !baseAvailable
    ? "base_inference_unavailable"
    : failures.rows.length > 0
      ? "incomplete_delete_one_ledger"
      : null;
  const inferenceStatus = text(23);
  if (globalReason === null) {
    if (inferenceStatus !== "available") fail(`${summaryPath}.inference_status`, "must be available with complete ledgers");
    exactBootstrapMissingNotApplicable(cells[24], `${summaryPath}.unavailable_reason_code`);
    exactBootstrapMissingNotApplicable(cells[25], `${summaryPath}.unavailable_message`);
  } else {
    const expectedMessage = globalReason === "base_inference_unavailable"
      ? `BCa inference is unavailable because ${usableReplicates} successful bootstrap point refits are below the bound minimum ${minimumUsableReplicates}.`
      : `BCa inference is unavailable because ${failures.rows.length} of ${completeCases} mandatory delete-one fits failed.`;
    if (inferenceStatus !== "unavailable"
      || canonicalTextCell(cells[24], `${summaryPath}.unavailable_reason_code`) !== globalReason
      || canonicalTextCell(cells[25], `${summaryPath}.unavailable_message`) !== expectedMessage) {
      fail(summaryPath, "does not exactly bind the global BCa availability state");
    }
  }
  if (intervals.rows.length !== parameterIds.length) {
    fail(`${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[1]}`, "must contain one typed outcome per parameter");
  }
  intervals.rows.forEach((row, parameterIndex) => {
    const rowPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS[1]}.rows[${parameterIndex}]`;
    const cells = row.cells;
    if (row.id !== `bootstrap_bca_interval_${String(parameterIndex).padStart(4, "0")}`
      || canonicalTextCell(cells[0], `${rowPath}.parameter_id`) !== parameterIds[parameterIndex]) {
      fail(rowPath, "has drifted BCa interval identity or parameter order");
    }
    const status = canonicalTextCell(cells[1], `${rowPath}.status`);
    if (status === "unavailable") {
      for (let index = 2; index <= 9; index += 1) {
        exactBootstrapMissingNotApplicable(cells[index], `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS[index]}`);
      }
      const reason = canonicalTextCell(cells[10], `${rowPath}.unavailable_reason`);
      if (!CBSEM_EXACT_BOOTSTRAP_BCA_UNAVAILABLE_REASONS.has(reason)
        || (globalReason !== null && reason !== globalReason)
        || (globalReason === null && (reason === "base_inference_unavailable" || reason === "incomplete_delete_one_ledger"))) {
        fail(rowPath, "has a BCa unavailable reason inconsistent with the global state");
      }
      return;
    }
    if (status !== "available" || globalReason !== null) fail(`${rowPath}.status`, "cannot be available under the global BCa state");
    const observed = cells.slice(2, 9).map((cell, offset) => (
      exactBootstrapFiniteNumber(cell, `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS[offset + 2]}`)
    ));
    const lowerProbability = observed[3];
    const upperProbability = observed[4];
    const sorted = estimateVectors.map((vector) => vector[parameterIndex]).sort((left, right) => left - right);
    if (!Object.is(observed[0], freeParameters[parameterIndex].estimate)
      || lowerProbability < 0 || upperProbability > 1 || lowerProbability > upperProbability
      || !Object.is(observed[5], exactBootstrapType7(sorted, lowerProbability))
      || !Object.is(observed[6], exactBootstrapType7(sorted, upperProbability))
      || observed[5] > observed[6]
      || exactBootstrapCount(cells[9], `${rowPath}.usable_replicates`) !== usableReplicates) {
      fail(rowPath, "has drifted base binding, adjusted probability, or exposed Type-7 arithmetic");
    }
    exactBootstrapMissingNotApplicable(cells[10], `${rowPath}.unavailable_reason`);
  });
}

/** Strict v9-v12 descriptor/arithmetic validator. Rust remains authoritative for seed-to-schedule replay. */
export function validateArchivedCbsemExactCaseBootstrapV1(
  document: CanonicalResultDocumentV2,
  path = "canonicalDocument",
): void {
  const capability = document.provenance.capability_cell;
  if (!isArchivedExactCbsemCapabilityV1(capability)) return;

  const tables = CBSEM_EXACT_BOOTSTRAP_TABLE_IDS.map((id) => document.tables.filter((table) => table.id === id));
  const sections = document.sections.filter((section) => section.id === "bootstrap_inference");
  const hypothesisTables = document.tables.filter((table) => table.id === CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID);
  const hypothesisSections = document.sections.filter((section) => section.id === CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID);
  const studentizedTables = document.tables.filter((table) => (
    CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS.includes(
      table.id as (typeof CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS)[number],
    )
  ));
  const studentizedSections = document.sections.filter((section) => (
    section.id === CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID
  ));
  const bcaTables = document.tables.filter((table) => (
    CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS.includes(
      table.id as (typeof CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS)[number],
    )
  ));
  const bcaSections = document.sections.filter((section) => section.id === CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID);
  const v10 = document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V10;
  const v11 = document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V11;
  const v12 = document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V12;
  const current = document.provenance.engine_version === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V9
    || v10 || v11 || v12;
  if (!current) {
    if (tables.some((matches) => matches.length) || sections.length || hypothesisTables.length
      || hypothesisSections.length || studentizedTables.length || studentizedSections.length
      || bcaTables.length || bcaSections.length) {
      fail(path, "pre-v9 CB-SEM result carries exact case-bootstrap artifacts");
    }
    return;
  }
  if (!v10 && !v11 && !v12 && (hypothesisTables.length || hypothesisSections.length)) {
    fail(path, "historical adapter v9 exact bootstrap carries injected v10 hypothesis-test artifacts");
  }
  if (!v11 && (studentizedTables.length || studentizedSections.length)) {
    fail(path, "adapter v9/v10 exact bootstrap carries injected v11 studentized artifacts");
  }
  if (!v12 && (bcaTables.length || bcaSections.length)) {
    fail(path, "adapter v9-v11 exact bootstrap carries injected v12 BCa artifacts");
  }
  if (v10 && (hypothesisTables.length !== 1 || hypothesisSections.length !== 1)) {
    fail(path, "adapter v10 requires exactly one exact case-bootstrap hypothesis-test table and section");
  }
  if (v11 && (hypothesisTables.length !== 1 || hypothesisSections.length !== 1)) {
    fail(path, "adapter v11 requires exactly one exact case-bootstrap hypothesis-test table and section");
  }
  if (v12 && (hypothesisTables.length !== 1 || hypothesisSections.length !== 1)) {
    fail(path, "adapter v12 requires exactly one exact case-bootstrap hypothesis-test table and section");
  }
  if (v11 && (studentizedTables.length !== CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TABLE_IDS.length
    || studentizedSections.length !== 1)) {
    fail(path, "adapter v11 requires exactly one complete studentized bootstrap table family and section");
  }
  if (v12 && (bcaTables.length !== CBSEM_EXACT_BOOTSTRAP_BCA_TABLE_IDS.length
    || bcaSections.length !== 1)) {
    fail(path, "adapter v12 requires exactly one complete BCa bootstrap table family and section");
  }
  if (tables.some((matches) => matches.length !== 1) || sections.length !== 1) {
    fail(path, "adapter v9-v12 requires exactly one complete exact case-bootstrap table family and section");
  }
  const section = sections[0];
  if (section.chart_ids.length !== 0
    || JSON.stringify(section.table_ids) !== JSON.stringify(CBSEM_EXACT_BOOTSTRAP_TABLE_IDS)) {
    fail(`${path}.bootstrap_inference`, "has drifted table ownership or order");
  }
  const [summary, intervals, successes, failures] = tables.map((matches) => matches[0]);
  exactTableColumns(summary, CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS, `${path}.exact_case_bootstrap_summary`);
  exactTableColumns(intervals, CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS, `${path}.exact_case_bootstrap_parameter_intervals`);
  exactTableColumns(successes, CBSEM_EXACT_BOOTSTRAP_SUCCESS_COLUMNS, `${path}.exact_case_bootstrap_successful_refits`);
  exactTableColumns(failures, CBSEM_EXACT_BOOTSTRAP_FAILURE_COLUMNS, `${path}.exact_case_bootstrap_failures`);
  if (summary.rows.length !== 1 || summary.rows[0].id !== "bootstrap") {
    fail(`${path}.exact_case_bootstrap_summary`, "must contain exactly the bootstrap row");
  }
  const cells = summary.rows[0].cells;
  const text = (index: number) => canonicalTextCell(cells[index], `${path}.exact_case_bootstrap_summary.${CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS[index]}`);
  const count = (index: number) => exactBootstrapCount(cells[index], `${path}.exact_case_bootstrap_summary.${CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS[index]}`);
  const number = (index: number) => exactBootstrapFiniteNumber(cells[index], `${path}.exact_case_bootstrap_summary.${CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS[index]}`);
  if (text(0) !== CBSEM_EXACT_BOOTSTRAP_METHOD_V1
    || text(1) !== "cbsem_ml_exact_parameter_table_v3"
    || text(1) !== document.provenance.method_version
    || text(2) !== document.provenance.dataset_id
    || text(3) !== document.provenance.dataset_fingerprint
    || text(4) !== document.provenance.recipe_digest
    || text(8) !== document.provenance.model_digest
    || text(10) !== "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1"
    || text(12) !== "maximum_likelihood_n"
    || text(13) !== "sha256_source_fingerprint_and_ordered_u64_indices_v1"
    || text(14) !== CBSEM_EXACT_BOOTSTRAP_SCHEDULE_DIGEST_V1
    || text(15) !== "percentile_type7_v1"
    || text(24) !== CBSEM_EXACT_BOOTSTRAP_STREAM_V1
    || text(25) !== "no_retry_fixed_preplanned_primary_draws_v1"
    || text(31) !== "schedule_and_arithmetic_only_no_raw_refit_replay_or_source_row_digest_recomputation") {
    fail(`${path}.exact_case_bootstrap_summary`, "has drifted method, provenance, digest, interval, stream, retry, or validation-scope identity");
  }
  [text(4), text(5), text(6), text(7), text(8), text(11)].forEach((value, index) => {
    if (!SHA256_HEX.test(value)) fail(`${path}.exact_case_bootstrap_summary.digest[${index}]`, "must be lowercase SHA-256");
  });
  const completeCases = count(9);
  const confidence = number(16);
  const requested = count(17);
  const attempted = count(18);
  const usable = count(19);
  const failed = count(20);
  const minimumFraction = number(21);
  const minimumUsable = count(22);
  const seedDecimal = text(23);
  const maxAttempts = count(26);
  const parameterIds = exactBootstrapCompactStringArray(text(27), `${path}.exact_case_bootstrap_summary.parameter_ids_json`);
  const status = text(28);
  const unavailableReason = canonicalOptionalTextCell(cells[29], `${path}.exact_case_bootstrap_summary.unavailable_reason_code`);
  const unavailableMessage = canonicalOptionalTextCell(cells[30], `${path}.exact_case_bootstrap_summary.unavailable_message`);
  if (confidence !== 0.95 || requested < 500 || requested > 10_000 || attempted !== requested
    || usable + failed !== requested || minimumFraction !== 0.9
    || minimumUsable !== Math.max(1_000, Math.ceil(0.9 * requested)) || maxAttempts !== 1
    || !/^(0|[1-9][0-9]*)$/.test(seedDecimal) || BigInt(seedDecimal) > 0xffff_ffff_ffff_ffffn
    || document.provenance.seed === null || !Number.isSafeInteger(document.provenance.seed)
    || BigInt(document.provenance.seed) !== BigInt(seedDecimal)) {
    fail(`${path}.exact_case_bootstrap_summary`, "has incoherent plan, accounting, threshold, or seed binding");
  }
  const estimation = document.tables.find((table) => table.id === "estimation_summary");
  if (!estimation || estimation.rows.length !== 1 || estimation.rows[0].cells.length < 13
    || completeCases !== canonicalCountCell(estimation.rows[0].cells[12], `${path}.estimation_summary.sample_size`)) {
    fail(`${path}.exact_case_bootstrap_summary.complete_case_sample_size`, "differs from the point-estimator sample size");
  }
  const parameters = document.tables.find((table) => table.id === "parameters");
  if (!parameters || parameters.columns.length !== 10
    || JSON.stringify(parameters.columns.map((column) => column.id)) !== JSON.stringify([
      "name", "parameter_id", "kind", "lhs", "rhs", "estimate", "standard_error", "z", "p_two_sided", "fixed",
    ])) fail(`${path}.parameters`, "has a drifted parameter contract");
  const freeParameters = parameters.rows.filter((row, index) => {
    if (row.id !== `parameter_${String(index).padStart(4, "0")}` || row.cells.length !== 10) fail(`${path}.parameters.rows[${index}]`, "has drifted order");
    return !canonicalBooleanCell(row.cells[9], `${path}.parameters.rows[${index}].fixed`);
  }).map((row, index) => ({
    id: canonicalTextCell(row.cells[1], `${path}.parameters.free[${index}].parameter_id`),
    estimate: exactBootstrapFiniteNumber(row.cells[5], `${path}.parameters.free[${index}].estimate`),
  }));
  if (JSON.stringify(freeParameters.map((row) => row.id)) !== JSON.stringify(parameterIds)) {
    fail(`${path}.exact_case_bootstrap_summary.parameter_ids_json`, "differs from the free point-parameter identity/order");
  }
  if ((v11 || v12) && (!Number.isSafeInteger(document.provenance.workers)
    || document.provenance.workers < 1 || document.provenance.workers > 12
    || completeCases < 1 || completeCases > 180
    || parameterIds.length < 1 || parameterIds.length > 18)) {
    fail(path, `adapter ${v12 ? "v12" : "v11"} exceeds the archived Labs W<=12, N<=180, or P<=18 workload envelope`);
  }

  const estimateVectors: number[][] = [];
  const successfulRefitIndices: number[] = [];
  const observedIndices: number[] = [];
  successes.rows.forEach((row, ordinal) => {
    const rowPath = `${path}.exact_case_bootstrap_successful_refits.rows[${ordinal}]`;
    const replicate = exactBootstrapCount(row.cells[0], `${rowPath}.replicate_index`);
    if (row.id !== `bootstrap_refit_${String(replicate).padStart(5, "0")}`
      || replicate >= requested || (ordinal > 0 && replicate <= observedIndices[observedIndices.length - 1])) fail(rowPath, "has a drifted identity or replicate order");
    const positionsDigest = canonicalTextCell(row.cells[1], `${rowPath}.sampling_positions_sha256`);
    const indicesDigest = canonicalTextCell(row.cells[2], `${rowPath}.sample_indices_sha256`);
    if (!SHA256_HEX.test(positionsDigest) || !SHA256_HEX.test(indicesDigest)) fail(rowPath, "has a malformed digest");
    const estimates = exactBootstrapCompactNumberArray(canonicalTextCell(row.cells[3], `${rowPath}.parameter_estimates_json`), parameterIds.length, `${rowPath}.parameter_estimates_json`);
    const iterations = exactBootstrapCount(row.cells[4], `${rowPath}.iterations`);
    const objective = exactBootstrapFiniteNumber(row.cells[5], `${rowPath}.objective`);
    const gradient = exactBootstrapFiniteNumber(row.cells[6], `${rowPath}.gradient_norm`);
    if (iterations === 0 || objective < 0 || gradient < 0) fail(rowPath, "has invalid refit convergence values");
    observedIndices.push(replicate);
    successfulRefitIndices.push(replicate);
    estimateVectors.push(estimates);
  });
  failures.rows.forEach((row, ordinal) => {
    const rowPath = `${path}.exact_case_bootstrap_failures.rows[${ordinal}]`;
    const replicate = exactBootstrapCount(row.cells[0], `${rowPath}.replicate_index`);
    if (row.id !== `bootstrap_failure_${String(replicate).padStart(5, "0")}` || replicate >= requested
      || (ordinal > 0 && replicate <= exactBootstrapCount(failures.rows[ordinal - 1].cells[0], `${rowPath}.previous_replicate_index`))) fail(rowPath, "has a drifted identity or replicate order");
    if (!SHA256_HEX.test(canonicalTextCell(row.cells[1], `${rowPath}.sampling_positions_sha256`))
      || !SHA256_HEX.test(canonicalTextCell(row.cells[2], `${rowPath}.sample_indices_sha256`))) fail(rowPath, "has a malformed digest");
    if (![
      "moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure",
    ].includes(canonicalTextCell(row.cells[3], `${rowPath}.kind`))
      || !canonicalTextCell(row.cells[4], `${rowPath}.message`).trim()) fail(rowPath, "has an unknown failure kind or empty message");
    observedIndices.push(replicate);
  });
  observedIndices.sort((left, right) => left - right);
  if (successes.rows.length !== usable || failures.rows.length !== failed
    || observedIndices.length !== requested || observedIndices.some((value, index) => value !== index)) {
    fail(`${path}.bootstrap_inference`, "does not form an exact success/failure partition of the preplanned schedule");
  }

  const available = usable >= minimumUsable;
  if (available !== (status === "available") || (!available && status !== "unavailable")) fail(`${path}.exact_case_bootstrap_summary.inference_status`, "differs from the usable-refit threshold");
  if (available) {
    if (unavailableReason !== null || unavailableMessage !== null || intervals.rows.length !== parameterIds.length) fail(`${path}.bootstrap_inference`, "available inference has drifted reason or interval cardinality");
  } else if (unavailableReason !== "insufficient_usable_refits" || !unavailableMessage?.trim() || intervals.rows.length !== 0) {
    fail(`${path}.bootstrap_inference`, "unavailable inference must retain its typed reason and omit intervals");
  }
  intervals.rows.forEach((row, parameterIndex) => {
    const rowPath = `${path}.exact_case_bootstrap_parameter_intervals.rows[${parameterIndex}]`;
    if (row.id !== `bootstrap_interval_${String(parameterIndex).padStart(4, "0")}`
      || canonicalTextCell(row.cells[0], `${rowPath}.parameter_id`) !== parameterIds[parameterIndex]) fail(rowPath, "has drifted parameter identity/order");
    const observed = row.cells.slice(1, 7).map((cell, index) => exactBootstrapFiniteNumber(cell, `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS[index + 1]}`));
    const values = estimateVectors.map((vector) => vector[parameterIndex]);
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
    const standardError = Math.sqrt(values.reduce((sum, value) => {
      const difference = value - mean;
      return sum + difference * difference;
    }, 0) / (values.length - 1));
    const sorted = [...values].sort((left, right) => left - right);
    const expected = [
      freeParameters[parameterIndex].estimate, mean, mean - freeParameters[parameterIndex].estimate,
      standardError, exactBootstrapType7(sorted, 0.025000000000000022), exactBootstrapType7(sorted, 0.975),
    ];
    if (observed.some((value, index) => !Object.is(value, expected[index]))
      || exactBootstrapCount(row.cells[7], `${rowPath}.usable_replicates`) !== usable) fail(rowPath, "has drifted point binding, sample-SD, bias, or Type-7 arithmetic");
  });

  if (!v10 && !v11 && !v12) return;
  const hypothesisSection = hypothesisSections[0];
  if (hypothesisSection.chart_ids.length !== 0
    || hypothesisSection.table_ids.length !== 1
    || hypothesisSection.table_ids[0] !== CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID) {
    fail(`${path}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID}`, "has drifted table ownership or order");
  }
  const hypothesis = hypothesisTables[0];
  exactTableColumns(hypothesis, CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS, `${path}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID}`);
  if (hypothesis.rows.length !== freeParameters.length) {
    fail(`${path}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID}`, "must contain exactly one row per free stable parameter ID");
  }
  const unavailableReasons = new Set([
    "insufficient_usable_replicates", "nonregular_variance_boundary",
    "zero_null_outside_open_domain", "unsupported_parameter_family",
  ]);
  let repeatedGlobalUnavailableMessage: string | null = null;
  let repeatedSelectedTail: "two_sided" | "one_sided_greater" | "one_sided_less" | null = null;
  hypothesis.rows.forEach((row, parameterIndex) => {
    const rowPath = `${path}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID}.rows[${parameterIndex}]`;
    const cells = row.cells;
    if (row.id !== `bootstrap_hypothesis_${String(parameterIndex).padStart(4, "0")}`) {
      fail(rowPath, "has a drifted stable row identity");
    }
    const hypothesisText = (index: number) => canonicalTextCell(cells[index], `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS[index]}`);
    if (hypothesisText(0) !== "cbsem_exact_case_bootstrap_null_centered_test_tail_v1"
      || hypothesisText(1) !== "compiled_free_parameter_equals_zero_v1"
      || hypothesisText(2) !== "unstudentized_null_centered_parameter_estimate_v1"
      || hypothesisText(3) !== "inclusive_ieee_comparison_v1"
      || hypothesisText(4) !== "plus_one_over_usable_plus_one_v1"
      || hypothesisText(5) !== "selected_p_value_less_than_or_equal_alpha_v1") {
      fail(rowPath, "has drifted hypothesis-test method literals");
    }
    const selectedTail = hypothesisText(6);
    if (selectedTail !== "two_sided" && selectedTail !== "one_sided_greater" && selectedTail !== "one_sided_less") {
      fail(`${rowPath}.selected_test_tail`, "adapter v10/v11 has an unknown selected test tail");
    }
    if (repeatedSelectedTail === null) repeatedSelectedTail = selectedTail;
    else if (selectedTail !== repeatedSelectedTail) fail(rowPath, "has a drifted repeated selected test tail");
    const nullValue = exactBootstrapFiniteNumber(cells[7], `${rowPath}.null_value`);
    const significanceLevel = exactBootstrapFiniteNumber(cells[8], `${rowPath}.significance_level`);
    if (!Object.is(nullValue, 0) || significanceLevel !== 0.05
      || exactBootstrapCount(cells[9], `${rowPath}.usable_replicates`) !== usable) {
      fail(rowPath, "has drifted null, alpha, or usable-refit binding");
    }
    const inferenceStatus = hypothesisText(10);
    const globalReason = canonicalOptionalTextCell(cells[11], `${rowPath}.global_unavailable_reason_code`);
    const globalMessage = canonicalOptionalTextCell(cells[12], `${rowPath}.global_unavailable_message`);
    if (available) {
      if (inferenceStatus !== "available" || globalReason !== null || globalMessage !== null) {
        fail(rowPath, "available global inference must omit its unavailable reason and message");
      }
    } else {
      if (inferenceStatus !== "unavailable" || globalReason !== "insufficient_usable_refits" || !globalMessage?.trim()) {
        fail(rowPath, "unavailable global inference must retain its typed reason and nonempty message");
      }
      if (repeatedGlobalUnavailableMessage === null) repeatedGlobalUnavailableMessage = globalMessage;
      else if (globalMessage !== repeatedGlobalUnavailableMessage) fail(rowPath, "has a drifted repeated global unavailable message");
    }
    const parameter = freeParameters[parameterIndex];
    if (hypothesisText(13) !== parameter.id) fail(rowPath, "has drifted stable parameter identity/order");
    const parameterStatus = hypothesisText(14);
    if (parameterStatus === "available") {
      if (!available) fail(rowPath, "parameter inference cannot be available when global inference is unavailable");
      const pointEstimate = exactBootstrapFiniteNumber(cells[15], `${rowPath}.point_estimate`);
      const observedCounts = [16, 17, 18].map((index) => exactBootstrapCount(cells[index], `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS[index]}`));
      const deltas = estimateVectors.map((vector) => vector[parameterIndex] - parameter.estimate);
      const expectedCounts = [
        deltas.filter((delta) => Math.abs(delta) >= Math.abs(parameter.estimate)).length,
        deltas.filter((delta) => delta >= parameter.estimate).length,
        deltas.filter((delta) => delta <= parameter.estimate).length,
      ];
      const observedProbabilities = [19, 20, 21].map((index) => exactBootstrapFiniteNumber(cells[index], `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS[index]}`));
      const expectedProbabilities = expectedCounts.map((countValue) => (countValue + 1) / (usable + 1));
      const selectedCount = exactBootstrapCount(cells[22], `${rowPath}.selected_exceedances`);
      const selectedPValue = exactBootstrapFiniteNumber(cells[23], `${rowPath}.selected_p_value`);
      const selectedIndex = selectedTail === "two_sided" ? 0 : selectedTail === "one_sided_greater" ? 1 : 2;
      if (!Object.is(pointEstimate, parameter.estimate)
        || observedCounts.some((value, index) => value !== expectedCounts[index] || value > usable)
        || observedProbabilities.some((value, index) => !Object.is(value, expectedProbabilities[index]))
        || selectedCount !== expectedCounts[selectedIndex]
        || !Object.is(selectedPValue, expectedProbabilities[selectedIndex])
        || canonicalBooleanCell(cells[24], `${rowPath}.reject_null`) !== (selectedPValue <= 0.05)) {
        fail(rowPath, "has drifted null-centered counts, plus-one probabilities, selected tail, or decision");
      }
      exactBootstrapMissingNotApplicable(cells[25], `${rowPath}.unavailable_reason`);
    } else if (parameterStatus === "unavailable") {
      for (let index = 15; index <= 24; index += 1) {
        exactBootstrapMissingNotApplicable(cells[index], `${rowPath}.${CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS[index]}`);
      }
      const reason = hypothesisText(25);
      if (!unavailableReasons.has(reason) || (available && reason === "insufficient_usable_replicates")) {
        fail(`${rowPath}.unavailable_reason`, "has an unknown or globally inconsistent typed reason");
      }
    } else {
      fail(`${rowPath}.parameter_status`, "must equal available or unavailable");
    }
  });
  if (v11) {
    validateArchivedCbsemExactCaseBootstrapStudentizedV1(
      document,
      path,
      parameterIds,
      freeParameters,
      successfulRefitIndices,
      estimateVectors,
      confidence,
      minimumFraction,
      minimumUsable,
    );
  }
  if (v12) {
    validateArchivedCbsemExactCaseBootstrapBcaV1(
      document,
      path,
      cells,
      parameterIds,
      freeParameters,
      estimateVectors,
      confidence,
      usable,
      minimumUsable,
      completeCases,
      available,
    );
  }
}

/** Fail-closed reader for the exact RMSEA interval introduced by CB-SEM adapters v5-v8. */
export function validateArchivedCbsemRmseaIntervalV1(
  document: CanonicalResultDocumentV2,
  path = "canonicalDocument",
): void {
  const capability = document.provenance.capability_cell;
  if (!isArchivedExactCbsemCapabilityV1(capability)) return;

  const adapter = document.provenance.engine_version;
  const estimator = document.provenance.method_version;
  const legacy = (
    (adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v2"
      || adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v4")
      && estimator === "cbsem_ml_exact_parameter_table_v3"
  ) || (
    adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v3"
      && estimator === "cbsem_ml_exact_parameter_table_v4"
  );
  const current = (
    (adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v5"
      || adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v8"
      || adapter === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V9
      || adapter === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V10
      || adapter === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V11
      || adapter === CBSEM_EXACT_BOOTSTRAP_ADAPTER_V12
      || adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v7")
      && estimator === "cbsem_ml_exact_parameter_table_v3"
  ) || (
    adapter === "compiled_recipe_v4_cbsem_plan_v2_execution_v6"
      && estimator === "cbsem_ml_exact_parameter_table_v4"
  );
  if (!legacy && !current) {
    fail(path, "has an unsupported exact CB-SEM estimator/adapter identity");
  }

  const fitTables = document.tables.filter((table) => table.id === "fit_indices");
  if (fitTables.length !== 1) fail(path, "must contain exactly one fit_indices table");
  const fit = fitTables[0];
  exactTableColumns(
    fit,
    current ? CBSEM_RMSEA_INTERVAL_COLUMNS : CBSEM_HISTORICAL_FIT_COLUMNS,
    `${path}.fit_indices`,
  );
  if (fit.rows.length !== 1 || fit.rows[0].id !== "model") {
    fail(`${path}.fit_indices`, "must contain exactly the model row");
  }
  if (legacy) return;
  const cells = fit.rows[0].cells;
  if (
    canonicalTextCell(cells[0], `${path}.fit_indices.fit_method_version`) !== "cbsem_fit_v1"
    || canonicalTextCell(cells[7], `${path}.fit_indices.rmsea_interval_method_version`)
      !== CBSEM_RMSEA_INTERVAL_METHOD_VERSION
    || canonicalNumberCell(cells[8], `${path}.fit_indices.rmsea_interval_confidence_level`) !== 0.9
  ) fail(`${path}.fit_indices`, "has a drifted fit or RMSEA interval attribution");

  const degreesOfFreedom = canonicalCountCell(cells[2], `${path}.fit_indices.degrees_of_freedom`);
  const rmsea = canonicalOptionalNumberCell(cells[6], `${path}.fit_indices.rmsea`);
  const lower = canonicalOptionalNumberCell(cells[9], `${path}.fit_indices.rmsea_ci_lower`);
  const upper = canonicalOptionalNumberCell(cells[10], `${path}.fit_indices.rmsea_ci_upper`);
  if (degreesOfFreedom === 0) {
    if (rmsea !== null || lower !== null || upper !== null) {
      fail(`${path}.fit_indices`, "must omit the RMSEA point and interval at zero degrees of freedom");
    }
    return;
  }
  if (rmsea === null || lower === null || upper === null) {
    fail(`${path}.fit_indices`, "must contain the RMSEA point and both interval bounds");
  }
  const normalizedRmsea = rmsea === 0 ? 0 : rmsea;
  const normalizedLower = lower === 0 ? 0 : lower;
  const normalizedUpper = upper === 0 ? 0 : upper;
  if (
    normalizedLower < 0
    || normalizedRmsea < 0
    || normalizedUpper < 0
    || normalizedLower > normalizedRmsea
    || normalizedRmsea > normalizedUpper
  ) fail(`${path}.fit_indices`, "has a negative, reversed, or point-excluding RMSEA interval");
}

/** Fail-closed self-consistency reader for the frozen CB-SEM mean-replacement tables. */
export function validateArchivedCbsemMissingDataExecutionV1(
  document: CanonicalResultDocumentV2,
  path = "canonicalDocument",
): void {
  const capability = document.provenance.capability_cell;
  if (!isArchivedExactCbsemCapabilityV1(capability)) return;

  const matching = (id: string) => document.tables.filter((table) => table.id === id);
  const executionTables = matching("missing_data_execution");
  const variableTables = matching("mean_replacement_variables");
  const cellTables = matching("mean_replacement_cells");
  const isMeanReplacement = document.provenance.engine_version
    === "compiled_recipe_v4_cbsem_plan_v2_execution_v4"
    || document.provenance.engine_version === "compiled_recipe_v4_cbsem_plan_v2_execution_v7";
  if (!isMeanReplacement) {
    if (executionTables.length || variableTables.length || cellTables.length) {
      fail(path, "non-mean-replacement CB-SEM result carries missing-data tables");
    }
    return;
  }
  if (
    document.provenance.method_version !== "cbsem_ml_exact_parameter_table_v3"
    || executionTables.length !== 1
    || variableTables.length !== 1
    || cellTables.length !== 1
  ) fail(path, "has a drifted mean-replacement method, adapter, or table identity");

  const execution = executionTables[0];
  const variables = variableTables[0];
  const cells = cellTables[0];
  exactTableColumns(execution, MISSING_DATA_EXECUTION_COLUMNS, `${path}.missing_data_execution`);
  exactTableColumns(variables, MEAN_REPLACEMENT_VARIABLE_COLUMNS, `${path}.mean_replacement_variables`);
  exactTableColumns(cells, MEAN_REPLACEMENT_CELL_COLUMNS, `${path}.mean_replacement_cells`);
  if (execution.rows.length !== 1 || execution.rows[0].id !== "execution") {
    fail(`${path}.missing_data_execution`, "must contain exactly the execution row");
  }
  const summary = execution.rows[0].cells;
  if (
    canonicalTextCell(summary[0], `${path}.missing_data_execution.method_version`) !== "mean_replacement_v1"
    || canonicalTextCell(summary[1], `${path}.missing_data_execution.policy`) !== "mean_replacement"
    || canonicalTextCell(summary[2], `${path}.missing_data_execution.archive_validation_scope`)
      !== "descriptor_identity_shape_and_receipt_only"
    || canonicalBooleanCell(summary[3], `${path}.missing_data_execution.raw_replay_performed`)
  ) fail(`${path}.missing_data_execution`, "must disclose descriptor-only validation without raw replay");
  const sourceDatasetId = canonicalTextCell(summary[4], `${path}.missing_data_execution.source_dataset_id`);
  const sourceFingerprint = dataFingerprintAt(
    canonicalTextCell(
      summary[5],
      `${path}.missing_data_execution.source_dataset_fingerprint`,
    ),
    `${path}.missing_data_execution.source_dataset_fingerprint`,
  );
  const sourceRows = canonicalCountCell(summary[6], `${path}.missing_data_execution.source_row_count`);
  const retainedRows = canonicalCountCell(summary[7], `${path}.missing_data_execution.retained_row_count`);
  const omittedRows = canonicalCountCell(summary[8], `${path}.missing_data_execution.omitted_row_count`);
  const modeledVariables = canonicalCountCell(summary[9], `${path}.missing_data_execution.modeled_variable_count`);
  const imputedCells = canonicalCountCell(summary[10], `${path}.missing_data_execution.imputed_cell_count`);
  const affectedCases = canonicalCountCell(summary[11], `${path}.missing_data_execution.affected_case_count`);
  if (
    retainedRows !== sourceRows
    || omittedRows !== 0
    || modeledVariables !== variables.rows.length
    || imputedCells !== cells.rows.length
    || canonicalNumberCell(summary[12], `${path}.missing_data_execution.variable_warning_threshold`) !== 0.05
    || canonicalNumberCell(summary[13], `${path}.missing_data_execution.high_missingness_threshold`) !== 0.15
  ) fail(`${path}.missing_data_execution`, "has incoherent counts or frozen thresholds");
  [14, 15, 16].forEach((index) => {
    const digest = canonicalTextCell(summary[index], `${path}.missing_data_execution.cells[${index}]`);
    if (!SHA256_HEX.test(digest)) fail(`${path}.missing_data_execution.cells[${index}]`, "must be lowercase SHA-256");
  });

  type Variable = {
    id: string;
    source: string;
    mean: number;
    missing: number;
  };
  const variableById = new Map<string, Variable>();
  const parsedVariables = variables.rows.map((row, index): Variable => {
    const rowPath = `${path}.mean_replacement_variables.rows[${index}]`;
    if (
      row.id !== `mean_replacement_variable_${index.toString().padStart(4, "0")}`
      || canonicalCountCell(row.cells[0], `${rowPath}.variable_order`) !== index
    ) fail(rowPath, "has a non-canonical row identity or variable order");
    const id = canonicalTextCell(row.cells[1], `${rowPath}.variable_id`);
    const source = canonicalTextCell(row.cells[2], `${rowPath}.source_column`);
    if (!id || !source || variableById.has(id)) fail(rowPath, "has an empty or duplicate identity");
    const markersJson = canonicalTextCell(row.cells[3], `${rowPath}.canonical_missing_markers_json`);
    let markers: unknown;
    try {
      markers = JSON.parse(markersJson);
    } catch {
      fail(`${rowPath}.canonical_missing_markers_json`, "must be valid canonical JSON");
    }
    if (
      !Array.isArray(markers)
      || markers.some((marker) => typeof marker !== "string" || !marker.trim())
      || JSON.stringify(markers) !== markersJson
      || markers.some((marker, markerIndex) => marker !== [...markers].sort()[markerIndex])
      || new Set(markers).size !== markers.length
    ) fail(`${rowPath}.canonical_missing_markers_json`, "must be a sorted unique string array");
    const observed = canonicalCountCell(row.cells[4], `${rowPath}.observed_count`);
    const missing = canonicalCountCell(row.cells[5], `${rowPath}.missing_count`);
    const mean = canonicalNumberCell(row.cells[6], `${rowPath}.replacement_mean`);
    const fraction = canonicalNumberCell(row.cells[7], `${rowPath}.missing_fraction`);
    const warning = canonicalTextCell(row.cells[8], `${rowPath}.warning_level`);
    const expectedWarning = missing * 100 > sourceRows * 15
      ? "above_fifteen_percent"
      : missing * 100 >= sourceRows * 5 ? "at_least_five_percent" : "none";
    if (observed + missing !== sourceRows || fraction !== missing / sourceRows || warning !== expectedWarning) {
      fail(rowPath, "has incoherent counts, fraction, or warning classification");
    }
    const variable = { id, source, mean, missing };
    variableById.set(id, variable);
    return variable;
  });

  const missingByVariable = new Map<string, number>();
  const caseCells = new Map<number, Array<{ order: number; id: string }>>();
  const caseMetadata = new Map<number, { fraction: number; warning: boolean }>();
  let previousIdentity: readonly [number, number] | null = null;
  cells.rows.forEach((row, index) => {
    const rowPath = `${path}.mean_replacement_cells.rows[${index}]`;
    if (row.id !== `mean_replacement_cell_${index.toString().padStart(6, "0")}`) {
      fail(`${rowPath}.id`, "is non-canonical");
    }
    const sourceRow = canonicalCountCell(row.cells[0], `${rowPath}.row_index_zero_based`);
    const order = canonicalCountCell(row.cells[1], `${rowPath}.variable_order`);
    const id = canonicalTextCell(row.cells[2], `${rowPath}.variable_id`);
    const variable = parsedVariables[order];
    if (
      sourceRow >= sourceRows
      || !variable
      || variable.id !== id
      || variable.source !== canonicalTextCell(row.cells[3], `${rowPath}.source_column`)
      || !Object.is(variable.mean, canonicalNumberCell(row.cells[4], `${rowPath}.replacement_mean`))
      || (previousIdentity !== null
        && (sourceRow < previousIdentity[0]
          || (sourceRow === previousIdentity[0] && order <= previousIdentity[1])))
    ) fail(rowPath, "has a missing, duplicate, reordered, or drifted cell identity");
    previousIdentity = [sourceRow, order];
    missingByVariable.set(id, (missingByVariable.get(id) ?? 0) + 1);
    const caseList = caseCells.get(sourceRow) ?? [];
    caseList.push({ order, id });
    caseCells.set(sourceRow, caseList);
    const metadata = {
      fraction: canonicalNumberCell(row.cells[5], `${rowPath}.case_missing_fraction`),
      warning: canonicalBooleanCell(row.cells[6], `${rowPath}.high_missingness_warning`),
    };
    const prior = caseMetadata.get(sourceRow);
    if (prior && (!Object.is(prior.fraction, metadata.fraction) || prior.warning !== metadata.warning)) {
      fail(rowPath, "has inconsistent case metadata");
    }
    caseMetadata.set(sourceRow, metadata);
  });
  parsedVariables.forEach((variable) => {
    if ((missingByVariable.get(variable.id) ?? 0) !== variable.missing) {
      fail(`${path}.mean_replacement_cells`, "differs from variable missing counts");
    }
  });
  if (caseCells.size !== affectedCases) fail(`${path}.mean_replacement_cells`, "differs from affected_case_count");
  caseCells.forEach((items, sourceRow) => {
    const metadata = caseMetadata.get(sourceRow)!;
    if (
      metadata.fraction !== items.length / modeledVariables
      || metadata.warning !== (items.length * 100 > modeledVariables * 15)
    ) fail(`${path}.mean_replacement_cells`, "has a drifted case fraction or warning");
  });

  const missingSections = document.sections.filter((section) => section.id === "missing_data");
  if (
    missingSections.length !== 1
    || JSON.stringify(missingSections[0].table_ids) !== JSON.stringify([
      "missing_data_execution",
      "mean_replacement_variables",
      "mean_replacement_cells",
    ])
  ) fail(path, "has an absent, duplicate, or reordered missing_data section");

  const estimationTables = matching("estimation_summary");
  if (estimationTables.length !== 1 || estimationTables[0].rows.length !== 1) {
    fail(path, "must contain exactly one estimation_summary row");
  }
  const estimation = estimationTables[0];
  const estimationCell = (id: string) => {
    const indexes = estimation.columns.flatMap((column, index) => column.id === id ? [index] : []);
    if (indexes.length !== 1) fail(`${path}.estimation_summary`, `must contain one ${id} column`);
    return estimation.rows[0].cells[indexes[0]];
  };
  if (
    canonicalTextCell(estimationCell("execution_adapter_version"), `${path}.estimation_summary.execution_adapter_version`)
      !== document.provenance.engine_version
    || canonicalTextCell(estimationCell("estimator_method_version"), `${path}.estimation_summary.estimator_method_version`)
      !== "cbsem_ml_exact_parameter_table_v3"
    || canonicalTextCell(estimationCell("moment_input_method_version"), `${path}.estimation_summary.moment_input_method_version`)
      !== "cbsem_ml_compiled_moment_input_mean_replacement_v1"
    || canonicalCountCell(estimationCell("compiled_moment_schema_version"), `${path}.estimation_summary.compiled_moment_schema_version`) !== 4
    || canonicalBooleanCell(estimationCell("mean_structure"), `${path}.estimation_summary.mean_structure`)
    || canonicalTextCell(estimationCell("input"), `${path}.estimation_summary.input`) !== "raw"
    || !canonicalBooleanCell(estimationCell("converged"), `${path}.estimation_summary.converged`)
    || canonicalCountCell(estimationCell("sample_size"), `${path}.estimation_summary.sample_size`) !== retainedRows
    || canonicalCountCell(estimationCell("omitted_observations"), `${path}.estimation_summary.omitted_observations`) !== 0
    || canonicalTextCell(estimationCell("covariance_denominator"), `${path}.estimation_summary.covariance_denominator`) !== "maximum_likelihood_n"
  ) fail(`${path}.estimation_summary`, "differs from the frozen mean-replacement execution identity");
  const declaredSampleSize = estimationCell("declared_sample_size");
  if (declaredSampleSize.kind !== "missing" || declaredSampleSize.reason !== "not_estimated" || declaredSampleSize.display !== undefined) {
    fail(`${path}.estimation_summary.declared_sample_size`, "must be undisplayed not_estimated");
  }
  const observedMeansDigest = estimationCell("canonical_observed_means_sha256");
  if (observedMeansDigest.kind !== "missing" || observedMeansDigest.reason !== "not_estimated" || observedMeansDigest.display !== undefined) {
    fail(`${path}.estimation_summary.canonical_observed_means_sha256`, "must be undisplayed not_estimated");
  }

  const provenanceFingerprint = dataFingerprintAt(
    document.provenance.dataset_fingerprint,
    `${path}.provenance.dataset_fingerprint`,
  );
  if (
    document.provenance.dataset_id !== sourceDatasetId
    || provenanceFingerprint.recordedSha256 !== sourceFingerprint.recordedSha256
  ) fail(`${path}.provenance`, "differs from the missing-data execution source identity");
  const covarianceTables = matching("canonical_ml_covariance");
  if (
    covarianceTables.length !== 1
    || covarianceTables[0].rows.length !== parsedVariables.length
    || covarianceTables[0].columns.length !== parsedVariables.length + 1
  ) fail(`${path}.canonical_ml_covariance`, "has a drifted matrix shape");
  parsedVariables.forEach((variable, index) => {
    const covariance = covarianceTables[0];
    if (
      covariance.columns[index + 1].id !== `column_${index.toString().padStart(4, "0")}`
      || covariance.columns[index + 1].label !== variable.source
      || covariance.rows[index].id !== `row_${index.toString().padStart(4, "0")}`
      || canonicalTextCell(covariance.rows[index].cells[0], `${path}.canonical_ml_covariance.rows[${index}]`) !== variable.source
    ) fail(`${path}.canonical_ml_covariance`, "has a drifted variable identity or order");
  });
}

/** Hashes the exact UTF-8 canonical JSON bytes supplied by the Rust archive reader. */
export async function canonicalResultDocumentJsonSha256V1(
  canonicalDocumentJson: string,
): Promise<string> {
  return sha256Hex(
    canonicalDocumentJson,
    "canonicalDocumentJsonSha256",
  );
}

async function parseEntry(
  input: unknown,
  path: string,
  projectId: string,
): Promise<InternalProjectSchema6CanonicalResultEntryV1> {
  const entry = recordAt(input, path);
  requireExactKeys(entry, ENTRY_KEYS, path);

  const documentId = nonemptyStringAt(entry.documentId, `${path}.documentId`);
  const runId = nonemptyStringAt(entry.runId, `${path}.runId`);
  const canonicalDocumentSha256 = sha256At(
    entry.canonicalDocumentSha256,
    `${path}.canonicalDocumentSha256`,
  );
  if (entry.immutable !== true) fail(`${path}.immutable`, "must equal true");

  const canonicalDocumentJson = nonemptyStringAt(
    entry.canonicalDocumentJson,
    `${path}.canonicalDocumentJson`,
  );
  const observedSha256 = await canonicalResultDocumentJsonSha256V1(canonicalDocumentJson);
  if (observedSha256 !== canonicalDocumentSha256) {
    fail(`${path}.canonicalDocumentSha256`, "does not match the exact canonicalDocumentJson bytes");
  }

  let parsedCanonicalDocument: unknown;
  try {
    parsedCanonicalDocument = JSON.parse(canonicalDocumentJson);
  } catch {
    fail(`${path}.canonicalDocumentJson`, "must contain valid JSON");
  }
  validateCanonicalDocumentWireShape(
    parsedCanonicalDocument,
    `${path}.canonicalDocumentJson`,
  );
  const canonicalDocumentFromJson = parsedCanonicalDocument as CanonicalResultDocumentV2;
  let jsonValidation;
  try {
    jsonValidation = validateCanonicalResultDocumentV2(canonicalDocumentFromJson);
  } catch {
    fail(`${path}.canonicalDocumentJson`, "must encode a valid CanonicalResultDocumentV2");
  }
  if (!jsonValidation.passed) {
    fail(`${path}.canonicalDocumentJson`, jsonValidation.errors.join("; "));
  }
  validateArchivedPlsScoreExecutionV2(
    canonicalDocumentFromJson,
    `${path}.canonicalDocumentJson`,
  );
  validateArchivedCbsemMissingDataExecutionV1(
    canonicalDocumentFromJson,
    `${path}.canonicalDocumentJson`,
  );
  validateArchivedCbsemRmseaIntervalV1(
    canonicalDocumentFromJson,
    `${path}.canonicalDocumentJson`,
  );
  validateArchivedCbsemCfaScoreLmV1(
    canonicalDocumentFromJson,
    `${path}.canonicalDocumentJson`,
  );
  validateArchivedCbsemExactCaseBootstrapV1(
    canonicalDocumentFromJson,
    `${path}.canonicalDocumentJson`,
  );

  const canonicalDocument = entry.canonicalDocument as CanonicalResultDocumentV2;
  validateCanonicalDocumentWireShape(canonicalDocument, `${path}.canonicalDocument`);
  let validation;
  try {
    validation = validateCanonicalResultDocumentV2(canonicalDocument);
  } catch {
    fail(`${path}.canonicalDocument`, "must be a structurally valid CanonicalResultDocumentV2");
  }
  if (!validation.passed) {
    fail(`${path}.canonicalDocument`, validation.errors.join("; "));
  }
  validateArchivedPlsScoreExecutionV2(canonicalDocument, `${path}.canonicalDocument`);
  validateArchivedCbsemMissingDataExecutionV1(
    canonicalDocument,
    `${path}.canonicalDocument`,
  );
  validateArchivedCbsemRmseaIntervalV1(
    canonicalDocument,
    `${path}.canonicalDocument`,
  );
  validateArchivedCbsemCfaScoreLmV1(
    canonicalDocument,
    `${path}.canonicalDocument`,
  );
  validateArchivedCbsemExactCaseBootstrapV1(
    canonicalDocument,
    `${path}.canonicalDocument`,
  );
  if (
    canonicalResultDocumentJson(canonicalDocumentFromJson)
    !== canonicalResultDocumentJson(canonicalDocument)
  ) {
    fail(`${path}.canonicalDocument`, "does not semantically match canonicalDocumentJson");
  }
  if (documentId !== canonicalDocument.document_id) {
    fail(`${path}.documentId`, "does not match canonicalDocument.document_id");
  }
  if (runId !== canonicalDocument.provenance.run_id) {
    fail(`${path}.runId`, "does not match canonicalDocument.provenance.run_id");
  }
  if (projectId !== canonicalDocument.provenance.project_id) {
    fail(`${path}.canonicalDocument.provenance.project_id`, "does not match value.projectId");
  }

  return {
    documentId,
    runId,
    canonicalDocumentSha256,
    immutable: true,
    canonicalDocumentJson,
    canonicalDocument,
  };
}

/**
 * Treats the native IPC value as untrusted JSON. Historical schema-6 projects
 * with no canonical attachments remain valid and return an empty document set.
 */
export async function parseInternalProjectSchema6ResultReadOutcomeV1(
  input: unknown,
  expectedRequest?: InternalProjectSchema6ResultReadRequestV1,
): Promise<InternalProjectSchema6ResultReadOutcomeV1> {
  const root = recordAt(input, "root");
  if (root.status === "blocked") {
    requireExactKeys(root, BLOCKED_OUTCOME_KEYS, "root");
    const diagnostic = recordAt(root.diagnostic, "root.diagnostic");
    requireExactKeys(diagnostic, DIAGNOSTIC_KEYS, "root.diagnostic");
    return {
      status: "blocked",
      diagnostic: {
        code: nonemptyStringAt(diagnostic.code, "root.diagnostic.code"),
        message: nonemptyStringAt(diagnostic.message, "root.diagnostic.message"),
        correctiveAction: nonemptyStringAt(
          diagnostic.correctiveAction,
          "root.diagnostic.correctiveAction",
        ),
      },
    };
  }
  if (root.status !== "ok") fail("root.status", "must equal ok or blocked");
  requireExactKeys(root, OK_OUTCOME_KEYS, "root");

  const value = recordAt(root.value, "root.value");
  requireExactKeys(value, SNAPSHOT_KEYS, "root.value");
  if (value.schemaVersion !== 1) fail("root.value.schemaVersion", "must equal 1");
  const projectId = nonemptyStringAt(value.projectId, "root.value.projectId");
  const archivePath = nonemptyStringAt(value.archivePath, "root.value.archivePath");
  const sourceDocumentSha256 = sha256At(
    value.sourceDocumentSha256,
    "root.value.sourceDocumentSha256",
  );
  const canonicalResultDocumentCount = countAt(
    value.canonicalResultDocumentCount,
    "root.value.canonicalResultDocumentCount",
  );
  if (!Array.isArray(value.documents)) fail("root.value.documents", "must be an array");
  if (canonicalResultDocumentCount !== value.documents.length) {
    fail("root.value.canonicalResultDocumentCount", "does not match documents.length");
  }
  if (value.sourceRecheckedUnchanged !== true) {
    fail("root.value.sourceRecheckedUnchanged", "must equal true");
  }
  if (expectedRequest) {
    if (archivePath !== expectedRequest.archivePath) {
      fail("root.value.archivePath", "does not match the requested archivePath");
    }
    if (sourceDocumentSha256 !== expectedRequest.expectedSourceSha256) {
      fail(
        "root.value.sourceDocumentSha256",
        "does not match the requested expectedSourceSha256",
      );
    }
  }

  const documents = await Promise.all(
    value.documents.map((entry, index) => parseEntry(entry, `root.value.documents[${index}]`, projectId)),
  );
  const documentIds = documents.map((entry) => entry.documentId);
  const runIds = documents.map((entry) => entry.runId);
  if (new Set(documentIds).size !== documentIds.length) {
    fail("root.value.documents", "contains duplicate documentId values");
  }
  if (new Set(runIds).size !== runIds.length) {
    fail("root.value.documents", "contains duplicate runId values");
  }
  const sortedDocumentIds = [...documentIds].sort();
  if (!documentIds.every((documentId, index) => documentId === sortedDocumentIds[index])) {
    fail("root.value.documents", "must be ordered by documentId");
  }

  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      projectId,
      archivePath,
      sourceDocumentSha256,
      canonicalResultDocumentCount,
      documents,
      sourceRecheckedUnchanged: true,
    },
  };
}
