use crate::{
    CapabilityCellReferenceV2, InteractionHierarchyPolicyV2, InteractionMethodV4, SemConstraintV4,
    SemDerivedTermV4, SemModelV4, SemParameterTargetV4, SemParameterV4, SemRelationV4,
    SemVariableV4, StructuralRelationRoleV4,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const COMPILED_PLS_THREE_WAY_INTERACTION_V1_VERSION: &str =
    "qpls.compiled-pls-three-way-interaction.v1";
pub const PLS_GENERAL_THREE_WAY_MODERATION_CAPABILITY_ID_V1: &str = "smartpls.moderation";
pub const PLS_GENERAL_THREE_WAY_MODERATION_POINT_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_three_way_moderation_point";
pub const PLS_GENERAL_THREE_WAY_MODERATION_POINT_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_three_way_moderation_point_v1";
pub const PLS_GENERAL_THREE_WAY_MODERATION_BOOTSTRAP_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_three_way_moderation_bootstrap";
pub const PLS_GENERAL_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1";

pub fn pls_general_three_way_moderation_point_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_THREE_WAY_MODERATION_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_THREE_WAY_MODERATION_POINT_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_THREE_WAY_MODERATION_POINT_CAPABILITY_VERSION_V1.into(),
    }
}

pub fn pls_general_three_way_moderation_bootstrap_capability_cell_v1() -> CapabilityCellReferenceV2
{
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_THREE_WAY_MODERATION_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_THREE_WAY_MODERATION_BOOTSTRAP_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_THREE_WAY_MODERATION_BOOTSTRAP_CAPABILITY_VERSION_V1.into(),
    }
}

/// Probe authority for a moderator in the bounded three-way cell. A binary
/// probe is admitted only when the scientific model binds the moderator to one
/// observed binary indicator whose authored categories are exactly 0 and 1.
/// All latent or multi-indicator moderator scores use standardized probes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompiledPlsThreeWayModeratorScaleV1 {
    ContinuousStandardized,
    BinaryZeroOne,
}

/// Exact compiled authority for the bounded v1 three-way cell. Operand order
/// is scientific: X, first moderator W, conditioning moderator Z.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsThreeWayInteractionV1 {
    contract_version: String,
    interaction_id: String,
    output_variable_id: String,
    operand_ids: [String; 3],
    outcome_id: String,
    focal_relation_id: String,
    interaction_effect_relation_id: String,
    interaction_effect_parameter_id: String,
    lower_order_interaction_ids: [String; 3],
    first_moderator_scale: CompiledPlsThreeWayModeratorScaleV1,
    second_moderator_scale: CompiledPlsThreeWayModeratorScaleV1,
    method: InteractionMethodV4,
    hierarchy_policy: InteractionHierarchyPolicyV2,
    generated_product_column_id: String,
}

