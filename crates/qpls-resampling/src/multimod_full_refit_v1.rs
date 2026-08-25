//! Additive MultiMod full-refit resampling receipts.
//!
//! This module owns deterministic draw generation, shard/resume identity,
//! no-retry failure receipts, and usable-ledger gates. It deliberately does
//! not own a conditional-process or causal estimator: the runner supplies one
//! full scientific refit callback and a SHA-256 identity covering its data,
//! compiled model, estimands, probes, and ordered target vector. A callback
//! failure remains a failed draw; this layer never substitutes an estimate or
//! changes the requested method.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use statrs::distribution::{Binomial, DiscreteCDF};
use std::collections::{BTreeMap, BTreeSet};

pub const MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1: &str = "qpls.multimod.full_refit_orchestrator.v1";
pub const MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1: u32 = 1;
pub const MULTIMOD_MINIMUM_USABLE_FRACTION_V1: f64 = 0.90;
pub const MULTIMOD_MAX_BOOTSTRAP_REPLICATES_V1: u32 = 10_000;
pub const MULTIMOD_MAX_STUDENTIZED_OUTER_REPLICATES_V1: u32 = 5_000;
pub const MULTIMOD_MAX_STUDENTIZED_INNER_REPLICATES_V1: u32 = 1_000;
pub const MULTIMOD_MAX_STUDENTIZED_INNER_REFITS_V1: u64 = 1_000_000;
pub const MULTIMOD_MAX_FREQUENCY_TOTAL_V1: u64 = 9_007_199_254_740_991;
pub const MULTIMOD_MAX_CASE_WEIGHT_RATIO_V1: f64 = 1.0e6;

const RNG_DOMAIN_V1: &[u8] = b"QuickPLS MultiMod full-refit orchestrator v1\0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModBootstrapPlanV1 {
    pub schema_version: u32,
    /// Runner-owned digest over the complete scientific refit identity,
    /// including the data, compiled model, estimands, probes, and target order.
    /// qpls-resampling cannot infer this identity from an opaque callback.
    pub scientific_refit_identity_sha256: String,
    pub requested_replicates: u32,
    pub master_seed: u64,
    pub minimum_usable_fraction: f64,
}

