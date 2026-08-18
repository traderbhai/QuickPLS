import type {
  CapabilityCellReferenceV2,
  CanonicalResultCell,
  CanonicalResultDocumentV2,
  CanonicalResultTable,
} from "./canonicalResultDocumentV2";
import { validateCanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import type {
  AnalysisRecipeV4,
  AnalysisRecipeV4MissingDataPolicy,
  InternalRecipeV4ExecutionFailureV1,
  InternalRecipeV4PlsJobStateV1,
  RecipeV4CompilationReceiptV1,
} from "./internalRecipeV4PlsExecution";
import { compareUtf8StringsV1, type SemCovarianceDenominatorV4, type SemModelV4 } from "./semModelV4";
export type { SemCovarianceDenominatorV4 } from "./semModelV4";
import type {
  CbsemAnalysis,
  CbsemExactCaseBootstrapBcaSidecarV1,
  CbsemExactCaseBootstrapBcaUnavailableReasonV1,
  CbsemCfaScoreLmBundleV1,
  CbsemCfaScoreLmOutcomeV1,
  CbsemExactCaseBootstrapResultV1,
  CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
  CbsemExactCaseBootstrapRefitStandardErrorsV1,
  CbsemExactCaseBootstrapStudentizedSidecarV1,
  CbsemExactCaseBootstrapWithStudentizedResultV1,
  CbsemExactCaseBootstrapWithBcaResultV1,
} from "../types";

export const INTERNAL_RECIPE_V4_CBSEM_COMMAND_SCHEMA_VERSION = 1 as const;
export const CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1 = "cbsem_ml_exact_parameter_table_v3" as const;
export const CBSEM_COMPILED_MOMENT_MEAN_STRUCTURE_METHOD_VERSION_V1 = "cbsem_ml_compiled_moment_input_v4" as const;
export const CBSEM_COMPILED_MOMENT_MEAN_REPLACEMENT_METHOD_VERSION_V1 = "cbsem_ml_compiled_moment_input_mean_replacement_v1" as const;

export type CbsemCompiledMomentMethodVersionV1 =
  | typeof CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1
  | typeof CBSEM_COMPILED_MOMENT_MEAN_STRUCTURE_METHOD_VERSION_V1
  | typeof CBSEM_COMPILED_MOMENT_MEAN_REPLACEMENT_METHOD_VERSION_V1;

export interface InternalRecipeV4CbsemPointCapabilityCellV1 extends CapabilityCellReferenceV2 {
  registry_schema_version: 2;
  capability_id: "smartpls.cbsem";
  cell_id: "qpls3.cbsem.ml";
  capability_version: "cbsem_ml_v1";
}

export interface InternalRecipeV4CbsemBootstrapCapabilityCellV1 extends CapabilityCellReferenceV2 {
  registry_schema_version: 2;
  capability_id: "smartpls.cbsem_bootstrapping";
  cell_id: "qpls3.cbsem.bootstrap";
  capability_version: "cbsem_exact_case_bootstrap_v1";
}

export type InternalRecipeV4CbsemCapabilityCellV1 =
  | InternalRecipeV4CbsemPointCapabilityCellV1
  | InternalRecipeV4CbsemBootstrapCapabilityCellV1;

export const INTERNAL_RECIPE_V4_CBSEM_CAPABILITY_CELL: InternalRecipeV4CbsemPointCapabilityCellV1 = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem",
  cell_id: "qpls3.cbsem.ml",
  capability_version: "cbsem_ml_v1",
};

export const INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL: InternalRecipeV4CbsemBootstrapCapabilityCellV1 = {
  registry_schema_version: 2,
  capability_id: "smartpls.cbsem_bootstrapping",
  cell_id: "qpls3.cbsem.bootstrap",
  capability_version: "cbsem_exact_case_bootstrap_v1",
};

/** Internal-only request. Matrix cells remain resident in the native project. */
export interface InternalLabsRecipeV4CbsemExecutionRequestV1 {
  surface: "standard" | "internal_labs";
  experimentalLabsEnabled: boolean;
  residentData: "project_resident";
  datasetId: string;
  datasetFingerprint: string;
  recipe: AnalysisRecipeV4<AnalysisRecipeV4MissingDataPolicy>;
  model: SemModelV4;
  compilerTarget: "cbsem_plan_v2";
  capabilityCell: InternalRecipeV4CbsemCapabilityCellV1;
}

export interface RecipeV4CbsemCompilationReceiptV1
  extends Omit<RecipeV4CompilationReceiptV1, "compiler_target" | "capability_cell"> {
  compiler_target: "cbsem_plan_v2";
  capability_cell: InternalRecipeV4CbsemCapabilityCellV1;
}

export type CbsemMomentInputKindV2 = "raw" | "covariance" | "correlation";

export const INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_METHOD_VERSION_V1 = "mean_replacement_v1" as const;
export const INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1 = 0.05 as const;
export const INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1 = 0.15 as const;
export const INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_RECEIPT_HASH_DOMAIN_V1 = "quickpls-mean-replacement-receipt-v1\0" as const;

export type MeanReplacementVariableWarningLevelV1 =
  | "none"
  | "at_least_five_percent"
  | "above_fifteen_percent";

export interface MeanReplacementVariableReceiptV1 {
  variable_order: number;
  variable_id: string;
  source_column: string;
  canonical_missing_markers: string[];
  observed_count: number;
  missing_count: number;
  replacement_mean: number;
  missing_fraction: number;
  warning_level: MeanReplacementVariableWarningLevelV1;
}

export interface MeanReplacementCaseReceiptV1 {
  row_index_zero_based: number;
  imputed_variable_ids: string[];
  missing_fraction: number;
  high_missingness_warning: boolean;
}

/**
 * Analysis-time treatment receipt. It binds the completed matrix to the exact
 * resident source without creating a derived dataset or changing lineage.
 * `receipt_sha256` hashes the ordered fields below (excluding itself) after the
 * `INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_RECEIPT_HASH_DOMAIN_V1` byte prefix.
 */
export interface MeanReplacementReceiptV1 {
  method_version: typeof INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_METHOD_VERSION_V1;
  policy: "mean_replacement";
  source_dataset_id: string;
  source_dataset_fingerprint: string;
  source_row_count: number;
  retained_row_count: number;
  omitted_row_count: number;
  modeled_variable_count: number;
  imputed_cell_count: number;
  affected_case_count: number;
  variable_warning_threshold: typeof INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1;
  high_missingness_threshold: typeof INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1;
  variables: MeanReplacementVariableReceiptV1[];
  cases: MeanReplacementCaseReceiptV1[];
  missingness_sha256: string;
  completed_matrix_sha256: string;
  receipt_sha256: string;
}

export interface CbsemMomentInputProvenanceV2 {
  kind: CbsemMomentInputKindV2;
  dataset_id: string;
  dataset_fingerprint: string;
  declared_sample_size: number | null;
  used_sample_size: number;
  omitted_observations: number;
  covariance_denominator: SemCovarianceDenominatorV4;
  variable_ids: string[];
  source_columns: string[];
  standard_deviations: Record<string, number> | null;
  canonical_ml_covariance_sha256: string;
  canonical_observed_means_sha256?: string;
  /** Omitted for listwise and for matrix input. */
  missing_data_treatment?: MeanReplacementReceiptV1;
}

export interface CbsemMeanCellV4 {
  variable: string;
  value: number;
}

export interface CbsemCompiledMomentResultV2 {
  schema_version: 2 | 3 | 4;
  method_version: CbsemCompiledMomentMethodVersionV1;
  compiler_analytical_identity_sha256: string;
  plan_sha256: string;
  model_scientific_sha256: string;
  input: CbsemMomentInputProvenanceV2;
  covariance_ml: number[][];
  parameter_ids: Record<string, string>;
  observed_means?: CbsemMeanCellV4[];
  implied_means?: CbsemMeanCellV4[];
  residual_means?: CbsemMeanCellV4[];
  analysis: CbsemAnalysis;
}

export interface InternalRecipeV4CbsemExecutionProvenanceV1 {
  adapter_version: string;
  compilation_receipt: RecipeV4CbsemCompilationReceiptV1;
  dataset_id: string;
  estimator_method_version: string;
  moment_input_method_version: CbsemCompiledMomentMethodVersionV1;
}

export interface InternalRecipeV4CbsemExecutionResultV1 {
  schema_version: typeof INTERNAL_RECIPE_V4_CBSEM_COMMAND_SCHEMA_VERSION;
  provenance: InternalRecipeV4CbsemExecutionProvenanceV1;
  estimation: CbsemCompiledMomentResultV2;
}

export interface InternalRecipeV4CbsemCompletedResultV1 {
  schemaVersion: typeof INTERNAL_RECIPE_V4_CBSEM_COMMAND_SCHEMA_VERSION;
  analyticalResult: InternalRecipeV4CbsemExecutionResultV1;
  canonicalDocument: CanonicalResultDocumentV2;
}

const CBSEM_SCORE_LM_CANONICAL_COLUMNS = [
  "method_version", "scope", "parameter_id", "kind", "lhs", "rhs", "status",
  "score", "efficient_score", "candidate_information", "efficient_information",
  "modification_index", "expected_parameter_change", "degrees_of_freedom", "p_value",
  "unavailable_reason",
] as const;