impl CompiledPlsThreeWayInteractionV1 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }
    pub fn interaction_id(&self) -> &str {
        &self.interaction_id
    }
    pub fn output_variable_id(&self) -> &str {
        &self.output_variable_id
    }
    pub fn operand_ids(&self) -> &[String; 3] {
        &self.operand_ids
    }
    pub fn focal_predictor_id(&self) -> &str {
        &self.operand_ids[0]
    }
    pub fn first_moderator_id(&self) -> &str {
        &self.operand_ids[1]
    }
    pub fn second_moderator_id(&self) -> &str {
        &self.operand_ids[2]
    }
    pub fn outcome_id(&self) -> &str {
        &self.outcome_id
    }
    pub fn focal_relation_id(&self) -> &str {
        &self.focal_relation_id
    }
    pub fn interaction_effect_relation_id(&self) -> &str {
        &self.interaction_effect_relation_id
    }
    pub fn interaction_effect_parameter_id(&self) -> &str {
        &self.interaction_effect_parameter_id
    }
    pub fn lower_order_interaction_ids(&self) -> &[String; 3] {
        &self.lower_order_interaction_ids
    }
    pub fn first_moderator_scale(&self) -> CompiledPlsThreeWayModeratorScaleV1 {
        self.first_moderator_scale
    }
    pub fn second_moderator_scale(&self) -> CompiledPlsThreeWayModeratorScaleV1 {
        self.second_moderator_scale
    }
    pub fn method(&self) -> &InteractionMethodV4 {
        &self.method
    }
    pub fn hierarchy_policy(&self) -> InteractionHierarchyPolicyV2 {
        self.hierarchy_policy
    }
    pub fn generated_product_column_id(&self) -> &str {
        &self.generated_product_column_id
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledPlsThreeWayInteractionErrorV1 {
    #[error(transparent)]
    InvalidModel(#[from] crate::SemModelV4ValidationError),
    #[error("the bounded three-way cell supports exactly one three-way term; found {found}")]
    Cardinality { found: usize },
    #[error("interaction {interaction_id} requires exactly three ordered operands")]
    OperandOrder { interaction_id: String },
    #[error("interaction {interaction_id} requires two_stage construction and strong hierarchy")]
    UnsupportedPolicy { interaction_id: String },
    #[error("interaction {interaction_id} has unsupported product-indicator settings")]
    ProductIndicatorSpecification { interaction_id: String },
    #[error("interaction {interaction_id} has no exact focal structural relation")]
    FocalRelation { interaction_id: String },
    #[error("interaction {interaction_id} must have exactly one free effect path to {outcome_id}")]
    EffectPath {
        interaction_id: String,
        outcome_id: String,
    },
    #[error("interaction output {output_id} participates in unsupported relation {relation_id}")]
    OutputRelation {
        output_id: String,
        relation_id: String,
    },
    #[error(
        "interaction {interaction_id} effect parameter is not an unconstrained free regression"
    )]
    EffectParameter { interaction_id: String },
    #[error(
        "interaction {interaction_id} is missing an exact pairwise lower-order term for {operands:?}"
    )]
    LowerOrderMissing {
        interaction_id: String,
        operands: [String; 2],
    },
    #[error(
        "binary moderator {moderator_id} must be authored with exact 0/1 categories; found {categories:?}"
    )]
    BinaryModeratorCoding {
        moderator_id: String,
        categories: Vec<String>,
    },
    #[error("generated three-way product column id collides: {column_id}")]
    GeneratedProductColumnCollision { column_id: String },
}

