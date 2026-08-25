//! Exact joint interaction equations for MultiMod conditional-process V2.
//!
//! The historical General SEM interaction executor is deliberately unchanged.
//! This additive kernel accepts an already-refitted set of construct scores and
//! an explicit, compiler-bound interaction inventory.  It is used in two
//! places where the historical `CompiledPlsPlanV3` cannot carry the complete
//! runtime contract: positive-weight/count-space fits and the final stage of a
//! multiple-HOC fit.

use qpls_core::{CompiledPlsPlanV2, sha256_serialized};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ConditionalEdgeFunctionV2, ConditionalLinearCoefficientV2,
    GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1,
};

pub const MULTIMOD_CONDITIONAL_INTERACTION_POINT_METHOD_VERSION_V2: &str =
    "multimod_conditional_joint_two_way_point_v2";
pub const MULTIMOD_CONDITIONAL_WEIGHT_MOMENT_CONTRACT_V2: &str =
    "multimod_conditional_weight_moments_v2";
pub const MULTIMOD_CONDITIONAL_FREQUENCY_EQUIVALENCE_CONTRACT_V2: &str =
    "positive_integer_count_space_equals_expanded_rows_v2";

/// Complete identity required to add one TwoStage + Strong scientific product
/// to a final ordinary structural equation.  The MultiMod compiler derives
/// this from the immutable SemModelV4 rather than guessing from score names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultimodConditionalTwoWayInteractionV2 {
    pub interaction_id: String,
    pub output_id: String,
    pub focal_relation_id: String,
    pub interaction_effect_relation_id: String,
    pub interaction_effect_parameter_id: String,
    pub focal_predictor_id: String,
    pub moderator_id: String,
    pub outcome_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultimodConditionalMomentSemanticsV2 {
    UnweightedSample,
    PositiveCaseReliability,
    PositiveIntegerFrequencyCountSpace,
}

