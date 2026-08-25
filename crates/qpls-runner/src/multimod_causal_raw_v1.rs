//! Runner-owned raw-data execution for the separate observed interventional
//! mediation V1 workflow.
//!
//! The public entry point binds the compiled Recipe V4 authority directly to
//! the selected dataset, derives one shared complete-case frame, refits every
//! declared observed equation in every case-bootstrap draw, and retains the
//! immutable no-retry draw ledger as sidecar evidence.

use crate::{
    MultiModRunOutputV1, MultiModRunnerErrorV1, MultiModRunnerEvidenceV1, MultiModRunnerPhaseV1,
    MultiModRunnerProgressV1, causal_effects,
    multimod_row_order_v1::canonical_multimod_row_permutation_v1, provenance, report,
    validate_authority,
};
use qpls_core::{
    AnalysisRecipeV4, CausalPositivityDiagnosticV1, CompiledMultiModRecipeV1,
    INTERVENTIONAL_MEDIATION_RESULT_INTERPRETATION_LABEL_V1,
    INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION, InferenceAlternativeV1,
    InterventionalMediationResultV1, MULTIMOD_SIDECAR_MAX_BYTES_V1, MultiModAnalysisResultV1,
    MultiModCompilerTargetV1, MultimodIntervalV1, MultimodReplicateFailureKindV1,
    MultimodReplicateFailureV1, MultimodReplicateLedgerSummaryV1, SemModelV4, SemVariableV4,
    sha256_serialized,
};
use qpls_data::{ColumnType, Dataset};
use qpls_estimation::{
    ConditionalAlternativeV2, InterventionalMediationBlockerCodeV1,
    InterventionalMediationResultV1 as EstimationInterventionalMediationResultV1,
    estimate_interventional_mediation_v1, percentile_interval_v2,
    prepare_interventional_causal_inputs_from_dataset_v1,
};
use qpls_resampling::{
    MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1, MultiModBootstrapPlanV1, MultiModCaseBootstrapDrawV1,
    MultiModFinalLedgerV1, MultiModRefitFailureV1, MultiModRefitOutcomeV1, MultiModShardSpecV1,
    finalize_multimod_case_bootstrap_v1, run_multimod_case_bootstrap_shard_v1,
};
use std::collections::{BTreeMap, BTreeSet};

pub type InterventionalFullRefitLedgerV1 =
    MultiModFinalLedgerV1<MultiModCaseBootstrapDrawV1, Vec<f64>>;

