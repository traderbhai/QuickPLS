//! Additive V2 kernels for unobserved heterogeneity analysis.
//!
//! This module deliberately has no dependency on the legacy FIMIX/PLS-POS
//! payloads in `pls.rs`.  Its public boundary is either (a) a set of pooled,
//! standardized structural equations or (b) a full-refit callback supplied by
//! the PLS compiler.  Keeping those boundaries explicit prevents the V1
//! score-space previews from being silently reinterpreted as the V2 methods.

use qpls_core::FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2;
use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIMIX_PLS_METHOD_VERSION_V2: &str = "qpls.fimix-pls.v2";
pub const PLS_POS_PUBLISHED_METHOD_VERSION_V2: &str = "qpls.pls-pos.published.v2";
pub const PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2: &str =
    "qpls.pls-pos.destination-scored-interactions.v2";
pub const POS_COMMON_METRIC_COMPARABILITY_METHOD_VERSION_V1: &str =
    "qpls.pos-common-metric-comparability.v1";
pub const HETEROGENEITY_BOOTSTRAP_LEDGER_METHOD_VERSION_V2: &str =
    "qpls.heterogeneity-bootstrap-ledger.v2";
pub const POOLED_STRUCTURAL_BASELINE_METHOD_VERSION_V2: &str =
    "qpls.heterogeneity-pooled-structural-baseline.v2";
pub const HETEROGENEITY_TARGET_PAYLOAD_DIGEST_DOMAIN_V2: &[u8] =
    b"quickpls:heterogeneity:target-payload:v2\0";
pub const HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2: u32 = 1;
pub const HETEROGENEITY_MULTISTART_PARTITION_DIGEST_DOMAIN_V2: &[u8] =
    b"quickpls:heterogeneity:multistart-partition:v2\0";
pub const HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2: &[u8] =
    b"quickpls:heterogeneity:multistart-coefficients:v2\0";
pub const HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2: &[u8] =
    b"quickpls:heterogeneity:multistart-posteriors:v2\0";
pub const HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2: &[u8] =
    b"quickpls:heterogeneity:multistart-parameters:v2\0";
pub const HETEROGENEITY_MULTISTART_FIT_STATISTIC_DIGEST_DOMAIN_V2: &[u8] =
    b"quickpls:heterogeneity:multistart-fit-statistic:v2\0";
pub const POS_STANDARDIZED_OUTCOME_MEAN_TOLERANCE_V2: f64 = 1.0e-8;

const LOG_TWO_PI: f64 = 1.8378770664093453;
const MAX_CLASSES_OR_SEGMENTS: usize = 5;
const MIN_CLASSES_OR_SEGMENTS: usize = 2;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityInteractionProfileV2 {
    P0Structural,
    P2MultiTwoWay,
    P23AllCurrent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PooledStandardizedMetricReceiptV2 {
    pub metric_id: String,
    pub source_sha256: String,
    pub observation_count: usize,
    pub scores_standardized_once_on_pooled_rows: bool,
    pub products_standardized_once_on_pooled_rows: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StandardizedStructuralEquationV2 {
    pub equation_id: String,
    pub outcome_id: String,
    pub predictor_ids: Vec<String>,
    /// Row-major predictor matrix. An intercept, when requested, is generated
    /// by the kernel and must not be included as a predictor column.
    pub design: Vec<Vec<f64>>,
    pub outcome: Vec<f64>,
    pub include_intercept: bool,
}

impl StandardizedStructuralEquationV2 {
    fn coefficient_count(&self) -> usize {
        self.predictor_ids.len() + usize::from(self.include_intercept)
    }

    fn coefficient_ids(&self) -> Vec<String> {
        let mut ids = Vec::with_capacity(self.coefficient_count());
        if self.include_intercept {
            ids.push("(intercept)".to_string());
        }
        ids.extend(self.predictor_ids.iter().cloned());
        ids
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StandardizedFimixInputV2 {
    pub interaction_profile: HeterogeneityInteractionProfileV2,
    pub metric: PooledStandardizedMetricReceiptV2,
    pub equations: Vec<StandardizedStructuralEquationV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PooledStructuralBaselineEquationV2 {
    pub equation_id: String,
    pub outcome_id: String,
    pub coefficients: Vec<FimixCoefficientV2>,
    pub residual_variance: f64,
    pub r_squared: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PooledStructuralBaselineV2 {
    pub method_version: String,
    pub metric_source_sha256: String,
    pub observations: usize,
    pub equations: Vec<PooledStructuralBaselineEquationV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixPlsV2Config {
    pub classes: usize,
    pub starts: usize,
    pub seed: u64,
    pub max_iterations: usize,
    pub relative_log_likelihood_tolerance: f64,
    pub consecutive_stable_iterations: usize,
    pub likelihood_decrease_tolerance: f64,
    pub residual_variance_floor: f64,
    pub rank_tolerance: f64,
    pub minimum_class_share: f64,
    pub minimum_class_observations: usize,
    pub required_reproducing_starts: usize,
    pub optimum_relative_log_likelihood_tolerance: f64,
    pub optimum_maximum_coefficient_difference: f64,
    pub optimum_mean_posterior_difference: f64,
    pub standardized_mean_tolerance: f64,
    pub standardized_sample_sd_tolerance: f64,
}

impl FimixPlsV2Config {
    pub fn for_classes(classes: usize) -> Self {
        Self {
            classes,
            starts: 30,
            seed: 42,
            max_iterations: 5_000,
            relative_log_likelihood_tolerance: 1.0e-10,
            consecutive_stable_iterations: 3,
            likelihood_decrease_tolerance: FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2,
            residual_variance_floor: 1.0e-8,
            rank_tolerance: 1.0e-11,
            minimum_class_share: 0.05,
            minimum_class_observations: 20,
            required_reproducing_starts: 2,
            optimum_relative_log_likelihood_tolerance: 1.0e-8,
            optimum_maximum_coefficient_difference: 1.0e-6,
            optimum_mean_posterior_difference: 1.0e-4,
            standardized_mean_tolerance: 1.0e-8,
            standardized_sample_sd_tolerance: 1.0e-8,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FimixStartFailureCodeV2 {
    CollapsedClass,
    VarianceCollapse,
    RankDeficient,
    NonFinite,
    LikelihoodDecrease,
    MaximumIterations,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixIterationTraceV2 {
    pub iteration: usize,
    pub log_likelihood: f64,
    pub relative_change: Option<f64>,
    pub minimum_effective_class_size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixStartDiagnosticV2 {
    pub start_index: usize,
    pub start_seed: u64,
    pub converged: bool,
    pub iterations: usize,
    pub final_log_likelihood: Option<f64>,
    pub maximum_likelihood_decrease: f64,
    pub final_effective_class_sizes: Vec<f64>,
    pub failure_code: Option<FimixStartFailureCodeV2>,
    pub failure_message: Option<String>,
    pub trace: Vec<FimixIterationTraceV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixCoefficientV2 {
    pub parameter_id: String,
    pub estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixEquationEstimateV2 {
    pub equation_id: String,
    pub outcome_id: String,
    pub coefficients: Vec<FimixCoefficientV2>,
    pub residual_variance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixClassEstimateV2 {
    pub class_id: String,
    pub proportion: f64,
    pub effective_observations: f64,
    pub equations: Vec<FimixEquationEstimateV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixInformationCriteriaV2 {
    pub parameter_count: usize,
    pub aic: f64,
    pub aic3: f64,
    pub aic4: f64,
    pub bic: f64,
    pub caic: f64,
    pub hq: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixEntropyV2 {
    pub raw: f64,
    /// Classification certainty: `1 - raw / (n * ln(K))`.
    pub normalized_certainty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixMultistartStabilityV2 {
    pub required_reproducing_starts: usize,
    pub reproducing_start_indices: Vec<usize>,
    pub maximum_aligned_coefficient_difference: f64,
    pub maximum_aligned_mean_posterior_difference: f64,
    pub stable: bool,
}

/// Validation-only receipt for one completed FIMIX start. The retained values
/// are exactly those required to replay the three multistart tolerances after
/// exhaustive label alignment; the digests bind their order and dimensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixCompletedStartEvidenceV2 {
    pub start_index: usize,
    pub final_log_likelihood: f64,
    pub canonical_hard_assignments: Vec<usize>,
    pub canonical_coefficient_signatures: Vec<Vec<f64>>,
    pub canonical_posteriors: Vec<Vec<f64>>,
    pub partition_sha256: String,
    pub coefficient_sha256: String,
    pub posterior_sha256: String,
    pub fit_statistic_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixMultistartEvidenceV2 {
    pub schema_version: u32,
    pub selected_start_index: usize,
    pub observations: usize,
    pub classes: usize,
    pub required_reproducing_starts: usize,
    pub relative_log_likelihood_tolerance: f64,
    pub maximum_coefficient_difference_tolerance: f64,
    pub mean_posterior_difference_tolerance: f64,
    pub completed_starts: Vec<FimixCompletedStartEvidenceV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FimixPlsV2Result {
    pub method_version: String,
    pub interaction_profile: HeterogeneityInteractionProfileV2,
    pub metric: PooledStandardizedMetricReceiptV2,
    pub observations: usize,
    pub selected_start_index: usize,
    pub iterations: usize,
    pub log_likelihood: f64,
    pub classes: Vec<FimixClassEstimateV2>,
    /// Complete N by K posterior-probability matrix.
    pub posteriors: Vec<Vec<f64>>,
    /// Zero-based canonical class labels.
    pub hard_assignments: Vec<usize>,
    pub criteria: FimixInformationCriteriaV2,
    pub entropy: FimixEntropyV2,
    pub minimum_effective_class_size: usize,
    pub starts: Vec<FimixStartDiagnosticV2>,
    pub stability: FimixMultistartStabilityV2,
    /// Additive validation evidence; it is persisted in checked Arrow
    /// sidecars and does not change the selected scientific estimates.
    pub multistart_evidence: FimixMultistartEvidenceV2,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum HeterogeneityV2Error {
    #[error("heterogeneity V2 contract is invalid: {0}")]
    InvalidContract(String),
    #[error("non-finite input in equation {equation_id}, row {row}, column {column}")]
    NonFiniteInput {
        equation_id: String,
        row: usize,
        column: String,
    },
    #[error("no FIMIX start converged")]
    NoConvergedFimixStart {
        diagnostics: Vec<FimixStartDiagnosticV2>,
    },
    #[error(
        "FIMIX optimum was reproduced by {reproducing_starts} starts; {required_starts} required"
    )]
    UnstableFimixOptimum {
        reproducing_starts: usize,
        required_starts: usize,
    },
    #[error("PLS-POS full refit failed: {0}")]
    PosRefit(String),
    #[error("no PLS-POS start completed")]
    NoCompletedPosStart {
        diagnostics: Vec<PosStartDiagnosticV2>,
    },
    #[error(
        "PLS-POS optimum was reproduced by {reproducing_starts} starts; {required_starts} required"
    )]
    UnstablePosOptimum {
        reproducing_starts: usize,
        required_starts: usize,
    },
}

#[derive(Debug, Clone)]
struct InternalEquationEstimate {
    coefficients: Vec<f64>,
    residual_variance: f64,
}

#[derive(Debug, Clone)]
struct InternalClassEstimate {
    proportion: f64,
    equations: Vec<InternalEquationEstimate>,
}

#[derive(Debug, Clone)]
struct InternalFimixRun {
    start_index: usize,
    classes: Vec<InternalClassEstimate>,
    posteriors: Vec<Vec<f64>>,
    hard_assignments: Vec<usize>,
    log_likelihood: f64,
    iterations: usize,
    diagnostic: FimixStartDiagnosticV2,
}

#[derive(Debug)]
struct InternalStartFailure {
    code: FimixStartFailureCodeV2,
    message: String,
    iteration: usize,
    trace: Vec<FimixIterationTraceV2>,
    effective_sizes: Vec<f64>,
    maximum_decrease: f64,
}

/// Estimate a conditional Gaussian finite mixture over the supplied structural
/// equations.  The same class posterior weights are used for every equation;
/// each class/equation pair receives its own coefficients and residual
/// variance.
pub fn fit_fimix_pls_v2(
    input: &StandardizedFimixInputV2,
    config: &FimixPlsV2Config,
) -> Result<FimixPlsV2Result, HeterogeneityV2Error> {
    let observations = validate_fimix_contract(input, config)?;
    let minimum_effective_size = minimum_fimix_class_size(input, config, observations);
    if observations < config.classes.saturating_mul(minimum_effective_size) {
        return Err(HeterogeneityV2Error::InvalidContract(format!(
            "{observations} observations cannot support {} classes at minimum effective size {minimum_effective_size}",
            config.classes
        )));
    }
    let parameter_count = fimix_parameter_count(input, config.classes);
    if parameter_count >= observations {
        return Err(HeterogeneityV2Error::InvalidContract(format!(
            "FIMIX parameter count {parameter_count} must be smaller than observation count {observations}"
        )));
    }

    let mut diagnostics = Vec::with_capacity(config.starts);
    let mut completed = Vec::new();
    for start_index in 0..config.starts {
        let start_seed = derive_domain_seed_v2(config.seed, "fimix_start", start_index as u64);
        match run_fimix_start(
            input,
            config,
            minimum_effective_size,
            start_index,
            start_seed,
        ) {
            Ok(run) => {
                diagnostics.push(run.diagnostic.clone());
                completed.push(run);
            }
            Err(failure) => diagnostics.push(FimixStartDiagnosticV2 {
                start_index,
                start_seed,
                converged: false,
                iterations: failure.iteration,
                final_log_likelihood: failure.trace.last().map(|row| row.log_likelihood),
                maximum_likelihood_decrease: failure.maximum_decrease,
                final_effective_class_sizes: failure.effective_sizes,
                failure_code: Some(failure.code),
                failure_message: Some(failure.message),
                trace: failure.trace,
            }),
        }
    }
    if completed.is_empty() {
        return Err(HeterogeneityV2Error::NoConvergedFimixStart { diagnostics });
    }
    completed.sort_by(|left, right| {
        right
            .log_likelihood
            .total_cmp(&left.log_likelihood)
            .then(left.start_index.cmp(&right.start_index))
    });
    let best = completed[0].clone();
    let (stability, reproducing_count) = fimix_multistart_stability(&best, &completed, config)?;
    if reproducing_count < config.required_reproducing_starts {
        return Err(HeterogeneityV2Error::UnstableFimixOptimum {
            reproducing_starts: reproducing_count,
            required_starts: config.required_reproducing_starts,
        });
    }

    let criteria = information_criteria(best.log_likelihood, parameter_count, observations);
    let entropy = posterior_entropy(&best.posteriors, config.classes);
    let effective_sizes = posterior_class_sizes(&best.posteriors, config.classes);
    let multistart_evidence = build_fimix_multistart_evidence_v2(&best, &completed, config)?;
    let classes = best
        .classes
        .iter()
        .enumerate()
        .map(|(class_index, class)| FimixClassEstimateV2 {
            class_id: format!("class_{}", class_index + 1),
            proportion: class.proportion,
            effective_observations: effective_sizes[class_index],
            equations: class
                .equations
                .iter()
                .zip(&input.equations)
                .map(|(estimate, equation)| FimixEquationEstimateV2 {
                    equation_id: equation.equation_id.clone(),
                    outcome_id: equation.outcome_id.clone(),
                    coefficients: equation
                        .coefficient_ids()
                        .into_iter()
                        .zip(&estimate.coefficients)
                        .map(|(parameter_id, estimate)| FimixCoefficientV2 {
                            parameter_id,
                            estimate: *estimate,
                        })
                        .collect(),
                    residual_variance: estimate.residual_variance,
                })
                .collect(),
        })
        .collect();

    let result = FimixPlsV2Result {
        method_version: FIMIX_PLS_METHOD_VERSION_V2.to_string(),
        interaction_profile: input.interaction_profile,
        metric: input.metric.clone(),
        observations,
        selected_start_index: best.start_index,
        iterations: best.iterations,
        log_likelihood: best.log_likelihood,
        classes,
        posteriors: best.posteriors,
        hard_assignments: best.hard_assignments,
        criteria,
        entropy,
        minimum_effective_class_size: minimum_effective_size,
        starts: diagnostics,
        stability,
        multistart_evidence,
    };
    validate_fimix_multistart_evidence_v2(&result)?;
    Ok(result)
}

fn validate_fimix_contract(
    input: &StandardizedFimixInputV2,
    config: &FimixPlsV2Config,
) -> Result<usize, HeterogeneityV2Error> {
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&config.classes) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX V2 requires 2 through 5 fixed classes".to_string(),
        ));
    }
    if !(10..=100).contains(&config.starts) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX V2 requires 10 through 100 starts".to_string(),
        ));
    }
    if config.max_iterations == 0
        || config.consecutive_stable_iterations == 0
        || config.required_reproducing_starts < 2
        || config.required_reproducing_starts > config.starts
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX iteration and multistart requirements are invalid".to_string(),
        ));
    }
    let positive_finite = [
        config.relative_log_likelihood_tolerance,
        config.residual_variance_floor,
        config.rank_tolerance,
        config.optimum_relative_log_likelihood_tolerance,
        config.optimum_maximum_coefficient_difference,
        config.optimum_mean_posterior_difference,
        config.standardized_mean_tolerance,
        config.standardized_sample_sd_tolerance,
    ];
    if positive_finite
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
        || !config.likelihood_decrease_tolerance.is_finite()
        || config.likelihood_decrease_tolerance.to_bits()
            != FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2.to_bits()
        || !config.minimum_class_share.is_finite()
        || !(0.05..=0.40).contains(&config.minimum_class_share)
        || config.minimum_class_observations == 0
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX numerical tolerances and minimum-class rules are invalid".to_string(),
        ));
    }
    if input.metric.metric_id.trim().is_empty()
        || input.metric.source_sha256.trim().is_empty()
        || !input.metric.scores_standardized_once_on_pooled_rows
        || (input.interaction_profile != HeterogeneityInteractionProfileV2::P0Structural
            && !input.metric.products_standardized_once_on_pooled_rows)
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX V2 requires a pooled score/product standardization receipt".to_string(),
        ));
    }
    if input.equations.is_empty() {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX V2 requires at least one structural equation".to_string(),
        ));
    }
    let observations = input.equations[0].outcome.len();
    if observations < 2 || input.metric.observation_count != observations {
        return Err(HeterogeneityV2Error::InvalidContract(
            "metric receipt and structural equations disagree on observation count".to_string(),
        ));
    }
    let mut equation_ids = BTreeSet::new();
    for equation in &input.equations {
        if equation.equation_id.trim().is_empty()
            || equation.outcome_id.trim().is_empty()
            || !equation_ids.insert(equation.equation_id.clone())
            || equation.design.len() != observations
            || equation.outcome.len() != observations
            || equation.coefficient_count() == 0
        {
            return Err(HeterogeneityV2Error::InvalidContract(format!(
                "structural equation {} has invalid identity or dimensions",
                equation.equation_id
            )));
        }
        let mut predictor_ids = BTreeSet::new();
        if equation.predictor_ids.iter().any(|id| {
            id.trim().is_empty() || id == "(intercept)" || !predictor_ids.insert(id.clone())
        }) {
            return Err(HeterogeneityV2Error::InvalidContract(format!(
                "structural equation {} has invalid predictor identities",
                equation.equation_id
            )));
        }
        for (row_index, row) in equation.design.iter().enumerate() {
            if row.len() != equation.predictor_ids.len() {
                return Err(HeterogeneityV2Error::InvalidContract(format!(
                    "structural equation {} has a ragged design matrix",
                    equation.equation_id
                )));
            }
            for (column_index, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    return Err(HeterogeneityV2Error::NonFiniteInput {
                        equation_id: equation.equation_id.clone(),
                        row: row_index,
                        column: equation.predictor_ids[column_index].clone(),
                    });
                }
            }
            if !equation.outcome[row_index].is_finite() {
                return Err(HeterogeneityV2Error::NonFiniteInput {
                    equation_id: equation.equation_id.clone(),
                    row: row_index,
                    column: equation.outcome_id.clone(),
                });
            }
        }
        validate_standardized_vector(
            &equation.outcome,
            &equation.equation_id,
            &equation.outcome_id,
            config,
        )?;
        for column in 0..equation.predictor_ids.len() {
            let values = equation
                .design
                .iter()
                .map(|row| row[column])
                .collect::<Vec<_>>();
            validate_standardized_vector(
                &values,
                &equation.equation_id,
                &equation.predictor_ids[column],
                config,
            )?;
        }
    }
    Ok(observations)
}

