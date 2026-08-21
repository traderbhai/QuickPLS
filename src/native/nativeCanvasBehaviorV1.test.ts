import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { indicatorNodeId } from "../domain/diagramGraph";
import type { ConstructData } from "../types";
import {
  nativeCanvasSemanticZoomLevelV1,
  planNativeCanvasConnectionV1,
  projectNativeCanvasSemanticZoomV1,
} from "./nativeCanvasBehaviorV1";

const construct = (id: string): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  data: { label: id.toUpperCase(), shortName: id.toUpperCase(), mode: "reflective", indicators: [`${id}1`] },
});

describe("native Canvas semantic zoom and Connect planning", () => {
  it("uses stable far, medium, and near zoom boundaries", () => {
    expect(nativeCanvasSemanticZoomLevelV1(0.47)).toBe("far");
    expect(nativeCanvasSemanticZoomLevelV1(0.48)).toBe("medium");
    expect(nativeCanvasSemanticZoomLevelV1(0.77)).toBe("medium");
    expect(nativeCanvasSemanticZoomLevelV1(0.78)).toBe("near");
    expect(nativeCanvasSemanticZoomLevelV1(Number.NaN)).toBe("near");
  });

  it("suppresses only measurement detail at far/medium zoom and keeps scientific identities intact", () => {
    const x = construct("x");
    const y = construct("y");
    const indicator: Node<ConstructData> = {
      id: indicatorNodeId("x", "x1"),
      position: { x: -80, y: 0 },
      data: { label: "x1", shortName: "x1", mode: "reflective", indicators: [] },
    };
    const anchor: Node<Record<string, unknown>> = {
      id: "moderation-anchor::term%3Axw",
      position: { x: 80, y: 0 },
      data: {
        visualOnly: true,
        relationshipKind: "moderation_anchor",
        interactionNodeId: "generated:xw",
        interactionTermId: "term:xw",
        focalRelationId: "x-y",
        parentInteractionTermId: null,
        predictorId: "x",
        moderatorIds: ["w"],
        outcomeId: "y",
        order: 2,
        slotIndex: 0,
        slotCount: 1,
        fraction: 0.5,
        label: "Moderating effect",
      },
    };
    const nodes: Array<Node<Record<string, unknown>>> = [x, y, indicator, anchor];
    const edges: Edge[] = [
      { id: "x-y", source: "x", target: "y" },
      { id: "measurement::x::x1", source: "x", target: indicator.id },
      { id: "moderation-connector::term%3Axw::w", source: "w", target: anchor.id, data: { visualOnly: true } },
    ];
    const nodeIds = nodes.map((node) => node.id);
    const edgeIds = edges.map((edge) => edge.id);

    const medium = projectNativeCanvasSemanticZoomV1(nodes, edges, "medium");
    expect(medium.nodes.map((node) => node.id)).toEqual(nodeIds);
    expect(medium.edges.map((edge) => edge.id)).toEqual(edgeIds);
    expect(medium.nodes.find((node) => node.id === indicator.id)?.hidden).toBe(true);
    expect(medium.edges.find((edge) => edge.id.startsWith("measurement::"))?.hidden).toBe(true);
    expect(medium.nodes.find((node) => node.id === "x")?.data.semanticZoomLevel).toBe("medium");
    expect(medium.nodes.find((node) => node.id === anchor.id)?.data.semanticZoomLevel).toBeUndefined();
    expect(medium.edges.find((edge) => edge.id === "x-y")?.hidden).toBe(false);

    const near = projectNativeCanvasSemanticZoomV1(nodes, edges, "near");
    expect(near.nodes.find((node) => node.id === indicator.id)?.hidden).toBe(false);
    expect(near.edges.find((edge) => edge.id.startsWith("measurement::"))?.hidden).toBe(false);
  });

  it("isolates a selected construct with its indicators without mutating the input graph", () => {
    const indicatorId = indicatorNodeId("x", "x1");
    const nodes: Array<Node<Record<string, unknown>>> = [
      construct("x"),
      construct("y"),
      { id: indicatorId, position: { x: 0, y: 0 }, data: {} },
    ];
    const edges: Edge[] = [
      { id: "x-y", source: "x", target: "y" },
      { id: "measurement::x::x1", source: "x", target: indicatorId },
    ];
    const original = structuredClone({ nodes, edges });

    const focused = projectNativeCanvasSemanticZoomV1(nodes, edges, "near", new Set(["x", indicatorId]));
    expect(focused.nodes.find((node) => node.id === "x")?.hidden).toBe(false);
    expect(focused.nodes.find((node) => node.id === indicatorId)?.hidden).toBe(false);
    expect(focused.nodes.find((node) => node.id === "y")?.hidden).toBe(true);
    expect(focused.edges.find((edge) => edge.id === "x-y")?.hidden).toBe(true);
    expect({ nodes, edges }).toEqual(original);
  });

  it("routes construct, focal-path, and two-way-anchor targets without persisting a visual endpoint", () => {
    expect(planNativeCanvasConnectionV1({
      sourceConstructId: "x",
      target: { kind: "construct", constructId: "y" },
      relationId: "relation:x-y",
    })).toEqual({
      status: "ready",
      operation: "structural_path",
      command: { kind: "add_path", relationId: "relation:x-y", sourceId: "x", targetId: "y", label: "Path" },
    });

    expect(planNativeCanvasConnectionV1({
      sourceConstructId: "w",
      target: { kind: "focal_relation", relationId: "relation:x-y" },
    })).toEqual({
      status: "ready",
      operation: "moderating_effect",
      request: {
        action: "create",
        target: { kind: "focal_relation", relationId: "relation:x-y" },
        moderatorId: "w",
        origin: "drag",
      },
    });

    const anchorPlan = planNativeCanvasConnectionV1({
      sourceConstructId: "z",
      target: {
        kind: "moderation_anchor",
        visualNodeId: "moderation-anchor::term%3Axw",
        interactionTermId: "term:xw",
        order: 2,
      },
    });
    expect(anchorPlan).toEqual({
      status: "ready",
      operation: "moderating_effect",
      request: {
        action: "create",
        target: { kind: "parent_interaction", interactionTermId: "term:xw" },
        moderatorId: "z",
        origin: "drag",
      },
    });
    expect(JSON.stringify(anchorPlan)).not.toContain("moderation-anchor::");
  });

  it("keeps invalid and fourth-order Connect gestures mutation-free", () => {
    expect(planNativeCanvasConnectionV1({
      sourceConstructId: "x",
      target: { kind: "construct", constructId: "x" },
      relationId: "relation:x-x",
    })).toMatchObject({ status: "blocked", code: "self_path" });
    expect(planNativeCanvasConnectionV1({
      sourceConstructId: "x",
      target: { kind: "construct", constructId: "y" },
      relationId: "relation:x-y-duplicate",
      structuralPathExists: true,
    })).toMatchObject({ status: "blocked", code: "duplicate_path" });
    expect(planNativeCanvasConnectionV1({
      sourceConstructId: "q",
      target: {
        kind: "moderation_anchor",
        visualNodeId: "moderation-anchor::term%3Axwz",
        interactionTermId: "term:xwz",
        order: 3,
      },
    })).toMatchObject({ status: "blocked", code: "fourth_order" });
  });
});
