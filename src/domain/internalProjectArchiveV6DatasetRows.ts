import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./internalProjectArchiveV6Read";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1,
  internalProjectArchiveV6AccessPairV1,
  type InternalProjectArchiveV6AccessV1,
} from "./internalProjectArchiveV6Access";
import { supportsGeneralSemV1 } from "./internalProjectArchiveV6Wire";
import type { Dataset } from "../types";

const LOWER_SHA256 = /^[0-9a-f]{64}$/;
const MAX_PAGE_SIZE = 500;

type WireRecord = Record<string, unknown>;

/** Historical surface retained for callers that explicitly request Labs. */
export const INTERNAL_PROJECT_ARCHIVE_V6_DATASET_ROWS_SURFACE =
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1.surface;

export type InternalProjectArchiveV6DatasetRowsRequestV1 =
  InternalProjectArchiveV6AccessV1 & {
  archivePath: string;
  expectedArchiveSha256: string;
  projectId: string;
  datasetId: string;
  datasetFingerprint: string;
  offset: number;
  limit: number;
};

export interface InternalProjectArchiveV6DatasetRowsPageV1 {
  schemaVersion: 1;
  archivePath: string;
  archiveSha256: string;
  projectId: string;
  datasetId: string;
  datasetFingerprint: string;
  offset: number;
  limit: number;
  rowCount: number;
  columns: string[];
  rows: Dataset["rows"];
  sourceRecheckedUnchanged: true;
}

export interface InternalProjectArchiveV6DatasetRowsDiagnosticV1 {
  code: string;
  message: string;
  correctiveAction: string;
}

export type InternalProjectArchiveV6DatasetRowsOutcomeV1 =
  | { status: "ok"; value: InternalProjectArchiveV6DatasetRowsPageV1 }
  | { status: "blocked"; diagnostic: InternalProjectArchiveV6DatasetRowsDiagnosticV1 };

export class InternalProjectArchiveV6DatasetRowsWireError extends Error {
  constructor(
    public readonly code: string,
    public readonly path: string,
    message: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6DatasetRowsWireError";
  }
}

function fail(code: string, path: string, message: string): never {
  throw new InternalProjectArchiveV6DatasetRowsWireError(code, path, message);
}

function recordAt(value: unknown, path: string): WireRecord {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("schema6_dataset_rows.object_required", path, `${path} must be an object.`);
  }
  return value as WireRecord;
}

function exactRecordAt(value: unknown, fields: readonly string[], path: string): WireRecord {
  const record = recordAt(value, path);
  const allowed = new Set(fields);
  for (const field of fields) {
    if (!Object.prototype.hasOwnProperty.call(record, field)) {
      fail("schema6_dataset_rows.field_missing", `${path}.${field}`, `${path}.${field} is required.`);
    }
  }
  for (const field of Object.keys(record)) {
    if (!allowed.has(field)) {
      fail("schema6_dataset_rows.field_unknown", `${path}.${field}`, `${path}.${field} is not part of the strict row-page contract.`);
    }
  }
  return record;
}

function textAt(value: unknown, path: string): string {
  if (typeof value !== "string" || !value.trim()) {
    fail("schema6_dataset_rows.text_required", path, `${path} must be a nonempty string.`);
  }
  return value;
}

function sha256At(value: unknown, path: string): string {
  const digest = textAt(value, path);
  if (!LOWER_SHA256.test(digest)) {
    fail(
      "schema6_dataset_rows.archive_sha256_invalid",
      path,
      `${path} must be a lowercase SHA-256 digest.`,
    );
  }
  return digest;
}

function countAt(value: unknown, path: string, positive = false): number {
  if (!Number.isSafeInteger(value) || Object.is(value, -0) || (value as number) < (positive ? 1 : 0)) {
    fail("schema6_dataset_rows.count_invalid", path, `${path} must be a ${positive ? "positive" : "nonnegative"} safe integer.`);
  }
  return value as number;
}

function diagnosticAt(value: unknown): InternalProjectArchiveV6DatasetRowsDiagnosticV1 {
  const path = "outcome.diagnostic";
  const record = exactRecordAt(value, ["code", "message", "correctiveAction"], path);
  return {
    code: textAt(record.code, `${path}.code`),
    message: textAt(record.message, `${path}.message`),
    correctiveAction: textAt(record.correctiveAction, `${path}.correctiveAction`),
  };
}