fn validate_standardized_vector(
    values: &[f64],
    equation_id: &str,
    variable_id: &str,
    config: &FimixPlsV2Config,
) -> Result<(), HeterogeneityV2Error> {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sample_variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let sample_sd = sample_variance.sqrt();
    if mean.abs() > config.standardized_mean_tolerance
        || (sample_sd - 1.0).abs() > config.standardized_sample_sd_tolerance
    {
        return Err(HeterogeneityV2Error::InvalidContract(format!(
            "{equation_id}:{variable_id} is not pooled sample-standardized (mean {mean}, sample SD {sample_sd})"
        )));
    }
    Ok(())
}

/// Fits the read-only K=1 pooled reference without invoking a mixture or POS
/// algorithm.  Coefficients use the exact pooled standardized equations later
/// supplied to FIMIX; residual variance follows the maximum-likelihood
/// denominator used by the mixture likelihood.
pub fn fit_pooled_structural_baseline_v2(
    input: &StandardizedFimixInputV2,
    rank_tolerance: f64,
) -> Result<PooledStructuralBaselineV2, HeterogeneityV2Error> {
    if !rank_tolerance.is_finite() || rank_tolerance <= 0.0 {
        return Err(HeterogeneityV2Error::InvalidContract(
            "pooled baseline rank tolerance must be finite and positive".into(),
        ));
    }
    let mut validation = FimixPlsV2Config::for_classes(2);
    validation.rank_tolerance = rank_tolerance;
    let observations = validate_fimix_contract(input, &validation)?;
    let weights = vec![1.0; observations];
    let effective_sizes = [observations as f64];
    let mut equations = Vec::with_capacity(input.equations.len());
    for equation in &input.equations {
        let (estimates, residual_sum_of_squares) =
            weighted_least_squares(equation, &weights, rank_tolerance, 0, 0, &effective_sizes)
                .map_err(|failure| {
                    HeterogeneityV2Error::InvalidContract(format!(
                        "pooled baseline equation {} failed: {}",
                        equation.equation_id, failure.message
                    ))
                })?;
        let residual_variance = residual_sum_of_squares / observations as f64;
        let mean = equation.outcome.iter().sum::<f64>() / observations as f64;
        let total_sum_of_squares = equation
            .outcome
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>();
        let r_squared = 1.0 - residual_sum_of_squares / total_sum_of_squares;
        if !residual_variance.is_finite()
            || residual_variance <= 0.0
            || !r_squared.is_finite()
            || !(-1.0e-10..=1.0 + 1.0e-10).contains(&r_squared)
        {
            return Err(HeterogeneityV2Error::InvalidContract(format!(
                "pooled baseline equation {} produced invalid residual variance or R-squared",
                equation.equation_id
            )));
        }
        equations.push(PooledStructuralBaselineEquationV2 {
            equation_id: equation.equation_id.clone(),
            outcome_id: equation.outcome_id.clone(),
            coefficients: equation
                .coefficient_ids()
                .into_iter()
                .zip(estimates)
                .map(|(parameter_id, estimate)| FimixCoefficientV2 {
                    parameter_id,
                    estimate,
                })
                .collect(),
            residual_variance,
            r_squared: r_squared.clamp(0.0, 1.0),
        });
    }
    Ok(PooledStructuralBaselineV2 {
        method_version: POOLED_STRUCTURAL_BASELINE_METHOD_VERSION_V2.into(),
        metric_source_sha256: input.metric.source_sha256.clone(),
        observations,
        equations,
    })
}

/// Refit the pooled standardized structural metric separately within fixed,
/// aligned POS segments.  Measurement scores/products are never re-estimated
/// here: every segment receives the exact pooled columns from `input`.
pub fn fit_pooled_metric_segment_baselines_v2(
    input: &StandardizedFimixInputV2,
    assignments: &[usize],
    segments: usize,
    rank_tolerance: f64,
) -> Result<Vec<PooledStructuralBaselineV2>, HeterogeneityV2Error> {
    let pooled = fit_pooled_structural_baseline_v2(input, rank_tolerance)?;
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&segments)
        || assignments.len() != pooled.observations
        || assignments.iter().any(|segment| *segment >= segments)
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "pooled common-metric segment assignments are invalid".into(),
        ));
    }
    let counts = (0..segments)
        .map(|segment| {
            assignments
                .iter()
                .filter(|assignment| **assignment == segment)
                .count()
        })
        .collect::<Vec<_>>();
    let mut results = Vec::with_capacity(segments);
    for (segment, observations) in counts.iter().copied().enumerate() {
        if observations == 0 {
            return Err(HeterogeneityV2Error::InvalidContract(format!(
                "pooled common-metric segment {} is empty",
                segment + 1
            )));
        }
        let weights = assignments
            .iter()
            .map(|assignment| f64::from(*assignment == segment))
            .collect::<Vec<_>>();
        let effective_sizes = [observations as f64];
        let mut equations = Vec::with_capacity(input.equations.len());
        for equation in &input.equations {
            if observations <= equation.coefficient_count() {
                return Err(HeterogeneityV2Error::InvalidContract(format!(
                    "pooled common-metric segment {} has insufficient rows for equation {}",
                    segment + 1,
                    equation.equation_id
                )));
            }
            let (estimates, residual_sum_of_squares) =
                weighted_least_squares(equation, &weights, rank_tolerance, 0, 0, &effective_sizes)
                    .map_err(|failure| {
                        HeterogeneityV2Error::InvalidContract(format!(
                            "pooled common-metric segment {}, equation {} failed: {}",
                            segment + 1,
                            equation.equation_id,
                            failure.message
                        ))
                    })?;
            let outcome_values = equation
                .outcome
                .iter()
                .zip(assignments)
                .filter_map(|(value, assignment)| (*assignment == segment).then_some(*value))
                .collect::<Vec<_>>();
            let mean = outcome_values.iter().sum::<f64>() / observations as f64;
            let total_sum_of_squares = outcome_values
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>();
            let residual_variance = residual_sum_of_squares / observations as f64;
            let r_squared = 1.0 - residual_sum_of_squares / total_sum_of_squares;
            if !residual_variance.is_finite()
                || residual_variance <= 0.0
                || !r_squared.is_finite()
                || !(-1.0e-10..=1.0 + 1.0e-10).contains(&r_squared)
            {
                return Err(HeterogeneityV2Error::InvalidContract(format!(
                    "pooled common-metric segment {}, equation {} produced invalid diagnostics",
                    segment + 1,
                    equation.equation_id
                )));
            }
            equations.push(PooledStructuralBaselineEquationV2 {
                equation_id: equation.equation_id.clone(),
                outcome_id: equation.outcome_id.clone(),
                coefficients: equation
                    .coefficient_ids()
                    .into_iter()
                    .zip(estimates)
                    .map(|(parameter_id, estimate)| FimixCoefficientV2 {
                        parameter_id,
                        estimate,
                    })
                    .collect(),
                residual_variance,
                r_squared: r_squared.clamp(0.0, 1.0),
            });
        }
        results.push(PooledStructuralBaselineV2 {
            method_version: POOLED_STRUCTURAL_BASELINE_METHOD_VERSION_V2.into(),
            metric_source_sha256: input.metric.source_sha256.clone(),
            observations,
            equations,
        });
    }
    Ok(results)
}

fn minimum_fimix_class_size(
    input: &StandardizedFimixInputV2,
    config: &FimixPlsV2Config,
    observations: usize,
) -> usize {
    let equation_minimum = input
        .equations
        .iter()
        .map(|equation| equation.predictor_ids.len() + 2)
        .max()
        .unwrap_or(0);
    config
        .minimum_class_observations
        .max((config.minimum_class_share * observations as f64).ceil() as usize)
        .max(equation_minimum)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FimixLikelihoodStepV2 {
    Accept { relative_change: f64 },
    RetainPriorAcceptedState { numerical_decrease: f64 },
}

fn validate_fimix_likelihood_step_v2(
    accepted_likelihood: f64,
    candidate_likelihood: f64,
    relative_decrease_tolerance: f64,
    iteration: usize,
) -> Result<FimixLikelihoodStepV2, InternalStartFailure> {
    if !accepted_likelihood.is_finite() || !candidate_likelihood.is_finite() {
        return Err(start_failure(
            FimixStartFailureCodeV2::NonFinite,
            format!(
                "likelihood update from {accepted_likelihood} to {candidate_likelihood} is non-finite"
            ),
            iteration,
            Vec::new(),
        ));
    }
    let decrease = (accepted_likelihood - candidate_likelihood).max(0.0);
    let permitted_decrease = relative_decrease_tolerance
        * (1.0 + accepted_likelihood.abs().max(candidate_likelihood.abs()));
    if candidate_likelihood < accepted_likelihood {
        if candidate_likelihood + permitted_decrease < accepted_likelihood {
            let mut failure = start_failure(
                FimixStartFailureCodeV2::LikelihoodDecrease,
                format!(
                    "observed likelihood decreased materially from {accepted_likelihood} to {candidate_likelihood}; decrease {decrease} exceeds numerical allowance {permitted_decrease}"
                ),
                iteration,
                Vec::new(),
            );
            failure.maximum_decrease = decrease;
            Err(failure)
        } else {
            Ok(FimixLikelihoodStepV2::RetainPriorAcceptedState {
                numerical_decrease: decrease,
            })
        }
    } else {
        Ok(FimixLikelihoodStepV2::Accept {
            relative_change: (candidate_likelihood - accepted_likelihood)
                / (1.0 + accepted_likelihood.abs()),
        })
    }
}

fn run_fimix_start(
    input: &StandardizedFimixInputV2,
    config: &FimixPlsV2Config,
    minimum_effective_size: usize,
    start_index: usize,
    start_seed: u64,
) -> Result<InternalFimixRun, InternalStartFailure> {
    let mut posteriors = initial_responsibilities(input, config.classes, start_index, start_seed);
    let mut classes = fimix_m_step(input, &posteriors, config, minimum_effective_size, 0)?;
    let (mut log_likelihood, updated) = fimix_e_step(input, &classes, 0)?;
    posteriors = updated;
    let mut trace = vec![FimixIterationTraceV2 {
        iteration: 0,
        log_likelihood,
        relative_change: None,
        minimum_effective_class_size: posterior_class_sizes(&posteriors, config.classes)
            .into_iter()
            .fold(f64::INFINITY, f64::min),
    }];
    let mut stable_iterations = 0usize;
    let mut maximum_decrease = 0.0f64;
    for iteration in 1..=config.max_iterations {
        let candidate_classes = match fimix_m_step(
            input,
            &posteriors,
            config,
            minimum_effective_size,
            iteration,
        ) {
            Ok(classes) => classes,
            Err(mut failure) => {
                failure.trace = trace;
                failure.maximum_decrease = maximum_decrease;
                return Err(failure);
            }
        };
        let (candidate_likelihood, candidate_posteriors) =
            match fimix_e_step(input, &candidate_classes, iteration) {
                Ok(step) => step,
                Err(mut failure) => {
                    failure.trace = trace;
                    failure.maximum_decrease = maximum_decrease;
                    return Err(failure);
                }
            };
        let relative_change = match validate_fimix_likelihood_step_v2(
            log_likelihood,
            candidate_likelihood,
            config.likelihood_decrease_tolerance,
            iteration,
        ) {
            Ok(FimixLikelihoodStepV2::Accept { relative_change }) => {
                classes = candidate_classes;
                posteriors = candidate_posteriors;
                log_likelihood = candidate_likelihood;
                relative_change
            }
            Ok(FimixLikelihoodStepV2::RetainPriorAcceptedState { numerical_decrease }) => {
                maximum_decrease = maximum_decrease.max(numerical_decrease);
                0.0
            }
            Err(mut failure) => {
                maximum_decrease = maximum_decrease.max(failure.maximum_decrease);
                failure.trace = trace;
                failure.effective_sizes = posterior_class_sizes(&posteriors, config.classes);
                failure.maximum_decrease = maximum_decrease;
                return Err(failure);
            }
        };
        let effective_sizes = posterior_class_sizes(&posteriors, config.classes);
        trace.push(FimixIterationTraceV2 {
            iteration,
            log_likelihood,
            relative_change: Some(relative_change),
            minimum_effective_class_size: effective_sizes
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
        });
        if relative_change <= config.relative_log_likelihood_tolerance {
            stable_iterations += 1;
        } else {
            stable_iterations = 0;
        }
        if stable_iterations >= config.consecutive_stable_iterations {
            let hard_assignments = hard_assignments(&posteriors);
            let mut run = InternalFimixRun {
                start_index,
                classes,
                posteriors,
                hard_assignments,
                log_likelihood,
                iterations: iteration,
                diagnostic: FimixStartDiagnosticV2 {
                    start_index,
                    start_seed,
                    converged: true,
                    iterations: iteration,
                    final_log_likelihood: Some(log_likelihood),
                    maximum_likelihood_decrease: maximum_decrease,
                    final_effective_class_sizes: effective_sizes,
                    failure_code: None,
                    failure_message: None,
                    trace,
                },
            };
            canonicalize_internal_fimix_run(&mut run);
            return Ok(run);
        }
    }
    Err(InternalStartFailure {
        code: FimixStartFailureCodeV2::MaximumIterations,
        message: format!(
            "did not meet the consecutive convergence rule within {} iterations",
            config.max_iterations
        ),
        iteration: config.max_iterations,
        trace,
        effective_sizes: posterior_class_sizes(&posteriors, config.classes),
        maximum_decrease,
    })
}

fn initial_responsibilities(
    input: &StandardizedFimixInputV2,
    classes: usize,
    start_index: usize,
    seed: u64,
) -> Vec<Vec<f64>> {
    let observations = input.equations[0].outcome.len();
    let mut order = (0..observations).collect::<Vec<_>>();
    if start_index == 0 {
        order.sort_by(|left, right| {
            initial_row_key(input, *left)
                .total_cmp(&initial_row_key(input, *right))
                .then(left.cmp(right))
        });
    } else {
        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        order.shuffle(&mut rng);
    }
    let mut assigned = vec![0usize; observations];
    for (rank, row) in order.into_iter().enumerate() {
        assigned[row] = if start_index == 0 {
            (rank * classes / observations).min(classes - 1)
        } else {
            rank % classes
        };
    }
    let off_class = 0.05 / classes as f64;
    let on_class = 0.95 + off_class;
    assigned
        .into_iter()
        .map(|class| {
            (0..classes)
                .map(|candidate| {
                    if candidate == class {
                        on_class
                    } else {
                        off_class
                    }
                })
                .collect()
        })
        .collect()
}

fn initial_row_key(input: &StandardizedFimixInputV2, row: usize) -> f64 {
    input
        .equations
        .iter()
        .enumerate()
        .map(|(equation_index, equation)| {
            let outcome = equation.outcome[row] * (equation_index + 1) as f64;
            let predictors = equation.design[row]
                .iter()
                .enumerate()
                .map(|(column, value)| value * (column + 2) as f64 * 0.03125)
                .sum::<f64>();
            outcome + predictors
        })
        .sum()
}

fn fimix_m_step(
    input: &StandardizedFimixInputV2,
    posteriors: &[Vec<f64>],
    config: &FimixPlsV2Config,
    minimum_effective_size: usize,
    iteration: usize,
) -> Result<Vec<InternalClassEstimate>, InternalStartFailure> {
    let observations = posteriors.len();
    let effective_sizes = posterior_class_sizes(posteriors, config.classes);
    for (class, effective_size) in effective_sizes.iter().enumerate() {
        if !effective_size.is_finite()
            || *effective_size < minimum_effective_size as f64
            || *effective_size / (observations as f64) < config.minimum_class_share
        {
            return Err(start_failure(
                FimixStartFailureCodeV2::CollapsedClass,
                format!(
                    "class {} has effective size {effective_size}; minimum is {minimum_effective_size}",
                    class + 1
                ),
                iteration,
                effective_sizes,
            ));
        }
    }
    let mut classes = Vec::with_capacity(config.classes);
    for class in 0..config.classes {
        let weights = posteriors.iter().map(|row| row[class]).collect::<Vec<_>>();
        let mut equations = Vec::with_capacity(input.equations.len());
        for equation in &input.equations {
            let (coefficients, weighted_sse) = weighted_least_squares(
                equation,
                &weights,
                config.rank_tolerance,
                class,
                iteration,
                &effective_sizes,
            )?;
            let variance = weighted_sse / effective_sizes[class];
            if !variance.is_finite() {
                return Err(start_failure(
                    FimixStartFailureCodeV2::NonFinite,
                    format!(
                        "class {}, equation {} produced non-finite residual variance",
                        class + 1,
                        equation.equation_id
                    ),
                    iteration,
                    effective_sizes,
                ));
            }
            if variance <= config.residual_variance_floor {
                return Err(start_failure(
                    FimixStartFailureCodeV2::VarianceCollapse,
                    format!(
                        "class {}, equation {} residual variance {variance} is at or below floor {}",
                        class + 1,
                        equation.equation_id,
                        config.residual_variance_floor
                    ),
                    iteration,
                    effective_sizes,
                ));
            }
            equations.push(InternalEquationEstimate {
                coefficients,
                residual_variance: variance,
            });
        }
        classes.push(InternalClassEstimate {
            proportion: effective_sizes[class] / observations as f64,
            equations,
        });
    }
    Ok(classes)
}

fn weighted_least_squares(
    equation: &StandardizedStructuralEquationV2,
    weights: &[f64],
    rank_tolerance: f64,
    class: usize,
    iteration: usize,
    effective_sizes: &[f64],
) -> Result<(Vec<f64>, f64), InternalStartFailure> {
    let coefficient_count = equation.coefficient_count();
    let mut columns = Vec::with_capacity(coefficient_count);
    if equation.include_intercept {
        columns.push(vec![1.0; equation.outcome.len()]);
    }
    for column in 0..equation.predictor_ids.len() {
        columns.push(equation.design.iter().map(|row| row[column]).collect());
    }
    let sqrt_weights = weights
        .iter()
        .map(|weight| weight.sqrt())
        .collect::<Vec<_>>();
    let weighted_outcome = equation
        .outcome
        .iter()
        .zip(&sqrt_weights)
        .map(|(value, weight)| value * weight)
        .collect::<Vec<_>>();
    let mut q_columns = Vec::<Vec<f64>>::with_capacity(coefficient_count);
    let mut r = vec![vec![0.0; coefficient_count]; coefficient_count];
    let tolerance = rank_tolerance * effective_sizes[class].sqrt().max(1.0);
    for (column_index, column) in columns.iter().enumerate() {
        let mut residual = column
            .iter()
            .zip(&sqrt_weights)
            .map(|(value, weight)| value * weight)
            .collect::<Vec<_>>();
        for previous in 0..column_index {
            let projection = dot(&q_columns[previous], &residual);
            r[previous][column_index] += projection;
            subtract_scaled(&mut residual, &q_columns[previous], projection);
        }
        for previous in 0..column_index {
            let correction = dot(&q_columns[previous], &residual);
            r[previous][column_index] += correction;
            subtract_scaled(&mut residual, &q_columns[previous], correction);
        }
        let norm = dot(&residual, &residual).sqrt();
        if !norm.is_finite() || norm <= tolerance {
            return Err(start_failure(
                FimixStartFailureCodeV2::RankDeficient,
                format!(
                    "class {}, equation {} is rank deficient at coefficient {}",
                    class + 1,
                    equation.equation_id,
                    equation.coefficient_ids()[column_index]
                ),
                iteration,
                effective_sizes.to_vec(),
            ));
        }
        r[column_index][column_index] = norm;
        for value in &mut residual {
            *value /= norm;
        }
        q_columns.push(residual);
    }
    let mut coefficients = q_columns
        .iter()
        .map(|column| dot(column, &weighted_outcome))
        .collect::<Vec<_>>();
    for row in (0..coefficient_count).rev() {
        for column in row + 1..coefficient_count {
            coefficients[row] -= r[row][column] * coefficients[column];
        }
        coefficients[row] /= r[row][row];
    }
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(start_failure(
            FimixStartFailureCodeV2::NonFinite,
            format!(
                "class {}, equation {} produced non-finite coefficients",
                class + 1,
                equation.equation_id
            ),
            iteration,
            effective_sizes.to_vec(),
        ));
    }
    let weighted_sse = equation
        .design
        .iter()
        .zip(&equation.outcome)
        .zip(weights)
        .map(|((row, outcome), weight)| {
            let mut offset = 0usize;
            let mut fitted = 0.0;
            if equation.include_intercept {
                fitted = coefficients[0];
                offset = 1;
            }
            fitted += row
                .iter()
                .zip(&coefficients[offset..])
                .map(|(predictor, coefficient)| predictor * coefficient)
                .sum::<f64>();
            weight * (outcome - fitted).powi(2)
        })
        .sum::<f64>();
    if !weighted_sse.is_finite() {
        return Err(start_failure(
            FimixStartFailureCodeV2::NonFinite,
            format!(
                "class {}, equation {} produced non-finite weighted SSE",
                class + 1,
                equation.equation_id
            ),
            iteration,
            effective_sizes.to_vec(),
        ));
    }
    Ok((coefficients, weighted_sse))
}

