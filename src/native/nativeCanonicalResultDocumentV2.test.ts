import { describe, expect, it } from "vitest";
import {
  canonicalAnalyticalResultJson,
  validateCanonicalResultDocumentV2,
} from "../domain/canonicalResultDocumentV2";
import {
  ESTABLISHED_METHOD_CONTRACTS_V1,
  establishedCanonicalTableOwnerOptionsV1,
} from "../domain/generated/establishedMethodContractsV1";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import { completedGscaRun } from "./nativeGsca.testFixture";
import {
  convertNativeCovarianceToPresentationV4,
  newNativeScientificCovarianceEdgeV4,
  withNativeConstructEstimandV4,
} from "../domain/semModelV4Authoring";
import {
  type CanonicalGeneralSemEstimateV1,
  type CanonicalGeneralSemResultsV1,
  type NativeCanonicalResultDocumentV2,
  canonicalResultDocumentFromAnalysisRunV2,
  nativeCapabilityRequirementsForTableV2,
  parseNativeCanonicalResultDocumentV2,
} from "./nativeCanonicalResultDocumentV2";
import { buildNativeResultNavigation } from "./nativeResults";

function currentPlsRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    modelId: "corporate-reputation-model",
    modelSnapshot: {
      nodes: [
        {
          id: "competence",
          type: "construct",
          position: { x: 50, y: 100 },
          data: { label: "Competence", shortName: "COMP", mode: "reflective", indicators: ["COMP1", "COMP2", "COMP3"] },
        },
        {
          id: "satisfaction",
          type: "construct",
          position: { x: 420, y: 100 },
          data: { label: "Satisfaction", shortName: "CUSA", mode: "reflective", indicators: ["CUSA1", "CUSA2"] },
        },
      ],
      edges: [{ id: "competence-satisfaction", source: "competence", target: "satisfaction" }],
    },
    provenance: {
      recipe_id: "recipe-pls-runtime-v2",
      dataset_fingerprint: `sha256:${"a".repeat(64)}`,
      method: "pls_pm",
      method_version: base.result!.method_version,
      engine_version: "qpls-estimation-test",
      seed: base.seed,
      settings: {
        method: "pls_pm",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: base.seed,
        workers: 4,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-07-19T11:59:59.000Z",
      completed_at: "2026-07-19T12:00:00.000Z",
    },
  };
}

