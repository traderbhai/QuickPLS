import type { Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, ConstructData, Dataset } from "../types";
import {
  nativeCtaPlsCombinationCount,
  nativeCtaPlsEligibleBlocks,
  nativeCtaPlsSetupAssessment,
} from "./nativeCtaPls";

const settings = (patch: Partial<AnalysisUiSettings> = {}): AnalysisUiSettings => ({
  method: "cta_pls",
  weightingScheme: "path",
  preprocessing: "standardized",
  tolerance: 1e-7,
  maxIterations: 3_000,
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_813,
  workers: 1,
  confidenceLevel: 0.95,
  caseWeightColumn: null,
  ...patch,
});

const nodes = (indicatorCount = 5): Array<Node<ConstructData>> => [{
  id: "quality",
  position: { x: 0, y: 0 },
  data: {
    label: "Quality",
    shortName: "Q",
    mode: "formative",
    indicators: Array.from({ length: indicatorCount }, (_, index) => `q${index + 1}`),
  },
}];

const dataset = (rows: Dataset["rows"], rowCount = rows.length): Dataset => ({
  id: "cta-data",
  name: "cta.csv",
  columns: ["q1", "q2", "q3", "q4", "q5"],
  rows,
  rowCount,
  missing: 0,
  fingerprint: "sha256:cta",
  kind: "raw",
  columnMetadata: ["q1", "q2", "q3", "q4", "q5"].map((name) => ({
    name,
    label: null,
    column_type: "numeric" as const,
    scale_type: "continuous" as const,
    missing_markers: [],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: {},
  })),
});

describe("native CTA-PLS setup contract", () => {
  it("counts every four-indicator subset and all three frozen pairings", () => {
    expect(nativeCtaPlsCombinationCount(3)).toBe(0);
    expect(nativeCtaPlsCombinationCount(4)).toBe(1);
    expect(nativeCtaPlsCombinationCount(5)).toBe(5);
    expect(nativeCtaPlsEligibleBlocks(nodes())).toEqual([expect.objectContaining({
      constructId: "quality",
      indicators: ["q1", "q2", "q3", "q4", "q5"],
      quadruples: 5,
      tetrads: 15,
    })]);
  });

  it("accepts reflective or formative ordinary blocks and reports complete cases", () => {
    const assessment = nativeCtaPlsSetupAssessment(dataset([
      { q1: 1, q2: 2, q3: 3, q4: 4, q5: 5 },
      { q1: 2, q2: 4, q3: 4, q4: 7, q5: 8 },
      { q1: 4, q2: 5, q3: 7, q4: 9, q5: 12 },
      { q1: 7, q2: 8, q3: 8, q4: 14, q5: 15 },
    ]), nodes(), settings());
    expect(assessment).toMatchObject({ canRun: true, completeCases: 4 });
    expect(assessment.detail).toContain("5 four-indicator subsets and 15 descriptive tetrads");
  });

  it("fails closed for ineligible blocks, special constructs, PCA weighting, resampling, and case weights", () => {
    const special = nodes();
    special[0].data.semantic = "higher_order";
    const assessment = nativeCtaPlsSetupAssessment(
      dataset([{ q1: 1, q2: 2, q3: 3, q4: 4, q5: 5 }], 100),
      special,
      settings({ weightingScheme: "pca", bootstrapSamples: 999, caseWeightColumn: "q1" }),
    );
    expect(assessment.canRun).toBe(false);
    expect(assessment.blockers).toEqual(expect.arrayContaining([
      expect.stringContaining("four or more"),
      expect.stringContaining("interaction or higher-order"),
      expect.stringContaining("path or factor"),
      expect.stringContaining("case weights"),
      expect.stringContaining("resampling inference"),
    ]));
  });

  it("names missing, nonnumeric, constant, and too-small complete-case inputs", () => {
    const data = dataset([
      { q1: 1, q2: "bad", q3: 3, q4: 9, q5: 2 },
      { q1: 1, q2: null, q3: 4, q4: 10, q5: 3 },
    ]);
    data.columns = ["q1", "q2", "q3", "q4"];
    data.columnMetadata = data.columnMetadata?.map((column) => column.name === "q2"
      ? { ...column, column_type: "text", scale_type: "nominal" }
      : column);
    const assessment = nativeCtaPlsSetupAssessment(data, nodes(), settings());
    expect(assessment.blockers).toEqual(expect.arrayContaining([
      expect.stringContaining("q5 is absent"),
      expect.stringContaining("q2 is not numeric"),
      expect.stringContaining("0 remain"),
    ]));

    const constant = nativeCtaPlsSetupAssessment(dataset([
      { q1: 1, q2: 1, q3: 1, q4: 1, q5: 1 },
      { q1: 1, q2: 2, q3: 2, q4: 2, q5: 2 },
      { q1: 1, q2: 3, q3: 3, q4: 3, q5: 3 },
    ]), nodes(4), settings());
    expect(constant.blockers).toContain("CTA-PLS indicator q1 has zero variance after listwise deletion");
  });

  it("defers complete-data checks when only a fingerprinted preview is resident", () => {
    const assessment = nativeCtaPlsSetupAssessment(dataset([
      { q1: 1, q2: 2, q3: 3, q4: 4, q5: 5 },
    ], 120), nodes(4), settings());
    expect(assessment).toMatchObject({ canRun: true, completeCases: null });
    expect(assessment.detail).toContain("full fingerprinted dataset");
  });
});
