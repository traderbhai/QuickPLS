import { describe, expect, it } from "vitest";
import {
  convertLegacyBasicModelV4,
  parseSemModelV4AuthoringDraft,
  type LegacyBasicModelV4Input,
  type SemModelV4,
  type SemParameterV4,
} from "./semModelV4";
import {
  applySemModelV4AuthorityOperationBatchV1,
  parseSemModelV4AuthorityOperationBatchJsonV1,
  parseSemModelV4AuthorityOperationBatchV1,
  SEM_MODEL_V4_AUTHORITY_OPERATION_UNSUPPORTED_ACTIONS,
  type SemModelV4AuthorityOperationBatchV1,
} from "./semModelV4AuthorityOperations";

function baseModel(): SemModelV4 {
  const legacy: LegacyBasicModelV4Input = {
    id: "authority-model",
    name: "Canonical authority model",
    constructs: [
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "m", name: "Moderator", short_name: "M", mode: "reflective", indicators: ["m1", "m2"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [
      { source: "x", target: "y" },
      { source: "m", target: "y" },
    ],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  };
  return convertLegacyBasicModelV4(legacy, "cbsem_common_factor");
}

function advancedBatch(model: SemModelV4): SemModelV4AuthorityOperationBatchV1 {
  const loading = model.parameters.find((parameter) => (
    parameter.kind === "free" && parameter.target.kind === "loading"
  ));
  if (loading?.kind !== "free") throw new Error("Expected a free loading fixture.");
  const replacement: SemParameterV4 = {
    ...loading,
    start: 0.7,
    lower: 0,
    upper: 1,
    equality_label: "loading_equal",
    group_overrides: [
      { group: "a", specification: { kind: "fixed", value: 0.7 } },
      { group: "b", specification: { kind: "free", start: 0.7, lower: 0, upper: 1 } },
    ],
  };
  return {
    schema_version: 1,
    expected_model_id: model.id,
    operations: [
      {
        kind: "set_group",
        group: {
          kind: "observed_groups",
          grouping_variable: "observed:x1",
          levels: [
            { id: "a", value: "A", label: "Group A" },
            { id: "b", value: "B", label: "Group B" },
          ],
        },
      },
      { kind: "replace_parameter", parameter_id: loading.id, replacement },
      { kind: "append_variable", variable: { kind: "derived", id: "derived:x-square", label: "X squared" } },
      {
        kind: "append_parameter",
        parameter: {
          kind: "free",
          id: "parameter:x-square-y",
          label: "X squared to Y",
          target: { kind: "regression", source: "derived:x-square", target: "construct:y" },
          start: 0.1,
          lower: -1,
          upper: 1,
          equality_label: null,
          group_overrides: [],
        },
      },
      {
        kind: "append_relation",
        relation: {
          kind: "structural",
          id: "relation:x-square-y",
          source: "derived:x-square",
          target: "construct:y",
          parameter: "parameter:x-square-y",
          intercept_parameter: null,
        },
      },
      {
        kind: "append_derived_term",
        term: { kind: "polynomial", id: "term:x-square", output: "derived:x-square", source: "construct:x", degree: 2 },
      },
      {
        kind: "append_constraint",
        constraint: { kind: "bound", id: "constraint:x-square", parameter: "parameter:x-square-y", lower: -0.8, upper: 0.8 },
      },
    ],
  };
}

describe("SemModelV4 canonical authority operation batches", () => {
  it("applies advanced scientific edits atomically without mutating the selected authority", () => {
    const source = baseModel();
    const sourceJson = JSON.stringify(source);
    const batch = parseSemModelV4AuthorityOperationBatchJsonV1(JSON.stringify(advancedBatch(source)));
    const result = applySemModelV4AuthorityOperationBatchV1(source, batch);

    expect(JSON.stringify(source)).toBe(sourceJson);
    expect(result.readiness).toBe("ready");
    expect(result.readiness_issues).toEqual([]);
    expect(result.model.group).toMatchObject({ kind: "observed_groups", grouping_variable: "observed:x1" });
    expect(result.model.derived_terms).toContainEqual(expect.objectContaining({ kind: "polynomial", degree: 2 }));
    expect(result.model.relations).toContainEqual(expect.objectContaining({ id: "relation:x-square-y", source: "derived:x-square" }));
    expect(result.model.constraints).toContainEqual(expect.objectContaining({ id: "constraint:x-square", kind: "bound" }));
    const edited = result.model.parameters.find((parameter) => (
      parameter.kind === "free" && parameter.equality_label === "loading_equal"
    ));
    expect(edited).toMatchObject({ start: 0.7, lower: 0, upper: 1 });
    expect(edited?.group_overrides).toHaveLength(2);

    const reopened = parseSemModelV4AuthoringDraft(JSON.parse(JSON.stringify(result.model)));
    expect(reopened).toEqual(result.model);
  });

  it("distinguishes an authoring-integrity-safe draft from a ready scientific model", () => {
    const source = baseModel();
    const factor = source.variables.find((variable) => variable.kind === "common_factor");
    if (factor?.kind !== "common_factor") throw new Error("Expected a common factor.");
    const result = applySemModelV4AuthorityOperationBatchV1(source, {
      schema_version: 1,
      expected_model_id: source.id,
      operations: [{
        kind: "replace_variable",
        variable_id: factor.id,
        replacement: { ...factor, identification: { kind: "fixed_variance" } },
      }],
    });

    expect(result.readiness).toBe("draft");
    expect(result.readiness_issues.map((issue) => issue.code)).toContain("identification.fixed_variance.missing");
  });

  it("fails closed on stale identities, missing targets, unknown operation fields, and nested model drift", () => {
    const source = baseModel();
    const sourceJson = JSON.stringify(source);
    expect(() => applySemModelV4AuthorityOperationBatchV1(source, {
      ...advancedBatch(source),
      expected_model_id: "stale-model",
    })).toThrowError(expect.objectContaining({ code: "sem_model_v4.authority_operation.model_id_mismatch" }));

    expect(() => applySemModelV4AuthorityOperationBatchV1(source, {
      schema_version: 1,
      expected_model_id: source.id,
      operations: [{
        kind: "replace_parameter",
        parameter_id: "missing-parameter",
        replacement: source.parameters[0],
      }],
    })).toThrowError(expect.objectContaining({ code: "sem_model_v4.authority_operation.target_missing" }));

    expect(() => parseSemModelV4AuthorityOperationBatchV1({
      schema_version: 1,
      expected_model_id: source.id,
      operations: [{ kind: "append_variable", variable: source.variables[0], invented: true }],
    })).toThrowError(expect.objectContaining({ code: "sem_model_v4.authority_operation.field_unknown" }));

    expect(() => applySemModelV4AuthorityOperationBatchV1(source, parseSemModelV4AuthorityOperationBatchV1({
      schema_version: 1,
      expected_model_id: source.id,
      operations: [{
        kind: "append_variable",
        variable: { kind: "derived", id: "derived:bad", label: "Bad", invented: true },
      }],
    }))).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));
    expect(JSON.stringify(source)).toBe(sourceJson);
  });

  it("enumerates edits intentionally excluded from the bounded operation lane", () => {
    expect(SEM_MODEL_V4_AUTHORITY_OPERATION_UNSUPPORTED_ACTIONS.map((entry) => entry.action)).toEqual([
      "delete_or_reorder",
      "change_model_identity",
      "edit_annotations_or_presentation",
    ]);
    expect(SEM_MODEL_V4_AUTHORITY_OPERATION_UNSUPPORTED_ACTIONS.every((entry) => entry.correctiveAction.length > 20)).toBe(true);
  });
});
