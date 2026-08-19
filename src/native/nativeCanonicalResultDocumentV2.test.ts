import { describe, expect, it } from "vitest";
import {
  canonicalAnalyticalResultJson,
  validateCanonicalResultDocumentV2,
} from "../domain/canonicalResultDocumentV2";
import {
  ESTABLISHED_METHOD_CONTRACTS_V1,
  establishedCanonicalTableOwnerOptionsV1,
} from "../domain/generated/establishedMethodContractsV1";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import { completedGscaRun } from "./nativeGsca.testFixture";
import {
  convertNativeCovarianceToPresentationV4,
  newNativeScientificCovarianceEdgeV4,
  withNativeConstructEstimandV4,
} from "../domain/semModelV4Authoring";
import {
  type CanonicalGeneralSemEstimateV1,
  type CanonicalGeneralSemResultsV1,
  type NativeCanonicalResultDocumentV2,
  GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
  GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
  GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
  GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
  GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1,
  GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
  GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
  GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
  canonicalResultDocumentFromAnalysisRunV2,
  nativeCapabilityRequirementsForTableV2,
  parseNativeCanonicalResultDocumentV2,
} from "./nativeCanonicalResultDocumentV2";
import { buildNativeResultNavigation } from "./nativeResults";

const SPECIFIC_PATH_EFFECT_ID = "sem_specific_path_v1_be495c8d1bd8639e5065b03ffe6cf107b3a753d6509fde70bd03e7c0cd94e6b1";
const GENERAL_SEM_MULTIPLE_MEDIATION_BOOTSTRAP_CELL = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
  capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
} as const;

function currentPlsRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    modelId: "corporate-reputation-model",
    modelSnapshot: {
      nodes: [
        {
          id: "competence",
          type: "construct",
          position: { x: 50, y: 100 },
          data: { label: "Competence", shortName: "COMP", mode: "reflective", indicators: ["COMP1", "COMP2", "COMP3"] },
        },
        {
          id: "satisfaction",
          type: "construct",
          position: { x: 420, y: 100 },
          data: { label: "Satisfaction", shortName: "CUSA", mode: "reflective", indicators: ["CUSA1", "CUSA2"] },
        },
      ],
      edges: [{ id: "competence-satisfaction", source: "competence", target: "satisfaction" }],
    },
    provenance: {
      recipe_id: "recipe-pls-runtime-v2",
      dataset_fingerprint: `sha256:${"a".repeat(64)}`,
      method: "pls_pm",
      method_version: base.result!.method_version,
      engine_version: "qpls-estimation-test",
      seed: base.seed,
      settings: {
        method: "pls_pm",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: base.seed,
        workers: 4,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-07-19T11:59:59.000Z",
      completed_at: "2026-07-19T12:00:00.000Z",
    },
  };
}

function completeGeneralSemResultsFixture(
  document: NativeCanonicalResultDocumentV2,
): CanonicalGeneralSemResultsV1 {
  const capabilityCell = document.capability_cells?.find((cell) => cell.capability_id === "smartpls.mediation");
  if (!capabilityCell) throw new Error("Mediation capability fixture is missing");
  const trace = () => ({ model_id: document.provenance.model_id, capability_cell: { ...capabilityCell } });
  const effectValue = (estimate: number): CanonicalGeneralSemEstimateV1 => ({
    estimate,
  });
  return {
    schema_version: 1,
    specific_indirect_effects: [{
      effect_id: SPECIFIC_PATH_EFFECT_ID,
      estimand_id: "estimand_specific_1",
      trace: trace(),
      source_id: "construct_x",
      target_id: "construct_y",
      ordered_relation_ids: ["relation_a", "relation_b"],
      value: effectValue(0.12),
    }],
    aggregate_effects: [
      {
        effect_id: "estimand_total_effect_1",
        estimand_id: "estimand_total_effect_1",
        trace: trace(),
        kind: "total_effect",
        source_id: "construct_x",
        target_id: "construct_y",
        direct_relation_ids: ["relation_direct"],
        contributing_path_identities: [SPECIFIC_PATH_EFFECT_ID],
        value: effectValue(0.6),
      },
      {
        effect_id: "estimand_total_indirect_1",
        estimand_id: "estimand_total_indirect_1",
        trace: trace(),
        kind: "total_indirect",
        source_id: "construct_x",
        target_id: "construct_y",
        direct_relation_ids: [],
        contributing_path_identities: [SPECIFIC_PATH_EFFECT_ID],
        value: effectValue(0.18),
      },
    ],
    conditional_effect_probes: [
      {
        probe_id: "probe_data",
        trace: trace(),
        moderator_id: "moderator_m",
        values: { kind: "data_derived_mean_plus_minus_one_sd", mean: 1, standard_deviation: 1 },
      },
      {
        probe_id: "probe_explicit",
        trace: trace(),
        moderator_id: "moderator_m",
        values: { kind: "explicit", values: [-1, 0, 1] },
      },
    ],
    conditional_effects: [{
      effect_id: "effect_conditional_1",
      estimand_id: "estimand_conditional_1",
      trace: trace(),
      interaction_id: "interaction_1",
      focal_relation_id: "relation_focal_1",
      probe_id: "probe_data",
      moderator_id: "moderator_m",
      probe_value_index: 1,
      moderator_value: 1,
      value: effectValue(0.42),
    }],
    interaction_plots: [{
      plot_id: "interaction_plot_1",
      trace: trace(),
      interaction_id: "interaction_1",
      focal_relation_id: "relation_focal_1",
      focal_predictor_id: "construct_x",
      moderator_id: "moderator_m",
      outcome_id: "construct_y",
      series: [
        {
          series_id: "series_01_low",
          probe_id: "probe_data",
          probe_value_index: 0,
          moderator_value: 0,
          points: [
            { focal_value: -1, predicted_value: -0.2, lower: -0.3, upper: -0.1 },
            { focal_value: 1, predicted_value: 0.2, lower: 0.1, upper: 0.3 },
          ],
        },
        {
          series_id: "series_02_high",
          probe_id: "probe_data",
          probe_value_index: 2,
          moderator_value: 2,
          points: [
            { focal_value: -1, predicted_value: -0.5, lower: -0.6, upper: -0.4 },
            { focal_value: 1, predicted_value: 0.5, lower: 0.4, upper: 0.6 },
          ],
        },
      ],
    }],
    higher_order_stages: [
      {
        stage_id: "hoc_stage_1",
        trace: trace(),
        higher_order_construct_id: "hoc_ab",
        stage_number: 1,
        kind: "lower_order_score_estimation",
        input_construct_ids: ["construct_a", "construct_b"],
        output_variable_ids: ["score_a", "score_b"],
      },
      {
        stage_id: "hoc_stage_2",
        trace: trace(),
        higher_order_construct_id: "hoc_ab",
        stage_number: 2,
        kind: "higher_order_estimation",
        input_construct_ids: ["score_a", "score_b"],
        output_variable_ids: ["hoc_ab"],
        relation_estimates: [{
          relation_id: "relation_hoc_1",
          source_id: "hoc_ab",
          target_id: "construct_y",
          value: effectValue(0.31),
        }],
      },
    ],
    cbsem_fit: [{
      fit_id: "cbsem_fit_1",
      trace: trace(),
      chi_square: 12.5,
      degrees_of_freedom: 8,
      chi_square_p_value: 0.13,
      rmsea: 0.04,
      rmsea_interval: { confidence_level: 0.9, lower: 0.01, upper: 0.08 },
      cfi: 0.98,
      tli: 0.97,
      srmr: 0.03,
      aic: 101.2,
      bic: 120.4,
    }],
    identification_diagnostics: [{
      diagnostic_id: "identification_model_1",
      trace: trace(),
      scope: "model",
      subject_id: document.provenance.model_id,
      status: "identified",
      code: "identified",
      message: "The compiled model passed identification checks.",
      degrees_of_freedom: 8,
    }],
  };
}

