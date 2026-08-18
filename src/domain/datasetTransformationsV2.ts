import type { ColumnMetadata, Dataset } from "../types";

export const DATASET_TRANSFORMATION_SCHEMA_V2 = 2 as const;
export const DATASET_TRANSFORMATION_ENGINE_V2 = "qpls.dataset_transform.v2" as const;

// Project rows persist categorical/binary values as strings or finite numbers.
// ColumnMetadata may classify a numeric 0/1 column as boolean, but boolean JS
// values are not part of the Dataset row wire contract.
export type DatasetCellV2 = string | number | null;
export type DatasetMissingPolicyV2 = "propagate" | "available";

export interface DatasetMissingMarkerColumnV2 {
  source_column: string;
  target_column: string;
  markers: readonly Exclude<DatasetCellV2, null>[];
  target_type: ColumnMetadata["column_type"];
  target_scale: ColumnMetadata["scale_type"];
  target_label?: string | null;
  value_labels?: Readonly<Record<string, string>>;
}

export type DatasetTransformationSpecV2 =
  | {
      kind: "add_column";
      target_column: string;
      value: DatasetCellV2;
      target_type: ColumnMetadata["column_type"];
      target_scale: ColumnMetadata["scale_type"];
      target_label?: string | null;
      value_labels?: Readonly<Record<string, string>>;
    }
  | {
      kind: "missing_markers";
      columns: readonly DatasetMissingMarkerColumnV2[];
    }
  | {
      kind: "reverse_scale";
      source_column: string;
      target_column: string;
      scale_min: number;
      scale_max: number;
      target_label?: string | null;
    }
  | {
      kind: "standardize";
      source_column: string;
      target_column: string;
      denominator: "sample_n_minus_one";
      target_label?: string | null;
    }
  | {
      kind: "recode";
      source_column: string;
      target_column: string;
      mappings: readonly { source: Exclude<DatasetCellV2, null>; target: DatasetCellV2 }[];
      unmapped: "keep" | "missing" | "error";
      target_type: ColumnMetadata["column_type"];
      target_scale: ColumnMetadata["scale_type"];
      target_label?: string | null;
      value_labels?: Readonly<Record<string, string>>;
    }
  | {
      kind: "arithmetic";
      left_column: string;
      right: { kind: "column"; column: string } | { kind: "constant"; value: number };
      operator: "add" | "subtract" | "multiply" | "divide";
      target_column: string;
      target_label?: string | null;
    }
  | {
      kind: "row_aggregate";
      source_columns: readonly string[];
      operation: "sum" | "mean";
      missing_policy: DatasetMissingPolicyV2;
      minimum_non_missing?: number;
      target_column: string;
      target_label?: string | null;
    }
  | {
      kind: "dummy";
      source_column: string;
      match_value: Exclude<DatasetCellV2, null>;
      missing_policy: "missing" | "zero";
      target_column: string;
      target_label?: string | null;
    }
  | {
      kind: "group";
      source_column: string;
      rules: readonly DatasetGroupRuleV2[];
      unmatched: "missing" | "error";
      target_column: string;
      target_label?: string | null;
    };

export type DatasetGroupRuleV2 =
  | {
      kind: "values";
      output: Exclude<DatasetCellV2, null>;
      values: readonly Exclude<DatasetCellV2, null>[];
      label?: string | null;
    }
  | {
      kind: "numeric_range";
      output: Exclude<DatasetCellV2, null>;
      minimum: number | null;
      maximum: number | null;
      include_minimum: boolean;
      include_maximum: boolean;
      label?: string | null;
    };

export interface DatasetTransformationIssueV2 {
  code: string;
  field: string;
  message: string;
  row_index: number | null;
}

export interface DatasetTransformationPreviewRowV2 {
  row_index: number;
  inputs: Readonly<Record<string, DatasetCellV2>>;
  output: DatasetCellV2;
  outputs: Readonly<Record<string, DatasetCellV2>>;
}

export interface DatasetTransformationPreviewV2 {
  schema_version: typeof DATASET_TRANSFORMATION_SCHEMA_V2;
  source_dataset_id: string;
  target_column: string;
  output_columns: readonly string[];
  input_columns: readonly string[];
  inspected_rows: number;
  total_rows: number;
  /** Total missing cells across all derived output columns. */
  output_missing_count: number;
  rows: readonly DatasetTransformationPreviewRowV2[];
  issues: readonly DatasetTransformationIssueV2[];
}

