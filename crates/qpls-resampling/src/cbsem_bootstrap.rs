use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use thiserror::Error;

use qpls_core::{
    AnalysisMethod, CbsemEstimator, CbsemInput, MethodConfig, ValidatedExecutionRecipe,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    CBSEM_BOOTSTRAP_ALGORITHM_V2, CBSEM_BOOTSTRAP_INTERVAL_METHOD_V2,
    CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2, CBSEM_BOOTSTRAP_METHOD_VERSION_V2,
    CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2, CBSEM_BOOTSTRAP_PRIMARY_OPERATION_V2,
    CBSEM_BOOTSTRAP_RETRY_POLICY_V2, CBSEM_BOOTSTRAP_STREAM_TOKEN_V2,
    CBSEM_BOOTSTRAP_VALIDATION_WITNESS_V2, CBSEM_ML_METHOD_VERSION, CFA_ML_METHOD_VERSION,
    CbsemBootstrapAnalysisV2, CbsemBootstrapFailedReplicateV2, CbsemBootstrapInferenceV2,
    CbsemBootstrapParameterIntervalV2, CbsemBootstrapValidationWitnessV2,
    CbsemBootstrapWitnessReplicateV2, EstimationError, PlsResult,
    estimate_cbsem_case_resample_validated_with_control,
};
use serde::Serialize;

use crate::{ResamplingError, ResamplingPhase, ResamplingProgress, bootstrap_indices};

const MINIMUM_REQUESTED_REPLICATES: u32 = 500;
const MINIMUM_USABLE_REPLICATES: u32 = 1_000;
const MAXIMUM_REPLICATES: u32 = 10_000;

#[derive(Debug, Error)]
pub enum CbsemBootstrapError {
    #[error("CB-SEM bootstrap v2 requires raw observations")]
    RawDataRequired,
    #[error(
        "CB-SEM bootstrap v2 requires typed single-group raw-data ML with 500 to 10000 replicates"
    )]
    InvalidMethod,
    #[error(
        "CB-SEM bootstrap v2 requires a converged point-only cfa_ml_v1 or cbsem_ml_v1 base result"
    )]
    InvalidBaseResult,
    #[error("CB-SEM bootstrap v2 result is inconsistent with the base estimate: {0}")]
    InconsistentResult(String),
    #[error(transparent)]
    Resampling(#[from] ResamplingError),
}

#[derive(Debug)]
struct ReplicateSlot {
    replicate_index: u32,
    failures: Vec<CbsemBootstrapFailedReplicateV2>,
    success: Option<CbsemBootstrapWitnessReplicateV2>,
}

