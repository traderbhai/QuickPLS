import { describe, expect, it } from "vitest";
import rawCapabilityRegistryV2 from "../../validation/capabilities/capability_registry_v2.json";
import {
  CapabilityRegistryV2Adapter,
  capabilityRegistryV2,
  parseCapabilityRegistryV2,
} from "./capabilityRegistryV2";

function mutableRegistry(): Record<string, any> {
  return JSON.parse(JSON.stringify(rawCapabilityRegistryV2)) as Record<string, any>;
}

describe("Capability Registry V2 frontend adapter", () => {
  it("loads the frozen 45-row / 43-active baseline and declared surfaces", () => {
    expect(capabilityRegistryV2.summary).toEqual({
      row_count: 45,
      active_row_count: 43,
      coverage: { full: 0, partial: 32, absent: 11, intentionally_excluded: 2 },
      surfaces: { standard: 26, labs: 17, legacy: 2, internal: 0 },
      option_cell_count: 50,
      option_cell_coverage: { full: 0, partial: 37, absent: 11, intentionally_excluded: 2 },
      option_cell_surfaces: { standard: 29, labs: 19, legacy: 2, internal: 0 },
    });
    expect(capabilityRegistryV2.visibleProductCapabilities(false)).toHaveLength(26);
    expect(capabilityRegistryV2.visibleProductCapabilities(true)).toHaveLength(29);
  });

  it("keeps the exact General SEM multiple-mediation bootstrap cell opt-in and engine-only", () => {
    const capabilityId = "smartpls.mediation";
    const cellId = "qpls3.pls.general_sem_multiple_mediation_bootstrap";
    expect(capabilityRegistryV2.quickPlsCell(cellId)).toHaveLength(1);
    expect(capabilityRegistryV2.quickPlsCell(cellId)[0]).toMatchObject({
      row: { capability_id: capabilityId },
      cell: {
        capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
        coverage_state: "partial",
        evidence_state: "engine_only",
        surface: "labs",
      },
    });
    expect(capabilityRegistryV2.availability(capabilityId, cellId, false)).toMatchObject({
      visibility: "hidden",
      selectable: false,
      reason: "labs_disabled",
    });
    expect(capabilityRegistryV2.availability(capabilityId, cellId, true)).toMatchObject({
      visibility: "experimental",
      selectable: true,
      customer_label: "Experimental",
      reason: "labs_ready",
    });
  });

  it("keeps Blindfolding and GoF as the only explicit Legacy exclusions", () => {
    for (const capabilityId of ["smartpls.blindfolding", "smartpls.gof"]) {
      const row = capabilityRegistryV2.requireCapability(capabilityId);
      expect(row).toMatchObject({
        official_lifecycle: "legacy",
        coverage_state: "intentionally_excluded",
        evidence_state: "absent",
        surface: "legacy",
      });
      expect(capabilityRegistryV2.availability(capabilityId, row.option_cells[0].cell_id, true)).toMatchObject({
        visibility: "hidden",
        selectable: false,
        reason: "intentionally_excluded",
      });
    }
  });

  it("indexes known QuickPLS cells without assuming that a cell maps to one official row", () => {
    expect(capabilityRegistryV2.quickPlsCell("qpls3.pls.algorithm")).toHaveLength(1);
    expect(capabilityRegistryV2.quickPlsCell("qpls3.pls.algorithm")[0]).toMatchObject({
      row: { capability_id: "smartpls.pls_algorithm" },
      cell: { coverage_state: "partial", evidence_state: "release_qualified", surface: "standard" },
      link: { capability_version: "pls_pm_v1" },
    });
    expect(capabilityRegistryV2.availability("smartpls.pls_algorithm", "qpls3.pls.algorithm", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.wpls", "qpls3.pls.weighted", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.plsc", "qpls3.pls.consistent", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.cca", "qpls3.assessment.cca_residuals", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.cta_pls", "qpls3.assessment.cta_pls", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.ipma", "qpls3.assessment.ipma", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.pls_bootstrapping", "qpls3.inference.bootstrap", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.higher_order_models", "qpls3.pls.higher_order_two_stage", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    for (const capabilityId of ["smartpls.micom", "smartpls.mga"]) {
      expect(capabilityRegistryV2.availability(capabilityId, "qpls3.groups.micom_permutation_mga", false)).toMatchObject({
        visibility: "supported",
        selectable: true,
        customer_label: "Supported",
      });
    }
    for (const cellId of [
      "qpls3.groups.micom_permutation_mga",
      "qpls3.inference.structural_path_randomization",
    ]) {
      expect(capabilityRegistryV2.availability("smartpls.permutation", cellId, false)).toMatchObject({
        visibility: "supported",
        selectable: true,
        customer_label: "Supported",
      });
    }
    for (const capabilityId of ["smartpls.plspredict", "smartpls.cvpat"]) {
      expect(capabilityRegistryV2.availability(capabilityId, "qpls3.prediction.plspredict_cvpat", false)).toMatchObject({
        visibility: "supported",
        selectable: true,
        customer_label: "Supported",
      });
    }
    expect(capabilityRegistryV2.quickPlsCell("qpls3.standalone.pca").map((match) => match.row.capability_id)).toEqual([
      "smartpls.pca_core",
      "smartpls.pca_cbsem",
    ]);
    for (const capabilityId of ["smartpls.pca_core", "smartpls.pca_cbsem"]) {
      expect(capabilityRegistryV2.availability(capabilityId, "qpls3.standalone.pca", false)).toMatchObject({
        visibility: "supported",
        selectable: true,
        customer_label: "Supported",
      });
    }
    expect(capabilityRegistryV2.availability("smartpls.gsca", "qpls3.gsca.als", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.nca", "qpls3.standalone.nca", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.logistic_regression", "qpls3.standalone.logistic", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.regression", "qpls3.standalone.ols", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability(
      "smartpls.regression_bootstrapping",
      "qpls3.standalone.regression_bootstrap",
      false,
    )).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.cbsem", "qpls3.cbsem.ml", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.cfa", "qpls3.cbsem.ml", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.availability("smartpls.cbsem_bootstrapping", "qpls3.cbsem.bootstrap", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
    });
    expect(capabilityRegistryV2.quickPlsCell("qpls3.unknown")).toEqual([]);
  });

  it("fails closed on unknown and extra enum values", () => {
    const unknown = mutableRegistry();
    unknown.capabilities[0].option_cells[0].coverage_state = "mostly_full";
    expect(() => parseCapabilityRegistryV2(unknown)).toThrow(/coverage_state.*must be one of/);

    const extra = mutableRegistry();
    extra.state_contract.coverage_states.push("mostly_full");
    expect(() => parseCapabilityRegistryV2(extra)).toThrow(/coverage_states.*must equal/);

    const duplicate = mutableRegistry();
    duplicate.state_contract.evidence_states.push("release_qualified");
    expect(() => parseCapabilityRegistryV2(duplicate)).toThrow(/evidence_states.*must equal/);
  });

  it("fails closed on duplicate row, position, and qualification-link identities", () => {
    const duplicateRow = mutableRegistry();
    const duplicateId = duplicateRow.capabilities[0].capability_id;
    duplicateRow.capabilities[1].capability_id = duplicateId;
    duplicateRow.capabilities[1].legacy_row.id = duplicateId;
    for (const link of duplicateRow.capabilities[1].qualification_links) link.capability_id = duplicateId;
    for (const link of duplicateRow.capabilities[1].qualification_spec.links) link.capability_id = duplicateId;
    for (const cell of duplicateRow.capabilities[1].option_cells) {
      cell.capability_id = duplicateId;
      cell.qualification_spec.links[0].capability_id = duplicateId;
    }
    expect(() => parseCapabilityRegistryV2(duplicateRow)).toThrow(/duplicate capability_id|qualification-link identity/);

    const duplicatePosition = mutableRegistry();
    duplicatePosition.capabilities[1].catalogue_position = duplicatePosition.capabilities[0].catalogue_position;
    duplicatePosition.capabilities[1].legacy_row.catalogue_position = duplicatePosition.capabilities[0].catalogue_position;
    expect(() => parseCapabilityRegistryV2(duplicatePosition)).toThrow(/duplicate catalogue_position/);

    const duplicateLink = mutableRegistry();
    duplicateLink.capabilities[0].option_cells.push(
      JSON.parse(JSON.stringify(duplicateLink.capabilities[0].option_cells[0])),
    );
    expect(() => parseCapabilityRegistryV2(duplicateLink)).toThrow(/duplicate authoritative identity/);
  });

  it("requires scalar surfaces and the exact four-field qualification identity", () => {
    const arraySurface = mutableRegistry();
    arraySurface.capabilities[0].option_cells[0].surface = ["labs"];
    expect(() => parseCapabilityRegistryV2(arraySurface)).toThrow(/surface.*must be one of/);

    const extraLinkField = mutableRegistry();
    extraLinkField.capabilities[0].option_cells[0].qualification_spec.links[0].manifest = "validation/methods/pls_algorithm_v1.manifest.json";
    expect(() => parseCapabilityRegistryV2(extraLinkField)).toThrow(/option_cells\[0\].*must contain exactly/);
  });

  it("moves a full, release-qualified row to Standard when a revised snapshot says so", () => {
    const revised = mutableRegistry();
    revised.capabilities[0].coverage_state = "full";
    revised.capabilities[0].evidence_state = "release_qualified";
    revised.capabilities[0].surface = "standard";
    revised.capabilities[0].option_cells[0].coverage_state = "full";
    revised.capabilities[0].option_cells[0].evidence_state = "release_qualified";
    revised.capabilities[0].option_cells[0].surface = "standard";
    revised.state_contract.baseline_counts.full = 1;
    revised.state_contract.baseline_counts.partial = 31;
    revised.surface_contract.baseline_counts.standard = 26;
    revised.surface_contract.baseline_counts.labs = 17;
    const adapter = new CapabilityRegistryV2Adapter(revised, { requireFrozenStateDistribution: false });
    expect(adapter.availability("smartpls.pls_algorithm", "qpls3.pls.algorithm", false)).toMatchObject({
      visibility: "supported",
      selectable: true,
      customer_label: "Supported",
      reason: "standard_ready",
    });
  });

  it("keeps cells without current derived evidence unavailable even when Labs is enabled", () => {
    expect(capabilityRegistryV2.requireCapability("smartpls.model_fit")).toMatchObject({
      coverage_state: "partial",
      evidence_state: "absent",
      surface: "labs",
      legacy_row: { status: "absent" },
      option_cells: [{ capability_version: "pls_model_fit_v2" }],
    });
    expect(capabilityRegistryV2.availability("smartpls.model_fit", "qpls3.assessment.model_fit", true)).toMatchObject({
      visibility: "hidden",
      selectable: false,
      reason: "not_executable",
    });
  });

  it("rejects row-only promotion because row state is not authoritative", () => {
    const rowOnly = mutableRegistry();
    rowOnly.capabilities[0].coverage_state = "full";
    rowOnly.capabilities[0].surface = "standard";
    expect(() => parseCapabilityRegistryV2(rowOnly, { requireFrozenStateDistribution: false })).toThrow(
      /must equal the derived option-cell projection/,
    );
  });

  it("resolves maturity and visibility independently for two cells in one catalogue row", () => {
    expect(capabilityRegistryV2.availability(
      "smartpls.pls_power_analysis",
      "qpls3.pls.posthoc_technical_minimum_sample_size",
      true,
    )).toMatchObject({ visibility: "supported", selectable: true, reason: "standard_ready" });
    expect(capabilityRegistryV2.availability(
      "smartpls.pls_power_analysis",
      "qpls3.pls.sample_size_power",
      true,
    )).toMatchObject({ visibility: "supported", selectable: true, reason: "standard_ready" });
    expect(capabilityRegistryV2.rowAvailability("smartpls.pls_power_analysis", true)).toMatchObject({
      visibility: "supported",
      selectable: true,
      reason: "standard_ready",
    });
  });

  it("returns a product-safe projection with no evidence or qualification internals", () => {
    const projection = capabilityRegistryV2.productProjection("smartpls.pls_algorithm", true);
    expect(projection).toMatchObject({
      id: "smartpls.pls_algorithm",
      method: "PLS-SEM Algorithm",
      channel: "standard",
      availability: { visibility: "supported", selectable: true, label: "Supported" },
    });
    const serialized = JSON.stringify(projection);
    expect(serialized).not.toMatch(/evidence_state|qualification|manifest|validation\//i);
    expect(Object.keys(projection)).not.toContain("coverage_state");
  });
});
