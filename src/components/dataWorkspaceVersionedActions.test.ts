import { readFileSync } from "node:fs";
import { describe, expect, it, vi } from "vitest";
import type { DatasetTransformationSpecV2 } from "../domain/datasetTransformationsV2";
import type { Dataset, DatasetVersionMutation } from "../types";
import { executeDataWorkspaceVersionedAction, sortDataWorkspaceViewRows, type DataWorkspaceVersionedAction } from "./dataWorkspaceVersionedActions";

const sourceDataset = (): Dataset => ({
  id: "source-dataset",
  name: "Source",
  columns: ["score", "group"],
  rows: [
    { score: 1, group: "A" },
    { score: 2, group: "B" },
  ],
  missing: 0,
  rowCount: 2,
  kind: "raw",
  fingerprint: "sha256:source",
  columnMetadata: [
    { name: "score", label: "Score", column_type: "numeric", scale_type: "continuous", missing_markers: [""], theoretical_min: null, theoretical_max: null, value_labels: {} },
    { name: "group", label: "Group", column_type: "text", scale_type: "nominal", missing_markers: [""], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
});

const recodeMutation = (): DatasetVersionMutation => ({
  dataset: {
    ...sourceDataset(),
    id: "recode-version",
    columns: ["score", "group", "group_recoded"],
    rows: [
      { score: 1, group: "A", group_recoded: "1" },
      { score: 2, group: "B", group_recoded: "B" },
    ],
  },
  version: {
    datasetId: "recode-version",
    parentDatasetId: "source-dataset",
    operation: "recode",
    createdAt: "2026-08-15T12:00:00.000Z",
    summary: "Recoded group into group_recoded",
    sourceColumn: "group",
    targetColumn: "group_recoded",
  },
});

const missingValueMutation = (): DatasetVersionMutation => ({
  dataset: {
    ...sourceDataset(),
    id: "missing-version",
    name: "Source (missing-cleaned 1 column)",
    fingerprint: "sha256:output",
    columns: ["score", "group", "group_clean"],
    rows: [
      { score: 1, group: "A", group_clean: "A" },
      { score: 2, group: "B", group_clean: null },
    ],
  },
  version: {
    datasetId: "missing-version",
    parentDatasetId: "source-dataset",
    operation: "transform",
    createdAt: "2026-08-15T12:00:00.000Z",
    summary: "Derived group_clean from group",
    sourceColumn: "group",
    targetColumn: "group_clean",
    transformation: {
      schema_version: 2,
      engine: "qpls.dataset_transform.v2",
      operation_id: "dataset_transform:missing",
      source_dataset_id: "source-dataset",
      source_dataset_fingerprint: "sha256:source",
      output_dataset_id: "missing-version",
      output_dataset_fingerprint: "sha256:output",
      created_at: "2026-08-15T12:00:00.000Z",
      spec_sha256: "spec-sha256",
      spec: {
        kind: "missing_markers",
        columns: [{
          source_column: "group",
          target_column: "group_clean",
          markers: ["B"],
          target_type: "text",
          target_scale: "nominal",
          target_label: "Missing-cleaned Group",
          value_labels: {},
        }],
      },
      input_columns: ["group"],
      output_columns: ["group_clean"],
      source_row_count: 2,
      output_missing_count: 1,
    },
  },
});

const addColumnMutation = (): DatasetVersionMutation => ({
  dataset: {
    ...sourceDataset(),
    id: "add-version",
    name: "Source (added cohort)",
    fingerprint: "sha256:add",
    columns: ["score", "group", "cohort"],
    rows: [
      { score: 1, group: "A", cohort: "pilot" },
      { score: 2, group: "B", cohort: "pilot" },
    ],
  },
  version: {
    datasetId: "add-version",
    parentDatasetId: "source-dataset",
    operation: "transform",
    createdAt: "2026-08-15T12:00:00.000Z",
    summary: "Added cohort",
    sourceColumn: null,
    targetColumn: "cohort",
    transformation: {
      schema_version: 2,
      engine: "qpls.dataset_transform.v2",
      operation_id: "dataset_transform:add",
      source_dataset_id: "source-dataset",
      source_dataset_fingerprint: "sha256:source",
      output_dataset_id: "add-version",
      output_dataset_fingerprint: "sha256:add",
      created_at: "2026-08-15T12:00:00.000Z",
      spec_sha256: "add-spec-sha256",
      spec: {
        kind: "add_column",
        target_column: "cohort",
        value: "pilot",
        target_type: "text",
        target_scale: "nominal",
        target_label: "cohort",
        value_labels: {},
      },
      input_columns: [],
      output_columns: ["cohort"],
      source_row_count: 2,
      output_missing_count: 0,
    },
  },
});

const multiMissingValueMutation = (): DatasetVersionMutation => {
  const mutation = missingValueMutation();
  mutation.dataset = {
    ...sourceDataset(),
    id: "missing-version",
    name: "Source (missing-cleaned 2 columns)",
    fingerprint: "sha256:output",
    columns: ["score", "group", "score_clean", "group_clean"],
    rows: [
      { score: 1, group: "A", score_clean: null, group_clean: "A" },
      { score: 2, group: "B", score_clean: 2, group_clean: null },
    ],
  };
  mutation.version.summary = "Derived score_clean, group_clean from score, group";
  mutation.version.sourceColumn = "score";
  mutation.version.targetColumn = "score_clean";
  mutation.version.transformation!.spec = {
    kind: "missing_markers",
    columns: [
      { source_column: "score", target_column: "score_clean", markers: [1, "B"], target_type: "numeric", target_scale: "continuous", target_label: "Missing-cleaned Score", value_labels: {} },
      { source_column: "group", target_column: "group_clean", markers: ["1", "B"], target_type: "text", target_scale: "nominal", target_label: "Missing-cleaned Group", value_labels: {} },
    ],
  };
  mutation.version.transformation!.input_columns = ["score", "group"];
  mutation.version.transformation!.output_columns = ["score_clean", "group_clean"];
  mutation.version.transformation!.output_missing_count = 2;
  return mutation;
};

const standardizeMutation = (): DatasetVersionMutation => ({
  dataset: {
    ...sourceDataset(),
    id: "standardize-version",
    name: "Source (standardized score)",
    fingerprint: "sha256:standardized",
    columns: ["score", "group", "z_score"],
    rows: [
      { score: 1, group: "A", z_score: -Math.SQRT1_2 },
      { score: 2, group: "B", z_score: Math.SQRT1_2 },
    ],
  },
  version: {
    datasetId: "standardize-version",
    parentDatasetId: "source-dataset",
    operation: "transform",
    createdAt: "2026-08-15T12:00:00.000Z",
    summary: "Derived z_score from score",
    sourceColumn: "score",
    targetColumn: "z_score",
    transformation: {
      schema_version: 2,
      engine: "qpls.dataset_transform.v2",
      operation_id: "dataset_transform:standardize",
      source_dataset_id: "source-dataset",
      source_dataset_fingerprint: "sha256:source",
      output_dataset_id: "standardize-version",
      output_dataset_fingerprint: "sha256:standardized",
      created_at: "2026-08-15T12:00:00.000Z",
      spec_sha256: "standardize-spec-sha256",
      spec: {
        kind: "standardize",
        source_column: "score",
        target_column: "z_score",
        denominator: "sample_n_minus_one",
        target_label: "Standardized score",
      },
      input_columns: ["score"],
      output_columns: ["z_score"],
      source_row_count: 2,
      output_missing_count: 0,
    },
  },
});

const numericSourceDataset = (): Dataset => {
  const source = sourceDataset();
  return {
    ...source,
    columns: ["score", "other", "group"],
    rows: [
      { score: 1, other: 3, group: "A" },
      { score: 2, other: 4, group: "B" },
    ],
    columnMetadata: [
      source.columnMetadata![0],
      { name: "other", label: "Other", column_type: "numeric", scale_type: "continuous", missing_markers: [""], theoretical_min: null, theoretical_max: null, value_labels: {} },
      source.columnMetadata![1],
    ],
  };
};

const transformationMutationFor = (source: Dataset, spec: DatasetTransformationSpecV2): DatasetVersionMutation => {
  const inputColumns = spec.kind === "add_column" ? []
    : spec.kind === "missing_markers" ? spec.columns.map((column) => column.source_column)
      : spec.kind === "arithmetic" ? spec.right.kind === "column" ? [spec.left_column, spec.right.column] : [spec.left_column]
        : spec.kind === "row_aggregate" ? [...spec.source_columns]
          : [spec.source_column];
  const outputColumns = spec.kind === "missing_markers" ? spec.columns.map((column) => column.target_column) : [spec.target_column];
  const outputId = `derived-${spec.kind}`;
  return {
    dataset: {
      ...source,
      id: outputId,
      name: `Derived ${spec.kind}`,
      columns: [...source.columns, ...outputColumns],
      rows: source.rows.map((row) => ({ ...row, ...Object.fromEntries(outputColumns.map((column) => [column, null])) })),
      fingerprint: `sha256:${outputId}`,
    },
    version: {
      datasetId: outputId,
      parentDatasetId: source.id,
      operation: "transform",
      createdAt: "2026-08-15T13:00:00.000Z",
      summary: `Derived ${outputColumns.join(", ")}`,
      sourceColumn: inputColumns[0] ?? null,
      targetColumn: outputColumns[0] ?? null,
      transformation: {
        schema_version: 2,
        engine: "qpls.dataset_transform.v2",
        operation_id: `dataset_transform:${spec.kind}`,
        source_dataset_id: source.id,
        source_dataset_fingerprint: source.fingerprint!,
        output_dataset_id: outputId,
        output_dataset_fingerprint: `sha256:${outputId}`,
        created_at: "2026-08-15T13:00:00.000Z",
        spec_sha256: `sha256:spec-${spec.kind}`,
        spec,
        input_columns: inputColumns,
        output_columns: outputColumns,
        source_row_count: source.rows.length,
        output_missing_count: source.rows.length,
      },
    },
  };
};

describe("DataWorkspace immutable transformation gate", () => {
  it("routes all five legacy mutators through the version gate and exposes an accessible status", () => {
    const workspace = readFileSync("src/components/DataWorkspace.tsx", "utf8");

    for (const action of [
      '{ kind: "sort", column: detail?.column ?? (selectedColumn || null), direction }',
      'runVersionedAction({ kind: "add-column", name, value })',
      '{ kind: "recode", column: recodeColumn, from, to }',
      '{ kind: "missing-values", column: detail?.column ?? null, markers: detail?.markers ?? "" }',
      '{ kind: "z-score", column: detail?.column ?? (selectedColumn || null), outputName: detail?.outputName ?? "" }',
    ]) {
      expect(workspace).toContain(action.startsWith("runVersionedAction") ? action : `runVersionedAction(${action})`);
    }
    expect(workspace).toContain('role="status" aria-live="polite"');
    expect(workspace).not.toContain("dataset.rows.map((row)");
    expect(workspace).not.toContain("[...dataset.rows].sort(");
    expect(workspace).not.toContain("next[column] = null");
    expect(workspace).toContain('aria-label="Preview row order"');
    expect(workspace).toContain('aria-label="Immutable derived variable controls"');
    expect(workspace).toContain('<option value="reverse">Reverse scale</option>');
    expect(workspace).toContain('<option value="multiply">Multiply variables</option>');
    expect(workspace).toContain('<option value="divide">Divide variables</option>');
    expect(workspace).toContain('<option value="sum">Row-wise sum</option>');
    expect(workspace).toContain('<option value="mean">Row-wise average</option>');
    expect(workspace).toContain('<option value="dummy">Dummy variable</option>');
    expect(workspace).toContain('<option value="group">Group values</option>');
    expect(workspace).toContain("onClick={createDerivedVariable}");
    expect(workspace).toContain("aria-sort={activeSort}");
    expect(workspace).toContain("useEffect(() => { setRowSort(null); }, [dataset.id]);");
  });

  it("sorts a copied presentation index stably while retaining source row order and identity", () => {
    const rows: Dataset["rows"] = [
      { score: 2, label: "second" },
      { score: null, label: "missing" },
      { score: 1, label: "first-a" },
      { score: 1, label: "first-b" },
    ];
    const before = structuredClone(rows);

    const ascending = sortDataWorkspaceViewRows(rows, { column: "score", direction: "asc" });
    const descending = sortDataWorkspaceViewRows(rows, { column: "score", direction: "desc" });

    expect(ascending.map((item) => item.sourceIndex)).toEqual([2, 3, 0, 1]);
    expect(descending.map((item) => item.sourceIndex)).toEqual([0, 2, 3, 1]);
    expect(rows).toEqual(before);
    expect(ascending.every((item) => item.row === rows[item.sourceIndex])).toBe(true);
  });

  it("returns a view-only sort instruction without invoking dataset or version services", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const createRecodeVersion = vi.fn();
    const createTransformationVersion = vi.fn();
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion,
      createTransformationVersion,
      commitVersion,
    }, { kind: "sort", column: "score", direction: "desc" });

    expect(result).toEqual({
      kind: "view-only",
      message: "Preview rows sorted by score (descending). The scientific dataset and saved row order were not changed.",
      selectedColumn: "score",
      sort: { column: "score", direction: "desc" },
    });
    expect(createRecodeVersion).not.toHaveBeenCalled();
    expect(createTransformationVersion).not.toHaveBeenCalled();
    expect(commitVersion).not.toHaveBeenCalled();
    expect(dataset).toEqual(sourceBefore);
  });

  it("blocks preview sorting when no real dataset column is selected", async () => {
    const commitVersion = vi.fn();
    const result = await executeDataWorkspaceVersionedAction({
      dataset: sourceDataset(),
      nativeDesktop: false,
      createRecodeVersion: vi.fn(),
      createTransformationVersion: vi.fn(),
      commitVersion,
    }, { kind: "sort", column: "unknown", direction: "asc" });

    expect(result).toMatchObject({ kind: "blocked", message: expect.stringContaining("not changed") });
    expect(commitVersion).not.toHaveBeenCalled();
  });

  it("creates and activates a distinct native recode version with provenance without changing source rows", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const mutation = recodeMutation();
    const createRecodeVersion = vi.fn().mockResolvedValue(mutation);
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion,
      createTransformationVersion: vi.fn(),
      commitVersion,
    }, { kind: "recode", column: "group", from: "A", to: "1" });

    expect(result).toMatchObject({ kind: "committed", selectedColumn: "group_recoded" });
    expect(createRecodeVersion).toHaveBeenCalledWith("source-dataset", {
      sourceColumn: "group",
      targetColumn: "group_recoded",
      targetLabel: "Recoded Group",
      targetType: "text",
      targetScale: "nominal",
      mappings: [{ source: "A", target: "1" }],
      unmapped: "keep_original",
    });
    expect(commitVersion).toHaveBeenCalledWith(mutation);
    expect(dataset).toEqual(sourceBefore);
    expect(dataset.rows).toEqual(sourceBefore.rows);
  });

  it("fails closed in browser preview and never calls or commits the native recode service", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const createRecodeVersion = vi.fn();
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: false,
      createRecodeVersion,
      createTransformationVersion: vi.fn(),
      commitVersion,
    }, { kind: "recode", column: "group", from: "A", to: "1" });

    expect(result).toMatchObject({ kind: "blocked", message: expect.stringContaining("not changed") });
    expect(createRecodeVersion).not.toHaveBeenCalled();
    expect(commitVersion).not.toHaveBeenCalled();
    expect(dataset).toEqual(sourceBefore);
  });

  it.each<DataWorkspaceVersionedAction>([
    { kind: "add-column", name: "cohort", value: "pilot" },
    { kind: "missing-values", column: null, markers: "NA" },
    { kind: "z-score", column: null, outputName: "z_score" },
    { kind: "reverse-scale", column: "score", minimum: "1", maximum: "5", outputName: "score_r" },
    { kind: "arithmetic", leftColumn: "score", right: { kind: "column", column: "group" }, operator: "multiply", outputName: "product" },
    { kind: "row-aggregate", columns: ["score", "group"], operation: "sum", missingPolicy: "propagate", minimumNonMissing: "", outputName: "total" },
    { kind: "dummy", column: "group", matchValue: "A", missingPolicy: "missing", outputName: "is_a" },
    { kind: "group-values", column: "group", rules: "A = First; B = Second", unmatched: "missing", outputName: "grouped" },
  ])("fails closed for browser $kind mutations and leaves the source exact", async (action) => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const createRecodeVersion = vi.fn();
    const createTransformationVersion = vi.fn();
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: false,
      createRecodeVersion,
      createTransformationVersion,
      commitVersion,
    }, action);

    expect(result).toMatchObject({ kind: "blocked", message: expect.stringContaining("not changed") });
    expect(createRecodeVersion).not.toHaveBeenCalled();
    expect(createTransformationVersion).not.toHaveBeenCalled();
    expect(commitVersion).not.toHaveBeenCalled();
    expect(dataset).toEqual(sourceBefore);
    expect(dataset.rows).toEqual(sourceBefore.rows);
  });

  it("adds one constant column through one immutable native transformation", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const mutation = addColumnMutation();
    const createTransformationVersion = vi.fn().mockResolvedValue(mutation);
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion: vi.fn(),
      createTransformationVersion,
      commitVersion,
    }, { kind: "add-column", name: "cohort", value: "pilot" });

    expect(result).toMatchObject({ kind: "committed", selectedColumn: "cohort", mutation });
    expect(createTransformationVersion).toHaveBeenCalledWith("source-dataset", mutation.version.transformation!.spec, "Source (added cohort)");
    expect(createTransformationVersion).toHaveBeenCalledOnce();
    expect(commitVersion).toHaveBeenCalledOnce();
    expect(dataset).toEqual(sourceBefore);
  });

  it.each<Array<{ label: string; action: DataWorkspaceVersionedAction; expected: DatasetTransformationSpecV2 }>>([
    [{ label: "reverse scale", action: { kind: "reverse-scale", column: "score", minimum: "1", maximum: "5", outputName: "score reversed" }, expected: { kind: "reverse_scale", source_column: "score", target_column: "score_reversed", scale_min: 1, scale_max: 5, target_label: "Reverse-scaled score" } }],
    [{ label: "addition", action: { kind: "arithmetic", leftColumn: "score", right: { kind: "column", column: "other" }, operator: "add", outputName: "" }, expected: { kind: "arithmetic", left_column: "score", right: { kind: "column", column: "other" }, operator: "add", target_column: "score_add", target_label: "add derived from score" } }],
    [{ label: "multiplication", action: { kind: "arithmetic", leftColumn: "score", right: { kind: "column", column: "other" }, operator: "multiply", outputName: "product" }, expected: { kind: "arithmetic", left_column: "score", right: { kind: "column", column: "other" }, operator: "multiply", target_column: "product", target_label: "multiply derived from score" } }],
    [{ label: "division", action: { kind: "arithmetic", leftColumn: "score", right: { kind: "column", column: "other" }, operator: "divide", outputName: "ratio" }, expected: { kind: "arithmetic", left_column: "score", right: { kind: "column", column: "other" }, operator: "divide", target_column: "ratio", target_label: "divide derived from score" } }],
    [{ label: "row sum", action: { kind: "row-aggregate", columns: ["score", "other"], operation: "sum", missingPolicy: "propagate", minimumNonMissing: "", outputName: "total" }, expected: { kind: "row_aggregate", source_columns: ["score", "other"], operation: "sum", missing_policy: "propagate", target_column: "total", target_label: "Row-wise sum" } }],
    [{ label: "available-case average", action: { kind: "row-aggregate", columns: ["score", "other"], operation: "mean", missingPolicy: "available", minimumNonMissing: "1", outputName: "average" }, expected: { kind: "row_aggregate", source_columns: ["score", "other"], operation: "mean", missing_policy: "available", minimum_non_missing: 1, target_column: "average", target_label: "Row-wise average" } }],
    [{ label: "dummy", action: { kind: "dummy", column: "group", matchValue: "A", missingPolicy: "missing", outputName: "is a" }, expected: { kind: "dummy", source_column: "group", match_value: "A", missing_policy: "missing", target_column: "is_a", target_label: "group equals A" } }],
    [{ label: "value groups", action: { kind: "group-values", column: "group", rules: "A = Treatment; B = Control", unmatched: "error", outputName: "arm" }, expected: { kind: "group", source_column: "group", rules: [{ kind: "values", output: "Treatment", values: ["A"], label: "Treatment" }, { kind: "values", output: "Control", values: ["B"], label: "Control" }], unmatched: "error", target_column: "arm", target_label: "Grouped group" } }],
  ])("routes $label through exactly one generic native transformation and one validated commit", async ({ action, expected }) => {
    const dataset = numericSourceDataset();
    const sourceBefore = structuredClone(dataset);
    const createTransformationVersion = vi.fn(async (_datasetId: string, spec: DatasetTransformationSpecV2) => transformationMutationFor(dataset, spec));
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion: vi.fn(),
      createTransformationVersion,
      commitVersion,
    }, action);

    expect(result).toMatchObject({ kind: "committed", selectedColumn: expected.kind === "missing_markers" ? expected.columns[0].target_column : expected.target_column });
    expect(createTransformationVersion).toHaveBeenCalledOnce();
    expect(createTransformationVersion.mock.calls[0][0]).toBe(dataset.id);
    expect(createTransformationVersion.mock.calls[0][1]).toEqual(expected);
    expect(commitVersion).toHaveBeenCalledOnce();
    expect(dataset).toEqual(sourceBefore);
  });

  it("rejects an in-place or provenance-free recode response before store activation", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const invalid = recodeMutation();
    invalid.dataset.id = dataset.id;
    invalid.version.datasetId = dataset.id;
    invalid.version.parentDatasetId = null;
    const commitVersion = vi.fn();

    await expect(executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion: vi.fn().mockResolvedValue(invalid),
      createTransformationVersion: vi.fn(),
      commitVersion,
    }, { kind: "recode", column: "group", from: "A", to: "1" })).rejects.toThrow("did not prove a new immutable dataset version");

    expect(commitVersion).not.toHaveBeenCalled();
    expect(dataset).toEqual(sourceBefore);
  });

  it("creates a single-column missing-cleaned dataset through the native transformation service", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const mutation = missingValueMutation();
    const createTransformationVersion = vi.fn().mockResolvedValue(mutation);
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion: vi.fn(),
      createTransformationVersion,
      commitVersion,
    }, { kind: "missing-values", column: "group", markers: " B, B " });

    expect(result).toMatchObject({ kind: "committed", selectedColumn: "group_clean" });
    expect(createTransformationVersion).toHaveBeenCalledWith("source-dataset", {
      kind: "missing_markers",
      columns: [{
        source_column: "group",
        target_column: "group_clean",
        markers: ["B"],
        target_type: "text",
        target_scale: "nominal",
        target_label: "Missing-cleaned Group",
        value_labels: {},
      }],
    }, "Source (missing-cleaned 1 column)");
    expect(commitVersion).toHaveBeenCalledWith(mutation);
    expect(dataset).toEqual(sourceBefore);
    expect(dataset.rows).toEqual(sourceBefore.rows);
  });

  it("cleans all columns atomically in one child and one commit with exact multi-output lineage", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const mutation = multiMissingValueMutation();
    const createTransformationVersion = vi.fn().mockResolvedValue(mutation);
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion: vi.fn(),
      createTransformationVersion,
      commitVersion,
    }, { kind: "missing-values", column: null, markers: "1, B" });

    expect(result).toMatchObject({ kind: "committed", selectedColumn: "score_clean", mutation });
    expect(createTransformationVersion).toHaveBeenCalledWith("source-dataset", mutation.version.transformation!.spec, "Source (missing-cleaned 2 columns)");
    expect(createTransformationVersion).toHaveBeenCalledOnce();
    expect(commitVersion).toHaveBeenCalledOnce();
    expect(dataset).toEqual(sourceBefore);
  });

  it("creates one atomic standardized child dataset with an explicit sample denominator", async () => {
    const dataset = sourceDataset();
    const sourceBefore = structuredClone(dataset);
    const mutation = standardizeMutation();
    const createTransformationVersion = vi.fn().mockResolvedValue(mutation);
    const commitVersion = vi.fn();

    const result = await executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: true,
      createRecodeVersion: vi.fn(),
      createTransformationVersion,
      commitVersion,
    }, { kind: "z-score", column: "score", outputName: "z score" });

    expect(result).toMatchObject({ kind: "committed", selectedColumn: "z_score", mutation });
    expect(createTransformationVersion).toHaveBeenCalledWith("source-dataset", {
      kind: "standardize",
      source_column: "score",
      target_column: "z_score",
      denominator: "sample_n_minus_one",
      target_label: "Standardized score",
    }, "Source (standardized score)");
    expect(createTransformationVersion).toHaveBeenCalledTimes(1);
    expect(commitVersion).toHaveBeenCalledOnce();
    expect(commitVersion).toHaveBeenCalledWith(mutation);
    expect(dataset).toEqual(sourceBefore);
  });

  it("keeps standardization unavailable in browser preview without calling the native service", async () => {
    const createTransformationVersion = vi.fn();
    const commitVersion = vi.fn();
    const result = await executeDataWorkspaceVersionedAction({
      dataset: sourceDataset(),
      nativeDesktop: false,
      createRecodeVersion: vi.fn(),
      createTransformationVersion,
      commitVersion,
    }, { kind: "z-score", column: "score", outputName: "z_score" });

    expect(result).toMatchObject({ kind: "blocked", message: expect.stringContaining("not changed") });
    expect(createTransformationVersion).not.toHaveBeenCalled();
    expect(commitVersion).not.toHaveBeenCalled();
  });

  it("rejects a transform response whose lineage does not bind the new dataset", async () => {
    const invalid = missingValueMutation();
    invalid.version.transformation!.output_dataset_id = "different-dataset";
    const commitVersion = vi.fn();

    await expect(executeDataWorkspaceVersionedAction({
      dataset: sourceDataset(),
      nativeDesktop: true,
      createRecodeVersion: vi.fn(),
      createTransformationVersion: vi.fn().mockResolvedValue(invalid),
      commitVersion,
    }, { kind: "missing-values", column: "group", markers: "B" })).rejects.toThrow("reproducible lineage");

    expect(commitVersion).not.toHaveBeenCalled();
  });
});
