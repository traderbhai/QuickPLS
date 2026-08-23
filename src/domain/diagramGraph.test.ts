import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import {
  buildDiagramGraph,
  defaultDiagramLayout,
  indicatorNodeId,
  layoutSmartplsModel,
  measurementConnectorEdgeId,
  modelFingerprint,
  parseIndicatorNodeId,
  parseMeasurementConnectorEdgeId,
} from "./diagramGraph";
import { polylineMidpoint, renderedEdgePolyline, semRectsOverlap } from "./semGeometry";
import type { AnalysisRun, ConstructData, DiagramMode, EdgeRouteStyle, PlsResult } from "../types";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", type: "construct", position: { x: 200, y: 100 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", type: "construct", position: { x: 500, y: 100 }, data: { label: "Outcome", shortName: "Y", mode: "formative", indicators: ["y1"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y", label: "Path" }];
const result: PlsResult = {
  method_version: "pls_pm_v1",
  converged: true,
  iterations: 3,
  used_observations: 20,
  omitted_observations: 0,
  outer_estimates: [
    { construct: "x", indicator: "x1", weight: 0.5, loading: 0.91 },
    { construct: "x", indicator: "x2", weight: 0.5, loading: 0.82 },
    { construct: "y", indicator: "y1", weight: 0.67, loading: 0.75 },
  ],
  paths: [{ source: "x", target: "y", coefficient: 0.4567 }],
  effects: [],
  r_squared: { y: 0.208 },
  warnings: [],
};
const run: AnalysisRun = { id: "run-1", name: "PLS run", method: "PLS-SEM", createdAt: "2026-07-19T00:00:00.000Z", seed: 1, status: "completed", warnings: [], fingerprint: "fixture", result };

describe("SEM diagram graph", () => {
  it.each([
    ["construct::quality", "qual::1"],
    ["construct with spaces", "indicator with spaces"],
    ["construct%encoded", "indicator%value"],
  ])("round-trips collision-prone derived connector identities (%s, %s)", (constructId, indicator) => {
    const indicatorId = indicatorNodeId(constructId, indicator);
    const connectorId = measurementConnectorEdgeId(constructId, indicator);
    expect(parseIndicatorNodeId(indicatorId)).toEqual({ constructId, indicator });
    expect(parseMeasurementConnectorEdgeId(connectorId)).toEqual({ constructId, indicator });
    expect(indicatorId.split("::")).toHaveLength(3);
    expect(connectorId.split("::")).toHaveLength(3);
  });

  it("keeps derived IDs distinct when delimiters move between the construct and indicator", () => {
    expect(indicatorNodeId("a::b", "c")).not.toBe(indicatorNodeId("a", "b::c"));
    expect(measurementConnectorEdgeId("a::b", "c")).not.toBe(measurementConnectorEdgeId("a", "b::c"));
  });

  it("derives visible latent and indicator nodes without changing model ids", () => {
    const graph = buildDiagramGraph(nodes, edges, "sem", "model");
    expect(graph.nodes.filter((node) => node.type === "latent")).toHaveLength(2);
    expect(graph.nodes.filter((node) => node.type === "indicator")).toHaveLength(3);
    expect(graph.edges.filter((edge) => edge.id.startsWith("measurement::"))).toHaveLength(3);
    expect(graph.edges.find((edge) => edge.id === "x-y")?.data).toMatchObject({ perimeterRouting: "continuous" });
    expect(graph.edges.filter((edge) => edge.id.startsWith("measurement::"))
      .every((edge) => edge.data?.perimeterRouting === "continuous")).toBe(true);
  });

  it("expands a presentation-only mediation relation chain to its intermediate nodes", () => {
    const mediationNodes: Array<Node<ConstructData>> = [
      { id: "x", type: "construct", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: [] } },
      { id: "m", type: "construct", position: { x: 250, y: 0 }, data: { label: "Mediator", shortName: "M", mode: "reflective", indicators: [] } },
      { id: "y", type: "construct", position: { x: 500, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: [] } },
    ];
    const mediationEdges: Edge[] = [
      { id: "relation:x_m", source: "x", target: "m" },
      { id: "relation:m_y", source: "m", target: "y" },
    ];
    const before = structuredClone({ mediationNodes, mediationEdges });

    const graph = buildDiagramGraph(mediationNodes, mediationEdges, "smartpls_result", "model", undefined, {
      resultOverlay: {
        kind: "mediation",
        nodeIds: ["x", "y"],
        relationIds: ["relation:x_m", "relation:m_y"],
        interactionTermIds: [],
        label: "Indirect path",
      },
    });

    expect(graph.nodes.filter((node) => ["x", "m", "y"].includes(node.id))
      .every((node) => node.className?.includes("result-overlay-highlight"))).toBe(true);
    expect(graph.edges.filter((edge) => mediationEdges.some((source) => source.id === edge.id))
      .every((edge) => edge.className?.includes("result-overlay-edge-highlight"))).toBe(true);
    expect({ mediationNodes, mediationEdges }).toEqual(before);
  });

  it("projects selected HOC membership as arrowless non-persisted presentation edges", () => {
    const hocNodes: Array<Node<ConstructData>> = [
      { id: "a", type: "construct", position: { x: 100, y: 40 }, data: { label: "Component A", shortName: "A", mode: "reflective", indicators: ["a1"] } },
      { id: "b", type: "construct", position: { x: 100, y: 180 }, data: { label: "Component B", shortName: "B", mode: "reflective", indicators: ["b1"] } },
      {
        id: "hoc",
        type: "construct",
        position: { x: 380, y: 110 },
        data: {
          label: "Organizational strength",
          shortName: "HOC",
          mode: "reflective",
          indicators: [],
          semantic: "higher_order",
          higherOrder: {
            id: "term:hoc",
            components: ["a", "b"],
            method: "two_stage",
            canonicalApproach: "disjoint_two_stage",
            measurementType: "reflective_reflective",
          },
        },
      },
    ];
    const modelEdges: Edge[] = [];
    const before = structuredClone(modelEdges);

    const unselected = buildDiagramGraph(hocNodes, modelEdges, "sem", "model");
    const selected = buildDiagramGraph(hocNodes, modelEdges, "sem", "model", undefined, { selectedHigherOrderId: "hoc" });
    const membership = selected.edges.filter((edge) => edge.id.startsWith("hoc-membership::"));

    expect(unselected.edges.some((edge) => edge.id.startsWith("hoc-membership::"))).toBe(false);
    expect(membership).toHaveLength(2);
    expect(membership.map((edge) => [edge.source, edge.target])).toEqual([["a", "hoc"], ["b", "hoc"]]);
    expect(selected.nodes.filter((node) => ["a", "b"].includes(node.id)).every((node) => node.className?.includes("hoc-component-highlight"))).toBe(true);
    for (const edge of membership) {
      expect(edge).toMatchObject({ selectable: false, deletable: false, focusable: false, reconnectable: false });
      expect(edge.markerStart).toBeUndefined();
      expect(edge.markerEnd).toBeUndefined();
      expect(edge.style?.strokeDasharray).toBe("5 4");
      expect(edge.data).toMatchObject({ visualOnly: true, relationshipKind: "higher_order_membership" });
    }
    expect(modelEdges).toEqual(before);
  });

  it("keeps two-stage HOC result overlays compatible without authoring generated score indicators", () => {
    const hocNodes: Array<Node<ConstructData>> = [
      { id: "op", type: "construct", position: { x: 20, y: 100 }, data: { label: "Prestige", shortName: "OP", mode: "reflective", indicators: ["op1"] } },
      { id: "oi", type: "construct", position: { x: 220, y: 100 }, data: { label: "Identification", shortName: "OI", mode: "reflective", indicators: ["oi1"] } },
      { id: "acj", type: "construct", position: { x: 220, y: 260 }, data: { label: "Joy", shortName: "ACJ", mode: "reflective", indicators: ["joy1"] } },
      { id: "acl", type: "construct", position: { x: 220, y: 400 }, data: { label: "Love", shortName: "ACL", mode: "reflective", indicators: ["love1"] } },
      {
        id: "ac",
        type: "construct",
        position: { x: 480, y: 160 },
        data: {
          label: "Affective commitment",
          shortName: "AC",
          mode: "reflective",
          indicators: [],
          semantic: "higher_order",
          higherOrder: {
            id: "term:ac",
            components: ["acj", "acl"],
            method: "two_stage",
            canonicalApproach: "disjoint_two_stage",
            measurementType: "reflective_reflective",
          },
        },
      },
    ];
    const hocEdges: Edge[] = [
      { id: "op-oi", source: "op", target: "oi" },
      { id: "oi-ac", source: "oi", target: "ac" },
      { id: "op-ac", source: "op", target: "ac" },
    ];
    const hocResult: PlsResult = {
      ...result,
      outer_estimates: [
        { construct: "op", indicator: "op1", weight: 0.7, loading: 0.82 },
        { construct: "oi", indicator: "oi1", weight: 0.8, loading: 0.86 },
        { construct: "acj", indicator: "joy1", weight: 0.6, loading: 0.81 },
        { construct: "acl", indicator: "love1", weight: 0.5, loading: 0.79 },
        { construct: "ac", indicator: "__qpls_hoc_ac_acj", weight: 0.71, loading: 0.916 },
        { construct: "ac", indicator: "__qpls_hoc_ac_acl", weight: -0.45, loading: -0.774 },
      ],
      paths: [
        { source: "op", target: "oi", coefficient: 0.361 },
        { source: "oi", target: "ac", coefficient: 0.553 },
        { source: "op", target: "ac", coefficient: 0.169 },
      ],
      r_squared: { oi: 0.131, ac: 0.403 },
    };
    const hocRun: AnalysisRun = { ...run, id: "hoc-run", result: hocResult };

    const graph = buildDiagramGraph(hocNodes, hocEdges, "smartpls_result", "paths_r2", hocRun);

    expect(graph.compatible).toBe(true);
    expect(graph.edges.find((edge) => edge.id === "oi-ac")?.label).toBe("0.553");
    expect(graph.nodes.find((node) => node.id === "ac")?.data.resultR2).toBe(0.403);
    expect(graph.edges.some((edge) => edge.id.includes("__qpls_hoc_"))).toBe(false);

    const stale = buildDiagramGraph(hocNodes, hocEdges, "smartpls_result", "paths_r2", {
      ...hocRun,
      result: {
        ...hocResult,
        outer_estimates: [
          ...hocResult.outer_estimates,
          { construct: "ac", indicator: "__qpls_hoc_ac_unknown", weight: 0.1, loading: 0.1 },
        ],
      },
    });
    expect(stale.compatible).toBe(false);
  });

  it("hides generated interaction constructs and projects visual-only path anchors", () => {
    const moderationNodes: Array<Node<ConstructData>> = [
      ...nodes,
      { id: "w", type: "construct", position: { x: 350, y: -80 }, data: { label: "Moderator", shortName: "W", mode: "reflective", indicators: ["w1"] } },
      {
        id: "xw",
        type: "construct",
        position: { x: 360, y: 240 },
        data: {
          label: "X × W",
          shortName: "XW",
          mode: "formative",
          indicators: [],
          semantic: "interaction",
          interaction: {
            kind: "interaction_v2",
            termId: "term:xw",
            operands: ["x", "w"],
            focalRelationId: "x-y",
            outcome: "y",
            canonicalMethod: "two_stage",
            hierarchyPolicy: "strong",
          },
        },
      },
    ];
    const moderationEdges: Edge[] = [
      ...edges,
      { id: "w-y", source: "w", target: "y", label: "Main effect" },
      { id: "xw-y", source: "xw", target: "y", label: "Interaction" },
    ];
    const beforeNodes = structuredClone(moderationNodes);
    const beforeEdges = structuredClone(moderationEdges);

    const graph = buildDiagramGraph(moderationNodes, moderationEdges, "sem", "model");
    const anchor = graph.nodes.find((node) => node.type === "moderationAnchor");
    const connector = graph.edges.find((edge) => edge.data?.relationshipKind === "moderation_connector");

    expect(graph.nodes.some((node) => node.id === "xw")).toBe(false);
    expect(graph.edges.some((edge) => edge.id === "xw-y")).toBe(false);
    expect(anchor?.data).toMatchObject({
      visualOnly: true,
      interactionTermId: "term:xw",
      focalRelationId: "x-y",
      moderatorIds: ["w"],
      order: 2,
    });
    expect(connector).toMatchObject({ source: "w", target: anchor?.id, deletable: false, reconnectable: false });
    expect(connector?.data).toMatchObject({ visualOnly: true, relationshipKind: "moderation_connector" });
    expect(graph.edges.find((edge) => edge.id === "x-y")?.className).toContain("moderated-focal-edge");
    expect(moderationNodes).toEqual(beforeNodes);
    expect(moderationEdges).toEqual(beforeEdges);

    const layout = defaultDiagramLayout(moderationNodes, moderationEdges);
    layout.moderationAnchorFractions = { "term:xw": 0.7 };
    const connectorId = connector!.id;
    layout.moderationConnectorBendPoints = { [connectorId]: [{ x: 85, y: 64 }] };
    const restored = buildDiagramGraph(moderationNodes, moderationEdges, "sem", "model", undefined, { layout });
    expect(restored.nodes.find((node) => node.type === "moderationAnchor")?.data.fraction).toBe(0.7);
    expect(restored.edges.find((edge) => edge.id === connectorId)?.data).toMatchObject({
      routing: "polyline",
      bendPoints: [{ x: 85, y: 64 }],
    });

    const expert = buildDiagramGraph(moderationNodes, moderationEdges, "sem", "model", undefined, { showGeneratedInteractionTerms: true });
    expect(expert.nodes.some((node) => node.id === "xw")).toBe(true);
    expect(expert.edges.some((edge) => edge.id === "xw-y")).toBe(true);
  });

  it("orients reflective and formative measurement arrows differently", () => {
    const graph = buildDiagramGraph(nodes, edges, "sem", "model");
    const reflective = graph.edges.find((edge) => edge.id === "measurement::x::x1")!;
    const formative = graph.edges.find((edge) => edge.id === "measurement::y::y1")!;
    expect(reflective.source).toBe("x");
    expect(reflective.target).toBe(indicatorNodeId("x", "x1"));
    expect(formative.source).toBe(indicatorNodeId("y", "y1"));
    expect(formative.target).toBe("y");
    expect(reflective.data).toMatchObject({
      visualOnly: true,
      relationshipKind: "measurement_connector",
      constructId: "x",
      indicator: "x1",
      routeEditable: true,
      perimeterRouting: "continuous",
      routing: "straight",
    });
    expect(formative.data).toMatchObject({
      visualOnly: true,
      relationshipKind: "measurement_connector",
      constructId: "y",
      indicator: "y1",
      routeEditable: true,
      perimeterRouting: "continuous",
      routing: "straight",
    });
    expect(reflective).toMatchObject({ selectable: true, focusable: true, deletable: false, reconnectable: false });
    expect(formative).toMatchObject({ selectable: true, focusable: true, deletable: false, reconnectable: false });
  });

  it("maps every saved measurement connector style without changing scientific arrow direction", () => {
    const routes: Array<[EdgeRouteStyle, string]> = [
      ["straight", "straight"],
      ["curved", "default"],
      ["orthogonal", "smoothstep"],
      ["polyline", "polyline"],
    ];
    for (const [savedStyle, renderedStyle] of routes) {
      const layout = defaultDiagramLayout(nodes, edges);
      const bendPoints = [{ x: 314, y: 72 }, { x: 392, y: 154 }];
      layout.measurementConnectorLayouts = {
        x: { x1: { routing: savedStyle, ...(savedStyle === "polyline" ? { bendPoints } : {}) } },
        y: { y1: { routing: savedStyle, ...(savedStyle === "polyline" ? { bendPoints } : {}) } },
      };
      const graph = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
      const reflective = graph.edges.find((edge) => edge.id === "measurement::x::x1")!;
      const formative = graph.edges.find((edge) => edge.id === "measurement::y::y1")!;
      expect(reflective.data?.routing).toBe(renderedStyle);
      expect(formative.data?.routing).toBe(renderedStyle);
      expect(reflective.data?.bendPoints).toEqual(savedStyle === "polyline" ? bendPoints : undefined);
      expect(formative.data?.bendPoints).toEqual(savedStyle === "polyline" ? bendPoints : undefined);
      expect([reflective.source, reflective.target]).toEqual(["x", indicatorNodeId("x", "x1")]);
      expect([formative.source, formative.target]).toEqual([indicatorNodeId("y", "y1"), "y"]);
    }
  });

  it("keeps reflective loadings and formative weights on every measurement route", () => {
    const routes: EdgeRouteStyle[] = ["straight", "curved", "orthogonal", "polyline"];
    const surfaces: DiagramMode[] = ["sem", "smartpls_result", "publication"];
    for (const routing of routes) {
      const layout = defaultDiagramLayout(nodes, edges);
      layout.measurementConnectorLayouts = {
        x: { x1: { routing, ...(routing === "polyline" ? { bendPoints: [{ x: 314, y: 72 }] } : {}) } },
        y: { y1: { routing, ...(routing === "polyline" ? { bendPoints: [{ x: 430, y: 176 }] } : {}) } },
      };

      for (const surface of surfaces) {
        const graph = buildDiagramGraph(nodes, edges, surface, "model", run, { layout });
        const reflective = graph.edges.find((edge) => edge.id === measurementConnectorEdgeId("x", "x1"))!;
        const formative = graph.edges.find((edge) => edge.id === measurementConnectorEdgeId("y", "y1"))!;

        expect(reflective.label).toBe("0.910");
        expect(formative.label).toBe("0.670");
        expect(reflective.data?.labelOffset).toBeUndefined();
        expect(formative.data?.labelOffset).toBeUndefined();
        const structuralOffset = graph.edges.find((edge) => edge.id === "x-y")?.data?.labelOffset;
        expect(structuralOffset).toEqual(expect.objectContaining({ x: expect.any(Number), y: expect.any(Number) }));
        expect(structuralOffset).not.toEqual({ x: 0, y: 0 });
      }
    }
  });

  it("reserves centered measurement badges when allocating later covariance labels", () => {
    const covarianceEdges: Edge[] = [
      ...edges,
      { id: "x-y-covariance", source: "x", target: "y", label: "Covariance", data: { role: "covariance" } },
    ];
    const layout = defaultDiagramLayout(nodes, covarianceEdges);
    layout.indicatorLayouts.x.x1 = { side: "free", x: 500, y: 120, order: 0, pinned: true };

    const graph = buildDiagramGraph(nodes, covarianceEdges, "sem", "model", run, { layout });
    const measurement = graph.edges.find((edge) => edge.id === measurementConnectorEdgeId("x", "x1"))!;
    const covariance = graph.edges.find((edge) => edge.id === "x-y-covariance")!;
    const labelRect = (edge: Edge) => {
      const source = graph.nodes.find((node) => node.id === edge.source)!;
      const target = graph.nodes.find((node) => node.id === edge.target)!;
      const mid = polylineMidpoint(renderedEdgePolyline(edge, source, target));
      const label = String(edge.label ?? "");
      const width = Math.min(190, Math.max(34, label.length * 7 + 14));
      const offset = edge.data?.labelOffset as { x?: number; y?: number } | undefined;
      return {
        x: mid.x + Number(offset?.x ?? 0) - width / 2,
        y: mid.y + Number(offset?.y ?? 0) - 10,
        width,
        height: 20,
      };
    };

    expect(measurement.data?.labelOffset).toBeUndefined();
    expect(covariance.data?.labelOffset).toEqual(expect.objectContaining({ x: expect.any(Number), y: expect.any(Number) }));
    expect(semRectsOverlap(labelRect(measurement), labelRect(covariance))).toBe(false);
  });

  it("creates one deterministic editable bend for a polyline connector without saved points", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    layout.measurementConnectorLayouts = { x: { x1: { routing: "polyline" } } };
    const first = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
    const second = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
    const firstBends = first.edges.find((edge) => edge.id === "measurement::x::x1")?.data?.bendPoints;
    const secondBends = second.edges.find((edge) => edge.id === "measurement::x::x1")?.data?.bendPoints;
    expect(firstBends).toEqual([expect.objectContaining({ x: expect.any(Number), y: expect.any(Number) })]);
    expect(secondBends).toEqual(firstBends);
  });

  it("regenerates measurement polyline bends inside tidy-publication coordinates", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    const farCanvasBend = [{ x: 100_000, y: -100_000 }];
    layout.measurementConnectorLayouts = { x: { x1: { routing: "polyline", bendPoints: farCanvasBend } } };

    const current = buildDiagramGraph(nodes, edges, "smartpls_result", "model", run, {
      layout,
      layoutSource: "current_canvas",
    });
    expect(current.edges.find((edge) => edge.id === measurementConnectorEdgeId("x", "x1"))?.data?.bendPoints)
      .toEqual(farCanvasBend);

    const tidy = buildDiagramGraph(nodes, edges, "smartpls_result", "model", run, {
      layout,
      layoutSource: "tidy_publication",
    });
    const connector = tidy.edges.find((edge) => edge.id === measurementConnectorEdgeId("x", "x1"))!;
    const source = tidy.nodes.find((node) => node.id === connector.source)!;
    const target = tidy.nodes.find((node) => node.id === connector.target)!;
    const route = renderedEdgePolyline(connector, source, target);
    expect(connector.data?.routing).toBe("polyline");
    expect(connector.data?.bendPoints).not.toEqual(farCanvasBend);
    expect(Math.max(...route.map((point) => Math.abs(point.x)))).toBeLessThan(2_000);
    expect(Math.max(...route.map((point) => Math.abs(point.y)))).toBeLessThan(2_000);
  });

  it("selects measurement connectors only while the editor layout is editable", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    const selectedMeasurementConnector = { constructId: "x", indicator: "x1" };
    const editor = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout, selectedMeasurementConnector });
    expect(editor.edges.find((edge) => edge.id === "measurement::x::x1")).toMatchObject({
      selected: true,
      selectable: true,
      focusable: true,
      data: { routeEditable: true },
    });

    const lockedLayout = { ...layout, layoutLocked: true };
    const locked = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout: lockedLayout, selectedMeasurementConnector });
    expect(locked.edges.find((edge) => edge.id === "measurement::x::x1")).toMatchObject({
      selected: false,
      selectable: false,
      focusable: false,
      data: { routeEditable: false },
    });

    const publicationPending = buildDiagramGraph(nodes, edges, "sem", "model", undefined, {
      layout,
      selectedMeasurementConnector,
      layoutEditingEnabled: false,
    });
    expect(publicationPending.edges.find((edge) => edge.id === "measurement::x::x1")).toMatchObject({
      selected: false,
      selectable: false,
      focusable: false,
      data: { routeEditable: false },
    });

    const results = buildDiagramGraph(nodes, edges, "smartpls_result", "model", run, { layout, selectedMeasurementConnector });
    expect(results.edges.find((edge) => edge.id === "measurement::x::x1")).toMatchObject({
      selected: false,
      selectable: false,
      focusable: false,
      data: { routeEditable: false },
    });

    const publication = buildDiagramGraph(nodes, edges, "publication", "model", run, { layout, selectedMeasurementConnector });
    expect(publication.edges.find((edge) => edge.id === "measurement::x::x1")).toMatchObject({
      selected: false,
      selectable: false,
      focusable: false,
      data: { routeEditable: false },
    });
  });

  it("projects explicit structural editability for editor and every locked surface", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    const editor = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
    expect(editor.edges.find((edge) => edge.id === "x-y")).toMatchObject({
      selectable: true,
      focusable: true,
      reconnectable: true,
      deletable: true,
      data: { relationshipEditable: true },
    });

    const pending = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout, layoutEditingEnabled: false });
    expect(pending.edges.find((edge) => edge.id === "x-y")).toMatchObject({
      selectable: false,
      focusable: false,
      reconnectable: false,
      deletable: false,
      data: { relationshipEditable: false },
    });

    const locked = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout: { ...layout, layoutLocked: true } });
    expect(locked.edges.find((edge) => edge.id === "x-y")?.data).toMatchObject({ relationshipEditable: false });

    const results = buildDiagramGraph(nodes, edges, "smartpls_result", "model", run, { layout });
    expect(results.edges.find((edge) => edge.id === "x-y")?.data).toMatchObject({ relationshipEditable: false });

    const otherScientificRelationships: Edge[] = [
      { id: "control:x-y", source: "x", target: "y", data: { role: "control" } },
      { id: "covariance:x-y", source: "x", target: "y", data: { role: "covariance" } },
    ];
    const lockedRelationships = buildDiagramGraph(nodes, otherScientificRelationships, "sem", "model", undefined, {
      layout: defaultDiagramLayout(nodes, otherScientificRelationships),
      layoutEditingEnabled: false,
    });
    for (const id of otherScientificRelationships.map((edge) => edge.id)) {
      expect(lockedRelationships.edges.find((edge) => edge.id === id)).toMatchObject({
        selectable: false,
        focusable: false,
        reconnectable: false,
        deletable: false,
        data: { relationshipEditable: false },
      });
    }
  });

  it("shows numeric overlays only for compatible selected runs", () => {
    const graph = buildDiagramGraph(nodes, edges, "sem", "paths_r2", run);
    expect(graph.compatible).toBe(true);
    expect(graph.edges.find((edge) => edge.id === "x-y")?.label).toBe("0.457");
    expect(graph.nodes.find((node) => node.id === "y")?.data.resultR2).toBe(0.208);
    const stale = buildDiagramGraph(nodes, [{ id: "y-x", source: "y", target: "x" }], "sem", "paths_r2", run);
    expect(stale.compatible).toBe(false);
    expect(stale.diagnostic).toContain("Selected run does not match");
  });

  it("uses PLSc corrected outer loadings for result overlays without changing corrected common paths or R-square", () => {
    const reflectiveNodes = nodes.map((node) => ({
      ...node,
      data: { ...node.data, mode: "reflective" as const },
    }));
    const correctedOuterLoadings = result.outer_estimates.map((row, index) => ({
      ...row,
      loading: 0.991 - index * 0.01,
      weight: 0.451 + index * 0.01,
    }));
    const plscRun: AnalysisRun = {
      ...run,
      id: "plsc-run",
      name: "Consistent PLS run",
      method: "Consistent PLS",
      result: {
        ...result,
        method_version: "plsc_v2",
        paths: [{ source: "x", target: "y", coefficient: 0.6543 }],
        r_squared: { y: 0.4281 },
        plsc: {
          method_version: "plsc_v2",
          reliability_method_version: "dijkstra_henseler_rho_a_v1",
          tolerance: 1e-12,
          reliabilities: [
            { construct: "x", rho_a: 0.9 },
            { construct: "y", rho_a: 0.88 },
          ],
          construct_correlations: [{ left: "x", right: "y", original: 0.6, corrected: 0.7 }],
          corrected_paths: [{ source: "x", target: "y", coefficient: 0.6543 }],
          corrected_outer_loadings: correctedOuterLoadings,
          corrected_r_squared: { y: 0.4281 },
          warnings: [],
        },
      },
    };

    const graph = buildDiagramGraph(reflectiveNodes, edges, "smartpls_result", "paths_r2", plscRun);
    expect(graph.compatible).toBe(true);
    expect(graph.edges.find((edge) => edge.id === "measurement::x::x1")?.label).toBe("0.991");
    expect(graph.edges.find((edge) => edge.id === "measurement::x::x2")?.label).toBe("0.981");
    expect(graph.edges.find((edge) => edge.id === "x-y")?.label).toBe("0.654");
    expect(graph.nodes.find((node) => node.id === "y")?.data.resultR2).toBe(0.4281);
  });

  it("keeps structural path labels visible in editable academic mode before results", () => {
    const graph = buildDiagramGraph(nodes, edges, "sem", "model");
    expect(graph.edges.find((edge) => edge.id === "x-y")?.label).toBe("Path");
    expect(graph.edges.find((edge) => edge.id === "x-y")?.data?.routing).toBe("straight");
    const locked = buildDiagramGraph(nodes, edges, "publication", "model");
    expect(locked.edges.find((edge) => edge.id === "x-y")?.label).toBe("");
  });

  it("normalizes legacy bent construct paths to straight academic routes unless explicitly pinned", () => {
    const legacyBent: Edge[] = [{ id: "x-y", source: "x", target: "y", label: "Path", type: "smoothstep" }];
    const graph = buildDiagramGraph(nodes, legacyBent, "sem", "model");
    expect(graph.edges.find((edge) => edge.id === "x-y")?.data?.routing).toBe("straight");

    const layout = defaultDiagramLayout(nodes, legacyBent);
    layout.edgeLayouts["x-y"] = { routing: "orthogonal", pinned: true };
    const pinned = buildDiagramGraph(nodes, legacyBent, "sem", "model", undefined, { layout });
    expect(pinned.edges.find((edge) => edge.id === "x-y")?.data?.routing).toBe("smoothstep");

    layout.edgeLayouts["x-y"] = { routing: "curved", pinned: true };
    const curved = buildDiagramGraph(nodes, legacyBent, "sem", "model", undefined, { layout });
    expect(curved.edges.find((edge) => edge.id === "x-y")?.data).toMatchObject({
      perimeterRouting: "continuous",
      routing: "default",
    });
  });

  it("keeps the editable SEM canvas tied to manual node positions", () => {
    const moved = nodes.map((node) => node.id === "x" ? { ...node, position: { x: 720, y: 310 } } : node);
    const graph = buildDiagramGraph(moved, edges, "sem", "model");
    const predictor = graph.nodes.find((node) => node.id === "x")!;
    const predictorIndicator = graph.nodes.find((node) => node.id === indicatorNodeId("x", "x1"))!;
    expect(predictor.position).toEqual({ x: 720, y: 310 });
    expect(predictor.draggable).toBe(true);
    expect(predictorIndicator.position.x).toBeLessThan(predictor.position.x);
  });

  it("applies SmartPLS arrangement only when explicitly requested", () => {
    const arranged = layoutSmartplsModel(nodes, edges);
    expect(arranged.find((node) => node.id === "x")?.position.x).toBeLessThan(arranged.find((node) => node.id === "y")!.position.x);
    expect(arranged.find((node) => node.id === "x")?.position).not.toEqual(nodes.find((node) => node.id === "x")?.position);
  });

  it("reserves complete indicator envelopes and a structural lane during automatic arrangement", () => {
    const envelopeNodes: Array<Node<ConstructData>> = [
      { id: "left", type: "construct", position: { x: 0, y: 0 }, data: { label: "Left", shortName: "L", mode: "reflective", indicators: Array.from({ length: 8 }, (_, index) => `l${index + 1}`) } },
      { id: "right", type: "construct", position: { x: 200, y: 0 }, data: { label: "Right", shortName: "R", mode: "reflective", indicators: Array.from({ length: 8 }, (_, index) => `r${index + 1}`) } },
    ];
    const envelopeEdges: Edge[] = [{ id: "left-right", source: "left", target: "right" }];
    const arranged = layoutSmartplsModel(envelopeNodes, envelopeEdges);
    const graph = buildDiagramGraph(arranged, envelopeEdges, "sem", "model");
    const leftObjects = graph.nodes.filter((node) => node.id === "left" || node.id.startsWith("indicator::left::"));
    const rightObjects = graph.nodes.filter((node) => node.id === "right" || node.id.startsWith("indicator::right::"));
    const leftRight = Math.max(...leftObjects.map((node) => node.position.x + (node.type === "indicator" ? 88 : 104)));
    const rightLeft = Math.min(...rightObjects.map((node) => node.position.x));
    expect(rightLeft - leftRight).toBeGreaterThanOrEqual(100);
  });

  it("keeps free indicators at their absolute coordinates without inflating tidy result layout", () => {
    const freeLayout = defaultDiagramLayout(nodes, edges);
    freeLayout.indicatorLayouts.x.x1 = { side: "free", x: 1_700, y: 920, order: 0, pinned: true };
    const graph = buildDiagramGraph(nodes, edges, "smartpls_result", "model", undefined, {
      layout: freeLayout,
      layoutSource: "tidy_publication",
    });
    const predictor = graph.nodes.find((node) => node.id === "x")!;
    const outcome = graph.nodes.find((node) => node.id === "y")!;
    const freeIndicator = graph.nodes.find((node) => node.id === indicatorNodeId("x", "x1"))!;

    expect(freeIndicator.position).toEqual({ x: 1_700, y: 920 });
    expect(outcome.position.x - predictor.position.x).toBeLessThan(700);
  });

  it("keeps default structural paths and measurement connectors straight through obstacles", () => {
    const obstacleNodes: Array<Node<ConstructData>> = [
      { id: "source", type: "construct", position: { x: 0, y: 80 }, data: { label: "Source", shortName: "S", mode: "reflective", indicators: ["s1"] } },
      { id: "blocker", type: "construct", position: { x: 250, y: 80 }, data: { label: "Blocker", shortName: "B", mode: "reflective", indicators: [] } },
      { id: "target", type: "construct", position: { x: 500, y: 80 }, data: { label: "Target", shortName: "T", mode: "reflective", indicators: [] } },
    ];
    const obstacleEdges: Edge[] = [{ id: "source-target", source: "source", target: "target", label: "Effect" }];
    const layout = defaultDiagramLayout(obstacleNodes, obstacleEdges);
    layout.indicatorLayouts.source.s1 = { side: "free", x: 500, y: 100, order: 0, pinned: true };
    const graph = buildDiagramGraph(obstacleNodes, obstacleEdges, "sem", "model", undefined, { layout });
    expect(graph.edges.find((edge) => edge.id === "source-target")?.data).toMatchObject({
      perimeterRouting: "continuous",
      routing: "straight",
    });
    expect(graph.edges.find((edge) => edge.id === "source-target")?.data?.bendPoints).toBeUndefined();
    const measurement = graph.edges.find((edge) => edge.id === "measurement::source::s1")!;
    expect(measurement.data).toMatchObject({
      perimeterRouting: "continuous",
      routing: "straight",
    });
    expect(measurement.data?.bendPoints).toBeUndefined();

    layout.edgeLayouts["source-target"] = { routing: "straight", pinned: true, labelOffset: { x: 12, y: -8 } };
    const pinned = buildDiagramGraph(obstacleNodes, obstacleEdges, "sem", "model", undefined, { layout });
    expect(pinned.edges.find((edge) => edge.id === "source-target")?.data).toMatchObject({
      routing: "straight",
      labelOffset: { x: 12, y: -8 },
    });
  });

  it("keeps crossing construct paths straight without adding junction semantics", () => {
    const crossingNodes: Array<Node<ConstructData>> = [
      { id: "top-left", type: "construct", position: { x: 0, y: 0 }, data: { label: "Top left", shortName: "TL", mode: "reflective", indicators: [] } },
      { id: "bottom-left", type: "construct", position: { x: 0, y: 260 }, data: { label: "Bottom left", shortName: "BL", mode: "reflective", indicators: [] } },
      { id: "top-right", type: "construct", position: { x: 500, y: 0 }, data: { label: "Top right", shortName: "TR", mode: "reflective", indicators: [] } },
      { id: "bottom-right", type: "construct", position: { x: 500, y: 260 }, data: { label: "Bottom right", shortName: "BR", mode: "reflective", indicators: [] } },
    ];
    const crossingEdges: Edge[] = [
      { id: "descending", source: "top-left", target: "bottom-right" },
      { id: "ascending-control", source: "bottom-left", target: "top-right", data: { role: "control" } },
    ];

    const graph = buildDiagramGraph(crossingNodes, crossingEdges, "sem", "model");
    for (const id of ["descending", "ascending-control"]) {
      const edge = graph.edges.find((candidate) => candidate.id === id)!;
      expect(edge.data).toMatchObject({ perimeterRouting: "continuous", routing: "straight" });
      expect(edge.data?.bendPoints).toBeUndefined();
      expect(edge.data?.junction).toBeUndefined();
    }
  });

  it("keeps manual polyline bends and separates colliding automatic path labels", () => {
    const parallelEdges: Edge[] = [
      { id: "first", source: "x", target: "y", label: "First effect" },
      { id: "second", source: "x", target: "y", label: "Second effect" },
    ];
    const automatic = buildDiagramGraph(nodes, parallelEdges, "sem", "model");
    expect(automatic.edges.find((edge) => edge.id === "first")?.data?.labelOffset)
      .not.toEqual(automatic.edges.find((edge) => edge.id === "second")?.data?.labelOffset);

    const layout = defaultDiagramLayout(nodes, edges);
    layout.edgeLayouts["x-y"] = { routing: "polyline", bendPoints: [{ x: 360, y: 40 }, { x: 440, y: 160 }], pinned: true };
    const manual = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
    expect(manual.edges.find((edge) => edge.id === "x-y")?.data).toMatchObject({
      routing: "polyline",
      bendPoints: [{ x: 360, y: 40 }, { x: 440, y: 160 }],
    });
  });

  it("orders SmartPLS arrangement by structural neighbors to reduce crossings", () => {
    const crossingNodes: Array<Node<ConstructData>> = [
      { id: "a", type: "construct", position: { x: 0, y: 300 }, data: { label: "A", shortName: "A", mode: "reflective", indicators: ["a1"] } },
      { id: "b", type: "construct", position: { x: 0, y: 0 }, data: { label: "B", shortName: "B", mode: "reflective", indicators: ["b1"] } },
      { id: "c", type: "construct", position: { x: 500, y: 300 }, data: { label: "C", shortName: "C", mode: "reflective", indicators: ["c1"] } },
      { id: "d", type: "construct", position: { x: 500, y: 0 }, data: { label: "D", shortName: "D", mode: "reflective", indicators: ["d1"] } },
    ];
    const crossingEdges: Edge[] = [
      { id: "a-d", source: "a", target: "d" },
      { id: "b-c", source: "b", target: "c" },
    ];
    const arranged = layoutSmartplsModel(crossingNodes, crossingEdges);
    expect(arranged.find((node) => node.id === "b")!.position.y).toBeLessThan(arranged.find((node) => node.id === "a")!.position.y);
    expect(arranged.find((node) => node.id === "c")!.position.y).toBeLessThan(arranged.find((node) => node.id === "d")!.position.y);
  });

  it("lays out SmartPLS-like result diagrams with predictors left and outcomes right", () => {
    const graph = buildDiagramGraph(nodes, edges, "smartpls_result", "paths_r2", run);
    const predictor = graph.nodes.find((node) => node.id === "x")!;
    const outcome = graph.nodes.find((node) => node.id === "y")!;
    const predictorIndicator = graph.nodes.find((node) => node.id === indicatorNodeId("x", "x1"))!;
    const outcomeIndicator = graph.nodes.find((node) => node.id === indicatorNodeId("y", "y1"))!;
    expect(predictor.position.x).toBeLessThan(outcome.position.x);
    expect(predictorIndicator.position.x).toBeLessThan(predictor.position.x);
    expect(outcomeIndicator.position.x).toBeGreaterThan(outcome.position.x);
    expect(graph.edges.find((edge) => edge.id === "x-y")?.label).toBe("0.457");
    expect(graph.edges.find((edge) => edge.id === "measurement::x::x1")?.label).toBe("0.910");
    expect(graph.edges.find((edge) => edge.id === "x-y")).toMatchObject({ sourceHandle: "source-right", targetHandle: "target-left" });
    expect(graph.edges.find((edge) => edge.id === "measurement::x::x1")).toMatchObject({ sourceHandle: "source-left", targetHandle: "target-right" });
  });

  it("keeps mediator indicators away from the latent label zone", () => {
    const mediatorNodes: Array<Node<ConstructData>> = [
      { id: "x1", type: "construct", position: { x: 120, y: 80 }, data: { label: "Predictor A", shortName: "XA", mode: "reflective", indicators: ["xa1"] } },
      { id: "x2", type: "construct", position: { x: 120, y: 220 }, data: { label: "Predictor B", shortName: "XB", mode: "reflective", indicators: ["xb1"] } },
      { id: "m", type: "construct", position: { x: 390, y: 150 }, data: { label: "Mediator", shortName: "M", mode: "reflective", indicators: ["m1", "m2"] } },
      { id: "y", type: "construct", position: { x: 660, y: 150 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
    ];
    const mediatorEdges: Edge[] = [
      { id: "x1-m", source: "x1", target: "m" },
      { id: "x2-m", source: "x2", target: "m" },
      { id: "m-y", source: "m", target: "y" },
    ];
    const graph = buildDiagramGraph(mediatorNodes, mediatorEdges, "sem", "model");
    const mediator = graph.nodes.find((node) => node.id === "m")!;
    const firstIndicator = graph.nodes.find((node) => node.id === indicatorNodeId("m", "m1"))!;
    const firstMeasurement = graph.edges.find((edge) => edge.id === "measurement::m::m1")!;
    expect(firstIndicator.position.y).toBeLessThan(mediator.position.y - 50);
    expect(firstMeasurement).toMatchObject({ sourceHandle: "source-top", targetHandle: "target-bottom" });

    const arranged = buildDiagramGraph(mediatorNodes, mediatorEdges, "smartpls_result", "model", undefined, { layoutSource: "tidy_publication" });
    const arrangedPredictor = arranged.nodes.find((node) => node.id === "x1")!;
    const arrangedMediator = arranged.nodes.find((node) => node.id === "m")!;
    const arrangedOutcome = arranged.nodes.find((node) => node.id === "y")!;
    expect(arrangedMediator.position.x - arrangedPredictor.position.x).toBeGreaterThanOrEqual(270);
    expect(arrangedOutcome.position.x - arrangedMediator.position.x).toBeGreaterThanOrEqual(270);
  });

  it("suppresses SmartPLS-like result labels for stale runs", () => {
    const graph = buildDiagramGraph(nodes, [{ id: "y-x", source: "y", target: "x" }], "smartpls_result", "paths_r2", run);
    expect(graph.compatible).toBe(false);
    expect(graph.edges.find((edge) => edge.id === "y-x")?.label).toBe("");
    expect(graph.diagnostic).toContain("Numeric overlays are hidden");
  });

  it("round-trips encoded indicator visual ids", () => {
    expect(parseIndicatorNodeId(indicatorNodeId("x", "item 1"))).toEqual({ constructId: "x", indicator: "item 1" });
  });

  it("uses persisted free indicator positions on the editable academic canvas", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    layout.indicatorLayouts.x.x1 = { side: "free", x: 44, y: 55, order: 0, pinned: true };
    const graph = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
    expect(graph.nodes.find((node) => node.id === indicatorNodeId("x", "x1"))?.position).toEqual({ x: 44, y: 55 });
  });

  it("preserves Standard presentation objects while synchronizing scientific nodes", () => {
    const layout = defaultDiagramLayout(nodes, edges, {
      standardSemPresentation: {
        schemaVersion: 1,
        objects: [{
          kind: "note",
          id: "generated-hierarchy-note",
          subject: "x-y",
          text: "Generated strong-hierarchy dependency.",
          x: 40,
          y: 40,
        }],
      },
    });
    expect(layout.standardSemPresentation).toEqual({
      schemaVersion: 1,
      objects: [{
        kind: "note",
        id: "generated-hierarchy-note",
        subject: "x-y",
        text: "Generated strong-hierarchy dependency.",
        x: 40,
        y: 40,
      }],
    });
  });

  it("can export result diagrams from current canvas positions instead of forcing tidy layout", () => {
    const moved = nodes.map((node) => node.id === "x" ? { ...node, position: { x: 900, y: 240 } } : node);
    const graph = buildDiagramGraph(moved, edges, "smartpls_result", "model", undefined, { layoutSource: "current_canvas" });
    expect(graph.nodes.find((node) => node.id === "x")?.position).toEqual({ x: 900, y: 240 });
    const tidy = buildDiagramGraph(moved, edges, "smartpls_result", "model", undefined, { layoutSource: "tidy_publication" });
    expect(tidy.nodes.find((node) => node.id === "x")?.position).not.toEqual({ x: 900, y: 240 });
  });

  it("keeps edit mode draggable and publication/result modes locked", () => {
    const editable = buildDiagramGraph(nodes, edges, "sem", "model");
    const publication = buildDiagramGraph(nodes, edges, "publication", "model");
    const resultGraph = buildDiagramGraph(nodes, edges, "smartpls_result", "model");
    expect(editable.nodes.find((node) => node.id === "x")?.draggable).toBe(true);
    expect(publication.nodes.find((node) => node.id === "x")?.draggable).toBe(false);
    expect(resultGraph.nodes.find((node) => node.id === "x")?.draggable).toBe(false);
  });

  it("applies persisted edge label offsets to graph edges", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    layout.edgeLayouts["x-y"].labelOffset = { x: 18, y: -12 };
    const graph = buildDiagramGraph(nodes, edges, "sem", "model", undefined, { layout });
    expect(graph.edges.find((edge) => edge.id === "x-y")?.data?.labelOffset).toEqual({ x: 18, y: -12 });
  });

  it("fingerprints only engine-relevant structural paths", () => {
    const first = modelFingerprint(nodes, [...edges, { id: "cov", source: "x", target: "y", data: { role: "covariance" } }]);
    const second = modelFingerprint(nodes, edges);
    expect(first).toBe(second);
  });
});
