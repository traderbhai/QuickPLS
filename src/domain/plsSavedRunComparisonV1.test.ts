import { describe, expect, it } from "vitest";
import {
  capabilityCellReferenceIdentityV2,
  type CanonicalResultCell,
  type CanonicalResultColumn,
  type CanonicalResultDocumentV2,
  type CanonicalResultTable,
  type CapabilityCellReferenceV2,
} from "./canonicalResultDocumentV2";
import { buildPlsSavedRunComparisonV1 } from "./plsSavedRunComparisonV1";

const plspredictCell: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.plspredict",
  cell_id: "qpls3.prediction.plspredict_cvpat",
  capability_version: "plspredict_indicator_v2",
};
const cvpatCell: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.cvpat",
  cell_id: "qpls3.prediction.plspredict_cvpat",
  capability_version: "plspredict_indicator_v2",
};
const comparisonCell: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.pls_model_comparison",
  cell_id: "qpls3.comparison.pls_models",
  capability_version: "pls_model_comparison_v1",
};

function sortedCells(...cells: CapabilityCellReferenceV2[]): CapabilityCellReferenceV2[] {
  return cells.sort((left, right) => capabilityCellReferenceIdentityV2(left).localeCompare(capabilityCellReferenceIdentityV2(right)));
}

function column(id: string, label: string, data_type: CanonicalResultColumn["data_type"]): CanonicalResultColumn {
  return { id, label, data_type, description: `${label} test value.` };
}

function text(value: string): CanonicalResultCell {
  return { kind: "text", value };
}

function number(value: number): CanonicalResultCell {
  return { kind: "number", value };
}

function missing(): CanonicalResultCell {
  return { kind: "missing", reason: "not_estimated" };
}

function predictionTable(plsRmse: number): CanonicalResultTable {
  return {
    id: "plspredict_indicator_summary",
    title: "Indicator prediction summary (10-fold × 10-repeat)",
    columns: [
      column("construct", "Construct", "text"),
      column("indicator", "Indicator", "text"),
      column("predictor_set", "Predictor set", "text"),
      column("predictors", "Predictors", "number"),
      column("observations", "Observations", "number"),
      column("q2_predict", "Q²_predict", "number"),
      column("pls-sem_rmse", "PLS-SEM RMSE", "number"),
      column("ia_rmse", "IA RMSE", "number"),
      column("lm_rmse", "LM RMSE", "number"),
      column("pls-sem_mae", "PLS-SEM MAE", "number"),
      column("ia_mae", "IA MAE", "number"),
      column("lm_mae", "LM MAE", "number"),
      column("pls-sem_mape", "PLS-SEM MAPE (%)", "number"),
      column("ia_mape", "IA MAPE (%)", "number"),
      column("lm_mape", "LM MAPE (%)", "number"),
      column("mape_observations", "MAPE observations", "number"),
      column("lm_benchmark", "LM benchmark", "text"),
    ],
    rows: [{
      id: "loyalty:loy1",
      cells: [
        text("Loyalty"), text("LOY1"), text("Earliest antecedent indicators"), number(2), number(64),
        number(0.28), number(plsRmse), number(0.55), number(0.43), number(0.32), number(0.44), number(0.34),
        number(10.2), number(12.5), number(10.8), number(64), text("Available"),
      ],
    }],
    footnote_ids: [],
    capability_cells: [plspredictCell],
  };
}

function validationPlanTable(assignmentDigest = `sha256:${"e".repeat(64)}`): CanonicalResultTable {
  return {
    id: "plspredict_validation_plan",
    title: "Cross-validation design",
    columns: [
      column("procedure", "Procedure", "text"),
      column("complete_cases", "Complete cases", "number"),
      column("folds", "Folds", "number"),
      column("repeats", "Repeats", "number"),
      column("assignment", "Assignment", "text"),
      column("assignment_digest", "Assignment digest", "text"),
      column("seed", "Seed", "number"),
      column("test_predictions", "Test predictions", "number"),
    ],
    rows: [{
      id: "primary_plan",
      cells: [text("Primary repeated cross-validation"), number(64), number(10), number(10), text("Seeded balanced folds"), text(assignmentDigest), number(20260815), number(640)],
    }],
    footnote_ids: [],
    capability_cells: [plspredictCell],
  };
}

