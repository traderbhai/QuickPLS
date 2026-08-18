import {
  compareUtf8StringsV1,
  parseSemModelV4AuthoringDraft,
  validateSemModelV4,
  type SemEndpointV4,
  type SemModelV4,
  type SemModelV4Issue,
} from "./semModelV4";

type WireRecord = Record<string, unknown>;
const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const COMPLETE_MODEL_FIELDS = [
  "schema_version",
  "id",
  "name",
  "variables",
  "relations",
  "parameters",
  "constraints",
  "derived_terms",
  "group",
  "data_binding",
  "annotations",
  "presentation",
] as const;

export interface StandardSemModelV4AuthorityCasRequestV1 {
  expectedSourceModelDocumentSha256: string;
  sourceModel: SemModelV4;
  candidate: SemModelV4;
}

export interface StandardSemModelV4AuthorityResolveRequestV1 {
  model: SemModelV4;
}

export interface StandardSemModelV4AuthorityCasResultV1 {
  schemaVersion: 1;
  sourceModelDocumentSha256: string;
  canonicalCandidate: SemModelV4;
  candidateModelDocumentSha256: string;
  candidateScientificSha256: string | null;
  readiness: "ready" | "authoring_only";
  authoringIssues: SemModelV4Issue[];
  readinessIssues: SemModelV4Issue[];
}

export interface StandardSemModelV4AuthorityResolveResultV1 {
  schemaVersion: 1;
  canonicalModel: SemModelV4;
  modelDocumentSha256: string;
  scientificSha256: string | null;
  readiness: "ready" | "authoring_only";
  authoringIssues: SemModelV4Issue[];
  readinessIssues: SemModelV4Issue[];
}

export interface StandardSemModelV4AuthorityCasDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
  authoringIssues: SemModelV4Issue[];
  readinessIssues: SemModelV4Issue[];
}

export type StandardSemModelV4AuthorityCasOutcomeV1 =
  | { status: "ok"; value: StandardSemModelV4AuthorityCasResultV1 }
  | { status: "blocked"; diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1 };

export type StandardSemModelV4AuthorityResolveOutcomeV1 =
  | { status: "ok"; value: StandardSemModelV4AuthorityResolveResultV1 }
  | { status: "blocked"; diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1 };

export class StandardSemModelV4AuthorityCasWireError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "StandardSemModelV4AuthorityCasWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new StandardSemModelV4AuthorityCasWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("standard_sem_model_v4_authority.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(value: unknown, fields: readonly string[], path: string): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(fields);
  for (const field of fields) {
    if (!Object.prototype.hasOwnProperty.call(record, field)) {
      fail("standard_sem_model_v4_authority.field_missing", `${path}.${field}`, `${path}.${field} is required.`);
    }
  }
  for (const field of Object.keys(record)) {
    if (!allowed.has(field)) {
      fail("standard_sem_model_v4_authority.field_unknown", `${path}.${field}`, `${path}.${field} is not allowed.`);
    }
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    fail("standard_sem_model_v4_authority.text_required", path, `${path} must be a nonempty string.`);
  }
  return value;
}

