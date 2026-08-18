use chrono::{DateTime, Utc};
use qpls_core::{
    CompiledPlsPlanV2Error, CompositeWeightingV4, Construct, FactorMeanPolicyV4,
    HigherOrderConstructionApproachV4, HigherOrderMeasurementTypeV4, InteractionMethodV4,
    LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, ObservedRoleV4, ObservedScaleV4,
    ProductIndicatorCenteringV4, ProductIndicatorPairingV4, ProductIndicatorSpecificationV4,
    ProductIndicatorStandardizationV4, SamplingWeightNormalizationV4, SemAnnotationV4,
    SemCanvasEdgeV4, SemCanvasImageV4, SemCanvasLineV4, SemCanvasNodeV4, SemCanvasShapeKindV4,
    SemCanvasShapeV4, SemConstraintV4, SemDerivedTermV4, SemEndpointV4, SemGroupLevelV4,
    SemGroupV4, SemModelV4, SemParameterGroupOverrideSpecV4, SemParameterGroupOverrideV4,
    SemParameterTargetV4, SemParameterV4, SemPresentationV4, SemRelationV4, SemVariableV4,
    SemWeightBindingV4, StructuralPath, StructuralRelationRoleV4, compile_cbsem_plan_v2,
    compile_pls_plan_v2, convert_legacy_basic_model_v4,
    validate_cbsem_ml_v1_estimator_capability_v2,
};
use qpls_project::{
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, ProjectArchiveDocumentV6, ProjectModelPayloadV6,
    ProjectModelRecordV6, ProjectOriginV6, read_project_document_v6, serialize_project_document_v6,
    write_project_document_v6_new,
};
use std::collections::{BTreeMap, BTreeSet};
use tempfile::tempdir;
use uuid::Uuid;

fn free_parameter(id: &str, label: &str, target: SemParameterTargetV4) -> SemParameterV4 {
    SemParameterV4::Free {
        id: id.into(),
        label: label.into(),
        target,
        start: None,
        lower: None,
        upper: None,
        equality_label: None,
        group_overrides: Vec::new(),
    }
}

fn observed(
    id: &str,
    label: &str,
    scale: ObservedScaleV4,
    role: ObservedRoleV4,
    categories: &[&str],
) -> SemVariableV4 {
    SemVariableV4::Observed {
        id: id.into(),
        label: label.into(),
        source_column: id.trim_start_matches("observed:").into(),
        scale,
        role,
        categories: categories.iter().map(|value| (*value).into()).collect(),
        value_labels: BTreeMap::new(),
        missing_markers: Vec::new(),
        transformation_lineage: Vec::new(),
    }
}

fn push_structural(
    model: &mut SemModelV4,
    id: &str,
    source: &str,
    target: &str,
    intercept_parameter: Option<&str>,
) {
    let parameter_id = format!("parameter:{id}");
    model.relations.push(SemRelationV4::Structural {
        id: id.into(),
        source: source.into(),
        target: target.into(),
        parameter: parameter_id.clone(),
        role: StructuralRelationRoleV4::Structural,
        intercept_parameter: intercept_parameter.map(str::to_owned),
    });
    model.parameters.push(free_parameter(
        &parameter_id,
        &format!("{source} to {target}"),
        SemParameterTargetV4::Regression {
            source: source.into(),
            target: target.into(),
        },
    ));
}

fn push_covariance(model: &mut SemModelV4, id: &str, left: SemEndpointV4, right: SemEndpointV4) {
    let parameter_id = format!("parameter:{id}");
    model.relations.push(SemRelationV4::Covariance {
        id: id.into(),
        left: left.clone(),
        right: right.clone(),
        parameter: parameter_id.clone(),
    });
    model.parameters.push(free_parameter(
        &parameter_id,
        id,
        SemParameterTargetV4::Covariance { left, right },
    ));
}

fn push_derived_effect(
    model: &mut SemModelV4,
    variable_id: &str,
    label: &str,
    term: SemDerivedTermV4,
    target: Option<&str>,
) {
    model.variables.push(SemVariableV4::Derived {
        id: variable_id.into(),
        label: label.into(),
    });
    if let Some(target) = target {
        push_structural(
            model,
            &format!("relation:{variable_id}:{target}"),
            variable_id,
            target,
            None,
        );
    }
    model.derived_terms.push(term);
}

