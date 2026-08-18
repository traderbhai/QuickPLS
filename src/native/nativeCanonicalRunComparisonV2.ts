import {
  compareCanonicalResultDocumentsV2,
  type CanonicalComparisonCellDeltaV2,
  type CanonicalComparisonIssueV2,
  type CanonicalResultComparisonDocumentV2,
  type CanonicalResultComparisonBuildV2,
} from "../domain/canonicalResultComparisonV2";
import type { AnalysisRun } from "../types";
import {
  canonicalResultDocumentFromAnalysisRunV2,
  type NativeCanonicalResultContextV2,
} from "./nativeCanonicalResultDocumentV2";

export type NativeCanonicalRunComparisonBuildV2 =
  | { status: "ready"; comparison: CanonicalResultComparisonDocumentV2 }
  | { status: "blocked"; issues: CanonicalComparisonIssueV2[] }
  | { status: "unavailable"; messages: string[] };

export interface CanonicalRunComparisonDisplayRowV2 {
  id: string;
  tableId: string;
  tableTitle: string;
  sourceRow: string;
  rowLabel: string;
  field: string;
  first: string;
  second: string;
  change: string;
  changed: boolean;
}

/**
 * Build an exact typed comparison from two completed runtime runs.
 *
 * The compatibility engine, rather than the UI, decides whether the data,
 * model, analysis components, settings, tables, rows, and value types match.
 */
export async function canonicalRunComparisonFromAnalysisRunsV2(
  first: AnalysisRun,
  second: AnalysisRun,
  context: NativeCanonicalResultContextV2 = {},
): Promise<NativeCanonicalRunComparisonBuildV2> {
  const [left, right] = await Promise.all([
    canonicalResultDocumentFromAnalysisRunV2(first, context),
    canonicalResultDocumentFromAnalysisRunV2(second, context),
  ]);
  if (!left.ok || !right.ok) {
    return {
      status: "unavailable",
      messages: [
        ...(!left.ok ? left.errors : []),
        ...(!right.ok ? right.errors : []),
      ],
    };
  }
  const built: CanonicalResultComparisonBuildV2 = compareCanonicalResultDocumentsV2(
    left.document,
    right.document,
  );
  return built.compatible
    ? { status: "ready", comparison: built.comparison }
    : { status: "blocked", issues: built.issues };
}

export function canonicalComparisonDisplayRowsV2(
  comparison: CanonicalResultComparisonDocumentV2,
): CanonicalRunComparisonDisplayRowV2[] {
  return comparison.tables.flatMap((table) => {
    const columnLabels = new Map(table.columns.map((column) => [column.id, column.label]));
    return table.rows.flatMap((row) => {
      const rowLabel = row.cells
        .filter((cell) => cell.kind === "text")
        .slice(0, 2)
        .map((cell) => `${columnLabels.get(cell.column_id) ?? cell.column_id}: ${firstValue(cell)}`)
        .join(" · ") || row.source_row_id;
      return row.cells.map((cell) => ({
        id: `${table.source_table_id}:${row.source_row_id}:${cell.column_id}`,
        tableId: table.source_table_id,
        tableTitle: table.title,
        sourceRow: row.source_row_id,
        rowLabel,
        field: columnLabels.get(cell.column_id) ?? cell.column_id,
        first: firstValue(cell),
        second: secondValue(cell),
        change: changeValue(cell),
        changed: cell.changed,
      }));
    });
  });
}

function firstValue(cell: CanonicalComparisonCellDeltaV2): string {
  if (cell.kind === "missing") return snapshotValue(cell.left);
  return displayPrimitive(cell.left);
}

function secondValue(cell: CanonicalComparisonCellDeltaV2): string {
  if (cell.kind === "missing") return snapshotValue(cell.right);
  return displayPrimitive(cell.right);
}

function changeValue(cell: CanonicalComparisonCellDeltaV2): string {
  if (cell.kind === "number") return displayNumber(cell.change);
  if (cell.kind === "missing") return cell.transition.replaceAll("_", " ");
  return cell.changed ? "changed" : "unchanged";
}

function snapshotValue(snapshot: Extract<CanonicalComparisonCellDeltaV2, { kind: "missing" }>["left"]): string {
  if (snapshot.kind === "missing") return `Not available (${snapshot.reason.replaceAll("_", " ")})`;
  return displayPrimitive(snapshot.value);
}

function displayPrimitive(value: number | string | boolean): string {
  if (typeof value === "number") return displayNumber(value);
  return typeof value === "boolean" ? (value ? "Yes" : "No") : value;
}

function displayNumber(value: number): string {
  if (Object.is(value, -0) || value === 0) return "0";
  return Number.isInteger(value) ? String(value) : String(Number(value.toPrecision(8)));
}