impl MultiModBootstrapPlanV1 {
    pub fn ensure_valid(&self) -> Result<(), MultiModFullRefitErrorV1> {
        if self.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1 {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(
                "bootstrap plan schema_version must equal 1".into(),
            ));
        }
        validate_scientific_refit_identity(&self.scientific_refit_identity_sha256)?;
        if self.requested_replicates == 0
            || self.requested_replicates > MULTIMOD_MAX_BOOTSTRAP_REPLICATES_V1
        {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(format!(
                "bootstrap replicates must be between 1 and {MULTIMOD_MAX_BOOTSTRAP_REPLICATES_V1}"
            )));
        }
        validate_usable_fraction(self.minimum_usable_fraction)
    }

    pub fn identity_sha256(&self) -> Result<String, MultiModFullRefitErrorV1> {
        self.ensure_valid()?;
        sha256_json(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModStudentizedPlanV1 {
    pub schema_version: u32,
    /// Runner-owned digest over the complete scientific refit identity,
    /// including the data, compiled model, estimands, probes, and target order.
    pub scientific_refit_identity_sha256: String,
    pub outer_replicates: u32,
    pub inner_replicates: u32,
    pub master_seed: u64,
    pub minimum_outer_usable_fraction: f64,
    pub minimum_inner_usable_fraction: f64,
}

impl MultiModStudentizedPlanV1 {
    pub fn ensure_valid(&self) -> Result<(), MultiModFullRefitErrorV1> {
        if self.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1 {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(
                "studentized plan schema_version must equal 1".into(),
            ));
        }
        validate_scientific_refit_identity(&self.scientific_refit_identity_sha256)?;
        if self.outer_replicates == 0
            || self.outer_replicates > MULTIMOD_MAX_STUDENTIZED_OUTER_REPLICATES_V1
        {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(format!(
                "studentized outer replicates must be between 1 and {MULTIMOD_MAX_STUDENTIZED_OUTER_REPLICATES_V1}"
            )));
        }
        if self.inner_replicates == 0
            || self.inner_replicates > MULTIMOD_MAX_STUDENTIZED_INNER_REPLICATES_V1
        {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(format!(
                "studentized inner replicates must be between 1 and {MULTIMOD_MAX_STUDENTIZED_INNER_REPLICATES_V1}"
            )));
        }
        if u64::from(self.outer_replicates) * u64::from(self.inner_replicates)
            > MULTIMOD_MAX_STUDENTIZED_INNER_REFITS_V1
        {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(format!(
                "studentized nested refits cannot exceed {MULTIMOD_MAX_STUDENTIZED_INNER_REFITS_V1}"
            )));
        }
        validate_usable_fraction(self.minimum_outer_usable_fraction)?;
        validate_usable_fraction(self.minimum_inner_usable_fraction)
    }

    pub fn identity_sha256(&self) -> Result<String, MultiModFullRefitErrorV1> {
        self.ensure_valid()?;
        sha256_json(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModJackknifePlanV1 {
    pub schema_version: u32,
    /// Runner-owned digest over the complete scientific refit identity,
    /// including the data, compiled model, estimands, probes, and target order.
    pub scientific_refit_identity_sha256: String,
}

impl MultiModJackknifePlanV1 {
    pub fn ensure_valid(&self) -> Result<(), MultiModFullRefitErrorV1> {
        if self.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1 {
            return Err(MultiModFullRefitErrorV1::InvalidPlan(
                "jackknife plan schema_version must equal 1".into(),
            ));
        }
        validate_scientific_refit_identity(&self.scientific_refit_identity_sha256)
    }

    pub fn identity_sha256(&self) -> Result<String, MultiModFullRefitErrorV1> {
        self.ensure_valid()?;
        sha256_json(self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModShardSpecV1 {
    pub shard_index: u32,
    pub shard_count: u32,
}

impl MultiModShardSpecV1 {
    fn ensure_valid(self, work_items: u32) -> Result<(), MultiModFullRefitErrorV1> {
        if self.shard_count == 0
            || self.shard_index >= self.shard_count
            || self.shard_count > work_items
        {
            return Err(MultiModFullRefitErrorV1::InvalidShard {
                shard_index: self.shard_index,
                shard_count: self.shard_count,
                work_items,
            });
        }
        Ok(())
    }

    fn owns(self, index: u32) -> bool {
        index % self.shard_count == self.shard_index
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiModFullRefitPhaseV1 {
    CaseBootstrap,
    FrequencyBootstrap,
    DeleteOneJackknife,
    StudentizedOuter,
    StudentizedInner,
}

impl MultiModFullRefitPhaseV1 {
    const fn as_str(self) -> &'static str {
        match self {
            Self::CaseBootstrap => "case_bootstrap",
            Self::FrequencyBootstrap => "frequency_bootstrap",
            Self::DeleteOneJackknife => "delete_one_jackknife",
            Self::StudentizedOuter => "studentized_outer",
            Self::StudentizedInner => "studentized_inner",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModShardIdentityV1 {
    pub schema_version: u32,
    pub method_version: String,
    pub phase: MultiModFullRefitPhaseV1,
    pub execution_identity_sha256: String,
    pub shard_index: u32,
    pub shard_count: u32,
    pub shard_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModCaseBootstrapDrawV1 {
    pub replicate_index: u32,
    /// Source-row indices in sampled-position order. Duplicate indices are
    /// intentional and preserve ordinary case-bootstrap semantics.
    pub source_rows: Vec<u32>,
    /// Positive weights selected with `source_rows` and normalized to mean one
    /// for this refit. `None` means the unweighted profile.
    pub case_weights: Option<Vec<f64>>,
    pub source_rows_sha256: String,
    pub case_weights_sha256: Option<String>,
    pub draw_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModDeleteOneJackknifeDrawV1 {
    pub omitted_row: u32,
    pub retained_source_rows: Vec<u32>,
    pub case_weights: Option<Vec<f64>>,
    pub retained_rows_sha256: String,
    pub case_weights_sha256: Option<String>,
    pub draw_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModFrequencyBootstrapDrawV1 {
    pub replicate_index: u32,
    /// One multinomial count per original row. No expanded row vector is
    /// created, even when the frequency total is large.
    pub counts: Vec<u64>,
    pub total_count: u64,
    pub counts_sha256: String,
    pub draw_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModStudentizedInnerDrawV1 {
    pub outer_replicate_index: u32,
    pub inner_replicate_index: u32,
    pub outer_draw_identity_sha256: String,
    /// Original source rows reached by resampling positions within the frozen
    /// outer draw. This is a full nested refit, not an analytical shortcut.
    pub source_rows: Vec<u32>,
    pub case_weights: Option<Vec<f64>>,
    pub source_rows_sha256: String,
    pub case_weights_sha256: Option<String>,
    pub draw_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModRefitFailureV1 {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum MultiModRefitOutcomeV1<T> {
    Success {
        value: T,
        value_sha256: String,
    },
    Failed {
        failure: MultiModRefitFailureV1,
        failure_sha256: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModRefitRecordV1<Draw, Estimate> {
    pub index: u32,
    /// Every completed record has exactly one scientific refit attempt.
    pub attempt_count: u8,
    pub draw: Draw,
    pub outcome: MultiModRefitOutcomeV1<Estimate>,
    /// Binds the index, one-attempt receipt, deterministic draw, and outcome so
    /// a valid outcome cannot be moved to another replicate during resume.
    pub record_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModShardCacheV1<Draw, Estimate> {
    pub schema_version: u32,
    pub shard: MultiModShardIdentityV1,
    pub cancelled: bool,
    pub records: Vec<MultiModRefitRecordV1<Draw, Estimate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate> {
    pub outer: MultiModRefitRecordV1<MultiModCaseBootstrapDrawV1, OuterEstimate>,
    pub inner_ledger_identity_sha256: String,
    pub inner_records: Vec<MultiModRefitRecordV1<MultiModStudentizedInnerDrawV1, InnerEstimate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate> {
    pub schema_version: u32,
    pub shard: MultiModShardIdentityV1,
    pub cancelled: bool,
    pub records: Vec<MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModFinalLedgerV1<Draw, Estimate> {
    pub schema_version: u32,
    pub method_version: String,
    pub execution_identity_sha256: String,
    pub requested: u32,
    pub usable: u32,
    pub usable_indices: Vec<u32>,
    pub minimum_required: u32,
    pub usable_fraction: f64,
    pub complete: bool,
    pub records: Vec<MultiModRefitRecordV1<Draw, Estimate>>,
    pub ledger_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultiModStudentizedFinalLedgerV1<OuterEstimate, InnerEstimate> {
    pub schema_version: u32,
    pub method_version: String,
    pub execution_identity_sha256: String,
    pub requested_outer: u32,
    pub usable_outer: u32,
    /// Outer replicates whose outer fit succeeded and whose complete inner
    /// ledger independently met the configured usable gate.
    pub usable_outer_indices: Vec<u32>,
    pub minimum_outer_required: u32,
    pub requested_inner_per_outer: u32,
    pub minimum_inner_required: u32,
    pub complete: bool,
    pub records: Vec<MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate>>,
    pub ledger_sha256: String,
}

/// Scientific estimation remains outside qpls-resampling. Implementors must
/// perform the complete requested point fit for the supplied draw and return a
/// stable failure rather than simplifying the model or substituting a fallback.
pub trait MultiModFullRefitCallbackV1<Draw, Estimate> {
    fn full_refit(&mut self, draw: &Draw) -> Result<Estimate, MultiModRefitFailureV1>;
}

impl<Draw, Estimate, F> MultiModFullRefitCallbackV1<Draw, Estimate> for F
where
    F: FnMut(&Draw) -> Result<Estimate, MultiModRefitFailureV1>,
{
    fn full_refit(&mut self, draw: &Draw) -> Result<Estimate, MultiModRefitFailureV1> {
        self(draw)
    }
}

/// Additive control result for refits whose estimator can observe cancellation
/// while one draw is in flight. An interruption is execution state, not a
/// scientific outcome, so it never becomes an immutable ledger record.
#[derive(Debug, Clone, PartialEq)]
pub enum MultiModRefitAttemptV1<Estimate> {
    Completed(Result<Estimate, MultiModRefitFailureV1>),
    Interrupted,
}

/// Interruptible counterpart to `MultiModFullRefitCallbackV1`. Existing V1
/// callbacks retain their exact signature and no-retry behavior.
pub trait MultiModInterruptibleFullRefitCallbackV1<Draw, Estimate> {
    fn full_refit_attempt(&mut self, draw: &Draw) -> MultiModRefitAttemptV1<Estimate>;
}

impl<Draw, Estimate, F> MultiModInterruptibleFullRefitCallbackV1<Draw, Estimate> for F
where
    F: FnMut(&Draw) -> MultiModRefitAttemptV1<Estimate>,
{
    fn full_refit_attempt(&mut self, draw: &Draw) -> MultiModRefitAttemptV1<Estimate> {
        self(draw)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MultiModFullRefitErrorV1 {
    #[error("invalid MultiMod full-refit plan: {0}")]
    InvalidPlan(String),
    #[error(
        "invalid shard {shard_index}/{shard_count} for an execution with {work_items} work items"
    )]
    InvalidShard {
        shard_index: u32,
        shard_count: u32,
        work_items: u32,
    },
    #[error("case count must be between {minimum} and u32::MAX (found {actual})")]
    InvalidCaseCount { minimum: usize, actual: usize },
    #[error("invalid positive case-weight contract: {0}")]
    InvalidCaseWeights(String),
    #[error("invalid positive-integer frequency contract: {0}")]
    InvalidFrequencyWeights(String),
    #[error("resume cache identity differs from the requested deterministic shard")]
    ResumeIdentityMismatch,
    #[error("cancelled shard {0} must be resumed before scientific finalization")]
    CancelledShardNotResumed(u32),
    #[error("resume/final ledger contains duplicate record {0}")]
    DuplicateRecord(u32),
    #[error("record {0} does not belong to the declared shard")]
    RecordOutsideShard(u32),
    #[error("record {0} draw identity differs from deterministic regeneration")]
    DrawIdentityMismatch(u32),
    #[error("record {0} outcome digest or failure identity is invalid")]
    OutcomeIdentityMismatch(u32),
    #[error("record {0} identity does not bind its draw and outcome")]
    RecordIdentityMismatch(u32),
    #[error("callback returned an invalid stable failure for record {0}")]
    InvalidCallbackFailure(u32),
    #[error("finalization requires {expected} records but found {actual}")]
    IncompleteLedger { expected: u32, actual: usize },
    #[error("usable-refit gate failed: {usable} usable, {minimum_required} required")]
    UsableGateFailed { usable: u32, minimum_required: u32 },
    #[error("full delete-one jackknife requires all {required} successful refits (found {usable})")]
    IncompleteJackknife { usable: u32, required: u32 },
    #[error("studentized outer record {outer_index} has an invalid inner ledger: {reason}")]
    InvalidInnerLedger { outer_index: u32, reason: String },
    #[error("deterministic JSON identity serialization failed: {0}")]
    Serialization(String),
    #[error("count-space multinomial generation failed: {0}")]
    CountSpace(String),
}

pub fn run_multimod_case_bootstrap_shard_v1<Estimate, Callback, Cancelled>(
    plan: &MultiModBootstrapPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
    resume: Option<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>>,
    callback: &mut Callback,
    is_cancelled: Cancelled,
) -> Result<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>, MultiModFullRefitErrorV1>
where
    Estimate: Serialize,
    Callback: MultiModFullRefitCallbackV1<MultiModCaseBootstrapDrawV1, Estimate>,
    Cancelled: FnMut() -> bool,
{
    run_multimod_case_bootstrap_shard_with_attempt_v1(
        plan,
        case_count,
        case_weights,
        shard,
        resume,
        |draw| MultiModRefitAttemptV1::Completed(callback.full_refit(draw)),
        is_cancelled,
    )
}

/// Runs one deterministic case-bootstrap shard with in-flight interruption.
///
/// `Interrupted` returns a resumable cache with `cancelled=true` and omits the
/// interrupted index entirely. A completed stable failure is retained exactly
/// once and is never retried, matching the ordinary V1 callback contract.
pub fn run_multimod_case_bootstrap_shard_interruptible_v1<Estimate, Callback, Cancelled>(
    plan: &MultiModBootstrapPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
    resume: Option<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>>,
    callback: &mut Callback,
    is_cancelled: Cancelled,
) -> Result<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>, MultiModFullRefitErrorV1>
where
    Estimate: Serialize,
    Callback: MultiModInterruptibleFullRefitCallbackV1<MultiModCaseBootstrapDrawV1, Estimate>,
    Cancelled: FnMut() -> bool,
{
    run_multimod_case_bootstrap_shard_with_attempt_v1(
        plan,
        case_count,
        case_weights,
        shard,
        resume,
        |draw| callback.full_refit_attempt(draw),
        is_cancelled,
    )
}

fn run_multimod_case_bootstrap_shard_with_attempt_v1<Estimate, Attempt, Cancelled>(
    plan: &MultiModBootstrapPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
    resume: Option<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>>,
    mut attempt: Attempt,
    mut is_cancelled: Cancelled,
) -> Result<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>, MultiModFullRefitErrorV1>
where
    Estimate: Serialize,
    Attempt: FnMut(&MultiModCaseBootstrapDrawV1) -> MultiModRefitAttemptV1<Estimate>,
    Cancelled: FnMut() -> bool,
{
    plan.ensure_valid()?;
    validate_case_input(case_count, case_weights, 2)?;
    shard.ensure_valid(plan.requested_replicates)?;
    let execution_identity = case_execution_identity(
        plan,
        MultiModFullRefitPhaseV1::CaseBootstrap,
        case_count,
        case_weights,
    )?;
    let shard_identity = make_shard_identity(
        MultiModFullRefitPhaseV1::CaseBootstrap,
        &execution_identity,
        shard,
    )?;
    let mut records = resume_records(resume, &shard_identity, |record| {
        validate_case_record(
            record,
            plan,
            &execution_identity,
            case_count,
            case_weights,
            shard,
        )
    })?;

    for replicate_index in 0..plan.requested_replicates {
        if !shard.owns(replicate_index) || records.contains_key(&replicate_index) {
            continue;
        }
        if is_cancelled() {
            return Ok(shard_cache(shard_identity, true, records));
        }
        let draw = make_case_draw(
            plan,
            &execution_identity,
            case_count,
            case_weights,
            replicate_index,
            MultiModFullRefitPhaseV1::CaseBootstrap,
        )?;
        match attempt(&draw) {
            MultiModRefitAttemptV1::Completed(result) => {
                records.insert(replicate_index, make_record(replicate_index, draw, result)?);
            }
            MultiModRefitAttemptV1::Interrupted => {
                return Ok(shard_cache(shard_identity, true, records));
            }
        }
    }
    Ok(shard_cache(shard_identity, false, records))
}

pub fn finalize_multimod_case_bootstrap_v1<Estimate>(
    plan: &MultiModBootstrapPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shards: Vec<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, Estimate>>,
) -> Result<MultiModFinalLedgerV1<MultiModCaseBootstrapDrawV1, Estimate>, MultiModFullRefitErrorV1>
where
    Estimate: Serialize,
{
    plan.ensure_valid()?;
    validate_case_input(case_count, case_weights, 2)?;
    let execution_identity = case_execution_identity(
        plan,
        MultiModFullRefitPhaseV1::CaseBootstrap,
        case_count,
        case_weights,
    )?;
    let records = collect_complete_shards(
        shards,
        MultiModFullRefitPhaseV1::CaseBootstrap,
        &execution_identity,
        plan.requested_replicates,
        |record, shard| {
            validate_case_record(
                record,
                plan,
                &execution_identity,
                case_count,
                case_weights,
                shard,
            )
        },
    )?;
    finalize_ledger(
        &execution_identity,
        plan.requested_replicates,
        plan.minimum_usable_fraction,
        records,
        false,
    )
}

pub fn run_multimod_frequency_bootstrap_shard_v1<Estimate, Callback, Cancelled>(
    plan: &MultiModBootstrapPlanV1,
    frequencies: &[u64],
    shard: MultiModShardSpecV1,
    resume: Option<MultiModShardCacheV1<MultiModFrequencyBootstrapDrawV1, Estimate>>,
    callback: &mut Callback,
    mut is_cancelled: Cancelled,
) -> Result<
    MultiModShardCacheV1<MultiModFrequencyBootstrapDrawV1, Estimate>,
    MultiModFullRefitErrorV1,
>
where
    Estimate: Serialize,
    Callback: MultiModFullRefitCallbackV1<MultiModFrequencyBootstrapDrawV1, Estimate>,
    Cancelled: FnMut() -> bool,
{
    plan.ensure_valid()?;
    validate_frequency_weights_v1(frequencies)?;
    shard.ensure_valid(plan.requested_replicates)?;
    let execution_identity = frequency_execution_identity(plan, frequencies)?;
    let shard_identity = make_shard_identity(
        MultiModFullRefitPhaseV1::FrequencyBootstrap,
        &execution_identity,
        shard,
    )?;
    let mut records = resume_records(resume, &shard_identity, |record| {
        validate_frequency_record(record, plan, &execution_identity, frequencies, shard)
    })?;

    for replicate_index in 0..plan.requested_replicates {
        if !shard.owns(replicate_index) || records.contains_key(&replicate_index) {
            continue;
        }
        if is_cancelled() {
            return Ok(shard_cache(shard_identity, true, records));
        }
        let draw = make_frequency_draw(plan, &execution_identity, frequencies, replicate_index)?;
        let result = callback.full_refit(&draw);
        records.insert(replicate_index, make_record(replicate_index, draw, result)?);
    }
    Ok(shard_cache(shard_identity, false, records))
}

pub fn finalize_multimod_frequency_bootstrap_v1<Estimate>(
    plan: &MultiModBootstrapPlanV1,
    frequencies: &[u64],
    shards: Vec<MultiModShardCacheV1<MultiModFrequencyBootstrapDrawV1, Estimate>>,
) -> Result<
    MultiModFinalLedgerV1<MultiModFrequencyBootstrapDrawV1, Estimate>,
    MultiModFullRefitErrorV1,
>
where
    Estimate: Serialize,
{
    plan.ensure_valid()?;
    validate_frequency_weights_v1(frequencies)?;
    let execution_identity = frequency_execution_identity(plan, frequencies)?;
    let records = collect_complete_shards(
        shards,
        MultiModFullRefitPhaseV1::FrequencyBootstrap,
        &execution_identity,
        plan.requested_replicates,
        |record, shard| {
            validate_frequency_record(record, plan, &execution_identity, frequencies, shard)
        },
    )?;
    finalize_ledger(
        &execution_identity,
        plan.requested_replicates,
        plan.minimum_usable_fraction,
        records,
        false,
    )
}

pub fn run_multimod_delete_one_jackknife_shard_v1<Estimate, Callback, Cancelled>(
    plan: &MultiModJackknifePlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
    resume: Option<MultiModShardCacheV1<MultiModDeleteOneJackknifeDrawV1, Estimate>>,
    callback: &mut Callback,
    mut is_cancelled: Cancelled,
) -> Result<
    MultiModShardCacheV1<MultiModDeleteOneJackknifeDrawV1, Estimate>,
    MultiModFullRefitErrorV1,
>
where
    Estimate: Serialize,
    Callback: MultiModFullRefitCallbackV1<MultiModDeleteOneJackknifeDrawV1, Estimate>,
    Cancelled: FnMut() -> bool,
{
    plan.ensure_valid()?;
    validate_case_input(case_count, case_weights, 3)?;
    let work_items =
        u32::try_from(case_count).map_err(|_| MultiModFullRefitErrorV1::InvalidCaseCount {
            minimum: 3,
            actual: case_count,
        })?;
    shard.ensure_valid(work_items)?;
    let execution_identity = jackknife_execution_identity(plan, case_count, case_weights)?;
    let shard_identity = make_shard_identity(
        MultiModFullRefitPhaseV1::DeleteOneJackknife,
        &execution_identity,
        shard,
    )?;
    let mut records = resume_records(resume, &shard_identity, |record| {
        validate_jackknife_record(record, &execution_identity, case_count, case_weights, shard)
    })?;

    for omitted_row in 0..work_items {
        if !shard.owns(omitted_row) || records.contains_key(&omitted_row) {
            continue;
        }
        if is_cancelled() {
            return Ok(shard_cache(shard_identity, true, records));
        }
        let draw = make_jackknife_draw(&execution_identity, case_count, case_weights, omitted_row)?;
        let result = callback.full_refit(&draw);
        records.insert(omitted_row, make_record(omitted_row, draw, result)?);
    }
    Ok(shard_cache(shard_identity, false, records))
}

pub fn finalize_multimod_delete_one_jackknife_v1<Estimate>(
    plan: &MultiModJackknifePlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shards: Vec<MultiModShardCacheV1<MultiModDeleteOneJackknifeDrawV1, Estimate>>,
) -> Result<
    MultiModFinalLedgerV1<MultiModDeleteOneJackknifeDrawV1, Estimate>,
    MultiModFullRefitErrorV1,
>
where
    Estimate: Serialize,
{
    plan.ensure_valid()?;
    validate_case_input(case_count, case_weights, 3)?;
    let requested =
        u32::try_from(case_count).map_err(|_| MultiModFullRefitErrorV1::InvalidCaseCount {
            minimum: 3,
            actual: case_count,
        })?;
    let execution_identity = jackknife_execution_identity(plan, case_count, case_weights)?;
    let records = collect_complete_shards(
        shards,
        MultiModFullRefitPhaseV1::DeleteOneJackknife,
        &execution_identity,
        requested,
        |record, shard| {
            validate_jackknife_record(record, &execution_identity, case_count, case_weights, shard)
        },
    )?;
    finalize_ledger(&execution_identity, requested, 1.0, records, true)
}

pub fn run_multimod_studentized_shard_v1<
    OuterEstimate,
    InnerEstimate,
    OuterCallback,
    InnerCallback,
    Cancelled,
>(
    plan: &MultiModStudentizedPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
    resume: Option<MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate>>,
    outer_callback: &mut OuterCallback,
    inner_callback: &mut InnerCallback,
    mut is_cancelled: Cancelled,
) -> Result<MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate>, MultiModFullRefitErrorV1>
where
    OuterEstimate: Serialize,
    InnerEstimate: Serialize,
    OuterCallback: MultiModFullRefitCallbackV1<MultiModCaseBootstrapDrawV1, OuterEstimate>,
    InnerCallback: MultiModFullRefitCallbackV1<MultiModStudentizedInnerDrawV1, InnerEstimate>,
    Cancelled: FnMut() -> bool,
{
    plan.ensure_valid()?;
    validate_case_input(case_count, case_weights, 2)?;
    shard.ensure_valid(plan.outer_replicates)?;
    let execution_identity = studentized_execution_identity(plan, case_count, case_weights)?;
    let shard_identity = make_shard_identity(
        MultiModFullRefitPhaseV1::StudentizedOuter,
        &execution_identity,
        shard,
    )?;
    let mut records = resume_studentized_records(
        resume,
        &shard_identity,
        plan,
        &execution_identity,
        case_count,
        case_weights,
        shard,
    )?;

    for outer_index in 0..plan.outer_replicates {
        if !shard.owns(outer_index) {
            continue;
        }
        if !records.contains_key(&outer_index) {
            if is_cancelled() {
                return Ok(studentized_shard_cache(shard_identity, true, records));
            }
            let outer_draw = make_studentized_outer_draw(
                plan,
                &execution_identity,
                case_count,
                case_weights,
                outer_index,
            )?;
            let outer_result = outer_callback.full_refit(&outer_draw);
            records.insert(
                outer_index,
                MultiModStudentizedOuterRecordV1 {
                    outer: make_record(outer_index, outer_draw, outer_result)?,
                    inner_ledger_identity_sha256: inner_ledger_identity(
                        &execution_identity,
                        outer_index,
                    )?,
                    inner_records: Vec::new(),
                },
            );
        }

        let outer_record = records.get_mut(&outer_index).ok_or_else(|| {
            MultiModFullRefitErrorV1::InvalidInnerLedger {
                outer_index,
                reason: "outer record disappeared before nested refits".into(),
            }
        })?;
        if matches!(
            outer_record.outer.outcome,
            MultiModRefitOutcomeV1::Failed { .. }
        ) {
            continue;
        }
        let mut inner_by_index = std::mem::take(&mut outer_record.inner_records)
            .into_iter()
            .map(|record| (record.index, record))
            .collect::<BTreeMap<_, _>>();
        for inner_index in 0..plan.inner_replicates {
            if inner_by_index.contains_key(&inner_index) {
                continue;
            }
            if is_cancelled() {
                outer_record.inner_records = inner_by_index.into_values().collect();
                return Ok(studentized_shard_cache(shard_identity, true, records));
            }
            let inner_draw = make_studentized_inner_draw(
                plan,
                &execution_identity,
                &outer_record.outer.draw,
                inner_index,
            )?;
            let inner_result = inner_callback.full_refit(&inner_draw);
            inner_by_index.insert(
                inner_index,
                make_record(inner_index, inner_draw, inner_result)?,
            );
        }
        outer_record.inner_records = inner_by_index.into_values().collect();
    }
    Ok(studentized_shard_cache(shard_identity, false, records))
}

pub fn finalize_multimod_studentized_v1<OuterEstimate, InnerEstimate>(
    plan: &MultiModStudentizedPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shards: Vec<MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate>>,
) -> Result<MultiModStudentizedFinalLedgerV1<OuterEstimate, InnerEstimate>, MultiModFullRefitErrorV1>
where
    OuterEstimate: Serialize,
    InnerEstimate: Serialize,
{
    plan.ensure_valid()?;
    validate_case_input(case_count, case_weights, 2)?;
    let execution_identity = studentized_execution_identity(plan, case_count, case_weights)?;
    let records = collect_complete_studentized_shards(
        shards,
        plan,
        &execution_identity,
        case_count,
        case_weights,
    )?;
    let minimum_inner = minimum_required(plan.inner_replicates, plan.minimum_inner_usable_fraction);
    let usable_outer_indices = records
        .iter()
        .filter(|record| {
            matches!(record.outer.outcome, MultiModRefitOutcomeV1::Success { .. })
                && record
                    .inner_records
                    .iter()
                    .filter(|inner| matches!(inner.outcome, MultiModRefitOutcomeV1::Success { .. }))
                    .count()
                    >= minimum_inner as usize
        })
        .map(|record| record.outer.index)
        .collect::<Vec<_>>();
    let usable_outer = usable_outer_indices.len() as u32;
    let minimum_outer = minimum_required(plan.outer_replicates, plan.minimum_outer_usable_fraction);
    if usable_outer < minimum_outer {
        return Err(MultiModFullRefitErrorV1::UsableGateFailed {
            usable: usable_outer,
            minimum_required: minimum_outer,
        });
    }
    #[derive(Serialize)]
    struct LedgerIdentity<'a, O, I> {
        method_version: &'static str,
        execution_identity_sha256: &'a str,
        requested_outer: u32,
        usable_outer: u32,
        usable_outer_indices: &'a [u32],
        minimum_outer_required: u32,
        requested_inner_per_outer: u32,
        minimum_inner_required: u32,
        records: &'a [MultiModStudentizedOuterRecordV1<O, I>],
    }
    let ledger_sha256 = sha256_json(&LedgerIdentity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        execution_identity_sha256: &execution_identity,
        requested_outer: plan.outer_replicates,
        usable_outer,
        usable_outer_indices: &usable_outer_indices,
        minimum_outer_required: minimum_outer,
        requested_inner_per_outer: plan.inner_replicates,
        minimum_inner_required: minimum_inner,
        records: &records,
    })?;
    Ok(MultiModStudentizedFinalLedgerV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1.into(),
        execution_identity_sha256: execution_identity,
        requested_outer: plan.outer_replicates,
        usable_outer,
        usable_outer_indices,
        minimum_outer_required: minimum_outer,
        requested_inner_per_outer: plan.inner_replicates,
        minimum_inner_required: minimum_inner,
        complete: true,
        records,
        ledger_sha256,
    })
}

pub fn validate_frequency_weights_v1(frequencies: &[u64]) -> Result<u64, MultiModFullRefitErrorV1> {
    if frequencies.len() < 2 {
        return Err(MultiModFullRefitErrorV1::InvalidFrequencyWeights(
            "at least two positive frequency rows are required".into(),
        ));
    }
    if frequencies.iter().any(|frequency| *frequency == 0) {
        return Err(MultiModFullRefitErrorV1::InvalidFrequencyWeights(
            "every frequency must be an integer greater than or equal to one".into(),
        ));
    }
    let total = frequencies.iter().try_fold(0u64, |sum, frequency| {
        sum.checked_add(*frequency).ok_or_else(|| {
            MultiModFullRefitErrorV1::InvalidFrequencyWeights(
                "frequency total overflowed u64".into(),
            )
        })
    })?;
    if total > MULTIMOD_MAX_FREQUENCY_TOTAL_V1 {
        return Err(MultiModFullRefitErrorV1::InvalidFrequencyWeights(format!(
            "frequency total exceeds {MULTIMOD_MAX_FREQUENCY_TOTAL_V1}"
        )));
    }
    Ok(total)
}

fn validate_scientific_refit_identity(
    scientific_refit_identity_sha256: &str,
) -> Result<(), MultiModFullRefitErrorV1> {
    if !is_sha256(scientific_refit_identity_sha256) {
        return Err(MultiModFullRefitErrorV1::InvalidPlan(
            "scientific_refit_identity_sha256 must be a lowercase SHA-256 digest over the runner-owned data, compiled model, estimands, probes, and target order"
                .into(),
        ));
    }
    Ok(())
}

fn validate_usable_fraction(value: f64) -> Result<(), MultiModFullRefitErrorV1> {
    if !value.is_finite() || !(MULTIMOD_MINIMUM_USABLE_FRACTION_V1..=1.0).contains(&value) {
        Err(MultiModFullRefitErrorV1::InvalidPlan(format!(
            "minimum usable fraction must be between {MULTIMOD_MINIMUM_USABLE_FRACTION_V1} and 1"
        )))
    } else {
        Ok(())
    }
}

fn validate_case_input(
    case_count: usize,
    case_weights: Option<&[f64]>,
    minimum: usize,
) -> Result<(), MultiModFullRefitErrorV1> {
    if case_count < minimum || u32::try_from(case_count).is_err() {
        return Err(MultiModFullRefitErrorV1::InvalidCaseCount {
            minimum,
            actual: case_count,
        });
    }
    let Some(weights) = case_weights else {
        return Ok(());
    };
    if weights.len() != case_count {
        return Err(MultiModFullRefitErrorV1::InvalidCaseWeights(
            "weight vector length differs from the row count".into(),
        ));
    }
    if weights
        .iter()
        .any(|weight| !weight.is_finite() || *weight <= 0.0)
    {
        return Err(MultiModFullRefitErrorV1::InvalidCaseWeights(
            "case weights must be finite and strictly positive".into(),
        ));
    }
    let minimum_weight = weights.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum_weight = weights.iter().copied().fold(0.0, f64::max);
    if maximum_weight / minimum_weight > MULTIMOD_MAX_CASE_WEIGHT_RATIO_V1 {
        return Err(MultiModFullRefitErrorV1::InvalidCaseWeights(format!(
            "case-weight max/min ratio exceeds {MULTIMOD_MAX_CASE_WEIGHT_RATIO_V1}"
        )));
    }
    let total = weights.iter().sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err(MultiModFullRefitErrorV1::InvalidCaseWeights(
            "case-weight sum is nonfinite or nonpositive".into(),
        ));
    }
    Ok(())
}

fn minimum_required(requested: u32, fraction: f64) -> u32 {
    (f64::from(requested) * fraction).ceil() as u32
}

fn case_execution_identity(
    plan: &MultiModBootstrapPlanV1,
    phase: MultiModFullRefitPhaseV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
) -> Result<String, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        method_version: &'static str,
        phase: MultiModFullRefitPhaseV1,
        plan_sha256: String,
        case_count: usize,
        case_weights_sha256: Option<String>,
        scientific_refit_identity_sha256: &'a str,
    }
    sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        phase,
        plan_sha256: plan.identity_sha256()?,
        case_count,
        case_weights_sha256: case_weights.map(sha256_f64_slice),
        scientific_refit_identity_sha256: &plan.scientific_refit_identity_sha256,
    })
}

fn jackknife_execution_identity(
    plan: &MultiModJackknifePlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
) -> Result<String, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        method_version: &'static str,
        phase: MultiModFullRefitPhaseV1,
        plan_sha256: String,
        case_count: usize,
        case_weights_sha256: Option<String>,
        scientific_refit_identity_sha256: &'a str,
    }
    sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        phase: MultiModFullRefitPhaseV1::DeleteOneJackknife,
        plan_sha256: plan.identity_sha256()?,
        case_count,
        case_weights_sha256: case_weights.map(sha256_f64_slice),
        scientific_refit_identity_sha256: &plan.scientific_refit_identity_sha256,
    })
}

fn frequency_execution_identity(
    plan: &MultiModBootstrapPlanV1,
    frequencies: &[u64],
) -> Result<String, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        method_version: &'static str,
        phase: MultiModFullRefitPhaseV1,
        plan_sha256: String,
        frequencies_sha256: String,
        frequency_rows: usize,
        frequency_total: u64,
        scientific_refit_identity_sha256: &'a str,
    }
    sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        phase: MultiModFullRefitPhaseV1::FrequencyBootstrap,
        plan_sha256: plan.identity_sha256()?,
        frequencies_sha256: sha256_u64_slice(frequencies),
        frequency_rows: frequencies.len(),
        frequency_total: validate_frequency_weights_v1(frequencies)?,
        scientific_refit_identity_sha256: &plan.scientific_refit_identity_sha256,
    })
}

fn studentized_execution_identity(
    plan: &MultiModStudentizedPlanV1,
    case_count: usize,
    case_weights: Option<&[f64]>,
) -> Result<String, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        method_version: &'static str,
        phase: MultiModFullRefitPhaseV1,
        plan_sha256: String,
        case_count: usize,
        case_weights_sha256: Option<String>,
        scientific_refit_identity_sha256: &'a str,
    }
    sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        phase: MultiModFullRefitPhaseV1::StudentizedOuter,
        plan_sha256: plan.identity_sha256()?,
        case_count,
        case_weights_sha256: case_weights.map(sha256_f64_slice),
        scientific_refit_identity_sha256: &plan.scientific_refit_identity_sha256,
    })
}

fn make_shard_identity(
    phase: MultiModFullRefitPhaseV1,
    execution_identity_sha256: &str,
    shard: MultiModShardSpecV1,
) -> Result<MultiModShardIdentityV1, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        method_version: &'static str,
        phase: MultiModFullRefitPhaseV1,
        execution_identity_sha256: &'a str,
        shard_index: u32,
        shard_count: u32,
    }
    let shard_identity_sha256 = sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        phase,
        execution_identity_sha256,
        shard_index: shard.shard_index,
        shard_count: shard.shard_count,
    })?;
    Ok(MultiModShardIdentityV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1.into(),
        phase,
        execution_identity_sha256: execution_identity_sha256.into(),
        shard_index: shard.shard_index,
        shard_count: shard.shard_count,
        shard_identity_sha256,
    })
}