/// Runs a deterministic raw-data case bootstrap around the production CB-SEM
/// ML estimator. Each requested replicate has exactly one preplanned primary
/// draw and one full ML fit. Failed fits are never replaced: they remain in the
/// ledger and count against the frozen usable-replicate threshold.
pub fn bootstrap_cbsem_ml_validated(
    dataset: &Dataset,
    execution: &ValidatedExecutionRecipe,
    original: &PlsResult,
    workers: usize,
    is_cancelled: impl Fn() -> bool + Sync,
    report_progress: impl Fn(ResamplingProgress) + Sync,
) -> Result<CbsemBootstrapAnalysisV2, CbsemBootstrapError> {
    if dataset.schema.kind != DataKind::Raw {
        return Err(CbsemBootstrapError::RawDataRequired);
    }
    if workers == 0 || workers > 64 {
        return Err(ResamplingError::InvalidPlan("workers must be between 1 and 64".into()).into());
    }

    execution
        .effective_for_dataset(&dataset.fingerprint.0)
        .map_err(|error| CbsemBootstrapError::InconsistentResult(error.to_string()))?;
    let source = execution.source();
    let Some(MethodConfig::Cbsem {
        estimator,
        input,
        mean_structure,
        bootstrap_samples,
        bootstrap_v2,
        group_column,
        invariance_steps,
        ..
    }) = source.method_config.as_ref()
    else {
        return Err(CbsemBootstrapError::InvalidMethod);
    };
    if source.settings.method != AnalysisMethod::Cbsem
        || *estimator != CbsemEstimator::Ml
        || *input != CbsemInput::Raw
        || *mean_structure
        || bootstrap_v2.as_ref().is_none_or(|bootstrap| {
            bootstrap.algorithm != qpls_core::CbsemBootstrapAlgorithm::CaseResamplingFullMl
                || bootstrap.interval != qpls_core::CbsemBootstrapInterval::PercentileType7
        })
        || !(MINIMUM_REQUESTED_REPLICATES..=MAXIMUM_REPLICATES).contains(bootstrap_samples)
        || group_column.is_some()
        || !invariance_steps.is_empty()
        || source.settings.confidence_level.to_bits() != 0.95_f64.to_bits()
    {
        return Err(CbsemBootstrapError::InvalidMethod);
    }
    let original_cbsem = original
        .cbsem
        .as_ref()
        .filter(|cbsem| {
            matches!(
                cbsem.method_version.as_str(),
                CBSEM_ML_METHOD_VERSION | CFA_ML_METHOD_VERSION
            ) && cbsem.converged
                && cbsem.bootstrap.is_none()
                && cbsem.bootstrap_v2.is_none()
                && cbsem.multigroup.is_none()
        })
        .ok_or(CbsemBootstrapError::InvalidBaseResult)?;
    let parameter_names = original_cbsem
        .parameters
        .iter()
        .filter(|parameter| !parameter.fixed)
        .map(|parameter| parameter.name.clone())
        .collect::<Vec<_>>();
    let original_estimates = original_cbsem
        .parameters
        .iter()
        .filter(|parameter| !parameter.fixed)
        .map(|parameter| parameter.estimate)
        .collect::<Vec<_>>();
    if parameter_names.is_empty()
        || original_estimates
            .iter()
            .any(|estimate| !estimate.is_finite())
    {
        return Err(CbsemBootstrapError::InvalidBaseResult);
    }
    let recipe_sha256 = cbsem_bootstrap_scientific_recipe_sha256(source)
        .map_err(CbsemBootstrapError::InconsistentResult)?;
    // Bind the complete point-result identity rather than only its CB-SEM
    // projection. The outer archive also binds the unnormalized recipe and
    // provenance; this digest makes swapping any part of the accepted base
    // result detectable during v2 payload recomputation.
    let base_result_sha256 = cbsem_bootstrap_base_result_sha256(original)
        .map_err(CbsemBootstrapError::InconsistentResult)?;

    let point_execution = execution
        .without_outer_resampling()
        .map_err(|error| CbsemBootstrapError::InconsistentResult(error.to_string()))?;
    let complete_rows = super::complete_case_rows(dataset, point_execution.effective());
    if complete_rows.len() != original.used_observations
        || complete_rows.len() != original_cbsem.sample_size
        || complete_rows.len() < 10
    {
        return Err(CbsemBootstrapError::InconsistentResult(
            "base observation count differs from the fixed complete-case sampling frame".into(),
        ));
    }
    if is_cancelled() {
        return Err(ResamplingError::Cancelled.into());
    }

    let requested = *bootstrap_samples;
    let seed = source.settings.seed;
    let cancellation = &is_cancelled;
    let progress_callback = &report_progress;
    let completed = AtomicU64::new(0);
    let progress_guard = Mutex::new(());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .map_err(|error| ResamplingError::WorkerPool(error.to_string()))?;
    let mut slots = pool.install(|| {
        (0..requested)
            .into_par_iter()
            .map(|replicate_index| {
                if cancellation() {
                    return None;
                }
                let mut failures = Vec::new();
                let mut success = None;
                let sampled_positions = bootstrap_indices(
                    complete_rows.len(),
                    seed,
                    cbsem_bootstrap_primary_operation(),
                    replicate_index,
                );
                let raw_indices = sampled_positions
                    .iter()
                    .map(|position| complete_rows[*position])
                    .collect::<Vec<_>>();
                // Hash sampling-frame positions, not storage row numbers. This
                // is the deterministic plan that archive validation can
                // recompute from seed, n, and replicate index without loading
                // raw observations; the dataset fingerprint separately binds
                // the concrete imported row ordering.
                let sample_indices_sha256 = cbsem_bootstrap_sample_indices_sha256(
                    seed,
                    replicate_index,
                    &sampled_positions,
                );
                match estimate_cbsem_case_resample_validated_with_control(
                    dataset,
                    &point_execution,
                    &raw_indices,
                    |_| !cancellation(),
                ) {
                    Ok(estimate) => match cbsem_witness_from_estimate(
                        replicate_index,
                        sample_indices_sha256.clone(),
                        &parameter_names,
                        estimate,
                        complete_rows.len(),
                    ) {
                        Ok(witness) => success = Some(witness),
                        Err((reason_code, message)) => {
                            failures.push(CbsemBootstrapFailedReplicateV2 {
                                replicate_index,
                                sample_indices_sha256,
                                reason_code,
                                message,
                            });
                        }
                    },
                    Err(EstimationError::Cancelled) if cancellation() => return None,
                    Err(error) => {
                        let reason_code = cbsem_refit_error_code(&error).into();
                        failures.push(CbsemBootstrapFailedReplicateV2 {
                            replicate_index,
                            sample_indices_sha256,
                            reason_code,
                            message: error.to_string(),
                        });
                    }
                }
                let _guard = progress_guard
                    .lock()
                    .expect("CB-SEM bootstrap progress mutex poisoned");
                let completed_replicates = completed.fetch_add(1, Ordering::Relaxed) as u32 + 1;
                progress_callback(ResamplingProgress {
                    phase: ResamplingPhase::Bootstrap,
                    completed_replicates,
                    total_replicates: requested,
                });
                Some(ReplicateSlot {
                    replicate_index,
                    failures,
                    success,
                })
            })
            .collect::<Vec<_>>()
    });
    if cancellation() || slots.iter().any(Option::is_none) {
        return Err(ResamplingError::Cancelled.into());
    }
    let mut slots = slots.drain(..).map(Option::unwrap).collect::<Vec<_>>();
    slots.sort_by_key(|slot| slot.replicate_index);

    let successful_replicates = slots
        .iter()
        .filter_map(|slot| slot.success.clone())
        .collect::<Vec<_>>();
    let failures = slots
        .iter()
        .flat_map(|slot| slot.failures.iter().cloned())
        .collect::<Vec<_>>();
    let usable = successful_replicates.len();
    let required = cbsem_bootstrap_required_usable_replicates(requested);
    let attempted_fits = successful_replicates.len() + failures.len();
    let failed_replicates = slots.iter().filter(|slot| slot.success.is_none()).count();
    if attempted_fits != requested as usize
        || failed_replicates != failures.len()
        || usable + failed_replicates != requested as usize
    {
        return Err(CbsemBootstrapError::InconsistentResult(
            "preplanned primary-draw accounting is inconsistent".into(),
        ));
    }
    let (inference, intervals) = cbsem_inference_summary(
        &parameter_names,
        &original_estimates,
        &successful_replicates,
        source.settings.confidence_level,
        required,
    )?;
    let mut warnings = vec![
        "CB-SEM bootstrap v2 uses raw complete-case resampling with replacement and a full production ML refit for every preplanned draw.".into(),
        "The engine executes exactly B preplanned primary draws with no retry or replacement draw; failed fits remain explicit and count against the frozen usable-replicate threshold.".into(),
    ];
    match &inference {
        CbsemBootstrapInferenceV2::Available => warnings.push(
            "Percentile Type-7 intervals are reported from usable full-refit estimates without normal-theory substitution."
                .into(),
        ),
        CbsemBootstrapInferenceV2::Unavailable { message, .. } => {
            warnings.push(message.clone())
        }
    }
    if !failures.is_empty() {
        warnings.push(format!(
            "{} of {} preplanned ML bootstrap fits failed and were excluded; {} usable primary draws remain.",
            failures.len(), requested, usable
        ));
    }

    Ok(CbsemBootstrapAnalysisV2 {
        method_version: CBSEM_BOOTSTRAP_METHOD_VERSION_V2.into(),
        algorithm: CBSEM_BOOTSTRAP_ALGORITHM_V2.into(),
        interval_method: CBSEM_BOOTSTRAP_INTERVAL_METHOD_V2.into(),
        retry_policy: CBSEM_BOOTSTRAP_RETRY_POLICY_V2.into(),
        confidence_level: source.settings.confidence_level,
        requested_replicates: requested,
        attempted_fits: attempted_fits as u32,
        usable_replicates: usable as u32,
        failed_replicates: failed_replicates as u32,
        minimum_usable_fraction: CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2,
        minimum_usable_replicates: required as u32,
        max_attempts_per_replicate: CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2,
        complete_case_sample_size: complete_rows.len(),
        seed,
        stream_token: CBSEM_BOOTSTRAP_STREAM_TOKEN_V2.into(),
        inference,
        intervals,
        failures,
        validation_witness: CbsemBootstrapValidationWitnessV2 {
            method_version: CBSEM_BOOTSTRAP_VALIDATION_WITNESS_V2.into(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe_sha256,
            base_result_sha256,
            parameter_names,
            successful_replicates,
        },
        warnings,
    })
}

fn cbsem_inference_summary(
    parameter_names: &[String],
    original_estimates: &[f64],
    successful_replicates: &[CbsemBootstrapWitnessReplicateV2],
    confidence_level: f64,
    required: usize,
) -> Result<
    (
        CbsemBootstrapInferenceV2,
        Vec<CbsemBootstrapParameterIntervalV2>,
    ),
    CbsemBootstrapError,
> {
    let usable = successful_replicates.len();
    if usable < required {
        return Ok((
            CbsemBootstrapInferenceV2::Unavailable {
                reason_code: "insufficient_usable_replicates".into(),
                message: format!(
                    "CB-SEM bootstrap inference is unavailable because {usable} usable primary fits are below the required {required}; no intervals were emitted."
                ),
            },
            Vec::new(),
        ));
    }
    Ok((
        CbsemBootstrapInferenceV2::Available,
        summarize_cbsem_percentile_intervals(
            parameter_names,
            original_estimates,
            successful_replicates,
            confidence_level,
        )?,
    ))
}

fn canonical_sha256(value: &impl Serialize) -> Result<String, String> {
    fn sorted_json(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Array(values) => {
                serde_json::Value::Array(values.into_iter().map(sorted_json).collect())
            }
            serde_json::Value::Object(object) => {
                let mut entries = object.into_iter().collect::<Vec<_>>();
                entries.sort_by(|left, right| left.0.cmp(&right.0));
                let mut sorted = serde_json::Map::new();
                for (key, value) in entries {
                    sorted.insert(key, sorted_json(value));
                }
                serde_json::Value::Object(sorted)
            }
            serde_json::Value::Number(number)
                if number.as_i64().is_none() && number.as_u64().is_none() =>
            {
                // serde_json::Value can round an f64 by one final decimal
                // digit when a typed payload passes through the archive's
                // untyped Value envelope. Thirteen significant decimal digits
                // are stricter than the estimator's 1e-10 validation boundary
                // while making identity stable across that representation
                // boundary. The `f64:` tag prevents numeric/string ambiguity.
                let value = number
                    .as_f64()
                    .expect("finite JSON number has an f64 representation");
                serde_json::Value::String(format!("f64:{value:.12e}"))
            }
            scalar => scalar,
        }
    }
    let value = serde_json::to_value(value)
        .map_err(|error| format!("scientific identity serialization failed: {error}"))?;
    let bytes = serde_json::to_vec(&sorted_json(value))
        .map_err(|error| format!("scientific identity canonicalization failed: {error}"))?;
    let mut digest = Sha256::new();
    digest.update(b"QuickPLS CB-SEM bootstrap identity v2\0");
    digest.update((bytes.len() as u64).to_le_bytes());
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}

