import { readFileSync } from "node:fs";
import type { Edge, EdgeChange, Node, NodeChange } from "@xyflow/react";
import { describe, expect, it } from "vitest";
import { persistentModelEdgeChanges, persistentModelNodeChanges } from "./ModelCanvas";

describe("ModelCanvas strict Standard authority routing", () => {
  it("routes canvas mutations through the shared command gateway while retaining layout actions", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain("executeModelEditCommand");
    expect(source).toContain("runModelEditCommand");
    for (const kind of ["add_construct", "assign_indicators", "add_path", "move_construct", "move_indicator", "set_standard_sem_presentation", "arrange_model", "remove_path"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
    expect(source).toContain('persistentChanges.filter((change) => change.type !== "remove")');
    expect(source).toContain("if (strictAuthority) return;");
    expect(source).toContain("StandardSemPresentationLayer");
    expect(source).toContain("if (!strictAuthority || !canEditLayout) return;");
    expect(source).not.toContain('commitStrict({ kind: "caption"');
  });

  it("reconnects legacy paths but blocks strict retargeting behind a versioned revision", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain("if (!strictAuthority) {");
    expect(source).toContain("reconnectPath(edge, connection);");
    expect(source).toContain("Retargeting a calculation-ready relationship needs a versioned revision.");
  });

  it("provides a non-editing Results presentation without borrowing a legacy run overlay", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain('presentation?: "editor" | "results_readonly";');
    expect(source).toContain('readOnlyResultsPresentation ? "smartpls_result" : diagramMode');
    expect(source).toContain("readOnlyResultsPresentation ? undefined : selectedResultRun");
    expect(source).toContain("if (readOnlyResultsPresentation) return;");
    expect(source).toContain("data-model-canvas-presentation={presentation}");
  });

  it("passes selected HOC membership to the visual graph and never persists visual-only edges", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain("selectedHigherOrderId: selectedNodeId");
    expect(source).toContain("persistentModelEdgeChanges(changes, graph.edges)");

    const scientific: Edge = { id: "path:x-y", source: "x", target: "y" };
    const membership: Edge = {
      id: "hoc-membership::hoc::component",
      source: "component",
      target: "hoc",
      data: { visualOnly: true },
    };
    const measurement: Edge = { id: "measurement::x::x1", source: "x", target: "indicator::x::x1" };
    const changes: EdgeChange[] = [
      { type: "remove", id: scientific.id },
      { type: "remove", id: membership.id },
      { type: "remove", id: measurement.id },
      { type: "add", item: { ...membership, id: "hoc-membership::hoc::another-component" } },
    ];

    expect(persistentModelEdgeChanges(changes, [scientific, membership, measurement])).toEqual([
      { type: "remove", id: scientific.id },
    ]);
  });

  it("keeps measurement connector selection separate from scientific path and context actions", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain("selectedMeasurementConnector,");
    expect(source).toContain("layoutEditingEnabled: canEditLayout,");
    expect(source).toContain("setSelectedMeasurementConnector({ constructId, indicator });");
    expect(source).toContain("if (canEditLayout) selectMeasurementConnector(measurementConnector.constructId, measurementConnector.indicator);");
    expect(source).toContain('window.setTimeout(() => window.dispatchEvent(new CustomEvent("quickpls:model-inspector-show-appearance")), 0);');
    expect(source).not.toContain('requestNativeContextMenu(event, { kind: "path", id: measurementConnector');
  });

  it("never persists presentation-only moderation anchors", () => {
    const scientific: Node = { id: "x", position: { x: 0, y: 0 }, data: {} };
    const anchor: Node = { id: "moderation-anchor::term", position: { x: 20, y: 20 }, data: { visualOnly: true } };
    const changes: NodeChange[] = [
      { type: "position", id: scientific.id, position: { x: 10, y: 10 } },
      { type: "select", id: anchor.id, selected: true },
      { type: "remove", id: anchor.id },
      { type: "add", item: { ...anchor, id: "moderation-anchor::another" } },
    ];
    expect(persistentModelNodeChanges(changes, [scientific, anchor])).toEqual([
      { type: "position", id: scientific.id, position: { x: 10, y: 10 } },
    ]);
  });
});
