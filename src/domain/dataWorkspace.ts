import type { ColumnMetadata, Dataset } from "../types";

export type DataColumnFilter = "all" | "continuous" | "ordinal" | "nominal" | "binary" | "identifier" | "nonnumeric" | "missing_heavy";

export interface DataQualitySummary {
  rows: number;
  variables: number;
  missingCells: number;
  numericVariables: number;
  nonnumericVariables: number;
  constantColumns: string[];
  duplicateHeaders: string[];
  invalidHeaders: string[];
  missingHeavyColumns: string[];
  sampleReady: boolean;
  sampleWarning: string | null;
}

export interface PrefixGroup {
  prefix: string;
  indicators: string[];
}

const metadataFor = (dataset: Dataset, column: string): ColumnMetadata | undefined =>
  dataset.columnMetadata?.find((metadata) => metadata.name === column);

const valuesFor = (dataset: Dataset, column: string) =>
  dataset.rows.map((row) => row[column]).filter((value) => value !== null && value !== undefined && value !== "");

export function columnScale(dataset: Dataset, column: string) {
  return metadataFor(dataset, column)?.scale_type ?? "continuous";
}

export function isNumericColumn(dataset: Dataset, column: string) {
  const metadata = metadataFor(dataset, column);
  if (metadata?.column_type) return metadata.column_type === "numeric";
  const values = valuesFor(dataset, column);
  return values.length > 0 && values.every((value) => typeof value === "number" || (typeof value === "string" && value.trim() !== "" && Number.isFinite(Number(value))));
}

export function missingCount(dataset: Dataset, column: string) {
  return dataset.rows.filter((row) => row[column] === null || row[column] === undefined || row[column] === "").length;
}

export function dataQualitySummary(dataset: Dataset): DataQualitySummary {
  const rows = dataset.rowCount ?? dataset.rows.length;
  const lowerCounts = new Map<string, number>();
  for (const column of dataset.columns) lowerCounts.set(column.toLowerCase(), (lowerCounts.get(column.toLowerCase()) ?? 0) + 1);
  const duplicateHeaders = dataset.columns.filter((column) => (lowerCounts.get(column.toLowerCase()) ?? 0) > 1);
  const invalidHeaders = dataset.columns.filter((column) => !column.trim() || /\s/.test(column));
  const numericVariables = dataset.columns.filter((column) => isNumericColumn(dataset, column)).length;
  const constantColumns = dataset.columns.filter((column) => new Set(valuesFor(dataset, column).map(String)).size <= 1);
  const missingHeavyColumns = dataset.columns.filter((column) => rows > 0 && missingCount(dataset, column) / rows >= 0.25);
  const sampleReady = rows >= 30;
  return {
    rows,
    variables: dataset.columns.length,
    missingCells: dataset.missing,
    numericVariables,
    nonnumericVariables: dataset.columns.length - numericVariables,
    constantColumns,
    duplicateHeaders: [...new Set(duplicateHeaders)],
    invalidHeaders,
    missingHeavyColumns,
    sampleReady,
    sampleWarning: sampleReady ? null : "Small sample: use for demos or checks, not publication-style SEM evidence.",
  };
}

export function prefixForColumn(column: string) {
  const clean = column.trim();
  const prefix = clean.match(/^[A-Za-z]+/)?.[0] ?? clean;
  return prefix.replace(/[^a-zA-Z0-9]/g, "").toUpperCase().slice(0, 8) || "GROUP";
}

export function detectPrefixGroups(columns: string[]): PrefixGroup[] {
  const groups = new Map<string, string[]>();
  for (const column of columns) {
    const prefix = prefixForColumn(column);
    groups.set(prefix, [...(groups.get(prefix) ?? []), column]);
  }
  return [...groups.entries()]
    .map(([prefix, indicators]) => ({ prefix, indicators }))
    .filter((group) => group.indicators.length >= 2)
    .sort((left, right) => left.prefix.localeCompare(right.prefix));
}

export function filteredColumns(dataset: Dataset, query: string, filter: DataColumnFilter) {
  const normalizedQuery = query.trim().toLowerCase();
  return dataset.columns.filter((column) => {
    if (normalizedQuery && !column.toLowerCase().includes(normalizedQuery)) return false;
    if (filter === "all") return true;
    if (filter === "nonnumeric") return !isNumericColumn(dataset, column);
    if (filter === "missing_heavy") return (dataset.rowCount ?? dataset.rows.length) > 0 && missingCount(dataset, column) / (dataset.rowCount ?? dataset.rows.length) >= 0.25;
    return columnScale(dataset, column) === filter;
  });
}
