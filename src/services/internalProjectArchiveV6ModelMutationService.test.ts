import { beforeEach, describe, expect, it, vi } from "vitest";
import { parseInternalProjectArchiveV6Wire } from "../domain/internalProjectArchiveV6Wire";
import {
  convertLegacyBasicModelV4,
  parseSemModelV4AuthoringDraft,
  type LegacyBasicModelV4Input,
} from "../domain/semModelV4";
import {
  appendResolvedInternalProjectArchiveV6ModelRevision,
  insertInternalProjectArchiveV6ModelDraft,
  promoteInternalProjectArchiveV6ModelDraft,
  replaceInternalProjectArchiveV6ModelDraft,
} from "./internalProjectArchiveV6ModelMutationService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

const PROJECT_ID = "00000000-0000-0000-0000-000000000101";
const SHA_A = "a".repeat(64);

function model(name = "Draft model") {
  const input: LegacyBasicModelV4Input = {
    id: "model:draft:service",
    name,
    constructs: [
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [{ source: "x", target: "y" }],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  };
  return convertLegacyBasicModelV4(input, "cbsem_common_factor");
}

function project() {
  return parseInternalProjectArchiveV6Wire({
    schema_version: 6,
    project_id: PROJECT_ID,
    name: "Service fixture",
    created_at: "2026-08-15T09:00:00Z",
    modified_at: "2026-08-15T09:01:00Z",
    origin: { kind: "new_project" },
  });
}

function draftRecord(draft = model()) {
  return {
    model_id: draft.id,
    payload: {
      kind: "sem_model_v4_draft" as const,
      model: draft,
      model_document_sha256: SHA_A,
    },
  };
}

const blocked = {
  status: "blocked",
  diagnostic: {
    code: "schema6_model_mutation.stale_model_digest",
    message: "The draft digest is stale.",
    correctiveAction: "Refresh and retry.",
  },
};

describe("Internal/Labs schema-6 model-mutation service", () => {
  beforeEach(() => mocks.invoke.mockReset());

  it("uses only the dedicated ephemeral mutation command for insert", async () => {
    const source = project();
    const draft = model();
    mocks.invoke.mockResolvedValue({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: { ...source, models: [draftRecord(draft)] },
      },
    });

    await expect(insertInternalProjectArchiveV6ModelDraft(source, draft))
      .resolves.toMatchObject({ status: "ok", value: { persistence: "not_persisted" } });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke.mock.calls[0][0]).toBe("mutate_internal_project_archive_v6_model");
    expect(mocks.invoke.mock.calls[0][1].request).toMatchObject({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      project: source,
    });
    expect(mocks.invoke.mock.calls[0][1].request.mutation).toEqual({
      kind: "insert_draft",
      draft: parseSemModelV4AuthoringDraft(draft),
    });
    expect(mocks.invoke).not.toHaveBeenCalledWith("save_project", expect.anything());
    expect(mocks.invoke).not.toHaveBeenCalledWith("open_project", expect.anything());
  });

  it("sends replace and promote CAS requests without a path or persistence option", async () => {
    const sourceDraft = model();
    const source = { ...project(), models: [draftRecord(sourceDraft)] };
    const replacement = model("Edited draft");
    mocks.invoke.mockResolvedValueOnce(blocked).mockResolvedValueOnce(blocked);

    await expect(replaceInternalProjectArchiveV6ModelDraft(
      source,
      sourceDraft.id,
      SHA_A,
      replacement,
    )).resolves.toEqual(blocked);
    await expect(promoteInternalProjectArchiveV6ModelDraft(
      source,
      sourceDraft.id,
      SHA_A,
    )).resolves.toEqual(blocked);

    expect(mocks.invoke.mock.calls[0][1].request.mutation).toEqual({
      kind: "replace_draft",
      modelId: sourceDraft.id,
      expectedModelDocumentSha256: SHA_A,
      replacement: parseSemModelV4AuthoringDraft(replacement),
    });
    expect(mocks.invoke.mock.calls[1][1].request.mutation).toEqual({
      kind: "promote_draft",
      modelId: sourceDraft.id,
      expectedModelDocumentSha256: SHA_A,
    });
    for (const call of mocks.invoke.mock.calls) {
      expect(call[1].request).not.toHaveProperty("archivePath");
      expect(call[1].request).not.toHaveProperty("save");
    }
  });

  it("rejects malformed native success payloads at the service boundary", async () => {
    const source = project();
    const draft = model();
    mocks.invoke.mockResolvedValue({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "persisted",
        project: { ...source, models: [draftRecord(draft)] },
      },
    });

    await expect(insertInternalProjectArchiveV6ModelDraft(source, draft))
      .rejects.toMatchObject({ code: "schema6_model_mutation.persistence_forbidden" });
  });

  it("fails strict malformed input before invoking native code", async () => {
    const malformed = { ...project(), schema_version: 7 };

    await expect(insertInternalProjectArchiveV6ModelDraft(
      malformed as ReturnType<typeof project>,
      model(),
    )).rejects.toMatchObject({ code: "project_archive_v6.future_read_only" });
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("appends and promotes only when both native resolver digests match", async () => {
    const source = project();
    const revision = model("Revision 2");
    const inserted = { ...source, models: [draftRecord(revision)] };
    const promoted = {
      ...source,
      models: [{
        model_id: revision.id,
        payload: { kind: "sem_model_v4", model: revision, scientific_sha256: "b".repeat(64) },
      }],
    };
    mocks.invoke
      .mockResolvedValueOnce({ status: "ok", value: { schemaVersion: 1, persistence: "not_persisted", project: inserted } })
      .mockResolvedValueOnce({ status: "ok", value: { schemaVersion: 1, persistence: "not_persisted", project: promoted } });

    await expect(appendResolvedInternalProjectArchiveV6ModelRevision(source, {
      schemaVersion: 1,
      canonicalModel: revision,
      modelDocumentSha256: SHA_A,
      scientificSha256: "b".repeat(64),
      readiness: "ready",
      authoringIssues: [],
      readinessIssues: [],
    })).resolves.toMatchObject({
      models: [{ model_id: revision.id, payload: { kind: "sem_model_v4", scientific_sha256: "b".repeat(64) } }],
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(2);
    expect(mocks.invoke.mock.calls[1][1].request.mutation).toEqual({
      kind: "promote_draft",
      modelId: revision.id,
      expectedModelDocumentSha256: SHA_A,
    });
  });
});
