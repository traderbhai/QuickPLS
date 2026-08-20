import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, ConstructData } from "../types";
import {
  canCreateNativeHigherOrder,
  nativeHigherOrderComponentOptions,
  nativeHigherOrderDraftProblems,
  nativeHigherOrderScopeProblems,
} from "./nativeHigherOrder";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["x1", "x2"] } },
  { id: "z", position: { x: 0, y: 180 }, data: { label: "Reputation", shortName: "REP", mode: "reflective", indicators: ["z1", "z2"] } },
  { id: "y", position: { x: 500, y: 90 }, data: { label: "Loyalty", shortName: "LOY", mode: "reflective", indicators: ["y1", "y2"] } },
];

const settings: AnalysisUiSettings = {
  method: "pls_pm",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20260811,
  workers: 1,
  confidenceLevel: 0.95,
  weightingScheme: "path",
  preprocessing: "standardized",
};

const hocNode: Node<ConstructData> = {
  id: "hoc",
  position: { x: 260, y: 90 },
  data: {
    label: "Corporate standing",
    shortName: "HOC",
    mode: "reflective",
    indicators: [],
    semantic: "higher_order",
    higherOrder: { id: "hoc", components: ["x", "z"], method: "two_stage", stage_one_recipe: null },
  },
};

describe("native higher-order construct scope", () => {
  it("offers only measured reflective components that are not structurally connected", () => {
    const options = nativeHigherOrderComponentOptions(nodes, [{ id: "y-x", source: "y", target: "x" }]);
    expect(options.find((option) => option.id === "x")).toMatchObject({ eligible: false });
    expect(options.find((option) => option.id === "z")).toMatchObject({ eligible: true });
    expect(canCreateNativeHigherOrder(nodes, [])).toBe(true);
  });

  it("validates names, uniqueness, and at least two eligible components", () => {
    expect(nativeHigherOrderDraftProblems({ name: "Standing", shortName: "HOC", components: ["x", "z"] }, nodes, [])).toEqual([]);
    expect(nativeHigherOrderDraftProblems({ name: "Capability", shortName: "CAP", components: ["x", "x"] }, nodes, [])).toEqual(expect.arrayContaining([
      "Choose each lower-order component only once.",
      "Choose a name that is not already used by another construct.",
      "Choose a short name that is not already used by another construct.",
    ]));
  });

  it("accepts the bounded disjoint point/bootstrap workflow and incoming or outgoing HOC paths", () => {
    const edges: Edge[] = [{ id: "hoc-y", source: "hoc", target: "y" }];
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], edges, settings)).toEqual([]);
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], edges, { ...settings, method: "bootstrap", bootstrapSamples: 500 })).toEqual([]);
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], [...edges, { id: "x-y", source: "x", target: "y" }], settings)).toContain(
      "Lower-order components must remain measurement-only in the disjoint two-stage model",
    );
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], [{ id: "y-hoc", source: "y", target: "hoc" }], settings)).toEqual([]);
  });

  it("applies the exact repeated and extended-repeated topology matrix", () => {
    const repeated = {
      ...hocNode,
      data: {
        ...hocNode.data,
        higherOrder: {
          ...hocNode.data.higherOrder!,
          canonicalApproach: "repeated_indicators" as const,
          measurementType: "reflective_formative" as const,
        },
        mode: "formative" as const,
      },
    };
    expect(nativeHigherOrderScopeProblems([...nodes, repeated], [{ id: "hoc-y", source: "hoc", target: "y" }], settings)).toEqual([]);
    expect(nativeHigherOrderScopeProblems([...nodes, repeated], [{ id: "y-hoc", source: "y", target: "hoc" }], settings)).toContain(
      "The chosen approach/HCM type does not support the higher-order construct's current exogenous/endogenous position",
    );

    const extended = {
      ...repeated,
      data: {
        ...repeated.data,
        higherOrder: {
          ...repeated.data.higherOrder!,
          canonicalApproach: "extended_repeated_indicators" as const,
        },
      },
    };
    expect(nativeHigherOrderScopeProblems([...nodes, extended], [{ id: "y-hoc", source: "y", target: "hoc" }], settings)).toEqual([]);
  });
});
