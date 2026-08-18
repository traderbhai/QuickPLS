import {
  parseSemWeightBindingV4,
  type SamplingWeightNormalizationV4,
  type SemModelV4,
  validateSemModelV4,
} from "./semModelV4";

/**
 * Scientific contract only. `NativeCanonicalModelSpec` has no model-level
 * SemModelV4 data-binding field, while `modelPresentations` is layout state.
 * A product authoring control must remain absent until canonical project
 * persistence can save and reopen this declaration without a second authority.
 */

export const SEM_WEIGHT_DECLARATION_CONTRACT_VERSION_V1 = "sem_weight_declaration_v1" as const;

export type ResolvedWeightBindingV1 =
  | { kind: "case"; variable_id: string; source_column: string }
  | { kind: "frequency"; variable_id: string; source_column: string }
  | {
      kind: "sampling";
      variable_id: string;
      source_column: string;
      normalization: SamplingWeightNormalizationV4;
    };

export interface ResolvedWeightDeclarationV1 {
  contract_version: typeof SEM_WEIGHT_DECLARATION_CONTRACT_VERSION_V1;
  dataset_id: string;
  binding: ResolvedWeightBindingV1;
}

export type WeightCapabilityTargetV1 =
  | "pls_plan_v2"
  | "cbsem_ml_v1"
  | "cbsem_ml_mean_replacement_v1"
  | "cbsem_product_indicator_plan_v1";

export type WeightCapabilityIssueCodeV1 =
  | "case_weight_unsupported"
  | "frequency_weight_unsupported"
  | "sampling_weight_unsupported"
  | "sampling_weight_normalization_unsupported"
  | "legacy_case_weight_binding_ambiguous";

export type WeightCapabilityIssueV1 =
  | {
      code: Exclude<WeightCapabilityIssueCodeV1, "legacy_case_weight_binding_ambiguous">;
      target: WeightCapabilityTargetV1;
      declaration: ResolvedWeightDeclarationV1;
      subject: string;
      corrective_action: string;
    }
  | {
      code: "legacy_case_weight_binding_ambiguous";
      target: WeightCapabilityTargetV1;
      declaration: ResolvedWeightDeclarationV1 | null;
      subject: string;
      corrective_action: string;
    };

export class SemWeightDeclarationV1Error extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
  ) {
    super(message);
    this.name = "SemWeightDeclarationV1Error";
  }
}

const CASE_CORRECTIVE_ACTION = "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted.";
const FREQUENCY_CORRECTIVE_ACTION = "Remove the frequency-weight binding or choose an estimator that explicitly supports frequency weights; no executable plan was emitted.";
const SAMPLING_CORRECTIVE_ACTION = "Remove the sampling-weight binding or choose an estimator with explicit sampling-design support; no executable plan was emitted.";
const NORMALIZATION_CORRECTIVE_ACTION = "Choose a supported sampling-weight normalization or remove the sampling-weight binding; no executable plan was emitted.";

type UnknownRecord = Record<string, unknown>;

function record(value: unknown, path: string): UnknownRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new SemWeightDeclarationV1Error("schema.invalid_shape", path, `${path} must be an object`);
  }
  return value as UnknownRecord;
}

function exactRecord(value: unknown, allowed: readonly string[], path: string): UnknownRecord {
  const candidate = record(value, path);
  const unknown = Object.keys(candidate).find((key) => !allowed.includes(key));
  if (unknown) throw new SemWeightDeclarationV1Error("schema.unknown_field", `${path}.${unknown}`, `Unknown field ${path}.${unknown}`);
  return candidate;
}

function requiredText(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    throw new SemWeightDeclarationV1Error("schema.invalid_shape", path, `${path} must be a non-empty string`);
  }
  return value;
}

function requiredExactText(value: unknown, path: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new SemWeightDeclarationV1Error("schema.invalid_shape", path, `${path} must be a non-empty string`);
  }
  return value;
}

function parseNormalization(value: unknown, path: string): SamplingWeightNormalizationV4 {
  if (value !== "none" && value !== "mean_one" && value !== "sum_to_sample_size") {
    throw new SemWeightDeclarationV1Error("schema.invalid_discriminator", path, `${path} is not a supported normalization`);
  }
  return value;
}

function parseResolvedBinding(value: unknown, path: string): ResolvedWeightBindingV1 {
  const candidate = record(value, path);
  if (candidate.kind === "case" || candidate.kind === "frequency") {
    exactRecord(candidate, ["kind", "variable_id", "source_column"], path);
    return {
      kind: candidate.kind,
      variable_id: requiredText(candidate.variable_id, `${path}.variable_id`),
      source_column: requiredText(candidate.source_column, `${path}.source_column`),
    };
  }
  if (candidate.kind === "sampling") {
    exactRecord(candidate, ["kind", "variable_id", "source_column", "normalization"], path);
    return {
      kind: "sampling",
      variable_id: requiredText(candidate.variable_id, `${path}.variable_id`),
      source_column: requiredText(candidate.source_column, `${path}.source_column`),
      normalization: parseNormalization(candidate.normalization, `${path}.normalization`),
    };
  }
  throw new SemWeightDeclarationV1Error("schema.invalid_discriminator", `${path}.kind`, `${path}.kind must be case, frequency, or sampling`);
}

