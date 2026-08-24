//! Exact count-space resampling for positive-integer frequency-weighted MGA.
//!
//! A physical row is retained once with an integer multiplicity. Permutation
//! allocations are multivariate-hypergeometric and preserve expanded group
//! totals; bootstrap allocations are multinomial with probabilities f_i/N.
//! No operation materializes the expanded row vector.

use crate::{
    AlternativeHypothesisV1, BootstrapGroupLedgerV1, BootstrapLedgerEntryV1,
    EligibilityBlockerCodeV1, EligibilityBlockerV1, EligibilityWarningCodeV1, EligibilityWarningV1,
    FitSampleKindV1, GroupBootstrapBankV1, GroupBootstrapBanksV1, GroupEligibilitySummaryV1,
    GroupFitLedgerV1, GroupIdentityV1, GroupIndexV1, GroupParameterVectorV1,
    InferenceAvailabilityV1, MicomConfiguralReceiptV1, MicomConstructResultV1, MicomFitKindV1,
    MicomFitV1, MicomPairwiseResultV1, MicomPermutationConfigV1, MicomPermutationLedgerEntryV1,
    MicomPermutationStatusV1, MultigroupDesignV1, MultigroupEligibilityV1,
    MultigroupResamplingConfigV1, OmnibusPermutationParameterV1, OmnibusPermutationResultV1,
    OrderedGroupPairV1, PairwisePermutationParameterV1, PairwisePermutationResultV1,
    PairwisePointParameterV1, ParameterIdentityV1, ParameterVectorV1, PermutationLedgerEntryV1,
    RefitFailureCodeV1, RefitFailureV1, ResampleFitStatusV1, SelectedGroupRowV1,
    assess_multigroup_design_v1,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{Binomial, DiscreteCDF, Hypergeometric};
use std::collections::{BTreeMap, BTreeSet};

pub const FREQUENCY_MULTIGROUP_PAIRWISE_PLAN_VERSION_V1: &str =
    "qpls.mga.frequency-count-space.pairwise-plan.v1";
pub const FREQUENCY_MULTIGROUP_PAIRWISE_PERMUTATION_VERSION_V1: &str =
    "qpls.mga.frequency-count-space.pairwise-permutation.v1";
pub const FREQUENCY_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1: &str =
    "qpls.mga.frequency-count-space.omnibus-permutation.v1";
pub const FREQUENCY_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1: &str =
    "qpls.mga.frequency-count-space.bootstrap-bank.v1";
pub const FREQUENCY_MICOM_PAIRWISE_METHOD_VERSION_V1: &str =
    "qpls.micom.frequency-count-space.pairwise-permutation.v1";

const PAIRWISE_STREAM: &[u8] = b"qpls-mga-frequency-pairwise-v1";
const OMNIBUS_STREAM: &[u8] = b"qpls-mga-frequency-omnibus-v1";
const BOOTSTRAP_STREAM: &[u8] = b"qpls-mga-frequency-bootstrap-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencySelectedGroupRowV1 {
    pub source_row: u64,
    pub group: GroupIndexV1,
    pub frequency: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyMultigroupDesignV1 {
    pub groups: Vec<GroupIdentityV1>,
    pub rows: Vec<FrequencySelectedGroupRowV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyGroupSampleV1 {
    pub group: GroupIndexV1,
    pub source_rows: Vec<u64>,
    pub counts: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyMultigroupFitRequestV1 {
    pub sample_kind: FitSampleKindV1,
    pub group: GroupIndexV1,
    pub replicate: Option<usize>,
    pub source_rows: Vec<u64>,
    pub counts: Vec<u64>,
}

pub trait FrequencyMultigroupRefitterV1 {
    fn fit_frequency(
        &mut self,
        request: &FrequencyMultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1>;
}

impl<F> FrequencyMultigroupRefitterV1 for F
where
    F: FnMut(&FrequencyMultigroupFitRequestV1) -> Result<ParameterVectorV1, RefitFailureV1>,
{
    fn fit_frequency(
        &mut self,
        request: &FrequencyMultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        self(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyPairwisePartitionPlanEntryV1 {
    pub replicate: usize,
    pub partition_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencyPairwisePartitionPlanV1 {
    pub method_version: String,
    pub pair: OrderedGroupPairV1,
    pub seed: u64,
    pub requested: usize,
    pub group_low_total: u64,
    pub group_high_total: u64,
    pub observed_membership_sha256: String,
    pub plan_sha256: String,
    pub entries: Vec<FrequencyPairwisePartitionPlanEntryV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyPairwisePartitionMaterializationV1 {
    pub replicate: usize,
    pub partition_sha256: String,
    pub samples: Vec<FrequencyGroupSampleV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyMicomGroupTrainingV1 {
    pub group: GroupIndexV1,
    pub source_rows: Vec<u64>,
    pub counts: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrequencyMicomFitRequestV1 {
    pub kind: MicomFitKindV1,
    pub replicate: Option<usize>,
    pub training_groups: Vec<FrequencyMicomGroupTrainingV1>,
    pub scoring_rows: Vec<u64>,
}

pub trait FrequencyMicomRefitterV1 {
    fn fit_frequency_micom(
        &mut self,
        request: &FrequencyMicomFitRequestV1,
    ) -> Result<MicomFitV1, RefitFailureV1>;
}

impl<F> FrequencyMicomRefitterV1 for F
where
    F: FnMut(&FrequencyMicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1>,
{
    fn fit_frequency_micom(
        &mut self,
        request: &FrequencyMicomFitRequestV1,
    ) -> Result<MicomFitV1, RefitFailureV1> {
        self(request)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum FrequencyMultigroupErrorV1 {
    #[error("frequency multigroup design is invalid: {0}")]
    InvalidDesign(String),
    #[error("frequency partition plan is invalid: {0}")]
    InvalidPlan(String),
    #[error("frequency count-space distribution failed: {0}")]
    Distribution(String),
    #[error("observed frequency fit failed for group {group:?}: {failure:?}")]
    ObservedFitFailed {
        group: GroupIndexV1,
        failure: RefitFailureV1,
    },
    #[error("frequency MICOM contract is invalid: {0}")]
    MicomContract(String),
    #[error("frequency MICOM observed refit failed: {0}")]
    MicomObservedRefit(String),
}

fn standard_design(design: &FrequencyMultigroupDesignV1) -> MultigroupDesignV1 {
    MultigroupDesignV1 {
        groups: design.groups.clone(),
        rows: design
            .rows
            .iter()
            .map(|row| SelectedGroupRowV1 {
                source_row: row.source_row,
                group: row.group,
            })
            .collect(),
    }
}

pub fn assess_frequency_multigroup_design_v1(
    design: &FrequencyMultigroupDesignV1,
) -> MultigroupEligibilityV1 {
    let base = assess_multigroup_design_v1(&standard_design(design));
    let mut blockers = base
        .blockers
        .into_iter()
        .filter(|blocker| {
            !matches!(
                blocker.code,
                EligibilityBlockerCodeV1::InsufficientCompleteCases
                    | EligibilityBlockerCodeV1::ExtremeGroupImbalance
            )
        })
        .collect::<Vec<_>>();
    let mut warnings = base
        .warnings
        .into_iter()
        .filter(|warning| {
            !matches!(
                warning.code,
                EligibilityWarningCodeV1::SmallGroup
                    | EligibilityWarningCodeV1::GroupImbalanceAboveTwoToOne
            )
        })
        .collect::<Vec<_>>();
    if let Some(row) = design.rows.iter().find(|row| row.frequency == 0) {
        blockers.push(EligibilityBlockerV1 {
            code: EligibilityBlockerCodeV1::InsufficientCompleteCases,
            group: Some(row.group),
            observed: Some(0.0),
            required: Some(1.0),
            detail: "frequency rows must carry positive integer counts".into(),
        });
    }
    let totals = design
        .groups
        .iter()
        .map(|group| {
            design
                .rows
                .iter()
                .filter(|row| row.group == group.index)
                .try_fold(0_u64, |sum, row| sum.checked_add(row.frequency))
                .unwrap_or(u64::MAX)
        })
        .collect::<Vec<_>>();
    for (position, total) in totals.iter().enumerate() {
        let group = GroupIndexV1::new(position).ok();
        if *total < 10 {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::InsufficientCompleteCases,
                group,
                observed: Some(*total as f64),
                required: Some(10.0),
                detail: "frequency-expanded group requires at least ten complete cases".into(),
            });
        } else if *total < 30 {
            warnings.push(EligibilityWarningV1 {
                code: EligibilityWarningCodeV1::SmallGroup,
                group,
                observed: *total as f64,
                threshold: 30.0,
                detail: "frequency-expanded group has fewer than thirty complete cases".into(),
            });
        }
    }
    let minimum = totals.iter().copied().min().unwrap_or(0);
    let maximum = totals.iter().copied().max().unwrap_or(0);
    let ratio = (minimum > 0).then_some(maximum as f64 / minimum as f64);
    if ratio.is_some_and(|value| value > 10.0) {
        blockers.push(EligibilityBlockerV1 {
            code: EligibilityBlockerCodeV1::ExtremeGroupImbalance,
            group: None,
            observed: ratio,
            required: Some(10.0),
            detail: "frequency-expanded group imbalance exceeds ten to one".into(),
        });
    } else if ratio.is_some_and(|value| value > 2.0) {
        warnings.push(EligibilityWarningV1 {
            code: EligibilityWarningCodeV1::GroupImbalanceAboveTwoToOne,
            group: None,
            observed: ratio.unwrap_or(0.0),
            threshold: 2.0,
            detail: "frequency-expanded group imbalance exceeds two to one".into(),
        });
    }
    MultigroupEligibilityV1 {
        eligible: blockers.is_empty(),
        group_counts: totals
            .into_iter()
            .enumerate()
            .filter_map(|(index, total)| {
                Some(GroupEligibilitySummaryV1 {
                    group: GroupIndexV1::new(index).ok()?,
                    complete_cases: usize::try_from(total).unwrap_or(usize::MAX),
                })
            })
            .collect(),
        maximum_imbalance_ratio: ratio,
        blockers,
        warnings,
    }
}

fn eligible_design(
    design: &FrequencyMultigroupDesignV1,
) -> Result<MultigroupEligibilityV1, FrequencyMultigroupErrorV1> {
    let eligibility = assess_frequency_multigroup_design_v1(design);
    if eligibility.eligible {
        Ok(eligibility)
    } else {
        Err(FrequencyMultigroupErrorV1::InvalidDesign(format!(
            "{:?}",
            eligibility.blockers
        )))
    }
}

fn validate_parameters(
    parameters: &[ParameterIdentityV1],
) -> Result<(), FrequencyMultigroupErrorV1> {
    let unique = parameters
        .iter()
        .map(|parameter| parameter.stable_id.as_str())
        .collect::<BTreeSet<_>>();
    if parameters.is_empty()
        || parameters
            .iter()
            .any(|parameter| parameter.stable_id.is_empty())
        || unique.len() != parameters.len()
    {
        return Err(FrequencyMultigroupErrorV1::InvalidDesign(
            "parameter identities must be nonempty and unique".into(),
        ));
    }
    Ok(())
}

fn canonical_pair(pair: OrderedGroupPairV1) -> OrderedGroupPairV1 {
    if pair.group_a < pair.group_b {
        pair
    } else {
        OrderedGroupPairV1 {
            group_a: pair.group_b,
            group_b: pair.group_a,
        }
    }
}

fn canonical_rows(design: &FrequencyMultigroupDesignV1) -> Vec<FrequencySelectedGroupRowV1> {
    let mut rows = design.rows.clone();
    rows.sort_by_key(|row| row.source_row);
    rows
}

fn group_sample(
    rows: &[FrequencySelectedGroupRowV1],
    group: GroupIndexV1,
) -> FrequencyGroupSampleV1 {
    let selected = rows
        .iter()
        .filter(|row| row.group == group && row.frequency > 0)
        .collect::<Vec<_>>();
    FrequencyGroupSampleV1 {
        group,
        source_rows: selected.iter().map(|row| row.source_row).collect(),
        counts: selected.iter().map(|row| row.frequency).collect(),
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn frequency_membership_digest(rows: &[FrequencySelectedGroupRowV1]) -> String {
    let mut bytes = Vec::with_capacity(rows.len() * 17);
    for row in rows {
        bytes.extend_from_slice(&row.source_row.to_le_bytes());
        bytes.push(u8::from(row.group));
        bytes.extend_from_slice(&row.frequency.to_le_bytes());
    }
    sha256(&bytes)
}

fn samples_digest(samples: &[FrequencyGroupSampleV1]) -> String {
    let mut bytes = Vec::new();
    for sample in samples {
        bytes.push(u8::from(sample.group));
        for (row, count) in sample.source_rows.iter().zip(&sample.counts) {
            bytes.extend_from_slice(&row.to_le_bytes());
            bytes.extend_from_slice(&count.to_le_bytes());
        }
    }
    sha256(&bytes)
}

fn derive_seed(master: u64, stream: &[u8], replicate: usize, extra: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"QuickPLS frequency count-space v1\0");
    hasher.update(master.to_le_bytes());
    hasher.update(stream);
    hasher.update((replicate as u64).to_le_bytes());
    hasher.update(extra);
    hasher.finalize().into()
}

fn open_unit_interval(rng: &mut ChaCha20Rng) -> f64 {
    const DENOMINATOR: u64 = 1_u64 << 53;
    rng.random_range(1..DENOMINATOR) as f64 / DENOMINATOR as f64
}

fn hypergeometric_draw(
    population: u64,
    successes: u64,
    draws: u64,
    rng: &mut ChaCha20Rng,
) -> Result<u64, FrequencyMultigroupErrorV1> {
    if draws == 0 || successes == 0 {
        return Ok(0);
    }
    if successes == population {
        return Ok(draws);
    }
    Hypergeometric::new(population, successes, draws)
        .map_err(|error| FrequencyMultigroupErrorV1::Distribution(error.to_string()))
        .map(|distribution| distribution.inverse_cdf(open_unit_interval(rng)))
}

fn multinomial_draw(
    frequencies: &[u64],
    total: u64,
    rng: &mut ChaCha20Rng,
) -> Result<Vec<u64>, FrequencyMultigroupErrorV1> {
    if frequencies.is_empty() || frequencies.iter().sum::<u64>() != total {
        return Err(FrequencyMultigroupErrorV1::InvalidDesign(
            "multinomial source frequencies do not sum to their group total".into(),
        ));
    }
    let mut remaining_draws = total;
    let mut remaining_mass = total;
    let mut counts = Vec::with_capacity(frequencies.len());
    for frequency in &frequencies[..frequencies.len() - 1] {
        let probability = *frequency as f64 / remaining_mass as f64;
        let count = if remaining_draws == 0 {
            0
        } else if probability >= 1.0 {
            remaining_draws
        } else {
            Binomial::new(probability, remaining_draws)
                .map_err(|error| FrequencyMultigroupErrorV1::Distribution(error.to_string()))?
                .inverse_cdf(open_unit_interval(rng))
        };
        counts.push(count);
        remaining_draws -= count;
        remaining_mass -= *frequency;
    }
    counts.push(remaining_draws);
    Ok(counts)
}

fn pair_rows(
    design: &FrequencyMultigroupDesignV1,
    pair: OrderedGroupPairV1,
) -> Result<Vec<FrequencySelectedGroupRowV1>, FrequencyMultigroupErrorV1> {
    if pair.group_a == pair.group_b
        || pair.group_a.get() >= design.groups.len()
        || pair.group_b.get() >= design.groups.len()
    {
        return Err(FrequencyMultigroupErrorV1::InvalidDesign(
            "pair references absent or identical groups".into(),
        ));
    }
    let pair = canonical_pair(pair);
    Ok(canonical_rows(design)
        .into_iter()
        .filter(|row| row.group == pair.group_a || row.group == pair.group_b)
        .collect())
}

fn materialize_pairwise(
    rows: &[FrequencySelectedGroupRowV1],
    pair: OrderedGroupPairV1,
    seed: u64,
    replicate: usize,
) -> Result<Vec<FrequencyGroupSampleV1>, FrequencyMultigroupErrorV1> {
    let pair = canonical_pair(pair);
    let low_total = rows
        .iter()
        .filter(|row| row.group == pair.group_a)
        .map(|row| row.frequency)
        .sum::<u64>();
    let total = rows.iter().map(|row| row.frequency).sum::<u64>();
    let mut remaining_population = total;
    let mut remaining_low = low_total;
    let mut rng = ChaCha20Rng::from_seed(derive_seed(
        seed,
        PAIRWISE_STREAM,
        replicate,
        &[u8::from(pair.group_a), u8::from(pair.group_b)],
    ));
    let mut low_rows = Vec::new();
    let mut low_counts = Vec::new();
    let mut high_rows = Vec::new();
    let mut high_counts = Vec::new();
    for row in rows {
        let low =
            hypergeometric_draw(remaining_population, remaining_low, row.frequency, &mut rng)?;
        let high = row.frequency - low;
        if low > 0 {
            low_rows.push(row.source_row);
            low_counts.push(low);
        }
        if high > 0 {
            high_rows.push(row.source_row);
            high_counts.push(high);
        }
        remaining_population -= row.frequency;
        remaining_low -= low;
    }
    if remaining_population != 0 || remaining_low != 0 {
        return Err(FrequencyMultigroupErrorV1::Distribution(
            "pairwise allocation did not preserve the canonical low-group total".into(),
        ));
    }
    Ok(vec![
        FrequencyGroupSampleV1 {
            group: pair.group_a,
            source_rows: low_rows,
            counts: low_counts,
        },
        FrequencyGroupSampleV1 {
            group: pair.group_b,
            source_rows: high_rows,
            counts: high_counts,
        },
    ])
}

fn pairwise_plan_digest(
    pair: OrderedGroupPairV1,
    seed: u64,
    requested: usize,
    membership: &str,
    entries: &[FrequencyPairwisePartitionPlanEntryV1],
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(FREQUENCY_MULTIGROUP_PAIRWISE_PLAN_VERSION_V1.as_bytes());
    bytes.push(u8::from(pair.group_a));
    bytes.push(u8::from(pair.group_b));
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(&(requested as u64).to_le_bytes());
    bytes.extend_from_slice(membership.as_bytes());
    for entry in entries {
        bytes.extend_from_slice(&(entry.replicate as u64).to_le_bytes());
        bytes.extend_from_slice(entry.partition_sha256.as_bytes());
    }
    sha256(&bytes)
}

pub fn build_frequency_pairwise_partition_plan_v1(
    design: &FrequencyMultigroupDesignV1,
    pair: OrderedGroupPairV1,
    requested: usize,
    seed: u64,
) -> Result<FrequencyPairwisePartitionPlanV1, FrequencyMultigroupErrorV1> {
    eligible_design(design)?;
    MultigroupResamplingConfigV1 {
        requested,
        seed,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: AlternativeHypothesisV1::TwoSided,
    }
    .validate()
    .map_err(|error| FrequencyMultigroupErrorV1::InvalidPlan(error.to_string()))?;
    let pair = canonical_pair(pair);
    let rows = pair_rows(design, pair)?;
    let group_low_total = rows
        .iter()
        .filter(|row| row.group == pair.group_a)
        .map(|row| row.frequency)
        .sum();
    let group_high_total = rows
        .iter()
        .filter(|row| row.group == pair.group_b)
        .map(|row| row.frequency)
        .sum();
    let observed_membership_sha256 = frequency_membership_digest(&rows);
    let entries = (0..requested)
        .map(|replicate| {
            let samples = materialize_pairwise(&rows, pair, seed, replicate)?;
            Ok(FrequencyPairwisePartitionPlanEntryV1 {
                replicate,
                partition_sha256: samples_digest(&samples),
            })
        })
        .collect::<Result<Vec<_>, FrequencyMultigroupErrorV1>>()?;
    let plan_sha256 =
        pairwise_plan_digest(pair, seed, requested, &observed_membership_sha256, &entries);
    Ok(FrequencyPairwisePartitionPlanV1 {
        method_version: FREQUENCY_MULTIGROUP_PAIRWISE_PLAN_VERSION_V1.into(),
        pair,
        seed,
        requested,
        group_low_total,
        group_high_total,
        observed_membership_sha256,
        plan_sha256,
        entries,
    })
}

fn validate_frequency_plan(
    design: &FrequencyMultigroupDesignV1,
    pair: OrderedGroupPairV1,
    config: MultigroupResamplingConfigV1,
    plan: &FrequencyPairwisePartitionPlanV1,
) -> Result<Vec<FrequencySelectedGroupRowV1>, FrequencyMultigroupErrorV1> {
    config
        .validate()
        .map_err(|error| FrequencyMultigroupErrorV1::InvalidPlan(error.to_string()))?;
    let pair = canonical_pair(pair);
    let rows = pair_rows(design, pair)?;
    let low: u64 = rows
        .iter()
        .filter(|row| row.group == pair.group_a)
        .map(|row| row.frequency)
        .sum();
    let high: u64 = rows
        .iter()
        .filter(|row| row.group == pair.group_b)
        .map(|row| row.frequency)
        .sum();
    let membership = frequency_membership_digest(&rows);
    if plan.method_version != FREQUENCY_MULTIGROUP_PAIRWISE_PLAN_VERSION_V1
        || plan.pair != pair
        || plan.seed != config.seed
        || plan.requested != config.requested
        || plan.group_low_total != low
        || plan.group_high_total != high
        || plan.observed_membership_sha256 != membership
        || plan.entries.len() != config.requested
        || plan
            .entries
            .iter()
            .enumerate()
            .any(|(index, entry)| entry.replicate != index)
        || plan.plan_sha256
            != pairwise_plan_digest(
                pair,
                config.seed,
                config.requested,
                &membership,
                &plan.entries,
            )
    {
        return Err(FrequencyMultigroupErrorV1::InvalidPlan(
            "method, pair, seed, expanded totals, membership, or digest differs".into(),
        ));
    }
    Ok(rows)
}

pub fn materialize_frequency_pairwise_partition_v1(
    design: &FrequencyMultigroupDesignV1,
    pair: OrderedGroupPairV1,
    plan: &FrequencyPairwisePartitionPlanV1,
    replicate: usize,
) -> Result<FrequencyPairwisePartitionMaterializationV1, FrequencyMultigroupErrorV1> {
    let config = MultigroupResamplingConfigV1 {
        requested: plan.requested,
        seed: plan.seed,
        confidence_level: 0.95,
        alpha: 0.05,
        alternative: AlternativeHypothesisV1::TwoSided,
    };
    let rows = validate_frequency_plan(design, pair, config, plan)?;
    let entry = plan.entries.get(replicate).ok_or_else(|| {
        FrequencyMultigroupErrorV1::InvalidPlan(format!("replicate {replicate} is absent"))
    })?;
    let samples = materialize_pairwise(&rows, pair, plan.seed, replicate)?;
    let digest = samples_digest(&samples);
    if entry.replicate != replicate || entry.partition_sha256 != digest {
        return Err(FrequencyMultigroupErrorV1::InvalidPlan(format!(
            "replicate {replicate} does not reproduce its partition digest"
        )));
    }
    Ok(FrequencyPairwisePartitionMaterializationV1 {
        replicate,
        partition_sha256: digest,
        samples,
    })
}

fn checked_fit<R: FrequencyMultigroupRefitterV1>(
    refitter: &mut R,
    parameters: &[ParameterIdentityV1],
    request: FrequencyMultigroupFitRequestV1,
) -> Result<Vec<f64>, RefitFailureV1> {
    if request.source_rows.len() != request.counts.len()
        || request.source_rows.is_empty()
        || request.counts.iter().any(|count| *count == 0)
    {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::InsufficientRows,
            "frequency refit requires nonempty aligned positive counts",
        ));
    }
    let vector = refitter.fit_frequency(&request)?;
    if vector.parameters.len() != parameters.len()
        || vector
            .parameters
            .iter()
            .zip(parameters)
            .any(|(actual, expected)| actual.parameter != *expected || !actual.estimate.is_finite())
    {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::ParameterContractMismatch,
            "frequency refitter changed the parameter identity, order, dimension, or finiteness",
        ));
    }
    Ok(vector
        .parameters
        .into_iter()
        .map(|parameter| parameter.estimate)
        .collect())
}

fn observed_fit<R: FrequencyMultigroupRefitterV1>(
    design: &FrequencyMultigroupDesignV1,
    group: GroupIndexV1,
    parameters: &[ParameterIdentityV1],
    refitter: &mut R,
) -> Result<Vec<f64>, FrequencyMultigroupErrorV1> {
    let sample = group_sample(&canonical_rows(design), group);
    checked_fit(
        refitter,
        parameters,
        FrequencyMultigroupFitRequestV1 {
            sample_kind: FitSampleKindV1::ObservedGroup,
            group,
            replicate: None,
            source_rows: sample.source_rows,
            counts: sample.counts,
        },
    )
    .map_err(|failure| FrequencyMultigroupErrorV1::ObservedFitFailed { group, failure })
}

fn usable_group(group: GroupIndexV1) -> GroupFitLedgerV1 {
    GroupFitLedgerV1 {
        group,
        status: ResampleFitStatusV1::Usable,
        failure: None,
    }
}

fn failed_group(group: GroupIndexV1, failure: RefitFailureV1) -> GroupFitLedgerV1 {
    GroupFitLedgerV1 {
        group,
        status: ResampleFitStatusV1::Failed,
        failure: Some(failure),
    }
}

fn add_one_probability(extreme: usize, usable: usize) -> f64 {
    (extreme as f64 + 1.0) / (usable as f64 + 1.0)
}

fn selected_probability(
    alternative: AlternativeHypothesisV1,
    two_sided: f64,
    greater: f64,
    less: f64,
) -> f64 {
    match alternative {
        AlternativeHypothesisV1::TwoSided => two_sided,
        AlternativeHypothesisV1::Greater => greater,
        AlternativeHypothesisV1::Less => less,
    }
}

pub fn run_frequency_pairwise_permutation_with_plan_v1<R: FrequencyMultigroupRefitterV1>(
    design: &FrequencyMultigroupDesignV1,
    pair: OrderedGroupPairV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    plan: &FrequencyPairwisePartitionPlanV1,
    refitter: &mut R,
) -> Result<PairwisePermutationResultV1, FrequencyMultigroupErrorV1> {
    let eligibility = eligible_design(design)?;
    validate_parameters(parameters)?;
    validate_frequency_plan(design, pair, config, plan)?;
    let point_a = observed_fit(design, pair.group_a, parameters, refitter)?;
    let point_b = observed_fit(design, pair.group_b, parameters, refitter)?;
    let observed = point_a
        .iter()
        .zip(&point_b)
        .map(|(left, right)| left - right)
        .collect::<Vec<_>>();
    let mut absolute = vec![0; parameters.len()];
    let mut greater = vec![0; parameters.len()];
    let mut less = vec![0; parameters.len()];
    let mut audit_null_differences = Vec::with_capacity(config.requested);
    let mut usable = 0;
    let mut ledger = Vec::with_capacity(config.requested);
    for replicate in 0..config.requested {
        let partition = materialize_frequency_pairwise_partition_v1(design, pair, plan, replicate)?;
        let sample_for = |group| {
            partition
                .samples
                .iter()
                .find(|sample| sample.group == group)
                .expect("pair plan contains both canonical groups")
        };
        let sample_a = sample_for(pair.group_a);
        let sample_b = sample_for(pair.group_b);
        let fit_a = checked_fit(
            refitter,
            parameters,
            FrequencyMultigroupFitRequestV1 {
                sample_kind: FitSampleKindV1::PairwisePermutation,
                group: pair.group_a,
                replicate: Some(replicate),
                source_rows: sample_a.source_rows.clone(),
                counts: sample_a.counts.clone(),
            },
        );
        let fit_b = checked_fit(
            refitter,
            parameters,
            FrequencyMultigroupFitRequestV1 {
                sample_kind: FitSampleKindV1::PairwisePermutation,
                group: pair.group_b,
                replicate: Some(replicate),
                source_rows: sample_b.source_rows.clone(),
                counts: sample_b.counts.clone(),
            },
        );
        let group_fits = vec![
            fit_a
                .as_ref()
                .map(|_| usable_group(pair.group_a))
                .unwrap_or_else(|failure| failed_group(pair.group_a, failure.clone())),
            fit_b
                .as_ref()
                .map(|_| usable_group(pair.group_b))
                .unwrap_or_else(|failure| failed_group(pair.group_b, failure.clone())),
        ];
        let status = if let (Ok(a), Ok(b)) = (fit_a, fit_b) {
            for index in 0..parameters.len() {
                let difference = a[index] - b[index];
                if index == 0 {
                    audit_null_differences.push(difference);
                }
                if difference.abs() >= observed[index].abs() {
                    absolute[index] += 1;
                }
                if difference >= observed[index] {
                    greater[index] += 1;
                }
                if difference <= observed[index] {
                    less[index] += 1;
                }
            }
            usable += 1;
            ResampleFitStatusV1::Usable
        } else {
            ResampleFitStatusV1::Failed
        };
        ledger.push(PermutationLedgerEntryV1 {
            replicate,
            partition_sha256: partition.partition_sha256,
            status,
            group_fits,
        });
    }
    let minimum_usable = config.minimum_usable();
    let availability = if usable >= minimum_usable {
        InferenceAvailabilityV1::Available
    } else {
        InferenceAvailabilityV1::InsufficientUsableResamples
    };
    let point_estimates = parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| PairwisePointParameterV1 {
            parameter: parameter.clone(),
            estimate_a: point_a[index],
            estimate_b: point_b[index],
            difference_a_minus_b: observed[index],
        })
        .collect();
    let inference = if availability == InferenceAvailabilityV1::Available {
        parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let two_sided = add_one_probability(absolute[index], usable);
                let p_greater = add_one_probability(greater[index], usable);
                let p_less = add_one_probability(less[index], usable);
                PairwisePermutationParameterV1 {
                    parameter: parameter.clone(),
                    estimate_a: point_a[index],
                    estimate_b: point_b[index],
                    difference_a_minus_b: observed[index],
                    p_value_two_sided: two_sided,
                    p_value_greater: p_greater,
                    p_value_less: p_less,
                    selected_alternative: config.alternative,
                    selected_probability: selected_probability(
                        config.alternative,
                        two_sided,
                        p_greater,
                        p_less,
                    ),
                    null_differences: if index == 0 {
                        audit_null_differences.clone()
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(PairwisePermutationResultV1 {
        method_version: FREQUENCY_MULTIGROUP_PAIRWISE_PERMUTATION_VERSION_V1.into(),
        pair,
        seed: config.seed,
        requested: config.requested,
        attempted: ledger.len(),
        usable,
        failed: ledger.len() - usable,
        minimum_usable,
        retry_policy: "none".into(),
        plan_sha256: plan.plan_sha256.clone(),
        availability,
        point_estimates,
        parameters: inference,
        ledger,
        group_counts: eligibility.group_counts,
        eligibility_warnings: eligibility.warnings,
    })
}

fn omnibus_samples(
    rows: &[FrequencySelectedGroupRowV1],
    groups: &[GroupIdentityV1],
    seed: u64,
    replicate: usize,
) -> Result<Vec<FrequencyGroupSampleV1>, FrequencyMultigroupErrorV1> {
    let mut quotas = groups
        .iter()
        .map(|group| {
            rows.iter()
                .filter(|row| row.group == group.index)
                .map(|row| row.frequency)
                .sum::<u64>()
        })
        .collect::<Vec<_>>();
    let mut population = quotas.iter().sum::<u64>();
    let mut rng = ChaCha20Rng::from_seed(derive_seed(
        seed,
        OMNIBUS_STREAM,
        replicate,
        &(groups.len() as u64).to_le_bytes(),
    ));
    let mut samples = groups
        .iter()
        .map(|group| FrequencyGroupSampleV1 {
            group: group.index,
            source_rows: Vec::new(),
            counts: Vec::new(),
        })
        .collect::<Vec<_>>();
    for row in rows {
        let mut row_remaining = row.frequency;
        let mut conditional_population = population;
        for group_index in 0..groups.len() - 1 {
            let count = hypergeometric_draw(
                conditional_population,
                quotas[group_index],
                row_remaining,
                &mut rng,
            )?;
            if count > 0 {
                samples[group_index].source_rows.push(row.source_row);
                samples[group_index].counts.push(count);
            }
            quotas[group_index] -= count;
            row_remaining -= count;
            conditional_population -= quotas[group_index] + count;
        }
        let last = groups.len() - 1;
        if row_remaining > 0 {
            samples[last].source_rows.push(row.source_row);
            samples[last].counts.push(row_remaining);
        }
        quotas[last] -= row_remaining;
        population -= row.frequency;
    }
    if population != 0 || quotas.iter().any(|quota| *quota != 0) {
        return Err(FrequencyMultigroupErrorV1::Distribution(
            "omnibus allocation did not preserve every group total".into(),
        ));
    }
    Ok(samples)
}

fn maximum_spreads(values: &[Vec<f64>], parameter_count: usize) -> Vec<f64> {
    (0..parameter_count)
        .map(|parameter| {
            let minimum = values
                .iter()
                .map(|row| row[parameter])
                .fold(f64::INFINITY, f64::min);
            let maximum = values
                .iter()
                .map(|row| row[parameter])
                .fold(f64::NEG_INFINITY, f64::max);
            maximum - minimum
        })
        .collect()
}

pub fn run_frequency_max_spread_omnibus_permutation_v1<R: FrequencyMultigroupRefitterV1>(
    design: &FrequencyMultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    refitter: &mut R,
) -> Result<OmnibusPermutationResultV1, FrequencyMultigroupErrorV1> {
    let eligibility = eligible_design(design)?;
    validate_parameters(parameters)?;
    config
        .validate()
        .map_err(|error| FrequencyMultigroupErrorV1::InvalidDesign(error.to_string()))?;
    if design.groups.len() < 3 {
        return Err(FrequencyMultigroupErrorV1::InvalidDesign(
            "omnibus permutation requires at least three groups".into(),
        ));
    }
    let rows = canonical_rows(design);
    let mut point_values = Vec::with_capacity(design.groups.len());
    for group in &design.groups {
        point_values.push(observed_fit(design, group.index, parameters, refitter)?);
    }
    let observed = maximum_spreads(&point_values, parameters.len());
    let mut extremes = vec![0; parameters.len()];
    let mut null_spreads = vec![Vec::with_capacity(config.requested); parameters.len()];
    let mut usable = 0;
    let mut ledger = Vec::with_capacity(config.requested);
    for replicate in 0..config.requested {
        let samples = omnibus_samples(&rows, &design.groups, config.seed, replicate)?;
        let partition_sha256 = samples_digest(&samples);
        let mut values = Vec::with_capacity(design.groups.len());
        let mut group_fits = Vec::with_capacity(design.groups.len());
        let mut failed = false;
        for sample in samples {
            let fit = checked_fit(
                refitter,
                parameters,
                FrequencyMultigroupFitRequestV1 {
                    sample_kind: FitSampleKindV1::OmnibusPermutation,
                    group: sample.group,
                    replicate: Some(replicate),
                    source_rows: sample.source_rows,
                    counts: sample.counts,
                },
            );
            match fit {
                Ok(fit) => {
                    group_fits.push(usable_group(sample.group));
                    values.push(fit);
                }
                Err(failure) => {
                    failed = true;
                    group_fits.push(failed_group(sample.group, failure));
                }
            }
        }
        let status = if failed {
            ResampleFitStatusV1::Failed
        } else {
            let spread = maximum_spreads(&values, parameters.len());
            for index in 0..parameters.len() {
                null_spreads[index].push(spread[index]);
                if spread[index] >= observed[index] {
                    extremes[index] += 1;
                }
            }
            usable += 1;
            ResampleFitStatusV1::Usable
        };
        ledger.push(PermutationLedgerEntryV1 {
            replicate,
            partition_sha256,
            status,
            group_fits,
        });
    }
    let minimum_usable = config.minimum_usable();
    let availability = if usable >= minimum_usable {
        InferenceAvailabilityV1::Available
    } else {
        InferenceAvailabilityV1::InsufficientUsableResamples
    };
    let parameters_out = if availability == InferenceAvailabilityV1::Available {
        parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| OmnibusPermutationParameterV1 {
                parameter: parameter.clone(),
                observed_maximum_pairwise_spread: observed[index],
                p_value_right_tailed: add_one_probability(extremes[index], usable),
                null_maximum_pairwise_spreads: null_spreads[index].clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let plan_sha256 = sha256(
        &ledger
            .iter()
            .flat_map(|entry| {
                let mut bytes = (entry.replicate as u64).to_le_bytes().to_vec();
                bytes.extend_from_slice(entry.partition_sha256.as_bytes());
                bytes
            })
            .collect::<Vec<_>>(),
    );
    Ok(OmnibusPermutationResultV1 {
        method_version: FREQUENCY_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1.into(),
        seed: config.seed,
        requested: config.requested,
        attempted: ledger.len(),
        usable,
        failed: ledger.len() - usable,
        minimum_usable,
        retry_policy: "none".into(),
        plan_sha256,
        availability,
        group_point_estimates: design
            .groups
            .iter()
            .zip(point_values)
            .map(|(group, values)| GroupParameterVectorV1 {
                group: group.index,
                values,
            })
            .collect(),
        parameters: parameters_out,
        ledger,
        group_counts: eligibility.group_counts,
        eligibility_warnings: eligibility.warnings,
    })
}

pub fn run_frequency_group_bootstrap_banks_v1<R: FrequencyMultigroupRefitterV1>(
    design: &FrequencyMultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    refitter: &mut R,
) -> Result<GroupBootstrapBanksV1, FrequencyMultigroupErrorV1> {
    let eligibility = eligible_design(design)?;
    validate_parameters(parameters)?;
    config
        .validate()
        .map_err(|error| FrequencyMultigroupErrorV1::InvalidDesign(error.to_string()))?;
    let rows = canonical_rows(design);
    let source_samples = design
        .groups
        .iter()
        .map(|group| group_sample(&rows, group.index))
        .collect::<Vec<_>>();
    let mut banks = Vec::with_capacity(design.groups.len());
    for group in &design.groups {
        banks.push(GroupBootstrapBankV1 {
            group: group.index,
            point_estimates: observed_fit(design, group.index, parameters, refitter)?,
            usable: 0,
            failed: 0,
            replicate_estimates: Vec::with_capacity(config.requested),
        });
    }
    let mut ledger = Vec::with_capacity(config.requested);
    for replicate in 0..config.requested {
        let mut group_ledger = Vec::with_capacity(design.groups.len());
        let mut all_usable = true;
        for (position, source) in source_samples.iter().enumerate() {
            let total = source.counts.iter().sum::<u64>();
            let mut rng = ChaCha20Rng::from_seed(derive_seed(
                config.seed,
                BOOTSTRAP_STREAM,
                replicate,
                &[u8::from(source.group)],
            ));
            let draw = multinomial_draw(&source.counts, total, &mut rng)?;
            let selected = source
                .source_rows
                .iter()
                .copied()
                .zip(draw)
                .filter(|(_, count)| *count > 0)
                .collect::<Vec<_>>();
            let sample = FrequencyGroupSampleV1 {
                group: source.group,
                source_rows: selected.iter().map(|(row, _)| *row).collect(),
                counts: selected.iter().map(|(_, count)| *count).collect(),
            };
            let digest = samples_digest(std::slice::from_ref(&sample));
            let fit = checked_fit(
                refitter,
                parameters,
                FrequencyMultigroupFitRequestV1 {
                    sample_kind: FitSampleKindV1::GroupBootstrap,
                    group: source.group,
                    replicate: Some(replicate),
                    source_rows: sample.source_rows,
                    counts: sample.counts,
                },
            );
            match fit {
                Ok(values) => {
                    banks[position].usable += 1;
                    banks[position].replicate_estimates.push(Some(values));
                    group_ledger.push(BootstrapGroupLedgerV1 {
                        group: source.group,
                        sample_sha256: digest,
                        status: ResampleFitStatusV1::Usable,
                        failure: None,
                    });
                }
                Err(failure) => {
                    all_usable = false;
                    banks[position].failed += 1;
                    banks[position].replicate_estimates.push(None);
                    group_ledger.push(BootstrapGroupLedgerV1 {
                        group: source.group,
                        sample_sha256: digest,
                        status: ResampleFitStatusV1::Failed,
                        failure: Some(failure),
                    });
                }
            }
        }
        ledger.push(BootstrapLedgerEntryV1 {
            replicate,
            status: if all_usable {
                ResampleFitStatusV1::Usable
            } else {
                ResampleFitStatusV1::Failed
            },
            groups: group_ledger,
        });
    }
    let minimum_usable = config.minimum_usable();
    let availability = if banks.iter().all(|bank| bank.usable >= minimum_usable) {
        InferenceAvailabilityV1::Available
    } else {
        InferenceAvailabilityV1::InsufficientUsableResamples
    };
    let plan_sha256 = sha256(
        &ledger
            .iter()
            .flat_map(|entry| {
                let mut bytes = (entry.replicate as u64).to_le_bytes().to_vec();
                for group in &entry.groups {
                    bytes.extend_from_slice(group.sample_sha256.as_bytes());
                }
                bytes
            })
            .collect::<Vec<_>>(),
    );
    Ok(GroupBootstrapBanksV1 {
        method_version: FREQUENCY_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1.into(),
        parameters: parameters.to_vec(),
        seed: config.seed,
        requested: config.requested,
        attempted: ledger.len(),
        minimum_usable,
        retry_policy: "none".into(),
        plan_sha256,
        availability,
        groups: banks,
        ledger,
        group_counts: eligibility.group_counts,
        eligibility_warnings: eligibility.warnings,
    })
}

fn type7(values: &mut [f64], probability: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let position = (values.len() - 1) as f64 * probability;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let fraction = position - lower as f64;
        values[lower] * (1.0 - fraction) + values[upper] * fraction
    }
}

fn weighted_mean(values: &[f64], counts: &[u64]) -> Result<f64, FrequencyMultigroupErrorV1> {
    if values.len() != counts.len() || values.is_empty() {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "weighted score/count dimensions differ".into(),
        ));
    }
    let total = counts.iter().sum::<u64>();
    if total == 0 {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "weighted score statistic has zero total count".into(),
        ));
    }
    Ok(values
        .iter()
        .zip(counts)
        .map(|(value, count)| value * *count as f64)
        .sum::<f64>()
        / total as f64)
}

fn expanded_variance(values: &[f64], counts: &[u64]) -> Result<f64, FrequencyMultigroupErrorV1> {
    let total = counts.iter().sum::<u64>();
    if total < 2 {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "expanded variance requires at least two cases".into(),
        ));
    }
    let mean = weighted_mean(values, counts)?;
    Ok(values
        .iter()
        .zip(counts)
        .map(|(value, count)| *count as f64 * (value - mean).powi(2))
        .sum::<f64>()
        / (total - 1) as f64)
}

fn expanded_correlation(
    left: &[f64],
    right: &[f64],
    counts: &[u64],
) -> Result<f64, FrequencyMultigroupErrorV1> {
    let left_mean = weighted_mean(left, counts)?;
    let right_mean = weighted_mean(right, counts)?;
    let mut cross = 0.0;
    let mut left_square = 0.0;
    let mut right_square = 0.0;
    for ((left, right), count) in left.iter().zip(right).zip(counts) {
        let weight = *count as f64;
        cross += weight * (left - left_mean) * (right - right_mean);
        left_square += weight * (left - left_mean).powi(2);
        right_square += weight * (right - right_mean).powi(2);
    }
    let denominator = (left_square * right_square).sqrt();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "frequency MICOM compositional score has zero variance".into(),
        ));
    }
    Ok((cross / denominator).clamp(-1.0, 1.0))
}

fn micom_score_maps(
    fit: &MicomFitV1,
    pair: OrderedGroupPairV1,
    construct_ids: &[String],
    expected_len: usize,
) -> Result<BTreeMap<(GroupIndexV1, String), Vec<f64>>, FrequencyMultigroupErrorV1> {
    let mut map = BTreeMap::new();
    for row in &fit.scores {
        if ![pair.group_a, pair.group_b].contains(&row.group)
            || !construct_ids.contains(&row.construct_id)
            || row.pooled_scores.len() != expected_len
            || row.pooled_scores.iter().any(|value| !value.is_finite())
            || map
                .insert(
                    (row.group, row.construct_id.clone()),
                    row.pooled_scores.clone(),
                )
                .is_some()
        {
            return Err(FrequencyMultigroupErrorV1::MicomContract(
                "frequency MICOM scoring output is incomplete, duplicate, or nonfinite".into(),
            ));
        }
    }
    if map.len() != construct_ids.len() * 2 {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "frequency MICOM scoring output omitted a group-construct cell".into(),
        ));
    }
    Ok(map)
}

fn pooled_score_map(
    fit: &MicomFitV1,
    construct_ids: &[String],
    expected_len: usize,
) -> Result<BTreeMap<String, Vec<f64>>, FrequencyMultigroupErrorV1> {
    let mut map = BTreeMap::new();
    for row in &fit.pooled_reference_scores {
        if !construct_ids.contains(&row.construct_id)
            || row.pooled_scores.len() != expected_len
            || row.pooled_scores.iter().any(|value| !value.is_finite())
            || map
                .insert(row.construct_id.clone(), row.pooled_scores.clone())
                .is_some()
        {
            return Err(FrequencyMultigroupErrorV1::MicomContract(
                "frequency MICOM pooled Step-3 scores are invalid".into(),
            ));
        }
    }
    if map.len() != construct_ids.len() {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "frequency MICOM pooled Step-3 scores are incomplete".into(),
        ));
    }
    Ok(map)
}

