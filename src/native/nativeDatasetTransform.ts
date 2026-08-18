import type {
  DatasetCellV2,
  DatasetGroupRuleV2,
  DatasetTransformationIssueV2,
  DatasetTransformationSpecV2,
} from "../domain/datasetTransformationsV2";
import type { ColumnMetadata, Dataset } from "../types";

export type NativeDatasetTransformKindV2 = DatasetTransformationSpecV2["kind"];

export interface NativeRecodeMappingDraftV2 {
  source: string;
  target: string;
}

export interface NativeGroupRuleDraftV2 {
  kind: DatasetGroupRuleV2["kind"];
  output: string;
  label: string;
  values: string;
  minimum: string;
  maximum: string;
  includeMinimum: boolean;
  includeMaximum: boolean;
}

export interface NativeDatasetTransformDraftV2 {
  kind: NativeDatasetTransformKindV2;
  sourceColumn: string;
  targetColumn: string;
  targetLabel: string;
  outputDatasetName: string;
  scaleMinimum: string;
  scaleMaximum: string;
  recodeMappings: NativeRecodeMappingDraftV2[];
  recodeUnmapped: "keep" | "missing" | "error";
  recodeTargetType: ColumnMetadata["column_type"];
  recodeTargetScale: ColumnMetadata["scale_type"];
  arithmeticOperator: "add" | "subtract" | "multiply" | "divide";
  arithmeticRightKind: "column" | "constant";
  arithmeticRightColumn: string;
  arithmeticConstant: string;
  aggregateSourceColumns: string[];
  aggregateOperation: "sum" | "mean";
  aggregateMissingPolicy: "propagate" | "available";
  aggregateMinimumNonMissing: string;
  dummyMatchValue: string;
  dummyMissingPolicy: "missing" | "zero";
  groupOutputType: "text" | "numeric";
  groupRules: NativeGroupRuleDraftV2[];
  groupUnmatched: "missing" | "error";
}

export interface NativeDatasetTransformBuildV2 {
  spec: DatasetTransformationSpecV2 | null;
  issues: DatasetTransformationIssueV2[];
}

export const NATIVE_DATASET_TRANSFORM_KINDS_V2: ReadonlyArray<{
  id: NativeDatasetTransformKindV2;
  label: string;
  description: string;
}> = [
  { id: "reverse_scale", label: "Reverse scale", description: "Reverse a numeric rating within its declared minimum and maximum." },
  { id: "recode", label: "Recode values", description: "Map source values into a new variable without changing the source." },
  { id: "arithmetic", label: "Arithmetic", description: "Add, subtract, multiply, or divide using another variable or a constant." },
  { id: "row_aggregate", label: "Row sum or mean", description: "Combine two or more numeric variables for each case." },
  { id: "dummy", label: "Dummy variable", description: "Create a binary 0/1 variable for one selected value." },
  { id: "group", label: "Group variable", description: "Derive named groups from exact values or numeric ranges." },
];

function issue(code: string, field: string, message: string): DatasetTransformationIssueV2 {
  return { code, field, message, row_index: null };
}

function safeStem(value: string): string {
  const stem = value.trim().replace(/[^\p{L}\p{N}_]+/gu, "_").replace(/^_+|_+$/g, "");
  return stem || "variable";
}

function uniqueColumn(dataset: Pick<Dataset, "columns">, base: string): string {
  if (!dataset.columns.includes(base)) return base;
  let suffix = 2;
  while (dataset.columns.includes(`${base}_${suffix}`)) suffix += 1;
  return `${base}_${suffix}`;
}

function kindSuffix(kind: NativeDatasetTransformKindV2): string {
  if (kind === "reverse_scale") return "reversed";
  if (kind === "recode") return "recoded";
  if (kind === "arithmetic") return "calculated";
  if (kind === "row_aggregate") return "combined";
  if (kind === "dummy") return "dummy";
  return "group";
}

export function nativeDatasetTransformTargetV2(
  dataset: Pick<Dataset, "columns">,
  sourceColumn: string,
  kind: NativeDatasetTransformKindV2,
): string {
  return uniqueColumn(dataset, `${safeStem(sourceColumn)}_${kindSuffix(kind)}`);
}

export function defaultNativeDatasetTransformDraftV2(
  dataset: Dataset,
  selectedColumn: string,
): NativeDatasetTransformDraftV2 {
  const sourceColumn = dataset.columns.includes(selectedColumn) ? selectedColumn : dataset.columns[0] ?? "";
  const targetColumn = nativeDatasetTransformTargetV2(dataset, sourceColumn, "reverse_scale");
  return {
    kind: "reverse_scale",
    sourceColumn,
    targetColumn,
    targetLabel: "",
    outputDatasetName: `${dataset.name} - ${targetColumn}`,
    scaleMinimum: "1",
    scaleMaximum: "5",
    recodeMappings: [{ source: "", target: "" }],
    recodeUnmapped: "keep",
    recodeTargetType: "numeric",
    recodeTargetScale: "continuous",
    arithmeticOperator: "add",
    arithmeticRightKind: "column",
    arithmeticRightColumn: dataset.columns.find((column) => column !== sourceColumn) ?? sourceColumn,
    arithmeticConstant: "1",
    aggregateSourceColumns: dataset.columns.slice(0, 2),
    aggregateOperation: "mean",
    aggregateMissingPolicy: "propagate",
    aggregateMinimumNonMissing: "1",
    dummyMatchValue: "",
    dummyMissingPolicy: "missing",
    groupOutputType: "text",
    groupRules: [{
      kind: "values",
      output: "Group 1",
      label: "Group 1",
      values: "",
      minimum: "",
      maximum: "",
      includeMinimum: true,
      includeMaximum: true,
    }],
    groupUnmatched: "missing",
  };
}