fn fimix_e_step(
    input: &StandardizedFimixInputV2,
    classes: &[InternalClassEstimate],
    iteration: usize,
) -> Result<(f64, Vec<Vec<f64>>), InternalStartFailure> {
    let mut log_likelihood = 0.0;
    let mut posteriors = Vec::with_capacity(input.metric.observation_count);
    for row in 0..input.metric.observation_count {
        let mut log_joints = Vec::with_capacity(classes.len());
        for class in classes {
            let mut log_joint = class.proportion.ln();
            for (equation, estimate) in input.equations.iter().zip(&class.equations) {
                let mut offset = 0usize;
                let mut fitted = 0.0;
                if equation.include_intercept {
                    fitted = estimate.coefficients[0];
                    offset = 1;
                }
                fitted += equation.design[row]
                    .iter()
                    .zip(&estimate.coefficients[offset..])
                    .map(|(predictor, coefficient)| predictor * coefficient)
                    .sum::<f64>();
                let residual = equation.outcome[row] - fitted;
                log_joint += -0.5
                    * (LOG_TWO_PI
                        + estimate.residual_variance.ln()
                        + residual * residual / estimate.residual_variance);
            }
            log_joints.push(log_joint);
        }
        let row_log_likelihood = log_sum_exp_v2(&log_joints).map_err(|message| {
            start_failure(
                FimixStartFailureCodeV2::NonFinite,
                format!("row {row}: {message}"),
                iteration,
                Vec::new(),
            )
        })?;
        log_likelihood += row_log_likelihood;
        let probabilities = log_joints
            .iter()
            .map(|value| (value - row_log_likelihood).exp())
            .collect::<Vec<_>>();
        if probabilities.iter().any(|value| !value.is_finite())
            || (probabilities.iter().sum::<f64>() - 1.0).abs() > 1.0e-10
        {
            return Err(start_failure(
                FimixStartFailureCodeV2::NonFinite,
                format!("row {row} produced invalid posterior probabilities"),
                iteration,
                Vec::new(),
            ));
        }
        posteriors.push(probabilities);
    }
    if !log_likelihood.is_finite() {
        return Err(start_failure(
            FimixStartFailureCodeV2::NonFinite,
            "observed-data log likelihood is non-finite".to_string(),
            iteration,
            Vec::new(),
        ));
    }
    Ok((log_likelihood, posteriors))
}

/// Stable `ln(sum(exp(values)))`. Empty or non-finite input fails closed.
pub fn log_sum_exp_v2(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("log-sum-exp requires a non-empty finite vector".to_string());
    }
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let shifted_sum = values
        .iter()
        .map(|value| (value - maximum).exp())
        .sum::<f64>();
    let result = maximum + shifted_sum.ln();
    if result.is_finite() {
        Ok(result)
    } else {
        Err("log-sum-exp produced a non-finite result".to_string())
    }
}

fn start_failure(
    code: FimixStartFailureCodeV2,
    message: String,
    iteration: usize,
    effective_sizes: Vec<f64>,
) -> InternalStartFailure {
    InternalStartFailure {
        code,
        message,
        iteration,
        trace: Vec::new(),
        effective_sizes,
        maximum_decrease: 0.0,
    }
}

fn posterior_class_sizes(posteriors: &[Vec<f64>], classes: usize) -> Vec<f64> {
    let mut sizes = vec![0.0; classes];
    for row in posteriors {
        for (class, probability) in row.iter().enumerate().take(classes) {
            sizes[class] += probability;
        }
    }
    sizes
}

fn hard_assignments(posteriors: &[Vec<f64>]) -> Vec<usize> {
    posteriors
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .max_by(|left, right| left.1.total_cmp(right.1).then(right.0.cmp(&left.0)))
                .map(|(class, _)| class)
                .unwrap_or(0)
        })
        .collect()
}

fn canonicalize_internal_fimix_run(run: &mut InternalFimixRun) {
    let mut order = (0..run.classes.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        compare_f64_slices(
            &internal_class_signature(&run.classes[*left]),
            &internal_class_signature(&run.classes[*right]),
        )
        .then(left.cmp(right))
    });
    let mut old_to_new = vec![0usize; order.len()];
    for (new, old) in order.iter().enumerate() {
        old_to_new[*old] = new;
    }
    run.classes = order.iter().map(|old| run.classes[*old].clone()).collect();
    let old_effective_sizes = run.diagnostic.final_effective_class_sizes.clone();
    run.diagnostic.final_effective_class_sizes =
        order.iter().map(|old| old_effective_sizes[*old]).collect();
    for row in &mut run.posteriors {
        let old = row.clone();
        for (new, old_index) in order.iter().enumerate() {
            row[new] = old[*old_index];
        }
    }
    run.hard_assignments = run
        .hard_assignments
        .iter()
        .map(|old| old_to_new[*old])
        .collect();
}

fn internal_class_signature(class: &InternalClassEstimate) -> Vec<f64> {
    let mut signature = Vec::new();
    for equation in &class.equations {
        signature.extend(&equation.coefficients);
        signature.push(equation.residual_variance);
    }
    signature.push(class.proportion);
    signature
}

fn compare_f64_slices(left: &[f64], right: &[f64]) -> std::cmp::Ordering {
    left.iter()
        .zip(right)
        .map(|(left, right)| left.total_cmp(right))
        .find(|ordering| !ordering.is_eq())
        .unwrap_or_else(|| left.len().cmp(&right.len()))
}

fn fimix_multistart_stability(
    best: &InternalFimixRun,
    completed: &[InternalFimixRun],
    config: &FimixPlsV2Config,
) -> Result<(FimixMultistartStabilityV2, usize), HeterogeneityV2Error> {
    let mut reproducing = vec![best.start_index];
    let mut maximum_coefficient_difference = 0.0f64;
    let mut maximum_posterior_difference = 0.0f64;
    for candidate in completed.iter().skip(1) {
        let likelihood_difference = (candidate.log_likelihood - best.log_likelihood).abs()
            / (1.0 + best.log_likelihood.abs());
        if likelihood_difference > config.optimum_relative_log_likelihood_tolerance {
            continue;
        }
        let alignment = align_labels_exhaustive_v2(
            &best.hard_assignments,
            &candidate.hard_assignments,
            best.classes.len(),
        )?;
        if alignment.ambiguous {
            continue;
        }
        let coefficient_difference = aligned_coefficient_difference(best, candidate, &alignment);
        let posterior_difference = aligned_mean_posterior_difference(best, candidate, &alignment);
        if coefficient_difference <= config.optimum_maximum_coefficient_difference
            && posterior_difference <= config.optimum_mean_posterior_difference
        {
            reproducing.push(candidate.start_index);
            maximum_coefficient_difference =
                maximum_coefficient_difference.max(coefficient_difference);
            maximum_posterior_difference = maximum_posterior_difference.max(posterior_difference);
        }
    }
    let count = reproducing.len();
    Ok((
        FimixMultistartStabilityV2 {
            required_reproducing_starts: config.required_reproducing_starts,
            reproducing_start_indices: reproducing,
            maximum_aligned_coefficient_difference: maximum_coefficient_difference,
            maximum_aligned_mean_posterior_difference: maximum_posterior_difference,
            stable: count >= config.required_reproducing_starts,
        },
        count,
    ))
}

fn aligned_coefficient_difference(
    reference: &InternalFimixRun,
    candidate: &InternalFimixRun,
    alignment: &LabelAlignmentV2,
) -> f64 {
    let mut maximum = 0.0f64;
    for (candidate_class, reference_class) in
        alignment.candidate_to_reference.iter().copied().enumerate()
    {
        let left = internal_class_coefficient_signature(&reference.classes[reference_class]);
        let right = internal_class_coefficient_signature(&candidate.classes[candidate_class]);
        for (left, right) in left.iter().zip(&right) {
            maximum = maximum.max((left - right).abs());
        }
    }
    maximum
}

fn internal_class_coefficient_signature(class: &InternalClassEstimate) -> Vec<f64> {
    class
        .equations
        .iter()
        .flat_map(|equation| equation.coefficients.iter().copied())
        .collect()
}

fn aligned_mean_posterior_difference(
    reference: &InternalFimixRun,
    candidate: &InternalFimixRun,
    alignment: &LabelAlignmentV2,
) -> f64 {
    let total = reference
        .posteriors
        .iter()
        .zip(&candidate.posteriors)
        .map(|(reference_row, candidate_row)| {
            alignment
                .candidate_to_reference
                .iter()
                .copied()
                .enumerate()
                .map(|(candidate_class, reference_class)| {
                    (reference_row[reference_class] - candidate_row[candidate_class]).abs()
                })
                .sum::<f64>()
        })
        .sum::<f64>();
    total / (reference.posteriors.len() * reference.classes.len()) as f64
}

fn build_fimix_multistart_evidence_v2(
    best: &InternalFimixRun,
    completed: &[InternalFimixRun],
    config: &FimixPlsV2Config,
) -> Result<FimixMultistartEvidenceV2, HeterogeneityV2Error> {
    let completed_starts = completed
        .iter()
        .map(|run| {
            let coefficient_signatures = run
                .classes
                .iter()
                .map(internal_class_coefficient_signature)
                .collect::<Vec<_>>();
            Ok(FimixCompletedStartEvidenceV2 {
                start_index: run.start_index,
                final_log_likelihood: run.log_likelihood,
                canonical_hard_assignments: run.hard_assignments.clone(),
                canonical_coefficient_signatures: coefficient_signatures.clone(),
                canonical_posteriors: run.posteriors.clone(),
                partition_sha256: heterogeneity_multistart_partition_sha256_v2(
                    &run.hard_assignments,
                )?,
                coefficient_sha256: heterogeneity_multistart_matrix_sha256_v2(
                    HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2,
                    &coefficient_signatures,
                )?,
                posterior_sha256: heterogeneity_multistart_matrix_sha256_v2(
                    HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2,
                    &run.posteriors,
                )?,
                fit_statistic_sha256: heterogeneity_multistart_fit_statistic_sha256_v2(
                    run.log_likelihood,
                )?,
            })
        })
        .collect::<Result<Vec<_>, HeterogeneityV2Error>>()?;
    Ok(FimixMultistartEvidenceV2 {
        schema_version: HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2,
        selected_start_index: best.start_index,
        observations: best.posteriors.len(),
        classes: best.classes.len(),
        required_reproducing_starts: config.required_reproducing_starts,
        relative_log_likelihood_tolerance: config.optimum_relative_log_likelihood_tolerance,
        maximum_coefficient_difference_tolerance: config.optimum_maximum_coefficient_difference,
        mean_posterior_difference_tolerance: config.optimum_mean_posterior_difference,
        completed_starts,
    })
}