const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_CANONICAL_COLUMNS = [
  "method_version", "null_hypothesis", "statistic", "tie_policy", "probability_method",
  "decision_rule", "selected_test_tail", "null_value", "significance_level", "usable_replicates",
  "inference_status", "global_unavailable_reason_code", "global_unavailable_message", "parameter_id",
  "parameter_status", "point_estimate", "two_sided_exceedances", "greater_or_equal_exceedances",
  "less_or_equal_exceedances", "p_value_two_sided", "p_value_greater", "p_value_less",
  "selected_exceedances", "selected_p_value", "reject_null", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_SUMMARY_CANONICAL_COLUMNS = [
  "method_version", "estimator_method_version", "source_dataset_id", "source_dataset_fingerprint",
  "outer_recipe_analytical_identity_sha256", "base_point_result_sha256", "compiler_analytical_identity_sha256",
  "plan_sha256", "model_scientific_sha256", "complete_case_sample_size", "complete_case_universe_digest_method",
  "complete_case_universe_sha256", "covariance_denominator", "sample_indices_digest_method",
  "sampling_positions_digest_method", "interval_method", "confidence_level", "requested_replicates",
  "attempted_refits", "usable_replicates", "failed_replicates", "minimum_usable_fraction",
  "minimum_usable_replicates", "seed_decimal", "stream_token", "retry_policy", "max_attempts_per_replicate",
  "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message", "archive_validation_scope",
] as const;
const CBSEM_EXACT_BOOTSTRAP_INTERVAL_CANONICAL_COLUMNS = [
  "parameter_id", "original", "bootstrap_mean", "bias", "standard_error", "percentile_lower", "percentile_upper", "usable_replicates",
] as const;
const CBSEM_EXACT_BOOTSTRAP_SUCCESS_CANONICAL_COLUMNS = [
  "replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm",
] as const;
const CBSEM_EXACT_BOOTSTRAP_FAILURE_CANONICAL_COLUMNS = [
  "replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "kind", "message",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_CANONICAL_COLUMNS = [
  "method_version", "standard_error_method_version", "expected_information_method", "pivot_method",
  "quantile_method", "interval_method", "archive_validation_scope", "confidence_level",
  "minimum_usable_fraction", "minimum_usable_replicates", "studentized_usable_replicates",
  "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_CANONICAL_COLUMNS = [
  "method_version", "parameter_id", "status", "information_method", "standard_error", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_CANONICAL_COLUMNS = [
  "parameter_id", "status", "point_estimate", "point_standard_error", "lower_pivot_quantile",
  "upper_pivot_quantile", "interval_lower", "interval_upper", "usable_replicates", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_CANONICAL_COLUMNS = [
  "replicate_index", "status", "information_method", "standard_errors_json", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_CANONICAL_COLUMNS = [
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
const CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_CANONICAL_COLUMNS = [
  "parameter_id", "status", "point_estimate", "bias_correction", "acceleration",
  "adjusted_lower_probability", "adjusted_upper_probability", "interval_lower", "interval_upper",
  "usable_replicates", "unavailable_reason",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_CANONICAL_COLUMNS = [
  "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
  "retained_sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm",
] as const;
const CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_CANONICAL_COLUMNS = [
  "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
  "retained_sample_indices_sha256", "kind", "message",
] as const;

const CBSEM_EXACT_BOOTSTRAP_SE_METHOD_V1 = "cbsem_exact_case_bootstrap_refit_standard_errors_v1" as const;
const CBSEM_EXACT_BOOTSTRAP_INFORMATION_METHOD_V1 = "cbsem_ml_expected_information_delta_method_v1" as const;
const CBSEM_EXACT_BOOTSTRAP_SE_UNAVAILABLE_REASONS = [
  "singular_information", "information_not_positive_definite",
  "invalid_information_variance_or_standard_error", "derivative_unavailable",
  "numerical_information_failure",
] as const satisfies readonly CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1[];
const CBSEM_EXACT_BOOTSTRAP_BCA_UNAVAILABLE_REASONS = [
  "base_inference_unavailable", "incomplete_delete_one_ledger",
  "bias_correction_probability_at_boundary", "degenerate_jackknife_acceleration",
  "nonfinite_jackknife_arithmetic", "singular_acceleration_adjustment",
  "invalid_adjusted_probability", "adjusted_probability_order_invalid",
  "nonfinite_or_reversed_interval",
] as const satisfies readonly CbsemExactCaseBootstrapBcaUnavailableReasonV1[];
const CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE =
  "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1" as const;

const CBSEM_WIRE_SHA256 = /^[a-f0-9]{64}$/;

function cbsemWireFail(path: string, message: string): never {
  throw new Error(`Recipe-v4 CB-SEM ${path}: ${message}`);
}

function cbsemWireRecord(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) cbsemWireFail(path, "must be an object");
  return value as Record<string, unknown>;
}

function cbsemWireExactRecord(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): Record<string, unknown> {
  const record = cbsemWireRecord(value, path);
  const allowed = new Set([...required, ...optional]);
  const unknownKeys = Object.keys(record).filter((key) => !allowed.has(key));
  const missingKeys = required.filter((key) => !Object.prototype.hasOwnProperty.call(record, key));
  if (unknownKeys.length || missingKeys.length) cbsemWireFail(
    path,
    `has a drifted key contract${unknownKeys.length ? `; unknown ${unknownKeys.join(", ")}` : ""}${missingKeys.length ? `; missing ${missingKeys.join(", ")}` : ""}`,
  );
  return record;
}

function cbsemWireText(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) cbsemWireFail(path, "must be a nonempty string");
  return value;
}

function cbsemWireCount(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) cbsemWireFail(path, "must be a nonnegative safe integer");
  return value as number;
}

function cbsemWireBoolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") cbsemWireFail(path, "must be a boolean");
  return value;
}

function cbsemWireFinite(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) cbsemWireFail(path, "must be finite");
  return value;
}

function cbsemWirePositiveZeroFinite(value: unknown, path: string): number {
  const number = cbsemWireFinite(value, path);
  if (Object.is(number, -0)) cbsemWireFail(path, "must use canonical positive zero");
  return number;
}

export function cbsemCfaScoreLmNumbersCloseV1(left: number, right: number): boolean {
  return Number.isFinite(left)
    && Number.isFinite(right)
    && Math.abs(left - right) <= 64 * Number.EPSILON * Math.max(1, Math.abs(left), Math.abs(right));
}

function cbsemScoreLmLogGamma(value: number): number {
  const coefficients = [
    676.5203681218851, -1259.1392167224028, 771.3234287776531,
    -176.6150291621406, 12.507343278686905, -0.13857109526572012,
    9.984369578019572e-6, 1.5056327351493116e-7,
  ];
  if (value < 0.5) return Math.log(Math.PI) - Math.log(Math.sin(Math.PI * value)) - cbsemScoreLmLogGamma(1 - value);
  const shifted = value - 1;
  let series = 0.9999999999998099;
  coefficients.forEach((coefficient, index) => { series += coefficient / (shifted + index + 1); });
  const base = shifted + coefficients.length - 0.5;
  return 0.5 * Math.log(2 * Math.PI) + (shifted + 0.5) * Math.log(base) - base + Math.log(series);
}

/** Deterministic chi-square(1) survival used only to verify the persisted engine value. */
export function cbsemCfaScoreLmChiSquare1PValueV1(value: number): number {
  if (!Number.isFinite(value) || value < 0) return Number.NaN;
  if (value === 0) return 1;
  const shape = 0.5;
  const scaled = value / 2;
  const logScale = -scaled + shape * Math.log(scaled) - cbsemScoreLmLogGamma(shape);
  if (scaled < shape + 1) {
    let term = 1 / shape;
    let sum = term;
    let denominator = shape;
    for (let iteration = 1; iteration <= 200; iteration += 1) {
      denominator += 1;
      term *= scaled / denominator;
      sum += term;
      if (Math.abs(term) <= Math.abs(sum) * 3e-14) break;
    }
    return Math.min(1, Math.max(0, 1 - sum * Math.exp(logScale)));
  }
  let offset = scaled + 1 - shape;
  let numerator = 1 / 1e-300;
  let denominator = 1 / Math.max(Math.abs(offset), 1e-300) * Math.sign(offset || 1);
  let fraction = denominator;
  for (let iteration = 1; iteration <= 200; iteration += 1) {
    const coefficient = -iteration * (iteration - shape);
    offset += 2;
    denominator = coefficient * denominator + offset;
    if (Math.abs(denominator) < 1e-300) denominator = 1e-300;
    numerator = offset + coefficient / numerator;
    if (Math.abs(numerator) < 1e-300) numerator = 1e-300;
    denominator = 1 / denominator;
    const change = denominator * numerator;
    fraction *= change;
    if (Math.abs(change - 1) <= 3e-14) break;
  }
  return Math.min(1, Math.max(0, Math.exp(logScale) * fraction));
}

export function parseCbsemCfaScoreLmBundleV1(
  value: unknown,
  path = "result.estimation.analysis.score_lm",
): CbsemCfaScoreLmBundleV1 {
  const bundle = cbsemWireExactRecord(value, ["method_version", "scope", "rows"], [], path);
  if (bundle.method_version !== "cbsem_cfa_score_lm_v1") cbsemWireFail(`${path}.method_version`, "must equal cbsem_cfa_score_lm_v1");
  if (bundle.scope !== "covariance_only_declared_zero_residual_covariances") cbsemWireFail(`${path}.scope`, "has a drifted scientific scope");
  if (!Array.isArray(bundle.rows)) cbsemWireFail(`${path}.rows`, "must be an array");
  let previousParameterId: string | null = null;
  const rows = bundle.rows.map((value, index) => {
    const rowPath = `${path}.rows[${index}]`;
    const row = cbsemWireExactRecord(value, ["parameter_id", "kind", "lhs", "rhs", "outcome"], [], rowPath);
    const parameterId = cbsemWireText(row.parameter_id, `${rowPath}.parameter_id`);
    if (previousParameterId !== null && compareUtf8StringsV1(previousParameterId, parameterId) >= 0) cbsemWireFail(`${path}.rows`, "must be in stable parameter-id order without duplicates");
    previousParameterId = parameterId;
    if (row.kind !== "residual_covariance") cbsemWireFail(`${rowPath}.kind`, "must equal residual_covariance");
    const lhs = cbsemWireText(row.lhs, `${rowPath}.lhs`);
    const rhs = cbsemWireText(row.rhs, `${rowPath}.rhs`);
    if (lhs === rhs) cbsemWireFail(rowPath, "must identify an off-diagonal residual covariance");
    const outcomeRecord = cbsemWireRecord(row.outcome, `${rowPath}.outcome`);
    let outcome: CbsemCfaScoreLmOutcomeV1;
    if (outcomeRecord.status === "available") {
      const available = cbsemWireExactRecord(outcomeRecord, [
        "status", "score", "efficient_score", "candidate_information", "efficient_information",
        "modification_index", "expected_parameter_change", "p_value",
      ], [], `${rowPath}.outcome`);
      const score = cbsemWirePositiveZeroFinite(available.score, `${rowPath}.outcome.score`);
      const efficientScore = cbsemWirePositiveZeroFinite(available.efficient_score, `${rowPath}.outcome.efficient_score`);
      const candidateInformation = cbsemWirePositiveZeroFinite(available.candidate_information, `${rowPath}.outcome.candidate_information`);
      const efficientInformation = cbsemWirePositiveZeroFinite(available.efficient_information, `${rowPath}.outcome.efficient_information`);
      const modificationIndex = cbsemWirePositiveZeroFinite(available.modification_index, `${rowPath}.outcome.modification_index`);
      const expectedParameterChange = cbsemWirePositiveZeroFinite(available.expected_parameter_change, `${rowPath}.outcome.expected_parameter_change`);
      const pValue = cbsemWirePositiveZeroFinite(available.p_value, `${rowPath}.outcome.p_value`);
      if (candidateInformation <= 0 || efficientInformation <= 0 || modificationIndex < 0 || pValue < 0 || pValue > 1
        || !Object.is(modificationIndex, efficientScore * efficientScore / efficientInformation)
        || !Object.is(expectedParameterChange, efficientScore / efficientInformation)
        || !cbsemCfaScoreLmNumbersCloseV1(pValue, cbsemCfaScoreLmChiSquare1PValueV1(modificationIndex))) {
        cbsemWireFail(`${rowPath}.outcome`, "has incoherent score/LM arithmetic or chi-square(1) probability");
      }
      outcome = { status: "available", score, efficient_score: efficientScore, candidate_information: candidateInformation, efficient_information: efficientInformation, modification_index: modificationIndex, expected_parameter_change: expectedParameterChange, p_value: pValue };
    } else if (outcomeRecord.status === "unavailable") {
      const unavailable = cbsemWireExactRecord(outcomeRecord, ["status", "reason"], [], `${rowPath}.outcome`);
      const reason = unavailable.reason;
      if (reason !== "nuisance_information_unavailable" && reason !== "efficient_information_non_positive" && reason !== "non_finite_computation") cbsemWireFail(`${rowPath}.outcome.reason`, "has an unknown unavailable reason");
      outcome = { status: "unavailable", reason };
    } else cbsemWireFail(`${rowPath}.outcome.status`, "must equal available or unavailable");
    return { parameter_id: parameterId, kind: "residual_covariance" as const, lhs, rhs, outcome };
  });
  return { method_version: "cbsem_cfa_score_lm_v1", scope: "covariance_only_declared_zero_residual_covariances", rows };
}

function cbsemWireSha256(value: unknown, path: string): string {
  const digest = cbsemWireText(value, path);
  if (!CBSEM_WIRE_SHA256.test(digest)) cbsemWireFail(path, "must be a lowercase SHA-256");
  return digest;
}

function cbsemWireFraction(value: unknown, expected: number, path: string): number {
  const fraction = cbsemWireFinite(value, path);
  if (fraction < 0 || fraction > 1 || Math.abs(fraction - expected) > 1e-12) {
    cbsemWireFail(path, `must equal the exact count-derived fraction ${expected}`);
  }
  return fraction;
}

function cbsemWireTextArray(value: unknown, path: string): string[] {
  if (!Array.isArray(value)) cbsemWireFail(path, "must be an array");
  const values = value.map((item, index) => cbsemWireText(item, `${path}[${index}]`));
  if (new Set(values).size !== values.length) cbsemWireFail(path, "must not contain duplicate identities");
  return values;
}

function cbsemWireMarkerArray(value: unknown, path: string): string[] {
  if (!Array.isArray(value)) cbsemWireFail(path, "must be an array");
  const values = value.map((item, index) => {
    if (typeof item !== "string" || !item || item.trim() !== item) cbsemWireFail(`${path}[${index}]`, "must be a canonical nonempty marker");
    return item;
  });
  if (values.some((marker, index) => index > 0 && compareUtf8StringsV1(values[index - 1]!, marker) >= 0)) cbsemWireFail(path, "must be sorted and deduplicated");
  return values;
}

function expectedVariableWarningLevel(missingFraction: number): MeanReplacementVariableWarningLevelV1 {
  if (missingFraction > INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1) return "above_fifteen_percent";
  if (missingFraction >= INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1) return "at_least_five_percent";
  return "none";
}

/** Strict parser for the native analysis-time mean-replacement receipt. */
export function parseMeanReplacementReceiptV1(input: unknown): MeanReplacementReceiptV1 {
  const receipt = cbsemWireExactRecord(input, [
    "method_version",
    "policy",
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
    "variables",
    "cases",
    "missingness_sha256",
    "completed_matrix_sha256",
    "receipt_sha256",
  ], [], "meanReplacementReceipt");
  if (receipt.method_version !== INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_METHOD_VERSION_V1) cbsemWireFail("meanReplacementReceipt.method_version", "must equal mean_replacement_v1");
  if (receipt.policy !== "mean_replacement") cbsemWireFail("meanReplacementReceipt.policy", "must equal mean_replacement");
  const sourceDatasetId = cbsemWireText(receipt.source_dataset_id, "meanReplacementReceipt.source_dataset_id");
  const sourceDatasetFingerprint = cbsemWireText(receipt.source_dataset_fingerprint, "meanReplacementReceipt.source_dataset_fingerprint");
  const sourceRowCount = cbsemWireCount(receipt.source_row_count, "meanReplacementReceipt.source_row_count");
  const retainedRowCount = cbsemWireCount(receipt.retained_row_count, "meanReplacementReceipt.retained_row_count");
  const omittedRowCount = cbsemWireCount(receipt.omitted_row_count, "meanReplacementReceipt.omitted_row_count");
  const modeledVariableCount = cbsemWireCount(receipt.modeled_variable_count, "meanReplacementReceipt.modeled_variable_count");
  const imputedCellCount = cbsemWireCount(receipt.imputed_cell_count, "meanReplacementReceipt.imputed_cell_count");
  const affectedCaseCount = cbsemWireCount(receipt.affected_case_count, "meanReplacementReceipt.affected_case_count");
  if (sourceRowCount === 0 || modeledVariableCount === 0) cbsemWireFail("meanReplacementReceipt", "must cover at least one source row and modeled variable");
  if (retainedRowCount !== sourceRowCount || omittedRowCount !== 0) cbsemWireFail("meanReplacementReceipt", "mean replacement must retain every source row and omit none");
  if (receipt.variable_warning_threshold !== INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1) cbsemWireFail("meanReplacementReceipt.variable_warning_threshold", "must equal 0.05");
  if (receipt.high_missingness_threshold !== INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1) cbsemWireFail("meanReplacementReceipt.high_missingness_threshold", "must equal 0.15");
  if (!Array.isArray(receipt.variables) || receipt.variables.length !== modeledVariableCount) cbsemWireFail("meanReplacementReceipt.variables", "must exactly cover every modeled variable");

  const variableIds = new Set<string>();
  const variableOrder = new Map<string, number>();
  const missingByVariable = new Map<string, number>();
  const variables = receipt.variables.map((item, index) => {
    const path = `meanReplacementReceipt.variables[${index}]`;
    const variable = cbsemWireExactRecord(item, [
      "variable_order",
      "variable_id",
      "source_column",
      "canonical_missing_markers",
      "observed_count",
      "missing_count",
      "replacement_mean",
      "missing_fraction",
      "warning_level",
    ], [], path);
    const order = cbsemWireCount(variable.variable_order, `${path}.variable_order`);
    if (order !== index) cbsemWireFail(`${path}.variable_order`, "must equal its canonical zero-based array position");
    const variableId = cbsemWireText(variable.variable_id, `${path}.variable_id`);
    if (variableIds.has(variableId)) cbsemWireFail(`${path}.variable_id`, "must be unique");
    variableIds.add(variableId);
    variableOrder.set(variableId, index);
    const sourceColumn = cbsemWireText(variable.source_column, `${path}.source_column`);
    const canonicalMissingMarkers = cbsemWireMarkerArray(variable.canonical_missing_markers, `${path}.canonical_missing_markers`);
    const observedCount = cbsemWireCount(variable.observed_count, `${path}.observed_count`);
    const missingCount = cbsemWireCount(variable.missing_count, `${path}.missing_count`);
    if (observedCount === 0 || observedCount + missingCount !== sourceRowCount) cbsemWireFail(path, "must have a replacement mean backed by observed rows and exact source-row accounting");
    const replacementMean = cbsemWireFinite(variable.replacement_mean, `${path}.replacement_mean`);
    const missingFraction = cbsemWireFraction(variable.missing_fraction, missingCount / sourceRowCount, `${path}.missing_fraction`);
    const warningLevel = expectedVariableWarningLevel(missingFraction);
    if (variable.warning_level !== warningLevel) cbsemWireFail(`${path}.warning_level`, `must equal ${warningLevel}`);
    missingByVariable.set(variableId, missingCount);
    return {
      variable_order: order,
      variable_id: variableId,
      source_column: sourceColumn,
      canonical_missing_markers: canonicalMissingMarkers,
      observed_count: observedCount,
      missing_count: missingCount,
      replacement_mean: replacementMean,
      missing_fraction: missingFraction,
      warning_level: warningLevel,
    };
  });

  if (!Array.isArray(receipt.cases) || receipt.cases.length !== affectedCaseCount) cbsemWireFail("meanReplacementReceipt.cases", "must exactly cover every affected case");
  const caseMissingByVariable = new Map<string, number>(variables.map((variable) => [variable.variable_id, 0]));
  let previousRowIndex = -1;
  let caseImputedCellCount = 0;
  const cases = receipt.cases.map((item, index) => {
    const path = `meanReplacementReceipt.cases[${index}]`;
    const entry = cbsemWireExactRecord(item, ["row_index_zero_based", "imputed_variable_ids", "missing_fraction", "high_missingness_warning"], [], path);
    const rowIndex = cbsemWireCount(entry.row_index_zero_based, `${path}.row_index_zero_based`);
    if (rowIndex >= sourceRowCount || rowIndex <= previousRowIndex) cbsemWireFail(`${path}.row_index_zero_based`, "must be unique, ascending, and within the source rows");
    previousRowIndex = rowIndex;
    const imputedVariableIds = cbsemWireTextArray(entry.imputed_variable_ids, `${path}.imputed_variable_ids`);
    if (imputedVariableIds.length === 0) cbsemWireFail(`${path}.imputed_variable_ids`, "must name at least one imputed variable");
    let previousVariableOrder = -1;
    for (const variableId of imputedVariableIds) {
      const order = variableOrder.get(variableId);
      if (order == null || order <= previousVariableOrder) cbsemWireFail(`${path}.imputed_variable_ids`, "must follow canonical modeled-variable order");
      previousVariableOrder = order;
      caseMissingByVariable.set(variableId, (caseMissingByVariable.get(variableId) ?? 0) + 1);
    }
    caseImputedCellCount += imputedVariableIds.length;
    const missingFraction = cbsemWireFraction(entry.missing_fraction, imputedVariableIds.length / modeledVariableCount, `${path}.missing_fraction`);
    const highMissingnessWarning = missingFraction > INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1;
    if (entry.high_missingness_warning !== highMissingnessWarning) cbsemWireFail(`${path}.high_missingness_warning`, `must equal ${highMissingnessWarning}`);
    return {
      row_index_zero_based: rowIndex,
      imputed_variable_ids: imputedVariableIds,
      missing_fraction: missingFraction,
      high_missingness_warning: highMissingnessWarning,
    };
  });
  const variableImputedCellCount = variables.reduce((total, variable) => total + variable.missing_count, 0);
  if (imputedCellCount !== variableImputedCellCount || imputedCellCount !== caseImputedCellCount) cbsemWireFail("meanReplacementReceipt.imputed_cell_count", "differs from variable or case accounting");
  for (const [variableId, missingCount] of missingByVariable) {
    if (caseMissingByVariable.get(variableId) !== missingCount) cbsemWireFail("meanReplacementReceipt.cases", `does not reproduce missing-cell accounting for ${variableId}`);
  }
  const missingnessSha256 = cbsemWireSha256(receipt.missingness_sha256, "meanReplacementReceipt.missingness_sha256");
  const completedMatrixSha256 = cbsemWireSha256(receipt.completed_matrix_sha256, "meanReplacementReceipt.completed_matrix_sha256");
  const receiptSha256 = cbsemWireSha256(receipt.receipt_sha256, "meanReplacementReceipt.receipt_sha256");
  return {
    method_version: INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_METHOD_VERSION_V1,
    policy: "mean_replacement",
    source_dataset_id: sourceDatasetId,
    source_dataset_fingerprint: sourceDatasetFingerprint,
    source_row_count: sourceRowCount,
    retained_row_count: retainedRowCount,
    omitted_row_count: omittedRowCount,
    modeled_variable_count: modeledVariableCount,
    imputed_cell_count: imputedCellCount,
    affected_case_count: affectedCaseCount,
    variable_warning_threshold: INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1,
    high_missingness_threshold: INTERNAL_RECIPE_V4_MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1,
    variables,
    cases,
    missingness_sha256: missingnessSha256,
    completed_matrix_sha256: completedMatrixSha256,
    receipt_sha256: receiptSha256,
  };
}

function parseCbsemCapabilityCellV1(value: unknown, path: string): InternalRecipeV4CbsemCapabilityCellV1 {
  const cell = cbsemWireExactRecord(value, ["registry_schema_version", "capability_id", "cell_id", "capability_version"], [], path);
  if (cell.registry_schema_version !== 2) cbsemWireFail(path, "has an unsupported registry schema");
  if (cell.capability_id === "smartpls.cbsem" && cell.cell_id === "qpls3.cbsem.ml" && cell.capability_version === "cbsem_ml_v1") {
    return INTERNAL_RECIPE_V4_CBSEM_CAPABILITY_CELL;
  }
  if (cell.capability_id === "smartpls.cbsem_bootstrapping" && cell.cell_id === "qpls3.cbsem.bootstrap" && cell.capability_version === "cbsem_exact_case_bootstrap_v1") {
    return INTERNAL_RECIPE_V4_CBSEM_BOOTSTRAP_CAPABILITY_CELL;
  }
  cbsemWireFail(path, "differs from the frozen point-ML and exact-bootstrap CB-SEM capability cells");
}

function parseCbsemCompilationReceiptV1(value: unknown): RecipeV4CbsemCompilationReceiptV1 {
  const path = "result.provenance.compilation_receipt";
  const receipt = cbsemWireExactRecord(value, [
    "schema_version", "recipe_id", "recipe_document_sha256", "recipe_analytical_sha256", "model_id",
    "model_document_sha256", "model_scientific_sha256", "dataset_fingerprint", "compiler_target",
    "compiler_version", "capability_cell", "plan_sha256", "analytical_identity_sha256",
  ], [], path);
  if (receipt.schema_version !== 1 || receipt.compiler_target !== "cbsem_plan_v2") cbsemWireFail(path, "has an unsupported schema or compiler target");
  for (const key of ["recipe_id", "model_id", "dataset_fingerprint", "compiler_version"] as const) cbsemWireText(receipt[key], `${path}.${key}`);
  for (const key of ["recipe_document_sha256", "recipe_analytical_sha256", "model_document_sha256", "model_scientific_sha256", "plan_sha256", "analytical_identity_sha256"] as const) cbsemWireSha256(receipt[key], `${path}.${key}`);
  parseCbsemCapabilityCellV1(receipt.capability_cell, `${path}.capability_cell`);
  return value as RecipeV4CbsemCompilationReceiptV1;
}

function parseCbsemMeanCells(value: unknown, path: string): CbsemMeanCellV4[] {
  if (!Array.isArray(value)) cbsemWireFail(path, "must be an array");
  const seen = new Set<string>();
  return value.map((item, index) => {
    const cell = cbsemWireExactRecord(item, ["variable", "value"], [], `${path}[${index}]`);
    const variable = cbsemWireText(cell.variable, `${path}[${index}].variable`);
    if (seen.has(variable)) cbsemWireFail(`${path}[${index}].variable`, "must be unique");
    seen.add(variable);
    return { variable, value: cbsemWireFinite(cell.value, `${path}[${index}].value`) };
  });
}

function parseCbsemMomentInputV2(value: unknown): CbsemMomentInputProvenanceV2 {
  const path = "result.estimation.input";
  const input = cbsemWireExactRecord(value, [
    "kind", "dataset_id", "dataset_fingerprint", "declared_sample_size", "used_sample_size", "omitted_observations",
    "covariance_denominator", "variable_ids", "source_columns", "standard_deviations", "canonical_ml_covariance_sha256",
  ], ["canonical_observed_means_sha256", "missing_data_treatment"], path);
  if (input.kind !== "raw" && input.kind !== "covariance" && input.kind !== "correlation") cbsemWireFail(`${path}.kind`, "is unsupported");
  const kind = input.kind;
  const datasetId = cbsemWireText(input.dataset_id, `${path}.dataset_id`);
  const datasetFingerprint = cbsemWireText(input.dataset_fingerprint, `${path}.dataset_fingerprint`);
  const declaredSampleSize = input.declared_sample_size === null ? null : cbsemWireCount(input.declared_sample_size, `${path}.declared_sample_size`);
  const usedSampleSize = cbsemWireCount(input.used_sample_size, `${path}.used_sample_size`);
  const omittedObservations = cbsemWireCount(input.omitted_observations, `${path}.omitted_observations`);
  if (usedSampleSize === 0 || (kind === "raw" ? declaredSampleSize !== null : declaredSampleSize === null)) cbsemWireFail(path, "has invalid raw/matrix sample-size semantics");
  if (input.covariance_denominator !== "sample_n_minus_one" && input.covariance_denominator !== "maximum_likelihood_n") cbsemWireFail(`${path}.covariance_denominator`, "is unsupported");
  const variableIds = cbsemWireTextArray(input.variable_ids, `${path}.variable_ids`);
  const sourceColumns = cbsemWireTextArray(input.source_columns, `${path}.source_columns`);
  if (!variableIds.length || variableIds.length !== sourceColumns.length) cbsemWireFail(path, "must bind one source column per modeled variable");
  let standardDeviations: Record<string, number> | null = null;
  if (input.standard_deviations !== null) {
    const record = cbsemWireRecord(input.standard_deviations, `${path}.standard_deviations`);
    if (Object.keys(record).length !== variableIds.length || variableIds.some((id) => !Object.prototype.hasOwnProperty.call(record, id))) cbsemWireFail(`${path}.standard_deviations`, "must exactly cover modeled variable IDs");
    standardDeviations = Object.fromEntries(variableIds.map((id) => {
      const scale = cbsemWireFinite(record[id], `${path}.standard_deviations.${id}`);
      if (scale <= 0) cbsemWireFail(`${path}.standard_deviations.${id}`, "must be positive");
      return [id, scale];
    }));
  }
  if ((kind === "correlation") !== (standardDeviations !== null)) cbsemWireFail(`${path}.standard_deviations`, "must be present only for correlation input");
  const covarianceSha256 = cbsemWireSha256(input.canonical_ml_covariance_sha256, `${path}.canonical_ml_covariance_sha256`);
  const meansSha256 = "canonical_observed_means_sha256" in input
    ? cbsemWireSha256(input.canonical_observed_means_sha256, `${path}.canonical_observed_means_sha256`)
    : undefined;
  const treatment = "missing_data_treatment" in input ? parseMeanReplacementReceiptV1(input.missing_data_treatment) : undefined;
  if (treatment) {
    if (kind !== "raw" || treatment.source_dataset_id !== datasetId || treatment.source_dataset_fingerprint !== datasetFingerprint) cbsemWireFail(`${path}.missing_data_treatment`, "does not bind the exact raw source identity");
    if (treatment.retained_row_count !== usedSampleSize || treatment.omitted_row_count !== omittedObservations || treatment.source_row_count !== usedSampleSize + omittedObservations) cbsemWireFail(`${path}.missing_data_treatment`, "does not match moment sample accounting");
    if (treatment.variables.length !== variableIds.length || treatment.variables.some((variable, index) => variable.variable_id !== variableIds[index] || variable.source_column !== sourceColumns[index])) cbsemWireFail(`${path}.missing_data_treatment.variables`, "does not match canonical moment variable order");
  }
  return {
    kind,
    dataset_id: datasetId,
    dataset_fingerprint: datasetFingerprint,
    declared_sample_size: declaredSampleSize,
    used_sample_size: usedSampleSize,
    omitted_observations: omittedObservations,
    covariance_denominator: input.covariance_denominator,
    variable_ids: variableIds,
    source_columns: sourceColumns,
    standard_deviations: standardDeviations,
    canonical_ml_covariance_sha256: covarianceSha256,
    ...(meansSha256 ? { canonical_observed_means_sha256: meansSha256 } : {}),
    ...(treatment ? { missing_data_treatment: treatment } : {}),
  };
}

function cbsemExactBootstrapType7(sorted: readonly number[], probability: number): number {
  const position = probability * (sorted.length - 1);
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  return lower === upper ? sorted[lower] : sorted[lower] + (position - lower) * (sorted[upper] - sorted[lower]);
}

function cbsemExactBootstrapSourceFingerprintPayloadV1(momentInputFingerprint: string): string {
  return /^v2:[0-9a-f]{64}$/.test(momentInputFingerprint)
    ? momentInputFingerprint.slice(3)
    : momentInputFingerprint;
}

function parseCbsemExactCaseBootstrapResultV1(
  value: unknown,
  path = "result.estimation.analysis.exact_case_bootstrap",
): CbsemExactCaseBootstrapResultV1 {
  const raw = cbsemWireExactRecord(value, [
    "method_version", "estimator_method_version", "source_dataset_id", "source_dataset_fingerprint",
    "outer_recipe_analytical_identity_sha256", "base_point_result_sha256", "compiler_analytical_identity_sha256",
    "plan_sha256", "model_scientific_sha256", "complete_case_sample_size",
    "complete_case_universe_digest_method", "complete_case_universe_sha256", "covariance_denominator",
    "sample_indices_digest_method", "sampling_positions_digest_method", "interval_method", "confidence_level",
    "requested_replicates", "attempted_refits", "usable_replicates", "failed_replicates",
    "minimum_usable_fraction", "minimum_usable_replicates", "seed", "stream_token", "retry_policy",
    "max_attempts_per_replicate", "parameter_ids", "inference", "intervals", "successful_refits", "failed_refits",
  ], ["hypothesis_tests"], path);
  if (raw.method_version !== "cbsem_exact_case_bootstrap_v1"
    || raw.estimator_method_version !== "cbsem_ml_exact_parameter_table_v3"
    || raw.complete_case_universe_digest_method !== "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1"
    || raw.covariance_denominator !== "maximum_likelihood_n"
    || raw.sample_indices_digest_method !== "sha256_source_fingerprint_and_ordered_u64_indices_v1"
    || raw.sampling_positions_digest_method !== "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1"
    || raw.interval_method !== "percentile_type7_v1" || raw.confidence_level !== 0.95
    || raw.stream_token !== "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1"
    || raw.retry_policy !== "no_retry_fixed_preplanned_primary_draws_v1"
    || raw.max_attempts_per_replicate !== 1) cbsemWireFail(path, "has a drifted exact case-bootstrap method contract");
  cbsemWireText(raw.source_dataset_id, `${path}.source_dataset_id`);
  for (const [field, digest] of Object.entries({
    source_dataset_fingerprint: raw.source_dataset_fingerprint,
    outer_recipe_analytical_identity_sha256: raw.outer_recipe_analytical_identity_sha256,
    base_point_result_sha256: raw.base_point_result_sha256,
    compiler_analytical_identity_sha256: raw.compiler_analytical_identity_sha256,
    plan_sha256: raw.plan_sha256,
    model_scientific_sha256: raw.model_scientific_sha256,
    complete_case_universe_sha256: raw.complete_case_universe_sha256,
  })) cbsemWireSha256(digest, `${path}.${field}`);
  const completeCases = cbsemWireCount(raw.complete_case_sample_size, `${path}.complete_case_sample_size`);
  const requested = cbsemWireCount(raw.requested_replicates, `${path}.requested_replicates`);
  const attempted = cbsemWireCount(raw.attempted_refits, `${path}.attempted_refits`);
  const usable = cbsemWireCount(raw.usable_replicates, `${path}.usable_replicates`);
  const failed = cbsemWireCount(raw.failed_replicates, `${path}.failed_replicates`);
  const minimumUsable = cbsemWireCount(raw.minimum_usable_replicates, `${path}.minimum_usable_replicates`);
  cbsemWireCount(raw.seed, `${path}.seed`);
  if (completeCases < 10 || requested < 500 || requested > 10_000 || attempted !== requested
    || usable + failed !== requested || raw.minimum_usable_fraction !== 0.9
    || minimumUsable !== Math.max(1_000, Math.ceil(0.9 * requested))) {
    cbsemWireFail(path, "has incoherent sample, plan, accounting, or usable-refit threshold");
  }
  if (!Array.isArray(raw.parameter_ids) || !raw.parameter_ids.length) cbsemWireFail(`${path}.parameter_ids`, "must be a nonempty array");
  const parameterIds = raw.parameter_ids.map((id, index) => cbsemWireText(id, `${path}.parameter_ids[${index}]`));
  if (new Set(parameterIds).size !== parameterIds.length
    || parameterIds.some((id, index) => index > 0 && compareUtf8StringsV1(parameterIds[index - 1], id) >= 0)) {
    cbsemWireFail(`${path}.parameter_ids`, "must be unique and in stable UTF-8 order");
  }
  if (!Array.isArray(raw.successful_refits) || !Array.isArray(raw.failed_refits) || !Array.isArray(raw.intervals)) {
    cbsemWireFail(path, "must retain interval, successful-refit, and failed-refit arrays");
  }
  const successfulRefitRows: unknown[] = raw.successful_refits;
  const failedRefitRows: unknown[] = raw.failed_refits;
  const intervalRows: unknown[] = raw.intervals;
  let priorSuccessfulReplicateIndex = -1;
  const successfulRefits = successfulRefitRows.map((entry, ordinal) => {
    const rowPath = `${path}.successful_refits[${ordinal}]`;
    const row = cbsemWireExactRecord(entry, ["replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "parameter_estimates", "iterations", "objective", "gradient_norm"], [], rowPath);
    const replicateIndex = cbsemWireCount(row.replicate_index, `${rowPath}.replicate_index`);
    cbsemWireSha256(row.sampling_positions_sha256, `${rowPath}.sampling_positions_sha256`);
    cbsemWireSha256(row.sample_indices_sha256, `${rowPath}.sample_indices_sha256`);
    if (!Array.isArray(row.parameter_estimates) || row.parameter_estimates.length !== parameterIds.length) cbsemWireFail(`${rowPath}.parameter_estimates`, "has drifted parameter width");
    const estimates = row.parameter_estimates.map((estimate, index) => cbsemWireFinite(estimate, `${rowPath}.parameter_estimates[${index}]`));
    const iterations = cbsemWireCount(row.iterations, `${rowPath}.iterations`);
    const objective = cbsemWireFinite(row.objective, `${rowPath}.objective`);
    const gradient = cbsemWireFinite(row.gradient_norm, `${rowPath}.gradient_norm`);
    if (replicateIndex >= requested || (ordinal > 0 && replicateIndex <= priorSuccessfulReplicateIndex)
      || iterations === 0 || objective < 0 || gradient < 0) cbsemWireFail(rowPath, "has invalid order or convergence values");
    priorSuccessfulReplicateIndex = replicateIndex;
    return { replicateIndex, estimates };
  });
  let priorFailedReplicateIndex = -1;
  const failedRefits = failedRefitRows.map((entry, ordinal) => {
    const rowPath = `${path}.failed_refits[${ordinal}]`;
    const row = cbsemWireExactRecord(entry, ["replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "kind", "message"], [], rowPath);
    const replicateIndex = cbsemWireCount(row.replicate_index, `${rowPath}.replicate_index`);
    cbsemWireSha256(row.sampling_positions_sha256, `${rowPath}.sampling_positions_sha256`);
    cbsemWireSha256(row.sample_indices_sha256, `${rowPath}.sample_indices_sha256`);
    if (replicateIndex >= requested || (ordinal > 0 && replicateIndex <= priorFailedReplicateIndex)
      || !["moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure"].includes(String(row.kind))
      || !cbsemWireText(row.message, `${rowPath}.message`).trim()) cbsemWireFail(rowPath, "has invalid order, kind, or message");
    priorFailedReplicateIndex = replicateIndex;
    return replicateIndex;
  });
  const partition = [...successfulRefits.map((row) => row.replicateIndex), ...failedRefits].sort((left, right) => left - right);
  if (successfulRefits.length !== usable || failedRefits.length !== failed
    || partition.length !== requested || partition.some((index, ordinal) => index !== ordinal)) cbsemWireFail(path, "does not form the exact preplanned success/failure partition");
  const globallyAvailable = usable >= minimumUsable;
  const inference = cbsemWireRecord(raw.inference, `${path}.inference`);
  if (globallyAvailable) {
    if (cbsemWireExactRecord(inference, ["status"], [], `${path}.inference`).status !== "available") {
      cbsemWireFail(`${path}.inference`, "differs from the usable-refit threshold");
    }
  } else {
    const unavailable = cbsemWireExactRecord(inference, ["status", "reason_code", "message"], [], `${path}.inference`);
    if (unavailable.status !== "unavailable" || unavailable.reason_code !== "insufficient_usable_refits"
      || !cbsemWireText(unavailable.message, `${path}.inference.message`).trim()) {
      cbsemWireFail(`${path}.inference`, "differs from the usable-refit threshold");
    }
  }
  if (intervalRows.length !== (globallyAvailable ? parameterIds.length : 0)) cbsemWireFail(`${path}.intervals`, "has drifted availability cardinality");
  const originals: number[] = [];
  intervalRows.forEach((entry, parameterIndex) => {
    const rowPath = `${path}.intervals[${parameterIndex}]`;
    const row = cbsemWireExactRecord(entry, ["parameter_id", "original", "bootstrap_mean", "bias", "standard_error", "percentile_lower", "percentile_upper", "usable_replicates"], [], rowPath);
    if (row.parameter_id !== parameterIds[parameterIndex] || cbsemWireCount(row.usable_replicates, `${rowPath}.usable_replicates`) !== usable) cbsemWireFail(rowPath, "has drifted parameter or usable-refit binding");
    const original = cbsemWireFinite(row.original, `${rowPath}.original`);
    const values = successfulRefits.map((refit) => refit.estimates[parameterIndex]);
    const mean = values.reduce((sum, item) => sum + item, 0) / values.length;
    const se = Math.sqrt(values.reduce((sum, item) => sum + (item - mean) ** 2, 0) / (values.length - 1));
    const sorted = [...values].sort((left, right) => left - right);
    const expected = [mean, mean - original, se, cbsemExactBootstrapType7(sorted, 0.025000000000000022), cbsemExactBootstrapType7(sorted, 0.975)];
    const observed = [row.bootstrap_mean, row.bias, row.standard_error, row.percentile_lower, row.percentile_upper]
      .map((item, index) => cbsemWireFinite(item, `${rowPath}[${index}]`));
    if (observed.some((item, index) => !Object.is(item, expected[index]))) cbsemWireFail(rowPath, "has drifted sample-SD, bias, or Type-7 arithmetic");
    originals.push(original);
  });
  if (Object.prototype.hasOwnProperty.call(raw, "hypothesis_tests")) {
    const tests = cbsemWireExactRecord(raw.hypothesis_tests, ["method_version", "null_hypothesis", "statistic", "tie_policy", "probability_method", "decision_rule", "selected_test_tail", "null_value", "significance_level", "usable_replicates", "inference", "parameters"], [], `${path}.hypothesis_tests`);
    if (tests.method_version !== "cbsem_exact_case_bootstrap_null_centered_test_tail_v1"
      || tests.null_hypothesis !== "compiled_free_parameter_equals_zero_v1"
      || tests.statistic !== "unstudentized_null_centered_parameter_estimate_v1"
      || tests.tie_policy !== "inclusive_ieee_comparison_v1"
      || tests.probability_method !== "plus_one_over_usable_plus_one_v1"
      || tests.decision_rule !== "selected_p_value_less_than_or_equal_alpha_v1"
      || !["two_sided", "one_sided_greater", "one_sided_less"].includes(String(tests.selected_test_tail))
      || !Object.is(tests.null_value, 0) || tests.significance_level !== 0.05
      || cbsemWireCount(tests.usable_replicates, `${path}.hypothesis_tests.usable_replicates`) !== usable
      || !Array.isArray(tests.parameters) || tests.parameters.length !== parameterIds.length) cbsemWireFail(`${path}.hypothesis_tests`, "has a drifted method, tail, null, alpha, usable count, or parameter cardinality");
    const testInference = cbsemWireRecord(tests.inference, `${path}.hypothesis_tests.inference`);
    if (globallyAvailable) {
      if (cbsemWireExactRecord(testInference, ["status"], [], `${path}.hypothesis_tests.inference`).status !== "available") {
        cbsemWireFail(`${path}.hypothesis_tests.inference`, "differs from the usable-refit threshold");
      }
    } else {
      const unavailable = cbsemWireExactRecord(testInference, ["status", "reason_code", "message"], [], `${path}.hypothesis_tests.inference`);
      if (unavailable.status !== "unavailable" || unavailable.reason_code !== "insufficient_usable_refits"
        || !cbsemWireText(unavailable.message, `${path}.hypothesis_tests.inference.message`).trim()) {
        cbsemWireFail(`${path}.hypothesis_tests.inference`, "differs from the usable-refit threshold");
      }
    }
    tests.parameters.forEach((entry, parameterIndex) => {
      const rowPath = `${path}.hypothesis_tests.parameters[${parameterIndex}]`;
      const row = cbsemWireExactRecord(entry, ["parameter_id", "outcome"], [], rowPath);
      if (row.parameter_id !== parameterIds[parameterIndex]) cbsemWireFail(`${rowPath}.parameter_id`, "has drifted stable order");
      const outcome = cbsemWireRecord(row.outcome, `${rowPath}.outcome`);
      if (outcome.status === "unavailable") {
        cbsemWireExactRecord(outcome, ["status", "reason"], [], `${rowPath}.outcome`);
        if (!["insufficient_usable_replicates", "nonregular_variance_boundary", "zero_null_outside_open_domain", "unsupported_parameter_family"].includes(String(outcome.reason))
          || (globallyAvailable && outcome.reason === "insufficient_usable_replicates")) cbsemWireFail(`${rowPath}.outcome.reason`, "has an invalid unavailable reason");
        return;
      }
      const availableOutcome = cbsemWireExactRecord(outcome, ["status", "point_estimate", "two_sided_exceedances", "greater_or_equal_exceedances", "less_or_equal_exceedances", "p_value_two_sided", "p_value_greater", "p_value_less", "selected_exceedances", "selected_p_value", "reject_null"], [], `${rowPath}.outcome`);
      if (availableOutcome.status !== "available" || !globallyAvailable) cbsemWireFail(`${rowPath}.outcome`, "cannot be available under the global threshold");
      const point = cbsemWireFinite(availableOutcome.point_estimate, `${rowPath}.point_estimate`);
      if (!Object.is(point, originals[parameterIndex])) cbsemWireFail(`${rowPath}.point_estimate`, "differs from the point estimate");
      const deltas = successfulRefits.map((refit) => refit.estimates[parameterIndex] - point);
      const counts = [deltas.filter((delta) => Math.abs(delta) >= Math.abs(point)).length, deltas.filter((delta) => delta >= point).length, deltas.filter((delta) => delta <= point).length];
      const probabilities = counts.map((count) => (count + 1) / (usable + 1));
      const observedCounts = [availableOutcome.two_sided_exceedances, availableOutcome.greater_or_equal_exceedances, availableOutcome.less_or_equal_exceedances].map((count, index) => cbsemWireCount(count, `${rowPath}.count[${index}]`));
      const observedProbabilities = [availableOutcome.p_value_two_sided, availableOutcome.p_value_greater, availableOutcome.p_value_less].map((probability, index) => cbsemWireFinite(probability, `${rowPath}.p[${index}]`));
      const tailIndex = tests.selected_test_tail === "two_sided" ? 0 : tests.selected_test_tail === "one_sided_greater" ? 1 : 2;
      const selectedCount = cbsemWireCount(availableOutcome.selected_exceedances, `${rowPath}.selected_exceedances`);
      const selectedProbability = cbsemWireFinite(availableOutcome.selected_p_value, `${rowPath}.selected_p_value`);
      if (observedCounts.some((count, index) => count !== counts[index]) || observedProbabilities.some((probability, index) => !Object.is(probability, probabilities[index]))
        || selectedCount !== counts[tailIndex] || !Object.is(selectedProbability, probabilities[tailIndex])
        || cbsemWireBoolean(availableOutcome.reject_null, `${rowPath}.reject_null`) !== (selectedProbability <= 0.05)) cbsemWireFail(rowPath, "has drifted null-centered counts, plus-one probabilities, selected tail, or decision");
    });
  }
  return raw as unknown as CbsemExactCaseBootstrapResultV1;
}

function parseCbsemExactBootstrapPointStandardErrorsV1(
  value: unknown,
  parameterIds: readonly string[],
  path: string,
): { receipt: CbsemExactCaseBootstrapRefitStandardErrorsV1; values: number[] | null } {
  const receipt = cbsemWireExactRecord(value, ["method_version", "outcome"], [], path);
  if (receipt.method_version !== CBSEM_EXACT_BOOTSTRAP_SE_METHOD_V1) {
    cbsemWireFail(`${path}.method_version`, `must equal ${CBSEM_EXACT_BOOTSTRAP_SE_METHOD_V1}`);
  }
  const outcome = cbsemWireRecord(receipt.outcome, `${path}.outcome`);
  if (outcome.status === "available") {
    const available = cbsemWireExactRecord(outcome, ["status", "information_method", "parameters"], [], `${path}.outcome`);
    if (available.information_method !== CBSEM_EXACT_BOOTSTRAP_INFORMATION_METHOD_V1 || !Array.isArray(available.parameters)
      || available.parameters.length !== parameterIds.length) {
      cbsemWireFail(`${path}.outcome`, "has drifted expected-information method or parameter cardinality");
    }
    const values = available.parameters.map((entry, index) => {
      const rowPath = `${path}.outcome.parameters[${index}]`;
      const row = cbsemWireExactRecord(entry, ["parameter_id", "standard_error"], [], rowPath);
      const standardError = cbsemWireFinite(row.standard_error, `${rowPath}.standard_error`);
      if (row.parameter_id !== parameterIds[index] || standardError <= 0) {
        cbsemWireFail(rowPath, "has drifted parameter order or a non-positive standard error");
      }
      return standardError;
    });
    return { receipt: receipt as unknown as CbsemExactCaseBootstrapRefitStandardErrorsV1, values };
  }
  const unavailable = cbsemWireExactRecord(outcome, ["status", "reason"], [], `${path}.outcome`);
  if (unavailable.status !== "unavailable"
    || !CBSEM_EXACT_BOOTSTRAP_SE_UNAVAILABLE_REASONS.includes(unavailable.reason as CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1)) {
    cbsemWireFail(`${path}.outcome`, "has an untyped analytical standard-error unavailability reason");
  }
  return { receipt: receipt as unknown as CbsemExactCaseBootstrapRefitStandardErrorsV1, values: null };
}

function parseCbsemExactCaseBootstrapStudentizedSidecarV1(
  value: unknown,
  base: CbsemExactCaseBootstrapResultV1,
  path: string,
): CbsemExactCaseBootstrapStudentizedSidecarV1 {
  const sidecar = cbsemWireExactRecord(value, [
    "method_version", "standard_error_method_version", "expected_information_method", "pivot_method",
    "quantile_method", "interval_method", "archive_validation_scope", "confidence_level",
    "minimum_usable_fraction", "minimum_usable_replicates", "studentized_usable_replicates",
    "parameter_ids", "point_standard_errors", "inference", "intervals", "refit_standard_errors",
  ], [], path);
  if (sidecar.method_version !== "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1"
    || sidecar.standard_error_method_version !== CBSEM_EXACT_BOOTSTRAP_SE_METHOD_V1
    || sidecar.expected_information_method !== CBSEM_EXACT_BOOTSTRAP_INFORMATION_METHOD_V1
    || sidecar.pivot_method !== "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1"
    || sidecar.quantile_method !== "percentile_type7_v1"
    || sidecar.interval_method !== "reversed_type7_studentized_pivot_v1"
    || sidecar.archive_validation_scope !== "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1"
    || sidecar.confidence_level !== 0.95 || sidecar.minimum_usable_fraction !== base.minimum_usable_fraction
    || sidecar.minimum_usable_replicates !== base.minimum_usable_replicates) {
    cbsemWireFail(path, "has a drifted analytic-studentized method, threshold, or archive scope");
  }
  if (!Array.isArray(sidecar.parameter_ids)
    || sidecar.parameter_ids.length !== base.parameter_ids.length
    || sidecar.parameter_ids.some((id, index) => id !== base.parameter_ids[index])) {
    cbsemWireFail(`${path}.parameter_ids`, "must exactly match the v10 base parameter identity and order");
  }
  const point = parseCbsemExactBootstrapPointStandardErrorsV1(
    sidecar.point_standard_errors,
    base.parameter_ids,
    `${path}.point_standard_errors`,
  );
  if (!Array.isArray(sidecar.refit_standard_errors)
    || sidecar.refit_standard_errors.length !== base.successful_refits.length) {
    cbsemWireFail(`${path}.refit_standard_errors`, "must contain exactly one compact receipt per successful base refit");
  }
  const usableRefits: Array<{ estimates: readonly number[]; standardErrors: readonly number[] }> = [];
  const refitStandardErrors = sidecar.refit_standard_errors.map((entry, index) => {
    const rowPath = `${path}.refit_standard_errors[${index}]`;
    const row = cbsemWireExactRecord(entry, ["replicate_index", "outcome"], [], rowPath);
    const baseRefit = base.successful_refits[index];
    if (cbsemWireCount(row.replicate_index, `${rowPath}.replicate_index`) !== baseRefit.replicate_index) {
      cbsemWireFail(`${rowPath}.replicate_index`, "does not follow the successful v10 base-refit ledger");
    }
    const outcome = cbsemWireRecord(row.outcome, `${rowPath}.outcome`);
    if (outcome.status === "available") {
      const available = cbsemWireExactRecord(outcome, ["status", "information_method", "standard_errors"], [], `${rowPath}.outcome`);
      if (available.information_method !== CBSEM_EXACT_BOOTSTRAP_INFORMATION_METHOD_V1
        || !Array.isArray(available.standard_errors) || available.standard_errors.length !== base.parameter_ids.length) {
        cbsemWireFail(`${rowPath}.outcome`, "has drifted expected-information method or standard-error width");
      }
      const standardErrors = available.standard_errors.map((standardError, parameterIndex) => {
        const parsed = cbsemWireFinite(standardError, `${rowPath}.outcome.standard_errors[${parameterIndex}]`);
        if (parsed <= 0) cbsemWireFail(`${rowPath}.outcome.standard_errors[${parameterIndex}]`, "must be positive");
        return parsed;
      });
      usableRefits.push({ estimates: baseRefit.parameter_estimates, standardErrors });
    } else {
      const unavailable = cbsemWireExactRecord(outcome, ["status", "reason"], [], `${rowPath}.outcome`);
      if (unavailable.status !== "unavailable"
        || !CBSEM_EXACT_BOOTSTRAP_SE_UNAVAILABLE_REASONS.includes(unavailable.reason as CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1)) {
        cbsemWireFail(`${rowPath}.outcome`, "has an untyped analytical standard-error unavailability reason");
      }
    }
    return row;
  });
  const studentizedUsable = cbsemWireCount(sidecar.studentized_usable_replicates, `${path}.studentized_usable_replicates`);
  if (studentizedUsable !== usableRefits.length) {
    cbsemWireFail(`${path}.studentized_usable_replicates`, "differs from the compact standard-error receipt partition");
  }
  const unavailableReason = point.values === null
    ? "point_standard_errors_unavailable" as const
    : studentizedUsable < base.minimum_usable_replicates
      ? "insufficient_studentized_usable_replicates" as const
      : null;
  const inference = cbsemWireRecord(sidecar.inference, `${path}.inference`);
  if (unavailableReason === null) {
    const available = cbsemWireExactRecord(inference, ["status"], [], `${path}.inference`);
    if (available.status !== "available") cbsemWireFail(`${path}.inference`, "must be available above the whole-vector usable threshold");
  } else {
    const unavailable = cbsemWireExactRecord(inference, ["status", "reason", "message"], [], `${path}.inference`);
    const expectedMessage = unavailableReason === "point_standard_errors_unavailable"
      ? "Analytically studentized inference is unavailable because the point estimate has no whole-vector analytical standard-error receipt."
      : `Analytically studentized inference is unavailable because ${studentizedUsable} whole-vector usable refits are below the required ${base.minimum_usable_replicates}.`;
    if (unavailable.status !== "unavailable" || unavailable.reason !== unavailableReason
      || unavailable.message !== expectedMessage) {
      cbsemWireFail(`${path}.inference`, "does not exactly bind the analytical-SE availability state");
    }
  }
  if (!Array.isArray(sidecar.intervals) || sidecar.intervals.length !== base.parameter_ids.length) {
    cbsemWireFail(`${path}.intervals`, "must contain one typed outcome per base parameter");
  }
  const lowerProbability = (1 - 0.95) / 2;
  const upperProbability = 1 - lowerProbability;
  const intervals = sidecar.intervals.map((entry, parameterIndex) => {
    const rowPath = `${path}.intervals[${parameterIndex}]`;
    const row = cbsemWireExactRecord(entry, ["parameter_id", "outcome"], [], rowPath);
    if (row.parameter_id !== base.parameter_ids[parameterIndex]) cbsemWireFail(`${rowPath}.parameter_id`, "has drifted stable order");
    const outcome = cbsemWireRecord(row.outcome, `${rowPath}.outcome`);
    if (unavailableReason !== null) {
      const unavailable = cbsemWireExactRecord(outcome, ["status", "reason"], [], `${rowPath}.outcome`);
      if (unavailable.status !== "unavailable" || unavailable.reason !== unavailableReason) {
        cbsemWireFail(`${rowPath}.outcome`, "differs from the global studentized availability state");
      }
      return row;
    }
    const available = cbsemWireExactRecord(outcome, [
      "status", "point_estimate", "point_standard_error", "lower_pivot_quantile", "upper_pivot_quantile",
      "interval_lower", "interval_upper", "usable_replicates",
    ], [], `${rowPath}.outcome`);
    if (available.status !== "available") cbsemWireFail(`${rowPath}.outcome`, "must be available under the global studentized state");
    const baseInterval = base.intervals[parameterIndex];
    if (!baseInterval) cbsemWireFail(rowPath, "cannot bind a studentized interval without the available v10 base point estimate");
    const pointEstimate = cbsemWireFinite(available.point_estimate, `${rowPath}.outcome.point_estimate`);
    const pointStandardError = cbsemWireFinite(available.point_standard_error, `${rowPath}.outcome.point_standard_error`);
    const lowerPivot = cbsemWireFinite(available.lower_pivot_quantile, `${rowPath}.outcome.lower_pivot_quantile`);
    const upperPivot = cbsemWireFinite(available.upper_pivot_quantile, `${rowPath}.outcome.upper_pivot_quantile`);
    const intervalLower = cbsemWireFinite(available.interval_lower, `${rowPath}.outcome.interval_lower`);
    const intervalUpper = cbsemWireFinite(available.interval_upper, `${rowPath}.outcome.interval_upper`);
    const pivots = usableRefits.map((refit) => (
      (refit.estimates[parameterIndex] - baseInterval.original) / refit.standardErrors[parameterIndex]
    ));
    if (pivots.some((pivot) => !Number.isFinite(pivot))) cbsemWireFail(rowPath, "has a nonfinite studentized pivot");
    pivots.sort((left, right) => left - right);
    const expectedLowerPivot = cbsemExactBootstrapType7(pivots, lowerProbability);
    const expectedUpperPivot = cbsemExactBootstrapType7(pivots, upperProbability);
    const expectedLower = baseInterval.original - expectedUpperPivot * point.values![parameterIndex];
    const expectedUpper = baseInterval.original - expectedLowerPivot * point.values![parameterIndex];
    if (!Object.is(pointEstimate, baseInterval.original)
      || !Object.is(pointStandardError, point.values![parameterIndex])
      || !Object.is(lowerPivot, expectedLowerPivot) || !Object.is(upperPivot, expectedUpperPivot)
      || !Object.is(intervalLower, expectedLower) || !Object.is(intervalUpper, expectedUpper)
      || cbsemWireCount(available.usable_replicates, `${rowPath}.outcome.usable_replicates`) !== studentizedUsable
      || intervalLower > intervalUpper) {
      cbsemWireFail(rowPath, "has drifted outer-SE pivots or reversed Type-7 interval arithmetic");
    }
    return row;
  });
  return {
    ...sidecar,
    point_standard_errors: point.receipt,
    inference: inference as CbsemExactCaseBootstrapStudentizedSidecarV1["inference"],
    intervals: intervals as unknown as CbsemExactCaseBootstrapStudentizedSidecarV1["intervals"],
    refit_standard_errors: refitStandardErrors as unknown as CbsemExactCaseBootstrapStudentizedSidecarV1["refit_standard_errors"],
  } as unknown as CbsemExactCaseBootstrapStudentizedSidecarV1;
}

function parseCbsemExactCaseBootstrapWithStudentizedResultV1(
  value: unknown,
  path = "result.estimation.analysis.exact_case_bootstrap_studentized",
): CbsemExactCaseBootstrapWithStudentizedResultV1 {
  const wrapper = cbsemWireExactRecord(value, ["base", "studentized"], [], path);
  const base = parseCbsemExactCaseBootstrapResultV1(wrapper.base, `${path}.base`);
  const studentized = parseCbsemExactCaseBootstrapStudentizedSidecarV1(wrapper.studentized, base, `${path}.studentized`);
  return { base, studentized };
}

function parseCbsemExactCaseBootstrapBcaSidecarV1(
  value: unknown,
  base: CbsemExactCaseBootstrapResultV1,
  path: string,
): CbsemExactCaseBootstrapBcaSidecarV1 {
  const sidecar = cbsemWireExactRecord(value, [
    "method_version", "base_bootstrap_method_version", "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256",
    "model_scientific_sha256", "delete_one_refit_method_version", "bias_correction_method",
    "acceleration_method", "adjusted_probability_method", "quantile_method", "retry_policy",
    "confidence_level", "bootstrap_usable_replicates", "minimum_bootstrap_usable_replicates",
    "delete_one_case_count", "parameter_ids", "inference", "intervals",
    "successful_delete_one_refits", "failed_delete_one_refits",
  ], [], path);
  if (sidecar.method_version !== "cbsem_exact_case_bootstrap_bca_interval_v1"
    || sidecar.base_bootstrap_method_version !== base.method_version
    || sidecar.delete_one_refit_method_version !== "cbsem_exact_case_bootstrap_delete_one_refit_v1"
    || sidecar.bias_correction_method !== "midrank_less_plus_half_ties_no_clamp_v1"
    || sidecar.acceleration_method !== "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2"
    || sidecar.adjusted_probability_method !== "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2"
    || sidecar.quantile_method !== "percentile_type7_v1"
    || sidecar.retry_policy !== "no_retry_exactly_one_fit_per_omitted_case_v1"
    || sidecar.confidence_level !== base.confidence_level) {
    cbsemWireFail(path, "has a drifted BCa method or confidence contract");
  }
  for (const [field, expected] of Object.entries({
    outer_recipe_analytical_identity_sha256: base.outer_recipe_analytical_identity_sha256,
    base_point_result_sha256: base.base_point_result_sha256,
    compiler_analytical_identity_sha256: base.compiler_analytical_identity_sha256,
    plan_sha256: base.plan_sha256,
    model_scientific_sha256: base.model_scientific_sha256,
  })) {
    const observed = cbsemWireSha256(sidecar[field], `${path}.${field}`);
    if (observed !== expected) cbsemWireFail(`${path}.${field}`, "differs from the atomic base authority");
  }
  const bootstrapUsable = cbsemWireCount(sidecar.bootstrap_usable_replicates, `${path}.bootstrap_usable_replicates`);
  const minimumBootstrapUsable = cbsemWireCount(sidecar.minimum_bootstrap_usable_replicates, `${path}.minimum_bootstrap_usable_replicates`);
  const deleteOneCaseCount = cbsemWireCount(sidecar.delete_one_case_count, `${path}.delete_one_case_count`);
  if (bootstrapUsable !== base.usable_replicates
    || minimumBootstrapUsable !== base.minimum_usable_replicates
    || deleteOneCaseCount !== base.complete_case_sample_size) {
    cbsemWireFail(path, "has threshold or case authority different from the atomic base");
  }
  if (!Array.isArray(sidecar.parameter_ids)
    || sidecar.parameter_ids.length !== base.parameter_ids.length
    || sidecar.parameter_ids.some((id, index) => id !== base.parameter_ids[index])) {
    cbsemWireFail(`${path}.parameter_ids`, "must exactly match the base parameter identity and order");
  }
  if (!Array.isArray(sidecar.successful_delete_one_refits)
    || !Array.isArray(sidecar.failed_delete_one_refits)) {
    cbsemWireFail(path, "must retain both typed delete-one ledgers");
  }
  let priorSuccessfulPosition = -1;
  const successful = sidecar.successful_delete_one_refits.map((entry, index) => {
    const rowPath = `${path}.successful_delete_one_refits[${index}]`;
    const row = cbsemWireExactRecord(entry, [
      "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
      "retained_sample_indices_sha256", "parameter_estimates", "iterations", "objective", "gradient_norm",
    ], [], rowPath);
    const position = cbsemWireCount(row.omitted_complete_case_position, `${rowPath}.omitted_complete_case_position`);
    const sourceRow = cbsemWireCount(row.omitted_source_row_index, `${rowPath}.omitted_source_row_index`);
    if (position >= deleteOneCaseCount || position <= priorSuccessfulPosition) cbsemWireFail(rowPath, "has invalid omission order");
    priorSuccessfulPosition = position;
    cbsemWireSha256(row.retained_sampling_positions_sha256, `${rowPath}.retained_sampling_positions_sha256`);
    cbsemWireSha256(row.retained_sample_indices_sha256, `${rowPath}.retained_sample_indices_sha256`);
    if (!Array.isArray(row.parameter_estimates) || row.parameter_estimates.length !== base.parameter_ids.length) {
      cbsemWireFail(`${rowPath}.parameter_estimates`, "has drifted parameter width");
    }
    row.parameter_estimates.forEach((estimate, parameterIndex) => cbsemWireFinite(estimate, `${rowPath}.parameter_estimates[${parameterIndex}]`));
    const iterations = cbsemWireCount(row.iterations, `${rowPath}.iterations`);
    const objective = cbsemWireFinite(row.objective, `${rowPath}.objective`);
    const gradient = cbsemWireFinite(row.gradient_norm, `${rowPath}.gradient_norm`);
    if (iterations === 0 || objective < 0 || gradient < 0) cbsemWireFail(rowPath, "has invalid convergence values");
    return { position, sourceRow };
  });
  let priorFailedPosition = -1;
  const failureKinds = [
    "moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure",
  ];
  const failed = sidecar.failed_delete_one_refits.map((entry, index) => {
    const rowPath = `${path}.failed_delete_one_refits[${index}]`;
    const row = cbsemWireExactRecord(entry, [
      "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
      "retained_sample_indices_sha256", "kind", "message",
    ], [], rowPath);
    const position = cbsemWireCount(row.omitted_complete_case_position, `${rowPath}.omitted_complete_case_position`);
    const sourceRow = cbsemWireCount(row.omitted_source_row_index, `${rowPath}.omitted_source_row_index`);
    if (position >= deleteOneCaseCount || position <= priorFailedPosition
      || !failureKinds.includes(String(row.kind))
      || !cbsemWireText(row.message, `${rowPath}.message`).trim()) cbsemWireFail(rowPath, "has invalid omission order, kind, or message");
    priorFailedPosition = position;
    cbsemWireSha256(row.retained_sampling_positions_sha256, `${rowPath}.retained_sampling_positions_sha256`);
    cbsemWireSha256(row.retained_sample_indices_sha256, `${rowPath}.retained_sample_indices_sha256`);
    return { position, sourceRow };
  });
  const omissionPartition = [...successful, ...failed].sort((left, right) => left.position - right.position);
  if (omissionPartition.length !== deleteOneCaseCount
    || omissionPartition.some((row, index) => row.position !== index)
    || omissionPartition.some((row, index) => index > 0 && omissionPartition[index - 1].sourceRow >= row.sourceRow)) {
    cbsemWireFail(path, "does not form the complete, source-ordered delete-one omission partition");
  }

  const globalReason = base.inference.status !== "available"
    ? "base_inference_unavailable" as const
    : failed.length > 0 || successful.length !== deleteOneCaseCount
      ? "incomplete_delete_one_ledger" as const
      : null;
  const inference = cbsemWireRecord(sidecar.inference, `${path}.inference`);
  if (globalReason === null) {
    if (cbsemWireExactRecord(inference, ["status"], [], `${path}.inference`).status !== "available") {
      cbsemWireFail(`${path}.inference`, "must be available with complete base and delete-one ledgers");
    }
  } else {
    const unavailable = cbsemWireExactRecord(inference, ["status", "reason", "message"], [], `${path}.inference`);
    const expectedMessage = globalReason === "base_inference_unavailable"
      ? `BCa inference is unavailable because ${base.usable_replicates} successful bootstrap point refits are below the bound minimum ${base.minimum_usable_replicates}.`
      : `BCa inference is unavailable because ${failed.length} of ${deleteOneCaseCount} mandatory delete-one fits failed.`;
    if (unavailable.status !== "unavailable" || unavailable.reason !== globalReason
      || unavailable.message !== expectedMessage) cbsemWireFail(`${path}.inference`, "does not exactly bind the global BCa availability state");
  }
  if (!Array.isArray(sidecar.intervals) || sidecar.intervals.length !== base.parameter_ids.length) {
    cbsemWireFail(`${path}.intervals`, "must contain one typed outcome per base parameter");
  }
  const intervals = sidecar.intervals.map((entry, parameterIndex) => {
    const rowPath = `${path}.intervals[${parameterIndex}]`;
    const row = cbsemWireExactRecord(entry, ["parameter_id", "outcome"], [], rowPath);
    if (row.parameter_id !== base.parameter_ids[parameterIndex]) cbsemWireFail(`${rowPath}.parameter_id`, "has drifted stable order");
    const outcome = cbsemWireRecord(row.outcome, `${rowPath}.outcome`);
    if (outcome.status === "unavailable") {
      const unavailable = cbsemWireExactRecord(outcome, ["status", "reason"], [], `${rowPath}.outcome`);
      if (!CBSEM_EXACT_BOOTSTRAP_BCA_UNAVAILABLE_REASONS.includes(unavailable.reason as CbsemExactCaseBootstrapBcaUnavailableReasonV1)
        || (globalReason !== null && unavailable.reason !== globalReason)
        || (globalReason === null && (unavailable.reason === "base_inference_unavailable" || unavailable.reason === "incomplete_delete_one_ledger"))) {
        cbsemWireFail(`${rowPath}.outcome`, "has a BCa reason inconsistent with the global state");
      }
      return row;
    }
    if (globalReason !== null) cbsemWireFail(`${rowPath}.outcome`, "cannot be available under global BCa unavailability");
    const available = cbsemWireExactRecord(outcome, [
      "status", "point_estimate", "bias_correction", "acceleration", "adjusted_lower_probability",
      "adjusted_upper_probability", "interval_lower", "interval_upper", "usable_replicates",
    ], [], `${rowPath}.outcome`);
    if (available.status !== "available") cbsemWireFail(`${rowPath}.outcome.status`, "is unsupported");
    const baseInterval = base.intervals[parameterIndex];
    if (!baseInterval) cbsemWireFail(rowPath, "cannot bind BCa inference without an available base point estimate");
    const point = cbsemWireFinite(available.point_estimate, `${rowPath}.outcome.point_estimate`);
    cbsemWireFinite(available.bias_correction, `${rowPath}.outcome.bias_correction`);
    cbsemWireFinite(available.acceleration, `${rowPath}.outcome.acceleration`);
    const lowerProbability = cbsemWireFinite(available.adjusted_lower_probability, `${rowPath}.outcome.adjusted_lower_probability`);
    const upperProbability = cbsemWireFinite(available.adjusted_upper_probability, `${rowPath}.outcome.adjusted_upper_probability`);
    const lower = cbsemWireFinite(available.interval_lower, `${rowPath}.outcome.interval_lower`);
    const upper = cbsemWireFinite(available.interval_upper, `${rowPath}.outcome.interval_upper`);
    const sortedBootstrap = base.successful_refits
      .map((refit) => refit.parameter_estimates[parameterIndex])
      .sort((left, right) => left - right);
    if (!Object.is(point, baseInterval.original)
      || lowerProbability < 0 || upperProbability > 1 || lowerProbability > upperProbability
      || !Object.is(lower, cbsemExactBootstrapType7(sortedBootstrap, lowerProbability))
      || !Object.is(upper, cbsemExactBootstrapType7(sortedBootstrap, upperProbability))
      || lower > upper
      || cbsemWireCount(available.usable_replicates, `${rowPath}.outcome.usable_replicates`) !== base.usable_replicates) {
      cbsemWireFail(rowPath, "has drifted base binding, adjusted probability, or exposed Type-7 arithmetic");
    }
    return row;
  });
  return {
    ...sidecar,
    inference: inference as CbsemExactCaseBootstrapBcaSidecarV1["inference"],
    intervals: intervals as unknown as CbsemExactCaseBootstrapBcaSidecarV1["intervals"],
  } as unknown as CbsemExactCaseBootstrapBcaSidecarV1;
}

function parseCbsemExactCaseBootstrapWithBcaResultV1(
  value: unknown,
  path = "result.estimation.analysis.exact_case_bootstrap_bca",
): CbsemExactCaseBootstrapWithBcaResultV1 {
  const wrapper = cbsemWireExactRecord(value, ["base", "bca"], [], path);
  const base = parseCbsemExactCaseBootstrapResultV1(wrapper.base, `${path}.base`);
  const bca = parseCbsemExactCaseBootstrapBcaSidecarV1(wrapper.bca, base, `${path}.bca`);
  return { base, bca };
}

/** Strict outer/result parser for the untrusted native CB-SEM wire payload. */
export function parseInternalRecipeV4CbsemExecutionResultV1(input: unknown): InternalRecipeV4CbsemExecutionResultV1 {
  const result = cbsemWireExactRecord(input, ["schema_version", "provenance", "estimation"], [], "result");
  if (result.schema_version !== INTERNAL_RECIPE_V4_CBSEM_COMMAND_SCHEMA_VERSION) cbsemWireFail("result.schema_version", "must equal 1");
  const provenance = cbsemWireExactRecord(result.provenance, ["adapter_version", "compilation_receipt", "dataset_id", "estimator_method_version", "moment_input_method_version"], [], "result.provenance");
  const adapterVersion = cbsemWireText(provenance.adapter_version, "result.provenance.adapter_version");
  const compilationReceipt = parseCbsemCompilationReceiptV1(provenance.compilation_receipt);
  const provenanceDatasetId = cbsemWireText(provenance.dataset_id, "result.provenance.dataset_id");
  const estimatorMethodVersion = cbsemWireText(provenance.estimator_method_version, "result.provenance.estimator_method_version");
  const provenanceMomentMethodVersion = cbsemWireText(provenance.moment_input_method_version, "result.provenance.moment_input_method_version");

  const estimation = cbsemWireExactRecord(result.estimation, [
    "schema_version", "method_version", "compiler_analytical_identity_sha256", "plan_sha256", "model_scientific_sha256",
    "input", "covariance_ml", "parameter_ids", "analysis",
  ], ["observed_means", "implied_means", "residual_means"], "result.estimation");
  const estimationSchemaVersion = cbsemWireCount(estimation.schema_version, "result.estimation.schema_version");
  const estimationMethodVersion = cbsemWireText(estimation.method_version, "result.estimation.method_version");
  const compilerIdentity = cbsemWireSha256(estimation.compiler_analytical_identity_sha256, "result.estimation.compiler_analytical_identity_sha256");
  const planSha256 = cbsemWireSha256(estimation.plan_sha256, "result.estimation.plan_sha256");
  const modelSha256 = cbsemWireSha256(estimation.model_scientific_sha256, "result.estimation.model_scientific_sha256");
  const momentInput = parseCbsemMomentInputV2(estimation.input);
  if (!Array.isArray(estimation.covariance_ml) || estimation.covariance_ml.length !== momentInput.variable_ids.length) cbsemWireFail("result.estimation.covariance_ml", "must be square in canonical modeled-variable order");
  const covarianceMl = estimation.covariance_ml.map((row, rowIndex) => {
    if (!Array.isArray(row) || row.length !== momentInput.variable_ids.length) cbsemWireFail(`result.estimation.covariance_ml[${rowIndex}]`, "must have canonical matrix width");
    return row.map((cell, columnIndex) => cbsemWireFinite(cell, `result.estimation.covariance_ml[${rowIndex}][${columnIndex}]`));
  });
  const parameterIdsRecord = cbsemWireRecord(estimation.parameter_ids, "result.estimation.parameter_ids");
  const parameterIds = Object.fromEntries(Object.entries(parameterIdsRecord).map(([name, stableId]) => [cbsemWireText(name, "result.estimation.parameter_ids key"), cbsemWireText(stableId, `result.estimation.parameter_ids.${name}`)]));
  const analysis = cbsemWireRecord(estimation.analysis, "result.estimation.analysis");
  if (cbsemWireText(analysis.method_version, "result.estimation.analysis.method_version") !== estimatorMethodVersion) cbsemWireFail("result.provenance.estimator_method_version", "differs from the estimator analysis");
  const scoreLm = "score_lm" in analysis
    ? parseCbsemCfaScoreLmBundleV1(analysis.score_lm)
    : undefined;
  const exactBootstrap = "exact_case_bootstrap" in analysis
    ? parseCbsemExactCaseBootstrapResultV1(analysis.exact_case_bootstrap)
    : undefined;
  const exactBootstrapStudentized = "exact_case_bootstrap_studentized" in analysis
    ? parseCbsemExactCaseBootstrapWithStudentizedResultV1(analysis.exact_case_bootstrap_studentized)
    : undefined;
  const exactBootstrapBca = "exact_case_bootstrap_bca" in analysis
    ? parseCbsemExactCaseBootstrapWithBcaResultV1(analysis.exact_case_bootstrap_bca)
    : undefined;
  const observedMeans = "observed_means" in estimation ? parseCbsemMeanCells(estimation.observed_means, "result.estimation.observed_means") : undefined;
  const impliedMeans = "implied_means" in estimation ? parseCbsemMeanCells(estimation.implied_means, "result.estimation.implied_means") : undefined;
  const residualMeans = "residual_means" in estimation ? parseCbsemMeanCells(estimation.residual_means, "result.estimation.residual_means") : undefined;
  const meanReplacement = Boolean(momentInput.missing_data_treatment);
  const meanStructure = Boolean(momentInput.canonical_observed_means_sha256 || observedMeans || impliedMeans || residualMeans);
  if (meanReplacement && meanStructure) cbsemWireFail("result.estimation", "cannot combine mean replacement v1 with a mean structure");
  const expectedSchemaVersion = meanReplacement ? 4 : meanStructure ? 3 : 2;
  const expectedMomentMethodVersion: CbsemCompiledMomentMethodVersionV1 = meanReplacement
    ? CBSEM_COMPILED_MOMENT_MEAN_REPLACEMENT_METHOD_VERSION_V1
    : meanStructure
      ? CBSEM_COMPILED_MOMENT_MEAN_STRUCTURE_METHOD_VERSION_V1
      : CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1;
  if (estimationSchemaVersion !== expectedSchemaVersion || estimationMethodVersion !== expectedMomentMethodVersion || provenanceMomentMethodVersion !== expectedMomentMethodVersion) cbsemWireFail("result.estimation", "has a compiled-moment identity inconsistent with its missing-data and mean-structure payload");
  const validGeneration = [
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v2", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v3", 3, CBSEM_COMPILED_MOMENT_MEAN_STRUCTURE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v4"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v4", 4, CBSEM_COMPILED_MOMENT_MEAN_REPLACEMENT_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v5", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v6", 3, CBSEM_COMPILED_MOMENT_MEAN_STRUCTURE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v4"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v7", 4, CBSEM_COMPILED_MOMENT_MEAN_REPLACEMENT_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v8", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v9", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v10", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v11", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
    ["compiled_recipe_v4_cbsem_plan_v2_execution_v12", 2, CBSEM_COMPILED_MOMENT_LISTWISE_METHOD_VERSION_V1, "cbsem_ml_exact_parameter_table_v3"],
  ].some(([adapter, schema, moment, estimator]) => adapterVersion === adapter
    && estimationSchemaVersion === schema && estimationMethodVersion === moment
    && estimatorMethodVersion === estimator);
  if (!validGeneration) cbsemWireFail("result.provenance", "has an unsupported exact CB-SEM estimator/adapter identity");
  const scoreLmOwner = [
    "compiled_recipe_v4_cbsem_plan_v2_execution_v8",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v9",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v10",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v11",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v12",
  ].includes(adapterVersion);
  const baseBootstrapOwner = adapterVersion === "compiled_recipe_v4_cbsem_plan_v2_execution_v9"
    || adapterVersion === "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
  const studentizedBootstrapOwner = adapterVersion === "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
  const bcaBootstrapOwner = adapterVersion === "compiled_recipe_v4_cbsem_plan_v2_execution_v12";
  const exactBootstrapSourceFingerprint = cbsemExactBootstrapSourceFingerprintPayloadV1(
    momentInput.dataset_fingerprint,
  );
  const exactBootstrapAuthority = exactBootstrap
    ?? exactBootstrapStudentized?.base
    ?? exactBootstrapBca?.base;
  const compilationFingerprintMatchesMoment = baseBootstrapOwner || studentizedBootstrapOwner || bcaBootstrapOwner
    ? cbsemExactBootstrapSourceFingerprintPayloadV1(compilationReceipt.dataset_fingerprint) === exactBootstrapSourceFingerprint
    : compilationReceipt.dataset_fingerprint === momentInput.dataset_fingerprint;
  const compilationIdentityMatchesMoment = exactBootstrapAuthority
    ? compilationReceipt.recipe_analytical_sha256 === exactBootstrapAuthority.outer_recipe_analytical_identity_sha256
    : compilationReceipt.analytical_identity_sha256 === compilerIdentity;
  if (scoreLmOwner) {
    if (!scoreLm || analysis.model_type !== "cfa" || analysis.mean_structure !== false) cbsemWireFail("result.estimation.analysis.score_lm", `adapter ${adapterVersion.split("_").at(-1)} requires the covariance-only CFA score/LM bundle`);
    if (!Array.isArray(analysis.modification_indices) || analysis.modification_indices.length !== 0) cbsemWireFail("result.estimation.analysis.modification_indices", "current score/LM adapters cannot carry legacy heuristic modification indices");
    const stableIds = new Set(Object.values(parameterIds));
    if (scoreLm.rows.some((row) => !stableIds.has(row.parameter_id))) cbsemWireFail("result.estimation.analysis.score_lm.rows", "contains a parameter outside the compiled stable-identity map");
  } else if (scoreLm) cbsemWireFail("result.estimation.analysis.score_lm", "is unavailable before adapter v8");
  if (baseBootstrapOwner) {
    if (!exactBootstrap || exactBootstrap.source_dataset_id !== momentInput.dataset_id
      || exactBootstrap.source_dataset_fingerprint !== exactBootstrapSourceFingerprint
      || exactBootstrap.compiler_analytical_identity_sha256 !== compilerIdentity
      || exactBootstrap.plan_sha256 !== planSha256 || exactBootstrap.model_scientific_sha256 !== modelSha256
      || exactBootstrap.complete_case_sample_size !== momentInput.used_sample_size) cbsemWireFail("result.estimation.analysis.exact_case_bootstrap", "does not bind the exact listwise CFA result identity");
    const hasHypothesisTests = exactBootstrap.hypothesis_tests != null;
    if (adapterVersion.endsWith("_v9") ? hasHypothesisTests : !hasHypothesisTests) cbsemWireFail("result.estimation.analysis.exact_case_bootstrap.hypothesis_tests", adapterVersion.endsWith("_v9")
      ? "historical adapter v9 must omit the v10 hypothesis receipt"
      : "adapter v10 requires the complete hypothesis receipt");
  } else if (exactBootstrap) cbsemWireFail("result.estimation.analysis.exact_case_bootstrap", studentizedBootstrapOwner
    ? "adapter v11 owns only the atomic exact_case_bootstrap_studentized wrapper"
    : bcaBootstrapOwner
      ? "adapter v12 owns only the atomic exact_case_bootstrap_bca wrapper"
    : "is unavailable before adapter v9");
  if (studentizedBootstrapOwner) {
    const base = exactBootstrapStudentized?.base;
    if (!exactBootstrapStudentized || !base
      || base.source_dataset_id !== momentInput.dataset_id
      || base.source_dataset_fingerprint !== exactBootstrapSourceFingerprint
      || base.compiler_analytical_identity_sha256 !== compilerIdentity
      || base.plan_sha256 !== planSha256 || base.model_scientific_sha256 !== modelSha256
      || base.complete_case_sample_size !== momentInput.used_sample_size
      || base.hypothesis_tests == null) {
      cbsemWireFail("result.estimation.analysis.exact_case_bootstrap_studentized", "does not atomically bind the exact v10 base, v10 hypothesis receipt, and v11 studentized sidecar");
    }
    if (momentInput.used_sample_size > 180 || momentInput.variable_ids.length > 9
      || base.parameter_ids.length > 18) {
      cbsemWireFail("result.estimation.analysis.exact_case_bootstrap_studentized", "exceeds the frozen N<=180, V<=9, or P<=18 Labs envelope");
    }
  } else if (exactBootstrapStudentized) {
    cbsemWireFail("result.estimation.analysis.exact_case_bootstrap_studentized", "is unavailable before adapter v11");
  }
  if (bcaBootstrapOwner) {
    const base = exactBootstrapBca?.base;
    if (!exactBootstrapBca || !base
      || base.source_dataset_id !== momentInput.dataset_id
      || base.source_dataset_fingerprint !== exactBootstrapSourceFingerprint
      || base.compiler_analytical_identity_sha256 !== compilerIdentity
      || base.plan_sha256 !== planSha256 || base.model_scientific_sha256 !== modelSha256
      || base.complete_case_sample_size !== momentInput.used_sample_size
      || base.hypothesis_tests == null) {
      cbsemWireFail("result.estimation.analysis.exact_case_bootstrap_bca", "does not atomically bind the exact v10 base, v10 hypothesis receipt, and v12 BCa sidecar");
    }
    if (momentInput.used_sample_size > 180 || momentInput.variable_ids.length > 9
      || base.parameter_ids.length > 18) {
      cbsemWireFail("result.estimation.analysis.exact_case_bootstrap_bca", "exceeds the frozen N<=180, V<=9, or P<=18 Labs envelope");
    }
  } else if (exactBootstrapBca) {
    cbsemWireFail("result.estimation.analysis.exact_case_bootstrap_bca", "is unavailable before adapter v12");
  }
  if (meanStructure && (!momentInput.canonical_observed_means_sha256 || !observedMeans?.length || !impliedMeans?.length || !residualMeans?.length)) cbsemWireFail("result.estimation", "mean-structure identity requires complete observed, implied, and residual mean provenance");
  if (provenanceDatasetId !== momentInput.dataset_id || !compilationFingerprintMatchesMoment || !compilationIdentityMatchesMoment || compilationReceipt.plan_sha256 !== planSha256 || compilationReceipt.model_scientific_sha256 !== modelSha256) cbsemWireFail("result", "has drift between compilation, moment-input, and estimator identities");
  return {
    schema_version: 1,
    provenance: result.provenance as unknown as InternalRecipeV4CbsemExecutionProvenanceV1,
    estimation: {
      schema_version: expectedSchemaVersion,
      method_version: expectedMomentMethodVersion,
      compiler_analytical_identity_sha256: compilerIdentity,
      plan_sha256: planSha256,
      model_scientific_sha256: modelSha256,
      input: momentInput,
      covariance_ml: covarianceMl,
      parameter_ids: parameterIds,
      ...(observedMeans ? { observed_means: observedMeans } : {}),
      ...(impliedMeans ? { implied_means: impliedMeans } : {}),
      ...(residualMeans ? { residual_means: residualMeans } : {}),
      analysis: {
        ...analysis,
        ...(scoreLm ? { score_lm: scoreLm } : {}),
        ...(exactBootstrap ? { exact_case_bootstrap: exactBootstrap } : {}),
        ...(exactBootstrapStudentized ? { exact_case_bootstrap_studentized: exactBootstrapStudentized } : {}),
        ...(exactBootstrapBca ? { exact_case_bootstrap_bca: exactBootstrapBca } : {}),
      } as unknown as CbsemAnalysis,
    },
  };
}

type CbsemCanonicalExpectedCell = string | number | boolean | null;

function cbsemCanonicalTable(
  document: CanonicalResultDocumentV2,
  id: string,
  columns: readonly string[],
  path: string,
): CanonicalResultTable {
  const matches = document.tables.filter((table) => table.id === id);
  if (matches.length !== 1 || matches[0].columns.length !== columns.length
    || matches[0].columns.some((column, index) => column.id !== columns[index])) cbsemWireFail(path, "has a missing, duplicate, or drifted table contract");
  return matches[0];
}

function cbsemCanonicalCellMatches(cell: CanonicalResultCell | undefined, expected: CbsemCanonicalExpectedCell): boolean {
  if (expected === null) return cell?.kind === "missing" && cell.reason === "not_applicable" && cell.display === undefined;
  if (typeof expected === "string") return cell?.kind === "text" && cell.value === expected;
  if (typeof expected === "boolean") return cell?.kind === "boolean" && cell.value === expected;
  return cell?.kind === "number" && Object.is(cell.value, expected);
}

function cbsemCanonicalRowMatches(
  cells: readonly CanonicalResultCell[],
  expected: readonly CbsemCanonicalExpectedCell[],
): boolean {
  return cells.length === expected.length && expected.every((value, index) => cbsemCanonicalCellMatches(cells[index], value));
}

function bindCbsemExactBootstrapCanonicalV1(
  document: CanonicalResultDocumentV2,
  bootstrap: CbsemExactCaseBootstrapResultV1,
  adapterVersion: string,
): void {
  const path = "completedResult.canonicalDocument";
  const baseTableIds = [
    "exact_case_bootstrap_summary", "exact_case_bootstrap_parameter_intervals",
    "exact_case_bootstrap_successful_refits", "exact_case_bootstrap_failures",
  ];
  const sections = document.sections.filter((section) => section.id === "bootstrap_inference");
  if (sections.length !== 1 || sections[0].chart_ids.length !== 0
    || JSON.stringify(sections[0].table_ids) !== JSON.stringify(baseTableIds)) cbsemWireFail(`${path}.bootstrap_inference`, "has drifted exact bootstrap table ownership or order");
  const summary = cbsemCanonicalTable(document, baseTableIds[0], CBSEM_EXACT_BOOTSTRAP_SUMMARY_CANONICAL_COLUMNS, `${path}.${baseTableIds[0]}`);
  const intervals = cbsemCanonicalTable(document, baseTableIds[1], CBSEM_EXACT_BOOTSTRAP_INTERVAL_CANONICAL_COLUMNS, `${path}.${baseTableIds[1]}`);
  const successes = cbsemCanonicalTable(document, baseTableIds[2], CBSEM_EXACT_BOOTSTRAP_SUCCESS_CANONICAL_COLUMNS, `${path}.${baseTableIds[2]}`);
  const failures = cbsemCanonicalTable(document, baseTableIds[3], CBSEM_EXACT_BOOTSTRAP_FAILURE_CANONICAL_COLUMNS, `${path}.${baseTableIds[3]}`);
  const unavailable = bootstrap.inference.status === "unavailable" ? bootstrap.inference : null;
  const summaryExpected: CbsemCanonicalExpectedCell[] = [
    bootstrap.method_version, bootstrap.estimator_method_version, bootstrap.source_dataset_id,
    bootstrap.source_dataset_fingerprint, bootstrap.outer_recipe_analytical_identity_sha256,
    bootstrap.base_point_result_sha256, bootstrap.compiler_analytical_identity_sha256,
    bootstrap.plan_sha256, bootstrap.model_scientific_sha256, bootstrap.complete_case_sample_size,
    bootstrap.complete_case_universe_digest_method, bootstrap.complete_case_universe_sha256,
    bootstrap.covariance_denominator, bootstrap.sample_indices_digest_method,
    bootstrap.sampling_positions_digest_method, bootstrap.interval_method, bootstrap.confidence_level,
    bootstrap.requested_replicates, bootstrap.attempted_refits, bootstrap.usable_replicates,
    bootstrap.failed_replicates, bootstrap.minimum_usable_fraction, bootstrap.minimum_usable_replicates,
    String(bootstrap.seed), bootstrap.stream_token, bootstrap.retry_policy, bootstrap.max_attempts_per_replicate,
    JSON.stringify(bootstrap.parameter_ids), bootstrap.inference.status, unavailable?.reason_code ?? null,
    unavailable?.message ?? null, "schedule_and_arithmetic_only_no_raw_refit_replay_or_source_row_digest_recomputation",
  ];
  if (summary.rows.length !== 1 || summary.rows[0].id !== "bootstrap"
    || !cbsemCanonicalRowMatches(summary.rows[0].cells, summaryExpected)) cbsemWireFail(`${path}.${baseTableIds[0]}`, "does not exactly bind the analytical bootstrap summary");
  if (intervals.rows.length !== bootstrap.intervals.length || bootstrap.intervals.some((row, index) => {
    const canonical = intervals.rows[index];
    return canonical.id !== `bootstrap_interval_${String(index).padStart(4, "0")}` || !cbsemCanonicalRowMatches(canonical.cells, [
      row.parameter_id, row.original, row.bootstrap_mean, row.bias, row.standard_error,
      row.percentile_lower, row.percentile_upper, row.usable_replicates,
    ]);
  })) cbsemWireFail(`${path}.${baseTableIds[1]}`, "does not exactly bind analytical interval rows");
  if (successes.rows.length !== bootstrap.successful_refits.length || bootstrap.successful_refits.some((row, index) => {
    const canonical = successes.rows[index];
    return canonical.id !== `bootstrap_refit_${String(row.replicate_index).padStart(5, "0")}` || !cbsemCanonicalRowMatches(canonical.cells, [
      row.replicate_index, row.sampling_positions_sha256, row.sample_indices_sha256,
      JSON.stringify(row.parameter_estimates), row.iterations, row.objective, row.gradient_norm,
    ]);
  })) cbsemWireFail(`${path}.${baseTableIds[2]}`, "does not exactly bind analytical successful-refit rows");
  if (failures.rows.length !== bootstrap.failed_refits.length || bootstrap.failed_refits.some((row, index) => {
    const canonical = failures.rows[index];
    return canonical.id !== `bootstrap_failure_${String(row.replicate_index).padStart(5, "0")}` || !cbsemCanonicalRowMatches(canonical.cells, [
      row.replicate_index, row.sampling_positions_sha256, row.sample_indices_sha256, row.kind, row.message,
    ]);
  })) cbsemWireFail(`${path}.${baseTableIds[3]}`, "does not exactly bind analytical failure rows");

  const hypothesisTables = document.tables.filter((table) => table.id === "exact_case_bootstrap_hypothesis_tests");
  const hypothesisSections = document.sections.filter((section) => section.id === "bootstrap_hypothesis_tests");
  if (adapterVersion.endsWith("_v9")) {
    if (bootstrap.hypothesis_tests != null || hypothesisTables.length || hypothesisSections.length) cbsemWireFail(path, "historical adapter v9 carries injected v10 hypothesis artifacts");
    return;
  }
  const tests = bootstrap.hypothesis_tests;
  if (!tests || hypothesisSections.length !== 1 || hypothesisSections[0].chart_ids.length !== 0
    || hypothesisSections[0].table_ids.length !== 1 || hypothesisSections[0].table_ids[0] !== "exact_case_bootstrap_hypothesis_tests") cbsemWireFail(`${path}.bootstrap_hypothesis_tests`, "adapter v10/v11 requires the complete hypothesis section");
  const hypothesis = cbsemCanonicalTable(document, "exact_case_bootstrap_hypothesis_tests", CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_CANONICAL_COLUMNS, `${path}.exact_case_bootstrap_hypothesis_tests`);
  const globalUnavailable = tests.inference.status === "unavailable" ? tests.inference : null;
  if (hypothesis.rows.length !== tests.parameters.length || tests.parameters.some((parameter, index) => {
    const canonical = hypothesis.rows[index];
    const outcome = parameter.outcome;
    const outcomeCells: CbsemCanonicalExpectedCell[] = outcome.status === "available" ? [
      outcome.point_estimate, outcome.two_sided_exceedances, outcome.greater_or_equal_exceedances,
      outcome.less_or_equal_exceedances, outcome.p_value_two_sided, outcome.p_value_greater,
      outcome.p_value_less, outcome.selected_exceedances, outcome.selected_p_value, outcome.reject_null,
    ] : Array.from({ length: 10 }, () => null);
    const expected: CbsemCanonicalExpectedCell[] = [
      tests.method_version, tests.null_hypothesis, tests.statistic, tests.tie_policy,
      tests.probability_method, tests.decision_rule, tests.selected_test_tail, tests.null_value,
      tests.significance_level, tests.usable_replicates, tests.inference.status,
      globalUnavailable?.reason_code ?? null, globalUnavailable?.message ?? null, parameter.parameter_id,
      outcome.status, ...outcomeCells, outcome.status === "unavailable" ? outcome.reason : null,
    ];
    return canonical.id !== `bootstrap_hypothesis_${String(index).padStart(4, "0")}`
      || !cbsemCanonicalRowMatches(canonical.cells, expected);
  })) cbsemWireFail(`${path}.exact_case_bootstrap_hypothesis_tests`, "does not exactly bind the analytical hypothesis receipt");
}

function bindCbsemExactBootstrapStudentizedCanonicalV1(
  document: CanonicalResultDocumentV2,
  sidecar: CbsemExactCaseBootstrapStudentizedSidecarV1,
): void {
  const path = "completedResult.canonicalDocument";
  const tableIds = [
    "exact_case_bootstrap_studentized_summary",
    "exact_case_bootstrap_studentized_point_standard_errors",
    "exact_case_bootstrap_studentized_parameter_intervals",
    "exact_case_bootstrap_studentized_refit_standard_errors",
  ];
  const sections = document.sections.filter((section) => section.id === "bootstrap_studentized_inference");
  if (sections.length !== 1 || sections[0].chart_ids.length !== 0
    || JSON.stringify(sections[0].table_ids) !== JSON.stringify(tableIds)) {
    cbsemWireFail(`${path}.bootstrap_studentized_inference`, "has drifted v11 studentized table ownership or order");
  }
  const summary = cbsemCanonicalTable(document, tableIds[0], CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_CANONICAL_COLUMNS, `${path}.${tableIds[0]}`);
  const point = cbsemCanonicalTable(document, tableIds[1], CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_CANONICAL_COLUMNS, `${path}.${tableIds[1]}`);
  const intervals = cbsemCanonicalTable(document, tableIds[2], CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_CANONICAL_COLUMNS, `${path}.${tableIds[2]}`);
  const refits = cbsemCanonicalTable(document, tableIds[3], CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_CANONICAL_COLUMNS, `${path}.${tableIds[3]}`);
  const unavailable = sidecar.inference.status === "unavailable" ? sidecar.inference : null;
  if (summary.rows.length !== 1 || summary.rows[0].id !== "bootstrap_studentized"
    || !cbsemCanonicalRowMatches(summary.rows[0].cells, [
      sidecar.method_version, sidecar.standard_error_method_version, sidecar.expected_information_method,
      sidecar.pivot_method, sidecar.quantile_method, sidecar.interval_method,
      sidecar.archive_validation_scope, sidecar.confidence_level, sidecar.minimum_usable_fraction,
      sidecar.minimum_usable_replicates, sidecar.studentized_usable_replicates,
      JSON.stringify(sidecar.parameter_ids), sidecar.inference.status, unavailable?.reason ?? null,
      unavailable?.message ?? null,
    ])) {
    cbsemWireFail(`${path}.${tableIds[0]}`, "does not exactly bind the analytical studentized summary");
  }
  const pointOutcome = sidecar.point_standard_errors.outcome;
  if (point.rows.length !== sidecar.parameter_ids.length || sidecar.parameter_ids.some((parameterId, index) => {
    const canonical = point.rows[index];
    const available = pointOutcome.status === "available" ? pointOutcome.parameters[index] : null;
    return canonical.id !== `bootstrap_studentized_point_standard_error_${String(index).padStart(4, "0")}`
      || !cbsemCanonicalRowMatches(canonical.cells, [
        sidecar.point_standard_errors.method_version, parameterId, pointOutcome.status,
        pointOutcome.status === "available" ? pointOutcome.information_method : null,
        available?.standard_error ?? null,
        pointOutcome.status === "unavailable" ? pointOutcome.reason : null,
      ]);
  })) cbsemWireFail(`${path}.${tableIds[1]}`, "does not exactly bind the point analytical standard-error receipt");
  if (intervals.rows.length !== sidecar.intervals.length || sidecar.intervals.some((interval, index) => {
    const canonical = intervals.rows[index];
    const outcome = interval.outcome;
    const values: CbsemCanonicalExpectedCell[] = outcome.status === "available" ? [
      outcome.point_estimate, outcome.point_standard_error, outcome.lower_pivot_quantile,
      outcome.upper_pivot_quantile, outcome.interval_lower, outcome.interval_upper,
      outcome.usable_replicates, null,
    ] : [null, null, null, null, null, null, null, outcome.reason];
    return canonical.id !== `bootstrap_studentized_interval_${String(index).padStart(4, "0")}`
      || !cbsemCanonicalRowMatches(canonical.cells, [interval.parameter_id, outcome.status, ...values]);
  })) cbsemWireFail(`${path}.${tableIds[2]}`, "does not bit-exactly bind the studentized interval outcomes");
  if (refits.rows.length !== sidecar.refit_standard_errors.length || sidecar.refit_standard_errors.some((receipt, index) => {
    const canonical = refits.rows[index];
    const outcome = receipt.outcome;
    return canonical.id !== `bootstrap_studentized_refit_standard_error_${String(receipt.replicate_index).padStart(5, "0")}`
      || !cbsemCanonicalRowMatches(canonical.cells, [
        receipt.replicate_index, outcome.status,
        outcome.status === "available" ? outcome.information_method : null,
        outcome.status === "available" ? JSON.stringify(outcome.standard_errors) : null,
        outcome.status === "unavailable" ? outcome.reason : null,
      ]);
  })) cbsemWireFail(`${path}.${tableIds[3]}`, "does not exactly bind the compact refit standard-error receipts");
}

function bindCbsemExactBootstrapBcaCanonicalV1(
  document: CanonicalResultDocumentV2,
  sidecar: CbsemExactCaseBootstrapBcaSidecarV1,
): void {
  const path = "completedResult.canonicalDocument";
  const tableIds = [
    "exact_case_bootstrap_bca_summary",
    "exact_case_bootstrap_bca_parameter_intervals",
    "exact_case_bootstrap_bca_successful_delete_one_refits",
    "exact_case_bootstrap_bca_failures",
  ];
  const sections = document.sections.filter((section) => section.id === "bootstrap_bca_inference");
  if (sections.length !== 1 || sections[0].chart_ids.length !== 0
    || JSON.stringify(sections[0].table_ids) !== JSON.stringify(tableIds)) {
    cbsemWireFail(`${path}.bootstrap_bca_inference`, "has drifted v12 BCa table ownership or order");
  }
  const summary = cbsemCanonicalTable(document, tableIds[0], CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_CANONICAL_COLUMNS, `${path}.${tableIds[0]}`);
  const intervals = cbsemCanonicalTable(document, tableIds[1], CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_CANONICAL_COLUMNS, `${path}.${tableIds[1]}`);
  const successes = cbsemCanonicalTable(document, tableIds[2], CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_CANONICAL_COLUMNS, `${path}.${tableIds[2]}`);
  const failures = cbsemCanonicalTable(document, tableIds[3], CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_CANONICAL_COLUMNS, `${path}.${tableIds[3]}`);
  const unavailable = sidecar.inference.status === "unavailable" ? sidecar.inference : null;
  if (summary.rows.length !== 1 || summary.rows[0].id !== "bootstrap_bca"
    || !cbsemCanonicalRowMatches(summary.rows[0].cells, [
      sidecar.method_version, sidecar.base_bootstrap_method_version,
      sidecar.outer_recipe_analytical_identity_sha256, sidecar.base_point_result_sha256,
      sidecar.compiler_analytical_identity_sha256, sidecar.plan_sha256,
      sidecar.model_scientific_sha256, sidecar.delete_one_refit_method_version,
      "sha256_complete_case_n_and_ordered_sampling_positions_v1",
      "sha256_source_fingerprint_and_ordered_u64_indices_v1",
      sidecar.bias_correction_method, sidecar.acceleration_method, sidecar.adjusted_probability_method,
      sidecar.quantile_method, sidecar.retry_policy, CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE,
      sidecar.confidence_level, sidecar.bootstrap_usable_replicates,
      sidecar.minimum_bootstrap_usable_replicates, sidecar.delete_one_case_count,
      sidecar.successful_delete_one_refits.length, sidecar.failed_delete_one_refits.length,
      JSON.stringify(sidecar.parameter_ids), sidecar.inference.status, unavailable?.reason ?? null,
      unavailable?.message ?? null,
    ])) cbsemWireFail(`${path}.${tableIds[0]}`, "does not exactly bind the analytical BCa summary");
  if (intervals.rows.length !== sidecar.intervals.length || sidecar.intervals.some((interval, index) => {
    const canonical = intervals.rows[index];
    const outcome = interval.outcome;
    const values: CbsemCanonicalExpectedCell[] = outcome.status === "available" ? [
      outcome.point_estimate, outcome.bias_correction, outcome.acceleration,
      outcome.adjusted_lower_probability, outcome.adjusted_upper_probability,
      outcome.interval_lower, outcome.interval_upper, outcome.usable_replicates, null,
    ] : [null, null, null, null, null, null, null, null, outcome.reason];
    return canonical.id !== `bootstrap_bca_interval_${String(index).padStart(4, "0")}`
      || !cbsemCanonicalRowMatches(canonical.cells, [interval.parameter_id, outcome.status, ...values]);
  })) cbsemWireFail(`${path}.${tableIds[1]}`, "does not bit-exactly bind the BCa interval outcomes");
  if (successes.rows.length !== sidecar.successful_delete_one_refits.length
    || sidecar.successful_delete_one_refits.some((row, index) => {
      const canonical = successes.rows[index];
      return canonical.id !== `bootstrap_bca_delete_one_refit_${String(row.omitted_complete_case_position).padStart(5, "0")}`
        || !cbsemCanonicalRowMatches(canonical.cells, [
          row.omitted_complete_case_position, row.omitted_source_row_index,
          row.retained_sampling_positions_sha256, row.retained_sample_indices_sha256,
          JSON.stringify(row.parameter_estimates), row.iterations, row.objective, row.gradient_norm,
        ]);
    })) cbsemWireFail(`${path}.${tableIds[2]}`, "does not exactly bind the successful delete-one ledger");
  if (failures.rows.length !== sidecar.failed_delete_one_refits.length
    || sidecar.failed_delete_one_refits.some((row, index) => {
      const canonical = failures.rows[index];
      return canonical.id !== `bootstrap_bca_delete_one_failure_${String(row.omitted_complete_case_position).padStart(5, "0")}`
        || !cbsemCanonicalRowMatches(canonical.cells, [
          row.omitted_complete_case_position, row.omitted_source_row_index,
          row.retained_sampling_positions_sha256, row.retained_sample_indices_sha256,
          row.kind, row.message,
        ]);
    })) cbsemWireFail(`${path}.${tableIds[3]}`, "does not exactly bind the failed delete-one ledger");
}

function documentHasExactBootstrapArtifacts(document: CanonicalResultDocumentV2): boolean {
  const tableIds = new Set([
    "exact_case_bootstrap_summary", "exact_case_bootstrap_parameter_intervals",
    "exact_case_bootstrap_successful_refits", "exact_case_bootstrap_failures",
    "exact_case_bootstrap_hypothesis_tests",
    "exact_case_bootstrap_studentized_summary", "exact_case_bootstrap_studentized_point_standard_errors",
    "exact_case_bootstrap_studentized_parameter_intervals", "exact_case_bootstrap_studentized_refit_standard_errors",
    "exact_case_bootstrap_bca_summary", "exact_case_bootstrap_bca_parameter_intervals",
    "exact_case_bootstrap_bca_successful_delete_one_refits", "exact_case_bootstrap_bca_failures",
  ]);
  return document.tables.some((table) => tableIds.has(table.id))
    || document.sections.some((section) => section.id === "bootstrap_inference"
      || section.id === "bootstrap_hypothesis_tests" || section.id === "bootstrap_studentized_inference"
      || section.id === "bootstrap_bca_inference");
}

function documentHasExactBootstrapStudentizedArtifacts(document: CanonicalResultDocumentV2): boolean {
  const tableIds = new Set([
    "exact_case_bootstrap_studentized_summary", "exact_case_bootstrap_studentized_point_standard_errors",
    "exact_case_bootstrap_studentized_parameter_intervals", "exact_case_bootstrap_studentized_refit_standard_errors",
  ]);
  return document.tables.some((table) => tableIds.has(table.id))
    || document.sections.some((section) => section.id === "bootstrap_studentized_inference");
}

function documentHasExactBootstrapBcaArtifacts(document: CanonicalResultDocumentV2): boolean {
  const tableIds = new Set([
    "exact_case_bootstrap_bca_summary", "exact_case_bootstrap_bca_parameter_intervals",
    "exact_case_bootstrap_bca_successful_delete_one_refits", "exact_case_bootstrap_bca_failures",
  ]);
  return document.tables.some((table) => tableIds.has(table.id))
    || document.sections.some((section) => section.id === "bootstrap_bca_inference");
}

/** Validates the native analytical/canonical pair before later persistence wiring. */
export function parseInternalRecipeV4CbsemCompletedResultV1(input: unknown): InternalRecipeV4CbsemCompletedResultV1 {
  const completed = cbsemWireExactRecord(input, ["schemaVersion", "analyticalResult", "canonicalDocument"], [], "completedResult");
  if (completed.schemaVersion !== 1) cbsemWireFail("completedResult.schemaVersion", "must equal 1");
  const analyticalResult = parseInternalRecipeV4CbsemExecutionResultV1(completed.analyticalResult);
  const canonicalDocument = completed.canonicalDocument as CanonicalResultDocumentV2;
  const validation = validateCanonicalResultDocumentV2(canonicalDocument);
  if (!validation.passed) cbsemWireFail("completedResult.canonicalDocument", validation.errors.join("; "));
  const completedBootstrapOwner = [
    "compiled_recipe_v4_cbsem_plan_v2_execution_v9",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v10",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v11",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v12",
  ].includes(analyticalResult.provenance.adapter_version);
  const canonicalFingerprintMatches = completedBootstrapOwner
    ? cbsemExactBootstrapSourceFingerprintPayloadV1(canonicalDocument.provenance.dataset_fingerprint)
      === cbsemExactBootstrapSourceFingerprintPayloadV1(analyticalResult.estimation.input.dataset_fingerprint)
    : canonicalDocument.provenance.dataset_fingerprint === analyticalResult.estimation.input.dataset_fingerprint;
  if (canonicalDocument.provenance.dataset_id !== analyticalResult.estimation.input.dataset_id
    || !canonicalFingerprintMatches
    || canonicalDocument.provenance.method_version !== analyticalResult.provenance.estimator_method_version
    || canonicalDocument.provenance.engine_version !== analyticalResult.provenance.adapter_version) cbsemWireFail("completedResult.canonicalDocument", "has drifted adapter, estimator, or dataset identity");
  const scoreTables = canonicalDocument.tables.filter((table) => table.id === "modification_index_score_tests");
  const scoreSections = canonicalDocument.sections.filter((section) => section.id === "modification_indices");
  const bundle = analyticalResult.estimation.analysis.score_lm;
  const scoreLmOwner = [
    "compiled_recipe_v4_cbsem_plan_v2_execution_v8",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v9",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v10",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v11",
    "compiled_recipe_v4_cbsem_plan_v2_execution_v12",
  ].includes(analyticalResult.provenance.adapter_version);
  if (!scoreLmOwner) {
    if (scoreTables.length || scoreSections.length || canonicalDocument.tables.some((table) => table.id === "modification_indices")) cbsemWireFail("completedResult.canonicalDocument", "pre-v8 result carries a score/LM or masquerading heuristic artifact");
  } else {
    if (!bundle || scoreTables.length !== 1 || scoreSections.length !== 1
      || scoreSections[0].table_ids.length !== 1 || scoreSections[0].table_ids[0] !== "modification_index_score_tests"
      || scoreSections[0].chart_ids.length !== 0) cbsemWireFail("completedResult.canonicalDocument", "current adapter requires exactly one score/LM table and owning section");
    const table = scoreTables[0];
    if (table.columns.length !== CBSEM_SCORE_LM_CANONICAL_COLUMNS.length
      || table.columns.some((column, index) => column.id !== CBSEM_SCORE_LM_CANONICAL_COLUMNS[index])
      || table.rows.length !== bundle.rows.length) cbsemWireFail("completedResult.canonicalDocument.modification_index_score_tests", "has drifted columns or row cardinality");
    table.rows.forEach((canonicalRow, index) => {
      const raw = bundle.rows[index]!;
      const cells = canonicalRow.cells;
      const textEquals = (cellIndex: number, expected: string) => cells[cellIndex]?.kind === "text" && cells[cellIndex].value === expected;
      if (cells.length !== CBSEM_SCORE_LM_CANONICAL_COLUMNS.length
        || !textEquals(0, bundle.method_version) || !textEquals(1, bundle.scope)
        || !textEquals(2, raw.parameter_id) || !textEquals(3, raw.kind)
        || !textEquals(4, raw.lhs) || !textEquals(5, raw.rhs)
        || !textEquals(6, raw.outcome.status)) cbsemWireFail(`completedResult.canonicalDocument.modification_index_score_tests.rows[${index}]`, "does not bind the analytical score/LM identity");
      if (raw.outcome.status === "available") {
        const expected = [raw.outcome.score, raw.outcome.efficient_score, raw.outcome.candidate_information, raw.outcome.efficient_information, raw.outcome.modification_index, raw.outcome.expected_parameter_change, 1, raw.outcome.p_value];
        if (expected.some((value, offset) => {
          const cell = cells[offset + 7];
          return cell?.kind !== "number" || !Object.is(cell.value, value);
        })
          || cells[15]?.kind !== "missing" || cells[15].reason !== "not_applicable" || cells[15].display !== undefined) cbsemWireFail(`completedResult.canonicalDocument.modification_index_score_tests.rows[${index}]`, "does not bit-exactly bind the available analytical outcome");
      } else if (cells.slice(7, 15).some((cell) => cell.kind !== "missing" || cell.reason !== "not_estimated" || cell.display !== undefined)
        || !textEquals(15, raw.outcome.reason)) cbsemWireFail(`completedResult.canonicalDocument.modification_index_score_tests.rows[${index}]`, "does not exactly bind the unavailable analytical outcome");
    });
  }
  const exactBootstrap = analyticalResult.estimation.analysis.exact_case_bootstrap;
  const exactBootstrapStudentized = analyticalResult.estimation.analysis.exact_case_bootstrap_studentized;
  const exactBootstrapBca = analyticalResult.estimation.analysis.exact_case_bootstrap_bca;
  const baseBootstrapOwner = analyticalResult.provenance.adapter_version === "compiled_recipe_v4_cbsem_plan_v2_execution_v9"
    || analyticalResult.provenance.adapter_version === "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
  const studentizedBootstrapOwner = analyticalResult.provenance.adapter_version === "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
  const bcaBootstrapOwner = analyticalResult.provenance.adapter_version === "compiled_recipe_v4_cbsem_plan_v2_execution_v12";
  const exactArtifactInjected = documentHasExactBootstrapArtifacts(canonicalDocument);
  if (baseBootstrapOwner) {
    if (!exactBootstrap) cbsemWireFail("completedResult.analyticalResult", "current exact-bootstrap adapter omitted its analytical aggregate");
    if (documentHasExactBootstrapStudentizedArtifacts(canonicalDocument)
      || documentHasExactBootstrapBcaArtifacts(canonicalDocument)) {
      cbsemWireFail("completedResult.canonicalDocument", "v9/v10 result carries injected newer-generation bootstrap artifacts");
    }
    bindCbsemExactBootstrapCanonicalV1(canonicalDocument, exactBootstrap, analyticalResult.provenance.adapter_version);
  } else if (studentizedBootstrapOwner) {
    if (!exactBootstrapStudentized) cbsemWireFail("completedResult.analyticalResult", "adapter v11 omitted its atomic analytical wrapper");
    if (documentHasExactBootstrapBcaArtifacts(canonicalDocument)) {
      cbsemWireFail("completedResult.canonicalDocument", "v11 result carries injected v12 BCa artifacts");
    }
    bindCbsemExactBootstrapCanonicalV1(canonicalDocument, exactBootstrapStudentized.base, analyticalResult.provenance.adapter_version);
    bindCbsemExactBootstrapStudentizedCanonicalV1(canonicalDocument, exactBootstrapStudentized.studentized);
  } else if (bcaBootstrapOwner) {
    if (!exactBootstrapBca) cbsemWireFail("completedResult.analyticalResult", "adapter v12 omitted its atomic analytical wrapper");
    bindCbsemExactBootstrapCanonicalV1(canonicalDocument, exactBootstrapBca.base, analyticalResult.provenance.adapter_version);
    bindCbsemExactBootstrapBcaCanonicalV1(canonicalDocument, exactBootstrapBca.bca);
  } else if (exactBootstrap || exactBootstrapStudentized || exactBootstrapBca || exactArtifactInjected) {
    cbsemWireFail("completedResult.canonicalDocument", "pre-v9 result carries exact case-bootstrap artifacts");
  }
  return { schemaVersion: 1, analyticalResult, canonicalDocument };
}

export interface InternalRecipeV4CbsemJobSnapshotV1 {
  schemaVersion: 1;
  jobId: string;
  state: InternalRecipeV4PlsJobStateV1;
  phase: string;
  completedUnits: number;
  totalUnits: number;
  message: string | null;
  failure: InternalRecipeV4ExecutionFailureV1 | null;
  queuedAt: string;
  startedAt: string | null;
  completedAt: string | null;
}
