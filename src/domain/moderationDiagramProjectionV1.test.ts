import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";
import { deriveModerationAnchorProjections, moderationAnchorPosition } from "./moderationDiagramProjectionV1";

const measured = (id: string, label: string): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  data: { label, shortName: id.toUpperCase(), mode: "reflective", indicators: [`${id}1`] },
});

const interaction = (id: string, operands: [string, string, ...string[]]): Node<ConstructData> => ({
  id,
  position: { x: 0, y: 0 },
  data: {
    label: id,
    shortName: id.toUpperCase(),
    mode: "formative",
    indicators: [],
    semantic: "interaction",
    interaction: {
      kind: "interaction_v2",
      termId: `term:${id}`,
      operands,
      focalRelationId: "x-y",
      outcome: "y",
      canonicalMethod: "two_stage",
      hierarchyPolicy: "strong",
    },
  },
});

describe("moderation diagram projection", () => {
  it("assigns stable symmetric slots for simultaneous two-way effects", () => {
    const nodes = [measured("x", "Predictor"), measured("w", "Moderator W"), measured("z", "Moderator Z"), measured("y", "Outcome"), interaction("xw", ["x", "w"]), interaction("xz", ["x", "z"])];
    const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];
    const projected = deriveModerationAnchorProjections(nodes, edges);
    expect(projected.map((item) => item.interactionTermId)).toEqual(["term:xw", "term:xz"]);
    expect(projected.map((item) => item.fraction)).toEqual([0.43, 0.57]);
    expect(projected.every((item) => item.visualOnly)).toBe(true);
  });

  it("folds strong lower-order closure into one researcher-facing three-way anchor", () => {
    const nodes = [
      measured("x", "Predictor"),
      measured("w", "Moderator W"),
      measured("z", "Moderator Z"),
      measured("y", "Outcome"),
      interaction("wx", ["w", "x"]),
      interaction("xw", ["x", "w"]),
      interaction("xwz", ["x", "w", "z"]),
    ];
    const projected = deriveModerationAnchorProjections(nodes, [{ id: "x-y", source: "x", target: "y" }]);
    expect(projected).toHaveLength(1);
    expect(projected[0]).toMatchObject({
      interactionTermId: "term:xwz",
      parentInteractionTermId: "term:xw",
      moderatorIds: ["w", "z"],
      order: 3,
    });
    expect(projected[0]?.label).toContain("Three-way moderating effect");
  });

  it("clamps optional presentation fractions without changing authority bytes", () => {
    expect(moderationAnchorPosition({ x: 0, y: 0 }, { x: 100, y: 40 }, 0.5)).toEqual({ x: 39, y: 9 });
    const nodes = [measured("x", "Predictor"), measured("w", "Moderator"), measured("y", "Outcome"), interaction("xw", ["x", "w"])];
    expect(deriveModerationAnchorProjections(nodes, [{ id: "x-y", source: "x", target: "y" }], { "term:xw": 9 })[0]?.fraction).toBe(0.8);
  });
});
