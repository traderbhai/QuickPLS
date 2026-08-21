use qpls_core::{
    CompiledPlsPlanV3, CompiledPlsTwoWayInteractionV3, InteractionHierarchyPolicyV2,
    InteractionMethodV4,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1;
pub const GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1;
pub const GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1;
pub const GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1: &str =
    qpls_core::GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1;

/// Auditable scale transformation for a two-stage product. Both operands are
/// first sample-standardized. Their row-wise product is then mean-centered and
/// divided by its sample standard deviation for the joint stage-two solve.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsProductScaleReceiptV1 {
    scale_version: String,
    interaction_id: String,
    generated_product_column_id: String,
    focal_predictor_id: String,
    moderator_id: String,
    observation_count: usize,
    unstandardized_product_mean: f64,
    unstandardized_product_sample_standard_deviation: f64,
}

impl GeneralSemPlsProductScaleReceiptV1 {
    pub fn scale_version(&self) -> &str {
        &self.scale_version
    }

    pub fn interaction_id(&self) -> &str {
        &self.interaction_id
    }

    pub fn generated_product_column_id(&self) -> &str {
        &self.generated_product_column_id
    }

    pub fn focal_predictor_id(&self) -> &str {
        &self.focal_predictor_id
    }

    pub fn moderator_id(&self) -> &str {
        &self.moderator_id
    }

    pub fn observation_count(&self) -> usize {
        self.observation_count
    }

    pub fn unstandardized_product_mean(&self) -> f64 {
        self.unstandardized_product_mean
    }

