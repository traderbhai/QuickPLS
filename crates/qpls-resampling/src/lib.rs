use faer::{Mat, prelude::*};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ContinuousCDF, Normal};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

use arrow::{
    array::{Array, Float64Array, Int64Array, UInt32Array},
    compute::take,
    record_batch::RecordBatch,
};
use qpls_core::{
    AnalysisMethod, AnalysisRecipe, MethodConfig, RegressionBootstrapAlgorithm,
    RegressionBootstrapInterval, RegressionModelConfig, ValidatedExecutionRecipe,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EffectEstimate, EstimationError, OuterEstimate, PathEstimate, PlsResult,
    ProcessBootstrapAnalysis, ProcessBootstrapEstimand, ProcessBootstrapFailedReplicate,
    ProcessBootstrapValidationWitness, ProcessBootstrapWitnessBootstrapRow,
    ProcessBootstrapWitnessJackknifeRow, REGRESSION_LOGISTIC_METHOD_VERSION,
    REGRESSION_OLS_METHOD_VERSION, REGRESSION_PROCESS_METHOD_VERSION, RegressionBootstrapAnalysis,
    RegressionBootstrapBcaInterval, RegressionBootstrapCoefficient,
    RegressionBootstrapFailedJackknife, RegressionBootstrapFailedReplicate,
    RegressionBootstrapOddsRatio, RegressionBootstrapTest, RegressionBootstrapValidationWitness,
    RegressionBootstrapWitnessBootstrapRow, RegressionBootstrapWitnessJackknifeRow,
    estimate_pls_validated_with_control, estimate_regression_case_resample_validated_with_control,
    process_bootstrap_estimands, process_bootstrap_estimands_at_reference,
};

pub const RESAMPLING_METHOD_VERSION_V1: &str = "indexed_resampling_v1";
pub const RESAMPLING_METHOD_VERSION_V2: &str = "indexed_resampling_v2";
pub const RESAMPLING_METHOD_VERSION_V3: &str = "indexed_resampling_v3";
pub const RESAMPLING_METHOD_VERSION: &str = "indexed_resampling_v4";
pub const JACKKNIFE_METHOD_VERSION: &str = "indexed_jackknife_v1";
pub const PERMUTATION_METHOD_VERSION: &str = "freedman_lane_permutation_v1";
pub const STUDENTIZED_METHOD_VERSION: &str = "nested_studentized_v1";
pub const REGRESSION_BOOTSTRAP_METHOD_VERSION: &str = "regression_bootstrap_v1";
pub const REGRESSION_BOOTSTRAP_ALGORITHM: &str = "indexed_case_resampling_v1";
pub const REGRESSION_BOOTSTRAP_STREAM_TOKEN: &str = "quickpls_indexed_resampling_v1";
pub const REGRESSION_BOOTSTRAP_INTERVAL_POLICY: &str = "percentile_primary_bca_conditional_v1";
pub const REGRESSION_BOOTSTRAP_TEST_REFERENCE: &str = "standard_normal_bootstrap_ratio_v1";
pub const REGRESSION_BOOTSTRAP_TEST_TOLERANCE_POLICY: &str = "64eps_max_1_original_replicates_v1";
pub const REGRESSION_BOOTSTRAP_VALIDATION_WITNESS_VERSION: &str =
    "regression_bootstrap_validation_witness_v1";
pub const REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION: f64 = 0.90;
pub const PROCESS_BOOTSTRAP_METHOD_VERSION: &str = "regression_process_bootstrap_v1";
pub const PROCESS_BOOTSTRAP_ALGORITHM: &str = "indexed_case_resampling_v1";
pub const PROCESS_BOOTSTRAP_STREAM_TOKEN: &str = "process_indexed_case_stream_v1";
pub const PROCESS_BOOTSTRAP_INTERVAL_POLICY: &str = "percentile_primary_bca_conditional_v1";
pub const PROCESS_BOOTSTRAP_TEST_REFERENCE: &str = "standard_normal_bootstrap_ratio_v1";
pub const PROCESS_BOOTSTRAP_VALIDATION_WITNESS_VERSION: &str =
    "regression_process_bootstrap_validation_witness_v1";
const SEED_DOMAIN: &[u8] = b"QuickPLS indexed resampling v1\0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapPlan {
    pub replicates: u32,
    pub master_seed: u64,
    pub operation: String,
}