pub fn cbsem_bootstrap_scientific_recipe_sha256(
    recipe: &qpls_core::AnalysisRecipe,
) -> Result<String, String> {
    let mut scientific = recipe.clone();
    // Worker count is operational provenance. Normalizing it keeps the bound
    // scientific recipe identity (and therefore the analytical payload) exactly
    // invariant when only the thread-pool size changes.
    scientific.settings.workers = 1;
    // Project validation uses the schema-v3 compatibility projection, which
    // adds legacy executable metadata derived from the authoritative typed
    // method_config. Bind only the original annotation metadata: derived keys
    // are redundant, while a user-provided executable-key conflict is already
    // rejected before this function is reachable.
    if scientific.schema_version == qpls_core::ANALYSIS_RECIPE_SCHEMA_VERSION {
        let original_keys = scientific.metadata.keys().cloned().collect::<Vec<_>>();
        if let Ok(projected) = scientific.with_effective_metadata() {
            for (key, projected_value) in projected.metadata {
                if !original_keys.contains(&key)
                    && scientific.metadata.get(&key) == Some(&projected_value)
                {
                    scientific.metadata.remove(&key);
                }
            }
        } else {
            // If this is already an effective compatibility view, reconstruct
            // the typed projection from an annotation-only clone and remove
            // only exact derived key/value pairs.
            let mut annotations = scientific.clone();
            for key in qpls_core::EXECUTABLE_LEGACY_METADATA_KEYS {
                annotations.metadata.remove(*key);
            }
            if let Ok(projected) = annotations.with_effective_metadata() {
                for (key, projected_value) in projected.metadata {
                    if annotations.metadata.get(&key).is_none()
                        && scientific.metadata.get(&key) == Some(&projected_value)
                    {
                        scientific.metadata.remove(&key);
                    }
                }
            }
        }
    }
    canonical_sha256(&scientific)
}

