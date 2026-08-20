use qpls_core::{
    AnalysisRecipeV4, CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1,
    CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1,
    CanonicalCbsemBootstrapFailedReplicateReasonV1, CanonicalCbsemBootstrapFailedReplicateV1,
    CanonicalCbsemBootstrapInferenceOutcomeV1, CanonicalCbsemBootstrapParameterInferenceV1,
    CanonicalCbsemBootstrapReceiptV1, CanonicalCbsemBootstrapUnavailableReasonV1,
    CanonicalCbsemParameterRoleV1, CanonicalCbsemParameterStateV1, CanonicalGeneralSemEstimateV1,
    CanonicalGeneralSemResultTraceV1, CanonicalGeneralSemResultsV1, CbsemBootstrapTestTail,
    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1,
    CbsemExactCaseBootstrapZeroNullEligibilityV1,
    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1, CompiledAnalysisRecipeV4,
    CompiledCbsemPlanV3, CompiledRecipePlanV4, GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1,
    GeneralSemInferenceV1, RecipeV4CompilationError, RecipeV4CompilationReceipt,
    RecipeV4CompilerTarget, SemModelV4, cbsem_general_sem_ml_capability_cell_v1,
    cbsem_recursive_sem_bootstrap_capability_cell_v1, compile_analysis_recipe_v4,
    sha256_serialized, validate_compiled_analysis_recipe_v4,
};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
    CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1, CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
    CbsemCompiledMomentErrorV2, CbsemExactCaseBootstrapFailureKindV1,
    CbsemExactCaseBootstrapHypothesisTestOutcomeV1, CbsemExactParameterTableErrorV3,
    estimate_cbsem_ml_exact_case_resample_v1_with_control,
    prepare_cbsem_ml_exact_recursive_sem_case_bootstrap_source_v1_with_control,
};
use qpls_resampling::{
    CbsemExactCaseBootstrapAttemptErrorV1, CbsemExactCaseBootstrapHypothesisTestPlanV1,
    CbsemExactCaseBootstrapScheduleV1, CbsemExactCaseBootstrapSchedulerErrorV1, ResamplingError,
    run_cbsem_exact_case_bootstrap_v1, summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1,
    summarize_cbsem_exact_case_bootstrap_percentile_type7_v1,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    RunnerProgress,
    recipe_v4_cbsem_general_sem_point_execution::{
        RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1,
        RecipeV4CbsemGeneralSemPointExecutionErrorV1, run_compiled_cbsem_general_sem_point_v1,
    },
};

pub(crate) const RECIPE_V4_CBSEM_GENERAL_SEM_BOOTSTRAP_EXECUTION_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub(crate) const RECIPE_V4_CBSEM_GENERAL_SEM_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_cbsem_plan_v3_recursive_sem_case_bootstrap_execution_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeV4CbsemGeneralSemBootstrapExecutionResultV1 {
    schema_version: u32,
    adapter_version: String,
    outer_v3_compilation_receipt: RecipeV4CompilationReceipt,
    point_v3_compilation_receipt: RecipeV4CompilationReceipt,
    point_adapter_version: String,
    general_sem_results: CanonicalGeneralSemResultsV1,
}

