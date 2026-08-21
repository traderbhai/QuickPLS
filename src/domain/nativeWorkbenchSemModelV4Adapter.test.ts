import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { parseSemModelV4, scientificSemModelV4HashInput } from "./semModelV4";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  adaptNativeWorkbenchToSemModelV4,
  nativeWorkbenchDerivedVariableIdV4,
  nativeWorkbenchObservedVariableIdV4,
  requireNativeWorkbenchSemModelV4,
  type NativeWorkbenchToSemModelV4Input,
} from "./nativeWorkbenchSemModelV4Adapter";
import {
  convertNativeCovarianceToPresentationV4,
  convertNativeCovarianceToScientificV4,
  newNativeScientificCovarianceEdgeV4,
  withNativeConstructEstimandV4,
} from "./semModelV4Authoring";

function construct(
  id: string,
  indicators: string[],
  position: { x: number; y: number },
  mode: ConstructData["mode"] = "reflective",
): Node<ConstructData> {
  return {
    id,
    type: "construct",
    position,
    data: {
      label: id === "x" ? "Predictor" : "Outcome",
      shortName: id.toUpperCase(),
      mode,
      indicators,
    },
  };
}

function baseInput(): NativeWorkbenchToSemModelV4Input {
  return {
    model_id: "live-model-v4",
    model_name: "Live model",
    nodes: [
      construct("x", ["x2", "x1"], { x: 20, y: 30 }),
      construct("y", ["y2", "y1"], { x: 300, y: 60 }),
    ],
    edges: [{ id: "path-x-y", source: "x", target: "y", type: "smoothstep" }],
    diagram_layout: {
      diagramViewport: { x: 12, y: 18, zoom: 1.25 },
      edgeLayouts: { "path-x-y": { routing: "curved" } },
    },
    data_binding: {
      kind: "raw",
      dataset_id: "data-live",
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    group: { kind: "single_group" },
    construct_estimands: {
      x: { kind: "common_factor", marker_indicator: "x1" },
      y: { kind: "common_factor", marker_indicator: "y1" },
    },
    covariance_semantics: {},
  };
}

function successful(input: NativeWorkbenchToSemModelV4Input) {
  const result = adaptNativeWorkbenchToSemModelV4(input);
  expect(result.ok, result.ok ? undefined : JSON.stringify(result.diagnostics)).toBe(true);
  if (!result.ok) throw new Error(result.diagnostics[0]?.message);
  return result;
}

function moderationInput(): NativeWorkbenchToSemModelV4Input {
  const input = baseInput();
  input.nodes = [
    construct("x", ["x1"], { x: 0, y: 0 }),
    construct("w", ["w1"], { x: 0, y: 120 }),
    construct("z", ["z1"], { x: 0, y: 240 }),
    construct("y", ["y1"], { x: 420, y: 120 }),
    {
      id: "x-w-y",
      type: "construct",
      position: { x: 210, y: 40 },
      data: {
        label: "X x W",
        shortName: "XW",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: "interaction-term:x-w-y",
          operands: ["x", "w"],
          outcome: "y",
          focalRelationId: "path-x-y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
          productIndicator: null,
        },
      },
    },
    {
      id: "x-z-y",
      type: "construct",
      position: { x: 210, y: 220 },
      data: {
        label: "X x Z",
        shortName: "XZ",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: "interaction-term:x-z-y",
          operands: ["x", "z"],
          outcome: "y",
          focalRelationId: "path-x-y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
          productIndicator: null,
        },
      },
    },
  ];
  input.edges = [
    { id: "path-x-y", source: "x", target: "y" },
    { id: "path-w-y", source: "w", target: "y" },
    { id: "path-z-y", source: "z", target: "y" },
    { id: "path-x-w-y", source: "x-w-y", target: "y" },
    { id: "path-x-z-y", source: "x-z-y", target: "y" },
  ];
  input.construct_estimands = {
    x: { kind: "composite" },
    w: { kind: "composite" },
    z: { kind: "composite" },
    y: { kind: "composite" },
  };
  return input;
}

