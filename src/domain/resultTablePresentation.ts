import type {
  ResultTable,
  ResultTableColumnKind,
  ResultTableColumnPriority,
} from "./resultTables";
import type { CanonicalResultTable } from "./canonicalResultDocumentV2";

export type { ResultTableColumnKind, ResultTableColumnPriority } from "./resultTables";

export interface ResultTableColumnPresentation {
  index: number;
  label: string;
  kind: ResultTableColumnKind;
  priority: ResultTableColumnPriority;
  sticky: boolean;
}

export interface ResultTablePresentation {
  columns: ResultTableColumnPresentation[];
  identityColumnIndex: number | null;
  confidenceHeading: string | null;
  hasLowerPriorityColumns: boolean;
  hasHorizontalOverflowRisk: boolean;
}

const IDENTITY_HEADING = /^(?:construct|indicator|relationship|relation|path|effect|parameter|criterion|metric|name|label|target|source|outcome|predictor|moderator|mediator|component|group|segment|class|condition|comparison|benchmark|method|variable|term|pair|sample)$/iu;
const NUMERIC_HEADING = /(?:estimate|coefficient|loading|weight|mean|median|bias|error|deviation|variance|value|statistic|ratio|correlation|residual|effect size|importance|performance|probability|power|alpha|p(?:[ -]?value)?|t(?:[ -]?value)?|f(?:[ -]?value)?|r(?: squared|²|2)|q(?: squared|²|2)|lower|upper|confidence|percentile|count|observations|replicates|samples|iterations|size|minimum|maximum|difference|fit|srmr|nfi|gfi|cfi|rmsea|aic|bic|chi)/iu;
const TERTIARY_HEADING = /(?:bootstrap mean|sample mean|bias|standard deviation|standard error|t statistic|t value|replicates|failures|usable|requested|seed|workers|digest|fingerprint|version|status|reason|source result|source table|accounting|ledger|index)/iu;
const SECONDARY_HEADING = /(?:estimate|coefficient|loading|weight|effect|p(?:[ -]?value)?|lower|upper|confidence|decision|significant|importance|performance|power|fit|srmr|nfi|gfi|cfi|rmsea|aic|bic)/iu;
const MISSING_VALUE = /^(?:|—|-|n\/a|na|not reported|not available|undefined)$/iu;
const NUMBER_VALUE = /^[+-]?(?:(?:\d+(?:[,.]\d{3})*(?:\.\d+)?)|(?:\.\d+))(?:e[+-]?\d+)?%?$/iu;

function looksNumeric(value: string): boolean {
  const trimmed = value.trim();
  return MISSING_VALUE.test(trimmed) || NUMBER_VALUE.test(trimmed.replace(/\s+/gu, ""));
}

function numericColumn(table: ResultTable, columnIndex: number): boolean {
  const populated = table.rows
    .map((row) => row[columnIndex]?.trim() ?? "")
    .filter((value) => value.length > 0 && !MISSING_VALUE.test(value));
  return populated.length > 0 && populated.every(looksNumeric);
}

function identityColumnIndex(table: ResultTable): number | null {
  const hinted = table.presentation?.columns?.findIndex((column) => column?.kind === "identity" || column?.sticky === true) ?? -1;
  if (hinted >= 0) return hinted;
  const explicit = table.columns.findIndex((label) => IDENTITY_HEADING.test(label.trim()));
  if (explicit >= 0) return explicit;
  const textual = table.columns.findIndex((_label, index) => !numericColumn(table, index));
  return textual >= 0 ? textual : null;
}

function normalizedConfidenceLevel(value?: number | null): number | null {
  if (value == null || !Number.isFinite(value)) return null;
  const proportion = value > 1 ? value / 100 : value;
  return proportion > 0 && proportion < 1 ? proportion : null;
}

function confidenceLevelFromColumns(columns: readonly string[]): number | null {
  for (const column of columns) {
    const match = column.match(/\b(\d{2}(?:\.\d+)?)\s*%\s*(?:ci|confidence)/iu);
    if (!match) continue;
    const level = Number(match[1]) / 100;
    if (level > 0 && level < 1) return level;
  }
  return null;
}

