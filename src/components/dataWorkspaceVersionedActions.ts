import { canonicalDatasetTransformationJsonV2, type DatasetTransformationSpecV2 } from "../domain/datasetTransformationsV2";
import type { Dataset, DatasetVersionMutation, RecodeColumnSpec } from "../types";

type RecodeDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "recode" }>;
type StandardizeDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "standardize" }>;
type AddColumnDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "add_column" }>;
type MissingMarkersDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "missing_markers" }>;
type ReverseScaleDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "reverse_scale" }>;
type ArithmeticDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "arithmetic" }>;
type RowAggregateDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "row_aggregate" }>;
type DummyDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "dummy" }>;
type GroupDatasetTransformationSpecV2 = Extract<DatasetTransformationSpecV2, { kind: "group" }>;

export type DataWorkspaceSortDirection = "asc" | "desc";

export interface DataWorkspaceViewSort {
  column: string;
  direction: DataWorkspaceSortDirection;
}

export interface DataWorkspaceViewRow {
  row: Dataset["rows"][number];
  sourceIndex: number;
}

export type DataWorkspaceVersionedAction =
  | { kind: "recode"; column: string; from: string; to: string }
  | { kind: "sort"; column: string | null; direction: DataWorkspaceSortDirection }
  | { kind: "add-column"; name: string; value: string }
  | { kind: "missing-values"; column: string | null; markers: string }
  | { kind: "z-score"; column: string | null; outputName: string }
  | { kind: "reverse-scale"; column: string | null; minimum: string; maximum: string; outputName: string }
  | { kind: "arithmetic"; leftColumn: string | null; right: { kind: "column"; column: string | null } | { kind: "constant"; value: string }; operator: "add" | "subtract" | "multiply" | "divide"; outputName: string }
  | { kind: "row-aggregate"; columns: readonly string[]; operation: "sum" | "mean"; missingPolicy: "propagate" | "available"; minimumNonMissing: string; outputName: string }
  | { kind: "dummy"; column: string | null; matchValue: string; missingPolicy: "missing" | "zero"; outputName: string }
  | { kind: "group-values"; column: string | null; rules: string; unmatched: "missing" | "error"; outputName: string };

export type DataWorkspaceVersionedActionResult =
  | { kind: "blocked"; message: string }
  | { kind: "view-only"; message: string; selectedColumn: string; sort: DataWorkspaceViewSort }
  | { kind: "committed"; message: string; selectedColumn: string; mutation: DatasetVersionMutation };

interface DataWorkspaceVersionedActionDependencies {
  dataset: Readonly<Dataset>;
  nativeDesktop: boolean;
  createRecodeVersion: (datasetId: string, spec: RecodeColumnSpec) => Promise<DatasetVersionMutation>;
  createTransformationVersion: (datasetId: string, spec: DatasetTransformationSpecV2, outputDatasetName: string) => Promise<DatasetVersionMutation>;
  commitVersion: (mutation: DatasetVersionMutation) => void;
}

const isViewMissing = (value: unknown) => value == null || value === "";

const compareViewValues = (left: unknown, right: unknown) => {
  if (isViewMissing(left)) return isViewMissing(right) ? 0 : 1;
  if (isViewMissing(right)) return -1;
  if (typeof left === "number" && Number.isFinite(left) && typeof right === "number" && Number.isFinite(right)) return left - right;
  return String(left).localeCompare(String(right));
};

export function sortDataWorkspaceViewRows(
  rows: Readonly<Dataset["rows"]>,
  sort: DataWorkspaceViewSort | null,
): DataWorkspaceViewRow[] {
  const indexed = rows.map((row, sourceIndex) => ({ row, sourceIndex }));
  if (!sort) return indexed;
  return indexed.sort((left, right) => {
    const comparison = compareViewValues(left.row[sort.column], right.row[sort.column]);
    if (!comparison) return left.sourceIndex - right.sourceIndex;
    const leftMissing = isViewMissing(left.row[sort.column]);
    const rightMissing = isViewMissing(right.row[sort.column]);
    if (leftMissing || rightMissing) return comparison;
    return sort.direction === "desc" ? -comparison : comparison;
  });
}

