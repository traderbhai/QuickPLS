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

  it("accepts a complete indicator-free two-stage interaction", () => {
    const interaction: Node<ConstructData> = {
      id: "xm",
      position: { x: 1, y: 0 },
      data: {
        label: "X x M",
        shortName: "XM",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "m", outcome: "y", method: "two_stage_product_score" },
      },
    };
    const moderator: Node<ConstructData> = { id: "m", position: { x: 0, y: 1 }, data: { label: "Moderator", shortName: "M", mode: "reflective", indicators: ["m1"] } };
    expect(validateModel([...nodes, moderator, interaction], [
      ...edges,
      { id: "m-y", source: "m", target: "y" },
      { id: "xm-y", source: "xm", target: "y" },
    ])).toEqual([]);
  });

  it("accepts a valid indicator-free higher-order construct placeholder", () => {
    const higherOrder: Node<ConstructData> = {
      id: "hoc",
      position: { x: 2, y: 0 },
      data: {
        label: "Higher-order construct",
        shortName: "HOC",
        mode: "reflective",
        indicators: [],
        semantic: "higher_order",
        higherOrder: { id: "hoc", components: ["x", "y"], method: "two_stage", stage_one_recipe: null },
      },
    };

    expect(validateModel([...nodes, higherOrder], edges)).toEqual([]);
  });

  it("blocks incomplete and infeasible higher-order declarations before dispatch", () => {
    const malformed: Node<ConstructData> = {
      id: "hoc",
      position: { x: 2, y: 0 },
      data: {
        label: "Higher-order construct",
        shortName: "HOC",
        mode: "reflective",
        indicators: [],
        semantic: "higher_order",
        higherOrder: { id: "wrong-id", components: ["x"], method: "hybrid", stage_one_recipe: null },
      },
    };
    expect(validateModel([...nodes, malformed], edges)).toContainEqual({ code: "higher_order.invalid", subject: "hoc" });

    const infeasible = {
      ...malformed,
      data: {
        ...malformed.data,
        higherOrder: { id: "hoc", components: ["x", "missing", "x"], method: "hybrid" as const, stage_one_recipe: null },
      },
    };
    const singleIndicatorX = nodes.map((node) => node.id === "x" ? { ...node, data: { ...node.data, indicators: ["x1"] } } : node);
    const issues = validateModel([...singleIndicatorX, infeasible], edges);
    expect(issues).toEqual(expect.arrayContaining([
      { code: "higher_order.unknown_component", subject: "hoc:missing" },
      { code: "higher_order.duplicate_component", subject: "hoc:x" },
      { code: "higher_order.hybrid_component_indicators", subject: "hoc:x" },
    ]));
  });

  it("blocks incomplete and multiple moderation relationships", () => {
    const interaction = (id: string): Node<ConstructData> => ({
      id,
      position: { x: 1, y: 0 },
      data: {
        label: id,
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "missing", outcome: "y", method: "two_stage_product_score" },
      },
    });
    const issues = validateModel([...nodes, interaction("xm"), interaction("xm2")], edges);
    expect(issues).toContainEqual({ code: "interaction.multiple", subject: "model" });
    expect(issues).toContainEqual({ code: "interaction.invalid", subject: "xm" });
  });
});