function completeGeneralSemInteractionResultsFixture(
  document: NativeCanonicalResultDocumentV2,
): CanonicalGeneralSemResultsV1 {
  const moderationCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.moderation",
    cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
  };
  document.capability_cells ??= [document.provenance.capability_cell];
  document.capability_cells.push(moderationCell);
  document.capability_cells.sort((left, right) => {
    const leftIdentity = `${left.registry_schema_version}:${left.capability_id}:${left.cell_id}:${left.capability_version}`;
    const rightIdentity = `${right.registry_schema_version}:${right.capability_id}:${right.cell_id}:${right.capability_version}`;
    return leftIdentity < rightIdentity ? -1 : leftIdentity > rightIdentity ? 1 : 0;
  });
  const trace = {
    model_id: document.provenance.model_id,
    capability_cell: { ...moderationCell },
  };
  const interactionEffectId = "relation_interaction_x_by_w_effect";
  const probeId = "probe_interaction_x_by_w_standardized";
  const stageOneDigest = document.provenance.model_digest === "e".repeat(64)
    ? "f".repeat(64)
    : "e".repeat(64);
  return {
    schema_version: 1,
    joint_stage_structural_coefficients: [{
      relation_id: "relation_x_y",
      parameter_id: "parameter_x_y",
      trace: structuredClone(trace),
      source_id: "construct_x",
      target_id: "construct_y",
      role: "structural",
      estimate: { estimate: 0.3 },
      stage: "joint_stage_two",
      method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
    }],
    interaction_effects: [{
      effect_id: interactionEffectId,
      trace: structuredClone(trace),
      interaction_id: "interaction_x_by_w",
      focal_relation_id: "relation_x_y",
      interaction_effect_relation_id: interactionEffectId,
      interaction_effect_parameter_id: "parameter_interaction_x_by_w_effect",
      focal_predictor_id: "construct_x",
      moderator_id: "construct_w",
      outcome_id: "construct_y",
      generated_product_column_id: "generated_interaction_x_by_w_product",
      stage_one_model_scientific_sha256: stageOneDigest,
      method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
      construction_method: "two_stage",
      product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
      hierarchy_policy: "strong",
      hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
      conditioning_policy_version: GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1,
      observation_count: 61,
      unstandardized_product_mean: 0.125,
      unstandardized_product_sample_standard_deviation: 0.5,
      standardized_product_coefficient: { estimate: 0.2 },
      scientific_rescaled_gamma: { estimate: 0.4 },
    }],
    conditional_effect_probes: [{
      probe_id: probeId,
      trace: structuredClone(trace),
      moderator_id: "construct_w",
      values: { kind: "explicit", values: [-1, 0, 1] },
    }],
    conditional_effects: [-1, 0, 1].map((moderatorValue, probeValueIndex) => ({
      effect_id: `conditional_interaction_x_by_w_${probeValueIndex}`,
      estimand_id: "conditional_slope_interaction_x_by_w",
      trace: structuredClone(trace),
      interaction_id: "interaction_x_by_w",
      interaction_effect_id: interactionEffectId,
      focal_relation_id: "relation_x_y",
      probe_id: probeId,
      moderator_id: "construct_w",
      probe_value_index: probeValueIndex,
      moderator_value: moderatorValue,
      value: { estimate: 0.3 + 0.4 * moderatorValue },
    })),
    interaction_plots: [{
      plot_id: "plot_interaction_x_by_w",
      trace,
      interaction_id: "interaction_x_by_w",
      interaction_effect_id: interactionEffectId,
      focal_relation_id: "relation_x_y",
      focal_predictor_id: "construct_x",
      moderator_id: "construct_w",
      outcome_id: "construct_y",
      series: [-1, 0, 1].map((moderatorValue, probeValueIndex) => ({
        series_id: `series_interaction_x_by_w_${probeValueIndex}`,
        probe_id: probeId,
        probe_value_index: probeValueIndex,
        moderator_value: moderatorValue,
        points: [-1, 0, 1].map((focalValue) => ({
          focal_value: focalValue,
          predicted_value: focalValue * (0.3 + 0.4 * moderatorValue),
        })),
      })),
    }],
  };
}

