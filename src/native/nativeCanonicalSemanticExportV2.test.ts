import { describe, expect, it } from "vitest";
import { parseCanonicalResultSemanticExportJsonV2 } from "../domain/canonicalResultSemanticExportV2";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import { completedSamplePlsRun } from "../data/smokeRun";
import {
  previewNativeCanonicalDocumentSemanticExportV2,
  previewNativeCanonicalSemanticExportV2,
} from "./nativeCanonicalSemanticExportV2";

function nonlinearDocument(): CanonicalResultDocumentV2 {
  const base = { registry_schema_version: 2 as const, capability_id: "smartpls.pls_algorithm", cell_id: "qpls3.pls.algorithm", capability_version: "pls_pm_v1" };
  const primary = { registry_schema_version: 2 as const, capability_id: "smartpls.nonlinear_relationships", cell_id: "qpls3.pls.nonlinear_quadratic", capability_version: "pls_quadratic_nonlinear_effects_v1" };
  const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
  const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
  const text = (value: string) => ({ kind: "text" as const, value });
  const number = (value: number) => ({ kind: "number" as const, value });
  const primaryOwner = [primary];
  return {
    schema_version: 2,
    document_id: "nonlinear-result-v7",
    title: "PLS nonlinear quadratic diagnostics",
    provenance: {
      run_id: "run-v7", project_id: "project-v7", model_id: "model-v7",
      model_digest: "a".repeat(64), dataset_id: "dataset-v7", dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe-v7", recipe_digest: "c".repeat(64), capability_cell: primary,
      method_version: "pls_quadratic_nonlinear_effects_v1",
      engine_version: "compiled_recipe_v4_pls_plan_v2_execution_v7", seed: 42, workers: 1,
      started_at: "2026-08-16T00:00:00Z", completed_at: "2026-08-16T00:00:01Z",
    },
    capability_cells: [primary, base],
    sections: [
      { id: "run_details", title: "Run details", table_ids: ["estimation_summary", "point_estimate_attribution"], chart_ids: [], capability_cells: [base] },
      { id: "measurement_model", title: "Measurement model", table_ids: ["outer_model"], chart_ids: [], capability_cells: [base] },
      { id: "structural_model", title: "Structural model", table_ids: ["structural_paths", "effects", "r_squared"], chart_ids: [], capability_cells: [base] },
      { id: "nonlinear_relationships", title: "Nonlinear relationships", table_ids: ["nonlinear_quadratic_diagnostics", "nonlinear_equation_fit", "nonlinear_method_scope"], chart_ids: [], capability_cells: primaryOwner },
    ],
    tables: [
      {
        id: "estimation_summary", title: "Estimation summary",
        columns: [{ id: "converged", label: "converged", data_type: "boolean", description: "converged" }, numberColumn("iterations"), numberColumn("used_observations"), numberColumn("omitted_observations")],
        rows: [{ id: "run", cells: [{ kind: "boolean", value: true }, number(4), number(100), number(0)] }],
        footnote_ids: [], capability_cells: [base],
      },
      {
        id: "outer_model", title: "Outer model",
        columns: [textColumn("construct"), textColumn("indicator"), numberColumn("weight"), numberColumn("loading")],
        rows: [{ id: "outer_0000", cells: [text("x"), text("x_one"), number(1), number(0.8)] }],
        footnote_ids: [], capability_cells: [base],
      },
      {
        id: "structural_paths", title: "Structural paths",
        columns: [textColumn("source"), textColumn("target"), numberColumn("coefficient")],
        rows: [{ id: "path_0000", cells: [text("x"), text("y"), number(0.25)] }],
        footnote_ids: [], capability_cells: [base],
      },
      {
        id: "effects", title: "Effects",
        columns: [textColumn("source"), textColumn("target"), numberColumn("direct"), numberColumn("indirect"), numberColumn("total")],
        rows: [{ id: "effect_0000", cells: [text("x"), text("y"), number(0.25), number(0), number(0.25)] }],
        footnote_ids: [], capability_cells: [base],
      },
      {
        id: "r_squared", title: "R-squared",
        columns: [textColumn("construct"), numberColumn("r_squared")],
        rows: [{ id: "r_squared_0000", cells: [text("y"), number(0.4)] }],
        footnote_ids: [], capability_cells: [base],
      },
      {
        id: "point_estimate_attribution", title: "Point estimate attribution",
        columns: ["contract_version", "preprocessing", "indicator_centering", "indicator_scaling", "outer_weights", "outer_loadings", "construct_scores", "structural_paths", "effects"].map(textColumn),
        rows: [{ id: "attribution", cells: [
          "pls_point_estimate_attribution_v1", "standardized", "sample_mean",
          "sample_standard_deviation", "preprocessed_indicator_to_unit_variance_construct_score",
          "indicator_construct_score_correlation", "zero_mean_unit_variance_construct_score",
          "standardized_construct_score_regression", "standardized_structural_path_decomposition",
        ].map(text) }],
        footnote_ids: [], capability_cells: [base],
      },
      {
        id: "nonlinear_quadratic_diagnostics", title: "Quadratic diagnostics",
        columns: [textColumn("source"), textColumn("target"), numberColumn("linear_coefficient"), numberColumn("quadratic_coefficient"), numberColumn("standard_error"), numberColumn("t_statistic"), numberColumn("p_value_two_sided"), textColumn("warning")],
        rows: [{ id: "nonlinear_quadratic_diagnostic_0000", cells: [text("x"), text("y"), number(0.25), number(0.1), number(0.05), number(2), number(0.0455), { kind: "missing", reason: "not_estimated" }] }],
        footnote_ids: [], capability_cells: primaryOwner,
      },
      {
        id: "nonlinear_equation_fit", title: "Nonlinear equation fit",
        columns: [textColumn("target"), numberColumn("linear_r_squared"), numberColumn("augmented_r_squared"), numberColumn("delta_r_squared")],
        rows: [{ id: "nonlinear_equation_fit_0000", cells: [text("y"), number(0.4), number(0.45), number(0.04999999999999999)] }],
        footnote_ids: [], capability_cells: primaryOwner,
      },
      {
        id: "nonlinear_method_scope", title: "Nonlinear method scope",
        columns: [textColumn("method_version"), textColumn("term"), textColumn("warning")],
        rows: [{ id: "nonlinear_method_scope", cells: [text("pls_quadratic_nonlinear_effects_v1"), text("centered_squared_construct_score_v1"), text("Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms.")] }],
        footnote_ids: [], capability_cells: primaryOwner,
      },
    ],
    charts: [], notices: [], exclusions: [], footnotes: [],
    presentation: { default_section_id: "nonlinear_relationships", default_table_id: "nonlinear_quadratic_diagnostics", precision: 4, missing_value_label: "—", chart_defaults: {} },
  };
}