/// Borrowed row-mass authority.  Frequency counts remain integer-valued all
/// the way through the moment and normal-equation accumulators; no row is
/// physically repeated.
#[derive(Debug, Clone, Copy)]
pub enum MultimodConditionalRowMassV2<'a> {
    Unweighted,
    PositiveCase(&'a [f64]),
    PositiveIntegerFrequency(&'a [u64]),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodConditionalProductScaleReceiptV2 {
    pub scale_version: String,
    pub interaction_id: String,
    pub semantics: MultimodConditionalMomentSemanticsV2,
    pub compact_row_count: usize,
    pub represented_observation_count: u64,
    pub weighted_product_mean: f64,
    pub weighted_product_standard_deviation: f64,
    pub variance_denominator: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodConditionalInteractionPointReceiptV2 {
    pub method_version: String,
    pub moment_contract: String,
    pub base_plan_sha256: String,
    pub interaction_inventory_sha256: String,
    pub semantics: MultimodConditionalMomentSemanticsV2,
    pub compact_row_count: usize,
    pub represented_observation_count: u64,
    pub row_mass_sum: f64,
    pub variance_denominator: f64,
    pub exact_frequency_row_expansion_equivalence: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodConditionalInteractionPointV2 {
    pub edges: Vec<ConditionalEdgeFunctionV2>,
    pub product_scale_receipts: Vec<MultimodConditionalProductScaleReceiptV2>,
    pub receipt: MultimodConditionalInteractionPointReceiptV2,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MultimodConditionalInteractionPointErrorV2 {
    #[error("conditional interaction point estimation was cancelled")]
    Cancelled,
    #[error("at least one explicit two-way interaction is required")]
    NoInteractions,
    #[error("interaction inventory is not in strict stable-id order")]
    InteractionOrder,
    #[error("interaction identity contract is invalid: {0}")]
    InteractionContract(String),
    #[error("stage score is missing for {variable_id}")]
    MissingScore { variable_id: String },
    #[error("stage score {variable_id} has {actual} rows; expected {expected}")]
    ScoreLength {
        variable_id: String,
        expected: usize,
        actual: usize,
    },
    #[error("stage score {variable_id} contains a nonfinite value at row {row}")]
    NonfiniteScore { variable_id: String, row: usize },
    #[error("stage score {variable_id} has zero or nonfinite variance")]
    ConstantScore { variable_id: String },
    #[error("row-mass vector has {actual} rows; expected {expected}")]
    RowMassLength { expected: usize, actual: usize },
    #[error("positive case weight is invalid at row {row}")]
    InvalidCaseWeight { row: usize },
    #[error("positive integer frequency is invalid at row {row}")]
    InvalidFrequency { row: usize },
    #[error("positive integer frequency total exceeds 2^53-1")]
    FrequencyTotal,
    #[error("row masses have no positive variance degrees of freedom")]
    RowMassDegreesOfFreedom,
    #[error("interaction product {interaction_id} has zero or nonfinite variance")]
    ConstantProduct { interaction_id: String },
    #[error(
        "outcome {outcome_id} has {predictor_count} predictors but only {represented_observations} represented observations"
    )]
    InsufficientEquationObservations {
        outcome_id: String,
        represented_observations: u64,
        predictor_count: usize,
    },
    #[error("joint equation for {outcome_id} is rank deficient at {predictor_id}")]
    RankDeficient {
        outcome_id: String,
        predictor_id: String,
    },
    #[error("joint equation for {outcome_id} produced a nonfinite coefficient")]
    NonfiniteCoefficient { outcome_id: String },
    #[error("conditional interaction point result is internally inconsistent: {0}")]
    InvalidResult(String),
}

#[derive(Debug, Clone)]
struct OwnedRowMassV2 {
    values: Vec<f64>,
    semantics: MultimodConditionalMomentSemanticsV2,
    represented_observations: u64,
    sum: f64,
    variance_denominator: f64,
}

impl OwnedRowMassV2 {
    fn from_input(
        row_count: usize,
        input: MultimodConditionalRowMassV2<'_>,
    ) -> Result<Self, MultimodConditionalInteractionPointErrorV2> {
        if row_count < 3 {
            return Err(
                MultimodConditionalInteractionPointErrorV2::InsufficientEquationObservations {
                    outcome_id: "all".into(),
                    represented_observations: row_count as u64,
                    predictor_count: 0,
                },
            );
        }
        match input {
            MultimodConditionalRowMassV2::Unweighted => Ok(Self {
                values: vec![1.0; row_count],
                semantics: MultimodConditionalMomentSemanticsV2::UnweightedSample,
                represented_observations: row_count as u64,
                sum: row_count as f64,
                variance_denominator: (row_count - 1) as f64,
            }),
            MultimodConditionalRowMassV2::PositiveCase(weights) => {
                if weights.len() != row_count {
                    return Err(MultimodConditionalInteractionPointErrorV2::RowMassLength {
                        expected: row_count,
                        actual: weights.len(),
                    });
                }
                if let Some(row) = weights
                    .iter()
                    .position(|weight| !weight.is_finite() || *weight <= 0.0)
                {
                    return Err(
                        MultimodConditionalInteractionPointErrorV2::InvalidCaseWeight { row },
                    );
                }
                let sum = stable_sum_v2(weights);
                let sum_squared = stable_sum_v2(
                    &weights
                        .iter()
                        .map(|weight| weight * weight)
                        .collect::<Vec<_>>(),
                );
                let denominator = sum - sum_squared / sum;
                if !sum.is_finite() || !denominator.is_finite() || denominator <= f64::EPSILON {
                    return Err(
                        MultimodConditionalInteractionPointErrorV2::RowMassDegreesOfFreedom,
                    );
                }
                Ok(Self {
                    values: weights.to_vec(),
                    semantics: MultimodConditionalMomentSemanticsV2::PositiveCaseReliability,
                    represented_observations: row_count as u64,
                    sum,
                    variance_denominator: denominator,
                })
            }
            MultimodConditionalRowMassV2::PositiveIntegerFrequency(counts) => {
                if counts.len() != row_count {
                    return Err(MultimodConditionalInteractionPointErrorV2::RowMassLength {
                        expected: row_count,
                        actual: counts.len(),
                    });
                }
                let mut total = 0_u64;
                for (row, count) in counts.iter().enumerate() {
                    if *count == 0 {
                        return Err(
                            MultimodConditionalInteractionPointErrorV2::InvalidFrequency { row },
                        );
                    }
                    total = total
                        .checked_add(*count)
                        .filter(|total| *total <= (1_u64 << 53) - 1)
                        .ok_or(MultimodConditionalInteractionPointErrorV2::FrequencyTotal)?;
                }
                if total < 3 {
                    return Err(
                        MultimodConditionalInteractionPointErrorV2::RowMassDegreesOfFreedom,
                    );
                }
                Ok(Self {
                    values: counts.iter().map(|count| *count as f64).collect(),
                    semantics:
                        MultimodConditionalMomentSemanticsV2::PositiveIntegerFrequencyCountSpace,
                    represented_observations: total,
                    sum: total as f64,
                    variance_denominator: (total - 1) as f64,
                })
            }
        }
    }

    fn weighted_mean(&self, values: &[f64]) -> f64 {
        stable_sum_v2(
            &values
                .iter()
                .zip(&self.values)
                .map(|(value, weight)| value * weight)
                .collect::<Vec<_>>(),
        ) / self.sum
    }

    fn standardize(&self, values: &[f64]) -> Option<(Vec<f64>, f64, f64)> {
        let mean = self.weighted_mean(values);
        let centered_sum = stable_sum_v2(
            &values
                .iter()
                .zip(&self.values)
                .map(|(value, weight)| weight * (value - mean).powi(2))
                .collect::<Vec<_>>(),
        );
        let standard_deviation = (centered_sum / self.variance_denominator).sqrt();
        if !mean.is_finite()
            || !standard_deviation.is_finite()
            || standard_deviation <= f64::EPSILON
        {
            return None;
        }
        Some((
            values
                .iter()
                .map(|value| (value - mean) / standard_deviation)
                .collect(),
            mean,
            standard_deviation,
        ))
    }
}

#[derive(Debug, Clone)]
enum PredictorKindV2<'a> {
    Ordinary {
        relation_id: &'a str,
        source_id: &'a str,
    },
    Interaction {
        interaction: &'a MultimodConditionalTwoWayInteractionV2,
        product_standard_deviation: f64,
    },
}

#[derive(Debug, Clone)]
struct PredictorColumnV2<'a> {
    stable_id: &'a str,
    values: &'a [f64],
    kind: PredictorKindV2<'a>,
}

