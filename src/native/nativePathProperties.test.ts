import { describe, expect, it } from "vitest";
import type { Edge } from "@xyflow/react";
import {
  nativePathDisplayLabel,
  nativePathLabelPatch,
  nativePathRolePatch,
} from "./nativePathProperties";

describe("native path properties", () => {
  it("stores authored control labels in the recipe-backed control metadata", () => {
    const edge: Edge = { id: "age-y", source: "age", target: "y", label: "Control", data: { role: "control" } };
    const patch = nativePathLabelPatch(edge, "control", "Age covariate");
    expect(patch).toEqual({ label: "Age covariate", data: { role: "control", controlLabel: "Age covariate" } });
    expect(nativePathDisplayLabel({ ...edge, ...patch }, "control")).toBe("Age covariate");
  });

  it("preserves custom labels and replaces only generated role labels", () => {
    const path: Edge = { id: "x-y", source: "x", target: "y", label: "Path" };
    expect(nativePathRolePatch(path, "control")).toEqual({ label: "Control", data: { role: "control", controlLabel: null } });
    expect(nativePathRolePatch({ ...path, label: "Hypothesis 1" }, "control")).toEqual({ label: "Hypothesis 1", data: { role: "control", controlLabel: "Hypothesis 1" } });
    expect(nativePathRolePatch({ ...path, label: "Control", data: { role: "control", controlLabel: null } }, "structural")).toEqual({ label: "Path", data: {} });
  });
});
