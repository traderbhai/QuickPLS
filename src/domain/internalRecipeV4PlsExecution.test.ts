import { describe, expect, it } from "vitest";
import {
  INTERNAL_RECIPE_V4_PLS_NONLINEAR_CAPABILITY_CELL,
  PLS_NONLINEAR_ENGINE_WARNING_V1,
  parseInternalRecipeV4PlsExecutionResultV1,
  parsePlsResolvedScoreExecutionV2,
  type AnalysisRecipeV4,
  type AnalysisRecipeV4MissingDataPolicy,
  type InternalLabsRecipeV4PlsExecutionRequestV1,
} from "./internalRecipeV4PlsExecution";

function scoreExecutionFixture() {
  return {
    contract_version: "pls_score_execution_v2",
    blocks: [
      {
        construct_id: "x",
        indicator_ids: ["x_one", "x_two"],
        scoring: {
          kind: "estimated",
          mode: "mode_a",
          requested_initialization: {
            kind: "standard",
            weights: [
              { indicator_id: "x_one", value: 1 },
              { indicator_id: "x_two", value: 1 },
            ],
          },
          resolved_initial_weights: [
            { indicator_id: "x_one", value: 0.5 },
            { indicator_id: "x_two", value: 0.5 },
          ],
        },
      },
      {
        construct_id: "y",
        indicator_ids: ["y_one", "y_two"],
        scoring: {
          kind: "fixed_custom",
          normalization: "unit_variance",
          requested_weights: [
            { indicator_id: "y_one", value: 2 },
            { indicator_id: "y_two", value: -1 },
          ],
          resolved_effective_weights: [
            { indicator_id: "y_one", value: 0.8 },
            { indicator_id: "y_two", value: -0.4 },
          ],
        },
      },
    ],
    iteration_accounting: {
      maximum_iterations: 3_000,
      stop_criterion: 1e-7,
      estimated_block_count: 1,
      fixed_block_count: 1,
      performed_iterations: 5,
      estimated_block_updates: 5,
    },
  };
}