export function changeNativeDatasetTransformKindV2(
  dataset: Dataset,
  draft: NativeDatasetTransformDraftV2,
  kind: NativeDatasetTransformKindV2,
): NativeDatasetTransformDraftV2 {
  const targetColumn = nativeDatasetTransformTargetV2(dataset, draft.sourceColumn, kind);
  return {
    ...draft,
    kind,
    targetColumn,
    outputDatasetName: `${dataset.name} - ${targetColumn}`,
  };
}

export function nativeDatasetTransformAvailabilityReasonV2({
  dataset,
  nativeDesktop,
  projectWritable,
  mutationsLocked,
  datasetResident,
}: {
  dataset: Dataset;
  nativeDesktop: boolean;
  projectWritable: boolean;
  mutationsLocked: boolean;
  datasetResident: boolean;
}): string | null {
  if (!nativeDesktop) return "Derived variables are available only in the installed Windows app, where the complete dataset can be versioned safely.";
  if (!projectWritable) return "This project is read-only. Save a writable copy before deriving a variable.";
  if (mutationsLocked) return "Finish or cancel the active calculation before deriving a variable.";
  if ((dataset.kind ?? "raw") !== "raw") return "Choose a raw-observation dataset. Variables cannot be derived from a covariance or correlation matrix.";
  if (!datasetResident) return "Load the complete dataset before deriving a variable.";
  if (!dataset.columns.length) return "Import a raw-observation dataset before deriving a variable.";
  return null;
}

function metadataFor(dataset: Dataset, column: string): ColumnMetadata | undefined {
  return dataset.columnMetadata?.find((item) => item.name === column);
}

function parseFiniteNumber(raw: string, field: string, label: string, issues: DatasetTransformationIssueV2[]): number | null {
  const trimmed = raw.trim();
  const value = trimmed === "" ? Number.NaN : Number(trimmed);
  if (!Number.isFinite(value)) {
    issues.push(issue("input.number_required", field, `Enter a finite number for ${label}.`));
    return null;
  }
  return Object.is(value, -0) ? 0 : value;
}

function parseCell(
  raw: string,
  columnType: ColumnMetadata["column_type"] | undefined,
  field: string,
  label: string,
  issues: DatasetTransformationIssueV2[],
): Exclude<DatasetCellV2, null> | null {
  if (!raw.trim()) {
    issues.push(issue("input.value_required", field, `Enter ${label}.`));
    return null;
  }
  if (columnType === "numeric" || columnType === "boolean") return parseFiniteNumber(raw, field, label, issues);
  return raw.trim();
}

function targetLabel(draft: NativeDatasetTransformDraftV2): string | null {
  return draft.targetLabel.trim() || null;
}

