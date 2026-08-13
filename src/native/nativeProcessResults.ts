import type { ResultTable } from "../domain/resultTables";
import type {
  AnalysisRun,
  ProcessAnalysis,
  ProcessBootstrapAnalysis,
  ProcessGraphAnalysis,
  ProcessJohnsonNeymanAnalysis,
  ProcessModeratorValue,
  RegressionAnalysis,
} from "../types";
import { NATIVE_STANDALONE_ASSESSMENT_WARNING } from "./nativeNca";
import {
  NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION,
  NATIVE_PROCESS_INFERENCE_WARNING,
  NATIVE_PROCESS_METHOD_VERSION,
  NATIVE_PROCESS_PROMOTION_PENDING_WARNING,
  NATIVE_PROCESS_RESULT_WARNING,
  NATIVE_PROCESS_SCOPE_NOTE,
} from "./nativeProcess";

export const NATIVE_PROCESS_RESULT_IDS = [
  "process_model_summary",
  "process_paths",
  "process_equation_coefficients",
  "process_equation_fit",
  "process_reference_effects",
  "process_conditional_indirect_effects",
  "process_moderated_mediation_indices",
  "process_simple_slopes",
  "process_conditional_plot_points",
  "process_johnson_neyman",
  "process_johnson_neyman_curve_points",
  "process_bootstrap_summary",
  "process_bootstrap_failures",
  "process_bootstrap_inference",
  "process_bootstrap_bca",
  "process_scope",
] as const;

export const NATIVE_LEGACY_PROCESS_RESULT_IDS = [
  "legacy_process_effects",
  "legacy_process_simple_slopes",
  "legacy_process_scope",
] as const;

export interface NativeProcessResultProjection {
  methodVersion: typeof NATIVE_PROCESS_METHOD_VERSION;
  outcome: string;
  predictors: string[];
  controls: string[];
  observations: number;
  omittedObservations: number;
  graph: ProcessGraphAnalysis;
  bootstrap: ProcessBootstrapAnalysis | null;
  warnings: string[];
}

export interface NativeLegacyProcessResultProjection {
  methodVersion: "regression_process_v1";
  model: "mediation" | "moderation" | "moderated_mediation";
  analysis: ProcessAnalysis;
  warnings: string[];
}

const PROCESS_BOOTSTRAP_SCOPE_WARNING =
  "PROCESS bootstrap v1 uses deterministic indexed complete-case resampling with replacement; percentile intervals are primary and BCa intervals require every delete-one fit.";
const PROCESS_BOOTSTRAP_TEST_WARNING =
  "PROCESS bootstrap ratio tests use the original effect divided by its bootstrap standard error with a fixed two-sided standard-normal reference.";
export const NATIVE_PROCESS_REFERENCE_CONDITION =
  "Continuous moderators are evaluated at their original complete-sample raw means (coded 0); binary moderators are evaluated at 0.";
const PROCESS_JN_INVALID_COVARIANCE_MESSAGE =
  "Johnson-Neyman conditional-effect variance must be finite and strictly positive across the tested moderator range.";

