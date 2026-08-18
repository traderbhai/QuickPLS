use rayon::prelude::*;
use sha2::{Digest, Sha256};
use statrs::distribution::{ContinuousCDF, Normal};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

use qpls_core::{
    CbsemBootstrapTestTail, CbsemExactCaseBootstrapZeroNullEligibilityStatusV1,
    CbsemExactCaseBootstrapZeroNullEligibilityV1,
    CbsemExactCaseBootstrapZeroNullUnavailableReasonV1, SemCovarianceDenominatorV4,
};
use qpls_estimation::{
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
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3, CbsemExactCaseBootstrapBcaInferenceV1,
    CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1,
    CbsemExactCaseBootstrapBcaParameterIntervalV1, CbsemExactCaseBootstrapBcaSidecarV1,
    CbsemExactCaseBootstrapBcaUnavailableReasonV1, CbsemExactCaseBootstrapDeleteOneFailureV1,
    CbsemExactCaseBootstrapDeleteOneRefitV1, CbsemExactCaseBootstrapDeleteOneWitnessV1,
    CbsemExactCaseBootstrapFailureKindV1, CbsemExactCaseBootstrapFailureV1,
    CbsemExactCaseBootstrapHypothesisTestInferenceV1,
    CbsemExactCaseBootstrapHypothesisTestOutcomeV1,
    CbsemExactCaseBootstrapHypothesisTestParameterV1,
    CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1,
    CbsemExactCaseBootstrapHypothesisTestsV1, CbsemExactCaseBootstrapInferenceV1,
    CbsemExactCaseBootstrapParameterIntervalV1, CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapRefitStandardErrorsV1, CbsemExactCaseBootstrapRefitV1,
    CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1, CbsemExactCaseBootstrapResultV1,
    CbsemExactCaseBootstrapStudentizedInferenceV1,
    CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1,
    CbsemExactCaseBootstrapStudentizedParameterIntervalV1,
    CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1,
    CbsemExactCaseBootstrapStudentizedRefitStandardErrorsV1,
    CbsemExactCaseBootstrapStudentizedSidecarV1,
    CbsemExactCaseBootstrapStudentizedUnavailableReasonV1,
    CbsemExactCaseBootstrapWithStudentizedResultV1, CbsemExactCaseBootstrapWitnessV1,
    cbsem_exact_case_bootstrap_complete_case_universe_digest_v1,
    cbsem_exact_case_bootstrap_index_digest_v1,
    cbsem_exact_case_bootstrap_sampling_positions_digest_v1,
};

use crate::{ResamplingError, ResamplingPhase, ResamplingProgress, bootstrap_indices};

pub const CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1: &str =
    "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1";
