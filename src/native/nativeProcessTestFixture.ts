import type {
  AnalysisResultEnvelope,
  AnalysisRun,
  NativeCanonicalAnalysisRecipe,
  ProcessBootstrapAnalysis,
  ProcessGraphAnalysis,
} from "../types";
import { NATIVE_STANDALONE_ASSESSMENT_WARNING } from "./nativeNca";
import {
  NATIVE_PROCESS_INFERENCE_WARNING,
  NATIVE_PROCESS_RESULT_WARNING,
} from "./nativeProcess";

const OBSERVATIONS = 8;

function covariance(standardErrors: readonly number[]): number[][] {
  return Array.from({ length: standardErrors.length }, (_, row) => Array.from({ length: standardErrors.length }, (_, column) => (
    row === column ? standardErrors[row] ** 2 : 0
  )));
}

function plotPoints(intercept: number, slope: number) {
  return Array.from({ length: 25 }, (_, index) => {
    const predictor = -2 + index / 6;
    const predicted = intercept + slope * predictor;
    return {
      predictor_raw: predictor,
      predicted_raw: predicted,
      confidence_interval_lower: predicted - 0.2,
      confidence_interval_upper: predicted + 0.2,
    };
  });
}

function jnPoints() {
  return Array.from({ length: 101 }, (_, index) => {
    const moderator = -2 + index * 0.04;
    const effect = 0.6 + 0.1 * moderator;
    return {
      moderator_raw: moderator,
      effect,
      standard_error: 0.1,
      confidence_interval_lower: effect - 0.2,
      confidence_interval_upper: effect + 0.2,
    };
  });
}

function type7(values: readonly number[], probability: number): number {
  const ordered = [...values].sort((left, right) => left - right);
  const position = (ordered.length - 1) * probability;
  const lower = Math.floor(position);
  const fraction = position - lower;
  return ordered[lower] + fraction * (ordered[Math.min(lower + 1, ordered.length - 1)] - ordered[lower]);
}

function processGraph(): ProcessGraphAnalysis {
  const moderationId = "moderation:X->Y@W";
  return {
    policies: {
      centering: "equation_complete_case_mean_v1",
      covariance: "hc3_v1",
      inference_reference: "student_t_residual_df_v1",
      confidence_level: 0.95,
    },
    complete_cases: OBSERVATIONS,
    omitted_cases: 2,
    variable_profiles: [
      { variable: "X", role: "focal_predictor", scale: "continuous", raw_mean: 0, raw_sample_sd: 1, raw_min: -2, raw_max: 2, levels: [] },
      { variable: "W", role: "moderator", scale: "continuous", raw_mean: 0, raw_sample_sd: 1, raw_min: -2, raw_max: 2, levels: [] },
      { variable: "Y", role: "outcome", scale: "continuous", raw_mean: 0.3, raw_sample_sd: 1.2, raw_min: -2.1, raw_max: 2.4, levels: [] },
    ],
    paths: [{ path_id: "X->Y", from: "X", to: "Y" }],
    moderations: [{ moderation_id: moderationId, from: "X", to: "Y", moderator: "W" }],
    equations: [{
      equation_id: "equation:Y",
      outcome: "Y",
      term_ids: ["intercept", "X", "W", "X*W"],
      coefficients: [
        { term_id: "intercept", kind: "intercept", variables: [], estimate: 0.2, standard_error: 0.1, statistic: 2, p_value_two_sided: 0.09, confidence_interval_lower: -0.05, confidence_interval_upper: 0.45 },
        { term_id: "X", kind: "path", variables: ["X"], estimate: 0.6, standard_error: 0.1, statistic: 6, p_value_two_sided: 0.002, confidence_interval_lower: 0.35, confidence_interval_upper: 0.85 },
        { term_id: "W", kind: "moderator_main", variables: ["W"], estimate: 0.15, standard_error: 0.1, statistic: 1.5, p_value_two_sided: 0.19, confidence_interval_lower: -0.1, confidence_interval_upper: 0.4 },
        { term_id: "X*W", kind: "interaction", variables: ["X", "W"], estimate: 0.1, standard_error: 0.05, statistic: 2, p_value_two_sided: 0.09, confidence_interval_lower: -0.025, confidence_interval_upper: 0.225 },
      ],
      coefficient_covariance: covariance([0.1, 0.1, 0.1, 0.05]),
      residual_degrees_of_freedom: 4,
      fit: {
        observations: OBSERVATIONS,
        parameter_count: 4,
        residual_sum_squares: 1.28,
        total_sum_squares: 2.8444444444444446,
        r_squared: 0.55,
        adjusted_r_squared: 0.2125,
        f_statistic: 1.6296296296296298,
        aic: -6.660651709986484,
        bic: -6.34288554326714,
        rmse: 0.4,
      },
    }],
    reference_effects: [
      { effect_id: "direct:X->Y", kind: "direct", path: ["X", "Y"], estimate: 0.6 },
      { effect_id: "total_indirect:X->Y", kind: "total_indirect", path: ["X", "Y"], estimate: 0 },
      { effect_id: "total:X->Y", kind: "total", path: ["X", "Y"], estimate: 0.6 },
    ],
    conditional_indirect_effects: [],
    moderated_mediation_indices: [],
    simple_slopes: [-1, 0, 1].map((rawValue, index) => {
      const estimate = 0.6 + 0.1 * rawValue;
      const semanticProbe = ["minus_1sd", "mean", "plus_1sd"][index];
      return {
        effect_id: `slope:${moderationId}@W=${semanticProbe}`,
        moderation_id: moderationId,
        moderator_values: [{ variable: "W", raw_value: rawValue, coded_value: rawValue }],
        estimate,
        standard_error: 0.1,
        statistic: estimate / 0.1,
        p_value_two_sided: 0.01,
        confidence_interval_lower: estimate - 0.25,
        confidence_interval_upper: estimate + 0.25,
      };
    }),
    plots: [{
      plot_id: `plot:${moderationId}`,
      moderation_id: moderationId,
      series: [-1, 0, 1].map((rawValue, index) => ({
        series_id: `series:${index}:W=${["minus_1sd", "mean", "plus_1sd"][index]}`,
        moderator_values: [{ variable: "W", raw_value: rawValue, coded_value: rawValue }],
        points: plotPoints(0.2 + 0.15 * rawValue, 0.6 + 0.1 * rawValue),
      })),
    }],
    johnson_neyman: [{
      status: "available",
      moderation_id: moderationId,
      solved_moderator: "W",
      conditioning_values: [],
      raw_min: -2,
      raw_max: 2,
      roots: [],
      regions: [{ lower: -2, upper: 2, status: "significant_positive" }],
      curve_points: jnPoints(),
    }],
    bootstrap: null,
  };
}

