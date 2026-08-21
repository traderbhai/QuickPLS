import type { CapabilityCellReferenceV2 } from "./canonicalResultDocumentV2";
import {
  GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
} from "./internalRecipeV4GeneralSemWorkspace";
import { GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1 } from "./generalSemCapabilityPreflightV1";
export {
  GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1 as GENERAL_SEM_PLS_LABS_REVISION_RECIPE_EXECUTION_SURFACE_V1,
  GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1 as GENERAL_SEM_PLS_STANDARD_REVISION_RECIPE_EXECUTION_SURFACE_V1,
} from "./internalRecipeV4GeneralSemWorkspace";
import type {
  AddGeneralSemHigherOrderEditorIntentV1,
  AddGeneralSemInteractionV2EditorIntentV1,
  AddModeratingEffectIntentV3,
  RemoveModeratingEffectIntentV1,
  ReplaceModeratingEffectIntentV1,
  ReplaceGeneralSemHigherOrderEditorIntentV1,
} from "./standardSemModelV4Authority";
import {
  standardSemGeneralSemInteractionV2OutputIdV1,
  standardSemGeneralSemInteractionV2TermIdV1,
  standardSemGeneralSemThreeWayInteractionTermIdV1,
} from "./standardSemModelV4Authority";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
type WireRecord = Record<string, unknown>;

export const INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1 = "internal_labs" as const;
export const STANDARD_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1 = "standard" as const;
export const INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_COMMAND_V1 =
  "revise_internal_general_sem_execution_authority_v1" as const;

export interface GeneralSemExecutionAuthoritySourcePinV1 {
  projectId: string;
  modelId: string;
  modelDocumentSha256: string;
  modelScientificSha256: string;
  recipeId: string;
  recipeDocumentSha256: string;
}

export interface GeneralSemExecutionAuthorityRevisionIdentityV1 {
  projectId: string;
  projectName: string;
  createdAt: string;
  modelId: string;
  modelName: string;
  recipeId: string;
}

export type GeneralSemExecutionAuthorityRevisionEditorIntentV1 =
  | AddGeneralSemInteractionV2EditorIntentV1
  | AddModeratingEffectIntentV3
  | ReplaceModeratingEffectIntentV1
  | RemoveModeratingEffectIntentV1
  | AddGeneralSemHigherOrderEditorIntentV1
  | ReplaceGeneralSemHigherOrderEditorIntentV1;

export interface InternalGeneralSemExecutionAuthorityRevisionRequestV1 {
  surface:
    | typeof INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1
    | typeof STANDARD_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1;
  experimentalLabsEnabled: boolean;
  sourceArchivePath: string;
  expectedSourceArchiveSha256: string;
  destinationArchivePath: string;
  revision: {
    source: GeneralSemExecutionAuthoritySourcePinV1;
    revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
    intent: GeneralSemExecutionAuthorityRevisionEditorIntentV1;
    expectedCapabilityCell: CapabilityCellReferenceV2;
    recipeExecutionSurface:
      | typeof GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
      | typeof GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1;
  };
}

export interface GeneralSemExecutionAuthorityRevisionReceiptV1 {
  schemaVersion: 1;
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
  capabilityCell: CapabilityCellReferenceV2;
  recipeAnalyticalSha256: string;
  generalSemConfigSha256: string;
  compiledPlanSha256: string;
  compiledArtifactIdentitySha256: string;
  interactionTermId: string;
  interactionOutputId: string;
}

export interface InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
}

export type InternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1 =
  | {
    status: "ok";
    value: {
      schemaVersion: 1;
      persistence: "persisted_new_revision";
      receipt: GeneralSemExecutionAuthorityRevisionReceiptV1;
    };
  }
  | { status: "blocked"; diagnostic: InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1 };

