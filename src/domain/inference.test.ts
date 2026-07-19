import { describe, expect, it } from "vitest";
import { findBootstrapParameter, formatParameterIdentity, parseParameterIdentity } from "./inference";
import type { PlsBootstrapRun } from "../types";

describe("inference parameter identities", () => {
  it("parses and formats typed bootstrap parameter identities", () => {
    const identity = "[\"indirect_effect\",[\"x\",\"y\"]]";
    expect(parseParameterIdentity(identity)).toEqual({ kind: "indirect_effect", parts: ["x", "y"] });
    expect(formatParameterIdentity(identity)).toBe("indirect effect | x -> y");
  });

  it("finds mediation indirect-effect bootstrap rows without matching direct or total effects", () => {
    const bootstrap: PlsBootstrapRun = {
      method_version: "indexed_resampling_v4",
      plan: { replicates: 99, master_seed: 7, operation: "pls_pm_bootstrap_v1" },
      usable_replicates: 99,
      failed_replicates: [],
      percentile: {
        confidence_level: 0.95,
        parameters: [
          row("[\"direct_effect\",[\"x\",\"y\"]]", 0.1),
          row("[\"indirect_effect\",[\"x\",\"y\"]]", 0.2),
          row("[\"total_effect\",[\"x\",\"y\"]]", 0.3),
        ],
      },
    };
    expect(findBootstrapParameter(bootstrap, "indirect_effect", ["x", "y"])?.original).toBe(0.2);
    expect(findBootstrapParameter(bootstrap, "indirect_effect", ["x", "z"])).toBeUndefined();
  });
});

function row(parameter: string, original: number) {
  return {
    parameter,
    original,
    bootstrap_mean: original,
    bias: 0,
    standard_error: 0.01,
    lower: original - 0.02,
    upper: original + 0.02,
    usable_replicates: 99,
  };
}