fn basic_factor_model() -> SemModelV4 {
    let legacy = ModelSpec {
        id: Uuid::parse_str("00000000-0000-0000-0000-000000000421").unwrap(),
        name: "Section 3.1 authoring shapes".into(),
        constructs: ["x", "m", "y", "z"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect(),
        paths: vec![
            StructuralPath {
                source: "x".into(),
                target: "y".into(),
            },
            StructuralPath {
                source: "m".into(),
                target: "y".into(),
            },
            StructuralPath {
                source: "y".into(),
                target: "z".into(),
            },
        ],
        controls: Vec::new(),
        higher_order_constructs: Vec::new(),
        interactions: Vec::new(),
    };
    convert_legacy_basic_model_v4(
        &legacy,
        LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        &[],
    )
    .unwrap()
}

fn recursive_shape_model() -> SemModelV4 {
    let mut model = basic_factor_model();
    model.id = "sem-model-v4:section-3.1:recursive".into();
    model.name = "Section 3.1 recursive authoring fixture".into();

    // Explicit composite plus formative/causal measurement alongside the
    // reflective common-factor measurement emitted by the base conversion.
    model.variables.extend([
        observed(
            "observed:c1",
            "Composite indicator one",
            ObservedScaleV4::Continuous,
            ObservedRoleV4::Indicator,
            &[],
        ),
        observed(
            "observed:c2",
            "Composite indicator two",
            ObservedScaleV4::Continuous,
            ObservedRoleV4::Indicator,
            &[],
        ),
        SemVariableV4::Composite {
            id: "composite:c".into(),
            label: "Formative composite C".into(),
            weighting: CompositeWeightingV4::ModeB,
        },
    ]);
    for indicator in ["observed:c1", "observed:c2"] {
        let relation_id = format!("measurement:{indicator}:composite:c");
        let parameter_id = format!("parameter:{relation_id}");
        model.relations.push(SemRelationV4::MeasurementCausal {
            id: relation_id,
            indicator: indicator.into(),
            composite: "composite:c".into(),
            parameter: parameter_id.clone(),
        });
        model.parameters.push(free_parameter(
            &parameter_id,
            &format!("{indicator} weight"),
            SemParameterTargetV4::Weight {
                indicator: indicator.into(),
                composite: "composite:c".into(),
            },
        ));
    }

    // Observed structural predictor/outcomes plus observed and latent controls.
    model.variables.extend([
        observed(
            "observed:predictor",
            "Observed predictor",
            ObservedScaleV4::Continuous,
            ObservedRoleV4::Structural,
            &[],
        ),
        observed(
            "observed:outcome",
            "Observed outcome",
            ObservedScaleV4::Continuous,
            ObservedRoleV4::Structural,
            &[],
        ),
        observed(
            "observed:ordinal_outcome",
            "Ordinal outcome",
            ObservedScaleV4::Ordinal,
            ObservedRoleV4::Structural,
            &["low", "middle", "high"],
        ),
        observed(
            "observed:control",
            "Observed control",
            ObservedScaleV4::Continuous,
            ObservedRoleV4::Control,
            &[],
        ),
        observed(
            "observed:group",
            "Observed group",
            ObservedScaleV4::Nominal,
            ObservedRoleV4::Control,
            &["A", "B"],
        ),
        observed(
            "observed:sampling_weight",
            "Sampling weight",
            ObservedScaleV4::Continuous,
            ObservedRoleV4::Control,
            &[],
        ),
    ]);
    model.group = SemGroupV4::ObservedGroups {
        grouping_variable: "observed:group".into(),
        levels: vec![
            SemGroupLevelV4 {
                id: "group-a".into(),
                value: "A".into(),
                label: "Group A".into(),
            },
            SemGroupLevelV4 {
                id: "group-b".into(),
                value: "B".into(),
                label: "Group B".into(),
            },
        ],
    };
    if let qpls_core::SemDataBindingV4::Raw { weight, .. } = &mut model.data_binding {
        *weight = Some(SemWeightBindingV4::Sampling {
            variable: "observed:sampling_weight".into(),
            normalization: SamplingWeightNormalizationV4::MeanOne,
        });
    }

    let predictor_outcome_intercept = "parameter:intercept:observed-outcome";
    model.parameters.push(free_parameter(
        predictor_outcome_intercept,
        "Observed outcome intercept",
        SemParameterTargetV4::Intercept {
            variable: "observed:outcome".into(),
        },
    ));
    push_structural(
        &mut model,
        "relation:observed-predictor:y",
        "observed:predictor",
        "construct:y",
        None,
    );
    push_structural(
        &mut model,
        "relation:x:observed-outcome",
        "construct:x",
        "observed:outcome",
        Some(predictor_outcome_intercept),
    );
    push_structural(
        &mut model,
        "relation:x:ordinal-outcome",
        "construct:x",
        "observed:ordinal_outcome",
        None,
    );
    push_structural(
        &mut model,
        "relation:observed-control:y",
        "observed:control",
        "construct:y",
        None,
    );
    push_structural(
        &mut model,
        "relation:composite-c:y",
        "composite:c",
        "construct:y",
        None,
    );

    // Cross-loading is a scientific relation, not presentation metadata.
    model.relations.push(SemRelationV4::MeasurementEffect {
        id: "measurement:cross-loading:x:y1".into(),
        construct: "construct:x".into(),
        indicator: "observed:y1".into(),
        parameter: "parameter:cross-loading:x:y1".into(),
    });
    model.parameters.push(SemParameterV4::Fixed {
        id: "parameter:cross-loading:x:y1".into(),
        label: "Cross-loading X to y1".into(),
        target: SemParameterTargetV4::Loading {
            construct: "construct:x".into(),
            indicator: "observed:y1".into(),
        },
        value: 0.2,
        group_overrides: Vec::new(),
    });

    // Latent/observed, residual/error, and disturbance covariance identities.
    push_covariance(
        &mut model,
        "covariance:latent:x:m",
        SemEndpointV4::Variable("construct:x".into()),
        SemEndpointV4::Variable("construct:m".into()),
    );
    push_covariance(
        &mut model,
        "covariance:observed:predictor:outcome",
        SemEndpointV4::Variable("observed:predictor".into()),
        SemEndpointV4::Variable("observed:outcome".into()),
    );
    push_covariance(
        &mut model,
        "covariance:residual:x1:y1",
        SemEndpointV4::ResidualOf("observed:x1".into()),
        SemEndpointV4::ResidualOf("observed:y1".into()),
    );
    push_covariance(
        &mut model,
        "covariance:disturbance:y:z",
        SemEndpointV4::DisturbanceOf("construct:y".into()),
        SemEndpointV4::DisturbanceOf("construct:z".into()),
    );

    // Mean, regression intercept, observed intercept, ordinal thresholds,
    // start/bounds/equality labels, explicit equality/bound constraints, and
    // group-specific parameter overrides.
    model.parameters.push(SemParameterV4::Free {
        id: "parameter:mean:x".into(),
        label: "Mean X".into(),
        target: SemParameterTargetV4::Mean {
            variable: "construct:x".into(),
        },
        start: Some(0.0),
        lower: Some(-2.0),
        upper: Some(2.0),
        equality_label: Some("mean_x".into()),
        group_overrides: vec![
            SemParameterGroupOverrideV4 {
                group: "group-a".into(),
                specification: SemParameterGroupOverrideSpecV4::Fixed { value: 0.0 },
            },
            SemParameterGroupOverrideV4 {
                group: "group-b".into(),
                specification: SemParameterGroupOverrideSpecV4::Free {
                    start: Some(0.0),
                    lower: Some(-2.0),
                    upper: Some(2.0),
                },
            },
        ],
    });
    let x = model
        .variables
        .iter_mut()
        .find(|variable| variable.id() == "construct:x")
        .unwrap();
    if let SemVariableV4::CommonFactor { mean_policy, .. } = x {
        *mean_policy = FactorMeanPolicyV4::ReferenceGroup {
            reference_group: "group-a".into(),
            parameter: "parameter:mean:x".into(),
        };
    }

    let x_to_y = model
        .relations
        .iter_mut()
        .find(|relation| {
            matches!(relation, SemRelationV4::Structural { source, target, .. }
                if source == "construct:x" && target == "construct:y")
        })
        .unwrap();
    model.parameters.push(free_parameter(
        "parameter:intercept:y",
        "Regression intercept Y",
        SemParameterTargetV4::Intercept {
            variable: "construct:y".into(),
        },
    ));
    if let SemRelationV4::Structural {
        intercept_parameter,
        ..
    } = x_to_y
    {
        *intercept_parameter = Some("parameter:intercept:y".into());
    }
    for index in 0..2 {
        model.parameters.push(free_parameter(
            &format!("parameter:threshold:ordinal-outcome:{index}"),
            &format!("Ordinal outcome threshold {index}"),
            SemParameterTargetV4::Threshold {
                variable: "observed:ordinal_outcome".into(),
                index,
            },
        ));
    }

    let mut equality_parameters = Vec::new();
    for parameter in model.parameters.iter_mut().filter(|parameter| {
        matches!(
            parameter,
            SemParameterV4::Free {
                target: SemParameterTargetV4::Loading { .. },
                ..
            }
        )
    }) {
        if let SemParameterV4::Free {
            id,
            start,
            lower,
            upper,
            equality_label,
            ..
        } = parameter
        {
            *start = Some(0.7);
            *lower = Some(0.0);
            *upper = Some(1.0);
            *equality_label = Some("loading_equal".into());
            equality_parameters.push(id.clone());
            if equality_parameters.len() == 2 {
                break;
            }
        }
    }
    model.constraints.extend([
        SemConstraintV4::Equality {
            id: "constraint:equal-loadings".into(),
            parameters: equality_parameters.clone(),
        },
        SemConstraintV4::Bound {
            id: "constraint:bounded-loading".into(),
            parameter: equality_parameters[0].clone(),
            lower: Some(0.1),
            upper: Some(0.95),
        },
    ]);

    let focal_x_y = model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id, source, target, ..
            } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
            _ => None,
        })
        .unwrap();
    let focal_m_y = model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id, source, target, ..
            } if source == "construct:m" && target == "construct:y" => Some(id.clone()),
            _ => None,
        })
        .unwrap();
    push_derived_effect(
        &mut model,
        "derived:x-by-m",
        "X by M",
        SemDerivedTermV4::Interaction {
            id: "term:interaction:x:m".into(),
            output: "derived:x-by-m".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation: focal_x_y.clone(),
            method: InteractionMethodV4::TwoStage,
            product_indicator: None,
        },
        Some("construct:y"),
    );
    push_derived_effect(
        &mut model,
        "derived:m-by-x-product",
        "M by X product indicators",
        SemDerivedTermV4::Interaction {
            id: "term:interaction:m:x:product".into(),
            output: "derived:m-by-x-product".into(),
            predictor: "construct:m".into(),
            moderator: "construct:x".into(),
            focal_relation: focal_m_y,
            method: InteractionMethodV4::ProductIndicator,
            product_indicator: Some(ProductIndicatorSpecificationV4 {
                centering: ProductIndicatorCenteringV4::DoubleMeanCenter,
                standardization: ProductIndicatorStandardizationV4::SampleStandardDeviation,
                pairing: ProductIndicatorPairingV4::AllPairs,
            }),
        },
        Some("construct:y"),
    );
    let focal_xm_y = "relation:derived:x-by-m:construct:y";
    push_derived_effect(
        &mut model,
        "derived:x-by-m-by-c",
        "X by M by C",
        SemDerivedTermV4::Interaction {
            id: "term:interaction:x:m:c".into(),
            output: "derived:x-by-m-by-c".into(),
            predictor: "derived:x-by-m".into(),
            moderator: "composite:c".into(),
            focal_relation: focal_xm_y.into(),
            method: InteractionMethodV4::TwoStage,
            product_indicator: None,
        },
        Some("construct:y"),
    );
    push_derived_effect(
        &mut model,
        "derived:x-by-m-by-c-by-z",
        "X by M by C by Z",
        SemDerivedTermV4::Interaction {
            id: "term:interaction:x:m:c:z".into(),
            output: "derived:x-by-m-by-c-by-z".into(),
            predictor: "derived:x-by-m-by-c".into(),
            moderator: "construct:z".into(),
            focal_relation: "relation:derived:x-by-m-by-c:construct:y".into(),
            method: InteractionMethodV4::Orthogonalizing,
            product_indicator: None,
        },
        Some("construct:y"),
    );
    push_derived_effect(
        &mut model,
        "derived:x-square",
        "X squared",
        SemDerivedTermV4::Polynomial {
            id: "term:polynomial:x:2".into(),
            output: "derived:x-square".into(),
            source: "construct:x".into(),
            degree: 2,
        },
        Some("construct:y"),
    );
    push_derived_effect(
        &mut model,
        "derived:x-cube",
        "X cubed",
        SemDerivedTermV4::Polynomial {
            id: "term:polynomial:x:3".into(),
            output: "derived:x-cube".into(),
            source: "construct:x".into(),
            degree: 3,
        },
        Some("construct:z"),
    );

    let higher_order_shapes = [
        (
            "hoc:repeated:rr",
            HigherOrderConstructionApproachV4::RepeatedIndicators,
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
        ),
        (
            "hoc:extended:rf",
            HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
            HigherOrderMeasurementTypeV4::ReflectiveFormative,
        ),
        (
            "hoc:embedded:fr",
            HigherOrderConstructionApproachV4::EmbeddedTwoStage,
            HigherOrderMeasurementTypeV4::FormativeReflective,
        ),
        (
            "hoc:disjoint:ff",
            HigherOrderConstructionApproachV4::DisjointTwoStage,
            HigherOrderMeasurementTypeV4::FormativeFormative,
        ),
        (
            "hoc:hybrid:rr",
            HigherOrderConstructionApproachV4::Hybrid,
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
        ),
    ];
    for (id, approach, measurement_type) in higher_order_shapes {
        let output = format!("derived:{id}");
        push_derived_effect(
            &mut model,
            &output,
            id,
            SemDerivedTermV4::HigherOrder {
                id: format!("term:{id}"),
                output: output.clone(),
                components: vec!["construct:x".into(), "construct:m".into()],
                approach,
                measurement_type,
            },
            None,
        );
    }

    // Scientific relations remain separate from annotations and canvas-only
    // decorations. The scientific digest assertion below freezes that boundary.
    model.annotations.extend([
        SemAnnotationV4::DisplayOnlyCovariance {
            id: "annotation:display-covariance".into(),
            left: "construct:x".into(),
            right: "construct:m".into(),
            label: Some("Display only".into()),
        },
        SemAnnotationV4::Caption {
            id: "annotation:caption".into(),
            text: "Section 3.1 fixture".into(),
        },
        SemAnnotationV4::Note {
            id: "annotation:note".into(),
            subject: "construct:y".into(),
            text: "Scientific note".into(),
        },
    ]);
    model.presentation = SemPresentationV4::Canvas {
        nodes: vec![
            SemCanvasNodeV4 {
                variable: "construct:x".into(),
                x: 100.0,
                y: 120.0,
                style: BTreeMap::new(),
            },
            SemCanvasNodeV4 {
                variable: "construct:y".into(),
                x: 420.0,
                y: 120.0,
                style: BTreeMap::new(),
            },
        ],
        edges: vec![SemCanvasEdgeV4 {
            relation: focal_x_y,
            routing: Some("curved".into()),
        }],
        shapes: vec![SemCanvasShapeV4 {
            id: "presentation:shape".into(),
            shape: SemCanvasShapeKindV4::RoundedRectangle,
            x: 40.0,
            y: 40.0,
            width: 500.0,
            height: 240.0,
            label: Some("Model area".into()),
            style: BTreeMap::new(),
        }],
        images: vec![SemCanvasImageV4 {
            id: "presentation:image".into(),
            asset_ref: "project-asset:diagram-logo".into(),
            alt_text: "Diagram logo".into(),
            x: 20.0,
            y: 20.0,
            width: 48.0,
            height: 48.0,
            style: BTreeMap::new(),
        }],
        lines: vec![SemCanvasLineV4 {
            id: "presentation:line".into(),
            x1: 20.0,
            y1: 320.0,
            x2: 520.0,
            y2: 320.0,
            label: Some("Presentation only".into()),
            start_marker: None,
            end_marker: None,
            style: BTreeMap::new(),
        }],
        zoom: Some(1.1),
        pan_x: Some(12.0),
        pan_y: Some(-8.0),
    };

    model = model.canonicalized();
    model.ensure_valid().unwrap();
    model
}