export function buildNativeDatasetTransformationSpecV2(
  dataset: Dataset,
  draft: NativeDatasetTransformDraftV2,
): NativeDatasetTransformBuildV2 {
  const issues: DatasetTransformationIssueV2[] = [];
  const common = { target_column: draft.targetColumn.trim(), target_label: targetLabel(draft) };
  const sourceType = metadataFor(dataset, draft.sourceColumn)?.column_type;
  let spec: DatasetTransformationSpecV2 | null = null;

  if (!draft.outputDatasetName.trim()) issues.push(issue("output.name_required", "output_dataset_name", "Name the derived dataset version."));

  if (draft.kind === "reverse_scale") {
    const scaleMin = parseFiniteNumber(draft.scaleMinimum, "scale_min", "the scale minimum", issues);
    const scaleMax = parseFiniteNumber(draft.scaleMaximum, "scale_max", "the scale maximum", issues);
    if (scaleMin !== null && scaleMax !== null) spec = {
      kind: "reverse_scale",
      source_column: draft.sourceColumn,
      ...common,
      scale_min: scaleMin,
      scale_max: scaleMax,
    };
  } else if (draft.kind === "recode") {
    const mappings = draft.recodeMappings.flatMap((mapping, index) => {
      const source = parseCell(mapping.source, sourceType, `mappings.${index}.source`, `source value ${index + 1}`, issues);
      let target: DatasetCellV2 = null;
      if (mapping.target.trim()) {
        target = parseCell(mapping.target, draft.recodeTargetType, `mappings.${index}.target`, `new value ${index + 1}`, issues);
      }
      return source === null ? [] : [{ source, target }];
    });
    spec = {
      kind: "recode",
      source_column: draft.sourceColumn,
      ...common,
      mappings,
      unmapped: draft.recodeUnmapped,
      target_type: draft.recodeTargetType,
      target_scale: draft.recodeTargetScale,
    };
  } else if (draft.kind === "arithmetic") {
    const right = draft.arithmeticRightKind === "column"
      ? { kind: "column" as const, column: draft.arithmeticRightColumn }
      : (() => {
          const value = parseFiniteNumber(draft.arithmeticConstant, "right.value", "the arithmetic constant", issues);
          return value === null ? null : { kind: "constant" as const, value };
        })();
    if (right) spec = {
      kind: "arithmetic",
      left_column: draft.sourceColumn,
      ...common,
      operator: draft.arithmeticOperator,
      right,
    };
  } else if (draft.kind === "row_aggregate") {
    const minimum = parseFiniteNumber(draft.aggregateMinimumNonMissing, "minimum_non_missing", "the minimum complete-variable count", issues);
    spec = {
      kind: "row_aggregate",
      ...common,
      source_columns: draft.aggregateSourceColumns,
      operation: draft.aggregateOperation,
      missing_policy: draft.aggregateMissingPolicy,
      ...(minimum === null ? {} : { minimum_non_missing: minimum }),
    };
  } else if (draft.kind === "dummy") {
    const match = parseCell(draft.dummyMatchValue, sourceType, "match_value", "the value to match", issues);
    if (match !== null) spec = {
      kind: "dummy",
      source_column: draft.sourceColumn,
      ...common,
      match_value: match,
      missing_policy: draft.dummyMissingPolicy,
    };
  } else {
    const rules = draft.groupRules.flatMap((rule, index): DatasetGroupRuleV2[] => {
      const output = parseCell(rule.output, draft.groupOutputType === "numeric" ? "numeric" : "text", `rules.${index}.output`, `group output ${index + 1}`, issues);
      if (output === null) return [];
      if (rule.kind === "values") {
        const values = rule.values.split(/\r?\n/).filter((value) => value.trim()).flatMap((value, valueIndex) => {
          const parsed = parseCell(value, sourceType, `rules.${index}.values.${valueIndex}`, `source value ${valueIndex + 1} for group ${index + 1}`, issues);
          return parsed === null ? [] : [parsed];
        });
        return [{ kind: "values", output, values, label: rule.label.trim() || null }];
      }
      const minimum = rule.minimum.trim() ? parseFiniteNumber(rule.minimum, `rules.${index}.minimum`, `group ${index + 1} minimum`, issues) : null;
      const maximum = rule.maximum.trim() ? parseFiniteNumber(rule.maximum, `rules.${index}.maximum`, `group ${index + 1} maximum`, issues) : null;
      return [{
        kind: "numeric_range",
        output,
        minimum,
        maximum,
        include_minimum: rule.includeMinimum,
        include_maximum: rule.includeMaximum,
        label: rule.label.trim() || null,
      }];
    });
    spec = {
      kind: "group",
      source_column: draft.sourceColumn,
      ...common,
      rules,
      unmatched: draft.groupUnmatched,
    };
  }

  return { spec: issues.length ? null : spec, issues };
}

export function nativeDatasetTransformationScaleLabelV2(draft: NativeDatasetTransformDraftV2): string {
  if (draft.kind === "recode") return draft.recodeTargetScale[0].toUpperCase() + draft.recodeTargetScale.slice(1);
  if (draft.kind === "dummy") return "Binary";
  if (draft.kind === "group") return "Nominal";
  return "Continuous";
}

function normalizeIssue(candidate: unknown): DatasetTransformationIssueV2 | null {
  if (!candidate || typeof candidate !== "object") return null;
  const item = candidate as Record<string, unknown>;
  if (typeof item.message !== "string") return null;
  return {
    code: typeof item.code === "string" ? item.code : "transformation.failed",
    field: typeof item.field === "string" ? item.field : "spec",
    message: item.message,
    row_index: typeof item.row_index === "number" ? item.row_index : null,
  };
}

function issuesFromUnknown(candidate: unknown): DatasetTransformationIssueV2[] {
  if (!candidate || typeof candidate !== "object") return [];
  const value = candidate as Record<string, unknown>;
  if (!Array.isArray(value.issues)) return [];
  return value.issues.map(normalizeIssue).filter((item): item is DatasetTransformationIssueV2 => Boolean(item));
}

export function nativeDatasetTransformationIssuesFromErrorV2(reason: unknown): DatasetTransformationIssueV2[] {
  const direct = issuesFromUnknown(reason);
  if (direct.length) return direct;
  const message = reason instanceof Error ? reason.message : typeof reason === "string" ? reason : "The transformation could not be completed.";
  try {
    const parsed = issuesFromUnknown(JSON.parse(message));
    if (parsed.length) return parsed;
  } catch {
    // Tauri may return a plain customer-facing error string.
  }
  return [issue("transformation.failed", "spec", message)];
}
