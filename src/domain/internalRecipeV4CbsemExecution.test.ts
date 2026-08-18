import { describe, expect, it } from "vitest";
import {
  cbsemCfaScoreLmChiSquare1PValueV1,
  parseCbsemCfaScoreLmBundleV1,
  parseInternalRecipeV4CbsemCompletedResultV1,
  parseInternalRecipeV4CbsemExecutionResultV1,
  parseMeanReplacementReceiptV1,
} from "./internalRecipeV4CbsemExecution";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import type {
  CbsemCfaScoreLmBundleV1,
  CbsemExactCaseBootstrapResultV1,
  CbsemExactCaseBootstrapStudentizedSidecarV1,
  CbsemExactCaseBootstrapWithStudentizedResultV1,
} from "../types";

function meanReplacementReceiptFixture() {
  return {
    method_version: "mean_replacement_v1",
    policy: "mean_replacement",
    source_dataset_id: "raw-data",
    source_dataset_fingerprint: "raw-fingerprint-v1",
    source_row_count: 20,
    retained_row_count: 20,
    omitted_row_count: 0,
    modeled_variable_count: 3,
    imputed_cell_count: 6,
    affected_case_count: 4,
    variable_warning_threshold: 0.05,
    high_missingness_threshold: 0.15,
    variables: [
      {
        variable_order: 0,
        variable_id: "observed:x1",
        source_column: "x1",
        canonical_missing_markers: ["-99", "NA"],
        observed_count: 19,
        missing_count: 1,
        replacement_mean: 10,
        missing_fraction: 0.05,
        warning_level: "at_least_five_percent",
      },
      {
        variable_order: 1,
        variable_id: "observed:x2",
        source_column: "x2",
        canonical_missing_markers: ["-99", "NA"],
        observed_count: 16,
        missing_count: 4,
        replacement_mean: 11,
        missing_fraction: 0.2,
        warning_level: "above_fifteen_percent",
      },
      {
        variable_order: 2,
        variable_id: "observed:x3",
        source_column: "x3",
        canonical_missing_markers: ["-99", "NA"],
        observed_count: 19,
        missing_count: 1,
        replacement_mean: 12,
        missing_fraction: 0.05,
        warning_level: "at_least_five_percent",
      },
    ],
    cases: [
      {
        row_index_zero_based: 0,
        imputed_variable_ids: ["observed:x1", "observed:x2", "observed:x3"],
        missing_fraction: 1,
        high_missingness_warning: true,
      },
      {
        row_index_zero_based: 1,
        imputed_variable_ids: ["observed:x2"],
        missing_fraction: 1 / 3,
        high_missingness_warning: true,
      },
      {
        row_index_zero_based: 2,
        imputed_variable_ids: ["observed:x2"],
        missing_fraction: 1 / 3,
        high_missingness_warning: true,
      },
      {
        row_index_zero_based: 3,
        imputed_variable_ids: ["observed:x2"],
        missing_fraction: 1 / 3,
        high_missingness_warning: true,
      },
    ],
    missingness_sha256: "8".repeat(64),
    completed_matrix_sha256: "9".repeat(64),
    receipt_sha256: "a".repeat(64),
  };
}

function resultFixture() {
  return {
    schema_version: 1,
    provenance: {
      adapter_version: "compiled_recipe_v4_cbsem_plan_v2_execution_v7",
      compilation_receipt: {
        schema_version: 1,
        recipe_id: "00000000-0000-4000-8000-000000000001",
        recipe_document_sha256: "1".repeat(64),
        recipe_analytical_sha256: "2".repeat(64),
        model_id: "model-v4",
        model_document_sha256: "3".repeat(64),
        model_scientific_sha256: "4".repeat(64),
        dataset_fingerprint: "raw-fingerprint-v1",
        compiler_target: "cbsem_plan_v2",
        compiler_version: "sem_model_v4_cbsem_compiler_v2",
        capability_cell: {
          registry_schema_version: 2,
          capability_id: "smartpls.cbsem",
          cell_id: "qpls3.cbsem.ml",
          capability_version: "cbsem_ml_v1",
        },
        plan_sha256: "5".repeat(64),
        analytical_identity_sha256: "6".repeat(64),
      },
      dataset_id: "raw-data",
      estimator_method_version: "cbsem_ml_exact_parameter_table_v3",
      moment_input_method_version: "cbsem_ml_compiled_moment_input_mean_replacement_v1",
    },
    estimation: {
      schema_version: 4,
      method_version: "cbsem_ml_compiled_moment_input_mean_replacement_v1",
      compiler_analytical_identity_sha256: "6".repeat(64),
      plan_sha256: "5".repeat(64),
      model_scientific_sha256: "4".repeat(64),
      input: {
        kind: "raw",
        dataset_id: "raw-data",
        dataset_fingerprint: "raw-fingerprint-v1",
        declared_sample_size: null,
        used_sample_size: 20,
        omitted_observations: 0,
        covariance_denominator: "maximum_likelihood_n",
        variable_ids: ["observed:x1", "observed:x2", "observed:x3"],
        source_columns: ["x1", "x2", "x3"],
        standard_deviations: null,
        canonical_ml_covariance_sha256: "7".repeat(64),
        missing_data_treatment: meanReplacementReceiptFixture(),
      },
      covariance_ml: [
        [1, 0.2, 0.1],
        [0.2, 1.1, 0.3],
        [0.1, 0.3, 1.2],
      ],
      parameter_ids: { loading_x2: "parameter:loading:x2" },
      analysis: { method_version: "cbsem_ml_exact_parameter_table_v3" },
    },
  };
}

