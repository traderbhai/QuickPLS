use crate::{
    CompiledCbsemExecutionDispositionV3, CompiledCbsemStructuralFormV3, CompiledPlsPlanV3Error,
    GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemInferenceTailV1,
    GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1, MissingDataPolicyV4,
    ObservedScaleV4, SemCapabilityCellIdV1, SemCapabilityDecisionStatusV1, SemCapabilityDecisionV1,
    SemCapabilityDecisionV1ValidationError, SemCapabilityDiagnosticSeverityV1,
    SemCapabilityDiagnosticV1, SemCapabilityEvidenceV1, SemDataBindingV4, SemGroupV4, SemModelV4,
    SemVariableV4, compile_cbsem_plan_v3, compile_pls_plan_v3,
};

pub const GENERAL_SEM_PLS_ESTIMATOR_ID_V1: &str = "qpls.pls_sem.v3";
pub const GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1: &str = "qpls.cbsem.v3";

/// Exact, recovery-oriented preflight for the General SEM PLS point-estimation
/// and bounded percentile case-bootstrap compiler slices. Model semantics are
/// inspected but never changed.
pub fn preflight_general_sem_pls_v1(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<SemCapabilityDecisionV1, SemCapabilityDecisionV1ValidationError> {
    let capability_cells = pls_cells(config)?;
    let mut evidence = vec![
        SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.mediation:qpls3.pls.mediation:pls_mediation_v1",
            "Capability Registry V2 exposes the exact mediation option in Experimental Labs.",
        )?,
        SemCapabilityEvidenceV1::new(
            "compiler:recipe_v4_to_compiled_pls_plan_v3_v1",
            "The versioned PLS v3 compiler preserves the proven v2 scoring plan and adds stable topology and effect identities.",
        )?,
    ];
    if matches!(
        config.inference,
        GeneralSemInferenceV1::CaseBootstrap { .. }
    ) {
        evidence.push(SemCapabilityEvidenceV1::new(
            "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
            "The bootstrap compiler binds exact Recipe V4 inference settings to the General SEM config while retaining the proven point-scoring plan.",
        )?);
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.mediation:qpls3.pls.general_sem_multiple_mediation_bootstrap:general_sem_pls_full_model_case_bootstrap_v1",
            "Capability Registry V2 exposes this exact multiple-mediation, full-model percentile case-bootstrap combination in Experimental Labs.",
        )?);
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_dependency:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
            "The exact General SEM cell uses the separately governed indexed case-resampling mechanism without inheriting that mechanism cell's release maturity.",
        )?);
    }
    let mut diagnostics = execution_scope_diagnostics(model, config)?;
    match compile_pls_plan_v3(model, config) {
        Ok(plan) => {
            let found = plan.topology().specific_directed_paths().len();
            match config.inference {
                GeneralSemInferenceV1::None if found == 0 => {
                    diagnostics.push(SemCapabilityDiagnosticV1::new(
                        "sem.capability.pls.mediation_requires_indirect_path",
                        SemCapabilityDiagnosticSeverityV1::Error,
                        None,
                        "The PLS mediation point cell requires at least one compiled specific indirect path; this graph has none.",
                        vec![
                            "Add a supported mediator path, or use the existing ordinary PLS workflow for a direct-only recursive model.".into(),
                        ],
                    )?);
                }
                GeneralSemInferenceV1::CaseBootstrap { .. } if found < 2 => {
                    diagnostics.push(SemCapabilityDiagnosticV1::new(
                        "sem.capability.pls.multiple_mediation_requires_two_indirect_paths",
                        SemCapabilityDiagnosticSeverityV1::Error,
                        None,
                        format!(
                            "The exact multiple-mediation bootstrap cell requires at least two compiled specific indirect paths; this graph has {found}."
                        ),
                        vec![
                            "Add a second supported parallel or serial mediation path, or use point inference under the mediation cell until a single-mediation bootstrap cell is separately governed.".into(),
                        ],
                    )?);
                }
                _ => {}
            }
        }
        Err(error) => diagnostics.push(pls_compile_diagnostic(error)?),
    }
    if !diagnostics.is_empty() {
        return SemCapabilityDecisionV1::new(
            SemCapabilityDecisionStatusV1::Blocked,
            GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
            capability_cells,
            diagnostics,
            evidence,
            "PLS-SEM cannot calculate this exact General SEM request yet.",
            "The authored model remains intact. Apply one of the listed corrections or select an estimator whose exact capability cell supports the graph.",
        );
    }
    SemCapabilityDecisionV1::new(
        SemCapabilityDecisionStatusV1::Experimental,
        GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
        capability_cells,
        vec![SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.experimental_labs",
            SemCapabilityDiagnosticSeverityV1::Info,
            None,
            match config.inference {
                GeneralSemInferenceV1::None => {
                    "General recursive PLS point estimation and path-specific effects pass the Experimental Labs compiler preflight."
                }
                GeneralSemInferenceV1::CaseBootstrap { .. } => {
                    "General recursive PLS percentile case-bootstrap inference passes the bounded Experimental Labs compiler preflight."
                }
            },
            Vec::new(),
        )?],
        evidence,
        "PLS-SEM can compile this exact request in Experimental Labs.",
        match config.inference {
            GeneralSemInferenceV1::None => {
                "The compiler binds the proven PLS scoring plan to stable relation-path identities. Runtime validation remains authoritative before a result can be published."
            }
            GeneralSemInferenceV1::CaseBootstrap { .. } => {
                "The compiler binds percentile, two-sided case resampling to the exact multiple-mediation bootstrap cell and records the indexed-resampling mechanism as a dependency. Runtime inference must carry a matching complete-model re-estimation receipt before publication."
            }
        },
    )
}

