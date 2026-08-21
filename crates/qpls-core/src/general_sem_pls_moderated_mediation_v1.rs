use crate::{
    CapabilityCellReferenceV2, CompiledPlsPlanV3, CompiledPlsTwoWayInteractionV3,
    GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemEffectEstimandV1,
    GeneralSemInferenceTailV1, GeneralSemInferenceV1, PLS_ALGORITHM_CAPABILITY_ID,
    PLS_ALGORITHM_CAPABILITY_VERSION, PLS_ALGORITHM_CELL_ID,
    pls_general_multiple_moderation_point_capability_cell_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const COMPILED_PLS_TWO_WAY_MODERATED_MEDIATION_TARGET_VERSION_V1: &str =
    "qpls.compiled-pls-two-way-moderated-mediation-target.v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_ID_V1: &str =
    "smartpls.mediation";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_ID_V1: &str =
    "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_VERSION_V1: &str =
    "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_POINT_METHOD_VERSION_V1: &str =
    "general_sem_pls_two_way_moderated_mediation_point_v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_PROBE_POLICY_VERSION_V1: &str =
    "standardized_moderator_minus_one_zero_plus_one_v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CONDITIONAL_TARGET_VERSION_V1: &str =
    "conditional_indirect_effect_v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_INDEX_TARGET_VERSION_V1: &str =
    "index_of_moderated_mediation_v1";
pub const GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_PROBES_V1: [f64; 3] = [-1.0, 0.0, 1.0];

pub fn pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1()
-> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_ID_V1
            .into(),
        cell_id: GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_ID_V1.into(),
        capability_version:
            GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CAPABILITY_VERSION_V1.into(),
    }
}