pub fn compile_pls_three_way_interaction_v1(
    model: &SemModelV4,
) -> Result<Option<CompiledPlsThreeWayInteractionV1>, CompiledPlsThreeWayInteractionErrorV1> {
    model.ensure_valid()?;
    let terms = model
        .derived_terms
        .iter()
        .filter_map(|term| match term {
            SemDerivedTermV4::InteractionV2 { operands, .. } if operands.len() == 3 => Some(term),
            _ => None,
        })
        .collect::<Vec<_>>();
    if terms.len() > 1 {
        return Err(CompiledPlsThreeWayInteractionErrorV1::Cardinality { found: terms.len() });
    }
    let Some(SemDerivedTermV4::InteractionV2 {
        id,
        output,
        operands,
        focal_relation,
        method,
        hierarchy_policy,
        product_indicator,
    }) = terms.first().copied()
    else {
        return Ok(None);
    };
    if operands.len() != 3 {
        return Err(CompiledPlsThreeWayInteractionErrorV1::OperandOrder {
            interaction_id: id.clone(),
        });
    }
    if method != &InteractionMethodV4::TwoStage
        || *hierarchy_policy != InteractionHierarchyPolicyV2::Strong
    {
        return Err(CompiledPlsThreeWayInteractionErrorV1::UnsupportedPolicy {
            interaction_id: id.clone(),
        });
    }
    if product_indicator.is_some() {
        return Err(
            CompiledPlsThreeWayInteractionErrorV1::ProductIndicatorSpecification {
                interaction_id: id.clone(),
            },
        );
    }
    let (focal_predictor, outcome_id) = model
        .relations
        .iter()
        .find_map(|relation| match relation {
            SemRelationV4::Structural {
                id: relation_id,
                source,
                target,
                role: StructuralRelationRoleV4::Structural,
                ..
            } if relation_id == focal_relation => Some((source.clone(), target.clone())),
            _ => None,
        })
        .ok_or_else(|| CompiledPlsThreeWayInteractionErrorV1::FocalRelation {
            interaction_id: id.clone(),
        })?;
    if focal_predictor != operands[0] {
        return Err(CompiledPlsThreeWayInteractionErrorV1::FocalRelation {
            interaction_id: id.clone(),
        });
    }
    let effect_relations = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural {
                id: relation_id,
                source,
                target,
                parameter,
                role: StructuralRelationRoleV4::Structural,
                intercept_parameter,
            } if source == output && target == &outcome_id => Some((
                relation_id.clone(),
                parameter.clone(),
                intercept_parameter.clone(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    if effect_relations.len() != 1 || effect_relations[0].2.is_some() {
        return Err(CompiledPlsThreeWayInteractionErrorV1::EffectPath {
            interaction_id: id.clone(),
            outcome_id,
        });
    }
    let (effect_relation_id, effect_parameter_id, _) = effect_relations[0].clone();
    if let Some(relation) = model.relations.iter().find(|relation| {
        relation.id() != effect_relation_id && relation_references_variable(relation, output)
    }) {
        return Err(CompiledPlsThreeWayInteractionErrorV1::OutputRelation {
            output_id: output.clone(),
            relation_id: relation.id().into(),
        });
    }
    let parameter_valid = model
        .parameters
        .iter()
        .find(|parameter| parameter.id() == effect_parameter_id)
        .is_some_and(|parameter| {
            matches!(parameter, SemParameterV4::Free {
            target: SemParameterTargetV4::Regression { source, target },
            start: None, lower: None, upper: None, equality_label: None, group_overrides, ..
        } if source == output && target == &outcome_id && group_overrides.is_empty())
        });
    let constrained = model
        .constraints
        .iter()
        .any(|constraint| constraint_references_parameter(constraint, &effect_parameter_id));
    if !parameter_valid || constrained {
        return Err(CompiledPlsThreeWayInteractionErrorV1::EffectParameter {
            interaction_id: id.clone(),
        });
    }

    let pairs = [[0usize, 1usize], [0, 2], [1, 2]];
    let mut lower_order_interaction_ids = Vec::with_capacity(3);
    for pair in pairs {
        let required = BTreeSet::from([operands[pair[0]].as_str(), operands[pair[1]].as_str()]);
        let lower = model.derived_terms.iter().find_map(|candidate| match candidate {
            SemDerivedTermV4::InteractionV2 { id, output, operands: candidate_operands, method: InteractionMethodV4::TwoStage, hierarchy_policy: InteractionHierarchyPolicyV2::Strong, product_indicator: None, .. }
                if candidate_operands.len() == 2
                    && candidate_operands.iter().map(String::as_str).collect::<BTreeSet<_>>() == required
                    && model.relations.iter().any(|relation| matches!(relation, SemRelationV4::Structural { source, target, role: StructuralRelationRoleV4::Structural, .. } if source == output && target == &outcome_id)) => Some(id.clone()),
            _ => None,
        });
        lower_order_interaction_ids.push(lower.ok_or_else(|| {
            CompiledPlsThreeWayInteractionErrorV1::LowerOrderMissing {
                interaction_id: id.clone(),
                operands: [operands[pair[0]].clone(), operands[pair[1]].clone()],
            }
        })?);
    }
    lower_order_interaction_ids.sort();
    let lower_order_interaction_ids: [String; 3] = lower_order_interaction_ids
        .try_into()
        .expect("three lower-order terms");
    let first_moderator_scale = compiled_moderator_scale(model, &operands[1])?;
    let second_moderator_scale = compiled_moderator_scale(model, &operands[2])?;
    let generated_product_column_id = three_way_product_column_identity(
        id,
        output,
        operands,
        focal_relation,
        &effect_relation_id,
    );
    if model.variables.iter().any(|variable| matches!(variable, SemVariableV4::Observed { source_column, .. } if source_column == &generated_product_column_id)) {
        return Err(CompiledPlsThreeWayInteractionErrorV1::GeneratedProductColumnCollision { column_id: generated_product_column_id });
    }
    Ok(Some(CompiledPlsThreeWayInteractionV1 {
        contract_version: COMPILED_PLS_THREE_WAY_INTERACTION_V1_VERSION.into(),
        interaction_id: id.clone(),
        output_variable_id: output.clone(),
        operand_ids: [
            operands[0].clone(),
            operands[1].clone(),
            operands[2].clone(),
        ],
        outcome_id,
        focal_relation_id: focal_relation.clone(),
        interaction_effect_relation_id: effect_relation_id,
        interaction_effect_parameter_id: effect_parameter_id,
        lower_order_interaction_ids,
        first_moderator_scale,
        second_moderator_scale,
        method: method.clone(),
        hierarchy_policy: *hierarchy_policy,
        generated_product_column_id,
    }))
}

fn compiled_moderator_scale(
    model: &SemModelV4,
    moderator_id: &str,
) -> Result<CompiledPlsThreeWayModeratorScaleV1, CompiledPlsThreeWayInteractionErrorV1> {
    let indicator_ids = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::MeasurementEffect {
                construct,
                indicator,
                ..
            } if construct == moderator_id => Some(indicator.as_str()),
            SemRelationV4::MeasurementCausal {
                indicator,
                composite,
                ..
            } if composite == moderator_id => Some(indicator.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let observed_id = if indicator_ids.len() == 1 {
        indicator_ids[0]
    } else {
        moderator_id
    };
    let Some(SemVariableV4::Observed {
        scale, categories, ..
    }) = model
        .variables
        .iter()
        .find(|variable| variable.id() == observed_id)
    else {
        return Ok(CompiledPlsThreeWayModeratorScaleV1::ContinuousStandardized);
    };
    if *scale != crate::ObservedScaleV4::Binary || indicator_ids.len() > 1 {
        return Ok(CompiledPlsThreeWayModeratorScaleV1::ContinuousStandardized);
    }
    let mut numeric_categories = categories
        .iter()
        .filter_map(|category| category.trim().parse::<f64>().ok())
        .collect::<Vec<_>>();
    numeric_categories.sort_by(f64::total_cmp);
    if numeric_categories.len() != 2
        || numeric_categories[0].to_bits() != 0.0_f64.to_bits()
        || numeric_categories[1].to_bits() != 1.0_f64.to_bits()
    {
        return Err(
            CompiledPlsThreeWayInteractionErrorV1::BinaryModeratorCoding {
                moderator_id: moderator_id.into(),
                categories: categories.clone(),
            },
        );
    }
    Ok(CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne)
}

fn relation_references_variable(relation: &SemRelationV4, variable_id: &str) -> bool {
    match relation {
        SemRelationV4::MeasurementEffect {
            construct,
            indicator,
            ..
        }
        | SemRelationV4::MeasurementCausal {
            indicator,
            composite: construct,
            ..
        } => construct == variable_id || indicator == variable_id,
        SemRelationV4::Structural { source, target, .. } => {
            source == variable_id || target == variable_id
        }
        SemRelationV4::Covariance { left, right, .. } => {
            left.variable_id() == variable_id || right.variable_id() == variable_id
        }
    }
}

fn constraint_references_parameter(constraint: &SemConstraintV4, parameter_id: &str) -> bool {
    match constraint {
        SemConstraintV4::Equality { parameters, .. } => {
            parameters.iter().any(|id| id == parameter_id)
        }
        SemConstraintV4::Bound { parameter, .. } => parameter == parameter_id,
        SemConstraintV4::Linear { terms, .. } => {
            terms.iter().any(|term| term.parameter == parameter_id)
        }
    }
}

fn three_way_product_column_identity(
    interaction_id: &str,
    output_id: &str,
    operands: &[String],
    focal_relation_id: &str,
    effect_relation_id: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-pls-plan-v3.three-way-product\0");
    for value in [
        interaction_id,
        output_id,
        &operands[0],
        &operands[1],
        &operands[2],
        focal_relation_id,
        effect_relation_id,
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    format!("qpls_pls_three_way_product_v1_{:x}", digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_way_cells_are_distinct_and_keep_point_as_bootstrap_dependency() {
        let point = pls_general_three_way_moderation_point_capability_cell_v1();
        let bootstrap = pls_general_three_way_moderation_bootstrap_capability_cell_v1();
        assert_ne!(point.cell_id, bootstrap.cell_id);
        assert_ne!(point.capability_version, bootstrap.capability_version);
        assert_eq!(point.capability_id, bootstrap.capability_id);
        assert_eq!(point.registry_schema_version, 2);
    }

    #[test]
    fn moderator_scale_wire_is_explicit() {
        assert_eq!(
            serde_json::to_string(&CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne).unwrap(),
            "\"binary_zero_one\"",
        );
    }
}
