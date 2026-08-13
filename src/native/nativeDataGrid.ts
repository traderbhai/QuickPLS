import type { Dataset } from "../types";

export const DEFAULT_NATIVE_DATA_PAGE_SIZE = 100;
export const NATIVE_DATA_PAGE_SIZES = [50, 100, 250] as const;

export interface NativeDataPage {
  pageIndex: number;
  pageCount: number;
  start: number;
  end: number;
  hasPrevious: boolean;
  hasNext: boolean;
}

export function nativeDataPage(rowCount: number, pageSize: number, requestedPage: number): NativeDataPage {
  const safeRowCount = Math.max(0, Math.trunc(rowCount));
  const safePageSize = Math.max(1, Math.trunc(pageSize));
  const pageCount = Math.max(1, Math.ceil(safeRowCount / safePageSize));
  const pageIndex = Math.min(pageCount - 1, Math.max(0, Math.trunc(requestedPage)));
  const start = pageIndex * safePageSize;
  const end = Math.min(safeRowCount, start + safePageSize);

  return {
    pageIndex,
    pageCount,
    start,
    end,
    hasPrevious: pageIndex > 0,
    hasNext: pageIndex < pageCount - 1,
  };
}

export function nativeDataPageRows<T>(rows: readonly T[], page: Pick<NativeDataPage, "start" | "end">): readonly T[] {
  return rows.slice(page.start, page.end);
}

export function nativeMissingCounts(
  columns: readonly string[],
  rows: Dataset["rows"],
): ReadonlyMap<string, number> {
  const counts = new Map(columns.map((column) => [column, 0]));

  for (const row of rows) {
    for (const column of columns) {
      if (row[column] == null) counts.set(column, (counts.get(column) ?? 0) + 1);
    }
  }

  return counts;
}

export function nativeDataRangeLabel(rowCount: number, page: Pick<NativeDataPage, "start" | "end">): string {
  if (rowCount <= 0) return "0 cases";
  return `${page.start + 1}-${page.end} of ${rowCount} cases`;
}
