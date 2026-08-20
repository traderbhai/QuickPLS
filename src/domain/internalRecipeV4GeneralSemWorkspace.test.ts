import { describe, expect, it, vi } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import { canonicalResultDocumentJson, validateCanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
  GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
  GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
  GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1,
  GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
} from "./canonicalGeneralSemResultsV1";
import {
  bindGeneralSemPlsModelToDatasetV1,
  appendGeneralSemResultV1,
  buildGeneralSemCbsemRecipeV3,
  buildGeneralSemRecipeV1,
  defaultGeneralSemPlsEngineOptionsV1,
  GENERAL_SEM_CBSEM_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_CBSEM_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
  generalSemConfigFromEngineV1,
  generalSemCbsemJobRequestFromReceiptV1,
  generalSemJobRequestFromReceiptV1,
  monitorGeneralSemPlsJobV1,
  parseGeneralSemPlsCompletedResultV1,
  parseGeneralSemPlsJobSnapshotV1,
  parseGeneralSemProjectBootstrapOutcomeV1,
  preflightGeneralSemWorkspaceV1,
  rehydrateGeneralSemExecutionAuthorityV1,
  reopenGeneralSemResultV1,
  selectGeneralSemCbsemExecutionCapabilityV1,
  selectGeneralSemPlsExecutionCapabilityV1,
  validateGeneralSemPlsCompletedExecutionV1,
  type GeneralSemPlsCompletedResultV1,
  type GeneralSemPlsExecutionCapabilityV1,
  type GeneralSemPlsJobSnapshotV1,
  type GeneralSemProjectBootstrapReceiptV1,
} from "./internalRecipeV4GeneralSemWorkspace";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  preflightGeneralSemPlsV1,
} from "./generalSemCapabilityPreflightV1";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./internalProjectArchiveV6Read";
import { convertLegacyBasicModelV4, type SemModelV4 } from "./semModelV4";
import { sha256HexBytesV1, sha256HexUtf8V1 } from "./sha256V1";
import type { Dataset } from "../types";

const PROJECT_ID = "00000000-0000-4000-8000-000000000001";
const RECIPE_ID = "00000000-0000-4000-8000-000000000002";
const DIGEST_A = "a".repeat(64);
const DIGEST_B = "b".repeat(64);
const DIGEST_C = "c".repeat(64);

function rawDataset(): Dataset {
  const columns = ["x1", "x2", "m11", "m12", "m21", "m22", "y1", "y2"];
  return {
    id: "dataset:general-sem",
    name: "General SEM observations",
    kind: "raw",
    columns,
    rows: Array.from({ length: 24 }, (_, index) => Object.fromEntries(
      columns.map((column, columnIndex) => [column, index + columnIndex / 10]),
    )),
    rowCount: 24,
    missing: 0,
    fingerprint: DIGEST_B,
    columnMetadata: columns.map((name) => ({
      name,
      label: null,
      column_type: "numeric",
      role: "unassigned",
      scale_type: "continuous",
      missing_markers: [],
      theoretical_min: null,
      theoretical_max: null,
      value_labels: {},
    })),
  };
}

function multipleMediationModel(): SemModelV4 {
  return convertLegacyBasicModelV4({
    id: "model:general-sem",
    name: "Parallel mediation",
    constructs: ["x", "m1", "m2", "y"].map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective" as const,
      indicators: id === "m1" ? ["m11", "m12"] : id === "m2" ? ["m21", "m22"] : [`${id}1`, `${id}2`],
    })),
    paths: [
      { source: "x", target: "m1" },
      { source: "m1", target: "y" },
      { source: "x", target: "m2" },
      { source: "m2", target: "y" },
      { source: "x", target: "y" },
    ],
  }, "pls_composite");
}

function addTwoWayInteraction(
  value: SemModelV4,
  id: string,
  focalPredictor: string,
  moderator: string,
  outcome = "construct:y",
): void {
  const focal = value.relations.find((relation) => relation.kind === "structural"
    && relation.source === focalPredictor
    && relation.target === outcome);
  if (!focal) throw new Error(`Missing focal relation ${focalPredictor} -> ${outcome}`);
  const output = `derived:${id}`;
  const relationId = `relation:${id}:effect`;
  const parameterId = `parameter:${id}:effect`;
  value.variables.push({ kind: "derived", id: output, label: `${focalPredictor} × ${moderator}` });
  value.relations.push({
    kind: "structural",
    id: relationId,
    source: output,
    target: outcome,
    parameter: parameterId,
    intercept_parameter: null,
  });
  value.parameters.push({
    kind: "free",
    id: parameterId,
    label: `${id} effect`,
    target: { kind: "regression", source: output, target: outcome },
    group_overrides: [],
  });
  value.derived_terms.push({
    kind: "interaction_v2",
    id,
    output,
    operands: [focalPredictor, moderator],
    focal_relation: focal.id,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
}

function multipleModerationModel(layout: "same_focal" | "different_focal"): SemModelV4 {
  const value = convertLegacyBasicModelV4({
    id: "model:general-sem",
    name: "Multiple moderation",
    constructs: ["w", "x", "z", "y"].map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective" as const,
      indicators: [`${id}1`, `${id}2`],
    })),
    paths: ["w", "x", "z"].map((source) => ({ source, target: "y" })),
  }, "pls_composite");
  addTwoWayInteraction(value, "interaction:x:w", "construct:x", "construct:w");
  addTwoWayInteraction(
    value,
    layout === "same_focal" ? "interaction:x:z" : "interaction:z:w",
    layout === "same_focal" ? "construct:x" : "construct:z",
    layout === "same_focal" ? "construct:z" : "construct:w",
  );
  return value;
}

function receipt(): GeneralSemProjectBootstrapReceiptV1 {
  return {
    schemaVersion: 1,
    archiveSchemaVersion: 6,
    projectId: PROJECT_ID,
    name: "General SEM calculation",
    createdAt: "2026-08-19T00:00:00Z",
    destinationArchivePath: "D:\\General-Sem.qpls",
    destinationArchiveSha256: DIGEST_A,
    destinationArchiveBytes: 4096,
    strictReopenValidated: true,
    residentDatasetId: "dataset:general-sem",
    residentDatasetFingerprint: DIGEST_B,
    residentModelId: "model:general-sem",
    residentModelScientificSha256: DIGEST_C,
    residentRecipeId: RECIPE_ID,
    residentRecipeDocumentSha256: "d".repeat(64),
  };
}

function snapshot(
  state: GeneralSemPlsJobSnapshotV1["state"],
  completedUnits: number,
): GeneralSemPlsJobSnapshotV1 {
  return {
    schemaVersion: 1,
    jobId: "job:general-sem",
    state,
    phase: state === "completed" ? "publication" : "estimation",
    completedUnits,
    totalUnits: 3,
    message: null,
    failure: state === "failed" ? {
      schemaVersion: 1,
      stage: "estimation",
      subject: "model:general-sem",
      code: "general_sem.estimation.failed",
      message: "The estimator did not converge.",
      correctiveAction: "Review the model and retry.",
      issues: [],
    } : null,
    queuedAt: "2026-08-19T00:00:00Z",
    startedAt: state === "queued" ? null : "2026-08-19T00:00:01Z",
    completedAt: ["completed", "failed", "cancelled"].includes(state)
      ? "2026-08-19T00:00:02Z"
      : null,
  };
}

function canonicalDocument(projectId = PROJECT_ID): CanonicalResultDocumentV2 {
  const capabilityCell = GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  return {
    schema_version: 2,
    document_id: "result.general-sem:1",
    title: "General SEM result",
    provenance: {
      run_id: "run:general-sem:1",
      project_id: projectId,
      model_id: "model:general-sem",
      model_digest: DIGEST_C,
      dataset_id: "dataset:general-sem",
      dataset_fingerprint: DIGEST_B,
      recipe_id: RECIPE_ID,
      // The canonical provenance carries the analytical Recipe-v4 digest,
      // while archiveIdentity carries the full recipe-document digest.
      recipe_digest: "e".repeat(64),
      capability_cell: capabilityCell,
      method_version: "general_sem_pls_point_v1",
      engine_version: "test",
      seed: 42,
      workers: 1,
      started_at: "2026-08-19T00:00:00Z",
      completed_at: "2026-08-19T00:00:02Z",
    },
    capability_cells: [capabilityCell],
    sections: [],
    tables: [],
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: null,
      default_table_id: null,
      precision: 4,
      missing_value_label: "—",
      chart_defaults: {},
    },
    general_sem_results: {
      schema_version: 1,
      identification_diagnostics: [{
        diagnostic_id: "identification:model:general-sem",
        trace: { model_id: "model:general-sem", capability_cell: capabilityCell },
        scope: "model",
        subject_id: "model:general-sem",
        status: "identified",
        code: "identified",
        message: "The recursive model passed the current identification checks.",
        degrees_of_freedom: 1,
      }],
    },
  };
}

