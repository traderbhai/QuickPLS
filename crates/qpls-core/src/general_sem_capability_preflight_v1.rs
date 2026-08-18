use crate::{
    CompiledCbsemExecutionDispositionV3, CompiledCbsemStructuralFormV3, CompiledPlsPlanV3Error,
    GeneralSemConfigV1, GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1,
    SemCapabilityCellIdV1, SemCapabilityDecisionStatusV1, SemCapabilityDecisionV1,
    SemCapabilityDecisionV1ValidationError, SemCapabilityDiagnosticSeverityV1,
    SemCapabilityDiagnosticV1, SemCapabilityEvidenceV1, SemModelV4, compile_cbsem_plan_v3,
    compile_pls_plan_v3,
};

pub const GENERAL_SEM_PLS_ESTIMATOR_ID_V1: &str = "qpls.pls_sem.v3";
pub const GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1: &str = "qpls.cbsem.v3";

/// Exact, recovery-oriented preflight for the currently executable General SEM
/// PLS point-estimation slice. Model semantics are inspected but never changed.
pub fn preflight_general_sem_pls_v1(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<SemCapabilityDecisionV1, SemCapabilityDecisionV1ValidationError> {
    let cell = pls_cell()?;
    let evidence = vec![
        SemCapabilityEvidenceV1::new(
            "capability_registry_v2:smartpls.mediation:qpls3.pls.mediation:pls_mediation_v1",
            "Capability Registry V2 exposes the exact mediation option in Experimental Labs.",
        )?,
        SemCapabilityEvidenceV1::new(
            "compiler:recipe_v4_to_compiled_pls_plan_v3_v1",
            "The versioned PLS v3 compiler preserves the proven v2 scoring plan and adds stable topology and effect identities.",
        )?,
    ];
    let mut diagnostics = execution_scope_diagnostics(config)?;
    match compile_pls_plan_v3(model, config) {
        Ok(_) => {}
        Err(error) => diagnostics.push(pls_compile_diagnostic(error)?),
    }
    if !diagnostics.is_empty() {
        return SemCapabilityDecisionV1::new(
            SemCapabilityDecisionStatusV1::Blocked,
            GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
            vec![cell],
            diagnostics,
            evidence,
            "PLS-SEM cannot calculate this exact General SEM request yet.",
            "The authored model remains intact. Apply one of the listed corrections or select an estimator whose exact capability cell supports the graph.",
        );
    }
    SemCapabilityDecisionV1::new(
        SemCapabilityDecisionStatusV1::Experimental,
        GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
        vec![cell],
        vec![SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.experimental_labs",
            SemCapabilityDiagnosticSeverityV1::Info,
            None,
            "General recursive PLS point estimation and path-specific effects are available in Experimental Labs.",
            Vec::new(),
        )?],
        evidence,
        "PLS-SEM can calculate this request in Experimental Labs.",
        "The complete recursive model is re-estimated by the proven PLS score executor and decomposed through stable relation-path identities. Resampling and conditional effects require separate qualified cells.",
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
) -> Result<Vec<SemCapabilityDiagnosticV1>, SemCapabilityDecisionV1ValidationError> {
    let mut diagnostics = Vec::new();
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
    if config.inference != GeneralSemInferenceV1::None {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.general_inference_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "General SEM case-bootstrap inference is requested but is not connected to the v3 execution adapter.",
            vec![
                "Set General SEM inference to none for the current point-estimation slice, or use a separately qualified bounded bootstrap workflow.".into(),
            ],
        )?);
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

fn cbsem_cell() -> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    SemCapabilityCellIdV1::new(2, "smartpls.cbsem", "qpls3.cbsem.ml", "cbsem_ml_v1")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, StructuralPath,
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
    fn unimplemented_inference_is_never_silently_ignored() {
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
            diagnostic.code() == "sem.capability.pls.general_inference_not_executable"
        }));
    }
}
