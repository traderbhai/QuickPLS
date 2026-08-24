use crate::{
    DesktopProject, InternalRecipeV4ExecutionFailureV1, InternalRecipeV4ExecutionIssueV1,
    InternalRecipeV4ExecutionStageV1, InternalRecipeV4ExecutionSurfaceV1,
    InternalRecipeV4ResidentDataV1, internal_recipe_v4_failure, map_recipe_v4_compilation_failure,
};
use qpls_core::{
    AnalysisRecipeV4, CapabilityCellReferenceV2, MethodConfig, RecipeV4CompilationError,
    RecipeV4CompilerTarget, SemModelV4, cbsem_exact_bootstrap_capability_cell_v1,
    compile_analysis_recipe_v4,
};
use qpls_data::Dataset;
use qpls_estimation::CbsemCompiledMomentErrorV2;
use qpls_project::Project;
use qpls_runner::{
    RecipeV4CbsemExecutionError, RecipeV4CbsemExecutionResultV1, RunnerProgress,
    run_compiled_cbsem_recipe_v4,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InternalRecipeV4CbsemExecutionRequestV1 {
    pub(crate) surface: InternalRecipeV4ExecutionSurfaceV1,
    pub(crate) experimental_labs_enabled: bool,
    pub(crate) resident_data: InternalRecipeV4ResidentDataV1,
    pub(crate) dataset_id: String,
    pub(crate) dataset_fingerprint: String,
    pub(crate) recipe: AnalysisRecipeV4,
    pub(crate) model: SemModelV4,
    pub(crate) compiler_target: RecipeV4CompilerTarget,
    pub(crate) capability_cell: CapabilityCellReferenceV2,
}

pub(crate) fn validate_internal_recipe_v4_cbsem_access(
    request: &InternalRecipeV4CbsemExecutionRequestV1,
) -> Result<(), InternalRecipeV4ExecutionFailureV1> {
    let historical_internal = request.surface == InternalRecipeV4ExecutionSurfaceV1::InternalLabs
        && request.experimental_labs_enabled;
    let current_standard = request.surface == InternalRecipeV4ExecutionSurfaceV1::Standard
        && !request.experimental_labs_enabled;
    if !historical_internal && !current_standard {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Access,
            "surface",
            "recipe_v4.cbsem.execution_surface_mismatch",
            "CB-SEM Recipe-v4 execution requires either the current Standard surface or a historical internal-Labs request with matching access state.",
            "Use Standard with Experimental Labs disabled for current point/exact-bootstrap execution; preserve internal_labs plus the opt-in only when replaying a historical request.",
        ));
    }
    if request.resident_data != InternalRecipeV4ResidentDataV1::ProjectResident {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "residentData",
            "recipe_v4.cbsem.project_resident_data_required",
            "CB-SEM Recipe-v4 execution requires a dataset resident in the active project.",
            "Import the raw or moment dataset into the active project, then rebuild the request from its current identity.",
        ));
    }
    if request.compiler_target != RecipeV4CompilerTarget::CbsemPlanV2 {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "compilerTarget",
            "recipe_v4.cbsem.target_required",
            "This internal command executes only the bounded CB-SEM compiler target.",
            "Use compilerTarget cbsem_plan_v2 with the exact CB-SEM ML capability-cell identity.",
        ));
    }
    let exact_bootstrap_requested = matches!(
        request.recipe.method_config.as_ref(),
        Some(MethodConfig::Cbsem {
            bootstrap_v2: Some(_),
            ..
        })
    );
    let expected_capability_cell = if exact_bootstrap_requested {
        cbsem_exact_bootstrap_capability_cell_v1()
    } else {
        request.compiler_target.capability_cell()
    };
    if request.capability_cell != expected_capability_cell {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "capabilityCell",
            "recipe_v4.cbsem.capability_cell_mismatch",
            "The capability-cell identity does not match the selected CB-SEM point or exact-bootstrap method.",
            if exact_bootstrap_requested {
                "Use smartpls.cbsem_bootstrapping / qpls3.cbsem.bootstrap / cbsem_exact_case_bootstrap_v1."
            } else {
                "Use smartpls.cbsem / qpls3.cbsem.ml / cbsem_ml_v1."
            },
        ));
    }
    Ok(())
}

