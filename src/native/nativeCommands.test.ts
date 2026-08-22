import { describe, expect, it, vi } from "vitest";
import type { RunMonitorStatus } from "../types";
import {
  NATIVE_COMMANDS,
  executeNativeCommand,
  formatNativeShortcut,
  nativeCommandForShortcut,
  nativeContextMenuCommands,
  nativeCommandsFor,
  resolveNativeCommand,
  type NativeCommandContext,
} from "./nativeCommands";

function context(overrides: Partial<NativeCommandContext> = {}): NativeCommandContext {
  return {
    surface: "launcher",
    projectOpen: false,
    projectWritable: true,
    hasDataset: false,
    hasCompletedRun: false,
    selectedResultSaved: false,
    canOpenContextModel: true,
    canCalculate: false,
    canUndo: false,
    canRedo: false,
    canRecode: true,
    canConfigureGroups: true,
    selectedVariableIsGrouping: false,
    canAddModeration: true,
    canAddHigherOrder: true,
    propertiesOpen: true,
    selection: { kind: "none", count: 0 },
    calculationStatus: "idle",
    ...overrides,
  };
}

function ids(commands: ReturnType<typeof nativeCommandsFor>): string[] {
  return commands.map((command) => command.id);
}

describe("native command registry", () => {
  it("has unique command ids and a semantic action for every command", () => {
    expect(new Set(NATIVE_COMMANDS.map((command) => command.id)).size).toBe(NATIVE_COMMANDS.length);
    expect(NATIVE_COMMANDS.every((command) => command.action.id.includes("."))).toBe(true);
  });

  it("keeps the launcher toolbar quiet and project-oriented", () => {
    expect(ids(nativeCommandsFor({ kind: "toolbar", surface: "launcher" }, context()))).toEqual([
      "new-project",
      "open-project",
    ]);
    expect(ids(nativeCommandsFor({ kind: "menu", menu: "file" }, context()))).toEqual(["new-project", "open-project", "open-demo-project"]);
    expect(nativeCommandsFor({ kind: "menu", menu: "calculate" }, context())).toEqual([]);
  });

  it("exposes only truthful Workspace Explorer commands for the active tree item", () => {
    const model = context({
      projectOpen: true,
      hasCompletedRun: true,
      selection: { kind: "project-model", count: 1 },
    });
    expect(ids(nativeCommandsFor({ kind: "toolbar", surface: "launcher" }, model))).toEqual([
      "new-project",
      "open-project",
      "new-model",
      "save-report",
    ]);
    expect(ids(nativeContextMenuCommands(model))).toEqual([
      "open-project-item",
      "new-model",
      "rename-project-item",
      "delete-project-item",
    ]);
    expect(resolveNativeCommand("rename-project-item", model).label).toBe("Rename Model…");
    expect(nativeCommandForShortcut({ key: "F2" }, model)?.id).toBe("rename-project-item");
    expect(nativeCommandForShortcut({ key: "Delete" }, model)?.id).toBe("delete-project-item");

    const report = { ...model, selection: { kind: "project-report" as const, count: 1 } };
    expect(resolveNativeCommand("rename-project-item", report).label).toBe("Rename Report…");
    expect(resolveNativeCommand("delete-project-item", report).label).toBe("Remove Report…");
  });

  it("locks Workspace Explorer mutations for read-only projects and active calculations", () => {
    const selected = context({
      projectOpen: true,
      projectWritable: false,
      hasCompletedRun: true,
      selection: { kind: "project-model", count: 1 },
    });
    expect(resolveNativeCommand("open-project-item", selected).enabled).toBe(true);
    expect(resolveNativeCommand("new-model", selected).enabled).toBe(false);
    expect(resolveNativeCommand("rename-project-item", selected).enabled).toBe(false);
    expect(resolveNativeCommand("delete-project-item", selected).enabled).toBe(false);
    expect(resolveNativeCommand("save-report", { ...selected, selection: { kind: "project-reports", count: 1 } }).enabled).toBe(false);
    expect(resolveNativeCommand("new-model", { ...selected, projectWritable: true, calculationStatus: "running" }).enabled).toBe(false);
  });

  it("offers Save Report only for a selected completed result without an existing alias", () => {
    const unsaved = context({
      surface: "results",
      projectOpen: true,
      hasCompletedRun: true,
      selection: { kind: "result", count: 1 },
    });
    expect(ids(nativeCommandsFor({ kind: "toolbar", surface: "results" }, unsaved))).toContain("save-report");
    expect(ids(nativeContextMenuCommands(unsaved))).toContain("save-report");

    const saved = { ...unsaved, selectedResultSaved: true };
    expect(ids(nativeCommandsFor({ kind: "toolbar", surface: "results" }, saved))).not.toContain("save-report");
    expect(ids(nativeContextMenuCommands(saved))).not.toContain("save-report");
  });

  it("derives data toolbar availability from the project and dataset", () => {
    const empty = nativeCommandsFor(
      { kind: "toolbar", surface: "data" },
      context({ surface: "data", projectOpen: true }),
    );
    expect(ids(empty)).toEqual(["import-data", "save-project", "go-model", "open-calculation"]);
    expect(empty.find((command) => command.id === "go-model")?.enabled).toBe(false);
    expect(empty.find((command) => command.id === "open-calculation")?.enabled).toBe(false);

    const loaded = nativeCommandsFor(
      { kind: "toolbar", surface: "data" },
      context({ surface: "data", projectOpen: true, hasDataset: true }),
    );
    expect(loaded.find((command) => command.id === "go-model")?.enabled).toBe(true);
    expect(loaded.find((command) => command.id === "open-calculation")).toMatchObject({
      label: "Analyze…",
      visible: true,
      enabled: true,
    });

    const importedWithoutModel = nativeCommandsFor(
      { kind: "toolbar", surface: "data" },
      context({
        surface: "data",
        projectOpen: true,
        hasDataset: true,
        canOpenContextModel: false,
      }),
    );
    expect(ids(importedWithoutModel)).toEqual([
      "import-data",
      "save-project",
      "new-model",
      "open-calculation",
    ]);
    expect(importedWithoutModel.find((command) => command.id === "new-model")).toMatchObject({
      label: "New Model…",
      visible: true,
      enabled: true,
    });
    expect(importedWithoutModel.find((command) => command.id === "go-model")).toBeUndefined();
  });

  it("surfaces recode only for a mutable selected data variable", () => {
    const selected = context({
      surface: "data",
      projectOpen: true,
      hasDataset: true,
      canRecode: true,
      selection: { kind: "variable", count: 1 },
    });
    expect(resolveNativeCommand("recode-variable", selected)).toMatchObject({ visible: true, enabled: true });
    expect(ids(nativeCommandsFor({ kind: "toolbar", surface: "data" }, selected))).toEqual([
      "import-data",
      "recode-variable",
      "configure-groups",
      "save-project",
      "go-model",
      "open-calculation",
    ]);
    expect(resolveNativeCommand("recode-variable", { ...selected, canRecode: false }).enabled).toBe(false);
    expect(resolveNativeCommand("recode-variable", { ...selected, calculationStatus: "running" }).enabled).toBe(false);
  });

  it("offers a truthful adaptive Data grouping command only for an eligible selected variable", () => {
    const selected = context({
      surface: "data",
      projectOpen: true,
      hasDataset: true,
      canConfigureGroups: true,
      selection: { kind: "variable", count: 1 },
    });
    expect(resolveNativeCommand("configure-groups", selected)).toMatchObject({
      label: "Use as Grouping Variable…",
      visible: true,
      enabled: true,
      action: { id: "data.configure-groups" },
    });
    expect(resolveNativeCommand("configure-groups", { ...selected, selectedVariableIsGrouping: true }).label).toBe("Edit Groups…");
    expect(resolveNativeCommand("configure-groups", { ...selected, canConfigureGroups: false }).enabled).toBe(false);
    expect(resolveNativeCommand("configure-groups", { ...selected, calculationStatus: "running" }).enabled).toBe(false);
  });

  it.each<RunMonitorStatus>(["queued", "validating", "running", "cancelling"])(
    "locks the registry-owned Data recode command while %s",
    (calculationStatus) => {
      const selected = context({
        surface: "data",
        projectOpen: true,
        hasDataset: true,
        canRecode: true,
        selection: { kind: "variable", count: 1 },
        calculationStatus,
      });

      expect(resolveNativeCommand("recode-variable", selected)).toMatchObject({ visible: true, enabled: false });
    },
  );

  it("derives model undo, redo, deletion, and calculation from live context", () => {
    const modelContext = context({
      surface: "model",
      projectOpen: true,
      hasDataset: true,
      canCalculate: true,
      canUndo: true,
      selection: { kind: "construct", count: 1 },
    });
    expect(resolveNativeCommand("undo", modelContext).enabled).toBe(true);
    expect(resolveNativeCommand("redo", modelContext).enabled).toBe(false);
    expect(resolveNativeCommand("edit-selection", modelContext)).toMatchObject({
      label: "Edit Construct Properties…",
      visible: true,
      enabled: true,
    });
    expect(resolveNativeCommand("open-calculation", modelContext).enabled).toBe(true);
    expect(ids(nativeCommandsFor({ kind: "context-menu", surface: "model" }, modelContext))).toEqual([
      "edit-selection",
      "delete-selection",
      "arrange-model",
      "toggle-pin",
      "fit-model",
      "focus-selection",
      "open-calculation",
    ]);

    const noSelection = { ...modelContext, selection: { kind: "none" as const, count: 0 } };
    expect(ids(nativeCommandsFor({ kind: "context-menu", surface: "model" }, noSelection))).not.toContain("delete-selection");
  });

  it("resolves pointer and keyboard model context menus through the same registry snapshot", () => {
    const unselected = context({
      surface: "model",
      projectOpen: true,
      hasDataset: true,
      canCalculate: true,
      selection: { kind: "none", count: 0 },
    });
    const selected = { kind: "construct" as const, count: 1 };
    const pointerCommands = nativeContextMenuCommands(unselected, selected);
    const keyboardCommands = nativeContextMenuCommands({ ...unselected, selection: selected });

    expect(pointerCommands.map(({ id, label, enabled }) => ({ id, label, enabled })))
      .toEqual(keyboardCommands.map(({ id, label, enabled }) => ({ id, label, enabled })));
    expect(ids(pointerCommands)).toContain("edit-selection");
    expect(ids(pointerCommands)).toContain("delete-selection");
  });

  it("keeps an immutable activated General SEM model revision-reachable from Model, path context, and shortcut", () => {
    const revision = context({
      surface: "model",
      projectOpen: true,
      projectWritable: false,
      hasDataset: true,
      canUndo: true,
      canAddModeration: true,
      moderationMutationAuthority: {
        kind: "general_sem_revision",
        available: true,
      },
      selection: { kind: "path", count: 1 },
    });

    const modelMenuModeration = nativeCommandsFor({ kind: "menu", menu: "model" }, revision)
      .find((command) => command.id === "add-moderating-effect");
    expect(modelMenuModeration).toMatchObject({
      label: "Moderating Effect (Save As Revision)…",
      enabled: true,
      action: { id: "model.add-moderating-effect" },
    });

    const contextModeration = nativeContextMenuCommands(revision)
      .find((command) => command.id === "add-moderating-effect");
    expect(contextModeration).toMatchObject({
      label: "Moderating Effect (Save As Revision)…",
      enabled: true,
    });
    expect(nativeCommandForShortcut({ key: "m" }, revision)?.id).toBe("add-moderating-effect");
    const dispatch = vi.fn();
    expect(executeNativeCommand("add-moderating-effect", revision, dispatch)).toBe(true);
    expect(dispatch).toHaveBeenCalledWith({ id: "model.add-moderating-effect" });
    expect(resolveNativeCommand("add-higher-order", revision)).toMatchObject({
      label: "Higher-Order Construct (Save As Revision)…",
      enabled: true,
    });

    for (const directMutation of [
      "save-project",
      "undo",
      "add-construct",
      "path-tool",
      "edit-selection",
      "delete-selection",
    ] as const) {
      expect(resolveNativeCommand(directMutation, revision).enabled).toBe(false);
    }
    expect(resolveNativeCommand("arrange-model", revision).enabled).toBe(true);
  });

  it("offers selected HOC editing from Model, context, Properties authority, and Enter semantics", () => {
    const hoc = context({
      surface: "model",
      projectOpen: true,
      projectWritable: false,
      hasDataset: true,
      canAddHigherOrder: false,
      selectedHigherOrder: true,
      moderationMutationAuthority: { kind: "general_sem_revision", available: true },
      selection: { kind: "construct", count: 1 },
    });

    expect(resolveNativeCommand("edit-selection", hoc)).toMatchObject({
      label: "Edit Higher-Order Construct…",
      enabled: true,
    });
    expect(resolveNativeCommand("edit-higher-order", hoc)).toMatchObject({
      visible: true,
      enabled: true,
      action: { id: "model.edit-selection" },
    });
    expect(nativeCommandsFor({ kind: "menu", menu: "model" }, hoc).map((command) => command.id))
      .toContain("edit-higher-order");
    expect(nativeCommandForShortcut({ key: "Enter" }, hoc)?.action).toEqual({ id: "model.edit-selection" });
  });

  it("routes Enter, Model menu, and context-menu edits through the same model action", () => {
    const selections = [
      context({
        surface: "model",
        projectOpen: true,
        hasDataset: true,
        selection: { kind: "construct", count: 1 },
      }),
      context({
        surface: "model",
        projectOpen: true,
        hasDataset: true,
        selection: { kind: "path", count: 1 },
      }),
      context({
        surface: "model",
        projectOpen: true,
        projectWritable: false,
        hasDataset: true,
        selectedHigherOrder: true,
        moderationMutationAuthority: { kind: "general_sem_revision", available: true },
        selection: { kind: "construct", count: 1 },
      }),
    ];

    for (const selected of selections) {
      const keyboard = nativeCommandForShortcut({ key: "Enter" }, selected);
      const applicationMenu = nativeCommandsFor({
        kind: "menu",
        menu: selected.selectedHigherOrder ? "model" : "edit",
      }, selected)
        .find((command) => command.action.id === "model.edit-selection");
      const contextMenu = nativeContextMenuCommands(selected)
        .find((command) => command.action.id === "model.edit-selection");
      expect(keyboard?.action).toEqual({ id: "model.edit-selection" });
      expect(applicationMenu?.action).toEqual({ id: "model.edit-selection" });
      expect(contextMenu?.action).toEqual({ id: "model.edit-selection" });

      const dispatch = vi.fn();
      expect(executeNativeCommand(keyboard!.id, selected, dispatch)).toBe(true);
      expect(dispatch).toHaveBeenCalledWith({ id: "model.edit-selection" });
    }
  });

  it("fails closed across every moderation entry point when revision authority is busy", () => {
    const reason = "Wait for General SEM archive publication to finish.";
    const blockedRevision = context({
      surface: "model",
      projectOpen: true,
      projectWritable: false,
      hasDataset: true,
      canAddModeration: true,
      moderationMutationAuthority: {
        kind: "general_sem_revision",
        available: false,
        disabledReason: reason,
      },
      selection: { kind: "path", count: 1 },
    });

    expect(resolveNativeCommand("add-moderating-effect", blockedRevision)).toMatchObject({
      label: "Moderating Effect (Save As Revision)…",
      enabled: false,
      disabledReason: reason,
    });
    expect(nativeCommandsFor({ kind: "menu", menu: "model" }, blockedRevision)
      .find((command) => command.id === "add-moderating-effect")?.disabledReason).toBe(reason);
    expect(nativeContextMenuCommands(blockedRevision)
      .find((command) => command.id === "add-moderating-effect")?.disabledReason).toBe(reason);
    expect(nativeCommandForShortcut({ key: "m" }, blockedRevision)).toBeNull();
    const dispatch = vi.fn();
    expect(executeNativeCommand("add-moderating-effect", blockedRevision, dispatch)).toBe(false);
    expect(dispatch).not.toHaveBeenCalled();
  });

  it("does not let revision authority bypass graph, calculation, or legacy read-only locks", () => {
    const revision = context({
      surface: "model",
      projectOpen: true,
      projectWritable: false,
      hasDataset: true,
      canAddModeration: true,
      moderationMutationAuthority: { kind: "general_sem_revision", available: true },
      selection: { kind: "path", count: 1 },
    });
    expect(resolveNativeCommand("add-moderating-effect", { ...revision, canAddModeration: false })).toMatchObject({
      enabled: false,
      disabledReason: "Select an eligible directed structural path with an available measured moderator.",
    });
    expect(resolveNativeCommand("add-moderating-effect", { ...revision, calculationStatus: "running" })).toMatchObject({
      enabled: false,
      disabledReason: "Finish or cancel the active calculation before changing the model.",
    });

    const legacyReadOnly = {
      ...revision,
      moderationMutationAuthority: {
        kind: "blocked" as const,
        disabledReason: "This project does not permit direct model mutations.",
      },
    };
    expect(resolveNativeCommand("add-moderating-effect", legacyReadOnly)).toMatchObject({
      label: "Moderating Effect…",
      enabled: false,
      disabledReason: "This project does not permit direct model mutations.",
    });
    expect(nativeCommandForShortcut({ key: "m" }, legacyReadOnly)).toBeNull();

    const standardWritable = { ...revision, projectWritable: true, moderationMutationAuthority: { kind: "direct" as const } };
    expect(resolveNativeCommand("add-moderating-effect", standardWritable)).toMatchObject({
      label: "Moderating Effect…",
      enabled: true,
    });
    expect(nativeCommandForShortcut({ key: "m" }, standardWritable)?.id).toBe("add-moderating-effect");
  });

  it("opens calculation setup for a blocked model so the dialog can explain the blocker", () => {
    const blocked = context({ surface: "model", projectOpen: true, hasDataset: true, canCalculate: false });
    expect(resolveNativeCommand("open-calculation", blocked).enabled).toBe(true);
  });


  it.each<RunMonitorStatus>(["queued", "validating", "running"])(
    "locks project/model mutation and enables cancellation while %s",
    (calculationStatus) => {
      const running = context({
        surface: "model",
        projectOpen: true,
        hasDataset: true,
        canCalculate: true,
        canUndo: true,
        calculationStatus,
      });
      expect(resolveNativeCommand("new-project", running).enabled).toBe(false);
      expect(resolveNativeCommand("save-project", running).enabled).toBe(false);
      expect(resolveNativeCommand("undo", running).enabled).toBe(false);
      expect(resolveNativeCommand("path-tool", running).enabled).toBe(false);
      expect(resolveNativeCommand("open-calculation", running).enabled).toBe(false);
      expect(resolveNativeCommand("cancel-calculation", running)).toMatchObject({ visible: true, enabled: true });
    },
  );

  it("shows but disables cancellation after cancellation has begun", () => {
    const cancelling = context({ calculationStatus: "cancelling" });
    expect(resolveNativeCommand("cancel-calculation", cancelling)).toMatchObject({ visible: true, enabled: false });
  });

  it.each<RunMonitorStatus>(["completed", "failed", "cancelled"])(
    "unlocks retry after the terminal %s state",
    (calculationStatus) => {
      const terminal = context({
        surface: "model",
        projectOpen: true,
        hasDataset: true,
        canCalculate: true,
        calculationStatus,
      });
      expect(resolveNativeCommand("open-calculation", terminal).enabled).toBe(true);
      expect(resolveNativeCommand("cancel-calculation", terminal).visible).toBe(false);
    },
  );

  it("does not expose cancellation or fabricated run actions while idle", () => {
    const idle = context({ surface: "model", projectOpen: true, hasDataset: true, canCalculate: true });
    expect(resolveNativeCommand("cancel-calculation", idle).visible).toBe(false);
    expect(ids(nativeCommandsFor({ kind: "menu", menu: "calculate" }, idle))).toEqual([
      "open-calculation",
    ]);
  });

  it("keeps result actions disabled until a completed result exists", () => {
    const empty = context({ surface: "results", projectOpen: true, hasDataset: true, canCalculate: true });
    const emptyToolbar = nativeCommandsFor({ kind: "toolbar", surface: "results" }, empty);
    expect(ids(emptyToolbar)).toEqual(["go-model", "open-calculation", "export-results", "run-details"]);
    expect(emptyToolbar.find((command) => command.id === "export-results")?.enabled).toBe(false);
    expect(emptyToolbar.find((command) => command.id === "run-details")?.enabled).toBe(false);

    const completed = {
      ...empty,
      hasCompletedRun: true,
      selection: { kind: "result" as const, count: 1 },
    };
    expect(resolveNativeCommand("export-results", completed).enabled).toBe(true);
    expect(ids(nativeCommandsFor({ kind: "context-menu", surface: "results" }, completed))).toEqual([
      "go-model",
      "save-report",
      "export-results",
      "run-details",
    ]);
  });

  it("returns a model-free standalone result to Data without showing Edit Model", () => {
    const standalone = context({
      surface: "results",
      projectOpen: true,
      hasDataset: true,
      hasCompletedRun: true,
      canOpenContextModel: false,
      selection: { kind: "result", count: 1 },
    });
    const toolbar = nativeCommandsFor({ kind: "toolbar", surface: "results" }, standalone);
    expect(ids(toolbar)).toContain("go-result-data");
    expect(ids(toolbar)).not.toContain("go-model");
    expect(resolveNativeCommand("go-result-data", standalone)).toMatchObject({
      label: "Edit Data",
      visible: true,
      enabled: true,
      action: { id: "surface.navigate", surface: "data" },
    });
    expect(resolveNativeCommand("go-model", standalone).visible).toBe(false);
  });

  it("disables Results to Model when the result's source model is unavailable", () => {
    const missingSourceModel = context({
      surface: "results",
      projectOpen: true,
      hasDataset: true,
      hasCompletedRun: true,
      canOpenContextModel: false,
      selection: { kind: "result", count: 1 },
    });
    expect(resolveNativeCommand("go-model", missingSourceModel)).toMatchObject({ label: "Edit Model", enabled: false });
    expect(resolveNativeCommand("go-model", { ...missingSourceModel, canOpenContextModel: true })).toMatchObject({ label: "Edit Model", enabled: true });
    expect(resolveNativeCommand("go-model", { ...missingSourceModel, surface: "data" }).label).toBe("Model");
  });

  it("resolves shortcuts from the same visibility and enablement rules", () => {
    const modelContext = context({
      surface: "model",
      projectOpen: true,
      hasDataset: true,
      canCalculate: true,
      selection: { kind: "path", count: 1 },
    });
    expect(nativeCommandForShortcut({ key: "p" }, modelContext)?.id).toBe("path-tool");
    expect(nativeCommandForShortcut({ key: "p", editable: true }, modelContext)).toBeNull();
    expect(nativeCommandForShortcut({ key: "Enter" }, modelContext)?.id).toBe("edit-selection");
    expect(nativeCommandForShortcut({ key: "Enter", editable: true }, modelContext)).toBeNull();
    expect(nativeCommandForShortcut({ key: "Delete" }, modelContext)?.id).toBe("delete-selection");
    expect(nativeCommandForShortcut({ key: "r", ctrlKey: true }, modelContext)?.id).toBe("open-calculation");
    expect(nativeCommandForShortcut({ key: "s", ctrlKey: true, editable: true }, modelContext)?.id).toBe("save-project");
    expect(formatNativeShortcut(resolveNativeCommand("save-project-as", modelContext).shortcut)).toBe("Ctrl+Shift+S");
  });

  it("dispatches only commands currently allowed by the registry", () => {
    const dispatch = vi.fn();
    const empty = context({ surface: "results", projectOpen: true });
    expect(executeNativeCommand("export-results", empty, dispatch)).toBe(false);
    expect(dispatch).not.toHaveBeenCalled();

    const completed = { ...empty, hasCompletedRun: true };
    expect(executeNativeCommand("export-results", completed, dispatch)).toBe(true);
    expect(dispatch).toHaveBeenCalledWith({ id: "results.export" });
  });

  it("resolves adaptive labels without leaking UI state into renderers", () => {
    expect(resolveNativeCommand("toggle-properties", context({ surface: "data", propertiesOpen: true })).label).toBe("Hide Properties");
    expect(resolveNativeCommand("toggle-properties", context({ surface: "data", propertiesOpen: false })).label).toBe("Show Properties");
    expect(resolveNativeCommand("edit-selection", context({ surface: "model", selection: { kind: "path", count: 1 } })).label).toBe("Edit Path Properties…");
  });
});
