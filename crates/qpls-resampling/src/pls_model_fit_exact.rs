//! Adapted Bollen--Stine exact-fit inference for PLS/PLSc model fit.
//!
//! This is deliberately isolated from the ordinary parameter bootstrap.  It
//! null-transforms the complete indicator sample separately for the saturated
//! and estimated implied correlation matrices, then fully refits the selected
//! estimator for every fixed indexed draw.

use std::collections::BTreeMap;
use std::sync::Arc;

use arrow::{
    array::{ArrayRef, Float64Array},
    record_batch::RecordBatch,
};
use faer::{Mat, Side};
use qpls_assessment::{
    FitCriterionValue, FitMeasures, PLS_MODEL_FIT_METHOD_VERSION, PlsModelFit,
    assess_pls_validated_with_control,
};
use qpls_core::{AnalysisMethod, AnalysisRecipe, AnalysisSettings, ValidatedExecutionRecipe};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{PlsResult, estimate_pls_validated_with_control};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{
    BootstrapPlan, RESAMPLING_METHOD_VERSION, ReplicateOutcome, ResamplingError, ResamplingPhase,
    ResamplingProgress, bootstrap_indices, complete_case_rows, numeric_value,
    resample_model_dataset, run_bootstrap, type7_quantile,
};

pub const PLS_MODEL_FIT_EXACT_METHOD_VERSION: &str = "pls_model_fit_exact_v1";
pub const PLS_MODEL_FIT_EXACT_PROCEDURE: &str = "adapted_bollen_stine_saturated_and_estimated_v1";
pub const PLS_MODEL_FIT_EXACT_TRANSFORMATION: &str =
    "centered_standardized_x_times_s_inverse_half_times_sigma_half_v1";
pub const PLS_MODEL_FIT_EXACT_MATRIX_POWER: &str =
    "symmetric_self_adjoint_positive_definite_eigendecomposition_v1";
pub const PLS_MODEL_FIT_EXACT_QUANTILE_METHOD: &str = "hyndman_fan_type7_v1";
pub const PLS_MODEL_FIT_EXACT_DECISION_RULE: &str =
    "original_less_than_or_equal_to_upper_quantile_not_rejected_v1";
pub const PLS_MODEL_FIT_EXACT_RETRY_POLICY: &str = "no_retry_no_replacement_fixed_indexed_draws_v1";
pub const PLS_MODEL_FIT_EXACT_SAMPLE_DIGEST_METHOD: &str = "sha256_u64_le_v1";
pub const PLS_MODEL_FIT_EXACT_USABLE_INDEX_DIGEST_METHOD: &str = "sha256_u32_le_v1";
pub const PLS_MODEL_FIT_EXACT_MATRIX_DIGEST_METHOD: &str = "sha256_f64_bits_row_major_v1";
pub const PLS_MODEL_FIT_EXACT_MINIMUM_USABLE_FRACTION: f64 = 0.90;
pub const PLS_MODEL_FIT_EXACT_SATURATED_OPERATION: &str = "pls_model_fit_exact_saturated_v1";
pub const PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION: &str = "pls_model_fit_exact_estimated_v1";
pub const PLS_MODEL_FIT_EXACT_RECIPE_SELECTOR: &str = "pls_model_fit_exact_inference";