fn pls_algorithm_capability_cell_v1() -> CapabilityCellReferenceV2 {
    CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_ALGORITHM_CAPABILITY_ID.into(),
        cell_id: PLS_ALGORITHM_CELL_ID.into(),
        capability_version: PLS_ALGORITHM_CAPABILITY_VERSION.into(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompiledPlsTwoWayModeratedMediationStageV1 {
    FirstStage,
    SecondStage,
}

/// Exact scientific target for the bounded two-way moderated-mediation cell.
/// The three capability references make its cross-capability ownership
/// explicit: ordinary PLS supplies scores, the moderation point cell supplies
/// the joint equation and scientific gamma, and the supplemental mediation
/// cell owns the combined bootstrap inference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompiledPlsTwoWayModeratedMediationTargetV1 {
    contract_version: String,
    target_id: String,
    base_pls_capability_cell: CapabilityCellReferenceV2,
    moderation_point_capability_cell: CapabilityCellReferenceV2,
    bootstrap_capability_cell: CapabilityCellReferenceV2,
    estimand_id: String,
    specific_path_identity: String,
    ordered_relation_ids: Vec<String>,
    x_id: String,
    mediator_id: String,
    y_id: String,
    moderator_id: String,
    moderated_stage: CompiledPlsTwoWayModeratedMediationStageV1,
    moderated_relation_id: String,
    other_stage_relation_id: String,
    interaction_id: String,
    interaction_effect_relation_id: String,
    interaction_effect_parameter_id: String,
    generated_product_column_id: String,
    stage_one_model_scientific_sha256: String,
    product_scale_version: String,
    probe_policy_version: String,
    conditional_target_version: String,
    index_target_version: String,
}

impl CompiledPlsTwoWayModeratedMediationTargetV1 {
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn base_pls_capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.base_pls_capability_cell
    }

    pub fn moderation_point_capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.moderation_point_capability_cell
    }

    pub fn bootstrap_capability_cell(&self) -> &CapabilityCellReferenceV2 {
        &self.bootstrap_capability_cell
    }

    pub fn estimand_id(&self) -> &str {
        &self.estimand_id
    }

    pub fn specific_path_identity(&self) -> &str {
        &self.specific_path_identity
    }

    pub fn ordered_relation_ids(&self) -> &[String] {
        &self.ordered_relation_ids
    }

    pub fn x_id(&self) -> &str {
        &self.x_id
    }

    pub fn mediator_id(&self) -> &str {
        &self.mediator_id
    }

    pub fn y_id(&self) -> &str {
        &self.y_id
    }

    pub fn moderator_id(&self) -> &str {
        &self.moderator_id
    }

    pub fn moderated_stage(&self) -> CompiledPlsTwoWayModeratedMediationStageV1 {
        self.moderated_stage
    }

    pub fn moderated_relation_id(&self) -> &str {
        &self.moderated_relation_id
    }

    pub fn other_stage_relation_id(&self) -> &str {
        &self.other_stage_relation_id
    }

    pub fn interaction_id(&self) -> &str {
        &self.interaction_id
    }

    pub fn interaction_effect_relation_id(&self) -> &str {
        &self.interaction_effect_relation_id
    }

    pub fn interaction_effect_parameter_id(&self) -> &str {
        &self.interaction_effect_parameter_id
    }

    pub fn generated_product_column_id(&self) -> &str {
        &self.generated_product_column_id
    }

    pub fn stage_one_model_scientific_sha256(&self) -> &str {
        &self.stage_one_model_scientific_sha256
    }

    pub fn product_scale_version(&self) -> &str {
        &self.product_scale_version
    }

    pub fn probe_policy_version(&self) -> &str {
        &self.probe_policy_version
    }

    pub fn conditional_target_version(&self) -> &str {
        &self.conditional_target_version
    }

    pub fn index_target_version(&self) -> &str {
        &self.index_target_version
    }

    pub fn deterministic_sha256(&self) -> String {
        crate::sha256_serialized(self)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CompiledPlsTwoWayModeratedMediationTargetErrorV1 {
    #[error("two-way moderated mediation requires percentile case-bootstrap inference")]
    BootstrapRequired,
    #[error("two-way moderated mediation requires percentile bootstrap intervals")]
    BootstrapIntervalUnsupported,
    #[error("two-way moderated mediation requires two-sided bootstrap inference")]
    BootstrapTailUnsupported,
    #[error(
        "two-way moderated mediation uses the fixed -1/0/+1 probe policy; authored probes are not supported"
    )]
    AuthoredConditionalProbesUnsupported,
    #[error(
        "two-way moderated mediation requires exactly one requested effect estimand (found {found})"
    )]
    RequestedEffectCardinality { found: usize },
    #[error("two-way moderated mediation requires one requested specific path")]
    RequestedEffectMustBeSpecificPath,
    #[error("two-way moderated mediation requires an exact two-relation path (found {found})")]
    SpecificPathLength { found: usize },
    #[error("requested two-relation path {estimand_id} is not present in the compiled topology")]
    RequestedPathNotCompiled { estimand_id: String },
    #[error(
        "two-way moderated mediation requires exactly one compiled two-way interaction (found {found})"
    )]
    InteractionCardinality { found: usize },
    #[error("the one interaction does not moderate either selected path relation")]
    InteractionDoesNotModerateSelectedPath,
    #[error("moderator {moderator_id} must differ from X, M, and Y")]
    ModeratorOverlapsPath { moderator_id: String },
    #[error("compiled interaction target requires a projected stage-one model digest")]
    MissingStageOneProjection,
}

/// Compiles the exact bounded conditional-process target from an otherwise
/// valid PLS v3 plan. No heuristic path selection is performed.
pub fn compile_pls_two_way_moderated_mediation_target_v1(
    plan: &CompiledPlsPlanV3,
    config: &GeneralSemConfigV1,
) -> Result<
    CompiledPlsTwoWayModeratedMediationTargetV1,
    CompiledPlsTwoWayModeratedMediationTargetErrorV1,
