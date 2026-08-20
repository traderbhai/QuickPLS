import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./internalProjectArchiveV6Read";
import {
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1,
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_TARGET_INVENTORY_V1,
  type GeneralSemModeratedMediationSelectionReadyV1,
} from "./generalSemModeratedMediationAuthoringV1";
import { specificDirectedPathIdentityV1 } from "./generalSemCapabilityPreflightV1";
import type {
  CompiledPlsTwoWayModeratedMediationTargetV1,
  GeneralSemExecutionAuthorityRevisionLineageV2,
  GeneralSemExecutionAuthorityRevisionReceiptV2,
  InternalGeneralSemModeratedMediationRevisionRequestV2,
} from "./internalGeneralSemModeratedMediationRevisionV2";
import type { InternalGeneralSemModeratedMediationRevisionTransactionV2 } from "../services/internalGeneralSemModeratedMediationRevisionV2Service";

export const sha = (value: string) => value.repeat(64);
export const sourceProjectId = "10000000-0000-4000-8000-000000000001";
export const sourceRecipeId = "10000000-0000-4000-8000-000000000002";
export const projectId = "10000000-0000-4000-8000-000000000003";
export const recipeId = "10000000-0000-4000-8000-000000000004";
export const datasetId = "10000000-0000-4000-8000-000000000005";
export const relationIds = ["relation:x:m", "relation:m:y"] as const;
export const estimandId = specificDirectedPathIdentityV1(relationIds);

const baseCell = GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1.find(
  (cell) => cell.capability_id === "smartpls.pls_algorithm",
)!;
const moderationCell = GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1.find(
  (cell) => cell.capability_id === "smartpls.moderation",
)!;
const dependencies = [moderationCell, baseCell];

export function selectionV1(): GeneralSemModeratedMediationSelectionReadyV1 {
  const selectedPath = {
    pathId: estimandId,
    estimandId,
    orderedRelationIds: relationIds,
    xId: "construct:x",
    xLabel: "X",
    mediatorId: "construct:m",
    mediatorLabel: "M",
    yId: "construct:y",
    yLabel: "Y",
    moderatorId: "construct:w",
    moderatorLabel: "W",
    interactionId: "interaction:x:w:m",
    moderatedStage: "first_stage" as const,
    moderatedRelationId: relationIds[0],
    otherStageRelationId: relationIds[1],
  };
  return {
    status: "ready",
    candidates: [selectedPath],
    selectedPathId: estimandId,
    selectedPath,
    autoSelected: true,
    targetInventory: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_TARGET_INVENTORY_V1,
    supplementalCapabilityCell: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
    capabilityDependencies: GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1,
    revisedConfig: {
      schema_version: 1,
      requested_effect_estimands: [{
        kind: "specific_path",
        estimand_id: estimandId,
        ordered_relation_ids: [...relationIds],
      }],
      conditional_effect_probes: [],
      inference: {
        kind: "case_bootstrap",
        resamples: 500,
        seed: 20260820,
        confidence_level: 0.95,
        interval: "percentile",
        tail: "two_sided",
      },
      output_policy: {
        max_materialized_specific_paths: 10_000,
        lazy_specific_path_materialization: false,
        when_specific_path_limit_exceeded: "error",
      },
    },
    issues: [],
  };
}

export function requestV2(): InternalGeneralSemModeratedMediationRevisionRequestV2 {
  return {
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    sourceArchivePath: "D:\\source.qpls",
    expectedSourceArchiveSha256: sha("a"),
    destinationArchivePath: "D:\\revision-v2.qpls",
    revision: {
      source: {
        projectId: sourceProjectId,
        modelId: "model:source",
        modelDocumentSha256: sha("b"),
        modelScientificSha256: sha("c"),
        recipeId: sourceRecipeId,
        recipeDocumentSha256: sha("d"),
      },
      revision: {
        projectId,
        projectName: "Moderated mediation revision",
        createdAt: "2026-08-20T10:00:00Z",
        modelId: "model:revision-v2",
        modelName: "Moderated mediation revision",
        recipeId,
      },
      intent: {
        kind: "select_two_way_moderated_mediation_path",
        intent_version: 2,
        sem_generation: "general_sem_v1",
        estimand_id: estimandId,
        ordered_relation_ids: [...relationIds],
      },
    },
  };
}

