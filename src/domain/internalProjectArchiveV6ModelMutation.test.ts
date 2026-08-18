import { describe, expect, it } from "vitest";
import {
  parseInternalProjectArchiveV6ModelMutationOutcomeV1,
  parseInternalProjectArchiveV6ModelMutationRequestV1,
} from "./internalProjectArchiveV6ModelMutation";
import { parseInternalProjectArchiveV6Wire } from "./internalProjectArchiveV6Wire";
import {
  convertLegacyBasicModelV4,
  type LegacyBasicModelV4Input,
  type SemModelV4,
} from "./semModelV4";

const PROJECT_ID = "00000000-0000-0000-0000-000000000101";
const SHA_A = "a".repeat(64);
const SHA_B = "b".repeat(64);

function draft(name = "Draft model"): SemModelV4 {
  const input: LegacyBasicModelV4Input = {
    id: "model:draft:1",
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

function emptyProject() {
  return parseInternalProjectArchiveV6Wire({
    schema_version: 6,
    project_id: PROJECT_ID,
    name: "Ephemeral project",
    created_at: "2026-08-15T09:00:00Z",
    modified_at: "2026-08-15T09:01:00Z",
    layouts: { model_editor: { zoom: 1.25 } },
    origin: { kind: "new_project" },
  });
}

function insertRequest() {
  return parseInternalProjectArchiveV6ModelMutationRequestV1({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    project: emptyProject(),
    mutation: { kind: "insert_draft", draft: draft() },
  });
}

function draftRecord(model = draft()) {
  return {
    model_id: model.id,
    payload: {
      kind: "sem_model_v4_draft" as const,
      model,
      model_document_sha256: SHA_A,
    },
  };
}

describe("Internal/Labs schema-6 model-mutation wire", () => {
  it("strictly parses all three mutation requests and rejects non-Labs or stale-shaped input", () => {
    expect(insertRequest().mutation.kind).toBe("insert_draft");

    const source = { ...emptyProject(), models: [draftRecord()] };
    const replacement = draft("Edited draft");
    expect(parseInternalProjectArchiveV6ModelMutationRequestV1({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      project: source,
      mutation: {
        kind: "replace_draft",
        modelId: replacement.id,
        expectedModelDocumentSha256: SHA_A,
        replacement,
      },
    }).mutation.kind).toBe("replace_draft");
    expect(parseInternalProjectArchiveV6ModelMutationRequestV1({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      project: source,
      mutation: {
        kind: "promote_draft",
        modelId: replacement.id,
        expectedModelDocumentSha256: SHA_A,
      },
    }).mutation.kind).toBe("promote_draft");

    expect(() => parseInternalProjectArchiveV6ModelMutationRequestV1({
      surface: "standard",
      experimentalLabsEnabled: true,
      project: emptyProject(),
      mutation: { kind: "insert_draft", draft: draft() },
    })).toThrowError(expect.objectContaining({ code: "schema6_model_mutation.internal_labs_required" }));
    expect(() => parseInternalProjectArchiveV6ModelMutationRequestV1({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      project: source,
      mutation: {
        kind: "promote_draft",
        modelId: draft().id,
        expectedModelDocumentSha256: "not-a-digest",
      },
    })).toThrowError(expect.objectContaining({ code: "schema6_model_mutation.sha256_invalid" }));
  });

  it("accepts an exact ephemeral insert and returns a strict schema-6 document", () => {
    const request = insertRequest();
    const inserted = draftRecord(request.mutation.kind === "insert_draft" ? request.mutation.draft : draft());
    const outcome = parseInternalProjectArchiveV6ModelMutationOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: { ...request.project, models: [inserted] },
      },
    }, request);

    expect(outcome.status).toBe("ok");
    if (outcome.status !== "ok") throw new Error("Expected a successful mutation fixture.");
    expect(outcome.value.persistence).toBe("not_persisted");
    expect(outcome.value.project.models[0]).toEqual(inserted);
  });

  it("rejects native success payloads that change non-model fields or the wrong model", () => {
    const request = insertRequest();
    const inserted = draftRecord(request.mutation.kind === "insert_draft" ? request.mutation.draft : draft());
    expect(() => parseInternalProjectArchiveV6ModelMutationOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: { ...request.project, name: "Changed behind the bridge", models: [inserted] },
      },
    }, request)).toThrowError(expect.objectContaining({ code: "schema6_model_mutation.non_model_change" }));

    expect(() => parseInternalProjectArchiveV6ModelMutationOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: request.project,
      },
    }, request)).toThrowError(expect.objectContaining({ code: "schema6_model_mutation.insert_shape" }));
  });

  it("verifies exact replace and promote transitions without permitting other model changes", () => {
    const sourceDraft = draft();
    const source = { ...emptyProject(), models: [draftRecord(sourceDraft)] };
    const replacement = draft("Edited draft");
    const replaceRequest = parseInternalProjectArchiveV6ModelMutationRequestV1({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      project: source,
      mutation: {
        kind: "replace_draft",
        modelId: sourceDraft.id,
        expectedModelDocumentSha256: SHA_A,
        replacement,
      },
    });
    const replaced = parseInternalProjectArchiveV6ModelMutationOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: { ...replaceRequest.project, models: [draftRecord(replacement)] },
      },
    }, replaceRequest);
    expect(replaced.status).toBe("ok");

    const promoteRequest = parseInternalProjectArchiveV6ModelMutationRequestV1({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
      project: source,
      mutation: {
        kind: "promote_draft",
        modelId: sourceDraft.id,
        expectedModelDocumentSha256: SHA_A,
      },
    });
    const promoted = parseInternalProjectArchiveV6ModelMutationOutcomeV1({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: {
          ...promoteRequest.project,
          models: [{
            model_id: sourceDraft.id,
            payload: {
              kind: "sem_model_v4",
              model: sourceDraft,
              scientific_sha256: SHA_B,
            },
          }],
        },
      },
    }, promoteRequest);
    expect(promoted.status).toBe("ok");
  });

  it("keeps typed native diagnostics strict", () => {
    const request = insertRequest();
    expect(parseInternalProjectArchiveV6ModelMutationOutcomeV1({
      status: "blocked",
      diagnostic: {
        code: "schema6_model_mutation.model_id_unavailable",
        message: "Duplicate model id.",
        correctiveAction: "Choose a new revision id.",
      },
    }, request)).toEqual({
      status: "blocked",
      diagnostic: {
        code: "schema6_model_mutation.model_id_unavailable",
        message: "Duplicate model id.",
        correctiveAction: "Choose a new revision id.",
      },
    });
  });
});
