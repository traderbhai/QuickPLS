import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, Dataset } from "../types";
import { nativePcaNumericColumns, nativePcaReadiness } from "./nativePca";

const settings = (patch: Partial<AnalysisUiSettings> = {}): AnalysisUiSettings => ({
  method: "pca",
  weightingScheme: "path",
  preprocessing: "standardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_812,
  workers: 1,
  confidenceLevel: 0.95,
  pcaVariables: "a,b,c",
  pcaComponentRule: "variance_threshold",
  pcaVarianceThreshold: 0.80,
  ...patch,
});

const dataset = (rows: Dataset["rows"], rowCount = rows.length): Dataset => ({
  id: "pca-data",
  name: "pca.csv",
  columns: ["a", "b", "c", "group"],
  rows,
  rowCount,
  missing: 0,
  fingerprint: "sha256:pca",
  kind: "raw",
  columnMetadata: [
    { name: "a", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "b", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "c", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "group", label: null, column_type: "text", scale_type: "nominal", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
});

describe("native PCA frontend contract", () => {
  it("offers only numeric variables and accepts a valid no-model plan", () => {
    const data = dataset([
      { a: 1, b: 3, c: 2, group: "A" },
      { a: 2, b: 2, c: 5, group: "B" },
      { a: 4, b: 6, c: 1, group: "A" },
    ]);
    expect(nativePcaNumericColumns(data)).toEqual(["a", "b", "c"]);
    expect(nativePcaReadiness(data, settings())).toMatchObject({ canRun: true, completeCases: 3 });
  });

  it("blocks duplicates, nonnumeric variables, constants, and invalid retention plans", () => {
    const data = dataset([
      { a: 1, b: 3, c: 2, group: "A" },
      { a: 1, b: 2, c: 5, group: "B" },
      { a: 1, b: 6, c: 1, group: "A" },
    ]);
    expect(nativePcaReadiness(data, settings({ pcaVariables: "a,a,group", pcaComponentRule: "fixed", pcaComponents: 4 })).blockers)
      .toEqual(expect.arrayContaining([
        "Each PCA variable may be selected only once",
        "The selected variable group is not numeric",
        expect.stringContaining("Fixed components"),
      ]));
    expect(nativePcaReadiness(data, settings()).blockers).toContain("a is constant after listwise deletion");
    expect(nativePcaReadiness(data, settings({ pcaVarianceThreshold: 1 })).blockers)
      .toContain("Cumulative variance threshold must be from 1% to 99.9%");
  });

  it("defers complete-case validation when only a preview is resident", () => {
    const assessment = nativePcaReadiness(dataset([
      { a: 1, b: 2, c: 3, group: "A" },
    ], 120), settings());
    expect(assessment).toMatchObject({ canRun: true, completeCases: null });
    expect(assessment.detail).toContain("verified by the desktop engine");
  });
});