export interface DatasetTransformationLineageV2 {
  schema_version: typeof DATASET_TRANSFORMATION_SCHEMA_V2;
  engine: typeof DATASET_TRANSFORMATION_ENGINE_V2;
  operation_id: string;
  source_dataset_id: string;
  source_dataset_fingerprint: string;
  output_dataset_id: string;
  output_dataset_fingerprint: string;
  created_at: string;
  spec_sha256: string;
  spec: DatasetTransformationSpecV2;
  input_columns: readonly string[];
  output_columns: readonly string[];
  source_row_count: number;
  /** Total missing cells across all derived output columns. */
  output_missing_count: number;
}

export interface DatasetTransformationMutationV2 {
  dataset: Dataset;
  lineage: DatasetTransformationLineageV2;
}

export interface ApplyDatasetTransformationOptionsV2 {
  output_dataset_id: string;
  output_dataset_name: string;
  created_at: string;
}

export class DatasetTransformationErrorV2 extends Error {
  constructor(public readonly issues: readonly DatasetTransformationIssueV2[]) {
    super(issues[0]?.message ?? "Dataset transformation is invalid.");
    this.name = "DatasetTransformationErrorV2";
  }
}

function issue(code: string, field: string, message: string, rowIndex: number | null = null): DatasetTransformationIssueV2 {
  return { code, field, message, row_index: rowIndex };
}

function isMissing(value: unknown): value is null | undefined | "" {
  return value == null || value === "";
}

function cell(value: unknown): DatasetCellV2 {
  if (isMissing(value)) return null;
  if (typeof value === "string") return value;
  if (typeof value === "number" && Number.isFinite(value)) return Object.is(value, -0) ? 0 : value;
  throw new Error("unsupported_cell");
}

function numberCell(value: unknown): number | null {
  if (isMissing(value)) return null;
  if (typeof value === "number" && Number.isFinite(value)) return Object.is(value, -0) ? 0 : value;
  if (typeof value === "string" && value.trim() !== "" && Number.isFinite(Number(value))) return Number(value);
  throw new Error("not_numeric");
}

function valueKey(value: Exclude<DatasetCellV2, null>): string {
  return `${typeof value}:${String(value)}`;
}

function compareCell(left: DatasetCellV2, right: Exclude<DatasetCellV2, null>): boolean {
  return left !== null && valueKey(left) === valueKey(right);
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, item]) => [key, stableValue(item)]));
  }
  if (typeof value === "number" && !Number.isFinite(value)) throw new Error("non_finite_number");
  return value;
}

export function canonicalDatasetTransformationJsonV2(value: unknown): string {
  return JSON.stringify(stableValue(value));
}

