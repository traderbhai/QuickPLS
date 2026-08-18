import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset } from "../types";
import { dataGuidance, evaluateMethodApplicability, methodApplicabilityFor, modelGuidance, topBarMethods } from "./methodApplicability";

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

const powerNodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2", "m1"] } },
  { id: "y", position: { x: 500, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2", "m2"] } },
];

const powerEdges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];

const powerSettings: Partial<AnalysisUiSettings> = {
  method: "pls_sample_size_power",
  weightingScheme: "path",
  preprocessing: "standardized",
  tolerance: 1e-7,
  maxIterations: 3_000,
  plsPowerScenarioIdentity: "prospective_two_construct_path",
  plsPowerPredictorConstruct: "x",
  plsPowerOutcomeConstruct: "y",
  plsPowerPredictorLoadings: "0.70,0.75,0.80",
  plsPowerOutcomeLoadings: "0.72,0.77,0.82",
  plsPowerPopulationPath: 0.30,
  plsPowerSampleSizeGrid: "50,100,150",
  plsPowerAlpha: 0.05,
  plsPowerTargetPower: 0.80,
  plsPowerMonteCarloReplicates: 250,
  plsPowerBootstrapReplicates: 199,
};

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

  it("requires a two-group column for standalone MICOM v3.1", () => {
    const missing = methodApplicabilityFor("mga", input());
    expect(missing.status).toBe("needs_setup");
    expect(missing.checks.find((check) => check.id === "group-column")).toMatchObject({
      status: "failed",
      detail: "Select an observed group column for MICOM v3.1.",
    });

    const configured = methodApplicabilityFor("mga", input({ groupColumn: "group" }));
    expect(configured.status).toBe("recommended");
    expect(configured.reason).toBe("Recommended because a two-group column is selected for MICOM v3.1.");
    expect(configured.nextActionLabel).toBe("Setup MICOM v3.1");
    expect(`${configured.reason} ${configured.nextActionLabel}`).not.toContain("MICOM/MGA");
  });

  it("checks logistic regression outcome shape", () => {
    const nonBinary = methodApplicabilityFor("regression", input({ method: "regression", regressionType: "logistic", regressionOutcome: "x1", regressionPredictors: "x2" }));
    expect(nonBinary.status).toBe("needs_setup");
    expect(nonBinary.checks.find((check) => check.id === "logistic-binary")).toMatchObject({ status: "failed" });

    const binary = methodApplicabilityFor("regression", input({ method: "regression", regressionType: "logistic", regressionOutcome: "binary", regressionPredictors: "x1,x2" }));
    expect(binary.status).toBe("available");
  });

  it("blocks historical PROCESS settings and requires an explicit graph-defined relationship", () => {
    const legacy = methodApplicabilityFor("regression", input({
      method: "regression",
      regressionType: "process",
      regressionOutcome: "y1",
      regressionPredictors: "x1,m1",
      processModel: "mediation",
    }));
    expect(legacy.checks.find((check) => check.id === "process-scope")).toMatchObject({ status: "failed" });

    const graph = methodApplicabilityFor("regression", input({
      method: "regression",
      regressionType: "process",
      regressionOutcome: "y1",
      regressionPredictors: "x1,m1",
      processGraph: {
        model: "graph",
        focal_predictor: "x1",
        paths: [{ from: "x1", to: "m1" }, { from: "m1", to: "y1" }],
        moderators: [],
        moderations: [],
        continuous_product_centering: "equation_complete_case_mean_v1",
      },
    }));
    expect(graph.checks.some((check) => check.id === "process-scope")).toBe(false);
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

  it("exposes Structural Path Randomization as a supported fixed-score inference add-on", () => {
    const result = methodApplicabilityFor("permutation", input({ method: "permutation", permutationSamples: 999 }));
    expect(result).toMatchObject({
      category: "inference_add_on",
      status: "available",
      scopeStatus: "validated",
      nextActionLabel: "Setup path randomization",
    });
    expect(result.reason).toContain("raw and unadjusted for multiplicity");
    expect(result.expectedOutputs).toEqual([
      "structural path coefficients",
      "exceedance counts",
      "raw two-sided plus-one p values",
    ]);
  });

  it("exposes the bounded, calibrated prospective PLS sample-size and power workflow", () => {
    const result = methodApplicabilityFor("pls_sample_size_power", input(powerSettings, {
      dataset: { ...dataset, columns: [], rows: [], columnMetadata: [], fingerprint: "project-fingerprint" },
      nodes: powerNodes,
      edges: powerEdges,
    }));
    expect(result).toMatchObject({
      category: "workflow_analysis",
      status: "available",
      scopeStatus: "validated",
      nextActionLabel: "Setup prospective power",
    });
    expect(result.checks.map((check) => [check.id, check.status])).toEqual([
      ["runtime", "passed"],
      ["power-provenance", "passed"],
      ["power-model", "passed"],
      ["power-plan", "passed"],
    ]);
    expect(result.reason).toContain("Observed values and observed sample size are not used");
    expect(result.reason).toContain("null-centered two-sided case-bootstrap plus-one inference");
    expect(result.reason).toContain("not retrospective observed power");
    expect(result.expectedOutputs).toEqual(["power by sample size", "Wilson confidence intervals", "conservative grid decision", "failure accounting"]);
  });

  it("fails closed when the power model or explicit grid leaves the bounded scope", () => {
    const invalidModel = methodApplicabilityFor("pls_sample_size_power", input(powerSettings));
    expect(invalidModel.status).toBe("needs_setup");
    expect(invalidModel.checks.find((check) => check.id === "power-model")).toMatchObject({ status: "failed" });
    expect(invalidModel.reason).toContain("exactly two constructs and one directed path");

    const invalidPlan = methodApplicabilityFor("pls_sample_size_power", input({ ...powerSettings, plsPowerSampleSizeGrid: "50" }, { nodes: powerNodes, edges: powerEdges }));
    expect(invalidPlan.status).toBe("needs_setup");
    expect(invalidPlan.checks.find((check) => check.id === "power-plan")).toMatchObject({ status: "failed" });
    expect(invalidPlan.reason).toContain("between 2 and 16 sample sizes");
  });

  it("uses clean R2 text and exposes guidance cards for data and model states", () => {
    const all = evaluateMethodApplicability(input());
    const pls = all.find((item) => item.method.id === "pls_pm");
    expect(pls?.expectedOutputs.join(" ")).toContain("R²");
    expect(pls?.expectedOutputs.join(" ")).not.toContain(`R${"\u00c2"}²`);
    expect(dataGuidance(input()).length).toBeGreaterThan(0);
    expect(modelGuidance(input()).map((item) => item.title)).toContain("Mediation-shaped model");
  });
});
