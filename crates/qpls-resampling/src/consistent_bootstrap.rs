//! Full-reestimation case bootstrap for the bounded `plsc_v2` estimator.
//!
//! This module is deliberately separate from ordinary PLS-PM bootstrapping.
//! Every primary and delete-one sample is estimated with the validated PLSc
//! recipe, including fresh weights, rho_A values, attenuation correction,
//! corrected paths, corrected loadings, effects, and R-squared values.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use qpls_core::{AnalysisMethod, AnalysisSettings, ValidatedExecutionRecipe};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, PLSC_METHOD_VERSION, PlsResult, estimate_pls_validated_with_control,
};

use super::{
    BcaInference, BcaParameterInference, BootstrapParameterInference, BootstrapPlan,
    PLS_MODEL_FIT_EXACT_METHOD_VERSION, PercentileInference, PlsModelFitExactError,
    PlsModelFitExactInference, RESAMPLING_METHOD_VERSION, ReplicateOutcome, ResamplingError,
    ResamplingPhase, ResamplingProgress, align_pls_signs, bca_interval, bootstrap_indices,
    bootstrap_pls_model_fit_exact_validated, complete_case_rows, normal_reference_test,
    pls_model_fit_exact_requested, resample_model_dataset, run_bootstrap, run_jackknife,
    type7_quantile,
};

pub const PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION: &str = "plsc_bootstrap_v1";
pub const PLSC_CONSISTENT_BOOTSTRAP_OPERATION: &str = "plsc_consistent_bootstrap_v1";
pub const PLSC_CONSISTENT_JACKKNIFE_OPERATION: &str = "plsc_consistent_jackknife_v1";
pub const PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION: f64 = 0.90;
pub const PLSC_CONSISTENT_BOOTSTRAP_RETRY_POLICY: &str =
    "no_retry_no_replacement_fixed_indexed_draws_v1";
pub const PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING: &str = "Consistent bootstrapping fully re-estimated plsc_v2 for every accepted case resample; ordinary PLS bootstrap estimates were not reused.";
pub const PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING: &str = "Failed or inadmissible PLSc refits were retained in the fixed replicate ledger without retry, replacement, or clamping.";
pub const PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING: &str = "BCa intervals are unavailable because at least one required full-PLSc delete-one refit failed; percentile inference remains available.";
pub const PLSC_CONSISTENT_BOOTSTRAP_NUMERICAL_BCA_WARNING: &str = "One or more BCa intervals are unavailable because the full-PLSc acceleration or adjusted quantiles were numerically undefined; percentile inference remains available.";

