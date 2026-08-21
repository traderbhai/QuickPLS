use crate::{
    CapabilityCellReferenceV2, CompiledPlsBlockModeV2, CompiledPlsPlanV2, CompiledPlsPlanV2Error,
    CompiledPlsThreeWayInteractionErrorV1, CompiledPlsThreeWayInteractionV1,
    CompiledPlsTwoWayModeratedMediationTargetErrorV1, CompiledPlsTwoWayModeratedMediationTargetV1,
    CompiledSemSpecificDirectedPathV1, CompiledSemTopologyV1, CompiledSemTopologyV1Error,
    CompositeWeightingV4, GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1, GeneralSemConfigV1,
    GeneralSemConfigV1ValidationError, GeneralSemEffectEstimandV1,
    GeneralSemSpecificPathLimitBehaviorV1, HigherOrderConstructionApproachV4,
    HigherOrderMeasurementTypeV4, InteractionHierarchyPolicyV2, InteractionMethodV4,
    ObservedRoleV4, ObservedScaleV4, SemAnnotationV4, SemConstraintV4, SemDerivedTermV4,
    SemModelV4, SemParameterTargetV4, SemParameterV4, SemPresentationV4, SemRelationV4,
    SemVariableV4, StructuralRelationRoleV4, compile_pls_plan_v2,
    compile_pls_three_way_interaction_v1, compile_pls_two_way_moderated_mediation_target_v1,
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
pub const COMPILED_PLS_DISJOINT_HIGHER_ORDER_STAGE_TWO_PROJECTION_V1_VERSION: &str =
    "qpls.compiled-pls-disjoint-higher-order-stage-two-projection.v1";
pub const COMPILED_PLS_HIGHER_ORDER_REPEATED_PROJECTION_V1_VERSION: &str =
    "qpls.compiled-pls-higher-order-repeated-projection.v1";
pub const COMPILED_PLS_HIGHER_ORDER_SCORE_STAGE_PROJECTION_V1_VERSION: &str =
    "qpls.compiled-pls-higher-order-score-stage-projection.v1";

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

/// Deterministic schema-6 projection for the second stage of the bounded
/// disjoint HOC executor. The projected model contains the authored HOC and
/// ordinary substantive graph, replaces LOC blocks with generated score
/// indicators, and embeds the exact ordinary PLS plan used by the kernel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsDisjointHocStageTwoProjectionV1 {
    contract_version: String,
    source_scientific_sha256: String,
    hoc_stage_plan_sha256: String,
    projected_scientific_sha256: String,
    projected_model: SemModelV4,
    projected_plan: CompiledPlsPlanV2,
}

/// Shared projected-model envelope used by every bounded Rank-1 HOC
/// approach. The historical disjoint name remains as the serialized Rust
/// type so existing internal callers and archived identities are unchanged.
pub type CompiledPlsHocExecutionProjectionV1 = CompiledPlsDisjointHocStageTwoProjectionV1;

