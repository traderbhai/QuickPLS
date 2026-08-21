use crate::{
    GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1, GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1,
    GeneralSemPlsInteractionCoefficientV1, GeneralSemPlsProductScaleReceiptV1,
    GeneralSemPlsStructuralCoefficientV1,
};
use qpls_core::{
    CompiledPlsPlanV3, CompiledPlsThreeWayInteractionV1, CompiledPlsThreeWayModeratorScaleV1,
    CompiledPlsTwoWayInteractionV3, GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1, InteractionHierarchyPolicyV2,
    InteractionMethodV4,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeneralSemPlsModeratorProbeKindV1 {
    ContinuousStandardized,
    BinaryZeroOne,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsModeratorProbeV1 {
    pub probe_kind: GeneralSemPlsModeratorProbeKindV1,
    pub probe_index: u32,
    pub reported_value: f64,
    pub standardized_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayProductScaleReceiptV1 {
    pub scale_version: String,
    pub interaction_id: String,
    pub generated_product_column_id: String,
    pub operand_ids: [String; 3],
    pub observation_count: usize,
    pub unstandardized_product_mean: f64,
    pub unstandardized_product_sample_standard_deviation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayCoefficientV1 {
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub interaction_effect_relation_id: String,
    pub interaction_effect_parameter_id: String,
    pub operand_ids: [String; 3],
    pub outcome_id: String,
    pub construction_method: InteractionMethodV4,
    pub hierarchy_policy: InteractionHierarchyPolicyV2,
    pub hierarchy_policy_version: String,
    pub standardized_product_estimate: f64,
    pub scientific_rescaled_delta: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayConditionalInteractionV1 {
    pub target_id: String,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub first_moderator_id: String,
    pub second_moderator_id: String,
    pub second_moderator_probe: GeneralSemPlsModeratorProbeV1,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWaySimpleSlopeV1 {
    pub target_id: String,
    pub interaction_id: String,
    pub focal_relation_id: String,
    pub first_moderator_id: String,
    pub second_moderator_id: String,
    pub first_moderator_probe: GeneralSemPlsModeratorProbeV1,
    pub second_moderator_probe: GeneralSemPlsModeratorProbeV1,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsThreeWayPointResultV1 {
    pub method_version: String,
    pub probe_policy_version: String,
    pub observation_count: usize,
    pub lower_order_product_scale_receipts: Vec<GeneralSemPlsProductScaleReceiptV1>,
    pub three_way_product_scale_receipt: GeneralSemPlsThreeWayProductScaleReceiptV1,
    pub structural_coefficients: Vec<GeneralSemPlsStructuralCoefficientV1>,
    pub lower_order_interaction_coefficients: Vec<GeneralSemPlsInteractionCoefficientV1>,
    pub three_way_coefficient: GeneralSemPlsThreeWayCoefficientV1,
    pub first_moderator_probes: Vec<GeneralSemPlsModeratorProbeV1>,
    pub second_moderator_probes: Vec<GeneralSemPlsModeratorProbeV1>,
    pub conditional_interaction_effects: Vec<GeneralSemPlsThreeWayConditionalInteractionV1>,
    pub simple_slopes: Vec<GeneralSemPlsThreeWaySimpleSlopeV1>,
}

impl GeneralSemPlsThreeWayPointResultV1 {
    pub fn ensure_valid_against_plan_v1(
        &self,
        plan: &CompiledPlsPlanV3,
    ) -> Result<(), GeneralSemPlsThreeWayPointErrorV1> {
        let contract = plan
            .three_way_interaction()
            .ok_or(GeneralSemPlsThreeWayPointErrorV1::NoThreeWayInteraction)?;
        if self.method_version != GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
            || self.probe_policy_version != GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1
            || self.observation_count < 3
            || self.three_way_coefficient.interaction_id != contract.interaction_id()
            || self.three_way_coefficient.operand_ids != *contract.operand_ids()
            || self.three_way_coefficient.focal_relation_id != contract.focal_relation_id()
            || self.three_way_coefficient.interaction_effect_relation_id
                != contract.interaction_effect_relation_id()
            || self.three_way_coefficient.interaction_effect_parameter_id
                != contract.interaction_effect_parameter_id()
            || self.three_way_coefficient.outcome_id != contract.outcome_id()
            || self.three_way_product_scale_receipt.interaction_id != contract.interaction_id()
            || self
                .three_way_product_scale_receipt
                .generated_product_column_id
                != contract.generated_product_column_id()
            || self.three_way_product_scale_receipt.operand_ids != *contract.operand_ids()
            || self.three_way_product_scale_receipt.observation_count != self.observation_count
            || self.three_way_product_scale_receipt.scale_version
                != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
            || self.three_way_coefficient.hierarchy_policy_version
                != GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1
            || !self
                .three_way_product_scale_receipt
                .unstandardized_product_mean
                .is_finite()
            || !self
                .three_way_product_scale_receipt
                .unstandardized_product_sample_standard_deviation
                .is_finite()
            || self
                .three_way_product_scale_receipt
                .unstandardized_product_sample_standard_deviation
                <= f64::EPSILON
        {
            return Err(invalid(
                "three-way result metadata differs from the compiled contract",
            ));
        }
        let expected_delta = self.three_way_coefficient.standardized_product_estimate
            / self
                .three_way_product_scale_receipt
                .unstandardized_product_sample_standard_deviation;
        if expected_delta.to_bits()
            != self
                .three_way_coefficient
                .scientific_rescaled_delta
                .to_bits()
            || !expected_delta.is_finite()
            || self.three_way_coefficient.construction_method != InteractionMethodV4::TwoStage
            || self.three_way_coefficient.hierarchy_policy != InteractionHierarchyPolicyV2::Strong
            || !self
                .three_way_coefficient
                .standardized_product_estimate
                .is_finite()
        {
            return Err(invalid(
                "three-way coefficient scale or policy is inconsistent",
            ));
        }
        let expected_lower = plan
            .two_way_interactions()
            .iter()
            .map(|row| row.interaction_id())
            .collect::<BTreeSet<_>>();
        let actual_lower = self
            .lower_order_interaction_coefficients
            .iter()
            .map(|row| row.interaction_id())
            .collect::<BTreeSet<_>>();
        let actual_receipts = self
            .lower_order_product_scale_receipts
            .iter()
            .map(|row| row.interaction_id())
            .collect::<BTreeSet<_>>();
        let required_lower = contract
            .lower_order_interaction_ids()
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if actual_lower != expected_lower
            || actual_receipts != expected_lower
            || !required_lower.is_subset(&actual_lower)
            || self.lower_order_interaction_coefficients.len() != actual_lower.len()
            || self.lower_order_product_scale_receipts.len() != actual_receipts.len()
            || self
                .lower_order_product_scale_receipts
                .iter()
                .any(|receipt| {
                    receipt.scale_version() != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
                        || receipt.observation_count() != self.observation_count
                        || !receipt.unstandardized_product_mean().is_finite()
                        || !receipt
                            .unstandardized_product_sample_standard_deviation()
                            .is_finite()
                        || receipt.unstandardized_product_sample_standard_deviation()
                            <= f64::EPSILON
                })
            || self.lower_order_interaction_coefficients.iter().any(|row| {
                row.construction_method() != &InteractionMethodV4::TwoStage
                    || row.hierarchy_policy() != InteractionHierarchyPolicyV2::Strong
                    || row.hierarchy_policy_version()
                        != GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1
                    || !row.standardized_product_estimate().is_finite()
                    || !row.raw_product_estimate().is_finite()
            })
        {
            return Err(invalid(
                "three-way lower-order interaction inventory differs from strong hierarchy",
            ));
        }
        for coefficient in &self.lower_order_interaction_coefficients {
            let receipt = self
                .lower_order_product_scale_receipts
                .iter()
                .find(|receipt| receipt.interaction_id() == coefficient.interaction_id())
                .expect("validated lower-order receipt inventory");
            let expected = coefficient.standardized_product_estimate()
                / receipt.unstandardized_product_sample_standard_deviation();
            if expected.to_bits() != coefficient.raw_product_estimate().to_bits() {
                return Err(invalid(
                    "a lower-order interaction coefficient has inconsistent product rescaling",
                ));
            }
        }
        let expected_structural = plan
            .base_plan()
            .paths()
            .iter()
            .map(|path| path.relation_id())
            .collect::<BTreeSet<_>>();
        let actual_structural = self
            .structural_coefficients
            .iter()
            .map(|row| row.relation_id())
            .collect::<BTreeSet<_>>();
        if actual_structural != expected_structural
            || self.structural_coefficients.len() != actual_structural.len()
        {
            return Err(invalid(
                "three-way ordinary joint-stage inventory differs from the compiled plan",
            ));
        }
        if self.conditional_interaction_effects.len() != self.second_moderator_probes.len()
            || self.simple_slopes.len()
                != self.first_moderator_probes.len() * self.second_moderator_probes.len()
            || self.first_moderator_probes.is_empty()
            || self.second_moderator_probes.is_empty()
            || !probe_inventory_matches_scale(
                &self.first_moderator_probes,
                contract.first_moderator_scale(),
            )
            || !probe_inventory_matches_scale(
                &self.second_moderator_probes,
                contract.second_moderator_scale(),
            )
        {
            return Err(invalid("three-way fixed-probe inventory is incomplete"));
        }
        if self
            .structural_coefficients
            .iter()
            .any(|row| !row.estimate().is_finite())
            || self
                .lower_order_interaction_coefficients
                .iter()
                .any(|row| !row.raw_product_estimate().is_finite())
            || self
                .conditional_interaction_effects
                .iter()
                .any(|row| !row.estimate.is_finite())
            || self
                .simple_slopes
                .iter()
                .any(|row| !row.estimate.is_finite())
        {
            return Err(invalid("three-way result contains a non-finite estimate"));
        }
        let xw = pair_coefficient(
            &self.lower_order_interaction_coefficients,
            contract.focal_predictor_id(),
            contract.first_moderator_id(),
        )?;
        let xz = pair_coefficient(
            &self.lower_order_interaction_coefficients,
            contract.focal_predictor_id(),
            contract.second_moderator_id(),
        )?;
        let focal = self
            .structural_coefficients
            .iter()
            .find(|row| row.relation_id() == contract.focal_relation_id())
            .ok_or_else(|| {
                invalid("three-way focal coefficient is absent from the joint equation")
            })?
            .estimate();
        for row in &self.conditional_interaction_effects {
            let probe = &row.second_moderator_probe;
            let expected = xw
                + self.three_way_coefficient.scientific_rescaled_delta * probe.standardized_value;
            if row.target_id
                != conditional_interaction_target_id(contract.interaction_id(), probe.probe_index)
                || row.interaction_id != contract.interaction_id()
                || row.focal_relation_id != contract.focal_relation_id()
                || row.first_moderator_id != contract.first_moderator_id()
                || row.second_moderator_id != contract.second_moderator_id()
                || row.estimate.to_bits() != expected.to_bits()
            {
                return Err(invalid(
                    "three-way conditional-interaction formula or identity is inconsistent",
                ));
            }
        }
        for row in &self.simple_slopes {
            let w = &row.first_moderator_probe;
            let z = &row.second_moderator_probe;
            let expected = focal
                + xw * w.standardized_value
                + xz * z.standardized_value
                + self.three_way_coefficient.scientific_rescaled_delta
                    * w.standardized_value
                    * z.standardized_value;
            if row.target_id
                != simple_slope_target_id(contract.interaction_id(), w.probe_index, z.probe_index)
                || row.interaction_id != contract.interaction_id()
                || row.focal_relation_id != contract.focal_relation_id()
                || row.first_moderator_id != contract.first_moderator_id()
                || row.second_moderator_id != contract.second_moderator_id()
                || row.estimate.to_bits() != expected.to_bits()
            {
                return Err(invalid(
                    "three-way simple-slope formula or identity is inconsistent",
                ));
            }
        }
        Ok(())
    }

    pub fn target_values_v1(&self) -> BTreeMap<String, f64> {
        let mut values = BTreeMap::from([(
            three_way_delta_target_id(&self.three_way_coefficient.interaction_id),
            self.three_way_coefficient.scientific_rescaled_delta,
        )]);
        values.extend(
            self.conditional_interaction_effects
                .iter()
                .map(|row| (row.target_id.clone(), row.estimate)),
        );
        values.extend(
            self.simple_slopes
                .iter()
                .map(|row| (row.target_id.clone(), row.estimate)),
        );
        values
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemPlsThreeWayPointErrorV1 {
    #[error("three-way point estimation was cancelled")]
    Cancelled,
    #[error("compiled PLS v3 plan has no three-way interaction")]
    NoThreeWayInteraction,
    #[error("stage-one construct score is missing for {variable_id}")]
    MissingStageOneScore { variable_id: String },
    #[error("stage-one score {variable_id} has {actual} observations; expected {expected}")]
    ScoreLengthMismatch {
        variable_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("stage-one score {variable_id} is non-finite or constant")]
    InvalidStageOneScore { variable_id: String },
    #[error("at least three observations are required; received {observation_count}")]
    InsufficientObservations { observation_count: usize },
    #[error("interaction {interaction_id} product is non-finite or constant")]
    ConstantProduct { interaction_id: String },
    #[error(
        "outcome {outcome_id} has {predictor_count} predictors but only {observation_count} observations"
    )]
    InsufficientEquationObservations {
        outcome_id: String,
        observation_count: usize,
        predictor_count: usize,
    },
    #[error(
        "stage-two design for outcome {outcome_id} is rank deficient at predictor {predictor_id}"
    )]
    RankDeficient {
        outcome_id: String,
        predictor_id: String,
    },
    #[error("strong-hierarchy lower-order interaction is missing for operands {operands:?}")]
    LowerOrderCoefficientMissing { operands: [String; 2] },
    #[error("three-way point result contract is invalid: {0}")]
    InvalidResultContract(String),
}

#[derive(Clone)]
enum PredictorKind<'a> {
    Ordinary {
        source_id: &'a str,
    },
    TwoWay {
        contract: &'a CompiledPlsTwoWayInteractionV3,
        product_sd: f64,
    },
    ThreeWay {
        contract: &'a CompiledPlsThreeWayInteractionV1,
        product_sd: f64,
    },
}

#[derive(Clone)]
struct PredictorColumn<'a> {
    relation_id: &'a str,
    values: &'a [f64],
    kind: PredictorKind<'a>,
}

pub fn estimate_general_sem_pls_three_way_moderation_v1(
    plan: &CompiledPlsPlanV3,
    stage_one_scores: &BTreeMap<String, Vec<f64>>,
) -> Result<GeneralSemPlsThreeWayPointResultV1, GeneralSemPlsThreeWayPointErrorV1> {
    estimate_general_sem_pls_three_way_moderation_v1_with_control(plan, stage_one_scores, || true)
}

pub fn estimate_general_sem_pls_three_way_moderation_v1_with_control(
    plan: &CompiledPlsPlanV3,
    stage_one_scores: &BTreeMap<String, Vec<f64>>,
    should_continue: impl Fn() -> bool,
) -> Result<GeneralSemPlsThreeWayPointResultV1, GeneralSemPlsThreeWayPointErrorV1> {
    if !should_continue() {
        return Err(GeneralSemPlsThreeWayPointErrorV1::Cancelled);
    }
    let three_way = plan
        .three_way_interaction()
        .ok_or(GeneralSemPlsThreeWayPointErrorV1::NoThreeWayInteraction)?;
    let required_ids = plan
        .base_plan()
        .blocks()
        .iter()
        .map(|block| block.construct_id().to_string())
        .collect::<BTreeSet<_>>();
    let first_id = required_ids
        .iter()
        .next()
        .ok_or(GeneralSemPlsThreeWayPointErrorV1::NoThreeWayInteraction)?;
    let observation_count = stage_one_scores
        .get(first_id)
        .ok_or_else(|| GeneralSemPlsThreeWayPointErrorV1::MissingStageOneScore {
            variable_id: first_id.clone(),
        })?
        .len();
    if observation_count < 3 {
        return Err(
            GeneralSemPlsThreeWayPointErrorV1::InsufficientObservations { observation_count },
        );
    }
    let mut standardized_scores = BTreeMap::new();
    for id in required_ids {
        if !should_continue() {
            return Err(GeneralSemPlsThreeWayPointErrorV1::Cancelled);
        }
        let values = stage_one_scores.get(&id).ok_or_else(|| {
            GeneralSemPlsThreeWayPointErrorV1::MissingStageOneScore {
                variable_id: id.clone(),
            }
        })?;
        if values.len() != observation_count {
            return Err(GeneralSemPlsThreeWayPointErrorV1::ScoreLengthMismatch {
                variable_id: id,
                expected: observation_count,
                actual: values.len(),
            });
        }
        let standardized = standardize(values)
            .ok_or_else(|| GeneralSemPlsThreeWayPointErrorV1::InvalidStageOneScore {
                variable_id: id.clone(),
            })?
            .0;
        standardized_scores.insert(id, standardized);
    }

    let mut products = BTreeMap::<String, Vec<f64>>::new();
    let mut lower_receipts = Vec::new();
    for interaction in plan.two_way_interactions() {
        let left = &standardized_scores[interaction.focal_predictor_id()];
        let right = &standardized_scores[interaction.moderator_id()];
        let raw = left
            .iter()
            .zip(right)
            .map(|(a, b)| a * b)
            .collect::<Vec<_>>();
        let (values, mean, sd) = standardize(&raw).ok_or_else(|| {
            GeneralSemPlsThreeWayPointErrorV1::ConstantProduct {
                interaction_id: interaction.interaction_id().into(),
            }
        })?;
        products.insert(interaction.interaction_id().into(), values);
        lower_receipts.push(GeneralSemPlsProductScaleReceiptV1 {
            scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.into(),
            interaction_id: interaction.interaction_id().into(),
            generated_product_column_id: interaction.generated_product_column_id().into(),
            focal_predictor_id: interaction.focal_predictor_id().into(),
            moderator_id: interaction.moderator_id().into(),
            observation_count,
            unstandardized_product_mean: mean,
            unstandardized_product_sample_standard_deviation: sd,
        });
    }
    lower_receipts.sort_by(|a, b| a.interaction_id.cmp(&b.interaction_id));
    let operands = three_way.operand_ids();
    let raw_three = standardized_scores[&operands[0]]
        .iter()
        .zip(&standardized_scores[&operands[1]])
        .zip(&standardized_scores[&operands[2]])
        .map(|((x, w), z)| x * w * z)
        .collect::<Vec<_>>();
    let (three_values, three_mean, three_sd) = standardize(&raw_three).ok_or_else(|| {
        GeneralSemPlsThreeWayPointErrorV1::ConstantProduct {
            interaction_id: three_way.interaction_id().into(),
        }
    })?;
    products.insert(three_way.interaction_id().into(), three_values);
    let three_receipt = GeneralSemPlsThreeWayProductScaleReceiptV1 {
        scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.into(),
        interaction_id: three_way.interaction_id().into(),
        generated_product_column_id: three_way.generated_product_column_id().into(),
        operand_ids: (*operands).clone(),
        observation_count,
        unstandardized_product_mean: three_mean,
        unstandardized_product_sample_standard_deviation: three_sd,
    };

    let outcomes = plan
        .base_plan()
        .paths()
        .iter()
        .map(|path| path.target().to_string())
        .chain(std::iter::once(three_way.outcome_id().to_string()))
        .collect::<BTreeSet<_>>();
    let mut structural = Vec::new();
    let mut lower_coefficients = Vec::new();
    let mut top_coefficient = None;
    for outcome_id in outcomes {
        let outcome = standardized_scores.get(&outcome_id).ok_or_else(|| {
            GeneralSemPlsThreeWayPointErrorV1::MissingStageOneScore {
                variable_id: outcome_id.clone(),
            }
        })?;
        let mut columns = Vec::new();
        for path in plan
            .base_plan()
            .paths()
            .iter()
            .filter(|path| path.target() == outcome_id)
        {
            columns.push(PredictorColumn {
                relation_id: path.relation_id(),
                values: &standardized_scores[path.source()],
                kind: PredictorKind::Ordinary {
                    source_id: path.source(),
                },
            });
        }
        for interaction in plan
            .two_way_interactions()
            .iter()
            .filter(|interaction| interaction.outcome_id() == outcome_id)
        {
            let sd = lower_receipts
                .iter()
                .find(|receipt| receipt.interaction_id == interaction.interaction_id())
                .expect("lower receipt")
                .unstandardized_product_sample_standard_deviation;
            columns.push(PredictorColumn {
                relation_id: interaction.interaction_effect_relation_id(),
                values: &products[interaction.interaction_id()],
                kind: PredictorKind::TwoWay {
                    contract: interaction,
                    product_sd: sd,
                },
            });
        }
        if three_way.outcome_id() == outcome_id {
            columns.push(PredictorColumn {
                relation_id: three_way.interaction_effect_relation_id(),
                values: &products[three_way.interaction_id()],
                kind: PredictorKind::ThreeWay {
                    contract: three_way,
                    product_sd: three_sd,
                },
            });
        }
        columns.sort_by(|a, b| a.relation_id.cmp(b.relation_id));
        if columns.len() >= observation_count {
            return Err(
                GeneralSemPlsThreeWayPointErrorV1::InsufficientEquationObservations {
                    outcome_id,
                    observation_count,
                    predictor_count: columns.len(),
                },
            );
        }
        let estimates = solve_qr(&columns, outcome, &outcome_id, &should_continue)?;
        for (column, estimate) in columns.into_iter().zip(estimates) {
            match column.kind {
                PredictorKind::Ordinary { source_id } => {
                    structural.push(GeneralSemPlsStructuralCoefficientV1 {
                        relation_id: column.relation_id.into(),
                        source_id: source_id.into(),
                        target_id: outcome_id.clone(),
                        estimate,
                    })
                }
                PredictorKind::TwoWay {
                    contract,
                    product_sd,
                } => lower_coefficients.push(GeneralSemPlsInteractionCoefficientV1 {
                    interaction_id: contract.interaction_id().into(),
                    focal_relation_id: contract.focal_relation_id().into(),
                    interaction_effect_relation_id: contract
                        .interaction_effect_relation_id()
                        .into(),
                    interaction_effect_parameter_id: contract
                        .interaction_effect_parameter_id()
                        .into(),
                    focal_predictor_id: contract.focal_predictor_id().into(),
                    moderator_id: contract.moderator_id().into(),
                    outcome_id: outcome_id.clone(),
                    construction_method: contract.method().clone(),
                    hierarchy_policy: contract.hierarchy_policy(),
                    hierarchy_policy_version: GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1
                        .into(),
                    standardized_product_estimate: estimate,
                    raw_product_estimate: estimate / product_sd,
                }),
                PredictorKind::ThreeWay {
                    contract,
                    product_sd,
                } => {
                    top_coefficient = Some(GeneralSemPlsThreeWayCoefficientV1 {
                        interaction_id: contract.interaction_id().into(),
                        focal_relation_id: contract.focal_relation_id().into(),
                        interaction_effect_relation_id: contract
                            .interaction_effect_relation_id()
                            .into(),
                        interaction_effect_parameter_id: contract
                            .interaction_effect_parameter_id()
                            .into(),
                        operand_ids: (*contract.operand_ids()).clone(),
                        outcome_id: outcome_id.clone(),
                        construction_method: contract.method().clone(),
                        hierarchy_policy: contract.hierarchy_policy(),
                        hierarchy_policy_version:
                            GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1.into(),
                        standardized_product_estimate: estimate,
                        scientific_rescaled_delta: estimate / product_sd,
                    })
                }
            }
        }
    }
    structural.sort_by(|a, b| a.relation_id.cmp(&b.relation_id));
    lower_coefficients.sort_by(|a, b| a.interaction_id.cmp(&b.interaction_id));
    let top_coefficient = top_coefficient
        .ok_or_else(|| invalid("three-way coefficient was not jointly estimated"))?;
    let first_probes = probe_values(
        &standardized_scores[three_way.first_moderator_id()],
        three_way.first_moderator_scale(),
    )?;
    let second_probes = probe_values(
        &standardized_scores[three_way.second_moderator_id()],
        three_way.second_moderator_scale(),
    )?;
    let xw = pair_coefficient(
        &lower_coefficients,
        three_way.focal_predictor_id(),
        three_way.first_moderator_id(),
    )?;
    let xz = pair_coefficient(
        &lower_coefficients,
        three_way.focal_predictor_id(),
        three_way.second_moderator_id(),
    )?;
    let focal = structural
        .iter()
        .find(|row| row.relation_id() == three_way.focal_relation_id())
        .ok_or_else(|| invalid("focal coefficient is missing from the joint equation"))?
        .estimate();
    let mut conditional = Vec::new();
    for z in &second_probes {
        conditional.push(GeneralSemPlsThreeWayConditionalInteractionV1 {
            target_id: conditional_interaction_target_id(three_way.interaction_id(), z.probe_index),
            interaction_id: three_way.interaction_id().into(),
            focal_relation_id: three_way.focal_relation_id().into(),
            first_moderator_id: three_way.first_moderator_id().into(),
            second_moderator_id: three_way.second_moderator_id().into(),
            second_moderator_probe: z.clone(),
            estimate: xw + top_coefficient.scientific_rescaled_delta * z.standardized_value,
        });
    }
    let mut slopes = Vec::new();
    for w in &first_probes {
        for z in &second_probes {
            slopes.push(GeneralSemPlsThreeWaySimpleSlopeV1 {
                target_id: simple_slope_target_id(
                    three_way.interaction_id(),
                    w.probe_index,
                    z.probe_index,
                ),
                interaction_id: three_way.interaction_id().into(),
                focal_relation_id: three_way.focal_relation_id().into(),
                first_moderator_id: three_way.first_moderator_id().into(),
                second_moderator_id: three_way.second_moderator_id().into(),
                first_moderator_probe: w.clone(),
                second_moderator_probe: z.clone(),
                estimate: focal
                    + xw * w.standardized_value
                    + xz * z.standardized_value
                    + top_coefficient.scientific_rescaled_delta
                        * w.standardized_value
                        * z.standardized_value,
            });
        }
    }
    let result = GeneralSemPlsThreeWayPointResultV1 {
        method_version: GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1.into(),
        probe_policy_version: GENERAL_SEM_PLS_THREE_WAY_PROBE_POLICY_VERSION_V1.into(),
        observation_count,
        lower_order_product_scale_receipts: lower_receipts,
        three_way_product_scale_receipt: three_receipt,
        structural_coefficients: structural,
        lower_order_interaction_coefficients: lower_coefficients,
        three_way_coefficient: top_coefficient,
        first_moderator_probes: first_probes,
        second_moderator_probes: second_probes,
        conditional_interaction_effects: conditional,
        simple_slopes: slopes,
    };
    result.ensure_valid_against_plan_v1(plan)?;
    Ok(result)
}

fn pair_coefficient(
    rows: &[GeneralSemPlsInteractionCoefficientV1],
    left: &str,
    right: &str,
) -> Result<f64, GeneralSemPlsThreeWayPointErrorV1> {
    rows.iter()
        .find(|row| {
            BTreeSet::from([row.focal_predictor_id(), row.moderator_id()])
                == BTreeSet::from([left, right])
        })
        .map(GeneralSemPlsInteractionCoefficientV1::raw_product_estimate)
        .ok_or_else(
            || GeneralSemPlsThreeWayPointErrorV1::LowerOrderCoefficientMissing {
                operands: [left.into(), right.into()],
            },
        )
}

fn probe_values(
    values: &[f64],
    scale: CompiledPlsThreeWayModeratorScaleV1,
) -> Result<Vec<GeneralSemPlsModeratorProbeV1>, GeneralSemPlsThreeWayPointErrorV1> {
    let mut unique = values.iter().copied().collect::<Vec<_>>();
    unique.sort_by(f64::total_cmp);
    unique.dedup_by(|a, b| a.to_bits() == b.to_bits());
    match scale {
        CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne => {
            if unique.len() != 2 {
                return Err(invalid(
                    "a binary 0/1 moderator score must retain both categories",
                ));
            }
            Ok(unique
                .into_iter()
                .enumerate()
                .map(
                    |(index, standardized_value)| GeneralSemPlsModeratorProbeV1 {
                        probe_kind: GeneralSemPlsModeratorProbeKindV1::BinaryZeroOne,
                        probe_index: index as u32,
                        reported_value: index as f64,
                        standardized_value,
                    },
                )
                .collect())
        }
        CompiledPlsThreeWayModeratorScaleV1::ContinuousStandardized => Ok([-1.0, 0.0, 1.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| GeneralSemPlsModeratorProbeV1 {
                probe_kind: GeneralSemPlsModeratorProbeKindV1::ContinuousStandardized,
                probe_index: index as u32,
                reported_value: value,
                standardized_value: value,
            })
            .collect()),
    }
}

fn probe_inventory_matches_scale(
    probes: &[GeneralSemPlsModeratorProbeV1],
    scale: CompiledPlsThreeWayModeratorScaleV1,
) -> bool {
    let (expected_kind, expected_values): (_, &[f64]) = match scale {
        CompiledPlsThreeWayModeratorScaleV1::ContinuousStandardized => (
            GeneralSemPlsModeratorProbeKindV1::ContinuousStandardized,
            &[-1.0, 0.0, 1.0],
        ),
        CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne => (
            GeneralSemPlsModeratorProbeKindV1::BinaryZeroOne,
            &[0.0, 1.0],
        ),
    };
    probes.len() == expected_values.len()
        && probes
            .iter()
            .zip(expected_values)
            .enumerate()
            .all(|(index, (probe, expected))| {
                probe.probe_kind == expected_kind
                    && probe.probe_index == index as u32
                    && probe.reported_value.to_bits() == expected.to_bits()
                    && probe.standardized_value.is_finite()
            })
        && match scale {
            CompiledPlsThreeWayModeratorScaleV1::ContinuousStandardized => probes
                .iter()
                .zip(expected_values)
                .all(|(probe, expected)| probe.standardized_value.to_bits() == expected.to_bits()),
            CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne => {
                probes[0].standardized_value < probes[1].standardized_value
            }
        }
}

pub fn three_way_delta_target_id(interaction_id: &str) -> String {
    format!("three_way_delta:{interaction_id}")
}
pub fn conditional_interaction_target_id(interaction_id: &str, z_index: u32) -> String {
    format!("three_way_conditional_xw:{interaction_id}:z{z_index}")
}
pub fn simple_slope_target_id(interaction_id: &str, w_index: u32, z_index: u32) -> String {
    format!("three_way_simple_x:{interaction_id}:w{w_index}:z{z_index}")
}

fn standardize(values: &[f64]) -> Option<(Vec<f64>, f64, f64)> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sum = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>();
    let sd = (sum / (values.len() - 1) as f64).sqrt();
    if !sd.is_finite() || sd <= f64::EPSILON {
        return None;
    }
    Some((
        values.iter().map(|value| (value - mean) / sd).collect(),
        mean,
        sd,
    ))
}

fn solve_qr(
    predictors: &[PredictorColumn<'_>],
    outcome: &[f64],
    outcome_id: &str,
    should_continue: &impl Fn() -> bool,
) -> Result<Vec<f64>, GeneralSemPlsThreeWayPointErrorV1> {
    if predictors.is_empty() {
        return Ok(Vec::new());
    }
    let count = predictors.len();
    let mut q = Vec::<Vec<f64>>::with_capacity(count);
    let mut r = vec![vec![0.0; count]; count];
    let tolerance = (outcome.len() as f64).sqrt() * 1.0e-11;
    for (column_index, predictor) in predictors.iter().enumerate() {
        if !should_continue() {
            return Err(GeneralSemPlsThreeWayPointErrorV1::Cancelled);
        }
        let mut residual = predictor.values.to_vec();
        for previous in 0..column_index {
            let projection = dot(&q[previous], &residual);
            r[previous][column_index] += projection;
            subtract_scaled(&mut residual, &q[previous], projection);
        }
        for previous in 0..column_index {
            let correction = dot(&q[previous], &residual);
            r[previous][column_index] += correction;
            subtract_scaled(&mut residual, &q[previous], correction);
        }
        let norm = dot(&residual, &residual).sqrt();
        if !norm.is_finite() || norm <= tolerance {
            return Err(GeneralSemPlsThreeWayPointErrorV1::RankDeficient {
                outcome_id: outcome_id.into(),
                predictor_id: predictor.relation_id.into(),
            });
        }
        r[column_index][column_index] = norm;
        residual.iter_mut().for_each(|value| *value /= norm);
        q.push(residual);
    }
    let mut coefficients = q
        .iter()
        .map(|column| dot(column, outcome))
        .collect::<Vec<_>>();
    for row in (0..count).rev() {
        if !should_continue() {
            return Err(GeneralSemPlsThreeWayPointErrorV1::Cancelled);
        }
        for column in row + 1..count {
            coefficients[row] -= r[row][column] * coefficients[column];
        }
        coefficients[row] /= r[row][row];
    }
    Ok(coefficients)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}
fn subtract_scaled(target: &mut [f64], source: &[f64], scale: f64) {
    for (target, source) in target.iter_mut().zip(source) {
        *target -= scale * source;
    }
}
fn invalid(message: impl Into<String>) -> GeneralSemPlsThreeWayPointErrorV1 {
    GeneralSemPlsThreeWayPointErrorV1::InvalidResultContract(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuous_and_binary_probe_inventories_are_fixed_and_ordered() {
        assert_eq!(
            probe_values(
                &[-1.2, -0.1, 0.7],
                CompiledPlsThreeWayModeratorScaleV1::ContinuousStandardized
            )
            .unwrap()
            .iter()
            .map(|p| p.reported_value)
            .collect::<Vec<_>>(),
            vec![-1.0, 0.0, 1.0]
        );
        let binary = probe_values(
            &[-0.75, 1.25, -0.75, 1.25],
            CompiledPlsThreeWayModeratorScaleV1::BinaryZeroOne,
        )
        .unwrap();
        assert_eq!(
            binary.iter().map(|p| p.reported_value).collect::<Vec<_>>(),
            vec![0.0, 1.0]
        );
        assert!(
            binary
                .iter()
                .all(|p| p.probe_kind == GeneralSemPlsModeratorProbeKindV1::BinaryZeroOne)
        );
    }

    #[test]
    fn three_way_target_ids_are_stable_and_collision_free() {
        assert_eq!(three_way_delta_target_id("ix3"), "three_way_delta:ix3");
        assert_ne!(
            conditional_interaction_target_id("ix3", 1),
            simple_slope_target_id("ix3", 0, 1)
        );
    }
}
