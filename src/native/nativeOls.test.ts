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

  it("counts listwise-complete and omitted rows and enforces the fitted-term minimum", () => {
    const twoRowsOmitted: Dataset = {
      ...dataset,
      rows: dataset.rows.map((row, index) => index === 4
        ? { ...row, y: null }
        : index === 5
          ? { ...row, m: null }
          : row),
    };
    const ready = nativeOlsReadiness(twoRowsOmitted, settings());

    expect(ready).toMatchObject({ canRun: true, completeCases: 4 });
    expect(twoRowsOmitted.rowCount! - ready.completeCases!).toBe(2);
    expect(ready.detail).toContain("2 fitted terms and 4 complete finite rows");

    const belowMinimum: Dataset = {
      ...twoRowsOmitted,
      rows: twoRowsOmitted.rows.map((row, index) => index === 2 ? { ...row, x: null } : row),
    };
    expect(nativeOlsReadiness(belowMinimum, settings())).toMatchObject({
      canRun: false,
      completeCases: 3,
      blockers: ["OLS requires at least 4 complete finite rows for 2 fitted terms"],
    });
  });

  it("blocks a selected predictor that is constant after listwise deletion", () => {
    const constantPredictor: Dataset = {
      ...dataset,
      rows: dataset.rows.map((row) => ({ ...row, x: 1 })),
    };

    expect(nativeOlsReadiness(constantPredictor, settings())).toMatchObject({
      canRun: false,
      completeCases: 6,
      blockers: ["x is constant after listwise deletion"],
    });
  });

  it("defers complete-row and full-rank checks when only a nonresident preview is loaded", () => {
    const nonresident: Dataset = {
      ...dataset,
      rowCount: 100,
      rows: dataset.rows.slice(0, 2),
    };

    expect(nativeOlsReadiness(nonresident, settings())).toEqual({
      canRun: true,
      blockers: [],
      detail: "Standalone OLS is ready for y with 2 fitted terms. Complete rows and full-rank design will be verified by the desktop engine.",
      completeCases: null,
    });
  });
});