export function compiledTargetV1(): CompiledPlsTwoWayModeratedMediationTargetV1 {
  return {
    contract_version: "qpls.compiled-pls-two-way-moderated-mediation-target.v1",
    target_id: "sem_moderated_mediation_target_v1_fixture",
    base_pls_capability_cell: { ...baseCell },
    moderation_point_capability_cell: { ...moderationCell },
    bootstrap_capability_cell: { ...GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1 },
    estimand_id: estimandId,
    specific_path_identity: estimandId,
    ordered_relation_ids: [...relationIds],
    x_id: "construct:x",
    mediator_id: "construct:m",
    y_id: "construct:y",
    moderator_id: "construct:w",
    moderated_stage: "first_stage",
    moderated_relation_id: relationIds[0],
    other_stage_relation_id: relationIds[1],
    interaction_id: "interaction:x:w:m",
    interaction_effect_relation_id: "relation:interaction:m",
    interaction_effect_parameter_id: "parameter:interaction:m",
    generated_product_column_id: "generated:x:w",
    stage_one_model_scientific_sha256: sha("6"),
    product_scale_version: "qpls.general-sem-pls.two-stage-product.sample-standardized.v1",
    probe_policy_version: "standardized_moderator_minus_one_zero_plus_one_v1",
    conditional_target_version: "conditional_indirect_effect_v1",
    index_target_version: "index_of_moderated_mediation_v1",
  };
}

export function receiptV2(): GeneralSemExecutionAuthorityRevisionReceiptV2 {
  return {
    schemaVersion: 2,
    archiveSchemaVersion: 6,
    revisionNumber: 1,
    sourceArchivePath: "D:\\source.qpls",
    sourceArchiveSha256: sha("a"),
    sourceArchiveBytes: 100,
    sourceVerifiedUnchanged: true,
    sourceProjectId,
    sourceModelId: "model:source",
    sourceModelDocumentSha256: sha("b"),
    sourceModelScientificSha256: sha("c"),
    sourceRecipeId,
    sourceRecipeDocumentSha256: sha("d"),
    destinationArchivePath: "D:\\revision-v2.qpls",
    destinationArchiveSha256: sha("e"),
    destinationArchiveBytes: 200,
    strictReopenValidated: true,
    projectId,
    name: "Moderated mediation revision",
    createdAt: "2026-08-20T10:00:00Z",
    residentDatasetId: datasetId,
    residentDatasetFingerprint: "dataset-fingerprint",
    residentModelId: "model:revision-v2",
    residentModelDocumentSha256: sha("f"),
    residentModelScientificSha256: sha("0"),
    residentRecipeId: recipeId,
    residentRecipeDocumentSha256: sha("1"),
    compilerVersion: "recipe_v4_to_compiled_pls_plan_v3_two_way_moderated_mediation_bootstrap_v1",
    primaryCapabilityCell: { ...moderationCell },
    supplementalCapabilityCell: { ...GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1 },
    capabilityDependencies: dependencies.map((cell) => ({ ...cell })),
    recipeAnalyticalSha256: sha("2"),
    generalSemConfigSha256: sha("3"),
    compiledPlanSha256: sha("4"),
    compiledArtifactIdentitySha256: sha("5"),
    compiledTargetSha256: sha("7"),
    compiledTarget: compiledTargetV1(),
  };
}

export function lineageV2(): GeneralSemExecutionAuthorityRevisionLineageV2 {
  const request = requestV2();
  const receipt = receiptV2();
  return {
    schemaVersion: 2,
    revisionNumber: 1,
    parentRevisionNumber: 0,
    sourceArchiveSha256: request.expectedSourceArchiveSha256,
    sourceArchiveBytes: receipt.sourceArchiveBytes,
    source: {
      projectId: sourceProjectId,
      modelId: "model:source",
      modelDocumentSha256: sha("b"),
      modelScientificSha256: sha("c"),
      recipeId: sourceRecipeId,
      recipeDocumentSha256: sha("d"),
    },
    revised: {
      projectId,
      modelId: "model:revision-v2",
      modelDocumentSha256: sha("f"),
      modelScientificSha256: sha("0"),
      recipeId,
      recipeDocumentSha256: sha("1"),
    },
    compilation: {
      compilerVersion: receipt.compilerVersion,
      capabilityCell: { ...moderationCell },
      recipeAnalyticalSha256: sha("2"),
      generalSemConfigSha256: sha("3"),
      compiledPlanSha256: sha("4"),
      compiledArtifactIdentitySha256: sha("5"),
    },
    supplementalCapabilityCell: { ...GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1 },
    capabilityDependencies: dependencies.map((cell) => ({ ...cell })),
    compiledTargetSha256: sha("7"),
    compiledTarget: compiledTargetV1(),
    intent: { ...request.revision.intent, ordered_relation_ids: [...relationIds] },
  };
}

