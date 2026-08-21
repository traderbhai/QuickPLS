import {
  methodResultTables,
  type ResultTable,
  type ResultTableAdvisory,
} from "../domain/resultTables";
import { parseParameterIdentity } from "../domain/inference";
import type { ResultOverlaySelectionV1 } from "../domain/moderationDiagramProjectionV1";
import type {
  AnalysisRun,
  CbsemAnalysis,
  CtaPlsAnalysis,
  CvpatBenchmarkAssessment,
  HtmtAssessment,
  HtmtBootstrapInference,
  PlsModelFit,
  PlsModelFitExactCriterionInference,
  PlsModelFitExactInference,
  PlsModelFitExactStatus,
  PlsModelFitExactVariantInference,
  PlsPredictIndicatorTarget,
  PlsPredictTarget,
  PcaAnalysis,
  RegressionAnalysis,
  RegressionBootstrapAnalysis,
  GscaAnalysis,
} from "../types";
import { nativeIpmaPredecessorIds } from "./nativeIpma";
import {
  CURRENT_CVPAT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION,
  LEGACY_PLS_PREDICT_METHOD_VERSION,
  LEGACY_PLS_PREDICT_REPEATED_METHOD_VERSION,
} from "./nativeCalculationMode";
import { NATIVE_NCA_ENGINE_SCOPE_WARNING, NATIVE_STANDALONE_ASSESSMENT_WARNING } from "./nativeNca";
import { NATIVE_PCA_ENGINE_SCOPE_WARNING } from "./nativePca";
import { NATIVE_OLS_ENGINE_SCOPE_WARNING } from "./nativeOls";
import {
  NATIVE_LEGACY_LOGISTIC_ENGINE_SCOPE_WARNING,
  NATIVE_LOGISTIC_ENGINE_SCOPE_WARNING,
} from "./nativeLogistic";
import { isStandaloneNativeAnalysis } from "./nativeStandaloneAnalysis";
import { parseCbsemCfaScoreLmBundleV1 } from "../domain/internalRecipeV4CbsemExecution";
import { isNativeRegressionBootstrapValidationWitness } from "./nativeRegressionBootstrapWitness";
import {
  NATIVE_GSCA_ALGORITHM_VERSION,
  NATIVE_GSCA_ASSESSMENT_WARNING,
  NATIVE_GSCA_ENGINE_SCOPE_WARNING,
  NATIVE_GSCA_METHOD_VERSION,
} from "./nativeGsca";
import {
  NATIVE_CTA_PLS_COVARIANCE_VERSION,
  NATIVE_CTA_PLS_ESTIMATION_WARNING,
  NATIVE_CTA_PLS_METHOD_VERSION,
  NATIVE_CTA_PLS_PAIRINGS,
  NATIVE_CTA_PLS_RESULT_WARNING,
  nativeCtaPlsEligibleBlocks,
  type NativeCtaPlsEligibleBlock,
} from "./nativeCtaPls";
import {
  NATIVE_LEGACY_PROCESS_RESULT_IDS,
  NATIVE_PROCESS_RESULT_IDS,
  nativeLegacyProcessResultProjection,
  nativeLegacyProcessResultTables,
  nativeProcessResultProjection,
  nativeProcessResultTables,
} from "./nativeProcessResults";
import {
  nativeStructuralPathRandomizationProjection,
  nativeStructuralPathRandomizationTable,
} from "./nativeStructuralPathRandomization";
import { nativePlscConsistentBootstrapProjection } from "./nativeConsistentBootstrap";
import { nativePlscConsistentPermutationProjection } from "./nativeConsistentPermutation";
import {
  nativePlsSampleSizePowerExportTables,
  nativePlsSampleSizePowerPresentation,
  validateNativePlsSampleSizePowerResult,
  type NativePlsSampleSizePowerPresentation,
  type NativePlsSampleSizePowerRecipeV1,
  type NativePlsSampleSizePowerRecipeV2,
  type NativePlsSampleSizePowerResultV1,
  type NativePlsSampleSizePowerResultV2,
} from "./nativePlsSampleSizePower";

export {
  nativeLegacyProcessResultProjection,
  nativeProcessResultProjection,
} from "./nativeProcessResults";

export type NativeResultGroupId = "graphical" | "groups" | "assessment" | "covariance_sem" | "gsca_component_model" | "sample_size_power" | "importance_performance" | "necessary_conditions" | "components" | "process" | "regression" | "higher_order" | "final_results" | "mediation" | "moderation" | "three_way_moderation" | "quality_criteria" | "prediction" | "inference";

export type NativeResultNavigationItem =
  | {
      id: "model_estimates";
      kind: "diagram";
      title: "Model estimates";
      diagram: "model_estimates";
    }
  | {
      id: string;
      kind: "table";
      title: string;
      tableId: string;
    };

export interface NativeResultNavigationGroup {
  id: NativeResultGroupId;
  title: string;
  items: NativeResultNavigationItem[];
}

export interface NativeResultNavigation {
  runId: string | null;
  defaultItemId: string | null;
  groups: NativeResultNavigationGroup[];
  tables: ResultTable[];
}

export interface NativeModerationPlot {
  title: string;
  predictorLabel: string;
  outcomeLabel: string;
  moderatorLabel: string;
  slopes: Array<{ moderatorScore: number; effect: number; label: string }>;
}

export interface NativeIpmaPlot {
  targetId: string;
  targetLabel: string;
  scopeNote: string;
  points: Array<{
    constructId: string;
    constructLabel: string;
    importance: number;
    performance: number;
  }>;
}

export interface NativeNcaPlot {
  xLabel: string;
  yLabel: string;
  ceiling: "ce_fdh" | "cr_fdh" | "both";
  scope: { minimumX: number; maximumX: number; minimumY: number; maximumY: number };
  ceFdhPeers: Array<{ x: number; y: number }>;
  crFdh: { slope: number; intercept: number } | null;
}

export interface NativeNcaCeFdhPeerRow {
  peerIdentity: string;
  conditionVariable: string;
  conditionValue: number;
  outcomeVariable: string;
  outcomeValue: number;
}

export interface NativeNcaResultProjection {
  methodVersion: "nca_v2";
  x: string;
  y: string;
  observations: number;
  ceiling: "ce_fdh" | "cr_fdh" | "both";
  permutationSamples: number;
  usablePermutations: number;
  scope: NativeNcaPlot["scope"];
  ceFdhPeers: NativeNcaCeFdhPeerRow[];
  ceilingEffects: Array<{ ceiling: "ce_fdh" | "cr_fdh"; effectSize: number; permutationPValue: number; slope: number | null; intercept: number | null }>;
  bottlenecks: Array<{ ceiling: "ce_fdh" | "cr_fdh"; outcomePercent: number; requiredXPercent: number | null; status: "required" | "not_necessary" | "not_attainable" }>;
  warnings: string[];
  plot: NativeNcaPlot;
}

export interface NativePcaResultProjection {
  methodVersion: "pca_v1";
  componentRule: "kaiser" | "fixed" | "variance_threshold";
  retainedComponents: number;
  observations: number;
  variables: string[];
  components: PcaAnalysis["components"];
  loadings: PcaAnalysis["loadings"];
  scoresStored: number;
  warnings: string[];
}

export interface NativeCtaPlsResultProjection {
  methodVersion: typeof NATIVE_CTA_PLS_METHOD_VERSION;
  covarianceVersion: typeof NATIVE_CTA_PLS_COVARIANCE_VERSION;
  usedObservations: number;
  omittedObservations: number;
  blocks: NativeCtaPlsEligibleBlock[];
  estimates: CtaPlsAnalysis["estimates"];
  maxAbsoluteTetradByConstruct: Record<string, number>;
  warnings: string[];
}

export interface NativeOlsResultProjection {
  methodVersion: "regression_ols_v1";
  outcome: string;
  predictors: string[];
  controls: string[];
  observations: number;
  coefficients: RegressionAnalysis["coefficients"];
  fit: NonNullable<RegressionAnalysis["fit"]>;
  predictionsStored: number;
  bootstrap: RegressionBootstrapAnalysis | null;
  warnings: string[];
}

export interface NativeLogisticResultProjection {
  methodVersion: "regression_logistic_v2";
  outcome: string;
  predictors: string[];
  controls: string[];
  observations: number;
  coefficients: RegressionAnalysis["coefficients"];
  fit: NonNullable<RegressionAnalysis["fit"]>;
  predictions: RegressionAnalysis["predictions"];
  diagnostics: NonNullable<RegressionAnalysis["logistic"]>;
  bootstrap: RegressionBootstrapAnalysis | null;
  warnings: string[];
}

export interface NativeLegacyLogisticResultProjection {
  methodVersion: "regression_logistic_v1";
  recordedPreprocessing: "standardized" | "unstandardized";
  outcome: string;
  predictors: string[];
  controls: string[];
  observations: number;
  coefficients: RegressionAnalysis["coefficients"];
  fit: NonNullable<RegressionAnalysis["fit"]>;
  predictions: RegressionAnalysis["predictions"];
  warnings: string[];
}

export interface NativeCbsemResultProjection {
  methodVersion: "cfa_ml_v1" | "cbsem_ml_v1";
  modelType: "cfa" | "sem";
  analysis: CbsemAnalysis;
}

export interface NativeGscaResultProjection {
  methodVersion: typeof NATIVE_GSCA_METHOD_VERSION;
  algorithmVersion: typeof NATIVE_GSCA_ALGORITHM_VERSION;
  analysis: GscaAnalysis & {
    algorithm: string;
    converged: boolean;
    stop_criterion: number;
    final_change: number;
    objective: number;
    measurement_fit: number;
    structural_fit: number;
    srmr: number;
    covariance_discrepancy: number;
    covariance_sample_total: number;
    standardized_residual_sum: number;
    observations: number;
    free_parameters: number;
  };
  usedObservations: number;
  omittedObservations: number;
  constructModes: Readonly<Record<string, "reflective" | "formative">>;
}

type TableDraft = Omit<ResultTable, "status"> & { status?: ResultTable["status"] };
type ConstructDisplayLabel = (constructId: string) => string;

const FINAL_RESULT_IDS = [
  "path_coefficients",
  "control_effects",
  "outer_loadings",
  "outer_weights",
  "r_squared",
  "total_effects",
] as const;

const MODERATION_IDS = [
  "moderation_effects",
  "moderation_simple_slopes",
  "moderation_bootstrap",
  "moderation_bca",
  "moderation_studentized",
  "moderation_randomization",
] as const;

const MEDIATION_IDS = [
  "direct_effects",
  "specific_indirect_effects",
  "total_indirect_effects",
  "total_effects",
  "mediation_bootstrap",
] as const;

const HIGHER_ORDER_IDS = [
  "hoc_component_relationships",
  "hoc_structural_paths",
  "hoc_scope",
  "general_sem_higher_order_targets",
  "general_sem_higher_order_stages",
  "general_sem_higher_order_bootstrap_receipt",
] as const;

const QUALITY_CRITERIA_IDS = [
  "plsc_reliability",
  "plsc_correlations",
  "wpls_weights",
  "construct_reliability",
  "cross_loadings",
  "fornell_larcker",
  "htmt_plus",
  "htmt_original",
  "htmt_plus_bootstrap",
  "htmt_original_bootstrap",
  "htmt",
  "structural_quality",
  "structural_vif",
  "formative_indicator_vif",
  "f_squared",
  "model_fit",
  "model_fit_details",
  "blindfolding",
] as const;

const INFERENCE_IDS = [
  "model_fit_exact",
  "model_fit_exact_failures",
  "plsc_bootstrap_accounting",
  "plsc_bootstrap_failures",
  "plsc_bootstrap_jackknife_failures",
  "plsc_permutation_accounting",
  "plsc_permutation_groups",
  "plsc_permutation_paths",
  "plsc_permutation_outer_loadings",
  "plsc_permutation_construct_criteria",
  "plsc_permutation_failures",
  "control_bootstrap",
  "control_bca",
  "control_studentized",
  "control_randomization",
  "bootstrap_accounting",
  "bootstrap_failures",
  "bootstrap_percentile",
  "bootstrap_one_sided_test_tail",
  "bootstrap_bca",
  "bootstrap_bca_unavailable",
  "plsc_bootstrap_bca_unavailable",
  "bootstrap_studentized",
  "permutation",
] as const;

const PREDICTION_V2_IDS = [
  "plspredict_indicator_summary",
  "cvpat_benchmark_assessment",
  "plspredict_validation_plan",
  "plspredict_construct_summary",
  "plspredict_holdout_indicator_summary",
  "plspredict_holdout_construct_summary",
  "plspredict_holdout_split",
] as const;

const PREDICTION_V1_IDS = [
  "plspredict_holdout",
  "plspredict_split",
  "plspredict_repeated_kfold",
  "plspredict_repeated_kfold_plan",
  "cvpat",
] as const;

const PREDICTION_IDS = [...PREDICTION_V2_IDS, ...PREDICTION_V1_IDS] as const;

const MGA_GROUP_IDS = [
  "mga_group_summary",
  "micom_summary",
  "micom_configural",
  "micom_composition",
  "micom_means",
  "micom_variances",
  "micom_permutation_accounting",
  "mga_group_paths",
  "mga_group_r_squared",
  "mga_group_loadings",
  "mga_group_weights",
  "mga_path_differences",
  "mga_loading_differences",
  "mga_weight_differences",
  "mga_permutation",
  "mga_permutation_loadings",
  "mga_permutation_weights",
  "mga_permutation_accounting",
] as const;

const CCA_ASSESSMENT_IDS = [
  "cca_residual_summary",
  "cca_composite_residuals",
] as const;

const CTA_PLS_ASSESSMENT_IDS = [
  "cta_pls_summary",
  "cta_pls_tetrads",
  "cta_pls_scope",
] as const;

const ENDOGENEITY_ASSESSMENT_IDS = ["endogeneity_copula"] as const;

const IPMA_RESULT_IDS = [
  "ipma_constructs",
  "ipma_indicators",
  "ipma_scope",
] as const;

const NCA_RESULT_IDS = [
  "nca_ceiling_effects",
  "nca_ce_fdh_peers",
  "nca_cr_line",
  "nca_bottlenecks",
  "nca_scope",
] as const;

const PCA_RESULT_IDS = [
  "pca_component_summary",
  "pca_loadings",
  "pca_scope",
] as const;

const OLS_RESULT_IDS = [
  "ols_coefficients",
  "ols_model_fit",
  "ols_scope",
  "regression_bootstrap_summary",
  "regression_bootstrap_failures",
  "regression_bootstrap_coefficients",
  "regression_bootstrap_percentile",
  "regression_bootstrap_bca",
] as const;

const LOGISTIC_RESULT_IDS = [
  "logistic_coefficients",
  "logistic_fit",
  "logistic_classification",
  "logistic_outcome_profile",
  "logistic_convergence",
  "logistic_probabilities",
  "logistic_scope",
  "regression_bootstrap_summary",
  "regression_bootstrap_failures",
  "regression_bootstrap_coefficients",
  "regression_bootstrap_percentile",
  "regression_bootstrap_bca",
  "regression_bootstrap_odds_ratios",
] as const;

const LEGACY_LOGISTIC_RESULT_IDS = [
  "legacy_logistic_coefficients",
  "legacy_logistic_fit",
  "legacy_logistic_probabilities",
  "legacy_logistic_scope",
] as const;

const CBSEM_RESULT_IDS = [
  "cbsem_fit",
  "cbsem_exact_bootstrap_intervals",
  "cbsem_exact_bootstrap_hypothesis_tests",
  "cbsem_exact_bootstrap_successful_refits",
  "cbsem_exact_bootstrap_failures",
  "cbsem_exact_bootstrap_settings",
  "exact_case_bootstrap_studentized_summary",
  "exact_case_bootstrap_studentized_point_standard_errors",
  "exact_case_bootstrap_studentized_parameter_intervals",
  "exact_case_bootstrap_studentized_refit_standard_errors",
  "exact_case_bootstrap_bca_summary",
  "exact_case_bootstrap_bca_parameter_intervals",
  "exact_case_bootstrap_bca_successful_delete_one_refits",
  "exact_case_bootstrap_bca_failures",
  "cbsem_bootstrap_intervals",
  "cbsem_bootstrap_failures",
  "cbsem_bootstrap_settings",
  "cbsem_standardized_parameters",
  "cbsem_unstandardized_parameters",
  "cbsem_residual_correlations",
  "cbsem_residual_covariances",
  "cbsem_implied_covariances",
  "modification_index_score_tests",
  "cbsem_modification_diagnostics",
  "cbsem_scope",
] as const;

const GSCA_RESULT_IDS = [
  "gsca_fit",
  "gsca_paths",
  "gsca_r_squared",
  "gsca_loadings",
  "gsca_weights",
  "gsca_scope",
] as const;

const CBSEM_FIT_METHOD_VERSION = "cbsem_fit_v1";
const CBSEM_MODIFICATION_METHOD_VERSION = "cbsem_modification_indices_v1";
const CBSEM_SCORE_LM_METHOD_VERSION = "cbsem_cfa_score_lm_v1";
const CBSEM_BOOTSTRAP_METHOD_VERSION_V2 = "cbsem_bootstrap_v2";
const CBSEM_EXACT_BOOTSTRAP_METHOD_VERSION_V1 = "cbsem_exact_case_bootstrap_v1";
const CBSEM_EXACT_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1 =
  "cbsem_exact_case_bootstrap_null_centered_test_tail_v1";
const CBSEM_EXACT_BOOTSTRAP_TRUTH_NOTE =
  "Full exact-ML case bootstrap; percentile Type-7; failed fits retained. Archive validation verified the recorded schedule descriptors and arithmetic but did not replay raw fits or independently authenticate the Rust resampling schedule.";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TRUTH_NOTE =
  "Analytically studentized Labs inference. Archive reopening validates the persisted ledger and interval arithmetic only; it does not replay raw refits or expected-information calculations.";
const CBSEM_EXACT_BOOTSTRAP_BCA_TRUTH_NOTE =
  "BCa Type-7 Labs inference is complete-only across the delete-one schedule. Archive reopening validates persisted ledger identity, digests, and exposed interval arithmetic only; it does not replay raw base or delete-one ML fits.";
export const CBSEM_RMSEA_INTERVAL_METHOD_VERSION_V1 =
  "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1";
export const CBSEM_RMSEA_INTERVAL_CONFIDENCE_LEVEL_V1 = 0.9;
export const PLS_MODEL_FIT_METHOD_VERSION_V2 = "pls_model_fit_v2";
export const PLS_MODEL_FIT_MATRIX_CONVENTION_V2 = "indicator_correlation_lower_triangle_including_diagonal";
export const PLS_MODEL_FIT_GEODESIC_LOGARITHM_V2 = "natural_logarithm";
export const PLS_MODEL_FIT_EXACT_INFERENCE_PROCEDURE_V2 = "adapted_bollen_stine_saturated_and_estimated";

export interface NativeModelFitPresentationStateV2 {
  mode:
    | "higher_order_not_reported"
    | "descriptive"
    | "exact_available"
    | "exact_partial"
    | "exact_unavailable"
    | "exact_failed";
  aggregateStatus: PlsModelFitExactStatus | null;
  detailValue: string;
  advisory: ResultTableAdvisory;
}

export const NATIVE_IPMA_SCOPE_NOTE =
  "Performance uses 0–100 observed-range min–max scaling of listwise-standardized composite scores. No theoretical-range correction is applied.";

const CURRENT_MGA_METHOD_VERSION = "pls_mga_two_group_v4";
const CURRENT_MGA_PERMUTATION_METHOD_VERSION = "pls_mga_permutation_v4";
const CURRENT_COMBINED_MICOM_METHOD_VERSION = "micom_v4";
const HISTORICAL_MGA_METHOD_VERSION_V3 = "pls_mga_two_group_v3";
const HISTORICAL_MGA_PERMUTATION_METHOD_VERSION_V3 = "pls_mga_permutation_v3";
const CURRENT_MICOM_METHOD_VERSION = "micom_v3_1";
const LEGACY_COMBINED_MICOM_METHOD_VERSION_V3 = "micom_v3";
const CURRENT_MICOM_FAILURE_CODES = new Set([
  "micom.configural_review_required",
  "micom.configural_invariance_not_confirmed",
  "micom.empty_group",
  "micom.group_too_small",
  "micom.extreme_group_imbalance",
  "micom.degenerate_indicator",
  "micom.observed_model_fit_failed",
  "micom.score_contract_invalid",
  "micom.degenerate_composite_score",
  "micom.orientation_undefined",
  "micom.insufficient_usable_permutations",
]);
const HISTORICAL_MGA_METHOD_VERSION_V2 = "pls_mga_two_group_v2";
const HISTORICAL_MGA_PERMUTATION_METHOD_VERSION_V2 = "pls_mga_permutation_v2";
const HISTORICAL_MICOM_METHOD_VERSION_V2 = "micom_v2";
const HISTORICAL_MGA_V2_WARNING =
  "Historical MICOM/permutation-MGA v2 result. Its deterministic permutation stream was not invariant to exchanging Group A and Group B, so retain it for archive review only and do not interpret it as a current v4 result.";
const HISTORICAL_MGA_V3_WARNING =
  "Historical combined MICOM/permutation-MGA v3 result. Its replacement-retry schedule is archive-readable only and must not be interpreted as fixed-plan v4 evidence.";

const MAX_SPECIFIC_INDIRECT_EFFECTS = 5_000;
const SPECIFIC_INDIRECT_EFFECTS_TRUNCATED_WARNING =
  "Showing the first 5,000 specific indirect paths. Additional paths were omitted to keep Results responsive.";

export function completedResultRuns(runs: readonly AnalysisRun[]): AnalysisRun[] {
  return runs.filter((run) => isCompletedResultRun(run) || nativePlsSampleSizePowerResultProjection(run) !== null);
}

/**
 * Resolves the canonical native Results selection. The persisted/store
 * selection wins when it names an available completed run; otherwise the
 * newest available run (the first item in run history) is selected.
 */
export function resolveSelectedCompletedRun(
  runs: readonly AnalysisRun[],
  selectedRunId: string | null | undefined,
): AnalysisRun | undefined {
  const completedRuns = completedResultRuns(runs);
  return completedRuns.find((run) => run.id === selectedRunId) ?? completedRuns[0];
}

export function isCompletedResultRun(run: AnalysisRun | null | undefined): run is AnalysisRun & { result: NonNullable<AnalysisRun["result"]> } {
  return Boolean(run && run.status === "completed" && run.result);
}

export type NativePlsPosthocMinimumSampleSizeProjection = NonNullable<
  NonNullable<AnalysisRun["result"]>["posthoc_minimum_sample_size"]
>;

const PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V1_REQUIRED_KEYS = [
  "alpha",
  "analytical_sample_size",
  "caution",
  "driver_source",
  "driver_target",
  "inverse_square_root_constant",
  "meets_technical_requirement",
  "method_version",
  "minimum_absolute_path_coefficient",
  "power",
  "status",
  "technically_required_sample_size",
  "test",
] as const;

const PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V2_KEYS = [
  ...PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V1_REQUIRED_KEYS,
  "driver_p_value_two_sided",
  "eligible_path_count",
  "selection_rule",
  "significance_alpha",
  "significance_source",
  "significant_path_count",
] as const;

/** Recompute and fail closed on the stored post-hoc technical sample-size result. */
export function nativePlsPosthocMinimumSampleSizeProjection(
  run: AnalysisRun | null | undefined,
): NativePlsPosthocMinimumSampleSizeProjection | null {
  if (!isCompletedResultRun(run) || !run.result.posthoc_minimum_sample_size) return null;
  const stored = run.result.posthoc_minimum_sample_size;
  const keys = Object.keys(stored);
  const requiredKeysPresent = PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V1_REQUIRED_KEYS.every((key) => keys.includes(key));
  const allowedKeys = new Set(PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V2_KEYS);
  if (!requiredKeysPresent || keys.some((key) => !allowedKeys.has(key as typeof PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V2_KEYS[number]))
    || stored.alpha !== 0.05
    || stored.power !== 0.80
    || stored.test !== "directional"
    || stored.inverse_square_root_constant !== 2.486
    || !hasText(stored.caution)
    || !Number.isSafeInteger(stored.analytical_sample_size)
    || stored.analytical_sample_size !== run.result.used_observations
    || stored.analytical_sample_size < 3) return null;

  if (stored.method_version === "inverse_square_root_posthoc_v1") {
    return nativeLegacyPlsPosthocMinimumSampleSizeProjection(run, stored);
  }
  if (stored.method_version !== "inverse_square_root_posthoc_v2"
    || keys.sort().join("\u0000") !== [...PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_V2_KEYS].sort().join("\u0000")
    || stored.selection_rule !== "smallest_absolute_statistically_significant_structural_path"
    || stored.eligible_path_count !== run.result.paths.length) return null;

  const candidates = validPosthocPaths(run);
  if (!candidates) return null;
  if (candidates.length === 0) {
    return stored.status === "not_applicable_no_structural_path"
      && stored.significance_source === null
      && stored.significance_alpha === null
      && stored.significant_path_count === null
      && posthocDriverIsEmpty(stored)
      ? stored : null;
  }
  if (!run.bootstrap) {
    return stored.status === "inference_unavailable"
      && stored.significance_source === null
      && stored.significance_alpha === null
      && stored.significant_path_count === null
      && posthocDriverIsEmpty(stored)
      ? stored : null;
  }
  if (stored.significance_source !== "pls_bootstrap_normal_reference_two_sided"
    || stored.significance_alpha !== 0.05) return null;

  const pathProbabilities = new Map<string, number>();
  const linkedPathKeys = new Set<string>();
  let inferenceIncomplete = false;
  for (const parameter of run.bootstrap.percentile.parameters) {
    const identity = parseParameterIdentity(parameter.parameter);
    if (identity?.kind !== "path" || identity.parts.length !== 2) continue;
    const key = `${identity.parts[0]}\u0000${identity.parts[1]}`;
    const probability = parameter.p_value_two_sided;
    const linkedPath = candidates.find((path) => `${path.source}\u0000${path.target}` === key);
    if (!linkedPath
      || linkedPathKeys.has(key)
      || !isFiniteNumber(parameter.original)
      || !numbersClose(parameter.original, linkedPath.coefficient)) return null;
    linkedPathKeys.add(key);
    if (probability === null || probability === undefined) {
      inferenceIncomplete = true;
      continue;
    }
    if (!isFiniteNumber(probability) || probability < 0 || probability > 1) return null;
    pathProbabilities.set(key, probability);
  }
  if (linkedPathKeys.size !== candidates.length) return null;
  if (inferenceIncomplete) {
    return stored.status === "inference_incomplete"
      && stored.significant_path_count === null
      && posthocDriverIsEmpty(stored)
      ? stored : null;
  }
  const significant = candidates.filter((path) => (
    pathProbabilities.get(`${path.source}\u0000${path.target}`)! <= 0.05
  )).sort(comparePosthocPaths);
  if (significant.length === 0) {
    return stored.status === "no_statistically_significant_path"
      && stored.significant_path_count === 0
      && posthocDriverIsEmpty(stored)
      ? stored : null;
  }
  const driver = significant[0];
  const driverProbability = pathProbabilities.get(`${driver.source}\u0000${driver.target}`)!;
  return posthocDriverMatches(stored, driver, driverProbability, significant.length) ? stored : null;
}

function nativeLegacyPlsPosthocMinimumSampleSizeProjection(
  run: AnalysisRun,
  stored: NativePlsPosthocMinimumSampleSizeProjection,
): NativePlsPosthocMinimumSampleSizeProjection | null {
  if (!hasText(stored.driver_source) || !hasText(stored.driver_target)) return null;
  const candidates = validPosthocPaths(run);
  if (!candidates?.length) return null;
  const driver = [...candidates].sort(comparePosthocPaths)[0];
  const absolutePath = Math.abs(driver.coefficient);
  if (stored.driver_source !== driver.source
    || stored.driver_target !== driver.target
    || !isFiniteNumber(stored.minimum_absolute_path_coefficient)
    || !numbersClose(stored.minimum_absolute_path_coefficient, absolutePath)) return null;
  const roundedRequired = absolutePath === 0 ? Infinity : Math.ceil((2.486 / absolutePath) ** 2);
  const supportedInteger = Number.isSafeInteger(roundedRequired) && roundedRequired >= 1;
  const expectedRequired = supportedInteger ? roundedRequired : null;
  const expectedStatus = absolutePath === 0
    ? "undefined_zero_path"
    : supportedInteger ? "available" : "exceeds_supported_integer_range";
  const expectedMeets = expectedRequired === null ? null : stored.analytical_sample_size >= expectedRequired;
  return stored.status === expectedStatus
    && stored.technically_required_sample_size === expectedRequired
    && stored.meets_technical_requirement === expectedMeets ? stored : null;
}

function validPosthocPaths(run: AnalysisRun) {
  const candidates = run.result!.paths.filter((path) => (
    hasText(path.source) && hasText(path.target) && isFiniteNumber(path.coefficient)
  ));
  const identities = new Set(candidates.map((path) => `${path.source}\u0000${path.target}`));
  return candidates.length === run.result!.paths.length && identities.size === candidates.length
    ? candidates : null;
}

function comparePosthocPaths(
  left: { source: string; target: string; coefficient: number },
  right: { source: string; target: string; coefficient: number },
) {
  const byMagnitude = Math.abs(left.coefficient) - Math.abs(right.coefficient);
  if (byMagnitude !== 0) return byMagnitude;
  if (left.source !== right.source) return left.source < right.source ? -1 : 1;
  if (left.target !== right.target) return left.target < right.target ? -1 : 1;
  return 0;
}

function posthocDriverIsEmpty(stored: NativePlsPosthocMinimumSampleSizeProjection) {
  return stored.driver_source === null
    && stored.driver_target === null
    && stored.driver_p_value_two_sided === null
    && stored.minimum_absolute_path_coefficient === null
    && stored.technically_required_sample_size === null
    && stored.meets_technical_requirement === null;
}

function posthocDriverMatches(
  stored: NativePlsPosthocMinimumSampleSizeProjection,
  driver: { source: string; target: string; coefficient: number },
  driverProbability: number,
  significantPathCount: number,
) {
  const absolutePath = Math.abs(driver.coefficient);
  const roundedRequired = absolutePath === 0 ? Infinity : Math.ceil((2.486 / absolutePath) ** 2);
  const supportedInteger = Number.isSafeInteger(roundedRequired) && roundedRequired >= 1;
  const expectedRequired = supportedInteger ? roundedRequired : null;
  const expectedStatus = absolutePath === 0
    ? "undefined_zero_path"
    : supportedInteger ? "available" : "exceeds_supported_integer_range";
  const expectedMeets = expectedRequired === null ? null : stored.analytical_sample_size >= expectedRequired;
  return stored.status === expectedStatus
    && stored.driver_source === driver.source
    && stored.driver_target === driver.target
    && isFiniteNumber(stored.driver_p_value_two_sided)
    && numbersClose(stored.driver_p_value_two_sided, driverProbability)
    && isFiniteNumber(stored.minimum_absolute_path_coefficient)
    && numbersClose(stored.minimum_absolute_path_coefficient, absolutePath)
    && stored.significant_path_count === significantPathCount
    && stored.technically_required_sample_size === expectedRequired
    && stored.meets_technical_requirement === expectedMeets;
}

function posthocSampleSizeStatusLabel(
  status: NativePlsPosthocMinimumSampleSizeProjection["status"],
) {
  switch (status) {
    case "not_applicable_no_structural_path": return "Not applicable: the model has no structural path";
    case "inference_unavailable": return "Unavailable: run PLS bootstrapping to identify statistically significant paths";
    case "inference_incomplete": return "Unavailable: bootstrap inference is incomplete for one or more structural paths";
    case "no_statistically_significant_path": return "Unavailable: no structural path meets the 5% significance rule";
    case "undefined_zero_path": return "Unavailable: the driving path coefficient is zero";
    case "exceeds_supported_integer_range": return "Unavailable: the calculated requirement exceeds the supported integer range";
    case "available": return "Available";
  }
}

export interface NativePlsSampleSizePowerResultProjection {
  recipe: NativePlsSampleSizePowerRecipeV1 | NativePlsSampleSizePowerRecipeV2;
  result: NativePlsSampleSizePowerResultV1 | NativePlsSampleSizePowerResultV2;
  presentation: NativePlsSampleSizePowerPresentation;
}

export function nativePlsSampleSizePowerResultProjection(
  run: AnalysisRun | null | undefined,
): NativePlsSampleSizePowerResultProjection | null {
  if (!run
    || run.status !== "completed"
    || run.result
    || run.assessment
    || run.bootstrap
    || run.permutation
    || !run.plsSampleSizePower
    || !run.plsSampleSizePowerRecipe
    || run.provenance?.method !== "pls_sample_size_power"
    || run.provenance.method_version !== run.plsSampleSizePower.method_version
    || run.plsSampleSizePowerRecipe.method_version !== run.plsSampleSizePower.method_version
    || run.provenance.settings.method !== "pls_sample_size_power"
    || run.provenance.seed !== run.plsSampleSizePowerRecipe.master_seed
    || run.provenance.settings.workers !== run.plsSampleSizePowerRecipe.workers
    || run.provenance.settings.confidence_level !== run.plsSampleSizePowerRecipe.confidence_level) return null;
  try {
    validateNativePlsSampleSizePowerResult(run.plsSampleSizePowerRecipe, run.plsSampleSizePower);
    return {
      recipe: run.plsSampleSizePowerRecipe,
      result: run.plsSampleSizePower,
      presentation: nativePlsSampleSizePowerPresentation(run.plsSampleSizePowerRecipe, run.plsSampleSizePower),
    };
  } catch {
    return null;
  }
}

function nativePlsSampleSizePowerResultTables(
  projection: NativePlsSampleSizePowerResultProjection,
): ResultTable[] {
  const ids = {
    "Power by sample size": "pls_power_by_sample_size",
    "Bootstrap tail accounting": "pls_power_bootstrap_tail_accounting",
    "Simulation failures": "pls_power_simulation_failures",
    "Design assumptions": "pls_power_design_assumptions",
    "Run provenance": "pls_power_run_provenance",
  } as const;
  return nativePlsSampleSizePowerExportTables(projection.recipe, projection.result).map((table) => ({
    id: ids[table.name],
    title: table.name,
    status: projection.result.schema_version === 2 ? "validated" : "experimental",
    warning: null,
    columns: table.columns,
    rows: table.rows,
  }));
}

export function nativeCtaPlsResultProjection(
  run: AnalysisRun | null | undefined,
): NativeCtaPlsResultProjection | null {
  if (!isCompletedResultRun(run) || !run.modelSnapshot) return null;
  const result = run.result;
  const provenance = run.provenance;
  const cta = result.cta_pls;
  if (!cta
    || provenance?.method !== "cta_pls"
    || provenance.settings.method !== "cta_pls"
    || !["path", "factor"].includes(provenance.settings.weighting_scheme)
    || provenance.settings.missing_data !== "listwise_deletion"
    || provenance.settings.case_weight_column !== null
    || provenance.settings.bootstrap_samples !== 0
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || provenance.settings.workers !== 1
    || provenance.method_version.split("+").filter((version) => version === NATIVE_CTA_PLS_METHOD_VERSION).length !== 1
    || result.method_version !== NATIVE_CTA_PLS_METHOD_VERSION
    || cta.method_version !== NATIVE_CTA_PLS_METHOD_VERSION
    || cta.covariance !== NATIVE_CTA_PLS_COVARIANCE_VERSION
    || cta.warnings.length !== 1
    || cta.warnings[0] !== NATIVE_CTA_PLS_RESULT_WARNING
    || !result.warnings.includes(NATIVE_CTA_PLS_ESTIMATION_WARNING)
    || run.bootstrap
    || run.permutation) return null;

  if (run.modelSnapshot.nodes.some((node) => Boolean(node.data.semantic))
    || run.modelSnapshot.edges.some((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role === "control" || role === "covariance";
    })) return null;
  const blocks = nativeCtaPlsEligibleBlocks(run.modelSnapshot.nodes);
  if (!blocks.length) return null;

  const expected = new Set<string>();
  for (const block of blocks) {
    const indicators = block.indicators;
    for (let a = 0; a < indicators.length - 3; a += 1) {
      for (let b = a + 1; b < indicators.length - 2; b += 1) {
        for (let c = b + 1; c < indicators.length - 1; c += 1) {
          for (let d = c + 1; d < indicators.length; d += 1) {
            for (const pairing of NATIVE_CTA_PLS_PAIRINGS) {
              expected.add(ctaIdentity(block.constructId, indicators[a], indicators[b], indicators[c], indicators[d], pairing));
            }
          }
        }
      }
    }
  }
  if (cta.estimates.length !== expected.size) return null;

  const actual = new Set<string>();
  const valuesByQuadruple = new Map<string, Map<string, number>>();
  const maxima = new Map<string, number>();
  for (const row of cta.estimates) {
    if (!isFiniteNumber(row.tetrad)
      || !isFiniteNumber(row.absolute_tetrad)
      || row.absolute_tetrad < 0
      || !numbersClose(row.absolute_tetrad, Math.abs(row.tetrad))
      || !NATIVE_CTA_PLS_PAIRINGS.includes(row.pairing as typeof NATIVE_CTA_PLS_PAIRINGS[number])) return null;
    const identity = ctaIdentity(row.construct, row.indicator_a, row.indicator_b, row.indicator_c, row.indicator_d, row.pairing);
    if (!expected.has(identity) || actual.has(identity)) return null;
    actual.add(identity);
    const quadruple = [row.construct, row.indicator_a, row.indicator_b, row.indicator_c, row.indicator_d].join("\u0000");
    const values = valuesByQuadruple.get(quadruple) ?? new Map<string, number>();
    if (values.has(row.pairing)) return null;
    values.set(row.pairing, row.tetrad);
    valuesByQuadruple.set(quadruple, values);
    maxima.set(row.construct, Math.max(maxima.get(row.construct) ?? 0, row.absolute_tetrad));
  }
  if (actual.size !== expected.size
    || [...valuesByQuadruple.values()].some((values) => values.size !== 3
      || !numbersClose([...values.values()].reduce((sum, value) => sum + value, 0), 0))) return null;
  const summaryEntries = Object.entries(cta.max_absolute_tetrad_by_construct);
  if (summaryEntries.length !== blocks.length
    || blocks.some((block) => {
      const value = cta.max_absolute_tetrad_by_construct[block.constructId];
      return !isFiniteNumber(value) || !numbersClose(value, maxima.get(block.constructId) ?? Number.NaN);
    })) return null;

  const unsupportedArtifacts = [
    result.plsc, result.endogeneity, result.nonlinear_effects, result.moderated_mediation,
    result.wpls, result.cca, result.predict, result.segmentation, result.mga, result.micom,
    result.mga_permutation, result.fimix, result.ipma, result.cbsem, result.pca,
    result.regression, result.nca, result.gsca,
  ];
  if (unsupportedArtifacts.some(Boolean)) return null;
  return {
    methodVersion: NATIVE_CTA_PLS_METHOD_VERSION,
    covarianceVersion: NATIVE_CTA_PLS_COVARIANCE_VERSION,
    usedObservations: result.used_observations,
    omittedObservations: result.omitted_observations,
    blocks,
    estimates: cta.estimates,
    maxAbsoluteTetradByConstruct: cta.max_absolute_tetrad_by_construct,
    warnings: [...cta.warnings],
  };
}

function ctaIdentity(
  construct: string,
  a: string,
  b: string,
  c: string,
  d: string,
  pairing: string,
): string {
  return [construct, a, b, c, d, pairing].join("\u0000");
}

export function nativeModerationPlot(run: AnalysisRun | null | undefined): NativeModerationPlot | null {
  if (!isCompletedResultRun(run)) return null;
  const estimate = run.result.moderation?.estimates.find((candidate) =>
    hasText(candidate.predictor)
    && hasText(candidate.moderator)
    && hasText(candidate.outcome)
    && candidate.simple_slopes.filter((slope) => isFiniteNumber(slope.moderator_score) && isFiniteNumber(slope.effect)).length >= 2,
  );
  if (!estimate) return null;
  const constructLabel = constructDisplayLabelResolver(run);
  const predictorLabel = constructLabel(estimate.predictor);
  const moderatorLabel = constructLabel(estimate.moderator);
  const outcomeLabel = constructLabel(estimate.outcome);
  const slopes = estimate.simple_slopes
    .filter((slope) => isFiniteNumber(slope.moderator_score) && isFiniteNumber(slope.effect))
    .sort((left, right) => left.moderator_score - right.moderator_score)
    .map((slope) => ({
      moderatorScore: slope.moderator_score,
      effect: slope.effect,
      label: `${moderatorLabel} = ${formatNumber(slope.moderator_score)}`,
    }));
  return {
    title: `${predictorLabel} × ${moderatorLabel} → ${outcomeLabel}`,
    predictorLabel,
    moderatorLabel,
    outcomeLabel,
    slopes,
  };
}

export function nativeIpmaPlot(run: AnalysisRun | null | undefined): NativeIpmaPlot | null {
  if (!isCompletedResultRun(run) || !run.result.ipma || !run.modelSnapshot) return null;
  const ipma = run.result.ipma;
  if (ipma.method_version !== "ipma_v1" || ipma.performance_scale !== "min_max_0_100_from_standardized_scores_v1") return null;
  const targets = [...new Set(ipma.targets.filter(hasText))];
  if (targets.length !== 1) return null;
  const targetId = targets[0];
  if (!run.modelSnapshot.nodes.some((node) => node.id === targetId)) return null;

  const predecessors = nativeIpmaPredecessorIds(run.modelSnapshot.edges, targetId);
  const constructLabel = constructDisplayLabelResolver(run);
  const pointsByConstruct = new Map<string, NativeIpmaPlot["points"][number]>();
  for (const row of ipma.constructs) {
    if (row.target !== targetId || !predecessors.has(row.construct) || pointsByConstruct.has(row.construct)) continue;
    if (!isFiniteNumber(row.importance) || !isFiniteNumber(row.performance) || row.performance < 0 || row.performance > 100) continue;
    pointsByConstruct.set(row.construct, {
      constructId: row.construct,
      constructLabel: constructLabel(row.construct),
      importance: row.importance,
      performance: row.performance,
    });
  }
  const points = [...pointsByConstruct.values()];
  if (!points.length) return null;
  return {
    targetId,
    targetLabel: constructLabel(targetId),
    scopeNote: NATIVE_IPMA_SCOPE_NOTE,
    points,
  };
}

export function nativeNcaResultProjection(run: AnalysisRun | null | undefined): NativeNcaResultProjection | null {
  if (!isCompletedResultRun(run) || run.modelId || run.modelSnapshot) return null;
  const provenance = run.provenance;
  const result = run.result;
  const nca = result.nca;
  if (!nca) return null;
  if (provenance?.method !== "nca"
    || provenance.method_version !== "nca_v2"
    || provenance.settings.method !== "nca"
    || provenance.settings.preprocessing !== "unstandardized"
    || provenance.settings.bootstrap_samples !== 0
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || provenance.settings.case_weight_column !== null
    || result.method_version !== "nca_v2"
    || nca.method_version !== "nca_v2") return null;
  if (run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_STANDALONE_ASSESSMENT_WARNING) return null;
  if (!hasText(nca.x) || !hasText(nca.y) || nca.x === nca.y) return null;
  if (!isPositiveInteger(nca.observations) || nca.observations < 3 || result.used_observations !== nca.observations) return null;
  if (!isPositiveInteger(nca.permutation_samples)
    || nca.permutation_samples > 10_000
    || !isPositiveInteger(nca.usable_permutations)
    || nca.usable_permutations > nca.permutation_samples) return null;
  if (nca.ceiling !== "ce_fdh" && nca.ceiling !== "cr_fdh" && nca.ceiling !== "both") return null;
  const scope = nca.scope;
  if (!scope
    || ![scope.minimum_x, scope.maximum_x, scope.minimum_y, scope.maximum_y].every(isFiniteNumber)
    || scope.minimum_x >= scope.maximum_x
    || scope.minimum_y >= scope.maximum_y) return null;

  const peers = nca.ce_fdh_peers ?? [];
  if (!peers.length || peers.some((peer) => !isFiniteNumber(peer.x) || !isFiniteNumber(peer.y))) return null;
  if (!numbersClose(peers[0].x, scope.minimum_x) || !numbersClose(peers.at(-1)?.y, scope.maximum_y)) return null;
  if (peers.some((peer, index) => index > 0
    && (peer.x <= peers[index - 1].x || peer.y <= peers[index - 1].y))) return null;

  const expectedCeilings: Array<"ce_fdh" | "cr_fdh"> = nca.ceiling === "both"
    ? ["ce_fdh", "cr_fdh"]
    : [nca.ceiling];
  if (nca.ceilings.length !== expectedCeilings.length) return null;
  const ceilingEffects: NativeNcaResultProjection["ceilingEffects"] = [];
  for (const ceiling of expectedCeilings) {
    const rows = nca.ceilings.filter((row) => row.ceiling === ceiling);
    if (rows.length !== 1) return null;
    const row = rows[0];
    if (!isFiniteNumber(row.effect_size)
      || row.effect_size < 0
      || row.effect_size > 1
      || !isProbability(row.permutation_p_value)) return null;
    if (ceiling === "ce_fdh" && (row.slope != null || row.intercept != null)) return null;
    if (ceiling === "cr_fdh" && (!isFiniteNumber(row.slope) || !isFiniteNumber(row.intercept))) return null;
    ceilingEffects.push({
      ceiling,
      effectSize: row.effect_size,
      permutationPValue: row.permutation_p_value,
      slope: ceiling === "cr_fdh" ? row.slope as number : null,
      intercept: ceiling === "cr_fdh" ? row.intercept as number : null,
    });
  }

  const expectedOutcomes = [10, 20, 30, 40, 50, 60, 70, 80, 90];
  if (nca.bottlenecks.length !== expectedCeilings.length * expectedOutcomes.length) return null;
  const bottlenecks: NativeNcaResultProjection["bottlenecks"] = [];
  for (const ceiling of expectedCeilings) {
    for (const outcomePercent of expectedOutcomes) {
      const rows = nca.bottlenecks.filter((row) => row.ceiling === ceiling && row.outcome_percent === outcomePercent);
      if (rows.length !== 1) return null;
      const row = rows[0];
      if (row.status !== "required" && row.status !== "not_necessary" && row.status !== "not_attainable") return null;
      if (row.status === "required") {
        if (!isFiniteNumber(row.required_x_percent) || row.required_x_percent < 0 || row.required_x_percent > 100) return null;
      } else if (row.required_x_percent != null) return null;
      bottlenecks.push({
        ceiling,
        outcomePercent,
        requiredXPercent: row.status === "required" ? row.required_x_percent as number : null,
        status: row.status,
      });
    }
  }

  const warnings = nca.warnings.map((warning) => warning.trim()).filter(Boolean);
  if (!warnings.includes(NATIVE_NCA_ENGINE_SCOPE_WARNING)) return null;
  const ceFdhPeers: NativeNcaCeFdhPeerRow[] = peers.map((peer, index) => ({
    peerIdentity: `CE-FDH peer ${index + 1}`,
    conditionVariable: nca.x,
    conditionValue: peer.x,
    outcomeVariable: nca.y,
    outcomeValue: peer.y,
  }));
  const crFdh = ceilingEffects.find((row) => row.ceiling === "cr_fdh");
  const plot: NativeNcaPlot = {
    xLabel: nca.x,
    yLabel: nca.y,
    ceiling: nca.ceiling,
    scope: {
      minimumX: scope.minimum_x,
      maximumX: scope.maximum_x,
      minimumY: scope.minimum_y,
      maximumY: scope.maximum_y,
    },
    ceFdhPeers: ceFdhPeers.map((peer) => ({ x: peer.conditionValue, y: peer.outcomeValue })),
    crFdh: crFdh ? { slope: crFdh.slope as number, intercept: crFdh.intercept as number } : null,
  };
  return {
    methodVersion: "nca_v2",
    x: nca.x,
    y: nca.y,
    observations: nca.observations,
    ceiling: nca.ceiling,
    permutationSamples: nca.permutation_samples,
    usablePermutations: nca.usable_permutations,
    scope: plot.scope,
    ceFdhPeers,
    ceilingEffects,
    bottlenecks,
    warnings,
    plot,
  };
}

export function nativeNcaPlot(run: AnalysisRun | null | undefined): NativeNcaPlot | null {
  return nativeNcaResultProjection(run)?.plot ?? null;
}

export function nativeNcaCeilingLabel(ceiling: "ce_fdh" | "cr_fdh" | "both"): string {
  if (ceiling === "ce_fdh") return "CE-FDH";
  if (ceiling === "cr_fdh") return "CR-FDH";
  return "CE-FDH and CR-FDH";
}

export const NATIVE_NCA_CE_FDH_PEER_SOURCE_NOTE =
  "Peer identities follow the immutable stored CE-FDH frontier order; the NCA v2 payload does not retain original source-row identifiers.";

function ncaCeFdhPeerTableFromProjection(
  projection: NativeNcaResultProjection,
): ResultTable | null {
  if (projection.ceiling === "cr_fdh") return null;
  return {
    id: "nca_ce_fdh_peers",
    title: "CE-FDH frontier peers",
    status: "validated",
    warning: NATIVE_NCA_CE_FDH_PEER_SOURCE_NOTE,
    columns: [
      "Peer identity",
      "Condition variable (X)",
      "Condition value",
      "Outcome variable (Y)",
      "Outcome value",
    ],
    rows: projection.ceFdhPeers.map((peer) => [
      peer.peerIdentity,
      peer.conditionVariable,
      formatNumber(peer.conditionValue),
      peer.outcomeVariable,
      formatNumber(peer.outcomeValue),
    ]),
  };
}

export function nativeNcaCeFdhPeerTable(
  run: AnalysisRun | null | undefined,
): ResultTable | null {
  const projection = nativeNcaResultProjection(run);
  return projection ? ncaCeFdhPeerTableFromProjection(projection) : null;
}

export function nativeGscaResultProjection(run: AnalysisRun | null | undefined): NativeGscaResultProjection | null {
  if (!isCompletedResultRun(run) || !run.modelSnapshot) return null;
  const provenance = run.provenance;
  const result = run.result;
  const analysis = result.gsca;
  if (!analysis
    || provenance?.method !== "gsca"
    || provenance.method_version !== NATIVE_GSCA_METHOD_VERSION
    || provenance.settings.method !== "gsca"
    || provenance.settings.weighting_scheme !== "path"
    || provenance.settings.preprocessing !== "standardized"
    || provenance.settings.missing_data !== "listwise_deletion"
    || provenance.settings.max_iterations !== 3_000
    || provenance.settings.tolerance !== 1e-7
    || provenance.settings.workers !== 1
    || provenance.settings.bootstrap_samples !== 0
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || provenance.settings.case_weight_column !== null
    || result.method_version !== NATIVE_GSCA_METHOD_VERSION
    || analysis.method_version !== NATIVE_GSCA_METHOD_VERSION
    || analysis.algorithm !== NATIVE_GSCA_ALGORITHM_VERSION) return null;
  if (run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_GSCA_ASSESSMENT_WARNING
    || run.bootstrap
    || run.permutation) return null;
  if (!analysis.converged
    || !result.converged
    || !isPositiveInteger(analysis.iterations)
    || analysis.iterations > 3_000
    || result.iterations !== analysis.iterations
    || !isPositiveInteger(analysis.observations)
    || analysis.observations !== result.used_observations
    || !isNonNegativeInteger(result.omitted_observations)
    || !isFiniteNumber(analysis.stop_criterion)
    || analysis.stop_criterion !== 1e-7
    || !isFiniteNumber(analysis.final_change)
    || analysis.final_change < 0
    || analysis.final_change > analysis.stop_criterion) return null;

  const numericFitValues = [
    analysis.objective,
    analysis.fit,
    analysis.measurement_fit,
    analysis.structural_fit,
    analysis.adjusted_fit,
    analysis.gfi,
    analysis.srmr,
    analysis.covariance_discrepancy,
    analysis.covariance_sample_total,
    analysis.standardized_residual_sum,
  ];
  if (!numericFitValues.every(isFiniteNumber)
    || (analysis.objective as number) < 0
    || (analysis.gfi as number) > 1
    || (analysis.srmr as number) < 0
    || (analysis.covariance_discrepancy as number) < 0
    || (analysis.covariance_sample_total as number) <= 0
    || (analysis.standardized_residual_sum as number) < 0
    || !isPositiveInteger(analysis.free_parameters)) return null;

  const nodes = run.modelSnapshot.nodes;
  if (nodes.length < 2
    || nodes.some((node) => !hasText(node.id)
      || Boolean(node.data.semantic)
      || (node.data.mode !== "reflective" && node.data.mode !== "formative")
      || !node.data.indicators.length)) return null;
  const constructIds = new Set(nodes.map((node) => node.id));
  if (constructIds.size !== nodes.length) return null;
  const expectedOuterKeys = nodes.flatMap((node) => node.data.indicators.map((indicator) => `${node.id}\u0000${indicator}`));
  if (new Set(expectedOuterKeys).size !== expectedOuterKeys.length) return null;
  if (run.modelSnapshot.edges.some((edge) => {
    const role = (edge.data as { role?: string } | undefined)?.role;
    return role === "control" || role === "covariance";
  })) return null;
  const structuralEdges = run.modelSnapshot.edges.filter((edge) => {
    const role = (edge.data as { role?: string } | undefined)?.role;
    return !edge.id.startsWith("measurement::") && role !== "control" && role !== "covariance";
  });
  const expectedPathKeys = structuralEdges.map((edge) => `${edge.source}\u0000${edge.target}`);
  if (!expectedPathKeys.length
    || new Set(expectedPathKeys).size !== expectedPathKeys.length
    || structuralEdges.some((edge) => edge.source === edge.target
      || !constructIds.has(edge.source)
      || !constructIds.has(edge.target))) return null;
  const connected = new Set(structuralEdges.flatMap((edge) => [edge.source, edge.target]));
  if (nodes.some((node) => !connected.has(node.id))) return null;

  const outerMap = (rows: GscaAnalysis["weights"]) => {
    if (rows.length !== expectedOuterKeys.length) return null;
    const mapped = new Map<string, GscaAnalysis["weights"][number]>();
    for (const row of rows) {
      const key = `${row.construct}\u0000${row.indicator}`;
      if (!expectedOuterKeys.includes(key)
        || mapped.has(key)
        || !isFiniteNumber(row.weight)
        || !isFiniteNumber(row.loading)
        || Math.abs(row.loading) > 1 + 1e-10) return null;
      mapped.set(key, row);
    }
    return mapped.size === expectedOuterKeys.length ? mapped : null;
  };
  const weights = outerMap(analysis.weights);
  const loadings = outerMap(analysis.loadings);
  const resultOuter = outerMap(result.outer_estimates);
  if (!weights || !loadings || !resultOuter) return null;
  for (const key of expectedOuterKeys) {
    const weight = weights.get(key)!;
    const loading = loadings.get(key)!;
    const outer = resultOuter.get(key)!;
    if (!numbersClose(weight.weight, loading.weight)
      || !numbersClose(weight.loading, loading.loading)
      || !numbersClose(weight.weight, outer.weight)
      || !numbersClose(weight.loading, outer.loading)) return null;
  }

  const pathMap = (rows: Array<{ source: string; target: string; coefficient: number }>) => {
    if (rows.length !== expectedPathKeys.length) return null;
    const mapped = new Map<string, number>();
    for (const row of rows) {
      const key = `${row.source}\u0000${row.target}`;
      if (!expectedPathKeys.includes(key) || mapped.has(key) || !isFiniteNumber(row.coefficient)) return null;
      mapped.set(key, row.coefficient);
    }
    return mapped.size === expectedPathKeys.length ? mapped : null;
  };
  const paths = pathMap(analysis.paths);
  const resultPaths = pathMap(result.paths);
  if (!paths || !resultPaths || expectedPathKeys.some((key) => !numbersClose(paths.get(key), resultPaths.get(key)))) return null;

  const endogenous = new Set(structuralEdges.map((edge) => edge.target));
  const rSquaredKeys = Object.keys(analysis.r_squared);
  if (rSquaredKeys.length !== endogenous.size
    || rSquaredKeys.some((key) => !endogenous.has(key)
      || !isFiniteNumber(analysis.r_squared[key])
      || analysis.r_squared[key] > 1 + 1e-10)
    || Object.keys(result.r_squared).length !== endogenous.size
    || [...endogenous].some((key) => !numbersClose(analysis.r_squared[key], result.r_squared[key]))) return null;

  const indicatorCount = expectedOuterKeys.length;
  const constructCount = nodes.length;
  const freeParameters = nodes.reduce((total, node) => total + node.data.indicators.length - 1, 0)
    + nodes.filter((node) => node.data.mode === "reflective").reduce((total, node) => total + node.data.indicators.length, 0)
    + structuralEdges.length;
  const measurementResidual = nodes.reduce((total, node) => total + node.data.indicators.reduce((blockTotal, indicator) => {
    const loading = loadings.get(`${node.id}\u0000${indicator}`)!.loading;
    return blockTotal + (node.data.mode === "reflective" ? 1 - loading * loading : 1);
  }, 0), 0);
  const structuralResidual = nodes.reduce((total, node) => total + (Object.hasOwn(analysis.r_squared, node.id)
    ? 1 - analysis.r_squared[node.id]
    : 1), 0);
  const weightSums = nodes.map((node) => node.data.indicators.reduce(
    (total, indicator) => total + weights.get(`${node.id}\u0000${indicator}`)!.weight,
    0,
  ));
  const totalVariation = indicatorCount + constructCount;
  const nullDegrees = analysis.observations * indicatorCount;
  const expectedAdjustedFit = 1 - (1 - analysis.fit) * nullDegrees / (nullDegrees - freeParameters);
  const expectedSrmr = Math.sqrt(2 * (analysis.standardized_residual_sum as number) / (indicatorCount * (indicatorCount + 1)));
  if (analysis.free_parameters !== freeParameters
    || nullDegrees <= freeParameters
    || weightSums.some((sum) => sum < -1e-12)
    || !numbersClose(analysis.objective, measurementResidual + structuralResidual)
    || !numbersClose(analysis.fit, 1 - (analysis.objective as number) / totalVariation)
    || !numbersClose(analysis.measurement_fit, 1 - measurementResidual / indicatorCount)
    || !numbersClose(analysis.structural_fit, 1 - structuralResidual / constructCount)
    || !numbersClose(analysis.adjusted_fit, expectedAdjustedFit)
    || !numbersClose(analysis.gfi, 1 - (analysis.covariance_discrepancy as number) / (analysis.covariance_sample_total as number))
    || !numbersClose(analysis.srmr, expectedSrmr)) return null;

  const unsupportedArtifacts = [
    result.plsc,
    result.endogeneity,
    result.nonlinear_effects,
    result.moderated_mediation,
    result.cta_pls,
    result.wpls,
    result.cca,
    result.predict,
    result.segmentation,
    result.mga,
    result.micom,
    result.mga_permutation,
    result.fimix,
    result.ipma,
    result.cbsem,
    result.pca,
    result.regression,
    result.nca,
  ];
  if (unsupportedArtifacts.some(Boolean)
    || result.effects.length
    || (result.control_estimates?.length ?? 0) > 0
    || (result.mediation?.estimates.length ?? 0) > 0
    || (result.moderation?.estimates.length ?? 0) > 0
    || analysis.bootstrap_intervals.length
    || analysis.warnings.length !== 1
    || analysis.warnings[0] !== NATIVE_GSCA_ENGINE_SCOPE_WARNING
    || result.warnings.length !== 1
    || result.warnings[0] !== NATIVE_GSCA_ENGINE_SCOPE_WARNING) return null;

  return {
    methodVersion: NATIVE_GSCA_METHOD_VERSION,
    algorithmVersion: NATIVE_GSCA_ALGORITHM_VERSION,
    analysis: analysis as NativeGscaResultProjection["analysis"],
    usedObservations: result.used_observations,
    omittedObservations: result.omitted_observations,
    constructModes: Object.fromEntries(nodes.map((node) => [node.id, node.data.mode])) as Record<string, "reflective" | "formative">,
  };
}

export function nativePcaResultProjection(run: AnalysisRun | null | undefined): NativePcaResultProjection | null {
  if (!isCompletedResultRun(run) || run.modelId || run.modelSnapshot) return null;
  const provenance = run.provenance;
  const result = run.result;
  const pca = result.pca;
  if (!pca
    || provenance?.method !== "pca"
    || provenance.method_version !== "pca_v1"
    || provenance.settings.method !== "pca"
    || provenance.settings.weighting_scheme !== "path"
    || provenance.settings.preprocessing !== "standardized"
    || provenance.settings.bootstrap_samples !== 0
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || provenance.settings.case_weight_column !== null
    || result.method_version !== "pca_v1"
    || pca.method_version !== "pca_v1") return null;
  if (run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_STANDALONE_ASSESSMENT_WARNING) return null;
  if (pca.component_rule !== "kaiser"
    && pca.component_rule !== "fixed"
    && pca.component_rule !== "variance_threshold") return null;
  const variables = pca.variables.filter(hasText);
  if (variables.length < 2
    || variables.length > 50
    || new Set(variables).size !== variables.length
    || !isPositiveInteger(pca.observations)
    || pca.observations < 3
    || result.used_observations !== pca.observations
    || !isPositiveInteger(pca.retained_components)
    || pca.retained_components !== pca.components.length
    || pca.retained_components > Math.min(variables.length, pca.observations - 1)
    || pca.loadings.length !== variables.length * pca.retained_components
    || pca.scores.length !== pca.observations * pca.retained_components) return null;

  let cumulative = 0;
  for (const [index, component] of pca.components.entries()) {
    cumulative += component.explained_variance;
    if (component.component !== `PC${index + 1}`
      || !isFiniteNumber(component.eigenvalue)
      || component.eigenvalue <= 0
      || !isFiniteNumber(component.explained_variance)
      || component.explained_variance <= 0
      || !isFiniteNumber(component.cumulative_variance)
      || !numbersClose(component.explained_variance, component.eigenvalue / variables.length)
      || !numbersClose(component.cumulative_variance, cumulative)) return null;
  }
  for (const [componentIndex, component] of pca.components.entries()) {
    for (const [variableIndex, variable] of variables.entries()) {
      const row = pca.loadings[componentIndex * variables.length + variableIndex];
      if (!row
        || row.variable !== variable
        || row.component !== component.component
        || !isFiniteNumber(row.loading)
        || !isFiniteNumber(row.weight)
        || !numbersClose(row.loading, row.weight * Math.sqrt(component.eigenvalue))) return null;
    }
    for (let observation = 0; observation < pca.observations; observation += 1) {
      const row = pca.scores[componentIndex * pca.observations + observation];
      if (!row
        || row.observation !== observation
        || row.component !== component.component
        || !isFiniteNumber(row.score)) return null;
    }
  }
  const warnings = pca.warnings.map((warning) => warning.trim()).filter(Boolean);
  if (!warnings.includes(NATIVE_PCA_ENGINE_SCOPE_WARNING)) return null;
  return {
    methodVersion: "pca_v1",
    componentRule: pca.component_rule,
    retainedComponents: pca.retained_components,
    observations: pca.observations,
    variables,
    components: pca.components,
    loadings: pca.loadings,
    scoresStored: pca.scores.length,
    warnings,
  };
}

function validateRegressionBootstrap(
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
  regression: RegressionAnalysis,
  expectedTerms: readonly string[],
  logistic: boolean,
): RegressionBootstrapAnalysis | null {
  const scopeWarning = "Regression bootstrap v1 uses deterministic indexed case resampling with replacement; percentile intervals are primary and BCa intervals are conditional on stable delete-one fits.";
  const testWarning = "Bootstrap ratio statistics use an independently implemented two-sided standard-normal reference for both OLS and logistic coefficients; they are distinct from point-estimate t or Wald inference.";
  const bootstrap = regression.bootstrap;
  const settings = run.provenance?.settings;
  if (!bootstrap || !settings
    || bootstrap.method_version !== "regression_bootstrap_v1"
    || bootstrap.algorithm !== "indexed_case_resampling_v1"
    || bootstrap.alternative !== "two_sided"
    || bootstrap.interval_policy !== "percentile_primary_bca_conditional_v1"
    || bootstrap.test_reference !== "standard_normal_bootstrap_ratio_v1"
    || bootstrap.test_tolerance_policy !== "64eps_max_1_original_replicates_v1"
    || bootstrap.stream_token !== "quickpls_indexed_resampling_v1"
    || bootstrap.confidence_level !== 0.95
    || bootstrap.minimum_usable_fraction !== 0.9
    || bootstrap.requested_replicates !== settings.bootstrap_samples
    || bootstrap.seed !== settings.seed
    || bootstrap.seed !== run.provenance?.seed
    || bootstrap.seed !== run.seed
    || bootstrap.workers !== settings.workers
    || !isPositiveInteger(bootstrap.requested_replicates)
    || bootstrap.requested_replicates < 99
    || bootstrap.requested_replicates > 10_000
    || !isPositiveInteger(bootstrap.usable_replicates)
    || bootstrap.usable_replicates < Math.ceil(bootstrap.minimum_usable_fraction * bootstrap.requested_replicates)
    || bootstrap.usable_replicates > bootstrap.requested_replicates
    || !Array.isArray(bootstrap.failed_replicates)
    || bootstrap.failed_replicates.length !== bootstrap.requested_replicates - bootstrap.usable_replicates
    || !isPositiveInteger(bootstrap.jackknife_cases)
    || bootstrap.jackknife_cases < 3
    || bootstrap.jackknife_cases !== regression.observations
    || !isNonNegativeInteger(bootstrap.usable_jackknife_cases)
    || bootstrap.usable_jackknife_cases > bootstrap.jackknife_cases
    || !Array.isArray(bootstrap.coefficients)
    || bootstrap.coefficients.length !== expectedTerms.length
    || !Array.isArray(bootstrap.warnings)
    || !isNativeRegressionBootstrapValidationWitness(
      bootstrap.validation_witness,
      expectedTerms,
      bootstrap,
      logistic,
    )) return null;
  const expectedWarnings = [scopeWarning, testWarning];
  if (bootstrap.failed_replicates.length) {
    expectedWarnings.push(`${bootstrap.failed_replicates.length} of ${bootstrap.requested_replicates} bootstrap replicates failed and were excluded from inference.`);
  }
  const failedJackknife = bootstrap.jackknife_cases - bootstrap.usable_jackknife_cases;
  if (failedJackknife) {
    expectedWarnings.push(`${failedJackknife} of ${bootstrap.jackknife_cases} delete-one fits failed; affected BCa intervals are explicitly unavailable.`);
  }
  if (bootstrap.warnings.length !== expectedWarnings.length
    || bootstrap.warnings.some((warning, index) => warning !== expectedWarnings[index])) return null;

  const failedIndexes = new Set<number>();
  for (const failure of bootstrap.failed_replicates) {
    if (!isNonNegativeInteger(failure.replicate_index)
      || failure.replicate_index >= bootstrap.requested_replicates
      || failedIndexes.has(failure.replicate_index)
      || !hasText(failure.reason_code)
      || !hasText(failure.message)) return null;
    failedIndexes.add(failure.replicate_index);
  }

  const validateBca = (value: RegressionBootstrapAnalysis["coefficients"][number]["bca"]) => {
    if (value.status === "available") {
      return isFiniteNumber(value.bias_correction)
        && isFiniteNumber(value.acceleration)
        && isFiniteNumber(value.lower)
        && isFiniteNumber(value.upper)
        && value.lower <= value.upper;
    }
    return (value.reason_code === "insufficient_jackknife_estimates"
      || value.reason_code === "incomplete_jackknife"
      || value.reason_code === "degenerate_jackknife_acceleration")
      && hasText(value.message);
  };

  for (const [index, row] of bootstrap.coefficients.entries()) {
    const point = regression.coefficients[index];
    if (row.term !== expectedTerms[index]
      || row.term !== point?.term
      || !isFiniteNumber(row.original)
      || !numbersClose(row.original, point.estimate)
      || !isFiniteNumber(row.bootstrap_mean)
      || !isFiniteNumber(row.bias)
      || !numbersClose(row.bias, row.bootstrap_mean - row.original)
      || !isFiniteNumber(row.standard_error)
      || row.standard_error < 0
      || !isFiniteNumber(row.replicate_max_abs)
      || row.replicate_max_abs < 0
      || !isFiniteNumber(row.test_tolerance)
      || row.test_tolerance <= 0
      || !jsonRoundTripNumbersClose(
        row.test_tolerance,
        64 * Number.EPSILON * Math.max(1, Math.abs(row.original), row.replicate_max_abs),
      )
      || Math.abs(row.bootstrap_mean) > row.replicate_max_abs + row.test_tolerance
      || !isFiniteNumber(row.percentile_lower)
      || !isFiniteNumber(row.percentile_upper)
      || row.percentile_lower > row.percentile_upper
      || row.percentile_lower < -row.replicate_max_abs - row.test_tolerance
      || row.percentile_upper > row.replicate_max_abs + row.test_tolerance
      || row.usable_replicates !== bootstrap.usable_replicates
      || !validateBca(row.bca)) return null;
    if (row.bca.status === "available"
      && (row.bca.lower < -row.replicate_max_abs - row.test_tolerance
        || row.bca.upper > row.replicate_max_abs + row.test_tolerance)) return null;
    if (bootstrap.usable_jackknife_cases < bootstrap.jackknife_cases
      && (row.bca.status !== "unavailable" || row.bca.reason_code !== "incomplete_jackknife")) return null;

    if (row.standard_error > row.test_tolerance) {
      if (row.test.status !== "available"
        || !isFiniteNumber(row.test.statistic)
        || !numbersClose(row.test.statistic, row.original / row.standard_error)
        || !isProbability(row.test.p_value_two_sided)
        || !scientificNumbersClose(
          row.test.p_value_two_sided,
          chiSquareSurvival(row.test.statistic * row.test.statistic, 1),
        )) return null;
    } else if (row.test.status !== "unavailable"
      || row.test.reason_code !== "degenerate_bootstrap_standard_error"
      || !hasText(row.test.message)) return null;
    if (row.standard_error <= row.test_tolerance
      && (Math.abs(row.percentile_upper - row.percentile_lower) > row.test_tolerance
        || Math.abs(row.percentile_lower - row.bootstrap_mean) > row.test_tolerance
        || Math.abs(row.percentile_upper - row.bootstrap_mean) > row.test_tolerance
        || Math.abs(row.bootstrap_mean - row.original) > row.test_tolerance)) return null;

    const oddsRatio = row.odds_ratio;
    if (!logistic) {
      if (oddsRatio != null) return null;
      continue;
    }
    const oddsRatioMinimum = Math.exp(-row.replicate_max_abs);
    const oddsRatioMaximum = Math.exp(row.replicate_max_abs);
    if (!oddsRatio
      || !isFiniteNumber(oddsRatioMinimum)
      || !isFiniteNumber(oddsRatioMaximum)
      || !isFiniteNumber(oddsRatio.original)
      || oddsRatio.original <= 0
      || !numbersClose(oddsRatio.original, Math.exp(row.original))
      || !isFiniteNumber(oddsRatio.percentile_lower)
      || !isFiniteNumber(oddsRatio.percentile_upper)
      || oddsRatio.percentile_lower <= 0
      || oddsRatio.percentile_lower > oddsRatio.percentile_upper
      || (oddsRatio.percentile_lower < oddsRatioMinimum && !numbersClose(oddsRatio.percentile_lower, oddsRatioMinimum))
      || (oddsRatio.percentile_upper > oddsRatioMaximum && !numbersClose(oddsRatio.percentile_upper, oddsRatioMaximum))
      || !validateBca(oddsRatio.bca)) return null;
    if (oddsRatio.bca.status === "available"
      && (oddsRatio.bca.lower <= 0
        || oddsRatio.bca.upper <= 0
        || (oddsRatio.bca.lower < oddsRatioMinimum && !numbersClose(oddsRatio.bca.lower, oddsRatioMinimum))
        || (oddsRatio.bca.upper > oddsRatioMaximum && !numbersClose(oddsRatio.bca.upper, oddsRatioMaximum)))) return null;
    if (bootstrap.usable_jackknife_cases < bootstrap.jackknife_cases
      && (oddsRatio.bca.status !== "unavailable" || oddsRatio.bca.reason_code !== "incomplete_jackknife")) return null;
  }
  return bootstrap;
}

export function nativeOlsResultProjection(run: AnalysisRun | null | undefined): NativeOlsResultProjection | null {
  if (!isCompletedResultRun(run) || run.modelId || run.modelSnapshot) return null;
  const provenance = run.provenance;
  const result = run.result;
  const regression = result.regression;
  const bootstrapRun = provenance?.method_version === "regression_ols_v1+regression_bootstrap_v1";
  if (!regression
    || provenance?.method !== "regression"
    || (!bootstrapRun && provenance.method_version !== "regression_ols_v1")
    || provenance.settings.method !== "regression"
    || provenance.settings.weighting_scheme !== "path"
    || provenance.settings.preprocessing !== "unstandardized"
    || (bootstrapRun
      ? provenance.settings.bootstrap_samples < 99 || provenance.settings.bootstrap_samples > 10_000
      : provenance.settings.bootstrap_samples !== 0)
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || provenance.settings.case_weight_column !== null
    || !numbersClose(provenance.settings.confidence_level, 0.95)
    || (bootstrapRun ? provenance.settings.workers < 1 || provenance.settings.workers > 64 : provenance.settings.workers !== 1)
    || result.method_version !== "regression_ols_v1"
    || regression.method_version !== "regression_ols_v1"
    || regression.regression_type !== "ols") return null;
  if (run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_STANDALONE_ASSESSMENT_WARNING) return null;
  const outcome = regression.outcome.trim();
  const predictors = regression.predictors.filter(hasText);
  const controls = regression.controls.filter(hasText);
  const variables = [outcome, ...predictors, ...controls];
  const expectedTerms = ["intercept", ...predictors, ...controls];
  if (!outcome
    || predictors.length < 1
    || new Set(variables).size !== variables.length
    || !isPositiveInteger(regression.observations)
    || regression.observations <= expectedTerms.length
    || result.used_observations !== regression.observations
    || regression.coefficients.length !== expectedTerms.length
    || regression.predictions.length !== regression.observations
    || regression.logistic
    || regression.process
    || run.bootstrap
    || run.permutation) return null;
  for (const [index, coefficient] of regression.coefficients.entries()) {
    if (coefficient.term !== expectedTerms[index]
      || !isFiniteNumber(coefficient.estimate)
      || !isFiniteNumber(coefficient.standard_error)
      || coefficient.standard_error <= 0
      || !isFiniteNumber(coefficient.statistic)
      || !isFiniteNumber(coefficient.p_value_two_sided)
      || coefficient.p_value_two_sided < 0
      || coefficient.p_value_two_sided > 1
      || !isFiniteNumber(coefficient.confidence_interval_lower)
      || !isFiniteNumber(coefficient.confidence_interval_upper)
      || coefficient.confidence_interval_lower > coefficient.estimate
      || coefficient.confidence_interval_upper < coefficient.estimate
      || coefficient.odds_ratio != null
      || coefficient.odds_ratio_confidence_interval_lower != null
      || coefficient.odds_ratio_confidence_interval_upper != null) return null;
  }
  const fit = regression.fit;
  if (!fit
    || !isFiniteNumber(fit.r_squared)
    || !isFiniteNumber(fit.adjusted_r_squared)
    || !isFiniteNumber(fit.f_statistic)
    || !isFiniteNumber(fit.aic)
    || !isFiniteNumber(fit.bic)
    || !isFiniteNumber(fit.rmse)
    || fit.log_likelihood != null
    || fit.pseudo_r_squared != null
    || fit.null_log_likelihood != null
    || fit.deviance != null
    || fit.null_deviance != null
    || fit.likelihood_ratio_chi_square != null
    || fit.likelihood_ratio_degrees_of_freedom != null
    || fit.likelihood_ratio_p_value != null
    || fit.pseudo_r_squared_method != null) return null;
  for (const [index, prediction] of regression.predictions.entries()) {
    if (prediction.observation !== index
      || !isFiniteNumber(prediction.fitted)
      || !isFiniteNumber(prediction.residual)
      || prediction.probability != null) return null;
  }
  const warnings = regression.warnings.map((warning) => warning.trim()).filter(Boolean);
  if (!warnings.includes(NATIVE_OLS_ENGINE_SCOPE_WARNING)) return null;
  const bootstrap = bootstrapRun
    ? validateRegressionBootstrap(run, regression, expectedTerms, false)
    : null;
  if ((bootstrapRun && !bootstrap) || (!bootstrapRun && regression.bootstrap)) return null;
  return {
    methodVersion: "regression_ols_v1",
    outcome,
    predictors,
    controls,
    observations: regression.observations,
    coefficients: regression.coefficients,
    fit,
    predictionsStored: regression.predictions.length,
    bootstrap,
    warnings,
  };
}

export function nativeLogisticResultProjection(
  run: AnalysisRun | null | undefined,
): NativeLogisticResultProjection | null {
  if (!isCompletedResultRun(run) || run.modelId || run.modelSnapshot) return null;
  const provenance = run.provenance;
  const result = run.result;
  const regression = result.regression;
  const bootstrapRun = provenance?.method_version === "regression_logistic_v2+regression_bootstrap_v1";
  if (!regression
    || provenance?.method !== "regression"
    || (!bootstrapRun && provenance.method_version !== "regression_logistic_v2")
    || provenance.settings.method !== "regression"
    || provenance.settings.weighting_scheme !== "path"
    || provenance.settings.preprocessing !== "unstandardized"
    || (bootstrapRun
      ? provenance.settings.bootstrap_samples < 99 || provenance.settings.bootstrap_samples > 10_000
      : provenance.settings.bootstrap_samples !== 0)
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || (bootstrapRun ? provenance.settings.workers < 1 || provenance.settings.workers > 64 : provenance.settings.workers !== 1)
    || provenance.settings.case_weight_column !== null
    || !numbersClose(provenance.settings.confidence_level, 0.95)
    || result.method_version !== "regression_logistic_v2"
    || regression.method_version !== "regression_logistic_v2"
    || regression.regression_type !== "logistic"
    || regression.process
    || !regression.logistic
    || run.bootstrap
    || run.permutation) return null;
  if (run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_STANDALONE_ASSESSMENT_WARNING) return null;

  const outcome = regression.outcome.trim();
  const predictors = regression.predictors.filter(hasText);
  const controls = regression.controls.filter(hasText);
  const variables = [outcome, ...predictors, ...controls];
  const expectedTerms = ["intercept", ...predictors, ...controls];
  if (!outcome
    || predictors.length < 1
    || new Set(variables).size !== variables.length
    || !isPositiveInteger(regression.observations)
    || regression.observations <= expectedTerms.length
    || result.used_observations !== regression.observations
    || regression.coefficients.length !== expectedTerms.length
    || regression.predictions.length !== regression.observations) return null;

  for (const [index, coefficient] of regression.coefficients.entries()) {
    const expectedStatistic = coefficient.estimate / coefficient.standard_error;
    const expectedPValue = chiSquareSurvival(expectedStatistic * expectedStatistic, 1);
    const expectedLower = coefficient.estimate - NORMAL_95_PERCENT_CRITICAL_VALUE * coefficient.standard_error;
    const expectedUpper = coefficient.estimate + NORMAL_95_PERCENT_CRITICAL_VALUE * coefficient.standard_error;
    if (coefficient.term !== expectedTerms[index]
      || !isFiniteNumber(coefficient.estimate)
      || !isFiniteNumber(coefficient.standard_error)
      || coefficient.standard_error <= 0
      || !isFiniteNumber(coefficient.statistic)
      || !numbersClose(coefficient.statistic, expectedStatistic)
      || !isProbability(coefficient.p_value_two_sided)
      || !scientificNumbersClose(coefficient.p_value_two_sided, expectedPValue)
      || !isFiniteNumber(coefficient.confidence_interval_lower)
      || !isFiniteNumber(coefficient.confidence_interval_upper)
      || !scientificNumbersClose(coefficient.confidence_interval_lower, expectedLower)
      || !scientificNumbersClose(coefficient.confidence_interval_upper, expectedUpper)
      || !isFiniteNumber(coefficient.odds_ratio)
      || coefficient.odds_ratio <= 0
      || !numbersClose(coefficient.odds_ratio, Math.exp(coefficient.estimate))
      || !isFiniteNumber(coefficient.odds_ratio_confidence_interval_lower)
      || !isFiniteNumber(coefficient.odds_ratio_confidence_interval_upper)
      || !numbersClose(coefficient.odds_ratio_confidence_interval_lower, Math.exp(coefficient.confidence_interval_lower))
      || !numbersClose(coefficient.odds_ratio_confidence_interval_upper, Math.exp(coefficient.confidence_interval_upper))) return null;
  }

  const fit = regression.fit;
  const parameterCount = expectedTerms.length;
  if (!fit
    || fit.r_squared != null
    || fit.adjusted_r_squared != null
    || fit.f_statistic != null
    || fit.rmse != null
    || !isFiniteNumber(fit.log_likelihood)
    || !isFiniteNumber(fit.null_log_likelihood)
    || !isFiniteNumber(fit.pseudo_r_squared)
    || !isFiniteNumber(fit.deviance)
    || !isFiniteNumber(fit.null_deviance)
    || !isFiniteNumber(fit.likelihood_ratio_chi_square)
    || fit.likelihood_ratio_chi_square < 0
    || fit.likelihood_ratio_degrees_of_freedom !== expectedTerms.length - 1
    || !isProbability(fit.likelihood_ratio_p_value)
    || !scientificNumbersClose(
      fit.likelihood_ratio_p_value,
      chiSquareSurvival(fit.likelihood_ratio_chi_square, fit.likelihood_ratio_degrees_of_freedom),
    )
    || fit.pseudo_r_squared_method !== "mcfadden_v1"
    || !isFiniteNumber(fit.aic)
    || !isFiniteNumber(fit.bic)
    || !numbersClose(fit.pseudo_r_squared, 1 - fit.log_likelihood / fit.null_log_likelihood)
    || !numbersClose(fit.deviance, -2 * fit.log_likelihood)
    || !numbersClose(fit.null_deviance, -2 * fit.null_log_likelihood)
    || !numbersClose(fit.likelihood_ratio_chi_square, fit.null_deviance - fit.deviance)
    || !numbersClose(fit.aic, fit.deviance + 2 * parameterCount)
    || !numbersClose(fit.bic, fit.deviance + Math.log(regression.observations) * parameterCount)) return null;

  const diagnostics = regression.logistic;
  const profile = diagnostics.outcome_profile;
  if (profile.outcome !== outcome
    || profile.coding !== "numeric_0_1_exact_v1"
    || profile.readiness !== "ready"
    || profile.complete_cases !== regression.observations
    || profile.omitted_cases !== result.omitted_observations
    || profile.zero_count + profile.one_count !== profile.complete_cases
    || profile.zero_count < 1
    || profile.one_count < 1
    || profile.invalid_count !== 0
    || !isProbability(profile.prevalence)
    || !numbersClose(profile.prevalence, profile.one_count / profile.complete_cases)) return null;

  const convergence = diagnostics.convergence;
  if (convergence.algorithm !== "deterministic_newton_irls_v1"
    || !convergence.converged
    || !isPositiveInteger(convergence.iterations)
    || convergence.max_iterations !== 100
    || convergence.iterations > convergence.max_iterations
    || !numbersClose(convergence.tolerance, 1e-8)
    || !isFiniteNumber(convergence.final_max_abs_step)
    || convergence.final_max_abs_step < 0
    || convergence.final_max_abs_step >= convergence.tolerance
    || !numbersClose(convergence.separation_probability_tolerance, 1e-9)) return null;

  let truePositive = 0;
  let trueNegative = 0;
  let falsePositive = 0;
  let falseNegative = 0;
  let observedZeroCount = 0;
  let observedOneCount = 0;
  let reconstructedLogLikelihood = 0;
  const threshold = diagnostics.classification.threshold;
  if (!numbersClose(threshold, 0.5)) return null;
  for (const [index, prediction] of regression.predictions.entries()) {
    if (prediction.observation !== index
      || !isProbability(prediction.fitted)
      || !isFiniteNumber(prediction.residual)
      || !isProbability(prediction.probability)
      || !numbersClose(prediction.fitted, prediction.probability)
      || prediction.probability < convergence.separation_probability_tolerance
      || prediction.probability > 1 - convergence.separation_probability_tolerance) return null;
    const observed = prediction.probability + prediction.residual;
    if (!numbersClose(observed, 0) && !numbersClose(observed, 1)) return null;
    const observedOne = numbersClose(observed, 1);
    if (observedOne) observedOneCount += 1;
    else observedZeroCount += 1;
    reconstructedLogLikelihood += observedOne
      ? Math.log(prediction.probability)
      : Math.log(1 - prediction.probability);
    const predictedOne = prediction.probability >= threshold;
    if (observedOne && predictedOne) truePositive += 1;
    else if (!observedOne && !predictedOne) trueNegative += 1;
    else if (!observedOne && predictedOne) falsePositive += 1;
    else falseNegative += 1;
  }
  const classification = diagnostics.classification;
  const reconstructedPrevalence = observedOneCount / regression.observations;
  const reconstructedNullLogLikelihood = observedOneCount * Math.log(reconstructedPrevalence)
    + observedZeroCount * Math.log(1 - reconstructedPrevalence);
  if (profile.zero_count !== observedZeroCount
    || profile.one_count !== observedOneCount
    || !numbersClose(profile.prevalence, reconstructedPrevalence)
    || !numbersClose(fit.log_likelihood, reconstructedLogLikelihood)
    || !numbersClose(fit.null_log_likelihood, reconstructedNullLogLikelihood)
    || !numbersClose(fit.likelihood_ratio_chi_square, Math.max(0, 2 * (reconstructedLogLikelihood - reconstructedNullLogLikelihood)))
    || classification.true_positive !== truePositive
    || classification.true_negative !== trueNegative
    || classification.false_positive !== falsePositive
    || classification.false_negative !== falseNegative
    || !isProbability(classification.accuracy)
    || !isProbability(classification.sensitivity)
    || !isProbability(classification.specificity)
    || !numbersClose(classification.accuracy, (truePositive + trueNegative) / regression.observations)
    || !numbersClose(classification.sensitivity, truePositive / (truePositive + falseNegative))
    || !numbersClose(classification.specificity, trueNegative / (trueNegative + falsePositive))) return null;

  const warnings = regression.warnings.map((warning) => warning.trim()).filter(Boolean);
  if (!warnings.includes(NATIVE_LOGISTIC_ENGINE_SCOPE_WARNING)) return null;
  const bootstrap = bootstrapRun
    ? validateRegressionBootstrap(run, regression, expectedTerms, true)
    : null;
  if ((bootstrapRun && !bootstrap) || (!bootstrapRun && regression.bootstrap)) return null;
  return {
    methodVersion: "regression_logistic_v2",
    outcome,
    predictors,
    controls,
    observations: regression.observations,
    coefficients: regression.coefficients,
    fit,
    predictions: regression.predictions,
    diagnostics,
    bootstrap,
    warnings,
  };
}

export function nativeLegacyLogisticResultProjection(
  run: AnalysisRun | null | undefined,
): NativeLegacyLogisticResultProjection | null {
  if (!isCompletedResultRun(run) || run.modelId || run.modelSnapshot) return null;
  const provenance = run.provenance;
  const result = run.result;
  const regression = result.regression;
  if (!regression
    || provenance?.method !== "regression"
    || provenance.method_version !== "regression_logistic_v1"
    || provenance.settings.method !== "regression"
    || provenance.settings.weighting_scheme !== "path"
    || (provenance.settings.preprocessing !== "standardized" && provenance.settings.preprocessing !== "unstandardized")
    || provenance.settings.bootstrap_samples !== 0
    || provenance.settings.studentized_inner_samples !== 0
    || provenance.settings.permutation_samples !== 0
    || provenance.settings.case_weight_column !== null
    || !numbersClose(provenance.settings.confidence_level, 0.95)
    || result.method_version !== "regression_logistic_v1"
    || regression.method_version !== "regression_logistic_v1"
    || regression.regression_type !== "logistic"
    || regression.logistic
    || regression.bootstrap
    || regression.process) return null;
  if (run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_STANDALONE_ASSESSMENT_WARNING) return null;

  const outcome = regression.outcome.trim();
  const predictors = regression.predictors.filter(hasText);
  const controls = regression.controls.filter(hasText);
  const variables = [outcome, ...predictors, ...controls];
  const expectedTerms = ["intercept", ...predictors, ...controls];
  if (!outcome
    || predictors.length < 1
    || new Set(variables).size !== variables.length
    || !isPositiveInteger(regression.observations)
    || result.used_observations !== regression.observations
    || regression.coefficients.length !== expectedTerms.length
    || regression.predictions.length !== regression.observations) return null;
  if (regression.coefficients.some((row, index) => row.term !== expectedTerms[index]
    || !isFiniteNumber(row.estimate)
    || !isFiniteNumber(row.standard_error)
    || row.standard_error <= 0
    || !isFiniteNumber(row.statistic)
    || !isProbability(row.p_value_two_sided)
    || !isFiniteNumber(row.confidence_interval_lower)
    || !isFiniteNumber(row.confidence_interval_upper)
    || !isFiniteNumber(row.odds_ratio)
    || row.odds_ratio <= 0
    || row.odds_ratio_confidence_interval_lower != null
    || row.odds_ratio_confidence_interval_upper != null)) return null;
  const fit = regression.fit;
  if (!fit
    || fit.r_squared != null
    || fit.adjusted_r_squared != null
    || fit.f_statistic != null
    || fit.rmse != null
    || !isFiniteNumber(fit.log_likelihood)
    || !isFiniteNumber(fit.pseudo_r_squared)
    || !isFiniteNumber(fit.aic)
    || !isFiniteNumber(fit.bic)
    || fit.null_log_likelihood != null
    || fit.deviance != null
    || fit.null_deviance != null
    || fit.likelihood_ratio_chi_square != null
    || fit.likelihood_ratio_degrees_of_freedom != null
    || fit.likelihood_ratio_p_value != null
    || fit.pseudo_r_squared_method != null) return null;
  if (regression.predictions.some((row, index) => row.observation !== index
    || !isProbability(row.fitted)
    || !isFiniteNumber(row.residual)
    || !isProbability(row.probability)
    || !numbersClose(row.fitted, row.probability))) return null;
  const warnings = regression.warnings.map((warning) => warning.trim()).filter(Boolean);
  if (!warnings.includes(NATIVE_LEGACY_LOGISTIC_ENGINE_SCOPE_WARNING)) return null;
  return {
    methodVersion: "regression_logistic_v1",
    recordedPreprocessing: provenance.settings.preprocessing,
    outcome,
    predictors,
    controls,
    observations: regression.observations,
    coefficients: regression.coefficients,
    fit,
    predictions: regression.predictions,
    warnings,
  };
}

export function nativeRegressionBootstrapResultProjection(
  run: AnalysisRun | null | undefined,
): RegressionBootstrapAnalysis | null {
  return nativeOlsResultProjection(run)?.bootstrap
    ?? nativeLogisticResultProjection(run)?.bootstrap
    ?? null;
}

interface NativeHigherOrderProjection {
  constructIds: ReadonlySet<string>;
  componentRows: string[][];
  structuralRows: string[][];
  scopeRows: string[][];
}

function nativeHigherOrderProjection(
  run: AnalysisRun,
  constructLabel: ConstructDisplayLabel,
): NativeHigherOrderProjection | null {
  if (!run.result || !run.modelSnapshot) return null;
  const nodesById = new Map(run.modelSnapshot.nodes.map((node) => [node.id, node]));
  const declarations = run.modelSnapshot.nodes.flatMap((node) => {
    const declaration = node.data.higherOrder;
    if (node.data.semantic !== "higher_order"
      || !declaration
      || declaration.id !== node.id
      || declaration.method !== "two_stage"
      || declaration.components.length < 2
      || new Set(declaration.components).size !== declaration.components.length
      || declaration.components.some((componentId) => {
        const component = nodesById.get(componentId);
        return !component || Boolean(component.data.semantic) || component.data.mode !== "reflective" || component.data.indicators.length === 0;
      })) return [];
    return [{ hocId: node.id, componentIds: declaration.components }];
  });
  if (!declarations.length) return null;

  const constructIds = new Set(declarations.map((declaration) => declaration.hocId));
  const estimatesByIdentity = new Map(run.result.outer_estimates
    .filter((row) => hasText(row.construct) && hasText(row.indicator))
    .map((row) => [`${row.construct}\u0000${row.indicator}`, row]));
  const componentRows = declarations.flatMap((declaration) => declaration.componentIds.flatMap((componentId) => {
    const indicator = generatedHigherOrderIndicatorName(declaration.hocId, componentId);
    const estimate = estimatesByIdentity.get(`${declaration.hocId}\u0000${indicator}`);
    return estimate && isFiniteNumber(estimate.loading) && isFiniteNumber(estimate.weight)
      ? [[
          constructLabel(declaration.hocId),
          constructLabel(componentId),
          "Disjoint two-stage",
          formatNumber(estimate.loading),
          formatNumber(estimate.weight),
        ]]
      : [];
  }));
  if (!componentRows.length) return null;

  const structuralRows = run.result.paths
    .filter((row) => constructIds.has(row.source)
      && !constructIds.has(row.target)
      && hasText(row.target)
      && isFiniteNumber(row.coefficient))
    .map((row) => [constructPathLabel([row.source, row.target], constructLabel), formatNumber(row.coefficient)]);
  const scopeRows = declarations.map((declaration) => [
    constructLabel(declaration.hocId),
    declaration.componentIds.map(constructLabel).join(", "),
    "Reflective-reflective disjoint two-stage",
    "Stage 1 component scores; stage 2 generated score indicators",
  ]);
  return { constructIds, componentRows, structuralRows, scopeRows };
}

/**
 * Builds the compact native results contract from a completed run. Tables are
 * included only when the engine payload contains at least one real output row.
 */
export function nativePlsModelFitV2Projection(
  run: AnalysisRun | null | undefined,
): PlsModelFit | null {
  const fit = run?.assessment?.model_fit;
  const result = run?.result;
  const technicalConstructIds = new Set(run?.modelSnapshot?.nodes
    .filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order")
    .map((node) => node.id) ?? []);
  const analyticalOuterEstimates = result?.outer_estimates.filter((row) => !technicalConstructIds.has(row.construct)) ?? [];
  if (!fit || !result
    || fit.method_version !== PLS_MODEL_FIT_METHOD_VERSION_V2
    || !Number.isSafeInteger(fit.analytical_sample_size)
    || fit.analytical_sample_size !== result.used_observations
    || !Array.isArray(fit.indicator_order)
    || fit.indicator_order.length === 0
    || fit.indicator_order.length !== analyticalOuterEstimates.length
    || new Set(fit.indicator_order).size !== fit.indicator_order.length
    || fit.indicator_order.some((indicator, index) => (
      !hasText(indicator) || indicator !== analyticalOuterEstimates[index]?.indicator
    ))
    || fit.matrix_convention !== PLS_MODEL_FIT_MATRIX_CONVENTION_V2
    || fit.geodesic_logarithm !== PLS_MODEL_FIT_GEODESIC_LOGARITHM_V2
    || !fitMatrixV2Valid(fit.observed_correlation, fit.indicator_order.length)
    || !fitMatrixV2Valid(fit.saturated_implied_correlation, fit.indicator_order.length)
    || !fitMatrixV2Valid(fit.estimated_implied_correlation, fit.indicator_order.length)
    || !fitMeasureV2Valid(fit.saturated, fit.observed_correlation, fit.saturated_implied_correlation)
    || !fitMeasureV2Valid(fit.estimated, fit.observed_correlation, fit.estimated_implied_correlation)
    || !fitCriterionV2Valid(fit.null_model_chi_square, true)
    || !fitNfiV2Coherent(fit.saturated, fit.null_model_chi_square)
    || !fitNfiV2Coherent(fit.estimated, fit.null_model_chi_square)
    || fit.exact_fit_inference?.procedure !== PLS_MODEL_FIT_EXACT_INFERENCE_PROCEDURE_V2
    || fit.exact_fit_inference.status !== "unavailable"
    || fit.exact_fit_inference.reason_code !== "model_fit.adapted_bollen_stine_not_implemented") {
    return null;
  }
  return fit;
}

const PLS_MODEL_FIT_EXACT_METHOD_VERSION_V1 = "pls_model_fit_exact_v1";
const PLS_MODEL_FIT_EXACT_DIGEST = /^[a-f0-9]{64}$/;

export function nativePlsModelFitExactProjection(
  run: AnalysisRun | null | undefined,
): PlsModelFitExactInference | null {
  const point = nativePlsModelFitV2Projection(run);
  const exact = run?.bootstrap?.model_fit_exact_inference;
  const result = run?.result;
  const marker = run?.provenance?.method_version
    .split("+")
    .includes(PLS_MODEL_FIT_EXACT_METHOD_VERSION_V1) ?? false;
  if (!run || !result || !point || !exact || !marker
    || exact.method_version !== PLS_MODEL_FIT_EXACT_METHOD_VERSION_V1
    || exact.point_fit_method_version !== PLS_MODEL_FIT_METHOD_VERSION_V2
    || exact.estimator_method_version !== result.method_version
    || exact.resampling_method_version !== "indexed_resampling_v4"
    || exact.procedure !== "adapted_bollen_stine_saturated_and_estimated_v1"
    || exact.transformation !== "centered_standardized_x_times_s_inverse_half_times_sigma_half_v1"
    || exact.matrix_power !== "symmetric_self_adjoint_positive_definite_eigendecomposition_v1"
    || exact.quantile_method !== "hyndman_fan_type7_v1"
    || exact.decision_rule !== "original_less_than_or_equal_to_upper_quantile_not_rejected_v1"
    || exact.retry_policy !== "no_retry_no_replacement_fixed_indexed_draws_v1"
    || exact.sample_digest_method !== "sha256_u64_le_v1"
    || exact.usable_index_digest_method !== "sha256_u32_le_v1"
    || exact.matrix_digest_method !== "sha256_f64_bits_row_major_v1"
    || exact.minimum_usable_fraction !== 0.9
    || !Number.isSafeInteger(exact.analytical_sample_size)
    || exact.analytical_sample_size !== point.analytical_sample_size
    || exact.analytical_sample_size !== result.used_observations
    || !Number.isSafeInteger(exact.requested_replicates)
    || exact.requested_replicates < 999
    || exact.requested_replicates > 10_000
    || exact.requested_replicates !== run.bootstrap?.plan.replicates
    || exact.requested_replicates !== run.provenance?.settings.bootstrap_samples
    || exact.master_seed !== run.bootstrap?.plan.master_seed
    || exact.master_seed !== run.provenance?.settings.seed
    || exact.indicator_order.length !== point.indicator_order?.length
    || exact.indicator_order.some((indicator, index) => indicator !== point.indicator_order?.[index])
    || !PLS_MODEL_FIT_EXACT_DIGEST.test(exact.observed_correlation_sha256)
    || !fitExactVariantValid(exact.saturated, "saturated", point.saturated_implied_correlation!, point.saturated, exact)
    || !fitExactVariantValid(exact.estimated, "estimated", point.estimated_implied_correlation!, point.estimated, exact)
    || exact.status !== aggregateExactStatus([exact.saturated.status, exact.estimated.status])) {
    return null;
  }
  return exact;
}

function fitExactVariantValid(
  variant: PlsModelFitExactVariantInference,
  expectedVariant: "saturated" | "estimated",
  target: number[][],
  point: PlsModelFit["saturated"],
  bundle: PlsModelFitExactInference,
): boolean {
  const expectedOperation = `pls_model_fit_exact_${expectedVariant}_v1`;
  if (variant.variant !== expectedVariant
    || variant.operation !== expectedOperation
    || variant.requested_replicates !== bundle.requested_replicates
    || variant.ledger.length !== bundle.requested_replicates
    || !PLS_MODEL_FIT_EXACT_DIGEST.test(variant.target_correlation_sha256)
    || !PLS_MODEL_FIT_EXACT_DIGEST.test(variant.transformed_correlation_sha256)
    || !fitMatrixV2Valid(variant.transformed_correlation, target.length)
    || !isFiniteNumber(variant.transformation_max_abs_error)
    || variant.transformation_max_abs_error < 0) return false;
  let maxError = 0;
  for (let row = 0; row < target.length; row += 1) {
    for (let column = 0; column < target.length; column += 1) {
      maxError = Math.max(maxError, Math.abs(variant.transformed_correlation[row][column] - target[row][column]));
    }
  }
  if (maxError > 1e-9 || !numbersClose(maxError, variant.transformation_max_abs_error)) return false;
  if (variant.ledger.some((entry, index) => {
    if (entry.replicate_index !== index
      || !PLS_MODEL_FIT_EXACT_DIGEST.test(entry.sample_indices_sha256)) return true;
    const values = [entry.srmr, entry.d_uls, entry.d_g];
    if (values.some((value) => value !== null && (!isFiniteNumber(value) || value < 0))) return true;
    const usable = values.filter(isFiniteNumber).length;
    const expectedStatus = usable === 3 ? "success" : usable > 0 ? "partial" : "failed";
    const globalFailure = entry.failure_reason_code !== null || entry.failure_message !== null;
    if (globalFailure) {
      return usable !== 0
        || entry.status !== "failed"
        || entry.criterion_failures.length !== 0
        || !hasText(entry.failure_reason_code)
        || !hasText(entry.failure_message);
    }
    if (entry.status !== expectedStatus
      || entry.failure_reason_code !== null
      || entry.failure_message !== null
      || entry.criterion_failures.length !== 3 - usable) return true;
    const missing = new Set<"srmr" | "d_uls" | "d_g">();
    if (entry.srmr === null) missing.add("srmr");
    if (entry.d_uls === null) missing.add("d_uls");
    if (entry.d_g === null) missing.add("d_g");
    const failures = new Set(entry.criterion_failures.map((failure) => failure.criterion));
    return failures.size !== entry.criterion_failures.length
      || failures.size !== missing.size
      || entry.criterion_failures.some((failure) => !missing.has(failure.criterion) || !hasText(failure.reason_code));
  })) return false;
  const dG = fitCriterionV2Number(point.d_g);
  if (dG === null) return false;
  const expected = [
    ["srmr", point.srmr],
    ["d_uls", point.d_uls],
    ["d_g", dG],
  ] as const;
  if (variant.criteria.length !== expected.length
    || variant.criteria.some((summary, index) => !fitExactCriterionValid(
      summary,
      expected[index][0],
      expected[index][1],
      variant,
    ))) return false;
  return variant.status === aggregateExactStatus(variant.criteria.map((criterion) => criterion.status));
}

function fitExactCriterionValid(
  summary: PlsModelFitExactCriterionInference,
  criterion: "srmr" | "d_uls" | "d_g",
  original: number,
  variant: PlsModelFitExactVariantInference,
): boolean {
  const values = variant.ledger.flatMap((entry) => {
    const value = criterion === "srmr" ? entry.srmr : criterion === "d_uls" ? entry.d_uls : entry.d_g;
    return isFiniteNumber(value) ? [value] : [];
  }).sort((left, right) => left - right);
  const requested = variant.requested_replicates;
  const minimum = Math.max(2, Math.ceil(requested * 0.9));
  const usable = values.length;
  const exceedOrEqual = values.filter((value) => value >= original).length;
  if (summary.criterion !== criterion
    || !numbersClose(summary.original, original)
    || summary.requested_replicates !== requested
    || summary.minimum_usable_replicates !== minimum
    || summary.usable_replicates !== usable
    || summary.failed_replicates !== requested - usable
    || !PLS_MODEL_FIT_EXACT_DIGEST.test(summary.usable_replicate_indices_sha256)
    || summary.exceed_or_equal_count !== exceedOrEqual) return false;
  if (usable < minimum) {
    return summary.status === "unavailable"
      && summary.replicate_min === null
      && summary.replicate_max === null
      && summary.upper_95 === null
      && summary.upper_99 === null
      && summary.not_rejected_95 === null
      && summary.not_rejected_99 === null
      && summary.empirical_upper_tail_probability === null
      && summary.unavailable_reason_code === "model_fit_exact.insufficient_usable_replicates";
  }
  const upper95 = type7ExactQuantile(values, 0.95);
  const upper99 = type7ExactQuantile(values, 0.99);
  return summary.status === "available"
    && numbersClose(summary.replicate_min, values[0])
    && numbersClose(summary.replicate_max, values[usable - 1])
    && numbersClose(summary.upper_95, upper95)
    && numbersClose(summary.upper_99, upper99)
    && summary.not_rejected_95 === (original <= upper95)
    && summary.not_rejected_99 === (original <= upper99)
    && numbersClose(summary.empirical_upper_tail_probability, exceedOrEqual / usable)
    && summary.unavailable_reason_code === null;
}

function type7ExactQuantile(sorted: number[], probability: number): number {
  const position = (sorted.length - 1) * probability;
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  return lower === upper
    ? sorted[lower]
    : sorted[lower] + (position - lower) * (sorted[upper] - sorted[lower]);
}

function aggregateExactStatus(statuses: PlsModelFitExactStatus[]): PlsModelFitExactStatus {
  if (statuses.every((status) => status === "available")) return "available";
  if (statuses.some((status) => status !== "unavailable")) return "partial";
  return "unavailable";
}

/**
 * Classifies model-fit presentation from validated result authority. Exact-fit
 * tone is derived from the persisted aggregate status, never payload presence.
 */
export function nativeModelFitPresentationStateV2(
  run: AnalysisRun | null | undefined,
): NativeModelFitPresentationStateV2 | null {
  if (!run) return null;
  if (nativeHigherOrderProjection(run, constructDisplayLabelResolver(run))) {
    return {
      mode: "higher_order_not_reported",
      aggregateStatus: null,
      detailValue: "Not reported for this higher-order workflow",
      advisory: {
        tone: "neutral",
        title: "Model fit not reported",
        message: "QuickPLS does not report PLS model-fit measures for this supported higher-order construct workflow. Interpret the component relationships, structural paths, and method details instead.",
      },
    };
  }

  const hasExactMarker = run.provenance?.method_version
    .split("+")
    .includes(PLS_MODEL_FIT_EXACT_METHOD_VERSION_V1) ?? false;
  const hasExactPayload = Boolean(run.bootstrap?.model_fit_exact_inference);
  const fit = nativePlsModelFitV2Projection(run);
  const storedFit = run.assessment?.model_fit;
  if (!fit) {
    if (hasExactMarker || hasExactPayload) {
      return {
        mode: "exact_failed",
        aggregateStatus: null,
        detailValue: "Run failed",
        advisory: {
          tone: "error",
          title: "Exact-fit run failed",
          message: "The requested exact-fit result did not pass its stored authority checks. Its decisions are hidden; review Run Details before retrying.",
        },
      };
    }
    if (!storedFit || storedFit.method_version) return null;
    return {
      mode: "descriptive",
      aggregateStatus: null,
      detailValue: "Historical descriptive measures",
      advisory: {
        tone: "neutral",
        title: "About these measures",
        message: "This historical result reports descriptive SRMR and d_ULS only. It remains under its original result identity and has not been relabelled as the current model-fit method.",
      },
    };
  }

  const exact = nativePlsModelFitExactProjection(run);
  if (hasExactMarker !== hasExactPayload || (hasExactPayload && !exact)) {
    return {
      mode: "exact_failed",
      aggregateStatus: null,
      detailValue: "Run failed",
      advisory: {
        tone: "error",
        title: "Exact-fit run failed",
        message: "The requested exact-fit result did not pass its stored authority checks. Its decisions are hidden; review Run Details before retrying.",
      },
    };
  }
  if (!exact) {
    return {
      mode: "descriptive",
      aggregateStatus: null,
      detailValue: "Not run",
      advisory: {
        tone: "info",
        title: "About these measures",
        message: "SRMR and NFI summarize approximate fit. d_ULS and d_G are discrepancy values; treat them as descriptive unless this result also includes adapted Bollen-Stine exact-fit inference.",
      },
    };
  }

  if (exact.status === "available") {
    return {
      mode: "exact_available",
      aggregateStatus: exact.status,
      detailValue: "Results available",
      advisory: {
        tone: "info",
        title: "Exact fit available",
        message: "Adapted Bollen-Stine decisions for SRMR, d_ULS, and d_G are available under Model fit — exact inference.",
      },
    };
  }
  if (exact.status === "partial") {
    return {
      mode: "exact_partial",
      aggregateStatus: exact.status,
      detailValue: "Results partial",
      advisory: {
        tone: "warning",
        title: "Exact fit partially available",
        message: "The requested adapted Bollen-Stine run completed, but one or more criteria did not meet the usable-replicate requirement. Available decisions are shown and unavailable cells remain explicit.",
      },
    };
  }
  return {
    mode: "exact_unavailable",
    aggregateStatus: exact.status,
    detailValue: "Results unavailable",
    advisory: {
      tone: "warning",
      title: "Exact fit unavailable",
      message: "The requested adapted Bollen-Stine run did not meet the usable-replicate requirement for either model. Review the replicate exceptions before interpreting d_ULS or d_G.",
    },
  };
}

function fitMatrixV2Valid(matrix: unknown, dimension: number): matrix is number[][] {
  if (!Array.isArray(matrix) || matrix.length !== dimension) return false;
  for (let row = 0; row < dimension; row += 1) {
    if (!Array.isArray(matrix[row]) || matrix[row].length !== dimension) return false;
    for (let column = 0; column < dimension; column += 1) {
      const value = matrix[row][column];
      if (!isFiniteNumber(value)
        || Math.abs(value) > 1 + 1e-10
        || (row === column && !numbersClose(value, 1))
        || !numbersClose(value, matrix[column]?.[row])) return false;
    }
  }
  return true;
}

function fitCriterionV2Valid(
  criterion: unknown,
  nonnegative: boolean,
): criterion is NonNullable<PlsModelFit["null_model_chi_square"]> {
  if (!criterion || typeof criterion !== "object") return false;
  if ("value" in criterion) {
    return (criterion as { status?: unknown }).status === "available"
      && isFiniteNumber((criterion as { value?: unknown }).value)
      && (!nonnegative || (criterion as { value: number }).value >= 0);
  }
  return (criterion as { status?: unknown }).status === "unavailable"
    && hasText((criterion as { reason_code?: unknown }).reason_code);
}

function fitCriterionV2Number(
  criterion: PlsModelFit["null_model_chi_square"],
): number | null {
  return criterion?.status === "available" && isFiniteNumber(criterion.value)
    ? criterion.value
    : null;
}

function fitMeasureV2Valid(
  measure: PlsModelFit["saturated"],
  observed: number[][],
  implied: number[][],
): boolean {
  if (!isFiniteNumber(measure.srmr) || measure.srmr < 0
    || !isFiniteNumber(measure.d_uls) || measure.d_uls < 0
    || !fitCriterionV2Valid(measure.d_g, true)
    || !fitCriterionV2Valid(measure.chi_square, true)
    || !fitCriterionV2Valid(measure.degrees_of_freedom, true)
    || !fitCriterionV2Valid(measure.nfi, false)) return false;
  const dimension = observed.length;
  let expectedDuls = 0;
  for (let row = 0; row < dimension; row += 1) {
    for (let column = 0; column <= row; column += 1) {
      expectedDuls += (observed[row][column] - implied[row][column]) ** 2;
    }
  }
  const expectedSrmr = Math.sqrt(expectedDuls / (dimension * (dimension + 1) / 2));
  const degreesOfFreedom = fitCriterionV2Number(measure.degrees_of_freedom);
  return numbersClose(measure.d_uls, expectedDuls)
    && numbersClose(measure.srmr, expectedSrmr)
    && (degreesOfFreedom === null || Number.isSafeInteger(degreesOfFreedom));
}

function fitNfiV2Coherent(
  measure: PlsModelFit["saturated"],
  nullChiSquare: PlsModelFit["null_model_chi_square"],
): boolean {
  const model = fitCriterionV2Number(measure.chi_square);
  const baseline = fitCriterionV2Number(nullChiSquare);
  const nfi = fitCriterionV2Number(measure.nfi);
  if (model !== null && baseline !== null && baseline > Number.EPSILON) {
    return nfi !== null && numbersClose(nfi, 1 - model / baseline);
  }
  return nfi === null;
}

const PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION = "pls_bootstrap_null_centered_test_tail_v1";
const PLS_BOOTSTRAP_TEST_TAIL_INFERENCE_KEYS = [
  "method_version",
  "selected_test_tail",
  "parameters",
] as const;
const PLS_BOOTSTRAP_TEST_TAIL_PARAMETER_KEYS = [
  "parameter",
  "usable_replicates",
  "two_sided_exceedances",
  "greater_or_equal_exceedances",
  "less_or_equal_exceedances",
  "p_value_two_sided",
  "p_value_greater",
  "p_value_less",
] as const;

export interface NativePlsBootstrapTestTailProjection {
  selectedTestTail: "one_sided_greater" | "one_sided_less";
  rows: Array<{
    parameter: string;
    usableReplicates: number;
    selectedExceedances: number;
    selectedProbability: number;
  }>;
}

function exactObjectKeys(value: object, keys: readonly string[]): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

function parsedPlsBootstrapTestTail(
  run: AnalysisRun,
): { valid: boolean; projection: NativePlsBootstrapTestTailProjection | null } {
  const selected = run.provenance?.settings?.bootstrap_test_tail ?? "two_sided";
  if (!["two_sided", "one_sided_greater", "one_sided_less"].includes(selected)) {
    return { valid: false, projection: null };
  }
  const markerCount = run.provenance?.method_version
    .split("+")
    .filter((version) => version === PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION)
    .length ?? 0;
  const receipt = run.bootstrap?.test_tail_inference;
  if (selected === "two_sided") {
    return { valid: markerCount === 0 && receipt === undefined, projection: null };
  }
  if (!run.bootstrap
    || run.provenance?.method !== "pls_pm"
    || markerCount !== 1
    || !receipt
    || !exactObjectKeys(receipt, PLS_BOOTSTRAP_TEST_TAIL_INFERENCE_KEYS)
    || receipt.method_version !== PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION
    || receipt.selected_test_tail !== selected
    || !Array.isArray(receipt.parameters)
    || receipt.parameters.length !== run.bootstrap.percentile.parameters.length
    || !Number.isSafeInteger(run.bootstrap.usable_replicates)
    || run.bootstrap.usable_replicates < 1
    || run.bootstrap.usable_replicates > run.bootstrap.plan.replicates) {
    return { valid: false, projection: null };
  }
  const identities = new Set<string>();
  const rows: NativePlsBootstrapTestTailProjection["rows"] = [];
  for (const [index, row] of receipt.parameters.entries()) {
    const percentile = run.bootstrap.percentile.parameters[index];
    if (!exactObjectKeys(row, PLS_BOOTSTRAP_TEST_TAIL_PARAMETER_KEYS)
      || !hasText(row.parameter)
      || row.parameter !== percentile?.parameter
      || identities.has(row.parameter)
      || row.usable_replicates !== run.bootstrap.usable_replicates
      || percentile.usable_replicates !== row.usable_replicates) {
      return { valid: false, projection: null };
    }
    identities.add(row.parameter);
    const pairs = [
      [row.two_sided_exceedances, row.p_value_two_sided],
      [row.greater_or_equal_exceedances, row.p_value_greater],
      [row.less_or_equal_exceedances, row.p_value_less],
    ] as const;
    if (pairs.some(([count, probability]) => (
      !Number.isSafeInteger(count)
      || count < 0
      || count > row.usable_replicates
      || !isFiniteNumber(probability)
      || probability < 0
      || probability > 1
      || !Object.is(probability, (count + 1) / (row.usable_replicates + 1))
    ))) return { valid: false, projection: null };
    const greater = selected === "one_sided_greater";
    rows.push({
      parameter: row.parameter,
      usableReplicates: row.usable_replicates,
      selectedExceedances: greater
        ? row.greater_or_equal_exceedances
        : row.less_or_equal_exceedances,
      selectedProbability: greater ? row.p_value_greater : row.p_value_less,
    });
  }
  return { valid: true, projection: { selectedTestTail: selected, rows } };
}

/** Fail-closed bootstrap tail validator used by canonical hydration and rendering. */
export function nativePlsBootstrapTestTailContractValid(
  run: AnalysisRun | null | undefined,
): boolean {
  return Boolean(run && parsedPlsBootstrapTestTail(run).valid);
}

export function nativePlsBootstrapTestTailProjection(
  run: AnalysisRun | null | undefined,
): NativePlsBootstrapTestTailProjection | null {
  if (!run) return null;
  const parsed = parsedPlsBootstrapTestTail(run);
  return parsed.valid ? parsed.projection : null;
}

export function nativeResultTables(run: AnalysisRun | null | undefined): ResultTable[] {
  const power = nativePlsSampleSizePowerResultProjection(run);
  if (power) return nativePlsSampleSizePowerResultTables(power);
  if (run?.plsSampleSizePower || run?.provenance?.method === "pls_sample_size_power") return [];
  if (!isCompletedResultRun(run)) return [];
  if (!nativePlsBootstrapTestTailContractValid(run)) return [];

  const tables: ResultTable[] = [];
  const result = run.result;
  const consistentBootstrap = nativePlscConsistentBootstrapProjection(run);
  if (run.provenance?.method === "plsc"
    && (run.provenance.settings.bootstrap_samples > 0 || run.bootstrap)
    && !consistentBootstrap) return [];
  const consistentPermutation = nativePlscConsistentPermutationProjection(run);
  if (run.provenance?.method === "plsc"
    && (run.provenance.settings.permutation_samples > 0 || run.permutation)
    && !consistentPermutation) return [];
  const boundedHigherOrder = nativeHigherOrderProjection(run, constructDisplayLabelResolver(run));
  const modelFitV2 = nativePlsModelFitV2Projection(run);
  if (run.assessment?.model_fit?.method_version && !modelFitV2 && !boundedHigherOrder) return [];
  const modelFitExact = nativePlsModelFitExactProjection(run);
  const hasModelFitExactVersion = run.provenance?.method_version
    .split("+")
    .includes(PLS_MODEL_FIT_EXACT_METHOD_VERSION_V1) ?? false;
  const hasModelFitExactPayload = Boolean(run.bootstrap?.model_fit_exact_inference);
  if (hasModelFitExactVersion !== hasModelFitExactPayload
    || (hasModelFitExactPayload && !modelFitExact)) return [];
  const modelFitPresentation = nativeModelFitPresentationStateV2(run);
  const posthocMinimumSampleSize = nativePlsPosthocMinimumSampleSizeProjection(run);
  if (result.posthoc_minimum_sample_size && !posthocMinimumSampleSize) return [];
  const structuralPathRandomization = nativeStructuralPathRandomizationProjection(run);
  const inferenceRun = run.permutation && !structuralPathRandomization
    ? { ...run, permutation: undefined }
    : run;
  const ctaPls = nativeCtaPlsResultProjection(run);
  if ((run.provenance?.method === "cta_pls" || result.cta_pls || result.method_version === NATIVE_CTA_PLS_METHOD_VERSION) && !ctaPls) {
    return [];
  }
  if (run.provenance?.method === "gsca" || result.gsca || result.method_version === NATIVE_GSCA_METHOD_VERSION) {
    addGscaResultTables(tables, run);
    return tables;
  }
  if (run.provenance?.method === "nca" || result.nca || result.method_version === "nca_v2") {
    addNcaResultTables(tables, run);
    return tables;
  }
  if (run.provenance?.method === "pca" || result.pca || result.method_version === "pca_v1") {
    addPcaResultTables(tables, run);
    return tables;
  }
  if (run.provenance?.method === "regression" || result.regression || result.method_version.startsWith("regression_")) {
    const process = nativeProcessResultProjection(run);
    const legacyProcess = nativeLegacyProcessResultProjection(run);
    if (process) tables.push(...nativeProcessResultTables(process));
    else if (legacyProcess) tables.push(...nativeLegacyProcessResultTables(legacyProcess));
    else if (result.method_version === "regression_logistic_v2") addLogisticResultTables(tables, run);
    else if (result.method_version === "regression_logistic_v1") addLegacyLogisticResultTables(tables, run);
    else addOlsResultTables(tables, run);
    return tables;
  }
  if (run.provenance?.method === "cbsem" || result.cbsem) {
    addCbsemResultTables(tables, run);
    return tables;
  }
  if (result.endogeneity) {
    const endogeneity = methodResultTables(result).find((table) => table.id === "endogeneity_copula");
    if (endogeneity) tables.push(endogeneity);
  }
  const constructLabel = constructDisplayLabelResolver(run);
  const hasHtmtBootstrapVersion = run.provenance?.method_version
    .split("+")
    .includes("htmt_bias_corrected_bootstrap_inference_v1") ?? false;
  const hasHtmtBootstrapPayload = Boolean(run.bootstrap?.htmt_inference);
  if (hasHtmtBootstrapVersion !== hasHtmtBootstrapPayload) return [];
  if (result.mga) {
    addMgaResultTables(tables, run, constructLabel);
    return tables;
  }
  if (result.micom?.method_version === CURRENT_MICOM_METHOD_VERSION) {
    if (run.provenance?.method !== "mga"
      || !run.provenance.method_version.split("+").includes(CURRENT_MICOM_METHOD_VERSION)) return [];
    const projection = currentStandaloneMicomProjection(result.micom);
    if (!projection) return [];
    addMicomResultTables(
      tables,
      projection,
      projection.analysis.groups[0].group,
      projection.analysis.groups[1].group,
      constructLabel,
    );
    addMicomAccountingTable(tables, projection.analysis);
    return tables;
  }
  const moderationProductConstructIds = new Set((result.moderation?.estimates ?? [])
    .filter((row) => hasText(row.product_construct))
    .map((row) => row.product_construct));
  const higherOrder = boundedHigherOrder;
  const higherOrderConstructIds = new Set(run.modelSnapshot?.nodes
    .filter((node) => node.data.semantic === "higher_order")
    .map((node) => node.id) ?? []);
  const technicalConstructIds = new Set([...moderationProductConstructIds, ...higherOrderConstructIds]);
  const controlPairs = new Set((result.control_estimates ?? [])
    .filter((row) => hasText(row.source) && hasText(row.target))
    .map((row) => effectPairKey(row.source, row.target)));
  const substantivePaths = result.paths.filter((row) =>
    !controlPairs.has(effectPairKey(row.source, row.target))
    && !technicalConstructIds.has(row.source)
    && !technicalConstructIds.has(row.target)
    && !higherOrderConstructIds.has(row.source)
    && !higherOrderConstructIds.has(row.target));
  const specificIndirectEffects = deriveSpecificIndirectEffects(substantivePaths);
  const hasMediation = Boolean(result.mediation && specificIndirectEffects.effects.length);
  const totalIndirectEffects = (result.mediation?.estimates ?? []).filter((row) =>
    hasText(row.source)
    && hasText(row.target)
    && isFiniteNumber(row.indirect)
    && specificIndirectEffects.mediatedPairs.has(effectPairKey(row.source, row.target)),
  );
  const primaryOuterEstimates = result.plsc
    ? result.plsc.corrected_outer_loadings ?? []
    : result.outer_estimates;

  if (higherOrder) {
    addTable(tables, {
      id: "hoc_component_relationships",
      title: "Higher-order component relationships",
      warning: null,
      columns: ["Higher-order construct", "Lower-order component", "Method", "Loading", "Weight"],
      rows: higherOrder.componentRows,
    });
    addTable(tables, {
      id: "hoc_structural_paths",
      title: "Higher-order structural paths",
      warning: null,
      columns: ["Path", "Coefficient"],
      rows: higherOrder.structuralRows,
    });
    addTable(tables, {
      id: "hoc_scope",
      title: "Higher-order method and run details",
      warning: null,
      advisory: modelFitPresentation?.mode === "higher_order_not_reported"
        ? modelFitPresentation.advisory
        : null,
      columns: ["Higher-order construct", "Components", "Method", "Generated measurement"],
      rows: higherOrder.scopeRows,
    });
  }

  addTable(tables, {
    id: hasMediation ? "direct_effects" : "path_coefficients",
    title: hasMediation ? "Direct effects" : "Path coefficients",
    warning: null,
    columns: hasMediation ? ["Effect", "Direct effect"] : ["Path", "Coefficient"],
    rows: substantivePaths
      .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.coefficient))
      .map((row) => [constructPathLabel([row.source, row.target], constructLabel), formatNumber(row.coefficient)]),
  });

  const technicalSampleSize = posthocMinimumSampleSize;
  if (technicalSampleSize) {
    const required = technicalSampleSize.technically_required_sample_size;
    const status = technicalSampleSize.meets_technical_requirement;
    const hasDriver = hasText(technicalSampleSize.driver_source)
      && hasText(technicalSampleSize.driver_target)
      && isFiniteNumber(technicalSampleSize.minimum_absolute_path_coefficient);
    const resultRows = required === null
      ? [
          ["Result status", posthocSampleSizeStatusLabel(technicalSampleSize.status)],
          ["Analytical sample size", String(technicalSampleSize.analytical_sample_size)],
          ["Eligible structural paths", String(technicalSampleSize.eligible_path_count ?? result.paths.length)],
          ["Formula assumptions", "5% significance, 80% power, directional inverse-square-root test"],
          ["Inference requirement", technicalSampleSize.method_version === "inverse_square_root_posthoc_v2"
            ? "Complete two-sided PLS bootstrap probabilities are required for significance-aware path selection"
            : "Not recorded for this historical result"],
        ]
      : [
          ["Technically required sample size", String(required)],
          ["Analytical sample size", String(technicalSampleSize.analytical_sample_size)],
          ["Technical requirement", status === null ? "Cannot be determined" : status ? "Met" : "Not met"],
          ...(hasDriver ? [
            ["Driving path", constructPathLabel([technicalSampleSize.driver_source!, technicalSampleSize.driver_target!], constructLabel)],
            ["Absolute path coefficient", formatNumber(technicalSampleSize.minimum_absolute_path_coefficient!)],
          ] : []),
          ...(isFiniteNumber(technicalSampleSize.driver_p_value_two_sided) ? [
            ["Bootstrap p value (two-sided)", formatNumber(technicalSampleSize.driver_p_value_two_sided)],
          ] : []),
          ["Significant structural paths", String(technicalSampleSize.significant_path_count ?? "Not recorded")],
          ...(technicalSampleSize.method_version === "inverse_square_root_posthoc_v2" ? [
            ["Driver selection", "Smallest absolute path with two-sided normal-reference bootstrap p ≤ 0.05"],
          ] : []),
          ["Formula assumptions", "5% significance, 80% power, directional inverse-square-root test"],
          ["Method", "Inverse square root"],
        ];
    addTable(tables, {
      id: "posthoc_minimum_sample_size",
      title: "Post-hoc minimum sample size",
      status: technicalSampleSize.method_version === "inverse_square_root_posthoc_v2" ? "validated" : "experimental",
      warning: technicalSampleSize.caution,
      columns: ["Result", "Value"],
      rows: resultRows,
    });
  }

  addTable(tables, {
    id: "control_effects",
    title: "Control effects",
    warning: null,
    columns: ["Control relationship", "Label", "Coefficient"],
    rows: (result.control_estimates ?? [])
      .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.coefficient))
      .map((row) => [
        constructPathLabel([row.source, row.target], constructLabel),
        row.label?.trim() || "Control",
        formatNumber(row.coefficient),
      ]),
  });

  addTable(tables, {
    id: "outer_loadings",
    title: "Outer loadings",
    warning: null,
    columns: ["Construct", "Indicator", "Loading"],
    rows: primaryOuterEstimates
      .filter((row) => hasText(row.construct) && hasText(row.indicator) && isFiniteNumber(row.loading)
        && !technicalConstructIds.has(row.construct) && !isGeneratedTechnicalIndicator(row.indicator))
      .map((row) => [constructLabel(row.construct), row.indicator, formatNumber(row.loading)]),
  });

  addTable(tables, {
    id: "outer_weights",
    title: "Outer weights",
    warning: null,
    columns: ["Construct", "Indicator", "Weight"],
    rows: primaryOuterEstimates
      .filter((row) => hasText(row.construct) && hasText(row.indicator) && isFiniteNumber(row.weight)
        && !technicalConstructIds.has(row.construct) && !isGeneratedTechnicalIndicator(row.indicator))
      .map((row) => [constructLabel(row.construct), row.indicator, formatNumber(row.weight)]),
  });

  addTable(tables, {
    id: "r_squared",
    title: "R-square",
    warning: null,
    columns: ["Construct", "R²"],
    rows: finiteRecordRows(result.r_squared, constructLabel),
  });

  addTable(tables, {
    id: "specific_indirect_effects",
    title: "Specific indirect effects",
    warning: specificIndirectEffects.truncated ? SPECIFIC_INDIRECT_EFFECTS_TRUNCATED_WARNING : null,
    columns: ["Path", "Specific indirect effect"],
    rows: hasMediation
      ? specificIndirectEffects.effects.map((row) => [constructPathLabel(row.path, constructLabel), formatNumber(row.effect)])
      : [],
  });

  addTable(tables, {
    id: "total_indirect_effects",
    title: "Total indirect effects",
    warning: null,
    columns: ["Effect", "Total indirect effect"],
    rows: hasMediation
      ? totalIndirectEffects.map((row) => [constructPathLabel([row.source, row.target], constructLabel), formatNumber(row.indirect)])
      : [],
  });

  addTable(tables, {
    id: "total_effects",
    title: "Total effects",
    warning: null,
    columns: hasMediation ? ["Effect", "Total effect"] : ["Effect", "Direct", "Indirect", "Total"],
    rows: result.effects
      .filter((row) => !controlPairs.has(effectPairKey(row.source, row.target)))
      .filter((row) => !higherOrderConstructIds.has(row.source) && !higherOrderConstructIds.has(row.target))
      .filter((row) => hasText(row.source) && hasText(row.target) && [row.direct, row.indirect, row.total].every(isFiniteNumber))
      .map((row) => hasMediation
        ? [constructPathLabel([row.source, row.target], constructLabel), formatNumber(row.total)]
        : [constructPathLabel([row.source, row.target], constructLabel), formatNumber(row.direct), formatNumber(row.indirect), formatNumber(row.total)]),
  });

  const moderationEstimates = (result.moderation?.estimates ?? []).filter((row) =>
    hasText(row.predictor)
    && hasText(row.moderator)
    && hasText(row.product_construct)
    && hasText(row.outcome)
    && isFiniteNumber(row.interaction_effect),
  );
  const moderationWarnings = [...new Set(moderationEstimates
    .map((row) => row.warning?.trim())
    .filter((warning): warning is string => Boolean(warning)))];
  addTable(tables, {
    id: "moderation_effects",
    title: "Moderation effects",
    warning: moderationWarnings.length ? moderationWarnings.join(" ") : null,
    columns: ["Effect", "Role", "Coefficient"],
    rows: moderationEstimates.flatMap((row) => [
      ...(isFiniteNumber(row.predictor_main_effect) ? [[
        constructPathLabel([row.predictor, row.outcome], constructLabel),
        "Predictor main effect",
        formatNumber(row.predictor_main_effect),
      ]] : []),
      ...(isFiniteNumber(row.moderator_main_effect) ? [[
        constructPathLabel([row.moderator, row.outcome], constructLabel),
        "Moderator main effect",
        formatNumber(row.moderator_main_effect),
      ]] : []),
      [
        `${constructLabel(row.predictor)} × ${constructLabel(row.moderator)} → ${constructLabel(row.outcome)}`,
        "Interaction effect",
        formatNumber(row.interaction_effect),
      ],
    ]),
  });
  addTable(tables, {
    id: "moderation_simple_slopes",
    title: "Simple slope analysis",
    warning: null,
    columns: ["Relationship", "Moderator", "Moderator score", "Conditional effect"],
    rows: moderationEstimates.flatMap((row) => row.simple_slopes
      .filter((slope) => isFiniteNumber(slope.moderator_score) && isFiniteNumber(slope.effect))
      .map((slope) => [
        constructPathLabel([row.predictor, row.outcome], constructLabel),
        constructLabel(row.moderator),
        formatNumber(slope.moderator_score),
        formatNumber(slope.effect),
      ])),
  });

  if (result.plsc) {
    addTable(tables, {
      id: "plsc_reliability",
      title: "PLSc correction reliability",
      warning: null,
      columns: ["Construct", "rho_A"],
      rows: result.plsc.reliabilities
        .filter((row) => hasText(row.construct) && isFiniteNumber(row.rho_a))
        .map((row) => [constructLabel(row.construct), formatNumber(row.rho_a)]),
    });

    addTable(tables, {
      id: "plsc_correlations",
      title: "PLSc construct correlations",
      warning: null,
      columns: ["Left construct", "Right construct", "Original", "Corrected"],
      rows: result.plsc.construct_correlations
        .filter((row) => hasText(row.left) && hasText(row.right) && isFiniteNumber(row.original) && isFiniteNumber(row.corrected))
        .map((row) => [constructLabel(row.left), constructLabel(row.right), formatNumber(row.original), formatNumber(row.corrected)]),
    });
  }

  if (result.wpls) {
    const diagnosticRows = [
      hasText(result.wpls.case_weight_column) ? ["Case-weight column", result.wpls.case_weight_column] : null,
      isFiniteNumber(result.wpls.weight_sum) ? ["Weight sum", formatNumber(result.wpls.weight_sum)] : null,
      isFiniteNumber(result.wpls.effective_sample_size) ? ["Effective sample size", formatNumber(result.wpls.effective_sample_size)] : null,
      hasText(result.wpls.covariance) ? ["Covariance estimator", sentenceCase(result.wpls.covariance.replaceAll("_", " "))] : null,
    ].filter((row): row is string[] => row !== null);
    addTable(tables, {
      id: "wpls_weights",
      title: "WPLS case-weight diagnostics",
      warning: null,
      columns: ["Metric", "Value"],
      rows: diagnosticRows,
    });
  }

  if (result.cca) {
    const residualRows = result.cca.correlations
      .filter((row) => hasText(row.left)
        && hasText(row.right)
        && isFiniteNumber(row.observed)
        && isFiniteNumber(row.reproduced)
        && isFiniteNumber(row.residual)
        && isFiniteNumber(row.absolute_residual));
    const summaryRows = [
      hasText(result.cca.model)
        ? ["Model", result.cca.model === "recursive_standardized_composite_path_model_v1"
          ? "Recursive standardized composite path model"
          : sentenceCase(result.cca.model.replaceAll("_", " "))]
        : null,
      ["Correlation pairs", String(residualRows.length)],
      isFiniteNumber(result.cca.max_absolute_residual)
        ? ["Maximum absolute residual", formatNumber(result.cca.max_absolute_residual)]
        : null,
    ].filter((row): row is string[] => row !== null);
    addTable(tables, {
      id: "cca_residual_summary",
      title: "Residual summary",
      warning: null,
      columns: ["Metric", "Value"],
      rows: summaryRows,
    });
    addTable(tables, {
      id: "cca_composite_residuals",
      title: "Composite residuals",
      warning: null,
      columns: ["Composite pair", "Observed correlation", "Reproduced correlation", "Residual", "Absolute residual"],
      rows: residualRows.map((row) => [
        `${constructLabel(row.left)} ↔ ${constructLabel(row.right)}`,
        formatNumber(row.observed),
        formatNumber(row.reproduced),
        formatNumber(row.residual),
        formatNumber(row.absolute_residual),
      ]),
    });
  }

  if (ctaPls) {
    const constructLabel = constructDisplayLabelResolver(run);
    addTable(tables, {
      id: "cta_pls_summary",
      title: "CTA-PLS tetrad summary",
      status: "validated",
      warning: null,
      columns: ["Construct", "Indicators", "Four-indicator subsets", "Tetrads", "Maximum absolute tetrad"],
      rows: ctaPls.blocks.map((block) => [
        constructLabel(block.constructId),
        block.indicators.join(", "),
        String(block.quadruples),
        String(block.tetrads),
        formatNumber(ctaPls.maxAbsoluteTetradByConstruct[block.constructId]),
      ]),
    });
    addTable(tables, {
      id: "cta_pls_tetrads",
      title: "CTA-PLS tetrads",
      status: "validated",
      warning: null,
      columns: ["Construct", "Indicator A", "Indicator B", "Indicator C", "Indicator D", "Pairing", "Tetrad", "Absolute tetrad"],
      rows: ctaPls.estimates.map((row) => [
        constructLabel(row.construct),
        row.indicator_a,
        row.indicator_b,
        row.indicator_c,
        row.indicator_d,
        sentenceCase(row.pairing.replaceAll("_", " ")),
        formatNumber(row.tetrad),
        formatNumber(row.absolute_tetrad),
      ]),
    });
    addTable(tables, {
      id: "cta_pls_scope",
      title: "CTA-PLS run details",
      status: "validated",
      warning: null,
      columns: ["Field", "Value"],
      rows: [
        ["Method version", ctaPls.methodVersion],
        ["Covariance convention", ctaPls.covarianceVersion],
        ["Complete cases", String(ctaPls.usedObservations)],
        ["Omitted cases", String(ctaPls.omittedObservations)],
      ],
    });
  }

  if (result.ipma) {
    const plot = nativeIpmaPlot(run);
    if (plot) {
      const targetLabel = plot.targetLabel;
      const predecessorIds = new Set(plot.points.map((point) => point.constructId));
      addTable(tables, {
        id: "ipma_constructs",
        title: "Construct importance and performance",
        warning: null,
        columns: ["Target", "Predecessor construct", "Total importance", "Performance"],
        rows: plot.points.map((point) => [
          targetLabel,
          point.constructLabel,
          formatNumber(point.importance),
          formatNumber(point.performance, 4),
        ]),
      });
      addTable(tables, {
        id: "ipma_indicators",
        title: "Indicator performance",
        warning: null,
        columns: ["Target", "Construct", "Indicator", "Construct importance", "Loading", "Performance", "Standardized score mean"],
        rows: result.ipma.indicators
          .filter((row) => row.target === plot.targetId
            && predecessorIds.has(row.construct)
            && hasText(row.indicator)
            && [row.construct_importance, row.loading, row.performance, row.score_mean].every(isFiniteNumber)
            && row.performance >= 0
            && row.performance <= 100)
          .map((row) => [
            targetLabel,
            constructLabel(row.construct),
            row.indicator,
            formatNumber(row.construct_importance),
            formatNumber(row.loading),
            formatNumber(row.performance, 4),
            formatNumber(row.score_mean),
          ]),
      });
      addTable(tables, {
        id: "ipma_scope",
        title: "Run details",
        warning: null,
        columns: ["Field", "Value"],
        rows: [
          ["Target", targetLabel],
          ["Method version", result.ipma.method_version],
          ["Performance", "0–100 observed-range min–max scaling of standardized composite scores"],
          ["Missing data", "Listwise deletion"],
          ["Theoretical-range correction", "Not applied"],
        ],
      });
    }
  }

  const assessment = run.assessment;
  if (assessment) {
    const reliabilityMetrics = [
      { title: "Cronbach's alpha", value: (row: (typeof assessment.construct_quality)[number]) => row.cronbach_alpha },
      { title: "rho_A", value: (row: (typeof assessment.construct_quality)[number]) => row.rho_a },
      { title: "Composite reliability", value: (row: (typeof assessment.construct_quality)[number]) => row.rho_c },
      { title: "AVE", value: (row: (typeof assessment.construct_quality)[number]) => row.ave },
    ].filter((metric) => assessment.construct_quality.some((row) => !technicalConstructIds.has(row.construct) && isFiniteNumber(metric.value(row))));

    addTable(tables, {
      id: "construct_reliability",
      title: "Construct reliability and validity",
      warning: null,
      columns: ["Construct", ...reliabilityMetrics.map((metric) => metric.title)],
      rows: assessment.construct_quality
        .filter((row) => hasText(row.construct) && !technicalConstructIds.has(row.construct) && reliabilityMetrics.some((metric) => isFiniteNumber(metric.value(row))))
        .map((row) => [constructLabel(row.construct), ...reliabilityMetrics.map((metric) => formatOptionalNumber(metric.value(row)))]),
    });

    addTable(tables, {
      id: "cross_loadings",
      title: "Cross loadings",
      warning: null,
      columns: ["Indicator", "Assigned construct", "Construct", "Loading"],
      rows: assessment.cross_loadings
        .filter((row) => hasText(row.indicator) && hasText(row.assigned_construct) && hasText(row.construct) && isFiniteNumber(row.loading)
          && !isGeneratedTechnicalIndicator(row.indicator)
          && !technicalConstructIds.has(row.assigned_construct)
          && !technicalConstructIds.has(row.construct))
        .map((row) => [row.indicator, constructLabel(row.assigned_construct), constructLabel(row.construct), formatNumber(row.loading)]),
    });

    addTable(tables, {
      id: "fornell_larcker",
      title: "Fornell-Larcker criterion",
      warning: null,
      columns: ["Construct", "Compared with", "Value"],
      rows: numericMatrixRows(assessment.fornell_larcker.constructs, assessment.fornell_larcker.values, true, constructLabel, technicalConstructIds),
    });

    if (assessment.htmt_plus) {
      addTable(tables, htmtTable("htmt_plus", "HTMT+", assessment.htmt_plus, constructLabel, technicalConstructIds));
    }
    if (assessment.htmt_original) {
      addTable(tables, htmtTable("htmt_original", "Original HTMT", assessment.htmt_original, constructLabel, technicalConstructIds));
    }
    if (assessment.htmt && !assessment.htmt_plus && !assessment.htmt_original) {
      addTable(tables, {
        id: "htmt",
        title: "HTMT",
        warning: null,
        columns: ["Construct", "Compared with", "Value"],
        rows: numericMatrixRows(assessment.htmt.constructs, assessment.htmt.values, false, constructLabel, technicalConstructIds),
      });
    }
    if (run.bootstrap?.htmt_inference) {
      if (run.bootstrap.htmt_inference.method_version !== "htmt_bias_corrected_bootstrap_inference_v1"
        || !run.provenance?.method_version.split("+").includes("htmt_bias_corrected_bootstrap_inference_v1")) return [];
      const htmtInferenceTables = htmtBootstrapTables(
        run.bootstrap.htmt_inference.htmt_plus,
        run.bootstrap.htmt_inference.htmt_original,
        assessment,
        run.bootstrap.plan.replicates,
        run.bootstrap.failed_replicates.map((failure) => failure.replicate_index),
        constructLabel,
        technicalConstructIds,
      );
      if (!htmtInferenceTables) return [];
      for (const table of htmtInferenceTables) addTable(tables, table);
    }

    const hasAdjustedR2 = assessment.structural_quality.some((row) => isFiniteNumber(row.adjusted_r_squared));
    addTable(tables, {
      id: "structural_quality",
      title: "Structural model",
      warning: null,
      columns: ["Construct", "Predictors", "R²", ...(hasAdjustedR2 ? ["Adjusted R²"] : [])],
      rows: assessment.structural_quality
        .filter((row) => hasText(row.construct) && isFiniteNumber(row.predictor_count) && isFiniteNumber(row.r_squared))
        .map((row) => [constructLabel(row.construct), String(row.predictor_count), formatNumber(row.r_squared), ...(hasAdjustedR2 ? [formatOptionalNumber(row.adjusted_r_squared)] : [])]),
    });

    addTable(tables, {
      id: "structural_vif",
      title: "Inner VIF values",
      warning: null,
      columns: ["Target construct", "Predictor construct", "VIF"],
      rows: assessment.structural_vif
        .filter((row) => hasText(row.target_construct) && hasText(row.predictor_construct) && isFiniteNumber(row.vif))
        .map((row) => [constructLabel(row.target_construct), constructLabel(row.predictor_construct), formatOptionalNumber(row.vif)]),
    });

    addTable(tables, {
      id: "formative_indicator_vif",
      title: "Outer VIF values",
      warning: null,
      columns: ["Construct", "Indicator", "VIF"],
      rows: assessment.formative_indicator_vif
        .filter((row) => hasText(row.construct) && hasText(row.indicator) && isFiniteNumber(row.vif)
          && !technicalConstructIds.has(row.construct) && !isGeneratedTechnicalIndicator(row.indicator))
        .map((row) => [constructLabel(row.construct), row.indicator, formatOptionalNumber(row.vif)]),
    });

    const fSquaredRows = assessment.f_squared.filter((row) =>
      hasText(row.source_construct) && hasText(row.target_construct) && isFiniteNumber(row.f_squared),
    );
    const hasExcludedR2 = fSquaredRows.some((row) => isFiniteNumber(row.excluded_r_squared));
    addTable(tables, {
      id: "f_squared",
      title: "f-square effect sizes",
      warning: null,
      columns: ["Path", "R² included", ...(hasExcludedR2 ? ["R² excluded"] : []), "f²"],
      rows: fSquaredRows.map((row) => [
        constructPathLabel([row.source_construct, row.target_construct], constructLabel),
        formatNumber(row.included_r_squared),
        ...(hasExcludedR2 ? [formatOptionalNumber(row.excluded_r_squared)] : []),
        formatOptionalNumber(row.f_squared),
      ]),
    });

    if (assessment.model_fit && !higherOrder) {
      const fitColumns = modelFitV2
        ? ["Model", "SRMR", "d_ULS", "d_G", "Chi-square", "df", "NFI"]
        : ["Model", "SRMR", "d_ULS"];
      addTable(tables, {
        id: "model_fit",
        title: "Model fit — descriptive",
        warning: modelFitV2
          ? modelFitExact
            ? "SRMR and NFI are approximate fit measures. Exact-fit decisions for SRMR, d_ULS, and d_G are reported separately from the adapted Bollen-Stine run."
            : "SRMR and NFI are approximate fit measures. Interpret d_ULS and d_G only with adapted Bollen-Stine inference; that inference is not available for this run."
          : null,
        advisory: modelFitPresentation?.advisory ?? null,
        columns: fitColumns,
        rows: [
          fitRow("Saturated model", assessment.model_fit.saturated, Boolean(modelFitV2)),
          fitRow("Estimated model", assessment.model_fit.estimated, Boolean(modelFitV2)),
        ].filter((row): row is string[] => Boolean(row)),
      });
      if (modelFitV2) {
        addTable(tables, {
          id: "model_fit_details",
          title: "Model fit — method details",
          warning: null,
          advisory: modelFitPresentation?.advisory ?? null,
          columns: ["Field", "Value"],
          rows: [
            ["Analyzed observations", String(modelFitV2.analytical_sample_size)],
            ["Indicators", String(modelFitV2.indicator_order!.length)],
            ["Matrix basis", "Observed and model-implied indicator correlations"],
            ["Discrepancy cells", "Lower triangle, including the zero diagonal residuals"],
            ["d_G logarithm", "Natural logarithm"],
            ["Exact-fit procedure", "Adapted Bollen-Stine for saturated and estimated models"],
            ["Exact-fit inference", modelFitPresentation?.detailValue ?? "Unavailable for this run"],
            ...(modelFitExact ? [
              ["Requested exact-fit replicates per model", String(modelFitExact.requested_replicates)],
              ["Exact-fit retry policy", "No retry or replacement"],
            ] : []),
          ],
        });
        if (modelFitExact) {
          addTable(tables, {
            id: "model_fit_exact",
            title: "Model fit — exact inference",
            status: "experimental",
            warning: "Experimental adapted Bollen-Stine inference. The fixed saturated and estimated ledgers are separate from ordinary parameter bootstrapping.",
            advisory: modelFitPresentation?.advisory ?? null,
            columns: ["Model", "Criterion", "Original", "HI95", "HI99", "5% decision", "1% decision", "Empirical upper-tail probability", "Usable", "Failed"],
            rows: [modelFitExact.saturated, modelFitExact.estimated].flatMap((variant) =>
              variant.criteria.map((criterion) => [
                sentenceCase(variant.variant),
                criterion.criterion === "srmr" ? "SRMR" : criterion.criterion === "d_uls" ? "d_ULS" : "d_G",
                formatNumber(criterion.original),
                formatOptionalNumber(criterion.upper_95, 6),
                formatOptionalNumber(criterion.upper_99, 6),
                exactFitDecisionLabel(criterion.not_rejected_95),
                exactFitDecisionLabel(criterion.not_rejected_99),
                formatOptionalNumber(criterion.empirical_upper_tail_probability),
                String(criterion.usable_replicates),
                String(criterion.failed_replicates),
              ])),
          });
          const failureRows = [modelFitExact.saturated, modelFitExact.estimated].flatMap((variant) =>
            variant.ledger.filter((entry) => entry.status !== "success").map((entry) => [
              sentenceCase(variant.variant),
              String(entry.replicate_index),
              sentenceCase(entry.status),
              entry.failure_reason_code
                ?? entry.criterion_failures.map((failure) => `${failure.criterion}: ${failure.reason_code}`).join("; "),
              entry.failure_message ?? "Criterion-level unavailability",
              entry.sample_indices_sha256,
            ]));
          addTable(tables, {
            id: "model_fit_exact_failures",
            title: "Model fit — replicate exceptions",
            status: "experimental",
            warning: "Every partial or failed indexed draw is retained; no draw was retried or replaced.",
            advisory: modelFitPresentation?.advisory ?? null,
            columns: ["Model", "Replicate", "Status", "Reason", "Details", "Sample-index digest"],
            rows: failureRows,
          });
        }
      }
    }

    if (assessment.blindfolding) {
      const rows = assessment.blindfolding.constructs.filter((row) =>
        hasText(row.construct) && [row.q_squared, row.prediction_error_sum_squares, row.observation_sum_squares].some(isFiniteNumber),
      );
      const hasPress = rows.some((row) => isFiniteNumber(row.prediction_error_sum_squares));
      const hasSso = rows.some((row) => isFiniteNumber(row.observation_sum_squares));
      addTable(tables, {
        id: "blindfolding",
        title: "Construct cross-validated redundancy",
        warning: null,
        columns: ["Construct", "Q²", ...(hasPress ? ["Prediction error"] : []), ...(hasSso ? ["Observation sum"] : [])],
        rows: rows.map((row) => [
          constructLabel(row.construct),
          formatOptionalNumber(row.q_squared),
          ...(hasPress ? [formatOptionalNumber(row.prediction_error_sum_squares)] : []),
          ...(hasSso ? [formatOptionalNumber(row.observation_sum_squares)] : []),
        ]),
      });
    }
  }

  if (result.predict?.method_version === CURRENT_PLS_PREDICT_METHOD_VERSION) {
    const repeated = result.predict.repeated_kfold;
    const cvpat = repeated?.cvpat_benchmark_assessments ?? [];
    const exactV2 = repeated?.method_version === CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION
      && validPredictionAssignmentDigest(repeated.assignment_digest)
      && cvpat.length === 2
      && new Set(cvpat.map((row) => row.benchmark)).size === 2
      && cvpat.every((row) => (
        row.method_version === CURRENT_CVPAT_METHOD_VERSION
        && row.comparison_kind === "benchmark_assessment"
        && row.target_scope === "all_endogenous_indicators"
        && ["indicator_average", "linear_model"].includes(row.benchmark)
        && row.loss === "mean_squared_error_across_indicators_per_observation"
        && row.alternative === "pls_loss_less_than_benchmark"
        && row.confidence_level === 0.95
      ));
    if (exactV2) addCurrentPredictionTables(tables, run, constructLabel);
  } else if (result.predict?.method_version === LEGACY_PLS_PREDICT_METHOD_VERSION) {
    addLegacyPredictionTables(tables, result.predict, constructLabel);
  }

  const aggregateMediationParameters = hasMediation && run.bootstrap
    ? addAggregateMediationBootstrapTable(tables, run, totalIndirectEffects, controlPairs, constructLabel)
    : new Set<string>();
  const moderationInferenceParameters = moderationEstimates.length
    ? addModerationInferenceTables(tables, inferenceRun, moderationEstimates, constructLabel)
    : new Set<string>();
  const controlInferenceParameters = controlPairs.size
    ? addControlInferenceTables(tables, inferenceRun, result.control_estimates ?? [], constructLabel)
    : new Set<string>();

  if (consistentPermutation) {
    const permutation = consistentPermutation.permutation;
    const selectedTail = consistentPermutation.selectedTailInference;
    const directionalByParameter = new Map(
      (permutation.directional_inference?.parameters ?? []).map((parameter) => [parameter.parameter, parameter]),
    );
    const hasDirectional = directionalByParameter.size > 0;
    addTable(tables, {
      id: "plsc_permutation_accounting",
      title: "PLSc consistent permutation accounting",
      status: "experimental",
      warning: consistentPermutation.failedPermutations > 0
        ? `${consistentPermutation.failedPermutations} fixed group-label assignment(s) had a failed full-PLSc refit and were retained without retry or replacement.`
        : "Every fixed assignment completed both full-PLSc group refits.",
      columns: ["Field", "Value"],
      rows: [
        ["Requested label assignments", String(consistentPermutation.requestedPermutations)],
        ["Usable full-PLSc assignments", String(consistentPermutation.usablePermutations)],
        ["Failed full-PLSc assignments", String(consistentPermutation.failedPermutations)],
        ["Minimum usable assignments", String(consistentPermutation.minimumUsablePermutations)],
        ["Group column", permutation.group_column!],
        ["Test", hasDirectional
          ? "Two-tailed absolute and directed greater/less Group A minus Group B differences"
          : "Two-tailed absolute Group A minus Group B difference"],
        ["Significance level", formatNumber(permutation.significance_level!, 2)],
        ["Retry policy", "No retry or replacement assignment"],
      ],
    });
    addTable(tables, {
      id: "plsc_permutation_groups",
      title: "PLSc permutation groups",
      status: "experimental",
      warning: null,
      columns: ["Role", "Group", "Complete cases", "Parameter digest"],
      rows: [
        ["Group A", permutation.group_a!.group, String(permutation.group_a!.observations), permutation.group_a!.parameter_values_sha256],
        ["Group B", permutation.group_b!.group, String(permutation.group_b!.observations), permutation.group_b!.parameter_values_sha256],
      ],
    });
    if (selectedTail) {
      const selectedTailLabel = selectedTail.selected_test_tail === "group_a_greater"
        ? "Group A greater than Group B"
        : "Group A less than Group B";
      addTable(tables, {
        id: "plsc_permutation_selected_tail",
        title: "PLSc selected one-sided permutation test",
        status: "experimental",
        warning: "The selected one-sided probabilities use the same fixed usable-assignment ledger as the displayed directional inference; the directed contrast is Group A minus Group B.",
        columns: ["Field", "Value"],
        rows: [
          ["Method", selectedTail.method_version],
          ["Orientation", "Group A minus Group B"],
          ["Selected test", `${selectedTail.selected_test_tail} (${selectedTailLabel})`],
          ["Usable-assignment denominator", String(consistentPermutation.usablePermutations)],
        ],
      });
      addTable(tables, {
        id: "plsc_permutation_selected_tail_parameters",
        title: `PLSc selected one-sided results — ${selectedTailLabel}`,
        status: "experimental",
        warning: null,
        columns: ["Parameter", "Selected exceedances", "Selected p", "Usable assignments"],
        rows: selectedTail.parameters.map((parameter) => [
          parameterLabel(parameter.parameter, constructLabel),
          String(parameter.selected_exceedances),
          formatOptionalPValue(parameter.selected_p_value),
          String(parameter.permutations),
        ]),
      });
    }
    const parameterTable = (
      id: string,
      title: string,
      families: readonly NonNullable<(typeof permutation.parameters)[number]["family"]>[],
    ) => addTable(tables, {
      id,
      title,
      status: "experimental",
      warning: id === "plsc_permutation_construct_criteria"
        ? "This internal result does not include MICOM; do not treat these parameter differences as a measurement-invariance decision."
        : null,
      columns: hasDirectional
        ? ["Parameter", "Group A", "Group B", "Difference A − B", "p (two-tailed)", "Count ≥ observed", "p (greater)", "Count ≤ observed", "p (less)", "Usable assignments"]
        : ["Parameter", "Group A", "Group B", "Difference A − B", "p (two-tailed)", "Usable assignments"],
      rows: permutation.parameters
        .filter((parameter) => parameter.family != null && families.includes(parameter.family))
        .map((parameter) => {
          const directional = directionalByParameter.get(parameter.parameter);
          return [
            parameterLabel(parameter.parameter, constructLabel),
            formatOptionalNumber(parameter.estimate_a),
            formatOptionalNumber(parameter.estimate_b),
            formatNumber(parameter.original),
            formatOptionalPValue(parameter.p_value_two_sided),
            ...(directional ? [
              String(directional.greater_or_equal),
              formatOptionalPValue(directional.p_value_greater),
              String(directional.less_or_equal),
              formatOptionalPValue(directional.p_value_less),
            ] : []),
            String(parameter.permutations),
          ];
        }),
    });
    parameterTable("plsc_permutation_paths", "PLSc group path differences", ["path"]);
    parameterTable("plsc_permutation_outer_loadings", "PLSc group loading differences", ["outer_loading"]);
    parameterTable(
      "plsc_permutation_construct_criteria",
      "PLSc group construct-criterion differences",
      ["rho_a", "construct_correlation", "r_squared"],
    );
    if (permutation.failed_permutations!.length > 0) {
      addTable(tables, {
        id: "plsc_permutation_failures",
        title: "PLSc consistent-permutation failed assignments",
        status: "experimental",
        warning: null,
        columns: ["Assignment", "Reason", "Message", "Label-assignment digest"],
        rows: permutation.failed_permutations!.map((failure) => [
          String(failure.permutation_index + 1),
          failure.reason_code,
          failure.message,
          failure.label_assignment_sha256,
        ]),
      });
    }
  }

  if (run.bootstrap) {
    const isConsistentBootstrap = consistentBootstrap !== null;
    if (!isConsistentBootstrap) {
      addTable(tables, {
        id: "bootstrap_accounting",
        title: "PLS bootstrap replicate accounting",
        warning: run.bootstrap.failed_replicates.length > 0
          ? `${run.bootstrap.failed_replicates.length} preplanned PLS refit(s) failed and were retained without retry or replacement.`
          : null,
        columns: ["Field", "Value"],
        rows: [
          ["Requested case resamples", String(run.bootstrap.plan.replicates)],
          ["Attempted preplanned PLS refits", String(run.bootstrap.plan.replicates)],
          ["Usable PLS refits", String(run.bootstrap.usable_replicates)],
          ["Failed PLS refits", String(run.bootstrap.failed_replicates.length)],
          ["Retry policy", "No retry or replacement draw"],
        ],
      });
      if (run.bootstrap.failed_replicates.length > 0) {
        addTable(tables, {
          id: "bootstrap_failures",
          title: "PLS bootstrap failed refits",
          warning: null,
          columns: ["Replicate", "Reason code", "Message"],
          rows: run.bootstrap.failed_replicates.map((failure) => [
            String(failure.replicate_index + 1),
            failure.reason_code ?? "legacy_unclassified_failure",
            failure.message,
          ]),
        });
      }
    }
    if (consistentBootstrap) {
      addTable(tables, {
        id: "plsc_bootstrap_accounting",
        title: "PLSc bootstrap replicate accounting",
        warning: consistentBootstrap.failedReplicates > 0
          ? `${consistentBootstrap.failedReplicates} full-PLSc refit(s) failed and were retained without retry or replacement.`
          : null,
        columns: ["Field", "Value"],
        rows: [
          ["Requested case resamples", String(consistentBootstrap.requestedReplicates)],
          ["Attempted preplanned full-PLSc refits", String(consistentBootstrap.requestedReplicates)],
          ["Usable full-PLSc refits", String(consistentBootstrap.usableReplicates)],
          ["Failed full-PLSc refits", String(consistentBootstrap.failedReplicates)],
          ["Minimum usable refits", String(consistentBootstrap.minimumUsableReplicates)],
          ["Replayable successful-refit witnesses", String(consistentBootstrap.successfulReplicateWitnesses)],
          ["Delete-one PLSc fits", String(consistentBootstrap.jackknifeCases)],
          ["Replayable successful delete-one witnesses", String(consistentBootstrap.successfulJackknifeWitnesses)],
          ["Failed delete-one fits", String(consistentBootstrap.failedJackknifeCases)],
          ["BCa parameters available", String(consistentBootstrap.bcaAvailableParameters)],
          ["BCa parameters unavailable", String(consistentBootstrap.bcaUnavailableParameters)],
          ["Retry policy", "No retry or replacement draw"],
        ],
      });
      if (run.bootstrap.failed_replicates.length > 0) {
        addTable(tables, {
          id: "plsc_bootstrap_failures",
          title: "PLSc bootstrap failed refits",
          warning: null,
          columns: ["Replicate", "Reason", "Message", "Sample digest"],
          rows: run.bootstrap.failed_replicates.map((failure) => [
            String(failure.replicate_index + 1),
            failure.reason_code ?? "",
            failure.message,
            failure.sample_indices_sha256 ?? "",
          ]),
        });
      }
      if ((run.bootstrap.failed_jackknife_cases?.length ?? 0) > 0) {
        addTable(tables, {
          id: "plsc_bootstrap_jackknife_failures",
          title: "PLSc bootstrap failed delete-one fits",
          warning: "BCa intervals are unavailable when any required delete-one PLSc fit fails.",
          columns: ["Omitted complete case", "Reason", "Message"],
          rows: run.bootstrap.failed_jackknife_cases!.map((failure) => [
            String(failure.omitted_case + 1),
            failure.reason_code,
            failure.message,
          ]),
        });
      }
    }
    const percentileRows = run.bootstrap.percentile.parameters.filter((row) =>
      hasText(row.parameter)
      && !aggregateMediationParameters.has(row.parameter)
      && !moderationInferenceParameters.has(row.parameter)
      && !controlInferenceParameters.has(row.parameter)
      && [row.original, row.bootstrap_mean, row.standard_error, row.lower, row.upper].every(isFiniteNumber),
    );
    const hasTStatistic = percentileRows.some((row) => isFiniteNumber(row.t_statistic));
    const hasPValue = percentileRows.some((row) => isFiniteNumber(row.p_value_two_sided));
    addTable(tables, {
      id: "bootstrap_percentile",
      title: isConsistentBootstrap ? "PLSc consistent bootstrapping" : "Bootstrapping",
      warning: run.bootstrap.failed_replicates.length
        ? `${run.bootstrap.failed_replicates.length} ${isConsistentBootstrap ? "full-PLSc " : ""}bootstrap replicate(s) failed.`
        : null,
      columns: ["Parameter", "Original", "Mean", "STDEV", ...(hasTStatistic ? ["t"] : []), ...(hasPValue ? ["p"] : []), "CI lower", "CI upper"],
      rows: percentileRows.map((row) => [
        parameterLabel(row.parameter, constructLabel),
        formatNumber(row.original),
        formatNumber(row.bootstrap_mean),
        formatNumber(row.standard_error),
        ...(hasTStatistic ? [formatOptionalNumber(row.t_statistic, 3)] : []),
        ...(hasPValue ? [formatOptionalPValue(row.p_value_two_sided)] : []),
        formatNumber(row.lower),
        formatNumber(row.upper),
      ]),
    });

    const testTail = nativePlsBootstrapTestTailProjection(run);
    if (testTail) {
      const greater = testTail.selectedTestTail === "one_sided_greater";
      addTable(tables, {
        id: "bootstrap_one_sided_test_tail",
        title: greater ? "One-sided bootstrap test (greater)" : "One-sided bootstrap test (less)",
        warning: "Null-centered bootstrap differences use inclusive tail counts and the plus-one probability (count + 1) / (usable + 1).",
        columns: [
          "Parameter",
          greater ? "Count (null-centered Δ* ≥ original)" : "Count (null-centered Δ* ≤ original)",
          "Usable bootstrap draws",
          greater ? "p (greater, plus-one)" : "p (less, plus-one)",
        ],
        rows: testTail.rows.map((row) => [
          parameterLabel(row.parameter, constructLabel),
          String(row.selectedExceedances),
          String(row.usableReplicates),
          formatPValue(row.selectedProbability),
        ]),
      });
    }

    if (run.bootstrap.bca) {
      const rows = run.bootstrap.bca.parameters.filter((row) =>
        hasText(row.parameter)
        && !moderationInferenceParameters.has(row.parameter)
        && !controlInferenceParameters.has(row.parameter)
        && isFiniteNumber(row.lower)
        && isFiniteNumber(row.upper),
      );
      const hasBiasCorrection = rows.some((row) => isFiniteNumber(row.bias_correction));
      const hasAcceleration = rows.some((row) => isFiniteNumber(row.acceleration));
      addTable(tables, {
        id: "bootstrap_bca",
        title: "Bias-corrected and accelerated intervals",
        warning: null,
        columns: ["Parameter", ...(hasBiasCorrection ? ["Bias correction"] : []), ...(hasAcceleration ? ["Acceleration"] : []), "CI lower", "CI upper"],
        rows: rows.map((row) => [
          parameterLabel(row.parameter, constructLabel),
          ...(hasBiasCorrection ? [formatOptionalNumber(row.bias_correction)] : []),
          ...(hasAcceleration ? [formatOptionalNumber(row.acceleration)] : []),
          formatOptionalNumber(row.lower),
          formatOptionalNumber(row.upper),
        ]),
      });
      if (isConsistentBootstrap) {
        addTable(tables, {
          id: "plsc_bootstrap_bca_unavailable",
          title: "Unavailable PLSc BCa intervals",
          warning: null,
          columns: ["Parameter", "Reason"],
          rows: run.bootstrap.bca.parameters
            .filter((row) => hasText(row.unavailable_reason))
            .map((row) => [parameterLabel(row.parameter, constructLabel), row.unavailable_reason!]),
        });
      } else {
        addTable(tables, {
          id: "bootstrap_bca_unavailable",
          title: "Unavailable PLS bootstrap BCa intervals",
          warning: null,
          columns: ["Parameter", "Status", "Reason"],
          rows: run.bootstrap.bca.parameters
            .filter((row) => hasText(row.unavailable_reason))
            .map((row) => [
              parameterLabel(row.parameter, constructLabel),
              "Unavailable",
              row.unavailable_reason!,
            ]),
        });
      }
    }

    if (run.bootstrap.studentized) {
      const rows = run.bootstrap.studentized.parameters.filter((row) =>
        hasText(row.parameter)
        && !moderationInferenceParameters.has(row.parameter)
        && !controlInferenceParameters.has(row.parameter)
        && isFiniteNumber(row.original)
        && isFiniteNumber(row.outer_standard_error)
        && isFiniteNumber(row.lower)
        && isFiniteNumber(row.upper),
      );
      addTable(tables, {
        id: "bootstrap_studentized",
        title: "Studentized confidence intervals",
        warning: run.bootstrap.studentized.failure?.message ?? null,
        columns: ["Parameter", "Original", "Outer STDEV", "CI lower", "CI upper", "Usable replicates"],
        rows: rows.map((row) => [
          parameterLabel(row.parameter, constructLabel),
          formatNumber(row.original),
          formatNumber(row.outer_standard_error),
          formatOptionalNumber(row.lower),
          formatOptionalNumber(row.upper),
          String(row.usable_primary_replicates),
        ]),
      });
    }
  }

  if (structuralPathRandomization) {
    addTable(tables, nativeStructuralPathRandomizationTable(
      structuralPathRandomization,
      constructLabel,
      new Set([...moderationInferenceParameters, ...controlInferenceParameters]),
    ));
  }

  return tables;
}

export function buildNativeResultTree(run: AnalysisRun | null | undefined, tables = nativeResultTables(run)): NativeResultNavigationGroup[] {
  const power = nativePlsSampleSizePowerResultProjection(run);
  if (power) {
    return [{
      id: "sample_size_power",
      title: "Prospective sample size and power",
      items: tables.map((table) => ({ id: table.id, kind: "table", title: table.title, tableId: table.id })),
    }];
  }
  if (!isCompletedResultRun(run)) return [];

  const byId = new Map(tables.map((table) => [table.id, table]));
  const groups: NativeResultNavigationGroup[] = [];
  const hasGroupAnalysis = Boolean(
    run.result.mga
    || run.result.micom?.method_version === CURRENT_MICOM_METHOD_VERSION,
  );
  const standalone = isStandaloneNativeAnalysis(run.provenance?.method);
  if (!hasGroupAnalysis && !standalone) {
    groups.push({
      id: "graphical",
      title: "Graphical results",
      items: [{ id: "model_estimates", kind: "diagram", title: "Model estimates", diagram: "model_estimates" }],
    });
  }

  addTableGroup(groups, "groups", "Groups", MGA_GROUP_IDS, byId);
  addTableGroup(groups, "importance_performance", "Importance-performance map", IPMA_RESULT_IDS, byId);
  addTableGroup(groups, "necessary_conditions", "Necessary conditions", NCA_RESULT_IDS, byId);
  addTableGroup(groups, "components", "Principal components", PCA_RESULT_IDS, byId);
  const process = nativeProcessResultProjection(run);
  const legacyProcess = nativeLegacyProcessResultProjection(run);
  addTableGroup(
    groups,
    "process",
    process
      ? `Graph-defined path analysis${process.bootstrap ? " with bootstrap" : ""}`
      : "Historical PROCESS v1",
    process ? NATIVE_PROCESS_RESULT_IDS : NATIVE_LEGACY_PROCESS_RESULT_IDS,
    byId,
  );
  const logistic = nativeLogisticResultProjection(run);
  const legacyLogistic = nativeLegacyLogisticResultProjection(run);
  addTableGroup(
    groups,
    "regression",
    logistic
      ? `Binary logistic regression${logistic.bootstrap ? " with bootstrap" : ""}`
      : legacyLogistic
        ? "Legacy binary logistic regression (v1)"
        : `OLS regression${nativeOlsResultProjection(run)?.bootstrap ? " with bootstrap" : ""}`,
    logistic ? LOGISTIC_RESULT_IDS : legacyLogistic ? LEGACY_LOGISTIC_RESULT_IDS : OLS_RESULT_IDS,
    byId,
  );
  addTableGroup(groups, "covariance_sem", "CB-SEM / CFA", CBSEM_RESULT_IDS, byId);
  addTableGroup(groups, "gsca_component_model", "GSCA component model", GSCA_RESULT_IDS, byId);
  addTableGroup(groups, "assessment", "Assessment", CCA_ASSESSMENT_IDS, byId);
  addTableGroup(groups, "assessment", "Assessment", CTA_PLS_ASSESSMENT_IDS, byId);
  addTableGroup(groups, "assessment", "Endogeneity diagnostics", ENDOGENEITY_ASSESSMENT_IDS, byId);
  addTableGroup(groups, "higher_order", "Higher-order constructs", HIGHER_ORDER_IDS, byId);

  const hasMediation = MEDIATION_IDS.some((id) => id !== "total_effects" && byId.has(id));
  addTableGroup(
    groups,
    "final_results",
    "Final results",
    hasMediation ? FINAL_RESULT_IDS.filter((id) => id !== "total_effects") : FINAL_RESULT_IDS,
    byId,
  );
  if (hasMediation) addTableGroup(groups, "mediation", "Mediation", MEDIATION_IDS, byId);
  addTableGroup(groups, "moderation", "Moderation", MODERATION_IDS, byId);
  addTableGroup(groups, "quality_criteria", "Quality criteria", QUALITY_CRITERIA_IDS, byId);
  addTableGroup(groups, "prediction", "Prediction", PREDICTION_IDS, byId);
  addTableGroup(groups, "inference", "Inference", INFERENCE_IDS, byId);
  return groups;
}

export function buildNativeResultNavigation(run: AnalysisRun | null | undefined): NativeResultNavigation {
  const power = nativePlsSampleSizePowerResultProjection(run);
  if (power) {
    const tables = nativeResultTables(run);
    return {
      runId: run!.id,
      defaultItemId: "pls_power_by_sample_size",
      groups: buildNativeResultTree(run, tables),
      tables,
    };
  }
  if (!isCompletedResultRun(run)) {
    return { runId: null, defaultItemId: null, groups: [], tables: [] };
  }
  const tables = nativeResultTables(run);
  const groupDefault = ["micom_summary", "mga_permutation", "mga_path_differences", "mga_group_paths", "mga_group_summary"]
    .find((id) => tables.some((table) => table.id === id));
  const predictionDefault = PREDICTION_IDS.find((id) => tables.some((table) => table.id === id));
  const ccaDefault = CCA_ASSESSMENT_IDS.find((id) => tables.some((table) => table.id === id));
  const ctaPlsDefault = CTA_PLS_ASSESSMENT_IDS.find((id) => tables.some((table) => table.id === id));
  const endogeneityDefault = ENDOGENEITY_ASSESSMENT_IDS.find((id) => tables.some((table) => table.id === id));
  const ipmaDefault = IPMA_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const ncaDefault = NCA_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const pcaDefault = PCA_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const processDefault = tables.some((table) => table.id === "process_model_summary")
    ? "process_model_summary"
    : NATIVE_LEGACY_PROCESS_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const regressionBootstrapDefault = nativeRegressionBootstrapResultProjection(run)
    && tables.some((table) => table.id === "regression_bootstrap_summary")
    ? "regression_bootstrap_summary"
    : undefined;
  const olsDefault = OLS_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const logisticDefault = LOGISTIC_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const legacyLogisticDefault = LEGACY_LOGISTIC_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const cbsemDefault = CBSEM_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const gscaDefault = GSCA_RESULT_IDS.find((id) => tables.some((table) => table.id === id));
  const higherOrderDefault = HIGHER_ORDER_IDS.find((id) => tables.some((table) => table.id === id));
  const standalone = isStandaloneNativeAnalysis(run.provenance?.method);
  const fallbackDefault = run.result.mga
    || run.result.micom?.method_version === CURRENT_MICOM_METHOD_VERSION
    || standalone
    ? tables[0]?.id ?? null
    : "model_estimates";
  return {
    runId: run.id,
    defaultItemId: processDefault ?? regressionBootstrapDefault ?? groupDefault ?? ipmaDefault ?? ncaDefault ?? pcaDefault ?? logisticDefault ?? legacyLogisticDefault ?? olsDefault ?? cbsemDefault ?? gscaDefault ?? endogeneityDefault ?? ctaPlsDefault ?? ccaDefault ?? predictionDefault ?? higherOrderDefault ?? fallbackDefault,
    groups: buildNativeResultTree(run, tables),
    tables,
  };
}

export function resultTableForItem(navigation: NativeResultNavigation, itemId: string): ResultTable | undefined {
  const item = navigation.groups.flatMap((group) => group.items).find((candidate) => candidate.id === itemId);
  return item?.kind === "table" ? navigation.tables.find((table) => table.id === item.tableId) : undefined;
}

function addTableGroup(
  groups: NativeResultNavigationGroup[],
  id: NativeResultGroupId,
  title: string,
  tableIds: readonly string[],
  byId: ReadonlyMap<string, ResultTable>,
) {
  const items = tableIds.flatMap<NativeResultNavigationItem>((tableId) => {
    const table = byId.get(tableId);
    return table ? [{ id: table.id, kind: "table", title: table.title, tableId: table.id }] : [];
  });
  if (items.length) groups.push({ id, title, items });
}

type NativeStructuralPath = NonNullable<AnalysisRun["result"]>["paths"][number];

interface SpecificIndirectEffect {
  path: string[];
  effect: number;
}

interface SpecificIndirectEffectDerivation {
  effects: SpecificIndirectEffect[];
  mediatedPairs: ReadonlySet<string>;
  truncated: boolean;
}

function deriveSpecificIndirectEffects(paths: readonly NativeStructuralPath[]): SpecificIndirectEffectDerivation {
  const validPaths = paths.filter((path) =>
    hasText(path.source) && hasText(path.target) && isFiniteNumber(path.coefficient),
  );
  const adjacency = new Map<string, NativeStructuralPath[]>();
  for (const path of validPaths) {
    const outgoing = adjacency.get(path.source) ?? [];
    outgoing.push(path);
    adjacency.set(path.source, outgoing);
  }
  const mediatedPairs = deriveMediatedPairs(adjacency);

  const effects: SpecificIndirectEffect[] = [];
  const seen = new Set<string>();
  let truncated = false;
  for (const source of new Set(validPaths.map((path) => path.source))) {
    const walk = (
      current: string,
      visited: ReadonlySet<string>,
      chain: readonly NativeStructuralPath[],
      product: number,
    ) => {
      for (const path of adjacency.get(current) ?? []) {
        if (truncated) return;
        if (visited.has(path.target)) continue;
        const nextChain = [...chain, path];
        const nextProduct = product * path.coefficient;
        const nextVisited = new Set(visited).add(path.target);
        if (nextChain.length >= 2 && isFiniteNumber(nextProduct)) {
          const constructPath = [source, ...nextChain.map((item) => item.target)];
          const key = constructPath.join("\u0000");
          if (!seen.has(key)) {
            if (effects.length >= MAX_SPECIFIC_INDIRECT_EFFECTS) {
              truncated = true;
              return;
            }
            seen.add(key);
            effects.push({ path: constructPath, effect: nextProduct });
          }
        }
        walk(path.target, nextVisited, nextChain, nextProduct);
      }
    };
    walk(source, new Set([source]), [], 1);
    if (truncated) break;
  }
  return { effects, mediatedPairs, truncated };
}

function deriveMediatedPairs(adjacency: ReadonlyMap<string, readonly NativeStructuralPath[]>): ReadonlySet<string> {
  const pairs = new Set<string>();
  for (const [source, firstSteps] of adjacency) {
    for (const firstStep of firstSteps) {
      const visited = new Set([source, firstStep.target]);
      const pending = [...(adjacency.get(firstStep.target) ?? []).map((path) => path.target)];
      for (let index = 0; index < pending.length; index += 1) {
        const target = pending[index];
        if (visited.has(target)) continue;
        visited.add(target);
        pairs.add(effectPairKey(source, target));
        for (const path of adjacency.get(target) ?? []) pending.push(path.target);
      }
    }
  }
  return pairs;
}

function nativeResultRelationIdV1(run: AnalysisRun, source: string, target: string): string | null {
  const edge = run.modelSnapshot?.edges.find((candidate) => (
    candidate.source === source
    && candidate.target === target
    && (candidate.data as { role?: string } | undefined)?.role !== "control"
    && (candidate.data as { role?: string } | undefined)?.role !== "covariance"
  ));
  const authorityId = (edge?.data as {
    standardSemV4Authority?: { authorityObjectId?: string };
  } | undefined)?.standardSemV4Authority?.authorityObjectId;
  return authorityId?.trim() || edge?.id || null;
}

function nativeDistinctOverlayIdsV1(values: readonly (string | null | undefined)[]): string[] {
  return [...new Set(values.filter((value): value is string => Boolean(value)))];
}

/** Derives a read-only result focus without changing the archived run or model. */
export function nativeResultOverlaySelectionV1(
  run: AnalysisRun | null | undefined,
  selectedResultId: string | null | undefined,
): ResultOverlaySelectionV1 | null {
  if (!isCompletedResultRun(run) || !selectedResultId) return null;
  const result = run.result;

  if (/three_way|three-way/u.test(selectedResultId)) {
    const interactions = run.modelSnapshot?.nodes.flatMap((node) => {
      const interaction = node.data.interaction;
      return interaction?.kind === "interaction_v2" && interaction.operands.length === 3
        ? [{ interaction, nodeId: node.id }]
        : [];
    }) ?? [];
    if (!interactions.length) return null;
    const parentInteractions = interactions.flatMap(({ interaction }) => (
      run.modelSnapshot?.nodes.flatMap((node) => {
        const candidate = node.data.interaction;
        return candidate?.kind === "interaction_v2"
          && candidate.operands.length === 2
          && candidate.focalRelationId === interaction.focalRelationId
          && candidate.operands[0] === interaction.operands[0]
          && candidate.operands[1] === interaction.operands[1]
          ? [{ interaction: candidate, nodeId: node.id }]
          : [];
      }) ?? []
    ));
    return {
      kind: "three_way_moderation",
      nodeIds: nativeDistinctOverlayIdsV1(interactions.flatMap(({ interaction }) => [
        ...interaction.operands,
        interaction.outcome,
      ])),
      relationIds: nativeDistinctOverlayIdsV1(interactions.map(({ interaction }) => (
        interaction.focalRelationId
          || nativeResultRelationIdV1(run, interaction.operands[0], interaction.outcome)
      ))),
      interactionTermIds: nativeDistinctOverlayIdsV1([
        ...interactions.map(({ interaction }) => interaction.termId),
        ...parentInteractions.map(({ interaction }) => interaction.termId),
      ]),
      label: interactions.length === 1 ? "Three-way moderating effect" : `${interactions.length} three-way moderating effects`,
    };
  }

  if (/moderated_mediation|conditional_indirect/u.test(selectedResultId)) {
    const estimates = result.moderated_mediation?.estimates ?? [];
    if (!estimates.length) return null;
    return {
      kind: "moderated_mediation",
      nodeIds: nativeDistinctOverlayIdsV1(estimates.flatMap((estimate) => [
        estimate.predictor,
        estimate.mediator,
        estimate.target,
        estimate.moderator,
      ])),
      relationIds: nativeDistinctOverlayIdsV1(estimates.flatMap((estimate) => [
        nativeResultRelationIdV1(run, estimate.predictor, estimate.mediator),
        nativeResultRelationIdV1(run, estimate.mediator, estimate.target),
      ])),
      interactionTermIds: nativeDistinctOverlayIdsV1(estimates.map((estimate) => {
        const node = run.modelSnapshot?.nodes.find((candidate) => (
          candidate.data.semantic === "interaction"
          && candidate.data.interaction?.outcome === estimate.target
        ));
        return node?.data.interaction?.termId ?? estimate.interaction;
      })),
      label: estimates.length === 1 ? "Moderated mediation path" : `${estimates.length} moderated mediation paths`,
    };
  }

  if (/^moderation_/u.test(selectedResultId)) {
    const estimates = result.moderation?.estimates ?? [];
    if (!estimates.length) return null;
    return {
      kind: "moderation",
      nodeIds: nativeDistinctOverlayIdsV1(estimates.flatMap((estimate) => [
        estimate.predictor,
        estimate.moderator,
        estimate.outcome,
      ])),
      relationIds: nativeDistinctOverlayIdsV1(estimates.map((estimate) => (
        nativeResultRelationIdV1(run, estimate.predictor, estimate.outcome)
      ))),
      interactionTermIds: nativeDistinctOverlayIdsV1(estimates.map((estimate) => {
        const node = run.modelSnapshot?.nodes.find((candidate) => (
          candidate.id === estimate.product_construct
          || (candidate.data.semantic === "interaction"
            && candidate.data.interaction?.outcome === estimate.outcome
            && (candidate.data.interaction.kind === "interaction_v2"
              ? candidate.data.interaction.operands[0] === estimate.predictor
                && candidate.data.interaction.operands[1] === estimate.moderator
              : candidate.data.interaction?.predictor === estimate.predictor
                && candidate.data.interaction?.moderator === estimate.moderator))
        ));
        return node?.data.interaction?.termId ?? estimate.interaction;
      })),
      label: estimates.length === 1 ? "Moderating effect" : `${estimates.length} moderating effects`,
    };
  }

  if (/indirect|mediation|total_effect/u.test(selectedResultId)) {
    const technicalConstructIds = new Set(run.modelSnapshot?.nodes.flatMap((node) => (
      node.data.semantic === "interaction" || node.data.semantic === "higher_order"
        ? [node.id]
        : []
    )) ?? []);
    const controlPairs = new Set((result.control_estimates ?? []).map((row) => (
      effectPairKey(row.source, row.target)
    )));
    const paths = deriveSpecificIndirectEffects(result.paths.filter((row) => (
      !controlPairs.has(effectPairKey(row.source, row.target))
      && !technicalConstructIds.has(row.source)
      && !technicalConstructIds.has(row.target)
    ))).effects;
    if (!paths.length) return null;
    return {
      kind: "mediation",
      nodeIds: nativeDistinctOverlayIdsV1(paths.flatMap((path) => path.path)),
      relationIds: nativeDistinctOverlayIdsV1(paths.flatMap((path) => path.path.slice(0, -1).map((source, index) => (
        nativeResultRelationIdV1(run, source, path.path[index + 1]!)
      )))),
      interactionTermIds: [],
      label: paths.length === 1 ? "Indirect path" : `${paths.length} indirect paths`,
    };
  }
  return null;
}

function addAggregateMediationBootstrapTable(
  tables: ResultTable[],
  run: AnalysisRun,
  totalIndirectEffects: readonly { source: string; target: string }[],
  controlPairs: ReadonlySet<string>,
  constructLabel: ConstructDisplayLabel,
): Set<string> {
  if (!run.result || !run.bootstrap) return new Set();
  const directPairs = new Set(run.result.paths
    .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.coefficient) && !controlPairs.has(effectPairKey(row.source, row.target)))
    .map((row) => effectPairKey(row.source, row.target)));
  const indirectPairs = new Set(totalIndirectEffects.map((row) => effectPairKey(row.source, row.target)));
  const totalPairs = new Set(run.result.effects
    .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.total))
    .map((row) => effectPairKey(row.source, row.target)));
  const rows = run.bootstrap.percentile.parameters.flatMap((parameter) => {
    const identity = effectParameterIdentity(parameter.parameter);
    if (!identity || identity.parts.length !== 2) return [];
    if (![parameter.original, parameter.bootstrap_mean, parameter.standard_error, parameter.lower, parameter.upper].every(isFiniteNumber)) return [];
    const pair = effectPairKey(identity.parts[0], identity.parts[1]);
    const effectType = identity.kind === "direct_effect" && directPairs.has(pair)
      ? "Direct effect"
      : identity.kind === "indirect_effect" && indirectPairs.has(pair)
        ? "Total indirect effect (aggregate)"
        : identity.kind === "total_effect" && totalPairs.has(pair)
          ? "Total effect"
          : null;
    return effectType ? [{ effectType, identity, parameter }] : [];
  });
  if (!rows.length) return new Set();
  const hasTStatistic = rows.some((row) => isFiniteNumber(row.parameter.t_statistic));
  const hasPValue = rows.some((row) => isFiniteNumber(row.parameter.p_value_two_sided));
  addTable(tables, {
    id: "mediation_bootstrap",
    title: "Aggregate mediation effects bootstrap inference",
    warning: run.bootstrap.failed_replicates.length
      ? `${run.bootstrap.failed_replicates.length} bootstrap replicate(s) failed.`
      : null,
    columns: [
      "Effect type",
      "Effect",
      "Original sample (O)",
      "Sample mean (M)",
      "Standard deviation (STDEV)",
      ...(hasTStatistic ? ["T statistics (|O/STDEV|)"] : []),
      ...(hasPValue ? ["P values"] : []),
      "CI lower",
      "CI upper",
    ],
    rows: rows.map(({ effectType, identity, parameter }) => [
      effectType,
      constructPathLabel(identity.parts, constructLabel),
      formatNumber(parameter.original),
      formatNumber(parameter.bootstrap_mean),
      formatNumber(parameter.standard_error),
      ...(hasTStatistic ? [formatOptionalNumber(parameter.t_statistic, 3)] : []),
      ...(hasPValue ? [formatOptionalPValue(parameter.p_value_two_sided)] : []),
      formatNumber(parameter.lower),
      formatNumber(parameter.upper),
    ]),
  });
  return new Set(rows.map((row) => row.parameter.parameter));
}

type NativeModerationEstimate = NonNullable<NonNullable<AnalysisRun["result"]>["moderation"]>["estimates"][number];

function addModerationInferenceTables(
  tables: ResultTable[],
  run: AnalysisRun,
  estimates: readonly NativeModerationEstimate[],
  constructLabel: ConstructDisplayLabel,
): Set<string> {
  const estimatesByProductPath = new Map(estimates.map((estimate) => [
    effectPairKey(estimate.product_construct, estimate.outcome),
    estimate,
  ]));
  const match = (parameter: string) => {
    const identity = effectParameterIdentity(parameter);
    return identity?.kind === "path" && identity.parts.length === 2
      ? estimatesByProductPath.get(effectPairKey(identity.parts[0], identity.parts[1]))
      : undefined;
  };
  const matchedParameters = new Set<string>();
  const label = (estimate: NativeModerationEstimate) => `${constructLabel(estimate.predictor)} × ${constructLabel(estimate.moderator)} → ${constructLabel(estimate.outcome)}`;

  const percentileRows = (run.bootstrap?.percentile.parameters ?? []).flatMap((parameter) => {
    const identity = effectParameterIdentity(parameter.parameter);
    if (!identity || identity.kind !== "path" || identity.parts.length !== 2) return [];
    if (![parameter.original, parameter.bootstrap_mean, parameter.standard_error, parameter.lower, parameter.upper].every(isFiniteNumber)) return [];
    const estimate = match(parameter.parameter);
    return estimate ? [{ estimate, parameter }] : [];
  });
  if (percentileRows.length && run.bootstrap) {
    percentileRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    const hasTStatistic = percentileRows.some((row) => isFiniteNumber(row.parameter.t_statistic));
    const hasPValue = percentileRows.some((row) => isFiniteNumber(row.parameter.p_value_two_sided));
    addTable(tables, {
      id: "moderation_bootstrap",
      title: "Interaction effect bootstrap inference",
      warning: run.bootstrap.failed_replicates.length
        ? `${run.bootstrap.failed_replicates.length} bootstrap replicate(s) failed.`
        : null,
      columns: ["Interaction", "Original sample (O)", "Sample mean (M)", "Standard deviation (STDEV)", ...(hasTStatistic ? ["T statistics (|O/STDEV|)"] : []), ...(hasPValue ? ["P values"] : []), "CI lower", "CI upper"],
      rows: percentileRows.map(({ estimate, parameter }) => [
        label(estimate),
        formatNumber(parameter.original),
        formatNumber(parameter.bootstrap_mean),
        formatNumber(parameter.standard_error),
        ...(hasTStatistic ? [formatOptionalNumber(parameter.t_statistic, 3)] : []),
        ...(hasPValue ? [formatOptionalPValue(parameter.p_value_two_sided)] : []),
        formatNumber(parameter.lower),
        formatNumber(parameter.upper),
      ]),
    });
  }

  const bcaRows = (run.bootstrap?.bca?.parameters ?? []).flatMap((parameter) => {
    const estimate = match(parameter.parameter);
    return estimate && isFiniteNumber(parameter.lower) && isFiniteNumber(parameter.upper) ? [{ estimate, parameter }] : [];
  });
  if (bcaRows.length) {
    bcaRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    const hasBias = bcaRows.some((row) => isFiniteNumber(row.parameter.bias_correction));
    const hasAcceleration = bcaRows.some((row) => isFiniteNumber(row.parameter.acceleration));
    addTable(tables, {
      id: "moderation_bca",
      title: "Interaction effect BCa intervals",
      warning: null,
      columns: ["Interaction", ...(hasBias ? ["Bias correction"] : []), ...(hasAcceleration ? ["Acceleration"] : []), "CI lower", "CI upper"],
      rows: bcaRows.map(({ estimate, parameter }) => [label(estimate), ...(hasBias ? [formatOptionalNumber(parameter.bias_correction)] : []), ...(hasAcceleration ? [formatOptionalNumber(parameter.acceleration)] : []), formatOptionalNumber(parameter.lower), formatOptionalNumber(parameter.upper)]),
    });
  }

  const studentizedRows = (run.bootstrap?.studentized?.parameters ?? []).flatMap((parameter) => {
    const estimate = match(parameter.parameter);
    return estimate && [parameter.original, parameter.outer_standard_error, parameter.lower, parameter.upper].every(isFiniteNumber) ? [{ estimate, parameter }] : [];
  });
  if (studentizedRows.length) {
    studentizedRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    addTable(tables, {
      id: "moderation_studentized",
      title: "Interaction effect studentized intervals",
      warning: run.bootstrap?.studentized?.failure?.message ?? null,
      columns: ["Interaction", "Original", "Outer STDEV", "CI lower", "CI upper", "Usable replicates"],
      rows: studentizedRows.map(({ estimate, parameter }) => [label(estimate), formatNumber(parameter.original), formatNumber(parameter.outer_standard_error), formatOptionalNumber(parameter.lower), formatOptionalNumber(parameter.upper), String(parameter.usable_primary_replicates)]),
    });
  }

  const randomizationRows = (run.permutation?.parameters ?? []).flatMap((parameter) => {
    const estimate = match(parameter.parameter);
    return estimate && [parameter.original, parameter.p_value_two_sided, parameter.permutations].every(isFiniteNumber) ? [{ estimate, parameter }] : [];
  });
  if (randomizationRows.length) {
    randomizationRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    addTable(tables, {
      id: "moderation_randomization",
      title: "Interaction effect path randomization",
      status: "experimental",
      warning: null,
      columns: ["Interaction", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
      rows: randomizationRows.map(({ estimate, parameter }) => [
        label(estimate),
        formatNumber(parameter.original),
        String(parameter.exceedances),
        String(parameter.permutations),
        String(parameter.p_value_two_sided),
      ]),
    });
  }
  return matchedParameters;
}

type NativeControlEstimate = { source: string; target: string; label?: string | null; coefficient: number };

function addControlInferenceTables(
  tables: ResultTable[],
  run: AnalysisRun,
  controls: readonly NativeControlEstimate[],
  constructLabel: ConstructDisplayLabel,
): Set<string> {
  const controlsByPair = new Map(controls
    .filter((control) => hasText(control.source) && hasText(control.target))
    .map((control) => [effectPairKey(control.source, control.target), control]));
  const match = (parameter: string) => {
    const identity = effectParameterIdentity(parameter);
    return identity?.kind === "path" && identity.parts.length === 2
      ? controlsByPair.get(effectPairKey(identity.parts[0], identity.parts[1]))
      : undefined;
  };
  const label = (control: NativeControlEstimate) => control.label?.trim()
    ? `${control.label.trim()}: ${constructPathLabel([control.source, control.target], constructLabel)}`
    : constructPathLabel([control.source, control.target], constructLabel);
  const matchedParameters = new Set<string>();
  const percentileRows = (run.bootstrap?.percentile.parameters ?? []).flatMap((parameter) => {
    const control = match(parameter.parameter);
    return control && [parameter.original, parameter.bootstrap_mean, parameter.standard_error, parameter.lower, parameter.upper].every(isFiniteNumber) ? [{ control, parameter }] : [];
  });
  if (percentileRows.length) {
    percentileRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    addTable(tables, {
      id: "control_bootstrap",
      title: "Control effects bootstrap inference",
      warning: run.bootstrap?.failed_replicates.length ? `${run.bootstrap.failed_replicates.length} bootstrap replicate(s) failed.` : null,
      columns: ["Control", "Original", "Mean", "STDEV", "CI lower", "CI upper"],
      rows: percentileRows.map(({ control, parameter }) => [label(control), formatNumber(parameter.original), formatNumber(parameter.bootstrap_mean), formatNumber(parameter.standard_error), formatNumber(parameter.lower), formatNumber(parameter.upper)]),
    });
  }
  const bcaRows = (run.bootstrap?.bca?.parameters ?? []).flatMap((parameter) => {
    const control = match(parameter.parameter);
    return control && isFiniteNumber(parameter.lower) && isFiniteNumber(parameter.upper) ? [{ control, parameter }] : [];
  });
  if (bcaRows.length) {
    bcaRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    addTable(tables, {
      id: "control_bca",
      title: "Control effects BCa intervals",
      warning: null,
      columns: ["Control", "CI lower", "CI upper"],
      rows: bcaRows.map(({ control, parameter }) => [label(control), formatOptionalNumber(parameter.lower), formatOptionalNumber(parameter.upper)]),
    });
  }
  const studentizedRows = (run.bootstrap?.studentized?.parameters ?? []).flatMap((parameter) => {
    const control = match(parameter.parameter);
    return control && [parameter.original, parameter.outer_standard_error, parameter.lower, parameter.upper].every(isFiniteNumber) ? [{ control, parameter }] : [];
  });
  if (studentizedRows.length) {
    studentizedRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    addTable(tables, {
      id: "control_studentized",
      title: "Control effects studentized intervals",
      warning: run.bootstrap?.studentized?.failure?.message ?? null,
      columns: ["Control", "Original", "Outer STDEV", "CI lower", "CI upper", "Usable replicates"],
      rows: studentizedRows.map(({ control, parameter }) => [label(control), formatNumber(parameter.original), formatNumber(parameter.outer_standard_error), formatOptionalNumber(parameter.lower), formatOptionalNumber(parameter.upper), String(parameter.usable_primary_replicates)]),
    });
  }
  const randomizationRows = (run.permutation?.parameters ?? []).flatMap((parameter) => {
    const control = match(parameter.parameter);
    return control && [parameter.original, parameter.p_value_two_sided, parameter.permutations].every(isFiniteNumber) ? [{ control, parameter }] : [];
  });
  if (randomizationRows.length) {
    randomizationRows.forEach((row) => matchedParameters.add(row.parameter.parameter));
    addTable(tables, {
      id: "control_randomization",
      title: "Control effects path randomization",
      status: "experimental",
      warning: null,
      columns: ["Control", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
      rows: randomizationRows.map(({ control, parameter }) => [
        label(control),
        formatNumber(parameter.original),
        String(parameter.exceedances),
        String(parameter.permutations),
        String(parameter.p_value_two_sided),
      ]),
    });
  }
  return matchedParameters;
}

function effectPairKey(source: string, target: string): string {
  return `${source}\u0000${target}`;
}

function effectParameterIdentity(value: string): { kind: string; parts: string[] } | null {
  try {
    const parsed: unknown = JSON.parse(value);
    if (!Array.isArray(parsed) || typeof parsed[0] !== "string" || !Array.isArray(parsed[1])) return null;
    const parts = parsed[1];
    return parts.every((part): part is string => hasText(part)) ? { kind: parsed[0], parts } : null;
  } catch {
    return null;
  }
}

function addCurrentPredictionTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
  constructLabel: ConstructDisplayLabel,
) {
  const predict = run.result.predict;
  const repeated = predict?.repeated_kfold;
  if (
    predict?.method_version !== CURRENT_PLS_PREDICT_METHOD_VERSION
    || repeated?.method_version !== CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION
  ) return;

  addPredictionIndicatorTable(
    tables,
    "plspredict_indicator_summary",
    "Indicator prediction summary (10-fold × 10-repeat)",
    repeated.indicator_targets ?? [],
    constructLabel,
  );
  addCvpatBenchmarkTable(tables, repeated.cvpat_benchmark_assessments ?? []);
  addTable(tables, {
    id: "plspredict_validation_plan",
    title: "Prediction validation plan",
    warning: null,
    columns: ["Procedure", "Complete cases", "Folds", "Repeats", "Assignment", "Assignment digest", "Seed", "Test predictions"],
    rows: [[
      "Primary repeated cross-validation",
      String(run.result.used_observations),
      String(repeated.folds),
      String(repeated.repeats),
      predictionAssignmentLabel(repeated.assignment),
      repeated.assignment_digest ?? "",
      String(repeated.seed ?? run.provenance?.seed ?? run.seed),
      String(repeated.total_test_observations),
    ]],
  });
  addCurrentPredictTargetTable(
    tables,
    "plspredict_construct_summary",
    "Supplementary construct-score prediction (10-fold × 10-repeat)",
    repeated.targets,
    constructLabel,
  );
  addPredictionIndicatorTable(
    tables,
    "plspredict_holdout_indicator_summary",
    "Secondary holdout indicator summary",
    predict.indicator_targets ?? [],
    constructLabel,
  );
  addCurrentPredictTargetTable(
    tables,
    "plspredict_holdout_construct_summary",
    "Secondary holdout construct-score summary",
    predict.targets,
    constructLabel,
  );
  addTable(tables, {
    id: "plspredict_holdout_split",
    title: "Secondary deterministic holdout split",
    warning: null,
    columns: ["Procedure", "Training observations", "Test observations", "Assignment", "Benchmark"],
    rows: hasText(predict.split)
      && isFiniteNumber(predict.training_observations)
      && isFiniteNumber(predict.test_observations)
      && hasText(predict.benchmark)
      ? [[
          "Secondary holdout",
          String(predict.training_observations),
          String(predict.test_observations),
          predictionHoldoutLabel(predict.split),
          predictionBenchmarkLabel(predict.benchmark),
        ]]
      : [],
  });
}

function addPredictionIndicatorTable(
  tables: ResultTable[],
  id: string,
  title: string,
  targets: readonly PlsPredictIndicatorTarget[],
  constructLabel: ConstructDisplayLabel,
) {
  const rows = targets.filter(validPredictionIndicatorTarget);
  addTable(tables, {
    id,
    title,
    warning: null,
    columns: [
      "Construct",
      "Indicator",
      "Predictor set",
      "Predictors",
      "Observations",
      "Q²_predict",
      "PLS-SEM RMSE",
      "IA RMSE",
      "LM RMSE",
      "PLS-SEM MAE",
      "IA MAE",
      "LM MAE",
      "PLS-SEM MAPE (%)",
      "IA MAPE (%)",
      "LM MAPE (%)",
      "MAPE observations",
      "LM benchmark",
    ],
    rows: rows.map((row) => {
      const lm = row.linear_model.status === "available" ? row.linear_model.metrics ?? null : null;
      return [
        constructLabel(row.construct),
        row.indicator,
        predictionPredictorScopeLabel(row.predictor_scope),
        String(row.predictor_count),
        String(row.pls.observations),
        formatOptionalNumber(row.q_squared_predict),
        formatNumber(row.pls.rmse),
        formatNumber(row.indicator_average.rmse),
        formatOptionalNumber(lm?.rmse),
        formatNumber(row.pls.mae),
        formatNumber(row.indicator_average.mae),
        formatOptionalNumber(lm?.mae),
        formatOptionalNumber(row.pls.mape_percent),
        formatOptionalNumber(row.indicator_average.mape_percent),
        formatOptionalNumber(lm?.mape_percent),
        String(row.pls.mape_observations),
        linearModelAvailabilityLabel(row.linear_model.status, row.linear_model.reason),
      ];
    }),
  });
}

function validPredictionIndicatorTarget(row: PlsPredictIndicatorTarget): boolean {
  return hasText(row.construct)
    && hasText(row.indicator)
    && hasText(row.predictor_scope)
    && isFiniteNumber(row.predictor_count)
    && validPredictionMetrics(row.pls)
    && validPredictionMetrics(row.indicator_average)
    && (row.linear_model.status === "unavailable"
      || (row.linear_model.status === "available" && Boolean(row.linear_model.metrics) && validPredictionMetrics(row.linear_model.metrics!)));
}

function validPredictionMetrics(metrics: PlsPredictIndicatorTarget["pls"]): boolean {
  const validMape = metrics.mape_observations === 0
    ? metrics.absolute_percentage_error_sum === null && metrics.mape_percent === null
    : metrics.mape_observations > 0
      && isFiniteNumber(metrics.absolute_percentage_error_sum)
      && isFiniteNumber(metrics.mape_percent);
  return isFiniteNumber(metrics.observations)
    && isFiniteNumber(metrics.squared_error_sum)
    && isFiniteNumber(metrics.absolute_error_sum)
    && isFiniteNumber(metrics.rmse)
    && isFiniteNumber(metrics.mae)
    && isFiniteNumber(metrics.mape_observations)
    && validMape;
}

function validPredictionAssignmentDigest(value: string | undefined): value is string {
  return /^sha256:[0-9a-f]{64}$/.test(value ?? "");
}

function addCvpatBenchmarkTable(tables: ResultTable[], assessments: readonly CvpatBenchmarkAssessment[]) {
  const rows = assessments.filter((row) =>
    row.method_version === CURRENT_CVPAT_METHOD_VERSION
    && row.comparison_kind === "benchmark_assessment"
    && row.target_scope === "all_endogenous_indicators"
    && ["indicator_average", "linear_model"].includes(row.benchmark)
    && row.loss === "mean_squared_error_across_indicators_per_observation"
    && row.alternative === "pls_loss_less_than_benchmark"
    && row.confidence_level === 0.95
    && ["available", "inferential_test_unavailable", "benchmark_unavailable"].includes(row.status)
    && isFiniteNumber(row.observations)
    && isFiniteNumber(row.indicator_count),
  );
  addTable(tables, {
    id: "cvpat_benchmark_assessment",
    title: "CVPAT benchmark assessment (single model)",
    warning: null,
    columns: [
      "Benchmark",
      "Target set",
      "Loss",
      "Alternative",
      "Confidence",
      "PLS-SEM mean loss",
      "Benchmark mean loss",
      "Mean loss difference (PLS-SEM − benchmark)",
      "SE",
      "t",
      "p (one-sided)",
      "95% CI lower",
      "95% CI upper",
      "Complete cases",
      "Indicators",
      "Status",
      "Supported conclusion",
      "Reason",
    ],
    rows: rows.map((row) => [
      predictionBenchmarkLabel(row.benchmark),
      "All endogenous indicators",
      "Mean squared prediction loss per complete case",
      "PLS-SEM loss < benchmark",
      "95%",
      formatOptionalNumber(row.mean_loss_pls),
      formatOptionalNumber(row.mean_loss_benchmark),
      formatOptionalNumber(row.mean_loss_difference),
      formatOptionalNumber(row.standard_error),
      formatOptionalNumber(row.t_statistic, 3),
      formatOptionalPValue(row.p_value_one_sided),
      formatOptionalNumber(row.confidence_interval_lower),
      formatOptionalNumber(row.confidence_interval_upper),
      String(row.observations),
      String(row.indicator_count),
      cvpatStatusLabel(row.status),
      cvpatConclusionLabel(row),
      readablePredictionText(row.reason),
    ]),
  });
}

function addCurrentPredictTargetTable(
  tables: ResultTable[],
  id: string,
  title: string,
  targets: readonly PlsPredictTarget[],
  constructLabel: ConstructDisplayLabel,
) {
  const rows = targets.filter((row) =>
    hasText(row.construct)
    && isFiniteNumber(row.predictor_count)
    && isFiniteNumber(row.rmse_pls)
    && isFiniteNumber(row.mae_pls)
    && isFiniteNumber(row.rmse_benchmark)
    && isFiniteNumber(row.mae_benchmark),
  );
  addTable(tables, {
    id,
    title,
    warning: null,
    columns: [
      "Construct",
      "Predictors",
      "Q²_predict",
      "PLS-SEM RMSE",
      "Mean benchmark RMSE",
      "LM RMSE",
      "PLS-SEM MAE",
      "Mean benchmark MAE",
      "LM MAE",
    ],
    rows: rows.map((row) => [
      constructLabel(row.construct),
      String(row.predictor_count),
      formatOptionalNumber(row.q_squared_predict),
      formatNumber(row.rmse_pls),
      formatNumber(row.rmse_benchmark),
      formatOptionalNumber(row.rmse_lm),
      formatNumber(row.mae_pls),
      formatNumber(row.mae_benchmark),
      formatOptionalNumber(row.mae_lm),
    ]),
  });
}

function addLegacyPredictionTables(
  tables: ResultTable[],
  predict: NonNullable<NonNullable<AnalysisRun["result"]>["predict"]>,
  constructLabel: ConstructDisplayLabel,
) {
  const legacyWarning = "Legacy v1 construct-score output; this is not current indicator-level PLSpredict or CVPAT.";
  addLegacyPredictTargetTable(tables, "plspredict_holdout", "Legacy construct-score holdout metrics (v1)", predict.targets, constructLabel, legacyWarning);
  addTable(tables, {
    id: "plspredict_split",
    title: "Legacy construct-score holdout split (v1)",
    warning: legacyWarning,
    columns: ["Assignment", "Training observations", "Test observations", "Benchmark"],
    rows: hasText(predict.split)
      && isFiniteNumber(predict.training_observations)
      && isFiniteNumber(predict.test_observations)
      && hasText(predict.benchmark)
      ? [[predictionHoldoutLabel(predict.split), String(predict.training_observations), String(predict.test_observations), predictionBenchmarkLabel(predict.benchmark)]]
      : [],
  });

  const repeated = predict.repeated_kfold;
  if (repeated?.method_version !== LEGACY_PLS_PREDICT_REPEATED_METHOD_VERSION) return;
  addLegacyPredictTargetTable(tables, "plspredict_repeated_kfold", "Legacy construct-score repeated-fold metrics (v1)", repeated.targets, constructLabel, legacyWarning);
  addTable(tables, {
    id: "plspredict_repeated_kfold_plan",
    title: "Legacy construct-score repeated-fold plan (v1)",
    warning: legacyWarning,
    columns: ["Folds", "Repeats", "Test observations", "Assignment"],
    rows: isFiniteNumber(repeated.folds)
      && isFiniteNumber(repeated.repeats)
      && isFiniteNumber(repeated.total_test_observations)
      && hasText(repeated.assignment)
      ? [[String(repeated.folds), String(repeated.repeats), String(repeated.total_test_observations), readablePredictionText(repeated.assignment)]]
      : [],
  });
  const pairedRows = (repeated.cvpat ?? []).filter((row) =>
    hasText(row.target)
    && hasText(row.comparison)
    && hasText(row.loss)
    && isFiniteNumber(row.mean_loss_difference)
    && isFiniteNumber(row.observations)
    && hasText(row.preferred_model),
  );
  addTable(tables, {
    id: "cvpat",
    title: "Legacy paired loss diagnostics (v1)",
    warning: legacyWarning,
    columns: ["Target", "Comparison", "Loss", "Mean loss difference", "SE", "t", "p (two-sided)", "Observations", "Lower sample loss", "Warning"],
    rows: pairedRows.map((row) => [
      constructLabel(row.target),
      legacyPredictionComparisonLabel(row.comparison),
      "Squared error difference (PLS − comparison)",
      formatNumber(row.mean_loss_difference),
      formatOptionalNumber(row.standard_error),
      formatOptionalNumber(row.t_statistic, 3),
      formatOptionalPValue(row.p_value_two_sided),
      String(row.observations),
      legacyPreferredModelLabel(row.preferred_model),
      readablePredictionText(row.warning),
    ]),
  });
}

function addLegacyPredictTargetTable(
  tables: ResultTable[],
  id: string,
  title: string,
  targets: readonly PlsPredictTarget[],
  constructLabel: ConstructDisplayLabel,
  warning: string,
) {
  const rows = targets.filter((row) =>
    hasText(row.construct)
    && isFiniteNumber(row.predictor_count)
    && isFiniteNumber(row.rmse_pls)
    && isFiniteNumber(row.mae_pls)
    && isFiniteNumber(row.rmse_benchmark)
    && isFiniteNumber(row.mae_benchmark),
  );
  addTable(tables, {
    id,
    title,
    warning,
    columns: ["Construct", "Predictors", "PLS RMSE", "Benchmark RMSE", "LM RMSE", "PLS MAE", "Benchmark MAE", "LM MAE", "Q²_predict"],
    rows: rows.map((row) => [
      constructLabel(row.construct),
      String(row.predictor_count),
      formatNumber(row.rmse_pls),
      formatNumber(row.rmse_benchmark),
      formatOptionalNumber(row.rmse_lm),
      formatNumber(row.mae_pls),
      formatNumber(row.mae_benchmark),
      formatOptionalNumber(row.mae_lm),
      formatOptionalNumber(row.q_squared_predict),
    ]),
  });
}

function predictionPredictorScopeLabel(scope: string): string {
  return scope === "earliest_antecedent_indicators" ? "Earliest antecedent indicators" : readablePredictionText(scope);
}

function predictionBenchmarkLabel(benchmark: string): string {
  if (["indicator_average", "indicator_mean", "training_mean", "pls_vs_training_mean_benchmark"].includes(benchmark)) return "Indicator average (IA)";
  if (["linear_model", "lm", "lm_benchmark", "pls_vs_lm_benchmark"].includes(benchmark)) return "Linear model (LM)";
  return readablePredictionText(benchmark);
}

function predictionAssignmentLabel(assignment: string): string {
  return hasText(assignment) ? "Seeded balanced fold assignment" : "";
}

function predictionHoldoutLabel(split: string): string {
  return split === "deterministic_complete_case_modulo_4_test_rows"
    ? "Deterministic complete-case modulo-4 holdout"
    : readablePredictionText(split);
}

function linearModelAvailabilityLabel(status: "available" | "unavailable", reason: string | null | undefined): string {
  return status === "available" ? "Available" : `Unavailable${hasText(reason) ? ` — ${readablePredictionText(reason)}` : ""}`;
}

function cvpatStatusLabel(status: CvpatBenchmarkAssessment["status"]): string {
  if (status === "available") return "Available";
  if (status === "benchmark_unavailable") return "Benchmark unavailable";
  return "Inferential test unavailable";
}

function cvpatConclusionLabel(row: CvpatBenchmarkAssessment): string {
  if (row.status !== "available") return "";
  return row.preferred_model === "pls_sem"
    ? "PLS-SEM significantly better"
    : "No statistically supported advantage";
}

function legacyPredictionComparisonLabel(comparison: string): string {
  if (comparison === "pls_vs_training_mean_benchmark") return "PLS vs training-mean benchmark";
  if (comparison === "pls_vs_lm_benchmark") return "PLS vs linear-model benchmark";
  if (comparison.startsWith("pls_vs_model_pair:")) return "Within-run reduced-path diagnostic";
  return readablePredictionText(comparison);
}

function legacyPreferredModelLabel(model: string): string {
  if (model === "pls") return "PLS construct-score model";
  if (["lm", "lm_benchmark"].includes(model)) return "Linear-model benchmark";
  if (["training_mean", "training_mean_benchmark"].includes(model)) return "Training-mean benchmark";
  return readablePredictionText(model);
}

function readablePredictionText(value: string | null | undefined): string {
  if (!hasText(value)) return "";
  const text = value.trim().replaceAll("_", " ").replace(/\s+/g, " ");
  return text.charAt(0).toLocaleUpperCase() + text.slice(1);
}

function isNativeCbsemBootstrapV2(
  analysis: CbsemAnalysis,
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
): boolean {
  const bootstrap = analysis.bootstrap_v2;
  if (!bootstrap) return false;
  const requested = bootstrap.requested_replicates;
  const required = Math.max(1_000, Math.ceil(0.9 * requested));
  const available = bootstrap.usable_replicates >= required;
  const freeParameters = analysis.parameters.filter((parameter) => !parameter.fixed);
  const parameterNames = freeParameters.map((parameter) => parameter.name);
  const sha256 = /^[0-9a-f]{64}$/;
  const successes = bootstrap.validation_witness.successful_replicates;
  const failures = bootstrap.failures;
  const indices = [...successes.map((row) => row.replicate_index), ...failures.map((row) => row.replicate_index)]
    .sort((left, right) => left - right);
  const fullIndexSet = indices.length === requested
    && indices.every((value, index) => value === index);
  const successfulRowsValid = successes.every((row) => (
    Number.isInteger(row.replicate_index)
    && row.replicate_index >= 0
    && row.replicate_index < requested
    && sha256.test(row.sample_indices_sha256)
    && Number.isInteger(row.iterations)
    && row.iterations > 0
    && isFiniteNumber(row.objective)
    && row.objective >= 0
    && row.parameter_estimates.length === parameterNames.length
    && row.parameter_estimates.every(isFiniteNumber)
  ));
  const failureRowsValid = failures.every((row) => (
    Number.isInteger(row.replicate_index)
    && row.replicate_index >= 0
    && row.replicate_index < requested
    && sha256.test(row.sample_indices_sha256)
    && hasText(row.reason_code)
    && hasText(row.message)
  ));
  const intervalsValid = bootstrap.intervals.every((row, index) => (
    row.parameter === parameterNames[index]
    && row.usable_replicates === bootstrap.usable_replicates
    && [row.original, row.bootstrap_mean, row.bias, row.standard_error, row.percentile_lower, row.percentile_upper].every(isFiniteNumber)
    && row.standard_error >= 0
    && row.percentile_lower <= row.percentile_upper
  ));
  return bootstrap.method_version === CBSEM_BOOTSTRAP_METHOD_VERSION_V2
    && bootstrap.algorithm === "indexed_raw_case_refit_ml_v2"
    && bootstrap.interval_method === "percentile_type7_v1"
    && bootstrap.retry_policy === "no_retry_fixed_preplanned_primary_draws_v1"
    && isFiniteNumber(bootstrap.confidence_level)
    && bootstrap.confidence_level === 0.95
    && bootstrap.confidence_level === run.provenance!.settings.confidence_level
    && Number.isInteger(requested)
    && requested >= 500
    && requested <= 10_000
    && bootstrap.attempted_fits === requested
    && bootstrap.usable_replicates === successes.length
    && bootstrap.failed_replicates === failures.length
    && bootstrap.usable_replicates + bootstrap.failed_replicates === requested
    && bootstrap.minimum_usable_fraction === 0.9
    && bootstrap.minimum_usable_replicates === required
    && bootstrap.max_attempts_per_replicate === 1
    && bootstrap.complete_case_sample_size === analysis.sample_size
    && bootstrap.seed === run.provenance!.seed
    && bootstrap.stream_token === "quickpls_cbsem_ml_case_bootstrap_v2"
    && bootstrap.validation_witness.method_version === "cbsem_bootstrap_validation_witness_v2"
    && bootstrap.validation_witness.dataset_fingerprint === run.provenance!.dataset_fingerprint
    && sha256.test(bootstrap.validation_witness.recipe_sha256)
    && sha256.test(bootstrap.validation_witness.base_result_sha256)
    && JSON.stringify(bootstrap.validation_witness.parameter_names) === JSON.stringify(parameterNames)
    && fullIndexSet
    && successfulRowsValid
    && failureRowsValid
    && bootstrap.warnings.every(hasText)
    && (available
      ? bootstrap.inference.status === "available"
        && bootstrap.intervals.length === parameterNames.length
        && intervalsValid
      : bootstrap.inference.status === "unavailable"
        && bootstrap.inference.reason_code === "insufficient_usable_replicates"
        && hasText(bootstrap.inference.message)
        && bootstrap.intervals.length === 0);
}

function nativeCbsemType7(sorted: readonly number[], probability: number): number {
  const position = probability * (sorted.length - 1);
  const lower = Math.floor(position);
  const upper = Math.ceil(position);
  return lower === upper
    ? sorted[lower]
    : sorted[lower] + (position - lower) * (sorted[upper] - sorted[lower]);
}

function isNativeCbsemExactCaseBootstrapV1(
  bootstrap: NonNullable<CbsemAnalysis["exact_case_bootstrap"]>,
  analysis: CbsemAnalysis,
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
  requireHypothesisTests = false,
): boolean {
  const sha256 = /^[0-9a-f]{64}$/;
  const requested = bootstrap.requested_replicates;
  const minimum = Math.max(1_000, Math.ceil(0.9 * requested));
  const parameterIds = bootstrap.parameter_ids;
  if (!parameterIds.length || new Set(parameterIds).size !== parameterIds.length || parameterIds.some((id) => !hasText(id))) return false;
  const successes = bootstrap.successful_refits;
  const failures = bootstrap.failed_refits;
  const indices = [...successes.map((row) => row.replicate_index), ...failures.map((row) => row.replicate_index)].sort((left, right) => left - right);
  const successRowsValid = successes.every((row, index) => (
    Number.isSafeInteger(row.replicate_index) && row.replicate_index >= 0 && row.replicate_index < requested
    && (index === 0 || successes[index - 1].replicate_index < row.replicate_index)
    && sha256.test(row.sampling_positions_sha256) && sha256.test(row.sample_indices_sha256)
    && row.parameter_estimates.length === parameterIds.length
    && row.parameter_estimates.every((value) => isFiniteNumber(value) && !Object.is(value, -0))
    && Number.isSafeInteger(row.iterations) && row.iterations > 0
    && isFiniteNumber(row.objective) && row.objective >= 0 && !Object.is(row.objective, -0)
    && isFiniteNumber(row.gradient_norm) && row.gradient_norm >= 0 && !Object.is(row.gradient_norm, -0)
  ));
  const failureKinds = new Set([
    "moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure",
  ]);
  const failureRowsValid = failures.every((row, index) => (
    Number.isSafeInteger(row.replicate_index) && row.replicate_index >= 0 && row.replicate_index < requested
    && (index === 0 || failures[index - 1].replicate_index < row.replicate_index)
    && sha256.test(row.sampling_positions_sha256) && sha256.test(row.sample_indices_sha256)
    && failureKinds.has(row.kind) && hasText(row.message)
  ));
  const available = successes.length >= minimum;
  const intervalsValid = bootstrap.intervals.every((row, parameterIndex) => {
    if (row.parameter_id !== parameterIds[parameterIndex] || row.usable_replicates !== successes.length) return false;
    const values = successes.map((success) => success.parameter_estimates[parameterIndex]);
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
    const standardError = Math.sqrt(values.reduce((sum, value) => {
      const difference = value - mean;
      return sum + difference * difference;
    }, 0) / (values.length - 1));
    const sorted = [...values].sort((left, right) => left - right);
    const observed = [row.bootstrap_mean, row.bias, row.standard_error, row.percentile_lower, row.percentile_upper];
    const expected = [
      mean,
      mean - row.original,
      standardError,
      nativeCbsemType7(sorted, 0.025000000000000022),
      nativeCbsemType7(sorted, 0.975),
    ];
    return !Object.is(row.original, -0) && observed.every((value, index) => isFiniteNumber(value) && !Object.is(value, -0) && Object.is(value, expected[index]));
  });
  const hypothesisTestsValid = isNativeCbsemExactCaseBootstrapHypothesisTestsV1(bootstrap);
  return bootstrap.method_version === CBSEM_EXACT_BOOTSTRAP_METHOD_VERSION_V1
    && bootstrap.estimator_method_version === "cbsem_ml_exact_parameter_table_v3"
    && bootstrap.source_dataset_fingerprint === run.provenance!.dataset_fingerprint
    && [bootstrap.outer_recipe_analytical_identity_sha256, bootstrap.base_point_result_sha256,
      bootstrap.compiler_analytical_identity_sha256, bootstrap.plan_sha256,
      bootstrap.model_scientific_sha256, bootstrap.complete_case_universe_sha256].every((value) => sha256.test(value))
    && bootstrap.complete_case_sample_size === analysis.sample_size
    && bootstrap.complete_case_universe_digest_method === "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1"
    && bootstrap.covariance_denominator === "maximum_likelihood_n"
    && bootstrap.sample_indices_digest_method === "sha256_source_fingerprint_and_ordered_u64_indices_v1"
    && bootstrap.sampling_positions_digest_method === "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1"
    && bootstrap.interval_method === "percentile_type7_v1" && bootstrap.confidence_level === 0.95
    && Number.isSafeInteger(requested) && requested >= 500 && requested <= 10_000
    && bootstrap.attempted_refits === requested && bootstrap.usable_replicates === successes.length
    && bootstrap.failed_replicates === failures.length && successes.length + failures.length === requested
    && bootstrap.minimum_usable_fraction === 0.9 && bootstrap.minimum_usable_replicates === minimum
    && bootstrap.seed === run.provenance!.seed && bootstrap.stream_token === "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1"
    && bootstrap.retry_policy === "no_retry_fixed_preplanned_primary_draws_v1" && bootstrap.max_attempts_per_replicate === 1
    && indices.length === requested && indices.every((value, index) => value === index)
    && successRowsValid && failureRowsValid && hypothesisTestsValid
    && (!requireHypothesisTests || bootstrap.hypothesis_tests != null)
    && (available
      ? bootstrap.inference.status === "available" && bootstrap.intervals.length === parameterIds.length && intervalsValid
      : bootstrap.inference.status === "unavailable" && bootstrap.inference.reason_code === "insufficient_usable_refits"
        && hasText(bootstrap.inference.message) && bootstrap.intervals.length === 0);
}

function isNativeCbsemExactCaseBootstrapHypothesisTestsV1(
  bootstrap: NonNullable<CbsemAnalysis["exact_case_bootstrap"]>,
): boolean {
  const receipt = bootstrap.hypothesis_tests;
  if (!receipt) return true; // Historical adapter-v9 results remain readable.
  const usable = bootstrap.successful_refits.length;
  const globallyAvailable = usable >= bootstrap.minimum_usable_replicates;
  const selectedTail = receipt.selected_test_tail;
  const tails = new Set(["two_sided", "one_sided_greater", "one_sided_less"]);
  if (receipt.method_version !== CBSEM_EXACT_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1
    || receipt.null_hypothesis !== "compiled_free_parameter_equals_zero_v1"
    || receipt.statistic !== "unstudentized_null_centered_parameter_estimate_v1"
    || receipt.tie_policy !== "inclusive_ieee_comparison_v1"
    || receipt.probability_method !== "plus_one_over_usable_plus_one_v1"
    || receipt.decision_rule !== "selected_p_value_less_than_or_equal_alpha_v1"
    || !tails.has(selectedTail)
    || !Object.is(receipt.null_value, 0)
    || receipt.significance_level !== 0.05
    || receipt.usable_replicates !== usable
    || receipt.parameters.length !== bootstrap.parameter_ids.length
    || (globallyAvailable
      ? receipt.inference.status !== "available"
      : receipt.inference.status !== "unavailable"
        || receipt.inference.reason_code !== "insufficient_usable_refits"
        || !hasText(receipt.inference.message))) return false;

  const unavailableReasons = new Set([
    "insufficient_usable_replicates", "nonregular_variance_boundary",
    "zero_null_outside_open_domain", "unsupported_parameter_family",
  ]);
  return receipt.parameters.every((parameter, parameterIndex) => {
    if (parameter.parameter_id !== bootstrap.parameter_ids[parameterIndex]) return false;
    const outcome = parameter.outcome;
    if (outcome.status === "unavailable") {
      return unavailableReasons.has(outcome.reason)
        && (!globallyAvailable || outcome.reason !== "insufficient_usable_replicates");
    }
    if (!globallyAvailable) return false;
    const interval = bootstrap.intervals[parameterIndex];
    if (!interval || !Object.is(outcome.point_estimate, interval.original)
      || !isFiniteNumber(outcome.point_estimate) || Object.is(outcome.point_estimate, -0)) return false;
    const deltas = bootstrap.successful_refits.map((refit) => (
      refit.parameter_estimates[parameterIndex] - outcome.point_estimate
    ));
    const expectedCounts = [
      deltas.filter((delta) => Math.abs(delta) >= Math.abs(outcome.point_estimate)).length,
      deltas.filter((delta) => delta >= outcome.point_estimate).length,
      deltas.filter((delta) => delta <= outcome.point_estimate).length,
    ];
    const expectedProbabilities = expectedCounts.map((count) => (count + 1) / (usable + 1));
    const observedCounts = [
      outcome.two_sided_exceedances, outcome.greater_or_equal_exceedances,
      outcome.less_or_equal_exceedances,
    ];
    const observedProbabilities = [outcome.p_value_two_sided, outcome.p_value_greater, outcome.p_value_less];
    const selectedIndex = selectedTail === "two_sided" ? 0 : selectedTail === "one_sided_greater" ? 1 : 2;
    return observedCounts.every((count, index) => Number.isSafeInteger(count) && count >= 0
        && count <= usable && count === expectedCounts[index])
      && observedProbabilities.every((probability, index) => isFiniteNumber(probability)
        && !Object.is(probability, -0) && Object.is(probability, expectedProbabilities[index]))
      && outcome.selected_exceedances === expectedCounts[selectedIndex]
      && Object.is(outcome.selected_p_value, expectedProbabilities[selectedIndex])
      && outcome.reject_null === (outcome.selected_p_value <= 0.05);
  });
}

function isNativeCbsemExactCaseBootstrapStudentizedV1(
  wrapper: NonNullable<CbsemAnalysis["exact_case_bootstrap_studentized"]>,
  analysis: CbsemAnalysis,
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
): boolean {
  if (!wrapper?.base || !wrapper.studentized
    || Object.keys(wrapper).length !== 2
    || !("base" in wrapper) || !("studentized" in wrapper)) return false;
  const base = wrapper.base;
  const sidecar = wrapper.studentized;
  if (!isNativeCbsemExactCaseBootstrapV1(base, analysis, run, true)) return false;
  const modeledVariables = new Set(analysis.residual_correlation.flatMap((cell) => [cell.row, cell.column]));
  if (!Number.isSafeInteger(run.provenance!.settings.workers)
    || run.provenance!.settings.workers < 1 || run.provenance!.settings.workers > 12
    || analysis.sample_size > 180 || modeledVariables.size > 9
    || base.parameter_ids.length > 18) return false;
  if (sidecar.method_version !== "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1"
    || sidecar.standard_error_method_version !== "cbsem_exact_case_bootstrap_refit_standard_errors_v1"
    || sidecar.expected_information_method !== "cbsem_ml_expected_information_delta_method_v1"
    || sidecar.pivot_method !== "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1"
    || sidecar.quantile_method !== "percentile_type7_v1"
    || sidecar.interval_method !== "reversed_type7_studentized_pivot_v1"
    || sidecar.archive_validation_scope !== "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1"
    || sidecar.confidence_level !== base.confidence_level
    || sidecar.minimum_usable_fraction !== base.minimum_usable_fraction
    || sidecar.minimum_usable_replicates !== base.minimum_usable_replicates
    || JSON.stringify(sidecar.parameter_ids) !== JSON.stringify(base.parameter_ids)) return false;

  const standardErrorReasons = new Set([
    "singular_information", "information_not_positive_definite",
    "invalid_information_variance_or_standard_error", "derivative_unavailable",
    "numerical_information_failure",
  ]);
  const pointReceipt = sidecar.point_standard_errors;
  if (pointReceipt.method_version !== "cbsem_exact_case_bootstrap_refit_standard_errors_v1") return false;
  let pointStandardErrors: number[] | null = null;
  if (pointReceipt.outcome.status === "available") {
    if (pointReceipt.outcome.information_method !== "cbsem_ml_expected_information_delta_method_v1"
      || pointReceipt.outcome.parameters.length !== base.parameter_ids.length) return false;
    pointStandardErrors = [];
    for (const [index, parameter] of pointReceipt.outcome.parameters.entries()) {
      if (parameter.parameter_id !== base.parameter_ids[index]
        || !isFiniteNumber(parameter.standard_error) || parameter.standard_error <= 0
        || Object.is(parameter.standard_error, -0)) return false;
      pointStandardErrors.push(parameter.standard_error);
    }
  } else if (!standardErrorReasons.has(pointReceipt.outcome.reason)) return false;

  if (!Array.isArray(sidecar.refit_standard_errors)
    || sidecar.refit_standard_errors.length !== base.successful_refits.length) return false;
  const usableRefits: Array<{ estimates: readonly number[]; standardErrors: readonly number[] }> = [];
  for (const [index, receipt] of sidecar.refit_standard_errors.entries()) {
    const baseRefit = base.successful_refits[index];
    if (!Number.isSafeInteger(receipt.replicate_index)
      || receipt.replicate_index !== baseRefit.replicate_index) return false;
    if (receipt.outcome.status === "available") {
      if (receipt.outcome.information_method !== "cbsem_ml_expected_information_delta_method_v1"
        || receipt.outcome.standard_errors.length !== base.parameter_ids.length
        || receipt.outcome.standard_errors.some((value) => (
          !isFiniteNumber(value) || value <= 0 || Object.is(value, -0)
        ))) return false;
      usableRefits.push({
        estimates: baseRefit.parameter_estimates,
        standardErrors: receipt.outcome.standard_errors,
      });
    } else if (!standardErrorReasons.has(receipt.outcome.reason)) return false;
  }
  if (!Number.isSafeInteger(sidecar.studentized_usable_replicates)
    || sidecar.studentized_usable_replicates !== usableRefits.length) return false;
  const unavailableReason = pointStandardErrors === null
    ? "point_standard_errors_unavailable"
    : usableRefits.length < base.minimum_usable_replicates
      ? "insufficient_studentized_usable_replicates"
      : null;
  if (unavailableReason === null) {
    if (sidecar.inference.status !== "available") return false;
  } else {
    const expectedMessage = unavailableReason === "point_standard_errors_unavailable"
      ? "Analytically studentized inference is unavailable because the point estimate has no whole-vector analytical standard-error receipt."
      : `Analytically studentized inference is unavailable because ${usableRefits.length} whole-vector usable refits are below the required ${base.minimum_usable_replicates}.`;
    if (sidecar.inference.status !== "unavailable"
      || sidecar.inference.reason !== unavailableReason
      || sidecar.inference.message !== expectedMessage) return false;
  }
  if (!Array.isArray(sidecar.intervals) || sidecar.intervals.length !== base.parameter_ids.length) return false;
  const lowerProbability = (1 - base.confidence_level) / 2;
  const upperProbability = 1 - lowerProbability;
  return sidecar.intervals.every((interval, parameterIndex) => {
    if (interval.parameter_id !== base.parameter_ids[parameterIndex]) return false;
    const outcome = interval.outcome;
    if (unavailableReason !== null) {
      return outcome.status === "unavailable" && outcome.reason === unavailableReason;
    }
    if (outcome.status !== "available" || pointStandardErrors === null) return false;
    const baseInterval = base.intervals[parameterIndex];
    if (!baseInterval) return false;
    const pivots = usableRefits.map((refit) => (
      (refit.estimates[parameterIndex] - baseInterval.original) / refit.standardErrors[parameterIndex]
    ));
    if (pivots.some((pivot) => !isFiniteNumber(pivot))) return false;
    pivots.sort((left, right) => left - right);
    const lowerPivot = nativeCbsemType7(pivots, lowerProbability);
    const upperPivot = nativeCbsemType7(pivots, upperProbability);
    const expected = [
      baseInterval.original,
      pointStandardErrors[parameterIndex],
      lowerPivot,
      upperPivot,
      baseInterval.original - upperPivot * pointStandardErrors[parameterIndex],
      baseInterval.original - lowerPivot * pointStandardErrors[parameterIndex],
    ];
    const observed = [
      outcome.point_estimate,
      outcome.point_standard_error,
      outcome.lower_pivot_quantile,
      outcome.upper_pivot_quantile,
      outcome.interval_lower,
      outcome.interval_upper,
    ];
    return outcome.usable_replicates === usableRefits.length
      && observed.every((value, index) => (
        isFiniteNumber(value) && !Object.is(value, -0) && Object.is(value, expected[index])
      ))
      && outcome.interval_lower <= outcome.interval_upper;
  });
}

function isNativeCbsemExactCaseBootstrapBcaV1(
  wrapper: NonNullable<CbsemAnalysis["exact_case_bootstrap_bca"]>,
  analysis: CbsemAnalysis,
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
): boolean {
  if (!wrapper?.base || !wrapper.bca || Object.keys(wrapper).length !== 2
    || !("base" in wrapper) || !("bca" in wrapper)) return false;
  const base = wrapper.base;
  const sidecar = wrapper.bca;
  if (!isNativeCbsemExactCaseBootstrapV1(base, analysis, run, true)) return false;
  const modeledVariables = new Set(analysis.residual_correlation.flatMap((cell) => [cell.row, cell.column]));
  if (!Number.isSafeInteger(run.provenance!.settings.workers)
    || run.provenance!.settings.workers < 1 || run.provenance!.settings.workers > 12
    || analysis.sample_size > 180 || modeledVariables.size > 9 || base.parameter_ids.length > 18) return false;
  if (sidecar.method_version !== "cbsem_exact_case_bootstrap_bca_interval_v1"
    || sidecar.base_bootstrap_method_version !== base.method_version
    || sidecar.outer_recipe_analytical_identity_sha256 !== base.outer_recipe_analytical_identity_sha256
    || sidecar.base_point_result_sha256 !== base.base_point_result_sha256
    || sidecar.compiler_analytical_identity_sha256 !== base.compiler_analytical_identity_sha256
    || sidecar.plan_sha256 !== base.plan_sha256
    || sidecar.model_scientific_sha256 !== base.model_scientific_sha256
    || sidecar.delete_one_refit_method_version !== "cbsem_exact_case_bootstrap_delete_one_refit_v1"
    || sidecar.bias_correction_method !== "midrank_less_plus_half_ties_no_clamp_v1"
    || sidecar.acceleration_method !== "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2"
    || sidecar.adjusted_probability_method !== "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2"
    || sidecar.quantile_method !== "percentile_type7_v1"
    || sidecar.retry_policy !== "no_retry_exactly_one_fit_per_omitted_case_v1"
    || sidecar.confidence_level !== base.confidence_level
    || sidecar.bootstrap_usable_replicates !== base.usable_replicates
    || sidecar.minimum_bootstrap_usable_replicates !== base.minimum_usable_replicates
    || sidecar.delete_one_case_count !== base.complete_case_sample_size
    || JSON.stringify(sidecar.parameter_ids) !== JSON.stringify(base.parameter_ids)) return false;
  const sha256 = /^[0-9a-f]{64}$/;
  const failureKinds = new Set([
    "moment_matrix_not_positive_definite", "non_convergence", "inadmissible_solution", "numerical_failure",
  ]);
  const successes = sidecar.successful_delete_one_refits;
  const failures = sidecar.failed_delete_one_refits;
  const successfulRowsValid = successes.every((row, index) => (
    Number.isSafeInteger(row.omitted_complete_case_position)
    && row.omitted_complete_case_position >= 0 && row.omitted_complete_case_position < sidecar.delete_one_case_count
    && (index === 0 || successes[index - 1].omitted_complete_case_position < row.omitted_complete_case_position)
    && Number.isSafeInteger(row.omitted_source_row_index) && row.omitted_source_row_index >= 0
    && sha256.test(row.retained_sampling_positions_sha256) && sha256.test(row.retained_sample_indices_sha256)
    && row.parameter_estimates.length === base.parameter_ids.length
    && row.parameter_estimates.every((value) => isFiniteNumber(value) && !Object.is(value, -0))
    && Number.isSafeInteger(row.iterations) && row.iterations > 0
    && isFiniteNumber(row.objective) && row.objective >= 0 && !Object.is(row.objective, -0)
    && isFiniteNumber(row.gradient_norm) && row.gradient_norm >= 0 && !Object.is(row.gradient_norm, -0)
  ));
  const failureRowsValid = failures.every((row, index) => (
    Number.isSafeInteger(row.omitted_complete_case_position)
    && row.omitted_complete_case_position >= 0 && row.omitted_complete_case_position < sidecar.delete_one_case_count
    && (index === 0 || failures[index - 1].omitted_complete_case_position < row.omitted_complete_case_position)
    && Number.isSafeInteger(row.omitted_source_row_index) && row.omitted_source_row_index >= 0
    && sha256.test(row.retained_sampling_positions_sha256) && sha256.test(row.retained_sample_indices_sha256)
    && failureKinds.has(row.kind) && hasText(row.message)
  ));
  const omissions = [...successes, ...failures]
    .map((row) => ({ position: row.omitted_complete_case_position, sourceRow: row.omitted_source_row_index }))
    .sort((left, right) => left.position - right.position);
  if (!successfulRowsValid || !failureRowsValid || omissions.length !== sidecar.delete_one_case_count
    || omissions.some((row, index) => row.position !== index)
    || omissions.some((row, index) => index > 0 && omissions[index - 1].sourceRow >= row.sourceRow)) return false;
  const globalReason = base.inference.status !== "available"
    ? "base_inference_unavailable"
    : failures.length > 0 || successes.length !== sidecar.delete_one_case_count
      ? "incomplete_delete_one_ledger"
      : null;
  if (globalReason === null) {
    if (sidecar.inference.status !== "available") return false;
  } else {
    const expectedMessage = globalReason === "base_inference_unavailable"
      ? `BCa inference is unavailable because ${base.usable_replicates} successful bootstrap point refits are below the bound minimum ${base.minimum_usable_replicates}.`
      : `BCa inference is unavailable because ${failures.length} of ${sidecar.delete_one_case_count} mandatory delete-one fits failed.`;
    if (sidecar.inference.status !== "unavailable" || sidecar.inference.reason !== globalReason
      || sidecar.inference.message !== expectedMessage) return false;
  }
  const reasons = new Set([
    "base_inference_unavailable", "incomplete_delete_one_ledger",
    "bias_correction_probability_at_boundary", "degenerate_jackknife_acceleration",
    "nonfinite_jackknife_arithmetic", "singular_acceleration_adjustment",
    "invalid_adjusted_probability", "adjusted_probability_order_invalid",
    "nonfinite_or_reversed_interval",
  ]);
  return sidecar.intervals.length === base.parameter_ids.length
    && sidecar.intervals.every((interval, parameterIndex) => {
      if (interval.parameter_id !== base.parameter_ids[parameterIndex]) return false;
      const outcome = interval.outcome;
      if (outcome.status === "unavailable") {
        return reasons.has(outcome.reason)
          && (globalReason === null
            ? outcome.reason !== "base_inference_unavailable" && outcome.reason !== "incomplete_delete_one_ledger"
            : outcome.reason === globalReason);
      }
      if (globalReason !== null) return false;
      const baseInterval = base.intervals[parameterIndex];
      if (!baseInterval) return false;
      const values = [
        outcome.point_estimate, outcome.bias_correction, outcome.acceleration,
        outcome.adjusted_lower_probability, outcome.adjusted_upper_probability,
        outcome.interval_lower, outcome.interval_upper,
      ];
      const sorted = base.successful_refits
        .map((refit) => refit.parameter_estimates[parameterIndex])
        .sort((left, right) => left - right);
      return values.every((value) => isFiniteNumber(value) && !Object.is(value, -0))
        && Object.is(outcome.point_estimate, baseInterval.original)
        && outcome.adjusted_lower_probability >= 0 && outcome.adjusted_upper_probability <= 1
        && outcome.adjusted_lower_probability <= outcome.adjusted_upper_probability
        && Object.is(outcome.interval_lower, nativeCbsemType7(sorted, outcome.adjusted_lower_probability))
        && Object.is(outcome.interval_upper, nativeCbsemType7(sorted, outcome.adjusted_upper_probability))
        && outcome.interval_lower <= outcome.interval_upper
        && outcome.usable_replicates === base.usable_replicates;
    });
}

function hasValidCbsemRmseaIntervalAttribution(analysis: CbsemAnalysis): boolean {
  const attribution = analysis.fit.rmsea_interval_attribution;
  if (attribution == null) return true;
  if (
    attribution.method_version !== CBSEM_RMSEA_INTERVAL_METHOD_VERSION_V1
    || attribution.confidence_level !== CBSEM_RMSEA_INTERVAL_CONFIDENCE_LEVEL_V1
    || !Number.isSafeInteger(analysis.fit.degrees_of_freedom)
    || analysis.fit.degrees_of_freedom < 0
  ) return false;
  const values = [analysis.fit.rmsea, analysis.fit.rmsea_ci_lower, analysis.fit.rmsea_ci_upper];
  if (analysis.fit.degrees_of_freedom === 0) return values.every((value) => value == null);
  if (values.some((value) => !isFiniteNumber(value))) return false;
  const point = values[0] === 0 ? 0 : values[0]!;
  const lower = values[1] === 0 ? 0 : values[1]!;
  const upper = values[2] === 0 ? 0 : values[2]!;
  return lower >= 0 && point >= 0 && upper >= 0 && lower <= point && point <= upper;
}

export function nativeCbsemResultProjection(run: AnalysisRun | null | undefined): NativeCbsemResultProjection | null {
  if (!isCompletedResultRun(run) || run.provenance?.method !== "cbsem") return null;
  const analysis = run.result.cbsem;
  if (!analysis || (analysis.model_type !== "cfa" && analysis.model_type !== "sem")) return null;
  const methodVersion = analysis.model_type === "cfa" ? "cfa_ml_v1" : "cbsem_ml_v1";
  const assessmentVersion = run.assessment?.method_version;
  const hasBootstrapV2 = Boolean(analysis.bootstrap_v2);
  const exactBootstrap = analysis.exact_case_bootstrap;
  const exactBootstrapStudentized = analysis.exact_case_bootstrap_studentized;
  const exactBootstrapBca = analysis.exact_case_bootstrap_bca;
  const hasExactBootstrap = Boolean(exactBootstrap || exactBootstrapStudentized || exactBootstrapBca);
  try {
    if (analysis.score_lm != null) parseCbsemCfaScoreLmBundleV1(analysis.score_lm, "native CB-SEM score_lm");
  } catch {
    return null;
  }
  const diagnosticMethodVersion = analysis.score_lm ? CBSEM_SCORE_LM_METHOD_VERSION : CBSEM_MODIFICATION_METHOD_VERSION;
  const expectedProvenance = assessmentVersion
    ? `pls_pm_v1+${methodVersion}+${CBSEM_FIT_METHOD_VERSION}+${diagnosticMethodVersion}${hasBootstrapV2 ? `+${CBSEM_BOOTSTRAP_METHOD_VERSION_V2}` : ""}${hasExactBootstrap ? `+${CBSEM_EXACT_BOOTSTRAP_METHOD_VERSION_V1}` : ""}+pls_mediation_v1+${assessmentVersion}`
    : null;
  const requiredFitValues = [
    analysis.fit.chi_square,
    analysis.fit.degrees_of_freedom,
    analysis.fit.srmr,
    analysis.fit.aic,
    analysis.fit.bic,
    analysis.fit.baseline_chi_square,
    analysis.fit.baseline_degrees_of_freedom,
  ];
  if (run.result.method_version !== methodVersion
    || analysis.method_version !== methodVersion
    || analysis.fit.method_version !== CBSEM_FIT_METHOD_VERSION
    || !expectedProvenance
    || run.provenance.method_version !== expectedProvenance
    || analysis.estimator !== "ml"
    || analysis.input !== "raw"
    || analysis.mean_structure
    || !analysis.converged
    || !Number.isInteger(analysis.iterations)
    || analysis.iterations <= 0
    || !Number.isInteger(analysis.sample_size)
    || analysis.sample_size < 10
    || !isFiniteNumber(analysis.objective)
    || !isFiniteNumber(analysis.gradient_norm)
    || requiredFitValues.some((value) => !isFiniteNumber(value))
    || !hasValidCbsemRmseaIntervalAttribution(analysis)
    || analysis.bootstrap
    || [exactBootstrap, exactBootstrapStudentized, exactBootstrapBca].filter((value) => value != null).length > 1
    || (hasBootstrapV2 && hasExactBootstrap)
    || (hasExactBootstrap && analysis.model_type !== "cfa")
    || (hasExactBootstrap && (!analysis.score_lm || !analysis.fit.rmsea_interval_attribution))
    || (hasBootstrapV2 && !isNativeCbsemBootstrapV2(analysis, run))
    || (exactBootstrap != null && !isNativeCbsemExactCaseBootstrapV1(exactBootstrap, analysis, run))
    || (exactBootstrapStudentized != null
      && !isNativeCbsemExactCaseBootstrapStudentizedV1(exactBootstrapStudentized, analysis, run))
    || (exactBootstrapBca != null
      && !isNativeCbsemExactCaseBootstrapBcaV1(exactBootstrapBca, analysis, run))
    || analysis.multigroup
    || (analysis.score_lm != null && (analysis.model_type !== "cfa" || analysis.mean_structure))
    || (analysis.score_lm != null && analysis.modification_indices.length !== 0)
    || !analysis.parameters.length
    || analysis.parameters.length !== analysis.standardized.length
    || !analysis.implied_covariance.length
    || analysis.implied_covariance.length !== analysis.residual_covariance.length
    || analysis.implied_covariance.length !== analysis.residual_correlation.length) return null;
  const parametersAreValid = analysis.parameters.every((parameter, index) => {
    const standardized = analysis.standardized[index];
    return hasText(parameter.name)
      && hasText(parameter.kind)
      && hasText(parameter.lhs)
      && hasText(parameter.rhs)
      && isFiniteNumber(parameter.estimate)
      && Boolean(standardized)
      && standardized.name === parameter.name
      && standardized.kind === parameter.kind
      && standardized.lhs === parameter.lhs
      && standardized.rhs === parameter.rhs
      && isFiniteNumber(standardized.std_lv)
      && isFiniteNumber(standardized.std_all);
  });
  const matricesAreValid = [
    analysis.implied_covariance,
    analysis.residual_covariance,
    analysis.residual_correlation,
  ].every((cells) => cells.every((cell) => hasText(cell.row) && hasText(cell.column) && isFiniteNumber(cell.value)));
  const modificationIndicesAreValid = analysis.modification_indices.every((row) => (
    row.method_version === CBSEM_MODIFICATION_METHOD_VERSION
    && hasText(row.kind)
    && hasText(row.lhs)
    && hasText(row.rhs)
    && isFiniteNumber(row.modification_index)
    && (row.expected_parameter_change == null || isFiniteNumber(row.expected_parameter_change))
  ));
  return parametersAreValid && matricesAreValid && modificationIndicesAreValid
    ? { methodVersion, modelType: analysis.model_type, analysis }
    : null;
}

export function nativeCbsemDiagramRun(run: AnalysisRun): AnalysisRun {
  const projection = nativeCbsemResultProjection(run);
  if (!projection || !run.result) return run;
  const loadings = projection.analysis.standardized
    .filter((parameter) => parameter.kind === "loading")
    .map((parameter) => ({
      construct: parameter.lhs,
      indicator: parameter.rhs,
      loading: parameter.std_all,
      weight: parameter.std_all,
    }));
  const paths = projection.analysis.standardized
    .filter((parameter) => parameter.kind === "structural_path")
    .map((parameter) => ({ source: parameter.rhs, target: parameter.lhs, coefficient: parameter.std_all }));
  const endogenous = new Set(paths.map((path) => path.target));
  const rSquared = Object.fromEntries(projection.analysis.standardized
    .filter((parameter) => parameter.kind === "latent_variance" && endogenous.has(parameter.lhs))
    .map((parameter) => [parameter.lhs, Math.min(1, Math.max(0, 1 - parameter.std_all))]));
  return {
    ...run,
    result: {
      ...run.result,
      outer_estimates: loadings,
      paths,
      r_squared: rSquared,
    },
  };
}

function addCbsemResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativeCbsemResultProjection(run);
  if (!projection) return;
  const { analysis } = projection;
  const studentizedBootstrap = analysis.exact_case_bootstrap_studentized;
  const bcaBootstrap = analysis.exact_case_bootstrap_bca;
  const exactBootstrap = analysis.exact_case_bootstrap ?? studentizedBootstrap?.base ?? bcaBootstrap?.base ?? null;
  const rmseaIntervalAttribution = analysis.fit.rmsea_interval_attribution;
  const constructLabel = constructDisplayLabelResolver(run);
  const parameterLabel = (kind: string, lhs: string, rhs: string) => {
    if (kind === "loading") return `${rhs} ← ${constructLabel(lhs)}`;
    if (kind === "structural_path") return `${constructLabel(lhs)} ← ${constructLabel(rhs)}`;
    if (kind === "latent_covariance") return `${constructLabel(lhs)} ↔ ${constructLabel(rhs)}`;
    if (kind === "latent_variance") return `Variance: ${constructLabel(lhs)}`;
    if (kind === "residual_variance") return `Residual variance: ${lhs}`;
    if (kind === "residual_covariance") return `${lhs} ↔ ${rhs}`;
    return `${lhs} / ${rhs}`;
  };
  addTable(tables, {
    id: "cbsem_fit",
    title: "Model fit",
    warning: null,
    columns: ["Fit measure", "Value"],
    rows: [
      ["χ²", formatNumber(analysis.fit.chi_square)],
      ["Degrees of freedom", String(analysis.fit.degrees_of_freedom)],
      ["p (two-sided)", formatOptionalPValue(analysis.fit.p_value)],
      ["CFI", formatOptionalNumber(analysis.fit.cfi, 6)],
      ["TLI", formatOptionalNumber(analysis.fit.tli, 6)],
      ["RMSEA", formatOptionalNumber(analysis.fit.rmsea, 6)],
      ...(rmseaIntervalAttribution ? [
        ["RMSEA interval method", "Noncentral chi-square inversion (N - 1 denominator)"],
        ["RMSEA interval confidence", `${(rmseaIntervalAttribution.confidence_level * 100).toFixed(1)}%`],
      ] : []),
      ["RMSEA lower bound", formatOptionalNumber(analysis.fit.rmsea_ci_lower, 6)],
      ["RMSEA upper bound", formatOptionalNumber(analysis.fit.rmsea_ci_upper, 6)],
      ["SRMR", formatNumber(analysis.fit.srmr)],
      ["AIC", formatNumber(analysis.fit.aic)],
      ["BIC", formatNumber(analysis.fit.bic)],
      ["Baseline χ²", formatNumber(analysis.fit.baseline_chi_square)],
      ["Baseline degrees of freedom", String(analysis.fit.baseline_degrees_of_freedom)],
    ],
  });
  const bootstrap = analysis.bootstrap_v2;
  if (bootstrap) {
    const inferenceAvailable = bootstrap.inference.status === "available";
    const candidateWarning = "Experimental CB-SEM bootstrap output. Review Method Details and independently check it before final reporting.";
    const inferenceWarning = bootstrap.inference.status === "unavailable"
      ? `${candidateWarning} ${bootstrap.inference.message}`
      : `${candidateWarning} Percentile Type-7 intervals from ${bootstrap.usable_replicates.toLocaleString()} usable full-ML case refits.`;
    tables.push({
      id: "cbsem_bootstrap_intervals",
      title: "Bootstrap parameter intervals",
      warning: inferenceWarning,
      status: "experimental",
      columns: ["Parameter", "Original", "Bootstrap mean", "Bias", "Bootstrap SE", "Percentile lower", "Percentile upper", "Usable fits"],
      rows: bootstrap.intervals.map((row) => [
        row.parameter,
        formatNumber(row.original),
        formatNumber(row.bootstrap_mean),
        formatNumber(row.bias),
        formatNumber(row.standard_error),
        formatNumber(row.percentile_lower),
        formatNumber(row.percentile_upper),
        String(row.usable_replicates),
      ]),
    });
    tables.push({
      id: "cbsem_bootstrap_failures",
      title: "Bootstrap replicate failure ledger",
      warning: bootstrap.failures.length
        ? `${candidateWarning} Every failed preplanned primary draw remains visible and counts against the usable-fit threshold.`
        : candidateWarning,
      status: "experimental",
      columns: ["Replicate", "Reason code", "Message", "Sample-position digest"],
      rows: bootstrap.failures.map((row) => [
        String(row.replicate_index + 1),
        row.reason_code,
        row.message,
        row.sample_indices_sha256,
      ]),
    });
    tables.push({
      id: "cbsem_bootstrap_settings",
      title: "Bootstrap settings and threshold",
      warning: `${candidateWarning} ${bootstrap.warnings.join(" ")}`,
      status: "experimental",
      columns: ["Field", "Value"],
      rows: [
        ["Inference", inferenceAvailable ? "Available" : "Unavailable - insufficient usable full-ML fits"],
        ["Requested primary draws", String(bootstrap.requested_replicates)],
        ["Attempted ML fits", String(bootstrap.attempted_fits)],
        ["Usable ML fits", String(bootstrap.usable_replicates)],
        ["Failed ML fits", String(bootstrap.failed_replicates)],
        ["Minimum usable fits", String(bootstrap.minimum_usable_replicates)],
        ["Confidence level", `${(bootstrap.confidence_level * 100).toFixed(1)}%`],
        ["Interval method", "Percentile, Type-7 quantile"],
        ["Failure policy", "No retry or replacement draw"],
        ["Seed", String(bootstrap.seed)],
        ["Parallel workers", String(run.provenance!.settings.workers)],
        ["Method version", bootstrap.method_version],
      ],
    });
  }
  if (exactBootstrap) {
    const pilotUnavailable = exactBootstrap.requested_replicates === 500
      && exactBootstrap.inference.status === "unavailable";
    const warning = `${CBSEM_EXACT_BOOTSTRAP_TRUTH_NOTE}${pilotUnavailable
      ? " The 500-draw pilot is unavailable by design because the frozen minimum is 1,000 usable refits."
      : ""}`;
    tables.push({
      id: "cbsem_exact_bootstrap_intervals",
      title: "Exact case-bootstrap parameter intervals",
      warning,
      status: "experimental",
      columns: ["Parameter ID", "Original", "Bootstrap mean", "Bias", "Sample-SD SE", "Type-7 lower", "Type-7 upper", "Usable refits"],
      rows: exactBootstrap.intervals.map((row) => [
        row.parameter_id, formatNumber(row.original), formatNumber(row.bootstrap_mean),
        formatNumber(row.bias), formatNumber(row.standard_error), formatNumber(row.percentile_lower),
        formatNumber(row.percentile_upper), String(row.usable_replicates),
      ]),
    });
    const hypothesisTests = exactBootstrap.hypothesis_tests;
    if (hypothesisTests) {
      const selectedTest = hypothesisTests.selected_test_tail === "two_sided"
        ? "Two-sided: parameter differs from zero"
        : hypothesisTests.selected_test_tail === "one_sided_greater"
          ? "One-sided: parameter is greater than zero"
          : "One-sided: parameter is less than zero";
      const testWarning = hypothesisTests.inference.status === "available"
        ? "Null-centered unstudentized differences use inclusive tail counts and (count + 1) / (usable + 1). The selected test does not reinterpret the fixed two-sided 95% percentile interval."
        : `${hypothesisTests.inference.message} No parameter-level probability or decision is reported.`;
      tables.push({
        id: "cbsem_exact_bootstrap_hypothesis_tests",
        title: `Exact case-bootstrap zero-null tests — ${selectedTest}`,
        warning: `${CBSEM_EXACT_BOOTSTRAP_TRUTH_NOTE} ${testWarning}`,
        status: "experimental",
        columns: [
          "Parameter ID", "Status", "Point estimate", "Two-sided count", "p (two-sided)",
          "Greater/equal count", "p (greater)", "Less/equal count", "p (less)",
          "Selected count", "Selected p", "Decision at α = 0.05", "Unavailable reason", "Usable refits",
        ],
        rows: hypothesisTests.parameters.map((parameter) => {
          const outcome = parameter.outcome;
          return outcome.status === "available"
            ? [
                parameter.parameter_id, "Available", formatNumber(outcome.point_estimate),
                String(outcome.two_sided_exceedances), formatPValue(outcome.p_value_two_sided),
                String(outcome.greater_or_equal_exceedances), formatPValue(outcome.p_value_greater),
                String(outcome.less_or_equal_exceedances), formatPValue(outcome.p_value_less),
                String(outcome.selected_exceedances), formatPValue(outcome.selected_p_value),
                outcome.reject_null ? "Reject zero null" : "Do not reject zero null", "",
                String(hypothesisTests.usable_replicates),
              ]
            : [
                parameter.parameter_id, "Unavailable", "", "", "", "", "", "", "", "", "", "",
                sentenceCase(outcome.reason.replaceAll("_", " ")), String(hypothesisTests.usable_replicates),
              ];
        }),
      });
    }
    tables.push({
      id: "cbsem_exact_bootstrap_successful_refits",
      title: "Successful exact-ML bootstrap refits",
      warning,
      status: "experimental",
      columns: ["Replicate", "Schedule digest", "Source-row digest", "Parameter estimates", "Iterations", "Objective", "Gradient norm"],
      rows: exactBootstrap.successful_refits.map((row) => [
        String(row.replicate_index), row.sampling_positions_sha256, row.sample_indices_sha256,
        JSON.stringify(row.parameter_estimates), String(row.iterations), formatNumber(row.objective),
        formatNumber(row.gradient_norm),
      ]),
    });
    tables.push({
      id: "cbsem_exact_bootstrap_failures",
      title: "Failed exact-ML bootstrap refits",
      warning,
      status: "experimental",
      columns: ["Replicate", "Schedule digest", "Source-row digest", "Failure kind", "Message"],
      rows: exactBootstrap.failed_refits.map((row) => [
        String(row.replicate_index), row.sampling_positions_sha256, row.sample_indices_sha256,
        sentenceCase(row.kind.replaceAll("_", " ")), row.message,
      ]),
    });
    tables.push({
      id: "cbsem_exact_bootstrap_settings",
      title: "Exact case-bootstrap run details",
      warning,
      status: "experimental",
      columns: ["Field", "Value"],
      rows: [
        ["Inference", exactBootstrap.inference.status === "available" ? "Available" : "Unavailable — insufficient usable exact refits"],
        ["Requested preplanned refits", String(exactBootstrap.requested_replicates)],
        ["Attempted exact-ML refits", String(exactBootstrap.attempted_refits)],
        ["Usable exact-ML refits", String(exactBootstrap.usable_replicates)],
        ["Failed exact-ML refits", String(exactBootstrap.failed_replicates)],
        ["Minimum usable refits", String(exactBootstrap.minimum_usable_replicates)],
        ["Confidence level", `${(exactBootstrap.confidence_level * 100).toFixed(1)}%`],
        ["Interval method", "Percentile Type-7 with sample-SD standard errors"],
        ...(hypothesisTests ? [
          ["Zero-null test", hypothesisTests.selected_test_tail === "two_sided"
            ? "Two-sided"
            : hypothesisTests.selected_test_tail === "one_sided_greater" ? "One-sided greater" : "One-sided less"],
          ["Zero-null probability", "Inclusive null-centered count with plus-one correction"],
          ["Zero-null inference", hypothesisTests.inference.status === "available" ? "Available" : "Unavailable — insufficient usable exact refits"],
          ["Zero-null method version", hypothesisTests.method_version],
        ] : []),
        ["Failure handling", "Failed fits retained; no retry or replacement draw"],
        ["Archive validation", "Schedule descriptors and arithmetic checked; raw fits and the Rust schedule were not replayed in this browser reader"],
        ["Seed", String(exactBootstrap.seed)],
        ["Stream", exactBootstrap.stream_token],
        ["Method version", exactBootstrap.method_version],
      ],
    });
  }
  if (studentizedBootstrap) {
    const sidecar = studentizedBootstrap.studentized;
    const inferenceUnavailable = sidecar.inference.status === "unavailable" ? sidecar.inference : null;
    const warning = `${CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_TRUTH_NOTE}${inferenceUnavailable
      ? ` ${inferenceUnavailable.message}`
      : ` ${sidecar.studentized_usable_replicates.toLocaleString()} whole-vector refits supplied usable expected-information standard errors.`}`;
    tables.push({
      id: "exact_case_bootstrap_studentized_summary",
      title: "Analytically studentized bootstrap summary",
      warning,
      status: "experimental",
      columns: [
        "method_version", "standard_error_method_version", "expected_information_method", "pivot_method",
        "quantile_method", "interval_method", "archive_validation_scope", "confidence_level",
        "minimum_usable_fraction", "minimum_usable_replicates", "studentized_usable_replicates",
        "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message",
      ],
      rows: [[
        sidecar.method_version,
        sidecar.standard_error_method_version,
        sidecar.expected_information_method,
        sidecar.pivot_method,
        sidecar.quantile_method,
        sidecar.interval_method,
        sidecar.archive_validation_scope,
        String(sidecar.confidence_level),
        String(sidecar.minimum_usable_fraction),
        String(sidecar.minimum_usable_replicates),
        String(sidecar.studentized_usable_replicates),
        JSON.stringify(sidecar.parameter_ids),
        sidecar.inference.status,
        inferenceUnavailable?.reason ?? "",
        inferenceUnavailable?.message ?? "",
      ]],
    });
    const pointReceipt = sidecar.point_standard_errors;
    tables.push({
      id: "exact_case_bootstrap_studentized_point_standard_errors",
      title: "Point-estimate analytical standard errors",
      warning,
      status: "experimental",
      columns: [
        "method_version", "parameter_id", "status", "information_method", "standard_error", "unavailable_reason",
      ],
      rows: sidecar.parameter_ids.map((parameterId, index) => {
        const outcome = pointReceipt.outcome;
        const available = outcome.status === "available" ? outcome.parameters[index] : null;
        return [
          pointReceipt.method_version,
          parameterId,
          outcome.status,
          outcome.status === "available" ? outcome.information_method : "",
          available ? formatNumber(available.standard_error) : "",
          outcome.status === "unavailable" ? outcome.reason : "",
        ];
      }),
    });
    tables.push({
      id: "exact_case_bootstrap_studentized_parameter_intervals",
      title: "Analytically studentized parameter intervals",
      warning,
      status: "experimental",
      columns: [
        "parameter_id", "status", "point_estimate", "point_standard_error", "lower_pivot_quantile",
        "upper_pivot_quantile", "interval_lower", "interval_upper", "usable_replicates", "unavailable_reason",
      ],
      rows: sidecar.intervals.map((interval) => {
        const outcome = interval.outcome;
        return outcome.status === "available"
          ? [
              interval.parameter_id,
              outcome.status,
              formatNumber(outcome.point_estimate),
              formatNumber(outcome.point_standard_error),
              formatNumber(outcome.lower_pivot_quantile),
              formatNumber(outcome.upper_pivot_quantile),
              formatNumber(outcome.interval_lower),
              formatNumber(outcome.interval_upper),
              String(outcome.usable_replicates),
              "",
            ]
          : [interval.parameter_id, outcome.status, "", "", "", "", "", "", "", outcome.reason];
      }),
    });
    tables.push({
      id: "exact_case_bootstrap_studentized_refit_standard_errors",
      title: "Compact refit standard-error receipts",
      warning,
      status: "experimental",
      columns: [
        "replicate_index", "status", "information_method", "standard_errors_json", "unavailable_reason",
      ],
      rows: sidecar.refit_standard_errors.map((receipt) => {
        const outcome = receipt.outcome;
        return outcome.status === "available"
          ? [
              String(receipt.replicate_index),
              outcome.status,
              outcome.information_method,
              JSON.stringify(outcome.standard_errors),
              "",
            ]
          : [String(receipt.replicate_index), outcome.status, "", "", outcome.reason];
      }),
    });
  }
  if (bcaBootstrap) {
    const sidecar = bcaBootstrap.bca;
    const unavailable = sidecar.inference.status === "unavailable" ? sidecar.inference : null;
    const warning = `${CBSEM_EXACT_BOOTSTRAP_BCA_TRUTH_NOTE}${unavailable ? ` ${unavailable.message}` : ""}`;
    tables.push({
      id: "exact_case_bootstrap_bca_summary",
      title: "BCa bootstrap summary",
      warning,
      status: "experimental",
      columns: [
        "method_version", "base_bootstrap_method_version", "outer_recipe_analytical_identity_sha256",
        "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256",
        "model_scientific_sha256", "delete_one_refit_method_version",
        "delete_one_sampling_positions_digest_method", "delete_one_sample_indices_digest_method",
        "bias_correction_method", "acceleration_method", "adjusted_probability_method", "quantile_method",
        "retry_policy", "archive_validation_scope", "confidence_level", "bootstrap_usable_replicates",
        "minimum_bootstrap_usable_replicates", "delete_one_case_count", "successful_delete_one_refits",
        "failed_delete_one_refits", "parameter_ids_json", "inference_status", "unavailable_reason_code",
        "unavailable_message",
      ],
      rows: [[
        sidecar.method_version,
        sidecar.base_bootstrap_method_version,
        sidecar.outer_recipe_analytical_identity_sha256,
        sidecar.base_point_result_sha256,
        sidecar.compiler_analytical_identity_sha256,
        sidecar.plan_sha256,
        sidecar.model_scientific_sha256,
        sidecar.delete_one_refit_method_version,
        "sha256_complete_case_n_and_ordered_sampling_positions_v1",
        "sha256_source_fingerprint_and_ordered_u64_indices_v1",
        sidecar.bias_correction_method,
        sidecar.acceleration_method,
        sidecar.adjusted_probability_method,
        sidecar.quantile_method,
        sidecar.retry_policy,
        "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1",
        String(sidecar.confidence_level),
        String(sidecar.bootstrap_usable_replicates),
        String(sidecar.minimum_bootstrap_usable_replicates),
        String(sidecar.delete_one_case_count),
        String(sidecar.successful_delete_one_refits.length),
        String(sidecar.failed_delete_one_refits.length),
        JSON.stringify(sidecar.parameter_ids),
        sidecar.inference.status,
        unavailable?.reason ?? "",
        unavailable?.message ?? "",
      ]],
    });
    tables.push({
      id: "exact_case_bootstrap_bca_parameter_intervals",
      title: "BCa Type-7 parameter intervals",
      warning,
      status: "experimental",
      columns: [
        "parameter_id", "status", "point_estimate", "bias_correction", "acceleration",
        "adjusted_lower_probability", "adjusted_upper_probability", "interval_lower", "interval_upper",
        "usable_replicates", "unavailable_reason",
      ],
      rows: sidecar.intervals.map((interval) => {
        const outcome = interval.outcome;
        return outcome.status === "available"
          ? [
              interval.parameter_id, outcome.status, formatNumber(outcome.point_estimate),
              formatNumber(outcome.bias_correction), formatNumber(outcome.acceleration),
              formatNumber(outcome.adjusted_lower_probability), formatNumber(outcome.adjusted_upper_probability),
              formatNumber(outcome.interval_lower), formatNumber(outcome.interval_upper),
              String(outcome.usable_replicates), "",
            ]
          : [interval.parameter_id, outcome.status, "", "", "", "", "", "", "", "", sentenceCase(outcome.reason.replaceAll("_", " "))];
      }),
    });
    tables.push({
      id: "exact_case_bootstrap_bca_successful_delete_one_refits",
      title: "Successful BCa delete-one refits",
      warning,
      status: "experimental",
      columns: [
        "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
        "retained_sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm",
      ],
      rows: sidecar.successful_delete_one_refits.map((row) => [
        String(row.omitted_complete_case_position), String(row.omitted_source_row_index),
        row.retained_sampling_positions_sha256, row.retained_sample_indices_sha256,
        JSON.stringify(row.parameter_estimates), String(row.iterations), formatNumber(row.objective),
        formatNumber(row.gradient_norm),
      ]),
    });
    tables.push({
      id: "exact_case_bootstrap_bca_failures",
      title: "Failed BCa delete-one refits",
      warning,
      status: "experimental",
      columns: [
        "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
        "retained_sample_indices_sha256", "kind", "message",
      ],
      rows: sidecar.failed_delete_one_refits.map((row) => [
        String(row.omitted_complete_case_position), String(row.omitted_source_row_index),
        row.retained_sampling_positions_sha256, row.retained_sample_indices_sha256,
        sentenceCase(row.kind.replaceAll("_", " ")), row.message,
      ]),
    });
  }
  addTable(tables, {
    id: "cbsem_standardized_parameters",
    title: "Standardized parameters",
    warning: null,
    columns: ["Parameter", "Type", "Std. LV", "Std. all"],
    rows: analysis.standardized.map((parameter) => [
      parameterLabel(parameter.kind, parameter.lhs, parameter.rhs),
      sentenceCase(parameter.kind.replaceAll("_", " ")),
      formatNumber(parameter.std_lv),
      formatNumber(parameter.std_all),
    ]),
  });
  addTable(tables, {
    id: "cbsem_unstandardized_parameters",
    title: "Unstandardized parameters",
    warning: null,
    columns: ["Parameter", "Type", "Estimate", "SE", "z", "p (two-sided)", "Status"],
    rows: analysis.parameters.map((parameter) => [
      parameterLabel(parameter.kind, parameter.lhs, parameter.rhs),
      sentenceCase(parameter.kind.replaceAll("_", " ")),
      formatNumber(parameter.estimate),
      formatOptionalNumber(parameter.standard_error),
      formatOptionalNumber(parameter.z_statistic),
      formatOptionalPValue(parameter.p_value_two_sided),
      parameter.fixed ? "Fixed for marker identification" : "Estimated",
    ]),
  });
  const matrixRows = (cells: CbsemAnalysis["implied_covariance"]) => {
    const order = [...new Set(cells.map((cell) => cell.row))];
    const index = new Map(order.map((name, position) => [name, position]));
    return cells
      .filter((cell) => (index.get(cell.row) ?? -1) <= (index.get(cell.column) ?? -1))
      .map((cell) => [cell.row, cell.column, formatNumber(cell.value)]);
  };
  addTable(tables, {
    id: "cbsem_residual_correlations",
    title: "Residual correlations",
    warning: null,
    columns: ["Indicator", "Compared with", "Residual correlation"],
    rows: matrixRows(analysis.residual_correlation),
  });
  addTable(tables, {
    id: "cbsem_residual_covariances",
    title: "Residual covariances",
    warning: null,
    columns: ["Indicator", "Compared with", "Residual covariance"],
    rows: matrixRows(analysis.residual_covariance),
  });
  addTable(tables, {
    id: "cbsem_implied_covariances",
    title: "Model-implied covariances",
    warning: null,
    columns: ["Indicator", "Compared with", "Implied covariance"],
    rows: matrixRows(analysis.implied_covariance),
  });
  if (analysis.score_lm) addTable(tables, {
      id: "modification_index_score_tests",
      title: "Exact residual-covariance score/LM tests",
      warning: "Genuine one-degree-of-freedom score/LM tests only for residual covariances explicitly declared and fixed to zero.",
      columns: ["Candidate", "Type", "Status", "Score", "Efficient score", "Candidate information", "Efficient information", "Modification index", "Expected parameter change", "df", "p", "Unavailable reason"],
      rows: analysis.score_lm.rows.map((row) => {
        const available = row.outcome.status === "available" ? row.outcome : null;
        return [
          parameterLabel(row.kind, row.lhs, row.rhs),
          "Residual covariance fixed to zero",
          row.outcome.status === "available" ? "Available" : "Unavailable",
          formatOptionalNumber(available?.score, 6),
          formatOptionalNumber(available?.efficient_score, 6),
          formatOptionalNumber(available?.candidate_information, 6),
          formatOptionalNumber(available?.efficient_information, 6),
          formatOptionalNumber(available?.modification_index, 6),
          formatOptionalNumber(available?.expected_parameter_change, 6),
          available ? "1" : "",
          formatOptionalPValue(available?.p_value),
          row.outcome.status === "unavailable" ? sentenceCase(row.outcome.reason.replaceAll("_", " ")) : "",
        ];
      }),
    });
  if (!analysis.score_lm) addTable(tables, {
      id: "cbsem_modification_diagnostics",
      title: "Residual-based modification diagnostics",
      warning: "Screening diagnostics only. They are not an instruction to modify the specified model.",
      columns: ["Candidate", "Type", "Diagnostic index", "Expected parameter change"],
      rows: analysis.modification_indices.map((row) => [
        parameterLabel(row.kind, row.lhs, row.rhs),
        sentenceCase(row.kind.replaceAll("_", " ")),
        formatNumber(row.modification_index),
        formatOptionalNumber(row.expected_parameter_change),
      ]),
    });
  addTable(tables, {
    id: "cbsem_scope",
    title: "Run details",
    warning: null,
    columns: ["Field", "Value"],
    rows: [
      ["Model type", projection.modelType === "cfa" ? "Confirmatory factor analysis" : "Recursive structural equation model"],
      ["Estimator", "Maximum likelihood"],
      ["Input", "Raw case-level data; indicators standardized after listwise filtering"],
      ["Identification", "First loading fixed to 1 for each latent factor"],
      ["Mean structure", "Not estimated"],
      ["Analyzed observations", String(analysis.sample_size)],
      ["Converged", analysis.converged ? "Yes" : "No"],
      ["Optimizer iterations", String(analysis.iterations)],
      ["Objective", formatNumber(analysis.objective)],
      ["Gradient norm", formatNumber(analysis.gradient_norm)],
      ["Estimator method version", projection.methodVersion],
      ["Fit method version", analysis.fit.method_version],
      ...(rmseaIntervalAttribution ? [["RMSEA interval method version", rmseaIntervalAttribution.method_version]] : []),
      ...(!analysis.score_lm ? [["Modification-diagnostic version", CBSEM_MODIFICATION_METHOD_VERSION]] : []),
      ...(analysis.score_lm ? [["Score/LM method version", CBSEM_SCORE_LM_METHOD_VERSION]] : []),
      ["CB-SEM bootstrap", exactBootstrap
        ? `${exactBootstrap.requested_replicates.toLocaleString()} preplanned full exact-ML case-resampling draws${studentizedBootstrap ? " with analytic studentization" : bcaBootstrap ? " with complete-only BCa delete-one inference" : ""}`
        : bootstrap ? `${bootstrap.requested_replicates.toLocaleString()} preplanned full-ML case-resampling draws` : "Not requested"],
    ],
  });
}

function addGscaResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativeGscaResultProjection(run);
  if (!projection || !run.modelSnapshot) return;
  const { analysis } = projection;
  const constructLabel = constructDisplayLabelResolver(run);
  const modeLabel = (constructId: string) => projection.constructModes[constructId] === "formative" ? "Formative" : "Reflective";
  const outerKey = (construct: string, indicator: string) => `${construct}\u0000${indicator}`;
  const weightByKey = new Map(analysis.weights.map((row) => [outerKey(row.construct, row.indicator), row]));
  const loadingByKey = new Map(analysis.loadings.map((row) => [outerKey(row.construct, row.indicator), row]));
  const orderedOuterRows = run.modelSnapshot.nodes.flatMap((node) => node.data.indicators.map((indicator) => ({
    construct: node.id,
    indicator,
  })));

  addTable(tables, {
    id: "gsca_fit",
    title: "Model fit and convergence",
    warning: null,
    columns: ["Measure", "Value"],
    rows: [
      ["Global FIT", formatNumber(analysis.fit)],
      ["Adjusted FIT", formatNumber(analysis.adjusted_fit)],
      ["Measurement FIT", formatNumber(analysis.measurement_fit)],
      ["Structural FIT", formatNumber(analysis.structural_fit)],
      ["GFI", formatNumber(analysis.gfi)],
      ["SRMR", formatNumber(analysis.srmr)],
      ["Objective", formatNumber(analysis.objective)],
      ["Converged", analysis.converged ? "Yes" : "No"],
      ["ALS iterations", String(analysis.iterations)],
      ["Final objective-and-weight change", formatNumber(analysis.final_change)],
      ["Analyzed observations", String(projection.usedObservations)],
      ["Omitted observations", String(projection.omittedObservations)],
    ],
  });
  addTable(tables, {
    id: "gsca_paths",
    title: "Structural path coefficients",
    warning: null,
    columns: ["Path", "Coefficient"],
    rows: analysis.paths.map((row) => [
      `${constructLabel(row.target)} ← ${constructLabel(row.source)}`,
      formatNumber(row.coefficient),
    ]),
  });
  addTable(tables, {
    id: "gsca_r_squared",
    title: "Endogenous construct R²",
    warning: null,
    columns: ["Endogenous construct", "R²"],
    rows: run.modelSnapshot.nodes.flatMap((node) => Object.hasOwn(analysis.r_squared, node.id)
      ? [[constructLabel(node.id), formatNumber(analysis.r_squared[node.id])]]
      : []),
  });
  addTable(tables, {
    id: "gsca_loadings",
    title: "Measurement loadings",
    warning: null,
    columns: ["Construct", "Indicator", "Measurement model", "Loading"],
    rows: orderedOuterRows.map(({ construct, indicator }) => [
      constructLabel(construct),
      indicator,
      modeLabel(construct),
      formatNumber(loadingByKey.get(outerKey(construct, indicator))!.loading),
    ]),
  });
  addTable(tables, {
    id: "gsca_weights",
    title: "Component weights",
    warning: null,
    columns: ["Construct", "Indicator", "Measurement model", "Weight"],
    rows: orderedOuterRows.map(({ construct, indicator }) => [
      constructLabel(construct),
      indicator,
      modeLabel(construct),
      formatNumber(weightByKey.get(outerKey(construct, indicator))!.weight),
    ]),
  });
  addTable(tables, {
    id: "gsca_scope",
    title: "Run details",
    warning: null,
    columns: ["Field", "Value"],
    rows: [
      ["Estimator", "Joint global least-squares alternating least squares"],
      ["Method version", projection.methodVersion],
      ["Algorithm version", projection.algorithmVersion],
      ["Initialization", "Deterministic +1 block weights"],
      ["Maximum iterations", "3,000"],
      ["Stop criterion", "1e-7 for both objective and normalized weights"],
      ["Input", "Raw case-level data with listwise-standardized numeric indicators"],
      ["Measurement models", "Disjoint reflective and formative blocks"],
      ["Structural model", "Recursive single-group paths; every construct connected"],
      ["Inference", "Point estimates only"],
    ],
  });
}

function addNcaResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativeNcaResultProjection(run);
  if (!projection) return;

  addTable(tables, {
    id: "nca_ceiling_effects",
    title: "Ceiling effect sizes and permutation inference",
    warning: null,
    columns: ["Ceiling line", "Effect size", "Permutation p"],
    rows: projection.ceilingEffects.map((row) => [
      nativeNcaCeilingLabel(row.ceiling),
      formatNumber(row.effectSize, 4),
      formatPValue(row.permutationPValue),
    ]),
  });

  const ceFdhPeers = ncaCeFdhPeerTableFromProjection(projection);
  if (ceFdhPeers) tables.push(ceFdhPeers);

  addTable(tables, {
    id: "nca_cr_line",
    title: "CR-FDH ceiling coefficients",
    warning: null,
    columns: ["Ceiling line", "Slope", "Intercept"],
    rows: projection.ceilingEffects
      .filter((row) => row.ceiling === "cr_fdh" && row.slope !== null && row.intercept !== null)
      .map((row) => [
        "CR-FDH",
        formatNumber(row.slope as number),
        formatNumber(row.intercept as number),
      ]),
  });

  addTable(tables, {
    id: "nca_bottlenecks",
    title: "Observed-range bottlenecks",
    warning: null,
    columns: ["Ceiling line", "Outcome (% observed range)", "Condition requirement"],
    rows: projection.bottlenecks.map((row) => [
      nativeNcaCeilingLabel(row.ceiling),
      `${formatNumber(row.outcomePercent, 0)}%`,
      row.status === "required"
        ? `${formatNumber(row.requiredXPercent as number, 4)}% of observed X range`
        : row.status === "not_necessary"
          ? "Not necessary"
          : "Not attainable",
    ]),
  });

  addTable(tables, {
    id: "nca_scope",
    title: "Run details",
    warning: visibleResultWarning(projection.warnings, [NATIVE_NCA_ENGINE_SCOPE_WARNING]),
    columns: ["Field", "Value"],
    rows: [
      ["Condition variable (X)", projection.x],
      ["Outcome variable (Y)", projection.y],
      ["Analyzed observations", String(projection.observations)],
      ["X observed range", `${formatNumber(projection.scope.minimumX)} to ${formatNumber(projection.scope.maximumX)}`],
      ["Y observed range", `${formatNumber(projection.scope.minimumY)} to ${formatNumber(projection.scope.maximumY)}`],
      ["Ceiling lines", nativeNcaCeilingLabel(projection.ceiling)],
      ["Requested permutations", String(projection.permutationSamples)],
      ["Usable permutations", String(projection.usablePermutations)],
      ["Missing data", "Listwise deletion"],
      ["Method version", projection.methodVersion],
    ],
  });
}

function addPcaResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativePcaResultProjection(run);
  if (!projection) return;

  addTable(tables, {
    id: "pca_component_summary",
    title: "Component summary",
    warning: null,
    columns: ["Component", "Eigenvalue", "Explained variance", "Cumulative variance"],
    rows: projection.components.map((component) => [
      component.component,
      formatNumber(component.eigenvalue),
      `${formatNumber(component.explained_variance * 100, 2)}%`,
      `${formatNumber(component.cumulative_variance * 100, 2)}%`,
    ]),
  });

  addTable(tables, {
    id: "pca_loadings",
    title: "Component loadings and weights",
    warning: null,
    columns: ["Variable", "Component", "Loading", "Weight"],
    rows: projection.loadings.map((row) => [
      row.variable,
      row.component,
      formatNumber(row.loading),
      formatNumber(row.weight),
    ]),
  });

  addTable(tables, {
    id: "pca_scope",
    title: "Run details",
    warning: visibleResultWarning(projection.warnings, [NATIVE_PCA_ENGINE_SCOPE_WARNING]),
    columns: ["Field", "Value"],
    rows: [
      ["Variables", String(projection.variables.length)],
      ["Selected variables", projection.variables.join(", ")],
      ["Analyzed observations", String(projection.observations)],
      ["Retention rule", nativePcaComponentRuleLabel(projection.componentRule)],
      ["Retained components", String(projection.retainedComponents)],
      ["Stored component scores", String(projection.scoresStored)],
      ["Input matrix", "Correlation matrix of standardized variables"],
      ["Missing data", "Listwise deletion"],
      ["Rotation", "None"],
      ["Method version", projection.methodVersion],
    ],
  });
}

export function nativePcaComponentRuleLabel(
  rule: NativePcaResultProjection["componentRule"],
): string {
  if (rule === "fixed") return "Fixed component count";
  if (rule === "variance_threshold") return "Cumulative variance threshold";
  return "Kaiser criterion (eigenvalue at least 1)";
}

function addOlsResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativeOlsResultProjection(run);
  if (!projection) return;

  addTable(tables, {
    id: "ols_coefficients",
    title: "Coefficients",
    warning: null,
    columns: ["Term", "Estimate", "HC3 SE", "t", "p (two-sided)", "95% CI lower", "95% CI upper"],
    rows: projection.coefficients.map((row) => [
      row.term === "intercept" ? "Intercept" : row.term,
      formatNumber(row.estimate),
      formatNumber(row.standard_error),
      formatNumber(row.statistic),
      formatPValue(row.p_value_two_sided),
      formatNumber(row.confidence_interval_lower),
      formatNumber(row.confidence_interval_upper),
    ]),
  });

  addTable(tables, {
    id: "ols_model_fit",
    title: "Model fit",
    warning: null,
    columns: ["Observations", "R²", "Adjusted R²", "F", "RMSE", "AIC", "BIC"],
    rows: [[
      String(projection.observations),
      formatNumber(projection.fit.r_squared!),
      formatNumber(projection.fit.adjusted_r_squared!),
      formatNumber(projection.fit.f_statistic!),
      formatNumber(projection.fit.rmse!),
      formatNumber(projection.fit.aic),
      formatNumber(projection.fit.bic),
    ]],
  });

  addTable(tables, {
    id: "ols_scope",
    title: "Run details",
    warning: visibleResultWarning(projection.warnings, [NATIVE_OLS_ENGINE_SCOPE_WARNING]),
    columns: ["Field", "Value"],
    rows: [
      ["Outcome", projection.outcome],
      ["Predictors", projection.predictors.join(", ")],
      ["Controls", projection.controls.length ? projection.controls.join(", ") : "None"],
      ["Analyzed observations", String(projection.observations)],
      ["Stored fitted values and residuals", String(projection.predictionsStored)],
      ["Estimator", "Ordinary least squares with intercept"],
      ["Standard errors", "HC3 heteroskedasticity-consistent"],
      ["Confidence intervals", "Two-sided 95%"],
      ["Variable data", "Unstandardized observed numeric values"],
      ["Missing data", "Listwise deletion"],
      ["Method version", projection.methodVersion],
    ],
  });

  if (projection.bootstrap) addRegressionBootstrapTables(tables, projection.bootstrap, false);
}

function addRegressionBootstrapTables(
  tables: ResultTable[],
  bootstrap: RegressionBootstrapAnalysis,
  logistic: boolean,
) {
  const termLabel = (term: string) => term === "intercept" ? "Intercept" : term;
  const unavailable = (message: string) => `Unavailable — ${message}`;
  const runtimeWarning = "Runtime scales with the requested resamples. Indexed seeded streams make results deterministic and invariant to the selected worker count.";

  addTable(tables, {
    id: "regression_bootstrap_summary",
    title: "Regression bootstrap summary",
    warning: bootstrap.warnings.length ? `${runtimeWarning} ${bootstrap.warnings.join(" ")}` : runtimeWarning,
    columns: ["Field", "Value"],
    rows: [
      ["Method version", bootstrap.method_version],
      ["Sampling", "Case resampling with replacement"],
      ["Algorithm", bootstrap.algorithm],
      ["Stream", bootstrap.stream_token],
      ["Alternative", "Two-sided"],
      ["Test reference", "Standard normal bootstrap ratio"],
      ["Test tolerance policy", bootstrap.test_tolerance_policy],
      ["Confidence level", "95% (fixed)"],
      ["Interval policy", "Percentile primary; BCa conditional"],
      ["Requested replicates", String(bootstrap.requested_replicates)],
      ["Usable replicates", String(bootstrap.usable_replicates)],
      ["Failed replicates", String(bootstrap.failed_replicates.length)],
      ["Delete-one fits required", String(bootstrap.jackknife_cases)],
      ["Delete-one fits usable", String(bootstrap.usable_jackknife_cases)],
      ["Minimum usable fraction", "90%"],
      ["Seed", String(bootstrap.seed)],
      ["Workers", String(bootstrap.workers)],
    ],
  });

  addTable(tables, {
    id: "regression_bootstrap_coefficients",
    title: "Bootstrap coefficient inference",
    warning: "The bootstrap ratio uses a standard-normal reference and is reported separately from point-estimate t or Wald inference. Significance is not inferred solely from interval inclusion.",
    columns: ["Term", "Original", "Bootstrap mean", "Bias", "Bootstrap SE", "Replicate max |estimate|", "Test tolerance", "Test status", "Bootstrap ratio", "p (two-sided)", "Usable replicates"],
    rows: bootstrap.coefficients.map((row) => [
      termLabel(row.term),
      formatNumber(row.original),
      formatNumber(row.bootstrap_mean),
      formatNumber(row.bias),
      formatNumber(row.standard_error),
      formatNumber(row.replicate_max_abs),
      String(row.test_tolerance),
      row.test.status === "available" ? "Available" : unavailable(row.test.message),
      row.test.status === "available" ? formatNumber(row.test.statistic) : "",
      row.test.status === "available" ? formatPValue(row.test.p_value_two_sided) : "",
      String(row.usable_replicates),
    ]),
  });

  if (bootstrap.failed_replicates.length) {
    addTable(tables, {
      id: "regression_bootstrap_failures",
      title: "Failed bootstrap replicates",
      warning: "Failed fits are excluded from inference and retained with their engine reason. The run is rejected when fewer than 90% of requested replicates are usable.",
      columns: ["Replicate", "Reason code", "Message"],
      rows: bootstrap.failed_replicates.map((failure) => [
        String(failure.replicate_index + 1),
        failure.reason_code,
        failure.message,
      ]),
    });
  }

  addTable(tables, {
    id: "regression_bootstrap_percentile",
    title: "Percentile confidence intervals (primary)",
    warning: "Primary two-sided 95% case-resampling intervals.",
    columns: ["Term", "Original", "95% lower", "95% upper", "Usable replicates"],
    rows: bootstrap.coefficients.map((row) => [
      termLabel(row.term),
      formatNumber(row.original),
      formatNumber(row.percentile_lower),
      formatNumber(row.percentile_upper),
      String(row.usable_replicates),
    ]),
  });

  addTable(tables, {
    id: "regression_bootstrap_bca",
    title: "BCa confidence intervals (conditional)",
    warning: "BCa is an alternative interval. Failed delete-one refits or degenerate jackknife acceleration are disclosed per coefficient.",
    columns: ["Term", "Status", "Reason", "Bias correction", "Acceleration", "95% lower", "95% upper"],
    rows: bootstrap.coefficients.map((row) => row.bca.status === "available"
      ? [termLabel(row.term), "Available", "", formatNumber(row.bca.bias_correction), formatNumber(row.bca.acceleration), formatNumber(row.bca.lower), formatNumber(row.bca.upper)]
      : [termLabel(row.term), "Unavailable", row.bca.message, "", "", "", ""]),
  });

  if (logistic) {
    addTable(tables, {
      id: "regression_bootstrap_odds_ratios",
      title: "Bootstrap odds-ratio intervals",
      warning: "Odds-ratio BCa intervals are calculated on the exponentiated resampling and jackknife distributions; they are not reconstructed from coefficient BCa endpoints.",
      columns: ["Term", "Odds ratio", "Percentile 95% lower", "Percentile 95% upper", "BCa status", "BCa reason", "BCa 95% lower", "BCa 95% upper"],
      rows: bootstrap.coefficients.map((row) => {
        const oddsRatio = row.odds_ratio!;
        return oddsRatio.bca.status === "available"
          ? [termLabel(row.term), formatNumber(oddsRatio.original), formatNumber(oddsRatio.percentile_lower), formatNumber(oddsRatio.percentile_upper), "Available", "", formatNumber(oddsRatio.bca.lower), formatNumber(oddsRatio.bca.upper)]
          : [termLabel(row.term), formatNumber(oddsRatio.original), formatNumber(oddsRatio.percentile_lower), formatNumber(oddsRatio.percentile_upper), "Unavailable", oddsRatio.bca.message, "", ""];
      }),
    });
  }
}

function addLogisticResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativeLogisticResultProjection(run);
  if (!projection) return;
  const { classification, convergence, outcome_profile: profile } = projection.diagnostics;

  addTable(tables, {
    id: "logistic_coefficients",
    title: "Coefficients, Wald inference, and odds ratios",
    warning: null,
    columns: ["Term", "Estimate", "ML SE", "Wald z", "p (two-sided)", "95% CI lower", "95% CI upper", "Odds ratio", "OR 95% CI lower", "OR 95% CI upper"],
    rows: projection.coefficients.map((row) => [
      row.term === "intercept" ? "Intercept" : row.term,
      formatNumber(row.estimate),
      formatNumber(row.standard_error),
      formatNumber(row.statistic),
      formatPValue(row.p_value_two_sided),
      formatNumber(row.confidence_interval_lower),
      formatNumber(row.confidence_interval_upper),
      formatNumber(row.odds_ratio!),
      formatNumber(row.odds_ratio_confidence_interval_lower!),
      formatNumber(row.odds_ratio_confidence_interval_upper!),
    ]),
  });

  addTable(tables, {
    id: "logistic_fit",
    title: "Model fit and likelihood-ratio inference",
    warning: null,
    columns: ["Metric", "Value"],
    rows: [
      ["Analyzed observations", String(projection.observations)],
      ["Log likelihood", formatNumber(projection.fit.log_likelihood!)],
      ["Null log likelihood", formatNumber(projection.fit.null_log_likelihood!)],
      ["Deviance", formatNumber(projection.fit.deviance!)],
      ["Null deviance", formatNumber(projection.fit.null_deviance!)],
      ["Likelihood-ratio chi-square", formatNumber(projection.fit.likelihood_ratio_chi_square!)],
      ["Likelihood-ratio df", String(projection.fit.likelihood_ratio_degrees_of_freedom!)],
      ["Likelihood-ratio p", formatPValue(projection.fit.likelihood_ratio_p_value!)],
      ["McFadden pseudo-R²", formatNumber(projection.fit.pseudo_r_squared!)],
      ["AIC", formatNumber(projection.fit.aic)],
      ["BIC", formatNumber(projection.fit.bic)],
    ],
  });

  addTable(tables, {
    id: "logistic_classification",
    title: "Classification at probability threshold 0.5",
    warning: null,
    columns: ["True positive", "True negative", "False positive", "False negative", "Accuracy", "Sensitivity", "Specificity"],
    rows: [[
      String(classification.true_positive),
      String(classification.true_negative),
      String(classification.false_positive),
      String(classification.false_negative),
      formatNumber(classification.accuracy, 4),
      formatNumber(classification.sensitivity, 4),
      formatNumber(classification.specificity, 4),
    ]],
  });

  addTable(tables, {
    id: "logistic_outcome_profile",
    title: "Binary outcome profile",
    warning: null,
    columns: ["Outcome", "Coding", "Complete cases", "Omitted cases", "Class 0", "Class 1", "Class 1 prevalence", "Readiness"],
    rows: [[
      profile.outcome,
      "Numeric 0/1 (exact)",
      String(profile.complete_cases),
      String(profile.omitted_cases),
      String(profile.zero_count),
      String(profile.one_count),
      formatNumber(profile.prevalence!, 4),
      "Ready",
    ]],
  });

  addTable(tables, {
    id: "logistic_convergence",
    title: "Estimator convergence",
    warning: null,
    columns: ["Algorithm", "Converged", "Iterations", "Maximum iterations", "Tolerance", "Final maximum absolute step", "Separation probability tolerance"],
    rows: [[
      "Deterministic Newton IRLS",
      convergence.converged ? "Yes" : "No",
      String(convergence.iterations),
      String(convergence.max_iterations),
      String(convergence.tolerance),
      formatNumber(convergence.final_max_abs_step, 10),
      String(convergence.separation_probability_tolerance),
    ]],
  });

  addTable(tables, {
    id: "logistic_probabilities",
    title: "Complete-case fitted probabilities",
    warning: null,
    columns: ["Complete-case observation", "Fitted probability", "Residual"],
    rows: projection.predictions.map((row) => [
      String(row.observation + 1),
      formatNumber(row.probability!),
      formatNumber(row.residual!),
    ]),
  });

  addTable(tables, {
    id: "logistic_scope",
    title: "Run details",
    warning: visibleResultWarning(projection.warnings, [NATIVE_LOGISTIC_ENGINE_SCOPE_WARNING]),
    columns: ["Field", "Value"],
    rows: [
      ["Outcome", projection.outcome],
      ["Predictors", projection.predictors.join(", ")],
      ["Controls", projection.controls.length ? projection.controls.join(", ") : "None"],
      ["Estimator", "Binary logistic maximum likelihood with intercept"],
      ["Execution", projection.bootstrap
        ? `Deterministic Newton IRLS point estimation; indexed bootstrap resampling with ${projection.bootstrap.workers} worker${projection.bootstrap.workers === 1 ? "" : "s"}`
        : "Deterministic Newton IRLS; one worker"],
      ["Coefficient inference", "Maximum-likelihood SE; Wald z; two-sided 95% confidence intervals"],
      ["Classification threshold", "0.5"],
      ["Variable data", "Unstandardized observed numeric values"],
      ["Missing data", "Listwise deletion"],
      ["Method version", projection.methodVersion],
    ],
  });

  if (projection.bootstrap) addRegressionBootstrapTables(tables, projection.bootstrap, true);
}

function addLegacyLogisticResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
) {
  const projection = nativeLegacyLogisticResultProjection(run);
  if (!projection) return;
  const legacyWarning = `Historical v1 output is retained as originally computed and is not reinterpreted as current v2 output. ${projection.warnings.join(" ")}`;

  addTable(tables, {
    id: "legacy_logistic_coefficients",
    title: "Legacy v1 coefficients and odds ratios",
    warning: legacyWarning,
    columns: ["Term", "Estimate", "ML SE", "Wald z", "p (two-sided)", "95% CI lower", "95% CI upper", "Odds ratio"],
    rows: projection.coefficients.map((row) => [
      row.term === "intercept" ? "Intercept" : row.term,
      formatNumber(row.estimate),
      formatNumber(row.standard_error),
      formatNumber(row.statistic),
      formatPValue(row.p_value_two_sided),
      formatNumber(row.confidence_interval_lower),
      formatNumber(row.confidence_interval_upper),
      formatNumber(row.odds_ratio!),
    ]),
  });
  addTable(tables, {
    id: "legacy_logistic_fit",
    title: "Legacy v1 model fit",
    warning: legacyWarning,
    columns: ["Metric", "Value"],
    rows: [
      ["Analyzed observations", String(projection.observations)],
      ["Log likelihood", formatNumber(projection.fit.log_likelihood!)],
      ["McFadden pseudo-R²", formatNumber(projection.fit.pseudo_r_squared!)],
      ["AIC", formatNumber(projection.fit.aic)],
      ["BIC", formatNumber(projection.fit.bic)],
    ],
  });
  addTable(tables, {
    id: "legacy_logistic_probabilities",
    title: "Legacy v1 fitted probabilities",
    warning: legacyWarning,
    columns: ["Complete-case observation", "Fitted probability", "Residual"],
    rows: projection.predictions.map((row) => [String(row.observation + 1), formatNumber(row.probability!), formatNumber(row.residual!)]),
  });
  addTable(tables, {
    id: "legacy_logistic_scope",
    title: "Legacy binary logistic regression (v1)",
    warning: legacyWarning,
    columns: ["Field", "Value"],
    rows: [
      ["Outcome", projection.outcome],
      ["Predictors", projection.predictors.join(", ")],
      ["Controls", projection.controls.length ? projection.controls.join(", ") : "None"],
      ["Recorded historical preprocessing", projection.recordedPreprocessing],
      ["Historical preprocessing handling", "Recorded for archive provenance only; non-operative for this preserved v1 result"],
      ["Historical handling", "Readable and exportable under its original version; new analyses use the current method"],
      ["Method version", projection.methodVersion],
    ],
  });
}

function addMgaResultTables(
  tables: ResultTable[],
  run: AnalysisRun & { result: NonNullable<AnalysisRun["result"]> },
  constructLabel: ConstructDisplayLabel,
) {
  const mga = run.result.mga;
  if (!mga) return;

  const pair = resolveMgaGroupPair(mga);
  if (!pair) return;
  const [groupA, groupB] = pair;
  if (![groupA.observations, groupB.observations].every(isNonNegativeInteger)) return;

  const roleForGroup = (group: string) => group === groupA.group ? "Group A" : "Group B";
  const isCurrentMga = mga.method_version === CURRENT_MGA_METHOD_VERSION;
  const isHistoricalV3 = mga.method_version === HISTORICAL_MGA_METHOD_VERSION_V3;
  const isHistoricalV2 = mga.method_version === HISTORICAL_MGA_METHOD_VERSION_V2;
  const hasCompleteMeasurementContract = isCurrentMga || isHistoricalV3 || isHistoricalV2;
  const expectedPermutationVersion = isCurrentMga
    ? CURRENT_MGA_PERMUTATION_METHOD_VERSION
    : isHistoricalV3
      ? HISTORICAL_MGA_PERMUTATION_METHOD_VERSION_V3
      : HISTORICAL_MGA_PERMUTATION_METHOD_VERSION_V2;
  const expectedMicomVersion = isCurrentMga
    ? CURRENT_COMBINED_MICOM_METHOD_VERSION
    : isHistoricalV3
      ? LEGACY_COMBINED_MICOM_METHOD_VERSION_V3
      : HISTORICAL_MICOM_METHOD_VERSION_V2;
  const permutation = hasCompleteMeasurementContract
    ? currentMgaPermutation(run.result.mga_permutation, mga.group_column, expectedPermutationVersion)
    : null;
  const groupAOuter = validMgaOuterEstimates(groupA);
  const groupBOuter = validMgaOuterEstimates(groupB);
  const measurementComparisons = hasCompleteMeasurementContract
    ? validMgaMeasurementComparisons(mga, groupA, groupB, groupAOuter, groupBOuter)
    : [];
  const micom = hasCompleteMeasurementContract
    ? currentMicomProjection(run.result.micom, mga, groupA, groupB, groupAOuter, groupBOuter, expectedMicomVersion)
    : null;
  if (isCurrentMga && (!permutation
    || !micom
    || permutation.permutation_plan_sha256 !== micom.analysis.permutation_plan_sha256
    || JSON.stringify(permutation.permutation_ledger) !== JSON.stringify(micom.analysis.permutation_ledger))) return;
  const engineWarnings = [
    ...(isHistoricalV3 ? [HISTORICAL_MGA_V3_WARNING] : []),
    ...(isHistoricalV2 ? [HISTORICAL_MGA_V2_WARNING] : []),
    ...mga.warnings,
    ...(permutation?.warnings ?? []),
    ...(micom?.analysis.warnings ?? []),
  ]
    .map((warning) => warning.trim())
    .filter((warning) => warning.length > 0);
  const warning = [...new Set(engineWarnings)].join(" ") || null;

  addTable(tables, {
    id: "mga_group_summary",
    title: "Two-group sample summary",
    warning,
    columns: ["Group column", "Role", "Group value", "Analyzed observations"],
    rows: [groupA, groupB].map((group) => [
      mga.group_column,
      roleForGroup(group.group),
      group.group,
      String(group.observations),
    ]),
  });

  if (micom) {
    addMicomResultTables(tables, micom, groupA.group, groupB.group, constructLabel);
    if (isCurrentMga) addMicomAccountingTable(tables, micom.analysis);
  }

  addTable(tables, {
    id: "mga_group_paths",
    title: "Group path coefficients",
    warning: null,
    columns: ["Role", "Group value", "Path", "Coefficient"],
    rows: [groupA, groupB].flatMap((group) => group.paths
      .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.coefficient))
      .map((row) => [
        roleForGroup(group.group),
        group.group,
        constructPathLabel([row.source, row.target], constructLabel),
        formatNumber(row.coefficient),
      ])),
  });

  if (hasCompleteMeasurementContract) {
    addTable(tables, {
      id: "mga_group_loadings",
      title: "Group outer loadings",
      warning: null,
      columns: ["Role", "Group value", "Construct", "Indicator", "Outer loading"],
      rows: [
        ...groupAOuter.map((row) => ["Group A", groupA.group, constructLabel(row.construct), row.indicator, formatNumber(row.loading)]),
        ...groupBOuter.map((row) => ["Group B", groupB.group, constructLabel(row.construct), row.indicator, formatNumber(row.loading)]),
      ],
    });

    addTable(tables, {
      id: "mga_group_weights",
      title: "Group outer weights",
      warning: null,
      columns: ["Role", "Group value", "Construct", "Indicator", "Outer weight"],
      rows: [
        ...groupAOuter.map((row) => ["Group A", groupA.group, constructLabel(row.construct), row.indicator, formatNumber(row.weight)]),
        ...groupBOuter.map((row) => ["Group B", groupB.group, constructLabel(row.construct), row.indicator, formatNumber(row.weight)]),
      ],
    });
  }

  addTable(tables, {
    id: "mga_group_r_squared",
    title: "Group R-square",
    warning: null,
    columns: ["Role", "Group value", "Construct", "R²"],
    rows: [groupA, groupB].flatMap((group) => Object.entries(group.r_squared)
      .filter(([construct, value]) => hasText(construct) && isFiniteNumber(value))
      .map(([construct, value]) => [
        roleForGroup(group.group),
        group.group,
        constructLabel(construct),
        formatNumber(value),
      ])),
  });

  const groupAPaths = new Map(groupA.paths
    .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.coefficient))
    .map((row) => [effectPairKey(row.source, row.target), row.coefficient]));
  const groupBPaths = new Map(groupB.paths
    .filter((row) => hasText(row.source) && hasText(row.target) && isFiniteNumber(row.coefficient))
    .map((row) => [effectPairKey(row.source, row.target), row.coefficient]));
  const comparisons = mga.comparisons.filter((row) => {
    if (row.group_a !== groupA.group || row.group_b !== groupB.group) return false;
    if (!hasText(row.source) || !hasText(row.target)) return false;
    if (![row.coefficient_a, row.coefficient_b, row.difference].every(isFiniteNumber)) return false;
    const key = effectPairKey(row.source, row.target);
    const coefficientA = groupAPaths.get(key);
    const coefficientB = groupBPaths.get(key);
    return isFiniteNumber(coefficientA)
      && isFiniteNumber(coefficientB)
      && numbersClose(row.coefficient_a, coefficientA)
      && numbersClose(row.coefficient_b, coefficientB)
      && numbersClose(row.difference, coefficientA - coefficientB);
  });

  addTable(tables, {
    id: "mga_path_differences",
    title: "Group A minus Group B path differences",
    warning: null,
    columns: ["Path", "Group A", "Coefficient A", "Group B", "Coefficient B", "A − B"],
    rows: comparisons.map((row) => [
      constructPathLabel([row.source, row.target], constructLabel),
      row.group_a,
      formatNumber(row.coefficient_a),
      row.group_b,
      formatNumber(row.coefficient_b),
      formatNumber(row.difference),
    ]),
  });

  addMgaMeasurementDifferenceTable(
    tables,
    "outer_loading",
    "mga_loading_differences",
    "Group A minus Group B outer-loading differences",
    "Outer loading",
    measurementComparisons,
    constructLabel,
  );
  addMgaMeasurementDifferenceTable(
    tables,
    "outer_weight",
    "mga_weight_differences",
    "Group A minus Group B outer-weight differences",
    "Outer weight",
    measurementComparisons,
    constructLabel,
  );

  if (!permutation) return;

  const comparisonDifferences = new Map(comparisons.map((row) => [effectPairKey(row.source, row.target), row.difference]));
  const permutationRows = uniqueValidRows(
    permutation.comparisons,
    (row) => effectPairKey(row.source, row.target),
    (row) => hasText(row.source)
      && hasText(row.target)
      && numbersClose(row.original_difference, comparisonDifferences.get(effectPairKey(row.source, row.target)))
      && isFiniteNumber(row.original_difference)
      && isProbability(row.empirical_p_value_two_sided),
  );
  const includePercentileRank = permutationRows.length > 0
    && permutationRows.every((row) => isProbability(row.percentile_rank));
  addTable(tables, {
    id: "mga_permutation",
    title: "Two-tailed permutation path differences",
    warning: null,
    columns: [
      "Path",
      "A − B",
      "Two-tailed p",
      ...(includePercentileRank ? ["Percentile rank"] : []),
      "Requested permutations",
      "Usable permutations",
    ],
    rows: permutationRows.map((row) => [
      constructPathLabel([row.source, row.target], constructLabel),
      formatNumber(row.original_difference),
      formatPValue(row.empirical_p_value_two_sided as number),
      ...(includePercentileRank ? [formatNumber(row.percentile_rank as number, 4)] : []),
      String(permutation.permutation_samples),
      String(permutation.usable_permutations),
    ]),
  });

  const measurementDifferenceByKey = new Map(measurementComparisons.map((row) => [
    mgaMeasurementKey(row.parameter, row.construct, row.indicator),
    row.difference,
  ]));
  const permutationMeasurementRows = uniqueValidRows(
    permutation.measurement_comparisons ?? [],
    (row) => mgaMeasurementKey(row.parameter, row.construct, row.indicator),
    (row) => (row.parameter === "outer_loading" || row.parameter === "outer_weight")
      && hasText(row.construct)
      && hasText(row.indicator)
      && numbersClose(
        row.original_difference,
        measurementDifferenceByKey.get(mgaMeasurementKey(row.parameter, row.construct, row.indicator)),
      )
      && isProbability(row.empirical_p_value_two_sided)
      && isProbability(row.percentile_rank),
  );
  addMgaMeasurementPermutationTable(
    tables,
    "outer_loading",
    "mga_permutation_loadings",
    "Two-tailed permutation outer-loading differences",
    permutationMeasurementRows,
    permutation.permutation_samples,
    permutation.usable_permutations,
    constructLabel,
  );
  addMgaMeasurementPermutationTable(
    tables,
    "outer_weight",
    "mga_permutation_weights",
    "Two-tailed permutation outer-weight differences",
    permutationMeasurementRows,
    permutation.permutation_samples,
    permutation.usable_permutations,
    constructLabel,
  );
  if (isCurrentMga) {
    addTable(tables, {
      id: "mga_permutation_accounting",
      title: "Combined permutation plan and provenance",
      warning: null,
      columns: ["Field", "Value"],
      rows: [
        ["MGA method version", mga.method_version],
        ["Permutation method version", permutation.method_version],
        ["MICOM method version", micom?.analysis.method_version ?? "Unavailable"],
        ["Requested partitions", String(permutation.permutation_samples)],
        ["Attempted partitions", String(permutation.attempted_permutations)],
        ["Usable MGA partitions", String(permutation.usable_permutations)],
        ["Failed MGA partitions", String(permutation.failed_permutations)],
        ["Retry policy", permutation.retry_policy ?? "Unavailable"],
        ["Partition plan digest", permutation.permutation_plan_sha256 ?? "Unavailable"],
        ["Seed", String(run.provenance?.settings.seed ?? run.seed)],
      ],
    });
  }
}

type MgaAnalysis = NonNullable<NonNullable<AnalysisRun["result"]>["mga"]>;
type MgaGroup = MgaAnalysis["groups"][number];
type MgaOuterEstimate = NonNullable<MgaGroup["outer_estimates"]>[number];
type MgaMeasurementComparison = NonNullable<MgaAnalysis["measurement_comparisons"]>[number];
type MgaPermutationAnalysis = NonNullable<NonNullable<AnalysisRun["result"]>["mga_permutation"]>;
type MgaPermutationMeasurementComparison = NonNullable<MgaPermutationAnalysis["measurement_comparisons"]>[number];
type MicomAnalysisPayload = NonNullable<NonNullable<AnalysisRun["result"]>["micom"]>;
type MicomConstruct = MicomAnalysisPayload["constructs"][number];

interface CurrentMicomProjection {
  analysis: MicomAnalysisPayload;
  constructs: MicomConstruct[];
}

function currentStandaloneMicomProjection(
  micom: NonNullable<AnalysisRun["result"]>["micom"],
  expectedMethodVersion = CURRENT_MICOM_METHOD_VERSION,
): CurrentMicomProjection | null {
  if (!micom || micom.method_version !== expectedMethodVersion) return null;
  const ledger = micom.permutation_ledger ?? [];
  const step2Usable = micom.step2_usable_permutations;
  const step2Failed = micom.step2_failed_permutations;
  const step3Usable = micom.step3_usable_permutations;
  const step3Failed = micom.step3_failed_permutations;
  if (!hasText(micom.group_column)
    || !Array.isArray(micom.warnings)
    || micom.warnings.some((warning) => typeof warning !== "string")
    || !isPositiveInteger(micom.permutation_samples)
    || micom.permutation_samples < 5_000
    || micom.permutation_samples > 10_000
    || micom.attempted_permutations !== micom.permutation_samples
    || micom.retry_policy !== "none"
    || micom.step1_status !== "confirmed_by_researcher_review"
    || micom.step1_computed !== false
    || !isNonNegativeInteger(step2Usable)
    || !isNonNegativeInteger(step2Failed)
    || !isNonNegativeInteger(step3Usable)
    || !isNonNegativeInteger(step3Failed)
    || step2Usable + step2Failed !== micom.permutation_samples
    || step3Usable + step3Failed !== micom.permutation_samples
    || step2Usable < 19
    || step3Usable < 19
    || micom.usable_permutations !== Math.min(step2Usable, step3Usable)
    || micom.failed_permutations !== micom.permutation_samples - micom.usable_permutations
    || !/^sha256:[0-9a-f]{64}$/.test(micom.permutation_plan_sha256 ?? "")
    || ledger.length !== micom.permutation_samples
    || micom.groups.length !== 2
    || !hasText(micom.groups[0]?.group)
    || !hasText(micom.groups[1]?.group)
    || micom.groups[0].group === micom.groups[1].group
    || !isPositiveInteger(micom.groups[0].observations)
    || !isPositiveInteger(micom.groups[1].observations)
    || Math.min(micom.groups[0].observations, micom.groups[1].observations) < 10
    || Math.max(micom.groups[0].observations, micom.groups[1].observations)
      > Math.min(micom.groups[0].observations, micom.groups[1].observations) * 10
    || !isProbability(micom.confidence_level)
    || micom.confidence_level <= 0
    || micom.confidence_level >= 1) return null;

  let countedStep2 = 0;
  let countedStep3 = 0;
  for (let replicate = 0; replicate < ledger.length; replicate += 1) {
    const entry = ledger[replicate];
    const step2Ok = entry.step2_status === "usable";
    const step3Ok = entry.step3_status === "usable";
    if (entry.replicate !== replicate
      || !/^[0-9a-f]{64}$/.test(entry.partition_sha256)
      || entry.group_a_rows !== micom.groups[0].observations
      || entry.group_b_rows !== micom.groups[1].observations
      || (entry.step2_status !== "usable" && entry.step2_status !== "failed")
      || (entry.step3_status !== "usable" && entry.step3_status !== "failed")
      || (step2Ok
        ? entry.step2_failure_code != null
        : !hasText(entry.step2_failure_code) || !CURRENT_MICOM_FAILURE_CODES.has(entry.step2_failure_code))
      || (step3Ok
        ? entry.step3_failure_code != null
        : !hasText(entry.step3_failure_code) || !CURRENT_MICOM_FAILURE_CODES.has(entry.step3_failure_code))) return null;
    if (step2Ok) countedStep2 += 1;
    if (step3Ok) countedStep3 += 1;
  }
  if (countedStep2 !== step2Usable || countedStep3 !== step3Usable) return null;

  const commonConstructs = new Set(micom.constructs.map((row) => row.construct));
  const constructs = uniqueValidRows(
    micom.constructs,
    (row) => row.construct,
    (row) => validMicomConstruct(row, commonConstructs),
  );
  if (!constructs.length || constructs.length !== micom.constructs.length) return null;
  return { analysis: micom, constructs };
}

function currentMgaPermutation(
  permutation: NonNullable<AnalysisRun["result"]>["mga_permutation"],
  groupColumn: string,
  expectedMethodVersion: string,
): MgaPermutationAnalysis | null {
  if (!permutation
    || permutation.method_version !== expectedMethodVersion
    || permutation.group_column !== groupColumn
    || !isPositiveInteger(permutation.permutation_samples)
    || permutation.permutation_samples < 5_000
    || permutation.permutation_samples > 10_000
    || !isNonNegativeInteger(permutation.attempted_permutations)
    || !isNonNegativeInteger(permutation.failed_permutations)
    || permutation.attempted_permutations < permutation.usable_permutations
    || permutation.attempted_permutations - permutation.usable_permutations !== permutation.failed_permutations) return null;
  if (expectedMethodVersion === CURRENT_MGA_PERMUTATION_METHOD_VERSION) {
    const ledger = permutation.permutation_ledger ?? [];
    const usable = ledger.filter((entry) => entry.step2_status === "usable").length;
    if (permutation.attempted_permutations !== permutation.permutation_samples
      || permutation.retry_policy !== "none"
      || !/^sha256:[0-9a-f]{64}$/.test(permutation.permutation_plan_sha256 ?? "")
      || ledger.length !== permutation.permutation_samples
      || usable !== permutation.usable_permutations
      || permutation.failed_permutations !== permutation.permutation_samples - usable
      || ledger.some((entry, replicate) => entry.replicate !== replicate
        || !/^[0-9a-f]{64}$/.test(entry.partition_sha256)
        || (entry.step2_status === "usable") !== (entry.step2_failure_code == null))) return null;
  } else if (permutation.usable_permutations !== permutation.permutation_samples) return null;
  return permutation;
}

function validMgaOuterEstimates(group: MgaGroup): MgaOuterEstimate[] {
  return uniqueValidRows(
    group.outer_estimates ?? [],
    (row) => mgaOuterKey(row.construct, row.indicator),
    (row) => hasText(row.construct)
      && hasText(row.indicator)
      && isFiniteNumber(row.loading)
      && isFiniteNumber(row.weight),
  );
}

function validMgaMeasurementComparisons(
  mga: MgaAnalysis,
  groupA: MgaGroup,
  groupB: MgaGroup,
  groupAOuter: MgaOuterEstimate[],
  groupBOuter: MgaOuterEstimate[],
): MgaMeasurementComparison[] {
  const groupAByKey = new Map(groupAOuter.map((row) => [mgaOuterKey(row.construct, row.indicator), row]));
  const groupBByKey = new Map(groupBOuter.map((row) => [mgaOuterKey(row.construct, row.indicator), row]));
  return uniqueValidRows(
    mga.measurement_comparisons ?? [],
    (row) => mgaMeasurementKey(row.parameter, row.construct, row.indicator),
    (row) => {
      if ((row.parameter !== "outer_loading" && row.parameter !== "outer_weight")
        || row.group_a !== groupA.group
        || row.group_b !== groupB.group
        || !hasText(row.construct)
        || !hasText(row.indicator)) return false;
      const outerKey = mgaOuterKey(row.construct, row.indicator);
      const estimateA = groupAByKey.get(outerKey);
      const estimateB = groupBByKey.get(outerKey);
      if (!estimateA || !estimateB) return false;
      const valueA = row.parameter === "outer_loading" ? estimateA.loading : estimateA.weight;
      const valueB = row.parameter === "outer_loading" ? estimateB.loading : estimateB.weight;
      return numbersClose(row.estimate_a, valueA)
        && numbersClose(row.estimate_b, valueB)
        && numbersClose(row.difference, valueA - valueB);
    },
  );
}

function currentMicomProjection(
  micom: NonNullable<AnalysisRun["result"]>["micom"],
  mga: MgaAnalysis,
  groupA: MgaGroup,
  groupB: MgaGroup,
  groupAOuter: MgaOuterEstimate[],
  groupBOuter: MgaOuterEstimate[],
  expectedMethodVersion: string,
): CurrentMicomProjection | null {
  const exactCombined = expectedMethodVersion === CURRENT_COMBINED_MICOM_METHOD_VERSION;
  const exactProjection = exactCombined
    ? currentStandaloneMicomProjection(micom, expectedMethodVersion)
    : null;
  if (!micom
    || micom.method_version !== expectedMethodVersion
    || micom.group_column !== mga.group_column
    || !isPositiveInteger(micom.permutation_samples)
    || micom.permutation_samples < 5_000
    || micom.permutation_samples > 10_000
    || (exactCombined ? !exactProjection : micom.usable_permutations !== micom.permutation_samples)
    || !isNonNegativeInteger(micom.attempted_permutations)
    || !isNonNegativeInteger(micom.failed_permutations)
    || micom.attempted_permutations < micom.usable_permutations
    || micom.attempted_permutations - micom.usable_permutations !== micom.failed_permutations
    || !isProbability(micom.confidence_level)
    || micom.confidence_level <= 0
    || micom.confidence_level >= 1
    || micom.groups.length !== 2
    || micom.groups[0]?.group !== groupA.group
    || micom.groups[0]?.observations !== groupA.observations
    || micom.groups[1]?.group !== groupB.group
    || micom.groups[1]?.observations !== groupB.observations) return null;

  const groupAConstructs = new Set(groupAOuter.map((row) => row.construct));
  const groupBConstructs = new Set(groupBOuter.map((row) => row.construct));
  const commonConstructs = new Set([...groupAConstructs].filter((construct) => groupBConstructs.has(construct)));
  const constructs = uniqueValidRows(
    micom.constructs,
    (row) => row.construct,
    (row) => validMicomConstruct(row, commonConstructs),
  );
  if (!constructs.length
    || constructs.length !== micom.constructs.length
    || constructs.length !== commonConstructs.size) return null;
  return { analysis: micom, constructs };
}

function validMicomConstruct(row: MicomConstruct, commonConstructs: ReadonlySet<string>): boolean {
  if (!hasText(row.construct)
    || !commonConstructs.has(row.construct)
    || row.configural_invariance !== true
    || !isFiniteNumber(row.compositional_correlation)
    || row.compositional_correlation < -1
    || row.compositional_correlation > 1
    || !isFiniteNumber(row.compositional_correlation_lower)
    || row.compositional_correlation_lower < -1
    || row.compositional_correlation_lower > 1
    || !isProbability(row.compositional_p_value)
    || !isFiniteNumber(row.mean_a)
    || !isFiniteNumber(row.mean_b)
    || !isFiniteNumber(row.mean_difference)
    || !numbersClose(row.mean_difference, row.mean_a - row.mean_b)
    || !isFiniteNumber(row.mean_difference_lower)
    || !isFiniteNumber(row.mean_difference_upper)
    || row.mean_difference_lower > row.mean_difference_upper
    || !isProbability(row.mean_p_value)
    || !isFiniteNumber(row.variance_a)
    || row.variance_a <= 0
    || !isFiniteNumber(row.variance_b)
    || row.variance_b <= 0
    || !isFiniteNumber(row.variance_difference)
    || !numbersClose(row.variance_difference, Math.log(row.variance_a) - Math.log(row.variance_b))
    || !isFiniteNumber(row.variance_difference_lower)
    || !isFiniteNumber(row.variance_difference_upper)
    || row.variance_difference_lower > row.variance_difference_upper
    || !isProbability(row.variance_p_value)
    || typeof row.equal_means !== "boolean"
    || typeof row.equal_variances !== "boolean") return false;

  const compositional = greaterThanOrClose(row.compositional_correlation, row.compositional_correlation_lower);
  const equalMeans = inClosedInterval(row.mean_difference, row.mean_difference_lower, row.mean_difference_upper);
  const equalVariances = inClosedInterval(row.variance_difference, row.variance_difference_lower, row.variance_difference_upper);
  return row.partial_invariance === compositional
    && row.equal_means === equalMeans
    && row.equal_variances === equalVariances
    && row.full_invariance === (compositional && equalMeans && equalVariances);
}

function addMicomResultTables(
  tables: ResultTable[],
  projection: CurrentMicomProjection,
  groupA: string,
  groupB: string,
  constructLabel: ConstructDisplayLabel,
) {
  const { analysis, constructs } = projection;
  const confidence = `${formatNumber((analysis.confidence_level as number) * 100, 1)}%`;
  addTable(tables, {
    id: "micom_summary",
    title: "MICOM invariance summary",
    warning: null,
    columns: ["Construct", "Configural", "Compositional", "Partial invariance", "Equal means", "Equal variances", "Full invariance", "Confidence", "Usable permutations"],
    rows: constructs.map((row) => [
      constructLabel(row.construct),
      "Confirmed",
      establishedLabel(row.partial_invariance),
      establishedLabel(row.partial_invariance),
      equalityLabel(row.equal_means === true),
      equalityLabel(row.equal_variances === true),
      establishedLabel(row.full_invariance),
      confidence,
      String(analysis.usable_permutations),
    ]),
  });
  addTable(tables, {
    id: "micom_configural",
    title: "MICOM Step 1 - configural invariance",
    warning: null,
    columns: ["Construct", "Configural invariance"],
    rows: constructs.map((row) => [constructLabel(row.construct), "Confirmed"]),
  });
  addTable(tables, {
    id: "micom_composition",
    title: "MICOM Step 2 - compositional invariance",
    warning: null,
    columns: ["Construct", "Original correlation", "Lower confidence bound", "Permutation p", "Compositional invariance"],
    rows: constructs.map((row) => [
      constructLabel(row.construct),
      formatNumber(row.compositional_correlation),
      formatNumber(row.compositional_correlation_lower as number),
      formatPValue(row.compositional_p_value as number),
      establishedLabel(row.partial_invariance),
    ]),
  });
  addTable(tables, {
    id: "micom_means",
    title: "MICOM Step 3 - equality of composite means",
    warning: null,
    columns: ["Construct", `Mean ${groupA}`, `Mean ${groupB}`, `Mean difference (${groupA} - ${groupB})`, "Lower confidence bound", "Upper confidence bound", "Two-tailed p", "Equal means"],
    rows: constructs.map((row) => [
      constructLabel(row.construct),
      formatNumber(row.mean_a as number),
      formatNumber(row.mean_b as number),
      formatNumber(row.mean_difference),
      formatNumber(row.mean_difference_lower as number),
      formatNumber(row.mean_difference_upper as number),
      formatPValue(row.mean_p_value as number),
      equalityLabel(row.equal_means === true),
    ]),
  });
  addTable(tables, {
    id: "micom_variances",
    title: "MICOM Step 3 - equality of composite variances",
    warning: null,
    columns: ["Construct", `Variance ${groupA}`, `Variance ${groupB}`, `Log variance ratio (${groupA}/${groupB})`, "Lower confidence bound", "Upper confidence bound", "Two-tailed p", "Equal variances"],
    rows: constructs.map((row) => [
      constructLabel(row.construct),
      formatNumber(row.variance_a as number),
      formatNumber(row.variance_b as number),
      formatNumber(row.variance_difference),
      formatNumber(row.variance_difference_lower as number),
      formatNumber(row.variance_difference_upper as number),
      formatPValue(row.variance_p_value as number),
      equalityLabel(row.equal_variances === true),
    ]),
  });
}

function addMicomAccountingTable(tables: ResultTable[], analysis: MicomAnalysisPayload) {
  addTable(tables, {
    id: "micom_permutation_accounting",
    title: "MICOM permutation accounting",
    warning: analysis.warnings.map((warning) => warning.trim()).filter(Boolean).join(" ") || null,
    columns: ["Field", "Value"],
    rows: [
      ["Step 1", "Confirmed by researcher review; not computed from the data"],
      ["Requested permutations", String(analysis.permutation_samples)],
      ["Attempted permutations", String(analysis.attempted_permutations)],
      ["Retry policy", analysis.retry_policy ?? "Unavailable"],
      ["Step 2 usable", String(analysis.step2_usable_permutations)],
      ["Step 2 failed", String(analysis.step2_failed_permutations)],
      ["Step 3 usable", String(analysis.step3_usable_permutations)],
      ["Step 3 failed", String(analysis.step3_failed_permutations)],
      ["Ledger rows", String(analysis.permutation_ledger?.length ?? 0)],
      ["Permutation plan", analysis.permutation_plan_sha256 ?? "Unavailable"],
    ],
  });
}

function addMgaMeasurementDifferenceTable(
  tables: ResultTable[],
  parameter: "outer_loading" | "outer_weight",
  id: string,
  title: string,
  estimateLabel: string,
  comparisons: MgaMeasurementComparison[],
  constructLabel: ConstructDisplayLabel,
) {
  addTable(tables, {
    id,
    title,
    warning: null,
    columns: ["Construct", "Indicator", "Group A", `${estimateLabel} A`, "Group B", `${estimateLabel} B`, "A - B"],
    rows: comparisons
      .filter((row) => row.parameter === parameter)
      .map((row) => [
        constructLabel(row.construct),
        row.indicator,
        row.group_a,
        formatNumber(row.estimate_a),
        row.group_b,
        formatNumber(row.estimate_b),
        formatNumber(row.difference),
      ]),
  });
}

function addMgaMeasurementPermutationTable(
  tables: ResultTable[],
  parameter: "outer_loading" | "outer_weight",
  id: string,
  title: string,
  rows: MgaPermutationMeasurementComparison[],
  requestedPermutations: number,
  usablePermutations: number,
  constructLabel: ConstructDisplayLabel,
) {
  addTable(tables, {
    id,
    title,
    warning: null,
    columns: ["Construct", "Indicator", "A - B", "Two-tailed p", "Percentile rank", "Requested permutations", "Usable permutations"],
    rows: rows
      .filter((row) => row.parameter === parameter)
      .map((row) => [
        constructLabel(row.construct),
        row.indicator,
        formatNumber(row.original_difference),
        formatPValue(row.empirical_p_value_two_sided as number),
        formatNumber(row.percentile_rank as number, 4),
        String(requestedPermutations),
        String(usablePermutations),
      ]),
  });
}

function uniqueValidRows<T>(
  rows: readonly T[],
  identity: (row: T) => string,
  valid: (row: T) => boolean,
): T[] {
  const candidates = rows.filter(valid);
  const counts = new Map<string, number>();
  for (const row of candidates) counts.set(identity(row), (counts.get(identity(row)) ?? 0) + 1);
  return candidates.filter((row) => counts.get(identity(row)) === 1);
}

function mgaOuterKey(construct: string, indicator: string): string {
  return `${construct}\u0000${indicator}`;
}

function mgaMeasurementKey(parameter: string, construct: string, indicator: string): string {
  return `${parameter}\u0000${mgaOuterKey(construct, indicator)}`;
}

function greaterThanOrClose(value: number, lower: number): boolean {
  return value > lower || numbersClose(value, lower);
}

function inClosedInterval(value: number, lower: number, upper: number): boolean {
  return (value > lower || numbersClose(value, lower))
    && (value < upper || numbersClose(value, upper));
}

function establishedLabel(value: boolean): string {
  return value ? "Established" : "Not established";
}

function equalityLabel(value: boolean): string {
  return value ? "Equal" : "Different";
}

function resolveMgaGroupPair(
  mga: NonNullable<NonNullable<AnalysisRun["result"]>["mga"]>,
): [(typeof mga.groups)[number], (typeof mga.groups)[number]] | null {
  const comparison = mga.comparisons.find((row) =>
    hasText(row.group_a) && hasText(row.group_b) && row.group_a !== row.group_b,
  );
  const groupAValue = comparison?.group_a ?? mga.groups[0]?.group;
  const groupBValue = comparison?.group_b ?? mga.groups.find((group) => group.group !== groupAValue)?.group;
  if (!hasText(groupAValue) || !hasText(groupBValue) || groupAValue === groupBValue) return null;
  const groupA = mga.groups.find((group) => group.group === groupAValue);
  const groupB = mga.groups.find((group) => group.group === groupBValue);
  return groupA && groupB ? [groupA, groupB] : null;
}

function addTable(tables: ResultTable[], draft: TableDraft) {
  if (!draft.columns.length) return;
  const rows = draft.rows.filter((row) => row.length === draft.columns.length && row.some(hasText));
  if (!rows.length) return;
  tables.push({ ...draft, status: draft.status ?? "validated", rows });
}

function visibleResultWarning(
  warnings: readonly string[],
  methodGuidance: readonly string[],
): string | null {
  const guidance = new Set(methodGuidance);
  const visible = warnings
    .map((warning) => warning.trim())
    .filter((warning) => warning.length > 0 && !guidance.has(warning));
  return visible.join(" ") || null;
}

function finiteRecordRows(record: Record<string, number>, constructLabel: ConstructDisplayLabel): string[][] {
  return Object.entries(record)
    .filter(([name, value]) => hasText(name) && isFiniteNumber(value))
    .map(([name, value]) => [constructLabel(name), formatNumber(value)]);
}

function numericMatrixRows(
  constructs: string[],
  values: Array<Array<number | null>>,
  includeDiagonal: boolean,
  constructLabel: ConstructDisplayLabel,
  excludedConstructs: ReadonlySet<string> = new Set(),
): string[][] {
  const rows: string[][] = [];
  for (let rowIndex = 0; rowIndex < constructs.length; rowIndex += 1) {
    const rowName = constructs[rowIndex];
    if (!hasText(rowName) || excludedConstructs.has(rowName)) continue;
    const finalColumn = includeDiagonal ? rowIndex : rowIndex - 1;
    for (let columnIndex = 0; columnIndex <= finalColumn; columnIndex += 1) {
      const columnName = constructs[columnIndex];
      const value = firstFinite(values[rowIndex]?.[columnIndex], values[columnIndex]?.[rowIndex]);
      if (hasText(columnName) && !excludedConstructs.has(columnName) && value != null) rows.push([constructLabel(rowName), constructLabel(columnName), formatNumber(value)]);
    }
  }
  return rows;
}

function htmtTable(
  id: "htmt_plus" | "htmt_original",
  title: string,
  artifact: HtmtAssessment,
  constructLabel: ConstructDisplayLabel,
  excludedConstructs: ReadonlySet<string> = new Set(),
): TableDraft {
  const rows: string[][] = [];
  for (let rowIndex = 1; rowIndex < artifact.constructs.length; rowIndex += 1) {
    const rowName = artifact.constructs[rowIndex];
    if (!hasText(rowName) || excludedConstructs.has(rowName)) continue;
    for (let columnIndex = 0; columnIndex < rowIndex; columnIndex += 1) {
      const columnName = artifact.constructs[columnIndex];
      const forward = artifact.cells[rowIndex]?.[columnIndex];
      const reverse = artifact.cells[columnIndex]?.[rowIndex];
      const cell = [forward, reverse].find((candidate) => candidate?.status === "available" && isFiniteNumber(candidate.value));
      if (hasText(columnName) && !excludedConstructs.has(columnName) && cell && isFiniteNumber(cell.value)) {
        rows.push([constructLabel(rowName), constructLabel(columnName), formatNumber(cell.value)]);
      }
    }
  }
  return { id, title, status: "experimental", warning: null, columns: ["Construct", "Compared with", "Value"], rows };
}

function htmtBootstrapTables(
  plus: HtmtBootstrapInference,
  original: HtmtBootstrapInference,
  assessment: NonNullable<AnalysisRun["assessment"]>,
  requestedReplicates: number,
  globallyFailedReplicateIndices: number[],
  constructLabel: ConstructDisplayLabel,
  excludedConstructs: ReadonlySet<string>,
): TableDraft[] | null {
  const pointPlus = assessment.htmt_plus;
  const pointOriginal = assessment.htmt_original;
  if (!pointPlus || !pointOriginal
    || assessment.htmt_plus_method_version !== "ringle_et_al_htmt_plus_v1"
    || assessment.htmt_original_method_version !== "henseler_et_al_htmt_v1"
    || !validHtmtBootstrapArtifact(
      plus,
      pointPlus,
      requestedReplicates,
      "ringle_et_al_htmt_plus_bias_corrected_bootstrap_v1",
      "ringle_et_al_htmt_plus_v1",
      true,
      globallyFailedReplicateIndices,
    )
    || !validHtmtBootstrapArtifact(
      original,
      pointOriginal,
      requestedReplicates,
      "henseler_et_al_htmt_bias_corrected_bootstrap_v1",
      "henseler_et_al_htmt_v1",
      false,
      globallyFailedReplicateIndices,
    )) return null;

  const warning = "Experimental HTMT inference. The documented one-tailed alpha .05 decision is established only when the 90% bias-corrected percentile interval's upper bound is strictly below 0.90; justify any stricter context-specific criterion separately.";
  return [
    htmtBootstrapTable("htmt_plus_bootstrap", "HTMT+ bias-corrected bootstrap inference", plus, constructLabel, excludedConstructs, warning),
    htmtBootstrapTable("htmt_original_bootstrap", "Original HTMT bias-corrected bootstrap inference", original, constructLabel, excludedConstructs, null),
  ];
}

function validHtmtBootstrapArtifact(
  artifact: HtmtBootstrapInference,
  point: HtmtAssessment,
  requestedReplicates: number,
  methodVersion: string,
  pointMethodVersion: string,
  absoluteCorrelations: boolean,
  globallyFailedReplicateIndices: number[],
): boolean {
  const dimension = point.constructs.length;
  const minimumUsable = Math.max(2, Math.ceil(requestedReplicates * 0.9));
  if (artifact.method_version !== methodVersion
    || artifact.point_method_version !== pointMethodVersion
    || artifact.correlation_type !== "pearson"
    || artifact.absolute_correlations !== absoluteCorrelations
    || point.correlation_type !== "pearson"
    || point.absolute_correlations !== absoluteCorrelations
    || artifact.interval_method !== "bias_corrected_percentile_type7_v1"
    || artifact.test_type !== "one_tailed_upper"
    || artifact.significance_level !== 0.05
    || artifact.equivalent_two_sided_confidence_level !== 0.9
    || artifact.critical_value !== 0.9
    || artifact.decision_rule !== "bias_corrected_upper_bound_strictly_below_critical_value_v1"
    || artifact.replicate_index_digest_method !== "sha256_u32_le_v1"
    || artifact.retry_policy !== "no_retry_fixed_preplanned_primary_draws_v1"
    || !isPositiveInteger(requestedReplicates)
    || artifact.requested_replicates !== requestedReplicates
    || artifact.minimum_usable_replicates !== minimumUsable
    || artifact.constructs.length !== dimension
    || artifact.constructs.some((construct, index) => construct !== point.constructs[index])
    || artifact.cells.length !== dimension
    || artifact.cells.some((row) => row.length !== dimension)
    || point.cells.length !== dimension
    || point.cells.some((row) => row.length !== dimension)) return false;

  const globallyFailed = new Set(globallyFailedReplicateIndices);
  if (globallyFailed.size !== globallyFailedReplicateIndices.length
    || globallyFailedReplicateIndices.some((index) => !isNonNegativeInteger(index) || index >= requestedReplicates)) return false;

  for (let row = 0; row < dimension; row += 1) {
    for (let column = 0; column < dimension; column += 1) {
      const cell = artifact.cells[row][column];
      const mirror = artifact.cells[column][row];
      if (JSON.stringify(cell) !== JSON.stringify(mirror)
        || !validHtmtBootstrapCell(
          cell,
          point.cells[row][column],
          row === column,
          absoluteCorrelations,
          requestedReplicates,
          minimumUsable,
          globallyFailed,
        )) return false;
    }
  }
  return true;
}

function validHtmtBootstrapCell(
  cell: HtmtBootstrapInference["cells"][number][number],
  point: HtmtAssessment["cells"][number][number],
  diagonal: boolean,
  absoluteCorrelations: boolean,
  requestedReplicates: number,
  minimumUsable: number,
  globallyFailed: ReadonlySet<number>,
): boolean {
  const summariesAbsent = cell.bootstrap_mean == null
    && cell.bias == null
    && cell.standard_error == null
    && cell.bias_correction == null
    && cell.lower == null
    && cell.upper == null
    && cell.replicate_min == null
    && cell.replicate_max == null
    && cell.below_original === 0
    && cell.tied_original === 0;
  const noIndexLedger = cell.usable_replicate_indices_sha256 == null
    && cell.pair_unavailable_replicates.length === 0;
  if (diagonal) {
    return cell.status === "not_applicable"
      && cell.reason === "htmt.bootstrap.diagonal_not_inferred"
      && cell.original === point.value
      && cell.usable_replicates === 0
      && cell.failed_replicates === 0
      && cell.upper_bound_below_critical_value == null
      && noIndexLedger
      && summariesAbsent;
  }
  if (point.status !== "available") {
    return cell.status === point.status
      && cell.reason === point.reason
      && cell.original === point.value
      && cell.usable_replicates === 0
      && cell.failed_replicates === 0
      && cell.upper_bound_below_critical_value == null
      && noIndexLedger
      && summariesAbsent;
  }
  if (!isFiniteNumber(point.value)
    || !numbersClose(cell.original, point.value)
    || !isNonNegativeInteger(cell.usable_replicates)
    || !isNonNegativeInteger(cell.failed_replicates)
    || cell.usable_replicates > requestedReplicates
    || cell.failed_replicates !== requestedReplicates - cell.usable_replicates) return false;
  const pairUnavailable = new Set<number>();
  for (const entry of cell.pair_unavailable_replicates) {
    if (!isNonNegativeInteger(entry.replicate_index)
      || entry.replicate_index >= requestedReplicates
      || globallyFailed.has(entry.replicate_index)
      || pairUnavailable.has(entry.replicate_index)
      || !hasText(entry.reason_code)) return false;
    pairUnavailable.add(entry.replicate_index);
  }
  if (cell.usable_replicates + globallyFailed.size + pairUnavailable.size !== requestedReplicates
    || !/^[0-9a-f]{64}$/.test(cell.usable_replicate_indices_sha256 ?? "")) return false;
  if (cell.status === "unavailable") {
    const validReason = cell.reason === "htmt.bootstrap.insufficient_usable_replicates"
      ? cell.usable_replicates < minimumUsable
      : cell.reason === "htmt.bootstrap.bias_corrected_interval_unavailable"
        && cell.usable_replicates >= minimumUsable;
    return validReason
      && cell.upper_bound_below_critical_value == null
      && summariesAbsent;
  }
  if (cell.status !== "available" || cell.reason != null || cell.usable_replicates < minimumUsable) return false;
  const summaries = [
    cell.bootstrap_mean,
    cell.bias,
    cell.standard_error,
    cell.bias_correction,
    cell.lower,
    cell.upper,
    cell.replicate_min,
    cell.replicate_max,
  ];
  if (!summaries.every(isFiniteNumber)
    || !isNonNegativeInteger(cell.below_original)
    || !isNonNegativeInteger(cell.tied_original)
    || cell.below_original + cell.tied_original > cell.usable_replicates
    || cell.standard_error! < 0
    || cell.replicate_min! > cell.replicate_max!
    || cell.bootstrap_mean! < cell.replicate_min!
    || cell.bootstrap_mean! > cell.replicate_max!
    || cell.lower! < cell.replicate_min!
    || cell.upper! > cell.replicate_max!
    || cell.lower! > cell.upper!
    || cell.upper_bound_below_critical_value !== (cell.upper! < 0.9)
    || !numbersClose(cell.bias, cell.bootstrap_mean! - point.value)) return false;
  if (absoluteCorrelations && [
    cell.original,
    cell.bootstrap_mean,
    cell.lower,
    cell.upper,
    cell.replicate_min,
    cell.replicate_max,
  ].some((value) => value! < 0)) return false;
  return true;
}

function htmtBootstrapTable(
  id: "htmt_plus_bootstrap" | "htmt_original_bootstrap",
  title: string,
  artifact: HtmtBootstrapInference,
  constructLabel: ConstructDisplayLabel,
  excludedConstructs: ReadonlySet<string>,
  warning: string | null,
): TableDraft {
  const rows: string[][] = [];
  for (let row = 1; row < artifact.constructs.length; row += 1) {
    const rowName = artifact.constructs[row];
    if (!hasText(rowName) || excludedConstructs.has(rowName)) continue;
    for (let column = 0; column < row; column += 1) {
      const columnName = artifact.constructs[column];
      if (!hasText(columnName) || excludedConstructs.has(columnName)) continue;
      const cell = artifact.cells[row][column];
      rows.push([
        constructLabel(rowName),
        constructLabel(columnName),
        sentenceCase(cell.status.replaceAll("_", " ")),
        formatOptionalNumber(cell.original),
        formatOptionalNumber(cell.bootstrap_mean),
        formatOptionalNumber(cell.bias),
        formatOptionalNumber(cell.standard_error),
        formatOptionalNumber(cell.bias_correction),
        formatOptionalNumber(cell.lower),
        formatOptionalNumber(cell.upper),
        cell.upper_bound_below_critical_value == null
          ? "Unavailable"
          : cell.upper_bound_below_critical_value
            ? "Established: upper bound < 0.90"
            : "Not established: upper bound ≥ 0.90",
        String(cell.usable_replicates),
        String(cell.failed_replicates),
        String(cell.pair_unavailable_replicates.length),
        cell.usable_replicate_indices_sha256 ?? "",
        htmtBootstrapReason(cell.reason, artifact.minimum_usable_replicates),
      ]);
    }
  }
  return {
    id,
    title,
    status: "experimental",
    warning,
    columns: ["Construct", "Compared with", "Status", "Original", "Bootstrap mean", "Bias", "STDEV", "Bias correction", "BC 90% lower", "BC 90% upper", "Decision at 0.90", "Usable", "Unavailable/failed", "Pair unavailable", "Usable-index digest", "Reason"],
    rows,
  };
}

function htmtBootstrapReason(reason: string | null, minimumUsable: number): string {
  switch (reason) {
    case null: return "";
    case "htmt.bootstrap.insufficient_usable_replicates": return `Fewer than ${minimumUsable} planned replicates produced this comparison.`;
    case "htmt.bootstrap.bias_corrected_interval_unavailable": return "The bias-corrected interval could not be calculated from the usable replicate distribution.";
    case "htmt.formative_not_applicable": return "HTMT is not applicable to formative measurement blocks.";
    case "htmt.single_indicator_not_applicable": return "HTMT requires at least two indicators in each reflective block.";
    default: return "This construct comparison is unavailable for the stored point assessment.";
  }
}

function fitRow(label: string, fit: PlsModelFit["saturated"], advanced = false): string[] | null {
  if (![fit.srmr, fit.d_uls].some(isFiniteNumber)) return null;
  const row = [label, formatOptionalNumber(fit.srmr), formatOptionalNumber(fit.d_uls)];
  if (advanced) {
    row.push(
      fitCriterionDisplay(fit.d_g),
      fitCriterionDisplay(fit.chi_square),
      fitCriterionDisplay(fit.degrees_of_freedom),
      fitCriterionDisplay(fit.nfi),
    );
  }
  return row;
}

function fitCriterionDisplay(value: PlsModelFit["null_model_chi_square"]): string {
  return value?.status === "available" ? formatNumber(value.value) : "Unavailable";
}

function exactFitDecisionLabel(value: boolean | null): string {
  return value === null ? "Unavailable" : value ? "Not rejected" : "Rejected";
}

function parameterLabel(value: string, constructLabel: ConstructDisplayLabel): string {
  try {
    const parsed: unknown = JSON.parse(value);
    if (Array.isArray(parsed) && typeof parsed[0] === "string" && Array.isArray(parsed[1])) {
      const kind = parsed[0].replaceAll("_", " ");
      const parts = parsed[1].filter((part): part is string => typeof part === "string");
      const labelledParts = parameterPartsWithConstructLabels(parsed[0], parts, constructLabel);
      return labelledParts.length ? `${sentenceCase(kind)}: ${labelledParts.join(" → ")}` : sentenceCase(kind);
    }
  } catch {
    // Native result payloads from older engines may already contain a display label.
  }
  return value;
}

function parameterPartsWithConstructLabels(
  kind: string,
  parts: readonly string[],
  constructLabel: ConstructDisplayLabel,
): string[] {
  const labelledParts = [...parts];
  for (const index of constructParameterPartIndexes(kind)) {
    if (labelledParts[index]) labelledParts[index] = constructLabel(labelledParts[index]);
  }
  return labelledParts;
}

function constructDisplayLabelResolver(run: AnalysisRun): ConstructDisplayLabel {
  const labelsById = new Map<string, Set<string>>();
  for (const node of run.modelSnapshot?.nodes ?? []) {
    if (!hasText(node.id) || !hasText(node.data?.label)) continue;
    const labels = labelsById.get(node.id) ?? new Set<string>();
    labels.add(node.data.label.trim());
    labelsById.set(node.id, labels);
  }

  const uniqueLabels = new Map<string, string>();
  for (const [constructId, labels] of labelsById) {
    if (labels.size !== 1) continue;
    const label = [...labels][0];
    uniqueLabels.set(constructId, label);
  }

  const idsByNormalizedDisplay = new Map<string, Set<string>>();
  for (const constructId of constructIdsInRun(run)) {
    const display = uniqueLabels.get(constructId) ?? constructId;
    const normalizedDisplay = normalizeConstructDisplay(display);
    const ids = idsByNormalizedDisplay.get(normalizedDisplay) ?? new Set<string>();
    ids.add(constructId);
    idsByNormalizedDisplay.set(normalizedDisplay, ids);
  }

  return (constructId) => {
    const label = uniqueLabels.get(constructId);
    if (!label) return constructId;
    return (idsByNormalizedDisplay.get(normalizeConstructDisplay(label))?.size ?? 0) > 1
      ? `${label} [${constructId}]`
      : label;
  };
}

function constructIdsInRun(run: AnalysisRun): ReadonlySet<string> {
  const ids = new Set<string>();
  const add = (value: unknown) => {
    if (hasText(value)) ids.add(value);
  };
  const addPair = (source: unknown, target: unknown) => {
    add(source);
    add(target);
  };

  for (const node of run.modelSnapshot?.nodes ?? []) add(node.id);
  for (const edge of run.modelSnapshot?.edges ?? []) addPair(edge.source, edge.target);

  const result = run.result;
  if (result) {
    for (const row of result.paths) addPair(row.source, row.target);
    for (const row of result.effects) addPair(row.source, row.target);
    for (const row of result.outer_estimates) add(row.construct);
    for (const construct of Object.keys(result.r_squared)) add(construct);
    for (const row of result.mediation?.estimates ?? []) addPair(row.source, row.target);
    for (const row of result.plsc?.reliabilities ?? []) add(row.construct);
    for (const row of result.plsc?.construct_correlations ?? []) addPair(row.left, row.right);
    for (const row of result.plsc?.corrected_paths ?? []) addPair(row.source, row.target);
    for (const row of result.plsc?.corrected_outer_loadings ?? []) add(row.construct);
    for (const construct of Object.keys(result.plsc?.corrected_r_squared ?? {})) add(construct);
    for (const row of result.predict?.targets ?? []) add(row.construct);
    for (const row of result.predict?.repeated_kfold?.targets ?? []) add(row.construct);
    for (const row of result.predict?.repeated_kfold?.cvpat ?? []) add(row.target);
    for (const row of result.cca?.correlations ?? []) addPair(row.left, row.right);
    for (const row of result.cta_pls?.estimates ?? []) add(row.construct);
    for (const target of result.ipma?.targets ?? []) add(target);
    for (const row of result.ipma?.constructs ?? []) addPair(row.construct, row.target);
    for (const row of result.ipma?.indicators ?? []) addPair(row.construct, row.target);
    for (const group of result.mga?.groups ?? []) {
      for (const row of group.paths) addPair(row.source, row.target);
      for (const construct of Object.keys(group.r_squared)) add(construct);
      for (const row of group.outer_estimates ?? []) add(row.construct);
    }
    for (const row of result.mga?.comparisons ?? []) addPair(row.source, row.target);
    for (const row of result.mga?.measurement_comparisons ?? []) add(row.construct);
    for (const row of result.mga_permutation?.comparisons ?? []) addPair(row.source, row.target);
    for (const row of result.mga_permutation?.measurement_comparisons ?? []) add(row.construct);
    for (const row of result.micom?.constructs ?? []) add(row.construct);
  }

  const assessment = run.assessment;
  if (assessment) {
    for (const row of assessment.construct_quality ?? []) add(row.construct);
    for (const row of assessment.cross_loadings ?? []) {
      add(row.assigned_construct);
      add(row.construct);
    }
    for (const construct of assessment.fornell_larcker?.constructs ?? []) add(construct);
    for (const construct of assessment.htmt_plus?.constructs ?? []) add(construct);
    for (const construct of assessment.htmt_original?.constructs ?? []) add(construct);
    for (const construct of assessment.htmt?.constructs ?? []) add(construct);
    for (const construct of Object.keys(assessment.r_squared ?? {})) add(construct);
    for (const row of assessment.structural_quality ?? []) add(row.construct);
    for (const row of assessment.structural_vif ?? []) addPair(row.target_construct, row.predictor_construct);
    for (const row of assessment.formative_indicator_vif ?? []) add(row.construct);
    for (const row of assessment.f_squared ?? []) addPair(row.source_construct, row.target_construct);
    for (const row of assessment.blindfolding?.constructs ?? []) add(row.construct);
  }

  const addParameterConstructs = (parameter: string) => {
    const identity = effectParameterIdentity(parameter);
    if (!identity) return;
    for (const index of constructParameterPartIndexes(identity.kind)) add(identity.parts[index]);
  };
  for (const row of run.bootstrap?.percentile.parameters ?? []) addParameterConstructs(row.parameter);
  for (const row of run.bootstrap?.bca?.parameters ?? []) addParameterConstructs(row.parameter);
  for (const row of run.bootstrap?.studentized?.parameters ?? []) addParameterConstructs(row.parameter);
  for (const row of nativeStructuralPathRandomizationProjection(run)?.parameters ?? []) addParameterConstructs(row.parameter);
  return ids;
}

function constructParameterPartIndexes(kind: string): readonly number[] {
  if ([
    "path",
    "direct_effect",
    "indirect_effect",
    "total_effect",
    "plsc_construct_correlation",
    "plsc_path",
    "plsc_direct_effect",
    "plsc_indirect_effect",
    "plsc_total_effect",
  ].includes(kind)) return [0, 1];
  if ([
    "r_squared",
    "outer_loading",
    "outer_weight",
    "plsc_rho_a",
    "plsc_outer_loading",
    "plsc_outer_weight",
    "plsc_r_squared",
  ].includes(kind)) return [0];
  return [];
}

function normalizeConstructDisplay(value: string): string {
  return value.trim().normalize("NFKC").toLowerCase();
}

function constructPathLabel(constructIds: readonly string[], constructLabel: ConstructDisplayLabel): string {
  return constructIds.map(constructLabel).join(" → ");
}

function sentenceCase(value: string) {
  return value.length ? `${value[0].toUpperCase()}${value.slice(1)}` : value;
}

function firstFinite(...values: Array<number | null | undefined>): number | null {
  return values.find(isFiniteNumber) ?? null;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isNonNegativeInteger(value: unknown): value is number {
  return isFiniteNumber(value) && Number.isInteger(value) && value >= 0;
}

function isPositiveInteger(value: unknown): value is number {
  return isNonNegativeInteger(value) && value > 0;
}

function isProbability(value: unknown): value is number {
  return isFiniteNumber(value) && value >= 0 && value <= 1;
}

function numbersClose(left: unknown, right: unknown): boolean {
  return isFiniteNumber(left)
    && isFiniteNumber(right)
    && Math.abs(left - right) <= 1e-10 * Math.max(1, Math.abs(left), Math.abs(right));
}

/**
 * Allows only the few representable-value steps introduced when an already
 * validated Rust f64 is serialized through the Tauri JSON boundary. Unlike
 * general scientific comparisons, this is relative to the operands themselves
 * and therefore stays strict for derived values near machine epsilon.
 */
function jsonRoundTripNumbersClose(left: unknown, right: unknown): boolean {
  if (!isFiniteNumber(left) || !isFiniteNumber(right)) return false;
  if (left === right) return true;
  const scale = Math.max(Math.abs(left), Math.abs(right));
  return scale > 0
    && Math.abs(left - right) <= 4 * Number.EPSILON * scale;
}

const NORMAL_95_PERCENT_CRITICAL_VALUE = 1.959963984540054;
const GAMMA_EPSILON = 1e-14;
const GAMMA_MAX_ITERATIONS = 200;
const GAMMA_MIN_DENOMINATOR = 1e-300;

function scientificNumbersClose(left: unknown, right: unknown): boolean {
  return isFiniteNumber(left)
    && isFiniteNumber(right)
    && Math.abs(left - right) <= 1e-9 * Math.max(1, Math.abs(left), Math.abs(right));
}

function logGamma(value: number): number {
  const coefficients = [
    676.5203681218851,
    -1259.1392167224028,
    771.3234287776531,
    -176.6150291621406,
    12.507343278686905,
    -0.13857109526572012,
    9.984369578019572e-6,
    1.5056327351493116e-7,
  ];
  if (value < 0.5) {
    return Math.log(Math.PI) - Math.log(Math.sin(Math.PI * value)) - logGamma(1 - value);
  }
  const shifted = value - 1;
  let series = 0.9999999999998099;
  coefficients.forEach((coefficient, index) => {
    series += coefficient / (shifted + index + 1);
  });
  const base = shifted + coefficients.length - 0.5;
  return 0.5 * Math.log(2 * Math.PI) + (shifted + 0.5) * Math.log(base) - base + Math.log(series);
}

function regularizedGammaQ(shape: number, value: number): number {
  if (!Number.isFinite(shape) || shape <= 0 || !Number.isFinite(value) || value < 0) return Number.NaN;
  if (value === 0) return 1;
  const logScale = -value + shape * Math.log(value) - logGamma(shape);
  if (value < shape + 1) {
    let term = 1 / shape;
    let sum = term;
    let denominator = shape;
    for (let iteration = 1; iteration <= GAMMA_MAX_ITERATIONS; iteration += 1) {
      denominator += 1;
      term *= value / denominator;
      sum += term;
      if (Math.abs(term) <= Math.abs(sum) * GAMMA_EPSILON) break;
    }
    return Math.min(1, Math.max(0, 1 - sum * Math.exp(logScale)));
  }

  let offset = value + 1 - shape;
  let continuedNumerator = 1 / GAMMA_MIN_DENOMINATOR;
  let continuedDenominator = 1 / Math.max(Math.abs(offset), GAMMA_MIN_DENOMINATOR) * Math.sign(offset || 1);
  let fraction = continuedDenominator;
  for (let iteration = 1; iteration <= GAMMA_MAX_ITERATIONS; iteration += 1) {
    const coefficient = -iteration * (iteration - shape);
    offset += 2;
    continuedDenominator = coefficient * continuedDenominator + offset;
    if (Math.abs(continuedDenominator) < GAMMA_MIN_DENOMINATOR) continuedDenominator = GAMMA_MIN_DENOMINATOR;
    continuedNumerator = offset + coefficient / continuedNumerator;
    if (Math.abs(continuedNumerator) < GAMMA_MIN_DENOMINATOR) continuedNumerator = GAMMA_MIN_DENOMINATOR;
    continuedDenominator = 1 / continuedDenominator;
    const change = continuedDenominator * continuedNumerator;
    fraction *= change;
    if (Math.abs(change - 1) <= GAMMA_EPSILON) break;
  }
  return Math.min(1, Math.max(0, Math.exp(logScale) * fraction));
}

function chiSquareSurvival(value: number, degreesOfFreedom: number): number {
  return regularizedGammaQ(degreesOfFreedom / 2, value / 2);
}

function hasText(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function generatedHigherOrderIndicatorName(higherOrderId: string, componentId: string): string {
  return `__qpls_hoc_${higherOrderId}_${componentId}`;
}

function isGeneratedTechnicalIndicator(value: string): boolean {
  return value.startsWith("__qpls_interaction_") || value.startsWith("__qpls_hoc_");
}

function formatNumber(value: number, digits = 6): string {
  const formatted = value.toFixed(digits);
  return Number(formatted) === 0 ? (0).toFixed(digits) : formatted;
}

function formatOptionalNumber(value: number | null | undefined, digits = 4): string {
  return isFiniteNumber(value) ? formatNumber(value, digits) : "";
}

function formatPValue(value: number): string {
  return value < 0.0001 ? "<0.0001" : formatNumber(value, 4);
}

function formatOptionalPValue(value: number | null | undefined): string {
  return isFiniteNumber(value) ? formatPValue(value) : "";
}
