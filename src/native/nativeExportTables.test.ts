import { describe, expect, it } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun } from "../types";
import { NATIVE_LOGISTIC_ENGINE_SCOPE_WARNING } from "./nativeLogistic";
import { NATIVE_NCA_ENGINE_SCOPE_WARNING, NATIVE_STANDALONE_ASSESSMENT_WARNING } from "./nativeNca";
import { NATIVE_PCA_ENGINE_SCOPE_WARNING } from "./nativePca";
import { completedCbsemRun } from "./nativeCbsem.testFixture";
import { completedGscaRun } from "./nativeGsca.testFixture";
import {
  nativeNcaCeFdhPeerExportTable,
  nativePcaScoreExportTable,
  nativeRunProvenanceTable,
  nativeRunSettingApplicability,
} from "./nativeExportTables";

function completedNcaRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    modelId: null,
    modelSnapshot: undefined,
    method: "Necessary Condition Analysis",
    seed: 20_260_811,
    bootstrap: undefined,
    permutation: undefined,
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    provenance: {
      recipe_id: "recipe-nca",
      dataset_fingerprint: "sha256:nca-fixture",
      method: "nca",
      method_version: "nca_v2",
      engine_version: "2.45.0",
      seed: 20_260_811,
      settings: {
        method: "nca",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "unstandardized",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20_260_811,
        workers: 1,
        confidence_level: 0.95,
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-11T08:00:00.000Z",
      completed_at: "2026-08-11T08:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "nca_v2",
      iterations: 0,
      used_observations: 8,
      nca: {
        method_version: "nca_v2",
        ceiling: "both",
        permutation_samples: 19,
        usable_permutations: 19,
        x: "condition",
        y: "outcome",
        observations: 8,
        scope: { minimum_x: 1, maximum_x: 8, minimum_y: 1, maximum_y: 9 },
        ce_fdh_peers: [{ x: 1, y: 1 }, { x: 8, y: 9 }],
        ceilings: [
          { ceiling: "ce_fdh", effect_size: 0.3, permutation_p_value: 0.05, slope: null, intercept: null },
          { ceiling: "cr_fdh", effect_size: 0.2, permutation_p_value: 0.1, slope: 1, intercept: 0 },
        ],
        bottlenecks: ["ce_fdh", "cr_fdh"].flatMap((ceiling) => [10, 20, 30, 40, 50, 60, 70, 80, 90].map((outcome) => ({
          ceiling,
          outcome_percent: outcome,
          required_x_percent: outcome / 2,
          status: "required" as const,
        }))),
        warnings: [NATIVE_NCA_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

function completedPcaRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  return {
    ...base,
    modelId: null,
    modelSnapshot: undefined,
    method: "Principal Component Analysis",
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "recipe-pca",
      dataset_fingerprint: "sha256:pca-fixture",
      method: "pca",
      method_version: "pca_v1",
      engine_version: "2.45.0",
      seed: 7,
      settings: {
        method: "pca",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "standardized",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 7,
        workers: 1,
        confidence_level: 0.95,
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-12T01:00:00.000Z",
      completed_at: "2026-08-12T01:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "pca_v1",
      used_observations: 3,
      pca: {
        method_version: "pca_v1",
        component_rule: "fixed",
        retained_components: 1,
        observations: 3,
        variables: ["a", "b"],
        components: [{ component: "PC1", eigenvalue: 2, explained_variance: 1, cumulative_variance: 1 }],
        loadings: [
          { variable: "a", component: "PC1", loading: 1, weight: Math.SQRT1_2 },
          { variable: "b", component: "PC1", loading: 1, weight: Math.SQRT1_2 },
        ],
        scores: [
          { observation: 0, component: "PC1", score: -Number.EPSILON },
          { observation: 1, component: "PC1", score: 0.25 },
          { observation: 2, component: "PC1", score: 1.25 },
        ],
        warnings: [NATIVE_PCA_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

function completedLogisticRun(): AnalysisRun {
  const base = completedSamplePlsRun();
  const outcomes = [0, 0, 0, 1, 1, 1];
  const probabilities = [0.2, 0.4, 0.6, 0.4, 0.7, 0.8];
  const logLikelihood = outcomes.reduce((sum, outcome, index) => {
    const probability = probabilities[index];
    return sum + (outcome ? Math.log(probability) : Math.log(1 - probability));
  }, 0);
  const nullLogLikelihood = outcomes.length * Math.log(0.5);
  const deviance = -2 * logLikelihood;
  const nullDeviance = -2 * nullLogLikelihood;
  const coefficient = (
    term: string,
    estimate: number,
    standardError: number,
    lower: number,
    upper: number,
    pValue: number,
  ) => ({
    term,
    estimate,
    standard_error: standardError,
    statistic: estimate / standardError,
    p_value_two_sided: pValue,
    confidence_interval_lower: lower,
    confidence_interval_upper: upper,
    odds_ratio: Math.exp(estimate),
    odds_ratio_confidence_interval_lower: Math.exp(lower),
    odds_ratio_confidence_interval_upper: Math.exp(upper),
  });
  return {
    ...base,
    id: "logistic-v2-result",
    modelId: null,
    modelSnapshot: undefined,
    name: "Binary Logistic Regression run",
    method: "Binary Logistic Regression",
    assessment: {
      method_version: "assessment_not_applicable_v1",
      warnings: [NATIVE_STANDALONE_ASSESSMENT_WARNING],
    } as NonNullable<AnalysisRun["assessment"]>,
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "recipe-logistic-v2",
      dataset_fingerprint: "sha256:logistic-fixture",
      method: "regression",
      method_version: "regression_logistic_v2",
      engine_version: "2.46.0",
      seed: 7,
      settings: {
        method: "regression",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "unstandardized",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 7,
        workers: 1,
        confidence_level: 0.95,
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-12T01:00:00.000Z",
      completed_at: "2026-08-12T01:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: "regression_logistic_v2",
      used_observations: 6,
      omitted_observations: 2,
      regression: {
        method_version: "regression_logistic_v2",
        regression_type: "logistic",
        outcome: "converted",
        predictors: ["score"],
        controls: [],
        observations: 6,
        coefficients: [
          coefficient("intercept", -0.5, 0.25, -0.9899909961350135, -0.010009003864986488, 0.04550026389635843),
          coefficient("score", 0.8, 0.2, 0.40800720309198923, 1.1919927969080109, 0.00006334248366623993),
        ],
        fit: {
          r_squared: null,
          adjusted_r_squared: null,
          f_statistic: null,
          log_likelihood: logLikelihood,
          pseudo_r_squared: 1 - logLikelihood / nullLogLikelihood,
          aic: deviance + 4,
          bic: deviance + Math.log(6) * 2,
          rmse: null,
          null_log_likelihood: nullLogLikelihood,
          deviance,
          null_deviance: nullDeviance,
          likelihood_ratio_chi_square: nullDeviance - deviance,
          likelihood_ratio_degrees_of_freedom: 1,
          likelihood_ratio_p_value: 0.15472608195962645,
          pseudo_r_squared_method: "mcfadden_v1",
        },
        predictions: probabilities.map((probability, observation) => ({
          observation,
          fitted: probability,
          probability,
          residual: outcomes[observation] - probability,
        })),
        logistic: {
          outcome_profile: {
            outcome: "converted",
            coding: "numeric_0_1_exact_v1",
            complete_cases: 6,
            omitted_cases: 2,
            zero_count: 3,
            one_count: 3,
            invalid_count: 0,
            prevalence: 0.5,
            readiness: "ready",
          },
          convergence: {
            algorithm: "deterministic_newton_irls_v1",
            converged: true,
            iterations: 5,
            max_iterations: 100,
            tolerance: 1e-8,
            final_max_abs_step: 1e-9,
            separation_probability_tolerance: 1e-9,
          },
          classification: {
            threshold: 0.5,
            true_positive: 2,
            true_negative: 2,
            false_positive: 1,
            false_negative: 1,
            accuracy: 4 / 6,
            sensitivity: 2 / 3,
            specificity: 2 / 3,
          },
        },
        process: null,
        warnings: [NATIVE_LOGISTIC_ENGINE_SCOPE_WARNING],
      },
    },
  };
}

describe("native export provenance", () => {
  it("exports exact GSCA ALS v2 provenance without PLS or inference settings", () => {
    const run = completedGscaRun();
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    const table = nativeRunProvenanceTable(run);
    const fields = table.rows.map(([field]) => field);
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Method version", "gsca_als_v2"],
      ["Estimator", "Joint global least-squares alternating least squares"],
      ["Algorithm version", "alternating_least_squares_v1"],
      ["Analyzed observations", "140"],
      ["Converged", "Yes"],
      ["ALS iterations", "4"],
      ["Initialization", "Deterministic +1 block weights"],
      ["Inference", "Point estimates only"],
    ]));
    expect(fields).not.toEqual(expect.arrayContaining([
      "Seed",
      "Weighting scheme",
      "Confidence level",
      "Workers",
      "Bootstrap samples",
      "Permutation samples",
    ]));
    expect(table.rows.flat().join(" ")).not.toMatch(/\bN\/?A\b|percentile interval/i);
  });

  it("exports exact standalone logistic provenance without SEM-only settings", () => {
    const run = completedLogisticRun();
    const immutableRun = structuredClone(run);
    const table = nativeRunProvenanceTable(run);
    const fields = table.rows.map(([field]) => field);

    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: true,
      usesWorkers: false,
    });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Method version", "regression_logistic_v2"],
      ["Outcome", "converted"],
      ["Predictors", "score"],
      ["Controls", "None"],
      ["Analyzed observations", "6"],
      ["Estimator", "Binary logistic maximum likelihood with intercept"],
      ["Algorithm", "Deterministic Newton IRLS"],
      ["Converged", "Yes"],
      ["Optimizer iterations", "5"],
      ["Outcome coding", "Numeric 0/1 (exact)"],
      ["Classification threshold", "0.5"],
      ["Coefficient inference", "Maximum-likelihood SE; Wald z; two-sided 95% confidence intervals"],
      ["Pseudo-R-squared", "McFadden"],
      ["Preprocessing", "unstandardized"],
      ["Missing data", "listwise deletion"],
      ["Confidence level", "0.95"],
    ]));
    for (const field of [
      "Seed",
      "Weighting scheme",
      "Maximum iterations",
      "Stop criterion",
      "Workers",
      "Bootstrap samples",
      "Permutation samples",
      "Model type",
      "Identification",
      "Mean structure",
    ]) expect(fields).not.toContain(field);
    expect(table.rows.flat().join(" ")).not.toMatch(/\bN\/?A\b|Experimental Labs|CB-SEM/i);
    expect(run).toEqual(immutableRun);
  });

  it("exports bounded CB-SEM/CFA optimizer, identification, and method-version provenance without resampling fields", () => {
    const run = completedCbsemRun("cfa");
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    const table = nativeRunProvenanceTable(run);
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Model type", "Confirmatory factor analysis"],
      ["Estimator", "Maximum likelihood"],
      ["Analyzed observations", "120"],
      ["Identification", "First loading fixed to 1 for each latent factor"],
      ["Estimator method version", "cfa_ml_v1"],
      ["Fit method version", "cbsem_fit_v1"],
      ["Modification-diagnostic version", "cbsem_modification_indices_v1"],
    ]));
    expect(table.rows.map(([field]) => field)).not.toEqual(expect.arrayContaining(["Seed", "Workers", "Confidence level", "Bootstrap samples", "Permutation samples"]));
  });

  it("exports the attributed 90% RMSEA interval method and both bounds", () => {
    const run = completedCbsemRun("cfa");
    run.result!.cbsem!.fit.rmsea_interval_attribution = {
      method_version: "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1",
      confidence_level: 0.9,
    };
    expect(nativeRunProvenanceTable(run).rows).toEqual(expect.arrayContaining([
      ["RMSEA interval method", "Noncentral chi-square inversion (N - 1 denominator)"],
      ["RMSEA interval method version", "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1"],
      ["RMSEA interval confidence", "90.0%"],
      ["RMSEA interval lower bound", "0.021"],
      ["RMSEA interval upper bound", "0.121"],
    ]));
  });

  it("exports genuine CFA score\/LM provenance without relabeling heuristic diagnostics", () => {
    const run = completedCbsemRun("cfa");
    run.result!.cbsem!.score_lm = {
      method_version: "cbsem_cfa_score_lm_v1",
      scope: "covariance_only_declared_zero_residual_covariances",
      rows: [{
        parameter_id: "parameter:residual_covariance:x1:x2",
        kind: "residual_covariance",
        lhs: "x1",
        rhs: "x2",
        outcome: { status: "available", score: 0, efficient_score: 0, candidate_information: 1, efficient_information: 1, modification_index: 0, expected_parameter_change: 0, p_value: 1 },
      }],
    };
    run.result!.cbsem!.modification_indices = [];
    run.provenance!.method_version = run.provenance!.method_version
      .replace("cbsem_modification_indices_v1", "cbsem_cfa_score_lm_v1");
    expect(nativeRunProvenanceTable(run).rows).toEqual(expect.arrayContaining([
      ["Score/LM method version", "cbsem_cfa_score_lm_v1"],
      ["Score/LM scope", "Covariance-only CFA; explicitly declared zero residual covariances"],
      ["Score/LM candidate count", "1"],
      ["Score/LM available tests", "1"],
      ["Score/LM unavailable tests", "0"],
    ]));
    expect(nativeRunProvenanceTable(run).rows).not.toContainEqual([
      "Modification-diagnostic version", "cbsem_modification_indices_v1",
    ]);
  });

  it("exports truthful exact-CFA bootstrap provenance and labels a 500-draw pilot unavailable", () => {
    const run = completedCbsemRun("cfa");
    const analysis = run.result!.cbsem!;
    analysis.fit.rmsea_interval_attribution = {
      method_version: "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1", confidence_level: 0.9,
    };
    analysis.score_lm = {
      method_version: "cbsem_cfa_score_lm_v1", scope: "covariance_only_declared_zero_residual_covariances",
      rows: [{ parameter_id: "parameter:residual_covariance:x1:x2", kind: "residual_covariance", lhs: "x1", rhs: "x2",
        outcome: { status: "available", score: 0, efficient_score: 0, candidate_information: 1, efficient_information: 1, modification_index: 0, expected_parameter_change: 0, p_value: 1 } }],
    };
    analysis.modification_indices = [];
    analysis.exact_case_bootstrap = {
      method_version: "cbsem_exact_case_bootstrap_v1", estimator_method_version: "cbsem_ml_exact_parameter_table_v3",
      source_dataset_id: "dataset-1", source_dataset_fingerprint: run.provenance!.dataset_fingerprint,
      outer_recipe_analytical_identity_sha256: "1".repeat(64), base_point_result_sha256: "2".repeat(64),
      compiler_analytical_identity_sha256: "3".repeat(64), plan_sha256: "4".repeat(64), model_scientific_sha256: "5".repeat(64),
      complete_case_sample_size: analysis.sample_size,
      complete_case_universe_digest_method: "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1",
      complete_case_universe_sha256: "6".repeat(64), covariance_denominator: "maximum_likelihood_n",
      sample_indices_digest_method: "sha256_source_fingerprint_and_ordered_u64_indices_v1",
      sampling_positions_digest_method: "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1",
      interval_method: "percentile_type7_v1", confidence_level: 0.95, requested_replicates: 500,
      attempted_refits: 500, usable_replicates: 0, failed_replicates: 500,
      minimum_usable_fraction: 0.9, minimum_usable_replicates: 1000, seed: run.provenance!.seed,
      stream_token: "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1",
      retry_policy: "no_retry_fixed_preplanned_primary_draws_v1", max_attempts_per_replicate: 1,
      parameter_ids: ["parameter:loading:x:x1"],
      inference: { status: "unavailable", reason_code: "insufficient_usable_refits", message: "Pilot is below the frozen threshold." },
      intervals: [],
      hypothesis_tests: {
        method_version: "cbsem_exact_case_bootstrap_null_centered_test_tail_v1",
        null_hypothesis: "compiled_free_parameter_equals_zero_v1",
        statistic: "unstudentized_null_centered_parameter_estimate_v1",
        tie_policy: "inclusive_ieee_comparison_v1",
        probability_method: "plus_one_over_usable_plus_one_v1",
        decision_rule: "selected_p_value_less_than_or_equal_alpha_v1",
        selected_test_tail: "one_sided_greater", null_value: 0, significance_level: 0.05,
        usable_replicates: 0,
        inference: { status: "unavailable", reason_code: "insufficient_usable_refits", message: "Pilot is below the frozen threshold." },
        parameters: [{
          parameter_id: "parameter:loading:x:x1",
          outcome: { status: "unavailable", reason: "insufficient_usable_replicates" },
        }],
      },
      successful_refits: [],
      failed_refits: Array.from({ length: 500 }, (_, replicate_index) => ({
        replicate_index, sampling_positions_sha256: "7".repeat(64), sample_indices_sha256: "8".repeat(64),
        kind: "non_convergence" as const, message: "Did not converge.",
      })),
    };
    run.provenance!.method_version = run.provenance!.method_version
      .replace("cbsem_modification_indices_v1", "cbsem_cfa_score_lm_v1+cbsem_exact_case_bootstrap_v1");
    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: true, usesConfidenceLevel: true, usesWorkers: true });
    expect(nativeRunProvenanceTable(run).rows).toEqual(expect.arrayContaining([
      ["Exact CB-SEM bootstrap inference", "Unavailable — 500-draw pilot is below the frozen 1,000-usable-refit minimum"],
      ["Exact CB-SEM bootstrap interval", "Percentile Type-7; sample-SD standard errors"],
      ["Exact CB-SEM bootstrap failure policy", "Failed fits retained; no retry or replacement draw"],
      ["Exact CB-SEM archive validation scope", expect.stringContaining("Rust schedule were not replayed")],
      ["Exact CB-SEM zero-null selection", "One-sided: parameter is greater than zero"],
      ["Exact CB-SEM zero-null inference", "Unavailable — insufficient usable exact refits"],
      ["Exact CB-SEM zero-null usable refits", "0"],
      ["Exact CB-SEM interval relationship", expect.stringContaining("not reinterpreted")],
    ]));

    const base = analysis.exact_case_bootstrap!;
    delete analysis.exact_case_bootstrap;
    analysis.exact_case_bootstrap_studentized = {
      base,
      studentized: {
        method_version: "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1",
        standard_error_method_version: "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
        expected_information_method: "cbsem_ml_expected_information_delta_method_v1",
        pivot_method: "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1",
        quantile_method: "percentile_type7_v1",
        interval_method: "reversed_type7_studentized_pivot_v1",
        archive_validation_scope: "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1",
        confidence_level: 0.95,
        minimum_usable_fraction: 0.9,
        minimum_usable_replicates: 1000,
        studentized_usable_replicates: 0,
        parameter_ids: ["parameter:loading:x:x1"],
        point_standard_errors: {
          method_version: "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
          outcome: {
            status: "available",
            information_method: "cbsem_ml_expected_information_delta_method_v1",
            parameters: [{ parameter_id: "parameter:loading:x:x1", standard_error: 0.5 }],
          },
        },
        inference: {
          status: "unavailable",
          reason: "insufficient_studentized_usable_replicates",
          message: "Analytically studentized inference is unavailable because 0 whole-vector usable refits are below the required 1000.",
        },
        intervals: [{
          parameter_id: "parameter:loading:x:x1",
          outcome: {
            status: "unavailable",
            reason: "insufficient_studentized_usable_replicates",
          },
        }],
        refit_standard_errors: [],
      },
    };
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: true, usesConfidenceLevel: true, usesWorkers: true,
    });
    const studentizedProvenance = nativeRunProvenanceTable(run);
    expect(studentizedProvenance.status).toBe("experimental");
    expect(studentizedProvenance.rows).toEqual(expect.arrayContaining([
      ["Exact CB-SEM bootstrap method", "cbsem_exact_case_bootstrap_v1"],
      ["Exact CB-SEM zero-null method", "cbsem_exact_case_bootstrap_null_centered_test_tail_v1"],
      ["Studentized CB-SEM method", "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1"],
      ["Studentized CB-SEM expected-information method", "cbsem_ml_expected_information_delta_method_v1"],
      ["Studentized CB-SEM inference", expect.stringContaining("insufficient_studentized_usable_replicates")],
      ["Studentized CB-SEM point standard errors", "Available for 1 parameter(s)"],
      ["Studentized CB-SEM refit standard-error receipts", "0 available; 0 unavailable"],
      ["Studentized CB-SEM archive validation scope", "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1"],
      ["Studentized CB-SEM archive reopening", "Ledger and arithmetic only; raw refits and expected-information calculations were not replayed"],
    ]));

    const notCompleted = structuredClone(run);
    notCompleted.status = "failed";
    expect(nativeRunProvenanceTable(notCompleted).rows.some(([field]) => field.startsWith("Studentized CB-SEM")))
      .toBe(false);

    const mixed = structuredClone(run);
    mixed.result!.cbsem!.exact_case_bootstrap = mixed.result!.cbsem!.exact_case_bootstrap_studentized!.base;
    expect(nativeRunProvenanceTable(mixed).rows.some(([field]) => field.startsWith("Studentized CB-SEM")))
      .toBe(false);

    const bcaRun = structuredClone(run);
    const bcaAnalysis = bcaRun.result!.cbsem!;
    const bcaBase = bcaAnalysis.exact_case_bootstrap_studentized!.base;
    delete bcaAnalysis.exact_case_bootstrap_studentized;
    bcaAnalysis.exact_case_bootstrap_bca = {
      base: bcaBase,
      bca: {
        method_version: "cbsem_exact_case_bootstrap_bca_interval_v1",
        base_bootstrap_method_version: "cbsem_exact_case_bootstrap_v1",
        outer_recipe_analytical_identity_sha256: bcaBase.outer_recipe_analytical_identity_sha256,
        base_point_result_sha256: bcaBase.base_point_result_sha256,
        compiler_analytical_identity_sha256: bcaBase.compiler_analytical_identity_sha256,
        plan_sha256: bcaBase.plan_sha256,
        model_scientific_sha256: bcaBase.model_scientific_sha256,
        delete_one_refit_method_version: "cbsem_exact_case_bootstrap_delete_one_refit_v1",
        bias_correction_method: "midrank_less_plus_half_ties_no_clamp_v1",
        acceleration_method: "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2",
        adjusted_probability_method: "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2",
        quantile_method: "percentile_type7_v1",
        retry_policy: "no_retry_exactly_one_fit_per_omitted_case_v1",
        confidence_level: 0.95,
        bootstrap_usable_replicates: 0,
        minimum_bootstrap_usable_replicates: 1000,
        delete_one_case_count: bcaAnalysis.sample_size,
        parameter_ids: ["parameter:loading:x:x1"],
        inference: {
          status: "unavailable",
          reason: "base_inference_unavailable",
          message: "BCa inference is unavailable because 0 successful bootstrap point refits are below the bound minimum 1000.",
        },
        intervals: [{
          parameter_id: "parameter:loading:x:x1",
          outcome: { status: "unavailable", reason: "base_inference_unavailable" },
        }],
        successful_delete_one_refits: Array.from({ length: bcaAnalysis.sample_size }, (_, position) => ({
          omitted_complete_case_position: position,
          omitted_source_row_index: position,
          retained_sampling_positions_sha256: "6".repeat(64),
          retained_sample_indices_sha256: "7".repeat(64),
          parameter_estimates: [1],
          iterations: 1,
          objective: 0,
          gradient_norm: 0,
        })),
        failed_delete_one_refits: [],
      },
    };
    expect(nativeRunSettingApplicability(bcaRun)).toEqual({
      usesSeed: true, usesConfidenceLevel: true, usesWorkers: true,
    });
    const bcaProvenance = nativeRunProvenanceTable(bcaRun);
    expect(bcaProvenance.status).toBe("experimental");
    expect(bcaProvenance.rows).toEqual(expect.arrayContaining([
      ["BCa CB-SEM method", "cbsem_exact_case_bootstrap_bca_interval_v1"],
      ["BCa CB-SEM availability", "Experimental Labs; complete-only delete-one inference"],
      ["BCa CB-SEM bootstrap usable refits", "0"],
      ["BCa CB-SEM successful delete-one refits", String(bcaAnalysis.sample_size)],
      ["BCa CB-SEM failed delete-one refits", "0"],
      ["BCa CB-SEM inference", expect.stringContaining("base_inference_unavailable")],
      ["BCa CB-SEM failure policy", "Exactly one fit per omitted complete case; any failure makes global BCa inference unavailable"],
      ["BCa CB-SEM archive validation scope", "Persisted ledger identity, digests, and exposed interval arithmetic only"],
      ["BCa CB-SEM archive reopening", "Raw base and delete-one ML fits were not replayed; Rust remains authoritative for fitting and BCa normal-probability transforms"],
    ]));

    const injectedBca = structuredClone(bcaRun);
    injectedBca.result!.cbsem!.exact_case_bootstrap = injectedBca.result!.cbsem!.exact_case_bootstrap_bca!.base;
    expect(nativeRunProvenanceTable(injectedBca).rows.some(([field]) => field.startsWith("BCa CB-SEM")))
      .toBe(false);
  });

  it("exports model-free PCA scores and PCA-specific provenance without SEM weighting claims", () => {
    const run = completedPcaRun();
    const scoreTable = nativePcaScoreExportTable(run);
    const provenance = nativeRunProvenanceTable(run);

    expect(scoreTable).toMatchObject({
      id: "pca_scores",
      title: "Component scores",
      columns: ["Complete-case observation", "PC1"],
      rows: [["1", "0.000000"], ["2", "0.250000"], ["3", "1.250000"]],
    });
    expect(provenance.rows).toEqual(expect.arrayContaining([
      ["Selected variables", "a, b"],
      ["Retention rule", "Fixed component count"],
      ["Input matrix", "Correlation matrix of standardized variables"],
    ]));
    expect(provenance.rows.some(([field]) => field === "Weighting scheme" || field === "Seed")).toBe(false);
  });

  it("builds one truthful provenance table without legacy N/A placeholders", () => {
    const run = {
      ...completedSamplePlsRun(),
      warnings: ["Small sample warning"],
    };
    const table = nativeRunProvenanceTable(run);

    expect(table.id).toBe("run_provenance");
    expect(table.rows).toContainEqual(["Method version", run.result?.method_version]);
    expect(table.rows).toContainEqual(["Warnings", "Small sample warning"]);
    expect(table.rows.flat().some((cell) => /^N\/?A$/i.test(cell))).toBe(false);
  });

  it("exports complete ordinary PLS bootstrap fit accounting and no-retry provenance", () => {
    const run = structuredClone(completedSamplePlsRun());
    run.provenance = {
      recipe_id: "recipe-bootstrap-v4",
      dataset_fingerprint: run.fingerprint,
      method: "pls_pm",
      method_version: "pls_pm_v1+indexed_resampling_v4",
      engine_version: "2.46.0",
      seed: run.seed,
      settings: {
        method: "pls_pm",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 999,
        studentized_inner_samples: 99,
        permutation_samples: 0,
        seed: run.seed,
        workers: 4,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-18T00:00:00Z",
      completed_at: "2026-08-18T00:00:01Z",
    };
    run.bootstrap!.usable_replicates = 998;
    run.bootstrap!.failed_replicates = [{
      replicate_index: 3,
      reason_code: "constant_indicator",
      message: "constant indicator: competence_1",
    }];
    const table = nativeRunProvenanceTable(run);

    expect(table.rows).toEqual(expect.arrayContaining([
      ["PLS bootstrap method", run.bootstrap!.method_version],
      ["Requested PLS bootstrap refits", "999"],
      ["Attempted PLS bootstrap refits", "999"],
      ["Usable PLS bootstrap refits", "998"],
      ["Failed PLS bootstrap refits", "1"],
      ["PLS bootstrap failure policy", "No retry or replacement draw"],
    ]));
  });

  it("omits unavailable optional fields instead of manufacturing a value", () => {
    const run = {
      ...completedSamplePlsRun(),
      result: undefined,
      warnings: [],
    };
    const table = nativeRunProvenanceTable(run, "experimental");

    expect(table.status).toBe("experimental");
    expect(table.rows.some(([field]) => field === "Method version")).toBe(false);
    expect(table.rows.some(([field]) => field === "Warnings")).toBe(false);
  });

  it("exports immutable engine settings when native provenance is available", () => {
    const run = {
      ...completedSamplePlsRun(),
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-1",
        dataset_fingerprint: "sha256:full-fingerprint",
        method: "wpls" as const,
        method_version: "wpls_case_weighted_v1+assessment_v1",
        engine_version: "2.45.0",
        seed: 20_260_718,
        settings: {
          method: "wpls" as const,
          weighting_scheme: "path" as const,
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_718,
          workers: 1,
          confidence_level: 0.95,
          preprocessing: "standardized" as const,
          missing_data: "listwise_deletion" as const,
          case_weight_column: "case_weight",
        },
        started_at: "2026-08-10T10:00:00Z",
        completed_at: "2026-08-10T10:00:01Z",
      },
    };
    const table = nativeRunProvenanceTable(run);

    expect(table.rows).toContainEqual(["Dataset fingerprint", "sha256:full-fingerprint"]);
    expect(table.rows).toContainEqual(["Method version", "wpls_case_weighted_v1+assessment_v1"]);
    expect(table.rows).toContainEqual(["Case-weight variable", "case_weight"]);
    expect(table.rows).toContainEqual(["Preprocessing", "standardized"]);
    expect(table.rows.some(([field]) => field === "Seed")).toBe(false);
    expect(table.rows.some(([field]) => field === "Confidence level")).toBe(false);
    expect(table.rows.some(([field]) => field === "Workers")).toBe(false);
  });

  it("omits unused inference settings from descriptive CCA exports", () => {
    const run = {
      ...completedSamplePlsRun(),
      method: "CCA composite residual diagnostics",
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-cca",
        dataset_fingerprint: "sha256:cca",
        method: "cca" as const,
        method_version: "pls_pm_v1+cca_composite_residual_v1+pls_assessment_v7",
        engine_version: "2.45.0",
        seed: 20_260_718,
        settings: {
          method: "cca" as const,
          weighting_scheme: "path" as const,
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_718,
          workers: 1,
          confidence_level: 0.95,
          preprocessing: "standardized" as const,
          missing_data: "listwise_deletion" as const,
          case_weight_column: null,
        },
        started_at: "2026-08-11T08:00:00Z",
        completed_at: "2026-08-11T08:00:01Z",
      },
    };
    const table = nativeRunProvenanceTable(run);
    const fields = table.rows.map(([field]) => field);

    expect(fields).not.toContain("Seed");
    expect(fields).not.toContain("Confidence level");
    expect(fields).not.toContain("Workers");
    expect(nativeRunSettingApplicability(run)).toEqual({
      usesSeed: false,
      usesConfidenceLevel: false,
      usesWorkers: false,
    });
    expect(table.rows.flat().some((cell) => /^N\/?A$/i.test(cell))).toBe(false);
  });

  it("exports the genuine MICOM permutation plan without claiming parallel workers", () => {
    const base = completedSamplePlsRun();
    const run: AnalysisRun = {
      ...base,
      method: "MICOM and Two-Group Permutation MGA",
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-mga-v4",
        dataset_fingerprint: "sha256:mga-v4",
        method: "mga",
        method_version: "pls_mga_two_group_v4+pls_mga_permutation_v4+micom_v4+pls_assessment_v7",
        engine_version: "2.45.0",
        seed: 20_260_718,
        settings: {
          method: "mga",
          weighting_scheme: "path",
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 5_000,
          seed: 20_260_718,
          workers: 1,
          confidence_level: 0.95,
          preprocessing: "standardized",
          missing_data: "listwise_deletion",
          case_weight_column: null,
        },
        started_at: "2026-08-11T08:00:00Z",
        completed_at: "2026-08-11T08:00:03Z",
      },
      result: {
        ...base.result!,
        mga_permutation: {
          method_version: "pls_mga_permutation_v4",
          group_column: "group",
          permutation_samples: 5_000,
          usable_permutations: 5_000,
          attempted_permutations: 5_000,
          failed_permutations: 0,
          retry_policy: "none",
          permutation_plan_sha256: `sha256:${"b".repeat(64)}`,
          permutation_ledger: Array.from({ length: 5_000 }, (_, replicate) => ({
            replicate,
            partition_sha256: replicate.toString(16).padStart(64, "0"),
            group_a_rows: 50,
            group_b_rows: 50,
            step2_status: "usable" as const,
            step2_failure_code: null,
            step3_status: "usable" as const,
            step3_failure_code: null,
          })),
          comparisons: [],
          measurement_comparisons: [],
          warnings: [],
        },
        micom: {
          method_version: "micom_v4",
          group_column: "group",
          permutation_samples: 5_000,
          usable_permutations: 5_000,
          attempted_permutations: 5_000,
          failed_permutations: 0,
          retry_policy: "none",
          step1_status: "confirmed_by_researcher_review",
          step1_computed: false,
          step2_usable_permutations: 5_000,
          step2_failed_permutations: 0,
          step3_usable_permutations: 5_000,
          step3_failed_permutations: 0,
          permutation_plan_sha256: `sha256:${"b".repeat(64)}`,
          permutation_ledger: Array.from({ length: 5_000 }, (_, replicate) => ({
            replicate,
            partition_sha256: replicate.toString(16).padStart(64, "0"),
            group_a_rows: 50,
            group_b_rows: 50,
            step2_status: "usable" as const,
            step2_failure_code: null,
            step3_status: "usable" as const,
            step3_failure_code: null,
          })),
          confidence_level: 0.95,
          groups: [{ group: "A", observations: 50 }, { group: "B", observations: 50 }],
          constructs: [],
          warnings: [],
        },
      },
    };
    const table = nativeRunProvenanceTable(run);

    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: true, usesConfidenceLevel: true, usesWorkers: false });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Confidence level", "0.95"],
      ["MICOM Step 1", "Researcher confirmed configural invariance"],
      ["Requested group permutations", "5000"],
      ["Usable group permutations", "5000"],
    ]));
    expect(table.rows.map(([field]) => field)).not.toContain("Workers");
  });

  it("exports exact standalone NCA provenance without irrelevant SEM iteration settings", () => {
    const run = completedNcaRun();
    const immutableRun = structuredClone(run);
    const applicability = nativeRunSettingApplicability(run);
    const table = nativeRunProvenanceTable(run);
    const peers = nativeNcaCeFdhPeerExportTable(run);
    const fields = table.rows.map(([field]) => field);

    expect(applicability).toEqual({ usesSeed: true, usesConfidenceLevel: false, usesWorkers: false });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Seed", "20260811"],
      ["Method version", "nca_v2"],
      ["Condition variable (X)", "condition"],
      ["Outcome variable (Y)", "outcome"],
      ["Analyzed observations", "8"],
      ["Ceiling lines", "CE-FDH and CR-FDH"],
      ["Requested NCA permutations", "19"],
      ["Usable NCA permutations", "19"],
      ["Missing data", "listwise deletion"],
    ]));
    expect(fields).not.toContain("Weighting scheme");
    expect(fields).not.toContain("Maximum iterations");
    expect(fields).not.toContain("Stop criterion");
    expect(fields).not.toContain("Workers");
    expect(table.rows.flat().some((cell) => /^N\/?A$/i.test(cell))).toBe(false);
    expect(peers).toMatchObject({
      id: "nca_ce_fdh_peers",
      title: "CE-FDH frontier peers",
      columns: ["Peer identity", "Condition variable (X)", "Condition value", "Outcome variable (Y)", "Outcome value"],
      rows: [
        ["CE-FDH peer 1", "condition", "1.000000", "outcome", "1.000000"],
        ["CE-FDH peer 2", "condition", "8.000000", "outcome", "9.000000"],
      ],
    });
    expect(peers?.warning).toContain("does not retain original source-row identifiers");
    expect(run).toEqual(immutableRun);
  });

  it("exports exact PLSpredict / CVPAT v2 provenance and the assignment digest", () => {
    const base = completedSamplePlsRun();
    const assessment = (benchmark: "indicator_average" | "linear_model") => ({
      method_version: "cvpat_indicator_benchmarks_v2",
      comparison_kind: "benchmark_assessment" as const,
      target_scope: "all_endogenous_indicators" as const,
      benchmark,
      loss: "mean_squared_error_across_indicators_per_observation" as const,
      alternative: "pls_loss_less_than_benchmark" as const,
      confidence_level: 0.95,
      mean_loss_pls: null,
      mean_loss_benchmark: null,
      mean_loss_difference: null,
      standard_error: null,
      t_statistic: null,
      p_value_one_sided: null,
      confidence_interval_lower: null,
      confidence_interval_upper: null,
      observations: 64,
      indicator_count: 2,
      status: "benchmark_unavailable" as const,
      preferred_model: null,
      reason: "fixture_unavailable",
    });
    const digest = `sha256:${"b".repeat(64)}`;
    const run: AnalysisRun = {
      ...base,
      method: "PLSpredict / CVPAT",
      seed: 20_260_811,
      bootstrap: undefined,
      permutation: undefined,
      provenance: {
        recipe_id: "recipe-prediction-v2",
        dataset_fingerprint: "sha256:prediction-v2",
        method: "predict",
        method_version: "plspredict_indicator_v2",
        engine_version: "2.46.0",
        seed: 20_260_811,
        settings: {
          method: "predict",
          weighting_scheme: "path",
          tolerance: 1e-7,
          max_iterations: 3_000,
          bootstrap_samples: 0,
          studentized_inner_samples: 0,
          permutation_samples: 0,
          seed: 20_260_811,
          workers: 8,
          confidence_level: 0.9,
          preprocessing: "standardized",
          missing_data: "listwise_deletion",
          case_weight_column: null,
        },
        started_at: "2026-08-11T08:00:00Z",
        completed_at: "2026-08-11T08:00:03Z",
      },
      result: {
        ...base.result!,
        used_observations: 64,
        predict: {
          method_version: "plspredict_indicator_v2",
          split: "deterministic_complete_case_modulo_4_test_rows",
          training_observations: 48,
          test_observations: 16,
          benchmark: "indicator_average",
          targets: [],
          indicator_targets: [],
          repeated_kfold: {
            method_version: "plspredict_repeated_kfold_indicator_v2",
            folds: 10,
            repeats: 10,
            assignment: "seeded_chacha20_balanced_folds",
            assignment_digest: digest,
            seed: 20_260_811,
            total_test_observations: 640,
            targets: [],
            indicator_targets: [],
            cvpat_benchmark_assessments: [assessment("indicator_average"), assessment("linear_model")],
            warnings: [],
          },
          warnings: [],
        },
      },
    };

    const table = nativeRunProvenanceTable(run);
    expect(nativeRunSettingApplicability(run)).toEqual({ usesSeed: true, usesConfidenceLevel: true, usesWorkers: false });
    expect(table.rows).toEqual(expect.arrayContaining([
      ["Seed", "20260811"],
      ["CVPAT confidence level", "0.95"],
      ["Repeated prediction method version", "plspredict_repeated_kfold_indicator_v2"],
      ["CVPAT method version", "cvpat_indicator_benchmarks_v2"],
      ["Cross-validation design", "10-fold × 10-repeat cross-validation"],
      ["Assignment digest", digest],
      ["Benchmarks", "Indicator average (IA); Linear model (LM, where estimable)"],
      ["CVPAT alternative", "PLS-SEM loss < benchmark (one-sided)"],
    ]));
    expect(table.rows.map(([field]) => field)).not.toContain("Workers");
    expect(table.rows.flat().join(" ")).not.toContain("saved model");
  });
});
