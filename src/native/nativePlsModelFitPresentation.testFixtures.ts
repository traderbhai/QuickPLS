import { completedSamplePlsRun } from "../data/smokeRun";
import type {
  AnalysisRun,
  PlsModelFit,
  PlsModelFitExactCriterion,
  PlsModelFitExactInference,
  PlsModelFitExactVariantInference,
} from "../types";
import {
  PLS_MODEL_FIT_EXACT_INFERENCE_PROCEDURE_V2,
  PLS_MODEL_FIT_GEODESIC_LOGARITHM_V2,
  PLS_MODEL_FIT_MATRIX_CONVENTION_V2,
  PLS_MODEL_FIT_METHOD_VERSION_V2,
} from "./nativeResults";

export type NativePlsModelFitPresentationFixture =
  | "not_run"
  | "available"
  | "partial"
  | "unavailable"
  | "failed";

function identity(dimension: number): number[][] {
  return Array.from({ length: dimension }, (_, row) => (
    Array.from({ length: dimension }, (_, column) => row === column ? 1 : 0)
  ));
}

function exactVariant(
  variant: "saturated" | "estimated",
  matrix: number[][],
  replicates: number,
): PlsModelFitExactVariantInference {
  const ledger = Array.from({ length: replicates }, (_, replicate_index) => ({
    replicate_index,
    sample_indices_sha256: "a".repeat(64),
    status: "success" as const,
    srmr: 0,
    d_uls: 0,
    d_g: 0,
    criterion_failures: [],
    failure_reason_code: null,
    failure_message: null,
  }));
  const summary = (criterion: PlsModelFitExactCriterion) => ({
    criterion,
    status: "available" as const,
    original: 0,
    requested_replicates: replicates,
    minimum_usable_replicates: 900,
    usable_replicates: replicates,
    failed_replicates: 0,
    usable_replicate_indices_sha256: "b".repeat(64),
    replicate_min: 0,
    replicate_max: 0,
    upper_95: 0,
    upper_99: 0,
    not_rejected_95: true,
    not_rejected_99: true,
    exceed_or_equal_count: replicates,
    empirical_upper_tail_probability: 1,
    unavailable_reason_code: null,
  });
  return {
    variant,
    status: "available",
    operation: `pls_model_fit_exact_${variant}_v1`,
    target_correlation_sha256: "c".repeat(64),
    transformed_correlation: structuredClone(matrix),
    transformed_correlation_sha256: "d".repeat(64),
    transformation_max_abs_error: 0,
    requested_replicates: replicates,
    ledger,
    criteria: [summary("srmr"), summary("d_uls"), summary("d_g")],
  };
}

export function currentModelFitRun(): AnalysisRun {
  const run = structuredClone(completedSamplePlsRun());
  const result = run.result!;
  const indicatorOrder = result.outer_estimates.map((estimate) => estimate.indicator);
  const matrix = identity(indicatorOrder.length);
  const zero = { status: "available" as const, value: 0 };
  const unavailableNfi = {
    status: "unavailable" as const,
    reason_code: "model_fit.null_model_chi_square_zero",
  };
  const measure = {
    srmr: 0,
    d_uls: 0,
    d_g: zero,
    chi_square: zero,
    degrees_of_freedom: { status: "available" as const, value: 1 },
    nfi: unavailableNfi,
  };
  const fit: PlsModelFit = {
    method_version: PLS_MODEL_FIT_METHOD_VERSION_V2,
    analytical_sample_size: result.used_observations,
    indicator_order: indicatorOrder,
    matrix_convention: PLS_MODEL_FIT_MATRIX_CONVENTION_V2,
    geodesic_logarithm: PLS_MODEL_FIT_GEODESIC_LOGARITHM_V2,
    observed_correlation: matrix,
    saturated_implied_correlation: structuredClone(matrix),
    estimated_implied_correlation: structuredClone(matrix),
    null_model_chi_square: zero,
    saturated: structuredClone(measure),
    estimated: structuredClone(measure),
    exact_fit_inference: {
      procedure: PLS_MODEL_FIT_EXACT_INFERENCE_PROCEDURE_V2,
      status: "unavailable",
      reason_code: "model_fit.adapted_bollen_stine_not_implemented",
    },
  };
  run.assessment!.model_fit = fit;
  return run;
}