fn make_case_draw(
    plan: &MultiModBootstrapPlanV1,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    replicate_index: u32,
    phase: MultiModFullRefitPhaseV1,
) -> Result<MultiModCaseBootstrapDrawV1, MultiModFullRefitErrorV1> {
    let mut rng = coordinate_rng(
        execution_identity,
        phase,
        plan.master_seed,
        &[u64::from(replicate_index)],
    );
    let source_rows = (0..case_count)
        .map(|_| rng.random_range(0..case_count) as u32)
        .collect::<Vec<_>>();
    let selected_weights = select_and_normalize_weights(case_weights, &source_rows)?;
    case_draw_from_rows(replicate_index, source_rows, selected_weights)
}

fn make_studentized_outer_draw(
    plan: &MultiModStudentizedPlanV1,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    replicate_index: u32,
) -> Result<MultiModCaseBootstrapDrawV1, MultiModFullRefitErrorV1> {
    let mut rng = coordinate_rng(
        execution_identity,
        MultiModFullRefitPhaseV1::StudentizedOuter,
        plan.master_seed,
        &[u64::from(replicate_index)],
    );
    let source_rows = (0..case_count)
        .map(|_| rng.random_range(0..case_count) as u32)
        .collect::<Vec<_>>();
    let selected_weights = select_and_normalize_weights(case_weights, &source_rows)?;
    case_draw_from_rows(replicate_index, source_rows, selected_weights)
}

