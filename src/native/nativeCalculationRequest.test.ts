import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings } from "../types";
import { createNativeCalculationRequest, parseNativeCalculationRequest } from "./nativeCalculationRequest";

const settings: AnalysisUiSettings = {
  method: "plsc",
  bootstrapSamples: 0,
  studentizedInnerSamples: 0,
  permutationSamples: 0,
  seed: 20_260_718,
  workers: 1,
  confidenceLevel: 0.95,
};

describe("native calculation request", () => {
  it("copies the submitted settings snapshot", () => {
    const request = createNativeCalculationRequest("plsc", settings);
    settings.method = "wpls";
    expect(request).toEqual({ kind: "plsc", settings: expect.objectContaining({ method: "plsc" }) });
  });

  it("accepts only workbench-owned method requests", () => {
    expect(parseNativeCalculationRequest({ kind: "wpls", settings: { ...settings, method: "wpls" } }))
      .toEqual({ kind: "wpls", settings: expect.objectContaining({ method: "wpls" }) });
    expect(parseNativeCalculationRequest({ kind: "cca", settings: { ...settings, method: "cca" } }))
      .toEqual({ kind: "cca", settings: expect.objectContaining({ method: "cca" }) });
    expect(parseNativeCalculationRequest({ kind: "ipma", settings: { ...settings, method: "ipma", ipmaTargets: "target-id" } }))
      .toEqual({ kind: "ipma", settings: expect.objectContaining({ method: "ipma", ipmaTargets: "target-id" }) });
    expect(parseNativeCalculationRequest({
      kind: "mga",
      settings: {
        ...settings,
        method: "mga",
        groupColumn: "segment",
        groupAValue: "A",
        groupBValue: "B",
        groupMethods: "mga_permutation",
        groupPermutationSamples: 1_000,
      },
    })).toEqual({
      kind: "mga",
      settings: expect.objectContaining({
        method: "mga",
        groupColumn: "segment",
        groupAValue: "A",
        groupBValue: "B",
      }),
    });
    expect(parseNativeCalculationRequest({ kind: "cta_pls", settings: { ...settings, method: "cta_pls" } })).toBeNull();
    expect(parseNativeCalculationRequest({ kind: "cca", settings: { ...settings, method: "plsc" } })).toBeNull();
    expect(parseNativeCalculationRequest({ kind: "pls_bootstrap", settings: { ...settings, method: "bootstrap" } })).toBeNull();
    expect(parseNativeCalculationRequest({ kind: "pls_bootstrap", settings: { ...settings, method: "pls_pm" } })).toEqual({
      kind: "pls_bootstrap",
      settings: expect.objectContaining({ method: "pls_pm" }),
    });
    expect(parseNativeCalculationRequest({ kind: "plsc" })).toBeNull();
    expect(parseNativeCalculationRequest(null)).toBeNull();
  });
});
