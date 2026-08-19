use crate::{
    CompiledPlsPlanV2, CompiledPlsPlanV2Error, CompiledPlsTwoWayModeratedMediationTargetErrorV1,
    CompiledPlsTwoWayModeratedMediationTargetV1, CompiledSemSpecificDirectedPathV1,
    CompiledSemTopologyV1, CompiledSemTopologyV1Error, GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
    GeneralSemConfigV1, GeneralSemConfigV1ValidationError, GeneralSemEffectEstimandV1,
    GeneralSemSpecificPathLimitBehaviorV1, InteractionHierarchyPolicyV2, InteractionMethodV4,
    SemConstraintV4, SemDerivedTermV4, SemModelV4, SemParameterTargetV4, SemParameterV4,
    SemPresentationV4, SemRelationV4, SemVariableV4, StructuralRelationRoleV4, compile_pls_plan_v2,
    compile_pls_two_way_moderated_mediation_target_v1, compile_sem_topology_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const COMPILED_PLS_PLAN_V3_SCHEMA_VERSION: u32 = 3;
pub const COMPILED_PLS_TWO_WAY_INTERACTION_V3_VERSION: &str =
    "qpls.compiled-pls-two-way-interaction.v3.1";
pub const COMPILED_PLS_STAGE_ONE_PROJECTION_V3_VERSION: &str =
    "qpls.compiled-pls-stage-one-projection.v3.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FixedTwoWayProductConstructionPolicyV3 {
    TwoStage,
}

/// Product columns are commutative even though moderation roles are not. This
/// compiler-only signature prevents two fixed-cell interactions from asking
/// the executor to solve the same product column twice while the compiled
/// interaction contracts retain their authored focal/moderator order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct FixedTwoWayProductDesignSignatureV3 {
    sorted_operand_ids: [String; 2],
    outcome_id: String,
    construction_policy: FixedTwoWayProductConstructionPolicyV3,
    scale_policy_version: &'static str,
}

impl FixedTwoWayProductDesignSignatureV3 {
    fn for_qualified_cell(operands: &[String], outcome_id: &str) -> Self {
        debug_assert_eq!(operands.len(), 2);
        let mut sorted_operand_ids = [operands[0].clone(), operands[1].clone()];
        sorted_operand_ids.sort();
        Self {
            sorted_operand_ids,
            outcome_id: outcome_id.to_string(),
            construction_policy: FixedTwoWayProductConstructionPolicyV3::TwoStage,
            scale_policy_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
        }
    }
}

/// Deterministic executor contract for the first qualified interaction cell:
/// two operands, two-stage scoring, and strong hierarchy. The first operand is
/// the focal predictor and the second is the moderator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsTwoWayInteractionV3 {
    contract_version: String,
    interaction_id: String,
    output_variable_id: String,
    focal_predictor_id: String,
    moderator_id: String,
    outcome_id: String,
    focal_relation_id: String,
    interaction_effect_relation_id: String,
    interaction_effect_parameter_id: String,
    method: InteractionMethodV4,
    hierarchy_policy: InteractionHierarchyPolicyV2,
    generated_product_column_id: String,
}

impl CompiledPlsTwoWayInteractionV3 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub fn interaction_id(&self) -> &str {
        &self.interaction_id
    }

    pub fn output_variable_id(&self) -> &str {
        &self.output_variable_id
    }

    pub fn focal_predictor_id(&self) -> &str {
        &self.focal_predictor_id
    }

    pub fn moderator_id(&self) -> &str {
        &self.moderator_id
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

/// Lossless receipt for the scientific projection used to estimate stage-one
/// construct scores. The source digest identifies the complete model; the
/// projected digest identifies the ordinary PLS plan with derived outputs and
/// their effect paths removed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsStageOneProjectionV3 {
    contract_version: String,
    source_scientific_sha256: String,
    projected_scientific_sha256: String,
    projected_model: SemModelV4,
}

impl CompiledPlsStageOneProjectionV3 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub fn source_scientific_sha256(&self) -> &str {
        &self.source_scientific_sha256
    }

    pub fn projected_scientific_sha256(&self) -> &str {
        &self.projected_scientific_sha256
    }

    pub fn projected_model(&self) -> &SemModelV4 {
        &self.projected_model
    }

    pub fn deterministic_sha256(&self) -> String {
        sha256_serialized(self)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledPlsInteractionV3Error {
    #[error(transparent)]
    InvalidModel(#[from] crate::SemModelV4ValidationError),
    #[error("PLS v3 interaction cell does not support derived term {term_id} ({kind})")]
    UnsupportedDerivedTerm { term_id: String, kind: &'static str },
    #[error("interaction {interaction_id} requires exactly two operands; received {operand_count}")]
    UnsupportedInteractionOrder {
        interaction_id: String,
        operand_count: usize,
    },
    #[error("interaction {interaction_id} requires the two_stage construction method")]
    UnsupportedInteractionMethod { interaction_id: String },
    #[error("interaction {interaction_id} requires strong hierarchy")]
    UnsupportedInteractionHierarchy { interaction_id: String },
    #[error("interaction {interaction_id} has unsupported product-indicator settings")]
    UnsupportedInteractionProductSpecification { interaction_id: String },
    #[error(
        "interaction {interaction_id} must have exactly one structural-effect path from {output_id} to {outcome_id}; received {relation_count}"
    )]
    InteractionEffectRelationCardinality {
        interaction_id: String,
        output_id: String,
        outcome_id: String,
        relation_count: usize,
    },
    #[error("interaction output {output_id} participates in unsupported relation {relation_id}")]
    InteractionOutputRelationUnsupported {
        output_id: String,
        relation_id: String,
    },
    #[error(
        "interaction {interaction_id} effect parameter {parameter_id} must be an unconstrained free regression parameter"
    )]
    UnsupportedInteractionEffectParameter {
        interaction_id: String,
        parameter_id: String,
    },
    #[error(
        "interaction {interaction_id} effect parameter {parameter_id} participates in constraint {constraint_id}"
    )]
    UnsupportedInteractionEffectConstraint {
        interaction_id: String,
        parameter_id: String,
        constraint_id: String,
    },
    #[error(
        "interactions {first_interaction_id} and {second_interaction_id} compile to the same fixed two-way product design for operands {sorted_operand_ids:?} and outcome {outcome_id}"
    )]
    DuplicateInteractionProductDesign {
        first_interaction_id: String,
        second_interaction_id: String,
        sorted_operand_ids: [String; 2],
        outcome_id: String,
    },
    #[error("generated interaction product column id collides: {column_id}")]
    GeneratedProductColumnCollision { column_id: String },
}