pub const fn cbsem_bootstrap_primary_operation() -> &'static str {
    CBSEM_BOOTSTRAP_PRIMARY_OPERATION_V2
}

pub fn cbsem_bootstrap_required_usable_replicates(requested: u32) -> usize {
    ((requested as f64 * CBSEM_BOOTSTRAP_MINIMUM_USABLE_FRACTION_V2).ceil() as usize)
        .max(MINIMUM_USABLE_REPLICATES as usize)
}

fn cbsem_refit_error_code(error: &EstimationError) -> &'static str {
    match error {
        EstimationError::Cancelled => "cancelled",
        EstimationError::InsufficientObservations => "insufficient_complete_cases",
        EstimationError::ConstantIndicator(_) => "constant_indicator",
        EstimationError::RankDeficient(_) => "rank_deficient",
        EstimationError::NonConvergence(_) => "ml_nonconvergence",
        EstimationError::Numerical(message)
            if message.to_ascii_lowercase().contains("singular") =>
        {
            "singular_covariance"
        }
        EstimationError::Numerical(_) => "numerical_failure",
        EstimationError::UnsupportedMethod(_) => "inadmissible_or_unsupported_refit",
        EstimationError::InvalidIndicator(_) => "invalid_indicator",
        _ => "ml_refit_error",
    }
}

