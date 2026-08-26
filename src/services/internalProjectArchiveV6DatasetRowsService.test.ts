import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6DatasetRowsRequestV1 } from "../domain/internalProjectArchiveV6DatasetRows";
import { readInternalProjectArchiveV6DatasetRows } from "./internalProjectArchiveV6DatasetRowsService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const request: InternalProjectArchiveV6DatasetRowsRequestV1 = {
  surface: "internal_labs",
  experimentalLabsEnabled: true,
  archivePath: "D:\\projects\\general-sem.qpls",
  expectedArchiveSha256: "a".repeat(64),
  projectId: "00000000-0000-0000-0000-000000000601",
  datasetId: "00000000-0000-0000-0000-000000000602",
  datasetFingerprint: "resident-data",
  offset: 0,
  limit: 1,
};

function candidateAuthority(qualified: boolean) {
  return {
    schemaVersion: 1,
    state: qualified ? "release_qualified_candidate" : "labs_only",
    standardSurfaceAuthorized: qualified,
    embeddedDocumentSha256: "a".repeat(64),
    authorityBindingSha256: qualified ? "b".repeat(64) : null,
    candidateCommitSha: qualified ? "c".repeat(40) : null,
    candidateVersion: qualified ? "2.56.0" : null,
    qualificationPlanSha256: qualified ? "d".repeat(64) : null,
    gateBindingSha256: qualified ? "e".repeat(64) : null,
    capabilityIndexSha256: qualified ? "f".repeat(64) : null,
    prepackageManifestSetSha256: qualified ? "1".repeat(64) : null,
    exactProfileCells: qualified
      ? ["mga.general_sem_pls.v1::point_estimation"]
      : [],
  };
}

const labsDependencies = {
  candidateAuthority: async () => candidateAuthority(false),
};
const standardDependencies = {
  candidateAuthority: async () => candidateAuthority(true),
};

function successResponse(archiveSha256 = request.expectedArchiveSha256) {
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      archivePath: request.archivePath,
      archiveSha256,
      projectId: request.projectId,
      datasetId: request.datasetId,
      datasetFingerprint: request.datasetFingerprint,
      offset: 0,
      limit: 1,
      rowCount: 1,
      columns: ["x"],
      rows: [{ x: "1" }],
      sourceRecheckedUnchanged: true,
    },
  };
}

describe("strict General SEM dataset rows service", () => {
  beforeEach(() => mocks.invoke.mockReset());

  it("passes one exact request object to the dedicated native command", async () => {
    mocks.invoke.mockResolvedValue(successResponse());

    await expect(
      readInternalProjectArchiveV6DatasetRows(request, labsDependencies),
    ).resolves.toMatchObject({ status: "ok" });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith("read_internal_project_archive_v6_dataset_rows", { request });
  });

  it("upgrades a valid historical request to Standard(false) under qualified authority", async () => {
    mocks.invoke.mockResolvedValue(successResponse());

    await expect(
      readInternalProjectArchiveV6DatasetRows(request, standardDependencies),
    ).resolves.toMatchObject({ status: "ok" });
    expect(mocks.invoke).toHaveBeenCalledWith(
      "read_internal_project_archive_v6_dataset_rows",
      {
        request: {
          ...request,
          surface: "standard_multimod_v1",
          experimentalLabsEnabled: false,
        },
      },
    );
  });

  it("rejects a native response whose digest is not the requested digest", async () => {
    mocks.invoke.mockResolvedValue(successResponse("f".repeat(64)));
    await expect(
      readInternalProjectArchiveV6DatasetRows(request, labsDependencies),
    ).rejects.toThrow(/differs from the requested archive/);
  });
});
