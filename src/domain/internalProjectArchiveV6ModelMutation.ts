import {
  parseInternalProjectArchiveV6Wire,
  type InternalProjectArchiveV6Wire,
  type ProjectModelRecordV6Wire,
} from "./internalProjectArchiveV6Wire";
import {
  parseSemModelV4AuthoringDraft,
  type SemModelV4,
} from "./semModelV4";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
type WireRecord = Record<string, unknown>;

export const INTERNAL_PROJECT_ARCHIVE_V6_MODEL_MUTATION_SURFACE = "internal_labs" as const;

export type InternalProjectArchiveV6ModelMutationV1 =
  | { kind: "insert_draft"; draft: SemModelV4 }
  | {
    kind: "replace_draft";
    modelId: string;
    expectedModelDocumentSha256: string;
    replacement: SemModelV4;
  }
  | {
    kind: "promote_draft";
    modelId: string;
    expectedModelDocumentSha256: string;
  };

export interface InternalProjectArchiveV6ModelMutationRequestV1 {
  surface: typeof INTERNAL_PROJECT_ARCHIVE_V6_MODEL_MUTATION_SURFACE;
  experimentalLabsEnabled: true;
  project: InternalProjectArchiveV6Wire;
  mutation: InternalProjectArchiveV6ModelMutationV1;
}

export interface InternalProjectArchiveV6ModelMutationResultV1 {
  schemaVersion: 1;
  persistence: "not_persisted";
  project: InternalProjectArchiveV6Wire;
}

export interface InternalProjectArchiveV6ModelMutationDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
}

export type InternalProjectArchiveV6ModelMutationOutcomeV1 =
  | { status: "ok"; value: InternalProjectArchiveV6ModelMutationResultV1 }
  | { status: "blocked"; diagnostic: InternalProjectArchiveV6ModelMutationDiagnosticV1 };

export class InternalProjectArchiveV6ModelMutationWireError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6ModelMutationWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalProjectArchiveV6ModelMutationWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("schema6_model_mutation.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(
  value: unknown,
  required: readonly string[],
  path: string,
): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(required);
  for (const key of required) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      fail("schema6_model_mutation.field_missing", `${path}.${key}`, `${path}.${key} is required.`);
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail("schema6_model_mutation.field_unknown", `${path}.${key}`, `${path}.${key} is not part of the model-mutation bridge.`);
    }
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    fail("schema6_model_mutation.text_required", path, `${path} must be a nonempty string.`);
  }
  return value;
}

