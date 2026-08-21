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

  it("validates strong three-way interaction_v2 lower-order closure", () => {
    const moderators: Array<Node<ConstructData>> = [
      { id: "z", position: { x: 0, y: 1 }, data: { label: "Moderator Z", shortName: "Z", mode: "reflective", indicators: ["z1"] } },
      { id: "w", position: { x: 0, y: 2 }, data: { label: "Moderator W", shortName: "W", mode: "reflective", indicators: ["w1"] } },
    ];
    const lowerOrder = (id: string, predictor: string, moderator: string): Node<ConstructData> => ({
      id,
      position: { x: 1, y: 0 },
      data: {
        label: id,
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor, moderator, outcome: "y", method: "two_stage_product_score" },
      },
    });
    const highOrder: Node<ConstructData> = {
      id: "xzw",
      position: { x: 2, y: 0 },
      data: {
        label: "X × Z × W",
        shortName: "XZW",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: "interaction:x-z-w",
          operands: ["x", "z", "w"],
          outcome: "y",
          focalRelationId: "x-y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
          productIndicator: null,
        },
      },
    };
    const lowerOrders = [
      lowerOrder("xz", "x", "z"),
      lowerOrder("xw", "x", "w"),
      lowerOrder("zw", "z", "w"),
    ];
    const completeEdges: Edge[] = [
      ...edges,
      { id: "z-y", source: "z", target: "y" },
      { id: "w-y", source: "w", target: "y" },
      ...lowerOrders.map((interaction) => ({ id: `${interaction.id}-y`, source: interaction.id, target: "y" })),
      { id: "xzw-y", source: "xzw", target: "y" },
    ];

    expect(validateModel([...nodes, ...moderators, ...lowerOrders, highOrder], completeEdges)).toEqual([]);
    expect(validateModel([...nodes, ...moderators, ...lowerOrders.slice(1), highOrder], completeEdges))
      .toContainEqual({ code: "interaction.invalid", subject: "xzw" });
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

  it("accepts multiple distinct interactions on the same focal path", () => {
    const moderators: Array<Node<ConstructData>> = [
      { id: "m1", position: { x: 0, y: 1 }, data: { label: "Moderator 1", shortName: "M1", mode: "reflective", indicators: ["m1"] } },
      { id: "m2", position: { x: 0, y: 2 }, data: { label: "Moderator 2", shortName: "M2", mode: "reflective", indicators: ["m2"] } },
    ];
    const interaction = (id: string, moderator: string): Node<ConstructData> => ({
      id,
      position: { x: 1, y: 0 },
      data: {
        label: id,
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator, outcome: "y", method: "two_stage_product_score" },
      },
    });
    expect(validateModel([...nodes, ...moderators, interaction("xm1", "m1"), interaction("xm2", "m2")], [
      ...edges,
      { id: "m1-y", source: "m1", target: "y" },
      { id: "m2-y", source: "m2", target: "y" },
      { id: "xm1-y", source: "xm1", target: "y" },
      { id: "xm2-y", source: "xm2", target: "y" },
    ])).toEqual([]);
  });

  it("accepts multiple distinct interactions on different focal paths", () => {
    const moderator: Node<ConstructData> = { id: "m", position: { x: 0, y: 1 }, data: { label: "Moderator", shortName: "M", mode: "reflective", indicators: ["m1"] } };
    const secondOutcome: Node<ConstructData> = { id: "z", position: { x: 2, y: 1 }, data: { label: "Second outcome", shortName: "Z", mode: "reflective", indicators: ["z1"] } };
    const interaction = (id: string, outcome: string): Node<ConstructData> => ({
      id,
      position: { x: 1, y: 0 },
      data: {
        label: id,
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "m", outcome, method: "two_stage_product_score" },
      },
    });
    expect(validateModel([...nodes, moderator, secondOutcome, interaction("xm-y", "y"), interaction("xm-z", "z")], [
      ...edges,
      { id: "x-z", source: "x", target: "z" },
      { id: "m-y", source: "m", target: "y" },
      { id: "m-z", source: "m", target: "z" },
      { id: "xm-y-effect", source: "xm-y", target: "y" },
      { id: "xm-z-effect", source: "xm-z", target: "z" },
    ])).toEqual([]);
  });

  it("rejects duplicate and incomplete interaction declarations without rejecting distinct multiplicity", () => {
    const interaction = (id: string, moderator = "m"): Node<ConstructData> => ({
      id,
      position: { x: 1, y: 0 },
      data: {
        label: id,
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator, outcome: "y", method: "two_stage_product_score" },
      },
    });
    const moderator: Node<ConstructData> = { id: "m", position: { x: 0, y: 1 }, data: { label: "Moderator", shortName: "M", mode: "reflective", indicators: ["m1"] } };
    const duplicateIssues = validateModel([...nodes, moderator, interaction("xm"), interaction("xm-copy")], [
      ...edges,
      { id: "m-y", source: "m", target: "y" },
      { id: "xm-y", source: "xm", target: "y" },
      { id: "xm-copy-y", source: "xm-copy", target: "y" },
    ]);
    expect(duplicateIssues).toContainEqual({ code: "interaction.duplicate", subject: "xm-copy" });

    const incompleteIssues = validateModel([...nodes, interaction("invalid", "missing")], edges);
    expect(incompleteIssues).toContainEqual({ code: "interaction.invalid", subject: "invalid" });
  });
});
