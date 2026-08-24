import { invoke } from "@tauri-apps/api/core";
import {
  parseConditionalRawProbeFitMetricReceiptV2,
  parseMultiModResultAttachmentV1,
  type ConditionalRawProbeFitMetricReceiptV2,
  type MultiModRecipeConfigV1,
  type MultiModResultAttachmentV1,
  type TypedGroupValueV1,
} from "../domain/multimodContractsV1";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import { parseNativeCanonicalResultDocumentV2 } from "./nativeCanonicalResultDocumentV2";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu;
const SHA256 = /^[a-f0-9]{64}$/u;
const TARGETS = [
  "mga_multigroup_v1",
  "pls_heterogeneity_v2",
  "general_sem_conditional_process_v2",
  "interventional_causal_mediation_v1",
] as const;
const CACHE_STAGES = ["mga_execution", "archive_ready"] as const;

export type NativeMultiModTargetV1 = (typeof TARGETS)[number];
export type NativeMultiModExternalCacheStageV1 =
  (typeof CACHE_STAGES)[number];
export type NativeMultiModRuntimeReadinessV1 =
  "built_in_from_dataset" | "prepared_adapter_required" | "blocked";
export type NativeMultiModJobStateV1 =
  | "queued"
  | "running"
  | "cancelling"
  | "publishing"
  | "completed"
  | "failed"
  | "cancelled";

export interface NativeMultiModArchiveAuthorityV1 {
  readonly archivePath: string;
  readonly archiveSha256: string;
  readonly projectId: string;
  readonly datasetId: string;
  readonly datasetFingerprint: string;
  readonly modelId: string;
  readonly modelScientificSha256: string;
  readonly sourceRecipeId: string;
  readonly sourceRecipeDocumentSha256: string;
}

export interface NativeMultiModGroupingProfileGroupV1 {
  readonly groupId: string;
  readonly label: string;
  readonly value: TypedGroupValueV1;
  readonly selectedRows: number;
  readonly completeCases: number;
}

export interface NativeMultiModGroupingProfileColumnV1 {
  readonly column: string;
  readonly label: string;
  readonly usedAsIndicator: boolean;
  readonly groups: readonly NativeMultiModGroupingProfileGroupV1[];
}

export interface NativeMultiModGroupingProfileV1 {
  readonly schemaVersion: 1;
  readonly archiveSha256: string;
  readonly datasetFingerprint: string;
  readonly columns: readonly NativeMultiModGroupingProfileColumnV1[];
  readonly omittedHighCardinalityColumns: readonly string[];
  readonly sourceRecheckedUnchanged: true;
}

export interface NativeMultiModExternalCacheReceiptV1 {
  readonly schemaVersion: 1;
  readonly cacheId: string;
  readonly cacheDirectory: string;
  readonly manifestSha256: string;
  readonly embeddedAuthoritySha256: string;
  readonly sourceArchiveSha256: string;
  readonly resultId: string;
  readonly recipeId: string;
  readonly target: NativeMultiModTargetV1;
  readonly stage: NativeMultiModExternalCacheStageV1;
  readonly createdAt: string;
}

export interface NativeMultiModStagedRequestV1 {
  readonly surface: "internal_labs_multimod_v1";
  readonly experimentalLabsEnabled: true;
  readonly archivePath: string;
  readonly expectedArchiveSha256: string;
  readonly projectId: string;
  readonly datasetId: string;
  readonly datasetFingerprint: string;
  readonly modelId: string;
  readonly modelScientificSha256: string;
  readonly sourceRecipeId: string;
  readonly sourceRecipeDocumentSha256: string;
  readonly stagedRecipeId: string;
  readonly stagedCreatedAt: string;
  readonly config: {
    readonly kind: NativeMultiModTargetV1;
    readonly config: MultiModRecipeConfigV1["config"];
  };
  readonly resumeCache?: NativeMultiModExternalCacheReceiptV1;
}

export interface NativeMultiModPreflightV1 {
  readonly schemaVersion: 1;
  readonly target: NativeMultiModTargetV1;
  readonly capabilityCellId: string;
  readonly readiness: NativeMultiModRuntimeReadinessV1;
  readonly stableReasonCodes: readonly string[];
  readonly stagedRecipeId: string;
  readonly stagedRecipeDocumentSha256: string;
  readonly compilationIdentitySha256: string;
  readonly mgaGroupEligibility: NativeMgaGroupEligibilityV1 | null;
}