/// Replays every FIMIX multistart identity and tolerance from retained values.
/// Runner and persistence boundaries call this again instead of trusting the
/// engine's `stable` flag or reproducing-start list.
pub fn validate_fimix_multistart_evidence_v2(
    result: &FimixPlsV2Result,
) -> Result<(), HeterogeneityV2Error> {
    let evidence = &result.multistart_evidence;
    let classes = result.classes.len();
    let completed_indices = result
        .starts
        .iter()
        .filter(|start| start.converged)
        .map(|start| start.start_index)
        .collect::<BTreeSet<_>>();
    let retained_indices = evidence
        .completed_starts
        .iter()
        .map(|start| start.start_index)
        .collect::<BTreeSet<_>>();
    if evidence.schema_version != HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2
        || evidence.selected_start_index != result.selected_start_index
        || evidence.observations != result.observations
        || evidence.classes != classes
        || evidence.required_reproducing_starts != result.stability.required_reproducing_starts
        || evidence.completed_starts.is_empty()
        || evidence.completed_starts[0].start_index != result.selected_start_index
        || retained_indices.len() != evidence.completed_starts.len()
        || retained_indices != completed_indices
        || [
            evidence.relative_log_likelihood_tolerance,
            evidence.maximum_coefficient_difference_tolerance,
            evidence.mean_posterior_difference_tolerance,
        ]
        .into_iter()
        .any(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX multistart evidence identity or completed-start inventory is invalid".into(),
        ));
    }
    let selected_signatures = result
        .classes
        .iter()
        .map(|class| {
            class
                .equations
                .iter()
                .flat_map(|equation| equation.coefficients.iter().map(|row| row.estimate))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let signature_widths = selected_signatures.iter().map(Vec::len).collect::<Vec<_>>();
    if signature_widths.iter().any(|width| *width == 0) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX multistart evidence has an empty coefficient signature".into(),
        ));
    }
    for retained in &evidence.completed_starts {
        let diagnostic = result
            .starts
            .iter()
            .find(|start| start.start_index == retained.start_index)
            .filter(|start| start.converged)
            .ok_or_else(|| {
                HeterogeneityV2Error::InvalidContract(
                    "FIMIX multistart evidence references a nonconverged start".into(),
                )
            })?;
        if diagnostic.final_log_likelihood.map(f64::to_bits)
            != Some(retained.final_log_likelihood.to_bits())
            || !retained.final_log_likelihood.is_finite()
            || retained.canonical_hard_assignments.len() != result.observations
            || retained
                .canonical_hard_assignments
                .iter()
                .any(|class| *class >= classes)
            || retained.canonical_coefficient_signatures.len() != classes
            || retained
                .canonical_coefficient_signatures
                .iter()
                .enumerate()
                .any(|(class, values)| {
                    values.len() != signature_widths[class]
                        || values.iter().any(|value| !value.is_finite())
                })
            || retained.canonical_posteriors.len() != result.observations
            || retained.canonical_posteriors.iter().any(|row| {
                row.len() != classes
                    || row
                        .iter()
                        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                    || (row.iter().sum::<f64>() - 1.0).abs() > 1.0e-10
            })
            || retained.partition_sha256
                != heterogeneity_multistart_partition_sha256_v2(
                    &retained.canonical_hard_assignments,
                )?
            || retained.coefficient_sha256
                != heterogeneity_multistart_matrix_sha256_v2(
                    HETEROGENEITY_MULTISTART_COEFFICIENT_DIGEST_DOMAIN_V2,
                    &retained.canonical_coefficient_signatures,
                )?
            || retained.posterior_sha256
                != heterogeneity_multistart_matrix_sha256_v2(
                    HETEROGENEITY_MULTISTART_POSTERIOR_DIGEST_DOMAIN_V2,
                    &retained.canonical_posteriors,
                )?
            || retained.fit_statistic_sha256
                != heterogeneity_multistart_fit_statistic_sha256_v2(retained.final_log_likelihood)?
        {
            return Err(HeterogeneityV2Error::InvalidContract(format!(
                "FIMIX completed-start evidence {} is malformed or digest-inconsistent",
                retained.start_index
            )));
        }
    }
    let selected = &evidence.completed_starts[0];
    if selected.canonical_hard_assignments != result.hard_assignments
        || selected.canonical_coefficient_signatures != selected_signatures
        || selected.canonical_posteriors != result.posteriors
        || selected.final_log_likelihood.to_bits() != result.log_likelihood.to_bits()
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX selected-start evidence differs from the published selected solution".into(),
        ));
    }
    let mut reproducing = vec![selected.start_index];
    let mut maximum_coefficient_difference = 0.0_f64;
    let mut maximum_posterior_difference = 0.0_f64;
    for candidate in evidence.completed_starts.iter().skip(1) {
        let likelihood_difference =
            (candidate.final_log_likelihood - selected.final_log_likelihood).abs()
                / (1.0 + selected.final_log_likelihood.abs());
        if likelihood_difference > evidence.relative_log_likelihood_tolerance {
            continue;
        }
        let alignment = align_labels_exhaustive_v2(
            &selected.canonical_hard_assignments,
            &candidate.canonical_hard_assignments,
            classes,
        )?;
        if alignment.ambiguous {
            continue;
        }
        let mut coefficient_difference = 0.0_f64;
        for (candidate_class, reference_class) in
            alignment.candidate_to_reference.iter().copied().enumerate()
        {
            for (reference, observed) in selected.canonical_coefficient_signatures[reference_class]
                .iter()
                .zip(&candidate.canonical_coefficient_signatures[candidate_class])
            {
                coefficient_difference = coefficient_difference.max((reference - observed).abs());
            }
        }
        let posterior_difference = selected
            .canonical_posteriors
            .iter()
            .zip(&candidate.canonical_posteriors)
            .map(|(reference_row, candidate_row)| {
                alignment
                    .candidate_to_reference
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(candidate_class, reference_class)| {
                        (reference_row[reference_class] - candidate_row[candidate_class]).abs()
                    })
                    .sum::<f64>()
            })
            .sum::<f64>()
            / (result.observations * classes) as f64;
        if coefficient_difference <= evidence.maximum_coefficient_difference_tolerance
            && posterior_difference <= evidence.mean_posterior_difference_tolerance
        {
            reproducing.push(candidate.start_index);
            maximum_coefficient_difference =
                maximum_coefficient_difference.max(coefficient_difference);
            maximum_posterior_difference = maximum_posterior_difference.max(posterior_difference);
        }
    }
    if reproducing != result.stability.reproducing_start_indices
        || maximum_coefficient_difference.to_bits()
            != result
                .stability
                .maximum_aligned_coefficient_difference
                .to_bits()
        || maximum_posterior_difference.to_bits()
            != result
                .stability
                .maximum_aligned_mean_posterior_difference
                .to_bits()
        || result.stability.stable != (reproducing.len() >= evidence.required_reproducing_starts)
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX stability does not replay from completed-start evidence".into(),
        ));
    }
    Ok(())
}

fn fimix_parameter_count(input: &StandardizedFimixInputV2, classes: usize) -> usize {
    let per_class = input
        .equations
        .iter()
        .map(|equation| equation.coefficient_count() + 1)
        .sum::<usize>();
    classes * per_class + classes - 1
}

fn information_criteria(
    log_likelihood: f64,
    parameter_count: usize,
    observations: usize,
) -> FimixInformationCriteriaV2 {
    let deviance = -2.0 * log_likelihood;
    let parameters = parameter_count as f64;
    let log_n = (observations as f64).ln();
    FimixInformationCriteriaV2 {
        parameter_count,
        aic: deviance + 2.0 * parameters,
        aic3: deviance + 3.0 * parameters,
        aic4: deviance + 4.0 * parameters,
        bic: deviance + parameters * log_n,
        caic: deviance + parameters * (log_n + 1.0),
        hq: deviance + 2.0 * parameters * log_n.ln(),
    }
}

fn posterior_entropy(posteriors: &[Vec<f64>], classes: usize) -> FimixEntropyV2 {
    let raw = posteriors
        .iter()
        .flat_map(|row| row.iter())
        .filter(|probability| **probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum::<f64>();
    FimixEntropyV2 {
        raw,
        normalized_certainty: 1.0 - raw / (posteriors.len() as f64 * (classes as f64).ln()),
    }
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

fn derive_domain_seed_v2(master_seed: u64, domain: &str, index: u64) -> u64 {
    let mut digest = Sha256::new();
    digest.update(b"quickpls:heterogeneity:v2\0");
    digest.update(master_seed.to_le_bytes());
    digest.update(domain.as_bytes());
    digest.update([0u8]);
    digest.update(index.to_le_bytes());
    let bytes = digest.finalize();
    u64::from_le_bytes(
        bytes[..8]
            .try_into()
            .expect("SHA-256 prefix is eight bytes"),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CanonicalLabelingV2 {
    /// Mapping from the caller's old label to the canonical zero-based label.
    pub old_to_canonical: Vec<usize>,
    pub canonical_assignments: Vec<usize>,
}

/// Canonicalize arbitrary labels by lexicographically ordering their complete
/// finite parameter signatures. Equal signatures are broken by the old label,
/// making the operation deterministic without attributing substantive meaning
/// to the resulting label numbers.
pub fn canonicalize_labels_by_signature_v2(
    assignments: &[usize],
    signatures: &[Vec<f64>],
) -> Result<CanonicalLabelingV2, HeterogeneityV2Error> {
    let classes = signatures.len();
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&classes)
        || assignments.is_empty()
        || assignments.iter().any(|label| *label >= classes)
        || assignments.iter().copied().collect::<BTreeSet<_>>().len() != classes
        || signatures.iter().any(|signature| {
            signature.is_empty() || signature.iter().any(|value| !value.is_finite())
        })
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "canonical labeling requires 2 through 5 represented finite signatures".to_string(),
        ));
    }
    let mut order = (0..classes).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        compare_f64_slices(&signatures[*left], &signatures[*right]).then(left.cmp(right))
    });
    let mut old_to_canonical = vec![0usize; classes];
    for (canonical, old) in order.into_iter().enumerate() {
        old_to_canonical[old] = canonical;
    }
    Ok(CanonicalLabelingV2 {
        canonical_assignments: assignments
            .iter()
            .map(|old| old_to_canonical[*old])
            .collect(),
        old_to_canonical,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LabelAlignmentV2 {
    /// Mapping from candidate label to reference label.
    pub candidate_to_reference: Vec<usize>,
    pub matched_observations: usize,
    pub match_share: f64,
    pub ambiguous: bool,
    pub mutual_majority: bool,
    /// Rows are reference labels; columns are candidate labels.
    pub overlap: Vec<Vec<usize>>,
}

/// Recomputes every claim carried by retained label-alignment evidence from
/// its overlap matrix. This validator deliberately does not trust the stored
/// mapping, match count, ambiguity, or mutual-majority flags.
pub fn validate_retained_label_alignment_v2(
    alignment: &LabelAlignmentV2,
) -> Result<usize, HeterogeneityV2Error> {
    let classes = alignment.overlap.len();
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&classes)
        || alignment.candidate_to_reference.len() != classes
        || alignment.overlap.iter().any(|row| row.len() != classes)
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "retained label alignment must contain a square K=2..5 overlap matrix and K-label mapping"
                .to_string(),
        ));
    }
    let mapping_labels = alignment
        .candidate_to_reference
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if mapping_labels.len() != classes || mapping_labels.iter().any(|label| *label >= classes) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "retained label alignment mapping is not a K-label bijection".to_string(),
        ));
    }
    let reference_counts = alignment
        .overlap
        .iter()
        .map(|row| {
            row.iter()
                .try_fold(0usize, |total, value| total.checked_add(*value).ok_or(()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|()| {
            HeterogeneityV2Error::InvalidContract(
                "retained label overlap cardinality overflowed".to_string(),
            )
        })?;
    let candidate_counts = (0..classes)
        .map(|candidate| {
            alignment.overlap.iter().try_fold(0usize, |total, row| {
                total.checked_add(row[candidate]).ok_or(())
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|()| {
            HeterogeneityV2Error::InvalidContract(
                "retained label overlap cardinality overflowed".to_string(),
            )
        })?;
    if reference_counts.contains(&0) || candidate_counts.contains(&0) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "retained label alignment omits a represented reference or candidate label".to_string(),
        ));
    }
    let observations = reference_counts
        .iter()
        .try_fold(0usize, |total, value| total.checked_add(*value))
        .ok_or_else(|| {
            HeterogeneityV2Error::InvalidContract(
                "retained label overlap cardinality overflowed".to_string(),
            )
        })?;
    let candidate_observations = candidate_counts
        .iter()
        .try_fold(0usize, |total, value| total.checked_add(*value))
        .ok_or_else(|| {
            HeterogeneityV2Error::InvalidContract(
                "retained label overlap cardinality overflowed".to_string(),
            )
        })?;
    if observations == 0 || candidate_observations != observations {
        return Err(HeterogeneityV2Error::InvalidContract(
            "retained label alignment has an invalid overlap cardinality".to_string(),
        ));
    }
    let mut permutations = Vec::new();
    enumerate_permutations(&mut (0..classes).collect::<Vec<_>>(), 0, &mut permutations);
    let scores = permutations
        .iter()
        .map(|mapping| {
            (0..classes)
                .map(|candidate| alignment.overlap[mapping[candidate]][candidate])
                .sum::<usize>()
        })
        .collect::<Vec<_>>();
    let best_score = scores.iter().copied().max().expect("K is nonzero");
    let best_count = scores.iter().filter(|score| **score == best_score).count();
    let retained_score = (0..classes)
        .map(|candidate| alignment.overlap[alignment.candidate_to_reference[candidate]][candidate])
        .sum::<usize>();
    let expected_mutual_majority = alignment
        .candidate_to_reference
        .iter()
        .copied()
        .enumerate()
        .all(|(candidate, reference)| {
            let matched = alignment.overlap[reference][candidate];
            matched > reference_counts[reference] / 2 && matched > candidate_counts[candidate] / 2
        });
    let expected_share = best_score as f64 / observations as f64;
    if retained_score != best_score
        || alignment.matched_observations != best_score
        || !alignment.match_share.is_finite()
        || (alignment.match_share - expected_share).abs() > 1.0e-12
        || alignment.ambiguous != (best_count > 1)
        || alignment.mutual_majority != expected_mutual_majority
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "retained label alignment claims do not match exhaustive K! reconstruction".to_string(),
        ));
    }
    Ok(observations)
}

/// Exhaustively align at most five candidate labels to a reference partition.
/// The K! search is exact. Tied best mappings are reported as ambiguous rather
/// than silently resolved for inferential bootstrap use.
pub fn align_labels_exhaustive_v2(
    reference: &[usize],
    candidate: &[usize],
    classes: usize,
) -> Result<LabelAlignmentV2, HeterogeneityV2Error> {
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&classes)
        || reference.is_empty()
        || reference.len() != candidate.len()
        || reference.iter().any(|label| *label >= classes)
        || candidate.iter().any(|label| *label >= classes)
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "label alignment requires equal non-empty partitions with 2 through 5 labels"
                .to_string(),
        ));
    }
    let represented_reference = reference.iter().copied().collect::<BTreeSet<_>>();
    let represented_candidate = candidate.iter().copied().collect::<BTreeSet<_>>();
    if represented_reference.len() != classes || represented_candidate.len() != classes {
        return Err(HeterogeneityV2Error::InvalidContract(
            "every label must be represented before alignment".to_string(),
        ));
    }
    let mut overlap = vec![vec![0usize; classes]; classes];
    for (reference, candidate) in reference.iter().zip(candidate) {
        overlap[*reference][*candidate] += 1;
    }
    let mut permutations = Vec::new();
    enumerate_permutations(&mut (0..classes).collect::<Vec<_>>(), 0, &mut permutations);
    let mut best_score = 0usize;
    let mut best_mapping = Vec::new();
    let mut best_count = 0usize;
    for mapping in permutations {
        let score = (0..classes)
            .map(|candidate_label| overlap[mapping[candidate_label]][candidate_label])
            .sum::<usize>();
        if score > best_score || best_mapping.is_empty() {
            best_score = score;
            best_mapping = mapping;
            best_count = 1;
        } else if score == best_score {
            best_count += 1;
        }
    }
    let reference_counts = (0..classes)
        .map(|class| reference.iter().filter(|label| **label == class).count())
        .collect::<Vec<_>>();
    let candidate_counts = (0..classes)
        .map(|class| candidate.iter().filter(|label| **label == class).count())
        .collect::<Vec<_>>();
    let mutual_majority =
        best_mapping
            .iter()
            .copied()
            .enumerate()
            .all(|(candidate_label, reference_label)| {
                let matched = overlap[reference_label][candidate_label];
                matched * 2 > reference_counts[reference_label]
                    && matched * 2 > candidate_counts[candidate_label]
            });
    Ok(LabelAlignmentV2 {
        candidate_to_reference: best_mapping,
        matched_observations: best_score,
        match_share: best_score as f64 / reference.len() as f64,
        ambiguous: best_count > 1,
        mutual_majority,
        overlap,
    })
}

