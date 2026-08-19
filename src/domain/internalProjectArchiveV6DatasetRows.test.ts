import { describe, expect, it } from "vitest";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./internalProjectArchiveV6Read";
import {
  InternalProjectArchiveV6DatasetRowsWireError,
  buildInternalProjectArchiveV6DatasetRowsRequestV1,
  parseInternalProjectArchiveV6DatasetRowsOutcomeV1,
} from "./internalProjectArchiveV6DatasetRows";

const PROJECT_ID = "00000000-0000-0000-0000-000000000601";
const DATASET_ID = "00000000-0000-0000-0000-000000000602";
const ARCHIVE_SHA256 = "a".repeat(64);
const DATASET_FINGERPRINT = "schema6-general-sem-data";

const snapshot = {
  schemaVersion: 1,
  access: "read_only",
  loader: "strict_schema6_zip",
  archivePath: "D:\\projects\\general-sem.qpls",
  archiveSha256: ARCHIVE_SHA256,
  archiveBytes: 12_345,
  manifest: { project_id: PROJECT_ID },
  project: {
    schema_version: 6,
    project_id: PROJECT_ID,
    origin: { kind: "new_project" },
    sem_generation: "general_sem_v1",
    datasets: [{
      id: DATASET_ID,
      name: "Resident data",
      fingerprint: DATASET_FINGERPRINT,
      schema: {
        version: 1,
        kind: "raw",
        columns: ["x", "y"].map((name) => ({ name })),
        case_count: 3,
        sample_size: null,
      },
    }],
    models: [],
  },
  residentDatasets: [],
  counts: {},
  generalSemExecutionAuthority: {
    schemaVersion: 1,
    projectId: PROJECT_ID,
    datasetId: DATASET_ID,
    datasetFingerprint: DATASET_FINGERPRINT,
    modelId: "model:general-sem",
    modelScientificSha256: "b".repeat(64),
    recipeId: "00000000-0000-0000-0000-000000000603",
    recipeDocumentSha256: "c".repeat(64),
    recipe: {},
  },
  sourceRecheckedUnchanged: true,
} as unknown as InternalProjectArchiveV6ReadSnapshotV1;

function request() {
  return buildInternalProjectArchiveV6DatasetRowsRequestV1(snapshot, DATASET_ID, 1, 2);
}

function successWire() {
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      archivePath: snapshot.archivePath,
      archiveSha256: ARCHIVE_SHA256,
      projectId: PROJECT_ID,
      datasetId: DATASET_ID,
      datasetFingerprint: DATASET_FINGERPRINT,
      offset: 1,
      limit: 2,
      rowCount: 3,
      columns: ["x", "y"],
      rows: [{ x: "2", y: null }, { x: "3", y: "6" }],
      sourceRecheckedUnchanged: true,
    },
  };
}

describe("strict schema-6 General SEM dataset row paging wire", () => {
  it("builds the request only from exact native archive and execution identities", () => {
    expect(request()).toEqual({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      archivePath: snapshot.archivePath,
      expectedArchiveSha256: ARCHIVE_SHA256,
      projectId: PROJECT_ID,
      datasetId: DATASET_ID,
      datasetFingerprint: DATASET_FINGERPRINT,
      offset: 1,
      limit: 2,
    });

    const drifted = structuredClone(snapshot);
    drifted.generalSemExecutionAuthority!.datasetFingerprint = "different";
    expect(() => buildInternalProjectArchiveV6DatasetRowsRequestV1(drifted, DATASET_ID, 0, 1))
      .toThrowError(InternalProjectArchiveV6DatasetRowsWireError);
    expect(() => buildInternalProjectArchiveV6DatasetRowsRequestV1(snapshot, DATASET_ID, 0, 501))
      .toThrow(/limit from 1 through 500/);
  });

  it("accepts an exact complete page and rejects identity, shape, and row drift", () => {
    expect(parseInternalProjectArchiveV6DatasetRowsOutcomeV1(successWire(), request())).toMatchObject({
      status: "ok",
      value: { datasetId: DATASET_ID, offset: 1, rowCount: 3 },
    });

    for (const mutate of [
      (wire: ReturnType<typeof successWire>) => { wire.value.archiveSha256 = "f".repeat(64); },
      (wire: ReturnType<typeof successWire>) => { wire.value.datasetFingerprint = "different"; },
      (wire: ReturnType<typeof successWire>) => { wire.value.columns = ["x", "x"]; },
      (wire: ReturnType<typeof successWire>) => { delete (wire.value.rows[0] as Record<string, unknown>).x; },
      (wire: ReturnType<typeof successWire>) => { wire.value.rows.pop(); },
    ]) {
      const wire = successWire();
      mutate(wire);
      expect(() => parseInternalProjectArchiveV6DatasetRowsOutcomeV1(wire, request()))
        .toThrowError(InternalProjectArchiveV6DatasetRowsWireError);
    }
  });

  it("strictly parses a blocked diagnostic without weakening its wire", () => {
    expect(parseInternalProjectArchiveV6DatasetRowsOutcomeV1({
      status: "blocked",
      diagnostic: { code: "archive_changed", message: "Changed.", correctiveAction: "Reopen." },
    }, request())).toEqual({
      status: "blocked",
      diagnostic: { code: "archive_changed", message: "Changed.", correctiveAction: "Reopen." },
    });
    expect(() => parseInternalProjectArchiveV6DatasetRowsOutcomeV1({
      status: "blocked",
      diagnostic: { code: "archive_changed", message: "Changed.", corrective_action: "Reopen." },
    }, request())).toThrowError(InternalProjectArchiveV6DatasetRowsWireError);
  });
});
