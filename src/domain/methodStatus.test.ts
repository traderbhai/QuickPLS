import { describe, expect, it } from "vitest";
import { methods } from "../data/sample";
import { effectiveMethodStatus, isSelectableAnalysisMethod, methodStatusDescription, methodStatusLabel } from "./methodStatus";

const method = (id: string) => {
  const value = methods.find((candidate) => candidate.id === id);
  if (!value) throw new Error(`Missing method fixture: ${id}`);
  return value;
};

describe("methodStatus", () => {
  it("exposes calibrated prospective PLS sample-size and power as supported", () => {
    const power = method("pls_sample_size_power");
    expect(isSelectableAnalysisMethod(power)).toBe(true);
    expect(effectiveMethodStatus(power)).toBe("validated");
    expect(methodStatusDescription(power)).toContain("prospective Monte Carlo power v2");
    expect(methodStatusDescription(power)).toContain("null-centered two-sided case-bootstrap plus-one test");
    expect(methodStatusDescription(power)).toContain("not retrospective observed power");
  });

  it("keeps CB-SEM Supported and directs bootstrap work to the exact workspace", () => {
    const cbsem = method("cbsem");
    expect(effectiveMethodStatus(cbsem, { method: "cbsem", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, seed: 1, workers: 1, confidenceLevel: 0.95, cbsemBootstrapSamples: 0 })).toBe("validated");
    expect(methodStatusDescription(cbsem, { method: "cbsem", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, seed: 1, workers: 1, confidenceLevel: 0.95, cbsemBootstrapSamples: 0 })).toContain("point-only");

    const candidateSettings = { method: "cbsem" as const, bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, seed: 1, workers: 1, confidenceLevel: 0.95, cbsemBootstrapSamples: 1_000 };
    expect(effectiveMethodStatus(cbsem, candidateSettings)).toBe("validated");
    expect(methodStatusDescription(cbsem, candidateSettings)).toContain("Exact CB-SEM workspace");
    expect(methodStatusDescription(cbsem, candidateSettings)).toContain("historical bootstrap identities remain read-only");
  });

  it("keeps exact MICOM v3.1 Experimental and separates archived combined selections", () => {
    const micom = method("mga");
    const exactSettings = {
      method: "mga" as const,
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      seed: 1,
      workers: 1,
      confidenceLevel: 0.95,
      groupMethods: "micom",
    };
    expect(effectiveMethodStatus(micom, exactSettings)).toBe("experimental");
    expect(methodStatusDescription(micom, exactSettings)).toContain("calculates only Steps 2 and 3");
    expect(methodStatusDescription(micom, exactSettings)).toContain("permutation MGA is a separate workflow");

    const archivedCombined = { ...exactSettings, groupMethods: "micom,mga_permutation" };
    expect(effectiveMethodStatus(micom, archivedCombined)).toBe("experimental");
    expect(methodStatusDescription(micom, archivedCombined)).toContain("combined or non-MICOM group selection is not available");
  });

  it("routes graph-defined PROCESS v2 through Supported product status", () => {
    const regression = method("regression");
    const settings = {
      method: "regression" as const,
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      seed: 1,
      workers: 1,
      confidenceLevel: 0.95,
      regressionType: "process" as const,
    };
    expect(effectiveMethodStatus(regression, settings)).toBe("validated");
    expect(methodStatusDescription(regression, settings)).toContain("Supports the graph-defined continuous-outcome PROCESS v2 workflow");
  });

  it("uses customer availability labels rather than internal evidence states", () => {
    expect(methodStatusLabel("validated")).toBe("Supported");
    expect(methodStatusLabel("experimental")).toBe("Experimental");
    expect(methodStatusLabel("unsupported")).toBe("Not available");
  });
});