function resultFixture() {
  return {
    schema_version: 1,
    provenance: {
      adapter_version: "compiled_recipe_v4_pls_plan_v2_execution_v6",
      compilation_receipt: {},
      projected_recipe_schema_version: 3,
      projected_recipe_sha256: "a".repeat(64),
      projected_initialization_sha256: "b".repeat(64),
      dataset_id: "dataset-1",
      estimator_method_version: "pls_score_execution_v2",
    },
    estimation: {
      method_version: "pls_score_execution_v2",
      iterations: 5,
      score_execution: scoreExecutionFixture(),
      fixed_score_scale_receipt: {
        contract_version: "pls_fixed_score_scale_receipt_v1",
        blocks: [{
          construct_id: "y",
          indicator_ids: ["y_one", "y_two"],
          pre_standardization_center: 0.125,
          pre_standardization_scale: 0.4,
          effective_unit_score_weights: [
            { indicator_id: "y_one", value: 2 },
            { indicator_id: "y_two", value: -1 },
          ],
        }],
      },
      point_estimate_attribution: {
        contract_version: "pls_point_estimate_attribution_v1",
        preprocessing: "mean_centered",
        indicator_centering: "sample_mean",
        indicator_scaling: "unit_scale",
        outer_weights: "preprocessed_indicator_to_unit_variance_construct_score",
        outer_loadings: "indicator_construct_score_correlation",
        construct_scores: "zero_mean_unit_variance_construct_score",
        structural_paths: "standardized_construct_score_regression",
        effects: "standardized_structural_path_decomposition",
      },
      algorithm_convergence_receipt: {
        contract_version: "pls_algorithm_convergence_receipt_v1",
        weighting_scheme: "path",
        maximum_iterations: 3_000,
        stop_criterion: 1e-7,
        comparison: "less_than_or_equal",
        performed_iterations: 5,
        estimated_block_updates: 5,
        termination_reason: "converged_tolerance",
        final_max_outer_weight_change: 1e-8,
        blocks: [
          {
            construct_id: "x",
            indicator_order: ["x_one", "x_two"],
            update_rule: "mode_a_covariance",
            initialization: "standard_unit_weights",
          },
          {
            construct_id: "y",
            indicator_order: ["y_one", "y_two"],
            update_rule: "fixed_no_update",
            initialization: "fixed_custom_weights",
          },
        ],
      },
    },
  };
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

function nonlinearResultFixture() {
  const result = resultFixture();
  result.provenance.adapter_version = "compiled_recipe_v4_pls_plan_v2_execution_v7";
  result.provenance.estimator_method_version = "pls_quadratic_nonlinear_effects_v1";
  result.provenance.compilation_receipt = {
    schema_version: 1,
    recipe_id: "recipe-v7",
    recipe_document_sha256: "1".repeat(64),
    recipe_analytical_sha256: "2".repeat(64),
    model_id: "model-v7",
    model_document_sha256: "3".repeat(64),
    model_scientific_sha256: "4".repeat(64),
    dataset_fingerprint: "v2:" + "5".repeat(64),
    compiler_target: "pls_plan_v2",
    compiler_version: "recipe_v4_compiler_v1",
    capability_cell: clone(INTERNAL_RECIPE_V4_PLS_NONLINEAR_CAPABILITY_CELL),
    plan_sha256: "6".repeat(64),
    analytical_identity_sha256: "7".repeat(64),
  };
  delete (result.provenance as { projected_initialization_sha256?: unknown })
    .projected_initialization_sha256;
  result.estimation.method_version = "pls_quadratic_nonlinear_effects_v1";
  delete (result.estimation as { score_execution?: unknown }).score_execution;
  delete (result.estimation as { fixed_score_scale_receipt?: unknown }).fixed_score_scale_receipt;
  delete (result.estimation as { algorithm_convergence_receipt?: unknown })
    .algorithm_convergence_receipt;
  Object.assign(result.estimation, {
    paths: [{ source: "x", target: "y", coefficient: 0.25 }],
    nonlinear_effects: {
      method_version: "pls_quadratic_nonlinear_effects_v1",
      term: "centered_squared_construct_score_v1",
      estimates: [{
        source: "x",
        target: "y",
        linear_coefficient: 0.25,
        quadratic_coefficient: 0.1,
        standard_error: 0.05,
        t_statistic: 2,
        p_value_two_sided: 0.0455,
        linear_r_squared: 0.4,
        augmented_r_squared: 0.45,
        delta_r_squared: 0.04999999999999999,
        warning: null,
      }],
      warnings: [PLS_NONLINEAR_ENGINE_WARNING_V1],
    },
  });
  return result;
}

function nonlinearPayload(value: unknown) {
  return (value as {
    estimation: {
      nonlinear_effects: {
        term: string;
        estimates: Array<{ t_statistic: number }>;
      };
    };
  }).estimation.nonlinear_effects;
}

describe("Recipe-v4 PLS resolved score-execution reader", () => {
  it("widens Recipe-v4 only while the Internal PLS request remains listwise-only", () => {
    const cbsemLabsPolicy: AnalysisRecipeV4<AnalysisRecipeV4MissingDataPolicy>["settings"]["missing_data"] = "mean_replacement";
    const plsPolicy: InternalLabsRecipeV4PlsExecutionRequestV1["recipe"]["settings"]["missing_data"] = "listwise_deletion";
    // @ts-expect-error PLS must not acquire the Internal CB-SEM Labs option.
    const rejectedPlsPolicy: InternalLabsRecipeV4PlsExecutionRequestV1["recipe"]["settings"]["missing_data"] = "mean_replacement";
    expect([cbsemLabsPolicy, plsPolicy, rejectedPlsPolicy]).toEqual(["mean_replacement", "listwise_deletion", "mean_replacement"]);
  });

  it("accepts exact mixed estimated/fixed semantics and accounting", () => {
    expect(parsePlsResolvedScoreExecutionV2(scoreExecutionFixture()))
      .toEqual(scoreExecutionFixture());
    expect(parseInternalRecipeV4PlsExecutionResultV1(resultFixture()))
      .toEqual(resultFixture());
  });

  it("accepts all fixed-score normalizations and rejects exact-resolution tampering", () => {
    const none = scoreExecutionFixture();
    none.blocks[1].scoring.normalization = "none";
    none.blocks[1].scoring.requested_weights![0].value = -0;
    none.blocks[1].scoring.requested_weights![1].value = 0.75;
    none.blocks[1].scoring.resolved_effective_weights![0].value = -0;
    none.blocks[1].scoring.resolved_effective_weights![1].value = 0.75;
    expect(parsePlsResolvedScoreExecutionV2(none)).toEqual(none);

    const sumToOne = scoreExecutionFixture();
    sumToOne.blocks[1].scoring.normalization = "sum_to_one";
    sumToOne.blocks[1].scoring.requested_weights![0].value = -0.25;
    sumToOne.blocks[1].scoring.requested_weights![1].value = 0.75;
    sumToOne.blocks[1].scoring.resolved_effective_weights![0].value = -0.5;
    sumToOne.blocks[1].scoring.resolved_effective_weights![1].value = 1.5;
    expect(parsePlsResolvedScoreExecutionV2(sumToOne)).toEqual(sumToOne);
    expect(parsePlsResolvedScoreExecutionV2(scoreExecutionFixture()))
      .toEqual(scoreExecutionFixture());

    const signedZeroTamper = scoreExecutionFixture();
    signedZeroTamper.blocks[1].scoring.normalization = "none";
    signedZeroTamper.blocks[1].scoring.requested_weights![0].value = -0;
    signedZeroTamper.blocks[1].scoring.resolved_effective_weights![0].value = 0;
    signedZeroTamper.blocks[1].scoring.requested_weights![1].value = 1;
    signedZeroTamper.blocks[1].scoring.resolved_effective_weights![1].value = 1;
    expect(() => parsePlsResolvedScoreExecutionV2(signedZeroTamper))
      .toThrow(/none normalization contract/);

    const sumTamper = clone(sumToOne);
    sumTamper.blocks[1].scoring.resolved_effective_weights![1].value = 1.500_000_000_000_000_2;
    expect(() => parsePlsResolvedScoreExecutionV2(sumTamper))
      .toThrow(/sum_to_one normalization contract/);

    const zeroSum = scoreExecutionFixture();
    zeroSum.blocks[1].scoring.normalization = "sum_to_one";
    zeroSum.blocks[1].scoring.requested_weights![0].value = -1;
    zeroSum.blocks[1].scoring.requested_weights![1].value = 1;
    expect(() => parsePlsResolvedScoreExecutionV2(zeroSum))
      .toThrow(/sum_to_one normalization contract/);
  });

  it("rejects drifted defaults, normalization, stable order, and accounting", () => {
    const stop = scoreExecutionFixture();
    stop.iteration_accounting.stop_criterion = 1e-6;
    expect(() => parsePlsResolvedScoreExecutionV2(stop)).toThrow(/exact v2 contract/);

    const normalization = scoreExecutionFixture();
    normalization.blocks[1].scoring.normalization = "sum_to_one";
    expect(() => parsePlsResolvedScoreExecutionV2(normalization))
      .toThrow(/sum_to_one normalization contract/);

    const order = scoreExecutionFixture();
    order.blocks[0].scoring.resolved_initial_weights!.reverse();
    expect(() => parsePlsResolvedScoreExecutionV2(order)).toThrow(/block indicator order/);

    const updates = scoreExecutionFixture();
    updates.iteration_accounting.estimated_block_updates = 10;
    expect(() => parsePlsResolvedScoreExecutionV2(updates)).toThrow(/exact v2 contract/);
  });

  it("rejects orientation changes and incomplete method identity", () => {
    const orientation = scoreExecutionFixture();
    orientation.blocks[1].scoring.resolved_effective_weights![1].value = 0.4;
    expect(() => parsePlsResolvedScoreExecutionV2(orientation)).toThrow(/normalization contract/);

    const omitted = resultFixture();
    delete (omitted.estimation as { score_execution?: unknown }).score_execution;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(omitted))
      .toThrow(/omitted|must be an object/);

    const legacy = resultFixture();
    legacy.estimation.method_version = "pls_pm_v1";
    legacy.provenance.estimator_method_version = "pls_pm_v1";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(legacy))
      .toThrow(/must be omitted for legacy/);
  });

  it("keeps a score-contract-free legacy result backward-readable", () => {
    const legacy = resultFixture();
    legacy.estimation.method_version = "pls_pm_v1";
    legacy.estimation.iterations = 4;
    delete (legacy.estimation as { score_execution?: unknown }).score_execution;
    delete (legacy.estimation as { fixed_score_scale_receipt?: unknown })
      .fixed_score_scale_receipt;
    legacy.provenance.estimator_method_version = "pls_pm_v1";
    legacy.provenance.adapter_version = "compiled_recipe_v4_pls_plan_v2_execution_v3";
    delete (legacy.provenance as { projected_initialization_sha256?: string })
      .projected_initialization_sha256;
    delete (legacy.estimation as { point_estimate_attribution?: unknown })
      .point_estimate_attribution;
    delete (legacy.estimation as { algorithm_convergence_receipt?: unknown })
      .algorithm_convergence_receipt;

    expect(parseInternalRecipeV4PlsExecutionResultV1(legacy)).toEqual(legacy);
  });

  it("requires current typed families and rejects unknown adapter generations", () => {
    const missing = resultFixture();
    delete (missing.estimation as { algorithm_convergence_receipt?: unknown })
      .algorithm_convergence_receipt;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(missing))
      .toThrow(/algorithm_convergence_receipt/);

    const unknown = resultFixture();
    unknown.provenance.adapter_version = "compiled_recipe_v4_pls_plan_v2_execution_custom";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(unknown))
      .toThrow(/not an allowlisted PLS adapter generation/);

    const legacyScore = resultFixture();
    legacyScore.provenance.adapter_version = "compiled_recipe_v4_pls_plan_v2_execution_v4";
    delete (legacyScore.estimation as { point_estimate_attribution?: unknown })
      .point_estimate_attribution;
    delete (legacyScore.estimation as { algorithm_convergence_receipt?: unknown })
      .algorithm_convergence_receipt;
    delete (legacyScore.estimation as { fixed_score_scale_receipt?: unknown })
      .fixed_score_scale_receipt;
    expect(parseInternalRecipeV4PlsExecutionResultV1(legacyScore)).toEqual(legacyScore);

    const attributionTamper = resultFixture();
    attributionTamper.estimation.point_estimate_attribution.indicator_scaling = "sample_standard_deviation";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(attributionTamper))
      .toThrow(/preprocessing and point-estimate scale contract/);

    const convergenceTamper = resultFixture();
    convergenceTamper.estimation.algorithm_convergence_receipt.blocks[1].initialization = "fixed_unit_weights";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(convergenceTamper))
      .toThrow(/score-execution order or semantics/);

    const partialLegacy = resultFixture();
    partialLegacy.provenance.adapter_version = "compiled_recipe_v4_pls_plan_v2_execution_v4";
    delete (partialLegacy.estimation.point_estimate_attribution as unknown as Record<string, unknown>)
      .effects;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(partialLegacy))
      .toThrow(/drifted key contract/);
  });

  it("requires and binds the current fixed-score scale receipt", () => {
    const missing = resultFixture();
    delete (missing.estimation as { fixed_score_scale_receipt?: unknown })
      .fixed_score_scale_receipt;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(missing))
      .toThrow(/required for a current fixed-score adapter/);

    const valueTamper = resultFixture();
    valueTamper.estimation.fixed_score_scale_receipt.blocks[0]
      .effective_unit_score_weights[0].value = 2.000_000_000_000_000_4;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(valueTamper))
      .toThrow(/resolved coefficient \/ scale/);

    const orderTamper = resultFixture();
    orderTamper.estimation.fixed_score_scale_receipt.blocks[0].indicator_ids.reverse();
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(orderTamper))
      .toThrow(/fixed indicator order/);

    const duplicate = resultFixture();
    duplicate.estimation.fixed_score_scale_receipt.blocks[0].indicator_ids[1] = "y_one";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(duplicate))
      .toThrow(/fixed indicator order/);

    const signedZero = resultFixture();
    const scoring = signedZero.estimation.score_execution.blocks[1].scoring;
    scoring.normalization = "none";
    scoring.requested_weights![0].value = -0;
    scoring.resolved_effective_weights![0].value = -0;
    scoring.requested_weights![1].value = 0.75;
    scoring.resolved_effective_weights![1].value = 0.75;
    const scale = signedZero.estimation.fixed_score_scale_receipt.blocks[0];
    scale.pre_standardization_scale = 1;
    scale.effective_unit_score_weights[0].value = 0;
    scale.effective_unit_score_weights[1].value = 0.75;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(signedZero))
      .toThrow(/resolved coefficient \/ scale/);
  });

  it("strictly accepts only the v7 nonlinear wire and rejects identity, arithmetic, and score mixing", () => {
    const exact = nonlinearResultFixture();
    expect(parseInternalRecipeV4PlsExecutionResultV1(exact)).toEqual(exact);

    const adapter = clone(exact);
    adapter.provenance.adapter_version = "compiled_recipe_v4_pls_plan_v2_execution_v6";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(adapter)).toThrow(/allowlisted PLS adapter/);

    const term = clone(exact);
    nonlinearPayload(term).term = "uncentered_square";
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(term)).toThrow(/drifted method, term, warning/);

    const primary = clone(exact);
    (primary.provenance.compilation_receipt as { capability_cell: unknown }).capability_cell = {
      registry_schema_version: 2,
      capability_id: "smartpls.pls_algorithm",
      cell_id: "qpls3.pls.algorithm",
      capability_version: "pls_pm_v1",
    };
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(primary)).toThrow(/primary nonlinear capability cell/);

    const receiptInjection = clone(exact);
    (receiptInjection.provenance.compilation_receipt as Record<string, unknown>).unexpected = true;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(receiptInjection))
      .toThrow(/drifted key contract/);

    const fingerprint = clone(exact);
    (fingerprint.provenance.compilation_receipt as { dataset_fingerprint: string })
      .dataset_fingerprint = `v3:${"5".repeat(64)}`;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(fingerprint))
      .toThrow(/v2-prefixed lowercase SHA-256/);

    const arithmetic = clone(exact);
    nonlinearPayload(arithmetic).estimates[0].t_statistic = 2.0000000000000004;
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(arithmetic)).toThrow(/numerical or structural invariants/);

    const scoreMix = clone(exact);
    scoreMix.estimation.score_execution = scoreExecutionFixture();
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(scoreMix)).toThrow(/must be omitted/);

    const convergenceMix = clone(exact);
    convergenceMix.estimation.algorithm_convergence_receipt = clone(
      resultFixture().estimation.algorithm_convergence_receipt,
    );
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(convergenceMix))
      .toThrow(/must be omitted for the v7 nonlinear/);

    const projectedInitialization = clone(exact);
    (projectedInitialization.provenance as unknown as Record<string, unknown>)
      .projected_initialization_sha256 = "8".repeat(64);
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(projectedInitialization))
      .toThrow(/projected_initialization_sha256.*must be omitted/);

    const posthocPayload = clone(exact);
    (posthocPayload.estimation as unknown as Record<string, unknown>).posthoc_minimum_sample_size = {
      method_version: "injected",
    };
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(posthocPayload))
      .toThrow(/posthoc_minimum_sample_size.*must be omitted/);

    const legacyInjection = resultFixture();
    (legacyInjection.estimation as unknown as Record<string, unknown>).nonlinear_effects = clone(
      nonlinearPayload(exact),
    );
    expect(() => parseInternalRecipeV4PlsExecutionResultV1(legacyInjection)).toThrow(/outside the v7 nonlinear adapter/);
  });
});
