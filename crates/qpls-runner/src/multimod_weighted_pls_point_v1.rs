//! Production WPLS execution for the additive MultiMod compiled authority.

use crate::RunnerProgress;
use qpls_core::{
    AnalysisMethod, AnalysisRecipeV4, AnalysisWeightBindingV1, MethodConfig,
    MultimodCompiledWeightSemanticsV1, MultimodCompiledWeightedPlsRecipeV1, SemModelV4,
    StructuralRelationRoleV4, ValidatedExecutionRecipe,
    validate_multimod_weighted_pls_recipe_v4_v1,
};
use qpls_data::{DataKind, Dataset};
use qpls_estimation::{
    EstimationError, PlsPointEstimateAttributionV1, PlsResult, WPLS_METHOD_VERSION,
    estimate_pls_validated_with_control,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MULTIMOD_WEIGHTED_PLS_POINT_EXECUTION_VERSION_V1: &str =
    "multimod_weighted_pls_point_execution_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodWeightedPlsPointReceiptV1 {
    pub method_version: String,
    pub compiler_version: String,
    pub source_recipe_analytical_sha256: String,
    pub source_model_scientific_sha256: String,
    pub point_plan_sha256: String,
    pub dataset_fingerprint: String,
    pub weight_semantics: MultimodCompiledWeightSemanticsV1,
    pub weight_source_column: String,
    pub compact_row_count: usize,
    pub applied_weight_sum: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodWeightedPlsPointResultV1 {
    pub estimation: PlsResult,
    pub receipt: MultimodWeightedPlsPointReceiptV1,
}

#[derive(Debug, thiserror::Error)]
pub enum MultimodWeightedPlsPointErrorV1 {
    #[error("MultiMod weighted PLS point execution was cancelled")]
    Cancelled,
    #[error("compiled MultiMod weighted PLS authority is invalid: {0}")]
    Authority(String),
    #[error("MultiMod weighted PLS requires raw observations")]
    RawDataRequired,
    #[error("execution dataset differs from the compiled weighted authority")]
    DatasetIdentity,
    #[error("weighted point recipe projection failed: {0}")]
    Projection(String),
    #[error("weighted point estimation failed: {0}")]
    Estimation(String),
    #[error("weighted point result differs from its compiled plan: {0}")]
    ResultContract(String),
}

/// Runner-internal proof that the deterministic weighted authority has been
/// validated against its exact group-free source recipe/model once. MGA keeps
/// this value for the lifetime of a run so thousands of resampled point fits
/// cannot accidentally trigger thousands of compiler replays.
#[derive(Debug, Clone)]
pub(crate) struct PreparedMultimodWeightedPlsPointV1 {
    authority: MultimodCompiledWeightedPlsRecipeV1,
    execution: ValidatedExecutionRecipe,
}

impl PreparedMultimodWeightedPlsPointV1 {
    pub(crate) fn point_recipe(&self) -> &AnalysisRecipeV4 {
        self.authority.point_recipe()
    }

    pub(crate) fn point_model(&self) -> &SemModelV4 {
        self.authority.point_model()
    }

    pub(crate) fn plan(&self) -> &qpls_core::CompiledPlsPlanV2 {
        self.authority.plan()
    }

    #[cfg(test)]
    pub(crate) fn compiled_authority_for_test(&self) -> &MultimodCompiledWeightedPlsRecipeV1 {
        &self.authority
    }
}

pub(crate) fn prepare_compiled_multimod_weighted_pls_point_v1(
    source_recipe: &AnalysisRecipeV4,
    source_model: &SemModelV4,
    requested_weight: &AnalysisWeightBindingV1,
    authority: &MultimodCompiledWeightedPlsRecipeV1,
) -> Result<PreparedMultimodWeightedPlsPointV1, MultimodWeightedPlsPointErrorV1> {
    validate_multimod_weighted_pls_recipe_v4_v1(
        authority,
        source_recipe,
        source_model,
        requested_weight,
    )
    .map_err(|error| MultimodWeightedPlsPointErrorV1::Authority(error.to_string()))?;
    let mut projected = crate::project_pls_plan_to_current_recipe(
        authority.point_recipe(),
        authority.point_model(),
        authority.plan(),
    )
    .map_err(|error| MultimodWeightedPlsPointErrorV1::Projection(error.to_string()))?;
    projected.settings.method = AnalysisMethod::Wpls;
    projected.settings.case_weight_column =
        Some(authority.receipt().weight_source_column().to_string());
    projected.method_config = Some(MethodConfig::Wpls);
    let execution = ValidatedExecutionRecipe::for_dataset(
        &projected,
        authority.receipt().source_dataset_fingerprint(),
    )
    .map_err(|error| MultimodWeightedPlsPointErrorV1::Projection(error.to_string()))?;
    Ok(PreparedMultimodWeightedPlsPointV1 {
        authority: authority.clone(),
        execution,
    })
}

/// Execute one fully weighted PLS score/path fit.  The caller prepares the
/// dataset weight column first (mean-one case weights or positive compact
/// frequency counts); this function applies it in every production WPLS
/// preprocessing, scoring, loading, and structural stage.
pub fn run_compiled_multimod_weighted_pls_point_v1(
    dataset: &Dataset,
    source_recipe: &AnalysisRecipeV4,
    source_model: &SemModelV4,
    requested_weight: &AnalysisWeightBindingV1,
    authority: &MultimodCompiledWeightedPlsRecipeV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<MultimodWeightedPlsPointResultV1, MultimodWeightedPlsPointErrorV1> {
    if should_cancel() {
        return Err(MultimodWeightedPlsPointErrorV1::Cancelled);
    }
    let prepared = prepare_compiled_multimod_weighted_pls_point_v1(
        source_recipe,
        source_model,
        requested_weight,
        authority,
    )?;
    run_prepared_multimod_weighted_pls_point_v1(dataset, &prepared, should_cancel, progress)
}

pub(crate) fn run_prepared_multimod_weighted_pls_point_v1(
    dataset: &Dataset,
    prepared: &PreparedMultimodWeightedPlsPointV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<MultimodWeightedPlsPointResultV1, MultimodWeightedPlsPointErrorV1> {
    if should_cancel() {
        return Err(MultimodWeightedPlsPointErrorV1::Cancelled);
    }
    let authority = &prepared.authority;
    if dataset.schema.kind != DataKind::Raw {
        return Err(MultimodWeightedPlsPointErrorV1::RawDataRequired);
    }
    if dataset.fingerprint.0 != authority.receipt().source_dataset_fingerprint()
        || dataset.id.to_string() != authority.plan().dataset_id()
    {
        return Err(MultimodWeightedPlsPointErrorV1::DatasetIdentity);
    }

    let mut report_progress = |update: qpls_estimation::EstimationProgress| {
        progress(RunnerProgress {
            phase: format!("multimod_weighted_pls:{}", update.phase.as_str()),
            completed_units: update.completed_units,
            total_units: update.total_units,
        });
        !should_cancel()
    };
    let estimation =
        estimate_pls_validated_with_control(dataset, &prepared.execution, &mut report_progress)
            .map_err(|error| match error {
                EstimationError::Cancelled => MultimodWeightedPlsPointErrorV1::Cancelled,
                other => MultimodWeightedPlsPointErrorV1::Estimation(other.to_string()),
            })?;
    validate_weighted_point_result_v1(authority, dataset, &estimation)?;
    let applied_weight_sum = estimation
        .wpls
        .as_ref()
        .expect("validated weighted point has WPLS metadata")
        .weight_sum;
    Ok(MultimodWeightedPlsPointResultV1 {
        estimation,
        receipt: MultimodWeightedPlsPointReceiptV1 {
            method_version: MULTIMOD_WEIGHTED_PLS_POINT_EXECUTION_VERSION_V1.into(),
            compiler_version: authority.receipt().compiler_version().into(),
            source_recipe_analytical_sha256: authority
                .receipt()
                .source_recipe_analytical_sha256()
                .into(),
            source_model_scientific_sha256: authority
                .receipt()
                .source_model_scientific_sha256()
                .into(),
            point_plan_sha256: qpls_core::sha256_serialized(authority.plan()),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            weight_semantics: authority.receipt().weight_semantics(),
            weight_source_column: authority.receipt().weight_source_column().into(),
            compact_row_count: dataset.batch.num_rows(),
            applied_weight_sum,
        },
    })
}

fn validate_weighted_point_result_v1(
    authority: &MultimodCompiledWeightedPlsRecipeV1,
    dataset: &Dataset,
    result: &PlsResult,
) -> Result<(), MultimodWeightedPlsPointErrorV1> {
    if !result.converged
        || result.method_version != WPLS_METHOD_VERSION
        || result.used_observations != dataset.batch.num_rows()
        || result.omitted_observations != 0
        || result.point_estimate_attribution.as_ref()
            != Some(&PlsPointEstimateAttributionV1::for_preprocessing(
                authority.point_recipe().settings.preprocessing.clone(),
            ))
    {
        return Err(MultimodWeightedPlsPointErrorV1::ResultContract(
            "method, convergence, row accounting, or point scale is invalid".into(),
        ));
    }
    let Some(wpls) = &result.wpls else {
        return Err(MultimodWeightedPlsPointErrorV1::ResultContract(
            "production WPLS metadata is absent".into(),
        ));
    };
    if wpls.method_version != WPLS_METHOD_VERSION
        || wpls.case_weight_column != authority.receipt().weight_source_column()
        || !wpls.weight_sum.is_finite()
        || wpls.weight_sum <= 0.0
    {
        return Err(MultimodWeightedPlsPointErrorV1::ResultContract(
            "production WPLS receipt differs from the compiled binding".into(),
        ));
    }
    let expected_constructs = authority
        .plan()
        .blocks()
        .iter()
        .map(|block| block.construct_id())
        .collect::<BTreeSet<_>>();
    let actual_constructs = result
        .construct_scores
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if expected_constructs != actual_constructs
        || result.construct_scores.values().any(|scores| {
            scores.len() != dataset.batch.num_rows()
                || scores.iter().any(|value| !value.is_finite())
        })
    {
        return Err(MultimodWeightedPlsPointErrorV1::ResultContract(
            "construct-score inventory differs from the compiled point plan".into(),
        ));
    }
    for path in authority.plan().paths() {
        let count = if path.role() == StructuralRelationRoleV4::Control {
            result
                .control_estimates
                .iter()
                .filter(|estimate| {
                    estimate.source == path.source() && estimate.target == path.target()
                })
                .count()
        } else {
            result
                .paths
                .iter()
                .filter(|estimate| {
                    estimate.source == path.source() && estimate.target == path.target()
                })
                .count()
        };
        if count != 1 {
            return Err(MultimodWeightedPlsPointErrorV1::ResultContract(format!(
                "compiled relation {} has {count} weighted estimates",
                path.relation_id()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_identity_is_stable() {
        assert_eq!(
            MULTIMOD_WEIGHTED_PLS_POINT_EXECUTION_VERSION_V1,
            "multimod_weighted_pls_point_execution_v1"
        );
    }
}
