import { describe, expect, it } from "vitest";
import {
  GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
  GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
  GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
  GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
  GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
  GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1,
  GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
  GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
  parseCanonicalGeneralSemResultsV1,
  type CanonicalGeneralSemResultsV1Context,
} from "./canonicalGeneralSemResultsV1";
import { sha256HexUtf8V1 } from "./sha256V1";

const context: CanonicalGeneralSemResultsV1Context = {
  modelId: "model:test",
  modelDigest: "a".repeat(64),
  datasetFingerprint: "b".repeat(64),
  recipeDigest: "c".repeat(64),
  seed: null,
  workers: 1,
  capabilityCells: [GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1],
};

function pointFixture(): unknown {
  const trace = {
    model_id: context.modelId,
    capability_cell: GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  };
  return {
    schema_version: 1,
    three_way_interaction_effects: [{
      effect_id: "three_way_delta:interaction:x_w_z",
      trace,
      interaction_id: "interaction:x_w_z",
      focal_relation_id: "relation:x_y",
      interaction_effect_relation_id: "relation:x_w_z_y",
      interaction_effect_parameter_id: "parameter:x_w_z_y",
      operand_ids: ["construct:x", "construct:w", "construct:z"],
      outcome_id: "construct:y",
      generated_product_column_id: "generated:x_w_z",
      stage_one_model_scientific_sha256: "d".repeat(64),
      method_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
      product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
      hierarchy_policy: "strong",
      hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
      observation_count: 100,
      unstandardized_product_mean: 0,
      unstandardized_product_sample_standard_deviation: 2,
      standardized_product_coefficient: { estimate: 0.4 },
      scientific_rescaled_delta: { estimate: 0.2 },
    }],
    three_way_conditional_interaction_effects: [-1, 0, 1].map((probe, index) => ({
      effect_id: `conditional_interaction:${index}`,
      trace,
      interaction_id: "interaction:x_w_z",
      focal_relation_id: "relation:x_y",
      first_moderator_id: "construct:w",
      second_moderator_id: "construct:z",
      second_moderator_probe_kind: "continuous_standardized",
      second_moderator_probe_index: index,
      second_moderator_value: probe,
      value: { estimate: 0.25 + probe * 0.1 },
    })),
    three_way_simple_slopes: [-1, 0, 1].flatMap((secondProbe, secondIndex) => (
      [-1, 0, 1].map((firstProbe, firstIndex) => ({
        effect_id: `simple_slope:${secondIndex}:${firstIndex}`,
        trace,
        interaction_id: "interaction:x_w_z",
        focal_relation_id: "relation:x_y",
        first_moderator_id: "construct:w",
        second_moderator_id: "construct:z",
        first_moderator_probe_kind: "continuous_standardized",
        second_moderator_probe_kind: "continuous_standardized",
        first_probe_index: firstIndex,
        first_moderator_value: firstProbe,
        second_probe_index: secondIndex,
        second_moderator_value: secondProbe,
        value: { estimate: 0.3 + firstProbe * 0.1 + secondProbe * 0.05 },
      }))
    )),
  };
}