function cvpatTable(meanLoss: number): CanonicalResultTable {
  return {
    id: "cvpat_benchmark_assessment",
    title: "CVPAT benchmark assessment (single model)",
    columns: [
      column("benchmark", "Benchmark", "text"),
      column("target_set", "Target set", "text"),
      column("loss", "Loss", "text"),
      column("alternative", "Alternative", "text"),
      column("confidence", "Confidence", "text"),
      column("pls-sem_mean_loss", "PLS-SEM mean loss", "number"),
      column("benchmark_mean_loss", "Benchmark mean loss", "number"),
      column("mean_loss_difference_pls-sem_benchmark", "Mean loss difference (PLS-SEM − benchmark)", "number"),
      column("se", "SE", "number"),
      column("t", "t", "number"),
      column("p_one-sided", "p (one-sided)", "number"),
      column("95_ci_lower", "95% CI lower", "number"),
      column("95_ci_upper", "95% CI upper", "number"),
      column("complete_cases", "Complete cases", "number"),
      column("indicators", "Indicators", "number"),
      column("status", "Status", "text"),
      column("supported_conclusion", "Supported conclusion", "text"),
      column("reason", "Reason", "text"),
    ],
    rows: [{
      id: "indicator_average",
      cells: [
        text("Indicator average"), text("All endogenous indicators"), text("Mean squared prediction loss per complete case"),
        text("PLS-SEM loss < benchmark"), text("95%"), number(meanLoss), number(0.30), number(meanLoss - 0.30),
        number(0.04), number(-3), number(0.002), number(-0.20), number(-0.04), number(64), number(1),
        text("Available"), text("PLS-SEM has lower loss"), text("None"),
      ],
    }],
    footnote_ids: [],
    capability_cells: [cvpatCell],
  };
}

function bicTable(value: number, akaikeWeight: number | null = null): CanonicalResultTable {
  return {
    id: "pls_prediction_information_criteria",
    title: "Prediction-oriented information criteria",
    columns: [
      column("outcome", "Outcome", "text"),
      column("bic", "BIC", "number"),
      column("bic_definition", "BIC definition", "text"),
      column("observations", "Observations", "number"),
      column("parameter_count", "Parameter count", "number"),
      column("akaike_weight", "Akaike weight", "number"),
      column("akaike_weight_definition", "Akaike weight definition", "text"),
      column("candidate_set_digest", "Candidate set digest", "text"),
      column("candidate_count", "Candidate count", "number"),
    ],
    rows: [{
      id: "loyalty",
      cells: [
        text("Loyalty"),
        number(value),
        text("prediction_oriented_bic_v1"),
        number(64),
        number(3),
        ...(akaikeWeight === null
          ? [missing(), missing(), missing(), missing()]
          : [number(akaikeWeight), text("akaike_weight_v1"), text(`sha256:${"7".repeat(64)}`), number(2)]),
      ],
    }],
    footnote_ids: [],
    capability_cells: [comparisonCell],
  };
}

interface DocumentOptions {
  id: string;
  modelDigest: string;
  plsRmse: number;
  cvpatLoss?: number;
  bic?: number;
  datasetDigest?: string;
  recipeDigest?: string;
  assignmentDigest?: string;
}

