use crate::{
    CapabilityCellReferenceV2, CompiledPlsBlockModeV2, CompiledPlsPlanV2, CompiledPlsPlanV2Error,
    CompiledSemSpecificDirectedPathV1, CompiledSemTopologyV1, CompiledSemTopologyV1Error,
    CompositeWeightingV4, GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1, GeneralSemConfigV1,
    GeneralSemConfigV1ValidationError, GeneralSemEffectEstimandV1,
    GeneralSemSpecificPathLimitBehaviorV1, HigherOrderConstructionApproachV4,
    HigherOrderMeasurementTypeV4, InteractionHierarchyPolicyV2, InteractionMethodV4,
    SemConstraintV4, SemDerivedTermV4, SemModelV4, SemParameterTargetV4, SemParameterV4,
    SemPresentationV4, SemRelationV4, SemVariableV4, StructuralRelationRoleV4, compile_pls_plan_v2,
    compile_sem_topology_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const COMPILED_PLS_PLAN_V3_SCHEMA_VERSION: u32 = 3;
pub const COMPILED_PLS_TWO_WAY_INTERACTION_V3_VERSION: &str =
    "qpls.compiled-pls-two-way-interaction.v3.1";
pub const COMPILED_PLS_STAGE_ONE_PROJECTION_V3_VERSION: &str =
    "qpls.compiled-pls-stage-one-projection.v3.1";
pub const COMPILED_PLS_HIGHER_ORDER_STAGE_PLAN_V1_VERSION: &str =
    "qpls.compiled-pls-higher-order-stage-plan.v1";
pub const COMPILED_PLS_HIGHER_ORDER_PROJECTION_V1_VERSION: &str =
    "qpls.compiled-pls-higher-order-lower-order-projection.v1";

pub const PLS_GENERAL_HIGHER_ORDER_CAPABILITY_ID_V1: &str = "smartpls.higher_order_models";
pub const PLS_GENERAL_HIGHER_ORDER_POINT_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_higher_order_point";
pub const PLS_GENERAL_HIGHER_ORDER_POINT_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_higher_order_point_v1";
pub const PLS_GENERAL_HIGHER_ORDER_BOOTSTRAP_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_higher_order_full_model_case_bootstrap";
pub const PLS_GENERAL_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_higher_order_full_model_case_bootstrap_v1";

pub fn pls_general_higher_order_point_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_HIGHER_ORDER_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_HIGHER_ORDER_POINT_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_HIGHER_ORDER_POINT_CAPABILITY_VERSION_V1.into(),
    }
}

pub fn pls_general_higher_order_bootstrap_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_GENERAL_HIGHER_ORDER_CAPABILITY_ID_V1.into(),
        cell_id: PLS_GENERAL_HIGHER_ORDER_BOOTSTRAP_CELL_ID_V1.into(),
        capability_version: PLS_GENERAL_HIGHER_ORDER_BOOTSTRAP_CAPABILITY_VERSION_V1.into(),
    }
}

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

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledPlsHigherOrderV1Error {
    #[error(transparent)]
    InvalidModel(#[from] crate::SemModelV4ValidationError),
    #[error("General SEM HOC v1 supports exactly one higher-order term; found {found}")]
    HigherOrderCardinality { found: usize },
    #[error(
        "higher-order term {term_id} cannot be combined with derived term {other_term_id} ({kind})"
    )]
    DerivedTermCombination {
        term_id: String,
        other_term_id: String,
        kind: &'static str,
    },
    #[error("higher-order term {term_id} uses compatibility-only hybrid construction")]
    HybridCompatibilityOnly { term_id: String },
    #[error(
        "higher-order term {term_id} does not support {approach:?}/{measurement_type:?} when endogenous={hoc_is_endogenous}"
    )]
    UnsupportedApproachTypeTopology {
        term_id: String,
        approach: HigherOrderConstructionApproachV4,
        measurement_type: HigherOrderMeasurementTypeV4,
        hoc_is_endogenous: bool,
    },
    #[error("higher-order component {component_id} must be an ordinary, non-nested composite")]
    NestedOrNonCompositeComponent { component_id: String },
    #[error(
        "higher-order component {component_id} requires {expected_mode:?}, but its authored weighting resolves differently"
    )]
    ComponentModeMismatch {
        component_id: String,
        expected_mode: CompiledPlsBlockModeV2,
    },
    #[error("composite {construct_id} uses fixed/custom scoring outside HOC v1 Mode A/B scope")]
    FixedOrCustomScoring { construct_id: String },
    #[error(
        "higher-order output {output_id} participates in unsupported non-structural relation {relation_id}"
    )]
    OutputRelationUnsupported {
        output_id: String,
        relation_id: String,
    },
    #[error(
        "higher-order output {output_id} and component {component_id} cannot have an authored structural path {relation_id}; their HOC relationship is compiler-generated"
    )]
    AuthoredComponentRelation {
        output_id: String,
        component_id: String,
        relation_id: String,
    },
    #[error(
        "higher-order structural relation {relation_id} requires an unconstrained free regression parameter"
    )]
    UnsupportedStructuralParameter { relation_id: String },
    #[error(
        "higher-order generated identity collides with authored or generated identity {identity}"
    )]
    GeneratedIdentityCollision { identity: String },
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

