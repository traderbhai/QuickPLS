import { describe, expect, it } from "vitest";
import rawCapabilityRegistryV2 from "../../validation/capabilities/capability_registry_v2.json";
import type { AnalysisMethodId, AnalysisUiSettings } from "../types";
import { CapabilityRegistryV2Adapter, capabilityRegistryV2 } from "./capabilityRegistryV2";
import {
  ESTABLISHED_METHOD_CONTRACTS_V1,
  establishedMethodContractV1,
} from "./generated/establishedMethodContractsV1";
import {
  methodCapabilityAvailabilityV2,
  methodCapabilityRequirementsV2,
  type MethodCapabilityRequirementV2,
} from "./methodCapabilityRegistryV2";

const ALL_METHODS: Record<AnalysisMethodId, true> = {
  pls_pm: true,
  bootstrap: true,
  permutation: true,
  pls_sample_size_power: true,
  plsc: true,
  wpls: true,
  cca: true,
  cta_pls: true,
  endogeneity: true,
  nonlinear_effects: true,
  moderated_mediation: true,
  predict: true,
  mga: true,
  ipma: true,
  cbsem: true,
  pca: true,
  gsca: true,
  regression: true,
  nca: true,
};

const methodIds = Object.keys(ALL_METHODS) as AnalysisMethodId[];

const PRIOR_ESTABLISHED_METHOD_TUPLES = {
  cca: [
    ["smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_algorithm", "base"],
    ["smartpls.cca", "qpls3.assessment.cca_residuals", "cca", "primary"],
  ],
  gsca: [
    ["smartpls.gsca", "qpls3.gsca.als", "gsca", "primary"],
  ],
  ipma: [
    ["smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_algorithm", "base"],
    ["smartpls.ipma", "qpls3.assessment.ipma", "ipma", "primary"],
  ],
  nca: [
    ["smartpls.nca", "qpls3.standalone.nca", "nca", "primary"],
  ],
} as const satisfies Record<
  "cca" | "gsca" | "ipma" | "nca",
  readonly (readonly [string, string, string, "base" | "primary"])[]
>;

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

function identities(requirements: readonly MethodCapabilityRequirementV2[]): string[] {
  return requirements.map((item) => `${item.capability_id}::${item.cell_id}`);
}

function revisedRegistryWithStandardRows(...capabilityIds: string[]): CapabilityRegistryV2Adapter {
  const revised = JSON.parse(JSON.stringify(rawCapabilityRegistryV2)) as Record<string, any>;
  for (const capabilityId of capabilityIds) {
    const row = revised.capabilities.find((candidate: Record<string, unknown>) => candidate.capability_id === capabilityId);
    if (!row) throw new Error(`Missing registry test row ${capabilityId}`);
    row.coverage_state = "full";
    row.evidence_state = "release_qualified";
    row.surface = "standard";
    for (const cell of row.option_cells) {
      cell.coverage_state = "full";
      cell.evidence_state = "release_qualified";
      cell.surface = "standard";
    }
  }
  const coverage = { full: 0, partial: 0, absent: 0, intentionally_excluded: 0 };
  const surfaces = { standard: 0, labs: 0, legacy: 0, internal: 0 };
  for (const row of revised.capabilities) {
    coverage[row.coverage_state as keyof typeof coverage] += 1;
    surfaces[row.surface as keyof typeof surfaces] += 1;
  }
  revised.state_contract.baseline_counts = coverage;
  revised.surface_contract.baseline_counts = surfaces;
  return new CapabilityRegistryV2Adapter(revised, { requireFrozenStateDistribution: false });
}

function revisedRegistryWithAbsentRow(capabilityId: string): CapabilityRegistryV2Adapter {
  const revised = JSON.parse(JSON.stringify(rawCapabilityRegistryV2)) as Record<string, any>;
  const row = revised.capabilities.find((candidate: Record<string, unknown>) => candidate.capability_id === capabilityId);
  if (!row) throw new Error(`Missing registry test row ${capabilityId}`);
  row.coverage_state = "absent";
  row.evidence_state = "absent";
  row.surface = "labs";
  for (const cell of row.option_cells) {
    cell.coverage_state = "absent";
    cell.evidence_state = "absent";
    cell.surface = "labs";
  }
  const coverage = { full: 0, partial: 0, absent: 0, intentionally_excluded: 0 };
  const surfaces = { standard: 0, labs: 0, legacy: 0, internal: 0 };
  for (const candidate of revised.capabilities) {
    coverage[candidate.coverage_state as keyof typeof coverage] += 1;
    surfaces[candidate.surface as keyof typeof surfaces] += 1;
  }
  revised.state_contract.baseline_counts = coverage;
  revised.surface_contract.baseline_counts = surfaces;
  return new CapabilityRegistryV2Adapter(revised, { requireFrozenStateDistribution: false });
}

