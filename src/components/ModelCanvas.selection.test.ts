import { applyNodeChanges, type Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { focusedConstructSelectionChanges } from "./ModelCanvas";

const construct = (id: string, selected: boolean): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  selected,
  data: {
    label: id.toUpperCase(),
    shortName: id.toUpperCase(),
    mode: "reflective",
    indicators: [`${id}_1`],
  },
});

describe("ModelCanvas construct focus selection", () => {
  it("replaces a stale Canvas selection before React Flow Ctrl-selection extends it", () => {
    const nodes = [construct("x", false), construct("y", false), construct("z", true)];
    const changes = focusedConstructSelectionChanges(nodes, "x");

    expect(changes).toEqual([
      { type: "select", id: "x", selected: true },
      { type: "select", id: "z", selected: false },
    ]);
    expect(applyNodeChanges(changes, nodes).filter((node) => node.selected).map((node) => node.id)).toEqual(["x"]);
  });
});
