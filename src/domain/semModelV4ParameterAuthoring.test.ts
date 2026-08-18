import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData, SemModelV4ParameterAuthoringEntry } from "../types";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  type AuthoredNativeWorkbenchToSemModelV4Input,
} from "./nativeWorkbenchSemModelV4Adapter";
import { validateSemModelV4 } from "./semModelV4";
import { withNativeConstructEstimandV4 } from "./semModelV4Authoring";
import {
  nativeSemLatentMeanEntryV4,
  nativeSemObservedInterceptEntryV4,
  nativeSemOrdinalThresholdEntriesV4,
  parameterEntryFromSemParameterV4,
  unsupportedNativeSemParameterAuthoringDiagnosticsV4,
  validateNativeSemParameterSpecificationV4,
  withNativeSemFactorIdentificationV4,
  withNativeSemParameterEntriesOnConstructV4,
  withNativeSemParameterEntryOnConstructV4,
  withNativeSemParameterEntryOnEdgeV4,
} from "./semModelV4ParameterAuthoring";

function factor(id: string, indicators: string[], x: number): Node<ConstructData> {
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
  return {
    model_id: "parameter-authoring-model",
    model_name: "Parameter authoring model",
    nodes: [factor("x", ["x1", "x2", "x3"], 20), factor("y", ["y1", "y2", "y3"], 300)],
    edges: [{ id: "path-x-y", source: "x", target: "y" }],
    data_binding: {
      kind: "raw",
      dataset_id: "data-a",
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    },
    group: { kind: "single_group" },
    observed_semantics: {
      x1: { scale: "continuous" },
      x2: { scale: "continuous" },
      x3: { scale: "ordinal", categories: ["low", "middle", "high"] },
    },
  };
}

function adapted(source: AuthoredNativeWorkbenchToSemModelV4Input) {
  const result = adaptAuthoredNativeWorkbenchToSemModelV4(source);
  expect(result.ok, result.ok ? undefined : JSON.stringify(result.diagnostics)).toBe(true);
  if (!result.ok) throw new Error(result.diagnostics[0]?.message);
  return result;
}

