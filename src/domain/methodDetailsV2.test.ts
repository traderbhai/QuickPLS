import { describe, expect, it } from "vitest";
import type { AnalysisMethodId, AnalysisUiSettings } from "../types";
import { validateMethodDetailsV2 } from "./capabilitySurfaceV2";
import { methodDetailsForRequirementsV2, methodDetailsForSettingsV2 } from "./methodDetailsV2";

const METHODS: readonly AnalysisMethodId[] = [
  "pls_pm",
  "bootstrap",
  "permutation",
  "pls_sample_size_power",
  "plsc",
  "wpls",
  "cca",
  "cta_pls",
  "endogeneity",
  "nonlinear_effects",
  "moderated_mediation",
  "predict",
  "mga",
  "ipma",
  "cbsem",
  "pca",
  "gsca",
  "regression",
  "nca",
];

const NONLINEAR_OPTION_REQUIREMENT = [{
  capability_id: "smartpls.nonlinear_relationships",
  cell_id: "qpls3.pls.nonlinear_quadratic",
  option: "nonlinear_relationships",
}] as const;

function settings(method: AnalysisMethodId, overrides: Partial<AnalysisUiSettings> = {}): AnalysisUiSettings {
  return {
    method,
    bootstrapSamples: 0,
    studentizedInnerSamples: 0,
    permutationSamples: 0,
    seed: 20_260_718,
    workers: 1,
    confidenceLevel: 0.95,
    groupMethods: null,
    cbsemModelType: "sem",
    cbsemBootstrapSamples: 0,
    cbsemGroupColumn: null,
    cbsemInvarianceSteps: "configural,metric,scalar",
    regressionType: "ols",
    regressionBootstrap: false,
    ...overrides,
  };
}

describe("Method Details V2", () => {
  it("builds every required customer section from exact option cells for all live method IDs", () => {
    for (const method of METHODS) {
      const result = methodDetailsForSettingsV2(settings(method), true);
      expect(result.status, method).toBe("ready");
      expect(result.issues, method).toEqual([]);
      expect(result.items.length, method).toBeGreaterThan(0);
      for (const item of result.items) {
        expect(item.capability_cell.registry_schema_version).toBe(2);
        expect(item.capability_cell.capability_id).toBeTruthy();
        expect(item.capability_cell.cell_id).toBeTruthy();
        expect(item.capability_cell.capability_version).toBeTruthy();
        expect(validateMethodDetailsV2(item.details), item.option_name).toEqual([]);
        expect(item.details.method_references.every((reference) => reference.startsWith("https://"))).toBe(true);
      }
    }
  });

  it("explains the Standard CB-SEM point estimator and exact bootstrap add-on independently", () => {
    const result = methodDetailsForSettingsV2(settings("cbsem", { cbsemBootstrapSamples: 1_000 }), true);
    expect(result.status).toBe("ready");
    expect(result.items.map((item) => item.capability_cell.cell_id)).toEqual([
      "qpls3.cbsem.ml",
      "qpls3.cbsem.bootstrap",
    ]);
    expect(result.items[0].availability.visibility).toBe("supported");
    expect(result.items[0].availability.reason).toBe("standard_ready");
    expect(result.items[0].availability_message).toBe("Available in Standard.");
    expect(result.items[1].availability.visibility).toBe("supported");
    expect(result.items[1].availability.reason).toBe("standard_ready");
    expect(result.items[1].availability_message).toBe("Available in Standard.");
  });

  it("keeps supported use, defaults, and limitations in Method Details", () => {
    const logistic = methodDetailsForSettingsV2(settings("regression", { regressionType: "logistic" }), true);
    expect(logistic.status).toBe("ready");
    expect(logistic.items[0].details.required_model_and_data).toContain("Supported use:");
    expect(logistic.items[0].details.settings_and_defaults).toContain("exactly 0/1 numeric outcome");
    expect(logistic.items[0].details.assumptions_and_cautions).toContain("Multinomial, ordinal, weighted, clustered, penalized, and Firth-corrected variants are not included.");

    const power = methodDetailsForSettingsV2(settings("pls_sample_size_power"), true);
    expect(power.items[0].details.required_model_and_data).toContain("Prospective Monte Carlo power");
    expect(power.items[0].details.assumptions_and_cautions).toContain("not retrospective observed power");
  });

  it("uses one concise option caution without repeating the session-level Labs warning", () => {
    const result = methodDetailsForRequirementsV2(
      "Nonlinear Relationships",
      NONLINEAR_OPTION_REQUIREMENT,
      true,
    );
    expect(result.items[0].availability.visibility).toBe("experimental");
    const displayed = JSON.stringify(result.items.map((item) => ({
      method_name: item.method_name,
      option_name: item.option_name,
      availability_message: item.availability_message,
      details: item.details,
    })));
    expect(displayed).toContain("This option is Experimental. Independently check the result before final reporting.");
    expect(displayed).not.toContain("Experimental methods may change and should be independently checked before final reporting.");
    expect(displayed).not.toMatch(/validated scope|calculation scope|method scope|native-qualified|release-qualified|promotion evidence|packaged evidence|candidate\s*[/,;]\s*unqualified/i);
  });

  it("tells the user how to expose a Labs method when Labs is disabled", () => {
    const result = methodDetailsForRequirementsV2(
      "Nonlinear Relationships",
      NONLINEAR_OPTION_REQUIREMENT,
      false,
    );
    expect(result.status).toBe("ready");
    expect(result.items[0].availability.reason).toBe("labs_disabled");
    expect(result.items[0].availability_message).toBe("Turn on Experimental Labs in Preferences to use this option.");
  });

  it("fails closed when a completed run has no resolvable option-cell requirements", () => {
    const result = methodDetailsForRequirementsV2("Historical run", [], true);
    expect(result).toMatchObject({
      status: "unavailable",
      method_id: "Historical run",
      items: [],
      issues: ["Method information is unavailable for this run."],
    });
  });
});