impl RecipeV4CbsemGeneralSemBootstrapExecutionResultV1 {
    pub(crate) fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub(crate) fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub(crate) fn general_sem_results(&self) -> &CanonicalGeneralSemResultsV1 {
        &self.general_sem_results
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1 {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error(transparent)]
    Compilation(#[from] RecipeV4CompilationError),
    #[error("CB-SEM General SEM recursive bootstrap requires CbsemPlanV3 (found {0:?})")]
    CompilerTarget(RecipeV4CompilerTarget),
    #[error("CB-SEM General SEM recursive bootstrap request is outside the exact v1 predicate")]
    BootstrapOnly,
    #[error("CB-SEM General SEM point adapter failed: {0}")]
    Point(RecipeV4CbsemGeneralSemPointExecutionErrorV1),
    #[error("CB-SEM recursive bootstrap source/refit failed: {0}")]
    MomentInput(CbsemCompiledMomentErrorV2),
    #[error("CB-SEM recursive bootstrap scheduler failed: {0}")]
    Scheduler(CbsemExactCaseBootstrapSchedulerErrorV1),
    #[error("CB-SEM General SEM recursive-bootstrap authority mismatch: {0}")]
    Authority(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RecursiveBootstrapRequestV1 {
    requested_replicates: u32,
    seed: u64,
    confidence_level: f64,
    workers: usize,
}

/// Bounded adapter reached only through the Registry-authorized archive job
/// boundary. It cannot select or promote a cell. The established indexed
/// no-retry scheduler owns replicate planning and cancellation; the existing
/// exact V2 optimizer remains the sole numerical refit kernel.
pub(crate) fn run_compiled_cbsem_general_sem_recursive_bootstrap_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    artifact: &CompiledAnalysisRecipeV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<
    RecipeV4CbsemGeneralSemBootstrapExecutionResultV1,
    RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1,
> {
    cancellation_checkpoint_v1(&should_cancel)?;
    validate_compiled_analysis_recipe_v4(artifact, recipe, Some(resolved_model))?;
    let CompiledRecipePlanV4::CbsemPlanV3 { plan } = artifact.plan() else {
        return Err(
            RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::CompilerTarget(
                artifact.plan().target(),
            ),
        );
    };
    let request = recursive_bootstrap_request_v1(recipe, artifact, plan)?;

    let point_recipe = recursive_bootstrap_point_recipe_v1(recipe)?;
    let point_target = RecipeV4CompilerTarget::CbsemPlanV3;
    let point_artifact = compile_analysis_recipe_v4(
        &point_recipe,
        Some(resolved_model),
        point_target,
        cbsem_general_sem_ml_capability_cell_v1(),
    )?;
    ensure_point_derivation_authority_v1(plan, point_artifact.plan())?;
    let point_result = run_compiled_cbsem_general_sem_point_v1(
        dataset,
        &point_recipe,
        resolved_model,
        &point_artifact,
        &should_cancel,
        &progress,
    )
    .map_err(map_point_error_v1)?;
    cancellation_checkpoint_v1(&should_cancel)?;

    let kernel_target = RecipeV4CompilerTarget::CbsemPlanV2;
    let kernel_artifact = compile_analysis_recipe_v4(
        &point_recipe,
        Some(resolved_model),
        kernel_target,
        kernel_target.capability_cell(),
    )?;
    let CompiledRecipePlanV4::CbsemPlanV2 { plan: kernel_plan } = kernel_artifact.plan() else {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
            "deterministic point-kernel compilation returned the wrong plan target".into(),
        ));
    };
    if kernel_plan != plan.base_plan()
        || kernel_plan.deterministic_sha256() != plan.base_plan_sha256()
    {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
            "derived V2 point-kernel plan differs from the embedded V3 base plan".into(),
        ));
    }

