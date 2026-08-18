import { describe, expect, it } from "vitest";
import {
  parseInternalProjectArchiveV6SaveCopyOutcomeV1,
  parseInternalProjectArchiveV6SaveCopyRequestV1,
} from "./internalProjectArchiveV6SaveCopy";

const PROJECT_ID = "00000000-0000-0000-0000-000000000601";
const SOURCE_SHA256 = "a".repeat(64);
const DESTINATION_SHA256 = "b".repeat(64);
const PROJECT_SHA256 = "c".repeat(64);

function request() {
  return parseInternalProjectArchiveV6SaveCopyRequestV1({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    sourceArchivePath: "D:\\projects\\study-v6.qpls",
    expectedSourceArchiveSha256: SOURCE_SHA256,
    destinationArchivePath: "D:\\projects\\study-v6-model-copy.qpls",
    project: {
      schema_version: 6,
      project_id: PROJECT_ID,
      name: "Strict save-copy fixture",
      created_at: "2026-08-15T10:00:00Z",
      modified_at: "2026-08-15T10:01:00Z",
      origin: { kind: "new_project" },
    },
  });
}

function okOutcome() {
  const exactRequest = request();
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      persistence: "persisted_new_copy",
      receipt: {
        schemaVersion: 1,
        sourceArchivePath: exactRequest.sourceArchivePath,
        sourceArchiveSha256: SOURCE_SHA256,
        sourceVerifiedUnchanged: true,
        destinationArchivePath: exactRequest.destinationArchivePath,
        destinationArchiveSha256: DESTINATION_SHA256,
        destinationArchiveBytes: 2_048,
        strictReopenValidated: true,
        modelCount: 0,
      },
      snapshot: {
        schemaVersion: 1,
        access: "read_only",
        loader: "strict_schema6_zip",
        archivePath: exactRequest.destinationArchivePath,
        archiveSha256: DESTINATION_SHA256,
        archiveBytes: 2_048,
        manifest: {
          schema_version: 6,
          project_id: PROJECT_ID,
          name: exactRequest.project.name,
          created_at: exactRequest.project.created_at,
          modified_at: exactRequest.project.modified_at,
          engine_version: "quickpls-test",
          checksum_algorithm: "sha256",
          checksums: { "project.json": PROJECT_SHA256 },
        },
        project: exactRequest.project,
        residentDatasets: [],
        counts: {
          datasets: 0,
          models: 0,
          recipes: 0,
          historicalRecipes: 0,
          historicalResults: 0,
          canonicalResultDocuments: 0,
        },
        sourceRecheckedUnchanged: true,
      },
    },
  };
}

describe("Internal/Labs schema-6 Save copy contract", () => {
  it("accepts only a receipt-bound strict destination snapshot", () => {
    const parsed = parseInternalProjectArchiveV6SaveCopyOutcomeV1(okOutcome(), request());
    expect(parsed).toMatchObject({
      status: "ok",
      value: {
        persistence: "persisted_new_copy",
        receipt: {
          sourceVerifiedUnchanged: true,
          strictReopenValidated: true,
          destinationArchiveSha256: DESTINATION_SHA256,
        },
        snapshot: {
          archiveSha256: DESTINATION_SHA256,
          loader: "strict_schema6_zip",
        },
      },
    });
  });

  it("rejects receipt drift, false validation claims, and in-place requests", () => {
    const drifted = okOutcome();
    drifted.value.receipt.destinationArchiveSha256 = "d".repeat(64);
    expect(() => parseInternalProjectArchiveV6SaveCopyOutcomeV1(drifted, request()))
      .toThrowError(expect.objectContaining({ code: "schema6_save_copy.snapshot_receipt_mismatch" }));

    const unvalidated = okOutcome();
    unvalidated.value.receipt.strictReopenValidated = false;
    expect(() => parseInternalProjectArchiveV6SaveCopyOutcomeV1(unvalidated, request()))
      .toThrowError(expect.objectContaining({ code: "schema6_save_copy.receipt_invalid" }));

    expect(() => parseInternalProjectArchiveV6SaveCopyRequestV1({
      ...request(),
      destinationArchivePath: request().sourceArchivePath.toLocaleLowerCase(),
    })).toThrowError(expect.objectContaining({ code: "schema6_save_copy.new_destination_required" }));
    expect(() => parseInternalProjectArchiveV6SaveCopyRequestV1({
      ...request(),
      destinationArchivePath: ` ${request().destinationArchivePath}`,
    })).toThrowError(expect.objectContaining({ code: "schema6_save_copy.path_not_canonical" }));
  });
});