const uniqueColumn = (columns: readonly string[], base: string) => {
  const used = new Set(columns.map((column) => column.trim().normalize("NFKC").toLocaleLowerCase()));
  if (!used.has(base.trim().normalize("NFKC").toLocaleLowerCase())) return base;
  let suffix = 2;
  while (used.has(`${base}_${suffix}`.normalize("NFKC").toLocaleLowerCase())) suffix += 1;
  return `${base}_${suffix}`;
};

const uniqueTargetColumn = (columns: readonly string[], sourceColumn: string) =>
  uniqueColumn(columns, `${sourceColumn}_recoded`);

const derivedTargetColumn = (columns: readonly string[], requested: string, fallback: string) =>
  uniqueColumn(columns, requested.trim().replace(/\s+/g, "_") || fallback);

const inferSourceMetadata = (dataset: Readonly<Dataset>, sourceColumn: string) => {
  const metadata = dataset.columnMetadata?.find((column) => column.name === sourceColumn);
  if (metadata) return metadata;
  const nonMissing = dataset.rows
    .map((row) => row[sourceColumn])
    .filter((value) => value != null && value !== "");
  const numeric = nonMissing.every((value) => Number.isFinite(Number(value)));
  return {
    column_type: numeric ? "numeric" as const : "text" as const,
    scale_type: numeric ? "continuous" as const : "nominal" as const,
    label: null,
  };
};

const missingMarkerValues = (dataset: Readonly<Dataset>, sourceColumn: string, markers: string) => {
  const tokens = [...new Set(markers.split(",").map((value) => value.trim()).filter(Boolean))];
  const observed = dataset.rows.map((row) => row[sourceColumn]);
  const metadata = inferSourceMetadata(dataset, sourceColumn);
  const values: Array<string | number> = [];
  const add = (value: string | number) => {
    if (!values.some((candidate) => typeof candidate === typeof value && candidate === value)) values.push(value);
  };
  for (const token of tokens) {
    let matchedObservedType = false;
    if (observed.some((value) => typeof value === "string" && value === token)) {
      add(token);
      matchedObservedType = true;
    }
    const numeric = Number(token);
    if (Number.isFinite(numeric) && observed.some((value) => typeof value === "number" && value === numeric)) {
      add(numeric);
      matchedObservedType = true;
    }
    if (!matchedObservedType) {
      add(metadata.column_type !== "text" && Number.isFinite(numeric) ? numeric : token);
    }
  }
  return values;
};

const observedCellValue = (dataset: Readonly<Dataset>, sourceColumn: string, raw: string) => {
  const observed = dataset.rows.map((row) => row[sourceColumn]);
  if (observed.some((value) => typeof value === "string" && value === raw)) return raw;
  const numeric = Number(raw);
  if (Number.isFinite(numeric) && observed.some((value) => typeof value === "number" && value === numeric)) return numeric;
  return inferSourceMetadata(dataset, sourceColumn).column_type !== "text" && Number.isFinite(numeric) ? numeric : raw;
};

const outputCellValue = (raw: string) => {
  const numeric = Number(raw);
  return Number.isFinite(numeric) && raw.trim() !== "" ? numeric : raw;
};

const parseValueGroupRules = (dataset: Readonly<Dataset>, sourceColumn: string, source: string): GroupDatasetTransformationSpecV2["rules"] | null => {
  const rules = source.split(/[;\n]+/).map((rule) => rule.trim()).filter(Boolean);
  if (!rules.length) return null;
  const parsed: Array<GroupDatasetTransformationSpecV2["rules"][number]> = [];
  for (const rule of rules) {
    const separator = rule.includes("=>") ? "=>" : "=";
    const at = rule.indexOf(separator);
    if (at <= 0) return null;
    const inputs = rule.slice(0, at).split(",").map((value) => value.trim()).filter(Boolean);
    const output = rule.slice(at + separator.length).trim();
    if (!inputs.length || !output) return null;
    parsed.push({
      kind: "values",
      output: outputCellValue(output),
      values: inputs.map((value) => observedCellValue(dataset, sourceColumn, value)),
      label: output,
    });
  }
  return parsed;
};

const browserDerivationBlocked = (): DataWorkspaceVersionedActionResult => ({
  kind: "blocked",
  message: "Browser preview cannot create immutable dataset versions. Open the installed Windows app to derive a variable; the current dataset was not changed.",
});

