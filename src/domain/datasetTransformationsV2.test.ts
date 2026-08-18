import { describe, expect, it } from "vitest";
import type { Dataset } from "../types";
import {
  applyDatasetTransformationV2,
  canonicalDatasetTransformationJsonV2,
  DatasetTransformationErrorV2,
  previewDatasetTransformationV2,
  type DatasetTransformationSpecV2,
} from "./datasetTransformationsV2";

const dataset = (): Dataset => ({
  id: "study-v1",
  name: "Study",
  kind: "raw",
  columns: ["item", "x", "y", "segment"],
  rows: [
    { item: 1, x: 2, y: 4, segment: "A" },
    { item: 3, x: null, y: 2, segment: "B" },
    { item: 5, x: 6, y: 0, segment: "A" },
  ],
  rowCount: 3,
  missing: 1,
  missingByColumn: { item: 0, x: 1, y: 0, segment: 0 },
  fingerprint: `sha256:${"a".repeat(64)}`,
  columnMetadata: [],
});

const options = { output_dataset_id: "study-v2", output_dataset_name: "Study - derived", created_at: "2026-08-14T10:00:00.000Z" };

describe("DatasetTransformationV2", () => {
  it("adds a constant or missing column immutably with zero-input lineage", async () => {
    const source = dataset();
    const before = structuredClone(source);
    const constant: DatasetTransformationSpecV2 = {
      kind: "add_column",
      target_column: "cohort",
      value: "pilot",
      target_type: "text",
      target_scale: "nominal",
      target_label: "Cohort",
      value_labels: {},
    };
    const preview = previewDatasetTransformationV2(source, constant);
    expect(preview).toMatchObject({ issues: [], input_columns: [], output_columns: ["cohort"] });
    expect(preview.rows[0]).toMatchObject({ output: "pilot", outputs: { cohort: "pilot" } });
    const mutation = await applyDatasetTransformationV2(source, constant, options);
    expect(mutation.dataset.rows.map((row) => row.cohort)).toEqual(["pilot", "pilot", "pilot"]);
    expect(mutation.lineage).toMatchObject({ spec: constant, input_columns: [], output_columns: ["cohort"], output_missing_count: 0 });
    expect(source).toEqual(before);

    const missing: DatasetTransformationSpecV2 = { kind: "add_column", target_column: "placeholder", value: null, target_type: "numeric", target_scale: "continuous" };
    const missingMutation = await applyDatasetTransformationV2(source, missing, { ...options, output_dataset_id: "study-v3" });
    expect(missingMutation.dataset.rows.map((row) => row.placeholder)).toEqual([null, null, null]);
    expect(missingMutation.lineage.output_missing_count).toBe(3);
  });

  it("cleans multiple columns atomically in one child with exact multi-output lineage", async () => {
    const source = dataset();
    const before = structuredClone(source);
    const spec: DatasetTransformationSpecV2 = {
      kind: "missing_markers",
      columns: [
        { source_column: "item", target_column: "item_clean", markers: [3], target_type: "numeric", target_scale: "continuous", target_label: "Clean item", value_labels: {} },
        { source_column: "segment", target_column: "segment_clean", markers: ["B"], target_type: "text", target_scale: "nominal", target_label: "Clean segment", value_labels: {} },
      ],
    };
    const preview = previewDatasetTransformationV2(source, spec);
    expect(preview).toMatchObject({
      issues: [],
      input_columns: ["item", "segment"],
      output_columns: ["item_clean", "segment_clean"],
      output_missing_count: 2,
    });
    expect(preview.rows.map((row) => row.outputs)).toEqual([
      { item_clean: 1, segment_clean: "A" },
      { item_clean: null, segment_clean: null },
      { item_clean: 5, segment_clean: "A" },
    ]);
    const mutation = await applyDatasetTransformationV2(source, spec, options);
    expect(mutation.dataset.columns).toEqual([...source.columns, "item_clean", "segment_clean"]);
    expect(mutation.dataset.rows.map((row) => [row.item_clean, row.segment_clean])).toEqual([[1, "A"], [null, null], [5, "A"]]);
    expect(mutation.lineage).toMatchObject({ spec, input_columns: ["item", "segment"], output_columns: ["item_clean", "segment_clean"], output_missing_count: 2 });
    expect(source).toEqual(before);
  });

  it("rejects ambiguous add-column and multi-marker declarations before materialization", () => {
    const badAdd = { kind: "add_column", target_column: "bad", value: "text", target_type: "numeric", target_scale: "continuous" } as DatasetTransformationSpecV2;
    expect(previewDatasetTransformationV2(dataset(), badAdd).issues).toContainEqual(expect.objectContaining({ code: "add_column.value_type_mismatch" }));
    expect(previewDatasetTransformationV2(dataset(), { kind: "add_column", target_column: " item ", value: null, target_type: "numeric", target_scale: "continuous" }).issues).toContainEqual(expect.objectContaining({ code: "target.whitespace" }));
    expect(previewDatasetTransformationV2(dataset(), { kind: "add_column", target_column: "ITEM", value: null, target_type: "numeric", target_scale: "continuous" }).issues).toContainEqual(expect.objectContaining({ code: "target.exists" }));
    const duplicateTargets: DatasetTransformationSpecV2 = {
      kind: "missing_markers",
      columns: [
        { source_column: "item", target_column: "clean", markers: [3], target_type: "numeric", target_scale: "continuous" },
        { source_column: "segment", target_column: "CLEAN", markers: ["B"], target_type: "text", target_scale: "nominal" },
      ],
    };
    expect(previewDatasetTransformationV2(dataset(), duplicateTargets).issues).toContainEqual(expect.objectContaining({ code: "target.duplicate" }));
    const typed = dataset();
    typed.columnMetadata = [{ name: "item", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} }];
    expect(previewDatasetTransformationV2(typed, {
      kind: "missing_markers",
      columns: [{ source_column: "item", target_column: "item_clean", markers: [3], target_type: "text", target_scale: "nominal" }],
    }).issues).toContainEqual(expect.objectContaining({ code: "missing_markers.metadata_mismatch" }));
    const whitespaceSource = { ...duplicateTargets, columns: [{ ...duplicateTargets.columns[0], source_column: " item ", target_column: "item_clean" }] } as DatasetTransformationSpecV2;
    expect(previewDatasetTransformationV2(dataset(), whitespaceSource).issues).toContainEqual(expect.objectContaining({ code: "source.whitespace" }));
    expect(previewDatasetTransformationV2(dataset(), { kind: "missing_markers", columns: [] }).issues).toContainEqual(expect.objectContaining({ code: "missing_markers.columns_required" }));
  });

  it("previews and applies a reversible scale without mutating its source", async () => {
    const source = dataset();
    const spec: DatasetTransformationSpecV2 = { kind: "reverse_scale", source_column: "item", target_column: "item_r", scale_min: 1, scale_max: 5 };
    const preview = previewDatasetTransformationV2(source, spec);
    expect(preview).toMatchObject({
      issues: [],
      target_column: "item_r",
      output_columns: ["item_r"],
      output_missing_count: 0,
      rows: [{ output: 5 }, { output: 3 }, { output: 1 }],
    });
    expect(preview.rows.map((row) => row.outputs)).toEqual([{ item_r: 5 }, { item_r: 3 }, { item_r: 1 }]);
    const mutation = await applyDatasetTransformationV2(source, spec, options);
    expect(mutation.dataset.rows.map((row) => row.item_r)).toEqual([5, 3, 1]);
    expect(mutation.dataset.fingerprint).toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(mutation.lineage).toMatchObject({ source_dataset_id: "study-v1", output_dataset_id: "study-v2", input_columns: ["item"], output_columns: ["item_r"] });
    expect(source.columns).toEqual(["item", "x", "y", "segment"]);
    expect(source.rows[0]).not.toHaveProperty("item_r");
  });

  it("supports exact recoding, dummy variables, and deterministic group derivation", async () => {
    const recode: DatasetTransformationSpecV2 = {
      kind: "recode", source_column: "segment", target_column: "segment_label",
      mappings: [{ source: "A", target: "Treatment" }, { source: "B", target: "Control" }],
      unmapped: "error", target_type: "text", target_scale: "nominal",
    };
    expect((await applyDatasetTransformationV2(dataset(), recode, options)).dataset.rows.map((row) => row.segment_label)).toEqual(["Treatment", "Control", "Treatment"]);
    expect(previewDatasetTransformationV2(dataset(), { kind: "dummy", source_column: "segment", match_value: "A", missing_policy: "missing", target_column: "is_a" }).rows.map((row) => row.output)).toEqual([1, 0, 1]);
    const grouped: DatasetTransformationSpecV2 = {
      kind: "group", source_column: "item", target_column: "item_group", unmatched: "error",
      rules: [
        { kind: "numeric_range", output: "low", minimum: 1, maximum: 2, include_minimum: true, include_maximum: true },
        { kind: "numeric_range", output: "high", minimum: 2, maximum: 5, include_minimum: false, include_maximum: true },
      ],
    };
    expect(previewDatasetTransformationV2(dataset(), grouped).rows.map((row) => row.output)).toEqual(["low", "high", "high"]);
  });

  it("implements arithmetic, sum, and available-case mean with exact missing behavior", () => {
    expect(previewDatasetTransformationV2(dataset(), {
      kind: "arithmetic", left_column: "x", right: { kind: "column", column: "y" }, operator: "multiply", target_column: "xy",
    }).rows.map((row) => row.output)).toEqual([8, null, 0]);
    expect(previewDatasetTransformationV2(dataset(), {
      kind: "row_aggregate", source_columns: ["x", "y"], operation: "sum", missing_policy: "propagate", target_column: "total",
    }).rows.map((row) => row.output)).toEqual([6, null, 6]);
    expect(previewDatasetTransformationV2(dataset(), {
      kind: "row_aggregate", source_columns: ["x", "y"], operation: "mean", missing_policy: "available", minimum_non_missing: 1, target_column: "average",
    }).rows.map((row) => row.output)).toEqual([3, 2, 3]);
  });

  it("standardizes once with sample n-1 deviation, preserves missing cells, and binds exact lineage", async () => {
    const source = dataset();
    const sourceBefore = structuredClone(source);
    const spec: DatasetTransformationSpecV2 = {
      kind: "standardize",
      source_column: "item",
      target_column: "z_item",
      denominator: "sample_n_minus_one",
      target_label: "Standardized item",
    };
    const preview = previewDatasetTransformationV2(source, spec);
    expect(preview.issues).toEqual([]);
    expect(preview.rows.map((row) => row.output)).toEqual([-1, 0, 1]);

    const mutation = await applyDatasetTransformationV2(source, spec, options);
    expect(mutation.dataset.rows.map((row) => row.z_item)).toEqual([-1, 0, 1]);
    expect(mutation.dataset.columns).toEqual([...source.columns, "z_item"]);
    expect(mutation.dataset.columnMetadata?.at(-1)).toMatchObject({ name: "z_item", column_type: "numeric", scale_type: "continuous" });
    expect(mutation.lineage).toMatchObject({ spec, input_columns: ["item"], output_columns: ["z_item"], output_missing_count: 0 });
    expect(source).toEqual(sourceBefore);

    const missingSpec: DatasetTransformationSpecV2 = { kind: "standardize", source_column: "x", target_column: "z_x", denominator: "sample_n_minus_one" };
    const missingPreview = previewDatasetTransformationV2(source, missingSpec);
    expect(missingPreview.issues).toEqual([]);
    expect(missingPreview.output_missing_count).toBe(1);
    expect(missingPreview.rows[0].output).toBeCloseTo(-Math.SQRT1_2, 12);
    expect(missingPreview.rows[1].output).toBeNull();
    expect(missingPreview.rows[2].output).toBeCloseTo(Math.SQRT1_2, 12);

    const tiny = { ...source, rows: source.rows.map((row, index) => ({ ...row, item: [1e-150, 3e-150, 5e-150][index] })) };
    const tinyOutputs = previewDatasetTransformationV2(tiny, spec).rows.map((row) => row.output as number);
    expect(tinyOutputs[0]).toBeCloseTo(-1, 12);
    expect(tinyOutputs[1]).toBeCloseTo(0, 12);
    expect(tinyOutputs[2]).toBeCloseTo(1, 12);
  });

  it("rejects invalid standardization inputs and denominator without emitting a dataset", async () => {
    const spec = (source_column = "x"): DatasetTransformationSpecV2 => ({ kind: "standardize", source_column, target_column: "z_x", denominator: "sample_n_minus_one" });
    const withValues = (values: unknown[]): Dataset => ({
      ...dataset(),
      columns: ["x"],
      rows: values.map((x) => ({ x })) as Dataset["rows"],
      rowCount: values.length,
      missing: values.filter((value) => value == null).length,
      missingByColumn: { x: values.filter((value) => value == null).length },
      columnMetadata: [],
    });

    expect(previewDatasetTransformationV2(withValues([null, null]), spec()).issues).toContainEqual(expect.objectContaining({ code: "standardize.all_missing" }));
    expect(previewDatasetTransformationV2(withValues([null, 4]), spec()).issues).toContainEqual(expect.objectContaining({ code: "standardize.insufficient_observations" }));
    expect(previewDatasetTransformationV2(withValues([4, 4, 4]), spec()).issues).toContainEqual(expect.objectContaining({ code: "standardize.zero_variance" }));
    expect(previewDatasetTransformationV2(withValues([1, "not-a-number", 3]), spec()).issues).toContainEqual(expect.objectContaining({ code: "source.not_numeric", row_index: 1 }));
    expect(previewDatasetTransformationV2(withValues([1e308, -1e308]), spec()).issues).toContainEqual(expect.objectContaining({ code: "standardize.non_finite" }));
    const badDenominator = { ...spec(), denominator: "population_n" } as unknown as DatasetTransformationSpecV2;
    expect(previewDatasetTransformationV2(withValues([1, 2, 3]), badDenominator).issues).toContainEqual(expect.objectContaining({ code: "standardize.denominator_invalid" }));
    await expect(applyDatasetTransformationV2(withValues([4, 4, 4]), spec(), options)).rejects.toBeInstanceOf(DatasetTransformationErrorV2);
  });

  it("fails closed for nonresident rows, overwrite, invalid numbers, division by zero, and overlapping rules", async () => {
    const notResident = { ...dataset(), rowCount: 10 };
    expect(previewDatasetTransformationV2(notResident, { kind: "dummy", source_column: "segment", match_value: "A", missing_policy: "zero", target_column: "flag" }).issues).toContainEqual(expect.objectContaining({ code: "dataset.rows_not_resident" }));
    expect(previewDatasetTransformationV2(dataset(), { kind: "reverse_scale", source_column: "item", target_column: "x", scale_min: 1, scale_max: 5 }).issues).toContainEqual(expect.objectContaining({ code: "target.exists" }));
    const nonnumeric = { ...dataset(), rows: [{ ...dataset().rows[0], x: "not-a-number" }, ...dataset().rows.slice(1)] };
    expect(previewDatasetTransformationV2(nonnumeric, { kind: "arithmetic", left_column: "x", right: { kind: "constant", value: 2 }, operator: "add", target_column: "z" }).issues).toContainEqual(expect.objectContaining({ code: "source.not_numeric", row_index: 0 }));
    const divide = { kind: "arithmetic", left_column: "x", right: { kind: "column", column: "y" }, operator: "divide", target_column: "ratio" } as const;
    expect(previewDatasetTransformationV2(dataset(), divide).issues).toContainEqual(expect.objectContaining({ code: "arithmetic.division_by_zero", row_index: 2 }));
    await expect(applyDatasetTransformationV2(dataset(), divide, options)).rejects.toBeInstanceOf(DatasetTransformationErrorV2);
    expect(previewDatasetTransformationV2(dataset(), {
      kind: "group", source_column: "item", target_column: "g", unmatched: "missing",
      rules: [
        { kind: "numeric_range", output: 1, minimum: 1, maximum: 4, include_minimum: true, include_maximum: true },
        { kind: "numeric_range", output: 2, minimum: 3, maximum: 5, include_minimum: true, include_maximum: true },
      ],
    }).issues).toContainEqual(expect.objectContaining({ code: "group.rule_overlap", row_index: 1 }));
  });

  it("is deterministic under object-key order and changes identity when the scientific specification changes", async () => {
    const first: DatasetTransformationSpecV2 = { kind: "reverse_scale", source_column: "item", target_column: "item_r", scale_min: 1, scale_max: 5 };
    const reordered = { target_column: "item_r", scale_max: 5, kind: "reverse_scale", scale_min: 1, source_column: "item" } as DatasetTransformationSpecV2;
    expect(canonicalDatasetTransformationJsonV2(first)).toBe(canonicalDatasetTransformationJsonV2(reordered));
    const one = await applyDatasetTransformationV2(dataset(), first, options);
    const two = await applyDatasetTransformationV2(dataset(), reordered, options);
    expect(two).toEqual(one);
    const changed = await applyDatasetTransformationV2(dataset(), { ...first, scale_max: 7 }, options);
    expect(changed.lineage.spec_sha256).not.toBe(one.lineage.spec_sha256);
    expect(changed.dataset.fingerprint).not.toBe(one.dataset.fingerprint);
  });
});
