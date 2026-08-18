import { describe, expect, it } from "vitest";
import type { Edge } from "@xyflow/react";
import {
  nativeCovariancePresentationPatch,
  nativeCovarianceScientificPatch,
  nativePathDisplayLabel,
  nativePathLabelPatch,
  nativePathRolePatch,
} from "./nativePathProperties";
import { inspectNativeCovarianceAuthoringV4, newNativeScientificCovarianceEdgeV4 } from "../domain/semModelV4Authoring";

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

  it("marks a role-only conversion as unresolved instead of silently making it scientific", () => {
    const path: Edge = { id: "x-y", source: "x", target: "y", label: "Path" };
    const patch = nativePathRolePatch(path, "covariance");
    const converted = { ...path, ...patch };
    expect(converted).toMatchObject({
      label: "Covariance",
      data: {
        role: "covariance",
        semModelV4: {
          version: 1,
          covariance: { kind: "legacy_unspecified", origin: "role_conversion" },
        },
      },
    });
    expect(inspectNativeCovarianceAuthoringV4(converted).state).toBe("legacy_unspecified");
  });

  it("keeps an existing scientific decision when only its covariance role is reselected", () => {
    const scientific = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    const patch = nativePathRolePatch(scientific, "covariance");
    expect(patch.data).toEqual(scientific.data);
    expect(inspectNativeCovarianceAuthoringV4({ ...scientific, ...patch }).state).toBe("scientific");
  });

  it("uses explicit patches to switch between model and presentation-only covariance", () => {
    const legacy: Edge = { id: "legacy-cov", source: "x", target: "y", data: { role: "covariance" } };
    const scientific = { ...legacy, ...nativeCovarianceScientificPatch(legacy) };
    expect(inspectNativeCovarianceAuthoringV4(scientific).state).toBe("scientific");
    expect(inspectNativeCovarianceAuthoringV4(scientific)).toMatchObject({
      specification: { origin: "explicit_conversion", left: null, right: null },
    });
    const presentation = { ...scientific, ...nativeCovariancePresentationPatch(scientific) };
    expect(inspectNativeCovarianceAuthoringV4(presentation)).toMatchObject({
      state: "presentation_only",
      specification: { origin: "explicit_conversion" },
    });
  });

  it("clears covariance-only metadata when the user explicitly changes relationship type", () => {
    const covariance = newNativeScientificCovarianceEdgeV4("cov-x-y", "x", "y");
    expect(nativePathRolePatch(covariance, "structural")).toEqual({ label: "Path", data: {} });
    expect(nativePathRolePatch(covariance, "control")).toEqual({ label: "Control", data: { role: "control", controlLabel: null } });
  });
});
