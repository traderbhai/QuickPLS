//! Exact count-space resampling for positive-integer frequency-weighted MGA.
//!
//! A physical row is retained once with an integer multiplicity. Permutation
//! allocations are multivariate-hypergeometric and preserve expanded group
//! totals; bootstrap allocations are multinomial with probabilities f_i/N.
//! No operation materializes the expanded row vector.

use crate::multigroup_v1::{mga_greater_or_tied_v1, mga_less_or_tied_v1};
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
use statrs::distribution::{Binomial, DiscreteCDF};
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
const MAX_EXACT_FREQUENCY_TOTAL: u64 = (1_u64 << 53) - 1;
const HYPERGEOMETRIC_DIRECT_THRESHOLD: u64 = 10;
const HYPERGEOMETRIC_MAX_REJECTION_ATTEMPTS: usize = 1_000_000;
const HRUA_LOG_RATIO_OVERSHOOT_TOLERANCE: f64 = 1e-10;
const HRUA_D1: f64 = 1.715_527_769_921_413_5; // 2 * sqrt(2 / e)
const HRUA_D2: f64 = 0.898_916_162_058_898_8; // 3 - 2 * sqrt(3 / e)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrequencySelectedGroupRowV1 {
    pub source_row: u64,
    pub stable_row_token: u64,
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
                stable_row_token: row.stable_row_token,
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
    rows.sort_by_key(|row| row.stable_row_token);
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