function moderationCanonicalDocument(): CanonicalResultDocumentV2 {
  const document = canonicalDocument();
  const cell = GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1;
  const interactionId = "interaction:x:w";
  const interactionEffectId = "relation:interaction:x:w:effect";
  const probeId = "probe:interaction:x:w";
  const trace = { model_id: document.provenance.model_id, capability_cell: cell };
  document.title = "PLS-SEM simultaneous two-way moderation point estimates";
  document.provenance.capability_cell = cell;
  document.provenance.method_version = GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1;
  document.capability_cells = [cell];
  document.general_sem_results = {
    schema_version: 1,
    joint_stage_structural_coefficients: [{
      relation_id: "relation.x.y",
      parameter_id: "parameter:relation.x.y",
      trace: structuredClone(trace),
      source_id: "construct:x",
      target_id: "construct:y",
      role: "structural",
      estimate: { estimate: 0.3 },
      stage: "joint_stage_two",
      method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
    }],
    interaction_effects: [{
      effect_id: interactionEffectId,
      trace: structuredClone(trace),
      interaction_id: interactionId,
      focal_relation_id: "relation.x.y",
      interaction_effect_relation_id: interactionEffectId,
      interaction_effect_parameter_id: "parameter:interaction:x:w:effect",
      focal_predictor_id: "construct:x",
      moderator_id: "construct:w",
      outcome_id: "construct:y",
      generated_product_column_id: "qpls_pls_product_v1_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      stage_one_model_scientific_sha256: "f".repeat(64),
      method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
      construction_method: "two_stage",
      product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
      hierarchy_policy: "strong",
      hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
      conditioning_policy_version: GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1,
      observation_count: 24,
      unstandardized_product_mean: 0.125,
      unstandardized_product_sample_standard_deviation: 0.5,
      standardized_product_coefficient: { estimate: 0.2 },
      scientific_rescaled_gamma: { estimate: 0.4 },
    }],
    conditional_effect_probes: [{
      probe_id: probeId,
      trace: structuredClone(trace),
      moderator_id: "construct:w",
      values: { kind: "explicit", values: [-1, 0, 1] },
    }],
    conditional_effects: [-1, 0, 1].map((moderatorValue, probeValueIndex) => ({
      effect_id: `conditional:interaction:x:w:${probeValueIndex}`,
      estimand_id: "conditional-slope:interaction:x:w",
      trace: structuredClone(trace),
      interaction_id: interactionId,
      interaction_effect_id: interactionEffectId,
      focal_relation_id: "relation.x.y",
      probe_id: probeId,
      moderator_id: "construct:w",
      probe_value_index: probeValueIndex,
      moderator_value: moderatorValue,
      value: { estimate: 0.3 + 0.2 * moderatorValue },
    })),
    interaction_plots: [{
      plot_id: "plot:interaction:x:w",
      trace,
      interaction_id: interactionId,
      interaction_effect_id: interactionEffectId,
      focal_relation_id: "relation.x.y",
      focal_predictor_id: "construct:x",
      moderator_id: "construct:w",
      outcome_id: "construct:y",
      series: [-1, 0, 1].map((moderatorValue, probeValueIndex) => ({
        series_id: `series:interaction:x:w:${probeValueIndex}`,
        probe_id: probeId,
        probe_value_index: probeValueIndex,
        moderator_value: moderatorValue,
        points: [-1, 0, 1].map((focalValue) => ({
          focal_value: focalValue,
          predicted_value: focalValue * (0.3 + 0.2 * moderatorValue),
        })),
      })),
    }],
  };
  return document;
}

function moderationBootstrapCanonicalDocument(): CanonicalResultDocumentV2 {
  const document = moderationCanonicalDocument();
  const results = document.general_sem_results!;
  const effect = results.interaction_effects![0]!;
  const usableIndices = Array.from({ length: 9 }, (_, index) => index);
  const failedReplicates = [{
    replicate_index: 9,
    reason_code: "constant_interaction_product" as const,
    message: "The sampled interaction product was constant.",
  }];
  effect.scientific_rescaled_gamma = {
    estimate: 0.4,
    bootstrap_mean: 0.41,
    bootstrap_bias: 0.01,
    standard_error: 0.1,
    lower: 0.2,
    upper: 0.6,
    p_value: 0.3,
    bootstrap_usable_replicates: 9,
    bootstrap_two_sided_exceedances: 2,
  };
  const canonicalIdentity = {
    kind: "interaction_scientific_rescaled_gamma" as const,
    effect_id: effect.effect_id,
    interaction_id: effect.interaction_id,
    focal_relation_id: effect.focal_relation_id,
    interaction_effect_relation_id: effect.interaction_effect_relation_id,
    interaction_effect_parameter_id: effect.interaction_effect_parameter_id,
    generated_product_column_id: effect.generated_product_column_id,
    focal_predictor_id: effect.focal_predictor_id,
    moderator_id: effect.moderator_id,
    outcome_id: effect.outcome_id,
    stage_one_model_scientific_sha256: effect.stage_one_model_scientific_sha256,
    product_scale_version: effect.product_scale_version,
    method_version: effect.method_version,
  };
  results.inference_receipt = {
    kind: "case_bootstrap",
    capability_cell: GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
    method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    resampling_operation_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    resampling_stream_version: "indexed_case_resampling_v1",
    quantile_method_version: "type7_quantile_v1",
    standard_error_method_version: "sample_standard_error_b_minus_1_v1",
    summation_method_version: "neumaier_compensated_sum_v1",
    p_value_method_version: "null_centered_plus_one_v1",
    failure_policy_version: "minimum_usable_fraction_0_9_v1",
    compilation_artifact_identity_sha256: "1".repeat(64),
    compiled_plan_sha256: "2".repeat(64),
    general_sem_config_sha256: "3".repeat(64),
    recipe_analytical_sha256: document.provenance.recipe_digest,
    model_scientific_sha256: document.provenance.model_digest,
    source_dataset_fingerprint: document.provenance.dataset_fingerprint,
    complete_case_frame_sha256: "4".repeat(64),
    usable_replicate_indices_sha256: sha256HexUtf8V1(JSON.stringify(usableIndices)),
    effect_identity_set_sha256: sha256HexUtf8V1(JSON.stringify([canonicalIdentity])),
    effect_ids: [effect.effect_id],
    interval: "percentile_type7",
    tail: "two_sided",
    confidence_level: 0.95,
    resamples_requested: 10,
    resamples_usable: 9,
    minimum_usable_resamples: 9,
    seed: "42",
    workers: 1,
    complete_model_reestimated_per_replicate: true,
    failed_replicates: failedReplicates,
  };
  const baseCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  };
  document.provenance.engine_version =
    "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_percentile_bootstrap_execution_v1";
  document.provenance.method_version =
    GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1;
  document.capability_cells = [
    GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
    GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
    baseCell,
  ];
  return document;
}

