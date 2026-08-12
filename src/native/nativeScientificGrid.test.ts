import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { nativeGridClipboardText, nativeGridPositionForKey } from "./nativeScientificGrid";

describe("native scientific grid keyboard contract", () => {
  it("moves one cell with arrow keys and never leaves the rendered grid", () => {
    expect(nativeGridPositionForKey({ rowIndex: 1, columnIndex: 1 }, "ArrowLeft", 3, 4))
      .toEqual({ rowIndex: 1, columnIndex: 0 });
    expect(nativeGridPositionForKey({ rowIndex: 1, columnIndex: 1 }, "ArrowRight", 3, 4))
      .toEqual({ rowIndex: 1, columnIndex: 2 });
    expect(nativeGridPositionForKey({ rowIndex: 1, columnIndex: 1 }, "ArrowUp", 3, 4))
      .toEqual({ rowIndex: 0, columnIndex: 1 });
    expect(nativeGridPositionForKey({ rowIndex: 1, columnIndex: 1 }, "ArrowDown", 3, 4))
      .toEqual({ rowIndex: 2, columnIndex: 1 });
    expect(nativeGridPositionForKey({ rowIndex: 0, columnIndex: 0 }, "ArrowLeft", 3, 4))
      .toEqual({ rowIndex: 0, columnIndex: 0 });
    expect(nativeGridPositionForKey({ rowIndex: 2, columnIndex: 3 }, "ArrowDown", 3, 4))
      .toEqual({ rowIndex: 2, columnIndex: 3 });
  });

  it("supports row, page, and whole-grid boundary navigation", () => {
    const current = { rowIndex: 4, columnIndex: 3 };
    expect(nativeGridPositionForKey(current, "Home", 10, 6)).toEqual({ rowIndex: 4, columnIndex: 0 });
    expect(nativeGridPositionForKey(current, "End", 10, 6)).toEqual({ rowIndex: 4, columnIndex: 5 });
    expect(nativeGridPositionForKey(current, "PageUp", 10, 6)).toEqual({ rowIndex: 0, columnIndex: 3 });
    expect(nativeGridPositionForKey(current, "PageDown", 10, 6)).toEqual({ rowIndex: 9, columnIndex: 3 });
    expect(nativeGridPositionForKey(current, "Home", 10, 6, { ctrlKey: true }))
      .toEqual({ rowIndex: 0, columnIndex: 0 });
    expect(nativeGridPositionForKey(current, "End", 10, 6, { ctrlKey: true }))
      .toEqual({ rowIndex: 9, columnIndex: 5 });
    expect(nativeGridPositionForKey(current, "End", 10, 6, { metaKey: true }))
      .toEqual({ rowIndex: 9, columnIndex: 5 });
  });

  it("ignores unrelated keys and grids with no selectable cells", () => {
    expect(nativeGridPositionForKey({ rowIndex: 0, columnIndex: 0 }, "Enter", 2, 2)).toBeNull();
    expect(nativeGridPositionForKey({ rowIndex: 0, columnIndex: 0 }, "ArrowDown", 0, 2)).toBeNull();
    expect(nativeGridPositionForKey({ rowIndex: 0, columnIndex: 0 }, "ArrowDown", 2, 0)).toBeNull();
  });
});

describe("native scientific grid clipboard values", () => {
  it("copies exact values while representing missing cells as empty text", () => {
    expect(nativeGridClipboardText(0)).toBe("0");
    expect(nativeGridClipboardText(false)).toBe("false");
    expect(nativeGridClipboardText("0.500000")).toBe("0.500000");
    expect(nativeGridClipboardText(null)).toBe("");
    expect(nativeGridClipboardText(undefined)).toBe("");
  });
});

describe("native scientific grid viewport contract", () => {
  it("contains wide tables inside the document pane", () => {
    const css = readFileSync("src/native/nativeDesktop.css", "utf8");
    const tableScroller = css.match(/\.nd-table-scroll \{[^}]+\}/)?.[0] ?? "";

    expect(tableScroller).toContain("max-width: 100%");
    expect(tableScroller).toContain("max-height: 100%");
    expect(tableScroller).toContain("overflow: auto");
    expect(tableScroller).toContain("overscroll-behavior: contain");
  });
});