function higherOrderInput(): NativeWorkbenchToSemModelV4Input {
  const input = baseInput();
  input.nodes = [
    construct("x", ["x1", "x2"], { x: 0, y: 0 }),
    construct("w", ["w1", "w2"], { x: 0, y: 180 }),
    construct("y", ["y1", "y2"], { x: 440, y: 90 }),
    {
      id: "hoc-quality",
      type: "construct",
      position: { x: 220, y: 90 },
      data: {
        label: "Service quality",
        shortName: "SQ",
        mode: "reflective",
        indicators: [],
        semantic: "higher_order",
        higherOrder: {
          id: "higher-order:service-quality",
          components: ["x", "w"],
          method: "two_stage",
          canonicalApproach: "disjoint_two_stage",
          measurementType: "reflective_reflective",
          stage_one_recipe: null,
        },
      },
    },
  ];
  input.edges = [{ id: "path-hoc-y", source: "hoc-quality", target: "y" }];
  input.construct_estimands = {
    x: { kind: "composite" },
    w: { kind: "composite" },
    y: { kind: "composite" },
  };
  return input;
}

describe("native workbench to SemModelV4 adapter", () => {
  it("never infers or silently accepts reserved CB-SEM Special Assumptions", () => {
    const input = baseInput();
    input.special_assumptions = {
      imply_exogenous_latent_correlations: true,
      imply_causal_indicator_correlations: true,
      fix_causal_indicator_variances_to_one: true,
    };

    const result = adaptNativeWorkbenchToSemModelV4(input);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.special_assumption.imply_exogenous_latent_correlations_not_available" }),
        expect.objectContaining({ code: "native_workbench.special_assumption.imply_causal_indicator_correlations_not_available" }),
        expect.objectContaining({ code: "native_workbench.special_assumption.fix_causal_indicator_variances_to_one_not_available" }),
      ]),
    });

    input.special_assumptions = {
      imply_exogenous_latent_correlations: false,
      imply_causal_indicator_correlations: false,
      fix_causal_indicator_variances_to_one: false,
    };
    expect(successful(input).model).toEqual(successful(baseInput()).model);
  });

  it("preserves an explicitly scientific latent covariance as an executable relation", () => {
    const input = baseInput();
    input.edges = [
      ...input.edges,
      { id: "cov-x-y", source: "x", target: "y", label: "X with Y", data: { role: "covariance" } },
    ];
    input.covariance_semantics = { "cov-x-y": { kind: "scientific" } };

    const result = successful(input);
    const covariance = result.model.relations.find((relation) => relation.kind === "covariance");
    expect(covariance).toMatchObject({
      kind: "covariance",
      left: { kind: "variable", id: "construct:x" },
      right: { kind: "variable", id: "construct:y" },
    });
    expect(result.trace.edge_objects["cov-x-y"]).toMatchObject({
      kind: "scientific_relation",
      sem_id: covariance?.id,
    });
    expect(result.model.presentation.kind === "canvas" && result.model.presentation.edges.some((edge) => edge.relation === covariance?.id)).toBe(true);
  });

  it("preserves an explicitly scientific residual covariance without collapsing it to a latent covariance", () => {
    const input = baseInput();
    input.edges = [
      ...input.edges,
      { id: "residual-x1-y1", source: "x", target: "y", data: { role: "covariance" } },
    ];
    input.covariance_semantics = {
      "residual-x1-y1": {
        kind: "scientific",
        left: { kind: "residual_of", id: nativeWorkbenchObservedVariableIdV4("x1") },
        right: { kind: "residual_of", id: nativeWorkbenchObservedVariableIdV4("y1") },
      },
    };

    const result = successful(input);
    expect(result.model.relations.find((relation) => relation.kind === "covariance")).toMatchObject({
      left: { kind: "residual_of", id: "observed:x1" },
      right: { kind: "residual_of", id: "observed:y1" },
    });
  });

  it("classifies a presentation-only covariance as an annotation that cannot affect the scientific hash", () => {
    const baseline = baseInput();
    const scientificBefore = scientificSemModelV4HashInput(requireNativeWorkbenchSemModelV4(baseline));
    const input = baseInput();
    input.edges = [...input.edges, { id: "visual-x-y", source: "x", target: "y", label: "Layout cue", data: { role: "covariance" } }];
    input.covariance_semantics = { "visual-x-y": { kind: "presentation_only" } };

    const result = successful(input);
    expect(result.model.relations.filter((relation) => relation.kind === "covariance")).toEqual([]);
    expect(result.model.annotations).toEqual([expect.objectContaining({
      kind: "display_only_covariance",
      left: "construct:x",
      right: "construct:y",
      label: "Layout cue",
    })]);
    expect(result.trace.edge_objects["visual-x-y"].kind).toBe("presentation_annotation");
    expect(scientificSemModelV4HashInput(result.model)).toBe(scientificBefore);
  });

  it("keeps factor and composite estimands scientifically distinct for the same live graph", () => {
    const factors = successful(baseInput()).model;
    const compositeInput = baseInput();
    compositeInput.construct_estimands = { x: { kind: "composite" }, y: { kind: "composite" } };
    const composites = successful(compositeInput).model;

    expect(factors.variables.filter((variable) => variable.kind === "common_factor")).toHaveLength(2);
    expect(composites.variables.filter((variable) => variable.kind === "composite")).toHaveLength(2);
    expect(factors.parameters.some((parameter) => parameter.target.kind === "variance" && parameter.target.endpoint.kind === "residual_of")).toBe(true);
    expect(composites.parameters.some((parameter) => parameter.target.kind === "variance")).toBe(false);
    expect(scientificSemModelV4HashInput(factors)).not.toBe(scientificSemModelV4HashInput(composites));
  });

  it("fails closed when a legacy construct has no factor-versus-composite decision", () => {
    const input = baseInput();
    input.construct_estimands = {
      x: { kind: "legacy_estimand_unspecified" },
      y: { kind: "common_factor" },
    };
    const result = adaptNativeWorkbenchToSemModelV4(input);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [expect.objectContaining({
        code: "native_workbench.estimand_confirmation_required",
        subject: "x",
        corrective_action: expect.stringContaining("Choose Composite"),
      })],
    });
  });

  it("is stable across JSON save/reopen and strict SemModelV4 parsing", () => {
    const input = baseInput();
    input.edges = [...input.edges, { id: "cov-x-y", source: "x", target: "y", data: { role: "covariance" } }];
    input.covariance_semantics = { "cov-x-y": { kind: "scientific" } };
    const first = successful(input);
    const reopenedInput = JSON.parse(JSON.stringify(input)) as NativeWorkbenchToSemModelV4Input;
    const reopened = successful(reopenedInput);
    const reopenedModel = parseSemModelV4(JSON.parse(JSON.stringify(first.model)));

    expect(first.model.presentation.kind === "canvas" && first.model.presentation.nodes.every((node) => node.style && Object.keys(node.style).length === 0)).toBe(true);
    expect(reopened.model).toEqual(first.model);
    expect(reopened.trace).toEqual(first.trace);
    expect(reopenedModel).toEqual(first.model);
    expect(Object.isFrozen(first.model)).toBe(true);
  });

  it("preserves an explicit data binding and its observed control metadata", () => {
    const input = baseInput();
    input.data_binding = {
      kind: "raw",
      dataset_id: "weighted-data",
      missing_data: "mean_replacement",
      weight: { kind: "case", variable: "observed:case_weight" },
      cluster_variable: null,
      strata_variable: null,
    };
    input.observed_semantics = {
      case_weight: { label: "Case weight", scale: "continuous", role: "control" },
    };

    const result = successful(input);
    expect(result.model.data_binding).toEqual(input.data_binding);
    expect(result.model.variables).toContainEqual(expect.objectContaining({
      kind: "observed",
      id: "observed:case_weight",
      source_column: "case_weight",
      role: "control",
    }));
    expect(result.model.data_binding).not.toBe(input.data_binding);
  });

  it("keeps scientific IDs and canonical output invariant to node, edge, indicator, and declaration order", () => {
    const firstInput = baseInput();
    firstInput.edges = [
      ...firstInput.edges,
      { id: "cov-x-y", source: "x", target: "y", data: { role: "covariance" } },
    ];
    firstInput.covariance_semantics = { "cov-x-y": { kind: "scientific" } };
    const first = successful(firstInput);

    const reordered = baseInput();
    reordered.nodes = [...reordered.nodes].reverse().map((node) => ({
      ...node,
      data: { ...node.data, indicators: [...node.data.indicators].reverse() },
    }));
    reordered.edges = [
      { id: "cov-x-y", source: "x", target: "y", data: { role: "covariance" } },
      ...reordered.edges,
    ].reverse();
    reordered.construct_estimands = {
      y: { kind: "common_factor", marker_indicator: "y1" },
      x: { kind: "common_factor", marker_indicator: "x1" },
    };
    reordered.covariance_semantics = { "cov-x-y": { kind: "scientific" } };
    const second = successful(reordered);

    expect(second.model).toEqual(first.model);
    expect(second.trace).toEqual(first.trace);
    expect(scientificSemModelV4HashInput(second.model)).toBe(scientificSemModelV4HashInput(first.model));
  });

  it("returns an actionable diagnostic for a stale drawn endpoint", () => {
    const input = baseInput();
    input.edges = [{ id: "bad-path", source: "x", target: "deleted-construct" }];
    const result = adaptNativeWorkbenchToSemModelV4(input);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [expect.objectContaining({
        code: "native_workbench.edge_endpoint_unknown",
        subject: "bad-path",
        corrective_action: expect.stringContaining("Reconnect"),
      })],
    });
  });

  it("rejects a stale generated measurement edge endpoint", () => {
    const input = baseInput();
    input.edges = [{
      id: "measurement::x::x1",
      source: "x",
      target: "indicator::x::deleted",
    }];
    const result = adaptNativeWorkbenchToSemModelV4(input);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [expect.objectContaining({
        code: "native_workbench.measurement_edge_endpoint_invalid",
        corrective_action: expect.stringContaining("Refresh"),
      })],
    });
  });

  it("returns an actionable diagnostic for an unknown residual endpoint", () => {
    const input = baseInput();
    input.edges = [{ id: "bad-residual", source: "x", target: "y", data: { role: "covariance" } }];
    input.covariance_semantics = {
      "bad-residual": {
        kind: "scientific",
        left: { kind: "residual_of", id: "observed:not-present" },
        right: { kind: "residual_of", id: "observed:y1" },
      },
    };
    const result = adaptNativeWorkbenchToSemModelV4(input);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [expect.objectContaining({
        code: "native_workbench.covariance_endpoint_unknown",
        subject: "bad-residual",
        corrective_action: expect.stringContaining("existing construct or indicator"),
      })],
    });
  });

  it("never guesses whether a current covariance edge is scientific", () => {
    const input = baseInput();
    input.edges = [{ id: "unclassified", source: "x", target: "y", data: { role: "covariance" } }];
    const result = adaptNativeWorkbenchToSemModelV4(input);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [expect.objectContaining({
        code: "native_workbench.covariance_classification_required",
        corrective_action: expect.stringContaining("Model covariance"),
      })],
    });
  });

  it("reads explicit persisted authoring metadata without separate semantic maps", () => {
    const input = baseInput();
    const nodes = input.nodes.map((node) => withNativeConstructEstimandV4(node, { kind: "common_factor", marker_indicator: node.id === "x" ? "x1" : "y1" }));
    const covariance = newNativeScientificCovarianceEdgeV4("authored-covariance", "x", "y");
    const result = adaptAuthoredNativeWorkbenchToSemModelV4({
      ...input,
      nodes,
      edges: [...input.edges, covariance],
    });
    expect(result.ok, result.ok ? undefined : JSON.stringify(result.diagnostics)).toBe(true);
    if (!result.ok) throw new Error(result.diagnostics[0]?.message);
    expect(result.model.relations).toContainEqual(expect.objectContaining({ kind: "covariance" }));
  });

  it("keeps a persisted presentation-only conversion outside the scientific model", () => {
    const input = baseInput();
    const nodes = input.nodes.map((node) => withNativeConstructEstimandV4(node, { kind: "composite" }));
    const scientific = newNativeScientificCovarianceEdgeV4("authored-covariance", "x", "y");
    const presentation = convertNativeCovarianceToPresentationV4(scientific);
    const result = adaptAuthoredNativeWorkbenchToSemModelV4({ ...input, nodes, edges: [...input.edges, presentation] });
    expect(result.ok).toBe(true);
    if (!result.ok) throw new Error(result.diagnostics[0]?.message);
    expect(result.model.relations.some((relation) => relation.kind === "covariance")).toBe(false);
    expect(result.model.annotations).toContainEqual(expect.objectContaining({ kind: "display_only_covariance" }));
  });

  it("does not reinterpret a reopened legacy covariance as scientific", () => {
    const input = baseInput();
    const nodes = input.nodes.map((node) => withNativeConstructEstimandV4(node, { kind: "composite" }));
    const legacy: Edge = { id: "legacy-covariance", source: "x", target: "y", data: { role: "covariance" } };
    const result = adaptAuthoredNativeWorkbenchToSemModelV4({ ...input, nodes, edges: [...input.edges, legacy] });
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [expect.objectContaining({ code: "native_workbench.covariance_classification_required", subject: "legacy-covariance" })],
    });

    const upgraded = convertNativeCovarianceToScientificV4(legacy, {
      left: { kind: "residual_of", id: "observed:x1" },
      right: { kind: "residual_of", id: "observed:y1" },
    });
    const converted = adaptAuthoredNativeWorkbenchToSemModelV4({ ...input, nodes, edges: [...input.edges, upgraded] });
    expect(converted.ok).toBe(true);
    if (!converted.ok) throw new Error(converted.diagnostics[0]?.message);
    expect(converted.model.relations).toContainEqual(expect.objectContaining({
      kind: "covariance",
      left: { kind: "residual_of", id: "observed:x1" },
      right: { kind: "residual_of", id: "observed:y1" },
    }));
  });

  it("preserves simultaneous interaction_v2 terms with exact focal and effect relations", () => {
    const first = successful(moderationInput());
    expect(first.model.derived_terms).toEqual([
      {
        kind: "interaction_v2",
        id: "interaction-term:x-w-y",
        output: nativeWorkbenchDerivedVariableIdV4("x-w-y"),
        operands: ["construct:x", "construct:w"],
        focal_relation: first.trace.edge_objects["path-x-y"].sem_id,
        method: "two_stage",
        hierarchy_policy: "strong",
      },
      {
        kind: "interaction_v2",
        id: "interaction-term:x-z-y",
        output: nativeWorkbenchDerivedVariableIdV4("x-z-y"),
        operands: ["construct:x", "construct:z"],
        focal_relation: first.trace.edge_objects["path-x-y"].sem_id,
        method: "two_stage",
        hierarchy_policy: "strong",
      },
    ]);
    expect(first.model.variables).toEqual(expect.arrayContaining([
      expect.objectContaining({ kind: "derived", id: nativeWorkbenchDerivedVariableIdV4("x-w-y") }),
      expect.objectContaining({ kind: "derived", id: nativeWorkbenchDerivedVariableIdV4("x-z-y") }),
    ]));
    expect(first.model.relations).toEqual(expect.arrayContaining([
      expect.objectContaining({ kind: "structural", source: nativeWorkbenchDerivedVariableIdV4("x-w-y"), target: "construct:y" }),
      expect.objectContaining({ kind: "structural", source: nativeWorkbenchDerivedVariableIdV4("x-z-y"), target: "construct:y" }),
    ]));

    const reordered = moderationInput();
    reordered.nodes = [...reordered.nodes].reverse();
    reordered.edges = [...reordered.edges].reverse();
    const second = successful(reordered);
    expect(second.model).toEqual(first.model);
    expect(second.trace).toEqual(first.trace);
    expect(parseSemModelV4(JSON.parse(JSON.stringify(first.model)))).toEqual(first.model);

    const authored = moderationInput();
    authored.nodes = authored.nodes.map((node) => node.data.semantic === "interaction"
      ? node
      : withNativeConstructEstimandV4(node, { kind: "composite" }));
    const {
      construct_estimands: _constructEstimands,
      covariance_semantics: _covarianceSemantics,
      ...authoredInput
    } = authored;
    const authoredResult = adaptAuthoredNativeWorkbenchToSemModelV4(authoredInput);
    expect(authoredResult.ok, authoredResult.ok ? undefined : JSON.stringify(authoredResult.diagnostics)).toBe(true);
    if (!authoredResult.ok) throw new Error(authoredResult.diagnostics[0]?.message);
    expect(authoredResult.model).toEqual(first.model);
  });

  it("maps a complete higher-order node to one stable derived term and preserves its structural path", () => {
    const first = successful(higherOrderInput());
    const outputId = nativeWorkbenchDerivedVariableIdV4("hoc-quality");
    expect(first.model.derived_terms).toContainEqual({
      kind: "higher_order",
      id: "higher-order:service-quality",
      output: outputId,
      components: ["construct:w", "construct:x"],
      approach: "disjoint_two_stage",
      measurement_type: "reflective_reflective",
    });
    expect(first.model.variables).toContainEqual({
      kind: "derived",
      id: outputId,
      label: "Service quality",
    });
    expect(first.model.relations).toContainEqual(expect.objectContaining({
      kind: "structural",
      source: outputId,
      target: "construct:y",
    }));
    expect(first.trace.construct_variables["hoc-quality"]).toBe(outputId);

    const reordered = higherOrderInput();
    reordered.nodes = [...reordered.nodes].reverse();
    reordered.edges = [...reordered.edges].reverse();
    expect(successful(reordered).model).toEqual(first.model);

    const authored = higherOrderInput();
    authored.nodes = authored.nodes.map((node) => node.data.semantic === "higher_order"
      ? node
      : withNativeConstructEstimandV4(node, { kind: "composite" }));
    const {
      construct_estimands: _constructEstimands,
      covariance_semantics: _covarianceSemantics,
      ...authoredInput
    } = authored;
    const authoredResult = adaptAuthoredNativeWorkbenchToSemModelV4(authoredInput);
    expect(authoredResult.ok, authoredResult.ok ? undefined : JSON.stringify(authoredResult.diagnostics)).toBe(true);
    if (!authoredResult.ok) throw new Error(authoredResult.diagnostics[0]?.message);
    expect(authoredResult.model).toEqual(first.model);
  });

  it("rejects incomplete higher-order metadata and invalid components before scientific conversion", () => {
    const incomplete = higherOrderInput();
    incomplete.nodes = incomplete.nodes.map((node) => node.id === "hoc-quality"
      ? {
          ...node,
          data: {
            ...node.data,
            higherOrder: {
              ...node.data.higherOrder!,
              canonicalApproach: undefined,
              measurementType: undefined,
            },
          },
        }
      : node);
    expect(adaptNativeWorkbenchToSemModelV4(incomplete)).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.higher_order_approach_invalid", subject: "hoc-quality" }),
        expect.objectContaining({ code: "native_workbench.higher_order_measurement_type_invalid", subject: "hoc-quality" }),
      ]),
    });

    const malformed = higherOrderInput();
    malformed.nodes = malformed.nodes.map((node) => node.id === "hoc-quality"
      ? {
          ...node,
          data: {
            ...node.data,
            higherOrder: {
              ...node.data.higherOrder!,
              components: ["x", "x"],
              measurementType: "formative_reflective",
            },
          },
        }
      : node);
    expect(adaptNativeWorkbenchToSemModelV4(malformed)).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.higher_order_components_invalid", subject: "hoc-quality" }),
        expect.objectContaining({ code: "native_workbench.higher_order_component_mode_mismatch", subject: "hoc-quality" }),
      ]),
    });
  });

  it("rejects legacy or malformed interaction metadata instead of silently downgrading it", () => {
    const legacy = moderationInput();
    const legacyNode = legacy.nodes.find((node) => node.id === "x-w-y")!;
    legacy.nodes = legacy.nodes.map((node) => node.id === legacyNode.id
      ? {
          ...node,
          data: {
            ...node.data,
            interaction: { predictor: "x", moderator: "w", outcome: "y", method: "two_stage_product_score" },
          },
        }
      : node);
    expect(adaptNativeWorkbenchToSemModelV4(legacy)).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.derived_construct_requires_explicit_v4_definition", subject: "x-w-y" }),
      ]),
    });

    const staleFocal = moderationInput();
    staleFocal.nodes = staleFocal.nodes.map((node) => node.id === "x-w-y"
      ? {
          ...node,
          data: {
            ...node.data,
            interaction: { ...node.data.interaction!, focalRelationId: "path-w-y" },
          },
        }
      : node);
    expect(adaptNativeWorkbenchToSemModelV4(staleFocal)).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.interaction_v2_focal_relation_invalid", subject: "x-w-y" }),
      ]),
    });

    const falseyProductSettings = moderationInput();
    falseyProductSettings.nodes = falseyProductSettings.nodes.map((node) => node.id === "x-w-y"
      ? {
          ...node,
          data: {
            ...node.data,
            interaction: {
              ...node.data.interaction!,
              productIndicator: false,
            } as unknown as NonNullable<ConstructData["interaction"]>,
          },
        }
      : node);
    expect(adaptNativeWorkbenchToSemModelV4(falseyProductSettings)).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.interaction_v2_product_indicator_forbidden", subject: "x-w-y" }),
      ]),
    });

    const malformedProductSettings = moderationInput();
    malformedProductSettings.nodes = malformedProductSettings.nodes.map((node) => node.id === "x-w-y"
      ? {
          ...node,
          data: {
            ...node.data,
            interaction: {
              ...node.data.interaction!,
              canonicalMethod: "product_indicator",
              productIndicator: { centering: "unknown", standardization: "none", pairing: "all_pairs" },
            } as unknown as NonNullable<ConstructData["interaction"]>,
          },
        }
      : node);
    expect(adaptNativeWorkbenchToSemModelV4(malformedProductSettings)).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([
        expect.objectContaining({ code: "native_workbench.interaction_v2_product_indicator_invalid", subject: "x-w-y" }),
      ]),
    });
  });
});
