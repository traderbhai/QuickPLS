import { describe, expect, it } from "vitest";
import type { AnalysisUiSettings } from "../types";
import {
  nativeCalculationMethodName,
  nativeCalculationModeForSettings,
  nativeCalculationSettingsForMode,
  nativeCalculationStartLabel,
} from "./nativeCalculationMode";

const settings: AnalysisUiSettings = {
  method: "pls_pm",
  bootstrapSamples: 0,
  studentizedInnerSamples: 99,
  permutationSamples: 0,
  seed: 7,
  workers: 1,
  confidenceLevel: 0.95,
};

describe("native calculation modes", () => {
  it("keeps every primary calculation mode mutually exclusive", () => {
    expect(nativeCalculationSettingsForMode(settings, "pls")).toMatchObject({
      method: "pls_pm",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
    });
    expect(nativeCalculationSettingsForMode(settings, "bootstrap")).toMatchObject({
      bootstrapSamples: 10_000,
      studentizedInnerSamples: 99,
      permutationSamples: 0,
    });
    expect(nativeCalculationSettingsForMode(settings, "permutation")).toMatchObject({
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 999,
    });
    expect(nativeCalculationSettingsForMode({ ...settings, groupMethods: "pls_pos,fimix" }, "predict")).toMatchObject({
      method: "predict",
      groupMethods: null,
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      confidenceLevel: 0.95,
    });
    expect(nativeCalculationSettingsForMode({ ...settings, workers: 8 }, "pls").workers).toBe(1);
    expect(nativeCalculationSettingsForMode({ ...settings, workers: 8 }, "bootstrap").workers).toBe(8);
    expect(nativeCalculationSettingsForMode({ ...settings, workers: 8 }, "permutation").workers).toBe(8);
  });

  it("preserves valid integer sample counts and normalizes unsafe draft values", () => {
    expect(nativeCalculationSettingsForMode({ ...settings, bootstrapSamples: 999 }, "bootstrap").bootstrapSamples).toBe(999);
    expect(nativeCalculationSettingsForMode({ ...settings, permutationSamples: 4_321 }, "permutation").permutationSamples).toBe(4_321);

    expect(nativeCalculationSettingsForMode({ ...settings, bootstrapSamples: 999.9 }, "bootstrap").bootstrapSamples).toBe(999);
    expect(nativeCalculationSettingsForMode({ ...settings, bootstrapSamples: 10_001 }, "bootstrap").bootstrapSamples).toBe(10_000);
    expect(nativeCalculationSettingsForMode({ ...settings, permutationSamples: 98 }, "permutation").permutationSamples).toBe(99);
    expect(nativeCalculationSettingsForMode({ ...settings, permutationSamples: 321.9 }, "permutation").permutationSamples).toBe(321);
  });

  it("infers the active mode from stored result-producing settings", () => {
    expect(nativeCalculationModeForSettings(settings)).toBe("pls");
    expect(nativeCalculationModeForSettings({ ...settings, bootstrapSamples: 5_000 })).toBe("bootstrap");
    expect(nativeCalculationModeForSettings({ ...settings, bootstrapSamples: 5_000, permutationSamples: 999 })).toBe("permutation");
    expect(nativeCalculationModeForSettings({ ...settings, method: "predict", bootstrapSamples: 5_000, permutationSamples: 999 })).toBe("predict");
  });

  it("uses precise native labels for run history and retry actions", () => {
    expect(nativeCalculationMethodName("permutation")).toBe("Structural Path Randomization");
    expect(nativeCalculationStartLabel("permutation", false)).toBe("Start path randomization");
    expect(nativeCalculationStartLabel("bootstrap", true)).toBe("Retry bootstrapping");
    expect(nativeCalculationMethodName("predict")).toBe("PLSpredict / CVPAT");
    expect(nativeCalculationStartLabel("predict", false)).toBe("Start prediction");
  });
});