/// Stable effect identities compiled from ordinary directed structural paths.
/// Mediation remains a topology-derived estimand rather than a special model
/// object. Every contributing relation and path remains inspectable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledPlsEffectEstimandV3 {
    SpecificIndirect {
        estimand_id: String,
        path_identity: String,
        source_id: String,
        target_id: String,
        ordered_relation_ids: Vec<String>,
    },
    TotalIndirect {
        estimand_id: String,
        source_id: String,
        target_id: String,
        contributing_path_identities: Vec<String>,
    },
    TotalEffect {
        estimand_id: String,
        source_id: String,
        target_id: String,
        direct_relation_ids: Vec<String>,
        contributing_indirect_path_identities: Vec<String>,
    },
}

impl CompiledPlsEffectEstimandV3 {
    pub fn estimand_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { estimand_id, .. }
            | Self::TotalIndirect { estimand_id, .. }
            | Self::TotalEffect { estimand_id, .. } => estimand_id,
        }
    }

    pub fn source_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { source_id, .. }
            | Self::TotalIndirect { source_id, .. }
            | Self::TotalEffect { source_id, .. } => source_id,
        }
    }

    pub fn target_id(&self) -> &str {
        match self {
            Self::SpecificIndirect { target_id, .. }
            | Self::TotalIndirect { target_id, .. }
            | Self::TotalEffect { target_id, .. } => target_id,
        }
    }
}

/// PLS v3 foundation for general recursive path models. The proven v2 scoring
/// plan remains embedded unchanged while topology/effect authority and an
/// explicit two-stage interaction projection are added.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsPlanV3 {
    schema_version: u32,
    model_id: String,
    scientific_hash: String,
    general_sem_config_sha256: String,
    base_plan: CompiledPlsPlanV2,
    topology: CompiledSemTopologyV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage_one_projection_scientific_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    two_way_interactions: Vec<CompiledPlsTwoWayInteractionV3>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    two_way_moderated_mediation_target: Option<CompiledPlsTwoWayModeratedMediationTargetV1>,
    effect_estimands: Vec<CompiledPlsEffectEstimandV3>,
    auto_selected_effects: bool,
}

impl CompiledPlsPlanV3 {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub fn scientific_hash(&self) -> &str {
        &self.scientific_hash
    }

    pub fn general_sem_config_sha256(&self) -> &str {
        &self.general_sem_config_sha256
    }

    pub fn base_plan(&self) -> &CompiledPlsPlanV2 {
        &self.base_plan
    }

    pub fn topology(&self) -> &CompiledSemTopologyV1 {
        &self.topology
    }

    pub fn stage_one_projection_scientific_sha256(&self) -> Option<&str> {
        self.stage_one_projection_scientific_sha256.as_deref()
    }

    pub fn two_way_interactions(&self) -> &[CompiledPlsTwoWayInteractionV3] {
        &self.two_way_interactions
    }

    pub fn two_way_moderated_mediation_target(
        &self,
    ) -> Option<&CompiledPlsTwoWayModeratedMediationTargetV1> {
        self.two_way_moderated_mediation_target.as_ref()
    }

    pub fn effect_estimands(&self) -> &[CompiledPlsEffectEstimandV3] {
        &self.effect_estimands
    }

    pub fn auto_selected_effects(&self) -> bool {
        self.auto_selected_effects
    }

