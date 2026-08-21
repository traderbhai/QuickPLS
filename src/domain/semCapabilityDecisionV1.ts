export const SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION = 1 as const;

export type SemCapabilityDecisionStatusV1 = "supported" | "experimental" | "blocked";
export type SemCapabilityDiagnosticSeverityV1 = "error" | "warning" | "info";
export type SemCapabilityDiagnosticCodeV1 = `sem.capability.${string}`;

/** Exact Capability Registry cell identity; cell_id alone is not sufficient. */
export interface SemCapabilityCellIdV1 {
  readonly registry_schema_version: number;
  readonly capability_id: string;
  readonly cell_id: string;
  readonly capability_version: string;
}

export interface SemCapabilityDiagnosticV1 {
  readonly code: SemCapabilityDiagnosticCodeV1;
  readonly severity: SemCapabilityDiagnosticSeverityV1;
  readonly subject: string | null;
  readonly message: string;
  readonly corrections: readonly string[];
}

export interface SemCapabilityEvidenceV1 {
  readonly evidence_id: string;
  readonly description: string;
}

/**
 * Estimator-specific capability decision only. This contract does not compile,
 * estimate, rewrite, or otherwise change the scientific model.
 */
export interface SemCapabilityDecisionV1 {
  readonly schema_version: typeof SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION;
  readonly status: SemCapabilityDecisionStatusV1;
  /** Exact visible text so status never depends on color alone. */
  readonly status_label: "Supported" | "Experimental" | "Blocked";
  readonly estimator_id: string;
  readonly capability_cells: readonly SemCapabilityCellIdV1[];
  readonly diagnostics: readonly SemCapabilityDiagnosticV1[];
  readonly evidence: readonly SemCapabilityEvidenceV1[];
  readonly summary: string;
  readonly explanation: string;
}

export interface CreateSemCapabilityDecisionV1Input {
  readonly status: SemCapabilityDecisionStatusV1;
  readonly estimator_id: string;
  readonly capability_cells: readonly SemCapabilityCellIdV1[];
  readonly diagnostics: readonly SemCapabilityDiagnosticV1[];
  readonly evidence: readonly SemCapabilityEvidenceV1[];
  readonly summary: string;
  readonly explanation: string;
}

export type SemCapabilityDecisionV1ErrorCode =
  | "schema.invalid_shape"
  | "schema.unknown_field"
  | "schema.invalid_discriminator"
  | "schema.version_unsupported"
  | "schema.text_invalid"
  | "schema.diagnostic_code_invalid"
  | "decision.capability_cells_empty"
  | "decision.capability_cell_duplicate"
  | "decision.diagnostic_duplicate"
  | "decision.correction_duplicate"
  | "decision.correction_required"
  | "decision.evidence_empty"
  | "decision.evidence_duplicate"
  | "decision.status_label_mismatch"
  | "decision.runnable_status_has_blocking_diagnostic"
  | "decision.blocked_without_blocking_diagnostic";

export class SemCapabilityDecisionV1Error extends Error {
  constructor(
    public readonly code: SemCapabilityDecisionV1ErrorCode,
    public readonly subject: string,
    message: string,
  ) {
    super(message);
    this.name = "SemCapabilityDecisionV1Error";
  }
}

type UnknownRecord = Record<string, unknown>;

const STATUS_LABELS: Readonly<Record<SemCapabilityDecisionStatusV1, SemCapabilityDecisionV1["status_label"]>> = {
  supported: "Supported",
  experimental: "Experimental",
  blocked: "Blocked",
};

const SEVERITY_RANK: Readonly<Record<SemCapabilityDiagnosticSeverityV1, number>> = {
  error: 0,
  warning: 1,
  info: 2,
};

const UTF8_ENCODER = new TextEncoder();
const CONTROL_CHARACTER = /[\u0000-\u001f\u007f-\u009f]/u;
const DIAGNOSTIC_CODE_PREFIX = "sem.capability.";