pub(crate) fn resolve_internal_recipe_v4_cbsem_dataset(
    project: &Project,
    request: &InternalRecipeV4CbsemExecutionRequestV1,
) -> Result<Dataset, InternalRecipeV4ExecutionFailureV1> {
    let dataset = project
        .datasets
        .iter()
        .find(|dataset| dataset.id.to_string() == request.dataset_id)
        .ok_or_else(|| {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::DataResolution,
                "datasetId",
                "recipe_v4.cbsem.dataset_not_resident",
                format!(
                    "Dataset {} is not resident in the active project.",
                    request.dataset_id
                ),
                "Select a resident raw, covariance, or correlation dataset and rebuild the request from its current identity.",
            )
        })?;
    if dataset.fingerprint.0 != request.dataset_fingerprint {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "datasetFingerprint",
            "recipe_v4.cbsem.dataset_fingerprint_mismatch",
            "The requested dataset fingerprint does not match the resident dataset.",
            "Refresh the dataset binding and compile a new immutable CB-SEM execution artifact.",
        ));
    }
    Ok(dataset.clone())
}

fn attach_issues(
    mut failure: InternalRecipeV4ExecutionFailureV1,
    issues: impl IntoIterator<Item = InternalRecipeV4ExecutionIssueV1>,
) -> InternalRecipeV4ExecutionFailureV1 {
    failure.issues = issues.into_iter().collect();
    failure
}

