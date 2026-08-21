import { describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6ReadOnlySession } from "../internalProjectArchiveV6SessionStore";
import type { Dataset } from "../types";
import { readNativeDatasetPageV1 } from "./nativeDatasetRowPaging";

const PROJECT_ID = "00000000-0000-0000-0000-000000000601";
const DATASET_ID = "00000000-0000-0000-0000-000000000602";
const FINGERPRINT = "general-sem-resident-data";
const SHA256 = "a".repeat(64);

const dataset: Dataset = {
  id: DATASET_ID,
  name: "Descriptor data",
  columns: ["x", "y"],
  rows: [],
  rowCount: 2,
  missing: Number.NaN,
  fingerprint: FINGERPRINT,
  kind: "raw",
};

const session = {
  kind: "internal_schema6_read_only",
  access: "read_only",
  project: {
    schema_version: 6,
    project_id: PROJECT_ID,
    origin: { kind: "new_project" },
    sem_generation: "general_sem_v1",
    datasets: [{
      id: DATASET_ID,
      name: dataset.name,
      fingerprint: FINGERPRINT,
      schema: { version: 1, kind: "raw", columns: [{ name: "x" }, { name: "y" }], case_count: 2, sample_size: null },
    }],
  },
  snapshot: {
    archivePath: "D:\\projects\\general-sem.qpls",
    archiveSha256: SHA256,
    project: {
      schema_version: 6,
      project_id: PROJECT_ID,
      origin: { kind: "new_project" },
      sem_generation: "general_sem_v1",
      datasets: [{
        id: DATASET_ID,
        name: dataset.name,
        fingerprint: FINGERPRINT,
        schema: { version: 1, kind: "raw", columns: [{ name: "x" }, { name: "y" }], case_count: 2, sample_size: null },
      }],
    },
    generalSemExecutionAuthority: {
      projectId: PROJECT_ID,
      datasetId: DATASET_ID,
      datasetFingerprint: FINGERPRINT,
    },
  },
  standardActivation: { modelIds: ["model:general-sem"], sourceArchiveSha256: SHA256 },
} as unknown as InternalProjectArchiveV6ReadOnlySession;

describe("native Data surface row paging route", () => {
  it("uses only the strict archive reader for a descriptor-only bound dataset", async () => {
    const strictReader = vi.fn(async () => ({
      status: "ok" as const,
      value: {
        schemaVersion: 1 as const,
        archivePath: session.snapshot.archivePath,
        archiveSha256: SHA256,
        projectId: PROJECT_ID,
        datasetId: DATASET_ID,
        datasetFingerprint: FINGERPRINT,
        offset: 0,
        limit: 2,
        rowCount: 2,
        columns: ["x", "y"],
        rows: [{ x: "1", y: "2" }, { x: "3", y: "4" }],
        sourceRecheckedUnchanged: true as const,
      },
    }));
    const legacyReader = vi.fn();

    await expect(readNativeDatasetPageV1({
      dataset,
      datasetDescriptorOnly: true,
      session,
      offset: 0,
      limit: 2,
    }, strictReader, legacyReader)).resolves.toMatchObject({
      datasetId: DATASET_ID,
      rows: [{ x: "1", y: "2" }, { x: "3", y: "4" }],
    });
    expect(strictReader).toHaveBeenCalledWith(expect.objectContaining({
      archivePath: session.snapshot.archivePath,
      expectedArchiveSha256: SHA256,
      projectId: PROJECT_ID,
      datasetId: DATASET_ID,
      datasetFingerprint: FINGERPRINT,
    }));
    expect(legacyReader).not.toHaveBeenCalled();
  });

  it("fails closed without a bound authority and never falls back to legacy rows", async () => {
    const strictReader = vi.fn();
    const legacyReader = vi.fn();
    await expect(readNativeDatasetPageV1({
      dataset,
      datasetDescriptorOnly: true,
      session: { ...session, standardActivation: null },
      offset: 0,
      limit: 2,
    }, strictReader, legacyReader)).rejects.toThrow(/not bound to a strict General SEM archive/);
    expect(strictReader).not.toHaveBeenCalled();
    expect(legacyReader).not.toHaveBeenCalled();
  });

  it("retains the legacy reader only for ordinary resident projects", async () => {
    const strictReader = vi.fn();
    const legacyReader = vi.fn(async () => ({ datasetId: DATASET_ID, offset: 0, limit: 2, rowCount: 2, rows: [{ x: 1 }, { x: 2 }] }));
    await expect(readNativeDatasetPageV1({
      dataset: { ...dataset, rows: [{ x: 1 }, { x: 2 }] },
      datasetDescriptorOnly: false,
      session: null,
      offset: 0,
      limit: 2,
    }, strictReader, legacyReader)).resolves.toMatchObject({ rowCount: 2 });
    expect(strictReader).not.toHaveBeenCalled();
    expect(legacyReader).toHaveBeenCalledWith(DATASET_ID, 0, 2);
  });
});
