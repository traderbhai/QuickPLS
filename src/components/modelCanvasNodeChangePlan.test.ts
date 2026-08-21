import type { Node, NodeChange } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { indicatorNodeId } from "../domain/diagramGraph";
import { planModelCanvasNodeChanges } from "./modelCanvasNodeChangePlan";

const constructPosition: NodeChange<Node> = {
  type: "position",
  id: "construct-1",
  position: { x: 40, y: 20 },
  dragging: false,
};

const indicatorPosition: NodeChange<Node> = {
  type: "position",
  id: indicatorNodeId("construct-1", "item 1"),
  position: { x: 75, y: 90 },
  dragging: false,
};

describe("ModelCanvas node-change persistence plan", () => {
  it("keeps pointer position updates local until the drag-stop commit", () => {
    expect(planModelCanvasNodeChanges([constructPosition, indicatorPosition], true)).toEqual({
      modelChanges: [],
      constructKeyboardPositions: [],
      indicatorKeyboardPositions: [],
    });
  });

  it("routes React Flow keyboard positions through the model-edit gateway", () => {
    const plan = planModelCanvasNodeChanges([constructPosition, indicatorPosition], false);

    expect(plan.modelChanges).toEqual([]);
    expect(plan.constructKeyboardPositions).toEqual([{
      constructId: "construct-1",
      position: { x: 40, y: 20 },
    }]);
    expect(plan.indicatorKeyboardPositions).toEqual([{
      constructId: "construct-1",
      indicator: "item 1",
      position: { x: 75, y: 90 },
    }]);
  });

  it("does not create history for selection-only changes", () => {
    const selection: NodeChange<Node> = { type: "select", id: "construct-1", selected: true };

    expect(planModelCanvasNodeChanges([selection], false)).toEqual({
      modelChanges: [selection],
      constructKeyboardPositions: [],
      indicatorKeyboardPositions: [],
    });
  });
});
