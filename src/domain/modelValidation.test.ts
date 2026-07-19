import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";
import { validateModel } from "./modelValidation";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "y", position: { x: 1, y: 1 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];

describe("validateModel", () => {
  it("accepts a structurally valid model", () => expect(validateModel(nodes, edges)).toEqual([]));

  it("rejects indicators assigned to multiple constructs", () => {
    const duplicate = nodes.map((node) => ({ ...node, data: { ...node.data, indicators: node.id === "y" ? ["x1"] : node.data.indicators } }));
    expect(validateModel(duplicate, edges)).toContainEqual({ code: "indicator.duplicate", subject: "x1" });
  });

  it("rejects self-referential structural paths", () => {
    expect(validateModel(nodes, [{ id: "self", source: "x", target: "x" }])).toContainEqual({ code: "path.self", subject: "self" });
  });

  it("rejects duplicate structural paths", () => {
    const duplicate = [...edges, { id: "x-y-copy", source: "x", target: "y" }];
    expect(validateModel(nodes, duplicate)).toContainEqual({ code: "path.duplicate", subject: "x-y-copy" });
  });

  it("rejects directed structural cycles", () => {
    const cycle = [...edges, { id: "y-x", source: "y", target: "x" }];
    expect(validateModel(nodes, cycle)).toContainEqual({ code: "path.cycle", subject: "model" });
  });

  it("ignores visual covariance arcs for structural validation", () => {
    expect(validateModel(nodes, [...edges, { id: "cov", source: "y", target: "x", data: { role: "covariance" } }])).toEqual([]);
  });
});