export function parseResolvedWeightDeclarationV1(value: unknown): ResolvedWeightDeclarationV1 {
  const candidate = exactRecord(value, ["contract_version", "dataset_id", "binding"], "weight_declaration");
  if (candidate.contract_version !== SEM_WEIGHT_DECLARATION_CONTRACT_VERSION_V1) {
    throw new SemWeightDeclarationV1Error("schema.version", "weight_declaration.contract_version", "Weight declaration contract version is unsupported");
  }
  if (!("binding" in candidate)) {
    throw new SemWeightDeclarationV1Error("schema.invalid_shape", "weight_declaration.binding", "weight_declaration.binding is required");
  }
  return {
    contract_version: SEM_WEIGHT_DECLARATION_CONTRACT_VERSION_V1,
    dataset_id: requiredText(candidate.dataset_id, "weight_declaration.dataset_id"),
    binding: parseResolvedBinding(candidate.binding, "weight_declaration.binding"),
  };
}

export function parseWeightCapabilityIssueV1(value: unknown): WeightCapabilityIssueV1 {
  const candidate = exactRecord(value, ["code", "target", "declaration", "subject", "corrective_action"], "weight_issue");
  const codes: readonly WeightCapabilityIssueCodeV1[] = [
    "case_weight_unsupported",
    "frequency_weight_unsupported",
    "sampling_weight_unsupported",
    "sampling_weight_normalization_unsupported",
    "legacy_case_weight_binding_ambiguous",
  ];
  const targets: readonly WeightCapabilityTargetV1[] = [
    "pls_plan_v2",
    "cbsem_ml_v1",
    "cbsem_ml_mean_replacement_v1",
    "cbsem_product_indicator_plan_v1",
  ];
  if (!codes.includes(candidate.code as WeightCapabilityIssueCodeV1)) {
    throw new SemWeightDeclarationV1Error("schema.invalid_discriminator", "weight_issue.code", "Weight capability issue code is unsupported");
  }
  if (!targets.includes(candidate.target as WeightCapabilityTargetV1)) {
    throw new SemWeightDeclarationV1Error("schema.invalid_discriminator", "weight_issue.target", "Weight capability target is unsupported");
  }
  const code = candidate.code as WeightCapabilityIssueCodeV1;
  const hasDeclaration = Object.prototype.hasOwnProperty.call(candidate, "declaration");
  if (!hasDeclaration) {
    throw new SemWeightDeclarationV1Error("schema.invalid_shape", "weight_issue.declaration", "Weight capability diagnostics require an explicit nullable declaration");
  }
  const declaration = candidate.declaration === null
    ? null
    : parseResolvedWeightDeclarationV1(candidate.declaration);
  if (code !== "legacy_case_weight_binding_ambiguous" && declaration === null) {
    throw new SemWeightDeclarationV1Error("schema.invalid_shape", "weight_issue.declaration", "Unsupported-weight diagnostics require their resolved declaration");
  }
  const subject = code === "legacy_case_weight_binding_ambiguous"
    ? requiredExactText(candidate.subject, "weight_issue.subject")
    : requiredText(candidate.subject, "weight_issue.subject");
  const correctiveAction = requiredText(candidate.corrective_action, "weight_issue.corrective_action");
  if (code !== "legacy_case_weight_binding_ambiguous") {
    const expectedKind = code === "case_weight_unsupported" ? "case"
      : code === "frequency_weight_unsupported" ? "frequency"
        : "sampling";
    if (declaration!.binding.kind !== expectedKind) {
      throw new SemWeightDeclarationV1Error("schema.identity_mismatch", "weight_issue.code", "Weight issue code does not match the resolved binding kind");
    }
    if (subject !== declaration!.binding.variable_id) {
      throw new SemWeightDeclarationV1Error("schema.identity_mismatch", "weight_issue.subject", "Weight issue subject does not match the resolved variable id");
    }
  } else if (declaration?.binding.kind === "case" && declaration.binding.source_column === subject) {
    throw new SemWeightDeclarationV1Error("schema.identity_mismatch", "weight_issue.code", "An exact legacy case binding cannot be reported as ambiguous");
  }
  const expectedCorrectiveAction = code === "case_weight_unsupported" ? CASE_CORRECTIVE_ACTION
    : code === "frequency_weight_unsupported" ? FREQUENCY_CORRECTIVE_ACTION
      : code === "sampling_weight_unsupported" ? SAMPLING_CORRECTIVE_ACTION
        : code === "sampling_weight_normalization_unsupported" ? NORMALIZATION_CORRECTIVE_ACTION
          : `Legacy settings.case_weight_column '${subject}' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.`;
  if (correctiveAction !== expectedCorrectiveAction) {
    throw new SemWeightDeclarationV1Error("schema.identity_mismatch", "weight_issue.corrective_action", "Weight issue corrective action does not match its typed code and subject");
  }
  const common = {
    target: candidate.target as WeightCapabilityTargetV1,
    subject,
    corrective_action: correctiveAction,
  };
  return code === "legacy_case_weight_binding_ambiguous"
    ? { code, ...common, declaration }
    : { code, ...common, declaration: declaration! };
}