    let source = prepare_cbsem_ml_exact_recursive_sem_case_bootstrap_source_v1_with_control(
        dataset,
        &kernel_artifact,
        &point_recipe,
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
    .map_err(map_moment_error_v1)?;
    let sampling_frame = source.complete_source_row_indices().to_vec();
    if sampling_frame.len() != source.complete_case_sample_size()
        || sampling_frame.windows(2).any(|pair| pair[0] >= pair[1])
        || source.plan_sha256() != kernel_artifact.receipt().plan_sha256()
        || source.model_scientific_sha256() != plan.scientific_sha256()
    {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
            "prepared recursive source differs from the exact point plan or ordered listwise frame"
                .into(),
        ));
    }

    let identity_positions = (0..sampling_frame.len()).collect::<Vec<_>>();
    let original = estimate_cbsem_ml_exact_case_resample_v1_with_control(
        &source,
        &identity_positions,
        &should_cancel,
        |_| {},
    )
    .map_err(map_moment_error_v1)?;
    let point_general_sem = point_result.general_sem_results();
    validate_original_against_point_rows_v1(&original.free_parameters, point_general_sem)?;
    let parameter_eligibility =
        recursive_bootstrap_parameter_eligibility_v1(&original.free_parameters, point_general_sem)?;
    cancellation_checkpoint_v1(&should_cancel)?;

    let base_point_result_sha256 = sha256_serialized(point_general_sem);
    let schedule = CbsemExactCaseBootstrapScheduleV1 {
        outer_recipe_analytical_identity_sha256: artifact.receipt().recipe_analytical_sha256(),
        base_point_result_sha256: &base_point_result_sha256,
        requested_replicates: request.requested_replicates,
        seed: request.seed,
        stream_token: CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
        retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
        max_attempts_per_replicate: CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
        // The historical CFA aggregate uses a different publication floor.
        // Summarize the one retained witness ledger exactly once below with
        // the recursive cell's frozen 90% gate.
        hypothesis_test: None,
    };
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
            .map_err(map_refit_attempt_error_v1)
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
    .map_err(map_scheduler_error_v1)?;
    cancellation_checkpoint_v1(&should_cancel)?;

    let minimum_usable_resamples = minimum_usable_resamples_v1(request.requested_replicates);
    let intervals = if aggregate.usable_replicates >= minimum_usable_resamples {
        if aggregate.intervals.is_empty() {
            summarize_cbsem_exact_case_bootstrap_percentile_type7_v1(
                &original,
                &aggregate.successful_refits,
                request.confidence_level,
            )
            .map_err(map_scheduler_error_v1)?
        } else {
            aggregate.intervals.clone()
        }
    } else {
        Vec::new()
    };
    let hypothesis_tests = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
        &original,
        &aggregate.successful_refits,
        minimum_usable_resamples,
        CbsemExactCaseBootstrapHypothesisTestPlanV1 {
            selected_test_tail: CbsemBootstrapTestTail::TwoSided,
            parameter_eligibility: &parameter_eligibility,
        },
    )
    .map_err(map_scheduler_error_v1)?;
    let inference = canonical_recursive_bootstrap_inference_v1(
        plan,
        point_general_sem,
        &original.free_parameters,
        &parameter_eligibility,
        &intervals,
        &hypothesis_tests.parameters,
        aggregate.usable_replicates,
        minimum_usable_resamples,
        &should_cancel,
    )?;

    let parameter_ids = inference
        .iter()
        .map(|row| row.parameter_id.as_str())
        .collect::<Vec<_>>();
    let usable_replicate_indices = aggregate
        .successful_refits
        .iter()
        .map(|row| row.replicate_index)
        .collect::<Vec<_>>();
    let failed_replicates = aggregate
        .failed_refits
        .iter()
        .map(|failure| CanonicalCbsemBootstrapFailedReplicateV1 {
            replicate_index: failure.replicate_index,
            reason_code: canonical_failure_reason_v1(failure.kind),
            message: failure.message.clone(),
        })
        .collect::<Vec<_>>();
    let receipt = CanonicalCbsemBootstrapReceiptV1 {
        capability_cell: cbsem_recursive_sem_bootstrap_capability_cell_v1(),
        method_version: CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1.into(),
        resampling_operation_version: CBSEM_RECURSIVE_SEM_BOOTSTRAP_OPERATION_VERSION_V1.into(),
        quantile_method_version: GENERAL_SEM_TYPE7_QUANTILE_METHOD_VERSION_V1.into(),
        compiled_plan_sha256: artifact.receipt().plan_sha256().into(),
        base_plan_sha256: plan.base_plan_sha256().into(),
        parameter_inventory_sha256: sha256_serialized(&parameter_ids),
        model_scientific_sha256: plan.scientific_sha256().into(),
        general_sem_config_sha256: plan.general_sem_config_sha256().into(),
        recipe_analytical_sha256: artifact.receipt().recipe_analytical_sha256().into(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        complete_case_frame_sha256: source.complete_case_universe_sha256().into(),
        usable_replicate_indices_sha256: sha256_serialized(&usable_replicate_indices),
        confidence_level: request.confidence_level,
        resamples_requested: request.requested_replicates,
        resamples_usable: aggregate.usable_replicates,
        minimum_usable_resamples,
        seed: request.seed.to_string(),
        workers: u32::try_from(request.workers).map_err(|_| {
            RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
                "worker count exceeds the canonical u32 contract".into(),
            )
        })?,
        complete_model_reestimated_per_replicate: true,
        failed_replicates,
    };
    if receipt.resamples_usable as usize + receipt.failed_replicates.len()
        != receipt.resamples_requested as usize
    {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
            "successful and failed recursive refits do not exhaust the indexed schedule".into(),
        ));
    }

    let mut general_sem_results = point_general_sem.clone();
    general_sem_results.cbsem_bootstrap_receipt = Some(receipt);
    general_sem_results.cbsem_bootstrap_inference = inference;
    cancellation_checkpoint_v1(&should_cancel)?;
    Ok(RecipeV4CbsemGeneralSemBootstrapExecutionResultV1 {
        schema_version: RECIPE_V4_CBSEM_GENERAL_SEM_BOOTSTRAP_EXECUTION_RESULT_SCHEMA_VERSION_V1,
        adapter_version: RECIPE_V4_CBSEM_GENERAL_SEM_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1.into(),
        outer_v3_compilation_receipt: artifact.receipt().clone(),
        point_v3_compilation_receipt: point_artifact.receipt().clone(),
        point_adapter_version: RECIPE_V4_CBSEM_GENERAL_SEM_POINT_EXECUTION_ADAPTER_VERSION_V1
            .into(),
        general_sem_results,
    })
}