const invalidDerivation = (message: string): DataWorkspaceVersionedActionResult => ({
  kind: "blocked",
  message: `${message} The current dataset was not changed.`,
});

const assertRecodeVersion = (
  sourceDatasetId: string,
  sourceColumn: string,
  targetColumn: string,
  mutation: DatasetVersionMutation,
) => {
  const { dataset, version } = mutation;
  if (
    dataset.id === sourceDatasetId
    || version.datasetId !== dataset.id
    || version.parentDatasetId !== sourceDatasetId
    || version.operation !== "recode"
    || version.sourceColumn !== sourceColumn
    || version.targetColumn !== targetColumn
    || !version.createdAt
  ) {
    throw new Error("Recode was rejected because the native response did not prove a new immutable dataset version with provenance.");
  }
};

const assertTransformationVersion = (
  sourceDataset: Readonly<Dataset>,
  spec: DatasetTransformationSpecV2,
  mutation: DatasetVersionMutation,
) => {
  const sourceDatasetId = sourceDataset.id;
  const { dataset, version } = mutation;
  const lineage = version.transformation;
  const recordedSpec = lineage?.spec;
  const inputColumns = spec.kind === "add_column" ? []
    : spec.kind === "missing_markers" ? spec.columns.map((column) => column.source_column)
      : spec.kind === "arithmetic" ? spec.right.kind === "column" ? [spec.left_column, spec.right.column] : [spec.left_column]
        : spec.kind === "row_aggregate" ? [...spec.source_columns]
          : [spec.source_column];
  const outputColumns = spec.kind === "missing_markers" ? spec.columns.map((column) => column.target_column) : [spec.target_column];
  if (
    dataset.id === sourceDatasetId
    || version.datasetId !== dataset.id
    || version.parentDatasetId !== sourceDatasetId
    || version.operation !== "transform"
    || version.sourceColumn !== (inputColumns[0] ?? null)
    || version.targetColumn !== (outputColumns[0] ?? null)
    || !version.createdAt
    || !lineage
    || lineage.schema_version !== 2
    || lineage.engine !== "qpls.dataset_transform.v2"
    || lineage.source_dataset_id !== sourceDatasetId
    || !sourceDataset.fingerprint
    || lineage.source_dataset_fingerprint !== sourceDataset.fingerprint
    || lineage.output_dataset_id !== dataset.id
    || !dataset.fingerprint
    || lineage.output_dataset_fingerprint !== dataset.fingerprint
    || lineage.created_at !== version.createdAt
    || !lineage.operation_id.trim()
    || !lineage.spec_sha256.trim()
    || lineage.source_row_count !== (sourceDataset.rowCount ?? sourceDataset.rows.length)
    || (dataset.rowCount ?? dataset.rows.length) !== (sourceDataset.rowCount ?? sourceDataset.rows.length)
    || outputColumns.some((column) => !dataset.columns.includes(column))
    || !Number.isInteger(lineage.output_missing_count)
    || lineage.output_missing_count < 0
    || recordedSpec?.kind !== spec.kind
    || canonicalDatasetTransformationJsonV2(recordedSpec) !== canonicalDatasetTransformationJsonV2(spec)
    || canonicalDatasetTransformationJsonV2(lineage.input_columns) !== canonicalDatasetTransformationJsonV2(inputColumns)
    || canonicalDatasetTransformationJsonV2(lineage.output_columns) !== canonicalDatasetTransformationJsonV2(outputColumns)
  ) {
    throw new Error("Transformation was rejected because the native response did not prove the requested immutable dataset version with reproducible lineage.");
  }
};

async function commitTransformation(
  dependencies: DataWorkspaceVersionedActionDependencies,
  spec: DatasetTransformationSpecV2,
  outputDatasetName: string,
): Promise<DataWorkspaceVersionedActionResult> {
  const mutation = await dependencies.createTransformationVersion(dependencies.dataset.id, spec, outputDatasetName);
  assertTransformationVersion(dependencies.dataset, spec, mutation);
  dependencies.commitVersion(mutation);
  const targetColumn = spec.kind === "missing_markers" ? spec.columns[0]?.target_column : spec.target_column;
  if (!targetColumn) throw new Error("Transformation was rejected because its validated lineage has no output column.");
  return { kind: "committed", message: mutation.version.summary, selectedColumn: targetColumn, mutation };
}

