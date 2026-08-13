import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, Dataset } from "../types";
import { nativeNcaNumericColumns, nativeNcaReadiness } from "./nativeNca";

const settings = (patch: Partial<AnalysisUiSettings> = {}): AnalysisUiSettings => ({
  method: "nca",
  weightingScheme: "path",
  preprocessing: "unstandardized",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_718,
  workers: 1,
  confidenceLevel: 0.95,
  ncaX: "x",
  ncaY: "y",
  ncaCeiling: "both",
  ncaPermutationSamples: 999,
  ...patch,
});

const dataset = (rows: Dataset["rows"], rowCount = rows.length): Dataset => ({
  id: "data-1",
  name: "nca.csv",
  columns: ["x", "y", "group"],
  rows,
  rowCount,
  missing: 0,
  fingerprint: "sha256:nca",
  kind: "raw",
  columnMetadata: [
    { name: "x", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "y", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "group", label: null, column_type: "text", scale_type: "nominal", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
});

describe("native NCA frontend contract", () => {
  it("offers only declared numeric variables", () => {
    expect(nativeNcaNumericColumns(dataset([{ x: 1, y: 2, group: "A" }]))).toEqual(["x", "y"]);
  });

  it("permits a standalone no-model plan with three varying complete rows", () => {
    const readiness = nativeNcaReadiness(dataset([
      { x: 1, y: 2, group: "A" },
      { x: 2, y: 4, group: "A" },
      { x: 3, y: 8, group: "B" },
    ]), settings());
    expect(readiness).toMatchObject({ canRun: true, completeCases: 3 });
  });

  it("blocks duplicate, nonnumeric, constant, incomplete, and out-of-range plans", () => {
    expect(nativeNcaReadiness(dataset([
      { x: 1, y: 1, group: "A" },
      { x: 1, y: 2, group: "B" },
      { x: 1, y: null, group: "B" },
    ]), settings({ ncaPermutationSamples: 0 })).blockers).toEqual(expect.arrayContaining([
      expect.stringContaining("1 to 10,000 permutations"),
      expect.stringContaining("at least 3 complete finite"),
    ]));
    expect(nativeNcaReadiness(dataset([
      { x: 1, y: 1, group: "A" },
      { x: 1, y: 2, group: "B" },
      { x: 1, y: 3, group: "B" },
    ]), settings()).blockers).toContain("The selected X variable is constant after listwise deletion");
    expect(nativeNcaReadiness(dataset([{ x: 1, y: 2, group: "A" }]), settings({ ncaY: "x" })).blockers)
      .toContain("Condition X and outcome Y must be different variables");
  });

  it("defers complete-case inspection when only a resident preview is loaded", () => {
    const readiness = nativeNcaReadiness(dataset([{ x: 1, y: 2, group: "A" }], 100), settings());
    expect(readiness).toMatchObject({ canRun: true, completeCases: null });
    expect(readiness.detail).toContain("verified by the desktop engine");
  });
});