export interface NativeMgaGroupCountV1 {
  readonly groupId: string;
  readonly label: string;
  /** Physical complete rows retained from the source dataset. */
  readonly physicalCompleteRows: number;
  /** Expanded count for frequency weights; otherwise equals physical rows. */
  readonly effectiveCompleteCases: number;
}

export interface NativeMgaGroupEligibilityV1 {
  readonly countBasis: "physical_complete_rows" | "frequency_expanded_cases";
  readonly groups: readonly NativeMgaGroupCountV1[];
  readonly maximumImbalanceRatio: number | null;
  readonly eligible: boolean;
  readonly warningCodes: readonly string[];
  readonly blockerCodes: readonly string[];
}

export interface NativeMultiModJobFailureV1 {
  readonly schemaVersion: 1;
  readonly stage:
    | "access"
    | "archive_authority"
    | "compilation"
    | "runtime_support"
    | "estimation"
    | "evidence"
    | "cache"
    | "publication"
    | "reopen"
    | "integrity";
  readonly code: string;
  readonly message: string;
  readonly correctiveAction: string;
}

export interface NativeMultiModJobSnapshotV1 {
  readonly schemaVersion: 1;
  readonly jobId: string;
  readonly target: NativeMultiModTargetV1;
  readonly state: NativeMultiModJobStateV1;
  readonly phase: string;
  readonly shardId: string;
  readonly completedUnits: number;
  readonly totalUnits: number;
  readonly message: string | null;
  readonly warningCodes: readonly string[];
  readonly failure: NativeMultiModJobFailureV1 | null;
  readonly resumeCache: NativeMultiModExternalCacheReceiptV1 | null;
  readonly queuedAt: string;
  readonly startedAt: string | null;
  readonly completedAt: string | null;
}

export interface NativeMultiModArchiveAppendReceiptV1 {
  readonly schema_version: 1;
  readonly project_id: string;
  readonly result_id: string;
  readonly source_archive_sha256: string;
  readonly updated_archive_sha256: string;
  readonly sidecar_count: number;
  readonly source_verified_at_commit: true;
  readonly post_write_validated: true;
  readonly rollback_removed: boolean;
}

export interface NativeMultiModCompletedResultV1 {
  readonly schemaVersion: 1;
  readonly jobId: string;
  readonly archivePath: string;
  readonly archiveSha256: string;
  readonly projectId: string;
  readonly datasetId: string;
  readonly modelId: string;
  readonly attachment: MultiModResultAttachmentV1;
  readonly canonicalDocument: CanonicalResultDocumentV2;
  readonly appendReceipt: NativeMultiModArchiveAppendReceiptV1;
  readonly cacheReceipt: NativeMultiModExternalCacheReceiptV1;
  readonly cacheRemovedAfterCommit: boolean;
}

type WireRecord = Record<string, unknown>;

function record(value: unknown, path: string): WireRecord {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error(`${path} must be an object.`);
  return value as WireRecord;
}

function exact(
  value: unknown,
  required: readonly string[],
  optional: readonly string[],
  path: string,
): WireRecord {
  const item = record(value, path);
  const allowed = new Set([...required, ...optional]);
  for (const key of required)
    if (!Object.prototype.hasOwnProperty.call(item, key))
      throw new Error(`${path}.${key} is required.`);
  for (const key of Object.keys(item))
    if (!allowed.has(key))
      throw new Error(`${path}.${key} is not part of the versioned contract.`);
  return item;
}

function text(value: unknown, path: string): string {
  if (typeof value !== "string" || value.trim() !== value || !value)
    throw new Error(`${path} must be nonempty exact text.`);
  return value;
}

function uuid(value: unknown, path: string): string {
  const candidate = text(value, path);
  if (!UUID.test(candidate) || /^0{8}-0{4}-0{4}-0{4}-0{12}$/iu.test(candidate))
    throw new Error(`${path} must be a non-nil UUID.`);
  return candidate;
}

function sha(value: unknown, path: string): string {
  const candidate = text(value, path);
  if (!SHA256.test(candidate))
    throw new Error(`${path} must be a lowercase SHA-256.`);
  return candidate;
}

function timestamp(value: unknown, path: string): string {
  const candidate = text(value, path);
  if (!Number.isFinite(Date.parse(candidate)))
    throw new Error(`${path} must be an RFC 3339 timestamp.`);
  return candidate;
}

function count(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0)
    throw new Error(`${path} must be a nonnegative safe integer.`);
  return value;
}