    pub fn deterministic_sha256(&self) -> String {
        sha256_serialized(self)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum CompiledPlsPlanV3Error {
    #[error(transparent)]
    InvalidGeneralSemConfig(#[from] GeneralSemConfigV1ValidationError),
    #[error(transparent)]
    Topology(#[from] CompiledSemTopologyV1Error),
    #[error(transparent)]
    BasePlan(#[from] CompiledPlsPlanV2Error),
    #[error(transparent)]
    Interaction(#[from] CompiledPlsInteractionV3Error),
    #[error(transparent)]
    ModeratedMediationTarget(#[from] CompiledPlsTwoWayModeratedMediationTargetErrorV1),
    #[error("compiled PLS v3 does not yet implement lazy specific-path materialization")]
    LazySpecificPathMaterializationNotImplemented,
    #[error("PLS v3 requires an acyclic structural topology")]
    StructuralFeedback,
    #[error("aggregate estimand id {estimand_id} collides with a canonical specific-path identity")]
    AggregateEstimandIdCollidesWithSpecificPathIdentity { estimand_id: String },
    #[error("requested specific indirect estimand {estimand_id} is not an exact compiled path")]
    UnknownSpecificIndirectPath { estimand_id: String },
    #[error(
        "requested {kind} estimand {estimand_id} has no eligible path from {source_id} to {target_id}"
    )]
    UnreachableEffect {
        kind: &'static str,
        estimand_id: String,
        source_id: String,
        target_id: String,
    },
}

pub fn compile_pls_plan_v3(
    model: &SemModelV4,
    config: &GeneralSemConfigV1,
) -> Result<CompiledPlsPlanV3, CompiledPlsPlanV3Error> {
    config.ensure_valid()?;
    if config.output_policy.lazy_specific_path_materialization
        || config.output_policy.when_specific_path_limit_exceeded
            == GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy
    {
        return Err(CompiledPlsPlanV3Error::LazySpecificPathMaterializationNotImplemented);
    }
    let topology = compile_sem_topology_v1(
        model,
        config.output_policy.max_materialized_specific_paths as usize,
    )?;
    if topology.has_feedback() {
        return Err(CompiledPlsPlanV3Error::StructuralFeedback);
    }
    let two_way_interactions = compile_pls_two_way_interactions_v3(model)?;
    let (base_plan, stage_one_projection_scientific_sha256) = if two_way_interactions.is_empty() {
        (compile_pls_plan_v2(model)?, None)
    } else {
        let projection = compile_pls_stage_one_projection_v3(model)?;
        let base_plan = compile_pls_plan_v2(projection.projected_model())?;
        debug_assert_eq!(
            base_plan.scientific_hash(),
            projection.projected_scientific_sha256()
        );
        (
            base_plan,
            Some(projection.projected_scientific_sha256().to_string()),
        )
    };
    let auto_selected_effects = config.requested_effect_estimands.is_empty();
    let interaction_outputs = two_way_interactions
        .iter()
        .map(|interaction| interaction.output_variable_id().to_string())
        .collect::<BTreeSet<_>>();
    let effect_estimands = if auto_selected_effects {
        compile_all_effect_estimands(&topology, &interaction_outputs)
    } else {
        compile_requested_effect_estimands(&topology, config, &interaction_outputs)?
    };
    let scientific_hash = model
        .scientific_sha256()
        .map_err(CompiledPlsPlanV2Error::InvalidModel)?;
    debug_assert_eq!(topology.model_scientific_sha256(), scientific_hash);
    let mut plan = CompiledPlsPlanV3 {
        schema_version: COMPILED_PLS_PLAN_V3_SCHEMA_VERSION,
        model_id: model.id.clone(),
        scientific_hash,
        general_sem_config_sha256: sha256_serialized(config),
        base_plan,
        topology,
        stage_one_projection_scientific_sha256,
        two_way_interactions,
        two_way_moderated_mediation_target: None,
        effect_estimands,
        auto_selected_effects,
    };
    if !plan.two_way_interactions.is_empty() && !config.requested_effect_estimands.is_empty() {
        plan.two_way_moderated_mediation_target = Some(
            compile_pls_two_way_moderated_mediation_target_v1(&plan, config)?,
        );
    }
    Ok(plan)
}

/// Compiles only the exact first interaction cell. Unsupported derived
/// semantics remain in the scientific model and fail closed here.
pub fn compile_pls_two_way_interactions_v3(
    model: &SemModelV4,
) -> Result<Vec<CompiledPlsTwoWayInteractionV3>, CompiledPlsInteractionV3Error> {
    model.ensure_valid()?;
    let mut compiled = Vec::new();
    let mut product_designs =
        std::collections::BTreeMap::<FixedTwoWayProductDesignSignatureV3, String>::new();
    let mut generated_columns = BTreeSet::new();

    for term in &model.derived_terms {
        let SemDerivedTermV4::InteractionV2 {
            id,
            output,
            operands,
            focal_relation,
            method,
            hierarchy_policy,
            product_indicator,
        } = term
        else {
            let kind = match term {
                SemDerivedTermV4::Interaction { .. } => "interaction_v1",
                SemDerivedTermV4::HigherOrder { .. } => "higher_order",
                SemDerivedTermV4::Polynomial { .. } => "polynomial",
                SemDerivedTermV4::InteractionV2 { .. } => unreachable!(),
            };
            return Err(CompiledPlsInteractionV3Error::UnsupportedDerivedTerm {
                term_id: term.id().to_string(),
                kind,
            });
        };
        if operands.len() != 2 {
            return Err(CompiledPlsInteractionV3Error::UnsupportedInteractionOrder {
                interaction_id: id.clone(),
                operand_count: operands.len(),
            });
        }
        if method != &InteractionMethodV4::TwoStage {
            return Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionMethod {
                    interaction_id: id.clone(),
                },
            );
        }
        if *hierarchy_policy != InteractionHierarchyPolicyV2::Strong {
            return Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionHierarchy {
                    interaction_id: id.clone(),
                },
            );
        }
        if product_indicator.is_some() {
            return Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionProductSpecification {
                    interaction_id: id.clone(),
                },
            );
        }

        let (focal_predictor_id, outcome_id) = model
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
            .expect("validated interaction_v2 has an exact focal structural relation");
        debug_assert_eq!(focal_predictor_id, operands[0]);

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
        if effect_relations.len() != 1 {
            return Err(
                CompiledPlsInteractionV3Error::InteractionEffectRelationCardinality {
                    interaction_id: id.clone(),
                    output_id: output.clone(),
                    outcome_id,
                    relation_count: effect_relations.len(),
                },
            );
        }
        let (effect_relation_id, effect_parameter_id, effect_intercept) =
            effect_relations[0].clone();
        if effect_intercept.is_some() {
            return Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionEffectParameter {
                    interaction_id: id.clone(),
                    parameter_id: effect_parameter_id,
                },
            );
        }
        if let Some(relation) = model.relations.iter().find(|relation| {
            relation.id() != effect_relation_id
                && relation_references_variable(relation, output.as_str())
        }) {
            return Err(
                CompiledPlsInteractionV3Error::InteractionOutputRelationUnsupported {
                    output_id: output.clone(),
                    relation_id: relation.id().to_string(),
                },
            );
        }

        let effect_parameter = model
            .parameters
            .iter()
            .find(|parameter| parameter.id() == effect_parameter_id)
            .expect("validated effect relation has a parameter");
        if !matches!(
            effect_parameter,
            SemParameterV4::Free {
                target: SemParameterTargetV4::Regression { source, target },
                start: None,
                lower: None,
                upper: None,
                equality_label: None,
                group_overrides,
                ..
            } if source == output && target == &outcome_id && group_overrides.is_empty()
        ) {
            return Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionEffectParameter {
                    interaction_id: id.clone(),
                    parameter_id: effect_parameter_id,
                },
            );
        }
        if let Some(constraint) = model.constraints.iter().find(|constraint| {
            constraint_references_parameter(constraint, effect_parameter_id.as_str())
        }) {
            return Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionEffectConstraint {
                    interaction_id: id.clone(),
                    parameter_id: effect_parameter_id,
                    constraint_id: constraint.id().to_string(),
                },
            );
        }

