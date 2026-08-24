//! Additive multigroup MGA v1 statistical kernel.
//!
//! This module is deliberately independent of the legacy two-group MGA v4
//! implementation in `pls.rs`.  Upstream code owns row selection, model
//! compilation, score orientation, MICOM, and profile qualification.  The
//! kernel owns only typed group eligibility, deterministic resampling plans,
//! parameter-vector comparisons, failure accounting, and generic inference.
//! Model-specific estimation is supplied through [`MultigroupRefitterV1`].

use crate::PlsResult;
use rand::{Rng, SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{ChiSquared, ContinuousCDF, Normal, StudentsT};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const MGA_MULTIGROUP_KERNEL_METHOD_VERSION_V1: &str = "mga_multigroup_kernel_v1";
pub const MGA_MULTIGROUP_PAIRWISE_PERMUTATION_VERSION_V1: &str =
    "mga_multigroup_pairwise_permutation_v1";
pub const MGA_MULTIGROUP_PAIRWISE_PARTITION_PLAN_VERSION_V1: &str =
    "mga_multigroup_pairwise_partition_plan_v1";
pub const MGA_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1: &str =
    "mga_multigroup_max_spread_omnibus_v1";
pub const MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1: &str = "mga_multigroup_group_bootstrap_bank_v1";
pub const MGA_MULTIGROUP_HENSELER_PROBABILITY_VERSION_V1: &str =
    "henseler_directional_bootstrap_probability_v1";
pub const MGA_MULTIGROUP_BC_INTERVAL_VERSION_V1: &str =
    "efron_bias_corrected_zero_acceleration_type7_v1";
pub const MGA_MULTIGROUP_POOLED_TEST_VERSION_V1: &str =
    "equal_residual_variance_score_conditional_pooled_t_v1";
pub const MGA_MULTIGROUP_WELCH_TEST_VERSION_V1: &str =
    "welch_satterthwaite_parameter_difference_v1";
pub const MGA_MULTIGROUP_WALD_TEST_VERSION_V1: &str = "inverse_variance_k_group_wald_v1";
pub const MGA_ORDINARY_PLS_PATH_STANDARD_ERROR_VERSION_V1: &str =
    "ordinary_pls_score_conditional_centered_ols_path_se_v1";
pub const MGA_MULTIGROUP_MULTIPLICITY_VERSION_V1: &str = "mga_multigroup_probability_adjustment_v1";

pub const MGA_MULTIGROUP_MIN_GROUPS_V1: usize = 2;
pub const MGA_MULTIGROUP_MAX_GROUPS_V1: usize = 20;
pub const MGA_MULTIGROUP_MIN_COMPLETE_CASES_V1: usize = 10;
pub const MGA_MULTIGROUP_SMALL_GROUP_WARNING_V1: usize = 30;
pub const MGA_MULTIGROUP_IMBALANCE_WARNING_RATIO_V1: f64 = 2.0;
pub const MGA_MULTIGROUP_IMBALANCE_BLOCK_RATIO_V1: f64 = 10.0;
pub const MGA_MULTIGROUP_MIN_RESAMPLES_V1: usize = 5_000;
pub const MGA_MULTIGROUP_MAX_RESAMPLES_V1: usize = 10_000;

const STREAM_DOMAIN: &[u8] = b"quickpls/mga_multigroup_v1";
const PAIRWISE_STREAM: &[u8] = b"pairwise_fixed_size_permutation";
const OMNIBUS_STREAM: &[u8] = b"global_fixed_size_permutation";
const BOOTSTRAP_STREAM: &[u8] = b"group_case_bootstrap";

/// A checked zero-based group position.  Serialized values remain numeric and
/// cannot exceed the v1 maximum even when constructed by deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct GroupIndexV1(u8);

impl GroupIndexV1 {
    pub fn new(index: usize) -> Result<Self, MultigroupKernelErrorV1> {
        let value = u8::try_from(index)
            .map_err(|_| MultigroupKernelErrorV1::InvalidGroupIndex { index })?;
        Self::try_from(value)
    }

    pub const fn get(self) -> usize {
        self.0 as usize
    }
}

impl TryFrom<u8> for GroupIndexV1 {
    type Error = MultigroupKernelErrorV1;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if usize::from(value) < MGA_MULTIGROUP_MAX_GROUPS_V1 {
            Ok(Self(value))
        } else {
            Err(MultigroupKernelErrorV1::InvalidGroupIndex {
                index: usize::from(value),
            })
        }
    }
}

impl From<GroupIndexV1> for u8 {
    fn from(value: GroupIndexV1) -> Self {
        value.0
    }
}

/// Preserves the source type of a selected group value.  In particular,
/// integer `1`, numeric `1.0`, text `"1"`, and boolean `true` remain distinct.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypedGroupValueV1 {
    Text {
        value: String,
    },
    Boolean {
        value: bool,
    },
    Integer {
        value: i64,
    },
    /// Exact IEEE-754 representation of a finite numeric source value.
    Number {
        ieee754_bits: u64,
    },
}

impl TypedGroupValueV1 {
    pub fn finite_number(value: f64) -> Result<Self, MultigroupKernelErrorV1> {
        if !value.is_finite() {
            return Err(MultigroupKernelErrorV1::NonFiniteGroupValue);
        }
        Ok(Self::Number {
            ieee754_bits: value.to_bits(),
        })
    }

