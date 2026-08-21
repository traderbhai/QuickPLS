import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings, ConstructData } from "../types";
import {
  canCreateNativeHigherOrder,
  nativeHigherOrderApproachOptions,
  nativeHigherOrderComponentOptions,
  nativeHigherOrderDraftIssues,
  nativeHigherOrderDraftProblems,
  nativeHigherOrderMeasurementType,
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

describe("native higher-order construct authoring", () => {
  it("keeps the command broadly eligible while applying exact approach rules inside the dialog", () => {
    const connected: Edge[] = [{ id: "y-x", source: "y", target: "x" }];
    const exactOptions = nativeHigherOrderComponentOptions(nodes, connected);
    expect(exactOptions.find((option) => option.id === "x")).toMatchObject({ eligible: false });
    expect(exactOptions.find((option) => option.id === "z")).toMatchObject({ eligible: true });
    expect(canCreateNativeHigherOrder(nodes, connected)).toBe(true);

    const formativeNodes: Array<Node<ConstructData>> = [
      { id: "f1", position: { x: 0, y: 0 }, data: { label: "Resources", shortName: "RES", mode: "formative", indicators: ["r1"] } },
      { id: "f2", position: { x: 0, y: 100 }, data: { label: "Processes", shortName: "PRO", mode: "formative", indicators: ["p1"] } },
    ];
    expect(canCreateNativeHigherOrder(formativeNodes, [{ id: "f1-f2", source: "f1", target: "f2" }])).toBe(true);
  });

  it("derives RR, RF, FR, and FF from dimension mode and conceptual direction", () => {
    expect(nativeHigherOrderMeasurementType("reflective", "hoc_explains_components")).toBe("reflective_reflective");
    expect(nativeHigherOrderMeasurementType("reflective", "components_form_hoc")).toBe("reflective_formative");
    expect(nativeHigherOrderMeasurementType("formative", "hoc_explains_components")).toBe("formative_reflective");
    expect(nativeHigherOrderMeasurementType("formative", "components_form_hoc")).toBe("formative_formative");
  });

  it("recommends a topology-aware valid approach and exposes unsupported choices locally", () => {
    const unconnected = nativeHigherOrderApproachOptions({
      nodes,
      edges: [],
      components: ["x", "z"],
      measurementType: "reflective_reflective",
      hocIsEndogenous: null,
    });
    expect(unconnected.find((option) => option.approach === "disjoint_two_stage")).toMatchObject({
      valid: true,
      recommended: true,
    });

    const connected = nativeHigherOrderApproachOptions({
      nodes,
      edges: [{ id: "x-y", source: "x", target: "y" }],
      components: ["x", "z"],
      measurementType: "reflective_reflective",
      hocIsEndogenous: false,
    });
    expect(connected.find((option) => option.approach === "disjoint_two_stage")).toMatchObject({ valid: false });
    expect(connected.find((option) => option.approach === "embedded_two_stage")).toMatchObject({
      valid: true,
      recommended: true,
    });
    expect(connected.find((option) => option.approach === "extended_repeated_indicators")).toMatchObject({ valid: false });
  });

  it("validates locally, retains current edit components, and applies topology to the selected approach", () => {
    expect(nativeHigherOrderDraftProblems(
      { name: "Standing", shortName: "HOC", components: ["x", "z"] },
      nodes,
      [],
    )).toEqual([]);
    expect(nativeHigherOrderDraftProblems(
      { name: "Capability", shortName: "CAP", components: ["x", "x"] },
      nodes,
      [],
    )).toEqual(expect.arrayContaining([
      "Choose each lower-order component only once.",
      "Choose a name that is not already used by another construct.",
      "Choose a short name that is not already used by another construct.",
    ]));

    const formative: Node<ConstructData> = {
      id: "f",
      position: { x: 0, y: 300 },
      data: { label: "Formative dimension", shortName: "FOR", mode: "formative", indicators: ["f1"] },
    };
    expect(nativeHigherOrderDraftIssues(
      { name: "Standing", shortName: "HOC", components: ["x", "f"] },
      [...nodes, formative],
      [],
    )).toEqual(expect.arrayContaining([
      expect.objectContaining({ field: "components", message: "All selected dimensions must use the same Mode A/B measurement." }),
    ]));

    const strictHoc: Node<ConstructData> = {
      ...hocNode,
      id: "derived:standing",
      data: {
        ...hocNode.data,
        higherOrder: {
          ...hocNode.data.higherOrder!,
          id: "term:standing",
          canonicalApproach: "disjoint_two_stage",
          measurementType: "reflective_reflective",
        },
      },
    };
    expect(nativeHigherOrderDraftProblems(
      {
        name: "Corporate standing",
        shortName: "HOC",
        components: ["x", "z"],
        approach: "disjoint_two_stage",
        measurementType: "reflective_reflective",
      },
      [...nodes, strictHoc],
      [{ id: "hoc-y", source: strictHoc.id, target: "y" }],
      { editingHigherOrderId: strictHoc.id, hocIsEndogenous: false },
    )).toEqual([]);

    expect(nativeHigherOrderDraftIssues(
      {
        name: "Standing",
        shortName: "HOC",
        components: ["x", "z"],
        approach: "repeated_indicators",
        measurementType: "reflective_formative",
      },
      nodes,
      [],
      { hocIsEndogenous: true },
    )).toEqual(expect.arrayContaining([
      expect.objectContaining({ field: "approach" }),
    ]));
  });
});

describe("native higher-order calculation scope", () => {
  it("accepts the bounded disjoint point/bootstrap workflow and incoming or outgoing HOC paths", () => {
    const edges: Edge[] = [{ id: "hoc-y", source: "hoc", target: "y" }];
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], edges, settings)).toEqual([]);
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], edges, { ...settings, method: "bootstrap", bootstrapSamples: 500 })).toEqual([]);
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], [...edges, { id: "x-y", source: "x", target: "y" }], settings)).toContain(
      "Lower-order components must remain measurement-only in the disjoint two-stage model",
    );
    expect(nativeHigherOrderScopeProblems([...nodes, hocNode], [{ id: "y-hoc", source: "y", target: "hoc" }], settings)).toEqual([]);
  });

  it("keeps strict HOC term and output identities distinct", () => {
    const strictHoc: Node<ConstructData> = {
      ...hocNode,
      id: "derived:hoc",
      data: {
        ...hocNode.data,
        higherOrder: {
          ...hocNode.data.higherOrder!,
          id: "term:hoc",
          canonicalApproach: "disjoint_two_stage",
          measurementType: "reflective_reflective",
        },
      },
    };
    expect(nativeHigherOrderScopeProblems(
      [...nodes, strictHoc],
      [{ id: "hoc-y", source: strictHoc.id, target: "y" }],
      settings,
    )).toEqual([]);
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