function fail(code: SemCapabilityDecisionV1ErrorCode, subject: string, message: string): never {
  throw new SemCapabilityDecisionV1Error(code, subject, message);
}

function record(value: unknown, path: string): UnknownRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return fail("schema.invalid_shape", path, `${path} must be an object.`);
  }
  return value as UnknownRecord;
}

function exactRecord(value: unknown, fields: readonly string[], path: string): UnknownRecord {
  const candidate = record(value, path);
  const unknown = Object.keys(candidate).find((key) => !fields.includes(key));
  if (unknown) return fail("schema.unknown_field", `${path}.${unknown}`, `${path}.${unknown} is not supported.`);
  const missing = fields.find((key) => !Object.prototype.hasOwnProperty.call(candidate, key));
  if (missing) return fail("schema.invalid_shape", `${path}.${missing}`, `${path}.${missing} is required.`);
  return candidate;
}

function requiredText(value: unknown, path: string): string {
  if (typeof value !== "string"
    || !value.trim()
    || value.trim() !== value
    || CONTROL_CHARACTER.test(value)
    || value.normalize("NFC") !== value) {
    return fail(
      "schema.text_invalid",
      path,
      `${path} must be nonempty NFC text without surrounding whitespace or control characters.`,
    );
  }
  return value;
}

function array<T>(value: unknown, path: string, parse: (item: unknown, path: string) => T): T[] {
  if (!Array.isArray(value)) return fail("schema.invalid_shape", path, `${path} must be an array.`);
  return value.map((item, index) => parse(item, `${path}[${index}]`));
}

function status(value: unknown, path: string): SemCapabilityDecisionStatusV1 {
  if (value !== "supported" && value !== "experimental" && value !== "blocked") {
    return fail("schema.invalid_discriminator", path, `${path} must be supported, experimental, or blocked.`);
  }
  return value;
}

function severity(value: unknown, path: string): SemCapabilityDiagnosticSeverityV1 {
  if (value !== "error" && value !== "warning" && value !== "info") {
    return fail("schema.invalid_discriminator", path, `${path} must be error, warning, or info.`);
  }
  return value;
}

function diagnosticCode(value: unknown, path: string): SemCapabilityDiagnosticCodeV1 {
  const code = requiredText(value, path);
  if (!code.startsWith(DIAGNOSTIC_CODE_PREFIX) || code.length === DIAGNOSTIC_CODE_PREFIX.length) {
    return fail("schema.diagnostic_code_invalid", path, `${path} must use the sem.capability. namespace.`);
  }
  return code as SemCapabilityDiagnosticCodeV1;
}

function positiveInteger(value: unknown, path: string): number {
  if (!Number.isSafeInteger(value) || (value as number) <= 0 || (value as number) > 0xffff_ffff) {
    return fail("schema.invalid_shape", path, `${path} must be a positive unsigned 32-bit integer.`);
  }
  return value as number;
}

function compareUtf8(left: string, right: string): number {
  if (left === right) return 0;
  const leftBytes = UTF8_ENCODER.encode(left);
  const rightBytes = UTF8_ENCODER.encode(right);
  const length = Math.min(leftBytes.length, rightBytes.length);
  for (let index = 0; index < length; index += 1) {
    const difference = leftBytes[index]! - rightBytes[index]!;
    if (difference !== 0) return difference;
  }
  return leftBytes.length - rightBytes.length;
}

function compareNullableText(left: string | null, right: string | null): number {
  if (left === right) return 0;
  if (left === null) return -1;
  if (right === null) return 1;
  return compareUtf8(left, right);
}

function parseCell(value: unknown, path: string): SemCapabilityCellIdV1 {
  const candidate = exactRecord(value, ["registry_schema_version", "capability_id", "cell_id", "capability_version"], path);
  return {
    registry_schema_version: positiveInteger(candidate.registry_schema_version, `${path}.registry_schema_version`),
    capability_id: requiredText(candidate.capability_id, `${path}.capability_id`),
    cell_id: requiredText(candidate.cell_id, `${path}.cell_id`),
    capability_version: requiredText(candidate.capability_version, `${path}.capability_version`),
  };
}

