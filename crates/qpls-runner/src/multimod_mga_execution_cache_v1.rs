//! Deterministic external execution cache for MultiMod MGA V1.
//!
//! The statistical estimators own immutable replicate streams and never retry
//! failed draws. This module adds resumability at scientific task boundaries:
//! one point fit per group, one selected pair/procedure, the shared group
//! bootstrap bank, each omnibus procedure, and final multiplicity aggregation.
//! A result may be published only after every planned shard is complete.

use qpls_core::{
    MgaMultigroupV1, MgaPairwiseComparisonV1, MgaProcedureV1, MicomPairResultV1,
    MultiModCompilationReceiptV1, sha256_serialized,
};
use qpls_estimation::{
    GroupBootstrapBanksV1, GroupIndexV1, GroupParameterVectorV1, InverseVarianceWaldResultV1,
    MicomPairwiseResultV1, MultigroupDesignV1, OmnibusPermutationResultV1, OrderedGroupPairV1,
    OrdinaryPlsPathStandardErrorV1, PairwiseParametricTestV1, PairwisePermutationResultV1,
    ParameterIdentityV1,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MGA_EXECUTION_PLAN_CONTRACT_V1: &str = "qpls.mga.execution_plan.v1";
pub const MGA_EXECUTION_CACHE_CONTRACT_V1: &str = "qpls.mga.execution_cache.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MgaExecutionShardKindV1 {
    PointFit {
        group: GroupIndexV1,
    },
    MicomPair {
        pair: OrderedGroupPairV1,
    },
    PairwisePermutation {
        pair: OrderedGroupPairV1,
    },
    SharedGroupBootstrapBanks,
    PairwiseBootstrapDerived {
        procedure: MgaProcedureV1,
        pair: OrderedGroupPairV1,
    },
    OmnibusPermutation,
    ParametricPair {
        procedure: MgaProcedureV1,
        pair: OrderedGroupPairV1,
    },
    ParametricWaldOmnibus,
    MultiplicityAggregation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MgaExecutionShardV1 {
    pub ordinal: u32,
    pub kind: MgaExecutionShardKindV1,
    pub input_identity_sha256: String,
    pub shard_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MgaExecutionPlanV1 {
    pub contract: String,
    /// Frozen scientific task-graph scope shared by raw and prepared runners.
    pub execution_scope: String,
    pub analysis_identity_sha256: String,
    pub dataset_fingerprint: String,
    pub design_identity_sha256: String,
    pub parameter_inventory_sha256: String,
    pub seed: u64,
    pub retry_policy: String,
    pub shards: Vec<MgaExecutionShardV1>,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaOrdinaryPointSeReceiptV1 {
    pub parameter: ParameterIdentityV1,
    pub receipt: OrdinaryPlsPathStandardErrorV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MgaExecutionShardPayloadV1 {
    PointFit {
        value: GroupParameterVectorV1,
        #[serde(default)]
        ordinary_path_standard_errors: Vec<MgaOrdinaryPointSeReceiptV1>,
    },
    MicomPair {
        value: MicomPairwiseResultV1,
        rows: Vec<MicomPairResultV1>,
    },
    PairwisePermutation {
        value: PairwisePermutationResultV1,
    },
    SharedGroupBootstrapBanks {
        value: GroupBootstrapBanksV1,
    },
    PairwiseRows {
        procedure: MgaProcedureV1,
        pair: OrderedGroupPairV1,
        rows: Vec<MgaPairwiseComparisonV1>,
    },
    OmnibusPermutation {
        value: OmnibusPermutationResultV1,
    },
    ParametricPairRows {
        procedure: MgaProcedureV1,
        pair: OrderedGroupPairV1,
        rows: Vec<MgaPairwiseComparisonV1>,
        tests: Vec<PairwiseParametricTestV1>,
    },
    ParametricWaldOmnibus {
        output_identity_sha256: String,
        tests: Vec<InverseVarianceWaldResultV1>,
    },
    MultiplicityAggregation {
        input_rows_sha256: String,
        rows: Vec<MgaPairwiseComparisonV1>,
    },
}

impl MgaExecutionShardPayloadV1 {
    fn matches_kind(&self, kind: &MgaExecutionShardKindV1) -> bool {
        match (self, kind) {
            (Self::PointFit { value, .. }, MgaExecutionShardKindV1::PointFit { group }) => {
                value.group == *group
            }
            (Self::MicomPair { value, .. }, MgaExecutionShardKindV1::MicomPair { pair }) => {
                value.pair == *pair
            }
            (
                Self::PairwisePermutation { value },
                MgaExecutionShardKindV1::PairwisePermutation { pair },
            ) => value.pair == *pair,
            (
                Self::SharedGroupBootstrapBanks { .. },
                MgaExecutionShardKindV1::SharedGroupBootstrapBanks,
            ) => true,
            (
                Self::PairwiseRows {
                    procedure, pair, ..
                },
                MgaExecutionShardKindV1::PairwiseBootstrapDerived {
                    procedure: expected_procedure,
                    pair: expected_pair,
                },
            ) => procedure == expected_procedure && pair == expected_pair,
            (Self::OmnibusPermutation { .. }, MgaExecutionShardKindV1::OmnibusPermutation) => true,
            (
                Self::ParametricPairRows {
                    procedure, pair, ..
                },
                MgaExecutionShardKindV1::ParametricPair {
                    procedure: expected_procedure,
                    pair: expected_pair,
                },
            ) => procedure == expected_procedure && pair == expected_pair,
            (
                Self::ParametricWaldOmnibus { .. },
                MgaExecutionShardKindV1::ParametricWaldOmnibus,
            ) => true,
            (
                Self::MultiplicityAggregation { .. },
                MgaExecutionShardKindV1::MultiplicityAggregation,
            ) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaExecutionCacheEntryV1 {
    pub shard_identity_sha256: String,
    pub payload_sha256: String,
    pub payload: MgaExecutionShardPayloadV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MgaExecutionCacheV1 {
    pub contract: String,
    pub plan_sha256: String,
    pub entries: Vec<MgaExecutionCacheEntryV1>,
}

/// Exclusive, in-memory authority for a cache that has passed one complete
/// validation against an immutable execution plan.
///
/// The ordinary cache methods remain deliberately defensive because callers
/// can construct or deserialize `MgaExecutionCacheV1` directly. A runner that
/// holds this session, however, has exclusive access to the cache: historical
/// entries cannot change behind the validation, so cached lookups need not
/// reserialize every prior payload. New entries are validated and hashed once
/// before they become visible through the session.
pub(crate) struct ValidatedMgaExecutionCacheSessionV1<'a> {
    plan: &'a MgaExecutionPlanV1,
    cache: &'a mut MgaExecutionCacheV1,
    shard_identity_by_kind: BTreeMap<MgaExecutionShardKindV1, String>,
    ordinal_by_shard_identity: BTreeMap<String, usize>,
    entry_index_by_shard_identity: BTreeMap<String, usize>,
    completed_shard_identities: BTreeSet<String>,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum MgaExecutionCacheErrorV1 {
    #[error("MGA execution plan is invalid: {0}")]
    InvalidPlan(String),
    #[error("MGA execution cache is invalid: {0}")]
    InvalidCache(String),
    #[error("MGA execution cache payload is invalid: {0}")]
    InvalidPayload(String),
    #[error("MGA execution is incomplete: {0}")]
    Incomplete(String),
    #[error("MGA shard execution was cancelled before commit")]
    Cancelled,
    #[error("MGA shard execution failed: {0}")]
    ExecutionFailed(String),
    #[error("MGA execution-cache checkpoint failed: {0}")]
    CheckpointFailed(String),
}

fn canonical_pair(pair: OrderedGroupPairV1) -> OrderedGroupPairV1 {
    if pair.group_a <= pair.group_b {
        pair
    } else {
        OrderedGroupPairV1 {
            group_a: pair.group_b,
            group_b: pair.group_a,
        }
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn plan_sha256(plan: &MgaExecutionPlanV1) -> String {
    sha256_serialized(&(
        plan.contract.as_str(),
        plan.execution_scope.as_str(),
        plan.analysis_identity_sha256.as_str(),
        plan.dataset_fingerprint.as_str(),
        plan.design_identity_sha256.as_str(),
        plan.parameter_inventory_sha256.as_str(),
        plan.seed,
        plan.retry_policy.as_str(),
        plan.shards.as_slice(),
    ))
}

/// Builds the canonical task graph. `pairs` must already reflect the frozen
/// comparison plan; their direction is retained for A-minus-B result identity.
pub fn build_mga_execution_plan_v1(
    compilation: &MultiModCompilationReceiptV1,
    dataset_fingerprint: &str,
    config: &MgaMultigroupV1,
    design: &MultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    pairs: &[OrderedGroupPairV1],
) -> Result<MgaExecutionPlanV1, MgaExecutionCacheErrorV1> {
    build_mga_execution_plan_from_identity_v1(
        &compilation.analytical_identity_sha256,
        dataset_fingerprint,
        config,
        design,
        parameters,
        pairs,
    )
}

/// Identity-only form for coordinators that already validated the compiler
/// receipt. `analysis_identity_sha256` must be the exact compiled identity.
pub fn build_mga_execution_plan_from_identity_v1(
    analysis_identity_sha256: &str,
    dataset_fingerprint: &str,
    config: &MgaMultigroupV1,
    design: &MultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    pairs: &[OrderedGroupPairV1],
) -> Result<MgaExecutionPlanV1, MgaExecutionCacheErrorV1> {
    if dataset_fingerprint.trim().is_empty()
        || analysis_identity_sha256.len() != 64
        || design.groups.is_empty()
        || parameters.is_empty()
    {
        return Err(MgaExecutionCacheErrorV1::InvalidPlan(
            "analysis, dataset, group, and parameter identities are required".into(),
        ));
    }
    let mut seen_pairs = BTreeSet::new();
    if pairs.iter().any(|pair| {
        pair.group_a == pair.group_b
            || pair.group_a.get() >= design.groups.len()
            || pair.group_b.get() >= design.groups.len()
            || !seen_pairs.insert(canonical_pair(*pair))
    }) {
        return Err(MgaExecutionCacheErrorV1::InvalidPlan(
            "selected pairs must be unique and reference two configured groups".into(),
        ));
    }
    let design_identity_sha256 = sha256_serialized(design);
    let parameter_inventory_sha256 = sha256_serialized(&parameters);
    let config_identity_sha256 = sha256_serialized(config);
    let common_input = sha256_serialized(&(
        analysis_identity_sha256,
        dataset_fingerprint,
        &design_identity_sha256,
        &parameter_inventory_sha256,
        &config_identity_sha256,
        config.seed,
        config.permutation_samples,
        config.bootstrap_samples,
        config.alpha.to_bits(),
        config.confidence_level.to_bits(),
        config.alternative,
        config.multiplicity,
    ));
    let mut kinds = design
        .groups
        .iter()
        .map(|group| MgaExecutionShardKindV1::PointFit { group: group.index })
        .collect::<Vec<_>>();
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        kinds.extend(
            pairs
                .iter()
                .copied()
                .map(|pair| MgaExecutionShardKindV1::MicomPair { pair }),
        );
    }
    if config
        .procedures
        .contains(&MgaProcedureV1::PairwisePermutation)
    {
        kinds.extend(
            pairs
                .iter()
                .copied()
                .map(|pair| MgaExecutionShardKindV1::PairwisePermutation { pair }),
        );
    }
    if config
        .procedures
        .contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation)
    {
        kinds.push(MgaExecutionShardKindV1::OmnibusPermutation);
    }
    let has_bootstrap_bank = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::HenselerPlsMga | MgaProcedureV1::BootstrapDifferenceBc
        )
    });
    if has_bootstrap_bank {
        kinds.push(MgaExecutionShardKindV1::SharedGroupBootstrapBanks);
    }
    for procedure in config.procedures.iter().copied().filter(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::HenselerPlsMga | MgaProcedureV1::BootstrapDifferenceBc
        )
    }) {
        kinds.extend(
            pairs
                .iter()
                .copied()
                .map(|pair| MgaExecutionShardKindV1::PairwiseBootstrapDerived { procedure, pair }),
        );
    }
    for procedure in config.procedures.iter().copied().filter(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::ParametricPooledVariance | MgaProcedureV1::ParametricWelchSatterthwaite
        )
    }) {
        kinds.extend(
            pairs
                .iter()
                .copied()
                .map(|pair| MgaExecutionShardKindV1::ParametricPair { procedure, pair }),
        );
    }
    if config
        .procedures
        .contains(&MgaProcedureV1::ParametricWaldOmnibus)
    {
        kinds.push(MgaExecutionShardKindV1::ParametricWaldOmnibus);
    }
    kinds.push(MgaExecutionShardKindV1::MultiplicityAggregation);
    let shards = kinds
        .into_iter()
        .enumerate()
        .map(|(ordinal, kind)| {
            let input_identity_sha256 = sha256_serialized(&(&common_input, &kind));
            let shard_identity_sha256 = sha256_serialized(&(
                MGA_EXECUTION_PLAN_CONTRACT_V1,
                ordinal,
                &kind,
                &input_identity_sha256,
            ));
            MgaExecutionShardV1 {
                ordinal: ordinal as u32,
                kind,
                input_identity_sha256,
                shard_identity_sha256,
            }
        })
        .collect::<Vec<_>>();
    let mut plan = MgaExecutionPlanV1 {
        contract: MGA_EXECUTION_PLAN_CONTRACT_V1.into(),
        execution_scope: "mga_scientific_task_graph_v1".into(),
        analysis_identity_sha256: analysis_identity_sha256.into(),
        dataset_fingerprint: dataset_fingerprint.into(),
        design_identity_sha256,
        parameter_inventory_sha256,
        seed: config.seed,
        retry_policy: "none".into(),
        shards,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = plan_sha256(&plan);
    plan.ensure_valid()?;
    Ok(plan)
}

impl MgaExecutionPlanV1 {
    pub fn ensure_valid(&self) -> Result<(), MgaExecutionCacheErrorV1> {
        let unique = self
            .shards
            .iter()
            .map(|shard| shard.shard_identity_sha256.as_str())
            .collect::<BTreeSet<_>>();
        if self.contract != MGA_EXECUTION_PLAN_CONTRACT_V1
            || self.execution_scope != "mga_scientific_task_graph_v1"
            || self.retry_policy != "none"
            || !is_sha256(&self.analysis_identity_sha256)
            || self.dataset_fingerprint.trim().is_empty()
            || !is_sha256(&self.design_identity_sha256)
            || !is_sha256(&self.parameter_inventory_sha256)
            || self.shards.is_empty()
            || self.shards.iter().enumerate().any(|(index, shard)| {
                shard.ordinal as usize != index
                    || !is_sha256(&shard.input_identity_sha256)
                    || !is_sha256(&shard.shard_identity_sha256)
            })
            || unique.len() != self.shards.len()
            || !is_sha256(&self.plan_sha256)
            || self.plan_sha256 != plan_sha256(self)
        {
            return Err(MgaExecutionCacheErrorV1::InvalidPlan(
                "contract, ordering, identity, retry policy, or plan digest differs".into(),
            ));
        }
        Ok(())
    }

    pub fn shard(&self, kind: &MgaExecutionShardKindV1) -> Option<&MgaExecutionShardV1> {
        self.shards.iter().find(|shard| &shard.kind == kind)
    }

    fn prerequisite_shards(&self, kind: &MgaExecutionShardKindV1) -> Vec<&MgaExecutionShardV1> {
        if matches!(kind, MgaExecutionShardKindV1::PointFit { .. }) {
            return Vec::new();
        }
        if matches!(kind, MgaExecutionShardKindV1::MultiplicityAggregation) {
            return self
                .shards
                .iter()
                .filter(|shard| shard.kind != MgaExecutionShardKindV1::MultiplicityAggregation)
                .collect();
        }
        self.shards
            .iter()
            .filter(|shard| {
                matches!(&shard.kind, MgaExecutionShardKindV1::PointFit { .. })
                    || (matches!(
                        kind,
                        MgaExecutionShardKindV1::PairwiseBootstrapDerived { .. }
                    ) && shard.kind == MgaExecutionShardKindV1::SharedGroupBootstrapBanks)
            })
            .collect()
    }
}

impl MgaExecutionCacheV1 {
    pub fn empty(plan: &MgaExecutionPlanV1) -> Result<Self, MgaExecutionCacheErrorV1> {
        plan.ensure_valid()?;
        Ok(Self {
            contract: MGA_EXECUTION_CACHE_CONTRACT_V1.into(),
            plan_sha256: plan.plan_sha256.clone(),
            entries: Vec::new(),
        })
    }

    pub fn ensure_valid(&self, plan: &MgaExecutionPlanV1) -> Result<(), MgaExecutionCacheErrorV1> {
        plan.ensure_valid()?;
        if self.contract != MGA_EXECUTION_CACHE_CONTRACT_V1 || self.plan_sha256 != plan.plan_sha256
        {
            return Err(MgaExecutionCacheErrorV1::InvalidCache(
                "cache contract or plan identity differs".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        for entry in &self.entries {
            if !identities.insert(entry.shard_identity_sha256.as_str()) {
                return Err(MgaExecutionCacheErrorV1::InvalidCache(
                    "cache contains a duplicate completed shard".into(),
                ));
            }
            let shard = plan
                .shards
                .iter()
                .find(|shard| shard.shard_identity_sha256 == entry.shard_identity_sha256)
                .ok_or_else(|| {
                    MgaExecutionCacheErrorV1::InvalidCache(
                        "cache contains a shard outside the immutable plan".into(),
                    )
                })?;
            if !entry.payload.matches_kind(&shard.kind)
                || !is_sha256(&entry.payload_sha256)
                || entry.payload_sha256 != sha256_serialized(&entry.payload)
            {
                return Err(MgaExecutionCacheErrorV1::InvalidPayload(
                    "payload type or SHA-256 does not match its planned shard".into(),
                ));
            }
        }
        for entry in &self.entries {
            let shard = plan
                .shards
                .iter()
                .find(|shard| shard.shard_identity_sha256 == entry.shard_identity_sha256)
                .expect("planned cache entry resolved above");
            if plan
                .prerequisite_shards(&shard.kind)
                .iter()
                .any(|dependency| !identities.contains(dependency.shard_identity_sha256.as_str()))
            {
                return Err(MgaExecutionCacheErrorV1::InvalidCache(
                    "completed shard is present without every planned prerequisite".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn payload<'a>(
        &'a self,
        plan: &MgaExecutionPlanV1,
        kind: &MgaExecutionShardKindV1,
    ) -> Result<Option<&'a MgaExecutionShardPayloadV1>, MgaExecutionCacheErrorV1> {
        self.ensure_valid(plan)?;
        let shard = plan.shard(kind).ok_or_else(|| {
            MgaExecutionCacheErrorV1::InvalidPlan("requested shard is not planned".into())
        })?;
        Ok(self
            .entries
            .iter()
            .find(|entry| entry.shard_identity_sha256 == shard.shard_identity_sha256)
            .map(|entry| &entry.payload))
    }

    /// Inserts a completed shard once. A failed/cancelled computation has no
    /// payload and therefore is never inserted or retried under another draw.
    pub fn insert(
        &mut self,
        plan: &MgaExecutionPlanV1,
        kind: &MgaExecutionShardKindV1,
        payload: MgaExecutionShardPayloadV1,
    ) -> Result<(), MgaExecutionCacheErrorV1> {
        self.ensure_valid(plan)?;
        let shard = plan.shard(kind).ok_or_else(|| {
            MgaExecutionCacheErrorV1::InvalidPlan("completed shard is not planned".into())
        })?;
        if !payload.matches_kind(kind) {
            return Err(MgaExecutionCacheErrorV1::InvalidPayload(
                "completed payload type or identity differs from the planned shard".into(),
            ));
        }
        if plan.prerequisite_shards(kind).iter().any(|dependency| {
            !self
                .entries
                .iter()
                .any(|entry| entry.shard_identity_sha256 == dependency.shard_identity_sha256)
        }) {
            return Err(MgaExecutionCacheErrorV1::Incomplete(
                "cannot commit a shard before every planned prerequisite is complete".into(),
            ));
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.shard_identity_sha256 == shard.shard_identity_sha256)
        {
            return Err(MgaExecutionCacheErrorV1::InvalidCache(
                "no-retry cache refuses to replace a completed shard".into(),
            ));
        }
        self.entries.push(MgaExecutionCacheEntryV1 {
            shard_identity_sha256: shard.shard_identity_sha256.clone(),
            payload_sha256: sha256_serialized(&payload),
            payload,
        });
        self.entries.sort_by_key(|entry| {
            plan.shards
                .iter()
                .position(|shard| shard.shard_identity_sha256 == entry.shard_identity_sha256)
                .expect("inserted planned shard")
        });
        self.ensure_valid(plan)
    }

    pub fn pending_kinds(
        &self,
        plan: &MgaExecutionPlanV1,
    ) -> Result<Vec<MgaExecutionShardKindV1>, MgaExecutionCacheErrorV1> {
        self.ensure_valid(plan)?;
        let completed = self
            .entries
            .iter()
            .map(|entry| entry.shard_identity_sha256.as_str())
            .collect::<BTreeSet<_>>();
        Ok(plan
            .shards
            .iter()
            .filter(|shard| !completed.contains(shard.shard_identity_sha256.as_str()))
            .map(|shard| shard.kind.clone())
            .collect())
    }

    pub fn finalized_identity_sha256(
        &self,
        plan: &MgaExecutionPlanV1,
    ) -> Result<String, MgaExecutionCacheErrorV1> {
        let pending = self.pending_kinds(plan)?;
        if !pending.is_empty() {
            return Err(MgaExecutionCacheErrorV1::Incomplete(format!(
                "{} planned shards remain; no scientific result may be published",
                pending.len()
            )));
        }
        let payloads = self
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.shard_identity_sha256.as_str(),
                    entry.payload_sha256.as_str(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        Ok(sha256_serialized(&(
            MGA_EXECUTION_CACHE_CONTRACT_V1,
            plan.plan_sha256.as_str(),
            payloads,
        )))
    }
}

impl<'a> ValidatedMgaExecutionCacheSessionV1<'a> {
    /// Opens an exclusive fast-path session after one full payload-integrity
    /// validation. No unchecked cache can enter the session.
    pub(crate) fn open(
        plan: &'a MgaExecutionPlanV1,
        cache: &'a mut MgaExecutionCacheV1,
    ) -> Result<Self, MgaExecutionCacheErrorV1> {
        cache.ensure_valid(plan)?;
        let mut shard_identity_by_kind = BTreeMap::new();
        for shard in &plan.shards {
            // Preserve `MgaExecutionPlanV1::shard`'s first-match semantics.
            shard_identity_by_kind
                .entry(shard.kind.clone())
                .or_insert_with(|| shard.shard_identity_sha256.clone());
        }
        let ordinal_by_shard_identity = plan
            .shards
            .iter()
            .enumerate()
            .map(|(ordinal, shard)| (shard.shard_identity_sha256.clone(), ordinal))
            .collect::<BTreeMap<_, _>>();
        let entry_index_by_shard_identity = cache
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.shard_identity_sha256.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let completed_shard_identities = entry_index_by_shard_identity
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        Ok(Self {
            plan,
            cache,
            shard_identity_by_kind,
            ordinal_by_shard_identity,
            entry_index_by_shard_identity,
            completed_shard_identities,
        })
    }

    /// Returns a previously validated payload without rehashing historical
    /// payloads. The session's exclusive borrow keeps the index authoritative.
    pub(crate) fn payload(
        &self,
        kind: &MgaExecutionShardKindV1,
    ) -> Result<Option<&MgaExecutionShardPayloadV1>, MgaExecutionCacheErrorV1> {
        let shard_identity = self.shard_identity_by_kind.get(kind).ok_or_else(|| {
            MgaExecutionCacheErrorV1::InvalidPlan("requested shard is not planned".into())
        })?;
        Ok(self
            .entry_index_by_shard_identity
            .get(shard_identity)
            .map(|index| &self.cache.entries[*index].payload))
    }

    /// Inserts one new shard while validating only the new payload and its
    /// dependency boundary. Historical payloads were authenticated by `open`
    /// and remain immutable for the lifetime of this session.
    pub(crate) fn insert(
        &mut self,
        kind: &MgaExecutionShardKindV1,
        payload: MgaExecutionShardPayloadV1,
    ) -> Result<(), MgaExecutionCacheErrorV1> {
        let shard = self.plan.shard(kind).ok_or_else(|| {
            MgaExecutionCacheErrorV1::InvalidPlan("completed shard is not planned".into())
        })?;
        if self
            .completed_shard_identities
            .contains(&shard.shard_identity_sha256)
        {
            return Err(MgaExecutionCacheErrorV1::InvalidCache(
                "no-retry cache refuses to replace a completed shard".into(),
            ));
        }
        if !payload.matches_kind(kind) {
            return Err(MgaExecutionCacheErrorV1::InvalidPayload(
                "completed payload type or identity differs from the planned shard".into(),
            ));
        }
        if self
            .plan
            .prerequisite_shards(kind)
            .iter()
            .any(|dependency| {
                !self
                    .completed_shard_identities
                    .contains(&dependency.shard_identity_sha256)
            })
        {
            return Err(MgaExecutionCacheErrorV1::Incomplete(
                "cannot commit a shard before every planned prerequisite is complete".into(),
            ));
        }

        let entry = MgaExecutionCacheEntryV1 {
            shard_identity_sha256: shard.shard_identity_sha256.clone(),
            payload_sha256: sha256_serialized(&payload),
            payload,
        };
        self.cache.entries.push(entry);
        self.cache.entries.sort_by_key(|entry| {
            self.ordinal_by_shard_identity
                .get(&entry.shard_identity_sha256)
                .copied()
                .expect("session contains only fully planned shard identities")
        });
        self.completed_shard_identities
            .insert(shard.shard_identity_sha256.clone());
        self.rebuild_entry_index();
        Ok(())
    }

    /// Immutable cache exposure for transactional checkpoint callbacks.
    pub(crate) fn cache(&self) -> &MgaExecutionCacheV1 {
        self.cache
    }

    /// Immutable plan authority paired with this validated cache session.
    pub(crate) fn plan(&self) -> &MgaExecutionPlanV1 {
        self.plan
    }

    /// Produces the existing finalized identity after one final full audit.
    pub(crate) fn finalized_identity_sha256(&self) -> Result<String, MgaExecutionCacheErrorV1> {
        self.cache.finalized_identity_sha256(self.plan)
    }

    fn rebuild_entry_index(&mut self) {
        self.entry_index_by_shard_identity = self
            .cache
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.shard_identity_sha256.clone(), index))
            .collect();
    }
}

/// Runs one immutable shard transactionally. Completed payloads are reused;
/// Failed or unfinished attempts are never inserted. A shard that finishes
/// successfully while cancellation arrives is committed before cancellation
/// is returned. An unfinished shard may be invoked again after restart, but it
/// consumes the identical frozen draw plan and never replaces failed draws or
/// a completed payload.
pub fn execute_or_reuse_mga_shard_v1<F, C>(
    plan: &MgaExecutionPlanV1,
    cache: &mut MgaExecutionCacheV1,
    kind: &MgaExecutionShardKindV1,
    should_cancel: C,
    execute: F,
) -> Result<MgaExecutionShardPayloadV1, MgaExecutionCacheErrorV1>
where
    F: FnOnce() -> Result<MgaExecutionShardPayloadV1, MgaExecutionCacheErrorV1>,
    C: Fn() -> bool,
{
    let mut no_checkpoint = |_: &MgaExecutionPlanV1, _: &MgaExecutionCacheV1| Ok(());
    execute_or_reuse_mga_shard_with_checkpoint_v1(
        plan,
        cache,
        kind,
        should_cancel,
        execute,
        &mut no_checkpoint,
    )
}

/// Executes one immutable shard and checkpoints only after the insertion has
/// passed full cache validation. A cancellation arriving during a successful
/// execution is returned only after that shard is committed and checkpointed.
/// Reused shards do not trigger a checkpoint, and a checkpoint failure aborts
/// the run before any result can be published.
pub fn execute_or_reuse_mga_shard_with_checkpoint_v1<F, C, Q>(
    plan: &MgaExecutionPlanV1,
    cache: &mut MgaExecutionCacheV1,
    kind: &MgaExecutionShardKindV1,
    should_cancel: C,
    execute: F,
    checkpoint: &mut Q,
) -> Result<MgaExecutionShardPayloadV1, MgaExecutionCacheErrorV1>
where
    F: FnOnce() -> Result<MgaExecutionShardPayloadV1, MgaExecutionCacheErrorV1>,
    C: Fn() -> bool,
    Q: FnMut(&MgaExecutionPlanV1, &MgaExecutionCacheV1) -> Result<(), String> + ?Sized,
{
    if should_cancel() {
        return Err(MgaExecutionCacheErrorV1::Cancelled);
    }
    if let Some(payload) = cache.payload(plan, kind)? {
        return Ok(payload.clone());
    }
    let payload = execute()?;
    cache.insert(plan, kind, payload.clone())?;
    cache.ensure_valid(plan)?;
    checkpoint(plan, cache).map_err(MgaExecutionCacheErrorV1::CheckpointFailed)?;
    if should_cancel() {
        return Err(MgaExecutionCacheErrorV1::Cancelled);
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        InferenceAlternativeV1, MgaComparisonPlanV1, MgaModelProfileV1, MicomConfiguralChecklistV1,
        MultiplicityAdjustmentV1, SelectedGroupV1, TypedGroupValueV1,
    };
    use qpls_estimation::{
        GroupIdentityV1, MultigroupFitRequestV1, MultigroupResamplingConfigV1, ParameterEstimateV1,
        ParameterFamilyV1, ParameterVectorV1, SelectedGroupRowV1, TypedGroupValueV1 as EstGroup,
        run_max_spread_omnibus_permutation_v1,
    };

    fn group(index: usize, label: &str) -> GroupIdentityV1 {
        GroupIdentityV1 {
            index: GroupIndexV1::new(index).unwrap(),
            value: EstGroup::Text {
                value: label.into(),
            },
            display_label: label.into(),
        }
    }

    fn fixture() -> (
        MgaMultigroupV1,
        MultigroupDesignV1,
        Vec<ParameterIdentityV1>,
        Vec<OrderedGroupPairV1>,
    ) {
        let config = MgaMultigroupV1 {
            schema_version: 1,
            profile: MgaModelProfileV1::GeneralSemPls,
            grouping_column: "group".into(),
            groups: vec![
                SelectedGroupV1 {
                    group_id: "a".into(),
                    label: "A".into(),
                    value: TypedGroupValueV1::Text { value: "A".into() },
                },
                SelectedGroupV1 {
                    group_id: "b".into(),
                    label: "B".into(),
                    value: TypedGroupValueV1::Text { value: "B".into() },
                },
            ],
            comparison_plan: MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: false,
            },
            procedures: vec![MgaProcedureV1::ParametricWaldOmnibus],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: MicomConfiguralChecklistV1 {
                identical_indicators_and_coding: true,
                identical_data_treatment: true,
                identical_algorithm_settings: true,
                identical_model_specification: true,
                deterministic_sign_orientation_reviewed: true,
                analyst_review_confirmed: true,
            },
            weight: None,
            selected_parameter_ids: vec!["path:x->y".into()],
        };
        let design = MultigroupDesignV1 {
            groups: vec![group(0, "A"), group(1, "B")],
            rows: Vec::new(),
        };
        let parameters = vec![ParameterIdentityV1 {
            stable_id: "path:x->y".into(),
            family: ParameterFamilyV1::StructuralPath,
        }];
        let pairs = vec![
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap(),
        ];
        (config, design, parameters, pairs)
    }

    fn plan() -> MgaExecutionPlanV1 {
        let (config, design, parameters, pairs) = fixture();
        build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:test-dataset",
            &config,
            &design,
            &parameters,
            &pairs,
        )
        .unwrap()
    }

    fn point_payload(group: GroupIndexV1, estimate: f64) -> MgaExecutionShardPayloadV1 {
        MgaExecutionShardPayloadV1::PointFit {
            value: GroupParameterVectorV1 {
                group,
                values: vec![estimate],
            },
            ordinary_path_standard_errors: Vec::new(),
        }
    }

    fn omnibus_plan_and_payload() -> (MgaExecutionPlanV1, MgaExecutionShardPayloadV1) {
        let (mut config, mut design, parameters, _) = fixture();
        config.groups.push(SelectedGroupV1 {
            group_id: "c".into(),
            label: "C".into(),
            value: TypedGroupValueV1::Text { value: "C".into() },
        });
        config.procedures = vec![MgaProcedureV1::OmnibusMaxSpreadPermutation];
        design.groups.push(group(2, "C"));
        design.rows = (0..30)
            .map(|source_row| SelectedGroupRowV1 {
                source_row,
                stable_row_token: source_row,
                group: GroupIndexV1::new((source_row / 10) as usize).unwrap(),
            })
            .collect();
        let pairs = vec![
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(1).unwrap())
                .unwrap(),
            OrderedGroupPairV1::new(GroupIndexV1::new(0).unwrap(), GroupIndexV1::new(2).unwrap())
                .unwrap(),
            OrderedGroupPairV1::new(GroupIndexV1::new(1).unwrap(), GroupIndexV1::new(2).unwrap())
                .unwrap(),
        ];
        let plan = build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:test-dataset",
            &config,
            &design,
            &parameters,
            &pairs,
        )
        .unwrap();
        let parameter = parameters[0].clone();
        let mut refitter = |request: &MultigroupFitRequestV1| {
            let estimate =
                request.source_rows.iter().sum::<u64>() as f64 / request.source_rows.len() as f64;
            Ok(ParameterVectorV1 {
                parameters: vec![ParameterEstimateV1 {
                    parameter: parameter.clone(),
                    estimate,
                }],
            })
        };
        let result = run_max_spread_omnibus_permutation_v1(
            &design,
            &parameters,
            MultigroupResamplingConfigV1::official_defaults(),
            &mut refitter,
        )
        .unwrap();
        (
            plan,
            MgaExecutionShardPayloadV1::OmnibusPermutation { value: result },
        )
    }

    #[test]
    fn validated_session_rejects_tampered_cache_on_open() {
        let plan = plan();
        let kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        cache
            .insert(
                &plan,
                &kind,
                point_payload(GroupIndexV1::new(0).unwrap(), 0.25),
            )
            .unwrap();
        cache.entries[0].payload_sha256 = "0".repeat(64);

        assert!(matches!(
            ValidatedMgaExecutionCacheSessionV1::open(&plan, &mut cache),
            Err(MgaExecutionCacheErrorV1::InvalidPayload(_))
        ));
    }

    #[test]
    fn validated_session_indexes_reuse_and_checks_only_the_new_shard_boundary() {
        let plan = plan();
        let first_kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let second_kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(1).unwrap(),
        };
        let expected = point_payload(GroupIndexV1::new(0).unwrap(), 0.25);
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        cache.insert(&plan, &first_kind, expected.clone()).unwrap();

        let mut session = ValidatedMgaExecutionCacheSessionV1::open(&plan, &mut cache).unwrap();
        assert_eq!(session.plan().plan_sha256, plan.plan_sha256);
        assert_eq!(session.payload(&first_kind).unwrap(), Some(&expected));
        assert_eq!(session.cache().entries.len(), 1);

        assert!(matches!(
            session.insert(
                &MgaExecutionShardKindV1::ParametricWaldOmnibus,
                MgaExecutionShardPayloadV1::ParametricWaldOmnibus {
                    output_identity_sha256: "b".repeat(64),
                    tests: Vec::new(),
                },
            ),
            Err(MgaExecutionCacheErrorV1::Incomplete(_))
        ));
        assert!(matches!(
            session.insert(
                &second_kind,
                point_payload(GroupIndexV1::new(0).unwrap(), 0.5),
            ),
            Err(MgaExecutionCacheErrorV1::InvalidPayload(_))
        ));

        let second_payload = point_payload(GroupIndexV1::new(1).unwrap(), 0.5);
        session
            .insert(&second_kind, second_payload.clone())
            .unwrap();
        assert_eq!(
            session.payload(&second_kind).unwrap(),
            Some(&second_payload)
        );
        let second_entry = session
            .cache()
            .entries
            .iter()
            .find(|entry| entry.payload == second_payload)
            .unwrap();
        assert_eq!(
            second_entry.payload_sha256,
            sha256_serialized(&second_payload)
        );
        assert!(matches!(
            session.insert(&second_kind, second_payload),
            Err(MgaExecutionCacheErrorV1::InvalidCache(_))
        ));
        session.cache().ensure_valid(session.plan()).unwrap();
    }

    #[test]
    fn validated_session_preserves_cache_serialization_and_final_identity() {
        let plan = plan();
        let payloads = plan
            .shards
            .iter()
            .map(|shard| {
                let payload = match &shard.kind {
                    MgaExecutionShardKindV1::PointFit { group } => {
                        point_payload(*group, group.get() as f64)
                    }
                    MgaExecutionShardKindV1::ParametricWaldOmnibus => {
                        MgaExecutionShardPayloadV1::ParametricWaldOmnibus {
                            output_identity_sha256: "b".repeat(64),
                            tests: Vec::new(),
                        }
                    }
                    MgaExecutionShardKindV1::MultiplicityAggregation => {
                        MgaExecutionShardPayloadV1::MultiplicityAggregation {
                            input_rows_sha256: "c".repeat(64),
                            rows: Vec::new(),
                        }
                    }
                    other => panic!("unexpected fixture shard {other:?}"),
                };
                (shard.kind.clone(), payload)
            })
            .collect::<Vec<_>>();

        let mut legacy_cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        for (kind, payload) in payloads.iter().cloned() {
            legacy_cache.insert(&plan, &kind, payload).unwrap();
        }
        let legacy_identity = legacy_cache.finalized_identity_sha256(&plan).unwrap();
        let legacy_json = serde_json::to_vec(&legacy_cache).unwrap();

        let mut session_cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        let session_identity;
        {
            let mut session =
                ValidatedMgaExecutionCacheSessionV1::open(&plan, &mut session_cache).unwrap();
            for (kind, payload) in payloads {
                session.insert(&kind, payload).unwrap();
            }
            assert_eq!(session.cache().entries.len(), plan.shards.len());
            session_identity = session.finalized_identity_sha256().unwrap();
        }

        assert_eq!(session_identity, legacy_identity);
        assert_eq!(serde_json::to_vec(&session_cache).unwrap(), legacy_json);
        assert_eq!(session_cache, legacy_cache);
    }

    #[test]
    fn cancellation_does_not_commit_and_resume_reuses_completed_shard() {
        let plan = plan();
        let kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        let cancelled = execute_or_reuse_mga_shard_v1(
            &plan,
            &mut cache,
            &kind,
            || true,
            || Ok(point_payload(GroupIndexV1::new(0).unwrap(), 0.25)),
        )
        .unwrap_err();
        assert_eq!(cancelled, MgaExecutionCacheErrorV1::Cancelled);
        assert!(cache.entries.is_empty());

        let expected = point_payload(GroupIndexV1::new(0).unwrap(), 0.25);
        execute_or_reuse_mga_shard_v1(&plan, &mut cache, &kind, || false, || Ok(expected.clone()))
            .unwrap();
        let serialized = serde_json::to_string(&cache).unwrap();
        let mut reopened: MgaExecutionCacheV1 = serde_json::from_str(&serialized).unwrap();
        let reused = execute_or_reuse_mga_shard_v1(
            &plan,
            &mut reopened,
            &kind,
            || false,
            || panic!("completed shard must not execute twice"),
        )
        .unwrap();
        assert_eq!(reused, expected);
    }

    #[test]
    fn checkpoint_runs_after_validated_insert_but_not_after_reuse() {
        let plan = plan();
        let kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        let checkpoints = std::cell::Cell::new(0usize);
        let mut checkpoint = |checkpoint_plan: &MgaExecutionPlanV1,
                              checkpoint_cache: &MgaExecutionCacheV1| {
            checkpoint_cache
                .ensure_valid(checkpoint_plan)
                .map_err(|error| error.to_string())?;
            assert_eq!(checkpoint_cache.entries.len(), 1);
            checkpoints.set(checkpoints.get() + 1);
            Ok(())
        };
        let expected = point_payload(GroupIndexV1::new(0).unwrap(), 0.25);
        execute_or_reuse_mga_shard_with_checkpoint_v1(
            &plan,
            &mut cache,
            &kind,
            || false,
            || Ok(expected.clone()),
            &mut checkpoint,
        )
        .unwrap();
        assert_eq!(checkpoints.get(), 1);
        execute_or_reuse_mga_shard_with_checkpoint_v1(
            &plan,
            &mut cache,
            &kind,
            || false,
            || panic!("completed shard must not execute twice"),
            &mut checkpoint,
        )
        .unwrap();
        assert_eq!(checkpoints.get(), 1);

        let second_kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(1).unwrap(),
        };
        let mut failing_checkpoint = |_: &MgaExecutionPlanV1, _: &MgaExecutionCacheV1| {
            Err("atomic persistence failed".into())
        };
        assert!(matches!(
            execute_or_reuse_mga_shard_with_checkpoint_v1(
                &plan,
                &mut cache,
                &second_kind,
                || false,
                || Ok(point_payload(GroupIndexV1::new(1).unwrap(), 0.5)),
                &mut failing_checkpoint,
            ),
            Err(MgaExecutionCacheErrorV1::CheckpointFailed(detail))
                if detail == "atomic persistence failed"
        ));
        assert_eq!(cache.entries.len(), 2);
        cache.ensure_valid(&plan).unwrap();
    }

    #[test]
    fn cancellation_after_success_checkpoints_once_and_resume_reuses_the_shard() {
        let plan = plan();
        let kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        let cancelled = std::cell::Cell::new(false);
        let checkpoints = std::cell::Cell::new(0usize);
        let mut checkpoint = |checkpoint_plan: &MgaExecutionPlanV1,
                              checkpoint_cache: &MgaExecutionCacheV1| {
            checkpoint_cache
                .ensure_valid(checkpoint_plan)
                .map_err(|error| error.to_string())?;
            checkpoints.set(checkpoints.get() + 1);
            Ok(())
        };
        let expected = point_payload(GroupIndexV1::new(0).unwrap(), 0.25);
        let outcome = execute_or_reuse_mga_shard_with_checkpoint_v1(
            &plan,
            &mut cache,
            &kind,
            || cancelled.get(),
            || {
                cancelled.set(true);
                Ok(expected.clone())
            },
            &mut checkpoint,
        );
        assert_eq!(outcome.unwrap_err(), MgaExecutionCacheErrorV1::Cancelled);
        assert_eq!(checkpoints.get(), 1);
        assert_eq!(cache.entries.len(), 1);
        cache.ensure_valid(&plan).unwrap();

        cancelled.set(false);
        let resumed = execute_or_reuse_mga_shard_with_checkpoint_v1(
            &plan,
            &mut cache,
            &kind,
            || cancelled.get(),
            || panic!("checkpointed shard must not execute again"),
            &mut checkpoint,
        )
        .unwrap();
        assert_eq!(resumed, expected);
        assert_eq!(checkpoints.get(), 1);
    }

    #[test]
    fn tamper_and_completed_shard_replacement_fail_closed() {
        let plan = plan();
        let kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        cache
            .insert(
                &plan,
                &kind,
                point_payload(GroupIndexV1::new(0).unwrap(), 0.25),
            )
            .unwrap();
        assert!(matches!(
            cache.insert(
                &plan,
                &kind,
                point_payload(GroupIndexV1::new(0).unwrap(), 0.5),
            ),
            Err(MgaExecutionCacheErrorV1::InvalidCache(_))
        ));
        cache.entries[0].payload_sha256 = "0".repeat(64);
        assert!(matches!(
            cache.ensure_valid(&plan),
            Err(MgaExecutionCacheErrorV1::InvalidPayload(_))
        ));
    }

    #[test]
    fn omnibus_cache_roundtrip_accepts_valid_payload_and_rejects_null_vector_tamper() {
        let (plan, payload) = omnibus_plan_and_payload();
        let kind = MgaExecutionShardKindV1::OmnibusPermutation;
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        let MgaExecutionShardPayloadV1::OmnibusPermutation { value } = &payload else {
            unreachable!("fixture returns an omnibus payload")
        };
        for point in &value.group_point_estimates {
            cache
                .insert(
                    &plan,
                    &MgaExecutionShardKindV1::PointFit { group: point.group },
                    point_payload(point.group, point.values[0]),
                )
                .unwrap();
        }
        cache.insert(&plan, &kind, payload.clone()).unwrap();

        let serialized = serde_json::to_string(&cache).unwrap();
        let reopened: MgaExecutionCacheV1 = serde_json::from_str(&serialized).unwrap();
        reopened.ensure_valid(&plan).unwrap();
        assert_eq!(reopened.payload(&plan, &kind).unwrap(), Some(&payload));

        let mut tampered = reopened;
        let entry = tampered
            .entries
            .iter_mut()
            .find(|entry| {
                matches!(
                    &entry.payload,
                    MgaExecutionShardPayloadV1::OmnibusPermutation { .. }
                )
            })
            .unwrap();
        let MgaExecutionShardPayloadV1::OmnibusPermutation { value } = &mut entry.payload else {
            unreachable!("located omnibus cache entry")
        };
        value.parameters[0].null_maximum_pairwise_spreads.pop();
        assert!(matches!(
            tampered.ensure_valid(&plan),
            Err(MgaExecutionCacheErrorV1::InvalidPayload(_))
        ));
    }

    #[test]
    fn aggregation_identity_is_unavailable_until_every_shard_is_complete() {
        let plan = plan();
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        assert!(matches!(
            cache.finalized_identity_sha256(&plan),
            Err(MgaExecutionCacheErrorV1::Incomplete(_))
        ));
        for shard in &plan.shards {
            let payload = match &shard.kind {
                MgaExecutionShardKindV1::PointFit { group } => {
                    point_payload(*group, group.get() as f64)
                }
                MgaExecutionShardKindV1::ParametricWaldOmnibus => {
                    MgaExecutionShardPayloadV1::ParametricWaldOmnibus {
                        output_identity_sha256: "b".repeat(64),
                        tests: Vec::new(),
                    }
                }
                MgaExecutionShardKindV1::MultiplicityAggregation => {
                    MgaExecutionShardPayloadV1::MultiplicityAggregation {
                        input_rows_sha256: "c".repeat(64),
                        rows: Vec::new(),
                    }
                }
                other => panic!("unexpected fixture shard {other:?}"),
            };
            cache.insert(&plan, &shard.kind, payload).unwrap();
        }
        let identity = cache.finalized_identity_sha256(&plan).unwrap();
        assert_eq!(identity.len(), 64);
        assert!(cache.pending_kinds(&plan).unwrap().is_empty());
    }

    #[test]
    fn plan_identity_binds_full_config_design_dataset_and_procedure_inventory() {
        let (mut config, mut design, parameters, pairs) = fixture();
        let baseline = build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:test-dataset",
            &config,
            &design,
            &parameters,
            &pairs,
        )
        .unwrap();
        config.procedures = vec![MgaProcedureV1::PairwisePermutation];
        let changed_procedure = build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:test-dataset",
            &config,
            &design,
            &parameters,
            &pairs,
        )
        .unwrap();
        assert_ne!(baseline.plan_sha256, changed_procedure.plan_sha256);

        config.procedures = vec![MgaProcedureV1::ParametricWaldOmnibus];
        design.groups[0].display_label = "A changed".into();
        let changed_design = build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:test-dataset",
            &config,
            &design,
            &parameters,
            &pairs,
        )
        .unwrap();
        assert_ne!(baseline.plan_sha256, changed_design.plan_sha256);

        let changed_dataset = build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:other-dataset",
            &config,
            &fixture().1,
            &parameters,
            &pairs,
        )
        .unwrap();
        assert_ne!(baseline.plan_sha256, changed_dataset.plan_sha256);
        let cache = MgaExecutionCacheV1::empty(&baseline).unwrap();
        assert!(matches!(
            cache.ensure_valid(&changed_procedure),
            Err(MgaExecutionCacheErrorV1::InvalidCache(_))
        ));
    }

    #[test]
    fn bootstrap_derived_shards_follow_and_require_the_shared_bank() {
        let (mut config, design, parameters, pairs) = fixture();
        config.procedures = vec![MgaProcedureV1::BootstrapDifferenceBc];
        let plan = build_mga_execution_plan_from_identity_v1(
            &"a".repeat(64),
            "v2:test-dataset",
            &config,
            &design,
            &parameters,
            &pairs,
        )
        .unwrap();
        let bank_ordinal = plan
            .shard(&MgaExecutionShardKindV1::SharedGroupBootstrapBanks)
            .unwrap()
            .ordinal;
        let derived_kind = MgaExecutionShardKindV1::PairwiseBootstrapDerived {
            procedure: MgaProcedureV1::BootstrapDifferenceBc,
            pair: pairs[0],
        };
        assert!(bank_ordinal < plan.shard(&derived_kind).unwrap().ordinal);

        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        for group in &design.groups {
            cache
                .insert(
                    &plan,
                    &MgaExecutionShardKindV1::PointFit { group: group.index },
                    point_payload(group.index, group.index.get() as f64),
                )
                .unwrap();
        }
        assert!(matches!(
            cache.insert(
                &plan,
                &derived_kind,
                MgaExecutionShardPayloadV1::PairwiseRows {
                    procedure: MgaProcedureV1::BootstrapDifferenceBc,
                    pair: pairs[0],
                    rows: Vec::new(),
                },
            ),
            Err(MgaExecutionCacheErrorV1::Incomplete(_))
        ));
    }

    #[test]
    fn cancellation_is_honored_before_reusing_a_completed_shard() {
        let plan = plan();
        let kind = MgaExecutionShardKindV1::PointFit {
            group: GroupIndexV1::new(0).unwrap(),
        };
        let mut cache = MgaExecutionCacheV1::empty(&plan).unwrap();
        cache
            .insert(
                &plan,
                &kind,
                point_payload(GroupIndexV1::new(0).unwrap(), 0.25),
            )
            .unwrap();
        assert_eq!(
            execute_or_reuse_mga_shard_v1(
                &plan,
                &mut cache,
                &kind,
                || true,
                || { panic!("cancelled cache reuse must not execute") }
            )
            .unwrap_err(),
            MgaExecutionCacheErrorV1::Cancelled
        );
    }
}
