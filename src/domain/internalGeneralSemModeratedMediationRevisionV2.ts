import type { CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import {
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1,
} from "./generalSemModeratedMediationAuthoringV1";
import { specificDirectedPathIdentityV1 } from "./generalSemCapabilityPreflightV1";
import type {
  GeneralSemExecutionAuthorityRevisionIdentityV1,
  GeneralSemExecutionAuthoritySourcePinV1,
  InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1,
} from "./internalGeneralSemExecutionAuthorityRevisionV1";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const MAX_SAFE_COUNT = Number.MAX_SAFE_INTEGER;
const TARGET_CONTRACT_VERSION = "qpls.compiled-pls-two-way-moderated-mediation-target.v1";
const PRODUCT_SCALE_VERSION = "qpls.general-sem-pls.two-stage-product.sample-standardized.v1";
const PROBE_POLICY_VERSION = "standardized_moderator_minus_one_zero_plus_one_v1";
const CONDITIONAL_TARGET_VERSION = "conditional_indirect_effect_v1";
const INDEX_TARGET_VERSION = "index_of_moderated_mediation_v1";

type WireRecord = Record<string, unknown>;

export const INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_COMMAND_V2 =
  "revise_internal_general_sem_execution_authority_v2" as const;
export const INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_SURFACE_V2 =
  "internal_labs" as const;
export const GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY =
  "general_sem_execution_authority_revision_v2" as const;

export interface GeneralSemModeratedMediationRevisionIntentV2 {
  kind: "select_two_way_moderated_mediation_path";
  intent_version: 2;
  sem_generation: "general_sem_v1";
  estimand_id: string;
  ordered_relation_ids: [string, string];
}

export interface InternalGeneralSemModeratedMediationRevisionRequestV2 {
  surface: typeof INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_SURFACE_V2;
  experimentalLabsEnabled: true;
  sourceArchivePath: string;
  expectedSourceArchiveSha256: string;
  destinationArchivePath: string;
  revision: {
    source: GeneralSemExecutionAuthoritySourcePinV1;
    revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
    intent: GeneralSemModeratedMediationRevisionIntentV2;
  };
}

export type CompiledPlsTwoWayModeratedMediationStageV1 = "first_stage" | "second_stage";

export interface CompiledPlsTwoWayModeratedMediationTargetV1 {
  contract_version: typeof TARGET_CONTRACT_VERSION;
  target_id: string;
  base_pls_capability_cell: CapabilityCellReferenceV2;
  moderation_point_capability_cell: CapabilityCellReferenceV2;
  bootstrap_capability_cell: CapabilityCellReferenceV2;
  estimand_id: string;
  specific_path_identity: string;
  ordered_relation_ids: [string, string];
  x_id: string;
  mediator_id: string;
  y_id: string;
  moderator_id: string;
  moderated_stage: CompiledPlsTwoWayModeratedMediationStageV1;
  moderated_relation_id: string;
  other_stage_relation_id: string;
  interaction_id: string;
  interaction_effect_relation_id: string;
  interaction_effect_parameter_id: string;
  generated_product_column_id: string;
  stage_one_model_scientific_sha256: string;
  product_scale_version: typeof PRODUCT_SCALE_VERSION;
  probe_policy_version: typeof PROBE_POLICY_VERSION;
  conditional_target_version: typeof CONDITIONAL_TARGET_VERSION;
  index_target_version: typeof INDEX_TARGET_VERSION;
}

interface GeneralSemRevisionAuthorityIdentityV2 {
  projectId: string;
  modelId: string;
  modelDocumentSha256: string;
  modelScientificSha256: string;
  recipeId: string;
  recipeDocumentSha256: string;
}

interface GeneralSemRevisionCompilationIdentityV2 {
  compilerVersion: string;
  capabilityCell: CapabilityCellReferenceV2;
  recipeAnalyticalSha256: string;
  generalSemConfigSha256: string;
  compiledPlanSha256: string;
  compiledArtifactIdentitySha256: string;
}

export interface GeneralSemExecutionAuthorityRevisionReceiptV2 {
  schemaVersion: 2;
  archiveSchemaVersion: 6;
  revisionNumber: number;
  sourceArchivePath: string;
  sourceArchiveSha256: string;
  sourceArchiveBytes: number;
  sourceVerifiedUnchanged: true;
  sourceProjectId: string;
  sourceModelId: string;
  sourceModelDocumentSha256: string;
  sourceModelScientificSha256: string;
  sourceRecipeId: string;
  sourceRecipeDocumentSha256: string;
  destinationArchivePath: string;
  destinationArchiveSha256: string;
  destinationArchiveBytes: number;
  strictReopenValidated: true;
  projectId: string;
  name: string;
  createdAt: string;
  residentDatasetId: string;
  residentDatasetFingerprint: string;
  residentModelId: string;
  residentModelDocumentSha256: string;
  residentModelScientificSha256: string;
  residentRecipeId: string;
  residentRecipeDocumentSha256: string;
  compilerVersion: string;
  primaryCapabilityCell: CapabilityCellReferenceV2;
  supplementalCapabilityCell: CapabilityCellReferenceV2;
  capabilityDependencies: CapabilityCellReferenceV2[];
  recipeAnalyticalSha256: string;
  generalSemConfigSha256: string;
  compiledPlanSha256: string;
  compiledArtifactIdentitySha256: string;
  compiledTargetSha256: string;
  compiledTarget: CompiledPlsTwoWayModeratedMediationTargetV1;
}

export interface GeneralSemExecutionAuthorityRevisionLineageV2 {
  schemaVersion: 2;
  revisionNumber: number;
  parentRevisionNumber: number;
  sourceArchiveSha256: string;
  sourceArchiveBytes: number;
  source: GeneralSemRevisionAuthorityIdentityV2;
  revised: GeneralSemRevisionAuthorityIdentityV2;
  compilation: GeneralSemRevisionCompilationIdentityV2;
  supplementalCapabilityCell: CapabilityCellReferenceV2;
  capabilityDependencies: CapabilityCellReferenceV2[];
  compiledTargetSha256: string;
  compiledTarget: CompiledPlsTwoWayModeratedMediationTargetV1;
  intent: GeneralSemModeratedMediationRevisionIntentV2;
}

export type InternalGeneralSemModeratedMediationRevisionNativeOutcomeV2 =
  | {
    status: "ok";
    value: {
      schemaVersion: 2;
      persistence: "persisted_new_revision";
      receipt: GeneralSemExecutionAuthorityRevisionReceiptV2;
    };
  }
  | { status: "blocked"; diagnostic: InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1 };

export class InternalGeneralSemModeratedMediationRevisionWireError extends Error {
  constructor(public readonly code: string, public readonly path: string, message: string) {
    super(message);
    this.name = "InternalGeneralSemModeratedMediationRevisionWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalGeneralSemModeratedMediationRevisionWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail("schema6_general_sem_revision_v2.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(value: unknown, fields: readonly string[], path: string): WireRecord {
  const record = recordAt(value, path);
  const expected = new Set(fields);
  for (const field of fields) {
    if (!Object.prototype.hasOwnProperty.call(record, field)) {
      fail("schema6_general_sem_revision_v2.field_missing", `${path}.${field}`, `${path}.${field} is required.`);
    }
  }
  for (const field of Object.keys(record)) {
    if (!expected.has(field)) {
      fail("schema6_general_sem_revision_v2.field_unknown", `${path}.${field}`, `${path}.${field} is not allowed.`);
    }
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0 || value.trim() !== value) {
    fail("schema6_general_sem_revision_v2.text_invalid", path, `${path} must be nonempty without surrounding whitespace.`);
  }
  return value;
}

function uuidAt(value: unknown, path: string): string {
  const uuid = textAt(value, path);
  if (!CANONICAL_UUID.test(uuid)) {
    fail("schema6_general_sem_revision_v2.uuid_invalid", path, `${path} must be a canonical non-nil lowercase UUID.`);
  }
  return uuid;
}

function shaAt(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) {
    fail("schema6_general_sem_revision_v2.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  }
  return digest;
}

function countAt(value: unknown, path: string, positive = false): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0)
    || (value as number) < (positive ? 1 : 0) || (value as number) > MAX_SAFE_COUNT) {
    fail("schema6_general_sem_revision_v2.count_invalid", path, `${path} must be a ${positive ? "positive" : "nonnegative"} safe integer.`);
  }
  return value as number;
}

function timestampAt(value: unknown, path: string): string {
  const timestamp = textAt(value, path);
  if (Number.isNaN(Date.parse(timestamp))) {
    fail("schema6_general_sem_revision_v2.timestamp_invalid", path, `${path} must be an RFC3339 timestamp.`);
  }
  return timestamp;
}

function pairAt(value: unknown, path: string): [string, string] {
  if (!Array.isArray(value) || value.length !== 2) {
    fail("schema6_general_sem_revision_v2.relation_pair_invalid", path, `${path} must contain exactly two ordered relation ids.`);
  }
  const pair: [string, string] = [textAt(value[0], `${path}[0]`), textAt(value[1], `${path}[1]`)];
  if (pair[0] === pair[1]) {
    fail("schema6_general_sem_revision_v2.relation_pair_invalid", path, `${path} must contain two distinct relation ids.`);
  }
  return pair;
}

function parseCapabilityCell(value: unknown, path: string): CapabilityCellReferenceV2 {
  const cell = exactRecordAt(value, [
    "registry_schema_version", "capability_id", "cell_id", "capability_version",
  ], path);
  if (cell.registry_schema_version !== 2) {
    fail("schema6_general_sem_revision_v2.capability_invalid", path, "Capability registry schema must equal 2.");
  }
  return {
    registry_schema_version: 2,
    capability_id: textAt(cell.capability_id, `${path}.capability_id`),
    cell_id: textAt(cell.cell_id, `${path}.cell_id`),
    capability_version: textAt(cell.capability_version, `${path}.capability_version`),
  };
}

function capabilityIdentity(cell: CapabilityCellReferenceV2): string {
  return `${cell.registry_schema_version}:${cell.capability_id}:${cell.cell_id}:${cell.capability_version}`;
}

function sameCapability(left: CapabilityCellReferenceV2, right: CapabilityCellReferenceV2): boolean {
  return capabilityIdentity(left) === capabilityIdentity(right);
}

function expectedDependencies(): CapabilityCellReferenceV2[] {
  return [...GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_DEPENDENCIES_V1]
    .sort((left, right) => capabilityIdentity(left).localeCompare(capabilityIdentity(right), "en"));
}

function parseCapabilityDependencies(value: unknown, path: string): CapabilityCellReferenceV2[] {
  if (!Array.isArray(value) || value.length !== 2) {
    fail("schema6_general_sem_revision_v2.dependencies_invalid", path, `${path} must contain exactly the base PLS and moderation-point cells.`);
  }
  const dependencies = value.map((cell, index) => parseCapabilityCell(cell, `${path}[${index}]`));
  const expected = expectedDependencies();
  if (!dependencies.every((cell, index) => sameCapability(cell, expected[index]!))) {
    fail("schema6_general_sem_revision_v2.dependencies_invalid", path, `${path} differs from the canonical dependency inventory.`);
  }
  return dependencies;
}

function parseIntent(value: unknown, path: string): GeneralSemModeratedMediationRevisionIntentV2 {
  const intent = exactRecordAt(value, [
    "kind", "intent_version", "sem_generation", "estimand_id", "ordered_relation_ids",
  ], path);
  if (intent.kind !== "select_two_way_moderated_mediation_path"
    || intent.intent_version !== 2 || intent.sem_generation !== "general_sem_v1") {
    fail("schema6_general_sem_revision_v2.intent_invalid", path, "Revision-v2 requires the exact moderated-mediation SpecificPath intent.");
  }
  const ordered_relation_ids = pairAt(intent.ordered_relation_ids, `${path}.ordered_relation_ids`);
  const estimand_id = textAt(intent.estimand_id, `${path}.estimand_id`);
  if (estimand_id !== specificDirectedPathIdentityV1(ordered_relation_ids)) {
    fail("schema6_general_sem_revision_v2.estimand_identity_invalid", `${path}.estimand_id`, "Estimand id must equal the stable selected-path identity.");
  }
  return {
    kind: "select_two_way_moderated_mediation_path",
    intent_version: 2,
    sem_generation: "general_sem_v1",
    estimand_id,
    ordered_relation_ids,
  };
}

function parseSourcePin(value: unknown, path: string): GeneralSemExecutionAuthoritySourcePinV1 {
  const source = exactRecordAt(value, [
    "projectId", "modelId", "modelDocumentSha256", "modelScientificSha256",
    "recipeId", "recipeDocumentSha256",
  ], path);
  return {
    projectId: uuidAt(source.projectId, `${path}.projectId`),
    modelId: textAt(source.modelId, `${path}.modelId`),
    modelDocumentSha256: shaAt(source.modelDocumentSha256, `${path}.modelDocumentSha256`),
    modelScientificSha256: shaAt(source.modelScientificSha256, `${path}.modelScientificSha256`),
    recipeId: uuidAt(source.recipeId, `${path}.recipeId`),
    recipeDocumentSha256: shaAt(source.recipeDocumentSha256, `${path}.recipeDocumentSha256`),
  };
}

function parseRevisionIdentity(value: unknown, path: string): GeneralSemExecutionAuthorityRevisionIdentityV1 {
  const identity = exactRecordAt(value, [
    "projectId", "projectName", "createdAt", "modelId", "modelName", "recipeId",
  ], path);
  return {
    projectId: uuidAt(identity.projectId, `${path}.projectId`),
    projectName: textAt(identity.projectName, `${path}.projectName`),
    createdAt: timestampAt(identity.createdAt, `${path}.createdAt`),
    modelId: textAt(identity.modelId, `${path}.modelId`),
    modelName: textAt(identity.modelName, `${path}.modelName`),
    recipeId: uuidAt(identity.recipeId, `${path}.recipeId`),
  };
}

export function parseInternalGeneralSemModeratedMediationRevisionRequestV2(
  input: unknown,
): InternalGeneralSemModeratedMediationRevisionRequestV2 {
  const request = exactRecordAt(input, [
    "surface", "experimentalLabsEnabled", "sourceArchivePath",
    "expectedSourceArchiveSha256", "destinationArchivePath", "revision",
  ], "request");
  if (request.surface !== INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_SURFACE_V2
    || request.experimentalLabsEnabled !== true) {
    fail("schema6_general_sem_revision_v2.internal_labs_required", "request", "Revision-v2 requires the Experimental Labs boundary.");
  }
  const revision = exactRecordAt(request.revision, ["source", "revision", "intent"], "request.revision");
  const parsed: InternalGeneralSemModeratedMediationRevisionRequestV2 = {
    surface: INTERNAL_GENERAL_SEM_MODERATED_MEDIATION_REVISION_SURFACE_V2,
    experimentalLabsEnabled: true,
    sourceArchivePath: textAt(request.sourceArchivePath, "request.sourceArchivePath"),
    expectedSourceArchiveSha256: shaAt(request.expectedSourceArchiveSha256, "request.expectedSourceArchiveSha256"),
    destinationArchivePath: textAt(request.destinationArchivePath, "request.destinationArchivePath"),
    revision: {
      source: parseSourcePin(revision.source, "request.revision.source"),
      revision: parseRevisionIdentity(revision.revision, "request.revision.revision"),
      intent: parseIntent(revision.intent, "request.revision.intent"),
    },
  };
  if (parsed.sourceArchivePath.toLowerCase() === parsed.destinationArchivePath.toLowerCase()) {
    fail("schema6_general_sem_revision_v2.new_destination_required", "request.destinationArchivePath", "Revision-v2 requires a new destination path.");
  }
  if (parsed.revision.source.projectId === parsed.revision.revision.projectId
    || parsed.revision.source.modelId === parsed.revision.revision.modelId
    || parsed.revision.source.recipeId === parsed.revision.revision.recipeId) {
    fail("schema6_general_sem_revision_v2.new_identity_required", "request.revision.revision", "Project, model, and Recipe identities must all be new.");
  }
  return parsed;
}

export function parseCompiledPlsTwoWayModeratedMediationTargetV1(
  value: unknown,
  intent: GeneralSemModeratedMediationRevisionIntentV2,
  path = "compiledTarget",
): CompiledPlsTwoWayModeratedMediationTargetV1 {
  const target = exactRecordAt(value, [
    "contract_version", "target_id", "base_pls_capability_cell",
    "moderation_point_capability_cell", "bootstrap_capability_cell", "estimand_id",
    "specific_path_identity", "ordered_relation_ids", "x_id", "mediator_id", "y_id",
    "moderator_id", "moderated_stage", "moderated_relation_id", "other_stage_relation_id",
    "interaction_id", "interaction_effect_relation_id", "interaction_effect_parameter_id",
    "generated_product_column_id", "stage_one_model_scientific_sha256", "product_scale_version",
    "probe_policy_version", "conditional_target_version", "index_target_version",
  ], path);
  const ordered_relation_ids = pairAt(target.ordered_relation_ids, `${path}.ordered_relation_ids`);
  const moderated_stage = target.moderated_stage;
  if (moderated_stage !== "first_stage" && moderated_stage !== "second_stage") {
    fail("schema6_general_sem_revision_v2.stage_invalid", `${path}.moderated_stage`, "Moderated stage must be first_stage or second_stage.");
  }
  const parsed: CompiledPlsTwoWayModeratedMediationTargetV1 = {
    contract_version: target.contract_version as typeof TARGET_CONTRACT_VERSION,
    target_id: textAt(target.target_id, `${path}.target_id`),
    base_pls_capability_cell: parseCapabilityCell(target.base_pls_capability_cell, `${path}.base_pls_capability_cell`),
    moderation_point_capability_cell: parseCapabilityCell(target.moderation_point_capability_cell, `${path}.moderation_point_capability_cell`),
    bootstrap_capability_cell: parseCapabilityCell(target.bootstrap_capability_cell, `${path}.bootstrap_capability_cell`),
    estimand_id: textAt(target.estimand_id, `${path}.estimand_id`),
    specific_path_identity: textAt(target.specific_path_identity, `${path}.specific_path_identity`),
    ordered_relation_ids,
    x_id: textAt(target.x_id, `${path}.x_id`),
    mediator_id: textAt(target.mediator_id, `${path}.mediator_id`),
    y_id: textAt(target.y_id, `${path}.y_id`),
    moderator_id: textAt(target.moderator_id, `${path}.moderator_id`),
    moderated_stage,
    moderated_relation_id: textAt(target.moderated_relation_id, `${path}.moderated_relation_id`),
    other_stage_relation_id: textAt(target.other_stage_relation_id, `${path}.other_stage_relation_id`),
    interaction_id: textAt(target.interaction_id, `${path}.interaction_id`),
    interaction_effect_relation_id: textAt(target.interaction_effect_relation_id, `${path}.interaction_effect_relation_id`),
    interaction_effect_parameter_id: textAt(target.interaction_effect_parameter_id, `${path}.interaction_effect_parameter_id`),
    generated_product_column_id: textAt(target.generated_product_column_id, `${path}.generated_product_column_id`),
    stage_one_model_scientific_sha256: shaAt(target.stage_one_model_scientific_sha256, `${path}.stage_one_model_scientific_sha256`),
    product_scale_version: target.product_scale_version as typeof PRODUCT_SCALE_VERSION,
    probe_policy_version: target.probe_policy_version as typeof PROBE_POLICY_VERSION,
    conditional_target_version: target.conditional_target_version as typeof CONDITIONAL_TARGET_VERSION,
    index_target_version: target.index_target_version as typeof INDEX_TARGET_VERSION,
  };
  const expected = expectedDependencies();
  const moderatedIndex = moderated_stage === "first_stage" ? 0 : 1;
  const otherIndex = moderated_stage === "first_stage" ? 1 : 0;
  if (target.contract_version !== TARGET_CONTRACT_VERSION
    || target.product_scale_version !== PRODUCT_SCALE_VERSION
    || target.probe_policy_version !== PROBE_POLICY_VERSION
    || target.conditional_target_version !== CONDITIONAL_TARGET_VERSION
    || target.index_target_version !== INDEX_TARGET_VERSION
    || !sameCapability(parsed.base_pls_capability_cell, expected.find((cell) => cell.capability_id === "smartpls.pls_algorithm")!)
    || !sameCapability(parsed.moderation_point_capability_cell, expected.find((cell) => cell.capability_id === "smartpls.moderation")!)
    || !sameCapability(parsed.bootstrap_capability_cell, GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1)
    || parsed.estimand_id !== intent.estimand_id
    || parsed.specific_path_identity !== intent.estimand_id
    || parsed.ordered_relation_ids[0] !== intent.ordered_relation_ids[0]
    || parsed.ordered_relation_ids[1] !== intent.ordered_relation_ids[1]
    || parsed.moderated_relation_id !== intent.ordered_relation_ids[moderatedIndex]
    || parsed.other_stage_relation_id !== intent.ordered_relation_ids[otherIndex]
    || new Set([parsed.x_id, parsed.mediator_id, parsed.y_id, parsed.moderator_id]).size !== 4) {
    fail("schema6_general_sem_revision_v2.target_mismatch", path, "Compiled target differs from the exact bounded moderated-mediation contract.");
  }
  return parsed;
}

function parseAuthorityIdentity(value: unknown, path: string): GeneralSemRevisionAuthorityIdentityV2 {
  const identity = exactRecordAt(value, [
    "projectId", "modelId", "modelDocumentSha256", "modelScientificSha256",
    "recipeId", "recipeDocumentSha256",
  ], path);
  return {
    projectId: uuidAt(identity.projectId, `${path}.projectId`),
    modelId: textAt(identity.modelId, `${path}.modelId`),
    modelDocumentSha256: shaAt(identity.modelDocumentSha256, `${path}.modelDocumentSha256`),
    modelScientificSha256: shaAt(identity.modelScientificSha256, `${path}.modelScientificSha256`),
    recipeId: uuidAt(identity.recipeId, `${path}.recipeId`),
    recipeDocumentSha256: shaAt(identity.recipeDocumentSha256, `${path}.recipeDocumentSha256`),
  };
}

function parseCompilationIdentity(value: unknown, path: string): GeneralSemRevisionCompilationIdentityV2 {
  const compilation = exactRecordAt(value, [
    "compilerVersion", "capabilityCell", "recipeAnalyticalSha256", "generalSemConfigSha256",
    "compiledPlanSha256", "compiledArtifactIdentitySha256",
  ], path);
  return {
    compilerVersion: textAt(compilation.compilerVersion, `${path}.compilerVersion`),
    capabilityCell: parseCapabilityCell(compilation.capabilityCell, `${path}.capabilityCell`),
    recipeAnalyticalSha256: shaAt(compilation.recipeAnalyticalSha256, `${path}.recipeAnalyticalSha256`),
    generalSemConfigSha256: shaAt(compilation.generalSemConfigSha256, `${path}.generalSemConfigSha256`),
    compiledPlanSha256: shaAt(compilation.compiledPlanSha256, `${path}.compiledPlanSha256`),
    compiledArtifactIdentitySha256: shaAt(compilation.compiledArtifactIdentitySha256, `${path}.compiledArtifactIdentitySha256`),
  };
}

function parseReceipt(
  value: unknown,
  request: InternalGeneralSemModeratedMediationRevisionRequestV2,
): GeneralSemExecutionAuthorityRevisionReceiptV2 {
  const path = "outcome.value.receipt";
  const receipt = exactRecordAt(value, [
    "schemaVersion", "archiveSchemaVersion", "revisionNumber", "sourceArchivePath",
    "sourceArchiveSha256", "sourceArchiveBytes", "sourceVerifiedUnchanged", "sourceProjectId",
    "sourceModelId", "sourceModelDocumentSha256", "sourceModelScientificSha256", "sourceRecipeId",
    "sourceRecipeDocumentSha256", "destinationArchivePath", "destinationArchiveSha256",
    "destinationArchiveBytes", "strictReopenValidated", "projectId", "name", "createdAt",
    "residentDatasetId", "residentDatasetFingerprint", "residentModelId",
    "residentModelDocumentSha256", "residentModelScientificSha256", "residentRecipeId",
    "residentRecipeDocumentSha256", "compilerVersion", "primaryCapabilityCell",
    "supplementalCapabilityCell", "capabilityDependencies", "recipeAnalyticalSha256",
    "generalSemConfigSha256", "compiledPlanSha256", "compiledArtifactIdentitySha256",
    "compiledTargetSha256", "compiledTarget",
  ], path);
  if (receipt.schemaVersion !== 2 || receipt.archiveSchemaVersion !== 6
    || receipt.sourceVerifiedUnchanged !== true || receipt.strictReopenValidated !== true) {
    fail("schema6_general_sem_revision_v2.receipt_invalid", path, "Receipt must prove revision-v2, schema-6, source stability, and strict reopen.");
  }
  const parsed: GeneralSemExecutionAuthorityRevisionReceiptV2 = {
    schemaVersion: 2,
    archiveSchemaVersion: 6,
    revisionNumber: countAt(receipt.revisionNumber, `${path}.revisionNumber`, true),
    sourceArchivePath: textAt(receipt.sourceArchivePath, `${path}.sourceArchivePath`),
    sourceArchiveSha256: shaAt(receipt.sourceArchiveSha256, `${path}.sourceArchiveSha256`),
    sourceArchiveBytes: countAt(receipt.sourceArchiveBytes, `${path}.sourceArchiveBytes`, true),
    sourceVerifiedUnchanged: true,
    sourceProjectId: uuidAt(receipt.sourceProjectId, `${path}.sourceProjectId`),
    sourceModelId: textAt(receipt.sourceModelId, `${path}.sourceModelId`),
    sourceModelDocumentSha256: shaAt(receipt.sourceModelDocumentSha256, `${path}.sourceModelDocumentSha256`),
    sourceModelScientificSha256: shaAt(receipt.sourceModelScientificSha256, `${path}.sourceModelScientificSha256`),
    sourceRecipeId: uuidAt(receipt.sourceRecipeId, `${path}.sourceRecipeId`),
    sourceRecipeDocumentSha256: shaAt(receipt.sourceRecipeDocumentSha256, `${path}.sourceRecipeDocumentSha256`),
    destinationArchivePath: textAt(receipt.destinationArchivePath, `${path}.destinationArchivePath`),
    destinationArchiveSha256: shaAt(receipt.destinationArchiveSha256, `${path}.destinationArchiveSha256`),
    destinationArchiveBytes: countAt(receipt.destinationArchiveBytes, `${path}.destinationArchiveBytes`, true),
    strictReopenValidated: true,
    projectId: uuidAt(receipt.projectId, `${path}.projectId`),
    name: textAt(receipt.name, `${path}.name`),
    createdAt: timestampAt(receipt.createdAt, `${path}.createdAt`),
    residentDatasetId: uuidAt(receipt.residentDatasetId, `${path}.residentDatasetId`),
    residentDatasetFingerprint: textAt(receipt.residentDatasetFingerprint, `${path}.residentDatasetFingerprint`),
    residentModelId: textAt(receipt.residentModelId, `${path}.residentModelId`),
    residentModelDocumentSha256: shaAt(receipt.residentModelDocumentSha256, `${path}.residentModelDocumentSha256`),
    residentModelScientificSha256: shaAt(receipt.residentModelScientificSha256, `${path}.residentModelScientificSha256`),
    residentRecipeId: uuidAt(receipt.residentRecipeId, `${path}.residentRecipeId`),
    residentRecipeDocumentSha256: shaAt(receipt.residentRecipeDocumentSha256, `${path}.residentRecipeDocumentSha256`),
    compilerVersion: textAt(receipt.compilerVersion, `${path}.compilerVersion`),
    primaryCapabilityCell: parseCapabilityCell(receipt.primaryCapabilityCell, `${path}.primaryCapabilityCell`),
    supplementalCapabilityCell: parseCapabilityCell(receipt.supplementalCapabilityCell, `${path}.supplementalCapabilityCell`),
    capabilityDependencies: parseCapabilityDependencies(receipt.capabilityDependencies, `${path}.capabilityDependencies`),
    recipeAnalyticalSha256: shaAt(receipt.recipeAnalyticalSha256, `${path}.recipeAnalyticalSha256`),
    generalSemConfigSha256: shaAt(receipt.generalSemConfigSha256, `${path}.generalSemConfigSha256`),
    compiledPlanSha256: shaAt(receipt.compiledPlanSha256, `${path}.compiledPlanSha256`),
    compiledArtifactIdentitySha256: shaAt(receipt.compiledArtifactIdentitySha256, `${path}.compiledArtifactIdentitySha256`),
    compiledTargetSha256: shaAt(receipt.compiledTargetSha256, `${path}.compiledTargetSha256`),
    compiledTarget: parseCompiledPlsTwoWayModeratedMediationTargetV1(
      receipt.compiledTarget,
      request.revision.intent,
      `${path}.compiledTarget`,
    ),
  };
  const { source, revision } = request.revision;
  if (parsed.sourceArchivePath.toLowerCase() !== request.sourceArchivePath.toLowerCase()
    || parsed.sourceArchiveSha256 !== request.expectedSourceArchiveSha256
    || parsed.sourceProjectId !== source.projectId
    || parsed.sourceModelId !== source.modelId
    || parsed.sourceModelDocumentSha256 !== source.modelDocumentSha256
    || parsed.sourceModelScientificSha256 !== source.modelScientificSha256
    || parsed.sourceRecipeId !== source.recipeId
    || parsed.sourceRecipeDocumentSha256 !== source.recipeDocumentSha256
    || parsed.destinationArchivePath.toLowerCase()
      !== request.destinationArchivePath.toLowerCase()
    || parsed.projectId !== revision.projectId
    || parsed.name !== revision.projectName
    || Date.parse(parsed.createdAt) !== Date.parse(revision.createdAt)
    || parsed.residentModelId !== revision.modelId
    || parsed.residentRecipeId !== revision.recipeId
    || !sameCapability(parsed.primaryCapabilityCell, parsed.compiledTarget.moderation_point_capability_cell)
    || !sameCapability(parsed.supplementalCapabilityCell, GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1)
    || !sameCapability(parsed.supplementalCapabilityCell, parsed.compiledTarget.bootstrap_capability_cell)) {
    fail("schema6_general_sem_revision_v2.receipt_request_mismatch", path, "Native revision-v2 receipt differs from the exact pinned request or target authority.");
  }
  return parsed;
}

export function parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2(
  input: unknown,
  requestInput: InternalGeneralSemModeratedMediationRevisionRequestV2,
): InternalGeneralSemModeratedMediationRevisionNativeOutcomeV2 {
  const request = parseInternalGeneralSemModeratedMediationRevisionRequestV2(requestInput);
  const outcome = recordAt(input, "outcome");
  if (outcome.status === "blocked") {
    exactRecordAt(outcome, ["status", "diagnostic"], "outcome");
    const diagnostic = exactRecordAt(outcome.diagnostic, ["code", "message", "correctiveAction"], "outcome.diagnostic");
    return { status: "blocked", diagnostic: {
      code: textAt(diagnostic.code, "outcome.diagnostic.code"),
      message: textAt(diagnostic.message, "outcome.diagnostic.message"),
      correctiveAction: textAt(diagnostic.correctiveAction, "outcome.diagnostic.correctiveAction"),
    } };
  }
  if (outcome.status !== "ok") {
    fail("schema6_general_sem_revision_v2.status_invalid", "outcome.status", "Revision-v2 outcome must be ok or blocked.");
  }
  exactRecordAt(outcome, ["status", "value"], "outcome");
  const result = exactRecordAt(outcome.value, ["schemaVersion", "persistence", "receipt"], "outcome.value");
  if (result.schemaVersion !== 2 || result.persistence !== "persisted_new_revision") {
    fail("schema6_general_sem_revision_v2.result_invalid", "outcome.value", "Revision-v2 result contract is invalid.");
  }
  return { status: "ok", value: {
    schemaVersion: 2,
    persistence: "persisted_new_revision",
    receipt: parseReceipt(result.receipt, request),
  } };
}

export function parseGeneralSemExecutionAuthorityRevisionLineageV2(
  value: unknown,
  request: InternalGeneralSemModeratedMediationRevisionRequestV2,
  receipt: GeneralSemExecutionAuthorityRevisionReceiptV2,
): GeneralSemExecutionAuthorityRevisionLineageV2 {
  const path = `project.layouts.${GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_V2_LAYOUT_KEY}`;
  const lineage = exactRecordAt(value, [
    "schemaVersion", "revisionNumber", "parentRevisionNumber", "sourceArchiveSha256",
    "sourceArchiveBytes", "source", "revised", "compilation", "supplementalCapabilityCell",
    "capabilityDependencies", "compiledTargetSha256", "compiledTarget", "intent",
  ], path);
  const parsed: GeneralSemExecutionAuthorityRevisionLineageV2 = {
    schemaVersion: lineage.schemaVersion as 2,
    revisionNumber: countAt(lineage.revisionNumber, `${path}.revisionNumber`, true),
    parentRevisionNumber: countAt(lineage.parentRevisionNumber, `${path}.parentRevisionNumber`),
    sourceArchiveSha256: shaAt(lineage.sourceArchiveSha256, `${path}.sourceArchiveSha256`),
    sourceArchiveBytes: countAt(lineage.sourceArchiveBytes, `${path}.sourceArchiveBytes`, true),
    source: parseAuthorityIdentity(lineage.source, `${path}.source`),
    revised: parseAuthorityIdentity(lineage.revised, `${path}.revised`),
    compilation: parseCompilationIdentity(lineage.compilation, `${path}.compilation`),
    supplementalCapabilityCell: parseCapabilityCell(lineage.supplementalCapabilityCell, `${path}.supplementalCapabilityCell`),
    capabilityDependencies: parseCapabilityDependencies(lineage.capabilityDependencies, `${path}.capabilityDependencies`),
    compiledTargetSha256: shaAt(lineage.compiledTargetSha256, `${path}.compiledTargetSha256`),
    compiledTarget: parseCompiledPlsTwoWayModeratedMediationTargetV1(lineage.compiledTarget, request.revision.intent, `${path}.compiledTarget`),
    intent: parseIntent(lineage.intent, `${path}.intent`),
  };
  const expectedSource: GeneralSemRevisionAuthorityIdentityV2 = {
    projectId: request.revision.source.projectId,
    modelId: request.revision.source.modelId,
    modelDocumentSha256: request.revision.source.modelDocumentSha256,
    modelScientificSha256: request.revision.source.modelScientificSha256,
    recipeId: request.revision.source.recipeId,
    recipeDocumentSha256: request.revision.source.recipeDocumentSha256,
  };
  const expectedRevised: GeneralSemRevisionAuthorityIdentityV2 = {
    projectId: receipt.projectId,
    modelId: receipt.residentModelId,
    modelDocumentSha256: receipt.residentModelDocumentSha256,
    modelScientificSha256: receipt.residentModelScientificSha256,
    recipeId: receipt.residentRecipeId,
    recipeDocumentSha256: receipt.residentRecipeDocumentSha256,
  };
  const targetMatches = JSON.stringify(parsed.compiledTarget) === JSON.stringify(receipt.compiledTarget);
  if (lineage.schemaVersion !== 2
    || parsed.revisionNumber !== receipt.revisionNumber
    || parsed.parentRevisionNumber + 1 !== parsed.revisionNumber
    || parsed.sourceArchiveSha256 !== receipt.sourceArchiveSha256
    || parsed.sourceArchiveBytes !== receipt.sourceArchiveBytes
    || JSON.stringify(parsed.source) !== JSON.stringify(expectedSource)
    || JSON.stringify(parsed.revised) !== JSON.stringify(expectedRevised)
    || parsed.compilation.compilerVersion !== receipt.compilerVersion
    || !sameCapability(parsed.compilation.capabilityCell, receipt.primaryCapabilityCell)
    || parsed.compilation.recipeAnalyticalSha256 !== receipt.recipeAnalyticalSha256
    || parsed.compilation.generalSemConfigSha256 !== receipt.generalSemConfigSha256
    || parsed.compilation.compiledPlanSha256 !== receipt.compiledPlanSha256
    || parsed.compilation.compiledArtifactIdentitySha256 !== receipt.compiledArtifactIdentitySha256
    || !sameCapability(parsed.supplementalCapabilityCell, receipt.supplementalCapabilityCell)
    || JSON.stringify(parsed.capabilityDependencies) !== JSON.stringify(receipt.capabilityDependencies)
    || parsed.compiledTargetSha256 !== receipt.compiledTargetSha256
    || !targetMatches
    || JSON.stringify(parsed.intent) !== JSON.stringify(request.revision.intent)) {
    fail("schema6_general_sem_revision_v2.lineage_mismatch", path, "Revision-v2 lineage differs from the request, receipt, or compiled target.");
  }
  return parsed;
}
