import { describe, expect, it } from "vitest";
import type { NativeCanonicalModelSpec, NativeSavedReport } from "../types";
import {
  buildNativeWorkspaceTree,
  nativeWorkspaceModelTreeId,
  nativeWorkspaceReportTreeId,
  nativeWorkspaceSelectionForNode,
  nativeWorkspaceTreeIdForSelection,
  nativeWorkspaceTreeNavigation,
  nextNativeWorkspaceModelName,
} from "./nativeWorkspaceTree";

const models: Array<Pick<NativeCanonicalModelSpec, "id" | "name">> = [
  { id: "model-b", name: "Retention drivers" },
  { id: "model-a", name: "Customer loyalty" },
];
const reports: NativeSavedReport[] = [
  { resultId: "run-b", name: "Structural model", savedAt: "2026-08-11T00:00:00.000Z" },
  { resultId: "run-a", name: "PLS Algorithm", savedAt: "2026-08-11T00:00:00.000Z" },
];

function tree(expandedIds: readonly string[] = ["project", "models", "reports"]) {
  return buildNativeWorkspaceTree({
    projectName: "Corporate reputation",
    datasetName: "corporate-reputation.csv",
    models,
    reports,
    expandedIds: new Set(expandedIds),
  });
}

describe("native workspace explorer tree", () => {
  it("builds a deterministic alphabetical Project > Data / Models / Reports hierarchy", () => {
    expect(tree().map(({ id, level }) => [id, level])).toEqual([
      ["project", 1],
      ["data", 2],
      ["models", 2],
      [nativeWorkspaceModelTreeId("model-a"), 3],
      [nativeWorkspaceModelTreeId("model-b"), 3],
      ["reports", 2],
      [nativeWorkspaceReportTreeId("run-a"), 3],
      [nativeWorkspaceReportTreeId("run-b"), 3],
    ]);
  });

  it("uses the stable item id when normalized labels compare equally", () => {
    const rows = buildNativeWorkspaceTree({
      projectName: "Project",
      datasetName: "Data",
      models: [{ id: "z", name: "alpha" }, { id: "a", name: "Alpha" }],
      reports: [],
      expandedIds: new Set(["project", "models"]),
    });
    expect(rows.filter((node) => node.kind === "model").map((node) => node.modelId)).toEqual(["a", "z"]);
  });

  it("omits descendants of collapsed branches", () => {
    expect(tree(["project"]).map((node) => node.id)).toEqual(["project", "data", "models", "reports"]);
    expect(tree([]).map((node) => node.id)).toEqual(["project"]);
  });

  it("maps shared explorer selections to stable tree ids and back", () => {
    expect(nativeWorkspaceTreeIdForSelection({ kind: "project" })).toBe("project");
    expect(nativeWorkspaceTreeIdForSelection({ kind: "model", modelId: "model-b" })).toBe("model:model-b");
    expect(nativeWorkspaceTreeIdForSelection({ kind: "report", resultId: "run-a" })).toBe("report:run-a");
    expect(nativeWorkspaceSelectionForNode(tree()[3])).toEqual({ kind: "model", modelId: "model-a" });
    expect(nativeWorkspaceSelectionForNode(tree()[0])).toEqual({ kind: "project" });
  });

  it("generates a normalized non-conflicting default model name", () => {
    expect(nextNativeWorkspaceModelName([
      { name: "model 1" },
      { name: "\uFF2D\uFF4F\uFF44\uFF45\uFF4C 2" },
    ])).toBe("Model 3");
  });
});

describe("native workspace explorer keyboard navigation", () => {
  it("moves through visible rows with arrows and boundary keys", () => {
    const rows = tree();
    expect(nativeWorkspaceTreeNavigation(rows, "data", "ArrowDown")?.focusId).toBe("models");
    expect(nativeWorkspaceTreeNavigation(rows, "data", "ArrowUp")?.focusId).toBe("project");
    expect(nativeWorkspaceTreeNavigation(rows, "data", "Home")?.focusId).toBe("project");
    expect(nativeWorkspaceTreeNavigation(rows, "data", "End")?.focusId).toBe("report:run-b");
    expect(nativeWorkspaceTreeNavigation(rows, "project", "ArrowUp")?.focusId).toBe("project");
  });

  it("expands, enters, collapses, and returns to a parent using Windows tree semantics", () => {
    const collapsed = tree(["project", "reports"]);
    expect(nativeWorkspaceTreeNavigation(collapsed, "models", "ArrowRight")).toEqual({
      focusId: "models",
      expansion: { id: "models", expanded: true },
    });

    const expanded = tree();
    expect(nativeWorkspaceTreeNavigation(expanded, "models", "ArrowRight")?.focusId).toBe("model:model-a");
    expect(nativeWorkspaceTreeNavigation(expanded, "model:model-a", "ArrowLeft")?.focusId).toBe("models");
    expect(nativeWorkspaceTreeNavigation(expanded, "models", "ArrowLeft")).toEqual({
      focusId: "models",
      expansion: { id: "models", expanded: false },
    });
  });
});