pub const CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V1: f64 = 0.90;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_REQUESTED_REPLICATES_V1: u32 = 500;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_REPLICATES_V1: u32 = 1_000;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_MAXIMUM_REPLICATES_V1: u32 = 10_000;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1: f64 = 0.95;
pub const CBSEM_EXACT_CASE_BOOTSTRAP_BCA_MIN_ABS_ADJUSTMENT_DENOMINATOR_V1: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CbsemExactCaseBootstrapScheduleV1<'a> {
    pub outer_recipe_analytical_identity_sha256: &'a str,
    pub base_point_result_sha256: &'a str,
    pub requested_replicates: u32,
    pub seed: u64,
    pub stream_token: &'a str,
    pub retry_policy: &'a str,
    pub max_attempts_per_replicate: u8,
    pub hypothesis_test: Option<CbsemExactCaseBootstrapHypothesisTestPlanV1<'a>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CbsemExactCaseBootstrapHypothesisTestPlanV1<'a> {
    pub selected_test_tail: CbsemBootstrapTestTail,
    pub parameter_eligibility: &'a [CbsemExactCaseBootstrapZeroNullEligibilityV1],
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CbsemExactCaseBootstrapAttemptErrorV1 {
    #[error("exact CB-SEM case-bootstrap refit was cancelled")]
    Cancelled,
    #[error("exact CB-SEM case-bootstrap refit failed: {message}")]
    Failed {
        kind: CbsemExactCaseBootstrapFailureKindV1,
        message: String,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CbsemExactCaseBootstrapSchedulerErrorV1 {
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
    #[error("invalid exact CB-SEM case-bootstrap point refit: {0}")]
    InvalidOriginal(String),
    #[error(
        "exact CB-SEM case-bootstrap refit {replicate_index} violates the scheduler contract: {reason}"
    )]
    RefitIntegrity {
        replicate_index: u32,
        reason: String,
    },
    #[error("exact CB-SEM case-bootstrap summary is invalid: {0}")]
    InvalidSummary(String),
}

#[derive(Debug)]
enum ScheduledSlot {
    Success(CbsemExactCaseBootstrapWitnessV1),
    Failure(CbsemExactCaseBootstrapFailureV1),
    Cancelled,
    Invalid {
        replicate_index: u32,
        reason: String,
    },
}

#[derive(Debug)]
enum DeleteOneSlot {
    Success(CbsemExactCaseBootstrapDeleteOneWitnessV1),
    Failure(CbsemExactCaseBootstrapDeleteOneFailureV1),
    Cancelled,
    Invalid {
        omitted_position: usize,
        reason: String,
    },
}

/// Schedule the exact compiled-parameter-table CFA case bootstrap around a
/// caller-supplied refit seam. `sampling_frame` is the canonical, strictly
/// increasing list of listwise-complete source storage rows. The callback
/// receives ordered positions in that frame; source-row mapping is retained
/// here solely for digest binding. Every replicate receives exactly one
/// deterministic with-replacement draw.
pub fn run_cbsem_exact_case_bootstrap_v1(
    sampling_frame: &[usize],
    original: &CbsemExactCaseBootstrapRefitV1,
    schedule: CbsemExactCaseBootstrapScheduleV1<'_>,
    workers: usize,
    refit: impl Fn(
        u32,
        &[usize],
    ) -> Result<CbsemExactCaseBootstrapRefitV1, CbsemExactCaseBootstrapAttemptErrorV1>
    + Sync,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<CbsemExactCaseBootstrapResultV1, CbsemExactCaseBootstrapSchedulerErrorV1> {
    validate_plan(sampling_frame, original, schedule, workers)?;
    if is_cancelled() {
        return Err(ResamplingError::Cancelled.into());
    }

    let primary_operation = format!("{}:primary", schedule.stream_token);

    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.clone())
        .collect::<Vec<_>>();
    let original_estimates = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.estimate)
        .collect::<Vec<_>>();
    let cancellation = &is_cancelled;
    let callback = &refit;
    let progress_callback = &report_progress;
    let completed = AtomicU64::new(0);
    let aborted = std::sync::atomic::AtomicBool::new(false);
    let progress_guard = Mutex::new(());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| ResamplingError::WorkerPool(error.to_string()))?;

    let mut slots = pool.install(|| {
        (0..schedule.requested_replicates)
            .into_par_iter()
            .map(|replicate_index| {
                if cancellation() || aborted.load(Ordering::Relaxed) {
                    return ScheduledSlot::Cancelled;
                }
                let sampled_positions = bootstrap_indices(
                    sampling_frame.len(),
                    schedule.seed,
                    &primary_operation,
                    replicate_index,
                );
                let schedule_positions_digest =
                    cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                        schedule.stream_token,
                        schedule.seed,
                        replicate_index,
                        sampling_frame.len(),
                        &sampled_positions,
                    );
                let refit_positions_digest =
                    cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                        sampling_frame.len(),
                        &sampled_positions,
                    );
                let source_rows = sampled_positions
                    .iter()
                    .map(|position| sampling_frame[*position])
                    .collect::<Vec<_>>();
                let source_rows_digest = cbsem_exact_case_bootstrap_index_digest_v1(
                    &original.source_dataset_fingerprint,
                    original.source_row_count,
                    &source_rows,
                );
                let slot = match callback(replicate_index, &sampled_positions) {
                    Ok(result) => match witness_from_refit(
                        replicate_index,
                        &schedule_positions_digest,
                        &refit_positions_digest,
                        &source_rows_digest,
                        &parameter_ids,
                        original,
                        result,
                    ) {
                        Ok(witness) => ScheduledSlot::Success(witness),
                        Err(reason) => ScheduledSlot::Invalid {
                            replicate_index,
                            reason,
                        },
                    },
                    Err(CbsemExactCaseBootstrapAttemptErrorV1::Cancelled) => {
                        aborted.store(true, Ordering::Relaxed);
                        ScheduledSlot::Cancelled
                    }
                    Err(CbsemExactCaseBootstrapAttemptErrorV1::Failed { kind, message }) => {
                        if message.trim().is_empty() {
                            ScheduledSlot::Invalid {
                                replicate_index,
                                reason: "failure message is empty".into(),
                            }
                        } else {
                            ScheduledSlot::Failure(CbsemExactCaseBootstrapFailureV1 {
                                replicate_index,
                                sampling_positions_sha256: schedule_positions_digest,
                                sample_indices_sha256: source_rows_digest,
                                kind,
                                message,
                            })
                        }
                    }
                };
                let _guard = progress_guard
                    .lock()
                    .expect("exact CB-SEM bootstrap progress mutex poisoned");
                let completed_replicates = completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                progress_callback(ResamplingProgress {
                    phase: ResamplingPhase::Bootstrap,
                    completed_replicates,
                    total_replicates: schedule.requested_replicates,
                });
                slot
            })
            .collect::<Vec<_>>()
    });
    if cancellation()
        || aborted.load(Ordering::Relaxed)
        || slots
            .iter()
            .any(|slot| matches!(slot, ScheduledSlot::Cancelled))
    {
        return Err(ResamplingError::Cancelled.into());
    }
    slots.sort_by_key(slot_replicate_index);
    if let Some(ScheduledSlot::Invalid {
        replicate_index,
        reason,
    }) = slots
        .iter()
        .find(|slot| matches!(slot, ScheduledSlot::Invalid { .. }))
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::RefitIntegrity {
            replicate_index: *replicate_index,
            reason: reason.clone(),
        });
    }

    let successful_refits = slots
        .iter()
        .filter_map(|slot| match slot {
            ScheduledSlot::Success(witness) => Some(witness.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let failed_refits = slots
        .iter()
        .filter_map(|slot| match slot {
            ScheduledSlot::Failure(failure) => Some(failure.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if successful_refits.len() + failed_refits.len() != schedule.requested_replicates as usize {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "preplanned refit accounting is incomplete".into(),
        ));
    }

    let minimum_usable = required_usable_refits(schedule.requested_replicates);
    let (inference, intervals) = if successful_refits.len() >= minimum_usable as usize {
        (
            CbsemExactCaseBootstrapInferenceV1::Available,
            summarize_intervals(
                &parameter_ids,
                &original_estimates,
                &successful_refits,
                CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
            )?,
        )
    } else {
        (
            CbsemExactCaseBootstrapInferenceV1::Unavailable {
                reason_code: "insufficient_usable_refits".into(),
                message: format!(
                    "Exact CB-SEM case-bootstrap inference is unavailable because {} usable refits are below the required {minimum_usable}; no intervals were emitted.",
                    successful_refits.len()
                ),
            },
            Vec::new(),
        )
    };
    let hypothesis_tests = schedule
        .hypothesis_test
        .map(|plan| {
            summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
                original,
                &successful_refits,
                minimum_usable,
                plan,
            )
        })
        .transpose()?;

    Ok(CbsemExactCaseBootstrapResultV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
        estimator_method_version: original.estimator_method_version.clone(),
        source_dataset_id: original.source_dataset_id.clone(),
        source_dataset_fingerprint: original.source_dataset_fingerprint.clone(),
        outer_recipe_analytical_identity_sha256: schedule
            .outer_recipe_analytical_identity_sha256
            .into(),
        base_point_result_sha256: schedule.base_point_result_sha256.into(),
        compiler_analytical_identity_sha256: original.compiler_analytical_identity_sha256.clone(),
        plan_sha256: original.plan_sha256.clone(),
        model_scientific_sha256: original.model_scientific_sha256.clone(),
        complete_case_sample_size: original.complete_case_sample_size,
        complete_case_universe_digest_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1.into(),
        complete_case_universe_sha256: original.complete_case_universe_sha256.clone(),
        covariance_denominator: original.covariance_denominator,
        sample_indices_digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.into(),
        sampling_positions_digest_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1.into(),
        interval_method: CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1.into(),
        confidence_level: CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
        requested_replicates: schedule.requested_replicates,
        attempted_refits: schedule.requested_replicates,
        usable_replicates: successful_refits.len() as u32,
        failed_replicates: failed_refits.len() as u32,
        minimum_usable_fraction: CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V1,
        minimum_usable_replicates: minimum_usable,
        seed: schedule.seed,
        stream_token: schedule.stream_token.into(),
        retry_policy: schedule.retry_policy.into(),
        max_attempts_per_replicate: schedule.max_attempts_per_replicate,
        parameter_ids,
        inference,
        intervals,
        hypothesis_tests,
        successful_refits,
        failed_refits,
    })
}

/// Opt-in analytically studentized exact-CFA bootstrap. This deliberately
/// delegates all draws, point-success accounting, cancellation, progress, and
/// v10 aggregation to [`run_cbsem_exact_case_bootstrap_v1`]. Each draw is
/// therefore executed exactly once. Product callers must add a separate
/// workload/preflight policy before exposing this more expensive path.
pub fn run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
    sampling_frame: &[usize],
    original: &CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1,
    schedule: CbsemExactCaseBootstrapScheduleV1<'_>,
    workers: usize,
    refit: impl Fn(
        u32,
        &[usize],
    ) -> Result<
        CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1,
        CbsemExactCaseBootstrapAttemptErrorV1,
    > + Sync,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<CbsemExactCaseBootstrapWithStudentizedResultV1, CbsemExactCaseBootstrapSchedulerErrorV1>
{
    let parameter_ids = original
        .refit
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .collect::<Vec<_>>();
    validate_analytic_standard_errors(&parameter_ids, &original.standard_errors)
        .map_err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal)?;
    let receipts = Mutex::new(BTreeMap::<u32, CbsemExactCaseBootstrapRefitStandardErrorsV1>::new());
    let base = run_cbsem_exact_case_bootstrap_v1(
        sampling_frame,
        &original.refit,
        schedule,
        workers,
        |replicate_index, sampled_positions| {
            let result = refit(replicate_index, sampled_positions)?;
            let previous = receipts
                .lock()
                .expect("exact CB-SEM studentized receipt mutex poisoned")
                .insert(replicate_index, result.standard_errors);
            debug_assert!(previous.is_none(), "replicate was scheduled more than once");
            Ok(result.refit)
        },
        &is_cancelled,
        &report_progress,
    )?;
    let mut receipts = receipts.into_inner().map_err(|_| {
        CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "studentized standard-error receipt mutex was poisoned".into(),
        )
    })?;
    let ordered_receipts = base
        .successful_refits
        .iter()
        .map(|witness| {
            receipts.remove(&witness.replicate_index).ok_or_else(|| {
                CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(format!(
                    "successful refit {} has no analytical standard-error receipt",
                    witness.replicate_index
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !receipts.is_empty() {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "analytical standard-error receipts do not match the point-success ledger".into(),
        ));
    }
    let studentized = recompute_cbsem_exact_case_bootstrap_studentized_sidecar_v1(
        &original.refit,
        &original.standard_errors,
        &base,
        ordered_receipts,
    )?;
    Ok(CbsemExactCaseBootstrapWithStudentizedResultV1 { base, studentized })
}

/// Opt-in exact-CFA BCa sidecar. The supplied v10 point-bootstrap aggregate is
/// never recomputed or modified. Exactly one N-1 ML fit is requested for each
/// complete-case omission; failures remain ordered evidence and block all BCa
/// intervals. Outer-recipe and base-point digests remain the supplied v10
/// ledger authority because this reduced refit seam cannot reconstruct the
/// full canonical point projection. Product exposure requires a separate
/// workload/cap policy.
pub fn run_cbsem_exact_case_bootstrap_bca_v1(
    sampling_frame: &[usize],
    original: &CbsemExactCaseBootstrapRefitV1,
    base: &CbsemExactCaseBootstrapResultV1,
    workers: usize,
    delete_one: impl Fn(
        usize,
    ) -> Result<
        CbsemExactCaseBootstrapDeleteOneRefitV1,
        CbsemExactCaseBootstrapAttemptErrorV1,
    > + Sync,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<CbsemExactCaseBootstrapBcaSidecarV1, CbsemExactCaseBootstrapSchedulerErrorV1> {
    validate_bca_base(sampling_frame, original, base, workers)?;
    if is_cancelled() {
        return Err(ResamplingError::Cancelled.into());
    }
    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.clone())
        .collect::<Vec<_>>();
    let completed = AtomicU64::new(0);
    let aborted = std::sync::atomic::AtomicBool::new(false);
    let progress_guard = Mutex::new(());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| ResamplingError::WorkerPool(error.to_string()))?;
    let mut slots = pool.install(|| {
        (0..sampling_frame.len())
            .into_par_iter()
            .map(|omitted_position| {
                if is_cancelled() || aborted.load(Ordering::Relaxed) {
                    return DeleteOneSlot::Cancelled;
                }
                let retained_positions = (0..sampling_frame.len())
                    .filter(|position| *position != omitted_position)
                    .collect::<Vec<_>>();
                let retained_source_rows = retained_positions
                    .iter()
                    .map(|position| sampling_frame[*position])
                    .collect::<Vec<_>>();
                let retained_sampling_positions_sha256 =
                    cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                        sampling_frame.len(),
                        &retained_positions,
                    );
                let retained_sample_indices_sha256 = cbsem_exact_case_bootstrap_index_digest_v1(
                    &original.source_dataset_fingerprint,
                    original.source_row_count,
                    &retained_source_rows,
                );
                let slot = match delete_one(omitted_position) {
                    Ok(refit) => match witness_from_delete_one_refit(
                        omitted_position,
                        sampling_frame[omitted_position],
                        &retained_sampling_positions_sha256,
                        &retained_sample_indices_sha256,
                        &parameter_ids,
                        original,
                        refit,
                    ) {
                        Ok(witness) => DeleteOneSlot::Success(witness),
                        Err(reason) => DeleteOneSlot::Invalid {
                            omitted_position,
                            reason,
                        },
                    },
                    Err(CbsemExactCaseBootstrapAttemptErrorV1::Cancelled) => {
                        aborted.store(true, Ordering::Relaxed);
                        DeleteOneSlot::Cancelled
                    }
                    Err(CbsemExactCaseBootstrapAttemptErrorV1::Failed { kind, message }) => {
                        if message.trim().is_empty() {
                            DeleteOneSlot::Invalid {
                                omitted_position,
                                reason: "delete-one failure message is empty".into(),
                            }
                        } else {
                            DeleteOneSlot::Failure(CbsemExactCaseBootstrapDeleteOneFailureV1 {
                                omitted_complete_case_position: omitted_position,
                                omitted_source_row_index: sampling_frame[omitted_position],
                                retained_sampling_positions_sha256,
                                retained_sample_indices_sha256,
                                kind,
                                message,
                            })
                        }
                    }
                };
                let _guard = progress_guard
                    .lock()
                    .expect("exact CB-SEM BCa progress mutex poisoned");
                let completed_replicates = completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                report_progress(ResamplingProgress {
                    phase: ResamplingPhase::Jackknife,
                    completed_replicates,
                    total_replicates: sampling_frame.len() as u32,
                });
                slot
            })
            .collect::<Vec<_>>()
    });
    if is_cancelled()
        || aborted.load(Ordering::Relaxed)
        || slots
            .iter()
            .any(|slot| matches!(slot, DeleteOneSlot::Cancelled))
    {
        return Err(ResamplingError::Cancelled.into());
    }
    slots.sort_by_key(delete_one_slot_position);
    if let Some(DeleteOneSlot::Invalid {
        omitted_position,
        reason,
    }) = slots
        .iter()
        .find(|slot| matches!(slot, DeleteOneSlot::Invalid { .. }))
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::RefitIntegrity {
            replicate_index: u32::try_from(*omitted_position).unwrap_or(u32::MAX),
            reason: reason.clone(),
        });
    }
    let successful = slots
        .iter()
        .filter_map(|slot| match slot {
            DeleteOneSlot::Success(witness) => Some(witness.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let failed = slots
        .iter()
        .filter_map(|slot| match slot {
            DeleteOneSlot::Failure(failure) => Some(failure.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if successful.len() + failed.len() != sampling_frame.len() {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "delete-one accounting is incomplete".into(),
        ));
    }
    recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1(original, base, successful, failed)
}

pub fn required_usable_refits(requested_replicates: u32) -> u32 {
    ((requested_replicates as f64 * CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V1).ceil()
        as u32)
        .max(CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_REPLICATES_V1)
}

pub fn cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
    stream_token: &str,
    seed: u64,
    replicate_index: u32,
    complete_case_sample_size: usize,
    sampling_positions: &[usize],
) -> String {
    let mut digest = Sha256::new();
    digest.update(CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1.as_bytes());
    digest.update([0]);
    digest.update((stream_token.len() as u64).to_le_bytes());
    digest.update(stream_token.as_bytes());
    digest.update(seed.to_le_bytes());
    digest.update(replicate_index.to_le_bytes());
    digest.update((complete_case_sample_size as u64).to_le_bytes());
    digest.update((sampling_positions.len() as u64).to_le_bytes());
    for position in sampling_positions {
        digest.update((*position as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn validate_bca_base(
    sampling_frame: &[usize],
    original: &CbsemExactCaseBootstrapRefitV1,
    base: &CbsemExactCaseBootstrapResultV1,
    workers: usize,
) -> Result<(), CbsemExactCaseBootstrapSchedulerErrorV1> {
    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .collect::<Vec<_>>();
    let identity_positions = (0..sampling_frame.len()).collect::<Vec<_>>();
    let expected_complete_case_universe =
        cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
            &original.source_dataset_fingerprint,
            original.source_row_count,
            sampling_frame,
        );
    let expected_original_positions_digest =
        cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
            sampling_frame.len(),
            &identity_positions,
        );
    let expected_original_indices_digest = cbsem_exact_case_bootstrap_index_digest_v1(
        &original.source_dataset_fingerprint,
        original.source_row_count,
        sampling_frame,
    );
    let inference_available = matches!(
        &base.inference,
        CbsemExactCaseBootstrapInferenceV1::Available
    );
    if workers == 0 || workers > 64 {
        return Err(ResamplingError::InvalidPlan("workers must be between 1 and 64".into()).into());
    }
    if sampling_frame.len() < 10
        || sampling_frame.len() > u32::MAX as usize
        || sampling_frame.len() != original.complete_case_sample_size
        || sampling_frame.windows(2).any(|pair| pair[0] >= pair[1])
        || sampling_frame
            .iter()
            .any(|source_row| *source_row >= original.source_row_count)
        || parameter_ids.is_empty()
        || parameter_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || original.free_parameters.iter().any(|parameter| {
            parameter.parameter_id.trim().is_empty() || !parameter.estimate.is_finite()
        })
        || original.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || original.estimator_method_version != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || original.source_dataset_id.trim().is_empty()
        || original.source_dataset_fingerprint.trim().is_empty()
        || !is_sha256(&original.compiler_analytical_identity_sha256)
        || !is_sha256(&original.plan_sha256)
        || !is_sha256(&original.model_scientific_sha256)
        || original.complete_case_universe_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1
        || original.complete_case_universe_sha256 != expected_complete_case_universe
        || original.resampled_observations != sampling_frame.len()
        || original.covariance_denominator != SemCovarianceDenominatorV4::MaximumLikelihoodN
        || original.sample_indices_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || original.sampling_positions_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1
        || original.sampling_positions_sha256 != expected_original_positions_digest
        || original.sample_indices_sha256 != expected_original_indices_digest
        || original.iterations == 0
        || !original.objective.is_finite()
        || original.objective < 0.0
        || !original.gradient_norm.is_finite()
        || original.gradient_norm < 0.0
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal(
            "delete-one frame or stable point parameter identity/order is invalid".into(),
        ));
    }
    let expected_parameter_ids = parameter_ids
        .iter()
        .map(|parameter_id| (*parameter_id).to_string())
        .collect::<Vec<_>>();
    let minimum = required_usable_refits(base.requested_replicates);
    let replicate_indices = base
        .successful_refits
        .iter()
        .map(|witness| witness.replicate_index)
        .chain(
            base.failed_refits
                .iter()
                .map(|failure| failure.replicate_index),
        )
        .collect::<BTreeSet<_>>();
    let complete_replicate_partition = replicate_indices.len()
        == base.requested_replicates as usize
        && (0..base.requested_replicates).all(|index| replicate_indices.contains(&index));
    if base.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || !(CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_REQUESTED_REPLICATES_V1
            ..=CBSEM_EXACT_CASE_BOOTSTRAP_MAXIMUM_REPLICATES_V1)
            .contains(&base.requested_replicates)
        || base.estimator_method_version != original.estimator_method_version
        || base.source_dataset_id != original.source_dataset_id
        || base.source_dataset_fingerprint != original.source_dataset_fingerprint
        || base.compiler_analytical_identity_sha256 != original.compiler_analytical_identity_sha256
        || base.plan_sha256 != original.plan_sha256
        || base.model_scientific_sha256 != original.model_scientific_sha256
        || base.complete_case_sample_size != original.complete_case_sample_size
        || base.complete_case_universe_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1
        || base.complete_case_universe_sha256 != original.complete_case_universe_sha256
        || base.covariance_denominator != original.covariance_denominator
        || base.sample_indices_digest_method != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || base.sampling_positions_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1
        || base.interval_method != CBSEM_EXACT_CASE_BOOTSTRAP_INTERVAL_METHOD_V1
        || base.stream_token != CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1
        || base.retry_policy != CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1
        || base.max_attempts_per_replicate
            != CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1
        || !is_sha256(&base.outer_recipe_analytical_identity_sha256)
        || !is_sha256(&base.base_point_result_sha256)
        || !is_sha256(&base.compiler_analytical_identity_sha256)
        || !is_sha256(&base.plan_sha256)
        || !is_sha256(&base.model_scientific_sha256)
        || !is_sha256(&base.complete_case_universe_sha256)
        || base.parameter_ids != expected_parameter_ids
        || base.confidence_level != CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1
        || base.minimum_usable_fraction != CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V1
        || base.minimum_usable_replicates != minimum
        || base.attempted_refits != base.requested_replicates
        || base.usable_replicates as usize != base.successful_refits.len()
        || base.failed_replicates as usize != base.failed_refits.len()
        || base.successful_refits.len() + base.failed_refits.len()
            != base.requested_replicates as usize
        || inference_available != (base.usable_replicates >= minimum)
        || base
            .successful_refits
            .windows(2)
            .any(|pair| pair[0].replicate_index >= pair[1].replicate_index)
        || base
            .failed_refits
            .windows(2)
            .any(|pair| pair[0].replicate_index >= pair[1].replicate_index)
        || !complete_replicate_partition
        || base.successful_refits.iter().any(|witness| {
            witness.parameter_estimates.len() != parameter_ids.len()
                || witness
                    .parameter_estimates
                    .iter()
                    .any(|value| !value.is_finite())
                || witness.iterations == 0
                || !witness.objective.is_finite()
                || witness.objective < 0.0
                || !witness.gradient_norm.is_finite()
                || witness.gradient_norm < 0.0
                || !is_sha256(&witness.sampling_positions_sha256)
                || !is_sha256(&witness.sample_indices_sha256)
        })
        || base.failed_refits.iter().any(|failure| {
            failure.message.trim().is_empty()
                || !is_sha256(&failure.sampling_positions_sha256)
                || !is_sha256(&failure.sample_indices_sha256)
        })
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "BCa base v10 authority, parameter order, accounting, or minimum-usable binding differs"
                .into(),
        ));
    }

    let primary_operation = format!("{}:primary", base.stream_token);
    let ledger_digest_matches =
        |replicate_index: u32, sampling_positions_sha256: &str, sample_indices_sha256: &str| {
            let sampling_positions = bootstrap_indices(
                sampling_frame.len(),
                base.seed,
                &primary_operation,
                replicate_index,
            );
            let expected_sampling_positions_sha256 =
                cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                    &base.stream_token,
                    base.seed,
                    replicate_index,
                    sampling_frame.len(),
                    &sampling_positions,
                );
            let sampled_source_rows = sampling_positions
                .iter()
                .map(|position| sampling_frame[*position])
                .collect::<Vec<_>>();
            let expected_sample_indices_sha256 = cbsem_exact_case_bootstrap_index_digest_v1(
                &original.source_dataset_fingerprint,
                original.source_row_count,
                &sampled_source_rows,
            );
            sampling_positions_sha256 == expected_sampling_positions_sha256
                && sample_indices_sha256 == expected_sample_indices_sha256
        };
    if base.successful_refits.iter().any(|witness| {
        !ledger_digest_matches(
            witness.replicate_index,
            &witness.sampling_positions_sha256,
            &witness.sample_indices_sha256,
        )
    }) || base.failed_refits.iter().any(|failure| {
        !ledger_digest_matches(
            failure.replicate_index,
            &failure.sampling_positions_sha256,
            &failure.sample_indices_sha256,
        )
    }) {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "BCa base schedule-position or source-row digest differs from the deterministic ledger"
                .into(),
        ));
    }

    let expected_intervals = if inference_available {
        let original_estimates = original
            .free_parameters
            .iter()
            .map(|parameter| parameter.estimate)
            .collect::<Vec<_>>();
        summarize_intervals(
            &expected_parameter_ids,
            &original_estimates,
            &base.successful_refits,
            CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
        )?
    } else {
        Vec::new()
    };
    if !bootstrap_intervals_are_bit_exact(&base.intervals, &expected_intervals) {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "BCa base percentile intervals differ from the original point and successful-refit ledger"
                .into(),
        ));
    }
    Ok(())
}

fn bootstrap_intervals_are_bit_exact(
    actual: &[CbsemExactCaseBootstrapParameterIntervalV1],
    expected: &[CbsemExactCaseBootstrapParameterIntervalV1],
) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| {
            actual.parameter_id == expected.parameter_id
                && actual.original.to_bits() == expected.original.to_bits()
                && actual.bootstrap_mean.to_bits() == expected.bootstrap_mean.to_bits()
                && actual.bias.to_bits() == expected.bias.to_bits()
                && actual.standard_error.to_bits() == expected.standard_error.to_bits()
                && actual.percentile_lower.to_bits() == expected.percentile_lower.to_bits()
                && actual.percentile_upper.to_bits() == expected.percentile_upper.to_bits()
                && actual.usable_replicates == expected.usable_replicates
        })
}

