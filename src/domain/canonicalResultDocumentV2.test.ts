import { describe, expect, it } from "vitest";
import {
  canonicalAnalyticalResultJson,
  canonicalResultDocumentFromLegacyTables,
  canonicalResultDocumentJson,
  type CanonicalResultDocumentV2,
  validateCanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";

const digest = "a".repeat(64);

function documentFixture(): CanonicalResultDocumentV2 {
  return {
    schema_version: 2,
    document_id: "result.document:1",
    title: "PLS path results",
    provenance: {
      run_id: "run-1",
      project_id: "project-1",
      model_id: "model-1",
      model_digest: digest,
      dataset_id: "dataset-1",
      dataset_fingerprint: digest,
      recipe_id: "recipe-1",
      recipe_digest: digest,
      capability_cell: {
        registry_schema_version: 2,
        capability_id: "qpls3.pls.algorithm",
        cell_id: "standard.reflective_recursive",
        capability_version: "pls_algorithm_v2",
      },
      method_version: "pls_algorithm_v2",
      engine_version: "qpls-estimation-test",
      seed: 42,
      workers: 4,
      started_at: "2026-08-14T00:00:00Z",
      completed_at: "2026-08-14T00:00:01Z",
    },
    sections: [{ id: "structural", title: "Structural model", table_ids: ["paths"], chart_ids: ["path_plot"] }],
    tables: [{
      id: "paths",
      title: "Path coefficients",
      columns: [
        { id: "path", label: "Path", data_type: "text", description: "Directed structural path", role: "label" },
        { id: "estimate", label: "Estimate", data_type: "number", description: "Standardized path estimate", role: "estimate", default_precision: 4 },
      ],
      rows: [{ id: "x_to_y", cells: [{ kind: "text", value: "X → Y" }, { kind: "number", value: 0.42, display: "0.4200" }] }],
      footnote_ids: ["standardized"],
    }],
    charts: [{
      id: "path_plot",
      title: "Path coefficient",
      description: "One bar showing the standardized X to Y path coefficient.",
      kind: "bar",
      series: [{ id: "estimate", label: "Estimate", points: [{ x: "X → Y", y: 0.42 }] }],
      source_table_id: "paths",
      display: { palette: "institutional_navy", show_values: true },
    }],
    notices: [],
    exclusions: [],
    footnotes: [{ id: "standardized", text: "Standardized estimates." }],
    presentation: {
      default_section_id: "structural",
      default_table_id: "paths",
      precision: 4,
      missing_value_label: "—",
      chart_defaults: { show_legend: true },
    },
  };
}

type GeneralSemAnalyticalFixture = Omit<CanonicalResultDocumentV2, "general_sem_results"> & {
  general_sem_results: {
    schema_version: 1;
    inference_receipt: {
      method_version: string;
      seed: string;
      workers: number;
      usable_replicate_indices_sha256: string;
    };
    specific_indirect_effects: Array<{
      effect_id: string;
      value: { estimate: number };
    }>;
  };
};

function generalSemDocumentFixture(): GeneralSemAnalyticalFixture {
  const document = documentFixture() as unknown as GeneralSemAnalyticalFixture;
  document.general_sem_results = {
    schema_version: 1,
    inference_receipt: {
      method_version: "general_sem_pls_full_model_case_bootstrap_v1",
      seed: "42",
      workers: 4,
      usable_replicate_indices_sha256: "b".repeat(64),
    },
    specific_indirect_effects: [{
      effect_id: "specific:x>m>y",
      value: { estimate: 0.21 },
    }],
  };
  return document;
}

function generalSemAnalyticalJson(document: GeneralSemAnalyticalFixture): string {
  return canonicalAnalyticalResultJson(document as unknown as CanonicalResultDocumentV2);
}

function cbsemV3DocumentFixture(): CanonicalResultDocumentV2 {
  const document = documentFixture();
  const pointCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.cbsem",
    cell_id: "qpls3.cbsem.general_sem_ml",
    capability_version: "cbsem_general_sem_ml_v1",
  };
  const bootstrapCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.cbsem_bootstrapping",
    cell_id: "qpls3.cbsem.bootstrap.recursive_sem",
    capability_version: "cbsem_exact_recursive_sem_case_bootstrap_v1",
  };
  document.provenance.capability_cell = pointCell;
  document.provenance.method_version = "cbsem_general_sem_ml_v1";
  document.capability_cells = [pointCell, bootstrapCell];
  document.sections[0].capability_cells = [pointCell];
  document.tables[0].capability_cells = [pointCell];
  const pointTrace = { model_id: document.provenance.model_id, capability_cell: pointCell };
  const bootstrapTrace = { model_id: document.provenance.model_id, capability_cell: bootstrapCell };
  document.general_sem_results = {
    schema_version: 1,
    cbsem_parameters: [{
      parameter_id: "parameter_loading_x1",
      trace: pointTrace,
      role: "loading",
      target: { kind: "loading", factor_id: "construct_x", indicator_id: "observed_x1" },
      relation_id: "relation_loading_x1",
      state: { kind: "free", equality_label: "metric equality", lower: -2, upper: 2 },
      estimate: 0.8,
      standard_error: 0.1,
      z_value: 8,
      p_value: 0,
      standardized_estimate: 0.75,
    }],
    cbsem_bootstrap_receipt: {
      capability_cell: bootstrapCell,
      method_version: "cbsem_exact_recursive_sem_case_bootstrap_v1",
      resampling_operation_version: "cbsem_recursive_sem_full_ml_case_bootstrap_v1",
      quantile_method_version: "type7_quantile_v1",
      compiled_plan_sha256: "b".repeat(64),
      base_plan_sha256: "c".repeat(64),
      parameter_inventory_sha256: "2c097a401178e7164c09e3d31fc758a40934431f161cca45b2094d4aa8b72bc2",
      model_scientific_sha256: document.provenance.model_digest,
      general_sem_config_sha256: "e".repeat(64),
      recipe_analytical_sha256: document.provenance.recipe_digest,
      source_dataset_fingerprint: document.provenance.dataset_fingerprint,
      complete_case_frame_sha256: "f".repeat(64),
      usable_replicate_indices_sha256: "1".repeat(64),
      confidence_level: 0.95,
      resamples_requested: 500,
      resamples_usable: 500,
      minimum_usable_resamples: 450,
      seed: "42",
      workers: 4,
      complete_model_reestimated_per_replicate: true,
      failed_replicates: [],
    },
    cbsem_bootstrap_inference: [{
      parameter_id: "parameter_loading_x1",
      trace: bootstrapTrace,
      point_estimate: 0.8,
      outcome: {
        kind: "available",
        value: {
          estimate: 0.8,
          bootstrap_mean: 0.81,
          bootstrap_bias: 0.01,
          standard_error: 0.05,
          lower: 0.7,
          upper: 0.9,
          p_value: 1 / 501,
          bootstrap_usable_replicates: 500,
          bootstrap_two_sided_exceedances: 0,
        },
      },
    }],
  };
  return document;
}

