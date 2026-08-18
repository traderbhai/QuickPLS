import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { newNativeScientificCovarianceEdgeV4 } from "./semModelV4Authoring";
import {
  confirmNativeSemConstructAuthoringV4,
  confirmNativeSemCovarianceAuthoringV4,
  nativeSemCovarianceChoiceSignatureV4,
  projectNativeSemConstructAuthoringV4,
  projectNativeSemCovarianceAuthoringV4,
  validateNativeSemCovarianceChoiceV4,
} from "./semModelV4ScientificAuthoring";

function node(id: string, indicators: string[] = [`${id}1`, `${id}2`]): Node<ConstructData> {
  return {
    id,
    type: "construct",
    position: { x: 0, y: 0 },
    data: { label: id.toUpperCase(), shortName: id.toUpperCase(), mode: "reflective", indicators },
  };
}

function path(id: string, source: string, target: string): Edge {
  return { id, source, target, type: "straight", label: "Path" };
}

describe("SemModelV4 scientific authoring workflow", () => {
  it("fails closed on ambiguous legacy constructs and confirms stable factor/composite intent", () => {
    const legacy = node("x", ["x2", "x1"]);
    const projection = projectNativeSemConstructAuthoringV4(legacy);
    expect(projection).toMatchObject({ choice: "decision_required", marker_candidates: ["x1", "x2"] });
    expect(projection.diagnostics[0]).toMatchObject({
      code: "sem_model_v4.authoring.construct_decision_required",
      corrective_action: expect.stringContaining("Choose Composite"),
    });

    const factor = confirmNativeSemConstructAuthoringV4(legacy, "common_factor");
    expect(factor.ok).toBe(true);
    if (!factor.ok) return;
    expect(factor.node.data.semModelV4).toEqual({
      version: 1,
      construct: { kind: "common_factor", marker_indicator: "x1" },
      identification: { kind: "marker_loading", indicator: "x1" },
    });
    expect(projectNativeSemConstructAuthoringV4(factor.node)).toMatchObject({
      choice: "common_factor",
      marker_indicator: "x1",
      diagnostics: [],
    });

    const composite = confirmNativeSemConstructAuthoringV4(legacy, "composite");
    expect(composite.ok && composite.node.data.semModelV4?.construct).toEqual({ kind: "composite" });
    expect(legacy.data.semModelV4).toBeUndefined();
  });

  it("requires an assigned marker before a common factor can be confirmed", () => {
    const result = confirmNativeSemConstructAuthoringV4(node("empty", []), "common_factor");
    expect(result).toMatchObject({
      ok: false,
      diagnostics: [{
        code: "sem_model_v4.authoring.common_factor_indicator_required",
        corrective_action: expect.stringContaining("Assign at least one indicator"),
      }],
    });
  });

  it("represents all four covariance uses without changing the drawn stable id", () => {
    const nodes = [node("x"), node("y")];
    const covariance = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    expect(projectNativeSemCovarianceAuthoringV4(covariance, nodes, [covariance]).choice).toEqual({ kind: "model_covariance" });

    const residual = confirmNativeSemCovarianceAuthoringV4(covariance, nodes, [covariance], {
      kind: "residual_covariance",
      source_indicator: "x2",
      target_indicator: "y1",
    });
    expect(residual.ok).toBe(true);
    if (!residual.ok) return;
    expect(residual.edge.id).toBe(covariance.id);
    expect(residual.edge.data?.semModelV4).toEqual({
      version: 1,
      covariance: {
        kind: "scientific",
        origin: "explicit_conversion",
        left: { kind: "residual_of", id: "observed:x2" },
        right: { kind: "residual_of", id: "observed:y1" },
      },
    });
    expect(projectNativeSemCovarianceAuthoringV4(residual.edge, nodes, [residual.edge]).choice).toEqual({
      kind: "residual_covariance",
      source_indicator: "x2",
      target_indicator: "y1",
    });

    const presentation = confirmNativeSemCovarianceAuthoringV4(residual.edge, nodes, [residual.edge], { kind: "presentation_only" });
    expect(presentation.ok && presentation.edge.id).toBe(covariance.id);
    expect(presentation.ok && presentation.edge.data?.semModelV4).toMatchObject({ covariance: { kind: "presentation_only" } });
  });

  it("validates residual endpoints against the two constructs on the canvas", () => {
    const nodes = [node("x"), node("y")];
    const covariance = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    const diagnostics = validateNativeSemCovarianceChoiceV4(covariance, nodes, [covariance], {
      kind: "residual_covariance",
      source_indicator: "y1",
      target_indicator: "",
    });
    expect(diagnostics.map((item) => item.code)).toEqual([
      "sem_model_v4.authoring.source_residual_indicator_not_owned",
      "sem_model_v4.authoring.target_residual_indicator_required",
    ]);
    expect(diagnostics.every((item) => Boolean(item.corrective_action))).toBe(true);
  });

  it("allows disturbance covariance only between two endogenous constructs", () => {
    const nodes = [node("x"), node("y"), node("z"), node("w")];
    const covariance = newNativeScientificCovarianceEdgeV4("cov-y-w", "y", "w");
    const incompleteEdges = [path("x-y", "x", "y"), covariance];
    expect(validateNativeSemCovarianceChoiceV4(covariance, nodes, incompleteEdges, { kind: "disturbance_covariance" }))
      .toMatchObject([{ code: "sem_model_v4.authoring.disturbance_target_not_endogenous" }]);

    const completeEdges = [path("x-y", "x", "y"), path("z-w", "z", "w"), covariance];
    const result = confirmNativeSemCovarianceAuthoringV4(covariance, nodes, completeEdges, { kind: "disturbance_covariance" });
    expect(result.ok).toBe(true);
    if (!result.ok) return;
    expect(result.edge.data?.semModelV4).toEqual({
      version: 1,
      covariance: {
        kind: "scientific",
        origin: "explicit_conversion",
        left: { kind: "disturbance_of", id: "construct:y" },
        right: { kind: "disturbance_of", id: "construct:w" },
      },
    });
    expect(projectNativeSemCovarianceAuthoringV4(result.edge, nodes, completeEdges)).toMatchObject({
      choice: { kind: "disturbance_covariance" },
      source_is_endogenous: true,
      target_is_endogenous: true,
      diagnostics: [],
    });
  });

  it("normalizes symmetric residual endpoint order and produces deterministic signatures", () => {
    const nodes = [node("x"), node("y")];
    const covariance: Edge = {
      ...newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y"),
      data: {
        role: "covariance",
        semModelV4: {
          version: 1,
          covariance: {
            kind: "scientific",
            origin: "explicit_conversion",
            left: { kind: "residual_of", id: "observed:y2" },
            right: { kind: "residual_of", id: "observed:x1" },
          },
        },
      },
    };
    const projection = projectNativeSemCovarianceAuthoringV4(covariance, nodes, [covariance]);
    expect(projection.choice).toEqual({ kind: "residual_covariance", source_indicator: "x1", target_indicator: "y2" });
    expect(nativeSemCovarianceChoiceSignatureV4(projection.choice)).toBe("residual_covariance\0x1\0y2");
    expect(Object.isFrozen(projection)).toBe(true);
    expect(Object.isFrozen(projection.source_indicator_candidates)).toBe(true);
  });
});
