import { describe, expect, it } from "vitest";
import { methodResultTables, runExportTables, tablesToCsv, tablesToHtml } from "./resultTables";
import type { AnalysisRun, PlsResult } from "../types";

const result: PlsResult = {
  method_version: "wpls_case_weighted_v1",
  converged: true,
  iterations: 4,
  used_observations: 120,
  omitted_observations: 0,
  outer_estimates: [],
  paths: [],
  effects: [],
  r_squared: {},
  warnings: [],
  wpls: {
    method_version: "wpls_case_weighted_v1",
    case_weight_column: "WEIGHT",
    weight_sum: 135.25,
    effective_sample_size: 111.125,
    covariance: "weighted_sample_covariance",
    warnings: ["WPLS is experimental."],
  },
  cca: {
    method_version: "cca_composite_residual_v1",
    model: "recursive_composite",
    max_absolute_residual: 0.01234567,
    warnings: [],
    correlations: [{
      left: "A",
      right: "B",
      observed: 0.4,
      reproduced: 0.35,
      residual: 0.05,
      absolute_residual: 0.05,
    }],
  },
  cta_pls: {
    method_version: "cta_pls_tetrad_v1",
    covariance: "sample_covariance",
    warnings: [],
    max_absolute_tetrad_by_construct: { A: 0.002 },
    estimates: [{
      construct: "A",
      indicator_a: "a1",
      indicator_b: "a2",
      indicator_c: "a3",
      indicator_d: "a4",
      pairing: "ab_cd",
      tetrad: -0.002,
      absolute_tetrad: 0.002,
    }],
  },
  predict: {
    method_version: "plspredict_holdout_v1",
    split: "deterministic_complete_case_modulo_4_test_rows",
    training_observations: 24,
    test_observations: 8,
    benchmark: "training-mean construct-score benchmark fixed at zero",
    warnings: ["PLSpredict is experimental."],
    targets: [{
      construct: "B",
      predictor_count: 1,
      rmse_pls: 0.1234567,
      mae_pls: 0.101,
      rmse_benchmark: 0.8,
      mae_benchmark: 0.7,
      q_squared_predict: 0.97654321,
      rmse_lm: 0.11,
      mae_lm: 0.09,
      q_squared_predict_lm: 0.98,
    }],
    repeated_kfold: {
      method_version: "plspredict_repeated_kfold_v1",
      folds: 5,
      repeats: 3,
      assignment: "deterministic_complete_case_index_multiplier_modulo_5",
      total_test_observations: 96,
      warnings: ["Repeated k-fold is experimental."],
      cvpat: [{
        target: "B",
        comparison: "pls_vs_lm_benchmark",
        loss: "squared_error_difference_pls_minus_comparison",
        mean_loss_difference: 0.001,
        standard_error: 0.0005,
        t_statistic: 2,
        p_value_two_sided: 0.049,
        observations: 96,
        preferred_model: "lm_benchmark",
        warning: null,
      }],
      targets: [{
        construct: "B",
        predictor_count: 1,
        rmse_pls: 0.13,
        mae_pls: 0.10,
        rmse_benchmark: 0.82,
        mae_benchmark: 0.72,
        q_squared_predict: 0.97,
        rmse_lm: 0.12,
        mae_lm: 0.095,
        q_squared_predict_lm: 0.979,
      }],
    },
  },
  segmentation: {
    method_version: "pls_pos_bounded_v1",
    algorithm: "deterministic_two_segment_multi_path_alignment_sse_scan",
    requested_segments: 2,
    selected_segments: 2,
    assignment: "sort source_score*target_score ascending; split at rank 96 of 192",
    observations: 192,
    objective: 10,
    pooled_objective: 100,
    objective_improvement: 0.9,
    min_segment_share: 0.5,
    segment_size_imbalance: 0,
    max_path_separation: 1.85,
    warnings: ["Bounded PLS-POS is experimental."],
    segments: [{
      segment: "segment_1_low_alignment",
      observations: 96,
      share: 0.5,
      paths: [{ source: "A", target: "B", coefficient: -0.75 }],
      r_squared: { B: 0.88 },
    }, {
      segment: "segment_2_high_alignment",
      observations: 96,
      share: 0.5,
      paths: [{ source: "A", target: "B", coefficient: 1.1 }],
      r_squared: { B: 0.91 },
    }],
    memberships: [
      { observation: 0, segment: "segment_1_low_alignment" },
      { observation: 1, segment: "segment_2_high_alignment" },
    ],
  },
  mga: {
    method_version: "pls_mga_two_group_v1",
    group_column: "group",
    warnings: ["Bounded two-group MGA is experimental."],
    groups: [{
      group: "A",
      observations: 60,
      paths: [{ source: "A", target: "B", coefficient: 0.7 }],
      r_squared: { B: 0.49 },
    }, {
      group: "B",
      observations: 60,
      paths: [{ source: "A", target: "B", coefficient: -0.2 }],
      r_squared: { B: 0.04 },
    }],
    comparisons: [{
      source: "A",
      target: "B",
      group_a: "A",
      group_b: "B",
      coefficient_a: 0.7,
      coefficient_b: -0.2,
      difference: 0.9,
      standard_error: 0.1,
      t_statistic: 9,
      p_value_two_sided: 0.00001,
      warning: null,
    }],
  },
  ipma: {
    method_version: "ipma_v1",
    performance_scale: "min_max_0_100_from_standardized_scores_v1",
    targets: ["B"],
    warnings: ["IPMA is experimental."],
    constructs: [{
      target: "B",
      construct: "A",
      importance: 0.7,
      performance: 52.5,
      score_mean: 0,
    }],
    indicators: [{
      target: "B",
      construct: "A",
      indicator: "a1",
      construct_importance: 0.7,
      loading: 0.92,
      performance: 51.5,
      score_mean: 0,
    }],
  },
};