function compareCells(left: SemCapabilityCellIdV1, right: SemCapabilityCellIdV1): number {
  return left.registry_schema_version - right.registry_schema_version
    || compareUtf8(left.capability_id, right.capability_id)
    || compareUtf8(left.cell_id, right.cell_id)
    || compareUtf8(left.capability_version, right.capability_version);
}

function cellIdentity(cell: SemCapabilityCellIdV1): string {
  return `${cell.registry_schema_version}\0${cell.capability_id}\0${cell.cell_id}\0${cell.capability_version}`;
}

function parseDiagnostic(value: unknown, path: string): SemCapabilityDiagnosticV1 {
  const candidate = exactRecord(value, ["code", "severity", "subject", "message", "corrections"], path);
  const code = diagnosticCode(candidate.code, `${path}.code`);
  const parsedSeverity = severity(candidate.severity, `${path}.severity`);
  const subject = candidate.subject === null
    ? null
    : requiredText(candidate.subject, `${path}.subject`);
  const corrections = array(candidate.corrections, `${path}.corrections`, requiredText)
    .sort(compareUtf8);
  if (parsedSeverity !== "info" && corrections.length === 0) {
    return fail(
      "decision.correction_required",
      `${path}.corrections`,
      `Warning or error diagnostic ${code} requires an actionable correction.`,
    );
  }
  for (let index = 1; index < corrections.length; index += 1) {
    if (corrections[index - 1] === corrections[index]) {
      return fail(
        "decision.correction_duplicate",
        `${path}.corrections[${index}]`,
        `Diagnostic ${code} repeats correction ${corrections[index]}.`,
      );
    }
  }
  return {
    code,
    severity: parsedSeverity,
    subject,
    message: requiredText(candidate.message, `${path}.message`),
    corrections,
  };
}

function compareDiagnostics(left: SemCapabilityDiagnosticV1, right: SemCapabilityDiagnosticV1): number {
  return SEVERITY_RANK[left.severity] - SEVERITY_RANK[right.severity]
    || compareUtf8(left.code, right.code)
    || compareNullableText(left.subject, right.subject);
}

function diagnosticIdentity(diagnostic: SemCapabilityDiagnosticV1): string {
  return `${diagnostic.code}\0${diagnostic.subject ?? ""}`;
}

function parseEvidence(value: unknown, path: string): SemCapabilityEvidenceV1 {
  const candidate = exactRecord(value, ["evidence_id", "description"], path);
  return {
    evidence_id: requiredText(candidate.evidence_id, `${path}.evidence_id`),
    description: requiredText(candidate.description, `${path}.description`),
  };
}

function assertUnique<T>(
  values: readonly T[],
  identity: (value: T) => string,
  code: SemCapabilityDecisionV1ErrorCode,
  subject: (value: T, index: number) => string,
  label: string,
) {
  const seen = new Set<string>();
  values.forEach((value, index) => {
    const key = identity(value);
    if (seen.has(key)) fail(code, subject(value, index), `${label} ${key.replaceAll("\0", "::")} is duplicated.`);
    seen.add(key);
  });
}