function literal<T extends string>(
  value: unknown,
  options: readonly T[],
  path: string,
): T {
  if (typeof value !== "string" || !options.includes(value as T))
    throw new Error(`${path} has an unsupported value.`);
  return value as T;
}

function nullableText(value: unknown, path: string): string | null {
  return value === null ? null : text(value, path);
}

function boolean(value: unknown, path: string): boolean {
  if (typeof value !== "boolean") throw new Error(`${path} must be Boolean.`);
  return value;
}

function finite(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value))
    throw new Error(`${path} must be finite.`);
  return value;
}

function parseMgaGroupEligibility(
  value: unknown,
  path: string,
): NativeMgaGroupEligibilityV1 | null {
  if (value === null) return null;
  const item = exact(
    value,
    [
      "countBasis",
      "groups",
      "maximumImbalanceRatio",
      "eligible",
      "warningCodes",
      "blockerCodes",
    ],
    [],
    path,
  );
  if (
    !Array.isArray(item.groups) ||
    !Array.isArray(item.warningCodes) ||
    !Array.isArray(item.blockerCodes)
  ) {
    throw new Error(`${path} contains invalid eligibility arrays.`);
  }
  const groups = item.groups.map((value, index) => {
    const group = exact(
      value,
      [
        "groupId",
        "label",
        "physicalCompleteRows",
        "effectiveCompleteCases",
      ],
      [],
      `${path}.groups[${index}]`,
    );
    return {
      groupId: text(group.groupId, `${path}.groups[${index}].groupId`),
      label: text(group.label, `${path}.groups[${index}].label`),
      physicalCompleteRows: count(
        group.physicalCompleteRows,
        `${path}.groups[${index}].physicalCompleteRows`,
      ),
      effectiveCompleteCases: count(
        group.effectiveCompleteCases,
        `${path}.groups[${index}].effectiveCompleteCases`,
      ),
    };
  });
  if (new Set(groups.map((group) => group.groupId)).size !== groups.length) {
    throw new Error(`${path}.groups contains duplicate group identities.`);
  }
  return {
    countBasis: literal(
      item.countBasis,
      ["physical_complete_rows", "frequency_expanded_cases"] as const,
      `${path}.countBasis`,
    ),
    groups,
    maximumImbalanceRatio:
      item.maximumImbalanceRatio === null
        ? null
        : finite(item.maximumImbalanceRatio, `${path}.maximumImbalanceRatio`),
    eligible: boolean(item.eligible, `${path}.eligible`),
    warningCodes: item.warningCodes.map((value, index) =>
      text(value, `${path}.warningCodes[${index}]`),
    ),
    blockerCodes: item.blockerCodes.map((value, index) =>
      text(value, `${path}.blockerCodes[${index}]`),
    ),
  };
}

function parseGroupingValue(value: unknown, path: string): TypedGroupValueV1 {
  const item = exact(value, ["kind", "value"], [], path);
  if (item.kind === "text")
    return { kind: "text", value: text(item.value, `${path}.value`) };
  if (item.kind === "boolean")
    return { kind: "boolean", value: boolean(item.value, `${path}.value`) };
  if (item.kind === "integer") {
    const parsed = finite(item.value, `${path}.value`);
    if (!Number.isSafeInteger(parsed))
      throw new Error(`${path}.value must be a safe integer.`);
    return { kind: "integer", value: parsed };
  }
  if (item.kind === "number")
    return { kind: "number", value: finite(item.value, `${path}.value`) };
  throw new Error(`${path}.kind is not a supported typed grouping value.`);
}