> {
    if config.requested_effect_estimands.len() != 1 {
        return Err(
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::RequestedEffectCardinality {
                found: config.requested_effect_estimands.len(),
            },
        );
    }
    let GeneralSemEffectEstimandV1::SpecificPath {
        estimand_id,
        ordered_relation_ids,
    } = &config.requested_effect_estimands[0]
    else {
        return Err(
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::RequestedEffectMustBeSpecificPath,
        );
    };
    if ordered_relation_ids.len() != 2 {
        return Err(
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::SpecificPathLength {
                found: ordered_relation_ids.len(),
            },
        );
    }
    let path = plan
        .topology()
        .specific_directed_paths()
        .iter()
        .find(|path| path.relation_ids() == ordered_relation_ids)
        .ok_or_else(|| {
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::RequestedPathNotCompiled {
                estimand_id: estimand_id.clone(),
            }
        })?;
    match config.inference {
        GeneralSemInferenceV1::None => {
            return Err(CompiledPlsTwoWayModeratedMediationTargetErrorV1::BootstrapRequired);
        }
        GeneralSemInferenceV1::CaseBootstrap { interval, tail, .. } => {
            if interval != GeneralSemBootstrapIntervalV1::Percentile {
                return Err(
                    CompiledPlsTwoWayModeratedMediationTargetErrorV1::BootstrapIntervalUnsupported,
                );
            }
            if tail != GeneralSemInferenceTailV1::TwoSided {
                return Err(
                    CompiledPlsTwoWayModeratedMediationTargetErrorV1::BootstrapTailUnsupported,
                );
            }
        }
    }
    if !config.conditional_effect_probes.is_empty() {
        return Err(
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::AuthoredConditionalProbesUnsupported,
        );
    }
    if plan.two_way_interactions().len() != 1 {
        return Err(
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::InteractionCardinality {
                found: plan.two_way_interactions().len(),
            },
        );
    }
    let interaction = &plan.two_way_interactions()[0];
    let [x_id, mediator_id, y_id] = path.node_ids() else {
        unreachable!("an exact two-relation compiled path has three nodes")
    };
    if [x_id.as_str(), mediator_id.as_str(), y_id.as_str()].contains(&interaction.moderator_id()) {
        return Err(
            CompiledPlsTwoWayModeratedMediationTargetErrorV1::ModeratorOverlapsPath {
                moderator_id: interaction.moderator_id().into(),
            },
        );
    }
    let (moderated_stage, moderated_relation_id, other_stage_relation_id) =
        classify_moderated_stage(interaction, ordered_relation_ids, x_id, mediator_id, y_id)?;
    let stage_one_model_scientific_sha256 = plan
        .stage_one_projection_scientific_sha256()
        .ok_or(CompiledPlsTwoWayModeratedMediationTargetErrorV1::MissingStageOneProjection)?
        .to_string();
    let mut target = CompiledPlsTwoWayModeratedMediationTargetV1 {
        contract_version: COMPILED_PLS_TWO_WAY_MODERATED_MEDIATION_TARGET_VERSION_V1.into(),
        target_id: String::new(),
        base_pls_capability_cell: pls_algorithm_capability_cell_v1(),
        moderation_point_capability_cell: pls_general_multiple_moderation_point_capability_cell_v1(
        ),
        bootstrap_capability_cell:
            pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1(),
        estimand_id: estimand_id.clone(),
        specific_path_identity: path.identity().into(),
        ordered_relation_ids: ordered_relation_ids.clone(),
        x_id: x_id.clone(),
        mediator_id: mediator_id.clone(),
        y_id: y_id.clone(),
        moderator_id: interaction.moderator_id().into(),
        moderated_stage,
        moderated_relation_id,
        other_stage_relation_id,
        interaction_id: interaction.interaction_id().into(),
        interaction_effect_relation_id: interaction.interaction_effect_relation_id().into(),
        interaction_effect_parameter_id: interaction.interaction_effect_parameter_id().into(),
        generated_product_column_id: interaction.generated_product_column_id().into(),
        stage_one_model_scientific_sha256,
        product_scale_version: crate::GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.into(),
        probe_policy_version: GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_PROBE_POLICY_VERSION_V1
            .into(),
        conditional_target_version:
            GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_CONDITIONAL_TARGET_VERSION_V1.into(),
        index_target_version: GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_INDEX_TARGET_VERSION_V1
            .into(),
    };
    target.target_id = moderated_mediation_target_identity_v1(&target);
    Ok(target)
}