function documentFixture(options: DocumentOptions): CanonicalResultDocumentV2 {
  const tables = [predictionTable(options.plsRmse), validationPlanTable(options.assignmentDigest)];
  if (options.cvpatLoss !== undefined) tables.push(cvpatTable(options.cvpatLoss));
  if (options.bic !== undefined) tables.push(bicTable(options.bic));
  const capabilityCells = sortedCells(
    plspredictCell,
    ...(options.cvpatLoss !== undefined ? [cvpatCell] : []),
    ...(options.bic !== undefined ? [comparisonCell] : []),
  );
  return {
    schema_version: 2,
    document_id: `result:${options.id}`,
    title: options.id,
    provenance: {
      run_id: `run:${options.id}`,
      project_id: "project:comparison",
      model_id: `model:${options.id}`,
      model_digest: options.modelDigest,
      dataset_id: "dataset:shared",
      dataset_fingerprint: options.datasetDigest ?? "a".repeat(64),
      recipe_id: `recipe:${options.id}`,
      recipe_digest: options.recipeDigest ?? "d".repeat(64),
      capability_cell: plspredictCell,
      method_version: "pls_pm_v1",
      engine_version: "qpls-estimation-test",
      seed: 20260815,
      workers: 1,
      started_at: "2026-08-15T10:00:00.000Z",
      completed_at: "2026-08-15T10:00:01.000Z",
    },
    capability_cells: capabilityCells,
    sections: [{
      id: "prediction",
      title: "Prediction",
      table_ids: tables.map((table) => table.id),
      chart_ids: [],
      capability_cells: capabilityCells,
    }],
    tables,
    charts: [],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: "prediction",
      default_table_id: "plspredict_indicator_summary",
      precision: 4,
      missing_value_label: "—",
      chart_defaults: {},
    },
  };
}