function exactBootstrapResultFixture(adapter: "v9" | "v10" | "v11" | "v12") {
  const result = resultFixture();
  delete (result.estimation.input as { missing_data_treatment?: unknown }).missing_data_treatment;
  const fingerprint = "a".repeat(64);
  result.provenance.adapter_version = `compiled_recipe_v4_cbsem_plan_v2_execution_${adapter}`;
  result.provenance.compilation_receipt.dataset_fingerprint = fingerprint;
  result.provenance.moment_input_method_version = "cbsem_ml_exact_parameter_table_v3";
  result.estimation.schema_version = 2;
  result.estimation.method_version = "cbsem_ml_exact_parameter_table_v3";
  result.estimation.input.dataset_fingerprint = fingerprint;
  result.estimation.input.used_sample_size = 18;
  result.estimation.input.omitted_observations = 2;
  const failures = Array.from({ length: 500 }, (_, replicate_index) => ({
    replicate_index,
    sampling_positions_sha256: "7".repeat(64),
    sample_indices_sha256: "8".repeat(64),
    kind: "non_convergence",
    message: "Did not converge.",
  }));
  const exact = {
    method_version: "cbsem_exact_case_bootstrap_v1",
    estimator_method_version: "cbsem_ml_exact_parameter_table_v3",
    source_dataset_id: "raw-data",
    source_dataset_fingerprint: fingerprint,
    outer_recipe_analytical_identity_sha256: "2".repeat(64),
    base_point_result_sha256: "b".repeat(64),
    compiler_analytical_identity_sha256: "6".repeat(64),
    plan_sha256: "5".repeat(64),
    model_scientific_sha256: "4".repeat(64),
    complete_case_sample_size: 18,
    complete_case_universe_digest_method: "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1",
    complete_case_universe_sha256: "c".repeat(64),
    covariance_denominator: "maximum_likelihood_n",
    sample_indices_digest_method: "sha256_source_fingerprint_and_ordered_u64_indices_v1",
    sampling_positions_digest_method: "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1",
    interval_method: "percentile_type7_v1",
    confidence_level: 0.95,
    requested_replicates: 500,
    attempted_refits: 500,
    usable_replicates: 0,
    failed_replicates: 500,
    minimum_usable_fraction: 0.9,
    minimum_usable_replicates: 1000,
    seed: 91,
    stream_token: "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1",
    retry_policy: "no_retry_fixed_preplanned_primary_draws_v1",
    max_attempts_per_replicate: 1,
    parameter_ids: ["parameter:loading:x2"],
    inference: { status: "unavailable", reason_code: "insufficient_usable_refits", message: "Pilot is below the frozen threshold." },
    intervals: [],
    successful_refits: [],
    failed_refits: failures,
    ...(adapter !== "v9" ? {
      hypothesis_tests: {
        method_version: "cbsem_exact_case_bootstrap_null_centered_test_tail_v1",
        null_hypothesis: "compiled_free_parameter_equals_zero_v1",
        statistic: "unstudentized_null_centered_parameter_estimate_v1",
        tie_policy: "inclusive_ieee_comparison_v1",
        probability_method: "plus_one_over_usable_plus_one_v1",
        decision_rule: "selected_p_value_less_than_or_equal_alpha_v1",
        selected_test_tail: "two_sided",
        null_value: 0,
        significance_level: 0.05,
        usable_replicates: 0,
        inference: { status: "unavailable", reason_code: "insufficient_usable_refits", message: "Pilot is below the frozen threshold." },
        parameters: [{
          parameter_id: "parameter:loading:x2",
          outcome: { status: "unavailable", reason: "insufficient_usable_replicates" },
        }],
      },
    } : {}),
  };
  const studentized = {
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
    parameter_ids: ["parameter:loading:x2"],
    point_standard_errors: {
      method_version: "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
      outcome: {
        status: "available",
        information_method: "cbsem_ml_expected_information_delta_method_v1",
        parameters: [{ parameter_id: "parameter:loading:x2", standard_error: 0.1 }],
      },
    },
    inference: {
      status: "unavailable",
      reason: "insufficient_studentized_usable_replicates",
      message: "Analytically studentized inference is unavailable because 0 whole-vector usable refits are below the required 1000.",
    },
    intervals: [{
      parameter_id: "parameter:loading:x2",
      outcome: { status: "unavailable", reason: "insufficient_studentized_usable_replicates" },
    }],
    refit_standard_errors: [],
  };
  const bca = {
    method_version: "cbsem_exact_case_bootstrap_bca_interval_v1",
    base_bootstrap_method_version: "cbsem_exact_case_bootstrap_v1",
    outer_recipe_analytical_identity_sha256: exact.outer_recipe_analytical_identity_sha256,
    base_point_result_sha256: exact.base_point_result_sha256,
    compiler_analytical_identity_sha256: exact.compiler_analytical_identity_sha256,
    plan_sha256: exact.plan_sha256,
    model_scientific_sha256: exact.model_scientific_sha256,
    delete_one_refit_method_version: "cbsem_exact_case_bootstrap_delete_one_refit_v1",
    bias_correction_method: "midrank_less_plus_half_ties_no_clamp_v1",
    acceleration_method: "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2",
    adjusted_probability_method: "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2",
    quantile_method: "percentile_type7_v1",
    retry_policy: "no_retry_exactly_one_fit_per_omitted_case_v1",
    confidence_level: 0.95,
    bootstrap_usable_replicates: 0,
    minimum_bootstrap_usable_replicates: 1_000,
    delete_one_case_count: 18,
    parameter_ids: ["parameter:loading:x2"],
    inference: {
      status: "unavailable",
      reason: "base_inference_unavailable",
      message: "BCa inference is unavailable because 0 successful bootstrap point refits are below the bound minimum 1000.",
    },
    intervals: [{
      parameter_id: "parameter:loading:x2",
      outcome: { status: "unavailable", reason: "base_inference_unavailable" },
    }],
    successful_delete_one_refits: Array.from({ length: 18 }, (_, position) => ({
      omitted_complete_case_position: position,
      omitted_source_row_index: position + 1,
      retained_sampling_positions_sha256: "d".repeat(64),
      retained_sample_indices_sha256: "e".repeat(64),
      parameter_estimates: [position % 2 === 0 ? 0.5 : 1.5],
      iterations: 1,
      objective: 0,
      gradient_norm: 0,
    })),
    failed_delete_one_refits: [],
  };
  Object.assign(result.estimation.analysis, {
    model_type: "cfa",
    mean_structure: false,
    modification_indices: [],
    score_lm: {
      method_version: "cbsem_cfa_score_lm_v1",
      scope: "covariance_only_declared_zero_residual_covariances",
      rows: [{
        parameter_id: "parameter:loading:x2", kind: "residual_covariance", lhs: "x1", rhs: "x2",
        outcome: { status: "available", score: 2, efficient_score: 2, candidate_information: 1,
          efficient_information: 1, modification_index: 4, expected_parameter_change: 2,
          p_value: cbsemCfaScoreLmChiSquare1PValueV1(4) },
      }],
    },
    ...(adapter === "v11"
      ? { exact_case_bootstrap_studentized: { base: exact, studentized } }
      : adapter === "v12"
        ? { exact_case_bootstrap_bca: { base: exact, bca } }
        : { exact_case_bootstrap: exact }),
  });
  return result;
}

function availableStudentizedResultFixture() {
  const result = exactBootstrapResultFixture("v11");
  const wrapper = (result.estimation.analysis as unknown as {
    exact_case_bootstrap_studentized: {
      base: Record<string, any>;
      studentized: Record<string, any>;
    };
  }).exact_case_bootstrap_studentized;
  const successes = Array.from({ length: 1_000 }, (_, replicate_index) => ({
    replicate_index,
    sampling_positions_sha256: "7".repeat(64),
    sample_indices_sha256: "8".repeat(64),
    parameter_estimates: [2],
    iterations: 1,
    objective: 0,
    gradient_norm: 0,
  }));
  Object.assign(wrapper.base, {
    requested_replicates: 1_000,
    attempted_refits: 1_000,
    usable_replicates: 1_000,
    failed_replicates: 0,
    minimum_usable_replicates: 1_000,
    inference: { status: "available" },
    intervals: [{
      parameter_id: "parameter:loading:x2",
      original: 1,
      bootstrap_mean: 2,
      bias: 1,
      standard_error: 0,
      percentile_lower: 2,
      percentile_upper: 2,
      usable_replicates: 1_000,
    }],
    successful_refits: successes,
    failed_refits: [],
    hypothesis_tests: {
      ...wrapper.base.hypothesis_tests,
      usable_replicates: 1_000,
      inference: { status: "available" },
      parameters: [{
        parameter_id: "parameter:loading:x2",
        outcome: {
          status: "available",
          point_estimate: 1,
          two_sided_exceedances: 1_000,
          greater_or_equal_exceedances: 1_000,
          less_or_equal_exceedances: 1_000,
          p_value_two_sided: 1,
          p_value_greater: 1,
          p_value_less: 1,
          selected_exceedances: 1_000,
          selected_p_value: 1,
          reject_null: false,
        },
      }],
    },
  });
  Object.assign(wrapper.studentized, {
    minimum_usable_replicates: 1_000,
    studentized_usable_replicates: 1_000,
    inference: { status: "available" },
    intervals: [{
      parameter_id: "parameter:loading:x2",
      outcome: {
        status: "available",
        point_estimate: 1,
        point_standard_error: 0.25,
        lower_pivot_quantile: 2,
        upper_pivot_quantile: 2,
        interval_lower: 0.5,
        interval_upper: 0.5,
        usable_replicates: 1_000,
      },
    }],
    point_standard_errors: {
      method_version: "cbsem_exact_case_bootstrap_refit_standard_errors_v1",
      outcome: {
        status: "available",
        information_method: "cbsem_ml_expected_information_delta_method_v1",
        parameters: [{ parameter_id: "parameter:loading:x2", standard_error: 0.25 }],
      },
    },
    refit_standard_errors: successes.map((row) => ({
      replicate_index: row.replicate_index,
      outcome: {
        status: "available",
        information_method: "cbsem_ml_expected_information_delta_method_v1",
        standard_errors: [0.5],
      },
    })),
  });
  return result;
}