export async function executeDataWorkspaceVersionedAction(
  dependencies: DataWorkspaceVersionedActionDependencies,
  action: DataWorkspaceVersionedAction,
): Promise<DataWorkspaceVersionedActionResult> {
  if (action.kind === "sort") {
    if (!action.column || !dependencies.dataset.columns.includes(action.column) || (action.direction !== "asc" && action.direction !== "desc")) {
      return { kind: "blocked", message: "Preview sorting needs a selected data column and ascending or descending order. The current dataset was not changed." };
    }
    return {
      kind: "view-only",
      message: `Preview rows sorted by ${action.column} (${action.direction === "desc" ? "descending" : "ascending"}). The scientific dataset and saved row order were not changed.`,
      selectedColumn: action.column,
      sort: { column: action.column, direction: action.direction },
    };
  }
  if (action.kind === "add-column") {
    if (!dependencies.nativeDesktop) return {
      kind: "blocked",
      message: "Browser preview cannot create immutable dataset versions. Open the installed Windows app to add a column; the current dataset was not changed.",
    };
    const requestedTarget = action.name.trim().replace(/\s+/g, "_");
    if (!requestedTarget) return { kind: "blocked", message: "Enter a name for the new column. The current dataset was not changed." };
    const targetColumn = uniqueColumn(dependencies.dataset.columns, requestedTarget);
    const rawValue = action.value.trim();
    const numericValue = Number(rawValue);
    const value = rawValue === "" ? null : Number.isFinite(numericValue) ? numericValue : action.value;
    const numeric = value === null || typeof value === "number";
    const spec: AddColumnDatasetTransformationSpecV2 = {
      kind: "add_column",
      target_column: targetColumn,
      value,
      target_type: numeric ? "numeric" : "text",
      target_scale: numeric ? "continuous" : "nominal",
      target_label: targetColumn,
      value_labels: {},
    };
    const mutation = await dependencies.createTransformationVersion(
      dependencies.dataset.id,
      spec,
      `${dependencies.dataset.name} (added ${targetColumn})`,
    );
    assertTransformationVersion(dependencies.dataset, spec, mutation);
    dependencies.commitVersion(mutation);
    return { kind: "committed", message: mutation.version.summary, selectedColumn: targetColumn, mutation };
  }
  if (action.kind === "missing-values") {
    if (!dependencies.nativeDesktop) {
      return {
        kind: "blocked",
        message: "Browser preview cannot create immutable dataset versions. Open the installed Windows app to handle missing markers; the current dataset was not changed.",
      };
    }
    if (action.column && !dependencies.dataset.columns.includes(action.column)) {
      return { kind: "blocked", message: "The selected missing-value column is not in this dataset. The current dataset was not changed." };
    }
    const sourceColumns = action.column ? [action.column] : [...dependencies.dataset.columns];
    const plannedTargets = [...dependencies.dataset.columns];
    const columns = sourceColumns.map((sourceColumn) => {
      const markers = missingMarkerValues(dependencies.dataset, sourceColumn, action.markers);
      const metadata = inferSourceMetadata(dependencies.dataset, sourceColumn);
      const targetColumn = uniqueColumn(plannedTargets, `${sourceColumn}_clean`);
      plannedTargets.push(targetColumn);
      return {
        source_column: sourceColumn,
        target_column: targetColumn,
        markers,
        target_type: metadata.column_type,
        target_scale: metadata.scale_type,
        target_label: `Missing-cleaned ${metadata.label || sourceColumn}`,
        value_labels: { ...("value_labels" in metadata ? metadata.value_labels : {}) },
      };
    });
    if (!columns.length || columns.some((column) => !column.markers.length)) {
      return { kind: "blocked", message: "Enter at least one non-blank missing marker. The current dataset was not changed." };
    }
    const spec: MissingMarkersDatasetTransformationSpecV2 = { kind: "missing_markers", columns };
    const mutation = await dependencies.createTransformationVersion(
      dependencies.dataset.id,
      spec,
      `${dependencies.dataset.name} (missing-cleaned ${sourceColumns.length} column${sourceColumns.length === 1 ? "" : "s"})`,
    );
    assertTransformationVersion(dependencies.dataset, spec, mutation);
    dependencies.commitVersion(mutation);
    return {
      kind: "committed",
      message: mutation.version.summary,
      selectedColumn: columns[0].target_column,
      mutation,
    };
  }
  if (action.kind === "reverse-scale") {
    if (!dependencies.nativeDesktop) return browserDerivationBlocked();
    if (!action.column || !dependencies.dataset.columns.includes(action.column)) return invalidDerivation("Reverse scale needs a selected numeric data column.");
    const minimum = Number(action.minimum);
    const maximum = Number(action.maximum);
    if (!Number.isFinite(minimum) || !Number.isFinite(maximum) || minimum >= maximum) return invalidDerivation("Enter a finite scale minimum smaller than the maximum.");
    const targetColumn = derivedTargetColumn(dependencies.dataset.columns, action.outputName, `${action.column}_reversed`);
    const spec: ReverseScaleDatasetTransformationSpecV2 = {
      kind: "reverse_scale",
      source_column: action.column,
      target_column: targetColumn,
      scale_min: minimum,
      scale_max: maximum,
      target_label: `Reverse-scaled ${action.column}`,
    };
    return commitTransformation(dependencies, spec, `${dependencies.dataset.name} (reverse-scaled ${action.column})`);
  }
  if (action.kind === "arithmetic") {
    if (!dependencies.nativeDesktop) return browserDerivationBlocked();
    if (!action.leftColumn || !dependencies.dataset.columns.includes(action.leftColumn)) return invalidDerivation("Arithmetic needs a selected numeric left-hand column.");
    let right: ArithmeticDatasetTransformationSpecV2["right"];
    if (action.right.kind === "column") {
      if (!action.right.column || !dependencies.dataset.columns.includes(action.right.column)) return invalidDerivation("Choose a numeric right-hand column.");
      right = { kind: "column", column: action.right.column };
    } else {
      const value = Number(action.right.value);
      if (!Number.isFinite(value)) return invalidDerivation("Enter a finite arithmetic constant.");
      right = { kind: "constant", value };
    }
    const targetColumn = derivedTargetColumn(dependencies.dataset.columns, action.outputName, `${action.leftColumn}_${action.operator}`);
    const spec: ArithmeticDatasetTransformationSpecV2 = {
      kind: "arithmetic",
      left_column: action.leftColumn,
      right,
      operator: action.operator,
      target_column: targetColumn,
      target_label: `${action.operator} derived from ${action.leftColumn}`,
    };
    return commitTransformation(dependencies, spec, `${dependencies.dataset.name} (${action.operator} ${action.leftColumn})`);
  }
  if (action.kind === "row-aggregate") {
    if (!dependencies.nativeDesktop) return browserDerivationBlocked();
    const sourceColumns = action.columns.map((column) => column.trim()).filter(Boolean);
    if (sourceColumns.length < 2 || new Set(sourceColumns).size !== sourceColumns.length || sourceColumns.some((column) => !dependencies.dataset.columns.includes(column))) {
      return invalidDerivation("Row sum or average needs at least two distinct dataset columns.");
    }
    const parsedMinimum = action.minimumNonMissing.trim() === "" ? undefined : Number(action.minimumNonMissing);
    if (parsedMinimum !== undefined && (!Number.isInteger(parsedMinimum) || parsedMinimum < 1 || parsedMinimum > sourceColumns.length)) {
      return invalidDerivation("Minimum non-missing values must be a whole number within the selected column count.");
    }
    const targetColumn = derivedTargetColumn(dependencies.dataset.columns, action.outputName, action.operation === "sum" ? "row_sum" : "row_average");
    const spec: RowAggregateDatasetTransformationSpecV2 = {
      kind: "row_aggregate",
      source_columns: sourceColumns,
      operation: action.operation,
      missing_policy: action.missingPolicy,
      ...(parsedMinimum === undefined ? {} : { minimum_non_missing: parsedMinimum }),
      target_column: targetColumn,
      target_label: action.operation === "sum" ? "Row-wise sum" : "Row-wise average",
    };
    return commitTransformation(dependencies, spec, `${dependencies.dataset.name} (${action.operation === "sum" ? "row sum" : "row average"})`);
  }
  if (action.kind === "dummy") {
    if (!dependencies.nativeDesktop) return browserDerivationBlocked();
    if (!action.column || !dependencies.dataset.columns.includes(action.column)) return invalidDerivation("Dummy generation needs a selected source column.");
    if (!action.matchValue.trim()) return invalidDerivation("Enter the exact source value that should map to one.");
    const targetColumn = derivedTargetColumn(dependencies.dataset.columns, action.outputName, `is_${action.column}`);
    const spec: DummyDatasetTransformationSpecV2 = {
      kind: "dummy",
      source_column: action.column,
      match_value: observedCellValue(dependencies.dataset, action.column, action.matchValue),
      missing_policy: action.missingPolicy,
      target_column: targetColumn,
      target_label: `${action.column} equals ${action.matchValue}`,
    };
    return commitTransformation(dependencies, spec, `${dependencies.dataset.name} (dummy ${action.column})`);
  }
  if (action.kind === "group-values") {
    if (!dependencies.nativeDesktop) return browserDerivationBlocked();
    if (!action.column || !dependencies.dataset.columns.includes(action.column)) return invalidDerivation("Group derivation needs a selected source column.");
    const rules = parseValueGroupRules(dependencies.dataset, action.column, action.rules);
    if (!rules) return invalidDerivation("Enter group rules as source values = output, separated by semicolons.");
    const targetColumn = derivedTargetColumn(dependencies.dataset.columns, action.outputName, `${action.column}_group`);
    const spec: GroupDatasetTransformationSpecV2 = {
      kind: "group",
      source_column: action.column,
      rules,
      unmatched: action.unmatched,
      target_column: targetColumn,
      target_label: `Grouped ${action.column}`,
    };
    return commitTransformation(dependencies, spec, `${dependencies.dataset.name} (grouped ${action.column})`);
  }
  if (action.kind === "z-score") {
    if (!dependencies.nativeDesktop) {
      return {
        kind: "blocked",
        message: "Browser preview cannot create immutable dataset versions. Open the installed Windows app to standardize data; the current dataset was not changed.",
      };
    }
    if (!action.column || !dependencies.dataset.columns.includes(action.column)) {
      return { kind: "blocked", message: "Standardization needs a selected numeric data column. The current dataset was not changed." };
    }
    const requestedTarget = action.outputName.trim().replace(/\s+/g, "_") || `z_${action.column}`;
    const targetColumn = uniqueColumn(dependencies.dataset.columns, requestedTarget);
    const spec: StandardizeDatasetTransformationSpecV2 = {
      kind: "standardize",
      source_column: action.column,
      target_column: targetColumn,
      denominator: "sample_n_minus_one",
      target_label: `Standardized ${action.column}`,
    };
    const mutation = await dependencies.createTransformationVersion(
      dependencies.dataset.id,
      spec,
      `${dependencies.dataset.name} (standardized ${action.column})`,
    );
    assertTransformationVersion(dependencies.dataset, spec, mutation);
    dependencies.commitVersion(mutation);
    return {
      kind: "committed",
      message: mutation.version.summary,
      selectedColumn: targetColumn,
      mutation,
    };
  }
  if (!dependencies.nativeDesktop) {
    return {
      kind: "blocked",
      message: "Browser preview cannot create immutable dataset versions. Open the installed Windows app to recode data; the current dataset was not changed.",
    };
  }
  if (!action.column || !dependencies.dataset.columns.includes(action.column)) {
    return { kind: "blocked", message: "Recode needs a selected data column. The current dataset was not changed." };
  }

  const metadata = inferSourceMetadata(dependencies.dataset, action.column);
  const targetColumn = uniqueTargetColumn(dependencies.dataset.columns, action.column);
  const spec: RecodeColumnSpec = {
    sourceColumn: action.column,
    targetColumn,
    targetLabel: `Recoded ${metadata.label || action.column}`,
    targetType: metadata.column_type,
    targetScale: metadata.scale_type,
    mappings: [{ source: action.from, target: action.to.trim() ? action.to : null }],
    unmapped: "keep_original",
  };
  const mutation = await dependencies.createRecodeVersion(dependencies.dataset.id, spec);
  assertRecodeVersion(dependencies.dataset.id, action.column, targetColumn, mutation);
  dependencies.commitVersion(mutation);
  return {
    kind: "committed",
    message: mutation.version.summary,
    selectedColumn: targetColumn,
    mutation,
  };
}
