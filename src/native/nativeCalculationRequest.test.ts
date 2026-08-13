import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings } from "../types";
import { createNativeCalculationRequest, parseNativeCalculationRequest } from "./nativeCalculationRequest";
import type { NativeLogisticProfile } from "./nativeLogistic";

const settings: AnalysisUiSettings = {
  method: "plsc",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_718,
  workers: 1,
  confidenceLevel: 0.95,
};

const logisticProfile: NativeLogisticProfile = {
  datasetId: "data-1",
  datasetFingerprint: "sha256:logistic",
  outcome: "converted",
  predictors: ["score"],
  controls: [],
  expectedRows: 8,
  scannedRows: 8,
  completeCases: 8,
  omittedRows: 0,
  zeroCases: 4,
  oneCases: 4,
  invalidOutcomeRows: 0,
  constantTerms: [],
};

describe("native calculation request", () => {
  it("copies the submitted settings snapshot", () => {
    const request = createNativeCalculationRequest("plsc", settings);
    settings.method = "wpls";
    expect(request).toEqual({ kind: "plsc", settings: expect.objectContaining({ method: "plsc" }) });
  });

  it("accepts only workbench-owned method requests", () => {
    expect(parseNativeCalculationRequest({ kind: "wpls", settings: { ...settings, method: "wpls" } }))
      .toEqual({ kind: "wpls", settings: expect.objectContaining({ method: "wpls" }) });
    expect(parseNativeCalculationRequest({ kind: "cca", settings: { ...settings, method: "cca" } }))
      .toEqual({ kind: "cca", settings: expect.objectContaining({ method: "cca" }) });
    expect(parseNativeCalculationRequest({ kind: "ipma", settings: { ...settings, method: "ipma", ipmaTargets: "target-id" } }))
      .toEqual({ kind: "ipma", settings: expect.objectContaining({ method: "ipma", ipmaTargets: "target-id" }) });
    expect(parseNativeCalculationRequest({
      kind: "mga",
      settings: {
        ...settings,
        method: "mga",
        groupColumn: "segment",
        groupAValue: "A",
        groupBValue: "B",
        groupMethods: "mga_permutation",
        groupPermutationSamples: 1_000,
      },
    })).toEqual({
      kind: "mga",
      settings: expect.objectContaining({
        method: "mga",
        groupColumn: "segment",
        groupAValue: "A",
        groupBValue: "B",
      }),
    });
    expect(parseNativeCalculationRequest({ kind: "cta_pls", settings: { ...settings, method: "cta_pls" } })).toEqual({
      kind: "cta_pls",
      settings: expect.objectContaining({ method: "cta_pls" }),
    });
    expect(parseNativeCalculationRequest({ kind: "cca", settings: { ...settings, method: "plsc" } })).toBeNull();
    expect(parseNativeCalculationRequest({ kind: "pls_bootstrap", settings: { ...settings, method: "bootstrap" } })).toBeNull();
    expect(parseNativeCalculationRequest({ kind: "pls_bootstrap", settings: { ...settings, method: "pls_pm" } })).toEqual({
      kind: "pls_bootstrap",
      settings: expect.objectContaining({ method: "pls_pm" }),
    });
    expect(parseNativeCalculationRequest({ kind: "plsc" })).toBeNull();
    expect(parseNativeCalculationRequest(null)).toBeNull();
  });

  it("accepts only an explicit mutually exclusive Structural Path Randomization plan", () => {
    const request = {
      kind: "pls_permutation" as const,
      settings: { ...settings, method: "pls_pm" as const, permutationSamples: 999, workers: 4 },
    };
    expect(parseNativeCalculationRequest(request)).toEqual(request);
    for (const settingsPatch of [
      { permutationSamples: 0 },
      { permutationSamples: 98 },
      { permutationSamples: 10_001 },
      { permutationSamples: 999.5 },
      { bootstrapSamples: 999 },
      { studentizedInnerSamples: 99 },
      { workers: 0 },
      { workers: 65 },
      { workers: 1.5 },
    ]) {
      expect(parseNativeCalculationRequest({ ...request, settings: { ...request.settings, ...settingsPatch } })).toBeNull();
    }
  });

  it("carries a cloned full-profile proof only for binary logistic dispatch", () => {
    const logisticSettings: AnalysisUiSettings = {
      ...settings,
      method: "regression",
      regressionType: "logistic",
      regressionOutcome: "converted",
      regressionPredictors: "score",
      regressionControls: null,
    };
    const submittedProfile = { ...logisticProfile, predictors: [...logisticProfile.predictors] };
    const request = createNativeCalculationRequest("regression", logisticSettings, submittedProfile);
    submittedProfile.predictors[0] = "tampered-after-create";
    expect(request).toMatchObject({
      kind: "regression",
      logisticProfile: { datasetFingerprint: "sha256:logistic", predictors: ["score"], expectedRows: 8, scannedRows: 8 },
    });
    expect(parseNativeCalculationRequest(request)).toEqual(request);

    const malformed = {
      ...request,
      logisticProfile: { ...request.logisticProfile, scannedRows: 7 },
    };
    expect(parseNativeCalculationRequest(malformed)).toBeNull();
    expect(createNativeCalculationRequest("plsc", settings, request.logisticProfile)).not.toHaveProperty("logisticProfile");
  });

  it("preserves only a complete typed regression-bootstrap dispatch snapshot", () => {
    const bootstrapSettings: AnalysisUiSettings = {
      ...settings,
      method: "regression",
      regressionType: "ols",
      regressionOutcome: "y",
      regressionPredictors: "x",
      regressionBootstrap: true,
      bootstrapSamples: 10_000,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      confidenceLevel: 0.95,
      workers: 4,
    };
    const request = createNativeCalculationRequest("regression", bootstrapSettings);
    expect(parseNativeCalculationRequest(request)).toEqual(request);
    expect(parseNativeCalculationRequest({
      ...request,
      settings: { ...request.settings, bootstrapSamples: 0 },
    })).toBeNull();
    expect(parseNativeCalculationRequest({
      ...request,
      settings: { ...request.settings, regressionType: "process" },
    })).toBeNull();
    expect(parseNativeCalculationRequest({
      ...request,
      settings: {
        ...request.settings,
        regressionPredictors: Array.from({ length: 51 }, (_, index) => `x${index + 1}`).join(","),
      },
    })).toBeNull();
    expect(parseNativeCalculationRequest({
      ...request,
      settings: {
        ...request.settings,
        regressionPredictors: Array.from({ length: 50 }, (_, index) => `x${index + 1}`).join(","),
      },
    })).not.toBeNull();
  });

  it("preserves a graph-defined PROCESS request only with its typed graph and current full-data proof", () => {
    const processSettings: AnalysisUiSettings = {
      ...settings,
      method: "regression",
      preprocessing: "unstandardized",
      regressionType: "process",
      regressionOutcome: "y",
      regressionPredictors: "x,m,w",
      regressionControls: "c",
      regressionBootstrap: true,
      bootstrapSamples: 999,
      workers: 2,
      processGraph: {
        model: "graph",
        focal_predictor: "x",
        paths: [{ from: "x", to: "m" }, { from: "m", to: "y" }],
        moderators: [{ variable: "w", scale: "continuous" }],
        moderations: [{ from: "x", to: "m", moderator: "w" }],
        continuous_product_centering: "equation_complete_case_mean_v1",
      },
    };
    const processProfile = {
      datasetId: "process-data",
      datasetFingerprint: "sha256:process",
      selectionToken: JSON.stringify({
        outcome: "y",
        predictors: ["x", "m", "w"],
        controls: ["c"],
        graph: processSettings.processGraph,
      }),
      variables: ["y", "x", "m", "w", "c"],
      binaryModerators: [],
      expectedRows: 40,
      scannedRows: 40,
      completeCases: 39,
      omittedRows: 1,
      invalidBinaryRows: {},
      binaryEquationOutcomes: [],
      constantVariables: [],
    };
    const request = createNativeCalculationRequest("regression", processSettings, processProfile);
    expect(request).toMatchObject({
      kind: "regression",
      settings: { regressionType: "process" },
      processProfile: { completeCases: 39 },
    });
    expect(parseNativeCalculationRequest(request)).toEqual(request);
    expect(parseNativeCalculationRequest({
      ...request,
      settings: { ...request.settings, regressionPredictors: "x,w,m" },
    })).toBeNull();
    expect(parseNativeCalculationRequest({
      ...request,
      settings: { ...request.settings, bootstrapSamples: 98 },
    })).toBeNull();
    expect(parseNativeCalculationRequest({
      ...request,
      processProfile: { ...request.processProfile, scannedRows: 39 },
    })).toBeNull();
  });
});