export class InternalGeneralSemExecutionAuthorityRevisionWireError extends Error {
  constructor(public readonly code: string, public readonly path: string, message: string) {
    super(message);
    this.name = "InternalGeneralSemExecutionAuthorityRevisionWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalGeneralSemExecutionAuthorityRevisionWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail("schema6_general_sem_revision.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(value: unknown, fields: readonly string[], path: string): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(fields);
  for (const field of fields) if (!Object.prototype.hasOwnProperty.call(record, field)) {
    fail("schema6_general_sem_revision.field_missing", `${path}.${field}`, `${path}.${field} is required.`);
  }
  for (const field of Object.keys(record)) if (!allowed.has(field)) {
    fail("schema6_general_sem_revision.field_unknown", `${path}.${field}`, `${path}.${field} is not allowed.`);
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value || value.trim() !== value) {
    fail("schema6_general_sem_revision.text_invalid", path, `${path} must be nonempty without surrounding whitespace.`);
  }
  return value;
}

function uuidAt(value: unknown, path: string): string {
  const uuid = textAt(value, path);
  if (!CANONICAL_UUID.test(uuid)) fail("schema6_general_sem_revision.uuid_invalid", path, `${path} must be a canonical non-nil lowercase UUID.`);
  return uuid;
}

function shaAt(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) fail("schema6_general_sem_revision.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  return digest;
}

function countAt(value: unknown, path: string, positive = false): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < (positive ? 1 : 0)) {
    fail("schema6_general_sem_revision.count_invalid", path, `${path} must be a ${positive ? "positive" : "nonnegative"} safe integer.`);
  }
  return value as number;
}

function timestampAt(value: unknown, path: string): string {
  const timestamp = textAt(value, path);
  if (Number.isNaN(Date.parse(timestamp))) fail("schema6_general_sem_revision.timestamp_invalid", path, `${path} must be an RFC3339 timestamp.`);
  return timestamp;
}