fn witness_from_delete_one_refit(
    omitted_position: usize,
    omitted_source_row: usize,
    retained_sampling_positions_sha256: &str,
    retained_sample_indices_sha256: &str,
    parameter_ids: &[String],
    original: &CbsemExactCaseBootstrapRefitV1,
    refit: CbsemExactCaseBootstrapDeleteOneRefitV1,
) -> Result<CbsemExactCaseBootstrapDeleteOneWitnessV1, String> {
    if refit.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1
        || refit.estimator_method_version != original.estimator_method_version
        || refit.source_dataset_id != original.source_dataset_id
        || refit.source_dataset_fingerprint != original.source_dataset_fingerprint
        || refit.compiler_analytical_identity_sha256 != original.compiler_analytical_identity_sha256
        || refit.plan_sha256 != original.plan_sha256
        || refit.model_scientific_sha256 != original.model_scientific_sha256
        || refit.source_row_count != original.source_row_count
        || refit.complete_case_sample_size != original.complete_case_sample_size
        || refit.complete_case_universe_sha256 != original.complete_case_universe_sha256
        || refit.omitted_complete_case_position != omitted_position
        || refit.omitted_source_row_index != omitted_source_row
        || refit.retained_observations + 1 != original.complete_case_sample_size
        || refit.covariance_denominator != SemCovarianceDenominatorV4::MaximumLikelihoodN
        || refit.sampling_positions_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1
        || refit.retained_sampling_positions_sha256 != retained_sampling_positions_sha256
        || refit.sample_indices_digest_method != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || refit.retained_sample_indices_sha256 != retained_sample_indices_sha256
        || refit.free_parameters.len() != parameter_ids.len()
        || refit
            .free_parameters
            .iter()
            .zip(parameter_ids)
            .any(|(parameter, expected_id)| {
                parameter.parameter_id != *expected_id || !parameter.estimate.is_finite()
            })
        || refit.iterations == 0
        || !refit.objective.is_finite()
        || refit.objective < 0.0
        || !refit.gradient_norm.is_finite()
        || refit.gradient_norm < 0.0
    {
        return Err(
            "delete-one authority, retained-row digest, parameter order, or finite values differ"
                .into(),
        );
    }
    Ok(CbsemExactCaseBootstrapDeleteOneWitnessV1 {
        omitted_complete_case_position: omitted_position,
        omitted_source_row_index: omitted_source_row,
        retained_sampling_positions_sha256: refit.retained_sampling_positions_sha256,
        retained_sample_indices_sha256: refit.retained_sample_indices_sha256,
        parameter_estimates: refit
            .free_parameters
            .into_iter()
            .map(|parameter| parameter.estimate)
            .collect(),
        iterations: refit.iterations,
        objective: refit.objective,
        gradient_norm: refit.gradient_norm,
    })
}

fn delete_one_slot_position(slot: &DeleteOneSlot) -> usize {
    match slot {
        DeleteOneSlot::Success(witness) => witness.omitted_complete_case_position,
        DeleteOneSlot::Failure(failure) => failure.omitted_complete_case_position,
        DeleteOneSlot::Invalid {
            omitted_position, ..
        } => *omitted_position,
        DeleteOneSlot::Cancelled => usize::MAX,
    }
}

fn validate_plan(
    sampling_frame: &[usize],
    original: &CbsemExactCaseBootstrapRefitV1,
    schedule: CbsemExactCaseBootstrapScheduleV1<'_>,
    workers: usize,
) -> Result<(), CbsemExactCaseBootstrapSchedulerErrorV1> {
    if !(CBSEM_EXACT_CASE_BOOTSTRAP_MINIMUM_REQUESTED_REPLICATES_V1
        ..=CBSEM_EXACT_CASE_BOOTSTRAP_MAXIMUM_REPLICATES_V1)
        .contains(&schedule.requested_replicates)
    {
        return Err(ResamplingError::InvalidPlan(
            "exact CB-SEM case bootstrap requires 500 to 10000 preplanned refits".into(),
        )
        .into());
    }
    if !(1..=64).contains(&workers) {
        return Err(ResamplingError::InvalidPlan("workers must be between 1 and 64".into()).into());
    }
    if !is_sha256(schedule.outer_recipe_analytical_identity_sha256)
        || !is_sha256(schedule.base_point_result_sha256)
        || schedule.stream_token != CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1
        || schedule.retry_policy != CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1
        || schedule.max_attempts_per_replicate
            != CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1
    {
        return Err(ResamplingError::InvalidPlan(
            "outer/base authority, stream, or no-retry identity is invalid".into(),
        )
        .into());
    }
    if sampling_frame.len() < 10
        || sampling_frame.windows(2).any(|rows| rows[0] >= rows[1])
        || sampling_frame
            .iter()
            .any(|row| *row >= original.source_row_count)
    {
        return Err(ResamplingError::InvalidPlan(
            "sampling frame must contain at least ten distinct source rows in strictly increasing storage order"
                .into(),
        )
        .into());
    }
    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .collect::<BTreeSet<_>>();
    let identity_positions = (0..sampling_frame.len()).collect::<Vec<_>>();
    let expected_complete_case_universe =
        cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
            &original.source_dataset_fingerprint,
            original.source_row_count,
            sampling_frame,
        );
    let expected_original_positions_digest =
        cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
            sampling_frame.len(),
            &identity_positions,
        );
    let expected_original_digest = cbsem_exact_case_bootstrap_index_digest_v1(
        &original.source_dataset_fingerprint,
        original.source_row_count,
        sampling_frame,
    );
    if original.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || original.estimator_method_version != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || original.source_dataset_id.trim().is_empty()
        || original.source_dataset_fingerprint.trim().is_empty()
        || !is_sha256(&original.compiler_analytical_identity_sha256)
        || !is_sha256(&original.plan_sha256)
        || !is_sha256(&original.model_scientific_sha256)
        || original.complete_case_sample_size != sampling_frame.len()
        || original.complete_case_universe_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1
        || original.complete_case_universe_sha256 != expected_complete_case_universe
        || original.resampled_observations != sampling_frame.len()
        || original.covariance_denominator != SemCovarianceDenominatorV4::MaximumLikelihoodN
        || original.sample_indices_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        || original.sampling_positions_digest_method
            != CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1
        || original.sampling_positions_sha256 != expected_original_positions_digest
        || original.sample_indices_sha256 != expected_original_digest
        || original.free_parameters.is_empty()
        || parameter_ids.len() != original.free_parameters.len()
        || original.free_parameters.iter().any(|parameter| {
            parameter.parameter_id.trim().is_empty() || !parameter.estimate.is_finite()
        })
        || original.iterations == 0
        || !original.objective.is_finite()
        || original.objective < 0.0
        || !original.gradient_norm.is_finite()
        || original.gradient_norm < 0.0
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal(
            "method/authority/sample digest/parameter identity is inconsistent".into(),
        ));
    }
    if let Some(plan) = schedule.hypothesis_test {
        let ordered_ids = original
            .free_parameters
            .iter()
            .map(|parameter| parameter.parameter_id.as_str())
            .collect::<Vec<_>>();
        if plan.parameter_eligibility.len() != ordered_ids.len()
            || plan
                .parameter_eligibility
                .iter()
                .zip(ordered_ids)
                .any(|(eligibility, parameter_id)| eligibility.parameter_id != parameter_id)
        {
            return Err(ResamplingError::InvalidPlan(
                "zero-null eligibility must contain every exact free parameter exactly once in stable scheduler order"
                    .into(),
            )
            .into());
        }
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn witness_from_refit(
    replicate_index: u32,
    schedule_positions_digest: &str,
    expected_refit_positions_digest: &str,
    expected_source_rows_digest: &str,
    parameter_ids: &[String],
    original: &CbsemExactCaseBootstrapRefitV1,
    refit: CbsemExactCaseBootstrapRefitV1,
) -> Result<CbsemExactCaseBootstrapWitnessV1, String> {
    if refit.method_version != original.method_version
        || refit.estimator_method_version != original.estimator_method_version
        || refit.source_dataset_id != original.source_dataset_id
        || refit.source_dataset_fingerprint != original.source_dataset_fingerprint
        || refit.compiler_analytical_identity_sha256 != original.compiler_analytical_identity_sha256
        || refit.plan_sha256 != original.plan_sha256
        || refit.model_scientific_sha256 != original.model_scientific_sha256
        || refit.source_row_count != original.source_row_count
        || refit.complete_case_sample_size != original.complete_case_sample_size
        || refit.complete_case_universe_digest_method
            != original.complete_case_universe_digest_method
        || refit.complete_case_universe_sha256 != original.complete_case_universe_sha256
        || refit.resampled_observations != original.resampled_observations
        || refit.covariance_denominator != original.covariance_denominator
        || refit.sample_indices_digest_method != original.sample_indices_digest_method
        || refit.sampling_positions_digest_method != original.sampling_positions_digest_method
        || refit.sampling_positions_sha256 != expected_refit_positions_digest
        || refit.sample_indices_sha256 != expected_source_rows_digest
        || refit.free_parameters.len() != parameter_ids.len()
        || refit
            .free_parameters
            .iter()
            .zip(parameter_ids)
            .any(|(parameter, expected_id)| {
                parameter.parameter_id != *expected_id || !parameter.estimate.is_finite()
            })
        || refit.iterations == 0
        || !refit.objective.is_finite()
        || refit.objective < 0.0
        || !refit.gradient_norm.is_finite()
        || refit.gradient_norm < 0.0
    {
        return Err("authority, sample digest, parameter order, or finite value differs".into());
    }
    Ok(CbsemExactCaseBootstrapWitnessV1 {
        replicate_index,
        sampling_positions_sha256: schedule_positions_digest.into(),
        sample_indices_sha256: expected_source_rows_digest.into(),
        parameter_estimates: refit
            .free_parameters
            .into_iter()
            .map(|parameter| parameter.estimate)
            .collect(),
        iterations: refit.iterations,
        objective: refit.objective,
        gradient_norm: refit.gradient_norm,
    })
}

fn slot_replicate_index(slot: &ScheduledSlot) -> u32 {
    match slot {
        ScheduledSlot::Success(witness) => witness.replicate_index,
        ScheduledSlot::Failure(failure) => failure.replicate_index,
        ScheduledSlot::Invalid {
            replicate_index, ..
        } => *replicate_index,
        ScheduledSlot::Cancelled => u32::MAX,
    }
}

/// Compute exact-CFA zero-null selected-tail tests from the successful refit
/// ledger. Failed fits never enter either counts or the plus-one denominator.
pub fn summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
    original: &CbsemExactCaseBootstrapRefitV1,
    successful_refits: &[CbsemExactCaseBootstrapWitnessV1],
    minimum_usable_replicates: u32,
    plan: CbsemExactCaseBootstrapHypothesisTestPlanV1<'_>,
) -> Result<CbsemExactCaseBootstrapHypothesisTestsV1, CbsemExactCaseBootstrapSchedulerErrorV1> {
    let usable_replicates = u32::try_from(successful_refits.len()).map_err(|_| {
        CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "usable refit count exceeds the typed u32 contract".into(),
        )
    })?;
    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .collect::<Vec<_>>();
    if minimum_usable_replicates == 0
        || parameter_ids.is_empty()
        || parameter_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || original
            .free_parameters
            .iter()
            .any(|parameter| !parameter.estimate.is_finite())
        || plan.parameter_eligibility.len() != parameter_ids.len()
        || plan
            .parameter_eligibility
            .iter()
            .zip(parameter_ids.iter().copied())
            .any(|(eligibility, parameter_id)| eligibility.parameter_id != parameter_id)
        || successful_refits
            .windows(2)
            .any(|pair| pair[0].replicate_index >= pair[1].replicate_index)
        || successful_refits.iter().any(|witness| {
            witness.parameter_estimates.len() != parameter_ids.len()
                || witness
                    .parameter_estimates
                    .iter()
                    .any(|estimate| !estimate.is_finite())
        })
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "zero-null parameter identity, order, dimensions, or finite values are invalid".into(),
        ));
    }

    let globally_available = usable_replicates >= minimum_usable_replicates;
    let inference = if globally_available {
        CbsemExactCaseBootstrapHypothesisTestInferenceV1::Available
    } else {
        CbsemExactCaseBootstrapHypothesisTestInferenceV1::Unavailable {
            reason_code: "insufficient_usable_refits".into(),
            message: format!(
                "Exact CB-SEM zero-null tests are unavailable because {usable_replicates} usable refits are below the required {minimum_usable_replicates}."
            ),
        }
    };
    let mut parameters = Vec::with_capacity(parameter_ids.len());
    for (parameter_index, (point, eligibility)) in original
        .free_parameters
        .iter()
        .zip(plan.parameter_eligibility)
        .enumerate()
    {
        let outcome = match &eligibility.status {
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable { reason } => {
                CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Unavailable {
                    reason: map_zero_null_unavailable_reason(*reason),
                }
            }
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available
                if !globally_available =>
            {
                CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Unavailable {
                    reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::InsufficientUsableReplicates,
                }
            }
            CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available => {
                let threshold = point.estimate;
                let absolute_threshold = threshold.abs();
                let mut two_sided_exceedances = 0_u32;
                let mut greater_or_equal_exceedances = 0_u32;
                let mut less_or_equal_exceedances = 0_u32;
                for witness in successful_refits {
                    let delta = witness.parameter_estimates[parameter_index] - threshold;
                    if !delta.is_finite() {
                        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
                            format!("nonfinite null-centered arithmetic for parameter {}", point.parameter_id),
                        ));
                    }
                    two_sided_exceedances += (delta.abs() >= absolute_threshold) as u32;
                    greater_or_equal_exceedances += (delta >= threshold) as u32;
                    less_or_equal_exceedances += (delta <= threshold) as u32;
                }
                let p_value_two_sided = plus_one_probability(
                    two_sided_exceedances,
                    usable_replicates,
                );
                let p_value_greater = plus_one_probability(
                    greater_or_equal_exceedances,
                    usable_replicates,
                );
                let p_value_less =
                    plus_one_probability(less_or_equal_exceedances, usable_replicates);
                let (selected_exceedances, selected_p_value) = match plan.selected_test_tail {
                    CbsemBootstrapTestTail::TwoSided => {
                        (two_sided_exceedances, p_value_two_sided)
                    }
                    CbsemBootstrapTestTail::OneSidedGreater => {
                        (greater_or_equal_exceedances, p_value_greater)
                    }
                    CbsemBootstrapTestTail::OneSidedLess => {
                        (less_or_equal_exceedances, p_value_less)
                    }
                };
                CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
                    point_estimate: threshold,
                    two_sided_exceedances,
                    greater_or_equal_exceedances,
                    less_or_equal_exceedances,
                    p_value_two_sided,
                    p_value_greater,
                    p_value_less,
                    selected_exceedances,
                    selected_p_value,
                    reject_null: selected_p_value
                        <= CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1,
                }
            }
        };
        parameters.push(CbsemExactCaseBootstrapHypothesisTestParameterV1 {
            parameter_id: point.parameter_id.clone(),
            outcome,
        });
    }
    Ok(CbsemExactCaseBootstrapHypothesisTestsV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_METHOD_VERSION_V1.into(),
        null_hypothesis: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_NULL_HYPOTHESIS_V1.into(),
        statistic: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_STATISTIC_V1.into(),
        tie_policy: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_TIE_POLICY_V1.into(),
        probability_method: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_PROBABILITY_METHOD_V1.into(),
        decision_rule: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_DECISION_RULE_V1.into(),
        selected_test_tail: plan.selected_test_tail,
        null_value: 0.0,
        significance_level: CBSEM_EXACT_CASE_BOOTSTRAP_TEST_TAIL_SIGNIFICANCE_LEVEL_V1,
        usable_replicates,
        inference,
        parameters,
    })
}

fn plus_one_probability(exceedances: u32, usable_replicates: u32) -> f64 {
    (f64::from(exceedances) + 1.0) / (f64::from(usable_replicates) + 1.0)
}

