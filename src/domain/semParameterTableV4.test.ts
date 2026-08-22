import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  type AuthoredNativeWorkbenchToSemModelV4Input,
} from "./nativeWorkbenchSemModelV4Adapter";
import {
  convertNativeCovarianceToPresentationV4,
  withNativeConstructEstimandV4,
} from "./semModelV4Authoring";
import {
  projectNativeWorkbenchSemParameterTableV4,
  projectSemModelV4ParameterTable,
} from "./semParameterTableV4";

function construct(id: string, indicators: string[], x: number): Node<ConstructData> {
  return withNativeConstructEstimandV4({
    id,
    type: "construct",
    position: { x, y: 40 },
    data: {
      label: id === "x" ? "Predictor" : "Outcome",
      shortName: id.toUpperCase(),
      mode: "reflective",
      indicators,
    },
  }, { kind: "common_factor", marker_indicator: indicators[0] });
}

function input(): AuthoredNativeWorkbenchToSemModelV4Input {
  const presentationCovariance = convertNativeCovarianceToPresentationV4({
    id: "visual-x-y",
    source: "x",
    target: "y",
    label: "Layout cue",
    data: { role: "covariance" },
  } as Edge);
  return {
    model_id: "model-a",
    model_name: "Model A",
    nodes: [construct("x", ["x1", "x2"], 20), construct("y", ["y1", "y2"], 300)],
    edges: [
      { id: "path-x-y", source: "x", target: "y" },
      presentationCovariance,
    ],
    data_binding: {
      kind: "raw",
      dataset_id: "data-a",
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    group: { kind: "single_group" },
  };
}

describe("SemModelV4 parameter-table projection", () => {
  it("projects scientific and presentation objects with stable source ids", () => {
    const projection = projectNativeWorkbenchSemParameterTableV4(input());

    expect(projection.status).toBe("ready");
    expect(projection.counts.scientific).toBeGreaterThan(0);
    expect(projection.counts.presentation).toBe(1);
    expect(projection.counts.unresolved).toBe(0);
    expect(projection.rows).toEqual(expect.arrayContaining([
      expect.objectContaining({ section: "variable", object_kind: "common_factor", source: { kind: "construct", id: "x" } }),
      expect.objectContaining({ section: "relation", object_kind: "structural", source: { kind: "edge", id: "path-x-y" } }),
      expect.objectContaining({ section: "parameter", object_kind: "regression", source: { kind: "edge", id: "path-x-y" } }),
      expect.objectContaining({ section: "annotation", classification: "presentation", source: { kind: "edge", id: "visual-x-y" } }),
      expect.objectContaining({ section: "group", object_kind: "single_group" }),
    ]));
    expect(Object.isFrozen(projection)).toBe(true);
    expect(Object.isFrozen(projection.rows)).toBe(true);
  });

  it("derives parameter rows directly from a resident SemModelV4 authority and its source trace", () => {
    const adapted = adaptAuthoredNativeWorkbenchToSemModelV4(input());
    expect(adapted.ok).toBe(true);
    if (!adapted.ok) throw new Error(adapted.diagnostics[0]?.message);

    const projection = projectSemModelV4ParameterTable(adapted.model, adapted.trace);
    expect(projection.status).toBe("ready");
    expect(projection.rows).toEqual(expect.arrayContaining([
      expect.objectContaining({ section: "variable", sem_id: "construct:x", source: { kind: "construct", id: "x" } }),
      expect.objectContaining({ section: "relation", sem_id: expect.any(String), source: { kind: "edge", id: "path-x-y" } }),
      expect.objectContaining({ section: "parameter", parameter_id: expect.any(String), classification: "scientific" }),
    ]));
  });

  it("is invariant to live node and edge declaration order", () => {
    const first = input();
    const reordered = input();
    reordered.nodes = [...reordered.nodes].reverse();
    reordered.edges = [...reordered.edges].reverse();

    expect(projectNativeWorkbenchSemParameterTableV4(reordered)).toEqual(
      projectNativeWorkbenchSemParameterTableV4(first),
    );
  });

  it("fails closed with selectable typed diagnostics for unresolved authoring intent", () => {
    const unresolved = input();
    unresolved.nodes = unresolved.nodes.map((node) => ({
      ...node,
      data: { ...node.data, semModelV4: undefined },
    }));
    unresolved.edges = unresolved.edges.map((edge) => edge.id === "visual-x-y"
      ? { ...edge, data: { role: "covariance" } }
      : edge);

    const projection = projectNativeWorkbenchSemParameterTableV4(unresolved);

    expect(projection.status).toBe("needs_attention");
    expect(projection.counts.scientific).toBe(0);
    expect(projection.counts.presentation).toBe(0);
    expect(projection.counts.unresolved).toBeGreaterThanOrEqual(3);
    expect(projection.rows.every((row) => row.classification === "unresolved" && row.section === "diagnostic")).toBe(true);
    expect(projection.diagnostics).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "native_workbench.estimand_confirmation_required", source: { kind: "construct", id: "x" } }),
      expect.objectContaining({ code: "native_workbench.covariance_classification_required", source: { kind: "edge", id: "visual-x-y" } }),
    ]));
  });

  it("covers constraints, observed groups, derived terms, and canvas-only objects", () => {
    const adapted = adaptAuthoredNativeWorkbenchToSemModelV4(input());
    expect(adapted.ok).toBe(true);
    if (!adapted.ok) throw new Error(adapted.diagnostics[0]?.message);
    const model = structuredClone(adapted.model);
    const parameterIds = model.parameters.filter((parameter) => parameter.kind === "free").slice(0, 2).map((parameter) => parameter.id);
    model.constraints = [{ kind: "equality", id: "same-loading", parameters: parameterIds }];
    model.group = {
      kind: "observed_groups",
      grouping_variable: "segment",
      levels: [
        { id: "a", value: "A", label: "Group A" },
        { id: "b", value: "B", label: "Group B" },
      ],
    };
    model.variables.push({ kind: "derived", id: "derived:x2", label: "Predictor squared" });
    model.derived_terms.push({ kind: "polynomial", id: "term:x2", output: "derived:x2", source: "construct:x", degree: 2 });
    if (model.presentation.kind === "canvas") model.presentation.shapes.push({
      id: "shape-note",
      shape: "rectangle",
      x: 1,
      y: 2,
      width: 30,
      height: 20,
      label: "Visual grouping",
    });

    const projection = projectSemModelV4ParameterTable(model, adapted.trace);
    expect(projection.rows).toEqual(expect.arrayContaining([
      expect.objectContaining({ section: "constraint", sem_id: "same-loading", classification: "scientific" }),
      expect.objectContaining({ section: "group", object_kind: "observed_groups", specification: "Group A (A), Group B (B)" }),
      expect.objectContaining({ section: "derived_term", sem_id: "term:x2", classification: "scientific" }),
      expect.objectContaining({ section: "presentation", object_kind: "shape", classification: "presentation" }),
    ]));
  });

  it("renders ordered interaction_v2 operands and hierarchy without treating the term as polynomial", () => {
    const adapted = adaptAuthoredNativeWorkbenchToSemModelV4(input());
    expect(adapted.ok).toBe(true);
    if (!adapted.ok) throw new Error(adapted.diagnostics[0]?.message);
    const model = structuredClone(adapted.model);
    const focal = model.relations.find((relation) => relation.kind === "structural");
    if (focal?.kind !== "structural") throw new Error("Expected a focal structural path.");
    const output = "derived:x-by-y:v2";
    const effectParameter = "parameter:x-by-y:v2";
    model.variables.push({ kind: "derived", id: output, label: "X by Y V2" });
    model.relations.push({
      kind: "structural",
      id: "relation:x-by-y:v2",
      source: output,
      target: focal.target,
      parameter: effectParameter,
      intercept_parameter: null,
    });
    model.parameters.push({
      kind: "free",
      id: effectParameter,
      label: "X by Y V2 effect",
      target: { kind: "regression", source: output, target: focal.target },
      group_overrides: [],
    });
    model.derived_terms.push({
      kind: "interaction_v2",
      id: "term:x-by-y:v2",
      output,
      operands: [focal.source, focal.target],
      focal_relation: focal.id,
      method: "two_stage",
      hierarchy_policy: "none",
    });

    const row = projectSemModelV4ParameterTable(model, adapted.trace).rows
      .find((candidate) => candidate.sem_id === "term:x-by-y:v2");
    expect(row).toMatchObject({
      section: "derived_term",
      object_kind: "interaction_v2",
      specification: "Predictor × Outcome; Two stage; None hierarchy",
    });
  });

  it("blocks unsupported feedback and group-specific parameter overrides with corrective diagnostics", () => {
    const feedbackInput = input();
    feedbackInput.edges = [...feedbackInput.edges, { id: "path-y-x", source: "y", target: "x" }];
    const feedback = projectNativeWorkbenchSemParameterTableV4(feedbackInput);
    expect(feedback).toMatchObject({
      status: "needs_attention",
      diagnostics: [expect.objectContaining({
        code: "sem_model_v4.parameter.feedback_not_available",
        corrective_action: expect.stringContaining("reciprocal path"),
      })],
    });

    const adapted = adaptAuthoredNativeWorkbenchToSemModelV4(input());
    expect(adapted.ok).toBe(true);
    if (!adapted.ok) throw new Error(adapted.diagnostics[0]?.message);
    const model = structuredClone(adapted.model);
    const parameter = model.parameters.find((candidate) => candidate.kind === "free")!;
    parameter.group_overrides = [{ group: "a", specification: { kind: "fixed", value: 1 } }];
    const grouped = projectSemModelV4ParameterTable(model, adapted.trace);
    expect(grouped).toMatchObject({
      status: "needs_attention",
      diagnostics: [expect.objectContaining({
        code: "sem_model_v4.parameter.group_overrides_not_available",
        corrective_action: expect.stringContaining("single-group"),
      })],
    });
  });
});
