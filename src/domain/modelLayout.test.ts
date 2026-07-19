import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";
import { layoutModel } from "./modelLayout";

const node = (id: string): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  data: { label: id, shortName: id.toUpperCase(), mode: "reflective", indicators: [] },
});

describe("layoutModel", () => {
  it("places dependent constructs to the right of their predictors", () => {
    const nodes = [node("x"), node("m"), node("y")];
    const edges: Edge[] = [
      { id: "x-m", source: "x", target: "m" },
      { id: "m-y", source: "m", target: "y" },
    ];
    const result = layoutModel(nodes, edges);
    expect(result.find((item) => item.id === "x")!.position.x).toBeLessThan(result.find((item) => item.id === "m")!.position.x);
    expect(result.find((item) => item.id === "m")!.position.x).toBeLessThan(result.find((item) => item.id === "y")!.position.x);
  });

  it("keeps constructs in the same stage vertically separated", () => {
    const result = layoutModel([node("x1"), node("x2"), node("y")], [
      { id: "x1-y", source: "x1", target: "y" },
      { id: "x2-y", source: "x2", target: "y" },
    ]);
    expect(result.find((item) => item.id === "x1")!.position.y).not.toBe(result.find((item) => item.id === "x2")!.position.y);
  });

  it("places dependent constructs below predictors in vertical layout", () => {
    const result = layoutModel([node("x"), node("y")], [{ id: "x-y", source: "x", target: "y" }], "vertical");
    expect(result.find((item) => item.id === "x")!.position.y).toBeLessThan(result.find((item) => item.id === "y")!.position.y);
  });
});
