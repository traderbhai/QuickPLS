use crate::recipe_v4_cbsem_execution::InternalRecipeV4CbsemExecutionRequestV1;
use qpls_core::{
    CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION, CBSEM_EXACT_BOOTSTRAP_CAPABILITY_ID,
    CBSEM_EXACT_BOOTSTRAP_CAPABILITY_VERSION, CBSEM_EXACT_BOOTSTRAP_CELL_ID,
    CanonicalChartDisplayOptions, CanonicalColumnRole, CanonicalColumnType, CanonicalMissingReason,
    CanonicalNoticeSeverity, CanonicalResultCell, CanonicalResultColumn, CanonicalResultDocumentV2,
    CanonicalResultExclusion, CanonicalResultNotice, CanonicalResultPresentationV2,
    CanonicalResultProvenanceV2, CanonicalResultRow, CanonicalResultSection, CanonicalResultTable,
    CbsemBootstrapInterval, CbsemBootstrapTestTail, MethodConfig,
    validate_canonical_result_document_v2,
};
use qpls_estimation::{
    CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1, CBSEM_CFA_SCORE_LM_SCOPE_V1,
    CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
    CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3, CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2, CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1,
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3, CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
    CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1, CBSEM_FIT_METHOD_VERSION,
    CbsemCfaScoreLmBundleV1, CbsemCfaScoreLmOutcomeV1, CbsemCfaScoreLmUnavailableReasonV1,
    CbsemExactCaseBootstrapBcaInferenceV1, CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1,
    CbsemExactCaseBootstrapBcaSidecarV1, CbsemExactCaseBootstrapBcaUnavailableReasonV1,
    CbsemExactCaseBootstrapFailureKindV1, CbsemExactCaseBootstrapHypothesisTestInferenceV1,
    CbsemExactCaseBootstrapHypothesisTestOutcomeV1,
    CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1,
    CbsemExactCaseBootstrapHypothesisTestsV1, CbsemExactCaseBootstrapInferenceV1,
    CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1, CbsemExactCaseBootstrapResultV1,
    CbsemExactCaseBootstrapStudentizedInferenceV1,
    CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1,
    CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapStudentizedSidecarV1, MeanReplacementReceiptV1,
    MeanReplacementWarningLevelV1,
};
use qpls_resampling::CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1;
use qpls_runner::{
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2, RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V3,
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V4, RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5,
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6, RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7,
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8, RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9,
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10, RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11,
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12, RECIPE_V4_CBSEM_EXECUTION_RESULT_SCHEMA_VERSION,
    RecipeV4CbsemExecutionResultV1,
};
use uuid::Uuid;

const CBSEM_CAPABILITY_ID: &str = "smartpls.cbsem";
const CBSEM_CAPABILITY_CELL_ID: &str = "qpls3.cbsem.ml";
const CBSEM_CAPABILITY_VERSION: &str = "cbsem_ml_v1";
const CBSEM_EXACT_BOOTSTRAP_SECTION_ID: &str = "bootstrap_inference";
const CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID: &str = "exact_case_bootstrap_summary";
const CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID: &str = "exact_case_bootstrap_parameter_intervals";
const CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID: &str = "exact_case_bootstrap_successful_refits";
const CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID: &str = "exact_case_bootstrap_failures";
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID: &str = "bootstrap_hypothesis_tests";
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID: &str = "exact_case_bootstrap_hypothesis_tests";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID: &str = "bootstrap_studentized_inference";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_summary";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_point_standard_errors";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_parameter_intervals";
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID: &str =
    "exact_case_bootstrap_studentized_refit_standard_errors";
const CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID: &str = "bootstrap_bca_inference";
const CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID: &str = "exact_case_bootstrap_bca_summary";
const CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID: &str =
    "exact_case_bootstrap_bca_parameter_intervals";
const CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID: &str =
    "exact_case_bootstrap_bca_successful_delete_one_refits";
const CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID: &str = "exact_case_bootstrap_bca_failures";
const CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE: &str =
    "ledger_identity_digest_and_arithmetic_replay_only_no_raw_base_or_delete_one_ml_replay_v1";
const CBSEM_EXACT_BOOTSTRAP_ARCHIVE_SCOPE: &str =
    "schedule_and_arithmetic_only_no_raw_refit_replay_or_source_row_digest_recomputation";
const CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS: &[&str] = &[
    "method_version",
    "estimator_method_version",
    "source_dataset_id",
    "source_dataset_fingerprint",
    "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256",
    "compiler_analytical_identity_sha256",
    "plan_sha256",
    "model_scientific_sha256",
    "complete_case_sample_size",
    "complete_case_universe_digest_method",
    "complete_case_universe_sha256",
    "covariance_denominator",
    "sample_indices_digest_method",
    "sampling_positions_digest_method",
    "interval_method",
    "confidence_level",
    "requested_replicates",
    "attempted_refits",
    "usable_replicates",
    "failed_replicates",
    "minimum_usable_fraction",
    "minimum_usable_replicates",
    "seed_decimal",
    "stream_token",
    "retry_policy",
    "max_attempts_per_replicate",
    "parameter_ids_json",
    "inference_status",
    "unavailable_reason_code",
    "unavailable_message",
    "archive_validation_scope",
];
const CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS: &[&str] = &[
    "parameter_id",
    "original",
    "bootstrap_mean",
    "bias",
    "standard_error",
    "percentile_lower",
    "percentile_upper",
    "usable_replicates",
];
const CBSEM_EXACT_BOOTSTRAP_REFIT_COLUMNS: &[&str] = &[
    "replicate_index",
    "sampling_positions_sha256",
    "sample_indices_sha256",
    "parameter_estimates_json",
    "iterations",
    "objective",
    "gradient_norm",
];
const CBSEM_EXACT_BOOTSTRAP_FAILURE_COLUMNS: &[&str] = &[
    "replicate_index",
    "sampling_positions_sha256",
    "sample_indices_sha256",
    "kind",
    "message",
];
const CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS: &[&str] = &[
    "method_version",
    "null_hypothesis",
    "statistic",
    "tie_policy",
    "probability_method",
    "decision_rule",
    "selected_test_tail",
    "null_value",
    "significance_level",
    "usable_replicates",
    "inference_status",
    "global_unavailable_reason_code",
    "global_unavailable_message",
    "parameter_id",
    "parameter_status",
    "point_estimate",
    "two_sided_exceedances",
    "greater_or_equal_exceedances",
    "less_or_equal_exceedances",
    "p_value_two_sided",
    "p_value_greater",
    "p_value_less",
    "selected_exceedances",
    "selected_p_value",
    "reject_null",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS: &[&str] = &[
    "method_version",
    "standard_error_method_version",
    "expected_information_method",
    "pivot_method",
    "quantile_method",
    "interval_method",
    "archive_validation_scope",
    "confidence_level",
    "minimum_usable_fraction",
    "minimum_usable_replicates",
    "studentized_usable_replicates",
    "parameter_ids_json",
    "inference_status",
    "unavailable_reason_code",
    "unavailable_message",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS: &[&str] = &[
    "method_version",
    "parameter_id",
    "status",
    "information_method",
    "standard_error",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS: &[&str] = &[
    "parameter_id",
    "status",
    "point_estimate",
    "point_standard_error",
    "lower_pivot_quantile",
    "upper_pivot_quantile",
    "interval_lower",
    "interval_upper",
    "usable_replicates",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS: &[&str] = &[
    "replicate_index",
    "status",
    "information_method",
    "standard_errors_json",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS: &[&str] = &[
    "method_version",
    "base_bootstrap_method_version",
    "outer_recipe_analytical_identity_sha256",
    "base_point_result_sha256",
    "compiler_analytical_identity_sha256",
    "plan_sha256",
    "model_scientific_sha256",
    "delete_one_refit_method_version",
    "delete_one_sampling_positions_digest_method",
    "delete_one_sample_indices_digest_method",
    "bias_correction_method",
    "acceleration_method",
    "adjusted_probability_method",
    "quantile_method",
    "retry_policy",
    "archive_validation_scope",
    "confidence_level",
    "bootstrap_usable_replicates",
    "minimum_bootstrap_usable_replicates",
    "delete_one_case_count",
    "successful_delete_one_refits",
    "failed_delete_one_refits",
    "parameter_ids_json",
    "inference_status",
    "unavailable_reason_code",
    "unavailable_message",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS: &[&str] = &[
    "parameter_id",
    "status",
    "point_estimate",
    "bias_correction",
    "acceleration",
    "adjusted_lower_probability",
    "adjusted_upper_probability",
    "interval_lower",
    "interval_upper",
    "usable_replicates",
    "unavailable_reason",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS: &[&str] = &[
    "omitted_complete_case_position",
    "omitted_source_row_index",
    "retained_sampling_positions_sha256",
    "retained_sample_indices_sha256",
    "parameter_estimates_json",
    "iterations",
    "objective",
    "gradient_norm",
];
const CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS: &[&str] = &[
    "omitted_complete_case_position",
    "omitted_source_row_index",
    "retained_sampling_positions_sha256",
    "retained_sample_indices_sha256",
    "kind",
    "message",
];
#[cfg(test)]
const CBSEM_HISTORICAL_FIT_COLUMNS: &[&str] = &[
    "chi_square",
    "degrees_of_freedom",
    "p_value",
    "cfi",
    "tli",
    "rmsea",
    "srmr",
    "aic",
    "bic",
];
#[cfg(test)]
const CBSEM_CURRENT_FIT_COLUMNS: &[&str] = &[
    "fit_method_version",
    "chi_square",
    "degrees_of_freedom",
    "p_value",
    "cfi",
    "tli",
    "rmsea",
    "rmsea_interval_method_version",
    "rmsea_interval_confidence_level",
    "rmsea_ci_lower",
    "rmsea_ci_upper",
    "srmr",
    "aic",
    "bic",
];

fn validate_execution_method_identity(
    result: &RecipeV4CbsemExecutionResultV1,
) -> Result<(), String> {
    if result.schema_version() != RECIPE_V4_CBSEM_EXECUTION_RESULT_SCHEMA_VERSION {
        return Err(
            "CB-SEM execution result schema version is not the expected v1 contract".into(),
        );
    }
    if !result.estimation().analysis.modification_indices.is_empty() {
        return Err(
            "exact Recipe-v4 CB-SEM cannot substitute legacy heuristic modification indices".into(),
        );
    }
    let (adapter, estimator, moment, schema) = if result
        .estimation()
        .input
        .missing_data_treatment
        .is_some()
    {
        if result.estimation().analysis.mean_structure
            || result.estimation().analysis.score_lm.is_some()
            || result.estimation().analysis.exact_case_bootstrap.is_some()
            || result
                .estimation()
                .analysis
                .exact_case_bootstrap_studentized
                .is_some()
            || result
                .estimation()
                .analysis
                .exact_case_bootstrap_bca
                .is_some()
        {
            return Err(
                    "CB-SEM mean replacement cannot carry mean-structure, score/LM, or exact case-bootstrap output"
                        .into(),
                );
        }
        (
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
            CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
        )
    } else if result.estimation().analysis.mean_structure {
        if result.estimation().analysis.score_lm.is_some()
            || result.estimation().analysis.exact_case_bootstrap.is_some()
            || result
                .estimation()
                .analysis
                .exact_case_bootstrap_studentized
                .is_some()
            || result
                .estimation()
                .analysis
                .exact_case_bootstrap_bca
                .is_some()
        {
            return Err(
                "CB-SEM mean structure cannot carry score/LM or exact case-bootstrap output".into(),
            );
        }
        (
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
        )
    } else {
        let adapter = if let Some(score_lm) = result.estimation().analysis.score_lm.as_ref() {
            if score_lm.method_version != CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1
                || score_lm.scope != CBSEM_CFA_SCORE_LM_SCOPE_V1
            {
                return Err("CB-SEM score/LM method or scope identity drifted".into());
            }
            let legacy = result.estimation().analysis.exact_case_bootstrap.as_ref();
            let studentized = result
                .estimation()
                .analysis
                .exact_case_bootstrap_studentized
                .as_ref();
            let bca = result
                .estimation()
                .analysis
                .exact_case_bootstrap_bca
                .as_ref();
            if usize::from(legacy.is_some())
                + usize::from(studentized.is_some())
                + usize::from(bca.is_some())
                > 1
            {
                return Err(
                    "CB-SEM exact case bootstrap ownership is not atomic for the selected adapter"
                        .into(),
                );
            }
            if let Some(bootstrap) = legacy {
                if bootstrap.hypothesis_tests.is_none() {
                    return Err(
                        "current CB-SEM exact bootstrap omits selected-tail inference".into(),
                    );
                }
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
            } else if let Some(wrapper) = studentized {
                if wrapper.base.hypothesis_tests.is_none() {
                    return Err(
                        "studentized CB-SEM exact bootstrap omits selected-tail inference".into(),
                    );
                }
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
            } else if let Some(wrapper) = bca {
                if wrapper.base.hypothesis_tests.is_none() {
                    return Err("BCa CB-SEM exact bootstrap omits selected-tail inference".into());
                }
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
            } else {
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8
            }
        } else if result.estimation().analysis.exact_case_bootstrap.is_some()
            || result
                .estimation()
                .analysis
                .exact_case_bootstrap_studentized
                .is_some()
            || result
                .estimation()
                .analysis
                .exact_case_bootstrap_bca
                .is_some()
        {
            return Err("CB-SEM exact case bootstrap requires the v8 score/LM point result".into());
        } else {
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5
        };
        (
            adapter,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
        )
    };
    if result.provenance().adapter_version() != adapter {
        return Err(
            "CB-SEM execution adapter identity does not match its analytical method".into(),
        );
    }
    if result.provenance().moment_input_method_version() != moment
        || result.estimation().method_version != moment
    {
        return Err("CB-SEM moment-input identity does not match its analytical method".into());
    }
    if result.provenance().estimator_method_version() != estimator
        || result.estimation().analysis.method_version != estimator
    {
        return Err("CB-SEM estimator identity does not match its analytical method".into());
    }
    if result.estimation().schema_version != schema {
        return Err("CB-SEM compiled-moment result schema does not match its method".into());
    }
    Ok(())
}

fn archived_number_cell(
    table: &qpls_project::CanonicalResultTableV2,
    row_id: &str,
    column_id: &str,
) -> Option<f64> {
    let column_index = table
        .columns
        .iter()
        .position(|column| column.id == column_id)?;
    let row = table.rows.iter().find(|row| row.id == row_id)?;
    match row.cells.get(column_index)? {
        qpls_project::CanonicalResultCellV2::Number { value, .. } => Some(*value),
        _ => None,
    }
}

fn archived_boolean_cell(
    table: &qpls_project::CanonicalResultTableV2,
    row_id: &str,
    column_id: &str,
) -> Option<bool> {
    let column_index = table
        .columns
        .iter()
        .position(|column| column.id == column_id)?;
    let row = table.rows.iter().find(|row| row.id == row_id)?;
    match row.cells.get(column_index)? {
        qpls_project::CanonicalResultCellV2::Boolean { value } => Some(*value),
        _ => None,
    }
}

fn archived_text_cell<'a>(
    table: &'a qpls_project::CanonicalResultTableV2,
    row_id: &str,
    column_id: &str,
) -> Option<&'a str> {
    let column_index = table
        .columns
        .iter()
        .position(|column| column.id == column_id)?;
    let row = table.rows.iter().find(|row| row.id == row_id)?;
    match row.cells.get(column_index)? {
        qpls_project::CanonicalResultCellV2::Text { value } => Some(value),
        _ => None,
    }
}

fn archived_not_applicable_cell(
    table: &qpls_project::CanonicalResultTableV2,
    row_id: &str,
    column_id: &str,
) -> bool {
    let Some(column_index) = table
        .columns
        .iter()
        .position(|column| column.id == column_id)
    else {
        return false;
    };
    let Some(row) = table.rows.iter().find(|row| row.id == row_id) else {
        return false;
    };
    matches!(
        row.cells.get(column_index),
        Some(qpls_project::CanonicalResultCellV2::Missing {
            reason: qpls_project::CanonicalMissingReasonV2::NotApplicable,
            display: None,
        })
    )
}

fn archived_exact_finite_number(
    table: &qpls_project::CanonicalResultTableV2,
    row_id: &str,
    column_id: &str,
) -> Option<f64> {
    let column_index = table
        .columns
        .iter()
        .position(|column| column.id == column_id)?;
    let row = table.rows.iter().find(|row| row.id == row_id)?;
    match row.cells.get(column_index)? {
        qpls_project::CanonicalResultCellV2::Number {
            value,
            display: None,
        } if value.is_finite() && value.to_bits() != (-0.0_f64).to_bits() => Some(*value),
        _ => None,
    }
}

fn validate_archived_studentized_bootstrap_artifacts(
    document: &qpls_project::CanonicalResultDocumentV2,
    adapter: &str,
) -> Result<(), String> {
    let table_ids = [
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
    ];
    let tables = document
        .tables
        .iter()
        .filter(|table| table_ids.contains(&table.id.as_str()))
        .collect::<Vec<_>>();
    let sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID)
        .collect::<Vec<_>>();
    if adapter != RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11 {
        if !tables.is_empty() || !sections.is_empty() {
            return Err("CB-SEM v2-v10/v12 adapter carries studentized bootstrap artifacts".into());
        }
        return Ok(());
    }
    let [section] = sections.as_slice() else {
        return Err("CB-SEM v11 requires exactly one studentized-inference section".into());
    };
    if tables.len() != table_ids.len()
        || section.table_ids.iter().map(String::as_str).ne(table_ids)
        || !section.chart_ids.is_empty()
    {
        return Err("CB-SEM v11 studentized table ownership or order drifted".into());
    }
    let table = |id: &str| {
        tables
            .iter()
            .copied()
            .find(|table| table.id == id)
            .ok_or_else(|| format!("CB-SEM v11 is missing {id}"))
    };
    for (id, columns) in [
        (
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS,
        ),
        (
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS,
        ),
        (
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS,
        ),
        (
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS,
        ),
    ] {
        let candidate = table(id)?;
        if candidate
            .columns
            .iter()
            .map(|column| column.id.as_str())
            .ne(columns.iter().copied())
            || candidate
                .rows
                .iter()
                .any(|row| row.cells.len() != columns.len())
        {
            return Err(format!("CB-SEM v11 {id} columns or row width drifted"));
        }
    }

    let summary = table(CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID)?;
    if summary.rows.len() != 1
        || summary.rows[0].id != "bootstrap_studentized"
        || archived_text_cell(summary, "bootstrap_studentized", "method_version")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1)
        || archived_text_cell(
            summary,
            "bootstrap_studentized",
            "standard_error_method_version",
        ) != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1)
        || archived_text_cell(
            summary,
            "bootstrap_studentized",
            "expected_information_method",
        ) != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1)
        || archived_text_cell(summary, "bootstrap_studentized", "pivot_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1)
        || archived_text_cell(summary, "bootstrap_studentized", "quantile_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1)
        || archived_text_cell(summary, "bootstrap_studentized", "interval_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1)
        || archived_text_cell(summary, "bootstrap_studentized", "archive_validation_scope")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1)
    {
        return Err("CB-SEM v11 studentized summary identity drifted".into());
    }
    let parameter_ids_text =
        archived_text_cell(summary, "bootstrap_studentized", "parameter_ids_json")
            .ok_or_else(|| "CB-SEM v11 studentized parameter IDs are missing".to_owned())?;
    let parameter_ids = serde_json::from_str::<Vec<String>>(parameter_ids_text)
        .ok()
        .filter(|ids| serde_json::to_string(ids).ok().as_deref() == Some(parameter_ids_text))
        .filter(|ids| {
            !ids.is_empty()
                && ids.iter().all(|id| !id.trim().is_empty())
                && ids.iter().collect::<std::collections::BTreeSet<_>>().len() == ids.len()
        })
        .ok_or_else(|| "CB-SEM v11 studentized parameter IDs are noncanonical".to_owned())?;
    let base_summary = document
        .tables
        .iter()
        .find(|candidate| candidate.id == CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)
        .ok_or_else(|| "CB-SEM v11 studentized sidecar omits its base summary".to_owned())?;
    for (studentized_column, base_column) in [
        ("confidence_level", "confidence_level"),
        ("minimum_usable_fraction", "minimum_usable_fraction"),
        ("minimum_usable_replicates", "minimum_usable_replicates"),
    ] {
        if archived_exact_finite_number(summary, "bootstrap_studentized", studentized_column)
            .map(f64::to_bits)
            != archived_exact_finite_number(base_summary, "bootstrap", base_column)
                .map(f64::to_bits)
        {
            return Err("CB-SEM v11 studentized thresholds differ from the base ledger".into());
        }
    }
    if archived_text_cell(base_summary, "bootstrap", "parameter_ids_json")
        != Some(parameter_ids_text)
    {
        return Err("CB-SEM v11 studentized parameter order differs from the base ledger".into());
    }
    let studentized_usable = archived_exact_finite_number(
        summary,
        "bootstrap_studentized",
        "studentized_usable_replicates",
    )
    .filter(|value| value.fract() == 0.0 && *value >= 0.0)
    .ok_or_else(|| "CB-SEM v11 studentized usable count is invalid".to_owned())?;
    match archived_text_cell(summary, "bootstrap_studentized", "inference_status") {
        Some("available") => {
            if !archived_not_applicable_cell(
                summary,
                "bootstrap_studentized",
                "unavailable_reason_code",
            ) || !archived_not_applicable_cell(
                summary,
                "bootstrap_studentized",
                "unavailable_message",
            ) {
                return Err("available CB-SEM v11 studentized summary carries a reason".into());
            }
        }
        Some("unavailable") => {
            if !matches!(
                archived_text_cell(summary, "bootstrap_studentized", "unavailable_reason_code"),
                Some(
                    "point_standard_errors_unavailable"
                        | "insufficient_studentized_usable_replicates"
                )
            ) || !matches!(
                archived_text_cell(summary, "bootstrap_studentized", "unavailable_message"),
                Some(message) if !message.trim().is_empty()
            ) {
                return Err("unavailable CB-SEM v11 studentized summary omits its reason".into());
            }
        }
        _ => return Err("CB-SEM v11 studentized inference status drifted".into()),
    }

    let refit_reason = |value: Option<&str>| {
        matches!(
            value,
            Some(
                "singular_information"
                    | "information_not_positive_definite"
                    | "invalid_information_variance_or_standard_error"
                    | "derivative_unavailable"
                    | "numerical_information_failure"
            )
        )
    };
    let point = table(CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID)?;
    if point.rows.len() != parameter_ids.len() {
        return Err("CB-SEM v11 point standard-error cardinality drifted".into());
    }
    for (index, (row, parameter_id)) in point.rows.iter().zip(&parameter_ids).enumerate() {
        if row.id != format!("bootstrap_studentized_point_standard_error_{index:04}")
            || archived_text_cell(point, &row.id, "method_version")
                != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1)
            || archived_text_cell(point, &row.id, "parameter_id") != Some(parameter_id)
        {
            return Err("CB-SEM v11 point standard-error order drifted".into());
        }
        match archived_text_cell(point, &row.id, "status") {
            Some("available") => {
                if archived_text_cell(point, &row.id, "information_method")
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1)
                    || archived_exact_finite_number(point, &row.id, "standard_error")
                        .is_none_or(|value| value <= 0.0)
                    || !archived_not_applicable_cell(point, &row.id, "unavailable_reason")
                {
                    return Err("available CB-SEM v11 point standard error is invalid".into());
                }
            }
            Some("unavailable") => {
                if !archived_not_applicable_cell(point, &row.id, "information_method")
                    || !archived_not_applicable_cell(point, &row.id, "standard_error")
                    || !refit_reason(archived_text_cell(point, &row.id, "unavailable_reason"))
                {
                    return Err("unavailable CB-SEM v11 point standard error is invalid".into());
                }
            }
            _ => return Err("CB-SEM v11 point standard-error status drifted".into()),
        }
    }

    let intervals = table(CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID)?;
    if intervals.rows.len() != parameter_ids.len() {
        return Err("CB-SEM v11 studentized interval cardinality drifted".into());
    }
    for (index, (row, parameter_id)) in intervals.rows.iter().zip(&parameter_ids).enumerate() {
        if row.id != format!("bootstrap_studentized_interval_{index:04}")
            || archived_text_cell(intervals, &row.id, "parameter_id") != Some(parameter_id)
        {
            return Err("CB-SEM v11 studentized interval order drifted".into());
        }
        match archived_text_cell(intervals, &row.id, "status") {
            Some("available") => {
                if [
                    "point_estimate",
                    "point_standard_error",
                    "lower_pivot_quantile",
                    "upper_pivot_quantile",
                    "interval_lower",
                    "interval_upper",
                    "usable_replicates",
                ]
                .iter()
                .any(|column| archived_exact_finite_number(intervals, &row.id, column).is_none())
                    || archived_exact_finite_number(intervals, &row.id, "usable_replicates")
                        != Some(studentized_usable)
                    || !archived_not_applicable_cell(intervals, &row.id, "unavailable_reason")
                {
                    return Err("available CB-SEM v11 studentized interval is invalid".into());
                }
            }
            Some("unavailable") => {
                if [
                    "point_estimate",
                    "point_standard_error",
                    "lower_pivot_quantile",
                    "upper_pivot_quantile",
                    "interval_lower",
                    "interval_upper",
                    "usable_replicates",
                ]
                .iter()
                .any(|column| !archived_not_applicable_cell(intervals, &row.id, column))
                    || !matches!(
                        archived_text_cell(intervals, &row.id, "unavailable_reason"),
                        Some(
                            "point_standard_errors_unavailable"
                                | "insufficient_studentized_usable_replicates"
                        )
                    )
                {
                    return Err("unavailable CB-SEM v11 studentized interval is invalid".into());
                }
            }
            _ => return Err("CB-SEM v11 studentized interval status drifted".into()),
        }
    }

    let base_refits = document
        .tables
        .iter()
        .find(|candidate| candidate.id == CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID)
        .ok_or_else(|| "CB-SEM v11 studentized sidecar omits its base refits".to_owned())?;
    let refits = table(CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID)?;
    if refits.rows.len() != base_refits.rows.len() {
        return Err("CB-SEM v11 refit standard-error cardinality drifted".into());
    }
    let mut available_refits = 0_u32;
    for (row, base_row) in refits.rows.iter().zip(&base_refits.rows) {
        let replicate = archived_exact_finite_number(base_refits, &base_row.id, "replicate_index")
            .filter(|value| value.fract() == 0.0 && *value >= 0.0)
            .ok_or_else(|| "CB-SEM v11 base refit index is invalid".to_owned())?;
        let replicate_index = replicate as u32;
        if row.id != format!("bootstrap_studentized_refit_standard_error_{replicate_index:05}")
            || archived_exact_finite_number(refits, &row.id, "replicate_index") != Some(replicate)
        {
            return Err("CB-SEM v11 refit standard-error order drifted".into());
        }
        match archived_text_cell(refits, &row.id, "status") {
            Some("available") => {
                let encoded = archived_text_cell(refits, &row.id, "standard_errors_json")
                    .ok_or_else(|| "CB-SEM v11 refit standard errors are missing".to_owned())?;
                let values = serde_json::from_str::<Vec<f64>>(encoded)
                    .ok()
                    .filter(|values| serde_json::to_string(values).ok().as_deref() == Some(encoded))
                    .filter(|values| {
                        values.len() == parameter_ids.len()
                            && values.iter().all(|value| {
                                value.is_finite()
                                    && *value > 0.0
                                    && value.to_bits() != (-0.0_f64).to_bits()
                            })
                    })
                    .ok_or_else(|| {
                        "CB-SEM v11 refit standard errors are noncanonical".to_owned()
                    })?;
                let _ = values;
                if archived_text_cell(refits, &row.id, "information_method")
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1)
                    || !archived_not_applicable_cell(refits, &row.id, "unavailable_reason")
                {
                    return Err("available CB-SEM v11 refit standard error is invalid".into());
                }
                available_refits += 1;
            }
            Some("unavailable") => {
                if !archived_not_applicable_cell(refits, &row.id, "information_method")
                    || !archived_not_applicable_cell(refits, &row.id, "standard_errors_json")
                    || !refit_reason(archived_text_cell(refits, &row.id, "unavailable_reason"))
                {
                    return Err("unavailable CB-SEM v11 refit standard error is invalid".into());
                }
            }
            _ => return Err("CB-SEM v11 refit standard-error status drifted".into()),
        }
    }
    if f64::from(available_refits).to_bits() != studentized_usable.to_bits() {
        return Err("CB-SEM v11 studentized usable count differs from its refit receipts".into());
    }
    Ok(())
}

