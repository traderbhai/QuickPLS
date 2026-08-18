import {
  parseSemModelV4AuthoringDraft,
  validateSemModelV4,
  type SemConstraintV4,
  type SemDataBindingV4,
  type SemDerivedTermV4,
  type SemGroupV4,
  type SemModelV4,
  type SemModelV4Issue,
  type SemParameterV4,
  type SemRelationV4,
  type SemVariableV4,
} from "./semModelV4";

export const SEM_MODEL_V4_AUTHORITY_OPERATION_BATCH_VERSION = 1 as const;
export const SEM_MODEL_V4_AUTHORITY_OPERATION_LIMIT = 512 as const;

export const SEM_MODEL_V4_AUTHORITY_OPERATION_UNSUPPORTED_ACTIONS = [
  {
    action: "delete_or_reorder",
    correctiveAction: "Use exact full-model JSON replacement when an object must be deleted or scientific ordering must change.",
  },
  {
    action: "change_model_identity",
    correctiveAction: "Insert a new draft with a new model id; an operation batch cannot change the selected draft identity.",
  },
  {
    action: "edit_annotations_or_presentation",
    correctiveAction: "Use exact full-model JSON replacement for annotations or canvas presentation; neither is scientific authority in this operation lane.",
  },
] as const;

export type SemModelV4AuthorityOperationV1 =
  | { kind: "append_variable"; variable: SemVariableV4 }
  | { kind: "replace_variable"; variable_id: string; replacement: SemVariableV4 }
  | { kind: "append_relation"; relation: SemRelationV4 }
  | { kind: "append_parameter"; parameter: SemParameterV4 }
  | { kind: "replace_parameter"; parameter_id: string; replacement: SemParameterV4 }
  | { kind: "append_constraint"; constraint: SemConstraintV4 }
  | { kind: "append_derived_term"; term: SemDerivedTermV4 }
  | { kind: "set_group"; group: SemGroupV4 }
  | { kind: "set_data_binding"; data_binding: SemDataBindingV4 };

export interface SemModelV4AuthorityOperationBatchV1 {
  schema_version: typeof SEM_MODEL_V4_AUTHORITY_OPERATION_BATCH_VERSION;
  expected_model_id: string;
  operations: SemModelV4AuthorityOperationV1[];
}

export interface SemModelV4AuthorityOperationResultV1 {
  model: SemModelV4;
  readiness: "ready" | "draft";
  readiness_issues: SemModelV4Issue[];
}

export class SemModelV4AuthorityOperationError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
    public readonly correctiveAction: string,
  ) {
    super(message);
    this.name = "SemModelV4AuthorityOperationError";
  }
}

type UnknownRecord = Record<string, unknown>;

function fail(code: string, path: string, message: string, correctiveAction: string): never {
  throw new SemModelV4AuthorityOperationError(code, path, message, correctiveAction);
}

function recordAt(value: unknown, path: string): UnknownRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    fail(
      "sem_model_v4.authority_operation.object_required",
      path,
      `${path} must be an object.`,
      "Use the documented exact operation-batch object shape.",
    );
  }
  return value as UnknownRecord;
}

function exactRecordAt(value: unknown, required: readonly string[], path: string): UnknownRecord {
  const record = recordAt(value, path);
  const allowed = new Set(required);
  for (const key of required) {
    if (!Object.prototype.hasOwnProperty.call(record, key)) {
      fail(
        "sem_model_v4.authority_operation.field_missing",
        `${path}.${key}`,
        `${path}.${key} is required.`,
        "Add the missing field without adding undocumented fields.",
      );
    }
  }
  for (const key of Object.keys(record)) {
    if (!allowed.has(key)) {
      fail(
        "sem_model_v4.authority_operation.field_unknown",
        `${path}.${key}`,
        `${path}.${key} is not part of the operation contract.`,
        "Remove the unknown field or use exact full-model JSON replacement for unsupported edits.",
      );
    }
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    fail(
      "sem_model_v4.authority_operation.text_required",
      path,
      `${path} must be a nonempty string.`,
      "Provide the exact stable model or object id.",
    );
  }
  return value;
}

