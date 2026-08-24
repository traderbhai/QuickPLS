import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type {
  CanonicalResultExportFormatV2,
  CanonicalResultPublicationQualificationV2,
  PreparedCanonicalResultExportV2,
} from "../domain/canonicalResultCrossFormatExportV2";
import { sha256HexBytesV1 } from "../domain/sha256V1";
import {
  parseMultimodCandidateQualificationReceiptV1,
  type MultimodCandidateQualificationReceiptV1,
} from "../domain/multimodContractsV1";

const NATIVE_CANONICAL_EXPORT_PUBLICATION_SCHEMA_VERSION_V2 = 2 as const;
const NATIVE_CANONICAL_EXPORT_PUBLICATION_COMMAND_V2 = "publish_canonical_result_export_v2";
const CANONICAL_EXPORT_FORMATS_V2 = ["csv", "xlsx", "json", "html", "pdf", "svg", "png"] as const;

interface NativeCanonicalResultExportIdentityV2 {
  documentId: string;
  runId: string;
  projectId: string;
  modelId: string;
  modelDigest: string;
  datasetId: string;
  datasetFingerprint: string;
  recipeId: string;
  recipeDigest: string;
  capabilityCellId: string;
  methodVersion: string;
  engineVersion: string;
  publicationQualification?: CanonicalResultPublicationQualificationV2;
  candidateQualificationReceipt?: MultimodCandidateQualificationReceiptV1;
  stableTableIds: string[];
  stableChartIds: string[];
  semanticSha256: string;
}

type NativeCanonicalResultExportPayloadV2 =
  | {
      kind: "exact_bytes";
      contentsBase64: string;
      byteLength: number;
      sha256: string;
    }
  | {
      kind: "xlsx_tables_json";
      tablesJson: string;
      byteLength: number;
      sha256: string;
    };

interface NativeCanonicalResultExportPublicationRequestV2 {
  schemaVersion: typeof NATIVE_CANONICAL_EXPORT_PUBLICATION_SCHEMA_VERSION_V2;
  format: CanonicalResultExportFormatV2;
  destinationPath: string;
  identity: NativeCanonicalResultExportIdentityV2;
  payload: NativeCanonicalResultExportPayloadV2;
}

export interface NativeCanonicalResultExportPublicationReceiptV2 {
  schemaVersion: typeof NATIVE_CANONICAL_EXPORT_PUBLICATION_SCHEMA_VERSION_V2;
  format: CanonicalResultExportFormatV2;
  path: string;
  bytes: number;
  sha256: string;
  payloadSha256: string;
  identity: NativeCanonicalResultExportIdentityV2;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isFormat(value: unknown): value is CanonicalResultExportFormatV2 {
  return typeof value === "string" && CANONICAL_EXPORT_FORMATS_V2.includes(value as CanonicalResultExportFormatV2);
}

function isLowerSha256(value: unknown): value is string {
  return typeof value === "string" && /^[a-f0-9]{64}$/u.test(value);
}

function bytesToBase64(bytes: Uint8Array): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const chunks: string[] = [];
  let chunk = "";
  for (let index = 0; index < bytes.length; index += 3) {
    const first = bytes[index] ?? 0;
    const second = bytes[index + 1] ?? 0;
    const third = bytes[index + 2] ?? 0;
    const combined = (first << 16) | (second << 8) | third;
    chunk += alphabet[(combined >>> 18) & 63];
    chunk += alphabet[(combined >>> 12) & 63];
    chunk += index + 1 < bytes.length ? alphabet[(combined >>> 6) & 63] : "=";
    chunk += index + 2 < bytes.length ? alphabet[combined & 63] : "=";
    if (chunk.length >= 32_768) {
      chunks.push(chunk);
      chunk = "";
    }
  }
  if (chunk) chunks.push(chunk);
  return chunks.join("");
}

function exactArtifactBytes(artifact: PreparedCanonicalResultExportV2): Uint8Array | null {
  switch (artifact.format) {
    case "xlsx": return null;
    case "pdf":
    case "png": return artifact.bytes;
    case "csv":
    case "json":
    case "html":
    case "svg": return new TextEncoder().encode(artifact.contents);
  }
}

