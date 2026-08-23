import { describe, expect, it } from "vitest";
import {
  semEdgeBendEditCommand,
  semEdgeEditPermissions,
  semEdgeLabelInteractionContract,
  semEdgeResetRouteCommand,
} from "./SemEdge";

const measurementData = (routeEditable: boolean) => ({
  visualOnly: true,
  relationshipKind: "measurement_connector",
  constructId: "quality",
  indicator: "qual_1",
  routeEditable,
  perimeterRouting: "continuous",
  routing: "polyline",
  bendPoints: [{ x: 140, y: 90 }],
  edgeClassName: "measurement-edge reflective",
});

describe("SemEdge measurement connector editing", () => {
  it("allows only bend editing for an editable visual-only measurement connector", () => {
    expect(semEdgeEditPermissions(measurementData(true))).toEqual({
      bendPoints: true,
      moveLabel: false,
      deleteRelationship: false,
      reverseRelationship: false,
    });
  });

  it("blocks every edit for a locked or result-view measurement connector", () => {
    expect(semEdgeEditPermissions(measurementData(false))).toEqual({
      bendPoints: false,
      moveLabel: false,
      deleteRelationship: false,
      reverseRelationship: false,
    });
  });

  it("keeps other visual-only relationships noneditable while preserving structural path edits", () => {
    expect(semEdgeEditPermissions({ visualOnly: true, relationshipKind: "higher_order_membership" }))
      .toEqual({ bendPoints: false, moveLabel: false, deleteRelationship: false, reverseRelationship: false });
    expect(semEdgeEditPermissions({ visualOnly: false }))
      .toEqual({ bendPoints: true, moveLabel: true, deleteRelationship: true, reverseRelationship: true });
  });

  it("blocks label, bend, deletion, and reversal actions for an explicitly locked structural edge", () => {
    const locked = semEdgeEditPermissions({
      relationshipEditable: false,
      routing: "polyline",
      bendPoints: [{ x: 120, y: 80 }],
    });
    expect(locked).toEqual({
      bendPoints: false,
      moveLabel: false,
      deleteRelationship: false,
      reverseRelationship: false,
    });
    expect(semEdgeLabelInteractionContract("0.457", locked)).toEqual({ interactive: false });
  });

  it("exposes button semantics and deletion handling only for an editable structural label", () => {
    const editable = semEdgeEditPermissions({ relationshipEditable: true });
    expect(editable.deleteRelationship).toBe(true);
    expect(semEdgeLabelInteractionContract("0.457", editable)).toEqual({
      interactive: true,
      role: "button",
      tabIndex: 0,
      ariaLabel: "Move label for 0.457",
      title: "Drag to move label. Arrow keys nudge; Home resets.",
    });

    const lockedMeasurement = semEdgeEditPermissions(measurementData(false));
    expect(semEdgeLabelInteractionContract("0.910", lockedMeasurement)).toEqual({ interactive: false });
  });

  it("dispatches measurement-specific bend and reset commands without using the derived edge id as a relation", () => {
    const points = [{ x: 220, y: 80 }, { x: 300, y: 150 }];
    expect(semEdgeBendEditCommand("measurement::quality::qual_1", measurementData(true), points)).toEqual({
      kind: "set_measurement_connector_bend_points",
      constructId: "quality",
      column: "qual_1",
      points,
    });
    expect(semEdgeResetRouteCommand("measurement::quality::qual_1", measurementData(true))).toEqual({
      kind: "reset_measurement_connector_route",
      constructId: "quality",
      column: "qual_1",
    });
  });

  it("retains structural path commands for non-measurement edges", () => {
    const points = [{ x: 100, y: 120 }];
    expect(semEdgeBendEditCommand("relation:x-y", { visualOnly: false }, points)).toEqual({
      kind: "set_path_bend_points",
      relationId: "relation:x-y",
      points,
    });
    expect(semEdgeResetRouteCommand("relation:x-y", { visualOnly: false })).toEqual({
      kind: "reset_path_route",
      relationId: "relation:x-y",
    });
  });
});