function parseIntent(value: unknown, path: string): GeneralSemExecutionAuthorityRevisionEditorIntentV1 {
  const candidate = recordAt(value, path);
  if (candidate.kind === "add_moderating_effect_v3" || candidate.kind === "replace_moderating_effect") {
    const replacing = candidate.kind === "replace_moderating_effect";
    const intent = exactRecordAt(candidate, [
      "kind", "intent_version", "sem_generation",
      ...(replacing ? ["term_id", "output_id"] : []),
      "label", "operands", "target", "outcome", "method", "hierarchy_policy",
    ], path);
    if (intent.intent_version !== 3
      || intent.sem_generation !== "general_sem_v1"
      || intent.method !== "two_stage"
      || intent.hierarchy_policy !== "strong"
      || !Array.isArray(intent.operands)
      || (intent.operands.length !== 2 && intent.operands.length !== 3)) {
      fail("schema6_general_sem_revision.intent_invalid", path, "Moderating-effect v3 requires two or three operands, two_stage, and strong hierarchy.");
    }
    const operands = intent.operands.map((operand, index) => textAt(operand, `${path}.operands[${index}]`));
    if (new Set(operands).size !== operands.length) {
      fail("schema6_general_sem_revision.operands_invalid", `${path}.operands`, "Moderating-effect operands must be distinct.");
    }
    const rawTarget = recordAt(intent.target, `${path}.target`);
    const target = rawTarget.kind === "focal_relation"
      ? (() => {
        const parsed = exactRecordAt(rawTarget, ["kind", "relationId"], `${path}.target`);
        return { kind: "focal_relation" as const, relationId: textAt(parsed.relationId, `${path}.target.relationId`) };
      })()
      : rawTarget.kind === "parent_interaction"
        ? (() => {
          const parsed = exactRecordAt(rawTarget, ["kind", "interactionTermId"], `${path}.target`);
          return { kind: "parent_interaction" as const, interactionTermId: textAt(parsed.interactionTermId, `${path}.target.interactionTermId`) };
        })()
        : fail("schema6_general_sem_revision.target_invalid", `${path}.target`, "The moderation target must be a focal relation or parent interaction.");
    if (operands.length === 2 && target.kind !== "focal_relation"
      || operands.length === 3 && target.kind !== "parent_interaction") {
      fail("schema6_general_sem_revision.target_invalid", `${path}.target`, "Two-way moderation targets a focal relation; three-way moderation targets a parent interaction.");
    }
    const common = {
      intent_version: 3 as const,
      sem_generation: "general_sem_v1" as const,
      label: textAt(intent.label, `${path}.label`),
      operands: operands as [string, string] | [string, string, string],
      target,
      outcome: textAt(intent.outcome, `${path}.outcome`),
      method: "two_stage" as const,
      hierarchy_policy: "strong" as const,
    };
    return replacing
      ? {
        kind: "replace_moderating_effect",
        term_id: textAt(intent.term_id, `${path}.term_id`),
        output_id: textAt(intent.output_id, `${path}.output_id`),
        ...common,
      }
      : { kind: "add_moderating_effect_v3", ...common };
  }
  if (candidate.kind === "remove_moderating_effect") {
    const intent = exactRecordAt(candidate, [
      "kind", "intent_version", "sem_generation", "term_id", "output_id",
    ], path);
    if (intent.intent_version !== 3 || intent.sem_generation !== "general_sem_v1") {
      fail("schema6_general_sem_revision.intent_invalid", path, "Removing moderation requires the exact version-3 General SEM intent.");
    }
    return {
      kind: "remove_moderating_effect",
      intent_version: 3,
      sem_generation: "general_sem_v1",
      term_id: textAt(intent.term_id, `${path}.term_id`),
      output_id: textAt(intent.output_id, `${path}.output_id`),
    };
  }
  if (candidate.kind === "replace_higher_order") {
    const intent = exactRecordAt(candidate, [
      "kind", "term_id", "output_id", "label", "components", "approach", "measurement_type",
    ], path);
    if (!Array.isArray(intent.components) || intent.components.length < 2) {
      fail("schema6_general_sem_revision.components_invalid", `${path}.components`, "A HOC replacement requires at least two lower-order components.");
    }
    const components = intent.components.map((component, index) => textAt(component, `${path}.components[${index}]`));
    if (new Set(components).size !== components.length) {
      fail("schema6_general_sem_revision.components_invalid", `${path}.components`, "HOC components must be distinct.");
    }
    const approaches = ["repeated_indicators", "extended_repeated_indicators", "embedded_two_stage", "disjoint_two_stage", "hybrid"] as const;
    const measurementTypes = ["reflective_reflective", "reflective_formative", "formative_reflective", "formative_formative"] as const;
    if (!approaches.includes(intent.approach as typeof approaches[number])
      || !measurementTypes.includes(intent.measurement_type as typeof measurementTypes[number])) {
      fail("schema6_general_sem_revision.intent_invalid", path, "The HOC approach or measurement type is invalid.");
    }
    return {
      kind: "replace_higher_order",
      term_id: textAt(intent.term_id, `${path}.term_id`),
      output_id: textAt(intent.output_id, `${path}.output_id`),
      label: textAt(intent.label, `${path}.label`),
      components,
      approach: intent.approach as ReplaceGeneralSemHigherOrderEditorIntentV1["approach"],
      measurement_type: intent.measurement_type as ReplaceGeneralSemHigherOrderEditorIntentV1["measurement_type"],
    };
  }
  if (candidate.kind === "add_higher_order") {
    const intent = exactRecordAt(candidate, [
      "kind", "term_id", "output_id", "label", "components", "approach", "measurement_type",
      "initial_path",
    ], path);
    if (!Array.isArray(intent.components) || intent.components.length < 2) {
      fail("schema6_general_sem_revision.components_invalid", `${path}.components`, "A HOC revision requires at least two lower-order components.");
    }
    const components = intent.components.map((value, index) => textAt(value, `${path}.components[${index}]`));
    if (new Set(components).size !== components.length) {
      fail("schema6_general_sem_revision.components_invalid", `${path}.components`, "HOC components must be distinct.");
    }
    const approaches = ["repeated_indicators", "extended_repeated_indicators", "embedded_two_stage", "disjoint_two_stage", "hybrid"] as const;
    const measurementTypes = ["reflective_reflective", "reflective_formative", "formative_reflective", "formative_formative"] as const;
    if (!approaches.includes(intent.approach as typeof approaches[number])
      || !measurementTypes.includes(intent.measurement_type as typeof measurementTypes[number])) {
      fail("schema6_general_sem_revision.intent_invalid", path, "The HOC approach or measurement type is invalid.");
    }
    const initialPath = exactRecordAt(intent.initial_path, [
      "relation_id", "source", "target", "label",
    ], `${path}.initial_path`);
    return {
      kind: "add_higher_order",
      term_id: textAt(intent.term_id, `${path}.term_id`),
      output_id: textAt(intent.output_id, `${path}.output_id`),
      label: textAt(intent.label, `${path}.label`),
      components,
      approach: intent.approach as AddGeneralSemHigherOrderEditorIntentV1["approach"],
      measurement_type: intent.measurement_type as AddGeneralSemHigherOrderEditorIntentV1["measurement_type"],
      initial_path: {
        relation_id: textAt(initialPath.relation_id, `${path}.initial_path.relation_id`),
        source: textAt(initialPath.source, `${path}.initial_path.source`),
        target: textAt(initialPath.target, `${path}.initial_path.target`),
        label: textAt(initialPath.label, `${path}.initial_path.label`),
      },
    };
  }
  const intent = exactRecordAt(candidate, [
    "kind", "intent_version", "sem_generation", "label", "operands",
    "focal_relation", "outcome", "method", "hierarchy_policy",
  ], path);
  if (intent.kind !== "add_general_sem_interaction_v2"
    || intent.intent_version !== 1
    || intent.sem_generation !== "general_sem_v1"
    || intent.method !== "two_stage"
    || intent.hierarchy_policy !== "strong") {
    fail("schema6_general_sem_revision.intent_invalid", path, "Revision v1 requires the exact General SEM interaction-v2 intent." );
  }
  if (!Array.isArray(intent.operands) || intent.operands.length !== 2) {
    fail("schema6_general_sem_revision.operands_invalid", `${path}.operands`, "The interaction must have exactly two ordered operands.");
  }
  return {
    kind: "add_general_sem_interaction_v2",
    intent_version: 1,
    sem_generation: "general_sem_v1",
    label: textAt(intent.label, `${path}.label`),
    operands: [textAt(intent.operands[0], `${path}.operands[0]`), textAt(intent.operands[1], `${path}.operands[1]`)],
    focal_relation: textAt(intent.focal_relation, `${path}.focal_relation`),
    outcome: textAt(intent.outcome, `${path}.outcome`),
    method: "two_stage",
    hierarchy_policy: "strong",
  };
}