describe("PLS saved-run comparison v1", () => {
  it("compares distinct models with identical analytical settings and exact prediction inputs", () => {
    const built = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41, cvpatLoss: 0.16 }),
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36, cvpatLoss: 0.13 }),
    );
    expect(built.status).toBe("ready");
    if (built.status !== "ready") return;
    expect(built.comparison.compatibility).toMatchObject({
      analytical_settings_digest: "d".repeat(64),
      first_model_digest: "b".repeat(64),
      second_model_digest: "c".repeat(64),
    });
    expect(built.comparison.prediction_rows[0].metrics).toContainEqual(expect.objectContaining({
      id: "pls_rmse",
      first: { value: 0.41, missing_reason: null },
      second: { value: 0.36, missing_reason: null },
      change: -0.04999999999999999,
    }));
    expect(built.comparison.cvpat_rows).toHaveLength(1);
    expect(built.comparison.issues).toContainEqual(expect.objectContaining({
      code: "cvpat_between_model_test_unavailable",
      severity: "information",
      title: "CVPAT rows are single-model benchmark assessments",
    }));
  });

  it("rejects a same-model pair even when the saved runs differ", () => {
    const digest = "b".repeat(64);
    const built = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: digest, plsRmse: 0.41 }),
      documentFixture({ id: "second", modelDigest: digest, plsRmse: 0.36 }),
    );
    expect(built).toMatchObject({
      status: "blocked",
      issues: [expect.objectContaining({ code: "same_model_selected", title: "Choose two distinct models" })],
    });
  });

  it("blocks mismatched settings and cross-validation assignments with corrective issues", () => {
    const settings = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 }),
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36, recipeDigest: "f".repeat(64) }),
    );
    expect(settings).toMatchObject({ status: "blocked", issues: [expect.objectContaining({ code: "settings_mismatch" })] });

    const folds = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 }),
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36, assignmentDigest: `sha256:${"9".repeat(64)}` }),
    );
    expect(folds).toMatchObject({ status: "blocked", issues: [expect.objectContaining({ code: "cross_validation_mismatch" })] });
  });

  it("blocks different data, outcomes, and evaluation estimands with distinct issue codes", () => {
    const first = documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 });
    const differentData = buildPlsSavedRunComparisonV1(
      first,
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36, datasetDigest: "9".repeat(64) }),
    );
    expect(differentData).toMatchObject({ status: "blocked", issues: [expect.objectContaining({ code: "dataset_mismatch" })] });

    const outcomeDocument = documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36 });
    const outcomeTable = outcomeDocument.tables.find((table) => table.id === "plspredict_indicator_summary")!;
    outcomeTable.rows[0].cells[outcomeTable.columns.findIndex((item) => item.id === "indicator")] = text("LOY2");
    const differentOutcome = buildPlsSavedRunComparisonV1(first, outcomeDocument);
    expect(differentOutcome).toMatchObject({ status: "blocked", issues: [expect.objectContaining({ code: "prediction_outcome_mismatch" })] });

    const estimandDocument = documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36 });
    const estimandTable = estimandDocument.tables.find((table) => table.id === "plspredict_indicator_summary")!;
    estimandTable.rows[0].cells[estimandTable.columns.findIndex((item) => item.id === "ia_rmse")] = number(0.56);
    const differentEstimand = buildPlsSavedRunComparisonV1(first, estimandDocument);
    expect(differentEstimand).toMatchObject({ status: "blocked", issues: [expect.objectContaining({ code: "prediction_estimand_mismatch" })] });
  });

  it("compares exact attributed BIC without manufacturing Akaike weights", () => {
    const built = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41, bic: -261.602 }),
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36, bic: -261.603 }),
    );
    expect(built.status).toBe("ready");
    if (built.status !== "ready") return;
    const bic = built.comparison.bic_rows[0];
    expect(bic.preferred).toBe("second");
    expect(bic.bic_change).toBeCloseTo(-0.001, 12);
    expect(bic.first_akaike_weight).toBeNull();
    expect(bic.second_akaike_weight).toBeNull();
    expect(bic.akaike_weight_source).toBe("unavailable");
    expect(built.comparison.issues).toContainEqual(expect.objectContaining({
      code: "akaike_weights_missing",
      message: "BIC values are shown, but BIC-only data are not relabeled as Akaike weights.",
    }));
  });

  it("accepts exact stored Akaike weights only for the same two-model candidate set", () => {
    const first = documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 });
    const second = documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36 });
    first.tables.push(bicTable(-261.602, 0.499875));
    second.tables.push(bicTable(-261.603, 0.500125));
    first.capability_cells = sortedCells(...first.capability_cells!, comparisonCell);
    second.capability_cells = sortedCells(...second.capability_cells!, comparisonCell);
    first.sections[0].table_ids.push("pls_prediction_information_criteria");
    second.sections[0].table_ids.push("pls_prediction_information_criteria");
    first.sections[0].capability_cells = first.capability_cells;
    second.sections[0].capability_cells = second.capability_cells;
    const stored = buildPlsSavedRunComparisonV1(first, second);
    expect(stored.status).toBe("ready");
    if (stored.status !== "ready") return;
    expect(stored.comparison.bic_rows[0]).toMatchObject({
      first_akaike_weight: 0.499875,
      second_akaike_weight: 0.500125,
      akaike_weight_source: "stored_exact",
    });
  });

  it("uses an exact deterministic lower-BIC tie rule", () => {
    const built = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41, bic: 12 }),
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36, bic: 12 }),
    );
    expect(built.status).toBe("ready");
    if (built.status !== "ready") return;
    expect(built.comparison.bic_rows[0]).toMatchObject({
      preferred: "tie",
      first_akaike_weight: null,
      second_akaike_weight: null,
    });
  });

  it("does not fabricate BIC or Akaike weights when exact canonical criteria are absent", () => {
    const built = buildPlsSavedRunComparisonV1(
      documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 }),
      documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36 }),
    );
    expect(built.status).toBe("ready");
    if (built.status !== "ready") return;
    expect(built.comparison.bic_rows).toEqual([]);
    expect(built.comparison.issues).toContainEqual(expect.objectContaining({ code: "information_criteria_missing" }));
  });

  it("uses stable column IDs so display-label changes do not alter compatibility", () => {
    const first = documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 });
    const second = documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36 });
    for (const table of second.tables) {
      for (const item of table.columns) item.label = `Localized ${item.label}`;
    }
    const built = buildPlsSavedRunComparisonV1(first, second);
    expect(built.status).toBe("ready");
  });

  it("rejects a renamed column ID even when its display label is unchanged", () => {
    const first = documentFixture({ id: "first", modelDigest: "b".repeat(64), plsRmse: 0.41 });
    const second = documentFixture({ id: "second", modelDigest: "c".repeat(64), plsRmse: 0.36 });
    second.tables.find((table) => table.id === "plspredict_indicator_summary")!
      .columns.find((item) => item.id === "pls-sem_rmse")!.id = "renamed_rmse";
    const built = buildPlsSavedRunComparisonV1(first, second);
    expect(built).toMatchObject({
      status: "blocked",
      issues: [expect.objectContaining({ code: "prediction_contract_invalid" })],
    });
  });
});
