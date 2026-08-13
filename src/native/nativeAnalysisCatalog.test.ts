import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings } from "../types";
import { buildNativeAnalysisRecipe, nativeAnalysisRecipeDescriptor } from "./nativeAnalysisRecipe";
import { nativePlsReadiness } from "./nativePlsReadiness";
import {
  NATIVE_ANALYSIS_CATALOG,
  filterNativeAnalysisCatalog,
  nativeAnalysisSettingsForWorkbenchKind,
  nativeAnalysisStartLabel,
  nativeWorkbenchAnalysisKindForSettings,
} from "./nativeAnalysisCatalog";

const settings: AnalysisUiSettings = {
  method: "pls_pm",
  weightingScheme: "pca",
  preprocessing: "mean_centered",
  tolerance: 1e-7,
  maxIterations: 3_000,
  bootstrapSamples: 5_000,
  studentizedInnerSamples: 199,
  permutationSamples: 999,
  seed: 20_260_718,
  workers: 4,
  confidenceLevel: 0.95,
  caseWeightColumn: "WEIGHT",
  groupColumn: "segment",
  groupAValue: "A",
  groupBValue: "B",
  ipmaTargets: "y",
  groupMethods: "micom,mga_permutation",
  groupPermutationSamples: 5_000,
  micomConfiguralConfirmed: true,
};

