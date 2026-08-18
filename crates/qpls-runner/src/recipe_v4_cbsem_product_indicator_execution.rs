use qpls_core::{
    AnalysisRecipeV4, CBSEM_PRODUCT_INDICATOR_MODERATION_METHOD_VERSION_V1,
    CompiledAnalysisRecipeV4, SemModelV4,
};
use qpls_data::Dataset;
use qpls_estimation::{
    CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1,
    CBSEM_PRODUCT_INDICATOR_TRANSFORMATION_VERSION_V1, CbsemProductIndicatorExecutionErrorV1,
    CbsemProductIndicatorModerationResultV1,
    estimate_internal_cbsem_product_indicator_moderation_v1_with_control,
};
use serde::{Deserialize, Serialize};

use crate::RunnerProgress;

pub const INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_REQUEST_SCHEMA_VERSION_V1: u32 = 1;
pub const INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_ADAPTER_VERSION_V1: &str =
    "internal_cbsem_product_indicator_execution_v1";
pub const INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_SURFACE: &str = "internal";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InternalLabsCbsemProductIndicatorRequestV1 {
    schema_version: u32,
    surface: String,
}

impl InternalLabsCbsemProductIndicatorRequestV1 {
    pub fn exact_internal() -> Self {
        Self {
            schema_version: INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_REQUEST_SCHEMA_VERSION_V1,
            surface: INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_SURFACE.into(),
        }
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn surface(&self) -> &str {
        &self.surface
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InternalLabsCbsemProductIndicatorExecutionProvenanceV1 {
    adapter_version: String,
    surface: String,
    dataset_id: String,
    method_version: String,
    transformation_version: String,
}

impl InternalLabsCbsemProductIndicatorExecutionProvenanceV1 {
    pub fn adapter_version(&self) -> &str {
        &self.adapter_version
    }

    pub fn surface(&self) -> &str {
        &self.surface
    }

    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub fn method_version(&self) -> &str {
        &self.method_version
    }

    pub fn transformation_version(&self) -> &str {
        &self.transformation_version
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InternalLabsCbsemProductIndicatorExecutionResultV1 {
    schema_version: u32,
    provenance: InternalLabsCbsemProductIndicatorExecutionProvenanceV1,
    moderation: CbsemProductIndicatorModerationResultV1,
}

impl InternalLabsCbsemProductIndicatorExecutionResultV1 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn provenance(&self) -> &InternalLabsCbsemProductIndicatorExecutionProvenanceV1 {
        &self.provenance
    }

    pub fn moderation(&self) -> &CbsemProductIndicatorModerationResultV1 {
        &self.moderation
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InternalLabsCbsemProductIndicatorExecutionErrorV1 {
    #[error("CB-SEM product-indicator moderation was cancelled")]
    Cancelled,
    #[error("Internal-only product-indicator request schema changed from {expected} to {actual}")]
    RequestSchemaMismatch { expected: u32, actual: u32 },
    #[error("CB-SEM product-indicator moderation is restricted to the Internal surface")]
    InternalLabsSurfaceRequired,
    #[error("CB-SEM product-indicator execution failed: {0}")]
    Execution(CbsemProductIndicatorExecutionErrorV1),
    #[error("product-indicator result schema changed from {expected} to {actual}")]
    ResultSchemaMismatch { expected: u32, actual: u32 },
    #[error("product-indicator method version changed from {expected} to {actual}")]
    MethodVersionMismatch {
        expected: &'static str,
        actual: String,
    },
    #[error("product-indicator transformation version changed from {expected} to {actual}")]
    TransformationVersionMismatch {
        expected: &'static str,
        actual: String,
    },
}

/// Executes the explicitly Internal-only CB-SEM product-indicator path.
///
/// The generic CB-SEM Recipe-v4 runner remains unchanged and continues to
/// reject derived terms. This additive boundary revalidates the exact request,
/// forwards cancellation and progress to materialization and optimization, and
/// returns no payload if cancellation wins at any boundary.
pub fn run_internal_cbsem_product_indicator_moderation_v1(
    request: &InternalLabsCbsemProductIndicatorRequestV1,
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    artifact: &CompiledAnalysisRecipeV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(RunnerProgress) + Sync,
) -> Result<
    InternalLabsCbsemProductIndicatorExecutionResultV1,
    InternalLabsCbsemProductIndicatorExecutionErrorV1,
> {
    validate_request(request)?;
    if should_cancel() {
        return Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::Cancelled);
    }
    let moderation = estimate_internal_cbsem_product_indicator_moderation_v1_with_control(
        dataset,
        artifact,
        recipe,
        resolved_model,
        &should_cancel,
        |update| {
            progress(RunnerProgress {
                phase: update.phase,
                completed_units: update.completed_units,
                total_units: update.total_units,
            });
        },
    )
    .map_err(map_execution_error)?;
    if moderation.schema_version != CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1 {
        return Err(
            InternalLabsCbsemProductIndicatorExecutionErrorV1::ResultSchemaMismatch {
                expected: CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1,
                actual: moderation.schema_version,
            },
        );
    }
    if moderation.method_version != CBSEM_PRODUCT_INDICATOR_MODERATION_METHOD_VERSION_V1 {
        return Err(
            InternalLabsCbsemProductIndicatorExecutionErrorV1::MethodVersionMismatch {
                expected: CBSEM_PRODUCT_INDICATOR_MODERATION_METHOD_VERSION_V1,
                actual: moderation.method_version,
            },
        );
    }
    let transformation_version = &moderation.provenance.transformation.transformation_version;
    if transformation_version != CBSEM_PRODUCT_INDICATOR_TRANSFORMATION_VERSION_V1 {
        return Err(
            InternalLabsCbsemProductIndicatorExecutionErrorV1::TransformationVersionMismatch {
                expected: CBSEM_PRODUCT_INDICATOR_TRANSFORMATION_VERSION_V1,
                actual: transformation_version.clone(),
            },
        );
    }
    if should_cancel() {
        return Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::Cancelled);
    }
    Ok(InternalLabsCbsemProductIndicatorExecutionResultV1 {
        schema_version: INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1,
        provenance: InternalLabsCbsemProductIndicatorExecutionProvenanceV1 {
            adapter_version: INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_ADAPTER_VERSION_V1.into(),
            surface: INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_SURFACE.into(),
            dataset_id: dataset.id.to_string(),
            method_version: moderation.method_version.clone(),
            transformation_version: transformation_version.clone(),
        },
        moderation,
    })
}

fn validate_request(
    request: &InternalLabsCbsemProductIndicatorRequestV1,
) -> Result<(), InternalLabsCbsemProductIndicatorExecutionErrorV1> {
    if request.schema_version != INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_REQUEST_SCHEMA_VERSION_V1 {
        return Err(
            InternalLabsCbsemProductIndicatorExecutionErrorV1::RequestSchemaMismatch {
                expected: INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_REQUEST_SCHEMA_VERSION_V1,
                actual: request.schema_version,
            },
        );
    }
    if request.surface != INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_SURFACE {
        return Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::InternalLabsSurfaceRequired);
    }
    Ok(())
}

fn map_execution_error(
    error: CbsemProductIndicatorExecutionErrorV1,
) -> InternalLabsCbsemProductIndicatorExecutionErrorV1 {
    match error {
        CbsemProductIndicatorExecutionErrorV1::Cancelled => {
            InternalLabsCbsemProductIndicatorExecutionErrorV1::Cancelled
        }
        other => InternalLabsCbsemProductIndicatorExecutionErrorV1::Execution(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipeModelBindingV4,
        AnalysisSettings, CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1,
        CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1, CbsemEstimator, CbsemInput, CbsemModelType,
        Construct, InteractionMethodV4, LegacyBasicModelInterpretationV4,
        LegacyEstimandConfirmationV4, MeasurementMode, MethodConfig, ModelSpec, Preprocessing,
        ProductIndicatorCenteringV4, ProductIndicatorPairingV4, ProductIndicatorSpecificationV4,
        ProductIndicatorStandardizationV4, RecipeV4CompilerTarget, SemDerivedTermV4, SemEndpointV4,
        SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath,
        StructuralRelationRoleV4, compile_analysis_recipe_v4, convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::{collections::BTreeMap, sync::Mutex};
    use uuid::Uuid;

    fn runner_fixture() -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let mut csv = String::from("x1,x2,m1,m2,y1,y2,y3\n");
        for index in 0..180 {
            let a = ((index * 37 + 11) % 101) as f64 / 50.0 - 1.0;
            let b = ((index * 53 + 17) % 103) as f64 / 51.0 - 1.0;
            let c = ((index * 29 + 7) % 97) as f64 / 48.0 - 1.0;
            let errors = [
                ((index * 19 + 3) % 89) as f64 / 44.0 - 1.0,
                ((index * 23 + 5) % 83) as f64 / 41.0 - 1.0,
                ((index * 31 + 9) % 79) as f64 / 39.0 - 1.0,
                ((index * 17 + 13) % 73) as f64 / 36.0 - 1.0,
                ((index * 27 + 15) % 71) as f64 / 35.0 - 1.0,
                ((index * 11 + 19) % 67) as f64 / 33.0 - 1.0,
                ((index * 7 + 21) % 61) as f64 / 30.0 - 1.0,
            ];
            let predictor = 1.5 * a + 0.18 * b;
            let moderator = 1.5 * b + 0.12 * a;
            let outcome =
                0.45 * predictor + 0.30 * moderator + 0.35 * predictor * moderator + 0.90 * c;
            let row = [
                predictor + 1.1 * errors[0],
                0.82 * predictor + 1.1 * errors[1],
                moderator + 1.1 * errors[2],
                0.86 * moderator - 1.1 * errors[3],
                outcome + 1.1 * errors[4],
                0.83 * outcome + 1.1 * errors[5],
                0.68 * outcome - 1.1 * errors[6],
            ];
            csv.push_str(&format!(
                "{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",
                row[0], row[1], row[2], row[3], row[4], row[5], row[6]
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "cbsem-product-indicator-runner-fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_5200),
            name: "CB-SEM product indicator runner fixture".into(),
            constructs: vec![
                construct("x", &["x1", "x2"]),
                construct("m", &["m1", "m2"]),
                construct("y", &["y1", "y2", "y3"]),
            ],
            paths: vec![path("x", "y"), path("m", "y")],
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
        model.data_binding = qpls_core::SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: qpls_core::MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        let covariance_left = SemEndpointV4::Variable("construct:m".into());
        let covariance_right = SemEndpointV4::Variable("construct:x".into());
        model.relations.push(SemRelationV4::Covariance {
            id: "relation:covariance:m:x".into(),
            left: covariance_left.clone(),
            right: covariance_right.clone(),
            parameter: "parameter:covariance:m:x".into(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:covariance:m:x".into(),
            label: "Cov(M,X)".into(),
            target: SemParameterTargetV4::Covariance {
                left: covariance_left,
                right: covariance_right,
            },
            start: Some(0.1),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.variables.push(SemVariableV4::Derived {
            id: "derived:x_by_m".into(),
            label: "X × M".into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "relation:interaction_effect".into(),
            source: "derived:x_by_m".into(),
            target: "construct:y".into(),
            parameter: "parameter:interaction_effect".into(),
            intercept_parameter: None,
            role: StructuralRelationRoleV4::Structural,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:interaction_effect".into(),
            label: "X × M -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x_by_m".into(),
                target: "construct:y".into(),
            },
            start: Some(0.3),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        model.derived_terms.push(SemDerivedTermV4::Interaction {
            id: "term:x_by_m".into(),
            output: "derived:x_by_m".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation,
            method: InteractionMethodV4::ProductIndicator,
            product_indicator: Some(ProductIndicatorSpecificationV4 {
                centering: ProductIndicatorCenteringV4::DoubleMeanCenter,
                standardization: ProductIndicatorStandardizationV4::None,
                pairing: ProductIndicatorPairingV4::AllPairs,
            }),
        });
        model.ensure_valid().unwrap();

        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.max_iterations = CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1;
        settings.tolerance = CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1;
        settings.workers = 1;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0xCB5E_5201),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                scientific_sha256: model.scientific_sha256().unwrap(),
                model: model.clone(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        let target = RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn construct(id: &str, indicators: &[&str]) -> Construct {
        Construct {
            id: id.into(),
            name: id.to_uppercase(),
            short_name: id.to_uppercase(),
            mode: MeasurementMode::Reflective,
            indicators: indicators.iter().map(|value| (*value).into()).collect(),
        }
    }

    fn path(source: &str, target: &str) -> StructuralPath {
        StructuralPath {
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn exact_request_is_internal_only_and_versioned() {
        let request = InternalLabsCbsemProductIndicatorRequestV1::exact_internal();
        assert_eq!(
            request.schema_version(),
            INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_REQUEST_SCHEMA_VERSION_V1
        );
        assert_eq!(
            request.surface(),
            INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_SURFACE
        );
        validate_request(&request).unwrap();
    }

    #[test]
    fn tampered_request_fails_closed_before_execution() {
        let mut wrong_schema = InternalLabsCbsemProductIndicatorRequestV1::exact_internal();
        wrong_schema.schema_version += 1;
        assert!(matches!(
            validate_request(&wrong_schema),
            Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::RequestSchemaMismatch { .. })
        ));

        let mut standard = InternalLabsCbsemProductIndicatorRequestV1::exact_internal();
        standard.surface = "standard".into();
        assert!(matches!(
            validate_request(&standard),
            Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::InternalLabsSurfaceRequired)
        ));

        let mut labs = InternalLabsCbsemProductIndicatorRequestV1::exact_internal();
        labs.surface = "labs".into();
        assert!(matches!(
            validate_request(&labs),
            Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::InternalLabsSurfaceRequired)
        ));
    }

    #[test]
    fn estimator_cancellation_maps_to_runner_cancellation() {
        assert!(matches!(
            map_execution_error(CbsemProductIndicatorExecutionErrorV1::Cancelled),
            InternalLabsCbsemProductIndicatorExecutionErrorV1::Cancelled
        ));
    }

    #[test]
    fn real_runner_executes_exact_product_artifact_with_distinct_source_and_inner_receipts() {
        let (dataset, recipe, model, artifact) = runner_fixture();
        let progress = Mutex::new(Vec::<RunnerProgress>::new());
        let result = run_internal_cbsem_product_indicator_moderation_v1(
            &InternalLabsCbsemProductIndicatorRequestV1::exact_internal(),
            &dataset,
            &recipe,
            &model,
            &artifact,
            || false,
            |update| progress.lock().unwrap().push(update),
        )
        .unwrap();
        assert_eq!(
            result.provenance().surface(),
            INTERNAL_LABS_CBSEM_PRODUCT_INDICATOR_SURFACE
        );
        assert_eq!(
            result.provenance().method_version(),
            CBSEM_PRODUCT_INDICATOR_MODERATION_METHOD_VERSION_V1
        );
        assert_eq!(
            result.provenance().transformation_version(),
            CBSEM_PRODUCT_INDICATOR_TRANSFORMATION_VERSION_V1
        );
        let moderation = result.moderation();
        assert!(moderation.estimation.analysis.converged);
        assert_eq!(
            moderation
                .provenance
                .source_compilation_receipt
                .compiler_target(),
            RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1
        );
        assert_eq!(
            moderation
                .provenance
                .source_compilation_receipt
                .capability_cell(),
            &RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1.capability_cell()
        );
        assert_eq!(
            moderation
                .provenance
                .expanded_compilation_receipt
                .compiler_target(),
            RecipeV4CompilerTarget::CbsemPlanV2
        );
        assert_eq!(
            moderation
                .provenance
                .expanded_compilation_receipt
                .capability_cell(),
            &RecipeV4CompilerTarget::CbsemPlanV2.capability_cell()
        );
        assert_ne!(
            moderation
                .provenance
                .source_compilation_receipt
                .plan_sha256(),
            moderation
                .provenance
                .expanded_compilation_receipt
                .plan_sha256()
        );
        assert_eq!(
            moderation.compiled_product_plan.product_indicators().len(),
            4
        );
        assert_eq!(moderation.provenance.transformation.used_observations, 180);
        assert!(
            moderation
                .output_exclusions
                .iter()
                .any(|exclusion| exclusion.code == "normal_theory_inference_unqualified")
        );

        // Frozen independently by validation/cbsem_product_indicator_moderation_v1_oracle.py.
        let oracle_product_statistics = [
            (0.09669090380507826, 1.1914675909281927, 1.4117083812309417),
            (0.1639133903133054, 1.059829130567554, 1.1169975760773572),
            (0.16434632418680892, 0.9937091741342706, 0.9819720454099558),
            (0.11205729684898048, 0.9722945030077327, 0.9401046194647259),
        ];
        for ((product, statistic), (oracle_pre_mean, oracle_sample_sd, oracle_ml_variance)) in
            moderation
                .compiled_product_plan
                .product_indicators()
                .iter()
                .zip(&moderation.provenance.transformation.product_statistics)
                .zip(oracle_product_statistics)
        {
            assert!((statistic.mean_before_second_centering - oracle_pre_mean).abs() <= 2e-14);
            assert!((statistic.final_sample_standard_deviation - oracle_sample_sd).abs() <= 2e-14);
            let covariance_position = moderation
                .estimation
                .input
                .source_columns
                .iter()
                .position(|column| column == product.source_column())
                .expect("every materialized product must be present in the consumed covariance");
            assert!(
                (moderation.estimation.covariance_ml[covariance_position][covariance_position]
                    - oracle_ml_variance)
                    .abs()
                    <= 2e-14
            );
        }

        let (interaction_name, _) = moderation
            .estimation
            .parameter_ids
            .iter()
            .find(|(_, parameter_id)| parameter_id.as_str() == "parameter:interaction_effect")
            .expect("stable interaction parameter id must be projected");
        let interaction = moderation
            .estimation
            .analysis
            .parameters
            .iter()
            .find(|parameter| parameter.name == interaction_name.as_str())
            .expect("interaction estimate must use the projected exact-engine name");
        assert!(
            (interaction.estimate - 0.23008951).abs() <= 5e-5,
            "Rust interaction estimate {} drifted from the frozen independent NumPy/SciPy oracle",
            interaction.estimate
        );

        let phases = progress
            .into_inner()
            .unwrap()
            .into_iter()
            .map(|update| update.phase)
            .collect::<Vec<_>>();
        assert!(phases.iter().any(|phase| phase == "product_rows"));
        assert!(phases.iter().any(|phase| phase.starts_with("estimation:")));
        assert!(phases.iter().any(|phase| phase == "result"));
    }

    #[test]
    fn real_runner_cancellation_returns_no_partial_result() {
        let (dataset, recipe, model, artifact) = runner_fixture();
        assert!(matches!(
            run_internal_cbsem_product_indicator_moderation_v1(
                &InternalLabsCbsemProductIndicatorRequestV1::exact_internal(),
                &dataset,
                &recipe,
                &model,
                &artifact,
                || true,
                |_| {},
            ),
            Err(InternalLabsCbsemProductIndicatorExecutionErrorV1::Cancelled)
        ));
    }
}