const SAMPLE_DIGEST_DOMAIN: &[u8] = b"QuickPLS PLSc consistent bootstrap sample v1\0";
const PARAMETER_DIGEST_DOMAIN: &[u8] = b"QuickPLS PLSc consistent bootstrap parameters v1\0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
/// Replayable successful primary refit retained for digest and inference validation.
pub struct PlscBootstrapEstimate {
    pub replicate_index: u32,
    pub iterations: u32,
    pub used_observations: usize,
    pub omitted_observations: usize,
    pub parameters: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlscBootstrapReplicateStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlscBootstrapReplicateLedgerEntry {
    pub replicate_index: u32,
    pub sample_indices_sha256: String,
    pub status: PlscBootstrapReplicateStatus,
    pub parameter_values_sha256: Option<String>,
    pub reason_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlscBootstrapFailedReplicate {
    pub replicate_index: u32,
    pub sample_indices_sha256: String,
    pub reason_code: String,
    pub message: String,
}

/// Replayable successful full-PLSc delete-one refit used by BCa.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscBootstrapSuccessfulJackknifeCase {
    pub omitted_case: usize,
    pub iterations: u32,
    pub used_observations: usize,
    pub omitted_observations: usize,
    pub parameters: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlscBootstrapFailedJackknifeCase {
    pub omitted_case: usize,
    pub reason_code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlscConsistentBootstrapResult {
    pub method_version: String,
    pub estimator_method_version: String,
    pub resampling_method_version: String,
    pub plan: BootstrapPlan,
    pub minimum_usable_fraction: f64,
    pub retry_policy: String,
    pub original_parameter_values_sha256: String,
    pub usable_replicates: u32,
    pub failed_replicates: Vec<PlscBootstrapFailedReplicate>,
    pub replicate_ledger: Vec<PlscBootstrapReplicateLedgerEntry>,
    #[serde(default)]
    pub successful_replicates: Vec<PlscBootstrapEstimate>,
    pub percentile: PercentileInference,
    pub bca: Option<BcaInference>,
    #[serde(default)]
    pub successful_jackknife_cases: Vec<PlscBootstrapSuccessfulJackknifeCase>,
    pub failed_jackknife_cases: Vec<PlscBootstrapFailedJackknifeCase>,
    pub warnings: Vec<String>,
    #[serde(default)]
    pub model_fit_exact_inference: Option<PlsModelFitExactInference>,
}

#[derive(Debug, Error)]
pub enum PlscConsistentBootstrapError {
    #[error("PLSc consistent bootstrap requires raw observations")]
    RawDataRequired,
    #[error("PLSc consistent bootstrap requires method plsc")]
    InvalidMethod,
    #[error("PLSc consistent bootstrap requires 1000 to 10000 primary replicates")]
    InvalidReplicateCount,
    #[error(
        "PLSc consistent bootstrap does not support studentized or combined permutation inference"
    )]
    UnsupportedInferenceCombination,
    #[error("PLSc consistent bootstrap point result is inconsistent: {0}")]
    InconsistentPointResult(String),
    #[error(
        "PLSc consistent bootstrap produced {usable} usable replicates; at least {required} are required"
    )]
    InsufficientUsableReplicates { usable: usize, required: usize },
    #[error(transparent)]
    ExactFit(#[from] PlsModelFitExactError),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

/// Execute the bounded consistent bootstrap from a preflight-validated PLSc
/// recipe. Ordinary PLS bootstrap estimates are never accepted by this API.
pub fn bootstrap_plsc_consistent_validated(
    dataset: &Dataset,
    execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<PlscConsistentBootstrapResult, PlscConsistentBootstrapError> {
    let recipe = execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| {
            PlscConsistentBootstrapError::InconsistentPointResult(error.to_string())
        })?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(PlscConsistentBootstrapError::RawDataRequired);
    }
    if recipe.settings.method != AnalysisMethod::Plsc {
        return Err(PlscConsistentBootstrapError::InvalidMethod);
    }
    if !(1_000..=10_000).contains(&recipe.settings.bootstrap_samples) {
        return Err(PlscConsistentBootstrapError::InvalidReplicateCount);
    }
    if recipe.settings.studentized_inner_samples > 0 || recipe.settings.permutation_samples > 0 {
        return Err(PlscConsistentBootstrapError::UnsupportedInferenceCombination);
    }
    if !original.converged || original.method_version != PLSC_METHOD_VERSION {
        return Err(PlscConsistentBootstrapError::InconsistentPointResult(
            "the base estimate is not a converged plsc_v2 result".into(),
        ));
    }
    let original_values = plsc_parameter_values(original)
        .map_err(PlscConsistentBootstrapError::InconsistentPointResult)?;
    let original_parameter_values_sha256 = parameter_values_sha256(&original_values);
    let base_execution = execution.without_outer_resampling().map_err(|error| {
        PlscConsistentBootstrapError::InconsistentPointResult(error.to_string())
    })?;
    let base_recipe = base_execution.effective();
    if base_recipe.settings.method != AnalysisMethod::Plsc
        || base_recipe.settings.bootstrap_samples != 0
        || base_recipe.settings.studentized_inner_samples != 0
        || base_recipe.settings.permutation_samples != 0
    {
        return Err(PlscConsistentBootstrapError::InconsistentPointResult(
            "the derived replicate recipe is not point-only plsc_v2".into(),
        ));
    }
    let complete_rows = complete_case_rows(dataset, base_recipe);
    if original.used_observations != complete_rows.len() {
        return Err(PlscConsistentBootstrapError::InconsistentPointResult(
            "base estimate observation count differs from the complete-case sample".into(),
        ));
    }

    let plan = BootstrapPlan {
        replicates: recipe.settings.bootstrap_samples,
        master_seed: recipe.settings.seed,
        operation: PLSC_CONSISTENT_BOOTSTRAP_OPERATION.into(),
    };
    let cancellation = &is_cancelled;
    let run = run_bootstrap(
        complete_rows.len(),
        &plan,
        workers,
        |replicate_index, sample_positions| {
            let raw_indices = sample_positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let sampled = resample_model_dataset(dataset, base_recipe, &raw_indices, cancellation)
                .map_err(|error| plsc_refit_failure(&error))?;
            let mut estimate =
                estimate_pls_validated_with_control(&sampled, &base_execution, |_| !cancellation())
                    .map_err(|error| plsc_refit_failure(&error))?;
            align_pls_signs(
                &mut estimate,
                &original.construct_scores,
                sample_positions,
                cancellation,
            )
            .map_err(|error| plsc_refit_failure(&error))?;
            let parameters = plsc_parameter_values(&estimate)
                .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
            ensure_same_parameter_identity(&original_values, &parameters)
                .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
            Ok::<_, String>(PlscBootstrapEstimate {
                replicate_index,
                iterations: estimate.iterations,
                used_observations: estimate.used_observations,
                omitted_observations: estimate.omitted_observations,
                parameters,
            })
        },
        cancellation,
        &report_progress,
    )?;

    let usable = run
        .outcomes
        .iter()
        .filter(|outcome| matches!(outcome, ReplicateOutcome::Success { .. }))
        .count();
    let required = ((plan.replicates as f64 * PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION)
        .ceil() as usize)
        .max(2);
    if usable < required {
        return Err(PlscConsistentBootstrapError::InsufficientUsableReplicates {
            usable,
            required,
        });
    }

    let successful_replicates = run
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value.clone()),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();

    let percentile = summarize_plsc_percentile(
        &original_values,
        &successful_replicates,
        recipe.settings.confidence_level,
    )?;
    let (bca, successful_jackknife_cases, failed_jackknife_cases) = summarize_plsc_bca(
        dataset,
        &base_execution,
        original,
        &complete_rows,
        &original_values,
        &successful_replicates,
        recipe.settings.confidence_level,
        workers,
        cancellation,
        &report_progress,
    )?;

    let mut failed_replicates = Vec::new();
    let mut replicate_ledger = Vec::with_capacity(plan.replicates as usize);
    for (replicate_index, outcome) in run.outcomes.iter().enumerate() {
        let replicate_index = replicate_index as u32;
        let indices = bootstrap_indices(
            complete_rows.len(),
            plan.master_seed,
            &plan.operation,
            replicate_index,
        );
        let sample_indices_sha256 = sample_indices_sha256(&indices);
        match outcome {
            ReplicateOutcome::Success { value } => {
                replicate_ledger.push(PlscBootstrapReplicateLedgerEntry {
                    replicate_index,
                    sample_indices_sha256,
                    status: PlscBootstrapReplicateStatus::Success,
                    parameter_values_sha256: Some(parameter_values_sha256(&value.parameters)),
                    reason_code: None,
                    message: None,
                });
            }
            ReplicateOutcome::Failed { message } => {
                let (reason_code, detail) = split_failure(message);
                failed_replicates.push(PlscBootstrapFailedReplicate {
                    replicate_index,
                    sample_indices_sha256: sample_indices_sha256.clone(),
                    reason_code: reason_code.clone(),
                    message: detail.clone(),
                });
                replicate_ledger.push(PlscBootstrapReplicateLedgerEntry {
                    replicate_index,
                    sample_indices_sha256,
                    status: PlscBootstrapReplicateStatus::Failed,
                    parameter_values_sha256: None,
                    reason_code: Some(reason_code),
                    message: Some(detail),
                });
            }
        }
    }

    let mut warnings = vec![
        PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING.into(),
        PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING.into(),
    ];
    if !failed_jackknife_cases.is_empty() {
        warnings.push(PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING.into());
    } else if bca
        .parameters
        .iter()
        .any(|parameter| parameter.unavailable_reason.is_some())
    {
        warnings.push(PLSC_CONSISTENT_BOOTSTRAP_NUMERICAL_BCA_WARNING.into());
    }

    let model_fit_exact_inference = if pls_model_fit_exact_requested(execution) {
        Some(
            bootstrap_pls_model_fit_exact_validated(
                dataset,
                execution,
                original,
                workers,
                || cancellation(),
                &report_progress,
            )
            .map_err(PlscConsistentBootstrapError::ExactFit)?,
        )
    } else {
        None
    };
    if model_fit_exact_inference
        .as_ref()
        .is_some_and(|inference| inference.method_version != PLS_MODEL_FIT_EXACT_METHOD_VERSION)
    {
        return Err(PlscConsistentBootstrapError::InconsistentPointResult(
            "model-fit exact inference identity mismatch".into(),
        ));
    }
    let result = PlscConsistentBootstrapResult {
        method_version: PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION.into(),
        estimator_method_version: PLSC_METHOD_VERSION.into(),
        resampling_method_version: RESAMPLING_METHOD_VERSION.into(),
        plan,
        minimum_usable_fraction: PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION,
        retry_policy: PLSC_CONSISTENT_BOOTSTRAP_RETRY_POLICY.into(),
        original_parameter_values_sha256,
        usable_replicates: usable as u32,
        failed_replicates,
        replicate_ledger,
        successful_replicates,
        percentile,
        bca: Some(bca),
        successful_jackknife_cases,
        failed_jackknife_cases,
        warnings,
        model_fit_exact_inference,
    };
    validate_plsc_consistent_bootstrap_result(&result, original, &recipe.settings)
        .map_err(PlscConsistentBootstrapError::InconsistentPointResult)?;
    Ok(result)
}