fn enumerate_permutations(values: &mut [usize], index: usize, output: &mut Vec<Vec<usize>>) {
    if index == values.len() {
        output.push(values.to_vec());
        return;
    }
    for candidate in index..values.len() {
        values.swap(index, candidate);
        enumerate_permutations(values, index + 1, output);
        values.swap(index, candidate);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosScoringContractV2 {
    PublishedP0FullSegmentPls,
    DestinationScoredInteractions {
        profile: HeterogeneityInteractionProfileV2,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PosFullRefitReceiptV2 {
    pub method_version: String,
    pub full_segment_pls_refit: bool,
    pub measurement_scores_reestimated: bool,
    pub score_orientation_reapplied: bool,
    pub interaction_stage_one_refit: bool,
    pub interaction_operands_restandardized_within_destination: bool,
    pub interaction_products_rebuilt_within_destination: bool,
    pub joint_structural_equations_refit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosOutcomeR2V2 {
    pub outcome_id: String,
    pub r_squared: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosOutcomeFitAuditV2 {
    pub outcome_id: String,
    pub source_row_indices: Vec<usize>,
    pub observed_scores: Vec<f64>,
    pub fitted_scores: Vec<f64>,
    /// Retained independently checkable centering receipt for standardized
    /// destination-local outcome scores.
    pub observed_mean: f64,
    /// Exact denominator used by the POS R-squared calculation.
    pub centered_total_sum_of_squares: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosSegmentFullFitV2 {
    pub r_squared: Vec<PosOutcomeR2V2>,
    /// Final outcome-score rows retained so R-squared and the unweighted POS
    /// objective can be reconstructed independently.
    pub outcome_fit_audits: Vec<PosOutcomeFitAuditV2>,
    /// Stable, complete parameter vector used for canonical labels and
    /// multistart reproducibility. The compiler owns its documented ordering.
    pub parameter_signature: Vec<f64>,
    pub receipt: PosFullRefitReceiptV2,
}

/// Adapter implemented by the PLS layer. Every request must execute a complete
/// segment fit; cached approximations and partial path-only updates are outside
/// this contract.
pub trait PlsPosFullRefitterV2 {
    fn refit_segment(
        &mut self,
        segment_index: usize,
        row_indices: &[usize],
        scoring: PosScoringContractV2,
    ) -> Result<PosSegmentFullFitV2, String>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPosV2Config {
    pub segments: usize,
    pub minimum_segment_size: usize,
    pub maximum_accepted_moves: usize,
    pub strict_improvement_tolerance: f64,
    pub stable_objective_tolerance: f64,
    pub required_reproducing_starts: usize,
}

impl PlsPosV2Config {
    pub fn for_segments(segments: usize, observations: usize) -> Self {
        Self {
            segments,
            minimum_segment_size: 20usize.max((0.05 * observations as f64).ceil() as usize),
            maximum_accepted_moves: 1_000usize.max(observations.saturating_mul(2)),
            strict_improvement_tolerance: 1.0e-12,
            stable_objective_tolerance: 1.0e-10,
            required_reproducing_starts: 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PosCandidateRefitFailureV2 {
    pub observation: usize,
    pub source_segment: usize,
    pub destination_segment: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosStartDiagnosticV2 {
    pub start_index: usize,
    pub completed: bool,
    pub accepted_moves: usize,
    pub final_objective: Option<f64>,
    pub failure_reason: Option<String>,
    pub candidate_refit_failures: Vec<PosCandidateRefitFailureV2>,
    pub objective_history: Vec<f64>,
}

/// Validation-only receipt for one completed PLS-POS start. Segment labels
/// and parameter rows are in the start's canonical order; an independent
/// verifier can exhaustively realign them to the selected partition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosCompletedStartEvidenceV2 {
    pub start_index: usize,
    pub final_objective: f64,
    pub canonical_assignments: Vec<usize>,
    pub canonical_parameter_signatures: Vec<Vec<f64>>,
    pub partition_sha256: String,
    pub parameter_sha256: String,
    pub fit_statistic_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosMultistartEvidenceV2 {
    pub schema_version: u32,
    pub selected_start_index: usize,
    pub observations: usize,
    pub segments: usize,
    pub required_reproducing_starts: usize,
    pub objective_and_parameter_tolerance: f64,
    pub completed_starts: Vec<PosCompletedStartEvidenceV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosSegmentResultV2 {
    pub segment_id: String,
    pub observations: usize,
    pub objective_contribution: f64,
    pub fit: PosSegmentFullFitV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlsPosV2Result {
    pub method_version: String,
    pub scoring_contract: PosScoringContractV2,
    pub observations: usize,
    pub selected_start_index: usize,
    pub objective: f64,
    pub objective_definition: String,
    pub assignments: Vec<usize>,
    pub segments: Vec<PosSegmentResultV2>,
    pub accepted_moves: usize,
    pub objective_history: Vec<f64>,
    pub starts: Vec<PosStartDiagnosticV2>,
    pub reproducing_start_indices: Vec<usize>,
    /// Additive validation evidence; it is persisted in checked Arrow
    /// sidecars and does not change the selected scientific estimates.
    pub multistart_evidence: PosMultistartEvidenceV2,
}

#[derive(Debug, Clone)]
struct InternalPosRun {
    start_index: usize,
    objective: f64,
    assignments: Vec<usize>,
    fits: Vec<PosSegmentFullFitV2>,
    accepted_moves: usize,
    objective_history: Vec<f64>,
    candidate_failures: Vec<PosCandidateRefitFailureV2>,
}

#[derive(Debug)]
struct InternalPosStartFailureV2 {
    reason: String,
    candidate_refit_failures: Vec<PosCandidateRefitFailureV2>,
    accepted_moves: usize,
    objective_history: Vec<f64>,
}

/// Build the frozen ten-start plan. Nine starts are deterministic seeded
/// presegmentations. The tenth is a same-K FIMIX partition when supplied,
/// otherwise a tenth seeded presegmentation.
pub fn build_pls_pos_start_plan_v2(
    features: &[Vec<f64>],
    segments: usize,
    seed: u64,
    same_k_fimix_assignments: Option<&[usize]>,
) -> Result<Vec<Vec<usize>>, HeterogeneityV2Error> {
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&segments)
        || features.is_empty()
        || features[0].is_empty()
        || features
            .iter()
            .any(|row| row.len() != features[0].len() || row.iter().any(|value| !value.is_finite()))
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS start features must be a finite rectangular matrix for K=2..5".to_string(),
        ));
    }
    if features.len() < segments {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS requires at least one row per segment".to_string(),
        ));
    }
    let mut starts = Vec::with_capacity(10);
    for start_index in 0..10 {
        if start_index == 9 {
            if let Some(fimix) = same_k_fimix_assignments {
                if fimix.len() != features.len()
                    || fimix.iter().any(|segment| *segment >= segments)
                    || fimix.iter().copied().collect::<BTreeSet<_>>().len() != segments
                {
                    return Err(HeterogeneityV2Error::InvalidContract(
                        "same-K FIMIX start has invalid labels or row count".to_string(),
                    ));
                }
                starts.push(fimix.to_vec());
                continue;
            }
        }
        let mut order = (0..features.len()).collect::<Vec<_>>();
        if start_index == 0 {
            order.sort_by(|left, right| {
                feature_projection_key(&features[*left])
                    .total_cmp(&feature_projection_key(&features[*right]))
                    .then(left.cmp(right))
            });
        } else {
            let start_seed = derive_domain_seed_v2(seed, "pls_pos_start", start_index as u64);
            let mut rng = ChaCha20Rng::seed_from_u64(start_seed);
            order.shuffle(&mut rng);
        }
        let mut assignments = vec![0usize; features.len()];
        for (rank, row) in order.into_iter().enumerate() {
            assignments[row] = if start_index == 0 {
                (rank * segments / features.len()).min(segments - 1)
            } else {
                rank % segments
            };
        }
        starts.push(assignments);
    }
    Ok(starts)
}

fn feature_projection_key(row: &[f64]) -> f64 {
    row.iter()
        .enumerate()
        .map(|(column, value)| value * (column + 1) as f64)
        .sum()
}

pub fn fit_pls_pos_published_v2<R: PlsPosFullRefitterV2>(
    start_assignments: &[Vec<usize>],
    config: &PlsPosV2Config,
    refitter: &mut R,
) -> Result<PlsPosV2Result, HeterogeneityV2Error> {
    fit_pls_pos_internal(
        start_assignments,
        config,
        PosScoringContractV2::PublishedP0FullSegmentPls,
        PLS_POS_PUBLISHED_METHOD_VERSION_V2,
        refitter,
    )
}

pub fn fit_pls_pos_destination_scored_interactions_v2<R: PlsPosFullRefitterV2>(
    start_assignments: &[Vec<usize>],
    profile: HeterogeneityInteractionProfileV2,
    config: &PlsPosV2Config,
    refitter: &mut R,
) -> Result<PlsPosV2Result, HeterogeneityV2Error> {
    if profile == HeterogeneityInteractionProfileV2::P0Structural {
        return Err(HeterogeneityV2Error::InvalidContract(
            "the destination-scored extension requires P2 or P23; P0 uses published PLS-POS"
                .to_string(),
        ));
    }
    fit_pls_pos_internal(
        start_assignments,
        config,
        PosScoringContractV2::DestinationScoredInteractions { profile },
        PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2,
        refitter,
    )
}

fn fit_pls_pos_internal<R: PlsPosFullRefitterV2>(
    start_assignments: &[Vec<usize>],
    config: &PlsPosV2Config,
    scoring: PosScoringContractV2,
    method_version: &str,
    refitter: &mut R,
) -> Result<PlsPosV2Result, HeterogeneityV2Error> {
    let observations = validate_pos_contract(start_assignments, config)?;
    let mut diagnostics = Vec::with_capacity(start_assignments.len());
    let mut runs = Vec::new();
    for (start_index, assignments) in start_assignments.iter().enumerate() {
        match run_pos_start(start_index, assignments, config, scoring, refitter) {
            Ok(mut run) => {
                canonicalize_internal_pos_run(&mut run)?;
                diagnostics.push(PosStartDiagnosticV2 {
                    start_index,
                    completed: true,
                    accepted_moves: run.accepted_moves,
                    final_objective: Some(run.objective),
                    failure_reason: None,
                    candidate_refit_failures: run.candidate_failures.clone(),
                    objective_history: run.objective_history.clone(),
                });
                runs.push(run);
            }
            Err(failure) => diagnostics.push(PosStartDiagnosticV2 {
                start_index,
                completed: false,
                accepted_moves: failure.accepted_moves,
                final_objective: None,
                failure_reason: Some(failure.reason),
                candidate_refit_failures: failure.candidate_refit_failures,
                objective_history: failure.objective_history,
            }),
        }
    }
    if runs.is_empty() {
        return Err(HeterogeneityV2Error::NoCompletedPosStart { diagnostics });
    }
    runs.sort_by(|left, right| {
        right
            .objective
            .total_cmp(&left.objective)
            .then(left.start_index.cmp(&right.start_index))
    });
    let best = runs[0].clone();
    let mut reproducing = vec![best.start_index];
    for candidate in runs.iter().skip(1) {
        if (candidate.objective - best.objective).abs() > config.stable_objective_tolerance {
            continue;
        }
        let alignment =
            align_labels_exhaustive_v2(&best.assignments, &candidate.assignments, config.segments)?;
        if !alignment.ambiguous
            && alignment.matched_observations == observations
            && aligned_pos_parameter_difference(&best, candidate, &alignment)
                <= config.stable_objective_tolerance
        {
            reproducing.push(candidate.start_index);
        }
    }
    if reproducing.len() < config.required_reproducing_starts {
        return Err(HeterogeneityV2Error::UnstablePosOptimum {
            reproducing_starts: reproducing.len(),
            required_starts: config.required_reproducing_starts,
        });
    }
    let multistart_evidence = build_pos_multistart_evidence_v2(&best, &runs, config)?;
    let counts = partition_counts(&best.assignments, config.segments);
    let segments = best
        .fits
        .iter()
        .enumerate()
        .map(|(segment, fit)| PosSegmentResultV2 {
            segment_id: format!("segment_{}", segment + 1),
            observations: counts[segment],
            objective_contribution: fit.r_squared.iter().map(|row| row.r_squared).sum(),
            fit: fit.clone(),
        })
        .collect();
    let result = PlsPosV2Result {
        method_version: method_version.to_string(),
        scoring_contract: scoring,
        observations,
        selected_start_index: best.start_index,
        objective: best.objective,
        objective_definition: "unweighted_sum_of_all_endogenous_r_squared_over_all_segments"
            .to_string(),
        assignments: best.assignments,
        segments,
        accepted_moves: best.accepted_moves,
        objective_history: best.objective_history,
        starts: diagnostics,
        reproducing_start_indices: reproducing,
        multistart_evidence,
    };
    validate_pos_multistart_evidence_v2(&result)?;
    Ok(result)
}

fn validate_pos_contract(
    starts: &[Vec<usize>],
    config: &PlsPosV2Config,
) -> Result<usize, HeterogeneityV2Error> {
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&config.segments)
        || starts.len() != 10
        || starts[0].is_empty()
        || config.minimum_segment_size == 0
        || config.maximum_accepted_moves == 0
        || config.required_reproducing_starts < 2
        || config.required_reproducing_starts > starts.len()
        || !config.strict_improvement_tolerance.is_finite()
        || config.strict_improvement_tolerance <= 0.0
        || !config.stable_objective_tolerance.is_finite()
        || config.stable_objective_tolerance <= 0.0
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS requires K=2..5, exactly ten starts, and positive search tolerances"
                .to_string(),
        ));
    }
    let observations = starts[0].len();
    if observations < config.segments.saturating_mul(config.minimum_segment_size) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS rows cannot satisfy the minimum segment size".to_string(),
        ));
    }
    for assignments in starts {
        if assignments.len() != observations
            || assignments
                .iter()
                .any(|segment| *segment >= config.segments)
            || partition_counts(assignments, config.segments)
                .iter()
                .any(|count| *count < config.minimum_segment_size)
        {
            return Err(HeterogeneityV2Error::InvalidContract(
                "every PLS-POS start must contain the same rows and satisfy each segment minimum"
                    .to_string(),
            ));
        }
    }
    Ok(observations)
}

fn run_pos_start<R: PlsPosFullRefitterV2>(
    start_index: usize,
    assignments: &[usize],
    config: &PlsPosV2Config,
    scoring: PosScoringContractV2,
    refitter: &mut R,
) -> Result<InternalPosRun, InternalPosStartFailureV2> {
    let mut current_assignments = assignments.to_vec();
    let (mut objective, mut fits) =
        refit_pos_partition(&current_assignments, config.segments, scoring, refitter).map_err(
            |reason| InternalPosStartFailureV2 {
                reason,
                candidate_refit_failures: Vec::new(),
                accepted_moves: 0,
                objective_history: Vec::new(),
            },
        )?;
    let mut objective_history = vec![objective];
    let mut accepted_moves = 0usize;
    let candidate_failures = Vec::new();
    while accepted_moves < config.maximum_accepted_moves {
        let counts = partition_counts(&current_assignments, config.segments);
        let mut best_candidate: Option<(f64, usize, usize, Vec<PosSegmentFullFitV2>)> = None;
        for observation in 0..current_assignments.len() {
            let source = current_assignments[observation];
            if counts[source] <= config.minimum_segment_size {
                continue;
            }
            for destination in 0..config.segments {
                if destination == source {
                    continue;
                }
                let mut candidate_assignments = current_assignments.clone();
                candidate_assignments[observation] = destination;
                match refit_pos_partition(
                    &candidate_assignments,
                    config.segments,
                    scoring,
                    refitter,
                ) {
                    Ok((candidate_objective, candidate_fits)) => {
                        let beats_current =
                            candidate_objective > objective + config.strict_improvement_tolerance;
                        let beats_best = best_candidate.as_ref().is_none_or(|best| {
                            candidate_objective > best.0 + config.strict_improvement_tolerance
                                || ((candidate_objective - best.0).abs()
                                    <= config.strict_improvement_tolerance
                                    && (observation, destination) < (best.1, best.2))
                        });
                        if beats_current && beats_best {
                            best_candidate = Some((
                                candidate_objective,
                                observation,
                                destination,
                                candidate_fits,
                            ));
                        }
                    }
                    Err(reason) => {
                        let failure = PosCandidateRefitFailureV2 {
                            observation,
                            source_segment: source,
                            destination_segment: destination,
                            reason,
                        };
                        return Err(InternalPosStartFailureV2 {
                            reason: format!(
                                "PLS-POS candidate full refit failed during sweep for observation {}, source segment {}, destination segment {}: {}",
                                failure.observation,
                                failure.source_segment,
                                failure.destination_segment,
                                failure.reason
                            ),
                            candidate_refit_failures: vec![failure],
                            accepted_moves,
                            objective_history,
                        });
                    }
                }
            }
        }
        let Some((candidate_objective, observation, destination, candidate_fits)) = best_candidate
        else {
            break;
        };
        current_assignments[observation] = destination;
        objective = candidate_objective;
        fits = candidate_fits;
        accepted_moves += 1;
        objective_history.push(objective);
    }
    if accepted_moves == config.maximum_accepted_moves {
        return Err(InternalPosStartFailureV2 {
            reason: format!(
                "PLS-POS reached the accepted-move cap {} before a no-improvement sweep",
                config.maximum_accepted_moves
            ),
            candidate_refit_failures: candidate_failures,
            accepted_moves,
            objective_history,
        });
    }
    Ok(InternalPosRun {
        start_index,
        objective,
        assignments: current_assignments,
        fits,
        accepted_moves,
        objective_history,
        candidate_failures,
    })
}

fn refit_pos_partition<R: PlsPosFullRefitterV2>(
    assignments: &[usize],
    segments: usize,
    scoring: PosScoringContractV2,
    refitter: &mut R,
) -> Result<(f64, Vec<PosSegmentFullFitV2>), String> {
    let mut fits = Vec::with_capacity(segments);
    let mut expected_outcomes: Option<BTreeSet<String>> = None;
    let mut expected_parameter_count: Option<usize> = None;
    let mut objective = 0.0;
    for segment in 0..segments {
        let rows = assignments
            .iter()
            .enumerate()
            .filter_map(|(row, assigned)| (*assigned == segment).then_some(row))
            .collect::<Vec<_>>();
        let fit = refitter.refit_segment(segment, &rows, scoring)?;
        validate_pos_fit(&fit, scoring)?;
        let outcomes = fit
            .r_squared
            .iter()
            .map(|row| row.outcome_id.clone())
            .collect::<BTreeSet<_>>();
        if let Some(expected) = &expected_outcomes {
            if expected != &outcomes {
                return Err(
                    "segment refits did not return the same endogenous outcomes".to_string()
                );
            }
        } else {
            expected_outcomes = Some(outcomes);
        }
        if let Some(expected) = expected_parameter_count {
            if expected != fit.parameter_signature.len() {
                return Err("segment refits returned inconsistent parameter signatures".to_string());
            }
        } else {
            expected_parameter_count = Some(fit.parameter_signature.len());
        }
        objective += fit.r_squared.iter().map(|row| row.r_squared).sum::<f64>();
        fits.push(fit);
    }
    if !objective.is_finite() {
        return Err("PLS-POS objective is non-finite".to_string());
    }
    Ok((objective, fits))
}

fn validate_pos_fit(
    fit: &PosSegmentFullFitV2,
    scoring: PosScoringContractV2,
) -> Result<(), String> {
    if fit.r_squared.is_empty()
        || fit.parameter_signature.is_empty()
        || fit
            .parameter_signature
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err("full segment refit omitted outcomes or its parameter signature".to_string());
    }
    let mut outcomes = BTreeSet::new();
    for outcome in &fit.r_squared {
        if outcome.outcome_id.trim().is_empty()
            || !outcomes.insert(outcome.outcome_id.clone())
            || !outcome.r_squared.is_finite()
            || !(-1.0e-10..=1.0 + 1.0e-10).contains(&outcome.r_squared)
        {
            return Err(
                "full segment refit returned invalid or duplicate R-squared values".to_string(),
            );
        }
    }
    if !fit.outcome_fit_audits.is_empty() {
        if fit.outcome_fit_audits.len() != fit.r_squared.len() {
            return Err(
                "PLS-POS outcome audit inventory differs from the R-squared inventory".to_string(),
            );
        }
        let reported = fit
            .r_squared
            .iter()
            .map(|row| (row.outcome_id.as_str(), row.r_squared))
            .collect::<BTreeMap<_, _>>();
        let mut audited = BTreeSet::new();
        for audit in &fit.outcome_fit_audits {
            let rows = audit.source_row_indices.len();
            if audit.outcome_id.trim().is_empty()
                || !audited.insert(audit.outcome_id.as_str())
                || rows < 2
                || audit.observed_scores.len() != rows
                || audit.fitted_scores.len() != rows
                || audit
                    .source_row_indices
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != rows
                || audit
                    .observed_scores
                    .iter()
                    .chain(&audit.fitted_scores)
                    .any(|value| !value.is_finite())
                || !audit.observed_mean.is_finite()
                || !audit.centered_total_sum_of_squares.is_finite()
                || audit.centered_total_sum_of_squares <= f64::EPSILON
            {
                return Err("PLS-POS outcome audit is incomplete or nonfinite".to_string());
            }
            let observed_mean = audit.observed_scores.iter().sum::<f64>() / rows as f64;
            let centered_total_sum_of_squares = audit
                .observed_scores
                .iter()
                .map(|value| (value - observed_mean).powi(2))
                .sum::<f64>();
            let residual_sum_of_squares = audit
                .observed_scores
                .iter()
                .zip(&audit.fitted_scores)
                .map(|(observed, fitted)| (observed - fitted).powi(2))
                .sum::<f64>();
            let reconstructed =
                (1.0 - residual_sum_of_squares / centered_total_sum_of_squares).clamp(0.0, 1.0);
            let scale = 1.0 + centered_total_sum_of_squares.abs();
            if observed_mean.abs() > POS_STANDARDIZED_OUTCOME_MEAN_TOLERANCE_V2
                || (audit.observed_mean - observed_mean).abs() > 1.0e-12
                || (audit.centered_total_sum_of_squares - centered_total_sum_of_squares).abs()
                    > 1.0e-12 * scale
                || reported
                    .get(audit.outcome_id.as_str())
                    .is_none_or(|reported| (reported - reconstructed).abs() > 1.0e-10)
            {
                return Err(
                    "PLS-POS R-squared is not reproducible from centered standardized outcome scores"
                        .to_string(),
                );
            }
        }
    }
    if !fit.receipt.full_segment_pls_refit
        || !fit.receipt.measurement_scores_reestimated
        || !fit.receipt.score_orientation_reapplied
    {
        return Err("PLS-POS requires a complete measurement and structural PLS refit".to_string());
    }
    match scoring {
        PosScoringContractV2::PublishedP0FullSegmentPls => {
            if fit.receipt.method_version != PLS_POS_PUBLISHED_METHOD_VERSION_V2 {
                return Err(
                    "published PLS-POS refit receipt has the wrong method identity".to_string(),
                );
            }
        }
        PosScoringContractV2::DestinationScoredInteractions { .. } => {
            if fit.receipt.method_version
                != PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2
                || !fit.receipt.interaction_stage_one_refit
                || !fit
                    .receipt
                    .interaction_operands_restandardized_within_destination
                || !fit.receipt.interaction_products_rebuilt_within_destination
                || !fit.receipt.joint_structural_equations_refit
            {
                return Err(
                    "destination-scored interaction refit lacks its complete stage-one/product/joint receipt"
                        .to_string(),
                );
            }
        }
    }
    Ok(())
}