function hasConfidenceColumns(columns: readonly string[]): boolean {
  const joined = columns.join(" ");
  return /\bconfidence\b|\bci\b|\blower\b.*\bupper\b/iu.test(joined);
}

function displayPercentage(proportion: number): string {
  const percentage = proportion * 100;
  return Number.isInteger(percentage)
    ? percentage.toFixed(0)
    : percentage.toFixed(2).replace(/0+$/u, "").replace(/\.$/u, "");
}

/**
 * Derives stable presentation metadata without changing result values or the
 * archive-facing table contract. The metadata is intentionally CSS-oriented
 * so native and future table renderers can share alignment and priority rules.
 */
export function resultTablePresentation(
  table: ResultTable,
  confidenceLevel?: number | null,
): ResultTablePresentation {
  const identityIndex = identityColumnIndex(table);
  const columns = table.columns.map((label, index): ResultTableColumnPresentation => {
    const hint = table.presentation?.columns?.[index];
    const identity = index === identityIndex;
    const numeric = !identity && (NUMERIC_HEADING.test(label) || numericColumn(table, index));
    const priority: ResultTableColumnPriority = hint?.priority ?? (identity
      ? "primary"
      : TERTIARY_HEADING.test(label)
        ? "tertiary"
        : SECONDARY_HEADING.test(label) || index < 3
          ? "secondary"
          : "tertiary");
    return {
      index,
      label,
      kind: hint?.kind ?? (identity ? "identity" : numeric ? "number" : "text"),
      priority,
      sticky: hint?.sticky ?? identity,
    };
  });
  const level = normalizedConfidenceLevel(confidenceLevel)
    ?? normalizedConfidenceLevel(table.presentation?.confidenceLevel)
    ?? confidenceLevelFromColumns(table.columns);
  return {
    columns,
    identityColumnIndex: identityIndex,
    confidenceHeading: hasConfidenceColumns(table.columns) && level != null
      ? `${displayPercentage(level)}% confidence intervals`
      : null,
    hasLowerPriorityColumns: columns.some((column) => column.priority === "tertiary"),
    hasHorizontalOverflowRisk: columns.length > 4,
  };
}

/** Uses the typed canonical column contract instead of inspecting cell text. */
export function canonicalResultTablePresentation(
  table: CanonicalResultTable,
  confidenceLevel?: number | null,
): ResultTablePresentation {
  const explicitIdentity = table.columns.findIndex((column) => column.role === "label");
  const identityIndex = explicitIdentity >= 0
    ? explicitIdentity
    : table.columns.findIndex((column) => column.data_type === "text" && column.role !== "provenance");
  const columns = table.columns.map((column, index): ResultTableColumnPresentation => {
    const identity = index === identityIndex;
    const priority: ResultTableColumnPriority = identity
      ? "primary"
      : column.role === "estimate" || column.role === "decision"
        ? "secondary"
        : column.role === "uncertainty" || column.role === "diagnostic" || column.role === "provenance"
          ? "tertiary"
          : index < 3 ? "secondary" : "tertiary";
    return {
      index,
      label: column.label,
      kind: identity ? "identity" : column.data_type === "number" ? "number" : "text",
      priority,
      sticky: identity,
    };
  });
  const level = normalizedConfidenceLevel(confidenceLevel) ?? confidenceLevelFromColumns(table.columns.map((column) => column.label));
  return {
    columns,
    identityColumnIndex: identityIndex >= 0 ? identityIndex : null,
    confidenceHeading: hasConfidenceColumns(table.columns.map((column) => column.label)) && level != null
      ? `${displayPercentage(level)}% confidence intervals`
      : null,
    hasLowerPriorityColumns: columns.some((column) => column.priority === "tertiary"),
    hasHorizontalOverflowRisk: columns.length > 4,
  };
}