/// Strict semantic validation used by project archives in addition to the
/// archive member checksums. It binds the payload to the PLSc point result and
/// immutable recipe settings and rejects ordinary-bootstrap attribution.
pub fn validate_plsc_consistent_bootstrap_result(
    result: &PlscConsistentBootstrapResult,
    original: &PlsResult,
    settings: &AnalysisSettings,
) -> Result<(), String> {
    if result.method_version != PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION
        || result.estimator_method_version != PLSC_METHOD_VERSION
        || result.resampling_method_version != RESAMPLING_METHOD_VERSION
        || result.plan.operation != PLSC_CONSISTENT_BOOTSTRAP_OPERATION
        || result.plan.replicates != settings.bootstrap_samples
        || result.plan.master_seed != settings.seed
        || !(1_000..=10_000).contains(&result.plan.replicates)
        || settings.method != AnalysisMethod::Plsc
        || settings.studentized_inner_samples != 0
        || settings.permutation_samples != 0
        || result.minimum_usable_fraction != PLSC_CONSISTENT_BOOTSTRAP_MINIMUM_USABLE_FRACTION
        || result.retry_policy != PLSC_CONSISTENT_BOOTSTRAP_RETRY_POLICY
    {
        return Err("method, plan, or immutable setting identity is invalid".into());
    }
    let original_values = plsc_parameter_values(original)?;
    if !is_sha256_hex(&result.original_parameter_values_sha256)
        || result.original_parameter_values_sha256 != parameter_values_sha256(&original_values)
    {
        return Err("point-parameter digest differs from the linked plsc_v2 result".into());
    }
    let required =
        ((result.plan.replicates as f64 * result.minimum_usable_fraction).ceil() as u32).max(2);
    if result.usable_replicates < required
        || result.usable_replicates as usize + result.failed_replicates.len()
            != result.plan.replicates as usize
        || result.replicate_ledger.len() != result.plan.replicates as usize
        || result.successful_replicates.len() != result.usable_replicates as usize
    {
        return Err("replicate accounting is invalid".into());
    }
    let successful = result
        .successful_replicates
        .iter()
        .map(|entry| (entry.replicate_index, entry))
        .collect::<BTreeMap<_, _>>();
    if successful.len() != result.successful_replicates.len() {
        return Err("successful-replicate witnesses contain duplicate indices".into());
    }
    let failed = result
        .failed_replicates
        .iter()
        .map(|entry| (entry.replicate_index, entry))
        .collect::<BTreeMap<_, _>>();
    if failed.len() != result.failed_replicates.len() {
        return Err("failure ledger contains duplicate replicate indices".into());
    }
    for (position, entry) in result.replicate_ledger.iter().enumerate() {
        if entry.replicate_index != position as u32 {
            return Err("replicate ledger is not in exact index order".into());
        }
        let indices = bootstrap_indices(
            original.used_observations,
            result.plan.master_seed,
            &result.plan.operation,
            entry.replicate_index,
        );
        if entry.sample_indices_sha256 != sample_indices_sha256(&indices) {
            return Err("replicate sample-index digest is invalid".into());
        }
        match entry.status {
            PlscBootstrapReplicateStatus::Success => {
                let witness = successful.get(&entry.replicate_index).ok_or_else(|| {
                    "successful replicate is absent from the replayable witness set".to_string()
                })?;
                if failed.contains_key(&entry.replicate_index)
                    || entry
                        .parameter_values_sha256
                        .as_deref()
                        .is_none_or(|digest| !is_sha256_hex(digest))
                    || entry.reason_code.is_some()
                    || entry.message.is_some()
                    || witness.iterations == 0
                    || witness.used_observations != original.used_observations
                    || witness.omitted_observations != 0
                    || ensure_same_parameter_identity(&original_values, &witness.parameters)
                        .is_err()
                    || witness.parameters.values().any(|value| !value.is_finite())
                    || entry.parameter_values_sha256.as_deref()
                        != Some(parameter_values_sha256(&witness.parameters).as_str())
                {
                    return Err("successful replicate ledger entry is malformed".into());
                }
            }
            PlscBootstrapReplicateStatus::Failed => {
                let failure = failed
                    .get(&entry.replicate_index)
                    .ok_or_else(|| "failed replicate is absent from failure ledger".to_string())?;
                if entry.parameter_values_sha256.is_some()
                    || entry.reason_code.as_deref() != Some(failure.reason_code.as_str())
                    || entry.message.as_deref() != Some(failure.message.as_str())
                    || entry.sample_indices_sha256 != failure.sample_indices_sha256
                    || !is_plsc_failure_reason(&failure.reason_code)
                    || failure.message.trim().is_empty()
                {
                    return Err("failed replicate ledger entry is malformed".into());
                }
            }
        }
    }

    validate_percentile(&result.percentile, &original_values, result, settings)?;
    let expected_percentile = summarize_plsc_percentile(
        &original_values,
        &result.successful_replicates,
        settings.confidence_level,
    )
    .map_err(|error| format!("cannot recompute percentile inference from witnesses: {error}"))?;
    if !percentile_inference_equivalent(&result.percentile, &expected_percentile) {
        return Err(
            "percentile and normal-reference inference differs from replayable witnesses".into(),
        );
    }
    let bca = result.bca.as_ref().ok_or_else(|| {
        "consistent bootstrap requires an explicit BCa availability table".to_string()
    })?;
    if bca.confidence_level != settings.confidence_level
        || bca.jackknife_case_count != original.used_observations
        || bca.parameters.len() != original_values.len()
    {
        return Err("BCa table identity is invalid".into());
    }
    let bca_names = bca
        .parameters
        .iter()
        .map(|entry| entry.parameter.as_str())
        .collect::<BTreeSet<_>>();
    if bca_names.len() != bca.parameters.len()
        || bca_names != original_values.keys().map(String::as_str).collect()
    {
        return Err("BCa parameter identity differs from the PLSc point result".into());
    }
    let jackknife_incomplete =
        !result.failed_jackknife_cases.is_empty() || original.used_observations < 3;
    for entry in &bca.parameters {
        let available = matches!(
            (
                entry.bias_correction,
                entry.acceleration,
                entry.lower,
                entry.upper,
                entry.unavailable_reason.as_ref(),
            ),
            (Some(z0), Some(acceleration), Some(lower), Some(upper), None)
                if z0.is_finite()
                    && acceleration.is_finite()
                    && lower.is_finite()
                    && upper.is_finite()
                    && lower <= upper
        );
        let unavailable = entry.bias_correction.is_none()
            && entry.acceleration.is_none()
            && entry.lower.is_none()
            && entry.upper.is_none()
            && entry
                .unavailable_reason
                .as_deref()
                .is_some_and(|reason| !reason.trim().is_empty());
        if !(available || unavailable) || (jackknife_incomplete && !unavailable) {
            return Err("BCa availability encoding is invalid".into());
        }
    }
    let mut last_omitted_case = None;
    if result.failed_jackknife_cases.iter().any(|failure| {
        let out_of_order =
            last_omitted_case.is_some_and(|previous| failure.omitted_case <= previous);
        last_omitted_case = Some(failure.omitted_case);
        failure.omitted_case >= original.used_observations
            || out_of_order
            || !is_plsc_failure_reason(&failure.reason_code)
            || failure.message.trim().is_empty()
    }) {
        return Err("delete-one failure ledger is invalid".into());
    }
    if original.used_observations < 3 {
        if !result.successful_jackknife_cases.is_empty()
            || !result.failed_jackknife_cases.is_empty()
        {
            return Err(
                "delete-one witnesses must be empty when BCa refits are inapplicable".into(),
            );
        }
    } else if result.successful_jackknife_cases.len() + result.failed_jackknife_cases.len()
        != original.used_observations
    {
        return Err("delete-one witness accounting is invalid".into());
    }
    let failed_jackknife_indices = result
        .failed_jackknife_cases
        .iter()
        .map(|entry| entry.omitted_case)
        .collect::<BTreeSet<_>>();
    let mut successful_jackknife_indices = BTreeSet::new();
    for witness in &result.successful_jackknife_cases {
        if !successful_jackknife_indices.insert(witness.omitted_case)
            || failed_jackknife_indices.contains(&witness.omitted_case)
            || witness.omitted_case >= original.used_observations
            || witness.iterations == 0
            || witness.used_observations + 1 != original.used_observations
            || witness.omitted_observations != 0
            || ensure_same_parameter_identity(&original_values, &witness.parameters).is_err()
            || witness.parameters.values().any(|value| !value.is_finite())
        {
            return Err("successful delete-one witness is invalid".into());
        }
    }
    let expected_bca = summarize_plsc_bca_from_witnesses(
        &original_values,
        &result.successful_replicates,
        &result.successful_jackknife_cases,
        &result.failed_jackknife_cases,
        settings.confidence_level,
        original.used_observations,
    );
    if !bca_inference_equivalent(bca, &expected_bca) {
        return Err(
            "BCa inference differs from replayable primary and delete-one witnesses".into(),
        );
    }
    let mut expected_warnings = vec![
        PLSC_CONSISTENT_BOOTSTRAP_FULL_REFIT_WARNING,
        PLSC_CONSISTENT_BOOTSTRAP_FAILURE_LEDGER_WARNING,
    ];
    if !result.failed_jackknife_cases.is_empty() {
        expected_warnings.push(PLSC_CONSISTENT_BOOTSTRAP_INCOMPLETE_JACKKNIFE_WARNING);
    } else if bca
        .parameters
        .iter()
        .any(|parameter| parameter.unavailable_reason.is_some())
    {
        expected_warnings.push(PLSC_CONSISTENT_BOOTSTRAP_NUMERICAL_BCA_WARNING);
    }
    if result
        .warnings
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != expected_warnings
    {
        return Err("consistent-bootstrap warnings differ from the frozen v1 contract".into());
    }
    Ok(())
}