        let product_design =
            FixedTwoWayProductDesignSignatureV3::for_qualified_cell(operands, &outcome_id);
        if let Some(first_interaction_id) =
            product_designs.insert(product_design.clone(), id.clone())
        {
            let mut duplicate_interaction_ids = [first_interaction_id, id.clone()];
            duplicate_interaction_ids.sort();
            let [first_interaction_id, second_interaction_id] = duplicate_interaction_ids;
            return Err(
                CompiledPlsInteractionV3Error::DuplicateInteractionProductDesign {
                    first_interaction_id,
                    second_interaction_id,
                    sorted_operand_ids: product_design.sorted_operand_ids,
                    outcome_id: product_design.outcome_id,
                },
            );
        }
        let generated_product_column_id = interaction_product_column_identity(
            id,
            output,
            operands,
            focal_relation,
            &effect_relation_id,
        );
        let collides_with_source_column = model.variables.iter().any(|variable| {
            matches!(variable, SemVariableV4::Observed { source_column, .. } if source_column == &generated_product_column_id)
        });
        if collides_with_source_column
            || !generated_columns.insert(generated_product_column_id.clone())
        {
            return Err(
                CompiledPlsInteractionV3Error::GeneratedProductColumnCollision {
                    column_id: generated_product_column_id,
                },
            );
        }
        compiled.push(CompiledPlsTwoWayInteractionV3 {
            contract_version: COMPILED_PLS_TWO_WAY_INTERACTION_V3_VERSION.to_string(),
            interaction_id: id.clone(),
            output_variable_id: output.clone(),
            focal_predictor_id,
            moderator_id: operands[1].clone(),
            outcome_id,
            focal_relation_id: focal_relation.clone(),
            interaction_effect_relation_id: effect_relation_id,
            interaction_effect_parameter_id: effect_parameter_id,
            method: method.clone(),
            hierarchy_policy: *hierarchy_policy,
            generated_product_column_id,
        });
    }
    compiled.sort_by(|left, right| left.interaction_id.cmp(&right.interaction_id));
    Ok(compiled)
}

/// Produces the ordinary PLS scientific model used to obtain stage-one scores.
/// Only compiled interaction outputs, effect paths, and effect parameters are
/// removed. Unsupported semantics are rejected before projection.
pub fn compile_pls_stage_one_projection_v3(
    model: &SemModelV4,
) -> Result<CompiledPlsStageOneProjectionV3, CompiledPlsInteractionV3Error> {
    let interactions = compile_pls_two_way_interactions_v3(model)?;
    let source_scientific_sha256 = model.scientific_sha256()?;
    if interactions.is_empty() {
        return Ok(CompiledPlsStageOneProjectionV3 {
            contract_version: COMPILED_PLS_STAGE_ONE_PROJECTION_V3_VERSION.to_string(),
            source_scientific_sha256: source_scientific_sha256.clone(),
            projected_scientific_sha256: source_scientific_sha256,
            projected_model: model.canonicalized(),
        });
    }

    let output_ids = interactions
        .iter()
        .map(|interaction| interaction.output_variable_id())
        .collect::<BTreeSet<_>>();
    let relation_ids = interactions
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id())
        .collect::<BTreeSet<_>>();
    let parameter_ids = interactions
        .iter()
        .map(|interaction| interaction.interaction_effect_parameter_id())
        .collect::<BTreeSet<_>>();
    let mut projected_model = model.clone();
    projected_model
        .variables
        .retain(|variable| !output_ids.contains(variable.id()));
    projected_model
        .relations
        .retain(|relation| !relation_ids.contains(relation.id()));
    projected_model
        .parameters
        .retain(|parameter| !parameter_ids.contains(parameter.id()));
    projected_model.derived_terms.clear();
    projected_model.annotations.clear();
    projected_model.presentation = SemPresentationV4::None;
    projected_model = projected_model.canonicalized();
    projected_model.ensure_valid()?;
    let projected_scientific_sha256 = projected_model.scientific_sha256()?;
    Ok(CompiledPlsStageOneProjectionV3 {
        contract_version: COMPILED_PLS_STAGE_ONE_PROJECTION_V3_VERSION.to_string(),
        source_scientific_sha256,
        projected_scientific_sha256,
        projected_model,
    })
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
            parameters.iter().any(|candidate| candidate == parameter_id)
        }
        SemConstraintV4::Bound { parameter, .. } => parameter == parameter_id,
        SemConstraintV4::Linear { terms, .. } => {
            terms.iter().any(|term| term.parameter == parameter_id)
        }
    }
}