fn micom_ledger_sha256(
    ledger: &[MicomPermutationLedgerEntryV1],
) -> Result<String, FrequencyMultigroupErrorV1> {
    let bytes = serde_json::to_vec(ledger)
        .map_err(|error| FrequencyMultigroupErrorV1::MicomContract(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
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

fn bounded_discrete_inverse_cdf<F>(lower: u64, upper: u64, probability: f64, mut cdf: F) -> u64
where
    F: FnMut(u64) -> f64,
{
    debug_assert!(lower <= upper);
    debug_assert!(probability > 0.0 && probability < 1.0);
    let mut left = lower;
    let mut right = upper;
    while left < right {
        let midpoint = left + (right - left) / 2;
        if cdf(midpoint) >= probability {
            right = midpoint;
        } else {
            left = midpoint + 1;
        }
    }
    left
}

// The direct-complement and HRUA structure below is adapted from NumPy commit
// ffa72d99810dc54fa4222c3ffc623c4b268191b1, file
// `numpy/random/src/distributions/random_hypergeometric.c` (NumPy Developers,
// BSD-3-Clause; full source digest and notice in THIRD_PARTY_NOTICES.md). NumPy
// explicitly limits each population class to less than 1e9 because subtracting
// four large log-factorials loses precision. QuickPLS admits totals through
// 2^53-1, so a shifted, analytically combined log-ratio avoids that cancellation.

fn log1pmx(value: f64) -> f64 {
    debug_assert!(value > -1.0);
    if value.abs() < 0.01 {
        // log(1+x)-x, evaluated without cancellation. At |x|<0.01 the first
        // omitted term after twenty terms is below 5e-43.
        let mut power = value * value;
        let mut sum = -power / 2.0;
        for denominator in 3..=20 {
            power *= value;
            let term = power / f64::from(denominator);
            if denominator % 2 == 0 {
                sum -= term;
            } else {
                sum += term;
            }
        }
        sum
    } else {
        value.ln_1p() - value
    }
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum: f64 = 0.0;
    let mut correction: f64 = 0.0;
    for value in values {
        let next = sum + value;
        correction += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    sum + correction
}

fn stirling_correction(value: f64) -> f64 {
    let inverse = value.recip();
    let inverse_squared = inverse * inverse;
    inverse
        * (1.0 / 12.0
            + inverse_squared
                * (-1.0 / 360.0
                    + inverse_squared * (1.0 / 1_260.0 + inverse_squared * (-1.0 / 1_680.0))))
}

fn log_u128_ratio(numerator: u128, denominator: u128) -> f64 {
    debug_assert!(numerator > 0 && denominator > 0);
    if numerator == denominator {
        return 0.0;
    }
    if numerator > denominator {
        ((numerator - denominator) as f64 / denominator as f64).ln_1p()
    } else {
        -((denominator - numerator) as f64 / numerator as f64).ln_1p()
    }
}

fn signed_cell(cell: u64, sign: i64, delta: i64) -> u64 {
    let value = i128::from(cell) + i128::from(sign) * i128::from(delta);
    u64::try_from(value).expect("candidate is inside the hypergeometric support")
}

fn interior_hypergeometric_log_ratio(cells: [u64; 4], delta: i64) -> f64 {
    const SIGNS: [i64; 4] = [1, -1, -1, 1];
    debug_assert!(cells.iter().all(|cell| *cell >= 128));
    let candidate_cells: [u64; 4] = std::array::from_fn(|index| {
        let value = signed_cell(cells[index], SIGNS[index], delta);
        debug_assert!(value >= 128);
        value
    });
    let cross_ratio = log_u128_ratio(
        u128::from(cells[0]) * u128::from(cells[3]),
        u128::from(cells[1]) * u128::from(cells[2]),
    );
    let delta = delta as f64;
    let inverse_sum = compensated_sum(cells.map(|cell| (cell as f64).recip()));
    let signed_inverse_sum = compensated_sum(
        cells
            .iter()
            .zip(SIGNS)
            .map(|(cell, sign)| sign as f64 / *cell as f64),
    );
    let log1pmx_sum = compensated_sum(cells.iter().zip(candidate_cells).zip(SIGNS).map(
        |((cell, candidate), sign)| {
            (candidate as f64 + 0.5) * log1pmx(sign as f64 * delta / *cell as f64)
        },
    ));
    let correction_sum =
        compensated_sum(cells.iter().zip(candidate_cells).map(|(cell, candidate)| {
            stirling_correction(*cell as f64) - stirling_correction(candidate as f64)
        }));
    compensated_sum([
        -delta * cross_ratio,
        -delta * delta * inverse_sum,
        -0.5 * delta * signed_inverse_sum,
        -log1pmx_sum,
        correction_sum,
    ])
}

fn hypergeometric_cells(
    mode: u64,
    minimum_class: u64,
    maximum_class: u64,
    computed_sample: u64,
) -> [u64; 4] {
    [
        mode,
        minimum_class - mode,
        computed_sample - mode,
        maximum_class - computed_sample + mode,
    ]
}

fn log_ratio_requires_shift(cells: [u64; 4], delta: i64) -> bool {
    const SIGNS: [i64; 4] = [1, -1, -1, 1];
    cells.iter().any(|cell| *cell < 128)
        || cells
            .into_iter()
            .zip(SIGNS)
            .any(|(cell, sign)| signed_cell(cell, sign, delta) < 128)
}

fn hypergeometric_log_ratio(
    mode: u64,
    candidate: u64,
    minimum_class: u64,
    maximum_class: u64,
    computed_sample: u64,
) -> f64 {
    const SHIFT: u64 = 128;
    const SIGNS: [i64; 4] = [1, -1, -1, 1];
    let delta = i64::try_from(candidate).expect("admitted frequency bound fits i64")
        - i64::try_from(mode).expect("admitted frequency bound fits i64");
    if delta == 0 {
        return 0.0;
    }
    let cells = hypergeometric_cells(mode, minimum_class, maximum_class, computed_sample);
    debug_assert!(SIGNS.into_iter().enumerate().all(|(index, sign)| {
        i128::from(cells[index]) + i128::from(sign) * i128::from(delta) >= 0
    }));
    if !log_ratio_requires_shift(cells, delta) {
        return interior_hypergeometric_log_ratio(cells, delta);
    }
    // Shifting every factorial cell by the same fixed amount gives an exact
    // identity. It keeps both baseline and candidate cells in the accurate
    // Stirling interior while the correction restores the original factorials.
    let shifted_cells = cells.map(|cell| cell + SHIFT);
    let shifted_ratio = interior_hypergeometric_log_ratio(shifted_cells, delta);
    let delta_float = delta as f64;
    let exact_shift_correction = compensated_sum((1..=SHIFT).flat_map(|offset| {
        cells
            .into_iter()
            .zip(SIGNS)
            .map(move |(cell, sign)| (sign as f64 * delta_float / (cell + offset) as f64).ln_1p())
    }));
    compensated_sum([shifted_ratio, exact_shift_correction])
}

fn direct_hypergeometric_draw(
    population: u64,
    successes: u64,
    draws: u64,
    rng: &mut ChaCha20Rng,
) -> u64 {
    let mut computed_sample = draws.min(population - draws);
    let mut remaining_population = population;
    let mut remaining_successes = successes;
    while computed_sample > 0
        && remaining_successes > 0
        && remaining_population > remaining_successes
    {
        if rng.random_range(0..remaining_population) < remaining_successes {
            remaining_successes -= 1;
        }
        remaining_population -= 1;
        computed_sample -= 1;
    }
    if remaining_population == remaining_successes {
        remaining_successes -= computed_sample;
    }
    if draws > population / 2 {
        remaining_successes
    } else {
        successes - remaining_successes
    }
}

fn hrua_proposal_upper_exclusive(computed_sample: u64, minimum_class: u64) -> u64 {
    computed_sample.min(minimum_class) + 1
}

fn hrua_hypergeometric_draw(
    population: u64,
    successes: u64,
    draws: u64,
    rng: &mut ChaCha20Rng,
) -> Result<u64, FrequencyMultigroupErrorV1> {
    let failures = population - successes;
    let minimum_class = successes.min(failures);
    let maximum_class = successes.max(failures);
    let computed_sample = draws.min(population - draws);
    let success_probability = minimum_class as f64 / population as f64;
    let failure_probability = maximum_class as f64 / population as f64;
    let mean = computed_sample as f64 * success_probability;
    let center = mean + 0.5;
    let variance = (population - computed_sample) as f64
        * computed_sample as f64
        * success_probability
        * failure_probability
        / (population - 1) as f64;
    let scale = (variance + 0.5).sqrt();
    let envelope = HRUA_D1 * scale + HRUA_D2;
    let mode = (((u128::from(computed_sample) + 1) * (u128::from(minimum_class) + 1))
        / (u128::from(population) + 2)) as u64;
    // Unlike NumPy's pragmatic 16-sigma cap, QuickPLS retains the complete
    // mathematical support. Extremely remote proposals are rare, but assigning
    // them zero probability would not be an exact count-space draw.
    let proposal_upper_exclusive = hrua_proposal_upper_exclusive(computed_sample, minimum_class);
    if proposal_upper_exclusive == 0
        || !center.is_finite()
        || !envelope.is_finite()
        || envelope <= 0.0
    {
        return Err(FrequencyMultigroupErrorV1::Distribution(
            "HRUA initialization produced a nonfinite or empty proposal".into(),
        ));
    }

    for _ in 0..HYPERGEOMETRIC_MAX_REJECTION_ATTEMPTS {
        let uniform = open_unit_interval(rng);
        let auxiliary = open_unit_interval(rng);
        let proposal = center + envelope * (auxiliary - 0.5) / uniform;
        if proposal < 0.0 || proposal >= proposal_upper_exclusive as f64 {
            continue;
        }
        let candidate = proposal.floor() as u64;
        let raw_log_ratio = hypergeometric_log_ratio(
            mode,
            candidate,
            minimum_class,
            maximum_class,
            computed_sample,
        );
        if !raw_log_ratio.is_finite() || raw_log_ratio > HRUA_LOG_RATIO_OVERSHOOT_TOLERANCE {
            return Err(FrequencyMultigroupErrorV1::Distribution(format!(
                "HRUA acceptance ratio is numerically invalid: {raw_log_ratio}"
            )));
        }
        // The exact integer mode makes this ratio nonpositive. Clamp only the
        // harmless final-rounding overshoot admitted by the tolerance above.
        let log_ratio = raw_log_ratio.min(0.0);
        if uniform * (4.0 - uniform) - 3.0 <= log_ratio
            || (uniform * (uniform - log_ratio) < 1.0 && 2.0 * uniform.ln() <= log_ratio)
        {
            let mut result = candidate;
            if successes > failures {
                result = computed_sample - result;
            }
            if computed_sample < draws {
                result = successes - result;
            }
            return Ok(result);
        }
    }
    Err(FrequencyMultigroupErrorV1::Distribution(format!(
        "HRUA did not accept a draw within {HYPERGEOMETRIC_MAX_REJECTION_ATTEMPTS} attempts"
    )))
}

fn hypergeometric_draw(
    population: u64,
    successes: u64,
    draws: u64,
    rng: &mut ChaCha20Rng,
) -> Result<u64, FrequencyMultigroupErrorV1> {
    if population > MAX_EXACT_FREQUENCY_TOTAL {
        return Err(FrequencyMultigroupErrorV1::Distribution(format!(
            "frequency population {population} exceeds the admitted exact-integer total {MAX_EXACT_FREQUENCY_TOTAL}"
        )));
    }
    if successes > population || draws > population {
        return Err(FrequencyMultigroupErrorV1::Distribution(
            "hypergeometric successes and draws must not exceed the population".into(),
        ));
    }
    if draws == 0 || successes == 0 {
        return Ok(0);
    }
    if successes == population {
        return Ok(draws);
    }
    let complement_draws = population - draws;
    let result = if draws.min(complement_draws) <= HYPERGEOMETRIC_DIRECT_THRESHOLD {
        direct_hypergeometric_draw(population, successes, draws, rng)
    } else {
        hrua_hypergeometric_draw(population, successes, draws, rng)?
    };
    let lower = draws.saturating_sub(population - successes);
    let upper = draws.min(successes);
    if result < lower || result > upper {
        return Err(FrequencyMultigroupErrorV1::Distribution(format!(
            "hypergeometric draw {result} fell outside [{lower}, {upper}]"
        )));
    }
    Ok(result)
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
            let distribution = Binomial::new(probability, remaining_draws)
                .map_err(|error| FrequencyMultigroupErrorV1::Distribution(error.to_string()))?;
            bounded_discrete_inverse_cdf(0, remaining_draws, open_unit_interval(rng), |value| {
                distribution.cdf(value)
            })
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
                if mga_greater_or_tied_v1(difference.abs(), observed[index].abs()) {
                    absolute[index] += 1;
                }
                if mga_greater_or_tied_v1(difference, observed[index]) {
                    greater[index] += 1;
                }
                if mga_less_or_tied_v1(difference, observed[index]) {
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
                if mga_greater_or_tied_v1(spread[index], observed[index]) {
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
                    .filter(|value| mga_less_or_tied_v1(**value, point.correlation))
                    .count(),
                usable,
            )
        });
        let mean_probability = complete.then(|| {
            add_one_probability(
                means[construct]
                    .iter()
                    .filter(|value| {
                        mga_greater_or_tied_v1(value.abs(), point.mean_difference.abs())
                    })
                    .count(),
                usable,
            )
        });
        let variance_probability = complete.then(|| {
            add_one_probability(
                variances[construct]
                    .iter()
                    .filter(|value| {
                        mga_greater_or_tied_v1(value.abs(), point.log_variance_ratio.abs())
                    })
                    .count(),
                usable,
            )
        });
        let compositional_invariance = complete
            && lower.is_some_and(|threshold| mga_greater_or_tied_v1(point.correlation, threshold));
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
    let ledger_sha256 = micom_ledger_sha256(&ledger)?;
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
    use std::time::Instant;

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
                    stable_row_token: source_row,
                    group: GroupIndexV1::new(group).unwrap(),
                    frequency,
                },
            )
            .collect(),
        }
    }

    #[test]
    fn frequency_micom_ledger_uses_canonical_bare_sha256() {
        let ledger = vec![MicomPermutationLedgerEntryV1 {
            replicate: 0,
            seed: 42,
            partition_sha256: "sha256:fixture".into(),
            status: MicomPermutationStatusV1::Usable,
        }];
        assert_eq!(
            micom_ledger_sha256(&ledger).unwrap(),
            "22c0812dd0b6d7efcb07355f7596b30113a01acd380d0c590266ab66873b4c7a"
        );
    }

    #[test]
    fn frequency_stable_tokens_preserve_count_space_draws_under_row_reversal() {
        let baseline = design();
        let mut reversed = baseline.clone();
        let maximum = baseline
            .rows
            .iter()
            .map(|row| row.source_row)
            .max()
            .unwrap();
        for row in &mut reversed.rows {
            row.source_row = maximum - row.source_row;
        }
        let pair =
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap();
        let baseline_rows = pair_rows(&baseline, pair).unwrap();
        let reversed_rows = pair_rows(&reversed, pair).unwrap();
        assert_eq!(
            baseline_rows
                .iter()
                .map(|row| (row.stable_row_token, row.frequency))
                .collect::<Vec<_>>(),
            reversed_rows
                .iter()
                .map(|row| (row.stable_row_token, row.frequency))
                .collect::<Vec<_>>()
        );
        let baseline_samples = materialize_pairwise(&baseline_rows, pair, 42, 17).unwrap();
        let reversed_samples = materialize_pairwise(&reversed_rows, pair, 42, 17).unwrap();
        for (baseline_sample, reversed_sample) in baseline_samples.iter().zip(&reversed_samples) {
            assert_eq!(baseline_sample.group, reversed_sample.group);
            assert_eq!(baseline_sample.counts, reversed_sample.counts);
            assert_eq!(
                baseline_sample.source_rows,
                reversed_sample
                    .source_rows
                    .iter()
                    .map(|row| maximum - row)
                    .collect::<Vec<_>>()
            );
        }

        let baseline_group =
            group_sample(&canonical_rows(&baseline), GroupIndexV1::new(0).unwrap());
        let reversed_group =
            group_sample(&canonical_rows(&reversed), GroupIndexV1::new(0).unwrap());
        let total = baseline_group.counts.iter().sum::<u64>();
        let mut baseline_rng = ChaCha20Rng::from_seed(derive_seed(42, BOOTSTRAP_STREAM, 31, &[0]));
        let mut reversed_rng = ChaCha20Rng::from_seed(derive_seed(42, BOOTSTRAP_STREAM, 31, &[0]));
        assert_eq!(
            multinomial_draw(&baseline_group.counts, total, &mut baseline_rng).unwrap(),
            multinomial_draw(&reversed_group.counts, total, &mut reversed_rng).unwrap()
        );
    }

    fn parameter() -> ParameterIdentityV1 {
        ParameterIdentityV1 {
            stable_id: "path:x:y".into(),
            family: ParameterFamilyV1::StructuralPath,
        }
    }

    fn combinations(total: u64, selected: u64) -> u128 {
        if selected > total {
            return 0;
        }
        let selected = selected.min(total - selected);
        (1..=selected).fold(1_u128, |value, index| {
            value * u128::from(total - selected + index) / u128::from(index)
        })
    }

    fn assert_matches_expanded_hypergeometric(
        population: u64,
        successes: u64,
        draws: u64,
        trials: usize,
        seed: [u8; 32],
    ) {
        let lower = draws.saturating_sub(population - successes);
        let upper = draws.min(successes);
        let mut observed = vec![0_usize; (upper - lower + 1) as usize];
        let mut rng = ChaCha20Rng::from_seed(seed);
        for _ in 0..trials {
            let draw = hypergeometric_draw(population, successes, draws, &mut rng).unwrap();
            observed[(draw - lower) as usize] += 1;
        }
        let denominator = combinations(population, draws) as f64;
        let mut expected_cdf = 0.0;
        let mut observed_cdf = 0.0;
        for value in lower..=upper {
            let ways = combinations(successes, value)
                * combinations(population - successes, draws - value);
            expected_cdf += ways as f64 / denominator;
            observed_cdf += observed[(value - lower) as usize] as f64 / trials as f64;
            assert!(
                (observed_cdf - expected_cdf).abs() < 0.01,
                "count-space CDF {observed_cdf} differs from expanded CDF {expected_cdf} at {value}"
            );
        }
    }

    #[test]
    fn small_count_space_hypergeometric_matches_literal_expanded_sampling() {
        // The first case exercises the exact direct-complement path; the
        // second exercises HRUA. The reference PMF counts all equally likely
        // subsets of the physically expanded population.
        assert_matches_expanded_hypergeometric(10, 4, 3, 100_000, [17; 32]);
        assert_matches_expanded_hypergeometric(40, 17, 19, 100_000, [29; 32]);
    }

    #[test]
    fn shifted_log_ratio_matches_high_precision_frequency_boundary_oracles() {
        let population = MAX_EXACT_FREQUENCY_TOTAL;
        let cases = [
            // Balanced 2^53-1 population: approximately 16 and 1 standard
            // deviations on both sides of the exact mode.
            (
                4_503_599_627_370_495,
                4_503_599_627_370_496,
                4_503_599_627_316_174,
                2_251_799_813_658_087,
                -379_625_063_i64,
                -128.000_000_170_606_85,
            ),
            (
                4_503_599_627_370_495,
                4_503_599_627_370_496,
                4_503_599_627_316_174,
                2_251_799_813_658_087,
                379_625_062,
                -127.999_999_833_432_06,
            ),
            (
                4_503_599_627_370_495,
                4_503_599_627_370_496,
                4_503_599_627_316_174,
                2_251_799_813_658_087,
                -23_726_567,
                -0.500_000_014_495_865_4,
            ),
            (
                4_503_599_627_370_495,
                4_503_599_627_370_496,
                4_503_599_627_316_174,
                2_251_799_813_658_087,
                23_726_566,
                -0.499_999_993_422_441,
            ),
            // Highly skewed class split at the same maximum population.
            (
                9_007_199_254_740,
                population - 9_007_199_254_740,
                4_503_599_627_370_495,
                4_503_599_627_370,
                -23_997_590,
                -128.000_007_609_462_9,
            ),
            (
                9_007_199_254_740,
                population - 9_007_199_254_740,
                4_503_599_627_370_495,
                4_503_599_627_370,
                23_997_589,
                -127.999_996_952_392_34,
            ),
        ];
        for (minimum, maximum, sample, mode, delta, expected) in cases {
            let cells = hypergeometric_cells(mode, minimum, maximum, sample);
            assert!(
                !log_ratio_requires_shift(cells, delta),
                "large interior oracle should use the four-term fast path"
            );
            let candidate = u64::try_from(i64::try_from(mode).unwrap() + delta).unwrap();
            let actual = hypergeometric_log_ratio(mode, candidate, minimum, maximum, sample);
            assert!(actual.is_finite() && actual <= 1e-12);
            assert!(
                (actual - expected).abs() <= 5e-11,
                "boundary log-ratio {actual} differs from oracle {expected} for d={delta}"
            );
        }
    }

    #[test]
    fn shifted_log_ratio_preserves_two_modes_and_zero_mode_boundary() {
        // Hypergeometric(10, 5, 5) has adjacent modes 2 and 3.
        let tied = hypergeometric_log_ratio(3, 2, 5, 5, 5);
        assert!(
            tied.abs() <= 5e-14,
            "tied modes must have equal mass: {tied}"
        );

        let population = MAX_EXACT_FREQUENCY_TOTAL;
        let minimum = 1;
        let maximum = population - minimum;
        let sample = 100;
        let mode = (((u128::from(sample) + 1) * (u128::from(minimum) + 1))
            / (u128::from(population) + 2)) as u64;
        assert_eq!(mode, 0);
        assert!(log_ratio_requires_shift(
            hypergeometric_cells(mode, minimum, maximum, sample),
            1
        ));
        assert_eq!(
            hypergeometric_log_ratio(mode, mode, minimum, maximum, sample),
            0.0
        );
        let adjacent = hypergeometric_log_ratio(mode, 1, minimum, maximum, sample);
        assert!(adjacent.is_finite() && adjacent < 0.0);
    }

    #[test]
    fn hypergeometric_draw_scales_to_the_exact_frequency_boundary() {
        let population = MAX_EXACT_FREQUENCY_TOTAL;
        let successes = population / 2 + 12_345;
        let draws = population / 2 - 54_321;
        let computed_sample = draws.min(population - draws);
        let minimum_class = successes.min(population - successes);
        let full_proposal_support = hrua_proposal_upper_exclusive(computed_sample, minimum_class);
        let probability = minimum_class as f64 / population as f64;
        let variance = (population - computed_sample) as f64
            * computed_sample as f64
            * probability
            * (1.0 - probability)
            / (population - 1) as f64;
        let former_sixteen_sigma_cap =
            (computed_sample as f64 * probability + 0.5 + 16.0 * (variance + 0.5).sqrt()).floor()
                as u64;
        assert_eq!(
            full_proposal_support,
            computed_sample.min(minimum_class) + 1
        );
        assert!(full_proposal_support > former_sixteen_sigma_cap);
        let lower = draws.saturating_sub(population - successes);
        let upper = draws.min(successes);
        let started = Instant::now();
        let sample = |seed| {
            let mut rng = ChaCha20Rng::from_seed(seed);
            (0..512)
                .map(|_| hypergeometric_draw(population, successes, draws, &mut rng).unwrap())
                .collect::<Vec<_>>()
        };
        let first = sample([41; 32]);
        let second = sample([41; 32]);
        assert_eq!(first, second);
        assert!(first.iter().all(|value| *value >= lower && *value <= upper));
        assert!(
            started.elapsed().as_secs_f64() < 10.0,
            "boundary sampler must remain independent of expanded population size"
        );

        // Near-complete draws use the exact complement path and preserve the
        // same admitted boundary without allocating expanded rows.
        let mut rng = ChaCha20Rng::from_seed([43; 32]);
        let near_complete =
            hypergeometric_draw(population, successes, population - 7, &mut rng).unwrap();
        assert!((successes - 7..=successes).contains(&near_complete));
        assert!(matches!(
            hypergeometric_draw(population + 1, successes, draws, &mut rng),
            Err(FrequencyMultigroupErrorV1::Distribution(_))
        ));
    }

    #[test]
    fn boundary_draws_preserve_class_swap_and_sample_complement_symmetry() {
        let population = MAX_EXACT_FREQUENCY_TOTAL;
        let successes = population / 3 + 17;
        let draws = population / 2 - 31;
        let mut original_rng = ChaCha20Rng::from_seed([53; 32]);
        let mut class_swap_rng = ChaCha20Rng::from_seed([53; 32]);
        let mut sample_complement_rng = ChaCha20Rng::from_seed([53; 32]);
        for _ in 0..64 {
            let original =
                hypergeometric_draw(population, successes, draws, &mut original_rng).unwrap();
            let class_swap = hypergeometric_draw(
                population,
                population - successes,
                draws,
                &mut class_swap_rng,
            )
            .unwrap();
            let sample_complement = hypergeometric_draw(
                population,
                successes,
                population - draws,
                &mut sample_complement_rng,
            )
            .unwrap();
            assert_eq!(class_swap, draws - original);
            assert_eq!(sample_complement, successes - original);
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