fn case_draw_from_rows(
    replicate_index: u32,
    source_rows: Vec<u32>,
    case_weights: Option<Vec<f64>>,
) -> Result<MultiModCaseBootstrapDrawV1, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        replicate_index: u32,
        source_rows: &'a [u32],
        case_weights: Option<&'a [f64]>,
    }
    let source_rows_sha256 = sha256_u32_slice(&source_rows);
    let case_weights_sha256 = case_weights.as_deref().map(sha256_f64_slice);
    let draw_identity_sha256 = sha256_json(&Identity {
        replicate_index,
        source_rows: &source_rows,
        case_weights: case_weights.as_deref(),
    })?;
    Ok(MultiModCaseBootstrapDrawV1 {
        replicate_index,
        source_rows,
        case_weights,
        source_rows_sha256,
        case_weights_sha256,
        draw_identity_sha256,
    })
}

fn make_jackknife_draw(
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    omitted_row: u32,
) -> Result<MultiModDeleteOneJackknifeDrawV1, MultiModFullRefitErrorV1> {
    let retained_source_rows = (0..case_count as u32)
        .filter(|row| *row != omitted_row)
        .collect::<Vec<_>>();
    let selected_weights = select_and_normalize_weights(case_weights, &retained_source_rows)?;
    #[derive(Serialize)]
    struct Identity<'a> {
        execution_identity_sha256: &'a str,
        omitted_row: u32,
        retained_source_rows: &'a [u32],
        case_weights: Option<&'a [f64]>,
    }
    let retained_rows_sha256 = sha256_u32_slice(&retained_source_rows);
    let case_weights_sha256 = selected_weights.as_deref().map(sha256_f64_slice);
    let draw_identity_sha256 = sha256_json(&Identity {
        execution_identity_sha256: execution_identity,
        omitted_row,
        retained_source_rows: &retained_source_rows,
        case_weights: selected_weights.as_deref(),
    })?;
    Ok(MultiModDeleteOneJackknifeDrawV1 {
        omitted_row,
        retained_source_rows,
        case_weights: selected_weights,
        retained_rows_sha256,
        case_weights_sha256,
        draw_identity_sha256,
    })
}

