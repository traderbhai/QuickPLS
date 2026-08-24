use qpls_core::{
    AnalysisRecipeV4, CbsemBootstrapAlgorithm, CbsemBootstrapConfigV2, CbsemBootstrapInterval,
    CbsemBootstrapTestTail, CbsemEstimator, CbsemInput, CbsemModelType, CompiledAnalysisRecipeV4,
    CompiledRecipePlanV4, MethodConfig, MissingDataPolicy, MissingDataPolicyV4,
    RecipeV4CompilationError, RecipeV4CompilationReceipt, RecipeV4CompilerTarget, SemDataBindingV4,
    SemModelV4, SemVariableV4, compile_analysis_recipe_v4,
    compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1, sha256_hex,
    validate_compiled_analysis_recipe_v4,
};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1, CBSEM_CFA_SCORE_LM_SCOPE_V1,
    CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
    CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3, CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2, CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
    CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1, CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3, CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
    CbsemCompiledMomentErrorV2, CbsemCompiledMomentResultV2, CbsemExactCaseBootstrapFailureKindV1,
    CbsemExactCaseBootstrapSourceV1, CbsemExactCaseBootstrapSourceWorkloadLimitsV1,
    CbsemExactCaseBootstrapWithBcaResultV1, CbsemExactParameterTableErrorV3,
    MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1, MEAN_REPLACEMENT_METHOD_VERSION_V1,
    MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1, MeanReplacementCaseReceiptV1,
    MeanReplacementPolicyV1, MeanReplacementReceiptV1, MeanReplacementVariableReceiptV1,
    cbsem_exact_case_bootstrap_base_point_digest_projection_v1,
    cbsem_exact_case_bootstrap_base_point_sha256_v1,
    cbsem_exact_case_bootstrap_complete_case_universe_digest_v1,
    cbsem_exact_case_bootstrap_modeled_variable_count_v1,
    estimate_cbsem_ml_compiled_moments_v2_with_control,
    estimate_cbsem_ml_exact_case_delete_one_v1_with_control,
    estimate_cbsem_ml_exact_case_resample_v1_with_control,
    estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control,
    prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_control,
    prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_workload_limits_and_control,
};
use qpls_resampling::{
    CbsemExactCaseBootstrapAttemptErrorV1, CbsemExactCaseBootstrapHypothesisTestPlanV1,
    CbsemExactCaseBootstrapScheduleV1, CbsemExactCaseBootstrapSchedulerErrorV1, ResamplingError,
    run_cbsem_exact_case_bootstrap_bca_v1, run_cbsem_exact_case_bootstrap_v1,
    run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1,
};
use serde::{Deserialize, Serialize};

use crate::RunnerProgress;

pub const RECIPE_V4_CBSEM_EXECUTION_RESULT_SCHEMA_VERSION: u32 = 1;
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v2";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V3: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v3";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V4: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v4";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v5";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v6";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v7";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v8";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V9: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v9";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v12";
pub const RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION: &str =
    RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12;

const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_WORKERS_V1: usize = 12;
const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_COMPLETE_CASES_V1: usize = 180;
const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_MODELED_VARIABLES_V1: usize = 9;
const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_FREE_PARAMETER_ROWS_V1: usize = 18;
const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_OPTIMIZER_DIMENSIONS_V1: usize = 18;
const CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_SOURCE_LIMITS_V1:
    CbsemExactCaseBootstrapSourceWorkloadLimitsV1 = CbsemExactCaseBootstrapSourceWorkloadLimitsV1 {
    maximum_complete_case_sample_size: CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_COMPLETE_CASES_V1,
    maximum_modeled_variable_count: CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_MODELED_VARIABLES_V1,
    maximum_free_parameter_row_count:
        CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_FREE_PARAMETER_ROWS_V1,
    maximum_optimizer_dimension_count:
        CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_OPTIMIZER_DIMENSIONS_V1,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4CbsemExecutionProvenanceV1 {
    adapter_version: String,
    compilation_receipt: RecipeV4CompilationReceipt,
    dataset_id: String,
    estimator_method_version: String,
    moment_input_method_version: String,
}

impl RecipeV4CbsemExecutionProvenanceV1 {
    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub fn compilation_receipt(&self) -> &RecipeV4CompilationReceipt {
        &self.compilation_receipt
    }

    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub fn estimator_method_version(&self) -> &str {
        &self.estimator_method_version
    }

    pub fn moment_input_method_version(&self) -> &str {
        &self.moment_input_method_version
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecipeV4CbsemExecutionResultV1 {
    schema_version: u32,
    provenance: RecipeV4CbsemExecutionProvenanceV1,
    estimation: CbsemCompiledMomentResultV2,
}

impl RecipeV4CbsemExecutionResultV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn provenance(&self) -> &RecipeV4CbsemExecutionProvenanceV1 {
        &self.provenance
    }

    pub fn estimation(&self) -> &CbsemCompiledMomentResultV2 {
        &self.estimation
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeV4CbsemExecutionError {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error(transparent)]
    Compilation(#[from] RecipeV4CompilationError),
    #[error("recipe-v4 CB-SEM execution requires the compiled CB-SEM plan target (found {0:?})")]
    CompilerTarget(RecipeV4CompilerTarget),
    #[error("compiled CB-SEM moment-input execution failed: {0}")]
    MomentInput(CbsemCompiledMomentErrorV2),
    #[error("CB-SEM moment-input method version changed from {expected} to {actual}")]
    MomentMethodVersionMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("CB-SEM exact estimator method version changed from {expected} to {actual}")]
    EstimatorMethodVersionMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("CB-SEM compiled-moment result schema changed from {expected} to {actual}")]
    MomentResultSchemaMismatch { expected: u32, actual: u32 },
    #[error("CB-SEM mean-replacement execution contract mismatch: {0}")]
    MeanReplacementContractMismatch(String),
    #[error("CB-SEM score/LM execution contract mismatch: {0}")]
    ScoreLmContractMismatch(String),
    #[error("exact CB-SEM case-bootstrap execution contract mismatch: {0}")]
    ExactCaseBootstrapContract(String),
    #[error("exact CB-SEM case-bootstrap scheduling failed: {0}")]
    ExactCaseBootstrapScheduler(CbsemExactCaseBootstrapSchedulerErrorV1),
}

#[derive(Serialize)]
struct MeanReplacementReceiptHashInputV1<'a> {
    method_version: &'a str,
    policy: MeanReplacementPolicyV1,
    source_dataset_id: &'a str,
    source_dataset_fingerprint: &'a str,
    source_row_count: usize,
    retained_row_count: usize,
    omitted_row_count: usize,
    modeled_variable_count: usize,
    imputed_cell_count: usize,
    affected_case_count: usize,
    variable_warning_threshold: f64,
    high_missingness_threshold: f64,
    variables: &'a [MeanReplacementVariableReceiptV1],
    cases: &'a [MeanReplacementCaseReceiptV1],
    missingness_sha256: &'a str,
    completed_matrix_sha256: &'a str,
}

fn canonical_missing_markers(markers: &[String]) -> Vec<String> {
    let mut canonical = markers
        .iter()
        .map(|marker| marker.trim())
        .filter(|marker| !marker.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    canonical.sort();
    canonical.dedup();
    canonical
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn mean_replacement_receipt_sha256(
    receipt: &MeanReplacementReceiptV1,
) -> Result<String, RecipeV4CbsemExecutionError> {
    let input = MeanReplacementReceiptHashInputV1 {
        method_version: &receipt.method_version,
        policy: receipt.policy,
        source_dataset_id: &receipt.source_dataset_id,
        source_dataset_fingerprint: &receipt.source_dataset_fingerprint,
        source_row_count: receipt.source_row_count,
        retained_row_count: receipt.retained_row_count,
        omitted_row_count: receipt.omitted_row_count,
        modeled_variable_count: receipt.modeled_variable_count,
        imputed_cell_count: receipt.imputed_cell_count,
        affected_case_count: receipt.affected_case_count,
        variable_warning_threshold: receipt.variable_warning_threshold,
        high_missingness_threshold: receipt.high_missingness_threshold,
        variables: &receipt.variables,
        cases: &receipt.cases,
        missingness_sha256: &receipt.missingness_sha256,
        completed_matrix_sha256: &receipt.completed_matrix_sha256,
    };
    let mut bytes = b"quickpls-mean-replacement-receipt-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(&input).map_err(|error| {
        RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(format!(
            "receipt hash input is not serializable: {error}"
        ))
    })?);
    Ok(sha256_hex(&bytes))
}