#[derive(Debug, Clone, Copy)]
struct FrequencyMicomStatisticV1 {
    correlation: f64,
    mean_difference: f64,
    log_variance_ratio: f64,
}

fn scoring_counts(
    scoring_rows: &[u64],
    samples: &[FrequencyMicomGroupTrainingV1],
) -> Result<Vec<u64>, FrequencyMultigroupErrorV1> {
    let mut by_row = BTreeMap::<u64, u64>::new();
    for sample in samples {
        if sample.source_rows.len() != sample.counts.len() {
            return Err(FrequencyMultigroupErrorV1::MicomContract(
                "frequency MICOM training row/count dimensions differ".into(),
            ));
        }
        for (row, count) in sample.source_rows.iter().zip(&sample.counts) {
            let entry = by_row.entry(*row).or_default();
            *entry = entry.checked_add(*count).ok_or_else(|| {
                FrequencyMultigroupErrorV1::MicomContract(
                    "frequency MICOM pooled count overflowed".into(),
                )
            })?;
        }
    }
    scoring_rows
        .iter()
        .map(|row| {
            by_row.get(row).copied().ok_or_else(|| {
                FrequencyMultigroupErrorV1::MicomContract(
                    "frequency MICOM scoring row has no pooled count".into(),
                )
            })
        })
        .collect()
}