function moderationBootstrapCompletedFixture(): {
  completed: GeneralSemPlsCompletedResultV1;
  execution: GeneralSemPlsExecutionCapabilityV1;
} {
  const canonical = moderationBootstrapCanonicalDocument();
  const effect = canonical.general_sem_results!.interaction_effects![0]!;
  const receipt = canonical.general_sem_results!.inference_receipt!;
  const rawTarget = {
    kind: "interaction_scientific_rescaled_gamma" as const,
    target_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
    target_id: effect.effect_id,
    interaction_id: effect.interaction_id,
    focal_relation_id: effect.focal_relation_id,
    interaction_effect_relation_id: effect.interaction_effect_relation_id,
    interaction_effect_parameter_id: effect.interaction_effect_parameter_id,
    generated_product_column_id: effect.generated_product_column_id,
    focal_predictor_id: effect.focal_predictor_id,
    moderator_id: effect.moderator_id,
    outcome_id: effect.outcome_id,
    stage_one_model_scientific_sha256: effect.stage_one_model_scientific_sha256,
    product_scale_version: effect.product_scale_version,
    method_version: effect.method_version,
  };
  const failedReplicates = structuredClone(receipt.failed_replicates);
  const moderationBootstrapInference = {
    schema_version: 1,
    method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    point_method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
    resampling_operation_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_CASE_BOOTSTRAP_OPERATION_VERSION_V1,
    resampling_stream_version: "indexed_case_resampling_v1",
    quantile_method_version: "type7_quantile_v1",
    standard_error_method_version: "sample_standard_error_b_minus_1_v1",
    summation_method_version: "neumaier_compensated_sum_v1",
    p_value_method_version: "null_centered_plus_one_v1",
    failure_policy_version: "minimum_usable_fraction_0_9_v1",
    sign_alignment_method_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_SIGN_ALIGNMENT_VERSION_V1,
    product_scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
    gamma_target_version: GENERAL_SEM_PLS_MULTIPLE_MODERATION_GAMMA_TARGET_VERSION_V1,
    general_sem_config_sha256: receipt.general_sem_config_sha256,
    compiled_plan_sha256: receipt.compiled_plan_sha256,
    model_scientific_sha256: receipt.model_scientific_sha256,
    stage_one_model_scientific_sha256: effect.stage_one_model_scientific_sha256,
    source_dataset_fingerprint: receipt.source_dataset_fingerprint,
    complete_case_frame_sha256: receipt.complete_case_frame_sha256,
    usable_replicate_indices_sha256: receipt.usable_replicate_indices_sha256,
    gamma_target_identity_set_sha256: sha256HexUtf8V1(JSON.stringify([rawTarget])),
    gamma_target_ids: [rawTarget.target_id],
    interval: "percentile",
    tail: "two_sided",
    confidence_level: receipt.confidence_level,
    resamples_requested: receipt.resamples_requested,
    resamples_usable: receipt.resamples_usable,
    minimum_usable_resamples: receipt.minimum_usable_resamples,
    seed: receipt.seed,
    workers: receipt.workers,
    complete_model_reestimated_per_replicate: true,
    shared_stage_one_reestimated_per_replicate: true,
    score_vectors_sign_aligned_before_products: true,
    product_scaling_recomputed_per_replicate: true,
    joint_stage_two_reestimated_per_replicate: true,
    complete_joint_point_contract_validated_per_replicate: true,
    failed_replicates: failedReplicates,
    interaction_gammas: [{
      target: rawTarget,
      original: 0.4,
      bootstrap_mean: 0.41,
      bootstrap_bias: 0.01,
      standard_error: 0.1,
      lower: 0.2,
      upper: 0.6,
      p_value_two_sided: 0.3,
      usable_replicates: 9,
      two_sided_exceedances: 2,
    }],
  };
  const completed = completedResult();
  completed.canonicalDocument = canonical;
  completed.analyticalResult = {
    schema_version: 1,
    adapter_version:
      "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_percentile_bootstrap_execution_v1",
    capability_cell: GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
    compilation_artifact_identity_sha256: receipt.compilation_artifact_identity_sha256,
    compiled_plan_sha256: receipt.compiled_plan_sha256,
    recipe_analytical_sha256: canonical.provenance.recipe_digest,
    model_scientific_sha256: canonical.provenance.model_digest,
    stage_one_model_scientific_sha256: effect.stage_one_model_scientific_sha256,
    source_dataset_fingerprint: canonical.provenance.dataset_fingerprint,
    general_sem_config_sha256: receipt.general_sem_config_sha256,
    point_estimation: {},
    requested_effects: [],
    interaction_point_estimation: {
      method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
      observation_count: 24,
      product_scale_receipts: [],
      structural_coefficients: [],
      interaction_coefficients: [{
        interaction_id: effect.interaction_id,
        focal_relation_id: effect.focal_relation_id,
        interaction_effect_relation_id: effect.interaction_effect_relation_id,
        interaction_effect_parameter_id: effect.interaction_effect_parameter_id,
        focal_predictor_id: effect.focal_predictor_id,
        moderator_id: effect.moderator_id,
        outcome_id: effect.outcome_id,
        construction_method: "two_stage",
        hierarchy_policy: "strong",
        hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
        standardized_product_estimate: 0.2,
        raw_product_estimate: 0.4,
      }],
      simple_slopes: [],
    },
    moderation_bootstrap_inference: moderationBootstrapInference,
  };
  return {
    completed,
    execution: {
      kind: "multiple_two_way_moderation_bootstrap",
      capabilityCell: GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
      interactionIds: [effect.interaction_id],
      focalRelationIds: [effect.focal_relation_id],
    },
  };
}

function rustSpecificPathIdentityV1(relationIds: readonly string[]): string {
  const encoder = new TextEncoder();
  const domain = encoder.encode("qpls.compiled-sem-topology-v1.specific-directed-path\0");
  const encodedIds = relationIds.map((relationId) => encoder.encode(relationId));
  const totalLength = domain.length + encodedIds.reduce((total, bytes) => total + 8 + bytes.length, 0);
  const identityInput = new Uint8Array(totalLength);
  identityInput.set(domain);
  let offset = domain.length;
  for (const bytes of encodedIds) {
    const lengthView = new DataView(identityInput.buffer, offset, 8);
    const length = BigInt(bytes.length);
    lengthView.setUint32(0, Number(length >> 32n), false);
    lengthView.setUint32(4, Number(length & 0xffff_ffffn), false);
    offset += 8;
    identityInput.set(bytes, offset);
    offset += bytes.length;
  }
  return `sem_specific_path_v1_${sha256HexBytesV1(identityInput)}`;
}

function rustShapedBootstrapCanonicalDocument(): CanonicalResultDocumentV2 {
  const pointCell = GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  const bootstrapCell = GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1;
  const baseCell = {
    registry_schema_version: 2,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  } as const;
  const orderedRelationIds = ["relation.x.m1", "relation.m1.y"];
  const effectId = rustSpecificPathIdentityV1(orderedRelationIds);
  const effectIdentity = {
    kind: "specific_indirect" as const,
    effect_id: effectId,
    estimand_id: "estimand.x.y.m1",
    source_id: "x",
    target_id: "y",
    ordered_relation_ids: orderedRelationIds,
  };
  const usableReplicateIndices = [0, 1];

  return {
    ...canonicalDocument(),
    title: "PLS-SEM multiple mediation with full-model bootstrap",
    provenance: {
      ...canonicalDocument().provenance,
      capability_cell: pointCell,
      method_version: "general_sem_pls_full_model_case_bootstrap_v1",
      seed: 42,
      workers: 1,
    },
    // Rust sorts exact option-cell identities before serialization.
    capability_cells: [bootstrapCell, pointCell, baseCell],
    general_sem_results: {
      schema_version: 1,
      inference_receipt: {
        kind: "case_bootstrap",
        capability_cell: bootstrapCell,
        method_version: "general_sem_pls_full_model_case_bootstrap_v1",
        resampling_operation_version: "general_sem_pls_case_bootstrap_v1",
        resampling_stream_version: "indexed_case_resampling_v1",
        quantile_method_version: "type7_quantile_v1",
        standard_error_method_version: "sample_standard_error_b_minus_1_v1",
        summation_method_version: "neumaier_compensated_sum_v1",
        p_value_method_version: "null_centered_plus_one_v1",
        failure_policy_version: "minimum_usable_fraction_0_9_v1",
        compilation_artifact_identity_sha256: "1".repeat(64),
        compiled_plan_sha256: "2".repeat(64),
        general_sem_config_sha256: "3".repeat(64),
        recipe_analytical_sha256: "e".repeat(64),
        model_scientific_sha256: DIGEST_C,
        source_dataset_fingerprint: DIGEST_B,
        complete_case_frame_sha256: "4".repeat(64),
        usable_replicate_indices_sha256: sha256HexUtf8V1(JSON.stringify(usableReplicateIndices)),
        effect_identity_set_sha256: sha256HexUtf8V1(JSON.stringify([effectIdentity])),
        effect_ids: [effectId],
        interval: "percentile_type7",
        tail: "two_sided",
        confidence_level: 0.95,
        resamples_requested: 2,
        resamples_usable: 2,
        minimum_usable_resamples: 2,
        seed: "42",
        workers: 1,
        complete_model_reestimated_per_replicate: true,
        failed_replicates: [],
      },
      specific_indirect_effects: [{
        effect_id: effectId,
        estimand_id: effectIdentity.estimand_id,
        trace: { model_id: "model:general-sem", capability_cell: pointCell },
        source_id: effectIdentity.source_id,
        target_id: effectIdentity.target_id,
        ordered_relation_ids: orderedRelationIds,
        value: {
          estimate: 0.25,
          bootstrap_mean: 0.375,
          bootstrap_bias: 0.125,
          standard_error: 0.125,
          lower: 0.125,
          upper: 0.5,
          p_value: 1 / 3,
          bootstrap_usable_replicates: 2,
          bootstrap_two_sided_exceedances: 0,
        },
      }],
    },
  };
}

