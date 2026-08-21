import type { Edge, Node } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import type { ConstructData } from "../types";
import { nativeModelNavigatorRelationshipsV1 } from "./nativeModelNavigatorV1";

const measured = (id: string, label: string): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  data: { label, shortName: id.toUpperCase(), mode: "reflective", indicators: [`${id}1`] },
});

describe("native model Relationships navigator", () => {
  it("uses authored labels and suppresses technical, measurement, and visual-only rows", () => {
    const nodes: Array<Node<ConstructData>> = [
      measured("x-internal", "Motivation"),
      measured("w-internal", "Gender"),
      measured("y-internal", "Performance"),
      {
        id: "generated:interaction:xw",
        position: { x: 0, y: 0 },
        data: {
          label: "generated internal product",
          shortName: "XW",
          mode: "formative",
          indicators: [],
          semantic: "interaction",
          interaction: {
            kind: "interaction_v2",
            termId: "term:motivation-gender",
            operands: ["x-internal", "w-internal"],
            focalRelationId: "relation:x-y",
            outcome: "y-internal",
            canonicalMethod: "two_stage",
            hierarchyPolicy: "strong",
          },
        },
      },
      {
        id: "hoc:quality",
        position: { x: 0, y: 0 },
        data: {
          label: "Service Quality",
          shortName: "SQ",
          mode: "reflective",
          indicators: [],
          semantic: "higher_order",
          higherOrder: {
            id: "hoc-term:quality",
            components: ["x-internal", "y-internal"],
            method: "two_stage",
            canonicalApproach: "disjoint_two_stage",
            measurementType: "reflective_reflective",
          },
        },
      },
    ];
    const edges: Edge[] = [
      { id: "relation:x-y", source: "x-internal", target: "y-internal" },
      { id: "covariance:w-y", source: "w-internal", target: "y-internal", data: { role: "covariance" } },
      { id: "measurement::x::x1", source: "x-internal", target: "x-internal" },
      { id: "technical", source: "x-internal", target: "y-internal", data: { technicalGenerated: true } },
      { id: "visual", source: "w-internal", target: "y-internal", data: { visualOnly: true } },
      { id: "generated-path", source: "generated:interaction:xw", target: "y-internal" },
    ];

    const rows = nativeModelNavigatorRelationshipsV1(nodes, edges);
    expect(rows).toEqual(expect.arrayContaining([
      {
        id: "relationship:relation:x-y",
        kind: "relationship",
        relationId: "relation:x-y",
        label: "Motivation → Performance",
        detail: "Structural relationship",
      },
      {
        id: "relationship:covariance:w-y",
        kind: "relationship",
        relationId: "covariance:w-y",
        label: "Gender ↔ Performance",
        detail: "Covariance",
      },
      {
        id: "moderation:term:motivation-gender",
        kind: "moderation",
        interactionTermId: "term:motivation-gender",
        label: "Gender moderates Motivation → Performance",
        detail: "Two-way moderation",
      },
      {
        id: "higher-order:hoc:quality",
        kind: "higher_order",
        constructId: "hoc:quality",
        label: "Service Quality: Motivation, Performance",
        detail: "Higher-order components",
      },
    ]));
    expect(rows).toHaveLength(4);
    expect(rows.map((row) => row.label).join("\n")).not.toContain("generated internal product");
    expect(rows.map((row) => row.label).join("\n")).not.toContain("x-internal");
  });

  it("labels an ordered three-way relationship from authored constructs", () => {
    const nodes: Array<Node<ConstructData>> = [
      measured("x", "Ability"),
      measured("w", "Group"),
      measured("z", "Brain activity"),
      measured("y", "Insomnia"),
      {
        id: "generated:three-way",
        position: { x: 0, y: 0 },
        data: {
          label: "technical",
          shortName: "XWZ",
          mode: "formative",
          indicators: [],
          semantic: "interaction",
          interaction: {
            kind: "interaction_v2",
            termId: "term:x-w-z",
            operands: ["x", "w", "z"],
            focalRelationId: "x-y",
            outcome: "y",
            canonicalMethod: "two_stage",
            hierarchyPolicy: "strong",
          },
        },
      },
    ];
    const rows = nativeModelNavigatorRelationshipsV1(nodes, [{ id: "x-y", source: "x", target: "y" }]);
    expect(rows.find((row) => row.kind === "moderation")).toMatchObject({
      label: "Brain activity extends Ability × Group → Insomnia",
      detail: "Three-way moderation",
    });
  });
});