fn recursive_bootstrap_request_v1(
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledAnalysisRecipeV4,
    plan: &CompiledCbsemPlanV3,
) -> Result<RecursiveBootstrapRequestV1, RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1> {
    let Some(config) = recipe.general_sem_config.as_ref() else {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::BootstrapOnly);
    };
    let GeneralSemInferenceV1::CaseBootstrap {
        resamples,
        seed,
        confidence_level,
        interval: qpls_core::GeneralSemBootstrapIntervalV1::Percentile,
        tail: qpls_core::GeneralSemInferenceTailV1::TwoSided,
    } = config.inference
    else {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::BootstrapOnly);
    };
    let exact_cell = cbsem_recursive_sem_bootstrap_capability_cell_v1();
    if artifact.receipt().capability_cell() != &exact_cell
        || plan.capability_cells()
            != [
                cbsem_general_sem_ml_capability_cell_v1(),
                exact_cell.clone(),
            ]
        || plan.base_plan().regressions().is_empty()
        || !(500..=10_000).contains(&resamples)
        || confidence_level.to_bits() != 0.95_f64.to_bits()
        || !(1..=64).contains(&recipe.settings.workers)
    {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::BootstrapOnly);
    }
    Ok(RecursiveBootstrapRequestV1 {
        requested_replicates: resamples,
        seed,
        confidence_level,
        workers: recipe.settings.workers,
    })
}

fn recursive_bootstrap_point_recipe_v1(
    recipe: &AnalysisRecipeV4,
) -> Result<AnalysisRecipeV4, RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1> {
    let mut point = recipe.clone();
    point.settings.bootstrap_samples = 0;
    let Some(qpls_core::MethodConfig::Cbsem {
        bootstrap_samples,
        bootstrap_v2,
        ..
    }) = point.method_config.as_mut()
    else {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::BootstrapOnly);
    };
    *bootstrap_samples = 0;
    *bootstrap_v2 = None;
    let Some(config) = point.general_sem_config.as_mut() else {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::BootstrapOnly);
    };
    config.inference = GeneralSemInferenceV1::None;
    Ok(point)
}

fn ensure_point_derivation_authority_v1(
    outer: &CompiledCbsemPlanV3,
    point_artifact_plan: &CompiledRecipePlanV4,
) -> Result<(), RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1> {
    let CompiledRecipePlanV4::CbsemPlanV3 { plan: point } = point_artifact_plan else {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
            "derived point compilation returned the wrong V3 plan target".into(),
        ));
    };
    if outer.base_plan() != point.base_plan()
        || outer.base_plan_sha256() != point.base_plan_sha256()
        || outer.scientific_sha256() != point.scientific_sha256()
        || outer.data_binding_sha256() != point.data_binding_sha256()
        || outer.topology() != point.topology()
        || outer.parameter_table_authority() != point.parameter_table_authority()
        || outer.identification_evidence() != point.identification_evidence()
        || point.capability_cells() != [cbsem_general_sem_ml_capability_cell_v1()]
    {
        return Err(
            RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
                "derived point authority changed the model, parameter table, topology, identification, or base plan"
                    .into(),
            ),
        );
    }
    Ok(())
}

fn validate_original_against_point_rows_v1(
    original: &[qpls_estimation::CbsemExactCaseBootstrapParameterEstimateV1],
    point: &CanonicalGeneralSemResultsV1,
) -> Result<(), RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1> {
    let mut expected = point
        .cbsem_parameters
        .iter()
        .filter(|row| matches!(row.state, CanonicalCbsemParameterStateV1::Free { .. }))
        .map(|row| (row.parameter_id.as_str(), row.estimate.to_bits()))
        .collect::<Vec<_>>();
    expected.sort_by(|left, right| left.0.cmp(right.0));
    let actual = original
        .iter()
        .map(|row| (row.parameter_id.as_str(), row.estimate.to_bits()))
        .collect::<Vec<_>>();
    if expected.is_empty() || expected != actual {
        return Err(
            RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
                "identity-position recursive refit differs from the point free-parameter IDs or estimates"
                    .into(),
            ),
        );
    }
    Ok(())
}