/** Strictly parses, validates, canonicalizes, and deeply freezes a decision. */
export function parseSemCapabilityDecisionV1(value: unknown): SemCapabilityDecisionV1 {
  const candidate = exactRecord(value, [
    "schema_version",
    "status",
    "status_label",
    "estimator_id",
    "capability_cells",
    "diagnostics",
    "evidence",
    "summary",
    "explanation",
  ], "decision");
  if (candidate.schema_version !== SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION) {
    return fail("schema.version_unsupported", "decision.schema_version", "SEM capability decision schema version is unsupported.");
  }
  const parsedStatus = status(candidate.status, "decision.status");
  const statusLabel = requiredText(candidate.status_label, "decision.status_label");
  if (statusLabel !== STATUS_LABELS[parsedStatus]) {
    return fail(
      "decision.status_label_mismatch",
      "decision.status_label",
      `Status ${parsedStatus} requires accessible label ${STATUS_LABELS[parsedStatus]}.`,
    );
  }

  const capabilityCells = array(candidate.capability_cells, "decision.capability_cells", parseCell)
    .sort(compareCells);
  if (capabilityCells.length === 0) {
    return fail("decision.capability_cells_empty", "decision.capability_cells", "A capability decision requires at least one exact capability cell.");
  }
  assertUnique(
    capabilityCells,
    cellIdentity,
    "decision.capability_cell_duplicate",
    (_, index) => `decision.capability_cells[${index}]`,
    "Capability cell",
  );

  const diagnostics = array(candidate.diagnostics, "decision.diagnostics", parseDiagnostic)
    .sort(compareDiagnostics);
  assertUnique(
    diagnostics,
    diagnosticIdentity,
    "decision.diagnostic_duplicate",
    (_, index) => `decision.diagnostics[${index}]`,
    "Diagnostic",
  );

  const evidence = array(candidate.evidence, "decision.evidence", parseEvidence)
    .sort((left, right) => compareUtf8(left.evidence_id, right.evidence_id));
  if (evidence.length === 0) {
    return fail("decision.evidence_empty", "decision.evidence", "A capability decision requires at least one evidence item.");
  }
  assertUnique(
    evidence,
    (item) => item.evidence_id,
    "decision.evidence_duplicate",
    (_, index) => `decision.evidence[${index}]`,
    "Evidence item",
  );

  const blocking = diagnostics.find((diagnostic) => diagnostic.severity === "error");
  if (parsedStatus !== "blocked" && blocking) {
    return fail(
      "decision.runnable_status_has_blocking_diagnostic",
      `decision.diagnostics.${blocking.code}`,
      `Runnable status ${parsedStatus} contradicts blocking diagnostic ${blocking.code}.`,
    );
  }
  if (parsedStatus === "blocked" && !blocking) {
    return fail(
      "decision.blocked_without_blocking_diagnostic",
      "decision.diagnostics",
      "Blocked status requires at least one error diagnostic.",
    );
  }

  return deepFreeze({
    schema_version: SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION,
    status: parsedStatus,
    status_label: STATUS_LABELS[parsedStatus],
    estimator_id: requiredText(candidate.estimator_id, "decision.estimator_id"),
    capability_cells: capabilityCells,
    diagnostics,
    evidence,
    summary: requiredText(candidate.summary, "decision.summary"),
    explanation: requiredText(candidate.explanation, "decision.explanation"),
  });
}

/** Constructs through the same strict boundary and derives the accessible label. */
export function createSemCapabilityDecisionV1(input: CreateSemCapabilityDecisionV1Input): SemCapabilityDecisionV1 {
  return parseSemCapabilityDecisionV1({
    schema_version: SEM_CAPABILITY_DECISION_V1_SCHEMA_VERSION,
    status: input.status,
    status_label: STATUS_LABELS[input.status],
    estimator_id: input.estimator_id,
    capability_cells: input.capability_cells,
    diagnostics: input.diagnostics,
    evidence: input.evidence,
    summary: input.summary,
    explanation: input.explanation,
  });
}

/** Stable JSON after strict validation and canonical collection ordering. */
export function canonicalSemCapabilityDecisionV1Json(value: unknown): string {
  return JSON.stringify(parseSemCapabilityDecisionV1(value));
}

function deepFreeze<T>(value: T): T {
  if (value && typeof value === "object" && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const child of Object.values(value as Record<string, unknown>)) deepFreeze(child);
  }
  return value;
}
