import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import type {
  InternalProjectArchiveV6Wire,
  ProjectModelRecordV6Wire,
} from "../domain/internalProjectArchiveV6Wire";
import {
  convertLegacyBasicModelV4,
  type LegacyBasicModelV4Input,
} from "../domain/semModelV4";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES,
  type InternalProjectArchiveV6ReadOnlySession,
} from "../internalProjectArchiveV6SessionStore";
import {
  buildInternalProjectArchiveV6JsonMutation,
  buildInternalProjectArchiveV6Promotion,
  InternalProjectArchiveV6ModelAuthorityEditorView,
  parseInternalProjectArchiveV6SemModelJson,
} from "./InternalProjectArchiveV6ModelAuthorityEditor";

const DRAFT_SHA = "a".repeat(64);
const READY_SHA = "b".repeat(64);

function model(id: string, name: string) {
  const input: LegacyBasicModelV4Input = {
    id,
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

const draftModel = model("model:draft:1", "Draft authority");
const readyModel = model("model:ready:1", "Ready authority");
const draftRecord = {
  model_id: draftModel.id,
  payload: {
    kind: "sem_model_v4_draft",
    model: draftModel,
    model_document_sha256: DRAFT_SHA,
  },
} satisfies ProjectModelRecordV6Wire;
const readyRecord = {
  model_id: readyModel.id,
  payload: {
    kind: "sem_model_v4",
    model: readyModel,
    scientific_sha256: READY_SHA,
  },
} satisfies ProjectModelRecordV6Wire;

const project = {
  models: [draftRecord, readyRecord],
} as unknown as InternalProjectArchiveV6Wire;
const snapshot = {
  project,
  archivePath: "D:\\projects\\study-v6.qpls",
} as unknown as InternalProjectArchiveV6ReadSnapshotV1;
const session: InternalProjectArchiveV6ReadOnlySession = {
  kind: "internal_schema6_read_only",
  access: "read_only",
  snapshot,
  project,
  capabilities: INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES,
};

const noop = () => undefined;

function view({
  dirty = false,
  persistence = null,
  failure = null,
  mode = "insert",
  json = "",
}: {
  dirty?: boolean;
  persistence?: "not_persisted" | null;
  failure?: { code: string; message: string; correctiveAction?: string } | null;
  mode?: "insert" | "replace" | "operations";
  json?: string;
} = {}) {
  return renderToStaticMarkup(<InternalProjectArchiveV6ModelAuthorityEditorView
    session={session}
    mode={mode}
    json={json}
    selectedDraftId={draftRecord.model_id}
    pending={false}
    dirty={dirty}
    persistence={persistence}
    statusMessage="No ephemeral model change has been applied."
    failure={failure}
    onModeChange={noop}
    onJsonChange={noop}
    onSelectedDraftChange={noop}
    onApplyJson={noop}
    onPromote={noop}
  />);
}

describe("Internal/Labs schema-6 ephemeral model-authority editor", () => {
  it("uses the existing strict SemModelV4 draft decoder after JSON parsing", () => {
    const parsed = parseInternalProjectArchiveV6SemModelJson(JSON.stringify(draftModel));
    expect(parsed.id).toBe(draftModel.id);
    expect(parsed.name).toBe(draftModel.name);
    expect(parsed.schema_version).toBe(4);

    expect(() => parseInternalProjectArchiveV6SemModelJson("{not-json"))
      .toThrowError(expect.objectContaining({ code: "schema6_model_editor.json_invalid" }));
    expect(() => parseInternalProjectArchiveV6SemModelJson(JSON.stringify({
      ...draftModel,
      invented_field: true,
    }))).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));
    expect(() => parseInternalProjectArchiveV6SemModelJson(JSON.stringify({
      ...draftModel,
      variables: undefined,
    }))).toThrowError(expect.objectContaining({ code: "schema.invalid_shape" }));
  });

  it("builds insert and replace requests using exact decoded JSON and current draft CAS", () => {
    const json = JSON.stringify(draftModel);
    const parsedDraft = parseInternalProjectArchiveV6SemModelJson(json);
    expect(buildInternalProjectArchiveV6JsonMutation("insert", json)).toEqual({
      kind: "insert_draft",
      draft: parsedDraft,
    });
    expect(buildInternalProjectArchiveV6JsonMutation("replace", json, draftRecord)).toEqual({
      kind: "replace_draft",
      modelId: draftRecord.model_id,
      expectedModelDocumentSha256: DRAFT_SHA,
      replacement: parsedDraft,
    });

    const otherId = { ...draftModel, id: "model:draft:other" };
    expect(() => buildInternalProjectArchiveV6JsonMutation(
      "replace",
      JSON.stringify(otherId),
      draftRecord,
    )).toThrowError(expect.objectContaining({ code: "schema6_model_editor.model_id_mismatch" }));
  });

  it("applies a strict canonical operation batch to the selected draft with its current CAS", () => {
    const operations = JSON.stringify({
      schema_version: 1,
      expected_model_id: draftModel.id,
      operations: [{
        kind: "set_group",
        group: {
          kind: "observed_groups",
          grouping_variable: "observed:x1",
          levels: [
            { id: "a", value: "A", label: "Group A" },
            { id: "b", value: "B", label: "Group B" },
          ],
        },
      }],
    });
    const mutation = buildInternalProjectArchiveV6JsonMutation(
      "operations",
      operations,
      draftRecord,
    );

    expect(mutation).toMatchObject({
      kind: "replace_draft",
      modelId: draftRecord.model_id,
      expectedModelDocumentSha256: DRAFT_SHA,
      replacement: {
        id: draftRecord.model_id,
        group: { kind: "observed_groups", grouping_variable: "observed:x1" },
      },
    });
    expect(() => buildInternalProjectArchiveV6JsonMutation("operations", operations))
      .toThrowError(expect.objectContaining({ code: "schema6_model_editor.draft_required" }));
  });

  it("promotes only a selected draft and carries its exact CAS digest", () => {
    expect(buildInternalProjectArchiveV6Promotion(draftRecord)).toEqual({
      kind: "promote_draft",
      modelId: draftRecord.model_id,
      expectedModelDocumentSha256: DRAFT_SHA,
    });
    expect(() => buildInternalProjectArchiveV6Promotion())
      .toThrowError(expect.objectContaining({ code: "schema6_model_editor.draft_required" }));
  });

  it("lists ready and draft authority with their distinct exact digests", () => {
    const html = view();

    expect(html).toContain('data-schema6-model-authority-editor="ephemeral"');
    expect(html).toContain('aria-label="Schema-6 model authority records"');
    expect(html).toContain("Ready and draft model records in the ephemeral session document");
    expect(html).toContain("Draft authority");
    expect(html).toContain("Ready authority");
    expect(html).toContain(">Draft<");
    expect(html).toContain(">Ready<");
    expect(html).toContain(DRAFT_SHA);
    expect(html).toContain(READY_SHA);
    expect(html).toContain('for="internal-schema6-model-json"');
    expect(html).toContain("strict SemModelV4 decoder");
  });

  it("marks successful edits dirty and explicitly not persisted", () => {
    const html = view({ dirty: true, persistence: "not_persisted" });

    expect(html).toContain("Unsaved ephemeral changes");
    expect(html).toContain("dirty and not_persisted");
    expect(html).toContain("Save validated new copy action");
    expect(html).not.toContain("Save project");
  });

  it("describes canonical operations and their fail-closed exclusions without implying persistence", () => {
    const html = view({ mode: "operations", json: "{}" });

    expect(html).toContain("Exact canonical authority-operation batch JSON");
    expect(html).toContain("Apply operation batch");
    expect(html).toContain("applied atomically to the selected canonical SemModelV4 authority");
    expect(html).toContain("delete_or_reorder");
    expect(html).toContain("change_model_identity");
    expect(html).toContain("edit_annotations_or_presentation");
    expect(html).toContain("Persistence · source_snapshot");
  });

  it("renders decoder or native failures as accessible diagnostics", () => {
    const html = view({
      failure: {
        code: "schema6_model_mutation.stale_model_digest",
        message: "The draft digest is stale.",
        correctiveAction: "Refresh and retry.",
      },
    });

    expect(html).toContain('role="alert"');
    expect(html).toContain("The draft digest is stale.");
    expect(html).toContain("Refresh and retry.");
    expect(html).toContain("schema6_model_mutation.stale_model_digest");
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
  });
});