export function parseNativeMultiModGroupingProfileV1(
  value: unknown,
  authority: NativeMultiModArchiveAuthorityV1,
): NativeMultiModGroupingProfileV1 {
  const item = exact(
    value,
    [
      "schemaVersion",
      "archiveSha256",
      "datasetFingerprint",
      "columns",
      "omittedHighCardinalityColumns",
      "sourceRecheckedUnchanged",
    ],
    [],
    "multimodGroupingProfile",
  );
  if (
    item.schemaVersion !== 1 ||
    item.sourceRecheckedUnchanged !== true ||
    !Array.isArray(item.columns) ||
    !Array.isArray(item.omittedHighCardinalityColumns)
  ) {
    throw new Error(
      "The native grouping profile has an unsupported schema or incomplete authority receipt.",
    );
  }
  const archiveSha256 = sha(
    item.archiveSha256,
    "multimodGroupingProfile.archiveSha256",
  );
  const datasetFingerprint = sha(
    item.datasetFingerprint,
    "multimodGroupingProfile.datasetFingerprint",
  );
  if (
    archiveSha256 !== authority.archiveSha256 ||
    datasetFingerprint !== authority.datasetFingerprint
  ) {
    throw new Error(
      "The native grouping profile differs from the requested archive or dataset identity.",
    );
  }
  const columns = item.columns.map((candidate, columnIndex) => {
    const path = `multimodGroupingProfile.columns[${columnIndex}]`;
    const column = exact(
      candidate,
      ["column", "label", "usedAsIndicator", "groups"],
      [],
      path,
    );
    if (!Array.isArray(column.groups))
      throw new Error(`${path}.groups must be an array.`);
    const groups = column.groups.map((groupCandidate, groupIndex) => {
      const groupPath = `${path}.groups[${groupIndex}]`;
      const group = exact(
        groupCandidate,
        ["groupId", "label", "value", "selectedRows", "completeCases"],
        [],
        groupPath,
      );
      return {
        groupId: text(group.groupId, `${groupPath}.groupId`),
        label: text(group.label, `${groupPath}.label`),
        value: parseGroupingValue(group.value, `${groupPath}.value`),
        selectedRows: count(group.selectedRows, `${groupPath}.selectedRows`),
        completeCases: count(group.completeCases, `${groupPath}.completeCases`),
      };
    });
    if (new Set(groups.map((group) => group.groupId)).size !== groups.length)
      throw new Error(`${path}.groups contains duplicate identities.`);
    return {
      column: text(column.column, `${path}.column`),
      label: text(column.label, `${path}.label`),
      usedAsIndicator: boolean(
        column.usedAsIndicator,
        `${path}.usedAsIndicator`,
      ),
      groups,
    };
  });
  if (new Set(columns.map((column) => column.column)).size !== columns.length)
    throw new Error("The native grouping profile contains duplicate columns.");
  return {
    schemaVersion: 1,
    archiveSha256,
    datasetFingerprint,
    columns,
    omittedHighCardinalityColumns: item.omittedHighCardinalityColumns.map(
      (column, index) =>
        text(
          column,
          `multimodGroupingProfile.omittedHighCardinalityColumns[${index}]`,
        ),
    ),
    sourceRecheckedUnchanged: true,
  };
}

function parseCacheReceipt(
  value: unknown,
  path: string,
): NativeMultiModExternalCacheReceiptV1 {
  const item = exact(
    value,
    [
      "schemaVersion",
      "cacheId",
      "cacheDirectory",
      "manifestSha256",
      "embeddedAuthoritySha256",
      "sourceArchiveSha256",
      "resultId",
      "recipeId",
      "target",
      "stage",
      "createdAt",
    ],
    [],
    path,
  );
  if (item.schemaVersion !== 1)
    throw new Error(`${path} has an unsupported schema.`);
  const stage = literal(item.stage, CACHE_STAGES, `${path}.stage`);
  return {
    schemaVersion: 1,
    cacheId: uuid(item.cacheId, `${path}.cacheId`),
    cacheDirectory: text(item.cacheDirectory, `${path}.cacheDirectory`),
    manifestSha256: sha(item.manifestSha256, `${path}.manifestSha256`),
    embeddedAuthoritySha256: sha(
      item.embeddedAuthoritySha256,
      `${path}.embeddedAuthoritySha256`,
    ),
    sourceArchiveSha256: sha(
      item.sourceArchiveSha256,
      `${path}.sourceArchiveSha256`,
    ),
    resultId: text(item.resultId, `${path}.resultId`),
    recipeId: uuid(item.recipeId, `${path}.recipeId`),
    target: literal(item.target, TARGETS, `${path}.target`),
    stage,
    createdAt: timestamp(item.createdAt, `${path}.createdAt`),
  };
}

function parseFailure(
  value: unknown,
  path: string,
): NativeMultiModJobFailureV1 {
  const item = exact(
    value,
    ["schemaVersion", "stage", "code", "message", "correctiveAction"],
    [],
    path,
  );
  if (item.schemaVersion !== 1)
    throw new Error(`${path}.schemaVersion must equal 1.`);
  return {
    schemaVersion: 1,
    stage: literal(
      item.stage,
      [
        "access",
        "archive_authority",
        "compilation",
        "runtime_support",
        "estimation",
        "evidence",
        "cache",
        "publication",
        "reopen",
        "integrity",
      ] as const,
      `${path}.stage`,
    ),
    code: text(item.code, `${path}.code`),
    message: text(item.message, `${path}.message`),
    correctiveAction: text(item.correctiveAction, `${path}.correctiveAction`),
  };
}

