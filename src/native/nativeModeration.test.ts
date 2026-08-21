import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";
import {
  canAddNativeModeration,
  nearestNativeModerationDropTarget,
  nativeModeratingEffects,
  nativeModerationCreationError,
  nativeModerationRelationships,
  nativeModeratorCandidates,
} from "./nativeModeration";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1"] } },
  { id: "m", position: { x: 0, y: 100 }, data: { label: "Moderator", shortName: "M", mode: "reflective", indicators: ["m1"] } },
  { id: "y", position: { x: 200, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
];

describe("native moderation authoring contract", () => {
  it("offers only substantive existing paths and eligible measured moderators", () => {
    const edges: Edge[] = [
      { id: "x-y", source: "x", target: "y" },
      { id: "control-m-y", source: "m", target: "y", data: { role: "control" } },
      { id: "cov-x-m", source: "x", target: "m", data: { role: "covariance" } },
      { id: "generated-m-y", source: "m", target: "y", data: { technicalGenerated: true } },
      { id: "visual-m-y", source: "m", target: "y", data: { visualOnly: true } },
      { id: "measurement::x::x1", source: "x", target: "x" },
    ];
    const relationships = nativeModerationRelationships(nodes, edges);
    expect(relationships).toEqual([expect.objectContaining({ edgeId: "x-y", predictor: "x", outcome: "y", label: "Predictor → Outcome" })]);
    expect(nativeModeratorCandidates(nodes, relationships[0])).toEqual([{ id: "m", label: "Moderator" }]);
    expect(canAddNativeModeration(nodes, edges)).toBe(false);
    expect(canAddNativeModeration(nodes, edges, "x-y")).toBe(false);
    expect(canAddNativeModeration(nodes, edges, "control-m-y")).toBe(false);
    const withoutControls = edges.filter((edge) => edge.id !== "control-m-y");
    expect(canAddNativeModeration(nodes, withoutControls)).toBe(true);
    expect(canAddNativeModeration(nodes, withoutControls, "x-y")).toBe(true);
    expect(nativeModerationCreationError("control_paths_unsupported")).toContain("Remove or convert control paths");
  });

  it("keeps distinct moderators eligible while filtering an exact duplicate", () => {
    const interaction: Node<ConstructData> = {
      id: "xm",
      position: { x: 100, y: 100 },
      data: {
        label: "X x M",
        shortName: "XM",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator: "m", outcome: "y", method: "two_stage_product_score" },
      },
    };
    const secondModerator: Node<ConstructData> = {
      id: "m2",
      position: { x: 0, y: 200 },
      data: { label: "Moderator 2", shortName: "M2", mode: "reflective", indicators: ["m2"] },
    };
    const relationship = nativeModerationRelationships(nodes, [{ id: "x-y", source: "x", target: "y" }])[0];
    expect(canAddNativeModeration([...nodes, interaction], [{ id: "x-y", source: "x", target: "y" }])).toBe(false);
    expect(nativeModeratorCandidates([...nodes, interaction, secondModerator], relationship)).toEqual([{ id: "m2", label: "Moderator 2" }]);
    expect(nativeModeratorCandidates([...nodes, interaction, secondModerator], relationship, undefined, true)).toEqual([
      { id: "m", label: "Moderator" },
      { id: "m2", label: "Moderator 2" },
    ]);
    expect(canAddNativeModeration([...nodes, interaction, secondModerator], [{ id: "x-y", source: "x", target: "y" }])).toBe(true);
    expect(nativeModerationCreationError("duplicate_interaction")).toContain("already define a moderating effect");
  });

  it("finds a generous path drop target without creating a scientific edge", () => {
    const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];
    const centers = new Map([
      ["x", { x: 20, y: 20 }],
      ["m", { x: 20, y: 120 }],
      ["y", { x: 220, y: 20 }],
    ]);
    const before = structuredClone(edges);
    const target = nearestNativeModerationDropTarget(nodes, edges, "m", { x: 120, y: 44 }, (id) => centers.get(id));
    expect(target?.relationship.edgeId).toBe("x-y");
    expect(target?.distance).toBe(24);
    expect(edges).toEqual(before);
    expect(nearestNativeModerationDropTarget(nodes, edges, "m", { x: 120, y: 80 }, (id) => centers.get(id))).toBeNull();
  });

  it("reads ordered three-way effects for edit presentation", () => {
    const parent: Node<ConstructData> = {
      id: "x-m",
      position: { x: 100, y: 100 },
      data: {
        label: "X × M",
        shortName: "XM",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: "term:x-m",
          operands: ["x", "m"],
          focalRelationId: "x-y",
          outcome: "y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
        },
      },
    };
    const reversed: Node<ConstructData> = {
      ...parent,
      id: "m-x",
      data: {
        ...parent.data,
        label: "M × X",
        interaction: {
          kind: "interaction_v2",
          termId: "term:m-x",
          operands: ["m", "x"],
          focalRelationId: "m-y",
          outcome: "y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
        },
      },
    };
    const threeWay: Node<ConstructData> = {
      id: "xmw",
      position: { x: 100, y: 100 },
      data: {
        label: "X × M × W",
        shortName: "XMW",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: "term:xmw",
          operands: ["x", "m", "w"],
          focalRelationId: "x-y",
          outcome: "y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
        },
      },
    };
    const moderatorW = measuredModerator("w", "Moderator W");
    expect(nativeModeratingEffects(
      [...nodes, moderatorW, reversed, parent, threeWay],
      [{ id: "x-y", source: "x", target: "y" }, { id: "m-y", source: "m", target: "y" }],
    )).toContainEqual(expect.objectContaining({
      interactionTermId: "term:xmw",
      parentInteractionTermId: "term:x-m",
      moderatorIds: ["m", "w"],
      order: 3,
    }));
  });
});

function measuredModerator(id: string, label: string): Node<ConstructData> {
  return { id, position: { x: 0, y: 200 }, data: { label, shortName: id.toUpperCase(), mode: "reflective", indicators: [`${id}1`] } };
}