function sha256At(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) {
    fail("standard_sem_model_v4_authority.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  }
  return digest;
}

function completeModelAt(value: unknown, path: string): SemModelV4 {
  exactRecordAt(value, COMPLETE_MODEL_FIELDS, path);
  try {
    return parseSemModelV4AuthoringDraft(value);
  } catch (error) {
    if (error && typeof error === "object" && "issues" in error) throw error;
    return fail("standard_sem_model_v4_authority.model_invalid", path, `${path} must be a strict, complete SemModelV4 document.`);
  }
}

function issueAt(value: unknown, path: string): SemModelV4Issue {
  const issue = exactRecordAt(value, ["code", "subject", "message"], path);
  if (issue.subject !== null && typeof issue.subject !== "string") {
    fail("standard_sem_model_v4_authority.issue_subject_invalid", `${path}.subject`, `${path}.subject must be text or null.`);
  }
  return {
    code: textAt(issue.code, `${path}.code`),
    subject: issue.subject as string | null,
    message: textAt(issue.message, `${path}.message`),
  };
}

function issuesAt(value: unknown, path: string): SemModelV4Issue[] {
  if (!Array.isArray(value)) {
    fail("standard_sem_model_v4_authority.issues_array_required", path, `${path} must be an issue array.`);
  }
  return value.map((issue, index) => issueAt(issue, `${path}[${index}]`));
}

function compareNativeEndpoint(left: SemEndpointV4, right: SemEndpointV4): number {
  const rank = { variable: 0, residual_of: 1, disturbance_of: 2 } as const;
  return rank[left.kind] - rank[right.kind] || compareUtf8StringsV1(left.id, right.id);
}

/** Mirrors Rust `SemModelV4::canonicalized` using UTF-8/Rust enum ordering. */
function canonicalizeNativeCandidate(model: SemModelV4): SemModelV4 {
  const canonical = JSON.parse(JSON.stringify(model)) as SemModelV4;
  for (const variable of canonical.variables) {
    if (variable.kind === "observed") variable.missing_markers.sort(compareUtf8StringsV1);
  }
  canonical.variables.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  for (const relation of canonical.relations) {
    if (relation.kind === "covariance" && compareNativeEndpoint(relation.left, relation.right) > 0) {
      [relation.left, relation.right] = [relation.right, relation.left];
    }
  }
  canonical.relations.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  for (const parameter of canonical.parameters) {
    if (parameter.target.kind === "covariance"
      && compareNativeEndpoint(parameter.target.left, parameter.target.right) > 0) {
      [parameter.target.left, parameter.target.right] = [parameter.target.right, parameter.target.left];
    }
    parameter.group_overrides?.sort((left, right) => compareUtf8StringsV1(left.group, right.group));
  }
  canonical.parameters.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  for (const constraint of canonical.constraints) {
    if (constraint.kind === "equality") constraint.parameters.sort(compareUtf8StringsV1);
    else if (constraint.kind === "linear") {
      constraint.terms.sort((left, right) => compareUtf8StringsV1(left.parameter, right.parameter));
    }
  }
  canonical.constraints.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  for (const term of canonical.derived_terms) {
    if (term.kind === "higher_order") term.components.sort(compareUtf8StringsV1);
  }
  canonical.derived_terms.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  if (canonical.group.kind === "observed_groups") {
    canonical.group.levels.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  }
  for (const annotation of canonical.annotations) {
    if (annotation.kind === "display_only_covariance"
      && compareUtf8StringsV1(annotation.left, annotation.right) > 0) {
      [annotation.left, annotation.right] = [annotation.right, annotation.left];
    }
  }
  canonical.annotations.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  if (canonical.presentation.kind === "canvas") {
    canonical.presentation.nodes.sort((left, right) => compareUtf8StringsV1(left.variable, right.variable));
    canonical.presentation.edges.sort((left, right) => compareUtf8StringsV1(left.relation, right.relation));
    canonical.presentation.shapes.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
    canonical.presentation.images.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
    canonical.presentation.lines.sort((left, right) => compareUtf8StringsV1(left.id, right.id));
  }
  return canonical;
}

export function parseStandardSemModelV4AuthorityCasRequestV1(
  input: unknown,
): StandardSemModelV4AuthorityCasRequestV1 {
  const request = exactRecordAt(input, [
    "expectedSourceModelDocumentSha256",
    "sourceModel",
    "candidate",
  ], "request");
  const sourceModel = completeModelAt(request.sourceModel, "request.sourceModel");
  const candidate = completeModelAt(request.candidate, "request.candidate");
  if (candidate.id !== sourceModel.id) {
    fail("standard_sem_model_v4_authority.model_id_mismatch", "request.candidate.id", "Candidate and source model ids must match exactly.");
  }
  return {
    expectedSourceModelDocumentSha256: sha256At(
      request.expectedSourceModelDocumentSha256,
      "request.expectedSourceModelDocumentSha256",
    ),
    sourceModel,
    candidate,
  };
}

export function parseStandardSemModelV4AuthorityResolveRequestV1(
  input: unknown,
): StandardSemModelV4AuthorityResolveRequestV1 {
  const request = exactRecordAt(input, ["model"], "request");
  return { model: completeModelAt(request.model, "request.model") };
}

export function parseStandardSemModelV4AuthorityCasOutcomeV1(
  input: unknown,
  request: StandardSemModelV4AuthorityCasRequestV1,
): StandardSemModelV4AuthorityCasOutcomeV1 {
  const outcome = recordAt(input, "outcome");
  if (outcome.status === "blocked") {
    exactRecordAt(outcome, ["status", "diagnostic"], "outcome");
    const diagnostic = exactRecordAt(outcome.diagnostic, [
      "code",
      "message",
      "correctiveAction",
      "authoringIssues",
      "readinessIssues",
    ], "outcome.diagnostic");
    return {
      status: "blocked",
      diagnostic: {
        code: textAt(diagnostic.code, "outcome.diagnostic.code"),
        message: textAt(diagnostic.message, "outcome.diagnostic.message"),
        correctiveAction: textAt(diagnostic.correctiveAction, "outcome.diagnostic.correctiveAction"),
        authoringIssues: issuesAt(diagnostic.authoringIssues, "outcome.diagnostic.authoringIssues"),
        readinessIssues: issuesAt(diagnostic.readinessIssues, "outcome.diagnostic.readinessIssues"),
      },
    };
  }
  if (outcome.status !== "ok") {
    return fail("standard_sem_model_v4_authority.status_invalid", "outcome.status", "CAS outcome status must be ok or blocked.");
  }
  exactRecordAt(outcome, ["status", "value"], "outcome");
  const value = exactRecordAt(outcome.value, [
    "schemaVersion",
    "sourceModelDocumentSha256",
    "canonicalCandidate",
    "candidateModelDocumentSha256",
    "candidateScientificSha256",
    "readiness",
    "authoringIssues",
    "readinessIssues",
  ], "outcome.value");
  if (value.schemaVersion !== 1) {
    fail("standard_sem_model_v4_authority.result_schema_invalid", "outcome.value.schemaVersion", "CAS result schemaVersion must be 1.");
  }
  const sourceModelDocumentSha256 = sha256At(
    value.sourceModelDocumentSha256,
    "outcome.value.sourceModelDocumentSha256",
  );
  if (sourceModelDocumentSha256 !== request.expectedSourceModelDocumentSha256) {
    fail("standard_sem_model_v4_authority.source_digest_mismatch", "outcome.value.sourceModelDocumentSha256", "CAS result is not bound to the requested source digest.");
  }
  const canonicalCandidate = completeModelAt(value.canonicalCandidate, "outcome.value.canonicalCandidate");
  if (canonicalCandidate.id !== request.sourceModel.id
    || JSON.stringify(canonicalCandidate) !== JSON.stringify(canonicalizeNativeCandidate(request.candidate))) {
    fail("standard_sem_model_v4_authority.candidate_mismatch", "outcome.value.canonicalCandidate", "Native canonical candidate differs from the complete requested candidate.");
  }
  const readiness = value.readiness === "ready"
    ? "ready"
    : value.readiness === "authoring_only"
      ? "authoring_only"
      : fail("standard_sem_model_v4_authority.readiness_invalid", "outcome.value.readiness", "Readiness must be ready or authoring_only.");
  const authoringIssues = issuesAt(value.authoringIssues, "outcome.value.authoringIssues");
  const readinessIssues = issuesAt(value.readinessIssues, "outcome.value.readinessIssues");
  if (authoringIssues.length !== 0) {
    fail("standard_sem_model_v4_authority.accepted_authoring_issues", "outcome.value.authoringIssues", "An accepted candidate cannot contain authoring-integrity issues.");
  }
  if (JSON.stringify(readinessIssues) !== JSON.stringify(validateSemModelV4(canonicalCandidate))) {
    fail("standard_sem_model_v4_authority.readiness_issues_mismatch", "outcome.value.readinessIssues", "Native readiness issues differ from the returned canonical candidate.");
  }
  const candidateScientificSha256 = value.candidateScientificSha256 === null
    ? null
    : sha256At(value.candidateScientificSha256, "outcome.value.candidateScientificSha256");
  if ((readiness === "ready" && (candidateScientificSha256 === null || readinessIssues.length !== 0))
    || (readiness === "authoring_only" && (candidateScientificSha256 !== null || readinessIssues.length === 0))) {
    fail("standard_sem_model_v4_authority.readiness_digest_mismatch", "outcome.value", "Scientific digest and readiness issues contradict the readiness state.");
  }
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      sourceModelDocumentSha256,
      canonicalCandidate,
      candidateModelDocumentSha256: sha256At(
        value.candidateModelDocumentSha256,
        "outcome.value.candidateModelDocumentSha256",
      ),
      candidateScientificSha256,
      readiness,
      authoringIssues,
      readinessIssues,
    },
  };
}