async function sha256(value: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function inputColumns(spec: DatasetTransformationSpecV2): string[] {
  if (spec.kind === "add_column") return [];
  if (spec.kind === "missing_markers") return spec.columns.map((column) => column.source_column);
  if (spec.kind === "arithmetic") return spec.right.kind === "column"
    ? [spec.left_column, spec.right.column]
    : [spec.left_column];
  if (spec.kind === "row_aggregate") return [...spec.source_columns];
  return [spec.source_column];
}

function targetColumns(spec: DatasetTransformationSpecV2): string[] {
  return spec.kind === "missing_markers"
    ? spec.columns.map((column) => column.target_column)
    : [spec.target_column];
}

function targetColumn(spec: DatasetTransformationSpecV2): string {
  return targetColumns(spec)[0] ?? "";
}

function targetValueIsCompatible(value: DatasetCellV2, targetType: ColumnMetadata["column_type"]): boolean {
  if (value === null) return true;
  if (targetType === "numeric") return typeof value === "number" && Number.isFinite(value);
  if (targetType === "text") return typeof value === "string";
  return typeof value === "number" && (value === 0 || value === 1);
}

function validateDataset(dataset: Readonly<Dataset>): DatasetTransformationIssueV2[] {
  const issues: DatasetTransformationIssueV2[] = [];
  if (dataset.kind && dataset.kind !== "raw") issues.push(issue("dataset.raw_required", "dataset", "Choose a raw-observation dataset before deriving a variable."));
  const declaredRows = dataset.rowCount ?? dataset.rows.length;
  if (declaredRows !== dataset.rows.length) issues.push(issue("dataset.rows_not_resident", "dataset", "Load all observation rows before deriving a variable."));
  const normalized = new Set<string>();
  for (const [index, column] of dataset.columns.entries()) {
    if (!column.trim()) issues.push(issue("dataset.column_blank", `columns.${index}`, "Column names cannot be blank."));
    const key = column.normalize("NFKC").toLocaleLowerCase();
    if (normalized.has(key)) issues.push(issue("dataset.column_duplicate", `columns.${index}`, `Column ${column} is duplicated.`));
    normalized.add(key);
  }
  return issues;
}

function validateSpec(dataset: Readonly<Dataset>, spec: DatasetTransformationSpecV2): DatasetTransformationIssueV2[] {
  const issues = validateDataset(dataset);
  const targets = targetColumns(spec);
  targets.forEach((candidate, index) => {
    const target = candidate.trim();
    const field = targets.length === 1 ? "target_column" : `columns.${index}.target_column`;
    if (!target) issues.push(issue("target.required", field, "Enter a name for the derived variable."));
    else if (candidate !== target) issues.push(issue("target.whitespace", field, "Derived variable names cannot begin or end with whitespace."));
    else if (dataset.columns.some((column) => column.normalize("NFKC").toLocaleLowerCase() === target.normalize("NFKC").toLocaleLowerCase())) issues.push(issue("target.exists", field, `Column ${target} already exists. Choose a new variable name.`));
  });
  const normalizedTargets = targets.map((target) => target.trim().normalize("NFKC").toLocaleLowerCase());
  if (new Set(normalizedTargets).size !== normalizedTargets.length) issues.push(issue("target.duplicate", "output_columns", "Each derived variable name must be unique."));
  const inputs = inputColumns(spec);
  for (const [index, column] of inputs.entries()) {
    if (column !== column.trim()) issues.push(issue("source.whitespace", `input_columns.${index}`, "Input variable names cannot begin or end with whitespace."));
    if (!dataset.columns.includes(column)) issues.push(issue("source.unknown", `input_columns.${index}`, `Column ${column} is not present in this dataset.`));
  }
  if (new Set(inputs.map((column) => column.trim().normalize("NFKC").toLocaleLowerCase())).size !== inputs.length) issues.push(issue("source.duplicate", "input_columns", "Choose each input variable only once."));

  if (spec.kind === "add_column") {
    if (typeof spec.value === "number" && !Number.isFinite(spec.value)) issues.push(issue("add_column.value_invalid", "value", "The constant value must be finite or missing."));
    else if (!targetValueIsCompatible(spec.value, spec.target_type)) issues.push(issue("add_column.value_type_mismatch", "value", "The constant value must match the declared column type."));
  } else if (spec.kind === "missing_markers") {
    if (!spec.columns.length) issues.push(issue("missing_markers.columns_required", "columns", "Choose at least one column to clean."));
    spec.columns.forEach((column, columnIndex) => {
      const sourceMetadata = dataset.columnMetadata?.find((metadata) => metadata.name === column.source_column);
      if (sourceMetadata && (column.target_type !== sourceMetadata.column_type
        || column.target_scale !== sourceMetadata.scale_type
        || canonicalDatasetTransformationJsonV2(column.value_labels ?? {}) !== canonicalDatasetTransformationJsonV2(sourceMetadata.value_labels ?? {}))) {
        issues.push(issue("missing_markers.metadata_mismatch", `columns.${columnIndex}`, "A cleaned column must preserve its source type, scale, and value labels."));
      }
      if (!column.markers.length) issues.push(issue("missing_markers.markers_required", `columns.${columnIndex}.markers`, "Add at least one missing-value marker."));
      const markers = new Set<string>();
      column.markers.forEach((marker, markerIndex) => {
        if (typeof marker === "number" && !Number.isFinite(marker)) issues.push(issue("missing_markers.marker_invalid", `columns.${columnIndex}.markers.${markerIndex}`, "Missing-value markers must be finite numbers or strings."));
        const key = valueKey(marker);
        if (markers.has(key)) issues.push(issue("missing_markers.marker_duplicate", `columns.${columnIndex}.markers.${markerIndex}`, "Each marker may appear only once per source column."));
        markers.add(key);
      });
    });
  } else if (spec.kind === "reverse_scale") {
    if (!Number.isFinite(spec.scale_min) || !Number.isFinite(spec.scale_max) || spec.scale_min >= spec.scale_max) {
      issues.push(issue("reverse_scale.range_invalid", "scale_min", "Enter a finite minimum smaller than the maximum."));
    }
  } else if (spec.kind === "standardize") {
    if (spec.denominator !== "sample_n_minus_one") issues.push(issue("standardize.denominator_invalid", "denominator", "Use the explicit sample n-1 standard-deviation denominator."));
  } else if (spec.kind === "recode") {
    if (!spec.mappings.length) issues.push(issue("recode.mappings_required", "mappings", "Add at least one recode mapping."));
    const sources = new Set<string>();
    spec.mappings.forEach((mapping, index) => {
      const key = valueKey(mapping.source);
      if (sources.has(key)) issues.push(issue("recode.source_duplicate", `mappings.${index}.source`, "Each source value can appear only once."));
      sources.add(key);
      if (typeof mapping.target === "number" && !Number.isFinite(mapping.target)) issues.push(issue("recode.target_invalid", `mappings.${index}.target`, "Recode targets must be finite values or missing."));
      if (spec.target_type === "boolean" && mapping.target !== null && mapping.target !== 0 && mapping.target !== 1) issues.push(issue("recode.boolean_target_invalid", `mappings.${index}.target`, "Binary recode targets must be 0, 1, or missing."));
    });
  } else if (spec.kind === "arithmetic") {
    if (spec.right.kind === "constant" && !Number.isFinite(spec.right.value)) issues.push(issue("arithmetic.constant_invalid", "right.value", "Enter a finite arithmetic constant."));
  } else if (spec.kind === "row_aggregate") {
    if (spec.source_columns.length < 2) issues.push(issue("aggregate.sources_required", "source_columns", "Choose at least two variables to combine."));
    const minimum = spec.minimum_non_missing ?? (spec.missing_policy === "available" ? 1 : spec.source_columns.length);
    if (!Number.isInteger(minimum) || minimum < 1 || minimum > spec.source_columns.length) issues.push(issue("aggregate.minimum_invalid", "minimum_non_missing", "The minimum complete-variable count must be within the selected variables."));
  } else if (spec.kind === "group") {
    if (!spec.rules.length) issues.push(issue("group.rules_required", "rules", "Add at least one group rule."));
    const outputs = new Set<string>();
    spec.rules.forEach((rule, index) => {
      const output = valueKey(rule.output);
      if (outputs.has(output)) issues.push(issue("group.output_duplicate", `rules.${index}.output`, "Each group output must be unique."));
      outputs.add(output);
      if (rule.kind === "values") {
        if (!rule.values.length) issues.push(issue("group.values_required", `rules.${index}.values`, "Add at least one source value for this group."));
        if (new Set(rule.values.map(valueKey)).size !== rule.values.length) issues.push(issue("group.values_duplicate", `rules.${index}.values`, "A group rule cannot repeat a source value."));
      } else {
        if (rule.minimum === null && rule.maximum === null) issues.push(issue("group.range_unbounded", `rules.${index}`, "A numeric group range needs a minimum or maximum."));
        if (rule.minimum !== null && !Number.isFinite(rule.minimum)) issues.push(issue("group.range_invalid", `rules.${index}.minimum`, "The group minimum must be finite."));
        if (rule.maximum !== null && !Number.isFinite(rule.maximum)) issues.push(issue("group.range_invalid", `rules.${index}.maximum`, "The group maximum must be finite."));
        if (rule.minimum !== null && rule.maximum !== null && rule.minimum > rule.maximum) issues.push(issue("group.range_invalid", `rules.${index}`, "The group minimum cannot exceed its maximum."));
      }
    });
    const claimedValues = new Set<string>();
    spec.rules.forEach((rule, index) => {
      if (rule.kind !== "values") return;
      rule.values.forEach((value) => {
        const key = valueKey(value);
        if (claimedValues.has(key)) issues.push(issue("group.value_overlap", `rules.${index}.values`, `Source value ${String(value)} belongs to more than one group.`));
        claimedValues.add(key);
      });
    });
  }
  return issues;
}

function numericRangeMatches(value: number, rule: Extract<DatasetGroupRuleV2, { kind: "numeric_range" }>): boolean {
  const aboveMinimum = rule.minimum === null || (rule.include_minimum ? value >= rule.minimum : value > rule.minimum);
  const belowMaximum = rule.maximum === null || (rule.include_maximum ? value <= rule.maximum : value < rule.maximum);
  return aboveMinimum && belowMaximum;
}

interface StandardizationParametersV2 {
  mean: number;
  standardDeviation: number;
}

function standardizationParametersV2(dataset: Readonly<Dataset>, sourceColumn: string): StandardizationParametersV2 {
  const issues: DatasetTransformationIssueV2[] = [];
  let count = 0;
  let mean = 0;
  const observed: number[] = [];
  dataset.rows.forEach((row, rowIndex) => {
    let value: number | null;
    try {
      value = numberCell(row[sourceColumn]);
    } catch {
      issues.push(issue("source.not_numeric", "source", "This transformation requires numeric input values.", rowIndex));
      return;
    }
    if (value === null) return;
    count += 1;
    const delta = value - mean;
    const nextMean = mean + delta / count;
    if (![delta, nextMean].every(Number.isFinite)) {
      throw new DatasetTransformationErrorV2([issue("standardize.non_finite", "source_column", "The observed values cannot produce finite standardization statistics.")]);
    }
    mean = Object.is(nextMean, -0) ? 0 : nextMean;
    observed.push(value);
  });
  if (issues.length) throw new DatasetTransformationErrorV2(issues);
  if (count === 0) throw new DatasetTransformationErrorV2([issue("standardize.all_missing", "source_column", "Standardization needs at least two observed numeric values; this column is entirely missing.")]);
  if (count < 2) throw new DatasetTransformationErrorV2([issue("standardize.insufficient_observations", "source_column", "Standardization needs at least two observed numeric values for sample standard deviation.")]);
  let squaredDeviationSum = 0;
  let compensation = 0;
  for (const value of observed) {
    const deviation = value - mean;
    const squaredDeviation = deviation * deviation;
    const adjusted = squaredDeviation - compensation;
    const nextSum = squaredDeviationSum + adjusted;
    if (![deviation, squaredDeviation, adjusted, nextSum].every(Number.isFinite)) throw new DatasetTransformationErrorV2([issue("standardize.non_finite", "source_column", "The observed values cannot produce a finite sample sum of squares.")]);
    compensation = (nextSum - squaredDeviationSum) - adjusted;
    squaredDeviationSum = Object.is(nextSum, -0) ? 0 : nextSum;
  }
  const variance = squaredDeviationSum / (count - 1);
  if (variance <= 0) throw new DatasetTransformationErrorV2([issue("standardize.zero_variance", "source_column", "A zero-variance column cannot be standardized.")]);
  if (![mean, variance].every(Number.isFinite)) throw new DatasetTransformationErrorV2([issue("standardize.non_finite", "source_column", "The observed values cannot produce finite standardization statistics.")]);
  const standardDeviation = Math.sqrt(variance);
  if (!Number.isFinite(standardDeviation) || standardDeviation === 0) throw new DatasetTransformationErrorV2([issue("standardize.non_finite", "source_column", "The observed values cannot produce a finite nonzero sample standard deviation.")]);
  return { mean, standardDeviation };
}

function evaluateRow(
  spec: DatasetTransformationSpecV2,
  row: Readonly<Record<string, unknown>>,
  rowIndex: number,
  standardization?: StandardizationParametersV2,
): DatasetCellV2 {
  try {
    if (spec.kind === "add_column") return spec.value;
    if (spec.kind === "missing_markers") throw new DatasetTransformationErrorV2([issue("missing_markers.internal", "columns", "Multi-column marker cleanup must be evaluated atomically.", rowIndex)]);
    if (spec.kind === "reverse_scale") {
      const source = numberCell(row[spec.source_column]);
      return source === null ? null : spec.scale_min + spec.scale_max - source;
    }
    if (spec.kind === "standardize") {
      if (!standardization) throw new DatasetTransformationErrorV2([issue("standardize.parameters_unavailable", "source_column", "Standardization parameters were not available.")]);
      const source = numberCell(row[spec.source_column]);
      if (source === null) return null;
      const output = (source - standardization.mean) / standardization.standardDeviation;
      if (!Number.isFinite(output)) throw new DatasetTransformationErrorV2([issue("standardize.non_finite", "source_column", "This standardization produced a non-finite value.", rowIndex)]);
      return Object.is(output, -0) ? 0 : output;
    }
    if (spec.kind === "recode") {
      const source = cell(row[spec.source_column]);
      if (source === null) return null;
      const mapping = spec.mappings.find((candidate) => compareCell(source, candidate.source));
      if (mapping) return mapping.target;
      if (spec.unmapped === "keep") return source;
      if (spec.unmapped === "missing") return null;
      throw new DatasetTransformationErrorV2([issue("recode.unmapped", "mappings", `No recode mapping exists for ${String(source)}.`, rowIndex)]);
    }
    if (spec.kind === "arithmetic") {
      const left = numberCell(row[spec.left_column]);
      const right = spec.right.kind === "constant" ? spec.right.value : numberCell(row[spec.right.column]);
      if (left === null || right === null) return null;
      if (spec.operator === "divide" && right === 0) throw new DatasetTransformationErrorV2([issue("arithmetic.division_by_zero", "right", "Division by zero is not defined.", rowIndex)]);
      const output = spec.operator === "add" ? left + right
        : spec.operator === "subtract" ? left - right
          : spec.operator === "multiply" ? left * right
            : left / right;
      if (!Number.isFinite(output)) throw new DatasetTransformationErrorV2([issue("arithmetic.non_finite", "operator", "This calculation produced a non-finite value.", rowIndex)]);
      return Object.is(output, -0) ? 0 : output;
    }
    if (spec.kind === "row_aggregate") {
      const values = spec.source_columns.map((column) => numberCell(row[column]));
      const complete = values.filter((value): value is number => value !== null);
      const minimum = spec.minimum_non_missing ?? (spec.missing_policy === "available" ? 1 : values.length);
      if (spec.missing_policy === "propagate" && complete.length !== values.length) return null;
      if (complete.length < minimum) return null;
      const sum = complete.reduce((total, value) => total + value, 0);
      return spec.operation === "sum" ? sum : sum / complete.length;
    }
    if (spec.kind === "dummy") {
      const source = cell(row[spec.source_column]);
      if (source === null) return spec.missing_policy === "zero" ? 0 : null;
      return compareCell(source, spec.match_value) ? 1 : 0;
    }
    const source = cell(row[spec.source_column]);
    if (source === null) return null;
    const matches = spec.rules.filter((rule) => rule.kind === "values"
      ? rule.values.some((value) => compareCell(source, value))
      : numericRangeMatches(numberCell(source) as number, rule));
    if (matches.length > 1) throw new DatasetTransformationErrorV2([issue("group.rule_overlap", "rules", `Value ${String(source)} belongs to more than one group.`, rowIndex)]);
    if (matches.length === 1) return matches[0].output;
    if (spec.unmatched === "missing") return null;
    throw new DatasetTransformationErrorV2([issue("group.unmatched", "rules", `No group rule includes ${String(source)}.`, rowIndex)]);
  } catch (error) {
    if (error instanceof DatasetTransformationErrorV2) throw error;
    const code = error instanceof Error ? error.message : "invalid_value";
    throw new DatasetTransformationErrorV2([issue(
      code === "not_numeric" ? "source.not_numeric" : "source.invalid_value",
      "source",
      code === "not_numeric" ? "This transformation requires numeric input values." : "The source data contains an unsupported value.",
      rowIndex,
    )]);
  }
}

function evaluateRows(dataset: Readonly<Dataset>, spec: DatasetTransformationSpecV2): { outputs: DatasetCellV2[][]; issues: DatasetTransformationIssueV2[] } {
  const outputs: DatasetCellV2[][] = [];
  const issues: DatasetTransformationIssueV2[] = [];
  let standardization: StandardizationParametersV2 | undefined;
  if (spec.kind === "standardize") {
    try {
      standardization = standardizationParametersV2(dataset, spec.source_column);
    } catch (error) {
      return {
        outputs: dataset.rows.map(() => [null]),
        issues: error instanceof DatasetTransformationErrorV2 ? [...error.issues] : [issue("standardize.failed", "source_column", "The standardization parameters could not be computed.")],
      };
    }
  }
  dataset.rows.forEach((row, rowIndex) => {
    try {
      const rowOutputs = spec.kind === "missing_markers"
        ? spec.columns.map((column, columnIndex) => {
          const source = cell(row[column.source_column]);
          const output = source === null || column.markers.some((marker) => compareCell(source, marker)) ? null : source;
          if (!targetValueIsCompatible(output, column.target_type)) throw new DatasetTransformationErrorV2([issue("output.type_mismatch", `columns.${columnIndex}.target_type`, "A cleaned value does not match the declared target type.", rowIndex)]);
          return output;
        })
        : [evaluateRow(spec, row, rowIndex, standardization)];
      outputs.push(rowOutputs);
    } catch (error) {
      if (error instanceof DatasetTransformationErrorV2) issues.push(...error.issues);
      else issues.push(issue("transformation.failed", "spec", "The transformation could not be evaluated.", rowIndex));
      outputs.push(targetColumns(spec).map(() => null));
    }
  });
  return { outputs, issues };
}

export function previewDatasetTransformationV2(
  dataset: Readonly<Dataset>,
  spec: DatasetTransformationSpecV2,
  previewLimit = 20,
): DatasetTransformationPreviewV2 {
  const staticIssues = validateSpec(dataset, spec);
  if (!Number.isInteger(previewLimit) || previewLimit < 1 || previewLimit > 100) staticIssues.push(issue("preview.limit_invalid", "preview_limit", "Preview between 1 and 100 rows."));
  const { outputs, issues } = staticIssues.length ? { outputs: [] as DatasetCellV2[][], issues: [] as DatasetTransformationIssueV2[] } : evaluateRows(dataset, spec);
  const inputs = inputColumns(spec);
  const targets = targetColumns(spec);
  const limit = Number.isInteger(previewLimit) && previewLimit > 0 ? Math.min(previewLimit, 100) : 20;
  return {
    schema_version: DATASET_TRANSFORMATION_SCHEMA_V2,
    source_dataset_id: dataset.id,
    target_column: targetColumn(spec),
    output_columns: targets,
    input_columns: inputs,
    inspected_rows: Math.min(dataset.rows.length, limit),
    total_rows: dataset.rowCount ?? dataset.rows.length,
    output_missing_count: outputs.flat().filter((value) => value === null).length,
    rows: outputs.slice(0, limit).map((rowOutputs, rowIndex) => ({
      row_index: rowIndex,
      inputs: Object.fromEntries(inputs.map((column) => [column, cell(dataset.rows[rowIndex][column])])),
      output: rowOutputs[0] ?? null,
      outputs: Object.fromEntries(targets.map((target, outputIndex) => [target, rowOutputs[outputIndex] ?? null])),
    })),
    issues: [...staticIssues, ...issues],
  };
}

function declaredTargetMetadata(
  targetColumnName: string,
  targetType: ColumnMetadata["column_type"],
  targetScale: ColumnMetadata["scale_type"],
  targetLabel?: string | null,
  valueLabels?: Readonly<Record<string, string>>,
): ColumnMetadata {
  return {
    name: targetColumnName,
    label: targetLabel?.trim() || null,
    column_type: targetType,
    scale_type: targetScale,
    missing_markers: [],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: { ...(valueLabels ?? {}) },
  };
}

function targetMetadata(spec: DatasetTransformationSpecV2): ColumnMetadata[] {
  if (spec.kind === "missing_markers") return spec.columns.map((column) => declaredTargetMetadata(
    column.target_column,
    column.target_type,
    column.target_scale,
    column.target_label,
    column.value_labels,
  ));
  if (spec.kind === "add_column") return [declaredTargetMetadata(
    spec.target_column,
    spec.target_type,
    spec.target_scale,
    spec.target_label,
    spec.value_labels,
  )];
  if (spec.kind === "recode") return [{
    name: spec.target_column,
    label: spec.target_label?.trim() || null,
    column_type: spec.target_type,
    scale_type: spec.target_scale,
    missing_markers: [],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: { ...(spec.value_labels ?? {}) },
  }];
  if (spec.kind === "dummy") return [{
    name: spec.target_column,
    label: spec.target_label?.trim() || null,
    column_type: "numeric",
    scale_type: "binary",
    missing_markers: [],
    theoretical_min: 0,
    theoretical_max: 1,
    value_labels: { "0": "No", "1": "Yes" },
  }];
  if (spec.kind === "group") return [{
    name: spec.target_column,
    label: spec.target_label?.trim() || null,
    column_type: spec.rules.every((rule) => typeof rule.output === "number") ? "numeric" : "text",
    scale_type: "nominal",
    missing_markers: [],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: Object.fromEntries(spec.rules.flatMap((rule) => rule.label?.trim() ? [[String(rule.output), rule.label.trim()]] : [])),
  }];
  const bounds = spec.kind === "reverse_scale" ? [spec.scale_min, spec.scale_max] as const : [null, null] as const;
  return [{
    name: spec.target_column,
    label: spec.target_label?.trim() || null,
    column_type: "numeric",
    scale_type: "continuous",
    missing_markers: [],
    theoretical_min: bounds[0],
    theoretical_max: bounds[1],
    value_labels: {},
  }];
}

export async function applyDatasetTransformationV2(
  dataset: Readonly<Dataset>,
  spec: DatasetTransformationSpecV2,
  options: ApplyDatasetTransformationOptionsV2,
): Promise<DatasetTransformationMutationV2> {
  const preview = previewDatasetTransformationV2(dataset, spec, 20);
  if (preview.issues.length) throw new DatasetTransformationErrorV2(preview.issues);
  if (!options.output_dataset_id.trim()) throw new DatasetTransformationErrorV2([issue("output.id_required", "output_dataset_id", "Provide a stable identifier for the derived dataset.")]);
  if (options.output_dataset_id === dataset.id) throw new DatasetTransformationErrorV2([issue("output.id_conflict", "output_dataset_id", "The derived dataset must have a new identifier.")]);
  if (!options.output_dataset_name.trim()) throw new DatasetTransformationErrorV2([issue("output.name_required", "output_dataset_name", "Name the derived dataset.")]);
  if (!Number.isFinite(Date.parse(options.created_at))) throw new DatasetTransformationErrorV2([issue("output.created_at_invalid", "created_at", "Use an ISO date-time for transformation lineage.")]);

  const evaluated = evaluateRows(dataset, spec);
  if (evaluated.issues.length) throw new DatasetTransformationErrorV2(evaluated.issues);
  const targets = targetColumns(spec);
  const targetMetadataEntries = targetMetadata(spec);
  const rows = dataset.rows.map((row, rowIndex) => ({
    ...row,
    ...Object.fromEntries(targets.map((target, outputIndex) => [target, evaluated.outputs[rowIndex][outputIndex]])),
  }));
  const columns = [...dataset.columns, ...targets];
  const missingByColumn = Object.fromEntries(columns.map((column) => [column, rows.filter((row) => isMissing(row[column])).length]));
  const sourceFingerprint = dataset.fingerprint?.trim() || `sha256:${await sha256(canonicalDatasetTransformationJsonV2({ columns: dataset.columns, rows: dataset.rows, columnMetadata: dataset.columnMetadata ?? [] }))}`;
  const specSha256 = await sha256(canonicalDatasetTransformationJsonV2(spec));
  const outputFingerprint = `sha256:${await sha256(canonicalDatasetTransformationJsonV2({
    source_fingerprint: sourceFingerprint,
    columns,
    rows,
    columnMetadata: [...(dataset.columnMetadata ?? []), ...targetMetadataEntries],
    spec_sha256: specSha256,
  }))}`;
  const outputMissingCount = evaluated.outputs.flat().filter((value) => value === null).length;
  const next: Dataset = {
    ...dataset,
    id: options.output_dataset_id,
    name: options.output_dataset_name.trim(),
    columns,
    rows,
    rowCount: rows.length,
    missing: Object.values(missingByColumn).reduce((total, count) => total + count, 0),
    missingByColumn,
    fingerprint: outputFingerprint,
    columnMetadata: [...(dataset.columnMetadata ?? []), ...targetMetadataEntries],
  };
  const operationId = `dataset_transform:${(await sha256(`${sourceFingerprint}\u0000${specSha256}\u0000${options.output_dataset_id}`)).slice(0, 24)}`;
  return {
    dataset: next,
    lineage: {
      schema_version: DATASET_TRANSFORMATION_SCHEMA_V2,
      engine: DATASET_TRANSFORMATION_ENGINE_V2,
      operation_id: operationId,
      source_dataset_id: dataset.id,
      source_dataset_fingerprint: sourceFingerprint,
      output_dataset_id: next.id,
      output_dataset_fingerprint: outputFingerprint,
      created_at: new Date(options.created_at).toISOString(),
      spec_sha256: specSha256,
      spec: JSON.parse(canonicalDatasetTransformationJsonV2(spec)) as DatasetTransformationSpecV2,
      input_columns: inputColumns(spec),
      output_columns: targets,
      source_row_count: dataset.rows.length,
      output_missing_count: outputMissingCount,
    },
  };
}