export function currentExactFitRun(): AnalysisRun {
  const run = currentModelFitRun();
  const point = run.assessment!.model_fit!;
  const replicates = 999;
  const exact: PlsModelFitExactInference = {
    method_version: "pls_model_fit_exact_v1",
    point_fit_method_version: "pls_model_fit_v2",
    estimator_method_version: run.result!.method_version,
    resampling_method_version: "indexed_resampling_v4",
    procedure: "adapted_bollen_stine_saturated_and_estimated_v1",
    transformation: "centered_standardized_x_times_s_inverse_half_times_sigma_half_v1",
    matrix_power: "symmetric_self_adjoint_positive_definite_eigendecomposition_v1",
    quantile_method: "hyndman_fan_type7_v1",
    decision_rule: "original_less_than_or_equal_to_upper_quantile_not_rejected_v1",
    retry_policy: "no_retry_no_replacement_fixed_indexed_draws_v1",
    sample_digest_method: "sha256_u64_le_v1",
    usable_index_digest_method: "sha256_u32_le_v1",
    matrix_digest_method: "sha256_f64_bits_row_major_v1",
    status: "available",
    analytical_sample_size: run.result!.used_observations,
    indicator_order: [...point.indicator_order!],
    master_seed: run.seed,
    requested_replicates: replicates,
    minimum_usable_fraction: 0.9,
    observed_correlation_sha256: "e".repeat(64),
    saturated: exactVariant("saturated", point.saturated_implied_correlation!, replicates),
    estimated: exactVariant("estimated", point.estimated_implied_correlation!, replicates),
  };
  run.bootstrap!.method_version = "indexed_resampling_v4";
  run.bootstrap!.plan = {
    replicates,
    master_seed: run.seed,
    operation: "pls_pm_bootstrap_v1",
  };
  run.bootstrap!.model_fit_exact_inference = exact;
  run.provenance = {
    recipe_id: "recipe-exact",
    dataset_fingerprint: run.fingerprint,
    method: "pls_pm",
    method_version: `${run.result!.method_version}+assessment_v8+indexed_resampling_v4+pls_model_fit_exact_v1`,
    engine_version: "2.46.0",
    seed: run.seed,
    settings: {
      method: "pls_pm",
      weighting_scheme: "path",
      tolerance: 1e-7,
      max_iterations: 300,
      bootstrap_samples: replicates,
      studentized_inner_samples: 99,
      permutation_samples: 0,
      seed: run.seed,
      workers: 2,
      confidence_level: 0.95,
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      case_weight_column: null,
    },
    started_at: run.createdAt,
    completed_at: run.createdAt,
  };
  return run;
}

function makeExactCriterionUnavailable(
  variant: PlsModelFitExactVariantInference,
  criterion: PlsModelFitExactCriterion,
) {
  for (const entry of variant.ledger) {
    entry[criterion] = null;
    entry.criterion_failures.push({
      criterion,
      reason_code: "model_fit_exact.test_criterion_unavailable",
    });
    const usable = [entry.srmr, entry.d_uls, entry.d_g]
      .filter((value) => typeof value === "number").length;
    entry.status = usable === 0 ? "failed" : "partial";
  }
  const summary = variant.criteria.find((candidate) => candidate.criterion === criterion)!;
  Object.assign(summary, {
    status: "unavailable",
    usable_replicates: 0,
    failed_replicates: variant.requested_replicates,
    replicate_min: null,
    replicate_max: null,
    upper_95: null,
    upper_99: null,
    not_rejected_95: null,
    not_rejected_99: null,
    exceed_or_equal_count: 0,
    empirical_upper_tail_probability: null,
    unavailable_reason_code: "model_fit_exact.insufficient_usable_replicates",
  });
}

export function exactFitRunWithAggregateStatus(status: "partial" | "unavailable"): AnalysisRun {
  const run = currentExactFitRun();
  const exact = run.bootstrap!.model_fit_exact_inference!;
  const criteria: PlsModelFitExactCriterion[] = status === "partial"
    ? ["d_g"]
    : ["srmr", "d_uls", "d_g"];
  for (const variant of [exact.saturated, exact.estimated]) {
    for (const criterion of criteria) makeExactCriterionUnavailable(variant, criterion);
    variant.status = status;
  }
  exact.status = status;
  return run;
}

export function nativePlsModelFitPresentationFixture(
  state: NativePlsModelFitPresentationFixture,
): AnalysisRun {
  if (state === "not_run") return currentModelFitRun();
  if (state === "available") return currentExactFitRun();
  if (state === "partial" || state === "unavailable") {
    return exactFitRunWithAggregateStatus(state);
  }
  const failed = currentExactFitRun();
  delete failed.bootstrap!.model_fit_exact_inference;
  return failed;
}