/// CB-SEM v3 currently compiles a complete parameter table and conservative
/// identification evidence, but its General SEM runtime adapter is not yet
/// connected. The decision therefore remains blocked even for recursive plans.
pub fn preflight_general_sem_cbsem_v1(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<SemCapabilityDecisionV1, SemCapabilityDecisionV1ValidationError> {
    let cell = cbsem_cell()?;
    let evidence = vec![
        SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.cbsem:qpls3.cbsem.ml:cbsem_ml_v1",
            "Capability Registry V2 is the exact authority for the bounded CB-SEM ML cell.",
        )?,
        SemCapabilityEvidenceV1::new(
            "compiler:compiled_cbsem_plan_v3",
            "CB-SEM v3 preserves the complete v2 parameter table and adds SCC and identification evidence without implying execution support.",
        )?,
    ];
    let mut diagnostics = Vec::new();
    match compile_cbsem_plan_v3(model, config) {
        Ok(plan)
            if plan.identification_evidence().structural_form()
                == CompiledCbsemStructuralFormV3::Feedback
                || plan.identification_evidence().execution_disposition()
                    == CompiledCbsemExecutionDispositionV3::FeedbackExecutionBlocked =>
        {
            diagnostics.push(SemCapabilityDiagnosticV1::new(
                "sem.capability.cbsem.feedback_execution_blocked",
                SemCapabilityDiagnosticSeverityV1::Error,
                None,
                "The reciprocal block is preserved, but the current CB-SEM executor is not qualified to estimate feedback systems.",
                vec![
                    "Remove the reciprocal path to create a recursive model, or retain the model until the identified feedback capability is qualified.".into(),
                ],
            )?);
        }
        Ok(_) => diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.cbsem.general_runtime_not_connected",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "The CB-SEM v3 parameter and identification plan is available, but the General SEM runtime adapter is not connected.",
            vec![
                "Use the currently qualified bounded CB-SEM workflow, or keep this request in Labs until the v3 adapter is qualified.".into(),
            ],
        )?),
        Err(error) => diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.cbsem.compile_blocked",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            format!("CB-SEM cannot compile this exact graph: {error}"),
            vec![
                "Open the estimator compatibility inspector and resolve the reported model or identification issue.".into(),
            ],
        )?),
    }
    SemCapabilityDecisionV1::new(
        SemCapabilityDecisionStatusV1::Blocked,
        GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
        vec![cell],
        diagnostics,
        evidence,
        "CB-SEM cannot calculate this exact General SEM request yet.",
        "Compilation and identification diagnostics remain visible, while execution stays disabled until the exact runtime cell is qualified.",
    )
}