fn map_cbsem_compilation_failure(
    error: RecipeV4CompilationError,
) -> InternalRecipeV4ExecutionFailureV1 {
    let issues = match &error {
        RecipeV4CompilationError::CbsemEstimatorCapability(error) => error
            .issues
            .iter()
            .map(|issue| InternalRecipeV4ExecutionIssueV1 {
                code: issue.code.clone(),
                subject: issue.subject.clone(),
                message: issue.message.clone(),
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let mut failure = map_recipe_v4_compilation_failure(error);
    if !issues.is_empty() {
        failure.code = "recipe_v4.cbsem.unsupported_plan".into();
        failure.corrective_action =
            "Correct every typed CB-SEM model or data issue and compile a new immutable artifact."
                .into();
    }
    attach_issues(failure, issues)
}

fn matrix_issues(
    issues: &[qpls_estimation::CbsemMatrixModelIssueV2],
) -> Vec<InternalRecipeV4ExecutionIssueV1> {
    issues
        .iter()
        .map(|issue| InternalRecipeV4ExecutionIssueV1 {
            code: issue.code.clone(),
            subject: issue.subject.clone(),
            message: issue.message.clone(),
        })
        .collect()
}

fn map_cbsem_execution_failure(
    error: RecipeV4CbsemExecutionError,
) -> InternalRecipeV4ExecutionFailureV1 {
    match error {
        RecipeV4CbsemExecutionError::Cancelled => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Estimation,
            "execution",
            "recipe_v4.cbsem.execution_cancelled",
            error.to_string(),
            "Run the internal CB-SEM analysis again when ready.",
        ),
        RecipeV4CbsemExecutionError::Compilation(error) => {
            let mut failure = map_cbsem_compilation_failure(error);
            failure.stage = InternalRecipeV4ExecutionStageV1::Integrity;
            failure.code = "recipe_v4.cbsem.compiled_artifact_revalidation_failed".into();
            failure.corrective_action =
                "Discard the stale artifact and compile again from the exact recipe and resolved model."
                    .into();
            failure
        }
        RecipeV4CbsemExecutionError::CompilerTarget(_) => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Capability,
            "compilerTarget",
            "recipe_v4.cbsem.target_required",
            error.to_string(),
            "Compile and execute only the cbsem_plan_v2 target through this internal command.",
        ),
        RecipeV4CbsemExecutionError::MomentInput(moment_error) => match moment_error {
            CbsemCompiledMomentErrorV2::Cancelled => internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Estimation,
                "execution",
                "recipe_v4.cbsem.execution_cancelled",
                moment_error.to_string(),
                "Run the internal CB-SEM analysis again when ready.",
            ),
            CbsemCompiledMomentErrorV2::UnsupportedPlan { ref issues } => attach_issues(
                internal_recipe_v4_failure(
                    InternalRecipeV4ExecutionStageV1::Capability,
                    "model",
                    "recipe_v4.cbsem.unsupported_plan",
                    moment_error.to_string(),
                    "Correct every typed CB-SEM model or data issue and compile a new immutable artifact.",
                ),
                matrix_issues(issues),
            ),
            CbsemCompiledMomentErrorV2::CompiledArtifact(_)
            | CbsemCompiledMomentErrorV2::DatasetFingerprintMismatch
            | CbsemCompiledMomentErrorV2::DatasetIntegrity(_) => internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Integrity,
                "artifact",
                "recipe_v4.cbsem.integrity_failed",
                moment_error.to_string(),
                "Discard the stale artifact, reload the resident dataset, and compile again.",
            ),
            CbsemCompiledMomentErrorV2::WrongCompiledPlan
            | CbsemCompiledMomentErrorV2::UnsupportedPreprocessing => internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Capability,
                "recipe",
                "recipe_v4.cbsem.configuration_unsupported",
                moment_error.to_string(),
                "Use the exact bounded CB-SEM ML Recipe-v4 settings and compile again.",
            ),
            CbsemCompiledMomentErrorV2::MeanReplacement(_) => internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::DataResolution,
                "missingData",
                "recipe_v4.cbsem.mean_replacement_failed",
                moment_error.to_string(),
                "Correct the continuous raw data, missing-marker metadata, or modeled-variable binding and compile again.",
            ),
            CbsemCompiledMomentErrorV2::NonConvergence
            | CbsemCompiledMomentErrorV2::Estimation(_) => internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Estimation,
                "estimator",
                "recipe_v4.cbsem.estimation_failed",
                moment_error.to_string(),
                "Review the typed diagnostics, correct the model or data, and execute again.",
            ),
            other => internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::DataResolution,
                "dataset",
                "recipe_v4.cbsem.matrix_binding_failed",
                other.to_string(),
                "Correct the exact sample, denominator, scale, variable-order, or matrix data issue and compile again.",
            ),
        },
        RecipeV4CbsemExecutionError::MomentMethodVersionMismatch { .. }
        | RecipeV4CbsemExecutionError::EstimatorMethodVersionMismatch { .. }
        | RecipeV4CbsemExecutionError::MomentResultSchemaMismatch { .. } => {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Integrity,
                "methodVersion",
                "recipe_v4.cbsem.method_version_mismatch",
                error.to_string(),
                "Rebuild and requalify the internal adapter before executing this method version.",
            )
        }
        RecipeV4CbsemExecutionError::MeanReplacementContractMismatch(_) => {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Integrity,
                "missingDataTreatment",
                "recipe_v4.cbsem.mean_replacement_contract_mismatch",
                error.to_string(),
                "Discard the result and rebuild the exact mean-replacement execution from the resident dataset.",
            )
        }
        RecipeV4CbsemExecutionError::ScoreLmContractMismatch(_) => internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::Integrity,
            "scoreLm",
            "recipe_v4.cbsem.score_lm_contract_mismatch",
            error.to_string(),
            "Discard the result and rebuild the exact score/LM execution from the resident recipe, model, and dataset.",
        ),
        RecipeV4CbsemExecutionError::ExactCaseBootstrapContract(_)
        | RecipeV4CbsemExecutionError::ExactCaseBootstrapScheduler(_) => {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::Integrity,
                "bootstrap",
                "recipe_v4.cbsem.exact_case_bootstrap_contract_mismatch",
                error.to_string(),
                "Discard the result and rerun the exact CFA case bootstrap from the resident recipe, model, and dataset.",
            )
        }
    }
}

