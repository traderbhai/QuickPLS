import { describe, expect, it } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";
import {
  canAddNativeModeration,
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

  it("enforces the validated single-interaction scope", () => {
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
    expect(canAddNativeModeration([...nodes, interaction], [{ id: "x-y", source: "x", target: "y" }])).toBe(false);
  });
});