fn validate_archived_bca_bootstrap_artifacts(
    document: &qpls_project::CanonicalResultDocumentV2,
    adapter: &str,
) -> Result<(), String> {
    let table_ids = [
        CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID,
    ];
    let tables = document
        .tables
        .iter()
        .filter(|table| table_ids.contains(&table.id.as_str()))
        .collect::<Vec<_>>();
    let sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID)
        .collect::<Vec<_>>();
    if adapter != RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12 {
        if !tables.is_empty() || !sections.is_empty() {
            return Err("CB-SEM v2-v11 adapter carries BCa bootstrap artifacts".into());
        }
        return Ok(());
    }
    let [section] = sections.as_slice() else {
        return Err("CB-SEM v12 requires exactly one BCa-inference section".into());
    };
    if tables.len() != table_ids.len()
        || section.table_ids.iter().map(String::as_str).ne(table_ids)
        || !section.chart_ids.is_empty()
    {
        return Err("CB-SEM v12 BCa table ownership or order drifted".into());
    }
    let table = |id: &str| {
        tables
            .iter()
            .copied()
            .find(|table| table.id == id)
            .ok_or_else(|| format!("CB-SEM v12 is missing {id}"))
    };
    for (id, columns) in [
        (
            CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS,
        ),
        (
            CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS,
        ),
        (
            CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS,
        ),
        (
            CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS,
        ),
    ] {
        let candidate = table(id)?;
        if candidate
            .columns
            .iter()
            .map(|column| column.id.as_str())
            .ne(columns.iter().copied())
            || candidate
                .rows
                .iter()
                .any(|row| row.cells.len() != columns.len())
        {
            return Err(format!("CB-SEM v12 {id} columns or row width drifted"));
        }
    }

    let summary = table(CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID)?;
    if summary.rows.len() != 1 || summary.rows[0].id != "bootstrap_bca" {
        return Err("CB-SEM v12 BCa summary identity drifted".into());
    }
    let text_at = |column: &str| archived_text_cell(summary, "bootstrap_bca", column);
    let count_at = |column: &str| {
        archived_exact_finite_number(summary, "bootstrap_bca", column)
            .filter(|value| value.fract() == 0.0 && *value >= 0.0)
            .map(|value| value as usize)
    };
    if text_at("method_version") != Some(CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1)
        || text_at("delete_one_refit_method_version")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1)
        || text_at("delete_one_sampling_positions_digest_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1)
        || text_at("delete_one_sample_indices_digest_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1)
        || text_at("bias_correction_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1)
        || text_at("acceleration_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2)
        || text_at("adjusted_probability_method")
            != Some(CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2)
        || text_at("quantile_method") != Some(CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1)
        || text_at("retry_policy") != Some(CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1)
        || text_at("archive_validation_scope") != Some(CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE)
    {
        return Err("CB-SEM v12 BCa method or archive scope identity drifted".into());
    }
    for column in [
        "outer_recipe_analytical_identity_sha256",
        "base_point_result_sha256",
        "compiler_analytical_identity_sha256",
        "plan_sha256",
        "model_scientific_sha256",
    ] {
        if !matches!(text_at(column), Some(value) if is_lowercase_sha256(value)) {
            return Err(format!(
                "CB-SEM v12 BCa summary {column} is not lowercase SHA-256"
            ));
        }
    }
    let base = document
        .tables
        .iter()
        .find(|candidate| candidate.id == CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)
        .ok_or_else(|| "CB-SEM v12 BCa sidecar omits its base summary".to_owned())?;
    for (bca_column, base_column) in [
        ("base_bootstrap_method_version", "method_version"),
        (
            "outer_recipe_analytical_identity_sha256",
            "outer_recipe_analytical_identity_sha256",
        ),
        ("base_point_result_sha256", "base_point_result_sha256"),
        (
            "compiler_analytical_identity_sha256",
            "compiler_analytical_identity_sha256",
        ),
        ("plan_sha256", "plan_sha256"),
        ("model_scientific_sha256", "model_scientific_sha256"),
        ("parameter_ids_json", "parameter_ids_json"),
    ] {
        if text_at(bca_column) != archived_text_cell(base, "bootstrap", base_column) {
            return Err("CB-SEM v12 BCa authority differs from its base ledger".into());
        }
    }
    for (bca_column, base_column) in [
        ("confidence_level", "confidence_level"),
        ("bootstrap_usable_replicates", "usable_replicates"),
        (
            "minimum_bootstrap_usable_replicates",
            "minimum_usable_replicates",
        ),
        ("delete_one_case_count", "complete_case_sample_size"),
    ] {
        if archived_exact_finite_number(summary, "bootstrap_bca", bca_column).map(f64::to_bits)
            != archived_exact_finite_number(base, "bootstrap", base_column).map(f64::to_bits)
        {
            return Err("CB-SEM v12 BCa numeric authority differs from its base ledger".into());
        }
    }
    let parameter_ids_text = text_at("parameter_ids_json")
        .ok_or_else(|| "CB-SEM v12 BCa parameter IDs are missing".to_owned())?;
    let parameter_ids = serde_json::from_str::<Vec<String>>(parameter_ids_text)
        .ok()
        .filter(|ids| serde_json::to_string(ids).ok().as_deref() == Some(parameter_ids_text))
        .filter(|ids| {
            !ids.is_empty()
                && ids.iter().all(|id| !id.trim().is_empty())
                && ids.iter().collect::<std::collections::BTreeSet<_>>().len() == ids.len()
        })
        .ok_or_else(|| "CB-SEM v12 BCa parameter IDs are noncanonical".to_owned())?;
    let case_count = count_at("delete_one_case_count")
        .ok_or_else(|| "CB-SEM v12 BCa delete-one case count is invalid".to_owned())?;
    let successful_count = count_at("successful_delete_one_refits")
        .ok_or_else(|| "CB-SEM v12 BCa successful count is invalid".to_owned())?;
    let failed_count = count_at("failed_delete_one_refits")
        .ok_or_else(|| "CB-SEM v12 BCa failure count is invalid".to_owned())?;
    let refits = table(CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID)?;
    let failures = table(CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID)?;
    if case_count < 10
        || successful_count != refits.rows.len()
        || failed_count != failures.rows.len()
        || successful_count + failed_count != case_count
    {
        return Err("CB-SEM v12 BCa delete-one accounting drifted".into());
    }
    let mut omission_source_rows = vec![None; case_count];
    for (candidate, success) in [(refits, true), (failures, false)] {
        let mut prior = None;
        for row in &candidate.rows {
            let position =
                archived_exact_finite_number(candidate, &row.id, "omitted_complete_case_position")
                    .filter(|value| value.fract() == 0.0 && *value >= 0.0)
                    .map(|value| value as usize)
                    .ok_or_else(|| "CB-SEM v12 BCa omission position is invalid".to_owned())?;
            let source_row =
                archived_exact_finite_number(candidate, &row.id, "omitted_source_row_index")
                    .filter(|value| value.fract() == 0.0 && *value >= 0.0)
                    .map(|value| value as usize)
                    .ok_or_else(|| "CB-SEM v12 BCa omission source row is invalid".to_owned())?;
            let expected_id = if success {
                format!("bootstrap_bca_delete_one_refit_{position:05}")
            } else {
                format!("bootstrap_bca_delete_one_failure_{position:05}")
            };
            if row.id != expected_id
                || prior.is_some_and(|prior| prior >= position)
                || position >= case_count
                || omission_source_rows[position].replace(source_row).is_some()
                || !matches!(
                    archived_text_cell(candidate, &row.id, "retained_sampling_positions_sha256"),
                    Some(value) if is_lowercase_sha256(value)
                )
                || !matches!(
                    archived_text_cell(candidate, &row.id, "retained_sample_indices_sha256"),
                    Some(value) if is_lowercase_sha256(value)
                )
            {
                return Err("CB-SEM v12 BCa delete-one identity, order, or digest drifted".into());
            }
            if success {
                let estimates = archived_text_cell(candidate, &row.id, "parameter_estimates_json")
                    .and_then(|encoded| {
                        serde_json::from_str::<Vec<f64>>(encoded)
                            .ok()
                            .filter(|values| {
                                serde_json::to_string(values).ok().as_deref() == Some(encoded)
                            })
                    })
                    .filter(|values| {
                        values.len() == parameter_ids.len()
                            && values.iter().all(|value| value.is_finite())
                    });
                if estimates.is_none()
                    || archived_exact_finite_number(candidate, &row.id, "iterations")
                        .is_none_or(|value| value.fract() != 0.0 || value < 1.0)
                    || archived_exact_finite_number(candidate, &row.id, "objective")
                        .is_none_or(|value| value < 0.0)
                    || archived_exact_finite_number(candidate, &row.id, "gradient_norm")
                        .is_none_or(|value| value < 0.0)
                {
                    return Err("CB-SEM v12 BCa successful delete-one payload is invalid".into());
                }
            } else if !matches!(
                archived_text_cell(candidate, &row.id, "kind"),
                Some(
                    "moment_matrix_not_positive_definite"
                        | "non_convergence"
                        | "inadmissible_solution"
                        | "numerical_failure"
                )
            ) || !matches!(
                archived_text_cell(candidate, &row.id, "message"),
                Some(message) if !message.trim().is_empty()
            ) {
                return Err("CB-SEM v12 BCa failed delete-one payload is invalid".into());
            }
            prior = Some(position);
        }
    }
    let source_rows = omission_source_rows
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "CB-SEM v12 BCa omission partition is incomplete".to_owned())?;
    if source_rows.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("CB-SEM v12 BCa omission source-row order drifted".into());
    }

    match text_at("inference_status") {
        Some("available") => {
            if !archived_not_applicable_cell(summary, "bootstrap_bca", "unavailable_reason_code")
                || !archived_not_applicable_cell(summary, "bootstrap_bca", "unavailable_message")
            {
                return Err("available CB-SEM v12 BCa summary carries a reason".into());
            }
        }
        Some("unavailable") => {
            if !matches!(
                text_at("unavailable_reason_code"),
                Some("base_inference_unavailable" | "incomplete_delete_one_ledger")
            ) || !matches!(text_at("unavailable_message"), Some(message) if !message.trim().is_empty())
            {
                return Err("unavailable CB-SEM v12 BCa summary omits its reason".into());
            }
        }
        _ => return Err("CB-SEM v12 BCa inference status drifted".into()),
    }
    let intervals = table(CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID)?;
    if intervals.rows.len() != parameter_ids.len() {
        return Err("CB-SEM v12 BCa interval cardinality drifted".into());
    }
    for (index, (row, parameter_id)) in intervals.rows.iter().zip(&parameter_ids).enumerate() {
        if row.id != format!("bootstrap_bca_interval_{index:04}")
            || archived_text_cell(intervals, &row.id, "parameter_id") != Some(parameter_id)
        {
            return Err("CB-SEM v12 BCa interval order drifted".into());
        }
        match archived_text_cell(intervals, &row.id, "status") {
            Some("available") => {
                if [
                    "point_estimate",
                    "bias_correction",
                    "acceleration",
                    "adjusted_lower_probability",
                    "adjusted_upper_probability",
                    "interval_lower",
                    "interval_upper",
                    "usable_replicates",
                ]
                .iter()
                .any(|column| archived_exact_finite_number(intervals, &row.id, column).is_none())
                    || !archived_not_applicable_cell(intervals, &row.id, "unavailable_reason")
                {
                    return Err("available CB-SEM v12 BCa interval is invalid".into());
                }
            }
            Some("unavailable") => {
                if [
                    "point_estimate",
                    "bias_correction",
                    "acceleration",
                    "adjusted_lower_probability",
                    "adjusted_upper_probability",
                    "interval_lower",
                    "interval_upper",
                    "usable_replicates",
                ]
                .iter()
                .any(|column| !archived_not_applicable_cell(intervals, &row.id, column))
                    || !matches!(
                        archived_text_cell(intervals, &row.id, "unavailable_reason"),
                        Some(
                            "base_inference_unavailable"
                                | "incomplete_delete_one_ledger"
                                | "bias_correction_probability_at_boundary"
                                | "degenerate_jackknife_acceleration"
                                | "nonfinite_jackknife_arithmetic"
                                | "singular_acceleration_adjustment"
                                | "invalid_adjusted_probability"
                                | "adjusted_probability_order_invalid"
                                | "nonfinite_or_reversed_interval"
                        )
                    )
                {
                    return Err("unavailable CB-SEM v12 BCa interval is invalid".into());
                }
            }
            _ => return Err("CB-SEM v12 BCa interval status drifted".into()),
        }
    }
    Ok(())
}

/// Fail-closed semantic validator for the current Internal/Labs CB-SEM
/// canonical payload. Generic CanonicalResultDocumentV2 validation proves the
/// shape; this additionally proves that a CB-SEM ML attachment has not been
/// relabelled from either qualified internal exact parameter-table identity or
/// its bound adapter.
pub(crate) fn validate_archived_recipe_v4_cbsem_method_identity(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let capability = &document.provenance.capability_cell;
    let base_capability = capability.capability_id == CBSEM_CAPABILITY_ID
        && capability.cell_id == CBSEM_CAPABILITY_CELL_ID;
    let exact_bootstrap_capability = capability.capability_id
        == CBSEM_EXACT_BOOTSTRAP_CAPABILITY_ID
        && capability.cell_id == CBSEM_EXACT_BOOTSTRAP_CELL_ID;
    if !base_capability && !exact_bootstrap_capability {
        return Ok(());
    }
    let expected_capability_version = if exact_bootstrap_capability {
        CBSEM_EXACT_BOOTSTRAP_CAPABILITY_VERSION
    } else {
        CBSEM_CAPABILITY_VERSION
    };
    if capability.capability_version != expected_capability_version {
        return Err("CB-SEM canonical capability version does not match its option cell".into());
    }
    let (adapter, estimator, moment, schema, mean_structure, mean_replacement, _current_rmsea) =
        match (
            document.provenance.method_version.as_str(),
            document.provenance.engine_version.as_str(),
        ) {
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                false,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                false,
                false,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V4,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V4,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
                false,
                true,
                false,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
                false,
                true,
                true,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V3,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V3,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
                true,
                false,
                false,
            ),
            (
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6,
            ) => (
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
                true,
                false,
                true,
            ),
            _ => return Err("CB-SEM canonical method identity is unsupported".into()),
        };
    if document.provenance.engine_version != adapter {
        return Err("CB-SEM canonical engine identity does not match its method".into());
    }
    let exact_bootstrap_adapter = matches!(
        adapter,
        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
    );
    if exact_bootstrap_adapter != exact_bootstrap_capability {
        return Err("CB-SEM canonical capability cell does not match its execution adapter".into());
    }
    let mut summaries = document
        .tables
        .iter()
        .filter(|table| table.id == "estimation_summary");
    let summary = summaries
        .next()
        .ok_or_else(|| "CB-SEM canonical estimation summary is missing".to_owned())?;
    if summaries.next().is_some() {
        return Err("CB-SEM canonical estimation summary is duplicated".into());
    }
    for (column_id, expected) in [
        ("execution_adapter_version", adapter),
        ("estimator_method_version", estimator),
        ("moment_input_method_version", moment),
    ] {
        if archived_text_cell(summary, "run", column_id) != Some(expected) {
            return Err(format!(
                "CB-SEM canonical estimation summary has an invalid {column_id}"
            ));
        }
    }
    if archived_number_cell(summary, "run", "compiled_moment_schema_version") != Some(schema as f64)
        || archived_boolean_cell(summary, "run", "mean_structure") != Some(mean_structure)
    {
        return Err("CB-SEM canonical schema or mean-structure identity is inconsistent".into());
    }
    let recorded_means_digest =
        archived_text_cell(summary, "run", "canonical_observed_means_sha256");
    if mean_structure {
        if recorded_means_digest.and_then(recorded_sha256).is_none() {
            return Err(
                "CB-SEM canonical mean structure is missing its observed-means digest".into(),
            );
        }
    } else if recorded_means_digest.is_some() {
        return Err(
            "CB-SEM covariance-only canonical result unexpectedly records observed means".into(),
        );
    }
    for table_id in ["observed_means", "implied_means", "residual_means"] {
        let matching = document
            .tables
            .iter()
            .filter(|table| table.id == table_id)
            .collect::<Vec<_>>();
        let expected_count = usize::from(mean_structure);
        if matching.len() != expected_count
            || (mean_structure && matching.first().is_some_and(|table| table.rows.is_empty()))
        {
            return Err(format!(
                "CB-SEM canonical {table_id} table does not match its mean-structure identity"
            ));
        }
    }
    let missing_data_table_count = document
        .tables
        .iter()
        .filter(|table| {
            matches!(
                table.id.as_str(),
                qpls_project::MISSING_DATA_EXECUTION_TABLE_ID_V1
                    | qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1
                    | qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1
            )
        })
        .count();
    if mean_replacement {
        if missing_data_table_count != 3 {
            return Err("CB-SEM mean-replacement canonical tables are incomplete".into());
        }
        qpls_project::mean_replacement_receipt_from_document_v1(document)
            .map_err(|error| error.to_string())?;
    } else if missing_data_table_count != 0 {
        return Err("non-mean-replacement CB-SEM result carries missing-data tables".into());
    }
    let score_lm_tables = document
        .tables
        .iter()
        .filter(|table| {
            matches!(
                table.id.as_str(),
                "modification_index_score_tests" | "modification_indices"
            )
        })
        .collect::<Vec<_>>();
    let score_lm_sections = document
        .sections
        .iter()
        .filter(|section| section.id == "modification_indices")
        .collect::<Vec<_>>();
    if matches!(
        adapter,
        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
    ) {
        if score_lm_tables.len() != 1
            || score_lm_tables[0].id != "modification_index_score_tests"
            || score_lm_sections.len() != 1
            || score_lm_sections[0].table_ids != vec!["modification_index_score_tests".to_owned()]
            || !score_lm_sections[0].chart_ids.is_empty()
        {
            return Err("current CB-SEM score/LM canonical artifacts are incomplete".into());
        }
    } else if !score_lm_tables.is_empty() || !score_lm_sections.is_empty() {
        return Err("legacy CB-SEM adapter carries score/LM or heuristic MI artifacts".into());
    }
    let bootstrap_table_ids = [
        CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID,
        CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID,
    ];
    let bootstrap_tables = document
        .tables
        .iter()
        .filter(|table| bootstrap_table_ids.contains(&table.id.as_str()))
        .collect::<Vec<_>>();
    let bootstrap_sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_SECTION_ID)
        .collect::<Vec<_>>();
    if matches!(
        adapter,
        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
    ) {
        let [section] = bootstrap_sections.as_slice() else {
            return Err("CB-SEM v9 requires exactly one bootstrap-inference section".into());
        };
        if bootstrap_tables.len() != bootstrap_table_ids.len()
            || section
                .table_ids
                .iter()
                .map(String::as_str)
                .ne(bootstrap_table_ids)
            || !section.chart_ids.is_empty()
        {
            return Err(
                "CB-SEM v9 bootstrap canonical artifacts are incomplete or reordered".into(),
            );
        }
        for (table_id, columns) in [
            (
                CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_SUMMARY_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_INTERVAL_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_REFIT_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_FAILURE_COLUMNS,
            ),
        ] {
            let table = bootstrap_tables
                .iter()
                .find(|table| table.id == table_id)
                .ok_or_else(|| format!("CB-SEM v9 is missing {table_id}"))?;
            if table
                .columns
                .iter()
                .map(|column| column.id.as_str())
                .ne(columns.iter().copied())
                || table
                    .rows
                    .iter()
                    .any(|row| row.cells.len() != columns.len())
            {
                return Err(format!(
                    "CB-SEM v9 {table_id} column order or row width drifted"
                ));
            }
        }
        let summary = bootstrap_tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)
            .expect("v9 bootstrap summary was checked above");
        if summary.rows.len() != 1
            || summary.rows[0].id != "bootstrap"
            || archived_text_cell(summary, "bootstrap", "archive_validation_scope")
                != Some(CBSEM_EXACT_BOOTSTRAP_ARCHIVE_SCOPE)
        {
            return Err("CB-SEM v9 bootstrap summary row or validation scope drifted".into());
        }
        for column in [
            "source_dataset_fingerprint",
            "outer_recipe_analytical_identity_sha256",
            "base_point_result_sha256",
            "compiler_analytical_identity_sha256",
            "plan_sha256",
            "model_scientific_sha256",
            "complete_case_universe_sha256",
        ] {
            if !matches!(
                archived_text_cell(summary, "bootstrap", column),
                Some(value) if is_lowercase_sha256(value)
            ) {
                return Err(format!(
                    "CB-SEM v9 bootstrap summary {column} is not lowercase SHA-256"
                ));
            }
        }
        for table_id in [
            CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID,
        ] {
            let table = bootstrap_tables
                .iter()
                .find(|table| table.id == table_id)
                .expect("v9 bootstrap ledger was checked above");
            for row in &table.rows {
                for column in ["sampling_positions_sha256", "sample_indices_sha256"] {
                    if !matches!(
                        archived_text_cell(table, &row.id, column),
                        Some(value) if is_lowercase_sha256(value)
                    ) {
                        return Err(format!(
                            "CB-SEM v9 {table_id} {column} is not lowercase SHA-256"
                        ));
                    }
                }
            }
        }
        match archived_text_cell(summary, "bootstrap", "inference_status") {
            Some("available") => {
                for column in ["unavailable_reason_code", "unavailable_message"] {
                    let column_index = summary
                        .columns
                        .iter()
                        .position(|candidate| candidate.id == column)
                        .expect("frozen summary column exists");
                    if !matches!(
                        summary.rows[0].cells.get(column_index),
                        Some(qpls_project::CanonicalResultCellV2::Missing {
                            reason: qpls_project::CanonicalMissingReasonV2::NotApplicable,
                            display: None,
                        })
                    ) {
                        return Err("available CB-SEM v9 inference has invalid reason nulls".into());
                    }
                }
            }
            Some("unavailable") => {
                if !matches!(
                    archived_text_cell(summary, "bootstrap", "unavailable_reason_code"),
                    Some(value) if !value.trim().is_empty()
                ) || !matches!(
                    archived_text_cell(summary, "bootstrap", "unavailable_message"),
                    Some(value) if !value.trim().is_empty()
                ) {
                    return Err("unavailable CB-SEM v9 inference omits its typed reason".into());
                }
            }
            _ => return Err("CB-SEM v9 inference status is unsupported".into()),
        }
    } else if !bootstrap_tables.is_empty() || !bootstrap_sections.is_empty() {
        return Err("CB-SEM v2-v8 adapter carries exact case-bootstrap artifacts".into());
    }
    let hypothesis_tables = document
        .tables
        .iter()
        .filter(|table| table.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID)
        .collect::<Vec<_>>();
    let hypothesis_sections = document
        .sections
        .iter()
        .filter(|section| section.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID)
        .collect::<Vec<_>>();
    if matches!(
        adapter,
        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
            | RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
    ) {
        let [table] = hypothesis_tables.as_slice() else {
            return Err("CB-SEM v10 requires exactly one hypothesis-test table".into());
        };
        let [section] = hypothesis_sections.as_slice() else {
            return Err("CB-SEM v10 requires exactly one hypothesis-test section".into());
        };
        if section.table_ids != vec![CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID.to_owned()]
            || !section.chart_ids.is_empty()
            || table
                .columns
                .iter()
                .map(|column| column.id.as_str())
                .ne(CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS.iter().copied())
            || table.rows.is_empty()
            || table
                .rows
                .iter()
                .any(|row| row.cells.len() != CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_COLUMNS.len())
        {
            return Err("CB-SEM v10 hypothesis-test ownership or columns drifted".into());
        }
        let mut prior_parameter = None::<&str>;
        for (ordinal, row) in table.rows.iter().enumerate() {
            if row.id != format!("bootstrap_hypothesis_{ordinal:04}") {
                return Err("CB-SEM v10 hypothesis-test row identity drifted".into());
            }
            let text_at = |column: &str| archived_text_cell(table, &row.id, column);
            let number_at = |column: &str| archived_number_cell(table, &row.id, column);
            let missing_at = |column: &str| {
                table
                    .columns
                    .iter()
                    .position(|candidate| candidate.id == column)
                    .and_then(|index| row.cells.get(index))
                    .is_some_and(|cell| {
                        matches!(
                            cell,
                            qpls_project::CanonicalResultCellV2::Missing {
                                reason: qpls_project::CanonicalMissingReasonV2::NotApplicable,
                                display: None,
                            }
                        )
                    })
            };
            let parameter_id = text_at("parameter_id")
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "CB-SEM v10 hypothesis parameter id is invalid".to_owned())?;
            if prior_parameter.is_some_and(|prior| prior >= parameter_id)
                || text_at("method_version")
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1)
                || text_at("null_hypothesis")
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1)
                || text_at("statistic") != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1)
                || text_at("tie_policy") != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1)
                || text_at("probability_method")
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1)
                || text_at("decision_rule")
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1)
                || !matches!(
                    text_at("selected_test_tail"),
                    Some("two_sided" | "one_sided_greater" | "one_sided_less")
                )
                || number_at("null_value").map(f64::to_bits) != Some(0.0_f64.to_bits())
                || number_at("significance_level").map(f64::to_bits)
                    != Some(CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1.to_bits())
            {
                return Err("CB-SEM v10 hypothesis method, tail, null, or order drifted".into());
            }
            let globally_available = match text_at("inference_status") {
                Some("available") => {
                    if !missing_at("global_unavailable_reason_code")
                        || !missing_at("global_unavailable_message")
                    {
                        return Err(
                            "available CB-SEM v10 hypothesis inference carries a global reason"
                                .into(),
                        );
                    }
                    true
                }
                Some("unavailable") => {
                    if !matches!(text_at("global_unavailable_reason_code"), Some(value) if !value.trim().is_empty())
                        || !matches!(text_at("global_unavailable_message"), Some(value) if !value.trim().is_empty())
                    {
                        return Err(
                            "unavailable CB-SEM v10 hypothesis inference omits its global reason"
                                .into(),
                        );
                    }
                    false
                }
                _ => return Err("CB-SEM v10 hypothesis inference status drifted".into()),
            };
            match text_at("parameter_status") {
                Some("available") if globally_available => {
                    for column in [
                        "point_estimate",
                        "two_sided_exceedances",
                        "greater_or_equal_exceedances",
                        "less_or_equal_exceedances",
                        "p_value_two_sided",
                        "p_value_greater",
                        "p_value_less",
                        "selected_exceedances",
                        "selected_p_value",
                    ] {
                        if number_at(column).is_none() {
                            return Err(
                                "available CB-SEM v10 hypothesis row omits numeric evidence".into(),
                            );
                        }
                    }
                    if archived_boolean_cell(table, &row.id, "reject_null").is_none()
                        || !missing_at("unavailable_reason")
                    {
                        return Err(
                            "available CB-SEM v10 hypothesis row has invalid decision nulls".into(),
                        );
                    }
                }
                Some("unavailable") => {
                    for column in [
                        "point_estimate",
                        "two_sided_exceedances",
                        "greater_or_equal_exceedances",
                        "less_or_equal_exceedances",
                        "p_value_two_sided",
                        "p_value_greater",
                        "p_value_less",
                        "selected_exceedances",
                        "selected_p_value",
                        "reject_null",
                    ] {
                        if !missing_at(column) {
                            return Err(
                                "unavailable CB-SEM v10 hypothesis row carries outcome evidence"
                                    .into(),
                            );
                        }
                    }
                    if !matches!(
                        text_at("unavailable_reason"),
                        Some(
                            "insufficient_usable_replicates"
                                | "nonregular_variance_boundary"
                                | "zero_null_outside_open_domain"
                                | "unsupported_parameter_family"
                        )
                    ) {
                        return Err(
                            "unavailable CB-SEM v10 hypothesis row has an invalid reason".into(),
                        );
                    }
                }
                _ => return Err("CB-SEM v10 hypothesis parameter status drifted".into()),
            }
            prior_parameter = Some(parameter_id);
        }
    } else if !hypothesis_tables.is_empty() || !hypothesis_sections.is_empty() {
        return Err("CB-SEM v2-v9 adapter carries selected-tail hypothesis artifacts".into());
    }
    validate_archived_studentized_bootstrap_artifacts(document, adapter)?;
    validate_archived_bca_bootstrap_artifacts(document, adapter)?;
    qpls_project::validate_recipe_v4_cbsem_rmsea_fit_document_v1(document)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn text_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Text,
        description: description.into(),
        role: Some(CanonicalColumnRole::Label),
        unit: None,
        default_precision: None,
    }
}