fn make_frequency_draw(
    plan: &MultiModBootstrapPlanV1,
    execution_identity: &str,
    frequencies: &[u64],
    replicate_index: u32,
) -> Result<MultiModFrequencyBootstrapDrawV1, MultiModFullRefitErrorV1> {
    let total_count = validate_frequency_weights_v1(frequencies)?;
    let mut rng = coordinate_rng(
        execution_identity,
        MultiModFullRefitPhaseV1::FrequencyBootstrap,
        plan.master_seed,
        &[u64::from(replicate_index)],
    );
    let counts = multinomial_count_space_draw(frequencies, total_count, &mut rng)?;
    #[derive(Serialize)]
    struct Identity<'a> {
        replicate_index: u32,
        counts: &'a [u64],
        total_count: u64,
    }
    let counts_sha256 = sha256_u64_slice(&counts);
    let draw_identity_sha256 = sha256_json(&Identity {
        replicate_index,
        counts: &counts,
        total_count,
    })?;
    Ok(MultiModFrequencyBootstrapDrawV1 {
        replicate_index,
        counts,
        total_count,
        counts_sha256,
        draw_identity_sha256,
    })
}

fn make_studentized_inner_draw(
    plan: &MultiModStudentizedPlanV1,
    execution_identity: &str,
    outer_draw: &MultiModCaseBootstrapDrawV1,
    inner_replicate_index: u32,
) -> Result<MultiModStudentizedInnerDrawV1, MultiModFullRefitErrorV1> {
    let case_count = outer_draw.source_rows.len();
    let mut rng = coordinate_rng(
        execution_identity,
        MultiModFullRefitPhaseV1::StudentizedInner,
        plan.master_seed,
        &[
            u64::from(outer_draw.replicate_index),
            u64::from(inner_replicate_index),
        ],
    );
    let positions = (0..case_count)
        .map(|_| rng.random_range(0..case_count))
        .collect::<Vec<_>>();
    let source_rows = positions
        .iter()
        .map(|position| outer_draw.source_rows[*position])
        .collect::<Vec<_>>();
    let case_weights = outer_draw.case_weights.as_deref().map(|outer_weights| {
        positions
            .iter()
            .map(|position| outer_weights[*position])
            .collect::<Vec<_>>()
    });
    let case_weights = normalize_owned_weights(case_weights)?;
    #[derive(Serialize)]
    struct Identity<'a> {
        outer_replicate_index: u32,
        inner_replicate_index: u32,
        outer_draw_identity_sha256: &'a str,
        source_rows: &'a [u32],
        case_weights: Option<&'a [f64]>,
    }
    let source_rows_sha256 = sha256_u32_slice(&source_rows);
    let case_weights_sha256 = case_weights.as_deref().map(sha256_f64_slice);
    let draw_identity_sha256 = sha256_json(&Identity {
        outer_replicate_index: outer_draw.replicate_index,
        inner_replicate_index,
        outer_draw_identity_sha256: &outer_draw.draw_identity_sha256,
        source_rows: &source_rows,
        case_weights: case_weights.as_deref(),
    })?;
    Ok(MultiModStudentizedInnerDrawV1 {
        outer_replicate_index: outer_draw.replicate_index,
        inner_replicate_index,
        outer_draw_identity_sha256: outer_draw.draw_identity_sha256.clone(),
        source_rows,
        case_weights,
        source_rows_sha256,
        case_weights_sha256,
        draw_identity_sha256,
    })
}

fn multinomial_count_space_draw(
    frequencies: &[u64],
    total: u64,
    rng: &mut ChaCha20Rng,
) -> Result<Vec<u64>, MultiModFullRefitErrorV1> {
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
                .map_err(|error| MultiModFullRefitErrorV1::CountSpace(error.to_string()))?;
            distribution.inverse_cdf(open_unit_interval(rng))
        };
        if count > remaining_draws {
            return Err(MultiModFullRefitErrorV1::CountSpace(
                "binomial component exceeded remaining multinomial count".into(),
            ));
        }
        counts.push(count);
        remaining_draws -= count;
        remaining_mass -= *frequency;
    }
    counts.push(remaining_draws);
    if counts
        .iter()
        .try_fold(0u64, |sum, count| sum.checked_add(*count))
        != Some(total)
    {
        return Err(MultiModFullRefitErrorV1::CountSpace(
            "multinomial count total was not preserved".into(),
        ));
    }
    Ok(counts)
}

fn open_unit_interval(rng: &mut ChaCha20Rng) -> f64 {
    const DENOMINATOR: u64 = 1_u64 << 53;
    let numerator = rng.random_range(1..DENOMINATOR);
    numerator as f64 / DENOMINATOR as f64
}

fn select_and_normalize_weights(
    case_weights: Option<&[f64]>,
    source_rows: &[u32],
) -> Result<Option<Vec<f64>>, MultiModFullRefitErrorV1> {
    normalize_owned_weights(case_weights.map(|weights| {
        source_rows
            .iter()
            .map(|row| weights[*row as usize])
            .collect::<Vec<_>>()
    }))
}

fn normalize_owned_weights(
    weights: Option<Vec<f64>>,
) -> Result<Option<Vec<f64>>, MultiModFullRefitErrorV1> {
    let Some(mut weights) = weights else {
        return Ok(None);
    };
    let total = weights.iter().sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err(MultiModFullRefitErrorV1::InvalidCaseWeights(
            "resampled case-weight sum is nonfinite or nonpositive".into(),
        ));
    }
    let scale = weights.len() as f64 / total;
    for weight in &mut weights {
        *weight *= scale;
        if !weight.is_finite() || *weight <= 0.0 {
            return Err(MultiModFullRefitErrorV1::InvalidCaseWeights(
                "normalized resampled case weight is nonfinite or nonpositive".into(),
            ));
        }
    }
    Ok(Some(weights))
}

fn coordinate_rng(
    execution_identity: &str,
    phase: MultiModFullRefitPhaseV1,
    master_seed: u64,
    coordinates: &[u64],
) -> ChaCha20Rng {
    let mut digest = Sha256::new();
    digest.update(RNG_DOMAIN_V1);
    digest.update((execution_identity.len() as u64).to_le_bytes());
    digest.update(execution_identity.as_bytes());
    digest.update((phase.as_str().len() as u64).to_le_bytes());
    digest.update(phase.as_str().as_bytes());
    digest.update(master_seed.to_le_bytes());
    digest.update((coordinates.len() as u64).to_le_bytes());
    for coordinate in coordinates {
        digest.update(coordinate.to_le_bytes());
    }
    ChaCha20Rng::from_seed(digest.finalize().into())
}

fn make_record<Draw: Serialize, Estimate: Serialize>(
    index: u32,
    draw: Draw,
    result: Result<Estimate, MultiModRefitFailureV1>,
) -> Result<MultiModRefitRecordV1<Draw, Estimate>, MultiModFullRefitErrorV1> {
    let outcome = callback_outcome(index, result)?;
    let record_identity_sha256 = record_identity(index, 1, &draw, &outcome)?;
    Ok(MultiModRefitRecordV1 {
        index,
        attempt_count: 1,
        draw,
        outcome,
        record_identity_sha256,
    })
}

fn record_identity<Draw: Serialize, Estimate: Serialize>(
    index: u32,
    attempt_count: u8,
    draw: &Draw,
    outcome: &MultiModRefitOutcomeV1<Estimate>,
) -> Result<String, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a, D, T> {
        method_version: &'static str,
        index: u32,
        attempt_count: u8,
        draw: &'a D,
        outcome: &'a MultiModRefitOutcomeV1<T>,
    }
    sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        index,
        attempt_count,
        draw,
        outcome,
    })
}

fn callback_outcome<T: Serialize>(
    index: u32,
    result: Result<T, MultiModRefitFailureV1>,
) -> Result<MultiModRefitOutcomeV1<T>, MultiModFullRefitErrorV1> {
    match result {
        Ok(value) => Ok(MultiModRefitOutcomeV1::Success {
            value_sha256: sha256_json(&value)?,
            value,
        }),
        Err(failure) => {
            validate_failure(&failure)
                .map_err(|_| MultiModFullRefitErrorV1::InvalidCallbackFailure(index))?;
            Ok(MultiModRefitOutcomeV1::Failed {
                failure_sha256: sha256_json(&failure)?,
                failure,
            })
        }
    }
}

fn validate_failure(failure: &MultiModRefitFailureV1) -> Result<(), ()> {
    if failure.code.is_empty()
        || failure.code.len() > 128
        || failure.code.trim() != failure.code
        || !failure.code.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
        || failure.message.len() > 4096
        || failure.message.trim() != failure.message
        || failure.message.chars().any(char::is_control)
    {
        Err(())
    } else {
        Ok(())
    }
}

fn validate_outcome<T: Serialize>(
    index: u32,
    outcome: &MultiModRefitOutcomeV1<T>,
) -> Result<(), MultiModFullRefitErrorV1> {
    match outcome {
        MultiModRefitOutcomeV1::Success {
            value,
            value_sha256,
        } => {
            if is_sha256(value_sha256) && sha256_json(value)? == *value_sha256 {
                Ok(())
            } else {
                Err(MultiModFullRefitErrorV1::OutcomeIdentityMismatch(index))
            }
        }
        MultiModRefitOutcomeV1::Failed {
            failure,
            failure_sha256,
        } => {
            if validate_failure(failure).is_ok()
                && is_sha256(failure_sha256)
                && sha256_json(failure)? == *failure_sha256
            {
                Ok(())
            } else {
                Err(MultiModFullRefitErrorV1::OutcomeIdentityMismatch(index))
            }
        }
    }
}

fn resume_records<Draw, Estimate, Validate>(
    resume: Option<MultiModShardCacheV1<Draw, Estimate>>,
    expected_shard: &MultiModShardIdentityV1,
    mut validate: Validate,
) -> Result<BTreeMap<u32, MultiModRefitRecordV1<Draw, Estimate>>, MultiModFullRefitErrorV1>
where
    Estimate: Serialize,
    Validate: FnMut(&MultiModRefitRecordV1<Draw, Estimate>) -> Result<(), MultiModFullRefitErrorV1>,
{
    let Some(resume) = resume else {
        return Ok(BTreeMap::new());
    };
    if resume.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1
        || resume.shard != *expected_shard
    {
        return Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch);
    }
    let mut records = BTreeMap::new();
    for record in resume.records {
        validate(&record)?;
        let index = record.index;
        if records.insert(index, record).is_some() {
            return Err(MultiModFullRefitErrorV1::DuplicateRecord(index));
        }
    }
    Ok(records)
}

fn shard_cache<Draw, Estimate>(
    shard: MultiModShardIdentityV1,
    cancelled: bool,
    records: BTreeMap<u32, MultiModRefitRecordV1<Draw, Estimate>>,
) -> MultiModShardCacheV1<Draw, Estimate> {
    MultiModShardCacheV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        shard,
        cancelled,
        records: records.into_values().collect(),
    }
}

