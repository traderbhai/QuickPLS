import { describe, expect, it } from "vitest";
import {
  contextMenuCoordinates,
  isContextMenuKeyboardGesture,
  nextEnabledItemIndex,
  nextMenuIndex,
} from "./nativeMenuNavigation";

describe("native menu keyboard navigation", () => {
  it("wraps between top-level menus", () => {
    expect(nextMenuIndex(0, 6, -1)).toBe(5);
    expect(nextMenuIndex(5, 6, 1)).toBe(0);
  });

  it("moves through enabled popup items and supports Home and End", () => {
    const disabled = [false, true, false, false];
    expect(nextEnabledItemIndex(disabled, 0, "ArrowDown")).toBe(2);
    expect(nextEnabledItemIndex(disabled, 0, "ArrowUp")).toBe(3);
    expect(nextEnabledItemIndex(disabled, 2, "Home")).toBe(0);
    expect(nextEnabledItemIndex(disabled, 2, "End")).toBe(3);
  });

  it("recognizes both Windows context-menu keyboard gestures", () => {
    expect(isContextMenuKeyboardGesture("ContextMenu", false)).toBe(true);
    expect(isContextMenuKeyboardGesture("F10", true)).toBe(true);
    expect(isContextMenuKeyboardGesture("F10", false)).toBe(false);
  });

  it("keeps a context menu inside the visible desktop workspace", () => {
    expect(contextMenuCoordinates(1000, 690, 1024, 700)).toEqual({ x: 776, y: 456 });
    expect(contextMenuCoordinates(-10, -12, 1024, 700)).toEqual({ x: 4, y: 4 });
  });
});