async function serializedSha256Fixture(value: unknown): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(JSON.stringify(value)),
  );
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function typedEffectIdentitiesFixture(results: CanonicalGeneralSemResultsV1): unknown[] {
  const identities = [
    ...(results.specific_indirect_effects ?? []).map((effect) => ({
      kind: "specific_indirect",
      effect_id: effect.effect_id,
      estimand_id: effect.estimand_id,
      source_id: effect.source_id,
      target_id: effect.target_id,
      ordered_relation_ids: effect.ordered_relation_ids,
    })),
    ...(results.aggregate_effects ?? []).map((effect) => effect.kind === "total_indirect" ? {
      kind: "total_indirect",
      effect_id: effect.effect_id,
      estimand_id: effect.estimand_id,
      source_id: effect.source_id,
      target_id: effect.target_id,
      contributing_path_identities: effect.contributing_path_identities,
    } : {
      kind: "total_effect",
      effect_id: effect.effect_id,
      estimand_id: effect.estimand_id,
      source_id: effect.source_id,
      target_id: effect.target_id,
      direct_relation_ids: effect.direct_relation_ids,
      contributing_path_identities: effect.contributing_path_identities,
    }),
  ];
  return identities.sort((left, right) => (
    left.effect_id < right.effect_id ? -1 : left.effect_id > right.effect_id ? 1 : 0
  ));
}

async function completeGeneralSemInferenceResultsFixture(
  document: NativeCanonicalResultDocumentV2,
): Promise<CanonicalGeneralSemResultsV1> {
  const results = completeGeneralSemResultsFixture(document);
  const bootstrapCell = { ...GENERAL_SEM_MULTIPLE_MEDIATION_BOOTSTRAP_CELL };
  document.capability_cells = [
    ...(document.capability_cells ?? []),
    bootstrapCell,
  ].sort((left, right) => {
    const leftIdentity = `${left.registry_schema_version}:${left.capability_id}:${left.cell_id}:${left.capability_version}`;
    const rightIdentity = `${right.registry_schema_version}:${right.capability_id}:${right.cell_id}:${right.capability_version}`;
    return leftIdentity.localeCompare(rightIdentity);
  });
  const inferredValue = (estimate: number): CanonicalGeneralSemEstimateV1 => ({
    estimate,
    bootstrap_mean: estimate + 0.01,
    bootstrap_bias: 0.01,
    standard_error: 0.04,
    lower: estimate - 0.08,
    upper: estimate + 0.08,
    p_value: 0.2,
    bootstrap_usable_replicates: 9,
    bootstrap_two_sided_exceedances: 1,
  });
  for (const effect of results.specific_indirect_effects ?? []) effect.value = inferredValue(effect.value.estimate);
  for (const effect of results.aggregate_effects ?? []) effect.value = inferredValue(effect.value.estimate);
  const effectIds = [
    ...(results.specific_indirect_effects ?? []).map((effect) => effect.effect_id),
    ...(results.aggregate_effects ?? []).map((effect) => effect.effect_id),
  ].sort();
  const failedReplicates = [{
    replicate_index: 7,
    reason_code: "estimation_nonconvergence" as const,
    message: "The complete PLS model did not converge for this draw.",
  }];
  const usableReplicateIndices = Array.from({ length: 10 }, (_, index) => index)
    .filter((index) => index !== 7);
  results.inference_receipt = {
    kind: "case_bootstrap",
    capability_cell: { ...bootstrapCell },
    method_version: GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    resampling_operation_version: GENERAL_SEM_PLS_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    resampling_stream_version: GENERAL_SEM_INDEXED_CASE_RESAMPLING_STREAM_VERSION_V1,
    quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
    standard_error_method_version: GENERAL_SEM_SAMPLE_STANDARD_ERROR_METHOD_VERSION_V1,
    summation_method_version: GENERAL_SEM_NEUMAIER_SUMMATION_METHOD_VERSION_V1,
    p_value_method_version: GENERAL_SEM_NULL_CENTERED_PLUS_ONE_P_VALUE_METHOD_VERSION_V1,
    failure_policy_version: GENERAL_SEM_MINIMUM_USABLE_FRACTION_POLICY_VERSION_V1,
    compilation_artifact_identity_sha256: "d".repeat(64),
    compiled_plan_sha256: "9".repeat(64),
    general_sem_config_sha256: "e".repeat(64),
    recipe_analytical_sha256: document.provenance.recipe_digest,
    model_scientific_sha256: document.provenance.model_digest,
    source_dataset_fingerprint: document.provenance.dataset_fingerprint,
    complete_case_frame_sha256: "f".repeat(64),
    usable_replicate_indices_sha256: await serializedSha256Fixture(usableReplicateIndices),
    effect_identity_set_sha256: await serializedSha256Fixture(typedEffectIdentitiesFixture(results)),
    effect_ids: effectIds,
    interval: "percentile_type7",
    tail: "two_sided",
    confidence_level: 0.95,
    resamples_requested: 10,
    resamples_usable: 9,
    minimum_usable_resamples: 9,
    seed: String(document.provenance.seed),
    workers: document.provenance.workers,
    complete_model_reestimated_per_replicate: true,
    failed_replicates: failedReplicates,
  };
  return results;
}

const PRIOR_ESTABLISHED_TABLE_OWNERS = [
  {
    tableId: "cca_prior_parity",
    ownerOptions: ["cca"],
    requirements: [
      { capability_id: "smartpls.cca", cell_id: "qpls3.assessment.cca_residuals", option: "cca" },
    ],
  },
  {
    tableId: "gsca_prior_parity",
    ownerOptions: ["gsca"],
    requirements: [
      { capability_id: "smartpls.gsca", cell_id: "qpls3.gsca.als", option: "gsca" },
    ],
  },
  {
    tableId: "ipma_prior_parity",
    ownerOptions: ["ipma"],
    requirements: [
      { capability_id: "smartpls.ipma", cell_id: "qpls3.assessment.ipma", option: "ipma" },
    ],
  },
  {
    tableId: "nca_prior_parity",
    ownerOptions: ["nca"],
    requirements: [
      { capability_id: "smartpls.nca", cell_id: "qpls3.standalone.nca", option: "nca" },
    ],
  },
] as const;

