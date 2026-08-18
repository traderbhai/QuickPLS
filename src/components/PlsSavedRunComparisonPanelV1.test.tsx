import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { PlsSavedRunComparisonDocumentV1 } from "../domain/plsSavedRunComparisonV1";
import { PlsSavedRunComparisonPanelV1 } from "./PlsSavedRunComparisonPanelV1";

function comparisonFixture(): PlsSavedRunComparisonDocumentV1 {
  return {
    schema_version: 1,
    kind: "descriptive_saved_run_projection",
    surface: "labs",
    comparison_id: "pls_saved_run_comparison:first:to:second",
    source_documents: { first_document_id: "first", second_document_id: "second" },
    compatibility: {
      dataset_fingerprint: "a".repeat(64),
      method_version: "pls_pm_v1",
      analytical_settings_digest: "d".repeat(64),
      first_model_digest: "b".repeat(64),
      second_model_digest: "c".repeat(64),
      cross_validation_plan: { Folds: 10, Repeats: 10 },
    },
    prediction_rows: [{
      id: "loyalty:loy1",
      construct: "Loyalty",
      indicator: "LOY1",
      first_predictor_count: 2,
      second_predictor_count: 3,
      metrics: [{
        id: "pls_rmse",
        label: "PLS-SEM RMSE",
        preference: "lower",
        first: { value: 0.41, missing_reason: null },
        second: { value: 0.36, missing_reason: null },
        change: -0.05,
      }],
    }],
    cvpat_rows: [{
      id: "indicator_average",
      benchmark: "Indicator average",
      target_set: "All endogenous indicators",
      loss: "Mean squared prediction loss per complete case",
      alternative: "PLS-SEM loss < benchmark",
      confidence: "95%",
      first: {
        pls_mean_loss: { value: 0.1681, missing_reason: null },
        benchmark_mean_loss: { value: 0.3025, missing_reason: null },
        mean_loss_difference: { value: -0.1344, missing_reason: null },
        standard_error: { value: 0.04, missing_reason: null },
        t_statistic: { value: -3.36, missing_reason: null },
        p_value_one_sided: { value: 0.001, missing_reason: null },
        confidence_interval_lower: { value: -0.2, missing_reason: null },
        confidence_interval_upper: { value: -0.0688, missing_reason: null },
        observations: 64,
        indicators: 1,
        status: "Available",
        conclusion: "PLS-SEM has lower loss",
        reason: "",
      },
      second: {
        pls_mean_loss: { value: 0.1296, missing_reason: null },
        benchmark_mean_loss: { value: 0.3025, missing_reason: null },
        mean_loss_difference: { value: -0.1729, missing_reason: null },
        standard_error: { value: 0.04, missing_reason: null },
        t_statistic: { value: -4.32, missing_reason: null },
        p_value_one_sided: { value: 0.0002, missing_reason: null },
        confidence_interval_lower: { value: -0.24, missing_reason: null },
        confidence_interval_upper: { value: -0.09, missing_reason: null },
        observations: 64,
        indicators: 1,
        status: "Available",
        conclusion: "PLS-SEM has lower loss",
        reason: "",
      },
    }],
    bic_rows: [{
      id: "loyalty",
      outcome: "Loyalty",
      definition: "prediction_oriented_bic_v1",
      observations: 64,
      first_parameter_count: 2,
      second_parameter_count: 3,
      first_bic: -261.602,
      second_bic: -261.603,
      bic_change: -0.001,
      first_akaike_weight: 0.499875,
      second_akaike_weight: 0.500125,
      akaike_weight_source: "stored_exact",
      preferred: "second",
    }],
    issues: [{
      id: "cvpat_contract_mismatch",
      code: "cvpat_between_model_test_unavailable",
      severity: "information",
      title: "CVPAT rows are single-model benchmark assessments",
      message: "They are not a paired CVPAT test between the two models.",
      related_ids: [],
      technical_details: [],
    }],
  };
}

describe("PLS saved-run comparison panel v1", () => {
  it("renders nothing outside Experimental Labs", () => {
    const markup = renderToStaticMarkup(
      <PlsSavedRunComparisonPanelV1 state={{ status: "hidden" }} firstName="First" secondName="Second" />,
    );
    expect(markup).toBe("");
  });

  it("renders typed corrective issues as an accessible alert", () => {
    const markup = renderToStaticMarkup(
      <PlsSavedRunComparisonPanelV1
        state={{
          status: "blocked",
          issues: [{
            id: "dataset_mismatch",
            code: "dataset_mismatch",
            severity: "blocking",
            title: "Data differs",
            message: "Choose runs calculated from the same immutable dataset.",
            related_ids: [],
            technical_details: [],
          }],
        }}
        firstName="First"
        secondName="Second"
      />,
    );
    expect(markup).toContain('role="alert"');
    expect(markup).toContain("Data differs");
    expect(markup).toContain("Choose runs calculated from the same immutable dataset.");
    expect(markup).not.toContain("Experimental feature");
  });

  it("renders one Experimental chip and accessible tables without implying paired CVPAT", () => {
    const markup = renderToStaticMarkup(
      <PlsSavedRunComparisonPanelV1
        state={{ status: "ready", comparison: comparisonFixture() }}
        firstName="Model A"
        secondName="Model B"
      />,
    );
    expect(markup.match(/Experimental/g)).toHaveLength(2); // Visible text plus its accessible label.
    expect(markup).toContain("Indicator-level PLSpredict metrics");
    expect(markup).toContain("Stored CVPAT benchmark assessments for each model");
    expect(markup).toContain("Prediction-oriented BIC and exact stored Akaike weights");
    expect(markup).toContain('scope="row"');
    expect(markup).toContain('scope="col"');
    expect(markup).toContain("does not refit either model or run a paired CVPAT test between them");
    expect(markup).toContain("Model B has lower BIC");
    expect(markup).toContain("50.01%");
  });
});