export function parseInternalGeneralSemExecutionAuthorityRevisionRequestV1(
  input: unknown,
): InternalGeneralSemExecutionAuthorityRevisionRequestV1 {
  const request = exactRecordAt(input, [
    "surface", "experimentalLabsEnabled", "sourceArchivePath",
    "expectedSourceArchiveSha256", "destinationArchivePath", "revision",
  ], "request");
  const validAccess = (request.surface === INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1
      && request.experimentalLabsEnabled === true)
    || (request.surface === STANDARD_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1
      && request.experimentalLabsEnabled === false);
  if (!validAccess) {
    fail("schema6_general_sem_revision.access_invalid", "request", "Revision requires one exact Standard or opted-in Labs access pair.");
  }
  const revision = exactRecordAt(request.revision, [
    "source", "revision", "intent", "expectedCapabilityCell", "recipeExecutionSurface",
  ], "request.revision");
  const source = exactRecordAt(revision.source, [
    "projectId", "modelId", "modelDocumentSha256", "modelScientificSha256",
    "recipeId", "recipeDocumentSha256",
  ], "request.revision.source");
  const identity = exactRecordAt(revision.revision, [
    "projectId", "projectName", "createdAt", "modelId", "modelName", "recipeId",
  ], "request.revision.revision");
  const parsed: InternalGeneralSemExecutionAuthorityRevisionRequestV1 = {
    surface: request.surface === STANDARD_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1
      ? STANDARD_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1
      : INTERNAL_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1,
    experimentalLabsEnabled: request.experimentalLabsEnabled as boolean,
    sourceArchivePath: textAt(request.sourceArchivePath, "request.sourceArchivePath"),
    expectedSourceArchiveSha256: shaAt(request.expectedSourceArchiveSha256, "request.expectedSourceArchiveSha256"),
    destinationArchivePath: textAt(request.destinationArchivePath, "request.destinationArchivePath"),
    revision: {
      source: {
        projectId: uuidAt(source.projectId, "request.revision.source.projectId"),
        modelId: textAt(source.modelId, "request.revision.source.modelId"),
        modelDocumentSha256: shaAt(source.modelDocumentSha256, "request.revision.source.modelDocumentSha256"),
        modelScientificSha256: shaAt(source.modelScientificSha256, "request.revision.source.modelScientificSha256"),
        recipeId: uuidAt(source.recipeId, "request.revision.source.recipeId"),
        recipeDocumentSha256: shaAt(source.recipeDocumentSha256, "request.revision.source.recipeDocumentSha256"),
      },
      revision: {
        projectId: uuidAt(identity.projectId, "request.revision.revision.projectId"),
        projectName: textAt(identity.projectName, "request.revision.revision.projectName"),
        createdAt: timestampAt(identity.createdAt, "request.revision.revision.createdAt"),
        modelId: textAt(identity.modelId, "request.revision.revision.modelId"),
        modelName: textAt(identity.modelName, "request.revision.revision.modelName"),
        recipeId: uuidAt(identity.recipeId, "request.revision.revision.recipeId"),
      },
      intent: parseIntent(revision.intent, "request.revision.intent"),
      expectedCapabilityCell: parseCapabilityCell(
        revision.expectedCapabilityCell,
        "request.revision.expectedCapabilityCell",
      ),
      recipeExecutionSurface: revision.recipeExecutionSurface === GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
        ? GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1
        : revision.recipeExecutionSurface === GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1
          ? GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1
          : fail(
            "schema6_general_sem_revision.execution_surface_invalid",
            "request.revision.recipeExecutionSurface",
            "Revision recipeExecutionSurface must be one frozen General SEM v1 identity.",
          ),
    },
  };
  const expectedRecipeSurface = parsed.surface === STANDARD_GENERAL_SEM_EXECUTION_AUTHORITY_REVISION_SURFACE_V1
    ? GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1
    : GENERAL_SEM_PLS_LABS_RECIPE_EXECUTION_SURFACE_V1;
  if (parsed.revision.recipeExecutionSurface !== expectedRecipeSurface) {
    fail(
      "schema6_general_sem_revision.execution_surface_mismatch",
      "request.revision.recipeExecutionSurface",
      "Revision recipe execution metadata must match its selected Registry surface.",
    );
  }
  const higherOrderIntent = parsed.revision.intent.kind === "add_higher_order"
    || parsed.revision.intent.kind === "replace_higher_order";
  const allowedExecutionCells = higherOrderIntent
    ? [
      GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1,
      GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_CELL_V1,
    ]
    : GENERAL_SEM_REVISION_EXECUTION_CELLS_V1;
  if (!allowedExecutionCells.some((cell) => (
    sameCapabilityCell(parsed.revision.expectedCapabilityCell, cell)
  ))) {
    fail(
      "schema6_general_sem_revision.capability_invalid",
      "request.revision.expectedCapabilityCell",
      "The revision requires the exact point or supplemental bootstrap cell for its resulting scientific authority.",
    );
  }
  if (parsed.sourceArchivePath.toLocaleLowerCase() === parsed.destinationArchivePath.toLocaleLowerCase()) {
    fail("schema6_general_sem_revision.new_destination_required", "request.destinationArchivePath", "Revision requires a new destination path.");
  }
  if (parsed.revision.source.projectId === parsed.revision.revision.projectId
    || parsed.revision.source.modelId === parsed.revision.revision.modelId
    || parsed.revision.source.recipeId === parsed.revision.revision.recipeId) {
    fail("schema6_general_sem_revision.new_identity_required", "request.revision.revision", "Project, model, and recipe revision identities must all be new.");
  }
  return parsed;
}