fn map_zero_null_unavailable_reason(
    reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1,
) -> CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1 {
    match reason {
        CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary => {
            CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::NonregularVarianceBoundary
        }
        CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::ZeroNullOutsideOpenDomain => {
            CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::ZeroNullOutsideOpenDomain
        }
        CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::UnsupportedParameterFamily => {
            CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::UnsupportedParameterFamily
        }
    }
}

fn summarize_intervals(
    parameter_ids: &[String],
    original_estimates: &[f64],
    successful_refits: &[CbsemExactCaseBootstrapWitnessV1],
    confidence_level: f64,
) -> Result<Vec<CbsemExactCaseBootstrapParameterIntervalV1>, CbsemExactCaseBootstrapSchedulerErrorV1>
{
    if parameter_ids.is_empty()
        || parameter_ids.len() != original_estimates.len()
        || successful_refits.len() < 2
        || confidence_level <= 0.0
        || confidence_level >= 1.0
        || original_estimates.iter().any(|value| !value.is_finite())
        || successful_refits.iter().any(|refit| {
            refit.parameter_estimates.len() != parameter_ids.len()
                || refit
                    .parameter_estimates
                    .iter()
                    .any(|value| !value.is_finite())
        })
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "parameter dimensions, confidence, or finite values are invalid".into(),
        ));
    }
    let lower_probability = (1.0 - confidence_level) / 2.0;
    let upper_probability = 1.0 - lower_probability;
    let mut intervals = Vec::with_capacity(parameter_ids.len());
    for (parameter_index, (parameter_id, original)) in
        parameter_ids.iter().zip(original_estimates).enumerate()
    {
        let values = successful_refits
            .iter()
            .map(|refit| refit.parameter_estimates[parameter_index])
            .collect::<Vec<_>>();
        let bootstrap_mean = values.iter().sum::<f64>() / values.len() as f64;
        let standard_error = (values
            .iter()
            .map(|value| (value - bootstrap_mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64)
            .sqrt();
        let mut sorted = values;
        sorted.sort_by(f64::total_cmp);
        let percentile_lower = type7_quantile(&sorted, lower_probability);
        let percentile_upper = type7_quantile(&sorted, upper_probability);
        let bias = bootstrap_mean - original;
        if [
            bootstrap_mean,
            standard_error,
            percentile_lower,
            percentile_upper,
            bias,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
                format!("nonfinite interval arithmetic for parameter {parameter_id}"),
            ));
        }
        intervals.push(CbsemExactCaseBootstrapParameterIntervalV1 {
            parameter_id: parameter_id.clone(),
            original: *original,
            bootstrap_mean,
            bias,
            standard_error,
            percentile_lower,
            percentile_upper,
            usable_replicates: successful_refits.len() as u32,
        });
    }
    Ok(intervals)
}

#[derive(Debug)]
enum ValidatedAnalyticStandardErrors {
    Available {
        information_method: String,
        standard_errors: Vec<f64>,
    },
    Unavailable {
        reason: qpls_estimation::CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1,
    },
}

fn validate_analytic_standard_errors(
    parameter_ids: &[&str],
    receipt: &CbsemExactCaseBootstrapRefitStandardErrorsV1,
) -> Result<ValidatedAnalyticStandardErrors, String> {
    if receipt.method_version != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
    {
        return Err("analytical standard-error method version differs".into());
    }
    match &receipt.outcome {
        CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
            information_method,
            parameters,
        } => {
            if information_method != CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
                || parameters.len() != parameter_ids.len()
                || parameters.iter().zip(parameter_ids).any(|(row, expected)| {
                    row.parameter_id != *expected
                        || !row.standard_error.is_finite()
                        || row.standard_error <= 0.0
                })
            {
                return Err(
                    "analytical standard-error information method, order, or positive finite values differ"
                        .into(),
                );
            }
            Ok(ValidatedAnalyticStandardErrors::Available {
                information_method: information_method.clone(),
                standard_errors: parameters.iter().map(|row| row.standard_error).collect(),
            })
        }
        CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable { reason } => {
            Ok(ValidatedAnalyticStandardErrors::Unavailable { reason: *reason })
        }
    }
}

/// Purely recompute the analytically studentized sidecar from the immutable
/// point refit, unchanged v10 base ledger, and one standard-error receipt per
/// successful refit. Archive readers can compare this result bit-for-bit with
/// a persisted sidecar without executing an optimizer or drawing samples.
pub fn recompute_cbsem_exact_case_bootstrap_studentized_sidecar_v1(
    original: &CbsemExactCaseBootstrapRefitV1,
    point_standard_errors: &CbsemExactCaseBootstrapRefitStandardErrorsV1,
    base: &CbsemExactCaseBootstrapResultV1,
    refit_standard_errors: Vec<CbsemExactCaseBootstrapRefitStandardErrorsV1>,
) -> Result<CbsemExactCaseBootstrapStudentizedSidecarV1, CbsemExactCaseBootstrapSchedulerErrorV1> {
    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.as_str())
        .collect::<Vec<_>>();
    if base
        .parameter_ids
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != parameter_ids
        || base.successful_refits.len() != refit_standard_errors.len()
        || base
            .successful_refits
            .windows(2)
            .any(|pair| pair[0].replicate_index >= pair[1].replicate_index)
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "studentized parameter or successful-refit ledger order differs from the base aggregate"
                .into(),
        ));
    }

    let point = validate_analytic_standard_errors(&parameter_ids, point_standard_errors)
        .map_err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal)?;
    let mut compact_receipts = Vec::with_capacity(refit_standard_errors.len());
    let mut usable = Vec::new();
    for (witness, receipt) in base
        .successful_refits
        .iter()
        .zip(refit_standard_errors.iter())
    {
        let validated =
            validate_analytic_standard_errors(&parameter_ids, receipt).map_err(|reason| {
                CbsemExactCaseBootstrapSchedulerErrorV1::RefitIntegrity {
                    replicate_index: witness.replicate_index,
                    reason,
                }
            })?;
        let outcome = match validated {
            ValidatedAnalyticStandardErrors::Available {
                information_method,
                standard_errors,
            } => {
                usable.push((witness, standard_errors.clone()));
                CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Available {
                    information_method,
                    standard_errors,
                }
            }
            ValidatedAnalyticStandardErrors::Unavailable { reason } => {
                CbsemExactCaseBootstrapStudentizedRefitStandardErrorOutcomeV1::Unavailable {
                    reason,
                }
            }
        };
        compact_receipts.push(CbsemExactCaseBootstrapStudentizedRefitStandardErrorsV1 {
            replicate_index: witness.replicate_index,
            outcome,
        });
    }
    let studentized_usable_replicates = u32::try_from(usable.len()).map_err(|_| {
        CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "studentized usable count exceeds the typed u32 contract".into(),
        )
    })?;
    let minimum_usable_replicates = base.minimum_usable_replicates;
    let unavailable_reason = match &point {
        ValidatedAnalyticStandardErrors::Unavailable { .. } => Some(
            CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::PointStandardErrorsUnavailable,
        ),
        ValidatedAnalyticStandardErrors::Available { .. }
            if studentized_usable_replicates < minimum_usable_replicates =>
        {
            Some(
                CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates,
            )
        }
        ValidatedAnalyticStandardErrors::Available { .. } => None,
    };
    let inference = match unavailable_reason {
        None => CbsemExactCaseBootstrapStudentizedInferenceV1::Available,
        Some(reason) => CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable {
            reason,
            message: match reason {
                CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::PointStandardErrorsUnavailable =>
                    "Analytically studentized inference is unavailable because the point estimate has no whole-vector analytical standard-error receipt.".into(),
                CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates => format!(
                    "Analytically studentized inference is unavailable because {studentized_usable_replicates} whole-vector usable refits are below the required {minimum_usable_replicates}."
                ),
            },
        },
    };

    let intervals = if let (
        None,
        ValidatedAnalyticStandardErrors::Available {
            standard_errors: point_standard_errors,
            ..
        },
    ) = (unavailable_reason, &point)
    {
        let lower_probability = (1.0 - CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1) / 2.0;
        let upper_probability = 1.0 - lower_probability;
        parameter_ids
            .iter()
            .enumerate()
            .map(|(parameter_index, parameter_id)| {
                let point_estimate = original.free_parameters[parameter_index].estimate;
                let point_standard_error = point_standard_errors[parameter_index];
                let mut pivots = usable
                    .iter()
                    .map(|(witness, standard_errors)| {
                        (witness.parameter_estimates[parameter_index] - point_estimate)
                            / standard_errors[parameter_index]
                    })
                    .collect::<Vec<_>>();
                if pivots.iter().any(|pivot| !pivot.is_finite()) {
                    return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
                        format!("nonfinite studentized pivot for parameter {parameter_id}"),
                    ));
                }
                pivots.sort_by(f64::total_cmp);
                let lower_pivot_quantile = type7_quantile(&pivots, lower_probability);
                let upper_pivot_quantile = type7_quantile(&pivots, upper_probability);
                let interval_lower = point_estimate - upper_pivot_quantile * point_standard_error;
                let interval_upper = point_estimate - lower_pivot_quantile * point_standard_error;
                if [
                    lower_pivot_quantile,
                    upper_pivot_quantile,
                    interval_lower,
                    interval_upper,
                ]
                .iter()
                .any(|value| !value.is_finite())
                    || interval_lower > interval_upper
                {
                    return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
                        format!(
                            "invalid reversed studentized interval for parameter {parameter_id}"
                        ),
                    ));
                }
                Ok(CbsemExactCaseBootstrapStudentizedParameterIntervalV1 {
                    parameter_id: (*parameter_id).into(),
                    outcome:
                        CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Available {
                            point_estimate,
                            point_standard_error,
                            lower_pivot_quantile,
                            upper_pivot_quantile,
                            interval_lower,
                            interval_upper,
                            usable_replicates: studentized_usable_replicates,
                        },
                })
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let reason = unavailable_reason.expect("unavailable studentized inference has a reason");
        parameter_ids
            .iter()
            .map(
                |parameter_id| CbsemExactCaseBootstrapStudentizedParameterIntervalV1 {
                    parameter_id: (*parameter_id).into(),
                    outcome:
                        CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Unavailable {
                            reason,
                        },
                },
            )
            .collect()
    };

    Ok(CbsemExactCaseBootstrapStudentizedSidecarV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_METHOD_VERSION_V1.into(),
        standard_error_method_version:
            CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1.into(),
        expected_information_method:
            CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1.into(),
        pivot_method: CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_PIVOT_METHOD_V1.into(),
        quantile_method: CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_QUANTILE_METHOD_V1.into(),
        interval_method: CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_INTERVAL_METHOD_V1.into(),
        archive_validation_scope:
            CBSEM_EXACT_CASE_BOOTSTRAP_STUDENTIZED_ARCHIVE_VALIDATION_SCOPE_V1.into(),
        confidence_level: CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
        minimum_usable_fraction: base.minimum_usable_fraction,
        minimum_usable_replicates,
        studentized_usable_replicates,
        parameter_ids: base.parameter_ids.clone(),
        point_standard_errors: point_standard_errors.clone(),
        inference,
        intervals,
        refit_standard_errors: compact_receipts,
    })
}

/// Purely recompute an exact-CFA BCa sidecar from the immutable point refit,
/// unchanged base bootstrap ledger, and persisted ordered delete-one evidence.
/// The complete-case frame is reconstructed from the mandatory 0..N-1
/// omission partition and is then used to revalidate every authority and
/// digest before any derived interval is accepted. No optimizer, bootstrap
/// draw, raw-data access, or expected-information replay is performed.
pub fn recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1(
    original: &CbsemExactCaseBootstrapRefitV1,
    base: &CbsemExactCaseBootstrapResultV1,
    successful_delete_one_refits: Vec<CbsemExactCaseBootstrapDeleteOneWitnessV1>,
    failed_delete_one_refits: Vec<CbsemExactCaseBootstrapDeleteOneFailureV1>,
) -> Result<CbsemExactCaseBootstrapBcaSidecarV1, CbsemExactCaseBootstrapSchedulerErrorV1> {
    let sampling_frame = reconstruct_and_validate_bca_delete_one_frame(
        original,
        &successful_delete_one_refits,
        &failed_delete_one_refits,
    )?;
    validate_bca_base(&sampling_frame, original, base, 1)?;
    summarize_bca_sidecar(
        original,
        base,
        successful_delete_one_refits,
        failed_delete_one_refits,
    )
}

fn reconstruct_and_validate_bca_delete_one_frame(
    original: &CbsemExactCaseBootstrapRefitV1,
    successful: &[CbsemExactCaseBootstrapDeleteOneWitnessV1],
    failed: &[CbsemExactCaseBootstrapDeleteOneFailureV1],
) -> Result<Vec<usize>, CbsemExactCaseBootstrapSchedulerErrorV1> {
    let case_count = original.complete_case_sample_size;
    if case_count < 10
        || successful.len() + failed.len() != case_count
        || successful.windows(2).any(|pair| {
            pair[0].omitted_complete_case_position >= pair[1].omitted_complete_case_position
        })
        || failed.windows(2).any(|pair| {
            pair[0].omitted_complete_case_position >= pair[1].omitted_complete_case_position
        })
    {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "delete-one evidence is not an ordered complete 0..N-1 partition".into(),
        ));
    }

    let mut source_rows = vec![None; case_count];
    for (position, source_row) in successful
        .iter()
        .map(|row| {
            (
                row.omitted_complete_case_position,
                row.omitted_source_row_index,
            )
        })
        .chain(failed.iter().map(|row| {
            (
                row.omitted_complete_case_position,
                row.omitted_source_row_index,
            )
        }))
    {
        if position >= case_count
            || source_row >= original.source_row_count
            || source_rows[position].replace(source_row).is_some()
        {
            return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
                "delete-one evidence has an out-of-range or duplicate omission identity".into(),
            ));
        }
    }
    let sampling_frame = source_rows
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
                "delete-one evidence does not cover every complete-case omission".into(),
            )
        })?;
    if sampling_frame.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "delete-one source-row evidence is not a canonical increasing sampling frame".into(),
        ));
    }

    let expected_receipts = (0..case_count)
        .map(|omitted_position| {
            let retained_positions = (0..case_count)
                .filter(|position| *position != omitted_position)
                .collect::<Vec<_>>();
            let retained_source_rows = retained_positions
                .iter()
                .map(|position| sampling_frame[*position])
                .collect::<Vec<_>>();
            (
                cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                    case_count,
                    &retained_positions,
                ),
                cbsem_exact_case_bootstrap_index_digest_v1(
                    &original.source_dataset_fingerprint,
                    original.source_row_count,
                    &retained_source_rows,
                ),
            )
        })
        .collect::<Vec<_>>();

    let parameter_count = original.free_parameters.len();
    if successful.iter().any(|row| {
        let expected = &expected_receipts[row.omitted_complete_case_position];
        row.retained_sampling_positions_sha256 != expected.0
            || row.retained_sample_indices_sha256 != expected.1
            || row.parameter_estimates.len() != parameter_count
            || row
                .parameter_estimates
                .iter()
                .any(|value| !value.is_finite())
            || row.iterations == 0
            || !row.objective.is_finite()
            || row.objective < 0.0
            || !row.gradient_norm.is_finite()
            || row.gradient_norm < 0.0
    }) || failed.iter().any(|row| {
        let expected = &expected_receipts[row.omitted_complete_case_position];
        row.retained_sampling_positions_sha256 != expected.0
            || row.retained_sample_indices_sha256 != expected.1
            || row.message.trim().is_empty()
    }) {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "delete-one evidence digest, parameter, convergence, or failure payload is invalid"
                .into(),
        ));
    }
    Ok(sampling_frame)
}