export function parseStandardSemModelV4AuthorityResolveOutcomeV1(
  input: unknown,
  request: StandardSemModelV4AuthorityResolveRequestV1,
): StandardSemModelV4AuthorityResolveOutcomeV1 {
  const outcome = recordAt(input, "outcome");
  if (outcome.status === "blocked") {
    exactRecordAt(outcome, ["status", "diagnostic"], "outcome");
    const diagnostic = exactRecordAt(outcome.diagnostic, [
      "code",
      "message",
      "correctiveAction",
      "authoringIssues",
      "readinessIssues",
    ], "outcome.diagnostic");
    return {
      status: "blocked",
      diagnostic: {
        code: textAt(diagnostic.code, "outcome.diagnostic.code"),
        message: textAt(diagnostic.message, "outcome.diagnostic.message"),
        correctiveAction: textAt(diagnostic.correctiveAction, "outcome.diagnostic.correctiveAction"),
        authoringIssues: issuesAt(diagnostic.authoringIssues, "outcome.diagnostic.authoringIssues"),
        readinessIssues: issuesAt(diagnostic.readinessIssues, "outcome.diagnostic.readinessIssues"),
      },
    };
  }
  if (outcome.status !== "ok") {
    return fail("standard_sem_model_v4_authority.status_invalid", "outcome.status", "Resolve outcome status must be ok or blocked.");
  }
  exactRecordAt(outcome, ["status", "value"], "outcome");
  const value = exactRecordAt(outcome.value, [
    "schemaVersion",
    "canonicalModel",
    "modelDocumentSha256",
    "scientificSha256",
    "readiness",
    "authoringIssues",
    "readinessIssues",
  ], "outcome.value");
  if (value.schemaVersion !== 1) {
    fail("standard_sem_model_v4_authority.result_schema_invalid", "outcome.value.schemaVersion", "Resolve result schemaVersion must be 1.");
  }
  const canonicalModel = completeModelAt(value.canonicalModel, "outcome.value.canonicalModel");
  if (JSON.stringify(canonicalModel) !== JSON.stringify(canonicalizeNativeCandidate(request.model))) {
    fail("standard_sem_model_v4_authority.resolved_model_mismatch", "outcome.value.canonicalModel", "Native canonical model differs from the complete requested model.");
  }
  const readiness = value.readiness === "ready"
    ? "ready"
    : value.readiness === "authoring_only"
      ? "authoring_only"
      : fail("standard_sem_model_v4_authority.readiness_invalid", "outcome.value.readiness", "Readiness must be ready or authoring_only.");
  const authoringIssues = issuesAt(value.authoringIssues, "outcome.value.authoringIssues");
  const readinessIssues = issuesAt(value.readinessIssues, "outcome.value.readinessIssues");
  if (authoringIssues.length !== 0) {
    fail("standard_sem_model_v4_authority.accepted_authoring_issues", "outcome.value.authoringIssues", "An accepted authority cannot contain authoring-integrity issues.");
  }
  if (JSON.stringify(readinessIssues) !== JSON.stringify(validateSemModelV4(canonicalModel))) {
    fail("standard_sem_model_v4_authority.readiness_issues_mismatch", "outcome.value.readinessIssues", "Native readiness issues differ from the returned canonical model.");
  }
  const scientificSha256 = value.scientificSha256 === null
    ? null
    : sha256At(value.scientificSha256, "outcome.value.scientificSha256");
  if ((readiness === "ready" && (scientificSha256 === null || readinessIssues.length !== 0))
    || (readiness === "authoring_only" && (scientificSha256 !== null || readinessIssues.length === 0))) {
    fail("standard_sem_model_v4_authority.readiness_digest_mismatch", "outcome.value", "Scientific digest and readiness issues contradict the readiness state.");
  }
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      canonicalModel,
      modelDocumentSha256: sha256At(value.modelDocumentSha256, "outcome.value.modelDocumentSha256"),
      scientificSha256,
      readiness,
      authoringIssues,
      readinessIssues,
    },
  };
}
