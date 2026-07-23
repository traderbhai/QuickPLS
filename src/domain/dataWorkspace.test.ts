import { describe, expect, it } from "vitest";
import { dataQualitySummary, detectPrefixGroups, filteredColumns } from "./dataWorkspace";
import type { Dataset } from "../types";

const dataset: Dataset = {
  id: "test",
  name: "test.csv",
  columns: ["COMP1", "COMP2", "LIKE1", "LIKE2", "TEXT1", "CONST1"],
  rows: [
    { COMP1: 1, COMP2: 2, LIKE1: 3, LIKE2: 4, TEXT1: "A", CONST1: 9 },
    { COMP1: 2, COMP2: 3, LIKE1: 4, LIKE2: null, TEXT1: "B", CONST1: 9 },
    { COMP1: 3, COMP2: 4, LIKE1: 5, LIKE2: null, TEXT1: "C", CONST1: 9 },
  ],
  missing: 2,
  rowCount: 3,
  kind: "raw",
  columnMetadata: [
    { name: "COMP1", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: ["", "NA"], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "COMP2", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: ["", "NA"], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "LIKE1", label: null, column_type: "numeric", scale_type: "ordinal", missing_markers: ["", "NA"], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "LIKE2", label: null, column_type: "numeric", scale_type: "ordinal", missing_markers: ["", "NA"], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "TEXT1", label: null, column_type: "text", scale_type: "nominal", missing_markers: ["", "NA"], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "CONST1", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: ["", "NA"], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
};

describe("data workspace helpers", () => {
  it("summarizes data quality for researcher-facing cards", () => {
    const summary = dataQualitySummary(dataset);
    expect(summary.rows).toBe(3);
    expect(summary.variables).toBe(6);
    expect(summary.missingCells).toBe(2);
    expect(summary.numericVariables).toBe(5);
    expect(summary.nonnumericVariables).toBe(1);
    expect(summary.constantColumns).toContain("CONST1");
    expect(summary.missingHeavyColumns).toContain("LIKE2");
    expect(summary.sampleReady).toBe(false);
  });

  it("detects construct-ready variable prefix groups", () => {
    expect(detectPrefixGroups(dataset.columns)).toEqual([
      { prefix: "COMP", indicators: ["COMP1", "COMP2"] },
      { prefix: "LIKE", indicators: ["LIKE1", "LIKE2"] },
    ]);
  });

  it("filters visible preview columns by search and scale", () => {
    expect(filteredColumns(dataset, "comp", "all")).toEqual(["COMP1", "COMP2"]);
    expect(filteredColumns(dataset, "", "ordinal")).toEqual(["LIKE1", "LIKE2"]);
    expect(filteredColumns(dataset, "", "nonnumeric")).toEqual(["TEXT1"]);
    expect(filteredColumns(dataset, "", "missing_heavy")).toEqual(["LIKE2"]);
  });
});