fn summarize_bca_sidecar(
    original: &CbsemExactCaseBootstrapRefitV1,
    base: &CbsemExactCaseBootstrapResultV1,
    successful_delete_one_refits: Vec<CbsemExactCaseBootstrapDeleteOneWitnessV1>,
    failed_delete_one_refits: Vec<CbsemExactCaseBootstrapDeleteOneFailureV1>,
) -> Result<CbsemExactCaseBootstrapBcaSidecarV1, CbsemExactCaseBootstrapSchedulerErrorV1> {
    let parameter_ids = original
        .free_parameters
        .iter()
        .map(|parameter| parameter.parameter_id.clone())
        .collect::<Vec<_>>();
    if successful_delete_one_refits.windows(2).any(|pair| {
        pair[0].omitted_complete_case_position >= pair[1].omitted_complete_case_position
    }) || failed_delete_one_refits.windows(2).any(|pair| {
        pair[0].omitted_complete_case_position >= pair[1].omitted_complete_case_position
    }) || successful_delete_one_refits.iter().any(|witness| {
        witness.parameter_estimates.len() != parameter_ids.len()
            || witness
                .parameter_estimates
                .iter()
                .any(|value| !value.is_finite())
    }) {
        return Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(
            "delete-one stable order, dimensions, or finite values are invalid".into(),
        ));
    }
    let global_reason = if base.usable_replicates < base.minimum_usable_replicates
        || !matches!(
            &base.inference,
            CbsemExactCaseBootstrapInferenceV1::Available
        ) {
        Some(CbsemExactCaseBootstrapBcaUnavailableReasonV1::BaseInferenceUnavailable)
    } else if !failed_delete_one_refits.is_empty()
        || successful_delete_one_refits.len() != original.complete_case_sample_size
    {
        Some(CbsemExactCaseBootstrapBcaUnavailableReasonV1::IncompleteDeleteOneLedger)
    } else {
        None
    };
    let inference = match global_reason {
        None => CbsemExactCaseBootstrapBcaInferenceV1::Available,
        Some(reason) => CbsemExactCaseBootstrapBcaInferenceV1::Unavailable {
            reason,
            message: match reason {
                CbsemExactCaseBootstrapBcaUnavailableReasonV1::BaseInferenceUnavailable => format!(
                    "BCa inference is unavailable because {} successful bootstrap point refits are below the bound minimum {}.",
                    base.usable_replicates, base.minimum_usable_replicates
                ),
                CbsemExactCaseBootstrapBcaUnavailableReasonV1::IncompleteDeleteOneLedger => {
                    format!(
                        "BCa inference is unavailable because {} of {} mandatory delete-one fits failed.",
                        failed_delete_one_refits.len(),
                        original.complete_case_sample_size
                    )
                }
                _ => unreachable!("global BCa unavailability has a global reason"),
            },
        },
    };
    let intervals = if let Some(reason) = global_reason {
        parameter_ids
            .iter()
            .map(
                |parameter_id| CbsemExactCaseBootstrapBcaParameterIntervalV1 {
                    parameter_id: parameter_id.clone(),
                    outcome: CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable {
                        reason,
                    },
                },
            )
            .collect()
    } else {
        parameter_ids
            .iter()
            .enumerate()
            .map(|(parameter_index, parameter_id)| {
                let bootstrap = base
                    .successful_refits
                    .iter()
                    .map(|witness| witness.parameter_estimates[parameter_index])
                    .collect::<Vec<_>>();
                let delete_one = successful_delete_one_refits
                    .iter()
                    .map(|witness| witness.parameter_estimates[parameter_index])
                    .collect::<Vec<_>>();
                CbsemExactCaseBootstrapBcaParameterIntervalV1 {
                    parameter_id: parameter_id.clone(),
                    outcome: bca_parameter_interval(
                        original.free_parameters[parameter_index].estimate,
                        &bootstrap,
                        &delete_one,
                        CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
                    ),
                }
            })
            .collect()
    };
    Ok(CbsemExactCaseBootstrapBcaSidecarV1 {
        method_version: CBSEM_EXACT_CASE_BOOTSTRAP_BCA_METHOD_VERSION_V1.into(),
        base_bootstrap_method_version: base.method_version.clone(),
        outer_recipe_analytical_identity_sha256: base
            .outer_recipe_analytical_identity_sha256
            .clone(),
        base_point_result_sha256: base.base_point_result_sha256.clone(),
        compiler_analytical_identity_sha256: base.compiler_analytical_identity_sha256.clone(),
        plan_sha256: base.plan_sha256.clone(),
        model_scientific_sha256: base.model_scientific_sha256.clone(),
        delete_one_refit_method_version:
            CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1.into(),
        bias_correction_method: CBSEM_EXACT_CASE_BOOTSTRAP_BCA_BIAS_CORRECTION_METHOD_V1.into(),
        acceleration_method: CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ACCELERATION_METHOD_V2.into(),
        adjusted_probability_method: CBSEM_EXACT_CASE_BOOTSTRAP_BCA_ADJUSTMENT_METHOD_V2.into(),
        quantile_method: CBSEM_EXACT_CASE_BOOTSTRAP_BCA_QUANTILE_METHOD_V1.into(),
        retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_BCA_RETRY_POLICY_V1.into(),
        confidence_level: CBSEM_EXACT_CASE_BOOTSTRAP_CONFIDENCE_LEVEL_V1,
        bootstrap_usable_replicates: base.usable_replicates,
        minimum_bootstrap_usable_replicates: base.minimum_usable_replicates,
        delete_one_case_count: original.complete_case_sample_size,
        parameter_ids,
        inference,
        intervals,
        successful_delete_one_refits,
        failed_delete_one_refits,
    })
}

fn bca_parameter_interval(
    point_estimate: f64,
    bootstrap_values: &[f64],
    delete_one_values: &[f64],
    confidence_level: f64,
) -> CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1 {
    let unavailable =
        |reason| CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable { reason };
    if bootstrap_values.is_empty()
        || delete_one_values.len() < 3
        || !point_estimate.is_finite()
        || !(0.0..1.0).contains(&confidence_level)
        || bootstrap_values.iter().any(|value| !value.is_finite())
        || delete_one_values.iter().any(|value| !value.is_finite())
    {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteJackknifeArithmetic,
        );
    }
    let less = bootstrap_values
        .iter()
        .filter(|value| **value < point_estimate)
        .count() as f64;
    let ties = bootstrap_values
        .iter()
        .filter(|value| **value == point_estimate)
        .count() as f64;
    let probability = (less + 0.5 * ties) / bootstrap_values.len() as f64;
    if !probability.is_finite() || probability <= 0.0 || probability >= 1.0 {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::BiasCorrectionProbabilityAtBoundary,
        );
    }
    let normal = Normal::standard();
    let bias_correction = normal.inverse_cdf(probability);
    let jackknife_mean =
        neumaier_sum(delete_one_values.iter().copied()) / delete_one_values.len() as f64;
    let centered = delete_one_values
        .iter()
        .map(|value| jackknife_mean - value)
        .collect::<Vec<_>>();
    let sum_squares = neumaier_sum(centered.iter().map(|value| value.powi(2)));
    if !jackknife_mean.is_finite() || !sum_squares.is_finite() {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteJackknifeArithmetic,
        );
    }
    if sum_squares == 0.0 {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::DegenerateJackknifeAcceleration,
        );
    }
    let sum_cubes = neumaier_sum(centered.iter().map(|value| value.powi(3)));
    let acceleration = sum_cubes / (6.0 * sum_squares.powf(1.5));
    if !bias_correction.is_finite() || !acceleration.is_finite() {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteJackknifeArithmetic,
        );
    }
    let tail = (1.0 - confidence_level) / 2.0;
    let adjusted_lower_probability =
        match bca_adjusted_probability(&normal, bias_correction, acceleration, tail) {
            Ok(value) => value,
            Err(reason) => return unavailable(reason),
        };
    let adjusted_upper_probability =
        match bca_adjusted_probability(&normal, bias_correction, acceleration, 1.0 - tail) {
            Ok(value) => value,
            Err(reason) => return unavailable(reason),
        };
    if adjusted_lower_probability > adjusted_upper_probability {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::AdjustedProbabilityOrderInvalid,
        );
    }
    let mut sorted = bootstrap_values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let interval_lower = type7_quantile(&sorted, adjusted_lower_probability);
    let interval_upper = type7_quantile(&sorted, adjusted_upper_probability);
    if !interval_lower.is_finite() || !interval_upper.is_finite() || interval_lower > interval_upper
    {
        return unavailable(
            CbsemExactCaseBootstrapBcaUnavailableReasonV1::NonfiniteOrReversedInterval,
        );
    }
    CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Available {
        point_estimate,
        bias_correction,
        acceleration,
        adjusted_lower_probability,
        adjusted_upper_probability,
        interval_lower,
        interval_upper,
        usable_replicates: bootstrap_values.len() as u32,
    }
}

fn bca_adjusted_probability(
    normal: &Normal,
    bias_correction: f64,
    acceleration: f64,
    nominal: f64,
) -> Result<f64, CbsemExactCaseBootstrapBcaUnavailableReasonV1> {
    let z = normal.inverse_cdf(nominal);
    let denominator = 1.0 - acceleration * (bias_correction + z);
    if !denominator.is_finite()
        || denominator.abs() <= CBSEM_EXACT_CASE_BOOTSTRAP_BCA_MIN_ABS_ADJUSTMENT_DENOMINATOR_V1
    {
        return Err(CbsemExactCaseBootstrapBcaUnavailableReasonV1::SingularAccelerationAdjustment);
    }
    let argument = bias_correction + (bias_correction + z) / denominator;
    let value = 0.5 * libm::erfc(-argument / std::f64::consts::SQRT_2);
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(CbsemExactCaseBootstrapBcaUnavailableReasonV1::InvalidAdjustedProbability);
    }
    Ok(value)
}

/// Deterministic Neumaier compensated summation in the supplied order.
/// This is intentionally local to the BCa jackknife submethod: the bound base
/// percentile implementation and its serialized historical arithmetic remain
/// byte-for-byte unchanged.
fn neumaier_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for value in values {
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    sum + correction
}