function rowAt(value: unknown, columns: readonly string[], path: string): Dataset["rows"][number] {
  const row = recordAt(value, path);
  const expected = new Set(columns);
  for (const column of columns) {
    if (!Object.prototype.hasOwnProperty.call(row, column)) {
      fail("schema6_dataset_rows.row_column_missing", `${path}.${column}`, `The strict row page omitted ${column}.`);
    }
  }
  for (const [column, cell] of Object.entries(row)) {
    if (!expected.has(column)) {
      fail("schema6_dataset_rows.row_column_unknown", `${path}.${column}`, `The strict row page returned an undeclared column.`);
    }
    if (cell !== null && typeof cell !== "string" && typeof cell !== "number") {
      fail("schema6_dataset_rows.cell_invalid", `${path}.${column}`, `Dataset cells must be strings, numbers, or null.`);
    }
  }
  return row as Dataset["rows"][number];
}

export function parseInternalProjectArchiveV6DatasetRowsRequestV1(
  input: unknown,
): InternalProjectArchiveV6DatasetRowsRequestV1 {
  const path = "request";
  const request = exactRecordAt(
    input,
    [
      "surface",
      "experimentalLabsEnabled",
      "archivePath",
      "expectedArchiveSha256",
      "projectId",
      "datasetId",
      "datasetFingerprint",
      "offset",
      "limit",
    ],
    path,
  );
  const access = internalProjectArchiveV6AccessPairV1(
    request.surface,
    request.experimentalLabsEnabled,
  );
  if (!access) {
    fail(
      "schema6_dataset_rows.surface_pair_invalid",
      path,
      "Dataset paging requires exact internal_labs/true or standard_multimod_v1/false access.",
    );
  }
  const offset = countAt(request.offset, `${path}.offset`);
  const limit = countAt(request.limit, `${path}.limit`, true);
  if (limit > MAX_PAGE_SIZE) {
    fail(
      "schema6_dataset_rows.page_bounds_invalid",
      `${path}.limit`,
      `Dataset page limit must be from 1 through ${MAX_PAGE_SIZE}.`,
    );
  }
  return {
    ...access,
    archivePath: textAt(request.archivePath, `${path}.archivePath`),
    expectedArchiveSha256: sha256At(
      request.expectedArchiveSha256,
      `${path}.expectedArchiveSha256`,
    ),
    projectId: textAt(request.projectId, `${path}.projectId`),
    datasetId: textAt(request.datasetId, `${path}.datasetId`),
    datasetFingerprint: textAt(
      request.datasetFingerprint,
      `${path}.datasetFingerprint`,
    ),
    offset,
    limit,
  };
}

/** Builds an exact archive-bound request from the current strict read receipt. */
export function buildInternalProjectArchiveV6DatasetRowsRequestV1(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  datasetId: string,
  offset: number,
  limit: number,
  access: InternalProjectArchiveV6AccessV1 =
    INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1,
): InternalProjectArchiveV6DatasetRowsRequestV1 {
  if (!supportsGeneralSemV1(snapshot.project)) {
    fail("schema6_dataset_rows.general_sem_required", "snapshot.project.sem_generation", "Strict archive paging is limited to marked General SEM projects.");
  }
  const authority = snapshot.generalSemExecutionAuthority;
  if (!authority) {
    fail("schema6_dataset_rows.execution_authority_required", "snapshot.generalSemExecutionAuthority", "The marked project has no exact native execution authority.");
  }
  const descriptor = snapshot.project.datasets.find((candidate) => candidate.id === datasetId);
  if (!descriptor
    || authority.projectId !== snapshot.project.project_id
    || authority.datasetId !== descriptor.id
    || authority.datasetFingerprint !== descriptor.fingerprint) {
    fail("schema6_dataset_rows.authority_mismatch", "snapshot", "The requested dataset is not the dataset bound by the current General SEM execution authority.");
  }
  if (!LOWER_SHA256.test(snapshot.archiveSha256)) {
    fail("schema6_dataset_rows.archive_sha256_invalid", "snapshot.archiveSha256", "The strict snapshot archive digest is invalid.");
  }
  if (!Number.isSafeInteger(offset) || offset < 0 || !Number.isSafeInteger(limit) || limit < 1 || limit > MAX_PAGE_SIZE) {
    fail("schema6_dataset_rows.page_bounds_invalid", "request", `Dataset pages require a nonnegative offset and a limit from 1 through ${MAX_PAGE_SIZE}.`);
  }
  return parseInternalProjectArchiveV6DatasetRowsRequestV1({
    ...access,
    archivePath: snapshot.archivePath,
    expectedArchiveSha256: snapshot.archiveSha256,
    projectId: snapshot.project.project_id,
    datasetId: descriptor.id,
    datasetFingerprint: descriptor.fingerprint,
    offset,
    limit,
  });
}