fn canonicalize_internal_pos_run(run: &mut InternalPosRun) -> Result<(), HeterogeneityV2Error> {
    let signatures = run
        .fits
        .iter()
        .map(|fit| {
            let mut signature = fit.parameter_signature.clone();
            signature.extend(fit.r_squared.iter().map(|row| row.r_squared));
            signature
        })
        .collect::<Vec<_>>();
    let labeling = canonicalize_labels_by_signature_v2(&run.assignments, &signatures)?;
    let mut order = vec![0usize; signatures.len()];
    for (old, canonical) in labeling.old_to_canonical.iter().copied().enumerate() {
        order[canonical] = old;
    }
    run.assignments = labeling.canonical_assignments;
    run.fits = order.iter().map(|old| run.fits[*old].clone()).collect();
    Ok(())
}

fn aligned_pos_parameter_difference(
    reference: &InternalPosRun,
    candidate: &InternalPosRun,
    alignment: &LabelAlignmentV2,
) -> f64 {
    let mut maximum = 0.0f64;
    for (candidate_segment, reference_segment) in
        alignment.candidate_to_reference.iter().copied().enumerate()
    {
        if reference.fits[reference_segment].parameter_signature.len()
            != candidate.fits[candidate_segment].parameter_signature.len()
        {
            return f64::INFINITY;
        }
        for (left, right) in reference.fits[reference_segment]
            .parameter_signature
            .iter()
            .zip(&candidate.fits[candidate_segment].parameter_signature)
        {
            maximum = maximum.max((left - right).abs());
        }
    }
    maximum
}

/// Ordered digest for a canonical hard partition. Length and every label are
/// encoded as little-endian u64 values so independent languages can replay it.
pub fn heterogeneity_multistart_partition_sha256_v2(
    assignments: &[usize],
) -> Result<String, HeterogeneityV2Error> {
    if assignments.is_empty() {
        return Err(HeterogeneityV2Error::InvalidContract(
            "multistart partition digest requires a nonempty partition".into(),
        ));
    }
    let mut digest = Sha256::new();
    digest.update(HETEROGENEITY_MULTISTART_PARTITION_DIGEST_DOMAIN_V2);
    digest.update((assignments.len() as u64).to_le_bytes());
    for assignment in assignments {
        digest.update((*assignment as u64).to_le_bytes());
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Ordered digest for a finite ragged matrix. Both dimensions are retained;
/// this prevents a reshape from preserving the same byte stream.
pub fn heterogeneity_multistart_matrix_sha256_v2(
    domain: &[u8],
    rows: &[Vec<f64>],
) -> Result<String, HeterogeneityV2Error> {
    if rows.is_empty()
        || domain.is_empty()
        || rows
            .iter()
            .any(|row| row.is_empty() || row.iter().any(|value| !value.is_finite()))
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "multistart matrix digest requires a nonempty finite matrix and domain".into(),
        ));
    }
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update((rows.len() as u64).to_le_bytes());
    for row in rows {
        digest.update((row.len() as u64).to_le_bytes());
        for value in row {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn heterogeneity_multistart_fit_statistic_sha256_v2(
    value: f64,
) -> Result<String, HeterogeneityV2Error> {
    if !value.is_finite() {
        return Err(HeterogeneityV2Error::InvalidContract(
            "multistart fit-statistic digest requires a finite value".into(),
        ));
    }
    let mut digest = Sha256::new();
    digest.update(HETEROGENEITY_MULTISTART_FIT_STATISTIC_DIGEST_DOMAIN_V2);
    digest.update(value.to_bits().to_le_bytes());
    Ok(format!("{:x}", digest.finalize()))
}

fn build_pos_multistart_evidence_v2(
    best: &InternalPosRun,
    completed: &[InternalPosRun],
    config: &PlsPosV2Config,
) -> Result<PosMultistartEvidenceV2, HeterogeneityV2Error> {
    let completed_starts = completed
        .iter()
        .map(|run| {
            let signatures = run
                .fits
                .iter()
                .map(|fit| fit.parameter_signature.clone())
                .collect::<Vec<_>>();
            Ok(PosCompletedStartEvidenceV2 {
                start_index: run.start_index,
                final_objective: run.objective,
                canonical_assignments: run.assignments.clone(),
                canonical_parameter_signatures: signatures.clone(),
                partition_sha256: heterogeneity_multistart_partition_sha256_v2(&run.assignments)?,
                parameter_sha256: heterogeneity_multistart_matrix_sha256_v2(
                    HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2,
                    &signatures,
                )?,
                fit_statistic_sha256: heterogeneity_multistart_fit_statistic_sha256_v2(
                    run.objective,
                )?,
            })
        })
        .collect::<Result<Vec<_>, HeterogeneityV2Error>>()?;
    Ok(PosMultistartEvidenceV2 {
        schema_version: HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2,
        selected_start_index: best.start_index,
        observations: best.assignments.len(),
        segments: best.fits.len(),
        required_reproducing_starts: config.required_reproducing_starts,
        objective_and_parameter_tolerance: config.stable_objective_tolerance,
        completed_starts,
    })
}

/// Replays PLS-POS exact-partition, objective, and parameter-signature
/// reproducibility without trusting the retained reproducing-start list.
pub fn validate_pos_multistart_evidence_v2(
    result: &PlsPosV2Result,
) -> Result<(), HeterogeneityV2Error> {
    let evidence = &result.multistart_evidence;
    let segments = result.segments.len();
    let completed_indices = result
        .starts
        .iter()
        .filter(|start| start.completed)
        .map(|start| start.start_index)
        .collect::<BTreeSet<_>>();
    let retained_indices = evidence
        .completed_starts
        .iter()
        .map(|start| start.start_index)
        .collect::<BTreeSet<_>>();
    if evidence.schema_version != HETEROGENEITY_MULTISTART_EVIDENCE_SCHEMA_VERSION_V2
        || evidence.selected_start_index != result.selected_start_index
        || evidence.observations != result.observations
        || evidence.segments != segments
        || evidence.required_reproducing_starts < 2
        || !evidence.objective_and_parameter_tolerance.is_finite()
        || evidence.objective_and_parameter_tolerance <= 0.0
        || evidence.completed_starts.is_empty()
        || evidence.completed_starts[0].start_index != result.selected_start_index
        || retained_indices.len() != evidence.completed_starts.len()
        || retained_indices != completed_indices
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS multistart evidence identity or completed-start inventory is invalid".into(),
        ));
    }
    let selected_signatures = result
        .segments
        .iter()
        .map(|segment| segment.fit.parameter_signature.clone())
        .collect::<Vec<_>>();
    let signature_widths = selected_signatures.iter().map(Vec::len).collect::<Vec<_>>();
    if signature_widths.iter().any(|width| *width == 0) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS multistart evidence has an empty parameter signature".into(),
        ));
    }
    for retained in &evidence.completed_starts {
        let diagnostic = result
            .starts
            .iter()
            .find(|start| start.start_index == retained.start_index)
            .filter(|start| start.completed)
            .ok_or_else(|| {
                HeterogeneityV2Error::InvalidContract(
                    "PLS-POS multistart evidence references an incomplete start".into(),
                )
            })?;
        if diagnostic.final_objective.map(f64::to_bits) != Some(retained.final_objective.to_bits())
            || !retained.final_objective.is_finite()
            || retained.canonical_assignments.len() != result.observations
            || retained
                .canonical_assignments
                .iter()
                .any(|segment| *segment >= segments)
            || retained.canonical_parameter_signatures.len() != segments
            || retained
                .canonical_parameter_signatures
                .iter()
                .enumerate()
                .any(|(segment, values)| {
                    values.len() != signature_widths[segment]
                        || values.iter().any(|value| !value.is_finite())
                })
            || retained.partition_sha256
                != heterogeneity_multistart_partition_sha256_v2(&retained.canonical_assignments)?
            || retained.parameter_sha256
                != heterogeneity_multistart_matrix_sha256_v2(
                    HETEROGENEITY_MULTISTART_PARAMETER_DIGEST_DOMAIN_V2,
                    &retained.canonical_parameter_signatures,
                )?
            || retained.fit_statistic_sha256
                != heterogeneity_multistart_fit_statistic_sha256_v2(retained.final_objective)?
        {
            return Err(HeterogeneityV2Error::InvalidContract(format!(
                "PLS-POS completed-start evidence {} is malformed or digest-inconsistent",
                retained.start_index
            )));
        }
    }
    let selected = &evidence.completed_starts[0];
    if selected.canonical_assignments != result.assignments
        || selected.canonical_parameter_signatures != selected_signatures
        || selected.final_objective.to_bits() != result.objective.to_bits()
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS selected-start evidence differs from the published selected solution".into(),
        ));
    }
    let mut reproducing = vec![selected.start_index];
    for candidate in evidence.completed_starts.iter().skip(1) {
        if (candidate.final_objective - selected.final_objective).abs()
            > evidence.objective_and_parameter_tolerance
        {
            continue;
        }
        let alignment = align_labels_exhaustive_v2(
            &selected.canonical_assignments,
            &candidate.canonical_assignments,
            segments,
        )?;
        let mut maximum_parameter_difference = 0.0_f64;
        for (candidate_segment, reference_segment) in
            alignment.candidate_to_reference.iter().copied().enumerate()
        {
            for (reference, observed) in selected.canonical_parameter_signatures[reference_segment]
                .iter()
                .zip(&candidate.canonical_parameter_signatures[candidate_segment])
            {
                maximum_parameter_difference =
                    maximum_parameter_difference.max((reference - observed).abs());
            }
        }
        if !alignment.ambiguous
            && alignment.matched_observations == result.observations
            && maximum_parameter_difference <= evidence.objective_and_parameter_tolerance
        {
            reproducing.push(candidate.start_index);
        }
    }
    if reproducing != result.reproducing_start_indices
        || reproducing.len() < evidence.required_reproducing_starts
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "PLS-POS reproducing starts do not replay from completed-start evidence".into(),
        ));
    }
    Ok(())
}