impl CompiledPlsDisjointHocStageTwoProjectionV1 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub fn source_scientific_sha256(&self) -> &str {
        &self.source_scientific_sha256
    }

    pub fn hoc_stage_plan_sha256(&self) -> &str {
        &self.hoc_stage_plan_sha256
    }

    pub fn projected_scientific_sha256(&self) -> &str {
        &self.projected_scientific_sha256
    }

    pub fn projected_model(&self) -> &SemModelV4 {
        &self.projected_model
    }

    pub fn projected_plan(&self) -> &CompiledPlsPlanV2 {
        &self.projected_plan
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
    #[error("three-way interaction projection is invalid: {message}")]
    ThreeWayProjection { message: String },
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
    #[error("the supplied compiled PLS v3 plan does not match the higher-order source model")]
    CompiledPlanMismatch,
    #[error("the point-stage executor currently requires disjoint_two_stage construction")]
    DisjointStageTwoRequired,
    #[error(
        "the repeated-indicator projection requires repeated, extended-repeated, or embedded-two-stage construction"
    )]
    RepeatedStageRequired,
    #[error(
        "the generated-score projection requires embedded_two_stage or disjoint_two_stage construction"
    )]
    ScoreStageRequired,
    #[error(
        "disjoint two-stage checkpoint requires measurement-only lower-order component {component_id}; authored structural relation {relation_id} is outside this checkpoint"
    )]
    DisjointComponentStructuralRelation {
        component_id: String,
        relation_id: String,
    },
    #[error(
        "higher-order output {output_id} must be a derived variable before stage-two projection"
    )]
    StageTwoOutputVariableKind { output_id: String },
    #[error(transparent)]
    StageTwoPlan(#[from] CompiledPlsPlanV2Error),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    three_way_interaction: Option<CompiledPlsThreeWayInteractionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    two_way_moderated_mediation_target: Option<CompiledPlsTwoWayModeratedMediationTargetV1>,
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

    pub fn three_way_interaction(&self) -> Option<&CompiledPlsThreeWayInteractionV1> {
        self.three_way_interaction.as_ref()
    }

    pub fn two_way_moderated_mediation_target(
        &self,
    ) -> Option<&CompiledPlsTwoWayModeratedMediationTargetV1> {
        self.two_way_moderated_mediation_target.as_ref()
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
    ThreeWayInteraction(#[from] CompiledPlsThreeWayInteractionErrorV1),
    #[error(transparent)]
    ModeratedMediationTarget(#[from] CompiledPlsTwoWayModeratedMediationTargetErrorV1),
    #[error(transparent)]
    HigherOrder(#[from] CompiledPlsHigherOrderV1Error),
    #[error("compiled PLS v3 does not yet implement lazy specific-path materialization")]
    LazySpecificPathMaterializationNotImplemented,
    #[error("PLS v3 requires an acyclic structural topology")]
    StructuralFeedback,
    #[error(
        "General SEM HOC v1 publishes effects through its typed HOC stages and does not accept generic requested-effect estimands"
    )]
    HigherOrderRequestedEffectsNotExecutable,
    #[error("General SEM three-way moderation v1 does not execute mediated-path estimands")]
    ThreeWayRequestedEffectsNotExecutable,
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
    let three_way_interaction = if higher_order_stage_plans.is_empty() {
        compile_pls_three_way_interaction_v1(model)?
    } else {
        None
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
        } else if two_way_interactions.is_empty() && three_way_interaction.is_none() {
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
    if !higher_order_stage_plans.is_empty() && !config.requested_effect_estimands.is_empty() {
        return Err(CompiledPlsPlanV3Error::HigherOrderRequestedEffectsNotExecutable);
    }
    if three_way_interaction.is_some() && !config.requested_effect_estimands.is_empty() {
        return Err(CompiledPlsPlanV3Error::ThreeWayRequestedEffectsNotExecutable);
    }
    let auto_selected_effects =
        higher_order_stage_plans.is_empty() && config.requested_effect_estimands.is_empty();
    let interaction_outputs = two_way_interactions
        .iter()
        .map(|interaction| interaction.output_variable_id().to_string())
        .chain(
            three_way_interaction
                .iter()
                .map(|interaction| interaction.output_variable_id().to_string()),
        )
        .collect::<BTreeSet<_>>();
    let generated_hierarchy_relation_ids = generated_hierarchy_relation_ids_v3(model);
    let effect_estimands = if !higher_order_stage_plans.is_empty() {
        Vec::new()
    } else if auto_selected_effects {
        compile_all_effect_estimands(
            &topology,
            &interaction_outputs,
            &generated_hierarchy_relation_ids,
        )
    } else {
        compile_requested_effect_estimands(
            &topology,
            config,
            &interaction_outputs,
            &generated_hierarchy_relation_ids,
        )?
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
        three_way_interaction,
        two_way_moderated_mediation_target: None,
        higher_order_stage_plans,
        effect_estimands,
        auto_selected_effects,
    };
    if plan.three_way_interaction.is_none()
        && !plan.two_way_interactions.is_empty()
        && !config.requested_effect_estimands.is_empty()
    {
        plan.two_way_moderated_mediation_target = Some(
            compile_pls_two_way_moderated_mediation_target_v1(&plan, config)?,
        );
    }
    Ok(plan)
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
                let generated_variable_id = reserve_hoc_generated_identity_v1(
                    "virtual_indicator",
                    &identity_parts,
                    &mut occupied_identities,
                )?;
                let generated_source_column_id = reserve_hoc_generated_identity_v1(
                    "virtual_source_column",
                    &identity_parts,
                    &mut occupied_identities,
                )?;
                let generated_relation_id = reserve_hoc_generated_identity_v1(
                    "virtual_relation",
                    &identity_parts,
                    &mut occupied_identities,
                )?;
                let generated_parameter_id = reserve_hoc_generated_identity_v1(
                    "virtual_parameter",
                    &identity_parts,
                    &mut occupied_identities,
                )?;
                virtual_indicators.push(CompiledPlsHocVirtualIndicatorV1 {
                    source_indicator_variable_id: indicator_id,
                    source_column,
                    generated_variable_id,
                    generated_source_column_id,
                    generated_relation_id,
                    generated_parameter_id,
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

/// Compiles the repeated-indicator model used directly by repeated and
/// extended-repeated HOCs and as stage one for embedded two-stage HOCs.
/// Source indicators are represented by deterministic virtual aliases so the
/// ordinary PLS engine keeps its source-column uniqueness invariant while the
/// scientific SemModelV4 remains the sole authored authority.
pub fn compile_pls_higher_order_repeated_stage_projection_v1(
    model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
) -> Result<CompiledPlsHocExecutionProjectionV1, CompiledPlsHigherOrderV1Error> {
    let source_scientific_sha256 = model.scientific_sha256()?;
    let expected_hoc_plans = compile_pls_higher_order_stage_plans_v1(model)?;
    let expected_stage_one_projection = compile_pls_higher_order_lower_order_projection_v1(model)?;
    let expected_base_plan = compile_pls_plan_v2(expected_stage_one_projection.projected_model())?;
    if plan.scientific_hash() != source_scientific_sha256
        || plan.higher_order_stage_plans() != expected_hoc_plans.as_slice()
        || plan.base_plan() != &expected_base_plan
        || plan.stage_one_projection_scientific_sha256()
            != Some(expected_stage_one_projection.projected_scientific_sha256())
        || expected_hoc_plans.len() != 1
    {
        return Err(CompiledPlsHigherOrderV1Error::CompiledPlanMismatch);
    }
    let hoc = &expected_hoc_plans[0];
    if !matches!(
        hoc.approach(),
        HigherOrderConstructionApproachV4::RepeatedIndicators
            | HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators
            | HigherOrderConstructionApproachV4::EmbeddedTwoStage
    ) {
        return Err(CompiledPlsHigherOrderV1Error::RepeatedStageRequired);
    }

    let mut projected_model = model.clone();
    let output_index = projected_model
        .variables
        .iter()
        .position(|variable| variable.id() == hoc.output_variable_id())
        .ok_or_else(
            || CompiledPlsHigherOrderV1Error::StageTwoOutputVariableKind {
                output_id: hoc.output_variable_id().to_string(),
            },
        )?;
    let (output_id, output_label) = match &projected_model.variables[output_index] {
        SemVariableV4::Derived { id, label } => (id.clone(), label.clone()),
        _ => {
            return Err(CompiledPlsHigherOrderV1Error::StageTwoOutputVariableKind {
                output_id: hoc.output_variable_id().to_string(),
            });
        }
    };
    projected_model.variables[output_index] = SemVariableV4::Composite {
        id: output_id.clone(),
        label: output_label,
        weighting: match hoc.hoc_component_mode() {
            CompiledPlsBlockModeV2::ModeA => CompositeWeightingV4::ModeA,
            CompiledPlsBlockModeV2::ModeB => CompositeWeightingV4::ModeB,
        },
    };
    projected_model.derived_terms.clear();

    for mapping in hoc.component_mappings() {
        for indicator in mapping.virtual_indicators() {
            projected_model.variables.push(SemVariableV4::Observed {
                id: indicator.generated_variable_id().to_string(),
                label: format!(
                    "Repeated indicator for {} from {}",
                    hoc.output_variable_id(),
                    mapping.component_id()
                ),
                source_column: indicator.generated_source_column_id().to_string(),
                scale: ObservedScaleV4::Continuous,
                role: ObservedRoleV4::Indicator,
                categories: Vec::new(),
                value_labels: BTreeMap::new(),
                missing_markers: Vec::new(),
                transformation_lineage: Vec::new(),
            });
            let (relation, target) = match hoc.hoc_component_mode() {
                CompiledPlsBlockModeV2::ModeA => (
                    SemRelationV4::MeasurementEffect {
                        id: indicator.generated_relation_id().to_string(),
                        construct: output_id.clone(),
                        indicator: indicator.generated_variable_id().to_string(),
                        parameter: indicator.generated_parameter_id().to_string(),
                    },
                    SemParameterTargetV4::Loading {
                        construct: output_id.clone(),
                        indicator: indicator.generated_variable_id().to_string(),
                    },
                ),
                CompiledPlsBlockModeV2::ModeB => (
                    SemRelationV4::MeasurementCausal {
                        id: indicator.generated_relation_id().to_string(),
                        indicator: indicator.generated_variable_id().to_string(),
                        composite: output_id.clone(),
                        parameter: indicator.generated_parameter_id().to_string(),
                    },
                    SemParameterTargetV4::Weight {
                        indicator: indicator.generated_variable_id().to_string(),
                        composite: output_id.clone(),
                    },
                ),
            };
            projected_model.relations.push(relation);
            projected_model.parameters.push(SemParameterV4::Free {
                id: indicator.generated_parameter_id().to_string(),
                label: format!("Repeated HOC indicator: {}", mapping.component_id()),
                target,
                start: None,
                lower: None,
                upper: None,
                equality_label: None,
                group_overrides: Vec::new(),
            });
        }
    }

    for path in hoc.technical_paths() {
        projected_model.relations.push(SemRelationV4::Structural {
            id: path.generated_relation_id().to_string(),
            source: path.source_id().to_string(),
            target: path.component_id().to_string(),
            parameter: path.generated_parameter_id().to_string(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        projected_model.parameters.push(SemParameterV4::Free {
            id: path.generated_parameter_id().to_string(),
            label: format!(
                "Extended repeated technical path: {} -> {}",
                path.source_id(),
                path.component_id()
            ),
            target: SemParameterTargetV4::Regression {
                source: path.source_id().to_string(),
                target: path.component_id().to_string(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }
    projected_model.annotations.clear();
    projected_model.presentation = SemPresentationV4::None;
    projected_model = projected_model.canonicalized();
    projected_model.ensure_valid()?;
    let projected_scientific_sha256 = projected_model.scientific_sha256()?;
    let projected_plan = compile_pls_plan_v2(&projected_model)?;
    Ok(CompiledPlsHocExecutionProjectionV1 {
        contract_version: COMPILED_PLS_HIGHER_ORDER_REPEATED_PROJECTION_V1_VERSION.into(),
        source_scientific_sha256,
        hoc_stage_plan_sha256: sha256_serialized(hoc),
        projected_scientific_sha256,
        projected_model,
        projected_plan,
    })
}

/// Compiles the generated-score HOC stage shared by embedded and disjoint
/// two-stage approaches. Embedded retains LOCs and their substantive paths;
/// disjoint preserves its historical removal behavior.
pub fn compile_pls_higher_order_score_stage_projection_v1(
    model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
) -> Result<CompiledPlsDisjointHocStageTwoProjectionV1, CompiledPlsHigherOrderV1Error> {
    let source_scientific_sha256 = model.scientific_sha256()?;
    let expected_hoc_plans = compile_pls_higher_order_stage_plans_v1(model)?;
    let expected_stage_one_projection = compile_pls_higher_order_lower_order_projection_v1(model)?;
    let expected_base_plan = compile_pls_plan_v2(expected_stage_one_projection.projected_model())?;
    if plan.scientific_hash() != source_scientific_sha256
        || plan.higher_order_stage_plans() != expected_hoc_plans.as_slice()
        || plan.base_plan() != &expected_base_plan
        || plan.stage_one_projection_scientific_sha256()
            != Some(expected_stage_one_projection.projected_scientific_sha256())
        || expected_hoc_plans.len() != 1
    {
        return Err(CompiledPlsHigherOrderV1Error::CompiledPlanMismatch);
    }
    let hoc = &expected_hoc_plans[0];
    if !matches!(
        hoc.approach(),
        HigherOrderConstructionApproachV4::EmbeddedTwoStage
            | HigherOrderConstructionApproachV4::DisjointTwoStage
    ) {
        return Err(CompiledPlsHigherOrderV1Error::ScoreStageRequired);
    }
    let remove_components = hoc.approach() == &HigherOrderConstructionApproachV4::DisjointTwoStage;
    let component_ids = hoc
        .component_ids()
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for relation in &model.relations {
        if !remove_components {
            break;
        }
        let SemRelationV4::Structural {
            id, source, target, ..
        } = relation
        else {
            continue;
        };
        if let Some(component_id) = [source.as_str(), target.as_str()]
            .into_iter()
            .find(|candidate| component_ids.contains(candidate))
        {
            return Err(
                CompiledPlsHigherOrderV1Error::DisjointComponentStructuralRelation {
                    component_id: component_id.to_string(),
                    relation_id: id.clone(),
                },
            );
        }
    }

    let component_indicator_ids = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::MeasurementEffect {
                construct,
                indicator,
                ..
            } if component_ids.contains(construct.as_str()) => Some(indicator.clone()),
            SemRelationV4::MeasurementCausal {
                composite,
                indicator,
                ..
            } if component_ids.contains(composite.as_str()) => Some(indicator.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let removed_variable_ids = if remove_components {
        component_ids
            .iter()
            .map(|value| (*value).to_string())
            .chain(component_indicator_ids.iter().cloned())
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let removed_relation_ids = model
        .relations
        .iter()
        .filter(|relation| {
            removed_variable_ids
                .iter()
                .any(|variable_id| relation_references_variable(relation, variable_id))
        })
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

    let mut projected_model = model.clone();
    projected_model
        .variables
        .retain(|variable| !removed_variable_ids.contains(variable.id()));
    let output_index = projected_model
        .variables
        .iter()
        .position(|variable| variable.id() == hoc.output_variable_id())
        .ok_or_else(
            || CompiledPlsHigherOrderV1Error::StageTwoOutputVariableKind {
                output_id: hoc.output_variable_id().to_string(),
            },
        )?;
    let (output_id, output_label) = match &projected_model.variables[output_index] {
        SemVariableV4::Derived { id, label } => (id.clone(), label.clone()),
        _ => {
            return Err(CompiledPlsHigherOrderV1Error::StageTwoOutputVariableKind {
                output_id: hoc.output_variable_id().to_string(),
            });
        }
    };
    projected_model.variables[output_index] = SemVariableV4::Composite {
        id: output_id.clone(),
        label: output_label,
        weighting: match hoc.hoc_component_mode() {
            CompiledPlsBlockModeV2::ModeA => CompositeWeightingV4::ModeA,
            CompiledPlsBlockModeV2::ModeB => CompositeWeightingV4::ModeB,
        },
    };
    projected_model
        .relations
        .retain(|relation| !removed_relation_ids.contains(relation.id()));
    projected_model
        .parameters
        .retain(|parameter| !removed_parameter_ids.contains(parameter.id()));
    projected_model.derived_terms.clear();

    for mapping in hoc.component_mappings() {
        let generated_id = mapping.generated_score_variable_id().to_string();
        projected_model.variables.push(SemVariableV4::Observed {
            id: generated_id.clone(),
            label: format!("Lower-order component score: {}", mapping.component_id()),
            source_column: generated_id.clone(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Indicator,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        let (relation, target) = match mapping.relation_interpretation() {
            CompiledPlsHocComponentRelationInterpretationV1::Loading => (
                SemRelationV4::MeasurementEffect {
                    id: mapping.generated_component_relation_id().to_string(),
                    construct: output_id.clone(),
                    indicator: generated_id.clone(),
                    parameter: mapping.generated_component_parameter_id().to_string(),
                },
                SemParameterTargetV4::Loading {
                    construct: output_id.clone(),
                    indicator: generated_id.clone(),
                },
            ),
            CompiledPlsHocComponentRelationInterpretationV1::WeightAndCollinearity => (
                SemRelationV4::MeasurementCausal {
                    id: mapping.generated_component_relation_id().to_string(),
                    indicator: generated_id.clone(),
                    composite: output_id.clone(),
                    parameter: mapping.generated_component_parameter_id().to_string(),
                },
                SemParameterTargetV4::Weight {
                    indicator: generated_id,
                    composite: output_id.clone(),
                },
            ),
        };
        projected_model.relations.push(relation);
        projected_model.parameters.push(SemParameterV4::Free {
            id: mapping.generated_component_parameter_id().to_string(),
            label: format!("HOC component relation: {}", mapping.component_id()),
            target,
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }
    projected_model.annotations.clear();
    projected_model.presentation = SemPresentationV4::None;
    projected_model = projected_model.canonicalized();
    projected_model.ensure_valid()?;
    let projected_scientific_sha256 = projected_model.scientific_sha256()?;
    let projected_plan = compile_pls_plan_v2(&projected_model)?;
    debug_assert_eq!(
        projected_plan.scientific_hash(),
        projected_scientific_sha256
    );
    Ok(CompiledPlsDisjointHocStageTwoProjectionV1 {
        contract_version: if remove_components {
            COMPILED_PLS_DISJOINT_HIGHER_ORDER_STAGE_TWO_PROJECTION_V1_VERSION
        } else {
            COMPILED_PLS_HIGHER_ORDER_SCORE_STAGE_PROJECTION_V1_VERSION
        }
        .into(),
        source_scientific_sha256,
        hoc_stage_plan_sha256: sha256_serialized(hoc),
        projected_scientific_sha256,
        projected_model,
        projected_plan,
    })
}

/// Backward-compatible exact disjoint wrapper retained for existing internal
/// callers and archived authority checks.
pub fn compile_pls_disjoint_higher_order_stage_two_projection_v1(
    model: &SemModelV4,
    plan: &CompiledPlsPlanV3,
) -> Result<CompiledPlsDisjointHocStageTwoProjectionV1, CompiledPlsHigherOrderV1Error> {
    let projection = compile_pls_higher_order_score_stage_projection_v1(model, plan)?;
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(CompiledPlsHigherOrderV1Error::CompiledPlanMismatch);
    };
    if hoc.approach() != &HigherOrderConstructionApproachV4::DisjointTwoStage {
        return Err(CompiledPlsHigherOrderV1Error::DisjointStageTwoRequired);
    }
    Ok(projection)
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
        if operands.len() == 3 {
            continue;
        }
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
    let three_way = compile_pls_three_way_interaction_v1(model).map_err(|error| {
        CompiledPlsInteractionV3Error::ThreeWayProjection {
            message: error.to_string(),
        }
    })?;
    let source_scientific_sha256 = model.scientific_sha256()?;
    if interactions.is_empty() && three_way.is_none() {
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
        .chain(
            three_way
                .iter()
                .map(|interaction| interaction.output_variable_id()),
        )
        .collect::<BTreeSet<_>>();
    let relation_ids = interactions
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id())
        .chain(
            three_way
                .iter()
                .map(|interaction| interaction.interaction_effect_relation_id()),
        )
        .collect::<BTreeSet<_>>();
    let parameter_ids = interactions
        .iter()
        .map(|interaction| interaction.interaction_effect_parameter_id())
        .chain(
            three_way
                .iter()
                .map(|interaction| interaction.interaction_effect_parameter_id()),
        )
        .collect::<BTreeSet<_>>();
    let binary_three_way_indicator_ids = three_way
        .iter()
        .flat_map(|interaction| {
            [
                (
                    interaction.first_moderator_id(),
                    interaction.first_moderator_scale(),
                ),
                (
                    interaction.second_moderator_id(),
                    interaction.second_moderator_scale(),
                ),
            ]
        })
        .filter_map(|(moderator_id, scale)| {
            (scale == crate::CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne)
                .then(|| exact_single_indicator_id_v3(model, moderator_id))
                .flatten()
        })
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
    // The established PLS score compiler accepts numeric continuous columns.
    // An authored exact 0/1 single-indicator moderator is therefore projected
    // as a numeric scoring column only inside this generated stage-one model.
    // The source SemModelV4 and compiled three-way contract retain BinaryZeroOne
    // as the scientific probe authority.
    for variable in &mut projected_model.variables {
        if !binary_three_way_indicator_ids.contains(variable.id()) {
            continue;
        }
        if let SemVariableV4::Observed {
            scale,
            categories,
            value_labels,
            ..
        } = variable
        {
            *scale = ObservedScaleV4::Continuous;
            categories.clear();
            value_labels.clear();
        }
    }
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

fn exact_single_indicator_id_v3<'a>(model: &'a SemModelV4, construct_id: &str) -> Option<&'a str> {
    let mut indicators = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::MeasurementEffect {
                construct,
                indicator,
                ..
            } if construct == construct_id => Some(indicator.as_str()),
            SemRelationV4::MeasurementCausal {
                indicator,
                composite,
                ..
            } if composite == construct_id => Some(indicator.as_str()),
            _ => None,
        });
    let indicator = indicators.next()?;
    indicators.next().is_none().then_some(indicator)
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

fn generated_hierarchy_relation_ids_v3(model: &SemModelV4) -> BTreeSet<String> {
    let structural_relation_ids = model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural { id, .. } => Some(id.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    model
        .annotations
        .iter()
        .filter_map(|annotation| match annotation {
            SemAnnotationV4::Note { id, subject, .. }
                if id.starts_with("general-sem:v1:interaction-generated:")
                    && structural_relation_ids.contains(subject.as_str()) =>
            {
                Some(subject.clone())
            }
            _ => None,
        })
        .collect()
}

fn compile_requested_effect_estimands(
    topology: &CompiledSemTopologyV1,
    config: &GeneralSemConfigV1,
    excluded_sources: &BTreeSet<String>,
    excluded_relation_ids: &BTreeSet<String>,
) -> Result<Vec<CompiledPlsEffectEstimandV3>, CompiledPlsPlanV3Error> {
    let reserved_specific_path_identities = topology
        .specific_directed_paths()
        .iter()
        .filter(|path| {
            !excluded_sources.contains(path.source())
                && path
                    .relation_ids()
                    .iter()
                    .all(|relation_id| !excluded_relation_ids.contains(relation_id))
        })
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
                            && path
                                .relation_ids()
                                .iter()
                                .all(|relation_id| !excluded_relation_ids.contains(relation_id))
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
                let path_ids = indirect_path_ids(
                    topology,
                    source_id,
                    target_id,
                    excluded_sources,
                    excluded_relation_ids,
                );
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
                let direct_relation_ids = direct_relation_ids(
                    topology,
                    source_id,
                    target_id,
                    excluded_sources,
                    excluded_relation_ids,
                );
                let path_ids = indirect_path_ids(
                    topology,
                    source_id,
                    target_id,
                    excluded_sources,
                    excluded_relation_ids,
                );
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
    excluded_relation_ids: &BTreeSet<String>,
) -> Vec<CompiledPlsEffectEstimandV3> {
    let mut compiled = topology
        .specific_directed_paths()
        .iter()
        .filter(|path| {
            !excluded_sources.contains(path.source())
                && path
                    .relation_ids()
                    .iter()
                    .all(|relation_id| !excluded_relation_ids.contains(relation_id))
        })
        .map(|path| specific_estimand(path.identity().to_string(), path))
        .collect::<Vec<_>>();
    let mut pairs = BTreeSet::new();
    for relation in topology.structural_relations() {
        if relation.role() == StructuralRelationRoleV4::Structural {
            if excluded_sources.contains(relation.source())
                || excluded_relation_ids.contains(relation.relation_id())
            {
                continue;
            }
            pairs.insert((relation.source().to_string(), relation.target().to_string()));
        }
    }
    for path in topology.specific_directed_paths() {
        if excluded_sources.contains(path.source())
            || path
                .relation_ids()
                .iter()
                .any(|relation_id| excluded_relation_ids.contains(relation_id))
        {
            continue;
        }
        pairs.insert((path.source().to_string(), path.target().to_string()));
    }
    for (source_id, target_id) in pairs {
        let path_ids = indirect_path_ids(
            topology,
            &source_id,
            &target_id,
            excluded_sources,
            excluded_relation_ids,
        );
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
                excluded_relation_ids,
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
    excluded_relation_ids: &BTreeSet<String>,
) -> Vec<String> {
    if excluded_sources.contains(source) {
        return Vec::new();
    }
    let mut identities = topology
        .specific_directed_paths()
        .iter()
        .filter(|path| {
            path.source() == source
                && path.target() == target
                && path
                    .relation_ids()
                    .iter()
                    .all(|relation_id| !excluded_relation_ids.contains(relation_id))
        })
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
    excluded_relation_ids: &BTreeSet<String>,
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
                && !excluded_relation_ids.contains(relation.relation_id())
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
        Construct, GeneralSemEffectEstimandV1, GeneralSemInferenceV1, InteractionHierarchyPolicyV2,
        InteractionMethodV4, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec,
        SemDerivedTermV4, SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
        StructuralPath, convert_legacy_basic_model_v4,
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

    fn measurement_only_disjoint_hoc_model() -> SemModelV4 {
        let mut model = recursive_model();
        let removed_parameters = model
            .relations
            .iter()
            .filter_map(|relation| match relation {
                SemRelationV4::Structural { parameter, .. } => Some(parameter.clone()),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        model
            .relations
            .retain(|relation| !matches!(relation, SemRelationV4::Structural { .. }));
        model
            .parameters
            .retain(|parameter| !removed_parameters.contains(parameter.id()));
        add_higher_order(
            &mut model,
            HigherOrderConstructionApproachV4::DisjointTwoStage,
            HigherOrderMeasurementTypeV4::ReflectiveReflective,
            false,
        );
        model
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
    fn generated_hierarchy_relations_remain_in_estimation_but_not_effect_discovery() {
        let mut model = recursive_model();
        let generated_relation_id = relation_id(&model, "construct:x", "construct:m2");
        model.annotations.push(SemAnnotationV4::Note {
            id: format!(
                "general-sem:v1:interaction-generated:{}",
                generated_relation_id
            ),
            subject: generated_relation_id.clone(),
            text: "QuickPLS-generated strong-hierarchy dependency.".into(),
        });
        let complete_topology = compile_sem_topology_v1(&model, 100).unwrap();
        let excluded_path_ids = complete_topology
            .specific_directed_paths()
            .iter()
            .filter(|path| path.relation_ids().contains(&generated_relation_id))
            .map(|path| path.identity().to_string())
            .collect::<BTreeSet<_>>();
        assert!(!excluded_path_ids.is_empty());

        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        assert!(
            plan.topology()
                .structural_relations()
                .iter()
                .any(|relation| relation.relation_id() == generated_relation_id)
        );
        for estimand in plan.effect_estimands() {
            match estimand {
                CompiledPlsEffectEstimandV3::SpecificIndirect {
                    ordered_relation_ids,
                    ..
                } => assert!(!ordered_relation_ids.contains(&generated_relation_id)),
                CompiledPlsEffectEstimandV3::TotalIndirect {
                    contributing_path_identities,
                    ..
                } => assert!(
                    contributing_path_identities
                        .iter()
                        .all(|identity| !excluded_path_ids.contains(identity))
                ),
                CompiledPlsEffectEstimandV3::TotalEffect {
                    direct_relation_ids,
                    contributing_indirect_path_identities,
                    ..
                } => {
                    assert!(!direct_relation_ids.contains(&generated_relation_id));
                    assert!(
                        contributing_indirect_path_identities
                            .iter()
                            .all(|identity| !excluded_path_ids.contains(identity))
                    );
                }
            }
        }
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
        assert!(plan.effect_estimands().is_empty());
        assert!(!plan.auto_selected_effects());

        let mut requested = GeneralSemConfigV1::default();
        requested.requested_effect_estimands = vec![GeneralSemEffectEstimandV1::TotalEffect {
            estimand_id: "effect:hoc".into(),
            source_id: "construct:x".into(),
            target_id: "construct:y".into(),
        }];
        assert_eq!(
            compile_pls_plan_v3(&model, &requested),
            Err(CompiledPlsPlanV3Error::HigherOrderRequestedEffectsNotExecutable)
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

    #[test]
    fn disjoint_stage_two_projection_removes_locs_and_compiles_generated_score_block() {
        let model = measurement_only_disjoint_hoc_model();
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let projection =
            compile_pls_disjoint_higher_order_stage_two_projection_v1(&model, &plan).unwrap();
        let hoc = &plan.higher_order_stage_plans()[0];

        assert_eq!(
            projection.contract_version(),
            COMPILED_PLS_DISJOINT_HIGHER_ORDER_STAGE_TWO_PROJECTION_V1_VERSION
        );
        assert_eq!(
            projection.source_scientific_sha256(),
            plan.scientific_hash()
        );
        assert_eq!(projection.hoc_stage_plan_sha256(), sha256_serialized(hoc));
        assert_eq!(
            projection.projected_plan().scientific_hash(),
            projection.projected_scientific_sha256()
        );
        assert!(hoc.component_ids().iter().all(|component_id| {
            projection
                .projected_model()
                .variables
                .iter()
                .all(|variable| variable.id() != component_id)
        }));
        let hoc_block = projection
            .projected_plan()
            .blocks()
            .iter()
            .find(|block| block.construct_id() == hoc.output_variable_id())
            .unwrap();
        assert_eq!(hoc_block.mode(), CompiledPlsBlockModeV2::ModeA);
        assert_eq!(
            hoc_block
                .indicators()
                .iter()
                .map(|indicator| indicator.variable_id())
                .collect::<std::collections::BTreeSet<_>>(),
            hoc.component_mappings()
                .iter()
                .map(|mapping| mapping.generated_score_variable_id())
                .collect::<std::collections::BTreeSet<_>>()
        );

        let mut reordered = model;
        reordered.variables.reverse();
        reordered.relations.reverse();
        reordered.parameters.reverse();
        let reordered_plan =
            compile_pls_plan_v3(&reordered, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(
            compile_pls_disjoint_higher_order_stage_two_projection_v1(&reordered, &reordered_plan)
                .unwrap(),
            projection
        );
    }

    #[test]
    fn disjoint_stage_two_projection_blocks_authored_loc_structural_paths() {
        let mut model = measurement_only_disjoint_hoc_model();
        let parameter_id = "parameter:loc_y".to_string();
        model.relations.push(SemRelationV4::Structural {
            id: "relation:loc_y".into(),
            source: "construct:x".into(),
            target: "construct:y".into(),
            parameter: parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter_id,
            label: "LOC -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "construct:x".into(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        assert_eq!(
            compile_pls_disjoint_higher_order_stage_two_projection_v1(&model, &plan),
            Err(
                CompiledPlsHigherOrderV1Error::DisjointComponentStructuralRelation {
                    component_id: "construct:x".into(),
                    relation_id: "relation:loc_y".into(),
                }
            )
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
