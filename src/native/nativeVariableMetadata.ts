import type { ColumnMetadata, Dataset } from "../types";

export interface NativeVariableMetadataDraft {
  label: string;
  scaleType: ColumnMetadata["scale_type"];
  theoreticalMin: string;
  theoreticalMax: string;
}

export interface NativeVariableMetadataValidation {
  metadata: ColumnMetadata | null;
  error: string | null;
}

export function defaultNativeColumnMetadata(dataset: Readonly<Dataset>, column: string): ColumnMetadata {
  const sample = dataset.rows.map((row) => row[column]).find((value) => value != null);
  const columnType: ColumnMetadata["column_type"] = typeof sample === "number" ? "numeric" : typeof sample === "boolean" ? "boolean" : "text";
  return {
    name: column,
    label: null,
    column_type: columnType,
    scale_type: columnType === "numeric" ? "continuous" : columnType === "boolean" ? "binary" : "nominal",
    missing_markers: ["", "NA", "N/A", "."],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: {},
  };
}

export function nativeVariableMetadataDraft(metadata: Readonly<ColumnMetadata>): NativeVariableMetadataDraft {
  return {
    label: metadata.label ?? "",
    scaleType: metadata.scale_type,
    theoreticalMin: metadata.theoretical_min == null ? "" : String(metadata.theoretical_min),
    theoreticalMax: metadata.theoretical_max == null ? "" : String(metadata.theoretical_max),
  };
}

function optionalFiniteNumber(value: string, label: string): { value: number | null; error: string | null } {
  if (!value.trim()) return { value: null, error: null };
  const parsed = Number(value);
  return Number.isFinite(parsed)
    ? { value: parsed, error: null }
    : { value: null, error: `${label} must be a finite number.` };
}

export function validateNativeVariableMetadata(
  current: Readonly<ColumnMetadata>,
  draft: Readonly<NativeVariableMetadataDraft>,
): NativeVariableMetadataValidation {
  const minimum = optionalFiniteNumber(draft.theoreticalMin, "Theoretical minimum");
  if (minimum.error) return { metadata: null, error: minimum.error };
  const maximum = optionalFiniteNumber(draft.theoreticalMax, "Theoretical maximum");
  if (maximum.error) return { metadata: null, error: maximum.error };
  if (minimum.value != null && maximum.value != null && minimum.value > maximum.value) {
    return { metadata: null, error: "Theoretical minimum cannot exceed the theoretical maximum." };
  }
  return {
    metadata: {
      ...current,
      label: draft.label.trim() || null,
      scale_type: draft.scaleType,
      theoretical_min: minimum.value,
      theoretical_max: maximum.value,
    },
    error: null,
  };
}
