//! Generalized conditional-process math for the additive V2 contract.
//!
//! This module is intentionally independent of the legacy moderated-mediation
//! implementation.  It compiles every explicitly selected indirect path into
//! one canonical multivariate polynomial and derives all reported estimands
//! from that polynomial.  Runtime and persistence adapters may therefore bind
//! this kernel without changing any V1 meaning.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ContinuousCDF, Normal};
use std::collections::{BTreeMap, BTreeSet};

pub const GENERAL_SEM_CONDITIONAL_PROCESS_METHOD_VERSION_V2: &str =
    "general_sem_conditional_process_v2";
pub const CONDITIONAL_PROCESS_DEFAULT_SEED_V2: u64 = 42;
pub const CONDITIONAL_PROCESS_MAX_MODERATORS_V2: usize = 4;
pub const CONDITIONAL_PROCESS_MAX_VALUES_PER_MODERATOR_V2: usize = 5;
pub const CONDITIONAL_PROCESS_MAX_CARTESIAN_PROBES_V2: usize = 81;
pub const CONDITIONAL_PROCESS_MAX_EXPLICIT_PROBES_V2: usize = 100;
pub const CONDITIONAL_PROCESS_MAX_PROBE_CONTRASTS_V2: usize = 16;
pub const CONDITIONAL_PROCESS_MAX_CONDITIONAL_CELLS_V2: usize = 512;
pub const CONDITIONAL_PROCESS_MAX_INFERENTIAL_TARGETS_V2: usize = 1_024;
pub const CONDITIONAL_PROCESS_MIN_USABLE_RESAMPLE_FRACTION_V2: f64 = 0.90;
pub const CONDITIONAL_PROCESS_CASE_WEIGHT_RATIO_LIMIT_V2: f64 = 1.0e6;
pub const CONDITIONAL_PROCESS_MAX_EXACT_FREQUENCY_TOTAL_V2: u64 = (1_u64 << 53) - 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProcessProfileV2 {
    MultiTwoWayPercentile,
    MultiTwoWayBca,
    Studentized,
    BoundedThreeWay,
    MultipleHoc,
    Grouped,
    CaseWeighted,
    FrequencyWeighted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalInferenceMethodV2 {
    Percentile,
    Bca,
    Studentized,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalAlternativeV2 {
    TwoSided,
    Less,
    Greater,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalWeightModeV2 {
    None,
    PositiveCase,
    PositiveIntegerFrequency,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalInteractionOrderV2 {
    TwoWay,
    ThreeWay,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HocApproachV2 {
    RepeatedIndicators,
    ExtendedRepeatedIndicators,
    EmbeddedTwoStage,
    DisjointTwoStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalLinearCoefficientV2 {
    pub moderator_id: String,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalPairwiseCoefficientV2 {
    pub first_moderator_id: String,
    pub second_moderator_id: String,
    pub estimate: f64,
}

/// One structural edge represented as
/// `intercept + sum(gamma_j z_j) + sum(delta_jk z_j z_k)`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalEdgeFunctionV2 {
    pub relation_id: String,
    pub source_id: String,
    pub target_id: String,
    pub intercept: f64,
    #[serde(default)]
    pub linear_coefficients: Vec<ConditionalLinearCoefficientV2>,
    #[serde(default)]
    pub pairwise_coefficients: Vec<ConditionalPairwiseCoefficientV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExplicitConditionalPathV2 {
    pub path_id: String,
    /// Edges are ordered from the focal source to the final outcome.
    pub edges: Vec<ConditionalEdgeFunctionV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ModeratorPowerV2 {
    pub moderator_id: String,
    pub exponent: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalPolynomialTermV2 {
    /// Empty powers identify the constant term.
    pub powers: Vec<ModeratorPowerV2>,
    pub coefficient: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalPathPolynomialV2 {
    pub method_version: String,
    pub path_id: String,
    pub relation_ids: Vec<String>,
    pub moderator_ids: Vec<String>,
    pub terms: Vec<ConditionalPolynomialTermV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProbePointV2 {
    pub probe_id: String,
    pub standardized_values: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalEffectV2 {
    pub target_id: String,
    pub path_id: String,
    pub probe_id: String,
    pub estimate: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalDerivativeKindV2 {
    First,
    Second,
    Cross,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalDerivativeV2 {
    pub target_id: String,
    pub path_id: String,
    pub probe_id: String,
    pub kind: ConditionalDerivativeKindV2,
    pub first_moderator_id: String,
    pub second_moderator_id: Option<String>,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScalarIndexOfModeratedMediationV2 {
    pub target_id: String,
    pub path_id: String,
    pub moderator_id: String,
    pub estimate: f64,
    pub eligibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProbeContrastV2 {
    pub target_id: String,
    pub path_id: String,
    pub left_probe_id: String,
    pub right_probe_id: String,
    /// Defined as the left conditional effect minus the right effect.
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalInteractionBindingV2 {
    pub interaction_id: String,
    pub order: ConditionalInteractionOrderV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalPathShapeV2 {
    pub path_id: String,
    pub edge_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalHocBindingV2 {
    pub hoc_id: String,
    pub approach: HocApproachV2,
    pub member_construct_ids: Vec<String>,
    pub nested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionalProbePlanV2 {
    Cartesian {
        axes: Vec<ConditionalProbeAxisV2>,
    },
    Explicit {
        points: Vec<ConditionalProbePointV2>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProbeAxisV2 {
    pub moderator_id: String,
    pub standardized_values: Vec<f64>,
}

impl Default for ConditionalProbePlanV2 {
    fn default() -> Self {
        Self::Cartesian {
            axes: vec![ConditionalProbeAxisV2 {
                moderator_id: "moderator".to_owned(),
                standardized_values: vec![-1.0, 0.0, 1.0],
            }],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessProfileRequestV2 {
    pub profile: ConditionalProcessProfileV2,
    pub inference_method: ConditionalInferenceMethodV2,
    pub alternative: ConditionalAlternativeV2,
    pub interactions: Vec<ConditionalInteractionBindingV2>,
    /// Required only by `bounded_three_way`; the compiler sets this after
    /// verifying both pairwise terms and all three main effects.
    pub three_way_lower_order_closure_complete: bool,
    pub selected_paths: Vec<ConditionalPathShapeV2>,
    pub moderator_ids: Vec<String>,
    #[serde(default)]
    pub hocs: Vec<ConditionalHocBindingV2>,
    #[serde(default)]
    pub group_ids: Vec<String>,
    pub weight_mode: ConditionalWeightModeV2,
    pub probes: ConditionalProbePlanV2,
    pub requested_probe_contrast_count: usize,
    pub requested_inferential_target_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessProfileReceiptV2 {
    pub method_version: String,
    pub profile: ConditionalProcessProfileV2,
    pub probe_point_count: usize,
    pub conditional_cell_count: usize,
    pub requested_inferential_target_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProcessBlockerCodeV2 {
    EmptyOrDuplicateIdentity,
    UnsupportedInferenceForProfile,
    UnsupportedAlternativeForProfile,
    InteractionCountOutsideProfile,
    ThreeWayOutsideProfile,
    PathCountOutsideProfile,
    PathLengthOutsideProfile,
    ModeratorCountOutsideProfile,
    HocOutsideProfile,
    HocNestedOrOverlapping,
    HocApproachMismatch,
    GroupCountOutsideProfile,
    WeightModeOutsideProfile,
    ProbePlanOutsideProfile,
    ProbeContrastLimitExceeded,
    ConditionalCellLimitExceeded,
    InferentialTargetLimitExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessBlockerV2 {
    pub code: ConditionalProcessBlockerCodeV2,
    pub detail: String,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConditionalProcessMathErrorV2 {
    #[error("identity must be non-empty and unique: {0}")]
    InvalidIdentity(String),
    #[error("conditional coefficient is non-finite: {0}")]
    NonFiniteCoefficient(String),
    #[error("path {0} contains no relations")]
    EmptyPath(String),
    #[error(
        "path {path_id} is discontinuous between relations {left_relation_id} and {right_relation_id}"
    )]
    DiscontinuousPath {
        path_id: String,
        left_relation_id: String,
        right_relation_id: String,
    },
    #[error("moderator identity is duplicated within edge {0}")]
    DuplicateModerator(String),
    #[error("a pairwise coefficient must bind two distinct moderators in edge {0}")]
    RepeatedPairwiseModerator(String),
    #[error("probe {probe_id} is missing standardized moderator {moderator_id}")]
    MissingProbeValue {
        probe_id: String,
        moderator_id: String,
    },
    #[error("probe {probe_id} has a non-finite value for moderator {moderator_id}")]
    NonFiniteProbeValue {
        probe_id: String,
        moderator_id: String,
    },
    #[error("polynomial exponent overflowed while multiplying the selected path")]
    ExponentOverflow,
    #[error("conditional path polynomial overflowed to a non-finite value")]
    NonFinitePolynomial,
    #[error(
        "the conventional scalar index is defined only for an affine indirect effect in exactly the requested moderator"
    )]
    ScalarIndexNotAffine,
    #[error("resampling confidence must be strictly between zero and one")]
    InvalidConfidence,
    #[error("resampling inputs are empty or contain non-finite values")]
    InvalidResamplingValues,
    #[error("only {usable} resamples are usable; at least {required} are required")]
    InsufficientUsableResamples { usable: usize, required: usize },
    #[error("studentized standard errors must be finite and strictly positive")]
    InvalidStudentizedStandardError,
    #[error("target identity serialization failed: {0}")]
    TargetIdentitySerialization(String),
}

type MonomialKeyV2 = Vec<(String, u8)>;

fn non_empty_identity(value: &str) -> bool {
    !value.trim().is_empty()
}

fn insert_term(
    terms: &mut BTreeMap<MonomialKeyV2, f64>,
    key: MonomialKeyV2,
    coefficient: f64,
) -> Result<(), ConditionalProcessMathErrorV2> {
    let entry = terms.entry(key).or_insert(0.0);
    *entry += coefficient;
    if entry.is_finite() {
        Ok(())
    } else {
        Err(ConditionalProcessMathErrorV2::NonFinitePolynomial)
    }
}

fn multiply_term_maps(
    left: &BTreeMap<MonomialKeyV2, f64>,
    right: &BTreeMap<MonomialKeyV2, f64>,
) -> Result<BTreeMap<MonomialKeyV2, f64>, ConditionalProcessMathErrorV2> {
    let mut product = BTreeMap::new();
    for (left_key, left_coefficient) in left {
        for (right_key, right_coefficient) in right {
            let mut powers = BTreeMap::<String, u8>::new();
            for (moderator_id, exponent) in left_key.iter().chain(right_key.iter()) {
                let accumulated = powers.get(moderator_id).copied().unwrap_or(0);
                let next = accumulated
                    .checked_add(*exponent)
                    .ok_or(ConditionalProcessMathErrorV2::ExponentOverflow)?;
                powers.insert(moderator_id.clone(), next);
            }
            insert_term(
                &mut product,
                powers.into_iter().collect(),
                left_coefficient * right_coefficient,
            )?;
        }
    }
    product.retain(|_, coefficient| *coefficient != 0.0);
    Ok(product)
}

fn edge_terms(
    edge: &ConditionalEdgeFunctionV2,
) -> Result<BTreeMap<MonomialKeyV2, f64>, ConditionalProcessMathErrorV2> {
    if !non_empty_identity(&edge.relation_id)
        || !non_empty_identity(&edge.source_id)
        || !non_empty_identity(&edge.target_id)
    {
        return Err(ConditionalProcessMathErrorV2::InvalidIdentity(
            edge.relation_id.clone(),
        ));
    }
    if !edge.intercept.is_finite() {
        return Err(ConditionalProcessMathErrorV2::NonFiniteCoefficient(
            edge.relation_id.clone(),
        ));
    }

    let mut terms = BTreeMap::new();
    insert_term(&mut terms, Vec::new(), edge.intercept)?;
    let mut linear_ids = BTreeSet::new();
    for coefficient in &edge.linear_coefficients {
        if !non_empty_identity(&coefficient.moderator_id)
            || !linear_ids.insert(coefficient.moderator_id.clone())
        {
            return Err(ConditionalProcessMathErrorV2::DuplicateModerator(
                edge.relation_id.clone(),
            ));
        }
        if !coefficient.estimate.is_finite() {
            return Err(ConditionalProcessMathErrorV2::NonFiniteCoefficient(
                edge.relation_id.clone(),
            ));
        }
        insert_term(
            &mut terms,
            vec![(coefficient.moderator_id.clone(), 1)],
            coefficient.estimate,
        )?;
    }

    let mut pair_ids = BTreeSet::new();
    for coefficient in &edge.pairwise_coefficients {
        if coefficient.first_moderator_id == coefficient.second_moderator_id {
            return Err(ConditionalProcessMathErrorV2::RepeatedPairwiseModerator(
                edge.relation_id.clone(),
            ));
        }
        if !non_empty_identity(&coefficient.first_moderator_id)
            || !non_empty_identity(&coefficient.second_moderator_id)
        {
            return Err(ConditionalProcessMathErrorV2::InvalidIdentity(
                edge.relation_id.clone(),
            ));
        }
        if !coefficient.estimate.is_finite() {
            return Err(ConditionalProcessMathErrorV2::NonFiniteCoefficient(
                edge.relation_id.clone(),
            ));
        }
        let (first, second) = if coefficient.first_moderator_id < coefficient.second_moderator_id {
            (
                coefficient.first_moderator_id.clone(),
                coefficient.second_moderator_id.clone(),
            )
        } else {
            (
                coefficient.second_moderator_id.clone(),
                coefficient.first_moderator_id.clone(),
            )
        };
        if !pair_ids.insert((first.clone(), second.clone())) {
            return Err(ConditionalProcessMathErrorV2::DuplicateModerator(
                edge.relation_id.clone(),
            ));
        }
        insert_term(
            &mut terms,
            vec![(first, 1), (second, 1)],
            coefficient.estimate,
        )?;
    }
    terms.retain(|_, coefficient| *coefficient != 0.0);
    Ok(terms)
}

/// Multiplies the edge functions of one explicit path in their declared order.
pub fn compile_explicit_conditional_path_v2(
    path: &ExplicitConditionalPathV2,
) -> Result<ConditionalPathPolynomialV2, ConditionalProcessMathErrorV2> {
    if !non_empty_identity(&path.path_id) {
        return Err(ConditionalProcessMathErrorV2::InvalidIdentity(
            path.path_id.clone(),
        ));
    }
    if path.edges.is_empty() {
        return Err(ConditionalProcessMathErrorV2::EmptyPath(
            path.path_id.clone(),
        ));
    }
    let mut relation_ids = BTreeSet::new();
    for edge in &path.edges {
        if !relation_ids.insert(edge.relation_id.clone()) {
            return Err(ConditionalProcessMathErrorV2::InvalidIdentity(
                edge.relation_id.clone(),
            ));
        }
    }
    for adjacent in path.edges.windows(2) {
        if adjacent[0].target_id != adjacent[1].source_id {
            return Err(ConditionalProcessMathErrorV2::DiscontinuousPath {
                path_id: path.path_id.clone(),
                left_relation_id: adjacent[0].relation_id.clone(),
                right_relation_id: adjacent[1].relation_id.clone(),
            });
        }
    }

    let mut product = BTreeMap::from([(Vec::new(), 1.0)]);
    for edge in &path.edges {
        product = multiply_term_maps(&product, &edge_terms(edge)?)?;
    }
    let moderator_ids = product
        .keys()
        .flat_map(|key| key.iter().map(|(id, _)| id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let terms = product
        .into_iter()
        .map(|(powers, coefficient)| ConditionalPolynomialTermV2 {
            powers: powers
                .into_iter()
                .map(|(moderator_id, exponent)| ModeratorPowerV2 {
                    moderator_id,
                    exponent,
                })
                .collect(),
            coefficient,
        })
        .collect();

    Ok(ConditionalPathPolynomialV2 {
        method_version: GENERAL_SEM_CONDITIONAL_PROCESS_METHOD_VERSION_V2.to_owned(),
        path_id: path.path_id.clone(),
        relation_ids: path
            .edges
            .iter()
            .map(|edge| edge.relation_id.clone())
            .collect(),
        moderator_ids,
        terms,
    })
}

fn validate_probe(
    polynomial: &ConditionalPathPolynomialV2,
    probe: &ConditionalProbePointV2,
) -> Result<(), ConditionalProcessMathErrorV2> {
    if !non_empty_identity(&probe.probe_id) {
        return Err(ConditionalProcessMathErrorV2::InvalidIdentity(
            probe.probe_id.clone(),
        ));
    }
    for moderator_id in &polynomial.moderator_ids {
        let value = probe.standardized_values.get(moderator_id).ok_or_else(|| {
            ConditionalProcessMathErrorV2::MissingProbeValue {
                probe_id: probe.probe_id.clone(),
                moderator_id: moderator_id.clone(),
            }
        })?;
        if !value.is_finite() {
            return Err(ConditionalProcessMathErrorV2::NonFiniteProbeValue {
                probe_id: probe.probe_id.clone(),
                moderator_id: moderator_id.clone(),
            });
        }
    }
    for (moderator_id, value) in &probe.standardized_values {
        if !value.is_finite() {
            return Err(ConditionalProcessMathErrorV2::NonFiniteProbeValue {
                probe_id: probe.probe_id.clone(),
                moderator_id: moderator_id.clone(),
            });
        }
    }
    Ok(())
}

fn evaluate_derivative(
    polynomial: &ConditionalPathPolynomialV2,
    probe: &ConditionalProbePointV2,
    derivative_orders: &BTreeMap<String, u8>,
) -> Result<f64, ConditionalProcessMathErrorV2> {
    validate_probe(polynomial, probe)?;
    let mut total = 0.0;
    for term in &polynomial.terms {
        let term_powers = term
            .powers
            .iter()
            .map(|power| (power.moderator_id.clone(), power.exponent))
            .collect::<BTreeMap<_, _>>();
        let mut multiplier = term.coefficient;
        let mut eliminated = false;
        let involved_moderators = polynomial
            .moderator_ids
            .iter()
            .cloned()
            .chain(derivative_orders.keys().cloned())
            .collect::<BTreeSet<_>>();
        for moderator_id in involved_moderators {
            let exponent = term_powers.get(&moderator_id).copied().unwrap_or(0);
            let derivative_order = derivative_orders.get(&moderator_id).copied().unwrap_or(0);
            if derivative_order > exponent {
                eliminated = true;
                break;
            }
            for factor in 0..derivative_order {
                multiplier *= f64::from(exponent - factor);
            }
            let remaining_exponent = exponent - derivative_order;
            if remaining_exponent > 0 {
                let value = probe
                    .standardized_values
                    .get(&moderator_id)
                    .copied()
                    .ok_or_else(|| ConditionalProcessMathErrorV2::MissingProbeValue {
                        probe_id: probe.probe_id.clone(),
                        moderator_id: moderator_id.clone(),
                    })?;
                multiplier *= value.powi(i32::from(remaining_exponent));
            }
        }
        if !eliminated {
            total += multiplier;
        }
    }
    if total.is_finite() {
        Ok(total)
    } else {
        Err(ConditionalProcessMathErrorV2::NonFinitePolynomial)
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TargetIdentityMaterialV2<'a> {
    method_version: &'static str,
    path_id: &'a str,
    target_kind: &'a str,
    identities: &'a [&'a str],
    probes: &'a [&'a ConditionalProbePointV2],
}

fn deterministic_target_id(
    polynomial: &ConditionalPathPolynomialV2,
    target_kind: &str,
    identities: &[&str],
    probes: &[&ConditionalProbePointV2],
) -> Result<String, ConditionalProcessMathErrorV2> {
    let material = TargetIdentityMaterialV2 {
        method_version: GENERAL_SEM_CONDITIONAL_PROCESS_METHOD_VERSION_V2,
        path_id: &polynomial.path_id,
        target_kind,
        identities,
        probes,
    };
    let bytes = serde_json::to_vec(&material).map_err(|error| {
        ConditionalProcessMathErrorV2::TargetIdentitySerialization(error.to_string())
    })?;
    let digest = Sha256::digest(bytes);
    Ok(format!("qpls.cp.v2.{:x}", digest))
}

pub fn conditional_effect_v2(
    polynomial: &ConditionalPathPolynomialV2,
    probe: &ConditionalProbePointV2,
) -> Result<ConditionalEffectV2, ConditionalProcessMathErrorV2> {
    let estimate = evaluate_derivative(polynomial, probe, &BTreeMap::new())?;
    Ok(ConditionalEffectV2 {
        target_id: deterministic_target_id(polynomial, "conditional_effect", &[], &[probe])?,
        path_id: polynomial.path_id.clone(),
        probe_id: probe.probe_id.clone(),
        estimate,
    })
}

/// Returns all first derivatives, pure second derivatives, and unique cross
/// derivatives at one joint probe.  A zero is retained because it is part of
/// the requested estimand, not treated as a missing result.
pub fn conditional_derivatives_v2(
    polynomial: &ConditionalPathPolynomialV2,
    probe: &ConditionalProbePointV2,
) -> Result<Vec<ConditionalDerivativeV2>, ConditionalProcessMathErrorV2> {
    validate_probe(polynomial, probe)?;
    let mut results = Vec::new();
    for moderator_id in &polynomial.moderator_ids {
        for (kind, order) in [
            (ConditionalDerivativeKindV2::First, 1_u8),
            (ConditionalDerivativeKindV2::Second, 2_u8),
        ] {
            let orders = BTreeMap::from([(moderator_id.clone(), order)]);
            let kind_id = match kind {
                ConditionalDerivativeKindV2::First => "first_derivative",
                ConditionalDerivativeKindV2::Second => "second_derivative",
                ConditionalDerivativeKindV2::Cross => unreachable!(),
            };
            results.push(ConditionalDerivativeV2 {
                target_id: deterministic_target_id(polynomial, kind_id, &[moderator_id], &[probe])?,
                path_id: polynomial.path_id.clone(),
                probe_id: probe.probe_id.clone(),
                kind,
                first_moderator_id: moderator_id.clone(),
                second_moderator_id: None,
                estimate: evaluate_derivative(polynomial, probe, &orders)?,
            });
        }
    }
    for first_index in 0..polynomial.moderator_ids.len() {
        for second_index in (first_index + 1)..polynomial.moderator_ids.len() {
            let first = &polynomial.moderator_ids[first_index];
            let second = &polynomial.moderator_ids[second_index];
            let orders = BTreeMap::from([(first.clone(), 1_u8), (second.clone(), 1_u8)]);
            results.push(ConditionalDerivativeV2 {
                target_id: deterministic_target_id(
                    polynomial,
                    "cross_derivative",
                    &[first, second],
                    &[probe],
                )?,
                path_id: polynomial.path_id.clone(),
                probe_id: probe.probe_id.clone(),
                kind: ConditionalDerivativeKindV2::Cross,
                first_moderator_id: first.clone(),
                second_moderator_id: Some(second.clone()),
                estimate: evaluate_derivative(polynomial, probe, &orders)?,
            });
        }
    }
    Ok(results)
}

/// Emits the conventional scalar index only when the complete selected-path
/// polynomial is affine in exactly one declared moderator.
pub fn scalar_index_of_moderated_mediation_v2(
    polynomial: &ConditionalPathPolynomialV2,
    moderator_id: &str,
) -> Result<ScalarIndexOfModeratedMediationV2, ConditionalProcessMathErrorV2> {
    if !non_empty_identity(moderator_id) {
        return Err(ConditionalProcessMathErrorV2::InvalidIdentity(
            moderator_id.to_owned(),
        ));
    }
    for term in &polynomial.terms {
        let total_degree = term
            .powers
            .iter()
            .map(|power| usize::from(power.exponent))
            .sum::<usize>();
        if total_degree > 1
            || term
                .powers
                .iter()
                .any(|power| power.moderator_id != moderator_id)
        {
            return Err(ConditionalProcessMathErrorV2::ScalarIndexNotAffine);
        }
    }
    let estimate = polynomial
        .terms
        .iter()
        .find(|term| {
            term.powers.len() == 1
                && term.powers[0].moderator_id == moderator_id
                && term.powers[0].exponent == 1
        })
        .map(|term| term.coefficient)
        .unwrap_or(0.0);
    Ok(ScalarIndexOfModeratedMediationV2 {
        target_id: deterministic_target_id(
            polynomial,
            "scalar_index_of_moderated_mediation",
            &[moderator_id],
            &[],
        )?,
        path_id: polynomial.path_id.clone(),
        moderator_id: moderator_id.to_owned(),
        estimate,
        eligibility: "affine_indirect_effect_in_exactly_one_moderator_v2".to_owned(),
    })
}

pub fn conditional_probe_contrast_v2(
    polynomial: &ConditionalPathPolynomialV2,
    left: &ConditionalProbePointV2,
    right: &ConditionalProbePointV2,
) -> Result<ConditionalProbeContrastV2, ConditionalProcessMathErrorV2> {
    let left_effect = conditional_effect_v2(polynomial, left)?;
    let right_effect = conditional_effect_v2(polynomial, right)?;
    Ok(ConditionalProbeContrastV2 {
        target_id: deterministic_target_id(
            polynomial,
            "probe_contrast_left_minus_right",
            &[],
            &[left, right],
        )?,
        path_id: polynomial.path_id.clone(),
        left_probe_id: left.probe_id.clone(),
        right_probe_id: right.probe_id.clone(),
        estimate: left_effect.estimate - right_effect.estimate,
    })
}

fn push_blocker(
    blockers: &mut Vec<ConditionalProcessBlockerV2>,
    code: ConditionalProcessBlockerCodeV2,
    detail: impl Into<String>,
) {
    blockers.push(ConditionalProcessBlockerV2 {
        code,
        detail: detail.into(),
    });
}

fn duplicate_or_empty(values: impl IntoIterator<Item = String>) -> bool {
    let mut seen = BTreeSet::new();
    values
        .into_iter()
        .any(|value| !non_empty_identity(&value) || !seen.insert(value))
}

fn probe_count_and_validity(
    plan: &ConditionalProbePlanV2,
    moderator_ids: &BTreeSet<String>,
) -> (usize, bool) {
    match plan {
        ConditionalProbePlanV2::Cartesian { axes } => {
            let mut axis_ids = BTreeSet::new();
            let mut valid = !axes.is_empty() && axes.len() <= CONDITIONAL_PROCESS_MAX_MODERATORS_V2;
            let mut count = 1_usize;
            for axis in axes {
                valid &= non_empty_identity(&axis.moderator_id)
                    && axis_ids.insert(axis.moderator_id.clone())
                    && moderator_ids.contains(&axis.moderator_id)
                    && !axis.standardized_values.is_empty()
                    && axis.standardized_values.len()
                        <= CONDITIONAL_PROCESS_MAX_VALUES_PER_MODERATOR_V2
                    && axis
                        .standardized_values
                        .iter()
                        .all(|value| value.is_finite());
                count = count.saturating_mul(axis.standardized_values.len());
            }
            valid &=
                axis_ids == *moderator_ids && count <= CONDITIONAL_PROCESS_MAX_CARTESIAN_PROBES_V2;
            (count, valid)
        }
        ConditionalProbePlanV2::Explicit { points } => {
            let mut point_ids = BTreeSet::new();
            let valid = !points.is_empty()
                && points.len() <= CONDITIONAL_PROCESS_MAX_EXPLICIT_PROBES_V2
                && points.iter().all(|point| {
                    non_empty_identity(&point.probe_id)
                        && point_ids.insert(point.probe_id.clone())
                        && point
                            .standardized_values
                            .keys()
                            .cloned()
                            .collect::<BTreeSet<_>>()
                            == *moderator_ids
                        && point
                            .standardized_values
                            .values()
                            .all(|value| value.is_finite())
                });
            (points.len(), valid)
        }
    }
}

/// Validates the deliberately non-Cartesian V2 qualification envelope.  The
/// caller receives every stable blocker in one pass and must not silently
/// simplify the requested model.
pub fn validate_conditional_process_profile_v2(
    request: &ConditionalProcessProfileRequestV2,
) -> Result<ConditionalProcessProfileReceiptV2, Vec<ConditionalProcessBlockerV2>> {
    let mut blockers = Vec::new();
    if duplicate_or_empty(
        request
            .interactions
            .iter()
            .map(|binding| binding.interaction_id.clone()),
    ) || duplicate_or_empty(
        request
            .selected_paths
            .iter()
            .map(|path| path.path_id.clone()),
    ) || duplicate_or_empty(request.moderator_ids.clone())
        || duplicate_or_empty(request.group_ids.clone())
        || duplicate_or_empty(request.hocs.iter().map(|hoc| hoc.hoc_id.clone()))
    {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::EmptyOrDuplicateIdentity,
            "all configured identities must be non-empty and unique within their family",
        );
    }

    let moderator_ids = request
        .moderator_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let (probe_point_count, probe_valid) =
        probe_count_and_validity(&request.probes, &moderator_ids);
    if !probe_valid {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::ProbePlanOutsideProfile,
            "joint probes must cover each declared moderator exactly once and remain within V2 caps",
        );
    }
    if request.moderator_ids.is_empty()
        || request.moderator_ids.len() > CONDITIONAL_PROCESS_MAX_MODERATORS_V2
    {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::ModeratorCountOutsideProfile,
            "V2 requires one to four moderators",
        );
    }
    if request.requested_probe_contrast_count > CONDITIONAL_PROCESS_MAX_PROBE_CONTRASTS_V2 {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::ProbeContrastLimitExceeded,
            "V2 supports at most 16 explicit probe contrasts",
        );
    }
    let conditional_cell_count = probe_point_count.saturating_mul(request.selected_paths.len());
    if conditional_cell_count > CONDITIONAL_PROCESS_MAX_CONDITIONAL_CELLS_V2 {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::ConditionalCellLimitExceeded,
            "the path-by-probe grid exceeds 512 conditional cells",
        );
    }
    if request.requested_inferential_target_count > CONDITIONAL_PROCESS_MAX_INFERENTIAL_TARGETS_V2 {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::InferentialTargetLimitExceeded,
            "the request exceeds 1,024 inferential targets",
        );
    }

    let interaction_count = request.interactions.len();
    let three_way_count = request
        .interactions
        .iter()
        .filter(|binding| binding.order == ConditionalInteractionOrderV2::ThreeWay)
        .count();
    let all_path_lengths_in = |minimum: usize, maximum: usize| {
        request
            .selected_paths
            .iter()
            .all(|path| (minimum..=maximum).contains(&path.edge_count))
    };
    let path_count = request.selected_paths.len();

    let expected_inference = match request.profile {
        ConditionalProcessProfileV2::MultiTwoWayPercentile
        | ConditionalProcessProfileV2::BoundedThreeWay
        | ConditionalProcessProfileV2::MultipleHoc
        | ConditionalProcessProfileV2::Grouped
        | ConditionalProcessProfileV2::CaseWeighted
        | ConditionalProcessProfileV2::FrequencyWeighted => {
            ConditionalInferenceMethodV2::Percentile
        }
        ConditionalProcessProfileV2::MultiTwoWayBca => ConditionalInferenceMethodV2::Bca,
        ConditionalProcessProfileV2::Studentized => ConditionalInferenceMethodV2::Studentized,
    };
    if request.inference_method != expected_inference {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::UnsupportedInferenceForProfile,
            "the requested interval method has not been qualified for this profile",
        );
    }
    if matches!(
        request.profile,
        ConditionalProcessProfileV2::MultipleHoc
            | ConditionalProcessProfileV2::Grouped
            | ConditionalProcessProfileV2::CaseWeighted
            | ConditionalProcessProfileV2::FrequencyWeighted
    ) && request.alternative != ConditionalAlternativeV2::TwoSided
    {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::UnsupportedAlternativeForProfile,
            "this profile is qualified only for two-sided inference",
        );
    }

    let (max_interactions, max_paths, min_edges, max_edges) = match request.profile {
        ConditionalProcessProfileV2::MultiTwoWayPercentile
        | ConditionalProcessProfileV2::MultiTwoWayBca => (8, 8, 2, 6),
        ConditionalProcessProfileV2::Studentized => (4, 2, 2, 4),
        ConditionalProcessProfileV2::BoundedThreeWay => (8, 4, 2, 5),
        ConditionalProcessProfileV2::MultipleHoc => (2, 8, 2, 6),
        ConditionalProcessProfileV2::Grouped
        | ConditionalProcessProfileV2::CaseWeighted
        | ConditionalProcessProfileV2::FrequencyWeighted => (4, 4, 2, 4),
    };
    if interaction_count == 0 || interaction_count > max_interactions {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::InteractionCountOutsideProfile,
            format!("profile permits one to {max_interactions} interactions"),
        );
    }
    if path_count == 0 || path_count > max_paths {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::PathCountOutsideProfile,
            format!("profile permits one to {max_paths} explicitly selected paths"),
        );
    }
    if !all_path_lengths_in(min_edges, max_edges) {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::PathLengthOutsideProfile,
            format!("profile permits path lengths from {min_edges} to {max_edges} edges"),
        );
    }

    let expected_weight_mode = match request.profile {
        ConditionalProcessProfileV2::CaseWeighted => ConditionalWeightModeV2::PositiveCase,
        ConditionalProcessProfileV2::FrequencyWeighted => {
            ConditionalWeightModeV2::PositiveIntegerFrequency
        }
        _ => ConditionalWeightModeV2::None,
    };
    if request.weight_mode != expected_weight_mode {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::WeightModeOutsideProfile,
            "weights are admitted only by their dedicated profile and cannot be combined",
        );
    }
    let expected_group_range = matches!(request.profile, ConditionalProcessProfileV2::Grouped);
    if (expected_group_range && !(2..=20).contains(&request.group_ids.len()))
        || (!expected_group_range && !request.group_ids.is_empty())
    {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::GroupCountOutsideProfile,
            "only the grouped profile admits two to twenty groups",
        );
    }

    match request.profile {
        ConditionalProcessProfileV2::BoundedThreeWay => {
            if three_way_count != 1 || !request.three_way_lower_order_closure_complete {
                push_blocker(
                    &mut blockers,
                    ConditionalProcessBlockerCodeV2::ThreeWayOutsideProfile,
                    "bounded-three-way requires exactly one three-way interaction with verified lower-order closure",
                );
            }
        }
        _ if three_way_count != 0 => push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::ThreeWayOutsideProfile,
            "three-way terms are admitted only by the bounded-three-way profile",
        ),
        _ => {}
    }

    if request.profile == ConditionalProcessProfileV2::MultipleHoc {
        if request.hocs.is_empty() || request.hocs.len() > 4 {
            push_blocker(
                &mut blockers,
                ConditionalProcessBlockerCodeV2::HocOutsideProfile,
                "multiple-HOC requires one to four HOCs",
            );
        }
        let mut members = BTreeSet::new();
        if request.hocs.iter().any(|hoc| {
            hoc.nested
                || hoc.member_construct_ids.is_empty()
                || hoc
                    .member_construct_ids
                    .iter()
                    .any(|member| !non_empty_identity(member) || !members.insert(member.clone()))
        }) {
            push_blocker(
                &mut blockers,
                ConditionalProcessBlockerCodeV2::HocNestedOrOverlapping,
                "HOCs must be nonnested and pairwise disjoint",
            );
        }
        if let Some(first) = request.hocs.first() {
            if request
                .hocs
                .iter()
                .any(|hoc| hoc.approach != first.approach)
            {
                push_blocker(
                    &mut blockers,
                    ConditionalProcessBlockerCodeV2::HocApproachMismatch,
                    "one HOC approach must be used consistently within a run",
                );
            }
        }
    } else if !request.hocs.is_empty() {
        push_blocker(
            &mut blockers,
            ConditionalProcessBlockerCodeV2::HocOutsideProfile,
            "HOCs are admitted only by the multiple-HOC profile",
        );
    }

    if request.profile == ConditionalProcessProfileV2::Studentized {
        if request.moderator_ids.len() > 3 || probe_point_count > 27 {
            push_blocker(
                &mut blockers,
                ConditionalProcessBlockerCodeV2::ProbePlanOutsideProfile,
                "studentized inference supports at most three moderators and 27 probes",
            );
        }
        if request.requested_inferential_target_count > 256 {
            push_blocker(
                &mut blockers,
                ConditionalProcessBlockerCodeV2::InferentialTargetLimitExceeded,
                "studentized inference supports at most 256 targets",
            );
        }
    }

    if blockers.is_empty() {
        Ok(ConditionalProcessProfileReceiptV2 {
            method_version: GENERAL_SEM_CONDITIONAL_PROCESS_METHOD_VERSION_V2.to_owned(),
            profile: request.profile,
            probe_point_count,
            conditional_cell_count,
            requested_inferential_target_count: request.requested_inferential_target_count,
        })
    } else {
        Err(blockers)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResamplingIntervalV2 {
    pub method: ConditionalInferenceMethodV2,
    pub alternative: ConditionalAlternativeV2,
    pub confidence: f64,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub usable_replicates: usize,
    pub requested_replicates: usize,
}

pub fn minimum_usable_resamples_v2(requested_replicates: usize) -> usize {
    ((requested_replicates as f64) * CONDITIONAL_PROCESS_MIN_USABLE_RESAMPLE_FRACTION_V2).ceil()
        as usize
}

fn ensure_usable_resamples(
    usable_replicates: usize,
    requested_replicates: usize,
) -> Result<(), ConditionalProcessMathErrorV2> {
    let required = minimum_usable_resamples_v2(requested_replicates);
    if requested_replicates == 0 || usable_replicates < required {
        Err(ConditionalProcessMathErrorV2::InsufficientUsableResamples {
            usable: usable_replicates,
            required,
        })
    } else {
        Ok(())
    }
}

pub fn type7_quantile_v2(
    values: &[f64],
    probability: f64,
) -> Result<f64, ConditionalProcessMathErrorV2> {
    if values.is_empty()
        || values.iter().any(|value| !value.is_finite())
        || !probability.is_finite()
        || !(0.0..=1.0).contains(&probability)
    {
        return Err(ConditionalProcessMathErrorV2::InvalidResamplingValues);
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return Ok(sorted[0]);
    }
    let h = (sorted.len() - 1) as f64 * probability;
    let lower_index = h.floor() as usize;
    let upper_index = h.ceil() as usize;
    let fraction = h - lower_index as f64;
    Ok(sorted[lower_index] + fraction * (sorted[upper_index] - sorted[lower_index]))
}

fn tail_probabilities(
    confidence: f64,
    alternative: ConditionalAlternativeV2,
) -> Result<(Option<f64>, Option<f64>), ConditionalProcessMathErrorV2> {
    if !confidence.is_finite() || !(0.0..1.0).contains(&confidence) {
        return Err(ConditionalProcessMathErrorV2::InvalidConfidence);
    }
    let alpha = 1.0 - confidence;
    Ok(match alternative {
        ConditionalAlternativeV2::TwoSided => (Some(alpha / 2.0), Some(1.0 - alpha / 2.0)),
        ConditionalAlternativeV2::Less => (None, Some(confidence)),
        ConditionalAlternativeV2::Greater => (Some(1.0 - confidence), None),
    })
}

pub fn percentile_interval_v2(
    replicates: &[f64],
    requested_replicates: usize,
    confidence: f64,
    alternative: ConditionalAlternativeV2,
) -> Result<ResamplingIntervalV2, ConditionalProcessMathErrorV2> {
    ensure_usable_resamples(replicates.len(), requested_replicates)?;
    let (lower_probability, upper_probability) = tail_probabilities(confidence, alternative)?;
    let lower = lower_probability
        .map(|probability| type7_quantile_v2(replicates, probability))
        .transpose()?;
    let upper = upper_probability
        .map(|probability| type7_quantile_v2(replicates, probability))
        .transpose()?;
    Ok(ResamplingIntervalV2 {
        method: ConditionalInferenceMethodV2::Percentile,
        alternative,
        confidence,
        lower,
        upper,
        usable_replicates: replicates.len(),
        requested_replicates,
    })
}

fn bca_adjusted_probability(
    probability: f64,
    bias_correction: f64,
    acceleration: f64,
    normal: &Normal,
) -> f64 {
    let z_alpha = normal.inverse_cdf(probability);
    let numerator = bias_correction + z_alpha;
    let denominator = 1.0 - acceleration * numerator;
    let transformed = bias_correction + numerator / denominator;
    normal.cdf(transformed).clamp(0.0, 1.0)
}

pub fn bca_interval_v2(
    observed_estimate: f64,
    bootstrap_replicates: &[f64],
    delete_one_jackknife_estimates: &[f64],
    requested_replicates: usize,
    confidence: f64,
    alternative: ConditionalAlternativeV2,
) -> Result<ResamplingIntervalV2, ConditionalProcessMathErrorV2> {
    ensure_usable_resamples(bootstrap_replicates.len(), requested_replicates)?;
    if !observed_estimate.is_finite()
        || bootstrap_replicates.is_empty()
        || delete_one_jackknife_estimates.len() < 2
        || bootstrap_replicates.iter().any(|value| !value.is_finite())
        || delete_one_jackknife_estimates
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err(ConditionalProcessMathErrorV2::InvalidResamplingValues);
    }
    let (lower_probability, upper_probability) = tail_probabilities(confidence, alternative)?;
    let normal = Normal::new(0.0, 1.0)
        .map_err(|_| ConditionalProcessMathErrorV2::InvalidResamplingValues)?;
    let strictly_below = bootstrap_replicates
        .iter()
        .filter(|value| **value < observed_estimate)
        .count() as f64;
    let ties = bootstrap_replicates
        .iter()
        .filter(|value| **value == observed_estimate)
        .count() as f64;
    let rank_fraction = (strictly_below + 0.5 * ties) / bootstrap_replicates.len() as f64;
    let finite_floor = 0.5 / bootstrap_replicates.len() as f64;
    let bias_correction = normal.inverse_cdf(rank_fraction.clamp(finite_floor, 1.0 - finite_floor));

    let jackknife_mean = delete_one_jackknife_estimates.iter().sum::<f64>()
        / delete_one_jackknife_estimates.len() as f64;
    let centered = delete_one_jackknife_estimates
        .iter()
        .map(|estimate| jackknife_mean - estimate)
        .collect::<Vec<_>>();
    let numerator = centered.iter().map(|value| value.powi(3)).sum::<f64>();
    let denominator_base = centered.iter().map(|value| value.powi(2)).sum::<f64>();
    let acceleration = if denominator_base == 0.0 {
        0.0
    } else {
        numerator / (6.0 * denominator_base.powf(1.5))
    };

    let lower = lower_probability
        .map(|probability| {
            type7_quantile_v2(
                bootstrap_replicates,
                bca_adjusted_probability(probability, bias_correction, acceleration, &normal),
            )
        })
        .transpose()?;
    let upper = upper_probability
        .map(|probability| {
            type7_quantile_v2(
                bootstrap_replicates,
                bca_adjusted_probability(probability, bias_correction, acceleration, &normal),
            )
        })
        .transpose()?;
    Ok(ResamplingIntervalV2 {
        method: ConditionalInferenceMethodV2::Bca,
        alternative,
        confidence,
        lower,
        upper,
        usable_replicates: bootstrap_replicates.len(),
        requested_replicates,
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StudentizedOuterReplicateV2 {
    pub estimate: f64,
    pub standard_error: f64,
}

pub fn studentized_interval_v2(
    observed_estimate: f64,
    observed_standard_error: f64,
    outer_replicates: &[StudentizedOuterReplicateV2],
    requested_outer_replicates: usize,
    confidence: f64,
    alternative: ConditionalAlternativeV2,
) -> Result<ResamplingIntervalV2, ConditionalProcessMathErrorV2> {
    ensure_usable_resamples(outer_replicates.len(), requested_outer_replicates)?;
    if !observed_estimate.is_finite()
        || !observed_standard_error.is_finite()
        || observed_standard_error <= 0.0
        || outer_replicates.is_empty()
        || outer_replicates.iter().any(|replicate| {
            !replicate.estimate.is_finite()
                || !replicate.standard_error.is_finite()
                || replicate.standard_error <= 0.0
        })
    {
        return Err(ConditionalProcessMathErrorV2::InvalidStudentizedStandardError);
    }
    // Validate confidence before constructing the pivot-tail probabilities.
    tail_probabilities(confidence, alternative)?;
    let pivots = outer_replicates
        .iter()
        .map(|replicate| (replicate.estimate - observed_estimate) / replicate.standard_error)
        .collect::<Vec<_>>();
    // Pivot inversion reverses the quantile used for each parameter bound.
    let alpha = 1.0 - confidence;
    let (lower, upper) = match alternative {
        ConditionalAlternativeV2::TwoSided => (
            Some(
                observed_estimate
                    - type7_quantile_v2(&pivots, 1.0 - alpha / 2.0)? * observed_standard_error,
            ),
            Some(
                observed_estimate
                    - type7_quantile_v2(&pivots, alpha / 2.0)? * observed_standard_error,
            ),
        ),
        ConditionalAlternativeV2::Less => (
            None,
            Some(observed_estimate - type7_quantile_v2(&pivots, alpha)? * observed_standard_error),
        ),
        ConditionalAlternativeV2::Greater => (
            Some(
                observed_estimate
                    - type7_quantile_v2(&pivots, 1.0 - alpha)? * observed_standard_error,
            ),
            None,
        ),
    };
    Ok(ResamplingIntervalV2 {
        method: ConditionalInferenceMethodV2::Studentized,
        alternative,
        confidence,
        lower,
        upper,
        usable_replicates: outer_replicates.len(),
        requested_replicates: requested_outer_replicates,
    })
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ConditionalWeightErrorV2 {
    #[error("weights must be non-empty and match the value count")]
    Shape,
    #[error("case weights must be finite and strictly positive")]
    InvalidCaseWeight,
    #[error("normalized case-weight max/min ratio exceeds 1e6")]
    CaseWeightRatioExceeded,
    #[error("frequency weights must be positive integers")]
    InvalidFrequencyWeight,
    #[error("frequency total exceeds the exact f64 integer bound 2^53")]
    FrequencyTotalExceeded,
    #[error("weighted result is non-finite")]
    NonFiniteResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaseWeightReceiptV2 {
    pub normalized_weights: Vec<f64>,
    pub raw_mean: f64,
    pub kish_effective_sample_size: f64,
    pub effective_sample_size_below_twenty_five_percent: bool,
    pub normalized_max_to_min_ratio: f64,
}

pub fn normalize_positive_case_weights_v2(
    weights: &[f64],
) -> Result<CaseWeightReceiptV2, ConditionalWeightErrorV2> {
    if weights.is_empty() {
        return Err(ConditionalWeightErrorV2::Shape);
    }
    if weights
        .iter()
        .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        return Err(ConditionalWeightErrorV2::InvalidCaseWeight);
    }
    let sum = weights.iter().sum::<f64>();
    let raw_mean = sum / weights.len() as f64;
    if !raw_mean.is_finite() || raw_mean <= 0.0 {
        return Err(ConditionalWeightErrorV2::InvalidCaseWeight);
    }
    let normalized_weights = weights
        .iter()
        .map(|weight| weight / raw_mean)
        .collect::<Vec<_>>();
    let minimum = normalized_weights
        .iter()
        .copied()
        .min_by(f64::total_cmp)
        .ok_or(ConditionalWeightErrorV2::Shape)?;
    let maximum = normalized_weights
        .iter()
        .copied()
        .max_by(f64::total_cmp)
        .ok_or(ConditionalWeightErrorV2::Shape)?;
    let ratio = maximum / minimum;
    if ratio > CONDITIONAL_PROCESS_CASE_WEIGHT_RATIO_LIMIT_V2 {
        return Err(ConditionalWeightErrorV2::CaseWeightRatioExceeded);
    }
    let squared_sum = weights.iter().map(|weight| weight * weight).sum::<f64>();
    if !squared_sum.is_finite() || squared_sum <= 0.0 {
        return Err(ConditionalWeightErrorV2::InvalidCaseWeight);
    }
    let kish_effective_sample_size = sum * sum / squared_sum;
    Ok(CaseWeightReceiptV2 {
        normalized_weights,
        raw_mean,
        kish_effective_sample_size,
        effective_sample_size_below_twenty_five_percent: kish_effective_sample_size
            < 0.25 * weights.len() as f64,
        normalized_max_to_min_ratio: ratio,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrequencyWeightReceiptV2 {
    pub total_expanded_count: u64,
    pub multinomial_probabilities: Vec<f64>,
}

pub fn validate_positive_frequency_weights_v2(
    frequencies: &[u64],
) -> Result<FrequencyWeightReceiptV2, ConditionalWeightErrorV2> {
    if frequencies.is_empty() || frequencies.contains(&0) {
        return Err(ConditionalWeightErrorV2::InvalidFrequencyWeight);
    }
    let total = frequencies.iter().try_fold(0_u64, |total, frequency| {
        total
            .checked_add(*frequency)
            .ok_or(ConditionalWeightErrorV2::FrequencyTotalExceeded)
    })?;
    if total > CONDITIONAL_PROCESS_MAX_EXACT_FREQUENCY_TOTAL_V2 {
        return Err(ConditionalWeightErrorV2::FrequencyTotalExceeded);
    }
    Ok(FrequencyWeightReceiptV2 {
        total_expanded_count: total,
        multinomial_probabilities: frequencies
            .iter()
            .map(|frequency| *frequency as f64 / total as f64)
            .collect(),
    })
}

pub fn case_weighted_mean_v2(
    values: &[f64],
    weights: &[f64],
) -> Result<f64, ConditionalWeightErrorV2> {
    if values.len() != weights.len() || values.is_empty() {
        return Err(ConditionalWeightErrorV2::Shape);
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(ConditionalWeightErrorV2::NonFiniteResult);
    }
    let receipt = normalize_positive_case_weights_v2(weights)?;
    let result = values
        .iter()
        .zip(&receipt.normalized_weights)
        .map(|(value, weight)| value * weight)
        .sum::<f64>()
        / receipt.normalized_weights.iter().sum::<f64>();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ConditionalWeightErrorV2::NonFiniteResult)
    }
}

/// Computes the same mean as physically expanding each row `frequency` times,
/// without allocating the expanded data.
pub fn frequency_weighted_mean_v2(
    values: &[f64],
    frequencies: &[u64],
) -> Result<f64, ConditionalWeightErrorV2> {
    if values.len() != frequencies.len() || values.is_empty() {
        return Err(ConditionalWeightErrorV2::Shape);
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err(ConditionalWeightErrorV2::NonFiniteResult);
    }
    let receipt = validate_positive_frequency_weights_v2(frequencies)?;
    let numerator = values
        .iter()
        .zip(frequencies)
        .map(|(value, frequency)| value * *frequency as f64)
        .sum::<f64>();
    let result = numerator / receipt.total_expanded_count as f64;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ConditionalWeightErrorV2::NonFiniteResult)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn edge(
        relation_id: &str,
        source_id: &str,
        target_id: &str,
        intercept: f64,
        linear: &[(&str, f64)],
        pairwise: &[(&str, &str, f64)],
    ) -> ConditionalEdgeFunctionV2 {
        ConditionalEdgeFunctionV2 {
            relation_id: relation_id.to_owned(),
            source_id: source_id.to_owned(),
            target_id: target_id.to_owned(),
            intercept,
            linear_coefficients: linear
                .iter()
                .map(|(moderator_id, estimate)| ConditionalLinearCoefficientV2 {
                    moderator_id: (*moderator_id).to_owned(),
                    estimate: *estimate,
                })
                .collect(),
            pairwise_coefficients: pairwise
                .iter()
                .map(
                    |(first, second, estimate)| ConditionalPairwiseCoefficientV2 {
                        first_moderator_id: (*first).to_owned(),
                        second_moderator_id: (*second).to_owned(),
                        estimate: *estimate,
                    },
                )
                .collect(),
        }
    }

    fn probe(id: &str, z: f64, w: f64) -> ConditionalProbePointV2 {
        ConditionalProbePointV2 {
            probe_id: id.to_owned(),
            standardized_values: BTreeMap::from([("w".to_owned(), w), ("z".to_owned(), z)]),
        }
    }

    #[test]
    fn explicit_path_product_conditional_effects_and_derivatives_are_exact() {
        // (2 + 3z + 5zw)(7 + 11w)
        let path = ExplicitConditionalPathV2 {
            path_id: "x_m_y".to_owned(),
            edges: vec![
                edge("x_m", "x", "m", 2.0, &[("z", 3.0)], &[("z", "w", 5.0)]),
                edge("m_y", "m", "y", 7.0, &[("w", 11.0)], &[]),
            ],
        };
        let polynomial = compile_explicit_conditional_path_v2(&path).unwrap();
        let at = probe("z2_w3", 2.0, 3.0);
        let effect = conditional_effect_v2(&polynomial, &at).unwrap();
        assert_abs_diff_eq!(effect.estimate, 38.0 * 40.0, epsilon = 1e-12);

        let derivatives = conditional_derivatives_v2(&polynomial, &at).unwrap();
        let first_z = derivatives
            .iter()
            .find(|value| {
                value.kind == ConditionalDerivativeKindV2::First && value.first_moderator_id == "z"
            })
            .unwrap();
        // d/dz = (3 + 5w)(7 + 11w)
        assert_abs_diff_eq!(first_z.estimate, 18.0 * 40.0, epsilon = 1e-12);
        let cross = derivatives
            .iter()
            .find(|value| value.kind == ConditionalDerivativeKindV2::Cross)
            .unwrap();
        // d2/(dw dz) = 5(7+11w) + 11(3+5w)
        assert_abs_diff_eq!(cross.estimate, 398.0, epsilon = 1e-12);
        let second_w = derivatives
            .iter()
            .find(|value| {
                value.kind == ConditionalDerivativeKindV2::Second && value.first_moderator_id == "w"
            })
            .unwrap();
        // Only 55*z*w^2 contributes: d2/dw2 = 110*z.
        assert_abs_diff_eq!(second_w.estimate, 220.0, epsilon = 1e-12);

        let reversed = probe("z0_w0", 0.0, 0.0);
        let contrast = conditional_probe_contrast_v2(&polynomial, &at, &reversed).unwrap();
        assert_abs_diff_eq!(contrast.estimate, 1_506.0, epsilon = 1e-12);
    }

    #[test]
    fn scalar_index_is_emitted_only_for_an_affine_one_moderator_effect() {
        let affine = compile_explicit_conditional_path_v2(&ExplicitConditionalPathV2 {
            path_id: "affine".to_owned(),
            edges: vec![
                edge("x_m", "x", "m", 2.0, &[("z", 3.0)], &[]),
                edge("m_y", "m", "y", 5.0, &[], &[]),
            ],
        })
        .unwrap();
        assert_abs_diff_eq!(
            scalar_index_of_moderated_mediation_v2(&affine, "z")
                .unwrap()
                .estimate,
            15.0,
            epsilon = 1e-12
        );

        let nonlinear = compile_explicit_conditional_path_v2(&ExplicitConditionalPathV2 {
            path_id: "nonlinear".to_owned(),
            edges: vec![
                edge("x_m", "x", "m", 2.0, &[("z", 3.0)], &[]),
                edge("m_y", "m", "y", 5.0, &[("z", 7.0)], &[]),
            ],
        })
        .unwrap();
        assert_eq!(
            scalar_index_of_moderated_mediation_v2(&nonlinear, "z"),
            Err(ConditionalProcessMathErrorV2::ScalarIndexNotAffine)
        );
    }

    #[test]
    fn target_ids_are_canonical_and_change_with_the_probe() {
        let polynomial = compile_explicit_conditional_path_v2(&ExplicitConditionalPathV2 {
            path_id: "stable".to_owned(),
            edges: vec![edge("x_y", "x", "y", 1.0, &[("z", 2.0), ("w", 3.0)], &[])],
        })
        .unwrap();
        let first = conditional_effect_v2(&polynomial, &probe("one", 1.0, 2.0)).unwrap();
        let same = conditional_effect_v2(&polynomial, &probe("one", 1.0, 2.0)).unwrap();
        let changed = conditional_effect_v2(&polynomial, &probe("two", 1.0, 2.0)).unwrap();
        assert_eq!(first.target_id, same.target_id);
        assert_ne!(first.target_id, changed.target_id);
    }

    #[test]
    fn profile_validation_fails_closed_for_cross_profile_combinations() {
        let request = ConditionalProcessProfileRequestV2 {
            profile: ConditionalProcessProfileV2::Grouped,
            inference_method: ConditionalInferenceMethodV2::Percentile,
            alternative: ConditionalAlternativeV2::TwoSided,
            interactions: vec![ConditionalInteractionBindingV2 {
                interaction_id: "x_z".to_owned(),
                order: ConditionalInteractionOrderV2::TwoWay,
            }],
            three_way_lower_order_closure_complete: false,
            selected_paths: vec![ConditionalPathShapeV2 {
                path_id: "x_m_y".to_owned(),
                edge_count: 2,
            }],
            moderator_ids: vec!["z".to_owned()],
            hocs: Vec::new(),
            group_ids: vec!["a".to_owned(), "b".to_owned()],
            weight_mode: ConditionalWeightModeV2::PositiveCase,
            probes: ConditionalProbePlanV2::Cartesian {
                axes: vec![ConditionalProbeAxisV2 {
                    moderator_id: "z".to_owned(),
                    standardized_values: vec![-1.0, 0.0, 1.0],
                }],
            },
            requested_probe_contrast_count: 1,
            requested_inferential_target_count: 3,
        };
        let blockers = validate_conditional_process_profile_v2(&request).unwrap_err();
        assert!(blockers.iter().any(|blocker| {
            blocker.code == ConditionalProcessBlockerCodeV2::WeightModeOutsideProfile
        }));

        let mut valid = request.clone();
        valid.weight_mode = ConditionalWeightModeV2::None;
        let receipt = validate_conditional_process_profile_v2(&valid).unwrap();
        assert_eq!(receipt.probe_point_count, 3);
        assert_eq!(receipt.conditional_cell_count, 3);

        valid.requested_inferential_target_count =
            CONDITIONAL_PROCESS_MAX_INFERENTIAL_TARGETS_V2 + 1;
        let cap_blockers = validate_conditional_process_profile_v2(&valid).unwrap_err();
        assert!(cap_blockers.iter().any(|blocker| {
            blocker.code == ConditionalProcessBlockerCodeV2::InferentialTargetLimitExceeded
        }));
    }

    #[test]
    fn type7_bca_and_studentized_helpers_have_expected_invariants() {
        let values = [0.0, 10.0, 20.0, 30.0, 40.0];
        assert_abs_diff_eq!(type7_quantile_v2(&values, 0.25).unwrap(), 10.0);
        let percentile =
            percentile_interval_v2(&values, 5, 0.80, ConditionalAlternativeV2::TwoSided).unwrap();
        assert!(percentile.lower.unwrap() < percentile.upper.unwrap());

        let bca = bca_interval_v2(
            20.0,
            &values,
            &[18.0, 19.0, 20.0, 21.0, 22.0],
            5,
            0.80,
            ConditionalAlternativeV2::TwoSided,
        )
        .unwrap();
        assert!(bca.lower.unwrap().is_finite());
        assert!(bca.upper.unwrap().is_finite());

        let outer = [-2.0, -1.0, 0.0, 1.0, 2.0]
            .into_iter()
            .map(|pivot| StudentizedOuterReplicateV2 {
                estimate: 5.0 + pivot,
                standard_error: 1.0,
            })
            .collect::<Vec<_>>();
        let studentized = studentized_interval_v2(
            5.0,
            1.0,
            &outer,
            5,
            0.80,
            ConditionalAlternativeV2::TwoSided,
        )
        .unwrap();
        assert_abs_diff_eq!(studentized.lower.unwrap(), 3.4, epsilon = 1e-12);
        assert_abs_diff_eq!(studentized.upper.unwrap(), 6.6, epsilon = 1e-12);
    }

    #[test]
    fn case_and_frequency_weights_preserve_declared_semantics() {
        let case = normalize_positive_case_weights_v2(&[1.0, 2.0, 3.0]).unwrap();
        assert_abs_diff_eq!(case.normalized_weights.iter().sum::<f64>(), 3.0);
        assert_abs_diff_eq!(
            case_weighted_mean_v2(&[2.0, 4.0, 8.0], &[1.0, 2.0, 3.0]).unwrap(),
            34.0 / 6.0,
            epsilon = 1e-12
        );

        let compact = frequency_weighted_mean_v2(&[2.0, 10.0], &[3, 2]).unwrap();
        let expanded = [2.0, 2.0, 2.0, 10.0, 10.0].iter().sum::<f64>() / 5.0;
        assert_abs_diff_eq!(compact, expanded, epsilon = 1e-12);
        let receipt = validate_positive_frequency_weights_v2(&[3, 2]).unwrap();
        assert_eq!(receipt.total_expanded_count, 5);
        assert_abs_diff_eq!(receipt.multinomial_probabilities.iter().sum::<f64>(), 1.0);
    }
}