fn validate_mean_replacement_result(
    dataset: &Dataset,
    resolved_model: &SemModelV4,
    estimation: &CbsemCompiledMomentResultV2,
) -> Result<(), RecipeV4CbsemExecutionError> {
    let receipt = estimation
        .input
        .missing_data_treatment
        .as_ref()
        .ok_or_else(|| {
            RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                "the exact mean-replacement receipt is absent".into(),
            )
        })?;
    let identity_matches = receipt.method_version == MEAN_REPLACEMENT_METHOD_VERSION_V1
        && receipt.policy == MeanReplacementPolicyV1::MeanReplacement
        && receipt.source_dataset_id == dataset.id.to_string()
        && receipt.source_dataset_fingerprint == dataset.fingerprint.0
        && receipt.source_dataset_id == estimation.input.dataset_id
        && receipt.source_dataset_fingerprint == estimation.input.dataset_fingerprint;
    if !identity_matches {
        return Err(
            RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                "receipt method, policy, or source dataset identity differs from execution".into(),
            ),
        );
    }
    if receipt.source_row_count != dataset.batch.num_rows()
        || receipt.retained_row_count != estimation.input.used_sample_size
        || receipt.omitted_row_count != estimation.input.omitted_observations
        || receipt.retained_row_count != receipt.source_row_count
        || receipt.omitted_row_count != 0
        || receipt.modeled_variable_count != estimation.input.variable_ids.len()
        || receipt.modeled_variable_count != estimation.input.source_columns.len()
        || receipt.variables.len() != receipt.modeled_variable_count
        || receipt.affected_case_count != receipt.cases.len()
        || receipt.variable_warning_threshold.to_bits()
            != MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1.to_bits()
        || receipt.high_missingness_threshold.to_bits()
            != MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1.to_bits()
    {
        return Err(
            RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                "receipt row, variable, case, or warning-threshold counts are incoherent".into(),
            ),
        );
    }
    for (order, variable) in receipt.variables.iter().enumerate() {
        let markers = resolved_model
            .variables
            .iter()
            .find_map(|modeled| match modeled {
                SemVariableV4::Observed {
                    id,
                    source_column,
                    missing_markers,
                    ..
                } if id == &variable.variable_id && source_column == &variable.source_column => {
                    Some(canonical_missing_markers(missing_markers))
                }
                _ => None,
            });
        if variable.variable_order != order
            || estimation.input.variable_ids.get(order) != Some(&variable.variable_id)
            || estimation.input.source_columns.get(order) != Some(&variable.source_column)
            || markers.as_ref() != Some(&variable.canonical_missing_markers)
            || variable.observed_count + variable.missing_count != receipt.source_row_count
        {
            return Err(
                RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                    "receipt variable order, source binding, markers, or counts differ from execution"
                        .into(),
                ),
            );
        }
    }
    let variable_missing_cells = receipt
        .variables
        .iter()
        .map(|variable| variable.missing_count)
        .sum::<usize>();
    let case_missing_cells = receipt
        .cases
        .iter()
        .map(|case| case.imputed_variable_ids.len())
        .sum::<usize>();
    if receipt.imputed_cell_count != variable_missing_cells
        || receipt.imputed_cell_count != case_missing_cells
        || !is_lowercase_sha256(&receipt.missingness_sha256)
        || !is_lowercase_sha256(&receipt.completed_matrix_sha256)
        || !is_lowercase_sha256(&receipt.receipt_sha256)
        || mean_replacement_receipt_sha256(receipt)? != receipt.receipt_sha256
    {
        return Err(
            RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                "receipt imputation counts or SHA-256 identities do not verify".into(),
            ),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ExactCaseBootstrapRequestV1 {
    requested_replicates: u32,
    seed: u64,
    workers: usize,
    test_tail: CbsemBootstrapTestTail,
    interval: CbsemBootstrapInterval,
}

fn exact_case_bootstrap_request_v1(
    recipe: &AnalysisRecipeV4,
) -> Result<Option<ExactCaseBootstrapRequestV1>, RecipeV4CbsemExecutionError> {
    let Some(MethodConfig::Cbsem {
        model_type,
        estimator,
        input,
        mean_structure,
        bootstrap_samples,
        bootstrap_v2,
        group_column,
        invariance_steps,
    }) = recipe.method_config.as_ref()
    else {
        return Ok(None);
    };
    if recipe.settings.bootstrap_samples == 0 && *bootstrap_samples == 0 && bootstrap_v2.is_none() {
        return Ok(None);
    }
    let selected = *model_type == CbsemModelType::Cfa
        && *estimator == CbsemEstimator::Ml
        && *input == CbsemInput::Raw
        && !*mean_structure
        && group_column.is_none()
        && invariance_steps.is_empty()
        && *bootstrap_samples == recipe.settings.bootstrap_samples
        && matches!(
            bootstrap_v2,
            Some(CbsemBootstrapConfigV2 {
                algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
                interval: CbsemBootstrapInterval::PercentileType7
                    | CbsemBootstrapInterval::AnalyticStudentizedType7
                    | CbsemBootstrapInterval::BcaType7,
                ..
            })
        );
    if !selected {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            "positive or selected CB-SEM resampling is not the exact CFA case-bootstrap contract"
                .into(),
        ));
    }
    let selected_config = bootstrap_v2
        .as_ref()
        .expect("selected exact bootstrap has a typed configuration");
    Ok(Some(ExactCaseBootstrapRequestV1 {
        requested_replicates: *bootstrap_samples,
        seed: recipe.settings.seed,
        workers: recipe.settings.workers,
        test_tail: selected_config.test_tail,
        interval: selected_config.interval,
    }))
}

fn exact_case_bootstrap_uses_labs_preflight_v1(request: ExactCaseBootstrapRequestV1) -> bool {
    matches!(
        request.interval,
        CbsemBootstrapInterval::AnalyticStudentizedType7 | CbsemBootstrapInterval::BcaType7
    )
}

fn validate_exact_case_bootstrap_studentized_workload_v1(
    request: ExactCaseBootstrapRequestV1,
    complete_case_sample_size: usize,
    modeled_variable_count: usize,
    free_parameter_row_count: usize,
    optimizer_dimension_count: usize,
) -> Result<(), RecipeV4CbsemExecutionError> {
    let in_scope = (500..=10_000).contains(&request.requested_replicates)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_WORKERS_V1).contains(&request.workers)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_COMPLETE_CASES_V1)
            .contains(&complete_case_sample_size)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_MODELED_VARIABLES_V1)
            .contains(&modeled_variable_count)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_FREE_PARAMETER_ROWS_V1)
            .contains(&free_parameter_row_count)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_OPTIMIZER_DIMENSIONS_V1)
            .contains(&optimizer_dimension_count);
    if !exact_case_bootstrap_uses_labs_preflight_v1(request) || !in_scope {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            format!(
                "studentized or BCa exact CFA case bootstrap is outside the fail-closed Labs workload envelope B=500..10000, W=1..12, N=1..180, V=1..9, P=1..18, D=1..18 (actual B={}, W={}, N={}, V={}, P={}, D={})",
                request.requested_replicates,
                request.workers,
                complete_case_sample_size,
                modeled_variable_count,
                free_parameter_row_count,
                optimizer_dimension_count,
            ),
        ));
    }
    Ok(())
}

fn validate_exact_case_bootstrap_studentized_request_and_variables_v1(
    request: ExactCaseBootstrapRequestV1,
    modeled_variable_count: usize,
) -> Result<(), RecipeV4CbsemExecutionError> {
    let in_scope = exact_case_bootstrap_uses_labs_preflight_v1(request)
        && (500..=10_000).contains(&request.requested_replicates)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_WORKERS_V1).contains(&request.workers)
        && (1..=CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_MAX_MODELED_VARIABLES_V1)
            .contains(&modeled_variable_count);
    if !in_scope {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            format!(
                "studentized or BCa exact CFA case bootstrap is outside the fail-closed Labs request envelope B=500..10000, W=1..12, V=1..9 (actual B={}, W={}, V={})",
                request.requested_replicates, request.workers, modeled_variable_count,
            ),
        ));
    }
    Ok(())
}

fn map_exact_case_bootstrap_source_preflight_error_v1(
    error: CbsemCompiledMomentErrorV2,
) -> RecipeV4CbsemExecutionError {
    if matches!(
        &error,
        CbsemCompiledMomentErrorV2::ExactCaseBootstrapModeledVariableLimit { .. }
            | CbsemCompiledMomentErrorV2::ExactCaseBootstrapParameterDimensionLimit { .. }
            | CbsemCompiledMomentErrorV2::ExactCaseBootstrapCompleteCaseLimit { .. }
    ) {
        return RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(format!(
            "studentized or BCa exact CFA case bootstrap is outside the fail-closed Labs workload envelope: {error}"
        ));
    }
    match error {
        CbsemCompiledMomentErrorV2::Cancelled => RecipeV4CbsemExecutionError::Cancelled,
        other => RecipeV4CbsemExecutionError::MomentInput(other),
    }
}

fn prepare_exact_case_bootstrap_studentized_source_preflight_v1(
    dataset: &Dataset,
    execution_artifact: &CompiledAnalysisRecipeV4,
    execution_recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    request: ExactCaseBootstrapRequestV1,
    should_cancel: &(impl Fn() -> bool + Sync),
    progress: &(impl Fn(RunnerProgress) + Sync),
) -> Result<CbsemExactCaseBootstrapSourceV1, RecipeV4CbsemExecutionError> {
    let modeled_variable_count =
        cbsem_exact_case_bootstrap_modeled_variable_count_v1(resolved_model);
    validate_exact_case_bootstrap_studentized_request_and_variables_v1(
        request,
        modeled_variable_count,
    )?;

    // Cap failures must not look like point-estimation work to direct callers.
    // Buffer source-validation progress until every source dimension is known
    // to be inside the Labs envelope, then replay the successful preflight.
    let buffered_progress = std::sync::Mutex::new(Vec::new());
    let source = prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_workload_limits_and_control(
        dataset,
        execution_artifact,
        execution_recipe,
        resolved_model,
        CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_SOURCE_LIMITS_V1,
        should_cancel,
        |update| {
            buffered_progress
                .lock()
                .expect("studentized source-preflight progress mutex poisoned")
                .push(RunnerProgress {
                    phase: update.phase,
                    completed_units: update.completed_units,
                    total_units: update.total_units,
                });
        },
    )
    .map_err(map_exact_case_bootstrap_source_preflight_error_v1)?;

    validate_exact_case_bootstrap_studentized_workload_v1(
        request,
        source.complete_case_sample_size(),
        source.modeled_variable_count(),
        source.free_parameter_row_count(),
        source.optimizer_dimension_count(),
    )?;

    for update in buffered_progress
        .into_inner()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
    {
        progress(update);
        if should_cancel() {
            return Err(RecipeV4CbsemExecutionError::Cancelled);
        }
    }
    Ok(source)
}

