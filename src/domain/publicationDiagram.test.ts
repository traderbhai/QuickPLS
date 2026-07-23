import { describe, expect, it } from "vitest";
import { publicationDiagramSvg } from "./publicationDiagram";
import { defaultDiagramLayout } from "./diagramGraph";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisRun, ConstructData, PlsResult } from "../types";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", type: "construct", position: { x: 100, y: 80 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", type: "construct", position: { x: 390, y: 80 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
];

const edges: Edge[] = [{ id: "x-y", source: "x", target: "y", label: "Path" }];
const mojibakeR2 = `R${String.fromCharCode(0x00c2)}²`;

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
  it("renders SmartPLS-like path coefficients, loadings, R2, and supported-scope notice by default", () => {
    const svg = publicationDiagramSvg(nodes, edges, run);
    expect(svg).toContain("<svg");
    expect(svg).toContain("WPLS run publication diagram");
    expect(svg).toContain("class=\"smartpls-latent\"");
    expect(svg).toContain("class=\"smartpls-indicator\"");
    expect(svg).toContain("0.457");
    expect(svg).toContain("R&#178; 0.208");
    expect(svg).toContain("0.910");
    expect(svg).toContain("Validated for documented QuickPLS v1.0.0 supported scope");
    expect(svg).not.toContain("Mode A");
    expect(svg).not.toContain("Trash");
  });

  it("can still render the QuickPLS publication style", () => {
    const svg = publicationDiagramSvg(nodes, edges, run, { mode: "publication", palette: "quickpls_color" });
    expect(svg).toContain("class=\"latent reflective\"");
    expect(svg).toContain("class=\"indicator reflective\"");
    expect(svg).toContain("R&#178; 0.208");
  });

  it("escapes labels in model-only diagrams", () => {
    const svg = publicationDiagramSvg([{ ...nodes[0], data: { ...nodes[0].data, label: "A&B <test>" } }], [], undefined);
    expect(svg).toContain("A&amp;B &lt;test&gt;");
    expect(svg).not.toContain("Validated for documented QuickPLS v1.0.0 supported scope");
  });

  it("exports the current canvas indicator layout when requested", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    layout.indicatorLayouts.x.x1 = { side: "free", x: 25, y: 35, order: 0, pinned: true };
    const svg = publicationDiagramSvg(nodes, edges, run, { layoutSource: "current_canvas" }, layout);
    expect(svg).toContain('x="149" y="42" width="88" height="28"');
    expect(svg).toContain("R&#178; 0.208");
    expect(svg).not.toContain(mojibakeR2);
  });
  it("uses SmartPLS-like geometry and omits edit-only UI in SVG export", () => {
    const svg = publicationDiagramSvg(nodes, edges, run);
    expect(svg).toContain('rx="52" ry="34"');
    expect(svg).toContain('marker-end="url(#arrow)"');
    expect(svg).not.toContain("smartpls-edit-handle");
    expect(svg).not.toContain("diagram-context-menu");
    expect(svg).not.toContain("selection");
  });
  it("exports persisted edge label offsets", () => {
    const layout = defaultDiagramLayout(nodes, edges);
    const baseline = publicationDiagramSvg(nodes, edges, run, { layoutSource: "current_canvas" }, layout);
    layout.edgeLayouts["x-y"].labelOffset = { x: 20, y: -16 };
    const shifted = publicationDiagramSvg(nodes, edges, run, { layoutSource: "current_canvas" }, layout);
    expect(shifted).toContain("0.457");
    expect(shifted).not.toBe(baseline);
  });
});