fn execution_scope_diagnostics(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<Vec<SemCapabilityDiagnosticV1>, SemCapabilityDecisionV1ValidationError> {
    let mut diagnostics = Vec::new();
    match &model.data_binding {
        SemDataBindingV4::Raw {
            missing_data,
            weight,
            cluster_variable,
            strata_variable,
            ..
        } => {
            if *missing_data != MissingDataPolicyV4::ListwiseDeletion {
                diagnostics.push(SemCapabilityDiagnosticV1::new(
                    "sem.capability.pls.listwise_deletion_required",
                    SemCapabilityDiagnosticSeverityV1::Error,
                    Some(model.id.clone()),
                    "The exact General SEM PLS cell requires listwise deletion; the authored missing-data policy is preserved but unsupported.",
                    vec!["Select listwise deletion explicitly for this PLS request.".into()],
                )?);
            }
            if weight.is_some() || cluster_variable.is_some() || strata_variable.is_some() {
                diagnostics.push(SemCapabilityDiagnosticV1::new(
                    "sem.capability.pls.complex_sampling_not_executable",
                    SemCapabilityDiagnosticSeverityV1::Error,
                    Some(model.id.clone()),
                    "Weights, cluster variables, and strata variables are not executable in this exact General SEM PLS cell.",
                    vec!["Use an unweighted single-level request, or retain these semantics for a future qualified cell.".into()],
                )?);
            }
        }
        SemDataBindingV4::Covariance { .. } | SemDataBindingV4::Correlation { .. } => {
            diagnostics.push(SemCapabilityDiagnosticV1::new(
                "sem.capability.pls.raw_data_required",
                SemCapabilityDiagnosticSeverityV1::Error,
                Some(model.id.clone()),
                "The exact General SEM PLS cell requires raw case-level data.",
                vec![
                    "Choose a raw resident dataset, or use a qualified matrix-input CB-SEM cell."
                        .into(),
                ],
            )?);
        }
    }
    if model.group != SemGroupV4::SingleGroup {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.single_group_required",
            SemCapabilityDiagnosticSeverityV1::Error,
            Some(model.id.clone()),
            "The exact General SEM PLS cell currently executes single-group models only.",
            vec!["Select the single-group definition, or retain the group semantics for a future qualified multi-group cell.".into()],
        )?);
    }
    for variable in &model.variables {
        let SemVariableV4::Observed {
            id,
            scale,
            missing_markers,
            transformation_lineage,
            ..
        } = variable
        else {
            continue;
        };
        if *scale != ObservedScaleV4::Continuous
            || !missing_markers.is_empty()
            || !transformation_lineage.is_empty()
        {
            diagnostics.push(SemCapabilityDiagnosticV1::new(
                "sem.capability.pls.observed_semantics_not_executable",
                SemCapabilityDiagnosticSeverityV1::Error,
                Some(id.clone()),
                "This observed variable carries scale, missing-marker, or transformation semantics outside the exact General SEM PLS cell.",
                vec!["Keep the authored semantics unchanged and use an explicit, lineage-recorded dataset transformation or a future qualified cell.".into()],
            )?);
        }
    }
    if !config.conditional_effect_probes.is_empty() {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.conditional_probes_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "Conditional-effect probes are authored but are not executable in the current PLS v3 point-estimation slice.",
            vec![
                "Remove the probe request for point estimation, or wait for the qualified moderation execution cell.".into(),
            ],
        )?);
    }
    if let GeneralSemInferenceV1::CaseBootstrap { interval, tail, .. } = config.inference {
        if interval != GeneralSemBootstrapIntervalV1::Percentile {
            diagnostics.push(SemCapabilityDiagnosticV1::new(
                "sem.capability.pls.general_bootstrap_bca_not_executable",
                SemCapabilityDiagnosticSeverityV1::Error,
                None,
                "BCa intervals are represented in the General SEM contract but are not qualified for this execution slice.",
                vec![
                    "Choose percentile intervals with two-sided inference, or set inference to none until the full General SEM delete-one effect ledger is qualified.".into(),
                ],
            )?);
        }
        if tail != GeneralSemInferenceTailV1::TwoSided {
            diagnostics.push(SemCapabilityDiagnosticV1::new(
                "sem.capability.pls.general_bootstrap_one_sided_not_executable",
                SemCapabilityDiagnosticSeverityV1::Error,
                None,
                "One-sided General SEM bootstrap intervals are represented but their interval semantics are not yet qualified.",
                vec![
                    "Choose two-sided inference, or set inference to none until the directional interval contract is qualified.".into(),
                ],
            )?);
        }
    }
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.lazy_path_materialization_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "Lazy specific-path materialization is requested but is not implemented by the current executor.",
            vec![
                "Use bounded eager materialization with an explicit path limit, or reduce the model before calculation.".into(),
            ],
        )?);
    }
    Ok(diagnostics)
}