export function nativeMultiModTargetV1(
  request: MultiModRecipeConfigV1,
): NativeMultiModTargetV1 {
  if (request.kind === "pls_unobserved_heterogeneity_v2")
    return "pls_heterogeneity_v2";
  return request.kind;
}

export function stageNativeMultiModRequestV1(
  authority: NativeMultiModArchiveAuthorityV1,
  request: MultiModRecipeConfigV1,
  identity: { readonly recipeId?: string; readonly createdAt?: string } = {},
): NativeMultiModStagedRequestV1 {
  const recipeId = identity.recipeId ?? globalThis.crypto?.randomUUID?.();
  if (!recipeId)
    throw new Error("A secure staged Recipe V4 UUID is unavailable.");
  const createdAt = identity.createdAt ?? new Date().toISOString();
  const validatedAuthority = {
    archivePath: text(authority.archivePath, "authority.archivePath"),
    archiveSha256: sha(authority.archiveSha256, "authority.archiveSha256"),
    projectId: uuid(authority.projectId, "authority.projectId"),
    datasetId: uuid(authority.datasetId, "authority.datasetId"),
    datasetFingerprint: sha(
      authority.datasetFingerprint,
      "authority.datasetFingerprint",
    ),
    modelId: text(authority.modelId, "authority.modelId"),
    modelScientificSha256: sha(
      authority.modelScientificSha256,
      "authority.modelScientificSha256",
    ),
    sourceRecipeId: uuid(authority.sourceRecipeId, "authority.sourceRecipeId"),
    sourceRecipeDocumentSha256: sha(
      authority.sourceRecipeDocumentSha256,
      "authority.sourceRecipeDocumentSha256",
    ),
  };
  return {
    surface: "internal_labs_multimod_v1",
    experimentalLabsEnabled: true,
    archivePath: validatedAuthority.archivePath,
    expectedArchiveSha256: validatedAuthority.archiveSha256,
    projectId: validatedAuthority.projectId,
    datasetId: validatedAuthority.datasetId,
    datasetFingerprint: validatedAuthority.datasetFingerprint,
    modelId: validatedAuthority.modelId,
    modelScientificSha256: validatedAuthority.modelScientificSha256,
    sourceRecipeId: validatedAuthority.sourceRecipeId,
    sourceRecipeDocumentSha256: validatedAuthority.sourceRecipeDocumentSha256,
    stagedRecipeId: uuid(recipeId, "identity.recipeId"),
    stagedCreatedAt: timestamp(createdAt, "identity.createdAt"),
    config: { kind: nativeMultiModTargetV1(request), config: request.config },
  };
}

export function resumeNativeMultiModRequestV1(
  request: NativeMultiModStagedRequestV1,
  receipt: NativeMultiModExternalCacheReceiptV1,
): NativeMultiModStagedRequestV1 {
  if (
    receipt.recipeId !== request.stagedRecipeId ||
    receipt.sourceArchiveSha256 !== request.expectedArchiveSha256 ||
    receipt.target !== request.config.kind ||
    (receipt.stage === "mga_execution" &&
      request.config.kind !== "mga_multigroup_v1")
  )
    throw new Error(
      "The external cache receipt differs from the staged request or its supported resume stage.",
    );
  return { ...request, resumeCache: receipt };
}