/** Resolves a validated raw SemModelV4 binding to stable variable and source-column identities. */
export function resolveSemWeightDeclarationV1(model: SemModelV4): ResolvedWeightDeclarationV1 | null {
  const modelIssues = validateSemModelV4(model);
  if (modelIssues.length) {
    throw new SemWeightDeclarationV1Error("model.invalid", modelIssues[0]?.subject ?? "model", "The SemModelV4 is invalid");
  }
  if (model.data_binding.kind !== "raw" || model.data_binding.weight == null) return null;
  const weight = parseSemWeightBindingV4(model.data_binding.weight);
  const variable = model.variables.find((candidate) => candidate.id === weight.variable);
  if (variable?.kind !== "observed"
    || variable.scale !== "continuous"
    || variable.role !== "control") {
    throw new SemWeightDeclarationV1Error(
      "weight_variable_invalid",
      weight.variable,
      "A weight declaration must resolve to one continuous observed control variable",
    );
  }
  const identity = {
    variable_id: variable.id,
    source_column: variable.source_column,
  };
  return {
    contract_version: SEM_WEIGHT_DECLARATION_CONTRACT_VERSION_V1,
    dataset_id: requiredText(model.data_binding.dataset_id, "model.data_binding.dataset_id"),
    binding: weight.kind === "sampling"
      ? { kind: "sampling", ...identity, normalization: weight.normalization }
      : { kind: weight.kind, ...identity },
  };
}

/** Current targets all fail closed; the reserved normalization code is never emitted here. */
export function weightCapabilityIssueV1(
  target: WeightCapabilityTargetV1,
  declaration: ResolvedWeightDeclarationV1,
): WeightCapabilityIssueV1 {
  if (declaration.binding.kind === "case") return {
    code: "case_weight_unsupported",
    target,
    declaration,
    subject: declaration.binding.variable_id,
    corrective_action: CASE_CORRECTIVE_ACTION,
  };
  if (declaration.binding.kind === "frequency") return {
    code: "frequency_weight_unsupported",
    target,
    declaration,
    subject: declaration.binding.variable_id,
    corrective_action: FREQUENCY_CORRECTIVE_ACTION,
  };
  return {
    code: "sampling_weight_unsupported",
    target,
    declaration,
    subject: declaration.binding.variable_id,
    corrective_action: SAMPLING_CORRECTIVE_ACTION,
  };
}

/** Reserved for a future target that supports sampling weights but not this normalization. */
export function samplingWeightNormalizationIssueV1(
  target: WeightCapabilityTargetV1,
  declaration: ResolvedWeightDeclarationV1,
): WeightCapabilityIssueV1 {
  if (declaration.binding.kind !== "sampling") {
    throw new SemWeightDeclarationV1Error("weight_kind_invalid", declaration.binding.kind, "Sampling normalization diagnostics require a sampling-weight declaration");
  }
  return {
    code: "sampling_weight_normalization_unsupported",
    target,
    declaration,
    subject: declaration.binding.variable_id,
    corrective_action: NORMALIZATION_CORRECTIVE_ACTION,
  };
}

export function legacyCaseWeightCapabilityIssueV1(
  target: WeightCapabilityTargetV1,
  legacyCaseWeightColumn: string,
  declaration: ResolvedWeightDeclarationV1 | null,
): WeightCapabilityIssueV1 {
  const column = legacyCaseWeightColumn;
  if (column.length === 0) {
    throw new SemWeightDeclarationV1Error(
      "legacy_case_weight_column_empty",
      "settings.case_weight_column",
      "Legacy settings.case_weight_column cannot be empty",
    );
  }
  if (column.trim()
    && declaration?.binding.kind === "case"
    && declaration.binding.source_column === column) {
    return weightCapabilityIssueV1(target, declaration);
  }
  return {
    code: "legacy_case_weight_binding_ambiguous",
    target,
    declaration,
    subject: column,
    corrective_action: `Legacy settings.case_weight_column '${column}' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.`,
  };
}