function availableBcaResultFixture() {
  const result = exactBootstrapResultFixture("v12");
  const wrapper = (result.estimation.analysis as unknown as {
    exact_case_bootstrap_bca: {
      base: Record<string, any>;
      bca: Record<string, any>;
    };
  }).exact_case_bootstrap_bca;
  const { base, bca } = wrapper;
  base.requested_replicates = 1000;
  base.attempted_refits = 1000;
  base.usable_replicates = 1000;
  base.failed_replicates = 0;
  base.inference = { status: "available" };
  base.intervals = [{
    parameter_id: "parameter:loading:x2",
    original: 1,
    bootstrap_mean: 2,
    bias: 1,
    standard_error: 0,
    percentile_lower: 2,
    percentile_upper: 2,
    usable_replicates: 1000,
  }];
  base.successful_refits = Array.from({ length: 1000 }, (_, replicate_index) => ({
    replicate_index,
    sampling_positions_sha256: "7".repeat(64),
    sample_indices_sha256: "8".repeat(64),
    parameter_estimates: [2],
    iterations: 1,
    objective: 0,
    gradient_norm: 0,
  }));
  base.failed_refits = [];
  base.hypothesis_tests.usable_replicates = 1000;
  base.hypothesis_tests.inference = { status: "available" };
  base.hypothesis_tests.parameters[0].outcome = {
    status: "available",
    point_estimate: 1,
    two_sided_exceedances: 1000,
    greater_or_equal_exceedances: 1000,
    less_or_equal_exceedances: 1000,
    p_value_two_sided: 1,
    p_value_greater: 1,
    p_value_less: 1,
    selected_exceedances: 1000,
    selected_p_value: 1,
    reject_null: false,
  };
  bca.bootstrap_usable_replicates = 1000;
  bca.inference = { status: "available" };
  bca.intervals[0].outcome = {
    status: "available",
    point_estimate: 1,
    bias_correction: 0,
    acceleration: 0,
    adjusted_lower_probability: 0.025000000000000022,
    adjusted_upper_probability: 0.975,
    interval_lower: 2,
    interval_upper: 2,
    usable_replicates: 1000,
  };
  return result;
}