describe("SemModelV4 parameter authoring", () => {
  it("persists free and fixed specifications on their exact owning objects", () => {
    const source = input();
    const initial = adapted(source);
    const loading = initial.model.parameters.find((parameter) => parameter.target.kind === "loading"
      && parameter.target.construct === "construct:x"
      && parameter.target.indicator === "observed:x2")!;
    const path = initial.model.parameters.find((parameter) => parameter.target.kind === "regression")!;

    const loadingEntry = parameterEntryFromSemParameterV4(loading);
    loadingEntry.specification = { kind: "free", start: 0.6, lower: 0, upper: 1, equality_label: "loading_a" };
    const pathEntry = parameterEntryFromSemParameterV4(path);
    pathEntry.specification = { kind: "fixed", value: 0.25 };
    source.nodes = source.nodes.map((node) => node.id === "x" ? withNativeSemParameterEntryOnConstructV4(node, loadingEntry) : node);
    source.edges = source.edges.map((edge) => edge.id === "path-x-y" ? withNativeSemParameterEntryOnEdgeV4(edge, pathEntry) : edge);

    const result = adapted(source);
    expect(result.model.parameters.find((parameter) => parameter.id === loading.id)).toMatchObject({
      kind: "free",
      start: 0.6,
      lower: 0,
      upper: 1,
      equality_label: "loading_a",
    });
    expect(result.model.parameters.find((parameter) => parameter.id === path.id)).toMatchObject({ kind: "fixed", value: 0.25 });
    expect(validateSemModelV4(result.model)).toEqual([]);
  });

  it("switches common-factor identification without retaining the generated marker constraint", () => {
    const fixedVariance = input();
    fixedVariance.nodes = fixedVariance.nodes.map((node) => node.id === "x"
      ? withNativeSemFactorIdentificationV4(node, { kind: "fixed_variance" })
      : node);
    const fixedResult = adapted(fixedVariance);
    const fixedFactor = fixedResult.model.variables.find((variable) => variable.id === "construct:x");
    expect(fixedFactor).toMatchObject({ kind: "common_factor", identification: { kind: "fixed_variance" } });
    expect(fixedResult.model.parameters.filter((parameter) => parameter.target.kind === "loading"
      && parameter.target.construct === "construct:x"
      && parameter.kind === "fixed")).toHaveLength(0);
    expect(fixedResult.model.parameters).toContainEqual(expect.objectContaining({
      kind: "fixed",
      value: 1,
      target: { kind: "variance", endpoint: { kind: "variable", id: "construct:x" } },
    }));

    const effectsCoded = input();
    effectsCoded.nodes = effectsCoded.nodes.map((node) => node.id === "x"
      ? withNativeSemFactorIdentificationV4(node, { kind: "effects_coding" })
      : node);
    const effectsResult = adapted(effectsCoded);
    expect(effectsResult.model.variables.find((variable) => variable.id === "construct:x")).toMatchObject({
      kind: "common_factor",
      identification: { kind: "effects_coding" },
    });
    expect(effectsResult.model.constraints).toContainEqual(expect.objectContaining({ kind: "linear", value: 3 }));
    expect(effectsResult.model.parameters.filter((parameter) => parameter.target.kind === "loading"
      && parameter.target.construct === "construct:x"
      && parameter.kind === "fixed")).toHaveLength(0);
  });

  it("adds supported observed intercepts, latent means, and ordered ordinal threshold targets", () => {
    const source = input();
    const x = source.nodes.find((node) => node.id === "x")!;
    source.nodes = source.nodes.map((node) => node.id === "x"
      ? withNativeSemParameterEntriesOnConstructV4(node, [
        nativeSemObservedInterceptEntryV4("x1"),
        nativeSemLatentMeanEntryV4("x"),
        ...nativeSemOrdinalThresholdEntriesV4("x3", 3),
      ])
      : node);

    expect(source.nodes.find((node) => node.id === "x")?.data.semModelV4?.parameters).toHaveLength(4);
    expect(x.data.semModelV4?.parameters).toBeUndefined();
    const result = adapted(source);
    expect(result.model.parameters).toEqual(expect.arrayContaining([
      expect.objectContaining({ target: { kind: "intercept", variable: "observed:x1" } }),
      expect.objectContaining({ target: { kind: "mean", variable: "construct:x" }, kind: "free" }),
      expect.objectContaining({ target: { kind: "threshold", variable: "observed:x3", index: 1 } }),
      expect.objectContaining({ target: { kind: "threshold", variable: "observed:x3", index: 2 } }),
    ]));
    expect(result.model.variables.find((variable) => variable.id === "construct:x")).toMatchObject({
      kind: "common_factor",
      mean_policy: { kind: "estimated" },
    });
    expect(validateSemModelV4(result.model)).toEqual([]);
  });

  it("returns typed actionable diagnostics for bounds, cross-object ownership, and unsupported group overrides", () => {
    expect(validateNativeSemParameterSpecificationV4("p", {
      kind: "free",
      start: 4,
      lower: 5,
      upper: 2,
      equality_label: "1 invalid",
    }).map((value) => value.code)).toEqual(expect.arrayContaining([
      "sem_model_v4.parameter.bounds_invalid",
      "sem_model_v4.parameter.start_outside_bounds",
      "sem_model_v4.parameter.equality_label_invalid",
    ]));

    const source = input();
    const initial = adapted(source);
    const loading = initial.model.parameters.find((parameter) => parameter.target.kind === "loading" && parameter.target.construct === "construct:y")!;
    const misplaced = parameterEntryFromSemParameterV4(loading);
    source.nodes = source.nodes.map((node) => node.id === "x" ? withNativeSemParameterEntryOnConstructV4(node, misplaced) : node);
    const ownership = adaptAuthoredNativeWorkbenchToSemModelV4(source);
    expect(ownership).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([expect.objectContaining({
        code: "sem_model_v4.parameter.owner_mismatch",
        corrective_action: expect.stringContaining("current source"),
      })]),
    });

    const tampered = input();
    const entry = parameterEntryFromSemParameterV4(initial.model.parameters.find((parameter) => parameter.target.kind === "loading")!);
    (entry as unknown as Record<string, unknown>).group_overrides = [];
    tampered.nodes = tampered.nodes.map((node) => node.id === "x" ? withNativeSemParameterEntryOnConstructV4(node, entry) : node);
    const groupOverride = adaptAuthoredNativeWorkbenchToSemModelV4(tampered);
    expect(groupOverride).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([expect.objectContaining({
        code: "sem_model_v4.parameter.entry_invalid",
        corrective_action: expect.stringContaining("group overrides"),
      })]),
    });
  });

  it("rejects one stable parameter ID authored on two different model objects", () => {
    const source = input();
    const initial = adapted(source);
    const path = initial.model.parameters.find((parameter) => parameter.target.kind === "regression")!;
    const entry = parameterEntryFromSemParameterV4(path);
    source.edges = source.edges.map((edge) => edge.id === "path-x-y" ? withNativeSemParameterEntryOnEdgeV4(edge, entry) : edge);
    source.nodes = source.nodes.map((node) => node.id === "x" ? withNativeSemParameterEntryOnConstructV4(node, entry) : node);

    const result = adaptAuthoredNativeWorkbenchToSemModelV4(source);
    expect(result).toMatchObject({
      ok: false,
      diagnostics: expect.arrayContaining([expect.objectContaining({
        code: "sem_model_v4.parameter.id_duplicate",
        corrective_action: expect.stringContaining("owning construct or relationship"),
      })]),
    });
  });

  it("detects feedback as unsupported for this authoring surface without removing it from SemModelV4", () => {
    const source = input();
    source.edges = [...source.edges, { id: "path-y-x", source: "y", target: "x" } as Edge];
    const result = adapted(source);
    expect(result.model.relations.filter((relation) => relation.kind === "structural")).toHaveLength(2);
    expect(unsupportedNativeSemParameterAuthoringDiagnosticsV4(result.model)).toEqual([
      expect.objectContaining({
        code: "sem_model_v4.parameter.feedback_not_available",
        corrective_action: expect.stringContaining("reciprocal path"),
      }),
    ]);
  });

  it("keeps stable metadata and scientific output under JSON reopen and declaration reorder", () => {
    const source = input();
    const initial = adapted(source);
    const loading = initial.model.parameters.find((parameter) => parameter.target.kind === "loading"
      && parameter.target.construct === "construct:x"
      && parameter.target.indicator === "observed:x2")!;
    const entry: SemModelV4ParameterAuthoringEntry = {
      ...parameterEntryFromSemParameterV4(loading),
      specification: { kind: "free", start: 0.4, lower: -1, upper: 1, equality_label: null },
    };
    source.nodes = source.nodes.map((node) => node.id === "x" ? withNativeSemParameterEntryOnConstructV4(node, entry) : node);

    const reopened = JSON.parse(JSON.stringify(source)) as AuthoredNativeWorkbenchToSemModelV4Input;
    reopened.nodes = [...reopened.nodes].reverse();
    reopened.edges = [...reopened.edges].reverse();
    expect(adapted(reopened).model).toEqual(adapted(source).model);
    expect(reopened.nodes.find((node) => node.id === "x")?.data.semModelV4?.parameters?.[0]?.parameter_id).toBe(loading.id);
  });
});