fn recursive_bootstrap_parameter_eligibility_v1(
    original: &[qpls_estimation::CbsemExactCaseBootstrapParameterEstimateV1],
    point: &CanonicalGeneralSemResultsV1,
) -> Result<
    Vec<CbsemExactCaseBootstrapZeroNullEligibilityV1>,
    RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1,
> {
    let rows = point
        .cbsem_parameters
        .iter()
        .map(|row| (row.parameter_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    original
        .iter()
        .map(|parameter| {
            let row = rows.get(parameter.parameter_id.as_str()).ok_or_else(|| {
                RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(format!(
                    "point row {} is absent while compiling recursive-bootstrap eligibility",
                    parameter.parameter_id
                ))
            })?;
            let CanonicalCbsemParameterStateV1::Free { lower, upper, .. } = &row.state else {
                return Err(
                    RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(format!(
                        "bootstrap parameter {} is fixed in the point authority",
                        parameter.parameter_id
                    )),
                );
            };
            let zero_inside_open_bounds =
                lower.is_none_or(|value| value < 0.0) && upper.is_none_or(|value| value > 0.0);
            let status = match row.role {
                CanonicalCbsemParameterRoleV1::Variance => {
                    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                        reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary,
                    }
                }
                CanonicalCbsemParameterRoleV1::Loading
                | CanonicalCbsemParameterRoleV1::Regression
                | CanonicalCbsemParameterRoleV1::Covariance
                    if zero_inside_open_bounds =>
                {
                    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available
                }
                CanonicalCbsemParameterRoleV1::Loading
                | CanonicalCbsemParameterRoleV1::Regression
                | CanonicalCbsemParameterRoleV1::Covariance => {
                    CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                        reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::ZeroNullOutsideOpenDomain,
                    }
                }
            };
            Ok(CbsemExactCaseBootstrapZeroNullEligibilityV1 {
                parameter_id: parameter.parameter_id.clone(),
                status,
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn canonical_recursive_bootstrap_inference_v1(
    plan: &CompiledCbsemPlanV3,
    point: &CanonicalGeneralSemResultsV1,
    original: &[qpls_estimation::CbsemExactCaseBootstrapParameterEstimateV1],
    eligibility: &[CbsemExactCaseBootstrapZeroNullEligibilityV1],
    intervals: &[qpls_estimation::CbsemExactCaseBootstrapParameterIntervalV1],
    tests: &[qpls_estimation::CbsemExactCaseBootstrapHypothesisTestParameterV1],
    usable_replicates: u32,
    minimum_usable_replicates: u32,
    should_cancel: &(impl Fn() -> bool + Sync),
) -> Result<
    Vec<CanonicalCbsemBootstrapParameterInferenceV1>,
    RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1,
> {
    let point_rows = point
        .cbsem_parameters
        .iter()
        .map(|row| (row.parameter_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let interval_rows = intervals
        .iter()
        .map(|row| (row.parameter_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let test_rows = tests
        .iter()
        .map(|row| (row.parameter_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    if eligibility.len() != original.len() || tests.len() != original.len() {
        return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
            "recursive-bootstrap interval/test inventories have incompatible dimensions".into(),
        ));
    }
    let globally_available = usable_replicates >= minimum_usable_replicates;
    let trace = CanonicalGeneralSemResultTraceV1 {
        model_id: plan.model_id().into(),
        capability_cell: cbsem_recursive_sem_bootstrap_capability_cell_v1(),
    };
    let mut rows = Vec::with_capacity(original.len());
    for (index, parameter) in original.iter().enumerate() {
        cancellation_checkpoint_v1(should_cancel)?;
        let point_row = point_rows
            .get(parameter.parameter_id.as_str())
            .ok_or_else(|| {
                RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(format!(
                    "point parameter {} disappeared during inference projection",
                    parameter.parameter_id
                ))
            })?;
        if point_row.estimate.to_bits() != parameter.estimate.to_bits()
            || eligibility[index].parameter_id != parameter.parameter_id
        {
            return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
                format!(
                    "point, refit, and eligibility identities disagree for {}",
                    parameter.parameter_id
                ),
            ));
        }
        let outcome = if !globally_available {
            CanonicalCbsemBootstrapInferenceOutcomeV1::Unavailable {
                reason: CanonicalCbsemBootstrapUnavailableReasonV1::InsufficientUsableReplicates,
            }
        } else if !matches!(
            eligibility[index].status,
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available
        ) {
            CanonicalCbsemBootstrapInferenceOutcomeV1::Unavailable {
                reason: CanonicalCbsemBootstrapUnavailableReasonV1::ParameterNotEligible,
            }
        } else {
            let interval = interval_rows
                .get(parameter.parameter_id.as_str())
                .ok_or_else(|| {
                    RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(format!(
                        "available parameter {} has no Type-7 interval",
                        parameter.parameter_id
                    ))
                })?;
            let test = test_rows
                .get(parameter.parameter_id.as_str())
                .ok_or_else(|| {
                    RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(format!(
                        "available parameter {} has no zero-null test",
                        parameter.parameter_id
                    ))
                })?;
            let CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
                point_estimate,
                two_sided_exceedances,
                p_value_two_sided,
                ..
            } = test.outcome
            else {
                return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
                    format!(
                        "eligible parameter {} has unavailable zero-null inference",
                        parameter.parameter_id
                    ),
                ));
            };
            if point_estimate.to_bits() != parameter.estimate.to_bits()
                || interval.original.to_bits() != parameter.estimate.to_bits()
                || interval.usable_replicates != usable_replicates
            {
                return Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Authority(
                    format!(
                        "interval/test ledger does not bind point parameter {}",
                        parameter.parameter_id
                    ),
                ));
            }
            CanonicalCbsemBootstrapInferenceOutcomeV1::Available {
                value: CanonicalGeneralSemEstimateV1 {
                    estimate: parameter.estimate,
                    bootstrap_mean: Some(interval.bootstrap_mean),
                    bootstrap_bias: Some(interval.bias),
                    standard_error: Some(interval.standard_error),
                    lower: Some(interval.percentile_lower),
                    upper: Some(interval.percentile_upper),
                    p_value: Some(p_value_two_sided),
                    bootstrap_usable_replicates: Some(usable_replicates),
                    bootstrap_two_sided_exceedances: Some(two_sided_exceedances),
                },
            }
        };
        rows.push(CanonicalCbsemBootstrapParameterInferenceV1 {
            parameter_id: parameter.parameter_id.clone(),
            trace: trace.clone(),
            point_estimate: parameter.estimate,
            outcome,
        });
    }
    Ok(rows)
}

fn minimum_usable_resamples_v1(requested_replicates: u32) -> u32 {
    ((u64::from(requested_replicates) * 9 + 9) / 10) as u32
}

fn canonical_failure_reason_v1(
    kind: CbsemExactCaseBootstrapFailureKindV1,
) -> CanonicalCbsemBootstrapFailedReplicateReasonV1 {
    match kind {
        CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite => {
            CanonicalCbsemBootstrapFailedReplicateReasonV1::NonpositiveDefiniteSampleCovariance
        }
        CbsemExactCaseBootstrapFailureKindV1::NonConvergence => {
            CanonicalCbsemBootstrapFailedReplicateReasonV1::Nonconvergence
        }
        CbsemExactCaseBootstrapFailureKindV1::InadmissibleSolution
        | CbsemExactCaseBootstrapFailureKindV1::NumericalFailure => {
            CanonicalCbsemBootstrapFailedReplicateReasonV1::NumericalFailure
        }
    }
}

fn map_refit_attempt_error_v1(
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

fn map_point_error_v1(
    error: RecipeV4CbsemGeneralSemPointExecutionErrorV1,
) -> RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1 {
    if matches!(
        &error,
        RecipeV4CbsemGeneralSemPointExecutionErrorV1::Cancelled
    ) {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Cancelled
    } else {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Point(error)
    }
}

fn map_moment_error_v1(
    error: CbsemCompiledMomentErrorV2,
) -> RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1 {
    if matches!(&error, CbsemCompiledMomentErrorV2::Cancelled) {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Cancelled
    } else {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::MomentInput(error)
    }
}

fn map_scheduler_error_v1(
    error: CbsemExactCaseBootstrapSchedulerErrorV1,
) -> RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1 {
    if matches!(
        &error,
        CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(ResamplingError::Cancelled)
    ) {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Cancelled
    } else {
        RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Scheduler(error)
    }
}

fn cancellation_checkpoint_v1(
    should_cancel: &(impl Fn() -> bool + Sync),
) -> Result<(), RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1> {
    if should_cancel() {
        Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        CanonicalCbsemEndpointV1, CanonicalCbsemParameterResultV1, CanonicalCbsemParameterTargetV1,
    };

    fn point_row(
        parameter_id: &str,
        role: CanonicalCbsemParameterRoleV1,
        lower: Option<f64>,
        upper: Option<f64>,
    ) -> CanonicalCbsemParameterResultV1 {
        let target = match role {
            CanonicalCbsemParameterRoleV1::Loading => CanonicalCbsemParameterTargetV1::Loading {
                factor_id: "factor:x".into(),
                indicator_id: "observed:x1".into(),
            },
            CanonicalCbsemParameterRoleV1::Regression => {
                CanonicalCbsemParameterTargetV1::Regression {
                    source_id: "factor:x".into(),
                    target_id: "factor:y".into(),
                }
            }
            CanonicalCbsemParameterRoleV1::Covariance => {
                CanonicalCbsemParameterTargetV1::Covariance {
                    left: CanonicalCbsemEndpointV1::Variable {
                        variable_id: "factor:x".into(),
                    },
                    right: CanonicalCbsemEndpointV1::Variable {
                        variable_id: "factor:y".into(),
                    },
                }
            }
            CanonicalCbsemParameterRoleV1::Variance => CanonicalCbsemParameterTargetV1::Variance {
                endpoint: CanonicalCbsemEndpointV1::Variable {
                    variable_id: "factor:x".into(),
                },
            },
        };
        CanonicalCbsemParameterResultV1 {
            parameter_id: parameter_id.into(),
            trace: CanonicalGeneralSemResultTraceV1 {
                model_id: "model:rank3".into(),
                capability_cell: cbsem_general_sem_ml_capability_cell_v1(),
            },
            role,
            target,
            relation_id: None,
            state: CanonicalCbsemParameterStateV1::Free {
                equality_label: None,
                lower,
                upper,
            },
            estimate: 0.25,
            standard_error: Some(0.1),
            z_value: Some(2.5),
            p_value: Some(0.01),
            standardized_estimate: Some(0.2),
        }
    }

    fn free_parameter(
        parameter_id: &str,
    ) -> qpls_estimation::CbsemExactCaseBootstrapParameterEstimateV1 {
        qpls_estimation::CbsemExactCaseBootstrapParameterEstimateV1 {
            parameter_id: parameter_id.into(),
            estimate: 0.25,
        }
    }

    #[test]
    fn recursive_gate_is_exact_without_the_legacy_cfa_floor() {
        assert_eq!(minimum_usable_resamples_v1(500), 450);
        assert_eq!(minimum_usable_resamples_v1(501), 451);
        assert_eq!(minimum_usable_resamples_v1(10_000), 9_000);
        assert!(matches!(
            cancellation_checkpoint_v1(&|| true),
            Err(RecipeV4CbsemGeneralSemBootstrapExecutionErrorV1::Cancelled)
        ));
    }

    #[test]
    fn zero_null_eligibility_is_role_and_open_bound_aware() {
        let parameters = vec![
            free_parameter("a_loading"),
            free_parameter("b_regression"),
            free_parameter("c_variance"),
        ];
        let mut point: CanonicalGeneralSemResultsV1 = serde_json::from_value(serde_json::json!({
            "schema_version": qpls_core::CANONICAL_GENERAL_SEM_RESULTS_V1_SCHEMA_VERSION
        }))
        .unwrap();
        point.cbsem_parameters = vec![
            point_row(
                "a_loading",
                CanonicalCbsemParameterRoleV1::Loading,
                None,
                None,
            ),
            point_row(
                "b_regression",
                CanonicalCbsemParameterRoleV1::Regression,
                Some(0.0),
                None,
            ),
            point_row(
                "c_variance",
                CanonicalCbsemParameterRoleV1::Variance,
                Some(0.0),
                None,
            ),
        ];

        let eligibility =
            recursive_bootstrap_parameter_eligibility_v1(&parameters, &point).unwrap();
        assert_eq!(
            eligibility[0].status,
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available
        );
        assert_eq!(
            eligibility[1].status,
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                reason:
                    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::ZeroNullOutsideOpenDomain,
            }
        );
        assert_eq!(
            eligibility[2].status,
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                reason:
                    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary,
            }
        );
    }
}
