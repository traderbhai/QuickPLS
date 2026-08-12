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
  });

  it("shows an actionable blocker when no structural relationship exists", () => {
    const markup = renderToStaticMarkup(<NativeModerationDialog nodes={nodes} edges={[]} create={vi.fn(() => ({ status: "blocked" as const, reason: "focal_path_missing" as const }))} close={vi.fn()} />);
    expect(markup).toContain('role="alert"');
    expect(markup).toContain("Create and select a structural path");
    expect(markup).toMatch(/type="submit"[^>]*disabled/);
  });
});