function completedExactBootstrapFixture(adapter: "v9" | "v10" | "v11" | "v12") {
  const analyticalResult = exactBootstrapResultFixture(adapter);
  const analysis = analyticalResult.estimation.analysis as unknown as {
    score_lm: CbsemCfaScoreLmBundleV1;
    exact_case_bootstrap?: CbsemExactCaseBootstrapResultV1;
    exact_case_bootstrap_studentized?: CbsemExactCaseBootstrapWithStudentizedResultV1;
    exact_case_bootstrap_bca?: import("../types").CbsemExactCaseBootstrapWithBcaResultV1;
  };
  const score = analysis.score_lm;
  const exact = analysis.exact_case_bootstrap ?? analysis.exact_case_bootstrap_studentized?.base
    ?? analysis.exact_case_bootstrap_bca!.base;
  const studentized = analysis.exact_case_bootstrap_studentized?.studentized;
  const bca = analysis.exact_case_bootstrap_bca?.bca;
  const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
  const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
  const booleanColumn = (id: string) => ({ id, label: id, data_type: "boolean" as const, description: id });
  const text = (value: string) => ({ kind: "text" as const, value });
  const number = (value: number) => ({ kind: "number" as const, value });
  const missing = () => ({ kind: "missing" as const, reason: "not_applicable" as const });
  const capability = {
    ...analyticalResult.provenance.compilation_receipt.capability_cell,
    registry_schema_version: 2 as const,
  };
  const tables: CanonicalResultDocumentV2["tables"] = [];
  tables.push({
    id: "modification_index_score_tests", title: "Score tests",
    columns: ["method_version", "scope", "parameter_id", "kind", "lhs", "rhs", "status", "score", "efficient_score", "candidate_information", "efficient_information", "modification_index", "expected_parameter_change", "degrees_of_freedom", "p_value", "unavailable_reason"]
      .map((id, index) => index <= 6 || index === 15 ? textColumn(id) : numberColumn(id)),
    rows: score.rows.map((row, index) => {
      if (row.outcome.status !== "available") throw new Error("Expected available score fixture.");
      return { id: `score_lm_${String(index).padStart(4, "0")}`, cells: [
        text(score.method_version), text(score.scope), text(row.parameter_id), text(row.kind), text(row.lhs), text(row.rhs), text("available"),
        number(row.outcome.score), number(row.outcome.efficient_score), number(row.outcome.candidate_information),
        number(row.outcome.efficient_information), number(row.outcome.modification_index), number(row.outcome.expected_parameter_change),
        number(1), number(row.outcome.p_value), missing(),
      ] };
    }), footnote_ids: [],
  });
  const summaryColumns = [
    "method_version", "estimator_method_version", "source_dataset_id", "source_dataset_fingerprint", "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256", "model_scientific_sha256", "complete_case_sample_size",
    "complete_case_universe_digest_method", "complete_case_universe_sha256", "covariance_denominator", "sample_indices_digest_method",
    "sampling_positions_digest_method", "interval_method", "confidence_level", "requested_replicates", "attempted_refits", "usable_replicates",
    "failed_replicates", "minimum_usable_fraction", "minimum_usable_replicates", "seed_decimal", "stream_token", "retry_policy",
    "max_attempts_per_replicate", "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message", "archive_validation_scope",
  ];
  const summaryText = new Set([0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, 15, 23, 24, 25, 27, 28, 29, 30, 31]);
  const inference = exact.inference.status === "unavailable" ? exact.inference : null;
  tables.push({
    id: "exact_case_bootstrap_summary", title: "Summary",
    columns: summaryColumns.map((id, index) => summaryText.has(index) ? textColumn(id) : numberColumn(id)),
    rows: [{ id: "bootstrap", cells: [
      text(exact.method_version), text(exact.estimator_method_version), text(exact.source_dataset_id), text(exact.source_dataset_fingerprint),
      text(exact.outer_recipe_analytical_identity_sha256), text(exact.base_point_result_sha256), text(exact.compiler_analytical_identity_sha256),
      text(exact.plan_sha256), text(exact.model_scientific_sha256), number(exact.complete_case_sample_size),
      text(exact.complete_case_universe_digest_method), text(exact.complete_case_universe_sha256), text(exact.covariance_denominator),
      text(exact.sample_indices_digest_method), text(exact.sampling_positions_digest_method), text(exact.interval_method), number(exact.confidence_level),
      number(exact.requested_replicates), number(exact.attempted_refits), number(exact.usable_replicates), number(exact.failed_replicates),
      number(exact.minimum_usable_fraction), number(exact.minimum_usable_replicates), text(String(exact.seed)), text(exact.stream_token),
      text(exact.retry_policy), number(exact.max_attempts_per_replicate), text(JSON.stringify(exact.parameter_ids)), text(exact.inference.status),
      inference ? text(inference.reason_code) : missing(), inference ? text(inference.message) : missing(),
      text("schedule_and_arithmetic_only_no_raw_refit_replay_or_source_row_digest_recomputation"),
    ] }], footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_parameter_intervals", title: "Intervals",
    columns: ["parameter_id", "original", "bootstrap_mean", "bias", "standard_error", "percentile_lower", "percentile_upper", "usable_replicates"].map((id, index) => index === 0 ? textColumn(id) : numberColumn(id)),
    rows: [], footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_successful_refits", title: "Refits",
    columns: ["replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm"].map((id, index) => [1, 2, 3].includes(index) ? textColumn(id) : numberColumn(id)),
    rows: [], footnote_ids: [],
  }, {
    id: "exact_case_bootstrap_failures", title: "Failures",
    columns: ["replicate_index", "sampling_positions_sha256", "sample_indices_sha256", "kind", "message"].map((id, index) => index === 0 ? numberColumn(id) : textColumn(id)),
    rows: exact.failed_refits.map((row) => ({
      id: `bootstrap_failure_${String(row.replicate_index).padStart(5, "0")}`,
      cells: [number(row.replicate_index), text(row.sampling_positions_sha256), text(row.sample_indices_sha256), text(row.kind), text(row.message)],
    })), footnote_ids: [],
  });
  if (exact.hypothesis_tests) {
    const tests = exact.hypothesis_tests;
    const columns = [
      "method_version", "null_hypothesis", "statistic", "tie_policy", "probability_method", "decision_rule", "selected_test_tail",
      "null_value", "significance_level", "usable_replicates", "inference_status", "global_unavailable_reason_code", "global_unavailable_message",
      "parameter_id", "parameter_status", "point_estimate", "two_sided_exceedances", "greater_or_equal_exceedances", "less_or_equal_exceedances",
      "p_value_two_sided", "p_value_greater", "p_value_less", "selected_exceedances", "selected_p_value", "reject_null", "unavailable_reason",
    ];
    const textIndices = new Set([0, 1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 25]);
    const global = tests.inference.status === "unavailable" ? tests.inference : null;
    tables.push({
      id: "exact_case_bootstrap_hypothesis_tests", title: "Hypothesis tests",
      columns: columns.map((id, index) => index === 24 ? booleanColumn(id) : textIndices.has(index) ? textColumn(id) : numberColumn(id)),
      rows: tests.parameters.map((parameter, index) => ({
        id: `bootstrap_hypothesis_${String(index).padStart(4, "0")}`,
        cells: [
          text(tests.method_version), text(tests.null_hypothesis), text(tests.statistic), text(tests.tie_policy), text(tests.probability_method),
          text(tests.decision_rule), text(tests.selected_test_tail), number(tests.null_value), number(tests.significance_level), number(tests.usable_replicates),
          text(tests.inference.status), global ? text(global.reason_code) : missing(), global ? text(global.message) : missing(),
          text(parameter.parameter_id), text(parameter.outcome.status), ...Array.from({ length: 10 }, missing),
          text(parameter.outcome.status === "unavailable" ? parameter.outcome.reason : ""),
        ],
      })), footnote_ids: [],
    });
  }
  if (studentized) {
    const studentizedUnavailable = studentized.inference.status === "unavailable" ? studentized.inference : null;
    const pointOutcome = studentized.point_standard_errors.outcome;
    const summaryColumns = [
      "method_version", "standard_error_method_version", "expected_information_method", "pivot_method", "quantile_method",
      "interval_method", "archive_validation_scope", "confidence_level", "minimum_usable_fraction", "minimum_usable_replicates",
      "studentized_usable_replicates", "parameter_ids_json", "inference_status", "unavailable_reason_code", "unavailable_message",
    ];
    const summaryText = new Set([0, 1, 2, 3, 4, 5, 6, 11, 12, 13, 14]);
    tables.push({
      id: "exact_case_bootstrap_studentized_summary", title: "Studentized summary",
      columns: summaryColumns.map((id, index) => summaryText.has(index) ? textColumn(id) : numberColumn(id)),
      rows: [{ id: "bootstrap_studentized", cells: [
        text(studentized.method_version), text(studentized.standard_error_method_version), text(studentized.expected_information_method),
        text(studentized.pivot_method), text(studentized.quantile_method), text(studentized.interval_method),
        text(studentized.archive_validation_scope), number(studentized.confidence_level), number(studentized.minimum_usable_fraction),
        number(studentized.minimum_usable_replicates), number(studentized.studentized_usable_replicates),
        text(JSON.stringify(studentized.parameter_ids)), text(studentized.inference.status),
        studentizedUnavailable ? text(studentizedUnavailable.reason) : missing(),
        studentizedUnavailable ? text(studentizedUnavailable.message) : missing(),
      ] }], footnote_ids: [],
    }, {
      id: "exact_case_bootstrap_studentized_point_standard_errors", title: "Point standard errors",
      columns: ["method_version", "parameter_id", "status", "information_method", "standard_error", "unavailable_reason"]
        .map((id, index) => index === 4 ? numberColumn(id) : textColumn(id)),
      rows: studentized.parameter_ids.map((parameterId, index) => ({
        id: `bootstrap_studentized_point_standard_error_${String(index).padStart(4, "0")}`,
        cells: pointOutcome.status === "available" ? [
          text(studentized.point_standard_errors.method_version), text(parameterId), text("available"),
          text(pointOutcome.information_method), number(pointOutcome.parameters[index]!.standard_error), missing(),
        ] : [
          text(studentized.point_standard_errors.method_version), text(parameterId), text("unavailable"),
          missing(), missing(), text(pointOutcome.reason),
        ],
      })), footnote_ids: [],
    }, {
      id: "exact_case_bootstrap_studentized_parameter_intervals", title: "Studentized intervals",
      columns: ["parameter_id", "status", "point_estimate", "point_standard_error", "lower_pivot_quantile", "upper_pivot_quantile", "interval_lower", "interval_upper", "usable_replicates", "unavailable_reason"]
        .map((id, index) => index <= 1 || index === 9 ? textColumn(id) : numberColumn(id)),
      rows: studentized.intervals.map((interval, index) => ({
        id: `bootstrap_studentized_interval_${String(index).padStart(4, "0")}`,
        cells: interval.outcome.status === "available" ? [
          text(interval.parameter_id), text("available"), number(interval.outcome.point_estimate),
          number(interval.outcome.point_standard_error), number(interval.outcome.lower_pivot_quantile),
          number(interval.outcome.upper_pivot_quantile), number(interval.outcome.interval_lower),
          number(interval.outcome.interval_upper), number(interval.outcome.usable_replicates), missing(),
        ] : [
          text(interval.parameter_id), text("unavailable"), missing(), missing(), missing(), missing(), missing(), missing(), missing(),
          text(interval.outcome.reason),
        ],
      })), footnote_ids: [],
    }, {
      id: "exact_case_bootstrap_studentized_refit_standard_errors", title: "Refit standard errors",
      columns: ["replicate_index", "status", "information_method", "standard_errors_json", "unavailable_reason"]
        .map((id, index) => index === 0 ? numberColumn(id) : textColumn(id)),
      rows: studentized.refit_standard_errors.map((receipt) => ({
        id: `bootstrap_studentized_refit_standard_error_${String(receipt.replicate_index).padStart(5, "0")}`,
        cells: receipt.outcome.status === "available" ? [
          number(receipt.replicate_index), text("available"), text(receipt.outcome.information_method),
          text(JSON.stringify(receipt.outcome.standard_errors)), missing(),
        ] : [number(receipt.replicate_index), text("unavailable"), missing(), missing(), text(receipt.outcome.reason)],
      })), footnote_ids: [],
    });
  }
  if (bca) {
    const unavailable = bca.inference.status === "unavailable" ? bca.inference : null;
    const summaryColumns = [
      "method_version", "base_bootstrap_method_version", "outer_recipe_analytical_identity_sha256",
      "base_point_result_sha256", "compiler_analytical_identity_sha256", "plan_sha256",
      "model_scientific_sha256", "delete_one_refit_method_version",
      "delete_one_sampling_positions_digest_method", "delete_one_sample_indices_digest_method",
      "bias_correction_method", "acceleration_method", "adjusted_probability_method", "quantile_method",
      "retry_policy", "archive_validation_scope", "confidence_level", "bootstrap_usable_replicates",
      "minimum_bootstrap_usable_replicates", "delete_one_case_count", "successful_delete_one_refits",
      "failed_delete_one_refits", "parameter_ids_json", "inference_status", "unavailable_reason_code",
      "unavailable_message",
    ];
    const summaryText = new Set([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 22, 23, 24, 25]);
    tables.push({
      id: "exact_case_bootstrap_bca_summary", title: "BCa summary",
      columns: summaryColumns.map((id, index) => summaryText.has(index) ? textColumn(id) : numberColumn(id)),
      rows: [{ id: "bootstrap_bca", cells: [
        text(bca.method_version), text(bca.base_bootstrap_method_version),
        text(bca.outer_recipe_analytical_identity_sha256), text(bca.base_point_result_sha256),
        text(bca.compiler_analytical_identity_sha256), text(bca.plan_sha256), text(bca.model_scientific_sha256),
        text(bca.delete_one_refit_method_version),
        text("sha256_complete_case_n_and_ordered_sampling_positions_v1"),
        text("sha256_source_fingerprint_and_ordered_u64_indices_v1"), text(bca.bias_correction_method),
        text(bca.acceleration_method), text(bca.adjusted_probability_method), text(bca.quantile_method),
        text(bca.retry_policy),
        text("ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1"),
        number(bca.confidence_level), number(bca.bootstrap_usable_replicates),
        number(bca.minimum_bootstrap_usable_replicates), number(bca.delete_one_case_count),
        number(bca.successful_delete_one_refits.length), number(bca.failed_delete_one_refits.length),
        text(JSON.stringify(bca.parameter_ids)), text(bca.inference.status),
        unavailable ? text(unavailable.reason) : missing(), unavailable ? text(unavailable.message) : missing(),
      ] }], footnote_ids: [],
    }, {
      id: "exact_case_bootstrap_bca_parameter_intervals", title: "BCa parameter intervals",
      columns: [
        "parameter_id", "status", "point_estimate", "bias_correction", "acceleration",
        "adjusted_lower_probability", "adjusted_upper_probability", "interval_lower", "interval_upper",
        "usable_replicates", "unavailable_reason",
      ].map((id, index) => index <= 1 || index === 10 ? textColumn(id) : numberColumn(id)),
      rows: bca.intervals.map((interval, index) => ({
        id: `bootstrap_bca_interval_${String(index).padStart(4, "0")}`,
        cells: interval.outcome.status === "available" ? [
          text(interval.parameter_id), text("available"), number(interval.outcome.point_estimate),
          number(interval.outcome.bias_correction), number(interval.outcome.acceleration),
          number(interval.outcome.adjusted_lower_probability), number(interval.outcome.adjusted_upper_probability),
          number(interval.outcome.interval_lower), number(interval.outcome.interval_upper),
          number(interval.outcome.usable_replicates), missing(),
        ] : [
          text(interval.parameter_id), text("unavailable"), missing(), missing(), missing(), missing(), missing(),
          missing(), missing(), missing(), text(interval.outcome.reason),
        ],
      })), footnote_ids: [],
    }, {
      id: "exact_case_bootstrap_bca_successful_delete_one_refits", title: "BCa successful delete-one refits",
      columns: [
        "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
        "retained_sample_indices_sha256", "parameter_estimates_json", "iterations", "objective", "gradient_norm",
      ].map((id, index) => [2, 3, 4].includes(index) ? textColumn(id) : numberColumn(id)),
      rows: bca.successful_delete_one_refits.map((row) => ({
        id: `bootstrap_bca_delete_one_refit_${String(row.omitted_complete_case_position).padStart(5, "0")}`,
        cells: [number(row.omitted_complete_case_position), number(row.omitted_source_row_index),
          text(row.retained_sampling_positions_sha256), text(row.retained_sample_indices_sha256),
          text(JSON.stringify(row.parameter_estimates)), number(row.iterations), number(row.objective),
          number(row.gradient_norm)],
      })), footnote_ids: [],
    }, {
      id: "exact_case_bootstrap_bca_failures", title: "BCa delete-one failures",
      columns: [
        "omitted_complete_case_position", "omitted_source_row_index", "retained_sampling_positions_sha256",
        "retained_sample_indices_sha256", "kind", "message",
      ].map((id, index) => index >= 2 ? textColumn(id) : numberColumn(id)),
      rows: bca.failed_delete_one_refits.map((row) => ({
        id: `bootstrap_bca_delete_one_failure_${String(row.omitted_complete_case_position).padStart(5, "0")}`,
        cells: [number(row.omitted_complete_case_position), number(row.omitted_source_row_index),
          text(row.retained_sampling_positions_sha256), text(row.retained_sample_indices_sha256),
          text(row.kind), text(row.message)],
      })), footnote_ids: [],
    });
  }
  for (const table of tables) table.capability_cells = [capability];
  const document: CanonicalResultDocumentV2 = {
    schema_version: 2, document_id: "cbsem.exact:1", title: "Exact CB-SEM",
    provenance: {
      run_id: "run-1", project_id: "project-1", model_id: "model-v4",
      model_digest: exact.model_scientific_sha256, dataset_id: exact.source_dataset_id,
      dataset_fingerprint: exact.source_dataset_fingerprint,
      recipe_id: analyticalResult.provenance.compilation_receipt.recipe_id,
      recipe_digest: exact.outer_recipe_analytical_identity_sha256, capability_cell: capability,
      method_version: exact.estimator_method_version, engine_version: analyticalResult.provenance.adapter_version,
      seed: exact.seed, workers: 1, started_at: "2026-08-16T00:00:00Z", completed_at: "2026-08-16T00:00:01Z",
    },
    capability_cells: [capability],
    sections: [
      { id: "modification_indices", title: "Modification indices", table_ids: ["modification_index_score_tests"], chart_ids: [], capability_cells: [capability] },
      { id: "bootstrap_inference", title: "Bootstrap inference", table_ids: ["exact_case_bootstrap_summary", "exact_case_bootstrap_parameter_intervals", "exact_case_bootstrap_successful_refits", "exact_case_bootstrap_failures"], chart_ids: [], capability_cells: [capability] },
      ...(exact.hypothesis_tests ? [{ id: "bootstrap_hypothesis_tests", title: "Bootstrap hypothesis tests", table_ids: ["exact_case_bootstrap_hypothesis_tests"], chart_ids: [], capability_cells: [capability] }] : []),
      ...(studentized ? [{ id: "bootstrap_studentized_inference", title: "Studentized inference", table_ids: [
        "exact_case_bootstrap_studentized_summary", "exact_case_bootstrap_studentized_point_standard_errors",
        "exact_case_bootstrap_studentized_parameter_intervals", "exact_case_bootstrap_studentized_refit_standard_errors",
      ], chart_ids: [], capability_cells: [capability] }] : []),
      ...(bca ? [{ id: "bootstrap_bca_inference", title: "BCa inference", table_ids: [
        "exact_case_bootstrap_bca_summary", "exact_case_bootstrap_bca_parameter_intervals",
        "exact_case_bootstrap_bca_successful_delete_one_refits", "exact_case_bootstrap_bca_failures",
      ], chart_ids: [], capability_cells: [capability] }] : []),
    ],
    tables, charts: [], notices: [], exclusions: [], footnotes: [],
    presentation: { default_section_id: "bootstrap_inference", default_table_id: "exact_case_bootstrap_summary", precision: 4, missing_value_label: "-", chart_defaults: {} },
  };
  return { schemaVersion: 1, analyticalResult, canonicalDocument: document };
}

