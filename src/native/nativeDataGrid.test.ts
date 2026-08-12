import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  nativeDataPage,
  nativeDataPageRows,
  nativeDataRangeLabel,
  nativeMissingCounts,
} from "./nativeDataGrid";

describe("native data grid paging", () => {
  it("bounds the rendered row window and preserves global offsets", () => {
    const page = nativeDataPage(10_005, 100, 73);
    const rows = Array.from({ length: 10_005 }, (_, index) => index);

    expect(page).toMatchObject({ pageIndex: 73, pageCount: 101, start: 7_300, end: 7_400 });
    expect(nativeDataPageRows(rows, page)).toHaveLength(100);
    expect(nativeDataPageRows(rows, page)[0]).toBe(7_300);
    expect(nativeDataRangeLabel(rows.length, page)).toBe("7301-7400 of 10005 cases");
  });

  it("clamps an out-of-range page after a dataset shrinks", () => {
    expect(nativeDataPage(21, 10, 99)).toMatchObject({
      pageIndex: 2,
      pageCount: 3,
      start: 20,
      end: 21,
      hasPrevious: true,
      hasNext: false,
    });
    expect(nativeDataRangeLabel(0, nativeDataPage(0, 100, 0))).toBe("0 cases");
  });
});

describe("native data grid column profiles", () => {
  it("counts null and absent values once for every requested column", () => {
    const counts = nativeMissingCounts(
      ["service", "trust", "score"],
      [
        { service: 5, trust: null, score: 0 },
        { service: null, trust: "", score: 3 },
        { service: 4, trust: 2 },
      ],
    );

    expect(Object.fromEntries(counts)).toEqual({ service: 1, trust: 1, score: 1 });
  });
});

describe("native data surface integration", () => {
  it("uses the authoritative case count and fetches only the current native page", () => {
    const source = readFileSync("src/native/NativeDataSurface.tsx", "utf8");

    expect(source).toContain("const rowCount = dataset.rowCount ?? dataset.rows.length");
    expect(source).toContain("getNativeDatasetRows(dataset.id, page.start, pageSize)");
    expect(source).toContain("nativeDataPageRows(dataset.rows, page)");
    expect(source).toContain("aria-rowcount={rowCount + 1}");
    expect(source).toContain("visibleRows.map((row, localIndex)");
    expect(source).not.toContain("dataset.rows.map(");
    expect(source).toContain("Loading cases...");
    expect(source).toContain("Could not load this data page.");
    expect(source).toContain("useNativeScientificGrid({");
    expect(source).toContain('role="grid"');
    expect(source).toContain('aria-keyshortcuts="Control+C"');
    expect(source).toContain("dataGrid.cellProps(localIndex, columnIndex)");
    expect(source).toContain("nativeGridClipboardText(");
  });

  it("shows only truthful current quality and import details", () => {
    const source = readFileSync("src/native/NativeDataSurface.tsx", "utf8");

    expect(source).toContain("dataset.missingByColumn");
    expect(source).toContain("nativeMissingCounts(dataset.columns, dataset.rows)");
    expect(source).not.toContain("dataset.rows.filter(");
    expect(source).toContain("Data Quality");
    expect(source).toContain("Import Details");
    expect(source).toContain("dataset.fingerprint?.trim()");
    expect(source).not.toContain("Import history");
  });
});
