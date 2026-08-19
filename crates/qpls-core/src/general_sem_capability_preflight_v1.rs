use crate::{
    CompiledCbsemExecutionDispositionV3, CompiledCbsemStructuralFormV3,
    CompiledPlsInteractionV3Error, CompiledPlsPlanV3, CompiledPlsPlanV3Error,
    GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemInferenceTailV1,
    GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1, SemCapabilityCellIdV1,
    SemCapabilityDecisionStatusV1, SemCapabilityDecisionV1, SemCapabilityDecisionV1ValidationError,
    SemCapabilityDiagnosticSeverityV1, SemCapabilityDiagnosticV1, SemCapabilityEvidenceV1,
    SemDerivedTermV4, SemModelV4, compile_cbsem_plan_v3, compile_pls_plan_v3,
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
    let has_interactions = model
        .derived_terms
        .iter()
        .any(|term| matches!(term, SemDerivedTermV4::InteractionV2 { .. }));
    let capability_cells = pls_cells(has_interactions, config)?;
    let mut evidence = vec![SemCapabilityEvidenceV1::new(
        "compiler:recipe_v4_to_compiled_pls_plan_v3_v1",
        "The versioned PLS v3 compiler preserves the proven v2 scoring plan and adds stable topology and effect identities.",
    )?];
    if has_interactions {
        evidence.push(SemCapabilityEvidenceV1::new(
            "compiler:recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_point_v1",
            "The bounded compiler projects one shared stage-one score model and jointly solves every qualified two-way interaction in each stage-two equation.",
        )?);
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.moderation:qpls3.pls.general_sem_multiple_two_way_moderation_point:general_sem_pls_multiple_two_way_moderation_point_v1",
            "Capability Registry V2 exposes the exact simultaneous interaction_v2 point-estimation option in Experimental Labs.",
        )?);
    } else {
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.mediation:qpls3.pls.mediation:pls_mediation_v1",
            "Capability Registry V2 exposes the exact mediation option in Experimental Labs.",
        )?);
    }
    if matches!(
        config.inference,
        GeneralSemInferenceV1::CaseBootstrap { .. }
    ) {
        evidence.push(SemCapabilityEvidenceV1::new(
            "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
            "The bootstrap compiler binds exact Recipe V4 inference settings to the General SEM config while retaining the proven point-scoring plan.",
        )?);
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
            "Capability Registry V2 exposes the bounded indexed case-resampling primitive used by this General SEM compiler slice.",
        )?);
    }
    let mut diagnostics = execution_scope_diagnostics(config, has_interactions)?;
    match compile_pls_plan_v3(model, config) {
        Ok(plan) => diagnostics.extend(interaction_scope_diagnostics(config, &plan)?),
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
            if has_interactions {
                "General SEM simultaneous two-way moderation point estimation passes the Experimental Labs compiler preflight."
            } else {
                match config.inference {
                    GeneralSemInferenceV1::None => {
                        "General recursive PLS point estimation and path-specific effects pass the Experimental Labs compiler preflight."
                    }
                    GeneralSemInferenceV1::CaseBootstrap { .. } => {
                        "General recursive PLS percentile case-bootstrap inference passes the bounded Experimental Labs compiler preflight."
                    }
                }
            },
            Vec::new(),
        )?],
        evidence,
        "PLS-SEM can compile this exact request in Experimental Labs.",
        if has_interactions {
            "The compiler binds the source model to one stage-one projection, a joint stage-two solve, explicit product-scale receipts, and fixed -1/0/+1 conditional-slope provenance. Runtime validation remains authoritative before publication."
        } else {
            match config.inference {
                GeneralSemInferenceV1::None => {
                    "The compiler binds the proven PLS scoring plan to stable relation-path identities. Runtime validation remains authoritative before a result can be published."
                }
                GeneralSemInferenceV1::CaseBootstrap { .. } => {
                    "The compiler binds percentile, two-sided case resampling to both the mediation and indexed-resampling cells. Runtime inference must carry a matching complete-model re-estimation receipt before publication."
                }
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
    config: &GeneralSemConfigV1,
    has_interactions: bool,
) -> Result<Vec<SemCapabilityDiagnosticV1>, SemCapabilityDecisionV1ValidationError> {
    let mut diagnostics = Vec::new();
    if !config.conditional_effect_probes.is_empty() {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.conditional_probes_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            if has_interactions {
                "Authored probe policies are preserved, but the first interaction_v2 point cell uses the frozen standardized -1/0/+1 policy only."
            } else {
                "Conditional-effect probes are authored but are not executable in the current PLS v3 point-estimation slice."
            },
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

fn interaction_scope_diagnostics(
    config: &GeneralSemConfigV1,
    plan: &CompiledPlsPlanV3,
) -> Result<Vec<SemCapabilityDiagnosticV1>, SemCapabilityDecisionV1ValidationError> {
    if plan.two_way_interactions().is_empty() {
        return Ok(Vec::new());
    }
    let mut diagnostics = Vec::new();
    if !matches!(config.inference, GeneralSemInferenceV1::None) {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.multiple_moderation_bootstrap_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "Simultaneous interaction_v2 bootstrap inference is not qualified in the current point-only cell.",
            vec![
                "Set General SEM inference to none for descriptive point estimation, or keep the request in Labs until complete-model interaction resampling is qualified.".into(),
            ],
        )?);
    }
    if !config.requested_effect_estimands.is_empty() {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.multiple_moderation_effect_requests_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "Mediation-effect requests cannot be combined with the first simultaneous interaction_v2 point cell.",
            vec![
                "Clear requested indirect/total effects and calculate moderation point estimates only, or retain the model until the combined estimand cell is qualified.".into(),
            ],
        )?);
    }
    if !plan.topology().specific_directed_paths().is_empty() {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.moderated_mediation_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "A directed chain is present, so this graph may imply moderated mediation outside the bounded moderation-only point cell.",
            vec![
                "Use a direct-only structural graph for this point cell, or retain the authored chain until moderated-mediation execution is qualified.".into(),
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
        CompiledPlsPlanV3Error::Interaction(error) => match error {
            CompiledPlsInteractionV3Error::UnsupportedInteractionOrder { .. } => (
                "sem.capability.pls.interaction_order_not_executable",
                "Use exactly two operands per interaction_v2 term; three-way and higher-order moderation remain blocked.",
            ),
            CompiledPlsInteractionV3Error::UnsupportedInteractionMethod { .. } => (
                "sem.capability.pls.interaction_method_not_executable",
                "Choose the two-stage interaction construction method for this bounded point cell.",
            ),
            CompiledPlsInteractionV3Error::UnsupportedInteractionHierarchy { .. } => (
                "sem.capability.pls.interaction_hierarchy_not_executable",
                "Use strong hierarchy and retain every required lower-order path.",
            ),
            _ => (
                "sem.capability.pls.interaction_shape_not_executable",
                "Review the interaction output, effect relation, parameter, and generated-column identities in the compatibility inspector.",
            ),
        },
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
        "smartpls.pls_bootstrapping",
        "qpls3.inference.bootstrap",
        "indexed_resampling_v4",
    )
}