function parseOperation(value: unknown, path: string): SemModelV4AuthorityOperationV1 {
  const candidate = recordAt(value, path);
  switch (candidate.kind) {
    case "append_variable": {
      const operation = exactRecordAt(candidate, ["kind", "variable"], path);
      return { kind: "append_variable", variable: operation.variable as SemVariableV4 };
    }
    case "replace_variable": {
      const operation = exactRecordAt(candidate, ["kind", "variable_id", "replacement"], path);
      return {
        kind: "replace_variable",
        variable_id: textAt(operation.variable_id, `${path}.variable_id`),
        replacement: operation.replacement as SemVariableV4,
      };
    }
    case "append_relation": {
      const operation = exactRecordAt(candidate, ["kind", "relation"], path);
      return { kind: "append_relation", relation: operation.relation as SemRelationV4 };
    }
    case "append_parameter": {
      const operation = exactRecordAt(candidate, ["kind", "parameter"], path);
      return { kind: "append_parameter", parameter: operation.parameter as SemParameterV4 };
    }
    case "replace_parameter": {
      const operation = exactRecordAt(candidate, ["kind", "parameter_id", "replacement"], path);
      return {
        kind: "replace_parameter",
        parameter_id: textAt(operation.parameter_id, `${path}.parameter_id`),
        replacement: operation.replacement as SemParameterV4,
      };
    }
    case "append_constraint": {
      const operation = exactRecordAt(candidate, ["kind", "constraint"], path);
      return { kind: "append_constraint", constraint: operation.constraint as SemConstraintV4 };
    }
    case "append_derived_term": {
      const operation = exactRecordAt(candidate, ["kind", "term"], path);
      return { kind: "append_derived_term", term: operation.term as SemDerivedTermV4 };
    }
    case "set_group": {
      const operation = exactRecordAt(candidate, ["kind", "group"], path);
      return { kind: "set_group", group: operation.group as SemGroupV4 };
    }
    case "set_data_binding": {
      const operation = exactRecordAt(candidate, ["kind", "data_binding"], path);
      return { kind: "set_data_binding", data_binding: operation.data_binding as SemDataBindingV4 };
    }
    default:
      return fail(
        "sem_model_v4.authority_operation.kind_unsupported",
        `${path}.kind`,
        `Unsupported canonical authority operation ${String(candidate.kind)}.`,
        "Use one of the documented append, replace, group, or data-binding operations; use exact full-model JSON replacement for deletion, reordering, annotations, or presentation.",
      );
  }
}

export function parseSemModelV4AuthorityOperationBatchV1(
  input: unknown,
): SemModelV4AuthorityOperationBatchV1 {
  const batch = exactRecordAt(input, ["schema_version", "expected_model_id", "operations"], "batch");
  if (batch.schema_version !== SEM_MODEL_V4_AUTHORITY_OPERATION_BATCH_VERSION) {
    fail(
      "sem_model_v4.authority_operation.schema_version_unsupported",
      "batch.schema_version",
      `Operation batches must use schema version ${SEM_MODEL_V4_AUTHORITY_OPERATION_BATCH_VERSION}.`,
      "Set schema_version to 1 and review the operation contract before retrying.",
    );
  }
  if (!Array.isArray(batch.operations) || batch.operations.length === 0) {
    fail(
      "sem_model_v4.authority_operation.operations_required",
      "batch.operations",
      "An operation batch must contain at least one operation.",
      "Add one or more exact canonical operations.",
    );
  }
  if (batch.operations.length > SEM_MODEL_V4_AUTHORITY_OPERATION_LIMIT) {
    fail(
      "sem_model_v4.authority_operation.limit_exceeded",
      "batch.operations",
      `An operation batch cannot exceed ${SEM_MODEL_V4_AUTHORITY_OPERATION_LIMIT} operations.`,
      "Split the work into smaller CAS-protected batches.",
    );
  }
  return {
    schema_version: SEM_MODEL_V4_AUTHORITY_OPERATION_BATCH_VERSION,
    expected_model_id: textAt(batch.expected_model_id, "batch.expected_model_id"),
    operations: batch.operations.map((operation, index) => parseOperation(operation, `batch.operations[${index}]`)),
  };
}