export function parseNativeMultiModPreflightV1(
  value: unknown,
): NativeMultiModPreflightV1 {
  const item = exact(
    value,
    [
      "schemaVersion",
      "target",
      "capabilityCellId",
      "readiness",
      "stableReasonCodes",
      "stagedRecipeId",
      "stagedRecipeDocumentSha256",
      "compilationIdentitySha256",
      "mgaGroupEligibility",
    ],
    [],
    "multimodPreflight",
  );
  if (item.schemaVersion !== 1 || !Array.isArray(item.stableReasonCodes))
    throw new Error("MultiMod preflight schema or reasons are invalid.");
  return {
    schemaVersion: 1,
    target: literal(item.target, TARGETS, "multimodPreflight.target"),
    capabilityCellId: text(
      item.capabilityCellId,
      "multimodPreflight.capabilityCellId",
    ),
    readiness: literal(
      item.readiness,
      [
        "built_in_from_dataset",
        "prepared_adapter_required",
        "blocked",
      ] as const,
      "multimodPreflight.readiness",
    ),
    stableReasonCodes: item.stableReasonCodes.map((reason, index) =>
      text(reason, `multimodPreflight.stableReasonCodes[${index}]`),
    ),
    stagedRecipeId: uuid(
      item.stagedRecipeId,
      "multimodPreflight.stagedRecipeId",
    ),
    stagedRecipeDocumentSha256: sha(
      item.stagedRecipeDocumentSha256,
      "multimodPreflight.stagedRecipeDocumentSha256",
    ),
    compilationIdentitySha256: sha(
      item.compilationIdentitySha256,
      "multimodPreflight.compilationIdentitySha256",
    ),
    mgaGroupEligibility: parseMgaGroupEligibility(
      item.mgaGroupEligibility,
      "multimodPreflight.mgaGroupEligibility",
    ),
  };
}

export function parseNativeMultiModJobSnapshotV1(
  value: unknown,
): NativeMultiModJobSnapshotV1 {
  const item = exact(
    value,
    [
      "schemaVersion",
      "jobId",
      "target",
      "state",
      "phase",
      "shardId",
      "completedUnits",
      "totalUnits",
      "message",
      "warningCodes",
      "failure",
      "resumeCache",
      "queuedAt",
      "startedAt",
      "completedAt",
    ],
    [],
    "multimodJob",
  );
  if (item.schemaVersion !== 1 || !Array.isArray(item.warningCodes))
    throw new Error("MultiMod job schema or warnings are invalid.");
  const state = literal(
    item.state,
    [
      "queued",
      "running",
      "cancelling",
      "publishing",
      "completed",
      "failed",
      "cancelled",
    ] as const,
    "multimodJob.state",
  );
  const failureValue =
    item.failure === null
      ? null
      : parseFailure(item.failure, "multimodJob.failure");
  if ((state === "failed") !== Boolean(failureValue))
    throw new Error(
      "Only a failed MultiMod job may expose a terminal failure.",
    );
  const target = literal(item.target, TARGETS, "multimodJob.target");
  const resumeCache =
    item.resumeCache === null
      ? null
      : parseCacheReceipt(item.resumeCache, "multimodJob.resumeCache");
  if (
    resumeCache &&
    (resumeCache.target !== target ||
      (resumeCache.stage === "mga_execution" &&
        target !== "mga_multigroup_v1") ||
      (state === "publishing" && resumeCache.stage !== "archive_ready") ||
      (state === "completed" && resumeCache.stage !== "archive_ready"))
  )
    throw new Error(
      "The MultiMod job cache stage differs from its target or lifecycle state.",
    );
  return {
    schemaVersion: 1,
    jobId: uuid(item.jobId, "multimodJob.jobId"),
    target,
    state,
    phase: text(item.phase, "multimodJob.phase"),
    shardId: text(item.shardId, "multimodJob.shardId"),
    completedUnits: count(item.completedUnits, "multimodJob.completedUnits"),
    totalUnits: count(item.totalUnits, "multimodJob.totalUnits"),
    message: nullableText(item.message, "multimodJob.message"),
    warningCodes: item.warningCodes.map((warning, index) =>
      text(warning, `multimodJob.warningCodes[${index}]`),
    ),
    failure: failureValue,
    resumeCache,
    queuedAt: timestamp(item.queuedAt, "multimodJob.queuedAt"),
    startedAt:
      item.startedAt === null
        ? null
        : timestamp(item.startedAt, "multimodJob.startedAt"),
    completedAt:
      item.completedAt === null
        ? null
        : timestamp(item.completedAt, "multimodJob.completedAt"),
  };
}