fn interaction_product_column_identity(
    interaction_id: &str,
    output_id: &str,
    operands: &[String],
    focal_relation_id: &str,
    effect_relation_id: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-pls-plan-v3.two-stage-product\0");
    for value in [
        interaction_id,
        output_id,
        operands[0].as_str(),
        operands[1].as_str(),
        focal_relation_id,
        effect_relation_id,
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    format!("qpls_pls_product_v1_{:x}", digest.finalize())
}

fn compile_requested_effect_estimands(
    topology: &CompiledSemTopologyV1,
    config: &GeneralSemConfigV1,
    excluded_sources: &BTreeSet<String>,
) -> Result<Vec<CompiledPlsEffectEstimandV3>, CompiledPlsPlanV3Error> {
    let reserved_specific_path_identities = topology
        .specific_directed_paths()
        .iter()
        .filter(|path| !excluded_sources.contains(path.source()))
        .map(|path| path.identity().to_string())
        .collect::<BTreeSet<_>>();
    let mut compiled = Vec::with_capacity(config.requested_effect_estimands.len());
    for request in &config.requested_effect_estimands {
        match request {
            GeneralSemEffectEstimandV1::SpecificPath {
                estimand_id,
                ordered_relation_ids,
            } => {
                let path = topology
                    .specific_directed_paths()
                    .iter()
                    .find(|path| {
                        !excluded_sources.contains(path.source())
                            && path.relation_ids() == ordered_relation_ids
                    })
                    .ok_or_else(|| CompiledPlsPlanV3Error::UnknownSpecificIndirectPath {
                        estimand_id: estimand_id.clone(),
                    })?;
                compiled.push(specific_estimand(estimand_id.clone(), path));
            }
            GeneralSemEffectEstimandV1::TotalIndirect {
                estimand_id,
                source_id,
                target_id,
            } => {
                reject_aggregate_specific_path_identity_collision(
                    &reserved_specific_path_identities,
                    estimand_id,
                )?;
                let path_ids = indirect_path_ids(topology, source_id, target_id, excluded_sources);
                if path_ids.is_empty() {
                    return Err(CompiledPlsPlanV3Error::UnreachableEffect {
                        kind: "total_indirect",
                        estimand_id: estimand_id.clone(),
                        source_id: source_id.clone(),
                        target_id: target_id.clone(),
                    });
                }
                compiled.push(CompiledPlsEffectEstimandV3::TotalIndirect {
                    estimand_id: estimand_id.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    contributing_path_identities: path_ids,
                });
            }
            GeneralSemEffectEstimandV1::TotalEffect {
                estimand_id,
                source_id,
                target_id,
            } => {
                reject_aggregate_specific_path_identity_collision(
                    &reserved_specific_path_identities,
                    estimand_id,
                )?;
                let direct_relation_ids =
                    direct_relation_ids(topology, source_id, target_id, excluded_sources);
                let path_ids = indirect_path_ids(topology, source_id, target_id, excluded_sources);
                if direct_relation_ids.is_empty() && path_ids.is_empty() {
                    return Err(CompiledPlsPlanV3Error::UnreachableEffect {
                        kind: "total_effect",
                        estimand_id: estimand_id.clone(),
                        source_id: source_id.clone(),
                        target_id: target_id.clone(),
                    });
                }
                compiled.push(CompiledPlsEffectEstimandV3::TotalEffect {
                    estimand_id: estimand_id.clone(),
                    source_id: source_id.clone(),
                    target_id: target_id.clone(),
                    direct_relation_ids,
                    contributing_indirect_path_identities: path_ids,
                });
            }
        }
    }
    Ok(compiled)
}

fn reject_aggregate_specific_path_identity_collision(
    reserved_specific_path_identities: &BTreeSet<String>,
    estimand_id: &str,
) -> Result<(), CompiledPlsPlanV3Error> {
    if reserved_specific_path_identities.contains(estimand_id) {
        Err(
            CompiledPlsPlanV3Error::AggregateEstimandIdCollidesWithSpecificPathIdentity {
                estimand_id: estimand_id.to_string(),
            },
        )
    } else {
        Ok(())
    }
}

fn compile_all_effect_estimands(
    topology: &CompiledSemTopologyV1,
    excluded_sources: &BTreeSet<String>,
) -> Vec<CompiledPlsEffectEstimandV3> {
    let mut compiled = topology
        .specific_directed_paths()
        .iter()
        .filter(|path| !excluded_sources.contains(path.source()))
        .map(|path| specific_estimand(path.identity().to_string(), path))
        .collect::<Vec<_>>();
    let mut pairs = BTreeSet::new();
    for relation in topology.structural_relations() {
        if relation.role() == StructuralRelationRoleV4::Structural {
            if excluded_sources.contains(relation.source()) {
                continue;
            }
            pairs.insert((relation.source().to_string(), relation.target().to_string()));
        }
    }
    for path in topology.specific_directed_paths() {
        if excluded_sources.contains(path.source()) {
            continue;
        }
        pairs.insert((path.source().to_string(), path.target().to_string()));
    }
    for (source_id, target_id) in pairs {
        let path_ids = indirect_path_ids(topology, &source_id, &target_id, excluded_sources);
        if !path_ids.is_empty() {
            compiled.push(CompiledPlsEffectEstimandV3::TotalIndirect {
                estimand_id: auto_effect_identity("total_indirect", &source_id, &target_id),
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                contributing_path_identities: path_ids.clone(),
            });
        }
        compiled.push(CompiledPlsEffectEstimandV3::TotalEffect {
            estimand_id: auto_effect_identity("total_effect", &source_id, &target_id),
            direct_relation_ids: direct_relation_ids(
                topology,
                &source_id,
                &target_id,
                excluded_sources,
            ),
            source_id,
            target_id,
            contributing_indirect_path_identities: path_ids,
        });
    }
    compiled.sort_by(|left, right| left.estimand_id().cmp(right.estimand_id()));
    compiled
}

fn specific_estimand(
    estimand_id: String,
    path: &CompiledSemSpecificDirectedPathV1,
) -> CompiledPlsEffectEstimandV3 {
    CompiledPlsEffectEstimandV3::SpecificIndirect {
        estimand_id,
        path_identity: path.identity().to_string(),
        source_id: path.source().to_string(),
        target_id: path.target().to_string(),
        ordered_relation_ids: path.relation_ids().to_vec(),
    }
}

fn indirect_path_ids(
    topology: &CompiledSemTopologyV1,
    source: &str,
    target: &str,
    excluded_sources: &BTreeSet<String>,
) -> Vec<String> {
    if excluded_sources.contains(source) {
        return Vec::new();
    }
    let mut identities = topology
        .specific_directed_paths()
        .iter()
        .filter(|path| path.source() == source && path.target() == target)
        .map(|path| path.identity().to_string())
        .collect::<Vec<_>>();
    identities.sort();
    identities
}

fn direct_relation_ids(
    topology: &CompiledSemTopologyV1,
    source: &str,
    target: &str,
    excluded_sources: &BTreeSet<String>,
) -> Vec<String> {
    if excluded_sources.contains(source) {
        return Vec::new();
    }
    let mut relation_ids = topology
        .structural_relations()
        .iter()
        .filter(|relation| {
            relation.role() == StructuralRelationRoleV4::Structural
                && relation.source() == source
                && relation.target() == target
        })
        .map(|relation| relation.relation_id().to_string())
        .collect::<Vec<_>>();
    relation_ids.sort();
    relation_ids
}

fn auto_effect_identity(kind: &str, source: &str, target: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-pls-plan-v3.effect\0");
    for value in [kind, source, target] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    format!("sem_{kind}_v1_{:x}", digest.finalize())
}

fn sha256_serialized<T: Serialize>(value: &T) -> String {
    format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).expect("compiled SEM contract serializes"))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Construct, GeneralSemEffectEstimandV1, InteractionHierarchyPolicyV2, InteractionMethodV4,
        LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, SemDerivedTermV4,
        SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath,
        convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn recursive_model() -> SemModelV4 {
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
                id: Uuid::from_u128(0x5031_5303),
                name: "Parallel and serial mediation".into(),
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

    fn relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                crate::SemRelationV4::Structural {
                    id,
                    source: relation_source,
                    target: relation_target,
                    ..
                } if relation_source == source && relation_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn add_two_way_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        moderator_id: &str,
        focal_relation_id: &str,
    ) {
        add_two_way_interaction_for_focal(
            model,
            interaction_id,
            "construct:x",
            moderator_id,
            focal_relation_id,
        );
    }

    fn add_two_way_interaction_for_focal(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
        focal_relation_id: &str,
    ) {
        let outcome_id = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural { id, target, .. } if id == focal_relation_id => {
                    Some(target.clone())
                }
                _ => None,
            })
            .unwrap();
        let output_id = format!("derived:{interaction_id}");
        let effect_relation_id = format!("relation:{interaction_id}:effect");
        let effect_parameter_id = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output_id.clone(),
            label: interaction_id.to_string(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: effect_relation_id,
            source: output_id.clone(),
            target: outcome_id.clone(),
            parameter: effect_parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: effect_parameter_id,
            label: format!("{interaction_id} -> {outcome_id}"),
            target: SemParameterTargetV4::Regression {
                source: output_id.clone(),
                target: outcome_id,
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.derived_terms.push(SemDerivedTermV4::InteractionV2 {
            id: interaction_id.to_string(),
            output: output_id,
            operands: vec![focal_predictor_id.into(), moderator_id.into()],
            focal_relation: focal_relation_id.to_string(),
            method: InteractionMethodV4::TwoStage,
            hierarchy_policy: InteractionHierarchyPolicyV2::Strong,
            product_indicator: None,
        });
    }

    #[test]
    fn default_config_infers_all_specific_and_aggregate_effects_deterministically() {
        let model = recursive_model();
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(plan.schema_version(), 3);
        assert!(plan.auto_selected_effects());
        assert!(plan.effect_estimands().iter().any(|estimand| matches!(
            estimand,
            CompiledPlsEffectEstimandV3::SpecificIndirect {
                source_id,
                target_id,
                ordered_relation_ids,
                ..
            } if source_id == "construct:x" && target_id == "construct:y"
                && ordered_relation_ids.len() == 3
        )));
        assert!(plan.effect_estimands().iter().any(|estimand| matches!(
            estimand,
            CompiledPlsEffectEstimandV3::TotalIndirect {
                source_id,
                target_id,
                contributing_path_identities,
                ..
            } if source_id == "construct:x" && target_id == "construct:y"
                && contributing_path_identities.len() >= 3
        )));
        let encoded = serde_json::to_vec(&plan).unwrap();
        assert_eq!(
            serde_json::from_slice::<CompiledPlsPlanV3>(&encoded).unwrap(),
            plan
        );
        assert_eq!(
            compile_pls_plan_v3(&model, &GeneralSemConfigV1::default())
                .unwrap()
                .deterministic_sha256(),
            plan.deterministic_sha256()
        );
    }

    #[test]
    fn explicit_requests_bind_exact_relation_paths_and_reject_unreachable_effects() {
        let model = recursive_model();
        let relation_x_m1 = relation_id(&model, "construct:x", "construct:m1");
        let relation_m1_y = relation_id(&model, "construct:m1", "construct:y");
        let mut config = GeneralSemConfigV1::default();
        config.requested_effect_estimands = vec![
            GeneralSemEffectEstimandV1::SpecificPath {
                estimand_id: "effect:01:specific".into(),
                ordered_relation_ids: vec![relation_x_m1.clone(), relation_m1_y.clone()],
            },
            GeneralSemEffectEstimandV1::TotalIndirect {
                estimand_id: "effect:02:indirect".into(),
                source_id: "construct:x".into(),
                target_id: "construct:y".into(),
            },
            GeneralSemEffectEstimandV1::TotalEffect {
                estimand_id: "effect:03:total".into(),
                source_id: "construct:x".into(),
                target_id: "construct:y".into(),
            },
        ];
        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        assert!(!plan.auto_selected_effects());
        assert_eq!(plan.effect_estimands().len(), 3);

        {
            let GeneralSemEffectEstimandV1::SpecificPath {
                ordered_relation_ids,
                ..
            } = &mut config.requested_effect_estimands[0]
            else {
                unreachable!()
            };
            ordered_relation_ids.reverse();
        }
        assert!(matches!(
            compile_pls_plan_v3(&model, &config),
            Err(CompiledPlsPlanV3Error::UnknownSpecificIndirectPath { .. })
        ));

        let GeneralSemEffectEstimandV1::SpecificPath {
            ordered_relation_ids,
            ..
        } = &mut config.requested_effect_estimands[0]
        else {
            unreachable!()
        };
        *ordered_relation_ids = vec![relation_x_m1, relation_m1_y];
        let GeneralSemEffectEstimandV1::TotalEffect { target_id, .. } =
            &mut config.requested_effect_estimands[2]
        else {
            unreachable!()
        };
        *target_id = "construct:missing".into();
        assert!(matches!(
            compile_pls_plan_v3(&model, &config),
            Err(CompiledPlsPlanV3Error::UnreachableEffect {
                kind: "total_effect",
                ..
            })
        ));
    }

    #[test]
    fn declaration_order_does_not_change_v3_plan_or_effect_identities() {
        let model = recursive_model();
        let expected = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        let actual = compile_pls_plan_v3(&reordered, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn aggregate_estimand_cannot_reuse_a_canonical_specific_path_identity() {
        let model = recursive_model();
        let topology = compile_sem_topology_v1(&model, 100).unwrap();
        let path = topology
            .specific_directed_paths()
            .iter()
            .find(|path| path.source() == "construct:x" && path.target() == "construct:y")
            .unwrap();
        let collision = path.identity().to_string();
        let mut config = GeneralSemConfigV1::default();
        config.requested_effect_estimands = vec![GeneralSemEffectEstimandV1::TotalEffect {
            estimand_id: collision.clone(),
            source_id: "construct:x".into(),
            target_id: "construct:y".into(),
        }];

        assert_eq!(
            compile_pls_plan_v3(&model, &config),
            Err(
                CompiledPlsPlanV3Error::AggregateEstimandIdCollidesWithSpecificPathIdentity {
                    estimand_id: collision,
                }
            )
        );
    }

    #[test]
    fn direct_pls_v3_compilation_rejects_lazy_path_materialization() {
        let model = recursive_model();
        let mut config = GeneralSemConfigV1::default();
        config.output_policy.lazy_specific_path_materialization = true;
        config.output_policy.when_specific_path_limit_exceeded =
            GeneralSemSpecificPathLimitBehaviorV1::ReturnLazy;

        assert_eq!(
            compile_pls_plan_v3(&model, &config),
            Err(CompiledPlsPlanV3Error::LazySpecificPathMaterializationNotImplemented)
        );
    }

    #[test]
    fn two_interactions_compile_to_one_deterministic_stage_one_projection() {
        let mut model = recursive_model();
        let focal_relation_id = relation_id(&model, "construct:x", "construct:y");
        add_two_way_interaction(
            &mut model,
            "interaction:x_by_m1",
            "construct:m1",
            &focal_relation_id,
        );
        add_two_way_interaction(
            &mut model,
            "interaction:x_by_m2",
            "construct:m2",
            &focal_relation_id,
        );
        model.ensure_valid().unwrap();

        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(plan.two_way_interactions().len(), 2);
        assert_eq!(plan.scientific_hash(), model.scientific_sha256().unwrap());
        assert_eq!(
            plan.stage_one_projection_scientific_sha256(),
            Some(plan.base_plan().scientific_hash())
        );
        assert_ne!(plan.scientific_hash(), plan.base_plan().scientific_hash());
        assert!(
            plan.base_plan()
                .paths()
                .iter()
                .all(|path| !path.source().starts_with("derived:"))
        );
        assert_ne!(
            plan.two_way_interactions()[0].generated_product_column_id(),
            plan.two_way_interactions()[1].generated_product_column_id()
        );

        let projection = compile_pls_stage_one_projection_v3(&model).unwrap();
        assert!(projection.projected_model().derived_terms.is_empty());
        assert!(
            projection
                .projected_model()
                .variables
                .iter()
                .all(|variable| !matches!(variable, SemVariableV4::Derived { .. }))
        );
        assert_eq!(
            projection.projected_scientific_sha256(),
            plan.base_plan().scientific_hash()
        );

        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        reordered.derived_terms.reverse();
        let reordered_projection = compile_pls_stage_one_projection_v3(&reordered).unwrap();
        assert_eq!(reordered_projection, projection);
        assert_eq!(
            reordered_projection.deterministic_sha256(),
            projection.deterministic_sha256()
        );
        assert_eq!(
            compile_pls_plan_v3(&reordered, &GeneralSemConfigV1::default()).unwrap(),
            plan
        );
    }

    #[test]
    fn reversed_roles_on_the_same_product_and_outcome_are_a_typed_duplicate() {
        let mut model = recursive_model();
        let x_to_y = relation_id(&model, "construct:x", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:x_by_m1",
            "construct:x",
            "construct:m1",
            &x_to_y,
        );
        model.ensure_valid().unwrap();

        let compiled = compile_pls_two_way_interactions_v3(&model).unwrap();
        assert_eq!(compiled[0].focal_predictor_id(), "construct:x");
        assert_eq!(compiled[0].moderator_id(), "construct:m1");

        let m1_to_y = relation_id(&model, "construct:m1", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:m1_by_x",
            "construct:m1",
            "construct:x",
            &m1_to_y,
        );
        model.ensure_valid().unwrap();

        let expected = Err(
            CompiledPlsInteractionV3Error::DuplicateInteractionProductDesign {
                first_interaction_id: "interaction:m1_by_x".into(),
                second_interaction_id: "interaction:x_by_m1".into(),
                sorted_operand_ids: ["construct:m1".into(), "construct:x".into()],
                outcome_id: "construct:y".into(),
            },
        );
        assert_eq!(compile_pls_two_way_interactions_v3(&model), expected);

        model.derived_terms.reverse();
        assert_eq!(compile_pls_two_way_interactions_v3(&model), expected);
    }

    #[test]
    fn reversed_roles_remain_distinct_when_the_product_targets_another_outcome() {
        let mut model = recursive_model();
        let x_to_y = relation_id(&model, "construct:x", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:x_by_m1_to_y",
            "construct:x",
            "construct:m1",
            &x_to_y,
        );
        let m1_to_m2 = relation_id(&model, "construct:m1", "construct:m2");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:m1_by_x_to_m2",
            "construct:m1",
            "construct:x",
            &m1_to_m2,
        );
        model.ensure_valid().unwrap();

        let compiled = compile_pls_two_way_interactions_v3(&model).unwrap();
        assert_eq!(compiled.len(), 2);
        let m1_focal = compiled
            .iter()
            .find(|interaction| interaction.interaction_id() == "interaction:m1_by_x_to_m2")
            .unwrap();
        assert_eq!(m1_focal.focal_predictor_id(), "construct:m1");
        assert_eq!(m1_focal.moderator_id(), "construct:x");
        assert_eq!(m1_focal.outcome_id(), "construct:m2");
    }

    #[test]
    fn interaction_compilation_rejects_non_strong_hierarchy_before_projection() {
        let mut model = recursive_model();
        let focal_relation_id = relation_id(&model, "construct:x", "construct:y");
        add_two_way_interaction(
            &mut model,
            "interaction:x_by_m1",
            "construct:m1",
            &focal_relation_id,
        );
        let SemDerivedTermV4::InteractionV2 {
            hierarchy_policy, ..
        } = &mut model.derived_terms[0]
        else {
            unreachable!()
        };
        *hierarchy_policy = InteractionHierarchyPolicyV2::Weak;
        model.ensure_valid().unwrap();
        assert_eq!(
            compile_pls_two_way_interactions_v3(&model),
            Err(
                CompiledPlsInteractionV3Error::UnsupportedInteractionHierarchy {
                    interaction_id: "interaction:x_by_m1".into()
                }
            )
        );
    }

    fn moderated_mediation_config(relation_ids: Vec<String>) -> GeneralSemConfigV1 {
        let mut config = GeneralSemConfigV1::default();
        config.requested_effect_estimands = vec![GeneralSemEffectEstimandV1::SpecificPath {
            estimand_id: "estimand:selected_path".into(),
            ordered_relation_ids: relation_ids,
        }];
        config.inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 42,
            confidence_level: 0.95,
            interval: crate::GeneralSemBootstrapIntervalV1::Percentile,
            tail: crate::GeneralSemInferenceTailV1::TwoSided,
        };
        config
    }

    #[test]
    fn exact_first_stage_target_and_point_formulas_are_compiled_without_heuristics() {
        let mut model = recursive_model();
        let x_to_m2 = relation_id(&model, "construct:x", "construct:m2");
        let m2_to_y = relation_id(&model, "construct:m2", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:x_by_m1_to_m2",
            "construct:x",
            "construct:m1",
            &x_to_m2,
        );
        model.ensure_valid().unwrap();
        let config = moderated_mediation_config(vec![x_to_m2.clone(), m2_to_y.clone()]);

        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        let target = plan.two_way_moderated_mediation_target().unwrap();
        assert_eq!(
            target.moderated_stage(),
            crate::CompiledPlsTwoWayModeratedMediationStageV1::FirstStage
        );
        assert_eq!(target.moderated_relation_id(), x_to_m2);
        assert_eq!(target.other_stage_relation_id(), m2_to_y);
        assert_eq!(target.x_id(), "construct:x");
        assert_eq!(target.mediator_id(), "construct:m2");
        assert_eq!(target.y_id(), "construct:y");
        assert_eq!(target.moderator_id(), "construct:m1");
        assert_eq!(
            target.bootstrap_capability_cell(),
            &crate::pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1()
        );
        let inference_targets =
            crate::general_sem_pls_two_way_moderated_mediation_inference_targets_v1(target);
        assert_eq!(inference_targets.len(), 4);
        assert_eq!(
            inference_targets
                .iter()
                .filter(|target| matches!(
                    target,
                    crate::GeneralSemPlsTwoWayModeratedMediationInferenceTargetV1::ConditionalIndirect { .. }
                ))
                .count(),
            3
        );
        assert!(matches!(
            inference_targets.last(),
            Some(
                crate::GeneralSemPlsTwoWayModeratedMediationInferenceTargetV1::ModeratedMediationIndex { .. }
            )
        ));

        let point = crate::calculate_general_sem_pls_two_way_moderated_mediation_point_v1(
            target, 0.4, 0.5, 0.2,
        )
        .unwrap();
        assert_eq!(
            point
                .conditional_indirect_effects
                .iter()
                .map(|effect| effect.moderator_value)
                .collect::<Vec<_>>(),
            vec![-1.0, 0.0, 1.0]
        );
        assert!((point.conditional_indirect_effects[0].estimate - 0.1).abs() < 1e-12);
        assert!((point.conditional_indirect_effects[1].estimate - (0.4 * 0.5)).abs() < 1e-12);
        assert!((point.conditional_indirect_effects[2].estimate - 0.3).abs() < 1e-12);
        assert!((point.moderated_mediation_index.estimate - 0.1).abs() < 1e-12);

        let wire = serde_json::to_value(&plan).unwrap();
        assert!(wire.get("two_way_moderated_mediation_target").is_some());
        assert_eq!(
            serde_json::from_value::<CompiledPlsPlanV3>(wire).unwrap(),
            plan
        );
    }

    #[test]
    fn exact_second_stage_target_is_classified_from_stable_relation_ids() {
        let mut model = recursive_model();
        let x_to_m1 = relation_id(&model, "construct:x", "construct:m1");
        let m1_to_y = relation_id(&model, "construct:m1", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:m1_by_m2_to_y",
            "construct:m1",
            "construct:m2",
            &m1_to_y,
        );
        model.ensure_valid().unwrap();
        let config = moderated_mediation_config(vec![x_to_m1.clone(), m1_to_y.clone()]);

        let plan = compile_pls_plan_v3(&model, &config).unwrap();
        let target = plan.two_way_moderated_mediation_target().unwrap();
        assert_eq!(
            target.moderated_stage(),
            crate::CompiledPlsTwoWayModeratedMediationStageV1::SecondStage
        );
        assert_eq!(target.moderated_relation_id(), m1_to_y);
        assert_eq!(target.other_stage_relation_id(), x_to_m1);

        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        assert_eq!(compile_pls_plan_v3(&reordered, &config).unwrap(), plan);
    }

    #[test]
    fn moderated_mediation_target_rejects_point_long_path_and_multiple_interactions() {
        let mut model = recursive_model();
        let x_to_m1 = relation_id(&model, "construct:x", "construct:m1");
        let m1_to_y = relation_id(&model, "construct:m1", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:m1_by_m2_to_y",
            "construct:m1",
            "construct:m2",
            &m1_to_y,
        );
        model.ensure_valid().unwrap();

        let mut point = moderated_mediation_config(vec![x_to_m1.clone(), m1_to_y.clone()]);
        point.inference = GeneralSemInferenceV1::None;
        assert!(matches!(
            compile_pls_plan_v3(&model, &point),
            Err(CompiledPlsPlanV3Error::ModeratedMediationTarget(
                crate::CompiledPlsTwoWayModeratedMediationTargetErrorV1::BootstrapRequired
            ))
        ));

        let m1_to_m2 = relation_id(&model, "construct:m1", "construct:m2");
        let m2_to_y = relation_id(&model, "construct:m2", "construct:y");
        let long = moderated_mediation_config(vec![x_to_m1, m1_to_m2, m2_to_y]);
        assert!(matches!(
            compile_pls_plan_v3(&model, &long),
            Err(CompiledPlsPlanV3Error::ModeratedMediationTarget(
                crate::CompiledPlsTwoWayModeratedMediationTargetErrorV1::SpecificPathLength {
                    found: 3
                }
            ))
        ));

        let x_to_y = relation_id(&model, "construct:x", "construct:y");
        add_two_way_interaction_for_focal(
            &mut model,
            "interaction:x_by_m2_to_y",
            "construct:x",
            "construct:m2",
            &x_to_y,
        );
        model.ensure_valid().unwrap();
        let exact = moderated_mediation_config(vec![
            relation_id(&model, "construct:x", "construct:m1"),
            m1_to_y,
        ]);
        assert!(matches!(
            compile_pls_plan_v3(&model, &exact),
            Err(CompiledPlsPlanV3Error::ModeratedMediationTarget(
                crate::CompiledPlsTwoWayModeratedMediationTargetErrorV1::InteractionCardinality {
                    found: 2
                }
            ))
        ));
    }

    #[test]
    fn historical_interaction_plan_omits_empty_moderated_mediation_target() {
        let mut model = recursive_model();
        let x_to_y = relation_id(&model, "construct:x", "construct:y");
        add_two_way_interaction(&mut model, "interaction:x_by_m1", "construct:m1", &x_to_y);
        model.ensure_valid().unwrap();
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        assert!(plan.two_way_moderated_mediation_target().is_none());
        assert!(
            serde_json::to_value(plan)
                .unwrap()
                .get("two_way_moderated_mediation_target")
                .is_none()
        );
    }
}