fn membership_counts(
    scoring_rows: &[u64],
    sample: &FrequencyMicomGroupTrainingV1,
) -> Result<Vec<u64>, FrequencyMultigroupErrorV1> {
    if sample.source_rows.len() != sample.counts.len() {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "frequency MICOM membership row/count dimensions differ".into(),
        ));
    }
    let map = sample
        .source_rows
        .iter()
        .copied()
        .zip(sample.counts.iter().copied())
        .collect::<BTreeMap<_, _>>();
    Ok(scoring_rows
        .iter()
        .map(|row| map.get(row).copied().unwrap_or(0))
        .collect())
}

fn micom_statistics(
    scores: &BTreeMap<(GroupIndexV1, String), Vec<f64>>,
    pooled: &BTreeMap<String, Vec<f64>>,
    pair: OrderedGroupPairV1,
    construct_ids: &[String],
    scoring_rows: &[u64],
    training: &[FrequencyMicomGroupTrainingV1],
    pooled_counts: &[u64],
) -> Result<BTreeMap<String, FrequencyMicomStatisticV1>, FrequencyMultigroupErrorV1> {
    let sample_a = training
        .iter()
        .find(|sample| sample.group == pair.group_a)
        .ok_or_else(|| FrequencyMultigroupErrorV1::MicomContract("group A is absent".into()))?;
    let sample_b = training
        .iter()
        .find(|sample| sample.group == pair.group_b)
        .ok_or_else(|| FrequencyMultigroupErrorV1::MicomContract("group B is absent".into()))?;
    let counts_a = membership_counts(scoring_rows, sample_a)?;
    let counts_b = membership_counts(scoring_rows, sample_b)?;
    let mut output = BTreeMap::new();
    for construct in construct_ids {
        let score_a = &scores[&(pair.group_a, construct.clone())];
        let score_b = &scores[&(pair.group_b, construct.clone())];
        let common = &pooled[construct];
        let variance_a = expanded_variance(common, &counts_a)?;
        let variance_b = expanded_variance(common, &counts_b)?;
        if variance_a <= f64::EPSILON || variance_b <= f64::EPSILON {
            return Err(FrequencyMultigroupErrorV1::MicomContract(
                "frequency MICOM Step-3 variance is degenerate".into(),
            ));
        }
        output.insert(
            construct.clone(),
            FrequencyMicomStatisticV1 {
                correlation: expanded_correlation(score_a, score_b, pooled_counts)?,
                mean_difference: weighted_mean(common, &counts_a)?
                    - weighted_mean(common, &counts_b)?,
                log_variance_ratio: (variance_a / variance_b).ln(),
            },
        );
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub fn run_frequency_pairwise_micom_with_partition_plan_v1<R, C>(
    refitter: &mut R,
    design: &FrequencyMultigroupDesignV1,
    pair: OrderedGroupPairV1,
    construct_ids: &[String],
    configural_receipt: MicomConfiguralReceiptV1,
    config: MicomPermutationConfigV1,
    plan: &FrequencyPairwisePartitionPlanV1,
    cancelled: C,
) -> Result<MicomPairwiseResultV1, FrequencyMultigroupErrorV1>
where
    R: FrequencyMicomRefitterV1,
    C: Fn() -> bool,
{
    eligible_design(design)?;
    if !configural_receipt.complete()
        || construct_ids.is_empty()
        || construct_ids.iter().collect::<BTreeSet<_>>().len() != construct_ids.len()
    {
        return Err(FrequencyMultigroupErrorV1::MicomContract(
            "Step 1 and unique construct identities are required".into(),
        ));
    }
    let resampling = MultigroupResamplingConfigV1 {
        requested: config.requested,
        seed: config.seed,
        confidence_level: 0.95,
        alpha: config.alpha,
        alternative: AlternativeHypothesisV1::TwoSided,
    };
    let rows = validate_frequency_plan(design, pair, resampling, plan)?;
    let mut scoring_rows = rows.iter().map(|row| row.source_row).collect::<Vec<_>>();
    scoring_rows.sort_unstable();
    let observed_training = [pair.group_a, pair.group_b]
        .into_iter()
        .map(|group| {
            let sample = group_sample(&rows, group);
            FrequencyMicomGroupTrainingV1 {
                group,
                source_rows: sample.source_rows,
                counts: sample.counts,
            }
        })
        .collect::<Vec<_>>();
    let pooled_counts = scoring_counts(&scoring_rows, &observed_training)?;
    let observed_fit = refitter
        .fit_frequency_micom(&FrequencyMicomFitRequestV1 {
            kind: MicomFitKindV1::Observed,
            replicate: None,
            training_groups: observed_training.clone(),
            scoring_rows: scoring_rows.clone(),
        })
        .map_err(|failure| FrequencyMultigroupErrorV1::MicomObservedRefit(failure.detail))?;
    let observed_scores = micom_score_maps(&observed_fit, pair, construct_ids, scoring_rows.len())?;
    let pooled = pooled_score_map(&observed_fit, construct_ids, scoring_rows.len())?;
    let observed = micom_statistics(
        &observed_scores,
        &pooled,
        pair,
        construct_ids,
        &scoring_rows,
        &observed_training,
        &pooled_counts,
    )?;
    let mut correlations = construct_ids
        .iter()
        .map(|id| (id.clone(), Vec::with_capacity(config.requested)))
        .collect::<BTreeMap<_, _>>();
    let mut means = correlations.clone();
    let mut variances = correlations.clone();
    let mut ledger = Vec::with_capacity(config.requested);
    for replicate in 0..config.requested {
        if cancelled() {
            for later in replicate..config.requested {
                ledger.push(MicomPermutationLedgerEntryV1 {
                    replicate: later,
                    seed: config.seed,
                    partition_sha256: plan.entries[later].partition_sha256.clone(),
                    status: MicomPermutationStatusV1::Failed {
                        code: RefitFailureCodeV1::Cancelled,
                        detail: "cancelled before frequency MICOM refit".into(),
                    },
                });
            }
            break;
        }
        let partition = materialize_frequency_pairwise_partition_v1(design, pair, plan, replicate)?;
        let training = partition
            .samples
            .iter()
            .map(|sample| FrequencyMicomGroupTrainingV1 {
                group: sample.group,
                source_rows: sample.source_rows.clone(),
                counts: sample.counts.clone(),
            })
            .collect::<Vec<_>>();
        let request = FrequencyMicomFitRequestV1 {
            kind: MicomFitKindV1::Permutation,
            replicate: Some(replicate),
            training_groups: training.clone(),
            scoring_rows: scoring_rows.clone(),
        };
        let statistics = refitter.fit_frequency_micom(&request).and_then(|fit| {
            if !fit.pooled_reference_scores.is_empty() {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    "frequency permutation MICOM replaced frozen Step-3 scores",
                ));
            }
            let scores = micom_score_maps(&fit, pair, construct_ids, scoring_rows.len()).map_err(
                |error| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        error.to_string(),
                    )
                },
            )?;
            micom_statistics(
                &scores,
                &pooled,
                pair,
                construct_ids,
                &scoring_rows,
                &training,
                &pooled_counts,
            )
            .map_err(|error| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    error.to_string(),
                )
            })
        });
        match statistics {
            Ok(statistics) => {
                for (construct, statistic) in statistics {
                    correlations
                        .get_mut(&construct)
                        .expect("declared construct")
                        .push(statistic.correlation);
                    means
                        .get_mut(&construct)
                        .expect("declared construct")
                        .push(statistic.mean_difference);
                    variances
                        .get_mut(&construct)
                        .expect("declared construct")
                        .push(statistic.log_variance_ratio);
                }
                ledger.push(MicomPermutationLedgerEntryV1 {
                    replicate,
                    seed: config.seed,
                    partition_sha256: partition.partition_sha256,
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
    for (construct_position, construct) in construct_ids.iter().enumerate() {
        let point = observed[construct];
        let correlation_values = correlations.get_mut(construct).expect("declared construct");
        let lower = complete.then(|| type7(correlation_values, config.alpha));
        let compositional_probability = complete.then(|| {
            add_one_probability(
                correlation_values
                    .iter()
                    .filter(|value| **value <= point.correlation)
                    .count(),
                usable,
            )
        });
        let mean_probability = complete.then(|| {
            add_one_probability(
                means[construct]
                    .iter()
                    .filter(|value| value.abs() >= point.mean_difference.abs())
                    .count(),
                usable,
            )
        });
        let variance_probability = complete.then(|| {
            add_one_probability(
                variances[construct]
                    .iter()
                    .filter(|value| value.abs() >= point.log_variance_ratio.abs())
                    .count(),
                usable,
            )
        });
        let compositional_invariance =
            complete && lower.is_some_and(|threshold| point.correlation >= threshold);
        let equal_means = mean_probability.is_some_and(|probability| probability >= config.alpha);
        let equal_variances =
            variance_probability.is_some_and(|probability| probability >= config.alpha);
        constructs.push(MicomConstructResultV1 {
            construct_id: construct.clone(),
            observed_compositional_correlation: point.correlation,
            compositional_lower_quantile: lower,
            compositional_invariance_probability: compositional_probability,
            compositional_invariance,
            observed_mean_difference_a_minus_b: point.mean_difference,
            mean_difference_two_sided_probability: mean_probability,
            equal_means,
            observed_log_variance_ratio_a_minus_b: point.log_variance_ratio,
            variance_difference_two_sided_probability: variance_probability,
            equal_variances,
            partial_measurement_invariance: configural_receipt.complete()
                && compositional_invariance,
            full_measurement_invariance: configural_receipt.complete()
                && compositional_invariance
                && equal_means
                && equal_variances,
            permutation_compositional_correlations: correlation_values.clone(),
            permutation_mean_differences: if construct_position == 0 {
                means[construct].clone()
            } else {
                Vec::new()
            },
            permutation_log_variance_ratios: if construct_position == 0 {
                variances[construct].clone()
            } else {
                Vec::new()
            },
        });
    }
    let ledger_sha256 = sha256(
        &serde_json::to_vec(&ledger)
            .map_err(|error| FrequencyMultigroupErrorV1::MicomContract(error.to_string()))?,
    );
    Ok(MicomPairwiseResultV1 {
        method_version: FREQUENCY_MICOM_PAIRWISE_METHOD_VERSION_V1.into(),
        pair,
        configural_receipt,
        requested_permutations: config.requested,
        usable_permutations: usable,
        minimum_usable_permutations: minimum_usable,
        partition_plan_sha256: plan.plan_sha256.clone(),
        ledger_sha256,
        ledger,
        constructs,
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ParameterEstimateV1, ParameterFamilyV1, TypedGroupValueV1};

    fn design() -> FrequencyMultigroupDesignV1 {
        FrequencyMultigroupDesignV1 {
            groups: (0..3)
                .map(|index| GroupIdentityV1 {
                    index: GroupIndexV1::new(index).unwrap(),
                    value: TypedGroupValueV1::Integer {
                        value: index as i64,
                    },
                    display_label: format!("G{index}"),
                })
                .collect(),
            rows: vec![
                (0, 0, 4),
                (1, 0, 6),
                (2, 1, 3),
                (3, 1, 7),
                (4, 2, 5),
                (5, 2, 5),
            ]
            .into_iter()
            .map(
                |(source_row, group, frequency)| FrequencySelectedGroupRowV1 {
                    source_row,
                    group: GroupIndexV1::new(group).unwrap(),
                    frequency,
                },
            )
            .collect(),
        }
    }

    fn parameter() -> ParameterIdentityV1 {
        ParameterIdentityV1 {
            stable_id: "path:x:y".into(),
            family: ParameterFamilyV1::StructuralPath,
        }
    }

    #[test]
    fn point_counts_are_exactly_the_expanded_group_membership() {
        let design = design();
        let rows = canonical_rows(&design);
        for group in &design.groups {
            let sample = group_sample(&rows, group.index);
            let expanded = sample
                .source_rows
                .iter()
                .zip(&sample.counts)
                .flat_map(|(row, count)| std::iter::repeat_n(*row, *count as usize))
                .collect::<Vec<_>>();
            assert_eq!(
                expanded.len() as u64,
                design
                    .rows
                    .iter()
                    .filter(|row| row.group == group.index)
                    .map(|row| row.frequency)
                    .sum::<u64>()
            );
        }
    }

    #[test]
    fn pairwise_count_partition_matches_its_expanded_membership_and_reverses_exactly() {
        let design = design();
        let forward =
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap();
        let reverse = OrderedGroupPairV1::new(forward.group_b, forward.group_a).unwrap();
        let plan = build_frequency_pairwise_partition_plan_v1(&design, forward, 5_000, 42).unwrap();
        let a = materialize_frequency_pairwise_partition_v1(&design, forward, &plan, 17).unwrap();
        let b = materialize_frequency_pairwise_partition_v1(&design, reverse, &plan, 17).unwrap();
        assert_eq!(a.partition_sha256, b.partition_sha256);
        assert_eq!(a.samples, b.samples);
        assert_eq!(a.samples[0].counts.iter().sum::<u64>(), 10);
        assert_eq!(a.samples[1].counts.iter().sum::<u64>(), 10);
        let expanded_total = a
            .samples
            .iter()
            .flat_map(|sample| sample.counts.iter())
            .sum::<u64>();
        assert_eq!(expanded_total, 20);
    }

    #[test]
    fn multinomial_bootstrap_count_fit_equals_expanded_row_mean() {
        let design = design();
        let parameter = parameter();
        let values = BTreeMap::from([(0_u64, 1.0), (1_u64, 5.0)]);
        let mut observed = None;
        let mut refitter = |request: &FrequencyMultigroupFitRequestV1| {
            let count_total = request.counts.iter().sum::<u64>() as f64;
            let collapsed = request
                .source_rows
                .iter()
                .zip(&request.counts)
                .map(|(row, count)| values.get(row).copied().unwrap_or(0.0) * *count as f64)
                .sum::<f64>()
                / count_total;
            let expanded = request
                .source_rows
                .iter()
                .zip(&request.counts)
                .flat_map(|(row, count)| {
                    std::iter::repeat_n(values.get(row).copied().unwrap_or(0.0), *count as usize)
                })
                .sum::<f64>()
                / count_total;
            assert_eq!(collapsed.to_bits(), expanded.to_bits());
            if request.sample_kind == FitSampleKindV1::GroupBootstrap
                && request.group == GroupIndexV1::new(0).unwrap()
                && request.replicate == Some(0)
            {
                observed = Some(request.counts.iter().sum::<u64>());
            }
            Ok(ParameterVectorV1 {
                parameters: vec![ParameterEstimateV1 {
                    parameter: parameter.clone(),
                    estimate: collapsed,
                }],
            })
        };
        let banks = run_frequency_group_bootstrap_banks_v1(
            &design,
            std::slice::from_ref(&parameter),
            MultigroupResamplingConfigV1::official_defaults(),
            &mut refitter,
        )
        .unwrap();
        assert_eq!(observed, Some(10));
        assert_eq!(banks.groups[0].replicate_estimates.len(), 5_000);
    }

    #[test]
    fn frequency_omnibus_retains_every_usable_null_spread_for_tail_replay() {
        let design = design();
        let parameter = parameter();
        let mut refitter = |request: &FrequencyMultigroupFitRequestV1| {
            let represented = request.counts.iter().sum::<u64>() as f64;
            let estimate = request
                .source_rows
                .iter()
                .zip(&request.counts)
                .map(|(row, count)| *row as f64 * *count as f64)
                .sum::<f64>()
                / represented;
            Ok(ParameterVectorV1 {
                parameters: vec![ParameterEstimateV1 {
                    parameter: parameter.clone(),
                    estimate,
                }],
            })
        };
        let result = run_frequency_max_spread_omnibus_permutation_v1(
            &design,
            std::slice::from_ref(&parameter),
            MultigroupResamplingConfigV1::official_defaults(),
            &mut refitter,
        )
        .unwrap();

        assert_eq!(result.usable, result.requested);
        let inference = &result.parameters[0];
        assert_eq!(inference.null_maximum_pairwise_spreads.len(), result.usable);
        assert!(
            inference
                .null_maximum_pairwise_spreads
                .iter()
                .all(|spread| spread.is_finite() && *spread >= 0.0)
        );
        let extreme = inference
            .null_maximum_pairwise_spreads
            .iter()
            .filter(|spread| **spread >= inference.observed_maximum_pairwise_spread)
            .count();
        let replay = (extreme + 1) as f64 / (result.usable + 1) as f64;
        assert_eq!(inference.p_value_right_tailed.to_bits(), replay.to_bits());
    }
}
