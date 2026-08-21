import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Edge, Node } from "@xyflow/react";
import type { ConstructData } from "../types";
import NativeModerationDialog from "./NativeModerationDialog";

const nodes: Array<Node<ConstructData>> = [
  { id: "x", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1"] } },
  { id: "m", position: { x: 0, y: 100 }, data: { label: "Moderator", shortName: "M", mode: "reflective", indicators: ["m1"] } },
  { id: "y", position: { x: 200, y: 0 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
];
const edges: Edge[] = [{ id: "x-y", source: "x", target: "y" }];

describe("NativeModerationDialog", () => {
  it("renders one compact native setup with qualified construction details", () => {
    const markup = renderToStaticMarkup(<NativeModerationDialog nodes={nodes} edges={edges} selectedEdgeId="x-y" create={vi.fn(() => ({ status: "created" as const, interactionId: "xm" }))} close={vi.fn()} />);
    expect(markup).toContain("Predictor → Outcome");
    expect(markup).toContain("Moderator");
    expect(markup).toContain("Two-way moderation");
    expect(markup).toContain("Two-stage");
    expect(markup).toContain("Strong");
    expect(markup).toContain("Add moderating effect");
    expect(markup).toContain('for="nd-moderation-relationship"');
    expect(markup).toContain('id="nd-moderation-relationship"');
    expect(markup).toContain('for="nd-moderation-moderator"');
    expect(markup).toContain('id="nd-moderation-moderator"');
  });

  it("shows an actionable blocker when no structural relationship exists", () => {
    const markup = renderToStaticMarkup(<NativeModerationDialog nodes={nodes} edges={[]} create={vi.fn(() => ({ status: "blocked" as const, reason: "focal_path_missing" as const }))} close={vi.fn()} />);
    expect(markup).toContain('role="alert"');
    expect(markup).toContain("Create and select a structural path");
    expect(markup).toMatch(/type="submit"[^>]*disabled/);
  });

  it("keeps technical construction details collapsed and duplicate recovery local", () => {
    const interaction = (id: string, moderator: string): Node<ConstructData> => ({
      id,
      position: { x: 100, y: 100 },
      data: {
        label: id,
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: { predictor: "x", moderator, outcome: "y", method: "two_stage_product_score" },
      },
    });
    const secondModerator: Node<ConstructData> = { id: "m2", position: { x: 0, y: 200 }, data: { label: "Moderator 2", shortName: "M2", mode: "reflective", indicators: ["m2"] } };
    const markup = renderToStaticMarkup(<NativeModerationDialog
      nodes={[...nodes, secondModerator, interaction("xm", "m"), interaction("xm2", "m2")]}
      edges={edges}
      selectedEdgeId="x-y"
      create={vi.fn(() => ({ status: "blocked" as const, reason: "duplicate_interaction" as const }))}
      close={vi.fn()}
    />);

    expect(markup).toContain("Advanced");
    expect(markup).toContain("Construction");
    expect(markup).toContain("Strong");
    expect(markup).toContain('role="alert"');
    expect(markup).toContain("Add another measured construct");
    expect(markup).toMatch(/type="submit"[^>]*disabled/);
  });

  it("uses the same compact dialog contract to edit an existing effect", () => {
    const interaction: Node<ConstructData> = {
      id: "xm",
      position: { x: 100, y: 100 },
      data: {
        label: "X × M",
        shortName: "XM",
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId: "term:xm",
          operands: ["x", "m"],
          focalRelationId: "x-y",
          outcome: "y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
        },
      },
    };
    const markup = renderToStaticMarkup(<NativeModerationDialog
      nodes={[...nodes, interaction]}
      edges={edges}
      request={{ kind: "edit", interactionTermId: "term:xm" }}
      commit={vi.fn(() => ({ status: "updated" as const, interactionTermId: "term:xm" }))}
      close={vi.fn()}
    />);
    expect(markup).toContain("Save changes");
    expect(markup).toContain("Two-way moderation");
    expect(markup).not.toContain("Moderating-effect editing is not connected");
    expect(markup).toContain("Predictor → Outcome");
    expect(markup).not.toMatch(/id="nd-moderation-relationship"[^>]*disabled/);
  });

  it("blocks a second three-way effect before submission", () => {
    const interaction = (
      id: string,
      termId: string,
      operands: [string, string, ...string[]],
    ): Node<ConstructData> => ({
      id,
      position: { x: 100, y: 100 },
      data: {
        label: operands.join(" × "),
        shortName: id.toUpperCase(),
        mode: "formative",
        indicators: [],
        semantic: "interaction",
        interaction: {
          kind: "interaction_v2",
          termId,
          operands,
          focalRelationId: "x-y",
          outcome: "y",
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
        },
      },
    });
    const secondModerator: Node<ConstructData> = {
      id: "z",
      position: { x: 0, y: 200 },
      data: { label: "Second moderator", shortName: "Z", mode: "reflective", indicators: ["z1"] },
    };
    const thirdModerator: Node<ConstructData> = {
      id: "q",
      position: { x: 0, y: 300 },
      data: { label: "Another moderator", shortName: "Q", mode: "reflective", indicators: ["q1"] },
    };
    const markup = renderToStaticMarkup(<NativeModerationDialog
      nodes={[
        ...nodes,
        secondModerator,
        thirdModerator,
        interaction("xm", "term:xm", ["x", "m"]),
        interaction("xmz", "term:xmz", ["x", "m", "z"]),
      ]}
      edges={edges}
      request={{ kind: "create", target: { kind: "parent_interaction", interactionTermId: "term:xm" } }}
      commit={vi.fn(() => ({ status: "created" as const, interactionId: "xmq" }))}
      close={vi.fn()}
    />);

    expect(markup).toContain("already has its supported three-way moderating effect");
    expect(markup).toMatch(/type="submit"[^>]*disabled/);
  });
});
