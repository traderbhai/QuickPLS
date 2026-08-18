import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { convertLegacyBasicModelV4, type LegacyBasicModelV4Input } from "../domain/semModelV4";
import type {
  StandardSemModelV4AuthorityRecordV1,
  StandardSemModelV4EditorIntentV1,
} from "../domain/standardSemModelV4Authority";
import {
  commitStandardSemModelV4AdvancedDocument,
  parseStandardSemModelV4AdvancedDocument,
  StandardSemModelV4AdvancedEditor,
  standardSemModelV4AdvancedFeedbackFor,
} from "./StandardSemModelV4AdvancedEditor";

const legacy: LegacyBasicModelV4Input = {
  id: "advanced-standard-model",
  name: "Advanced Standard model",
  constructs: [
    { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};

const model = convertLegacyBasicModelV4(legacy, "cbsem_common_factor");
const authority: StandardSemModelV4AuthorityRecordV1 = {
  schema_version: 1,
  model_document_sha256: "a".repeat(64),
  model,
};

describe("Standard SemModelV4 advanced editor", () => {
  it("renders an accessible Expert fallback with explicit readiness and save-copy guidance", () => {
    const html = renderToStaticMarkup(<StandardSemModelV4AdvancedEditor
      authority={authority}
      commit={async () => ({ status: "stale" })}
    />);

    expect(html).toContain("Advanced canonical document");
    expect(html).toContain("Complete SemModelV4 JSON");
    expect(html).toContain('id="nd-standard-advanced-document"');
    expect(html).toContain('aria-describedby="nd-standard-advanced-help nd-standard-advanced-save-help"');
    expect(html).toContain("data_binding.dataset_id must remain unchanged");
    expect(html).toContain("Keep annotations and presentation unchanged");
    expect(html).toContain("use the canvas presentation layer");
    expect(html).toContain("one native authority commit decides the result");
    expect(html).toContain("resolve its readiness issues before calculation");
    expect(html).toContain("Save validated new copy…");
    expect(html).toContain("This editor does not overwrite the source project.");
  });

  it("strictly parses the full document and requires the active stable model ID", () => {
    const parsed = parseStandardSemModelV4AdvancedDocument(JSON.stringify(model), authority);
    expect(parsed).toEqual(expect.objectContaining({ id: model.id, name: model.name }));
    expect(parsed.variables).toEqual(model.variables);

    expect(() => parseStandardSemModelV4AdvancedDocument("{", authority)).toThrow("Invalid JSON");
    expect(() => parseStandardSemModelV4AdvancedDocument(
      JSON.stringify({ ...model, unknown_field: true }),
      authority,
    )).toThrow();
    expect(() => parseStandardSemModelV4AdvancedDocument(
      JSON.stringify({ ...model, id: "different-model" }),
      authority,
    )).toThrow("does not match the active model ID");
  });

  it("dispatches exactly one complete replacement intent and never mutates the parsed document optimistically", async () => {
    const edited = { ...model, name: "Edited through complete JSON" };
    const commit = vi.fn(async (_intent: StandardSemModelV4EditorIntentV1) => ({ status: "stale" as const }));

    const result = await commitStandardSemModelV4AdvancedDocument(JSON.stringify(edited), authority, commit);

    expect(result).toEqual({ status: "stale" });
    expect(commit).toHaveBeenCalledTimes(1);
    const intent = commit.mock.calls[0]?.[0];
    expect(intent).toEqual(expect.objectContaining({
      kind: "replace_complete_model",
      model: expect.objectContaining({ id: model.id, name: "Edited through complete JSON" }),
    }));
    expect(model.name).toBe("Advanced Standard model");
  });

  it("blocks dataset and presentation-owned changes before dispatching a native commit", async () => {
    const commit = vi.fn(async (_intent: StandardSemModelV4EditorIntentV1) => ({ status: "stale" as const }));

    await expect(commitStandardSemModelV4AdvancedDocument(JSON.stringify({
      ...model,
      data_binding: { ...model.data_binding, dataset_id: "dataset:other" },
    }), authority, commit)).rejects.toMatchObject({
      code: "standard_sem_authority.dataset_binding_switch_requires_descriptor_transaction",
      corrective_action: expect.stringContaining("descriptor-aware project transaction"),
    });

    await expect(commitStandardSemModelV4AdvancedDocument(JSON.stringify({
      ...model,
      annotations: [{ kind: "caption", id: "caption:expert", text: "Not owned here" }],
    }), authority, commit)).rejects.toMatchObject({
      code: "standard_sem_authority.presentation_annotations_owned_by_layout",
      corrective_action: expect.stringContaining("canvas presentation controls"),
    });

    await expect(commitStandardSemModelV4AdvancedDocument(JSON.stringify({
      ...model,
      presentation: { kind: "canvas", nodes: [], edges: [], shapes: [], images: [], lines: [] },
    }), authority, commit)).rejects.toMatchObject({
      code: "standard_sem_authority.presentation_owned_by_layout",
      corrective_action: expect.stringContaining("canvas presentation layer"),
    });

    expect(commit).not.toHaveBeenCalled();
  });

  it("maps committed, blocked, stale, and rejected results to explicit accessible feedback", () => {
    expect(standardSemModelV4AdvancedFeedbackFor({ status: "committed", authority }).tone).toBe("committed");
    expect(standardSemModelV4AdvancedFeedbackFor({
      status: "blocked",
      diagnostic: {
        code: "model.invalid",
        message: "The complete model is not authorable.",
        correctiveAction: "Correct the listed field.",
        authoringIssues: [],
        readinessIssues: [],
      },
    })).toEqual(expect.objectContaining({ tone: "blocked", message: expect.stringContaining("Correct the listed field.") }));
    expect(standardSemModelV4AdvancedFeedbackFor({ status: "stale" }).tone).toBe("stale");
    expect(standardSemModelV4AdvancedFeedbackFor({ status: "rejected", error: new Error("offline failure") }))
      .toEqual({ tone: "rejected", message: "Rejected: offline failure" });
  });
});