describe("CanonicalResultDocumentV2 native runtime adapter", () => {
  it("adopts generated canonical table owners without changing prior primary tuples", () => {
    for (const prior of PRIOR_ESTABLISHED_TABLE_OWNERS) {
      const generatedOwners = establishedCanonicalTableOwnerOptionsV1(prior.tableId);
      const generatedRequirements = generatedOwners.flatMap((ownerOption) => (
        ESTABLISHED_METHOD_CONTRACTS_V1.flatMap((contract) => contract.capability_requirements
          .filter((item) => item.option === ownerOption)
          .map((item) => ({
            capability_id: item.capability_id,
            cell_id: item.cell_id,
            option: item.option,
          })))
      ));
      expect(generatedOwners).toEqual(prior.ownerOptions);
      expect(generatedRequirements).toEqual(prior.requirements);
      expect(nativeCapabilityRequirementsForTableV2(prior.tableId)).toEqual(prior.requirements);
    }
  });

  it("continues legacy dynamic and unknown table fallbacks when generated ownership does not match", () => {
    expect(establishedCanonicalTableOwnerOptionsV1("plsc_permutation_accounting")).toEqual([]);
    expect(nativeCapabilityRequirementsForTableV2("plsc_permutation_accounting")).toEqual([{
      capability_id: "smartpls.consistent_permutation",
      cell_id: "qpls3.inference.consistent_permutation",
      option: "consistent_permutation",
    }]);
    expect(establishedCanonicalTableOwnerOptionsV1("future_method_table")).toEqual([]);
    expect(nativeCapabilityRequirementsForTableV2("future_method_table")).toBeNull();
  });

  it("attributes PLSc permutation tables to the exact consistent-permutation cell", () => {
    for (const tableId of [
      "plsc_permutation_accounting",
      "plsc_permutation_groups",
      "plsc_permutation_paths",
      "plsc_permutation_outer_loadings",
      "plsc_permutation_construct_criteria",
      "plsc_permutation_failures",
    ]) {
      expect(nativeCapabilityRequirementsForTableV2(tableId)).toEqual([{
        capability_id: "smartpls.consistent_permutation",
        cell_id: "qpls3.inference.consistent_permutation",
        option: "consistent_permutation",
      }]);
    }
    expect(nativeCapabilityRequirementsForTableV2("plsc_reliability")).toEqual([{
      capability_id: "smartpls.plsc",
      cell_id: "qpls3.pls.consistent",
      option: "consistent_pls",
    }]);
  });

  it("builds a strict typed PLS document with exact native table identities", async () => {
    const run = currentPlsRun();
    const built = await canonicalResultDocumentFromAnalysisRunV2(run, {
      projectId: "project-corporate-reputation",
      datasetId: "dataset-corporate-reputation",
    });

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.mode).toBe("current_typed_bridge");
    expect(validateCanonicalResultDocumentV2(built.document)).toEqual({ passed: true, errors: [] });
    expect(built.document.tables.map((table) => table.id)).toEqual(
      buildNativeResultNavigation(run).tables.map((table) => table.id).filter((id) => id !== "blindfolding"),
    );
    expect(new Set(built.document.sections.flatMap((section) => section.table_ids))).toEqual(
      new Set(built.document.tables.map((table) => table.id)),
    );
    const paths = built.document.tables.find((table) => table.id === "direct_effects");
    expect(paths?.rows[0].cells.some((cell) => cell.kind === "number")).toBe(true);
    expect(built.document.provenance).toMatchObject({
      run_id: run.id,
      project_id: "project-corporate-reputation",
      dataset_id: "dataset-corporate-reputation",
      dataset_fingerprint: "a".repeat(64),
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
      },
      workers: 4,
    });
    expect(built.document.capability_cells?.map((reference) => reference.capability_id)).toEqual([
      "smartpls.htmt",
      "smartpls.mediation",
      "smartpls.model_fit",
      "smartpls.pls_algorithm",
      "smartpls.pls_bootstrapping",
    ]);
    expect(built.document.tables.find((table) => table.id === "mediation_bootstrap")?.capability_cells
      ?.map((reference) => reference.capability_id)).toEqual([
      "smartpls.mediation",
      "smartpls.pls_bootstrapping",
    ]);
    expect(built.document.sections.every((section) => (section.capability_cells?.length ?? 0) > 0)).toBe(true);
    expect(built.document.tables.map((table) => table.id)).not.toContain("blindfolding");
    expect(built.document.exclusions).toEqual([
      expect.objectContaining({
        id: "historical_blindfolding_omitted",
        capability_cell: expect.objectContaining({ capability_id: "smartpls.blindfolding" }),
      }),
    ]);
  });

  it("adapts a current non-PLS family and preserves GSCA table ordering", async () => {
    const run = completedGscaRun();
    const built = await canonicalResultDocumentFromAnalysisRunV2(run);

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.mode).toBe("current_typed_bridge");
    expect(built.document.provenance.capability_cell).toMatchObject({
      capability_id: "smartpls.gsca",
      cell_id: "qpls3.gsca.als",
    });
    expect(built.document.tables.map((table) => table.id)).toEqual(
      buildNativeResultNavigation(run).tables.map((table) => table.id),
    );
    expect(built.document.tables.map((table) => table.id)).toContain("gsca_fit");
    expect(built.document.capability_cells).toEqual([built.document.provenance.capability_cell]);
    expect(new Set(built.document.notices.map((notice) => notice.message)).size).toBe(built.document.notices.length);
    expect(built.document.notices.map((notice) => notice.code)).toContain("legacy_dataset_fingerprint_identifier");
  });

  it("keeps display preferences and diagram presentation out of analytical equality", async () => {
    const firstRun = currentPlsRun();
    const secondRun = currentPlsRun();
    secondRun.modelSnapshot!.nodes[0].position = { x: 999, y: 888 };
    secondRun.modelSnapshot!.diagramLayout = {
      diagramVersion: "sem_designer_v1",
      constructLayouts: {},
      indicatorLayouts: {},
      edgeLayouts: {},
      diagramTheme: "journal_mono",
      showGrid: false,
      layoutLocked: true,
    };
    secondRun.provenance!.settings.workers = 1;

    const first = await canonicalResultDocumentFromAnalysisRunV2(firstRun, {
      presentation: { precision: 2, missingValueLabel: "N/A", chartDefaults: { palette: "institutional_navy" } },
    });
    const second = await canonicalResultDocumentFromAnalysisRunV2(secondRun, {
      presentation: { precision: 8, missingValueLabel: "—", chartDefaults: { palette: "journal_mono", show_values: true } },
    });

    expect(first.ok && second.ok).toBe(true);
    if (!first.ok || !second.ok) return;
    expect(first.document.presentation).not.toEqual(second.document.presentation);
    expect(first.document.provenance.model_digest).toBe(second.document.provenance.model_digest);
    expect(first.document.provenance.recipe_digest).toBe(second.document.provenance.recipe_digest);
    expect(canonicalAnalyticalResultJson(first.document)).toBe(canonicalAnalyticalResultJson(second.document));
  });

  it("treats the recipe id as provenance rather than an analytical setting", async () => {
    const firstRun = currentPlsRun();
    const secondRun = structuredClone(firstRun);
    secondRun.id = "run-pls-runtime-v2-repeat";
    secondRun.provenance!.recipe_id = "recipe-pls-runtime-v2-repeat";

    const first = await canonicalResultDocumentFromAnalysisRunV2(firstRun);
    const second = await canonicalResultDocumentFromAnalysisRunV2(secondRun);
    expect(first.ok).toBe(true);
    expect(second.ok).toBe(true);
    if (!first.ok || !second.ok) return;

    expect(first.document.provenance.recipe_id).not.toBe(second.document.provenance.recipe_id);
    expect(first.document.provenance.recipe_digest).toBe(second.document.provenance.recipe_digest);
  });

  it("keeps presentation-only edges outside the scientific model digest", async () => {
    const firstRun = currentPlsRun();
    const secondRun = structuredClone(firstRun);
    secondRun.modelSnapshot!.edges.push(convertNativeCovarianceToPresentationV4({
      id: "visual-covariance",
      source: "competence",
      target: "satisfaction",
      data: { role: "covariance" },
    }));

    const first = await canonicalResultDocumentFromAnalysisRunV2(firstRun);
    const second = await canonicalResultDocumentFromAnalysisRunV2(secondRun);
    expect(first.ok).toBe(true);
    expect(second.ok).toBe(true);
    if (!first.ok || !second.ok) return;
    expect(first.document.provenance.model_digest).toBe(second.document.provenance.model_digest);
  });

  it("binds explicit estimands and scientific covariances into the model digest", async () => {
    const baseline = currentPlsRun();
    const changed = structuredClone(baseline);
    changed.modelSnapshot!.nodes[0] = withNativeConstructEstimandV4(
      changed.modelSnapshot!.nodes[0],
      { kind: "common_factor", marker_indicator: "COMP1" },
    );
    changed.modelSnapshot!.edges.push(newNativeScientificCovarianceEdgeV4(
      "model-covariance",
      "competence",
      "satisfaction",
    ));

    const first = await canonicalResultDocumentFromAnalysisRunV2(baseline);
    const second = await canonicalResultDocumentFromAnalysisRunV2(changed);
    expect(first.ok).toBe(true);
    expect(second.ok).toBe(true);
    if (!first.ok || !second.ok) return;
    expect(first.document.provenance.model_digest).not.toBe(second.document.provenance.model_digest);
  });

  it("fails closed for non-finite analytical values and tampered method identity", async () => {
    const nonFinite = currentPlsRun();
    nonFinite.result!.paths[0].coefficient = Number.NaN;
    await expect(canonicalResultDocumentFromAnalysisRunV2(nonFinite)).resolves.toMatchObject({
      ok: false,
      code: "invalid_analytical_payload",
    });

    const unknownMethod = currentPlsRun();
    (unknownMethod.provenance as { method: string }).method = "tampered_method";
    (unknownMethod.provenance!.settings as { method: string }).method = "tampered_method";
    await expect(canonicalResultDocumentFromAnalysisRunV2(unknownMethod)).resolves.toMatchObject({
      ok: false,
      code: "unresolved_capability_cell",
    });
  });

  it("keeps historical runs readable through a text-only fallback", async () => {
    const historical = completedSamplePlsRun();
    expect(historical.provenance).toBeUndefined();

    const built = await canonicalResultDocumentFromAnalysisRunV2(historical);

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.mode).toBe("historical_text_fallback");
    expect(built.document.sections).toHaveLength(1);
    expect(built.document.sections[0].id).toBe("historical_results");
    expect(built.document.tables.flatMap((table) => table.rows).flatMap((row) => row.cells)
      .every((cell) => cell.kind === "text")).toBe(true);
    expect(built.document.tables.map((table) => table.id)).toContain("blindfolding");
    expect(validateCanonicalResultDocumentV2(built.document)).toEqual({ passed: true, errors: [] });
  });

  it("strictly and losslessly reads every General SEM result family", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = completeGeneralSemResultsFixture(document);
    const encoded = JSON.stringify(document);

    const readback = parseNativeCanonicalResultDocumentV2(JSON.parse(encoded));

    expect(readback).toEqual(document);
    expect(JSON.stringify(readback)).toBe(encoded);
    expect(readback.general_sem_results?.specific_indirect_effects).toHaveLength(1);
    expect(readback.general_sem_results?.aggregate_effects?.map((effect) => effect.kind)).toEqual([
      "total_effect",
      "total_indirect",
    ]);
    expect(readback.general_sem_results?.conditional_effect_probes?.map((probe) => probe.values.kind)).toEqual([
      "data_derived_mean_plus_minus_one_sd",
      "explicit",
    ]);
    expect(readback.general_sem_results?.interaction_plots?.[0].series).toHaveLength(2);
    expect(readback.general_sem_results?.higher_order_stages?.map((stage) => stage.kind)).toEqual([
      "lower_order_score_estimation",
      "higher_order_estimation",
    ]);
    expect(readback.general_sem_results?.cbsem_fit?.[0].rmsea_interval?.confidence_level).toBe(0.9);
    expect(readback.general_sem_results?.identification_diagnostics?.[0].status).toBe("identified");
    expect(Object.prototype.hasOwnProperty.call(readback.general_sem_results, "inference_receipt")).toBe(false);
    const pointEstimate = readback.general_sem_results?.specific_indirect_effects?.[0].value;
    for (const field of [
      "bootstrap_mean", "bootstrap_bias", "standard_error", "lower", "upper", "p_value",
      "bootstrap_usable_replicates", "bootstrap_two_sided_exceedances",
    ]) {
      expect(Object.prototype.hasOwnProperty.call(pointEstimate, field)).toBe(false);
    }
  });

  it("reads interaction coefficient provenance and rejects scale or cross-reference tampering", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = completeGeneralSemInteractionResultsFixture(document);

    const readback = parseNativeCanonicalResultDocumentV2(structuredClone(document));
    expect(readback.general_sem_results?.interaction_effects?.[0]).toMatchObject({
      interaction_id: "interaction_x_by_w",
      standardized_product_coefficient: { estimate: 0.2 },
      scientific_rescaled_gamma: { estimate: 0.4 },
      construction_method: "two_stage",
    });
    expect(readback.general_sem_results?.joint_stage_structural_coefficients?.[0]).toMatchObject({
      relation_id: "relation_x_y",
      parameter_id: "parameter_x_y",
      role: "structural",
      estimate: { estimate: 0.3 },
      stage: "joint_stage_two",
    });
    expect(readback.general_sem_results?.conditional_effects?.[0].interaction_effect_id)
      .toBe("relation_interaction_x_by_w_effect");
    expect(readback.general_sem_results?.interaction_plots?.[0].interaction_effect_id)
      .toBe("relation_interaction_x_by_w_effect");

    const wrongGamma = structuredClone(document);
    wrongGamma.general_sem_results!.interaction_effects![0]!.scientific_rescaled_gamma.estimate = 0.41;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongGamma))
      .toThrow(/scientific_rescaled_gamma must equal/);

    const missingCrossReference = structuredClone(document);
    delete missingCrossReference.general_sem_results!.conditional_effects![0]!.interaction_effect_id;
    expect(() => parseNativeCanonicalResultDocumentV2(missingCrossReference))
      .toThrow(/interaction_effect_id is required/);

    const wrongProjection = structuredClone(document);
    wrongProjection.general_sem_results!.interaction_effects![0]!.stage_one_model_scientific_sha256 =
      wrongProjection.provenance.model_digest;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongProjection))
      .toThrow(/projected interaction-free scoring model/);

    const omittedJointStageLedger = structuredClone(document);
    delete omittedJointStageLedger.general_sem_results!.joint_stage_structural_coefficients;
    expect(() => parseNativeCanonicalResultDocumentV2(omittedJointStageLedger))
      .toThrow(/joint_stage_structural_coefficients and interaction_effects must both be present/);

    const inferredJointStageCoefficient = structuredClone(document);
    inferredJointStageCoefficient.general_sem_results!
      .joint_stage_structural_coefficients![0]!.estimate = {
        estimate: 0.3,
        bootstrap_mean: 0.3,
        bootstrap_bias: 0,
        standard_error: 0.1,
        lower: 0.1,
        upper: 0.5,
        p_value: 0.2,
        bootstrap_usable_replicates: 9,
        bootstrap_two_sided_exceedances: 1,
      };
    expect(() => parseNativeCanonicalResultDocumentV2(inferredJointStageCoefficient))
      .toThrow(/must contain point estimation only/);

    const wrongProbePolicy = structuredClone(document);
    wrongProbePolicy.general_sem_results!.conditional_effect_probes![0]!.values = {
      kind: "explicit",
      values: [-2, 0, 1],
    };
    expect(() => parseNativeCanonicalResultDocumentV2(wrongProbePolicy))
      .toThrow(/frozen standardized -1\/0\/\+1 interaction policy/);

    const omittedConditionalRow = structuredClone(document);
    omittedConditionalRow.general_sem_results!.conditional_effects!.pop();
    expect(() => parseNativeCanonicalResultDocumentV2(omittedConditionalRow))
      .toThrow(/exactly three conditional rows/);

    const omittedPlot = structuredClone(document);
    omittedPlot.general_sem_results!.interaction_plots = [];
    expect(() => parseNativeCanonicalResultDocumentV2(omittedPlot))
      .toThrow(/exactly one cross-referenced interaction plot/);
  });

  it("strictly and losslessly reads the exact General SEM bootstrap inference receipt", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);
    const encoded = JSON.stringify(document);

    const readback = parseNativeCanonicalResultDocumentV2(JSON.parse(encoded));

    expect(readback).toEqual(document);
    expect(JSON.stringify(readback)).toBe(encoded);
    expect(readback.general_sem_results?.inference_receipt).toMatchObject({
      kind: "case_bootstrap",
      method_version: GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
      interval: "percentile_type7",
      tail: "two_sided",
      resamples_requested: 10,
      resamples_usable: 9,
      minimum_usable_resamples: 9,
      seed: String(document.provenance.seed),
    });
    expect(readback.general_sem_results?.specific_indirect_effects?.[0].value).toMatchObject({
      bootstrap_mean: 0.13,
      bootstrap_bias: 0.01,
      standard_error: 0.04,
      p_value: 0.2,
      bootstrap_usable_replicates: 9,
      bootstrap_two_sided_exceedances: 1,
    });
  });

  it("requires the complete eight-field estimate tuple and its receipt", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);

    const partial = structuredClone(document);
    delete partial.general_sem_results!.specific_indirect_effects![0]!.value.upper;
    expect(() => parseNativeCanonicalResultDocumentV2(partial)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].value",
    }));

    const missingLedgerCount = structuredClone(document);
    delete missingLedgerCount.general_sem_results!.specific_indirect_effects![0]!.value.bootstrap_two_sided_exceedances;
    expect(() => parseNativeCanonicalResultDocumentV2(missingLedgerCount)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].value",
    }));

    const wrongBias = structuredClone(document);
    wrongBias.general_sem_results!.specific_indirect_effects![0]!.value.bootstrap_bias = 0.02;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongBias)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].value.bootstrap_bias",
    }));

    const missingReceipt = structuredClone(document);
    delete missingReceipt.general_sem_results!.inference_receipt;
    expect(() => parseNativeCanonicalResultDocumentV2(missingReceipt)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.inference_receipt",
    }));
  });

  it("fails closed on receipt versions, executor choices, plan bounds, seed, and exact fields", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);
    const expectReceiptFailure = (
      mutate: (receipt: Record<string, unknown>) => void,
      path: string,
      code: "document.invalid" | "schema.unknown_field" = "document.invalid",
    ) => {
      const changed = structuredClone(document);
      const receipt = changed.general_sem_results!.inference_receipt as unknown as Record<string, unknown>;
      mutate(receipt);
      expect(() => parseNativeCanonicalResultDocumentV2(changed)).toThrowError(expect.objectContaining({ code, path }));
    };

    expectReceiptFailure((receipt) => { receipt.unexpected = true; }, "general_sem_results.inference_receipt.unexpected", "schema.unknown_field");
    expectReceiptFailure((receipt) => { receipt.method_version = "other_method_v1"; }, "general_sem_results.inference_receipt.method_version");
    expectReceiptFailure((receipt) => { receipt.resampling_operation_version = "other_operation_v1"; }, "general_sem_results.inference_receipt.resampling_operation_version");
    expectReceiptFailure((receipt) => { receipt.resampling_stream_version = "other_stream_v1"; }, "general_sem_results.inference_receipt.resampling_stream_version");
    expectReceiptFailure((receipt) => { receipt.quantile_method_version = "other_quantile_v1"; }, "general_sem_results.inference_receipt.quantile_method_version");
    expectReceiptFailure((receipt) => { receipt.standard_error_method_version = "other_standard_error_v1"; }, "general_sem_results.inference_receipt.standard_error_method_version");
    expectReceiptFailure((receipt) => { receipt.summation_method_version = "other_summation_v1"; }, "general_sem_results.inference_receipt.summation_method_version");
    expectReceiptFailure((receipt) => { receipt.p_value_method_version = "other_p_value_v1"; }, "general_sem_results.inference_receipt.p_value_method_version");
    expectReceiptFailure((receipt) => { receipt.failure_policy_version = "other_failure_v1"; }, "general_sem_results.inference_receipt.failure_policy_version");
    expectReceiptFailure((receipt) => { receipt.interval = "bca"; }, "general_sem_results.inference_receipt.interval");
    expectReceiptFailure((receipt) => { receipt.tail = "one_sided_upper"; }, "general_sem_results.inference_receipt.tail");
    expectReceiptFailure((receipt) => { receipt.resamples_requested = 1; }, "general_sem_results.inference_receipt.resamples_requested");
    expectReceiptFailure((receipt) => { receipt.minimum_usable_resamples = 8; }, "general_sem_results.inference_receipt.minimum_usable_resamples");
    expectReceiptFailure((receipt) => { receipt.seed = "01"; }, "general_sem_results.inference_receipt.seed");
    expectReceiptFailure((receipt) => { receipt.seed = "9007199254740992"; }, "general_sem_results.inference_receipt.seed");
    expectReceiptFailure((receipt) => { receipt.seed = "41"; }, "general_sem_results.inference_receipt.seed");
    expectReceiptFailure((receipt) => { receipt.workers = 3; }, "general_sem_results.inference_receipt.workers");
    expectReceiptFailure((receipt) => { receipt.complete_model_reestimated_per_replicate = false; }, "general_sem_results.inference_receipt.complete_model_reestimated_per_replicate");
    expectReceiptFailure((receipt) => {
      (receipt.capability_cell as Record<string, unknown>).capability_version = "general_sem_pls_full_model_case_bootstrap_v0";
    }, "general_sem_results.inference_receipt.capability_cell");
  });

  it("binds receipt provenance digests and accepts the exact versioned dataset fingerprint form", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);

    for (const field of [
      "compiled_plan_sha256",
      "recipe_analytical_sha256",
      "model_scientific_sha256",
    ] as const) {
      const changed = structuredClone(document);
      changed.general_sem_results!.inference_receipt![field] = "z".repeat(64);
      expect(() => parseNativeCanonicalResultDocumentV2(changed)).toThrowError(expect.objectContaining({
        path: `general_sem_results.inference_receipt.${field}`,
      }));
    }

    for (const field of ["recipe_analytical_sha256", "model_scientific_sha256"] as const) {
      const changed = structuredClone(document);
      const current = changed.general_sem_results!.inference_receipt![field];
      changed.general_sem_results!.inference_receipt![field] = current === "0".repeat(64)
        ? "1".repeat(64)
        : "0".repeat(64);
      expect(() => parseNativeCanonicalResultDocumentV2(changed)).toThrowError(expect.objectContaining({
        code: "document.invalid",
        path: `general_sem_results.inference_receipt.${field}`,
      }));
    }

    const versioned = structuredClone(document);
    versioned.provenance.dataset_fingerprint = `v2:${document.provenance.dataset_fingerprint}`;
    versioned.general_sem_results!.inference_receipt!.source_dataset_fingerprint = versioned.provenance.dataset_fingerprint;
    expect(parseNativeCanonicalResultDocumentV2(versioned)).toEqual(versioned);

    const mismatchedDataset = structuredClone(document);
    mismatchedDataset.general_sem_results!.inference_receipt!.source_dataset_fingerprint = "c".repeat(64);
    expect(() => parseNativeCanonicalResultDocumentV2(mismatchedDataset)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.inference_receipt.source_dataset_fingerprint",
    }));
  });

  it("binds the ordered failure ledger and exact covered effect identity digests", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);
    const expectReceiptFailure = (
      mutate: (receipt: Record<string, unknown>) => void,
      path: string,
      code: "document.invalid" | "schema.invalid_discriminator" = "document.invalid",
    ) => {
      const changed = structuredClone(document);
      const receipt = changed.general_sem_results!.inference_receipt as unknown as Record<string, unknown>;
      mutate(receipt);
      expect(() => parseNativeCanonicalResultDocumentV2(changed)).toThrowError(expect.objectContaining({
        code,
        path,
      }));
    };

    expectReceiptFailure((receipt) => {
      receipt.effect_ids = ["effect_other", "effect_total_1", "effect_total_2"];
    }, "general_sem_results.inference_receipt.effect_ids");
    expectReceiptFailure((receipt) => {
      receipt.effect_identity_set_sha256 = "a".repeat(64);
    }, "general_sem_results.inference_receipt.effect_identity_set_sha256");
    expectReceiptFailure((receipt) => {
      receipt.usable_replicate_indices_sha256 = "a".repeat(64);
    }, "general_sem_results.inference_receipt.usable_replicate_indices_sha256");
    expectReceiptFailure((receipt) => {
      receipt.resamples_requested = 20;
      receipt.resamples_usable = 18;
      receipt.minimum_usable_resamples = 18;
      receipt.failed_replicates = [
        { replicate_index: 7, reason_code: "estimation_nonconvergence", message: "First failure." },
        { replicate_index: 3, reason_code: "estimation_nonconvergence", message: "Second failure." },
      ];
    }, "general_sem_results.inference_receipt.failed_replicates");

    const changedTypedIdentity = structuredClone(document);
    changedTypedIdentity.general_sem_results!.aggregate_effects![0]!.source_id = "construct_other";
    expect(() => parseNativeCanonicalResultDocumentV2(changedTypedIdentity)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.inference_receipt.effect_identity_set_sha256",
    }));

    for (const reason of [
      "insufficient_observations",
      "constant_indicator",
      "rank_deficient",
      "isolated_construct",
      "estimation_nonconvergence",
      "numerical_failure",
    ] as const) {
      const recognizedReason = structuredClone(document);
      recognizedReason.general_sem_results!.inference_receipt!.failed_replicates[0]!.reason_code = reason;
      expect(parseNativeCanonicalResultDocumentV2(recognizedReason)).toEqual(recognizedReason);
    }

    expectReceiptFailure((receipt) => {
      (receipt.failed_replicates as Array<Record<string, unknown>>)[0]!.reason_code = "other_failure";
    }, "general_sem_results.inference_receipt.failed_replicates[0].reason_code", "schema.invalid_discriminator");
  });

  it("binds canonical path and aggregate decomposition identities", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = completeGeneralSemResultsFixture(document);

    const wrongSpecificIdentity = structuredClone(document);
    wrongSpecificIdentity.general_sem_results!.specific_indirect_effects![0]!.effect_id = "effect_specific_1";
    expect(() => parseNativeCanonicalResultDocumentV2(wrongSpecificIdentity)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].effect_id",
    }));

    const wrongAggregateIdentity = structuredClone(document);
    wrongAggregateIdentity.general_sem_results!.aggregate_effects![0]!.effect_id = "effect_total_1";
    expect(() => parseNativeCanonicalResultDocumentV2(wrongAggregateIdentity)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.aggregate_effects[0].effect_id",
    }));

    const contradictoryIndirect = structuredClone(document);
    contradictoryIndirect.general_sem_results!.aggregate_effects![1]!.direct_relation_ids = ["relation_direct"];
    expect(() => parseNativeCanonicalResultDocumentV2(contradictoryIndirect)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.aggregate_effects[1].direct_relation_ids",
    }));
  });

  it("binds per-effect replicate counts, plus-one p-values, and the recursive-effects trace cell", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);

    const wrongUsableCount = structuredClone(document);
    wrongUsableCount.general_sem_results!.specific_indirect_effects![0]!.value.bootstrap_usable_replicates = 8;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongUsableCount)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].value.bootstrap_usable_replicates",
    }));

    const wrongExceedanceCount = structuredClone(document);
    wrongExceedanceCount.general_sem_results!.specific_indirect_effects![0]!.value.bootstrap_two_sided_exceedances = 10;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongExceedanceCount)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].value.bootstrap_two_sided_exceedances",
    }));

    const wrongPValue = structuredClone(document);
    wrongPValue.general_sem_results!.specific_indirect_effects![0]!.value.p_value = 0.21;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongPValue)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].value.p_value",
    }));

    const wrongTraceCell = structuredClone(document);
    wrongTraceCell.general_sem_results!.specific_indirect_effects![0]!.trace.capability_cell = {
      ...wrongTraceCell.general_sem_results!.inference_receipt!.capability_cell,
    };
    expect(() => parseNativeCanonicalResultDocumentV2(wrongTraceCell)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.specific_indirect_effects[0].trace.capability_cell",
    }));
  });

  it("does not claim receipt coverage for conditional or higher-order inference", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = await completeGeneralSemInferenceResultsFixture(document);
    const inferred = structuredClone(document.general_sem_results.specific_indirect_effects![0]!.value);

    const conditional = structuredClone(document);
    conditional.general_sem_results!.conditional_effects![0]!.value = structuredClone(inferred);
    expect(() => parseNativeCanonicalResultDocumentV2(conditional)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.inference_receipt",
    }));

    const higherOrder = structuredClone(document);
    higherOrder.general_sem_results!.higher_order_stages![1]!.relation_estimates![0]!.value = structuredClone(inferred);
    expect(() => parseNativeCanonicalResultDocumentV2(higherOrder)).toThrowError(expect.objectContaining({
      code: "document.invalid",
      path: "general_sem_results.inference_receipt",
    }));
  });

  it("preserves legacy omission and byte ordering when General SEM results are absent", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const encoded = JSON.stringify(built.document);

    const readback = parseNativeCanonicalResultDocumentV2(JSON.parse(encoded));

    expect(Object.prototype.hasOwnProperty.call(readback, "general_sem_results")).toBe(false);
    expect(JSON.stringify(readback)).toBe(encoded);
  });

  it("rejects unknown extension fields, non-finite values, schemas, and discriminators", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const fixture = structuredClone(built.document);
    fixture.general_sem_results = completeGeneralSemResultsFixture(fixture);

    const unknown = structuredClone(fixture) as unknown as {
      general_sem_results: { specific_indirect_effects: Array<{ value: Record<string, unknown> }> };
    };
    unknown.general_sem_results.specific_indirect_effects[0]!.value.unexpected = true;
    expect(() => parseNativeCanonicalResultDocumentV2(unknown)).toThrowError(expect.objectContaining({
      code: "schema.unknown_field",
      path: "general_sem_results.specific_indirect_effects[0].value.unexpected",
    }));

    const nonFinite = structuredClone(fixture);
    nonFinite.general_sem_results!.cbsem_fit![0]!.rmsea = Number.NaN;
    expect(() => parseNativeCanonicalResultDocumentV2(nonFinite)).toThrowError(expect.objectContaining({
      code: "schema.non_finite",
    }));

    const wrongSchema = structuredClone(fixture) as unknown as {
      general_sem_results: { schema_version: number };
    };
    wrongSchema.general_sem_results.schema_version = 2;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongSchema)).toThrowError(expect.objectContaining({
      code: "schema.version_unsupported",
      path: "general_sem_results.schema_version",
    }));

    const wrongKind = structuredClone(fixture) as unknown as {
      general_sem_results: { aggregate_effects: Array<{ kind: string }> };
    };
    wrongKind.general_sem_results.aggregate_effects[0]!.kind = "direct_effect";
    expect(() => parseNativeCanonicalResultDocumentV2(wrongKind)).toThrowError(expect.objectContaining({
      code: "schema.invalid_discriminator",
      path: "general_sem_results.aggregate_effects[0].kind",
    }));
  });
});
