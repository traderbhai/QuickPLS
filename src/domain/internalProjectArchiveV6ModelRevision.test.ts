import { describe, expect, it } from "vitest";
import type { InternalProjectArchiveV6Wire } from "./internalProjectArchiveV6Wire";
import { buildInternalProjectArchiveV6ModelRevisionV1 } from "./internalProjectArchiveV6ModelRevision";
import type { SemModelV4 } from "./semModelV4";

const model = { schema_version: 4, id: "model:old", name: "Original" } as unknown as SemModelV4;
const project = {
  models: [{ model_id: "model:old", payload: { kind: "sem_model_v4", model, scientific_sha256: "a".repeat(64) } }],
  recipes: [{ model_binding: { kind: "project_sem_model_v4_reference", model_id: "model:old", scientific_sha256: "a".repeat(64) } }],
  canonical_result_documents: [],
} as unknown as InternalProjectArchiveV6Wire;

describe("schema-6 model revision builder", () => {
  it("forks only identity and name while leaving every source artifact untouched", () => {
    const before = JSON.stringify(project);
    const built = buildInternalProjectArchiveV6ModelRevisionV1(project, "model:old", "model:revision:new", "Revision 2");
    expect(built.revision).toEqual({ ...model, id: "model:revision:new", name: "Revision 2" });
    expect(JSON.stringify(project)).toBe(before);
    expect(built.source).toBe(project.models[0]);
  });

  it("rejects reuse, unready sources, and nonmatching recipe bindings", () => {
    expect(() => buildInternalProjectArchiveV6ModelRevisionV1(project, "model:old", "model:old", "Revision"))
      .toThrowError(expect.objectContaining({ code: "schema6_model_revision.new_identity_required" }));
    expect(() => buildInternalProjectArchiveV6ModelRevisionV1(
      { ...project, recipes: [] }, "model:old", "model:new", "Revision",
    )).toThrowError(expect.objectContaining({ code: "schema6_model_revision.recipe_binding_required" }));
    expect(() => buildInternalProjectArchiveV6ModelRevisionV1(
      { ...project, models: [{ model_id: "model:old", payload: { kind: "sem_model_v4_draft", model, model_document_sha256: "b".repeat(64) } }] },
      "model:old", "model:new", "Revision",
    )).toThrowError(expect.objectContaining({ code: "schema6_model_revision.ready_source_required" }));
  });
});