function processBootstrap(graph: ProcessGraphAnalysis): ProcessBootstrapAnalysis {
  const originals = [
    ...graph.reference_effects,
    ...graph.conditional_indirect_effects,
    ...graph.moderated_mediation_indices,
    ...graph.simple_slopes,
  ];
  const successfulBootstrap = Array.from({ length: 99 }, (_, replicateIndex) => ({
    replicate_index: replicateIndex,
    estimates: originals.map((row, index) => row.estimate + (replicateIndex - 49) * (index + 1) * 0.0005),
  }));
  const successfulJackknife = Array.from({ length: OBSERVATIONS }, (_, omittedCase) => ({
    omitted_case: omittedCase,
    estimates: originals.map((row, index) => row.estimate + (omittedCase - 3.5) * (index + 1) * 0.0002),
  }));
  return {
    method_version: "regression_process_bootstrap_v1",
    algorithm: "indexed_case_resampling_v1",
    interval_policy: "percentile_primary_bca_conditional_v1",
    test_reference: "standard_normal_bootstrap_ratio_v1",
    requested_replicates: 99,
    usable_replicates: 99,
    minimum_usable_fraction: 0.9,
    jackknife_cases: OBSERVATIONS,
    usable_jackknife_cases: OBSERVATIONS,
    seed: 20260812,
    workers: 2,
    stream_token: "process_indexed_case_stream_v1",
    failed_replicates: [],
    estimands: originals.map((row, index) => {
      const values = successfulBootstrap.map((replicate) => replicate.estimates[index]);
      const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
      const variance = values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / (values.length - 1);
      const standardError = Math.sqrt(variance);
      return {
        effect_id: row.effect_id,
        original: row.estimate,
        bootstrap_mean: mean,
        bias: mean - row.estimate,
        standard_error: standardError,
        test: { status: "available", statistic: row.estimate / standardError, p_value_two_sided: row.estimate === 0 ? 1 : 0.01 } as const,
        percentile_lower: type7(values, 0.025),
        percentile_upper: type7(values, 0.975),
        bca: { status: "available", bias_correction: 0, acceleration: 0, lower: type7(values, 0.025), upper: type7(values, 0.975) } as const,
        usable_replicates: 99,
      };
    }),
    validation_witness: {
      method_version: "regression_process_bootstrap_validation_witness_v1",
      estimand_ids: originals.map((row) => row.effect_id),
      successful_bootstrap: successfulBootstrap,
      successful_jackknife: successfulJackknife,
      failed_jackknife: [],
    },
    warnings: [
      "PROCESS bootstrap v1 uses deterministic indexed complete-case resampling with replacement; percentile intervals are primary and BCa intervals require every delete-one fit.",
      "PROCESS bootstrap ratio tests use the original effect divided by its bootstrap standard error with a fixed two-sided standard-normal reference.",
    ],
  };
}

