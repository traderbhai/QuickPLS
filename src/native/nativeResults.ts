import type { ResultTable } from "../domain/resultTables";
import type {
  AnalysisRun,
  CbsemAnalysis,
  CvpatBenchmarkAssessment,
  HtmtAssessment,
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
import { isNativeRegressionBootstrapValidationWitness } from "./nativeRegressionBootstrapWitness";
import {
  NATIVE_GSCA_ALGORITHM_VERSION,
  NATIVE_GSCA_ASSESSMENT_WARNING,
  NATIVE_GSCA_ENGINE_SCOPE_WARNING,
  NATIVE_GSCA_METHOD_VERSION,
  NATIVE_GSCA_SCOPE_NOTE,
} from "./nativeGsca";
import {
  NATIVE_LEGACY_PROCESS_RESULT_IDS,
  NATIVE_PROCESS_RESULT_IDS,
  nativeLegacyProcessResultProjection,
  nativeLegacyProcessResultTables,
  nativeProcessResultProjection,
  nativeProcessResultTables,
} from "./nativeProcessResults";
import {
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
  nativeStructuralPathRandomizationProjection,
  nativeStructuralPathRandomizationTable,
} from "./nativeStructuralPathRandomization";

export {
  nativeLegacyProcessResultProjection,
  nativeProcessResultProjection,
} from "./nativeProcessResults";

export type NativeResultGroupId = "graphical" | "groups" | "assessment" | "covariance_sem" | "gsca_component_model" | "importance_performance" | "necessary_conditions" | "components" | "process" | "regression" | "higher_order" | "final_results" | "mediation" | "moderation" | "quality_criteria" | "prediction" | "inference";

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

export interface NativeNcaResultProjection {
  methodVersion: "nca_v2";
  x: string;
  y: string;
  observations: number;
  ceiling: "ce_fdh" | "cr_fdh" | "both";
  permutationSamples: number;
  usablePermutations: number;
  scope: NativeNcaPlot["scope"];
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
  "htmt",
  "structural_quality",
  "structural_vif",
  "formative_indicator_vif",
  "f_squared",
  "model_fit",
  "blindfolding",
] as const;

const INFERENCE_IDS = [
  "control_bootstrap",
  "control_bca",
  "control_studentized",
  "control_randomization",
  "bootstrap_percentile",
  "bootstrap_bca",
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
] as const;

const CCA_ASSESSMENT_IDS = [
  "cca_residual_summary",
  "cca_composite_residuals",
] as const;

const IPMA_RESULT_IDS = [
  "ipma_constructs",
  "ipma_indicators",
  "ipma_scope",
] as const;

const NCA_RESULT_IDS = [
  "nca_ceiling_effects",
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
  "cbsem_standardized_parameters",
  "cbsem_unstandardized_parameters",
  "cbsem_residual_correlations",
  "cbsem_residual_covariances",
  "cbsem_implied_covariances",
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

export const NATIVE_IPMA_SCOPE_NOTE =
  "Performance uses 0–100 observed-range min–max scaling of listwise-standardized composite scores. No theoretical-range correction is applied.";

const CURRENT_MGA_METHOD_VERSION = "pls_mga_two_group_v2";
const CURRENT_MGA_PERMUTATION_METHOD_VERSION = "pls_mga_permutation_v2";
const CURRENT_MICOM_METHOD_VERSION = "micom_v2";

const MAX_SPECIFIC_INDIRECT_EFFECTS = 5_000;
const SPECIFIC_INDIRECT_EFFECTS_TRUNCATED_WARNING =
  "Showing the first 5,000 specific indirect paths. Additional paths were omitted to keep Results responsive.";

export function completedResultRuns(runs: readonly AnalysisRun[]): AnalysisRun[] {
  return runs.filter(isCompletedResultRun);
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
    ceFdhPeers: peers.map((peer) => ({ x: peer.x, y: peer.y })),
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
    "Point estimates only in the bounded native workflow",
  ]);
  return { constructIds, componentRows, structuralRows, scopeRows };
}

/**
 * Builds the compact native results contract from a completed run. Tables are
 * included only when the engine payload contains at least one real output row.
 */