function bootstrapFixture(): {
  value: Record<string, unknown>;
  bootstrapContext: CanonicalGeneralSemResultsV1Context;
} {
  const value = pointFixture() as {
    three_way_interaction_effects: Array<Record<string, unknown>>;
    three_way_conditional_interaction_effects: Array<Record<string, unknown>>;
    three_way_simple_slopes: Array<Record<string, unknown>>;
    three_way_moderation_bootstrap_receipt?: Record<string, unknown>;
  };
  const inferred = (estimate: number) => ({
    estimate,
    bootstrap_mean: estimate + 0.01,
    bootstrap_bias: 0.01,
    standard_error: 0.05,
    lower: estimate - 0.1,
    upper: estimate + 0.1,
    p_value: 2 / 11,
    bootstrap_usable_replicates: 10,
    bootstrap_two_sided_exceedances: 1,
  });
  const delta = value.three_way_interaction_effects[0].scientific_rescaled_delta as { estimate: number };
  value.three_way_interaction_effects[0].scientific_rescaled_delta = inferred(delta.estimate);
  for (const row of [
    ...value.three_way_conditional_interaction_effects,
    ...value.three_way_simple_slopes,
  ]) {
    const estimate = (row.value as { estimate: number }).estimate;
    row.value = inferred(estimate);
  }
  const targetIds = [
    ...value.three_way_interaction_effects.map((row) => row.effect_id as string),
    ...value.three_way_conditional_interaction_effects.map((row) => row.effect_id as string),
    ...value.three_way_simple_slopes.map((row) => row.effect_id as string),
  ].sort();
  const baseCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  };
  const capabilityDependencies = [
    baseCell,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  ].sort((left, right) => (
    `${left.registry_schema_version}:${left.capability_id}:${left.cell_id}:${left.capability_version}`
      .localeCompare(`${right.registry_schema_version}:${right.capability_id}:${right.cell_id}:${right.capability_version}`)
  ));
  value.three_way_moderation_bootstrap_receipt = {
    capability_cell: GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
    capability_dependencies: capabilityDependencies,
    method_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    point_method_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
    resampling_operation_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    resampling_stream_version: GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
    quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
    standard_error_method_version: GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
    summation_method_version: GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
    p_value_method_version: GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
    failure_policy_version: GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
    sign_alignment_method_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
    product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
    probe_policy_version: GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1,
    compiled_plan_sha256: "e".repeat(64),
    general_sem_config_sha256: "f".repeat(64),
    model_scientific_sha256: context.modelDigest,
    stage_one_model_scientific_sha256: "d".repeat(64),
    source_dataset_fingerprint: context.datasetFingerprint,
    complete_case_frame_sha256: "1".repeat(64),
    usable_replicate_indices_sha256: sha256HexUtf8V1(JSON.stringify(Array.from({ length: 10 }, (_, index) => index))),
    target_identity_set_sha256: sha256HexUtf8V1(JSON.stringify(targetIds)),
    target_ids: targetIds,
    interval: "percentile_type7",
    tail: "two_sided",
    confidence_level: 0.95,
    resamples_requested: 10,
    resamples_usable: 10,
    minimum_usable_resamples: 9,
    seed: "17",
    workers: 2,
    complete_model_reestimated_per_replicate: true,
    shared_stage_one_reestimated_per_replicate: true,
    score_vectors_sign_aligned_before_products: true,
    all_lower_order_and_three_way_products_recomputed_per_replicate: true,
    joint_stage_two_reestimated_per_replicate: true,
    complete_joint_point_contract_validated_per_replicate: true,
    all_three_way_targets_share_one_replicate_ledger: true,
    failed_replicates: [],
  };
  return {
    value,
    bootstrapContext: {
      ...context,
      seed: 17,
      workers: 2,
      capabilityCells: [
        baseCell,
        GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
        GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
      ],
    },
  };
}

describe("canonical General SEM three-way V1", () => {
  it("strictly and losslessly accepts one complete point-estimate probe grid", () => {
    const fixture = pointFixture();

    expect(parseCanonicalGeneralSemResultsV1(structuredClone(fixture), context)).toEqual(fixture);
  });

  it("rejects presentation-only anchor metadata from the scientific payload", () => {
    const fixture = pointFixture() as {
      three_way_interaction_effects: Array<Record<string, unknown>>;
    };
    fixture.three_way_interaction_effects[0].anchor_fraction = 0.5;

    expect(() => parseCanonicalGeneralSemResultsV1(fixture, context)).toThrowError(expect.objectContaining({
      code: "schema.unknown_field",
      path: "general_sem_results.three_way_interaction_effects[0].anchor_fraction",
    }));
  });

  it("accepts the additive shared-ledger three-way bootstrap receipt", () => {
    const { value, bootstrapContext } = bootstrapFixture();

    expect(parseCanonicalGeneralSemResultsV1(structuredClone(value), bootstrapContext)).toEqual(value);
  });
});
