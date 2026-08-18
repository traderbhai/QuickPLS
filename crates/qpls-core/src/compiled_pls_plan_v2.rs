use crate::{
    CompositeWeightNormalizationV4, CompositeWeightingV4, MissingDataPolicyV4, ObservedScaleV4,
    SemDataBindingV4, SemGroupV4, SemModelV4, SemModelV4ValidationError, SemRelationV4,
    SemVariableV4, StructuralRelationRoleV4, WeightCapabilityIssueV1, WeightCapabilityTargetV1,
    WeightDeclarationResolutionErrorV1, resolve_weight_declaration_v1,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompiledPlsBlockModeV2 {
    ModeA,
    ModeB,
}

/// Fixed construct-score semantics copied from the authoritative SemModelV4.
/// Absence means that the block weights are estimated with `mode`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledPlsFixedScoringV2 {
    Unit {
        normalization: CompositeWeightNormalizationV4,
    },
    Custom {
        weights: BTreeMap<String, f64>,
        normalization: CompositeWeightNormalizationV4,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsIndicatorV2 {
    variable_id: String,
    source_column: String,
    parameter_id: String,
}

impl CompiledPlsIndicatorV2 {
    pub fn variable_id(&self) -> &str {
        &self.variable_id
    }

    pub fn source_column(&self) -> &str {
        &self.source_column
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsBlockV2 {
    construct_id: String,
    mode: CompiledPlsBlockModeV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fixed_scoring: Option<CompiledPlsFixedScoringV2>,
    indicators: Vec<CompiledPlsIndicatorV2>,
}

impl CompiledPlsBlockV2 {
    pub fn construct_id(&self) -> &str {
        &self.construct_id
    }

    pub fn mode(&self) -> CompiledPlsBlockModeV2 {
        self.mode
    }

    pub fn fixed_scoring(&self) -> Option<&CompiledPlsFixedScoringV2> {
        self.fixed_scoring.as_ref()
    }

    pub fn indicators(&self) -> &[CompiledPlsIndicatorV2] {
        &self.indicators
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsPathV2 {
    relation_id: String,
    source: String,
    target: String,
    parameter_id: String,
    #[serde(
        default,
        skip_serializing_if = "StructuralRelationRoleV4::is_structural"
    )]
    role: StructuralRelationRoleV4,
}

impl CompiledPlsPathV2 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn parameter_id(&self) -> &str {
        &self.parameter_id
    }

    pub fn role(&self) -> StructuralRelationRoleV4 {
        self.role
    }
}

/// Immutable, estimator-ready plan for the intentionally basic PLS v2 slice.
/// Unsupported semantics are rejected during compilation rather than dropped.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsPlanV2 {
    model_id: String,
    scientific_hash: String,
    dataset_id: String,
    blocks: Vec<CompiledPlsBlockV2>,
    paths: Vec<CompiledPlsPathV2>,
}

impl CompiledPlsPlanV2 {
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn scientific_hash(&self) -> &str {
        &self.scientific_hash
    }

    pub fn dataset_id(&self) -> &str {
        &self.dataset_id
    }

    pub fn blocks(&self) -> &[CompiledPlsBlockV2] {
        &self.blocks
    }

    pub fn paths(&self) -> &[CompiledPlsPathV2] {
        &self.paths
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledPlsPlanV2Error {
    #[error(transparent)]
    InvalidModel(#[from] SemModelV4ValidationError),
    #[error(transparent)]
    WeightDeclaration(#[from] WeightDeclarationResolutionErrorV1),
    #[error(transparent)]
    UnsupportedWeight(#[from] WeightCapabilityIssueV1),
    #[error("PLS v2 basic compiler does not support {code}: {subject}")]
    Unsupported { code: String, subject: String },
    #[error("PLS v2 basic compiler requires an acyclic structural model")]
    StructuralFeedback,
}

pub fn compile_pls_plan_v2(
    model: &SemModelV4,
) -> Result<CompiledPlsPlanV2, CompiledPlsPlanV2Error> {
    model.ensure_valid()?;
    if model.has_structural_feedback() {
        return Err(CompiledPlsPlanV2Error::StructuralFeedback);
    }
    if !matches!(model.group, SemGroupV4::SingleGroup) {
        return Err(unsupported("multigroup", "group"));
    }
    if !model.constraints.is_empty() {
        return Err(unsupported(
            "parameter_constraints",
            model.constraints[0].id(),
        ));
    }
    if !model.derived_terms.is_empty() {
        return Err(unsupported("derived_terms", model.derived_terms[0].id()));
    }
    if let Some(parameter) = model
        .parameters
        .iter()
        .find(|parameter| !parameter.group_overrides().is_empty())
    {
        return Err(unsupported("parameter_group_overrides", parameter.id()));
    }

    let relation_parameter_ids = model
        .relations
        .iter()
        .map(SemRelationV4::parameter)
        .collect::<HashSet<_>>();
    for parameter in &model.parameters {
        if !relation_parameter_ids.contains(parameter.id()) {
            return Err(unsupported("unattached_parameter", parameter.id()));
        }
        match parameter {
            crate::SemParameterV4::Free {
                start: None,
                lower: None,
                upper: None,
                equality_label: None,
                ..
            } => {}
            crate::SemParameterV4::Free { .. } => {
                return Err(unsupported("parameter_options", parameter.id()));
            }
            crate::SemParameterV4::Fixed { .. } => {
                return Err(unsupported("fixed_parameter", parameter.id()));
            }
            crate::SemParameterV4::Derived { .. } => {
                return Err(unsupported("derived_parameter", parameter.id()));
            }
        }
    }

    let dataset_id = match &model.data_binding {
        SemDataBindingV4::Raw {
            dataset_id,
            missing_data: MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        } => dataset_id.clone(),
        SemDataBindingV4::Raw {
            weight: Some(_), ..
        } => {
            let declaration = resolve_weight_declaration_v1(model)?
                .ok_or_else(|| unsupported("raw_data_options", "data_binding.weight"))?;
            return Err(WeightCapabilityIssueV1::unsupported(
                WeightCapabilityTargetV1::PlsPlanV2,
                declaration,
            )
            .into());
        }
        SemDataBindingV4::Raw { .. } => {
            return Err(unsupported("raw_data_options", "data_binding"));
        }
        SemDataBindingV4::Covariance { .. } | SemDataBindingV4::Correlation { .. } => {
            return Err(unsupported("matrix_input", "data_binding"));
        }
    };

    let variables = model
        .variables
        .iter()
        .map(|variable| (variable.id(), variable))
        .collect::<HashMap<_, _>>();
    for variable in &model.variables {
        match variable {
            SemVariableV4::Observed {
                id,
                scale: ObservedScaleV4::Continuous,
                missing_markers,
                transformation_lineage,
                ..
            } => {
                if !missing_markers.is_empty() || !transformation_lineage.is_empty() {
                    return Err(unsupported("observed_data_metadata", id));
                }
                if model.relations.iter().any(|relation| {
                    matches!(relation, SemRelationV4::Structural { source, target, .. } if source == id || target == id)
                }) {
                    return Err(unsupported("observed_structural_variable", id));
                }
            }
            SemVariableV4::Observed { id, .. } => {
                return Err(unsupported("categorical_or_ordinal_variable", id));
            }
            SemVariableV4::Composite { .. } => {}
            SemVariableV4::CommonFactor { id, .. } => {
                return Err(unsupported("common_factor", id));
            }
            SemVariableV4::Derived { id, .. } => {
                return Err(unsupported("derived_variable", id));
            }
        }
    }

    let modeled_indicator_ids = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::MeasurementEffect { indicator, .. }
            | SemRelationV4::MeasurementCausal { indicator, .. } => Some(indicator.as_str()),
            SemRelationV4::Structural { .. } | SemRelationV4::Covariance { .. } => None,
        })
        .collect::<HashSet<_>>();
    if let Some(variable) = model.variables.iter().find(|variable| {
        matches!(variable, SemVariableV4::Observed { id, .. } if !modeled_indicator_ids.contains(id.as_str()))
    }) {
        return Err(unsupported("unmodeled_observed_variable", variable.id()));
    }

    if let Some(covariance) = model
        .relations
        .iter()
        .find(|relation| matches!(relation, SemRelationV4::Covariance { .. }))
    {
        return Err(unsupported("scientific_covariance", covariance.id()));
    }

    let mut blocks = Vec::new();
    for variable in &model.variables {
        let SemVariableV4::Composite { id, weighting, .. } = variable else {
            continue;
        };
        let (mode, fixed_scoring) = match weighting {
            CompositeWeightingV4::ModeA => (CompiledPlsBlockModeV2::ModeA, None),
            CompositeWeightingV4::ModeB => (CompiledPlsBlockModeV2::ModeB, None),
            CompositeWeightingV4::Unit { normalization } => (
                CompiledPlsBlockModeV2::ModeB,
                Some(CompiledPlsFixedScoringV2::Unit {
                    normalization: *normalization,
                }),
            ),
            CompositeWeightingV4::Custom {
                weights,
                normalization,
            } => (
                CompiledPlsBlockModeV2::ModeB,
                Some(CompiledPlsFixedScoringV2::Custom {
                    weights: weights.clone(),
                    normalization: *normalization,
                }),
            ),
        };
        let mut indicators = model
            .relations
            .iter()
            .filter_map(|relation| match (mode, relation) {
                (
                    CompiledPlsBlockModeV2::ModeA,
                    SemRelationV4::MeasurementEffect {
                        construct,
                        indicator,
                        parameter,
                        ..
                    },
                ) if construct == id => Some((indicator, parameter)),
                (
                    CompiledPlsBlockModeV2::ModeB,
                    SemRelationV4::MeasurementCausal {
                        indicator,
                        composite,
                        parameter,
                        ..
                    },
                ) if composite == id => Some((indicator, parameter)),
                _ => None,
            })
            .map(|(indicator, parameter)| {
                let SemVariableV4::Observed { source_column, .. } = variables[indicator.as_str()]
                else {
                    unreachable!("validated measurement indicator is observed")
                };
                CompiledPlsIndicatorV2 {
                    variable_id: indicator.clone(),
                    source_column: source_column.clone(),
                    parameter_id: parameter.clone(),
                }
            })
            .collect::<Vec<_>>();
        indicators.sort_by(|left, right| left.variable_id.cmp(&right.variable_id));
        blocks.push(CompiledPlsBlockV2 {
            construct_id: id.clone(),
            mode,
            fixed_scoring,
            indicators,
        });
    }
    blocks.sort_by(|left, right| left.construct_id.cmp(&right.construct_id));

    let mut source_columns = HashSet::new();
    if let Some(indicator) = blocks
        .iter()
        .flat_map(|block| block.indicators.iter())
        .find(|indicator| !source_columns.insert(indicator.source_column.as_str()))
    {
        return Err(unsupported(
            "shared_indicator_source_column",
            indicator.source_column.clone(),
        ));
    }

    let mut paths = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural {
                id,
                source,
                target,
                parameter,
                role,
                intercept_parameter,
            } => Some((id, source, target, parameter, role, intercept_parameter)),
            _ => None,
        })
        .map(|(id, source, target, parameter, role, intercept)| {
            if intercept.is_some() {
                return Err(unsupported("structural_intercept", id));
            }
            if !matches!(variables[source.as_str()], SemVariableV4::Composite { .. })
                || !matches!(variables[target.as_str()], SemVariableV4::Composite { .. })
            {
                return Err(unsupported("non_composite_structural_path", id));
            }
            Ok(CompiledPlsPathV2 {
                relation_id: id.clone(),
                source: source.clone(),
                target: target.clone(),
                parameter_id: parameter.clone(),
                role: *role,
            })
        })
        .collect::<Result<Vec<_>, CompiledPlsPlanV2Error>>()?;
    paths.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));

    Ok(CompiledPlsPlanV2 {
        model_id: model.id.clone(),
        scientific_hash: model.scientific_sha256()?,
        dataset_id,
        blocks,
        paths,
    })
}

fn unsupported(code: impl Into<String>, subject: impl Into<String>) -> CompiledPlsPlanV2Error {
    CompiledPlsPlanV2Error::Unsupported {
        code: code.into(),
        subject: subject.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, ObservedRoleV4,
        SamplingWeightNormalizationV4, SemParameterTargetV4, SemParameterV4, SemWeightBindingV4,
        StructuralPath, WeightCapabilityCodeV1, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn legacy() -> ModelSpec {
        ModelSpec {
            id: Uuid::nil(),
            name: "PLS".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: MeasurementMode::Formative,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: vec![],
            higher_order_constructs: vec![],
            interactions: vec![],
        }
    }

    fn weighted_model(weight: SemWeightBindingV4) -> SemModelV4 {
        let mut model = convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:weight".into(),
            label: "Weight".into(),
            source_column: "case_weight".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        let SemDataBindingV4::Raw {
            weight: configured, ..
        } = &mut model.data_binding
        else {
            unreachable!()
        };
        *configured = Some(weight);
        model.ensure_valid().unwrap();
        model
    }

    #[test]
    fn compiles_mixed_basic_blocks_deterministically() {
        let model = convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let plan = compile_pls_plan_v2(&model).unwrap();
        assert_eq!(plan.blocks().len(), 2);
        assert_eq!(plan.blocks()[0].mode(), CompiledPlsBlockModeV2::ModeA);
        assert_eq!(plan.blocks()[0].fixed_scoring(), None);
        assert_eq!(plan.blocks()[1].mode(), CompiledPlsBlockModeV2::ModeB);
        assert_eq!(plan.blocks()[1].fixed_scoring(), None);
        assert_eq!(plan.paths().len(), 1);
    }

    #[test]
    fn fixed_composite_semantics_are_preserved_exactly_in_the_plan() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let SemVariableV4::Composite { weighting, .. } = model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "construct:y")
            .unwrap()
        else {
            unreachable!("legacy formative construct is composite")
        };
        *weighting = CompositeWeightingV4::Custom {
            weights: BTreeMap::from([("observed:y1".into(), -0.25), ("observed:y2".into(), 0.75)]),
            normalization: CompositeWeightNormalizationV4::UnitVariance,
        };

        let plan = compile_pls_plan_v2(&model).unwrap();
        let block = plan
            .blocks()
            .iter()
            .find(|block| block.construct_id() == "construct:y")
            .unwrap();
        assert_eq!(
            block.fixed_scoring(),
            Some(&CompiledPlsFixedScoringV2::Custom {
                weights: BTreeMap::from([
                    ("observed:y1".into(), -0.25),
                    ("observed:y2".into(), 0.75),
                ]),
                normalization: CompositeWeightNormalizationV4::UnitVariance,
            })
        );

        *model
            .variables
            .iter_mut()
            .find_map(|variable| match variable {
                SemVariableV4::Composite { id, weighting, .. } if id == "construct:y" => {
                    Some(weighting)
                }
                _ => None,
            })
            .unwrap() = CompositeWeightingV4::Unit {
            normalization: CompositeWeightNormalizationV4::None,
        };
        let unit_plan = compile_pls_plan_v2(&model).unwrap();
        assert_eq!(
            unit_plan
                .blocks()
                .iter()
                .find(|block| block.construct_id() == "construct:y")
                .unwrap()
                .fixed_scoring(),
            Some(&CompiledPlsFixedScoringV2::Unit {
                normalization: CompositeWeightNormalizationV4::None,
            })
        );
    }

    #[test]
    fn legacy_plan_json_without_fixed_scoring_remains_readable() {
        let model = convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        let plan = compile_pls_plan_v2(&model).unwrap();
        let value = serde_json::to_value(&plan).unwrap();
        for block in value["blocks"].as_array().unwrap() {
            assert!(block.get("fixed_scoring").is_none());
        }
        let decoded: CompiledPlsPlanV2 = serde_json::from_value(value).unwrap();
        assert_eq!(decoded, plan);
    }

    #[test]
    fn feedback_is_representable_in_ir_but_rejected_by_pls_compiler() {
        let mut model = convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        model.relations.push(SemRelationV4::Structural {
            id: "feedback".into(),
            source: "construct:y".into(),
            target: "construct:x".into(),
            parameter: "feedback-p".into(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "feedback-p".into(),
            label: "Y -> X".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:y".into(),
                target: "construct:x".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        assert!(model.validate().is_empty());
        assert_eq!(
            compile_pls_plan_v2(&model),
            Err(CompiledPlsPlanV2Error::StructuralFeedback)
        );
    }

    #[test]
    fn parameter_semantics_that_the_basic_estimator_cannot_honor_fail_closed() {
        let model = convert_legacy_basic_model_v4(
            &legacy(),
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();

        let mut with_start_value = model.clone();
        let SemParameterV4::Free { start, .. } = &mut with_start_value.parameters[0] else {
            unreachable!("legacy PLS parameters are free")
        };
        *start = Some(0.25);
        with_start_value.ensure_valid().unwrap();
        assert!(matches!(
            compile_pls_plan_v2(&with_start_value),
            Err(CompiledPlsPlanV2Error::Unsupported { code, .. }) if code == "parameter_options"
        ));

        let mut with_fixed_path = model.clone();
        let parameter = with_fixed_path.parameters.pop().unwrap();
        let SemParameterV4::Free {
            id,
            label,
            target,
            group_overrides,
            ..
        } = parameter
        else {
            unreachable!("legacy PLS parameters are free")
        };
        with_fixed_path.parameters.push(SemParameterV4::Fixed {
            id,
            label,
            target,
            value: 0.5,
            group_overrides,
        });
        with_fixed_path.ensure_valid().unwrap();
        assert!(matches!(
            compile_pls_plan_v2(&with_fixed_path),
            Err(CompiledPlsPlanV2Error::Unsupported { code, .. }) if code == "fixed_parameter"
        ));
    }

    #[test]
    fn weight_kinds_fail_closed_with_exact_typed_declarations() {
        let cases = [
            (
                SemWeightBindingV4::Case {
                    variable: "observed:weight".into(),
                },
                WeightCapabilityCodeV1::CaseWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Frequency {
                    variable: "observed:weight".into(),
                },
                WeightCapabilityCodeV1::FrequencyWeightUnsupported,
            ),
            (
                SemWeightBindingV4::Sampling {
                    variable: "observed:weight".into(),
                    normalization: SamplingWeightNormalizationV4::SumToSampleSize,
                },
                WeightCapabilityCodeV1::SamplingWeightUnsupported,
            ),
        ];
        for (binding, expected_code) in cases {
            let error = compile_pls_plan_v2(&weighted_model(binding)).unwrap_err();
            let CompiledPlsPlanV2Error::UnsupportedWeight(issue) = error else {
                panic!("expected a typed weight capability issue, found {error:?}")
            };
            assert_eq!(issue.code, expected_code);
            assert_eq!(issue.target, WeightCapabilityTargetV1::PlsPlanV2);
            let declaration = issue.declaration.as_ref().unwrap();
            assert_eq!(declaration.dataset_id(), "legacy-unbound");
            assert_eq!(declaration.binding().variable_id(), "observed:weight");
            assert_eq!(declaration.binding().source_column(), "case_weight");
        }

        let mut unrelated = weighted_model(SemWeightBindingV4::Case {
            variable: "observed:weight".into(),
        });
        let SemDataBindingV4::Raw {
            weight,
            cluster_variable,
            ..
        } = &mut unrelated.data_binding
        else {
            unreachable!()
        };
        *weight = None;
        *cluster_variable = Some("observed:weight".into());
        assert!(matches!(
            compile_pls_plan_v2(&unrelated),
            Err(CompiledPlsPlanV2Error::Unsupported { code, .. }) if code == "raw_data_options"
        ));
    }
}
