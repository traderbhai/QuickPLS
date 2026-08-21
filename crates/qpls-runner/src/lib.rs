mod pls_model_comparison_execution;
mod recipe_v4_cbsem_execution;
mod recipe_v4_cbsem_product_indicator_execution;
mod recipe_v4_general_sem_pls_execution;
mod recipe_v4_pls_execution;

pub use pls_model_comparison_execution::*;
pub use recipe_v4_cbsem_execution::*;
pub use recipe_v4_cbsem_product_indicator_execution::*;
pub use recipe_v4_general_sem_pls_execution::*;
pub use recipe_v4_pls_execution::*;

use chrono::Utc;
#[cfg(test)]
use qpls_assessment::PLS_MODEL_FIT_METHOD_VERSION;
use qpls_assessment::{
    ASSESSMENT_METHOD_VERSION, AssessmentError, CCA_RESIDUAL_DIAGNOSTICS_METHOD_VERSION,
    assess_pls_validated_with_control,
};
#[cfg(test)]
use qpls_core::ANALYSIS_RECIPE_SCHEMA_VERSION;
use qpls_core::{AnalysisRecipe, AnalysisResult, PlsBootstrapTestTail, ValidatedExecutionRecipe};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_BOOTSTRAP_METHOD_VERSION_V2, CBSEM_FIT_METHOD_VERSION,
    CBSEM_MODIFICATION_INDICES_METHOD_VERSION, CBSEM_MULTIGROUP_METHOD_VERSION, EstimationError,
    FIMIX_PLS_METHOD_VERSION, GSCA_METHOD_VERSION, IPMA_METHOD_VERSION, MICOM_METHOD_VERSION,
    MICOM_METHOD_VERSION_V4, NCA_METHOD_VERSION, PCA_METHOD_VERSION, PLS_MEDIATION_METHOD_VERSION,
    PLS_METHOD_VERSION, PLS_MGA_METHOD_VERSION, PLS_MGA_PERMUTATION_METHOD_VERSION,
    PLS_POS_METHOD_VERSION, PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION,
    PLS_PREDICT_METHOD_VERSION, PLS_SEGMENTATION_METHOD_VERSION,
    PLS_TWO_STAGE_MODERATION_METHOD_VERSION, PlsPathSignificance,
    REGRESSION_LOGISTIC_METHOD_VERSION, REGRESSION_OLS_METHOD_VERSION,
    REGRESSION_PROCESS_METHOD_VERSION, REGRESSION_PROCESS_METHOD_VERSION_V1,
    estimate_pls_validated_with_control, pls_posthoc_minimum_sample_size_v2,
};
use qpls_resampling::{
    HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION, PERMUTATION_METHOD_VERSION,
    PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION, PLS_MODEL_FIT_EXACT_METHOD_VERSION,
    PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID, PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
    PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION_V2, PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION,
    PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION, PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION,
    PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION, PROCESS_BOOTSTRAP_METHOD_VERSION,
    PlsBootstrapError, PlsBootstrapTestTailInference, PlsPermutationError, PlsPowerDistributionV1,
    PlsPowerEstimatorSettingsV1, PlsPowerInferenceV1, PlsPowerMissingDataV1,
    PlsResamplingParameterFamily, PlsResamplingParameterIdentity, PlsSampleSizePowerError,
    PlsSampleSizePowerRecipeV1, PlscConsistentBootstrapError, PlscConsistentPermutationError,
    REGRESSION_BOOTSTRAP_METHOD_VERSION, REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION,
    RESAMPLING_METHOD_VERSION, ReflectiveGaussianPathDesignV1, RegressionBootstrapError,
    ResamplingError, bootstrap_cbsem_ml_validated, bootstrap_pls_validated_with_test_tail,
    bootstrap_plsc_consistent_validated, bootstrap_process_validated,
    bootstrap_regression_validated, permutation_pls_validated,
    permutation_plsc_consistent_validated, run_pls_sample_size_power_v2,
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
    #[error("analysis recipe is not executable: {0}")]
    Recipe(String),
    #[error("PLS estimation failed: {0}")]
    Estimation(String),
    #[error("PLS assessment failed: {0}")]
    Assessment(String),
    #[error("PLS bootstrap failed: {0}")]
    Bootstrap(String),
    #[error("PLS permutation failed: {0}")]
    Permutation(String),
    #[error("PLS sample-size/power analysis failed: {0}")]
    Power(String),
    #[error("result serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

fn pls_bootstrap_payload_value(
    bootstrap: &qpls_resampling::PlsBootstrapResult,
    test_tail_inference: &PlsBootstrapTestTailInference,
    selected_test_tail: PlsBootstrapTestTail,
) -> Result<serde_json::Value, RunnerError> {
    if test_tail_inference.method_version != PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION
        || test_tail_inference.selected_test_tail != selected_test_tail
    {
        return Err(RunnerError::Bootstrap(
            "PLS bootstrap test-tail inference is not bound to the selected typed contract".into(),
        ));
    }
    let mut payload = serde_json::to_value(bootstrap)?;
    if selected_test_tail != PlsBootstrapTestTail::TwoSided {
        payload
            .as_object_mut()
            .expect("PLS bootstrap result serializes as an object")
            .insert(
                "test_tail_inference".into(),
                serde_json::to_value(test_tail_inference)?,
            );
    }
    Ok(payload)
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
    if matches!(
        recipe.method_config.as_ref(),
        Some(qpls_core::MethodConfig::PlsAlgorithmConfiguredV2(_))
    ) {
        return Err(RunnerError::Recipe(
            "pls_algorithm_configured_v2 is Internal-only and requires the compiled Recipe-v4 PLS adapter"
                .into(),
        ));
    }
    if recipe.settings.method == qpls_core::AnalysisMethod::PlsSampleSizePower {
        return run_pls_sample_size_power_analysis(dataset, recipe, should_cancel, progress);
    }
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .map_err(|error| RunnerError::Recipe(error.to_string()))?;
    let base_execution = execution
        .without_outer_resampling()
        .map_err(|error| RunnerError::Recipe(error.to_string()))?;
    let effective_recipe = execution.effective();
    if effective_recipe.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided
        && !(effective_recipe.settings.method == qpls_core::AnalysisMethod::PlsPm
            && effective_recipe.settings.bootstrap_samples > 0)
    {
        return Err(RunnerError::Recipe(
            "a non-default bootstrap_test_tail requires a general PLS bootstrap run".into(),
        ));
    }
    let started_at = Utc::now();

    let mut estimation = estimate_pls_validated_with_control(dataset, &base_execution, |update| {
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

    let process_bootstrap_requested = matches!(
        effective_recipe.method_config.as_ref(),
        Some(qpls_core::MethodConfig::Regression {
            model: qpls_core::RegressionModelConfig::Process {
                relationship: qpls_core::ProcessRelationshipConfig::Graph { .. }
            },
            bootstrap: Some(_),
            ..
        })
    );
    let regression_bootstrap_performed = if effective_recipe.settings.method
        == qpls_core::AnalysisMethod::Regression
        && effective_recipe.settings.bootstrap_samples > 0
        && !process_bootstrap_requested
    {
        let bootstrap = bootstrap_regression_validated(
            dataset,
            &execution,
            &estimation,
            effective_recipe.settings.workers,
            &should_cancel,
            |update| {
                progress(RunnerProgress {
                    phase: update.phase.as_str().into(),
                    completed_units: update.completed_replicates as u64,
                    total_units: update.total_replicates as u64,
                });
            },
        )
        .map_err(map_regression_bootstrap_error)?;
        let regression = estimation.regression.as_mut().ok_or_else(|| {
            RunnerError::Bootstrap(
                "regression bootstrap completed without a base regression payload".into(),
            )
        })?;
        regression.bootstrap = Some(bootstrap);
        true
    } else {
        false
    };
    let process_bootstrap_performed = if effective_recipe.settings.method
        == qpls_core::AnalysisMethod::Regression
        && effective_recipe.settings.bootstrap_samples > 0
        && process_bootstrap_requested
    {
        let bootstrap = bootstrap_process_validated(
            dataset,
            &execution,
            &estimation,
            effective_recipe.settings.workers,
            &should_cancel,
            |update| {
                progress(RunnerProgress {
                    phase: update.phase.as_str().into(),
                    completed_units: update.completed_replicates as u64,
                    total_units: update.total_replicates as u64,
                });
            },
        )
        .map_err(map_regression_bootstrap_error)?;
        let graph = estimation
            .regression
            .as_mut()
            .and_then(|regression| regression.process.as_mut())
            .and_then(|process| process.graph_v2.as_mut())
            .ok_or_else(|| {
                RunnerError::Bootstrap(
                    "PROCESS bootstrap completed without a PROCESS v2 graph payload".into(),
                )
            })?;
        graph.bootstrap = Some(bootstrap);
        true
    } else {
        false
    };
    let cbsem_bootstrap_requested = matches!(
        effective_recipe.method_config.as_ref(),
        Some(qpls_core::MethodConfig::Cbsem {
            bootstrap_v2: Some(_),
            ..
        })
    );
    let cbsem_bootstrap_performed = if cbsem_bootstrap_requested {
        let bootstrap = bootstrap_cbsem_ml_validated(
            dataset,
            &execution,
            &estimation,
            effective_recipe.settings.workers,
            &should_cancel,
            |update| {
                progress(RunnerProgress {
                    phase: update.phase.as_str().into(),
                    completed_units: update.completed_replicates as u64,
                    total_units: update.total_replicates as u64,
                });
            },
        )
        .map_err(|error| match error {
            qpls_resampling::CbsemBootstrapError::Resampling(ResamplingError::Cancelled) => {
                RunnerError::Cancelled
            }
            other => RunnerError::Bootstrap(other.to_string()),
        })?;
        let cbsem = estimation.cbsem.as_mut().ok_or_else(|| {
            RunnerError::Bootstrap(
                "CB-SEM bootstrap completed without a base CB-SEM payload".into(),
            )
        })?;
        cbsem.bootstrap_v2 = Some(bootstrap);
        true
    } else {
        false
    };

    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let standalone_v08 = matches!(
        effective_recipe.settings.method,
        qpls_core::AnalysisMethod::Pca
            | qpls_core::AnalysisMethod::Regression
            | qpls_core::AnalysisMethod::Nca
    );
    let gsca_als = effective_recipe.settings.method == qpls_core::AnalysisMethod::Gsca;
    let assessment = if standalone_v08 || gsca_als {
        let warning = if gsca_als {
            "PLS assessment is not applicable to GSCA ALS component-model estimation."
        } else {
            "PLS assessment is not applicable to standalone raw-data analyses."
        };
        serde_json::json!({
            "method_version": "assessment_not_applicable_v1",
            "warnings": [warning]
        })
    } else {
        serde_json::to_value(
            assess_pls_validated_with_control(dataset, &base_execution, &estimation, |update| {
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
    let (bootstrap, bootstrap_test_tail_inference) = if effective_recipe.settings.bootstrap_samples
        > 0
        && !standalone_v08
        && effective_recipe.settings.method == qpls_core::AnalysisMethod::PlsPm
    {
        let execution_result = bootstrap_pls_validated_with_test_tail(
            dataset,
            &execution,
            &estimation,
            effective_recipe.settings.workers,
            &should_cancel,
            |update| {
                progress(RunnerProgress {
                    phase: update.phase.as_str().into(),
                    completed_units: update.completed_replicates as u64,
                    total_units: update.total_replicates as u64,
                });
            },
        )
        .map_err(map_bootstrap_error)?;
        (
            Some(execution_result.result),
            Some(execution_result.test_tail_inference),
        )
    } else {
        (None, None)
    };

    let consistent_bootstrap = if effective_recipe.settings.bootstrap_samples > 0
        && effective_recipe.settings.method == qpls_core::AnalysisMethod::Plsc
    {
        Some(
            bootstrap_plsc_consistent_validated(
                dataset,
                &execution,
                &estimation,
                effective_recipe.settings.workers,
                &should_cancel,
                |update| {
                    progress(RunnerProgress {
                        phase: update.phase.as_str().into(),
                        completed_units: update.completed_replicates as u64,
                        total_units: update.total_replicates as u64,
                    });
                },
            )
            .map_err(map_plsc_consistent_bootstrap_error)?,
        )
    } else {
        None
    };

    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let permutation = if effective_recipe.settings.permutation_samples > 0
        && !standalone_v08
        && effective_recipe.settings.method == qpls_core::AnalysisMethod::PlsPm
    {
        Some(
            permutation_pls_validated(
                dataset,
                &execution,
                &estimation,
                effective_recipe.settings.workers,
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

    let consistent_permutation = if effective_recipe.settings.permutation_samples > 0
        && effective_recipe.settings.method == qpls_core::AnalysisMethod::Plsc
    {
        Some(
            permutation_plsc_consistent_validated(
                dataset,
                &execution,
                &estimation,
                effective_recipe.settings.workers,
                &should_cancel,
                |update| {
                    progress(RunnerProgress {
                        phase: update.phase.as_str().into(),
                        completed_units: update.completed_replicates as u64,
                        total_units: update.total_replicates as u64,
                    });
                },
            )
            .map_err(map_plsc_consistent_permutation_error)?,
        )
    } else {
        None
    };

    let posthoc_technical_minimum_sample_size_requested = matches!(
        effective_recipe.method_config.as_ref(),
        Some(qpls_core::MethodConfig::PlsPosthocTechnicalMinimumSampleSize(_))
    );
    if posthoc_technical_minimum_sample_size_requested {
        let path_significance = bootstrap.as_ref().map(pls_bootstrap_path_significance);
        estimation.posthoc_minimum_sample_size = Some(pls_posthoc_minimum_sample_size_v2(
            &estimation.paths,
            estimation.used_observations,
            path_significance.as_deref(),
        ));
    }

    let mut warnings = estimation.warnings.clone();
    if standalone_v08 {
        warnings.push("PLS assessment and PLS resampling engines are not applicable to standalone raw-data analyses.".into());
    } else if gsca_als {
        warnings.push(
            "PLS assessment and PLS resampling engines are not applicable to GSCA ALS component-model estimation."
                .into(),
        );
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
    if let Some(bootstrap) = &consistent_bootstrap {
        let failed = bootstrap.failed_replicates.len();
        if failed > 0 {
            warnings.push(format!(
                "{failed} of {} full-PLSc bootstrap refits failed and were retained without retry or replacement",
                bootstrap.plan.replicates
            ));
        }
        warnings.extend(bootstrap.warnings.iter().cloned());
    }
    if let Some(permutation) = &consistent_permutation {
        let failed = permutation.failed_permutations.len();
        if failed > 0 {
            warnings.push(format!(
                "{failed} of {} fixed group-label assignments had a failed full-PLSc refit and were retained without retry or replacement",
                permutation.plan.permutations
            ));
        }
        warnings.extend(permutation.warnings.iter().cloned());
    }
    let estimation_method_version = estimation.method_version.clone();
    let estimation = serde_json::to_value(estimation)?;
    let mut base_versions = vec![
        PLS_METHOD_VERSION,
        PLS_MEDIATION_METHOD_VERSION,
        ASSESSMENT_METHOD_VERSION,
    ];
    if posthoc_technical_minimum_sample_size_requested {
        base_versions.push(PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION);
    }
    if matches!(
        effective_recipe.settings.method,
        qpls_core::AnalysisMethod::Plsc
            | qpls_core::AnalysisMethod::Wpls
            | qpls_core::AnalysisMethod::Cca
            | qpls_core::AnalysisMethod::CtaPls
            | qpls_core::AnalysisMethod::Endogeneity
    ) {
        base_versions.insert(1, estimation_method_version.as_str());
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Cca {
        base_versions.insert(2, CCA_RESIDUAL_DIAGNOSTICS_METHOD_VERSION);
    }
    if !effective_recipe.model.interactions.is_empty() {
        base_versions.insert(2, PLS_TWO_STAGE_MODERATION_METHOD_VERSION);
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Predict {
        base_versions.insert(1, PLS_PREDICT_METHOD_VERSION);
        if effective_recipe.metadata.contains_key("pls_pos_segments")
            || effective_recipe
                .metadata
                .contains_key("segmentation.pls_pos_segments")
        {
            base_versions.insert(2, PLS_SEGMENTATION_METHOD_VERSION);
        }
        if effective_recipe.metadata.contains_key("segment_count") {
            base_versions.insert(2, PLS_POS_METHOD_VERSION);
        }
        if metadata_list_contains(&effective_recipe, "group_methods", "fimix")
            || effective_recipe.metadata.contains_key("fimix_classes")
        {
            base_versions.insert(2, FIMIX_PLS_METHOD_VERSION);
        }
    }
    for (offset, method_version) in mga_method_versions(&effective_recipe)
        .into_iter()
        .enumerate()
    {
        base_versions.insert(1 + offset, method_version);
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Ipma {
        base_versions.insert(1, IPMA_METHOD_VERSION);
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Cbsem {
        base_versions.insert(1, estimation_method_version.as_str());
        base_versions.insert(2, CBSEM_FIT_METHOD_VERSION);
        base_versions.insert(3, CBSEM_MODIFICATION_INDICES_METHOD_VERSION);
        if cbsem_bootstrap_performed {
            base_versions.insert(4, CBSEM_BOOTSTRAP_METHOD_VERSION_V2);
        }
        if effective_recipe.metadata.contains_key("cbsem_group_column") {
            base_versions.insert(4, CBSEM_MULTIGROUP_METHOD_VERSION);
        }
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Pca {
        base_versions = vec![PCA_METHOD_VERSION];
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Regression {
        base_versions = vec![match effective_recipe
            .metadata
            .get("regression_type")
            .map(String::as_str)
            .unwrap_or("ols")
        {
            "logistic" => REGRESSION_LOGISTIC_METHOD_VERSION,
            "process" => {
                if matches!(
                    effective_recipe.method_config.as_ref(),
                    Some(qpls_core::MethodConfig::Regression {
                        model: qpls_core::RegressionModelConfig::Process {
                            relationship: qpls_core::ProcessRelationshipConfig::Graph { .. }
                        },
                        ..
                    })
                ) {
                    REGRESSION_PROCESS_METHOD_VERSION
                } else {
                    REGRESSION_PROCESS_METHOD_VERSION_V1
                }
            }
            _ => REGRESSION_OLS_METHOD_VERSION,
        }];
        if regression_bootstrap_performed {
            base_versions.push(REGRESSION_BOOTSTRAP_METHOD_VERSION);
        }
        if process_bootstrap_performed {
            base_versions.push(PROCESS_BOOTSTRAP_METHOD_VERSION);
        }
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Nca {
        base_versions = vec![NCA_METHOD_VERSION];
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Gsca {
        base_versions = vec![GSCA_METHOD_VERSION];
    }
    if permutation.is_some() {
        let mut versions = base_versions;
        if let Some(bootstrap) = &bootstrap {
            versions.push(RESAMPLING_METHOD_VERSION);
            if effective_recipe.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided {
                versions.push(PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION);
            }
            versions.push(
                bootstrap
                    .htmt_inference
                    .as_ref()
                    .map(|inference| inference.method_version.as_str())
                    .unwrap_or(HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION),
            );
            if bootstrap.model_fit_exact_inference.is_some() {
                versions.push(PLS_MODEL_FIT_EXACT_METHOD_VERSION);
            }
        }
        versions.push(PERMUTATION_METHOD_VERSION);
        Ok(AnalysisResult::completed_pls_inference(
            recipe,
            versions.join("+"),
            started_at,
            estimation,
            assessment,
            bootstrap
                .as_ref()
                .map(|bootstrap| {
                    pls_bootstrap_payload_value(
                        bootstrap,
                        bootstrap_test_tail_inference
                            .as_ref()
                            .expect("general PLS bootstrap carries test-tail inference"),
                        effective_recipe.settings.bootstrap_test_tail,
                    )
                })
                .transpose()?,
            permutation.map(serde_json::to_value).transpose()?,
            warnings,
        ))
    } else if let Some(permutation) = consistent_permutation {
        let mut versions = base_versions;
        versions.push(PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION);
        versions.push(PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION);
        if permutation.selected_tail_inference.is_some() {
            versions.push(PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION);
        }
        Ok(AnalysisResult::completed_pls_inference(
            recipe,
            versions.join("+"),
            started_at,
            estimation,
            assessment,
            None,
            Some(serde_json::to_value(permutation)?),
            warnings,
        ))
    } else if let Some(bootstrap) = bootstrap {
        let mut versions = base_versions;
        versions.push(RESAMPLING_METHOD_VERSION);
        if effective_recipe.settings.bootstrap_test_tail != PlsBootstrapTestTail::TwoSided {
            versions.push(PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION);
        }
        versions.push(
            bootstrap
                .htmt_inference
                .as_ref()
                .map(|inference| inference.method_version.as_str())
                .unwrap_or(HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION),
        );
        if bootstrap.model_fit_exact_inference.is_some() {
            versions.push(PLS_MODEL_FIT_EXACT_METHOD_VERSION);
        }
        Ok(AnalysisResult::completed_pls_bootstrap(
            recipe,
            versions.join("+"),
            started_at,
            estimation,
            assessment,
            pls_bootstrap_payload_value(
                &bootstrap,
                bootstrap_test_tail_inference
                    .as_ref()
                    .expect("general PLS bootstrap carries test-tail inference"),
                effective_recipe.settings.bootstrap_test_tail,
            )?,
            warnings,
        ))
    } else if let Some(bootstrap) = consistent_bootstrap {
        let mut versions = base_versions;
        versions.push(PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION);
        versions.push(RESAMPLING_METHOD_VERSION);
        if bootstrap.model_fit_exact_inference.is_some() {
            versions.push(PLS_MODEL_FIT_EXACT_METHOD_VERSION);
        }
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

fn pls_bootstrap_path_significance(
    bootstrap: &qpls_resampling::PlsBootstrapResult,
) -> Vec<PlsPathSignificance> {
    bootstrap
        .percentile
        .parameters
        .iter()
        .filter_map(|parameter| {
            let identity = PlsResamplingParameterIdentity::decode(&parameter.parameter).ok()?;
            (identity.family() == PlsResamplingParameterFamily::Path).then(|| {
                let components = identity.components();
                PlsPathSignificance {
                    source: components[0].clone(),
                    target: components[1].clone(),
                    p_value_two_sided: parameter.p_value_two_sided,
                }
            })
        })
        .collect()
}

fn run_pls_sample_size_power_analysis(
    dataset: &Dataset,
    recipe: &AnalysisRecipe,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<AnalysisResult, RunnerError> {
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .map_err(|error| RunnerError::Recipe(error.to_string()))?;
    let source = execution.source();
    let Some(qpls_core::MethodConfig::PlsSampleSizePower(config)) = source.method_config.as_ref()
    else {
        return Err(RunnerError::Recipe(
            "PLS sample-size/power requires its explicit typed method_config".into(),
        ));
    };
    if config.inference
        != qpls_core::PlsPowerInference::CaseBootstrapNullCenteredTwoSidedPlusOne
    {
        return Err(RunnerError::Recipe(
            "historical PLS sample-size/power v1 normal-reference recipes are read-only; new execution requires the null-centered two-sided plus-one v2 inference identity".into(),
        ));
    }
    let started_at = Utc::now();
    let power_recipe = PlsSampleSizePowerRecipeV1 {
        schema_version: PLS_SAMPLE_SIZE_POWER_RECIPE_SCHEMA_VERSION_V2,
        capability_id: PLS_SAMPLE_SIZE_POWER_CAPABILITY_ID.into(),
        method_version: PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2.into(),
        scenario_identity: config.scenario_identity.clone(),
        design: ReflectiveGaussianPathDesignV1 {
            predictor_construct: config.predictor_construct.clone(),
            outcome_construct: config.outcome_construct.clone(),
            predictor_indicator_loadings: config.predictor_indicator_loadings.clone(),
            outcome_indicator_loadings: config.outcome_indicator_loadings.clone(),
            population_path: config.population_path,
            exogenous_distribution: PlsPowerDistributionV1::StandardNormal,
            structural_disturbance_distribution: PlsPowerDistributionV1::StandardNormal,
            indicator_error_distribution: PlsPowerDistributionV1::StandardNormal,
            missing_data: PlsPowerMissingDataV1::None,
        },
        estimator: PlsPowerEstimatorSettingsV1 {
            weighting_scheme: source.settings.weighting_scheme.clone(),
            preprocessing: source.settings.preprocessing.clone(),
            tolerance: source.settings.tolerance,
            max_iterations: source.settings.max_iterations,
        },
        inference: PlsPowerInferenceV1::CaseBootstrapNullCenteredTwoSidedPlusOne,
        sample_size_grid: config.sample_size_grid.clone(),
        alpha: config.alpha,
        target_power: config.target_power,
        confidence_level: config.interval_confidence_level,
        monte_carlo_replicates: config.monte_carlo_replicates,
        bootstrap_replicates: config.bootstrap_replicates,
        master_seed: source.settings.seed,
        workers: source.settings.workers,
    };
    let analysis = run_pls_sample_size_power_v2(&power_recipe, &should_cancel, |update| {
        progress(RunnerProgress {
            phase: update.phase.as_str().into(),
            completed_units: update.completed_replicates,
            total_units: update.total_replicates,
        });
    })
    .map_err(|error| match error {
        PlsSampleSizePowerError::Cancelled => RunnerError::Cancelled,
        other => RunnerError::Power(other.to_string()),
    })?;
    let warnings = analysis
        .warnings
        .iter()
        .chain(analysis.exclusions.iter())
        .cloned()
        .collect::<Vec<_>>();
    Ok(AnalysisResult::completed_pls_sample_size_power_v2(
        source,
        PLS_SAMPLE_SIZE_POWER_METHOD_VERSION_V2,
        started_at,
        serde_json::to_value(analysis)?,
        warnings,
    ))
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

fn mga_method_versions(recipe: &AnalysisRecipe) -> Vec<&'static str> {
    if recipe.settings.method != qpls_core::AnalysisMethod::Mga {
        return Vec::new();
    }
    if matches!(
        recipe.method_config.as_ref(),
        Some(qpls_core::MethodConfig::Micom { .. })
    ) {
        return vec![MICOM_METHOD_VERSION];
    }

    let mut versions = vec![PLS_MGA_METHOD_VERSION];
    let configured_methods = match recipe.method_config.as_ref() {
        Some(qpls_core::MethodConfig::Mga { methods, .. }) => Some(methods.as_slice()),
        _ => None,
    };
    let mga_permutation_requested = configured_methods
        .is_some_and(|methods| methods.contains(&qpls_core::GroupAnalysisMethod::MgaPermutation))
        || configured_methods.is_none()
            && metadata_list_contains(recipe, "group_methods", "mga_permutation");
    let micom_requested = configured_methods
        .is_some_and(|methods| methods.contains(&qpls_core::GroupAnalysisMethod::Micom))
        || configured_methods.is_none() && metadata_list_contains(recipe, "group_methods", "micom");
    if mga_permutation_requested {
        versions.push(PLS_MGA_PERMUTATION_METHOD_VERSION);
    }
    if micom_requested {
        versions.push(MICOM_METHOD_VERSION_V4);
    }
    versions
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
        PlsBootstrapError::ExactFit(qpls_resampling::PlsModelFitExactError::Resampling(error)) => {
            map_resampling_error(error, RunnerError::Bootstrap)
        }
        other => RunnerError::Bootstrap(other.to_string()),
    }
}

fn map_plsc_consistent_bootstrap_error(error: PlscConsistentBootstrapError) -> RunnerError {
    match error {
        PlscConsistentBootstrapError::Resampling(error) => {
            map_resampling_error(error, RunnerError::Bootstrap)
        }
        PlscConsistentBootstrapError::ExactFit(
            qpls_resampling::PlsModelFitExactError::Resampling(error),
        ) => map_resampling_error(error, RunnerError::Bootstrap),
        other => RunnerError::Bootstrap(other.to_string()),
    }
}

fn map_plsc_consistent_permutation_error(error: PlscConsistentPermutationError) -> RunnerError {
    match error {
        PlscConsistentPermutationError::Resampling(error) => {
            map_resampling_error(error, RunnerError::Permutation)
        }
        other => RunnerError::Permutation(other.to_string()),
    }
}

fn map_regression_bootstrap_error(error: RegressionBootstrapError) -> RunnerError {
    match error {
        RegressionBootstrapError::Resampling(ResamplingError::Cancelled) => RunnerError::Cancelled,
        RegressionBootstrapError::InsufficientUsableReplicates { usable, required } => {
            RunnerError::Bootstrap(format!(
                "regression bootstrap retained {usable} usable replicates, below the {required} required by the {:.0}% policy",
                REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION * 100.0
            ))
        }
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
    use chrono::Utc;
    use qpls_core::{
        AnalysisMethod, AnalysisPayload, MethodConfig, Preprocessing, ProcessContinuousCentering,
        ProcessModerationConfig, ProcessModeratorConfig, ProcessModeratorScale, ProcessPathConfig,
        ProcessRelationshipConfig, RegressionBootstrapAlgorithm, RegressionBootstrapConfig,
        RegressionBootstrapInterval, RegressionModelConfig, RobustStandardError,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    fn run_fixture(data: &[u8], data_name: &str, recipe: &[u8]) -> AnalysisResult {
        let dataset =
            import_delimited_bytes(data, data_name, b',', &ImportOptions::default()).unwrap();
        let mut recipe = migrated_execution_recipe(recipe);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap()
    }

    fn migrated_execution_recipe(bytes: &[u8]) -> AnalysisRecipe {
        let recipe: AnalysisRecipe = serde_json::from_slice(bytes).unwrap();
        if recipe.schema_version == ANALYSIS_RECIPE_SCHEMA_VERSION {
            recipe
        } else {
            recipe.migrated_v3().unwrap()
        }
    }

    #[test]
    fn bootstrap_path_significance_accepts_only_canonical_typed_path_identities() {
        let parameter =
            |parameter: String, p_value_two_sided| qpls_resampling::BootstrapParameterInference {
                parameter,
                original: 0.5,
                bootstrap_mean: 0.5,
                bias: 0.0,
                standard_error: 0.1,
                lower: 0.3,
                upper: 0.7,
                usable_replicates: 99,
                t_statistic: Some(5.0),
                p_value_two_sided,
            };
        let canonical_path = PlsResamplingParameterIdentity::new(
            PlsResamplingParameterFamily::Path,
            ["source", "target"],
        )
        .unwrap()
        .encode();
        let non_path = PlsResamplingParameterIdentity::new(
            PlsResamplingParameterFamily::OuterLoading,
            ["construct", "indicator"],
        )
        .unwrap()
        .encode();
        let bootstrap = qpls_resampling::PlsBootstrapResult {
            method_version: RESAMPLING_METHOD_VERSION.into(),
            plan: qpls_resampling::BootstrapPlan {
                replicates: 99,
                master_seed: 1,
                operation: "runner_identity_test".into(),
            },
            usable_replicates: 99,
            failed_replicates: Vec::new(),
            percentile: qpls_resampling::PercentileInference {
                confidence_level: 0.95,
                parameters: vec![
                    parameter(canonical_path, Some(0.01)),
                    parameter(non_path, Some(0.02)),
                    parameter(r#"["path",["wrong-arity"]]"#.into(), Some(0.03)),
                    parameter(r#"[ "path", ["not", "canonical"]]"#.into(), Some(0.04)),
                    parameter("not-json".into(), Some(0.05)),
                ],
            },
            bca: None,
            studentized: None,
            htmt_inference: None,
            model_fit_exact_inference: None,
        };

        assert_eq!(
            pls_bootstrap_path_significance(&bootstrap),
            vec![PlsPathSignificance {
                source: "source".into(),
                target: "target".into(),
                p_value_two_sided: Some(0.01),
            }]
        );
    }

    #[test]
    fn runner_persists_typed_htmt_interval_and_tail_selection() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 24;
        recipe.settings.workers = 2;
        recipe.settings.htmt_bootstrap_inference = qpls_core::HtmtBootstrapInferenceConfig {
            interval_family: qpls_core::HtmtBootstrapIntervalFamily::Percentile,
            test_tail: qpls_core::HtmtBootstrapTestTail::TwoSided,
        };
        recipe.method_config = Some(MethodConfig::PlsBootstrap);

        let result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert!(result.provenance.method_version.split('+').any(|version| {
            version == qpls_resampling::HTMT_CONFIGURABLE_BOOTSTRAP_INFERENCE_METHOD_VERSION
        }));
        let AnalysisPayload::PlsPmV2 { bootstrap, .. } = result.payload else {
            panic!("configured HTMT bootstrap must use the bootstrap envelope")
        };
        let bootstrap: qpls_resampling::PlsBootstrapResult =
            serde_json::from_value(bootstrap).unwrap();
        let htmt = bootstrap.htmt_inference.unwrap();
        assert_eq!(
            htmt.method_version,
            qpls_resampling::HTMT_CONFIGURABLE_BOOTSTRAP_INFERENCE_METHOD_VERSION
        );
        for artifact in [&htmt.htmt_plus, &htmt.htmt_original] {
            assert_eq!(
                artifact.interval_method,
                qpls_resampling::HTMT_BOOTSTRAP_PERCENTILE_INTERVAL_METHOD
            );
            assert_eq!(
                artifact.test_type,
                qpls_resampling::HTMT_BOOTSTRAP_TWO_SIDED_TEST_TYPE
            );
            assert_eq!(artifact.equivalent_two_sided_confidence_level, 0.95);
            for cell in artifact.cells.iter().flatten().filter(|cell| {
                cell.status == qpls_resampling::HtmtBootstrapInferenceStatus::Available
            }) {
                assert_eq!(cell.bias_correction, None);
                assert_eq!(
                    cell.usable_replicates + cell.failed_replicates,
                    artifact.requested_replicates
                );
            }
        }
    }

    #[test]
    fn runner_adds_only_explicit_one_sided_pls_bootstrap_tail_receipt() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 24;
        recipe.settings.workers = 1;
        recipe.method_config = Some(MethodConfig::PlsBootstrap);

        let default = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert!(
            !default
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION)
        );
        let AnalysisPayload::PlsPmV2 {
            bootstrap: default_bootstrap,
            ..
        } = default.payload
        else {
            panic!("default PLS bootstrap must use the bootstrap envelope")
        };
        assert!(default_bootstrap.get("test_tail_inference").is_none());
        let usable_replicates = default_bootstrap["usable_replicates"].as_u64().unwrap() as u32;

        recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::OneSidedGreater;
        let configured = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert!(
            configured
                .provenance
                .method_version
                .split('+')
                .any(|version| { version == PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION })
        );
        let AnalysisPayload::PlsPmV2 {
            bootstrap: mut configured_bootstrap,
            ..
        } = configured.payload
        else {
            panic!("one-sided PLS bootstrap must use the bootstrap envelope")
        };
        let receipt: qpls_resampling::PlsBootstrapTestTailInference = serde_json::from_value(
            configured_bootstrap
                .get("test_tail_inference")
                .cloned()
                .expect("explicit one-sided selection persists a typed receipt"),
        )
        .unwrap();
        assert_eq!(
            receipt.method_version,
            PLS_BOOTSTRAP_TEST_TAIL_METHOD_VERSION
        );
        assert_eq!(
            receipt.selected_test_tail,
            PlsBootstrapTestTail::OneSidedGreater
        );
        for parameter in &receipt.parameters {
            assert_eq!(parameter.usable_replicates, usable_replicates);
            for (count, p_value) in [
                (parameter.two_sided_exceedances, parameter.p_value_two_sided),
                (
                    parameter.greater_or_equal_exceedances,
                    parameter.p_value_greater,
                ),
                (parameter.less_or_equal_exceedances, parameter.p_value_less),
            ] {
                let expected =
                    (f64::from(count) + 1.0) / (f64::from(parameter.usable_replicates) + 1.0);
                assert_eq!(p_value.to_bits(), expected.to_bits());
            }
        }

        configured_bootstrap
            .as_object_mut()
            .unwrap()
            .remove("test_tail_inference");
        assert_eq!(configured_bootstrap, default_bootstrap);

        let bootstrap: qpls_resampling::PlsBootstrapResult =
            serde_json::from_value(configured_bootstrap).unwrap();
        let mut tampered_receipt = receipt;
        tampered_receipt.selected_test_tail = PlsBootstrapTestTail::OneSidedLess;
        assert!(matches!(
            pls_bootstrap_payload_value(
                &bootstrap,
                &tampered_receipt,
                PlsBootstrapTestTail::OneSidedGreater,
            ),
            Err(RunnerError::Bootstrap(message))
                if message.contains("not bound to the selected typed contract")
        ));
    }

    fn plsc_consistent_permutation_fixture(workers: usize) -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            include_bytes!(
                "../../../validation/fixtures/plsc_consistent_permutation_two_group.csv"
            ),
            "plsc_consistent_permutation_two_group.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/micom_v2_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.method = AnalysisMethod::Plsc;
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.studentized_inner_samples = 0;
        recipe.settings.permutation_samples = 99;
        recipe.settings.workers = workers;
        recipe.settings.confidence_level = 0.95;
        recipe.settings.case_weight_column = None;
        recipe.method_config = Some(MethodConfig::PlscPermutation {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            test_tail: qpls_core::PlscPermutationTestTail::TwoSided,
        });
        recipe.metadata.clear();
        (dataset, recipe)
    }

    fn plsc_consistent_permutation_original_group_fit(group: &str) -> qpls_estimation::PlsResult {
        // Mutually orthogonal trigonometric bases make every three-indicator
        // block exactly compound-symmetric with correlation 0.75^2. All
        // structural coefficients are positive, so mixed-sign outer weights
        // indicate a real fixture/estimator regression rather than noise.
        let source =
            include_str!("../../../validation/fixtures/plsc_consistent_permutation_two_group.csv");
        let mut lines = source.lines();
        let header = lines.next().unwrap();
        let mut filtered = String::with_capacity(source.len() / 2);
        filtered.push_str(header);
        filtered.push('\n');
        for line in lines.filter(|line| line.split(',').next() == Some(group)) {
            filtered.push_str(line);
            filtered.push('\n');
        }
        let dataset = import_delimited_bytes(
            filtered.as_bytes(),
            &format!("plsc_consistent_permutation_group_{group}.csv"),
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        assert_eq!(dataset.batch.num_rows(), 160);
        let (_, mut recipe) = plsc_consistent_permutation_fixture(1);
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.permutation_samples = 0;
        recipe.method_config = Some(MethodConfig::Plsc);
        qpls_estimation::estimate_pls(&dataset, &recipe).unwrap()
    }

    fn logistic_fixture() -> (Dataset, AnalysisRecipe) {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/v08_extended_methods_fixture.csv"),
            "v08_extended_methods_fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/v08_regression_logistic.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.preprocessing = Preprocessing::Unstandardized;
        (dataset, recipe)
    }

    fn process_graph_fixture(with_bootstrap: bool) -> (Dataset, AnalysisRecipe) {
        let mut csv = String::from("X,M,W,Y\n");
        for index in 0..80 {
            let x = index as f64 / 5.0 - 8.0;
            let w = ((index * 7) % 17) as f64 / 4.0 - 2.0;
            let m = 0.55 * x + 0.2 * x * w + ((index * 3) % 11) as f64 / 50.0;
            let y = 0.3 * x + 0.75 * m + ((index * 5) % 13) as f64 / 60.0;
            csv.push_str(&format!("{x},{m},{w},{y}\n"));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "process-runner-v2.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut settings = qpls_core::AnalysisSettings::default();
        settings.method = AnalysisMethod::Regression;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.bootstrap_samples = if with_bootstrap { 99 } else { 0 };
        settings.workers = if with_bootstrap { 2 } else { 1 };
        let recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: serde_json::from_str("\"7e5a1a09-75a8-49ee-9b40-e70a50ab40f4\"").unwrap(),
            created_at: Utc::now(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: qpls_core::ModelSpec {
                id: serde_json::from_str("\"31b2648a-00b1-4fe2-bf2f-b292da00195a\"").unwrap(),
                name: "PROCESS runner v2".into(),
                constructs: Vec::new(),
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            settings,
            method_config: Some(MethodConfig::Regression {
                outcome: "Y".into(),
                predictors: vec!["X".into(), "M".into(), "W".into()],
                controls: Vec::new(),
                model: RegressionModelConfig::Process {
                    relationship: ProcessRelationshipConfig::Graph {
                        focal_predictor: "X".into(),
                        paths: vec![
                            ProcessPathConfig {
                                from: "X".into(),
                                to: "M".into(),
                            },
                            ProcessPathConfig {
                                from: "M".into(),
                                to: "Y".into(),
                            },
                            ProcessPathConfig {
                                from: "X".into(),
                                to: "Y".into(),
                            },
                        ],
                        moderators: vec![ProcessModeratorConfig {
                            variable: "W".into(),
                            scale: ProcessModeratorScale::Continuous,
                        }],
                        moderations: vec![ProcessModerationConfig {
                            from: "X".into(),
                            to: "M".into(),
                            moderator: "W".into(),
                            conditioning_moderator: None,
                        }],
                        continuous_product_centering:
                            ProcessContinuousCentering::EquationCompleteCaseMeanV1,
                    },
                },
                bootstrap: with_bootstrap.then_some(RegressionBootstrapConfig {
                    algorithm: RegressionBootstrapAlgorithm::CaseResampling,
                    intervals: vec![
                        RegressionBootstrapInterval::Percentile,
                        RegressionBootstrapInterval::Bca,
                    ],
                }),
            }),
            metadata: std::collections::BTreeMap::new(),
        };
        (dataset, recipe)
    }

    fn regression_bootstrap_recipe(
        mut recipe: AnalysisRecipe,
        model: RegressionModelConfig,
        workers: usize,
        replicates: u32,
    ) -> AnalysisRecipe {
        recipe.settings.bootstrap_samples = replicates;
        recipe.settings.seed = 91;
        recipe.settings.workers = workers;
        let Some(MethodConfig::Regression {
            model: recipe_model,
            bootstrap,
            ..
        }) = recipe.method_config.as_mut()
        else {
            panic!("fixture must use typed regression")
        };
        *recipe_model = model;
        *bootstrap = Some(RegressionBootstrapConfig {
            algorithm: RegressionBootstrapAlgorithm::CaseResampling,
            intervals: vec![
                RegressionBootstrapInterval::Percentile,
                RegressionBootstrapInterval::Bca,
            ],
        });
        recipe
    }

    fn assert_estimator_version_is_in_provenance(result: &AnalysisResult, expected: &str) {
        let estimation = match &result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. }
            | AnalysisPayload::PlsPmV2 { estimation, .. }
            | AnalysisPayload::PlsPmV3 { estimation, .. } => estimation,
            AnalysisPayload::PlsSampleSizePowerV1 { .. }
            | AnalysisPayload::PlsSampleSizePowerV2 { .. } => {
                panic!("expected an estimator payload, received sample-size power")
            }
            AnalysisPayload::Legacy { .. } => panic!("expected a typed PLS result payload"),
        };
        assert_eq!(estimation["method_version"].as_str(), Some(expected));
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == expected),
            "provenance '{}' does not contain estimator version '{expected}'",
            result.provenance.method_version
        );
    }

    #[test]
    fn deterministic_payload_is_stable_across_runner_invocations() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let left = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let right = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert_eq!(left.payload, right.payload);
        assert_eq!(left.diagnostics, right.diagnostics);
        assert_eq!(
            left.provenance.method_version,
            right.provenance.method_version
        );
        assert_eq!(
            left.provenance.method_version,
            format!(
                "{PLS_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            )
        );
        assert_estimator_version_is_in_provenance(&left, PLS_METHOD_VERSION);
        let AnalysisPayload::PlsPmV1 { estimation, .. } = &left.payload else {
            panic!("direct-only PLS must use the point-estimation envelope")
        };
        assert_eq!(
            estimation["mediation"]["method_version"],
            PLS_MEDIATION_METHOD_VERSION
        );
        assert_eq!(
            estimation["mediation"]["estimates"][0]["classification"],
            "direct_only"
        );
        assert!(estimation.get("moderation").is_none());
    }

    #[test]
    fn configured_pls_is_not_exposed_through_the_standard_schema3_runner() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.method_config = Some(MethodConfig::PlsAlgorithmConfiguredV2(
            qpls_core::PlsAlgorithmConfigV2::standard(),
        ));

        assert!(matches!(
            run_pls_analysis(&dataset, &recipe, || false, |_| {}),
            Err(RunnerError::Recipe(message))
                if message.contains("Internal-only") && message.contains("Recipe-v4")
        ));
    }

    #[test]
    fn point_pls_runner_persists_the_current_model_fit_identity_and_complete_point_family() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();

        let completed = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        let AnalysisPayload::PlsPmV1 { assessment, .. } = completed.payload else {
            panic!("point PLS must use the point-estimation envelope")
        };
        let assessment: qpls_assessment::AssessmentResult =
            serde_json::from_value(assessment).unwrap();
        let fit = assessment
            .model_fit
            .expect("current PLS assessment records model fit");
        assert_eq!(fit.method_version, PLS_MODEL_FIT_METHOD_VERSION);
        assert_eq!(fit.analytical_sample_size, dataset.batch.num_rows());
        for model in [&fit.saturated, &fit.estimated] {
            assert!(model.d_g.value().is_some());
            assert!(model.chi_square.value().is_some());
            assert!(model.nfi.value().is_some());
        }
        assert_eq!(fit.exact_fit_inference.status, "unavailable");
        assert_eq!(
            fit.exact_fit_inference.reason_code,
            "model_fit.adapted_bollen_stine_not_implemented"
        );
    }

    #[test]
    fn posthoc_sample_size_uses_bootstrap_significance_and_never_point_estimates_as_inference() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut point_recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        point_recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        let point = run_pls_analysis(&dataset, &point_recipe, || false, |_| {}).unwrap();
        let AnalysisPayload::PlsPmV1 { estimation, .. } = point.payload else {
            panic!("point PLS must use the point-estimation envelope")
        };
        let point: qpls_estimation::PlsResult = serde_json::from_value(estimation).unwrap();
        assert!(point.posthoc_minimum_sample_size.is_none());

        let mut opted_point_recipe = point_recipe.clone();
        opted_point_recipe.method_config = Some(
            qpls_core::MethodConfig::PlsPosthocTechnicalMinimumSampleSize(
                qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::point_estimate_v2(),
            ),
        );
        let opted_point =
            run_pls_analysis(&dataset, &opted_point_recipe, || false, |_| {}).unwrap();
        assert!(
            opted_point
                .provenance
                .method_version
                .split('+')
                .any(|version| {
                    version == qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_METHOD_VERSION
                })
        );
        let AnalysisPayload::PlsPmV1 { estimation, .. } = opted_point.payload else {
            panic!("point PLS must use the point-estimation envelope")
        };
        let opted_point: qpls_estimation::PlsResult = serde_json::from_value(estimation).unwrap();
        let diagnostic = opted_point.posthoc_minimum_sample_size.unwrap();
        assert_eq!(
            diagnostic.status,
            qpls_estimation::PlsPosthocMinimumSampleSizeStatus::InferenceUnavailable
        );
        assert_eq!(diagnostic.technically_required_sample_size, None);
        assert_eq!(diagnostic.driver_source, None);

        let mut bootstrap_recipe = point_recipe;
        bootstrap_recipe.settings.bootstrap_samples = 99;
        bootstrap_recipe.settings.workers = 1;
        bootstrap_recipe.method_config = Some(
            qpls_core::MethodConfig::PlsPosthocTechnicalMinimumSampleSize(
                qpls_core::PlsPosthocTechnicalMinimumSampleSizeConfigV2::bootstrap_v2(),
            ),
        );
        let bootstrapped = run_pls_analysis(&dataset, &bootstrap_recipe, || false, |_| {}).unwrap();
        let mut four_worker_recipe = bootstrap_recipe.clone();
        four_worker_recipe.settings.workers = 4;
        let four_worker =
            run_pls_analysis(&dataset, &four_worker_recipe, || false, |_| {}).unwrap();
        assert_eq!(bootstrapped.payload, four_worker.payload);
        assert!(
            bootstrapped
                .provenance
                .method_version
                .split('+')
                .any(|version| version == HTMT_BOOTSTRAP_INFERENCE_METHOD_VERSION)
        );
        let AnalysisPayload::PlsPmV2 {
            estimation,
            bootstrap,
            ..
        } = bootstrapped.payload
        else {
            panic!("bootstrapped PLS must use the bootstrap envelope")
        };
        let estimation: qpls_estimation::PlsResult = serde_json::from_value(estimation).unwrap();
        let bootstrap: qpls_resampling::PlsBootstrapResult =
            serde_json::from_value(bootstrap).unwrap();
        assert!(bootstrap.htmt_inference.is_some());
        let expected_significance = pls_bootstrap_path_significance(&bootstrap);
        let expected = pls_posthoc_minimum_sample_size_v2(
            &estimation.paths,
            estimation.used_observations,
            Some(&expected_significance),
        );
        assert_eq!(
            estimation.posthoc_minimum_sample_size.as_ref(),
            Some(&expected)
        );
        assert_eq!(
            expected.significance_source.as_deref(),
            Some(qpls_estimation::PLS_POSTHOC_MINIMUM_SAMPLE_SIZE_SIGNIFICANCE_SOURCE)
        );
        assert!(!matches!(
            expected.status,
            qpls_estimation::PlsPosthocMinimumSampleSizeStatus::InferenceUnavailable
                | qpls_estimation::PlsPosthocMinimumSampleSizeStatus::InferenceIncomplete
        ));

        let mut impossible = bootstrap_recipe;
        impossible.settings.bootstrap_samples = 0;
        let error = run_pls_analysis(&dataset, &impossible, || false, |_| {}).unwrap_err();
        assert!(
            matches!(error, RunnerError::Recipe(message) if message.contains("resampling_mismatch"))
        );
    }

    #[test]
    fn logistic_v2_is_deterministic_and_reports_exact_provenance_progress_and_diagnostics() {
        let (dataset, recipe) = logistic_fixture();
        let progress = Mutex::new(Vec::new());
        let left = run_pls_analysis(
            &dataset,
            &recipe,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();
        let right = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert_eq!(left.payload, right.payload);
        assert_eq!(
            left.provenance.method_version,
            REGRESSION_LOGISTIC_METHOD_VERSION
        );
        assert_eq!(left.provenance.method, AnalysisMethod::Regression);
        assert!(
            progress
                .lock()
                .unwrap()
                .iter()
                .any(|update| update.phase == "iterating")
        );
        let estimation = match &left.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("expected standalone logistic payload, got {other:?}"),
        };
        assert_eq!(
            estimation["regression"]["method_version"],
            REGRESSION_LOGISTIC_METHOD_VERSION
        );
        assert_eq!(
            estimation["regression"]["warnings"],
            serde_json::json!([qpls_estimation::REGRESSION_LOGISTIC_SCOPE_WARNING])
        );
        assert_eq!(
            estimation["regression"]["logistic"]["outcome_profile"]["readiness"],
            "ready"
        );
        assert_eq!(
            estimation["regression"]["logistic"]["convergence"]["algorithm"],
            "deterministic_newton_irls_v1"
        );
        assert_eq!(
            estimation["regression"]["fit"]["pseudo_r_squared_method"],
            "mcfadden_v1"
        );
    }

    #[test]
    fn logistic_v2_honors_cancellation_inside_irls() {
        let (dataset, recipe) = logistic_fixture();
        let iterating_updates = AtomicUsize::new(0);
        let cancelled = run_pls_analysis(
            &dataset,
            &recipe,
            || iterating_updates.load(Ordering::SeqCst) > 0,
            |update| {
                if update.phase == "iterating" {
                    iterating_updates.fetch_add(1, Ordering::SeqCst);
                }
            },
        );
        assert!(matches!(cancelled, Err(RunnerError::Cancelled)));
        assert_eq!(iterating_updates.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn logistic_nonconvergence_uses_method_specific_runner_message() {
        let message = RunnerError::Estimation(
            qpls_estimation::EstimationError::LogisticNonConvergence(100).to_string(),
        )
        .to_string();
        assert!(message.contains("logistic regression did not converge"));
        assert!(!message.contains("PLS weights"));
    }

    #[test]
    fn process_v1_current_job_is_rejected_before_estimation() {
        let (dataset, mut recipe) = logistic_fixture();
        recipe.settings.bootstrap_samples = 0;
        recipe.settings.workers = 1;
        recipe.method_config = Some(MethodConfig::Regression {
            outcome: "y".into(),
            predictors: vec!["x".into(), "m".into()],
            controls: Vec::new(),
            model: RegressionModelConfig::Process {
                relationship: ProcessRelationshipConfig::Mediation {
                    x: "x".into(),
                    mediator: "m".into(),
                },
            },
            bootstrap: None,
        });
        assert!(matches!(
            run_pls_analysis(&dataset, &recipe, || false, |_| {}),
            Err(RunnerError::Recipe(message))
                if message.contains("archive-readable only")
                    && message.contains("regression_process_v2")
        ));
    }

    #[test]
    fn process_v2_point_progress_completes_and_base_fit_cancellation_returns_no_result() {
        let (dataset, point_recipe) = process_graph_fixture(false);
        let progress = Mutex::new(Vec::new());
        let point = run_pls_analysis(
            &dataset,
            &point_recipe,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();
        assert_eq!(
            point.provenance.method_version,
            REGRESSION_PROCESS_METHOD_VERSION
        );
        let AnalysisPayload::PlsPmV1 { estimation, .. } = &point.payload else {
            panic!("point PROCESS v2 must use the point-estimation envelope")
        };
        assert!(estimation.get("mediation").is_none());
        assert!(estimation.get("moderation").is_none());
        let progress = progress.into_inner().unwrap();
        for phase in [
            "preparing_rows",
            "preparing_indicators",
            "iterating",
            "computing_effects",
            "assembling",
        ] {
            assert!(
                progress.iter().any(|update| {
                    update.phase == phase
                        && update.total_units > 0
                        && update.completed_units == update.total_units
                }),
                "missing completed PROCESS runner progress for {phase}: {progress:?}"
            );
        }

        let (_, bootstrap_recipe) = process_graph_fixture(true);
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let cancelled = run_pls_analysis(
            &dataset,
            &bootstrap_recipe,
            || cancel.load(Ordering::SeqCst),
            |update| {
                if update.phase == "preparing_rows" && update.completed_units > 0 {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
        );
        assert!(matches!(cancelled, Err(RunnerError::Cancelled)));
        assert!(cancel.load(Ordering::SeqCst));
    }

    #[test]
    fn process_v2_runner_rejects_exact_binary_endogenous_original_profiles() {
        for binary_variable in ["M", "Y"] {
            let mut csv = String::from("X,M,W,Y\n");
            for index in 0..80 {
                let x = index as f64 / 5.0 - 8.0;
                let w = ((index * 7) % 17) as f64 / 4.0 - 2.0;
                let binary = (index % 2) as f64;
                let m = if binary_variable == "M" {
                    binary
                } else {
                    0.55 * x + 0.2 * x * w + ((index * 3) % 11) as f64 / 50.0
                };
                let y = if binary_variable == "Y" {
                    binary
                } else {
                    0.3 * x + 0.75 * m + ((index * 5) % 13) as f64 / 60.0
                };
                csv.push_str(&format!("{x},{m},{w},{y}\n"));
            }
            let dataset = import_delimited_bytes(
                csv.as_bytes(),
                &format!("process-runner-binary-{binary_variable}.csv"),
                b',',
                &ImportOptions::default(),
            )
            .unwrap();
            let (_, mut recipe) = process_graph_fixture(false);
            recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
            assert!(matches!(
                run_pls_analysis(&dataset, &recipe, || false, |_| {}),
                Err(RunnerError::Estimation(message))
                    if message.contains("binary_process_equation_outcome")
                        && message.contains(binary_variable)
            ));
        }
    }

    #[test]
    fn regression_bootstrap_ols_and_logistic_are_nested_deterministic_and_worker_invariant() {
        let (dataset, point_recipe) = logistic_fixture();
        for (model, base_version) in [
            (
                RegressionModelConfig::Ols {
                    robust_se: RobustStandardError::Hc3,
                },
                REGRESSION_OLS_METHOD_VERSION,
            ),
            (
                RegressionModelConfig::Logistic,
                REGRESSION_LOGISTIC_METHOD_VERSION,
            ),
        ] {
            let serial_recipe =
                regression_bootstrap_recipe(point_recipe.clone(), model.clone(), 1, 99);
            let parallel_recipe = regression_bootstrap_recipe(point_recipe.clone(), model, 4, 99);
            let serial = run_pls_analysis(&dataset, &serial_recipe, || false, |_| {}).unwrap();
            let parallel = run_pls_analysis(&dataset, &parallel_recipe, || false, |_| {}).unwrap();
            assert_eq!(
                serial.provenance.method_version,
                format!("{base_version}+{REGRESSION_BOOTSTRAP_METHOD_VERSION}")
            );
            for result in [&serial, &parallel] {
                let AnalysisPayload::PlsPmV1 { estimation, .. } = &result.payload else {
                    panic!("regression bootstrap must not use the generic PLS bootstrap payload")
                };
                assert_eq!(estimation["method_version"], base_version);
                assert_eq!(
                    estimation["regression"]["bootstrap"]["method_version"],
                    REGRESSION_BOOTSTRAP_METHOD_VERSION
                );
                assert_eq!(
                    estimation["regression"]["bootstrap"]["requested_replicates"],
                    99
                );
                assert!(
                    estimation["regression"]["bootstrap"]["coefficients"]
                        .as_array()
                        .is_some_and(|rows| !rows.is_empty())
                );
            }
            let extract = |result: &AnalysisResult| match &result.payload {
                AnalysisPayload::PlsPmV1 { estimation, .. } => {
                    estimation["regression"]["bootstrap"]["coefficients"].clone()
                }
                _ => unreachable!(),
            };
            assert_eq!(extract(&serial), extract(&parallel));
        }
    }

    #[test]
    fn regression_bootstrap_cancellation_aborts_without_a_partial_result() {
        use std::sync::atomic::AtomicBool;
        let (dataset, recipe) = logistic_fixture();
        let recipe =
            regression_bootstrap_recipe(recipe, RegressionModelConfig::Logistic, 1, 10_000);
        let cancel = AtomicBool::new(false);
        let result = run_pls_analysis(
            &dataset,
            &recipe,
            || cancel.load(Ordering::Relaxed),
            |update| {
                if update.phase == "bootstrap" && update.completed_units >= 1 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
        );
        assert!(matches!(result, Err(RunnerError::Cancelled)));
    }

    #[test]
    fn bounded_pls_family_provenance_includes_the_actual_estimator_version() {
        let plsc = run_fixture(
            include_bytes!("../../../validation/results/plsc_reference.csv"),
            "plsc_reference.csv",
            include_bytes!("../../../validation/results/plsc_reference.recipe.json"),
        );
        assert_eq!(plsc.provenance.method, AnalysisMethod::Plsc);
        assert_estimator_version_is_in_provenance(&plsc, qpls_estimation::PLSC_METHOD_VERSION);

        let wpls = run_fixture(
            include_bytes!("../../../validation/results/wpls_reference.csv"),
            "wpls_reference.csv",
            include_bytes!("../../../validation/results/wpls_reference.recipe.json"),
        );
        assert_eq!(wpls.provenance.method, AnalysisMethod::Wpls);
        assert_estimator_version_is_in_provenance(&wpls, qpls_estimation::WPLS_METHOD_VERSION);

        let cca = run_fixture(
            include_bytes!("../../../validation/results/cca_reference.csv"),
            "cca_reference.csv",
            include_bytes!("../../../validation/results/cca_reference.recipe.json"),
        );
        assert_eq!(cca.provenance.method, AnalysisMethod::Cca);
        assert_estimator_version_is_in_provenance(&cca, qpls_estimation::CCA_METHOD_VERSION);

        let cta = run_fixture(
            include_bytes!("../../../validation/results/cta_pls_reference.csv"),
            "cta_pls_reference.csv",
            include_bytes!("../../../validation/results/cta_pls_reference.recipe.json"),
        );
        assert_eq!(cta.provenance.method, AnalysisMethod::CtaPls);
        assert_estimator_version_is_in_provenance(&cta, qpls_estimation::CTA_PLS_METHOD_VERSION);

        let ipma = run_fixture(
            include_bytes!("../../../validation/results/ipma_reference.csv"),
            "ipma_reference.csv",
            include_bytes!("../../../validation/results/ipma_reference.recipe.json"),
        );
        assert_eq!(ipma.provenance.method, AnalysisMethod::Ipma);
        assert_eq!(
            ipma.provenance.method_version,
            format!(
                "{PLS_METHOD_VERSION}+{IPMA_METHOD_VERSION}+{PLS_MEDIATION_METHOD_VERSION}+{ASSESSMENT_METHOD_VERSION}"
            )
        );
        assert_estimator_version_is_in_provenance(&ipma, IPMA_METHOD_VERSION);
    }

    #[test]
    fn runner_persists_complete_typed_cca_residual_diagnostics() {
        let result = run_fixture(
            include_bytes!("../../../validation/results/cca_reference.csv"),
            "cca_reference.csv",
            include_bytes!("../../../validation/results/cca_reference.recipe.json"),
        );
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == CCA_RESIDUAL_DIAGNOSTICS_METHOD_VERSION)
        );
        let AnalysisPayload::PlsPmV1 { assessment, .. } = result.payload else {
            panic!("point CCA must use the point-estimation envelope")
        };
        let assessment: qpls_assessment::AssessmentResult =
            serde_json::from_value(assessment).unwrap();
        let diagnostics = assessment.cca_residual_diagnostics.unwrap();
        assert_eq!(
            diagnostics.method_version,
            CCA_RESIDUAL_DIAGNOSTICS_METHOD_VERSION
        );
        assert_eq!(diagnostics.construct_order, ["x", "z", "y"]);
        assert_eq!(diagnostics.expected_pair_count, 3);
        assert_eq!(diagnostics.available_pair_count, 3);
        assert_eq!(diagnostics.unavailable_pair_count, 0);
        assert!(diagnostics.failures.is_empty());
        assert_eq!(diagnostics.cells.len(), 3);
    }

    #[test]
    fn plsc_consistent_bootstrap_runs_through_the_real_runner_with_distinct_attribution() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/plsc_reference.csv"),
            "plsc_consistent_bootstrap_runner.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/plsc_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 1_000;
        recipe.settings.workers = 2;
        recipe.method_config = Some(MethodConfig::Plsc);

        let result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert_eq!(result.provenance.method, AnalysisMethod::Plsc);
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION)
        );
        let AnalysisPayload::PlsPmV2 {
            estimation,
            bootstrap,
            ..
        } = &result.payload
        else {
            panic!("consistent bootstrap must persist as a linked inference payload")
        };
        assert_eq!(
            estimation["method_version"],
            qpls_estimation::PLSC_METHOD_VERSION
        );
        let bootstrap: qpls_resampling::PlscConsistentBootstrapResult =
            serde_json::from_value(bootstrap.clone()).unwrap();
        assert_eq!(
            bootstrap.method_version,
            PLSC_CONSISTENT_BOOTSTRAP_METHOD_VERSION
        );
        assert_eq!(
            bootstrap.estimator_method_version,
            qpls_estimation::PLSC_METHOD_VERSION
        );
        assert_eq!(
            bootstrap.usable_replicates as usize + bootstrap.failed_replicates.len(),
            1_000
        );
    }

    #[test]
    fn bounded_ipma_runner_rejects_resampling_before_base_recipe_normalization() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/ipma_reference.csv"),
            "ipma_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/ipma_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.bootstrap_samples = 999;
        assert!(matches!(
            run_pls_analysis(&dataset, &recipe, || false, |_| {}),
            Err(RunnerError::Recipe(message)) if message.contains("outside this contract")
        ));
    }

    #[test]
    fn prediction_v2_reports_progress_and_honors_cancellation() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/results/plspredict_holdout_reference.csv"),
            "plspredict_holdout_reference.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/plspredict_holdout_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();

        let updates = Mutex::new(Vec::<RunnerProgress>::new());
        let result = run_pls_analysis(
            &dataset,
            &recipe,
            || false,
            |update| updates.lock().unwrap().push(update),
        )
        .unwrap();
        assert_eq!(result.provenance.method, AnalysisMethod::Predict);
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == PLS_PREDICT_METHOD_VERSION)
        );
        let estimation = match &result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => estimation,
            other => panic!("expected a typed Prediction payload, got {other:?}"),
        };
        assert_eq!(
            estimation["predict"]["repeated_kfold"]["method_version"],
            qpls_estimation::PLS_PREDICT_REPEATED_KFOLD_METHOD_VERSION
        );
        let updates = updates.into_inner().unwrap();
        assert!(!updates.is_empty());
        assert!(updates.iter().any(|update| {
            update.total_units > 0 && update.completed_units <= update.total_units
        }));

        let progress_calls = AtomicUsize::new(0);
        let cancelled = run_pls_analysis(
            &dataset,
            &recipe,
            || progress_calls.load(Ordering::SeqCst) >= 8,
            |_| {
                progress_calls.fetch_add(1, Ordering::SeqCst);
            },
        );
        assert!(matches!(cancelled, Err(RunnerError::Cancelled)));
        assert!(progress_calls.load(Ordering::SeqCst) >= 8);

        let mut unsupported = recipe;
        unsupported.settings.permutation_samples = 99;
        assert!(matches!(
            run_pls_analysis(&dataset, &unsupported, || false, |_| {}),
            Err(RunnerError::Recipe(message)) if message.contains("does not accept bootstrap or permutation")
        ));
        unsupported.settings.permutation_samples = 0;
        unsupported.settings.confidence_level = 0.90;
        assert!(matches!(
            run_pls_analysis(&dataset, &unsupported, || false, |_| {}),
            Err(RunnerError::Recipe(message)) if message.contains("fixed one-sided 95%")
        ));
    }

    #[test]
    fn standalone_nca_uses_exact_v2_provenance_and_versionless_assessment_warning() {
        let result = run_fixture(
            include_bytes!("../../../validation/results/v08_extended_methods_fixture.csv"),
            "v08_extended_methods_fixture.csv",
            include_bytes!("../../../validation/results/v08_nca.recipe.json"),
        );

        assert_eq!(result.provenance.method, AnalysisMethod::Nca);
        assert_eq!(result.provenance.method_version, NCA_METHOD_VERSION);
        let (estimation, assessment) = match &result.payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => (estimation, assessment),
            other => panic!("expected standalone NCA payload, got {other:?}"),
        };
        assert_eq!(estimation["method_version"], NCA_METHOD_VERSION);
        assert_eq!(estimation["nca"]["method_version"], NCA_METHOD_VERSION);
        assert_eq!(
            assessment,
            &serde_json::json!({
                "method_version": "assessment_not_applicable_v1",
                "warnings": ["PLS assessment is not applicable to standalone raw-data analyses."]
            })
        );
        assert!(result.diagnostics.iter().all(|diagnostic| {
            !diagnostic.message.contains("v0.8")
                && !diagnostic.message.contains("experimental preview")
        }));
    }

    #[test]
    fn standalone_pca_uses_exact_v1_provenance_and_versionless_assessment_warning() {
        let result = run_fixture(
            include_bytes!("../../../validation/results/v08_extended_methods_fixture.csv"),
            "v08_extended_methods_fixture.csv",
            include_bytes!("../../../validation/results/v08_pca.recipe.json"),
        );

        assert_eq!(result.provenance.method, AnalysisMethod::Pca);
        assert_eq!(result.provenance.method_version, PCA_METHOD_VERSION);
        let (estimation, assessment) = match &result.payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => (estimation, assessment),
            other => panic!("expected standalone PCA payload, got {other:?}"),
        };
        assert_eq!(estimation["method_version"], PCA_METHOD_VERSION);
        assert_eq!(estimation["pca"]["method_version"], PCA_METHOD_VERSION);
        assert_eq!(estimation["pca"]["retained_components"], 3);
        assert_eq!(
            assessment,
            &serde_json::json!({
                "method_version": "assessment_not_applicable_v1",
                "warnings": ["PLS assessment is not applicable to standalone raw-data analyses."]
            })
        );
    }

    #[test]
    fn gsca_als_v2_uses_exact_provenance_reports_progress_and_honors_cancellation() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.settings.method = AnalysisMethod::Gsca;
        recipe.method_config = Some(MethodConfig::Gsca);
        recipe.settings.workers = 1;
        recipe.settings.max_iterations = 3_000;
        recipe.settings.tolerance = 1e-7;

        let updates = Mutex::new(Vec::<RunnerProgress>::new());
        let result = run_pls_analysis(
            &dataset,
            &recipe,
            || false,
            |update| updates.lock().unwrap().push(update),
        )
        .unwrap();
        assert_eq!(result.provenance.method, AnalysisMethod::Gsca);
        assert_eq!(result.provenance.method_version, GSCA_METHOD_VERSION);
        let (estimation, assessment) = match &result.payload {
            AnalysisPayload::PlsPmV1 {
                estimation,
                assessment,
            } => (estimation, assessment),
            other => panic!("expected typed GSCA payload, got {other:?}"),
        };
        assert_eq!(estimation["method_version"], GSCA_METHOD_VERSION);
        assert_eq!(estimation["gsca"]["method_version"], GSCA_METHOD_VERSION);
        assert_eq!(
            estimation["gsca"]["algorithm"],
            qpls_estimation::GSCA_ALGORITHM_VERSION
        );
        assert!(
            estimation["gsca"]["bootstrap_intervals"]
                .as_array()
                .is_some_and(Vec::is_empty)
        );
        assert_eq!(
            assessment,
            &serde_json::json!({
                "method_version": "assessment_not_applicable_v1",
                "warnings": ["PLS assessment is not applicable to GSCA ALS component-model estimation."]
            })
        );
        assert!(
            updates
                .into_inner()
                .unwrap()
                .iter()
                .any(|update| update.phase == "iterating")
        );

        let calls = AtomicUsize::new(0);
        let cancelled = run_pls_analysis(
            &dataset,
            &recipe,
            || calls.load(Ordering::SeqCst) >= 3,
            |_| {
                calls.fetch_add(1, Ordering::SeqCst);
            },
        );
        assert!(matches!(cancelled, Err(RunnerError::Cancelled)));

        recipe.settings.bootstrap_samples = 999;
        assert!(matches!(
            run_pls_analysis(&dataset, &recipe, || false, |_| {}),
            Err(RunnerError::Recipe(message)) if message.contains("does not expose inference")
        ));
    }

    #[test]
    fn plsc_consistent_permutation_dispatch_is_worker_invariant_and_cancellable() {
        for group in ["A", "B"] {
            let fit = plsc_consistent_permutation_original_group_fit(group);
            assert_eq!(fit.used_observations, 160);
            assert!(fit.outer_estimates.iter().all(|row| {
                row.weight.is_finite()
                    && row.weight > 0.0
                    && row.loading.is_finite()
                    && row.loading > 0.0
            }));
            assert!(
                fit.paths
                    .iter()
                    .all(|path| path.coefficient.is_finite() && path.coefficient > 0.0)
            );
            let plsc = fit.plsc.expect("original group point fit is PLSc");
            assert!(
                plsc.reliabilities
                    .iter()
                    .all(|row| row.rho_a.is_finite() && row.rho_a > 0.0 && row.rho_a <= 1.0)
            );
            assert!(
                plsc.construct_correlations
                    .iter()
                    .all(|row| { row.corrected.is_finite() && row.corrected.abs() <= 1.0 + 1e-10 })
            );
            assert!(plsc.corrected_outer_loadings.iter().all(|row| {
                row.loading.is_finite() && row.loading > 0.0 && row.loading <= 1.0 + 1e-10
            }));
            assert!(
                plsc.corrected_paths
                    .iter()
                    .all(|path| path.coefficient.is_finite() && path.coefficient > 0.0)
            );
        }
        let (dataset, mut serial_recipe) = plsc_consistent_permutation_fixture(1);
        let (_, mut parallel_recipe) = plsc_consistent_permutation_fixture(4);
        for recipe in [&mut serial_recipe, &mut parallel_recipe] {
            let Some(MethodConfig::PlscPermutation { test_tail, .. }) =
                recipe.method_config.as_mut()
            else {
                unreachable!()
            };
            *test_tail = qpls_core::PlscPermutationTestTail::GroupAGreater;
        }
        let serial = run_pls_analysis(&dataset, &serial_recipe, || false, |_| {}).unwrap();
        let parallel = run_pls_analysis(&dataset, &parallel_recipe, || false, |_| {}).unwrap();
        assert_eq!(serial.payload, parallel.payload);

        let AnalysisPayload::PlsPmV3 {
            bootstrap,
            permutation: Some(permutation),
            ..
        } = &serial.payload
        else {
            panic!("PLSc consistent permutation must use the inference envelope")
        };
        assert!(bootstrap.is_none());
        let permutation: qpls_resampling::PlscConsistentPermutationResult =
            serde_json::from_value(permutation.clone()).unwrap();
        assert_eq!(
            permutation.method_version,
            PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION
        );
        assert_eq!(
            permutation.scheduler_method_version,
            PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION
        );
        assert_eq!(permutation.plan.permutations, 99);
        assert_eq!(permutation.permutation_ledger.len(), 99);
        assert_eq!(
            permutation.usable_permutations as usize + permutation.failed_permutations.len(),
            99
        );
        let families = permutation
            .parameters
            .iter()
            .map(|parameter| parameter.family)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            families,
            [
                qpls_resampling::PlscPermutationParameterFamily::Path,
                qpls_resampling::PlscPermutationParameterFamily::OuterLoading,
                qpls_resampling::PlscPermutationParameterFamily::RhoA,
                qpls_resampling::PlscPermutationParameterFamily::ConstructCorrelation,
                qpls_resampling::PlscPermutationParameterFamily::RSquared,
            ]
            .into_iter()
            .collect()
        );
        let selected = permutation
            .selected_tail_inference
            .as_ref()
            .expect("explicit Group A greater selection emits a receipt");
        assert_eq!(
            selected.method_version,
            PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION
        );
        assert_eq!(
            selected.selected_test_tail,
            qpls_core::PlscPermutationTestTail::GroupAGreater
        );
        let directional = permutation.directional_inference.as_ref().unwrap();
        for (selected, directional) in selected.parameters.iter().zip(&directional.parameters) {
            assert_eq!(selected.parameter, directional.parameter);
            assert_eq!(selected.selected_exceedances, directional.greater_or_equal);
            assert_eq!(
                selected.selected_p_value.to_bits(),
                directional.p_value_greater.to_bits()
            );
            assert_eq!(selected.permutations, directional.permutations);
        }
        for version in [
            "plsc_v2",
            PLSC_CONSISTENT_PERMUTATION_METHOD_VERSION,
            PLSC_CONSISTENT_PERMUTATION_SCHEDULER_VERSION,
            PLSC_CONSISTENT_PERMUTATION_SELECTED_TAIL_METHOD_VERSION,
        ] {
            assert!(
                serial
                    .provenance
                    .method_version
                    .split('+')
                    .any(|actual| actual == version),
                "missing exact provenance marker {version}"
            );
        }

        let cancel = AtomicBool::new(false);
        let cancelled = run_pls_analysis(
            &dataset,
            &serial_recipe,
            || cancel.load(Ordering::SeqCst),
            |update| {
                if update.phase == "permutation" && update.completed_units > 0 {
                    cancel.store(true, Ordering::SeqCst);
                }
            },
        );
        assert!(cancel.load(Ordering::SeqCst));
        assert!(matches!(cancelled, Err(RunnerError::Cancelled)));
    }

    #[test]
    fn historical_recipe_is_readable_but_requires_explicit_migration_before_execution() {
        let dataset = import_delimited_bytes(
            include_bytes!("../../../validation/fixtures/simple_reflective.csv"),
            "simple_reflective.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut historical: AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/fixtures/simple_reflective.recipe.json"
        ))
        .unwrap();
        historical.dataset_fingerprint = dataset.fingerprint.0.clone();

        assert!(matches!(
            run_pls_analysis(&dataset, &historical, || false, |_| {}),
            Err(RunnerError::Recipe(message))
                if message.contains("readable but not executable")
                    && message.contains("migrate")
        ));
    }

    #[test]
    fn micom_v31_runner_method_version_seam_is_micom_only() {
        let mut exact = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/micom_v2_reference.recipe.json"
        ));
        exact.method_config = Some(MethodConfig::Micom {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            permutation_samples: 5_000,
            configural_invariance_confirmed: true,
        });
        assert_eq!(mga_method_versions(&exact), vec![MICOM_METHOD_VERSION]);

        let mut legacy = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/micom_v2_reference.recipe.json"
        ));
        legacy.method_config = Some(MethodConfig::Mga {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            methods: vec![
                qpls_core::GroupAnalysisMethod::Micom,
                qpls_core::GroupAnalysisMethod::MgaPermutation,
            ],
            permutation_samples: 5_000,
            configural_invariance_confirmed: true,
        });
        assert_eq!(
            mga_method_versions(&legacy),
            vec![
                PLS_MGA_METHOD_VERSION,
                PLS_MGA_PERMUTATION_METHOD_VERSION,
                MICOM_METHOD_VERSION_V4,
            ]
        );
    }

    #[test]
    #[ignore = "qualification-scale: runs the exact 5,000-attempt full-refit MICOM contract"]
    fn micom_v31_runner_qualification_scale_emits_micom_only_no_retry_provenance() {
        let dataset = import_delimited_bytes(
            include_bytes!(
                "../../../validation/fixtures/plsc_consistent_permutation_two_group.csv"
            ),
            "plsc_consistent_permutation_two_group.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut recipe = migrated_execution_recipe(include_bytes!(
            "../../../validation/results/micom_v2_reference.recipe.json"
        ));
        recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        recipe.method_config = Some(MethodConfig::Micom {
            group_column: "group".into(),
            group_a: "A".into(),
            group_b: "B".into(),
            permutation_samples: 5_000,
            configural_invariance_confirmed: true,
        });

        let result = run_pls_analysis(&dataset, &recipe, || false, |_| {}).unwrap();
        assert!(
            result
                .provenance
                .method_version
                .split('+')
                .any(|version| version == MICOM_METHOD_VERSION)
        );
        assert!(!result.provenance.method_version.split('+').any(|version| {
            version == PLS_MGA_METHOD_VERSION || version == PLS_MGA_PERMUTATION_METHOD_VERSION
        }));
        let estimation = match result.payload {
            AnalysisPayload::PlsPmV1 { estimation, .. } => {
                serde_json::from_value::<qpls_estimation::PlsResult>(estimation).unwrap()
            }
            other => panic!("unexpected MICOM payload: {other:?}"),
        };
        assert!(estimation.converged);
        assert!(estimation.mga.is_none());
        assert!(estimation.mga_permutation.is_none());
        let micom = estimation.micom.unwrap();
        assert_eq!(micom.method_version, MICOM_METHOD_VERSION);
        assert_eq!(micom.attempted_permutations, Some(5_000));
        assert_eq!(micom.retry_policy.as_deref(), Some("none"));
        assert_eq!(micom.permutation_ledger.len(), 5_000);
        assert_eq!(micom.step1_computed, Some(false));
    }
}