describe("Internal CB-SEM mean-replacement and result wire contracts", () => {
  function scoreLmBundleFixture() {
    return {
      method_version: "cbsem_cfa_score_lm_v1",
      scope: "covariance_only_declared_zero_residual_covariances",
      rows: [{
        parameter_id: "parameter:loading:x2",
        kind: "residual_covariance",
        lhs: "x1",
        rhs: "x2",
        outcome: {
          status: "available",
          score: 2,
          efficient_score: 2,
          candidate_information: 1,
          efficient_information: 1,
          modification_index: 4,
          expected_parameter_change: 2,
          p_value: cbsemCfaScoreLmChiSquare1PValueV1(4),
        },
      }],
    };
  }

  it("enforces Rust-compatible UTF-8 byte ordering for canonical missing markers", () => {
    const canonical = meanReplacementReceiptFixture();
    canonical.variables[0]!.canonical_missing_markers = ["\uE000", "\u{10000}"];
    expect(parseMeanReplacementReceiptV1(canonical).variables[0]!.canonical_missing_markers)
      .toEqual(["\uE000", "\u{10000}"]);

    const utf16Ordered = meanReplacementReceiptFixture();
    utf16Ordered.variables[0]!.canonical_missing_markers = ["\u{10000}", "\uE000"];
    expect(() => parseMeanReplacementReceiptV1(utf16Ordered)).toThrow(/sorted and deduplicated/);
  });

  it("accepts exact accounting and retains an all-missing modeled case", () => {
    const receipt = parseMeanReplacementReceiptV1(meanReplacementReceiptFixture());
    expect(receipt).toMatchObject({ retained_row_count: 20, omitted_row_count: 0, imputed_cell_count: 6 });
    expect(receipt.cases[0]).toEqual(expect.objectContaining({ missing_fraction: 1, high_missingness_warning: true }));
    expect(parseInternalRecipeV4CbsemExecutionResultV1(resultFixture()).estimation.input.missing_data_treatment).toEqual(receipt);
  });

  it("rejects drifted thresholds, warnings, unknown keys, and an uncomputable column mean", () => {
    const warning = meanReplacementReceiptFixture();
    warning.variables[0]!.warning_level = "none";
    expect(() => parseMeanReplacementReceiptV1(warning)).toThrow(/warning_level/);

    const caseWarning = meanReplacementReceiptFixture();
    caseWarning.cases[0]!.high_missingness_warning = false;
    expect(() => parseMeanReplacementReceiptV1(caseWarning)).toThrow(/high_missingness_warning/);

    const unknown = meanReplacementReceiptFixture() as ReturnType<typeof meanReplacementReceiptFixture> & { extra?: boolean };
    unknown.extra = true;
    expect(() => parseMeanReplacementReceiptV1(unknown)).toThrow(/unknown extra/);

    const noMean = meanReplacementReceiptFixture();
    noMean.variables[0]!.observed_count = 0;
    noMean.variables[0]!.missing_count = 20;
    noMean.variables[0]!.missing_fraction = 1;
    noMean.variables[0]!.warning_level = "above_fifteen_percent";
    expect(() => parseMeanReplacementReceiptV1(noMean)).toThrow(/backed by observed rows/);
  });

  it("cross-checks source identity and keeps listwise results receipt-free", () => {
    const drifted = resultFixture();
    drifted.estimation.input.missing_data_treatment.source_dataset_fingerprint = "other";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(drifted)).toThrow(/exact raw source identity/);

    const listwise = resultFixture();
    delete (listwise.estimation.input as { missing_data_treatment?: unknown }).missing_data_treatment;
    listwise.estimation.input.used_sample_size = 18;
    listwise.estimation.input.omitted_observations = 2;
    listwise.estimation.schema_version = 2;
    listwise.estimation.method_version = "cbsem_ml_exact_parameter_table_v3";
    listwise.provenance.adapter_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v5";
    listwise.provenance.moment_input_method_version = "cbsem_ml_exact_parameter_table_v3";
    expect(parseInternalRecipeV4CbsemExecutionResultV1(listwise).estimation.input.missing_data_treatment).toBeUndefined();

    const unknownResult = resultFixture() as ReturnType<typeof resultFixture> & { extra?: boolean };
    unknownResult.extra = true;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(unknownResult)).toThrow(/unknown extra/);
  });

  it("requires strict genuine score\/LM inference for adapters v8-v10", () => {
    const current = resultFixture();
    delete (current.estimation.input as { missing_data_treatment?: unknown }).missing_data_treatment;
    current.estimation.input.used_sample_size = 18;
    current.estimation.input.omitted_observations = 2;
    current.estimation.schema_version = 2;
    current.estimation.method_version = "cbsem_ml_exact_parameter_table_v3";
    current.provenance.moment_input_method_version = "cbsem_ml_exact_parameter_table_v3";
    current.provenance.adapter_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v8";
    Object.assign(current.estimation.analysis, {
      model_type: "cfa",
      mean_structure: false,
      modification_indices: [],
      score_lm: scoreLmBundleFixture(),
    });
    expect(parseInternalRecipeV4CbsemExecutionResultV1(current).estimation.analysis.score_lm)
      .toEqual(scoreLmBundleFixture());

    const arithmetic = structuredClone(scoreLmBundleFixture());
    arithmetic.rows[0]!.outcome.modification_index = 5;
    expect(() => parseCbsemCfaScoreLmBundleV1(arithmetic)).toThrow(/arithmetic/);

    const signedZero = structuredClone(scoreLmBundleFixture());
    signedZero.rows[0]!.outcome.score = -0;
    expect(() => parseCbsemCfaScoreLmBundleV1(signedZero)).toThrow(/positive zero/);

    const missing = structuredClone(current);
    delete (missing.estimation.analysis as { score_lm?: unknown }).score_lm;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(missing)).toThrow(/adapter v8 requires/);

    const injectedLegacy = structuredClone(current);
    injectedLegacy.provenance.adapter_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v5";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(injectedLegacy)).toThrow(/unavailable before adapter v8/);
  });

  it("accepts only the frozen v9/v10/v11 exact-bootstrap generation pairs", () => {
    const historical = exactBootstrapResultFixture("v9");
    const current = exactBootstrapResultFixture("v10");
    const studentized = exactBootstrapResultFixture("v11");

    expect(parseInternalRecipeV4CbsemExecutionResultV1(historical).estimation.analysis.exact_case_bootstrap)
      .not.toHaveProperty("hypothesis_tests");
    expect(parseInternalRecipeV4CbsemExecutionResultV1(current).estimation.analysis.exact_case_bootstrap)
      .toHaveProperty("hypothesis_tests.selected_test_tail", "two_sided");
    const parsedStudentized = parseInternalRecipeV4CbsemExecutionResultV1(studentized).estimation.analysis;
    expect(parsedStudentized).not.toHaveProperty("exact_case_bootstrap");
    expect(parsedStudentized.exact_case_bootstrap_studentized).toMatchObject({
      base: { hypothesis_tests: { selected_test_tail: "two_sided" } },
      studentized: {
        method_version: "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1",
        studentized_usable_replicates: 0,
        inference: { status: "unavailable", reason: "insufficient_studentized_usable_replicates" },
      },
    });
    const availableStudentized = availableStudentizedResultFixture();
    expect(parseInternalRecipeV4CbsemExecutionResultV1(availableStudentized).estimation.analysis)
      .toHaveProperty("exact_case_bootstrap_studentized.studentized.intervals.0.outcome.interval_lower", 0.5);

    const injectedHistorical = structuredClone(current);
    injectedHistorical.provenance.adapter_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v9";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(injectedHistorical)).toThrow(/historical adapter v9 must omit/);

    const missingCurrent = structuredClone(historical);
    missingCurrent.provenance.adapter_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(missingCurrent)).toThrow(/adapter v10 requires/);

    const injectedPreV9 = structuredClone(historical);
    injectedPreV9.provenance.adapter_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v8";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(injectedPreV9)).toThrow(/unavailable before adapter v9/);

    const driftedReason = structuredClone(current);
    const exact = (driftedReason.estimation.analysis as unknown as {
      exact_case_bootstrap: { hypothesis_tests: { parameters: Array<{ outcome: { reason: string } }> } };
    }).exact_case_bootstrap;
    exact.hypothesis_tests.parameters[0]!.outcome.reason = "unknown_reason";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(driftedReason)).toThrow(/unavailable reason/);

    const mixedV11 = structuredClone(studentized);
    const wrapper = (mixedV11.estimation.analysis as unknown as {
      exact_case_bootstrap_studentized: { base: unknown };
      exact_case_bootstrap?: unknown;
    }).exact_case_bootstrap_studentized;
    (mixedV11.estimation.analysis as unknown as { exact_case_bootstrap?: unknown }).exact_case_bootstrap = wrapper.base;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(mixedV11)).toThrow(/atomic exact_case_bootstrap_studentized wrapper/);

    const injectedV10 = structuredClone(current);
    (injectedV10.estimation.analysis as unknown as { exact_case_bootstrap_studentized?: unknown }).exact_case_bootstrap_studentized =
      structuredClone(wrapper);
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(injectedV10)).toThrow(/unavailable before adapter v11/);

    const unknownSidecar = structuredClone(studentized);
    const sidecar = (unknownSidecar.estimation.analysis as unknown as {
      exact_case_bootstrap_studentized: { studentized: Record<string, unknown> };
    }).exact_case_bootstrap_studentized.studentized;
    sidecar.injected_v10 = true;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(unknownSidecar)).toThrow(/unknown injected_v10/);

    const driftedPointOrder = structuredClone(studentized);
    const point = (driftedPointOrder.estimation.analysis as unknown as {
      exact_case_bootstrap_studentized: { studentized: { point_standard_errors: { outcome: { parameters: Array<{ parameter_id: string }> } } } };
    }).exact_case_bootstrap_studentized.studentized.point_standard_errors.outcome;
    point.parameters[0]!.parameter_id = "wrong";
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(driftedPointOrder)).toThrow(/parameter order/);

    const driftedPivot = availableStudentizedResultFixture();
    const driftedInterval = (driftedPivot.estimation.analysis as unknown as {
      exact_case_bootstrap_studentized: { studentized: { intervals: Array<{ outcome: { interval_lower: number } }> } };
    }).exact_case_bootstrap_studentized.studentized.intervals[0]!;
    driftedInterval.outcome.interval_lower = 0.5000000000000001;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(driftedPivot)).toThrow(/reversed Type-7 interval arithmetic/);

    const driftedRefitOrder = availableStudentizedResultFixture();
    const refitReceipts = (driftedRefitOrder.estimation.analysis as unknown as {
      exact_case_bootstrap_studentized: { studentized: { refit_standard_errors: Array<{ replicate_index: number }> } };
    }).exact_case_bootstrap_studentized.studentized.refit_standard_errors;
    refitReceipts[0]!.replicate_index = 1;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(driftedRefitOrder)).toThrow(/successful v10 base-refit ledger/);
  });

  it("binds exact-bootstrap bare fingerprints to the equivalent v2 resident identity only", () => {
    const versionedFingerprint = `v2:${"a".repeat(64)}`;
    for (const adapter of ["v10", "v11", "v12"] as const) {
      const result = exactBootstrapResultFixture(adapter);
      result.estimation.input.dataset_fingerprint = versionedFingerprint;
      result.provenance.compilation_receipt.analytical_identity_sha256 = "d".repeat(64);
      expect(() => parseInternalRecipeV4CbsemExecutionResultV1(result)).not.toThrow();

      const versionedCompilation = structuredClone(result);
      versionedCompilation.provenance.compilation_receipt.dataset_fingerprint = versionedFingerprint;
      expect(() => parseInternalRecipeV4CbsemExecutionResultV1(versionedCompilation)).not.toThrow();
    }

    const altered = exactBootstrapResultFixture("v10");
    altered.estimation.input.dataset_fingerprint = versionedFingerprint;
    (altered.estimation.analysis as unknown as {
      exact_case_bootstrap: { source_dataset_fingerprint: string };
    }).exact_case_bootstrap.source_dataset_fingerprint = "b".repeat(64);
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(altered))
      .toThrow(/does not bind the exact listwise CFA result identity/);

    const alteredCompilation = exactBootstrapResultFixture("v10");
    alteredCompilation.estimation.input.dataset_fingerprint = versionedFingerprint;
    alteredCompilation.provenance.compilation_receipt.dataset_fingerprint = "b".repeat(64);
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(alteredCompilation))
      .toThrow(/drift between compilation, moment-input, and estimator identities/);

    const alteredOuterRecipe = exactBootstrapResultFixture("v10");
    alteredOuterRecipe.provenance.compilation_receipt.recipe_analytical_sha256 = "b".repeat(64);
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(alteredOuterRecipe))
      .toThrow(/drift between compilation, moment-input, and estimator identities/);

    const pointOnly = resultFixture();
    pointOnly.estimation.input.dataset_fingerprint = `v2:${pointOnly.estimation.input.dataset_fingerprint}`;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(pointOnly))
      .toThrow(/does not bind the exact raw source identity/);
  });

  it("strictly owns the atomic v12 BCa sidecar and rejects older-adapter injection or ledger drift", () => {
    const current = exactBootstrapResultFixture("v12");
    const parsed = parseInternalRecipeV4CbsemExecutionResultV1(current).estimation.analysis;
    expect(parsed).not.toHaveProperty("exact_case_bootstrap");
    expect(parsed).not.toHaveProperty("exact_case_bootstrap_studentized");
    expect(parsed.exact_case_bootstrap_bca).toMatchObject({
      base: { hypothesis_tests: { selected_test_tail: "two_sided" } },
      bca: {
        method_version: "cbsem_exact_case_bootstrap_bca_interval_v1",
        delete_one_case_count: 18,
        inference: { status: "unavailable", reason: "base_inference_unavailable" },
      },
    });
    const available = availableBcaResultFixture();
    expect(parseInternalRecipeV4CbsemExecutionResultV1(available).estimation.analysis)
      .toHaveProperty("exact_case_bootstrap_bca.bca.intervals.0.outcome.interval_lower", 2);

    const arithmeticDrift = availableBcaResultFixture();
    const availableOutcome = (arithmeticDrift.estimation.analysis as unknown as {
      exact_case_bootstrap_bca: { bca: { intervals: Array<{ outcome: { interval_lower: number } }> } };
    }).exact_case_bootstrap_bca.bca.intervals[0]!.outcome;
    availableOutcome.interval_lower = 1.9;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(arithmeticDrift)).toThrow(/exposed Type-7 arithmetic/);

    const unknown = structuredClone(current);
    const unknownSidecar = (unknown.estimation.analysis as unknown as {
      exact_case_bootstrap_bca: { bca: Record<string, unknown> };
    }).exact_case_bootstrap_bca.bca;
    unknownSidecar.raw_delete_one_refits = [];
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(unknown)).toThrow(/unknown raw_delete_one_refits/);

    const mixed = structuredClone(current);
    const mixedWrapper = (mixed.estimation.analysis as unknown as {
      exact_case_bootstrap_bca: { base: unknown };
      exact_case_bootstrap?: unknown;
    }).exact_case_bootstrap_bca;
    (mixed.estimation.analysis as unknown as { exact_case_bootstrap?: unknown }).exact_case_bootstrap = mixedWrapper.base;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(mixed)).toThrow(/atomic exact_case_bootstrap_bca wrapper/);

    const injectedV11 = exactBootstrapResultFixture("v11");
    (injectedV11.estimation.analysis as unknown as { exact_case_bootstrap_bca?: unknown }).exact_case_bootstrap_bca =
      structuredClone((current.estimation.analysis as unknown as { exact_case_bootstrap_bca: unknown }).exact_case_bootstrap_bca);
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(injectedV11)).toThrow(/unavailable before adapter v12/);

    const identityDrift = structuredClone(current);
    const identitySidecar = (identityDrift.estimation.analysis as unknown as {
      exact_case_bootstrap_bca: { bca: { base_point_result_sha256: string } };
    }).exact_case_bootstrap_bca.bca;
    identitySidecar.base_point_result_sha256 = "f".repeat(64);
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(identityDrift)).toThrow(/differs from the atomic base authority/);

    const ledgerDrift = structuredClone(current);
    const successes = (ledgerDrift.estimation.analysis as unknown as {
      exact_case_bootstrap_bca: { bca: { successful_delete_one_refits: Array<{ omitted_complete_case_position: number }> } };
    }).exact_case_bootstrap_bca.bca.successful_delete_one_refits;
    successes[1]!.omitted_complete_case_position = 0;
    expect(() => parseInternalRecipeV4CbsemExecutionResultV1(ledgerDrift)).toThrow(/invalid omission order/);
  });

  it("bit-exactly binds v9/v10/v11 bootstrap tables and rejects canonical tampering", () => {
    const historical = completedExactBootstrapFixture("v9");
    const current = completedExactBootstrapFixture("v10");
    const studentized = completedExactBootstrapFixture("v11");
    current.analyticalResult.estimation.input.dataset_fingerprint = `v2:${current.analyticalResult.estimation.input.dataset_fingerprint}`;

    expect(parseInternalRecipeV4CbsemCompletedResultV1(historical).analyticalResult.provenance.adapter_version)
      .toBe("compiled_recipe_v4_cbsem_plan_v2_execution_v9");
    expect(parseInternalRecipeV4CbsemCompletedResultV1(current).canonicalDocument.sections)
      .toContainEqual(expect.objectContaining({ id: "bootstrap_hypothesis_tests" }));
    expect(parseInternalRecipeV4CbsemCompletedResultV1(studentized).canonicalDocument.sections)
      .toContainEqual(expect.objectContaining({
        id: "bootstrap_studentized_inference",
        table_ids: [
          "exact_case_bootstrap_studentized_summary",
          "exact_case_bootstrap_studentized_point_standard_errors",
          "exact_case_bootstrap_studentized_parameter_intervals",
          "exact_case_bootstrap_studentized_refit_standard_errors",
        ],
      }));

    const tamperedReceipt = structuredClone(current);
    const hypothesis = tamperedReceipt.canonicalDocument.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!;
    hypothesis.rows[0]!.cells[25] = { kind: "text", value: "unsupported_parameter_family" };
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(tamperedReceipt)).toThrow(/does not exactly bind the analytical hypothesis receipt/);

    const injectedHistorical = structuredClone(historical);
    const currentHypothesisTable = current.canonicalDocument.tables.find((table) => table.id === "exact_case_bootstrap_hypothesis_tests")!;
    const currentHypothesisSection = current.canonicalDocument.sections.find((section) => section.id === "bootstrap_hypothesis_tests")!;
    injectedHistorical.canonicalDocument.tables.push(structuredClone(currentHypothesisTable));
    injectedHistorical.canonicalDocument.sections.push(structuredClone(currentHypothesisSection));
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(injectedHistorical)).toThrow(/historical adapter v9 carries injected/);

    const tamperedEngine = structuredClone(current);
    tamperedEngine.canonicalDocument.provenance.engine_version = "compiled_recipe_v4_cbsem_plan_v2_execution_v9";
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(tamperedEngine)).toThrow(/drifted adapter, estimator, or dataset identity/);

    const tamperedFingerprint = structuredClone(current);
    tamperedFingerprint.canonicalDocument.provenance.dataset_fingerprint = "b".repeat(64);
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(tamperedFingerprint)).toThrow(/drifted adapter, estimator, or dataset identity/);

    const tamperedStudentized = structuredClone(studentized);
    const studentizedSummary = tamperedStudentized.canonicalDocument.tables.find((table) => table.id === "exact_case_bootstrap_studentized_summary")!;
    studentizedSummary.rows[0]!.cells[10] = { kind: "number", value: 1 };
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(tamperedStudentized)).toThrow(/analytical studentized summary/);

    const injectedStudentizedCanonical = structuredClone(current);
    injectedStudentizedCanonical.canonicalDocument.tables.push(...studentized.canonicalDocument.tables
      .filter((table) => table.id.startsWith("exact_case_bootstrap_studentized_"))
      .map((table) => structuredClone(table)));
    injectedStudentizedCanonical.canonicalDocument.sections.push(structuredClone(
      studentized.canonicalDocument.sections.find((section) => section.id === "bootstrap_studentized_inference")!,
    ));
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(injectedStudentizedCanonical)).toThrow(/injected newer-generation bootstrap artifacts/);
  });

  it("bit-exactly binds the v12 BCa four-table family and archive boundary", () => {
    const current = completedExactBootstrapFixture("v12");
    const parsed = parseInternalRecipeV4CbsemCompletedResultV1(current);
    expect(parsed.canonicalDocument.sections).toContainEqual(expect.objectContaining({
      id: "bootstrap_bca_inference",
      table_ids: [
        "exact_case_bootstrap_bca_summary",
        "exact_case_bootstrap_bca_parameter_intervals",
        "exact_case_bootstrap_bca_successful_delete_one_refits",
        "exact_case_bootstrap_bca_failures",
      ],
    }));
    const summary = parsed.canonicalDocument.tables.find((table) => table.id === "exact_case_bootstrap_bca_summary")!;
    expect(summary.rows[0]!.cells[15]).toEqual({
      kind: "text",
      value: "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1",
    });

    const sectionOrder = structuredClone(current);
    const section = sectionOrder.canonicalDocument.sections.find((candidate) => candidate.id === "bootstrap_bca_inference")!;
    section.table_ids.reverse();
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(sectionOrder)).toThrow(/drifted v12 BCa table ownership or order/);

    const summaryDrift = structuredClone(current);
    const driftedSummary = summaryDrift.canonicalDocument.tables.find((table) => table.id === "exact_case_bootstrap_bca_summary")!;
    driftedSummary.rows[0]!.cells[15] = { kind: "text", value: "raw_refit_replay" };
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(summaryDrift)).toThrow(/analytical BCa summary/);

    const deleteOneDrift = structuredClone(current);
    const successTable = deleteOneDrift.canonicalDocument.tables.find((table) => table.id === "exact_case_bootstrap_bca_successful_delete_one_refits")!;
    successTable.rows[0]!.cells[2] = { kind: "text", value: "f".repeat(64) };
    expect(() => parseInternalRecipeV4CbsemCompletedResultV1(deleteOneDrift)).toThrow(/successful delete-one ledger/);
  });
});