fn classify_moderated_stage(
    interaction: &CompiledPlsTwoWayInteractionV3,
    relation_ids: &[String],
    x_id: &str,
    mediator_id: &str,
    y_id: &str,
) -> Result<
    (CompiledPlsTwoWayModeratedMediationStageV1, String, String),
    CompiledPlsTwoWayModeratedMediationTargetErrorV1,
> {
    if interaction.focal_relation_id() == relation_ids[0].as_str()
        && interaction.focal_predictor_id() == x_id
        && interaction.outcome_id() == mediator_id
    {
        return Ok((
            CompiledPlsTwoWayModeratedMediationStageV1::FirstStage,
            relation_ids[0].clone(),
            relation_ids[1].clone(),
        ));
    }
    if interaction.focal_relation_id() == relation_ids[1].as_str()
        && interaction.focal_predictor_id() == mediator_id
        && interaction.outcome_id() == y_id
    {
        return Ok((
            CompiledPlsTwoWayModeratedMediationStageV1::SecondStage,
            relation_ids[1].clone(),
            relation_ids[0].clone(),
        ));
    }
    Err(CompiledPlsTwoWayModeratedMediationTargetErrorV1::InteractionDoesNotModerateSelectedPath)
}

fn moderated_mediation_target_identity_v1(
    target: &CompiledPlsTwoWayModeratedMediationTargetV1,
) -> String {
    let mut identity = target.clone();
    identity.target_id.clear();
    format!(
        "sem_moderated_mediation_target_v1_{:x}",
        Sha256::digest(
            serde_json::to_vec(&identity).expect("moderated-mediation identity is serializable")
        )
    )
}

pub fn conditional_indirect_effect_identity_v1(target_id: &str, probe_value_index: u32) -> String {
    format!(
        "sem_conditional_indirect_v1_{:x}",
        Sha256::digest(format!("{target_id}\0{probe_value_index}").as_bytes())
    )
}