function sha256At(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) {
    fail("schema6_model_mutation.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  }
  return digest;
}

function parseMutation(value: unknown): InternalProjectArchiveV6ModelMutationV1 {
  const candidate = recordAt(value, "request.mutation");
  if (candidate.kind === "insert_draft") {
    const mutation = exactRecordAt(candidate, ["kind", "draft"], "request.mutation");
    return {
      kind: "insert_draft",
      draft: parseSemModelV4AuthoringDraft(mutation.draft),
    };
  }
  if (candidate.kind === "replace_draft") {
    const mutation = exactRecordAt(candidate, [
      "kind",
      "modelId",
      "expectedModelDocumentSha256",
      "replacement",
    ], "request.mutation");
    return {
      kind: "replace_draft",
      modelId: textAt(mutation.modelId, "request.mutation.modelId"),
      expectedModelDocumentSha256: sha256At(
        mutation.expectedModelDocumentSha256,
        "request.mutation.expectedModelDocumentSha256",
      ),
      replacement: parseSemModelV4AuthoringDraft(mutation.replacement),
    };
  }
  if (candidate.kind === "promote_draft") {
    const mutation = exactRecordAt(candidate, [
      "kind",
      "modelId",
      "expectedModelDocumentSha256",
    ], "request.mutation");
    return {
      kind: "promote_draft",
      modelId: textAt(mutation.modelId, "request.mutation.modelId"),
      expectedModelDocumentSha256: sha256At(
        mutation.expectedModelDocumentSha256,
        "request.mutation.expectedModelDocumentSha256",
      ),
    };
  }
  return fail(
    "schema6_model_mutation.kind_invalid",
    "request.mutation.kind",
    "The mutation kind must be insert_draft, replace_draft, or promote_draft.",
  );
}

export function parseInternalProjectArchiveV6ModelMutationRequestV1(
  input: unknown,
): InternalProjectArchiveV6ModelMutationRequestV1 {
  const request = exactRecordAt(input, [
    "surface",
    "experimentalLabsEnabled",
    "project",
    "mutation",
  ], "request");
  if (request.surface !== INTERNAL_PROJECT_ARCHIVE_V6_MODEL_MUTATION_SURFACE
    || request.experimentalLabsEnabled !== true) {
    fail(
      "schema6_model_mutation.internal_labs_required",
      "request.surface",
      "Schema-6 model mutation is restricted to the internal Experimental Labs boundary.",
    );
  }
  return {
    surface: INTERNAL_PROJECT_ARCHIVE_V6_MODEL_MUTATION_SURFACE,
    experimentalLabsEnabled: true,
    project: parseInternalProjectArchiveV6Wire(request.project),
    mutation: parseMutation(request.mutation),
  };
}

function canonicalJson(value: unknown, path = "value"): string {
  if (value === null) return "null";
  if (typeof value === "string" || typeof value === "boolean") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value) || Object.is(value, -0)) {
      fail("schema6_model_mutation.non_json_value", path, `${path} must contain only canonical JSON values.`);
    }
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item, index) => canonicalJson(item, `${path}[${index}]`)).join(",")}]`;
  }
  if (typeof value === "object") {
    const record = value as WireRecord;
    return `{${Object.keys(record).sort().map((key) => (
      `${JSON.stringify(key)}:${canonicalJson(record[key], `${path}.${key}`)}`
    )).join(",")}}`;
  }
  return fail("schema6_model_mutation.non_json_value", path, `${path} must contain only JSON values.`);
}

function equalWire(left: unknown, right: unknown): boolean {
  return canonicalJson(left) === canonicalJson(right);
}

function projectWithoutModels(project: InternalProjectArchiveV6Wire): Omit<InternalProjectArchiveV6Wire, "models"> {
  const { models: _models, ...nonModel } = project;
  return nonModel;
}

function assertUnchangedRecords(
  source: readonly ProjectModelRecordV6Wire[],
  mutated: readonly ProjectModelRecordV6Wire[],
  excludedIndex: number | null,
): void {
  source.forEach((record, index) => {
    if (index !== excludedIndex && !equalWire(record, mutated[index])) {
      fail(
        "schema6_model_mutation.unexpected_model_change",
        `outcome.value.project.models[${index}]`,
        "The native bridge changed a model outside the requested draft operation.",
      );
    }
  });
}

function assertExpectedModelMutation(
  request: InternalProjectArchiveV6ModelMutationRequestV1,
  mutated: InternalProjectArchiveV6Wire,
): void {
  const source = request.project;
  const mutation = request.mutation;
  if (!equalWire(projectWithoutModels(source), projectWithoutModels(mutated))) {
    fail(
      "schema6_model_mutation.non_model_change",
      "outcome.value.project",
      "The native bridge changed non-model schema-6 project fields.",
    );
  }

  if (mutation.kind === "insert_draft") {
    if (mutated.models.length !== source.models.length + 1) {
      fail("schema6_model_mutation.insert_shape", "outcome.value.project.models", "A successful insert must append exactly one draft model.");
    }
    assertUnchangedRecords(source.models, mutated.models, null);
    const inserted = mutated.models[mutated.models.length - 1];
    if (inserted.model_id !== mutation.draft.id
      || inserted.payload.kind !== "sem_model_v4_draft"
      || !equalWire(inserted.payload.model, mutation.draft)) {
      fail("schema6_model_mutation.insert_mismatch", "outcome.value.project.models", "The inserted draft differs from the requested SemModelV4 draft.");
    }
    return;
  }

  const targetIndex = source.models.findIndex((record) => record.model_id === mutation.modelId);
  if (targetIndex < 0 || mutated.models.length !== source.models.length) {
    fail("schema6_model_mutation.target_shape", "outcome.value.project.models", "A successful replace or promote must update exactly one existing model.");
  }
  assertUnchangedRecords(source.models, mutated.models, targetIndex);
  const target = mutated.models[targetIndex];
  if (target.model_id !== mutation.modelId) {
    fail("schema6_model_mutation.target_identity", `outcome.value.project.models[${targetIndex}]`, "The mutated model identity changed.");
  }
  if (mutation.kind === "replace_draft") {
    if (target.payload.kind !== "sem_model_v4_draft"
      || !equalWire(target.payload.model, mutation.replacement)) {
      fail("schema6_model_mutation.replacement_mismatch", `outcome.value.project.models[${targetIndex}]`, "The replacement draft differs from the requested SemModelV4 draft.");
    }
  } else {
    const sourceTarget = source.models[targetIndex];
    if (sourceTarget.payload.kind !== "sem_model_v4_draft"
      || target.payload.kind !== "sem_model_v4"
      || !equalWire(target.payload.model, sourceTarget.payload.model)) {
      fail("schema6_model_mutation.promotion_mismatch", `outcome.value.project.models[${targetIndex}]`, "Promotion must preserve the exact draft model while changing only its authority state.");
    }
  }
}

export function parseInternalProjectArchiveV6ModelMutationOutcomeV1(
  input: unknown,
  request: InternalProjectArchiveV6ModelMutationRequestV1,
): InternalProjectArchiveV6ModelMutationOutcomeV1 {
  const outcome = recordAt(input, "outcome");
  if (outcome.status === "ok") {
    exactRecordAt(outcome, ["status", "value"], "outcome");
    const value = exactRecordAt(outcome.value, [
      "schemaVersion",
      "persistence",
      "project",
    ], "outcome.value");
    if (value.schemaVersion !== 1) {
      fail("schema6_model_mutation.result_schema", "outcome.value.schemaVersion", "The mutation result must use contract version 1.");
    }
    if (value.persistence !== "not_persisted") {
      fail("schema6_model_mutation.persistence_forbidden", "outcome.value.persistence", "This bridge must return an explicitly non-persisted document.");
    }
    const project = parseInternalProjectArchiveV6Wire(value.project);
    assertExpectedModelMutation(request, project);
    return {
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project,
      },
    };
  }
  if (outcome.status === "blocked") {
    exactRecordAt(outcome, ["status", "diagnostic"], "outcome");
    const diagnostic = exactRecordAt(outcome.diagnostic, [
      "code",
      "message",
      "correctiveAction",
    ], "outcome.diagnostic");
    return {
      status: "blocked",
      diagnostic: {
        code: textAt(diagnostic.code, "outcome.diagnostic.code"),
        message: textAt(diagnostic.message, "outcome.diagnostic.message"),
        correctiveAction: textAt(diagnostic.correctiveAction, "outcome.diagnostic.correctiveAction"),
      },
    };
  }
  return fail(
    "schema6_model_mutation.status_invalid",
    "outcome.status",
    "The schema-6 model-mutation outcome status must be ok or blocked.",
  );
}