export function parseInternalProjectArchiveV6DatasetRowsOutcomeV1(
  value: unknown,
  request: InternalProjectArchiveV6DatasetRowsRequestV1,
): InternalProjectArchiveV6DatasetRowsOutcomeV1 {
  request = parseInternalProjectArchiveV6DatasetRowsRequestV1(request);
  const outcome = recordAt(value, "outcome");
  if (outcome.status === "blocked") {
    exactRecordAt(outcome, ["status", "diagnostic"], "outcome");
    return { status: "blocked", diagnostic: diagnosticAt(outcome.diagnostic) };
  }
  if (outcome.status !== "ok") {
    fail("schema6_dataset_rows.status_invalid", "outcome.status", "The strict row-page outcome status is invalid.");
  }
  exactRecordAt(outcome, ["status", "value"], "outcome");
  const path = "outcome.value";
  const page = exactRecordAt(outcome.value, [
    "schemaVersion", "archivePath", "archiveSha256", "projectId", "datasetId",
    "datasetFingerprint", "offset", "limit", "rowCount", "columns", "rows",
    "sourceRecheckedUnchanged",
  ], path);
  if (page.schemaVersion !== 1 || page.sourceRecheckedUnchanged !== true) {
    fail("schema6_dataset_rows.receipt_invalid", path, "The strict row-page receipt is not version 1 or did not recheck the source.");
  }
  const archivePath = textAt(page.archivePath, `${path}.archivePath`);
  const archiveSha256 = textAt(page.archiveSha256, `${path}.archiveSha256`);
  const projectId = textAt(page.projectId, `${path}.projectId`);
  const datasetId = textAt(page.datasetId, `${path}.datasetId`);
  const datasetFingerprint = textAt(page.datasetFingerprint, `${path}.datasetFingerprint`);
  const offset = countAt(page.offset, `${path}.offset`);
  const limit = countAt(page.limit, `${path}.limit`, true);
  const rowCount = countAt(page.rowCount, `${path}.rowCount`);
  if (archivePath !== request.archivePath
    || archiveSha256 !== request.expectedArchiveSha256
    || projectId !== request.projectId
    || datasetId !== request.datasetId
    || datasetFingerprint !== request.datasetFingerprint
    || offset !== Math.min(request.offset, rowCount)
    || limit !== request.limit) {
    fail("schema6_dataset_rows.response_identity_mismatch", path, "The strict row page differs from the requested archive, project, dataset, fingerprint, or page identity.");
  }
  if (!LOWER_SHA256.test(archiveSha256)) {
    fail("schema6_dataset_rows.archive_sha256_invalid", `${path}.archiveSha256`, "The row-page archive digest is invalid.");
  }
  if (!Array.isArray(page.columns) || page.columns.some((column) => typeof column !== "string" || !column.trim())) {
    fail("schema6_dataset_rows.columns_invalid", `${path}.columns`, "The row-page columns must be nonempty strings.");
  }
  const columns = [...page.columns] as string[];
  if (new Set(columns).size !== columns.length) {
    fail("schema6_dataset_rows.columns_duplicate", `${path}.columns`, "The row-page columns must be unique.");
  }
  if (!Array.isArray(page.rows)) {
    fail("schema6_dataset_rows.rows_invalid", `${path}.rows`, "The row page rows must be an array.");
  }
  const maximumRows = Math.min(limit, Math.max(0, rowCount - offset));
  if (page.rows.length !== maximumRows) {
    fail("schema6_dataset_rows.row_count_mismatch", `${path}.rows`, "The strict row page has an unexpected number of rows.");
  }
  const rows = page.rows.map((row, index) => rowAt(row, columns, `${path}.rows[${index}]`));
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      archivePath,
      archiveSha256,
      projectId,
      datasetId,
      datasetFingerprint,
      offset,
      limit,
      rowCount,
      columns,
      rows,
      sourceRecheckedUnchanged: true,
    },
  };
}
