import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, Dataset } from "../types";
import { nativeOlsReadiness } from "./nativeOls";

const dataset: Dataset = {
  id: "ols-data",
  name: "ols.csv",
  fingerprint: "sha256:ols",
  kind: "raw",
  rowCount: 6,
  columns: ["y", "x", "m", "label"],
  missing: 0,
  rows: [
    { y: 2, x: 1, m: 0, label: "a" },
    { y: 4, x: 2, m: 1, label: "b" },
    { y: 5, x: 3, m: 0, label: "c" },
    { y: 8, x: 4, m: 1, label: "d" },
    { y: 9, x: 5, m: 0, label: "e" },
    { y: 12, x: 6, m: 1, label: "f" },
  ],
  columnMetadata: [
    { name: "y", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "x", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "m", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "label", label: null, column_type: "text", scale_type: "nominal", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
};

const settings = (patch: Partial<AnalysisUiSettings> = {}): AnalysisUiSettings => ({
  method: "regression",
  weightingScheme: "path",
  preprocessing: "unstandardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_812,
  workers: 1,
  confidenceLevel: 0.95,
  regressionType: "ols",
  regressionOutcome: "y",
  regressionPredictors: "x",
  regressionControls: "m",
  robustSe: "hc3",
  ...patch,
});

describe("nativeOlsReadiness", () => {
  it("accepts a distinct numeric outcome, predictor, and control", () => {
    expect(nativeOlsReadiness(dataset, settings())).toMatchObject({
      canRun: true,
      completeCases: 6,
    });
  });

  it("blocks missing, duplicate, nonnumeric, and underidentified selections", () => {
    expect(nativeOlsReadiness(dataset, settings({ regressionOutcome: null })).blockers)
      .toContain("Choose one numeric outcome variable");
    expect(nativeOlsReadiness(dataset, settings({ regressionControls: "x" })).blockers)
      .toContain("Outcome, predictors, and controls must be distinct variables");
    expect(nativeOlsReadiness(dataset, settings({ regressionPredictors: "label" })).blockers)
      .toContain("The selected variable label is not numeric");
    expect(nativeOlsReadiness(dataset, settings({ regressionPredictors: "x,m", regressionControls: "y" })).canRun)
      .toBe(false);
  });
});
