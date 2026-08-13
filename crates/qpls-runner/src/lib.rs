use chrono::Utc;
use qpls_assessment::{
    ASSESSMENT_METHOD_VERSION, AssessmentError, assess_pls_validated_with_control,
};
#[cfg(test)]
use qpls_core::ANALYSIS_RECIPE_SCHEMA_VERSION;
use qpls_core::{AnalysisRecipe, AnalysisResult, ValidatedExecutionRecipe};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_BOOTSTRAP_METHOD_VERSION, CBSEM_FIT_METHOD_VERSION,
    CBSEM_MODIFICATION_INDICES_METHOD_VERSION, CBSEM_MULTIGROUP_METHOD_VERSION, EstimationError,
    FIMIX_PLS_METHOD_VERSION, GSCA_METHOD_VERSION, IPMA_METHOD_VERSION, MICOM_METHOD_VERSION,
    NCA_METHOD_VERSION, PCA_METHOD_VERSION, PLS_MEDIATION_METHOD_VERSION, PLS_METHOD_VERSION,
    PLS_MGA_METHOD_VERSION, PLS_MGA_PERMUTATION_METHOD_VERSION, PLS_POS_METHOD_VERSION,
    PLS_PREDICT_METHOD_VERSION, PLS_SEGMENTATION_METHOD_VERSION,
    PLS_TWO_STAGE_MODERATION_METHOD_VERSION, REGRESSION_LOGISTIC_METHOD_VERSION,
    REGRESSION_OLS_METHOD_VERSION, REGRESSION_PROCESS_METHOD_VERSION,
    REGRESSION_PROCESS_METHOD_VERSION_V1, estimate_pls_validated_with_control,
};
use qpls_resampling::{
    PERMUTATION_METHOD_VERSION, PROCESS_BOOTSTRAP_METHOD_VERSION, PlsBootstrapError,
    PlsPermutationError, REGRESSION_BOOTSTRAP_METHOD_VERSION,
    REGRESSION_BOOTSTRAP_MINIMUM_USABLE_FRACTION, RESAMPLING_METHOD_VERSION,
    RegressionBootstrapError, ResamplingError, bootstrap_pls_validated,
    bootstrap_process_validated, bootstrap_regression_validated, permutation_pls_validated,
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
    let execution = ValidatedExecutionRecipe::for_dataset(recipe, &dataset.fingerprint.0)
        .map_err(|error| RunnerError::Recipe(error.to_string()))?;
    let base_execution = execution
        .without_outer_resampling()
        .map_err(|error| RunnerError::Recipe(error.to_string()))?;
    let effective_recipe = execution.effective();
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
    let bootstrap = if effective_recipe.settings.bootstrap_samples > 0 && !standalone_v08 {
        Some(
            bootstrap_pls_validated(
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
            .map_err(map_bootstrap_error)?,
        )
    } else {
        None
    };

    if should_cancel() {
        return Err(RunnerError::Cancelled);
    }
    let permutation = if effective_recipe.settings.permutation_samples > 0 && !standalone_v08 {
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
    let estimation_method_version = estimation.method_version.clone();
    let estimation = serde_json::to_value(estimation)?;
    let mut base_versions = vec![
        PLS_METHOD_VERSION,
        PLS_MEDIATION_METHOD_VERSION,
        ASSESSMENT_METHOD_VERSION,
    ];
    if matches!(
        effective_recipe.settings.method,
        qpls_core::AnalysisMethod::Plsc
            | qpls_core::AnalysisMethod::Wpls
            | qpls_core::AnalysisMethod::Cca
            | qpls_core::AnalysisMethod::CtaPls
    ) {
        base_versions.insert(1, estimation_method_version.as_str());
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
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Mga {
        base_versions.insert(1, PLS_MGA_METHOD_VERSION);
        if metadata_list_contains(&effective_recipe, "group_methods", "mga_permutation") {
            base_versions.insert(2, PLS_MGA_PERMUTATION_METHOD_VERSION);
        }
        if metadata_list_contains(&effective_recipe, "group_methods", "micom") {
            base_versions.insert(3, MICOM_METHOD_VERSION);
        }
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Ipma {
        base_versions.insert(1, IPMA_METHOD_VERSION);
    }
    if effective_recipe.settings.method == qpls_core::AnalysisMethod::Cbsem {
        base_versions.insert(1, estimation_method_version.as_str());
        base_versions.insert(2, CBSEM_FIT_METHOD_VERSION);
        base_versions.insert(3, CBSEM_MODIFICATION_INDICES_METHOD_VERSION);
        if effective_recipe
            .metadata
            .contains_key("cbsem_bootstrap_samples")
        {
            base_versions.insert(4, CBSEM_BOOTSTRAP_METHOD_VERSION);
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
        atomic::{AtomicUsize, Ordering},
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
}