fn studentized_shard_cache<OuterEstimate, InnerEstimate>(
    shard: MultiModShardIdentityV1,
    cancelled: bool,
    records: BTreeMap<u32, MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate>>,
) -> MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate> {
    MultiModStudentizedShardCacheV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        shard,
        cancelled,
        records: records.into_values().collect(),
    }
}

fn validate_case_record<Estimate: Serialize>(
    record: &MultiModRefitRecordV1<MultiModCaseBootstrapDrawV1, Estimate>,
    plan: &MultiModBootstrapPlanV1,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
) -> Result<(), MultiModFullRefitErrorV1> {
    if record.index >= plan.requested_replicates || !shard.owns(record.index) {
        return Err(MultiModFullRefitErrorV1::RecordOutsideShard(record.index));
    }
    let expected = make_case_draw(
        plan,
        execution_identity,
        case_count,
        case_weights,
        record.index,
        MultiModFullRefitPhaseV1::CaseBootstrap,
    )?;
    validate_record(record, &expected)
}

fn validate_frequency_record<Estimate: Serialize>(
    record: &MultiModRefitRecordV1<MultiModFrequencyBootstrapDrawV1, Estimate>,
    plan: &MultiModBootstrapPlanV1,
    execution_identity: &str,
    frequencies: &[u64],
    shard: MultiModShardSpecV1,
) -> Result<(), MultiModFullRefitErrorV1> {
    if record.index >= plan.requested_replicates || !shard.owns(record.index) {
        return Err(MultiModFullRefitErrorV1::RecordOutsideShard(record.index));
    }
    let expected = make_frequency_draw(plan, execution_identity, frequencies, record.index)?;
    validate_record(record, &expected)
}

fn validate_jackknife_record<Estimate: Serialize>(
    record: &MultiModRefitRecordV1<MultiModDeleteOneJackknifeDrawV1, Estimate>,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
) -> Result<(), MultiModFullRefitErrorV1> {
    if record.index >= case_count as u32 || !shard.owns(record.index) {
        return Err(MultiModFullRefitErrorV1::RecordOutsideShard(record.index));
    }
    let expected = make_jackknife_draw(execution_identity, case_count, case_weights, record.index)?;
    validate_record(record, &expected)
}

fn validate_record<Draw: PartialEq + Serialize, Estimate: Serialize>(
    record: &MultiModRefitRecordV1<Draw, Estimate>,
    expected_draw: &Draw,
) -> Result<(), MultiModFullRefitErrorV1> {
    if record.attempt_count != 1 || &record.draw != expected_draw {
        return Err(MultiModFullRefitErrorV1::DrawIdentityMismatch(record.index));
    }
    validate_outcome(record.index, &record.outcome)?;
    let expected_identity = record_identity(
        record.index,
        record.attempt_count,
        &record.draw,
        &record.outcome,
    )?;
    if !is_sha256(&record.record_identity_sha256)
        || record.record_identity_sha256 != expected_identity
    {
        return Err(MultiModFullRefitErrorV1::RecordIdentityMismatch(
            record.index,
        ));
    }
    Ok(())
}

fn collect_complete_shards<Draw, Estimate, Validate>(
    shards: Vec<MultiModShardCacheV1<Draw, Estimate>>,
    phase: MultiModFullRefitPhaseV1,
    execution_identity: &str,
    requested: u32,
    mut validate: Validate,
) -> Result<Vec<MultiModRefitRecordV1<Draw, Estimate>>, MultiModFullRefitErrorV1>
where
    Estimate: Serialize,
    Validate: FnMut(
        &MultiModRefitRecordV1<Draw, Estimate>,
        MultiModShardSpecV1,
    ) -> Result<(), MultiModFullRefitErrorV1>,
{
    let Some(first) = shards.first() else {
        return Err(MultiModFullRefitErrorV1::IncompleteLedger {
            expected: requested,
            actual: 0,
        });
    };
    let shard_count = first.shard.shard_count;
    if shards.len() != shard_count as usize {
        return Err(MultiModFullRefitErrorV1::IncompleteLedger {
            expected: shard_count,
            actual: shards.len(),
        });
    }
    let mut seen_shards = BTreeSet::new();
    let mut records = BTreeMap::new();
    for cache in shards {
        let spec = MultiModShardSpecV1 {
            shard_index: cache.shard.shard_index,
            shard_count: cache.shard.shard_count,
        };
        spec.ensure_valid(requested)?;
        if spec.shard_count != shard_count {
            return Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch);
        }
        let expected_shard = make_shard_identity(phase, execution_identity, spec)?;
        if cache.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1
            || cache.shard != expected_shard
            || !seen_shards.insert(spec.shard_index)
        {
            return Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch);
        }
        if cache.cancelled {
            return Err(MultiModFullRefitErrorV1::CancelledShardNotResumed(
                spec.shard_index,
            ));
        }
        for record in cache.records {
            validate(&record, spec)?;
            let index = record.index;
            if records.insert(index, record).is_some() {
                return Err(MultiModFullRefitErrorV1::DuplicateRecord(index));
            }
        }
    }
    if records.len() != requested as usize {
        return Err(MultiModFullRefitErrorV1::IncompleteLedger {
            expected: requested,
            actual: records.len(),
        });
    }
    Ok(records.into_values().collect())
}

fn finalize_ledger<Draw, Estimate>(
    execution_identity: &str,
    requested: u32,
    minimum_usable_fraction: f64,
    records: Vec<MultiModRefitRecordV1<Draw, Estimate>>,
    full_jackknife: bool,
) -> Result<MultiModFinalLedgerV1<Draw, Estimate>, MultiModFullRefitErrorV1>
where
    Draw: Serialize,
    Estimate: Serialize,
{
    let usable_indices = records
        .iter()
        .filter(|record| matches!(record.outcome, MultiModRefitOutcomeV1::Success { .. }))
        .map(|record| record.index)
        .collect::<Vec<_>>();
    let usable = usable_indices.len() as u32;
    let minimum_required = minimum_required(requested, minimum_usable_fraction);
    if full_jackknife && usable != requested {
        return Err(MultiModFullRefitErrorV1::IncompleteJackknife {
            usable,
            required: requested,
        });
    }
    if usable < minimum_required {
        return Err(MultiModFullRefitErrorV1::UsableGateFailed {
            usable,
            minimum_required,
        });
    }
    #[derive(Serialize)]
    struct LedgerIdentity<'a, D, T> {
        method_version: &'static str,
        execution_identity_sha256: &'a str,
        requested: u32,
        usable: u32,
        usable_indices: &'a [u32],
        minimum_required: u32,
        records: &'a [MultiModRefitRecordV1<D, T>],
    }
    let ledger_sha256 = sha256_json(&LedgerIdentity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        execution_identity_sha256: execution_identity,
        requested,
        usable,
        usable_indices: &usable_indices,
        minimum_required,
        records: &records,
    })?;
    Ok(MultiModFinalLedgerV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1.into(),
        execution_identity_sha256: execution_identity.into(),
        requested,
        usable,
        usable_indices,
        minimum_required,
        usable_fraction: f64::from(usable) / f64::from(requested),
        complete: true,
        records,
        ledger_sha256,
    })
}

fn inner_ledger_identity(
    execution_identity: &str,
    outer_index: u32,
) -> Result<String, MultiModFullRefitErrorV1> {
    #[derive(Serialize)]
    struct Identity<'a> {
        method_version: &'static str,
        phase: MultiModFullRefitPhaseV1,
        execution_identity_sha256: &'a str,
        outer_index: u32,
    }
    sha256_json(&Identity {
        method_version: MULTIMOD_FULL_REFIT_ORCHESTRATOR_V1,
        phase: MultiModFullRefitPhaseV1::StudentizedInner,
        execution_identity_sha256: execution_identity,
        outer_index,
    })
}

fn resume_studentized_records<OuterEstimate, InnerEstimate>(
    resume: Option<MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate>>,
    expected_shard: &MultiModShardIdentityV1,
    plan: &MultiModStudentizedPlanV1,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
) -> Result<
    BTreeMap<u32, MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate>>,
    MultiModFullRefitErrorV1,
>
where
    OuterEstimate: Serialize,
    InnerEstimate: Serialize,
{
    let Some(resume) = resume else {
        return Ok(BTreeMap::new());
    };
    if resume.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1
        || resume.shard != *expected_shard
    {
        return Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch);
    }
    let mut records = BTreeMap::new();
    for record in resume.records {
        validate_studentized_outer_record(
            &record,
            plan,
            execution_identity,
            case_count,
            case_weights,
            shard,
            false,
        )?;
        let index = record.outer.index;
        if records.insert(index, record).is_some() {
            return Err(MultiModFullRefitErrorV1::DuplicateRecord(index));
        }
    }
    Ok(records)
}

fn validate_studentized_outer_record<OuterEstimate, InnerEstimate>(
    record: &MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate>,
    plan: &MultiModStudentizedPlanV1,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
    shard: MultiModShardSpecV1,
    require_complete_inner: bool,
) -> Result<(), MultiModFullRefitErrorV1>
where
    OuterEstimate: Serialize,
    InnerEstimate: Serialize,
{
    let outer_index = record.outer.index;
    if outer_index >= plan.outer_replicates || !shard.owns(outer_index) {
        return Err(MultiModFullRefitErrorV1::RecordOutsideShard(outer_index));
    }
    let expected_outer = make_studentized_outer_draw(
        plan,
        execution_identity,
        case_count,
        case_weights,
        outer_index,
    )?;
    validate_record(&record.outer, &expected_outer)?;
    if record.inner_ledger_identity_sha256
        != inner_ledger_identity(execution_identity, outer_index)?
    {
        return Err(MultiModFullRefitErrorV1::InvalidInnerLedger {
            outer_index,
            reason: "inner ledger identity differs".into(),
        });
    }
    if matches!(record.outer.outcome, MultiModRefitOutcomeV1::Failed { .. }) {
        if !record.inner_records.is_empty() {
            return Err(MultiModFullRefitErrorV1::InvalidInnerLedger {
                outer_index,
                reason: "failed outer refit cannot carry inner refits".into(),
            });
        }
        return Ok(());
    }
    let mut inner_indices = BTreeSet::new();
    for inner in &record.inner_records {
        if inner.index >= plan.inner_replicates || !inner_indices.insert(inner.index) {
            return Err(MultiModFullRefitErrorV1::InvalidInnerLedger {
                outer_index,
                reason: "inner replicate index is duplicate or out of range".into(),
            });
        }
        let expected =
            make_studentized_inner_draw(plan, execution_identity, &record.outer.draw, inner.index)?;
        validate_record(inner, &expected).map_err(|error| {
            MultiModFullRefitErrorV1::InvalidInnerLedger {
                outer_index,
                reason: error.to_string(),
            }
        })?;
    }
    if require_complete_inner && record.inner_records.len() != plan.inner_replicates as usize {
        return Err(MultiModFullRefitErrorV1::InvalidInnerLedger {
            outer_index,
            reason: format!(
                "expected {} inner records, found {}",
                plan.inner_replicates,
                record.inner_records.len()
            ),
        });
    }
    Ok(())
}

