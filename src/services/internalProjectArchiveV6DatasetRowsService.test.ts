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

describe("strict General SEM dataset rows service", () => {
  beforeEach(() => mocks.invoke.mockReset());

  it("passes one exact request object to the dedicated native command", async () => {
    mocks.invoke.mockResolvedValue({
      status: "ok",
      value: {
        schemaVersion: 1,
        archivePath: request.archivePath,
        archiveSha256: request.expectedArchiveSha256,
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
    });

    await expect(readInternalProjectArchiveV6DatasetRows(request)).resolves.toMatchObject({ status: "ok" });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith("read_internal_project_archive_v6_dataset_rows", { request });
  });

  it("rejects a native response whose digest is not the requested digest", async () => {
    mocks.invoke.mockResolvedValue({
      status: "ok",
      value: {
        schemaVersion: 1,
        archivePath: request.archivePath,
        archiveSha256: "f".repeat(64),
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
    });
    await expect(readInternalProjectArchiveV6DatasetRows(request)).rejects.toThrow(/differs from the requested archive/);
  });
});