function parseAppendReceipt(
  value: unknown,
): NativeMultiModArchiveAppendReceiptV1 {
  const item = exact(
    value,
    [
      "schema_version",
      "project_id",
      "result_id",
      "source_archive_sha256",
      "updated_archive_sha256",
      "sidecar_count",
      "source_verified_at_commit",
      "post_write_validated",
      "rollback_removed",
    ],
    [],
    "multimodResult.appendReceipt",
  );
  if (
    item.schema_version !== 1 ||
    item.source_verified_at_commit !== true ||
    item.post_write_validated !== true ||
    typeof item.rollback_removed !== "boolean"
  ) {
    throw new Error(
      "MultiMod append receipt did not prove a completed atomic publication.",
    );
  }
  return {
    schema_version: 1,
    project_id: uuid(
      item.project_id,
      "multimodResult.appendReceipt.project_id",
    ),
    result_id: text(item.result_id, "multimodResult.appendReceipt.result_id"),
    source_archive_sha256: sha(
      item.source_archive_sha256,
      "multimodResult.appendReceipt.source_archive_sha256",
    ),
    updated_archive_sha256: sha(
      item.updated_archive_sha256,
      "multimodResult.appendReceipt.updated_archive_sha256",
    ),
    sidecar_count: count(
      item.sidecar_count,
      "multimodResult.appendReceipt.sidecar_count",
    ),
    source_verified_at_commit: true,
    post_write_validated: true,
    rollback_removed: item.rollback_removed,
  };
}

export function parseNativeMultiModCompletedResultV1(
  value: unknown,
): NativeMultiModCompletedResultV1 {
  const item = exact(
    value,
    [
      "schemaVersion",
      "jobId",
      "archivePath",
      "archiveSha256",
      "projectId",
      "datasetId",
      "modelId",
      "attachment",
      "canonicalDocument",
      "appendReceipt",
      "cacheReceipt",
      "cacheRemovedAfterCommit",
    ],
    [],
    "multimodResult",
  );
  if (item.schemaVersion !== 1)
    throw new Error("MultiMod completed-result schema must equal 1.");
  const attachment = parseMultiModResultAttachmentV1(
    item.attachment,
    "multimodResult.attachment",
  );
  const canonicalDocument = parseNativeCanonicalResultDocumentV2(
    item.canonicalDocument,
  );
  const appendReceipt = parseAppendReceipt(item.appendReceipt);
  const cacheReceipt = parseCacheReceipt(
    item.cacheReceipt,
    "multimodResult.cacheReceipt",
  );
  const cacheRemovedAfterCommit = boolean(
    item.cacheRemovedAfterCommit,
    "multimodResult.cacheRemovedAfterCommit",
  );
  const archiveSha256 = sha(item.archiveSha256, "multimodResult.archiveSha256");
  const projectId = uuid(item.projectId, "multimodResult.projectId");
  const datasetId = uuid(item.datasetId, "multimodResult.datasetId");
  const modelId = text(item.modelId, "multimodResult.modelId");
  const resultTarget: NativeMultiModTargetV1 =
    attachment.result.kind === "pls_multigroup_analysis_v1"
      ? "mga_multigroup_v1"
      : attachment.result.kind === "pls_heterogeneity_analysis_v2"
        ? "pls_heterogeneity_v2"
        : attachment.result.kind === "general_sem_conditional_process_result_v2"
          ? "general_sem_conditional_process_v2"
          : "interventional_causal_mediation_v1";
  const provenance = attachment.result.analysis.provenance;
  if (
    appendReceipt.result_id !== attachment.result_id ||
    appendReceipt.updated_archive_sha256 !== archiveSha256 ||
    appendReceipt.project_id !== projectId ||
    appendReceipt.sidecar_count !== attachment.sidecars.length ||
    cacheReceipt.resultId !== attachment.result_id ||
    cacheReceipt.recipeId !== attachment.recipe_id ||
    cacheReceipt.target !== resultTarget ||
    cacheReceipt.stage !== "archive_ready" ||
    canonicalDocument.provenance.run_id !== attachment.result_id ||
    canonicalDocument.provenance.project_id !== projectId ||
    canonicalDocument.provenance.model_id !== modelId ||
    canonicalDocument.provenance.model_digest !==
      provenance.model_scientific_sha256 ||
    canonicalDocument.provenance.dataset_id !== datasetId ||
    canonicalDocument.provenance.dataset_fingerprint !==
      provenance.dataset_fingerprint ||
    canonicalDocument.provenance.recipe_id !== attachment.recipe_id ||
    canonicalDocument.provenance.recipe_digest !==
      provenance.recipe_analytical_sha256 ||
    canonicalDocument.provenance.method_version !== provenance.method_version ||
    canonicalDocument.provenance.engine_version !== provenance.engine_version ||
    canonicalDocument.provenance.seed !== provenance.seed ||
    canonicalDocument.provenance.capability_cell.registry_schema_version !==
      provenance.capability_cell.registry_schema_version ||
    canonicalDocument.provenance.capability_cell.capability_id !==
      provenance.capability_cell.capability_id ||
    canonicalDocument.provenance.capability_cell.cell_id !==
      provenance.capability_cell.cell_id ||
    canonicalDocument.provenance.capability_cell.capability_version !==
      provenance.capability_cell.capability_version
  )
    throw new Error(
      "MultiMod completed-result identities differ across strict receipts.",
    );
  return {
    schemaVersion: 1,
    jobId: uuid(item.jobId, "multimodResult.jobId"),
    archivePath: text(item.archivePath, "multimodResult.archivePath"),
    archiveSha256,
    projectId,
    datasetId,
    modelId,
    attachment,
    canonicalDocument,
    appendReceipt,
    cacheReceipt,
    cacheRemovedAfterCommit,
  };
}