function recipeWithPath() {
  return {
    general_sem_config: {
      requested_effect_estimands: [{
        kind: "specific_path",
        estimand_id: estimandId,
        ordered_relation_ids: [...relationIds],
      }],
    },
  } as never;
}

export function destinationSnapshotV2(): InternalProjectArchiveV6ReadSnapshotV1 {
  const receipt = receiptV2();
  const lineage = lineageV2();
  const recipe = recipeWithPath();
  return {
    schemaVersion: 1,
    access: "read_only",
    loader: "strict_schema6_zip",
    archivePath: receipt.destinationArchivePath,
    archiveSha256: receipt.destinationArchiveSha256,
    archiveBytes: receipt.destinationArchiveBytes,
    manifest: {
      schema_version: 6,
      project_id: projectId,
      name: receipt.name,
      created_at: receipt.createdAt,
      modified_at: receipt.createdAt,
      engine_version: "test",
      checksum_algorithm: "sha256",
      checksums: {},
    },
    project: {
      schema_version: 6,
      project_id: projectId,
      name: receipt.name,
      created_at: receipt.createdAt,
      modified_at: receipt.createdAt,
      datasets: [{
        id: datasetId,
        name: "Data",
        fingerprint: "dataset-fingerprint",
        schema: { version: 1, kind: "raw", case_count: 10, sample_size: 10, columns: [] },
      }],
      models: [{
        model_id: receipt.residentModelId,
        payload: { kind: "sem_model_v4", model: {} as never, scientific_sha256: receipt.residentModelScientificSha256 },
      }],
      recipes: [recipe],
      historical_recipes: [],
      layouts: { general_sem_execution_authority_revision_v2: lineage },
      historical_results: [],
      canonical_result_documents: [],
      origin: { kind: "new_project" },
      sem_generation: "general_sem_v1",
    },
    residentDatasets: [{
      datasetId,
      name: "Data",
      fingerprint: "dataset-fingerprint",
      rowCount: 10,
      columnCount: 0,
      sampleSize: 10,
      arrowResident: true,
    }],
    counts: {
      datasets: 1,
      models: 1,
      recipes: 1,
      historicalRecipes: 0,
      historicalResults: 0,
      canonicalResultDocuments: 0,
    },
    generalSemExecutionAuthority: {
      schemaVersion: 1,
      projectId,
      datasetId,
      datasetFingerprint: "dataset-fingerprint",
      modelId: receipt.residentModelId,
      modelScientificSha256: receipt.residentModelScientificSha256,
      recipeId,
      recipeDocumentSha256: receipt.residentRecipeDocumentSha256,
      recipe,
    },
    sourceRecheckedUnchanged: true,
  };
}

export function sourceSnapshotV2(): InternalProjectArchiveV6ReadSnapshotV1 {
  const destination = destinationSnapshotV2();
  const recipe = { general_sem_config: { requested_effect_estimands: [] } } as never;
  return {
    ...destination,
    archivePath: "D:\\source.qpls",
    archiveSha256: sha("a"),
    archiveBytes: 100,
    manifest: { ...destination.manifest, project_id: sourceProjectId, name: "Source" },
    project: {
      ...destination.project,
      project_id: sourceProjectId,
      name: "Source",
      models: [{
        model_id: "model:source",
        payload: { kind: "sem_model_v4", model: {} as never, scientific_sha256: sha("c") },
      }],
      recipes: [recipe],
      layouts: {},
    },
    generalSemExecutionAuthority: {
      schemaVersion: 1,
      projectId: sourceProjectId,
      datasetId,
      datasetFingerprint: "dataset-fingerprint",
      modelId: "model:source",
      modelScientificSha256: sha("c"),
      recipeId: sourceRecipeId,
      recipeDocumentSha256: sha("d"),
      recipe,
    },
  };
}

export function transactionV2(): InternalGeneralSemModeratedMediationRevisionTransactionV2 {
  const request = requestV2();
  return {
    snapshot: sourceSnapshotV2(),
    source: request.revision.source,
    revision: request.revision.revision,
    selection: selectionV1(),
    destinationArchivePath: request.destinationArchivePath,
  };
}