function publicationIdentity(
  artifact: PreparedCanonicalResultExportV2,
): NativeCanonicalResultExportIdentityV2 {
  const { semantic } = artifact;
  let candidateQualificationReceipt: MultimodCandidateQualificationReceiptV1 | undefined;
  if (semantic.candidate_qualification_receipt !== undefined) {
    candidateQualificationReceipt = parseMultimodCandidateQualificationReceiptV1(
      semantic.candidate_qualification_receipt,
      "canonical_export.candidateQualificationReceipt",
    );
  }
  if ((semantic.publication_qualification === "release_qualified_candidate")
    !== (candidateQualificationReceipt !== undefined)) {
    throw new Error("Canonical candidate state and embedded-authority receipt are inconsistent.");
  }
  return {
    documentId: semantic.source.document_id,
    runId: semantic.provenance.run_id,
    projectId: semantic.provenance.project_id,
    modelId: semantic.provenance.model_id,
    modelDigest: semantic.provenance.model_digest,
    datasetId: semantic.provenance.dataset_id,
    datasetFingerprint: semantic.provenance.dataset_fingerprint,
    recipeId: semantic.provenance.recipe_id,
    recipeDigest: semantic.provenance.recipe_digest,
    capabilityCellId: semantic.provenance.capability_cell.cell_id,
    methodVersion: semantic.provenance.method_version,
    engineVersion: semantic.provenance.engine_version,
    ...(semantic.publication_qualification !== undefined
      ? { publicationQualification: semantic.publication_qualification }
      : {}),
    ...(candidateQualificationReceipt !== undefined
      ? { candidateQualificationReceipt }
      : {}),
    stableTableIds: [...semantic.selection.table_ids],
    stableChartIds: [...semantic.selection.chart_ids],
    semanticSha256: semantic.semantic_sha256,
  };
}

function publicationPayload(
  artifact: PreparedCanonicalResultExportV2,
): NativeCanonicalResultExportPayloadV2 {
  if (artifact.format === "xlsx") {
    const tablesJson = JSON.stringify(artifact.workbookTables);
    const bytes = new TextEncoder().encode(tablesJson);
    return {
      kind: "xlsx_tables_json",
      tablesJson,
      byteLength: bytes.length,
      sha256: sha256HexBytesV1(bytes),
    };
  }
  const bytes = exactArtifactBytes(artifact);
  if (!bytes) throw new Error("The canonical export payload kind is unavailable.");
  return {
    kind: "exact_bytes",
    contentsBase64: bytesToBase64(bytes),
    byteLength: bytes.length,
    sha256: sha256HexBytesV1(bytes),
  };
}

function sameIdentity(
  actual: unknown,
  expected: NativeCanonicalResultExportIdentityV2,
): actual is NativeCanonicalResultExportIdentityV2 {
  return isRecord(actual)
    && actual.documentId === expected.documentId
    && actual.runId === expected.runId
    && actual.projectId === expected.projectId
    && actual.modelId === expected.modelId
    && actual.modelDigest === expected.modelDigest
    && actual.datasetId === expected.datasetId
    && actual.datasetFingerprint === expected.datasetFingerprint
    && actual.recipeId === expected.recipeId
    && actual.recipeDigest === expected.recipeDigest
    && actual.capabilityCellId === expected.capabilityCellId
    && actual.methodVersion === expected.methodVersion
    && actual.engineVersion === expected.engineVersion
    && actual.publicationQualification === expected.publicationQualification
    && JSON.stringify(actual.candidateQualificationReceipt ?? null)
      === JSON.stringify(expected.candidateQualificationReceipt ?? null)
    && JSON.stringify(actual.stableTableIds) === JSON.stringify(expected.stableTableIds)
    && JSON.stringify(actual.stableChartIds) === JSON.stringify(expected.stableChartIds)
    && actual.semanticSha256 === expected.semanticSha256;
}

