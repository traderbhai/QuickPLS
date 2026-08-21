import { describe, expect, it } from "vitest";
import {
  nativeCommandsFor,
  resolveNativeCommand,
  type NativeCommandContext,
} from "./nativeCommands";
import {
  NATIVE_CANVAS_ARRANGE_MENU_V1,
  NATIVE_CANVAS_FIT_MENU_V1,
} from "./nativeCanvasCommandMenusV1";

function editableModelContext(overrides: Partial<NativeCommandContext> = {}): NativeCommandContext {
  return {
    surface: "model",
    projectOpen: true,
    projectWritable: true,
    hasDataset: true,
    hasCompletedRun: false,
    selectedResultSaved: false,
    canOpenContextModel: true,
    canCalculate: true,
    canUndo: false,
    canRedo: false,
    canRecode: false,
    canConfigureGroups: false,
    selectedVariableIsGrouping: false,
    canAddModeration: true,
    canAddHigherOrder: true,
    hasActiveModel: true,
    calculationReady: true,
    canConfigureConditionalProcess: true,
    canOpenAdvancedParameters: true,
    selectedConstructPinned: false,
    propertiesOpen: true,
    selection: { kind: "none", count: 0 },
    calculationStatus: "idle",
    ...overrides,
  };
}

describe("QuickPLS 2.54 Canvas command surface", () => {
  it("keeps exactly seven primary Canvas commands in the approved order", () => {
    const commands = nativeCommandsFor(
      { kind: "toolbar", surface: "model" },
      editableModelContext(),
    );

    expect(commands.map(({ id, label, action }) => ({ id, label, action }))).toEqual([
      { id: "select-tool", label: "Select", action: { id: "model.set-tool", tool: "select" } },
      { id: "pan-tool", label: "Pan", action: { id: "model.set-tool", tool: "pan" } },
      { id: "add-construct", label: "Construct", action: { id: "model.add-construct" } },
      { id: "path-tool", label: "Connect", action: { id: "model.set-tool", tool: "path" } },
      { id: "arrange-model", label: "Arrange", action: { id: "model.arrange", strategy: "model-horizontal" } },
      { id: "fit-model", label: "Fit", action: { id: "model.fit", scope: "structure" } },
      { id: "open-calculation", label: "Calculate…", action: { id: "calculation.open" } },
    ]);
    expect(commands.map((command) => command.id)).not.toEqual(expect.arrayContaining([
      "add-higher-order",
      "add-moderating-effect",
      "prepare-calculation-ready",
      "open-conditional-process",
      "open-advanced-parameters",
    ]));
  });

  it("offers the complete local Arrange and Fit operations as typed actions", () => {
    expect(NATIVE_CANVAS_ARRANGE_MENU_V1.map((item) => item.action)).toEqual([
      { id: "model.arrange", strategy: "tidy-selection" },
      { id: "model.arrange", strategy: "align-left" },
      { id: "model.arrange", strategy: "align-center" },
      { id: "model.arrange", strategy: "align-right" },
      { id: "model.arrange", strategy: "align-top" },
      { id: "model.arrange", strategy: "align-middle" },
      { id: "model.arrange", strategy: "align-bottom" },
      { id: "model.arrange", strategy: "distribute-horizontal" },
      { id: "model.arrange", strategy: "distribute-vertical" },
      { id: "model.arrange", strategy: "model-horizontal" },
      { id: "model.arrange", strategy: "model-vertical" },
    ]);
    expect(NATIVE_CANVAS_FIT_MENU_V1.map((item) => item.action)).toEqual([
      { id: "model.fit", scope: "structure" },
      { id: "model.fit", scope: "all" },
      { id: "model.fit", scope: "selection" },
    ]);
  });

  it("adapts Pin without adding it to the primary toolbar", () => {
    const selected = editableModelContext({ selection: { kind: "construct", count: 1 } });
    expect(resolveNativeCommand("toggle-pin", selected)).toMatchObject({
      label: "Pin Construct",
      visible: true,
      enabled: true,
      action: { id: "model.toggle-pin" },
    });
    expect(resolveNativeCommand("toggle-pin", { ...selected, selectedConstructPinned: true }).label)
      .toBe("Unpin Construct");
    expect(nativeCommandsFor({ kind: "toolbar", surface: "model" }, selected).map((command) => command.id))
      .not.toContain("toggle-pin");
  });
});