function completeGeneralSemResultsFixture(
  document: NativeCanonicalResultDocumentV2,
): CanonicalGeneralSemResultsV1 {
  const capabilityCell = document.capability_cells?.find((cell) => cell.capability_id === "smartpls.mediation");
  if (!capabilityCell) throw new Error("Mediation capability fixture is missing");
  const trace = () => ({ model_id: document.provenance.model_id, capability_cell: { ...capabilityCell } });
  const effectValue = (estimate: number): CanonicalGeneralSemEstimateV1 => ({
    estimate,
    standard_error: 0.04,
    lower: estimate - 0.08,
    upper: estimate + 0.08,
    p_value: 0.02,
  });
  return {
    schema_version: 1,
    specific_indirect_effects: [{
      effect_id: "effect_specific_1",
      estimand_id: "estimand_specific_1",
      trace: trace(),
      ordered_relation_ids: ["relation_a", "relation_b"],
      value: effectValue(0.12),
    }],
    aggregate_effects: [
      {
        effect_id: "effect_total_1",
        estimand_id: "estimand_total_indirect_1",
        trace: trace(),
        kind: "total_indirect",
        source_id: "construct_x",
        target_id: "construct_y",
        value: effectValue(0.18),
      },
      {
        effect_id: "effect_total_2",
        estimand_id: "estimand_total_effect_1",
        trace: trace(),
        kind: "total_effect",
        source_id: "construct_x",
        target_id: "construct_y",
        value: effectValue(0.6),
      },
    ],
    conditional_effect_probes: [
      {
        probe_id: "probe_data",
        trace: trace(),
        moderator_id: "moderator_m",
        values: { kind: "data_derived_mean_plus_minus_one_sd", mean: 1, standard_deviation: 1 },
      },
      {
        probe_id: "probe_explicit",
        trace: trace(),
        moderator_id: "moderator_m",
        values: { kind: "explicit", values: [-1, 0, 1] },
      },
    ],
    conditional_effects: [{
      effect_id: "effect_conditional_1",
      estimand_id: "estimand_conditional_1",
      trace: trace(),
      interaction_id: "interaction_1",
      focal_relation_id: "relation_focal_1",
      probe_id: "probe_data",
      moderator_id: "moderator_m",
      probe_value_index: 1,
      moderator_value: 1,
      value: effectValue(0.42),
    }],
    interaction_plots: [{
      plot_id: "interaction_plot_1",
      trace: trace(),
      interaction_id: "interaction_1",
      focal_relation_id: "relation_focal_1",
      focal_predictor_id: "construct_x",
      moderator_id: "moderator_m",
      outcome_id: "construct_y",
      series: [
        {
          series_id: "series_01_low",
          probe_id: "probe_data",
          probe_value_index: 0,
          moderator_value: 0,
          points: [
            { focal_value: -1, predicted_value: -0.2, lower: -0.3, upper: -0.1 },
            { focal_value: 1, predicted_value: 0.2, lower: 0.1, upper: 0.3 },
          ],
        },
        {
          series_id: "series_02_high",
          probe_id: "probe_data",
          probe_value_index: 2,
          moderator_value: 2,
          points: [
            { focal_value: -1, predicted_value: -0.5, lower: -0.6, upper: -0.4 },
            { focal_value: 1, predicted_value: 0.5, lower: 0.4, upper: 0.6 },
          ],
        },
      ],
    }],
    higher_order_stages: [
      {
        stage_id: "hoc_stage_1",
        trace: trace(),
        higher_order_construct_id: "hoc_ab",
        stage_number: 1,
        kind: "lower_order_score_estimation",
        input_construct_ids: ["construct_a", "construct_b"],
        output_variable_ids: ["score_a", "score_b"],
      },
      {
        stage_id: "hoc_stage_2",
        trace: trace(),
        higher_order_construct_id: "hoc_ab",
        stage_number: 2,
        kind: "higher_order_estimation",
        input_construct_ids: ["score_a", "score_b"],
        output_variable_ids: ["hoc_ab"],
        relation_estimates: [{
          relation_id: "relation_hoc_1",
          source_id: "hoc_ab",
          target_id: "construct_y",
          value: effectValue(0.31),
        }],
      },
    ],
    cbsem_fit: [{
      fit_id: "cbsem_fit_1",
      trace: trace(),
      chi_square: 12.5,
      degrees_of_freedom: 8,
      chi_square_p_value: 0.13,
      rmsea: 0.04,
      rmsea_interval: { confidence_level: 0.9, lower: 0.01, upper: 0.08 },
      cfi: 0.98,
      tli: 0.97,
      srmr: 0.03,
      aic: 101.2,
      bic: 120.4,
    }],
    identification_diagnostics: [{
      diagnostic_id: "identification_model_1",
      trace: trace(),
      scope: "model",
      subject_id: document.provenance.model_id,
      status: "identified",
      code: "identified",
      message: "The compiled model passed identification checks.",
      degrees_of_freedom: 8,
    }],
  };
}

