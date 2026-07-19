use chrono::Utc;
use qpls_assessment::{ASSESSMENT_METHOD_VERSION, AssessmentError, assess_pls_with_control};
use qpls_core::{AnalysisRecipe, AnalysisResult};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_BOOTSTRAP_METHOD_VERSION, CBSEM_FIT_METHOD_VERSION, CBSEM_ML_METHOD_VERSION,
    CBSEM_MODIFICATION_INDICES_METHOD_VERSION, CBSEM_MULTIGROUP_METHOD_VERSION,
    CFA_ML_METHOD_VERSION, EstimationError, FIMIX_PLS_METHOD_VERSION, GSCA_METHOD_VERSION,
    IPMA_METHOD_VERSION, MICOM_METHOD_VERSION, NCA_METHOD_VERSION, PCA_METHOD_VERSION,
    PLS_MEDIATION_METHOD_VERSION, PLS_METHOD_VERSION, PLS_MGA_METHOD_VERSION,
    PLS_MGA_PERMUTATION_METHOD_VERSION, PLS_POS_METHOD_VERSION, PLS_PREDICT_METHOD_VERSION,
    PLS_SEGMENTATION_METHOD_VERSION, PLS_TWO_STAGE_MODERATION_METHOD_VERSION,
    REGRESSION_LOGISTIC_METHOD_VERSION, REGRESSION_OLS_METHOD_VERSION,
    REGRESSION_PROCESS_METHOD_VERSION, estimate_pls_with_control,
};
use qpls_resampling::{
    PERMUTATION_METHOD_VERSION, PlsBootstrapError, PlsPermutationError, RESAMPLING_METHOD_VERSION,
    ResamplingError, bootstrap_pls, permutation_pls,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerProgress {
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error("PLS estimation failed: {0}")]
    Estimation(String),
    #[error("PLS assessment failed: {0}")]
    Assessment(String),
    #[error("PLS bootstrap failed: {0}")]
    Bootstrap(String),
    #[error("PLS permutation failed: {0}")]
    Permutation(String),
    #[error("result serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub fn run_pls_analysis(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<AnalysisResult, RunnerError> {
    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let started_at = Utc::now();
    let mut base_recipe = recipe.clone();
    base_recipe.settings.bootstrap_samples = 0;
    base_recipe.settings.studentized_inner_samples = 0;
    base_recipe.settings.permutation_samples = 0;

    let estimation = estimate_pls_with_control(dataset, &base_recipe, |update| {
        progress(RunnerProgress {
            phase: update.phase.as_str().into(),
            completed_units: update.completed_units,
            total_units: update.total_units,
        });
        !should_cancel()
    })
    .map_err(|error| match error {
        EstimationError::Cancelled => RunnerError::Cancelled,
        other => RunnerError::Estimation(other.to_string()),
    })?;

    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let standalone_v08 = matches!(
        recipe.settings.method,
        qpls_core::AnalysisMethod::Pca
            | qpls_core::AnalysisMethod::Regression
            | qpls_core::AnalysisMethod::Nca
            | qpls_core::AnalysisMethod::Gsca
    );
    let assessment = if standalone_v08 {
        serde_json::json!({
            "method_version": "assessment_not_applicable_v1",
            "warnings": ["PLS assessment is not applicable to standalone v0.8 methods."]
        })
    } else {
        serde_json::to_value(
            assess_pls_with_control(dataset, &base_recipe, &estimation, |update| {
                progress(RunnerProgress {
                    phase: update.phase.as_str().into(),
                    completed_units: update.completed_units,
                    total_units: update.total_units,
                });
                !should_cancel()
            })
            .map_err(|error| match error {
                AssessmentError::Cancelled => RunnerError::Cancelled,
                other => RunnerError::Assessment(other.to_string()),
            })?,
        )?
    };

    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let bootstrap = if recipe.settings.bootstrap_samples > 0 && !standalone_v08 {
        Some(
            bootstrap_pls(
                dataset,
                recipe,
                &estimation,
                recipe.settings.workers,
                &should_cancel,
                |update| {
                    progress(RunnerProgress {
                        phase: update.phase.as_str().into(),
                        completed_units: update.completed_replicates as u64,
                        total_units: update.total_replicates as u64,
                    });
                },
            )
            .map_err(map_bootstrap_error)?,
        )
    } else {
        None
    };

    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let permutation = if recipe.settings.permutation_samples > 0 && !standalone_v08 {
        Some(
            permutation_pls(
                dataset,
                recipe,
                &estimation,
                recipe.settings.workers,
                &should_cancel,
                |update| {
                    progress(RunnerProgress {
                        phase: update.phase.as_str().into(),
                        completed_units: update.completed_replicates as u64,
                        total_units: update.total_replicates as u64,
                    });
                },
            )
            .map_err(map_permutation_error)?,
        )
    } else {
        None
    };

    let mut warnings = estimation.warnings.clone();
    if standalone_v08 {
        warnings.push("PLS assessment and PLS resampling engines are not applicable to standalone v0.8 methods.".into());
    }
    if let Some(bootstrap) = &bootstrap {
        let failed = bootstrap.failed_replicates.len();
        if failed > 0 {
            warnings.push(format!(
                "{failed} of {} bootstrap replicates failed",
                bootstrap.plan.replicates
            ));
        }
    }
    let estimation = serde_json::to_value(estimation)?;
    let mut base_versions = vec![
        PLS_METHOD_VERSION,
        PLS_MEDIATION_METHOD_VERSION,
        ASSESSMENT_METHOD_VERSION,
    ];
    if !recipe.model.interactions.is_empty() {
        base_versions.insert(2, PLS_TWO_STAGE_MODERATION_METHOD_VERSION);
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Predict {
        base_versions.insert(1, PLS_PREDICT_METHOD_VERSION);
        if recipe.metadata.contains_key("pls_pos_segments")
            || recipe
                .metadata
                .contains_key("segmentation.pls_pos_segments")
        {
            base_versions.insert(2, PLS_SEGMENTATION_METHOD_VERSION);
        }
        if recipe.metadata.contains_key("segment_count") {
            base_versions.insert(2, PLS_POS_METHOD_VERSION);
        }
        if metadata_list_contains(recipe, "group_methods", "fimix")
            || recipe.metadata.contains_key("fimix_classes")
        {
            base_versions.insert(2, FIMIX_PLS_METHOD_VERSION);
        }
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Mga {
        base_versions.insert(1, PLS_MGA_METHOD_VERSION);
        if metadata_list_contains(recipe, "group_methods", "micom") {
            base_versions.insert(2, MICOM_METHOD_VERSION);
        }
        if metadata_list_contains(recipe, "group_methods", "mga_permutation") {
            base_versions.insert(2, PLS_MGA_PERMUTATION_METHOD_VERSION);
        }
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Ipma {
        base_versions.insert(1, IPMA_METHOD_VERSION);
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Cbsem {
        base_versions.insert(1, CBSEM_ML_METHOD_VERSION);
        base_versions.insert(1, CFA_ML_METHOD_VERSION);
        base_versions.insert(2, CBSEM_FIT_METHOD_VERSION);
        base_versions.insert(3, CBSEM_MODIFICATION_INDICES_METHOD_VERSION);
        if recipe.metadata.contains_key("cbsem_bootstrap_samples") {
            base_versions.insert(4, CBSEM_BOOTSTRAP_METHOD_VERSION);
        }
        if recipe.metadata.contains_key("cbsem_group_column") {
            base_versions.insert(4, CBSEM_MULTIGROUP_METHOD_VERSION);
        }
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Pca {
        base_versions = vec![PCA_METHOD_VERSION];
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Regression {
        base_versions = vec![match recipe
            .metadata
            .get("regression_type")
            .map(String::as_str)
            .unwrap_or("ols")
        {
            "logistic" => REGRESSION_LOGISTIC_METHOD_VERSION,
            "process" => REGRESSION_PROCESS_METHOD_VERSION,
            _ => REGRESSION_OLS_METHOD_VERSION,
        }];
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Nca {
        base_versions = vec![NCA_METHOD_VERSION];
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::Gsca {
        base_versions.insert(1, GSCA_METHOD_VERSION);
    }
    if permutation.is_some() {
        let mut versions = base_versions;
        if bootstrap.is_some() {
            versions.push(RESAMPLING_METHOD_VERSION);
        }
        versions.push(PERMUTATION_METHOD_VERSION);
        Ok(AnalysisResult::completed_pls_inference(
            recipe,
            versions.join("+"),
            started_at,
            estimation,
            assessment,
            bootstrap.map(serde_json::to_value).transpose()?,
            permutation.map(serde_json::to_value).transpose()?,
            warnings,
        ))
    } else if let Some(bootstrap) = bootstrap {
        let mut versions = base_versions;
        versions.push(RESAMPLING_METHOD_VERSION);
        Ok(AnalysisResult::completed_pls_bootstrap(
            recipe,
            versions.join("+"),
            started_at,
            estimation,
            assessment,
            serde_json::to_value(bootstrap)?,
            warnings,
        ))
    } else {
        Ok(AnalysisResult::completed_pls(
            recipe,
            base_versions.join("+"),
            started_at,
            estimation,
            assessment,
            warnings,
        ))
    }
}

fn metadata_list_contains(recipe: &AnalysisRecipe, key: &str, value: &str) -> bool {
    recipe
        .metadata
        .get(key)
        .map(|items| {
            items
                .split(',')
                .map(str::trim)
                .any(|item| item.eq_ignore_ascii_case(value))
        })
        .unwrap_or(false)
}

fn map_resampling_error(
    error: ResamplingError,
    wrap: impl FnOnce(String) -> RunnerError,
) -> RunnerError {
    match error {
        ResamplingError::Cancelled => RunnerError::Cancelled,
        other => wrap(other.to_string()),
    }
}

fn map_bootstrap_error(error: PlsBootstrapError) -> RunnerError {
    match error {
        PlsBootstrapError::Resampling(error) => map_resampling_error(error, RunnerError::Bootstrap),
        other => RunnerError::Bootstrap(other.to_string()),
    }
}

fn map_permutation_error(error: PlsPermutationError) -> RunnerError {
    match error {
        PlsPermutationError::Resampling(error) => {
            map_resampling_error(error, RunnerError::Permutation)
        }
        other => RunnerError::Permutation(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_data::{ImportOptions, import_delimited_bytes};

    #[test]
    fn deterministic_payload_is_stable_across_runner_invocations() {
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
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let left = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let right = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert_eq!(left.payload, right.payload);
        assert_eq!(left.diagnostics, right.diagnostics);
        assert_eq!(
            left.provenance.method_version,
            right.provenance.method_version
        );
    }
}