fn pls_compile_diagnostic(
    error: CompiledPlsPlanV3Error,
) -> Result<SemCapabilityDiagnosticV1, SemCapabilityDecisionV1ValidationError> {
    let (code, correction) = match &error {
        CompiledPlsPlanV3Error::StructuralFeedback => (
            "sem.capability.pls.feedback_blocked",
            "Remove the reciprocal path for PLS-SEM, or use a future qualified nonrecursive CB-SEM cell.",
        ),
        CompiledPlsPlanV3Error::UnknownSpecificIndirectPath { .. } => (
            "sem.capability.pls.requested_path_missing",
            "Re-open the mediation inspector and select an exact directed relation path that exists in the current model.",
        ),
        CompiledPlsPlanV3Error::UnreachableEffect { .. } => (
            "sem.capability.pls.requested_effect_unreachable",
            "Choose endpoints connected by a supported directed path, or add the intended structural relation on the canvas.",
        ),
        CompiledPlsPlanV3Error::AggregateEstimandIdCollidesWithSpecificPathIdentity { .. } => (
            "sem.capability.pls.effect_identity_collision",
            "Choose an aggregate estimand id that does not use a reserved sem_specific_path_v1 identity.",
        ),
        CompiledPlsPlanV3Error::LazySpecificPathMaterializationNotImplemented => (
            "sem.capability.pls.lazy_path_materialization_not_executable",
            "Use bounded eager materialization with an explicit path limit, or reduce the model before calculation.",
        ),
        CompiledPlsPlanV3Error::BasePlan(_) => (
            "sem.capability.pls.model_shape_not_executable",
            "Review generated terms and construct types in the estimator compatibility inspector; unsupported semantics will remain saved.",
        ),
        CompiledPlsPlanV3Error::Topology(_) => (
            "sem.capability.pls.topology_not_compilable",
            "Resolve the reported path-limit or topology issue without deleting unsupported semantics silently.",
        ),
        CompiledPlsPlanV3Error::InvalidGeneralSemConfig(_) => (
            "sem.capability.pls.general_config_invalid",
            "Correct the General SEM request settings and retry capability preflight.",
        ),
    };
    SemCapabilityDiagnosticV1::new(
        code,
        SemCapabilityDiagnosticSeverityV1::Error,
        None,
        format!("PLS-SEM cannot compile this exact request: {error}"),
        vec![correction.into()],
    )
}

fn pls_cell() -> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    SemCapabilityCellIdV1::new(
        2,
        "smartpls.mediation",
        "qpls3.pls.mediation",
        "pls_mediation_v1",
    )
}

fn pls_bootstrap_cell() -> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    SemCapabilityCellIdV1::new(
        2,
        "smartpls.mediation",
        "qpls3.pls.general_sem_multiple_mediation_bootstrap",
        "general_sem_pls_full_model_case_bootstrap_v1",
    )
}

fn pls_cells(
    config: &GeneralSemConfigV1,
) -> Result<Vec<SemCapabilityCellIdV1>, SemCapabilityDecisionV1ValidationError> {
    let mut cells = vec![pls_cell()?];
    if matches!(
        config.inference,
        GeneralSemInferenceV1::CaseBootstrap { .. }
    ) {
        cells.push(pls_bootstrap_cell()?);
    }
    Ok(cells)
}

