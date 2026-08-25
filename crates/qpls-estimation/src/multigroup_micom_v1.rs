//! Pairwise MICOM Steps 1--3 for the additive multigroup workflow.
//!
//! Step 1 is an explicit analyst-reviewed configuration receipt. Steps 2 and
//! 3 use one deterministic, fixed-size pairwise permutation ledger. The
//! model-specific refitter must re-estimate the complete measurement/scoring
//! model for each permuted group and apply both fitted scoring rules to the
//! same pooled scoring rows. The kernel never substitutes static scores and
//! never retries failed partitions.

use crate::{
    GroupIndexV1, OrderedGroupPairV1, PairwisePartitionPlanV1, RefitFailureCodeV1, RefitFailureV1,
    SelectedGroupRowV1, build_pairwise_partition_plan_from_rows_v1,
    materialize_pairwise_partition_v1, validate_pairwise_partition_plan_for_rows_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MICOM_PAIRWISE_METHOD_VERSION_V1: &str = "qpls.micom.pairwise-permutation.v1";
pub const MICOM_CASE_WEIGHTED_PAIRWISE_METHOD_VERSION_V1: &str =
    "qpls.micom.case-weighted-pairwise-permutation.v1";
pub const MICOM_PAIRWISE_MIN_PERMUTATIONS_V1: usize = 5_000;
pub const MICOM_PAIRWISE_MAX_PERMUTATIONS_V1: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomConfiguralReceiptV1 {
    pub identical_indicators_and_coding: bool,
    pub identical_data_treatment: bool,
    pub identical_algorithm_settings: bool,
    pub identical_model_specification: bool,
    pub deterministic_orientation_reviewed: bool,
    pub analyst_review_confirmed: bool,
}

impl MicomConfiguralReceiptV1 {
    pub fn complete(&self) -> bool {
        self.identical_indicators_and_coding
            && self.identical_data_treatment
            && self.identical_algorithm_settings
            && self.identical_model_specification
            && self.deterministic_orientation_reviewed
            && self.analyst_review_confirmed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicomGroupTrainingRowsV1 {
    pub group: GroupIndexV1,
    pub source_rows: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicomFitKindV1 {
    Observed,
    Permutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicomFitRequestV1 {
    pub kind: MicomFitKindV1,
    pub replicate: Option<usize>,
    pub training_groups: Vec<MicomGroupTrainingRowsV1>,
    /// Canonical pooled row order on which every fitted group scoring rule is
    /// evaluated. This prevents a correlation between differently ordered
    /// score vectors from being mistaken for compositional invariance.
    pub scoring_rows: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomGroupConstructScoresV1 {
    pub group: GroupIndexV1,
    pub construct_id: String,
    pub pooled_scores: Vec<f64>,
}

/// Pair-pooled construct scores used exclusively for MICOM Step 3.  They are
/// fit once on the canonical A+B observed sample and then held fixed while
/// the shared partition plan changes membership.  Group-local scoring rules
/// must never be substituted for this common origin and scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomPooledConstructScoresV1 {
    pub construct_id: String,
    pub pooled_scores: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomFitV1 {
    pub scores: Vec<MicomGroupConstructScoresV1>,
    #[serde(default)]
    pub pooled_reference_scores: Vec<MicomPooledConstructScoresV1>,
}

pub trait MicomRefitterV1 {
    fn fit_micom(&mut self, request: &MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1>;
}

impl<F> MicomRefitterV1 for F
where
    F: FnMut(&MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1>,
{
    fn fit_micom(&mut self, request: &MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1> {
        self(request)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomPermutationConfigV1 {
    pub requested: usize,
    pub seed: u64,
    pub alpha: f64,
}

impl MicomPermutationConfigV1 {
    pub fn minimum_usable(&self) -> usize {
        1_000usize.max((0.90 * self.requested as f64).ceil() as usize)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MicomPermutationStatusV1 {
    Usable,
    Failed {
        code: RefitFailureCodeV1,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomPermutationLedgerEntryV1 {
    pub replicate: usize,
    /// The master seed is repeated only as provenance.  Partition identity is
    /// the checked SHA-256 receipt shared with permutation MGA.
    pub seed: u64,
    pub partition_sha256: String,
    pub status: MicomPermutationStatusV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomConstructResultV1 {
    pub construct_id: String,
    pub observed_compositional_correlation: f64,
    pub compositional_lower_quantile: Option<f64>,
    /// Directional probability `Pr(c_perm <= c_observed)` with plus-one
    /// correction. This is the Step-2 invariance probability, not a generic
    /// two-sided path p-value.
    pub compositional_invariance_probability: Option<f64>,
    pub compositional_invariance: bool,
    pub observed_mean_difference_a_minus_b: f64,
    pub mean_difference_two_sided_probability: Option<f64>,
    pub equal_means: bool,
    pub observed_log_variance_ratio_a_minus_b: f64,
    pub variance_difference_two_sided_probability: Option<f64>,
    pub equal_variances: bool,
    pub partial_measurement_invariance: bool,
    pub full_measurement_invariance: bool,
    /// Step-2 values are retained for every construct. Step-3 values are
    /// retained only for the deterministic first construct in each pair so
    /// independent formula coverage remains bounded. Values follow
    /// usable-ledger order; an empty Step-3 vector means non-audit construct.
    pub permutation_compositional_correlations: Vec<f64>,
    pub permutation_mean_differences: Vec<f64>,
    pub permutation_log_variance_ratios: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MicomPairwiseResultV1 {
    pub method_version: String,
    pub pair: OrderedGroupPairV1,
    pub configural_receipt: MicomConfiguralReceiptV1,
    pub requested_permutations: usize,
    pub usable_permutations: usize,
    pub minimum_usable_permutations: usize,
    pub partition_plan_sha256: String,
    pub ledger_sha256: String,
    pub ledger: Vec<MicomPermutationLedgerEntryV1>,
    pub constructs: Vec<MicomConstructResultV1>,
    pub complete: bool,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MicomErrorV1 {
    #[error("MICOM contract is invalid: {0}")]
    InvalidContract(String),
    #[error("MICOM observed refit failed: {0}")]
    ObservedRefit(String),
    #[error("MICOM compositional score contract is invalid: {0}")]
    ScoreContract(String),
}

pub fn run_pairwise_micom_v1<R, C>(
    refitter: &mut R,
    pair: OrderedGroupPairV1,
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    construct_ids: &[String],
    configural_receipt: MicomConfiguralReceiptV1,
    config: MicomPermutationConfigV1,
    cancelled: C,
) -> Result<MicomPairwiseResultV1, MicomErrorV1>
where
    R: MicomRefitterV1,
    C: Fn() -> bool,
{
    validate_contract(
        pair,
        rows_by_group,
        construct_ids,
        &configural_receipt,
        &config,
    )?;
    let selected_rows = selected_pair_rows(rows_by_group, pair)?;
    let plan = build_pairwise_partition_plan_from_rows_v1(
        &selected_rows,
        pair,
        config.requested,
        config.seed,
    )
    .map_err(|error| MicomErrorV1::InvalidContract(error.to_string()))?;
    run_pairwise_micom_with_partition_plan_v1(
        refitter,
        pair,
        rows_by_group,
        &selected_rows,
        construct_ids,
        configural_receipt,
        config,
        &plan,
        cancelled,
    )
}

/// Executes MICOM against the same immutable partition authority that may be
/// consumed by pairwise permutation MGA.  Fit failures remain method-local;
/// neither method replaces a failed partition with a new draw.
#[allow(clippy::too_many_arguments)]
pub fn run_pairwise_micom_with_partition_plan_v1<R, C>(
    refitter: &mut R,
    pair: OrderedGroupPairV1,
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    stable_rows: &[SelectedGroupRowV1],
    construct_ids: &[String],
    configural_receipt: MicomConfiguralReceiptV1,
    config: MicomPermutationConfigV1,
    partition_plan: &PairwisePartitionPlanV1,
    cancelled: C,
) -> Result<MicomPairwiseResultV1, MicomErrorV1>
where
    R: MicomRefitterV1,
    C: Fn() -> bool,
{
    run_pairwise_micom_with_partition_plan_internal_v1(
        refitter,
        pair,
        rows_by_group,
        stable_rows,
        construct_ids,
        configural_receipt,
        config,
        partition_plan,
        None,
        cancelled,
    )
}

/// Executes MICOM with positive case weights attached to stable source rows.
/// Every training fit still receives the ordinary row partition, while Steps
/// 2--3 use the same positive weighted moments as the WPLS scoring model.
#[allow(clippy::too_many_arguments)]
pub fn run_pairwise_case_weighted_micom_with_partition_plan_v1<R, C>(
    refitter: &mut R,
    pair: OrderedGroupPairV1,
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    stable_rows: &[SelectedGroupRowV1],
    construct_ids: &[String],
    configural_receipt: MicomConfiguralReceiptV1,
    config: MicomPermutationConfigV1,
    partition_plan: &PairwisePartitionPlanV1,
    case_weights_by_row: &BTreeMap<u64, f64>,
    cancelled: C,
) -> Result<MicomPairwiseResultV1, MicomErrorV1>
where
    R: MicomRefitterV1,
    C: Fn() -> bool,
{
    run_pairwise_micom_with_partition_plan_internal_v1(
        refitter,
        pair,
        rows_by_group,
        stable_rows,
        construct_ids,
        configural_receipt,
        config,
        partition_plan,
        Some(case_weights_by_row),
        cancelled,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_pairwise_micom_with_partition_plan_internal_v1<R, C>(
    refitter: &mut R,
    pair: OrderedGroupPairV1,
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    stable_rows: &[SelectedGroupRowV1],
    construct_ids: &[String],
    configural_receipt: MicomConfiguralReceiptV1,
    config: MicomPermutationConfigV1,
    partition_plan: &PairwisePartitionPlanV1,
    case_weights_by_row: Option<&BTreeMap<u64, f64>>,
    cancelled: C,
) -> Result<MicomPairwiseResultV1, MicomErrorV1>
where
    R: MicomRefitterV1,
    C: Fn() -> bool,
{
    validate_contract(
        pair,
        rows_by_group,
        construct_ids,
        &configural_receipt,
        &config,
    )?;
    let selected_rows = authoritative_selected_pair_rows(stable_rows, rows_by_group, pair)?;
    validate_pairwise_partition_plan_for_rows_v1(
        &selected_rows,
        pair,
        config.requested,
        config.seed,
        partition_plan,
    )
    .map_err(|error| MicomErrorV1::InvalidContract(error.to_string()))?;
    let rows_a = rows_by_group
        .get(&pair.group_a)
        .expect("validated group A rows");
    let rows_b = rows_by_group
        .get(&pair.group_b)
        .expect("validated group B rows");
    let mut scoring_rows = rows_a.iter().chain(rows_b).copied().collect::<Vec<_>>();
    scoring_rows.sort_unstable();
    if let Some(weights) = case_weights_by_row {
        let selected = scoring_rows
            .iter()
            .map(|row| weights.get(row).copied())
            .collect::<Option<Vec<_>>>();
        let Some(selected) = selected else {
            return Err(MicomErrorV1::InvalidContract(
                "case-weighted MICOM omitted a selected row weight".into(),
            ));
        };
        let minimum = selected.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = selected.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if selected
            .iter()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
            || maximum / minimum > 1.0e6
        {
            return Err(MicomErrorV1::InvalidContract(
                "case-weighted MICOM requires finite positive weights with max/min at most 1e6"
                    .into(),
            ));
        }
    }
    let observed_request = MicomFitRequestV1 {
        kind: MicomFitKindV1::Observed,
        replicate: None,
        training_groups: vec![
            MicomGroupTrainingRowsV1 {
                group: pair.group_a,
                source_rows: rows_a.clone(),
            },
            MicomGroupTrainingRowsV1 {
                group: pair.group_b,
                source_rows: rows_b.clone(),
            },
        ],
        scoring_rows: scoring_rows.clone(),
    };
    let observed = refitter
        .fit_micom(&observed_request)
        .map_err(|failure| MicomErrorV1::ObservedRefit(failure.detail))?;
    let observed_scores = validated_score_map(&observed, pair, construct_ids, scoring_rows.len())?;
    let pooled_reference_scores =
        validated_pooled_score_map(&observed, construct_ids, scoring_rows.len())?;
    let observed_statistics = construct_statistics(
        &observed_scores,
        &pooled_reference_scores,
        pair,
        construct_ids,
        &scoring_rows,
        rows_a,
        rows_b,
        case_weights_by_row,
    )?;

    let mut compositional_distributions = construct_ids
        .iter()
        .map(|id| (id.clone(), Vec::with_capacity(config.requested)))
        .collect::<BTreeMap<_, _>>();
    let mut mean_distributions = compositional_distributions.clone();
    let mut variance_distributions = compositional_distributions.clone();
    let mut ledger = Vec::with_capacity(config.requested);
    for replicate in 0..config.requested {
        if cancelled() {
            ledger.push(MicomPermutationLedgerEntryV1 {
                replicate,
                seed: config.seed,
                partition_sha256: partition_plan.entries[replicate].partition_sha256.clone(),
                status: MicomPermutationStatusV1::Failed {
                    code: RefitFailureCodeV1::Cancelled,
                    detail: "cancelled before MICOM permutation refit".into(),
                },
            });
            for later in (replicate + 1)..config.requested {
                ledger.push(MicomPermutationLedgerEntryV1 {
                    replicate: later,
                    seed: config.seed,
                    partition_sha256: partition_plan.entries[later].partition_sha256.clone(),
                    status: MicomPermutationStatusV1::Failed {
                        code: RefitFailureCodeV1::Cancelled,
                        detail: "cancelled before MICOM permutation refit".into(),
                    },
                });
            }
            break;
        }
        let partition =
            materialize_pairwise_partition_v1(&selected_rows, pair, partition_plan, replicate)
                .map_err(|error| MicomErrorV1::InvalidContract(error.to_string()))?;
        let membership_a = partition
            .assignments
            .iter()
            .filter_map(|row| (row.group == pair.group_a).then_some(row.source_row))
            .collect::<Vec<_>>();
        let membership_b = partition
            .assignments
            .iter()
            .filter_map(|row| (row.group == pair.group_b).then_some(row.source_row))
            .collect::<Vec<_>>();
        let request = MicomFitRequestV1 {
            kind: MicomFitKindV1::Permutation,
            replicate: Some(replicate),
            training_groups: vec![
                MicomGroupTrainingRowsV1 {
                    group: pair.group_a,
                    source_rows: membership_a,
                },
                MicomGroupTrainingRowsV1 {
                    group: pair.group_b,
                    source_rows: membership_b,
                },
            ],
            scoring_rows: scoring_rows.clone(),
        };
        match refitter.fit_micom(&request).and_then(|fit| {
            if !fit.pooled_reference_scores.is_empty() {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    "permutation MICOM refit attempted to replace the frozen pooled Step-3 scores",
                ));
            }
            let scores = validated_score_map(&fit, pair, construct_ids, scoring_rows.len())
                .map_err(|error| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        error.to_string(),
                    )
                })?;
            construct_statistics(
                &scores,
                &pooled_reference_scores,
                pair,
                construct_ids,
                &scoring_rows,
                &request.training_groups[0].source_rows,
                &request.training_groups[1].source_rows,
                case_weights_by_row,
            )
            .map_err(|error| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    error.to_string(),
                )
            })
        }) {
            Ok(statistics) => {
                for (construct_id, statistic) in statistics {
                    compositional_distributions
                        .get_mut(&construct_id)
                        .expect("declared construct")
                        .push(statistic.compositional_correlation);
                    mean_distributions
                        .get_mut(&construct_id)
                        .expect("declared construct")
                        .push(statistic.mean_difference);
                    variance_distributions
                        .get_mut(&construct_id)
                        .expect("declared construct")
                        .push(statistic.log_variance_ratio);
                }
                ledger.push(MicomPermutationLedgerEntryV1 {
                    replicate,
                    seed: config.seed,
                    partition_sha256: partition.partition_sha256.clone(),
                    status: MicomPermutationStatusV1::Usable,
                });
            }
            Err(failure) => ledger.push(MicomPermutationLedgerEntryV1 {
                replicate,
                seed: config.seed,
                partition_sha256: partition.partition_sha256,
                status: MicomPermutationStatusV1::Failed {
                    code: failure.code,
                    detail: failure.detail,
                },
            }),
        }
    }
    let usable = ledger
        .iter()
        .filter(|entry| entry.status == MicomPermutationStatusV1::Usable)
        .count();
    let minimum_usable = config.minimum_usable();
    let complete = usable >= minimum_usable;
    let mut constructs = Vec::with_capacity(construct_ids.len());
    for (construct_position, construct_id) in construct_ids.iter().enumerate() {
        let observed = observed_statistics
            .get(construct_id)
            .expect("observed statistic exists");
        let correlations = &mut compositional_distributions
            .remove(construct_id)
            .expect("distribution exists");
        let means = &mean_distributions[construct_id];
        let variances = &variance_distributions[construct_id];
        correlations.sort_by(f64::total_cmp);
        let lower = complete.then(|| type7_quantile(correlations, config.alpha));
        let compositional_probability = complete.then(|| {
            plus_one_probability(
                correlations
                    .iter()
                    .filter(|value| **value <= observed.compositional_correlation)
                    .count(),
                correlations.len(),
            )
        });
        let mean_probability = complete.then(|| {
            plus_one_probability(
                means
                    .iter()
                    .filter(|value| value.abs() >= observed.mean_difference.abs())
                    .count(),
                means.len(),
            )
        });
        let variance_probability = complete.then(|| {
            plus_one_probability(
                variances
                    .iter()
                    .filter(|value| value.abs() >= observed.log_variance_ratio.abs())
                    .count(),
                variances.len(),
            )
        });
        let compositional_invariance = complete
            && lower.is_some_and(|threshold| observed.compositional_correlation >= threshold);
        let equal_means = mean_probability.is_some_and(|value| value >= config.alpha);
        let equal_variances = variance_probability.is_some_and(|value| value >= config.alpha);
        constructs.push(MicomConstructResultV1 {
            construct_id: construct_id.clone(),
            observed_compositional_correlation: observed.compositional_correlation,
            compositional_lower_quantile: lower,
            compositional_invariance_probability: compositional_probability,
            compositional_invariance,
            observed_mean_difference_a_minus_b: observed.mean_difference,
            mean_difference_two_sided_probability: mean_probability,
            equal_means,
            observed_log_variance_ratio_a_minus_b: observed.log_variance_ratio,
            variance_difference_two_sided_probability: variance_probability,
            equal_variances,
            partial_measurement_invariance: configural_receipt.complete()
                && compositional_invariance,
            full_measurement_invariance: configural_receipt.complete()
                && compositional_invariance
                && equal_means
                && equal_variances,
            permutation_compositional_correlations: correlations.clone(),
            permutation_mean_differences: if construct_position == 0 {
                means.clone()
            } else {
                Vec::new()
            },
            permutation_log_variance_ratios: if construct_position == 0 {
                variances.clone()
            } else {
                Vec::new()
            },
        });
    }
    let ledger_sha256 = format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(&ledger)
                .map_err(|error| MicomErrorV1::InvalidContract(error.to_string()))?
        )
    );
    Ok(MicomPairwiseResultV1 {
        method_version: if case_weights_by_row.is_some() {
            MICOM_CASE_WEIGHTED_PAIRWISE_METHOD_VERSION_V1
        } else {
            MICOM_PAIRWISE_METHOD_VERSION_V1
        }
        .into(),
        pair,
        configural_receipt,
        requested_permutations: config.requested,
        usable_permutations: usable,
        minimum_usable_permutations: minimum_usable,
        partition_plan_sha256: partition_plan.plan_sha256.clone(),
        ledger_sha256,
        ledger,
        constructs,
        complete,
    })
}

#[derive(Debug, Clone)]
struct ConstructStatistic {
    compositional_correlation: f64,
    mean_difference: f64,
    log_variance_ratio: f64,
}

fn selected_pair_rows(
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    pair: OrderedGroupPairV1,
) -> Result<Vec<SelectedGroupRowV1>, MicomErrorV1> {
    let rows_a = rows_by_group
        .get(&pair.group_a)
        .ok_or_else(|| MicomErrorV1::InvalidContract("group A rows are missing".into()))?;
    let rows_b = rows_by_group
        .get(&pair.group_b)
        .ok_or_else(|| MicomErrorV1::InvalidContract("group B rows are missing".into()))?;
    Ok(rows_a
        .iter()
        .map(|source_row| SelectedGroupRowV1 {
            source_row: *source_row,
            stable_row_token: *source_row,
            group: pair.group_a,
        })
        .chain(rows_b.iter().map(|source_row| SelectedGroupRowV1 {
            source_row: *source_row,
            stable_row_token: *source_row,
            group: pair.group_b,
        }))
        .collect())
}

fn authoritative_selected_pair_rows(
    stable_rows: &[SelectedGroupRowV1],
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    pair: OrderedGroupPairV1,
) -> Result<Vec<SelectedGroupRowV1>, MicomErrorV1> {
    let selected = stable_rows
        .iter()
        .copied()
        .filter(|row| row.group == pair.group_a || row.group == pair.group_b)
        .collect::<Vec<_>>();
    let expected = rows_by_group
        .iter()
        .filter(|(group, _)| **group == pair.group_a || **group == pair.group_b)
        .flat_map(|(group, rows)| rows.iter().map(|row| (*row, *group)))
        .collect::<BTreeSet<_>>();
    let actual = selected
        .iter()
        .map(|row| (row.source_row, row.group))
        .collect::<BTreeSet<_>>();
    let stable_count = selected
        .iter()
        .map(|row| row.stable_row_token)
        .collect::<BTreeSet<_>>()
        .len();
    if actual != expected || actual.len() != selected.len() || stable_count != selected.len() {
        return Err(MicomErrorV1::InvalidContract(
            "MICOM stable-row authority differs from its physical pair membership".into(),
        ));
    }
    Ok(selected)
}

fn validate_contract(
    pair: OrderedGroupPairV1,
    rows_by_group: &BTreeMap<GroupIndexV1, Vec<u64>>,
    construct_ids: &[String],
    configural_receipt: &MicomConfiguralReceiptV1,
    config: &MicomPermutationConfigV1,
) -> Result<(), MicomErrorV1> {
    if !configural_receipt.complete() {
        return Err(MicomErrorV1::InvalidContract(
            "MICOM Step 1 requires explicit completion before permutation".into(),
        ));
    }
    if !(MICOM_PAIRWISE_MIN_PERMUTATIONS_V1..=MICOM_PAIRWISE_MAX_PERMUTATIONS_V1)
        .contains(&config.requested)
        || !config.alpha.is_finite()
        || !(0.0..0.5).contains(&config.alpha)
    {
        return Err(MicomErrorV1::InvalidContract(
            "MICOM requires 5000 through 10000 permutations and alpha in (0, .5)".into(),
        ));
    }
    let mut ids = BTreeSet::new();
    if construct_ids.is_empty()
        || construct_ids
            .iter()
            .any(|id| id.trim().is_empty() || !ids.insert(id.as_str()))
    {
        return Err(MicomErrorV1::InvalidContract(
            "MICOM construct identities must be nonempty and unique".into(),
        ));
    }
    let Some(rows_a) = rows_by_group.get(&pair.group_a) else {
        return Err(MicomErrorV1::InvalidContract(
            "group A rows are missing".into(),
        ));
    };
    let Some(rows_b) = rows_by_group.get(&pair.group_b) else {
        return Err(MicomErrorV1::InvalidContract(
            "group B rows are missing".into(),
        ));
    };
    let mut rows = BTreeSet::new();
    if rows_a.len() < 10
        || rows_b.len() < 10
        || rows_a.iter().chain(rows_b).any(|row| !rows.insert(*row))
    {
        return Err(MicomErrorV1::InvalidContract(
            "MICOM pair requires ten distinct complete-case rows per group".into(),
        ));
    }
    Ok(())
}

fn validated_score_map(
    fit: &MicomFitV1,
    pair: OrderedGroupPairV1,
    construct_ids: &[String],
    expected_rows: usize,
) -> Result<BTreeMap<(GroupIndexV1, String), Vec<f64>>, MicomErrorV1> {
    let groups = BTreeSet::from([pair.group_a, pair.group_b]);
    let constructs = construct_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut scores = BTreeMap::new();
    for row in &fit.scores {
        if !groups.contains(&row.group)
            || !constructs.contains(&row.construct_id)
            || row.pooled_scores.len() != expected_rows
            || row.pooled_scores.iter().any(|value| !value.is_finite())
            || scores
                .insert(
                    (row.group, row.construct_id.clone()),
                    row.pooled_scores.clone(),
                )
                .is_some()
        {
            return Err(MicomErrorV1::ScoreContract(
                "MICOM scores have unknown/duplicate identities, nonfinite values, or wrong row count"
                    .into(),
            ));
        }
    }
    if scores.len() != groups.len() * constructs.len() {
        return Err(MicomErrorV1::ScoreContract(
            "MICOM fit omitted a group-construct score vector".into(),
        ));
    }
    Ok(scores)
}

fn validated_pooled_score_map(
    fit: &MicomFitV1,
    construct_ids: &[String],
    expected_rows: usize,
) -> Result<BTreeMap<String, Vec<f64>>, MicomErrorV1> {
    let constructs = construct_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut scores = BTreeMap::new();
    for row in &fit.pooled_reference_scores {
        if !constructs.contains(&row.construct_id)
            || row.pooled_scores.len() != expected_rows
            || row.pooled_scores.iter().any(|value| !value.is_finite())
            || scores
                .insert(row.construct_id.clone(), row.pooled_scores.clone())
                .is_some()
        {
            return Err(MicomErrorV1::ScoreContract(
                "MICOM pooled-reference scores have unknown/duplicate identities, nonfinite values, or wrong row count"
                    .into(),
            ));
        }
    }
    if scores.len() != constructs.len() {
        return Err(MicomErrorV1::ScoreContract(
            "MICOM observed fit omitted a pair-pooled Step-3 construct score vector".into(),
        ));
    }
    Ok(scores)
}

fn construct_statistics(
    scores: &BTreeMap<(GroupIndexV1, String), Vec<f64>>,
    pooled_reference_scores: &BTreeMap<String, Vec<f64>>,
    pair: OrderedGroupPairV1,
    construct_ids: &[String],
    scoring_rows: &[u64],
    membership_a: &[u64],
    membership_b: &[u64],
    case_weights_by_row: Option<&BTreeMap<u64, f64>>,
) -> Result<BTreeMap<String, ConstructStatistic>, MicomErrorV1> {
    let positions = scoring_rows
        .iter()
        .enumerate()
        .map(|(index, row)| (*row, index))
        .collect::<BTreeMap<_, _>>();
    let positions_a = membership_a
        .iter()
        .map(|row| positions.get(row).copied())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            MicomErrorV1::ScoreContract("group A row is outside pooled scores".into())
        })?;
    let positions_b = membership_b
        .iter()
        .map(|row| positions.get(row).copied())
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            MicomErrorV1::ScoreContract("group B row is outside pooled scores".into())
        })?;
    let scoring_weights = case_weights_by_row
        .map(|weights| {
            scoring_rows
                .iter()
                .map(|row| weights.get(row).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    MicomErrorV1::ScoreContract(
                        "case-weighted MICOM scoring row is missing its weight".into(),
                    )
                })
        })
        .transpose()?;
    let mut result = BTreeMap::new();
    for construct_id in construct_ids {
        let a = &scores[&(pair.group_a, construct_id.clone())];
        let b = &scores[&(pair.group_b, construct_id.clone())];
        let compositional_correlation = if let Some(weights) = &scoring_weights {
            weighted_pearson(a, b, weights)?
        } else {
            pearson(a, b)?
        };
        let pooled = pooled_reference_scores.get(construct_id).ok_or_else(|| {
            MicomErrorV1::ScoreContract(format!(
                "pair-pooled Step-3 scores are missing for {construct_id}"
            ))
        })?;
        let own_a = positions_a
            .iter()
            .map(|index| pooled[*index])
            .collect::<Vec<_>>();
        let own_b = positions_b
            .iter()
            .map(|index| pooled[*index])
            .collect::<Vec<_>>();
        let (mean_difference, variance_a, variance_b) = if let Some(weights) = case_weights_by_row {
            let weights_a = membership_a
                .iter()
                .map(|row| weights.get(row).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    MicomErrorV1::ScoreContract(
                        "case-weighted MICOM group A row is missing its weight".into(),
                    )
                })?;
            let weights_b = membership_b
                .iter()
                .map(|row| weights.get(row).copied())
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    MicomErrorV1::ScoreContract(
                        "case-weighted MICOM group B row is missing its weight".into(),
                    )
                })?;
            (
                weighted_mean(&own_a, &weights_a)? - weighted_mean(&own_b, &weights_b)?,
                weighted_sample_variance(&own_a, &weights_a)?,
                weighted_sample_variance(&own_b, &weights_b)?,
            )
        } else {
            (
                mean(&own_a) - mean(&own_b),
                sample_variance(&own_a)?,
                sample_variance(&own_b)?,
            )
        };
        // Written as a difference so reversing A/B is the exact arithmetic
        // negation, not merely algebraically equivalent after extra rounding.
        let log_variance_ratio = variance_a.ln() - variance_b.ln();
        if !mean_difference.is_finite() || !log_variance_ratio.is_finite() {
            return Err(MicomErrorV1::ScoreContract(
                "MICOM mean or log-variance contrast is nonfinite".into(),
            ));
        }
        result.insert(
            construct_id.clone(),
            ConstructStatistic {
                compositional_correlation,
                mean_difference,
                log_variance_ratio,
            },
        );
    }
    Ok(result)
}

fn pearson(left: &[f64], right: &[f64]) -> Result<f64, MicomErrorV1> {
    if left.len() != right.len() || left.len() < 2 {
        return Err(MicomErrorV1::ScoreContract(
            "correlation vectors have incompatible dimensions".into(),
        ));
    }
    let left_mean = mean(left);
    let right_mean = mean(right);
    let cross = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>();
    let left_ss = left
        .iter()
        .map(|value| (value - left_mean).powi(2))
        .sum::<f64>();
    let right_ss = right
        .iter()
        .map(|value| (value - right_mean).powi(2))
        .sum::<f64>();
    let denominator = (left_ss * right_ss).sqrt();
    if denominator <= 0.0 || !denominator.is_finite() {
        return Err(MicomErrorV1::ScoreContract(
            "constant or nonfinite compositional score vector".into(),
        ));
    }
    let correlation = (cross / denominator).clamp(-1.0, 1.0);
    if correlation.is_finite() {
        Ok(correlation)
    } else {
        Err(MicomErrorV1::ScoreContract(
            "compositional correlation is nonfinite".into(),
        ))
    }
}

fn weighted_mean(values: &[f64], weights: &[f64]) -> Result<f64, MicomErrorV1> {
    if values.len() != weights.len()
        || values.is_empty()
        || weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        return Err(MicomErrorV1::ScoreContract(
            "weighted MICOM values and positive weights have incompatible dimensions".into(),
        ));
    }
    let sum = weights.iter().sum::<f64>();
    let center = values
        .iter()
        .zip(weights)
        .map(|(value, weight)| value * weight)
        .sum::<f64>()
        / sum;
    if center.is_finite() {
        Ok(center)
    } else {
        Err(MicomErrorV1::ScoreContract(
            "weighted MICOM mean is nonfinite".into(),
        ))
    }
}

fn weighted_sample_variance(values: &[f64], weights: &[f64]) -> Result<f64, MicomErrorV1> {
    let center = weighted_mean(values, weights)?;
    let sum = weights.iter().sum::<f64>();
    let sum_squared = weights.iter().map(|weight| weight * weight).sum::<f64>();
    let denominator = sum - sum_squared / sum;
    let variance = values
        .iter()
        .zip(weights)
        .map(|(value, weight)| weight * (value - center).powi(2))
        .sum::<f64>()
        / denominator;
    if variance > 0.0 && variance.is_finite() {
        Ok(variance)
    } else {
        Err(MicomErrorV1::ScoreContract(
            "weighted MICOM variance is zero or nonfinite".into(),
        ))
    }
}

fn weighted_pearson(left: &[f64], right: &[f64], weights: &[f64]) -> Result<f64, MicomErrorV1> {
    if left.len() != right.len() || left.len() != weights.len() || left.len() < 2 {
        return Err(MicomErrorV1::ScoreContract(
            "weighted correlation vectors have incompatible dimensions".into(),
        ));
    }
    let left_mean = weighted_mean(left, weights)?;
    let right_mean = weighted_mean(right, weights)?;
    let mut cross = 0.0;
    let mut left_square = 0.0;
    let mut right_square = 0.0;
    for ((left, right), weight) in left.iter().zip(right).zip(weights) {
        cross += weight * (left - left_mean) * (right - right_mean);
        left_square += weight * (left - left_mean).powi(2);
        right_square += weight * (right - right_mean).powi(2);
    }
    let denominator = (left_square * right_square).sqrt();
    if denominator <= f64::EPSILON || !denominator.is_finite() {
        return Err(MicomErrorV1::ScoreContract(
            "weighted compositional score vector is constant or nonfinite".into(),
        ));
    }
    Ok((cross / denominator).clamp(-1.0, 1.0))
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_variance(values: &[f64]) -> Result<f64, MicomErrorV1> {
    if values.len() < 2 {
        return Err(MicomErrorV1::ScoreContract(
            "variance requires at least two scores".into(),
        ));
    }
    let center = mean(values);
    let variance = values
        .iter()
        .map(|value| (value - center).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    if variance > 0.0 && variance.is_finite() {
        Ok(variance)
    } else {
        Err(MicomErrorV1::ScoreContract(
            "MICOM own-group score variance is zero or nonfinite".into(),
        ))
    }
}

fn plus_one_probability(extreme: usize, usable: usize) -> f64 {
    (extreme + 1) as f64 / (usable + 1) as f64
}

fn type7_quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let index = probability * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] + (index - lower as f64) * (sorted[upper] - sorted[lower])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reviewed() -> MicomConfiguralReceiptV1 {
        MicomConfiguralReceiptV1 {
            identical_indicators_and_coding: true,
            identical_data_treatment: true,
            identical_algorithm_settings: true,
            identical_model_specification: true,
            deterministic_orientation_reviewed: true,
            analyst_review_confirmed: true,
        }
    }

    #[test]
    fn incomplete_step_one_fails_before_refitting() {
        let a = GroupIndexV1::new(0).unwrap();
        let b = GroupIndexV1::new(1).unwrap();
        let pair = OrderedGroupPairV1::new(a, b).unwrap();
        let mut called = false;
        let mut refitter = |_request: &MicomFitRequestV1| {
            called = true;
            Ok(MicomFitV1 {
                scores: Vec::new(),
                pooled_reference_scores: Vec::new(),
            })
        };
        let mut receipt = reviewed();
        receipt.analyst_review_confirmed = false;
        let rows = BTreeMap::from([
            (a, (0..10).collect::<Vec<_>>()),
            (b, (10..20).collect::<Vec<_>>()),
        ]);
        let error = run_pairwise_micom_v1(
            &mut refitter,
            pair,
            &rows,
            &["c".into()],
            receipt,
            MicomPermutationConfigV1 {
                requested: 5_000,
                seed: 42,
                alpha: 0.05,
            },
            || false,
        )
        .unwrap_err();
        assert!(matches!(error, MicomErrorV1::InvalidContract(_)));
        assert!(!called);
    }

    #[test]
    fn type7_quantile_and_plus_one_probability_are_frozen() {
        assert_eq!(type7_quantile(&[0.0, 1.0, 2.0], 0.25), 0.5);
        assert_eq!(plus_one_probability(0, 99), 0.01);
    }

    #[test]
    fn step_three_uses_one_pair_pooled_score_metric_and_reverses_exactly() {
        let a = GroupIndexV1::new(0).unwrap();
        let b = GroupIndexV1::new(1).unwrap();
        let pair = OrderedGroupPairV1::new(a, b).unwrap();
        let scores = BTreeMap::from([
            ((a, "c".into()), vec![1.0, 2.0, 3.0, 4.0]),
            ((b, "c".into()), vec![101.0, 102.0, 103.0, 104.0]),
        ]);
        let pooled = BTreeMap::from([("c".into(), vec![-2.0, -1.0, 1.0, 2.0])]);
        let scoring_rows = vec![10, 11, 20, 21];
        let forward = construct_statistics(
            &scores,
            &pooled,
            pair,
            &["c".into()],
            &scoring_rows,
            &[10, 11],
            &[20, 21],
            None,
        )
        .unwrap();
        let reverse_pair = OrderedGroupPairV1::new(b, a).unwrap();
        let reverse = construct_statistics(
            &scores,
            &pooled,
            reverse_pair,
            &["c".into()],
            &scoring_rows,
            &[20, 21],
            &[10, 11],
            None,
        )
        .unwrap();
        assert_eq!(forward["c"].mean_difference, -3.0);
        assert_eq!(forward["c"].mean_difference, -reverse["c"].mean_difference);
        assert_eq!(
            forward["c"].log_variance_ratio,
            -reverse["c"].log_variance_ratio
        );
        assert_eq!(
            forward["c"].compositional_correlation,
            reverse["c"].compositional_correlation
        );
    }

    #[test]
    fn case_weighted_micom_moments_equal_expanded_weighted_identities_and_reverse() {
        let a = GroupIndexV1::new(0).unwrap();
        let b = GroupIndexV1::new(1).unwrap();
        let pair = OrderedGroupPairV1::new(a, b).unwrap();
        let scores = BTreeMap::from([
            ((a, "c".into()), vec![1.0, 2.0, 4.0, 8.0]),
            ((b, "c".into()), vec![1.5, 2.5, 4.5, 8.5]),
        ]);
        let pooled = BTreeMap::from([("c".into(), vec![-3.0, -1.0, 2.0, 7.0])]);
        let scoring_rows = vec![10, 11, 20, 21];
        let weights = BTreeMap::from([(10, 1.0), (11, 3.0), (20, 2.0), (21, 4.0)]);
        let forward = construct_statistics(
            &scores,
            &pooled,
            pair,
            &["c".into()],
            &scoring_rows,
            &[10, 11],
            &[20, 21],
            Some(&weights),
        )
        .unwrap();
        let reverse_pair = OrderedGroupPairV1::new(b, a).unwrap();
        let reverse = construct_statistics(
            &scores,
            &pooled,
            reverse_pair,
            &["c".into()],
            &scoring_rows,
            &[20, 21],
            &[10, 11],
            Some(&weights),
        )
        .unwrap();
        assert_eq!(forward["c"].mean_difference, -reverse["c"].mean_difference);
        assert_eq!(
            forward["c"].log_variance_ratio,
            -reverse["c"].log_variance_ratio
        );
        assert_eq!(
            forward["c"].compositional_correlation,
            reverse["c"].compositional_correlation
        );
    }

    #[test]
    fn nonfirst_pair_uses_authoritative_stable_tokens() {
        let a = GroupIndexV1::new(0).unwrap();
        let b = GroupIndexV1::new(1).unwrap();
        let c = GroupIndexV1::new(2).unwrap();
        let rows = vec![
            SelectedGroupRowV1 {
                source_row: 0,
                stable_row_token: 50,
                group: a,
            },
            SelectedGroupRowV1 {
                source_row: 4,
                stable_row_token: 10,
                group: b,
            },
            SelectedGroupRowV1 {
                source_row: 1,
                stable_row_token: 40,
                group: c,
            },
            SelectedGroupRowV1 {
                source_row: 5,
                stable_row_token: 30,
                group: b,
            },
            SelectedGroupRowV1 {
                source_row: 2,
                stable_row_token: 20,
                group: c,
            },
        ];
        let rows_by_group = BTreeMap::from([(a, vec![0]), (b, vec![4, 5]), (c, vec![1, 2])]);
        let pair = OrderedGroupPairV1::new(b, c).unwrap();
        let selected = authoritative_selected_pair_rows(&rows, &rows_by_group, pair).unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|row| (row.source_row, row.stable_row_token, row.group))
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([(4, 10, b), (5, 30, b), (1, 40, c), (2, 20, c)])
        );

        let mut mismatched = rows_by_group.clone();
        mismatched.get_mut(&c).unwrap()[0] = 3;
        assert!(authoritative_selected_pair_rows(&rows, &mismatched, pair).is_err());
    }

    #[test]
    fn micom_and_mga_consume_the_same_unequal_size_partition_ledger() {
        let a = GroupIndexV1::new(0).unwrap();
        let b = GroupIndexV1::new(1).unwrap();
        let pair = OrderedGroupPairV1::new(a, b).unwrap();
        let design = crate::MultigroupDesignV1 {
            groups: vec![
                crate::GroupIdentityV1 {
                    index: a,
                    value: crate::TypedGroupValueV1::Text { value: "a".into() },
                    display_label: "A".into(),
                },
                crate::GroupIdentityV1 {
                    index: b,
                    value: crate::TypedGroupValueV1::Text { value: "b".into() },
                    display_label: "B".into(),
                },
            ],
            rows: (0..10)
                .map(|source_row| SelectedGroupRowV1 {
                    source_row,
                    stable_row_token: source_row,
                    group: a,
                })
                .chain((10..27).map(|source_row| SelectedGroupRowV1 {
                    source_row,
                    stable_row_token: source_row,
                    group: b,
                }))
                .collect(),
        };
        let plan = crate::build_pairwise_partition_plan_v1(&design, pair, 5_000, 42).unwrap();
        let rows_by_group = BTreeMap::from([
            (a, (0..10).collect::<Vec<_>>()),
            (b, (10..27).collect::<Vec<_>>()),
        ]);
        let mut micom_requests = BTreeMap::<(usize, GroupIndexV1), Vec<u64>>::new();
        let mut micom_refitter = |request: &MicomFitRequestV1| {
            if let Some(replicate) = request.replicate {
                for group in &request.training_groups {
                    micom_requests.insert((replicate, group.group), group.source_rows.clone());
                }
            }
            let axis = (0..request.scoring_rows.len())
                .map(|index| index as f64)
                .collect::<Vec<_>>();
            Ok(MicomFitV1 {
                scores: vec![
                    MicomGroupConstructScoresV1 {
                        group: a,
                        construct_id: "c".into(),
                        pooled_scores: axis.clone(),
                    },
                    MicomGroupConstructScoresV1 {
                        group: b,
                        construct_id: "c".into(),
                        pooled_scores: axis.clone(),
                    },
                ],
                pooled_reference_scores: (request.kind == MicomFitKindV1::Observed)
                    .then(|| MicomPooledConstructScoresV1 {
                        construct_id: "c".into(),
                        pooled_scores: axis,
                    })
                    .into_iter()
                    .collect(),
            })
        };
        let micom = run_pairwise_micom_with_partition_plan_v1(
            &mut micom_refitter,
            pair,
            &rows_by_group,
            &design.rows,
            &["c".into()],
            reviewed(),
            MicomPermutationConfigV1 {
                requested: 5_000,
                seed: 42,
                alpha: 0.05,
            },
            &plan,
            || false,
        )
        .unwrap();

        let parameter = crate::ParameterIdentityV1 {
            stable_id: "path:x:y".into(),
            family: crate::ParameterFamilyV1::StructuralPath,
        };
        let refit_parameter = parameter.clone();
        let mut mga_requests = BTreeMap::<(usize, GroupIndexV1), Vec<u64>>::new();
        let mut mga_refitter = |request: &crate::MultigroupFitRequestV1| {
            if let Some(replicate) = request.replicate {
                mga_requests.insert((replicate, request.group), request.source_rows.clone());
            }
            Ok(crate::ParameterVectorV1 {
                parameters: vec![crate::ParameterEstimateV1 {
                    parameter: refit_parameter.clone(),
                    estimate: request.source_rows.iter().sum::<u64>() as f64
                        / request.source_rows.len() as f64,
                }],
            })
        };
        let mga = crate::run_pairwise_permutation_with_plan_v1(
            &design,
            pair,
            &[parameter],
            crate::MultigroupResamplingConfigV1::official_defaults(),
            &plan,
            &mut mga_refitter,
        )
        .unwrap();
        assert_eq!(micom.partition_plan_sha256, plan.plan_sha256);
        assert_eq!(mga.plan_sha256, plan.plan_sha256);
        assert_eq!(micom_requests, mga_requests);
        assert!(
            micom
                .ledger
                .iter()
                .zip(&mga.ledger)
                .all(|(left, right)| left.partition_sha256 == right.partition_sha256)
        );
    }
}