const PRIOR_ESTABLISHED_TABLE_OWNERS = [
  {
    tableId: "cca_prior_parity",
    ownerOptions: ["cca"],
    requirements: [
      { capability_id: "smartpls.cca", cell_id: "qpls3.assessment.cca_residuals", option: "cca" },
    ],
  },
  {
    tableId: "gsca_prior_parity",
    ownerOptions: ["gsca"],
    requirements: [
      { capability_id: "smartpls.gsca", cell_id: "qpls3.gsca.als", option: "gsca" },
    ],
  },
  {
    tableId: "ipma_prior_parity",
    ownerOptions: ["ipma"],
    requirements: [
      { capability_id: "smartpls.ipma", cell_id: "qpls3.assessment.ipma", option: "ipma" },
    ],
  },
  {
    tableId: "nca_prior_parity",
    ownerOptions: ["nca"],
    requirements: [
      { capability_id: "smartpls.nca", cell_id: "qpls3.standalone.nca", option: "nca" },
    ],
  },
] as const;

describe("CanonicalResultDocumentV2 native runtime adapter", () => {
  it("adopts generated canonical table owners without changing prior primary tuples", () => {
    for (const prior of PRIOR_ESTABLISHED_TABLE_OWNERS) {
      const generatedOwners = establishedCanonicalTableOwnerOptionsV1(prior.tableId);
      const generatedRequirements = generatedOwners.flatMap((ownerOption) => (
        ESTABLISHED_METHOD_CONTRACTS_V1.flatMap((contract) => contract.capability_requirements
          .filter((item) => item.option === ownerOption)
          .map((item) => ({
            capability_id: item.capability_id,
            cell_id: item.cell_id,
            option: item.option,
          })))
      ));
      expect(generatedOwners).toEqual(prior.ownerOptions);
      expect(generatedRequirements).toEqual(prior.requirements);
      expect(nativeCapabilityRequirementsForTableV2(prior.tableId)).toEqual(prior.requirements);
    }
  });

  it("continues legacy dynamic and unknown table fallbacks when generated ownership does not match", () => {
    expect(establishedCanonicalTableOwnerOptionsV1("plsc_permutation_accounting")).toEqual([]);
    expect(nativeCapabilityRequirementsForTableV2("plsc_permutation_accounting")).toEqual([{
      capability_id: "smartpls.consistent_permutation",
      cell_id: "qpls3.inference.consistent_permutation",
      option: "consistent_permutation",
    }]);
    expect(establishedCanonicalTableOwnerOptionsV1("future_method_table")).toEqual([]);
    expect(nativeCapabilityRequirementsForTableV2("future_method_table")).toBeNull();
  });

  it("attributes PLSc permutation tables to the exact consistent-permutation cell", () => {
    for (const tableId of [
      "plsc_permutation_accounting",
      "plsc_permutation_groups",
      "plsc_permutation_paths",
      "plsc_permutation_outer_loadings",
      "plsc_permutation_construct_criteria",
      "plsc_permutation_failures",
    ]) {
      expect(nativeCapabilityRequirementsForTableV2(tableId)).toEqual([{
        capability_id: "smartpls.consistent_permutation",
        cell_id: "qpls3.inference.consistent_permutation",
        option: "consistent_permutation",
      }]);
    }
    expect(nativeCapabilityRequirementsForTableV2("plsc_reliability")).toEqual([{
      capability_id: "smartpls.plsc",
      cell_id: "qpls3.pls.consistent",
      option: "consistent_pls",
    }]);
  });

  it("builds a strict typed PLS document with exact native table identities", async () => {
    const run = currentPlsRun();
    const built = await canonicalResultDocumentFromAnalysisRunV2(run, {
      projectId: "project-corporate-reputation",
      datasetId: "dataset-corporate-reputation",
    });

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.mode).toBe("current_typed_bridge");
    expect(validateCanonicalResultDocumentV2(built.document)).toEqual({ passed: true, errors: [] });
    expect(built.document.tables.map((table) => table.id)).toEqual(
      buildNativeResultNavigation(run).tables.map((table) => table.id).filter((id) => id !== "blindfolding"),
    );
    expect(new Set(built.document.sections.flatMap((section) => section.table_ids))).toEqual(
      new Set(built.document.tables.map((table) => table.id)),
    );
    const paths = built.document.tables.find((table) => table.id === "direct_effects");
    expect(paths?.rows[0].cells.some((cell) => cell.kind === "number")).toBe(true);
    expect(built.document.provenance).toMatchObject({
      run_id: run.id,
      project_id: "project-corporate-reputation",
      dataset_id: "dataset-corporate-reputation",
      dataset_fingerprint: "a".repeat(64),
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "smartpls.pls_algorithm",
        cell_id: "qpls3.pls.algorithm",
      },
      workers: 4,
    });
    expect(built.document.capability_cells?.map((reference) => reference.capability_id)).toEqual([
      "smartpls.htmt",
      "smartpls.mediation",
      "smartpls.model_fit",
      "smartpls.pls_algorithm",
      "smartpls.pls_bootstrapping",
    ]);
    expect(built.document.tables.find((table) => table.id === "mediation_bootstrap")?.capability_cells
      ?.map((reference) => reference.capability_id)).toEqual([
      "smartpls.mediation",
      "smartpls.pls_bootstrapping",
    ]);
    expect(built.document.sections.every((section) => (section.capability_cells?.length ?? 0) > 0)).toBe(true);
    expect(built.document.tables.map((table) => table.id)).not.toContain("blindfolding");
    expect(built.document.exclusions).toEqual([
      expect.objectContaining({
        id: "historical_blindfolding_omitted",
        capability_cell: expect.objectContaining({ capability_id: "smartpls.blindfolding" }),
      }),
    ]);
  });

  it("adapts a current non-PLS family and preserves GSCA table ordering", async () => {
    const run = completedGscaRun();
    const built = await canonicalResultDocumentFromAnalysisRunV2(run);

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.mode).toBe("current_typed_bridge");
    expect(built.document.provenance.capability_cell).toMatchObject({
      capability_id: "smartpls.gsca",
      cell_id: "qpls3.gsca.als",
    });
    expect(built.document.tables.map((table) => table.id)).toEqual(
      buildNativeResultNavigation(run).tables.map((table) => table.id),
    );
    expect(built.document.tables.map((table) => table.id)).toContain("gsca_fit");
    expect(built.document.capability_cells).toEqual([built.document.provenance.capability_cell]);
    expect(new Set(built.document.notices.map((notice) => notice.message)).size).toBe(built.document.notices.length);
    expect(built.document.notices.map((notice) => notice.code)).toContain("legacy_dataset_fingerprint_identifier");
  });

  it("keeps display preferences and diagram presentation out of analytical equality", async () => {
    const firstRun = currentPlsRun();
    const secondRun = currentPlsRun();
    secondRun.modelSnapshot!.nodes[0].position = { x: 999, y: 888 };
    secondRun.modelSnapshot!.diagramLayout = {
      diagramVersion: "sem_designer_v1",
      constructLayouts: {},
      indicatorLayouts: {},
      edgeLayouts: {},
      diagramTheme: "journal_mono",
      showGrid: false,
      layoutLocked: true,
    };
    secondRun.provenance!.settings.workers = 1;

    const first = await canonicalResultDocumentFromAnalysisRunV2(firstRun, {
      presentation: { precision: 2, missingValueLabel: "N/A", chartDefaults: { palette: "institutional_navy" } },
    });
    const second = await canonicalResultDocumentFromAnalysisRunV2(secondRun, {
      presentation: { precision: 8, missingValueLabel: "—", chartDefaults: { palette: "journal_mono", show_values: true } },
    });

    expect(first.ok && second.ok).toBe(true);
    if (!first.ok || !second.ok) return;
    expect(first.document.presentation).not.toEqual(second.document.presentation);
    expect(first.document.provenance.model_digest).toBe(second.document.provenance.model_digest);
    expect(first.document.provenance.recipe_digest).toBe(second.document.provenance.recipe_digest);
    expect(canonicalAnalyticalResultJson(first.document)).toBe(canonicalAnalyticalResultJson(second.document));
  });

  it("treats the recipe id as provenance rather than an analytical setting", async () => {
    const firstRun = currentPlsRun();
    const secondRun = structuredClone(firstRun);
    secondRun.id = "run-pls-runtime-v2-repeat";
    secondRun.provenance!.recipe_id = "recipe-pls-runtime-v2-repeat";

    const first = await canonicalResultDocumentFromAnalysisRunV2(firstRun);
    const second = await canonicalResultDocumentFromAnalysisRunV2(secondRun);
    expect(first.ok).toBe(true);
    expect(second.ok).toBe(true);
    if (!first.ok || !second.ok) return;

    expect(first.document.provenance.recipe_id).not.toBe(second.document.provenance.recipe_id);
    expect(first.document.provenance.recipe_digest).toBe(second.document.provenance.recipe_digest);
  });

  it("keeps presentation-only edges outside the scientific model digest", async () => {
    const firstRun = currentPlsRun();
    const secondRun = structuredClone(firstRun);
    secondRun.modelSnapshot!.edges.push(convertNativeCovarianceToPresentationV4({
      id: "visual-covariance",
      source: "competence",
      target: "satisfaction",
      data: { role: "covariance" },
    }));

    const first = await canonicalResultDocumentFromAnalysisRunV2(firstRun);
    const second = await canonicalResultDocumentFromAnalysisRunV2(secondRun);
    expect(first.ok).toBe(true);
    expect(second.ok).toBe(true);
    if (!first.ok || !second.ok) return;
    expect(first.document.provenance.model_digest).toBe(second.document.provenance.model_digest);
  });

  it("binds explicit estimands and scientific covariances into the model digest", async () => {
    const baseline = currentPlsRun();
    const changed = structuredClone(baseline);
    changed.modelSnapshot!.nodes[0] = withNativeConstructEstimandV4(
      changed.modelSnapshot!.nodes[0],
      { kind: "common_factor", marker_indicator: "COMP1" },
    );
    changed.modelSnapshot!.edges.push(newNativeScientificCovarianceEdgeV4(
      "model-covariance",
      "competence",
      "satisfaction",
    ));

    const first = await canonicalResultDocumentFromAnalysisRunV2(baseline);
    const second = await canonicalResultDocumentFromAnalysisRunV2(changed);
    expect(first.ok).toBe(true);
    expect(second.ok).toBe(true);
    if (!first.ok || !second.ok) return;
    expect(first.document.provenance.model_digest).not.toBe(second.document.provenance.model_digest);
  });

  it("fails closed for non-finite analytical values and tampered method identity", async () => {
    const nonFinite = currentPlsRun();
    nonFinite.result!.paths[0].coefficient = Number.NaN;
    await expect(canonicalResultDocumentFromAnalysisRunV2(nonFinite)).resolves.toMatchObject({
      ok: false,
      code: "invalid_analytical_payload",
    });

    const unknownMethod = currentPlsRun();
    (unknownMethod.provenance as { method: string }).method = "tampered_method";
    (unknownMethod.provenance!.settings as { method: string }).method = "tampered_method";
    await expect(canonicalResultDocumentFromAnalysisRunV2(unknownMethod)).resolves.toMatchObject({
      ok: false,
      code: "unresolved_capability_cell",
    });
  });

  it("keeps historical runs readable through a text-only fallback", async () => {
    const historical = completedSamplePlsRun();
    expect(historical.provenance).toBeUndefined();

    const built = await canonicalResultDocumentFromAnalysisRunV2(historical);

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.mode).toBe("historical_text_fallback");
    expect(built.document.sections).toHaveLength(1);
    expect(built.document.sections[0].id).toBe("historical_results");
    expect(built.document.tables.flatMap((table) => table.rows).flatMap((row) => row.cells)
      .every((cell) => cell.kind === "text")).toBe(true);
    expect(built.document.tables.map((table) => table.id)).toContain("blindfolding");
    expect(validateCanonicalResultDocumentV2(built.document)).toEqual({ passed: true, errors: [] });
  });

  it("strictly and losslessly reads every General SEM result family", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const document = structuredClone(built.document);
    document.general_sem_results = completeGeneralSemResultsFixture(document);
    const encoded = JSON.stringify(document);

    const readback = parseNativeCanonicalResultDocumentV2(JSON.parse(encoded));

    expect(readback).toEqual(document);
    expect(JSON.stringify(readback)).toBe(encoded);
    expect(readback.general_sem_results?.specific_indirect_effects).toHaveLength(1);
    expect(readback.general_sem_results?.aggregate_effects?.map((effect) => effect.kind)).toEqual([
      "total_indirect",
      "total_effect",
    ]);
    expect(readback.general_sem_results?.conditional_effect_probes?.map((probe) => probe.values.kind)).toEqual([
      "data_derived_mean_plus_minus_one_sd",
      "explicit",
    ]);
    expect(readback.general_sem_results?.interaction_plots?.[0].series).toHaveLength(2);
    expect(readback.general_sem_results?.higher_order_stages?.map((stage) => stage.kind)).toEqual([
      "lower_order_score_estimation",
      "higher_order_estimation",
    ]);
    expect(readback.general_sem_results?.cbsem_fit?.[0].rmsea_interval?.confidence_level).toBe(0.9);
    expect(readback.general_sem_results?.identification_diagnostics?.[0].status).toBe("identified");
  });

  it("preserves legacy omission and byte ordering when General SEM results are absent", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const encoded = JSON.stringify(built.document);

    const readback = parseNativeCanonicalResultDocumentV2(JSON.parse(encoded));

    expect(Object.prototype.hasOwnProperty.call(readback, "general_sem_results")).toBe(false);
    expect(JSON.stringify(readback)).toBe(encoded);
  });

  it("rejects unknown extension fields, non-finite values, schemas, and discriminators", async () => {
    const built = await canonicalResultDocumentFromAnalysisRunV2(currentPlsRun());
    expect(built.ok).toBe(true);
    if (!built.ok) return;
    const fixture = structuredClone(built.document);
    fixture.general_sem_results = completeGeneralSemResultsFixture(fixture);

    const unknown = structuredClone(fixture) as unknown as {
      general_sem_results: { specific_indirect_effects: Array<{ value: Record<string, unknown> }> };
    };
    unknown.general_sem_results.specific_indirect_effects[0]!.value.unexpected = true;
    expect(() => parseNativeCanonicalResultDocumentV2(unknown)).toThrowError(expect.objectContaining({
      code: "schema.unknown_field",
      path: "general_sem_results.specific_indirect_effects[0].value.unexpected",
    }));

    const nonFinite = structuredClone(fixture);
    nonFinite.general_sem_results!.cbsem_fit![0]!.rmsea = Number.NaN;
    expect(() => parseNativeCanonicalResultDocumentV2(nonFinite)).toThrowError(expect.objectContaining({
      code: "schema.non_finite",
    }));

    const wrongSchema = structuredClone(fixture) as unknown as {
      general_sem_results: { schema_version: number };
    };
    wrongSchema.general_sem_results.schema_version = 2;
    expect(() => parseNativeCanonicalResultDocumentV2(wrongSchema)).toThrowError(expect.objectContaining({
      code: "schema.version_unsupported",
      path: "general_sem_results.schema_version",
    }));

    const wrongKind = structuredClone(fixture) as unknown as {
      general_sem_results: { aggregate_effects: Array<{ kind: string }> };
    };
    wrongKind.general_sem_results.aggregate_effects[0]!.kind = "direct_effect";
    expect(() => parseNativeCanonicalResultDocumentV2(wrongKind)).toThrowError(expect.objectContaining({
      code: "schema.invalid_discriminator",
      path: "general_sem_results.aggregate_effects[0].kind",
    }));
  });
});
