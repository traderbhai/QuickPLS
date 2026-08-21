import { describe, expect, it } from "vitest";
import {
  parseInternalProjectArchiveV6ReadOutcomeV1,
} from "./internalProjectArchiveV6Read";

const PROJECT_ID = "00000000-0000-0000-0000-000000000601";
const ARCHIVE_SHA256 = "a".repeat(64);
const PROJECT_SHA256 = "b".repeat(64);

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

function okOutcome() {
  const project = {
    schema_version: 6,
    project_id: PROJECT_ID,
    name: "Strict ZIP fixture",
    created_at: "2026-08-15T10:00:00Z",
    modified_at: "2026-08-15T10:01:00Z",
    origin: { kind: "new_project" },
  };
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      access: "read_only",
      loader: "strict_schema6_zip",
      archivePath: "D:\\projects\\strict-v6.qpls",
      archiveSha256: ARCHIVE_SHA256,
      archiveBytes: 1_024,
      manifest: {
        schema_version: 6,
        project_id: PROJECT_ID,
        name: project.name,
        created_at: project.created_at,
        modified_at: project.modified_at,
        engine_version: "quickpls-test",
        checksum_algorithm: "sha256",
        checksums: { "project.json": PROJECT_SHA256 },
      },
      project,
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
  };
}

describe("Internal/Labs strict schema-6 ZIP read contract", () => {
  it("returns a typed read-only snapshot from the dedicated ZIP loader", () => {
    const parsed = parseInternalProjectArchiveV6ReadOutcomeV1(okOutcome());
    expect(parsed).toMatchObject({
      status: "ok",
      value: {
        access: "read_only",
        loader: "strict_schema6_zip",
        archiveSha256: ARCHIVE_SHA256,
        project: { schema_version: 6, project_id: PROJECT_ID },
        sourceRecheckedUnchanged: true,
      },
    });
    if (parsed.status !== "ok") throw new Error("Expected an inspectable schema-6 ZIP.");
    expect(parsed.value.project.datasets).toEqual([]);
    expect(parsed.value.residentDatasets).toEqual([]);
  });

  it("rejects any non-read-only or non-strict-loader snapshot", () => {
    const editable = clone(okOutcome());
    editable.value.access = "current_editable";
    expect(() => parseInternalProjectArchiveV6ReadOutcomeV1(editable))
      .toThrowError(expect.objectContaining({ code: "schema6_archive_read.read_only_required" }));

    const legacy = clone(okOutcome());
    legacy.value.loader = "legacy_schema5";
    expect(() => parseInternalProjectArchiveV6ReadOutcomeV1(legacy))
      .toThrowError(expect.objectContaining({ code: "schema6_archive_read.strict_loader_required" }));
  });

  it("cross-checks manifest identity and all project counts", () => {
    const manifestMismatch = clone(okOutcome());
    manifestMismatch.value.manifest.name = "Different project";
    expect(() => parseInternalProjectArchiveV6ReadOutcomeV1(manifestMismatch))
      .toThrowError(expect.objectContaining({ code: "schema6_archive_read.manifest_project_mismatch" }));

    const countMismatch = clone(okOutcome());
    countMismatch.value.counts.models = 1;
    expect(() => parseInternalProjectArchiveV6ReadOutcomeV1(countMismatch))
      .toThrowError(expect.objectContaining({ code: "schema6_archive_read.count_mismatch" }));
  });

  it("preserves typed blocked diagnostics without interpreting an archive", () => {
    expect(parseInternalProjectArchiveV6ReadOutcomeV1({
      status: "blocked",
      diagnostic: {
        code: "schema6_archive_read.invalid_archive",
        message: "Checksum mismatch.",
        correctiveAction: "Restore a trusted archive.",
      },
    })).toEqual({
      status: "blocked",
      diagnostic: {
        code: "schema6_archive_read.invalid_archive",
        message: "Checksum mismatch.",
        correctiveAction: "Restore a trusted archive.",
      },
    });
  });

  it("allows an exact blank marked project but requires native authority once populated", () => {
    const blank = okOutcome();
    const blankProject = blank.value.project as typeof blank.value.project & {
      sem_generation?: string;
      datasets?: unknown[];
    };
    const blankValue = blank.value as typeof blank.value & {
      generalSemExecutionAuthority?: unknown;
      residentDatasets: unknown[];
    };
    blankProject.sem_generation = "general_sem_v1";
    blankValue.generalSemExecutionAuthority = null;
    expect(parseInternalProjectArchiveV6ReadOutcomeV1(blank)).toMatchObject({
      status: "ok",
      value: { project: { sem_generation: "general_sem_v1" }, generalSemExecutionAuthority: null },
    });

    const populated = clone(blank);
    const populatedProject = populated.value.project as typeof populated.value.project & { datasets: unknown[] };
    const populatedValue = populated.value as unknown as { residentDatasets: unknown[] };
    populatedProject.datasets = [{
      id: "00000000-0000-4000-8000-000000000610",
      name: "Resident data",
      schema: { version: 1, kind: "raw", columns: [], case_count: 0, sample_size: null },
      fingerprint: "dataset-fingerprint",
    }];
    populatedValue.residentDatasets = [{
      datasetId: "00000000-0000-4000-8000-000000000610",
      name: "Resident data",
      fingerprint: "dataset-fingerprint",
      rowCount: 0,
      columnCount: 0,
      sampleSize: null,
      arrowResident: true,
    }];
    populated.value.counts.datasets = 1;
    expect(() => parseInternalProjectArchiveV6ReadOutcomeV1(populated))
      .toThrowError(expect.objectContaining({ code: "schema6_archive_read.general_sem_authority_missing" }));
  });
});