fn collect_complete_studentized_shards<OuterEstimate, InnerEstimate>(
    shards: Vec<MultiModStudentizedShardCacheV1<OuterEstimate, InnerEstimate>>,
    plan: &MultiModStudentizedPlanV1,
    execution_identity: &str,
    case_count: usize,
    case_weights: Option<&[f64]>,
) -> Result<
    Vec<MultiModStudentizedOuterRecordV1<OuterEstimate, InnerEstimate>>,
    MultiModFullRefitErrorV1,
>
where
    OuterEstimate: Serialize,
    InnerEstimate: Serialize,
{
    let Some(first) = shards.first() else {
        return Err(MultiModFullRefitErrorV1::IncompleteLedger {
            expected: plan.outer_replicates,
            actual: 0,
        });
    };
    let shard_count = first.shard.shard_count;
    if shards.len() != shard_count as usize {
        return Err(MultiModFullRefitErrorV1::IncompleteLedger {
            expected: shard_count,
            actual: shards.len(),
        });
    }
    let mut seen_shards = BTreeSet::new();
    let mut records = BTreeMap::new();
    for cache in shards {
        let shard = MultiModShardSpecV1 {
            shard_index: cache.shard.shard_index,
            shard_count: cache.shard.shard_count,
        };
        shard.ensure_valid(plan.outer_replicates)?;
        if shard.shard_count != shard_count {
            return Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch);
        }
        let expected_shard = make_shard_identity(
            MultiModFullRefitPhaseV1::StudentizedOuter,
            execution_identity,
            shard,
        )?;
        if cache.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1
            || cache.shard != expected_shard
            || !seen_shards.insert(shard.shard_index)
        {
            return Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch);
        }
        if cache.cancelled {
            return Err(MultiModFullRefitErrorV1::CancelledShardNotResumed(
                shard.shard_index,
            ));
        }
        for record in cache.records {
            validate_studentized_outer_record(
                &record,
                plan,
                execution_identity,
                case_count,
                case_weights,
                shard,
                true,
            )?;
            let index = record.outer.index;
            if records.insert(index, record).is_some() {
                return Err(MultiModFullRefitErrorV1::DuplicateRecord(index));
            }
        }
    }
    if records.len() != plan.outer_replicates as usize {
        return Err(MultiModFullRefitErrorV1::IncompleteLedger {
            expected: plan.outer_replicates,
            actual: records.len(),
        });
    }
    Ok(records.into_values().collect())
}

