import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset } from "../types";
import { nativeAnalysisSettingsForWorkbenchKind } from "./nativeAnalysisCatalog";
import { nativePlsReadiness } from "./nativePlsReadiness";

const numericMetadata = (name: string): ColumnMetadata => ({
  name,
  label: null,
  column_type: "numeric",
  scale_type: "continuous",
  missing_markers: [],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: {},
});

const dataset: Dataset = {
  id: "study",
  name: "study.csv",
  columns: ["x1", "x2", "y1", "y2"],
  rows: Array.from({ length: 30 }, (_, index) => ({ x1: index, x2: index + 1, y1: index + 2, y2: index + 3 })),
  missing: 0,
  fingerprint: "sha256:study",
  kind: "raw",
  columnMetadata: ["x1", "x2", "y1", "y2"].map(numericMetadata),
};

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", position: { x: 300, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];

const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];

const settings: AnalysisUiSettings = {
  method: "pls_pm",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20260810,
  workers: 1,
  confidenceLevel: 0.95,
};

const readiness = (overrides: Partial<{ dataset: Dataset; nodes: Array<Node<ConstructData>>; edges: Edge[]; settings: AnalysisUiSettings; nativeDesktop: boolean }> = {}) => nativePlsReadiness({
  dataset: overrides.dataset ?? dataset,
  nodes: overrides.nodes ?? nodes,
  edges: overrides.edges ?? edges,
  settings: overrides.settings ?? settings,
  nativeDesktop: overrides.nativeDesktop ?? true,
});

