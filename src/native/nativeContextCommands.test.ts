import { describe, expect, it } from "vitest";
import { nativeCommandsFor, type NativeCommandContext, type NativeSurface } from "./nativeCommands";

function context(surface: NativeSurface): NativeCommandContext {
  return {
    surface,
    projectOpen: true,
    projectWritable: true,
    hasDataset: true,
    hasCompletedRun: surface === "results",
    selectedResultSaved: false,
    canOpenContextModel: true,
    canCalculate: true,
    canUndo: false,
    canRedo: false,
    canRecode: true,
    canConfigureGroups: true,
    selectedVariableIsGrouping: false,
    canAddModeration: true,
    canAddHigherOrder: true,
    propertiesOpen: true,
    selection: surface === "data"
      ? { kind: "variable", count: 1 }
      : surface === "model"
        ? { kind: "construct", count: 1 }
        : { kind: "result", count: 1 },
    calculationStatus: "idle",
  };
}

describe("registry-derived native context commands", () => {
  it.each<[NativeSurface, string[]]>([
    ["data", ["recode-variable", "configure-groups", "go-model"]],
    ["model", ["edit-selection", "delete-selection", "arrange-model", "fit-model", "open-calculation"]],
    ["results", ["go-model", "save-report", "export-results", "run-details"]],
  ])("provides functional %s workspace commands", (surface, expected) => {
    const commands = nativeCommandsFor({ kind: "context-menu", surface }, context(surface));
    expect(commands.map((command) => command.id)).toEqual(expected);
    expect(commands.every((command) => command.enabled)).toBe(true);
  });
});
