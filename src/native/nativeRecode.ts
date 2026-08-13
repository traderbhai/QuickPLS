import type {
  ColumnMetadata,
  Dataset,
  RecodeColumnSpec,
  RecodeUnmappedPolicy,
  RecodeValueMapping,
} from "../types";

export const NATIVE_RECODE_TARGET_TYPES = ["numeric", "text", "boolean"] as const;
export const NATIVE_RECODE_TARGET_SCALES = ["continuous", "ordinal", "nominal", "binary", "identifier"] as const;
export const NATIVE_RECODE_UNMAPPED_POLICIES = ["keep_original", "set_missing", "error"] as const;

export type NativeRecodeTargetType = (typeof NATIVE_RECODE_TARGET_TYPES)[number];
export type NativeRecodeTargetScale = (typeof NATIVE_RECODE_TARGET_SCALES)[number];
export type NativeRecodeUnmappedPolicy = RecodeUnmappedPolicy;

export interface NativeRecodeMappingDraft {
  source: string;
  target: string;
}

export interface NativeRecodeDraft {
  sourceColumn: string;
  targetColumn: string;
  targetLabel: string;
  targetType: NativeRecodeTargetType;
  targetScale: NativeRecodeTargetScale;
  mappings: NativeRecodeMappingDraft[];
  unmapped: NativeRecodeUnmappedPolicy;
}

export type NativeRecodeValueMapping = RecodeValueMapping;
/** Camel-case payload accepted by the Tauri `recode_dataset_column` command. */
export type NativeRecodeColumnSpec = RecodeColumnSpec;

export type NativeRecodeIssueCode =
  | "dataset_not_raw"
  | "source_not_found"
  | "target_required"
  | "target_not_trimmed"
  | "target_exists"
  | "target_type_invalid"
  | "target_scale_invalid"
  | "target_scale_incompatible"
  | "mappings_required"
  | "mapping_source_required"
  | "mapping_source_invalid"
  | "mapping_source_duplicate"
  | "mapping_target_invalid"
  | "unmapped_policy_invalid";

export interface NativeRecodeValidationIssue {
  code: NativeRecodeIssueCode;
  path: string;
  message: string;
}

export interface NativeRecodeValidation {
  spec: NativeRecodeColumnSpec | null;
  error: string | null;
  issues: NativeRecodeValidationIssue[];
}

const NUMERIC_VALUE = /^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$/;

function includesLiteral<const T extends readonly string[]>(values: T, value: unknown): value is T[number] {
  return typeof value === "string" && (values as readonly string[]).includes(value);
}

function sourceMetadata(dataset: Readonly<Dataset>, sourceColumn: string): ColumnMetadata | undefined {
  return dataset.columnMetadata?.find((metadata) => metadata.name === sourceColumn);
}

function compatibleDefaultScale(
  targetType: NativeRecodeTargetType,
  preferred: NativeRecodeTargetScale | undefined,
): NativeRecodeTargetScale {
  if (targetType === "boolean") return "binary";
  if (targetType === "text" && preferred === "continuous") return "nominal";
  return preferred ?? (targetType === "numeric" ? "continuous" : "nominal");
}

/**
 * Derives a new indicator name using schema information only. Dataset rows are
 * deliberately outside this helper so opening the dialog remains cheap even
 * when the active dataset is paged by the native backend.
 */
export function deriveNativeRecodeTargetColumn(dataset: Readonly<Dataset>, sourceColumn: string): string {
  const source = sourceColumn.trim() || "indicator";
  const base = `${source}_recoded`;
  const existing = new Set(dataset.columns);
  if (!existing.has(base)) return base;

  let suffix = 2;
  while (existing.has(`${base}_${suffix}`)) suffix += 1;
  return `${base}_${suffix}`;
}

export function defaultNativeRecodeDraft(dataset: Readonly<Dataset>, sourceColumn: string): NativeRecodeDraft {
  const metadata = sourceMetadata(dataset, sourceColumn);
  const targetType: NativeRecodeTargetType = metadata?.column_type ?? "text";
  return {
    sourceColumn,
    targetColumn: deriveNativeRecodeTargetColumn(dataset, sourceColumn),
    targetLabel: `${metadata?.label?.trim() || sourceColumn.trim() || "Indicator"} (recoded)`,
    targetType,
    targetScale: compatibleDefaultScale(targetType, metadata?.scale_type),
    mappings: [{ source: "", target: "" }],
    unmapped: "keep_original",
  };
}

interface ParsedMappingValue {
  canonical: string;
  error: string | null;
}

function parseMappingValue(value: string, type: NativeRecodeTargetType, role: "source" | "target"): ParsedMappingValue {
  if (type === "text") return { canonical: `text:${value}`, error: null };

  const trimmed = value.trim();
  if (type === "numeric") {
    if (!NUMERIC_VALUE.test(trimmed)) {
      return { canonical: "", error: `Mapping ${role} value ${JSON.stringify(value)} is not numeric.` };
    }
    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed)) {
      return { canonical: "", error: `Mapping ${role} value ${JSON.stringify(value)} must be finite.` };
    }
    return { canonical: `numeric:${Object.is(parsed, -0) ? 0 : parsed}`, error: null };
  }

  switch (trimmed.toLocaleLowerCase("en-US")) {
    case "true":
    case "1":
      return { canonical: "boolean:true", error: null };
    case "false":
    case "0":
      return { canonical: "boolean:false", error: null };
    default:
      return { canonical: "", error: `Mapping ${role} value ${JSON.stringify(value)} is not boolean (use true, false, 1, or 0).` };
  }
}