fn validate_percentile(
    percentile: &PercentileInference,
    original_values: &BTreeMap<String, f64>,
    result: &PlscConsistentBootstrapResult,
    settings: &AnalysisSettings,
) -> Result<(), String> {
    if percentile.confidence_level != settings.confidence_level
        || percentile.parameters.len() != original_values.len()
    {
        return Err("percentile table identity is invalid".into());
    }
    let mut names = BTreeSet::new();
    for entry in &percentile.parameters {
        let Some(original) = original_values.get(&entry.parameter) else {
            return Err("percentile parameter is absent from the PLSc point result".into());
        };
        let expected_test = normal_reference_test(entry.original, entry.standard_error);
        if !names.insert(entry.parameter.as_str())
            || entry.original.to_bits() != original.to_bits()
            || entry.usable_replicates != result.usable_replicates
            || !entry.bootstrap_mean.is_finite()
            || !entry.bias.is_finite()
            || !entry.standard_error.is_finite()
            || entry.standard_error < 0.0
            || !entry.lower.is_finite()
            || !entry.upper.is_finite()
            || entry.lower > entry.upper
            || !approximately_equal(entry.bias, entry.bootstrap_mean - entry.original, 1e-12)
            || !optional_approximately_equal(entry.t_statistic, expected_test.0, 1e-12)
            || !optional_approximately_equal(entry.p_value_two_sided, expected_test.1, 1e-12)
        {
            return Err("percentile parameter summary is invalid".into());
        }
    }
    Ok(())
}

