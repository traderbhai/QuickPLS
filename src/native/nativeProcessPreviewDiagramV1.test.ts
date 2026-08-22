import { describe, expect, it } from "vitest";
import type { NativeProcessGraphRelationshipConfig } from "../types";
import { nativeProcessPreviewDiagramV1 } from "./nativeProcessPreviewDiagramV1";

describe("native PROCESS diagram preview v1", () => {
  it("reuses the locked diagram projection with moderator-only nodes and semantic anchors", () => {
    const source: NativeProcessGraphRelationshipConfig = {
      model: "graph",
      focal_predictor: "X",
      paths: [{ from: "X", to: "M" }, { from: "M", to: "Y" }, { from: "X", to: "Y" }],
      moderators: [{ variable: "W", scale: "continuous" }, { variable: "Z", scale: "binary_0_1" }],
      moderations: [
        { from: "X", to: "M", moderator: "W" },
        { from: "X", to: "Y", moderator: "W", conditioning_moderator: "Z" },
      ],
      continuous_product_centering: "equation_complete_case_mean_v1",
    };
    const before = structuredClone(source);
    const preview = nativeProcessPreviewDiagramV1(source, "Y");

    expect(preview.nodes.map((node) => node.id)).toEqual(expect.arrayContaining(["X", "M", "Y", "W", "Z"]));
    expect(preview.nodes.filter((node) => node.type === "moderationAnchor").map((node) => node.data.order).sort()).toEqual([2, 3]);
    expect(preview.nodes.every((node) => node.draggable === false)).toBe(true);
    expect(preview.edges.filter((edge) => edge.data?.relationshipKind === "process_preview_path")
      .every((edge) => edge.data?.visualOnly === true)).toBe(true);
    expect(preview.edges.filter((edge) => edge.data?.relationshipKind === "moderation_connector")).toHaveLength(3);
    expect(source).toEqual(before);
  });
});