export function nativeResultTables(run: AnalysisRun | null | undefined): ResultTable[] {
  if (!isCompletedResultRun(run)) return [];

  const tables: ResultTable[] = [];
  const result = run.result;
  const structuralPathRandomization = nativeStructuralPathRandomizationProjection(run);
  const inferenceRun = run.permutation && !structuralPathRandomization
    ? { ...run, permutation: undefined }
    : run;
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
  const constructLabel = constructDisplayLabelResolver(run);
  if (result.mga) {
    addMgaResultTables(tables, run, constructLabel);
    return tables;
  }
  const moderationProductConstructIds = new Set((result.moderation?.estimates ?? [])
    .filter((row) => hasText(row.product_construct))
    .map((row) => row.product_construct));
  const higherOrder = nativeHigherOrderProjection(run, constructLabel);
  const higherOrderConstructIds = new Set(run.modelSnapshot?.nodes
    .filter((node) => node.data.semantic === "higher_order")
    .map((node) => node.id) ?? []);
  const technicalConstructIds = new Set([...moderationProductConstructIds, ...higherOrderConstructIds]);
  const controlPairs = new Set((result.control_estimates ?? [])
    .filter((row) => hasText(row.source) && hasText(row.target))
    .map((row) => effectPairKey(row.source, row.target)));
  const substantivePaths = result.paths.filter((row) =>
    !controlPairs.has(effectPairKey(row.source, row.target))
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
      title: "Higher-order calculation scope",
      warning: null,
      columns: ["Higher-order construct", "Components", "Method", "Generated measurement", "Inference"],
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
        title: "Calculation scope",
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

    if (assessment.model_fit) {
      addTable(tables, {
        id: "model_fit",
        title: "Model fit",
        warning: null,
        columns: ["Model", "SRMR", "d_ULS"],
        rows: [
          fitRow("Saturated model", assessment.model_fit.saturated),
          fitRow("Estimated model", assessment.model_fit.estimated),
        ].filter((row): row is string[] => Boolean(row)),
      });
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

  if (run.bootstrap) {
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
      title: "Bootstrapping",
      warning: run.bootstrap.failed_replicates.length
        ? `${run.bootstrap.failed_replicates.length} bootstrap replicate(s) failed.`
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
  if (!isCompletedResultRun(run)) return [];

  const byId = new Map(tables.map((table) => [table.id, table]));
  const groups: NativeResultNavigationGroup[] = [];
  const hasMga = Boolean(run.result.mga);
  const standalone = isStandaloneNativeAnalysis(run.provenance?.method);
  if (!hasMga && !standalone) {
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
  addTableGroup(groups, "higher_order", "Higher-order construct", HIGHER_ORDER_IDS, byId);

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
  if (!isCompletedResultRun(run)) {
    return { runId: null, defaultItemId: null, groups: [], tables: [] };
  }
  const tables = nativeResultTables(run);
  const groupDefault = ["micom_summary", "mga_permutation", "mga_path_differences", "mga_group_paths", "mga_group_summary"]
    .find((id) => tables.some((table) => table.id === id));
  const predictionDefault = PREDICTION_IDS.find((id) => tables.some((table) => table.id === id));
  const ccaDefault = CCA_ASSESSMENT_IDS.find((id) => tables.some((table) => table.id === id));
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
  const fallbackDefault = run.result.mga || standalone ? tables[0]?.id ?? null : "model_estimates";
  return {
    runId: run.id,
    defaultItemId: processDefault ?? regressionBootstrapDefault ?? groupDefault ?? ipmaDefault ?? ncaDefault ?? pcaDefault ?? logisticDefault ?? legacyLogisticDefault ?? olsDefault ?? cbsemDefault ?? gscaDefault ?? ccaDefault ?? predictionDefault ?? higherOrderDefault ?? fallbackDefault,
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
      warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
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
      warning: NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
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
      "Predictor scope",
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
      "Target scope",
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
  const legacyWarning = "Legacy v1 construct-score scope; this output is not current indicator-level PLSpredict or CVPAT.";
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

export function nativeCbsemResultProjection(run: AnalysisRun | null | undefined): NativeCbsemResultProjection | null {
  if (!isCompletedResultRun(run) || run.provenance?.method !== "cbsem") return null;
  const analysis = run.result.cbsem;
  if (!analysis || (analysis.model_type !== "cfa" && analysis.model_type !== "sem")) return null;
  const methodVersion = analysis.model_type === "cfa" ? "cfa_ml_v1" : "cbsem_ml_v1";
  const assessmentVersion = run.assessment?.method_version;
  const expectedProvenance = assessmentVersion
    ? `pls_pm_v1+${methodVersion}+${CBSEM_FIT_METHOD_VERSION}+${CBSEM_MODIFICATION_METHOD_VERSION}+pls_mediation_v1+${assessmentVersion}`
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
    || analysis.bootstrap
    || analysis.multigroup
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
  const constructLabel = constructDisplayLabelResolver(run);
  const parameterLabel = (kind: string, lhs: string, rhs: string) => {
    if (kind === "loading") return `${rhs} ← ${constructLabel(lhs)}`;
    if (kind === "structural_path") return `${constructLabel(lhs)} ← ${constructLabel(rhs)}`;
    if (kind === "latent_covariance") return `${constructLabel(lhs)} ↔ ${constructLabel(rhs)}`;
    if (kind === "latent_variance") return `Variance: ${constructLabel(lhs)}`;
    if (kind === "residual_variance") return `Residual variance: ${lhs}`;
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
      ["RMSEA lower bound", formatOptionalNumber(analysis.fit.rmsea_ci_lower, 6)],
      ["RMSEA upper bound", formatOptionalNumber(analysis.fit.rmsea_ci_upper, 6)],
      ["SRMR", formatNumber(analysis.fit.srmr)],
      ["AIC", formatNumber(analysis.fit.aic)],
      ["BIC", formatNumber(analysis.fit.bic)],
      ["Baseline χ²", formatNumber(analysis.fit.baseline_chi_square)],
      ["Baseline degrees of freedom", String(analysis.fit.baseline_degrees_of_freedom)],
    ],
  });
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
  addTable(tables, {
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
    title: "Calculation scope",
    warning: analysis.warnings.join(" "),
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
      ["Modification-diagnostic version", CBSEM_MODIFICATION_METHOD_VERSION],
      ["Unsupported in this workflow", "Bootstrap, multigroup/invariance, robust/ordinal/FIML estimators, interactions, higher-order constructs, and mean structures"],
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
    title: "Calculation scope",
    warning: analysis.warnings.join(" "),
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
      ["Inference", "Point estimates only; no bootstrap or permutation inference"],
      ["Bounded native scope", NATIVE_GSCA_SCOPE_NOTE],
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
    title: "Calculation scope",
    warning: projection.warnings.join(" "),
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
    title: "Calculation scope",
    warning: projection.warnings.join(" "),
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
    title: "Calculation scope",
    warning: projection.warnings.join(" "),
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
  const classificationWarning = "In-sample descriptive classification; not out-of-sample predictive performance.";

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
    warning: classificationWarning,
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
    title: "Calculation scope",
    warning: projection.warnings.join(" "),
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
      ["Classification interpretation", classificationWarning],
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
  const legacyWarning = `Historical v1 output is retained as originally computed and is not reinterpreted as current v2 evidence. ${projection.warnings.join(" ")}`;

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
      ["Historical handling", "Readable and exportable under its original version; not promoted to v2 evidence"],
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
  const permutation = isCurrentMga
    ? currentMgaPermutation(run.result.mga_permutation, mga.group_column)
    : null;
  const groupAOuter = validMgaOuterEstimates(groupA);
  const groupBOuter = validMgaOuterEstimates(groupB);
  const measurementComparisons = isCurrentMga
    ? validMgaMeasurementComparisons(mga, groupA, groupB, groupAOuter, groupBOuter)
    : [];
  const micom = isCurrentMga
    ? currentMicomProjection(run.result.micom, mga, groupA, groupB, groupAOuter, groupBOuter)
    : null;
  const engineWarnings = [
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

  if (micom) addMicomResultTables(tables, micom, groupA.group, groupB.group, constructLabel);

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

  if (isCurrentMga) {
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

function currentMgaPermutation(
  permutation: NonNullable<AnalysisRun["result"]>["mga_permutation"],
  groupColumn: string,
): MgaPermutationAnalysis | null {
  if (!permutation
    || permutation.method_version !== CURRENT_MGA_PERMUTATION_METHOD_VERSION
    || permutation.group_column !== groupColumn
    || !isPositiveInteger(permutation.permutation_samples)
    || permutation.permutation_samples < 5_000
    || permutation.permutation_samples > 10_000
    || permutation.usable_permutations !== permutation.permutation_samples
    || !isNonNegativeInteger(permutation.attempted_permutations)
    || !isNonNegativeInteger(permutation.failed_permutations)
    || permutation.attempted_permutations < permutation.usable_permutations
    || permutation.attempted_permutations - permutation.usable_permutations !== permutation.failed_permutations) return null;
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
): CurrentMicomProjection | null {
  if (!micom
    || micom.method_version !== CURRENT_MICOM_METHOD_VERSION
    || micom.group_column !== mga.group_column
    || !isPositiveInteger(micom.permutation_samples)
    || micom.permutation_samples < 5_000
    || micom.permutation_samples > 10_000
    || micom.usable_permutations !== micom.permutation_samples
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
    || !numbersClose(row.variance_difference, Math.log(row.variance_a / row.variance_b))
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
  return { id, title, warning: null, columns: ["Construct", "Compared with", "Value"], rows };
}

function fitRow(label: string, fit: { srmr: number; d_uls: number }): string[] | null {
  if (![fit.srmr, fit.d_uls].some(isFiniteNumber)) return null;
  return [label, formatOptionalNumber(fit.srmr), formatOptionalNumber(fit.d_uls)];
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
  if (["path", "direct_effect", "indirect_effect", "total_effect"].includes(kind)) return [0, 1];
  if (["r_squared", "outer_loading", "outer_weight"].includes(kind)) return [0];
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
