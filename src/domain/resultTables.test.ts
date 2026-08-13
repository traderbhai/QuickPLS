import { describe, expect, it } from "vitest";
import { methodResultTables, resultScopeStatus, runExportTables, tablesToCsv, tablesToHtml } from "./resultTables";
import type { AnalysisRun, PlsResult } from "../types";
import { processV2Run } from "../native/nativeProcessTestFixture";

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
    method_version: "pls_pos_v1",
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
  it("uses graph equation tables when the PROCESS v2 generic fit shell is null", () => {
    const processResult = processV2Run().result!;
    const tables = methodResultTables(processResult);
    expect(tables.map((table) => table.id)).toEqual([
      "process_model_summary",
      "process_equation_coefficients",
    ]);
    expect(JSON.stringify(tables)).not.toContain("N/A");
    expect(tables.find((table) => table.id === "process_equation_coefficients")?.rows).toHaveLength(4);
  });

  it("routes completed PROCESS runs through the complete canonical report/export table set", () => {
    const run = processV2Run(true);
    const tables = runExportTables(run);
    expect(tables.map((table) => table.id)).toEqual([
      "run_provenance",
      "process_model_summary",
      "process_paths",
      "process_equation_coefficients",
      "process_equation_fit",
      "process_reference_effects",
      "process_simple_slopes",
      "process_conditional_plot_points",
      "process_johnson_neyman",
      "process_johnson_neyman_curve_points",
      "process_bootstrap_summary",
      "process_bootstrap_failures",
      "process_bootstrap_inference",
      "process_bootstrap_bca",
      "process_scope",
    ]);
    expect(tables.every((table) => table.status === "experimental")).toBe(true);
    expect(tables.find((table) => table.id === "process_reference_effects")?.columns)
      .toContain("Reference condition");
    const csv = tablesToCsv(tables);
    const html = tablesToHtml(tables);
    for (const expected of [
      "Reference condition",
      "Conditional outcome plot data",
      "Johnson-Neyman curve data",
      "Bootstrap inference",
    ]) {
      expect(csv).toContain(expected);
      expect(html).toContain(expected);
    }

    const failed = { ...run, status: "failed" as const };
    expect(runExportTables(failed)).toEqual([]);
  });

  it("fails closed for unknown PROCESS versions and models", () => {
    for (const withBootstrap of [false, true]) {
      expect(methodResultTables(processV2Run(withBootstrap).result!).every(
        (table) => table.status === "experimental",
      )).toBe(true);
    }
    const futureRegression = structuredClone(processV2Run().result!);
    futureRegression.regression!.method_version = "regression_process_v3";
    expect(methodResultTables(futureRegression).every((table) => table.status === "experimental")).toBe(true);

    const futureProcess = structuredClone(processV2Run().result!);
    futureProcess.regression!.process!.method_version = "regression_process_v3";
    expect(methodResultTables(futureProcess).every((table) => table.status === "experimental")).toBe(true);

    const unknownModel = structuredClone(processV2Run().result!);
    unknownModel.regression!.process!.model = "future_graph";
    expect(methodResultTables(unknownModel).every((table) => table.status === "experimental")).toBe(true);

    const futureEnvelope = structuredClone(processV2Run().result!);
    futureEnvelope.method_version = "regression_process_v3";
    expect(methodResultTables(futureEnvelope).every((table) => table.status === "experimental")).toBe(true);
  });

  it("builds method-specific tables and keeps PLS-POS as a bounded preview", () => {
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
    expect(tables.some((table) => table.status === "validated")).toBe(true);
    expect(tables.filter((table) => table.id.startsWith("segmentation_")).every((table) => table.status === "experimental")).toBe(true);
    expect(tables.find((table) => table.id === "segmentation_summary")?.warning).toContain("does not claim unrestricted published PLS-POS equivalence");
    expect(tables.find((table) => table.id === "wpls_weights")?.status).toBe("validated");
    expect(tables.find((table) => table.id === "plspredict_holdout")?.status).toBe("validated");
    expect(tables.find((table) => table.id === "ipma_constructs")?.status).toBe("validated");
    expect(tables.find((table) => table.id === "cca_residuals")?.status).toBe("validated");
    expect(tables[0].warning).toContain("Validated for the documented QuickPLS supported scope");
    expect(tables[0].rows[0]).toEqual(["WEIGHT", "135.250000", "111.1250", "weighted sample covariance"]);
  });

  it("marks FIMIX-style output experimental and discloses its non-EM boundary", () => {
    const fimixResult: PlsResult = {
      ...result,
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      fimix: {
        method_version: "fimix_pls_v1",
        classes: 2,
        starts: 10,
        iterations: 4,
        log_likelihood: -12,
        aic: 30,
        bic: 33,
        caic: 35,
        entropy: 0.6,
        classes_summary: [{
          class: "class_1",
          observations: 20,
          share: 1,
          paths: [{ source: "A", target: "B", coefficient: 0.5 }],
          r_squared: { B: 0.25 },
        }],
        memberships: [{ observation: 0, class: "class_1", probability: 1 }],
        warnings: [],
      },
    };
    const fimixTables = methodResultTables(fimixResult);

    expect(fimixTables.map((table) => table.id)).toEqual(["fimix_summary", "fimix_paths"]);
    expect(fimixTables.every((table) => table.status === "experimental")).toBe(true);
    expect(fimixTables.every((table) => table.warning?.includes("not finite-mixture posterior probabilities"))).toBe(true);
    expect(resultScopeStatus(fimixResult)).toBe("experimental");
  });

  it("marks promoted PCA, OLS, logistic, and historical bounded PROCESS v1 tables as validated", () => {
    const pcaTables = methodResultTables({
      ...result,
      method_version: "pca_v1",
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      pca: {
        method_version: "pca_v1",
        component_rule: "fixed",
        retained_components: 1,
        observations: 4,
        variables: ["x1", "x2"],
        components: [{ component: "PC1", eigenvalue: 1.8, explained_variance: 0.9, cumulative_variance: 0.9 }],
        loadings: [{ variable: "x1", component: "PC1", loading: 0.95, weight: 0.71 }],
        scores: [{ observation: 0, component: "PC1", score: 1.2 }],
        warnings: ["Standalone PCA v1 is validated for the documented QuickPLS v1.2 supported scope."],
      },
    });
    expect(pcaTables.every((table) => table.status === "validated")).toBe(true);
    expect(pcaTables[0].warning).toContain("Validated for the documented QuickPLS supported scope");

    const olsTables = methodResultTables({
      ...result,
      method_version: "regression_ols_v1",
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      regression: {
        method_version: "regression_ols_v1",
        regression_type: "ols",
        outcome: "y",
        predictors: ["x"],
        controls: [],
        observations: 10,
        coefficients: [{ term: "x", estimate: 2, standard_error: 0.1, statistic: 20, p_value_two_sided: 0.00001, confidence_interval_lower: 1.8, confidence_interval_upper: 2.2, odds_ratio: null }],
        fit: { r_squared: 0.8, adjusted_r_squared: 0.78, aic: 12, bic: 13 },
        predictions: [{ observation: 0, fitted: 1.5, residual: 0.1 }],
        process: null,
        warnings: ["OLS regression v1 is validated for the documented QuickPLS v1.2 OLS scope."],
      },
    });
    expect(olsTables.every((table) => table.status === "validated")).toBe(true);

    const logisticTables = methodResultTables({
      ...result,
      method_version: "regression_logistic_v2",
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      regression: {
        method_version: "regression_logistic_v2",
        regression_type: "logistic",
        outcome: "y",
        predictors: ["x"],
        controls: [],
        observations: 10,
        coefficients: [{ term: "x", estimate: 2, standard_error: 0.1, statistic: 20, p_value_two_sided: 0.00001, confidence_interval_lower: 1.8, confidence_interval_upper: 2.2, odds_ratio: 7.389 }],
        fit: { pseudo_r_squared: 0.8, aic: 12, bic: 13 },
        predictions: [{ observation: 0, fitted: 0.8, probability: 0.8 }],
        logistic: {
          outcome_profile: { outcome: "y", coding: "numeric_0_1_exact_v1", complete_cases: 10, omitted_cases: 0, zero_count: 5, one_count: 5, invalid_count: 0, prevalence: 0.5, readiness: "ready" },
          convergence: { algorithm: "deterministic_newton_irls_v1", converged: true, iterations: 5, max_iterations: 100, tolerance: 1e-8, final_max_abs_step: 1e-10, separation_probability_tolerance: 1e-9 },
          classification: { threshold: 0.5, true_positive: 4, true_negative: 4, false_positive: 1, false_negative: 1, accuracy: 0.8, sensitivity: 0.8, specificity: 0.8 },
        },
        process: null,
        warnings: ["Logistic regression v2 is validated for the documented binary numeric complete-case scope."],
      },
    });
    expect(logisticTables.every((table) => table.status === "validated")).toBe(true);

    const legacyLogisticTables = methodResultTables({
      ...result,
      method_version: "regression_logistic_v1",
      regression: {
        method_version: "regression_logistic_v1",
        regression_type: "logistic",
        outcome: "y",
        predictors: ["x"],
        controls: [],
        observations: 10,
        coefficients: [{ term: "x", estimate: 2, standard_error: 0.1, statistic: 20, p_value_two_sided: 0.00001, confidence_interval_lower: 1.8, confidence_interval_upper: 2.2, odds_ratio: 7.389 }],
        fit: { pseudo_r_squared: 0.8, aic: 12, bic: 13 },
        predictions: [{ observation: 0, fitted: 0.8, probability: 0.8 }],
        process: null,
        warnings: ["Historical logistic v1 archive."],
      },
    });
    expect(
      legacyLogisticTables
        .filter((table) => table.id === "regression_coefficients" || table.id === "regression_fit")
        .every((table) => table.status === "experimental"),
    ).toBe(true);

    const processTables = methodResultTables({
      ...result,
      method_version: "regression_process_v1",
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      regression: {
        method_version: "regression_process_v1",
        regression_type: "process",
        outcome: "y",
        predictors: ["x"],
        controls: [],
        observations: 10,
        coefficients: [{ term: "x", estimate: 2, standard_error: 0.1, statistic: 20, p_value_two_sided: 0.00001, confidence_interval_lower: 1.8, confidence_interval_upper: 2.2, odds_ratio: null }],
        fit: { r_squared: 0.8, adjusted_r_squared: 0.78, aic: 12, bic: 13 },
        predictions: [{ observation: 0, fitted: 1.5, residual: 0.1 }],
        process: { method_version: "regression_process_v1", model: "mediation", effects: [{ effect: "indirect", estimate: 0.2, lower_percentile: 0.1, upper_percentile: 0.3 }], simple_slopes: [], warnings: ["PROCESS v1 reports bounded deterministic mediation/moderation effects validated for the documented QuickPLS v1.2.2 scope."] },
        warnings: ["PROCESS-style regression v1 is validated for the documented QuickPLS v1.2.2 bounded mediation/moderation workflow scope."],
      },
    });
    expect(processTables.every((table) => table.status === "experimental")).toBe(true);

    const moderatedMediationTables = methodResultTables({
      ...result,
      method_version: "regression_process_v1",
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      regression: {
        method_version: "regression_process_v1",
        regression_type: "process",
        outcome: "y",
        predictors: ["x"],
        controls: [],
        observations: 10,
        coefficients: [{ term: "x", estimate: 2, standard_error: 0.1, statistic: 20, p_value_two_sided: 0.00001, confidence_interval_lower: 1.8, confidence_interval_upper: 2.2, odds_ratio: null }],
        fit: { r_squared: 0.8, adjusted_r_squared: 0.78, aic: 12, bic: 13 },
        predictions: [{ observation: 0, fitted: 1.5, residual: 0.1 }],
        process: { method_version: "regression_process_v1", model: "moderated_mediation", effects: [{ effect: "conditional_indirect", estimate: 0.2, lower_percentile: null, upper_percentile: null }], simple_slopes: [], warnings: ["Moderated mediation remains experimental."] },
        warnings: ["PROCESS moderated mediation remains experimental."],
      },
    });
    expect(moderatedMediationTables.every((table) => table.status === "experimental")).toBe(true);
  });

  it("reports only nca_v2 as current and distinguishes nullable bottleneck states without N/A", () => {
    const ncaV2: PlsResult = {
      ...result,
      method_version: "nca_v2",
      wpls: undefined,
      cca: undefined,
      cta_pls: undefined,
      predict: undefined,
      segmentation: undefined,
      mga: undefined,
      ipma: undefined,
      nca: {
        method_version: "nca_v2",
        ceiling: "both",
        permutation_samples: 19,
        usable_permutations: 19,
        x: "x",
        y: "y",
        observations: 4,
        scope: { minimum_x: 0, maximum_x: 3, minimum_y: 1, maximum_y: 4 },
        ce_fdh_peers: [{ x: 0, y: 1 }, { x: 1, y: 3 }, { x: 3, y: 4 }],
        ceilings: [
          { ceiling: "ce_fdh", effect_size: 5 / 9, permutation_p_value: 0.1, slope: null, intercept: null },
          { ceiling: "cr_fdh", effect_size: 36 / 91, permutation_p_value: 0.15, slope: 13 / 14, intercept: 10 / 7 },
        ],
        bottlenecks: [
          { ceiling: "ce_fdh", outcome_percent: 50, required_x_percent: 100 / 3, status: "required" },
          { ceiling: "cr_fdh", outcome_percent: 10, required_x_percent: null, status: "not_necessary" },
        ],
        warnings: ["NCA v2 bounded observed numeric X/Y scope."],
      },
    };

    const current = methodResultTables(ncaV2);
    expect(current).toHaveLength(2);
    expect(current.every((table) => table.status === "validated")).toBe(true);
    expect(current[0].columns).toEqual(["Ceiling", "Effect size", "Permutation p", "Slope", "Intercept"]);
    expect(current[0].rows[0].slice(-2)).toEqual(["", ""]);
    expect(current[1].rows[1]).toEqual(["cr fdh", "10.0", "", "Not necessary at this outcome level"]);
    expect(JSON.stringify(current)).not.toContain("N/A");

    const legacy = methodResultTables({
      ...ncaV2,
      method_version: "nca_v1",
      nca: { ...ncaV2.nca!, method_version: "nca_v1" },
    });
    expect(legacy.every((table) => table.status === "experimental")).toBe(true);
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
    expect(tables[0].warning).toContain("Experimental output");
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
    expect(html).toContain("Validated for the documented QuickPLS supported scope");
  });
});