/// Interpretation of the implicit HOC-to-LOC relationship. Reflective
/// relationships publish loadings; formative relationships publish weights
/// and their collinearity diagnostics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledPlsHocComponentRelationInterpretationV1 {
    Loading,
    WeightAndCollinearity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompiledPlsHocStageRoleV1 {
    RepeatedIndicatorEstimation,
    ExtendedRepeatedIndicatorEstimation,
    EmbeddedRepeatedIndicatorEstimation,
    DisjointLowerOrderScoreEstimation,
    HigherOrderFromLowerOrderScores,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsHocVirtualIndicatorV1 {
    source_indicator_variable_id: String,
    source_column: String,
    generated_variable_id: String,
    generated_source_column_id: String,
    generated_relation_id: String,
    generated_parameter_id: String,
}

impl CompiledPlsHocVirtualIndicatorV1 {
    pub fn source_indicator_variable_id(&self) -> &str {
        &self.source_indicator_variable_id
    }

    pub fn source_column(&self) -> &str {
        &self.source_column
    }

    pub fn generated_variable_id(&self) -> &str {
        &self.generated_variable_id
    }

    pub fn generated_source_column_id(&self) -> &str {
        &self.generated_source_column_id
    }

    pub fn generated_relation_id(&self) -> &str {
        &self.generated_relation_id
    }

    pub fn generated_parameter_id(&self) -> &str {
        &self.generated_parameter_id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsHocComponentMappingV1 {
    component_id: String,
    loc_measurement_mode: CompiledPlsBlockModeV2,
    generated_score_variable_id: String,
    generated_component_relation_id: String,
    generated_component_parameter_id: String,
    component_relation_source_id: String,
    component_relation_target_id: String,
    relation_interpretation: CompiledPlsHocComponentRelationInterpretationV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    virtual_indicators: Vec<CompiledPlsHocVirtualIndicatorV1>,
}

impl CompiledPlsHocComponentMappingV1 {
    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn loc_measurement_mode(&self) -> CompiledPlsBlockModeV2 {
        self.loc_measurement_mode
    }

    pub fn generated_score_variable_id(&self) -> &str {
        &self.generated_score_variable_id
    }

    pub fn generated_component_relation_id(&self) -> &str {
        &self.generated_component_relation_id
    }

    pub fn generated_component_parameter_id(&self) -> &str {
        &self.generated_component_parameter_id
    }

    pub fn component_relation_source_id(&self) -> &str {
        &self.component_relation_source_id
    }

    pub fn component_relation_target_id(&self) -> &str {
        &self.component_relation_target_id
    }

    pub fn relation_interpretation(&self) -> CompiledPlsHocComponentRelationInterpretationV1 {
        self.relation_interpretation
    }

    pub fn virtual_indicators(&self) -> &[CompiledPlsHocVirtualIndicatorV1] {
        &self.virtual_indicators
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsHocTechnicalPathV1 {
    authored_antecedent_relation_id: String,
    source_id: String,
    component_id: String,
    generated_relation_id: String,
    generated_parameter_id: String,
}

impl CompiledPlsHocTechnicalPathV1 {
    pub fn authored_antecedent_relation_id(&self) -> &str {
        &self.authored_antecedent_relation_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn generated_relation_id(&self) -> &str {
        &self.generated_relation_id
    }

    pub fn generated_parameter_id(&self) -> &str {
        &self.generated_parameter_id
    }
}

/// Ordered, digest-bound transformation contract for one estimator stage.
/// The descriptors name authored construct-level retention/removal and every
/// generated identity without creating a second serialized scientific model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsHocStageProjectionV1 {
    stage_number: u8,
    role: CompiledPlsHocStageRoleV1,
    retained_structural_variable_ids: Vec<String>,
    removed_structural_variable_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    generated_variable_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    generated_relation_ids: Vec<String>,
    projection_identity_sha256: String,
}

impl CompiledPlsHocStageProjectionV1 {
    pub fn stage_number(&self) -> u8 {
        self.stage_number
    }

    pub fn role(&self) -> CompiledPlsHocStageRoleV1 {
        self.role
    }

    pub fn retained_structural_variable_ids(&self) -> &[String] {
        &self.retained_structural_variable_ids
    }

    pub fn removed_structural_variable_ids(&self) -> &[String] {
        &self.removed_structural_variable_ids
    }

    pub fn generated_variable_ids(&self) -> &[String] {
        &self.generated_variable_ids
    }

    pub fn generated_relation_ids(&self) -> &[String] {
        &self.generated_relation_ids
    }

    pub fn projection_identity_sha256(&self) -> &str {
        &self.projection_identity_sha256
    }
}

/// Exact compiler authority for one bounded, non-nested second-order HOC.
/// All generated values are internal identities; authored hypothesis IDs stay
/// separate from technical relations throughout later execution and results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsHigherOrderStagePlanV1 {
    contract_version: String,
    authored_term_id: String,
    output_variable_id: String,
    component_ids: Vec<String>,
    authored_hoc_structural_relation_ids: Vec<String>,
    approach: HigherOrderConstructionApproachV4,
    measurement_type: HigherOrderMeasurementTypeV4,
    loc_measurement_mode: CompiledPlsBlockModeV2,
    hoc_component_mode: CompiledPlsBlockModeV2,
    component_relation_interpretation: CompiledPlsHocComponentRelationInterpretationV1,
    hoc_is_endogenous: bool,
    component_mappings: Vec<CompiledPlsHocComponentMappingV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    technical_paths: Vec<CompiledPlsHocTechnicalPathV1>,
    stage_projections: Vec<CompiledPlsHocStageProjectionV1>,
    point_capability_cell: CapabilityCellReferenceV2,
    bootstrap_capability_cell: CapabilityCellReferenceV2,
}

impl CompiledPlsHigherOrderStagePlanV1 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub fn authored_term_id(&self) -> &str {
        &self.authored_term_id
    }

    pub fn output_variable_id(&self) -> &str {
        &self.output_variable_id
    }

    pub fn component_ids(&self) -> &[String] {
        &self.component_ids
    }

    pub fn authored_hoc_structural_relation_ids(&self) -> &[String] {
        &self.authored_hoc_structural_relation_ids
    }

    pub fn approach(&self) -> &HigherOrderConstructionApproachV4 {
        &self.approach
    }

    pub fn measurement_type(&self) -> &HigherOrderMeasurementTypeV4 {
        &self.measurement_type
    }

    pub fn loc_measurement_mode(&self) -> CompiledPlsBlockModeV2 {
        self.loc_measurement_mode
    }

    pub fn hoc_component_mode(&self) -> CompiledPlsBlockModeV2 {
        self.hoc_component_mode
    }

    pub fn component_relation_interpretation(
        &self,
    ) -> CompiledPlsHocComponentRelationInterpretationV1 {
        self.component_relation_interpretation
    }

    pub fn hoc_is_endogenous(&self) -> bool {
        self.hoc_is_endogenous
    }

    pub fn component_mappings(&self) -> &[CompiledPlsHocComponentMappingV1] {
        &self.component_mappings
    }

    pub fn technical_paths(&self) -> &[CompiledPlsHocTechnicalPathV1] {
        &self.technical_paths
    }

    pub fn stage_projections(&self) -> &[CompiledPlsHocStageProjectionV1] {
        &self.stage_projections
    }

    pub fn point_capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.point_capability_cell
    }

    pub fn bootstrap_capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.bootstrap_capability_cell
    }
}

/// PLS v3 foundation for general recursive path models. The proven v2 scoring
/// plan remains embedded while topology/effect authority, two-stage
/// interaction projection, and additive HOC stage contracts are added.
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    higher_order_stage_plans: Vec<CompiledPlsHigherOrderStagePlanV1>,
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

    pub fn higher_order_stage_plans(&self) -> &[CompiledPlsHigherOrderStagePlanV1] {
        &self.higher_order_stage_plans
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
    HigherOrder(#[from] CompiledPlsHigherOrderV1Error),
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
    let higher_order_stage_plans = compile_pls_higher_order_stage_plans_v1(model)?;
    let two_way_interactions = if higher_order_stage_plans.is_empty() {
        compile_pls_two_way_interactions_v3(model)?
    } else {
        Vec::new()
    };
    let (base_plan, stage_one_projection_scientific_sha256) =
        if !higher_order_stage_plans.is_empty() {
            let projection = compile_pls_higher_order_lower_order_projection_v1(model)?;
            let base_plan = compile_pls_plan_v2(projection.projected_model())?;
            debug_assert_eq!(
                base_plan.scientific_hash(),
                projection.projected_scientific_sha256()
            );
            (
                base_plan,
                Some(projection.projected_scientific_sha256().to_string()),
            )
        } else if two_way_interactions.is_empty() {
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
    Ok(CompiledPlsPlanV3 {
        schema_version: COMPILED_PLS_PLAN_V3_SCHEMA_VERSION,
        model_id: model.id.clone(),
        scientific_hash,
        general_sem_config_sha256: sha256_serialized(config),
        base_plan,
        topology,
        stage_one_projection_scientific_sha256,
        two_way_interactions,
        higher_order_stage_plans,
        effect_estimands,
        auto_selected_effects,
    })
}

/// Exact Rank-1 approach/type/topology predicate. `hoc_is_endogenous` means
/// that at least one researcher-authored structural relation targets the HOC.
pub fn pls_hoc_approach_type_supported_v1(
    approach: &HigherOrderConstructionApproachV4,
    measurement_type: &HigherOrderMeasurementTypeV4,
    hoc_is_endogenous: bool,
) -> bool {
    match approach {
        HigherOrderConstructionApproachV4::RepeatedIndicators => match measurement_type {
            HigherOrderMeasurementTypeV4::ReflectiveReflective
            | HigherOrderMeasurementTypeV4::FormativeReflective => true,
            HigherOrderMeasurementTypeV4::ReflectiveFormative
            | HigherOrderMeasurementTypeV4::FormativeFormative => !hoc_is_endogenous,
        },
        HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators => {
            hoc_is_endogenous
                && matches!(
                    measurement_type,
                    HigherOrderMeasurementTypeV4::ReflectiveFormative
                        | HigherOrderMeasurementTypeV4::FormativeFormative
                )
        }
        HigherOrderConstructionApproachV4::EmbeddedTwoStage
        | HigherOrderConstructionApproachV4::DisjointTwoStage => true,
        HigherOrderConstructionApproachV4::Hybrid => false,
    }
}

/// Compiles the generated identities and ordered stage projections for the
/// bounded HOC cell. It never converts the scientific model to legacy
/// `ModelSpec`; the SemModelV4 identities remain authoritative throughout.
pub fn compile_pls_higher_order_stage_plans_v1(
    model: &SemModelV4,
) -> Result<Vec<CompiledPlsHigherOrderStagePlanV1>, CompiledPlsHigherOrderV1Error> {
    model.ensure_valid()?;
    let hoc_terms = model
        .derived_terms
        .iter()
        .filter(|term| matches!(term, SemDerivedTermV4::HigherOrder { .. }))
        .collect::<Vec<_>>();
    if hoc_terms.is_empty() {
        return Ok(Vec::new());
    }
    if hoc_terms.len() != 1 {
        return Err(CompiledPlsHigherOrderV1Error::HigherOrderCardinality {
            found: hoc_terms.len(),
        });
    }
    let SemDerivedTermV4::HigherOrder {
        id: term_id,
        output,
        components,
        approach,
        measurement_type,
    } = hoc_terms[0]
    else {
        unreachable!()
    };
    if let Some(other) = model.derived_terms.iter().find(|term| term.id() != term_id) {
        let kind = match other {
            SemDerivedTermV4::Interaction { .. } => "interaction_v1",
            SemDerivedTermV4::InteractionV2 { .. } => "interaction_v2",
            SemDerivedTermV4::HigherOrder { .. } => "higher_order",
            SemDerivedTermV4::Polynomial { .. } => "polynomial",
        };
        return Err(CompiledPlsHigherOrderV1Error::DerivedTermCombination {
            term_id: term_id.clone(),
            other_term_id: other.id().to_string(),
            kind,
        });
    }
    if approach == &HigherOrderConstructionApproachV4::Hybrid {
        return Err(CompiledPlsHigherOrderV1Error::HybridCompatibilityOnly {
            term_id: term_id.clone(),
        });
    }

    for variable in &model.variables {
        if let SemVariableV4::Composite {
            id,
            weighting: CompositeWeightingV4::Unit { .. } | CompositeWeightingV4::Custom { .. },
            ..
        } = variable
        {
            return Err(CompiledPlsHigherOrderV1Error::FixedOrCustomScoring {
                construct_id: id.clone(),
            });
        }
    }

    let variables = model
        .variables
        .iter()
        .map(|variable| (variable.id(), variable))
        .collect::<BTreeMap<_, _>>();
    let expected_loc_mode = hoc_loc_mode_v1(measurement_type);
    let hoc_component_mode = hoc_component_mode_v1(measurement_type);
    let relation_interpretation = match hoc_component_mode {
        CompiledPlsBlockModeV2::ModeA => CompiledPlsHocComponentRelationInterpretationV1::Loading,
        CompiledPlsBlockModeV2::ModeB => {
            CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity
        }
    };
    let mut component_ids = components.clone();
    component_ids.sort();
    for component_id in &component_ids {
        let Some(SemVariableV4::Composite { weighting, .. }) =
            variables.get(component_id.as_str()).copied()
        else {
            return Err(
                CompiledPlsHigherOrderV1Error::NestedOrNonCompositeComponent {
                    component_id: component_id.clone(),
                },
            );
        };
        let actual_mode = match weighting {
            CompositeWeightingV4::ModeA => CompiledPlsBlockModeV2::ModeA,
            CompositeWeightingV4::ModeB => CompiledPlsBlockModeV2::ModeB,
            CompositeWeightingV4::Unit { .. } | CompositeWeightingV4::Custom { .. } => {
                unreachable!("fixed/custom scoring rejected above")
            }
        };
        if actual_mode != expected_loc_mode {
            return Err(CompiledPlsHigherOrderV1Error::ComponentModeMismatch {
                component_id: component_id.clone(),
                expected_mode: expected_loc_mode,
            });
        }
    }

    let component_set = component_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut authored_hoc_relations = Vec::new();
    let mut incoming_hoc_relations = Vec::new();
    for relation in &model.relations {
        match relation {
            SemRelationV4::Structural {
                id,
                source,
                target,
                parameter,
                intercept_parameter,
                ..
            } if source == output || target == output => {
                let opposite = if source == output { target } else { source };
                if component_set.contains(opposite.as_str()) {
                    return Err(CompiledPlsHigherOrderV1Error::AuthoredComponentRelation {
                        output_id: output.clone(),
                        component_id: opposite.clone(),
                        relation_id: id.clone(),
                    });
                }
                let parameter_is_exact = intercept_parameter.is_none()
                    && model.parameters.iter().any(|candidate| {
                        matches!(
                            candidate,
                            SemParameterV4::Free {
                                id: candidate_id,
                                target: SemParameterTargetV4::Regression {
                                    source: parameter_source,
                                    target: parameter_target,
                                },
                                start: None,
                                lower: None,
                                upper: None,
                                equality_label: None,
                                group_overrides,
                                ..
                            } if candidate_id == parameter
                                && parameter_source == source
                                && parameter_target == target
                                && group_overrides.is_empty()
                        )
                    })
                    && !model
                        .constraints
                        .iter()
                        .any(|constraint| constraint_references_parameter(constraint, parameter));
                if !parameter_is_exact {
                    return Err(
                        CompiledPlsHigherOrderV1Error::UnsupportedStructuralParameter {
                            relation_id: id.clone(),
                        },
                    );
                }
                authored_hoc_relations.push(id.clone());
                if target == output {
                    incoming_hoc_relations.push((id.clone(), source.clone()));
                }
            }
            _ if relation_references_variable(relation, output) => {
                return Err(CompiledPlsHigherOrderV1Error::OutputRelationUnsupported {
                    output_id: output.clone(),
                    relation_id: relation.id().to_string(),
                });
            }
            _ => {}
        }
    }
    authored_hoc_relations.sort();
    incoming_hoc_relations.sort();
    let hoc_is_endogenous = !incoming_hoc_relations.is_empty();
    if !pls_hoc_approach_type_supported_v1(approach, measurement_type, hoc_is_endogenous) {
        return Err(
            CompiledPlsHigherOrderV1Error::UnsupportedApproachTypeTopology {
                term_id: term_id.clone(),
                approach: approach.clone(),
                measurement_type: measurement_type.clone(),
                hoc_is_endogenous,
            },
        );
    }

    let mut occupied_identities = model
        .variables
        .iter()
        .map(|value| value.id().to_string())
        .chain(model.relations.iter().map(|value| value.id().to_string()))
        .chain(model.parameters.iter().map(|value| value.id().to_string()))
        .chain(model.constraints.iter().map(|value| value.id().to_string()))
        .chain(
            model
                .derived_terms
                .iter()
                .map(|value| value.id().to_string()),
        )
        .chain(model.variables.iter().filter_map(|value| match value {
            SemVariableV4::Observed { source_column, .. } => Some(source_column.clone()),
            _ => None,
        }))
        .collect::<BTreeSet<_>>();
    let uses_virtual_indicators = matches!(
        approach,
        HigherOrderConstructionApproachV4::RepeatedIndicators
            | HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators
            | HigherOrderConstructionApproachV4::EmbeddedTwoStage
    );
    let mut component_mappings = Vec::new();
    for component_id in &component_ids {
        let generated_score_variable_id = reserve_hoc_generated_identity_v1(
            "score",
            &[term_id, output, component_id],
            &mut occupied_identities,
        )?;
        let generated_component_relation_id = reserve_hoc_generated_identity_v1(
            "component_relation",
            &[term_id, output, component_id],
            &mut occupied_identities,
        )?;
        let generated_component_parameter_id = reserve_hoc_generated_identity_v1(
            "component_parameter",
            &[term_id, output, component_id],
            &mut occupied_identities,
        )?;
        let (component_relation_source_id, component_relation_target_id) =
            match relation_interpretation {
                CompiledPlsHocComponentRelationInterpretationV1::Loading => {
                    (output.clone(), component_id.clone())
                }
                CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity => {
                    (component_id.clone(), output.clone())
                }
            };
        let mut virtual_indicators = Vec::new();
        if uses_virtual_indicators {
            let mut indicators = model
                .relations
                .iter()
                .filter_map(|relation| match relation {
                    SemRelationV4::MeasurementEffect {
                        id,
                        construct,
                        indicator,
                        ..
                    } if construct == component_id => Some((id.clone(), indicator.clone())),
                    SemRelationV4::MeasurementCausal {
                        id,
                        composite,
                        indicator,
                        ..
                    } if composite == component_id => Some((id.clone(), indicator.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>();
            indicators.sort();
            for (source_relation_id, indicator_id) in indicators {
                let source_column = match variables.get(indicator_id.as_str()).copied() {
                    Some(SemVariableV4::Observed { source_column, .. }) => source_column.clone(),
                    _ => unreachable!("validated measurement relation references observed data"),
                };
                let identity_parts = [
                    term_id.as_str(),
                    output.as_str(),
                    component_id.as_str(),
                    source_relation_id.as_str(),
                    indicator_id.as_str(),
                ];
                virtual_indicators.push(CompiledPlsHocVirtualIndicatorV1 {
                    source_indicator_variable_id: indicator_id,
                    source_column,
                    generated_variable_id: reserve_hoc_generated_identity_v1(
                        "virtual_indicator",
                        &identity_parts,
                        &mut occupied_identities,
                    )?,
                    generated_source_column_id: reserve_hoc_generated_identity_v1(
                        "virtual_source_column",
                        &identity_parts,
                        &mut occupied_identities,
                    )?,
                    generated_relation_id: reserve_hoc_generated_identity_v1(
                        "virtual_relation",
                        &identity_parts,
                        &mut occupied_identities,
                    )?,
                    generated_parameter_id: reserve_hoc_generated_identity_v1(
                        "virtual_parameter",
                        &identity_parts,
                        &mut occupied_identities,
                    )?,
                });
            }
        }
        component_mappings.push(CompiledPlsHocComponentMappingV1 {
            component_id: component_id.clone(),
            loc_measurement_mode: expected_loc_mode,
            generated_score_variable_id,
            generated_component_relation_id,
            generated_component_parameter_id,
            component_relation_source_id,
            component_relation_target_id,
            relation_interpretation,
            virtual_indicators,
        });
    }

    let mut technical_paths = Vec::new();
    if approach == &HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators {
        for (antecedent_relation_id, source_id) in &incoming_hoc_relations {
            for component_id in &component_ids {
                let identity_parts = [
                    term_id.as_str(),
                    output.as_str(),
                    antecedent_relation_id.as_str(),
                    source_id.as_str(),
                    component_id.as_str(),
                ];
                technical_paths.push(CompiledPlsHocTechnicalPathV1 {
                    authored_antecedent_relation_id: antecedent_relation_id.clone(),
                    source_id: source_id.clone(),
                    component_id: component_id.clone(),
                    generated_relation_id: reserve_hoc_generated_identity_v1(
                        "technical_relation",
                        &identity_parts,
                        &mut occupied_identities,
                    )?,
                    generated_parameter_id: reserve_hoc_generated_identity_v1(
                        "technical_parameter",
                        &identity_parts,
                        &mut occupied_identities,
                    )?,
                });
            }
        }
    }
    technical_paths.sort_by(|left, right| {
        left.authored_antecedent_relation_id
            .cmp(&right.authored_antecedent_relation_id)
            .then_with(|| left.component_id.cmp(&right.component_id))
    });

    let stage_projections = build_hoc_stage_projections_v1(
        model,
        term_id,
        output,
        approach,
        &component_ids,
        &component_mappings,
        &technical_paths,
    );
    Ok(vec![CompiledPlsHigherOrderStagePlanV1 {
        contract_version: COMPILED_PLS_HIGHER_ORDER_STAGE_PLAN_V1_VERSION.into(),
        authored_term_id: term_id.clone(),
        output_variable_id: output.clone(),
        component_ids,
        authored_hoc_structural_relation_ids: authored_hoc_relations,
        approach: approach.clone(),
        measurement_type: measurement_type.clone(),
        loc_measurement_mode: expected_loc_mode,
        hoc_component_mode,
        component_relation_interpretation: relation_interpretation,
        hoc_is_endogenous,
        component_mappings,
        technical_paths,
        stage_projections,
        point_capability_cell: pls_general_higher_order_point_capability_cell_v1(),
        bootstrap_capability_cell: pls_general_higher_order_bootstrap_capability_cell_v1(),
    }])
}

/// Ordinary lower-order score authority embedded in CompiledPlsPlanV3. HOC
/// output paths and their exact parameters are projected away; all other
/// SemModelV4 science stays in place and is compiled directly by PLS v2.
pub fn compile_pls_higher_order_lower_order_projection_v1(
    model: &SemModelV4,
) -> Result<CompiledPlsStageOneProjectionV3, CompiledPlsHigherOrderV1Error> {
    let plans = compile_pls_higher_order_stage_plans_v1(model)?;
    if plans.is_empty() {
        let scientific_sha256 = model.scientific_sha256()?;
        return Ok(CompiledPlsStageOneProjectionV3 {
            contract_version: COMPILED_PLS_HIGHER_ORDER_PROJECTION_V1_VERSION.into(),
            source_scientific_sha256: scientific_sha256.clone(),
            projected_scientific_sha256: scientific_sha256,
            projected_model: model.canonicalized(),
        });
    }
    let output_id = plans[0].output_variable_id();
    let removed_relation_ids = model
        .relations
        .iter()
        .filter(|relation| relation_references_variable(relation, output_id))
        .map(|relation| relation.id().to_string())
        .collect::<BTreeSet<_>>();
    let mut removed_parameter_ids = BTreeSet::new();
    for relation in &model.relations {
        if !removed_relation_ids.contains(relation.id()) {
            continue;
        }
        removed_parameter_ids.insert(relation.parameter().to_string());
        if let SemRelationV4::Structural {
            intercept_parameter: Some(parameter),
            ..
        } = relation
        {
            removed_parameter_ids.insert(parameter.clone());
        }
    }
    let source_scientific_sha256 = model.scientific_sha256()?;
    let mut projected_model = model.clone();
    projected_model
        .variables
        .retain(|variable| variable.id() != output_id);
    projected_model
        .relations
        .retain(|relation| !removed_relation_ids.contains(relation.id()));
    projected_model
        .parameters
        .retain(|parameter| !removed_parameter_ids.contains(parameter.id()));
    projected_model.derived_terms.clear();
    projected_model.annotations.clear();
    projected_model.presentation = SemPresentationV4::None;
    projected_model = projected_model.canonicalized();
    projected_model.ensure_valid()?;
    let projected_scientific_sha256 = projected_model.scientific_sha256()?;
    Ok(CompiledPlsStageOneProjectionV3 {
        contract_version: COMPILED_PLS_HIGHER_ORDER_PROJECTION_V1_VERSION.into(),
        source_scientific_sha256,
        projected_scientific_sha256,
        projected_model,
    })
}

fn hoc_loc_mode_v1(measurement_type: &HigherOrderMeasurementTypeV4) -> CompiledPlsBlockModeV2 {
    match measurement_type {
        HigherOrderMeasurementTypeV4::ReflectiveReflective
        | HigherOrderMeasurementTypeV4::ReflectiveFormative => CompiledPlsBlockModeV2::ModeA,
        HigherOrderMeasurementTypeV4::FormativeReflective
        | HigherOrderMeasurementTypeV4::FormativeFormative => CompiledPlsBlockModeV2::ModeB,
    }
}

fn hoc_component_mode_v1(
    measurement_type: &HigherOrderMeasurementTypeV4,
) -> CompiledPlsBlockModeV2 {
    match measurement_type {
        HigherOrderMeasurementTypeV4::ReflectiveReflective
        | HigherOrderMeasurementTypeV4::FormativeReflective => CompiledPlsBlockModeV2::ModeA,
        HigherOrderMeasurementTypeV4::ReflectiveFormative
        | HigherOrderMeasurementTypeV4::FormativeFormative => CompiledPlsBlockModeV2::ModeB,
    }
}

fn reserve_hoc_generated_identity_v1(
    kind: &str,
    values: &[&str],
    occupied: &mut BTreeSet<String>,
) -> Result<String, CompiledPlsHigherOrderV1Error> {
    let mut digest = Sha256::new();
    digest.update(b"qpls.compiled-pls-plan-v3.higher-order\0");
    digest.update((kind.len() as u64).to_be_bytes());
    digest.update(kind.as_bytes());
    for value in values {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    let identity = format!("qpls_hoc_{kind}_v1_{:x}", digest.finalize());
    if !occupied.insert(identity.clone()) {
        return Err(CompiledPlsHigherOrderV1Error::GeneratedIdentityCollision { identity });
    }
    Ok(identity)
}

fn build_hoc_stage_projections_v1(
    model: &SemModelV4,
    term_id: &str,
    output_id: &str,
    approach: &HigherOrderConstructionApproachV4,
    component_ids: &[String],
    mappings: &[CompiledPlsHocComponentMappingV1],
    technical_paths: &[CompiledPlsHocTechnicalPathV1],
) -> Vec<CompiledPlsHocStageProjectionV1> {
    let structural_ids = model
        .variables
        .iter()
        .filter(|variable| {
            !matches!(
                variable,
                SemVariableV4::Observed {
                    role: crate::ObservedRoleV4::Indicator,
                    ..
                }
            )
        })
        .map(|variable| variable.id().to_string())
        .collect::<BTreeSet<_>>();
    let component_set = component_ids.iter().cloned().collect::<BTreeSet<_>>();
    let score_ids = mappings
        .iter()
        .map(|mapping| mapping.generated_score_variable_id.clone())
        .collect::<Vec<_>>();
    let virtual_variable_ids = mappings
        .iter()
        .flat_map(|mapping| {
            mapping
                .virtual_indicators
                .iter()
                .map(|indicator| indicator.generated_variable_id.clone())
        })
        .collect::<Vec<_>>();
    let virtual_relation_ids = mappings
        .iter()
        .flat_map(|mapping| {
            mapping
                .virtual_indicators
                .iter()
                .map(|indicator| indicator.generated_relation_id.clone())
        })
        .collect::<Vec<_>>();
    let component_relation_ids = mappings
        .iter()
        .map(|mapping| mapping.generated_component_relation_id.clone())
        .collect::<Vec<_>>();
    let technical_relation_ids = technical_paths
        .iter()
        .map(|path| path.generated_relation_id.clone())
        .collect::<Vec<_>>();
    let all_structural = structural_ids.iter().cloned().collect::<Vec<_>>();
    let without_hoc = structural_ids
        .iter()
        .filter(|id| id.as_str() != output_id)
        .cloned()
        .collect::<Vec<_>>();
    let without_components = structural_ids
        .iter()
        .filter(|id| !component_set.contains(*id))
        .cloned()
        .collect::<Vec<_>>();

    let mut projections = Vec::new();
    match approach {
        HigherOrderConstructionApproachV4::RepeatedIndicators => {
            projections.push(make_hoc_stage_projection_v1(
                model,
                term_id,
                1,
                CompiledPlsHocStageRoleV1::RepeatedIndicatorEstimation,
                all_structural,
                Vec::new(),
                score_ids.into_iter().chain(virtual_variable_ids).collect(),
                component_relation_ids
                    .into_iter()
                    .chain(virtual_relation_ids)
                    .collect(),
            ));
        }
        HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators => {
            projections.push(make_hoc_stage_projection_v1(
                model,
                term_id,
                1,
                CompiledPlsHocStageRoleV1::ExtendedRepeatedIndicatorEstimation,
                all_structural,
                Vec::new(),
                score_ids.into_iter().chain(virtual_variable_ids).collect(),
                component_relation_ids
                    .into_iter()
                    .chain(virtual_relation_ids)
                    .chain(technical_relation_ids)
                    .collect(),
            ));
        }
        HigherOrderConstructionApproachV4::EmbeddedTwoStage => {
            projections.push(make_hoc_stage_projection_v1(
                model,
                term_id,
                1,
                CompiledPlsHocStageRoleV1::EmbeddedRepeatedIndicatorEstimation,
                all_structural.clone(),
                Vec::new(),
                score_ids
                    .iter()
                    .cloned()
                    .chain(virtual_variable_ids)
                    .collect(),
                component_relation_ids
                    .iter()
                    .cloned()
                    .chain(virtual_relation_ids)
                    .collect(),
            ));
            projections.push(make_hoc_stage_projection_v1(
                model,
                term_id,
                2,
                CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores,
                all_structural,
                Vec::new(),
                score_ids,
                component_relation_ids,
            ));
        }
        HigherOrderConstructionApproachV4::DisjointTwoStage => {
            projections.push(make_hoc_stage_projection_v1(
                model,
                term_id,
                1,
                CompiledPlsHocStageRoleV1::DisjointLowerOrderScoreEstimation,
                without_hoc,
                vec![output_id.to_string()],
                score_ids.clone(),
                Vec::new(),
            ));
            projections.push(make_hoc_stage_projection_v1(
                model,
                term_id,
                2,
                CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores,
                without_components,
                component_ids.to_vec(),
                score_ids,
                component_relation_ids,
            ));
        }
        HigherOrderConstructionApproachV4::Hybrid => unreachable!("hybrid rejected above"),
    }
    projections
}

#[allow(clippy::too_many_arguments)]
fn make_hoc_stage_projection_v1(
    model: &SemModelV4,
    term_id: &str,
    stage_number: u8,
    role: CompiledPlsHocStageRoleV1,
    mut retained_structural_variable_ids: Vec<String>,
    mut removed_structural_variable_ids: Vec<String>,
    mut generated_variable_ids: Vec<String>,
    mut generated_relation_ids: Vec<String>,
) -> CompiledPlsHocStageProjectionV1 {
    retained_structural_variable_ids.sort();
    removed_structural_variable_ids.sort();
    generated_variable_ids.sort();
    generated_relation_ids.sort();
    let projection_identity_sha256 = sha256_serialized(&(
        COMPILED_PLS_HIGHER_ORDER_STAGE_PLAN_V1_VERSION,
        model.scientific_sha256().expect("validated HOC model"),
        term_id,
        stage_number,
        role,
        &retained_structural_variable_ids,
        &removed_structural_variable_ids,
        &generated_variable_ids,
        &generated_relation_ids,
    ));
    CompiledPlsHocStageProjectionV1 {
        stage_number,
        role,
        retained_structural_variable_ids,
        removed_structural_variable_ids,
        generated_variable_ids,
        generated_relation_ids,
        projection_identity_sha256,
    }
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

    fn add_higher_order(
        model: &mut SemModelV4,
        approach: HigherOrderConstructionApproachV4,
        measurement_type: HigherOrderMeasurementTypeV4,
        endogenous: bool,
    ) {
        let output_id = "derived:hoc".to_string();
        model.variables.push(SemVariableV4::Derived {
            id: output_id.clone(),
            label: "Higher order".into(),
        });
        let mut add_path = |id: &str, source: &str, target: &str| {
            let parameter_id = format!("parameter:{id}");
            model.relations.push(SemRelationV4::Structural {
                id: id.into(),
                source: source.into(),
                target: target.into(),
                parameter: parameter_id.clone(),
                role: StructuralRelationRoleV4::Structural,
                intercept_parameter: None,
            });
            model.parameters.push(SemParameterV4::Free {
                id: parameter_id,
                label: format!("{source} -> {target}"),
                target: SemParameterTargetV4::Regression {
                    source: source.into(),
                    target: target.into(),
                },
                start: None,
                lower: None,
                upper: None,
                equality_label: None,
                group_overrides: Vec::new(),
            });
        };
        if endogenous {
            add_path("relation:m2_hoc", "construct:m2", &output_id);
        }
        add_path("relation:hoc_y", &output_id, "construct:y");
        model.derived_terms.push(SemDerivedTermV4::HigherOrder {
            id: "term:hoc".into(),
            output: output_id,
            components: vec!["construct:x".into(), "construct:m1".into()],
            approach,
            measurement_type,
        });
        model.ensure_valid().unwrap();
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

    #[test]
    fn empty_hoc_collection_preserves_the_existing_v3_wire_shape() {
        let plan = compile_pls_plan_v3(&recursive_model(), &GeneralSemConfigV1::default()).unwrap();
        assert!(plan.higher_order_stage_plans().is_empty());
        let encoded = serde_json::to_value(plan).unwrap();
        assert!(encoded.get("higher_order_stage_plans").is_none());
    }

    #[test]
    fn hoc_approach_type_matrix_is_exact_and_hybrid_stays_compatibility_only() {
        use HigherOrderConstructionApproachV4 as Approach;
        use HigherOrderMeasurementTypeV4 as Hcm;

        for endogenous in [false, true] {
            assert!(pls_hoc_approach_type_supported_v1(
                &Approach::RepeatedIndicators,
                &Hcm::ReflectiveReflective,
                endogenous,
            ));
            assert!(pls_hoc_approach_type_supported_v1(
                &Approach::RepeatedIndicators,
                &Hcm::FormativeReflective,
                endogenous,
            ));
            assert_eq!(
                pls_hoc_approach_type_supported_v1(
                    &Approach::RepeatedIndicators,
                    &Hcm::ReflectiveFormative,
                    endogenous,
                ),
                !endogenous
            );
            assert_eq!(
                pls_hoc_approach_type_supported_v1(
                    &Approach::RepeatedIndicators,
                    &Hcm::FormativeFormative,
                    endogenous,
                ),
                !endogenous
            );
            assert_eq!(
                pls_hoc_approach_type_supported_v1(
                    &Approach::ExtendedRepeatedIndicators,
                    &Hcm::ReflectiveFormative,
                    endogenous,
                ),
                endogenous
            );
            assert_eq!(
                pls_hoc_approach_type_supported_v1(
                    &Approach::ExtendedRepeatedIndicators,
                    &Hcm::FormativeFormative,
                    endogenous,
                ),
                endogenous
            );
            for hcm in [
                Hcm::ReflectiveReflective,
                Hcm::ReflectiveFormative,
                Hcm::FormativeReflective,
                Hcm::FormativeFormative,
            ] {
                assert!(pls_hoc_approach_type_supported_v1(
                    &Approach::EmbeddedTwoStage,
                    &hcm,
                    endogenous,
                ));
                assert!(pls_hoc_approach_type_supported_v1(
                    &Approach::DisjointTwoStage,
                    &hcm,
                    endogenous,
                ));
                assert!(!pls_hoc_approach_type_supported_v1(
                    &Approach::Hybrid,
                    &hcm,
                    endogenous,
                ));
            }
        }
        assert!(!pls_hoc_approach_type_supported_v1(
            &Approach::ExtendedRepeatedIndicators,
            &Hcm::ReflectiveReflective,
            true,
        ));
        assert!(!pls_hoc_approach_type_supported_v1(
            &Approach::ExtendedRepeatedIndicators,
            &Hcm::FormativeReflective,
            true,
        ));
    }

    #[test]
    fn disjoint_hoc_plan_binds_stable_mappings_stages_and_exact_cells() {
        let mut model = recursive_model();
        add_higher_order(
            &mut model,
            HigherOrderConstructionApproachV4::DisjointTwoStage,
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            false,
        );
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let hoc = &plan.higher_order_stage_plans()[0];
        assert_eq!(hoc.authored_term_id(), "term:hoc");
        assert_eq!(
            hoc.component_ids(),
            &["construct:m1".to_string(), "construct:x".to_string()]
        );
        assert_eq!(hoc.loc_measurement_mode(), CompiledPlsBlockModeV2::ModeA);
        assert_eq!(hoc.hoc_component_mode(), CompiledPlsBlockModeV2::ModeA);
        assert_eq!(
            hoc.component_relation_interpretation(),
            CompiledPlsHocComponentRelationInterpretationV1::Loading
        );
        assert_eq!(hoc.stage_projections().len(), 2);
        assert_eq!(
            hoc.stage_projections()[0].role(),
            CompiledPlsHocStageRoleV1::DisjointLowerOrderScoreEstimation
        );
        assert_eq!(
            hoc.stage_projections()[1].role(),
            CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores
        );
        assert_eq!(
            hoc.point_capability_cell(),
            &pls_general_higher_order_point_capability_cell_v1()
        );
        assert_eq!(
            hoc.bootstrap_capability_cell(),
            &pls_general_higher_order_bootstrap_capability_cell_v1()
        );
        assert!(
            hoc.component_mappings()
                .iter()
                .all(|mapping| mapping.virtual_indicators().is_empty())
        );
        assert_eq!(
            plan.stage_one_projection_scientific_sha256(),
            Some(plan.base_plan().scientific_hash())
        );

        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        let SemDerivedTermV4::HigherOrder { components, .. } = &mut reordered.derived_terms[0]
        else {
            unreachable!()
        };
        components.reverse();
        assert_eq!(
            compile_pls_plan_v3(&reordered, &GeneralSemConfigV1::default()).unwrap(),
            plan
        );
    }

    #[test]
    fn repeated_and_extended_hoc_contracts_generate_only_their_required_technical_ids() {
        let mut repeated = recursive_model();
        add_higher_order(
            &mut repeated,
            HigherOrderConstructionApproachV4::RepeatedIndicators,
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            false,
        );
        let repeated_plan = compile_pls_higher_order_stage_plans_v1(&repeated).unwrap();
        assert_eq!(repeated_plan[0].stage_projections().len(), 1);
        assert!(repeated_plan[0].technical_paths().is_empty());
        assert!(
            repeated_plan[0]
                .component_mappings()
                .iter()
                .all(|mapping| !mapping.virtual_indicators().is_empty())
        );

        let mut extended = recursive_model();
        add_higher_order(
            &mut extended,
            HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators,
            HigherOrderMeasurementTypeV4::ReflectiveFormative,
            true,
        );
        let extended_plan = compile_pls_higher_order_stage_plans_v1(&extended).unwrap();
        assert_eq!(extended_plan[0].stage_projections().len(), 1);
        assert_eq!(extended_plan[0].technical_paths().len(), 2);
        assert_eq!(
            extended_plan[0].component_relation_interpretation(),
            CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity
        );
    }

    #[test]
    fn topology_and_derived_combinations_fail_with_hoc_specific_errors() {
        let mut endogenous_repeated_formative = recursive_model();
        add_higher_order(
            &mut endogenous_repeated_formative,
            HigherOrderConstructionApproachV4::RepeatedIndicators,
            HigherOrderMeasurementTypeV4::ReflectiveFormative,
            true,
        );
        assert!(matches!(
            compile_pls_higher_order_stage_plans_v1(&endogenous_repeated_formative),
            Err(CompiledPlsHigherOrderV1Error::UnsupportedApproachTypeTopology { .. })
        ));

        let mut combined = recursive_model();
        add_higher_order(
            &mut combined,
            HigherOrderConstructionApproachV4::DisjointTwoStage,
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            false,
        );
        let focal_relation_id = relation_id(&combined, "construct:x", "construct:y");
        add_two_way_interaction(
            &mut combined,
            "interaction:x_by_m1",
            "construct:m1",
            &focal_relation_id,
        );
        combined.ensure_valid().unwrap();
        assert!(matches!(
            compile_pls_higher_order_stage_plans_v1(&combined),
            Err(CompiledPlsHigherOrderV1Error::DerivedTermCombination {
                kind: "interaction_v2",
                ..
            })
        ));
    }
}
