import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset } from "../types";
import { evaluateMethodApplicability, methodApplicabilityFor, topBarMethods } from "./methodApplicability";

const metadata = (name: string, scale: ColumnMetadata["scale_type"] = "continuous", type: ColumnMetadata["column_type"] = "numeric"): ColumnMetadata => ({
  name,
  label: null,
  column_type: type,
  scale_type: scale,
  missing_markers: [],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: {},
});

const dataset: Dataset = {
  id: "d",
  name: "study.csv",
  columns: ["x1", "x2", "m1", "m2", "y1", "y2", "group", "weight", "binary", "nominal"],
  rows: Array.from({ length: 40 }, (_, index) => ({
    x1: index + 1,
    x2: index + 2,
    m1: index + 3,
    m2: index + 4,
    y1: index + 5,
    y2: index + 6,
    group: index % 2 ? "A" : "B",
    weight: index + 1,
    binary: index % 2,
    nominal: index % 3 === 0 ? "low" : "high",
  })),
  missing: 0,
  fingerprint: "fp",
  kind: "raw",
  columnMetadata: [
    "x1", "x2", "m1", "m2", "y1", "y2", "weight",
  ].map((name) => metadata(name)).concat([metadata("group", "nominal", "text"), metadata("binary", "binary", "numeric"), metadata("nominal", "nominal", "text")]),
};

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "m", position: { x: 250, y: 0 }, data: { label: "Mediator", shortName: "M", mode: "reflective", indicators: ["m1", "m2"] } },
  { id: "y", position: { x: 500, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];

const edges: Edge[] = [
  { id: "x-m", source: "x", target: "m" },
  { id: "m-y", source: "m", target: "y" },
  { id: "x-y", source: "x", target: "y" },
];

const baseSettings: AnalysisUiSettings = {
  method: "pls_pm",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 1,
  workers: 1,
  confidenceLevel: 0.95,
};

const input = (settings: Partial<AnalysisUiSettings> = {}, overrides: Partial<{ dataset: Dataset; nodes: Array<Node<ConstructData>>; edges: Edge[]; nativeDesktop: boolean }> = {}) => ({
  dataset: overrides.dataset ?? dataset,
  nodes: overrides.nodes ?? nodes,
  edges: overrides.edges ?? edges,
  settings: { ...baseSettings, ...settings },
  nativeDesktop: overrides.nativeDesktop ?? true,
});

describe("methodApplicability", () => {
  it("blocks PLS until a raw fingerprinted dataset exists", () => {
    const result = methodApplicabilityFor("pls_pm", input({}, { dataset: { ...dataset, columns: [], rows: [], fingerprint: undefined } }));
    expect(result.status).toBe("needs_setup");
    expect(result.checks.find((check) => check.id === "raw-data")).toMatchObject({ status: "failed", actionLabel: "Import raw dataset" });
  });

  it("recommends PLS for a raw numeric SEM model", () => {
    const result = methodApplicabilityFor("pls_pm", input());
    expect(result.status).toBe("recommended");
    expect(result.reason).toContain("Recommended");
  });

  it("blocks PLSc and CB-SEM for formative constructs", () => {
    const formativeNodes = nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, mode: "formative" as const } } : node);
    expect(methodApplicabilityFor("plsc", input({}, { nodes: formativeNodes })).status).toBe("not_applicable");
    expect(methodApplicabilityFor("cbsem", input({}, { nodes: formativeNodes })).status).toBe("not_applicable");
  });

  it("requires a two-group column for MICOM/MGA", () => {
    const missing = methodApplicabilityFor("mga", input());
    expect(missing.status).toBe("needs_setup");
    expect(missing.checks.find((check) => check.id === "group-column")).toMatchObject({ status: "failed" });

    const configured = methodApplicabilityFor("mga", input({ groupColumn: "group" }));
    expect(configured.status).toBe("recommended");
  });

  it("checks logistic regression outcome shape", () => {
    const nonBinary = methodApplicabilityFor("regression", input({ method: "regression", regressionType: "logistic", regressionOutcome: "x1", regressionPredictors: "x2" }));
    expect(nonBinary.status).toBe("needs_setup");
    expect(nonBinary.checks.find((check) => check.id === "logistic-binary")).toMatchObject({ status: "failed" });

    const binary = methodApplicabilityFor("regression", input({ method: "regression", regressionType: "logistic", regressionOutcome: "binary", regressionPredictors: "x1,x2" }));
    expect(binary.status).toBe("available");
  });

  it("requires positive numeric WPLS weights", () => {
    const invalidRows = dataset.rows.map((row, index) => ({ ...row, weight: index === 0 ? 0 : row.weight }));
    const result = methodApplicabilityFor("wpls", input({ caseWeightColumn: "weight" }, { dataset: { ...dataset, rows: invalidRows } }));
    expect(result.status).toBe("needs_setup");
    expect(result.reason).toContain("nonpositive");
  });

  it("requires explicit numeric PCA variables and NCA X/Y variables", () => {
    expect(methodApplicabilityFor("pca", input({ method: "pca" })).status).toBe("needs_setup");
    expect(methodApplicabilityFor("pca", input({ method: "pca", pcaVariables: "x1,x2" })).status).toBe("available");
    expect(methodApplicabilityFor("nca", input({ method: "nca" })).status).toBe("needs_setup");
    expect(methodApplicabilityFor("nca", input({ method: "nca", ncaX: "x1", ncaY: "y1" })).status).toBe("available");
  });

  it("keeps bootstrap out of the primary top-bar method list unless selected", () => {
    const all = evaluateMethodApplicability(input());
    expect(all.find((item) => item.method.id === "bootstrap")?.category).toBe("inference_add_on");
    expect(topBarMethods(all, "pls_pm").map((item) => item.method.id)).not.toContain("bootstrap");
    expect(topBarMethods(all, "bootstrap").map((item) => item.method.id)).toContain("bootstrap");
  });
});
