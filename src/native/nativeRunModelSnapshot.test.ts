import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { defaultDiagramLayout } from "../domain/diagramGraph";
import type { AnalysisRun, ConstructData } from "../types";
import {
  createAnalysisModelSnapshot,
  resolveAnalysisModel,
} from "./nativeRunModelSnapshot";

function model() {
  const nodes: Array<Node<ConstructData>> = [{
    id: "a",
    type: "construct",
    position: { x: 10, y: 20 },
    selected: true,
    dragging: true,
    measured: { width: 120, height: 80 },
    data: {
      label: "A",
      shortName: "A",
      mode: "reflective",
      indicators: ["A1"],
    },
  }];
  const edges: Edge[] = [{
    id: "a-b",
    source: "a",
    target: "b",
    selected: true,
    data: { role: "structural" },
  }];
  return { nodes, edges, layout: defaultDiagramLayout(nodes, edges) };
}

function run(modelSnapshot?: AnalysisRun["modelSnapshot"]): AnalysisRun {
  return {
    id: "run-1",
    name: "PLS run",
    method: "PLS-SEM Algorithm",
    createdAt: "2026-08-10T00:00:00.000Z",
    seed: 42,
    status: "completed",
    warnings: [],
    fingerprint: "dataset-1",
    modelSnapshot,
  };
}

describe("native run model snapshots", () => {
  it("deeply detaches the completed-run model and removes transient canvas state", () => {
    const current = model();
    const snapshot = createAnalysisModelSnapshot(current.nodes, current.edges, current.layout);

    current.nodes[0].position.x = 999;
    current.nodes[0].data.indicators.push("A2");
    (current.edges[0].data as Record<string, unknown>).role = "covariance";
    current.layout.constructLayouts.a.x = 999;

    expect(snapshot.nodes[0].position.x).toBe(10);
    expect(snapshot.nodes[0].data.indicators).toEqual(["A1"]);
    expect(snapshot.edges[0].data?.role).toBe("structural");
    expect(snapshot.diagramLayout?.constructLayouts.a.x).toBe(10);
    expect(snapshot.nodes[0]).not.toHaveProperty("selected");
    expect(snapshot.nodes[0]).not.toHaveProperty("dragging");
    expect(snapshot.nodes[0]).not.toHaveProperty("measured");
    expect(snapshot.edges[0]).not.toHaveProperty("selected");
  });

  it("uses the stored run model and falls back to the live model for old runs", () => {
    const captured = model();
    const snapshot = createAnalysisModelSnapshot(captured.nodes, captured.edges, captured.layout);
    const live = model();
    live.nodes[0].position.x = 500;
    live.layout.constructLayouts.a.x = 500;

    const resolvedSnapshot = resolveAnalysisModel(run(snapshot), live.nodes, live.edges, live.layout);
    expect(resolvedSnapshot.nodes).toBe(snapshot.nodes);
    expect(resolvedSnapshot.edges).toBe(snapshot.edges);
    expect(resolvedSnapshot.diagramLayout).toBe(snapshot.diagramLayout);

    const resolvedLegacy = resolveAnalysisModel(run(), live.nodes, live.edges, live.layout);
    expect(resolvedLegacy.nodes).toBe(live.nodes);
    expect(resolvedLegacy.edges).toBe(live.edges);
    expect(resolvedLegacy.diagramLayout).toBe(live.layout);
  });
});
