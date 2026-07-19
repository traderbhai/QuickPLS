import { describe, expect, it } from "vitest";
import { publicationDiagramSvg } from "./publicationDiagram";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisRun, ConstructData, PlsResult } from "../types";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", type: "construct", position: { x: 100, y: 80 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", type: "construct", position: { x: 390, y: 80 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
];

const edges: Edge[] = [{ id: "x-y", source: "x", target: "y", label: "Path" }];

const result: PlsResult = {
  method_version: "wpls_case_weighted_v1",
  converged: true,
  iterations: 5,
  used_observations: 30,
  omitted_observations: 0,
  outer_estimates: [
    { construct: "x", indicator: "x1", weight: 0.5, loading: 0.91 },
    { construct: "x", indicator: "x2", weight: 0.5, loading: 0.82 },
    { construct: "y", indicator: "y1", weight: 1, loading: 1 },
  ],
  paths: [{ source: "x", target: "y", coefficient: 0.4567 }],
  effects: [],
  r_squared: { y: 0.208 },
  warnings: [],
};

const run: AnalysisRun = {
  id: "run-1",
  name: "WPLS run",
  method: "Weighted PLS",
  createdAt: "2026-07-19T00:00:00.000Z",
  seed: 1,
  status: "completed",
  warnings: [],
  fingerprint: "fixture",
  result,
};

describe("publication diagram SVG", () => {
  it("renders path coefficients, loadings, R2, and supported-scope notice", () => {
    const svg = publicationDiagramSvg(nodes, edges, run);
    expect(svg).toContain("<svg");
    expect(svg).toContain("WPLS run publication diagram");
    expect(svg).toContain("0.457");
    expect(svg).toContain("R2 0.208");
    expect(svg).toContain("0.910");
    expect(svg).toContain("Validated for documented QuickPLS v0.9.0-rc.1 supported scope");
  });

  it("escapes labels in model-only diagrams", () => {
    const svg = publicationDiagramSvg([{ ...nodes[0], data: { ...nodes[0].data, label: "A&B <test>" } }], [], undefined);
    expect(svg).toContain("A&amp;B &lt;test&gt;");
    expect(svg).not.toContain("Validated for documented QuickPLS v0.9.0-rc.1 supported scope");
  });
});