    pub fn unstandardized_product_sample_standard_deviation(&self) -> f64 {
        self.unstandardized_product_sample_standard_deviation
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsStructuralCoefficientV1 {
    relation_id: String,
    source_id: String,
    target_id: String,
    estimate: f64,
}

impl GeneralSemPlsStructuralCoefficientV1 {
    pub fn relation_id(&self) -> &str {
        &self.relation_id
    }

    pub fn source_id(&self) -> &str {
        &self.source_id
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn estimate(&self) -> f64 {
        self.estimate
    }
}

/// The standardized coefficient is the coefficient fitted to the standardized
/// product column. The raw-product coefficient rescales it back to the product
/// of the two sample-standardized operand scores and is therefore the quantity
/// that changes a focal slope per one standardized moderator unit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsInteractionCoefficientV1 {
    interaction_id: String,
    focal_relation_id: String,
    interaction_effect_relation_id: String,
    interaction_effect_parameter_id: String,
    focal_predictor_id: String,
    moderator_id: String,
    outcome_id: String,
    construction_method: InteractionMethodV4,
    hierarchy_policy: InteractionHierarchyPolicyV2,
    hierarchy_policy_version: String,
    standardized_product_estimate: f64,
    raw_product_estimate: f64,
}

impl GeneralSemPlsInteractionCoefficientV1 {
    pub fn interaction_id(&self) -> &str {
        &self.interaction_id
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

    pub fn focal_predictor_id(&self) -> &str {
        &self.focal_predictor_id
    }

    pub fn moderator_id(&self) -> &str {
        &self.moderator_id
    }

    pub fn outcome_id(&self) -> &str {
        &self.outcome_id
    }

    pub fn construction_method(&self) -> &InteractionMethodV4 {
        &self.construction_method
    }

    pub fn hierarchy_policy(&self) -> InteractionHierarchyPolicyV2 {
        self.hierarchy_policy
    }

    pub fn hierarchy_policy_version(&self) -> &str {
        &self.hierarchy_policy_version
    }

    pub fn standardized_product_estimate(&self) -> f64 {
        self.standardized_product_estimate
    }

    pub fn raw_product_estimate(&self) -> f64 {
        self.raw_product_estimate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsSimpleSlopeV1 {
    policy_version: String,
    interaction_id: String,
    focal_relation_id: String,
    moderator_id: String,
    moderator_value_standardized: f64,
    other_same_focal_moderators_held_at_standardized_zero: bool,
    estimate: f64,
}

impl GeneralSemPlsSimpleSlopeV1 {
    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }

    pub fn interaction_id(&self) -> &str {
        &self.interaction_id
    }

    pub fn focal_relation_id(&self) -> &str {
        &self.focal_relation_id
    }

    pub fn moderator_id(&self) -> &str {
        &self.moderator_id
    }

    pub fn moderator_value_standardized(&self) -> f64 {
        self.moderator_value_standardized
    }

    pub fn other_same_focal_moderators_held_at_standardized_zero(&self) -> bool {
        self.other_same_focal_moderators_held_at_standardized_zero
    }

    pub fn estimate(&self) -> f64 {
        self.estimate
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsMultipleInteractionPointResultV1 {
    method_version: String,
    observation_count: usize,
    product_scale_receipts: Vec<GeneralSemPlsProductScaleReceiptV1>,
    structural_coefficients: Vec<GeneralSemPlsStructuralCoefficientV1>,
    interaction_coefficients: Vec<GeneralSemPlsInteractionCoefficientV1>,
    simple_slopes: Vec<GeneralSemPlsSimpleSlopeV1>,
}

impl GeneralSemPlsMultipleInteractionPointResultV1 {
    pub fn method_version(&self) -> &str {
        &self.method_version
    }

    pub fn observation_count(&self) -> usize {
        self.observation_count
    }

    pub fn product_scale_receipts(&self) -> &[GeneralSemPlsProductScaleReceiptV1] {
        &self.product_scale_receipts
    }

    pub fn structural_coefficients(&self) -> &[GeneralSemPlsStructuralCoefficientV1] {
        &self.structural_coefficients
    }

    pub fn interaction_coefficients(&self) -> &[GeneralSemPlsInteractionCoefficientV1] {
        &self.interaction_coefficients
    }

    pub fn simple_slopes(&self) -> &[GeneralSemPlsSimpleSlopeV1] {
        &self.simple_slopes
    }

    /// Revalidates the complete point payload against the immutable compiled
    /// interaction plan. Runtime adapters call this before publishing any
    /// canonical result so drifted IDs, scaling receipts, or coefficients fail
    /// closed rather than being interpreted as a different estimand.
    pub fn ensure_valid_against_plan_v1(
        &self,
        plan: &CompiledPlsPlanV3,
    ) -> Result<(), GeneralSemPlsInteractionPointErrorV1> {
        self.ensure_self_consistent_v1()?;
        let expected_interaction_ids = plan
            .two_way_interactions()
            .iter()
            .map(|interaction| interaction.interaction_id())
            .collect::<Vec<_>>();
        let actual_interaction_ids = self
            .interaction_coefficients
            .iter()
            .map(|coefficient| coefficient.interaction_id.as_str())
            .collect::<Vec<_>>();
        if actual_interaction_ids != expected_interaction_ids {
            return Err(invalid_result(
                "interaction coefficient identities differ from the compiled PLS v3 plan",
            ));
        }
        let expected_relation_ids = plan
            .base_plan()
            .paths()
            .iter()
            .map(|path| path.relation_id())
            .collect::<Vec<_>>();
        let actual_relation_ids = self
            .structural_coefficients
            .iter()
            .map(|coefficient| coefficient.relation_id.as_str())
            .collect::<Vec<_>>();
        if actual_relation_ids != expected_relation_ids {
            return Err(invalid_result(
                "ordinary structural coefficient identities differ from the compiled stage-one plan",
            ));
        }
        for contract in plan.two_way_interactions() {
            let receipt = self
                .product_scale_receipts
                .iter()
                .find(|receipt| receipt.interaction_id == contract.interaction_id())
                .ok_or_else(|| {
                    invalid_result("a compiled interaction has no product-scale receipt")
                })?;
            let coefficient = self
                .interaction_coefficients
                .iter()
                .find(|coefficient| coefficient.interaction_id == contract.interaction_id())
                .ok_or_else(|| invalid_result("a compiled interaction has no coefficient"))?;
            if receipt.generated_product_column_id != contract.generated_product_column_id()
                || receipt.focal_predictor_id != contract.focal_predictor_id()
                || receipt.moderator_id != contract.moderator_id()
                || coefficient.focal_relation_id != contract.focal_relation_id()
                || coefficient.interaction_effect_relation_id
                    != contract.interaction_effect_relation_id()
                || coefficient.interaction_effect_parameter_id
                    != contract.interaction_effect_parameter_id()
                || coefficient.focal_predictor_id != contract.focal_predictor_id()
                || coefficient.moderator_id != contract.moderator_id()
                || coefficient.outcome_id != contract.outcome_id()
                || coefficient.construction_method != *contract.method()
                || coefficient.hierarchy_policy != contract.hierarchy_policy()
            {
                return Err(invalid_result(format!(
                    "interaction {} metadata differs from its compiled contract",
                    contract.interaction_id()
                )));
            }
        }
        Ok(())
    }

    /// Validates identities, ordering, frozen policies, and the exact algebraic
    /// relationship between the standardized-product coefficient and the
    /// scientifically interpreted gamma coefficient.
    pub fn ensure_self_consistent_v1(&self) -> Result<(), GeneralSemPlsInteractionPointErrorV1> {
        if self.method_version != GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1 {
            return Err(invalid_result(
                "the interaction method version is unsupported",
            ));
        }
        if self.observation_count < 3 {
            return Err(invalid_result(
                "the interaction result has fewer than three observations",
            ));
        }
        if self.product_scale_receipts.len() != self.interaction_coefficients.len()
            || self.simple_slopes.len() != self.interaction_coefficients.len() * 3
        {
            return Err(invalid_result(
                "interaction coefficients, scale receipts, and simple slopes have inconsistent cardinality",
            ));
        }
        if !strictly_sorted_unique(
            self.product_scale_receipts
                .iter()
                .map(|receipt| receipt.interaction_id.as_str()),
        ) || !strictly_sorted_unique(
            self.interaction_coefficients
                .iter()
                .map(|coefficient| coefficient.interaction_id.as_str()),
        ) || !strictly_sorted_unique(
            self.structural_coefficients
                .iter()
                .map(|coefficient| coefficient.relation_id.as_str()),
        ) {
            return Err(invalid_result(
                "interaction result identities are not in strict canonical order",
            ));
        }
        for receipt in &self.product_scale_receipts {
            if receipt.scale_version != GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1
                || receipt.observation_count != self.observation_count
                || !receipt.unstandardized_product_mean.is_finite()
                || !receipt
                    .unstandardized_product_sample_standard_deviation
                    .is_finite()
                || receipt.unstandardized_product_sample_standard_deviation <= f64::EPSILON
            {
                return Err(invalid_result(format!(
                    "interaction {} has an invalid product-scale receipt",
                    receipt.interaction_id
                )));
            }
        }
        for coefficient in &self.interaction_coefficients {
            let receipt = self
                .product_scale_receipts
                .iter()
                .find(|receipt| receipt.interaction_id == coefficient.interaction_id)
                .ok_or_else(|| invalid_result("an interaction coefficient has no scale receipt"))?;
            let expected_gamma = coefficient.standardized_product_estimate
                / receipt.unstandardized_product_sample_standard_deviation;
            if !coefficient.standardized_product_estimate.is_finite()
                || !coefficient.raw_product_estimate.is_finite()
                || expected_gamma.to_bits() != coefficient.raw_product_estimate.to_bits()
                || coefficient.construction_method != InteractionMethodV4::TwoStage
                || coefficient.hierarchy_policy != InteractionHierarchyPolicyV2::Strong
                || coefficient.hierarchy_policy_version
                    != GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1
            {
                return Err(invalid_result(format!(
                    "interaction {} has inconsistent coefficient scaling or policy provenance",
                    coefficient.interaction_id
                )));
            }
        }
        for (interaction_index, coefficient) in self.interaction_coefficients.iter().enumerate() {
            let slopes = &self.simple_slopes[interaction_index * 3..interaction_index * 3 + 3];
            for (slope, expected_probe) in slopes.iter().zip([-1.0_f64, 0.0, 1.0]) {
                let expected = self.conditional_focal_slope_v1(
                    &coefficient.focal_relation_id,
                    &BTreeMap::from([(coefficient.moderator_id.clone(), expected_probe)]),
                )?;
                if slope.policy_version != GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1
                    || slope.interaction_id != coefficient.interaction_id
                    || slope.focal_relation_id != coefficient.focal_relation_id
                    || slope.moderator_id != coefficient.moderator_id
                    || slope.moderator_value_standardized.to_bits() != expected_probe.to_bits()
                    || !slope.other_same_focal_moderators_held_at_standardized_zero
                    || slope.estimate.to_bits() != expected.to_bits()
                {
                    return Err(invalid_result(format!(
                        "interaction {} has an inconsistent frozen simple-slope ledger",
                        coefficient.interaction_id
                    )));
                }
            }
        }
        Ok(())
    }

    /// Conditional derivative of a standardized outcome with respect to its
    /// standardized focal score. Missing moderator values are explicitly held
    /// at standardized zero.
    pub fn conditional_focal_slope_v1(
        &self,
        focal_relation_id: &str,
        moderator_values_standardized: &BTreeMap<String, f64>,
    ) -> Result<f64, GeneralSemPlsInteractionPointErrorV1> {
        if let Some((moderator_id, _)) = moderator_values_standardized
            .iter()
            .find(|(_, value)| !value.is_finite())
        {
            return Err(
                GeneralSemPlsInteractionPointErrorV1::NonFiniteModeratorProbe {
                    moderator_id: moderator_id.clone(),
                },
            );
        }
        let mut slope = self
            .structural_coefficients
            .iter()
            .find(|coefficient| coefficient.relation_id == focal_relation_id)
            .map(|coefficient| coefficient.estimate)
            .ok_or_else(
                || GeneralSemPlsInteractionPointErrorV1::UnknownFocalRelation {
                    focal_relation_id: focal_relation_id.to_string(),
                },
            )?;
        for coefficient in self
            .interaction_coefficients
            .iter()
            .filter(|coefficient| coefficient.focal_relation_id == focal_relation_id)
        {
            slope += coefficient.raw_product_estimate
                * moderator_values_standardized
                    .get(&coefficient.moderator_id)
                    .copied()
                    .unwrap_or(0.0);
        }
        Ok(slope)
    }

    /// Focal contribution to the stage-two standardized linear predictor. This
    /// small API gives plot builders and finite-difference checks one source of
    /// truth for conditional slopes.
    pub fn focal_linear_predictor_component_v1(
        &self,
        focal_relation_id: &str,
        focal_value_standardized: f64,
        moderator_values_standardized: &BTreeMap<String, f64>,
    ) -> Result<f64, GeneralSemPlsInteractionPointErrorV1> {
        if !focal_value_standardized.is_finite() {
            return Err(GeneralSemPlsInteractionPointErrorV1::NonFiniteFocalProbe);
        }
        Ok(focal_value_standardized
            * self.conditional_focal_slope_v1(focal_relation_id, moderator_values_standardized)?)
    }

    /// Complete stage-two standardized linear predictor for one outcome. Any
    /// omitted ordinary predictor or interaction operand is held at zero. The
    /// product mean is retained, so plots do not silently drop the intercept
    /// introduced when the raw product of standardized operands is centered.
    pub fn standardized_outcome_linear_predictor_v1(
        &self,
        outcome_id: &str,
        predictor_values_standardized: &BTreeMap<String, f64>,
    ) -> Result<f64, GeneralSemPlsInteractionPointErrorV1> {
        if let Some((variable_id, _)) = predictor_values_standardized
            .iter()
            .find(|(_, value)| !value.is_finite())
        {
            return Err(
                GeneralSemPlsInteractionPointErrorV1::NonFinitePredictorProbe {
                    variable_id: variable_id.clone(),
                },
            );
        }
        if !self
            .structural_coefficients
            .iter()
            .any(|coefficient| coefficient.target_id == outcome_id)
            && !self
                .interaction_coefficients
                .iter()
                .any(|coefficient| coefficient.outcome_id == outcome_id)
        {
            return Err(GeneralSemPlsInteractionPointErrorV1::UnknownOutcome {
                outcome_id: outcome_id.to_string(),
            });
        }
        let mut predicted = self
            .structural_coefficients
            .iter()
            .filter(|coefficient| coefficient.target_id == outcome_id)
            .map(|coefficient| {
                coefficient.estimate
                    * predictor_values_standardized
                        .get(&coefficient.source_id)
                        .copied()
                        .unwrap_or(0.0)
            })
            .sum::<f64>();
        for coefficient in self
            .interaction_coefficients
            .iter()
            .filter(|coefficient| coefficient.outcome_id == outcome_id)
        {
            let receipt = self
                .product_scale_receipts
                .iter()
                .find(|receipt| receipt.interaction_id == coefficient.interaction_id)
                .ok_or_else(|| invalid_result("an interaction coefficient has no scale receipt"))?;
            let focal = predictor_values_standardized
                .get(&coefficient.focal_predictor_id)
                .copied()
                .unwrap_or(0.0);
            let moderator = predictor_values_standardized
                .get(&coefficient.moderator_id)
                .copied()
                .unwrap_or(0.0);
            predicted += coefficient.standardized_product_estimate
                * (focal * moderator - receipt.unstandardized_product_mean)
                / receipt.unstandardized_product_sample_standard_deviation;
        }
        Ok(predicted)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemPlsInteractionPointErrorV1 {
    #[error("interaction point estimation was cancelled")]
    Cancelled,
    #[error("compiled PLS v3 plan has no two-way interactions")]
    NoInteractions,
    #[error("stage-one construct score is missing for {variable_id}")]
    MissingStageOneScore { variable_id: String },
    #[error("stage-one score {variable_id} has {actual} observations; expected {expected}")]
    ScoreLengthMismatch {
        variable_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("stage-one score {variable_id} contains a non-finite value at row {row_index}")]
    NonFiniteScore {
        variable_id: String,
        row_index: usize,
    },
    #[error("stage-one score {variable_id} has zero or non-finite sample variance")]
    ConstantStageOneScore { variable_id: String },
    #[error("at least three observations are required; received {observation_count}")]
    InsufficientObservations { observation_count: usize },
    #[error("interaction {interaction_id} product has zero or non-finite sample variance")]
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
    #[error(
        "interaction {interaction_id} refers to an ordinary focal relation not present in stage one"
    )]
    MissingFocalRelation { interaction_id: String },
    #[error("unknown focal relation {focal_relation_id}")]
    UnknownFocalRelation { focal_relation_id: String },
    #[error("moderator probe for {moderator_id} is non-finite")]
    NonFiniteModeratorProbe { moderator_id: String },
    #[error("focal probe is non-finite")]
    NonFiniteFocalProbe,
    #[error("predictor probe for {variable_id} is non-finite")]
    NonFinitePredictorProbe { variable_id: String },
    #[error("outcome {outcome_id} is absent from the joint stage-two result")]
    UnknownOutcome { outcome_id: String },
    #[error("interaction point result contract is invalid: {0}")]
    InvalidResultContract(String),
}

#[derive(Clone)]
enum PredictorKind<'a> {
    Ordinary {
        source_id: &'a str,
    },
    Interaction {
        contract: &'a CompiledPlsTwoWayInteractionV3,
        product_sample_standard_deviation: f64,
    },
}

#[derive(Clone)]
struct PredictorColumn<'a> {
    relation_id: &'a str,
    values: &'a [f64],
    kind: PredictorKind<'a>,
}

/// Fits all ordinary and two-way interaction predictors for each outcome in
/// one deterministic stage-two equation. The caller supplies the shared
/// stage-one construct scores; no interaction is scored or refitted in
/// isolation.
pub fn estimate_general_sem_pls_multiple_two_way_interactions_v1(
    plan: &CompiledPlsPlanV3,
    stage_one_scores: &BTreeMap<String, Vec<f64>>,
) -> Result<GeneralSemPlsMultipleInteractionPointResultV1, GeneralSemPlsInteractionPointErrorV1> {
    estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
        plan,
        stage_one_scores,
        || true,
    )
}

/// Cancellation-aware form used by full-model resampling and native jobs.
/// The continuation callback is checked throughout score validation, product
/// construction, each joint outcome equation, QR factorization, and frozen
/// simple-slope construction. The legacy entry point delegates here with an
/// always-continue callback and therefore retains its exact point semantics.
pub fn estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
    plan: &CompiledPlsPlanV3,
    stage_one_scores: &BTreeMap<String, Vec<f64>>,
    should_continue: impl Fn() -> bool,
) -> Result<GeneralSemPlsMultipleInteractionPointResultV1, GeneralSemPlsInteractionPointErrorV1> {
    if !should_continue() {
        return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
    }
    if plan.two_way_interactions().is_empty() {
        return Err(GeneralSemPlsInteractionPointErrorV1::NoInteractions);
    }

    let required_score_ids = plan
        .base_plan()
        .blocks()
        .iter()
        .map(|block| block.construct_id().to_string())
        .collect::<BTreeSet<_>>();
    let first_required_score_id = required_score_ids
        .iter()
        .next()
        .expect("a compiled interaction PLS plan has at least one construct block");
    let observation_count = stage_one_scores
        .get(first_required_score_id)
        .ok_or_else(
            || GeneralSemPlsInteractionPointErrorV1::MissingStageOneScore {
                variable_id: first_required_score_id.clone(),
            },
        )?
        .len();
    if observation_count < 3 {
        return Err(
            GeneralSemPlsInteractionPointErrorV1::InsufficientObservations { observation_count },
        );
    }

    let mut standardized_scores = BTreeMap::<String, Vec<f64>>::new();
    for variable_id in required_score_ids {
        if !should_continue() {
            return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
        }
        let values = stage_one_scores.get(&variable_id).ok_or_else(|| {
            GeneralSemPlsInteractionPointErrorV1::MissingStageOneScore {
                variable_id: variable_id.clone(),
            }
        })?;
        if values.len() != observation_count {
            return Err(GeneralSemPlsInteractionPointErrorV1::ScoreLengthMismatch {
                variable_id,
                expected: observation_count,
                actual: values.len(),
            });
        }
        standardized_scores.insert(
            variable_id.clone(),
            sample_standardize(values, &variable_id)?,
        );
    }

    let mut product_columns = BTreeMap::<String, Vec<f64>>::new();
    let mut product_scale_receipts = Vec::new();
    for interaction in plan.two_way_interactions() {
        if !should_continue() {
            return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
        }
        let focal = standardized_scores
            .get(interaction.focal_predictor_id())
            .ok_or_else(
                || GeneralSemPlsInteractionPointErrorV1::MissingStageOneScore {
                    variable_id: interaction.focal_predictor_id().to_string(),
                },
            )?;
        let moderator = standardized_scores
            .get(interaction.moderator_id())
            .ok_or_else(
                || GeneralSemPlsInteractionPointErrorV1::MissingStageOneScore {
                    variable_id: interaction.moderator_id().to_string(),
                },
            )?;
        let product = focal
            .iter()
            .zip(moderator)
            .map(|(left, right)| left * right)
            .collect::<Vec<_>>();
        let (standardized_product, product_mean, product_sd) =
            sample_standardize_with_receipt(&product).ok_or_else(|| {
                GeneralSemPlsInteractionPointErrorV1::ConstantProduct {
                    interaction_id: interaction.interaction_id().to_string(),
                }
            })?;
        product_columns.insert(
            interaction.interaction_id().to_string(),
            standardized_product,
        );
        product_scale_receipts.push(GeneralSemPlsProductScaleReceiptV1 {
            scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.to_string(),
            interaction_id: interaction.interaction_id().to_string(),
            generated_product_column_id: interaction.generated_product_column_id().to_string(),
            focal_predictor_id: interaction.focal_predictor_id().to_string(),
            moderator_id: interaction.moderator_id().to_string(),
            observation_count,
            unstandardized_product_mean: product_mean,
            unstandardized_product_sample_standard_deviation: product_sd,
        });
    }

    let outcomes = plan
        .base_plan()
        .paths()
        .iter()
        .map(|path| path.target().to_string())
        .chain(
            plan.two_way_interactions()
                .iter()
                .map(|interaction| interaction.outcome_id().to_string()),
        )
        .collect::<BTreeSet<_>>();
    let mut structural_coefficients = Vec::new();
    let mut interaction_coefficients = Vec::new();

    for outcome_id in outcomes {
        if !should_continue() {
            return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
        }
        let outcome = standardized_scores.get(&outcome_id).ok_or_else(|| {
            GeneralSemPlsInteractionPointErrorV1::MissingStageOneScore {
                variable_id: outcome_id.clone(),
            }
        })?;
        let mut predictors = Vec::<PredictorColumn<'_>>::new();
        for path in plan
            .base_plan()
            .paths()
            .iter()
            .filter(|path| path.target() == outcome_id)
        {
            let values = standardized_scores.get(path.source()).ok_or_else(|| {
                GeneralSemPlsInteractionPointErrorV1::MissingStageOneScore {
                    variable_id: path.source().to_string(),
                }
            })?;
            predictors.push(PredictorColumn {
                relation_id: path.relation_id(),
                values,
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
            if !plan
                .base_plan()
                .paths()
                .iter()
                .any(|path| path.relation_id() == interaction.focal_relation_id())
            {
                return Err(GeneralSemPlsInteractionPointErrorV1::MissingFocalRelation {
                    interaction_id: interaction.interaction_id().to_string(),
                });
            }
            let product_sd = product_scale_receipts
                .iter()
                .find(|receipt| receipt.interaction_id == interaction.interaction_id())
                .expect("each interaction has one product receipt")
                .unstandardized_product_sample_standard_deviation;
            predictors.push(PredictorColumn {
                relation_id: interaction.interaction_effect_relation_id(),
                values: &product_columns[interaction.interaction_id()],
                kind: PredictorKind::Interaction {
                    contract: interaction,
                    product_sample_standard_deviation: product_sd,
                },
            });
        }
        predictors.sort_by(|left, right| left.relation_id.cmp(right.relation_id));
        if predictors.len() >= observation_count {
            return Err(
                GeneralSemPlsInteractionPointErrorV1::InsufficientEquationObservations {
                    outcome_id,
                    observation_count,
                    predictor_count: predictors.len(),
                },
            );
        }
        let estimates =
            solve_least_squares_qr(&predictors, outcome, &outcome_id, &should_continue)?;
        for (predictor, estimate) in predictors.into_iter().zip(estimates) {
            match predictor.kind {
                PredictorKind::Ordinary { source_id } => {
                    structural_coefficients.push(GeneralSemPlsStructuralCoefficientV1 {
                        relation_id: predictor.relation_id.to_string(),
                        source_id: source_id.to_string(),
                        target_id: outcome_id.clone(),
                        estimate,
                    });
                }
                PredictorKind::Interaction {
                    contract,
                    product_sample_standard_deviation,
                } => {
                    interaction_coefficients.push(GeneralSemPlsInteractionCoefficientV1 {
                        interaction_id: contract.interaction_id().to_string(),
                        focal_relation_id: contract.focal_relation_id().to_string(),
                        interaction_effect_relation_id: contract
                            .interaction_effect_relation_id()
                            .to_string(),
                        interaction_effect_parameter_id: contract
                            .interaction_effect_parameter_id()
                            .to_string(),
                        focal_predictor_id: contract.focal_predictor_id().to_string(),
                        moderator_id: contract.moderator_id().to_string(),
                        outcome_id: outcome_id.clone(),
                        construction_method: contract.method().clone(),
                        hierarchy_policy: contract.hierarchy_policy(),
                        hierarchy_policy_version:
                            GENERAL_SEM_PLS_STRONG_HIERARCHY_POLICY_VERSION_V1.to_string(),
                        standardized_product_estimate: estimate,
                        raw_product_estimate: estimate / product_sample_standard_deviation,
                    });
                }
            }
        }
    }
    structural_coefficients.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    interaction_coefficients.sort_by(|left, right| left.interaction_id.cmp(&right.interaction_id));
    product_scale_receipts.sort_by(|left, right| left.interaction_id.cmp(&right.interaction_id));

    let provisional = GeneralSemPlsMultipleInteractionPointResultV1 {
        method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.to_string(),
        observation_count,
        product_scale_receipts,
        structural_coefficients,
        interaction_coefficients,
        simple_slopes: Vec::new(),
    };
    let mut simple_slopes = Vec::new();
    for interaction in &provisional.interaction_coefficients {
        if !should_continue() {
            return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
        }
        for moderator_value_standardized in [-1.0, 0.0, 1.0] {
            let moderator_values = BTreeMap::from([(
                interaction.moderator_id.clone(),
                moderator_value_standardized,
            )]);
            simple_slopes.push(GeneralSemPlsSimpleSlopeV1 {
                policy_version: GENERAL_SEM_PLS_SIMPLE_SLOPE_POLICY_VERSION_V1.to_string(),
                interaction_id: interaction.interaction_id.clone(),
                focal_relation_id: interaction.focal_relation_id.clone(),
                moderator_id: interaction.moderator_id.clone(),
                moderator_value_standardized,
                other_same_focal_moderators_held_at_standardized_zero: true,
                estimate: provisional.conditional_focal_slope_v1(
                    &interaction.focal_relation_id,
                    &moderator_values,
                )?,
            });
        }
    }
    let result = GeneralSemPlsMultipleInteractionPointResultV1 {
        simple_slopes,
        ..provisional
    };
    result.ensure_valid_against_plan_v1(plan)?;
    Ok(result)
}

fn invalid_result(message: impl Into<String>) -> GeneralSemPlsInteractionPointErrorV1 {
    GeneralSemPlsInteractionPointErrorV1::InvalidResultContract(message.into())
}

fn strictly_sorted_unique<'a>(values: impl IntoIterator<Item = &'a str>) -> bool {
    let values = values.into_iter().collect::<Vec<_>>();
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn sample_standardize(
    values: &[f64],
    variable_id: &str,
) -> Result<Vec<f64>, GeneralSemPlsInteractionPointErrorV1> {
    if let Some(row_index) = values.iter().position(|value| !value.is_finite()) {
        return Err(GeneralSemPlsInteractionPointErrorV1::NonFiniteScore {
            variable_id: variable_id.to_string(),
            row_index,
        });
    }
    sample_standardize_with_receipt(values)
        .map(|(values, _, _)| values)
        .ok_or_else(
            || GeneralSemPlsInteractionPointErrorV1::ConstantStageOneScore {
                variable_id: variable_id.to_string(),
            },
        )
}

fn sample_standardize_with_receipt(values: &[f64]) -> Option<(Vec<f64>, f64, f64)> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let centered_sum_of_squares = values
        .iter()
        .map(|value| {
            let centered = value - mean;
            centered * centered
        })
        .sum::<f64>();
    let sample_standard_deviation = (centered_sum_of_squares / (values.len() - 1) as f64).sqrt();
    if !sample_standard_deviation.is_finite() || sample_standard_deviation <= f64::EPSILON {
        return None;
    }
    Some((
        values
            .iter()
            .map(|value| (value - mean) / sample_standard_deviation)
            .collect(),
        mean,
        sample_standard_deviation,
    ))
}

fn solve_least_squares_qr(
    predictors: &[PredictorColumn<'_>],
    outcome: &[f64],
    outcome_id: &str,
    should_continue: &impl Fn() -> bool,
) -> Result<Vec<f64>, GeneralSemPlsInteractionPointErrorV1> {
    let column_count = predictors.len();
    if column_count == 0 {
        return Ok(Vec::new());
    }
    let mut q_columns = Vec::<Vec<f64>>::with_capacity(column_count);
    let mut r = vec![vec![0.0; column_count]; column_count];
    let rank_tolerance = (outcome.len() as f64).sqrt() * 1.0e-11;
    for (column_index, predictor) in predictors.iter().enumerate() {
        if !should_continue() {
            return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
        }
        let mut residual = predictor.values.to_vec();
        for previous in 0..column_index {
            let projection = dot(&q_columns[previous], &residual);
            r[previous][column_index] += projection;
            subtract_scaled(&mut residual, &q_columns[previous], projection);
        }
        // A second pass materially improves modified Gram-Schmidt when two
        // interaction products are close to collinear.
        for previous in 0..column_index {
            let correction = dot(&q_columns[previous], &residual);
            r[previous][column_index] += correction;
            subtract_scaled(&mut residual, &q_columns[previous], correction);
        }
        let norm = dot(&residual, &residual).sqrt();
        if !norm.is_finite() || norm <= rank_tolerance {
            return Err(GeneralSemPlsInteractionPointErrorV1::RankDeficient {
                outcome_id: outcome_id.to_string(),
                predictor_id: predictor.relation_id.to_string(),
            });
        }
        r[column_index][column_index] = norm;
        for value in &mut residual {
            *value /= norm;
        }
        q_columns.push(residual);
    }

    let mut q_transpose_y = q_columns
        .iter()
        .map(|column| dot(column, outcome))
        .collect::<Vec<_>>();
    for row in (0..column_count).rev() {
        if !should_continue() {
            return Err(GeneralSemPlsInteractionPointErrorV1::Cancelled);
        }
        for column in row + 1..column_count {
            q_transpose_y[row] -= r[row][column] * q_transpose_y[column];
        }
        q_transpose_y[row] /= r[row][row];
    }
    Ok(q_transpose_y)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn subtract_scaled(target: &mut [f64], source: &[f64], scale: f64) {
    for (target, source) in target.iter_mut().zip(source) {
        *target -= scale * source;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use qpls_core::{
        Construct, GeneralSemConfigV1, InteractionHierarchyPolicyV2, InteractionMethodV4,
        LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, SemDerivedTermV4, SemModelV4,
        SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath,
        StructuralRelationRoleV4, compile_pls_plan_v3, convert_legacy_basic_model_v4,
    };
    use uuid::Uuid;

    fn moderation_model() -> SemModelV4 {
        let constructs = ["x", "w", "z", "y"]
            .into_iter()
            .map(|id| Construct {
                id: id.into(),
                name: id.to_uppercase(),
                short_name: id.to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}1"), format!("{id}2")],
            })
            .collect();
        let paths = [("x", "y"), ("w", "y"), ("z", "y")]
            .into_iter()
            .map(|(source, target)| StructuralPath {
                source: source.into(),
                target: target.into(),
            })
            .collect();
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(0x4d4f_4452_4552_4154_494f_4e01),
                name: "Multiple moderation".into(),
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

    fn structural_relation_id(model: &SemModelV4, source: &str, target: &str) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source: actual_source,
                    target: actual_target,
                    ..
                } if actual_source == source && actual_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    fn add_interaction(
        model: &mut SemModelV4,
        interaction_id: &str,
        focal_predictor_id: &str,
        moderator_id: &str,
    ) {
        let focal_relation = structural_relation_id(model, focal_predictor_id, "construct:y");
        let output = format!("derived:{interaction_id}");
        let relation_id = format!("relation:{interaction_id}:effect");
        let parameter_id = format!("parameter:{interaction_id}:effect");
        model.variables.push(SemVariableV4::Derived {
            id: output.clone(),
            label: interaction_id.into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: relation_id,
            source: output.clone(),
            target: "construct:y".into(),
            parameter: parameter_id.clone(),
            role: StructuralRelationRoleV4::Structural,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: parameter_id,
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
    }

    fn standardized(values: &[f64]) -> Vec<f64> {
        sample_standardize_with_receipt(values).unwrap().0
    }

    fn sample_sd(values: &[f64]) -> f64 {
        sample_standardize_with_receipt(values).unwrap().2
    }

    fn deterministic_scores() -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let x = (0..61).map(|row| row as f64 - 30.0).collect::<Vec<_>>();
        let w = (0..61)
            .map(|row| (row as f64 * 0.71).sin() + (row as f64 * 0.13).cos() * 0.2)
            .collect::<Vec<_>>();
        let z = (0..61)
            .map(|row| (row as f64 * 0.37).cos() - (row as f64 * 0.19).sin() * 0.3)
            .collect::<Vec<_>>();
        (x, w, z)
    }

    #[test]
    fn jointly_estimates_two_interactions_on_the_same_focal_path_and_rescales_slopes() {
        let mut model = moderation_model();
        add_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        add_interaction(
            &mut model,
            "interaction:x_by_z",
            "construct:x",
            "construct:z",
        );
        let focal_relation_id = structural_relation_id(&model, "construct:x", "construct:y");
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();

        let (x, w, z) = deterministic_scores();
        let xs = standardized(&x);
        let ws = standardized(&w);
        let zs = standardized(&z);
        let y = xs
            .iter()
            .zip(&ws)
            .zip(&zs)
            .map(|((x, w), z)| 0.25 * x + 0.20 * w - 0.15 * z + 0.70 * x * w - 0.40 * x * z)
            .collect::<Vec<_>>();
        let y_sd = sample_sd(&y);
        let scores = BTreeMap::from([
            ("construct:x".into(), x),
            ("construct:w".into(), w),
            ("construct:z".into(), z),
            ("construct:y".into(), y),
        ]);
        let result =
            estimate_general_sem_pls_multiple_two_way_interactions_v1(&plan, &scores).unwrap();

        assert_eq!(result.interaction_coefficients().len(), 2);
        assert_eq!(result.product_scale_receipts().len(), 2);
        assert_eq!(result.simple_slopes().len(), 6);
        let x_by_w = result
            .interaction_coefficients()
            .iter()
            .find(|coefficient| coefficient.interaction_id() == "interaction:x_by_w")
            .unwrap();
        let x_by_z = result
            .interaction_coefficients()
            .iter()
            .find(|coefficient| coefficient.interaction_id() == "interaction:x_by_z")
            .unwrap();
        assert_abs_diff_eq!(x_by_w.raw_product_estimate(), 0.70 / y_sd, epsilon = 1e-10);
        assert_abs_diff_eq!(x_by_z.raw_product_estimate(), -0.40 / y_sd, epsilon = 1e-10);

        let moderator_values =
            BTreeMap::from([("construct:w".into(), 1.0), ("construct:z".into(), -0.5)]);
        let slope = result
            .conditional_focal_slope_v1(&focal_relation_id, &moderator_values)
            .unwrap();
        assert_abs_diff_eq!(slope, (0.25 + 0.70 + 0.20) / y_sd, epsilon = 1e-10);

        let epsilon = 1e-6;
        let upper = result
            .focal_linear_predictor_component_v1(
                &focal_relation_id,
                0.4 + epsilon,
                &moderator_values,
            )
            .unwrap();
        let lower = result
            .focal_linear_predictor_component_v1(
                &focal_relation_id,
                0.4 - epsilon,
                &moderator_values,
            )
            .unwrap();
        assert_abs_diff_eq!((upper - lower) / (2.0 * epsilon), slope, epsilon = 1e-9);

        let product_sd = result
            .product_scale_receipts()
            .iter()
            .find(|receipt| receipt.interaction_id() == "interaction:x_by_w")
            .unwrap()
            .unstandardized_product_sample_standard_deviation();
        assert_abs_diff_eq!(
            x_by_w.standardized_product_estimate(),
            x_by_w.raw_product_estimate() * product_sd,
            epsilon = 1e-12
        );
    }

    #[test]
    fn jointly_estimates_interactions_attached_to_different_focal_paths() {
        let mut model = moderation_model();
        add_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        add_interaction(
            &mut model,
            "interaction:w_by_z",
            "construct:w",
            "construct:z",
        );
        let x_relation = structural_relation_id(&model, "construct:x", "construct:y");
        let w_relation = structural_relation_id(&model, "construct:w", "construct:y");
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();

        let (x, w, z) = deterministic_scores();
        let xs = standardized(&x);
        let ws = standardized(&w);
        let zs = standardized(&z);
        let y = xs
            .iter()
            .zip(&ws)
            .zip(&zs)
            .map(|((x, w), z)| 0.10 * x + 0.20 * w + 0.30 * z + 0.50 * x * w - 0.35 * w * z)
            .collect::<Vec<_>>();
        let y_sd = sample_sd(&y);
        let result = estimate_general_sem_pls_multiple_two_way_interactions_v1(
            &plan,
            &BTreeMap::from([
                ("construct:x".into(), x),
                ("construct:w".into(), w),
                ("construct:z".into(), z),
                ("construct:y".into(), y),
            ]),
        )
        .unwrap();

        assert_abs_diff_eq!(
            result
                .conditional_focal_slope_v1(
                    &x_relation,
                    &BTreeMap::from([("construct:w".into(), 1.0)])
                )
                .unwrap(),
            0.60 / y_sd,
            epsilon = 1e-10
        );
        assert_abs_diff_eq!(
            result
                .conditional_focal_slope_v1(
                    &w_relation,
                    &BTreeMap::from([("construct:z".into(), -1.0)])
                )
                .unwrap(),
            0.55 / y_sd,
            epsilon = 1e-10
        );
    }

    #[test]
    fn rejects_a_constant_two_stage_product_before_joint_regression() {
        let mut model = moderation_model();
        add_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let alternating = vec![-1.0, 1.0, -1.0, 1.0, -1.0, 1.0];
        let scores = BTreeMap::from([
            ("construct:x".into(), alternating.clone()),
            ("construct:w".into(), alternating),
            ("construct:z".into(), vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            ("construct:y".into(), vec![0.0, 1.0, 4.0, 2.0, 5.0, 3.0]),
        ]);
        assert_eq!(
            estimate_general_sem_pls_multiple_two_way_interactions_v1(&plan, &scores),
            Err(GeneralSemPlsInteractionPointErrorV1::ConstantProduct {
                interaction_id: "interaction:x_by_w".into()
            })
        );
    }

    #[test]
    fn rejects_mismatched_rows_in_the_shared_stage_one_score_set() {
        let mut model = moderation_model();
        add_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let scores = BTreeMap::from([
            ("construct:w".into(), vec![-2.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            ("construct:x".into(), vec![0.0, 1.0, 4.0, 2.0, 5.0, 3.0]),
            ("construct:y".into(), vec![1.0, 3.0, 2.0, 6.0, 4.0, 5.0]),
            ("construct:z".into(), vec![0.0, 1.0, 2.0, 3.0, 4.0]),
        ]);
        assert_eq!(
            estimate_general_sem_pls_multiple_two_way_interactions_v1(&plan, &scores),
            Err(GeneralSemPlsInteractionPointErrorV1::ScoreLengthMismatch {
                variable_id: "construct:z".into(),
                expected: 6,
                actual: 5,
            })
        );
    }

    #[test]
    fn cancellation_aware_entry_point_preserves_point_results_and_stops_terminally() {
        let mut model = moderation_model();
        add_interaction(
            &mut model,
            "interaction:x_by_w",
            "construct:x",
            "construct:w",
        );
        add_interaction(
            &mut model,
            "interaction:x_by_z",
            "construct:x",
            "construct:z",
        );
        let plan = compile_pls_plan_v3(&model, &GeneralSemConfigV1::default()).unwrap();
        let (x, w, z) = deterministic_scores();
        let xs = standardized(&x);
        let ws = standardized(&w);
        let zs = standardized(&z);
        let y = xs
            .iter()
            .zip(&ws)
            .zip(&zs)
            .map(|((x, w), z)| 0.25 * x + 0.2 * w - 0.1 * z + 0.6 * x * w - 0.3 * x * z)
            .collect::<Vec<_>>();
        let scores = BTreeMap::from([
            ("construct:x".into(), x),
            ("construct:w".into(), w),
            ("construct:z".into(), z),
            ("construct:y".into(), y),
        ]);

        let legacy =
            estimate_general_sem_pls_multiple_two_way_interactions_v1(&plan, &scores).unwrap();
        let controlled = estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
            &plan,
            &scores,
            || true,
        )
        .unwrap();
        assert_eq!(controlled, legacy);
        assert_eq!(
            estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
                &plan,
                &scores,
                || false,
            ),
            Err(GeneralSemPlsInteractionPointErrorV1::Cancelled)
        );
    }
}