function parseCapabilityCell(value: unknown, path: string): CapabilityCellReferenceV2 {
  const cell = exactRecordAt(value, [
    "registry_schema_version", "capability_id", "cell_id", "capability_version",
  ], path);
  if (cell.registry_schema_version !== 2) fail("schema6_general_sem_revision.capability_invalid", path, "Capability registry schema must equal 2.");
  return {
    registry_schema_version: 2,
    capability_id: textAt(cell.capability_id, `${path}.capability_id`),
    cell_id: textAt(cell.cell_id, `${path}.cell_id`),
    capability_version: textAt(cell.capability_version, `${path}.capability_version`),
  };
}

function sameCapabilityCell(
  left: CapabilityCellReferenceV2,
  right: CapabilityCellReferenceV2,
): boolean {
  return left.registry_schema_version === right.registry_schema_version
    && left.capability_id === right.capability_id
    && left.cell_id === right.cell_id
    && left.capability_version === right.capability_version;
}

const GENERAL_SEM_REVISION_EXECUTION_CELLS_V1: readonly CapabilityCellReferenceV2[] = [
  GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1,
  GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_CELL_V1,
];

function primaryCapabilityCellForExecutionV1(
  executionCell: CapabilityCellReferenceV2,
): CapabilityCellReferenceV2 {
  if (sameCapabilityCell(executionCell, GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1)
    || sameCapabilityCell(executionCell, GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_CELL_V1)) {
    return GENERAL_SEM_PLS_HIGHER_ORDER_POINT_CAPABILITY_CELL_V1;
  }
  if (sameCapabilityCell(executionCell, GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1)
    || sameCapabilityCell(executionCell, GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1)) {
    return GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_CAPABILITY_CELL_V1;
  }
  if (sameCapabilityCell(executionCell, GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1)
    || sameCapabilityCell(executionCell, GENERAL_SEM_PLS_MODERATION_BOOTSTRAP_CAPABILITY_CELL_V1)
    || sameCapabilityCell(executionCell, GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1)) {
    return GENERAL_SEM_PLS_MODERATION_POINT_CAPABILITY_CELL_V1;
  }
  return GENERAL_SEM_PLS_POINT_CAPABILITY_CELL_V1;
}