fn cbsem_cell() -> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    SemCapabilityCellIdV1::new(2, "smartpls.cbsem", "qpls3.cbsem.ml", "cbsem_ml_v1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, ObservedRoleV4,
        ObservedTransformationOperationV4, ObservedTransformationStepV4, SemWeightBindingV4,
        StructuralPath, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn recursive_model() -> SemModelV4 {
        let constructs = ["x", "m", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = [("x", "m"), ("m", "y"), ("x", "y")]
            .into_iter()
            .map(|(source, target)| StructuralPath {
                source: source.into(),
                target: target.into(),
            })
            .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x5031_5310),
                name: "Recursive preflight".into(),
                constructs,
                paths,
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn direct_only_model() -> SemModelV4 {
        let constructs = ["x", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x5031_5312),
                name: "Direct-only preflight".into(),
                constructs,
                paths: vec![StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                }],
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn multiple_mediation_model() -> SemModelV4 {
        let constructs = ["x", "m1", "m2", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = [
            ("x", "m1"),
            ("x", "m2"),
            ("x", "y"),
            ("m1", "m2"),
            ("m1", "y"),
            ("m2", "y"),
        ]
        .into_iter()
        .map(|(source, target)| StructuralPath {
            source: source.into(),
            target: target.into(),
        })
        .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x5031_5311),
                name: "Multiple-mediation preflight".into(),
                constructs,
                paths,
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap()
    }

    fn add_sampling_control(model: &mut SemModelV4) {
        model.variables.push(SemVariableV4::Observed {
            id: "observed:sampling_control".into(),
            label: "Sampling control".into(),
            source_column: "sampling_control".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: Default::default(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
    }

    #[test]
    fn recursive_pls_is_experimental_and_feedback_is_blocked_with_correction() {
        let model = recursive_model();
        let decision =
            preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(
            decision.status(),
            SemCapabilityDecisionStatusV1::Experimental
        );
        assert_eq!(decision.status_label(), "Experimental");

        let mut feedback = model;
        let relation = feedback
            .relations
            .iter()
            .find_map(|relation| match relation {
                crate::SemRelationV4::Structural { source, target, .. }
                    if source == "construct:x" && target == "construct:m" =>
                {
                    Some((source.clone(), target.clone()))
                }
                _ => None,
            })
            .unwrap();
        let parameter_id = "parameter:feedback".to_string();
        feedback.relations.push(crate::SemRelationV4::Structural {
            id: "relation:feedback".into(),
            source: relation.1.clone(),
            target: relation.0.clone(),
            parameter: parameter_id.clone(),
            role: crate::StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        feedback.parameters.push(crate::SemParameterV4::Free {
            id: parameter_id,
            label: "M to X".into(),
            target: crate::SemParameterTargetV4::Regression {
                source: relation.1,
                target: relation.0,
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        feedback.ensure_valid().unwrap();
        let decision =
            preflight_general_sem_pls_v1(&feedback, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.feedback_blocked"
                && !diagnostic.corrections().is_empty()
        }));
    }

    #[test]
    fn direct_only_graph_is_not_mislabelled_as_the_mediation_point_cell() {
        let decision =
            preflight_general_sem_pls_v1(&direct_only_model(), &GeneralSemConfigV1::default())
                .unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.mediation_requires_indirect_path"
                && !diagnostic.corrections().is_empty()
        }));
    }

    #[test]
    fn authored_missing_markers_and_transformation_lineage_are_blocked_without_mutation() {
        let mut missing_marker_model = recursive_model();
        let SemVariableV4::Observed {
            missing_markers, ..
        } = missing_marker_model
            .variables
            .iter_mut()
            .find(|variable| {
                matches!(
                    variable,
                    SemVariableV4::Observed { source_column, .. } if source_column == "x1"
                )
            })
            .unwrap()
        else {
            unreachable!()
        };
        *missing_markers = vec!["-999".into()];

        let mut transformed_model = recursive_model();
        let SemVariableV4::Observed {
            transformation_lineage,
            ..
        } = transformed_model
            .variables
            .iter_mut()
            .find(|variable| {
                matches!(
                    variable,
                    SemVariableV4::Observed { source_column, .. } if source_column == "x1"
                )
            })
            .unwrap()
        else {
            unreachable!()
        };
        *transformation_lineage = vec![ObservedTransformationStepV4 {
            id: "transform:x1:mean_center".into(),
            input_columns: vec!["x1_raw".into()],
            output_column: "x1".into(),
            operation: ObservedTransformationOperationV4::MeanCenter,
        }];

        for model in [missing_marker_model, transformed_model] {
            model.ensure_valid().unwrap();
            let before = model.clone();
            let decision =
                preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
            assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
            assert!(decision.diagnostics().iter().any(|diagnostic| {
                diagnostic.code() == "sem.capability.pls.observed_semantics_not_executable"
                    && diagnostic.subject() == Some("observed:x1")
            }));
            assert_eq!(model, before);
        }
    }

    #[test]
    fn non_listwise_and_each_complex_sampling_role_are_blocked() {
        let mut non_listwise = recursive_model();
        let SemDataBindingV4::Raw { missing_data, .. } = &mut non_listwise.data_binding else {
            unreachable!()
        };
        *missing_data = MissingDataPolicyV4::MeanReplacement;
        non_listwise.ensure_valid().unwrap();
        let decision =
            preflight_general_sem_pls_v1(&non_listwise, &GeneralSemConfigV1::default()).unwrap();
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.listwise_deletion_required"
        }));

        let mut weighted = recursive_model();
        add_sampling_control(&mut weighted);
        let SemDataBindingV4::Raw { weight, .. } = &mut weighted.data_binding else {
            unreachable!()
        };
        *weight = Some(SemWeightBindingV4::Case {
            variable: "observed:sampling_control".into(),
        });

        let mut clustered = recursive_model();
        add_sampling_control(&mut clustered);
        let SemDataBindingV4::Raw {
            cluster_variable, ..
        } = &mut clustered.data_binding
        else {
            unreachable!()
        };
        *cluster_variable = Some("observed:sampling_control".into());

        let mut stratified = recursive_model();
        add_sampling_control(&mut stratified);
        let SemDataBindingV4::Raw {
            strata_variable, ..
        } = &mut stratified.data_binding
        else {
            unreachable!()
        };
        *strata_variable = Some("observed:sampling_control".into());

        for model in [weighted, clustered, stratified] {
            model.ensure_valid().unwrap();
            let decision =
                preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
            assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
            assert!(decision.diagnostics().iter().any(|diagnostic| {
                diagnostic.code() == "sem.capability.pls.complex_sampling_not_executable"
            }));
        }
    }

    #[test]
    fn percentile_two_sided_bootstrap_is_experimental_and_requires_both_exact_cells() {
        let model = multiple_mediation_model();
        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 11,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        let decision = preflight_general_sem_pls_v1(&model, &config).unwrap();
        assert_eq!(
            decision.status(),
            SemCapabilityDecisionStatusV1::Experimental
        );
        assert_eq!(decision.capability_cells().len(), 2);
        assert!(decision.capability_cells().iter().any(|cell| {
            cell.capability_id() == "smartpls.mediation" && cell.cell_id() == "qpls3.pls.mediation"
        }));
        assert!(decision.capability_cells().iter().any(|cell| {
            cell.capability_id() == "smartpls.mediation"
                && cell.cell_id() == "qpls3.pls.general_sem_multiple_mediation_bootstrap"
                && cell.capability_version() == "general_sem_pls_full_model_case_bootstrap_v1"
        }));
        assert!(decision.evidence().iter().any(|item| {
            item.evidence_id()
                == "capability_dependency:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4"
        }));
        assert!(decision.evidence().iter().any(|item| {
            item.evidence_id() == "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1"
        }));
        assert!(decision.explanation().contains("matching complete-model"));
    }

    #[test]
    fn single_indirect_path_is_blocked_from_the_exact_multiple_mediation_bootstrap_cell() {
        let model = recursive_model();
        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 11,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        let decision = preflight_general_sem_pls_v1(&model, &config).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.multiple_mediation_requires_two_indirect_paths"
                && !diagnostic.corrections().is_empty()
        }));
    }

    #[test]
    fn bca_and_one_sided_bootstrap_are_typed_blocked_without_dropping_cells() {
        let model = multiple_mediation_model();
        for (interval, tail, expected_code) in [
            (
                crate::GeneralSemBootstrapIntervalV1::Bca,
                crate::GeneralSemInferenceTailV1::TwoSided,
                "sem.capability.pls.general_bootstrap_bca_not_executable",
            ),
            (
                crate::GeneralSemBootstrapIntervalV1::Percentile,
                crate::GeneralSemInferenceTailV1::OneSidedLower,
                "sem.capability.pls.general_bootstrap_one_sided_not_executable",
            ),
        ] {
            let mut config = GeneralSemConfigV1::default();
            config.inference = GeneralSemInferenceV1::CaseBootstrap {
                resamples: 500,
                seed: 11,
                confidence_level: 0.95,
                interval,
                tail,
            };
            let decision = preflight_general_sem_pls_v1(&model, &config).unwrap();
            assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
            assert_eq!(decision.capability_cells().len(), 2);
            assert!(
                decision
                    .diagnostics()
                    .iter()
                    .any(|diagnostic| diagnostic.code() == expected_code)
            );
            assert!(
                decision
                    .diagnostics()
                    .iter()
                    .filter(|diagnostic| diagnostic.severity()
                        == SemCapabilityDiagnosticSeverityV1::Error)
                    .all(|diagnostic| !diagnostic.corrections().is_empty())
            );
        }
    }
}