describe("Analysis method to Capability Registry V2 bridge", () => {
  it("adopts generated established-method tuples without changing prior order or base roles", () => {
    for (const contract of ESTABLISHED_METHOD_CONTRACTS_V1) {
      const method = contract.analysis_method as keyof typeof PRIOR_ESTABLISHED_METHOD_TUPLES;
      const prior = PRIOR_ESTABLISHED_METHOD_TUPLES[method];
      expect(contract.method_config_kind).toBe(method);
      expect(contract.capability_requirements.map((item) => [
        item.capability_id,
        item.cell_id,
        item.option,
        item.role,
      ])).toEqual(prior);
      const requirements = methodCapabilityRequirementsV2(settings(method));
      expect(requirements).toEqual(prior.map(([
        capability_id,
        cell_id,
        option,
      ]) => ({ capability_id, cell_id, option })));
      expect(Object.isFrozen(requirements)).toBe(true);
      expect(requirements.every(Object.isFrozen)).toBe(true);
    }
  });

  it("falls through the generated lookup for dynamic and unknown methods", () => {
    expect(establishedMethodContractV1("plsc", "plsc")).toBeNull();
    expect(methodCapabilityRequirementsV2(settings("plsc", {
      bootstrapSamples: 1_000,
      permutationSamples: 1_000,
    }))).toEqual([
      { capability_id: "smartpls.plsc", cell_id: "qpls3.pls.consistent", option: "consistent_pls" },
      {
        capability_id: "smartpls.consistent_bootstrapping",
        cell_id: "qpls3.inference.consistent_bootstrap",
        option: "consistent_bootstrap",
      },
      {
        capability_id: "smartpls.consistent_permutation",
        cell_id: "qpls3.inference.consistent_permutation",
        option: "consistent_permutation",
      },
    ]);

    const unknown = { ...settings("pls_pm"), method: "future_sem" } as unknown as AnalysisUiSettings;
    expect(establishedMethodContractV1("future_sem", "future_sem")).toBeNull();
    expect(() => methodCapabilityRequirementsV2(unknown)).toThrowError("Unknown analysis-method mapping: future_sem");
  });

  it("maps every current AnalysisMethodId to exact existing registry identities", () => {
    expect(methodIds).toHaveLength(19);
    for (const method of methodIds) {
      const requirements = methodCapabilityRequirementsV2(settings(method));
      expect(requirements.length, method).toBeGreaterThan(0);
      for (const required of requirements) {
        expect(
          capabilityRegistryV2.quickPlsCell(required.cell_id).some((match) => (
            match.row.capability_id === required.capability_id
            && match.cell.capability_id === required.capability_id
            && match.cell.cell_id === required.cell_id
            && match.link.capability_id === required.capability_id
          )),
          `${method}: ${required.capability_id}::${required.cell_id}`,
        ).toBe(true);
      }
    }
  });

  it("selects OLS, logistic, bootstrap, and PROCESS cells from regression settings", () => {
    expect(identities(methodCapabilityRequirementsV2(settings("regression")))).toEqual([
      "smartpls.regression::qpls3.standalone.ols",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("regression", { regressionType: "logistic" })))).toEqual([
      "smartpls.logistic_regression::qpls3.standalone.logistic",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("regression", {
      regressionType: "ols",
      regressionBootstrap: true,
    })))).toEqual([
      "smartpls.regression::qpls3.standalone.ols",
      "smartpls.regression_bootstrapping::qpls3.standalone.regression_bootstrap",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("regression", {
      regressionType: "process",
      regressionBootstrap: true,
    })))).toEqual([
      "smartpls.process::qpls3.standalone.process",
      "smartpls.process_bootstrapping::qpls3.standalone.process",
    ]);
  });

  it("selects CB-SEM ML, CFA, optional bootstrap, multigroup, and invariance independently", () => {
    expect(identities(methodCapabilityRequirementsV2(settings("cbsem")))).toEqual([
      "smartpls.cbsem::qpls3.cbsem.ml",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("cbsem", { cbsemModelType: "cfa" })))).toEqual([
      "smartpls.cfa::qpls3.cbsem.ml",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("cbsem", {
      cbsemBootstrapSamples: 1_000,
    })))).toEqual([
      "smartpls.cbsem::qpls3.cbsem.ml",
      "smartpls.cbsem_bootstrapping::qpls3.cbsem.bootstrap",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("cbsem", {
      cbsemGroupColumn: "group",
    })))).toEqual([
      "smartpls.cbsem::qpls3.cbsem.ml",
      "smartpls.cbsem_mga::qpls3.cbsem.multigroup",
      "smartpls.cbsem_measurement_invariance::qpls3.cbsem.measurement_invariance",
    ]);
  });

  it("maps each requested PLS group and segmentation option without collapsing shared cells", () => {
    expect(identities(methodCapabilityRequirementsV2(settings("mga", {
      groupMethods: "micom,mga_permutation,pls_pos,fimix",
    })))).toEqual([
      "smartpls.pls_algorithm::qpls3.pls.algorithm",
      "smartpls.micom::qpls3.groups.micom_permutation_mga",
      "smartpls.mga::qpls3.groups.micom_permutation_mga",
      "smartpls.pls_pos::qpls3.segmentation.pls_pos",
      "smartpls.fimix_pls::qpls3.segmentation.fimix_pls",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("mga")))).toEqual([
      "smartpls.pls_algorithm::qpls3.pls.algorithm",
      "smartpls.micom::qpls3.groups.micom_permutation_mga",
      "smartpls.mga::qpls3.groups.micom_permutation_mga",
    ]);
  });

  it("requires both PLSpredict and CVPAT rows and adds requested prediction segmentation", () => {
    expect(identities(methodCapabilityRequirementsV2(settings("predict")))).toEqual([
      "smartpls.pls_algorithm::qpls3.pls.algorithm",
      "smartpls.plspredict::qpls3.prediction.plspredict_cvpat",
      "smartpls.cvpat::qpls3.prediction.plspredict_cvpat",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("predict", { groupMethods: "pls_pos,fimix" })))).toEqual([
      "smartpls.pls_algorithm::qpls3.pls.algorithm",
      "smartpls.plspredict::qpls3.prediction.plspredict_cvpat",
      "smartpls.cvpat::qpls3.prediction.plspredict_cvpat",
      "smartpls.pls_pos::qpls3.segmentation.pls_pos",
      "smartpls.fimix_pls::qpls3.segmentation.fimix_pls",
    ]);
  });

  it("routes prospective power through its exact scoped Standard v2 cell", () => {
    expect(methodCapabilityRequirementsV2(settings("pls_sample_size_power"))).toMatchObject([
      {
        capability_id: "smartpls.pls_power_analysis",
        cell_id: "qpls3.pls.sample_size_power",
        option: "prospective_sample_size_power",
      },
    ]);
    expect(methodCapabilityAvailabilityV2(settings("pls_sample_size_power"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      blocked_cell_ids: [],
      internal_reason: "all_required_cells_standard",
    });
  });

  it("requires the exact post-hoc add-on plus its Standard base cells", () => {
    const opted = settings("pls_pm", {
      bootstrapSamples: 5_000,
      posthocTechnicalMinimumSampleSize: true,
    });
    expect(identities(methodCapabilityRequirementsV2(opted))).toEqual([
      "smartpls.pls_algorithm::qpls3.pls.algorithm",
      "smartpls.pls_bootstrapping::qpls3.inference.bootstrap",
      "smartpls.pls_power_analysis::qpls3.pls.posthoc_technical_minimum_sample_size",
    ]);
    expect(methodCapabilityAvailabilityV2(opted, {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(opted, {
      experimentalLabsEnabled: true,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(identities(methodCapabilityRequirementsV2(settings("pls_pm", {
      bootstrapSamples: 5_000,
    })))).toEqual([
      "smartpls.pls_algorithm::qpls3.pls.algorithm",
    ]);
  });

  it("tracks PLSc point estimation and its resampling methods as independent exact option cells", () => {
    expect(identities(methodCapabilityRequirementsV2(settings("plsc")))).toEqual([
      "smartpls.plsc::qpls3.pls.consistent",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("plsc", { bootstrapSamples: 1_000 })))).toEqual([
      "smartpls.plsc::qpls3.pls.consistent",
      "smartpls.consistent_bootstrapping::qpls3.inference.consistent_bootstrap",
    ]);
    expect(identities(methodCapabilityRequirementsV2(settings("plsc", { permutationSamples: 1_000 })))).toEqual([
      "smartpls.plsc::qpls3.pls.consistent",
      "smartpls.consistent_permutation::qpls3.inference.consistent_permutation",
    ]);

    expect(methodCapabilityAvailabilityV2(settings("plsc"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({ tier: "standard", selectable: true, label: "Supported" });
    expect(methodCapabilityAvailabilityV2(settings("plsc", { bootstrapSamples: 1_000 }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("plsc", { permutationSamples: 1_000 }), {
      experimentalLabsEnabled: true,
    })).toMatchObject({
      tier: "hidden",
      selectable: false,
      blocked_cell_ids: ["qpls3.inference.consistent_permutation"],
      blocked_capability_ids: ["smartpls.consistent_permutation"],
      internal_reason: "required_cells_not_executable",
    });
  });

  it("keeps scoped PLS Standard while Labs cannot override missing add-on evidence", () => {
    expect(methodCapabilityAvailabilityV2(settings("pls_pm"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("pls_pm"), {
      experimentalLabsEnabled: true,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("wpls"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("cca"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("cta_pls"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("ipma"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
    });
    expect(methodCapabilityAvailabilityV2(settings("bootstrap", {
      bootstrapSamples: 5_000,
    }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_capability_ids: ["smartpls.pls_algorithm", "smartpls.pls_bootstrapping"],
    });
    expect(methodCapabilityAvailabilityV2(settings("permutation", {
      permutationSamples: 999,
    }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_capability_ids: ["smartpls.pls_algorithm", "smartpls.permutation"],
    });
    expect(methodCapabilityAvailabilityV2(settings("mga", {
      groupMethods: "micom,mga_permutation",
    }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_capability_ids: ["smartpls.pls_algorithm", "smartpls.micom", "smartpls.mga"],
    });
    expect(methodCapabilityAvailabilityV2(settings("predict"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_capability_ids: ["smartpls.pls_algorithm", "smartpls.plspredict", "smartpls.cvpat"],
    });

    const projections = methodIds.map((method) => methodCapabilityAvailabilityV2(settings(method), {
      experimentalLabsEnabled: true,
    }));
    expect(projections.filter((item) => item.tier === "standard")).toHaveLength(16);
    expect(methodCapabilityAvailabilityV2(settings("cbsem"), {
      experimentalLabsEnabled: false,
    })).toMatchObject({ tier: "standard", selectable: true, label: "Supported" });
    expect(methodCapabilityAvailabilityV2(settings("regression", { regressionType: "logistic" }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({ tier: "standard", selectable: true, label: "Supported" });
    expect(methodCapabilityAvailabilityV2(settings("regression", { regressionType: "process" }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({ tier: "standard", selectable: true, label: "Supported" });
    expect(methodCapabilityAvailabilityV2(settings("regression", {
      regressionType: "process",
      regressionBootstrap: true,
    }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({ tier: "standard", selectable: true, label: "Supported" });
  });

  it("keeps the exact CFA bootstrap Standard while an explicitly demoted add-on still fails closed", () => {
    expect(methodCapabilityAvailabilityV2(settings("cbsem", {
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
    }), {
      experimentalLabsEnabled: false,
    })).toMatchObject({
      tier: "standard",
      selectable: true,
      label: "Supported",
      required_capability_ids: ["smartpls.cfa", "smartpls.cbsem_bootstrapping"],
    });

    const registry = revisedRegistryWithAbsentRow("smartpls.cbsem_bootstrapping");
    expect(methodCapabilityAvailabilityV2(settings("cbsem", { cbsemModelType: "cfa" }), {
      experimentalLabsEnabled: false,
      registry,
    })).toMatchObject({ tier: "standard", selectable: true, label: "Supported" });

    expect(methodCapabilityAvailabilityV2(settings("cbsem", {
      cbsemModelType: "cfa",
      cbsemBootstrapSamples: 1_000,
    }), {
      experimentalLabsEnabled: true,
      registry,
    })).toMatchObject({
      tier: "hidden",
      selectable: false,
      blocked_cell_ids: ["qpls3.cbsem.bootstrap"],
      blocked_capability_ids: ["smartpls.cbsem_bootstrapping"],
      internal_reason: "required_cells_not_executable",
    });
  });

  it("fails closed for unknown method, option, and exact registry mapping", () => {
    const unknownMethod = { ...settings("pls_pm"), method: "future_sem" } as unknown as AnalysisUiSettings;
    expect(methodCapabilityAvailabilityV2(unknownMethod, {
      experimentalLabsEnabled: true,
    })).toMatchObject({
      method_id: "future_sem",
      tier: "hidden",
      blocked_cell_ids: [],
      internal_reason: "unknown_method_mapping",
    });

    expect(methodCapabilityAvailabilityV2(settings("mga", { groupMethods: "micom,mystery" }), {
      experimentalLabsEnabled: true,
    })).toMatchObject({ tier: "hidden", internal_reason: "unknown_option_mapping" });

    expect(methodCapabilityAvailabilityV2(settings("pls_pm"), {
      experimentalLabsEnabled: true,
      registry: {
        quickPlsCell: () => [],
        availability: () => { throw new Error("must not resolve a missing identity"); },
      },
    })).toMatchObject({
      tier: "hidden",
      blocked_cell_ids: ["qpls3.pls.algorithm"],
      internal_reason: "registry_mapping_missing",
    });
  });

  it("exposes no scientific qualification or evidence labels in the product projection", () => {
    const projection = methodCapabilityAvailabilityV2(settings("pls_pm"), {
      experimentalLabsEnabled: true,
    });
    expect(Object.isFrozen(projection)).toBe(true);
    expect(JSON.stringify(projection)).not.toMatch(
      /coverage_state|evidence_state|release_qualified|native_qualified|archive_qualified|engine_only|qualification/i,
    );
  });
});