pub fn moderated_mediation_index_identity_v1(target_id: &str) -> String {
    format!(
        "sem_moderated_mediation_index_v1_{:x}",
        Sha256::digest(target_id.as_bytes())
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsConditionalIndirectPointV1 {
    pub effect_id: String,
    pub target_id: String,
    pub probe_value_index: u32,
    pub moderator_value: f64,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsModeratedMediationIndexPointV1 {
    pub effect_id: String,
    pub target_id: String,
    pub estimate: f64,
}

/// The four derived targets owned by the supplemental bootstrap cell. The
/// scientific-gamma target remains the existing moderation-point target and
/// is deliberately not duplicated here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneralSemPlsTwoWayModeratedMediationInferenceTargetV1 {
    ConditionalIndirect {
        effect_id: String,
        target_id: String,
        target_version: String,
        probe_policy_version: String,
        probe_value_index: u32,
        moderator_value: f64,
    },
    ModeratedMediationIndex {
        effect_id: String,
        target_id: String,
        target_version: String,
    },
}

pub fn general_sem_pls_two_way_moderated_mediation_inference_targets_v1(
    target: &CompiledPlsTwoWayModeratedMediationTargetV1,
) -> Vec<GeneralSemPlsTwoWayModeratedMediationInferenceTargetV1> {
    let mut targets = GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_PROBES_V1
        .into_iter()
        .enumerate()
        .map(|(probe_value_index, moderator_value)| {
            GeneralSemPlsTwoWayModeratedMediationInferenceTargetV1::ConditionalIndirect {
                effect_id: conditional_indirect_effect_identity_v1(
                    target.target_id(),
                    probe_value_index as u32,
                ),
                target_id: target.target_id().into(),
                target_version: target.conditional_target_version().into(),
                probe_policy_version: target.probe_policy_version().into(),
                probe_value_index: probe_value_index as u32,
                moderator_value,
            }
        })
        .collect::<Vec<_>>();
    targets.push(
        GeneralSemPlsTwoWayModeratedMediationInferenceTargetV1::ModeratedMediationIndex {
            effect_id: moderated_mediation_index_identity_v1(target.target_id()),
            target_id: target.target_id().into(),
            target_version: target.index_target_version().into(),
        },
    );
    targets
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsTwoWayModeratedMediationPointResultV1 {
    pub method_version: String,
    pub target_id: String,
    pub moderated_stage: CompiledPlsTwoWayModeratedMediationStageV1,
    pub moderated_stage_coefficient: f64,
    pub other_stage_coefficient: f64,
    pub scientific_gamma: f64,
    pub conditional_indirect_effects: Vec<GeneralSemPlsConditionalIndirectPointV1>,
    pub moderated_mediation_index: GeneralSemPlsModeratedMediationIndexPointV1,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemPlsTwoWayModeratedMediationPointErrorV1 {
    #[error("{quantity} must be finite")]
    NonFiniteInput { quantity: &'static str },
    #[error("moderated-mediation point calculation produced a non-finite {quantity}")]
    NonFiniteResult { quantity: &'static str },
}

pub fn calculate_general_sem_pls_two_way_moderated_mediation_point_v1(
    target: &CompiledPlsTwoWayModeratedMediationTargetV1,
    moderated_stage_coefficient: f64,
    other_stage_coefficient: f64,
    scientific_gamma: f64,
) -> Result<
    GeneralSemPlsTwoWayModeratedMediationPointResultV1,
    GeneralSemPlsTwoWayModeratedMediationPointErrorV1,
> {
    for (quantity, value) in [
        ("moderated_stage_coefficient", moderated_stage_coefficient),
        ("other_stage_coefficient", other_stage_coefficient),
        ("scientific_gamma", scientific_gamma),
    ] {
        if !value.is_finite() {
            return Err(
                GeneralSemPlsTwoWayModeratedMediationPointErrorV1::NonFiniteInput { quantity },
            );
        }
    }
    let mut conditional_indirect_effects = Vec::with_capacity(3);
    for (probe_value_index, moderator_value) in
        GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_PROBES_V1
            .into_iter()
            .enumerate()
    {
        let estimate = (moderated_stage_coefficient + scientific_gamma * moderator_value)
            * other_stage_coefficient;
        if !estimate.is_finite() {
            return Err(
                GeneralSemPlsTwoWayModeratedMediationPointErrorV1::NonFiniteResult {
                    quantity: "conditional_indirect_effect",
                },
            );
        }
        conditional_indirect_effects.push(GeneralSemPlsConditionalIndirectPointV1 {
            effect_id: conditional_indirect_effect_identity_v1(
                target.target_id(),
                probe_value_index as u32,
            ),
            target_id: target.target_id().into(),
            probe_value_index: probe_value_index as u32,
            moderator_value,
            estimate,
        });
    }
    let index = scientific_gamma * other_stage_coefficient;
    if !index.is_finite() {
        return Err(
            GeneralSemPlsTwoWayModeratedMediationPointErrorV1::NonFiniteResult {
                quantity: "index_of_moderated_mediation",
            },
        );
    }
    Ok(GeneralSemPlsTwoWayModeratedMediationPointResultV1 {
        method_version: GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_POINT_METHOD_VERSION_V1.into(),
        target_id: target.target_id().into(),
        moderated_stage: target.moderated_stage(),
        moderated_stage_coefficient,
        other_stage_coefficient,
        scientific_gamma,
        conditional_indirect_effects,
        moderated_mediation_index: GeneralSemPlsModeratedMediationIndexPointV1 {
            effect_id: moderated_mediation_index_identity_v1(target.target_id()),
            target_id: target.target_id().into(),
            estimate: index,
        },
    })
}
