use crate::{
    CompiledCbsemExecutionDispositionV3, CompiledCbsemStructuralFormV3,
    CompiledPlsHigherOrderV1Error, CompiledPlsInteractionV3Error, CompiledPlsPlanV3,
    CompiledPlsPlanV3Error, GeneralSemBootstrapIntervalV1, GeneralSemConfigV1,
    GeneralSemInferenceTailV1, GeneralSemInferenceV1, GeneralSemSpecificPathLimitBehaviorV1,
    MissingDataPolicyV4, ObservedScaleV4, SemCapabilityCellIdV1, SemCapabilityDecisionStatusV1,
    SemCapabilityDecisionV1, SemCapabilityDecisionV1ValidationError,
    SemCapabilityDiagnosticSeverityV1, SemCapabilityDiagnosticV1, SemCapabilityEvidenceV1,
    SemDataBindingV4, SemDerivedTermV4, SemGroupV4, SemModelV4, SemVariableV4,
    compile_cbsem_plan_v3, compile_pls_plan_v3,
    pls_general_higher_order_bootstrap_capability_cell_v1,
    pls_general_higher_order_point_capability_cell_v1,
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
    let has_higher_order = model
        .derived_terms
        .iter()
        .any(|term| matches!(term, SemDerivedTermV4::HigherOrder { .. }));
    let capability_cells = pls_cells(has_interactions, has_higher_order, config)?;
    let mut evidence = vec![SemCapabilityEvidenceV1::new(
        "compiler:recipe_v4_to_compiled_pls_plan_v3_v1",
        "The versioned PLS v3 compiler preserves the proven v2 scoring plan and adds stable topology and effect identities.",
    )?];
    if has_higher_order {
        evidence.push(SemCapabilityEvidenceV1::new(
            "compiler:recipe_v4_to_compiled_pls_plan_v3_higher_order_point_v1",
            "The bounded compiler binds one SemModelV4 HOC to explicit Mode A/B semantics, stable generated identities, and ordered approach-specific stages.",
        )?);
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_contract:smartpls.higher_order_models:qpls3.pls.general_sem_higher_order_point:general_sem_pls_higher_order_point_v1",
            "The exact bounded General SEM HOC point identity owns approach-specific staged execution and canonical result authority.",
        )?);
    } else if has_interactions {
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
        if has_higher_order {
            evidence.push(SemCapabilityEvidenceV1::new(
                "compiler:recipe_v4_to_compiled_pls_plan_v3_higher_order_full_model_case_bootstrap_v1",
                "The supplemental HOC bootstrap compiler binds indexed raw-case resampling to complete approach-specific stage refitting.",
            )?);
            evidence.push(SemCapabilityEvidenceV1::new(
                "capability_contract:smartpls.higher_order_models:qpls3.pls.general_sem_higher_order_full_model_case_bootstrap:general_sem_pls_higher_order_full_model_case_bootstrap_v1",
                "The exact bounded HOC bootstrap identity owns indexed raw-case resampling and complete approach-specific stage refitting.",
            )?);
        } else if has_interactions {
            evidence.push(SemCapabilityEvidenceV1::new(
                "compiler:recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_bootstrap_v1",
                "The supplemental moderation bootstrap compiler binds percentile, two-sided full-model case resampling while preserving the point cell as the compiled artifact's primary authority.",
            )?);
            evidence.push(SemCapabilityEvidenceV1::new(
                "capability_registry_v2:smartpls.moderation:qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap:general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
                "Capability Registry V2 exposes the exact gamma-only simultaneous interaction_v2 full-model case-bootstrap option in Experimental Labs.",
            )?);
        } else {
            evidence.push(SemCapabilityEvidenceV1::new(
                "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
                "The bootstrap compiler binds exact Recipe V4 inference settings to the General SEM config while retaining the proven point-scoring plan.",
            )?);
            evidence.push(SemCapabilityEvidenceV1::new(
                "capability_registry_v2:smartpls.mediation:qpls3.pls.general_sem_multiple_mediation_bootstrap:general_sem_pls_full_model_case_bootstrap_v1",
                "Capability Registry V2 exposes this exact multiple-mediation, full-model percentile case-bootstrap combination in Experimental Labs.",
            )?);
        }
        evidence.push(SemCapabilityEvidenceV1::new(
            "capability_dependency:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
            "The exact General SEM cell uses the separately governed indexed case-resampling mechanism without inheriting that mechanism cell's release maturity.",
        )?);
    }
    let mut diagnostics = execution_scope_diagnostics(model, config, has_interactions)?;
    match compile_pls_plan_v3(model, config) {
        Ok(plan) => {
            if has_higher_order {
                debug_assert_eq!(plan.higher_order_stage_plans().len(), 1);
            } else if has_interactions {
                diagnostics.extend(interaction_scope_diagnostics(config, &plan)?);
            } else {
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
            if has_higher_order {
                match config.inference {
                    GeneralSemInferenceV1::None => {
                        "General SEM higher-order point estimation passes the bounded Experimental Labs compiler preflight."
                    }
                    GeneralSemInferenceV1::CaseBootstrap { .. } => {
                        "General SEM higher-order full-model percentile case-bootstrap inference passes the bounded Experimental Labs compiler preflight."
                    }
                }
            } else if has_interactions {
                match config.inference {
                    GeneralSemInferenceV1::None => {
                        "General SEM simultaneous two-way moderation point estimation passes the Experimental Labs compiler preflight."
                    }
                    GeneralSemInferenceV1::CaseBootstrap { .. } => {
                        "General SEM simultaneous two-way moderation gamma-only percentile case-bootstrap inference passes the bounded Experimental Labs compiler preflight."
                    }
                }
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
        if has_higher_order {
            match config.inference {
                GeneralSemInferenceV1::None => {
                    "The compiler binds one exact HOC approach/type predicate to stable generated mappings and ordered stage projections. Runtime qualification remains required before Standard promotion."
                }
                GeneralSemInferenceV1::CaseBootstrap { .. } => {
                    "The point HOC cell remains primary authority and the supplemental Labs cell requires every raw-case replicate to rerun all compiled stages under one usable/failure ledger."
                }
            }
        } else if has_interactions {
            match config.inference {
                GeneralSemInferenceV1::None => {
                    "The compiler binds the source model to one stage-one projection, a joint stage-two solve, explicit product-scale receipts, and fixed -1/0/+1 conditional-slope provenance. Runtime validation remains authoritative before publication."
                }
                GeneralSemInferenceV1::CaseBootstrap { .. } => {
                    "The point moderation cell remains the primary artifact authority and the supplemental Labs cell authorizes percentile, two-sided full-model case-bootstrap inference for scientific rescaled gamma only. A runtime must retain indexed-resampling and complete-model re-estimation receipts before publication."
                }
            }
        } else {
            match config.inference {
                GeneralSemInferenceV1::None => {
                    "The compiler binds the proven PLS scoring plan to stable relation-path identities. Runtime validation remains authoritative before a result can be published."
                }
                GeneralSemInferenceV1::CaseBootstrap { .. } => {
                    "The compiler binds percentile, two-sided case resampling to the exact multiple-mediation bootstrap cell and records the indexed-resampling mechanism as a dependency. Runtime inference must carry a matching complete-model re-estimation receipt before publication."
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
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
    has_interactions: bool,
) -> Result<Vec<SemCapabilityDiagnosticV1>, SemCapabilityDecisionV1ValidationError> {
    let mut diagnostics = Vec::new();
    if has_interactions {
        for term in &model.derived_terms {
            let SemDerivedTermV4::InteractionV2 { id, operands, .. } = term else {
                continue;
            };
            if operands.len() != 2 {
                diagnostics.push(SemCapabilityDiagnosticV1::new(
                    "sem.capability.pls.interaction_order_not_executable",
                    SemCapabilityDiagnosticSeverityV1::Error,
                    Some(id.clone()),
                    format!(
                        "Interaction {id} requires exactly two operands; received {}.",
                        operands.len()
                    ),
                    vec![
                        "Use exactly two operands per interaction_v2 term; three-way and higher-order moderation remain blocked.".into(),
                    ],
                )?);
            }
        }
    }
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
            if has_interactions {
                "Authored probe policies are preserved, but the moderation point cell uses the frozen standardized -1/0/+1 policy and the supplemental bootstrap cell is gamma-only."
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
    if !config.requested_effect_estimands.is_empty() {
        diagnostics.push(SemCapabilityDiagnosticV1::new(
            "sem.capability.pls.multiple_moderation_effect_requests_not_executable",
            SemCapabilityDiagnosticSeverityV1::Error,
            None,
            "Mediation-effect requests cannot be combined with the simultaneous interaction_v2 point or gamma-only bootstrap cells.",
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
            "A directed chain is present, so this graph may imply moderated mediation outside the bounded direct-only moderation cells.",
            vec![
                "Use a direct-only structural graph for these cells, or retain the authored chain until moderated-mediation execution is qualified.".into(),
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
        CompiledPlsPlanV3Error::HigherOrderRequestedEffectsNotExecutable => (
            "sem.capability.pls.higher_order_generic_effect_requests_not_executable",
            "Clear generic requested effects; HOC loadings, weights, authored HOC paths, and extended-repeated effects are published through the typed HOC stage tables.",
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
            CompiledPlsInteractionV3Error::DuplicateInteractionProductDesign { .. } => (
                "sem.capability.pls.duplicate_interaction_product_design",
                "Keep one two-way product per operand pair and outcome under the fixed two-stage sample-standardized policy; its authored operand order still defines the focal predictor and moderator roles.",
            ),
            _ => (
                "sem.capability.pls.interaction_shape_not_executable",
                "Review the interaction output, effect relation, parameter, and generated-column identities in the compatibility inspector.",
            ),
        },
        CompiledPlsPlanV3Error::HigherOrder(error) => match error {
            CompiledPlsHigherOrderV1Error::HigherOrderCardinality { .. } => (
                "sem.capability.pls.higher_order_cardinality_not_executable",
                "Keep exactly one non-nested second-order HOC in this bounded General SEM request.",
            ),
            CompiledPlsHigherOrderV1Error::DerivedTermCombination { .. } => (
                "sem.capability.pls.higher_order_derived_combination_not_executable",
                "Remove interaction, polynomial, nested, or additional HOC terms from this exact HOC calculation; the authored model remains saved.",
            ),
            CompiledPlsHigherOrderV1Error::HybridCompatibilityOnly { .. } => (
                "sem.capability.pls.higher_order_hybrid_compatibility_only",
                "Choose repeated indicators, extended repeated indicators, embedded two-stage, or disjoint two-stage; Hybrid remains compatibility-only.",
            ),
            CompiledPlsHigherOrderV1Error::UnsupportedApproachTypeTopology { .. } => (
                "sem.capability.pls.higher_order_approach_type_topology_not_executable",
                "Use the exact approach/HCM matrix: repeated RR/FR or exogenous RF/FF; endogenous extended RF/FF; embedded/disjoint with any HCM type.",
            ),
            CompiledPlsHigherOrderV1Error::NestedOrNonCompositeComponent { .. } => (
                "sem.capability.pls.higher_order_component_not_executable",
                "Select at least two ordinary non-nested composite lower-order components.",
            ),
            CompiledPlsHigherOrderV1Error::ComponentModeMismatch { .. }
            | CompiledPlsHigherOrderV1Error::FixedOrCustomScoring { .. } => (
                "sem.capability.pls.higher_order_measurement_mode_not_executable",
                "Use Mode A LOCs for reflective-first HCM types and Mode B LOCs for formative-first HCM types; fixed/custom scoring is outside this cell.",
            ),
            _ => (
                "sem.capability.pls.higher_order_shape_not_executable",
                "Review the HOC output, components, authored structural paths, parameters, and generated-identity diagnostics in the compatibility inspector.",
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
        "smartpls.mediation",
        "qpls3.pls.general_sem_multiple_mediation_bootstrap",
        "general_sem_pls_full_model_case_bootstrap_v1",
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

fn pls_multiple_moderation_bootstrap_cell()
-> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    SemCapabilityCellIdV1::new(
        2,
        "smartpls.moderation",
        "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
    )
}

fn pls_higher_order_point_cell()
-> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    let cell = pls_general_higher_order_point_capability_cell_v1();
    SemCapabilityCellIdV1::new(
        cell.registry_schema_version,
        cell.capability_id,
        cell.cell_id,
        cell.capability_version,
    )
}

fn pls_higher_order_bootstrap_cell()
-> Result<SemCapabilityCellIdV1, SemCapabilityDecisionV1ValidationError> {
    let cell = pls_general_higher_order_bootstrap_capability_cell_v1();
    SemCapabilityCellIdV1::new(
        cell.registry_schema_version,
        cell.capability_id,
        cell.cell_id,
        cell.capability_version,
    )
}

fn pls_cells(
    has_interactions: bool,
    has_higher_order: bool,
    config: &GeneralSemConfigV1,
) -> Result<Vec<SemCapabilityCellIdV1>, SemCapabilityDecisionV1ValidationError> {
    let mut cells = if has_higher_order {
        vec![pls_higher_order_point_cell()?]
    } else if has_interactions {
        vec![pls_multiple_moderation_point_cell()?]
    } else {
        vec![pls_cell()?]
    };
    if matches!(
        config.inference,
        GeneralSemInferenceV1::CaseBootstrap { .. }
    ) {
        cells.push(if has_higher_order {
            pls_higher_order_bootstrap_cell()?
        } else if has_interactions {
            pls_multiple_moderation_bootstrap_cell()?
        } else {
            pls_bootstrap_cell()?
        });
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
        Construct, GeneralSemConditionalEffectProbeV1, GeneralSemConditionalProbeValuesV1,
        GeneralSemEffectEstimandV1, HigherOrderConstructionApproachV4,
        HigherOrderMeasurementTypeV4, InteractionHierarchyPolicyV2, InteractionMethodV4,
        LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, ObservedRoleV4,
        ObservedTransformationOperationV4, ObservedTransformationStepV4, SemParameterTargetV4,
        SemParameterV4, SemRelationV4, SemWeightBindingV4, StructuralPath,
        StructuralRelationRoleV4, convert_legacy_basic_model_v4,
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

    fn disjoint_higher_order_model() -> SemModelV4 {
        let mut model = recursive_model();
        let output = "derived:hoc".to_string();
        let relation = "relation:hoc_y".to_string();
        let parameter = "parameter:hoc_y".to_string();
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: "Higher order".into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: relation,
            source: output.clone(),
            target: "construct:y".into(),
            parameter: parameter.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter,
            label: "HOC -> Y".into(),
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
        model.derived_terms.push(SemDerivedTermV4::HigherOrder {
            id: "term:hoc".into(),
            output,
            components: vec!["construct:x".into(), "construct:m".into()],
            approach: HigherOrderConstructionApproachV4::DisjointTwoStage,
            measurement_type: HigherOrderMeasurementTypeV4::ReflectiveReflective,
        });
        model.ensure_valid().unwrap();
        model
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

    fn multiple_moderation_model_for_layout(different_focal: bool) -> SemModelV4 {
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
            if different_focal {
                "interaction:z_by_w"
            } else {
                "interaction:x_by_z"
            },
            if different_focal {
                "construct:z"
            } else {
                "construct:x"
            },
            if different_focal {
                "construct:w"
            } else {
                "construct:z"
            },
        );
        model
    }

    fn multiple_moderation_model() -> SemModelV4 {
        multiple_moderation_model_for_layout(false)
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
    fn multiple_two_way_moderation_uses_only_the_exact_point_labs_cell() {
        for different_focal in [false, true] {
            let model = multiple_moderation_model_for_layout(different_focal);
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
    }

    #[test]
    fn reversed_role_duplicate_product_design_has_a_corrective_preflight_code() {
        let mut model = multiple_moderation_model();
        add_preflight_interaction(
            &mut model,
            "interaction:w_by_x",
            "construct:w",
            "construct:x",
        );

        let decision =
            preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        let diagnostic = decision
            .diagnostics()
            .iter()
            .find(|diagnostic| {
                diagnostic.code() == "sem.capability.pls.duplicate_interaction_product_design"
            })
            .unwrap();
        assert!(diagnostic.message().contains("interaction:x_by_w"));
        assert!(diagnostic.message().contains("interaction:w_by_x"));
        assert!(diagnostic.message().contains("construct:y"));
        assert!(
            diagnostic
                .corrections()
                .iter()
                .any(|correction| correction.contains("authored operand order"))
        );
    }

    #[test]
    fn interaction_bootstrap_uses_point_plus_supplemental_cells_for_each_focal_layout() {
        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 11,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        for different_focal in [false, true] {
            let model = multiple_moderation_model_for_layout(different_focal);
            let point_before =
                preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
            let decision = preflight_general_sem_pls_v1(&model, &config).unwrap();
            assert_eq!(
                decision.status(),
                SemCapabilityDecisionStatusV1::Experimental
            );
            assert_eq!(decision.capability_cells().len(), 2);
            assert!(decision.capability_cells().iter().any(|cell| {
                cell.cell_id() == "qpls3.pls.general_sem_multiple_two_way_moderation_point"
                    && cell.capability_version()
                        == "general_sem_pls_multiple_two_way_moderation_point_v1"
            }));
            assert!(decision.capability_cells().iter().any(|cell| {
                cell.cell_id() == "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap"
                    && cell.capability_version()
                        == "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
            }));
            assert!(decision.evidence().iter().any(|item| {
                item.evidence_id()
                    == "compiler:recipe_v4_to_compiled_pls_plan_v3_multiple_two_way_moderation_bootstrap_v1"
            }));
            assert!(
                decision
                    .explanation()
                    .contains("scientific rescaled gamma only")
            );
            assert_eq!(
                preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap(),
                point_before,
                "supplemental bootstrap preflight must not change the point decision"
            );
        }
    }

    #[test]
    fn interaction_bootstrap_boundaries_keep_exact_cells_and_corrective_diagnostics() {
        let model = multiple_moderation_model();
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
        }

        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 11,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        config.conditional_effect_probes = vec![GeneralSemConditionalEffectProbeV1 {
            probe_id: "probe:w".into(),
            moderator_id: "construct:w".into(),
            values: GeneralSemConditionalProbeValuesV1::DataDerivedMeanPlusMinusOneSd,
        }];
        config.requested_effect_estimands = vec![GeneralSemEffectEstimandV1::TotalEffect {
            estimand_id: "effect:x_to_y".into(),
            source_id: "construct:x".into(),
            target_id: "construct:y".into(),
        }];
        let decision = preflight_general_sem_pls_v1(&model, &config).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert_eq!(decision.capability_cells().len(), 2);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.conditional_probes_not_executable"
        }));
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code()
                == "sem.capability.pls.multiple_moderation_effect_requests_not_executable"
        }));
        assert!(
            decision
                .diagnostics()
                .iter()
                .filter(|diagnostic| {
                    diagnostic.severity() == SemCapabilityDiagnosticSeverityV1::Error
                })
                .all(|diagnostic| !diagnostic.corrections().is_empty())
        );
    }

    #[test]
    fn interaction_order_method_and_hierarchy_boundaries_remain_typed() {
        let mut order = multiple_moderation_model();
        let SemDerivedTermV4::InteractionV2 { operands, .. } = &mut order.derived_terms[0] else {
            unreachable!()
        };
        operands.push("construct:z".into());

        let mut method = multiple_moderation_model();
        let SemDerivedTermV4::InteractionV2 {
            method: interaction_method,
            ..
        } = &mut method.derived_terms[0]
        else {
            unreachable!()
        };
        *interaction_method = InteractionMethodV4::Orthogonalizing;

        let mut hierarchy = multiple_moderation_model();
        let SemDerivedTermV4::InteractionV2 {
            hierarchy_policy, ..
        } = &mut hierarchy.derived_terms[0]
        else {
            unreachable!()
        };
        *hierarchy_policy = InteractionHierarchyPolicyV2::Weak;

        let mut derived_scope = multiple_moderation_model();
        derived_scope.variables.push(SemVariableV4::Derived {
            id: "derived:x_squared".into(),
            label: "X squared".into(),
        });
        derived_scope
            .derived_terms
            .push(SemDerivedTermV4::Polynomial {
                id: "polynomial:x_squared".into(),
                output: "derived:x_squared".into(),
                source: "construct:x".into(),
                degree: 2,
            });

        for (model, expected_code) in [
            (order, "sem.capability.pls.interaction_order_not_executable"),
            (
                method,
                "sem.capability.pls.interaction_method_not_executable",
            ),
            (
                hierarchy,
                "sem.capability.pls.interaction_hierarchy_not_executable",
            ),
            (
                derived_scope,
                "sem.capability.pls.interaction_shape_not_executable",
            ),
        ] {
            let decision =
                preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
            assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
            assert!(
                decision
                    .diagnostics()
                    .iter()
                    .any(|diagnostic| diagnostic.code() == expected_code)
            );
        }
    }

    #[test]
    fn directed_chain_preflight_remains_explicitly_blocked() {
        let model = multiple_moderation_model();

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
        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 11,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        let decision = preflight_general_sem_pls_v1(&chain, &config).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code() == "sem.capability.pls.moderated_mediation_not_executable"
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

    #[test]
    fn hoc_preflight_binds_exact_cells_and_reaches_the_connected_runtime() {
        let model = disjoint_higher_order_model();
        let point = preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(point.status(), SemCapabilityDecisionStatusV1::Experimental);
        assert_eq!(point.capability_cells().len(), 1);
        assert_eq!(
            point.capability_cells()[0].cell_id(),
            "qpls3.pls.general_sem_higher_order_point"
        );
        assert!(point.diagnostics().iter().all(|diagnostic| {
            diagnostic.code() != "sem.capability.pls.higher_order_runtime_not_connected"
        }));
        assert!(point.evidence().iter().any(|evidence| {
            evidence.evidence_id()
                == "capability_contract:smartpls.higher_order_models:qpls3.pls.general_sem_higher_order_point:general_sem_pls_higher_order_point_v1"
        }));

        let mut config = GeneralSemConfigV1::default();
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 11,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        let bootstrap = preflight_general_sem_pls_v1(&model, &config).unwrap();
        assert_eq!(
            bootstrap.status(),
            SemCapabilityDecisionStatusV1::Experimental
        );
        assert_eq!(bootstrap.capability_cells().len(), 2);
        assert!(bootstrap.capability_cells().iter().any(|cell| {
            cell.cell_id() == "qpls3.pls.general_sem_higher_order_full_model_case_bootstrap"
        }));
    }

    #[test]
    fn unsupported_hoc_matrix_returns_a_stable_corrective_diagnostic() {
        let mut model = disjoint_higher_order_model();
        let SemDerivedTermV4::HigherOrder {
            approach,
            measurement_type,
            ..
        } = &mut model.derived_terms[0]
        else {
            unreachable!()
        };
        *approach = HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators;
        *measurement_type = HigherOrderMeasurementTypeV4::ReflectiveReflective;
        model.ensure_valid().unwrap();
        let decision =
            preflight_general_sem_pls_v1(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(decision.status(), SemCapabilityDecisionStatusV1::Blocked);
        assert!(decision.diagnostics().iter().any(|diagnostic| {
            diagnostic.code()
                == "sem.capability.pls.higher_order_approach_type_topology_not_executable"
                && !diagnostic.corrections().is_empty()
        }));
    }
}