function completedResult(projectId = PROJECT_ID): GeneralSemPlsCompletedResultV1 {
  return {
    schemaVersion: 1,
    archiveIdentity: {
      archivePath: "D:\\General-Sem.qpls",
      archiveSha256: DIGEST_A,
      projectId: PROJECT_ID,
      datasetId: "dataset:general-sem",
      datasetFingerprint: DIGEST_B,
      modelId: "model:general-sem",
      modelScientificSha256: DIGEST_C,
      recipeId: RECIPE_ID,
      recipeDocumentSha256: "d".repeat(64),
    },
    analyticalResult: { schema_version: 1 },
    canonicalDocument: canonicalDocument(projectId),
  };
}

function completedExecutionFixture(
  kind: GeneralSemPlsExecutionCapabilityV1["kind"],
): { completed: GeneralSemPlsCompletedResultV1; execution: GeneralSemPlsExecutionCapabilityV1 } {
  const model = kind === "multiple_two_way_moderation_point"
    ? multipleModerationModel("same_focal")
    : multipleMediationModel();
  const config = generalSemConfigFromEngineV1({
    ...defaultGeneralSemPlsEngineOptionsV1(),
    inference: kind === "mediation_bootstrap" ? "percentile_case_bootstrap" : "none",
    bootstrapSamples: 500,
  });
  const execution = selectGeneralSemPlsExecutionCapabilityV1({
    model,
    config,
    decision: preflightGeneralSemPlsV1(model, config),
  });
  expect(execution.kind).toBe(kind);
  const completed = completedResult();
  const baseCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  };
  const primaryCell = kind === "multiple_two_way_moderation_point"
    ? GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1
    : GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
  const methodVersion = kind === "multiple_two_way_moderation_point"
    ? GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
    : kind === "mediation_bootstrap"
      ? "general_sem_pls_full_model_case_bootstrap_v1"
      : "general_sem_effects_v1";
  const adapterVersion = kind === "multiple_two_way_moderation_point"
    ? "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1"
    : kind === "mediation_bootstrap"
      ? "compiled_general_sem_pls_recipe_v1_percentile_bootstrap_execution_v1"
      : "compiled_general_sem_pls_recipe_v1_point_execution_v1";
  completed.canonicalDocument.provenance.capability_cell = primaryCell;
  completed.canonicalDocument.provenance.method_version = methodVersion;
  completed.canonicalDocument.provenance.engine_version = adapterVersion;
  completed.canonicalDocument.capability_cells = kind === "mediation_bootstrap"
    ? [GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1, GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1, baseCell]
    : [primaryCell, baseCell];
  const moderationIdentities = model.derived_terms
    .filter((term): term is Extract<SemModelV4["derived_terms"][number], { kind: "interaction_v2" }> => (
      term.kind === "interaction_v2"
    ))
    .map((term) => ({ interaction_id: term.id, focal_relation_id: term.focal_relation }))
    .sort((left, right) => left.interaction_id.localeCompare(right.interaction_id));
  if (kind === "multiple_two_way_moderation_point") {
    const interactionEffectTemplate = moderationCanonicalDocument()
      .general_sem_results?.interaction_effects?.[0];
    if (!interactionEffectTemplate) throw new Error("Expected the moderation canonical effect template.");
    completed.canonicalDocument.general_sem_results = {
      schema_version: 1,
      interaction_effects: moderationIdentities.map((identity, index) => ({
        ...structuredClone(interactionEffectTemplate),
        effect_id: `effect:completed:${index}`,
        trace: { model_id: completed.canonicalDocument.provenance.model_id, capability_cell: primaryCell },
        interaction_id: identity.interaction_id,
        focal_relation_id: identity.focal_relation_id,
        interaction_effect_relation_id: `relation:completed:interaction:${index}`,
        interaction_effect_parameter_id: `parameter:completed:interaction:${index}`,
      })),
    };
  }
  completed.analyticalResult = {
    schema_version: 1,
    adapter_version: adapterVersion,
    capability_cell: primaryCell,
    compilation_artifact_identity_sha256: "1".repeat(64),
    compiled_plan_sha256: "2".repeat(64),
    recipe_analytical_sha256: completed.canonicalDocument.provenance.recipe_digest,
    model_scientific_sha256: completed.archiveIdentity.modelScientificSha256,
    stage_one_model_scientific_sha256: kind === "multiple_two_way_moderation_point" ? "f".repeat(64) : completed.archiveIdentity.modelScientificSha256,
    source_dataset_fingerprint: completed.archiveIdentity.datasetFingerprint,
    general_sem_config_sha256: "3".repeat(64),
    point_estimation: {},
    requested_effects: [],
    ...(kind === "multiple_two_way_moderation_point" ? {
      interaction_point_estimation: {
        interaction_coefficients: moderationIdentities.map((identity) => ({ ...identity })),
      },
    } : {}),
    ...(kind === "mediation_bootstrap" ? { bootstrap_inference: {} } : {}),
  };
  return { completed, execution };
}

function rawCompletedInteractionCoefficients(
  completed: GeneralSemPlsCompletedResultV1,
): Array<Record<string, unknown>> {
  const analytical = completed.analyticalResult as Record<string, unknown>;
  const interactionPoint = analytical.interaction_point_estimation as Record<string, unknown>;
  return interactionPoint.interaction_coefficients as Array<Record<string, unknown>>;
}

function canonicalCompletedInteractionEffects(
  completed: GeneralSemPlsCompletedResultV1,
): Array<Record<string, unknown>> {
  return completed.canonicalDocument.general_sem_results?.interaction_effects as unknown as Array<Record<string, unknown>>;
}