fn type7_quantile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] + fraction * (sorted[upper] - sorted[lower])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_estimation::CbsemExactCaseBootstrapParameterEstimateV1;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize},
    };

    const OUTER_RECIPE_SHA256: &str =
        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const BASE_POINT_SHA256: &str =
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn schedule(
        requested_replicates: u32,
        seed: u64,
    ) -> CbsemExactCaseBootstrapScheduleV1<'static> {
        CbsemExactCaseBootstrapScheduleV1 {
            outer_recipe_analytical_identity_sha256: OUTER_RECIPE_SHA256,
            base_point_result_sha256: BASE_POINT_SHA256,
            requested_replicates,
            seed,
            stream_token: CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
            retry_policy: CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1,
            max_attempts_per_replicate: CBSEM_EXACT_CASE_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V1,
            hypothesis_test: None,
        }
    }

    fn sampling_frame() -> Vec<usize> {
        (2..22).collect()
    }

    fn original_refit(frame: &[usize]) -> CbsemExactCaseBootstrapRefitV1 {
        let source_fingerprint = "dataset-fingerprint".to_string();
        let identity_positions = (0..frame.len()).collect::<Vec<_>>();
        CbsemExactCaseBootstrapRefitV1 {
            method_version: CBSEM_EXACT_CASE_BOOTSTRAP_METHOD_VERSION_V1.into(),
            estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3.into(),
            source_dataset_id: "dataset-id".into(),
            source_dataset_fingerprint: source_fingerprint.clone(),
            compiler_analytical_identity_sha256: "aa".repeat(32),
            plan_sha256: "bb".repeat(32),
            model_scientific_sha256: "cc".repeat(32),
            source_row_count: 24,
            complete_case_sample_size: frame.len(),
            complete_case_universe_digest_method:
                CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1.into(),
            complete_case_universe_sha256:
                cbsem_exact_case_bootstrap_complete_case_universe_digest_v1(
                    &source_fingerprint,
                    24,
                    frame,
                ),
            resampled_observations: frame.len(),
            covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
            sample_indices_digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.into(),
            sampling_positions_digest_method:
                CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1.into(),
            sampling_positions_sha256: cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                frame.len(),
                &identity_positions,
            ),
            sample_indices_sha256: cbsem_exact_case_bootstrap_index_digest_v1(
                &source_fingerprint,
                24,
                frame,
            ),
            free_parameters: vec![
                CbsemExactCaseBootstrapParameterEstimateV1 {
                    parameter_id: "loading:x1".into(),
                    estimate: 1.5,
                },
                CbsemExactCaseBootstrapParameterEstimateV1 {
                    parameter_id: "variance:x".into(),
                    estimate: 2.5,
                },
            ],
            iterations: 5,
            objective: 0.25,
            gradient_norm: 0.01,
        }
    }

    fn successful_refit(
        original: &CbsemExactCaseBootstrapRefitV1,
        frame: &[usize],
        positions: &[usize],
        first: f64,
    ) -> CbsemExactCaseBootstrapRefitV1 {
        let mut refit = original.clone();
        let source_rows = positions
            .iter()
            .map(|position| frame[*position])
            .collect::<Vec<_>>();
        refit.sampling_positions_sha256 =
            cbsem_exact_case_bootstrap_sampling_positions_digest_v1(frame.len(), positions);
        refit.sample_indices_sha256 = cbsem_exact_case_bootstrap_index_digest_v1(
            &refit.source_dataset_fingerprint,
            refit.source_row_count,
            &source_rows,
        );
        refit.free_parameters[0].estimate = first;
        refit.free_parameters[1].estimate = first * 2.0;
        refit.iterations = 3;
        refit.objective = first.abs() / 100.0;
        refit.gradient_norm = 0.001;
        refit
    }

    fn available_standard_errors(
        parameter_ids: &[&str],
        standard_errors: &[f64],
    ) -> CbsemExactCaseBootstrapRefitStandardErrorsV1 {
        CbsemExactCaseBootstrapRefitStandardErrorsV1 {
            method_version: CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
                .into(),
            outcome: CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available {
                information_method: CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_EXPECTED_INFORMATION_METHOD_V1
                    .into(),
                parameters: parameter_ids
                    .iter()
                    .zip(standard_errors)
                    .map(|(parameter_id, standard_error)| {
                        qpls_estimation::CbsemExactCaseBootstrapParameterStandardErrorV1 {
                            parameter_id: (*parameter_id).into(),
                            standard_error: *standard_error,
                        }
                    })
                    .collect(),
            },
        }
    }

    fn unavailable_standard_errors() -> CbsemExactCaseBootstrapRefitStandardErrorsV1 {
        CbsemExactCaseBootstrapRefitStandardErrorsV1 {
            method_version: CBSEM_EXACT_CASE_BOOTSTRAP_REFIT_STANDARD_ERRORS_METHOD_VERSION_V1
                .into(),
            outcome: CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Unavailable {
                reason: qpls_estimation::CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1::SingularInformation,
            },
        }
    }

    fn with_standard_errors(
        refit: CbsemExactCaseBootstrapRefitV1,
        standard_errors: CbsemExactCaseBootstrapRefitStandardErrorsV1,
    ) -> CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1 {
        CbsemExactCaseBootstrapRefitWithAnalyticStandardErrorsV1 {
            refit,
            standard_errors,
        }
    }

    fn delete_one_refit(
        original: &CbsemExactCaseBootstrapRefitV1,
        frame: &[usize],
        omitted_position: usize,
        first: f64,
        second: f64,
    ) -> CbsemExactCaseBootstrapDeleteOneRefitV1 {
        let retained_positions = (0..frame.len())
            .filter(|position| *position != omitted_position)
            .collect::<Vec<_>>();
        let retained_rows = retained_positions
            .iter()
            .map(|position| frame[*position])
            .collect::<Vec<_>>();
        CbsemExactCaseBootstrapDeleteOneRefitV1 {
            method_version: CBSEM_EXACT_CASE_BOOTSTRAP_DELETE_ONE_REFIT_METHOD_VERSION_V1.into(),
            estimator_method_version: original.estimator_method_version.clone(),
            source_dataset_id: original.source_dataset_id.clone(),
            source_dataset_fingerprint: original.source_dataset_fingerprint.clone(),
            compiler_analytical_identity_sha256: original
                .compiler_analytical_identity_sha256
                .clone(),
            plan_sha256: original.plan_sha256.clone(),
            model_scientific_sha256: original.model_scientific_sha256.clone(),
            source_row_count: original.source_row_count,
            complete_case_sample_size: frame.len(),
            complete_case_universe_sha256: original.complete_case_universe_sha256.clone(),
            omitted_complete_case_position: omitted_position,
            omitted_source_row_index: frame[omitted_position],
            retained_observations: frame.len() - 1,
            covariance_denominator: SemCovarianceDenominatorV4::MaximumLikelihoodN,
            sampling_positions_digest_method:
                CBSEM_EXACT_CASE_BOOTSTRAP_SAMPLING_POSITIONS_DIGEST_METHOD_V1.into(),
            retained_sampling_positions_sha256:
                cbsem_exact_case_bootstrap_sampling_positions_digest_v1(
                    frame.len(),
                    &retained_positions,
                ),
            sample_indices_digest_method: CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1.into(),
            retained_sample_indices_sha256: cbsem_exact_case_bootstrap_index_digest_v1(
                &original.source_dataset_fingerprint,
                original.source_row_count,
                &retained_rows,
            ),
            free_parameters: vec![
                CbsemExactCaseBootstrapParameterEstimateV1 {
                    parameter_id: "loading:x1".into(),
                    estimate: first,
                },
                CbsemExactCaseBootstrapParameterEstimateV1 {
                    parameter_id: "variance:x".into(),
                    estimate: second,
                },
            ],
            iterations: 3,
            objective: 0.1,
            gradient_norm: 0.001,
        }
    }

    fn zero_null_eligibility() -> Vec<CbsemExactCaseBootstrapZeroNullEligibilityV1> {
        vec![
            CbsemExactCaseBootstrapZeroNullEligibilityV1 {
                parameter_id: "loading:x1".into(),
                status: CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Available,
            },
            CbsemExactCaseBootstrapZeroNullEligibilityV1 {
                parameter_id: "variance:x".into(),
                status: CbsemExactCaseBootstrapZeroNullEligibilityStatusV1::Unavailable {
                    reason: CbsemExactCaseBootstrapZeroNullUnavailableReasonV1::NonregularVarianceBoundary,
                },
            },
        ]
    }

    fn witness(replicate_index: u32, first: f64, second: f64) -> CbsemExactCaseBootstrapWitnessV1 {
        CbsemExactCaseBootstrapWitnessV1 {
            replicate_index,
            sampling_positions_sha256: format!("positions-{replicate_index}"),
            sample_indices_sha256: format!("rows-{replicate_index}"),
            parameter_estimates: vec![first, second],
            iterations: 2,
            objective: 0.1,
            gradient_norm: 0.01,
        }
    }

    #[test]
    fn exact_schedule_is_bit_identical_across_worker_counts() {
        let execute = |workers| {
            let frame = sampling_frame();
            let original = original_refit(&frame);
            let eligibility = zero_null_eligibility();
            let mut execution_schedule = schedule(1_000, 47);
            execution_schedule.hypothesis_test =
                Some(CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                    selected_test_tail: CbsemBootstrapTestTail::OneSidedGreater,
                    parameter_eligibility: &eligibility,
                });
            run_cbsem_exact_case_bootstrap_v1(
                &frame,
                &original,
                execution_schedule,
                workers,
                |replicate_index, positions| {
                    let value = positions.iter().sum::<usize>() as f64
                        + f64::from(replicate_index) / 1_000.0;
                    Ok(successful_refit(&original, &frame, positions, value))
                },
                || false,
                |_| {},
            )
            .unwrap()
        };
        let serial = execute(1);
        let parallel = execute(4);
        assert_eq!(serial, parallel);
        assert_eq!(serial.attempted_refits, 1_000);
        assert_eq!(serial.usable_replicates, 1_000);
        assert!(matches!(
            serial.inference,
            CbsemExactCaseBootstrapInferenceV1::Available
        ));
        assert_eq!(serial.intervals.len(), 2);
        assert!(serial.hypothesis_tests.is_some());
        assert_eq!(serial.successful_refits.len(), 1_000);
        assert!(serial.failed_refits.is_empty());
        assert_eq!(
            serial.outer_recipe_analytical_identity_sha256,
            OUTER_RECIPE_SHA256
        );
        assert_eq!(serial.base_point_result_sha256, BASE_POINT_SHA256);
        assert_eq!(serial.complete_case_sample_size, sampling_frame().len());
        assert_eq!(serial.seed, 47);
        assert_eq!(
            serial.stream_token,
            CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1
        );
        assert_eq!(
            serial.retry_policy,
            CBSEM_EXACT_CASE_BOOTSTRAP_RETRY_POLICY_V1
        );
        assert_eq!(serial.max_attempts_per_replicate, 1);
        assert_eq!(
            serial.complete_case_universe_digest_method,
            CBSEM_EXACT_CASE_BOOTSTRAP_COMPLETE_CASE_UNIVERSE_DIGEST_METHOD_V1
        );
        assert_eq!(
            serial.sample_indices_digest_method,
            CBSEM_EXACT_CASE_BOOTSTRAP_INDEX_DIGEST_METHOD_V1
        );
        assert_eq!(
            serial.sampling_positions_digest_method,
            CBSEM_EXACT_CASE_BOOTSTRAP_SCHEDULE_POSITIONS_DIGEST_METHOD_V1
        );
    }

    #[test]
    fn every_preplanned_draw_runs_once_and_failures_are_sorted_without_retry() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let calls = Arc::new((0..1_000).map(|_| AtomicUsize::new(0)).collect::<Vec<_>>());
        let saw_duplicate = Arc::new(AtomicBool::new(false));
        let callback_calls = Arc::clone(&calls);
        let callback_duplicate = Arc::clone(&saw_duplicate);
        let result = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_000, 91),
            4,
            |replicate_index, positions| {
                callback_calls[replicate_index as usize].fetch_add(1, Ordering::Relaxed);
                let unique = positions.iter().copied().collect::<BTreeSet<_>>();
                if unique.len() < positions.len() {
                    callback_duplicate.store(true, Ordering::Relaxed);
                }
                if replicate_index == 3 {
                    return Err(CbsemExactCaseBootstrapAttemptErrorV1::Failed {
                        kind: CbsemExactCaseBootstrapFailureKindV1::NonConvergence,
                        message: "optimizer iteration limit".into(),
                    });
                }
                if replicate_index == 17 {
                    return Err(CbsemExactCaseBootstrapAttemptErrorV1::Failed {
                        kind: CbsemExactCaseBootstrapFailureKindV1::MomentMatrixNotPositiveDefinite,
                        message: "sample covariance is singular".into(),
                    });
                }
                Ok(successful_refit(
                    &original,
                    &frame,
                    positions,
                    f64::from(replicate_index),
                ))
            },
            || false,
            |_| {},
        )
        .unwrap();

        assert!(calls.iter().all(|count| count.load(Ordering::Relaxed) == 1));
        assert!(saw_duplicate.load(Ordering::Relaxed));
        assert_eq!(result.usable_replicates, 998);
        assert_eq!(result.failed_replicates, 2);
        assert_eq!(
            result
                .failed_refits
                .iter()
                .map(|failure| failure.replicate_index)
                .collect::<Vec<_>>(),
            vec![3, 17]
        );
        assert!(matches!(
            result.inference,
            CbsemExactCaseBootstrapInferenceV1::Unavailable { ref reason_code, .. }
                if reason_code == "insufficient_usable_refits"
        ));
        assert!(result.intervals.is_empty());
        let positions = bootstrap_indices(
            frame.len(),
            91,
            &format!("{}:primary", CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1),
            3,
        );
        let rows = positions
            .iter()
            .map(|position| frame[*position])
            .collect::<Vec<_>>();
        assert_eq!(
            result.failed_refits[0].sampling_positions_sha256,
            cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
                91,
                3,
                frame.len(),
                &positions,
            )
        );
        assert_ne!(
            result.failed_refits[0].sampling_positions_sha256,
            cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
                CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
                92,
                3,
                frame.len(),
                &positions,
            )
        );
        assert_ne!(
            result.failed_refits[0].sampling_positions_sha256,
            cbsem_exact_case_bootstrap_sampling_positions_digest_v1(frame.len(), &positions)
        );
        assert_eq!(
            result.failed_refits[0].sample_indices_sha256,
            cbsem_exact_case_bootstrap_index_digest_v1(
                &original.source_dataset_fingerprint,
                original.source_row_count,
                &rows,
            )
        );
    }

    #[test]
    fn type7_and_sample_standard_deviation_use_successful_refit_order() {
        assert_eq!(required_usable_refits(500), 1_000);
        assert_eq!(required_usable_refits(1_000), 1_000);
        assert_eq!(required_usable_refits(1_100), 1_000);
        assert_eq!(required_usable_refits(2_000), 1_800);
        assert_eq!(required_usable_refits(10_000), 9_000);
        let parameter_ids = vec!["p".into()];
        let successful = [1.0, 2.0, 4.0, 8.0, 16.0]
            .into_iter()
            .enumerate()
            .map(
                |(replicate_index, value)| CbsemExactCaseBootstrapWitnessV1 {
                    replicate_index: replicate_index as u32,
                    sampling_positions_sha256: format!("positions-{replicate_index}"),
                    sample_indices_sha256: format!("digest-{replicate_index}"),
                    parameter_estimates: vec![value],
                    iterations: 2,
                    objective: 0.1,
                    gradient_norm: 0.01,
                },
            )
            .collect::<Vec<_>>();
        let intervals = summarize_intervals(&parameter_ids, &[5.0], &successful, 0.80).unwrap();
        let interval = &intervals[0];
        assert!((interval.bootstrap_mean - 6.2).abs() < 1e-12);
        assert!((interval.bias - 1.2).abs() < 1e-12);
        assert!((interval.standard_error - 6.099_180_272_790_763).abs() < 1e-12);
        assert!((interval.percentile_lower - 1.4).abs() < 1e-12);
        assert!((interval.percentile_upper - 12.8).abs() < 1e-12);
        assert_eq!(interval.usable_replicates, 5);
    }

    #[test]
    fn selected_tail_uses_null_centered_inclusive_counts_and_plus_one_denominator() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let eligibility = zero_null_eligibility();
        let successful = [-0.5, 0.0, 1.5, 3.0, 3.5]
            .into_iter()
            .enumerate()
            .map(|(index, estimate)| witness(index as u32, estimate, 2.5))
            .collect::<Vec<_>>();
        let tests = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &original,
            &successful,
            5,
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: CbsemBootstrapTestTail::OneSidedGreater,
                parameter_eligibility: &eligibility,
            },
        )
        .unwrap();
        assert_eq!(tests.null_value.to_bits(), 0.0_f64.to_bits());
        assert_eq!(tests.usable_replicates, 5);
        assert_eq!(tests.parameters.len(), 2);
        let CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
            two_sided_exceedances,
            greater_or_equal_exceedances,
            less_or_equal_exceedances,
            p_value_two_sided,
            p_value_greater,
            p_value_less,
            selected_exceedances,
            selected_p_value,
            reject_null,
            ..
        } = tests.parameters[0].outcome
        else {
            panic!("loading test must be available")
        };
        assert_eq!(two_sided_exceedances, 4);
        assert_eq!(greater_or_equal_exceedances, 2);
        assert_eq!(less_or_equal_exceedances, 4);
        assert_eq!(p_value_two_sided.to_bits(), (5.0_f64 / 6.0).to_bits());
        assert_eq!(p_value_greater.to_bits(), (3.0_f64 / 6.0).to_bits());
        assert_eq!(p_value_less.to_bits(), (5.0_f64 / 6.0).to_bits());
        assert_eq!(selected_exceedances, 2);
        assert_eq!(selected_p_value.to_bits(), (3.0_f64 / 6.0).to_bits());
        assert!(!reject_null);
        assert!(matches!(
            tests.parameters[1].outcome,
            CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::NonregularVarianceBoundary
            }
        ));
    }

    #[test]
    fn sign_reversal_exchanges_directional_tail_counts() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let eligibility = zero_null_eligibility();
        let successful = [-0.5, 0.0, 1.5, 3.0, 3.5]
            .into_iter()
            .enumerate()
            .map(|(index, estimate)| witness(index as u32, estimate, 2.5))
            .collect::<Vec<_>>();
        let forward = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &original,
            &successful,
            5,
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: CbsemBootstrapTestTail::TwoSided,
                parameter_eligibility: &eligibility,
            },
        )
        .unwrap();
        let mut reversed_original = original;
        reversed_original.free_parameters[0].estimate *= -1.0;
        let reversed = successful
            .into_iter()
            .map(|mut row| {
                row.parameter_estimates[0] *= -1.0;
                row
            })
            .collect::<Vec<_>>();
        let reversed = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &reversed_original,
            &reversed,
            5,
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: CbsemBootstrapTestTail::TwoSided,
                parameter_eligibility: &eligibility,
            },
        )
        .unwrap();
        let CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
            greater_or_equal_exceedances: forward_greater,
            less_or_equal_exceedances: forward_less,
            ..
        } = forward.parameters[0].outcome
        else {
            unreachable!()
        };
        let CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
            greater_or_equal_exceedances: reversed_greater,
            less_or_equal_exceedances: reversed_less,
            ..
        } = reversed.parameters[0].outcome
        else {
            unreachable!()
        };
        assert_eq!(forward_greater, reversed_less);
        assert_eq!(forward_less, reversed_greater);
    }

    #[test]
    fn selected_probability_equal_to_point_zero_five_rejects() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let eligibility = zero_null_eligibility();
        let successful = (0..19)
            .map(|index| witness(index, 1.5, 2.5))
            .collect::<Vec<_>>();
        let tests = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &original,
            &successful,
            19,
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: CbsemBootstrapTestTail::OneSidedGreater,
                parameter_eligibility: &eligibility,
            },
        )
        .unwrap();
        let CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
            selected_exceedances,
            selected_p_value,
            reject_null,
            ..
        } = tests.parameters[0].outcome
        else {
            unreachable!()
        };
        assert_eq!(selected_exceedances, 0);
        assert_eq!(selected_p_value.to_bits(), 0.05_f64.to_bits());
        assert!(reject_null);
    }

    #[test]
    fn failed_fit_is_absent_from_plus_one_denominator() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let eligibility = zero_null_eligibility();
        // Four successful rows from a five-attempt ledger; the omitted failed
        // fit must not enlarge R. Exactly one centered draw reaches theta-hat.
        let successful = [3.0, 1.5, 1.5, 1.5]
            .into_iter()
            .enumerate()
            .map(|(index, estimate)| witness(index as u32, estimate, 2.5))
            .collect::<Vec<_>>();
        let tests = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &original,
            &successful,
            4,
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: CbsemBootstrapTestTail::OneSidedGreater,
                parameter_eligibility: &eligibility,
            },
        )
        .unwrap();
        let CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Available {
            selected_exceedances,
            selected_p_value,
            ..
        } = tests.parameters[0].outcome
        else {
            unreachable!()
        };
        assert_eq!(tests.usable_replicates, 4);
        assert_eq!(selected_exceedances, 1);
        assert_eq!(selected_p_value.to_bits(), (2.0_f64 / 5.0).to_bits());
    }

    #[test]
    fn five_hundred_pilot_keeps_every_parameter_but_types_tests_unavailable() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let eligibility = zero_null_eligibility();
        let successful = (0..500)
            .map(|index| witness(index, 1.5, 2.5))
            .collect::<Vec<_>>();
        let tests = summarize_cbsem_exact_case_bootstrap_hypothesis_tests_v1(
            &original,
            &successful,
            required_usable_refits(500),
            CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                selected_test_tail: CbsemBootstrapTestTail::TwoSided,
                parameter_eligibility: &eligibility,
            },
        )
        .unwrap();
        assert!(matches!(
            tests.inference,
            CbsemExactCaseBootstrapHypothesisTestInferenceV1::Unavailable { ref reason_code, .. }
                if reason_code == "insufficient_usable_refits"
        ));
        assert_eq!(
            tests
                .parameters
                .iter()
                .map(|row| row.parameter_id.as_str())
                .collect::<Vec<_>>(),
            vec!["loading:x1", "variance:x"]
        );
        assert!(matches!(
            tests.parameters[0].outcome,
            CbsemExactCaseBootstrapHypothesisTestOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1::InsufficientUsableReplicates
            }
        ));
    }

    #[test]
    fn initial_five_hundred_refit_workflow_completes_without_inference() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let result = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(500, 19),
            4,
            |replicate_index, positions| {
                Ok(successful_refit(
                    &original,
                    &frame,
                    positions,
                    f64::from(replicate_index),
                ))
            },
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(result.requested_replicates, 500);
        assert_eq!(result.attempted_refits, 500);
        assert_eq!(result.usable_replicates, 500);
        assert_eq!(result.failed_replicates, 0);
        assert_eq!(result.minimum_usable_replicates, 1_000);
        assert!(matches!(
            result.inference,
            CbsemExactCaseBootstrapInferenceV1::Unavailable { ref reason_code, .. }
                if reason_code == "insufficient_usable_refits"
        ));
        assert!(result.intervals.is_empty());
        assert!(
            !serde_json::to_value(&result)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("hypothesis_tests"),
            "omitted selected-tail plan must preserve historical aggregate bytes"
        );
    }

    #[test]
    fn successful_refit_receipts_and_optimizer_outputs_fail_closed_on_tamper() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let positions = bootstrap_indices(
            frame.len(),
            13,
            &format!("{}:primary", CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1),
            2,
        );
        let source_rows = positions
            .iter()
            .map(|position| frame[*position])
            .collect::<Vec<_>>();
        let schedule_digest = cbsem_exact_case_bootstrap_schedule_positions_digest_v1(
            CBSEM_EXACT_CASE_BOOTSTRAP_STREAM_TOKEN_V1,
            13,
            2,
            frame.len(),
            &positions,
        );
        let refit_positions_digest =
            cbsem_exact_case_bootstrap_sampling_positions_digest_v1(frame.len(), &positions);
        let source_rows_digest = cbsem_exact_case_bootstrap_index_digest_v1(
            &original.source_dataset_fingerprint,
            original.source_row_count,
            &source_rows,
        );
        let parameter_ids = original
            .free_parameters
            .iter()
            .map(|parameter| parameter.parameter_id.clone())
            .collect::<Vec<_>>();
        let valid = successful_refit(&original, &frame, &positions, 1.0);
        assert!(
            witness_from_refit(
                2,
                &schedule_digest,
                &refit_positions_digest,
                &source_rows_digest,
                &parameter_ids,
                &original,
                valid.clone(),
            )
            .is_ok()
        );
        for tampered in [
            {
                let mut value = valid.clone();
                value.sampling_positions_sha256 = "00".repeat(32);
                value
            },
            {
                let mut value = valid.clone();
                value.sample_indices_sha256 = "11".repeat(32);
                value
            },
            {
                let mut value = valid.clone();
                value.iterations = 0;
                value
            },
            {
                let mut value = valid.clone();
                value.objective = -0.1;
                value
            },
            {
                let mut value = valid.clone();
                value.gradient_norm = f64::NAN;
                value
            },
        ] {
            assert!(
                witness_from_refit(
                    2,
                    &schedule_digest,
                    &refit_positions_digest,
                    &source_rows_digest,
                    &parameter_ids,
                    &original,
                    tampered,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn cancellation_and_identity_tamper_discard_the_complete_schedule() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let calls = AtomicUsize::new(0);
        let cancelled = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_000, 7),
            2,
            |_, _| {
                calls.fetch_add(1, Ordering::Relaxed);
                Ok(original.clone())
            },
            || true,
            |_| {},
        );
        assert!(matches!(
            cancelled,
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 0);

        let cancelled_refit = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_000, 7),
            2,
            |replicate_index, positions| {
                if replicate_index == 4 {
                    Err(CbsemExactCaseBootstrapAttemptErrorV1::Cancelled)
                } else {
                    Ok(successful_refit(&original, &frame, positions, 1.0))
                }
            },
            || false,
            |_| {},
        );
        assert!(matches!(
            cancelled_refit,
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));

        let tampered = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_000, 7),
            2,
            |replicate_index, positions| {
                let mut refit = successful_refit(&original, &frame, positions, 1.0);
                if replicate_index == 8 {
                    refit.free_parameters.reverse();
                }
                Ok(refit)
            },
            || false,
            |_| {},
        );
        assert!(matches!(
            tampered,
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::RefitIntegrity {
                replicate_index: 8,
                ..
            })
        ));
    }

    #[test]
    fn plan_and_original_boundaries_fail_before_workers_start() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        for requested in [499, 10_001] {
            assert!(matches!(
                run_cbsem_exact_case_bootstrap_v1(
                    &frame,
                    &original,
                    schedule(requested, 1),
                    1,
                    |_, positions| Ok(successful_refit(&original, &frame, positions, 1.0)),
                    || false,
                    |_| {},
                ),
                Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                    ResamplingError::InvalidPlan(_)
                ))
            ));
        }

        let mut wrong_digest = original.clone();
        wrong_digest.sample_indices_sha256 = "00".repeat(32);
        assert!(matches!(
            run_cbsem_exact_case_bootstrap_v1(
                &frame,
                &wrong_digest,
                schedule(1_000, 1),
                1,
                |_, _| unreachable!(),
                || false,
                |_| {},
            ),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal(_))
        ));

        let invalid_schedule = CbsemExactCaseBootstrapScheduleV1 {
            stream_token: "wrong-stream",
            ..schedule(1_000, 1)
        };
        assert!(matches!(
            run_cbsem_exact_case_bootstrap_v1(
                &frame,
                &original,
                invalid_schedule,
                1,
                |_, _| unreachable!(),
                || false,
                |_| {},
            ),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                ResamplingError::InvalidPlan(_)
            ))
        ));

        let rejects_original = |candidate: &CbsemExactCaseBootstrapRefitV1| {
            matches!(
                run_cbsem_exact_case_bootstrap_v1(
                    &frame,
                    candidate,
                    schedule(1_000, 1),
                    1,
                    |_, _| unreachable!(),
                    || false,
                    |_| {},
                ),
                Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal(_))
            )
        };
        let mut wrong_estimator = original.clone();
        wrong_estimator.estimator_method_version = "cbsem_ml_exact_parameter_table_v4".into();
        assert!(rejects_original(&wrong_estimator));
        let mut wrong_universe = original.clone();
        wrong_universe.complete_case_universe_sha256 = "00".repeat(32);
        assert!(rejects_original(&wrong_universe));
        let mut zero_iterations = original.clone();
        zero_iterations.iterations = 0;
        assert!(rejects_original(&zero_iterations));
        let mut negative_objective = original.clone();
        negative_objective.objective = -0.1;
        assert!(rejects_original(&negative_objective));
        let mut invalid_gradient = original;
        invalid_gradient.gradient_norm = f64::NAN;
        assert!(rejects_original(&invalid_gradient));
    }

    #[test]
    fn studentized_scheduler_executes_each_draw_once_and_preserves_base_bytes_and_worker_order() {
        let execute = |workers| {
            let frame = sampling_frame();
            let original = original_refit(&frame);
            let eligibility = zero_null_eligibility();
            let mut execution_schedule = schedule(1_000, 73);
            execution_schedule.hypothesis_test =
                Some(CbsemExactCaseBootstrapHypothesisTestPlanV1 {
                    selected_test_tail: CbsemBootstrapTestTail::OneSidedLess,
                    parameter_eligibility: &eligibility,
                });
            let point = with_standard_errors(
                original.clone(),
                available_standard_errors(&["loading:x1", "variance:x"], &[0.5, 0.75]),
            );
            let calls = Arc::new(AtomicUsize::new(0));
            let callback_calls = Arc::clone(&calls);
            let result = run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
                &frame,
                &point,
                execution_schedule,
                workers,
                |replicate_index, positions| {
                    callback_calls.fetch_add(1, Ordering::Relaxed);
                    let value = positions.iter().sum::<usize>() as f64
                        + f64::from(replicate_index) / 1_000.0;
                    Ok(with_standard_errors(
                        successful_refit(&original, &frame, positions, value),
                        available_standard_errors(&["loading:x1", "variance:x"], &[1.0, 2.0]),
                    ))
                },
                || false,
                |_| {},
            )
            .unwrap();
            assert_eq!(calls.load(Ordering::Relaxed), 1_000);
            result
        };

        let serial = execute(1);
        let parallel = execute(4);
        assert_eq!(serial, parallel);
        assert_eq!(serial.studentized.studentized_usable_replicates, 1_000);
        assert!(matches!(
            serial.studentized.inference,
            CbsemExactCaseBootstrapStudentizedInferenceV1::Available
        ));
        assert_eq!(serial.studentized.refit_standard_errors.len(), 1_000);

        let frame = sampling_frame();
        let original = original_refit(&frame);
        let eligibility = zero_null_eligibility();
        let mut execution_schedule = schedule(1_000, 73);
        execution_schedule.hypothesis_test = Some(CbsemExactCaseBootstrapHypothesisTestPlanV1 {
            selected_test_tail: CbsemBootstrapTestTail::OneSidedLess,
            parameter_eligibility: &eligibility,
        });
        let legacy = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            execution_schedule,
            1,
            |replicate_index, positions| {
                let value =
                    positions.iter().sum::<usize>() as f64 + f64::from(replicate_index) / 1_000.0;
                Ok(successful_refit(&original, &frame, positions, value))
            },
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(serial.base, legacy);
    }

    #[test]
    fn studentized_interval_uses_outer_se_pivots_and_reversed_type7_quantiles() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let mut base = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(500, 19),
            1,
            |_, positions| Ok(successful_refit(&original, &frame, positions, 1.0)),
            || false,
            |_| {},
        )
        .unwrap();
        base.successful_refits = vec![
            witness(0, -0.5, 0.5),
            witness(1, 0.5, 1.5),
            witness(2, 2.5, 3.5),
            witness(3, 4.5, 5.5),
        ];
        base.minimum_usable_replicates = 4;
        let receipts = (0..4)
            .map(|_| available_standard_errors(&["loading:x1", "variance:x"], &[1.0, 1.0]))
            .collect();
        let point_standard_errors =
            available_standard_errors(&["loading:x1", "variance:x"], &[2.0, 1.0]);
        let sidecar = recompute_cbsem_exact_case_bootstrap_studentized_sidecar_v1(
            &original,
            &point_standard_errors,
            &base,
            receipts,
        )
        .unwrap();

        let CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Available {
            lower_pivot_quantile,
            upper_pivot_quantile,
            interval_lower,
            interval_upper,
            usable_replicates,
            ..
        } = sidecar.intervals[0].outcome
        else {
            panic!("fixed hand ledger must yield an available interval");
        };
        assert!((lower_pivot_quantile - -1.925).abs() < 1.0e-12);
        assert!((upper_pivot_quantile - 2.85).abs() < 1.0e-12);
        assert!((interval_lower - -4.2).abs() < 1.0e-12);
        assert!((interval_upper - 5.35).abs() < 1.0e-12);
        assert_eq!(usable_replicates, 4);
    }

    #[test]
    fn unavailable_refit_standard_errors_preserve_point_success_and_b500_is_typed_unavailable() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let point = with_standard_errors(
            original.clone(),
            available_standard_errors(&["loading:x1", "variance:x"], &[0.5, 0.75]),
        );
        let result = run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
            &frame,
            &point,
            schedule(500, 31),
            3,
            |replicate_index, positions| {
                let receipt = if replicate_index % 2 == 0 {
                    available_standard_errors(&["loading:x1", "variance:x"], &[1.0, 2.0])
                } else {
                    unavailable_standard_errors()
                };
                Ok(with_standard_errors(
                    successful_refit(
                        &original,
                        &frame,
                        positions,
                        f64::from(replicate_index) + 1.0,
                    ),
                    receipt,
                ))
            },
            || false,
            |_| {},
        )
        .unwrap();

        assert_eq!(result.base.usable_replicates, 500);
        assert_eq!(result.base.successful_refits.len(), 500);
        assert_eq!(result.studentized.refit_standard_errors.len(), 500);
        assert_eq!(result.studentized.studentized_usable_replicates, 250);
        assert!(matches!(
            result.studentized.inference,
            CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable {
                reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates,
                ..
            }
        ));
        assert!(result.studentized.intervals.iter().all(|row| matches!(
            row.outcome,
            CbsemExactCaseBootstrapStudentizedParameterIntervalOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::InsufficientStudentizedUsableReplicates
            }
        )));

        let available_receipts = (0..result.base.successful_refits.len())
            .map(|_| available_standard_errors(&["loading:x1", "variance:x"], &[1.0, 2.0]))
            .collect();
        let unavailable_point = recompute_cbsem_exact_case_bootstrap_studentized_sidecar_v1(
            &original,
            &unavailable_standard_errors(),
            &result.base,
            available_receipts,
        )
        .unwrap();
        assert!(matches!(
            unavailable_point.inference,
            CbsemExactCaseBootstrapStudentizedInferenceV1::Unavailable {
                reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1::PointStandardErrorsUnavailable,
                ..
            }
        ));
    }

    #[test]
    fn studentized_receipts_fail_closed_and_cancellation_prevents_refits() {
        let ids = ["loading:x1", "variance:x"];
        let mut wrong_order = available_standard_errors(&ids, &[1.0, 2.0]);
        let CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available { parameters, .. } =
            &mut wrong_order.outcome
        else {
            unreachable!()
        };
        parameters.reverse();
        assert!(validate_analytic_standard_errors(&ids, &wrong_order).is_err());

        let mut zero = available_standard_errors(&ids, &[1.0, 2.0]);
        let CbsemExactCaseBootstrapRefitStandardErrorOutcomeV1::Available { parameters, .. } =
            &mut zero.outcome
        else {
            unreachable!()
        };
        parameters[0].standard_error = 0.0;
        assert!(validate_analytic_standard_errors(&ids, &zero).is_err());

        let frame = sampling_frame();
        let original = original_refit(&frame);
        let point = with_standard_errors(original, available_standard_errors(&ids, &[1.0, 2.0]));
        let calls = AtomicUsize::new(0);
        let cancelled = run_cbsem_exact_case_bootstrap_with_analytic_studentized_intervals_v1(
            &frame,
            &point,
            schedule(500, 5),
            2,
            |_, _| {
                calls.fetch_add(1, Ordering::Relaxed);
                unreachable!()
            },
            || true,
            |_| {},
        );
        assert!(matches!(
            cancelled,
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn bca_delete_one_schedule_is_exact_once_ordered_and_worker_invariant() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let base = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_000, 211),
            2,
            |replicate_index, positions| {
                let delta = f64::from(replicate_index % 5) - 2.0;
                let mut refit = successful_refit(
                    &original,
                    &frame,
                    positions,
                    original.free_parameters[0].estimate + 0.1 * delta,
                );
                refit.free_parameters[1].estimate =
                    original.free_parameters[1].estimate + 0.2 * delta;
                Ok(refit)
            },
            || false,
            |_| {},
        )
        .unwrap();
        let execute = |workers| {
            let calls = AtomicUsize::new(0);
            let result = run_cbsem_exact_case_bootstrap_bca_v1(
                &frame,
                &original,
                &base,
                workers,
                |omitted_position| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    let delta = omitted_position as f64 - (frame.len() - 1) as f64 / 2.0;
                    Ok(delete_one_refit(
                        &original,
                        &frame,
                        omitted_position,
                        original.free_parameters[0].estimate + 0.01 * delta,
                        original.free_parameters[1].estimate + 0.02 * delta,
                    ))
                },
                || false,
                |_| {},
            )
            .unwrap();
            assert_eq!(calls.load(Ordering::Relaxed), frame.len());
            result
        };
        let serial = execute(1);
        let parallel = execute(4);
        assert_eq!(serial, parallel);
        let recomputed = recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1(
            &original,
            &base,
            serial.successful_delete_one_refits.clone(),
            serial.failed_delete_one_refits.clone(),
        )
        .unwrap();
        assert_eq!(recomputed, serial);
        assert_eq!(
            serde_json::to_vec(&recomputed).unwrap(),
            serde_json::to_vec(&serial).unwrap(),
            "archive-pure BCa recomputation must be byte-exact"
        );

        let mut tampered_evidence = serial.successful_delete_one_refits.clone();
        tampered_evidence[0].retained_sample_indices_sha256 = "0".repeat(64);
        assert!(matches!(
            recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1(
                &original,
                &base,
                tampered_evidence,
                serial.failed_delete_one_refits.clone(),
            ),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(_))
        ));
        assert!(matches!(
            serial.inference,
            CbsemExactCaseBootstrapBcaInferenceV1::Available
        ));
        assert!(serial.failed_delete_one_refits.is_empty());
        assert_eq!(serial.successful_delete_one_refits.len(), frame.len());
        assert!(
            serial
                .successful_delete_one_refits
                .iter()
                .enumerate()
                .all(|(position, witness)| witness.omitted_complete_case_position == position)
        );
        assert!(serial.intervals.iter().all(|row| matches!(
            row.outcome,
            CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Available { .. }
        )));
    }

    #[test]
    fn bca_n10_failures_are_complete_typed_and_block_every_interval() {
        let frame = (2..12).collect::<Vec<_>>();
        let original = original_refit(&frame);
        let base = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_000, 307),
            1,
            |replicate_index, positions| {
                let delta = f64::from(replicate_index % 5) - 2.0;
                let mut refit = successful_refit(
                    &original,
                    &frame,
                    positions,
                    original.free_parameters[0].estimate + delta * 0.1,
                );
                refit.free_parameters[1].estimate =
                    original.free_parameters[1].estimate + delta * 0.2;
                Ok(refit)
            },
            || false,
            |_| {},
        )
        .unwrap();
        let calls = AtomicUsize::new(0);
        let result = run_cbsem_exact_case_bootstrap_bca_v1(
            &frame,
            &original,
            &base,
            3,
            |_| {
                calls.fetch_add(1, Ordering::Relaxed);
                Err(CbsemExactCaseBootstrapAttemptErrorV1::Failed {
                    kind: CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
                    message: "N-1=9 is below the exact-refit minimum".into(),
                })
            },
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 10);
        assert!(result.successful_delete_one_refits.is_empty());
        assert_eq!(result.failed_delete_one_refits.len(), 10);
        assert!(
            result
                .failed_delete_one_refits
                .iter()
                .enumerate()
                .all(|(position, failure)| failure.omitted_complete_case_position == position)
        );
        assert!(matches!(
            result.inference,
            CbsemExactCaseBootstrapBcaInferenceV1::Unavailable {
                reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1::IncompleteDeleteOneLedger,
                ..
            }
        ));
        assert!(result.intervals.iter().all(|row| matches!(
            row.outcome,
            CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1::IncompleteDeleteOneLedger
            }
        )));

        let mut duplicate_index = base.clone();
        duplicate_index.successful_refits[1].replicate_index = 0;
        assert!(matches!(
            run_cbsem_exact_case_bootstrap_bca_v1(
                &frame,
                &original,
                &duplicate_index,
                1,
                |_| unreachable!(),
                || false,
                |_| {}
            ),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(_))
        ));

        let mut tampered_original = original.clone();
        tampered_original.method_version = "coordinated-tamper".into();
        let tamper_calls = AtomicUsize::new(0);
        assert!(matches!(
            run_cbsem_exact_case_bootstrap_bca_v1(
                &frame,
                &tampered_original,
                &base,
                1,
                |_| {
                    tamper_calls.fetch_add(1, Ordering::Relaxed);
                    unreachable!()
                },
                || false,
                |_| {}
            ),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidOriginal(_))
        ));
        assert_eq!(tamper_calls.load(Ordering::Relaxed), 0);

        let cancellation_calls = AtomicUsize::new(0);
        assert!(matches!(
            run_cbsem_exact_case_bootstrap_bca_v1(
                &frame,
                &original,
                &base,
                2,
                |_| {
                    cancellation_calls.fetch_add(1, Ordering::Relaxed);
                    unreachable!()
                },
                || true,
                |_| {}
            ),
            Err(CbsemExactCaseBootstrapSchedulerErrorV1::Resampling(
                ResamplingError::Cancelled
            ))
        ));
        assert_eq!(cancellation_calls.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn bca_base_integrity_rejects_witness_digest_and_interval_tamper_before_delete_one() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let base = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(1_001, 389),
            2,
            |replicate_index, positions| {
                if replicate_index == 7 {
                    return Err(CbsemExactCaseBootstrapAttemptErrorV1::Failed {
                        kind: CbsemExactCaseBootstrapFailureKindV1::NumericalFailure,
                        message: "typed fixture failure".into(),
                    });
                }
                let delta = f64::from(replicate_index % 5) - 2.0;
                let mut refit = successful_refit(
                    &original,
                    &frame,
                    positions,
                    original.free_parameters[0].estimate + delta * 0.1,
                );
                refit.free_parameters[1].estimate =
                    original.free_parameters[1].estimate + delta * 0.2;
                Ok(refit)
            },
            || false,
            |_| {},
        )
        .unwrap();
        assert!(matches!(
            &base.inference,
            CbsemExactCaseBootstrapInferenceV1::Available
        ));
        assert_eq!(base.successful_refits.len(), 1_000);
        assert_eq!(base.failed_refits.len(), 1);

        let corrupt_digest = |digest: &mut String| {
            let replacement = if digest.starts_with('0') { "1" } else { "0" };
            digest.replace_range(0..1, replacement);
        };
        let mut cases = Vec::new();

        let mut finite_witness_estimate = base.clone();
        finite_witness_estimate.successful_refits[0].parameter_estimates[0] += 0.125;
        cases.push(("finite witness estimate", finite_witness_estimate));

        let mut successful_schedule_digest = base.clone();
        corrupt_digest(
            &mut successful_schedule_digest.successful_refits[0].sampling_positions_sha256,
        );
        cases.push(("successful schedule digest", successful_schedule_digest));

        let mut successful_source_digest = base.clone();
        corrupt_digest(&mut successful_source_digest.successful_refits[0].sample_indices_sha256);
        cases.push(("successful source digest", successful_source_digest));

        let mut failed_schedule_digest = base.clone();
        corrupt_digest(&mut failed_schedule_digest.failed_refits[0].sampling_positions_sha256);
        cases.push(("failed schedule digest", failed_schedule_digest));

        let mut failed_source_digest = base.clone();
        corrupt_digest(&mut failed_source_digest.failed_refits[0].sample_indices_sha256);
        cases.push(("failed source digest", failed_source_digest));

        let mut interval_value = base.clone();
        interval_value.intervals[0].bootstrap_mean += 0.125;
        cases.push(("interval value", interval_value));

        let mut interval_original = base.clone();
        interval_original.intervals[0].original += 0.125;
        cases.push(("interval original", interval_original));

        let mut interval_order = base.clone();
        interval_order.intervals.swap(0, 1);
        cases.push(("interval order", interval_order));

        let mut unavailable_with_injected_interval = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(500, 397),
            2,
            |replicate_index, positions| {
                let delta = f64::from(replicate_index % 5) - 2.0;
                Ok(successful_refit(
                    &original,
                    &frame,
                    positions,
                    original.free_parameters[0].estimate + delta * 0.1,
                ))
            },
            || false,
            |_| {},
        )
        .unwrap();
        assert!(matches!(
            &unavailable_with_injected_interval.inference,
            CbsemExactCaseBootstrapInferenceV1::Unavailable { .. }
        ));
        unavailable_with_injected_interval
            .intervals
            .push(base.intervals[0].clone());
        cases.push((
            "unavailable base injected interval",
            unavailable_with_injected_interval,
        ));

        for (case, tampered) in cases {
            let calls = AtomicUsize::new(0);
            let result = run_cbsem_exact_case_bootstrap_bca_v1(
                &frame,
                &original,
                &tampered,
                2,
                |_| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    unreachable!("invalid base must fail before delete-one callbacks")
                },
                || false,
                |_| {},
            );
            assert!(
                matches!(
                    &result,
                    Err(CbsemExactCaseBootstrapSchedulerErrorV1::InvalidSummary(_))
                ),
                "case {case} unexpectedly returned {result:?}"
            );
            assert_eq!(calls.load(Ordering::Relaxed), 0, "case {case}");
        }
    }

    #[test]
    fn bca_b500_preserves_complete_delete_one_evidence_but_is_typed_unavailable() {
        let frame = sampling_frame();
        let original = original_refit(&frame);
        let base = run_cbsem_exact_case_bootstrap_v1(
            &frame,
            &original,
            schedule(500, 401),
            2,
            |replicate_index, positions| {
                let delta = f64::from(replicate_index % 5) - 2.0;
                let mut refit = successful_refit(
                    &original,
                    &frame,
                    positions,
                    original.free_parameters[0].estimate + delta * 0.1,
                );
                refit.free_parameters[1].estimate =
                    original.free_parameters[1].estimate + delta * 0.2;
                Ok(refit)
            },
            || false,
            |_| {},
        )
        .unwrap();
        let result = run_cbsem_exact_case_bootstrap_bca_v1(
            &frame,
            &original,
            &base,
            2,
            |omitted_position| {
                Ok(delete_one_refit(
                    &original,
                    &frame,
                    omitted_position,
                    1.0 + omitted_position as f64 * 0.01,
                    2.0 + omitted_position as f64 * 0.02,
                ))
            },
            || false,
            |_| {},
        )
        .unwrap();
        let recomputed = recompute_cbsem_exact_case_bootstrap_bca_sidecar_v1(
            &original,
            &base,
            result.successful_delete_one_refits.clone(),
            result.failed_delete_one_refits.clone(),
        )
        .unwrap();
        assert_eq!(recomputed, result);
        assert_eq!(
            serde_json::to_vec(&recomputed).unwrap(),
            serde_json::to_vec(&result).unwrap(),
            "B=500 unavailable BCa recomputation must be byte-exact"
        );
        assert_eq!(result.successful_delete_one_refits.len(), frame.len());
        assert!(result.failed_delete_one_refits.is_empty());
        assert!(matches!(
            result.inference,
            CbsemExactCaseBootstrapBcaInferenceV1::Unavailable {
                reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1::BaseInferenceUnavailable,
                ..
            }
        ));
    }

    #[test]
    fn bca_midrank_ties_have_no_clamp_and_numeric_singularities_are_typed() {
        let available =
            bca_parameter_interval(0.0, &[-1.0, -0.0, 0.0, 1.0], &[-1.0, -0.25, 0.5, 1.5], 0.95);
        let CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Available {
            bias_correction, ..
        } = available
        else {
            panic!("midrank signed-zero ties must yield an interior probability");
        };
        assert_eq!(bias_correction.to_bits(), 0.0_f64.to_bits());
        assert!(matches!(
            bca_parameter_interval(0.0, &[1.0, 2.0, 3.0], &[-1.0, 0.0, 1.0], 0.95),
            CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable {
                reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1::BiasCorrectionProbabilityAtBoundary
            }
        ));
        assert!(matches!(
            bca_parameter_interval(0.0, &[-1.0, 0.0, 1.0], &[2.0, 2.0, 2.0], 0.95),
            CbsemExactCaseBootstrapBcaParameterIntervalOutcomeV1::Unavailable {
                reason:
                    CbsemExactCaseBootstrapBcaUnavailableReasonV1::DegenerateJackknifeAcceleration
            }
        ));
        let normal = Normal::standard();
        let z = normal.inverse_cdf(0.025);
        assert!(matches!(
            bca_adjusted_probability(&normal, 0.0, 1.0 / z, 0.025),
            Err(CbsemExactCaseBootstrapBcaUnavailableReasonV1::SingularAccelerationAdjustment)
        ));
    }

    #[test]
    fn bca_v2_neumaier_kernel_recovers_ordered_mean_square_and_cube_residuals() {
        // These are exact binary64 input vectors whose naïve left-to-right
        // sums lose the unit terms. They pin the V2 kernel used independently
        // for the jackknife mean, centered squares, and centered cubes.
        assert_eq!(
            neumaier_sum([1.0e16, 1.0, -1.0e16]).to_bits(),
            1.0_f64.to_bits()
        );
        assert_eq!(
            neumaier_sum([1.0e16, 1.0, 1.0]).to_bits(),
            10_000_000_000_000_002.0_f64.to_bits()
        );
        assert_eq!(
            neumaier_sum([-1.0e16, -1.0, 1.0e16]).to_bits(),
            (-1.0_f64).to_bits()
        );
    }

    #[test]
    fn bca_v2_libm_erfc_cdf_matches_high_precision_rejected_pilot_vectors() {
        let normal = Normal::standard();
        let bias_correction = -0.005_013_277_548_926_656;
        let acceleration = -0.021_957_786_021_311_34;
        let vectors = [
            (0.025, 0.019_766_500_689_861_716),
            (0.975, 0.969_221_448_476_489_7),
        ];
        for (nominal, high_precision_reference) in vectors {
            let repaired =
                bca_adjusted_probability(&normal, bias_correction, acceleration, nominal)
                    .expect("rejected-pilot vector must be numerically available");
            assert!(
                (repaired - high_precision_reference).abs() <= 2.0e-16,
                "libm erfc CDF differs: nominal={nominal}, actual={repaired:.17e}, reference={high_precision_reference:.17e}"
            );

            let z = normal.inverse_cdf(nominal);
            let denominator = 1.0 - acceleration * (bias_correction + z);
            let argument = bias_correction + (bias_correction + z) / denominator;
            let historical_statrs = normal.cdf(argument);
            assert!(
                (historical_statrs - high_precision_reference).abs() > 5.0e-13,
                "historical statrs CDF unexpectedly ceased reproducing the rejected evidence"
            );
        }
    }
}