fn exact_case_bootstrap_point_recipe_v1(
    recipe: &AnalysisRecipeV4,
) -> Result<AnalysisRecipeV4, RecipeV4CbsemExecutionError> {
    let mut point = recipe.clone();
    point.settings.bootstrap_samples = 0;
    let Some(MethodConfig::Cbsem {
        bootstrap_samples,
        bootstrap_v2,
        ..
    }) = point.method_config.as_mut()
    else {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            "exact CFA case bootstrap has no typed CB-SEM method configuration".into(),
        ));
    };
    *bootstrap_samples = 0;
    *bootstrap_v2 = None;
    Ok(point)
}

fn exact_case_bootstrap_sampling_frame_v1(
    dataset: &Dataset,
    estimation: &CbsemCompiledMomentResultV2,
) -> Result<Vec<usize>, RecipeV4CbsemExecutionError> {
    let schema = dataset.batch.schema();
    let positions = estimation
        .input
        .source_columns
        .iter()
        .map(|column| {
            schema.index_of(column).map_err(|error| {
                RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(format!(
                    "point-result source column {column} is absent from the resident dataset: {error}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let frame = (0..dataset.batch.num_rows())
        .filter(|row| {
            positions
                .iter()
                .all(|position| !dataset.batch.column(*position).is_null(*row))
        })
        .collect::<Vec<_>>();
    if frame.len() != estimation.input.used_sample_size {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            "reconstructed listwise sampling frame differs from the exact point input".into(),
        ));
    }
    Ok(frame)
}

fn exact_case_bootstrap_recorded_fingerprint_v1(
    versioned: &str,
) -> Result<&str, RecipeV4CbsemExecutionError> {
    let fingerprint = versioned.strip_prefix("v2:").unwrap_or(versioned);
    if fingerprint.len() == 64
        && fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(fingerprint)
    } else {
        Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            "resident dataset fingerprint has no canonical lowercase SHA-256 payload".into(),
        ))
    }
}

fn validate_exact_case_bootstrap_point_refit_v1(
    projection: &qpls_estimation::CbsemExactCaseBootstrapBasePointDigestProjectionV1,
    original: &qpls_estimation::CbsemExactCaseBootstrapRefitV1,
) -> Result<(), RecipeV4CbsemExecutionError> {
    let mut expected = projection
        .parameters
        .iter()
        .filter(|parameter| !parameter.fixed)
        .map(|parameter| {
            (
                parameter.parameter_id.as_str(),
                parameter.estimate.to_bits(),
            )
        })
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| left.0.cmp(right.0));
    let actual = original
        .free_parameters
        .iter()
        .map(|parameter| {
            (
                parameter.parameter_id.as_str(),
                parameter.estimate.to_bits(),
            )
        })
        .collect::<Vec<_>>();
    if expected.is_empty() || expected != actual {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            "identity-position refit differs from the stable-ID-sorted canonical point parameter IDs or estimates"
                .into(),
        ));
    }
    Ok(())
}

fn map_exact_case_bootstrap_refit_error_v1(
    error: CbsemCompiledMomentErrorV2,
) -> CbsemExactCaseBootstrapAttemptErrorV1 {
    if matches!(&error, CbsemCompiledMomentErrorV2::Cancelled) {
        return CbsemExactCaseBootstrapAttemptErrorV1::Cancelled;
    }
    let message = error.to_string();
    let kind = match error {
        CbsemCompiledMomentErrorV2::MatrixNotPositiveDefinite { .. } => {
            CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite
        }
        CbsemCompiledMomentErrorV2::NonConvergence
        | CbsemCompiledMomentErrorV2::ExactParameterTable(
            CbsemExactParameterTableErrorV3::NonConvergence
            | CbsemExactParameterTableErrorV3::OptimizerLineSearchFailed { .. }
            | CbsemExactParameterTableErrorV3::OptimizerObjectiveStagnation { .. }
            | CbsemExactParameterTableErrorV3::OptimizerIterationLimit { .. },
        ) => CbsemExactCaseBootstrapFailureKindV1::NonConvergence,
        CbsemCompiledMomentErrorV2::ExactParameterTable(
            CbsemExactParameterTableErrorV3::Numerical(_),
        ) => CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
        CbsemCompiledMomentErrorV2::ExactParameterTable(_) => {
            CbsemExactCaseBootstrapFailureKindV1::InadmissibleSolution
        }
        _ => CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
    };
    CbsemExactCaseBootstrapAttemptErrorV1::Failed { kind, message }
}