    pub fn validate(&self) -> Result<(), MultigroupKernelErrorV1> {
        match self {
            Self::Text { value } if value.is_empty() => {
                Err(MultigroupKernelErrorV1::EmptyGroupValue)
            }
            Self::Number { ieee754_bits } if !f64::from_bits(*ieee754_bits).is_finite() => {
                Err(MultigroupKernelErrorV1::NonFiniteGroupValue)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupIdentityV1 {
    pub index: GroupIndexV1,
    pub value: TypedGroupValueV1,
    pub display_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedGroupRowV1 {
    /// Stable identity supplied by upstream complete-case selection.
    pub source_row: u64,
    pub group: GroupIndexV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultigroupDesignV1 {
    pub groups: Vec<GroupIdentityV1>,
    pub rows: Vec<SelectedGroupRowV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityBlockerCodeV1 {
    GroupCountOutsideTwoToTwenty,
    NoncontiguousGroupIndex,
    DuplicateTypedGroupValue,
    EmptyGroupLabel,
    InvalidTypedGroupValue,
    DuplicateSourceRow,
    UnknownRowGroup,
    InsufficientCompleteCases,
    ExtremeGroupImbalance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EligibilityBlockerV1 {
    pub code: EligibilityBlockerCodeV1,
    pub group: Option<GroupIndexV1>,
    pub observed: Option<f64>,
    pub required: Option<f64>,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityWarningCodeV1 {
    SmallGroup,
    GroupImbalanceAboveTwoToOne,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EligibilityWarningV1 {
    pub code: EligibilityWarningCodeV1,
    pub group: Option<GroupIndexV1>,
    pub observed: f64,
    pub threshold: f64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupEligibilitySummaryV1 {
    pub group: GroupIndexV1,
    pub complete_cases: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultigroupEligibilityV1 {
    pub eligible: bool,
    pub group_counts: Vec<GroupEligibilitySummaryV1>,
    pub maximum_imbalance_ratio: Option<f64>,
    pub blockers: Vec<EligibilityBlockerV1>,
    pub warnings: Vec<EligibilityWarningV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedGroupPairV1 {
    pub group_a: GroupIndexV1,
    pub group_b: GroupIndexV1,
}

impl OrderedGroupPairV1 {
    pub fn new(
        group_a: GroupIndexV1,
        group_b: GroupIndexV1,
    ) -> Result<Self, MultigroupKernelErrorV1> {
        if group_a == group_b {
            return Err(MultigroupKernelErrorV1::SameGroupComparison { group: group_a });
        }
        Ok(Self { group_a, group_b })
    }

    fn canonical(self) -> (GroupIndexV1, GroupIndexV1) {
        if self.group_a < self.group_b {
            (self.group_a, self.group_b)
        } else {
            (self.group_b, self.group_a)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterFamilyV1 {
    StructuralPath,
    OuterLoading,
    OuterWeight,
    RSquared,
    SpecificIndirect,
    TotalIndirect,
    InteractionGamma,
    ThreeWayDelta,
    SimpleSlope,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterIdentityV1 {
    pub stable_id: String,
    pub family: ParameterFamilyV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterEstimateV1 {
    pub parameter: ParameterIdentityV1,
    pub estimate: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterVectorV1 {
    pub parameters: Vec<ParameterEstimateV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitSampleKindV1 {
    ObservedGroup,
    PairwisePermutation,
    OmnibusPermutation,
    GroupBootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultigroupFitRequestV1 {
    pub sample_kind: FitSampleKindV1,
    pub group: GroupIndexV1,
    pub replicate: Option<usize>,
    /// May contain repeated source rows for a case-bootstrap fit.
    pub source_rows: Vec<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefitFailureCodeV1 {
    Cancelled,
    UnsupportedProfile,
    InsufficientRows,
    SingularModel,
    Nonconvergence,
    NonFiniteEstimate,
    OrientationUndefined,
    ParameterContractMismatch,
    EngineFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefitFailureV1 {
    pub code: RefitFailureCodeV1,
    pub detail: String,
}

impl RefitFailureV1 {
    pub fn new(code: RefitFailureCodeV1, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

/// Model-specific adapters implement this trait.  They must refit the complete
/// qualified model for exactly the rows supplied and return every expected
/// parameter under its stable identity.  The kernel never retries a failure.
pub trait MultigroupRefitterV1 {
    fn fit(
        &mut self,
        request: &MultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1>;
}

impl<F> MultigroupRefitterV1 for F
where
    F: FnMut(&MultigroupFitRequestV1) -> Result<ParameterVectorV1, RefitFailureV1>,
{
    fn fit(
        &mut self,
        request: &MultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        self(request)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativeHypothesisV1 {
    TwoSided,
    Greater,
    Less,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiplicityMethodV1 {
    Holm,
    Bonferroni,
    Sidak,
    BenjaminiHochberg,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultigroupResamplingConfigV1 {
    pub requested: usize,
    pub seed: u64,
    pub confidence_level: f64,
    pub alpha: f64,
    pub alternative: AlternativeHypothesisV1,
}

impl MultigroupResamplingConfigV1 {
    pub const fn official_defaults() -> Self {
        Self {
            requested: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: AlternativeHypothesisV1::TwoSided,
        }
    }

    pub fn validate(self) -> Result<(), MultigroupKernelErrorV1> {
        if !(MGA_MULTIGROUP_MIN_RESAMPLES_V1..=MGA_MULTIGROUP_MAX_RESAMPLES_V1)
            .contains(&self.requested)
        {
            return Err(MultigroupKernelErrorV1::InvalidResampleCount {
                requested: self.requested,
            });
        }
        if !self.confidence_level.is_finite()
            || self.confidence_level <= 0.0
            || self.confidence_level >= 1.0
        {
            return Err(MultigroupKernelErrorV1::InvalidConfidenceLevel {
                value: self.confidence_level,
            });
        }
        if !self.alpha.is_finite() || self.alpha <= 0.0 || self.alpha >= 0.5 {
            return Err(MultigroupKernelErrorV1::InvalidAlpha { value: self.alpha });
        }
        Ok(())
    }

    pub fn minimum_usable(self) -> usize {
        minimum_usable_resamples_v1(self.requested)
    }
}

pub const fn minimum_usable_resamples_v1(requested: usize) -> usize {
    let ninety_percent = requested.saturating_mul(9).saturating_add(9) / 10;
    if ninety_percent > 1_000 {
        ninety_percent
    } else {
        1_000
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum MultigroupKernelErrorV1 {
    #[error("group index {index} is outside the MGA multigroup v1 range 0..20")]
    InvalidGroupIndex { index: usize },
    #[error("numeric group values must be finite")]
    NonFiniteGroupValue,
    #[error("text group values must not be empty")]
    EmptyGroupValue,
    #[error("group {group:?} cannot be compared with itself")]
    SameGroupComparison { group: GroupIndexV1 },
    #[error("multigroup design is ineligible: {0:?}")]
    IneligibleDesign(Vec<EligibilityBlockerV1>),
    #[error("ordered comparison references a group absent from the design")]
    UnknownComparisonGroup,
    #[error("MGA multigroup v1 requires 5000 to 10000 resamples; found {requested}")]
    InvalidResampleCount { requested: usize },
    #[error("pairwise partition plan is incompatible with the requested operation: {0}")]
    InvalidPairwisePartitionPlan(String),
    #[error("confidence level must be finite and strictly between zero and one; found {value}")]
    InvalidConfidenceLevel { value: f64 },
    #[error("alpha must be finite and strictly between zero and 0.5; found {value}")]
    InvalidAlpha { value: f64 },
    #[error("parameter identities must be nonempty and unique")]
    InvalidParameterIdentities,
    #[error("observed fit failed for group {group:?}: {failure:?}")]
    ObservedFitFailed {
        group: GroupIndexV1,
        failure: RefitFailureV1,
    },
    #[error("at least three groups are required for max-spread omnibus inference")]
    OmnibusRequiresThreeGroups,
    #[error("group bootstrap bank is incompatible with the requested operation: {0}")]
    InvalidBootstrapBank(String),
    #[error("insufficient usable resamples: required {required}, found {found}")]
    InsufficientUsableResamples { required: usize, found: usize },
    #[error("probability must be finite and in [0,1]")]
    InvalidProbability,
    #[error("probability hypothesis identities must be nonempty and unique")]
    InvalidHypothesisIdentities,
    #[error("parametric test input is inadmissible: {0}")]
    InvalidParametricInput(String),
    #[error("distribution calculation failed: {0}")]
    Distribution(String),
}

/// Applies all structural eligibility rules after upstream complete-case row
/// selection.  It never mutates or silently drops a row.
pub fn assess_multigroup_design_v1(design: &MultigroupDesignV1) -> MultigroupEligibilityV1 {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if !(MGA_MULTIGROUP_MIN_GROUPS_V1..=MGA_MULTIGROUP_MAX_GROUPS_V1).contains(&design.groups.len())
    {
        blockers.push(EligibilityBlockerV1 {
            code: EligibilityBlockerCodeV1::GroupCountOutsideTwoToTwenty,
            group: None,
            observed: Some(design.groups.len() as f64),
            required: Some(MGA_MULTIGROUP_MIN_GROUPS_V1 as f64),
            detail: "select between two and twenty groups".into(),
        });
    }

    let mut typed_values = BTreeSet::new();
    for (position, group) in design.groups.iter().enumerate() {
        if group.index.get() != position {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::NoncontiguousGroupIndex,
                group: Some(group.index),
                observed: Some(group.index.get() as f64),
                required: Some(position as f64),
                detail: "group indices must be contiguous and match group order".into(),
            });
        }
        if !typed_values.insert(group.value.clone()) {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::DuplicateTypedGroupValue,
                group: Some(group.index),
                observed: None,
                required: None,
                detail: "each selected typed group value must be unique".into(),
            });
        }
        if group.display_label.trim().is_empty() {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::EmptyGroupLabel,
                group: Some(group.index),
                observed: None,
                required: None,
                detail: "group display labels must not be empty".into(),
            });
        }
        if group.value.validate().is_err() {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::InvalidTypedGroupValue,
                group: Some(group.index),
                observed: None,
                required: None,
                detail: "typed group values must be finite and nonempty".into(),
            });
        }
    }

    let mut seen_rows = BTreeSet::new();
    let mut counts = vec![0usize; design.groups.len()];
    for row in &design.rows {
        if !seen_rows.insert(row.source_row) {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::DuplicateSourceRow,
                group: Some(row.group),
                observed: Some(row.source_row as f64),
                required: None,
                detail: "each upstream source-row token must occur exactly once".into(),
            });
        }
        if let Some(count) = counts.get_mut(row.group.get()) {
            *count += 1;
        } else {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::UnknownRowGroup,
                group: Some(row.group),
                observed: Some(row.group.get() as f64),
                required: None,
                detail: "row references a group absent from the selected design".into(),
            });
        }
    }

    let group_counts = design
        .groups
        .iter()
        .enumerate()
        .map(|(position, group)| GroupEligibilitySummaryV1 {
            group: group.index,
            complete_cases: counts.get(position).copied().unwrap_or(0),
        })
        .collect::<Vec<_>>();
    for summary in &group_counts {
        if summary.complete_cases < MGA_MULTIGROUP_MIN_COMPLETE_CASES_V1 {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::InsufficientCompleteCases,
                group: Some(summary.group),
                observed: Some(summary.complete_cases as f64),
                required: Some(MGA_MULTIGROUP_MIN_COMPLETE_CASES_V1 as f64),
                detail: "every selected group requires at least ten complete model cases".into(),
            });
        } else if summary.complete_cases < MGA_MULTIGROUP_SMALL_GROUP_WARNING_V1 {
            warnings.push(EligibilityWarningV1 {
                code: EligibilityWarningCodeV1::SmallGroup,
                group: Some(summary.group),
                observed: summary.complete_cases as f64,
                threshold: MGA_MULTIGROUP_SMALL_GROUP_WARNING_V1 as f64,
                detail: "group has fewer than thirty complete model cases".into(),
            });
        }
    }

    let nonzero = counts
        .iter()
        .copied()
        .filter(|count| *count > 0)
        .collect::<Vec<_>>();
    let maximum_imbalance_ratio = if nonzero.len() >= 2 {
        let minimum = *nonzero.iter().min().expect("nonempty count collection") as f64;
        let maximum = *nonzero.iter().max().expect("nonempty count collection") as f64;
        Some(maximum / minimum)
    } else {
        None
    };
    if let Some(ratio) = maximum_imbalance_ratio {
        if ratio > MGA_MULTIGROUP_IMBALANCE_BLOCK_RATIO_V1 {
            blockers.push(EligibilityBlockerV1 {
                code: EligibilityBlockerCodeV1::ExtremeGroupImbalance,
                group: None,
                observed: Some(ratio),
                required: Some(MGA_MULTIGROUP_IMBALANCE_BLOCK_RATIO_V1),
                detail: "largest-to-smallest group ratio exceeds ten to one".into(),
            });
        } else if ratio > MGA_MULTIGROUP_IMBALANCE_WARNING_RATIO_V1 {
            warnings.push(EligibilityWarningV1 {
                code: EligibilityWarningCodeV1::GroupImbalanceAboveTwoToOne,
                group: None,
                observed: ratio,
                threshold: MGA_MULTIGROUP_IMBALANCE_WARNING_RATIO_V1,
                detail: "largest-to-smallest group ratio exceeds two to one".into(),
            });
        }
    }

    MultigroupEligibilityV1 {
        eligible: blockers.is_empty(),
        group_counts,
        maximum_imbalance_ratio,
        blockers,
        warnings,
    }
}

fn eligible_design(
    design: &MultigroupDesignV1,
) -> Result<MultigroupEligibilityV1, MultigroupKernelErrorV1> {
    let eligibility = assess_multigroup_design_v1(design);
    if eligibility.eligible {
        Ok(eligibility)
    } else {
        Err(MultigroupKernelErrorV1::IneligibleDesign(
            eligibility.blockers,
        ))
    }
}

fn validate_parameter_identities(
    parameters: &[ParameterIdentityV1],
) -> Result<(), MultigroupKernelErrorV1> {
    let mut ids = BTreeSet::new();
    if parameters.is_empty()
        || parameters
            .iter()
            .any(|parameter| parameter.stable_id.trim().is_empty())
        || parameters
            .iter()
            .any(|parameter| !ids.insert(parameter.stable_id.clone()))
    {
        return Err(MultigroupKernelErrorV1::InvalidParameterIdentities);
    }
    Ok(())
}

fn canonical_rows(design: &MultigroupDesignV1) -> Vec<SelectedGroupRowV1> {
    let mut rows = design.rows.clone();
    rows.sort_by_key(|row| row.source_row);
    rows
}

fn rows_for_group(rows: &[SelectedGroupRowV1], group: GroupIndexV1) -> Vec<u64> {
    rows.iter()
        .filter_map(|row| (row.group == group).then_some(row.source_row))
        .collect()
}

fn checked_vector(
    expected: &[ParameterIdentityV1],
    vector: ParameterVectorV1,
) -> Result<Vec<f64>, RefitFailureV1> {
    let mut values = BTreeMap::new();
    for parameter in vector.parameters {
        if parameter.parameter.stable_id.trim().is_empty()
            || !parameter.estimate.is_finite()
            || values
                .insert(parameter.parameter.stable_id.clone(), parameter)
                .is_some()
        {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "callback returned duplicate, empty, or nonfinite parameter output",
            ));
        }
    }
    if values.len() != expected.len() {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::ParameterContractMismatch,
            "callback parameter count differs from the frozen parameter vector",
        ));
    }
    expected
        .iter()
        .map(|identity| {
            let value = values.remove(&identity.stable_id).ok_or_else(|| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    format!("callback omitted parameter {}", identity.stable_id),
                )
            })?;
            if value.parameter != *identity {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    format!(
                        "callback changed the family of parameter {}",
                        identity.stable_id
                    ),
                ));
            }
            Ok(value.estimate)
        })
        .collect()
}

fn fit_checked<R: MultigroupRefitterV1>(
    refitter: &mut R,
    expected: &[ParameterIdentityV1],
    request: MultigroupFitRequestV1,
) -> Result<Vec<f64>, RefitFailureV1> {
    checked_vector(expected, refitter.fit(&request)?)
}

fn derive_rng_seed(seed: u64, operation: &[u8], replicate: usize, extra: &[u8]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(STREAM_DOMAIN);
    digest.update((operation.len() as u64).to_le_bytes());
    digest.update(operation);
    digest.update(seed.to_le_bytes());
    digest.update((replicate as u64).to_le_bytes());
    digest.update((extra.len() as u64).to_le_bytes());
    digest.update(extra);
    digest.finalize().into()
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    format!("sha256:{hex}")
}

fn partition_digest(assignments: &[(u64, GroupIndexV1)]) -> String {
    let mut bytes = Vec::with_capacity(assignments.len() * 9);
    for (row, group) in assignments {
        bytes.extend_from_slice(&row.to_le_bytes());
        bytes.push(u8::from(*group));
    }
    sha256_prefixed(&bytes)
}

fn sample_digest(group: GroupIndexV1, rows: &[u64]) -> String {
    let mut bytes = Vec::with_capacity(1 + rows.len() * 8);
    bytes.push(u8::from(group));
    for row in rows {
        bytes.extend_from_slice(&row.to_le_bytes());
    }
    sha256_prefixed(&bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartitionV1 {
    assignments: Vec<(u64, GroupIndexV1)>,
    digest: String,
}

/// One immutable entry in the pairwise fixed-size partition plan.  The
/// assignment itself is regenerated from the frozen master seed and checked
/// against this digest before either MICOM or MGA may consume it.  Keeping the
/// ledger hash-only avoids retaining O(permutations x rows) assignments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairwisePartitionPlanEntryV1 {
    pub replicate: usize,
    pub partition_sha256: String,
}

/// Shared deterministic authority for pairwise MICOM and permutation MGA.
/// `pair` is always stored in canonical group-index order, so reversing the
/// requested A/B contrast reuses the same partitions and plan identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairwisePartitionPlanV1 {
    pub method_version: String,
    pub pair: OrderedGroupPairV1,
    pub seed: u64,
    pub requested: usize,
    pub group_low_count: usize,
    pub group_high_count: usize,
    pub observed_membership_sha256: String,
    pub plan_sha256: String,
    pub entries: Vec<PairwisePartitionPlanEntryV1>,
}

/// A checked materialization of one shared-plan entry.  Assignments use the
/// canonical scientific group identities, not transient A/B positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairwisePartitionMaterializationV1 {
    pub replicate: usize,
    pub partition_sha256: String,
    pub assignments: Vec<SelectedGroupRowV1>,
}

fn canonical_pair(pair: OrderedGroupPairV1) -> OrderedGroupPairV1 {
    let (group_a, group_b) = pair.canonical();
    OrderedGroupPairV1 { group_a, group_b }
}

fn canonical_pair_rows(
    rows: &[SelectedGroupRowV1],
    pair: OrderedGroupPairV1,
) -> Result<Vec<SelectedGroupRowV1>, MultigroupKernelErrorV1> {
    let pair = canonical_pair(pair);
    let mut selected = rows
        .iter()
        .copied()
        .filter(|row| row.group == pair.group_a || row.group == pair.group_b)
        .collect::<Vec<_>>();
    selected.sort_by_key(|row| row.source_row);
    let mut identities = BTreeSet::new();
    if selected
        .iter()
        .any(|row| !identities.insert(row.source_row))
    {
        return Err(MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(
            "the pairwise source-row universe contains a duplicate stable row token".into(),
        ));
    }
    let low_count = selected
        .iter()
        .filter(|row| row.group == pair.group_a)
        .count();
    let high_count = selected.len().saturating_sub(low_count);
    if low_count < MGA_MULTIGROUP_MIN_COMPLETE_CASES_V1
        || high_count < MGA_MULTIGROUP_MIN_COMPLETE_CASES_V1
    {
        return Err(MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(
            "the pairwise partition universe requires ten rows in each group".into(),
        ));
    }
    Ok(selected)
}

fn observed_membership_digest(rows: &[SelectedGroupRowV1]) -> String {
    partition_digest(
        &rows
            .iter()
            .map(|row| (row.source_row, row.group))
            .collect::<Vec<_>>(),
    )
}

fn pairwise_plan_digest(
    pair: OrderedGroupPairV1,
    seed: u64,
    requested: usize,
    membership_sha256: &str,
    entries: &[PairwisePartitionPlanEntryV1],
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(MGA_MULTIGROUP_PAIRWISE_PARTITION_PLAN_VERSION_V1.as_bytes());
    bytes.push(u8::from(pair.group_a));
    bytes.push(u8::from(pair.group_b));
    bytes.extend_from_slice(&seed.to_le_bytes());
    bytes.extend_from_slice(&(requested as u64).to_le_bytes());
    bytes.extend_from_slice(membership_sha256.as_bytes());
    for entry in entries {
        bytes.extend_from_slice(&(entry.replicate as u64).to_le_bytes());
        bytes.extend_from_slice(entry.partition_sha256.as_bytes());
    }
    sha256_prefixed(&bytes)
}

fn pairwise_partition(
    rows: &[SelectedGroupRowV1],
    pair: OrderedGroupPairV1,
    seed: u64,
    replicate: usize,
) -> PartitionV1 {
    let (low, high) = pair.canonical();
    let universe = rows
        .iter()
        .filter_map(|row| (row.group == low || row.group == high).then_some(row.source_row))
        .collect::<Vec<_>>();
    let low_count = rows.iter().filter(|row| row.group == low).count();
    let mut labels = Vec::with_capacity(universe.len());
    labels.extend(std::iter::repeat_n(low, low_count));
    labels.extend(std::iter::repeat_n(high, universe.len() - low_count));
    let mut extra = [0u8; 2];
    extra[0] = u8::from(low);
    extra[1] = u8::from(high);
    let mut rng = ChaCha20Rng::from_seed(derive_rng_seed(seed, PAIRWISE_STREAM, replicate, &extra));
    labels.shuffle(&mut rng);
    let assignments = universe.into_iter().zip(labels).collect::<Vec<_>>();
    let digest = partition_digest(&assignments);
    PartitionV1 {
        assignments,
        digest,
    }
}

/// Freezes the shared pairwise partition ledger from stable row tokens.  This
/// lower-level constructor is used by MICOM, whose pairwise API intentionally
/// does not require identities for unrelated groups.
pub fn build_pairwise_partition_plan_from_rows_v1(
    rows: &[SelectedGroupRowV1],
    pair: OrderedGroupPairV1,
    requested: usize,
    seed: u64,
) -> Result<PairwisePartitionPlanV1, MultigroupKernelErrorV1> {
    if !(MGA_MULTIGROUP_MIN_RESAMPLES_V1..=MGA_MULTIGROUP_MAX_RESAMPLES_V1).contains(&requested) {
        return Err(MultigroupKernelErrorV1::InvalidResampleCount { requested });
    }
    let pair = canonical_pair(pair);
    let rows = canonical_pair_rows(rows, pair)?;
    let group_low_count = rows.iter().filter(|row| row.group == pair.group_a).count();
    let group_high_count = rows.len() - group_low_count;
    let observed_membership_sha256 = observed_membership_digest(&rows);
    let entries = (0..requested)
        .map(|replicate| {
            let partition = pairwise_partition(&rows, pair, seed, replicate);
            PairwisePartitionPlanEntryV1 {
                replicate,
                partition_sha256: partition.digest,
            }
        })
        .collect::<Vec<_>>();
    let plan_sha256 =
        pairwise_plan_digest(pair, seed, requested, &observed_membership_sha256, &entries);
    Ok(PairwisePartitionPlanV1 {
        method_version: MGA_MULTIGROUP_PAIRWISE_PARTITION_PLAN_VERSION_V1.into(),
        pair,
        seed,
        requested,
        group_low_count,
        group_high_count,
        observed_membership_sha256,
        plan_sha256,
        entries,
    })
}

/// Builds a pairwise plan after applying the full multigroup design gates.
pub fn build_pairwise_partition_plan_v1(
    design: &MultigroupDesignV1,
    pair: OrderedGroupPairV1,
    requested: usize,
    seed: u64,
) -> Result<PairwisePartitionPlanV1, MultigroupKernelErrorV1> {
    eligible_design(design)?;
    validate_pair(design, pair)?;
    build_pairwise_partition_plan_from_rows_v1(&canonical_rows(design), pair, requested, seed)
}

/// Validates plan identity and its complete hash ledger against the current
/// row universe.  Consumers call this once before any scientific refit.
pub fn validate_pairwise_partition_plan_for_rows_v1(
    rows: &[SelectedGroupRowV1],
    pair: OrderedGroupPairV1,
    requested: usize,
    seed: u64,
    plan: &PairwisePartitionPlanV1,
) -> Result<(), MultigroupKernelErrorV1> {
    let pair = canonical_pair(pair);
    let rows = canonical_pair_rows(rows, pair)?;
    let low_count = rows.iter().filter(|row| row.group == pair.group_a).count();
    let high_count = rows.len() - low_count;
    let membership_sha256 = observed_membership_digest(&rows);
    if plan.method_version != MGA_MULTIGROUP_PAIRWISE_PARTITION_PLAN_VERSION_V1
        || plan.pair != pair
        || plan.seed != seed
        || plan.requested != requested
        || plan.group_low_count != low_count
        || plan.group_high_count != high_count
        || plan.observed_membership_sha256 != membership_sha256
        || plan.entries.len() != requested
        || plan.entries.iter().enumerate().any(|(replicate, entry)| {
            entry.replicate != replicate || !entry.partition_sha256.starts_with("sha256:")
        })
        || plan.plan_sha256
            != pairwise_plan_digest(pair, seed, requested, &membership_sha256, &plan.entries)
    {
        return Err(MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(
            "method, pair, seed, row universe, replicate identities, or plan digest differs".into(),
        ));
    }
    Ok(())
}

/// Regenerates and verifies exactly one entry of an already header-validated
/// plan.  There is no replacement draw if a downstream refit fails.
pub fn materialize_pairwise_partition_v1(
    rows: &[SelectedGroupRowV1],
    pair: OrderedGroupPairV1,
    plan: &PairwisePartitionPlanV1,
    replicate: usize,
) -> Result<PairwisePartitionMaterializationV1, MultigroupKernelErrorV1> {
    let pair = canonical_pair(pair);
    if plan.pair != pair {
        return Err(MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(
            "materialization requested a different pair".into(),
        ));
    }
    let entry = plan.entries.get(replicate).ok_or_else(|| {
        MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(format!(
            "replicate {replicate} is absent from the frozen plan"
        ))
    })?;
    if entry.replicate != replicate {
        return Err(MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(
            "replicate identity differs from its frozen ledger position".into(),
        ));
    }
    let rows = canonical_pair_rows(rows, pair)?;
    let partition = pairwise_partition(&rows, pair, plan.seed, replicate);
    if partition.digest != entry.partition_sha256 {
        return Err(MultigroupKernelErrorV1::InvalidPairwisePartitionPlan(
            format!("replicate {replicate} partition digest does not reproduce"),
        ));
    }
    Ok(PairwisePartitionMaterializationV1 {
        replicate,
        partition_sha256: partition.digest,
        assignments: partition
            .assignments
            .into_iter()
            .map(|(source_row, group)| SelectedGroupRowV1 { source_row, group })
            .collect(),
    })
}

fn omnibus_partition(
    rows: &[SelectedGroupRowV1],
    group_count: usize,
    seed: u64,
    replicate: usize,
) -> PartitionV1 {
    let universe = rows.iter().map(|row| row.source_row).collect::<Vec<_>>();
    let mut labels = Vec::with_capacity(rows.len());
    for group_position in 0..group_count {
        let group = GroupIndexV1::new(group_position).expect("validated group position");
        labels.extend(std::iter::repeat_n(
            group,
            rows.iter().filter(|row| row.group == group).count(),
        ));
    }
    let mut rng = ChaCha20Rng::from_seed(derive_rng_seed(
        seed,
        OMNIBUS_STREAM,
        replicate,
        &(group_count as u64).to_le_bytes(),
    ));
    labels.shuffle(&mut rng);
    let assignments = universe.into_iter().zip(labels).collect::<Vec<_>>();
    let digest = partition_digest(&assignments);
    PartitionV1 {
        assignments,
        digest,
    }
}

fn bootstrap_rows(
    source_rows: &[u64],
    group: GroupIndexV1,
    seed: u64,
    replicate: usize,
) -> Vec<u64> {
    let mut rng = ChaCha20Rng::from_seed(derive_rng_seed(
        seed,
        BOOTSTRAP_STREAM,
        replicate,
        &[u8::from(group)],
    ));
    (0..source_rows.len())
        .map(|_| source_rows[rng.random_range(0..source_rows.len())])
        .collect()
}

fn rows_from_assignments(assignments: &[(u64, GroupIndexV1)], group: GroupIndexV1) -> Vec<u64> {
    assignments
        .iter()
        .filter_map(|(row, assigned)| (*assigned == group).then_some(*row))
        .collect()
}

fn finish_plan_digest(method: &str, entries: impl IntoIterator<Item = (usize, String)>) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(method.as_bytes());
    for (replicate, digest) in entries {
        bytes.extend_from_slice(&(replicate as u64).to_le_bytes());
        bytes.extend_from_slice(digest.as_bytes());
    }
    sha256_prefixed(&bytes)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResampleFitStatusV1 {
    Usable,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupFitLedgerV1 {
    pub group: GroupIndexV1,
    pub status: ResampleFitStatusV1,
    pub failure: Option<RefitFailureV1>,
}

impl GroupFitLedgerV1 {
    fn usable(group: GroupIndexV1) -> Self {
        Self {
            group,
            status: ResampleFitStatusV1::Usable,
            failure: None,
        }
    }

    fn failed(group: GroupIndexV1, failure: RefitFailureV1) -> Self {
        Self {
            group,
            status: ResampleFitStatusV1::Failed,
            failure: Some(failure),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermutationLedgerEntryV1 {
    pub replicate: usize,
    pub partition_sha256: String,
    pub status: ResampleFitStatusV1,
    pub group_fits: Vec<GroupFitLedgerV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceAvailabilityV1 {
    Available,
    InsufficientUsableResamples,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairwisePermutationParameterV1 {
    pub parameter: ParameterIdentityV1,
    pub estimate_a: f64,
    pub estimate_b: f64,
    pub difference_a_minus_b: f64,
    pub p_value_two_sided: f64,
    pub p_value_greater: f64,
    pub p_value_less: f64,
    pub selected_alternative: AlternativeHypothesisV1,
    pub selected_probability: f64,
    /// Usable permutation A-minus-B differences in usable-ledger order for
    /// the deterministic first parameter in the frozen inventory. Other
    /// parameters retain an empty vector so the audit stays bounded.
    pub null_differences: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairwisePointParameterV1 {
    pub parameter: ParameterIdentityV1,
    pub estimate_a: f64,
    pub estimate_b: f64,
    pub difference_a_minus_b: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairwisePermutationResultV1 {
    pub method_version: String,
    pub pair: OrderedGroupPairV1,
    pub seed: u64,
    pub requested: usize,
    pub attempted: usize,
    pub usable: usize,
    pub failed: usize,
    pub minimum_usable: usize,
    pub retry_policy: String,
    pub plan_sha256: String,
    pub availability: InferenceAvailabilityV1,
    /// Always retained after successful observed fits, even when the usable
    /// permutation threshold blocks inferential probabilities.
    pub point_estimates: Vec<PairwisePointParameterV1>,
    pub parameters: Vec<PairwisePermutationParameterV1>,
    pub ledger: Vec<PermutationLedgerEntryV1>,
    pub group_counts: Vec<GroupEligibilitySummaryV1>,
    pub eligibility_warnings: Vec<EligibilityWarningV1>,
}

fn fit_observed_group<R: MultigroupRefitterV1>(
    rows: &[SelectedGroupRowV1],
    group: GroupIndexV1,
    parameters: &[ParameterIdentityV1],
    refitter: &mut R,
) -> Result<Vec<f64>, MultigroupKernelErrorV1> {
    fit_checked(
        refitter,
        parameters,
        MultigroupFitRequestV1 {
            sample_kind: FitSampleKindV1::ObservedGroup,
            group,
            replicate: None,
            source_rows: rows_for_group(rows, group),
        },
    )
    .map_err(|failure| MultigroupKernelErrorV1::ObservedFitFailed { group, failure })
}

fn validate_pair(
    design: &MultigroupDesignV1,
    pair: OrderedGroupPairV1,
) -> Result<(), MultigroupKernelErrorV1> {
    if pair.group_a == pair.group_b {
        return Err(MultigroupKernelErrorV1::SameGroupComparison {
            group: pair.group_a,
        });
    }
    if pair.group_a.get() >= design.groups.len() || pair.group_b.get() >= design.groups.len() {
        return Err(MultigroupKernelErrorV1::UnknownComparisonGroup);
    }
    Ok(())
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

/// Executes an ordered A-minus-B fixed-size label-permutation comparison.
/// The partition stream is canonical for the unordered pair, so reversing A
/// and B preserves partitions and two-sided evidence while reversing signs.
pub fn run_pairwise_permutation_v1<R: MultigroupRefitterV1>(
    design: &MultigroupDesignV1,
    pair: OrderedGroupPairV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    refitter: &mut R,
) -> Result<PairwisePermutationResultV1, MultigroupKernelErrorV1> {
    let plan = build_pairwise_partition_plan_v1(design, pair, config.requested, config.seed)?;
    run_pairwise_permutation_with_plan_v1(design, pair, parameters, config, &plan, refitter)
}

/// Executes permutation MGA against a previously frozen pairwise plan.  This
/// is the integration seam used to make MICOM and MGA consume identical
/// group-size-preserving partitions without sharing their method-specific fit
/// status ledgers.
pub fn run_pairwise_permutation_with_plan_v1<R: MultigroupRefitterV1>(
    design: &MultigroupDesignV1,
    pair: OrderedGroupPairV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    plan: &PairwisePartitionPlanV1,
    refitter: &mut R,
) -> Result<PairwisePermutationResultV1, MultigroupKernelErrorV1> {
    let eligibility = eligible_design(design)?;
    validate_pair(design, pair)?;
    validate_parameter_identities(parameters)?;
    config.validate()?;
    let rows = canonical_rows(design);
    validate_pairwise_partition_plan_for_rows_v1(&rows, pair, config.requested, config.seed, plan)?;
    let point_a = fit_observed_group(&rows, pair.group_a, parameters, refitter)?;
    let point_b = fit_observed_group(&rows, pair.group_b, parameters, refitter)?;
    let observed_differences = point_a
        .iter()
        .zip(&point_b)
        .map(|(left, right)| left - right)
        .collect::<Vec<_>>();
    let mut absolute_extremes = vec![0usize; parameters.len()];
    let mut greater_extremes = vec![0usize; parameters.len()];
    let mut less_extremes = vec![0usize; parameters.len()];
    let mut audit_null_differences = Vec::with_capacity(config.requested);
    let mut ledger = Vec::with_capacity(config.requested);
    let mut usable = 0usize;

    for replicate in 0..config.requested {
        let partition = materialize_pairwise_partition_v1(&rows, pair, plan, replicate)?;
        let mut group_fits = Vec::with_capacity(2);
        let fit_a = fit_checked(
            refitter,
            parameters,
            MultigroupFitRequestV1 {
                sample_kind: FitSampleKindV1::PairwisePermutation,
                group: pair.group_a,
                replicate: Some(replicate),
                source_rows: rows_for_group(&partition.assignments, pair.group_a),
            },
        );
        match &fit_a {
            Ok(_) => group_fits.push(GroupFitLedgerV1::usable(pair.group_a)),
            Err(failure) => {
                group_fits.push(GroupFitLedgerV1::failed(pair.group_a, failure.clone()))
            }
        }
        let fit_b = fit_checked(
            refitter,
            parameters,
            MultigroupFitRequestV1 {
                sample_kind: FitSampleKindV1::PairwisePermutation,
                group: pair.group_b,
                replicate: Some(replicate),
                source_rows: rows_for_group(&partition.assignments, pair.group_b),
            },
        );
        match &fit_b {
            Ok(_) => group_fits.push(GroupFitLedgerV1::usable(pair.group_b)),
            Err(failure) => {
                group_fits.push(GroupFitLedgerV1::failed(pair.group_b, failure.clone()))
            }
        }

        let status = if let (Ok(values_a), Ok(values_b)) = (fit_a, fit_b) {
            for index in 0..parameters.len() {
                let difference = values_a[index] - values_b[index];
                let observed = observed_differences[index];
                if index == 0 {
                    audit_null_differences.push(difference);
                }
                if difference.abs() >= observed.abs() {
                    absolute_extremes[index] += 1;
                }
                if difference >= observed {
                    greater_extremes[index] += 1;
                }
                if difference <= observed {
                    less_extremes[index] += 1;
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
            difference_a_minus_b: observed_differences[index],
        })
        .collect::<Vec<_>>();
    let inference = if availability == InferenceAvailabilityV1::Available {
        parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let two_sided = add_one_probability(absolute_extremes[index], usable);
                let greater = add_one_probability(greater_extremes[index], usable);
                let less = add_one_probability(less_extremes[index], usable);
                PairwisePermutationParameterV1 {
                    parameter: parameter.clone(),
                    estimate_a: point_a[index],
                    estimate_b: point_b[index],
                    difference_a_minus_b: observed_differences[index],
                    p_value_two_sided: two_sided,
                    p_value_greater: greater,
                    p_value_less: less,
                    selected_alternative: config.alternative,
                    selected_probability: selected_probability(
                        config.alternative,
                        two_sided,
                        greater,
                        less,
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
        method_version: MGA_MULTIGROUP_PAIRWISE_PERMUTATION_VERSION_V1.into(),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmnibusPermutationParameterV1 {
    pub parameter: ParameterIdentityV1,
    pub observed_maximum_pairwise_spread: f64,
    pub p_value_right_tailed: f64,
    /// Maximum across-group spread from every usable permutation, in usable
    /// ledger order. Retained so qualification and archive readers can
    /// independently reconstruct the published right-tailed probability.
    pub null_maximum_pairwise_spreads: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OmnibusPermutationResultV1 {
    pub method_version: String,
    pub seed: u64,
    pub requested: usize,
    pub attempted: usize,
    pub usable: usize,
    pub failed: usize,
    pub minimum_usable: usize,
    pub retry_policy: String,
    pub plan_sha256: String,
    pub availability: InferenceAvailabilityV1,
    pub group_point_estimates: Vec<GroupParameterVectorV1>,
    pub parameters: Vec<OmnibusPermutationParameterV1>,
    pub ledger: Vec<PermutationLedgerEntryV1>,
    pub group_counts: Vec<GroupEligibilitySummaryV1>,
    pub eligibility_warnings: Vec<EligibilityWarningV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupParameterVectorV1 {
    pub group: GroupIndexV1,
    pub values: Vec<f64>,
}

fn maximum_spreads(group_values: &[Vec<f64>], parameter_count: usize) -> Vec<f64> {
    (0..parameter_count)
        .map(|parameter| {
            let mut minimum = f64::INFINITY;
            let mut maximum = f64::NEG_INFINITY;
            for values in group_values {
                minimum = minimum.min(values[parameter]);
                maximum = maximum.max(values[parameter]);
            }
            maximum - minimum
        })
        .collect()
}

/// Runs the K-group max-spread omnibus permutation.  Each global label
/// assignment preserves the complete-case size of every selected group.
pub fn run_max_spread_omnibus_permutation_v1<R: MultigroupRefitterV1>(
    design: &MultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    refitter: &mut R,
) -> Result<OmnibusPermutationResultV1, MultigroupKernelErrorV1> {
    let eligibility = eligible_design(design)?;
    if design.groups.len() < 3 {
        return Err(MultigroupKernelErrorV1::OmnibusRequiresThreeGroups);
    }
    validate_parameter_identities(parameters)?;
    config.validate()?;
    let rows = canonical_rows(design);
    let mut point_values = Vec::with_capacity(design.groups.len());
    for group in &design.groups {
        point_values.push(fit_observed_group(
            &rows,
            group.index,
            parameters,
            refitter,
        )?);
    }
    let observed_spreads = maximum_spreads(&point_values, parameters.len());
    let mut extremes = vec![0usize; parameters.len()];
    let mut null_spreads = vec![Vec::with_capacity(config.requested); parameters.len()];
    let mut ledger = Vec::with_capacity(config.requested);
    let mut usable = 0usize;
    for replicate in 0..config.requested {
        let partition = omnibus_partition(&rows, design.groups.len(), config.seed, replicate);
        let mut group_fits = Vec::with_capacity(design.groups.len());
        let mut replicate_values = Vec::with_capacity(design.groups.len());
        let mut failed = false;
        for group in &design.groups {
            let fit = fit_checked(
                refitter,
                parameters,
                MultigroupFitRequestV1 {
                    sample_kind: FitSampleKindV1::OmnibusPermutation,
                    group: group.index,
                    replicate: Some(replicate),
                    source_rows: rows_from_assignments(&partition.assignments, group.index),
                },
            );
            match fit {
                Ok(values) => {
                    group_fits.push(GroupFitLedgerV1::usable(group.index));
                    replicate_values.push(values);
                }
                Err(failure) => {
                    failed = true;
                    group_fits.push(GroupFitLedgerV1::failed(group.index, failure));
                }
            }
        }
        let status = if failed {
            ResampleFitStatusV1::Failed
        } else {
            let spreads = maximum_spreads(&replicate_values, parameters.len());
            for index in 0..parameters.len() {
                null_spreads[index].push(spreads[index]);
                if spreads[index] >= observed_spreads[index] {
                    extremes[index] += 1;
                }
            }
            usable += 1;
            ResampleFitStatusV1::Usable
        };
        ledger.push(PermutationLedgerEntryV1 {
            replicate,
            partition_sha256: partition.digest,
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
    let inference = if availability == InferenceAvailabilityV1::Available {
        parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| OmnibusPermutationParameterV1 {
                parameter: parameter.clone(),
                observed_maximum_pairwise_spread: observed_spreads[index],
                p_value_right_tailed: add_one_probability(extremes[index], usable),
                null_maximum_pairwise_spreads: null_spreads[index].clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    let plan_sha256 = finish_plan_digest(
        MGA_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1,
        ledger
            .iter()
            .map(|entry| (entry.replicate, entry.partition_sha256.clone())),
    );
    Ok(OmnibusPermutationResultV1 {
        method_version: MGA_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1.into(),
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
        parameters: inference,
        ledger,
        group_counts: eligibility.group_counts,
        eligibility_warnings: eligibility.warnings,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapGroupLedgerV1 {
    pub group: GroupIndexV1,
    pub sample_sha256: String,
    pub status: ResampleFitStatusV1,
    pub failure: Option<RefitFailureV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapLedgerEntryV1 {
    pub replicate: usize,
    pub status: ResampleFitStatusV1,
    pub groups: Vec<BootstrapGroupLedgerV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupBootstrapBankV1 {
    pub group: GroupIndexV1,
    pub point_estimates: Vec<f64>,
    pub usable: usize,
    pub failed: usize,
    /// Entry `b` is `None` when replicate `b` failed for this group.
    pub replicate_estimates: Vec<Option<Vec<f64>>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupBootstrapBanksV1 {
    pub method_version: String,
    pub parameters: Vec<ParameterIdentityV1>,
    pub seed: u64,
    pub requested: usize,
    pub attempted: usize,
    pub minimum_usable: usize,
    pub retry_policy: String,
    pub plan_sha256: String,
    pub availability: InferenceAvailabilityV1,
    pub groups: Vec<GroupBootstrapBankV1>,
    pub ledger: Vec<BootstrapLedgerEntryV1>,
    pub group_counts: Vec<GroupEligibilitySummaryV1>,
    pub eligibility_warnings: Vec<EligibilityWarningV1>,
}

/// Builds deterministic independent case-bootstrap streams for every group.
/// The shared replicate index is retained so pairwise PLS-MGA probabilities
/// can use exactly the intersection of successful group fits.
pub fn run_group_bootstrap_banks_v1<R: MultigroupRefitterV1>(
    design: &MultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    refitter: &mut R,
) -> Result<GroupBootstrapBanksV1, MultigroupKernelErrorV1> {
    let eligibility = eligible_design(design)?;
    validate_parameter_identities(parameters)?;
    config.validate()?;
    let rows = canonical_rows(design);
    let source_by_group = design
        .groups
        .iter()
        .map(|group| rows_for_group(&rows, group.index))
        .collect::<Vec<_>>();
    let mut banks = Vec::with_capacity(design.groups.len());
    for group in &design.groups {
        banks.push(GroupBootstrapBankV1 {
            group: group.index,
            point_estimates: fit_observed_group(&rows, group.index, parameters, refitter)?,
            usable: 0,
            failed: 0,
            replicate_estimates: Vec::with_capacity(config.requested),
        });
    }
    let mut ledger = Vec::with_capacity(config.requested);
    for replicate in 0..config.requested {
        let mut group_ledger = Vec::with_capacity(design.groups.len());
        let mut all_usable = true;
        for (position, group) in design.groups.iter().enumerate() {
            let sample = bootstrap_rows(
                &source_by_group[position],
                group.index,
                config.seed,
                replicate,
            );
            let digest = sample_digest(group.index, &sample);
            let fit = fit_checked(
                refitter,
                parameters,
                MultigroupFitRequestV1 {
                    sample_kind: FitSampleKindV1::GroupBootstrap,
                    group: group.index,
                    replicate: Some(replicate),
                    source_rows: sample,
                },
            );
            match fit {
                Ok(values) => {
                    banks[position].usable += 1;
                    banks[position].replicate_estimates.push(Some(values));
                    group_ledger.push(BootstrapGroupLedgerV1 {
                        group: group.index,
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
                        group: group.index,
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
    let plan_sha256 = finish_plan_digest(
        MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1,
        ledger.iter().map(|entry| {
            let joined = entry
                .groups
                .iter()
                .map(|group| group.sample_sha256.as_str())
                .collect::<Vec<_>>()
                .join("|");
            (entry.replicate, sha256_prefixed(joined.as_bytes()))
        }),
    );
    Ok(GroupBootstrapBanksV1 {
        method_version: MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1.into(),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BiasCorrectedIntervalV1 {
    pub method_version: String,
    pub confidence_level: f64,
    pub usable_replicates: usize,
    pub bias_correction_z0: f64,
    /// Always zero.  This is BC, not BCa.
    pub acceleration: f64,
    pub adjusted_lower_probability: f64,
    pub adjusted_upper_probability: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectionalBiasCorrectedIntervalV1 {
    pub method_version: String,
    pub confidence_level: f64,
    pub alternative: AlternativeHypothesisV1,
    pub usable_replicates: usize,
    pub bias_correction_z0: f64,
    /// Always zero. This is BC, not BCa.
    pub acceleration: f64,
    pub adjusted_lower_probability: Option<f64>,
    pub adjusted_upper_probability: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

fn type7_quantile(values: &[f64], probability: f64) -> f64 {
    debug_assert!(!values.is_empty());
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = (sorted.len() - 1) as f64 * probability.clamp(0.0, 1.0);
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

/// Computes an Efron bias-corrected interval with acceleration fixed to zero.
/// The continuity correction prevents infinite z0 when all draws lie on one
/// side of the point estimate.  It must be labelled BC, never BCa.
pub fn bias_corrected_interval_v1(
    point_estimate: f64,
    replicates: &[f64],
    confidence_level: f64,
) -> Result<BiasCorrectedIntervalV1, MultigroupKernelErrorV1> {
    let directional = bias_corrected_interval_for_alternative_v1(
        point_estimate,
        replicates,
        confidence_level,
        AlternativeHypothesisV1::TwoSided,
    )?;
    Ok(BiasCorrectedIntervalV1 {
        method_version: directional.method_version,
        confidence_level: directional.confidence_level,
        usable_replicates: directional.usable_replicates,
        bias_correction_z0: directional.bias_correction_z0,
        acceleration: directional.acceleration,
        adjusted_lower_probability: directional
            .adjusted_lower_probability
            .expect("two-sided BC has a lower probability"),
        adjusted_upper_probability: directional
            .adjusted_upper_probability
            .expect("two-sided BC has an upper probability"),
        lower: directional.lower.expect("two-sided BC has a lower bound"),
        upper: directional.upper.expect("two-sided BC has an upper bound"),
    })
}

/// Computes a two- or one-sided Efron bias-corrected interval with
/// acceleration fixed to zero. A one-sided interval publishes only its finite
/// bound: `Less` has an upper bound and `Greater` has a lower bound.
pub fn bias_corrected_interval_for_alternative_v1(
    point_estimate: f64,
    replicates: &[f64],
    confidence_level: f64,
    alternative: AlternativeHypothesisV1,
) -> Result<DirectionalBiasCorrectedIntervalV1, MultigroupKernelErrorV1> {
    if !point_estimate.is_finite()
        || replicates.len() < 2
        || replicates.iter().any(|value| !value.is_finite())
    {
        return Err(MultigroupKernelErrorV1::InvalidBootstrapBank(
            "BC intervals require a finite point estimate and at least two finite draws".into(),
        ));
    }
    if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err(MultigroupKernelErrorV1::InvalidConfidenceLevel {
            value: confidence_level,
        });
    }
    let less = replicates
        .iter()
        .filter(|value| **value < point_estimate)
        .count();
    let ties = replicates
        .iter()
        .filter(|value| **value == point_estimate)
        .count();
    let n = replicates.len() as f64;
    let corrected_rank = (less as f64 + 0.5 * ties as f64 + 0.5) / (n + 1.0);
    let normal = Normal::standard();
    let z0 = normal.inverse_cdf(corrected_rank.clamp(f64::EPSILON, 1.0 - f64::EPSILON));
    let alpha = 1.0 - confidence_level;
    let (nominal_lower, nominal_upper) = match alternative {
        AlternativeHypothesisV1::TwoSided => (Some(alpha / 2.0), Some(1.0 - alpha / 2.0)),
        AlternativeHypothesisV1::Less => (None, Some(confidence_level)),
        AlternativeHypothesisV1::Greater => (Some(alpha), None),
    };
    let adjusted_lower =
        nominal_lower.map(|probability| normal.cdf(2.0 * z0 + normal.inverse_cdf(probability)));
    let adjusted_upper =
        nominal_upper.map(|probability| normal.cdf(2.0 * z0 + normal.inverse_cdf(probability)));
    if !z0.is_finite()
        || adjusted_lower.is_some_and(|probability| !probability.is_finite())
        || adjusted_upper.is_some_and(|probability| !probability.is_finite())
        || matches!((adjusted_lower, adjusted_upper), (Some(lower), Some(upper)) if lower > upper)
    {
        return Err(MultigroupKernelErrorV1::Distribution(
            "bias-corrected probability transform was nonfinite or reversed".into(),
        ));
    }
    Ok(DirectionalBiasCorrectedIntervalV1 {
        method_version: MGA_MULTIGROUP_BC_INTERVAL_VERSION_V1.into(),
        confidence_level,
        alternative,
        usable_replicates: replicates.len(),
        bias_correction_z0: z0,
        acceleration: 0.0,
        adjusted_lower_probability: adjusted_lower,
        adjusted_upper_probability: adjusted_upper,
        lower: adjusted_lower.map(|probability| type7_quantile(replicates, probability)),
        upper: adjusted_upper.map(|probability| type7_quantile(replicates, probability)),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupBiasCorrectedIntervalV1 {
    pub group: GroupIndexV1,
    pub parameter: ParameterIdentityV1,
    pub interval: BiasCorrectedIntervalV1,
}

fn validate_bootstrap_banks(banks: &GroupBootstrapBanksV1) -> Result<(), MultigroupKernelErrorV1> {
    validate_parameter_identities(&banks.parameters)?;
    if ![
        MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1,
        crate::FREQUENCY_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1,
    ]
    .contains(&banks.method_version.as_str())
        || banks.requested != banks.attempted
        || banks.ledger.len() != banks.requested
        || banks.groups.len() < MGA_MULTIGROUP_MIN_GROUPS_V1
        || banks.groups.len() > MGA_MULTIGROUP_MAX_GROUPS_V1
        || banks.group_counts.len() != banks.groups.len()
    {
        return Err(MultigroupKernelErrorV1::InvalidBootstrapBank(
            "method identity, counts, or group envelope is invalid".into(),
        ));
    }
    let mut seen = BTreeSet::new();
    for (position, bank) in banks.groups.iter().enumerate() {
        if !seen.insert(bank.group)
            || bank.group.get() != position
            || banks.group_counts[position].group != bank.group
            || banks.group_counts[position].complete_cases < MGA_MULTIGROUP_MIN_COMPLETE_CASES_V1
            || bank.point_estimates.len() != banks.parameters.len()
            || bank.replicate_estimates.len() != banks.requested
            || bank.usable + bank.failed != banks.requested
            || bank.point_estimates.iter().any(|value| !value.is_finite())
        {
            return Err(MultigroupKernelErrorV1::InvalidBootstrapBank(
                "group bank identity, vector dimensions, or counts are invalid".into(),
            ));
        }
        let actual_usable = bank
            .replicate_estimates
            .iter()
            .filter(|values| values.is_some())
            .count();
        if actual_usable != bank.usable
            || bank.replicate_estimates.iter().flatten().any(|values| {
                values.len() != banks.parameters.len()
                    || values.iter().any(|value| !value.is_finite())
            })
        {
            return Err(MultigroupKernelErrorV1::InvalidBootstrapBank(
                "group bank payload disagrees with usable accounting".into(),
            ));
        }
    }
    Ok(())
}

pub fn group_bias_corrected_intervals_v1(
    banks: &GroupBootstrapBanksV1,
    confidence_level: f64,
) -> Result<Vec<GroupBiasCorrectedIntervalV1>, MultigroupKernelErrorV1> {
    validate_bootstrap_banks(banks)?;
    let mut intervals = Vec::with_capacity(banks.groups.len() * banks.parameters.len());
    for bank in &banks.groups {
        if bank.usable < banks.minimum_usable {
            return Err(MultigroupKernelErrorV1::InsufficientUsableResamples {
                required: banks.minimum_usable,
                found: bank.usable,
            });
        }
        for (parameter_index, parameter) in banks.parameters.iter().enumerate() {
            let replicates = bank
                .replicate_estimates
                .iter()
                .flatten()
                .map(|values| values[parameter_index])
                .collect::<Vec<_>>();
            intervals.push(GroupBiasCorrectedIntervalV1 {
                group: bank.group,
                parameter: parameter.clone(),
                interval: bias_corrected_interval_v1(
                    bank.point_estimates[parameter_index],
                    &replicates,
                    confidence_level,
                )?,
            });
        }
    }
    Ok(intervals)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HenselerDirectionalDecisionV1 {
    GroupALower,
    GroupAHigher,
    NotSignificant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HenselerDirectionalParameterV1 {
    pub method_version: String,
    pub pair: OrderedGroupPairV1,
    pub parameter: ParameterIdentityV1,
    pub point_difference_a_minus_b: f64,
    pub matched_usable_replicates: usize,
    pub greater_differences: usize,
    pub equal_differences: usize,
    pub less_differences: usize,
    /// Empirical Pr(theta_A* > theta_B*), with half weight for ties.  This is
    /// directional probability, not a conventional two-sided p-value.
    pub directional_probability_a_greater: f64,
    pub alpha: f64,
    pub decision: HenselerDirectionalDecisionV1,
}

/// Summarizes matched bootstrap-bank draws for an ordered pair.  Reversing the
/// pair returns exactly `1 - probability` and the opposite point difference.
pub fn henseler_directional_probabilities_v1(
    banks: &GroupBootstrapBanksV1,
    pair: OrderedGroupPairV1,
    alpha: f64,
) -> Result<Vec<HenselerDirectionalParameterV1>, MultigroupKernelErrorV1> {
    validate_bootstrap_banks(banks)?;
    if pair.group_a == pair.group_b {
        return Err(MultigroupKernelErrorV1::SameGroupComparison {
            group: pair.group_a,
        });
    }
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 0.5 {
        return Err(MultigroupKernelErrorV1::InvalidAlpha { value: alpha });
    }
    let bank_a = banks
        .groups
        .iter()
        .find(|bank| bank.group == pair.group_a)
        .ok_or(MultigroupKernelErrorV1::UnknownComparisonGroup)?;
    let bank_b = banks
        .groups
        .iter()
        .find(|bank| bank.group == pair.group_b)
        .ok_or(MultigroupKernelErrorV1::UnknownComparisonGroup)?;
    let mut output = Vec::with_capacity(banks.parameters.len());
    for (parameter_index, parameter) in banks.parameters.iter().enumerate() {
        let mut greater = 0usize;
        let mut equal = 0usize;
        let mut less = 0usize;
        for (left, right) in bank_a
            .replicate_estimates
            .iter()
            .zip(&bank_b.replicate_estimates)
        {
            if let (Some(left), Some(right)) = (left, right) {
                let difference = left[parameter_index] - right[parameter_index];
                if difference > 0.0 {
                    greater += 1;
                } else if difference < 0.0 {
                    less += 1;
                } else {
                    equal += 1;
                }
            }
        }
        let usable = greater + equal + less;
        if usable < banks.minimum_usable {
            return Err(MultigroupKernelErrorV1::InsufficientUsableResamples {
                required: banks.minimum_usable,
                found: usable,
            });
        }
        let probability = (greater as f64 + 0.5 * equal as f64) / usable as f64;
        let decision = if probability <= alpha {
            HenselerDirectionalDecisionV1::GroupALower
        } else if probability >= 1.0 - alpha {
            HenselerDirectionalDecisionV1::GroupAHigher
        } else {
            HenselerDirectionalDecisionV1::NotSignificant
        };
        output.push(HenselerDirectionalParameterV1 {
            method_version: MGA_MULTIGROUP_HENSELER_PROBABILITY_VERSION_V1.into(),
            pair,
            parameter: parameter.clone(),
            point_difference_a_minus_b: bank_a.point_estimates[parameter_index]
                - bank_b.point_estimates[parameter_index],
            matched_usable_replicates: usable,
            greater_differences: greater,
            equal_differences: equal,
            less_differences: less,
            directional_probability_a_greater: probability,
            alpha,
            decision,
        });
    }
    Ok(output)
}

/// Audit receipt for the ordinary recursive PLS path sensitivity SE.  This is
/// classical homoskedastic OLS conditional on the estimated construct scores;
/// it is not a full measurement-model or bootstrap uncertainty estimate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrdinaryPlsPathStandardErrorV1 {
    pub method_version: String,
    pub source: String,
    pub target: String,
    pub estimate: f64,
    pub standard_error: f64,
    pub observations: usize,
    pub predictor_count: usize,
    pub variance_degrees_of_freedom: f64,
    pub residual_sum_of_squares: f64,
    /// Diagonal element of `(X'X)^-1` for the selected centered predictor.
    pub coefficient_variance_factor: f64,
}

fn solve_parametric_linear_system_v1(
    mut system: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
    subject: &str,
) -> Result<Vec<f64>, MultigroupKernelErrorV1> {
    let count = rhs.len();
    if system.len() != count || system.iter().any(|row| row.len() != count) {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(format!(
            "{subject} regression system has incompatible dimensions"
        )));
    }
    let scale = system
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let pivot_floor = 1e-12 * scale.max(1.0);
    for pivot in 0..count {
        let mut selected = pivot;
        let mut selected_abs = system[pivot][pivot].abs();
        for candidate in (pivot + 1)..count {
            if system[candidate][pivot].abs() > selected_abs {
                selected = candidate;
                selected_abs = system[candidate][pivot].abs();
            }
        }
        if !selected_abs.is_finite() || selected_abs <= pivot_floor {
            return Err(MultigroupKernelErrorV1::InvalidParametricInput(format!(
                "{subject} construct-score design is rank deficient"
            )));
        }
        if selected != pivot {
            system.swap(selected, pivot);
            rhs.swap(selected, pivot);
        }
        let pivot_value = system[pivot][pivot];
        for column in pivot..count {
            system[pivot][column] /= pivot_value;
        }
        rhs[pivot] /= pivot_value;
        for row in 0..count {
            if row == pivot {
                continue;
            }
            let factor = system[row][pivot];
            for column in pivot..count {
                system[row][column] -= factor * system[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    if rhs.iter().all(|value| value.is_finite()) {
        Ok(rhs)
    } else {
        Err(MultigroupKernelErrorV1::InvalidParametricInput(format!(
            "{subject} regression solution is nonfinite"
        )))
    }
}

fn ordinary_pls_path_se_from_scores_v1(
    scores: &BTreeMap<String, Vec<f64>>,
    source: &str,
    target: &str,
    predecessor_ids: &[String],
    fitted_estimate: f64,
    expected_observations: usize,
) -> Result<OrdinaryPlsPathStandardErrorV1, MultigroupKernelErrorV1> {
    let mut identities = BTreeSet::new();
    if predecessor_ids.is_empty()
        || predecessor_ids
            .iter()
            .any(|id| id.trim().is_empty() || !identities.insert(id.as_str()))
    {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "path SE requires a nonempty unique predecessor set".into(),
        ));
    }
    let selected_index = predecessor_ids
        .iter()
        .position(|candidate| candidate == source)
        .ok_or_else(|| {
            MultigroupKernelErrorV1::InvalidParametricInput(format!(
                "selected source {source} is absent from the {target} equation"
            ))
        })?;
    let outcome = scores.get(target).ok_or_else(|| {
        MultigroupKernelErrorV1::InvalidParametricInput(format!(
            "construct scores are missing for outcome {target}"
        ))
    })?;
    let predictors = predecessor_ids
        .iter()
        .map(|id| {
            scores.get(id).ok_or_else(|| {
                MultigroupKernelErrorV1::InvalidParametricInput(format!(
                    "construct scores are missing for predictor {id}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let observations = outcome.len();
    let predictor_count = predictors.len();
    let minimum_observations = predictor_count.checked_add(1).ok_or_else(|| {
        MultigroupKernelErrorV1::InvalidParametricInput(
            "path predecessor count exceeds the supported integer range".into(),
        )
    })?;
    if observations != expected_observations
        || observations <= minimum_observations
        || outcome.iter().any(|value| !value.is_finite())
        || predictors.iter().any(|values| {
            values.len() != observations || values.iter().any(|value| !value.is_finite())
        })
        || !fitted_estimate.is_finite()
    {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(format!(
            "{source}->{target} scores, observations, or fitted estimate are invalid"
        )));
    }
    let predictor_means = predictors
        .iter()
        .map(|values| values.iter().sum::<f64>() / observations as f64)
        .collect::<Vec<_>>();
    let outcome_mean = outcome.iter().sum::<f64>() / observations as f64;
    let mut xtx = vec![vec![0.0; predictor_count]; predictor_count];
    let mut xty = vec![0.0; predictor_count];
    for row in 0..observations {
        let centered_outcome = outcome[row] - outcome_mean;
        for left in 0..predictor_count {
            let centered_left = predictors[left][row] - predictor_means[left];
            xty[left] += centered_left * centered_outcome;
            for right in 0..predictor_count {
                xtx[left][right] +=
                    centered_left * (predictors[right][row] - predictor_means[right]);
            }
        }
    }
    let subject = format!("ordinary PLS path {source}->{target}");
    let coefficients = solve_parametric_linear_system_v1(xtx.clone(), xty, &subject)?;
    let recomputed = coefficients[selected_index];
    let agreement_tolerance = 1e-9 * fitted_estimate.abs().max(recomputed.abs()).max(1.0);
    if (recomputed - fitted_estimate).abs() > agreement_tolerance {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(format!(
            "{source}->{target} fitted coefficient does not reproduce from its construct scores"
        )));
    }
    let residual_sum_of_squares = (0..observations)
        .map(|row| {
            let fitted = (0..predictor_count)
                .map(|column| {
                    coefficients[column] * (predictors[column][row] - predictor_means[column])
                })
                .sum::<f64>();
            (outcome[row] - outcome_mean - fitted).powi(2)
        })
        .sum::<f64>();
    let variance_degrees_of_freedom = (observations - minimum_observations) as f64;
    let mut basis = vec![0.0; predictor_count];
    basis[selected_index] = 1.0;
    let inverse_column = solve_parametric_linear_system_v1(xtx, basis, &subject)?;
    let coefficient_variance_factor = inverse_column[selected_index];
    let variance =
        residual_sum_of_squares / variance_degrees_of_freedom * coefficient_variance_factor;
    if !residual_sum_of_squares.is_finite()
        || residual_sum_of_squares <= 0.0
        || !coefficient_variance_factor.is_finite()
        || coefficient_variance_factor <= 0.0
        || !variance.is_finite()
        || variance <= 0.0
    {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(format!(
            "{source}->{target} has zero/nonfinite residual variance or coefficient variance factor"
        )));
    }
    Ok(OrdinaryPlsPathStandardErrorV1 {
        method_version: MGA_ORDINARY_PLS_PATH_STANDARD_ERROR_VERSION_V1.into(),
        source: source.into(),
        target: target.into(),
        estimate: fitted_estimate,
        standard_error: variance.sqrt(),
        observations,
        predictor_count,
        variance_degrees_of_freedom,
        residual_sum_of_squares,
        coefficient_variance_factor,
    })
}

/// Extracts a qualified score-conditional structural-path SE from one complete
/// ordinary PLS group fit. All structural and control predecessors of the
/// outcome must be supplied in the exact fitted equation.
pub fn ordinary_pls_path_standard_error_v1(
    result: &PlsResult,
    source: &str,
    target: &str,
    predecessor_ids: &[String],
) -> Result<OrdinaryPlsPathStandardErrorV1, MultigroupKernelErrorV1> {
    let fitted_estimate = result
        .paths
        .iter()
        .find(|path| path.source == source && path.target == target)
        .map(|path| path.coefficient)
        .or_else(|| {
            result
                .control_estimates
                .iter()
                .find(|path| path.source == source && path.target == target)
                .map(|path| path.coefficient)
        })
        .ok_or_else(|| {
            MultigroupKernelErrorV1::InvalidParametricInput(format!(
                "ordinary PLS result omits selected path {source}->{target}"
            ))
        })?;
    ordinary_pls_path_se_from_scores_v1(
        &result.construct_scores,
        source,
        target,
        predecessor_ids,
        fitted_estimate,
        result.used_observations,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParametricGroupSeMethodV1 {
    /// Classical homoskedastic OLS conditional on ordinary PLS construct
    /// scores; measurement/scoring uncertainty is not included.
    OrdinaryPlsScoreConditionalCenteredOls,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParametricGroupEstimateV1 {
    pub group: GroupIndexV1,
    pub estimate: f64,
    pub standard_error_method: ParametricGroupSeMethodV1,
    /// Standard error of the group-specific parameter estimator.
    pub standard_error: f64,
    pub observations: usize,
    pub predictor_count: usize,
    /// Degrees of freedom attached to the variance estimate.
    pub variance_degrees_of_freedom: f64,
    /// Residual sum of squares from the centered group structural equation.
    pub residual_sum_of_squares: f64,
    /// Diagonal `(X'X)^-1` factor for this coefficient. Required so pooled
    /// variance inference remains valid with correlated/multiple predictors.
    pub coefficient_variance_factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairwiseParametricMethodV1 {
    PooledEqualResidualVariance,
    WelchSatterthwaite,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairwiseParametricTestV1 {
    pub method_version: String,
    pub method: PairwiseParametricMethodV1,
    pub pair: OrderedGroupPairV1,
    pub difference_a_minus_b: f64,
    pub standard_error_of_difference: f64,
    pub t_statistic: f64,
    pub degrees_of_freedom: f64,
    pub p_value_two_sided: f64,
    pub p_value_greater: f64,
    pub p_value_less: f64,
    pub selected_alternative: AlternativeHypothesisV1,
    pub selected_probability: f64,
}

pub fn validate_parametric_group_estimate_v1(
    input: ParametricGroupEstimateV1,
) -> Result<(), MultigroupKernelErrorV1> {
    let minimum_observations = input.predictor_count.checked_add(1).ok_or_else(|| {
        MultigroupKernelErrorV1::InvalidParametricInput(
            "predictor count exceeds the supported integer range".into(),
        )
    })?;
    if !input.estimate.is_finite()
        || !input.standard_error.is_finite()
        || input.standard_error <= 0.0
        || input.predictor_count == 0
        || input.observations <= minimum_observations
        || !input.variance_degrees_of_freedom.is_finite()
        || input.variance_degrees_of_freedom <= 0.0
        || !input.residual_sum_of_squares.is_finite()
        || input.residual_sum_of_squares <= 0.0
        || !input.coefficient_variance_factor.is_finite()
        || input.coefficient_variance_factor <= 0.0
    {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "finite estimates, positive standard errors/RSS/design factors/df, and n greater than predictors plus one are required"
                .into(),
        ));
    }
    let expected_df = (input.observations - minimum_observations) as f64;
    if input.standard_error_method
        != ParametricGroupSeMethodV1::OrdinaryPlsScoreConditionalCenteredOls
        || input.variance_degrees_of_freedom != expected_df
    {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "ordinary PLS path SE requires its typed score-conditional method and df = n - predictors - 1"
                .into(),
        ));
    }
    let reconstructed_variance = input.residual_sum_of_squares / input.variance_degrees_of_freedom
        * input.coefficient_variance_factor;
    let reported_variance = input.standard_error.powi(2);
    let tolerance = 1e-8
        * reconstructed_variance
            .abs()
            .max(reported_variance.abs())
            .max(f64::EPSILON);
    if (reconstructed_variance - reported_variance).abs() > tolerance {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "reported standard error is inconsistent with RSS, residual df, and coefficient variance factor"
                .into(),
        ));
    }
    Ok(())
}

fn validate_parametric_pair(
    a: ParametricGroupEstimateV1,
    b: ParametricGroupEstimateV1,
) -> Result<OrderedGroupPairV1, MultigroupKernelErrorV1> {
    let pair = OrderedGroupPairV1::new(a.group, b.group)?;
    validate_parametric_group_estimate_v1(a)?;
    validate_parametric_group_estimate_v1(b)?;
    Ok(pair)
}

fn t_tail_probabilities(
    statistic: f64,
    degrees_of_freedom: f64,
) -> Result<(f64, f64, f64), MultigroupKernelErrorV1> {
    if !statistic.is_finite() || !degrees_of_freedom.is_finite() || degrees_of_freedom <= 0.0 {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "finite t statistic and positive degrees of freedom are required".into(),
        ));
    }
    let distribution = StudentsT::new(0.0, 1.0, degrees_of_freedom)
        .map_err(|error| MultigroupKernelErrorV1::Distribution(error.to_string()))?;
    // Derive both directional tails from one |t| calculation so reversing
    // labels swaps greater/less exactly while preserving the two-sided value.
    let tail_abs = distribution.sf(statistic.abs()).clamp(0.0, 0.5);
    let (greater, less) = if statistic.is_sign_negative() {
        (1.0 - tail_abs, tail_abs)
    } else {
        (tail_abs, 1.0 - tail_abs)
    };
    let two_sided = (2.0 * tail_abs).clamp(0.0, 1.0);
    Ok((two_sided, greater, less))
}

fn assemble_parametric_test(
    version: &str,
    method: PairwiseParametricMethodV1,
    pair: OrderedGroupPairV1,
    difference: f64,
    standard_error: f64,
    degrees_of_freedom: f64,
    alternative: AlternativeHypothesisV1,
) -> Result<PairwiseParametricTestV1, MultigroupKernelErrorV1> {
    if !standard_error.is_finite() || standard_error <= 0.0 {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "difference standard error must be finite and positive".into(),
        ));
    }
    let statistic = difference / standard_error;
    let (two_sided, greater, less) = t_tail_probabilities(statistic, degrees_of_freedom)?;
    Ok(PairwiseParametricTestV1 {
        method_version: version.into(),
        method,
        pair,
        difference_a_minus_b: difference,
        standard_error_of_difference: standard_error,
        t_statistic: statistic,
        degrees_of_freedom,
        p_value_two_sided: two_sided,
        p_value_greater: greater,
        p_value_less: less,
        selected_alternative: alternative,
        selected_probability: selected_probability(alternative, two_sided, greater, less),
    })
}

/// Equal-residual-variance score-conditional sensitivity test. Residual sums
/// of squares are pooled by their equation df; each group's exact centered
/// design factor converts that pooled variance to a coefficient variance.
pub fn pooled_variance_parameter_test_v1(
    a: ParametricGroupEstimateV1,
    b: ParametricGroupEstimateV1,
    alternative: AlternativeHypothesisV1,
) -> Result<PairwiseParametricTestV1, MultigroupKernelErrorV1> {
    let pair = validate_parametric_pair(a, b)?;
    let df = a.variance_degrees_of_freedom + b.variance_degrees_of_freedom;
    let pooled = (a.residual_sum_of_squares + b.residual_sum_of_squares) / df;
    let difference_se =
        (pooled * (a.coefficient_variance_factor + b.coefficient_variance_factor)).sqrt();
    assemble_parametric_test(
        MGA_MULTIGROUP_POOLED_TEST_VERSION_V1,
        PairwiseParametricMethodV1::PooledEqualResidualVariance,
        pair,
        a.estimate - b.estimate,
        difference_se,
        df,
        alternative,
    )
}

pub fn welch_satterthwaite_parameter_test_v1(
    a: ParametricGroupEstimateV1,
    b: ParametricGroupEstimateV1,
    alternative: AlternativeHypothesisV1,
) -> Result<PairwiseParametricTestV1, MultigroupKernelErrorV1> {
    let pair = validate_parametric_pair(a, b)?;
    let variance_a = a.standard_error.powi(2);
    let variance_b = b.standard_error.powi(2);
    let combined = variance_a + variance_b;
    let df = combined.powi(2)
        / (variance_a.powi(2) / a.variance_degrees_of_freedom
            + variance_b.powi(2) / b.variance_degrees_of_freedom);
    assemble_parametric_test(
        MGA_MULTIGROUP_WELCH_TEST_VERSION_V1,
        PairwiseParametricMethodV1::WelchSatterthwaite,
        pair,
        a.estimate - b.estimate,
        combined.sqrt(),
        df,
        alternative,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaldGroupEstimateV1 {
    pub group: GroupIndexV1,
    pub estimate: f64,
    pub standard_error: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InverseVarianceWaldResultV1 {
    pub method_version: String,
    pub groups: Vec<GroupIndexV1>,
    pub inverse_variance_weighted_mean: f64,
    pub chi_square: f64,
    pub degrees_of_freedom: usize,
    pub p_value_right_tailed: f64,
}

/// K-group heterogeneity test for independent estimates with externally
/// qualified standard errors.  This is a sensitivity cell, not MICOM.
pub fn inverse_variance_wald_test_v1(
    inputs: &[WaldGroupEstimateV1],
) -> Result<InverseVarianceWaldResultV1, MultigroupKernelErrorV1> {
    if inputs.len() < 2 || inputs.len() > MGA_MULTIGROUP_MAX_GROUPS_V1 {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "inverse-variance Wald requires two to twenty groups".into(),
        ));
    }
    let mut groups = BTreeSet::new();
    if inputs.iter().any(|input| {
        !groups.insert(input.group)
            || !input.estimate.is_finite()
            || !input.standard_error.is_finite()
            || input.standard_error <= 0.0
    }) {
        return Err(MultigroupKernelErrorV1::InvalidParametricInput(
            "Wald inputs require unique groups, finite estimates, and positive standard errors"
                .into(),
        ));
    }
    let total_weight = inputs
        .iter()
        .map(|input| 1.0 / input.standard_error.powi(2))
        .sum::<f64>();
    let weighted_mean = inputs
        .iter()
        .map(|input| input.estimate / input.standard_error.powi(2))
        .sum::<f64>()
        / total_weight;
    let statistic = inputs
        .iter()
        .map(|input| (input.estimate - weighted_mean).powi(2) / input.standard_error.powi(2))
        .sum::<f64>();
    let degrees_of_freedom = inputs.len() - 1;
    let distribution = ChiSquared::new(degrees_of_freedom as f64)
        .map_err(|error| MultigroupKernelErrorV1::Distribution(error.to_string()))?;
    Ok(InverseVarianceWaldResultV1 {
        method_version: MGA_MULTIGROUP_WALD_TEST_VERSION_V1.into(),
        groups: inputs.iter().map(|input| input.group).collect(),
        inverse_variance_weighted_mean: weighted_mean,
        chi_square: statistic,
        degrees_of_freedom,
        p_value_right_tailed: distribution.sf(statistic).clamp(0.0, 1.0),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HypothesisProbabilityV1 {
    pub hypothesis_id: String,
    pub raw_probability: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdjustedProbabilityV1 {
    pub method_version: String,
    pub method: MultiplicityMethodV1,
    pub hypothesis_id: String,
    pub raw_probability: f64,
    pub adjusted_probability: f64,
}

/// Adjusts one predeclared probability family.  Missing/unavailable hypotheses
/// must be excluded before calling; the caller persists that family choice.
pub fn adjust_probabilities_v1(
    hypotheses: &[HypothesisProbabilityV1],
    method: MultiplicityMethodV1,
) -> Result<Vec<AdjustedProbabilityV1>, MultigroupKernelErrorV1> {
    let mut identities = BTreeSet::new();
    if hypotheses.is_empty()
        || hypotheses.iter().any(|hypothesis| {
            hypothesis.hypothesis_id.trim().is_empty()
                || !identities.insert(hypothesis.hypothesis_id.clone())
        })
    {
        return Err(MultigroupKernelErrorV1::InvalidHypothesisIdentities);
    }
    if hypotheses.iter().any(|hypothesis| {
        !hypothesis.raw_probability.is_finite()
            || hypothesis.raw_probability < 0.0
            || hypothesis.raw_probability > 1.0
    }) {
        return Err(MultigroupKernelErrorV1::InvalidProbability);
    }
    let m = hypotheses.len();
    let mut adjusted = vec![0.0; m];
    match method {
        MultiplicityMethodV1::None => {
            for (index, hypothesis) in hypotheses.iter().enumerate() {
                adjusted[index] = hypothesis.raw_probability;
            }
        }
        MultiplicityMethodV1::Bonferroni => {
            for (index, hypothesis) in hypotheses.iter().enumerate() {
                adjusted[index] = (hypothesis.raw_probability * m as f64).min(1.0);
            }
        }
        MultiplicityMethodV1::Sidak => {
            for (index, hypothesis) in hypotheses.iter().enumerate() {
                adjusted[index] = if hypothesis.raw_probability >= 1.0 {
                    1.0
                } else {
                    (-(m as f64 * (-hypothesis.raw_probability).ln_1p()).exp_m1()).clamp(0.0, 1.0)
                };
            }
        }
        MultiplicityMethodV1::Holm => {
            let mut order = (0..m).collect::<Vec<_>>();
            order.sort_by(|left, right| {
                hypotheses[*left]
                    .raw_probability
                    .total_cmp(&hypotheses[*right].raw_probability)
                    .then_with(|| {
                        hypotheses[*left]
                            .hypothesis_id
                            .cmp(&hypotheses[*right].hypothesis_id)
                    })
            });
            let mut running: f64 = 0.0;
            for (rank, index) in order.into_iter().enumerate() {
                running = running
                    .max(hypotheses[index].raw_probability * (m - rank) as f64)
                    .min(1.0);
                adjusted[index] = running;
            }
        }
        MultiplicityMethodV1::BenjaminiHochberg => {
            let mut order = (0..m).collect::<Vec<_>>();
            order.sort_by(|left, right| {
                hypotheses[*left]
                    .raw_probability
                    .total_cmp(&hypotheses[*right].raw_probability)
                    .then_with(|| {
                        hypotheses[*left]
                            .hypothesis_id
                            .cmp(&hypotheses[*right].hypothesis_id)
                    })
            });
            let mut running: f64 = 1.0;
            for rank_from_zero in (0..m).rev() {
                let index = order[rank_from_zero];
                running = running
                    .min(hypotheses[index].raw_probability * m as f64 / (rank_from_zero + 1) as f64)
                    .min(1.0);
                adjusted[index] = running;
            }
        }
    }
    Ok(hypotheses
        .iter()
        .enumerate()
        .map(|(index, hypothesis)| AdjustedProbabilityV1 {
            method_version: MGA_MULTIGROUP_MULTIPLICITY_VERSION_V1.into(),
            method,
            hypothesis_id: hypothesis.hypothesis_id.clone(),
            raw_probability: hypothesis.raw_probability,
            adjusted_probability: adjusted[index],
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(index: usize, label: &str) -> GroupIdentityV1 {
        GroupIdentityV1 {
            index: GroupIndexV1::new(index).unwrap(),
            value: TypedGroupValueV1::Text {
                value: label.into(),
            },
            display_label: label.into(),
        }
    }

    fn design_with_sizes(sizes: &[usize]) -> MultigroupDesignV1 {
        let groups = sizes
            .iter()
            .enumerate()
            .map(|(index, _)| group(index, &format!("G{index}")))
            .collect::<Vec<_>>();
        let mut source_row = 100u64;
        let mut rows = Vec::new();
        for (index, size) in sizes.iter().enumerate() {
            for _ in 0..*size {
                rows.push(SelectedGroupRowV1 {
                    source_row,
                    group: GroupIndexV1::new(index).unwrap(),
                });
                source_row += 3;
            }
        }
        MultigroupDesignV1 { groups, rows }
    }

    fn parameter(id: &str) -> ParameterIdentityV1 {
        ParameterIdentityV1 {
            stable_id: id.into(),
            family: ParameterFamilyV1::StructuralPath,
        }
    }

    fn vector(id: &str, value: f64) -> ParameterVectorV1 {
        ParameterVectorV1 {
            parameters: vec![ParameterEstimateV1 {
                parameter: parameter(id),
                estimate: value,
            }],
        }
    }

    #[test]
    fn typed_group_values_do_not_collapse_across_source_types() {
        let values = BTreeSet::from([
            TypedGroupValueV1::Text { value: "1".into() },
            TypedGroupValueV1::Integer { value: 1 },
            TypedGroupValueV1::finite_number(1.0).unwrap(),
            TypedGroupValueV1::Boolean { value: true },
        ]);
        assert_eq!(values.len(), 4);
        assert!(TypedGroupValueV1::finite_number(f64::NAN).is_err());
    }

    #[test]
    fn eligibility_enforces_minimum_small_group_and_imbalance_boundaries() {
        let eligible = assess_multigroup_design_v1(&design_with_sizes(&[10, 21, 30]));
        assert!(eligible.eligible);
        assert_eq!(eligible.maximum_imbalance_ratio, Some(3.0));
        assert!(eligible.warnings.iter().any(|warning| {
            warning.code == EligibilityWarningCodeV1::GroupImbalanceAboveTwoToOne
        }));
        assert_eq!(
            eligible
                .warnings
                .iter()
                .filter(|warning| warning.code == EligibilityWarningCodeV1::SmallGroup)
                .count(),
            2
        );

        let ten_to_one = assess_multigroup_design_v1(&design_with_sizes(&[10, 100]));
        assert!(ten_to_one.eligible);
        let over_ten = assess_multigroup_design_v1(&design_with_sizes(&[10, 101]));
        assert!(!over_ten.eligible);
        assert!(
            over_ten
                .blockers
                .iter()
                .any(|blocker| { blocker.code == EligibilityBlockerCodeV1::ExtremeGroupImbalance })
        );

        let too_small = assess_multigroup_design_v1(&design_with_sizes(&[9, 20]));
        assert!(!too_small.eligible);
        assert!(too_small.blockers.iter().any(|blocker| {
            blocker.code == EligibilityBlockerCodeV1::InsufficientCompleteCases
        }));
    }

    #[test]
    fn eligibility_rejects_duplicate_source_row_tokens() {
        let mut design = design_with_sizes(&[10, 10]);
        design.rows[11].source_row = design.rows[0].source_row;
        let assessment = assess_multigroup_design_v1(&design);
        assert!(!assessment.eligible);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|blocker| { blocker.code == EligibilityBlockerCodeV1::DuplicateSourceRow })
        );
    }

    #[test]
    fn pairwise_partition_is_deterministic_size_preserving_and_reversal_coupled() {
        let design = design_with_sizes(&[10, 17, 20]);
        let rows = canonical_rows(&design);
        let first =
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap();
        let reversed = OrderedGroupPairV1::new(first.group_b, first.group_a).unwrap();
        let a = pairwise_partition(&rows, first, 42, 7);
        let b = pairwise_partition(&rows, first, 42, 7);
        let reverse = pairwise_partition(&rows, reversed, 42, 7);
        assert_eq!(a, b);
        assert_eq!(a, reverse);
        assert_eq!(
            rows_from_assignments(&a.assignments, first.group_a).len(),
            10
        );
        assert_eq!(
            rows_from_assignments(&a.assignments, first.group_b).len(),
            17
        );
        assert_eq!(a.assignments.len(), 27);
        assert_ne!(a.digest, pairwise_partition(&rows, first, 42, 8).digest);
    }

    #[test]
    fn shared_pairwise_plan_is_canonical_reproducible_and_tamper_evident() {
        let design = design_with_sizes(&[10, 17, 20]);
        let first =
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap();
        let reversed = OrderedGroupPairV1::new(first.group_b, first.group_a).unwrap();
        let plan = build_pairwise_partition_plan_v1(&design, first, 5_000, 42).unwrap();
        let reverse_plan = build_pairwise_partition_plan_v1(&design, reversed, 5_000, 42).unwrap();
        assert_eq!(plan, reverse_plan);
        let rows = canonical_rows(&design);
        validate_pairwise_partition_plan_for_rows_v1(&rows, reversed, 5_000, 42, &plan).unwrap();
        let forward = materialize_pairwise_partition_v1(&rows, first, &plan, 37).unwrap();
        let reverse = materialize_pairwise_partition_v1(&rows, reversed, &plan, 37).unwrap();
        assert_eq!(forward, reverse);
        assert_eq!(
            rows_for_group(&forward.assignments, first.group_a).len(),
            10
        );
        assert_eq!(
            rows_for_group(&forward.assignments, first.group_b).len(),
            17
        );

        let mut tampered = plan.clone();
        tampered.entries[37].partition_sha256 = "sha256:tampered".into();
        assert!(
            validate_pairwise_partition_plan_for_rows_v1(&rows, first, 5_000, 42, &tampered)
                .is_err()
        );
    }

    #[test]
    fn omnibus_partition_preserves_every_group_size() {
        let design = design_with_sizes(&[10, 12, 14, 16]);
        let rows = canonical_rows(&design);
        let partition = omnibus_partition(&rows, 4, 99, 3);
        for (index, expected) in [10usize, 12, 14, 16].into_iter().enumerate() {
            assert_eq!(
                rows_from_assignments(&partition.assignments, GroupIndexV1::new(index).unwrap())
                    .len(),
                expected
            );
        }
        assert_eq!(partition.assignments.len(), rows.len());
    }

    #[test]
    fn bootstrap_stream_is_group_separated_and_repeatable() {
        let source = (0..20u64).collect::<Vec<_>>();
        let group_zero = GroupIndexV1::new(0).unwrap();
        let group_one = GroupIndexV1::new(1).unwrap();
        let a = bootstrap_rows(&source, group_zero, 42, 12);
        let b = bootstrap_rows(&source, group_zero, 42, 12);
        let other_group = bootstrap_rows(&source, group_one, 42, 12);
        assert_eq!(a, b);
        assert_ne!(a, other_group);
        assert_eq!(a.len(), source.len());
        assert!(a.iter().all(|row| source.contains(row)));
    }

    #[test]
    fn parameter_vector_is_reordered_to_frozen_identity_and_rejects_family_changes() {
        let expected = vec![parameter("a"), parameter("b")];
        let ordered = checked_vector(
            &expected,
            ParameterVectorV1 {
                parameters: vec![
                    ParameterEstimateV1 {
                        parameter: parameter("b"),
                        estimate: 2.0,
                    },
                    ParameterEstimateV1 {
                        parameter: parameter("a"),
                        estimate: 1.0,
                    },
                ],
            },
        )
        .unwrap();
        assert_eq!(ordered, vec![1.0, 2.0]);
        let changed = ParameterVectorV1 {
            parameters: vec![
                ParameterEstimateV1 {
                    parameter: parameter("a"),
                    estimate: 1.0,
                },
                ParameterEstimateV1 {
                    parameter: ParameterIdentityV1 {
                        stable_id: "b".into(),
                        family: ParameterFamilyV1::OuterLoading,
                    },
                    estimate: 2.0,
                },
            ],
        };
        assert_eq!(
            checked_vector(&expected, changed).unwrap_err().code,
            RefitFailureCodeV1::ParameterContractMismatch
        );
    }

    #[test]
    fn pairwise_runner_retains_failed_indices_without_replacement() {
        let design = design_with_sizes(&[10, 10]);
        let pair =
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap();
        let parameters = vec![parameter("path:x->y")];
        let mut refitter = |request: &MultigroupFitRequestV1| {
            if request.sample_kind == FitSampleKindV1::PairwisePermutation
                && request.group == pair.group_b
                && request
                    .replicate
                    .is_some_and(|replicate| replicate % 10 == 0)
            {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::SingularModel,
                    "fixture failure",
                ));
            }
            let mean =
                request.source_rows.iter().sum::<u64>() as f64 / request.source_rows.len() as f64;
            Ok(vector("path:x->y", mean))
        };
        let result = run_pairwise_permutation_v1(
            &design,
            pair,
            &parameters,
            MultigroupResamplingConfigV1::official_defaults(),
            &mut refitter,
        )
        .unwrap();
        assert_eq!(result.requested, 5_000);
        assert_eq!(result.attempted, 5_000);
        assert_eq!(result.usable, 4_500);
        assert_eq!(result.failed, 500);
        assert_eq!(result.minimum_usable, 4_500);
        assert_eq!(result.availability, InferenceAvailabilityV1::Available);
        assert_eq!(result.ledger.len(), 5_000);
        assert_eq!(
            result
                .ledger
                .iter()
                .filter(|entry| entry.status == ResampleFitStatusV1::Failed)
                .count(),
            500
        );
        assert!(result.parameters[0].p_value_two_sided > 0.0);
    }

    #[test]
    fn omnibus_runner_retains_every_usable_null_spread_for_independent_tail_replay() {
        let design = design_with_sizes(&[10, 10, 10]);
        let parameters = vec![parameter("path:x->y")];
        let mut refitter = |request: &MultigroupFitRequestV1| {
            let mean =
                request.source_rows.iter().sum::<u64>() as f64 / request.source_rows.len() as f64;
            Ok(vector("path:x->y", mean))
        };
        let result = run_max_spread_omnibus_permutation_v1(
            &design,
            &parameters,
            MultigroupResamplingConfigV1::official_defaults(),
            &mut refitter,
        )
        .unwrap();
        let parameter = &result.parameters[0];
        assert_eq!(parameter.null_maximum_pairwise_spreads.len(), result.usable);
        assert!(
            parameter
                .null_maximum_pairwise_spreads
                .iter()
                .all(|spread| spread.is_finite() && *spread >= 0.0)
        );
        let extremes = parameter
            .null_maximum_pairwise_spreads
            .iter()
            .filter(|spread| **spread >= parameter.observed_maximum_pairwise_spread)
            .count();
        let replay = (extremes + 1) as f64 / (result.usable + 1) as f64;
        assert_eq!(parameter.p_value_right_tailed.to_bits(), replay.to_bits());
    }

    fn synthetic_banks() -> GroupBootstrapBanksV1 {
        let group_a = GroupIndexV1::new(0).unwrap();
        let group_b = GroupIndexV1::new(1).unwrap();
        GroupBootstrapBanksV1 {
            method_version: MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1.into(),
            parameters: vec![parameter("p")],
            seed: 42,
            requested: 4,
            attempted: 4,
            minimum_usable: 4,
            retry_policy: "none".into(),
            plan_sha256: "sha256:test".into(),
            availability: InferenceAvailabilityV1::Available,
            groups: vec![
                GroupBootstrapBankV1 {
                    group: group_a,
                    point_estimates: vec![2.0],
                    usable: 4,
                    failed: 0,
                    replicate_estimates: vec![
                        Some(vec![3.0]),
                        Some(vec![1.0]),
                        Some(vec![4.0]),
                        Some(vec![2.0]),
                    ],
                },
                GroupBootstrapBankV1 {
                    group: group_b,
                    point_estimates: vec![1.0],
                    usable: 4,
                    failed: 0,
                    replicate_estimates: vec![
                        Some(vec![1.0]),
                        Some(vec![2.0]),
                        Some(vec![4.0]),
                        Some(vec![0.0]),
                    ],
                },
            ],
            ledger: (0..4)
                .map(|replicate| BootstrapLedgerEntryV1 {
                    replicate,
                    status: ResampleFitStatusV1::Usable,
                    groups: Vec::new(),
                })
                .collect(),
            group_counts: vec![
                GroupEligibilitySummaryV1 {
                    group: group_a,
                    complete_cases: 10,
                },
                GroupEligibilitySummaryV1 {
                    group: group_b,
                    complete_cases: 10,
                },
            ],
            eligibility_warnings: Vec::new(),
        }
    }

    #[test]
    fn henseler_probability_is_directional_and_group_reversal_symmetric() {
        let banks = synthetic_banks();
        let forward_pair =
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap();
        let reverse_pair =
            OrderedGroupPairV1::new(forward_pair.group_b, forward_pair.group_a).unwrap();
        let forward = henseler_directional_probabilities_v1(&banks, forward_pair, 0.05)
            .unwrap()
            .remove(0);
        let reverse = henseler_directional_probabilities_v1(&banks, reverse_pair, 0.05)
            .unwrap()
            .remove(0);
        assert!(
            (forward.directional_probability_a_greater + reverse.directional_probability_a_greater
                - 1.0)
                .abs()
                < 1e-15
        );
        assert_eq!(
            forward.point_difference_a_minus_b,
            -reverse.point_difference_a_minus_b
        );
        assert_eq!(forward.matched_usable_replicates, 4);
        assert_eq!(forward.greater_differences, 2);
        assert_eq!(forward.equal_differences, 1);
        assert_eq!(forward.less_differences, 1);
        let ordinary_bc = group_bias_corrected_intervals_v1(&banks, 0.95).unwrap();

        let mut frequency_banks = banks;
        frequency_banks.method_version =
            crate::FREQUENCY_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1.into();
        assert_eq!(
            henseler_directional_probabilities_v1(&frequency_banks, forward_pair, 0.05)
                .unwrap()
                .remove(0),
            forward
        );
        assert_eq!(
            group_bias_corrected_intervals_v1(&frequency_banks, 0.95).unwrap(),
            ordinary_bc
        );
    }

    #[test]
    fn bc_interval_has_zero_acceleration_and_type7_order() {
        let interval =
            bias_corrected_interval_v1(0.0, &[-0.4, -0.2, -0.1, 0.0, 0.2, 0.5, 0.7], 0.95).unwrap();
        assert_eq!(interval.acceleration.to_bits(), 0.0f64.to_bits());
        assert!(interval.adjusted_lower_probability < interval.adjusted_upper_probability);
        assert!(interval.lower <= interval.upper);
        assert_eq!(
            interval.method_version,
            MGA_MULTIGROUP_BC_INTERVAL_VERSION_V1
        );
    }

    #[test]
    fn directional_bc_interval_publishes_only_the_selected_finite_bound() {
        let draws = [-0.4, -0.2, -0.1, 0.0, 0.2, 0.5, 0.7];
        let less = bias_corrected_interval_for_alternative_v1(
            0.0,
            &draws,
            0.95,
            AlternativeHypothesisV1::Less,
        )
        .unwrap();
        assert_eq!(less.lower, None);
        assert!(less.upper.is_some());
        assert_eq!(less.adjusted_lower_probability, None);
        assert!(less.adjusted_upper_probability.is_some());

        let greater = bias_corrected_interval_for_alternative_v1(
            0.0,
            &draws,
            0.95,
            AlternativeHypothesisV1::Greater,
        )
        .unwrap();
        assert!(greater.lower.is_some());
        assert_eq!(greater.upper, None);
        assert!(greater.adjusted_lower_probability.is_some());
        assert_eq!(greater.adjusted_upper_probability, None);
    }

    #[test]
    fn pooled_and_welch_tests_reverse_sign_but_preserve_two_sided_probability() {
        let a = ParametricGroupEstimateV1 {
            group: GroupIndexV1::new(0).unwrap(),
            estimate: 0.8,
            standard_error_method:
                ParametricGroupSeMethodV1::OrdinaryPlsScoreConditionalCenteredOls,
            standard_error: 0.10,
            observations: 50,
            predictor_count: 1,
            variance_degrees_of_freedom: 48.0,
            residual_sum_of_squares: 24.0,
            coefficient_variance_factor: 0.02,
        };
        let b = ParametricGroupEstimateV1 {
            group: GroupIndexV1::new(1).unwrap(),
            estimate: 0.5,
            standard_error_method:
                ParametricGroupSeMethodV1::OrdinaryPlsScoreConditionalCenteredOls,
            standard_error: 0.15,
            observations: 40,
            predictor_count: 1,
            variance_degrees_of_freedom: 38.0,
            residual_sum_of_squares: 34.2,
            coefficient_variance_factor: 0.025,
        };
        for test in [
            pooled_variance_parameter_test_v1(a, b, AlternativeHypothesisV1::TwoSided).unwrap(),
            welch_satterthwaite_parameter_test_v1(a, b, AlternativeHypothesisV1::TwoSided).unwrap(),
        ] {
            let reversed = match test.method {
                PairwiseParametricMethodV1::PooledEqualResidualVariance => {
                    pooled_variance_parameter_test_v1(b, a, AlternativeHypothesisV1::TwoSided)
                        .unwrap()
                }
                PairwiseParametricMethodV1::WelchSatterthwaite => {
                    welch_satterthwaite_parameter_test_v1(b, a, AlternativeHypothesisV1::TwoSided)
                        .unwrap()
                }
            };
            assert_eq!(test.t_statistic, -reversed.t_statistic);
            assert_eq!(
                test.standard_error_of_difference,
                reversed.standard_error_of_difference
            );
            assert_eq!(test.degrees_of_freedom, reversed.degrees_of_freedom);
            assert_eq!(test.p_value_two_sided, reversed.p_value_two_sided);
            assert_eq!(test.p_value_greater, reversed.p_value_less);
            assert_eq!(test.p_value_less, reversed.p_value_greater);
        }

        let mut impossible_df = a;
        impossible_df.variance_degrees_of_freedom = 49.0;
        assert!(matches!(
            pooled_variance_parameter_test_v1(impossible_df, b, AlternativeHypothesisV1::TwoSided),
            Err(MultigroupKernelErrorV1::InvalidParametricInput(_))
        ));
    }

    #[test]
    fn ordinary_pls_path_se_matches_centered_two_predictor_identity() {
        let x1 = vec![-2.0, -1.0, 0.0, 0.0, 1.0, 2.0];
        let x2 = vec![1.0, -1.0, -1.0, 1.0, -1.0, 1.0];
        let residual = [1.0, 1.0, -2.0, -2.0, 1.0, 1.0];
        let y = x1
            .iter()
            .zip(&x2)
            .zip(residual)
            .map(|((x1, x2), residual)| 7.0 + 0.5 * x1 - 0.25 * x2 + 0.1 * residual)
            .collect::<Vec<_>>();
        let scores = BTreeMap::from([
            ("x1".into(), x1),
            ("x2".into(), x2),
            ("y".into(), y.clone()),
        ]);
        let receipt = ordinary_pls_path_se_from_scores_v1(
            &scores,
            "x1",
            "y",
            &["x1".into(), "x2".into()],
            0.5,
            6,
        )
        .unwrap();
        assert_eq!(receipt.observations, 6);
        assert_eq!(receipt.predictor_count, 2);
        assert_eq!(receipt.variance_degrees_of_freedom, 3.0);
        assert!((receipt.residual_sum_of_squares - 0.12).abs() < 1e-12);
        assert!((receipt.coefficient_variance_factor - 0.1).abs() < 1e-12);
        assert!((receipt.standard_error - 0.004_f64.sqrt()).abs() < 1e-12);

        let reordered = ordinary_pls_path_se_from_scores_v1(
            &scores,
            "x1",
            "y",
            &["x2".into(), "x1".into()],
            0.5,
            6,
        )
        .unwrap();
        assert_eq!(receipt.standard_error, reordered.standard_error);
        assert_eq!(
            receipt.residual_sum_of_squares,
            reordered.residual_sum_of_squares
        );

        let exact_y = scores["x1"]
            .iter()
            .zip(&scores["x2"])
            .map(|(x1, x2)| 7.0 + 0.5 * x1 - 0.25 * x2)
            .collect::<Vec<_>>();
        let mut zero_residual_scores = scores;
        zero_residual_scores.insert("y".into(), exact_y);
        assert!(
            ordinary_pls_path_se_from_scores_v1(
                &zero_residual_scores,
                "x1",
                "y",
                &["x1".into(), "x2".into()],
                0.5,
                6,
            )
            .is_err()
        );
    }

    #[test]
    fn inverse_variance_wald_is_zero_for_identical_estimates() {
        let equal = inverse_variance_wald_test_v1(&[
            WaldGroupEstimateV1 {
                group: GroupIndexV1::new(0).unwrap(),
                estimate: 0.4,
                standard_error: 0.1,
            },
            WaldGroupEstimateV1 {
                group: GroupIndexV1::new(1).unwrap(),
                estimate: 0.4,
                standard_error: 0.2,
            },
            WaldGroupEstimateV1 {
                group: GroupIndexV1::new(2).unwrap(),
                estimate: 0.4,
                standard_error: 0.3,
            },
        ])
        .unwrap();
        assert!(equal.chi_square.abs() < 1e-15);
        assert!((equal.p_value_right_tailed - 1.0).abs() < 1e-15);
        assert_eq!(equal.degrees_of_freedom, 2);
    }

    #[test]
    fn multiplicity_adjustments_match_hand_calculated_examples() {
        let inputs = vec![
            HypothesisProbabilityV1 {
                hypothesis_id: "a".into(),
                raw_probability: 0.01,
            },
            HypothesisProbabilityV1 {
                hypothesis_id: "b".into(),
                raw_probability: 0.04,
            },
            HypothesisProbabilityV1 {
                hypothesis_id: "c".into(),
                raw_probability: 0.03,
            },
        ];
        let holm = adjust_probabilities_v1(&inputs, MultiplicityMethodV1::Holm).unwrap();
        assert!((holm[0].adjusted_probability - 0.03).abs() < 1e-15);
        assert!((holm[1].adjusted_probability - 0.06).abs() < 1e-15);
        assert!((holm[2].adjusted_probability - 0.06).abs() < 1e-15);
        let bh = adjust_probabilities_v1(&inputs, MultiplicityMethodV1::BenjaminiHochberg).unwrap();
        assert!((bh[0].adjusted_probability - 0.03).abs() < 1e-15);
        assert!((bh[1].adjusted_probability - 0.04).abs() < 1e-15);
        assert!((bh[2].adjusted_probability - 0.04).abs() < 1e-15);
        let sidak = adjust_probabilities_v1(&inputs, MultiplicityMethodV1::Sidak).unwrap();
        assert!((sidak[0].adjusted_probability - 0.029701).abs() < 1e-12);
    }

    #[test]
    fn minimum_usable_rule_is_ceiling_ninety_percent_with_floor_one_thousand() {
        assert_eq!(minimum_usable_resamples_v1(5_000), 4_500);
        assert_eq!(minimum_usable_resamples_v1(5_001), 4_501);
        assert_eq!(minimum_usable_resamples_v1(10_000), 9_000);
        assert_eq!(minimum_usable_resamples_v1(99), 1_000);
    }
}