fn sha256_json(value: &impl Serialize) -> Result<String, MultiModFullRefitErrorV1> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|error| MultiModFullRefitErrorV1::Serialization(error.to_string()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_u32_slice(values: &[u32]) -> String {
    let mut digest = Sha256::new();
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        digest.update(value.to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn sha256_u64_slice(values: &[u64]) -> String {
    let mut digest = Sha256::new();
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        digest.update(value.to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn sha256_f64_slice(values: &[f64]) -> String {
    let mut digest = Sha256::new();
    digest.update((values.len() as u64).to_le_bytes());
    for value in values {
        digest.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scientific_identity(label: &str) -> String {
        sha256_bytes(label.as_bytes())
    }

    fn bootstrap_plan(replicates: u32) -> MultiModBootstrapPlanV1 {
        MultiModBootstrapPlanV1 {
            schema_version: 1,
            scientific_refit_identity_sha256: scientific_identity("fixture:conditional-process"),
            requested_replicates: replicates,
            master_seed: 42,
            minimum_usable_fraction: 0.90,
        }
    }

    fn failure(code: &str) -> MultiModRefitFailureV1 {
        MultiModRefitFailureV1 {
            code: code.into(),
            message: code.into(),
        }
    }

    fn run_case_shards(
        plan: &MultiModBootstrapPlanV1,
        shard_count: u32,
        weights: Option<&[f64]>,
    ) -> Vec<MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, u64>> {
        (0..shard_count)
            .map(|shard_index| {
                let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
                    Ok(draw.source_rows.iter().map(|row| u64::from(*row)).sum())
                };
                run_multimod_case_bootstrap_shard_v1(
                    plan,
                    6,
                    weights,
                    MultiModShardSpecV1 {
                        shard_index,
                        shard_count,
                    },
                    None,
                    &mut callback,
                    || false,
                )
                .unwrap()
            })
            .collect()
    }

    #[test]
    fn case_ledger_is_shard_invariant_and_weights_follow_rows() {
        let plan = bootstrap_plan(10);
        let weights = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let serial = finalize_multimod_case_bootstrap_v1(
            &plan,
            6,
            Some(&weights),
            run_case_shards(&plan, 1, Some(&weights)),
        )
        .unwrap();
        let sharded = finalize_multimod_case_bootstrap_v1(
            &plan,
            6,
            Some(&weights),
            run_case_shards(&plan, 2, Some(&weights)),
        )
        .unwrap();
        assert_eq!(serial.ledger_sha256, sharded.ledger_sha256);
        assert_eq!(serial.records, sharded.records);
        for record in &serial.records {
            let selected = record.draw.case_weights.as_ref().unwrap();
            assert!((selected.iter().sum::<f64>() / selected.len() as f64 - 1.0).abs() < 1e-12);
            for (position, row) in record.draw.source_rows.iter().enumerate() {
                let raw_ratio = selected[position] / weights[*row as usize];
                assert!(raw_ratio.is_finite() && raw_ratio > 0.0);
            }
        }
    }

    #[test]
    fn finalizer_rejects_mixed_shard_partitions_even_when_indices_cover_the_ledger() {
        let plan = bootstrap_plan(3);
        let shard_zero_of_two = run_case_shards(&plan, 2, None).remove(0);
        let shard_one_of_three = run_case_shards(&plan, 3, None).remove(1);
        assert!(matches!(
            finalize_multimod_case_bootstrap_v1(
                &plan,
                6,
                None,
                vec![shard_zero_of_two, shard_one_of_three],
            ),
            Err(MultiModFullRefitErrorV1::ResumeIdentityMismatch)
        ));
    }

    #[test]
    fn cancellation_resume_and_failures_never_retry_completed_indices() {
        let plan = bootstrap_plan(10);
        let calls = std::cell::Cell::new(0u32);
        let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
            calls.set(calls.get() + 1);
            if draw.replicate_index == 1 {
                Err(failure("rank_deficient"))
            } else {
                Ok(draw.replicate_index)
            }
        };
        let cancellation_checks = std::cell::Cell::new(0u32);
        let cache = run_multimod_case_bootstrap_shard_v1(
            &plan,
            6,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || {
                cancellation_checks.set(cancellation_checks.get() + 1);
                cancellation_checks.get() > 4
            },
        )
        .unwrap();
        assert!(cache.cancelled);
        assert_eq!(cache.records.len(), 4);
        assert_eq!(calls.get(), 4);
        assert!(matches!(
            finalize_multimod_case_bootstrap_v1(&plan, 6, None, vec![cache.clone()]),
            Err(MultiModFullRefitErrorV1::CancelledShardNotResumed(0))
        ));

        let mut resumed_callback = |draw: &MultiModCaseBootstrapDrawV1| {
            assert!(draw.replicate_index >= 4);
            calls.set(calls.get() + 1);
            Ok(draw.replicate_index)
        };
        let resumed = run_multimod_case_bootstrap_shard_v1(
            &plan,
            6,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            Some(cache),
            &mut resumed_callback,
            || false,
        )
        .unwrap();
        assert!(!resumed.cancelled);
        assert_eq!(calls.get(), 10);
        let ledger = finalize_multimod_case_bootstrap_v1(&plan, 6, None, vec![resumed]).unwrap();
        assert_eq!(ledger.usable, 9);
        assert!(!ledger.usable_indices.contains(&1));
        assert_eq!(ledger.minimum_required, 9);
    }

    #[test]
    fn in_flight_interruption_is_resumable_and_never_becomes_a_failure_record() {
        let plan = bootstrap_plan(10);
        let interrupted_once = std::cell::Cell::new(false);
        let calls = std::cell::Cell::new(0u32);
        let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
            calls.set(calls.get() + 1);
            if draw.replicate_index == 3 && !interrupted_once.replace(true) {
                MultiModRefitAttemptV1::Interrupted
            } else if draw.replicate_index == 1 {
                MultiModRefitAttemptV1::Completed(Err(failure("rank_deficient")))
            } else {
                MultiModRefitAttemptV1::Completed(Ok(draw.replicate_index))
            }
        };
        let cache = run_multimod_case_bootstrap_shard_interruptible_v1(
            &plan,
            6,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || false,
        )
        .unwrap();
        assert!(cache.cancelled);
        assert_eq!(cache.records.len(), 3);
        assert!(cache.records.iter().all(|record| record.index != 3));
        assert!(matches!(
            cache.records[1].outcome,
            MultiModRefitOutcomeV1::Failed { .. }
        ));

        let resumed = run_multimod_case_bootstrap_shard_interruptible_v1(
            &plan,
            6,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            Some(cache),
            &mut callback,
            || false,
        )
        .unwrap();
        assert!(!resumed.cancelled);
        assert_eq!(calls.get(), 11);
        let ledger = finalize_multimod_case_bootstrap_v1(&plan, 6, None, vec![resumed]).unwrap();
        assert_eq!(ledger.records.len(), 10);
        assert_eq!(ledger.usable, 9);
        assert_eq!(
            ledger
                .records
                .iter()
                .filter(|record| matches!(record.outcome, MultiModRefitOutcomeV1::Failed { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn ninety_percent_gate_fails_closed_without_fallback() {
        let plan = bootstrap_plan(10);
        let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
            if draw.replicate_index < 2 {
                Err(failure("nonfinite_estimate"))
            } else {
                Ok(draw.replicate_index)
            }
        };
        let cache = run_multimod_case_bootstrap_shard_v1(
            &plan,
            5,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || false,
        )
        .unwrap();
        assert!(matches!(
            finalize_multimod_case_bootstrap_v1(&plan, 5, None, vec![cache]),
            Err(MultiModFullRefitErrorV1::UsableGateFailed {
                usable: 8,
                minimum_required: 9
            })
        ));
    }

    #[test]
    fn jackknife_requires_every_delete_one_refit() {
        let plan = MultiModJackknifePlanV1 {
            schema_version: 1,
            scientific_refit_identity_sha256: scientific_identity("fixture:bca"),
        };
        let weights = [1.0, 2.0, 3.0, 4.0];
        let mut callback = |draw: &MultiModDeleteOneJackknifeDrawV1| {
            assert_eq!(draw.retained_source_rows.len(), 3);
            let selected = draw.case_weights.as_ref().unwrap();
            assert!((selected.iter().sum::<f64>() / 3.0 - 1.0).abs() < 1e-12);
            if draw.omitted_row == 2 {
                Err(failure("singular_design"))
            } else {
                Ok(draw.omitted_row)
            }
        };
        let cache = run_multimod_delete_one_jackknife_shard_v1(
            &plan,
            4,
            Some(&weights),
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || false,
        )
        .unwrap();
        assert!(matches!(
            finalize_multimod_delete_one_jackknife_v1(&plan, 4, Some(&weights), vec![cache]),
            Err(MultiModFullRefitErrorV1::IncompleteJackknife {
                usable: 3,
                required: 4
            })
        ));
    }

    #[test]
    fn frequency_bootstrap_is_deterministic_count_space_only() {
        let plan = bootstrap_plan(6);
        let frequencies = [2_u64, 3, 5];
        let run = || {
            let mut callback = |draw: &MultiModFrequencyBootstrapDrawV1| {
                assert_eq!(draw.counts.len(), frequencies.len());
                assert_eq!(draw.counts.iter().sum::<u64>(), 10);
                Ok(draw.counts.clone())
            };
            run_multimod_frequency_bootstrap_shard_v1(
                &plan,
                &frequencies,
                MultiModShardSpecV1 {
                    shard_index: 0,
                    shard_count: 1,
                },
                None,
                &mut callback,
                || false,
            )
            .unwrap()
        };
        let first = run();
        let second = run();
        assert_eq!(first, second);
        assert_eq!(
            validate_frequency_weights_v1(&[MULTIMOD_MAX_FREQUENCY_TOTAL_V1 - 1, 1]).unwrap(),
            MULTIMOD_MAX_FREQUENCY_TOTAL_V1
        );
        assert!(validate_frequency_weights_v1(&[1, 0]).is_err());
    }

    #[test]
    fn identity_and_weight_boundaries_fail_closed() {
        let mut plan = bootstrap_plan(1);
        plan.scientific_refit_identity_sha256 = "not-a-digest".into();
        assert!(matches!(
            plan.ensure_valid(),
            Err(MultiModFullRefitErrorV1::InvalidPlan(_))
        ));
        assert!(matches!(
            validate_case_input(2, Some(&[1.0, MULTIMOD_MAX_CASE_WEIGHT_RATIO_V1 + 1.0]), 2),
            Err(MultiModFullRefitErrorV1::InvalidCaseWeights(_))
        ));
        assert!(matches!(
            validate_frequency_weights_v1(&[MULTIMOD_MAX_FREQUENCY_TOTAL_V1, 1]),
            Err(MultiModFullRefitErrorV1::InvalidFrequencyWeights(_))
        ));
    }

    #[test]
    fn nested_studentized_ledgers_are_complete_and_gated_independently() {
        let plan = MultiModStudentizedPlanV1 {
            schema_version: 1,
            scientific_refit_identity_sha256: scientific_identity("fixture:studentized"),
            outer_replicates: 3,
            inner_replicates: 4,
            master_seed: 42,
            minimum_outer_usable_fraction: 0.90,
            minimum_inner_usable_fraction: 0.90,
        };
        let outer_calls = std::cell::Cell::new(0u32);
        let inner_calls = std::cell::Cell::new(0u32);
        let mut outer = |draw: &MultiModCaseBootstrapDrawV1| {
            outer_calls.set(outer_calls.get() + 1);
            Ok(draw.replicate_index)
        };
        let mut inner = |draw: &MultiModStudentizedInnerDrawV1| {
            inner_calls.set(inner_calls.get() + 1);
            assert_eq!(draw.source_rows.len(), 5);
            Ok((draw.outer_replicate_index, draw.inner_replicate_index))
        };
        let cache = run_multimod_studentized_shard_v1(
            &plan,
            5,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut outer,
            &mut inner,
            || false,
        )
        .unwrap();
        assert_eq!(outer_calls.get(), 3);
        assert_eq!(inner_calls.get(), 12);
        let ledger = finalize_multimod_studentized_v1(&plan, 5, None, vec![cache]).unwrap();
        assert_eq!(ledger.usable_outer, 3);
        assert_eq!(ledger.usable_outer_indices, vec![0, 1, 2]);
        assert_eq!(ledger.minimum_inner_required, 4);
        assert!(
            ledger
                .records
                .iter()
                .all(|record| record.inner_records.len() == 4)
        );

        let failure_plan = MultiModStudentizedPlanV1 {
            outer_replicates: 1,
            ..plan
        };
        let mut outer = |draw: &MultiModCaseBootstrapDrawV1| Ok(draw.replicate_index);
        let mut inner = |draw: &MultiModStudentizedInnerDrawV1| {
            if draw.inner_replicate_index == 0 {
                Err(failure("inner_rank_failure"))
            } else {
                Ok(draw.inner_replicate_index)
            }
        };
        let failed = run_multimod_studentized_shard_v1(
            &failure_plan,
            5,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut outer,
            &mut inner,
            || false,
        )
        .unwrap();
        assert!(matches!(
            finalize_multimod_studentized_v1(&failure_plan, 5, None, vec![failed]),
            Err(MultiModFullRefitErrorV1::UsableGateFailed {
                usable: 0,
                minimum_required: 1
            })
        ));
    }

    #[test]
    fn nested_studentized_resume_only_runs_missing_outer_and_inner_refits() {
        let plan = MultiModStudentizedPlanV1 {
            schema_version: 1,
            scientific_refit_identity_sha256: scientific_identity("fixture:studentized-resume"),
            outer_replicates: 2,
            inner_replicates: 3,
            master_seed: 42,
            minimum_outer_usable_fraction: 0.90,
            minimum_inner_usable_fraction: 0.90,
        };
        let outer_calls = std::cell::Cell::new(0u32);
        let inner_calls = std::cell::Cell::new(0u32);
        let mut outer = |draw: &MultiModCaseBootstrapDrawV1| {
            outer_calls.set(outer_calls.get() + 1);
            Ok(draw.replicate_index)
        };
        let mut inner = |draw: &MultiModStudentizedInnerDrawV1| {
            inner_calls.set(inner_calls.get() + 1);
            Ok((draw.outer_replicate_index, draw.inner_replicate_index))
        };
        let cancellation_checks = std::cell::Cell::new(0u32);
        let partial = run_multimod_studentized_shard_v1(
            &plan,
            5,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut outer,
            &mut inner,
            || {
                cancellation_checks.set(cancellation_checks.get() + 1);
                cancellation_checks.get() > 3
            },
        )
        .unwrap();
        assert!(partial.cancelled);
        assert_eq!(outer_calls.get(), 1);
        assert_eq!(inner_calls.get(), 2);

        let mut resumed_outer = |draw: &MultiModCaseBootstrapDrawV1| {
            assert_eq!(draw.replicate_index, 1);
            outer_calls.set(outer_calls.get() + 1);
            Ok(draw.replicate_index)
        };
        let mut resumed_inner = |draw: &MultiModStudentizedInnerDrawV1| {
            if draw.outer_replicate_index == 0 {
                assert_eq!(draw.inner_replicate_index, 2);
            }
            inner_calls.set(inner_calls.get() + 1);
            Ok((draw.outer_replicate_index, draw.inner_replicate_index))
        };
        let complete = run_multimod_studentized_shard_v1(
            &plan,
            5,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            Some(partial),
            &mut resumed_outer,
            &mut resumed_inner,
            || false,
        )
        .unwrap();
        assert!(!complete.cancelled);
        assert_eq!(outer_calls.get(), 2);
        assert_eq!(inner_calls.get(), 6);
        let ledger = finalize_multimod_studentized_v1(&plan, 5, None, vec![complete]).unwrap();
        assert_eq!(ledger.usable_outer_indices, vec![0, 1]);
    }

    #[test]
    fn cache_identity_tampering_is_rejected_before_refit() {
        let plan = bootstrap_plan(4);
        let mut callback = |draw: &MultiModCaseBootstrapDrawV1| Ok(draw.replicate_index);
        let mut cache = run_multimod_case_bootstrap_shard_v1(
            &plan,
            4,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || false,
        )
        .unwrap();
        cache.records[0].draw.source_rows[0] ^= 1;
        let calls = std::cell::Cell::new(0u32);
        let mut must_not_run = |_draw: &MultiModCaseBootstrapDrawV1| {
            calls.set(calls.get() + 1);
            Ok(0)
        };
        assert!(matches!(
            run_multimod_case_bootstrap_shard_v1(
                &plan,
                4,
                None,
                MultiModShardSpecV1 {
                    shard_index: 0,
                    shard_count: 1,
                },
                Some(cache),
                &mut must_not_run,
                || false,
            ),
            Err(MultiModFullRefitErrorV1::DrawIdentityMismatch(0))
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn valid_outcomes_cannot_be_swapped_between_replicates() {
        let plan = bootstrap_plan(2);
        let mut callback = |draw: &MultiModCaseBootstrapDrawV1| Ok(draw.replicate_index);
        let mut cache = run_multimod_case_bootstrap_shard_v1(
            &plan,
            4,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || false,
        )
        .unwrap();
        let (first, second) = cache.records.split_at_mut(1);
        std::mem::swap(&mut first[0].outcome, &mut second[0].outcome);
        let calls = std::cell::Cell::new(0u32);
        let mut must_not_run = |_draw: &MultiModCaseBootstrapDrawV1| {
            calls.set(calls.get() + 1);
            Ok(0)
        };
        assert!(matches!(
            run_multimod_case_bootstrap_shard_v1(
                &plan,
                4,
                None,
                MultiModShardSpecV1 {
                    shard_index: 0,
                    shard_count: 1,
                },
                Some(cache),
                &mut must_not_run,
                || false,
            ),
            Err(MultiModFullRefitErrorV1::RecordIdentityMismatch(0))
        ));
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn failure_identity_tampering_is_rejected_before_refit() {
        let plan = bootstrap_plan(2);
        let mut callback =
            |_draw: &MultiModCaseBootstrapDrawV1| Err::<u32, _>(failure("rank_deficient"));
        let mut cache = run_multimod_case_bootstrap_shard_v1(
            &plan,
            4,
            None,
            MultiModShardSpecV1 {
                shard_index: 0,
                shard_count: 1,
            },
            None,
            &mut callback,
            || false,
        )
        .unwrap();
        let MultiModRefitOutcomeV1::Failed { failure, .. } = &mut cache.records[0].outcome else {
            panic!("fixture must fail");
        };
        failure.code = "changed_after_cache".into();
        let calls = std::cell::Cell::new(0u32);
        let mut must_not_run = |_draw: &MultiModCaseBootstrapDrawV1| {
            calls.set(calls.get() + 1);
            Ok(0)
        };
        assert!(matches!(
            run_multimod_case_bootstrap_shard_v1(
                &plan,
                4,
                None,
                MultiModShardSpecV1 {
                    shard_index: 0,
                    shard_count: 1,
                },
                Some(cache),
                &mut must_not_run,
                || false,
            ),
            Err(MultiModFullRefitErrorV1::OutcomeIdentityMismatch(0))
        ));
        assert_eq!(calls.get(), 0);
    }
}