function parseReceipt(
  value: unknown,
  request: NativeCanonicalResultExportPublicationRequestV2,
): NativeCanonicalResultExportPublicationReceiptV2 {
  if (!isRecord(value)
    || value.schemaVersion !== NATIVE_CANONICAL_EXPORT_PUBLICATION_SCHEMA_VERSION_V2
    || value.format !== request.format
    || value.path !== request.destinationPath
    || !Number.isSafeInteger(value.bytes)
    || (value.bytes as number) <= 0
    || !isLowerSha256(value.sha256)
    || value.payloadSha256 !== request.payload.sha256
    || !sameIdentity(value.identity, request.identity)) {
    throw new Error("The native canonical export publication receipt does not match the verified handoff.");
  }
  if (request.payload.kind === "exact_bytes"
    && (value.bytes !== request.payload.byteLength || value.sha256 !== request.payload.sha256)) {
    throw new Error("The native canonical export bytes differ from the exact verified payload.");
  }
  return value as unknown as NativeCanonicalResultExportPublicationReceiptV2;
}

function assertPreparedArtifactFormat(artifact: PreparedCanonicalResultExportV2): void {
  if (!isFormat((artifact as { format?: unknown }).format)) {
    throw new Error("Unknown canonical export format; no destination was opened.");
  }
  if (artifact.extension !== artifact.format
    || !artifact.defaultFileName.toLowerCase().endsWith(`.${artifact.format}`)) {
    throw new Error("The prepared canonical export filename does not match its format.");
  }
}

/**
 * Publishes an already semantically verified cross-format artifact through the
 * native no-replace writer. Save-dialog cancellation and an AbortSignal that
 * fires before invoke both perform no native filesystem call.
 */
export async function publishNativeCanonicalResultExportV2(
  artifact: PreparedCanonicalResultExportV2,
  signal?: AbortSignal,
): Promise<NativeCanonicalResultExportPublicationReceiptV2 | null> {
  assertPreparedArtifactFormat(artifact);
  if (signal?.aborted) return null;
  const selected = await save({
    defaultPath: artifact.defaultFileName,
    filters: [{
      name: `QuickPLS verified ${artifact.format.toUpperCase()} export`,
      extensions: [artifact.extension],
    }],
  });
  if (typeof selected !== "string" || !selected) return null;
  if (signal?.aborted) return null;
  if (!selected.toLowerCase().endsWith(`.${artifact.format}`)) {
    throw new Error(`The selected destination must end in .${artifact.format}; no file was written.`);
  }
  return publishNativeCanonicalResultExportAtV2(artifact, selected, signal);
}

/**
 * Publishes to an already selected absolute destination. The ordinary product
 * flow continues to use the Save-dialog wrapper above; this seam lets the
 * build-only packaged qualification bridge exercise the identical typed
 * handoff and native no-replace publisher without automating a system dialog.
 */
export async function publishNativeCanonicalResultExportAtV2(
  artifact: PreparedCanonicalResultExportV2,
  destinationPath: string,
  signal?: AbortSignal,
): Promise<NativeCanonicalResultExportPublicationReceiptV2 | null> {
  assertPreparedArtifactFormat(artifact);
  if (signal?.aborted) return null;
  if (!destinationPath || destinationPath.trim() !== destinationPath) {
    throw new Error("The canonical export destination must be nonempty exact text.");
  }
  if (!destinationPath.toLowerCase().endsWith(`.${artifact.format}`)) {
    throw new Error(`The selected destination must end in .${artifact.format}; no file was written.`);
  }
  const request: NativeCanonicalResultExportPublicationRequestV2 = {
    schemaVersion: NATIVE_CANONICAL_EXPORT_PUBLICATION_SCHEMA_VERSION_V2,
    format: artifact.format,
    destinationPath,
    identity: publicationIdentity(artifact),
    payload: publicationPayload(artifact),
  };
  if (signal?.aborted) return null;
  const response = await invoke<unknown>(NATIVE_CANONICAL_EXPORT_PUBLICATION_COMMAND_V2, { request });
  return parseReceipt(response, request);
}