/// Fits all ordinary paths and all declared two-way products jointly for each
/// outcome.  Positive case weights use the production WPLS reliability-weight
/// moment convention.  Positive integer frequencies use `N - 1` moments and
/// weighted normal equations, which are algebraically identical to physically
/// expanding each compact row `count` times.
pub fn estimate_multimod_conditional_interactions_v2_with_control(
    base_plan: &CompiledPlsPlanV2,
    interactions: &[MultimodConditionalTwoWayInteractionV2],
    stage_scores: &BTreeMap<String, Vec<f64>>,
    row_mass: MultimodConditionalRowMassV2<'_>,
    should_continue: impl Fn() -> bool,
) -> Result<MultimodConditionalInteractionPointV2, MultimodConditionalInteractionPointErrorV2> {
    if !should_continue() {
        return Err(MultimodConditionalInteractionPointErrorV2::Cancelled);
    }
    validate_interactions_v2(base_plan, interactions)?;
    let required_ids = base_plan
        .blocks()
        .iter()
        .map(|block| block.construct_id().to_string())
        .collect::<BTreeSet<_>>();
    let first = required_ids.iter().next().ok_or_else(|| {
        MultimodConditionalInteractionPointErrorV2::InvalidResult(
            "base plan has no score blocks".into(),
        )
    })?;
    let row_count = stage_scores
        .get(first)
        .ok_or_else(
            || MultimodConditionalInteractionPointErrorV2::MissingScore {
                variable_id: first.clone(),
            },
        )?
        .len();
    let row_mass = OwnedRowMassV2::from_input(row_count, row_mass)?;

    let mut scores = BTreeMap::<String, Vec<f64>>::new();
    for variable_id in required_ids {
        if !should_continue() {
            return Err(MultimodConditionalInteractionPointErrorV2::Cancelled);
        }
        let values = stage_scores.get(&variable_id).ok_or_else(|| {
            MultimodConditionalInteractionPointErrorV2::MissingScore {
                variable_id: variable_id.clone(),
            }
        })?;
        if values.len() != row_count {
            return Err(MultimodConditionalInteractionPointErrorV2::ScoreLength {
                variable_id,
                expected: row_count,
                actual: values.len(),
            });
        }
        if let Some(row) = values.iter().position(|value| !value.is_finite()) {
            return Err(MultimodConditionalInteractionPointErrorV2::NonfiniteScore {
                variable_id,
                row,
            });
        }
        let standardized = row_mass.standardize(values).ok_or_else(|| {
            MultimodConditionalInteractionPointErrorV2::ConstantScore {
                variable_id: variable_id.clone(),
            }
        })?;
        scores.insert(variable_id, standardized.0);
    }

    let mut products = BTreeMap::<String, Vec<f64>>::new();
    let mut product_receipts = Vec::with_capacity(interactions.len());
    for interaction in interactions {
        if !should_continue() {
            return Err(MultimodConditionalInteractionPointErrorV2::Cancelled);
        }
        let focal = &scores[&interaction.focal_predictor_id];
        let moderator = &scores[&interaction.moderator_id];
        let raw_product = focal
            .iter()
            .zip(moderator)
            .map(|(focal, moderator)| focal * moderator)
            .collect::<Vec<_>>();
        let (standardized, mean, standard_deviation) =
            row_mass.standardize(&raw_product).ok_or_else(|| {
                MultimodConditionalInteractionPointErrorV2::ConstantProduct {
                    interaction_id: interaction.interaction_id.clone(),
                }
            })?;
        products.insert(interaction.interaction_id.clone(), standardized);
        product_receipts.push(MultimodConditionalProductScaleReceiptV2 {
            scale_version: GENERAL_SEM_PLS_PRODUCT_SCALE_VERSION_V1.into(),
            interaction_id: interaction.interaction_id.clone(),
            semantics: row_mass.semantics,
            compact_row_count: row_count,
            represented_observation_count: row_mass.represented_observations,
            weighted_product_mean: mean,
            weighted_product_standard_deviation: standard_deviation,
            variance_denominator: row_mass.variance_denominator,
        });
    }

    let outcomes = base_plan
        .paths()
        .iter()
        .map(|path| path.target().to_string())
        .chain(interactions.iter().map(|value| value.outcome_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut edge_map = BTreeMap::<String, ConditionalEdgeFunctionV2>::new();
    for outcome_id in outcomes {
        if !should_continue() {
            return Err(MultimodConditionalInteractionPointErrorV2::Cancelled);
        }
        let outcome = scores.get(&outcome_id).ok_or_else(|| {
            MultimodConditionalInteractionPointErrorV2::MissingScore {
                variable_id: outcome_id.clone(),
            }
        })?;
        let mut predictors = Vec::<PredictorColumnV2<'_>>::new();
        for path in base_plan
            .paths()
            .iter()
            .filter(|path| path.target() == outcome_id)
        {
            predictors.push(PredictorColumnV2 {
                stable_id: path.relation_id(),
                values: scores.get(path.source()).ok_or_else(|| {
                    MultimodConditionalInteractionPointErrorV2::MissingScore {
                        variable_id: path.source().into(),
                    }
                })?,
                kind: PredictorKindV2::Ordinary {
                    relation_id: path.relation_id(),
                    source_id: path.source(),
                },
            });
        }
        for interaction in interactions
            .iter()
            .filter(|interaction| interaction.outcome_id == outcome_id)
        {
            let receipt = product_receipts
                .iter()
                .find(|receipt| receipt.interaction_id == interaction.interaction_id)
                .expect("each validated interaction has one product receipt");
            predictors.push(PredictorColumnV2 {
                stable_id: &interaction.interaction_effect_relation_id,
                values: &products[&interaction.interaction_id],
                kind: PredictorKindV2::Interaction {
                    interaction,
                    product_standard_deviation: receipt.weighted_product_standard_deviation,
                },
            });
        }
        predictors.sort_by(|left, right| left.stable_id.cmp(right.stable_id));
        if predictors.len() >= row_mass.represented_observations as usize {
            return Err(
                MultimodConditionalInteractionPointErrorV2::InsufficientEquationObservations {
                    outcome_id,
                    represented_observations: row_mass.represented_observations,
                    predictor_count: predictors.len(),
                },
            );
        }
        let estimates = solve_weighted_qr_v2(
            &predictors,
            outcome,
            &row_mass,
            &outcome_id,
            &should_continue,
        )?;
        let mut pending_interaction_estimates = Vec::new();
        for (predictor, estimate) in predictors.into_iter().zip(estimates) {
            match predictor.kind {
                PredictorKindV2::Ordinary {
                    relation_id,
                    source_id,
                } => {
                    edge_map.insert(
                        relation_id.into(),
                        ConditionalEdgeFunctionV2 {
                            relation_id: relation_id.into(),
                            source_id: source_id.into(),
                            target_id: outcome_id.clone(),
                            intercept: estimate,
                            linear_coefficients: Vec::new(),
                            pairwise_coefficients: Vec::new(),
                        },
                    );
                }
                PredictorKindV2::Interaction {
                    interaction,
                    product_standard_deviation,
                } => {
                    pending_interaction_estimates.push((
                        interaction,
                        product_standard_deviation,
                        estimate,
                    ));
                }
            }
        }
        // QR columns retain canonical stable-id order, but result assembly must
        // not depend on an interaction relation sorting after its focal edge.
        // Attach scientific gammas only after every ordinary edge for this
        // joint outcome equation has been materialized.
        for (interaction, product_standard_deviation, estimate) in pending_interaction_estimates {
            let edge = edge_map
                .get_mut(&interaction.focal_relation_id)
                .ok_or_else(|| {
                    MultimodConditionalInteractionPointErrorV2::InvalidResult(format!(
                        "joint equation omitted focal relation {}",
                        interaction.focal_relation_id
                    ))
                })?;
            edge.linear_coefficients
                .push(ConditionalLinearCoefficientV2 {
                    moderator_id: interaction.moderator_id.clone(),
                    estimate: estimate / product_standard_deviation,
                });
        }
    }
    let mut edges = edge_map.into_values().collect::<Vec<_>>();
    for edge in &mut edges {
        edge.linear_coefficients
            .sort_by(|left, right| left.moderator_id.cmp(&right.moderator_id));
    }
    edges.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    product_receipts.sort_by(|left, right| left.interaction_id.cmp(&right.interaction_id));
    let receipt = MultimodConditionalInteractionPointReceiptV2 {
        method_version: MULTIMOD_CONDITIONAL_INTERACTION_POINT_METHOD_VERSION_V2.into(),
        moment_contract: if row_mass.semantics
            == MultimodConditionalMomentSemanticsV2::PositiveIntegerFrequencyCountSpace
        {
            MULTIMOD_CONDITIONAL_FREQUENCY_EQUIVALENCE_CONTRACT_V2.into()
        } else {
            MULTIMOD_CONDITIONAL_WEIGHT_MOMENT_CONTRACT_V2.into()
        },
        base_plan_sha256: sha256_serialized(base_plan),
        interaction_inventory_sha256: sha256_serialized(&interactions),
        semantics: row_mass.semantics,
        compact_row_count: row_count,
        represented_observation_count: row_mass.represented_observations,
        row_mass_sum: row_mass.sum,
        variance_denominator: row_mass.variance_denominator,
        exact_frequency_row_expansion_equivalence: row_mass.semantics
            == MultimodConditionalMomentSemanticsV2::PositiveIntegerFrequencyCountSpace,
    };
    let result = MultimodConditionalInteractionPointV2 {
        edges,
        product_scale_receipts: product_receipts,
        receipt,
    };
    validate_result_v2(base_plan, interactions, &result)?;
    Ok(result)
}

fn validate_interactions_v2(
    base_plan: &CompiledPlsPlanV2,
    interactions: &[MultimodConditionalTwoWayInteractionV2],
) -> Result<(), MultimodConditionalInteractionPointErrorV2> {
    if interactions.is_empty() {
        return Err(MultimodConditionalInteractionPointErrorV2::NoInteractions);
    }
    if interactions
        .windows(2)
        .any(|pair| pair[0].interaction_id >= pair[1].interaction_id)
    {
        return Err(MultimodConditionalInteractionPointErrorV2::InteractionOrder);
    }
    let base_relations = base_plan
        .paths()
        .iter()
        .map(|path| path.relation_id())
        .collect::<BTreeSet<_>>();
    let mut effect_relations = BTreeSet::new();
    let mut products = BTreeSet::new();
    for interaction in interactions {
        let focal = base_plan
            .paths()
            .iter()
            .find(|path| path.relation_id() == interaction.focal_relation_id)
            .ok_or_else(|| {
                MultimodConditionalInteractionPointErrorV2::InteractionContract(format!(
                    "{} lacks focal relation {}",
                    interaction.interaction_id, interaction.focal_relation_id
                ))
            })?;
        if focal.source() != interaction.focal_predictor_id
            || focal.target() != interaction.outcome_id
            || interaction.focal_predictor_id == interaction.moderator_id
            || interaction.output_id.is_empty()
            || interaction.interaction_effect_parameter_id.is_empty()
            || base_relations.contains(interaction.interaction_effect_relation_id.as_str())
            || !effect_relations.insert(interaction.interaction_effect_relation_id.as_str())
            || !products.insert((
                interaction.focal_predictor_id.as_str(),
                interaction.moderator_id.as_str(),
                interaction.outcome_id.as_str(),
            ))
        {
            return Err(
                MultimodConditionalInteractionPointErrorV2::InteractionContract(format!(
                    "{} has incoherent focal/effect/product identity",
                    interaction.interaction_id
                )),
            );
        }
    }
    Ok(())
}

fn solve_weighted_qr_v2(
    predictors: &[PredictorColumnV2<'_>],
    outcome: &[f64],
    mass: &OwnedRowMassV2,
    outcome_id: &str,
    should_continue: &impl Fn() -> bool,
) -> Result<Vec<f64>, MultimodConditionalInteractionPointErrorV2> {
    if predictors.is_empty() {
        return Ok(Vec::new());
    }
    let rows = outcome.len();
    let columns = predictors.len();
    let outcome_mean = mass.weighted_mean(outcome);
    let predictor_means = predictors
        .iter()
        .map(|predictor| mass.weighted_mean(predictor.values))
        .collect::<Vec<_>>();
    let mut q = vec![vec![0.0; rows]; columns];
    let mut r = vec![vec![0.0; columns]; columns];
    let mut rhs = vec![0.0; columns];
    let weighted_outcome = outcome
        .iter()
        .zip(&mass.values)
        .map(|(value, weight)| (value - outcome_mean) * weight.sqrt())
        .collect::<Vec<_>>();

    for column in 0..columns {
        if !should_continue() {
            return Err(MultimodConditionalInteractionPointErrorV2::Cancelled);
        }
        let mut vector = predictors[column]
            .values
            .iter()
            .zip(&mass.values)
            .map(|(value, weight)| (value - predictor_means[column]) * weight.sqrt())
            .collect::<Vec<_>>();
        // Two-pass modified Gram-Schmidt gives a deterministic, compact-row
        // factorization while avoiding any count-space expansion.
        for _ in 0..2 {
            for previous in 0..column {
                let projection = dot_v2(&q[previous], &vector);
                r[previous][column] += projection;
                subtract_scaled_v2(&mut vector, &q[previous], projection);
            }
        }
        let norm = dot_v2(&vector, &vector).sqrt();
        let reference_norm = predictors[column]
            .values
            .iter()
            .zip(&mass.values)
            .map(|(value, weight)| weight * (value - predictor_means[column]).powi(2))
            .sum::<f64>()
            .sqrt();
        let tolerance = 256.0 * f64::EPSILON * (rows.max(columns) as f64) * reference_norm.max(1.0);
        if !norm.is_finite() || norm <= tolerance {
            return Err(MultimodConditionalInteractionPointErrorV2::RankDeficient {
                outcome_id: outcome_id.into(),
                predictor_id: predictors[column].stable_id.into(),
            });
        }
        r[column][column] = norm;
        for (target, value) in q[column].iter_mut().zip(vector) {
            *target = value / norm;
        }
        rhs[column] = dot_v2(&q[column], &weighted_outcome);
    }

    let mut coefficients = vec![0.0; columns];
    for row in (0..columns).rev() {
        let remainder = (row + 1..columns)
            .map(|column| r[row][column] * coefficients[column])
            .sum::<f64>();
        coefficients[row] = (rhs[row] - remainder) / r[row][row];
    }
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(
            MultimodConditionalInteractionPointErrorV2::NonfiniteCoefficient {
                outcome_id: outcome_id.into(),
            },
        );
    }
    Ok(coefficients)
}

fn validate_result_v2(
    base_plan: &CompiledPlsPlanV2,
    interactions: &[MultimodConditionalTwoWayInteractionV2],
    result: &MultimodConditionalInteractionPointV2,
) -> Result<(), MultimodConditionalInteractionPointErrorV2> {
    if result.receipt.method_version != MULTIMOD_CONDITIONAL_INTERACTION_POINT_METHOD_VERSION_V2
        || result.receipt.base_plan_sha256 != sha256_serialized(base_plan)
        || result.receipt.interaction_inventory_sha256 != sha256_serialized(&interactions)
        || result.edges.len() != base_plan.paths().len()
        || result.product_scale_receipts.len() != interactions.len()
        || result
            .edges
            .windows(2)
            .any(|pair| pair[0].relation_id >= pair[1].relation_id)
    {
        return Err(MultimodConditionalInteractionPointErrorV2::InvalidResult(
            "receipt or canonical cardinality differs from the compiled inputs".into(),
        ));
    }
    for path in base_plan.paths() {
        let edge = result
            .edges
            .iter()
            .find(|edge| edge.relation_id == path.relation_id())
            .ok_or_else(|| {
                MultimodConditionalInteractionPointErrorV2::InvalidResult(format!(
                    "ordinary relation {} is absent",
                    path.relation_id()
                ))
            })?;
        if edge.source_id != path.source()
            || edge.target_id != path.target()
            || !edge.intercept.is_finite()
            || !edge.pairwise_coefficients.is_empty()
        {
            return Err(MultimodConditionalInteractionPointErrorV2::InvalidResult(
                format!(
                    "ordinary relation {} metadata is incoherent",
                    path.relation_id()
                ),
            ));
        }
        let mut expected = interactions
            .iter()
            .filter(|interaction| interaction.focal_relation_id == path.relation_id())
            .map(|interaction| interaction.moderator_id.as_str())
            .collect::<Vec<_>>();
        expected.sort_unstable();
        let actual = edge
            .linear_coefficients
            .iter()
            .map(|coefficient| coefficient.moderator_id.as_str())
            .collect::<Vec<_>>();
        if actual != expected
            || edge
                .linear_coefficients
                .iter()
                .any(|coefficient| !coefficient.estimate.is_finite())
        {
            return Err(MultimodConditionalInteractionPointErrorV2::InvalidResult(
                format!(
                    "ordinary relation {} gamma ledger is incoherent",
                    path.relation_id()
                ),
            ));
        }
    }
    Ok(())
}

fn dot_v2(left: &[f64], right: &[f64]) -> f64 {
    stable_sum_v2(
        &left
            .iter()
            .zip(right)
            .map(|(left, right)| left * right)
            .collect::<Vec<_>>(),
    )
}

fn subtract_scaled_v2(target: &mut [f64], source: &[f64], scale: f64) {
    for (target, source) in target.iter_mut().zip(source) {
        *target -= source * scale;
    }
}

fn stable_sum_v2(values: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for value in values {
        let updated = sum + value;
        if sum.abs() >= value.abs() {
            compensation += (sum - updated) + value;
        } else {
            compensation += (value - updated) + sum;
        }
        sum = updated;
    }
    sum + compensation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_space_moments_match_expanded_rows() {
        let compact = vec![1.0, 3.0, 8.0];
        let counts = vec![2_u64, 1, 3];
        let mass = OwnedRowMassV2::from_input(
            compact.len(),
            MultimodConditionalRowMassV2::PositiveIntegerFrequency(&counts),
        )
        .unwrap();
        let (_, compact_mean, compact_sd) = mass.standardize(&compact).unwrap();
        let expanded = vec![1.0, 1.0, 3.0, 8.0, 8.0, 8.0];
        let expanded_mass =
            OwnedRowMassV2::from_input(expanded.len(), MultimodConditionalRowMassV2::Unweighted)
                .unwrap();
        let (_, expanded_mean, expanded_sd) = expanded_mass.standardize(&expanded).unwrap();
        assert_eq!(compact_mean.to_bits(), expanded_mean.to_bits());
        assert!((compact_sd - expanded_sd).abs() < 1.0e-14);
    }

    #[test]
    fn case_weight_scale_does_not_change_standardized_scores() {
        let values = vec![0.0, 2.0, 5.0, 9.0];
        let left = OwnedRowMassV2::from_input(
            values.len(),
            MultimodConditionalRowMassV2::PositiveCase(&[0.5, 1.0, 2.0, 4.0]),
        )
        .unwrap()
        .standardize(&values)
        .unwrap()
        .0;
        let right = OwnedRowMassV2::from_input(
            values.len(),
            MultimodConditionalRowMassV2::PositiveCase(&[5.0, 10.0, 20.0, 40.0]),
        )
        .unwrap()
        .standardize(&values)
        .unwrap()
        .0;
        for (left, right) in left.iter().zip(right) {
            assert!((left - right).abs() < 1.0e-14);
        }
    }

    #[test]
    fn interaction_estimates_attach_after_ordinary_edges_regardless_of_stable_id_order() {
        let plan: CompiledPlsPlanV2 = serde_json::from_value(serde_json::json!({
            "model_id": "identifier-order-fixture",
            "scientific_hash": "f".repeat(64),
            "dataset_id": "identifier-order-data",
            "blocks": [
                {"construct_id": "x", "mode": "mode_a", "indicators": []},
                {"construct_id": "y", "mode": "mode_a", "indicators": []},
                {"construct_id": "z", "mode": "mode_a", "indicators": []}
            ],
            "paths": [
                {
                    "relation_id": "m_moderator_main",
                    "source": "z",
                    "target": "y",
                    "parameter_id": "parameter:z:y"
                },
                {
                    "relation_id": "z_focal",
                    "source": "x",
                    "target": "y",
                    "parameter_id": "parameter:x:y"
                }
            ]
        }))
        .unwrap();
        let interaction = MultimodConditionalTwoWayInteractionV2 {
            interaction_id: "interaction:x:z:y".into(),
            output_id: "derived:interaction:x:z:y".into(),
            focal_relation_id: "z_focal".into(),
            interaction_effect_relation_id: "a_interaction_effect".into(),
            interaction_effect_parameter_id: "parameter:interaction:x:z:y".into(),
            focal_predictor_id: "x".into(),
            moderator_id: "z".into(),
            outcome_id: "y".into(),
        };
        assert!(interaction.interaction_effect_relation_id < interaction.focal_relation_id);

        let x = [-2.0, -1.4, -0.9, -0.2, 0.4, 1.1, 1.6, 2.3];
        let z = [-1.2, 0.8, -0.4, 1.5, -1.6, 0.3, 1.2, -0.7];
        let y = x
            .iter()
            .zip(z)
            .map(|(x, z)| 0.55 * x + 0.21 * z + 0.34 * x * z)
            .collect::<Vec<_>>();
        let scores = BTreeMap::from([
            ("x".into(), x.to_vec()),
            ("y".into(), y),
            ("z".into(), z.to_vec()),
        ]);
        let weights = [0.7, 1.0, 1.4, 0.9, 1.8, 1.1, 0.8, 1.3];
        let result = estimate_multimod_conditional_interactions_v2_with_control(
            &plan,
            &[interaction],
            &scores,
            MultimodConditionalRowMassV2::PositiveCase(&weights),
            || true,
        )
        .unwrap();

        let focal = result
            .edges
            .iter()
            .find(|edge| edge.relation_id == "z_focal")
            .unwrap();
        assert_eq!(focal.linear_coefficients.len(), 1);
        assert_eq!(focal.linear_coefficients[0].moderator_id, "z");
        assert!(focal.linear_coefficients[0].estimate.is_finite());
        assert_eq!(
            result.receipt.semantics,
            MultimodConditionalMomentSemanticsV2::PositiveCaseReliability
        );
    }
}