impl BootstrapPlan {
    pub fn validate(&self, case_count: usize) -> Result<(), ResamplingError> {
        if self.replicates == 0 {
            return Err(ResamplingError::InvalidPlan(
                "replicates must be greater than zero".into(),
            ));
        }
        if self.replicates > 10_000 {
            return Err(ResamplingError::InvalidPlan(
                "replicates cannot exceed 10000".into(),
            ));
        }
        if case_count < 2 {
            return Err(ResamplingError::InvalidPlan(
                "at least two cases are required".into(),
            ));
        }
        if self.operation.trim().is_empty() {
            return Err(ResamplingError::InvalidPlan(
                "operation identifier cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PermutationPlan {
    pub permutations: u32,
    pub master_seed: u64,
    pub operation: String,
}

impl PermutationPlan {
    pub fn validate(&self, case_count: usize) -> Result<(), ResamplingError> {
        if self.permutations < 99 || self.permutations > 10_000 {
            return Err(ResamplingError::InvalidPlan(
                "permutations must be between 99 and 10000".into(),
            ));
        }
        if case_count < 4 {
            return Err(ResamplingError::InvalidPlan(
                "at least four cases are required".into(),
            ));
        }
        if self.operation.trim().is_empty() {
            return Err(ResamplingError::InvalidPlan(
                "operation identifier cannot be empty".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResamplingPhase {
    Bootstrap,
    Jackknife,
    Permutation,
    StudentizedInner,
}

impl ResamplingPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Jackknife => "jackknife",
            Self::Permutation => "permutation",
            Self::StudentizedInner => "studentized_inner",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResamplingProgress {
    pub phase: ResamplingPhase,
    pub completed_replicates: u32,
    pub total_replicates: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ReplicateOutcome<T> {
    Success { value: T },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapRun<T> {
    pub method_version: String,
    pub plan: BootstrapPlan,
    /// Strict replicate-index order. Vector position is the replicate index.
    pub outcomes: Vec<ReplicateOutcome<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PermutationRun<T> {
    pub method_version: String,
    pub plan: PermutationPlan,
    /// Strict permutation-index order. Vector position is the permutation index.
    pub outcomes: Vec<ReplicateOutcome<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JackknifeRun<T> {
    pub method_version: String,
    pub case_count: usize,
    pub operation: String,
    /// Strict omitted-case order. Vector position is the omitted case index.
    pub outcomes: Vec<ReplicateOutcome<T>>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ResamplingError {
    #[error("invalid resampling plan: {0}")]
    InvalidPlan(String),
    #[error("resampling was cancelled")]
    Cancelled,
    #[error("cannot create resampling worker pool: {0}")]
    WorkerPool(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsBootstrapEstimate {
    pub replicate_index: u32,
    pub iterations: u32,
    pub used_observations: usize,
    pub omitted_observations: usize,
    pub outer_estimates: Vec<OuterEstimate>,
    pub paths: Vec<PathEstimate>,
    pub effects: Vec<EffectEstimate>,
    pub r_squared: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub studentized_standard_errors: Option<std::collections::BTreeMap<String, f64>>,
    #[serde(default)]
    pub studentized_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PercentileInference {
    pub confidence_level: f64,
    pub parameters: Vec<BootstrapParameterInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BootstrapParameterInference {
    pub parameter: String,
    pub original: f64,
    pub bootstrap_mean: f64,
    pub bias: f64,
    pub standard_error: f64,
    pub lower: f64,
    pub upper: f64,
    pub usable_replicates: u32,
    /// Large-sample normal-reference statistic using the bootstrap SE.
    #[serde(default)]
    pub t_statistic: Option<f64>,
    /// Two-sided standard-normal reference probability for `t_statistic`.
    #[serde(default)]
    pub p_value_two_sided: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsBootstrapResult {
    pub method_version: String,
    pub plan: BootstrapPlan,
    pub usable_replicates: u32,
    pub failed_replicates: Vec<FailedReplicate>,
    pub percentile: PercentileInference,
    #[serde(default)]
    pub bca: Option<BcaInference>,
    #[serde(default)]
    pub studentized: Option<StudentizedInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentizedInference {
    pub method_version: String,
    pub confidence_level: f64,
    pub inner_replicates: u32,
    pub minimum_usable_fraction: f64,
    pub stream_domain: String,
    #[serde(default)]
    pub failure: Option<StudentizedFailure>,
    pub parameters: Vec<StudentizedParameterInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentizedFailure {
    pub reason_code: String,
    pub first_primary_replicate: u32,
    pub failed_primary_replicates: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StudentizedParameterInference {
    pub parameter: String,
    pub original: f64,
    pub outer_standard_error: f64,
    pub outer_scale: f64,
    pub usable_primary_replicates: u32,
    pub lower_pivot: Option<f64>,
    pub upper_pivot: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BcaInference {
    pub confidence_level: f64,
    pub jackknife_case_count: usize,
    pub parameters: Vec<BcaParameterInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BcaParameterInference {
    pub parameter: String,
    pub bias_correction: Option<f64>,
    pub acceleration: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailedReplicate {
    pub replicate_index: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlsJackknifeEstimate {
    pub omitted_case: usize,
    pub parameters: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPermutationResult {
    pub method_version: String,
    pub plan: PermutationPlan,
    pub parameters: Vec<PermutationParameterInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PermutationParameterInference {
    pub parameter: String,
    pub original: f64,
    pub exceedances: u32,
    pub p_value_two_sided: f64,
    pub permutations: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BcaIntervalValues {
    pub bias_correction: f64,
    pub acceleration: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StudentizedIntervalValues {
    pub lower_pivot: f64,
    pub upper_pivot: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Error)]
pub enum PlsBootstrapError {
    #[error("PLS bootstrap requires raw observations")]
    RawDataRequired,
    #[error("PLS bootstrap requires method pls_pm")]
    InvalidMethod,
    #[error("PLS bootstrap requires bootstrap_samples greater than zero")]
    MissingReplicates,
    #[error(
        "studentized bootstrap requires at least 999 primary replicates and an odd inner count from 99 to 999"
    )]
    InvalidStudentizedPlan,
    #[error("bootstrap produced {usable} usable replicates; at least {required} are required")]
    InsufficientUsableReplicates { usable: usize, required: usize },
    #[error("bootstrap result is inconsistent with the original model: {0}")]
    InconsistentResult(String),
    #[error("base PLS estimation failed: {0}")]
    BaseEstimation(#[from] EstimationError),
    #[error("PLS jackknife required for BCa inference failed: {0}")]
    Jackknife(String),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, Error)]
pub enum RegressionBootstrapError {
    #[error("regression bootstrap requires raw observations")]
    RawDataRequired,
    #[error("regression bootstrap requires a typed OLS or binary logistic case-bootstrap recipe")]
    InvalidMethod,
    #[error("regression bootstrap result is inconsistent with the base estimate: {0}")]
    InconsistentResult(String),
    #[error("regression bootstrap summary inputs are invalid: {0}")]
    InvalidSummary(String),
    #[error(
        "regression bootstrap produced {usable} usable replicates; at least {required} are required"
    )]
    InsufficientUsableReplicates { usable: usize, required: usize },
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, Error)]
pub enum PlsJackknifeError {
    #[error("PLS jackknife requires raw observations")]
    RawDataRequired,
    #[error("PLS jackknife requires method pls_pm")]
    InvalidMethod,
    #[error("PLS jackknife requires at least four complete cases; found {0}")]
    InsufficientCases(usize),
    #[error("jackknife result is inconsistent with the original model: {0}")]
    InconsistentResult(String),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, Error)]
pub enum PlsPermutationError {
    #[error("PLS permutation requires raw observations")]
    RawDataRequired,
    #[error("PLS permutation requires method pls_pm")]
    InvalidMethod,
    #[error("PLS permutation requires permutation_samples between 99 and 10000")]
    InvalidPermutationCount,
    #[error("PLS permutation result is inconsistent with the original model: {0}")]
    InconsistentResult(String),
    #[error("PLS permutation regression failed: {0}")]
    Regression(String),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

pub fn bootstrap_indices(
    case_count: usize,
    master_seed: u64,
    operation: &str,
    replicate_index: u32,
) -> Vec<usize> {
    let mut rng = ChaCha20Rng::from_seed(derive_seed(master_seed, operation, replicate_index));
    (0..case_count)
        .map(|_| rng.random_range(0..case_count))
        .collect()
}

pub fn permutation_indices(
    case_count: usize,
    master_seed: u64,
    operation: &str,
    permutation_index: u32,
) -> Vec<usize> {
    let mut indices = (0..case_count).collect::<Vec<_>>();
    let mut rng = ChaCha20Rng::from_seed(derive_seed(master_seed, operation, permutation_index));
    for upper in (1..case_count).rev() {
        indices.swap(upper, rng.random_range(0..=upper));
    }
    indices
}

pub fn run_bootstrap<T, E>(
    case_count: usize,
    plan: &BootstrapPlan,
    workers: usize,
    estimate: impl Fn(u32, &[usize]) -> Result<T, E> + Sync,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<BootstrapRun<T>, ResamplingError>
where
    T: Send,
    E: ToString,
{
    plan.validate(case_count)?;
    if workers == 0 || workers > 64 {
        return Err(ResamplingError::InvalidPlan(
            "workers must be between 1 and 64".into(),
        ));
    }
    if is_cancelled() {
        return Err(ResamplingError::Cancelled);
    }
    let completed = AtomicU64::new(0);
    let progress_guard = Mutex::new(());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| ResamplingError::WorkerPool(error.to_string()))?;
    let outcomes = pool.install(|| {
        (0..plan.replicates)
            .into_par_iter()
            .map(|replicate_index| {
                if is_cancelled() {
                    return None;
                }
                let indices = bootstrap_indices(
                    case_count,
                    plan.master_seed,
                    &plan.operation,
                    replicate_index,
                );
                let outcome = match estimate(replicate_index, &indices) {
                    Ok(value) => ReplicateOutcome::Success { value },
                    Err(error) => ReplicateOutcome::Failed {
                        message: error.to_string(),
                    },
                };
                let _guard = progress_guard.lock().expect("progress mutex poisoned");
                let completed_replicates = completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                report_progress(ResamplingProgress {
                    phase: ResamplingPhase::Bootstrap,
                    completed_replicates,
                    total_replicates: plan.replicates,
                });
                Some(outcome)
            })
            .collect::<Vec<_>>()
    });
    if is_cancelled() || outcomes.iter().any(Option::is_none) {
        return Err(ResamplingError::Cancelled);
    }
    Ok(BootstrapRun {
        method_version: RESAMPLING_METHOD_VERSION.into(),
        plan: plan.clone(),
        outcomes: outcomes.into_iter().map(Option::unwrap).collect(),
    })
}

pub fn run_permutation<T, E>(
    case_count: usize,
    plan: &PermutationPlan,
    workers: usize,
    estimate: impl Fn(u32) -> Result<T, E> + Sync,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PermutationRun<T>, ResamplingError>
where
    T: Send,
    E: ToString,
{
    plan.validate(case_count)?;
    if workers == 0 || workers > 64 {
        return Err(ResamplingError::InvalidPlan(
            "workers must be between 1 and 64".into(),
        ));
    }
    if is_cancelled() {
        return Err(ResamplingError::Cancelled);
    }
    let completed = AtomicU64::new(0);
    let progress_guard = Mutex::new(());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| ResamplingError::WorkerPool(error.to_string()))?;
    let outcomes = pool.install(|| {
        (0..plan.permutations)
            .into_par_iter()
            .map(|permutation_index| {
                if is_cancelled() {
                    return None;
                }
                let outcome = match estimate(permutation_index) {
                    Ok(value) => ReplicateOutcome::Success { value },
                    Err(error) => ReplicateOutcome::Failed {
                        message: error.to_string(),
                    },
                };
                let _guard = progress_guard.lock().expect("progress mutex poisoned");
                let completed_replicates = completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                report_progress(ResamplingProgress {
                    phase: ResamplingPhase::Permutation,
                    completed_replicates,
                    total_replicates: plan.permutations,
                });
                Some(outcome)
            })
            .collect::<Vec<_>>()
    });
    if is_cancelled() || outcomes.iter().any(Option::is_none) {
        return Err(ResamplingError::Cancelled);
    }
    Ok(PermutationRun {
        method_version: PERMUTATION_METHOD_VERSION.into(),
        plan: plan.clone(),
        outcomes: outcomes.into_iter().map(Option::unwrap).collect(),
    })
}

pub fn run_jackknife<T, E>(
    case_count: usize,
    operation: &str,
    workers: usize,
    estimate: impl Fn(usize) -> Result<T, E> + Sync,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<JackknifeRun<T>, ResamplingError>
where
    T: Send,
    E: ToString,
{
    if case_count < 3 {
        return Err(ResamplingError::InvalidPlan(
            "jackknife requires at least three cases".into(),
        ));
    }
    if case_count > u32::MAX as usize {
        return Err(ResamplingError::InvalidPlan(
            "jackknife case count exceeds progress index capacity".into(),
        ));
    }
    if operation.trim().is_empty() {
        return Err(ResamplingError::InvalidPlan(
            "operation identifier cannot be empty".into(),
        ));
    }
    if workers == 0 || workers > 64 {
        return Err(ResamplingError::InvalidPlan(
            "workers must be between 1 and 64".into(),
        ));
    }
    if is_cancelled() {
        return Err(ResamplingError::Cancelled);
    }
    let completed = AtomicU64::new(0);
    let progress_guard = Mutex::new(());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| ResamplingError::WorkerPool(error.to_string()))?;
    let outcomes = pool.install(|| {
        (0..case_count)
            .into_par_iter()
            .map(|omitted_case| {
                if is_cancelled() {
                    return None;
                }
                let outcome = match estimate(omitted_case) {
                    Ok(value) => ReplicateOutcome::Success { value },
                    Err(error) => ReplicateOutcome::Failed {
                        message: error.to_string(),
                    },
                };
                let _guard = progress_guard.lock().expect("progress mutex poisoned");
                let completed_replicates = completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                report_progress(ResamplingProgress {
                    phase: ResamplingPhase::Jackknife,
                    completed_replicates,
                    total_replicates: case_count as u32,
                });
                Some(outcome)
            })
            .collect::<Vec<_>>()
    });
    if is_cancelled() || outcomes.iter().any(Option::is_none) {
        return Err(ResamplingError::Cancelled);
    }
    Ok(JackknifeRun {
        method_version: JACKKNIFE_METHOD_VERSION.into(),
        case_count,
        operation: operation.into(),
        outcomes: outcomes.into_iter().map(Option::unwrap).collect(),
    })
}

pub fn bootstrap_pls(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlsBootstrapResult, PlsBootstrapError> {
    if recipe.settings.studentized_inner_samples > 0
        && (recipe.settings.bootstrap_samples < 999
            || !(99..=999).contains(&recipe.settings.studentized_inner_samples)
            || recipe.settings.studentized_inner_samples % 2 == 0)
    {
        return Err(PlsBootstrapError::InvalidStudentizedPlan);
    }
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .map_err(|error| PlsBootstrapError::InconsistentResult(error.to_string()))?;
    bootstrap_pls_validated(
        dataset,
        &execution,
        original,
        workers,
        is_cancelled,
        report_progress,
    )
}

/// Bootstraps an opaque schema-v3 recipe capability. The capability is
/// validated once before the worker pool and the no-resampling base is reused
/// by all replicates.
pub fn bootstrap_pls_validated(
    dataset: &Dataset,
    recipe: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlsBootstrapResult, PlsBootstrapError> {
    let effective_recipe = recipe
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| PlsBootstrapError::InconsistentResult(error.to_string()))?;
    let base_execution = recipe
        .without_outer_resampling()
        .map_err(|error| PlsBootstrapError::InconsistentResult(error.to_string()))?;
    let recipe = effective_recipe;
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlsBootstrapError::RawDataRequired);
    }
    if recipe.settings.method != AnalysisMethod::PlsPm {
        return Err(PlsBootstrapError::InvalidMethod);
    }
    if recipe.settings.bootstrap_samples == 0 {
        return Err(PlsBootstrapError::MissingReplicates);
    }
    if recipe.settings.studentized_inner_samples > 0
        && (recipe.settings.bootstrap_samples < 999
            || !(99..=999).contains(&recipe.settings.studentized_inner_samples)
            || recipe.settings.studentized_inner_samples % 2 == 0)
    {
        return Err(PlsBootstrapError::InvalidStudentizedPlan);
    }
    if !original.converged || original.method_version != qpls_estimation::PLS_METHOD_VERSION {
        return Err(PlsBootstrapError::InconsistentResult(
            "base estimate is not a converged PLS-PM v1 result".into(),
        ));
    }
    let base_recipe = base_execution.effective();
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if original.used_observations != complete_rows.len() {
        return Err(PlsBootstrapError::InconsistentResult(
            "base estimate observation count differs from the complete-case sample".into(),
        ));
    }
    let plan = BootstrapPlan {
        replicates: recipe.settings.bootstrap_samples,
        master_seed: recipe.settings.seed,
        operation: "pls_pm_bootstrap_v1".into(),
    };
    let cancellation = &is_cancelled;
    let progress_callback = &report_progress;
    let inner_completed = AtomicU64::new(0);
    let inner_progress_guard = Mutex::new(());
    let inner_total = recipe
        .settings
        .bootstrap_samples
        .saturating_mul(recipe.settings.studentized_inner_samples);
    let run = run_bootstrap(
        complete_rows.len(),
        &plan,
        workers,
        |replicate_index, indices| {
            let raw_indices = indices
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let sampled = resample_model_dataset(dataset, base_recipe, &raw_indices, cancellation)?;
            let mut estimate =
                estimate_pls_validated_with_control(&sampled, &base_execution, |_| {
                    !cancellation()
                })?;
            align_pls_signs(
                &mut estimate,
                &original.construct_scores,
                indices,
                cancellation,
            )?;
            let (studentized_standard_errors, studentized_error) =
                if recipe.settings.studentized_inner_samples > 0 {
                    match inner_bootstrap_standard_errors(
                        &sampled,
                        &base_execution,
                        &estimate,
                        plan.master_seed,
                        replicate_index,
                        recipe.settings.studentized_inner_samples,
                        cancellation,
                        &|| {
                            let _guard = inner_progress_guard
                                .lock()
                                .expect("studentized progress mutex poisoned");
                            let completed =
                                inner_completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                            progress_callback(ResamplingProgress {
                                phase: ResamplingPhase::StudentizedInner,
                                completed_replicates: completed,
                                total_replicates: inner_total,
                            });
                        },
                    ) {
                        Ok(summary) => (summary, None),
                        Err(EstimationError::Cancelled) => return Err(EstimationError::Cancelled),
                        Err(error) => (None, Some(error.to_string())),
                    }
                } else {
                    (None, None)
                };
            Ok::<_, EstimationError>(PlsBootstrapEstimate {
                replicate_index,
                iterations: estimate.iterations,
                used_observations: estimate.used_observations,
                omitted_observations: estimate.omitted_observations,
                outer_estimates: estimate.outer_estimates,
                paths: estimate.paths,
                effects: estimate.effects,
                r_squared: estimate.r_squared,
                studentized_standard_errors,
                studentized_error,
            })
        },
        cancellation,
        progress_callback,
    )?;
    let successful = run
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Success { .. }))
        .count();
    let required = ((run.plan.replicates as f64 * 0.9).ceil() as usize).max(2);
    if successful < required {
        return Err(PlsBootstrapError::InsufficientUsableReplicates {
            usable: successful,
            required,
        });
    }
    let percentile = summarize_percentile(original, &run, recipe.settings.confidence_level)?;
    let jackknife = jackknife_pls(
        dataset,
        &base_execution,
        original,
        workers,
        || cancellation(),
        progress_callback,
    )
    .map_err(|error| PlsBootstrapError::Jackknife(error.to_string()))?;
    let bca = summarize_bca(original, &run, &jackknife, recipe.settings.confidence_level)?;
    let studentized = if recipe.settings.studentized_inner_samples > 0 {
        Some(summarize_studentized(
            original,
            &run,
            &percentile,
            recipe.settings.confidence_level,
            recipe.settings.studentized_inner_samples,
        )?)
    } else {
        None
    };
    let failed_replicates = run
        .outcomes
        .iter()
        .enumerate()
        .filter_map(|(index, outcome)| match outcome {
            ReplicateOutcome::Failed { message } => Some(FailedReplicate {
                replicate_index: index as u32,
                message: message.clone(),
            }),
            ReplicateOutcome::Success { .. } => None,
        })
        .collect::<Vec<_>>();
    Ok(PlsBootstrapResult {
        method_version: run.method_version,
        plan: run.plan,
        usable_replicates: successful as u32,
        failed_replicates,
        percentile,
        bca: Some(bca),
        studentized,
    })
}

#[derive(Debug, Clone, PartialEq)]
struct RegressionBootstrapEstimate {
    coefficients: Vec<f64>,
}

/// Dedicated case-resampling inference for standalone OLS and binary logistic
/// regression. This path deliberately does not return `PlsBootstrapResult`:
/// its output is nested in the regression estimate and cannot be interpreted
/// as PLS-PM resampling evidence.
pub fn bootstrap_regression_validated(
    dataset: &Dataset,
    execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<RegressionBootstrapAnalysis, RegressionBootstrapError> {
    if dataset.schema.kind != DataKind::Raw {
        return Err(RegressionBootstrapError::RawDataRequired);
    }
    execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| RegressionBootstrapError::InconsistentResult(error.to_string()))?;
    let source = execution.source();
    let MethodConfig::Regression {
        outcome,
        predictors,
        controls,
        model,
        bootstrap: Some(bootstrap),
    } = source
        .method_config
        .as_ref()
        .ok_or(RegressionBootstrapError::InvalidMethod)?
    else {
        return Err(RegressionBootstrapError::InvalidMethod);
    };
    if source.settings.method != AnalysisMethod::Regression
        || workers != source.settings.workers
        || source.settings.studentized_inner_samples != 0
        || source.settings.permutation_samples != 0
        || !(99..=10_000).contains(&source.settings.bootstrap_samples)
        || bootstrap.algorithm != RegressionBootstrapAlgorithm::CaseResampling
        || bootstrap.intervals
            != [
                RegressionBootstrapInterval::Percentile,
                RegressionBootstrapInterval::Bca,
            ]
    {
        return Err(RegressionBootstrapError::InvalidMethod);
    }
    let (regression_type, base_method_version, logistic) = match model {
        RegressionModelConfig::Ols { .. } => ("ols", REGRESSION_OLS_METHOD_VERSION, false),
        RegressionModelConfig::Logistic => ("logistic", REGRESSION_LOGISTIC_METHOD_VERSION, true),
        RegressionModelConfig::Process { .. } => {
            return Err(RegressionBootstrapError::InvalidMethod);
        }
    };
    let original_regression = original.regression.as_ref().ok_or_else(|| {
        RegressionBootstrapError::InconsistentResult("base regression payload is missing".into())
    })?;
    if original.method_version != base_method_version
        || original_regression.method_version != base_method_version
        || original_regression.regression_type != regression_type
        || original_regression.bootstrap.is_some()
    {
        return Err(RegressionBootstrapError::InconsistentResult(
            "base estimate method identity is not current point-only regression".into(),
        ));
    }
    let expected_terms = std::iter::once("intercept".to_string())
        .chain(predictors.iter().cloned())
        .chain(controls.iter().cloned())
        .collect::<Vec<_>>();
    if original_regression.coefficients.len() != expected_terms.len()
        || original_regression
            .coefficients
            .iter()
            .zip(&expected_terms)
            .any(|(coefficient, expected)| coefficient.term != *expected)
    {
        return Err(RegressionBootstrapError::InconsistentResult(
            "base coefficient identities differ from the typed recipe".into(),
        ));
    }
    let mut variables = Vec::with_capacity(1 + predictors.len() + controls.len());
    variables.push(outcome.clone());
    variables.extend(predictors.iter().cloned());
    variables.extend(controls.iter().cloned());
    let positions = variables
        .iter()
        .map(|variable| {
            dataset.batch.schema().index_of(variable).map_err(|_| {
                RegressionBootstrapError::InconsistentResult(format!(
                    "typed regression variable is missing from the dataset: {variable}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let complete_rows = complete_case_rows_at_positions(dataset, &positions);
    if original.used_observations != complete_rows.len()
        || original_regression.observations != complete_rows.len()
    {
        return Err(RegressionBootstrapError::InconsistentResult(
            "base observation count differs from the regression complete-case sample".into(),
        ));
    }
    let base_execution = execution
        .without_outer_resampling()
        .map_err(|error| RegressionBootstrapError::InconsistentResult(error.to_string()))?;
    let plan = BootstrapPlan {
        replicates: source.settings.bootstrap_samples,
        master_seed: source.settings.seed,
        operation: format!("regression_{regression_type}_case_bootstrap_v1"),
    };
    let cancellation = &is_cancelled;
    let progress_callback = &report_progress;
    let run = run_bootstrap(
        complete_rows.len(),
        &plan,
        workers,
        |_replicate_index, sampled_positions| {
            let raw_indices = sampled_positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let estimate = estimate_regression_case_resample_validated_with_control(
                dataset,
                &base_execution,
                &raw_indices,
                |_| !cancellation(),
            )
            .map_err(regression_replicate_error)?;
            let regression = estimate.regression.ok_or_else(|| {
                "missing_regression_payload|resampled estimate omitted regression output"
                    .to_string()
            })?;
            let coefficients = regression
                .coefficients
                .iter()
                .map(|coefficient| coefficient.estimate)
                .collect::<Vec<_>>();
            if regression.method_version != base_method_version
                || regression.coefficients.len() != expected_terms.len()
                || regression.coefficients.iter().zip(&expected_terms).any(
                    |(coefficient, expected)| {
                        coefficient.term != *expected
                            || !coefficient.estimate.is_finite()
                            || (logistic && !coefficient.estimate.exp().is_finite())
                    },
                )
            {
                return Err(
                    "inconsistent_replicate|resampled coefficient identity or value is invalid"
                        .to_string(),
                );
            }
            Ok(RegressionBootstrapEstimate { coefficients })
        },
        cancellation,
        progress_callback,
    )?;
    let usable = run
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Success { .. }))
        .count();
    let required = ((run.plan.replicates as f64 * REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION)
        .ceil() as usize)
        .max(2);
    if usable < required {
        return Err(RegressionBootstrapError::InsufficientUsableReplicates { usable, required });
    }

    let jackknife = run_jackknife(
        complete_rows.len(),
        &format!("regression_{regression_type}_case_jackknife_v1"),
        workers,
        |omitted_case| {
            let raw_indices = complete_rows
                .iter()
                .enumerate()
                .filter_map(|(position, raw)| (position != omitted_case).then_some(*raw))
                .collect::<Vec<_>>();
            let estimate = estimate_regression_case_resample_validated_with_control(
                dataset,
                &base_execution,
                &raw_indices,
                |_| !cancellation(),
            )
            .map_err(regression_replicate_error)?;
            let regression = estimate.regression.ok_or_else(|| {
                "missing_regression_payload|delete-one estimate omitted regression output"
                    .to_string()
            })?;
            if regression.coefficients.len() != expected_terms.len()
                || regression.coefficients.iter().zip(&expected_terms).any(
                    |(coefficient, expected)| {
                        coefficient.term != *expected
                            || !coefficient.estimate.is_finite()
                            || (logistic && !coefficient.estimate.exp().is_finite())
                    },
                )
            {
                return Err(
                    "inconsistent_replicate|delete-one coefficient identity or value is invalid"
                        .to_string(),
                );
            }
            Ok(regression
                .coefficients
                .iter()
                .map(|coefficient| coefficient.estimate)
                .collect::<Vec<_>>())
        },
        cancellation,
        progress_callback,
    )?;

    let failed_replicates = run
        .outcomes
        .iter()
        .enumerate()
        .filter_map(|(index, outcome)| match outcome {
            ReplicateOutcome::Failed { message } => {
                let (reason_code, message) = split_regression_failure(message);
                Some(RegressionBootstrapFailedReplicate {
                    replicate_index: index as u32,
                    reason_code,
                    message,
                })
            }
            ReplicateOutcome::Success { .. } => None,
        })
        .collect::<Vec<_>>();
    let successful_bootstrap = run
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value.coefficients.clone()),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let successful_jackknife = jackknife
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value.clone()),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let coefficient_rows = summarize_regression_bootstrap_coefficients(
        &expected_terms,
        &original_regression
            .coefficients
            .iter()
            .map(|coefficient| coefficient.estimate)
            .collect::<Vec<_>>(),
        &successful_bootstrap,
        &successful_jackknife,
        jackknife.case_count,
        logistic,
        source.settings.confidence_level,
    )?;
    let failed_jackknife = jackknife
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Failed { .. }))
        .count();
    let validation_witness = RegressionBootstrapValidationWitness {
        method_version: REGRESSION_BOOTSTRAP_VALIDATION_WITNESS_VERSION.into(),
        terms: expected_terms,
        successful_bootstrap: run
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(replicate_index, outcome)| match outcome {
                ReplicateOutcome::Success { value } => {
                    Some(RegressionBootstrapWitnessBootstrapRow {
                        replicate_index: replicate_index as u32,
                        coefficients: value.coefficients.clone(),
                    })
                }
                ReplicateOutcome::Failed { .. } => None,
            })
            .collect(),
        successful_jackknife: jackknife
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(omitted_case, outcome)| match outcome {
                ReplicateOutcome::Success { value } => {
                    Some(RegressionBootstrapWitnessJackknifeRow {
                        omitted_case,
                        coefficients: value.clone(),
                    })
                }
                ReplicateOutcome::Failed { .. } => None,
            })
            .collect(),
        failed_jackknife: jackknife
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(omitted_case, outcome)| match outcome {
                ReplicateOutcome::Failed { message } => {
                    let (reason_code, message) = split_regression_failure(message);
                    Some(RegressionBootstrapFailedJackknife {
                        omitted_case,
                        reason_code,
                        message,
                    })
                }
                ReplicateOutcome::Success { .. } => None,
            })
            .collect(),
    };
    let mut warnings = vec![
        "Regression bootstrap v1 uses deterministic indexed case resampling with replacement; percentile intervals are primary and BCa intervals are conditional on stable delete-one fits."
            .into(),
        "Bootstrap ratio statistics use an independently implemented two-sided standard-normal reference for both OLS and logistic coefficients; they are distinct from point-estimate t or Wald inference."
            .into(),
    ];
    if !failed_replicates.is_empty() {
        warnings.push(format!(
            "{} of {} bootstrap replicates failed and were excluded from inference.",
            failed_replicates.len(),
            run.plan.replicates
        ));
    }
    if failed_jackknife > 0 {
        warnings.push(format!(
            "{failed_jackknife} of {} delete-one fits failed; affected BCa intervals are explicitly unavailable.",
            jackknife.case_count
        ));
    }
    Ok(RegressionBootstrapAnalysis {
        method_version: REGRESSION_BOOTSTRAP_METHOD_VERSION.into(),
        algorithm: REGRESSION_BOOTSTRAP_ALGORITHM.into(),
        confidence_level: source.settings.confidence_level,
        alternative: "two_sided".into(),
        interval_policy: REGRESSION_BOOTSTRAP_INTERVAL_POLICY.into(),
        test_reference: REGRESSION_BOOTSTRAP_TEST_REFERENCE.into(),
        test_tolerance_policy: REGRESSION_BOOTSTRAP_TEST_TOLERANCE_POLICY.into(),
        requested_replicates: run.plan.replicates,
        usable_replicates: usable as u32,
        minimum_usable_fraction: REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION,
        jackknife_cases: jackknife.case_count,
        usable_jackknife_cases: jackknife.case_count - failed_jackknife,
        seed: run.plan.master_seed,
        workers,
        stream_token: REGRESSION_BOOTSTRAP_STREAM_TOKEN.into(),
        failed_replicates,
        coefficients: coefficient_rows,
        validation_witness,
        warnings,
    })
}

/// Dedicated PROCESS v2 case resampling. Although it shares the generic
/// schema-v3 case-resampling request, its method identity, stream domain,
/// estimands, witness, and promotion evidence are separate from standalone
/// OLS/logistic regression bootstrapping.
pub fn bootstrap_process_validated(
    dataset: &Dataset,
    execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<ProcessBootstrapAnalysis, RegressionBootstrapError> {
    if dataset.schema.kind != DataKind::Raw {
        return Err(RegressionBootstrapError::RawDataRequired);
    }
    execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| RegressionBootstrapError::InconsistentResult(error.to_string()))?;
    let source = execution.source();
    let MethodConfig::Regression {
        outcome,
        predictors,
        controls,
        model:
            RegressionModelConfig::Process {
                relationship: qpls_core::ProcessRelationshipConfig::Graph { .. },
            },
        bootstrap: Some(bootstrap),
    } = source
        .method_config
        .as_ref()
        .ok_or(RegressionBootstrapError::InvalidMethod)?
    else {
        return Err(RegressionBootstrapError::InvalidMethod);
    };
    if source.settings.method != AnalysisMethod::Regression
        || workers != source.settings.workers
        || source.settings.studentized_inner_samples != 0
        || source.settings.permutation_samples != 0
        || !(99..=10_000).contains(&source.settings.bootstrap_samples)
        || bootstrap.algorithm != RegressionBootstrapAlgorithm::CaseResampling
        || bootstrap.intervals
            != [
                RegressionBootstrapInterval::Percentile,
                RegressionBootstrapInterval::Bca,
            ]
    {
        return Err(RegressionBootstrapError::InvalidMethod);
    }
    let original_graph = original
        .regression
        .as_ref()
        .and_then(|regression| regression.process.as_ref())
        .and_then(|process| process.graph_v2.as_ref())
        .ok_or_else(|| {
            RegressionBootstrapError::InconsistentResult(
                "base PROCESS v2 graph payload is missing".into(),
            )
        })?;
    if original.method_version != REGRESSION_PROCESS_METHOD_VERSION
        || original_graph.bootstrap.is_some()
    {
        return Err(RegressionBootstrapError::InconsistentResult(
            "base estimate method identity is not point-only PROCESS v2".into(),
        ));
    }
    let original_estimands = process_bootstrap_estimands(original_graph);
    let estimand_ids = original_estimands
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if estimand_ids.is_empty()
        || estimand_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != estimand_ids.len()
    {
        return Err(RegressionBootstrapError::InconsistentResult(
            "PROCESS v2 estimand identities are empty or duplicated".into(),
        ));
    }
    let mut variables = Vec::with_capacity(1 + predictors.len() + controls.len());
    variables.push(outcome.clone());
    variables.extend(predictors.iter().cloned());
    variables.extend(controls.iter().cloned());
    let positions = variables
        .iter()
        .map(|variable| {
            dataset.batch.schema().index_of(variable).map_err(|_| {
                RegressionBootstrapError::InconsistentResult(format!(
                    "typed PROCESS variable is missing from the dataset: {variable}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let complete_rows = complete_case_rows_at_positions(dataset, &positions);
    if original.used_observations != complete_rows.len()
        || original_graph.complete_cases != complete_rows.len()
    {
        return Err(RegressionBootstrapError::InconsistentResult(
            "base observation count differs from the PROCESS complete-case sample".into(),
        ));
    }
    let base_execution = execution
        .without_outer_resampling()
        .map_err(|error| RegressionBootstrapError::InconsistentResult(error.to_string()))?;
    let plan = BootstrapPlan {
        replicates: source.settings.bootstrap_samples,
        master_seed: source.settings.seed,
        operation: PROCESS_BOOTSTRAP_STREAM_TOKEN.into(),
    };
    let cancellation = &is_cancelled;
    let progress_callback = &report_progress;
    let run = run_bootstrap(
        complete_rows.len(),
        &plan,
        workers,
        |_replicate_index, sampled_positions| {
            let raw_indices = sampled_positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            process_resample_estimands(
                dataset,
                &base_execution,
                &raw_indices,
                &estimand_ids,
                original_graph,
                cancellation,
            )
        },
        cancellation,
        progress_callback,
    )?;
    let usable = run
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Success { .. }))
        .count();
    let required = ((run.plan.replicates as f64 * REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION)
        .ceil() as usize)
        .max(2);
    if usable < required {
        return Err(RegressionBootstrapError::InsufficientUsableReplicates { usable, required });
    }
    let jackknife = run_jackknife(
        complete_rows.len(),
        "regression_process_case_jackknife_v1",
        workers,
        |omitted_case| {
            let raw_indices = complete_rows
                .iter()
                .enumerate()
                .filter_map(|(position, raw)| (position != omitted_case).then_some(*raw))
                .collect::<Vec<_>>();
            process_resample_estimands(
                dataset,
                &base_execution,
                &raw_indices,
                &estimand_ids,
                original_graph,
                cancellation,
            )
        },
        cancellation,
        progress_callback,
    )?;
    let successful_bootstrap = run
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value.coefficients.clone()),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let successful_jackknife = jackknife
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value.coefficients.clone()),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let estimands = summarize_process_bootstrap_estimands(
        &estimand_ids,
        &original_estimands
            .iter()
            .map(|(_, estimate)| *estimate)
            .collect::<Vec<_>>(),
        &successful_bootstrap,
        &successful_jackknife,
        jackknife.case_count,
        source.settings.confidence_level,
    )?;
    let failed_replicates = run
        .outcomes
        .iter()
        .enumerate()
        .filter_map(|(replicate_index, outcome)| match outcome {
            ReplicateOutcome::Failed { message } => {
                let (reason_code, message) = split_regression_failure(message);
                Some(ProcessBootstrapFailedReplicate {
                    replicate_index: replicate_index as u32,
                    reason_code,
                    message,
                })
            }
            ReplicateOutcome::Success { .. } => None,
        })
        .collect::<Vec<_>>();
    let failed_jackknife = jackknife
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Failed { .. }))
        .count();
    let validation_witness = ProcessBootstrapValidationWitness {
        method_version: PROCESS_BOOTSTRAP_VALIDATION_WITNESS_VERSION.into(),
        estimand_ids,
        successful_bootstrap: run
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(replicate_index, outcome)| match outcome {
                ReplicateOutcome::Success { value } => Some(ProcessBootstrapWitnessBootstrapRow {
                    replicate_index: replicate_index as u32,
                    estimates: value.coefficients.clone(),
                }),
                ReplicateOutcome::Failed { .. } => None,
            })
            .collect(),
        successful_jackknife: jackknife
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(omitted_case, outcome)| match outcome {
                ReplicateOutcome::Success { value } => Some(ProcessBootstrapWitnessJackknifeRow {
                    omitted_case,
                    estimates: value.coefficients.clone(),
                }),
                ReplicateOutcome::Failed { .. } => None,
            })
            .collect(),
        failed_jackknife: jackknife
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(omitted_case, outcome)| match outcome {
                ReplicateOutcome::Failed { message } => {
                    let (reason_code, message) = split_regression_failure(message);
                    Some(RegressionBootstrapFailedJackknife {
                        omitted_case,
                        reason_code,
                        message,
                    })
                }
                ReplicateOutcome::Success { .. } => None,
            })
            .collect(),
    };
    let mut warnings = vec![
        "PROCESS bootstrap v1 uses deterministic indexed complete-case resampling with replacement; percentile intervals are primary and BCa intervals require every delete-one fit.".into(),
        "PROCESS bootstrap ratio tests use the original effect divided by its bootstrap standard error with a fixed two-sided standard-normal reference.".into(),
    ];
    if !failed_replicates.is_empty() {
        warnings.push(format!(
            "{} of {} PROCESS bootstrap replicates failed and were excluded from inference.",
            failed_replicates.len(),
            run.plan.replicates
        ));
    }
    if failed_jackknife > 0 {
        warnings.push(format!(
            "{failed_jackknife} of {} PROCESS delete-one fits failed; BCa intervals are explicitly unavailable.",
            jackknife.case_count
        ));
    }
    Ok(ProcessBootstrapAnalysis {
        method_version: PROCESS_BOOTSTRAP_METHOD_VERSION.into(),
        algorithm: PROCESS_BOOTSTRAP_ALGORITHM.into(),
        interval_policy: PROCESS_BOOTSTRAP_INTERVAL_POLICY.into(),
        test_reference: PROCESS_BOOTSTRAP_TEST_REFERENCE.into(),
        requested_replicates: run.plan.replicates,
        usable_replicates: usable as u32,
        minimum_usable_fraction: REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION,
        jackknife_cases: jackknife.case_count,
        usable_jackknife_cases: jackknife.case_count - failed_jackknife,
        seed: run.plan.master_seed,
        workers,
        stream_token: PROCESS_BOOTSTRAP_STREAM_TOKEN.into(),
        failed_replicates,
        estimands,
        validation_witness,
        warnings,
    })
}

fn process_resample_estimands(
    dataset: &Dataset,
    base_execution: &ValidatedExecutionRecipe,
    raw_indices: &[usize],
    expected_ids: &[String],
    reference: &qpls_estimation::ProcessGraphAnalysis,
    cancellation: &impl Fn() -> bool,
) -> Result<RegressionBootstrapEstimate, String> {
    let estimate = estimate_regression_case_resample_validated_with_control(
        dataset,
        base_execution,
        raw_indices,
        |_| !cancellation(),
    )
    .map_err(process_replicate_error)?;
    let graph = estimate
        .regression
        .as_ref()
        .and_then(|regression| regression.process.as_ref())
        .and_then(|process| process.graph_v2.as_ref())
        .ok_or_else(|| {
            "nonfinite_estimate|resampled PROCESS estimate omitted graph output".to_string()
        })?;
    let estimands = process_bootstrap_estimands_at_reference(graph, reference)
        .map_err(|message| format!("nonfinite_estimate|{message}"))?;
    if estimands.len() != expected_ids.len()
        || estimands
            .iter()
            .zip(expected_ids)
            .any(|((id, value), expected)| id != expected || !value.is_finite())
    {
        return Err(
            "nonfinite_estimate|resampled PROCESS estimand identity or value is invalid".into(),
        );
    }
    Ok(RegressionBootstrapEstimate {
        coefficients: estimands
            .into_iter()
            .map(|(_, estimate)| estimate)
            .collect(),
    })
}

fn process_replicate_error(error: EstimationError) -> String {
    let reason = match &error {
        EstimationError::RankDeficient(_) => "rank_deficient_equation",
        EstimationError::UnsupportedMethod(message)
            if message.starts_with("invalid_binary_profile|") =>
        {
            "invalid_binary_profile"
        }
        EstimationError::UnsupportedMethod(message)
            if message.starts_with("high_leverage_hc3_instability|") =>
        {
            "high_leverage_hc3_instability"
        }
        EstimationError::Numerical(message) if message.starts_with("invalid_hc3_covariance|") => {
            "invalid_hc3_covariance"
        }
        EstimationError::Numerical(message)
            if message.starts_with("degenerate_simple_slope_variance|") =>
        {
            "degenerate_simple_slope_variance"
        }
        EstimationError::Cancelled => "cancelled",
        _ => "nonfinite_estimate",
    };
    format!("{reason}|{error}")
}

fn normalize_process_bootstrap_statuses(
    summary: &mut RegressionBootstrapCoefficient,
    jackknife_values: &[f64],
) {
    if let RegressionBootstrapTest::Unavailable {
        reason_code,
        message,
    } = &mut summary.test
    {
        *reason_code = "zero_bootstrap_standard_error".into();
        *message = "PROCESS bootstrap ratio inference is unavailable because the estimand distribution has zero or nonfinite spread".into();
    }
    if let RegressionBootstrapBcaInterval::Unavailable {
        reason_code,
        message,
    } = &mut summary.bca
    {
        match reason_code.as_str() {
            "incomplete_jackknife" => {}
            "insufficient_jackknife_estimates" => {
                *reason_code = "zero_jackknife_variance".into();
                *message = "PROCESS BCa is unavailable because the complete delete-one distribution has insufficient nonzero variation".into();
            }
            _ => {
                let mean = if jackknife_values.is_empty() {
                    0.0
                } else {
                    jackknife_values.iter().sum::<f64>() / jackknife_values.len() as f64
                };
                let spread = jackknife_values
                    .iter()
                    .map(|value| (value - mean).abs())
                    .fold(0.0, f64::max);
                if spread <= 64.0 * f64::EPSILON * mean.abs().max(1.0) {
                    *reason_code = "zero_jackknife_variance".into();
                    *message = "PROCESS BCa is unavailable because the complete delete-one distribution has zero variance".into();
                } else {
                    *reason_code = "nonfinite_adjusted_probability".into();
                    *message = "PROCESS BCa is unavailable because the bias correction, acceleration, or adjusted probabilities are nonfinite".into();
                }
            }
        }
    }
}

/// Recomputes PROCESS v2 bootstrap summaries with PROCESS-specific tagged
/// unavailable reasons. Archive validation calls the same pure function, so
/// generic OLS/logistic reason tokens cannot accidentally enter PROCESS output.
pub fn summarize_process_bootstrap_estimands(
    estimand_ids: &[String],
    original: &[f64],
    bootstrap_estimates: &[Vec<f64>],
    jackknife_estimates: &[Vec<f64>],
    expected_jackknife_cases: usize,
    confidence_level: f64,
) -> Result<Vec<ProcessBootstrapEstimand>, RegressionBootstrapError> {
    let summaries = summarize_regression_bootstrap_coefficients(
        estimand_ids,
        original,
        bootstrap_estimates,
        jackknife_estimates,
        expected_jackknife_cases,
        false,
        confidence_level,
    )?;
    Ok(summaries
        .into_iter()
        .enumerate()
        .map(|(index, mut summary)| {
            normalize_process_bootstrap_statuses(
                &mut summary,
                &jackknife_estimates
                    .iter()
                    .map(|row| row[index])
                    .collect::<Vec<_>>(),
            );
            ProcessBootstrapEstimand {
                effect_id: summary.term,
                original: summary.original,
                bootstrap_mean: summary.bootstrap_mean,
                bias: summary.bias,
                standard_error: summary.standard_error,
                test: summary.test,
                percentile_lower: summary.percentile_lower,
                percentile_upper: summary.percentile_upper,
                bca: summary.bca,
                usable_replicates: summary.usable_replicates,
            }
        })
        .collect())
}

/// Pure, deterministic arithmetic used by the engine and validation-only
/// reference harnesses. Rows are replicate-major, columns follow `terms`.
pub fn summarize_regression_bootstrap_coefficients(
    terms: &[String],
    original: &[f64],
    bootstrap_estimates: &[Vec<f64>],
    jackknife_estimates: &[Vec<f64>],
    expected_jackknife_cases: usize,
    logistic: bool,
    confidence_level: f64,
) -> Result<Vec<RegressionBootstrapCoefficient>, RegressionBootstrapError> {
    let width = terms.len();
    if width == 0
        || original.len() != width
        || bootstrap_estimates.len() < 2
        || !confidence_level.is_finite()
        || !(0.0..1.0).contains(&confidence_level)
        || original
            .iter()
            .any(|value| !value.is_finite() || (logistic && !value.exp().is_finite()))
        || bootstrap_estimates.iter().any(|row| {
            row.len() != width
                || row
                    .iter()
                    .any(|value| !value.is_finite() || (logistic && !value.exp().is_finite()))
        })
        || expected_jackknife_cases < jackknife_estimates.len()
        || jackknife_estimates.iter().any(|row| {
            row.len() != width
                || row
                    .iter()
                    .any(|value| !value.is_finite() || (logistic && !value.exp().is_finite()))
        })
    {
        return Err(RegressionBootstrapError::InvalidSummary(
            "term, point, bootstrap, jackknife, confidence, or finite-value dimensions differ"
                .into(),
        ));
    }
    let normal = Normal::standard();
    let tail = (1.0 - confidence_level) / 2.0;
    let mut rows = Vec::with_capacity(width);
    for coefficient_index in 0..width {
        let mut values = bootstrap_estimates
            .iter()
            .map(|row| row[coefficient_index])
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        let point = original[coefficient_index];
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let standard_error = sample_standard_deviation(&values, mean);
        let replicate_max_abs = values.iter().map(|value| value.abs()).fold(0.0, f64::max);
        let test_tolerance = 64.0 * f64::EPSILON * point.abs().max(replicate_max_abs).max(1.0);
        let test = if standard_error.is_finite() && standard_error > test_tolerance {
            let statistic = point / standard_error;
            RegressionBootstrapTest::Available {
                statistic,
                p_value_two_sided: (2.0 * normal.sf(statistic.abs())).clamp(0.0, 1.0),
            }
        } else {
            RegressionBootstrapTest::Unavailable {
                reason_code: "degenerate_bootstrap_standard_error".into(),
                message: "Bootstrap ratio inference is unavailable because the coefficient distribution has zero or nonfinite spread".into(),
            }
        };
        let jackknife_values = jackknife_estimates
            .iter()
            .map(|row| row[coefficient_index])
            .collect::<Vec<_>>();
        let jackknife_complete = jackknife_estimates.len() == expected_jackknife_cases;
        let bca = regression_bca_status(
            &values,
            point,
            &jackknife_values,
            confidence_level,
            jackknife_complete,
        );
        let odds_ratio = logistic.then(|| {
            // `values` is sorted and exp is strictly monotone, so the
            // transformed vector remains sorted for Type-7 quantiles. The
            // interpolation is performed on the OR scale, not fabricated by
            // exponentiating coefficient-scale interval endpoints.
            let transformed = values.iter().map(|value| value.exp()).collect::<Vec<_>>();
            let transformed_jackknife = jackknife_values
                .iter()
                .map(|value| value.exp())
                .collect::<Vec<_>>();
            RegressionBootstrapOddsRatio {
                original: point.exp(),
                percentile_lower: type7_quantile(&transformed, tail),
                percentile_upper: type7_quantile(&transformed, 1.0 - tail),
                bca: regression_bca_status(
                    &transformed,
                    point.exp(),
                    &transformed_jackknife,
                    confidence_level,
                    jackknife_complete,
                ),
            }
        });
        rows.push(RegressionBootstrapCoefficient {
            term: terms[coefficient_index].clone(),
            original: point,
            bootstrap_mean: mean,
            bias: mean - point,
            standard_error,
            replicate_max_abs,
            test_tolerance,
            test,
            percentile_lower: type7_quantile(&values, tail),
            percentile_upper: type7_quantile(&values, 1.0 - tail),
            usable_replicates: bootstrap_estimates.len() as u32,
            bca,
            odds_ratio,
        });
    }
    Ok(rows)
}

fn regression_replicate_error(error: EstimationError) -> String {
    let reason = match &error {
        EstimationError::RankDeficient(_) => "rank_deficient",
        EstimationError::LogisticNonConvergence(_) | EstimationError::NonConvergence(_) => {
            "nonconvergence"
        }
        EstimationError::UnsupportedMethod(message) if message.contains("both 0 and 1") => {
            "single_class_resample"
        }
        EstimationError::Numerical(_) => "numerical_failure",
        EstimationError::Cancelled => "cancelled",
        _ => "estimation_failure",
    };
    format!("{reason}|{error}")
}

fn split_regression_failure(value: &str) -> (String, String) {
    value
        .split_once('|')
        .map(|(reason, message)| (reason.to_string(), message.to_string()))
        .unwrap_or_else(|| ("estimation_failure".into(), value.to_string()))
}

fn sample_standard_deviation(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return f64::NAN;
    }
    (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64)
        .sqrt()
}

fn regression_bca_status(
    bootstrap_values: &[f64],
    original: f64,
    jackknife_values: &[f64],
    confidence_level: f64,
    jackknife_complete: bool,
) -> RegressionBootstrapBcaInterval {
    if !jackknife_complete {
        return RegressionBootstrapBcaInterval::Unavailable {
            reason_code: "incomplete_jackknife".into(),
            message:
                "BCa is unavailable because at least one required delete-one regression fit failed"
                    .into(),
        };
    }
    if jackknife_values.len() < 3 {
        return RegressionBootstrapBcaInterval::Unavailable {
            reason_code: "insufficient_jackknife_estimates".into(),
            message: "BCa is unavailable because fewer than three finite delete-one estimates were usable"
                .into(),
        };
    }
    match bca_interval(
        bootstrap_values,
        original,
        jackknife_values,
        confidence_level,
    ) {
        Some(interval) => RegressionBootstrapBcaInterval::Available {
            bias_correction: interval.bias_correction,
            acceleration: interval.acceleration,
            lower: interval.lower,
            upper: interval.upper,
        },
        None => RegressionBootstrapBcaInterval::Unavailable {
            reason_code: "degenerate_jackknife_acceleration".into(),
            message: "BCa is unavailable because the delete-one acceleration or adjusted quantiles are numerically undefined"
                .into(),
        },
    }
}

fn jackknife_pls(
    dataset: &Dataset,
    recipe: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<JackknifeRun<PlsJackknifeEstimate>, PlsJackknifeError> {
    let effective_recipe = recipe.effective();
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlsJackknifeError::RawDataRequired);
    }
    if effective_recipe.settings.method != AnalysisMethod::PlsPm {
        return Err(PlsJackknifeError::InvalidMethod);
    }
    if !original.converged || original.method_version != qpls_estimation::PLS_METHOD_VERSION {
        return Err(PlsJackknifeError::InconsistentResult(
            "base estimate is not a converged PLS-PM v1 result".into(),
        ));
    }
    let base_recipe = effective_recipe;
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if complete_rows.len() < 4 {
        return Err(PlsJackknifeError::InsufficientCases(complete_rows.len()));
    }
    if original.used_observations != complete_rows.len() {
        return Err(PlsJackknifeError::InconsistentResult(
            "base estimate observation count differs from the complete-case sample".into(),
        ));
    }
    let cancellation = &is_cancelled;
    run_jackknife(
        complete_rows.len(),
        "pls_pm_jackknife_v1",
        workers,
        |omitted_case| {
            let sampled_positions = (0..complete_rows.len())
                .filter(|position| *position != omitted_case)
                .collect::<Vec<_>>();
            let raw_indices = sampled_positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let sampled = resample_model_dataset(dataset, base_recipe, &raw_indices, cancellation)?;
            let mut estimate =
                estimate_pls_validated_with_control(&sampled, recipe, |_| !cancellation())?;
            align_pls_signs(
                &mut estimate,
                &original.construct_scores,
                &sampled_positions,
                cancellation,
            )?;
            Ok::<_, EstimationError>(PlsJackknifeEstimate {
                omitted_case,
                parameters: result_values(
                    &estimate.outer_estimates,
                    &estimate.paths,
                    &estimate.effects,
                    &estimate.r_squared,
                ),
            })
        },
        cancellation,
        report_progress,
    )
    .map_err(PlsJackknifeError::from)
}

struct StructuralPermutationSetup {
    parameter: String,
    original: f64,
    focal_index: usize,
    predictors: Vec<Vec<f64>>,
    fitted_nuisance: Vec<f64>,
    residuals: Vec<f64>,
}

fn prepare_freedman_lane(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    focal_index: usize,
    subject: &str,
) -> Result<(f64, Vec<f64>, Vec<f64>), PlsPermutationError> {
    if focal_index >= predictors.len() {
        return Err(PlsPermutationError::Regression(format!(
            "focal predictor index is out of range for {subject}"
        )));
    }
    let (full_coefficients, _) = ols_with_intercept(predictors, outcome, subject)?;
    let nuisance = predictors
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != focal_index)
        .map(|(_, predictor)| predictor.clone())
        .collect::<Vec<_>>();
    let (_, fitted_nuisance) =
        ols_with_intercept(&nuisance, outcome, &format!("nuisance model for {subject}"))?;
    let residuals = outcome
        .iter()
        .zip(&fitted_nuisance)
        .map(|(actual, fitted)| actual - fitted)
        .collect::<Vec<_>>();
    Ok((full_coefficients[focal_index], fitted_nuisance, residuals))
}

fn permuted_freedman_lane_focal_coefficient(
    predictors: &[Vec<f64>],
    fitted_nuisance: &[f64],
    residuals: &[f64],
    focal_index: usize,
    permutation_indices: &[usize],
    subject: &str,
) -> Result<f64, PlsPermutationError> {
    let permuted_outcome = fitted_nuisance
        .iter()
        .enumerate()
        .map(|(row, fitted)| fitted + residuals[permutation_indices[row]])
        .collect::<Vec<_>>();
    let (coefficients, _) = ols_with_intercept(predictors, &permuted_outcome, subject)?;
    Ok(coefficients[focal_index])
}

/// Computes one Freedman-Lane focal-path coefficient for an explicit
/// permutation of complete-case construct-score rows. This non-persisted seam
/// is intended for independent scientific reference checks and uses the same
/// preparation and regression helpers as the production permutation engine.
pub fn freedman_lane_focal_coefficient(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    focal_index: usize,
    permutation_indices: &[usize],
) -> Result<f64, PlsPermutationError> {
    if permutation_indices.len() != outcome.len() {
        return Err(PlsPermutationError::Regression(
            "permutation index count differs from the outcome length".into(),
        ));
    }
    let mut seen = vec![false; outcome.len()];
    for &index in permutation_indices {
        if index >= outcome.len() || seen[index] {
            return Err(PlsPermutationError::Regression(
                "permutation indices must be an exact zero-based bijection".into(),
            ));
        }
        seen[index] = true;
    }
    if seen.iter().any(|value| !value) {
        return Err(PlsPermutationError::Regression(
            "permutation indices must be an exact zero-based bijection".into(),
        ));
    }
    let (_, fitted_nuisance, residuals) =
        prepare_freedman_lane(predictors, outcome, focal_index, "reference focal path")?;
    permuted_freedman_lane_focal_coefficient(
        predictors,
        &fitted_nuisance,
        &residuals,
        focal_index,
        permutation_indices,
        "reference focal path",
    )
}

pub fn permutation_pls(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlsPermutationResult, PlsPermutationError> {
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .map_err(|error| PlsPermutationError::InconsistentResult(error.to_string()))?;
    permutation_pls_validated(
        dataset,
        &execution,
        original,
        workers,
        is_cancelled,
        report_progress,
    )
}

/// Permutes an opaque schema-v3 recipe capability whose configuration and
/// compatibility projection cannot be forged by callers.
pub fn permutation_pls_validated(
    dataset: &Dataset,
    recipe: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlsPermutationResult, PlsPermutationError> {
    let effective_recipe = recipe
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| PlsPermutationError::InconsistentResult(error.to_string()))?;
    let recipe = effective_recipe;
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlsPermutationError::RawDataRequired);
    }
    if recipe.settings.method != AnalysisMethod::PlsPm {
        return Err(PlsPermutationError::InvalidMethod);
    }
    if !(99..=10_000).contains(&recipe.settings.permutation_samples) {
        return Err(PlsPermutationError::InvalidPermutationCount);
    }
    if !original.converged || original.method_version != qpls_estimation::PLS_METHOD_VERSION {
        return Err(PlsPermutationError::InconsistentResult(
            "base estimate is not a converged PLS-PM v1 result".into(),
        ));
    }
    let complete_rows = complete_case_rows(dataset, recipe);
    if complete_rows.len() != original.used_observations {
        return Err(PlsPermutationError::InconsistentResult(
            "base estimate observation count differs from the complete-case sample".into(),
        ));
    }
    let case_count = original.used_observations;
    let mut setups = Vec::with_capacity(original.paths.len());
    for path in &original.paths {
        let incoming = recipe
            .model
            .paths
            .iter()
            .filter(|candidate| candidate.target == path.target)
            .collect::<Vec<_>>();
        let focal_index = incoming
            .iter()
            .position(|candidate| candidate.source == path.source)
            .ok_or_else(|| {
                PlsPermutationError::InconsistentResult(format!(
                    "missing recipe path '{} -> {}'",
                    path.source, path.target
                ))
            })?;
        let outcome = original.construct_scores.get(&path.target).ok_or_else(|| {
            PlsPermutationError::InconsistentResult(format!(
                "missing target score '{}'",
                path.target
            ))
        })?;
        let predictors = incoming
            .iter()
            .map(|candidate| {
                original
                    .construct_scores
                    .get(&candidate.source)
                    .cloned()
                    .ok_or_else(|| {
                        PlsPermutationError::InconsistentResult(format!(
                            "missing predictor score '{}'",
                            candidate.source
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if outcome.len() != case_count
            || predictors
                .iter()
                .any(|predictor| predictor.len() != case_count)
        {
            return Err(PlsPermutationError::InconsistentResult(
                "construct-score length differs from the complete-case sample".into(),
            ));
        }
        let (reproduced, fitted_nuisance, residuals) = prepare_freedman_lane(
            &predictors,
            outcome,
            focal_index,
            &format!("full model for {} -> {}", path.source, path.target),
        )?;
        if (reproduced - path.coefficient).abs()
            > 1e-10 * reproduced.abs().max(path.coefficient.abs()).max(1.0)
        {
            return Err(PlsPermutationError::InconsistentResult(format!(
                "full structural regression does not reproduce path '{} -> {}'",
                path.source, path.target
            )));
        }
        setups.push(StructuralPermutationSetup {
            parameter: parameter_key("path", &[&path.source, &path.target]),
            original: path.coefficient,
            focal_index,
            predictors,
            fitted_nuisance,
            residuals,
        });
    }
    if setups.is_empty() {
        return Err(PlsPermutationError::InconsistentResult(
            "structural model contains no paths".into(),
        ));
    }
    let plan = PermutationPlan {
        permutations: recipe.settings.permutation_samples,
        master_seed: recipe.settings.seed,
        operation: "pls_pm_freedman_lane_v1".into(),
    };
    let cancellation = &is_cancelled;
    let run = run_permutation(
        case_count,
        &plan,
        workers,
        |permutation_index| {
            let mut coefficients = std::collections::BTreeMap::new();
            for setup in &setups {
                if cancellation() {
                    return Err("cancelled".to_owned());
                }
                let operation = format!("{}:{}", plan.operation, setup.parameter);
                let indices = permutation_indices(
                    case_count,
                    plan.master_seed,
                    &operation,
                    permutation_index,
                );
                let estimate = permuted_freedman_lane_focal_coefficient(
                    &setup.predictors,
                    &setup.fitted_nuisance,
                    &setup.residuals,
                    setup.focal_index,
                    &indices,
                    &setup.parameter,
                )
                .map_err(|error| error.to_string())?;
                coefficients.insert(setup.parameter.clone(), estimate);
            }
            Ok::<_, String>(coefficients)
        },
        cancellation,
        report_progress,
    )?;
    let successful = run
        .outcomes
        .iter()
        .map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Ok(value),
            ReplicateOutcome::Failed { message } => {
                Err(PlsPermutationError::Regression(message.clone()))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let parameters = setups
        .iter()
        .map(|setup| {
            let exceedances = successful
                .iter()
                .filter(|values| values[&setup.parameter].abs() >= setup.original.abs())
                .count() as u32;
            PermutationParameterInference {
                parameter: setup.parameter.clone(),
                original: setup.original,
                exceedances,
                p_value_two_sided: (exceedances as f64 + 1.0)
                    / (run.plan.permutations as f64 + 1.0),
                permutations: run.plan.permutations,
            }
        })
        .collect();
    Ok(PlsPermutationResult {
        method_version: run.method_version,
        plan: run.plan,
        parameters,
    })
}

fn ols_with_intercept(
    predictors: &[Vec<f64>],
    outcome: &[f64],
    subject: &str,
) -> Result<(Vec<f64>, Vec<f64>), PlsPermutationError> {
    let rows = outcome.len();
    let columns = predictors.len() + 1;
    if rows <= columns || predictors.iter().any(|predictor| predictor.len() != rows) {
        return Err(PlsPermutationError::Regression(format!(
            "insufficient or inconsistent observations for {subject}"
        )));
    }
    let matrix = Mat::from_fn(rows, columns, |row, column| {
        if column == 0 {
            1.0
        } else {
            predictors[column - 1][row]
        }
    });
    let qr = matrix.col_piv_qr();
    let diagonal = qr.thin_R();
    let max_diagonal = (0..columns)
        .map(|index| diagonal[(index, index)].abs())
        .fold(0.0, f64::max);
    let tolerance = max_diagonal * rows.max(columns) as f64 * f64::EPSILON * 100.0;
    let rank = (0..columns)
        .filter(|index| diagonal[(*index, *index)].abs() > tolerance)
        .count();
    if rank < columns {
        return Err(PlsPermutationError::Regression(format!(
            "rank-deficient design for {subject}"
        )));
    }
    let rhs = Mat::from_fn(rows, 1, |row, _| outcome[row]);
    let solution = qr.solve_lstsq(&rhs);
    let intercept = solution[(0, 0)];
    let coefficients = (1..columns)
        .map(|column| solution[(column, 0)])
        .collect::<Vec<_>>();
    if !intercept.is_finite() || coefficients.iter().any(|value| !value.is_finite()) {
        return Err(PlsPermutationError::Regression(format!(
            "non-finite regression for {subject}"
        )));
    }
    let fitted = (0..rows)
        .map(|row| {
            intercept
                + predictors
                    .iter()
                    .zip(&coefficients)
                    .map(|(predictor, coefficient)| predictor[row] * coefficient)
                    .sum::<f64>()
        })
        .collect();
    Ok((coefficients, fitted))
}

fn inner_bootstrap_standard_errors(
    primary_dataset: &Dataset,
    recipe: &ValidatedExecutionRecipe,
    primary: &PlsResult,
    master_seed: u64,
    primary_replicate: u32,
    inner_replicates: u32,
    is_cancelled: &(impl Fn() -> bool + Sync),
    report_progress: &(impl Fn() + Sync),
) -> Result<Option<std::collections::BTreeMap<String, f64>>, EstimationError> {
    let case_count = primary.used_observations;
    let operation = format!("pls_pm_studentized_inner_v1:{primary_replicate}");
    let mut successful = Vec::with_capacity(inner_replicates as usize);
    for inner_replicate in 0..inner_replicates {
        if is_cancelled() {
            return Err(EstimationError::Cancelled);
        }
        let indices = bootstrap_indices(case_count, master_seed, &operation, inner_replicate);
        let sampled =
            resample_model_dataset(primary_dataset, recipe.effective(), &indices, is_cancelled)?;
        let estimate = estimate_pls_validated_with_control(&sampled, recipe, |_| !is_cancelled());
        report_progress();
        let mut estimate = match estimate {
            Ok(estimate) => estimate,
            Err(EstimationError::Cancelled) => return Err(EstimationError::Cancelled),
            Err(_) => continue,
        };
        align_pls_signs(
            &mut estimate,
            &primary.construct_scores,
            &indices,
            is_cancelled,
        )?;
        successful.push(result_values(
            &estimate.outer_estimates,
            &estimate.paths,
            &estimate.effects,
            &estimate.r_squared,
        ));
    }
    let required = ((inner_replicates as f64 * 0.9).ceil() as usize).max(2);
    if successful.len() < required {
        return Ok(None);
    }
    let primary_values = result_values(
        &primary.outer_estimates,
        &primary.paths,
        &primary.effects,
        &primary.r_squared,
    );
    let standard_errors = primary_values
        .iter()
        .map(|(parameter, primary_value)| {
            let values = successful
                .iter()
                .map(|estimate| {
                    estimate.get(parameter).copied().ok_or_else(|| {
                        EstimationError::Numerical(format!(
                            "studentized inner estimate is missing parameter {parameter}"
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let standard_error = (values
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>()
                / (values.len() - 1) as f64)
                .sqrt();
            if !standard_error.is_finite() {
                return Err(EstimationError::Numerical(format!(
                    "studentized inner standard error is non-finite for {parameter}"
                )));
            }
            let threshold = numerical_zero_tolerance(*primary_value, values.iter().copied());
            Ok((
                parameter.clone(),
                if standard_error <= threshold {
                    0.0
                } else {
                    standard_error
                },
            ))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    Ok(Some(standard_errors))
}

fn summarize_percentile(
    original: &PlsResult,
    run: &BootstrapRun<PlsBootstrapEstimate>,
    confidence_level: f64,
) -> Result<PercentileInference, PlsBootstrapError> {
    let original_values = result_values(
        &original.outer_estimates,
        &original.paths,
        &original.effects,
        &original.r_squared,
    );
    let successful = run
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(result_values(
                &value.outer_estimates,
                &value.paths,
                &value.effects,
                &value.r_squared,
            )),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    if successful.len() < 2 {
        return Err(PlsBootstrapError::InsufficientUsableReplicates {
            usable: successful.len(),
            required: 2,
        });
    }
    let tail = (1.0 - confidence_level) / 2.0;
    let mut parameters = Vec::with_capacity(original_values.len());
    for (parameter, original) in original_values {
        let mut values = successful
            .iter()
            .map(|replicate| {
                replicate
                    .get(&parameter)
                    .copied()
                    .ok_or_else(|| PlsBootstrapError::InconsistentResult(parameter.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        values.sort_by(f64::total_cmp);
        let bootstrap_mean = values.iter().sum::<f64>() / values.len() as f64;
        let standard_error = (values
            .iter()
            .map(|value| (value - bootstrap_mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64)
            .sqrt();
        let (t_statistic, p_value_two_sided) = normal_reference_test(original, standard_error);
        parameters.push(BootstrapParameterInference {
            parameter,
            original,
            bootstrap_mean,
            bias: bootstrap_mean - original,
            standard_error,
            lower: type7_quantile(&values, tail),
            upper: type7_quantile(&values, 1.0 - tail),
            usable_replicates: values.len() as u32,
            t_statistic,
            p_value_two_sided,
        });
    }
    Ok(PercentileInference {
        confidence_level,
        parameters,
    })
}

fn summarize_studentized(
    original: &PlsResult,
    run: &BootstrapRun<PlsBootstrapEstimate>,
    percentile: &PercentileInference,
    confidence_level: f64,
    inner_replicates: u32,
) -> Result<StudentizedInference, PlsBootstrapError> {
    let nested_failures = run
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => value
                .studentized_error
                .as_ref()
                .map(|message| (value.replicate_index, message)),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    if let Some((first_primary_replicate, message)) = nested_failures.first() {
        return Ok(StudentizedInference {
            method_version: STUDENTIZED_METHOD_VERSION.into(),
            confidence_level,
            inner_replicates,
            minimum_usable_fraction: 0.9,
            stream_domain: "pls_pm_studentized_inner_v1".into(),
            failure: Some(StudentizedFailure {
                reason_code: "nested_infrastructure_failure".into(),
                first_primary_replicate: *first_primary_replicate,
                failed_primary_replicates: nested_failures.len() as u32,
                message: (*message).clone(),
            }),
            parameters: Vec::new(),
        });
    }
    let original_values = result_values(
        &original.outer_estimates,
        &original.paths,
        &original.effects,
        &original.r_squared,
    );
    let original_standard_errors = percentile
        .parameters
        .iter()
        .map(|parameter| (parameter.parameter.as_str(), parameter.standard_error))
        .collect::<std::collections::HashMap<_, _>>();
    let successful = run
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let replicate_values = successful
        .iter()
        .map(|estimate| {
            result_values(
                &estimate.outer_estimates,
                &estimate.paths,
                &estimate.effects,
                &estimate.r_squared,
            )
        })
        .collect::<Vec<_>>();
    let mut parameters = Vec::with_capacity(original_values.len());
    let required_primary = ((run.plan.replicates as f64 * 0.9).ceil() as usize).max(2);
    for (parameter, original) in original_values {
        let original_standard_error = original_standard_errors
            .get(parameter.as_str())
            .copied()
            .ok_or_else(|| PlsBootstrapError::InconsistentResult(parameter.clone()))?;
        let mut statistics = Vec::new();
        let outer_scale = replicate_values
            .iter()
            .filter_map(|values| values.get(&parameter).copied())
            .fold(original.abs().max(1.0), |scale, value| {
                scale.max(value.abs())
            });
        for (estimate, values) in successful.iter().zip(&replicate_values) {
            let Some(inner_standard_errors) = estimate.studentized_standard_errors.as_ref() else {
                continue;
            };
            let value = values
                .get(&parameter)
                .copied()
                .ok_or_else(|| PlsBootstrapError::InconsistentResult(parameter.clone()))?;
            let inner_standard_error = inner_standard_errors
                .get(&parameter)
                .copied()
                .ok_or_else(|| PlsBootstrapError::InconsistentResult(parameter.clone()))?;
            if inner_standard_error > 0.0 {
                let statistic = (value - original) / inner_standard_error;
                if statistic.is_finite() {
                    statistics.push(statistic);
                }
            }
        }
        let interval = (statistics.len() >= required_primary)
            .then(|| {
                studentized_interval(
                    original,
                    original_standard_error,
                    &statistics,
                    confidence_level,
                    outer_scale,
                )
            })
            .flatten();
        let reason = if statistics.len() < required_primary {
            "insufficient_pivots"
        } else if original_standard_error <= 64.0 * f64::EPSILON * outer_scale {
            "zero_outer_standard_error"
        } else {
            "invalid_bounds"
        };
        parameters.push(match interval {
            Some(interval) => StudentizedParameterInference {
                parameter,
                original,
                outer_standard_error: original_standard_error,
                outer_scale,
                usable_primary_replicates: statistics.len() as u32,
                lower_pivot: Some(interval.lower_pivot),
                upper_pivot: Some(interval.upper_pivot),
                lower: Some(interval.lower),
                upper: Some(interval.upper),
                unavailable_reason: None,
            },
            None => StudentizedParameterInference {
                parameter,
                original,
                outer_standard_error: original_standard_error,
                outer_scale,
                usable_primary_replicates: statistics.len() as u32,
                lower_pivot: None,
                upper_pivot: None,
                lower: None,
                upper: None,
                unavailable_reason: Some(reason.into()),
            },
        });
    }
    Ok(StudentizedInference {
        method_version: STUDENTIZED_METHOD_VERSION.into(),
        confidence_level,
        inner_replicates,
        minimum_usable_fraction: 0.9,
        stream_domain: "pls_pm_studentized_inner_v1".into(),
        failure: None,
        parameters,
    })
}

fn studentized_interval(
    original: f64,
    original_standard_error: f64,
    studentized_statistics: &[f64],
    confidence_level: f64,
    outer_scale: f64,
) -> Option<StudentizedIntervalValues> {
    if !original.is_finite()
        || !original_standard_error.is_finite()
        || !outer_scale.is_finite()
        || outer_scale < original.abs().max(1.0)
        || original_standard_error <= 64.0 * f64::EPSILON * outer_scale
        || studentized_statistics.len() < 2
        || studentized_statistics
            .iter()
            .any(|statistic| !statistic.is_finite())
        || !confidence_level.is_finite()
        || !(0.0..1.0).contains(&confidence_level)
    {
        return None;
    }
    let mut sorted = studentized_statistics.to_vec();
    sorted.sort_by(f64::total_cmp);
    let tail = (1.0 - confidence_level) / 2.0;
    let lower_statistic = type7_quantile(&sorted, tail);
    let upper_statistic = type7_quantile(&sorted, 1.0 - tail);
    let lower = original - upper_statistic * original_standard_error;
    let upper = original - lower_statistic * original_standard_error;
    (lower.is_finite() && upper.is_finite() && lower <= upper).then_some(
        StudentizedIntervalValues {
            lower_pivot: lower_statistic,
            upper_pivot: upper_statistic,
            lower,
            upper,
        },
    )
}

fn numerical_zero_tolerance(center: f64, values: impl IntoIterator<Item = f64>) -> f64 {
    let scale = values
        .into_iter()
        .fold(center.abs().max(1.0), |scale, value| scale.max(value.abs()));
    64.0 * f64::EPSILON * scale
}

fn summarize_bca(
    original: &PlsResult,
    bootstrap: &BootstrapRun<PlsBootstrapEstimate>,
    jackknife: &JackknifeRun<PlsJackknifeEstimate>,
    confidence_level: f64,
) -> Result<BcaInference, PlsBootstrapError> {
    let original_values = result_values(
        &original.outer_estimates,
        &original.paths,
        &original.effects,
        &original.r_squared,
    );
    let bootstrap_values = bootstrap
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(result_values(
                &value.outer_estimates,
                &value.paths,
                &value.effects,
                &value.r_squared,
            )),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let jackknife_values = jackknife
        .outcomes
        .iter()
        .map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Ok(&value.parameters),
            ReplicateOutcome::Failed { message } => Err(PlsBootstrapError::Jackknife(format!(
                "a delete-one estimate failed: {message}"
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut parameters = Vec::with_capacity(original_values.len());
    for (parameter, original) in original_values {
        let replicates = bootstrap_values
            .iter()
            .map(|values| {
                values
                    .get(&parameter)
                    .copied()
                    .ok_or_else(|| PlsBootstrapError::InconsistentResult(parameter.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let delete_one = jackknife_values
            .iter()
            .map(|values| {
                values
                    .get(&parameter)
                    .copied()
                    .ok_or_else(|| PlsBootstrapError::InconsistentResult(parameter.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let interval = bca_interval(&replicates, original, &delete_one, confidence_level);
        parameters.push(match interval {
            Some(interval) => BcaParameterInference {
                parameter,
                bias_correction: Some(interval.bias_correction),
                acceleration: Some(interval.acceleration),
                lower: Some(interval.lower),
                upper: Some(interval.upper),
                unavailable_reason: None,
            },
            None => BcaParameterInference {
                parameter,
                bias_correction: None,
                acceleration: None,
                lower: None,
                upper: None,
                unavailable_reason: Some(
                    "BCa is unavailable because the delete-one acceleration adjustment is numerically undefined"
                        .into(),
                ),
            },
        });
    }
    Ok(BcaInference {
        confidence_level,
        jackknife_case_count: jackknife.case_count,
        parameters,
    })
}

pub fn bca_interval(
    bootstrap_values: &[f64],
    original: f64,
    jackknife_values: &[f64],
    confidence_level: f64,
) -> Option<BcaIntervalValues> {
    if bootstrap_values.len() < 2
        || jackknife_values.len() < 3
        || !original.is_finite()
        || !confidence_level.is_finite()
        || !(0.0..1.0).contains(&confidence_level)
        || bootstrap_values.iter().any(|value| !value.is_finite())
        || jackknife_values.iter().any(|value| !value.is_finite())
    {
        return None;
    }

    let replicate_count = bootstrap_values.len() as f64;
    let below = bootstrap_values
        .iter()
        .filter(|value| **value < original)
        .count() as f64;
    let tied = bootstrap_values
        .iter()
        .filter(|value| **value == original)
        .count() as f64;
    let probability = ((below + 0.5 * tied) / replicate_count)
        .clamp(0.5 / replicate_count, 1.0 - 0.5 / replicate_count);
    let normal = Normal::standard();
    let bias_correction = normal.inverse_cdf(probability);

    let jackknife_mean = jackknife_values.iter().sum::<f64>() / jackknife_values.len() as f64;
    let centered = jackknife_values
        .iter()
        .map(|value| jackknife_mean - value)
        .collect::<Vec<_>>();
    let sum_squares = centered.iter().map(|value| value.powi(2)).sum::<f64>();
    if !sum_squares.is_finite() || sum_squares <= f64::EPSILON {
        return None;
    }
    let acceleration =
        centered.iter().map(|value| value.powi(3)).sum::<f64>() / (6.0 * sum_squares.powf(1.5));
    if !acceleration.is_finite() {
        return None;
    }

    let tail = (1.0 - confidence_level) / 2.0;
    let adjusted_probability = |nominal: f64| {
        let z = normal.inverse_cdf(nominal);
        let denominator = 1.0 - acceleration * (bias_correction + z);
        if !denominator.is_finite() || denominator.abs() <= f64::EPSILON {
            return None;
        }
        let adjusted = normal.cdf(bias_correction + (bias_correction + z) / denominator);
        adjusted.is_finite().then_some(adjusted.clamp(0.0, 1.0))
    };
    let lower_probability = adjusted_probability(tail)?;
    let upper_probability = adjusted_probability(1.0 - tail)?;
    if lower_probability > upper_probability {
        return None;
    }
    let mut sorted = bootstrap_values.to_vec();
    sorted.sort_by(f64::total_cmp);
    Some(BcaIntervalValues {
        bias_correction,
        acceleration,
        lower: type7_quantile(&sorted, lower_probability),
        upper: type7_quantile(&sorted, upper_probability),
    })
}

pub fn normal_reference_test(original: f64, standard_error: f64) -> (Option<f64>, Option<f64>) {
    if !original.is_finite() || !standard_error.is_finite() || standard_error <= f64::EPSILON {
        return (None, None);
    }
    let statistic = original / standard_error;
    if !statistic.is_finite() {
        return (None, None);
    }
    let probability = 2.0 * Normal::standard().sf(statistic.abs());
    (Some(statistic), Some(probability.clamp(0.0, 1.0)))
}

fn result_values(
    outer_estimates: &[OuterEstimate],
    paths: &[PathEstimate],
    effects: &[EffectEstimate],
    r_squared: &std::collections::BTreeMap<String, f64>,
) -> std::collections::BTreeMap<String, f64> {
    let mut values = std::collections::BTreeMap::new();
    for outer in outer_estimates {
        values.insert(
            parameter_key("outer_loading", &[&outer.construct, &outer.indicator]),
            outer.loading,
        );
        values.insert(
            parameter_key("outer_weight", &[&outer.construct, &outer.indicator]),
            outer.weight,
        );
    }
    for path in paths {
        values.insert(
            parameter_key("path", &[&path.source, &path.target]),
            path.coefficient,
        );
    }
    for effect in effects {
        let parts = [effect.source.as_str(), effect.target.as_str()];
        values.insert(parameter_key("direct_effect", &parts), effect.direct);
        values.insert(parameter_key("indirect_effect", &parts), effect.indirect);
        values.insert(parameter_key("total_effect", &parts), effect.total);
    }
    for (construct, value) in r_squared {
        values.insert(parameter_key("r_squared", &[construct]), *value);
    }
    values
}

fn parameter_key(kind: &str, parts: &[&str]) -> String {
    serde_json::to_string(&(kind, parts)).expect("bootstrap parameter identity is serializable")
}

fn type7_quantile(sorted: &[f64], probability: f64) -> f64 {
    let position = (sorted.len() - 1) as f64 * probability;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] + (position - lower as f64) * (sorted[upper] - sorted[lower])
    }
}

fn resample_model_dataset(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    indices: &[usize],
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<Dataset, EstimationError> {
    let indicator_names = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| &construct.indicators)
        .cloned()
        .collect::<Vec<_>>();
    resample_dataset_columns(dataset, &indicator_names, indices, is_cancelled)
}

fn resample_dataset_columns(
    dataset: &Dataset,
    column_names: &[String],
    indices: &[usize],
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<Dataset, EstimationError> {
    if indices.iter().any(|index| *index > u32::MAX as usize) {
        return Err(EstimationError::Numerical(
            "bootstrap row index exceeds Arrow UInt32 capacity".into(),
        ));
    }
    let indices = UInt32Array::from(
        indices
            .iter()
            .map(|index| *index as u32)
            .collect::<Vec<_>>(),
    );
    let mut columns = Vec::with_capacity(column_names.len());
    for column_name in column_names {
        if is_cancelled() {
            return Err(EstimationError::Cancelled);
        }
        let position = dataset
            .batch
            .schema()
            .index_of(column_name)
            .map_err(|_| EstimationError::InvalidIndicator(column_name.clone()))?;
        let values = take(dataset.batch.column(position).as_ref(), &indices, None)
            .map_err(|error| EstimationError::Numerical(error.to_string()))?;
        columns.push((column_name.clone(), values));
    }
    let batch = RecordBatch::try_from_iter(columns)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    let mut schema = dataset.schema.clone();
    schema.case_count = batch.num_rows();
    schema.columns.retain(|column| {
        column_names
            .iter()
            .any(|column_name| column_name == &column.name)
    });
    Ok(Dataset {
        id: dataset.id,
        name: dataset.name.clone(),
        schema,
        batch,
        fingerprint: dataset.fingerprint.clone(),
    })
}

fn complete_case_rows_at_positions(dataset: &Dataset, positions: &[usize]) -> Vec<usize> {
    (0..dataset.batch.num_rows())
        .filter(|row| {
            positions.iter().all(|position| {
                let array = dataset.batch.column(*position);
                !array.is_null(*row)
                    && numeric_value(array.as_ref(), *row).is_some_and(f64::is_finite)
            })
        })
        .collect()
}

fn complete_case_rows(dataset: &Dataset, recipe: &AnalysisRecipe) -> Vec<usize> {
    let positions = recipe
        .model
        .constructs
        .iter()
        .flat_map(|construct| &construct.indicators)
        .filter_map(|indicator| dataset.batch.schema().index_of(indicator).ok())
        .collect::<Vec<_>>();
    (0..dataset.batch.num_rows())
        .filter(|row| {
            positions.iter().all(|position| {
                let array = dataset.batch.column(*position);
                !array.is_null(*row)
                    && numeric_value(array.as_ref(), *row).is_some_and(f64::is_finite)
            })
        })
        .collect()
}

fn align_pls_signs(
    estimate: &mut qpls_estimation::PlsResult,
    original_scores: &std::collections::BTreeMap<String, Vec<f64>>,
    sampled_indices: &[usize],
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<(), EstimationError> {
    let mut signs = std::collections::HashMap::new();
    for (construct, replicate_scores) in &estimate.construct_scores {
        if is_cancelled() {
            return Err(EstimationError::Cancelled);
        }
        let original = original_scores.get(construct).ok_or_else(|| {
            EstimationError::Numerical(format!("missing original score for {construct}"))
        })?;
        let aligned_reference = sampled_indices
            .iter()
            .map(|position| original[*position])
            .collect::<Vec<_>>();
        if aligned_reference.len() != replicate_scores.len() {
            return Err(EstimationError::Numerical(format!(
                "bootstrap score alignment length mismatch for {construct}"
            )));
        }
        signs.insert(
            construct.clone(),
            if covariance(&aligned_reference, replicate_scores) < 0.0 {
                -1.0
            } else {
                1.0
            },
        );
    }
    for outer in &mut estimate.outer_estimates {
        let sign = signs[&outer.construct];
        outer.weight *= sign;
        outer.loading *= sign;
    }
    for path in &mut estimate.paths {
        path.coefficient *= signs[&path.source] * signs[&path.target];
    }
    for effect in &mut estimate.effects {
        let sign = signs[&effect.source] * signs[&effect.target];
        effect.direct *= sign;
        effect.indirect *= sign;
        effect.total *= sign;
    }
    Ok(())
}

fn numeric_value(array: &dyn Array, row: usize) -> Option<f64> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        Some(values.value(row))
    } else {
        array
            .as_any()
            .downcast_ref::<Int64Array>()
            .map(|values| values.value(row) as f64)
    }
}

fn covariance(left: &[f64], right: &[f64]) -> f64 {
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>()
}

fn derive_seed(master_seed: u64, operation: &str, replicate_index: u32) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(SEED_DOMAIN);
    digest.update(master_seed.to_le_bytes());
    digest.update((operation.len() as u64).to_le_bytes());
    digest.update(operation.as_bytes());
    digest.update(replicate_index.to_le_bytes());
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisSettings, Construct, MeasurementMode, ModelSpec,
        StructuralPath,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{Arc, atomic::AtomicBool};
    use uuid::Uuid;

    fn current_recipe(mut recipe: AnalysisRecipe) -> AnalysisRecipe {
        if recipe.schema_version < ANALYSIS_RECIPE_SCHEMA_VERSION {
            recipe = recipe.migrated_v3().unwrap();
        }
        recipe
    }

    fn regression_bootstrap_test_recipe(
        dataset: &Dataset,
        model: RegressionModelConfig,
        replicates: u32,
        seed: u64,
    ) -> AnalysisRecipe {
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/v08_regression_logistic.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.preprocessing = qpls_core::Preprocessing::Unstandardized;
        recipe.settings.bootstrap_samples = replicates;
        recipe.settings.seed = seed;
        recipe.settings.workers = 1;
        recipe.method_config = Some(MethodConfig::Regression {
            outcome: "y".into(),
            predictors: vec!["x".into()],
            controls: Vec::new(),
            model,
            bootstrap: Some(qpls_core::RegressionBootstrapConfig {
                algorithm: RegressionBootstrapAlgorithm::CaseResampling,
                intervals: vec![
                    RegressionBootstrapInterval::Percentile,
                    RegressionBootstrapInterval::Bca,
                ],
            }),
        });
        recipe
    }

    fn regression_bootstrap_test_original(
        dataset: &Dataset,
        recipe: &AnalysisRecipe,
    ) -> (ValidatedExecutionRecipe, PlsResult) {
        let execution =
            ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0).unwrap();
        let point_only = execution.without_outer_resampling().unwrap();
        let original = estimate_pls_validated_with_control(dataset, &point_only, |_| true).unwrap();
        (execution, original)
    }

    fn process_graph_bootstrap_fixture(workers: usize) -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/process_v2_reference_fixture.csv"),
            "process-v2-worker-invariance.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/process_v2_reference.recipe.json"
        ))
        .unwrap();
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 99;
        recipe.settings.workers = workers;
        (dataset, recipe)
    }

    fn multi_path_permutation_fixture(seed: u64) -> (Dataset, AnalysisRecipe, PlsResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/corporate_reputation.csv"),
            "corporate_reputation.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/corporate_reputation.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.permutation_samples = 199;
        recipe.settings.seed = seed;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsPermutation);
        let base_recipe = ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0)
            .unwrap()
            .without_outer_resampling()
            .unwrap();
        let original =
            estimate_pls_validated_with_control(&dataset, &base_recipe, |_| true).unwrap();
        (dataset, recipe, original)
    }

    #[test]
    fn process_graph_v2_case_bootstrap_is_worker_invariant_and_bca_conditional() {
        let (dataset, recipe_one) = process_graph_bootstrap_fixture(1);
        let execution_one =
            ValidatedExecutionRecipe::for_dataset(&recipe_one, &dataset.fingerprint.0).unwrap();
        let point_one = execution_one.without_outer_resampling().unwrap();
        let original_one =
            estimate_pls_validated_with_control(&dataset, &point_one, |_| true).unwrap();
        let one = bootstrap_process_validated(
            &dataset,
            &execution_one,
            &original_one,
            1,
            || false,
            |_| {},
        )
        .unwrap();

        let (_, mut recipe_four) = process_graph_bootstrap_fixture(4);
        recipe_four.dataset_fingerprint = dataset.fingerprint.0.clone();
        let execution_four =
            ValidatedExecutionRecipe::for_dataset(&recipe_four, &dataset.fingerprint.0).unwrap();
        let point_four = execution_four.without_outer_resampling().unwrap();
        let original_four =
            estimate_pls_validated_with_control(&dataset, &point_four, |_| true).unwrap();
        let four = bootstrap_process_validated(
            &dataset,
            &execution_four,
            &original_four,
            4,
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(
            original_one
                .regression
                .as_ref()
                .unwrap()
                .process
                .as_ref()
                .unwrap()
                .graph_v2
                .as_ref(),
            original_four
                .regression
                .as_ref()
                .unwrap()
                .process
                .as_ref()
                .unwrap()
                .graph_v2
                .as_ref(),
        );
        let mut normalized_four = four.clone();
        normalized_four.workers = one.workers;
        assert_eq!(one, normalized_four);
        assert_eq!(one.usable_replicates, 99);
        assert_eq!(one.jackknife_cases, original_one.used_observations);
        assert!(one.jackknife_cases >= 170);
        let exact_witness_values = (one.validation_witness.successful_bootstrap.len()
            + one.validation_witness.successful_jackknife.len())
            * one.validation_witness.estimand_ids.len();
        assert!(exact_witness_values > 5_000);
        assert!(one.estimands.iter().all(|estimand| matches!(
            estimand.bca,
            RegressionBootstrapBcaInterval::Available { .. }
                | RegressionBootstrapBcaInterval::Unavailable { .. }
        )));
    }

    #[test]
    fn process_graph_v2_bootstrap_maps_high_leverage_hc3_failure() {
        let mapped = process_replicate_error(EstimationError::UnsupportedMethod(
            "high_leverage_hc3_instability|PROCESS equation Y has unstable leverage".into(),
        ));
        assert!(mapped.starts_with("high_leverage_hc3_instability|"));
        assert!(mapped.contains("PROCESS equation Y has unstable leverage"));
    }

    #[test]
    fn process_graph_v2_bootstrap_maps_invalid_hc3_covariance_failure() {
        let mapped = process_replicate_error(EstimationError::Numerical(
            "invalid_hc3_covariance|PROCESS equation Y has a negative diagonal".into(),
        ));
        assert!(mapped.starts_with("invalid_hc3_covariance|"));
    }

    #[test]
    fn process_graph_v2_bootstrap_maps_degenerate_simple_slope_failure() {
        let mapped = process_replicate_error(EstimationError::Numerical(
            "degenerate_simple_slope_variance|PROCESS moderation X->Y@W is degenerate".into(),
        ));
        assert!(mapped.starts_with("degenerate_simple_slope_variance|"));
    }

    fn imbalanced_logistic_dataset(minority_cases: usize) -> Dataset {
        assert_eq!(minority_cases, 1);
        let outcomes = [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let predictors = [
            -2.0, -2.0, -2.0, -2.0, -1.0, -1.0, -1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0,
            2.0, 2.0, 2.0, 2.0,
        ];
        let mut csv = String::from("y,x\n");
        for (outcome, predictor) in outcomes.into_iter().zip(predictors) {
            csv.push_str(&format!("{outcome},{predictor}\n"));
        }
        import_delimited_bytes(
            csv.as_bytes(),
            &format!("regression-bootstrap-{minority_cases}-minority.csv"),
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn moderately_imbalanced_logistic_dataset() -> Dataset {
        let mut csv = String::from("y,x\n");
        for row in 0..40 {
            let outcome = usize::from(matches!(row, 0 | 6 | 12 | 18 | 24));
            let predictor = row as i32 % 5 - 2;
            csv.push_str(&format!("{outcome},{predictor}\n"));
        }
        import_delimited_bytes(
            csv.as_bytes(),
            "regression-bootstrap-five-minority.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    #[test]
    fn indexed_samples_are_repeatable_and_replicate_specific() {
        let first = bootstrap_indices(20, 42, "pls_bootstrap", 7);
        assert_eq!(first, bootstrap_indices(20, 42, "pls_bootstrap", 7));
        assert_ne!(first, bootstrap_indices(20, 42, "pls_bootstrap", 8));
        assert!(first.iter().all(|index| *index < 20));
    }

    #[test]
    fn indexed_permutations_are_bijections_and_replicate_specific() {
        let first = permutation_indices(20, 42, "path:x:y", 7);
        assert_eq!(first, permutation_indices(20, 42, "path:x:y", 7));
        assert_ne!(first, permutation_indices(20, 42, "path:x:y", 8));
        let mut sorted = first;
        sorted.sort_unstable();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());

        let frozen_operation = "pls_pm_freedman_lane_v1:[\"path\",[\"comp\",\"satisfaction\"]]";
        assert_eq!(
            permutation_indices(12, 20_260_718, frozen_operation, 0),
            vec![1, 8, 2, 9, 7, 6, 3, 10, 0, 11, 5, 4]
        );
        assert_eq!(
            permutation_indices(12, 20_260_718, frozen_operation, 98),
            vec![0, 5, 1, 10, 6, 2, 4, 7, 9, 11, 3, 8]
        );
    }

    #[test]
    fn generic_permutation_is_ordered_worker_invariant_and_cancellable() {
        let execute = |workers, progress: Arc<Mutex<Vec<ResamplingProgress>>>| {
            run_permutation(
                20,
                &PermutationPlan {
                    permutations: 99,
                    master_seed: 91,
                    operation: "fixture_permutation".into(),
                },
                workers,
                |index| Ok::<_, String>(permutation_indices(20, 91, "fixture", index)),
                || false,
                |update| progress.lock().unwrap().push(update),
            )
            .unwrap()
        };
        let serial_progress = Arc::new(Mutex::new(Vec::new()));
        let parallel_progress = Arc::new(Mutex::new(Vec::new()));
        let serial = execute(1, serial_progress.clone());
        let parallel = execute(4, parallel_progress.clone());
        assert_eq!(serial, parallel);
        let expected = (1..=99)
            .map(|completed_replicates| ResamplingProgress {
                phase: ResamplingPhase::Permutation,
                completed_replicates,
                total_replicates: 99,
            })
            .collect::<Vec<_>>();
        assert_eq!(*serial_progress.lock().unwrap(), expected);
        assert_eq!(*parallel_progress.lock().unwrap(), expected);
        assert_eq!(
            run_permutation(
                20,
                &PermutationPlan {
                    permutations: 99,
                    master_seed: 1,
                    operation: "cancel".into(),
                },
                1,
                |_| Ok::<_, String>(()),
                || true,
                |_| {},
            ),
            Err(ResamplingError::Cancelled)
        );
    }

    #[test]
    fn outputs_are_identical_across_worker_counts_and_progress_is_monotonic() {
        let run = |workers, progress: Arc<Mutex<Vec<u32>>>| {
            run_bootstrap(
                30,
                &BootstrapPlan {
                    replicates: 40,
                    master_seed: 20260718,
                    operation: "pls_bootstrap".into(),
                },
                workers,
                |_, indices| Ok::<_, String>(indices.iter().sum::<usize>()),
                || false,
                |update| progress.lock().unwrap().push(update.completed_replicates),
            )
            .unwrap()
        };
        let serial_progress = Arc::new(Mutex::new(Vec::new()));
        let parallel_progress = Arc::new(Mutex::new(Vec::new()));
        let serial = run(1, serial_progress.clone());
        let parallel = run(4, parallel_progress.clone());
        assert_eq!(serial.outcomes, parallel.outcomes);
        assert_eq!(
            *serial_progress.lock().unwrap(),
            (1..=40).collect::<Vec<_>>()
        );
        assert_eq!(
            *parallel_progress.lock().unwrap(),
            (1..=40).collect::<Vec<_>>()
        );
    }

    #[test]
    fn jackknife_is_ordered_and_exactly_worker_invariant() {
        let run = |workers, progress: Arc<Mutex<Vec<u32>>>| {
            run_jackknife(
                12,
                "fixture_jackknife",
                workers,
                |omitted_case| Ok::<_, String>(omitted_case * omitted_case),
                || false,
                |update| progress.lock().unwrap().push(update.completed_replicates),
            )
            .unwrap()
        };
        let serial_progress = Arc::new(Mutex::new(Vec::new()));
        let parallel_progress = Arc::new(Mutex::new(Vec::new()));
        let serial = run(1, serial_progress.clone());
        let parallel = run(4, parallel_progress.clone());
        assert_eq!(serial, parallel);
        assert_eq!(serial.method_version, JACKKNIFE_METHOD_VERSION);
        assert_eq!(
            *serial_progress.lock().unwrap(),
            (1..=12).collect::<Vec<_>>()
        );
        assert_eq!(
            *parallel_progress.lock().unwrap(),
            (1..=12).collect::<Vec<_>>()
        );
        for (index, outcome) in serial.outcomes.iter().enumerate() {
            assert_eq!(
                outcome,
                &ReplicateOutcome::Success {
                    value: index * index
                }
            );
        }
    }

    #[test]
    fn jackknife_rejects_invalid_plans_and_discards_cancelled_work() {
        assert_eq!(
            run_jackknife(2, "too_small", 1, |_| Ok::<_, String>(()), || false, |_| {}),
            Err(ResamplingError::InvalidPlan(
                "jackknife requires at least three cases".into()
            ))
        );
        assert_eq!(
            run_jackknife(10, "cancel", 2, |_| Ok::<_, String>(()), || true, |_| {}),
            Err(ResamplingError::Cancelled)
        );
    }

    #[test]
    fn cancellation_discards_partial_results() {
        let cancelled = AtomicBool::new(true);
        let result = run_bootstrap(
            10,
            &BootstrapPlan {
                replicates: 10,
                master_seed: 1,
                operation: "cancel".into(),
            },
            2,
            |_, _| Ok::<_, String>(()),
            || cancelled.load(Ordering::Relaxed),
            |_| {},
        );
        assert_eq!(result, Err(ResamplingError::Cancelled));
    }

    #[test]
    fn pls_bootstrap_is_exactly_invariant_to_worker_count() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 24;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsBootstrap);
        let mut base_recipe = recipe.clone();
        base_recipe.settings.bootstrap_samples = 0;
        base_recipe.method_config = Some(qpls_core::MethodConfig::PlsAlgorithm);
        let original = qpls_estimation::estimate_pls(&dataset, &base_recipe).unwrap();
        let progress = Arc::new(Mutex::new(Vec::new()));
        let serial_progress = progress.clone();
        let serial = bootstrap_pls(
            &dataset,
            &recipe,
            &original,
            1,
            || false,
            |update| {
                serial_progress.lock().unwrap().push((
                    update.phase,
                    update.completed_replicates,
                    update.total_replicates,
                ));
            },
        )
        .unwrap();
        let parallel = bootstrap_pls(&dataset, &recipe, &original, 4, || false, |_| {}).unwrap();
        assert_eq!(serial, parallel);
        assert_eq!(serial.method_version, RESAMPLING_METHOD_VERSION);
        assert_eq!(serial.usable_replicates, 24);
        assert!(serial.failed_replicates.is_empty());
        assert!(serial.percentile.parameters.iter().all(|parameter| {
            parameter.standard_error.is_finite()
                && parameter.lower.is_finite()
                && parameter.upper.is_finite()
                && parameter.lower <= parameter.upper
        }));
        let bca = serial.bca.as_ref().unwrap();
        assert_eq!(bca.jackknife_case_count, original.used_observations);
        assert_eq!(bca.parameters.len(), serial.percentile.parameters.len());
        assert!(
            bca.parameters
                .iter()
                .any(|parameter| parameter.lower.is_some())
        );
        assert!(bca.parameters.iter().all(|parameter| {
            let available = parameter.bias_correction.is_some()
                && parameter.acceleration.is_some()
                && parameter.lower.is_some()
                && parameter.upper.is_some()
                && parameter.unavailable_reason.is_none();
            let unavailable = parameter.bias_correction.is_none()
                && parameter.acceleration.is_none()
                && parameter.lower.is_none()
                && parameter.upper.is_none()
                && parameter.unavailable_reason.is_some();
            available || unavailable
        }));
        let progress = progress.lock().unwrap();
        assert_eq!(progress.len(), 24 + original.used_observations);
        assert!(progress[..24].iter().enumerate().all(|(index, update)| {
            *update == (ResamplingPhase::Bootstrap, index as u32 + 1, 24)
        }));
        assert!(progress[24..].iter().enumerate().all(|(index, update)| {
            *update
                == (
                    ResamplingPhase::Jackknife,
                    index as u32 + 1,
                    original.used_observations as u32,
                )
        }));
    }

    #[test]
    fn pls_bootstrap_carries_mediation_indirect_effect_inference() {
        let dataset = import_delimited_bytes(
            b"x,m,y\n1,2,3\n2,3,5\n3,5,8\n4,7,11\n5,11,16\n6,13,19\n7,17,24\n8,19,27\n",
            "mediation.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let model = ModelSpec {
            id: Uuid::nil(),
            name: "Mediation".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x".into()],
                },
                Construct {
                    id: "m".into(),
                    name: "M".into(),
                    short_name: "M".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["m".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["y".into()],
                },
            ],
            paths: vec![
                StructuralPath {
                    source: "x".into(),
                    target: "m".into(),
                },
                StructuralPath {
                    source: "m".into(),
                    target: "y".into(),
                },
            ],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut recipe = AnalysisRecipe {
            schema_version: qpls_core::ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::nil(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model,
            settings: AnalysisSettings::default(),
            method_config: Some(qpls_core::MethodConfig::PlsBootstrap),
            metadata: std::collections::BTreeMap::new(),
        };
        recipe.settings.bootstrap_samples = 99;
        let mut base_recipe = recipe.clone();
        base_recipe.settings.bootstrap_samples = 0;
        base_recipe.method_config = Some(qpls_core::MethodConfig::PlsAlgorithm);
        let original = qpls_estimation::estimate_pls(&dataset, &base_recipe).unwrap();
        let result = bootstrap_pls(&dataset, &recipe, &original, 1, || false, |_| {}).unwrap();
        let indirect_key = parameter_key("indirect_effect", &["x", "y"]);
        let percentile = result
            .percentile
            .parameters
            .iter()
            .find(|parameter| parameter.parameter == indirect_key)
            .unwrap();
        assert!(percentile.original > 0.9);
        assert!(percentile.standard_error.is_finite());
        assert!(percentile.lower <= percentile.upper);
        assert!(
            result
                .bca
                .as_ref()
                .unwrap()
                .parameters
                .iter()
                .any(|parameter| parameter.parameter == indirect_key)
        );
    }

    #[test]
    fn pls_jackknife_is_exactly_invariant_to_worker_count() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let original = qpls_estimation::estimate_pls(&dataset, &recipe).unwrap();
        let execution = ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0)
            .unwrap()
            .without_outer_resampling()
            .unwrap();
        let serial = jackknife_pls(&dataset, &execution, &original, 1, || false, |_| {}).unwrap();
        let parallel = jackknife_pls(&dataset, &execution, &original, 4, || false, |_| {}).unwrap();
        assert_eq!(serial, parallel);
        assert_eq!(serial.case_count, original.used_observations);
        assert!(serial.outcomes.iter().enumerate().all(|(index, outcome)| {
            matches!(outcome, ReplicateOutcome::Success { value }
                if value.omitted_case == index && value.parameters.len() == 13)
        }));
    }

    #[test]
    fn pls_freedman_lane_permutation_is_exactly_worker_invariant() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.permutation_samples = 199;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsPermutation);
        let base_recipe = ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0)
            .unwrap()
            .without_outer_resampling()
            .unwrap();
        let original =
            qpls_estimation::estimate_pls_validated_with_control(&dataset, &base_recipe, |_| true)
                .unwrap();
        let serial = permutation_pls(&dataset, &recipe, &original, 1, || false, |_| {}).unwrap();
        let parallel = permutation_pls(&dataset, &recipe, &original, 4, || false, |_| {}).unwrap();
        assert_eq!(serial, parallel);
        assert_eq!(serial.method_version, PERMUTATION_METHOD_VERSION);
        assert_eq!(serial.parameters.len(), 1);
        assert_eq!(serial.parameters[0].permutations, 199);
        assert!(serial.parameters[0].p_value_two_sided <= 0.01);
        assert_eq!(
            serial.parameters[0].p_value_two_sided,
            (serial.parameters[0].exceedances as f64 + 1.0) / 200.0
        );
    }

    #[test]
    fn freedman_lane_reference_seam_matches_orthogonal_multi_predictor_fixture_and_rejects_invalid_indices()
     {
        let focal = vec![-1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0];
        let nuisance_a = vec![-1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, 1.0];
        let nuisance_b = vec![-1.0, 1.0, -1.0, 1.0, -1.0, 1.0, -1.0, 1.0];
        let outcome = (0..focal.len())
            .map(|row| {
                2.0 * focal[row]
                    + 3.0 * nuisance_a[row]
                    + 5.0 * nuisance_b[row]
                    + focal[row] * nuisance_a[row]
            })
            .collect::<Vec<_>>();
        let predictors = vec![focal, nuisance_a, nuisance_b];
        let reversed = (0..outcome.len()).rev().collect::<Vec<_>>();
        let estimate =
            freedman_lane_focal_coefficient(&predictors, &outcome, 0, &reversed).unwrap();
        assert!((estimate + 2.0).abs() < 1e-12);

        for (focal_index, indices) in [
            (0, vec![0, 1, 2]),
            (0, vec![0, 1, 2, 3, 4, 5, 6, 6]),
            (0, vec![0, 1, 2, 3, 4, 5, 6, 8]),
            (predictors.len(), reversed.clone()),
        ] {
            assert!(matches!(
                freedman_lane_focal_coefficient(&predictors, &outcome, focal_index, &indices,),
                Err(PlsPermutationError::Regression(_))
            ));
        }
        let mut inconsistent = predictors.clone();
        inconsistent[1].pop();
        assert!(matches!(
            freedman_lane_focal_coefficient(&inconsistent, &outcome, 0, &reversed),
            Err(PlsPermutationError::Regression(_))
        ));
    }

    #[test]
    fn pls_freedman_lane_multi_path_is_seeded_worker_invariant_and_progressive() {
        let seed = 20_260_718;
        let (dataset, recipe, original) = multi_path_permutation_fixture(seed);
        let serial_progress = Arc::new(Mutex::new(Vec::new()));
        let serial_updates = serial_progress.clone();
        let serial = permutation_pls(
            &dataset,
            &recipe,
            &original,
            1,
            || false,
            |update| serial_updates.lock().unwrap().push(update),
        )
        .unwrap();
        let repeat = permutation_pls(&dataset, &recipe, &original, 1, || false, |_| {}).unwrap();
        let parallel_progress = Arc::new(Mutex::new(Vec::new()));
        let parallel_updates = parallel_progress.clone();
        let parallel = permutation_pls(
            &dataset,
            &recipe,
            &original,
            4,
            || false,
            |update| parallel_updates.lock().unwrap().push(update),
        )
        .unwrap();
        assert_eq!(serial, repeat);
        assert_eq!(serial, parallel);
        assert_eq!(serial.parameters.len(), 3);
        assert_eq!(
            serial
                .parameters
                .iter()
                .map(|parameter| parameter.parameter.clone())
                .collect::<Vec<_>>(),
            original
                .paths
                .iter()
                .map(|path| parameter_key("path", &[&path.source, &path.target]))
                .collect::<Vec<_>>()
        );
        let expected_progress = (1..=recipe.settings.permutation_samples)
            .map(|completed_replicates| ResamplingProgress {
                phase: ResamplingPhase::Permutation,
                completed_replicates,
                total_replicates: recipe.settings.permutation_samples,
            })
            .collect::<Vec<_>>();
        assert_eq!(*serial_progress.lock().unwrap(), expected_progress);
        assert_eq!(*parallel_progress.lock().unwrap(), expected_progress);

        let (_, alternate_recipe, alternate_original) = multi_path_permutation_fixture(seed + 1);
        let alternate = permutation_pls(
            &dataset,
            &alternate_recipe,
            &alternate_original,
            4,
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(
            serial
                .parameters
                .iter()
                .map(|parameter| (&parameter.parameter, parameter.original))
                .collect::<Vec<_>>(),
            alternate
                .parameters
                .iter()
                .map(|parameter| (&parameter.parameter, parameter.original))
                .collect::<Vec<_>>()
        );
        assert_ne!(serial.plan.master_seed, alternate.plan.master_seed);
        let first_parameter = &serial.parameters[0].parameter;
        let operation = format!("{}:{first_parameter}", serial.plan.operation);
        assert_ne!(
            permutation_indices(original.used_observations, seed, &operation, 0),
            permutation_indices(original.used_observations, seed + 1, &operation, 0)
        );
    }

    #[test]
    fn pls_freedman_lane_multi_path_cancellation_discards_partial_output() {
        let (dataset, recipe, original) = multi_path_permutation_fixture(20_260_718);
        let cancelled = Arc::new(AtomicBool::new(false));
        let cancellation = cancelled.clone();
        let progress = Arc::new(Mutex::new(Vec::new()));
        let progress_updates = progress.clone();
        let cancellation_from_progress = cancelled.clone();
        let result = permutation_pls(
            &dataset,
            &recipe,
            &original,
            1,
            || cancellation.load(Ordering::Relaxed),
            |update| {
                progress_updates.lock().unwrap().push(update);
                if update.completed_replicates == 3 {
                    cancellation_from_progress.store(true, Ordering::Relaxed);
                }
            },
        );
        assert!(matches!(
            result,
            Err(PlsPermutationError::Resampling(ResamplingError::Cancelled))
        ));
        assert_eq!(progress.lock().unwrap().len(), 3);
    }

    #[test]
    fn permutation_wire_contract_rejects_unknown_fields() {
        assert!(
            serde_json::from_value::<PermutationPlan>(serde_json::json!({
                "permutations": 99,
                "master_seed": 7,
                "operation": "fixture",
                "undeclared": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<PermutationRun<u32>>(serde_json::json!({
                "method_version": PERMUTATION_METHOD_VERSION,
                "plan": {
                    "permutations": 99,
                    "master_seed": 7,
                    "operation": "fixture"
                },
                "outcomes": [],
                "undeclared": true
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<PlsPermutationResult>(serde_json::json!({
                "method_version": PERMUTATION_METHOD_VERSION,
                "plan": {
                    "permutations": 99,
                    "master_seed": 7,
                    "operation": "fixture"
                },
                "parameters": [{
                    "parameter": "path",
                    "original": 0.5,
                    "exceedances": 2,
                    "p_value_two_sided": 0.03,
                    "permutations": 99,
                    "undeclared": true
                }]
            }))
            .is_err()
        );
    }

    #[test]
    fn permutation_regression_recovers_intercept_and_nuisance_coefficients() {
        let first = vec![-2.0, -1.0, 0.0, 1.0, 2.0, -2.0, 2.0];
        let second = vec![1.0, -1.0, 2.0, -2.0, 0.0, 3.0, -3.0];
        let outcome = first
            .iter()
            .zip(&second)
            .map(|(first, second)| 4.0 + 2.0 * first + 3.0 * second)
            .collect::<Vec<_>>();
        let (coefficients, fitted) =
            ols_with_intercept(&[first, second], &outcome, "hand fixture").unwrap();
        assert!((coefficients[0] - 2.0).abs() < 1e-12);
        assert!((coefficients[1] - 3.0).abs() < 1e-12);
        assert!(
            fitted
                .iter()
                .zip(outcome)
                .all(|(fitted, actual)| (fitted - actual).abs() < 1e-12)
        );
    }

    #[test]
    fn type7_percentile_interpolates_at_requested_probability() {
        let values = [1.0, 2.0, 4.0, 8.0];
        assert_eq!(type7_quantile(&values, 0.0), 1.0);
        assert_eq!(type7_quantile(&values, 1.0), 8.0);
        assert_eq!(type7_quantile(&values, 0.5), 3.0);
        assert_eq!(type7_quantile(&values, 0.25), 1.75);
    }

    #[test]
    fn bca_matches_hand_calculated_midrank_fixture() {
        let interval = bca_interval(
            &[1.1, 1.3, 1.7, 1.8, 2.0, 2.1, 2.4, 2.8, 3.0, 3.2],
            2.0,
            &[1.85, 1.90, 2.05, 2.10, 2.20, 1.95],
            0.95,
        )
        .unwrap();
        assert!((interval.bias_correction - -0.12566134685507402).abs() < 1e-12);
        assert!((interval.acceleration - -0.015853543711576476).abs() < 1e-12);
        assert!((interval.lower - 1.1202082785627896).abs() < 1e-12);
        assert!((interval.upper - 3.112197306363598).abs() < 1e-11);
    }

    #[test]
    fn bca_is_unavailable_for_degenerate_or_invalid_inputs() {
        assert!(bca_interval(&[1.0, 2.0, 3.0], 2.0, &[4.0, 4.0, 4.0], 0.95).is_none());
        assert!(bca_interval(&[1.0], 1.0, &[0.9, 1.0, 1.1], 0.95).is_none());
        assert!(bca_interval(&[1.0, 2.0], 1.5, &[0.9, 1.0, 1.1], 1.0).is_none());
    }

    #[test]
    fn regression_bootstrap_summary_freezes_type7_bca_normal_test_and_degenerate_status() {
        let terms = vec!["intercept".into(), "x".into()];
        let bootstrap = vec![
            vec![0.8, -0.6],
            vec![1.1, -0.4],
            vec![0.9, -0.55],
            vec![1.2, -0.45],
        ];
        let jackknife = vec![
            vec![0.95, -0.52],
            vec![1.05, -0.48],
            vec![0.98, -0.51],
            vec![1.02, -0.49],
        ];
        let rows = summarize_regression_bootstrap_coefficients(
            &terms,
            &[1.0, -0.5],
            &bootstrap,
            &jackknife,
            jackknife.len(),
            true,
            0.95,
        )
        .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].term, "intercept");
        assert!((rows[0].bootstrap_mean - 1.0).abs() < 1e-12);
        assert!((rows[0].bias - 0.0).abs() < 1e-12);
        assert_eq!(rows[0].usable_replicates, 4);
        assert!(matches!(
            rows[0].test,
            RegressionBootstrapTest::Available { .. }
        ));
        assert!(matches!(
            rows[0].bca,
            RegressionBootstrapBcaInterval::Available { .. }
        ));
        let odds = rows[0].odds_ratio.as_ref().unwrap();
        let transformed = [0.8_f64.exp(), 0.9_f64.exp(), 1.1_f64.exp(), 1.2_f64.exp()];
        assert!((odds.percentile_lower - type7_quantile(&transformed, 0.025)).abs() < 1e-12);
        assert!((odds.percentile_upper - type7_quantile(&transformed, 0.975)).abs() < 1e-12);

        let degenerate = summarize_regression_bootstrap_coefficients(
            &["x".into()],
            &[2.0],
            &[vec![2.0], vec![2.0], vec![2.0]],
            &[vec![2.0], vec![2.0], vec![2.0]],
            3,
            false,
            0.95,
        )
        .unwrap();
        assert!(matches!(
            &degenerate[0].test,
            RegressionBootstrapTest::Unavailable { reason_code, message }
                if reason_code == "degenerate_bootstrap_standard_error" && !message.is_empty()
        ));
        assert!(matches!(
            &degenerate[0].bca,
            RegressionBootstrapBcaInterval::Unavailable { reason_code, .. }
                if reason_code == "degenerate_jackknife_acceleration"
        ));
        assert!(
            summarize_regression_bootstrap_coefficients(
                &["x".into()],
                &[1.0],
                &[vec![1.0, 2.0], vec![1.1, 2.1]],
                &[],
                0,
                false,
                0.95,
            )
            .is_err()
        );

        let incomplete = summarize_regression_bootstrap_coefficients(
            &["x".into()],
            &[1.0],
            &[vec![0.8], vec![1.0], vec![1.2]],
            &[vec![0.9], vec![1.0], vec![1.1]],
            4,
            false,
            0.95,
        )
        .unwrap();
        assert!(matches!(
            &incomplete[0].bca,
            RegressionBootstrapBcaInterval::Unavailable { reason_code, .. }
                if reason_code == "incomplete_jackknife"
        ));
    }

    #[test]
    fn regression_bootstrap_failure_boundary_listwise_complete_cases_are_the_only_sampling_frame() {
        let dataset = import_delimited_bytes(
            b"y,x,unused\n1.1,1,10\n2.0,2,11\n3.2,3,12\n4.1,NA,13\n5.3,5,14\n6.0,6,15\n7.4,7,16\n8.2,8,17\n9.1,9,18\nNA,10,19\n11.3,11,20\n12.0,12,21\n13.2,13,22\n14.1,14,NA\n",
            "regression-bootstrap-listwise.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let recipe = regression_bootstrap_test_recipe(
            &dataset,
            RegressionModelConfig::Ols {
                robust_se: qpls_core::RobustStandardError::Hc3,
            },
            99,
            4201,
        );
        let positions = [
            dataset.batch.schema().index_of("y").unwrap(),
            dataset.batch.schema().index_of("x").unwrap(),
        ];
        let complete_rows = complete_case_rows_at_positions(&dataset, &positions);
        assert_eq!(complete_rows, vec![0, 1, 2, 4, 5, 6, 7, 8, 10, 11, 12, 13]);
        let (execution, original) = regression_bootstrap_test_original(&dataset, &recipe);
        assert_eq!(original.used_observations, complete_rows.len());
        assert_eq!(original.omitted_observations, 2);

        let bootstrap =
            bootstrap_regression_validated(&dataset, &execution, &original, 1, || false, |_| {})
                .unwrap();
        assert_eq!(bootstrap.jackknife_cases, complete_rows.len());
        assert_eq!(bootstrap.usable_jackknife_cases, complete_rows.len());
        assert_eq!(bootstrap.usable_replicates, 99);
        assert!(bootstrap.failed_replicates.is_empty());

        for replicate_index in 0..bootstrap.requested_replicates {
            let sampled_raw_rows = bootstrap_indices(
                complete_rows.len(),
                recipe.settings.seed,
                "regression_ols_case_bootstrap_v1",
                replicate_index,
            )
            .into_iter()
            .map(|position| complete_rows[position])
            .collect::<Vec<_>>();
            assert_eq!(sampled_raw_rows.len(), complete_rows.len());
            assert!(
                sampled_raw_rows
                    .iter()
                    .all(|row| complete_rows.contains(row) && !matches!(*row, 3 | 9))
            );
        }
    }

    #[test]
    fn regression_bootstrap_failure_boundary_captures_zero_based_single_class_replicates() {
        let dataset = moderately_imbalanced_logistic_dataset();
        let recipe =
            regression_bootstrap_test_recipe(&dataset, RegressionModelConfig::Logistic, 99, 9103);
        let (execution, original) = regression_bootstrap_test_original(&dataset, &recipe);
        let outcomes = (0..dataset.batch.num_rows())
            .map(|row| numeric_value(dataset.batch.column(0).as_ref(), row).unwrap())
            .collect::<Vec<_>>();
        let expected_single_class_indices = (0..recipe.settings.bootstrap_samples)
            .filter(|replicate_index| {
                let sampled = bootstrap_indices(
                    outcomes.len(),
                    recipe.settings.seed,
                    "regression_logistic_case_bootstrap_v1",
                    *replicate_index,
                );
                sampled
                    .iter()
                    .all(|position| outcomes[*position] == outcomes[sampled[0]])
            })
            .collect::<Vec<_>>();
        assert_eq!(expected_single_class_indices, vec![7]);

        let bootstrap =
            bootstrap_regression_validated(&dataset, &execution, &original, 1, || false, |_| {})
                .unwrap();
        let captured = bootstrap
            .failed_replicates
            .iter()
            .filter(|failure| failure.reason_code == "single_class_resample")
            .map(|failure| {
                (
                    failure.replicate_index,
                    failure.reason_code.as_str(),
                    failure.message.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            captured,
            expected_single_class_indices
                .iter()
                .map(|index| (
                    *index,
                    "single_class_resample",
                    "unsupported estimation method: logistic regression outcome must contain both 0 and 1 after listwise deletion",
                ))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn regression_bootstrap_failure_boundary_rejects_below_ninety_percent_usable() {
        let dataset = imbalanced_logistic_dataset(1);
        let recipe =
            regression_bootstrap_test_recipe(&dataset, RegressionModelConfig::Logistic, 99, 9101);
        let (execution, original) = regression_bootstrap_test_original(&dataset, &recipe);
        let error =
            bootstrap_regression_validated(&dataset, &execution, &original, 1, || false, |_| {})
                .unwrap_err();
        let RegressionBootstrapError::InsufficientUsableReplicates { usable, required } = error
        else {
            panic!("expected the 90% usable-replicate gate, got {error}")
        };
        assert_eq!(required, 90);
        assert!(usable < required, "usable={usable}, required={required}");
    }

    #[test]
    fn regression_bootstrap_failure_boundary_real_delete_one_failure_disables_all_bca() {
        let dataset = imbalanced_logistic_dataset(1);
        let recipe =
            regression_bootstrap_test_recipe(&dataset, RegressionModelConfig::Logistic, 99, 9101);
        let (execution, original) = regression_bootstrap_test_original(&dataset, &recipe);
        let point_only = execution.without_outer_resampling().unwrap();
        let regression = original.regression.as_ref().unwrap();
        let terms = regression
            .coefficients
            .iter()
            .map(|coefficient| coefficient.term.clone())
            .collect::<Vec<_>>();
        let complete_rows = (0..dataset.batch.num_rows()).collect::<Vec<_>>();
        let jackknife = run_jackknife(
            complete_rows.len(),
            "regression_logistic_case_jackknife_v1",
            1,
            |omitted_case| {
                let raw_indices = complete_rows
                    .iter()
                    .enumerate()
                    .filter_map(|(position, raw)| (position != omitted_case).then_some(*raw))
                    .collect::<Vec<_>>();
                let estimate = estimate_regression_case_resample_validated_with_control(
                    &dataset,
                    &point_only,
                    &raw_indices,
                    |_| true,
                )
                .map_err(regression_replicate_error)?;
                let resampled = estimate.regression.ok_or_else(|| {
                    "missing_regression_payload|delete-one estimate omitted regression output"
                        .to_string()
                })?;
                if resampled
                    .coefficients
                    .iter()
                    .zip(&terms)
                    .any(|(coefficient, expected)| {
                        coefficient.term != *expected || !coefficient.estimate.is_finite()
                    })
                {
                    return Err(
                        "inconsistent_replicate|delete-one coefficient identity or value is invalid"
                            .to_string(),
                    );
                }
                Ok(resampled
                    .coefficients
                    .iter()
                    .map(|coefficient| coefficient.estimate)
                    .collect::<Vec<_>>())
            },
            || false,
            |_| {},
        )
        .unwrap();
        let failures = jackknife
            .outcomes
            .iter()
            .enumerate()
            .filter_map(|(omitted_case, outcome)| match outcome {
                ReplicateOutcome::Failed { message } => {
                    let (reason_code, message) = split_regression_failure(message);
                    Some((omitted_case, reason_code, message))
                }
                ReplicateOutcome::Success { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            failures,
            vec![(
                9,
                "single_class_resample".to_string(),
                "unsupported estimation method: logistic regression outcome must contain both 0 and 1 after listwise deletion".to_string(),
            )]
        );
        let successful_jackknife = jackknife
            .outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                ReplicateOutcome::Success { value } => Some(value.clone()),
                ReplicateOutcome::Failed { .. } => None,
            })
            .collect::<Vec<_>>();
        let point = regression
            .coefficients
            .iter()
            .map(|coefficient| coefficient.estimate)
            .collect::<Vec<_>>();
        let deterministic_bootstrap = (0..99)
            .map(|replicate| {
                point
                    .iter()
                    .enumerate()
                    .map(|(term, estimate)| {
                        estimate + ((replicate % 11) as f64 - 5.0) * 0.002 * (term + 1) as f64
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let summaries = summarize_regression_bootstrap_coefficients(
            &terms,
            &point,
            &deterministic_bootstrap,
            &successful_jackknife,
            jackknife.case_count,
            true,
            0.95,
        )
        .unwrap();
        assert!(summaries.iter().all(|row| {
            matches!(
                &row.bca,
                RegressionBootstrapBcaInterval::Unavailable { reason_code, message }
                    if reason_code == "incomplete_jackknife" && !message.is_empty()
            ) && matches!(
                row.odds_ratio.as_ref().map(|odds| &odds.bca),
                Some(RegressionBootstrapBcaInterval::Unavailable { reason_code, message })
                    if reason_code == "incomplete_jackknife" && !message.is_empty()
            )
        }));
    }

    #[test]
    fn studentized_interval_matches_reversed_pivot_quantiles() {
        let interval =
            studentized_interval(10.0, 2.0, &[-2.0, -1.0, 0.0, 1.0, 2.0], 0.8, 12.0).unwrap();
        assert!((interval.lower_pivot - -1.6).abs() < 1e-12);
        assert!((interval.upper_pivot - 1.6).abs() < 1e-12);
        assert!((interval.lower - 6.8).abs() < 1e-12);
        assert!((interval.upper - 13.2).abs() < 1e-12);
        assert!(studentized_interval(10.0, 0.0, &[-1.0, 1.0], 0.95, 10.0).is_none());
        assert!(studentized_interval(10.0, 1.0, &[0.0], 0.95, 10.0).is_none());
        let extreme_scale = 1.0e16;
        let tolerance = numerical_zero_tolerance(1.0, [extreme_scale, -extreme_scale]);
        assert_eq!(tolerance, 64.0 * f64::EPSILON * extreme_scale);
        assert!(
            studentized_interval(1.0, tolerance / 2.0, &[-1.0, 1.0], 0.95, extreme_scale,)
                .is_none()
        );
    }

    #[test]
    fn nested_infrastructure_failure_is_explicit_without_failing_primary_outcome() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let original = qpls_estimation::estimate_pls(&dataset, &recipe).unwrap();
        let estimate = PlsBootstrapEstimate {
            replicate_index: 7,
            iterations: original.iterations,
            used_observations: original.used_observations,
            omitted_observations: original.omitted_observations,
            outer_estimates: original.outer_estimates.clone(),
            paths: original.paths.clone(),
            effects: original.effects.clone(),
            r_squared: original.r_squared.clone(),
            studentized_standard_errors: None,
            studentized_error: Some("inner estimate parameter schema mismatch".into()),
        };
        let run = BootstrapRun {
            method_version: RESAMPLING_METHOD_VERSION.into(),
            plan: BootstrapPlan {
                replicates: 999,
                master_seed: 91,
                operation: "pls_pm_bootstrap_v1".into(),
            },
            outcomes: vec![ReplicateOutcome::Success { value: estimate }],
        };
        let summary = summarize_studentized(
            &original,
            &run,
            &PercentileInference {
                confidence_level: 0.95,
                parameters: Vec::new(),
            },
            0.95,
            99,
        )
        .unwrap();
        assert!(summary.parameters.is_empty());
        let failure = summary.failure.unwrap();
        assert_eq!(failure.reason_code, "nested_infrastructure_failure");
        assert_eq!(failure.first_primary_replicate, 7);
        assert_eq!(failure.failed_primary_replicates, 1);
    }

    #[test]
    fn invalid_studentized_plans_fail_before_nested_estimation() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let original = qpls_estimation::estimate_pls(&dataset, &recipe).unwrap();
        recipe.settings.bootstrap_samples = 998;
        recipe.settings.studentized_inner_samples = 99;
        recipe.method_config = Some(qpls_core::MethodConfig::PlsBootstrap);
        assert!(matches!(
            bootstrap_pls(&dataset, &recipe, &original, 1, || false, |_| {}),
            Err(PlsBootstrapError::InvalidStudentizedPlan)
        ));
        recipe.settings.bootstrap_samples = 999;
        recipe.settings.studentized_inner_samples = 100;
        assert!(matches!(
            bootstrap_pls(&dataset, &recipe, &original, 1, || false, |_| {}),
            Err(PlsBootstrapError::InvalidStudentizedPlan)
        ));
    }

    #[test]
    fn parameter_identity_cannot_collide_on_identifier_delimiters() {
        let outer = vec![
            OuterEstimate {
                construct: "a".into(),
                indicator: "b:c".into(),
                weight: 1.0,
                loading: 2.0,
            },
            OuterEstimate {
                construct: "a:b".into(),
                indicator: "c".into(),
                weight: 3.0,
                loading: 4.0,
            },
        ];
        let values = result_values(&outer, &[], &[], &std::collections::BTreeMap::new());
        assert_eq!(values.len(), 4);
        assert!(values.contains_key(&parameter_key("outer_loading", &["a", "b:c"])));
        assert!(values.contains_key(&parameter_key("outer_loading", &["a:b", "c"])));
    }

    #[test]
    fn normal_reference_test_matches_standard_normal_probability() {
        let (statistic, probability) = normal_reference_test(1.0, 0.5);
        assert_eq!(statistic, Some(2.0));
        assert!((probability.unwrap() - 0.04550026389635842).abs() < 1e-10);
        assert_eq!(normal_reference_test(1.0, 0.0), (None, None));
    }

    #[test]
    fn process_graph_v2_unavailable_inference_uses_process_specific_tokens() {
        let ids = vec!["direct:X->Y".to_string()];
        let bootstrap = vec![vec![0.5]; 99];
        let jackknife = vec![vec![0.5]; 8];
        let summaries =
            summarize_process_bootstrap_estimands(&ids, &[0.5], &bootstrap, &jackknife, 8, 0.95)
                .unwrap();
        assert_eq!(summaries.len(), 1);
        assert!(matches!(
            &summaries[0].test,
            RegressionBootstrapTest::Unavailable { reason_code, .. }
                if reason_code == "zero_bootstrap_standard_error"
        ));
        assert!(matches!(
            &summaries[0].bca,
            RegressionBootstrapBcaInterval::Unavailable { reason_code, .. }
                if reason_code == "zero_jackknife_variance"
        ));

        let incomplete = summarize_process_bootstrap_estimands(
            &ids,
            &[0.5],
            &bootstrap,
            &jackknife[..7],
            8,
            0.95,
        )
        .unwrap();
        assert!(matches!(
            &incomplete[0].bca,
            RegressionBootstrapBcaInterval::Unavailable { reason_code, .. }
                if reason_code == "incomplete_jackknife"
        ));
    }

    #[test]
    fn missing_data_bootstrap_uses_the_fixed_complete_case_sample() {
        let mut csv = String::from("x1,x2,y1,y2\n");
        for row in 1..=30 {
            let x2 = if row == 7 {
                "NA".to_owned()
            } else {
                (row * 2 + row % 3).to_string()
            };
            csv.push_str(&format!(
                "{row},{x2},{},{}\n",
                row * 3 + row % 5,
                row * 4 + row % 7
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "missing.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        recipe = current_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let complete_rows = complete_case_rows(&dataset, &recipe);
        let original = qpls_estimation::estimate_pls(&dataset, &recipe).unwrap();
        assert_eq!(original.used_observations, complete_rows.len());
        assert_eq!(original.omitted_observations, 1);
        for replicate in 0..20 {
            let positions =
                bootstrap_indices(complete_rows.len(), 42, "missing_complete_cases", replicate);
            let raw_indices = positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let sampled =
                resample_model_dataset(&dataset, &recipe, &raw_indices, &|| false).unwrap();
            let execution =
                ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
            let estimate =
                qpls_estimation::estimate_pls_validated_with_control(&sampled, &execution, |_| {
                    true
                })
                .unwrap();
            assert_eq!(estimate.used_observations, original.used_observations);
            assert_eq!(estimate.omitted_observations, 0);
        }
    }
}
