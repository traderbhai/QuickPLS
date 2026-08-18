import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, ConstructData, Dataset } from "../types";
import { methodApplicabilityFor } from "./methodApplicability";

const dataset: Dataset = {
  id: "endogeneity-data",
  name: "endogeneity.csv",
  columns: ["x1", "x2", "y1", "y2"],
  rows: Array.from({ length: 20 }, (_, index) => ({ x1: index + 1, x2: index + 2, y1: index * 2 + 1, y2: index * 2 + 2 })),
  missing: 0,
  fingerprint: "sha256:endogeneity-data",
  kind: "raw",
  columnMetadata: [],
};
const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", position: { x: 300, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];
const path: Edge = { id: "path-x-y", source: "x", target: "y" };
const settings: AnalysisUiSettings = {
  method: "endogeneity",
  weightingScheme: "path",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 7,
  workers: 1,
  confidenceLevel: 0.95,
};

function evaluate(modelNodes = nodes, modelEdges: Edge[] = [path]) {
  return methodApplicabilityFor("endogeneity", {
    dataset,
    nodes: modelNodes,
    edges: modelEdges,
    settings,
    nativeDesktop: true,
  });
}

describe("Gaussian-copula applicability", () => {
  it("exposes the diagnostic-only bounded scope for an ordinary structural path", () => {
    const result = evaluate();
    expect(result.status).toBe("available");
    expect(result.reason).toContain("does not prove causality");
    expect(result.checks.find((check) => check.id === "endogeneity-bounded-shape")).toMatchObject({ status: "passed" });
  });

  it("blocks control-only, generated-interaction, and higher-order shapes before execution", () => {
    const control = evaluate(nodes, [{ id: "control-x-y", source: "x", target: "y", data: { role: "control" } }]);
    expect(control.status).toBe("needs_setup");
    expect(control.checks.find((check) => check.id === "structural-paths")).toMatchObject({ status: "failed" });
    expect(control.checks.find((check) => check.id === "endogeneity-bounded-shape")).toMatchObject({ status: "failed" });

    const higherOrderNodes = nodes.map((node) => node.id === "x" ? {
      ...node,
      data: { ...node.data, semantic: "higher_order" as const, higherOrder: { id: "x", components: ["y"], method: "two_stage" as const } },
    } : node);
    expect(evaluate(higherOrderNodes).checks.find((check) => check.id === "endogeneity-bounded-shape")).toMatchObject({ status: "failed" });

    const interactionNodes: Array<Node<ConstructData>> = [...nodes, {
      id: "x-by-x",
      position: { x: 150, y: 150 },
      data: {
        label: "Interaction",
        shortName: "XX",
        mode: "reflective",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "x", outcome: "y", method: "two_stage_product_score" },
      },
    }];
    expect(evaluate(interactionNodes).checks.find((check) => check.id === "endogeneity-bounded-shape")).toMatchObject({ status: "failed" });
  });
});
