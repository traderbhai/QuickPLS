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
  it("renders one compact native setup with the automatic main-effect disclosure", () => {
    const markup = renderToStaticMarkup(<NativeModerationDialog nodes={nodes} edges={edges} selectedEdgeId="x-y" create={vi.fn(() => ({ status: "created" as const, interactionId: "xm" }))} close={vi.fn()} />);
    expect(markup).toContain("Predictor → Outcome");
    expect(markup).toContain("Moderator");
    expect(markup).toContain("Two-stage product score");
    expect(markup).toContain("adds the moderator’s main-effect path");
    expect(markup).toContain("Create moderating effect");
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

  it("announces multiple-interaction estimator gating and duplicate recovery in text", () => {
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

    expect(markup).toContain('role="status"');
    expect(markup).toContain('aria-live="polite"');
    expect(markup).toContain("already contains 2 moderating effects");
    expect(markup).toContain("Authoring does not mean every estimator can calculate it");
    expect(markup).toContain("Calculation readiness is checked separately");
    expect(markup).toContain('role="alert"');
    expect(markup).toContain("Choose another relationship or remove an existing moderating effect");
    expect(markup).toMatch(/type="submit"[^>]*disabled/);
  });
});