describe("result export tables", () => {
  it("builds watermarked method-specific v0.5 tables", () => {
    const tables = methodResultTables(result);
    expect(tables.map((table) => table.id)).toEqual([
      "wpls_weights",
      "cca_residuals",
      "cca_summary",
      "plspredict_holdout",
      "plspredict_split",
      "plspredict_repeated_kfold",
      "plspredict_repeated_kfold_plan",
      "cvpat",
      "cta_pls_summary",
      "cta_pls_tetrads",
      "segmentation_summary",
      "segmentation_segments",
      "segmentation_memberships",
      "mga_summary",
      "mga_paths",
      "mga_comparisons",
      "ipma_constructs",
      "ipma_indicators",
    ]);
    expect(tables.every((table) => table.status === "experimental")).toBe(true);
    expect(tables[0].warning).toContain("Validated for the documented QuickPLS v1.0.0 supported scope");
    expect(tables[0].rows[0]).toEqual(["WEIGHT", "135.250000", "111.1250", "weighted sample covariance"]);
  });

  it("exports run provenance plus escaped CSV and HTML tables", () => {
    const run: AnalysisRun = {
      id: "run-1",
      name: "Weighted run",
      method: "Weighted PLS",
      createdAt: "2026-07-19T00:00:00.000Z",
      seed: 7,
      status: "completed",
      warnings: [],
      fingerprint: "abc123",
      result,
    };
    const tables = runExportTables(run);
    expect(tables[0].id).toBe("run_provenance");
    expect(tables[0].status).toBe("experimental");
    const csv = tablesToCsv(tables);
    expect(csv).toContain("WPLS case-weight metadata");
    expect(csv).toContain("PLSpredict holdout metrics");
    expect(csv).toContain("CVPAT paired loss comparisons");
    expect(csv).toContain("PLS-POS bounded segmentation summary");
    expect(csv).toContain("PLS-POS bounded segment memberships");
    expect(csv).toContain("MGA path comparisons");
    expect(csv).toContain("IPMA construct importance-performance");
    expect(csv).toContain("weighted sample covariance");
    const html = tablesToHtml(tables);
    expect(html).toContain("<title>QuickPLS export</title>");
    expect(html).toContain("Validated for the documented QuickPLS v1.0.0 supported scope");
  });
});