fn pls_multiple_moderation_point_cell()
-> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    SemCapabilityCellIdV1::new(
        2,
        "smartpls.moderation",
        "qpls3.pls.general_sem_multiple_two_way_moderation_point",
        "general_sem_pls_multiple_two_way_moderation_point_v1",
    )
}

fn pls_cells(
    has_interactions: bool,
    config: &GeneralSemConfigV1,
) -> Result<Vec<SemCapabilityCellIdV1>, SemCapabilityDecisionV1ValidationError> {
    let mut cells = if has_interactions {
        vec![pls_multiple_moderation_point_cell()?]
    } else {
        vec![pls_cell()?]
    };
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
        Construct, InteractionHierarchyPolicyV2, InteractionMethodV4,
        LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, SemParameterTargetV4,
        SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath, StructuralRelationRoleV4,
        convert_legacy_basic_model_v4,
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

    fn add_preflight_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
    ) {
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == focal_predictor_id && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        let output = format!("derived:{interaction_id}");
        let effect_relation = format!("relation:{interaction_id}:effect");
        let effect_parameter = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: interaction_id.into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: effect_relation,
            source: output.clone(),
            target: "construct:y".into(),
            parameter: effect_parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: effect_parameter,
            label: format!("{interaction_id} -> Y"),
            target: SemParameterTargetV4::Regression {
                source: output.clone(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.into(),
            output,
            operands: vec![focal_predictor_id.into(), moderator_id.into()],
            focal_relation,
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
        model.ensure_valid().unwrap();
    }

    fn multiple_moderation_model() -> SemModelV4 {
        let mut model = convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x5031_53b0),
                name: "Multiple moderation preflight".into(),
                constructs: ["x", "w", "z", "y"]
                    .into_iter()
                    .map(|id| Construct {
                        id: id.into(),
                        name: id.to_uppercase(),
                        short_name: id.to_uppercase(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec![format!("{id}1"), format!("{id}2")],
                    })
                    .collect(),
                paths: [("x", "y"), ("w", "y"), ("z", "y")]
                    .into_iter()
                    .map(|(source, target)| StructuralPath {
                        source: source.into(),
                        target: target.into(),
                    })
                    .collect(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        add_preflight_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        add_preflight_interaction(
            &mut model,
            "interaction:x_by_z",
            "construct:x",
            "construct:z",
        );
        model
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
    fn multiple_two_way_moderation_uses_only_the_exact_point_labs_cell() {
        let model = multiple_moderation_model();
        let decision =
            preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(
            decision.status(),
            SemCapabilityDecisionStatusV1::Experimental
        );
        assert_eq!(decision.capability_cells().len(), 1);
        let cell = &decision.capability_cells()[0];
        assert_eq!(cell.capability_id(), "smartpls.moderation");
        assert_eq!(
            cell.cell_id(),
            "qpls3.pls.general_sem_multiple_two_way_moderation_point"
        );
        assert_eq!(
            cell.capability_version(),
            "general_sem_pls_multiple_two_way_moderation_point_v1"
        );
        assert!(decision.evidence().iter().any(|item| {
            item.evidence_id()
                == "compiler:recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_point_v1"
        }));
        assert!(decision.explanation().contains("product-scale receipts"));
    }

    #[test]
    fn interaction_bootstrap_and_directed_chain_preflight_remain_explicitly_blocked() {
        let model = multiple_moderation_model();
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
            diagnostic.code() == "sem.capability.pls.multiple_moderation_bootstrap_not_executable"
        }));
        assert_eq!(decision.capability_cells().len(), 2);

        let mut chain = model;
        let parameter = "parameter:chain:x_to_w".to_string();
        chain.relations.push(SemRelationV4::Structural {
            id: "relation:chain:x_to_w".into(),
            source: "construct:x".into(),
            target: "construct:w".into(),
            parameter: parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        chain.parameters.push(SemParameterV4::Free {
            id: parameter,
            label: "X -> W".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:x".into(),
                target: "construct:w".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        chain.ensure_valid().unwrap();
        let decision =
            preflight_general_sem_pls_v1(&chain, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.moderated_mediation_not_executable"
                && !diagnostic.corrections().is_empty()
        }));
    }

    #[test]
    fn percentile_two_sided_bootstrap_is_experimental_and_requires_both_exact_cells() {
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
        assert_eq!(
            decision.status(),
            SemCapabilityDecisionStatusV1::Experimental
        );
        assert_eq!(decision.capability_cells().len(), 2);
        assert!(decision.capability_cells().iter().any(|cell| {
            cell.capability_id() == "smartpls.mediation" && cell.cell_id() == "qpls3.pls.mediation"
        }));
        assert!(decision.capability_cells().iter().any(|cell| {
            cell.capability_id() == "smartpls.pls_bootstrapping"
                && cell.cell_id() == "qpls3.inference.bootstrap"
                && cell.capability_version() == "indexed_resampling_v4"
        }));
        assert!(decision.evidence().iter().any(|item| {
            item.evidence_id() == "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1"
        }));
        assert!(decision.explanation().contains("matching complete-model"));
    }

    #[test]
    fn bca_and_one_sided_bootstrap_are_typed_blocked_without_dropping_cells() {
        let model = recursive_model();
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