pub(crate) fn execute_internal_recipe_v4_cbsem_with_control(
    dataset: &Dataset,
    request: &InternalRecipeV4CbsemExecutionRequestV1,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<RecipeV4CbsemExecutionResultV1, InternalRecipeV4ExecutionFailureV1> {
    validate_internal_recipe_v4_cbsem_access(request)?;
    if dataset.id.to_string() != request.dataset_id
        || dataset.fingerprint.0 != request.dataset_fingerprint
    {
        return Err(internal_recipe_v4_failure(
            InternalRecipeV4ExecutionStageV1::DataResolution,
            "dataset",
            "recipe_v4.cbsem.resident_dataset_identity_changed",
            "The resident dataset identity changed before CB-SEM Recipe-v4 execution began.",
            "Refresh the data binding and compile a new immutable execution artifact.",
        ));
    }
    let artifact = compile_analysis_recipe_v4(
        &request.recipe,
        Some(&request.model),
        request.compiler_target,
        request.capability_cell.clone(),
    )
    .map_err(map_cbsem_compilation_failure)?;
    run_compiled_cbsem_recipe_v4(
        dataset,
        &request.recipe,
        &request.model,
        &artifact,
        should_cancel,
        progress,
    )
    .map_err(map_cbsem_execution_failure)
}

pub(crate) fn execute_internal_recipe_v4_cbsem(
    dataset: &Dataset,
    request: &InternalRecipeV4CbsemExecutionRequestV1,
) -> Result<RecipeV4CbsemExecutionResultV1, InternalRecipeV4ExecutionFailureV1> {
    execute_internal_recipe_v4_cbsem_with_control(dataset, request, || false, |_| {})
}

#[tauri::command]
pub(crate) fn run_internal_labs_recipe_v4_cbsem_execution(
    request: InternalRecipeV4CbsemExecutionRequestV1,
    state: State<'_, DesktopProject>,
) -> Result<RecipeV4CbsemExecutionResultV1, InternalRecipeV4ExecutionFailureV1> {
    validate_internal_recipe_v4_cbsem_access(&request)?;
    let dataset = {
        let project = state.0.lock().map_err(|_| {
            internal_recipe_v4_failure(
                InternalRecipeV4ExecutionStageV1::DataResolution,
                "project",
                "recipe_v4.cbsem.project_state_unavailable",
                "The active project data is temporarily unavailable.",
                "Retry after the active project finishes its current operation.",
            )
        })?;
        resolve_internal_recipe_v4_cbsem_dataset(&project, &request)?
    };
    execute_internal_recipe_v4_cbsem(&dataset, &request)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisSettings, CbsemBootstrapAlgorithm, CbsemBootstrapConfigV2, CbsemBootstrapInterval,
        CbsemBootstrapTestTail, CbsemEstimator, CbsemInput, CbsemModelType, Construct,
        FactorMeanPolicyV4, LegacyBasicModelInterpretationV4, LegacyEstimandConfirmationV4,
        MeasurementMode, MethodConfig, MissingDataPolicyV4, ModelSpec, Preprocessing,
        SemCovarianceDenominatorV4, SemDataBindingV4, SemMatrixSampleMetadataV4,
        SemParameterTargetV4, SemParameterV4, SemVariableV4, StructuralPath,
        convert_legacy_basic_model_v4,
    };
    use qpls_data::{DataKind, ImportOptions, import_delimited_bytes, write_arrow};
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use uuid::Uuid;

    pub(crate) fn fixture() -> (Project, InternalRecipeV4CbsemExecutionRequestV1) {
        let dataset = import_delimited_bytes(
            b",x1,x2,x3\nx1,4.0,2.0,1.2\nx2,2.0,3.0,1.0\nx3,1.2,1.0,2.0\n",
            "cbsem-covariance.csv",
            b',',
            &ImportOptions {
                data_kind: DataKind::Covariance,
                sample_size: Some(120),
                ..ImportOptions::default()
            },
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_4001),
            name: "CB-SEM matrix job fixture".into(),
            constructs: vec![Construct {
                id: "f".into(),
                name: "Factor".into(),
                short_name: "F".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            }],
            paths: Vec::<StructuralPath>::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = SemDataBindingV4::Covariance {
            dataset_id: dataset.id.to_string(),
            variables: vec![
                "observed:x1".into(),
                "observed:x2".into(),
                "observed:x3".into(),
            ],
            means: None,
            standard_deviations: None,
            sample: SemMatrixSampleMetadataV4 {
                sample_size: 120,
                covariance_denominator: SemCovarianceDenominatorV4::SampleNMinusOne,
                effective_sample_size: None,
                degrees_of_freedom: None,
                group_sample_sizes: BTreeMap::new(),
            },
        };
        model.ensure_valid().unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.workers = 1;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0xCB5E_4002),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: model.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Cfa,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Covariance,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: None,
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        let request = InternalRecipeV4CbsemExecutionRequestV1 {
            surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
            experimental_labs_enabled: true,
            resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe,
            model,
            compiler_target: RecipeV4CompilerTarget::CbsemPlanV2,
            capability_cell: RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        };
        let mut project = Project::new("CB-SEM Recipe-v4 internal fixture");
        project.datasets.push(dataset);
        (project, request)
    }

    pub(crate) fn mean_fixture() -> (Project, InternalRecipeV4CbsemExecutionRequestV1) {
        let mut csv = String::from("x1,x2,x3\n");
        for index in 0..40 {
            let centered = index as f64 - 19.5;
            let a = ((index * 7) % 11) as f64 - 5.0;
            let b = ((index * 5) % 13) as f64 - 6.0;
            let x1 = centered + 0.30 * a + 3.0;
            let x2 = 0.80 * centered + 0.50 * b + 4.4;
            let x3 = 0.50 * centered - 0.40 * a + 0.20 * b + 0.5;
            csv.push_str(&format!("{x1:.17},{x2:.17},{x3:.17}\n"));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "cbsem-raw-mean-structure.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_4011),
            name: "CB-SEM raw mean job fixture".into(),
            constructs: vec![Construct {
                id: "f".into(),
                name: "Factor".into(),
                short_name: "F".into(),
                mode: MeasurementMode::Reflective,
                indicators: vec!["x1".into(), "x2".into(), "x3".into()],
            }],
            paths: Vec::<StructuralPath>::new(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        let factor_id = model
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::CommonFactor {
                    id, mean_policy, ..
                } => {
                    *mean_policy = FactorMeanPolicyV4::Estimated {
                        parameter: "parameter:factor_mean:f".into(),
                    };
                    Some(id.clone())
                }
                _ => None,
            })
            .unwrap();
        model.parameters.extend([
            SemParameterV4::Fixed {
                id: "parameter:intercept:x1".into(),
                label: "x1 intercept anchor".into(),
                target: SemParameterTargetV4::Intercept {
                    variable: "observed:x1".into(),
                },
                value: 0.0,
                group_overrides: Vec::new(),
            },
            SemParameterV4::Free {
                id: "parameter:intercept:x2".into(),
                label: "x2 intercept".into(),
                target: SemParameterTargetV4::Intercept {
                    variable: "observed:x2".into(),
                },
                start: Some(2.0),
                lower: Some(-20.0),
                upper: Some(20.0),
                equality_label: None,
                group_overrides: Vec::new(),
            },
            SemParameterV4::Free {
                id: "parameter:intercept:x3".into(),
                label: "x3 intercept".into(),
                target: SemParameterTargetV4::Intercept {
                    variable: "observed:x3".into(),
                },
                start: Some(-1.0),
                lower: Some(-20.0),
                upper: Some(20.0),
                equality_label: None,
                group_overrides: Vec::new(),
            },
            SemParameterV4::Free {
                id: "parameter:factor_mean:f".into(),
                label: "Factor mean".into(),
                target: SemParameterTargetV4::Mean {
                    variable: factor_id,
                },
                start: Some(3.0),
                lower: Some(-20.0),
                upper: Some(20.0),
                equality_label: None,
                group_overrides: Vec::new(),
            },
        ]);
        model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.workers = 1;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0xCB5E_4012),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: model.clone(),
                scientific_sha256: model.scientific_sha256().unwrap(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Cfa,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: true,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: None,
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        let request = InternalRecipeV4CbsemExecutionRequestV1 {
            surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
            experimental_labs_enabled: true,
            resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe,
            model,
            compiler_target: RecipeV4CompilerTarget::CbsemPlanV2,
            capability_cell: RecipeV4CompilerTarget::CbsemPlanV2.capability_cell(),
        };
        let mut project = Project::new("CB-SEM Recipe-v4 raw mean fixture");
        project.datasets.push(dataset);
        (project, request)
    }

    fn mean_replacement_fixture() -> (Project, InternalRecipeV4CbsemExecutionRequestV1) {
        let (_, mut request) = fixture();
        let rows = (0..40)
            .map(|index| {
                let centered = index as f64 - 19.5;
                let a = ((index * 7) % 11) as f64 - 5.0;
                let b = ((index * 5) % 13) as f64 - 6.0;
                vec![
                    centered + 0.30 * a,
                    0.80 * centered + 0.50 * b,
                    0.50 * centered - 0.40 * a + 0.20 * b,
                ]
            })
            .collect::<Vec<_>>();
        let means = (0..3)
            .map(|column| rows.iter().map(|row| row[column]).sum::<f64>() / rows.len() as f64)
            .collect::<Vec<_>>();
        let scales = (0..3)
            .map(|column| {
                (rows
                    .iter()
                    .map(|row| {
                        let deviation = row[column] - means[column];
                        deviation * deviation
                    })
                    .sum::<f64>()
                    / rows.len() as f64)
                    .sqrt()
            })
            .collect::<Vec<_>>();
        let mut csv = String::from("x1,x2,x3\n");
        for (index, row) in rows.into_iter().enumerate() {
            let standardized = [
                (row[0] - means[0]) / scales[0],
                (row[1] - means[1]) / scales[1],
                (row[2] - means[2]) / scales[2],
            ];
            let x1 = (index >= 2).then(|| format!("{:.17}", standardized[0]));
            let x2 = (index >= 7).then(|| format!("{:.17}", standardized[1]));
            let x3 = (index != 0).then(|| format!("{:.17}", standardized[2]));
            csv.push_str(&format!(
                "{},{},{}\n",
                x1.as_deref().unwrap_or("NA"),
                x2.as_deref().unwrap_or("NA"),
                x3.as_deref().unwrap_or("NA")
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "cbsem-raw-mean-replacement.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        request.model.data_binding = SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: MissingDataPolicyV4::MeanReplacement,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        for variable in &mut request.model.variables {
            let SemVariableV4::Observed {
                source_column,
                missing_markers,
                ..
            } = variable
            else {
                continue;
            };
            *missing_markers = dataset
                .schema
                .columns
                .iter()
                .find(|column| column.name == source_column.as_str())
                .unwrap()
                .missing_markers
                .iter()
                .map(|marker| marker.trim())
                .filter(|marker| !marker.is_empty())
                .map(str::to_owned)
                .collect();
            missing_markers.sort();
            missing_markers.dedup();
        }
        request.model.ensure_valid().unwrap();
        request.dataset_id = dataset.id.to_string();
        request.dataset_fingerprint = dataset.fingerprint.0.clone();
        request.recipe.dataset_fingerprint = dataset.fingerprint.0.clone();
        request.recipe.settings.missing_data = qpls_core::MissingDataPolicy::MeanReplacement;
        request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: request.model.clone(),
            scientific_sha256: request.model.scientific_sha256().unwrap(),
        };
        let Some(MethodConfig::Cbsem {
            input,
            mean_structure,
            ..
        }) = request.recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *input = CbsemInput::Raw;
        *mean_structure = false;
        let mut project = Project::new("CB-SEM Recipe-v4 raw mean-replacement fixture");
        project.datasets.push(dataset);
        (project, request)
    }

    #[test]
    fn internal_cbsem_request_executes_exact_resident_covariance_data() {
        let (project, request) = fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();

        assert!(result.estimation().analysis.converged);
        assert_eq!(result.estimation().analysis.input, "covariance");
        assert_eq!(
            result.provenance().compilation_receipt().capability_cell(),
            &RecipeV4CompilerTarget::CbsemPlanV2.capability_cell()
        );
        assert_eq!(
            result.provenance().adapter_version(),
            qpls_runner::RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V8
        );
        for actual in [
            result.provenance().moment_input_method_version(),
            result.estimation().method_version.as_str(),
        ] {
            assert_eq!(
                actual,
                qpls_estimation::CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3
            );
            assert_ne!(
                actual,
                qpls_estimation::CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V2
            );
        }
        for actual in [
            result.provenance().estimator_method_version(),
            result.estimation().analysis.method_version.as_str(),
        ] {
            assert_eq!(
                actual,
                qpls_estimation::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
            );
        }
        assert_eq!(result.estimation().parameter_ids.len(), 7);
    }

    #[test]
    fn internal_raw_cfa_mean_structure_binds_exact_v4_identity_and_location_results() {
        let (project, request) = mean_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_cbsem(&dataset, &request).unwrap();

        assert!(result.estimation().analysis.converged);
        assert!(result.estimation().analysis.mean_structure);
        assert_eq!(result.estimation().analysis.input, "raw");
        assert_eq!(
            result.provenance().adapter_version(),
            qpls_runner::RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V6
        );
        assert_eq!(
            result.provenance().moment_input_method_version(),
            qpls_estimation::CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4
        );
        assert_eq!(
            result.provenance().estimator_method_version(),
            qpls_estimation::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4
        );
        assert_eq!(
            result.estimation().schema_version,
            qpls_estimation::CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3
        );
        assert_eq!(result.estimation().observed_means.len(), 3);
        assert_eq!(result.estimation().implied_means.len(), 3);
        assert_eq!(result.estimation().residual_means.len(), 3);
        for parameter_id in [
            "parameter:intercept:x1",
            "parameter:intercept:x2",
            "parameter:intercept:x3",
            "parameter:factor_mean:f",
        ] {
            assert!(
                result
                    .estimation()
                    .parameter_ids
                    .values()
                    .any(|value| value == parameter_id),
                "missing stable parameter identity {parameter_id}"
            );
        }
    }

    #[test]
    fn internal_mean_replacement_binds_v4_receipt_progress_and_tamper_failures() {
        let (project, request) = mean_replacement_fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let source_arrow = write_arrow(&dataset.batch).unwrap();
        let source_fingerprint = dataset.fingerprint.clone();
        let progress = Mutex::new(Vec::new());
        let result = execute_internal_recipe_v4_cbsem_with_control(
            &dataset,
            &request,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();

        assert_eq!(
            result.provenance().adapter_version(),
            qpls_runner::RECIPE_V4_CBSEM_EXECUTION_ADAPTER_VERSION_V7
        );
        assert_eq!(
            result.provenance().moment_input_method_version(),
            qpls_estimation::CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1
        );
        assert_eq!(
            result.provenance().estimator_method_version(),
            qpls_estimation::CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        );
        assert_eq!(
            result.estimation().schema_version,
            qpls_estimation::CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4
        );
        let receipt = result
            .estimation()
            .input
            .missing_data_treatment
            .as_ref()
            .unwrap();
        assert_eq!(receipt.source_dataset_id, dataset.id.to_string());
        assert_eq!(receipt.source_dataset_fingerprint, dataset.fingerprint.0);
        assert_eq!(receipt.imputed_cell_count, 10);
        assert_eq!(receipt.affected_case_count, 7);
        assert_eq!(receipt.receipt_sha256.len(), 64);
        let phases = progress
            .lock()
            .unwrap()
            .iter()
            .map(|update| update.phase.clone())
            .collect::<Vec<_>>();
        for phase in ["integrity", "projection", "moments", "estimation", "result"] {
            assert!(phases.iter().any(|actual| actual == phase));
        }
        assert_eq!(write_arrow(&dataset.batch).unwrap(), source_arrow);
        assert_eq!(dataset.fingerprint, source_fingerprint);

        let cancelled =
            execute_internal_recipe_v4_cbsem_with_control(&dataset, &request, || true, |_| {})
                .unwrap_err();
        assert_eq!(cancelled.code, "recipe_v4.cbsem.execution_cancelled");

        let mut policy_tamper = request.clone();
        policy_tamper.recipe.settings.missing_data = qpls_core::MissingDataPolicy::ListwiseDeletion;
        let failure = execute_internal_recipe_v4_cbsem(&dataset, &policy_tamper).unwrap_err();
        assert_eq!(failure.stage, InternalRecipeV4ExecutionStageV1::Compilation);
        assert_eq!(failure.code, "recipe_v4.compilation_failed");

        let mut identity_tamper = request;
        identity_tamper.dataset_fingerprint = format!("v2:{}", "0".repeat(64));
        let failure = execute_internal_recipe_v4_cbsem(&dataset, &identity_tamper).unwrap_err();
        assert_eq!(
            failure.stage,
            InternalRecipeV4ExecutionStageV1::DataResolution
        );
        assert_eq!(
            failure.code,
            "recipe_v4.cbsem.resident_dataset_identity_changed"
        );
        assert_eq!(write_arrow(&dataset.batch).unwrap(), source_arrow);
        assert_eq!(dataset.fingerprint, source_fingerprint);
    }

    #[test]
    fn access_cancellation_and_unsupported_moments_remain_typed() {
        let (project, request) = fixture();
        let dataset = resolve_internal_recipe_v4_cbsem_dataset(&project, &request).unwrap();
        let cancelled =
            execute_internal_recipe_v4_cbsem_with_control(&dataset, &request, || true, |_| {})
                .unwrap_err();
        assert_eq!(cancelled.code, "recipe_v4.cbsem.execution_cancelled");

        let mut standard = request.clone();
        standard.surface = InternalRecipeV4ExecutionSurfaceV1::Standard;
        standard.experimental_labs_enabled = false;
        validate_internal_recipe_v4_cbsem_access(&standard).unwrap();

        let mut mismatched_access = standard.clone();
        mismatched_access.experimental_labs_enabled = true;
        assert_eq!(
            validate_internal_recipe_v4_cbsem_access(&mismatched_access)
                .unwrap_err()
                .stage,
            InternalRecipeV4ExecutionStageV1::Access
        );

        let mut exact_bootstrap = standard;
        exact_bootstrap.recipe.settings.bootstrap_samples = 1_000;
        let Some(MethodConfig::Cbsem {
            bootstrap_samples,
            bootstrap_v2,
            ..
        }) = exact_bootstrap.recipe.method_config.as_mut()
        else {
            unreachable!()
        };
        *bootstrap_samples = 1_000;
        *bootstrap_v2 = Some(CbsemBootstrapConfigV2 {
            algorithm: CbsemBootstrapAlgorithm::CaseResamplingFullMl,
            interval: CbsemBootstrapInterval::BcaType7,
            test_tail: CbsemBootstrapTestTail::TwoSided,
        });
        assert_eq!(
            validate_internal_recipe_v4_cbsem_access(&exact_bootstrap)
                .unwrap_err()
                .stage,
            InternalRecipeV4ExecutionStageV1::Capability
        );
        exact_bootstrap.capability_cell = cbsem_exact_bootstrap_capability_cell_v1();
        validate_internal_recipe_v4_cbsem_access(&exact_bootstrap).unwrap();

        let mut unsupported = request;
        let SemDataBindingV4::Covariance { means, .. } = &mut unsupported.model.data_binding else {
            unreachable!()
        };
        *means = Some(BTreeMap::from([
            ("observed:x1".into(), 0.0),
            ("observed:x2".into(), 0.0),
            ("observed:x3".into(), 0.0),
        ]));
        unsupported.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: unsupported.model.clone(),
            scientific_sha256: unsupported.model.scientific_sha256().unwrap(),
        };
        let failure = execute_internal_recipe_v4_cbsem(&dataset, &unsupported).unwrap_err();
        assert_eq!(failure.code, "recipe_v4.cbsem.unsupported_plan");
        assert!(
            failure
                .issues
                .iter()
                .any(|issue| issue.code == "matrix_means_unsupported")
        );
    }

    #[test]
    fn request_wire_is_strict_and_keeps_the_exact_cbsem_capability_identity() {
        let (_, request) = fixture();
        let mut value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["surface"], "internal_labs");
        assert_eq!(value["compilerTarget"], "cbsem_plan_v2");
        assert_eq!(value["capabilityCell"]["capability_id"], "smartpls.cbsem");
        assert_eq!(value["capabilityCell"]["cell_id"], "qpls3.cbsem.ml");
        assert_eq!(value["capabilityCell"]["capability_version"], "cbsem_ml_v1");
        value["standardAvailable"] = serde_json::json!(true);
        assert!(serde_json::from_value::<InternalRecipeV4CbsemExecutionRequestV1>(value).is_err());
    }
}