describe("General SEM Recipe-v4 workspace contract", () => {
  it("builds distinct resident CB-SEM point and recursive-bootstrap recipes with exact Labs ownership", () => {
    const dataset = rawDataset();
    const model = convertLegacyBasicModelV4({
      id: "model:general-sem-cbsem",
      name: "Recursive common-factor SEM",
      constructs: ["x", "m1", "m2", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: id === "m1" ? ["m11", "m12"] : id === "m2" ? ["m21", "m22"] : [`${id}1`, `${id}2`],
      })),
      paths: [
        { source: "x", target: "m1" },
        { source: "m1", target: "y" },
        { source: "x", target: "m2" },
        { source: "m2", target: "y" },
      ],
    }, "cbsem_common_factor");
    model.data_binding = {
      kind: "raw",
      dataset_id: dataset.id,
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    };
    const pointConfig = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
    const pointRecipe = buildGeneralSemCbsemRecipeV3({
      recipeId: RECIPE_ID,
      createdAt: "2026-08-21T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: DIGEST_C,
      config: pointConfig,
      engine: defaultGeneralSemPlsEngineOptionsV1(),
    });
    expect(pointRecipe.settings).toMatchObject({
      method: "cbsem",
      preprocessing: "unstandardized",
      bootstrap_samples: 0,
    });
    expect(pointRecipe.method_config).toStrictEqual({
      kind: "cbsem",
      model_type: "sem",
      estimator: "ml",
      input: "raw",
      mean_structure: false,
      bootstrap_samples: 0,
    });
    expect(pointRecipe.metadata.execution_surface).toBe("native_general_sem_cbsem_labs_v1");

    const bootstrapEngine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 500,
      confidenceLevel: 0.95,
    };
    const bootstrapConfig = generalSemConfigFromEngineV1(bootstrapEngine);
    const bootstrapRecipe = buildGeneralSemCbsemRecipeV3({
      recipeId: RECIPE_ID,
      createdAt: "2026-08-21T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: DIGEST_C,
      config: bootstrapConfig,
      engine: bootstrapEngine,
    });
    expect(bootstrapRecipe.method_config).toMatchObject({
      kind: "cbsem",
      model_type: "sem",
      bootstrap_samples: 500,
      bootstrap_v2: {
        algorithm: "case_resampling_full_ml",
        interval: "percentile_type7",
      },
    });

    const decision = {
      schema_version: 1 as const,
      status: "experimental" as const,
      status_label: "Experimental" as const,
      estimator_id: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
      capability_cells: [
        GENERAL_SEM_CBSEM_POINT_CAPABILITY_CELL_V1,
        GENERAL_SEM_CBSEM_BOOTSTRAP_CAPABILITY_CELL_V1,
      ],
      diagnostics: [],
      evidence: [
        GENERAL_SEM_CBSEM_POINT_CAPABILITY_CELL_V1,
        GENERAL_SEM_CBSEM_BOOTSTRAP_CAPABILITY_CELL_V1,
      ].map((cell) => ({
        evidence_id: `capability_registry_v2:${cell.capability_id}:${cell.cell_id}:${cell.capability_version}`,
        description: "Exact Registry-owned Experimental Labs cell.",
      })),
      summary: "Exact CB-SEM recursive bootstrap is available in Experimental Labs.",
      explanation: "The resident RecipeV4 and exact capability cells are unchanged.",
    };
    const execution = selectGeneralSemCbsemExecutionCapabilityV1({
      config: bootstrapConfig,
      decision,
    });
    expect(execution.capabilityCell).toStrictEqual(GENERAL_SEM_CBSEM_BOOTSTRAP_CAPABILITY_CELL_V1);
    expect(generalSemCbsemJobRequestFromReceiptV1(receipt(), bootstrapConfig, decision))
      .toMatchObject({
        surface: "internal_labs",
        experimentalLabsEnabled: true,
        capabilityCell: GENERAL_SEM_CBSEM_BOOTSTRAP_CAPABILITY_CELL_V1,
      });
  });

  it("selects the exact point and multiple-mediation bootstrap cells from the frozen inference config", () => {
    const mediationModel = multipleMediationModel();
    const point = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
    const bootstrapEngine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 1_000,
      seed: 20260819,
      workers: 4,
      confidenceLevel: 0.9,
    };
    const bootstrap = generalSemConfigFromEngineV1(bootstrapEngine);
    const pointDecision = preflightGeneralSemPlsV1(mediationModel, point);
    const bootstrapDecision = preflightGeneralSemPlsV1(mediationModel, bootstrap);

    expect(point.inference).toEqual({ kind: "none" });
    expect(generalSemJobRequestFromReceiptV1(
      receipt(), mediationModel, point, pointDecision,
    ).capabilityCell)
      .toStrictEqual(GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1);
    expect(bootstrap.inference).toStrictEqual({
      kind: "case_bootstrap",
      resamples: 1_000,
      seed: 20260819,
      confidence_level: 0.9,
      interval: "percentile",
      tail: "two_sided",
    });
    expect(generalSemJobRequestFromReceiptV1(
      receipt(), mediationModel, bootstrap, bootstrapDecision,
    ).capabilityCell)
      .toStrictEqual(GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1);
    expect(generalSemJobRequestFromReceiptV1(
      receipt(), mediationModel, bootstrap, bootstrapDecision, "f".repeat(64),
    ).expectedArchiveSha256)
      .toBe("f".repeat(64));
  });

  it.each(["same_focal", "different_focal"] as const)(
    "routes simultaneous two-way moderation with %s paths through exact point and supplemental bootstrap cells",
    (layout) => {
      const model = multipleModerationModel(layout);
      const config = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
      const decision = preflightGeneralSemPlsV1(model, config);
      const selected = selectGeneralSemPlsExecutionCapabilityV1({ model, config, decision });
      const bootstrapConfig = generalSemConfigFromEngineV1({
        ...defaultGeneralSemPlsEngineOptionsV1(),
        inference: "percentile_case_bootstrap",
      });
      const bootstrapDecision = preflightGeneralSemPlsV1(model, bootstrapConfig);
      const bootstrapSelected = selectGeneralSemPlsExecutionCapabilityV1({
        model,
        config: bootstrapConfig,
        decision: bootstrapDecision,
      });

      expect(selected).toMatchObject({
        kind: "multiple_two_way_moderation_point",
        capabilityCell: GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
        interactionIds: layout === "same_focal"
          ? ["interaction:x:w", "interaction:x:z"]
          : ["interaction:x:w", "interaction:z:w"],
      });
      expect(selected.focalRelationIds).toHaveLength(layout === "same_focal" ? 1 : 2);
      expect(generalSemJobRequestFromReceiptV1(
        receipt(), model, config, decision,
      ).capabilityCell).toStrictEqual(GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1);
      expect(bootstrapSelected).toMatchObject({
        kind: "multiple_two_way_moderation_bootstrap",
        capabilityCell: GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
        interactionIds: selected.interactionIds,
        focalRelationIds: selected.focalRelationIds,
      });
      expect(generalSemJobRequestFromReceiptV1(
        receipt(), model, bootstrapConfig, bootstrapDecision,
      ).capabilityCell).toStrictEqual(GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1);
    },
  );

  it("blocks stale native capability authority before job start without retiring interaction bootstrap", () => {
    const model = multipleModerationModel("same_focal");
    const point = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
    const staleMediationDecision = preflightGeneralSemPlsV1(multipleMediationModel(), point);
    expect(() => generalSemJobRequestFromReceiptV1(
      receipt(), model, point, staleMediationDecision,
    )).toThrowError(expect.objectContaining({
      code: "general_sem.capability.native_preflight_cell_mismatch",
    }));

    const bootstrap = generalSemConfigFromEngineV1({
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap",
    });
    const bootstrapDecision = preflightGeneralSemPlsV1(model, bootstrap);
    expect(generalSemJobRequestFromReceiptV1(
      receipt(), model, bootstrap, bootstrapDecision,
    ).capabilityCell).toStrictEqual(GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1);
  });

  it.each([
    "mediation_point",
    "mediation_bootstrap",
    "multiple_two_way_moderation_point",
  ] as const)("reconciles a completed %s result with its exact native execution authority", (kind) => {
    const fixture = completedExecutionFixture(kind);
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution)).not.toThrow();
  });

  it("keeps the mediation-bootstrap request cell distinct from the compiled analytical point cell", () => {
    const fixture = completedExecutionFixture("mediation_bootstrap");
    expect(fixture.execution.capabilityCell).toStrictEqual(GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1);
    expect((fixture.completed.analyticalResult as Record<string, unknown>).capability_cell)
      .toStrictEqual(GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1);
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution)).not.toThrow();

    (fixture.completed.analyticalResult as Record<string, unknown>).capability_cell =
      GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1;
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));
  });

  it("strictly parses and reconciles the gamma-only moderation bootstrap request, raw receipt, and canonical result", () => {
    const fixture = moderationBootstrapCompletedFixture();

    expect(validateCanonicalResultDocumentV2(fixture.completed.canonicalDocument))
      .toEqual({ passed: true, errors: [] });
    expect(parseGeneralSemPlsCompletedResultV1(fixture.completed)).toStrictEqual(fixture.completed);
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution))
      .not.toThrow();
    expect(fixture.execution.capabilityCell)
      .toStrictEqual(GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1);
    expect((fixture.completed.analyticalResult as Record<string, unknown>).capability_cell)
      .toStrictEqual(GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1);
    expect(fixture.completed.canonicalDocument.provenance).toMatchObject({
      capability_cell: GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
      method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    });

    fixture.completed.canonicalDocument.provenance.method_version =
      GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1;
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));
  });

  it.each([
    "generated_product_column_id",
    "stage_one_model_scientific_sha256",
    "product_scale_version",
    "method_version",
  ] as const)("rejects raw and canonical gamma identity tampering in %s", (field) => {
    const rawTamper = moderationBootstrapCompletedFixture();
    const raw = (rawTamper.completed.analyticalResult as Record<string, unknown>)
      .moderation_bootstrap_inference as Record<string, unknown>;
    const row = (raw.interaction_gammas as Array<Record<string, unknown>>)[0]!;
    const target = row.target as Record<string, unknown>;
    target[field] = field === "stage_one_model_scientific_sha256"
      ? "9".repeat(64)
      : "tampered:v1";
    expect(() => parseGeneralSemPlsCompletedResultV1(rawTamper.completed))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));

    const canonicalTamper = moderationBootstrapCompletedFixture();
    const effect = canonicalTamper.completed.canonicalDocument.general_sem_results!
      .interaction_effects![0] as unknown as Record<string, unknown>;
    effect[field] = field === "stage_one_model_scientific_sha256"
      ? "9".repeat(64)
      : "tampered:v1";
    expect(validateCanonicalResultDocumentV2(canonicalTamper.completed.canonicalDocument).passed)
      .toBe(false);
  });

  it("rejects raw gamma schema drift, inference on point-only moderation surfaces, and plot confidence bands", () => {
    const rawExtra = moderationBootstrapCompletedFixture();
    const raw = (rawExtra.completed.analyticalResult as Record<string, unknown>)
      .moderation_bootstrap_inference as Record<string, unknown>;
    const target = ((raw.interaction_gammas as Array<Record<string, unknown>>)[0]!
      .target as Record<string, unknown>);
    target.unexpected = true;
    expect(() => parseGeneralSemPlsCompletedResultV1(rawExtra.completed))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));

    for (const mutate of [
      (document: CanonicalResultDocumentV2) => {
        document.general_sem_results!.interaction_effects![0]!.standardized_product_coefficient = {
          estimate: 0.2,
          bootstrap_mean: 0.21,
          bootstrap_bias: 0.01,
          standard_error: 0.1,
          lower: 0,
          upper: 0.4,
          p_value: 0.3,
          bootstrap_usable_replicates: 9,
          bootstrap_two_sided_exceedances: 2,
        };
      },
      (document: CanonicalResultDocumentV2) => {
        document.general_sem_results!.joint_stage_structural_coefficients![0]!.estimate = {
          estimate: 0.3,
          bootstrap_mean: 0.31,
          bootstrap_bias: 0.01,
          standard_error: 0.1,
          lower: 0.1,
          upper: 0.5,
          p_value: 0.3,
          bootstrap_usable_replicates: 9,
          bootstrap_two_sided_exceedances: 2,
        };
      },
      (document: CanonicalResultDocumentV2) => {
        document.general_sem_results!.conditional_effects![0]!.value = {
          estimate: 0.1,
          bootstrap_mean: 0.11,
          bootstrap_bias: 0.01,
          standard_error: 0.1,
          lower: -0.1,
          upper: 0.3,
          p_value: 0.3,
          bootstrap_usable_replicates: 9,
          bootstrap_two_sided_exceedances: 2,
        };
      },
      (document: CanonicalResultDocumentV2) => {
        document.general_sem_results!.interaction_plots![0]!.series[0]!.points[0]!.lower = -1;
        document.general_sem_results!.interaction_plots![0]!.series[0]!.points[0]!.upper = 1;
      },
    ]) {
      const document = moderationBootstrapCanonicalDocument();
      mutate(document);
      expect(validateCanonicalResultDocumentV2(document).passed).toBe(false);
    }
  });

  it("rejects completed-result capability, method, engine, digest, inventory, and payload-shape relabeling", () => {
    const scenarios: Array<(completed: GeneralSemPlsCompletedResultV1) => void> = [
      (completed) => { completed.canonicalDocument.provenance.method_version = "general_sem_effects_v1"; },
      (completed) => { completed.canonicalDocument.provenance.engine_version = "wrong_adapter_v1"; },
      (completed) => { (completed.analyticalResult as Record<string, unknown>).capability_cell = GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1; },
      (completed) => { (completed.analyticalResult as Record<string, unknown>).recipe_analytical_sha256 = "9".repeat(64); },
      (completed) => { completed.canonicalDocument.capability_cells = [...(completed.canonicalDocument.capability_cells ?? []), GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1]; },
      (completed) => { delete (completed.analyticalResult as Record<string, unknown>).interaction_point_estimation; },
      (completed) => { (completed.analyticalResult as Record<string, unknown>).bootstrap_inference = {}; },
    ];
    for (const mutate of scenarios) {
      const fixture = completedExecutionFixture("multiple_two_way_moderation_point");
      mutate(fixture.completed);
      expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution))
        .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));
    }
  });

  it.each([
    {
      name: "missing analytical interaction",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => { rawCompletedInteractionCoefficients(completed).pop(); },
    },
    {
      name: "extra analytical interaction",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => {
        const coefficients = rawCompletedInteractionCoefficients(completed);
        coefficients.push({ ...coefficients[0]!, interaction_id: "interaction:extra" });
      },
    },
    {
      name: "swapped analytical interaction inventory",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => {
        const coefficient = rawCompletedInteractionCoefficients(completed)[0]!;
        coefficient.interaction_id = coefficient.focal_relation_id;
      },
    },
    {
      name: "analytical focal-relation substitution",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => {
        rawCompletedInteractionCoefficients(completed)[0]!.focal_relation_id = "relation:foreign";
      },
    },
    {
      name: "missing canonical interaction",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => { canonicalCompletedInteractionEffects(completed).shift(); },
    },
    {
      name: "extra canonical interaction",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => {
        const effects = canonicalCompletedInteractionEffects(completed);
        effects.push({ ...structuredClone(effects[0]!), interaction_id: "interaction:extra" });
      },
    },
    {
      name: "swapped canonical interaction inventory",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => {
        const effect = canonicalCompletedInteractionEffects(completed)[0]!;
        effect.interaction_id = effect.focal_relation_id;
      },
    },
    {
      name: "canonical focal-relation substitution",
      mutate: (completed: GeneralSemPlsCompletedResultV1) => {
        canonicalCompletedInteractionEffects(completed)[0]!.focal_relation_id = "relation:foreign";
      },
    },
  ])("rejects $name against the current moderation execution inventory", ({ mutate }) => {
    const fixture = completedExecutionFixture("multiple_two_way_moderation_point");
    mutate(fixture.completed);
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, fixture.execution))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));
  });

  it("rejects noncanonical or duplicated current moderation execution inventories", () => {
    const fixture = completedExecutionFixture("multiple_two_way_moderation_point");
    const reversed = {
      ...fixture.execution,
      interactionIds: [...fixture.execution.interactionIds].reverse(),
    };
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, reversed))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));

    const duplicated = {
      ...fixture.execution,
      focalRelationIds: [...fixture.execution.focalRelationIds, fixture.execution.focalRelationIds[0]!],
    };
    expect(() => validateGeneralSemPlsCompletedExecutionV1(fixture.completed, duplicated))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));
  });

  it.each(["mediation_point", "mediation_bootstrap"] as const)(
    "keeps %s free of execution and completed interaction payloads",
    (kind) => {
      const executionFixture = completedExecutionFixture(kind);
      const contaminatedExecution = {
        ...executionFixture.execution,
        interactionIds: ["interaction:foreign"],
        focalRelationIds: ["relation:foreign"],
      };
      expect(() => validateGeneralSemPlsCompletedExecutionV1(
        executionFixture.completed,
        contaminatedExecution,
      )).toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));

      const analyticalFixture = completedExecutionFixture(kind);
      (analyticalFixture.completed.analyticalResult as Record<string, unknown>).interaction_point_estimation = {
        interaction_coefficients: [],
      };
      expect(() => validateGeneralSemPlsCompletedExecutionV1(
        analyticalFixture.completed,
        analyticalFixture.execution,
      )).toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));

      const canonicalFixture = completedExecutionFixture(kind);
      canonicalFixture.completed.canonicalDocument.general_sem_results = {
        schema_version: 1,
        interaction_effects: moderationCanonicalDocument().general_sem_results!.interaction_effects!,
      };
      expect(() => validateGeneralSemPlsCompletedExecutionV1(
        canonicalFixture.completed,
        canonicalFixture.execution,
      )).toThrowError(expect.objectContaining({ code: "general_sem.wire.completed_execution_mismatch" }));
    },
  );

  it("rejects extra or non-canonically ordered native capability cells", () => {
    const model = multipleMediationModel();
    const point = generalSemConfigFromEngineV1(defaultGeneralSemPlsEngineOptionsV1());
    const basePointDecision = preflightGeneralSemPlsV1(model, point);
    const pointDecision = {
      ...basePointDecision,
      capability_cells: [
        ...basePointDecision.capability_cells,
        GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
      ],
    };
    expect(() => generalSemJobRequestFromReceiptV1(
      receipt(), model, point, pointDecision,
    )).toThrowError(expect.objectContaining({
      code: "general_sem.capability.native_preflight_cell_mismatch",
    }));

    const bootstrap = generalSemConfigFromEngineV1({
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap",
    });
    const baseBootstrapDecision = preflightGeneralSemPlsV1(model, bootstrap);
    const bootstrapDecision = {
      ...baseBootstrapDecision,
      capability_cells: [...baseBootstrapDecision.capability_cells].reverse(),
    };
    expect(() => generalSemJobRequestFromReceiptV1(
      receipt(), model, bootstrap, bootstrapDecision,
    )).toThrowError(expect.objectContaining({
      code: "general_sem.capability.native_preflight_cell_mismatch",
    }));
  });

  it("binds one resident raw dataset and emits a project-model Recipe-v4 authority without case rows", () => {
    const dataset = rawDataset();
    const source = multipleMediationModel();
    const authoredMissingMarker = "-999";
    const model = bindGeneralSemPlsModelToDatasetV1({
      ...source,
      variables: source.variables.map((variable) => variable.kind === "observed" && variable.source_column === "x1"
        ? { ...variable, missing_markers: [authoredMissingMarker] }
        : variable),
    }, dataset);
    expect(model.variables.find((variable) => variable.kind === "observed" && variable.source_column === "x1"))
      .toMatchObject({ missing_markers: [authoredMissingMarker] });
    const engine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 500,
      workers: 2,
    };
    const config = generalSemConfigFromEngineV1(engine);
    const recipe = buildGeneralSemRecipeV1({
      recipeId: RECIPE_ID,
      createdAt: "2026-08-19T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: DIGEST_C,
      config,
      engine,
    });

    expect(model.data_binding).toStrictEqual({
      kind: "raw",
      dataset_id: dataset.id,
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    });
    expect(recipe).toMatchObject({
      schema_version: 4,
      id: RECIPE_ID,
      dataset_fingerprint: DIGEST_B,
      model_binding: {
        kind: "project_sem_model_v4_reference",
        model_id: model.id,
        scientific_sha256: DIGEST_C,
      },
      settings: {
        method: "pls_pm",
        bootstrap_samples: 500,
        workers: 2,
        missing_data: "listwise_deletion",
      },
      method_config: { kind: "pls_algorithm" },
      general_sem_config: config,
      metadata: {
        execution_surface: "native_general_sem_pls_labs_v1",
        general_sem_generation: "general_sem_v1",
      },
    });
    expect(JSON.stringify(recipe)).not.toContain("\"rows\"");
  });

  it("rehydrates exact resident config and native recipe-document identity after restart", () => {
    const dataset = rawDataset();
    const model = bindGeneralSemPlsModelToDatasetV1(multipleMediationModel(), dataset);
    const engine = {
      ...defaultGeneralSemPlsEngineOptionsV1(),
      inference: "percentile_case_bootstrap" as const,
      bootstrapSamples: 777,
      seed: 20260819,
      workers: 3,
      confidenceLevel: 0.9,
      maxMaterializedSpecificPaths: 321,
    };
    const config = generalSemConfigFromEngineV1(engine, [{
      kind: "total_indirect",
      estimand_id: "effect:x:y",
      source_id: "x",
      target_id: "y",
    }]);
    const recipe = buildGeneralSemRecipeV1({
      recipeId: RECIPE_ID,
      createdAt: "2026-08-19T00:00:00Z",
      dataset,
      model,
      nativeScientificSha256: DIGEST_C,
      config,
      engine,
    });
    const nativeRecipeDocumentSha256 = "d".repeat(64);
    const restored = rehydrateGeneralSemExecutionAuthorityV1({
      archivePath: "D:\\General-Sem.qpls",
      archiveSha256: DIGEST_A,
      archiveBytes: 4096,
      project: {
        project_id: PROJECT_ID,
        name: "General SEM calculation",
        created_at: "2026-08-19T00:00:00Z",
      },
      generalSemExecutionAuthority: {
        schemaVersion: 1,
        projectId: PROJECT_ID,
        datasetId: dataset.id,
        datasetFingerprint: dataset.fingerprint!,
        modelId: model.id,
        modelScientificSha256: DIGEST_C,
        recipeId: RECIPE_ID,
        recipeDocumentSha256: nativeRecipeDocumentSha256,
        recipe,
      },
    } as InternalProjectArchiveV6ReadSnapshotV1);

    expect(restored.config).toStrictEqual(config);
    expect(restored.engine).toStrictEqual(engine);
    expect(restored.receipt.residentRecipeDocumentSha256).toBe(nativeRecipeDocumentSha256);
  });

  it("fails local preflight when resident observed-column descriptors are missing or noncontinuous", () => {
    const dataset = rawDataset();
    const model = bindGeneralSemPlsModelToDatasetV1(multipleMediationModel(), dataset);
    const engine = defaultGeneralSemPlsEngineOptionsV1();
    const config = generalSemConfigFromEngineV1(engine);
    const run = (candidateDataset: Dataset | null) => preflightGeneralSemWorkspaceV1({
      experimentalLabsEnabled: true,
      sourceProjectId: PROJECT_ID,
      dataset: candidateDataset,
      model,
      config,
      engine,
    });

    expect(run(dataset).ready).toBe(true);

    const cases: Array<[Dataset | null, string]> = [
      [null, "general_sem.dataset.required"],
      [{ ...dataset, columnMetadata: undefined }, "general_sem.dataset.continuous_numeric_required"],
      [{
        ...dataset,
        columns: dataset.columns.filter((column) => column !== "x1"),
      }, "general_sem.dataset.observed_column_missing"],
      [{
        ...dataset,
        columnMetadata: dataset.columnMetadata?.map((column) => column.name === "x1"
          ? { ...column, scale_type: "ordinal" as const }
          : column),
      }, "general_sem.dataset.continuous_numeric_required"],
    ];

    for (const [candidateDataset, expectedCode] of cases) {
      const decision = run(candidateDataset);
      expect(decision.ready).toBe(false);
      expect(decision.issues.map((item) => item.code)).toContain(expectedCode);
    }
  });

  it("fails closed on model digest and resident-dataset authority mismatches", () => {
    const dataset = rawDataset();
    const model = bindGeneralSemPlsModelToDatasetV1(multipleMediationModel(), dataset);
    const engine = defaultGeneralSemPlsEngineOptionsV1();
    const config = generalSemConfigFromEngineV1(engine);
    const common = {
      recipeId: RECIPE_ID,
      createdAt: "2026-08-19T00:00:00Z",
      dataset,
      model,
      config,
      engine,
    };

    expect(() => buildGeneralSemRecipeV1({ ...common, nativeScientificSha256: "ABC" }))
      .toThrowError(expect.objectContaining({ code: "general_sem.model.native_digest_invalid" }));
    expect(() => buildGeneralSemRecipeV1({
      ...common,
      dataset: { ...dataset, id: "dataset:tampered" },
      nativeScientificSha256: DIGEST_C,
    })).toThrowError(expect.objectContaining({ code: "general_sem.dataset.binding_mismatch" }));
  });

  it("parses strict schema-6 receipts and rejects digest or strict-reopen tampering", () => {
    const wire = { status: "ok", value: { schemaVersion: 1, receipt: receipt() } };
    expect(parseGeneralSemProjectBootstrapOutcomeV1(wire)).toStrictEqual(wire);

    const digestTamper = structuredClone(wire);
    digestTamper.value.receipt.destinationArchiveSha256 = DIGEST_A.toUpperCase();
    expect(() => parseGeneralSemProjectBootstrapOutcomeV1(digestTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.digest_invalid" }));

    const reopenTamper = structuredClone(wire) as unknown as {
      status: string;
      value: { schemaVersion: number; receipt: Record<string, unknown> };
    };
    reopenTamper.value.receipt.strictReopenValidated = false;
    expect(() => parseGeneralSemProjectBootstrapOutcomeV1(reopenTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.bootstrap_contract_invalid" }));
  });

  it("rejects malformed job snapshots and canonical/archive identity tampering", () => {
    expect(parseGeneralSemPlsJobSnapshotV1(snapshot("running", 1)))
      .toStrictEqual(snapshot("running", 1));
    expect(() => parseGeneralSemPlsJobSnapshotV1({ ...snapshot("running", 1), state: "publishing" }))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.job_snapshot_invalid" }));
    const invalidFailureStage = structuredClone(snapshot("failed", 1)) as unknown as {
      failure: Record<string, unknown>;
    };
    invalidFailureStage.failure.stage = "publishing";
    expect(() => parseGeneralSemPlsJobSnapshotV1(invalidFailureStage)).toThrow();
    expect(() => parseGeneralSemPlsJobSnapshotV1({
      ...snapshot("running", 1),
      completedUnits: 4,
      totalUnits: 3,
    })).toThrow();

    expect(parseGeneralSemPlsCompletedResultV1(completedResult()))
      .toStrictEqual(completedResult());
    expect(() => parseGeneralSemPlsCompletedResultV1(completedResult("00000000-0000-4000-8000-000000000099")))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.result_authority_mismatch" }));
    const datasetFingerprintTamper = structuredClone(completedResult());
    datasetFingerprintTamper.canonicalDocument.provenance.dataset_fingerprint = "f".repeat(64);
    expect(() => parseGeneralSemPlsCompletedResultV1(datasetFingerprintTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.result_authority_mismatch" }));
    const modelDigestTamper = structuredClone(completedResult());
    modelDigestTamper.canonicalDocument.provenance.model_digest = "f".repeat(64);
    expect(() => parseGeneralSemPlsCompletedResultV1(modelDigestTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.result_authority_mismatch" }));
    const archiveDigestTamper = structuredClone(completedResult()) as unknown as Record<string, unknown>;
    (archiveDigestTamper.archiveIdentity as Record<string, unknown>).archiveSha256 = "not-a-digest";
    expect(() => parseGeneralSemPlsCompletedResultV1(archiveDigestTamper))
      .toThrowError(expect.objectContaining({ code: "general_sem.wire.digest_invalid" }));
  });

  it("accepts the Rust-shaped exact-cell bootstrap result and rejects generic resampling ownership", () => {
    const canonical = rustShapedBootstrapCanonicalDocument();
    expect(validateCanonicalResultDocumentV2(canonical)).toEqual({ passed: true, errors: [] });
    expect(parseGeneralSemPlsCompletedResultV1({
      ...completedResult(),
      canonicalDocument: canonical,
    })).toMatchObject({ canonicalDocument: canonical });

    const genericCellTamper = structuredClone(canonical);
    genericCellTamper.general_sem_results!.inference_receipt!.capability_cell = {
      registry_schema_version: 2,
      capability_id: "smartpls.pls_bootstrapping",
      cell_id: "qpls3.inference.bootstrap",
      capability_version: "indexed_resampling_v4",
    };
    genericCellTamper.capability_cells = [
      GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
      GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
      {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
        capability_version: "pls_pm_v1",
      },
      genericCellTamper.general_sem_results!.inference_receipt!.capability_cell,
    ];
    const validation = validateCanonicalResultDocumentV2(genericCellTamper);
    expect(validation.passed).toBe(false);
    expect(validation.errors.join("\n")).toContain(
      "must equal the exact General SEM multiple-mediation or multiple two-way moderation full-model case-bootstrap option cell",
    );
    expect(() => parseGeneralSemPlsCompletedResultV1({
      ...completedResult(),
      canonicalDocument: genericCellTamper,
    })).toThrowError(expect.objectContaining({ code: "general_sem.wire.canonical_invalid" }));
  });

  it("preserves canonical interaction effects, conditional slopes, and plot points through strict reopen", async () => {
    const canonical = moderationCanonicalDocument();
    expect(validateCanonicalResultDocumentV2(canonical)).toEqual({ passed: true, errors: [] });
    const completed = parseGeneralSemPlsCompletedResultV1({
      ...completedResult(),
      canonicalDocument: canonical,
    });
    const canonicalJson = canonicalResultDocumentJson(canonical);
    const entry = {
      documentId: canonical.document_id,
      runId: canonical.provenance.run_id,
      canonicalDocumentSha256: "9".repeat(64),
      immutable: true as const,
      canonicalDocumentJson: canonicalJson,
      canonicalDocument: structuredClone(canonical),
    };
    const read = vi.fn().mockResolvedValue({
      status: "ok" as const,
      value: {
        schemaVersion: 1 as const,
        projectId: PROJECT_ID,
        archivePath: completed.archiveIdentity.archivePath,
        sourceDocumentSha256: "8".repeat(64),
        canonicalResultDocumentCount: 1,
        documents: [entry],
        sourceRecheckedUnchanged: true as const,
      },
    });

    const reopened = await reopenGeneralSemResultV1(completed, "8".repeat(64), read);

    expect(reopened.entry?.canonicalDocument).toStrictEqual(canonical);
    expect(reopened.entry?.canonicalDocument.general_sem_results).toMatchObject({
      interaction_effects: [{ interaction_id: "interaction:x:w" }],
      conditional_effects: [{ probe_value_index: 0 }, { probe_value_index: 1 }, { probe_value_index: 2 }],
      interaction_plots: [{
        interaction_id: "interaction:x:w",
        series: [
          { points: [{ focal_value: -1 }, { focal_value: 0 }, { focal_value: 1 }] },
          { points: [{ focal_value: -1 }, { focal_value: 0 }, { focal_value: 1 }] },
          { points: [{ focal_value: -1 }, { focal_value: 0 }, { focal_value: 1 }] },
        ],
      }],
    });
    expect(read).toHaveBeenCalledWith(expect.objectContaining({
      expectedSourceSha256: "8".repeat(64),
    }));
  });

  it("preserves the gamma inference receipt losslessly through schema-6 append and strict reopen", async () => {
    const fixture = moderationBootstrapCompletedFixture();
    const completed = parseGeneralSemPlsCompletedResultV1(fixture.completed);
    const canonicalJson = canonicalResultDocumentJson(completed.canonicalDocument);
    const append = vi.fn().mockResolvedValue({ status: "ok" });
    await appendGeneralSemResultV1(completed, append);
    expect(append).toHaveBeenCalledWith(expect.objectContaining({
      capabilityCell: GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
      canonicalDocument: completed.canonicalDocument,
    }));

    const read = vi.fn().mockResolvedValue({
      status: "ok" as const,
      value: {
        schemaVersion: 1 as const,
        projectId: PROJECT_ID,
        archivePath: completed.archiveIdentity.archivePath,
        sourceDocumentSha256: "8".repeat(64),
        canonicalResultDocumentCount: 1,
        documents: [{
          documentId: completed.canonicalDocument.document_id,
          runId: completed.canonicalDocument.provenance.run_id,
          canonicalDocumentSha256: "9".repeat(64),
          immutable: true as const,
          canonicalDocumentJson: canonicalJson,
          canonicalDocument: structuredClone(completed.canonicalDocument),
        }],
        sourceRecheckedUnchanged: true as const,
      },
    });
    const reopened = await reopenGeneralSemResultV1(completed, "8".repeat(64), read);
    expect(read).toHaveBeenCalledWith(expect.objectContaining({
      capabilityCell: GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
    }));
    expect(reopened.entry?.canonicalDocument.general_sem_results?.inference_receipt)
      .toStrictEqual(completed.canonicalDocument.general_sem_results?.inference_receipt);
    expect(reopened.entry?.canonicalDocument.general_sem_results?.interaction_effects?.[0]
      .scientific_rescaled_gamma)
      .toStrictEqual(completed.canonicalDocument.general_sem_results?.interaction_effects?.[0]
        .scientific_rescaled_gamma);
  });

  it("stops immediately when monitoring is cancelled and never requests a result", async () => {
    const controller = new AbortController();
    controller.abort();
    const getStatus = vi.fn();
    const getResult = vi.fn();
    const wait = vi.fn();

    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("queued", 0),
      getStatus,
      getResult,
      wait,
      signal: controller.signal,
    })).resolves.toEqual({ status: "aborted", snapshot: snapshot("queued", 0) });
    expect(wait).not.toHaveBeenCalled();
    expect(getStatus).not.toHaveBeenCalled();
    expect(getResult).not.toHaveBeenCalled();
  });

  it("returns failed or cancelled terminal snapshots without publishing a partial result", async () => {
    const failed = snapshot("failed", 1);
    const getResult = vi.fn();
    const onSnapshot = vi.fn();
    const outcome = await monitorGeneralSemPlsJobV1({
      initial: snapshot("running", 0),
      getStatus: vi.fn().mockResolvedValue(failed),
      getResult,
      onSnapshot,
      wait: async () => undefined,
    });

    expect(outcome).toEqual({ status: "terminal_without_result", snapshot: failed });
    expect(onSnapshot.mock.calls.map(([value]) => value.state)).toEqual(["running", "failed"]);
    expect(getResult).not.toHaveBeenCalled();
  });

  it("reads a completed result exactly once after the terminal completed snapshot", async () => {
    const running = snapshot("running", 1);
    const completedSnapshot = snapshot("completed", 3);
    const completed = completedResult();
    const statuses = [running, completedSnapshot];
    const getStatus = vi.fn().mockImplementation(async () => statuses.shift());
    const getResult = vi.fn().mockResolvedValue(completed);

    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("queued", 0),
      getStatus,
      getResult,
      wait: async () => undefined,
    })).resolves.toEqual({ status: "completed", snapshot: completedSnapshot, completed });
    expect(getStatus).toHaveBeenCalledTimes(2);
    expect(getResult).toHaveBeenCalledOnce();
    expect(getResult).toHaveBeenCalledWith("job:general-sem");
  });

  it("propagates post-start status and one-shot result retrieval failures to the lifecycle owner", async () => {
    const statusFailure = new Error("status transport unavailable");
    const statusResult = vi.fn();
    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("running", 0),
      getStatus: vi.fn().mockRejectedValue(statusFailure),
      getResult: statusResult,
      wait: async () => undefined,
    })).rejects.toBe(statusFailure);
    expect(statusResult).not.toHaveBeenCalled();

    const resultFailure = new Error("one-shot result parser rejected");
    const getResult = vi.fn().mockRejectedValue(resultFailure);
    await expect(monitorGeneralSemPlsJobV1({
      initial: snapshot("completed", 3),
      getStatus: vi.fn(),
      getResult,
    })).rejects.toBe(resultFailure);
    expect(getResult).toHaveBeenCalledOnce();
    expect(getResult).toHaveBeenCalledWith("job:general-sem");
  });
});