describe("native analysis catalog", () => {
  it("exposes each native-workbench method once using the canonical recipe label", () => {
    expect(NATIVE_ANALYSIS_CATALOG.map((item) => item.kind)).toEqual([
      "pls_algorithm",
      "plsc",
      "wpls",
      "gsca",
      "cca",
      "cta_pls",
      "ipma",
      "cbsem",
      "pls_bootstrap",
      "pls_permutation",
      "mga",
      "predict",
      "nca",
      "pca",
      "regression",
    ]);
    expect(new Set(NATIVE_ANALYSIS_CATALOG.map((item) => item.kind)).size).toBe(NATIVE_ANALYSIS_CATALOG.length);
    for (const item of NATIVE_ANALYSIS_CATALOG) {
      expect(item.label).toBe(item.kind === "regression" ? "Regression" : nativeAnalysisRecipeDescriptor(item.kind).label);
    }
    expect(NATIVE_ANALYSIS_CATALOG).toHaveLength(15);
  });

  it("links each catalog kind to an exact ordered tuple of unique parity capabilities", () => {
    expect(NATIVE_ANALYSIS_CATALOG.map(({ kind, capabilityIds }) => [kind, capabilityIds])).toEqual([
      ["pls_algorithm", ["qpls3.pls.algorithm"]],
      ["plsc", ["qpls3.pls.consistent"]],
      ["wpls", ["qpls3.pls.weighted"]],
      ["gsca", ["qpls3.gsca.als"]],
      ["cca", ["qpls3.assessment.cca_residuals"]],
      ["cta_pls", ["qpls3.assessment.cta_pls"]],
      ["ipma", ["qpls3.assessment.ipma"]],
      ["cbsem", ["qpls3.cbsem.ml"]],
      ["pls_bootstrap", ["qpls3.inference.bootstrap"]],
      ["pls_permutation", ["qpls3.inference.structural_path_randomization"]],
      ["mga", ["qpls3.groups.micom_permutation_mga"]],
      ["predict", ["qpls3.prediction.plspredict_cvpat"]],
      ["nca", ["qpls3.standalone.nca"]],
      ["pca", ["qpls3.standalone.pca"]],
      ["regression", ["qpls3.standalone.ols", "qpls3.standalone.logistic", "qpls3.standalone.regression_bootstrap", "qpls3.standalone.process"]],
    ]);
    const capabilityIds = NATIVE_ANALYSIS_CATALOG.flatMap((item) => item.capabilityIds);
    expect(capabilityIds).toHaveLength(18);
    expect(new Set(capabilityIds).size).toBe(capabilityIds.length);
  });

  it("filters labels, descriptions, categories, and method aliases without mutating order", () => {
    expect(filterNativeAnalysisCatalog("reflective correction").map((item) => item.kind)).toEqual(["plsc"]);
    expect(filterNativeAnalysisCatalog("case weights").map((item) => item.kind)).toEqual(["wpls"]);
    expect(filterNativeAnalysisCatalog("generalized structured component").map((item) => item.kind)).toEqual(["gsca"]);
    expect(filterNativeAnalysisCatalog("composite residual").map((item) => item.kind)).toEqual(["cca"]);
    expect(filterNativeAnalysisCatalog("assessment").map((item) => item.kind)).toEqual(["cca", "cta_pls", "ipma"]);
    expect(filterNativeAnalysisCatalog("confirmatory tetrad").map((item) => item.kind)).toEqual(["cta_pls"]);
    expect(filterNativeAnalysisCatalog("importance performance").map((item) => item.kind)).toEqual(["ipma"]);
    expect(filterNativeAnalysisCatalog("confirmatory factor maximum likelihood").map((item) => item.kind)).toEqual(["cbsem"]);
    expect(filterNativeAnalysisCatalog("inference").map((item) => item.kind)).toEqual(["pls_bootstrap", "pls_permutation", "mga", "regression"]);
    expect(filterNativeAnalysisCatalog("cvpat").map((item) => item.kind)).toEqual(["predict"]);
    expect(filterNativeAnalysisCatalog("indicator prediction").map((item) => item.kind)).toEqual(["predict"]);
    expect(filterNativeAnalysisCatalog("permutation group a group b").map((item) => item.kind)).toEqual(["mga"]);
    expect(filterNativeAnalysisCatalog("ce-fdh bottleneck").map((item) => item.kind)).toEqual(["nca"]);
    expect(filterNativeAnalysisCatalog("principal component eigenvalue").map((item) => item.kind)).toEqual(["pca"]);
    expect(filterNativeAnalysisCatalog("ordinary least squares hc3").map((item) => item.kind)).toEqual(["regression"]);
    expect(NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === "mga")).toMatchObject({
      label: "MICOM and Two-Group Permutation MGA",
      description: expect.stringContaining("paths, loadings, and weights"),
    });
    expect(NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === "predict")).toMatchObject({
      label: "PLSpredict / CVPAT",
      description: expect.stringContaining("10-fold × 10-repeat"),
    });
    expect(NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === "ipma")).toMatchObject({
      label: "Importance-Performance Map Analysis",
      description: expect.stringContaining("one endogenous target"),
    });
    expect(NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === "cbsem")).toMatchObject({
      label: "CB-SEM / CFA",
      description: expect.stringContaining("maximum likelihood"),
    });
    expect(NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === "nca")).toMatchObject({
      label: "Necessary Condition Analysis",
      description: expect.stringContaining("numeric observed condition"),
    });
    expect(NATIVE_ANALYSIS_CATALOG.find((item) => item.kind === "pca")).toMatchObject({
      label: "Principal Component Analysis",
      description: expect.stringContaining("correlation-matrix eigensystem"),
    });
    expect(filterNativeAnalysisCatalog("   ")).toEqual(NATIVE_ANALYSIS_CATALOG);
  });

  it("normalizes every selection into a mutually exclusive executable settings plan", () => {
    const algorithm = nativeAnalysisSettingsForWorkbenchKind(settings, "pls_algorithm");
    expect(algorithm).toMatchObject({ method: "pls_pm", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null });

    const bootstrap = nativeAnalysisSettingsForWorkbenchKind(settings, "pls_bootstrap");
    expect(bootstrap).toMatchObject({ method: "pls_pm", bootstrapSamples: 5_000, studentizedInnerSamples: 199, permutationSamples: 0, caseWeightColumn: null });

    const permutation = nativeAnalysisSettingsForWorkbenchKind(settings, "pls_permutation");
    expect(permutation).toMatchObject({ method: "pls_pm", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 999, caseWeightColumn: null });

    const prediction = nativeAnalysisSettingsForWorkbenchKind(settings, "predict");
    expect(prediction).toMatchObject({ method: "predict", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, confidenceLevel: 0.95, caseWeightColumn: null, groupMethods: null });

    const plsc = nativeAnalysisSettingsForWorkbenchKind(settings, "plsc");
    expect(plsc).toMatchObject({ method: "plsc", weightingScheme: "path", preprocessing: "mean_centered", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null });

    const wpls = nativeAnalysisSettingsForWorkbenchKind(settings, "wpls");
    expect(wpls).toMatchObject({ method: "wpls", weightingScheme: "path", preprocessing: "standardized", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: "WEIGHT" });

    const cca = nativeAnalysisSettingsForWorkbenchKind(settings, "cca");
    expect(cca).toMatchObject({ method: "cca", weightingScheme: "path", preprocessing: "standardized", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null });

    const ctaPls = nativeAnalysisSettingsForWorkbenchKind(settings, "cta_pls");
    expect(ctaPls).toMatchObject({ method: "cta_pls", weightingScheme: "path", preprocessing: "mean_centered", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null });

    const gsca = nativeAnalysisSettingsForWorkbenchKind(settings, "gsca");
    expect(gsca).toMatchObject({ method: "gsca", weightingScheme: "path", preprocessing: "standardized", tolerance: 1e-7, maxIterations: 3_000, bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null });

    const ipma = nativeAnalysisSettingsForWorkbenchKind(settings, "ipma");
    expect(ipma).toMatchObject({ method: "ipma", weightingScheme: "path", preprocessing: "standardized", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null, ipmaTargets: "y" });
    expect(nativeAnalysisSettingsForWorkbenchKind({ ...settings, ipmaTargets: "y,z" }, "ipma").ipmaTargets).toBeNull();

    const cbsem = nativeAnalysisSettingsForWorkbenchKind({
      ...settings,
      cbsemModelType: "cfa",
      cbsemMeanStructure: true,
      cbsemStandardization: "std_lv",
      cbsemGroupColumn: "segment",
      cbsemInvarianceSteps: "configural,metric",
      cbsemBootstrapSamples: 999,
    }, "cbsem");
    expect(cbsem).toMatchObject({
      method: "cbsem",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      cbsemModelType: "cfa",
      cbsemMeanStructure: false,
      cbsemStandardization: "std_all",
      cbsemGroupColumn: null,
      cbsemInvarianceSteps: null,
      cbsemBootstrapSamples: 0,
    });

    const mga = nativeAnalysisSettingsForWorkbenchKind(settings, "mga");
    expect(mga).toMatchObject({ method: "mga", weightingScheme: "path", preprocessing: "standardized", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, workers: 1, caseWeightColumn: null, groupColumn: "segment", groupAValue: "A", groupBValue: "B", groupMethods: "micom,mga_permutation", groupPermutationSamples: 5_000, micomConfiguralConfirmed: true });
    expect(nativeAnalysisSettingsForWorkbenchKind({ ...settings, weightingScheme: "factor", preprocessing: "mean_centered", groupPermutationSamples: 99 }, "mga")).toMatchObject({ weightingScheme: "path", preprocessing: "standardized", groupPermutationSamples: 5_000 });

    const nca = nativeAnalysisSettingsForWorkbenchKind({
      ...settings,
      ncaX: " x ",
      ncaY: " y ",
      ncaCeiling: "both",
      ncaPermutationSamples: 99_999,
    }, "nca");
    expect(nca).toMatchObject({
      method: "nca",
      weightingScheme: "path",
      preprocessing: "unstandardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      ncaX: "x",
      ncaY: "y",
      ncaCeiling: "both",
      ncaPermutationSamples: 10_000,
    });

    const pca = nativeAnalysisSettingsForWorkbenchKind({
      ...settings,
      pcaVariables: " a, b, c ",
      pcaComponentRule: "variance_threshold",
      pcaVarianceThreshold: 2,
    }, "pca");
    expect(pca).toMatchObject({
      method: "pca",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      pcaVariables: "a,b,c",
      pcaComponentRule: "variance_threshold",
      pcaVarianceThreshold: 0.999,
    });

    const logistic = nativeAnalysisSettingsForWorkbenchKind({
      ...settings,
      regressionType: "logistic",
      regressionOutcome: " y ",
      regressionPredictors: " x, m ",
      regressionControls: " z ",
      robustSe: "hc4",
      preprocessing: "standardized",
      confidenceLevel: 0.9,
    }, "regression");
    expect(logistic).toMatchObject({
      method: "regression",
      regressionType: "logistic",
      regressionOutcome: "y",
      regressionPredictors: "x,m",
      regressionControls: "z",
      robustSe: "none",
      weightingScheme: "path",
      preprocessing: "unstandardized",
      confidenceLevel: 0.95,
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
    });

    const bootstrapped = nativeAnalysisSettingsForWorkbenchKind({
      ...logistic,
      regressionBootstrap: true,
      bootstrapSamples: 0,
      workers: 128,
      studentizedInnerSamples: 199,
      permutationSamples: 999,
    }, "regression");
    expect(bootstrapped).toMatchObject({
      regressionBootstrap: true,
      bootstrapSamples: 10_000,
      workers: 64,
      confidenceLevel: 0.95,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
    });

    const process = nativeAnalysisSettingsForWorkbenchKind({
      ...settings,
      regressionType: "process",
      regressionOutcome: "y",
      regressionPredictors: "stale,order",
      regressionControls: " c ",
      regressionBootstrap: true,
      bootstrapSamples: 999,
      workers: 3,
      processGraph: {
        model: "graph",
        focal_predictor: "x",
        paths: [{ from: "x", to: "m" }, { from: "m", to: "y" }],
        moderators: [{ variable: "w", scale: "continuous" }],
        moderations: [{ from: "x", to: "m", moderator: "w" }],
        continuous_product_centering: "equation_complete_case_mean_v1",
      },
    }, "regression");
    expect(process).toMatchObject({
      method: "regression",
      regressionType: "process",
      regressionOutcome: "y",
      regressionPredictors: "x,m,w",
      regressionControls: "c",
      regressionBootstrap: true,
      bootstrapSamples: 999,
      workers: 3,
      robustSe: "hc3",
      processGraph: {
        model: "graph",
        continuous_product_centering: "equation_complete_case_mean_v1",
      },
    });
  });

  it("normalizes hidden common fields so CCA readiness and recipe construction cannot disagree", () => {
    const cca = nativeAnalysisSettingsForWorkbenchKind({
      ...settings,
      tolerance: 0,
      maxIterations: 0,
      seed: Number.NaN,
      workers: 0,
      confidenceLevel: 0,
    }, "cca");
    const nodes = [
      { id: "x", position: { x: 0, y: 0 }, data: { label: "X", shortName: "X", mode: "reflective" as const, indicators: ["x1", "x2"] } },
      { id: "y", position: { x: 240, y: 0 }, data: { label: "Y", shortName: "Y", mode: "reflective" as const, indicators: ["y1", "y2"] } },
    ];
    const edges = [{ id: "x-y", source: "x", target: "y" }];
    const dataset = {
      id: "dataset-1",
      name: "study.csv",
      columns: ["x1", "x2", "y1", "y2"],
      rows: Array.from({ length: 30 }, (_, index) => ({ x1: index, x2: index + 1, y1: index + 2, y2: index + 3 })),
      rowCount: 30,
      missing: 0,
      fingerprint: "sha256:cca",
      kind: "raw" as const,
    };

    expect(cca).toMatchObject({
      method: "cca",
      tolerance: 1e-12,
      maxIterations: 100,
      seed: 20_260_718,
      workers: 1,
      confidenceLevel: 0.8,
    });
    expect(nativePlsReadiness({ dataset, nodes, edges, settings: cca, nativeDesktop: true }).canRun).toBe(true);
    expect(() => buildNativeAnalysisRecipe({
      kind: "cca",
      recipeId: "11111111-1111-4111-8111-111111111111",
      modelId: "22222222-2222-4222-8222-222222222222",
      createdAt: "2026-08-11T00:00:00.000Z",
      datasetFingerprint: dataset.fingerprint,
      projectName: "CCA parity",
      nodes,
      edges,
      settings: cca,
    })).not.toThrow();
  });

  it("restores a supported selection from settings and falls back from unfinished workflows", () => {
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "plsc" })).toBe("plsc");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "wpls" })).toBe("wpls");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "gsca" })).toBe("gsca");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "predict" })).toBe("predict");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "mga" })).toBe("mga");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "cca" })).toBe("cca");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "ipma" })).toBe("ipma");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "cbsem" })).toBe("cbsem");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "nca" })).toBe("nca");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "pca" })).toBe("pca");
    expect(nativeWorkbenchAnalysisKindForSettings({ ...settings, method: "cta_pls" })).toBe("cta_pls");
  });

  it("uses action-specific labels for initial and retry runs", () => {
    expect(nativeAnalysisStartLabel("plsc", false)).toBe("Start consistent PLS");
    expect(nativeAnalysisStartLabel("wpls", true)).toBe("Retry weighted PLS");
    expect(nativeAnalysisStartLabel("gsca", false)).toBe("Start GSCA");
    expect(nativeAnalysisStartLabel("cca", false)).toBe("Start composite diagnostics");
    expect(nativeAnalysisStartLabel("cta_pls", false)).toBe("Start tetrad diagnostics");
    expect(nativeAnalysisStartLabel("ipma", false)).toBe("Start importance-performance analysis");
    expect(nativeAnalysisStartLabel("cbsem", false)).toBe("Start CB-SEM / CFA");
    expect(nativeAnalysisStartLabel("pls_bootstrap", false)).toBe("Start bootstrapping");
    expect(nativeAnalysisStartLabel("mga", false)).toBe("Start group analysis");
    expect(nativeAnalysisStartLabel("nca", true)).toBe("Retry necessary condition analysis");
    expect(nativeAnalysisStartLabel("pca", false)).toBe("Start principal component analysis");
    expect(nativeAnalysisStartLabel("regression", false, "logistic")).toBe("Start binary logistic regression");
    expect(nativeAnalysisStartLabel("regression", false, "logistic", true)).toBe("Start binary logistic regression with bootstrap");
  });
});