fn number_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Number,
        description: description.into(),
        role: Some(CanonicalColumnRole::Estimate),
        unit: None,
        default_precision: Some(6),
    }
}

fn boolean_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Boolean,
        description: description.into(),
        role: Some(CanonicalColumnRole::Diagnostic),
        unit: None,
        default_precision: None,
    }
}

fn text(value: impl Into<String>) -> CanonicalResultCell {
    CanonicalResultCell::Text {
        value: value.into(),
    }
}

fn number(value: f64) -> CanonicalResultCell {
    CanonicalResultCell::Number {
        value,
        display: None,
    }
}

fn boolean(value: bool) -> CanonicalResultCell {
    CanonicalResultCell::Boolean { value }
}

fn missing() -> CanonicalResultCell {
    CanonicalResultCell::Missing {
        reason: CanonicalMissingReason::NotEstimated,
        display: None,
    }
}

fn not_applicable() -> CanonicalResultCell {
    CanonicalResultCell::Missing {
        reason: CanonicalMissingReason::NotApplicable,
        display: None,
    }
}

fn optional_number(value: Option<f64>) -> CanonicalResultCell {
    value.map_or_else(missing, number)
}

fn optional_usize(value: Option<usize>) -> CanonicalResultCell {
    optional_number(value.map(|value| value as f64))
}

fn score_lm_unavailable_reason(reason: CbsemCfaScoreLmUnavailableReasonV1) -> &'static str {
    match reason {
        CbsemCfaScoreLmUnavailableReasonV1::NuisanceInformationUnavailable => {
            "nuisance_information_unavailable"
        }
        CbsemCfaScoreLmUnavailableReasonV1::EfficientInformationNonPositive => {
            "efficient_information_non_positive"
        }
        CbsemCfaScoreLmUnavailableReasonV1::NonFiniteComputation => "non_finite_computation",
    }
}

