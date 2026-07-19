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

  it("fingerprints only engine-relevant structural paths", () => {
    const first = modelFingerprint(nodes, [...edges, { id: "cov", source: "x", target: "y", data: { role: "covariance" } }]);
    const second = modelFingerprint(nodes, edges);
    expect(first).toBe(second);
  });
});