fn partition_counts(assignments: &[usize], segments: usize) -> Vec<usize> {
    let mut counts = vec![0usize; segments];
    for segment in assignments {
        counts[*segment] += 1;
    }
    counts
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityBootstrapAlgorithmV2 {
    FimixPlsV2,
    PlsPosPublishedV2,
    PlsPosDestinationScoredInteractionsV2,
}

impl HeterogeneityBootstrapAlgorithmV2 {
    fn stable_id(self) -> &'static str {
        match self {
            Self::FimixPlsV2 => FIMIX_PLS_METHOD_VERSION_V2,
            Self::PlsPosPublishedV2 => PLS_POS_PUBLISHED_METHOD_VERSION_V2,
            Self::PlsPosDestinationScoredInteractionsV2 => {
                PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityBootstrapPlanV2 {
    pub algorithm: HeterogeneityBootstrapAlgorithmV2,
    pub fixed_classes_or_segments: usize,
    pub requested_replicates: usize,
    pub master_seed: u64,
    pub confidence_level: f64,
    pub minimum_usable_share: f64,
}

impl HeterogeneityBootstrapPlanV2 {
    pub fn publication_default(
        algorithm: HeterogeneityBootstrapAlgorithmV2,
        fixed_classes_or_segments: usize,
    ) -> Self {
        Self {
            algorithm,
            fixed_classes_or_segments,
            requested_replicates: 5_000,
            master_seed: 42,
            confidence_level: 0.95,
            minimum_usable_share: 0.90,
        }
    }

    pub fn interactive_default(
        algorithm: HeterogeneityBootstrapAlgorithmV2,
        fixed_classes_or_segments: usize,
    ) -> Self {
        let mut plan = Self::publication_default(algorithm, fixed_classes_or_segments);
        plan.requested_replicates = 1_000;
        plan
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityBootstrapReplicateStatusV2 {
    Usable,
    FitFailed,
    LabelAmbiguous,
    LabelNotMutualMajority,
    ComparabilityFailed,
    NonFiniteTarget,
    Cancelled,
}

impl HeterogeneityBootstrapReplicateStatusV2 {
    fn stable_id(self) -> &'static str {
        match self {
            Self::Usable => "usable",
            Self::FitFailed => "fit_failed",
            Self::LabelAmbiguous => "label_ambiguous",
            Self::LabelNotMutualMajority => "label_not_mutual_majority",
            Self::ComparabilityFailed => "comparability_failed",
            Self::NonFiniteTarget => "nonfinite_target",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityBootstrapLedgerEntryV2 {
    pub replicate_index: usize,
    pub seed: u64,
    pub status: HeterogeneityBootstrapReplicateStatusV2,
    pub fit_statistic: Option<f64>,
    pub label_alignment: Option<LabelAlignmentV2>,
    pub target_payload_sha256: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeterogeneityBootstrapQualificationV2 {
    Qualified,
    InsufficientUsableReplicates,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HeterogeneityBootstrapLedgerSummaryV2 {
    pub method_version: String,
    pub algorithm: HeterogeneityBootstrapAlgorithmV2,
    pub fixed_classes_or_segments: usize,
    pub requested_replicates: usize,
    pub attempted_replicates: usize,
    pub usable_replicates: usize,
    pub required_usable_replicates: usize,
    pub failed_replicates: usize,
    pub retry_policy: String,
    pub interval_method: String,
    pub confidence_level: f64,
    pub qualification: HeterogeneityBootstrapQualificationV2,
    pub failure_counts: BTreeMap<String, usize>,
}

pub fn heterogeneity_bootstrap_replicate_seed_v2(
    plan: &HeterogeneityBootstrapPlanV2,
    replicate_index: usize,
) -> u64 {
    derive_domain_seed_v2(
        plan.master_seed,
        plan.algorithm.stable_id(),
        replicate_index as u64,
    )
}

/// Hashes one ordered, finite target vector using an explicitly versioned
/// binary encoding. JSON rendering is intentionally excluded so Rust and
/// independent comparators can reproduce the identity byte-for-byte.
pub fn heterogeneity_target_payload_sha256_v2(
    values: &[f64],
) -> Result<String, HeterogeneityV2Error> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(HeterogeneityV2Error::InvalidContract(
            "heterogeneity target payload must be non-empty and finite".to_string(),
        ));
    }
    let mut digest = Sha256::new();
    digest.update(HETEROGENEITY_TARGET_PAYLOAD_DIGEST_DOMAIN_V2);
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        digest.update(value.to_bits().to_le_bytes());
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .concat())
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Validate a fixed-K, no-retry bootstrap ledger. Usable rows require a target
/// digest, a finite fit statistic, and an unambiguous mutual-majority alignment.
pub fn summarize_heterogeneity_bootstrap_ledger_v2(
    plan: &HeterogeneityBootstrapPlanV2,
    entries: &[HeterogeneityBootstrapLedgerEntryV2],
) -> Result<HeterogeneityBootstrapLedgerSummaryV2, HeterogeneityV2Error> {
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS)
        .contains(&plan.fixed_classes_or_segments)
        || !(500..=10_000).contains(&plan.requested_replicates)
        || !plan.confidence_level.is_finite()
        || !(0.50..1.0).contains(&plan.confidence_level)
        || !plan.minimum_usable_share.is_finite()
        || !(0.90..=1.0).contains(&plan.minimum_usable_share)
        || entries.len() > plan.requested_replicates
    {
        return Err(HeterogeneityV2Error::InvalidContract(
            "bootstrap requires fixed K=2..5, 500..10000 draws, and at least 90% usable"
                .to_string(),
        ));
    }
    let mut indices = BTreeSet::new();
    let mut usable = 0usize;
    let mut cancelled = false;
    let mut failure_counts = BTreeMap::<String, usize>::new();
    for entry in entries {
        if entry.replicate_index >= plan.requested_replicates
            || !indices.insert(entry.replicate_index)
            || entry.seed != heterogeneity_bootstrap_replicate_seed_v2(plan, entry.replicate_index)
        {
            return Err(HeterogeneityV2Error::InvalidContract(
                "bootstrap ledger has a duplicate/out-of-range index or wrong derived seed"
                    .to_string(),
            ));
        }
        let status_id = entry.status.stable_id().to_string();
        if entry.status == HeterogeneityBootstrapReplicateStatusV2::Usable {
            let valid_alignment = entry.label_alignment.as_ref().is_some_and(|alignment| {
                validate_retained_label_alignment_v2(alignment).is_ok()
                    && !alignment.ambiguous
                    && alignment.mutual_majority
            });
            if entry.fit_statistic.is_none_or(|value| !value.is_finite())
                || entry
                    .target_payload_sha256
                    .as_ref()
                    .is_none_or(|digest| !is_lower_hex_sha256(digest))
                || !valid_alignment
                || entry.failure_reason.is_some()
            {
                return Err(HeterogeneityV2Error::InvalidContract(format!(
                    "usable bootstrap replicate {} lacks finite targets or valid label alignment",
                    entry.replicate_index
                )));
            }
            usable += 1;
        } else {
            if entry.fit_statistic.is_some()
                || entry.label_alignment.is_some()
                || entry.target_payload_sha256.is_some()
                || entry
                    .failure_reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
            {
                return Err(HeterogeneityV2Error::InvalidContract(format!(
                    "failed bootstrap replicate {} has incoherent retained fields",
                    entry.replicate_index
                )));
            }
            *failure_counts.entry(status_id).or_default() += 1;
            cancelled |= entry.status == HeterogeneityBootstrapReplicateStatusV2::Cancelled;
        }
    }
    let required = (plan.minimum_usable_share * plan.requested_replicates as f64).ceil() as usize;
    let qualification = if cancelled || entries.len() < plan.requested_replicates {
        HeterogeneityBootstrapQualificationV2::Cancelled
    } else if usable >= required {
        HeterogeneityBootstrapQualificationV2::Qualified
    } else {
        HeterogeneityBootstrapQualificationV2::InsufficientUsableReplicates
    };
    Ok(HeterogeneityBootstrapLedgerSummaryV2 {
        method_version: HETEROGENEITY_BOOTSTRAP_LEDGER_METHOD_VERSION_V2.to_string(),
        algorithm: plan.algorithm,
        fixed_classes_or_segments: plan.fixed_classes_or_segments,
        requested_replicates: plan.requested_replicates,
        attempted_replicates: entries.len(),
        usable_replicates: usable,
        required_usable_replicates: required,
        failed_replicates: entries.len() - usable,
        retry_policy: "none_one_attempt_per_index".to_string(),
        interval_method: "type_7_two_sided_percentile".to_string(),
        confidence_level: plan.confidence_level,
        qualification,
        failure_counts,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosPairwiseCompositionalInvarianceV1 {
    pub left_segment: usize,
    pub right_segment: usize,
    pub passed: bool,
    pub permutation_p_value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosPairwiseStep3EqualityV1 {
    pub left_segment: usize,
    pub right_segment: usize,
    pub mean_equality_passed: bool,
    pub variance_equality_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosConstructComparabilityEvidenceV1 {
    pub construct_id: String,
    pub configural_identity_passed: bool,
    pub compositional_invariance: Vec<PosPairwiseCompositionalInvarianceV1>,
    pub step3_equality: Vec<PosPairwiseStep3EqualityV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosCommonMetricGateInputV1 {
    pub pooled_metric_id: String,
    pub pooled_metric_sha256: String,
    pub segments: usize,
    pub applied_identically_to_all_segments: bool,
    pub required_construct_ids: Vec<String>,
    pub evidence: Vec<PosConstructComparabilityEvidenceV1>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PosCommonMetricBlockerCodeV1 {
    InvalidCommonMetricReceipt,
    MissingConstructEvidence,
    DuplicateConstructEvidence,
    ConfiguralIdentityFailed,
    MissingPairwiseCompositionalEvidence,
    DuplicatePairwiseCompositionalEvidence,
    CompositionalInvarianceFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PosCommonMetricBlockerV1 {
    pub code: PosCommonMetricBlockerCodeV1,
    pub construct_id: Option<String>,
    pub left_segment: Option<usize>,
    pub right_segment: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PosCommonMetricGateStatusV1 {
    Passed,
    DescriptiveOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PosCommonMetricGateResultV1 {
    pub method_version: String,
    pub status: PosCommonMetricGateStatusV1,
    pub inferential_gamma_delta_slope_contrasts_allowed: bool,
    pub required_construct_ids: Vec<String>,
    pub blockers: Vec<PosCommonMetricBlockerV1>,
    pub step3_failed_mean_comparisons: usize,
    pub step3_failed_variance_comparisons: usize,
    pub step3_required_for_standardized_path_comparison: bool,
}

/// Evaluate the mandatory common-scale gate for destination-scored interaction
/// contrasts. Step 3 is retained descriptively but does not block standardized
/// path/gamma/delta comparisons once configural and compositional invariance
/// pass.
pub fn evaluate_pos_common_metric_gate_v1(
    input: &PosCommonMetricGateInputV1,
) -> PosCommonMetricGateResultV1 {
    let mut blockers = Vec::new();
    if !(MIN_CLASSES_OR_SEGMENTS..=MAX_CLASSES_OR_SEGMENTS).contains(&input.segments)
        || input.pooled_metric_id.trim().is_empty()
        || input.pooled_metric_sha256.trim().is_empty()
        || !input.applied_identically_to_all_segments
    {
        blockers.push(PosCommonMetricBlockerV1 {
            code: PosCommonMetricBlockerCodeV1::InvalidCommonMetricReceipt,
            construct_id: None,
            left_segment: None,
            right_segment: None,
            message: "one pooled metric must be applied identically to every aligned segment"
                .to_string(),
        });
    }
    let required = input
        .required_construct_ids
        .iter()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect::<BTreeSet<_>>();
    if required.len() != input.required_construct_ids.len() || required.is_empty() {
        blockers.push(PosCommonMetricBlockerV1 {
            code: PosCommonMetricBlockerCodeV1::InvalidCommonMetricReceipt,
            construct_id: None,
            left_segment: None,
            right_segment: None,
            message: "required construct identities must be unique and non-empty".to_string(),
        });
    }
    let mut evidence_by_construct = BTreeMap::<String, &PosConstructComparabilityEvidenceV1>::new();
    for evidence in &input.evidence {
        if evidence_by_construct
            .insert(evidence.construct_id.clone(), evidence)
            .is_some()
        {
            blockers.push(PosCommonMetricBlockerV1 {
                code: PosCommonMetricBlockerCodeV1::DuplicateConstructEvidence,
                construct_id: Some(evidence.construct_id.clone()),
                left_segment: None,
                right_segment: None,
                message: "construct comparability evidence is duplicated".to_string(),
            });
        }
    }
    let expected_pairs = (0..input.segments)
        .flat_map(|left| (left + 1..input.segments).map(move |right| (left, right)))
        .collect::<BTreeSet<_>>();
    for construct_id in &required {
        let Some(evidence) = evidence_by_construct.get(construct_id) else {
            blockers.push(PosCommonMetricBlockerV1 {
                code: PosCommonMetricBlockerCodeV1::MissingConstructEvidence,
                construct_id: Some(construct_id.clone()),
                left_segment: None,
                right_segment: None,
                message: "required construct has no common-metric evidence".to_string(),
            });
            continue;
        };
        if !evidence.configural_identity_passed {
            blockers.push(PosCommonMetricBlockerV1 {
                code: PosCommonMetricBlockerCodeV1::ConfiguralIdentityFailed,
                construct_id: Some(construct_id.clone()),
                left_segment: None,
                right_segment: None,
                message: "configural identity failed".to_string(),
            });
        }
        let mut observed_pairs = BTreeMap::<(usize, usize), bool>::new();
        for pair in &evidence.compositional_invariance {
            let normalized = if pair.left_segment < pair.right_segment {
                (pair.left_segment, pair.right_segment)
            } else {
                (pair.right_segment, pair.left_segment)
            };
            if !expected_pairs.contains(&normalized) {
                blockers.push(PosCommonMetricBlockerV1 {
                    code: PosCommonMetricBlockerCodeV1::MissingPairwiseCompositionalEvidence,
                    construct_id: Some(construct_id.clone()),
                    left_segment: Some(pair.left_segment),
                    right_segment: Some(pair.right_segment),
                    message: "compositional evidence names an invalid segment pair".to_string(),
                });
                continue;
            }
            if observed_pairs.insert(normalized, pair.passed).is_some() {
                blockers.push(PosCommonMetricBlockerV1 {
                    code: PosCommonMetricBlockerCodeV1::DuplicatePairwiseCompositionalEvidence,
                    construct_id: Some(construct_id.clone()),
                    left_segment: Some(normalized.0),
                    right_segment: Some(normalized.1),
                    message: "pairwise compositional evidence is duplicated".to_string(),
                });
            }
        }
        for pair in &expected_pairs {
            match observed_pairs.get(pair) {
                None => blockers.push(PosCommonMetricBlockerV1 {
                    code: PosCommonMetricBlockerCodeV1::MissingPairwiseCompositionalEvidence,
                    construct_id: Some(construct_id.clone()),
                    left_segment: Some(pair.0),
                    right_segment: Some(pair.1),
                    message: "pairwise compositional evidence is missing".to_string(),
                }),
                Some(false) => blockers.push(PosCommonMetricBlockerV1 {
                    code: PosCommonMetricBlockerCodeV1::CompositionalInvarianceFailed,
                    construct_id: Some(construct_id.clone()),
                    left_segment: Some(pair.0),
                    right_segment: Some(pair.1),
                    message: "compositional invariance failed".to_string(),
                }),
                Some(true) => {}
            }
        }
    }
    blockers.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then(left.construct_id.cmp(&right.construct_id))
            .then(left.left_segment.cmp(&right.left_segment))
            .then(left.right_segment.cmp(&right.right_segment))
    });
    let step3_failed_mean_comparisons = input
        .evidence
        .iter()
        .flat_map(|evidence| &evidence.step3_equality)
        .filter(|pair| !pair.mean_equality_passed)
        .count();
    let step3_failed_variance_comparisons = input
        .evidence
        .iter()
        .flat_map(|evidence| &evidence.step3_equality)
        .filter(|pair| !pair.variance_equality_passed)
        .count();
    let passed = blockers.is_empty();
    PosCommonMetricGateResultV1 {
        method_version: POS_COMMON_METRIC_COMPARABILITY_METHOD_VERSION_V1.to_string(),
        status: if passed {
            PosCommonMetricGateStatusV1::Passed
        } else {
            PosCommonMetricGateStatusV1::DescriptiveOnly
        },
        inferential_gamma_delta_slope_contrasts_allowed: passed,
        required_construct_ids: required.into_iter().collect(),
        blockers,
        step3_failed_mean_comparisons,
        step3_failed_variance_comparisons,
        step3_required_for_standardized_path_comparison: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn standardize(values: &[f64]) -> Vec<f64> {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let sample_sd = (values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (values.len() - 1) as f64)
            .sqrt();
        values
            .iter()
            .map(|value| (value - mean) / sample_sd)
            .collect()
    }

    fn fixture_metric(observations: usize) -> PooledStandardizedMetricReceiptV2 {
        PooledStandardizedMetricReceiptV2 {
            metric_id: "fixture_pooled_metric".to_string(),
            source_sha256: "fixture_sha256".to_string(),
            observation_count: observations,
            scores_standardized_once_on_pooled_rows: true,
            products_standardized_once_on_pooled_rows: true,
        }
    }

    fn two_class_regression_fixture() -> StandardizedFimixInputV2 {
        let observations = 160usize;
        let raw_x = (0..observations)
            .map(|row| (row % 80) as f64 - 39.5)
            .collect::<Vec<_>>();
        let x = standardize(&raw_x);
        let raw_y = (0..observations)
            .map(|row| {
                let noise = 0.18 * ((row * 17 + 3) as f64).sin();
                if row < 80 {
                    0.85 * x[row] + noise
                } else {
                    -0.75 * x[row] + noise
                }
            })
            .collect::<Vec<_>>();
        let y = standardize(&raw_y);
        StandardizedFimixInputV2 {
            interaction_profile: HeterogeneityInteractionProfileV2::P0Structural,
            metric: fixture_metric(observations),
            equations: vec![StandardizedStructuralEquationV2 {
                equation_id: "x_to_y".to_string(),
                outcome_id: "y".to_string(),
                predictor_ids: vec!["x".to_string()],
                design: x.into_iter().map(|value| vec![value]).collect(),
                outcome: y,
                include_intercept: true,
            }],
        }
    }

    #[test]
    fn log_sum_exp_is_stable_for_large_negative_values() {
        let actual = log_sum_exp_v2(&[-1_000.0, -1_001.0]).unwrap();
        let expected = -1_000.0 + (1.0 + (-1.0f64).exp()).ln();
        assert_abs_diff_eq!(actual, expected, epsilon = 1.0e-12);
        assert!(log_sum_exp_v2(&[]).is_err());
        assert!(log_sum_exp_v2(&[f64::NAN]).is_err());
    }

    #[test]
    fn fimix_em_returns_monotone_likelihood_posteriors_and_full_criteria() {
        let input = two_class_regression_fixture();
        let mut config = FimixPlsV2Config::for_classes(2);
        config.starts = 10;
        config.max_iterations = 2_000;
        config.relative_log_likelihood_tolerance = 1.0e-9;
        config.optimum_maximum_coefficient_difference = 1.0e-4;
        config.optimum_mean_posterior_difference = 1.0e-3;
        let result = fit_fimix_pls_v2(&input, &config).unwrap();
        validate_fimix_multistart_evidence_v2(&result).unwrap();
        assert_eq!(
            result.multistart_evidence.completed_starts.len(),
            result.starts.iter().filter(|start| start.converged).count()
        );
        let mut digest_tampered = result.clone();
        digest_tampered.multistart_evidence.completed_starts[0].posterior_sha256 = "0".repeat(64);
        assert!(validate_fimix_multistart_evidence_v2(&digest_tampered).is_err());
        let mut stability_tampered = result.clone();
        stability_tampered.stability.stable = !stability_tampered.stability.stable;
        assert!(validate_fimix_multistart_evidence_v2(&stability_tampered).is_err());

        assert_eq!(result.method_version, FIMIX_PLS_METHOD_VERSION_V2);
        assert_eq!(result.classes.len(), 2);
        assert_eq!(result.posteriors.len(), 160);
        assert!(result.stability.stable);
        assert!(
            result
                .posteriors
                .iter()
                .all(|row| (row.iter().sum::<f64>() - 1.0).abs() <= 1.0e-10)
        );
        assert!(result.starts.iter().all(|start| {
            start
                .trace
                .windows(2)
                .all(|window| window[1].log_likelihood >= window[0].log_likelihood)
        }));
        let mut slopes = result
            .classes
            .iter()
            .map(|class| class.equations[0].coefficients[1].estimate)
            .collect::<Vec<_>>();
        slopes.sort_by(f64::total_cmp);
        assert!(slopes[0] < -0.5);
        assert!(slopes[1] > 0.5);
        assert!(
            result
                .classes
                .iter()
                .all(|class| class.equations[0].residual_variance > config.residual_variance_floor)
        );
        let deviance = -2.0 * result.log_likelihood;
        let parameters = result.criteria.parameter_count as f64;
        assert_abs_diff_eq!(result.criteria.aic, deviance + 2.0 * parameters);
        assert_abs_diff_eq!(result.criteria.aic3, deviance + 3.0 * parameters);
        assert_abs_diff_eq!(result.criteria.aic4, deviance + 4.0 * parameters);
        assert_abs_diff_eq!(
            result.criteria.caic,
            deviance + parameters * ((160.0f64).ln() + 1.0)
        );
        assert!((0.0..=1.0).contains(&result.entropy.normalized_certainty));
    }

    #[test]
    fn fimix_failure_boundary_likelihood_decrease_is_typed() {
        let retained = validate_fimix_likelihood_step_v2(
            -100.0,
            -100.0 - 5.0e-8,
            FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2,
            7,
        )
        .unwrap();
        assert!(matches!(
            retained,
            FimixLikelihoodStepV2::RetainPriorAcceptedState { .. }
        ));

        let material = validate_fimix_likelihood_step_v2(
            -100.0,
            -100.0 - 1.0e-5,
            FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2,
            8,
        )
        .unwrap_err();
        assert_eq!(material.code, FimixStartFailureCodeV2::LikelihoodDecrease);
        assert_eq!(material.iteration, 8);
        assert!(material.maximum_decrease > 0.0);
    }

    #[test]
    fn fimix_failure_boundary_nonfinite_update_is_typed() {
        let error = validate_fimix_likelihood_step_v2(
            -100.0,
            f64::NAN,
            FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2,
            9,
        )
        .unwrap_err();
        assert_eq!(error.code, FimixStartFailureCodeV2::NonFinite);
        assert_eq!(error.iteration, 9);

        let input = two_class_regression_fixture();
        let config = FimixPlsV2Config::for_classes(2);
        let posteriors = vec![vec![0.5, 0.5]; input.metric.observation_count];
        let mut classes = fimix_m_step(&input, &posteriors, &config, 20, 0).unwrap();
        classes[0].equations[0].residual_variance = f64::NAN;
        let e_step_error = fimix_e_step(&input, &classes, 10).unwrap_err();
        assert_eq!(e_step_error.code, FimixStartFailureCodeV2::NonFinite);
        assert_eq!(e_step_error.iteration, 10);
    }

    #[test]
    fn fimix_failure_boundary_responsibility_collapse_is_typed() {
        let input = two_class_regression_fixture();
        let config = FimixPlsV2Config::for_classes(2);
        let posteriors = vec![vec![1.0, 0.0]; input.metric.observation_count];
        let error = fimix_m_step(&input, &posteriors, &config, 20, 11).unwrap_err();
        assert_eq!(error.code, FimixStartFailureCodeV2::CollapsedClass);
        assert_eq!(error.iteration, 11);
        assert_eq!(error.effective_sizes, vec![160.0, 0.0]);
    }

    #[test]
    fn fimix_failure_boundary_dynamic_minimum_and_share_are_enforced() {
        let mut input = two_class_regression_fixture();
        let config = FimixPlsV2Config::for_classes(2);
        assert_eq!(minimum_fimix_class_size(&input, &config, 160), 20);
        assert_eq!(minimum_fimix_class_size(&input, &config, 1_000), 50);

        input.equations[0].predictor_ids =
            (0..25).map(|index| format!("predictor_{index}")).collect();
        assert_eq!(minimum_fimix_class_size(&input, &config, 160), 27);

        let input = two_class_regression_fixture();
        let below_share = vec![vec![0.951, 0.049]; input.metric.observation_count];
        let error = fimix_m_step(&input, &below_share, &config, 1, 14).unwrap_err();
        assert_eq!(error.code, FimixStartFailureCodeV2::CollapsedClass);
        assert_eq!(error.iteration, 14);
        assert!(error.effective_sizes[1] / 160.0 < config.minimum_class_share);
    }

    #[test]
    fn fimix_failure_boundary_variance_collapse_is_typed() {
        let mut input = two_class_regression_fixture();
        input.equations[0].outcome = input.equations[0]
            .design
            .iter()
            .map(|row| 0.75 + 1.25 * row[0])
            .collect();
        let config = FimixPlsV2Config::for_classes(2);
        let posteriors = vec![vec![0.5, 0.5]; input.metric.observation_count];
        let error = fimix_m_step(&input, &posteriors, &config, 20, 12).unwrap_err();
        assert_eq!(error.code, FimixStartFailureCodeV2::VarianceCollapse);
        assert_eq!(error.iteration, 12);
    }

    #[test]
    fn fimix_failure_boundary_singular_design_is_typed() {
        let base = two_class_regression_fixture();
        let input = StandardizedFimixInputV2 {
            interaction_profile: HeterogeneityInteractionProfileV2::P0Structural,
            metric: base.metric,
            equations: vec![StandardizedStructuralEquationV2 {
                equation_id: "singular_boundary".to_string(),
                outcome_id: "y".to_string(),
                predictor_ids: vec!["x".to_string(), "x_duplicate".to_string()],
                design: base.equations[0]
                    .design
                    .iter()
                    .map(|row| vec![row[0], row[0]])
                    .collect(),
                outcome: base.equations[0].outcome.clone(),
                include_intercept: false,
            }],
        };
        let config = FimixPlsV2Config::for_classes(2);
        let posteriors = vec![vec![0.5, 0.5]; input.metric.observation_count];
        let error = fimix_m_step(&input, &posteriors, &config, 20, 13).unwrap_err();
        assert_eq!(error.code, FimixStartFailureCodeV2::RankDeficient);
        assert_eq!(error.iteration, 13);
    }

    #[test]
    fn fimix_rejects_an_unfrozen_likelihood_decrease_tolerance() {
        let input = two_class_regression_fixture();
        let mut config = FimixPlsV2Config::for_classes(2);
        config.starts = 10;
        validate_fimix_contract(&input, &config).unwrap();

        config.likelihood_decrease_tolerance = 2.0e-9;
        assert!(matches!(
            validate_fimix_contract(&input, &config),
            Err(HeterogeneityV2Error::InvalidContract(message))
                if message.contains("numerical tolerances")
        ));
    }

    #[test]
    fn fimix_reports_rank_failure_for_duplicate_standardized_columns() {
        let base = two_class_regression_fixture();
        let x = base.equations[0]
            .design
            .iter()
            .map(|row| row[0])
            .collect::<Vec<_>>();
        let input = StandardizedFimixInputV2 {
            interaction_profile: HeterogeneityInteractionProfileV2::P0Structural,
            metric: base.metric,
            equations: vec![StandardizedStructuralEquationV2 {
                equation_id: "rank_failure".to_string(),
                outcome_id: "y".to_string(),
                predictor_ids: vec!["x".to_string(), "x_duplicate".to_string()],
                design: x.into_iter().map(|value| vec![value, value]).collect(),
                outcome: base.equations[0].outcome.clone(),
                include_intercept: false,
            }],
        };
        let mut config = FimixPlsV2Config::for_classes(2);
        config.starts = 10;
        let error = fit_fimix_pls_v2(&input, &config).unwrap_err();
        match error {
            HeterogeneityV2Error::NoConvergedFimixStart { diagnostics } => {
                assert_eq!(diagnostics.len(), 10);
                assert!(diagnostics.iter().all(|row| {
                    row.failure_code == Some(FimixStartFailureCodeV2::RankDeficient)
                }));
            }
            unexpected => panic!("unexpected error: {unexpected:?}"),
        }
    }

    #[test]
    fn exhaustive_alignment_recovers_permutation_and_flags_ties() {
        let aligned = align_labels_exhaustive_v2(&[0, 0, 1, 1], &[1, 1, 0, 0], 2).unwrap();
        assert_eq!(aligned.candidate_to_reference, vec![1, 0]);
        assert_eq!(aligned.matched_observations, 4);
        assert!(!aligned.ambiguous);
        assert!(aligned.mutual_majority);

        let tied = align_labels_exhaustive_v2(&[0, 0, 1, 1], &[0, 1, 0, 1], 2).unwrap();
        assert!(tied.ambiguous);
        assert!(!tied.mutual_majority);

        assert_eq!(validate_retained_label_alignment_v2(&aligned), Ok(4));
        let mut altered = aligned.clone();
        altered.candidate_to_reference = vec![0, 1];
        assert!(validate_retained_label_alignment_v2(&altered).is_err());
    }

    #[test]
    fn target_payload_digest_is_ordered_binary_and_rejects_nonfinite_values() {
        let left = heterogeneity_target_payload_sha256_v2(&[1.0, -0.0, 2.5]).unwrap();
        let same = heterogeneity_target_payload_sha256_v2(&[1.0, -0.0, 2.5]).unwrap();
        let reordered = heterogeneity_target_payload_sha256_v2(&[2.5, -0.0, 1.0]).unwrap();
        assert_eq!(left.len(), 64);
        assert_eq!(left, same);
        assert_ne!(left, reordered);
        assert!(heterogeneity_target_payload_sha256_v2(&[f64::NAN]).is_err());
    }

    struct MeanSeparationRefitter {
        values: Vec<f64>,
        method_version: &'static str,
    }

    impl PlsPosFullRefitterV2 for MeanSeparationRefitter {
        fn refit_segment(
            &mut self,
            _segment_index: usize,
            row_indices: &[usize],
            scoring: PosScoringContractV2,
        ) -> Result<PosSegmentFullFitV2, String> {
            let mean = row_indices.iter().map(|row| self.values[*row]).sum::<f64>()
                / row_indices.len() as f64;
            let interactions = matches!(
                scoring,
                PosScoringContractV2::DestinationScoredInteractions { .. }
            );
            Ok(PosSegmentFullFitV2 {
                r_squared: vec![PosOutcomeR2V2 {
                    outcome_id: "y".to_string(),
                    r_squared: mean * mean,
                }],
                outcome_fit_audits: Vec::new(),
                parameter_signature: vec![mean],
                receipt: PosFullRefitReceiptV2 {
                    method_version: self.method_version.to_string(),
                    full_segment_pls_refit: true,
                    measurement_scores_reestimated: true,
                    score_orientation_reapplied: true,
                    interaction_stage_one_refit: interactions,
                    interaction_operands_restandardized_within_destination: interactions,
                    interaction_products_rebuilt_within_destination: interactions,
                    joint_structural_equations_refit: interactions,
                },
            })
        }
    }

    #[test]
    fn published_pos_uses_full_refits_and_monotone_strict_hill_climb() {
        let values = (0..40)
            .map(|row| if row < 20 { -1.0 } else { 1.0 })
            .collect::<Vec<_>>();
        let features = values.iter().map(|value| vec![*value]).collect::<Vec<_>>();
        let starts = build_pls_pos_start_plan_v2(&features, 2, 42, None).unwrap();
        let mut config = PlsPosV2Config::for_segments(2, values.len());
        config.minimum_segment_size = 10;
        let mut refitter = MeanSeparationRefitter {
            values,
            method_version: PLS_POS_PUBLISHED_METHOD_VERSION_V2,
        };
        let result = fit_pls_pos_published_v2(&starts, &config, &mut refitter).unwrap();
        validate_pos_multistart_evidence_v2(&result).unwrap();
        assert_eq!(
            result.multistart_evidence.completed_starts.len(),
            result.starts.iter().filter(|start| start.completed).count()
        );
        let mut digest_tampered = result.clone();
        digest_tampered.multistart_evidence.completed_starts[0].partition_sha256 = "f".repeat(64);
        assert!(validate_pos_multistart_evidence_v2(&digest_tampered).is_err());
        let mut reproducing_tampered = result.clone();
        reproducing_tampered.reproducing_start_indices.pop();
        assert!(validate_pos_multistart_evidence_v2(&reproducing_tampered).is_err());
        assert_eq!(result.method_version, PLS_POS_PUBLISHED_METHOD_VERSION_V2);
        assert_abs_diff_eq!(result.objective, 2.0, epsilon = 1.0e-12);
        assert!(
            result
                .objective_history
                .windows(2)
                .all(|window| window[1] > window[0])
        );
        assert!(result.reproducing_start_indices.len() >= 2);
        assert_eq!(result.segments[0].observations, 20);
        assert_eq!(result.segments[1].observations, 20);
    }

    #[test]
    fn pos_outcome_audit_enforces_centered_sst_receipt() {
        let mut fit = PosSegmentFullFitV2 {
            r_squared: vec![PosOutcomeR2V2 {
                outcome_id: "y".into(),
                r_squared: 1.0,
            }],
            outcome_fit_audits: vec![PosOutcomeFitAuditV2 {
                outcome_id: "y".into(),
                source_row_indices: vec![0, 1],
                observed_scores: vec![-1.0, 1.0],
                fitted_scores: vec![-1.0, 1.0],
                observed_mean: 0.0,
                centered_total_sum_of_squares: 2.0,
            }],
            parameter_signature: vec![1.0],
            receipt: PosFullRefitReceiptV2 {
                method_version: PLS_POS_PUBLISHED_METHOD_VERSION_V2.into(),
                full_segment_pls_refit: true,
                measurement_scores_reestimated: true,
                score_orientation_reapplied: true,
                interaction_stage_one_refit: false,
                interaction_operands_restandardized_within_destination: false,
                interaction_products_rebuilt_within_destination: false,
                joint_structural_equations_refit: false,
            },
        };
        validate_pos_fit(&fit, PosScoringContractV2::PublishedP0FullSegmentPls).unwrap();
        fit.outcome_fit_audits[0].centered_total_sum_of_squares = 3.0;
        assert!(validate_pos_fit(&fit, PosScoringContractV2::PublishedP0FullSegmentPls).is_err());
    }

    struct CandidateFailureRefitter;

    impl PlsPosFullRefitterV2 for CandidateFailureRefitter {
        fn refit_segment(
            &mut self,
            _segment_index: usize,
            row_indices: &[usize],
            _scoring: PosScoringContractV2,
        ) -> Result<PosSegmentFullFitV2, String> {
            if row_indices.len() != 20 {
                return Err("synthetic candidate refit failure".into());
            }
            Ok(PosSegmentFullFitV2 {
                r_squared: vec![PosOutcomeR2V2 {
                    outcome_id: "y".into(),
                    r_squared: 0.5,
                }],
                outcome_fit_audits: Vec::new(),
                parameter_signature: vec![0.0],
                receipt: PosFullRefitReceiptV2 {
                    method_version: PLS_POS_PUBLISHED_METHOD_VERSION_V2.into(),
                    full_segment_pls_refit: true,
                    measurement_scores_reestimated: true,
                    score_orientation_reapplied: true,
                    interaction_stage_one_refit: false,
                    interaction_operands_restandardized_within_destination: false,
                    interaction_products_rebuilt_within_destination: false,
                    joint_structural_equations_refit: false,
                },
            })
        }
    }

    #[test]
    fn pos_candidate_refit_failure_fails_the_start_before_sweep_completion() {
        let assignments = (0..40)
            .map(|row| usize::from(row >= 20))
            .collect::<Vec<_>>();
        let mut config = PlsPosV2Config::for_segments(2, assignments.len());
        config.minimum_segment_size = 10;
        let error = run_pos_start(
            0,
            &assignments,
            &config,
            PosScoringContractV2::PublishedP0FullSegmentPls,
            &mut CandidateFailureRefitter,
        )
        .unwrap_err();
        assert!(
            error
                .reason
                .contains("candidate full refit failed during sweep")
        );
        assert!(error.reason.contains("synthetic candidate refit failure"));
        assert_eq!(error.candidate_refit_failures.len(), 1);
        assert_eq!(error.candidate_refit_failures[0].observation, 0);
        assert_eq!(error.candidate_refit_failures[0].source_segment, 0);
        assert_eq!(error.candidate_refit_failures[0].destination_segment, 1);

        let starts = vec![assignments; 10];
        let result = fit_pls_pos_published_v2(&starts, &config, &mut CandidateFailureRefitter);
        match result {
            Err(HeterogeneityV2Error::NoCompletedPosStart { diagnostics }) => {
                assert_eq!(diagnostics.len(), 10);
                assert!(diagnostics.iter().all(|row| {
                    !row.completed
                        && row.candidate_refit_failures.len() == 1
                        && row.failure_reason.as_ref().is_some_and(|reason| {
                            reason.contains("candidate full refit failed during sweep")
                        })
                }));
            }
            unexpected => panic!("unexpected POS result: {unexpected:?}"),
        }
    }

    #[test]
    fn destination_scored_pos_has_a_distinct_enforced_receipt() {
        let values = (0..40)
            .map(|row| if row < 20 { -1.0 } else { 1.0 })
            .collect::<Vec<_>>();
        let features = values.iter().map(|value| vec![*value]).collect::<Vec<_>>();
        let starts = build_pls_pos_start_plan_v2(&features, 2, 42, None).unwrap();
        let mut config = PlsPosV2Config::for_segments(2, values.len());
        config.minimum_segment_size = 10;
        let mut refitter = MeanSeparationRefitter {
            values,
            method_version: PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2,
        };
        let result = fit_pls_pos_destination_scored_interactions_v2(
            &starts,
            HeterogeneityInteractionProfileV2::P2MultiTwoWay,
            &config,
            &mut refitter,
        )
        .unwrap();
        assert_eq!(
            result.method_version,
            PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2
        );
        assert!(matches!(
            result.scoring_contract,
            PosScoringContractV2::DestinationScoredInteractions { .. }
        ));
    }

    fn perfect_alignment() -> LabelAlignmentV2 {
        LabelAlignmentV2 {
            candidate_to_reference: vec![0, 1],
            matched_observations: 40,
            match_share: 1.0,
            ambiguous: false,
            mutual_majority: true,
            overlap: vec![vec![20, 0], vec![0, 20]],
        }
    }

    #[test]
    fn bootstrap_ledger_is_seeded_fixed_k_no_retry_and_requires_ninety_percent() {
        let mut plan = HeterogeneityBootstrapPlanV2::interactive_default(
            HeterogeneityBootstrapAlgorithmV2::FimixPlsV2,
            2,
        );
        plan.requested_replicates = 500;
        let mut entries = (0..plan.requested_replicates)
            .map(|replicate_index| HeterogeneityBootstrapLedgerEntryV2 {
                replicate_index,
                seed: heterogeneity_bootstrap_replicate_seed_v2(&plan, replicate_index),
                status: HeterogeneityBootstrapReplicateStatusV2::Usable,
                fit_statistic: Some(-100.0),
                label_alignment: Some(perfect_alignment()),
                target_payload_sha256: Some(format!("digest_{replicate_index}")),
                failure_reason: None,
            })
            .collect::<Vec<_>>();
        let qualified = summarize_heterogeneity_bootstrap_ledger_v2(&plan, &entries).unwrap();
        assert_eq!(
            qualified.qualification,
            HeterogeneityBootstrapQualificationV2::Qualified
        );
        assert_eq!(qualified.required_usable_replicates, 450);
        for entry in entries.iter_mut().take(51) {
            entry.status = HeterogeneityBootstrapReplicateStatusV2::FitFailed;
            entry.fit_statistic = None;
            entry.label_alignment = None;
            entry.target_payload_sha256 = None;
            entry.failure_reason = Some("fixture failure".to_string());
        }
        let insufficient = summarize_heterogeneity_bootstrap_ledger_v2(&plan, &entries).unwrap();
        assert_eq!(
            insufficient.qualification,
            HeterogeneityBootstrapQualificationV2::InsufficientUsableReplicates
        );
        assert_eq!(insufficient.usable_replicates, 449);
        assert_eq!(insufficient.retry_policy, "none_one_attempt_per_index");
    }

    fn complete_compositional_pairs(segments: usize) -> Vec<PosPairwiseCompositionalInvarianceV1> {
        (0..segments)
            .flat_map(|left| {
                (left + 1..segments).map(move |right| PosPairwiseCompositionalInvarianceV1 {
                    left_segment: left,
                    right_segment: right,
                    passed: true,
                    permutation_p_value: Some(0.50),
                })
            })
            .collect()
    }

    #[test]
    fn common_metric_gate_requires_steps_one_and_two_but_reports_step_three() {
        let mut evidence = PosConstructComparabilityEvidenceV1 {
            construct_id: "x".to_string(),
            configural_identity_passed: true,
            compositional_invariance: complete_compositional_pairs(3),
            step3_equality: vec![PosPairwiseStep3EqualityV1 {
                left_segment: 0,
                right_segment: 1,
                mean_equality_passed: false,
                variance_equality_passed: false,
            }],
        };
        let input = PosCommonMetricGateInputV1 {
            pooled_metric_id: "pooled".to_string(),
            pooled_metric_sha256: "sha256".to_string(),
            segments: 3,
            applied_identically_to_all_segments: true,
            required_construct_ids: vec!["x".to_string()],
            evidence: vec![evidence.clone()],
        };
        let passed = evaluate_pos_common_metric_gate_v1(&input);
        assert_eq!(passed.status, PosCommonMetricGateStatusV1::Passed);
        assert!(passed.inferential_gamma_delta_slope_contrasts_allowed);
        assert_eq!(passed.step3_failed_mean_comparisons, 1);
        assert!(!passed.step3_required_for_standardized_path_comparison);

        evidence.compositional_invariance[0].passed = false;
        let failed = evaluate_pos_common_metric_gate_v1(&PosCommonMetricGateInputV1 {
            evidence: vec![evidence],
            ..input
        });
        assert_eq!(failed.status, PosCommonMetricGateStatusV1::DescriptiveOnly);
        assert!(!failed.inferential_gamma_delta_slope_contrasts_allowed);
        assert!(failed.blockers.iter().any(|blocker| {
            blocker.code == PosCommonMetricBlockerCodeV1::CompositionalInvarianceFailed
        }));
    }
}