export async function preflightNativeMultiModJobV1(
  request: NativeMultiModStagedRequestV1,
): Promise<NativeMultiModPreflightV1> {
  return parseNativeMultiModPreflightV1(
    await invoke<unknown>("preflight_internal_labs_multimod_v1", { request }),
  );
}

export async function prepareNativeConditionalRawProbeMetricsV2(
  staged: NativeMultiModStagedRequestV1,
  moderatorId: string,
  orientationSign: -1 | 1,
): Promise<readonly ConditionalRawProbeFitMetricReceiptV2[]> {
  if (staged.config.kind !== "general_sem_conditional_process_v2")
    throw new Error("Raw-probe metrics require a staged conditional-process V2 request.");
  const values = await invoke<unknown>(
    "prepare_internal_labs_multimod_raw_probe_metrics_v2",
    {
      request: {
        staged,
        moderatorId: text(moderatorId, "rawProbe.moderatorId"),
        orientationSign,
      },
    },
  );
  if (!Array.isArray(values) || values.length === 0)
    throw new Error("Native raw-probe preparation returned no fit-metric receipts.");
  return values.map((value, index) =>
    parseConditionalRawProbeFitMetricReceiptV2(
      value,
      moderatorId,
      `rawProbe.receipts[${index}]`,
    ));
}

export async function profileNativeMultiModGroupingV1(
  authority: NativeMultiModArchiveAuthorityV1,
): Promise<NativeMultiModGroupingProfileV1> {
  const request = {
    surface: "internal_labs_multimod_v1" as const,
    experimentalLabsEnabled: true as const,
    archivePath: authority.archivePath,
    expectedArchiveSha256: authority.archiveSha256,
    projectId: authority.projectId,
    datasetId: authority.datasetId,
    datasetFingerprint: authority.datasetFingerprint,
    modelId: authority.modelId,
    modelScientificSha256: authority.modelScientificSha256,
    sourceRecipeId: authority.sourceRecipeId,
    sourceRecipeDocumentSha256: authority.sourceRecipeDocumentSha256,
  };
  return parseNativeMultiModGroupingProfileV1(
    await invoke<unknown>("profile_internal_labs_multimod_grouping_v1", {
      request,
    }),
    authority,
  );
}

export async function startNativeMultiModJobV1(
  request: NativeMultiModStagedRequestV1,
): Promise<NativeMultiModJobSnapshotV1> {
  return parseNativeMultiModJobSnapshotV1(
    await invoke<unknown>("start_internal_labs_multimod_job_v1", { request }),
  );
}

export async function getNativeMultiModJobV1(
  jobId: string,
): Promise<NativeMultiModJobSnapshotV1> {
  return parseNativeMultiModJobSnapshotV1(
    await invoke<unknown>("status_internal_labs_multimod_job_v1", {
      jobId: uuid(jobId, "jobId"),
    }),
  );
}

export async function cancelNativeMultiModJobV1(
  jobId: string,
): Promise<NativeMultiModJobSnapshotV1> {
  return parseNativeMultiModJobSnapshotV1(
    await invoke<unknown>("cancel_internal_labs_multimod_job_v1", {
      jobId: uuid(jobId, "jobId"),
    }),
  );
}

export async function dismissNativeMultiModJobV1(jobId: string): Promise<void> {
  await invoke<void>("dismiss_internal_labs_multimod_job_v1", {
    jobId: uuid(jobId, "jobId"),
  });
}

export async function getNativeMultiModJobResultV1(
  jobId: string,
): Promise<NativeMultiModCompletedResultV1> {
  return parseNativeMultiModCompletedResultV1(
    await invoke<unknown>("result_internal_labs_multimod_job_v1", {
      jobId: uuid(jobId, "jobId"),
    }),
  );
}