describe("native canonical semantic export preview v2", () => {
  it("builds a readback-verified preview without invoking a file writer", async () => {
    const preview = await previewNativeCanonicalSemanticExportV2(completedSamplePlsRun(), {
      projectId: "project-semantic-export-preview",
      datasetId: "dataset-semantic-export-preview",
    });

    expect(preview.status).toBe("ready");
    if (preview.status !== "ready") return;
    expect(preview.sourceDocumentId).toBe(preview.projection.source.document_id);
    expect(preview.projection.tables.length).toBeGreaterThan(0);
    expect(parseCanonicalResultSemanticExportJsonV2(preview.json)).toMatchObject({
      ok: true,
      document: { document_id: preview.sourceDocumentId },
    });
  });

  it("preserves the native v7 nonlinear tables and ownership through semantic readback", () => {
    const preview = previewNativeCanonicalDocumentSemanticExportV2(nonlinearDocument());
    expect(preview.status).toBe("ready");
    if (preview.status !== "ready") return;
    expect(preview.projection.tables.map((table) => table.id).slice(-3)).toEqual([
      "nonlinear_quadratic_diagnostics", "nonlinear_equation_fit", "nonlinear_method_scope",
    ]);
    const readback = parseCanonicalResultSemanticExportJsonV2(preview.json);
    expect(readback).toMatchObject({
      ok: true,
      document: {
        capability_cells: [
          { cell_id: "qpls3.pls.nonlinear_quadratic" },
          { cell_id: "qpls3.pls.algorithm" },
        ],
        presentation: { default_table_id: "nonlinear_quadratic_diagnostics" },
      },
    });
  });
});