fn causal_sampling_source_rows_v1(
    dataset: &Dataset,
    model: &SemModelV4,
    config: &qpls_core::InterventionalCausalMediationConfigV1,
    base_source_rows: &[usize],
) -> Result<Vec<usize>, MultiModRunnerErrorV1> {
    let mut used_variable_ids =
        BTreeSet::from([config.treatment.as_str(), config.outcome.as_str()]);
    used_variable_ids.extend(config.mediators.iter().map(String::as_str));
    used_variable_ids.extend(config.baseline_moderators.iter().map(String::as_str));
    used_variable_ids.extend(config.adjustment_covariates.iter().map(String::as_str));
    used_variable_ids.extend(
        config
            .positivity_policy
            .positivity_strata_variable_ids
            .iter()
            .map(String::as_str),
    );
    for path in &config.paths {
        used_variable_ids.extend(path.ordered_variable_ids.iter().map(String::as_str));
        for equation in &path.equations {
            used_variable_ids.insert(equation.outcome_variable_id.as_str());
            for term in &equation.terms {
                used_variable_ids.extend(term.factor_variable_ids.iter().map(String::as_str));
            }
        }
    }

    let observed_columns = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((id.as_str(), source_column.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let columns = used_variable_ids
        .iter()
        .map(|variable_id| {
            observed_columns
                .get(variable_id)
                .map(|column| (*column).to_owned())
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(format!(
                        "causal scientific row order cannot bind observed variable {variable_id}"
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let numeric_columns = dataset
        .schema
        .columns
        .iter()
        .filter(|column| column.column_type == ColumnType::Numeric)
        .map(|column| column.name.as_str())
        .collect::<BTreeSet<_>>();
    let sign_invariant_numeric_columns = config
        .adjustment_covariates
        .iter()
        .filter_map(|variable_id| observed_columns.get(variable_id.as_str()))
        .filter(|column| numeric_columns.contains(**column))
        .map(|column| (*column).to_owned())
        .collect::<BTreeSet<_>>();
    let canonical_source_rows = base_source_rows
        .iter()
        .map(|source_row| {
            u32::try_from(*source_row).map_err(|_| {
                MultiModRunnerErrorV1::PreparedInput(
                    "causal complete-case row exceeds the bootstrap row-index contract".into(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let permutation = canonical_multimod_row_permutation_v1(
        dataset,
        &canonical_source_rows,
        &columns,
        &sign_invariant_numeric_columns,
    )
    .map_err(MultiModRunnerErrorV1::PreparedInput)?;
    Ok(permutation
        .into_iter()
        .map(|position| base_source_rows[position])
        .collect())
}

fn predicted_causal_sidecar_bytes_v1(rows: usize, replicates: u32, targets: usize) -> u64 {
    let per_draw = (rows as u64)
        .saturating_mul(4)
        .saturating_add((targets as u64).saturating_mul(8))
        .saturating_add(512);
    per_draw
        .saturating_mul(u64::from(replicates))
        .saturating_add((rows as u64).saturating_mul(96))
}

fn classify_refit_failure_v1(code: &str) -> MultimodReplicateFailureKindV1 {
    if code.contains("rank") {
        MultimodReplicateFailureKindV1::RankDeficient
    } else if code.contains("nonfinite") {
        MultimodReplicateFailureKindV1::NonfiniteEstimate
    } else if code.contains("insufficient") || code.contains("observation") {
        MultimodReplicateFailureKindV1::InsufficientCases
    } else {
        MultimodReplicateFailureKindV1::Other
    }
}

fn blocker_failure_v1(
    blockers: &[qpls_estimation::InterventionalMediationBlockerV1],
) -> MultiModRefitFailureV1 {
    let code = if blockers
        .iter()
        .any(|blocker| blocker.code == InterventionalMediationBlockerCodeV1::RankDeficientEquation)
    {
        "multimod.causal.bootstrap.rank_deficient"
    } else if blockers.iter().any(|blocker| {
        matches!(
            blocker.code,
            InterventionalMediationBlockerCodeV1::NonFiniteFit
                | InterventionalMediationBlockerCodeV1::NonFiniteObservedValue
        )
    }) {
        "multimod.causal.bootstrap.nonfinite"
    } else if blockers
        .iter()
        .any(|blocker| blocker.code == InterventionalMediationBlockerCodeV1::ObservationShape)
    {
        "multimod.causal.bootstrap.insufficient_observations"
    } else {
        "multimod.causal.bootstrap.refit_failed"
    };
    MultiModRefitFailureV1 {
        code: code.into(),
        message: blockers
            .iter()
            .map(|blocker| format!("{:?}: {}", blocker.code, blocker.detail))
            .collect::<Vec<_>>()
            .join("; "),
    }
}

fn estimate_path_vector_v1(
    prepared: &[qpls_estimation::PreparedInterventionalDatasetPathV1],
) -> Result<(Vec<EstimationInterventionalMediationResultV1>, Vec<f64>), MultiModRefitFailureV1> {
    let mut results = Vec::with_capacity(prepared.len());
    let mut estimates = Vec::with_capacity(prepared.len() * 3);
    for path in prepared {
        let result = estimate_interventional_mediation_v1(&path.input)
            .map_err(|blockers| blocker_failure_v1(&blockers))?;
        estimates.extend([
            result.interventional_direct_effect,
            result.joint_interventional_indirect_effect,
            result.total_interventional_contrast,
        ]);
        results.push(result);
    }
    if estimates.iter().any(|value| !value.is_finite()) {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.causal.bootstrap.nonfinite".into(),
            message: "the complete causal refit returned a nonfinite target".into(),
        });
    }
    Ok((results, estimates))
}

fn two_sided_empirical_zero_probability_v1(draws: &[f64]) -> f64 {
    let denominator = (draws.len() + 1) as f64;
    let lower = (1 + draws.iter().filter(|value| **value <= 0.0).count()) as f64 / denominator;
    let upper = (1 + draws.iter().filter(|value| **value >= 0.0).count()) as f64 / denominator;
    (2.0 * lower.min(upper)).min(1.0)
}

fn ledger_summary_v1(ledger: &InterventionalFullRefitLedgerV1) -> MultimodReplicateLedgerSummaryV1 {
    let mut failure_counts = BTreeMap::new();
    let mut failures = Vec::new();
    for record in &ledger.records {
        let MultiModRefitOutcomeV1::Failed { failure, .. } = &record.outcome else {
            continue;
        };
        *failure_counts.entry(failure.code.clone()).or_insert(0) += 1;
        failures.push(MultimodReplicateFailureV1 {
            replicate_index: record.index,
            kind: classify_refit_failure_v1(&failure.code),
            stable_code: failure.code.clone(),
            detail: failure.message.clone(),
        });
    }
    MultimodReplicateLedgerSummaryV1 {
        requested: ledger.requested,
        usable: ledger.usable,
        minimum_required: ledger.minimum_required,
        usable_fraction: ledger.usable_fraction,
        complete: ledger.complete,
        ledger_sha256: ledger.ledger_sha256.clone(),
        failure_counts,
        failures,
    }
}

fn positivity_rows_v1(
    path_id: &str,
    result: &EstimationInterventionalMediationResultV1,
    minimum_required_count: u64,
    support_rule: &str,
) -> Vec<CausalPositivityDiagnosticV1> {
    [
        ("x0", result.x0, result.positivity.x0_support_count),
        ("x1", result.x1, result.positivity.x1_support_count),
    ]
    .into_iter()
    .map(
        |(label, requested_value, support_count)| CausalPositivityDiagnosticV1 {
            variable_id: format!("{path_id}:{}:{label}", result.treatment_variable_id),
            observed_minimum: result.positivity.observed_treatment_minimum,
            observed_maximum: result.positivity.observed_treatment_maximum,
            requested_value,
            support_count: support_count as u64,
            minimum_required_count,
            support_rule: support_rule.into(),
            supported: requested_value >= result.positivity.observed_treatment_minimum
                && requested_value <= result.positivity.observed_treatment_maximum
                && support_count as u64 >= minimum_required_count,
        },
    )
    .collect()
}

/// Executes causal V1 from the exact dataset/model/recipe authority. This is
/// distinct from the prepared-adapter entry point retained for external
/// qualification harnesses.
pub fn run_compiled_interventional_causal_mediation_raw_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::ValidatingAuthority,
        0,
        1,
        "interventional_raw:authority",
    );
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::InterventionalCausalMediationV1,
    )?;
    let config = recipe
        .interventional_causal_mediation
        .as_ref()
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "interventional configuration disappeared after compilation".into(),
            )
        })?;
    let prepared_point =
        prepare_interventional_causal_inputs_from_dataset_v1(dataset, model, config, None)
            .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?;
    let base_source_rows = prepared_point
        .first()
        .map(|path| path.source_row_indices.clone())
        .ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput("no causal path was prepared".into())
        })?;
    if prepared_point
        .iter()
        .any(|path| path.source_row_indices != base_source_rows)
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "causal paths did not share one complete-case frame".into(),
        ));
    }
    let sampling_source_rows =
        causal_sampling_source_rows_v1(dataset, model, config, &base_source_rows)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::PointEstimation,
        0,
        config.paths.len() as u64,
        "interventional_raw:point",
    );
    let (point_results, _) = estimate_path_vector_v1(&prepared_point)
        .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.message))?;
    let mut effects = Vec::with_capacity(point_results.len() * 3);
    let mut positivity = Vec::with_capacity(point_results.len() * 2);
    let (minimum_required_count, support_rule) = match config.treatment_contrast {
        qpls_core::ObservedTreatmentContrastV1::Binary { .. } => (
            u64::from(config.positivity_policy.minimum_binary_arm_count),
            "binary_arm_count_and_declared_strata",
        ),
        qpls_core::ObservedTreatmentContrastV1::Continuous { .. } => (
            u64::from(
                config
                    .positivity_policy
                    .minimum_continuous_neighborhood_count,
            ),
            "continuous_neighborhood_count",
        ),
    };
    for (path, result) in config.paths.iter().zip(&point_results) {
        effects.extend(causal_effects(&path.path_id, result));
        positivity.extend(positivity_rows_v1(
            &path.path_id,
            result,
            minimum_required_count,
            support_rule,
        ));
    }
    let target_ids = effects
        .iter()
        .map(|effect| effect.target_id.clone())
        .collect::<Vec<_>>();
    let predicted_bytes = predicted_causal_sidecar_bytes_v1(
        base_source_rows.len(),
        config.bootstrap_resamples,
        target_ids.len(),
    );
    if predicted_bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1 {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.archive.sidecar_limit: predicted causal evidence is {predicted_bytes} bytes"
        )));
    }
    let scientific_refit_identity_sha256 = sha256_serialized(&(
        artifact.receipt().analytical_identity_sha256.as_str(),
        dataset.fingerprint.0.as_str(),
        &base_source_rows,
        &target_ids,
        "interventional_causal_mediation_v1.full_refit.v1",
    ));
    let plan = MultiModBootstrapPlanV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        scientific_refit_identity_sha256,
        requested_replicates: config.bootstrap_resamples,
        master_seed: config.seed,
        minimum_usable_fraction: 0.90,
    };
    let mut completed = 0_u64;
    let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
        let source_rows = draw
            .source_rows
            .iter()
            .map(|position| {
                sampling_source_rows
                    .get(*position as usize)
                    .copied()
                    .ok_or_else(|| MultiModRefitFailureV1 {
                        code: "multimod.causal.bootstrap.draw_identity".into(),
                        message: "bootstrap position is outside the frozen complete-case frame"
                            .into(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let prepared = prepare_interventional_causal_inputs_from_dataset_v1(
            dataset,
            model,
            config,
            Some(&source_rows),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: "multimod.causal.bootstrap.preparation".into(),
            message: error.to_string(),
        })?;
        let (_, estimates) = estimate_path_vector_v1(&prepared)?;
        if estimates.len() != target_ids.len() {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.causal.bootstrap.target_inventory".into(),
                message: "bootstrap refit returned the wrong ordered target vector".into(),
            });
        }
        completed += 1;
        report(
            &progress,
            MultiModRunnerPhaseV1::Resampling,
            completed,
            u64::from(config.bootstrap_resamples),
            format!("interventional_raw:bootstrap:{}", draw.replicate_index),
        );
        Ok(estimates)
    };
    let cache = run_multimod_case_bootstrap_shard_v1(
        &plan,
        base_source_rows.len(),
        None,
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &mut callback,
        &should_cancel,
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    if cache.cancelled || should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    let ledger =
        finalize_multimod_case_bootstrap_v1(&plan, base_source_rows.len(), None, vec![cache])
            .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))?;
    for (target_index, effect) in effects.iter_mut().enumerate() {
        let draws = ledger
            .records
            .iter()
            .filter_map(|record| match &record.outcome {
                MultiModRefitOutcomeV1::Success { value, .. } => value.get(target_index).copied(),
                MultiModRefitOutcomeV1::Failed { .. } => None,
            })
            .collect::<Vec<_>>();
        let inferred = percentile_interval_v2(
            &draws,
            config.bootstrap_resamples as usize,
            config.confidence_level,
            ConditionalAlternativeV2::TwoSided,
        )
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        effect.interval = Some(MultimodIntervalV1 {
            confidence_level: config.confidence_level,
            lower: inferred.lower,
            upper: inferred.upper,
            family: "type_7_two_sided_percentile".into(),
            alternative: InferenceAlternativeV1::TwoSided,
        });
        effect.p_value = Some(two_sided_empirical_zero_probability_v1(&draws));
    }
    let analysis = InterventionalMediationResultV1 {
        schema_version: INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION,
        provenance: provenance(artifact.receipt(), config.seed),
        interpretation_label: INTERVENTIONAL_MEDIATION_RESULT_INTERPRETATION_LABEL_V1.into(),
        identification_assumptions: vec![
            "temporal order, consistency, and the explicit linear model specification were reviewed".into(),
            "the declared adjustment set was judged sufficient for treatment-outcome, treatment-mediator, and mediator-outcome exchangeability".into(),
            "no exposure-induced mediator-outcome confounder or recanting-witness setting was declared".into(),
            "positivity was screened under the frozen treatment contrast and declared baseline strata".into(),
        ],
        positivity,
        effects,
        replicate_ledger: ledger_summary_v1(&ledger),
        sidecars: Vec::new(),
    };
    let result = MultiModAnalysisResultV1::InterventionalMediationResultV1(analysis);
    result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    report(
        &progress,
        MultiModRunnerPhaseV1::Completed,
        1,
        1,
        "interventional_raw:complete",
    );
    Ok(MultiModRunOutputV1 {
        compilation_receipt: artifact.receipt().clone(),
        result,
        evidence: vec![MultiModRunnerEvidenceV1::InterventionalFullRefitLedger(
            ledger,
        )],
    })
}
