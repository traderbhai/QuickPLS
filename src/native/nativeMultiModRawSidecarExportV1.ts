import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type {
  MultiModResultAttachmentV1,
  MultimodResultSidecarDescriptorV1,
} from "../domain/multimodContractsV1";

const COMMAND = "publish_internal_labs_multimod_raw_sidecar_v1";
const SHA256 = /^[a-f0-9]{64}$/u;

export interface NativeMultiModRawSidecarExportAuthorityV1 {
  readonly archivePath: string;
  readonly archiveSha256: string;
  readonly projectId: string;
}

export interface NativeMultiModRawSidecarExportReceiptV1 {
  readonly schemaVersion: 1;
  readonly archiveSha256: string;
  readonly projectId: string;
  readonly resultId: string;
  readonly entryName: string;
  readonly identitySha256: string;
  readonly path: string;
  readonly bytes: number;
  readonly sha256: string;
  readonly strictReopenValidated: true;
  readonly noReplacePublication: true;
}

const ELIGIBLE_SUFFIXES = [
  "-posteriors.arrow",
  "-memberships.arrow",
  "-assignments.arrow",
  "-hard-assignments.arrow",
  "-ledger.arrow",
  "-target-vectors.arrow",
  "-draw-rows.arrow",
  "-records.arrow",
  "-counts.arrow",
  "-usable-indices.arrow",
] as const;

export function isNativeMultiModRawSidecarExportableV1(
  resultId: string,
  descriptor: MultimodResultSidecarDescriptorV1,
): boolean {
  const prefix = `results/${resultId}/`;
  if (!descriptor.entry_name.startsWith(prefix)) return false;
  const leaf = descriptor.entry_name.slice(prefix.length);
  return Boolean(
    leaf &&
      !leaf.includes("/") &&
      !leaf.includes("\\") &&
      !leaf.includes("..") &&
      ELIGIBLE_SUFFIXES.some((suffix) => leaf.endsWith(suffix)),
  );
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("The native raw-sidecar receipt must be an object.");
  }
  return value as Record<string, unknown>;
}

function parseReceipt(
  value: unknown,
  expected: {
    authority: NativeMultiModRawSidecarExportAuthorityV1;
    attachment: MultiModResultAttachmentV1;
    descriptor: MultimodResultSidecarDescriptorV1;
    destinationPath: string;
  },
): NativeMultiModRawSidecarExportReceiptV1 {
  const item = record(value);
  const keys = [
    "schemaVersion",
    "archiveSha256",
    "projectId",
    "resultId",
    "entryName",
    "identitySha256",
    "path",
    "bytes",
    "sha256",
    "strictReopenValidated",
    "noReplacePublication",
  ] as const;
  if (
    Object.keys(item).length !== keys.length ||
    keys.some((key) => !Object.prototype.hasOwnProperty.call(item, key)) ||
    item.schemaVersion !== 1 ||
    item.archiveSha256 !== expected.authority.archiveSha256 ||
    item.projectId !== expected.authority.projectId ||
    item.resultId !== expected.attachment.result_id ||
    item.entryName !== expected.descriptor.entry_name ||
    item.identitySha256 !== expected.attachment.identity_sha256 ||
    item.path !== expected.destinationPath ||
    item.sha256 !== expected.descriptor.sha256 ||
    item.strictReopenValidated !== true ||
    item.noReplacePublication !== true ||
    !Number.isSafeInteger(item.bytes) ||
    (item.bytes as number) !== expected.descriptor.uncompressed_bytes ||
    !SHA256.test(String(item.sha256))
  ) {
    throw new Error(
      "The native raw-sidecar publication receipt differs from the strict completed-result descriptor.",
    );
  }
  return item as unknown as NativeMultiModRawSidecarExportReceiptV1;
}

export async function publishNativeMultiModRawSidecarV1(
  authority: NativeMultiModRawSidecarExportAuthorityV1,
  attachment: MultiModResultAttachmentV1,
  descriptor: MultimodResultSidecarDescriptorV1,
): Promise<NativeMultiModRawSidecarExportReceiptV1 | null> {
  if (
    descriptor.identity_sha256 !== attachment.identity_sha256 ||
    !isNativeMultiModRawSidecarExportableV1(attachment.result_id, descriptor)
  ) {
    throw new Error(
      "This descriptor is not an eligible posterior, membership, assignment, replicate-ledger, or target-vector Arrow payload.",
    );
  }
  const leaf = descriptor.entry_name.slice(
    descriptor.entry_name.lastIndexOf("/") + 1,
  );
  const destinationPath = await save({
    defaultPath: leaf,
    filters: [{ name: "QuickPLS validated Arrow evidence", extensions: ["arrow"] }],
  });
  if (typeof destinationPath !== "string" || !destinationPath) return null;
  if (!destinationPath.toLowerCase().endsWith(".arrow")) {
    throw new Error(
      "The selected raw-evidence destination must end in .arrow; no file was written.",
    );
  }
  const response = await invoke<unknown>(COMMAND, {
    request: {
      schemaVersion: 1,
      surface: "internal_labs_multimod_v1",
      experimentalLabsEnabled: true,
      archivePath: authority.archivePath,
      expectedArchiveSha256: authority.archiveSha256,
      projectId: authority.projectId,
      resultId: attachment.result_id,
      entryName: descriptor.entry_name,
      expectedIdentitySha256: attachment.identity_sha256,
      expectedPayloadSha256: descriptor.sha256,
      destinationPath,
    },
  });
  return parseReceipt(response, {
    authority,
    attachment,
    descriptor,
    destinationPath,
  });
}
