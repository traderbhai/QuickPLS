import { describe, expect, it } from "vitest";
import {
  nativeIndicatorDragLabelV1,
  nativeIndicatorDragSelectionV1,
  nextNativeIndicatorSelectionV1,
} from "./nativeIndicatorSelectionV1";
import { planNativeIndicatorGroupActionV1 } from "./nativeIndicatorGroupActionV1";

const visible = ["item-1", "item-2", "group", "item-3", "item-4"];

describe("Windows-native indicator selection and grouped actions", () => {
  it("supports single-click, Ctrl toggle, Shift range, and Ctrl+Shift extension in visible order", () => {
    const single = nextNativeIndicatorSelectionV1({
      visible,
      current: { selected: ["item-4"], anchor: "item-4" },
      indicator: "item-2",
      toggle: false,
      range: false,
    });
    expect(single).toEqual({ selected: ["item-2"], anchor: "item-2" });

    const toggled = nextNativeIndicatorSelectionV1({
      visible,
      current: single,
      indicator: "item-4",
      toggle: true,
      range: false,
    });
    expect(toggled).toEqual({ selected: ["item-2", "item-4"], anchor: "item-4" });

    const range = nextNativeIndicatorSelectionV1({
      visible,
      current: toggled,
      indicator: "item-2",
      toggle: false,
      range: true,
    });
    expect(range).toEqual({ selected: ["item-2", "group", "item-3", "item-4"], anchor: "item-4" });

    const extended = nextNativeIndicatorSelectionV1({
      visible,
      current: { selected: ["item-1"], anchor: "item-2" },
      indicator: "item-4",
      toggle: true,
      range: true,
    });
    expect(extended).toEqual({ selected: ["item-1", "item-2", "group", "item-3", "item-4"], anchor: "item-2" });
  });

  it("drags the complete current selection only when the grabbed indicator belongs to it", () => {
    expect(nativeIndicatorDragSelectionV1(["item-1", "item-3"], "item-3"))
      .toEqual(["item-1", "item-3"]);
    expect(nativeIndicatorDragSelectionV1(["item-1", "item-3"], "item-4"))
      .toEqual(["item-4"]);
    expect(nativeIndicatorDragLabelV1(["item-1", "item-2", "item-3", "item-4"]))
      .toBe("4 indicators: item-1, item-2, item-3, +1");
  });

  it("turns one filtered multi-selection into one deterministic gateway command", () => {
    expect(planNativeIndicatorGroupActionV1(
      visible,
      ["item-4", "group", "item-2", "item-2", "not-visible"],
      { kind: "create_construct", constructId: "construct:new", label: "  Service quality  ", position: { x: 40, y: 80 } },
      "group",
    )).toEqual({
      status: "ready",
      indicatorCount: 2,
      command: {
        kind: "add_construct",
        constructId: "construct:new",
        label: "Service quality",
        columns: ["item-2", "item-4"],
        position: { x: 40, y: 80 },
      },
    });

    expect(planNativeIndicatorGroupActionV1(
      visible,
      ["item-3", "item-1"],
      { kind: "assign_indicators", constructId: "construct:existing" },
    )).toMatchObject({
      status: "ready",
      indicatorCount: 2,
      command: { kind: "assign_indicators", constructId: "construct:existing", columns: ["item-1", "item-3"] },
    });
  });

  it("blocks a reserved-only group action instead of emitting an empty mutation", () => {
    expect(planNativeIndicatorGroupActionV1(
      visible,
      ["group"],
      { kind: "assign_indicators", constructId: "construct:existing" },
      "group",
    )).toEqual({
      status: "blocked",
      code: "empty_selection",
      message: "Select one or more available indicators.",
    });
  });
});