fn nonrecursive_shape_model(recursive: &SemModelV4) -> SemModelV4 {
    let mut model = recursive.clone();
    model.id = "sem-model-v4:section-3.1:nonrecursive".into();
    model.name = "Section 3.1 identified nonrecursive authoring fixture".into();
    push_structural(
        &mut model,
        "relation:z:y:feedback",
        "construct:z",
        "construct:y",
        None,
    );
    model = model.canonicalized();
    model.ensure_valid().unwrap();
    assert!(model.has_structural_feedback());
    model
}

#[test]
fn section_3_1_shapes_author_serialize_and_reopen_through_standalone_schema6() {
    let recursive = recursive_shape_model();
    assert!(!recursive.has_structural_feedback());
    let nonrecursive = nonrecursive_shape_model(&recursive);

    let mut presentation_free = recursive.clone();
    presentation_free.annotations.clear();
    presentation_free.presentation = SemPresentationV4::None;
    assert_eq!(
        recursive.scientific_sha256().unwrap(),
        presentation_free.scientific_sha256().unwrap(),
        "annotations and presentation-only canvas objects must not become scientific authority",
    );
    assert_ne!(
        recursive.model_document_sha256().unwrap(),
        presentation_free.model_document_sha256().unwrap(),
    );

    let cbsem_plan = compile_cbsem_plan_v2(&nonrecursive).unwrap();
    assert!(cbsem_plan.has_feedback());
    let capability_issues = validate_cbsem_ml_v1_estimator_capability_v2(&cbsem_plan);
    let issue_codes = capability_issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<BTreeSet<_>>();
    // This exact unique-code snapshot keeps representation claims separate
    // from the bounded ML v1 execution surface. The model is retained intact;
    // no unsupported shape is silently removed or reinterpreted.
    assert_eq!(
        issue_codes,
        BTreeSet::from([
            "categorical_or_ordinal_variable",
            "causal_measurement",
            "composite",
            "composite_weight",
            "derived_terms",
            "derived_variable",
            "factor_mean_or_disturbance_policy",
            "mean_or_threshold_structure",
            "multigroup",
            "parameter_group_overrides",
            "structural_intercept",
            "sampling_weight_unsupported",
        ]),
    );
    assert!(matches!(
        compile_pls_plan_v2(&nonrecursive),
        Err(CompiledPlsPlanV2Error::StructuralFeedback)
    ));

    let created_at: DateTime<Utc> = "2026-08-15T14:00:00Z".parse().unwrap();
    let modified_at: DateTime<Utc> = "2026-08-15T14:01:00Z".parse().unwrap();
    let document = ProjectArchiveDocumentV6 {
        schema_version: PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: Uuid::parse_str("00000000-0000-0000-0000-000000000422").unwrap(),
        name: "Section 3.1 canonical author and reopen witness".into(),
        created_at,
        modified_at,
        datasets: Vec::new(),
        models: vec![
            ProjectModelRecordV6 {
                model_id: recursive.id.clone(),
                payload: ProjectModelPayloadV6::SemModelV4 {
                    scientific_sha256: recursive.scientific_sha256().unwrap(),
                    model: recursive.clone(),
                },
            },
            ProjectModelRecordV6 {
                model_id: nonrecursive.id.clone(),
                payload: ProjectModelPayloadV6::SemModelV4 {
                    scientific_sha256: nonrecursive.scientific_sha256().unwrap(),
                    model: nonrecursive.clone(),
                },
            },
        ],
        recipes: Vec::new(),
        historical_recipes: Vec::new(),
        layouts: BTreeMap::new(),
        historical_results: Vec::new(),
        canonical_result_documents: Vec::new(),
        origin: ProjectOriginV6::NewProject,
    };
    document.ensure_valid().unwrap();
    let expected_bytes = serialize_project_document_v6(&document).unwrap();

    let directory = tempdir().unwrap();
    let destination = directory
        .path()
        .join("section-3-1-authoring-witness.schema6.json");
    let receipt = write_project_document_v6_new(&destination, &document).unwrap();
    assert!(receipt.post_write_validated);
    assert_eq!(receipt.schema_version, PROJECT_ARCHIVE_SCHEMA_V6_VERSION);

    let reopened = read_project_document_v6(&destination).unwrap();
    assert_eq!(
        serialize_project_document_v6(&reopened).unwrap(),
        expected_bytes
    );
    assert_eq!(reopened.models, document.models);
    assert!(matches!(
        &reopened.models[0].payload,
        ProjectModelPayloadV6::SemModelV4 { model, .. }
            if !model.has_structural_feedback()
                && model.derived_terms.len() == 11
                && model.annotations.len() == 3
    ));
    assert!(matches!(
        &reopened.models[1].payload,
        ProjectModelPayloadV6::SemModelV4 { model, .. }
            if model.has_structural_feedback()
    ));
}
