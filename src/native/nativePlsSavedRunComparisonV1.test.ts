import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun, PlsPredictErrorMetrics } from "../types";
import { nativePlsSavedRunComparisonV1 } from "./nativePlsSavedRunComparisonV1";

function metrics(rmse: number, mae: number): PlsPredictErrorMetrics {
  return {
    observations: 64,
    squared_error_sum: rmse * rmse * 64,
    absolute_error_sum: mae * 64,
    rmse,
    mae,
    absolute_percentage_error_sum: 6.4,
    mape_observations: 64,
    mape_percent: 10,
  };
}

function predictionRun(id: string, modelVariant: "direct" | "mediated", rmse: number): AnalysisRun {
  const source = completedSamplePlsRun();
  const result = structuredClone(source.result!);
  result.used_observations = 64;
  result.predict = {
    method_version: "plspredict_indicator_v2",
    split: "deterministic_complete_case_modulo_4_test_rows",
    training_observations: 48,
    test_observations: 16,
    benchmark: "indicator_average",
    targets: [{
      construct: "loyalty",
      predictor_count: 2,
      rmse_pls: rmse,
      mae_pls: 0.32,
      rmse_benchmark: 0.55,
      mae_benchmark: 0.44,
      q_squared_predict: 0.28,
      rmse_lm: 0.43,
      mae_lm: 0.34,
      q_squared_predict_lm: 0.25,
    }],
    indicator_targets: [{
      construct: "loyalty",
      indicator: "LOY1",
      predictor_scope: "earliest_antecedent_indicators",
      predictor_count: 2,
      pls: metrics(rmse, 0.32),
      indicator_average: metrics(0.55, 0.44),
      linear_model: { status: "available", metrics: metrics(0.43, 0.34), reason: null },
      q_squared_predict: 0.28,
    }],
    repeated_kfold: {
      method_version: "plspredict_repeated_kfold_indicator_v2",
      folds: 10,
      repeats: 10,
      assignment: "seeded_chacha20_balanced_folds",
      assignment_digest: `sha256:${"e".repeat(64)}`,
      seed: 20260815,
      total_test_observations: 640,
      targets: [{
        construct: "loyalty",
        predictor_count: 2,
        rmse_pls: rmse,
        mae_pls: 0.32,
        rmse_benchmark: 0.55,
        mae_benchmark: 0.44,
        q_squared_predict: 0.28,
        rmse_lm: 0.43,
        mae_lm: 0.34,
        q_squared_predict_lm: 0.25,
      }],
      indicator_targets: [{
        construct: "loyalty",
        indicator: "LOY1",
        predictor_scope: "earliest_antecedent_indicators",
        predictor_count: 2,
        pls: metrics(rmse, 0.32),
        indicator_average: metrics(0.55, 0.44),
        linear_model: { status: "available", metrics: metrics(0.43, 0.34), reason: null },
        q_squared_predict: 0.28,
      }],
      cvpat_benchmark_assessments: [
        {
          method_version: "cvpat_indicator_benchmarks_v2",
          comparison_kind: "benchmark_assessment",
          target_scope: "all_endogenous_indicators",
          benchmark: "indicator_average",
          loss: "mean_squared_error_across_indicators_per_observation",
          alternative: "pls_loss_less_than_benchmark",
          confidence_level: 0.95,
          mean_loss_pls: rmse * rmse,
          mean_loss_benchmark: 0.3025,
          mean_loss_difference: rmse * rmse - 0.3025,
          standard_error: 0.04,
          t_statistic: -3,
          p_value_one_sided: 0.002,
          confidence_interval_lower: -0.2,
          confidence_interval_upper: -0.04,
          observations: 64,
          indicator_count: 1,
          status: "available",
          preferred_model: "pls_sem",
          reason: null,
        },
        {
          method_version: "cvpat_indicator_benchmarks_v2",
          comparison_kind: "benchmark_assessment",
          target_scope: "all_endogenous_indicators",
          benchmark: "linear_model",
          loss: "mean_squared_error_across_indicators_per_observation",
          alternative: "pls_loss_less_than_benchmark",
          confidence_level: 0.95,
          mean_loss_pls: rmse * rmse,
          mean_loss_benchmark: 0.1849,
          mean_loss_difference: rmse * rmse - 0.1849,
          standard_error: 0.03,
          t_statistic: -0.5,
          p_value_one_sided: 0.31,
          confidence_interval_lower: -0.08,
          confidence_interval_upper: 0.05,
          observations: 64,
          indicator_count: 1,
          status: "available",
          preferred_model: "pls_sem",
          reason: null,
        },
      ],
      warnings: [],
    },
    warnings: [],
  };
  const directEdges = [
    { id: "competence-loyalty", source: "competence", target: "loyalty" },
  ];
  const mediatedEdges = [
    { id: "competence-satisfaction", source: "competence", target: "satisfaction" },
    { id: "satisfaction-loyalty", source: "satisfaction", target: "loyalty" },
  ];
  return {
    ...source,
    id,
    name: id,
    method: "PLSpredict / CVPAT",
    fingerprint: `sha256:${"a".repeat(64)}`,
    result,
    assessment: undefined,
    bootstrap: undefined,
    permutation: undefined,
    modelId: `model-${modelVariant}`,
    modelSnapshot: {
      nodes: [
        { id: "competence", position: { x: 0, y: 0 }, data: { label: "Competence", shortName: "COMP", mode: "reflective", indicators: ["COMP1", "COMP2"] } },
        { id: "satisfaction", position: { x: 100, y: 0 }, data: { label: "Satisfaction", shortName: "CUSA", mode: "reflective", indicators: ["CUSA1", "CUSA2"] } },
        { id: "loyalty", position: { x: 200, y: 0 }, data: { label: "Loyalty", shortName: "LOY", mode: "reflective", indicators: ["LOY1", "LOY2"] } },
      ],
      edges: modelVariant === "direct" ? directEdges : mediatedEdges,
    },
    provenance: {
      recipe_id: `recipe-${id}`,
      dataset_fingerprint: `sha256:${"a".repeat(64)}`,
      method: "predict",
      method_version: result.method_version,
      engine_version: "qpls-estimation-test",
      seed: 20260815,
      settings: {
        method: "predict",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20260815,
        workers: 1,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-15T10:00:00.000Z",
      completed_at: "2026-08-15T10:00:01.000Z",
    },
  };
}

describe("native PLS saved-run comparison v1", () => {
  it("is completely hidden when Experimental Labs is disabled", async () => {
    const first = predictionRun("first", "direct", 0.41);
    const built = await nativePlsSavedRunComparisonV1(first, first, { experimentalLabsEnabled: false });
    expect(built).toEqual({ status: "hidden" });
  });

  it("adapts two distinct compatible native runs through canonical documents", async () => {
    const built = await nativePlsSavedRunComparisonV1(
      predictionRun("first", "direct", 0.41),
      predictionRun("second", "mediated", 0.36),
      { experimentalLabsEnabled: true },
    );
    expect(built.status, JSON.stringify(built, null, 2)).toBe("ready");
    if (built.status !== "ready") return;
    expect(built.comparison.prediction_rows[0].metrics).toContainEqual(expect.objectContaining({
      id: "pls_rmse",
      first: { value: 0.41, missing_reason: null },
      second: { value: 0.36, missing_reason: null },
    }));
    expect(built.comparison.compatibility.first_model_digest).not.toBe(built.comparison.compatibility.second_model_digest);
  });
});