fn score_lm_table(
    bundle: &CbsemCfaScoreLmBundleV1,
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> Result<CanonicalResultTable, String> {
    if bundle.method_version != CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1
        || bundle.scope != CBSEM_CFA_SCORE_LM_SCOPE_V1
    {
        return Err("score/LM method or scope identity drifted".into());
    }
    let mut previous_parameter_id = None::<&str>;
    let rows = bundle
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            if row.kind != "residual_covariance"
                || row.parameter_id.trim().is_empty()
                || row.lhs.trim().is_empty()
                || row.rhs.trim().is_empty()
                || row.lhs == row.rhs
                || previous_parameter_id
                    .is_some_and(|previous| previous >= row.parameter_id.as_str())
            {
                return Err(
                    "score/LM rows are not unique ordered residual-covariance candidates".into(),
                );
            }
            previous_parameter_id = Some(row.parameter_id.as_str());
            let (status, numeric, unavailable_reason) = match &row.outcome {
                CbsemCfaScoreLmOutcomeV1::Available {
                    score,
                    efficient_score,
                    candidate_information,
                    efficient_information,
                    modification_index,
                    expected_parameter_change,
                    p_value,
                } => {
                    let expected_mi = efficient_score * efficient_score / efficient_information;
                    let expected_epc = efficient_score / efficient_information;
                    if [
                        *score,
                        *efficient_score,
                        *candidate_information,
                        *efficient_information,
                        *modification_index,
                        *expected_parameter_change,
                        *p_value,
                    ]
                    .into_iter()
                    .any(|value| !value.is_finite() || value.to_bits() == (-0.0_f64).to_bits())
                        || *candidate_information <= 0.0
                        || *efficient_information <= 0.0
                        || modification_index.to_bits() != expected_mi.to_bits()
                        || expected_parameter_change.to_bits() != expected_epc.to_bits()
                        || !(0.0..=1.0).contains(p_value)
                    {
                        return Err("available score/LM arithmetic is invalid".into());
                    }
                    (
                        "available",
                        vec![
                            number(*score),
                            number(*efficient_score),
                            number(*candidate_information),
                            number(*efficient_information),
                            number(*modification_index),
                            number(*expected_parameter_change),
                            number(1.0),
                            number(*p_value),
                        ],
                        not_applicable(),
                    )
                }
                CbsemCfaScoreLmOutcomeV1::Unavailable { reason } => (
                    "unavailable",
                    vec![missing(); 8],
                    text(score_lm_unavailable_reason(*reason)),
                ),
            };
            Ok(CanonicalResultRow {
                id: format!("score_lm_{index:04}"),
                cells: vec![
                    text(&bundle.method_version),
                    text(&bundle.scope),
                    text(&row.parameter_id),
                    text(&row.kind),
                    text(&row.lhs),
                    text(&row.rhs),
                    text(status),
                ]
                .into_iter()
                .chain(numeric)
                .chain(std::iter::once(unavailable_reason))
                .collect(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CanonicalResultTable {
        id: "modification_index_score_tests".into(),
        title: "Modification-index score tests".into(),
        description: Some("Exact one-degree-of-freedom score/LM tests for explicitly declared fixed-zero residual covariances. Unavailable inference remains typed and null; no heuristic substitution is performed.".into()),
        columns: vec![
            text_column("method_version", "Method version", "Frozen score/LM method identity."),
            text_column("scope", "Scope", "Frozen candidate scope."),
            text_column("parameter_id", "Parameter ID", "Compiled fixed-zero residual-covariance parameter identity."),
            text_column("kind", "Kind", "Candidate parameter family."),
            text_column("lhs", "Left endpoint", "Compiled left observed-variable endpoint."),
            text_column("rhs", "Right endpoint", "Compiled right observed-variable endpoint."),
            text_column("status", "Status", "Available or typed unavailable inference."),
            number_column("score", "Score", "Candidate score under the constrained model."),
            number_column("efficient_score", "Efficient score", "Nuisance-adjusted candidate score."),
            number_column("candidate_information", "Candidate information", "Unadjusted expected information."),
            number_column("efficient_information", "Efficient information", "Nuisance-adjusted expected information."),
            number_column("modification_index", "MI", "One-degree-of-freedom score/LM statistic."),
            number_column("expected_parameter_change", "EPC", "Expected parameter change under release."),
            number_column("degrees_of_freedom", "df", "Score/LM test degrees of freedom."),
            number_column("p_value", "p", "Chi-square(1) survival probability."),
            text_column("unavailable_reason", "Unavailable reason", "Typed reason when score/LM inference is unavailable."),
        ],
        rows,
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    })
}

fn recorded_sha256(value: &str) -> Option<String> {
    let candidate = value.rsplit_once(':').map_or(value, |(_, suffix)| suffix);
    is_lowercase_sha256(candidate).then(|| candidate.to_owned())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn exact_bootstrap_failure_kind(kind: CbsemExactCaseBootstrapFailureKindV1) -> &'static str {
    match kind {
        CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite => {
            "moment_matrix_not_positive_definite"
        }
        CbsemExactCaseBootstrapFailureKindV1::NonConvergence => "non_convergence",
        CbsemExactCaseBootstrapFailureKindV1::InadmissibleSolution => "inadmissible_solution",
        CbsemExactCaseBootstrapFailureKindV1::NumericalFailure => "numerical_failure",
    }
}

fn exact_case_bootstrap_tables(
    bootstrap: &CbsemExactCaseBootstrapResultV1,
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> Result<[CanonicalResultTable; 4], String> {
    let source_dataset_fingerprint = recorded_sha256(&bootstrap.source_dataset_fingerprint)
        .ok_or_else(|| {
            "exact case-bootstrap source fingerprint is not lowercase SHA-256".to_owned()
        })?;
    for value in [
        bootstrap.outer_recipe_analytical_identity_sha256.as_str(),
        bootstrap.base_point_result_sha256.as_str(),
        bootstrap.compiler_analytical_identity_sha256.as_str(),
        bootstrap.plan_sha256.as_str(),
        bootstrap.model_scientific_sha256.as_str(),
        bootstrap.complete_case_universe_sha256.as_str(),
    ] {
        if !is_lowercase_sha256(value) {
            return Err("exact case-bootstrap summary contains non-lowercase SHA-256".into());
        }
    }
    if bootstrap.successful_refits.iter().any(|refit| {
        !is_lowercase_sha256(&refit.sampling_positions_sha256)
            || !is_lowercase_sha256(&refit.sample_indices_sha256)
    }) || bootstrap.failed_refits.iter().any(|failure| {
        !is_lowercase_sha256(&failure.sampling_positions_sha256)
            || !is_lowercase_sha256(&failure.sample_indices_sha256)
    }) {
        return Err("exact case-bootstrap ledger contains non-lowercase SHA-256".into());
    }
    if bootstrap.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || bootstrap.estimator_method_version != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || bootstrap.complete_case_universe_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1
        || bootstrap.sample_indices_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || bootstrap.sampling_positions_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1
        || bootstrap.interval_method != CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1
        || bootstrap.stream_token != CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1
        || bootstrap.retry_policy != CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1
        || bootstrap.max_attempts_per_replicate
            != CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1
    {
        return Err(
            "exact case-bootstrap method, digest, stream, or retry identity drifted".into(),
        );
    }
    let capability_cells = Some(vec![capability_cell.clone()]);
    let parameter_ids_json = serde_json::to_string(&bootstrap.parameter_ids)
        .map_err(|error| format!("bootstrap parameter IDs are not serializable: {error}"))?;
    let (inference_status, unavailable_reason, unavailable_message) = match &bootstrap.inference {
        CbsemExactCaseBootstrapInferenceV1::Available => {
            ("available", not_applicable(), not_applicable())
        }
        CbsemExactCaseBootstrapInferenceV1::Unavailable {
            reason_code,
            message,
        } => ("unavailable", text(reason_code), text(message)),
    };
    let summary = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID.into(),
        title: "Exact case-bootstrap summary".into(),
        description: Some(
            "Frozen exact-CFA case-bootstrap plan, inference status, and descriptor-only archive validation scope."
                .into(),
        ),
        columns: vec![
            text_column("method_version", "Method version", "Exact bootstrap method identity."),
            text_column("estimator_method_version", "Estimator method", "Exact point/refit estimator identity."),
            text_column("source_dataset_id", "Dataset ID", "Bound source dataset identity."),
            text_column("source_dataset_fingerprint", "Dataset fingerprint", "Bound source dataset SHA-256."),
            text_column("outer_recipe_analytical_identity_sha256", "Outer recipe SHA-256", "Analytical identity of the bootstrap RecipeV4."),
            text_column("base_point_result_sha256", "Base point SHA-256", "Digest of the attached v8 point-result projection."),
            text_column("compiler_analytical_identity_sha256", "Point compiler SHA-256", "Point-estimator compiler analytical identity."),
            text_column("plan_sha256", "Plan SHA-256", "Compiled point-plan identity."),
            text_column("model_scientific_sha256", "Model SHA-256", "Bound scientific model identity."),
            number_column("complete_case_sample_size", "Complete cases", "Fixed complete-case sampling-frame size."),
            text_column("complete_case_universe_digest_method", "Universe digest method", "Complete-case-universe digest identity."),
            text_column("complete_case_universe_sha256", "Universe SHA-256", "Digest of ordered complete source rows."),
            text_column("covariance_denominator", "Covariance denominator", "Refit covariance denominator."),
            text_column("sample_indices_digest_method", "Source-row digest method", "Runtime mapped-source-row digest identity."),
            text_column("sampling_positions_digest_method", "Schedule digest method", "Reopen-verifiable sampling-position digest identity."),
            text_column("interval_method", "Interval method", "Percentile interval identity."),
            number_column("confidence_level", "Confidence level", "Interval confidence level."),
            number_column("requested_replicates", "Requested", "Preplanned primary draws."),
            number_column("attempted_refits", "Attempted", "Attempted primary refits."),
            number_column("usable_replicates", "Usable", "Successful refits used for inference."),
            number_column("failed_replicates", "Failed", "Failed primary refits."),
            number_column("minimum_usable_fraction", "Minimum usable fraction", "Frozen usable fraction."),
            number_column("minimum_usable_replicates", "Minimum usable", "Frozen usable-count threshold."),
            text_column("seed_decimal", "Seed", "Exact unsigned seed encoded as decimal text."),
            text_column("stream_token", "Stream token", "Deterministic resampling stream domain."),
            text_column("retry_policy", "Retry policy", "Frozen no-retry policy."),
            number_column("max_attempts_per_replicate", "Maximum attempts", "Maximum attempts for each preplanned draw."),
            text_column("parameter_ids_json", "Parameter IDs", "Compact canonical JSON array in estimator order."),
            text_column("inference_status", "Inference status", "Available or unavailable."),
            text_column("unavailable_reason_code", "Unavailable reason", "Typed reason when inference is unavailable."),
            text_column("unavailable_message", "Unavailable message", "Human-readable unavailable explanation."),
            text_column("archive_validation_scope", "Archive validation scope", "Truthful boundary of descriptor-only reopen validation."),
        ],
        rows: vec![CanonicalResultRow {
            id: "bootstrap".into(),
            cells: vec![
                text(&bootstrap.method_version),
                text(&bootstrap.estimator_method_version),
                text(&bootstrap.source_dataset_id),
                text(source_dataset_fingerprint),
                text(&bootstrap.outer_recipe_analytical_identity_sha256),
                text(&bootstrap.base_point_result_sha256),
                text(&bootstrap.compiler_analytical_identity_sha256),
                text(&bootstrap.plan_sha256),
                text(&bootstrap.model_scientific_sha256),
                number(bootstrap.complete_case_sample_size as f64),
                text(&bootstrap.complete_case_universe_digest_method),
                text(&bootstrap.complete_case_universe_sha256),
                text(match bootstrap.covariance_denominator {
                    qpls_core::SemCovarianceDenominatorV4::SampleNMinusOne => "sample_n_minus_one",
                    qpls_core::SemCovarianceDenominatorV4::MaximumLikelihoodN => "maximum_likelihood_n",
                }),
                text(&bootstrap.sample_indices_digest_method),
                text(&bootstrap.sampling_positions_digest_method),
                text(&bootstrap.interval_method),
                number(bootstrap.confidence_level),
                number(f64::from(bootstrap.requested_replicates)),
                number(f64::from(bootstrap.attempted_refits)),
                number(f64::from(bootstrap.usable_replicates)),
                number(f64::from(bootstrap.failed_replicates)),
                number(bootstrap.minimum_usable_fraction),
                number(f64::from(bootstrap.minimum_usable_replicates)),
                text(bootstrap.seed.to_string()),
                text(&bootstrap.stream_token),
                text(&bootstrap.retry_policy),
                number(f64::from(bootstrap.max_attempts_per_replicate)),
                text(parameter_ids_json),
                text(inference_status),
                unavailable_reason,
                unavailable_message,
                text(CBSEM_EXACT_BOOTSTRAP_ARCHIVE_SCOPE),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let intervals = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID.into(),
        title: "Exact case-bootstrap parameter intervals".into(),
        description: Some(
            "Sample-SD standard errors and Type-7 percentile intervals from usable exact refits."
                .into(),
        ),
        columns: vec![
            text_column(
                "parameter_id",
                "Parameter ID",
                "Stable compiled parameter identity.",
            ),
            number_column("original", "Original", "Bound point estimate."),
            number_column(
                "bootstrap_mean",
                "Bootstrap mean",
                "Mean usable refit estimate.",
            ),
            number_column("bias", "Bias", "Bootstrap mean minus original."),
            number_column(
                "standard_error",
                "Standard error",
                "Sample SD across usable refits.",
            ),
            number_column(
                "percentile_lower",
                "Percentile lower",
                "Type-7 lower percentile.",
            ),
            number_column(
                "percentile_upper",
                "Percentile upper",
                "Type-7 upper percentile.",
            ),
            number_column(
                "usable_replicates",
                "Usable",
                "Usable refits contributing to this row.",
            ),
        ],
        rows: bootstrap
            .intervals
            .iter()
            .enumerate()
            .map(|(index, interval)| CanonicalResultRow {
                id: format!("bootstrap_interval_{index:04}"),
                cells: vec![
                    text(&interval.parameter_id),
                    number(interval.original),
                    number(interval.bootstrap_mean),
                    number(interval.bias),
                    number(interval.standard_error),
                    number(interval.percentile_lower),
                    number(interval.percentile_upper),
                    number(f64::from(interval.usable_replicates)),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let mut successful_refits = bootstrap.successful_refits.clone();
    successful_refits.sort_by_key(|refit| refit.replicate_index);
    let refits = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID.into(),
        title: "Successful exact case-bootstrap refits".into(),
        description: Some("Compact successful-refit validation witnesses; parameter vectors remain canonical JSON text.".into()),
        columns: vec![
            number_column("replicate_index", "Replicate", "Zero-based preplanned replicate index."),
            text_column("sampling_positions_sha256", "Schedule SHA-256", "Reopen-verifiable sampling-position digest."),
            text_column("sample_indices_sha256", "Source rows SHA-256", "Runtime-only mapped-source-row digest."),
            text_column("parameter_estimates_json", "Parameter estimates", "Compact canonical JSON vector in parameter-ID order."),
            number_column("iterations", "Iterations", "Exact refit optimizer iterations."),
            number_column("objective", "Objective", "Exact refit ML objective."),
            number_column("gradient_norm", "Gradient norm", "Exact refit final gradient norm."),
        ],
        rows: successful_refits
            .iter()
            .map(|refit| {
                let estimates = serde_json::to_string(&refit.parameter_estimates)
                    .map_err(|error| format!("bootstrap parameter vector is not canonical JSON: {error}"))?;
                Ok(CanonicalResultRow {
                    id: format!("bootstrap_refit_{:05}", refit.replicate_index),
                    cells: vec![
                        number(f64::from(refit.replicate_index)),
                        text(&refit.sampling_positions_sha256),
                        text(&refit.sample_indices_sha256),
                        text(estimates),
                        number(f64::from(refit.iterations)),
                        number(refit.objective),
                        number(refit.gradient_norm),
                    ],
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let mut failed_refits = bootstrap.failed_refits.clone();
    failed_refits.sort_by_key(|failure| failure.replicate_index);
    let failures = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID.into(),
        title: "Failed exact case-bootstrap refits".into(),
        description: Some("Complete typed ledger of failed preplanned primary refits.".into()),
        columns: vec![
            number_column(
                "replicate_index",
                "Replicate",
                "Zero-based preplanned replicate index.",
            ),
            text_column(
                "sampling_positions_sha256",
                "Schedule SHA-256",
                "Reopen-verifiable sampling-position digest.",
            ),
            text_column(
                "sample_indices_sha256",
                "Source rows SHA-256",
                "Runtime-only mapped-source-row digest.",
            ),
            text_column(
                "kind",
                "Failure kind",
                "Typed exact-refit failure classification.",
            ),
            text_column(
                "message",
                "Message",
                "Failure detail retained without retry.",
            ),
        ],
        rows: failed_refits
            .iter()
            .map(|failure| CanonicalResultRow {
                id: format!("bootstrap_failure_{:05}", failure.replicate_index),
                cells: vec![
                    number(f64::from(failure.replicate_index)),
                    text(&failure.sampling_positions_sha256),
                    text(&failure.sample_indices_sha256),
                    text(exact_bootstrap_failure_kind(failure.kind)),
                    text(&failure.message),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells,
    };
    Ok([summary, intervals, refits, failures])
}

fn exact_bootstrap_refit_standard_error_unavailable_reason(
    reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
) -> &'static str {
    match reason {
        CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::SingularInformation => {
            "singular_information"
        }
        CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InformationNotPositiveDefinite => {
            "information_not_positive_definite"
        }
        CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::InvalidInformationVarianceOrStandardError => {
            "invalid_information_variance_or_standard_error"
        }
        CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::DerivativeUnavailable => {
            "derivative_unavailable"
        }
        CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::NumericalInformationFailure => {
            "numerical_information_failure"
        }
    }
}

fn exact_bootstrap_studentized_unavailable_reason(
    reason: qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1,
) -> &'static str {
    match reason {
        qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::PointStandardErrorsUnavailable => {
            "point_standard_errors_unavailable"
        }
        qpls_estimation::CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates => {
            "insufficient_studentized_usable_replicates"
        }
    }
}

fn exact_case_bootstrap_studentized_tables(
    studentized: &CbsemExactCaseBootstrapStudentizedSidecarV1,
    base: &CbsemExactCaseBootstrapResultV1,
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> Result<[CanonicalResultTable; 4], String> {
    if studentized.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1
        || studentized.standard_error_method_version
            != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
        || studentized.expected_information_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
        || studentized.pivot_method != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1
        || studentized.quantile_method != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1
        || studentized.interval_method != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1
        || studentized.archive_validation_scope
            != CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1
        || studentized.confidence_level.to_bits() != base.confidence_level.to_bits()
        || studentized.minimum_usable_fraction.to_bits() != base.minimum_usable_fraction.to_bits()
        || studentized.minimum_usable_replicates != base.minimum_usable_replicates
        || studentized.parameter_ids != base.parameter_ids
        || studentized.intervals.len() != base.parameter_ids.len()
        || studentized.refit_standard_errors.len() != base.successful_refits.len()
    {
        return Err("studentized exact-bootstrap sidecar identity or ownership drifted".into());
    }
    if studentized
        .intervals
        .iter()
        .zip(&base.parameter_ids)
        .any(|(interval, parameter_id)| interval.parameter_id != *parameter_id)
        || studentized
            .refit_standard_errors
            .iter()
            .zip(&base.successful_refits)
            .any(|(receipt, witness)| receipt.replicate_index != witness.replicate_index)
    {
        return Err("studentized exact-bootstrap parameter or refit order drifted".into());
    }

    let capability_cells = Some(vec![capability_cell.clone()]);
    let parameter_ids_json = serde_json::to_string(&studentized.parameter_ids)
        .map_err(|error| format!("studentized parameter IDs are not serializable: {error}"))?;
    let (inference_status, unavailable_reason_code, unavailable_message) =
        match &studentized.inference {
            CbsemExactCaseBootstrapStudentizedInferenceV1::Available => {
                ("available", not_applicable(), not_applicable())
            }
            CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable { reason, message } => (
                "unavailable",
                text(exact_bootstrap_studentized_unavailable_reason(*reason)),
                text(message),
            ),
        };
    let summary = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID.into(),
        title: "Analytically studentized bootstrap summary".into(),
        description: Some(
            "Frozen analytic-SE studentized interval identity and truthful arithmetic-only archive validation scope."
                .into(),
        ),
        columns: vec![
            text_column("method_version", "Method version", "Studentized interval method identity."),
            text_column("standard_error_method_version", "Standard-error method", "Whole-vector analytic standard-error receipt identity."),
            text_column("expected_information_method", "Information method", "Expected-information and delta-method identity."),
            text_column("pivot_method", "Pivot method", "Studentized pivot identity."),
            text_column("quantile_method", "Quantile method", "Frozen pivot quantile identity."),
            text_column("interval_method", "Interval method", "Reversed studentized interval identity."),
            text_column("archive_validation_scope", "Archive validation scope", "Exact boundary of reopen validation."),
            number_column("confidence_level", "Confidence level", "Interval confidence level."),
            number_column("minimum_usable_fraction", "Minimum usable fraction", "Frozen usable fraction."),
            number_column("minimum_usable_replicates", "Minimum usable", "Frozen usable-count threshold."),
            number_column("studentized_usable_replicates", "Studentized usable", "Successful base refits with whole-vector analytic standard errors."),
            text_column("parameter_ids_json", "Parameter IDs", "Compact canonical parameter-ID array."),
            text_column("inference_status", "Inference status", "Available or unavailable."),
            text_column("unavailable_reason_code", "Unavailable reason", "Typed global reason when inference is unavailable."),
            text_column("unavailable_message", "Unavailable message", "Human-readable unavailable explanation."),
        ],
        rows: vec![CanonicalResultRow {
            id: "bootstrap_studentized".into(),
            cells: vec![
                text(&studentized.method_version),
                text(&studentized.standard_error_method_version),
                text(&studentized.expected_information_method),
                text(&studentized.pivot_method),
                text(&studentized.quantile_method),
                text(&studentized.interval_method),
                text(&studentized.archive_validation_scope),
                number(studentized.confidence_level),
                number(studentized.minimum_usable_fraction),
                number(f64::from(studentized.minimum_usable_replicates)),
                number(f64::from(studentized.studentized_usable_replicates)),
                text(parameter_ids_json),
                text(inference_status),
                unavailable_reason_code,
                unavailable_message,
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let point_rows = match &studentized.point_standard_errors.outcome {
        CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
            information_method,
            parameters,
        } => {
            if studentized.point_standard_errors.method_version
                != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
                || information_method
                    != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
                || parameters.len() != base.parameter_ids.len()
                || parameters.iter().zip(&base.parameter_ids).any(|(row, id)| {
                    row.parameter_id != *id
                        || !row.standard_error.is_finite()
                        || row.standard_error <= 0.0
                })
            {
                return Err("studentized point standard-error receipt drifted".into());
            }
            parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| CanonicalResultRow {
                    id: format!("bootstrap_studentized_point_standard_error_{index:04}"),
                    cells: vec![
                        text(&studentized.point_standard_errors.method_version),
                        text(&parameter.parameter_id),
                        text("available"),
                        text(information_method),
                        number(parameter.standard_error),
                        not_applicable(),
                    ],
                })
                .collect()
        }
        CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable { reason } => {
            if studentized.point_standard_errors.method_version
                != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
            {
                return Err("studentized point standard-error method identity drifted".into());
            }
            base.parameter_ids
                .iter()
                .enumerate()
                .map(|(index, parameter_id)| CanonicalResultRow {
                    id: format!("bootstrap_studentized_point_standard_error_{index:04}"),
                    cells: vec![
                        text(&studentized.point_standard_errors.method_version),
                        text(parameter_id),
                        text("unavailable"),
                        not_applicable(),
                        not_applicable(),
                        text(exact_bootstrap_refit_standard_error_unavailable_reason(
                            *reason,
                        )),
                    ],
                })
                .collect()
        }
    };
    let point_standard_errors = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID.into(),
        title: "Point analytic standard errors".into(),
        description: Some("Whole-vector point expected-information standard-error receipt, repeated over the fixed parameter order.".into()),
        columns: vec![
            text_column("method_version", "Method version", "Standard-error receipt identity."),
            text_column("parameter_id", "Parameter ID", "Stable compiled parameter identity."),
            text_column("status", "Status", "Available or unavailable."),
            text_column("information_method", "Information method", "Expected-information identity when available."),
            number_column("standard_error", "Standard error", "Point analytic standard error when available."),
            text_column("unavailable_reason", "Unavailable reason", "Typed whole-vector reason when unavailable."),
        ],
        rows: point_rows,
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let intervals = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID.into(),
        title: "Analytically studentized parameter intervals".into(),
        description: Some(
            "Reversed Type-7 studentized-pivot intervals in the exact base parameter order.".into(),
        ),
        columns: vec![
            text_column(
                "parameter_id",
                "Parameter ID",
                "Stable compiled parameter identity.",
            ),
            text_column("status", "Status", "Available or unavailable."),
            number_column("point_estimate", "Point estimate", "Bound point estimate."),
            number_column(
                "point_standard_error",
                "Point standard error",
                "Analytic point standard error.",
            ),
            number_column(
                "lower_pivot_quantile",
                "Lower pivot quantile",
                "Lower Type-7 pivot quantile.",
            ),
            number_column(
                "upper_pivot_quantile",
                "Upper pivot quantile",
                "Upper Type-7 pivot quantile.",
            ),
            number_column(
                "interval_lower",
                "Interval lower",
                "Reversed studentized lower bound.",
            ),
            number_column(
                "interval_upper",
                "Interval upper",
                "Reversed studentized upper bound.",
            ),
            number_column(
                "usable_replicates",
                "Usable",
                "Whole-vector usable studentized refits.",
            ),
            text_column(
                "unavailable_reason",
                "Unavailable reason",
                "Typed global reason when unavailable.",
            ),
        ],
        rows: studentized
            .intervals
            .iter()
            .enumerate()
            .map(|(index, interval)| CanonicalResultRow {
                id: format!("bootstrap_studentized_interval_{index:04}"),
                cells: match interval.outcome {
                    CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Available {
                        point_estimate,
                        point_standard_error,
                        lower_pivot_quantile,
                        upper_pivot_quantile,
                        interval_lower,
                        interval_upper,
                        usable_replicates,
                    } => vec![
                        text(&interval.parameter_id),
                        text("available"),
                        number(point_estimate),
                        number(point_standard_error),
                        number(lower_pivot_quantile),
                        number(upper_pivot_quantile),
                        number(interval_lower),
                        number(interval_upper),
                        number(f64::from(usable_replicates)),
                        not_applicable(),
                    ],
                    CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Unavailable {
                        reason,
                    } => vec![
                        text(&interval.parameter_id),
                        text("unavailable"),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        text(exact_bootstrap_studentized_unavailable_reason(reason)),
                    ],
                },
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let refit_standard_errors = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID.into(),
        title: "Refit analytic standard errors".into(),
        description: Some("One compact whole-vector analytic standard-error receipt for every successful base refit, in base-ledger order.".into()),
        columns: vec![
            number_column("replicate_index", "Replicate", "Zero-based successful base replicate index."),
            text_column("status", "Status", "Available or unavailable."),
            text_column("information_method", "Information method", "Expected-information identity when available."),
            text_column("standard_errors_json", "Standard errors", "Compact canonical vector in parameter-ID order."),
            text_column("unavailable_reason", "Unavailable reason", "Typed whole-vector reason when unavailable."),
        ],
        rows: studentized
            .refit_standard_errors
            .iter()
            .map(|receipt| {
                let cells = match &receipt.outcome {
                    CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Available {
                        information_method,
                        standard_errors,
                    } => {
                        if information_method
                            != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
                            || standard_errors.len() != base.parameter_ids.len()
                            || standard_errors
                                .iter()
                                .any(|value| !value.is_finite() || *value <= 0.0)
                        {
                            return Err("studentized refit standard-error receipt drifted".to_owned());
                        }
                        vec![
                            number(f64::from(receipt.replicate_index)),
                            text("available"),
                            text(information_method),
                            text(serde_json::to_string(standard_errors).map_err(|error| {
                                format!("studentized refit standard errors are not canonical JSON: {error}")
                            })?),
                            not_applicable(),
                        ]
                    }
                    CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Unavailable {
                        reason,
                    } => vec![
                        number(f64::from(receipt.replicate_index)),
                        text("unavailable"),
                        not_applicable(),
                        not_applicable(),
                        text(exact_bootstrap_refit_standard_error_unavailable_reason(*reason)),
                    ],
                };
                Ok(CanonicalResultRow {
                    id: format!(
                        "bootstrap_studentized_refit_standard_error_{:05}",
                        receipt.replicate_index
                    ),
                    cells,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        footnote_ids: Vec::new(),
        capability_cells,
    };

    Ok([
        summary,
        point_standard_errors,
        intervals,
        refit_standard_errors,
    ])
}

fn exact_bootstrap_bca_unavailable_reason(
    reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1,
) -> &'static str {
    match reason {
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::BaseInferenceUnavailable => {
            "base_inference_unavailable"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::IncompleteDeleteOneLedger => {
            "incomplete_delete_one_ledger"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::BiasCorrectionProbabilityAtBoundary => {
            "bias_correction_probability_at_boundary"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::DegenerateJackknifeAcceleration => {
            "degenerate_jackknife_acceleration"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteJackknifeArithmetic => {
            "nonfinite_jackknife_arithmetic"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::SingularAccelerationAdjustment => {
            "singular_acceleration_adjustment"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::InvalidAdjustedProbability => {
            "invalid_adjusted_probability"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::AdjustedProbabilityOrderInvalid => {
            "adjusted_probability_order_invalid"
        }
        CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteOrReversedInterval => {
            "nonfinite_or_reversed_interval"
        }
    }
}

fn exact_case_bootstrap_bca_tables(
    bca: &CbsemExactCaseBootstrapBcaSidecarV1,
    base: &CbsemExactCaseBootstrapResultV1,
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> Result<[CanonicalResultTable; 4], String> {
    if bca.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1
        || bca.base_bootstrap_method_version != base.method_version
        || bca.outer_recipe_analytical_identity_sha256
            != base.outer_recipe_analytical_identity_sha256
        || bca.base_point_result_sha256 != base.base_point_result_sha256
        || bca.compiler_analytical_identity_sha256 != base.compiler_analytical_identity_sha256
        || bca.plan_sha256 != base.plan_sha256
        || bca.model_scientific_sha256 != base.model_scientific_sha256
        || bca.delete_one_refit_method_version
            != CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1
        || bca.bias_correction_method != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1
        || bca.acceleration_method != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2
        || bca.adjusted_probability_method != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2
        || bca.quantile_method != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1
        || bca.retry_policy != CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1
        || bca.confidence_level.to_bits() != base.confidence_level.to_bits()
        || bca.bootstrap_usable_replicates != base.usable_replicates
        || bca.minimum_bootstrap_usable_replicates != base.minimum_usable_replicates
        || bca.delete_one_case_count != base.complete_case_sample_size
        || bca.parameter_ids != base.parameter_ids
        || bca.intervals.len() != base.parameter_ids.len()
        || bca.successful_delete_one_refits.len() + bca.failed_delete_one_refits.len()
            != bca.delete_one_case_count
    {
        return Err(
            "BCa exact-bootstrap sidecar identity, authority, or accounting drifted".into(),
        );
    }
    for value in [
        bca.outer_recipe_analytical_identity_sha256.as_str(),
        bca.base_point_result_sha256.as_str(),
        bca.compiler_analytical_identity_sha256.as_str(),
        bca.plan_sha256.as_str(),
        bca.model_scientific_sha256.as_str(),
    ] {
        if !is_lowercase_sha256(value) {
            return Err("BCa exact-bootstrap authority is not lowercase SHA-256".into());
        }
    }
    if bca
        .intervals
        .iter()
        .zip(&base.parameter_ids)
        .any(|(interval, parameter_id)| interval.parameter_id != *parameter_id)
        || bca.successful_delete_one_refits.windows(2).any(|pair| {
            pair[0].omitted_complete_case_position >= pair[1].omitted_complete_case_position
        })
        || bca.failed_delete_one_refits.windows(2).any(|pair| {
            pair[0].omitted_complete_case_position >= pair[1].omitted_complete_case_position
        })
    {
        return Err("BCa exact-bootstrap parameter or delete-one order drifted".into());
    }
    let mut omissions = vec![None; bca.delete_one_case_count];
    for (position, source_row) in bca
        .successful_delete_one_refits
        .iter()
        .map(|row| {
            (
                row.omitted_complete_case_position,
                row.omitted_source_row_index,
            )
        })
        .chain(bca.failed_delete_one_refits.iter().map(|row| {
            (
                row.omitted_complete_case_position,
                row.omitted_source_row_index,
            )
        }))
    {
        if position >= omissions.len() || omissions[position].replace(source_row).is_some() {
            return Err("BCa exact-bootstrap omission partition is invalid".into());
        }
    }
    let source_rows = omissions
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "BCa exact-bootstrap omission partition is incomplete".to_owned())?;
    if source_rows.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("BCa exact-bootstrap source-row order drifted".into());
    }

    let capability_cells = Some(vec![capability_cell.clone()]);
    let parameter_ids_json = serde_json::to_string(&bca.parameter_ids)
        .map_err(|error| format!("BCa parameter IDs are not serializable: {error}"))?;
    let (inference_status, unavailable_reason_code, unavailable_message) = match &bca.inference {
        CbsemExactCaseBootstrapBcaInferenceV1::Available => {
            ("available", not_applicable(), not_applicable())
        }
        CbsemExactCaseBootstrapBcaInferenceV1::Unavailable { reason, message }
            if !message.trim().is_empty() =>
        {
            (
                "unavailable",
                text(exact_bootstrap_bca_unavailable_reason(*reason)),
                text(message),
            )
        }
        CbsemExactCaseBootstrapBcaInferenceV1::Unavailable { .. } => {
            return Err("unavailable BCa exact-bootstrap inference omits its message".into());
        }
    };
    let summary = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID.into(),
        title: "BCa bootstrap summary".into(),
        description: Some(
            "Frozen BCa identity and ledger/digest/arithmetic-only reopen scope; raw base and delete-one ML fits are not replayed."
                .into(),
        ),
        columns: vec![
            text_column("method_version", "Method version", "BCa interval method identity."),
            text_column("base_bootstrap_method_version", "Base method", "Bound base-bootstrap method identity."),
            text_column("outer_recipe_analytical_identity_sha256", "Recipe authority", "Bound outer Recipe-v4 analytical identity."),
            text_column("base_point_result_sha256", "Base point authority", "Bound canonical base-point result digest."),
            text_column("compiler_analytical_identity_sha256", "Compiler authority", "Bound point compiler identity."),
            text_column("plan_sha256", "Plan authority", "Bound point plan digest."),
            text_column("model_scientific_sha256", "Model authority", "Bound scientific model digest."),
            text_column("delete_one_refit_method_version", "Delete-one method", "Exact delete-one refit identity."),
            text_column("delete_one_sampling_positions_digest_method", "Position digest method", "Delete-one retained-position digest identity."),
            text_column("delete_one_sample_indices_digest_method", "Source digest method", "Delete-one retained-source-index digest identity."),
            text_column("bias_correction_method", "Bias correction", "Frozen BCa bias-correction identity."),
            text_column("acceleration_method", "Acceleration", "Frozen delete-one acceleration identity."),
            text_column("adjusted_probability_method", "Probability adjustment", "Frozen adjusted-probability identity."),
            text_column("quantile_method", "Quantile method", "Frozen interval quantile identity."),
            text_column("retry_policy", "Retry policy", "Frozen delete-one retry policy."),
            text_column("archive_validation_scope", "Archive validation scope", "Truthful boundary of reopen validation."),
            number_column("confidence_level", "Confidence level", "Interval confidence level."),
            number_column("bootstrap_usable_replicates", "Bootstrap usable", "Successful base-bootstrap refits."),
            number_column("minimum_bootstrap_usable_replicates", "Minimum bootstrap usable", "Bound base-bootstrap usability threshold."),
            number_column("delete_one_case_count", "Delete-one cases", "Mandatory complete-case omission count."),
            number_column("successful_delete_one_refits", "Delete-one successes", "Successful delete-one ledger count."),
            number_column("failed_delete_one_refits", "Delete-one failures", "Failed delete-one ledger count."),
            text_column("parameter_ids_json", "Parameter IDs", "Compact canonical parameter-ID array."),
            text_column("inference_status", "Inference status", "Available or unavailable."),
            text_column("unavailable_reason_code", "Unavailable reason", "Typed global reason when unavailable."),
            text_column("unavailable_message", "Unavailable message", "Human-readable global reason."),
        ],
        rows: vec![CanonicalResultRow {
            id: "bootstrap_bca".into(),
            cells: vec![
                text(&bca.method_version),
                text(&bca.base_bootstrap_method_version),
                text(&bca.outer_recipe_analytical_identity_sha256),
                text(&bca.base_point_result_sha256),
                text(&bca.compiler_analytical_identity_sha256),
                text(&bca.plan_sha256),
                text(&bca.model_scientific_sha256),
                text(&bca.delete_one_refit_method_version),
                text(CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1),
                text(CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1),
                text(&bca.bias_correction_method),
                text(&bca.acceleration_method),
                text(&bca.adjusted_probability_method),
                text(&bca.quantile_method),
                text(&bca.retry_policy),
                text(CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE),
                number(bca.confidence_level),
                number(f64::from(bca.bootstrap_usable_replicates)),
                number(f64::from(bca.minimum_bootstrap_usable_replicates)),
                number(bca.delete_one_case_count as f64),
                number(bca.successful_delete_one_refits.len() as f64),
                number(bca.failed_delete_one_refits.len() as f64),
                text(parameter_ids_json),
                text(inference_status),
                unavailable_reason_code,
                unavailable_message,
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let intervals = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID.into(),
        title: "BCa parameter intervals".into(),
        description: Some(
            "Bias-corrected and accelerated intervals in stable compiled parameter order.".into(),
        ),
        columns: vec![
            text_column(
                "parameter_id",
                "Parameter ID",
                "Stable compiled free-parameter identity.",
            ),
            text_column(
                "status",
                "Status",
                "Available or typed unavailable outcome.",
            ),
            number_column(
                "point_estimate",
                "Point estimate",
                "Bound original estimate.",
            ),
            number_column(
                "bias_correction",
                "Bias correction",
                "Midrank normal bias correction.",
            ),
            number_column(
                "acceleration",
                "Acceleration",
                "Delete-one jackknife acceleration.",
            ),
            number_column(
                "adjusted_lower_probability",
                "Adjusted lower probability",
                "BCa-adjusted lower probability.",
            ),
            number_column(
                "adjusted_upper_probability",
                "Adjusted upper probability",
                "BCa-adjusted upper probability.",
            ),
            number_column(
                "interval_lower",
                "Interval lower",
                "Type-7 lower bound at the adjusted probability.",
            ),
            number_column(
                "interval_upper",
                "Interval upper",
                "Type-7 upper bound at the adjusted probability.",
            ),
            number_column(
                "usable_replicates",
                "Usable",
                "Bound usable base-bootstrap count.",
            ),
            text_column(
                "unavailable_reason",
                "Unavailable reason",
                "Typed parameter-level reason.",
            ),
        ],
        rows: bca
            .intervals
            .iter()
            .enumerate()
            .map(|(index, interval)| CanonicalResultRow {
                id: format!("bootstrap_bca_interval_{index:04}"),
                cells: match interval.outcome {
                    CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Available {
                        point_estimate,
                        bias_correction,
                        acceleration,
                        adjusted_lower_probability,
                        adjusted_upper_probability,
                        interval_lower,
                        interval_upper,
                        usable_replicates,
                    } => vec![
                        text(&interval.parameter_id),
                        text("available"),
                        number(point_estimate),
                        number(bias_correction),
                        number(acceleration),
                        number(adjusted_lower_probability),
                        number(adjusted_upper_probability),
                        number(interval_lower),
                        number(interval_upper),
                        number(f64::from(usable_replicates)),
                        not_applicable(),
                    ],
                    CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable {
                        reason,
                    } => vec![
                        text(&interval.parameter_id),
                        text("unavailable"),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        not_applicable(),
                        text(exact_bootstrap_bca_unavailable_reason(reason)),
                    ],
                },
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let refits = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID.into(),
        title: "Successful delete-one refits".into(),
        description: Some(
            "Ordered delete-one identity, retained-frame digests, parameter vector, and optimizer witnesses; raw ML is not replayed on reopen."
                .into(),
        ),
        columns: vec![
            number_column("omitted_complete_case_position", "Omitted position", "Zero-based complete-case omission position."),
            number_column("omitted_source_row_index", "Omitted source row", "Zero-based source-row identity."),
            text_column("retained_sampling_positions_sha256", "Retained-position digest", "Digest of retained complete-case positions."),
            text_column("retained_sample_indices_sha256", "Retained-source digest", "Digest of retained source-row indices."),
            text_column("parameter_estimates_json", "Parameter estimates", "Compact canonical estimate vector in parameter-ID order."),
            number_column("iterations", "Iterations", "Optimizer iteration witness."),
            number_column("objective", "Objective", "Final optimizer objective witness."),
            number_column("gradient_norm", "Gradient norm", "Final gradient-norm witness."),
        ],
        rows: bca
            .successful_delete_one_refits
            .iter()
            .map(|witness| {
                if !is_lowercase_sha256(&witness.retained_sampling_positions_sha256)
                    || !is_lowercase_sha256(&witness.retained_sample_indices_sha256)
                    || witness.parameter_estimates.len() != bca.parameter_ids.len()
                    || witness.parameter_estimates.iter().any(|value| !value.is_finite())
                    || witness.iterations == 0
                    || !witness.objective.is_finite()
                    || witness.objective < 0.0
                    || !witness.gradient_norm.is_finite()
                    || witness.gradient_norm < 0.0
                {
                    return Err("BCa successful delete-one witness is invalid".to_owned());
                }
                Ok(CanonicalResultRow {
                    id: format!(
                        "bootstrap_bca_delete_one_refit_{:05}",
                        witness.omitted_complete_case_position
                    ),
                    cells: vec![
                        number(witness.omitted_complete_case_position as f64),
                        number(witness.omitted_source_row_index as f64),
                        text(&witness.retained_sampling_positions_sha256),
                        text(&witness.retained_sample_indices_sha256),
                        text(serde_json::to_string(&witness.parameter_estimates).map_err(
                            |error| format!("BCa delete-one estimates are not canonical JSON: {error}"),
                        )?),
                        number(f64::from(witness.iterations)),
                        number(witness.objective),
                        number(witness.gradient_norm),
                    ],
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let failures = CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID.into(),
        title: "Failed delete-one refits".into(),
        description: Some(
            "Ordered typed failure ledger for mandatory delete-one fits, including retained-frame digests."
                .into(),
        ),
        columns: vec![
            number_column("omitted_complete_case_position", "Omitted position", "Zero-based complete-case omission position."),
            number_column("omitted_source_row_index", "Omitted source row", "Zero-based source-row identity."),
            text_column("retained_sampling_positions_sha256", "Retained-position digest", "Digest of retained complete-case positions."),
            text_column("retained_sample_indices_sha256", "Retained-source digest", "Digest of retained source-row indices."),
            text_column("kind", "Failure kind", "Typed exact-ML failure classification."),
            text_column("message", "Failure message", "Deterministic failure detail."),
        ],
        rows: bca
            .failed_delete_one_refits
            .iter()
            .map(|failure| {
                if !is_lowercase_sha256(&failure.retained_sampling_positions_sha256)
                    || !is_lowercase_sha256(&failure.retained_sample_indices_sha256)
                    || failure.message.trim().is_empty()
                {
                    return Err("BCa failed delete-one witness is invalid".to_owned());
                }
                Ok(CanonicalResultRow {
                    id: format!(
                        "bootstrap_bca_delete_one_failure_{:05}",
                        failure.omitted_complete_case_position
                    ),
                    cells: vec![
                        number(failure.omitted_complete_case_position as f64),
                        number(failure.omitted_source_row_index as f64),
                        text(&failure.retained_sampling_positions_sha256),
                        text(&failure.retained_sample_indices_sha256),
                        text(exact_bootstrap_failure_kind(failure.kind)),
                        text(&failure.message),
                    ],
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        footnote_ids: Vec::new(),
        capability_cells,
    };
    Ok([summary, intervals, refits, failures])
}

fn exact_bootstrap_test_tail(tail: CbsemBootstrapTestTail) -> &'static str {
    match tail {
        CbsemBootstrapTestTail::TwoSided => "two_sided",
        CbsemBootstrapTestTail::OneSidedGreater => "one_sided_greater",
        CbsemBootstrapTestTail::OneSidedLess => "one_sided_less",
    }
}

fn exact_bootstrap_hypothesis_unavailable_reason(
    reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1,
) -> &'static str {
    match reason {
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::InsufficientUsableReplicates => {
            "insufficient_usable_replicates"
        }
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::NonregularVarianceBoundary => {
            "nonregular_variance_boundary"
        }
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::ZeroNullOutsideOpenDomain => {
            "zero_null_outside_open_domain"
        }
        CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::UnsupportedParameterFamily => {
            "unsupported_parameter_family"
        }
    }
}

fn exact_case_bootstrap_hypothesis_table(
    tests: &CbsemExactCaseBootstrapHypothesisTestsV1,
    expected_parameter_ids: &[String],
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> Result<CanonicalResultTable, String> {
    if tests.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1
        || tests.null_hypothesis != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1
        || tests.statistic != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1
        || tests.tie_policy != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1
        || tests.probability_method != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1
        || tests.decision_rule != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1
        || tests.null_value.to_bits() != 0.0_f64.to_bits()
        || tests.significance_level.to_bits()
            != CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1.to_bits()
        || tests.parameters.len() != expected_parameter_ids.len()
        || tests
            .parameters
            .iter()
            .zip(expected_parameter_ids)
            .any(|(parameter, expected)| parameter.parameter_id != *expected)
    {
        return Err("exact case-bootstrap hypothesis identity or parameter order drifted".into());
    }
    let (inference_status, global_reason, global_message, globally_available) =
        match &tests.inference {
            CbsemExactCaseBootstrapHypothesisTestInferenceV1::Available => {
                ("available", not_applicable(), not_applicable(), true)
            }
            CbsemExactCaseBootstrapHypothesisTestInferenceV1::Unavailable {
                reason_code,
                message,
            } if !reason_code.trim().is_empty() && !message.trim().is_empty() => {
                ("unavailable", text(reason_code), text(message), false)
            }
            CbsemExactCaseBootstrapHypothesisTestInferenceV1::Unavailable { .. } => {
                return Err(
                    "unavailable exact case-bootstrap hypothesis inference omits its reason".into(),
                );
            }
        };
    let rows = tests
        .parameters
        .iter()
        .enumerate()
        .map(|(ordinal, parameter)| {
            let (parameter_status, outcome_cells, unavailable_reason) = match &parameter.outcome {
                CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
                    point_estimate,
                    two_sided_exceedances,
                    greater_or_equal_exceedances,
                    less_or_equal_exceedances,
                    p_value_two_sided,
                    p_value_greater,
                    p_value_less,
                    selected_exceedances,
                    selected_p_value,
                    reject_null,
                } if globally_available => {
                    let denominator = f64::from(tests.usable_replicates) + 1.0;
                    let plus_one = |count: u32| (f64::from(count) + 1.0) / denominator;
                    let (expected_selected_count, expected_selected_p) =
                        match tests.selected_test_tail {
                            CbsemBootstrapTestTail::TwoSided => {
                                (*two_sided_exceedances, *p_value_two_sided)
                            }
                            CbsemBootstrapTestTail::OneSidedGreater => {
                                (*greater_or_equal_exceedances, *p_value_greater)
                            }
                            CbsemBootstrapTestTail::OneSidedLess => {
                                (*less_or_equal_exceedances, *p_value_less)
                            }
                        };
                    if !point_estimate.is_finite()
                        || point_estimate.to_bits() == (-0.0_f64).to_bits()
                        || [
                            *two_sided_exceedances,
                            *greater_or_equal_exceedances,
                            *less_or_equal_exceedances,
                            *selected_exceedances,
                        ]
                        .into_iter()
                        .any(|count| count > tests.usable_replicates)
                        || p_value_two_sided.to_bits() != plus_one(*two_sided_exceedances).to_bits()
                        || p_value_greater.to_bits()
                            != plus_one(*greater_or_equal_exceedances).to_bits()
                        || p_value_less.to_bits() != plus_one(*less_or_equal_exceedances).to_bits()
                        || *selected_exceedances != expected_selected_count
                        || selected_p_value.to_bits() != expected_selected_p.to_bits()
                        || *reject_null != (*selected_p_value <= tests.significance_level)
                    {
                        return Err(
                            "available exact case-bootstrap hypothesis arithmetic drifted"
                                .to_owned(),
                        );
                    }
                    (
                        "available",
                        vec![
                            number(*point_estimate),
                            number(f64::from(*two_sided_exceedances)),
                            number(f64::from(*greater_or_equal_exceedances)),
                            number(f64::from(*less_or_equal_exceedances)),
                            number(*p_value_two_sided),
                            number(*p_value_greater),
                            number(*p_value_less),
                            number(f64::from(*selected_exceedances)),
                            number(*selected_p_value),
                            boolean(*reject_null),
                        ],
                        not_applicable(),
                    )
                }
                CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available { .. } => {
                    return Err(
                        "globally unavailable hypothesis inference contains an available parameter"
                            .into(),
                    );
                }
                CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Unavailable { reason } => (
                    "unavailable",
                    vec![not_applicable(); 10],
                    text(exact_bootstrap_hypothesis_unavailable_reason(*reason)),
                ),
            };
            Ok(CanonicalResultRow {
                id: format!("bootstrap_hypothesis_{ordinal:04}"),
                cells: vec![
                    text(&tests.method_version),
                    text(&tests.null_hypothesis),
                    text(&tests.statistic),
                    text(&tests.tie_policy),
                    text(&tests.probability_method),
                    text(&tests.decision_rule),
                    text(exact_bootstrap_test_tail(tests.selected_test_tail)),
                    number(tests.null_value),
                    number(tests.significance_level),
                    number(f64::from(tests.usable_replicates)),
                    text(inference_status),
                    global_reason.clone(),
                    global_message.clone(),
                    text(&parameter.parameter_id),
                    text(parameter_status),
                ]
                .into_iter()
                .chain(outcome_cells)
                .chain(std::iter::once(unavailable_reason))
                .collect(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(CanonicalResultTable {
        id: CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID.into(),
        title: "Exact case-bootstrap zero-null tests".into(),
        description: Some("Null-centered selected-tail tests from the same fixed successful-refit ledger; variance-boundary and unsupported families remain explicitly unavailable.".into()),
        columns: vec![
            text_column("method_version", "Method version", "Frozen zero-null test method."),
            text_column("null_hypothesis", "Null hypothesis", "Frozen parameter-null identity."),
            text_column("statistic", "Statistic", "Frozen null-centered statistic."),
            text_column("tie_policy", "Tie policy", "Inclusive comparison policy."),
            text_column("probability_method", "Probability method", "Plus-one probability identity."),
            text_column("decision_rule", "Decision rule", "Frozen selected-tail decision rule."),
            text_column("selected_test_tail", "Selected tail", "Recipe-selected test tail."),
            number_column("null_value", "Null value", "Positive-zero null value."),
            number_column("significance_level", "Alpha", "Frozen significance level."),
            number_column("usable_replicates", "Usable", "Successful-refit denominator."),
            text_column("inference_status", "Inference status", "Global availability status."),
            text_column("global_unavailable_reason_code", "Global unavailable reason", "Typed global reason when below threshold."),
            text_column("global_unavailable_message", "Global unavailable message", "Human-readable global reason."),
            text_column("parameter_id", "Parameter ID", "Stable compiled free-parameter identity."),
            text_column("parameter_status", "Parameter status", "Available or typed unavailable outcome."),
            number_column("point_estimate", "Point estimate", "Bound original estimate."),
            number_column("two_sided_exceedances", "Two-sided count", "Inclusive null-centered two-sided exceedances."),
            number_column("greater_or_equal_exceedances", "Greater count", "Inclusive greater-tail exceedances."),
            number_column("less_or_equal_exceedances", "Less count", "Inclusive less-tail exceedances."),
            number_column("p_value_two_sided", "Two-sided p", "Plus-one two-sided probability."),
            number_column("p_value_greater", "Greater p", "Plus-one greater-tail probability."),
            number_column("p_value_less", "Less p", "Plus-one less-tail probability."),
            number_column("selected_exceedances", "Selected count", "Selected-tail exceedance count."),
            number_column("selected_p_value", "Selected p", "Selected-tail plus-one probability."),
            boolean_column("reject_null", "Reject null", "Selected p is at most alpha."),
            text_column("unavailable_reason", "Unavailable reason", "Typed parameter-level reason."),
        ],
        rows,
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    })
}

fn matrix_table(
    id: &str,
    title: &str,
    description: &str,
    cells: &[qpls_estimation::CbsemMatrixCell],
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    CanonicalResultTable {
        id: id.into(),
        title: title.into(),
        description: Some(description.into()),
        columns: vec![
            text_column("row", "Row", "Matrix row variable."),
            text_column("column", "Column", "Matrix column variable."),
            number_column("value", "Value", "Matrix cell value."),
        ],
        rows: cells
            .iter()
            .enumerate()
            .map(|(index, cell)| CanonicalResultRow {
                id: format!("cell_{index:06}"),
                cells: vec![text(&cell.row), text(&cell.column), number(cell.value)],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    }
}

fn mean_table(
    id: &str,
    title: &str,
    description: &str,
    cells: &[qpls_estimation::CbsemMeanCellV4],
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    CanonicalResultTable {
        id: id.into(),
        title: title.into(),
        description: Some(description.into()),
        columns: vec![
            text_column("variable", "Variable", "Observed variable."),
            number_column("value", "Value", "Mean-structure value."),
        ],
        rows: cells
            .iter()
            .enumerate()
            .map(|(index, cell)| CanonicalResultRow {
                id: format!("mean_{index:04}"),
                cells: vec![text(&cell.variable), number(cell.value)],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    }
}

fn mean_replacement_warning_level(level: MeanReplacementWarningLevelV1) -> &'static str {
    match level {
        MeanReplacementWarningLevelV1::None => "none",
        MeanReplacementWarningLevelV1::AtLeastFivePercent => "at_least_five_percent",
        MeanReplacementWarningLevelV1::AboveFifteenPercent => "above_fifteen_percent",
    }
}

fn mean_replacement_tables(
    receipt: &MeanReplacementReceiptV1,
    capability_cell: &qpls_core::CapabilityCellReferenceV2,
) -> Result<[CanonicalResultTable; 3], String> {
    let capability_cells = Some(vec![capability_cell.clone()]);
    let execution = CanonicalResultTable {
        id: qpls_project::MISSING_DATA_EXECUTION_TABLE_ID_V1.into(),
        title: "Missing-data execution".into(),
        description: Some(
            "Exact cell-wise mean-replacement receipt. Schema 6 validates descriptor identity, shape, and receipt consistency; archived Arrow rows are not replayed."
                .into(),
        ),
        columns: vec![
            text_column("method_version", "Method version", "Frozen missing-data method identity."),
            text_column("policy", "Policy", "Executed missing-data policy."),
            text_column("archive_validation_scope", "Archive validation scope", "Scientific validation available from schema-6 descriptors and canonical receipt tables."),
            boolean_column("raw_replay_performed", "Raw replay performed", "Whether schema-6 validation replayed source Arrow rows."),
            text_column("source_dataset_id", "Source dataset", "Exact source dataset identity."),
            text_column("source_dataset_fingerprint", "Source fingerprint", "Exact source dataset fingerprint."),
            number_column("source_row_count", "Source rows", "Rows in the immutable source dataset."),
            number_column("retained_row_count", "Retained rows", "Rows retained after cell-wise replacement."),
            number_column("omitted_row_count", "Omitted rows", "Rows omitted by the missing-data policy."),
            number_column("modeled_variable_count", "Modeled variables", "Modeled continuous variables."),
            number_column("imputed_cell_count", "Imputed cells", "Cells replaced by their variable mean."),
            number_column("affected_case_count", "Affected cases", "Rows containing at least one replaced cell."),
            number_column("variable_warning_threshold", "Variable warning threshold", "Frozen per-variable missingness warning threshold."),
            number_column("high_missingness_threshold", "High-missingness threshold", "Frozen high-missingness threshold."),
            text_column("missingness_sha256", "Missingness SHA-256", "Digest of the exact modeled missingness mask."),
            text_column("completed_matrix_sha256", "Completed matrix SHA-256", "Digest of the exact completed modeled matrix."),
            text_column("receipt_sha256", "Receipt SHA-256", "Digest of the typed mean-replacement receipt."),
        ],
        rows: vec![CanonicalResultRow {
            id: "execution".into(),
            cells: vec![
                text(&receipt.method_version),
                text("mean_replacement"),
                text(qpls_project::SCHEMA6_MISSING_DATA_VALIDATION_SCOPE_V1),
                boolean(false),
                text(&receipt.source_dataset_id),
                text(&receipt.source_dataset_fingerprint),
                number(receipt.source_row_count as f64),
                number(receipt.retained_row_count as f64),
                number(receipt.omitted_row_count as f64),
                number(receipt.modeled_variable_count as f64),
                number(receipt.imputed_cell_count as f64),
                number(receipt.affected_case_count as f64),
                number(receipt.variable_warning_threshold),
                number(receipt.high_missingness_threshold),
                text(&receipt.missingness_sha256),
                text(&receipt.completed_matrix_sha256),
                text(&receipt.receipt_sha256),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let variables = CanonicalResultTable {
        id: qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1.into(),
        title: "Mean-replacement variables".into(),
        description: Some("Canonical modeled-variable order, replacement means, missingness counts, and warnings.".into()),
        columns: vec![
            number_column("variable_order", "Order", "Zero-based canonical modeled-variable order."),
            text_column("variable_id", "Variable ID", "Stable SemModelV4 observed-variable identity."),
            text_column("source_column", "Source column", "Bound source column."),
            text_column("canonical_missing_markers_json", "Missing markers", "Canonical JSON array of declared missing markers."),
            number_column("observed_count", "Observed", "Observed finite cells."),
            number_column("missing_count", "Missing", "Cells replaced for this variable."),
            number_column("replacement_mean", "Replacement mean", "Mean calculated from observed finite cells."),
            number_column("missing_fraction", "Missing fraction", "Missing cells divided by source rows."),
            text_column("warning_level", "Warning level", "Frozen per-variable missingness warning classification."),
        ],
        rows: receipt
            .variables
            .iter()
            .enumerate()
            .map(|(index, variable)| {
                Ok(CanonicalResultRow {
                    id: format!("mean_replacement_variable_{index:04}"),
                    cells: vec![
                        number(variable.variable_order as f64),
                        text(&variable.variable_id),
                        text(&variable.source_column),
                        text(
                            serde_json::to_string(&variable.canonical_missing_markers)
                                .map_err(|error| error.to_string())?,
                        ),
                        number(variable.observed_count as f64),
                        number(variable.missing_count as f64),
                        number(variable.replacement_mean),
                        number(variable.missing_fraction),
                        text(mean_replacement_warning_level(variable.warning_level)),
                    ],
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let by_id = receipt
        .variables
        .iter()
        .map(|variable| (variable.variable_id.as_str(), variable))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut cell_rows = Vec::with_capacity(receipt.imputed_cell_count);
    for case in &receipt.cases {
        for variable_id in &case.imputed_variable_ids {
            let variable = by_id.get(variable_id.as_str()).ok_or_else(|| {
                format!("mean-replacement case references unknown variable {variable_id}")
            })?;
            let index = cell_rows.len();
            cell_rows.push(CanonicalResultRow {
                id: format!("mean_replacement_cell_{index:06}"),
                cells: vec![
                    number(case.row_index_zero_based as f64),
                    number(variable.variable_order as f64),
                    text(&variable.variable_id),
                    text(&variable.source_column),
                    number(variable.replacement_mean),
                    number(case.missing_fraction),
                    boolean(case.high_missingness_warning),
                ],
            });
        }
    }
    let cells = CanonicalResultTable {
        id: qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1.into(),
        title: "Mean-replacement cells".into(),
        description: Some("Canonical row/variable locations of every replaced cell. The table is present and may be empty when no modeled cell was missing.".into()),
        columns: vec![
            number_column("row_index_zero_based", "Row", "Zero-based source-row index."),
            number_column("variable_order", "Variable order", "Zero-based canonical modeled-variable order."),
            text_column("variable_id", "Variable ID", "Stable SemModelV4 observed-variable identity."),
            text_column("source_column", "Source column", "Bound source column."),
            number_column("replacement_mean", "Replacement mean", "Value inserted at this cell."),
            number_column("case_missing_fraction", "Case missing fraction", "Missing modeled variables divided by modeled-variable count for this row."),
            boolean_column("high_missingness_warning", "High missingness", "Whether the row exceeds the frozen high-missingness threshold."),
        ],
        rows: cell_rows,
        footnote_ids: Vec::new(),
        capability_cells,
    };
    Ok([execution, variables, cells])
}

pub(crate) fn build_recipe_v4_cbsem_canonical_result(
    job_id: Uuid,
    project_id: Uuid,
    started_at: &str,
    completed_at: &str,
    request: &InternalRecipeV4CbsemExecutionRequestV1,
    result: &RecipeV4CbsemExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, Vec<String>> {
    validate_execution_method_identity(result).map_err(|error| vec![error])?;
    let receipt = result.provenance().compilation_receipt();
    let capability_cell = receipt.capability_cell().clone();
    if capability_cell != request.capability_cell {
        return Err(vec![
            "completed CB-SEM result capability identity differs from its request".into(),
        ]);
    }
    let dataset_fingerprint = recorded_sha256(receipt.dataset_fingerprint())
        .ok_or_else(|| vec!["dataset fingerprint is not a recorded lowercase SHA-256".into()])?;
    let estimation = result.estimation();
    let analysis = &estimation.analysis;
    let legacy_bootstrap = analysis.exact_case_bootstrap.as_ref();
    let studentized_wrapper = analysis.exact_case_bootstrap_studentized.as_ref();
    let bca_wrapper = analysis.exact_case_bootstrap_bca.as_ref();
    if usize::from(legacy_bootstrap.is_some())
        + usize::from(studentized_wrapper.is_some())
        + usize::from(bca_wrapper.is_some())
        > 1
    {
        return Err(vec![
            "current exact bootstrap result mixes legacy, studentized, or BCa ownership".into(),
        ]);
    }
    let exact_bootstrap = legacy_bootstrap
        .or_else(|| studentized_wrapper.map(|wrapper| &wrapper.base))
        .or_else(|| bca_wrapper.map(|wrapper| &wrapper.base));
    if let Some(bootstrap) = exact_bootstrap {
        let Some(MethodConfig::Cbsem {
            bootstrap_v2: Some(config),
            ..
        }) = request.recipe.method_config.as_ref()
        else {
            return Err(vec![
                "current exact bootstrap result is not bound to a typed recipe configuration"
                    .into(),
            ]);
        };
        if bootstrap
            .hypothesis_tests
            .as_ref()
            .is_none_or(|tests| tests.selected_test_tail != config.test_tail)
            || (legacy_bootstrap.is_some()
                && config.interval != CbsemBootstrapInterval::PercentileType7)
            || (studentized_wrapper.is_some()
                && config.interval != CbsemBootstrapInterval::AnalyticStudentizedType7)
            || (bca_wrapper.is_some() && config.interval != CbsemBootstrapInterval::BcaType7)
        {
            return Err(vec![
                "current exact bootstrap interval or selected tail differs from its recipe".into(),
            ]);
        }
    }
    let rmsea_attribution = analysis
        .fit
        .rmsea_interval_attribution
        .as_ref()
        .ok_or_else(|| vec!["exact CB-SEM fit omitted RMSEA interval attribution".into()])?;
    if analysis.fit.method_version != CBSEM_FIT_METHOD_VERSION
        || rmsea_attribution.method_version != CBSEM_EXACT_RMSEA_INTERVAL_METHOD_VERSION_V1
        || rmsea_attribution.confidence_level.to_bits() != 0.90_f64.to_bits()
    {
        return Err(vec![
            "exact CB-SEM fit has unsupported RMSEA interval attribution".into(),
        ]);
    }
    let capability_cells = Some(vec![capability_cell.clone()]);
    let mean_replacement_receipt = estimation.input.missing_data_treatment.as_ref();

    let summary = CanonicalResultTable {
        id: "estimation_summary".into(),
        title: "Estimation summary".into(),
        description: Some(
            "Convergence, input representation, observation accounting, and exact moment provenance."
                .into(),
        ),
        columns: vec![
            text_column("model_type", "Model type", "CFA or structural equation model."),
            text_column("estimator", "Estimator", "Executed estimator."),
            text_column(
                "execution_adapter_version",
                "Execution adapter version",
                "Frozen Recipe-v4 execution adapter identity.",
            ),
            text_column(
                "estimator_method_version",
                "Estimator method version",
                "Frozen exact parameter-table estimator identity.",
            ),
            text_column(
                "moment_input_method_version",
                "Moment-input method version",
                "Frozen compiled moment-input identity.",
            ),
            number_column(
                "compiled_moment_schema_version",
                "Compiled-moment schema version",
                "Typed compiled-moment result schema.",
            ),
            boolean_column(
                "mean_structure",
                "Mean structure",
                "Whether observed intercepts and latent means were estimated.",
            ),
            text_column("input", "Input", "Raw, covariance, or correlation input."),
            boolean_column("converged", "Converged", "Whether ML optimization converged."),
            number_column("iterations", "Iterations", "Optimizer iterations."),
            number_column("objective", "Objective", "Final ML discrepancy objective."),
            number_column("gradient_norm", "Gradient norm", "Final objective-gradient norm."),
            number_column("sample_size", "Sample size", "Sample size used by the estimator."),
            number_column("declared_sample_size", "Declared sample size", "Matrix-input sample size, when applicable."),
            number_column("omitted_observations", "Omitted observations", "Raw rows removed by listwise deletion."),
            text_column("covariance_denominator", "Source denominator", "Declared source moment denominator."),
            text_column("canonical_covariance_sha256", "Canonical covariance SHA-256", "Digest of the exact ML covariance consumed."),
            text_column(
                "canonical_observed_means_sha256",
                "Observed means SHA-256",
                "Digest of the exact observed mean vector consumed, when applicable.",
            ),
        ],
        rows: vec![CanonicalResultRow {
            id: "run".into(),
            cells: vec![
                text(&analysis.model_type),
                text(&analysis.estimator),
                text(result.provenance().adapter_version()),
                text(result.provenance().estimator_method_version()),
                text(result.provenance().moment_input_method_version()),
                number(estimation.schema_version as f64),
                boolean(analysis.mean_structure),
                text(match estimation.input.kind {
                    qpls_estimation::CbsemMomentInputKindV2::Raw => "raw",
                    qpls_estimation::CbsemMomentInputKindV2::Covariance => "covariance",
                    qpls_estimation::CbsemMomentInputKindV2::Correlation => "correlation",
                }),
                boolean(analysis.converged),
                number(f64::from(analysis.iterations)),
                number(analysis.objective),
                number(analysis.gradient_norm),
                number(analysis.sample_size as f64),
                optional_usize(estimation.input.declared_sample_size),
                number(estimation.input.omitted_observations as f64),
                text(match estimation.input.covariance_denominator {
                    qpls_core::SemCovarianceDenominatorV4::SampleNMinusOne => "sample_n_minus_one",
                    qpls_core::SemCovarianceDenominatorV4::MaximumLikelihoodN => "maximum_likelihood_n",
                }),
                text(&estimation.input.canonical_ml_covariance_sha256),
                estimation
                    .input
                    .canonical_observed_means_sha256
                    .as_deref()
                    .map_or_else(missing, |value| text(value)),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let source_columns = &estimation.input.source_columns;
    let canonical_covariance = CanonicalResultTable {
        id: "canonical_ml_covariance".into(),
        title: "Canonical ML covariance".into(),
        description: Some(
            "Exact denominator-n covariance matrix supplied to the bounded ML optimizer.".into(),
        ),
        columns: std::iter::once(text_column("row", "Row", "Matrix row variable."))
            .chain(source_columns.iter().enumerate().map(|(index, column)| {
                number_column(
                    &format!("column_{index:04}"),
                    column,
                    "Canonical ML covariance cell.",
                )
            }))
            .collect(),
        rows: estimation
            .covariance_ml
            .iter()
            .enumerate()
            .map(|(row_index, row)| CanonicalResultRow {
                id: format!("row_{row_index:04}"),
                cells: std::iter::once(text(&source_columns[row_index]))
                    .chain(row.iter().copied().map(number))
                    .collect(),
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let parameters =
        CanonicalResultTable {
            id: "parameters".into(),
            title: "Model parameters".into(),
            description: Some(
                "Unstandardized estimates bound to stable SemModelV4 parameter identities.".into(),
            ),
            columns: vec![
                text_column("name", "Parameter", "Engine parameter name."),
                text_column(
                    "parameter_id",
                    "Scientific parameter ID",
                    "Stable SemModelV4 parameter identity.",
                ),
                text_column("kind", "Kind", "Parameter family."),
                text_column("lhs", "Left side", "Outcome or left endpoint."),
                text_column("rhs", "Right side", "Predictor or right endpoint."),
                number_column("estimate", "Estimate", "Unstandardized estimate."),
                number_column(
                    "standard_error",
                    "Standard error",
                    "Expected-information standard error.",
                ),
                number_column("z", "z", "Wald z statistic."),
                number_column(
                    "p_two_sided",
                    "p (two-sided)",
                    "Two-sided normal-theory probability.",
                ),
                boolean_column("fixed", "Fixed", "Whether the parameter was fixed."),
            ],
            rows: analysis
                .parameters
                .iter()
                .enumerate()
                .map(|(index, parameter)| {
                    let parameter_id = estimation.parameter_ids.get(&parameter.name).ok_or_else(
                        || {
                            format!(
                                "engine parameter {} has no stable SemModelV4 parameter identity",
                                parameter.name
                            )
                        },
                    )?;
                    Ok(CanonicalResultRow {
                        id: format!("parameter_{index:04}"),
                        cells: vec![
                            text(&parameter.name),
                            text(parameter_id),
                            text(&parameter.kind),
                            text(&parameter.lhs),
                            text(&parameter.rhs),
                            number(parameter.estimate),
                            optional_number(parameter.standard_error),
                            optional_number(parameter.z_statistic),
                            optional_number(parameter.p_value_two_sided),
                            boolean(parameter.fixed),
                        ],
                    })
                })
                .collect::<Result<Vec<_>, String>>()
                .map_err(|error| vec![error])?,
            footnote_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        };

    if estimation.parameter_ids.len() != analysis.parameters.len() {
        return Err(vec![
            "stable parameter map contains an engine/diagram cardinality mismatch".into(),
        ]);
    }

    let standardized = CanonicalResultTable {
        id: "standardized_parameters".into(),
        title: "Standardized parameters".into(),
        description: Some("Latent-variable and fully standardized parameter estimates.".into()),
        columns: vec![
            text_column("name", "Parameter", "Engine parameter name."),
            text_column("kind", "Kind", "Parameter family."),
            text_column("lhs", "Left side", "Outcome or left endpoint."),
            text_column("rhs", "Right side", "Predictor or right endpoint."),
            number_column("std_lv", "Std.lv", "Latent-variable standardized estimate."),
            number_column("std_all", "Std.all", "Fully standardized estimate."),
        ],
        rows: analysis
            .standardized
            .iter()
            .enumerate()
            .map(|(index, parameter)| CanonicalResultRow {
                id: format!("standardized_{index:04}"),
                cells: vec![
                    text(&parameter.name),
                    text(&parameter.kind),
                    text(&parameter.lhs),
                    text(&parameter.rhs),
                    number(parameter.std_lv),
                    number(parameter.std_all),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let fit = CanonicalResultTable {
        id: "fit_indices".into(),
        title: "Model fit".into(),
        description: Some(
            "Exact ML fit indices; the RMSEA 90% interval inverts the noncentral chi-square distribution using the N-1 denominator."
                .into(),
        ),
        columns: vec![
            text_column(
                "fit_method_version",
                "Fit method version",
                "Frozen fit-index method identity.",
            ),
            number_column("chi_square", "Chi-square", "Model chi-square statistic."),
            number_column(
                "degrees_of_freedom",
                "Degrees of freedom",
                "Model degrees of freedom.",
            ),
            number_column("p_value", "p", "Chi-square probability."),
            number_column("cfi", "CFI", "Comparative fit index."),
            number_column("tli", "TLI", "Tucker-Lewis index."),
            number_column("rmsea", "RMSEA", "Root mean square error of approximation."),
            text_column(
                "rmsea_interval_method_version",
                "RMSEA interval method",
                "Frozen noncentral-chi-square interval method identity.",
            ),
            number_column(
                "rmsea_interval_confidence_level",
                "RMSEA interval confidence",
                "Confidence level of the RMSEA interval.",
            ),
            number_column(
                "rmsea_ci_lower",
                "RMSEA 90% CI lower",
                "Lower RMSEA bound from noncentral-chi-square inversion using N-1.",
            ),
            number_column(
                "rmsea_ci_upper",
                "RMSEA 90% CI upper",
                "Upper RMSEA bound from noncentral-chi-square inversion using N-1.",
            ),
            number_column("srmr", "SRMR", "Standardized root mean square residual."),
            number_column("aic", "AIC", "Akaike information criterion."),
            number_column("bic", "BIC", "Bayesian information criterion."),
        ],
        rows: vec![CanonicalResultRow {
            id: "model".into(),
            cells: vec![
                text(&analysis.fit.method_version),
                number(analysis.fit.chi_square),
                number(analysis.fit.degrees_of_freedom as f64),
                optional_number(analysis.fit.p_value),
                optional_number(analysis.fit.cfi),
                optional_number(analysis.fit.tli),
                optional_number(analysis.fit.rmsea),
                text(&rmsea_attribution.method_version),
                number(rmsea_attribution.confidence_level),
                optional_number(analysis.fit.rmsea_ci_lower),
                optional_number(analysis.fit.rmsea_ci_upper),
                number(analysis.fit.srmr),
                number(analysis.fit.aic),
                number(analysis.fit.bic),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let implied = matrix_table(
        "implied_covariance",
        "Implied covariance",
        "Model-implied covariance cells.",
        &analysis.implied_covariance,
        &capability_cell,
    );
    let residual = matrix_table(
        "residual_covariance",
        "Residual covariance",
        "Observed minus model-implied covariance cells.",
        &analysis.residual_covariance,
        &capability_cell,
    );
    let residual_correlation = matrix_table(
        "residual_correlation",
        "Residual correlation",
        "Standardized residual covariance cells.",
        &analysis.residual_correlation,
        &capability_cell,
    );
    let mean_tables = if analysis.mean_structure {
        if estimation.observed_means.len() != source_columns.len()
            || estimation.implied_means.len() != source_columns.len()
            || estimation.residual_means.len() != source_columns.len()
        {
            return Err(vec![
                "CB-SEM mean vectors do not match the canonical observed-variable order".into(),
            ]);
        }
        Some([
            mean_table(
                "observed_means",
                "Observed means",
                "Canonical listwise observed means supplied to joint ML estimation.",
                &estimation.observed_means,
                &capability_cell,
            ),
            mean_table(
                "implied_means",
                "Implied means",
                "Model-implied observed means from explicit intercept and latent-mean parameters.",
                &estimation.implied_means,
                &capability_cell,
            ),
            mean_table(
                "residual_means",
                "Residual means",
                "Observed minus model-implied means.",
                &estimation.residual_means,
                &capability_cell,
            ),
        ])
    } else {
        if !estimation.observed_means.is_empty()
            || !estimation.implied_means.is_empty()
            || !estimation.residual_means.is_empty()
            || estimation.input.canonical_observed_means_sha256.is_some()
        {
            return Err(vec![
                "CB-SEM covariance-only result unexpectedly contains mean-structure payloads"
                    .into(),
            ]);
        }
        None
    };
    let score_lm = analysis
        .score_lm
        .as_ref()
        .map(|bundle| score_lm_table(bundle, &capability_cell))
        .transpose()
        .map_err(|error| vec![error])?;
    let exact_case_bootstrap_tables = exact_bootstrap
        .map(|bootstrap| exact_case_bootstrap_tables(bootstrap, &capability_cell))
        .transpose()
        .map_err(|error| vec![error])?;
    let exact_case_bootstrap_hypothesis_table = exact_bootstrap
        .and_then(|bootstrap| {
            bootstrap.hypothesis_tests.as_ref().map(|tests| {
                exact_case_bootstrap_hypothesis_table(
                    tests,
                    &bootstrap.parameter_ids,
                    &capability_cell,
                )
            })
        })
        .transpose()
        .map_err(|error| vec![error])?;
    let exact_case_bootstrap_studentized_tables = studentized_wrapper
        .map(|wrapper| {
            exact_case_bootstrap_studentized_tables(
                &wrapper.studentized,
                &wrapper.base,
                &capability_cell,
            )
        })
        .transpose()
        .map_err(|error| vec![error])?;
    let exact_case_bootstrap_bca_tables = bca_wrapper
        .map(|wrapper| {
            exact_case_bootstrap_bca_tables(&wrapper.bca, &wrapper.base, &capability_cell)
        })
        .transpose()
        .map_err(|error| vec![error])?;

    let mut notices = analysis
        .diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| CanonicalResultNotice {
            id: format!("diagnostic_{index:04}"),
            code: "cbsem_estimation_diagnostic".into(),
            severity: CanonicalNoticeSeverity::Information,
            message: diagnostic.clone(),
            section_ids: Vec::new(),
            table_ids: Vec::new(),
        })
        .collect::<Vec<_>>();
    notices.extend(
        analysis
            .warnings
            .iter()
            .enumerate()
            .map(|(index, warning)| CanonicalResultNotice {
                id: format!("warning_{index:04}"),
                code: "cbsem_estimation_warning".into(),
                severity: CanonicalNoticeSeverity::Warning,
                message: warning.clone(),
                section_ids: Vec::new(),
                table_ids: Vec::new(),
            }),
    );

    let mut sections = vec![CanonicalResultSection {
        id: "run_details".into(),
        title: "Run details".into(),
        description: Some("Input, convergence, and immutable moment provenance.".into()),
        table_ids: vec![
            "estimation_summary".into(),
            "canonical_ml_covariance".into(),
        ],
        chart_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    }];
    if mean_replacement_receipt.is_some() {
        sections.push(CanonicalResultSection {
            id: qpls_project::MISSING_DATA_SECTION_ID_V1.into(),
            title: "Missing data".into(),
            description: Some("Exact cell-wise mean-replacement execution receipt and schema-6 descriptor-only validation disclosure.".into()),
            table_ids: vec![
                qpls_project::MISSING_DATA_EXECUTION_TABLE_ID_V1.into(),
                qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1.into(),
                qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1.into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    sections.push(CanonicalResultSection {
        id: "parameters".into(),
        title: "Parameters".into(),
        description: Some("Unstandardized and standardized model parameters.".into()),
        table_ids: vec!["parameters".into(), "standardized_parameters".into()],
        chart_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    });
    if mean_tables.is_some() {
        sections.push(CanonicalResultSection {
            id: "mean_structure".into(),
            title: "Mean structure".into(),
            description: Some("Observed, model-implied, and residual means.".into()),
            table_ids: vec![
                "observed_means".into(),
                "implied_means".into(),
                "residual_means".into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    if score_lm.is_some() {
        sections.push(CanonicalResultSection {
            id: "modification_indices".into(),
            title: "Modification indices".into(),
            description: Some("Exact score/LM diagnostics for compiled, explicitly declared fixed-zero residual covariances.".into()),
            table_ids: vec!["modification_index_score_tests".into()],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    if exact_case_bootstrap_tables.is_some() {
        sections.push(CanonicalResultSection {
            id: CBSEM_EXACT_BOOTSTRAP_SECTION_ID.into(),
            title: "Bootstrap inference".into(),
            description: Some(
                "Exact CFA case-bootstrap inference with complete deterministic schedule and failure ledgers."
                    .into(),
            ),
            table_ids: vec![
                CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID.into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    if exact_case_bootstrap_hypothesis_table.is_some() {
        sections.push(CanonicalResultSection {
            id: CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID.into(),
            title: "Bootstrap hypothesis tests".into(),
            description: Some(
                "Exact null-centered selected-tail tests from the fixed successful-refit ledger."
                    .into(),
            ),
            table_ids: vec![CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID.into()],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    if exact_case_bootstrap_studentized_tables.is_some() {
        sections.push(CanonicalResultSection {
            id: CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID.into(),
            title: "Studentized bootstrap inference".into(),
            description: Some(
                "Analytic-SE studentized intervals and compact point/refit standard-error receipts bound to the unchanged base ledger."
                    .into(),
            ),
            table_ids: vec![
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID.into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    if exact_case_bootstrap_bca_tables.is_some() {
        sections.push(CanonicalResultSection {
            id: CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID.into(),
            title: "BCa bootstrap inference".into(),
            description: Some(
                "Bias-corrected and accelerated intervals plus the complete ordered delete-one evidence used for arithmetic-only reopen validation."
                    .into(),
            ),
            table_ids: vec![
                CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID.into(),
                CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID.into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        });
    }
    sections.push(CanonicalResultSection {
        id: "fit_and_residuals".into(),
        title: "Fit and residuals".into(),
        description: Some("Model fit and covariance reproduction.".into()),
        table_ids: vec![
            "fit_indices".into(),
            "implied_covariance".into(),
            "residual_covariance".into(),
            "residual_correlation".into(),
        ],
        chart_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    });
    let mut tables = vec![summary, canonical_covariance];
    if let Some(receipt) = mean_replacement_receipt {
        tables.extend(
            mean_replacement_tables(receipt, &capability_cell).map_err(|error| vec![error])?,
        );
    }
    tables.extend([
        parameters,
        standardized,
        fit,
        implied,
        residual,
        residual_correlation,
    ]);
    if let Some(mean_tables) = mean_tables {
        tables.extend(mean_tables);
    }
    if let Some(score_lm) = score_lm {
        tables.push(score_lm);
    }
    if let Some(bootstrap_tables) = exact_case_bootstrap_tables {
        tables.extend(bootstrap_tables);
    }
    if let Some(hypothesis_table) = exact_case_bootstrap_hypothesis_table {
        tables.push(hypothesis_table);
    }
    if let Some(studentized_tables) = exact_case_bootstrap_studentized_tables {
        tables.extend(studentized_tables);
    }
    if let Some(bca_tables) = exact_case_bootstrap_bca_tables {
        tables.extend(bca_tables);
    }

    let mut exclusions = vec![CanonicalResultExclusion {
        id: "bounded_cbsem_plan".into(),
        capability_cell: Some(capability_cell.clone()),
        title: "Bounded CB-SEM plan".into(),
        reason: "General structural intercepts, thresholds, groups, derived location terms, and nonrecursive mean structures are outside this internal execution slice."
            .into(),
    }];
    if exact_bootstrap.is_none() {
        exclusions.push(CanonicalResultExclusion {
            id: "point_estimation_only".into(),
            capability_cell: Some(capability_cell.clone()),
            title: "Point estimation only".into(),
            reason: "This internal Recipe-v4 CB-SEM path does not include bootstrap or other resampling inference."
                .into(),
        });
    }

    let document = CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: format!("result_{job_id}"),
        title: if analysis.model_type == "cfa" {
            "CB-SEM CFA results".into()
        } else {
            "CB-SEM results".into()
        },
        provenance: CanonicalResultProvenanceV2 {
            run_id: job_id.to_string(),
            project_id: project_id.to_string(),
            model_id: receipt.model_id().into(),
            model_digest: receipt.model_scientific_sha256().into(),
            dataset_id: result.provenance().dataset_id().into(),
            dataset_fingerprint,
            recipe_id: receipt.recipe_id().to_string(),
            recipe_digest: receipt.recipe_analytical_sha256().into(),
            capability_cell: capability_cell.clone(),
            method_version: result.provenance().estimator_method_version().into(),
            engine_version: result.provenance().adapter_version().into(),
            seed: exact_bootstrap
                .map(|bootstrap| {
                    i64::try_from(bootstrap.seed).map_err(|_| {
                        vec!["exact case-bootstrap seed exceeds canonical i64 range".into()]
                    })
                })
                .transpose()?,
            workers: if exact_bootstrap.is_some() {
                request.recipe.settings.workers as i64
            } else {
                1
            },
            started_at: started_at.into(),
            completed_at: completed_at.into(),
        },
        capability_cells: capability_cells.clone(),
        general_sem_results: None,
        sections,
        tables,
        charts: Vec::new(),
        notices,
        exclusions,
        footnotes: Vec::new(),
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some("parameters".into()),
            default_table_id: Some("parameters".into()),
            precision: 4,
            missing_value_label: "—".into(),
            chart_defaults: CanonicalChartDisplayOptions::default(),
        },
    };

    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(validation.errors);
    }
    let archive_value = serde_json::to_value(&document)
        .map_err(|error| vec![format!("canonical result serialization failed: {error}")])?;
    let archive_document =
        serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(archive_value)
            .map_err(|error| vec![format!("schema-6 canonical result parsing failed: {error}")])?;
    archive_document.ensure_valid().map_err(|error| {
        vec![format!(
            "schema-6 canonical result validation failed: {error}"
        )]
    })?;
    validate_archived_recipe_v4_cbsem_method_identity(&archive_document)
        .map_err(|error| vec![format!("schema-6 CB-SEM method identity failed: {error}")])?;
    if let Some(receipt) = mean_replacement_receipt {
        qpls_project::validate_mean_replacement_tables_against_receipt_v1(
            &archive_document,
            receipt,
        )
        .map_err(|error| {
            vec![format!(
                "schema-6 CB-SEM missing-data receipt validation failed: {error}"
            )]
        })?;
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipe_v4_cbsem_execution::{
        execute_internal_recipe_v4_cbsem, resolve_internal_recipe_v4_cbsem_dataset,
    };
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        AnalysisRecipeModelBindingV4, CbsemBootstrapAlgorithm, CbsemBootstrapConfigV2,
        CbsemBootstrapInterval, CbsemInput, MethodConfig, MissingDataPolicy, MissingDataPolicyV4,
        SemDataBindingV4, SemVariableV4, cbsem_exact_bootstrap_capability_cell_v1,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use qpls_project::{
        ProjectArchiveUpgradeRequestV6, attach_canonical_result_document_v2_v6,
        canonical_result_document_v2_json, deserialize_project_document_v6,
        plan_project_upgrade_to_v6, serialize_project_document_v6,
    };
    use std::collections::BTreeMap;

    fn replace_archived_summary_text(
        document: &mut qpls_project::CanonicalResultDocumentV2,
        column_id: &str,
        value: &str,
    ) {
        let table = document
            .tables
            .iter_mut()
            .find(|table| table.id == "estimation_summary")
            .unwrap();
        let column_index = table
            .columns
            .iter()
            .position(|column| column.id == column_id)
            .unwrap();
        let row = table.rows.iter_mut().find(|row| row.id == "run").unwrap();
        row.cells[column_index] = qpls_project::CanonicalResultCellV2::Text {
            value: value.into(),
        };
    }

    fn mean_replacement_fixture() -> (
        qpls_project::Project,
        InternalRecipeV4CbsemExecutionRequestV1,
    ) {
        let (mut project, mut request) = crate::recipe_v4_cbsem_execution::tests::fixture();
        let mut rows = Vec::new();
        for index in 0..40 {
            let centered = index as f64 - 19.5;
            let a = ((index * 7) % 11) as f64 - 5.0;
            let b = ((index * 5) % 13) as f64 - 6.0;
            rows.push([
                centered + 0.30 * a + 3.0,
                0.80 * centered + 0.50 * b + 4.4,
                0.50 * centered - 0.40 * a + 0.20 * b + 0.5,
            ]);
        }
        let means = (0..3)
            .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64)
            .collect::<Vec<_>>();
        let scales = (0..3)
            .map(|column| {
                (rows
                    .iter()
                    .map(|row| {
                        let deviation = row[column] - means[column];
                        deviation * deviation
                    })
                    .sum::<f64>()
                    / rows.len() as f64)
                    .sqrt()
            })
            .collect::<Vec<_>>();
        let mut csv = String::from("x1,x2,x3\n");
        for (index, row) in rows.into_iter().enumerate() {
            let standardized = [
                (row[0] - means[0]) / scales[0],
                (row[1] - means[1]) / scales[1],
                (row[2] - means[2]) / scales[2],
            ];
            let x1 = (index >= 2).then(|| format!("{:.17}", standardized[0]));
            let x2 = (index >= 7).then(|| format!("{:.17}", standardized[1]));
            let x3 = (index != 0).then(|| format!("{:.17}", standardized[2]));
            csv.push_str(&format!(
                "{},{},{}\n",
                x1.as_deref().unwrap_or("NA"),
                x2.as_deref().unwrap_or("NA"),
                x3.as_deref().unwrap_or("NA")
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "raw-mean-replacement-canonical.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        request.model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::MeanReplacement,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        for variable in &mut request.model.variables {
            let SemVariableV4::Observed {
                source_column,
                missing_markers,
                ..
            } = variable
            else {
                continue;
            };
            *missing_markers = dataset
                .schema
                .columns
                .iter()
                .find(|column| column.name == source_column.as_str())
                .unwrap()
                .missing_markers
                .iter()
                .map(|marker| marker.trim())
                .filter(|marker| !marker.is_empty())
                .map(str::to_owned)
                .collect();
            missing_markers.sort();
            missing_markers.dedup();
        }
        request.model.ensure_valid().unwrap();
        request.dataset_id = dataset.id.to_string();
        request.dataset_fingerprint = dataset.fingerprint.0.clone();
        request.recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        request.recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: request.model.clone(),
            scientific_sha256: request.model.scientific_sha256().unwrap(),
        };
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            ..
        }) = request.recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Raw;
        *mean_structure = false;
        project.datasets = vec![dataset];
        (project, request)
    }

    fn exact_case_bootstrap_fixture() -> (
        qpls_project::Project,
        InternalRecipeV4CbsemExecutionRequestV1,
    ) {
        let (project, mut request) = mean_replacement_fixture();
        let SemDataBindingV4::Raw { missing_data, .. } = &mut request.model.data_binding else {
            unreachable!("the shared raw fixture must retain raw data binding")
        };
        *missing_data = MissingDataPolicyV4::ListwiseDeletion;
        for variable in &mut request.model.variables {
            if let SemVariableV4::Observed {
                missing_markers, ..
            } = variable
            {
                missing_markers.clear();
            }
        }
        request.model.ensure_valid().unwrap();
        request.recipe.settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        request.recipe.settings.bootstrap_samples = 500;
        request.recipe.settings.seed = 20_260_816;
        request.recipe.settings.workers = 2;
        request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: request.model.clone(),
            scientific_sha256: request.model.scientific_sha256().unwrap(),
        };
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = request.recipe.method_config.as_mut()
        else {
            unreachable!("the shared raw fixture must retain CB-SEM configuration")
        };
        *input = CbsemInput::Raw;
        *mean_structure = false;
        *bootstrap_samples = 500;
        *bootstrap_v2 = Some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval: CbsemBootstrapInterval::PercentileType7,
            test_tail: CbsemBootstrapTestTail::TwoSided,
        });
        request.capability_cell = cbsem_exact_bootstrap_capability_cell_v1();
        (project, request)
    }

    fn exact_case_bootstrap_studentized_fixture() -> (
        qpls_project::Project,
        InternalRecipeV4CbsemExecutionRequestV1,
    ) {
        let (project, mut request) = exact_case_bootstrap_fixture();
        let Some(MethodConfig::Cbsem {
            bootstrap_v2: Some(config),
            ..
        }) = request.recipe.method_config.as_mut()
        else {
            unreachable!("the exact bootstrap fixture must retain its typed selector")
        };
        config.interval = CbsemBootstrapInterval::AnalyticStudentizedType7;
        (project, request)
    }

    fn exact_case_bootstrap_bca_fixture() -> (
        qpls_project::Project,
        InternalRecipeV4CbsemExecutionRequestV1,
    ) {
        let (project, mut request) = exact_case_bootstrap_fixture();
        let Some(MethodConfig::Cbsem {
            bootstrap_v2: Some(config),
            ..
        }) = request.recipe.method_config.as_mut()
        else {
            unreachable!("the exact bootstrap fixture must retain its typed selector")
        };
        config.interval = CbsemBootstrapInterval::BcaType7;
        (project, request)
    }

    fn set_archived_table_cell(
        document: &mut qpls_project::CanonicalResultDocumentV2,
        table_id: &str,
        row_index: usize,
        column_id: &str,
        value: qpls_project::CanonicalResultCellV2,
    ) {
        let table = document
            .tables
            .iter_mut()
            .find(|table| table.id == table_id)
            .unwrap();
        let column_index = table
            .columns
            .iter()
            .position(|column| column.id == column_id)
            .unwrap();
        table.rows[row_index].cells[column_index] = value;
    }

    #[test]
    fn cbsem_moment_result_is_canonical_and_reopens_through_schema6_exactly() {
        let (project, mut request) = crate::recipe_v4_cbsem_execution::tests::fixture();
        let left = qpls_core::SemEndpointV4::ResidualOf("observed:x1".into());
        let right = qpls_core::SemEndpointV4::ResidualOf("observed:x2".into());
        request
            .model
            .relations
            .push(qpls_core::SemRelationV4::Covariance {
                id: "residual:x1:x2".into(),
                left: left.clone(),
                right: right.clone(),
                parameter: "parameter:residual:x1:x2".into(),
            });
        request
            .model
            .parameters
            .push(qpls_core::SemParameterV4::Fixed {
                id: "parameter:residual:x1:x2".into(),
                label: "Residual covariance x1 x2".into(),
                target: qpls_core::SemParameterTargetV4::Covariance { left, right },
                value: 0.0,
                group_overrides: Vec::new(),
            });
        request.model.ensure_valid().unwrap();
        request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: request.model.clone(),
            scientific_sha256: request.model.scientific_sha256().unwrap(),
        };
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();
        let job_id = Uuid::parse_str("00000000-0000-0000-0000-00000000cb51").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &request,
            &analytical,
        )
        .unwrap();

        assert!(validate_canonical_result_document_v2(&canonical).passed);
        assert_eq!(
            canonical.provenance.capability_cell,
            request.capability_cell
        );
        assert_eq!(canonical.provenance.seed, None);
        assert_eq!(canonical.provenance.workers, 1);
        assert_eq!(
            canonical.provenance.method_version,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        );
        assert_eq!(
            canonical.provenance.engine_version,
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8
        );
        assert!(
            canonical
                .tables
                .iter()
                .any(|table| table.id == "canonical_ml_covariance")
        );
        assert!(
            canonical
                .tables
                .iter()
                .any(|table| table.id == "parameters" && table.rows.len() == 8)
        );

        let archive_canonical = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        validate_archived_recipe_v4_cbsem_method_identity(&archive_canonical).unwrap();
        let fit = archive_canonical
            .tables
            .iter()
            .find(|table| table.id == "fit_indices")
            .unwrap();
        assert_eq!(
            fit.columns
                .iter()
                .map(|column| column.id.as_str())
                .collect::<Vec<_>>(),
            CBSEM_CURRENT_FIT_COLUMNS
        );
        let score_lm = archive_canonical
            .tables
            .iter()
            .find(|table| table.id == "modification_index_score_tests")
            .unwrap();
        assert_eq!(score_lm.columns.len(), 16);
        assert_eq!(score_lm.rows.len(), 1);
        assert!(archive_canonical.sections.iter().any(|section| {
            section.id == "modification_indices"
                && section.table_ids.len() == 1
                && section.table_ids[0] == "modification_index_score_tests"
        }));
        assert!(!archive_canonical.tables.iter().any(|table| {
            matches!(
                table.id.as_str(),
                CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID
                    | CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID
                    | CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID
                    | CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID
            )
        }));
        assert!(
            !archive_canonical
                .sections
                .iter()
                .any(|section| section.id == CBSEM_EXACT_BOOTSTRAP_SECTION_ID)
        );
        let mut v8_with_bootstrap_artifact = archive_canonical.clone();
        let mut injected_bootstrap_table = score_lm.clone();
        injected_bootstrap_table.id = CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID.into();
        v8_with_bootstrap_artifact
            .tables
            .push(injected_bootstrap_table);
        assert!(
            validate_archived_recipe_v4_cbsem_method_identity(&v8_with_bootstrap_artifact).is_err(),
            "v8 must fail closed when an exact-bootstrap artifact is injected"
        );
        let mut omitted_score_lm = archive_canonical.clone();
        omitted_score_lm
            .tables
            .retain(|table| table.id != "modification_index_score_tests");
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&omitted_score_lm).is_err());
        let mut duplicated_score_lm = archive_canonical.clone();
        duplicated_score_lm.tables.push(score_lm.clone());
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&duplicated_score_lm).is_err());
        let mut v5 = archive_canonical.clone();
        v5.provenance.engine_version = RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5.into();
        replace_archived_summary_text(
            &mut v5,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5,
        );
        v5.tables
            .retain(|table| table.id != "modification_index_score_tests");
        v5.sections
            .retain(|section| section.id != "modification_indices");
        validate_archived_recipe_v4_cbsem_method_identity(&v5).unwrap();
        for (column_id, value) in [
            (
                "rmsea_interval_method_version",
                qpls_project::CanonicalResultCellV2::Text {
                    value: "tampered".into(),
                },
            ),
            (
                "rmsea_interval_confidence_level",
                qpls_project::CanonicalResultCellV2::Number {
                    value: 0.95,
                    display: None,
                },
            ),
            (
                "rmsea_ci_lower",
                qpls_project::CanonicalResultCellV2::Number {
                    value: -0.0,
                    display: None,
                },
            ),
        ] {
            let mut tampered = archive_canonical.clone();
            set_archived_table_cell(&mut tampered, "fit_indices", 0, column_id, value);
            assert!(
                validate_archived_recipe_v4_cbsem_method_identity(&tampered).is_err(),
                "{column_id} tamper must fail closed"
            );
        }
        let mut historical = archive_canonical.clone();
        historical.provenance.engine_version = RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2.into();
        replace_archived_summary_text(
            &mut historical,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2,
        );
        historical
            .tables
            .retain(|table| table.id != "modification_index_score_tests");
        historical
            .sections
            .retain(|section| section.id != "modification_indices");
        let historical_fit = historical
            .tables
            .iter_mut()
            .find(|table| table.id == "fit_indices")
            .unwrap();
        let historical_positions = historical_fit
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| {
                CBSEM_HISTORICAL_FIT_COLUMNS
                    .contains(&column.id.as_str())
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        historical_fit.columns = historical_positions
            .iter()
            .map(|index| historical_fit.columns[*index].clone())
            .collect();
        historical_fit.rows[0].cells = historical_positions
            .iter()
            .map(|index| historical_fit.rows[0].cells[*index].clone())
            .collect();
        validate_archived_recipe_v4_cbsem_method_identity(&historical).unwrap();
        let mut stale_method = archive_canonical.clone();
        stale_method.provenance.method_version = "cbsem_ml_compiled_moment_input_v2".into();
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&stale_method).is_err());
        let mut stale_engine = archive_canonical.clone();
        stale_engine.provenance.engine_version =
            "compiled_recipe_v4_cbsem_plan_v2_execution_v1".into();
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&stale_engine).is_err());
        for (column_id, stale_value) in [
            (
                "execution_adapter_version",
                "compiled_recipe_v4_cbsem_plan_v2_execution_v1",
            ),
            (
                "estimator_method_version",
                "cbsem_ml_compiled_moment_input_v2",
            ),
            (
                "moment_input_method_version",
                "cbsem_ml_compiled_moment_input_v2",
            ),
        ] {
            let mut stale_summary = archive_canonical.clone();
            replace_archived_summary_text(&mut stale_summary, column_id, stale_value);
            assert!(
                validate_archived_recipe_v4_cbsem_method_identity(&stale_summary).is_err(),
                "{column_id} tamper must fail closed"
            );
        }
        let expected_json = canonical_result_document_v2_json(&archive_canonical).unwrap();
        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: r"D:\source.qpls".into(),
                destination_archive_path: r"D:\cbsem-v6.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        attach_canonical_result_document_v2_v6(&plan.document, historical).unwrap();
        let mut current_document = plan.document;
        current_document.recipes.push(request.recipe.clone());
        current_document.ensure_valid().unwrap();
        let document =
            attach_canonical_result_document_v2_v6(&current_document, archive_canonical).unwrap();
        let bytes = serialize_project_document_v6(&document).unwrap();
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert_eq!(reopened.canonical_result_documents.len(), 1);
        let attachment = &reopened.canonical_result_documents[0];
        assert_eq!(attachment.run_id(), job_id.to_string());
        assert_eq!(
            canonical_result_document_v2_json(attachment.canonical_document()).unwrap(),
            expected_json
        );
    }

    #[test]
    fn exact_case_bootstrap_v10_500_pilot_round_trips_and_rejects_digest_tamper() {
        let (project, request) = exact_case_bootstrap_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();
        let bootstrap = analytical
            .estimation()
            .analysis
            .exact_case_bootstrap
            .as_ref()
            .expect("the exact 500-pilot aggregate must be attached");
        assert_eq!(bootstrap.requested_replicates, 500);
        assert_eq!(bootstrap.attempted_refits, 500);
        assert_eq!(
            bootstrap.usable_replicates + bootstrap.failed_replicates,
            500
        );
        assert!(matches!(
            &bootstrap.inference,
            CbsemExactCaseBootstrapInferenceV1::Unavailable { reason_code, .. }
                if reason_code == "insufficient_usable_refits"
        ));
        assert!(bootstrap.intervals.is_empty());

        let job_id = Uuid::parse_str("00000000-0000-0000-0000-00000000cb59").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &request,
            &analytical,
        )
        .unwrap();
        assert_eq!(
            canonical.provenance.engine_version,
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
        );
        let exact_table_ids = [
            CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID,
        ];
        assert!(exact_table_ids.iter().all(|id| {
            canonical
                .tables
                .iter()
                .filter(|table| table.id.as_str() == *id)
                .count()
                == 1
        }));
        assert_eq!(
            canonical
                .tables
                .iter()
                .filter(|table| table.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID)
                .count(),
            1
        );
        let bootstrap_section = canonical
            .sections
            .iter()
            .find(|section| section.id == CBSEM_EXACT_BOOTSTRAP_SECTION_ID)
            .unwrap();
        assert_eq!(
            bootstrap_section
                .table_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            exact_table_ids.to_vec()
        );

        let archive_canonical = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        validate_archived_recipe_v4_cbsem_method_identity(&archive_canonical).unwrap();
        assert_eq!(
            archived_text_cell(
                archive_canonical
                    .tables
                    .iter()
                    .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)
                    .unwrap(),
                "bootstrap",
                "inference_status",
            ),
            Some("unavailable")
        );
        assert!(
            archive_canonical
                .tables
                .iter()
                .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID)
                .unwrap()
                .rows
                .is_empty()
        );

        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: r"D:\source-exact-bootstrap.qpls".into(),
                destination_archive_path: r"D:\cbsem-exact-bootstrap-v10.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 16, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut schema6 = plan.document;
        schema6.recipes.push(request.recipe.clone());
        schema6.ensure_valid().unwrap();
        let expected_json = canonical_result_document_v2_json(&archive_canonical).unwrap();
        let attached =
            attach_canonical_result_document_v2_v6(&schema6, archive_canonical.clone()).unwrap();
        let reopened =
            deserialize_project_document_v6(&serialize_project_document_v6(&attached).unwrap())
                .unwrap();
        assert_eq!(reopened.canonical_result_documents.len(), 1);
        assert_eq!(
            canonical_result_document_v2_json(
                reopened.canonical_result_documents[0].canonical_document()
            )
            .unwrap(),
            expected_json
        );

        let alter_sha256 = |value: &str| {
            let replacement = if value.starts_with('0') { "1" } else { "0" };
            format!("{replacement}{}", &value[1..])
        };
        let mut base_point_tamper = archive_canonical.clone();
        let summary = base_point_tamper
            .tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)
            .unwrap();
        let altered_base = alter_sha256(
            archived_text_cell(summary, "bootstrap", "base_point_result_sha256").unwrap(),
        );
        set_archived_table_cell(
            &mut base_point_tamper,
            CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID,
            0,
            "base_point_result_sha256",
            qpls_project::CanonicalResultCellV2::Text {
                value: altered_base,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, base_point_tamper).is_err());

        let mut tail_tamper = archive_canonical.clone();
        set_archived_table_cell(
            &mut tail_tamper,
            CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID,
            0,
            "selected_test_tail",
            qpls_project::CanonicalResultCellV2::Text {
                value: "one_sided_greater".into(),
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, tail_tamper).is_err());

        let mut historical_v9 = archive_canonical.clone();
        historical_v9.provenance.engine_version =
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9.into();
        replace_archived_summary_text(
            &mut historical_v9,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9,
        );
        historical_v9
            .tables
            .retain(|table| table.id != CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID);
        historical_v9
            .sections
            .retain(|section| section.id != CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID);
        assert!(
            attach_canonical_result_document_v2_v6(&schema6, historical_v9.clone()).is_ok(),
            "frozen v9 bootstrap archives remain readable"
        );

        let mut injected_historical = archive_canonical.clone();
        injected_historical.provenance.engine_version =
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9.into();
        replace_archived_summary_text(
            &mut injected_historical,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9,
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, injected_historical).is_err());

        let mut injected_studentized_v10 = archive_canonical.clone();
        let mut injected_table = injected_studentized_v10
            .tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID)
            .unwrap()
            .clone();
        injected_table.id = CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID.into();
        injected_studentized_v10.tables.push(injected_table);
        assert!(
            validate_archived_recipe_v4_cbsem_method_identity(&injected_studentized_v10).is_err(),
            "v10 must reject an injected studentized table even without its section"
        );

        let ledger_table_id = if archive_canonical
            .tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID)
            .unwrap()
            .rows
            .is_empty()
        {
            CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID
        } else {
            CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID
        };
        let mut schedule_tamper = archive_canonical;
        let ledger = schedule_tamper
            .tables
            .iter()
            .find(|table| table.id == ledger_table_id)
            .unwrap();
        let row_id = ledger.rows[0].id.clone();
        let altered_schedule =
            alter_sha256(archived_text_cell(ledger, &row_id, "sampling_positions_sha256").unwrap());
        set_archived_table_cell(
            &mut schedule_tamper,
            ledger_table_id,
            0,
            "sampling_positions_sha256",
            qpls_project::CanonicalResultCellV2::Text {
                value: altered_schedule,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, schedule_tamper).is_err());
    }

    #[test]
    fn exact_case_bootstrap_v11_500_is_atomic_reopenable_and_fail_closed() {
        let (project, request) = exact_case_bootstrap_studentized_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();
        assert!(
            analytical
                .estimation()
                .analysis
                .exact_case_bootstrap
                .is_none()
        );
        let wrapper = analytical
            .estimation()
            .analysis
            .exact_case_bootstrap_studentized
            .as_ref()
            .expect("the v11 execution must attach only the atomic studentized wrapper");
        assert_eq!(wrapper.base.requested_replicates, 500);
        assert_eq!(wrapper.base.attempted_refits, 500);
        assert_eq!(
            wrapper.base.usable_replicates + wrapper.base.failed_replicates,
            500
        );
        assert_eq!(wrapper.base.minimum_usable_replicates, 1_000);
        assert!(matches!(
            &wrapper.base.inference,
            CbsemExactCaseBootstrapInferenceV1::Unavailable { .. }
        ));
        assert!(matches!(
            &wrapper.studentized.inference,
            CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable { .. }
        ));
        assert_eq!(
            wrapper.studentized.refit_standard_errors.len(),
            wrapper.base.successful_refits.len()
        );
        assert!(wrapper.base.hypothesis_tests.is_some());
        let parameter_count = wrapper.base.parameter_ids.len();
        let successful_refit_count = wrapper.base.successful_refits.len();

        let job_id = Uuid::parse_str("00000000-0000-0000-0000-00000000cb5b").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-16T01:00:00.000Z",
            "2026-08-16T01:00:01.000Z",
            &request,
            &analytical,
        )
        .unwrap();
        assert_eq!(
            canonical.provenance.engine_version,
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
        );
        let studentized_ids = [
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
        ];
        let studentized_section = canonical
            .sections
            .iter()
            .find(|section| section.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID)
            .unwrap();
        assert_eq!(
            studentized_section
                .table_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            studentized_ids
        );
        let base_section = canonical
            .sections
            .iter()
            .find(|section| section.id == CBSEM_EXACT_BOOTSTRAP_SECTION_ID)
            .unwrap();
        assert_eq!(
            base_section
                .table_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                CBSEM_EXACT_BOOTSTRAP_SUMMARY_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_INTERVALS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_REFITS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_FAILURES_TABLE_ID,
            ]
        );
        let hypothesis_section = canonical
            .sections
            .iter()
            .find(|section| section.id == CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_SECTION_ID)
            .unwrap();
        assert_eq!(
            hypothesis_section
                .table_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [CBSEM_EXACT_BOOTSTRAP_HYPOTHESIS_TABLE_ID]
        );
        for (table_id, columns) in [
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERROR_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVAL_COLUMNS,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERROR_COLUMNS,
            ),
        ] {
            let table = canonical
                .tables
                .iter()
                .find(|table| table.id == table_id)
                .unwrap();
            assert_eq!(
                table
                    .columns
                    .iter()
                    .map(|column| column.id.as_str())
                    .collect::<Vec<_>>(),
                columns
            );
            match table_id {
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID
                | CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID => {
                    assert_eq!(table.rows.len(), parameter_count)
                }
                CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID => {
                    assert_eq!(table.rows.len(), successful_refit_count)
                }
                _ => assert_eq!(table.rows.len(), 1),
            }
        }

        let archive_canonical = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        validate_archived_recipe_v4_cbsem_method_identity(&archive_canonical).unwrap();
        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "b".repeat(64),
                source_archive_path: r"D:\source-exact-bootstrap-studentized.qpls".into(),
                destination_archive_path: r"D:\cbsem-exact-bootstrap-v11.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 16, 1, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut schema6 = plan.document;
        schema6.recipes.push(request.recipe.clone());
        schema6.ensure_valid().unwrap();
        assert_eq!(
            archive_canonical.provenance.seed,
            Some(request.recipe.settings.seed)
        );
        let mut seedless = archive_canonical.clone();
        seedless.provenance.seed = None;
        assert!(attach_canonical_result_document_v2_v6(&schema6, seedless).is_err());
        let mut wrong_seed = archive_canonical.clone();
        wrong_seed.provenance.seed = Some(request.recipe.settings.seed + 1);
        assert!(attach_canonical_result_document_v2_v6(&schema6, wrong_seed).is_err());
        let attached =
            attach_canonical_result_document_v2_v6(&schema6, archive_canonical.clone()).unwrap();
        let bytes = serialize_project_document_v6(&attached).unwrap();
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert_eq!(serialize_project_document_v6(&reopened).unwrap(), bytes);
        assert_eq!(
            canonical_result_document_v2_json(
                reopened.canonical_result_documents[0].canonical_document()
            )
            .unwrap(),
            canonical_result_document_v2_json(&archive_canonical).unwrap()
        );

        let mut mixed_value = serde_json::to_value(&analytical).unwrap();
        let base_value = mixed_value
            .pointer("/estimation/analysis/exact_case_bootstrap_studentized/base")
            .unwrap()
            .clone();
        mixed_value["estimation"]["analysis"]["exact_case_bootstrap"] = base_value;
        let mixed = serde_json::from_value::<RecipeV4CbsemExecutionResultV1>(mixed_value).unwrap();
        assert!(
            build_recipe_v4_cbsem_canonical_result(
                job_id,
                project.manifest.project_id,
                "2026-08-16T01:00:00.000Z",
                "2026-08-16T01:00:01.000Z",
                &request,
                &mixed,
            )
            .is_err(),
            "legacy and studentized result ownership must not coexist"
        );

        let mut relabelled_v10 = archive_canonical.clone();
        relabelled_v10.provenance.engine_version =
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10.into();
        replace_archived_summary_text(
            &mut relabelled_v10,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10,
        );
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&relabelled_v10).is_err());

        let mut point_se_tamper = archive_canonical.clone();
        set_archived_table_cell(
            &mut point_se_tamper,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID,
            0,
            "standard_error",
            qpls_project::CanonicalResultCellV2::Number {
                value: -0.0,
                display: None,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, point_se_tamper).is_err());

        let mut refit_se_tamper = archive_canonical.clone();
        set_archived_table_cell(
            &mut refit_se_tamper,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID,
            0,
            "standard_errors_json",
            qpls_project::CanonicalResultCellV2::Text {
                value: "[-0.0]".into(),
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, refit_se_tamper).is_err());

        let mut parameter_order_tamper = archive_canonical.clone();
        let point_rows = &mut parameter_order_tamper
            .tables
            .iter_mut()
            .find(|table| {
                table.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_POINT_STANDARD_ERRORS_TABLE_ID
            })
            .unwrap()
            .rows;
        assert!(point_rows.len() >= 2);
        point_rows.swap(0, 1);
        assert!(attach_canonical_result_document_v2_v6(&schema6, parameter_order_tamper).is_err());

        let mut refit_order_tamper = archive_canonical.clone();
        let refit_rows = &mut refit_order_tamper
            .tables
            .iter_mut()
            .find(|table| {
                table.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_REFIT_STANDARD_ERRORS_TABLE_ID
            })
            .unwrap()
            .rows;
        assert!(refit_rows.len() >= 2);
        refit_rows.swap(0, 1);
        assert!(attach_canonical_result_document_v2_v6(&schema6, refit_order_tamper).is_err());

        let mut pivot_tamper = archive_canonical.clone();
        let interval_table = pivot_tamper
            .tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID)
            .unwrap();
        let lower_pivot = archived_number_cell(
            interval_table,
            &interval_table.rows[0].id,
            "lower_pivot_quantile",
        )
        .map_or(1.0, |value| value + 0.25);
        set_archived_table_cell(
            &mut pivot_tamper,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
            0,
            "lower_pivot_quantile",
            qpls_project::CanonicalResultCellV2::Number {
                value: lower_pivot,
                display: None,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, pivot_tamper).is_err());

        let mut interval_tamper = archive_canonical.clone();
        let interval_table = interval_tamper
            .tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID)
            .unwrap();
        let interval_lower =
            archived_number_cell(interval_table, &interval_table.rows[0].id, "interval_lower")
                .map_or(1.0, |value| value + 0.25);
        set_archived_table_cell(
            &mut interval_tamper,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_INTERVALS_TABLE_ID,
            0,
            "interval_lower",
            qpls_project::CanonicalResultCellV2::Number {
                value: interval_lower,
                display: None,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, interval_tamper).is_err());

        let mut signed_zero_tamper = archive_canonical.clone();
        set_archived_table_cell(
            &mut signed_zero_tamper,
            CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SUMMARY_TABLE_ID,
            0,
            "confidence_level",
            qpls_project::CanonicalResultCellV2::Number {
                value: -0.0,
                display: None,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, signed_zero_tamper).is_err());

        let mut section_order_tamper = archive_canonical.clone();
        let table_ids = &mut section_order_tamper
            .sections
            .iter_mut()
            .find(|section| section.id == CBSEM_EXACT_BOOTSTRAP_STUDENTIZED_SECTION_ID)
            .unwrap()
            .table_ids;
        table_ids.swap(0, 1);
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&section_order_tamper).is_err());
    }

    #[test]
    fn exact_case_bootstrap_v12_500_is_atomic_reopenable_and_fail_closed() {
        let (project, request) = exact_case_bootstrap_bca_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();
        let analysis = &analytical.estimation().analysis;
        assert!(analysis.exact_case_bootstrap.is_none());
        assert!(analysis.exact_case_bootstrap_studentized.is_none());
        let wrapper = analysis
            .exact_case_bootstrap_bca
            .as_ref()
            .expect("the v12 execution must attach only the atomic BCa wrapper");
        assert_eq!(wrapper.base.requested_replicates, 500);
        assert_eq!(wrapper.base.attempted_refits, 500);
        assert_eq!(
            wrapper.base.usable_replicates + wrapper.base.failed_replicates,
            500
        );
        assert_eq!(wrapper.base.minimum_usable_replicates, 1_000);
        assert!(wrapper.base.hypothesis_tests.is_some());
        assert_eq!(
            wrapper.bca.successful_delete_one_refits.len()
                + wrapper.bca.failed_delete_one_refits.len(),
            wrapper.bca.delete_one_case_count
        );
        assert_eq!(
            wrapper.bca.delete_one_case_count,
            wrapper.base.complete_case_sample_size
        );
        assert_eq!(wrapper.bca.parameter_ids, wrapper.base.parameter_ids);

        let job_id = Uuid::parse_str("00000000-0000-0000-0000-00000000cb5c").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-16T02:00:00.000Z",
            "2026-08-16T02:00:01.000Z",
            &request,
            &analytical,
        )
        .unwrap();
        assert_eq!(
            canonical.provenance.engine_version,
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
        );
        let bca_ids = [
            CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID,
            CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID,
        ];
        let section = canonical
            .sections
            .iter()
            .find(|section| section.id == CBSEM_EXACT_BOOTSTRAP_BCA_SECTION_ID)
            .unwrap();
        assert_eq!(
            section
                .table_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            bca_ids
        );
        for (table_id, columns, row_count) in [
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_COLUMNS,
                1,
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_INTERVALS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_INTERVAL_COLUMNS,
                wrapper.bca.parameter_ids.len(),
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_REFIT_COLUMNS,
                wrapper.bca.successful_delete_one_refits.len(),
            ),
            (
                CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID,
                CBSEM_EXACT_BOOTSTRAP_BCA_FAILURE_COLUMNS,
                wrapper.bca.failed_delete_one_refits.len(),
            ),
        ] {
            let table = canonical
                .tables
                .iter()
                .find(|table| table.id == table_id)
                .unwrap();
            assert_eq!(
                table
                    .columns
                    .iter()
                    .map(|column| column.id.as_str())
                    .collect::<Vec<_>>(),
                columns
            );
            assert_eq!(table.rows.len(), row_count);
        }

        let archive_canonical = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        validate_archived_recipe_v4_cbsem_method_identity(&archive_canonical).unwrap();
        let summary = archive_canonical
            .tables
            .iter()
            .find(|table| table.id == CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID)
            .unwrap();
        assert_eq!(
            archived_text_cell(summary, "bootstrap_bca", "archive_validation_scope"),
            Some(CBSEM_EXACT_BOOTSTRAP_BCA_ARCHIVE_SCOPE)
        );

        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "c".repeat(64),
                source_archive_path: r"D:\source-exact-bootstrap-bca.qpls".into(),
                destination_archive_path: r"D:\cbsem-exact-bootstrap-v12.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 16, 2, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut schema6 = plan.document;
        schema6.recipes.push(request.recipe.clone());
        schema6.ensure_valid().unwrap();
        let attached =
            attach_canonical_result_document_v2_v6(&schema6, archive_canonical.clone()).unwrap();
        let bytes = serialize_project_document_v6(&attached).unwrap();
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        assert_eq!(serialize_project_document_v6(&reopened).unwrap(), bytes);
        assert_eq!(
            canonical_result_document_v2_json(
                reopened.canonical_result_documents[0].canonical_document()
            )
            .unwrap(),
            canonical_result_document_v2_json(&archive_canonical).unwrap()
        );

        let mut mixed_value = serde_json::to_value(&analytical).unwrap();
        let base_value = mixed_value
            .pointer("/estimation/analysis/exact_case_bootstrap_bca/base")
            .unwrap()
            .clone();
        mixed_value["estimation"]["analysis"]["exact_case_bootstrap"] = base_value;
        let mixed = serde_json::from_value::<RecipeV4CbsemExecutionResultV1>(mixed_value).unwrap();
        assert!(
            build_recipe_v4_cbsem_canonical_result(
                job_id,
                project.manifest.project_id,
                "2026-08-16T02:00:00.000Z",
                "2026-08-16T02:00:01.000Z",
                &request,
                &mixed,
            )
            .is_err(),
            "legacy and BCa result ownership must not coexist"
        );

        let mut relabelled_v11 = archive_canonical.clone();
        relabelled_v11.provenance.engine_version =
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11.into();
        replace_archived_summary_text(
            &mut relabelled_v11,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11,
        );
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&relabelled_v11).is_err());

        let alter_sha256 = |value: &str| {
            let replacement = if value.starts_with('0') { "1" } else { "0" };
            format!("{replacement}{}", &value[1..])
        };
        let ledger_table_id = if wrapper.bca.successful_delete_one_refits.is_empty() {
            CBSEM_EXACT_BOOTSTRAP_BCA_FAILURES_TABLE_ID
        } else {
            CBSEM_EXACT_BOOTSTRAP_BCA_REFITS_TABLE_ID
        };
        let mut digest_tamper = archive_canonical.clone();
        let ledger = digest_tamper
            .tables
            .iter()
            .find(|table| table.id == ledger_table_id)
            .unwrap();
        let row_id = ledger.rows[0].id.clone();
        let altered_digest = alter_sha256(
            archived_text_cell(ledger, &row_id, "retained_sample_indices_sha256").unwrap(),
        );
        set_archived_table_cell(
            &mut digest_tamper,
            ledger_table_id,
            0,
            "retained_sample_indices_sha256",
            qpls_project::CanonicalResultCellV2::Text {
                value: altered_digest,
            },
        );
        assert!(attach_canonical_result_document_v2_v6(&schema6, digest_tamper).is_err());

        let mut method_tamper = archive_canonical;
        set_archived_table_cell(
            &mut method_tamper,
            CBSEM_EXACT_BOOTSTRAP_BCA_SUMMARY_TABLE_ID,
            0,
            "acceleration_method",
            qpls_project::CanonicalResultCellV2::Text {
                value: "complete_delete_one_jackknife_acceleration_v1".into(),
            },
        );
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&method_tamper).is_err());
    }

    #[test]
    fn raw_cfa_mean_v4_canonical_tables_and_schema6_identity_are_fail_closed() {
        let (project, request) = crate::recipe_v4_cbsem_execution::tests::mean_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();
        let job_id = Uuid::parse_str("00000000-0000-0000-0000-00000000cb52").unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            job_id,
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &request,
            &analytical,
        )
        .unwrap();

        assert_eq!(
            canonical.provenance.method_version,
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4
        );
        assert_eq!(
            canonical.provenance.engine_version,
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6
        );
        for table_id in ["observed_means", "implied_means", "residual_means"] {
            let table = canonical
                .tables
                .iter()
                .find(|table| table.id == table_id)
                .unwrap();
            assert_eq!(table.rows.len(), 3, "{table_id}");
        }
        assert!(
            canonical
                .sections
                .iter()
                .any(|section| section.id == "mean_structure")
        );

        let archive_canonical = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        validate_archived_recipe_v4_cbsem_method_identity(&archive_canonical).unwrap();
        let mut stale_adapter = archive_canonical.clone();
        replace_archived_summary_text(
            &mut stale_adapter,
            "execution_adapter_version",
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2,
        );
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&stale_adapter).is_err());
        let mut missing_mean_table = archive_canonical.clone();
        missing_mean_table
            .tables
            .retain(|table| table.id != "residual_means");
        assert!(validate_archived_recipe_v4_cbsem_method_identity(&missing_mean_table).is_err());

        let expected_json = canonical_result_document_v2_json(&archive_canonical).unwrap();
        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: r"D:\source-mean.qpls".into(),
                destination_archive_path: r"D:\cbsem-mean-v6.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut current_document = plan.document;
        current_document.recipes.push(request.recipe.clone());
        current_document.ensure_valid().unwrap();
        let document =
            attach_canonical_result_document_v2_v6(&current_document, archive_canonical).unwrap();
        let bytes = serialize_project_document_v6(&document).unwrap();
        let reopened = deserialize_project_document_v6(&bytes).unwrap();
        let attachment = &reopened.canonical_result_documents[0];
        validate_archived_recipe_v4_cbsem_method_identity(attachment.canonical_document()).unwrap();
        assert_eq!(
            canonical_result_document_v2_json(attachment.canonical_document()).unwrap(),
            expected_json
        );
    }

    #[test]
    fn raw_mean_replacement_receipt_is_canonical_and_schema6_tampering_fails_closed() {
        let (project, request) = mean_replacement_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let analytical = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();
        let expected_receipt = analytical
            .estimation()
            .input
            .missing_data_treatment
            .as_ref()
            .unwrap();
        let canonical = build_recipe_v4_cbsem_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-00000000cb53").unwrap(),
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &request,
            &analytical,
        )
        .unwrap();
        assert_eq!(
            canonical.provenance.engine_version,
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7
        );
        for table_id in [
            qpls_project::MISSING_DATA_EXECUTION_TABLE_ID_V1,
            qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1,
            qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1,
        ] {
            assert!(canonical.tables.iter().any(|table| table.id == table_id));
        }
        let archive_canonical = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&canonical).unwrap(),
        )
        .unwrap();
        qpls_project::validate_mean_replacement_tables_against_receipt_v1(
            &archive_canonical,
            expected_receipt,
        )
        .unwrap();
        validate_archived_recipe_v4_cbsem_method_identity(&archive_canonical).unwrap();

        let plan = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: r"D:\source-mean-replacement.qpls".into(),
                destination_archive_path: r"D:\cbsem-mean-replacement-v6.qpls".into(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 0, 0, 2).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let mut schema6 = plan.document;
        schema6.recipes.push(request.recipe.clone());
        let attached =
            attach_canonical_result_document_v2_v6(&schema6, archive_canonical.clone()).unwrap();
        let reopened =
            deserialize_project_document_v6(&serialize_project_document_v6(&attached).unwrap())
                .unwrap();
        assert_eq!(reopened.canonical_result_documents.len(), 1);

        let rejects = |tampered| {
            assert!(
                attach_canonical_result_document_v2_v6(&schema6, tampered).is_err(),
                "schema-6 must reject regenerated attachment digests over scientific tampering"
            );
        };
        let mut missing_table = archive_canonical.clone();
        missing_table
            .tables
            .retain(|table| table.id != qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1);
        rejects(missing_table);

        let mut duplicate_table = archive_canonical.clone();
        let duplicate_cells = duplicate_table
            .tables
            .iter()
            .find(|table| table.id == qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1)
            .unwrap()
            .clone();
        duplicate_table.tables.push(duplicate_cells);
        rejects(duplicate_table);

        let mut reordered = archive_canonical.clone();
        reordered
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1)
            .unwrap()
            .rows
            .swap(0, 1);
        rejects(reordered);

        let mut reordered_cells = archive_canonical.clone();
        reordered_cells
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1)
            .unwrap()
            .rows
            .swap(0, 1);
        rejects(reordered_cells);

        let mut coordinated_identity = archive_canonical.clone();
        set_archived_table_cell(
            &mut coordinated_identity,
            qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1,
            0,
            "variable_id",
            qpls_project::CanonicalResultCellV2::Text {
                value: "observed:forged".into(),
            },
        );
        let identity_cells = coordinated_identity
            .tables
            .iter()
            .find(|table| table.id == qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1)
            .unwrap()
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| match &row.cells[1] {
                qpls_project::CanonicalResultCellV2::Number { value, .. } if *value == 0.0 => {
                    Some(index)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for index in identity_cells {
            set_archived_table_cell(
                &mut coordinated_identity,
                qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1,
                index,
                "variable_id",
                qpls_project::CanonicalResultCellV2::Text {
                    value: "observed:forged".into(),
                },
            );
        }
        rejects(coordinated_identity);

        let replacement_mean = expected_receipt.variables[0].replacement_mean + 0.25;
        let mut coordinated_mean = archive_canonical.clone();
        set_archived_table_cell(
            &mut coordinated_mean,
            qpls_project::MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1,
            0,
            "replacement_mean",
            qpls_project::CanonicalResultCellV2::Number {
                value: replacement_mean,
                display: None,
            },
        );
        let affected_cells = coordinated_mean
            .tables
            .iter()
            .find(|table| table.id == qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1)
            .unwrap()
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| match &row.cells[2] {
                qpls_project::CanonicalResultCellV2::Text { value }
                    if value == &expected_receipt.variables[0].variable_id =>
                {
                    Some(index)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for index in affected_cells {
            set_archived_table_cell(
                &mut coordinated_mean,
                qpls_project::MEAN_REPLACEMENT_CELLS_TABLE_ID_V1,
                index,
                "replacement_mean",
                qpls_project::CanonicalResultCellV2::Number {
                    value: replacement_mean,
                    display: None,
                },
            );
        }
        rejects(coordinated_mean);

        let mut drifted_count = archive_canonical.clone();
        set_archived_table_cell(
            &mut drifted_count,
            qpls_project::MISSING_DATA_EXECUTION_TABLE_ID_V1,
            0,
            "imputed_cell_count",
            qpls_project::CanonicalResultCellV2::Number {
                value: (expected_receipt.imputed_cell_count + 1) as f64,
                display: None,
            },
        );
        rejects(drifted_count);

        let mut drifted_digest = archive_canonical.clone();
        set_archived_table_cell(
            &mut drifted_digest,
            qpls_project::MISSING_DATA_EXECUTION_TABLE_ID_V1,
            0,
            "missingness_sha256",
            qpls_project::CanonicalResultCellV2::Text {
                value: "0".repeat(64),
            },
        );
        rejects(drifted_digest);
    }
}