describe("nativePlsReadiness", () => {
  it("blocks an empty desktop project without inventing navigation actions", () => {
    const result = readiness({
      dataset: { id: "empty", name: "No dataset", columns: [], rows: [], missing: 0 },
      nodes: [],
      edges: [],
    });

    expect(result.canRun).toBe(false);
    expect(result.blockers.map((item) => item.id)).toEqual(expect.arrayContaining(["data", "constructs", "model"]));
    expect(result.items.every((item) => !("actionLabel" in item) && !("actionView" in item))).toBe(true);
  });

  it("blocks calculation in the web preview", () => {
    const result = readiness({ nativeDesktop: false });

    expect(result.canRun).toBe(false);
    expect(result.blockers).toContainEqual(expect.objectContaining({ id: "runtime", status: "blocked" }));
    expect(result.blockers.find((item) => item.id === "runtime")?.detail).toContain("desktop runtime");
  });

  it("accepts a fingerprinted numeric raw-data model in the desktop runtime", () => {
    const result = readiness();

    expect(result.canRun).toBe(true);
    expect(result.summary).toBe("Ready to calculate");
    expect(result.blockers).toEqual([]);
    expect(result.warnings).toEqual([]);
  });

  it("accepts bounded point ML and redirects archived bootstrap settings to unified CB-SEM Calculate", () => {
    const semSettings: AnalysisUiSettings = {
      ...settings,
      method: "cbsem",
      weightingScheme: "path",
      preprocessing: "standardized",
      cbsemModelType: "sem",
      cbsemMeanStructure: false,
      cbsemGroupColumn: null,
      cbsemInvarianceSteps: null,
      cbsemBootstrapSamples: 0,
      caseWeightColumn: null,
    };
    const sem = readiness({ settings: semSettings });
    expect(sem.canRun).toBe(true);
    expect(sem.items.find((item) => item.id === "calculation")?.detail).toContain("recursive CB-SEM");
    expect(sem.items.find((item) => item.id === "calculation")?.detail).toContain("30 complete cases");

    const cfa = readiness({ settings: { ...semSettings, cbsemModelType: "cfa" }, edges: [] });
    expect(cfa.canRun).toBe(true);
    expect(cfa.items.find((item) => item.id === "calculation")?.detail).toContain("reflective CFA");

    const cfaWithPath = readiness({ settings: { ...semSettings, cbsemModelType: "cfa" } });
    expect(cfaWithPath.canRun).toBe(false);
    expect(cfaWithPath.blockers.find((item) => item.id === "calculation")?.detail).toContain("measurement-only model");

    for (const cbsemBootstrapInterval of [
      "percentile_type7",
      "analytic_studentized_type7",
      "bca_type7",
    ] as const) {
      const archivedBootstrap = readiness({
        settings: {
          ...semSettings,
          cbsemModelType: "cfa",
          cbsemBootstrapSamples: 1_000,
          cbsemBootstrapInterval,
          workers: 4,
        },
        edges: [],
      });
      expect(archivedBootstrap.canRun).toBe(false);
      expect(archivedBootstrap.blockers.find((item) => item.id === "calculation")?.detail)
        .toContain("Choose Case bootstrap in the CB-SEM Calculate settings");
    }

    const invalidScope = readiness({
      settings: { ...semSettings, workers: 2, cbsemMeanStructure: true, bootstrapSamples: 999 },
      nodes: [{ ...nodes[0], data: { ...nodes[0].data, mode: "formative" } }, nodes[1]],
    });
    expect(invalidScope.canRun).toBe(false);
    expect(invalidScope.blockers.find((item) => item.id === "calculation")?.detail)
      .toMatch(/reflective factors.*Point-only CB-SEM \/ CFA uses one deterministic worker.*CB-SEM bootstrap v2 cannot be combined with generic PLS bootstrap/s);
  });

  it("accepts the bounded mixed-block GSCA ALS scope and blocks unsupported model/settings shapes", () => {
    const gscaSettings: AnalysisUiSettings = {
      ...settings,
      method: "gsca",
      weightingScheme: "path",
      preprocessing: "standardized",
      tolerance: 1e-7,
      maxIterations: 3_000,
      workers: 1,
      caseWeightColumn: null,
    };
    const mixedNodes = [
      { ...nodes[0], data: { ...nodes[0].data, mode: "formative" as const } },
      nodes[1],
    ];
    const valid = readiness({ settings: gscaSettings, nodes: mixedNodes });
    expect(valid.canRun).toBe(true);
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("joint global least-squares ALS");

    const dataOnly = readiness({ settings: gscaSettings, nodes: [], edges: [] });
    expect(dataOnly.canRun).toBe(false);
    expect(dataOnly.blockers.find((item) => item.id === "calculation")?.detail).toMatch(/at least two component constructs.*structural path/s);

    const unsupported = readiness({
      settings: { ...gscaSettings, maxIterations: 500, tolerance: 1e-5, workers: 2, bootstrapSamples: 999 },
      nodes: [...mixedNodes, {
        id: "z",
        position: { x: 600, y: 0 },
        data: { label: "Isolated", shortName: "Z", mode: "reflective", indicators: ["z1"] },
      }],
      edges: [{ ...edges[0], data: { role: "control" } }],
    });
    expect(unsupported.canRun).toBe(false);
    expect(unsupported.blockers.find((item) => item.id === "calculation")?.detail).toMatch(/isolated.*control paths.*3,000.*1e-7.*one deterministic worker.*inference/s);
  });

  it("admits bounded Standard PLSc bootstrapping while keeping point PLSc and permutation separate", () => {
    const pointSettings = nativeAnalysisSettingsForWorkbenchKind(settings, "plsc");
    const point = readiness({
      settings: pointSettings,
    });
    expect(point.canRun).toBe(true);
    expect(point.items.find((item) => item.id === "calculation")?.detail)
      .toBe("Consistent PLS correction is selected for reflective constructs with at least two indicators each.");

    for (const bootstrapSamples of [1_000, 5_000, 10_000]) {
      const bootstrapSettings = nativeAnalysisSettingsForWorkbenchKind({
        ...settings,
        weightingScheme: "factor",
        bootstrapSamples,
        workers: 4,
      }, "plsc_bootstrap");
      const bootstrap = readiness({
        settings: bootstrapSettings,
      });
      expect(bootstrap.canRun, `${bootstrapSamples} PLSc bootstrap samples`).toBe(true);
      expect(bootstrap.blockers).toEqual([]);
      expect(bootstrap.items.find((item) => item.id === "calculation")?.detail)
        .toContain("Full-refit consistent PLS bootstrapping is selected");
      expect(bootstrap.items.find((item) => item.id === "calculation")?.detail)
        .toContain("Point PLSc is re-estimated for every primary and delete-one sample");
    }

    const consistentPermutation = readiness({
      settings: { ...settings, method: "plsc", permutationSamples: 999 },
    });
    expect(consistentPermutation.canRun).toBe(false);
    expect(consistentPermutation.blockers.find((item) => item.id === "calculation")?.detail)
      .toContain("PLSc consistent permutation is not available in the Standard calculation workflow");
  });

  it("blocks PLSc bootstrap settings outside the frozen bounded contract", () => {
    const invalidSettings: Array<[string, AnalysisUiSettings, string]> = [
      ["too few samples", { ...settings, method: "plsc", bootstrapSamples: 999 }, "1,000 to 10,000 whole-number"],
      ["fractional samples", { ...settings, method: "plsc", bootstrapSamples: 1_000.5 }, "1,000 to 10,000 whole-number"],
      ["too many samples", { ...settings, method: "plsc", bootstrapSamples: 10_001 }, "1,000 to 10,000 whole-number"],
      ["studentized inference", { ...settings, method: "plsc", bootstrapSamples: 1_000, studentizedInnerSamples: 99 }, "does not support studentized intervals"],
      ["permutation combination", { ...settings, method: "plsc", bootstrapSamples: 1_000, permutationSamples: 999 }, "consistent permutation is not available"],
      ["one-sided inference", { ...settings, method: "plsc", bootstrapSamples: 1_000, bootstrapTestTail: "one_sided_greater" }, "fixed two-sided inference"],
      ["invalid workers", { ...settings, method: "plsc", bootstrapSamples: 1_000, workers: 65 }, "1 to 64 PLSc bootstrap workers"],
      ["invalid seed", { ...settings, method: "plsc", bootstrapSamples: 1_000, seed: -1 }, "whole-number PLSc bootstrap seed"],
      ["invalid confidence", { ...settings, method: "plsc", bootstrapSamples: 1_000, confidenceLevel: 0.799 }, "confidence level from 80% through 99.9%"],
    ];

    for (const [label, invalid, expected] of invalidSettings) {
      const result = readiness({ settings: invalid });
      expect(result.canRun, label).toBe(false);
      expect(result.blockers.find((item) => item.id === "calculation")?.detail, label).toContain(expected);
    }
  });

  it("accepts a valid generated HOC block and blocks malformed HOC metadata", () => {
    const component: Node<ConstructData> = {
      id: "z",
      position: { x: 0, y: 180 },
      data: { label: "Second component", shortName: "Z", mode: "reflective", indicators: ["y1"] },
    };
    const outcome: Node<ConstructData> = {
      id: "outcome",
      position: { x: 500, y: 90 },
      data: { label: "Outcome", shortName: "OUT", mode: "reflective", indicators: ["y2"] },
    };
    const higherOrder: Node<ConstructData> = {
      id: "hoc",
      position: { x: 150, y: 180 },
      data: {
        label: "Higher-order construct",
        shortName: "HOC",
        mode: "reflective",
        indicators: [],
        semantic: "higher_order",
        higherOrder: { id: "hoc", components: ["x", "z"], method: "two_stage", stage_one_recipe: null },
      },
    };
    const hocNodes = [nodes[0], component, outcome, higherOrder];
    const hocEdges = [{ id: "hoc-outcome", source: "hoc", target: "outcome" }];
    const valid = readiness({ nodes: hocNodes, edges: hocEdges });
    expect(valid.canRun).toBe(true);
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("chosen approach");
    expect(readiness({ nodes: hocNodes, edges: hocEdges, settings: { ...settings, method: "bootstrap", bootstrapSamples: 100 } }).canRun).toBe(true);

    const malformed: Node<ConstructData> = {
      ...higherOrder,
      data: { ...higherOrder.data, higherOrder: { ...higherOrder.data.higherOrder!, components: ["missing"] } },
    };
    const blocked = readiness({ nodes: [nodes[0], component, outcome, malformed], edges: hocEdges });
    expect(blocked.canRun).toBe(false);
    expect(blocked.blockers.find((item) => item.id === "model")?.detail).toContain("higher-order construct");
  });

  it("accepts bounded permutation inference and keeps it separate from bootstrapping", () => {
    for (const permutationSamples of [100, 999, 4_321]) {
      const permutation = readiness({ settings: { ...settings, permutationSamples } });
      expect(permutation.canRun, `${permutationSamples} permutations`).toBe(true);
      expect(permutation.items.find((item) => item.id === "calculation")?.detail).toContain("single-model Freedman-Lane");
      expect(permutation.items.find((item) => item.id === "calculation")?.detail).not.toContain("Experimental");
    }

    const mixed = readiness({ settings: { ...settings, bootstrapSamples: 500, permutationSamples: 999 } });
    expect(mixed.canRun).toBe(false);
    expect(mixed.blockers.find((item) => item.id === "calculation")?.detail).toContain("separate calculations");
  });

  it("accepts the qualified 999-sample bootstrap count and blocks invalid raw resample counts", () => {
    expect(readiness({ settings: { ...settings, bootstrapSamples: 999 } }).canRun).toBe(true);

    for (const invalidSettings of [
      { ...settings, bootstrapSamples: 999.5 },
      { ...settings, bootstrapSamples: 10_001 },
      { ...settings, permutationSamples: 321.5 },
      { ...settings, permutationSamples: 10_001 },
    ]) {
      const result = readiness({ settings: invalidSettings });
      expect(result.canRun).toBe(false);
      expect(result.blockers.find((item) => item.id === "calculation")?.detail).toMatch(/requires/);
    }
  });

  it("blocks an invalid studentized bootstrap plan before dispatch", () => {
    const result = readiness({
      settings: { ...settings, bootstrapSamples: 500, studentizedInnerSamples: 99 },
    });

    expect(result.canRun).toBe(false);
    expect(result.blockers.find((item) => item.id === "calculation")?.detail).toContain("at least 999 primary bootstrap samples");
  });

  it("accepts indicator PLSpredict / CVPAT and blocks unsupported or underpowered shapes", () => {
    const prediction = readiness({ settings: { ...settings, method: "predict" } });
    expect(prediction.canRun).toBe(true);
    expect(prediction.items.find((item) => item.id === "calculation")?.detail).toContain("10-fold × 10-repeat indicator prediction");
    expect(prediction.items.find((item) => item.id === "calculation")?.detail).toContain("IA/LM benchmarks and one-sided 95% CVPAT tests");

    const noPath = readiness({ settings: { ...settings, method: "predict" }, edges: [] });
    expect(noPath.canRun).toBe(false);
    expect(noPath.blockers.find((item) => item.id === "calculation")?.detail).toContain("structural path");

    const tooSmall = readiness({
      settings: { ...settings, method: "predict" },
      dataset: { ...dataset, rows: dataset.rows.slice(0, 12) },
    });
    expect(tooSmall.canRun).toBe(false);
    expect(tooSmall.blockers.find((item) => item.id === "calculation")?.detail).toContain("at least 20 observations");

    const incomplete = readiness({
      settings: { ...settings, method: "predict" },
      dataset: {
        ...dataset,
        rows: dataset.rows.map((row, index) => index < 15 ? { ...row, y2: null } : row),
        missing: 15,
      },
    });
    expect(incomplete.canRun).toBe(false);
    expect(incomplete.blockers.find((item) => item.id === "calculation")?.detail).toContain("15 remain after listwise filtering");

    const preview = readiness({
      settings: { ...settings, method: "predict" },
      dataset: { ...dataset, rowCount: 100, rows: dataset.rows.slice(0, 10) },
    });
    expect(preview.warnings.find((item) => item.id === "calculation")?.detail).toContain("QuickPLS verifies this after listwise filtering");

    const wrongConfidence = readiness({ settings: { ...settings, method: "predict", confidenceLevel: 0.9 } });
    expect(wrongConfidence.canRun).toBe(false);
    expect(wrongConfidence.blockers.find((item) => item.id === "calculation")?.detail).toContain("fixed 95% confidence level");

    const interaction: Node<ConstructData> = {
      id: "interaction",
      position: { x: 150, y: 180 },
      data: {
        label: "X x Z",
        shortName: "XxZ",
        mode: "reflective",
        indicators: ["x1"],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "y", outcome: "y", method: "two_stage_product_score" },
      },
    };
    const special = readiness({ settings: { ...settings, method: "predict" }, nodes: [...nodes, interaction] });
    expect(special.canRun).toBe(false);
    expect(special.blockers.find((item) => item.id === "calculation")?.detail).toContain("interaction");

    const formative = readiness({
      settings: { ...settings, method: "predict" },
      nodes: nodes.map((node) => node.id === "y" ? { ...node, data: { ...node.data, mode: "formative" as const } } : node),
    });
    expect(formative.canRun).toBe(false);
    expect(formative.blockers.find((item) => item.id === "calculation")?.detail).toContain("reflective endogenous constructs");
  });

  it("requires an explicit, non-indicator A-versus-B plan for combined MICOM and permutation MGA", () => {
    const groupedDataset: Dataset = {
      ...dataset,
      columns: [...dataset.columns, "group"],
      rows: dataset.rows.map((row, index) => ({ ...row, group: index < 15 ? "A" : "B" })),
      columnMetadata: [...(dataset.columnMetadata ?? []), {
        name: "group",
        label: null,
        column_type: "text",
        scale_type: "nominal",
        missing_markers: [],
        theoretical_min: null,
        theoretical_max: null,
        value_labels: {},
      }],
    };
    const mgaSettings: AnalysisUiSettings = {
      ...settings,
      method: "mga",
      weightingScheme: "path",
      groupColumn: "group",
      groupAValue: "A",
      groupBValue: "B",
      groupMethods: "micom,mga_permutation",
      groupPermutationSamples: 5_000,
      micomConfiguralConfirmed: true,
    };

    const valid = readiness({ dataset: groupedDataset, settings: mgaSettings });
    expect(valid.canRun).toBe(true);
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("A (Group A) and B (Group B)");
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("exactly 5000 deterministic size-preserving permutations");
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("structural-path group differences are calculated");

    const conflicts: AnalysisUiSettings[] = [
      { ...mgaSettings, groupAValue: null },
      { ...mgaSettings, groupBValue: "A" },
      { ...mgaSettings, groupMethods: "mga_permutation" },
      { ...mgaSettings, groupMethods: "micom" },
      { ...mgaSettings, micomConfiguralConfirmed: false },
      { ...mgaSettings, groupPermutationSamples: 4_999 },
      { ...mgaSettings, weightingScheme: "factor" },
      { ...mgaSettings, preprocessing: "mean_centered" },
      { ...mgaSettings, bootstrapSamples: 100 },
    ];
    for (const conflicting of conflicts) {
      expect(readiness({ dataset: groupedDataset, settings: conflicting }).canRun).toBe(false);
    }

    const indicatorConflict = readiness({
      dataset: groupedDataset,
      nodes: nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, indicators: ["x1", "group"] } } : node),
      settings: mgaSettings,
    });
    expect(indicatorConflict.canRun).toBe(false);
    expect(indicatorConflict.blockers.find((item) => item.id === "calculation")?.detail).toContain("cannot also be a model indicator");
  });

  it("exposes only descriptive CCA composite residual diagnostics in the bounded native scope", () => {
    const ccaSettings: AnalysisUiSettings = {
      ...settings,
      method: "cca",
      weightingScheme: "path",
      preprocessing: "standardized",
    };
    const valid = readiness({ settings: ccaSettings });
    expect(valid.canRun).toBe(true);
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("descriptive residuals only");
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("no thresholds or inferential classification");

    const singleIndicator = readiness({
      nodes: nodes.map((node) => ({ ...node, data: { ...node.data, indicators: [node.data.indicators[0]] } })),
      settings: ccaSettings,
    });
    expect(singleIndicator.canRun).toBe(true);

    const cases: Array<[string, Partial<{ nodes: Array<Node<ConstructData>>; edges: Edge[]; settings: AnalysisUiSettings }>]> = [
      ["at least two constructs", { nodes: [nodes[0]], edges: [] }],
      ["at least one structural path", { edges: [] }],
      ["reflective constructs", { nodes: nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, mode: "formative" as const } } : node) }],
      ["control paths", { edges: [{ id: "control-x-y", source: "x", target: "y", data: { role: "control" } }] }],
      ["standardized preprocessing", { settings: { ...ccaSettings, preprocessing: "mean_centered" } }],
      ["do not support case weights", { settings: { ...ccaSettings, caseWeightColumn: "x1" } }],
      ["do not calculate resampling inference", { settings: { ...ccaSettings, permutationSamples: 999 } }],
    ];
    for (const [message, patch] of cases) {
      const blocked = readiness({
        nodes: patch.nodes,
        edges: patch.edges,
        settings: patch.settings ?? ccaSettings,
      });
      expect(blocked.canRun, message).toBe(false);
      expect(blocked.blockers.find((item) => item.id === "calculation")?.detail).toContain(message);
    }
  });

  it("requires one immutable endogenous construct ID for fixed-scope importance-performance analysis", () => {
    const valid = readiness({
      settings: {
        ...settings,
        method: "ipma",
        weightingScheme: "path",
        preprocessing: "standardized",
        ipmaTargets: "y",
      },
    });
    expect(valid.canRun).toBe(true);
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("Outcome [y]");
    expect(valid.items.find((item) => item.id === "calculation")?.detail).toContain("observed-range");

    for (const invalidSettings of [
      { ...settings, method: "ipma" as const, ipmaTargets: null },
      { ...settings, method: "ipma" as const, ipmaTargets: "y,x" },
      { ...settings, method: "ipma" as const, ipmaTargets: "Outcome" },
      { ...settings, method: "ipma" as const, ipmaTargets: "x" },
      { ...settings, method: "ipma" as const, ipmaTargets: "y", weightingScheme: "factor" as const },
      { ...settings, method: "ipma" as const, ipmaTargets: "y", preprocessing: "mean_centered" as const },
      { ...settings, method: "ipma" as const, ipmaTargets: "y", bootstrapSamples: 999 },
      { ...settings, method: "ipma" as const, ipmaTargets: "y", caseWeightColumn: "x1" },
    ]) {
      const blocked = readiness({ settings: invalidSettings });
      expect(blocked.canRun).toBe(false);
      expect(blocked.blockers.find((item) => item.id === "calculation")).toBeDefined();
    }

    const controlOnly = readiness({
      settings: { ...settings, method: "ipma", ipmaTargets: "y" },
      edges: [{ id: "control-x-y", source: "x", target: "y", data: { role: "control" } }],
    });
    expect(controlOnly.canRun).toBe(false);
    expect(controlOnly.blockers.find((item) => item.id === "calculation")?.detail).toContain("incoming structural path");
  });

  it("accepts bounded Consistent PLS and blocks incompatible measurement models", () => {
    const consistent = readiness({
      settings: { ...settings, method: "plsc", weightingScheme: "path" },
    });
    expect(consistent.canRun).toBe(true);
    expect(consistent.items.find((item) => item.id === "calculation")?.detail).toContain("Consistent PLS correction");

    const singleIndicator = nodes.map((node) => node.id === "x"
      ? { ...node, data: { ...node.data, indicators: ["x1"] } }
      : node);
    const underspecified = readiness({
      nodes: singleIndicator,
      settings: { ...settings, method: "plsc", weightingScheme: "path" },
    });
    expect(underspecified.canRun).toBe(false);
    expect(underspecified.blockers.find((item) => item.id === "calculation")?.detail).toContain("at least two indicators");

    const formative = readiness({
      nodes: nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, mode: "formative" as const } } : node),
      settings: { ...settings, method: "plsc", weightingScheme: "path" },
    });
    expect(formative.canRun).toBe(false);
    expect(formative.blockers.find((item) => item.id === "calculation")?.detail).toContain("reflective measurement models");
  });

  it("accepts qualified interaction_v2 while keeping legacy two-stage moderation bounded", () => {
    const moderator: Node<ConstructData> = { id: "m", position: { x: 0, y: 160 }, data: { label: "Moderator", shortName: "M", mode: "reflective", indicators: ["m1", "m2"] } };
    const interaction: Node<ConstructData> = {
      id: "xm",
      position: { x: 160, y: 160 },
      data: {
        label: "X x M",
        shortName: "XM",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "m", outcome: "y", method: "two_stage_product_score" },
      },
    };
    const moderationNodes = [...nodes, moderator, interaction];
    const moderationEdges: Edge[] = [...edges, { id: "m-y", source: "m", target: "y" }, { id: "xm-y", source: "xm", target: "y" }];
    const moderationDataset: Dataset = {
      ...dataset,
      columns: [...dataset.columns, "m1", "m2"],
      rows: dataset.rows.map((row, index) => ({ ...row, m1: index + 4, m2: index + 5 })),
      columnMetadata: [...(dataset.columnMetadata ?? []), numericMetadata("m1"), numericMetadata("m2")],
    };
    const ready = readiness({
      dataset: moderationDataset,
      nodes: moderationNodes,
      edges: moderationEdges,
      settings: { ...settings, weightingScheme: "path", preprocessing: "standardized" },
    });
    expect(ready.canRun).toBe(true);
    expect(ready.items.find((item) => item.id === "calculation")?.detail).toContain("two-stage moderation");

    const interactionV2: Node<ConstructData> = {
      ...interaction,
      data: {
        ...interaction.data,
        interaction: {
          kind: "interaction_v2",
          termId: "interaction:x-m",
          operands: ["x", "m"],
          outcome: "y",
          focalRelationId: "x-y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
          productIndicator: null,
        },
      },
    };
    const v2Ready = readiness({
      dataset: moderationDataset,
      nodes: [...nodes, moderator, interactionV2],
      edges: moderationEdges,
      settings: { ...settings, weightingScheme: "path", preprocessing: "standardized" },
    });
    expect(v2Ready.canRun).toBe(true);
    expect(v2Ready.items.find((item) => item.id === "calculation")?.detail)
      .toContain("qualified simultaneous two-way moderation");

    for (const invalidSettings of [
      { ...settings, weightingScheme: "factor" as const, preprocessing: "standardized" as const },
      { ...settings, weightingScheme: "path" as const, preprocessing: "mean_centered" as const },
      { ...settings, weightingScheme: "path" as const, preprocessing: "standardized" as const, caseWeightColumn: "m1" },
    ]) {
      const blocked = readiness({ dataset: moderationDataset, nodes: moderationNodes, edges: moderationEdges, settings: invalidSettings });
      expect(blocked.canRun).toBe(false);
      expect(blocked.blockers.find((item) => item.id === "calculation")?.detail).toMatch(/moderation/);
    }

    const withControl = readiness({
      dataset: moderationDataset,
      nodes: moderationNodes,
      edges: moderationEdges.map((edge) => edge.id === "m-y" ? { ...edge, data: { role: "control" } } : edge),
      settings: { ...settings, weightingScheme: "path", preprocessing: "standardized" },
    });
    expect(withControl.canRun).toBe(false);
    expect(withControl.blockers.find((item) => item.id === "calculation")?.detail).toContain("control paths");

    const secondModerator: Node<ConstructData> = { id: "m-next", position: { x: 0, y: 260 }, data: { label: "Second moderator", shortName: "M2", mode: "reflective", indicators: ["m3", "m4"] } };
    const secondInteraction: Node<ConstructData> = {
      id: "xm-next",
      position: { x: 160, y: 260 },
      data: {
        label: "X x M2",
        shortName: "XM2",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "m-next", outcome: "y", method: "two_stage_product_score" },
      },
    };
    const multipleInteractions = readiness({
      dataset: {
        ...moderationDataset,
        columns: [...moderationDataset.columns, "m3", "m4"],
        rows: moderationDataset.rows.map((row, index) => ({ ...row, m3: index + 6, m4: index + 7 })),
        columnMetadata: [...(moderationDataset.columnMetadata ?? []), numericMetadata("m3"), numericMetadata("m4")],
      },
      nodes: [...moderationNodes, secondModerator, secondInteraction],
      edges: [...moderationEdges, { id: "m-next-y", source: "m-next", target: "y" }, { id: "xm-next-y", source: "xm-next", target: "y" }],
      settings: { ...settings, weightingScheme: "path", preprocessing: "standardized" },
    });
    expect(multipleInteractions.canRun).toBe(false);
    expect(multipleInteractions.items.find((item) => item.id === "model")?.status).toBe("ready");
    expect(multipleInteractions.blockers.find((item) => item.id === "calculation")?.detail).toContain("exactly one two-way interaction");
  });

  it("recognizes a strict strong-hierarchy three-way projection as qualified readiness", () => {
    const moderatorW: Node<ConstructData> = {
      id: "w",
      position: { x: 0, y: 160 },
      data: { label: "Moderator W", shortName: "W", mode: "reflective", indicators: ["w1", "w2"] },
    };
    const moderatorZ: Node<ConstructData> = {
      id: "z",
      position: { x: 0, y: 260 },
      data: { label: "Moderator Z", shortName: "Z", mode: "reflective", indicators: ["z1", "z2"] },
    };
    const interactionNode = (
      id: string,
      operands: [string, string] | [string, string, string],
      focalRelationId: string,
    ): Node<ConstructData> => ({
      id,
      position: { x: 160, y: 160 },
      data: {
        label: operands.join(" x "),
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: `interaction:${id}`,
          operands,
          outcome: "y",
          focalRelationId,
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
          productIndicator: null,
        },
      },
    });
    const hierarchy = [
      interactionNode("xw", ["x", "w"], "x-y"),
      interactionNode("xz", ["x", "z"], "x-y"),
      interactionNode("wz", ["w", "z"], "w-y"),
      interactionNode("xwz", ["x", "w", "z"], "x-y"),
    ];
    const threeWayEdges: Edge[] = [
      ...edges,
      { id: "w-y", source: "w", target: "y" },
      { id: "z-y", source: "z", target: "y" },
      ...hierarchy.map((node) => ({ id: `${node.id}-y`, source: node.id, target: "y" })),
    ];
    const threeWayDataset: Dataset = {
      ...dataset,
      columns: [...dataset.columns, "w1", "w2", "z1", "z2"],
      rows: dataset.rows.map((row, index) => ({
        ...row,
        w1: index + 4,
        w2: index + 5,
        z1: index + 6,
        z2: index + 7,
      })),
      columnMetadata: [
        ...(dataset.columnMetadata ?? []),
        numericMetadata("w1"),
        numericMetadata("w2"),
        numericMetadata("z1"),
        numericMetadata("z2"),
      ],
    };

    const result = readiness({
      dataset: threeWayDataset,
      nodes: [...nodes, moderatorW, moderatorZ, ...hierarchy],
      edges: threeWayEdges,
      settings: { ...settings, weightingScheme: "path", preprocessing: "standardized" },
    });

    expect(result.canRun).toBe(true);
    expect(result.blockers).toEqual([]);
    expect(result.items.find((item) => item.id === "calculation")?.detail)
      .toContain("qualified bounded three-way moderation");
  });

  it("checks Weighted PLS setup without claiming to validate unseen native rows", () => {
    const weightedDataset: Dataset = {
      ...dataset,
      columns: [...dataset.columns, "case_weight"],
      rows: dataset.rows.map((row, index) => ({ ...row, case_weight: index + 1 })),
      columnMetadata: [...(dataset.columnMetadata ?? []), numericMetadata("case_weight")],
    };
    const weightedSettings: AnalysisUiSettings = {
      ...settings,
      method: "wpls",
      weightingScheme: "path",
      preprocessing: "standardized",
      caseWeightColumn: "case_weight",
    };
    const ready = readiness({ dataset: weightedDataset, settings: weightedSettings });
    expect(ready.canRun).toBe(true);
    expect(ready.items.find((item) => item.id === "calculation")).toMatchObject({ status: "ready" });

    const partial = readiness({
      dataset: { ...weightedDataset, rows: weightedDataset.rows.slice(0, 5), rowCount: 2_000 },
      settings: weightedSettings,
    });
    expect(partial.canRun).toBe(true);
    expect(partial.items.find((item) => item.id === "calculation")).toMatchObject({ status: "warning" });
    expect(partial.items.find((item) => item.id === "calculation")?.detail).toContain("the calculation checks the complete");

    const missing = readiness({ dataset: weightedDataset, settings: { ...weightedSettings, caseWeightColumn: null } });
    expect(missing.canRun).toBe(false);
    expect(missing.blockers.find((item) => item.id === "calculation")?.detail).toContain("Choose a positive numeric");

    const invalid = readiness({
      dataset: {
        ...weightedDataset,
        rows: weightedDataset.rows.map((row, index) => ({ ...row, case_weight: index === 2 ? 0 : row.case_weight })),
      },
      settings: weightedSettings,
    });
    expect(invalid.canRun).toBe(false);
    expect(invalid.blockers.find((item) => item.id === "calculation")?.detail).toContain("nonpositive");
  });

  it("blocks duplicate indicator assignments and directed structural cycles", () => {
    const duplicateNodes = nodes.map((node) => node.id === "y"
      ? { ...node, data: { ...node.data, indicators: ["x1", "y2"] } }
      : node);
    const cyclicEdges = [...edges, { id: "y-x", source: "y", target: "x" }];
    const result = readiness({ nodes: duplicateNodes, edges: cyclicEdges });

    expect(result.canRun).toBe(false);
    expect(result.blockers.find((item) => item.id === "indicators")?.detail).toContain("assigned more than once");
    expect(result.blockers.find((item) => item.id === "model")?.detail).toContain("directed cycle");
  });

  it("warns for a very small sample without turning the warning into a blocker", () => {
    const result = readiness({ dataset: { ...dataset, rows: dataset.rows.slice(0, 5) } });

    expect(result.canRun).toBe(true);
    expect(result.summary).toBe("Ready with 1 warning");
    expect(result.warnings).toContainEqual(expect.objectContaining({ id: "sample-size", status: "warning" }));
  });

  it("uses the full native case count rather than the resident preview rows", () => {
    const result = readiness({ dataset: { ...dataset, rows: dataset.rows.slice(0, 5), rowCount: 2_000 } });

    expect(result.canRun).toBe(true);
    expect(result.items.find((item) => item.id === "data")?.detail).toContain("2000 observations");
    expect(result.items.find((item) => item.id === "sample-size")?.detail).toContain("2000 observations");
    expect(result.warnings).not.toContainEqual(expect.objectContaining({ id: "sample-size" }));
  });

  it("blocks assigned variables declared as non-numeric", () => {
    const nonNumeric = dataset.columnMetadata!.map((column) => column.name === "x1"
      ? { ...column, column_type: "text" as const, scale_type: "nominal" as const }
      : column);
    const result = readiness({ dataset: { ...dataset, columnMetadata: nonNumeric } });

    expect(result.canRun).toBe(false);
    expect(result.blockers.find((item) => item.id === "numeric-indicators")?.detail).toContain("x1");
  });

  it("allows standalone NCA without constructing a SEM model", () => {
    const result = readiness({
      nodes: [],
      edges: [],
      settings: {
        ...settings,
        method: "nca",
        preprocessing: "unstandardized",
        ncaX: "x1",
        ncaY: "y1",
        ncaCeiling: "both",
        ncaPermutationSamples: 999,
      },
    });

    expect(result.canRun).toBe(true);
    expect(result.items.map((item) => item.id)).toEqual(["runtime", "data", "calculation"]);
    expect(result.items.some((item) => ["constructs", "indicators", "model"].includes(item.id))).toBe(false);
    expect(result.items.find((item) => item.id === "calculation")?.detail).toContain("Standalone NCA is ready");
  });

  it("blocks invalid NCA variables and permutation bounds before dispatch", () => {
    const result = readiness({
      nodes: [],
      edges: [],
      settings: {
        ...settings,
        method: "nca",
        preprocessing: "unstandardized",
        ncaX: "x1",
        ncaY: "x1",
        ncaCeiling: "both",
        ncaPermutationSamples: 0,
      },
    });

    expect(result.canRun).toBe(false);
    expect(result.blockers).toHaveLength(1);
    expect(result.blockers[0].detail).toContain("must be different");
    expect(result.blockers[0].detail).toContain("1 to 10,000 permutations");
  });

  it("allows standalone PCA without a SEM model and validates retention before dispatch", () => {
    const ready = readiness({
      nodes: [],
      edges: [],
      settings: {
        ...settings,
        method: "pca",
        preprocessing: "standardized",
        pcaVariables: "x1,x2,y1",
        pcaComponentRule: "fixed",
        pcaComponents: 2,
      },
    });
    expect(ready.canRun).toBe(true);
    expect(ready.items.map((item) => item.id)).toEqual(["runtime", "data", "calculation"]);
    expect(ready.items.find((item) => item.id === "calculation")?.detail).toContain("Standalone PCA is ready");

    const blocked = readiness({
      nodes: [],
      edges: [],
      settings: {
        ...settings,
        method: "pca",
        preprocessing: "standardized",
        pcaVariables: "x1,x2",
        pcaComponentRule: "fixed",
        pcaComponents: 3,
      },
    });
    expect(blocked.canRun).toBe(false);
    expect(blocked.blockers[0].detail).toContain("number of selected variables");
  });

  it("routes model-free binary logistic regression through exact 0/1 readiness", () => {
    const logisticDataset: Dataset = {
      ...dataset,
      columns: ["converted", "score", "control"],
      rows: Array.from({ length: 12 }, (_, index) => ({
        converted: index % 2,
        score: index + 1,
        control: index % 3,
      })),
      rowCount: 12,
      columnMetadata: ["converted", "score", "control"].map(numericMetadata),
    };
    const logisticSettings: AnalysisUiSettings = {
      ...settings,
      method: "regression",
      preprocessing: "unstandardized",
      regressionType: "logistic",
      regressionOutcome: "converted",
      regressionPredictors: "score",
      regressionControls: "control",
      robustSe: "none",
    };

    const ready = readiness({ dataset: logisticDataset, nodes: [], edges: [], settings: logisticSettings });
    expect(ready.canRun).toBe(true);
    expect(ready.items.map((item) => item.id)).toEqual(["runtime", "data", "calculation"]);
    expect(ready.items.find((item) => item.id === "calculation")).toMatchObject({
      label: "Binary logistic regression",
      status: "ready",
    });
    expect(ready.items.find((item) => item.id === "calculation")?.detail).toContain("6 class 0 and 6 class 1");

    const bootstrapped = readiness({
      dataset: logisticDataset,
      nodes: [],
      edges: [],
      settings: {
        ...logisticSettings,
        regressionBootstrap: true,
        bootstrapSamples: 10_000,
        workers: 4,
        confidenceLevel: 0.95,
      },
    });
    expect(bootstrapped.canRun).toBe(true);
    expect(bootstrapped.items.map((item) => item.id)).toEqual(["runtime", "data", "calculation", "regression-bootstrap"]);
    expect(bootstrapped.items.at(-1)?.detail).toContain("percentile intervals primary, BCa conditional");
    expect(bootstrapped.items.at(-1)?.detail).toContain("worker-invariant");

    const invalidBootstrap = readiness({
      dataset: logisticDataset,
      nodes: [],
      edges: [],
      settings: { ...logisticSettings, regressionBootstrap: true, bootstrapSamples: 98, confidenceLevel: 0.9 },
    });
    expect(invalidBootstrap.canRun).toBe(false);
    expect(invalidBootstrap.blockers.find((item) => item.id === "regression-bootstrap")?.detail).toContain("99 to 10000");

    const invalid = readiness({
      dataset: {
        ...logisticDataset,
        rows: logisticDataset.rows.map((row, index) => index === 4 ? { ...row, converted: 2 } : row),
      },
      nodes: [],
      edges: [],
      settings: logisticSettings,
    });
    expect(invalid.canRun).toBe(false);
    expect(invalid.blockers).toHaveLength(1);
    expect(invalid.blockers[0]).toMatchObject({ id: "calculation", label: "Binary logistic regression" });
    expect(invalid.blockers[0].detail).toContain("not coded exactly 0 or 1");
  });

  it("enforces the 51-coefficient validation-witness cap only for regression bootstrap", () => {
    const predictorNames = Array.from({ length: 51 }, (_, index) => `x${index + 1}`);
    const regressionDataset: Dataset = {
      id: "wide-regression",
      name: "wide-regression.csv",
      columns: ["y", ...predictorNames],
      rows: Array.from({ length: 60 }, (_, rowIndex) => ({
        y: rowIndex % 2,
        ...Object.fromEntries(predictorNames.map((name, columnIndex) => [name, rowIndex * (columnIndex + 1) + columnIndex])),
      })),
      rowCount: 60,
      missing: 0,
      fingerprint: "sha256:wide-regression",
      kind: "raw",
      columnMetadata: ["y", ...predictorNames].map(numericMetadata),
    };
    const common: AnalysisUiSettings = {
      ...settings,
      method: "regression",
      preprocessing: "unstandardized",
      regressionOutcome: "y",
      regressionControls: null,
      robustSe: "hc3",
      regressionBootstrap: true,
      bootstrapSamples: 99,
      workers: 2,
      confidenceLevel: 0.95,
    };

    const maximumOls = readiness({
      dataset: regressionDataset,
      nodes: [],
      edges: [],
      settings: { ...common, regressionType: "ols", regressionPredictors: predictorNames.slice(0, 50).join(",") },
    });
    expect(maximumOls.canRun).toBe(true);

    const overMaximumOls = readiness({
      dataset: regressionDataset,
      nodes: [],
      edges: [],
      settings: { ...common, regressionType: "ols", regressionPredictors: predictorNames.join(",") },
    });
    expect(overMaximumOls.canRun).toBe(false);
    expect(overMaximumOls.blockers.find((item) => item.id === "calculation")?.detail).toContain("at most 50 predictors and controls (51 coefficient terms including the intercept)");

    const maximumLogistic = readiness({
      dataset: regressionDataset,
      nodes: [],
      edges: [],
      settings: { ...common, regressionType: "logistic", regressionPredictors: predictorNames.slice(0, 50).join(",") },
    });
    expect(maximumLogistic.canRun).toBe(true);

    const legacyPointLimit = readiness({
      dataset: regressionDataset,
      nodes: [],
      edges: [],
      settings: {
        ...common,
        regressionType: "ols",
        regressionBootstrap: false,
        regressionPredictors: predictorNames.slice(0, 26).join(","),
      },
    });
    expect(legacyPointLimit.canRun).toBe(false);
    expect(legacyPointLimit.blockers.find((item) => item.id === "calculation")?.detail).toContain("no more than 25 predictors and controls");
  });

  it("routes graph-defined PROCESS through graph, global-sample, and bootstrap readiness", () => {
    const processColumns = ["y", "x", "m", "w", "b", "c"];
    const processDataset: Dataset = {
      id: "process-data",
      name: "process.csv",
      columns: processColumns,
      rows: Array.from({ length: 40 }, (_, index) => ({
        y: index + (index % 3),
        x: index / 2,
        m: index / 3 + (index % 5),
        w: (index % 9) - 4,
        b: index % 2,
        c: index % 7,
      })),
      rowCount: 40,
      missing: 0,
      fingerprint: "sha256:process",
      kind: "raw",
      columnMetadata: processColumns.map(numericMetadata),
    };
    const processSettings: AnalysisUiSettings = {
      ...settings,
      method: "regression",
      preprocessing: "unstandardized",
      regressionType: "process",
      regressionOutcome: "y",
      regressionPredictors: "x,m,w,b",
      regressionControls: "c",
      regressionBootstrap: true,
      bootstrapSamples: 999,
      workers: 3,
      processGraph: {
        model: "graph",
        focal_predictor: "x",
        paths: [{ from: "x", to: "y" }, { from: "x", to: "m" }, { from: "m", to: "y" }],
        moderators: [{ variable: "w", scale: "continuous" }, { variable: "b", scale: "binary_0_1" }],
        moderations: [
          { from: "x", to: "y", moderator: "w", conditioning_moderator: "b" },
          { from: "x", to: "m", moderator: "w" },
        ],
        continuous_product_centering: "equation_complete_case_mean_v1",
      },
    };
    const ready = readiness({ dataset: processDataset, nodes: [], edges: [], settings: processSettings });
    expect(ready.canRun).toBe(true);
    expect(ready.items.map((item) => item.id)).toEqual(["runtime", "data", "calculation", "process-bootstrap"]);
    expect(ready.items[2]).toMatchObject({ label: "Graph-defined path analysis", status: "ready" });
    expect(ready.items[2].detail).toContain("global listwise-complete cases");
    expect(ready.items[3].detail).toContain("percentile intervals primary and BCa conditional");

    const invalidBinary = readiness({
      dataset: {
        ...processDataset,
        rows: processDataset.rows.map((row, index) => index === 5 ? { ...row, b: 2 } : row),
      },
      nodes: [],
      edges: [],
      settings: processSettings,
    });
    expect(invalidBinary.canRun).toBe(false);
    expect(invalidBinary.blockers.find((item) => item.id === "calculation")?.detail).toContain("outside exact 0/1 coding");
  });
});