function hasText(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function finite(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function positiveInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value > 0;
}

function nonNegativeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function probability(value: unknown): value is number {
  return finite(value) && value >= 0 && value <= 1;
}

function close(left: number, right: number, multiplier = 256): boolean {
  return finite(left)
    && finite(right)
    && Math.abs(left - right) <= multiplier * Number.EPSILON * Math.max(1, Math.abs(left), Math.abs(right));
}

function exactStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function strictlyAscending(values: readonly number[]): boolean {
  return values.every((value, index) => index === 0 || values[index - 1] < value);
}

function moderatorValuesValid(values: readonly ProcessModeratorValue[]): boolean {
  return Array.isArray(values)
    && new Set(values.map((value) => value.variable)).size === values.length
    && values.every((value) => hasText(value.variable) && finite(value.raw_value) && finite(value.coded_value));
}

function processShellIsExclusive(run: AnalysisRun): boolean {
  const result = run.result;
  return Boolean(result)
    && !run.modelId
    && !run.modelSnapshot
    && !run.bootstrap
    && !run.permutation
    && !result!.plsc
    && !result!.endogeneity
    && !result!.nonlinear_effects
    && !result!.moderated_mediation
    && !result!.cta_pls
    && !result!.wpls
    && !result!.cca
    && !result!.predict
    && !result!.segmentation
    && !result!.mga
    && !result!.micom
    && !result!.mga_permutation
    && !result!.fimix
    && !result!.ipma
    && !result!.cbsem
    && !result!.pca
    && !result!.nca
    && !result!.gsca
    && result!.converged
    && result!.iterations === 0
    && result!.outer_estimates.length === 0
    && result!.paths.length === 0
    && result!.effects.length === 0
    && !result!.mediation
    && !result!.moderation
    && (result!.control_estimates?.length ?? 0) === 0
    && Object.keys(result!.r_squared).length === 0;
}

function processRegressionShellValid(regression: RegressionAnalysis): boolean {
  return hasText(regression.outcome)
    && regression.predictors.length >= 1
    && regression.predictors.length <= 8
    && regression.controls.length <= 1
    && new Set([regression.outcome, ...regression.predictors, ...regression.controls]).size
      === 1 + regression.predictors.length + regression.controls.length
    && positiveInteger(regression.observations)
    && regression.coefficients.length === 0
    && regression.fit === null
    && regression.predictions.length === 0
    && !regression.logistic
    && !regression.bootstrap;
}

function variableProfilesValid(
  graph: ProcessGraphAnalysis,
  regression: RegressionAnalysis,
): boolean {
  const profiles = graph.variable_profiles;
  if (!Array.isArray(profiles)
    || profiles.length !== 1 + regression.predictors.length + regression.controls.length
    || new Set(profiles.map((profile) => profile.variable)).size !== profiles.length) return false;
  const expected = new Set([regression.outcome, ...regression.predictors, ...regression.controls]);
  const moderatorNames = new Set(graph.moderations.flatMap((row) => [row.moderator, row.conditioning_moderator].filter(hasText)));
  const focal = regression.predictors[0];
  return profiles.every((profile) => {
    if (!expected.has(profile.variable)
      || !["focal_predictor", "mediator", "moderator", "outcome", "control"].includes(profile.role)
      || (profile.scale !== "continuous" && profile.scale !== "binary_0_1")
      || !finite(profile.raw_mean)
      || !finite(profile.raw_sample_sd)
      || profile.raw_sample_sd < 0
      || !finite(profile.raw_min)
      || !finite(profile.raw_max)
      || profile.raw_min > profile.raw_max
      || !Array.isArray(profile.levels)
      || profile.levels.some((level) => !finite(level))) return false;
    const expectedRole = profile.variable === regression.outcome
      ? "outcome"
      : profile.variable === focal
        ? "focal_predictor"
        : regression.controls.includes(profile.variable)
          ? "control"
          : moderatorNames.has(profile.variable)
            ? "moderator"
            : "mediator";
    if (profile.role !== expectedRole || (expectedRole !== "moderator" && profile.scale !== "continuous")) return false;
    return profile.scale === "binary_0_1"
      ? exactStrings(profile.levels.map(String), ["0", "1"])
        && profile.raw_min >= 0 && profile.raw_max <= 1
      : profile.levels.length === 0;
  });
}

function graphPathsValid(graph: ProcessGraphAnalysis): boolean {
  const pathKeys = graph.paths.map((path) => `${path.from}->${path.to}`);
  const moderationKeys = graph.moderations.map((moderation) => (
    moderation.conditioning_moderator
      ? `moderation:${moderation.from}->${moderation.to}@${moderation.moderator}|${moderation.conditioning_moderator}`
      : `moderation:${moderation.from}->${moderation.to}@${moderation.moderator}`
  ));
  return graph.paths.length >= 1
    && new Set(pathKeys).size === graph.paths.length
    && graph.paths.every((path, index) => hasText(path.from)
      && hasText(path.to)
      && path.from !== path.to
      && path.path_id === pathKeys[index])
    && new Set(moderationKeys).size === graph.moderations.length
    && graph.moderations.every((moderation, index) => hasText(moderation.from)
      && hasText(moderation.to)
      && hasText(moderation.moderator)
      && pathKeys.includes(`${moderation.from}->${moderation.to}`)
      && (!moderation.conditioning_moderator
        || (hasText(moderation.conditioning_moderator)
          && moderation.conditioning_moderator !== moderation.moderator))
      && moderation.moderation_id === moderationKeys[index]);
}

function equationsValid(graph: ProcessGraphAnalysis): boolean {
  if (!Array.isArray(graph.equations)
    || graph.equations.length < 1
    || new Set(graph.equations.map((equation) => equation.outcome)).size !== graph.equations.length) return false;
  return graph.equations.every((equation) => {
    const width = equation.coefficients.length;
    if (!hasText(equation.outcome)
      || equation.equation_id !== `equation:${equation.outcome}`
      || width < 2
      || width > 51
      || !exactStrings(equation.term_ids, equation.coefficients.map((row) => row.term_id))
      || equation.term_ids[0] !== "intercept"
      || new Set(equation.term_ids).size !== equation.term_ids.length
      || !positiveInteger(equation.residual_degrees_of_freedom)
      || equation.coefficient_covariance.length !== width
      || equation.coefficient_covariance.some((row) => row.length !== width || row.some((value) => !finite(value)))) return false;
    if (equation.coefficient_covariance.some((row, rowIndex) => row.some((value, columnIndex) => (
      !close(value, equation.coefficient_covariance[columnIndex][rowIndex], 1_024)
        || (rowIndex === columnIndex && (value <= 0
          || !close(Math.sqrt(value), equation.coefficients[rowIndex].standard_error, 1_024)))
    )))) return false;
    if (!equation.coefficients.every((coefficient) => hasText(coefficient.term_id)
      && ["intercept", "path", "moderator_main", "interaction", "control"].includes(coefficient.kind)
      && Array.isArray(coefficient.variables)
      && coefficient.variables.every(hasText)
      && finite(coefficient.estimate)
      && finite(coefficient.standard_error)
      && coefficient.standard_error > 0
      && finite(coefficient.statistic)
      && close(coefficient.statistic, coefficient.estimate / coefficient.standard_error, 1_024)
      && probability(coefficient.p_value_two_sided)
      && finite(coefficient.confidence_interval_lower)
      && finite(coefficient.confidence_interval_upper)
      && coefficient.confidence_interval_lower <= coefficient.estimate
      && coefficient.confidence_interval_upper >= coefficient.estimate)) return false;
    const fit = equation.fit;
    if (!positiveInteger(fit.observations)
      || !positiveInteger(fit.parameter_count)
      || fit.observations !== graph.complete_cases
      || fit.parameter_count !== equation.coefficients.length
      || equation.residual_degrees_of_freedom !== fit.observations - fit.parameter_count
      || !finite(fit.residual_sum_squares)
      || fit.residual_sum_squares < 0
      || !finite(fit.total_sum_squares)
      || fit.total_sum_squares < 0) return false;
    const rSquared = fit.total_sum_squares > Number.EPSILON
      ? 1 - fit.residual_sum_squares / fit.total_sum_squares
      : 0;
    const adjusted = 1 - (1 - rSquared) * (fit.observations - 1) / equation.residual_degrees_of_freedom;
    const fStatistic = rSquared < 1
      ? (rSquared / (fit.parameter_count - 1)) / ((1 - rSquared) / equation.residual_degrees_of_freedom)
      : 0;
    const sigma2 = Math.max(fit.residual_sum_squares / fit.observations, Number.MIN_VALUE);
    return finite(fit.r_squared)
      && close(fit.r_squared, rSquared, 1_024)
      && finite(fit.adjusted_r_squared)
      && close(fit.adjusted_r_squared, adjusted, 1_024)
      && finite(fit.f_statistic)
      && close(fit.f_statistic, fStatistic, 1_024)
      && finite(fit.aic)
      && close(fit.aic, fit.observations * Math.log(sigma2) + 2 * fit.parameter_count, 1_024)
      && finite(fit.bic)
      && close(fit.bic, fit.observations * Math.log(sigma2)
        + Math.log(fit.observations) * fit.parameter_count, 1_024)
      && finite(fit.rmse)
      && close(fit.rmse, Math.sqrt(fit.residual_sum_squares / fit.observations), 1_024);
  });
}

function narrowJsonRoundTripClose(left: number, right: number): boolean {
  if (Object.is(left, right) || left === right) return true;
  const scale = Math.max(Math.abs(left), Math.abs(right));
  const tolerance = 4 * Math.max(Number.MIN_VALUE, Number.EPSILON * scale);
  return Math.abs(left - right) <= tolerance;
}

function uniqueSemanticLevelIndex(levels: readonly number[], actual: number): number | null {
  const exact = levels
    .map((level, index) => ({ index, matches: Object.is(level, actual) || level === actual }))
    .filter(({ matches }) => matches);
  if (exact.length === 1) return exact[0].index;
  if (exact.length > 1) return null;
  const distances = levels.map((level, index) => ({ index, distance: Math.abs(level - actual) }));
  const minimum = Math.min(...distances.map(({ distance }) => distance));
  const nearest = distances.filter(({ distance }) => distance === minimum);
  if (nearest.length !== 1 || !narrowJsonRoundTripClose(levels[nearest[0].index], actual)) return null;
  return nearest[0].index;
}

export function nativeProcessSemanticProbeSuffix(
  graph: ProcessGraphAnalysis,
  values: readonly ProcessModeratorValue[],
): string | null {
  const parts: string[] = [];
  for (const value of values) {
    const profile = graph.variable_profiles.find((candidate) => candidate.variable === value.variable);
    if (!profile) return null;
    const rawLevels = profile.scale === "binary_0_1"
      ? [0, 1]
      : [profile.raw_mean - profile.raw_sample_sd, profile.raw_mean, profile.raw_mean + profile.raw_sample_sd];
    if (!rawLevels.every(finite)
      || new Set(rawLevels.map((raw) => Object.is(raw, -0) ? 0 : raw)).size !== rawLevels.length
      || (profile.scale !== "binary_0_1" && !(rawLevels[0] < rawLevels[1] && rawLevels[1] < rawLevels[2]))) return null;
    const index = uniqueSemanticLevelIndex(rawLevels, value.raw_value);
    if (index === null) return null;
    const expectedCoded = profile.scale === "binary_0_1"
      ? rawLevels[index]
      : rawLevels[index] - profile.raw_mean;
    if (!narrowJsonRoundTripClose(expectedCoded, value.coded_value)) return null;
    const token = profile.scale === "binary_0_1"
      ? ["binary_0", "binary_1"][index]
      : ["minus_1sd", "mean", "plus_1sd"][index];
    parts.push(`${value.variable}=${token}`);
  }
  return parts.join(",");
}

function effectsValid(graph: ProcessGraphAnalysis): boolean {
  const allEffectIds = [
    ...graph.reference_effects.map((row) => row.effect_id),
    ...graph.conditional_indirect_effects.map((row) => row.effect_id),
    ...graph.moderated_mediation_indices.map((row) => row.effect_id),
    ...graph.simple_slopes.map((row) => row.effect_id),
  ];
  if (allEffectIds.length < 3
    || new Set(allEffectIds).size !== allEffectIds.length
    || !graph.reference_effects.every((effect) => hasText(effect.effect_id)
      && ["direct", "indirect", "total_indirect", "total"].includes(effect.kind)
      && effect.path.length >= 2
      && effect.path.every(hasText)
      && finite(effect.estimate))
    || !graph.conditional_indirect_effects.every((effect) => {
      const suffix = nativeProcessSemanticProbeSuffix(graph, effect.moderator_values);
      return suffix !== null
        && effect.effect_id === `indirect:${effect.path_id}@${suffix}`
        && hasText(effect.path_id)
        && moderatorValuesValid(effect.moderator_values)
        && finite(effect.estimate);
    })
    || !graph.moderated_mediation_indices.every((effect) => hasText(effect.effect_id)
      && hasText(effect.path_id)
      && hasText(effect.moderated_edge)
      && hasText(effect.moderator)
      && finite(effect.estimate))) return false;
  return graph.simple_slopes.every((slope) => {
    const suffix = nativeProcessSemanticProbeSuffix(graph, slope.moderator_values);
    return suffix !== null
    && slope.effect_id === `slope:${slope.moderation_id}@${suffix}`
    && hasText(slope.moderation_id)
    && moderatorValuesValid(slope.moderator_values)
    && finite(slope.estimate)
    && finite(slope.standard_error)
    && slope.standard_error >= 0
    && finite(slope.statistic)
    && (slope.standard_error === 0 || close(slope.statistic, slope.estimate / slope.standard_error, 1_024))
    && probability(slope.p_value_two_sided)
    && finite(slope.confidence_interval_lower)
    && finite(slope.confidence_interval_upper)
    && slope.confidence_interval_lower <= slope.confidence_interval_upper;
  });
}

function plotsValid(graph: ProcessGraphAnalysis): boolean {
  const moderationIds = new Set(graph.moderations.map((moderation) => moderation.moderation_id));
  return graph.plots.every((plot) => hasText(plot.plot_id)
    && moderationIds.has(plot.moderation_id)
    && plot.series.length >= 1
    && new Set(plot.series.map((series) => series.series_id)).size === plot.series.length
    && plot.series.every((series, seriesIndex) => {
      const suffix = nativeProcessSemanticProbeSuffix(graph, series.moderator_values);
      return suffix !== null
      && series.series_id === `series:${seriesIndex}:${suffix}`
      && moderatorValuesValid(series.moderator_values)
      && series.points.length === 25
      && series.points.every((point, index) => finite(point.predictor_raw)
        && finite(point.predicted_raw)
        && finite(point.confidence_interval_lower)
        && finite(point.confidence_interval_upper)
        && point.confidence_interval_lower <= point.confidence_interval_upper
        && (index === 0 || series.points[index - 1].predictor_raw < point.predictor_raw));
    }));
}

function johnsonNeymanInvalidCovariance(
  graph: ProcessGraphAnalysis,
  row: Extract<ProcessJohnsonNeymanAnalysis, { status: "unavailable" }>,
): boolean {
  const moderation = graph.moderations.find((candidate) => candidate.moderation_id === row.moderation_id);
  const equation = moderation
    ? graph.equations.find((candidate) => candidate.outcome === moderation.to)
    : undefined;
  const profile = graph.variable_profiles.find((candidate) => candidate.variable === row.solved_moderator);
  if (!moderation || !equation || !profile || profile.scale !== "continuous") return false;
  const probes = new Map(row.conditioning_values.map((value) => [value.variable, value.coded_value]));
  const slopeWeights = (solvedValue: number) => equation.coefficients.map((coefficient) => {
    const conditioning = moderation.conditioning_moderator
      ? (probes.get(moderation.conditioning_moderator) ?? 0)
      : 0;
    if (coefficient.variables.length === 1 && coefficient.variables[0] === moderation.from) return 1;
    if (coefficient.variables.length === 2
      && coefficient.variables[0] === moderation.from
      && coefficient.variables[1] === moderation.moderator) return solvedValue;
    if (moderation.conditioning_moderator
      && coefficient.variables.length === 2
      && coefficient.variables[0] === moderation.from
      && coefficient.variables[1] === moderation.conditioning_moderator) return conditioning;
    if (moderation.conditioning_moderator
      && coefficient.variables.length === 3
      && coefficient.variables[0] === moderation.from
      && coefficient.variables[1] === moderation.moderator
      && coefficient.variables[2] === moderation.conditioning_moderator) return solvedValue * conditioning;
    return 0;
  });
  const zero = slopeWeights(0);
  const one = slopeWeights(1);
  const delta = one.map((value, index) => value - zero[index]);
  const covarianceForm = (left: readonly number[], right: readonly number[]) => left.reduce(
    (sum, leftWeight, leftIndex) => sum + right.reduce(
      (inner, rightWeight, rightIndex) => inner
        + leftWeight * equation.coefficient_covariance[leftIndex][rightIndex] * rightWeight,
      0,
    ),
    0,
  );
  const v0 = covarianceForm(zero, zero);
  const v1 = covarianceForm(zero, delta);
  const v2 = covarianceForm(delta, delta);
  if (![v0, v1, v2].every(finite)) return true;
  const codedMin = profile.raw_min - profile.raw_mean;
  const codedMax = profile.raw_max - profile.raw_mean;
  const variance = (coded: number) => v0 + 2 * v1 * coded + v2 * coded * coded;
  if (!(variance(codedMin) > 0) || !(variance(codedMax) > 0)) return true;
  if (v2 > 0) {
    const vertex = -v1 / v2;
    if (vertex > codedMin && vertex < codedMax && !(variance(vertex) > 0)) return true;
  }
  return false;
}

function johnsonNeymanValid(graph: ProcessGraphAnalysis): boolean {
  const moderationIds = new Set(graph.moderations.map((moderation) => moderation.moderation_id));
  return graph.johnson_neyman.every((jn) => {
    if (!moderationIds.has(jn.moderation_id)
      || !hasText(jn.solved_moderator)
      || !moderatorValuesValid(jn.conditioning_values)) return false;
    if (jn.status === "unavailable") {
      const profile = graph.variable_profiles.find((candidate) => candidate.variable === jn.solved_moderator);
      if (profile?.scale === "binary_0_1") {
        return jn.reason_code === "binary_solved_moderator"
          && jn.message === "Johnson-Neyman regions require a continuous solved moderator.";
      }
      return jn.reason_code === "invalid_hc3_covariance"
        && jn.message === PROCESS_JN_INVALID_COVARIANCE_MESSAGE
        && johnsonNeymanInvalidCovariance(graph, jn);
    }
    return finite(jn.raw_min)
      && finite(jn.raw_max)
      && jn.raw_min <= jn.raw_max
      && jn.roots.every((root) => finite(root) && root >= jn.raw_min && root <= jn.raw_max)
      && jn.roots.every((root, index) => index === 0 || jn.roots[index - 1] <= root)
      && jn.regions.length === jn.roots.length + 1
      && jn.regions.every((region) => finite(region.lower)
        && finite(region.upper)
        && region.lower <= region.upper
        && ["significant_negative", "not_significant", "significant_positive"].includes(region.status))
      && jn.curve_points.length === 101
      && jn.curve_points.every((point, index) => finite(point.moderator_raw)
        && finite(point.effect)
        && finite(point.standard_error)
        && point.standard_error > 0
        && finite(point.confidence_interval_lower)
        && finite(point.confidence_interval_upper)
        && point.confidence_interval_lower <= point.confidence_interval_upper
        && (index === 0 || jn.curve_points[index - 1].moderator_raw < point.moderator_raw));
  });
}

function expectedBootstrapWarnings(bootstrap: ProcessBootstrapAnalysis): string[] {
  const warnings = [PROCESS_BOOTSTRAP_SCOPE_WARNING, PROCESS_BOOTSTRAP_TEST_WARNING];
  if (bootstrap.failed_replicates.length) {
    warnings.push(`${bootstrap.failed_replicates.length} of ${bootstrap.requested_replicates} PROCESS bootstrap replicates failed and were excluded from inference.`);
  }
  const failedJackknife = bootstrap.jackknife_cases - bootstrap.usable_jackknife_cases;
  if (failedJackknife > 0) {
    warnings.push(`${failedJackknife} of ${bootstrap.jackknife_cases} PROCESS delete-one fits failed; BCa intervals are explicitly unavailable.`);
  }
  return warnings;
}

function exactIndexComplement(successful: readonly number[], failed: readonly number[], total: number): boolean {
  if (!strictlyAscending(successful) || !strictlyAscending(failed)) return false;
  return exactStrings([...successful, ...failed].sort((left, right) => left - right).map(String),
    Array.from({ length: total }, (_, index) => String(index)));
}

function type7Quantile(values: readonly number[], probability: number): number {
  const ordered = [...values].sort((left, right) => left - right);
  const position = (ordered.length - 1) * probability;
  const lower = Math.floor(position);
  const fraction = position - lower;
  return ordered[lower] + fraction * (ordered[Math.min(lower + 1, ordered.length - 1)] - ordered[lower]);
}

function bootstrapValid(
  bootstrap: ProcessBootstrapAnalysis,
  graph: ProcessGraphAnalysis,
  run: AnalysisRun,
): boolean {
  const settings = run.provenance?.settings;
  const originalRows = [
    ...graph.reference_effects,
    ...graph.conditional_indirect_effects,
    ...graph.moderated_mediation_indices,
    ...graph.simple_slopes,
  ];
  const ids = originalRows.map((row) => row.effect_id);
  const witness = bootstrap.validation_witness;
  if (!settings
    || bootstrap.method_version !== NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION
    || bootstrap.algorithm !== "indexed_case_resampling_v1"
    || bootstrap.interval_policy !== "percentile_primary_bca_conditional_v1"
    || bootstrap.test_reference !== "standard_normal_bootstrap_ratio_v1"
    || bootstrap.minimum_usable_fraction !== 0.9
    || bootstrap.stream_token !== "process_indexed_case_stream_v1"
    || bootstrap.requested_replicates !== settings.bootstrap_samples
    || bootstrap.requested_replicates < 99
    || bootstrap.requested_replicates > 10_000
    || !positiveInteger(bootstrap.usable_replicates)
    || bootstrap.usable_replicates < Math.ceil(0.9 * bootstrap.requested_replicates)
    || bootstrap.usable_replicates > bootstrap.requested_replicates
    || bootstrap.failed_replicates.length !== bootstrap.requested_replicates - bootstrap.usable_replicates
    || bootstrap.jackknife_cases !== graph.complete_cases
    || !nonNegativeInteger(bootstrap.usable_jackknife_cases)
    || bootstrap.usable_jackknife_cases > bootstrap.jackknife_cases
    || bootstrap.seed !== settings.seed
    || bootstrap.seed !== run.seed
    || bootstrap.workers !== settings.workers
    || bootstrap.workers < 1
    || bootstrap.workers > 64
    || !exactStrings(bootstrap.warnings, expectedBootstrapWarnings(bootstrap))
    || !exactStrings(bootstrap.estimands.map((row) => row.effect_id), ids)
    || witness.method_version !== "regression_process_bootstrap_validation_witness_v1"
    || !exactStrings(witness.estimand_ids, ids)
    || witness.successful_bootstrap.length !== bootstrap.usable_replicates
    || witness.successful_jackknife.length !== bootstrap.usable_jackknife_cases
    || witness.failed_jackknife.length !== bootstrap.jackknife_cases - bootstrap.usable_jackknife_cases) return false;

  const validFailureCode = (reason: string) => [
    "rank_deficient_equation",
    "nonfinite_estimate",
    "invalid_binary_profile",
    "high_leverage_hc3_instability",
    "invalid_hc3_covariance",
    "degenerate_simple_slope_variance",
  ].includes(reason);
  if (!bootstrap.failed_replicates.every((failure) => nonNegativeInteger(failure.replicate_index)
      && failure.replicate_index < bootstrap.requested_replicates
      && validFailureCode(failure.reason_code)
      && hasText(failure.message))
    || !witness.failed_jackknife.every((failure) => nonNegativeInteger(failure.omitted_case)
      && failure.omitted_case < bootstrap.jackknife_cases
      && validFailureCode(failure.reason_code)
      && hasText(failure.message))) return false;
  if (!exactIndexComplement(
    witness.successful_bootstrap.map((row) => row.replicate_index),
    bootstrap.failed_replicates.map((row) => row.replicate_index),
    bootstrap.requested_replicates,
  ) || !exactIndexComplement(
    witness.successful_jackknife.map((row) => row.omitted_case),
    witness.failed_jackknife.map((row) => row.omitted_case),
    bootstrap.jackknife_cases,
  )) return false;
  const vectorsValid = (rows: readonly { estimates: number[] }[]) => rows.every((row) => (
    row.estimates.length === ids.length && row.estimates.every(finite)
  ));
  if (!vectorsValid(witness.successful_bootstrap) || !vectorsValid(witness.successful_jackknife)) return false;

  for (const [index, estimand] of bootstrap.estimands.entries()) {
    const original = originalRows[index].estimate;
    const values = witness.successful_bootstrap.map((row) => row.estimates[index]);
    const minimum = Math.min(...values);
    const maximum = Math.max(...values);
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
    const standardError = Math.sqrt(values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / (values.length - 1));
    const percentileLower = type7Quantile(values, 0.025);
    const percentileUpper = type7Quantile(values, 0.975);
    const testTolerance = 64 * Number.EPSILON * Math.max(1, Math.abs(estimand.original), ...values.map(Math.abs));
    if (!finite(estimand.original)
      || !close(estimand.original, original, 1_024)
      || !finite(estimand.bootstrap_mean)
      || !close(estimand.bootstrap_mean, mean, 4_096)
      || !finite(estimand.bias)
      || !close(estimand.bias, estimand.bootstrap_mean - estimand.original, 1_024)
      || !finite(estimand.standard_error)
      || estimand.standard_error < 0
      || !close(estimand.standard_error, standardError, 4_096)
      || !finite(estimand.percentile_lower)
      || !finite(estimand.percentile_upper)
      || !close(estimand.percentile_lower, percentileLower, 4_096)
      || !close(estimand.percentile_upper, percentileUpper, 4_096)
      || estimand.percentile_lower > estimand.percentile_upper
      || estimand.percentile_lower < minimum - 1e-12 * Math.max(1, Math.abs(minimum))
      || estimand.percentile_upper > maximum + 1e-12 * Math.max(1, Math.abs(maximum))
      || estimand.usable_replicates !== bootstrap.usable_replicates) return false;
    if (estimand.test.status === "available") {
      if (estimand.standard_error <= testTolerance
        || !finite(estimand.test.statistic)
        || !close(estimand.test.statistic, estimand.original / estimand.standard_error, 1_024)
        || !probability(estimand.test.p_value_two_sided)) return false;
    } else if (estimand.standard_error > testTolerance
      || estimand.test.reason_code !== "zero_bootstrap_standard_error"
      || !hasText(estimand.test.message)) return false;
    if (estimand.bca.status === "available") {
      if (bootstrap.usable_jackknife_cases !== bootstrap.jackknife_cases
        || !finite(estimand.bca.bias_correction)
        || !finite(estimand.bca.acceleration)
        || !finite(estimand.bca.lower)
        || !finite(estimand.bca.upper)
        || estimand.bca.lower > estimand.bca.upper
        || estimand.bca.lower < minimum - 1e-12 * Math.max(1, Math.abs(minimum))
        || estimand.bca.upper > maximum + 1e-12 * Math.max(1, Math.abs(maximum))) return false;
    } else if (!["incomplete_jackknife", "zero_jackknife_variance", "nonfinite_adjusted_probability"].includes(estimand.bca.reason_code)
      || !hasText(estimand.bca.message)
      || (bootstrap.usable_jackknife_cases < bootstrap.jackknife_cases
        && estimand.bca.reason_code !== "incomplete_jackknife")) return false;
  }
  return true;
}

function graphValid(graph: ProcessGraphAnalysis, regression: RegressionAnalysis): boolean {
  return graph.policies.centering === "equation_complete_case_mean_v1"
    && graph.policies.covariance === "hc3_v1"
    && graph.policies.inference_reference === "student_t_residual_df_v1"
    && graph.policies.confidence_level === 0.95
    && graph.complete_cases === regression.observations
    && positiveInteger(graph.complete_cases)
    && nonNegativeInteger(graph.omitted_cases)
    && variableProfilesValid(graph, regression)
    && graphPathsValid(graph)
    && equationsValid(graph)
    && effectsValid(graph)
    && plotsValid(graph)
    && johnsonNeymanValid(graph);
}

export function nativeProcessResultProjection(
  run: AnalysisRun | null | undefined,
): NativeProcessResultProjection | null {
  if (!run || run.status !== "completed" || !run.result || !run.provenance || !processShellIsExclusive(run)) return null;
  const regression = run.result.regression;
  const process = regression?.process;
  const graph = process?.graph_v2;
  const bootstrapRun = run.provenance.method_version
    === `${NATIVE_PROCESS_METHOD_VERSION}+${NATIVE_PROCESS_BOOTSTRAP_METHOD_VERSION}`;
  if (!regression || !process || !graph
    || run.provenance.method !== "regression"
    || (!bootstrapRun && run.provenance.method_version !== NATIVE_PROCESS_METHOD_VERSION)
    || run.provenance.settings.method !== "regression"
    || run.provenance.settings.weighting_scheme !== "path"
    || run.provenance.settings.preprocessing !== "unstandardized"
    || run.provenance.settings.missing_data !== "listwise_deletion"
    || run.provenance.settings.case_weight_column !== null
    || run.provenance.settings.studentized_inner_samples !== 0
    || run.provenance.settings.permutation_samples !== 0
    || run.provenance.settings.confidence_level !== 0.95
    || (bootstrapRun
      ? run.provenance.settings.bootstrap_samples < 99 || run.provenance.settings.bootstrap_samples > 10_000
      : run.provenance.settings.bootstrap_samples !== 0)
    || (bootstrapRun
      ? run.provenance.settings.workers < 1 || run.provenance.settings.workers > 64
      : run.provenance.settings.workers !== 1)
    || run.result.method_version !== NATIVE_PROCESS_METHOD_VERSION
    || regression.method_version !== NATIVE_PROCESS_METHOD_VERSION
    || regression.regression_type !== "process"
    || process.method_version !== NATIVE_PROCESS_METHOD_VERSION
    || process.model !== "graph"
    || process.effects.length !== 0
    || process.simple_slopes.length !== 0
    || !exactStrings(process.warnings, [NATIVE_PROCESS_RESULT_WARNING, NATIVE_PROCESS_INFERENCE_WARNING])
    || !exactStrings(regression.warnings, [NATIVE_PROCESS_RESULT_WARNING, NATIVE_PROCESS_INFERENCE_WARNING])
    || !exactStrings(run.result.warnings, [NATIVE_PROCESS_RESULT_WARNING, NATIVE_PROCESS_INFERENCE_WARNING])
    || run.result.used_observations !== regression.observations
    || run.result.omitted_observations !== graph.omitted_cases
    || run.result.used_observations + run.result.omitted_observations !== graph.complete_cases + graph.omitted_cases
    || !processRegressionShellValid(regression)
    || !graphValid(graph, regression)
    || run.assessment?.method_version !== "assessment_not_applicable_v1"
    || run.assessment.warnings.length !== 1
    || run.assessment.warnings[0] !== NATIVE_STANDALONE_ASSESSMENT_WARNING) return null;
  const bootstrap = graph.bootstrap ?? null;
  if ((bootstrapRun && (!bootstrap || !bootstrapValid(bootstrap, graph, run)))
    || (!bootstrapRun && bootstrap)) return null;
  return {
    methodVersion: NATIVE_PROCESS_METHOD_VERSION,
    outcome: regression.outcome,
    predictors: [...regression.predictors],
    controls: [...regression.controls],
    observations: regression.observations,
    omittedObservations: graph.omitted_cases,
    graph,
    bootstrap,
    warnings: [...process.warnings],
  };
}

export function nativeLegacyProcessResultProjection(
  run: AnalysisRun | null | undefined,
): NativeLegacyProcessResultProjection | null {
  if (!run || run.status !== "completed" || !run.result || !run.provenance || !processShellIsExclusive(run)) return null;
  const regression = run.result.regression;
  const process = regression?.process;
  if (!regression || !process
    || run.provenance.method !== "regression"
    || run.provenance.method_version !== "regression_process_v1"
    || run.result.method_version !== "regression_process_v1"
    || regression.method_version !== "regression_process_v1"
    || regression.regression_type !== "process"
    || process.method_version !== "regression_process_v1"
    || !["mediation", "moderation", "moderated_mediation"].includes(process.model)
    || process.graph_v2
    || process.effects.length < 1
    || process.effects.some((row) => !hasText(row.effect) || !finite(row.estimate)
      || (row.lower_percentile != null && !finite(row.lower_percentile))
      || (row.upper_percentile != null && !finite(row.upper_percentile)))
    || process.simple_slopes.some((row) => !finite(row.moderator_value) || !finite(row.slope))) return null;
  return {
    methodVersion: "regression_process_v1",
    model: process.model as NativeLegacyProcessResultProjection["model"],
    analysis: process,
    warnings: [...process.warnings],
  };
}

function number(value: number, digits = 6): string {
  const formatted = value.toFixed(digits);
  return Number(formatted) === 0 ? (0).toFixed(digits) : formatted;
}

function pValue(value: number): string {
  return value < 0.0001 ? "<0.0001" : number(value, 4);
}

function label(value: string): string {
  return value.replaceAll("_", " ").replace(/\b\w/g, (match) => match.toUpperCase());
}

function moderatorLabel(values: readonly ProcessModeratorValue[]): string {
  return values.map((value) => `${value.variable} = ${number(value.raw_value, 4)} (coded ${number(value.coded_value, 4)})`).join("; ");
}

function table(
  id: string,
  title: string,
  columns: string[],
  rows: string[][],
  warning: string | null = null,
  status: ResultTable["status"] = "experimental",
): ResultTable {
  return {
    id,
    title,
    status,
    warning: status === "validated"
      ? warning
      : warning ? `${NATIVE_PROCESS_PROMOTION_PENDING_WARNING} ${warning}` : NATIVE_PROCESS_PROMOTION_PENDING_WARNING,
    columns,
    rows,
  };
}

function jnRows(johnsonNeyman: readonly ProcessJohnsonNeymanAnalysis[]): string[][] {
  return johnsonNeyman.flatMap((jn) => {
    const conditioning = moderatorLabel(jn.conditioning_values);
    if (jn.status === "unavailable") {
      return [[jn.moderation_id, jn.solved_moderator, conditioning, "Unavailable", "", "", "", `${jn.reason_code}: ${jn.message}`]];
    }
    return jn.regions.map((region) => [
      jn.moderation_id,
      jn.solved_moderator,
      conditioning,
      label(region.status),
      number(region.lower),
      number(region.upper),
      jn.roots.map((root) => number(root)).join(", "),
      "",
    ]);
  });
}

export function nativeProcessResultTables(projection: NativeProcessResultProjection): ResultTable[] {
  const graph = projection.graph;
  const tables: ResultTable[] = [
    table("process_model_summary", "Model summary", ["Property", "Value"], [
      ["Method version", projection.methodVersion],
      ["Outcome", projection.outcome],
      ["Global complete cases", String(projection.observations)],
      ["Rows omitted listwise", String(projection.omittedObservations)],
      ["Directed paths", String(graph.paths.length)],
      ["Moderated paths", String(graph.moderations.length)],
      ["OLS equations", String(graph.equations.length)],
      ["Covariance", "HC3"],
      ["Inference", "Two-sided 95% Student-t using residual df"],
      ["Centering", "Equation complete-case mean for continuous product participants"],
      ["Bootstrap", projection.bootstrap ? `${projection.bootstrap.usable_replicates} usable of ${projection.bootstrap.requested_replicates}` : "Off"],
    ]),
    table("process_paths", "Directed paths", ["Path ID", "From", "To", "Moderation"], graph.paths.map((path) => {
      const moderation = graph.moderations.find((row) => row.from === path.from && row.to === path.to);
      return [path.path_id, path.from, path.to, moderation
        ? [moderation.moderator, moderation.conditioning_moderator].filter(Boolean).join(" x ")
        : "None"];
    })),
    table("process_equation_coefficients", "Equation coefficients", [
      "Equation", "Term", "Kind", "Variables", "Estimate", "SE (HC3)", "t", "p (two-sided)", "95% CI lower", "95% CI upper",
    ], graph.equations.flatMap((equation) => equation.coefficients.map((row) => [
      equation.outcome,
      row.term_id,
      label(row.kind),
      row.variables.join(" x ") || "Intercept",
      number(row.estimate),
      number(row.standard_error),
      number(row.statistic, 4),
      pValue(row.p_value_two_sided),
      number(row.confidence_interval_lower),
      number(row.confidence_interval_upper),
    ]))),
    table("process_equation_fit", "Equation fit", ["Equation", "Residual df", "R-squared", "Adjusted R-squared", "F", "AIC", "BIC", "RMSE"], graph.equations.map((equation) => [
      equation.outcome,
      String(equation.residual_degrees_of_freedom),
      number(equation.fit.r_squared),
      number(equation.fit.adjusted_r_squared),
      number(equation.fit.f_statistic),
      number(equation.fit.aic),
      number(equation.fit.bic),
      number(equation.fit.rmse),
    ])),
    table("process_reference_effects", "Reference effects", ["Effect ID", "Kind", "Path", "Estimate", "Reference condition"], graph.reference_effects.map((row) => [
      row.effect_id, label(row.kind), row.path.join(" -> "), number(row.estimate), NATIVE_PROCESS_REFERENCE_CONDITION,
    ])),
  ];
  if (graph.conditional_indirect_effects.length) tables.push(table(
    "process_conditional_indirect_effects",
    "Conditional indirect effects",
    ["Effect ID", "Path", "Moderator probe(s)", "Estimate"],
    graph.conditional_indirect_effects.map((row) => [row.effect_id, row.path_id.replaceAll("->", " -> "), moderatorLabel(row.moderator_values), number(row.estimate)]),
  ));
  if (graph.moderated_mediation_indices.length) tables.push(table(
    "process_moderated_mediation_indices",
    "Moderated-mediation indices",
    ["Effect ID", "Indirect path", "Moderated edge", "Moderator", "Estimate"],
    graph.moderated_mediation_indices.map((row) => [row.effect_id, row.path_id.replaceAll("->", " -> "), row.moderated_edge.replaceAll("->", " -> "), row.moderator, number(row.estimate)]),
  ));
  if (graph.simple_slopes.length) tables.push(table(
    "process_simple_slopes",
    "Simple slopes and conditional plots",
    ["Effect ID", "Moderation", "Moderator probe(s)", "Estimate", "SE (HC3)", "t", "p (two-sided)", "95% CI lower", "95% CI upper"],
    graph.simple_slopes.map((row) => [row.effect_id, row.moderation_id, moderatorLabel(row.moderator_values), number(row.estimate), number(row.standard_error), number(row.statistic, 4), pValue(row.p_value_two_sided), number(row.confidence_interval_lower), number(row.confidence_interval_upper)]),
  ));
  if (graph.plots.length) tables.push(table(
    "process_conditional_plot_points",
    "Conditional outcome plot data",
    ["Plot ID", "Moderation", "Series ID", "Moderator probe(s)", "Point", "Predictor raw", "Predicted outcome raw", "95% CI lower", "95% CI upper"],
    graph.plots.flatMap((plot) => plot.series.flatMap((series) => series.points.map((point, pointIndex) => [
      plot.plot_id,
      plot.moderation_id,
      series.series_id,
      moderatorLabel(series.moderator_values),
      String(pointIndex + 1),
      number(point.predictor_raw),
      number(point.predicted_raw),
      number(point.confidence_interval_lower),
      number(point.confidence_interval_upper),
    ]))),
    "Exact engine-persisted plot points; no chart values are reconstructed by the exporter.",
  ));
  if (graph.johnson_neyman.length) tables.push(table(
    "process_johnson_neyman",
    "Johnson-Neyman regions",
    ["Moderation", "Solved moderator", "Conditioning probe(s)", "Status", "Raw lower", "Raw upper", "Roots", "Reason"],
    jnRows(graph.johnson_neyman),
  ));
  const availableJohnsonNeyman = graph.johnson_neyman.filter(
    (row): row is Extract<ProcessJohnsonNeymanAnalysis, { status: "available" }> => row.status === "available",
  );
  if (availableJohnsonNeyman.length) tables.push(table(
    "process_johnson_neyman_curve_points",
    "Johnson-Neyman curve data",
    ["Moderation", "Solved moderator", "Conditioning probe(s)", "Point", "Moderator raw", "Effect", "SE", "95% CI lower", "95% CI upper"],
    availableJohnsonNeyman.flatMap((jn) => jn.curve_points.map((point, pointIndex) => [
      jn.moderation_id,
      jn.solved_moderator,
      moderatorLabel(jn.conditioning_values),
      String(pointIndex + 1),
      number(point.moderator_raw),
      number(point.effect),
      number(point.standard_error),
      number(point.confidence_interval_lower),
      number(point.confidence_interval_upper),
    ])),
    "Exact engine-persisted Johnson-Neyman curve points; bootstrap validation witnesses are never exported.",
  ));
  if (projection.bootstrap) {
    const bootstrap = projection.bootstrap;
    tables.push(table("process_bootstrap_summary", "Bootstrap summary", ["Property", "Value"], [
      ["Method version", bootstrap.method_version],
      ["Algorithm", "Indexed case resampling with replacement"],
      ["Interval policy", "Percentile primary; BCa conditional"],
      ["Test reference", "Two-sided standard-normal bootstrap ratio"],
      ["Confidence level", "0.95 (fixed)"],
      ["Requested replicates", String(bootstrap.requested_replicates)],
      ["Usable replicates", String(bootstrap.usable_replicates)],
      ["Failed replicates", String(bootstrap.failed_replicates.length)],
      ["Delete-one fits usable / required", `${bootstrap.usable_jackknife_cases} / ${bootstrap.jackknife_cases}`],
      ["Seed", String(bootstrap.seed)],
      ["Workers", String(bootstrap.workers)],
      ["Stream", bootstrap.stream_token],
      ["Probe grid", "Original-sample raw moderator probes; each resample and delete-one equation re-centered internally"],
    ], bootstrap.warnings.join(" ")));
    tables.push(table("process_bootstrap_failures", "Bootstrap failures", ["Replicate", "Reason code", "Message"],
      bootstrap.failed_replicates.length
        ? bootstrap.failed_replicates.map((row) => [String(row.replicate_index), row.reason_code, row.message])
        : [["", "No failed replicates", "Every requested replicate produced a usable estimand vector."]],
    ));
    tables.push(table("process_bootstrap_inference", "Bootstrap inference", [
      "Effect ID", "Original", "Bootstrap mean", "Bias", "SE", "Bootstrap ratio", "p (two-sided normal)", "Test status", "Percentile lower", "Percentile upper", "Usable",
    ], bootstrap.estimands.map((row) => [
      row.effect_id,
      number(row.original),
      number(row.bootstrap_mean),
      number(row.bias),
      number(row.standard_error),
      row.test.status === "available" ? number(row.test.statistic, 4) : "",
      row.test.status === "available" ? pValue(row.test.p_value_two_sided) : "",
      row.test.status === "available" ? "Available" : `Unavailable - ${row.test.reason_code}: ${row.test.message}`,
      number(row.percentile_lower),
      number(row.percentile_upper),
      String(row.usable_replicates),
    ])));
    tables.push(table("process_bootstrap_bca", "Bootstrap BCa intervals", [
      "Effect ID", "Status", "Bias correction", "Acceleration", "BCa lower", "BCa upper", "Reason",
    ], bootstrap.estimands.map((row) => row.bca.status === "available"
      ? [row.effect_id, "Available", number(row.bca.bias_correction), number(row.bca.acceleration), number(row.bca.lower), number(row.bca.upper), ""]
      : [row.effect_id, "Unavailable", "", "", "", "", `${row.bca.reason_code}: ${row.bca.message}`])));
  }
  tables.push(table("process_scope", "Scope and provenance", ["Item", "Disclosure"], [
    ["Implementation", NATIVE_PROCESS_RESULT_WARNING],
    ["Inference", NATIVE_PROCESS_INFERENCE_WARNING],
    ["Supported scope", NATIVE_PROCESS_SCOPE_NOTE],
    ["Plot provenance", "Conditional and Johnson-Neyman charts use only persisted engine-produced points and intervals; the UI does not recompute scientific values."],
  ]));
  return tables;
}

export function nativeLegacyProcessResultTables(projection: NativeLegacyProcessResultProjection): ResultTable[] {
  const historicalTable = (
    id: string,
    title: string,
    columns: string[],
    rows: string[][],
    warning: string,
  ): ResultTable => ({ id, title, columns, rows, status: "experimental", warning });
  const tables = [historicalTable("legacy_process_effects", "Historical PROCESS v1 effects", ["Effect", "Estimate", "Percentile lower", "Percentile upper"], projection.analysis.effects.map((row) => [
    row.effect,
    number(row.estimate),
    row.lower_percentile == null ? "" : number(row.lower_percentile),
    row.upper_percentile == null ? "" : number(row.upper_percentile),
  ]), "Historical read-only regression_process_v1 output; it is displayed under its original method label and is not current PROCESS v2 parity evidence.")];
  if (projection.analysis.simple_slopes.length) tables.push(historicalTable(
    "legacy_process_simple_slopes",
    "Historical PROCESS v1 simple slopes",
    ["Moderator value", "Slope"],
    projection.analysis.simple_slopes.map((row) => [number(row.moderator_value), number(row.slope)]),
    "Historical read-only regression_process_v1 output; fixed standardized probes from v1 are not reinterpreted as raw PROCESS v2 probes.",
  ));
  tables.push(historicalTable("legacy_process_scope", "Historical PROCESS v1 scope", ["Item", "Disclosure"], [
    ["Method version", projection.methodVersion],
    ["Model", label(projection.model)],
    ["Status", "Readable historical output only; create a graph-defined PROCESS v2 recipe for current evidence."],
    ...projection.warnings.map((warning) => ["Recorded warning", warning]),
  ], "Historical read-only archive output; no current execution, parity, or validation claim is made."));
  return tables;
}