export function parseSemModelV4AuthorityOperationBatchJsonV1(
  json: string,
): SemModelV4AuthorityOperationBatchV1 {
  if (!json.trim()) {
    fail(
      "sem_model_v4.authority_operation.json_required",
      "json",
      "Paste one canonical SemModelV4 authority-operation batch.",
      "Provide a JSON object with schema_version, expected_model_id, and operations.",
    );
  }
  let decoded: unknown;
  try {
    decoded = JSON.parse(json);
  } catch {
    fail(
      "sem_model_v4.authority_operation.json_invalid",
      "json",
      "The operation batch is not valid JSON.",
      "Correct the JSON syntax and retry.",
    );
  }
  return parseSemModelV4AuthorityOperationBatchV1(decoded);
}

function replaceById<T>(
  values: T[],
  id: string,
  replacement: T,
  idOf: (value: T) => string,
  path: string,
): void {
  const index = values.findIndex((value) => idOf(value) === id);
  if (index < 0) {
    fail(
      "sem_model_v4.authority_operation.target_missing",
      path,
      `The requested replacement target ${id} does not exist in the selected draft.`,
      "Refresh the draft authority and use an existing exact object id.",
    );
  }
  if (idOf(replacement) !== id) {
    fail(
      "sem_model_v4.authority_operation.target_identity_mismatch",
      path,
      `Replacement identity must remain ${id}.`,
      "Keep the existing object id or append a separate object with a new id.",
    );
  }
  values[index] = replacement;
}

/**
 * Applies a bounded operation batch to a cloned SemModelV4 authority document.
 * The selected model is never mutated. The entire batch is committed only if
 * the existing strict authoring-integrity decoder accepts the final document.
 */
export function applySemModelV4AuthorityOperationBatchV1(
  source: SemModelV4,
  batch: SemModelV4AuthorityOperationBatchV1,
): SemModelV4AuthorityOperationResultV1 {
  if (batch.expected_model_id !== source.id) {
    fail(
      "sem_model_v4.authority_operation.model_id_mismatch",
      "batch.expected_model_id",
      `The batch expects ${batch.expected_model_id}, but the selected draft is ${source.id}.`,
      "Refresh the selected draft and regenerate the batch with its exact model id.",
    );
  }
  const candidate = JSON.parse(JSON.stringify(source)) as SemModelV4;
  batch.operations.forEach((operation, index) => {
    const path = `batch.operations[${index}]`;
    switch (operation.kind) {
      case "append_variable":
        candidate.variables.push(operation.variable);
        break;
      case "replace_variable":
        replaceById(candidate.variables, operation.variable_id, operation.replacement, (value) => value.id, `${path}.variable_id`);
        break;
      case "append_relation":
        candidate.relations.push(operation.relation);
        break;
      case "append_parameter":
        candidate.parameters.push(operation.parameter);
        break;
      case "replace_parameter":
        replaceById(candidate.parameters, operation.parameter_id, operation.replacement, (value) => value.id, `${path}.parameter_id`);
        break;
      case "append_constraint":
        candidate.constraints.push(operation.constraint);
        break;
      case "append_derived_term":
        candidate.derived_terms.push(operation.term);
        break;
      case "set_group":
        candidate.group = operation.group;
        break;
      case "set_data_binding":
        candidate.data_binding = operation.data_binding;
        break;
    }
  });
  const model = parseSemModelV4AuthoringDraft(candidate);
  const readinessIssues = validateSemModelV4(model);
  return {
    model,
    readiness: readinessIssues.length ? "draft" : "ready",
    readiness_issues: readinessIssues,
  };
}