fn summarize_plsc_percentile(
    original: &BTreeMap<String, f64>,
    successful_replicates: &[PlscBootstrapEstimate],
    confidence_level: f64,
) -> Result<PercentileInference, PlscConsistentBootstrapError> {
    if successful_replicates.len() < 2 {
        return Err(PlscConsistentBootstrapError::InsufficientUsableReplicates {
            usable: successful_replicates.len(),
            required: 2,
        });
    }
    let tail = (1.0 - confidence_level) / 2.0;
    let mut parameters = Vec::with_capacity(original.len());
    for (parameter, original_value) in original {
        let mut values = successful_replicates
            .iter()
            .map(|replicate| {
                replicate.parameters.get(parameter).copied().ok_or_else(|| {
                    PlscConsistentBootstrapError::InconsistentPointResult(format!(
                        "replicate omitted parameter {parameter}"
                    ))
                })
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
        let (t_statistic, p_value_two_sided) =
            normal_reference_test(*original_value, standard_error);
        parameters.push(BootstrapParameterInference {
            parameter: parameter.clone(),
            original: *original_value,
            bootstrap_mean,
            bias: bootstrap_mean - original_value,
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

#[allow(clippy::too_many_arguments)]
fn summarize_plsc_bca(
    dataset: &Dataset,
    base_execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    complete_rows: &[usize],
    original_values: &BTreeMap<String, f64>,
    successful_replicates: &[PlscBootstrapEstimate],
    confidence_level: f64,
    workers: usize,
    is_cancelled: &(impl Fn() -> bool + Sync),
    report_progress: &(impl Fn(ResamplingProgress) + Sync),
) -> Result<
    (
        BcaInference,
        Vec<PlscBootstrapSuccessfulJackknifeCase>,
        Vec<PlscBootstrapFailedJackknifeCase>,
    ),
    PlscConsistentBootstrapError,
> {
    if complete_rows.len() < 3 {
        return Ok((
            unavailable_bca(
                original_values,
                confidence_level,
                complete_rows.len(),
                "BCa is unavailable because fewer than three complete cases are available for full-PLSc delete-one refits",
            ),
            Vec::new(),
            Vec::new(),
        ));
    }
    let base_recipe = base_execution.effective();
    let jackknife = run_jackknife(
        complete_rows.len(),
        PLSC_CONSISTENT_JACKKNIFE_OPERATION,
        workers,
        |omitted_case| {
            let sampled_positions = (0..complete_rows.len())
                .filter(|position| *position != omitted_case)
                .collect::<Vec<_>>();
            let raw_indices = sampled_positions
                .iter()
                .map(|position| complete_rows[*position])
                .collect::<Vec<_>>();
            let sampled = resample_model_dataset(dataset, base_recipe, &raw_indices, is_cancelled)
                .map_err(|error| plsc_refit_failure(&error))?;
            let mut estimate =
                estimate_pls_validated_with_control(&sampled, base_execution, |_| !is_cancelled())
                    .map_err(|error| plsc_refit_failure(&error))?;
            align_pls_signs(
                &mut estimate,
                &original.construct_scores,
                &sampled_positions,
                is_cancelled,
            )
            .map_err(|error| plsc_refit_failure(&error))?;
            let values = plsc_parameter_values(&estimate)
                .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
            ensure_same_parameter_identity(original_values, &values)
                .map_err(|message| format!("parameter_identity_mismatch|{message}"))?;
            Ok::<_, String>(PlscBootstrapSuccessfulJackknifeCase {
                omitted_case,
                iterations: estimate.iterations,
                used_observations: estimate.used_observations,
                omitted_observations: estimate.omitted_observations,
                parameters: values,
            })
        },
        is_cancelled,
        |update| {
            report_progress(ResamplingProgress {
                phase: ResamplingPhase::Jackknife,
                completed_replicates: update.completed_replicates,
                total_replicates: update.total_replicates,
            });
        },
    )?;
    let failed_jackknife_cases = jackknife
        .outcomes
        .iter()
        .enumerate()
        .filter_map(|(omitted_case, outcome)| match outcome {
            ReplicateOutcome::Success { .. } => None,
            ReplicateOutcome::Failed { message } => {
                let (reason_code, message) = split_failure(message);
                Some(PlscBootstrapFailedJackknifeCase {
                    omitted_case,
                    reason_code,
                    message,
                })
            }
        })
        .collect::<Vec<_>>();
    let successful_jackknife_cases = jackknife
        .outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ReplicateOutcome::Success { value } => Some(value.clone()),
            ReplicateOutcome::Failed { .. } => None,
        })
        .collect::<Vec<_>>();
    let bca = summarize_plsc_bca_from_witnesses(
        original_values,
        successful_replicates,
        &successful_jackknife_cases,
        &failed_jackknife_cases,
        confidence_level,
        complete_rows.len(),
    );
    Ok((bca, successful_jackknife_cases, failed_jackknife_cases))
}

fn summarize_plsc_bca_from_witnesses(
    original_values: &BTreeMap<String, f64>,
    successful_replicates: &[PlscBootstrapEstimate],
    successful_jackknife_cases: &[PlscBootstrapSuccessfulJackknifeCase],
    failed_jackknife_cases: &[PlscBootstrapFailedJackknifeCase],
    confidence_level: f64,
    jackknife_case_count: usize,
) -> BcaInference {
    if jackknife_case_count < 3 {
        return unavailable_bca(
            original_values,
            confidence_level,
            jackknife_case_count,
            "BCa is unavailable because fewer than three complete cases are available for full-PLSc delete-one refits",
        );
    }
    if !failed_jackknife_cases.is_empty() {
        return unavailable_bca(
            original_values,
            confidence_level,
            jackknife_case_count,
            "BCa is unavailable because at least one required full-PLSc delete-one refit failed",
        );
    }
    let mut parameters = Vec::with_capacity(original_values.len());
    for (parameter, original_value) in original_values {
        let bootstrap = successful_replicates
            .iter()
            .map(|witness| witness.parameters[parameter])
            .collect::<Vec<_>>();
        let delete_one = successful_jackknife_cases
            .iter()
            .map(|witness| witness.parameters[parameter])
            .collect::<Vec<_>>();
        parameters.push(match bca_interval(
            &bootstrap,
            *original_value,
            &delete_one,
            confidence_level,
        ) {
            Some(interval) => BcaParameterInference {
                parameter: parameter.clone(),
                bias_correction: Some(interval.bias_correction),
                acceleration: Some(interval.acceleration),
                lower: Some(interval.lower),
                upper: Some(interval.upper),
                unavailable_reason: None,
            },
            None => BcaParameterInference {
                parameter: parameter.clone(),
                bias_correction: None,
                acceleration: None,
                lower: None,
                upper: None,
                unavailable_reason: Some(
                    "BCa is unavailable because the full-PLSc delete-one acceleration or adjusted quantiles are numerically undefined"
                        .into(),
                ),
            },
        });
    }
    BcaInference {
        confidence_level,
        jackknife_case_count,
        parameters,
    }
}

fn unavailable_bca(
    original_values: &BTreeMap<String, f64>,
    confidence_level: f64,
    jackknife_case_count: usize,
    reason: &str,
) -> BcaInference {
    BcaInference {
        confidence_level,
        jackknife_case_count,
        parameters: original_values
            .keys()
            .map(|parameter| BcaParameterInference {
                parameter: parameter.clone(),
                bias_correction: None,
                acceleration: None,
                lower: None,
                upper: None,
                unavailable_reason: Some(reason.into()),
            })
            .collect(),
    }
}

fn plsc_parameter_values(result: &PlsResult) -> Result<BTreeMap<String, f64>, String> {
    if !result.converged || result.method_version != PLSC_METHOD_VERSION {
        return Err("estimate is not a converged plsc_v2 result".into());
    }
    let plsc = result
        .plsc
        .as_ref()
        .ok_or_else(|| "plsc_v2 payload is absent".to_string())?;
    if plsc.method_version != PLSC_METHOD_VERSION
        || plsc.corrected_paths != result.paths
        || plsc.corrected_r_squared != result.r_squared
    {
        return Err("PLSc correction payload differs from the result-level corrected model".into());
    }
    let mut values = BTreeMap::new();
    for reliability in &plsc.reliabilities {
        insert_parameter(
            &mut values,
            parameter_key("plsc_rho_a", &[&reliability.construct]),
            reliability.rho_a,
        )?;
    }
    for correlation in &plsc.construct_correlations {
        insert_parameter(
            &mut values,
            parameter_key(
                "plsc_construct_correlation",
                &[&correlation.left, &correlation.right],
            ),
            correlation.corrected,
        )?;
    }
    for outer in &plsc.corrected_outer_loadings {
        insert_parameter(
            &mut values,
            parameter_key("plsc_outer_loading", &[&outer.construct, &outer.indicator]),
            outer.loading,
        )?;
        insert_parameter(
            &mut values,
            parameter_key("plsc_outer_weight", &[&outer.construct, &outer.indicator]),
            outer.weight,
        )?;
    }
    for path in &plsc.corrected_paths {
        insert_parameter(
            &mut values,
            parameter_key("plsc_path", &[&path.source, &path.target]),
            path.coefficient,
        )?;
    }
    for effect in &result.effects {
        insert_parameter(
            &mut values,
            parameter_key("plsc_direct_effect", &[&effect.source, &effect.target]),
            effect.direct,
        )?;
        insert_parameter(
            &mut values,
            parameter_key("plsc_indirect_effect", &[&effect.source, &effect.target]),
            effect.indirect,
        )?;
        insert_parameter(
            &mut values,
            parameter_key("plsc_total_effect", &[&effect.source, &effect.target]),
            effect.total,
        )?;
    }
    for (construct, value) in &plsc.corrected_r_squared {
        insert_parameter(
            &mut values,
            parameter_key("plsc_r_squared", &[construct]),
            *value,
        )?;
    }
    if values.is_empty() {
        return Err("PLSc point result has no bootstrap-eligible parameters".into());
    }
    Ok(values)
}

fn insert_parameter(
    values: &mut BTreeMap<String, f64>,
    parameter: String,
    value: f64,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("parameter {parameter} is non-finite"));
    }
    if values.insert(parameter.clone(), value).is_some() {
        return Err(format!("parameter identity {parameter} is duplicated"));
    }
    Ok(())
}

fn ensure_same_parameter_identity(
    original: &BTreeMap<String, f64>,
    replicate: &BTreeMap<String, f64>,
) -> Result<(), String> {
    let original_keys = original.keys().collect::<Vec<_>>();
    let replicate_keys = replicate.keys().collect::<Vec<_>>();
    if original_keys != replicate_keys {
        return Err("replicate parameter identities differ from the PLSc point result".into());
    }
    Ok(())
}

fn parameter_key(kind: &str, parts: &[&str]) -> String {
    serde_json::to_string(&(kind, parts))
        .expect("consistent-bootstrap parameter identity is serializable")
}

fn plsc_refit_failure(error: &EstimationError) -> String {
    let detail = error.to_string();
    let lower = detail.to_ascii_lowercase();
    let reason = if matches!(error, EstimationError::Cancelled) {
        "cancelled"
    } else if lower.contains("rho_a") || lower.contains("rho a") {
        "inadmissible_rho_a"
    } else if lower.contains("corrected construct correlation") {
        "inadmissible_corrected_correlation"
    } else if lower.contains("converg") {
        "plsc_nonconvergence"
    } else if lower.contains("singular") || lower.contains("collinear") {
        "singular_plsc_equation"
    } else if lower.contains("non-finite") || lower.contains("nonfinite") {
        "nonfinite_plsc_parameter"
    } else {
        "plsc_refit_failed"
    };
    format!("{reason}|{detail}")
}

fn split_failure(value: &str) -> (String, String) {
    let Some((reason, message)) = value.split_once('|') else {
        return ("plsc_refit_failed".into(), value.into());
    };
    let reason = if reason.trim().is_empty() {
        "plsc_refit_failed"
    } else {
        reason.trim()
    };
    let message = if message.trim().is_empty() {
        "PLSc refit failed without a diagnostic message"
    } else {
        message.trim()
    };
    (reason.into(), message.into())
}

fn is_plsc_failure_reason(value: &str) -> bool {
    matches!(
        value,
        "cancelled"
            | "inadmissible_rho_a"
            | "inadmissible_corrected_correlation"
            | "plsc_nonconvergence"
            | "singular_plsc_equation"
            | "nonfinite_plsc_parameter"
            | "parameter_identity_mismatch"
            | "plsc_refit_failed"
    )
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sample_indices_sha256(indices: &[usize]) -> String {
    let mut digest = Sha256::new();
    digest.update(SAMPLE_DIGEST_DOMAIN);
    digest.update((indices.len() as u64).to_le_bytes());
    for index in indices {
        digest.update((*index as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn parameter_values_sha256(values: &BTreeMap<String, f64>) -> String {
    let mut digest = Sha256::new();
    digest.update(PARAMETER_DIGEST_DOMAIN);
    digest.update((values.len() as u64).to_le_bytes());
    for (parameter, value) in values {
        digest.update((parameter.len() as u64).to_le_bytes());
        digest.update(parameter.as_bytes());
        // The archive stores the estimation under a serde_json::Value
        // envelope. That representation can shift the final decimal digit
        // even though the scientific value remains well inside the 1e-10
        // PLSc contract tolerance. Thirteen significant decimal digits keep
        // the witness stricter than that tolerance and stable across the
        // typed -> JSON -> typed archive boundary.
        let canonical_value = format!("{value:.12e}");
        digest.update((canonical_value.len() as u64).to_le_bytes());
        digest.update(canonical_value.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn approximately_equal(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0)
}

fn optional_approximately_equal(left: Option<f64>, right: Option<f64>, tolerance: f64) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => approximately_equal(left, right, tolerance),
        (None, None) => true,
        _ => false,
    }
}

fn percentile_inference_equivalent(
    left: &PercentileInference,
    right: &PercentileInference,
) -> bool {
    approximately_equal(left.confidence_level, right.confidence_level, 1e-12)
        && left.parameters.len() == right.parameters.len()
        && left
            .parameters
            .iter()
            .zip(&right.parameters)
            .all(|(left, right)| {
                left.parameter == right.parameter
                    && approximately_equal(left.original, right.original, 1e-12)
                    && approximately_equal(left.bootstrap_mean, right.bootstrap_mean, 1e-12)
                    && approximately_equal(left.bias, right.bias, 1e-12)
                    && approximately_equal(left.standard_error, right.standard_error, 1e-12)
                    && approximately_equal(left.lower, right.lower, 1e-12)
                    && approximately_equal(left.upper, right.upper, 1e-12)
                    && left.usable_replicates == right.usable_replicates
                    && optional_approximately_equal(left.t_statistic, right.t_statistic, 1e-12)
                    && optional_approximately_equal(
                        left.p_value_two_sided,
                        right.p_value_two_sided,
                        1e-12,
                    )
            })
}

fn bca_inference_equivalent(left: &BcaInference, right: &BcaInference) -> bool {
    approximately_equal(left.confidence_level, right.confidence_level, 1e-12)
        && left.jackknife_case_count == right.jackknife_case_count
        && left.parameters.len() == right.parameters.len()
        && left
            .parameters
            .iter()
            .zip(&right.parameters)
            .all(|(left, right)| {
                left.parameter == right.parameter
                    && optional_approximately_equal(
                        left.bias_correction,
                        right.bias_correction,
                        1e-12,
                    )
                    && optional_approximately_equal(left.acceleration, right.acceleration, 1e-12)
                    && optional_approximately_equal(left.lower, right.lower, 1e-12)
                    && optional_approximately_equal(left.upper, right.upper, 1e-12)
                    && left.unavailable_reason == right.unavailable_reason
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisRecipe, MethodConfig};
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    fn fixture(workers: usize) -> (Dataset, AnalysisRecipe, ValidatedExecutionRecipe, PlsResult) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/plsc_reference.csv"),
            "plsc-consistent-bootstrap.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/plsc_reference.recipe.json"
        ))
        .unwrap();
        recipe = recipe.migrated_v3().unwrap();
        recipe.schema_version = ANALYSIS_RECIPE_SCHEMA_VERSION;
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 1_000;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 0;
        recipe.settings.workers = workers;
        recipe.method_config = Some(MethodConfig::Plsc);
        let execution =
            ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).unwrap();
        let point = execution.without_outer_resampling().unwrap();
        let original = estimate_pls_validated_with_control(&dataset, &point, |_| true).unwrap();
        (dataset, recipe, execution, original)
    }

    #[test]
    fn consistent_bootstrap_fully_refits_plsc_and_is_worker_invariant() {
        let (dataset, _, serial_execution, serial_original) = fixture(1);
        let serial = bootstrap_plsc_consistent_validated(
            &dataset,
            &serial_execution,
            &serial_original,
            1,
            || false,
            |_| {},
        )
        .unwrap();
        let (_, _, parallel_execution, parallel_original) = fixture(4);
        let parallel = bootstrap_plsc_consistent_validated(
            &dataset,
            &parallel_execution,
            &parallel_original,
            4,
            || false,
            |_| {},
        )
        .unwrap();
        assert_eq!(serial_original, parallel_original);
        assert_eq!(serial, parallel);
        assert_eq!(
            serial.method_version,
            PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION
        );
        assert_eq!(serial.estimator_method_version, PLSC_METHOD_VERSION);
        assert_eq!(serial.replicate_ledger.len(), 1_000);
        assert_eq!(
            serial.successful_replicates.len(),
            serial.usable_replicates as usize
        );
        assert_eq!(
            serial.successful_jackknife_cases.len() + serial.failed_jackknife_cases.len(),
            serial_original.used_observations
        );
        assert_eq!(
            serial.usable_replicates as usize + serial.failed_replicates.len(),
            1_000
        );
        assert!(serial.percentile.parameters.iter().all(|entry| {
            serde_json::from_str::<(String, Vec<String>)>(&entry.parameter)
                .is_ok_and(|(kind, parts)| kind.starts_with("plsc_") && !parts.is_empty())
        }));
        validate_plsc_consistent_bootstrap_result(
            &serial,
            &serial_original,
            &serial_execution.source().settings,
        )
        .unwrap();

        let mut tampered_digest = serial.clone();
        tampered_digest.replicate_ledger[0].parameter_values_sha256 = Some("not-a-digest".into());
        assert!(
            validate_plsc_consistent_bootstrap_result(
                &tampered_digest,
                &serial_original,
                &serial_execution.source().settings,
            )
            .is_err()
        );

        let mut tampered_warnings = serial.clone();
        tampered_warnings.warnings.clear();
        assert!(
            validate_plsc_consistent_bootstrap_result(
                &tampered_warnings,
                &serial_original,
                &serial_execution.source().settings,
            )
            .is_err()
        );

        let mut tampered_witness = serial.clone();
        let parameter = tampered_witness.successful_replicates[0]
            .parameters
            .values_mut()
            .next()
            .unwrap();
        *parameter += 0.125;
        assert!(
            validate_plsc_consistent_bootstrap_result(
                &tampered_witness,
                &serial_original,
                &serial_execution.source().settings,
            )
            .is_err()
        );

        let mut tampered_delete_one = serial.clone();
        let parameter = tampered_delete_one.successful_jackknife_cases[0]
            .parameters
            .values_mut()
            .next()
            .unwrap();
        *parameter += 0.125;
        assert!(
            validate_plsc_consistent_bootstrap_result(
                &tampered_delete_one,
                &serial_original,
                &serial_execution.source().settings,
            )
            .is_err()
        );
    }

    #[test]
    fn consistent_bootstrap_cancellation_returns_no_partial_result() {
        let (dataset, _, execution, original) = fixture(1);
        let cancelled = Arc::new(AtomicBool::new(false));
        let check = Arc::clone(&cancelled);
        let set = Arc::clone(&cancelled);
        let result = bootstrap_plsc_consistent_validated(
            &dataset,
            &execution,
            &original,
            1,
            move || check.load(Ordering::SeqCst),
            move |update| {
                if update.phase == ResamplingPhase::Bootstrap && update.completed_replicates >= 1 {
                    set.store(true, Ordering::SeqCst);
                }
            },
        );
        assert!(matches!(
            result,
            Err(PlscConsistentBootstrapError::Resampling(
                ResamplingError::Cancelled
            ))
        ));
    }

    #[test]
    fn consistent_bootstrap_rejects_ordinary_pls_point_results_and_bad_counts() {
        let (dataset, mut recipe, _, _) = fixture(1);
        recipe.settings.bootstrap_samples = 999;
        assert!(ValidatedExecutionRecipe::for_dataset(&recipe, &dataset.fingerprint.0).is_err());

        let (dataset, _, execution, _) = fixture(1);
        let point = execution.without_outer_resampling().unwrap();
        let mut ordinary_recipe = point.source().clone();
        ordinary_recipe.settings.method = AnalysisMethod::PlsPm;
        ordinary_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
        let ordinary_execution =
            ValidatedExecutionRecipe::for_dataset(&ordinary_recipe, &dataset.fingerprint.0)
                .unwrap();
        let ordinary =
            estimate_pls_validated_with_control(&dataset, &ordinary_execution, |_| true).unwrap();
        assert!(matches!(
            bootstrap_plsc_consistent_validated(
                &dataset,
                &execution,
                &ordinary,
                1,
                || false,
                |_| {},
            ),
            Err(PlscConsistentBootstrapError::InconsistentPointResult(_))
        ));
    }

    #[test]
    fn failure_codes_and_type7_micro_summary_are_stable() {
        assert_eq!(
            split_failure("inadmissible_rho_a|rho_A exceeded one"),
            ("inadmissible_rho_a".into(), "rho_A exceeded one".into())
        );
        let original = BTreeMap::from([("[\"plsc_path\",[\"x\",\"y\"]]".into(), 0.25)]);
        let outcomes = [0.1, 0.2, 0.3, 0.4]
            .into_iter()
            .enumerate()
            .map(|(replicate_index, value)| PlscBootstrapEstimate {
                replicate_index: replicate_index as u32,
                iterations: 1,
                used_observations: 4,
                omitted_observations: 0,
                parameters: BTreeMap::from([("[\"plsc_path\",[\"x\",\"y\"]]".into(), value)]),
            })
            .collect::<Vec<_>>();
        let summary = summarize_plsc_percentile(&original, &outcomes, 0.95).unwrap();
        let row = &summary.parameters[0];
        assert_eq!(row.bootstrap_mean, 0.25);
        assert!((row.lower - 0.1075).abs() < 1e-12);
        assert!((row.upper - 0.3925).abs() < 1e-12);
        assert_eq!(row.usable_replicates, 4);
    }
}