function issue(code: NativeRecodeIssueCode, path: string, message: string): NativeRecodeValidationIssue {
  return { code, path, message };
}

/** Validates a dialog draft and, only when valid, builds the exact native command payload. */
export function validateNativeRecodeDraft(
  dataset: Readonly<Dataset>,
  draft: Readonly<NativeRecodeDraft>,
): NativeRecodeValidation {
  const issues: NativeRecodeValidationIssue[] = [];

  if (dataset.kind && dataset.kind !== "raw") {
    issues.push(issue("dataset_not_raw", "dataset", "Recode is available only for raw row-level datasets."));
  }

  const sourceExists = dataset.columns.includes(draft.sourceColumn);
  if (!sourceExists) {
    issues.push(issue("source_not_found", "sourceColumn", "Select an indicator that exists in the active dataset."));
  }

  const trimmedTarget = draft.targetColumn.trim();
  if (!trimmedTarget) {
    issues.push(issue("target_required", "targetColumn", "Enter a name for the new indicator."));
  } else if (trimmedTarget !== draft.targetColumn) {
    issues.push(issue("target_not_trimmed", "targetColumn", "The new indicator name cannot begin or end with spaces."));
  } else if (dataset.columns.includes(draft.targetColumn)) {
    issues.push(issue("target_exists", "targetColumn", `An indicator named ${JSON.stringify(draft.targetColumn)} already exists.`));
  }

  const targetType = includesLiteral(NATIVE_RECODE_TARGET_TYPES, draft.targetType) ? draft.targetType : null;
  const targetScale = includesLiteral(NATIVE_RECODE_TARGET_SCALES, draft.targetScale) ? draft.targetScale : null;
  if (!targetType) {
    issues.push(issue("target_type_invalid", "targetType", "Select a supported target type."));
  }
  if (!targetScale) {
    issues.push(issue("target_scale_invalid", "targetScale", "Select a supported target scale."));
  }
  if (targetType === "text" && targetScale === "continuous") {
    issues.push(issue("target_scale_incompatible", "targetScale", "A text recode cannot use a continuous scale."));
  }
  if (targetType === "boolean" && targetScale && targetScale !== "binary") {
    issues.push(issue("target_scale_incompatible", "targetScale", "A boolean recode must use a binary scale."));
  }

  if (!draft.mappings.length) {
    issues.push(issue("mappings_required", "mappings", "Add at least one recode mapping."));
  }

  const sourceType: NativeRecodeTargetType = sourceMetadata(dataset, draft.sourceColumn)?.column_type ?? "text";
  const seenSources = new Map<string, number>();
  const mappings: NativeRecodeValueMapping[] = [];

  draft.mappings.forEach((mapping, index) => {
    const sourcePath = `mappings.${index}.source`;
    const targetPath = `mappings.${index}.target`;
    const sourceIsBlank = !mapping.source.trim();
    let parsedSource: ParsedMappingValue | null = null;

    if (sourceIsBlank) {
      issues.push(issue("mapping_source_required", sourcePath, `Mapping ${index + 1} needs a source value.`));
    } else {
      parsedSource = parseMappingValue(mapping.source, sourceType, "source");
      if (parsedSource.error) {
        issues.push(issue("mapping_source_invalid", sourcePath, `Mapping ${index + 1}: ${parsedSource.error}`));
      } else {
        const previousIndex = seenSources.get(parsedSource.canonical);
        if (previousIndex != null) {
          issues.push(
            issue(
              "mapping_source_duplicate",
              sourcePath,
              `Mapping ${index + 1} duplicates the source value in mapping ${previousIndex + 1}.`,
            ),
          );
        } else {
          seenSources.set(parsedSource.canonical, index);
        }
      }
    }

    const target = mapping.target.trim() ? mapping.target : null;
    if (target != null && targetType) {
      const parsedTarget = parseMappingValue(target, targetType, "target");
      if (parsedTarget.error) {
        issues.push(issue("mapping_target_invalid", targetPath, `Mapping ${index + 1}: ${parsedTarget.error}`));
      }
    }
    mappings.push({ source: mapping.source, target });
  });

  const unmapped = includesLiteral(NATIVE_RECODE_UNMAPPED_POLICIES, draft.unmapped) ? draft.unmapped : null;
  if (!unmapped) {
    issues.push(issue("unmapped_policy_invalid", "unmapped", "Select how unmapped values should be handled."));
  }

  const spec =
    issues.length === 0 && targetType && targetScale && unmapped
      ? {
          sourceColumn: draft.sourceColumn,
          targetColumn: draft.targetColumn,
          targetLabel: draft.targetLabel.trim() || null,
          targetType,
          targetScale,
          mappings,
          unmapped,
        }
      : null;

  return { spec, error: issues[0]?.message ?? null, issues };
}

/** Builds a command payload for callers that only need the valid/invalid result. */
export function buildNativeRecodeSpec(
  dataset: Readonly<Dataset>,
  draft: Readonly<NativeRecodeDraft>,
): NativeRecodeColumnSpec | null {
  return validateNativeRecodeDraft(dataset, draft).spec;
}
