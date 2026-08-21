import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { buildDiagramGraph, defaultDiagramLayout, indicatorNodeId } from "./diagramGraph";
import {
  isModerationAnchorData,
  isModerationConnectorData,
} from "./moderationDiagramProjectionV1";

const measured = (id: string, label: string, y: number): Node<ConstructData> => ({
  id,
  position: { x: id === "y" ? 440 : 40, y },
  data: { label, shortName: id.toUpperCase(), mode: "reflective", indicators: [`${id}1`] },
});

const interaction = (
  id: string,
  termId: string,
  operands: [string, string, ...string[]],
): Node<ConstructData> => ({
  id,
  position: { x: 240, y: 180 },
  data: {
    label: "Generated interaction",
    shortName: "INT",
    mode: "formative",
    indicators: [],
    semantic: "interaction",
    interaction: {
      kind: "interaction_v2",
      termId,
      operands,
      focalRelationId: "relation:x-y",
      outcome: "y",
      canonicalMethod: "two_stage",
      hierarchyPolicy: "strong",
    },
  },
});

describe("QuickPLS 2.54 diagram-native moderation projection", () => {
  it("honors explicit Above indicator placement in compact mode", () => {
    const nodes: Array<Node<ConstructData>> = [{
      id: "quality",
      position: { x: 240, y: 260 },
      data: { label: "Quality", shortName: "QUAL", mode: "reflective", indicators: ["q1", "q2"] },
    }];
    const layout = defaultDiagramLayout(nodes, []);
    layout.indicatorLayouts.quality.q1 = { side: "top", order: 0, pinned: true };
    layout.indicatorLayouts.quality.q2 = { side: "top", order: 1, pinned: true };

    const graph = buildDiagramGraph(nodes, [], "compact", "model", undefined, { layout });
    const indicators = ["q1", "q2"].map((column) => graph.nodes.find((node) => node.id === indicatorNodeId("quality", column))!);

    expect(indicators.every((node) => node.position.y < nodes[0]!.position.y)).toBe(true);
    expect(indicators[0]!.position.x).toBeLessThan(indicators[1]!.position.x);
  });

  it("projects a connectable two-way anchor and dashed connector without changing the scientific graph", () => {
    const nodes = [
      measured("x", "Motivation", 40),
      measured("w", "Gender", 280),
      measured("y", "Performance", 40),
      interaction("generated:xw", "term:xw", ["x", "w"]),
    ];
    const edges: Edge[] = [
      { id: "relation:x-y", source: "x", target: "y" },
      { id: "generated:xw-y", source: "generated:xw", target: "y", data: { technicalGenerated: true } },
    ];
    const original = structuredClone({ nodes, edges });

    const graph = buildDiagramGraph(nodes, edges, "sem", "model");
    const anchor = graph.nodes.find((node) => isModerationAnchorData(node.data));
    const connector = graph.edges.find((edge) => isModerationConnectorData(edge.data));

    expect(anchor).toMatchObject({
      connectable: true,
      deletable: false,
      data: {
        visualOnly: true,
        interactionTermId: "term:xw",
        focalRelationId: "relation:x-y",
        predictorId: "x",
        moderatorIds: ["w"],
        outcomeId: "y",
        order: 2,
      },
    });
    expect(connector).toMatchObject({
      source: "w",
      target: anchor?.id,
      deletable: false,
      reconnectable: false,
      data: {
        visualOnly: true,
        relationshipKind: "moderation_connector",
        interactionTermId: "term:xw",
      },
    });
    expect(graph.nodes.some((node) => node.id === "generated:xw")).toBe(false);
    expect(graph.edges.find((edge) => edge.id === "relation:x-y")).toMatchObject({ source: "x", target: "y" });
    expect(graph.edges
      .filter((edge) => edge.target === anchor?.id)
      .every((edge) => edge.data?.visualOnly === true)).toBe(true);
    expect({ nodes, edges }).toEqual(original);
  });

  it("projects one non-connectable three-way anchor instead of exposing lower-order closure terms", () => {
    const nodes = [
      measured("x", "Predictor", 20),
      measured("w", "First moderator", 220),
      measured("z", "Second moderator", 380),
      measured("y", "Outcome", 20),
      interaction("generated:xw", "term:xw", ["x", "w"]),
      interaction("generated:xwz", "term:xwz", ["x", "w", "z"]),
    ];
    const graph = buildDiagramGraph(nodes, [{ id: "relation:x-y", source: "x", target: "y" }], "sem", "model");
    const anchors = graph.nodes.filter((node) => isModerationAnchorData(node.data));

    expect(anchors).toHaveLength(1);
    expect(anchors[0]).toMatchObject({
      connectable: false,
      data: {
        visualOnly: true,
        interactionTermId: "term:xwz",
        parentInteractionTermId: "term:xw",
        moderatorIds: ["w", "z"],
        order: 3,
      },
    });
  });
});