pub fn cbsem_bootstrap_base_result_sha256(result: &PlsResult) -> Result<String, String> {
    let mut point = result.clone();
    if let Some(cbsem) = point.cbsem.as_mut() {
        cbsem.bootstrap_v2 = None;
    }
    canonical_sha256(&point)
}

pub fn cbsem_bootstrap_sample_indices_sha256(
    master_seed: u64,
    replicate_index: u32,
    sampling_frame_positions: &[usize],
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"QuickPLS CB-SEM bootstrap sample witness v2\0");
    digest.update(master_seed.to_le_bytes());
    digest.update(replicate_index.to_le_bytes());
    digest.update((sampling_frame_positions.len() as u64).to_le_bytes());
    for index in sampling_frame_positions {
        digest.update((*index as u64).to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn cbsem_witness_from_estimate(
    replicate_index: u32,
    sample_indices_sha256: String,
    expected_parameter_names: &[String],
    estimate: PlsResult,
    expected_sample_size: usize,
) -> Result<CbsemBootstrapWitnessReplicateV2, (String, String)> {
    let Some(cbsem) = estimate.cbsem else {
        return Err((
            "missing_cbsem_payload".into(),
            "ML refit completed without a CB-SEM payload".into(),
        ));
    };
    if !matches!(
        cbsem.method_version.as_str(),
        CBSEM_ML_METHOD_VERSION | CFA_ML_METHOD_VERSION
    ) || !cbsem.converged
    {
        return Err((
            "ml_nonconvergence".into(),
            "CB-SEM ML bootstrap fit did not converge under the v1 optimizer".into(),
        ));
    }
    if cbsem.sample_size != expected_sample_size
        || estimate.used_observations != expected_sample_size
        || estimate.omitted_observations != 0
    {
        return Err((
            "sample_size_mismatch".into(),
            "CB-SEM ML bootstrap fit changed the fixed complete-case sample size".into(),
        ));
    }
    let parameters = cbsem
        .parameters
        .into_iter()
        .filter(|parameter| !parameter.fixed)
        .collect::<Vec<_>>();
    let actual_names = parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<Vec<_>>();
    if actual_names != expected_parameter_names {
        return Err((
            "parameter_identity_mismatch".into(),
            "CB-SEM ML bootstrap fit changed free-parameter identity or order".into(),
        ));
    }
    let parameter_estimates = parameters
        .into_iter()
        .map(|parameter| parameter.estimate)
        .collect::<Vec<_>>();
    if !cbsem.objective.is_finite() || parameter_estimates.iter().any(|value| !value.is_finite()) {
        return Err((
            "nonfinite_ml_fit".into(),
            "CB-SEM ML bootstrap fit produced a nonfinite objective or parameter".into(),
        ));
    }
    Ok(CbsemBootstrapWitnessReplicateV2 {
        replicate_index,
        sample_indices_sha256,
        iterations: cbsem.iterations,
        objective: cbsem.objective,
        parameter_estimates,
    })
}

pub fn summarize_cbsem_percentile_intervals(
    parameter_names: &[String],
    original_estimates: &[f64],
    successful_replicates: &[CbsemBootstrapWitnessReplicateV2],
    confidence_level: f64,
) -> Result<Vec<CbsemBootstrapParameterIntervalV2>, CbsemBootstrapError> {
    if parameter_names.is_empty()
        || parameter_names.len() != original_estimates.len()
        || successful_replicates.len() < 2
        || !(0.0..1.0).contains(&confidence_level)
        || original_estimates.iter().any(|value| !value.is_finite())
        || successful_replicates.iter().any(|replicate| {
            replicate.parameter_estimates.len() != parameter_names.len()
                || replicate
                    .parameter_estimates
                    .iter()
                    .any(|value| !value.is_finite())
        })
    {
        return Err(CbsemBootstrapError::InconsistentResult(
            "invalid parameter identity, replicate dimensions, confidence level, or finite values"
                .into(),
        ));
    }
    let lower_probability = (1.0 - confidence_level) / 2.0;
    let upper_probability = 1.0 - lower_probability;
    let mut intervals = Vec::with_capacity(parameter_names.len());
    for (parameter_index, (parameter, original)) in
        parameter_names.iter().zip(original_estimates).enumerate()
    {
        let mut values = successful_replicates
            .iter()
            .map(|replicate| replicate.parameter_estimates[parameter_index])
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        let bootstrap_mean = values.iter().sum::<f64>() / values.len() as f64;
        let standard_error = (values
            .iter()
            .map(|value| (value - bootstrap_mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64)
            .sqrt();
        intervals.push(CbsemBootstrapParameterIntervalV2 {
            parameter: parameter.clone(),
            original: *original,
            bootstrap_mean,
            bias: bootstrap_mean - original,
            standard_error,
            percentile_lower: quantile_type7(&values, lower_probability),
            percentile_upper: quantile_type7(&values, upper_probability),
            usable_replicates: values.len() as u32,
        });
    }
    Ok(intervals)
}

fn quantile_type7(sorted: &[f64], probability: f64) -> f64 {
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
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

    fn witness(index: u32, estimates: Vec<f64>) -> CbsemBootstrapWitnessReplicateV2 {
        CbsemBootstrapWitnessReplicateV2 {
            replicate_index: index,
            sample_indices_sha256: format!("digest-{index}"),
            iterations: 4,
            objective: 0.01,
            parameter_estimates: estimates,
        }
    }

    #[test]
    fn type7_percentile_summary_is_exact_and_not_normal_theory() {
        let names = vec!["eta~xi".into()];
        let values = vec![
            witness(0, vec![-4.0]),
            witness(1, vec![-1.0]),
            witness(2, vec![0.0]),
            witness(3, vec![2.0]),
            witness(4, vec![9.0]),
        ];
        let summary = summarize_cbsem_percentile_intervals(&names, &[1.0], &values, 0.8)
            .expect("valid percentile summary");
        assert_eq!(summary.len(), 1);
        assert!((summary[0].percentile_lower - -2.8).abs() < 1e-12);
        assert!((summary[0].percentile_upper - 6.2).abs() < 1e-12);
        assert!((summary[0].bootstrap_mean - 1.2).abs() < 1e-12);
        assert_ne!(
            summary[0].percentile_lower,
            summary[0].original - 1.96 * summary[0].standard_error
        );
    }

    #[test]
    fn primary_draw_and_sample_witness_are_deterministic_without_retry() {
        assert_eq!(CBSEM_BOOTSTRAP_MAX_ATTEMPTS_PER_REPLICATE_V2, 1);
        assert_eq!(
            CBSEM_BOOTSTRAP_RETRY_POLICY_V2,
            "no_retry_fixed_preplanned_primary_draws_v1"
        );
        let primary = bootstrap_indices(25, 47, cbsem_bootstrap_primary_operation(), 8);
        assert_eq!(
            primary,
            bootstrap_indices(25, 47, cbsem_bootstrap_primary_operation(), 8)
        );
        assert_eq!(
            cbsem_bootstrap_sample_indices_sha256(47, 8, &primary),
            cbsem_bootstrap_sample_indices_sha256(47, 8, &primary)
        );
    }

    #[test]
    fn usable_threshold_matches_the_frozen_minimum_and_fraction() {
        assert_eq!(cbsem_bootstrap_required_usable_replicates(500), 1_000);
        assert_eq!(cbsem_bootstrap_required_usable_replicates(1_000), 1_000);
        assert_eq!(cbsem_bootstrap_required_usable_replicates(1_100), 1_000);
        assert_eq!(cbsem_bootstrap_required_usable_replicates(2_000), 1_800);
        assert_eq!(cbsem_bootstrap_required_usable_replicates(10_000), 9_000);
    }

    #[test]
    fn complete_500_replicate_pilot_is_typed_unavailable_without_intervals() {
        let names = vec!["eta~xi".into()];
        let successful = (0..500)
            .map(|index| witness(index, vec![f64::from(index) / 500.0]))
            .collect::<Vec<_>>();
        let required = cbsem_bootstrap_required_usable_replicates(500);
        assert_eq!(required, 1_000);
        let (inference, intervals) =
            cbsem_inference_summary(&names, &[0.3], &successful, 0.95, required).unwrap();
        assert!(intervals.is_empty());
        assert!(matches!(
            inference,
            CbsemBootstrapInferenceV2::Unavailable {
                ref reason_code,
                ..
            } if reason_code == "insufficient_usable_replicates"
        ));
    }

    #[test]
    fn below_threshold_completion_keeps_status_and_emits_no_intervals() {
        let names = vec!["eta~xi".into()];
        let successful = vec![
            witness(0, vec![0.1]),
            witness(1, vec![0.2]),
            witness(2, vec![0.3]),
            witness(3, vec![0.4]),
            witness(4, vec![0.5]),
        ];
        let (inference, intervals) =
            cbsem_inference_summary(&names, &[0.3], &successful, 0.95, 6).unwrap();
        assert!(intervals.is_empty());
        assert!(matches!(
            inference,
            CbsemBootstrapInferenceV2::Unavailable {
                ref reason_code,
                ..
            } if reason_code == "insufficient_usable_replicates"
        ));
    }

    #[test]
    fn summary_rejects_dimension_and_nonfinite_drift() {
        let names = vec!["eta~xi".into(), "eta~~eta".into()];
        let short = vec![witness(0, vec![0.1]), witness(1, vec![0.2])];
        assert!(summarize_cbsem_percentile_intervals(&names, &[0.1, 1.0], &short, 0.95).is_err());
        let nonfinite = vec![witness(0, vec![f64::NAN]), witness(1, vec![0.2])];
        assert!(
            summarize_cbsem_percentile_intervals(&["eta~xi".into()], &[0.1], &nonfinite, 0.95)
                .is_err()
        );
    }

    #[test]
    fn refit_failures_have_stable_reason_codes() {
        assert_eq!(
            cbsem_refit_error_code(&EstimationError::ConstantIndicator("x1".into())),
            "constant_indicator"
        );
        assert_eq!(
            cbsem_refit_error_code(&EstimationError::Numerical(
                "singular covariance matrix".into()
            )),
            "singular_covariance"
        );
        assert_eq!(
            cbsem_refit_error_code(&EstimationError::NonConvergence(3_000)),
            "ml_nonconvergence"
        );
    }
}
