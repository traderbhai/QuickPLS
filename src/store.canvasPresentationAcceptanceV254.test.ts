import { beforeEach, describe, expect, it } from "vitest";
import { modelFingerprint } from "./domain/diagramGraph";
import { planNativeIndicatorGroupActionV1 } from "./native/nativeIndicatorGroupActionV1";
import { useWorkspace } from "./store";

describe("QuickPLS 2.54 Canvas grouped and presentation transactions", () => {
  beforeEach(() => useWorkspace.getState().resetProject());

  it("creates one construct from many indicators as one undoable scientific transaction", async () => {
    const before = useWorkspace.getState();
    const source = before.nodes.find((node) => node.id === "competence")!;
    const beforeNodes = structuredClone(before.nodes);
    const historyLength = before.past.length;
    const plan = planNativeIndicatorGroupActionV1(
      before.dataset.columns,
      ["COMP2", "COMP1"],
      {
        kind: "create_construct",
        constructId: "construct:service-quality",
        label: "Service Quality",
        position: { x: 920, y: 420 },
      },
    );
    if (plan.status !== "ready") throw new Error(plan.message);

    await expect(before.executeModelEditCommand(plan.command)).resolves.toMatchObject({
      status: "applied",
      transaction: "scientific",
      undoable: true,
      stableIdsPreserved: true,
      affected: {
        constructIds: expect.arrayContaining([source.id, "construct:service-quality"]),
        indicatorIds: ["COMP1", "COMP2"],
      },
    });
    const created = useWorkspace.getState();
    expect(created.past).toHaveLength(historyLength + 1);
    expect(created.nodes.find((node) => node.id === "construct:service-quality")).toMatchObject({
      position: { x: 920, y: 420 },
      data: { label: "Service Quality", indicators: ["COMP1", "COMP2"] },
    });
    expect(created.nodes.find((node) => node.id === source.id)?.data.indicators)
      .toEqual(["COMP3"]);

    created.undo();
    expect(useWorkspace.getState().nodes).toEqual(beforeNodes);
  });

  it("moves an indicator through side, free, and automatic states without changing science", async () => {
    const state = useWorkspace.getState();
    const construct = state.nodes.find((node) => node.id === "competence")!;
    const column = "COMP1";
    const scientificBefore = modelFingerprint(state.nodes, state.edges);
    const historyLength = state.past.length;

    await expect(state.executeModelEditCommand({
      kind: "set_indicator_side",
      constructId: construct.id,
      column,
      side: "right",
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    expect(useWorkspace.getState().diagramLayout.indicatorLayouts[construct.id][column])
      .toMatchObject({ side: "right", pinned: true });

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "move_indicator",
      constructId: construct.id,
      column,
      position: { x: 24, y: 66 },
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    expect(useWorkspace.getState().diagramLayout.indicatorLayouts[construct.id][column])
      .toMatchObject({ side: "free", x: 24, y: 66, pinned: true });

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "reset_indicator_layout",
      constructId: construct.id,
      column,
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    const reset = useWorkspace.getState().diagramLayout.indicatorLayouts[construct.id][column];
    expect(reset.side).not.toBe("free");
    expect(reset.pinned).not.toBe(true);
    expect(reset.x).toBeUndefined();
    expect(reset.y).toBeUndefined();
    expect(useWorkspace.getState().past).toHaveLength(historyLength + 3);
    expect(modelFingerprint(useWorkspace.getState().nodes, useWorkspace.getState().edges))
      .toBe(scientificBefore);
  });

  it("preserves pins and manual overrides during whole-model Arrange and keeps local Align local", async () => {
    const state = useWorkspace.getState();
    const pinnedId = "competence";
    const selectedIds = ["satisfaction", "loyalty"];
    const unrelatedId = "likeability";
    const scientificBefore = modelFingerprint(state.nodes, state.edges);

    await state.executeModelEditCommand({ kind: "set_construct_pinned", constructId: pinnedId, pinned: true });
    await useWorkspace.getState().executeModelEditCommand({
      kind: "set_construct_indicator_side",
      constructId: pinnedId,
      side: "bottom",
    });
    await useWorkspace.getState().executeModelEditCommand({
      kind: "move_construct",
      constructId: "loyalty",
      position: { x: 1_400, y: 900 },
    });
    const beforeArrange = useWorkspace.getState();
    const arrangeSnapshot = structuredClone({
      nodes: beforeArrange.nodes,
      diagramLayout: beforeArrange.diagramLayout,
    });
    const pinnedPosition = { ...beforeArrange.nodes.find((node) => node.id === pinnedId)!.position };
    const pinnedIndicators = structuredClone(beforeArrange.diagramLayout.indicatorLayouts[pinnedId]);

    await expect(beforeArrange.executeModelEditCommand({ kind: "arrange_model", direction: "horizontal" }))
      .resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    const arranged = useWorkspace.getState();
    expect(arranged.nodes.find((node) => node.id === pinnedId)?.position).toEqual(pinnedPosition);
    expect(arranged.diagramLayout.constructLayouts[pinnedId].pinned).toBe(true);
    expect(arranged.diagramLayout.indicatorLayouts[pinnedId]).toEqual(pinnedIndicators);
    expect(modelFingerprint(arranged.nodes, arranged.edges)).toBe(scientificBefore);

    arranged.undo();
    expect(useWorkspace.getState().nodes).toEqual(arrangeSnapshot.nodes);
    expect(useWorkspace.getState().diagramLayout).toEqual(arrangeSnapshot.diagramLayout);

    const beforeAlign = useWorkspace.getState();
    const alignSnapshot = structuredClone({ nodes: beforeAlign.nodes, diagramLayout: beforeAlign.diagramLayout });
    const unrelatedPosition = { ...beforeAlign.nodes.find((node) => node.id === unrelatedId)!.position };
    const pinnedBeforeAlign = { ...beforeAlign.nodes.find((node) => node.id === pinnedId)!.position };
    await expect(beforeAlign.executeModelEditCommand({
      kind: "align_constructs",
      constructIds: [pinnedId, ...selectedIds],
      target: "top",
    })).resolves.toMatchObject({
      status: "applied",
      transaction: "presentation",
      affected: { constructIds: ["loyalty"] },
    });
    const aligned = useWorkspace.getState();
    expect(aligned.nodes.find((node) => node.id === unrelatedId)?.position).toEqual(unrelatedPosition);
    expect(aligned.nodes.find((node) => node.id === pinnedId)?.position).toEqual(pinnedBeforeAlign);
    expect(aligned.nodes.find((node) => node.id === "loyalty")?.position.y)
      .toBe(aligned.nodes.find((node) => node.id === "satisfaction")?.position.y);
    expect(modelFingerprint(aligned.nodes, aligned.edges)).toBe(scientificBefore);

    aligned.undo();
    expect(useWorkspace.getState().nodes).toEqual(alignSnapshot.nodes);
    expect(useWorkspace.getState().diagramLayout).toEqual(alignSnapshot.diagramLayout);
  });

  it("stores annotations as one undoable presentation transaction without changing science", async () => {
    const before = useWorkspace.getState();
    const layoutBefore = structuredClone(before.diagramLayout);
    const scientificBefore = modelFingerprint(before.nodes, before.edges);

    await expect(before.executeModelEditCommand({
      kind: "set_standard_sem_presentation",
      presentation: {
        schemaVersion: 1,
        objects: [{ kind: "note", id: "note:method", subject: "Method", text: "Review the focal path.", x: 520, y: 180 }],
      },
    })).resolves.toMatchObject({
      status: "applied",
      transaction: "presentation",
      undoable: true,
    });
    expect(useWorkspace.getState().diagramLayout.standardSemPresentation?.objects).toEqual([
      { kind: "note", id: "note:method", subject: "Method", text: "Review the focal path.", x: 520, y: 180 },
    ]);
    expect(modelFingerprint(useWorkspace.getState().nodes, useWorkspace.getState().edges)).toBe(scientificBefore);

    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout).toEqual(layoutBefore);
  });
});
