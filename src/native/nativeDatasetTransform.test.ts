import { describe, expect, it } from "vitest";
import type { Dataset } from "../types";
import {
  buildNativeDatasetTransformationSpecV2,
  changeNativeDatasetTransformKindV2,
  defaultNativeDatasetTransformDraftV2,
  nativeDatasetTransformAvailabilityReasonV2,
  nativeDatasetTransformationIssuesFromErrorV2,
  nativeDatasetTransformationScaleLabelV2,
} from "./nativeDatasetTransform";

const dataset: Dataset = {
  id: "dataset-source",
  name: "Survey",
  columns: ["score", "other", "segment"],
  rows: [
    { score: 1, other: 3, segment: "A" },
    { score: 4, other: 5, segment: "B" },
  ],
  rowCount: 2,
  missing: 0,
  fingerprint: "sha256:source",
  kind: "raw",
  columnMetadata: [
    { name: "score", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "other", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "segment", label: null, column_type: "text", scale_type: "nominal", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
};

describe("native Dataset Transformations V2 setup", () => {
  it("builds reverse-scale, arithmetic, aggregate, and dummy specs", () => {
    const base = defaultNativeDatasetTransformDraftV2(dataset, "score");
    expect(buildNativeDatasetTransformationSpecV2(dataset, base).spec).toEqual(expect.objectContaining({
      kind: "reverse_scale",
      source_column: "score",
      scale_min: 1,
      scale_max: 5,
    }));

    const arithmetic = changeNativeDatasetTransformKindV2(dataset, base, "arithmetic");
    expect(buildNativeDatasetTransformationSpecV2(dataset, {
      ...arithmetic,
      arithmeticRightKind: "constant",
      arithmeticConstant: "2.5",
      arithmeticOperator: "multiply",
    }).spec).toEqual(expect.objectContaining({
      kind: "arithmetic",
      left_column: "score",
      operator: "multiply",
      right: { kind: "constant", value: 2.5 },
    }));

    const aggregate = changeNativeDatasetTransformKindV2(dataset, base, "row_aggregate");
    expect(buildNativeDatasetTransformationSpecV2(dataset, {
      ...aggregate,
      aggregateSourceColumns: ["score", "other"],
      aggregateOperation: "sum",
      aggregateMissingPolicy: "available",
      aggregateMinimumNonMissing: "1",
    }).spec).toEqual(expect.objectContaining({
      kind: "row_aggregate",
      source_columns: ["score", "other"],
      operation: "sum",
      minimum_non_missing: 1,
    }));

    const dummy = changeNativeDatasetTransformKindV2(dataset, { ...base, sourceColumn: "segment" }, "dummy");
    expect(buildNativeDatasetTransformationSpecV2(dataset, { ...dummy, dummyMatchValue: "A" }).spec).toEqual(expect.objectContaining({
      kind: "dummy",
      source_column: "segment",
      match_value: "A",
    }));
  });

  it("builds typed recode mappings and all group rule variants", () => {
    const base = defaultNativeDatasetTransformDraftV2(dataset, "score");
    const recode = changeNativeDatasetTransformKindV2(dataset, base, "recode");
    expect(buildNativeDatasetTransformationSpecV2(dataset, {
      ...recode,
      recodeTargetType: "text",
      recodeTargetScale: "ordinal",
      recodeMappings: [{ source: "1", target: "Low" }, { source: "4", target: "High" }],
    }).spec).toEqual(expect.objectContaining({
      kind: "recode",
      mappings: [{ source: 1, target: "Low" }, { source: 4, target: "High" }],
      target_type: "text",
      target_scale: "ordinal",
    }));

    const group = changeNativeDatasetTransformKindV2(dataset, base, "group");
    const built = buildNativeDatasetTransformationSpecV2(dataset, {
      ...group,
      groupOutputType: "text",
      groupRules: [
        { kind: "values", output: "Low", label: "Low score", values: "1\n2", minimum: "", maximum: "", includeMinimum: true, includeMaximum: true },
        { kind: "numeric_range", output: "High", label: "High score", values: "", minimum: "3", maximum: "5", includeMinimum: true, includeMaximum: false },
      ],
    });
    expect(built.spec).toEqual(expect.objectContaining({
      kind: "group",
      rules: [
        expect.objectContaining({ kind: "values", output: "Low", values: [1, 2] }),
        expect.objectContaining({ kind: "numeric_range", output: "High", minimum: 3, maximum: 5, include_maximum: false }),
      ],
    }));
    expect(nativeDatasetTransformationScaleLabelV2(group)).toBe("Nominal");
  });

  it("blocks every unsafe workspace state with an actionable reason", () => {
    const base = { dataset, nativeDesktop: true, projectWritable: true, mutationsLocked: false, datasetResident: true };
    expect(nativeDatasetTransformAvailabilityReasonV2(base)).toBeNull();
    expect(nativeDatasetTransformAvailabilityReasonV2({ ...base, nativeDesktop: false })).toContain("installed Windows app");
    expect(nativeDatasetTransformAvailabilityReasonV2({ ...base, projectWritable: false })).toContain("read-only");
    expect(nativeDatasetTransformAvailabilityReasonV2({ ...base, mutationsLocked: true })).toContain("active calculation");
    expect(nativeDatasetTransformAvailabilityReasonV2({ ...base, dataset: { ...dataset, kind: "covariance" } })).toContain("raw-observation");
    expect(nativeDatasetTransformAvailabilityReasonV2({ ...base, datasetResident: false })).toContain("complete dataset");
  });

  it("preserves typed backend issues, including a JSON-serialized Tauri error", () => {
    const typed = { issues: [{ code: "source.not_numeric", field: "source", message: "Numeric input required.", row_index: 8 }] };
    expect(nativeDatasetTransformationIssuesFromErrorV2(typed)).toEqual(typed.issues);
    expect(nativeDatasetTransformationIssuesFromErrorV2(new Error(JSON.stringify(typed)))).toEqual(typed.issues);
  });
});
