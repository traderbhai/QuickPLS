import {
  parseInternalProjectArchiveV6ReadOutcomeV1,
  type InternalProjectArchiveV6ReadSnapshotV1,
} from "./internalProjectArchiveV6Read";
import {
  parseInternalProjectArchiveV6Wire,
  type InternalProjectArchiveV6Wire,
} from "./internalProjectArchiveV6Wire";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1,
  internalProjectArchiveV6AccessPairV1,
  type InternalProjectArchiveV6AccessV1,
} from "./internalProjectArchiveV6Access";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
type WireRecord = Record<string, unknown>;

/** Historical surface retained for callers that explicitly request Labs. */
export const INTERNAL_PROJECT_ARCHIVE_V6_SAVE_COPY_SURFACE =
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1.surface;

export type InternalProjectArchiveV6SaveCopyRequestV1 =
  InternalProjectArchiveV6AccessV1 & {
  sourceArchivePath: string;
  expectedSourceArchiveSha256: string;
  destinationArchivePath: string;
  project: InternalProjectArchiveV6Wire;
};

export interface InternalProjectArchiveV6SaveCopyReceiptV1 {
  schemaVersion: 1;
  sourceArchivePath: string;
  sourceArchiveSha256: string;
  sourceVerifiedUnchanged: true;
  destinationArchivePath: string;
  destinationArchiveSha256: string;
  destinationArchiveBytes: number;
  strictReopenValidated: true;
  modelCount: number;
}

export interface InternalProjectArchiveV6SaveCopyResultV1 {
  schemaVersion: 1;
  persistence: "persisted_new_copy";
  receipt: InternalProjectArchiveV6SaveCopyReceiptV1;
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
}

export interface InternalProjectArchiveV6SaveCopyDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
}

export type InternalProjectArchiveV6SaveCopyOutcomeV1 =
  | { status: "ok"; value: InternalProjectArchiveV6SaveCopyResultV1 }
  | { status: "blocked"; diagnostic: InternalProjectArchiveV6SaveCopyDiagnosticV1 };

export class InternalProjectArchiveV6SaveCopyWireError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6SaveCopyWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalProjectArchiveV6SaveCopyWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("schema6_save_copy.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(value: unknown, fields: readonly string[], path: string): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(fields);
  for (const field of fields) {
    if (!Object.prototype.hasOwnProperty.call(record, field)) {
      fail("schema6_save_copy.field_missing", `${path}.${field}`, `${path}.${field} is required.`);
    }
  }
  for (const field of Object.keys(record)) {
    if (!allowed.has(field)) {
      fail("schema6_save_copy.field_unknown", `${path}.${field}`, `${path}.${field} is not allowed.`);
    }
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    fail("schema6_save_copy.text_required", path, `${path} must be a nonempty string.`);
  }
  return value;
}

function sha256At(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) {
    fail("schema6_save_copy.sha256_invalid", path, `${path} must be a lowercase SHA-256 digest.`);
  }
  return digest;
}

function countAt(value: unknown, path: string, positive = false): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < (positive ? 1 : 0)) {
    fail("schema6_save_copy.count_invalid", path, `${path} must be a ${positive ? "positive" : "nonnegative"} safe integer.`);
  }
  return value as number;
}

export function parseInternalProjectArchiveV6SaveCopyRequestV1(
  input: unknown,
): InternalProjectArchiveV6SaveCopyRequestV1 {
  const path = "request";
  const request = exactRecordAt(input, [
    "surface",
    "experimentalLabsEnabled",
    "sourceArchivePath",
    "expectedSourceArchiveSha256",
    "destinationArchivePath",
    "project",
  ], path);
  const access = internalProjectArchiveV6AccessPairV1(
    request.surface,
    request.experimentalLabsEnabled,
  );
  if (!access) {
    fail(
      "schema6_save_copy.surface_pair_invalid",
      path,
      "Save copy requires exact internal_labs/true or standard_multimod_v1/false access.",
    );
  }
  const sourceArchivePath = textAt(request.sourceArchivePath, `${path}.sourceArchivePath`);
  const destinationArchivePath = textAt(request.destinationArchivePath, `${path}.destinationArchivePath`);
  if (sourceArchivePath !== sourceArchivePath.trim()
    || destinationArchivePath !== destinationArchivePath.trim()) {
    fail("schema6_save_copy.path_not_canonical", path, "Archive paths cannot have surrounding whitespace.");
  }
  if (sourceArchivePath.toLowerCase() === destinationArchivePath.toLowerCase()) {
    fail("schema6_save_copy.new_destination_required", `${path}.destinationArchivePath`, "Save copy requires a different destination path.");
  }
  return {
    ...access,
    sourceArchivePath,
    expectedSourceArchiveSha256: sha256At(
      request.expectedSourceArchiveSha256,
      `${path}.expectedSourceArchiveSha256`,
    ),
    destinationArchivePath,
    project: parseInternalProjectArchiveV6Wire(request.project),
  };
}

