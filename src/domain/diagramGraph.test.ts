import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import { buildDiagramGraph, defaultDiagramLayout, indicatorNodeId, layoutSmartplsModel, modelFingerprint, parseIndicatorNodeId } from "./diagramGraph";
import type { AnalysisRun, ConstructData, PlsResult } from "../types";

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
  it("derives visible latent and indicator nodes without changing model ids", () => {
    const graph = buildDiagramGraph(nodes, edges, "sem", "model");
    expect(graph.nodes.filter((node) => node.type === "latent")).toHaveLength(2);
    expect(graph.nodes.filter((node) => node.type === "indicator")).toHaveLength(3);
    expect(graph.edges.filter((edge) => edge.id.startsWith("measurement::"))).toHaveLength(3);
    expect(graph.edges.find((edge) => edge.id === "x-y")).toBeTruthy();
  });

  it("orients reflective and formative measurement arrows differently", () => {
    const graph = buildDiagramGraph(nodes, edges, "sem", "model");
    const reflective = graph.edges.find((edge) => edge.id === "measurement::x::x1")!;
    const formative = graph.edges.find((edge) => edge.id === "measurement::y::y1")!;
    expect(reflective.source).toBe("x");
    expect(reflective.target).toBe(indicatorNodeId("x", "x1"));
    expect(formative.source).toBe(indicatorNodeId("y", "y1"));
    expect(formative.target).toBe("y");
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