const SAMPLE_DIGEST_DOMAIN: &[u8] = b"QuickPLS PLS model-fit exact sample v1\0";
const INDEX_DIGEST_DOMAIN: &[u8] = b"QuickPLS PLS model-fit exact usable indices v1\0";
const MATRIX_DIGEST_DOMAIN: &[u8] = b"QuickPLS PLS model-fit exact matrix v1\0";
const MATRIX_IDENTITY_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsModelFitExactStatus {
    Available,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PlsModelFitExactCriterion {
    Srmr,
    DULS,
    DG,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlsModelFitExactReplicateStatus {
    Success,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlsModelFitExactCriterionFailure {
    pub criterion: PlsModelFitExactCriterion,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsModelFitExactReplicateLedgerEntry {
    pub replicate_index: u32,
    pub sample_indices_sha256: String,
    pub status: PlsModelFitExactReplicateStatus,
    pub srmr: Option<f64>,
    pub d_uls: Option<f64>,
    pub d_g: Option<f64>,
    pub criterion_failures: Vec<PlsModelFitExactCriterionFailure>,
    pub failure_reason_code: Option<String>,
    pub failure_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsModelFitExactCriterionInference {
    pub criterion: PlsModelFitExactCriterion,
    pub status: PlsModelFitExactStatus,
    pub original: f64,
    pub requested_replicates: u32,
    pub minimum_usable_replicates: u32,
    pub usable_replicates: u32,
    pub failed_replicates: u32,
    pub usable_replicate_indices_sha256: String,
    pub replicate_min: Option<f64>,
    pub replicate_max: Option<f64>,
    pub upper_95: Option<f64>,
    pub upper_99: Option<f64>,
    pub not_rejected_95: Option<bool>,
    pub not_rejected_99: Option<bool>,
    pub exceed_or_equal_count: u32,
    pub empirical_upper_tail_probability: Option<f64>,
    pub unavailable_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsModelFitExactVariantInference {
    pub variant: String,
    pub status: PlsModelFitExactStatus,
    pub operation: String,
    pub target_correlation_sha256: String,
    pub transformed_correlation: Vec<Vec<f64>>,
    pub transformed_correlation_sha256: String,
    pub transformation_max_abs_error: f64,
    pub requested_replicates: u32,
    pub ledger: Vec<PlsModelFitExactReplicateLedgerEntry>,
    pub criteria: Vec<PlsModelFitExactCriterionInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsModelFitExactInference {
    pub method_version: String,
    pub point_fit_method_version: String,
    pub estimator_method_version: String,
    pub resampling_method_version: String,
    pub procedure: String,
    pub transformation: String,
    pub matrix_power: String,
    pub quantile_method: String,
    pub decision_rule: String,
    pub retry_policy: String,
    pub sample_digest_method: String,
    pub usable_index_digest_method: String,
    pub matrix_digest_method: String,
    pub status: PlsModelFitExactStatus,
    pub analytical_sample_size: usize,
    pub indicator_order: Vec<String>,
    pub master_seed: u64,
    pub requested_replicates: u32,
    pub minimum_usable_fraction: f64,
    pub observed_correlation_sha256: String,
    pub saturated: PlsModelFitExactVariantInference,
    pub estimated: PlsModelFitExactVariantInference,
}

#[derive(Debug, Error)]
pub enum PlsModelFitExactError {
    #[error("PLS model-fit exact inference requires raw observations")]
    RawDataRequired,
    #[error("PLS model-fit exact inference requires method pls_pm or plsc")]
    InvalidMethod,
    #[error("PLS model-fit exact inference requires 999 to 10000 fixed draws")]
    InvalidReplicateCount,
    #[error("PLS model-fit exact inference point result is inconsistent: {0}")]
    InconsistentPointResult(String),
    #[error(
        "PLS model-fit exact inference transformation failed for {variant}: {reason_code}: {message}"
    )]
    Transformation {
        variant: String,
        reason_code: String,
        message: String,
    },
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug, Clone)]
struct ReplicateValues {
    srmr: Option<f64>,
    d_uls: Option<f64>,
    d_g: Option<f64>,
    criterion_failures: Vec<PlsModelFitExactCriterionFailure>,
}

/// Exact-fit inference is an explicit complete-result option.  It is not
/// silently added to ordinary/bootstrap-development runs.
pub fn pls_model_fit_exact_requested(execution: &ValidatedExecutionRecipe) -> bool {
    execution
        .effective()
        .metadata
        .get(PLS_MODEL_FIT_EXACT_RECIPE_SELECTOR)
        .is_some_and(|value| value == "true")
}

/// Execute the two null-transformed model-fit runs from an already validated
/// complete-bootstrap recipe. Ordinary parameter-bootstrap estimates are not
/// accepted or reused by this API.
pub fn bootstrap_pls_model_fit_exact_validated(
    dataset: &Dataset,
    execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlsModelFitExactInference, PlsModelFitExactError> {
    let recipe = execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| PlsModelFitExactError::InconsistentPointResult(error.to_string()))?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlsModelFitExactError::RawDataRequired);
    }
    if !matches!(
        recipe.settings.method,
        AnalysisMethod::PlsPm | AnalysisMethod::Plsc
    ) {
        return Err(PlsModelFitExactError::InvalidMethod);
    }
    if !(999..=10_000).contains(&recipe.settings.bootstrap_samples) {
        return Err(PlsModelFitExactError::InvalidReplicateCount);
    }
    if !original.converged {
        return Err(PlsModelFitExactError::InconsistentPointResult(
            "base estimate is not converged".into(),
        ));
    }
    if is_cancelled() {
        return Err(ResamplingError::Cancelled.into());
    }

    let base_execution = execution
        .without_outer_resampling()
        .map_err(|error| PlsModelFitExactError::InconsistentPointResult(error.to_string()))?;
    let base_recipe = base_execution.effective();
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if complete_rows.len() != original.used_observations {
        return Err(PlsModelFitExactError::InconsistentPointResult(
            "base estimate observation count differs from the complete-case sample".into(),
        ));
    }
    let assessment =
        assess_pls_validated_with_control(dataset, &base_execution, original, |_| !is_cancelled())
            .map_err(|error| PlsModelFitExactError::InconsistentPointResult(error.to_string()))?;
    let point_fit = assessment.model_fit.ok_or_else(|| {
        PlsModelFitExactError::InconsistentPointResult("point model fit is absent".into())
    })?;
    validate_point_fit(&point_fit, original, &complete_rows)?;
    let standardized = standardized_complete_cases(
        dataset,
        &point_fit.indicator_order,
        &complete_rows,
        &is_cancelled,
    )?;
    let standardized_correlation = sample_correlation(&standardized)
        .map_err(PlsModelFitExactError::InconsistentPointResult)?;
    let observed_error =
        max_abs_difference(&standardized_correlation, &point_fit.observed_correlation)
            .map_err(PlsModelFitExactError::InconsistentPointResult)?;
    if observed_error > MATRIX_IDENTITY_TOLERANCE {
        return Err(PlsModelFitExactError::InconsistentPointResult(format!(
            "standardized complete cases disagree with the point observed correlation (max abs error {observed_error})"
        )));
    }

    let saturated = run_variant(
        dataset,
        &base_execution,
        &point_fit,
        &standardized,
        "saturated",
        &point_fit.saturated_implied_correlation,
        &point_fit.saturated,
        PLS_MODEL_FIT_EXACT_SATURATED_OPERATION,
        recipe.settings.bootstrap_samples,
        recipe.settings.seed,
        workers,
        ResamplingPhase::ModelFitExactSaturated,
        &is_cancelled,
        &report_progress,
    )?;
    let estimated = run_variant(
        dataset,
        &base_execution,
        &point_fit,
        &standardized,
        "estimated",
        &point_fit.estimated_implied_correlation,
        &point_fit.estimated,
        PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
        recipe.settings.bootstrap_samples,
        recipe.settings.seed,
        workers,
        ResamplingPhase::ModelFitExactEstimated,
        &is_cancelled,
        &report_progress,
    )?;
    let status = aggregate_status([saturated.status, estimated.status]);
    Ok(PlsModelFitExactInference {
        method_version: PLS_MODEL_FIT_EXACT_METHOD_VERSION.into(),
        point_fit_method_version: point_fit.method_version,
        estimator_method_version: original.method_version.clone(),
        resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
        procedure: PLS_MODEL_FIT_EXACT_PROCEDURE.into(),
        transformation: PLS_MODEL_FIT_EXACT_TRANSFORMATION.into(),
        matrix_power: PLS_MODEL_FIT_EXACT_MATRIX_POWER.into(),
        quantile_method: PLS_MODEL_FIT_EXACT_QUANTILE_METHOD.into(),
        decision_rule: PLS_MODEL_FIT_EXACT_DECISION_RULE.into(),
        retry_policy: PLS_MODEL_FIT_EXACT_RETRY_POLICY.into(),
        sample_digest_method: PLS_MODEL_FIT_EXACT_SAMPLE_DIGEST_METHOD.into(),
        usable_index_digest_method: PLS_MODEL_FIT_EXACT_USABLE_INDEX_DIGEST_METHOD.into(),
        matrix_digest_method: PLS_MODEL_FIT_EXACT_MATRIX_DIGEST_METHOD.into(),
        status,
        analytical_sample_size: original.used_observations,
        indicator_order: point_fit.indicator_order,
        master_seed: recipe.settings.seed,
        requested_replicates: recipe.settings.bootstrap_samples,
        minimum_usable_fraction: PLS_MODEL_FIT_EXACT_MINIMUM_USABLE_FRACTION,
        observed_correlation_sha256: matrix_sha256(&point_fit.observed_correlation),
        saturated,
        estimated,
    })
}

/// Recompute all archive-visible identities, hashes, ledgers, summaries, and
/// decisions. This deliberately does not trust counts or derived fields from
/// a serialized result.
pub fn validate_pls_model_fit_exact_inference(
    result: &PlsModelFitExactInference,
    point_fit: &PlsModelFit,
    original: &PlsResult,
    recipe: &AnalysisRecipe,
) -> Result<(), String> {
    if recipe
        .metadata
        .get(PLS_MODEL_FIT_EXACT_RECIPE_SELECTOR)
        .map(String::as_str)
        != Some("true")
    {
        return Err("PLS model-fit exact recipe selector is absent or invalid".into());
    }
    validate_pls_model_fit_exact_inference_for_settings(
        result,
        point_fit,
        original,
        &recipe.settings,
    )
}

/// Revalidate a persisted exact-fit payload against the immutable settings
/// copied into a completed result envelope. Callers must separately bind the
/// explicit recipe selector or the exact method marker before using this seam.
pub fn validate_pls_model_fit_exact_inference_for_settings(
    result: &PlsModelFitExactInference,
    point_fit: &PlsModelFit,
    original: &PlsResult,
    settings: &AnalysisSettings,
) -> Result<(), String> {
    if result.method_version != PLS_MODEL_FIT_EXACT_METHOD_VERSION
        || result.point_fit_method_version != PLS_MODEL_FIT_METHOD_VERSION
        || result.point_fit_method_version != point_fit.method_version
        || result.estimator_method_version != original.method_version
        || result.resampling_method_version != RESAMPLING_METHOD_VERSION
        || result.procedure != PLS_MODEL_FIT_EXACT_PROCEDURE
        || result.transformation != PLS_MODEL_FIT_EXACT_TRANSFORMATION
        || result.matrix_power != PLS_MODEL_FIT_EXACT_MATRIX_POWER
        || result.quantile_method != PLS_MODEL_FIT_EXACT_QUANTILE_METHOD
        || result.decision_rule != PLS_MODEL_FIT_EXACT_DECISION_RULE
        || result.retry_policy != PLS_MODEL_FIT_EXACT_RETRY_POLICY
        || result.sample_digest_method != PLS_MODEL_FIT_EXACT_SAMPLE_DIGEST_METHOD
        || result.usable_index_digest_method != PLS_MODEL_FIT_EXACT_USABLE_INDEX_DIGEST_METHOD
        || result.matrix_digest_method != PLS_MODEL_FIT_EXACT_MATRIX_DIGEST_METHOD
        || !matches!(
            settings.method,
            AnalysisMethod::PlsPm | AnalysisMethod::Plsc
        )
        || result.analytical_sample_size != original.used_observations
        || result.analytical_sample_size != point_fit.analytical_sample_size
        || result.indicator_order != point_fit.indicator_order
        || result.master_seed != settings.seed
        || result.requested_replicates != settings.bootstrap_samples
        || !(999..=10_000).contains(&result.requested_replicates)
        || result.minimum_usable_fraction.to_bits()
            != PLS_MODEL_FIT_EXACT_MINIMUM_USABLE_FRACTION.to_bits()
        || result.observed_correlation_sha256 != matrix_sha256(&point_fit.observed_correlation)
    {
        return Err("PLS model-fit exact top-level identity or recipe linkage is invalid".into());
    }
    validate_variant_result(
        &result.saturated,
        "saturated",
        PLS_MODEL_FIT_EXACT_SATURATED_OPERATION,
        &point_fit.saturated_implied_correlation,
        &point_fit.saturated,
        result,
    )?;
    validate_variant_result(
        &result.estimated,
        "estimated",
        PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
        &point_fit.estimated_implied_correlation,
        &point_fit.estimated,
        result,
    )?;
    if result.status != aggregate_status([result.saturated.status, result.estimated.status]) {
        return Err("PLS model-fit exact aggregate status is inconsistent".into());
    }
    Ok(())
}

fn validate_variant_result(
    variant: &PlsModelFitExactVariantInference,
    expected_variant: &str,
    expected_operation: &str,
    target: &[Vec<f64>],
    original_measures: &FitMeasures,
    bundle: &PlsModelFitExactInference,
) -> Result<(), String> {
    if variant.variant != expected_variant
        || variant.operation != expected_operation
        || variant.requested_replicates != bundle.requested_replicates
        || variant.ledger.len() != bundle.requested_replicates as usize
        || variant.target_correlation_sha256 != matrix_sha256(target)
        || variant.transformed_correlation_sha256 != matrix_sha256(&variant.transformed_correlation)
    {
        return Err(format!(
            "PLS model-fit exact {expected_variant} identity, digest, or ledger length is invalid"
        ));
    }
    let transformation_error = max_abs_difference(&variant.transformed_correlation, target)?;
    if !variant.transformation_max_abs_error.is_finite()
        || variant.transformation_max_abs_error < 0.0
        || (variant.transformation_max_abs_error - transformation_error).abs() > 1e-14
        || transformation_error > MATRIX_IDENTITY_TOLERANCE
    {
        return Err(format!(
            "PLS model-fit exact {expected_variant} transformation witness is invalid"
        ));
    }
    for (position, entry) in variant.ledger.iter().enumerate() {
        if entry.replicate_index != position as u32 {
            return Err(format!(
                "PLS model-fit exact {expected_variant} ledger order is invalid"
            ));
        }
        let indices = bootstrap_indices(
            bundle.analytical_sample_size,
            bundle.master_seed,
            expected_operation,
            position as u32,
        );
        if entry.sample_indices_sha256 != sample_indices_sha256(expected_operation, &indices) {
            return Err(format!(
                "PLS model-fit exact {expected_variant} sample digest is invalid at replicate {position}"
            ));
        }
        validate_ledger_entry(entry).map_err(|message| {
            format!(
                "PLS model-fit exact {expected_variant} ledger replicate {position} is invalid: {message}"
            )
        })?;
    }
    let expected_originals = [
        (PlsModelFitExactCriterion::Srmr, original_measures.srmr),
        (PlsModelFitExactCriterion::DULS, original_measures.d_uls),
        (
            PlsModelFitExactCriterion::DG,
            original_measures
                .d_g
                .value()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{expected_variant} original d_G is unavailable"))?,
        ),
    ];
    if variant.criteria.len() != expected_originals.len() {
        return Err(format!(
            "PLS model-fit exact {expected_variant} criterion count is invalid"
        ));
    }
    for ((criterion, original), stored) in expected_originals.into_iter().zip(&variant.criteria) {
        let recomputed = summarize_criterion(
            criterion,
            original,
            &variant.ledger,
            variant.requested_replicates,
        );
        if *stored != recomputed {
            return Err(format!(
                "PLS model-fit exact {expected_variant} {criterion:?} summary or decision is invalid"
            ));
        }
    }
    if variant.status != aggregate_status(variant.criteria.iter().map(|criterion| criterion.status))
    {
        return Err(format!(
            "PLS model-fit exact {expected_variant} status is inconsistent"
        ));
    }
    Ok(())
}

fn validate_ledger_entry(entry: &PlsModelFitExactReplicateLedgerEntry) -> Result<(), String> {
    let values = [
        (PlsModelFitExactCriterion::Srmr, entry.srmr),
        (PlsModelFitExactCriterion::DULS, entry.d_uls),
        (PlsModelFitExactCriterion::DG, entry.d_g),
    ];
    if values
        .iter()
        .filter_map(|(_, value)| *value)
        .any(|value| !value.is_finite() || value < 0.0)
    {
        return Err("criterion value is negative or non-finite".into());
    }
    let usable = values.iter().filter(|(_, value)| value.is_some()).count();
    let global_failure = entry.failure_reason_code.is_some() || entry.failure_message.is_some();
    if global_failure {
        if usable != 0
            || !entry.criterion_failures.is_empty()
            || entry
                .failure_reason_code
                .as_deref()
                .is_none_or(str::is_empty)
            || entry.failure_message.as_deref().is_none_or(str::is_empty)
            || entry.status != PlsModelFitExactReplicateStatus::Failed
        {
            return Err("global failure fields are inconsistent".into());
        }
        return Ok(());
    }
    if entry.failure_reason_code.is_some() || entry.failure_message.is_some() {
        return Err("only one global failure field is present".into());
    }
    let expected_status = match usable {
        3 => PlsModelFitExactReplicateStatus::Success,
        1 | 2 => PlsModelFitExactReplicateStatus::Partial,
        _ => PlsModelFitExactReplicateStatus::Failed,
    };
    if entry.status != expected_status || entry.criterion_failures.len() != 3 - usable {
        return Err("criterion availability and status disagree".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for failure in &entry.criterion_failures {
        if failure.reason_code.is_empty()
            || !seen.insert(failure.criterion)
            || values
                .iter()
                .find(|(criterion, _)| *criterion == failure.criterion)
                .is_some_and(|(_, value)| value.is_some())
        {
            return Err("criterion failure identity is invalid".into());
        }
    }
    if values
        .iter()
        .filter(|(_, value)| value.is_none())
        .any(|(criterion, _)| !seen.contains(criterion))
    {
        return Err("a missing criterion has no typed failure".into());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_variant(
    source_dataset: &Dataset,
    base_execution: &ValidatedExecutionRecipe,
    point_fit: &PlsModelFit,
    standardized: &[Vec<f64>],
    variant: &str,
    target: &[Vec<f64>],
    original_measures: &FitMeasures,
    operation: &str,
    requested_replicates: u32,
    master_seed: u64,
    workers: usize,
    phase: ResamplingPhase,
    is_cancelled: &(impl Fn() -> bool + Sync),
    report_progress: &(impl Fn(ResamplingProgress) + Sync),
) -> Result<PlsModelFitExactVariantInference, PlsModelFitExactError> {
    let transformed = null_transform(standardized, target).map_err(|(reason_code, message)| {
        PlsModelFitExactError::Transformation {
            variant: variant.into(),
            reason_code,
            message,
        }
    })?;
    let transformed_correlation = sample_correlation(&transformed).map_err(|error| {
        PlsModelFitExactError::Transformation {
            variant: variant.into(),
            reason_code: "model_fit_exact.transformed_correlation_failed".into(),
            message: error,
        }
    })?;
    let transformation_max_abs_error = max_abs_difference(&transformed_correlation, target)
        .map_err(|error| PlsModelFitExactError::Transformation {
            variant: variant.into(),
            reason_code: "model_fit_exact.transformed_identity_invalid".into(),
            message: error,
        })?;
    if transformation_max_abs_error > MATRIX_IDENTITY_TOLERANCE {
        return Err(PlsModelFitExactError::Transformation {
            variant: variant.into(),
            reason_code: "model_fit_exact.transformed_identity_mismatch".into(),
            message: format!(
                "null-transformed sample misses its target correlation by {transformation_max_abs_error}"
            ),
        });
    }
    let transformed_dataset =
        transformed_dataset(source_dataset, &point_fit.indicator_order, &transformed).map_err(
            |message| PlsModelFitExactError::Transformation {
                variant: variant.into(),
                reason_code: "model_fit_exact.transformed_dataset_failed".into(),
                message,
            },
        )?;
    let plan = BootstrapPlan {
        replicates: requested_replicates,
        master_seed,
        operation: operation.into(),
    };
    let run = run_bootstrap(
        transformed.len(),
        &plan,
        workers,
        |_, indices| {
            let sampled = resample_model_dataset(
                &transformed_dataset,
                base_execution.effective(),
                indices,
                is_cancelled,
            )
            .map_err(|error| format!("model_fit_exact.resample_failed|{error}"))?;
            let estimate =
                estimate_pls_validated_with_control(&sampled, base_execution, |_| !is_cancelled())
                    .map_err(|error| format!("model_fit_exact.estimation_failed|{error}"))?;
            let assessment =
                assess_pls_validated_with_control(&sampled, base_execution, &estimate, |_| {
                    !is_cancelled()
                })
                .map_err(|error| format!("model_fit_exact.assessment_failed|{error}"))?;
            let fit = assessment.model_fit.ok_or_else(|| {
                "model_fit_exact.point_fit_absent|replicate assessment omitted model fit"
                    .to_string()
            })?;
            if fit.indicator_order != point_fit.indicator_order
                || fit.method_version != PLS_MODEL_FIT_METHOD_VERSION
            {
                return Err(
                    "model_fit_exact.identity_mismatch|replicate model-fit identity changed"
                        .to_string(),
                );
            }
            let measures = if variant == "saturated" {
                fit.saturated
            } else {
                fit.estimated
            };
            Ok::<_, String>(replicate_values(&measures))
        },
        is_cancelled,
        |update| {
            report_progress(ResamplingProgress {
                phase,
                completed_replicates: update.completed_replicates,
                total_replicates: update.total_replicates,
            });
        },
    )?;

    let ledger = run
        .outcomes
        .iter()
        .enumerate()
        .map(|(replicate_index, outcome)| {
            let indices = bootstrap_indices(
                transformed.len(),
                master_seed,
                operation,
                replicate_index as u32,
            );
            let sample_indices_sha256 = sample_indices_sha256(operation, &indices);
            match outcome {
                ReplicateOutcome::Success { value } => {
                    let usable = [value.srmr, value.d_uls, value.d_g]
                        .iter()
                        .filter(|value| value.is_some())
                        .count();
                    PlsModelFitExactReplicateLedgerEntry {
                        replicate_index: replicate_index as u32,
                        sample_indices_sha256,
                        status: match usable {
                            3 => PlsModelFitExactReplicateStatus::Success,
                            1 | 2 => PlsModelFitExactReplicateStatus::Partial,
                            _ => PlsModelFitExactReplicateStatus::Failed,
                        },
                        srmr: value.srmr,
                        d_uls: value.d_uls,
                        d_g: value.d_g,
                        criterion_failures: value.criterion_failures.clone(),
                        failure_reason_code: None,
                        failure_message: None,
                    }
                }
                ReplicateOutcome::Failed { message } => {
                    let (reason_code, message) = split_failure(message);
                    PlsModelFitExactReplicateLedgerEntry {
                        replicate_index: replicate_index as u32,
                        sample_indices_sha256,
                        status: PlsModelFitExactReplicateStatus::Failed,
                        srmr: None,
                        d_uls: None,
                        d_g: None,
                        criterion_failures: Vec::new(),
                        failure_reason_code: Some(reason_code),
                        failure_message: Some(message),
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    let criteria = [
        (PlsModelFitExactCriterion::Srmr, original_measures.srmr),
        (PlsModelFitExactCriterion::DULS, original_measures.d_uls),
        (
            PlsModelFitExactCriterion::DG,
            required_criterion(&original_measures.d_g, "original d_G")?,
        ),
    ]
    .into_iter()
    .map(|(criterion, original)| {
        summarize_criterion(criterion, original, &ledger, requested_replicates)
    })
    .collect::<Vec<_>>();
    let status = aggregate_status(criteria.iter().map(|criterion| criterion.status));
    Ok(PlsModelFitExactVariantInference {
        variant: variant.into(),
        status,
        operation: operation.into(),
        target_correlation_sha256: matrix_sha256(target),
        transformed_correlation: transformed_correlation.clone(),
        transformed_correlation_sha256: matrix_sha256(&transformed_correlation),
        transformation_max_abs_error,
        requested_replicates,
        ledger,
        criteria,
    })
}

fn validate_point_fit(
    fit: &PlsModelFit,
    original: &PlsResult,
    complete_rows: &[usize],
) -> Result<(), PlsModelFitExactError> {
    if fit.method_version != PLS_MODEL_FIT_METHOD_VERSION
        || fit.analytical_sample_size != complete_rows.len()
        || fit.analytical_sample_size != original.used_observations
        || fit.indicator_order.is_empty()
        || fit.observed_correlation.len() != fit.indicator_order.len()
        || fit.saturated_implied_correlation.len() != fit.indicator_order.len()
        || fit.estimated_implied_correlation.len() != fit.indicator_order.len()
        || !fit.saturated.srmr.is_finite()
        || !fit.saturated.d_uls.is_finite()
        || !fit.estimated.srmr.is_finite()
        || !fit.estimated.d_uls.is_finite()
        || fit
            .saturated
            .d_g
            .value()
            .is_none_or(|value| !value.is_finite())
        || fit
            .estimated
            .d_g
            .value()
            .is_none_or(|value| !value.is_finite())
    {
        return Err(PlsModelFitExactError::InconsistentPointResult(
            "point-fit identity, dimensions, or exact discrepancy values are invalid".into(),
        ));
    }
    Ok(())
}

fn standardized_complete_cases(
    dataset: &Dataset,
    indicator_order: &[String],
    complete_rows: &[usize],
    is_cancelled: &(impl Fn() -> bool + Sync),
) -> Result<Vec<Vec<f64>>, PlsModelFitExactError> {
    if complete_rows.len() < 2 {
        return Err(PlsModelFitExactError::InconsistentPointResult(
            "at least two complete cases are required".into(),
        ));
    }
    let mut columns = Vec::with_capacity(indicator_order.len());
    for indicator in indicator_order {
        if is_cancelled() {
            return Err(ResamplingError::Cancelled.into());
        }
        let position = dataset.batch.schema().index_of(indicator).map_err(|_| {
            PlsModelFitExactError::InconsistentPointResult(format!(
                "indicator '{indicator}' is absent from the dataset"
            ))
        })?;
        let values = complete_rows
            .iter()
            .map(|row| numeric_value(dataset.batch.column(position).as_ref(), *row))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                PlsModelFitExactError::InconsistentPointResult(format!(
                    "indicator '{indicator}' contains a nonnumeric complete case"
                ))
            })?;
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let sum_squares = values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>();
        let standard_deviation = (sum_squares / (values.len() - 1) as f64).sqrt();
        if !standard_deviation.is_finite() || standard_deviation <= f64::EPSILON {
            return Err(PlsModelFitExactError::InconsistentPointResult(format!(
                "indicator '{indicator}' has zero or non-finite complete-case variance"
            )));
        }
        columns.push(
            values
                .into_iter()
                .map(|value| (value - mean) / standard_deviation)
                .collect::<Vec<_>>(),
        );
    }
    Ok((0..complete_rows.len())
        .map(|row| columns.iter().map(|column| column[row]).collect())
        .collect())
}

fn null_transform(
    standardized: &[Vec<f64>],
    target: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, (String, String)> {
    validate_centered_standardized_rows(standardized)?;
    let observed = sample_correlation(standardized).map_err(|message| {
        (
            "model_fit_exact.observed_correlation_failed".into(),
            message,
        )
    })?;
    validate_correlation_matrix(&observed, "observed")?;
    validate_correlation_matrix(target, "target")?;
    let observed_inverse_half = symmetric_matrix_power(&observed, -0.5, "observed")?;
    let target_half = symmetric_matrix_power(target, 0.5, "target")?;
    let transform = multiply(&observed_inverse_half, &target_half)?;
    multiply_rows(standardized, &transform)
}

fn validate_centered_standardized_rows(rows: &[Vec<f64>]) -> Result<(), (String, String)> {
    if rows.len() < 2 || rows[0].is_empty() || rows.iter().any(|row| row.len() != rows[0].len()) {
        return Err((
            "model_fit_exact.standardized_input_invalid".into(),
            "null transformation requires at least two equal-width standardized rows".into(),
        ));
    }
    let denominator = (rows.len() - 1) as f64;
    for column in 0..rows[0].len() {
        let mean = rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64;
        let variance = rows
            .iter()
            .map(|row| (row[column] - mean).powi(2))
            .sum::<f64>()
            / denominator;
        if !mean.is_finite()
            || !variance.is_finite()
            || mean.abs() > MATRIX_IDENTITY_TOLERANCE
            || (variance - 1.0).abs() > MATRIX_IDENTITY_TOLERANCE
        {
            return Err((
                "model_fit_exact.standardized_input_invalid".into(),
                format!(
                    "null transformation column {column} has mean {mean} and sample variance {variance}; expected zero and one"
                ),
            ));
        }
    }
    Ok(())
}

fn symmetric_matrix_power(
    matrix: &[Vec<f64>],
    exponent: f64,
    subject: &str,
) -> Result<Vec<Vec<f64>>, (String, String)> {
    let dimension = matrix.len();
    let faer_matrix = Mat::from_fn(dimension, dimension, |row, column| matrix[row][column]);
    let eigen = faer_matrix.self_adjoint_eigen(Side::Lower).map_err(|_| {
        (
            format!("model_fit_exact.{subject}_eigendecomposition_failed"),
            format!("{subject} correlation eigendecomposition failed"),
        )
    })?;
    let maximum = (0..dimension)
        .map(|index| eigen.S()[index].abs())
        .fold(0.0, f64::max);
    let tolerance = maximum.max(1.0) * dimension.max(1) as f64 * f64::EPSILON * 128.0;
    if (0..dimension).any(|index| {
        let value = eigen.S()[index];
        !value.is_finite() || value <= tolerance
    }) {
        return Err((
            format!("model_fit_exact.{subject}_not_positive_definite"),
            format!("{subject} correlation is not numerically positive definite"),
        ));
    }
    let powered = (0..dimension)
        .map(|index| eigen.S()[index].powf(exponent))
        .collect::<Vec<_>>();
    Ok((0..dimension)
        .map(|row| {
            (0..dimension)
                .map(|column| {
                    (0..dimension)
                        .map(|index| {
                            eigen.U()[(row, index)] * powered[index] * eigen.U()[(column, index)]
                        })
                        .sum()
                })
                .collect()
        })
        .collect())
}

fn validate_correlation_matrix(matrix: &[Vec<f64>], subject: &str) -> Result<(), (String, String)> {
    let dimension = matrix.len();
    if dimension == 0
        || matrix.iter().any(|row| row.len() != dimension)
        || matrix.iter().flatten().any(|value| !value.is_finite())
    {
        return Err((
            format!("model_fit_exact.{subject}_matrix_invalid"),
            format!("{subject} correlation must be a finite nonempty square matrix"),
        ));
    }
    for row in 0..dimension {
        if (matrix[row][row] - 1.0).abs() > 1e-10 {
            return Err((
                format!("model_fit_exact.{subject}_diagonal_invalid"),
                format!("{subject} correlation diagonal differs from one"),
            ));
        }
        for column in (row + 1)..dimension {
            if (matrix[row][column] - matrix[column][row]).abs() > 1e-10 {
                return Err((
                    format!("model_fit_exact.{subject}_symmetry_invalid"),
                    format!("{subject} correlation is not symmetric"),
                ));
            }
        }
    }
    Ok(())
}

fn multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, (String, String)> {
    if left.is_empty()
        || right.is_empty()
        || left[0].len() != right.len()
        || left.iter().any(|row| row.len() != left[0].len())
        || right.iter().any(|row| row.len() != right[0].len())
    {
        return Err((
            "model_fit_exact.matrix_multiplication_invalid".into(),
            "matrix multiplication dimensions are incompatible".into(),
        ));
    }
    Ok((0..left.len())
        .map(|row| {
            (0..right[0].len())
                .map(|column| {
                    (0..right.len())
                        .map(|inner| left[row][inner] * right[inner][column])
                        .sum()
                })
                .collect()
        })
        .collect())
}

fn multiply_rows(
    rows: &[Vec<f64>],
    matrix: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, (String, String)> {
    if rows.is_empty()
        || matrix.is_empty()
        || rows.iter().any(|row| row.len() != matrix.len())
        || matrix.iter().any(|row| row.len() != matrix.len())
    {
        return Err((
            "model_fit_exact.row_transformation_invalid".into(),
            "row transformation dimensions are incompatible".into(),
        ));
    }
    Ok(rows
        .iter()
        .map(|row| {
            (0..matrix.len())
                .map(|column| {
                    row.iter()
                        .enumerate()
                        .map(|(inner, value)| value * matrix[inner][column])
                        .sum()
                })
                .collect()
        })
        .collect())
}

fn sample_correlation(rows: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if rows.len() < 2 || rows[0].is_empty() || rows.iter().any(|row| row.len() != rows[0].len()) {
        return Err("correlation input must have at least two equal-width rows".into());
    }
    let row_count = rows.len();
    let column_count = rows[0].len();
    let means = (0..column_count)
        .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / row_count as f64)
        .collect::<Vec<_>>();
    let sums_of_squares = (0..column_count)
        .map(|column| {
            rows.iter()
                .map(|row| (row[column] - means[column]).powi(2))
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    if sums_of_squares
        .iter()
        .any(|value| !value.is_finite() || *value <= f64::EPSILON)
    {
        return Err("correlation input contains a constant or non-finite column".into());
    }
    Ok((0..column_count)
        .map(|row| {
            (0..column_count)
                .map(|column| {
                    if row == column {
                        1.0
                    } else {
                        rows.iter()
                            .map(|values| {
                                (values[row] - means[row]) * (values[column] - means[column])
                            })
                            .sum::<f64>()
                            / (sums_of_squares[row] * sums_of_squares[column]).sqrt()
                    }
                })
                .collect()
        })
        .collect())
}

fn transformed_dataset(
    source: &Dataset,
    indicator_order: &[String],
    rows: &[Vec<f64>],
) -> Result<Dataset, String> {
    let columns = indicator_order
        .iter()
        .enumerate()
        .map(|(column, name)| {
            let values = rows.iter().map(|row| row[column]).collect::<Vec<_>>();
            (
                name.clone(),
                Arc::new(Float64Array::from(values)) as ArrayRef,
            )
        })
        .collect::<Vec<_>>();
    let batch = RecordBatch::try_from_iter(columns).map_err(|error| error.to_string())?;
    let mut schema = source.schema.clone();
    schema.case_count = batch.num_rows();
    let by_name = schema
        .columns
        .iter()
        .map(|column| (column.name.as_str(), column.clone()))
        .collect::<BTreeMap<_, _>>();
    schema.columns = indicator_order
        .iter()
        .map(|name| {
            by_name
                .get(name.as_str())
                .cloned()
                .ok_or_else(|| format!("missing source metadata for indicator '{name}'"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Dataset {
        id: source.id,
        name: format!("{} [PLS model-fit null transformed]", source.name),
        schema,
        batch,
        fingerprint: source.fingerprint.clone(),
    })
}

fn replicate_values(measures: &FitMeasures) -> ReplicateValues {
    let mut criterion_failures = Vec::new();
    let srmr = measures.srmr.is_finite().then_some(measures.srmr);
    if srmr.is_none() {
        criterion_failures.push(PlsModelFitExactCriterionFailure {
            criterion: PlsModelFitExactCriterion::Srmr,
            reason_code: "model_fit_exact.nonfinite_srmr".into(),
        });
    }
    let d_uls = measures.d_uls.is_finite().then_some(measures.d_uls);
    if d_uls.is_none() {
        criterion_failures.push(PlsModelFitExactCriterionFailure {
            criterion: PlsModelFitExactCriterion::DULS,
            reason_code: "model_fit_exact.nonfinite_d_uls".into(),
        });
    }
    let d_g = match &measures.d_g {
        FitCriterionValue::Available { value } if value.is_finite() => Some(*value),
        FitCriterionValue::Available { .. } => {
            criterion_failures.push(PlsModelFitExactCriterionFailure {
                criterion: PlsModelFitExactCriterion::DG,
                reason_code: "model_fit_exact.nonfinite_d_g".into(),
            });
            None
        }
        FitCriterionValue::Unavailable { reason_code } => {
            criterion_failures.push(PlsModelFitExactCriterionFailure {
                criterion: PlsModelFitExactCriterion::DG,
                reason_code: reason_code.clone(),
            });
            None
        }
    };
    ReplicateValues {
        srmr,
        d_uls,
        d_g,
        criterion_failures,
    }
}

fn summarize_criterion(
    criterion: PlsModelFitExactCriterion,
    original: f64,
    ledger: &[PlsModelFitExactReplicateLedgerEntry],
    requested_replicates: u32,
) -> PlsModelFitExactCriterionInference {
    let values = ledger
        .iter()
        .filter_map(|entry| {
            let value = match criterion {
                PlsModelFitExactCriterion::Srmr => entry.srmr,
                PlsModelFitExactCriterion::DULS => entry.d_uls,
                PlsModelFitExactCriterion::DG => entry.d_g,
            }?;
            value.is_finite().then_some((entry.replicate_index, value))
        })
        .collect::<Vec<_>>();
    let minimum_usable_replicates =
        ((requested_replicates as f64 * PLS_MODEL_FIT_EXACT_MINIMUM_USABLE_FRACTION).ceil() as u32)
            .max(2);
    let usable_replicates = values.len() as u32;
    let usable_indices = values.iter().map(|(index, _)| *index).collect::<Vec<_>>();
    let usable_replicate_indices_sha256 = usable_indices_sha256(&usable_indices);
    let exceed_or_equal_count = values
        .iter()
        .filter(|(_, value)| *value >= original)
        .count() as u32;
    if usable_replicates < minimum_usable_replicates {
        return PlsModelFitExactCriterionInference {
            criterion,
            status: PlsModelFitExactStatus::Unavailable,
            original,
            requested_replicates,
            minimum_usable_replicates,
            usable_replicates,
            failed_replicates: requested_replicates - usable_replicates,
            usable_replicate_indices_sha256,
            replicate_min: None,
            replicate_max: None,
            upper_95: None,
            upper_99: None,
            not_rejected_95: None,
            not_rejected_99: None,
            exceed_or_equal_count,
            empirical_upper_tail_probability: None,
            unavailable_reason_code: Some("model_fit_exact.insufficient_usable_replicates".into()),
        };
    }
    let mut sorted = values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    sorted.sort_by(f64::total_cmp);
    let upper_95 = type7_quantile(&sorted, 0.95);
    let upper_99 = type7_quantile(&sorted, 0.99);
    PlsModelFitExactCriterionInference {
        criterion,
        status: PlsModelFitExactStatus::Available,
        original,
        requested_replicates,
        minimum_usable_replicates,
        usable_replicates,
        failed_replicates: requested_replicates - usable_replicates,
        usable_replicate_indices_sha256,
        replicate_min: sorted.first().copied(),
        replicate_max: sorted.last().copied(),
        upper_95: Some(upper_95),
        upper_99: Some(upper_99),
        not_rejected_95: Some(original <= upper_95),
        not_rejected_99: Some(original <= upper_99),
        exceed_or_equal_count,
        empirical_upper_tail_probability: Some(
            exceed_or_equal_count as f64 / usable_replicates as f64,
        ),
        unavailable_reason_code: None,
    }
}

fn required_criterion(
    criterion: &FitCriterionValue,
    subject: &str,
) -> Result<f64, PlsModelFitExactError> {
    criterion
        .value()
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            PlsModelFitExactError::InconsistentPointResult(format!(
                "{subject} is unavailable or non-finite"
            ))
        })
}

fn aggregate_status(
    statuses: impl IntoIterator<Item = PlsModelFitExactStatus>,
) -> PlsModelFitExactStatus {
    let statuses = statuses.into_iter().collect::<Vec<_>>();
    if statuses
        .iter()
        .all(|status| *status == PlsModelFitExactStatus::Available)
    {
        PlsModelFitExactStatus::Available
    } else if statuses
        .iter()
        .any(|status| *status != PlsModelFitExactStatus::Unavailable)
    {
        PlsModelFitExactStatus::Partial
    } else {
        PlsModelFitExactStatus::Unavailable
    }
}

fn max_abs_difference(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<f64, String> {
    if left.len() != right.len()
        || left
            .iter()
            .zip(right)
            .any(|(left, right)| left.len() != right.len())
    {
        return Err("matrix dimensions differ".into());
    }
    Ok(left
        .iter()
        .zip(right)
        .flat_map(|(left, right)| left.iter().zip(right))
        .map(|(left, right)| (left - right).abs())
        .fold(0.0, f64::max))
}

fn sample_indices_sha256(operation: &str, indices: &[usize]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(SAMPLE_DIGEST_DOMAIN);
    hasher.update((operation.len() as u64).to_le_bytes());
    hasher.update(operation.as_bytes());
    hasher.update((indices.len() as u64).to_le_bytes());
    for index in indices {
        hasher.update((*index as u64).to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn usable_indices_sha256(indices: &[u32]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(INDEX_DIGEST_DOMAIN);
    hasher.update((indices.len() as u64).to_le_bytes());
    for index in indices {
        hasher.update(index.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

pub fn matrix_sha256(matrix: &[Vec<f64>]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(MATRIX_DIGEST_DOMAIN);
    hasher.update((matrix.len() as u64).to_le_bytes());
    for row in matrix {
        hasher.update((row.len() as u64).to_le_bytes());
        for value in row {
            hasher.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", hasher.finalize())
}

fn split_failure(value: &str) -> (String, String) {
    value
        .split_once('|')
        .map(|(reason, message)| (reason.to_string(), message.to_string()))
        .unwrap_or_else(|| ("model_fit_exact.replicate_failed".into(), value.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{ANALYSIS_RECIPE_SCHEMA_VERSION, MethodConfig};
    use qpls_data::{ImportOptions, import_delimited_bytes};

    fn exact_fit_fixture() -> (
        Dataset,
        ValidatedExecutionRecipe,
        PlsResult,
        PlsModelFit,
        Vec<Vec<f64>>,
    ) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective-exact-fit.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        if recipe.schema_version < ANALYSIS_RECIPE_SCHEMA_VERSION {
            recipe = recipe.migrated_v3().unwrap();
        }
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.seed = 20_260_814;
        recipe.settings.workers = 1;
        recipe.method_config = Some(MethodConfig::PlsAlgorithm);
        let execution =
            ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
        let original = estimate_pls_validated_with_control(&dataset, &execution, |_| true).unwrap();
        let assessment =
            assess_pls_validated_with_control(&dataset, &execution, &original, |_| true).unwrap();
        let point_fit = assessment.model_fit.unwrap();
        let complete_rows = complete_case_rows(&dataset, execution.effective());
        let standardized = standardized_complete_cases(
            &dataset,
            &point_fit.indicator_order,
            &complete_rows,
            &|| false,
        )
        .unwrap();
        (dataset, execution, original, point_fit, standardized)
    }

    #[test]
    fn symmetric_null_transform_reproduces_target_correlation() {
        let rows = vec![
            vec![-1.2, -0.8, 0.3],
            vec![-0.7, 0.1, -0.5],
            vec![-0.1, 0.6, 1.1],
            vec![0.4, -0.2, 0.7],
            vec![0.8, 1.3, -0.9],
            vec![1.4, -1.0, -0.7],
        ];
        let observed = sample_correlation(&rows).unwrap();
        let target = vec![
            vec![1.0, 0.35, -0.20],
            vec![0.35, 1.0, 0.25],
            vec![-0.20, 0.25, 1.0],
        ];
        let means = (0..rows[0].len())
            .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64)
            .collect::<Vec<_>>();
        let standard_deviations = (0..rows[0].len())
            .map(|column| {
                (rows
                    .iter()
                    .map(|row| (row[column] - means[column]).powi(2))
                    .sum::<f64>()
                    / (rows.len() - 1) as f64)
                    .sqrt()
            })
            .collect::<Vec<_>>();
        let standardized = rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(column, value)| (value - means[column]) / standard_deviations[column])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let transformed = null_transform(&standardized, &target).unwrap();
        let recovered = sample_correlation(&transformed).unwrap();
        let recovery_error = max_abs_difference(&recovered, &target).unwrap();
        assert!(
            recovery_error < 1e-12,
            "null transformation missed the target by {recovery_error:e}"
        );
        assert!(max_abs_difference(&observed, &target).unwrap() > 1e-3);
    }

    #[test]
    fn null_transform_rejects_nonstandardized_input() {
        let raw_rows = vec![vec![10.0, 1.0], vec![12.0, 2.0], vec![17.0, 5.0]];
        let error = null_transform(&raw_rows, &[vec![1.0, 0.2], vec![0.2, 1.0]]).unwrap_err();
        assert_eq!(error.0, "model_fit_exact.standardized_input_invalid");
        assert!(error.1.contains("expected zero and one"));
    }

    #[test]
    fn null_transform_rejects_singular_observed_or_target_without_repair() {
        let singular_rows = vec![vec![-1.0, -1.0], vec![0.0, 0.0], vec![1.0, 1.0]];
        let error = null_transform(&singular_rows, &[vec![1.0, 0.2], vec![0.2, 1.0]]).unwrap_err();
        assert_eq!(error.0, "model_fit_exact.observed_not_positive_definite");

        let rows = vec![vec![-1.0, 0.0], vec![0.0, 1.0], vec![1.0, -1.0]];
        let error = null_transform(&rows, &[vec![1.0, 1.0], vec![1.0, 1.0]]).unwrap_err();
        assert_eq!(error.0, "model_fit_exact.target_not_positive_definite");
    }

    #[test]
    fn criterion_summary_uses_fixed_usable_ledger_and_type7_bounds() {
        let ledger = (0..10)
            .map(|replicate_index| PlsModelFitExactReplicateLedgerEntry {
                replicate_index,
                sample_indices_sha256: format!("sample-{replicate_index}"),
                status: if replicate_index == 9 {
                    PlsModelFitExactReplicateStatus::Partial
                } else {
                    PlsModelFitExactReplicateStatus::Success
                },
                srmr: (replicate_index != 9).then_some(replicate_index as f64 / 10.0),
                d_uls: Some(replicate_index as f64),
                d_g: Some(replicate_index as f64 + 1.0),
                criterion_failures: Vec::new(),
                failure_reason_code: None,
                failure_message: None,
            })
            .collect::<Vec<_>>();
        let summary = summarize_criterion(PlsModelFitExactCriterion::Srmr, 0.4, &ledger, 10);
        assert_eq!(summary.status, PlsModelFitExactStatus::Available);
        assert_eq!(summary.usable_replicates, 9);
        assert_eq!(summary.failed_replicates, 1);
        assert_eq!(summary.upper_95, Some(0.76));
        assert_eq!(summary.upper_99, Some(0.792));
        assert_eq!(summary.exceed_or_equal_count, 5);
        assert_eq!(summary.empirical_upper_tail_probability, Some(5.0 / 9.0));
        assert_eq!(summary.not_rejected_95, Some(true));
    }

    #[test]
    fn criterion_summary_fails_closed_below_ninety_percent() {
        let ledger = (0..10)
            .map(|replicate_index| PlsModelFitExactReplicateLedgerEntry {
                replicate_index,
                sample_indices_sha256: String::new(),
                status: PlsModelFitExactReplicateStatus::Partial,
                srmr: (replicate_index < 8).then_some(0.1),
                d_uls: Some(0.1),
                d_g: Some(0.1),
                criterion_failures: Vec::new(),
                failure_reason_code: None,
                failure_message: None,
            })
            .collect::<Vec<_>>();
        let summary = summarize_criterion(PlsModelFitExactCriterion::Srmr, 0.1, &ledger, 10);
        assert_eq!(summary.status, PlsModelFitExactStatus::Unavailable);
        assert_eq!(summary.upper_95, None);
        assert_eq!(summary.not_rejected_95, None);
        assert_eq!(
            summary.unavailable_reason_code.as_deref(),
            Some("model_fit_exact.insufficient_usable_replicates")
        );
    }

    #[test]
    fn sample_and_usable_digests_are_order_and_operation_bound() {
        assert_eq!(
            sample_indices_sha256("a", &[1, 2, 1]),
            sample_indices_sha256("a", &[1, 2, 1])
        );
        assert_ne!(
            sample_indices_sha256("a", &[1, 2, 1]),
            sample_indices_sha256("b", &[1, 2, 1])
        );
        assert_ne!(
            usable_indices_sha256(&[1, 2]),
            usable_indices_sha256(&[2, 1])
        );
    }

    #[test]
    fn exact_variant_is_indexed_worker_invariant_and_cancellable() {
        let (dataset, execution, _original, point_fit, standardized) = exact_fit_fixture();
        let execute = |workers| {
            run_variant(
                &dataset,
                &execution,
                &point_fit,
                &standardized,
                "estimated",
                &point_fit.estimated_implied_correlation,
                &point_fit.estimated,
                PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
                19,
                20_260_814,
                workers,
                ResamplingPhase::ModelFitExactEstimated,
                &|| false,
                &|_| {},
            )
            .unwrap()
        };
        let serial = execute(1);
        let parallel = execute(4);
        assert_eq!(serial, parallel);
        assert_eq!(serial.ledger.len(), 19);
        assert!(
            serial
                .ledger
                .windows(2)
                .all(|pair| pair[0].replicate_index + 1 == pair[1].replicate_index)
        );

        let cancelled = run_variant(
            &dataset,
            &execution,
            &point_fit,
            &standardized,
            "estimated",
            &point_fit.estimated_implied_correlation,
            &point_fit.estimated,
            PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
            19,
            20_260_814,
            4,
            ResamplingPhase::ModelFitExactEstimated,
            &|| true,
            &|_| {},
        );
        assert!(matches!(
            cancelled,
            Err(PlsModelFitExactError::Resampling(
                ResamplingError::Cancelled
            ))
        ));
    }

    #[test]
    fn exact_variant_semantic_validator_rejects_ledger_decision_and_witness_tampering() {
        let (dataset, execution, original, point_fit, standardized) = exact_fit_fixture();
        let variant = run_variant(
            &dataset,
            &execution,
            &point_fit,
            &standardized,
            "estimated",
            &point_fit.estimated_implied_correlation,
            &point_fit.estimated,
            PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
            19,
            20_260_814,
            2,
            ResamplingPhase::ModelFitExactEstimated,
            &|| false,
            &|_| {},
        )
        .unwrap();
        let bundle = PlsModelFitExactInference {
            method_version: PLS_MODEL_FIT_EXACT_METHOD_VERSION.into(),
            point_fit_method_version: PLS_MODEL_FIT_METHOD_VERSION.into(),
            estimator_method_version: original.method_version.clone(),
            resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
            procedure: PLS_MODEL_FIT_EXACT_PROCEDURE.into(),
            transformation: PLS_MODEL_FIT_EXACT_TRANSFORMATION.into(),
            matrix_power: PLS_MODEL_FIT_EXACT_MATRIX_POWER.into(),
            quantile_method: PLS_MODEL_FIT_EXACT_QUANTILE_METHOD.into(),
            decision_rule: PLS_MODEL_FIT_EXACT_DECISION_RULE.into(),
            retry_policy: PLS_MODEL_FIT_EXACT_RETRY_POLICY.into(),
            sample_digest_method: PLS_MODEL_FIT_EXACT_SAMPLE_DIGEST_METHOD.into(),
            usable_index_digest_method: PLS_MODEL_FIT_EXACT_USABLE_INDEX_DIGEST_METHOD.into(),
            matrix_digest_method: PLS_MODEL_FIT_EXACT_MATRIX_DIGEST_METHOD.into(),
            status: variant.status,
            analytical_sample_size: original.used_observations,
            indicator_order: point_fit.indicator_order.clone(),
            master_seed: 20_260_814,
            requested_replicates: 19,
            minimum_usable_fraction: PLS_MODEL_FIT_EXACT_MINIMUM_USABLE_FRACTION,
            observed_correlation_sha256: matrix_sha256(&point_fit.observed_correlation),
            saturated: variant.clone(),
            estimated: variant.clone(),
        };
        assert!(
            validate_variant_result(
                &variant,
                "estimated",
                PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
                &point_fit.estimated_implied_correlation,
                &point_fit.estimated,
                &bundle,
            )
            .is_ok()
        );

        let mut ledger_tamper = variant.clone();
        ledger_tamper.ledger[0].sample_indices_sha256 = "0".repeat(64);
        assert!(
            validate_variant_result(
                &ledger_tamper,
                "estimated",
                PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
                &point_fit.estimated_implied_correlation,
                &point_fit.estimated,
                &bundle,
            )
            .is_err()
        );

        let mut decision_tamper = variant.clone();
        decision_tamper.criteria[0].not_rejected_95 = decision_tamper.criteria[0]
            .not_rejected_95
            .map(|decision| !decision);
        assert!(
            validate_variant_result(
                &decision_tamper,
                "estimated",
                PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
                &point_fit.estimated_implied_correlation,
                &point_fit.estimated,
                &bundle,
            )
            .is_err()
        );

        let mut witness_tamper = variant.clone();
        witness_tamper.transformed_correlation[0][0] += 1e-6;
        witness_tamper.transformed_correlation_sha256 =
            matrix_sha256(&witness_tamper.transformed_correlation);
        assert!(
            validate_variant_result(
                &witness_tamper,
                "estimated",
                PLS_MODEL_FIT_EXACT_ESTIMATED_OPERATION,
                &point_fit.estimated_implied_correlation,
                &point_fit.estimated,
                &bundle,
            )
            .is_err()
        );
    }
}