function parseReceipt(
  value: unknown,
  request: InternalProjectArchiveV6SaveCopyRequestV1,
): InternalProjectArchiveV6SaveCopyReceiptV1 {
  const path = "outcome.value.receipt";
  const receipt = exactRecordAt(value, [
    "schemaVersion",
    "sourceArchivePath",
    "sourceArchiveSha256",
    "sourceVerifiedUnchanged",
    "destinationArchivePath",
    "destinationArchiveSha256",
    "destinationArchiveBytes",
    "strictReopenValidated",
    "modelCount",
  ], path);
  if (receipt.schemaVersion !== 1
    || receipt.sourceVerifiedUnchanged !== true
    || receipt.strictReopenValidated !== true) {
    fail("schema6_save_copy.receipt_invalid", path, "The native writer receipt must prove source stability and strict reopen validation.");
  }
  const parsed: InternalProjectArchiveV6SaveCopyReceiptV1 = {
    schemaVersion: 1,
    sourceArchivePath: textAt(receipt.sourceArchivePath, `${path}.sourceArchivePath`),
    sourceArchiveSha256: sha256At(receipt.sourceArchiveSha256, `${path}.sourceArchiveSha256`),
    sourceVerifiedUnchanged: true,
    destinationArchivePath: textAt(receipt.destinationArchivePath, `${path}.destinationArchivePath`),
    destinationArchiveSha256: sha256At(receipt.destinationArchiveSha256, `${path}.destinationArchiveSha256`),
    destinationArchiveBytes: countAt(receipt.destinationArchiveBytes, `${path}.destinationArchiveBytes`, true),
    strictReopenValidated: true,
    modelCount: countAt(receipt.modelCount, `${path}.modelCount`),
  };
  if (parsed.sourceArchivePath !== request.sourceArchivePath
    || parsed.sourceArchiveSha256 !== request.expectedSourceArchiveSha256
    || parsed.destinationArchivePath !== request.destinationArchivePath
    || parsed.modelCount !== request.project.models.length) {
    fail("schema6_save_copy.receipt_request_mismatch", path, "The native writer receipt differs from the exact save-copy request.");
  }
  return parsed;
}

export function parseInternalProjectArchiveV6SaveCopyOutcomeV1(
  input: unknown,
  request: InternalProjectArchiveV6SaveCopyRequestV1,
): InternalProjectArchiveV6SaveCopyOutcomeV1 {
  request = parseInternalProjectArchiveV6SaveCopyRequestV1(request);
  const outcome = recordAt(input, "outcome");
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
  if (outcome.status !== "ok") {
    return fail("schema6_save_copy.status_invalid", "outcome.status", "Save copy must return ok or blocked.");
  }
  exactRecordAt(outcome, ["status", "value"], "outcome");
  const value = exactRecordAt(outcome.value, [
    "schemaVersion",
    "persistence",
    "receipt",
    "snapshot",
  ], "outcome.value");
  if (value.schemaVersion !== 1 || value.persistence !== "persisted_new_copy") {
    fail("schema6_save_copy.result_invalid", "outcome.value", "Save copy must return a version-1 persisted-new-copy result.");
  }
  const receipt = parseReceipt(value.receipt, request);
  const readOutcome = parseInternalProjectArchiveV6ReadOutcomeV1({
    status: "ok",
    value: value.snapshot,
  });
  if (readOutcome.status !== "ok") {
    return fail("schema6_save_copy.snapshot_invalid", "outcome.value.snapshot", "Save copy must return a strict destination snapshot.");
  }
  const snapshot = readOutcome.value;
  if (snapshot.archivePath !== receipt.destinationArchivePath
    || snapshot.archiveSha256 !== receipt.destinationArchiveSha256
    || snapshot.archiveBytes !== receipt.destinationArchiveBytes
    || JSON.stringify(snapshot.project) !== JSON.stringify(request.project)) {
    fail("schema6_save_copy.snapshot_receipt_mismatch", "outcome.value.snapshot", "The strict destination snapshot differs from the receipt or detached project.");
  }
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      persistence: "persisted_new_copy",
      receipt,
      snapshot,
    },
  };
}
