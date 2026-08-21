import type { Node, NodeChange, XYPosition } from "@xyflow/react";
import { isIndicatorNodeId, parseIndicatorNodeId } from "../domain/diagramGraph";

export interface IndicatorKeyboardPositionChange {
  constructId: string;
  indicator: string;
  position: XYPosition;
}

export interface ConstructKeyboardPositionChange {
  constructId: string;
  position: XYPosition;
}

export interface ModelCanvasNodeChangePlan {
  modelChanges: Array<NodeChange<Node>>;
  constructKeyboardPositions: ConstructKeyboardPositionChange[];
  indicatorKeyboardPositions: IndicatorKeyboardPositionChange[];
}

/**
 * Pointer drags stay local until drag-stop, while React Flow keyboard moves
 * have no drag-stop callback and therefore must be persisted immediately.
 */
export function planModelCanvasNodeChanges(
  changes: Array<NodeChange<Node>>,
  pointerDragActive: boolean,
): ModelCanvasNodeChangePlan {
  const modelChanges: Array<NodeChange<Node>> = [];
  const constructKeyboardPositions: ConstructKeyboardPositionChange[] = [];
  const indicatorKeyboardPositions: IndicatorKeyboardPositionChange[] = [];

  for (const change of changes) {
    if (!("id" in change) || !isIndicatorNodeId(change.id)) {
      if (!pointerDragActive && change.type === "position" && change.position) {
        constructKeyboardPositions.push({ constructId: change.id, position: change.position });
      } else if (change.type !== "position") {
        modelChanges.push(change);
      }
      continue;
    }

    if (pointerDragActive || change.type !== "position" || !change.position) continue;
    const indicator = parseIndicatorNodeId(change.id);
    if (!indicator) continue;
    indicatorKeyboardPositions.push({ ...indicator, position: change.position });
  }

  return { modelChanges, constructKeyboardPositions, indicatorKeyboardPositions };
}
