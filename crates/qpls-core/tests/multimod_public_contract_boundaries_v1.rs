use qpls_core::{
    CausalIdentificationChecklistV1, CausalLinearEquationV1, CausalLinearTermV1,
    CausalPositivityPolicyV1, InferenceAlternativeV1, InterventionalCausalMediationConfigV1,
    MgaComparisonPlanV1, MgaModelProfileV1, MgaMultigroupV1, MgaProcedureV1,
    MicomConfiguralChecklistV1, MultiplicityAdjustmentV1, ObservedCausalPathV1,
    ObservedTreatmentContrastV1, SelectedGroupV1, TypedGroupValueV1,
};

fn reviewed_identification() -> CausalIdentificationChecklistV1 {
    CausalIdentificationChecklistV1 {
        temporal_order_declared: true,
        adjustment_set_justified: true,
        consistency_assumption_acknowledged: true,
        no_unmeasured_treatment_outcome_confounding_acknowledged: true,
        no_unmeasured_treatment_mediator_confounding_acknowledged: true,
        no_unmeasured_mediator_outcome_confounding_acknowledged: true,
        no_exposure_induced_mediator_outcome_confounder_confirmed: true,
        no_recanting_witness_confirmed: true,
        linear_model_specification_reviewed: true,
        positivity_reviewed: true,
    }
}

fn term(id: &str, factors: &[&str]) -> CausalLinearTermV1 {
    CausalLinearTermV1 {
        term_id: id.into(),
        factor_variable_ids: factors.iter().map(|value| (*value).into()).collect(),
    }
}

fn valid_causal_config() -> InterventionalCausalMediationConfigV1 {
    InterventionalCausalMediationConfigV1 {
        schema_version: 1,
        treatment: "x".into(),
        treatment_contrast: ObservedTreatmentContrastV1::Binary {
            control: 0.0,
            treated: 1.0,
        },
        outcome: "y".into(),
        mediators: vec!["m".into()],
        baseline_moderators: vec!["z".into()],
        adjustment_covariates: vec!["c".into()],
        paths: vec![ObservedCausalPathV1 {
            path_id: "x-m-y".into(),
            ordered_variable_ids: vec!["x".into(), "m".into(), "y".into()],
            equations: vec![
                CausalLinearEquationV1 {
                    equation_id: "m-equation".into(),
                    outcome_variable_id: "m".into(),
                    terms: vec![term("x", &["x"]), term("c", &["c"]), term("z", &["z"])],
                },
                CausalLinearEquationV1 {
                    equation_id: "y-equation".into(),
                    outcome_variable_id: "y".into(),
                    terms: vec![
                        term("m", &["m"]),
                        term("x", &["x"]),
                        term("c", &["c"]),
                        term("z", &["z"]),
                        term("x-by-z", &["x", "z"]),
                    ],
                },
            ],
        }],
        positivity_policy: CausalPositivityPolicyV1::default(),
        identification: reviewed_identification(),
        bootstrap_resamples: 500,
        seed: 42,
        confidence_level: 0.95,
    }
}

#[test]
fn causal_v1_requires_an_explicit_nonempty_adjustment_set() {
    let mut config = valid_causal_config();
    config
        .ensure_valid()
        .expect("fixture is inside the V1 envelope");

    config.adjustment_covariates.clear();
    let error = config
        .ensure_valid()
        .expect_err("an absent adjustment set must fail closed");
    assert_eq!(
        error.code,
        "interventional_causal_mediation_v1.adjustment_set_missing"
    );
}

#[test]
fn causal_v1_rejects_unknown_wire_fields() {
    let config = valid_causal_config();
    let mut wire = serde_json::to_value(config).expect("serialize fixture");
    wire.as_object_mut()
        .expect("configuration is an object")
        .insert(
            "causality_established".into(),
            serde_json::Value::Bool(true),
        );

    let error = serde_json::from_value::<InterventionalCausalMediationConfigV1>(wire)
        .expect_err("unknown causal claims must not enter the typed contract");
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn causal_v1_interactions_require_explicit_lower_order_terms() {
    let mut config = valid_causal_config();
    let outcome_equation = config.paths[0]
        .equations
        .last_mut()
        .expect("outcome equation");
    outcome_equation
        .terms
        .retain(|candidate| candidate.term_id != "z");

    let error = config
        .ensure_valid()
        .expect_err("strong hierarchy cannot be synthesized silently");
    assert_eq!(
        error.code,
        "interventional_causal_mediation_v1.equation_term"
    );
}

fn reviewed_micom() -> MicomConfiguralChecklistV1 {
    MicomConfiguralChecklistV1 {
        identical_indicators_and_coding: true,
        identical_data_treatment: true,
        identical_algorithm_settings: true,
        identical_model_specification: true,
        deterministic_sign_orientation_reviewed: true,
        analyst_review_confirmed: true,
    }
}

#[test]
fn three_or_more_mga_groups_require_the_omnibus_gate() {
    let groups = ["a", "b", "c"]
        .into_iter()
        .enumerate()
        .map(|(index, id)| SelectedGroupV1 {
            group_id: id.into(),
            label: id.to_uppercase(),
            value: TypedGroupValueV1::Integer {
                value: index as i64,
            },
        })
        .collect();
    let mut config = MgaMultigroupV1 {
        schema_version: 1,
        profile: MgaModelProfileV1::GeneralSemPls,
        grouping_column: "group".into(),
        groups,
        comparison_plan: MgaComparisonPlanV1::AllPairs {
            heavy_run_confirmed: false,
        },
        procedures: vec![MgaProcedureV1::PairwisePermutation],
        permutation_samples: 5_000,
        bootstrap_samples: 5_000,
        seed: 42,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: InferenceAlternativeV1::TwoSided,
        multiplicity: MultiplicityAdjustmentV1::Holm,
        configural_checklist: reviewed_micom(),
        weight: None,
        selected_parameter_ids: vec!["path:x:y".into()],
    };

    let error = config
        .ensure_valid()
        .expect_err("pairwise K-group follow-up needs its omnibus gate");
    assert_eq!(error.code, "mga_multigroup_v1.omnibus_required");

    config
        .procedures
        .push(MgaProcedureV1::OmnibusMaxSpreadPermutation);
    config
        .ensure_valid()
        .expect("the explicit K-group omnibus gate admits follow-up");

    config.procedures = vec![MgaProcedureV1::ParametricWaldOmnibus];
    config
        .ensure_valid()
        .expect("a K-group omnibus-only sensitivity run has no pairwise follow-up to gate");
}