function revisionReceiptIdentityV1(
  intent: GeneralSemExecutionAuthorityRevisionEditorIntentV1,
): { termId: string; outputId: string } {
  switch (intent.kind) {
    case "add_higher_order":
    case "replace_higher_order":
    case "replace_moderating_effect":
    case "remove_moderating_effect":
      return { termId: intent.term_id, outputId: intent.output_id };
    case "add_general_sem_interaction_v2": {
      const termId = standardSemGeneralSemInteractionV2TermIdV1(
        intent.focal_relation,
        intent.operands[0],
        intent.operands[1],
      );
      return { termId, outputId: standardSemGeneralSemInteractionV2OutputIdV1(termId) };
    }
    case "add_moderating_effect_v3": {
      const termId = intent.target.kind === "parent_interaction"
        ? standardSemGeneralSemThreeWayInteractionTermIdV1(
          intent.target.interactionTermId,
          intent.operands[2]!,
        )
        : standardSemGeneralSemInteractionV2TermIdV1(
          intent.target.relationId,
          intent.operands[0],
          intent.operands[1],
        );
      return { termId, outputId: standardSemGeneralSemInteractionV2OutputIdV1(termId) };
    }
  }
}

function parseReceipt(
  value: unknown,
  request: InternalGeneralSemExecutionAuthorityRevisionRequestV1,
): GeneralSemExecutionAuthorityRevisionReceiptV1 {
  const path = "outcome.value.receipt";
  const fields = [
    "schemaVersion", "archiveSchemaVersion", "revisionNumber", "sourceArchivePath",
    "sourceArchiveSha256", "sourceArchiveBytes", "sourceVerifiedUnchanged", "sourceProjectId",
    "sourceModelId", "sourceModelDocumentSha256", "sourceModelScientificSha256", "sourceRecipeId",
    "sourceRecipeDocumentSha256", "destinationArchivePath", "destinationArchiveSha256",
    "destinationArchiveBytes", "strictReopenValidated", "projectId", "name", "createdAt",
    "residentDatasetId", "residentDatasetFingerprint", "residentModelId",
    "residentModelDocumentSha256", "residentModelScientificSha256", "residentRecipeId",
    "residentRecipeDocumentSha256", "compilerVersion", "capabilityCell",
    "recipeAnalyticalSha256", "generalSemConfigSha256", "compiledPlanSha256",
    "compiledArtifactIdentitySha256", "interactionTermId", "interactionOutputId",
  ] as const;
  const receipt = exactRecordAt(value, fields, path);
  if (receipt.schemaVersion !== 1 || receipt.archiveSchemaVersion !== 6
    || receipt.sourceVerifiedUnchanged !== true || receipt.strictReopenValidated !== true) {
    fail("schema6_general_sem_revision.receipt_invalid", path, "Receipt must prove schema-6, source stability, and strict reopen.");
  }
  const parsed: GeneralSemExecutionAuthorityRevisionReceiptV1 = {
    schemaVersion: 1,
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
    capabilityCell: parseCapabilityCell(receipt.capabilityCell, `${path}.capabilityCell`),
    recipeAnalyticalSha256: shaAt(receipt.recipeAnalyticalSha256, `${path}.recipeAnalyticalSha256`),
    generalSemConfigSha256: shaAt(receipt.generalSemConfigSha256, `${path}.generalSemConfigSha256`),
    compiledPlanSha256: shaAt(receipt.compiledPlanSha256, `${path}.compiledPlanSha256`),
    compiledArtifactIdentitySha256: shaAt(receipt.compiledArtifactIdentitySha256, `${path}.compiledArtifactIdentitySha256`),
    interactionTermId: textAt(receipt.interactionTermId, `${path}.interactionTermId`),
    interactionOutputId: textAt(receipt.interactionOutputId, `${path}.interactionOutputId`),
  };
  const { source, revision, intent } = request.revision;
  const { termId, outputId } = revisionReceiptIdentityV1(intent);
  const expectedPrimaryCell = primaryCapabilityCellForExecutionV1(
    request.revision.expectedCapabilityCell,
  );
  if (parsed.sourceArchivePath !== request.sourceArchivePath
    || parsed.sourceArchiveSha256 !== request.expectedSourceArchiveSha256
    || parsed.sourceProjectId !== source.projectId
    || parsed.sourceModelId !== source.modelId
    || parsed.sourceModelDocumentSha256 !== source.modelDocumentSha256
    || parsed.sourceModelScientificSha256 !== source.modelScientificSha256
    || parsed.sourceRecipeId !== source.recipeId
    || parsed.sourceRecipeDocumentSha256 !== source.recipeDocumentSha256
    || parsed.destinationArchivePath !== request.destinationArchivePath
    || parsed.projectId !== revision.projectId
    || parsed.name !== revision.projectName
    || Date.parse(parsed.createdAt) !== Date.parse(revision.createdAt)
    || parsed.residentModelId !== revision.modelId
    || parsed.residentRecipeId !== revision.recipeId
    // Compilation receipts retain the point-primary cell. A bootstrap
    // revision is authorized by its supplemental execution cell in the
    // request, without falsely relabelling the compiled point authority.
    || !sameCapabilityCell(parsed.capabilityCell, expectedPrimaryCell)
    || parsed.interactionTermId !== termId
    || parsed.interactionOutputId !== outputId) {
    fail("schema6_general_sem_revision.receipt_request_mismatch", path, "Native revision receipt differs from the exact pinned request.");
  }
  return parsed;
}

export function parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1(
  input: unknown,
  request: InternalGeneralSemExecutionAuthorityRevisionRequestV1,
): InternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1 {
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
  if (outcome.status !== "ok") fail("schema6_general_sem_revision.status_invalid", "outcome.status", "Revision outcome must be ok or blocked.");
  exactRecordAt(outcome, ["status", "value"], "outcome");
  const value = exactRecordAt(outcome.value, ["schemaVersion", "persistence", "receipt"], "outcome.value");
  if (value.schemaVersion !== 1 || value.persistence !== "persisted_new_revision") {
    fail("schema6_general_sem_revision.result_invalid", "outcome.value", "Revision result contract is invalid.");
  }
  return { status: "ok", value: {
    schemaVersion: 1,
    persistence: "persisted_new_revision",
    receipt: parseReceipt(value.receipt, request),
  } };
}