/// Executes the bounded CB-SEM Recipe-v4 artifact without projecting matrix
/// data into synthetic raw cases. The estimator revalidates the complete
/// artifact and dataset bytes, while this runner binds cancellation, progress,
/// and the immutable compilation receipt used by the native job layer.
pub fn run_compiled_cbsem_recipe_v4(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    artifact: &CompiledAnalysisRecipeV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<RecipeV4CbsemExecutionResultV1, RecipeV4CbsemExecutionError> {
    if should_cancel() {
        return Err(RecipeV4CbsemExecutionError::Cancelled);
    }
    validate_compiled_analysis_recipe_v4(artifact, recipe, Some(resolved_model))?;
    if artifact.plan().target() != RecipeV4CompilerTarget::CbsemPlanV2 {
        return Err(RecipeV4CbsemExecutionError::CompilerTarget(
            artifact.plan().target(),
        ));
    }

    let exact_case_bootstrap = exact_case_bootstrap_request_v1(recipe)?;
    if let Some(request) =
        exact_case_bootstrap.filter(|request| exact_case_bootstrap_uses_labs_preflight_v1(*request))
    {
        validate_exact_case_bootstrap_studentized_request_and_variables_v1(
            request,
            cbsem_exact_case_bootstrap_modeled_variable_count_v1(resolved_model),
        )?;
    }
    let point_execution = if exact_case_bootstrap.is_some() {
        let point_recipe = exact_case_bootstrap_point_recipe_v1(recipe)?;
        let point_artifact = compile_analysis_recipe_v4(
            &point_recipe,
            Some(resolved_model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )?;
        if point_artifact.receipt().plan_sha256() != artifact.receipt().plan_sha256()
            || point_artifact.receipt().model_scientific_sha256()
                != artifact.receipt().model_scientific_sha256()
            || point_artifact.receipt().dataset_fingerprint()
                != artifact.receipt().dataset_fingerprint()
        {
            return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                "derived point-only compiler artifact changed the outer scientific plan, model, or dataset identity"
                    .into(),
            ));
        }
        Some((point_recipe, point_artifact))
    } else {
        None
    };
    let (execution_recipe, execution_artifact) = point_execution
        .as_ref()
        .map_or((recipe, artifact), |(point_recipe, point_artifact)| {
            (point_recipe, point_artifact)
        });
    let labs_preflight_source = match exact_case_bootstrap {
        Some(request) if exact_case_bootstrap_uses_labs_preflight_v1(request) => Some(
            prepare_exact_case_bootstrap_studentized_source_preflight_v1(
                dataset,
                execution_artifact,
                execution_recipe,
                resolved_model,
                request,
                &should_cancel,
                &progress,
            )?,
        ),
        _ => None,
    };

    let mut estimation = estimate_cbsem_ml_compiled_moments_v2_with_control(
        dataset,
        execution_artifact,
        execution_recipe,
        resolved_model,
        &should_cancel,
        |update| {
            progress(RunnerProgress {
                phase: update.phase,
                completed_units: update.completed_units,
                total_units: update.total_units,
            });
        },
    )
    .map_err(|error| match error {
        CbsemCompiledMomentErrorV2::Cancelled => RecipeV4CbsemExecutionError::Cancelled,
        other => RecipeV4CbsemExecutionError::MomentInput(other),
    })?;
    if exact_case_bootstrap.is_some()
        && (estimation.analysis.bootstrap.is_some()
            || estimation.analysis.bootstrap_v2.is_some()
            || estimation.analysis.exact_case_bootstrap.is_some()
            || estimation
                .analysis
                .exact_case_bootstrap_studentized
                .is_some()
            || estimation.analysis.exact_case_bootstrap_bca.is_some()
            || estimation.analysis.multigroup.is_some())
    {
        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
            "point-only exact CFA execution unexpectedly contained nested or legacy inference"
                .into(),
        ));
    }
    if let Some(request) = exact_case_bootstrap {
        let projection = cbsem_exact_case_bootstrap_base_point_digest_projection_v1(&estimation)
            .map_err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract)?;
        let base_point_result_sha256 = cbsem_exact_case_bootstrap_base_point_sha256_v1(&projection)
            .map_err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract)?;
        let source = if exact_case_bootstrap_uses_labs_preflight_v1(request) {
            labs_preflight_source.ok_or_else(|| {
                RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                    "studentized or BCa exact CFA execution lost its bounded prepared source"
                        .into(),
                )
            })?
        } else {
            prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_control(
                dataset,
                execution_artifact,
                execution_recipe,
                resolved_model,
                &should_cancel,
                |update| {
                    progress(RunnerProgress {
                        phase: update.phase,
                        completed_units: update.completed_units,
                        total_units: update.total_units,
                    });
                },
            )
            .map_err(|error| match error {
                CbsemCompiledMomentErrorV2::Cancelled => RecipeV4CbsemExecutionError::Cancelled,
                other => RecipeV4CbsemExecutionError::MomentInput(other),
            })?
        };
        let sampling_frame = exact_case_bootstrap_sampling_frame_v1(dataset, &estimation)?;
        let recorded_fingerprint =
            exact_case_bootstrap_recorded_fingerprint_v1(&dataset.fingerprint.0)?;
        let expected_universe = cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
            recorded_fingerprint,
            dataset.batch.num_rows(),
            &sampling_frame,
        );
        if source.source_dataset_id() != dataset.id.to_string()
            || source.source_dataset_fingerprint() != recorded_fingerprint
            || source.compiler_analytical_identity_sha256()
                != execution_artifact.receipt().analytical_identity_sha256()
            || source.plan_sha256() != execution_artifact.receipt().plan_sha256()
            || source.model_scientific_sha256()
                != execution_artifact.receipt().model_scientific_sha256()
            || source.complete_case_sample_size() != sampling_frame.len()
            || source.complete_case_universe_sha256() != expected_universe
        {
            return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                "prepared exact source differs from the resident point compiler or listwise universe"
                .into(),
            ));
        }
        let identity_positions = (0..sampling_frame.len()).collect::<Vec<_>>();
        let CompiledRecipePlanV4::CbsemPlanV2 { plan } = execution_artifact.plan() else {
            return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                "exact case-bootstrap point artifact lost its compiled CB-SEM plan".into(),
            ));
        };
        let hypothesis_eligibility =
            compile_cbsem_exact_case_bootstrap_zero_null_eligibility_v1(plan);
        let schedule = CbsemExactCaseBootstrapScheduleV1 {
            outer_recipe_analytical_identity_sha256: artifact.receipt().recipe_analytical_sha256(),
            base_point_result_sha256: &base_point_result_sha256,
            requested_replicates: request.requested_replicates,
            seed: request.seed,
            stream_token: CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
            retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
            max_attempts_per_replicate: CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
            hypothesis_test: Some(CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: request.test_tail,
                parameter_eligibility: &hypothesis_eligibility,
            }),
        };
        match request.interval {
            CbsemBootstrapInterval::PercentileType7 => {
                let original = estimate_cbsem_ml_exact_case_resample_v1_with_control(
                    &source,
                    &identity_positions,
                    &should_cancel,
                    |_| {},
                )
                .map_err(|error| match error {
                    CbsemCompiledMomentErrorV2::Cancelled => RecipeV4CbsemExecutionError::Cancelled,
                    other => RecipeV4CbsemExecutionError::MomentInput(other),
                })?;
                validate_exact_case_bootstrap_point_refit_v1(&projection, &original)?;
                let aggregate = run_cbsem_exact_case_bootstrap_v1(
                    &sampling_frame,
                    &original,
                    schedule,
                    request.workers,
                    |_replicate_index, sampling_positions| {
                        estimate_cbsem_ml_exact_case_resample_v1_with_control(
                            &source,
                            sampling_positions,
                            &should_cancel,
                            |_| {},
                        )
                        .map_err(map_exact_case_bootstrap_refit_error_v1)
                    },
                    &should_cancel,
                    |update| {
                        progress(RunnerProgress {
                            phase: update.phase.as_str().into(),
                            completed_units: update.completed_replicates.into(),
                            total_units: update.total_replicates.into(),
                        });
                    },
                )
                .map_err(|error| match error {
                    CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                        ResamplingError::Cancelled,
                    ) => RecipeV4CbsemExecutionError::Cancelled,
                    other => RecipeV4CbsemExecutionError::ExactCaseBootstrapScheduler(other),
                })?;
                estimation.analysis.exact_case_bootstrap = Some(aggregate);
            }
            CbsemBootstrapInterval::AnalyticStudentizedType7 => {
                let original =
                    estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
                        &source,
                        &identity_positions,
                        &should_cancel,
                        |_| {},
                    )
                    .map_err(|error| match error {
                        CbsemCompiledMomentErrorV2::Cancelled => {
                            RecipeV4CbsemExecutionError::Cancelled
                        }
                        other => RecipeV4CbsemExecutionError::MomentInput(other),
                    })?;
                validate_exact_case_bootstrap_point_refit_v1(&projection, &original.refit)?;
                let aggregate =
                    run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
                        &sampling_frame,
                        &original,
                        schedule,
                        request.workers,
                        |_replicate_index, sampling_positions| {
                            estimate_cbsem_ml_exact_case_resample_with_analytic_standard_errors_v1_with_control(
                                &source,
                                sampling_positions,
                                &should_cancel,
                                |_| {},
                            )
                            .map_err(map_exact_case_bootstrap_refit_error_v1)
                        },
                        &should_cancel,
                        |update| {
                            progress(RunnerProgress {
                                phase: update.phase.as_str().into(),
                                completed_units: update.completed_replicates.into(),
                                total_units: update.total_replicates.into(),
                            });
                        },
                    )
                    .map_err(|error| match error {
                        CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                            ResamplingError::Cancelled,
                        ) => RecipeV4CbsemExecutionError::Cancelled,
                        other => RecipeV4CbsemExecutionError::ExactCaseBootstrapScheduler(other),
                    })?;
                estimation.analysis.exact_case_bootstrap_studentized = Some(aggregate);
            }
            CbsemBootstrapInterval::BcaType7 => {
                let original = estimate_cbsem_ml_exact_case_resample_v1_with_control(
                    &source,
                    &identity_positions,
                    &should_cancel,
                    |_| {},
                )
                .map_err(|error| match error {
                    CbsemCompiledMomentErrorV2::Cancelled => RecipeV4CbsemExecutionError::Cancelled,
                    other => RecipeV4CbsemExecutionError::MomentInput(other),
                })?;
                validate_exact_case_bootstrap_point_refit_v1(&projection, &original)?;
                let base = run_cbsem_exact_case_bootstrap_v1(
                    &sampling_frame,
                    &original,
                    schedule,
                    request.workers,
                    |_replicate_index, sampling_positions| {
                        estimate_cbsem_ml_exact_case_resample_v1_with_control(
                            &source,
                            sampling_positions,
                            &should_cancel,
                            |_| {},
                        )
                        .map_err(map_exact_case_bootstrap_refit_error_v1)
                    },
                    &should_cancel,
                    |update| {
                        progress(RunnerProgress {
                            phase: update.phase.as_str().into(),
                            completed_units: update.completed_replicates.into(),
                            total_units: update.total_replicates.into(),
                        });
                    },
                )
                .map_err(|error| match error {
                    CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                        ResamplingError::Cancelled,
                    ) => RecipeV4CbsemExecutionError::Cancelled,
                    other => RecipeV4CbsemExecutionError::ExactCaseBootstrapScheduler(other),
                })?;
                let bca = run_cbsem_exact_case_bootstrap_bca_v1(
                    &sampling_frame,
                    &original,
                    &base,
                    request.workers,
                    |omitted_position| {
                        estimate_cbsem_ml_exact_case_delete_one_v1_with_control(
                            &source,
                            omitted_position,
                            &should_cancel,
                            |_| {},
                        )
                        .map_err(map_exact_case_bootstrap_refit_error_v1)
                    },
                    &should_cancel,
                    |update| {
                        progress(RunnerProgress {
                            phase: update.phase.as_str().into(),
                            completed_units: update.completed_replicates.into(),
                            total_units: update.total_replicates.into(),
                        });
                    },
                )
                .map_err(|error| match error {
                    CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                        ResamplingError::Cancelled,
                    ) => RecipeV4CbsemExecutionError::Cancelled,
                    other => RecipeV4CbsemExecutionError::ExactCaseBootstrapScheduler(other),
                })?;
                estimation.analysis.exact_case_bootstrap_bca =
                    Some(CbsemExactCaseBootstrapWithBcaResultV1 { base, bca });
            }
        }
    }
    let recipe_mean_replacement =
        recipe.settings.missing_data == MissingDataPolicy::MeanReplacement;
    let model_mean_replacement = matches!(
        &resolved_model.data_binding,
        SemDataBindingV4::Raw {
            missing_data: MissingDataPolicyV4::MeanReplacement,
            ..
        }
    );
    if recipe_mean_replacement != model_mean_replacement {
        return Err(
            RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                "recipe and SemModelV4 policies differ".into(),
            ),
        );
    }
    let has_mean_replacement_receipt = estimation.input.missing_data_treatment.is_some();
    if !estimation.analysis.modification_indices.is_empty() {
        return Err(RecipeV4CbsemExecutionError::ScoreLmContractMismatch(
            "exact Recipe-v4 execution cannot substitute legacy heuristic modification indices"
                .into(),
        ));
    }
    let (expected_moment_method, expected_estimator_method, expected_schema, adapter_version) =
        if recipe_mean_replacement {
            if estimation.analysis.mean_structure
                || !has_mean_replacement_receipt
                || estimation.analysis.score_lm.is_some()
                || estimation.analysis.exact_case_bootstrap.is_some()
                || estimation
                    .analysis
                    .exact_case_bootstrap_studentized
                    .is_some()
                || estimation.analysis.exact_case_bootstrap_bca.is_some()
            {
                return Err(
                    RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                        "mean replacement requires covariance-structure output and its typed receipt"
                            .into(),
                    ),
                );
            }
            validate_mean_replacement_result(dataset, resolved_model, &estimation)?;
            (
                CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7,
            )
        } else if estimation.analysis.mean_structure {
            if has_mean_replacement_receipt
                || estimation.analysis.score_lm.is_some()
                || estimation.analysis.exact_case_bootstrap.is_some()
                || estimation
                    .analysis
                    .exact_case_bootstrap_studentized
                    .is_some()
                || estimation.analysis.exact_case_bootstrap_bca.is_some()
            {
                return Err(
                    RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                        "mean-structure output must not carry a mean-replacement receipt".into(),
                    ),
                );
            }
            (
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6,
            )
        } else {
            if has_mean_replacement_receipt {
                return Err(
                    RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(
                        "listwise output must not carry a mean-replacement receipt".into(),
                    ),
                );
            }
            let adapter = if let Some(score_lm) = estimation.analysis.score_lm.as_ref() {
                if score_lm.method_version != CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1
                    || score_lm.scope != CBSEM_CFA_SCORE_LM_SCOPE_V1
                {
                    return Err(RecipeV4CbsemExecutionError::ScoreLmContractMismatch(
                        "typed score/LM method or scope identity drifted".into(),
                    ));
                }
                match (
                    estimation.analysis.exact_case_bootstrap.as_ref(),
                    estimation
                        .analysis
                        .exact_case_bootstrap_studentized
                        .as_ref(),
                    estimation.analysis.exact_case_bootstrap_bca.as_ref(),
                ) {
                    (Some(_), None, None) => {
                        let bootstrap = estimation
                            .analysis
                            .exact_case_bootstrap
                            .as_ref()
                            .expect("matched base-only exact bootstrap");
                        if bootstrap.hypothesis_tests.is_none() {
                            return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                                "current exact case-bootstrap output is missing selected-tail inference"
                                    .into(),
                            ));
                        }
                        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
                    }
                    (None, Some(bootstrap), None) => {
                        if bootstrap.base.hypothesis_tests.is_none() {
                            return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                                "current analytically studentized exact case-bootstrap output is missing base selected-tail inference"
                                    .into(),
                            ));
                        }
                        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
                    }
                    (None, None, Some(bootstrap)) => {
                        if bootstrap.base.hypothesis_tests.is_none() {
                            return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                                "current BCa exact case-bootstrap output is missing base selected-tail inference"
                                    .into(),
                            ));
                        }
                        RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
                    }
                    (None, None, None) => RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8,
                    _ => {
                        return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                            "exact case-bootstrap output contains multiple mutually exclusive base or atomic sidecar envelopes"
                                .into(),
                        ));
                    }
                }
            } else {
                if estimation.analysis.exact_case_bootstrap.is_some()
                    || estimation
                        .analysis
                        .exact_case_bootstrap_studentized
                        .is_some()
                    || estimation.analysis.exact_case_bootstrap_bca.is_some()
                {
                    return Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(
                        "exact case-bootstrap output is missing its v8 score/LM point authority"
                            .into(),
                    ));
                }
                RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V5
            };
            (
                CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2,
                adapter,
            )
        };
    if estimation.method_version != expected_moment_method {
        return Err(RecipeV4CbsemExecutionError::MomentMethodVersionMismatch {
            expected: expected_moment_method,
            actual: estimation.method_version,
        });
    }
    if estimation.analysis.method_version != expected_estimator_method {
        return Err(
            RecipeV4CbsemExecutionError::EstimatorMethodVersionMismatch {
                expected: expected_estimator_method,
                actual: estimation.analysis.method_version,
            },
        );
    }
    if estimation.schema_version != expected_schema {
        return Err(RecipeV4CbsemExecutionError::MomentResultSchemaMismatch {
            expected: expected_schema,
            actual: estimation.schema_version,
        });
    }
    if should_cancel() {
        return Err(RecipeV4CbsemExecutionError::Cancelled);
    }

    Ok(RecipeV4CbsemExecutionResultV1 {
        schema_version: RECIPE_V4_CBSEM_EXECUTION_RESULT_SCHEMA_VERSION,
        provenance: RecipeV4CbsemExecutionProvenanceV1 {
            adapter_version: adapter_version.into(),
            compilation_receipt: artifact.receipt().clone(),
            dataset_id: dataset.id.to_string(),
            estimator_method_version: estimation.analysis.method_version.clone(),
            moment_input_method_version: estimation.method_version.clone(),
        },
        estimation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisSettings, CbsemEstimator, CbsemInput, CbsemModelType, Construct,
        FactorMeanPolicyV4, LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4,
        MeasurementMode, MethodConfig, MissingDataPolicyV4, ModelSpec, Preprocessing,
        SemCovarianceDenominatorV4, SemDataBindingV4, SemEndpointV4, SemMatrixSampleMetadataV4,
        SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath,
        compile_analysis_recipe_v4, convert_legacy_basic_model_v4, sha256_hex,
    };
    use qpls_data::{DataKind, ImportOptions, import_delimited_bytes, write_arrow};
    use serde_json::{Value, json};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use uuid::Uuid;

    fn fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let dataset = import_delimited_bytes(
            b",x1,x2,x3\nx1,4.0,2.0,1.2\nx2,2.0,3.0,1.0\nx3,1.2,1.0,2.0\n",
            "cbsem-covariance.csv",
            b',',
            &ImportOptions {
                data_kind: DataKind::Covariance,
                sample_size: Some(120),
                ..ImportOptions::default()
            },
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_4101),
            name: "CB-SEM runner identity fixture".into(),
            constructs: vec![Construct {
                id: "f".into(),
                name: "Factor".into(),
                short_name: "F".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            }],
            paths: Vec::<StructuralPath>::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Covariance {
            dataset_id: dataset.id.to_string(),
            variables: vec![
                "observed:x1".into(),
                "observed:x2".into(),
                "observed:x3".into(),
            ],
            means: None,
            standard_deviations: None,
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 120,
                covariance_denominator: SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        };
        model.ensure_valid().unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.workers = 1;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0xCB5E_4102),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: model.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Cfa,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Covariance,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: None,
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn mean_fixture_rows() -> Vec<Vec<f64>> {
        (0..40)
            .map(|index| {
                let centered = index as f64 - 19.5;
                let a = ((index * 7) % 11) as f64 - 5.0;
                let b = ((index * 5) % 13) as f64 - 6.0;
                vec![
                    centered + 0.30 * a + 3.0,
                    0.80 * centered + 0.50 * b + 4.4,
                    0.50 * centered - 0.40 * a + 0.20 * b + 0.5,
                ]
            })
            .collect()
    }

    fn mean_fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let (_, mut recipe, mut model, _) = fixture();
        let mut csv = String::from("x1,x2,x3\n");
        for row in mean_fixture_rows() {
            csv.push_str(&format!("{:.17},{:.17},{:.17}\n", row[0], row[1], row[2]));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "raw-cfa-mean-runner.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let factor_id = model
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor {
                    id, mean_policy, ..
                } => {
                    *mean_policy = FactorMeanPolicyV4::Estimated {
                        parameter: "parameter:factor_mean:f".into(),
                    };
                    Some(id.clone())
                }
                _ => None,
            })
            .unwrap();
        for (source, fixed, start) in [("x1", true, 0.0), ("x2", false, 2.0), ("x3", false, -1.0)] {
            let id = format!("parameter:intercept:{source}");
            let target = SemParameterTargetV4::Intercept {
                variable: format!("observed:{source}"),
            };
            model.parameters.push(if fixed {
                SemParameterV4::Fixed {
                    id,
                    label: format!("{source} intercept"),
                    target,
                    value: start,
                    group_overrides: Vec::new(),
                }
            } else {
                SemParameterV4::Free {
                    id,
                    label: format!("{source} intercept"),
                    target,
                    start: Some(start),
                    lower: Some(-20.0),
                    upper: Some(20.0),
                    equality_label: None,
                    group_overrides: Vec::new(),
                }
            });
        }
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:factor_mean:f".into(),
            label: "Factor mean".into(),
            target: SemParameterTargetV4::Mean {
                variable: factor_id,
            },
            start: Some(3.0),
            lower: Some(-20.0),
            upper: Some(20.0),
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            ..
        }) = recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Raw;
        *mean_structure = true;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn exact_case_bootstrap_fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        exact_case_bootstrap_fixture_with_interval(CbsemBootstrapInterval::PercentileType7)
    }

    fn exact_case_bootstrap_fixture_with_interval(
        interval: CbsemBootstrapInterval,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let (_, mut recipe, mut model, _) = fixture();
        let mut csv = String::from("x1,x2,x3\n");
        for (index, row) in mean_fixture_rows().into_iter().enumerate() {
            csv.push_str(&format!(
                "{},{},{}\n",
                if index == 3 {
                    "NA".into()
                } else {
                    format!("{:.17}", row[0])
                },
                if index == 17 {
                    "NA".into()
                } else {
                    format!("{:.17}", row[1])
                },
                format!("{:.17}", row[2])
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "raw-cfa-exact-case-bootstrap-runner.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        for variable in &mut model.variables {
            let SemVariableV4::Observed {
                missing_markers, ..
            } = variable
            else {
                continue;
            };
            missing_markers.clear();
        }
        model.ensure_valid().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        recipe.settings.bootstrap_samples = 500;
        recipe.settings.workers = 1;
        recipe.settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Raw;
        *mean_structure = false;
        *bootstrap_samples = 500;
        *bootstrap_v2 = Some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval,
            test_tail: CbsemBootstrapTestTail::TwoSided,
        });
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn exact_case_bootstrap_dimension_fixture(
        indicators_per_factor: &[usize],
        source_row_count: usize,
        missing_source_row: Option<usize>,
        include_factor_covariance: bool,
        equalize_two_loadings: bool,
        workers: usize,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let indicator_names = (0..indicators_per_factor.iter().sum::<usize>())
            .map(|index| format!("x{}", index + 1))
            .collect::<Vec<_>>();
        let mut csv = format!("{}\n", indicator_names.join(","));
        for source_row in 0..source_row_count {
            let cells = indicator_names
                .iter()
                .enumerate()
                .map(|(column, _)| {
                    if missing_source_row == Some(source_row) && column == 0 {
                        "NA".into()
                    } else {
                        let trend = source_row as f64 - source_row_count as f64 / 2.0;
                        let jitter = ((source_row * (column + 3) + column * 7) % 19) as f64;
                        format!(
                            "{:.17}",
                            trend * (1.0 + column as f64 * 0.03) + jitter * 0.07
                        )
                    }
                })
                .collect::<Vec<String>>();
            csv.push_str(&cells.join(","));
            csv.push('\n');
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "raw-cfa-studentized-workload-preflight.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();

        let mut next_indicator = 0;
        let constructs = indicators_per_factor
            .iter()
            .enumerate()
            .map(|(factor, count)| {
                let indicators = indicator_names[next_indicator..next_indicator + *count].to_vec();
                next_indicator += *count;
                Construct {
                    id: format!("f{}", factor + 1),
                    name: format!("Factor {}", factor + 1),
                    short_name: format!("F{}", factor + 1),
                    mode: MeasurementMode::Reflective,
                    indicators,
                }
            })
            .collect::<Vec<_>>();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_41F0),
            name: "Studentized workload preflight fixture".into(),
            constructs,
            paths: Vec::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        if include_factor_covariance {
            let left = SemEndpointV4::Variable("construct:f1".into());
            let right = SemEndpointV4::Variable("construct:f2".into());
            model.relations.push(SemRelationV4::Covariance {
                id: "relation:factor_covariance:f1:f2".into(),
                left: left.clone(),
                right: right.clone(),
                parameter: "parameter:factor_covariance:f1:f2".into(),
            });
            model.parameters.push(SemParameterV4::Free {
                id: "parameter:factor_covariance:f1:f2".into(),
                label: "Cov(F1, F2)".into(),
                target: SemParameterTargetV4::Covariance { left, right },
                start: Some(0.2),
                lower: None,
                upper: None,
                equality_label: None,
                group_overrides: Vec::new(),
            });
        }
        if equalize_two_loadings {
            let mut bound = 0;
            for parameter in &mut model.parameters {
                let SemParameterV4::Free {
                    target: SemParameterTargetV4::Loading { .. },
                    equality_label,
                    ..
                } = parameter
                else {
                    continue;
                };
                *equality_label = Some("equalized_preflight_loadings".into());
                bound += 1;
                if bound == 2 {
                    break;
                }
            }
            assert_eq!(bound, 2);
        }
        model.ensure_valid().unwrap();

        let (_, mut recipe, _, _) = fixture();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        recipe.settings.bootstrap_samples = 500;
        recipe.settings.workers = workers;
        recipe.settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Raw;
        *mean_structure = false;
        *bootstrap_samples = 500;
        *bootstrap_v2 = Some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval: CbsemBootstrapInterval::AnalyticStudentizedType7,
            test_tail: CbsemBootstrapTestTail::TwoSided,
        });
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn mean_replacement_fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let (_, mut recipe, mut model, _) = fixture();
        let rows = mean_fixture_rows();
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
            "raw-mean-replacement-runner.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::MeanReplacement,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        for variable in &mut model.variables {
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
        model.ensure_valid().unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.missing_data = MissingDataPolicy::MeanReplacement;
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: model.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            ..
        }) = recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Raw;
        *mean_structure = false;
        let artifact = compile_analysis_recipe_v4(
            &recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn raw_mean_v4_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("validation")
            .join("fixtures")
            .join("cbsem_raw_cfa_mean_structure_v4_product_fixture.json")
    }

    fn raw_mean_v4_rows_sha256(rows: &[Vec<f64>]) -> String {
        let canonical = BTreeMap::<&str, Value>::from([
            ("rows", serde_json::to_value(rows).unwrap()),
            ("variables", json!(["x1", "x2", "x3"])),
        ]);
        sha256_hex(&serde_json::to_vec(&canonical).unwrap())
    }

    fn raw_mean_v4_product_fixture() -> Value {
        let rows = mean_fixture_rows();
        let raw_sha256 = raw_mean_v4_rows_sha256(&rows);
        assert_eq!(
            raw_sha256, "5544141c7bd6bee163d5b3308a3aa75988653070f0cf01b3dc5fce3dceffd227",
            "Rust and the independent oracle must hash the preregistered rows identically"
        );

        let (dataset, recipe, model, artifact) = mean_fixture();
        let result =
            run_compiled_cbsem_recipe_v4(&dataset, &recipe, &model, &artifact, || false, |_| {})
                .unwrap();
        let estimation = result.estimation();
        let variable_order = estimation
            .observed_means
            .iter()
            .map(|cell| cell.variable.clone())
            .collect::<Vec<_>>();
        assert_eq!(variable_order, estimation.input.source_columns);

        let parameters = estimation
            .analysis
            .parameters
            .iter()
            .map(|parameter| {
                let stable_id = estimation
                    .parameter_ids
                    .get(&parameter.name)
                    .unwrap_or_else(|| panic!("missing stable id for {}", parameter.name));
                json!({
                    "name": parameter.name,
                    "stable_id": stable_id,
                    "estimate": parameter.estimate,
                    "standard_error": parameter.standard_error,
                    "fixed": parameter.fixed,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(parameters.len(), estimation.parameter_ids.len());

        json!({
            "schema_version": 1,
            "fixture_kind": "quickpls_cbsem_raw_cfa_mean_structure_v4_product_result",
            "identity": {
                "estimator": result.provenance().estimator_method_version(),
                "moment_adapter": result.provenance().moment_input_method_version(),
                "moment_result_schema_version": estimation.schema_version,
                "runner_adapter": result.provenance().adapter_version(),
            },
            "input": {
                "sample_size": estimation.input.used_sample_size,
                "variable_order": variable_order,
                "raw_sha256": raw_sha256,
                "observed_means": estimation
                    .observed_means
                    .iter()
                    .map(|cell| cell.value)
                    .collect::<Vec<_>>(),
                "covariance_ml": estimation.covariance_ml,
            },
            "parameters": parameters,
            "implied_means": estimation
                .implied_means
                .iter()
                .map(|cell| json!({ "variable": cell.variable, "value": cell.value }))
                .collect::<Vec<_>>(),
            "converged": estimation.analysis.converged,
            "objective": estimation.analysis.objective,
            "gradient_norm": estimation.analysis.gradient_norm,
        })
    }

    #[test]
    fn exact_v3_method_identity_is_bound_across_runner_provenance() {
        let (dataset, recipe, model, artifact) = fixture();

        let result =
            run_compiled_cbsem_recipe_v4(&dataset, &recipe, &model, &artifact, || false, |_| {})
                .unwrap();

        assert_eq!(
            result.schema_version(),
            RECIPE_V4_CBSEM_EXECUTION_RESULT_SCHEMA_VERSION
        );
        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8
        );
        assert_ne!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION
        );
        for actual in [
            result.provenance().moment_input_method_version(),
            result.estimation().method_version.as_str(),
        ] {
            assert_eq!(actual, CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3);
            assert_ne!(actual, "cbsem_ml_compiled_moment_input_v2");
        }
        for actual in [
            result.provenance().estimator_method_version(),
            result.estimation().analysis.method_version.as_str(),
        ] {
            assert_eq!(actual, CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3);
        }
        let score_lm = result.estimation().analysis.score_lm.as_ref().unwrap();
        assert_eq!(
            score_lm.method_version,
            CBSEM_CFA_SCORE_LM_METHOD_VERSION_V1
        );
        assert_eq!(score_lm.scope, CBSEM_CFA_SCORE_LM_SCOPE_V1);
    }

    #[test]
    fn exact_case_bootstrap_v10_binds_default_two_sided_tests_and_500_pilot_unavailability() {
        let (dataset, recipe, model, artifact) = exact_case_bootstrap_fixture();
        let point_recipe = exact_case_bootstrap_point_recipe_v1(&recipe).unwrap();
        let point_artifact = compile_analysis_recipe_v4(
            &point_recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();

        let result =
            run_compiled_cbsem_recipe_v4(&dataset, &recipe, &model, &artifact, || false, |_| {})
                .unwrap();
        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V10
        );
        assert_eq!(
            result.provenance().compilation_receipt(),
            artifact.receipt()
        );
        assert!(result.estimation().analysis.score_lm.is_some());
        assert!(result.estimation().analysis.bootstrap.is_none());
        assert!(result.estimation().analysis.bootstrap_v2.is_none());
        assert!(
            result
                .estimation()
                .analysis
                .exact_case_bootstrap_studentized
                .is_none()
        );
        assert!(
            result
                .estimation()
                .analysis
                .exact_case_bootstrap_bca
                .is_none()
        );
        assert!(
            serde_json::to_value(result.estimation()).unwrap()["analysis"]
                .get("exact_case_bootstrap_studentized")
                .is_none(),
            "the additive v11 field must remain omitted from v10 bytes"
        );
        assert!(
            serde_json::to_value(result.estimation()).unwrap()["analysis"]
                .get("exact_case_bootstrap_bca")
                .is_none(),
            "the additive v12 field must remain omitted from v10 bytes"
        );
        let aggregate = result
            .estimation()
            .analysis
            .exact_case_bootstrap
            .as_ref()
            .unwrap();
        let recorded_fingerprint =
            exact_case_bootstrap_recorded_fingerprint_v1(&dataset.fingerprint.0).unwrap();
        assert_eq!(aggregate.source_dataset_fingerprint, recorded_fingerprint);
        assert_eq!(aggregate.requested_replicates, 500);
        assert_eq!(aggregate.attempted_refits, 500);
        assert_eq!(
            aggregate.usable_replicates + aggregate.failed_replicates,
            500
        );
        assert_eq!(aggregate.minimum_usable_replicates, 1_000);
        assert!(matches!(
            aggregate.inference,
            qpls_estimation::CbsemExactCaseBootstrapInferenceV1::Unavailable { .. }
        ));
        assert!(aggregate.intervals.is_empty());
        let hypothesis = aggregate
            .hypothesis_tests
            .as_ref()
            .expect("current exact bootstrap must always emit selected-tail inference");
        assert_eq!(
            hypothesis.selected_test_tail,
            CbsemBootstrapTestTail::TwoSided
        );
        assert_eq!(hypothesis.parameters.len(), aggregate.parameter_ids.len());
        assert_eq!(
            aggregate.outer_recipe_analytical_identity_sha256,
            artifact.receipt().recipe_analytical_sha256()
        );
        assert_eq!(
            aggregate.compiler_analytical_identity_sha256,
            point_artifact.receipt().analytical_identity_sha256()
        );
        assert_eq!(
            aggregate.plan_sha256,
            point_artifact.receipt().plan_sha256()
        );
        assert_eq!(aggregate.complete_case_sample_size, 38);

        let mut point = result.estimation().clone();
        point.analysis.exact_case_bootstrap = None;
        let projection =
            cbsem_exact_case_bootstrap_base_point_digest_projection_v1(&point).unwrap();
        assert_eq!(
            aggregate.base_point_result_sha256,
            cbsem_exact_case_bootstrap_base_point_sha256_v1(&projection).unwrap()
        );
    }

    #[test]
    fn analytic_studentized_workload_caps_are_fail_closed() {
        let request = ExactCaseBootstrapRequestV1 {
            requested_replicates: 500,
            seed: 73,
            workers: 12,
            test_tail: CbsemBootstrapTestTail::TwoSided,
            interval: CbsemBootstrapInterval::AnalyticStudentizedType7,
        };
        assert!(
            validate_exact_case_bootstrap_studentized_workload_v1(request, 180, 9, 18, 18,).is_ok()
        );
        let mut bca_request = request;
        bca_request.interval = CbsemBootstrapInterval::BcaType7;
        assert!(
            validate_exact_case_bootstrap_studentized_request_and_variables_v1(bca_request, 9)
                .is_ok()
        );
        assert!(
            validate_exact_case_bootstrap_studentized_workload_v1(bca_request, 180, 9, 18, 18,)
                .is_ok()
        );

        let mut below_b = request;
        below_b.requested_replicates = 499;
        let mut above_b = request;
        above_b.requested_replicates = 10_001;
        let mut zero_w = request;
        zero_w.workers = 0;
        let mut above_w = request;
        above_w.workers = 13;
        for result in [
            validate_exact_case_bootstrap_studentized_request_and_variables_v1(below_b, 9),
            validate_exact_case_bootstrap_studentized_request_and_variables_v1(above_b, 9),
            validate_exact_case_bootstrap_studentized_request_and_variables_v1(zero_w, 9),
            validate_exact_case_bootstrap_studentized_request_and_variables_v1(above_w, 9),
            validate_exact_case_bootstrap_studentized_request_and_variables_v1(request, 10),
        ] {
            assert!(matches!(
                result,
                Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(message))
                    if message.contains("fail-closed Labs request envelope")
            ));
        }
        for result in [
            validate_exact_case_bootstrap_studentized_workload_v1(below_b, 180, 9, 18, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(above_b, 180, 9, 18, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(zero_w, 180, 9, 18, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(above_w, 180, 9, 18, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(request, 181, 9, 18, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(request, 180, 10, 18, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(request, 180, 9, 19, 18),
            validate_exact_case_bootstrap_studentized_workload_v1(request, 180, 9, 18, 19),
        ] {
            assert!(matches!(
                result,
                Err(RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(message))
                    if message.contains("fail-closed Labs workload envelope")
            ));
        }
    }

    #[test]
    fn labs_intervals_reject_caps_before_any_progress_or_point_work() {
        let cases = [
            (
                exact_case_bootstrap_dimension_fixture(&[3], 40, None, false, false, 13),
                "W=13",
            ),
            (
                exact_case_bootstrap_dimension_fixture(&[3], 181, None, false, false, 1),
                "actual N=181",
            ),
            (
                exact_case_bootstrap_dimension_fixture(&[10], 40, None, false, false, 1),
                "V=10",
            ),
            (
                exact_case_bootstrap_dimension_fixture(&[5, 4], 40, None, true, false, 1),
                "actual P=19, D=19",
            ),
        ];

        for ((dataset, recipe, model, _artifact), expected) in cases {
            for interval in [
                CbsemBootstrapInterval::AnalyticStudentizedType7,
                CbsemBootstrapInterval::BcaType7,
            ] {
                let mut selected_recipe = recipe.clone();
                let Some(MethodConfig::Cbsem {
                    bootstrap_v2: Some(config),
                    ..
                }) = selected_recipe.method_config.as_mut()
                else {
                    unreachable!()
                };
                config.interval = interval;
                let selected_artifact = compile_analysis_recipe_v4(
                    &selected_recipe,
                    Some(&model),
                    RecipeV4CompilerTarget::CbsemPlanV2,
                    RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
                )
                .unwrap();
                let progress = Mutex::new(Vec::new());
                let error = run_compiled_cbsem_recipe_v4(
                    &dataset,
                    &selected_recipe,
                    &model,
                    &selected_artifact,
                    || false,
                    |update| progress.lock().unwrap().push(update),
                )
                .unwrap_err();
                let RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(message) = &error
                else {
                    panic!("unexpected error for {interval:?} {expected}: {error}");
                };
                assert!(
                    message.contains(expected),
                    "unexpected error for {interval:?} {expected}: {error}"
                );
                assert!(
                    progress.lock().unwrap().is_empty(),
                    "over-cap {interval:?} {expected} must fail before source, point, refit, or bootstrap progress"
                );
            }
        }
    }

    #[test]
    fn analytic_studentized_bounded_source_accepts_all_caps_and_counts_only_complete_rows() {
        let (dataset, recipe, model, _) =
            exact_case_bootstrap_dimension_fixture(&[5, 4], 181, Some(180), false, false, 12);
        let point_recipe = exact_case_bootstrap_point_recipe_v1(&recipe).unwrap();
        let point_artifact = compile_analysis_recipe_v4(
            &point_recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();
        let request = exact_case_bootstrap_request_v1(&recipe).unwrap().unwrap();
        let progress = Mutex::new(Vec::new());

        let source = prepare_exact_case_bootstrap_studentized_source_preflight_v1(
            &dataset,
            &point_artifact,
            &point_recipe,
            &model,
            request,
            &|| false,
            &|update| progress.lock().unwrap().push(update),
        )
        .unwrap();

        assert_eq!(source.source_row_count(), 181);
        assert_eq!(source.complete_case_sample_size(), 180);
        assert_eq!(source.modeled_variable_count(), 9);
        assert_eq!(source.free_parameter_row_count(), 18);
        assert_eq!(source.optimizer_dimension_count(), 18);
        assert!(progress.lock().unwrap().iter().all(|update| {
            !matches!(
                update.phase.as_str(),
                "moments" | "estimation" | "refit" | "bootstrap"
            )
        }));
    }

    #[test]
    fn bounded_source_uses_equality_aware_optimizer_dimensions_without_duplicated_formulas() {
        let (dataset, recipe, model, _) =
            exact_case_bootstrap_dimension_fixture(&[5, 4], 40, None, true, true, 1);
        let point_recipe = exact_case_bootstrap_point_recipe_v1(&recipe).unwrap();
        let point_artifact = compile_analysis_recipe_v4(
            &point_recipe,
            Some(&model),
            RecipeV4CompilerTarget::CbsemPlanV2,
            RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        )
        .unwrap();

        let source =
            prepare_cbsem_ml_exact_case_bootstrap_source_v1_with_workload_limits_and_control(
                &dataset,
                &point_artifact,
                &point_recipe,
                &model,
                CbsemExactCaseBootstrapSourceWorkloadLimitsV1 {
                    maximum_complete_case_sample_size: 180,
                    maximum_modeled_variable_count: 9,
                    maximum_free_parameter_row_count: 19,
                    maximum_optimizer_dimension_count: 18,
                },
                || false,
                |_| {},
            )
            .unwrap();

        assert_eq!(source.free_parameter_row_count(), 19);
        assert_eq!(source.optimizer_dimension_count(), 18);
    }

    #[test]
    fn exact_case_bootstrap_v11_stores_only_atomic_studentized_wrapper() {
        let (dataset, recipe, model, artifact) = exact_case_bootstrap_fixture_with_interval(
            CbsemBootstrapInterval::AnalyticStudentizedType7,
        );
        let result =
            run_compiled_cbsem_recipe_v4(&dataset, &recipe, &model, &artifact, || false, |_| {})
                .unwrap();

        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V11
        );
        assert!(result.estimation().analysis.exact_case_bootstrap.is_none());
        assert!(
            result
                .estimation()
                .analysis
                .exact_case_bootstrap_bca
                .is_none()
        );
        let aggregate = result
            .estimation()
            .analysis
            .exact_case_bootstrap_studentized
            .as_ref()
            .expect("v11 must retain the base and studentized evidence atomically");
        assert_eq!(aggregate.base.requested_replicates, 500);
        assert_eq!(aggregate.base.attempted_refits, 500);
        assert_eq!(aggregate.base.minimum_usable_replicates, 1_000);
        assert!(matches!(
            aggregate.base.inference,
            qpls_estimation::CbsemExactCaseBootstrapInferenceV1::Unavailable { .. }
        ));
        assert!(matches!(
            aggregate.studentized.inference,
            qpls_estimation::CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable { .. }
        ));
        assert_eq!(
            aggregate.studentized.refit_standard_errors.len(),
            aggregate.base.successful_refits.len()
        );
        let analysis_wire = serde_json::to_value(&result.estimation().analysis).unwrap();
        assert!(analysis_wire.get("exact_case_bootstrap").is_none());
        assert!(
            analysis_wire
                .get("exact_case_bootstrap_studentized")
                .is_some()
        );
        assert!(analysis_wire.get("exact_case_bootstrap_bca").is_none());

        let mut point = result.estimation().clone();
        point.analysis.exact_case_bootstrap_studentized = None;
        let projection =
            cbsem_exact_case_bootstrap_base_point_digest_projection_v1(&point).unwrap();
        assert_eq!(
            aggregate.base.base_point_result_sha256,
            cbsem_exact_case_bootstrap_base_point_sha256_v1(&projection).unwrap()
        );
    }

    #[test]
    fn exact_case_bootstrap_v12_stores_atomic_bca_wrapper_and_b500_evidence() {
        let (dataset, recipe, model, artifact) =
            exact_case_bootstrap_fixture_with_interval(CbsemBootstrapInterval::BcaType7);
        let result =
            run_compiled_cbsem_recipe_v4(&dataset, &recipe, &model, &artifact, || false, |_| {})
                .unwrap();

        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V12
        );
        assert!(result.estimation().analysis.exact_case_bootstrap.is_none());
        assert!(
            result
                .estimation()
                .analysis
                .exact_case_bootstrap_studentized
                .is_none()
        );
        let aggregate = result
            .estimation()
            .analysis
            .exact_case_bootstrap_bca
            .as_ref()
            .expect("v12 must retain the base and BCa evidence atomically");
        assert_eq!(aggregate.base.requested_replicates, 500);
        assert_eq!(aggregate.base.attempted_refits, 500);
        assert_eq!(aggregate.base.minimum_usable_replicates, 1_000);
        assert!(matches!(
            aggregate.base.inference,
            qpls_estimation::CbsemExactCaseBootstrapInferenceV1::Unavailable { .. }
        ));
        assert_eq!(
            aggregate.bca.successful_delete_one_refits.len()
                + aggregate.bca.failed_delete_one_refits.len(),
            aggregate.base.complete_case_sample_size
        );
        assert!(matches!(
            aggregate.bca.inference,
            qpls_estimation::CbsemExactCaseBootstrapBcaInferenceV1::Unavailable {
                reason: qpls_estimation::CbsemExactCaseBootstrapBcaUnavailableReasonV1::BaseInferenceUnavailable,
                ..
            }
        ));
        let analysis_wire = serde_json::to_value(&result.estimation().analysis).unwrap();
        assert!(analysis_wire.get("exact_case_bootstrap").is_none());
        assert!(
            analysis_wire
                .get("exact_case_bootstrap_studentized")
                .is_none()
        );
        assert!(analysis_wire.get("exact_case_bootstrap_bca").is_some());

        let mut point = result.estimation().clone();
        point.analysis.exact_case_bootstrap_bca = None;
        let projection =
            cbsem_exact_case_bootstrap_base_point_digest_projection_v1(&point).unwrap();
        assert_eq!(
            aggregate.base.base_point_result_sha256,
            cbsem_exact_case_bootstrap_base_point_sha256_v1(&projection).unwrap()
        );
    }

    #[test]
    fn exact_case_bootstrap_cancellation_is_cooperative_after_point_execution() {
        for interval in [
            CbsemBootstrapInterval::PercentileType7,
            CbsemBootstrapInterval::AnalyticStudentizedType7,
            CbsemBootstrapInterval::BcaType7,
        ] {
            let (dataset, recipe, model, artifact) =
                exact_case_bootstrap_fixture_with_interval(interval);
            let cancel = AtomicBool::new(false);
            let bootstrap_progress_seen = AtomicBool::new(false);
            let result = run_compiled_cbsem_recipe_v4(
                &dataset,
                &recipe,
                &model,
                &artifact,
                || cancel.load(Ordering::SeqCst),
                |update| {
                    if update.phase == "bootstrap" {
                        bootstrap_progress_seen.store(true, Ordering::SeqCst);
                        cancel.store(true, Ordering::SeqCst);
                    }
                },
            );
            assert!(matches!(
                result,
                Err(RecipeV4CbsemExecutionError::Cancelled)
            ));
            assert!(bootstrap_progress_seen.load(Ordering::SeqCst));
        }
    }

    #[test]
    fn exact_v4_raw_mean_identity_is_bound_across_runner_provenance() {
        let (dataset, recipe, model, artifact) = mean_fixture();
        let result =
            run_compiled_cbsem_recipe_v4(&dataset, &recipe, &model, &artifact, || false, |_| {})
                .unwrap();

        assert!(result.estimation().analysis.mean_structure);
        assert!(result.estimation().analysis.score_lm.is_none());
        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6
        );
        assert_eq!(
            result.provenance().moment_input_method_version(),
            CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4
        );
        assert_eq!(
            result.provenance().estimator_method_version(),
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4
        );
        assert_eq!(
            result.estimation().schema_version,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3
        );
        assert!(
            result
                .estimation()
                .input
                .canonical_observed_means_sha256
                .is_some()
        );
        assert_eq!(result.estimation().observed_means.len(), 3);
        assert_eq!(result.estimation().implied_means.len(), 3);
        assert_eq!(result.estimation().residual_means.len(), 3);
    }

    #[test]
    fn mean_replacement_v1_binds_receipt_identity_progress_cancellation_and_tamper() {
        let (dataset, recipe, model, artifact) = mean_replacement_fixture();
        let source_arrow = write_arrow(&dataset.batch).unwrap();
        let source_fingerprint = dataset.fingerprint.clone();
        let progress = Mutex::new(Vec::new());
        let result = run_compiled_cbsem_recipe_v4(
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();

        assert_eq!(
            result.provenance().adapter_version(),
            RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7
        );
        assert!(result.estimation().analysis.score_lm.is_none());
        assert_eq!(
            result.provenance().moment_input_method_version(),
            CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1
        );
        assert_eq!(
            result.provenance().estimator_method_version(),
            CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        );
        assert_eq!(
            result.estimation().schema_version,
            CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4
        );
        let receipt = result
            .estimation()
            .input
            .missing_data_treatment
            .as_ref()
            .unwrap();
        assert_eq!(receipt.source_dataset_id, dataset.id.to_string());
        assert_eq!(receipt.source_dataset_fingerprint, dataset.fingerprint.0);
        assert_eq!(receipt.imputed_cell_count, 10);
        assert_eq!(receipt.affected_case_count, 7);
        assert!(is_lowercase_sha256(&receipt.missingness_sha256));
        assert!(is_lowercase_sha256(&receipt.completed_matrix_sha256));
        assert_eq!(
            mean_replacement_receipt_sha256(receipt).unwrap(),
            receipt.receipt_sha256
        );
        let phases = progress
            .lock()
            .unwrap()
            .iter()
            .map(|update| update.phase.clone())
            .collect::<Vec<_>>();
        for phase in ["integrity", "projection", "moments", "estimation", "result"] {
            assert!(
                phases.iter().any(|actual| actual == phase),
                "missing forwarded phase {phase}"
            );
        }
        assert_eq!(write_arrow(&dataset.batch).unwrap(), source_arrow);
        assert_eq!(dataset.fingerprint, source_fingerprint);

        let mut tampered_estimation = result.estimation().clone();
        tampered_estimation
            .input
            .missing_data_treatment
            .as_mut()
            .unwrap()
            .receipt_sha256 = "0".repeat(64);
        assert!(matches!(
            validate_mean_replacement_result(&dataset, &model, &tampered_estimation),
            Err(RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(_))
        ));

        let mut policy_tamper = recipe.clone();
        policy_tamper.settings.missing_data = MissingDataPolicy::ListwiseDeletion;
        assert!(matches!(
            run_compiled_cbsem_recipe_v4(
                &dataset,
                &policy_tamper,
                &model,
                &artifact,
                || false,
                |_| {}
            ),
            Err(RecipeV4CbsemExecutionError::Compilation(_))
        ));

        let mut identity_tamper = dataset.clone();
        identity_tamper.fingerprint.0 = format!("v2:{}", "0".repeat(64));
        assert!(matches!(
            run_compiled_cbsem_recipe_v4(
                &identity_tamper,
                &recipe,
                &model,
                &artifact,
                || false,
                |_| {}
            ),
            Err(RecipeV4CbsemExecutionError::MomentInput(
                CbsemCompiledMomentErrorV2::DatasetFingerprintMismatch
            ))
        ));

        let cancellation_checks = AtomicUsize::new(0);
        assert!(matches!(
            run_compiled_cbsem_recipe_v4(
                &dataset,
                &recipe,
                &model,
                &artifact,
                || cancellation_checks.fetch_add(1, Ordering::SeqCst) >= 7,
                |_| {}
            ),
            Err(RecipeV4CbsemExecutionError::Cancelled)
        ));
        assert_eq!(write_arrow(&dataset.batch).unwrap(), source_arrow);
        assert_eq!(dataset.fingerprint, source_fingerprint);
    }

    #[test]
    fn raw_mean_v4_frozen_product_fixture_matches_live_result_exactly() {
        let path = raw_mean_v4_fixture_path();
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "missing frozen product fixture {}; run the explicitly ignored fixture writer first: {error}",
                path.display()
            )
        });
        let frozen: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(frozen, raw_mean_v4_product_fixture());
    }

    #[test]
    #[ignore = "explicitly rewrites the validation-only frozen product fixture"]
    fn write_raw_mean_v4_product_fixture_from_live_result() {
        assert_eq!(
            std::env::var("QUICKPLS_WRITE_CBSEM_RAW_CFA_MEAN_V4_FIXTURE")
                .ok()
                .as_deref(),
            Some("1"),
            "set QUICKPLS_WRITE_CBSEM_RAW_CFA_MEAN_V4_FIXTURE=1 to rewrite the fixture"
        );
        let path = raw_mean_v4_fixture_path();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let fixture = raw_mean_v4_product_fixture();
        let mut bytes = serde_json::to_vec_pretty(&fixture).unwrap();
        bytes.push(b'\n');
        std::fs::write(&path, bytes).unwrap();
        let frozen: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(frozen, fixture);
        println!("wrote {}", path.display());
    }
}