export function processV2Run(withBootstrap = false): AnalysisRun {
  const graph = processGraph();
  if (withBootstrap) graph.bootstrap = processBootstrap(graph);
  const methodVersion = withBootstrap
    ? "regression_process_v2+regression_process_bootstrap_v1"
    : "regression_process_v2";
  const warnings = [NATIVE_PROCESS_RESULT_WARNING, NATIVE_PROCESS_INFERENCE_WARNING];
  return {
    id: withBootstrap ? "process-v2-bootstrap-run" : "process-v2-run",
    modelId: null,
    name: withBootstrap ? "Graph-defined path analysis with bootstrap run" : "Graph-defined path analysis run",
    method: withBootstrap ? "Graph-Defined Path Analysis with Bootstrap" : "Graph-Defined Path Analysis",
    createdAt: "2026-08-12T12:00:00.000Z",
    seed: 20260812,
    status: "completed",
    warnings,
    fingerprint: "process-v2-fingerprint".slice(0, 12),
    result: {
      method_version: "regression_process_v2",
      converged: true,
      iterations: 0,
      used_observations: OBSERVATIONS,
      omitted_observations: 2,
      outer_estimates: [],
      paths: [],
      effects: [],
      regression: {
        method_version: "regression_process_v2",
        regression_type: "process",
        outcome: "Y",
        predictors: ["X", "W"],
        controls: [],
        observations: OBSERVATIONS,
        coefficients: [],
        fit: null,
        predictions: [],
        process: {
          method_version: "regression_process_v2",
          model: "graph",
          effects: [],
          simple_slopes: [],
          warnings,
          graph_v2: graph,
        },
        warnings,
      },
      r_squared: {},
      warnings,
    },
    assessment: {
      method_version: "assessment_not_applicable_v1",
      construct_quality: [],
      cross_loadings: [],
      fornell_larcker: { constructs: [], values: [] },
      r_squared: {},
      structural_quality: [],
      structural_vif: [],
      formative_indicator_vif: [],
      f_squared: [],
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    },
    provenance: {
      recipe_id: withBootstrap ? "process-v2-bootstrap-recipe" : "process-v2-recipe",
      dataset_fingerprint: "process-v2-fingerprint",
      method: "regression",
      method_version: methodVersion,
      engine_version: "test-engine",
      seed: 20260812,
      settings: {
        method: "regression",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3000,
        bootstrap_samples: withBootstrap ? 99 : 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20260812,
        workers: withBootstrap ? 2 : 1,
        confidence_level: 0.95,
        preprocessing: "unstandardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-12T11:59:58.000Z",
      completed_at: "2026-08-12T12:00:00.000Z",
    },
  };
}

export function processV2RunWithHighLeverageBootstrapFailure(): AnalysisRun {
  const run = processV2Run(true);
  const bootstrap = run.result!.regression!.process!.graph_v2!.bootstrap!;
  const [failed] = bootstrap.validation_witness.successful_bootstrap.splice(49, 1);
  bootstrap.usable_replicates = 98;
  bootstrap.failed_replicates = [{
    replicate_index: failed.replicate_index,
    reason_code: "high_leverage_hc3_instability",
    message: "PROCESS equation Y has unstable HC3 leverage in this resample.",
  }];
  bootstrap.estimands.forEach((estimand, estimandIndex) => {
    const values = bootstrap.validation_witness.successful_bootstrap.map((row) => row.estimates[estimandIndex]);
    const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
    const standardError = Math.sqrt(values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / (values.length - 1));
    estimand.bootstrap_mean = mean;
    estimand.bias = mean - estimand.original;
    estimand.standard_error = standardError;
    estimand.test = {
      status: "available",
      statistic: estimand.original / standardError,
      p_value_two_sided: estimand.original === 0 ? 1 : 0.01,
    };
    estimand.percentile_lower = type7(values, 0.025);
    estimand.percentile_upper = type7(values, 0.975);
    estimand.usable_replicates = 98;
  });
  bootstrap.warnings.push(
    "1 of 99 PROCESS bootstrap replicates failed and were excluded from inference.",
  );
  return run;
}

export function processV2Recipe(withBootstrap = false): NativeCanonicalAnalysisRecipe {
  const run = processV2Run(withBootstrap);
  return {
    schema_version: 3,
    id: run.provenance!.recipe_id,
    created_at: "2026-08-12T11:59:57.000Z",
    dataset_fingerprint: run.provenance!.dataset_fingerprint,
    model: { id: "standalone-process", name: "Standalone PROCESS", constructs: [], paths: [], controls: [], higher_order_constructs: [], interactions: [] },
    settings: { ...run.provenance!.settings },
    method_config: {
      kind: "regression",
      outcome: "Y",
      predictors: ["X", "W"],
      model: {
        type: "process",
        relationship: {
          model: "graph",
          focal_predictor: "X",
          paths: [{ from: "X", to: "Y" }],
          moderators: [{ variable: "W", scale: "continuous" }],
          moderations: [{ from: "X", to: "Y", moderator: "W" }],
          continuous_product_centering: "equation_complete_case_mean_v1",
        },
      },
      ...(withBootstrap ? { bootstrap: { algorithm: "case_resampling", intervals: ["percentile", "bca"] } } : {}),
    },
    metadata: {},
  };
}

export function processV2Envelope(withBootstrap = false): AnalysisResultEnvelope {
  const run = processV2Run(withBootstrap);
  return {
    schema_version: 5,
    id: run.id,
    status: "completed",
    provenance: run.provenance!,
    diagnostics: [],
    payload: { kind: "pls_pm_v1", estimation: run.result!, assessment: run.assessment! },
  };
}
