import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData, Dataset } from "../types";
import { analysisReadiness } from "./analysisReadiness";

const dataset: Dataset = {
  id: "d",
  name: "data.csv",
  columns: ["x1", "x2", "y1", "y2", "group"],
  rows: Array.from({ length: 30 }, (_, index) => ({ x1: index, x2: index + 1, y1: index + 2, y2: index + 3, group: index % 2 })),
  missing: 0,
  fingerprint: "abc123",
};

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", position: { x: 300, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];
const settings = {
  method: "pls_pm" as const,
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 1,
  workers: 1,
  confidenceLevel: 0.95,
};

describe("analysisReadiness", () => {
  it("marks a complete desktop PLS setup as runnable", () => {
    const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: true });
    expect(readiness.canRun).toBe(true);
    expect(readiness.summary).toBe("Ready to run");
  });

  it("explains web preview runtime blocking", () => {
    const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: false });
    expect(readiness.canRun).toBe(false);
    expect(readiness.blockers[0].id).toBe("runtime");
  });

  it("explains visible design data that has not been fingerprinted for a desktop run", () => {
    const readiness = analysisReadiness({ dataset: { ...dataset, fingerprint: undefined }, nodes, edges, settings, nativeDesktop: true });
    expect(readiness.canRun).toBe(false);
    expect(readiness.blockers[0]).toMatchObject({
      id: "data",
      actionLabel: "Open data",
      actionView: "data",
    });
    expect(readiness.blockers[0].detail).toContain("design/preview");
    expect(readiness.blockers[0].detail).toContain("reproducible fingerprint");
  });

  it("blocks methods that need additional settings", () => {
    const readiness = analysisReadiness({ dataset, nodes, edges, settings: { ...settings, method: "mga" }, nativeDesktop: true });
    expect(readiness.canRun).toBe(false);
    const blocker = readiness.blockers.find((item) => item.id === "method");
    expect(blocker).toMatchObject({ actionLabel: "Open setup", actionView: "analyses" });
  });

  it("warns about demo-sized samples without blocking structural inspection", () => {
    const readiness = analysisReadiness({ dataset: { ...dataset, rows: dataset.rows.slice(0, 5) }, nodes, edges, settings, nativeDesktop: true });
    expect(readiness.canRun).toBe(true);
    expect(readiness.warnings.some((item) => item.id === "sample-size")).toBe(true);
  });
});