describe("CanonicalResultDocumentV2", () => {
  it("accepts a typed, cross-referenced result document", () => {
    expect(validateCanonicalResultDocumentV2(documentFixture())).toEqual({ passed: true, errors: [] });
  });

  it("accepts bare and versioned v2 lowercase dataset fingerprints", () => {
    const bare = documentFixture();
    const versioned = documentFixture();
    versioned.provenance.dataset_fingerprint = `v2:${digest}`;

    expect(validateCanonicalResultDocumentV2(bare)).toEqual({ passed: true, errors: [] });
    expect(validateCanonicalResultDocumentV2(versioned)).toEqual({ passed: true, errors: [] });
  });

  it("rejects duplicate IDs, row shape drift, nonfinite values, and dangling references", () => {
    const document = documentFixture();
    document.sections.push({ ...document.sections[0] });
    document.tables[0].rows[0].cells = [{ kind: "number", value: Number.NaN }];
    document.charts[0].source_table_id = "missing_table";
    document.charts[0].series[0].points[0].x = Number.POSITIVE_INFINITY;
    document.presentation.default_section_id = "missing_section";

    const validation = validateCanonicalResultDocumentV2(document);

    expect(validation.passed).toBe(false);
    expect(validation.errors).toEqual(expect.arrayContaining([
      expect.stringContaining("sections contains duplicate IDs"),
      expect.stringContaining("has 1 cells; expected 2"),
      expect.stringContaining("references missing table missing_table"),
      expect.stringContaining("point 0 x must be finite"),
      expect.stringContaining("default_section_id is missing"),
    ]));
  });

  it("canonicalizes object-key order while preserving meaningful array order", () => {
    const first = documentFixture();
    const second = JSON.parse(JSON.stringify(first)) as CanonicalResultDocumentV2;
    second.presentation = {
      chart_defaults: { show_legend: true },
      missing_value_label: "—",
      precision: 4,
      default_table_id: "paths",
      default_section_id: "structural",
    };

    expect(canonicalResultDocumentJson(second)).toBe(canonicalResultDocumentJson(first));
    second.tables[0].rows.reverse();
    expect(canonicalResultDocumentJson(second)).toBe(canonicalResultDocumentJson(first));
  });

  it("excludes presentation, cached formatting, workers, and timing from analytical equality", () => {
    const first = documentFixture();
    const second = documentFixture();
    second.presentation.precision = 6;
    second.presentation.chart_defaults.palette = "high_contrast";
    second.tables[0].rows[0].cells[1] = { kind: "number", value: 0.42, display: "0.420000" };
    second.charts[0].display.palette = "journal_mono";
    second.provenance.workers = 1;
    second.provenance.completed_at = "2026-08-14T00:00:09Z";

    expect(canonicalResultDocumentJson(second)).not.toBe(canonicalResultDocumentJson(first));
    expect(canonicalAnalyticalResultJson(second)).toBe(canonicalAnalyticalResultJson(first));
  });

  it("includes General SEM scientific results and inference identity in analytical equality", () => {
    const first = generalSemDocumentFixture();
    const changedEstimate = generalSemDocumentFixture();
    changedEstimate.general_sem_results.specific_indirect_effects[0].value.estimate = 0.22;
    const changedReceipt = generalSemDocumentFixture();
    changedReceipt.general_sem_results.inference_receipt.usable_replicate_indices_sha256 = "c".repeat(64);

    expect(generalSemAnalyticalJson(changedEstimate)).not.toBe(generalSemAnalyticalJson(first));
    expect(generalSemAnalyticalJson(changedReceipt)).not.toBe(generalSemAnalyticalJson(first));
  });

  it("excludes only inference receipt workers from General SEM analytical equality", () => {
    const first = generalSemDocumentFixture();
    const workerOnlyChange = generalSemDocumentFixture();
    workerOnlyChange.general_sem_results.inference_receipt.workers = 1;

    expect(canonicalResultDocumentJson(workerOnlyChange as unknown as CanonicalResultDocumentV2))
      .not.toBe(canonicalResultDocumentJson(first as unknown as CanonicalResultDocumentV2));
    expect(generalSemAnalyticalJson(workerOnlyChange)).toBe(generalSemAnalyticalJson(first));
  });

  it("validates typed CB-SEM V3 point and recursive-bootstrap ledgers fail closed", () => {
    const document = cbsemV3DocumentFixture();
    expect(validateCanonicalResultDocumentV2(document)).toEqual({ passed: true, errors: [] });

    const boundTamper = JSON.parse(JSON.stringify(document)) as CanonicalResultDocumentV2;
    const state = boundTamper.general_sem_results!.cbsem_parameters![0].state;
    if (state.kind !== "free") throw new Error("expected free parameter fixture");
    state.upper = 0.5;
    expect(validateCanonicalResultDocumentV2(boundTamper).errors).toEqual(expect.arrayContaining([
      expect.stringContaining("estimate must satisfy its declared bounds"),
    ]));

    const fixedInference = JSON.parse(JSON.stringify(document)) as CanonicalResultDocumentV2;
    const fixedRow = fixedInference.general_sem_results!.cbsem_parameters![0];
    fixedRow.state = { kind: "fixed", value: 0.8 };
    delete fixedRow.standard_error;
    delete fixedRow.z_value;
    delete fixedRow.p_value;
    expect(validateCanonicalResultDocumentV2(fixedInference).errors).toEqual(expect.arrayContaining([
      expect.stringContaining("cannot publish inference for a fixed parameter"),
    ]));

    const receiptTamper = JSON.parse(JSON.stringify(document)) as CanonicalResultDocumentV2;
    receiptTamper.general_sem_results!.cbsem_bootstrap_receipt!.workers = 1;
    expect(validateCanonicalResultDocumentV2(receiptTamper).errors).toEqual(expect.arrayContaining([
      expect.stringContaining("workers must be between 1 and 64 and equal provenance.workers"),
    ]));

    const workerOnlyChange = JSON.parse(JSON.stringify(document)) as CanonicalResultDocumentV2;
    workerOnlyChange.provenance.workers = 1;
    workerOnlyChange.general_sem_results!.cbsem_bootstrap_receipt!.workers = 1;
    expect(validateCanonicalResultDocumentV2(workerOnlyChange)).toEqual({ passed: true, errors: [] });
    expect(canonicalResultDocumentJson(workerOnlyChange)).not.toBe(canonicalResultDocumentJson(document));
    expect(canonicalAnalyticalResultJson(workerOnlyChange)).toBe(canonicalAnalyticalResultJson(document));
  });

  it("preserves legacy string tables without inferring numeric meaning", () => {
    const base = documentFixture();
    const migrated = canonicalResultDocumentFromLegacyTables({
      document_id: "historical.result:1",
      title: "Historical result",
      provenance: base.provenance,
    }, [{
      id: "legacy_paths",
      title: "Paths",
      columns: ["Path", "Estimate"],
      rows: [["X → Y", "0.4200"]],
      warning: "This result was created by a historical method version.",
    }]);

    expect(validateCanonicalResultDocumentV2(migrated)).toEqual({ passed: true, errors: [] });
    expect(migrated.tables[0].rows[0].cells[1]).toEqual({ kind: "text", value: "0.4200" });
    expect(migrated.notices).toHaveLength(1);
    expect(migrated.capability_cells).toBeUndefined();
    expect(migrated.tables[0].capability_cells).toBeUndefined();
  });

  it("validates explicit multi-cell attribution while retaining one primary cell", () => {
    const document = documentFixture();
    const primary = { ...document.provenance.capability_cell };
    const htmt = {
      registry_schema_version: 2 as const,
      capability_id: "smartpls.htmt",
      cell_id: "qpls3.assessment.htmt",
      capability_version: "ringle_et_al_htmt_plus_v1",
    };
    document.capability_cells = [primary, htmt];
    document.tables[0].capability_cells = [primary, htmt];
    document.sections[0].capability_cells = [primary, htmt];

    expect(validateCanonicalResultDocumentV2(document)).toEqual({ passed: true, errors: [] });

    delete document.tables[0].capability_cells;
    expect(validateCanonicalResultDocumentV2(document)).toMatchObject({
      passed: false,
      errors: expect.arrayContaining(["table paths must declare capability_cells"]),
    });
  });
});
