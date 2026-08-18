import { describe, expect, it } from "vitest";
import {
  acknowledgeExperimentalWarning,
  capabilityAvailabilityV2,
  capabilityCellSessionKey,
  EXPERIMENTAL_LABS_WARNING,
  shouldShowExperimentalWarning,
  type CapabilitySurfaceCellV2,
  validateMethodDetailsV2,
} from "./capabilitySurfaceV2";

const fullRelease: CapabilitySurfaceCellV2 = {
  capability_id: "qpls3.pls.algorithm",
  cell_id: "reflective_recursive",
  capability_version: "pls_algorithm_v2",
  coverage_state: "full",
  evidence_state: "release_qualified",
  surface: "standard",
};

describe("Capability Registry V2 product-surface policy", () => {
  it("shows explicitly assigned full or documented-scope Standard cells only with release evidence", () => {
    expect(capabilityAvailabilityV2(fullRelease, false)).toEqual({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
      reason: "standard_ready",
    });
    expect(capabilityAvailabilityV2({ ...fullRelease, coverage_state: "partial" }, true)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityAvailabilityV2({ ...fullRelease, evidence_state: "native_qualified" }, true).reason).toBe("incomplete_standard_cell");
  });

  it("does not infer Standard assignment from partial release evidence", () => {
    const partialRelease = { ...fullRelease, surface: "labs" as const, coverage_state: "partial" as const };
    expect(capabilityAvailabilityV2(partialRelease, false).visibility).toBe("hidden");
    expect(capabilityAvailabilityV2(partialRelease, true).visibility).toBe("experimental");
  });

  it("shows executable Labs cells only after the preference is enabled", () => {
    const labs = { ...fullRelease, surface: "labs" as const, coverage_state: "partial" as const, evidence_state: "engine_only" as const };
    expect(capabilityAvailabilityV2(labs, false).reason).toBe("labs_disabled");
    expect(capabilityAvailabilityV2(labs, true)).toMatchObject({ visibility: "experimental", selectable: true, customer_label: "Experimental" });
    expect(capabilityAvailabilityV2({ ...labs, evidence_state: "absent" }, true).reason).toBe("not_executable");
    expect(capabilityAvailabilityV2({ ...labs, coverage_state: "absent" }, true).reason).toBe("not_executable");
  });

  it("never exposes Legacy, Internal, or intentionally excluded cells in Calculate", () => {
    expect(capabilityAvailabilityV2({ ...fullRelease, surface: "legacy" }, true).reason).toBe("legacy_only");
    expect(capabilityAvailabilityV2({ ...fullRelease, surface: "internal" }, true).reason).toBe("internal_only");
    expect(capabilityAvailabilityV2({ ...fullRelease, coverage_state: "intentionally_excluded" }, true).reason).toBe("intentionally_excluded");
  });

  it("shows the Experimental warning once per capability cell and session", () => {
    const labs = { ...fullRelease, surface: "labs" as const, coverage_state: "partial" as const };
    const empty = new Set<string>();
    expect(shouldShowExperimentalWarning(labs, true, empty)).toBe(true);
    const acknowledged = acknowledgeExperimentalWarning(labs, empty);
    expect(shouldShowExperimentalWarning(labs, true, acknowledged)).toBe(false);
    expect(empty.size).toBe(0);
    expect(acknowledged).toContain(capabilityCellSessionKey(labs));
    expect(EXPERIMENTAL_LABS_WARNING).toBe("Experimental methods may change and should be independently checked before final reporting.");
  });

  it("requires all nine Method Details sections", () => {
    const valid = {
      what_it_answers: "Whether the structural paths are nonzero.",
      when_to_use: "For a supported recursive PLS model.",
      required_model_and_data: "Raw numeric observations and a supported model.",
      settings_and_defaults: "Path weighting with documented defaults.",
      outputs: "Paths, loadings, weights, scores, and quality criteria.",
      assumptions_and_cautions: "Interpret estimates under the stated model assumptions.",
      interpretation_guidance: "Use estimates with uncertainty where inference is requested.",
      method_references: ["Primary methodological reference"],
      advanced_technical_details: "Estimator and convergence definitions.",
    };
    expect(validateMethodDetailsV2(valid)).toEqual([]);
    expect(validateMethodDetailsV2({ ...valid, outputs: "", method_references: [] })).toEqual([
      "Outputs must be nonempty",
      "Method references must contain at least one reference",
    ]);
  });
});
