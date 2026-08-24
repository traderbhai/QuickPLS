//! Raw-data General-SEM conditional-process V2 execution.
//!
//! This module is additive. It does not route through, or widen, the bounded
//! moderated-mediation V1 implementation. Every point, bootstrap, jackknife,
//! and nested studentized draw is represented as one complete scientific
//! refit. The ordered target vector is assembled once from the compiled
//! MultiMod authority and reused without target discovery in resamples.

use crate::{
    MultiModRunOutputV1, MultiModRunnerErrorV1, MultiModRunnerEvidenceV1, MultiModRunnerPhaseV1,
    MultiModRunnerProgressV1, PreparedConditionalInferenceV2, PreparedReplicateEntryV1,
    PreparedReplicateStatusV1, PreparedSharedReplicateLedgerV1, PreparedTargetReplicatesV1,
    multimod_replicate_seed_v1, provenance, report, run_compiled_multimod_weighted_pls_point_v1,
    run_compiled_pls_recipe_v4, run_compiled_pls_recipe_v4_allowing_isolated, validate_authority,
};
use qpls_core::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4, AnalysisWeightBindingV1,
    CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2, CompiledAnalysisRecipeV4,
    CompiledInteractionIdentityV1, CompiledMultiModPlanV1, CompiledMultiModRecipeV1,
    CompiledPlsPlanV2, CompiledPlsPlanV3, CompiledRecipePlanV4, ConditionalProbeScaleV2,
    ConditionalProcessIntervalV2, ConditionalProcessProfileV2, ConditionalProcessTargetKindV2,
    ConditionalProcessTargetResultV2, ConditionalRawProbeFitMetricReceiptV2,
    ConditionalRawProbeFitScopeV2, ConditionalRawProbeMetricBasisV2,
    GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION, GeneralSemConditionalProcessConfigV2,
    GeneralSemConditionalProcessResultV2, HigherOrderConstructionApproachV4,
    InferenceAlternativeV1, MULTIMOD_SIDECAR_MAX_BYTES_V1, MULTIMOD_SIDECAR_WARN_BYTES_V1,
    MethodConfig, MissingDataPolicy, MultiModAnalysisResultV1, MultiModCompilerTargetV1,
    MultimodCompiledWeightedPlsRecipeV1, MultimodIntervalV1, MultimodReplicateFailureKindV1,
    MultimodReplicateFailureV1, MultimodReplicateLedgerSummaryV1, ObservedScaleV4,
    RecipeV4CompilerTarget, SemDataBindingV4, SemGroupV4, SemModelV4, SemRelationV4, SemVariableV4,
    StructuralRelationRoleV4, TypedGroupValueV1, compile_analysis_recipe_v4,
    compile_multimod_weighted_pls_recipe_v4_v1,
    compile_pls_higher_order_lower_order_projection_multimod_v2,
    compile_pls_higher_order_repeated_stage_projection_multimod_v2,
    compile_pls_higher_order_score_stage_projection_multimod_v2, compile_pls_plan_v3,
    compile_pls_plan_v3_multimod_multiple_hoc_v2, project_general_sem_pls_base_recipe_v1,
    project_general_sem_pls_stage_one_recipe_v1, sha256_serialized,
};
use qpls_data::Dataset;
use qpls_estimation::{
    ConditionalAlternativeV2, ConditionalEdgeFunctionV2, ConditionalLinearCoefficientV2,
    ConditionalPairwiseCoefficientV2, ConditionalPathPolynomialV2, ConditionalProbePointV2,
    ExplicitConditionalPathV2, GeneralSemPlsInteractionPointErrorV1,
    GeneralSemPlsMultipleInteractionPointResultV1, GeneralSemPlsThreeWayPointErrorV1,
    GeneralSemPlsThreeWayPointResultV1, MultimodConditionalRowMassV2,
    MultimodConditionalTwoWayInteractionV2, PlsAliasColumnSpecV1, PlsResult,
    StudentizedOuterReplicateV2, append_pls_alias_columns_v1, bca_interval_v2,
    compile_explicit_conditional_path_v2, conditional_effect_v2,
    estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control,
    estimate_general_sem_pls_three_way_moderation_v1_with_control,
    estimate_multimod_conditional_interactions_v2_with_control, normalize_positive_case_weights_v2,
    percentile_interval_v2, prepare_general_sem_pls_disjoint_hoc_score_dataset_multimod_v2,
    prepare_multimod_case_weight_dataset_v1, prepare_multimod_frequency_count_dataset_v1,
    scalar_index_of_moderated_mediation_v2, studentized_interval_v2,
    validate_positive_frequency_weights_v2,
};
use qpls_resampling::{
    MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1, MultiModBootstrapPlanV1, MultiModCaseBootstrapDrawV1,
    MultiModDeleteOneJackknifeDrawV1, MultiModFinalLedgerV1, MultiModFrequencyBootstrapDrawV1,
    MultiModJackknifePlanV1, MultiModRefitFailureV1, MultiModRefitOutcomeV1, MultiModShardSpecV1,
    MultiModStudentizedFinalLedgerV1, MultiModStudentizedInnerDrawV1, MultiModStudentizedPlanV1,
    finalize_multimod_case_bootstrap_v1, finalize_multimod_delete_one_jackknife_v1,
    finalize_multimod_frequency_bootstrap_v1, finalize_multimod_studentized_v1,
    resample_dataset_columns_v1, run_multimod_case_bootstrap_shard_v1,
    run_multimod_delete_one_jackknife_shard_v1, run_multimod_frequency_bootstrap_shard_v1,
    run_multimod_studentized_shard_v1,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use crate::recipe_v4_general_sem_hoc_point_execution::{
    GeneralSemPlsHocScoreAlignmentReferenceV1, align_general_sem_pls_hoc_result_signs_v1,
};

pub const CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2: &str =
    "qpls.general_sem_conditional_process.raw_full_refit.v2";
pub const CONDITIONAL_PROCESS_REFIT_CONTRACT_V2: &str =
    "qpls.general_sem_conditional_process.full_refitter.v2";
pub const CONDITIONAL_PROCESS_ANALYSIS_MASK_CONTRACT_V2: &str =
    "qpls.general_sem_conditional_process.analysis_row_mask.v2";

pub type ConditionalCaseBootstrapLedgerV2 =
    MultiModFinalLedgerV1<MultiModCaseBootstrapDrawV1, Vec<f64>>;
pub type ConditionalDeleteOneLedgerV2 =
    MultiModFinalLedgerV1<MultiModDeleteOneJackknifeDrawV1, Vec<f64>>;
pub type ConditionalFrequencyBootstrapLedgerV2 =
    MultiModFinalLedgerV1<MultiModFrequencyBootstrapDrawV1, Vec<f64>>;
pub type ConditionalStudentizedLedgerV2 = MultiModStudentizedFinalLedgerV1<Vec<f64>, Vec<f64>>;

/// One mutually exclusive analysis stratum. Ungrouped profiles contain one
/// stratum with `group_id == None`; grouped profiles contain exactly one for
/// every configured group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessAnalysisStratumV2 {
    pub group_id: Option<String>,
    /// Unique source-row indices in ascending dataset order.
    pub source_rows: Vec<u32>,
    /// Raw positive case weights in source-row order. The refit request carries
    /// the per-fit mean-one normalization produced by qpls-resampling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub case_weights: Option<Vec<f64>>,
    /// Positive integer frequencies in compact-row order. Zero-count bootstrap
    /// cells exist only in draw requests; the frozen source frame stays compact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequencies: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessAnalysisFrameV2 {
    pub method_version: String,
    pub dataset_fingerprint: String,
    pub required_source_columns: Vec<String>,
    pub strata: Vec<ConditionalProcessAnalysisStratumV2>,
    pub analysis_row_mask_sha256: String,
    pub excluded_rows: Vec<ConditionalProcessExcludedRowV2>,
}

/// Stable, row-level exclusion receipt. The UI/export layer can aggregate
/// these codes, but persistence retains the exact source-row identities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessExcludedRowV2 {
    pub source_row: u32,
    pub reason: ConditionalProcessExclusionReasonV2,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConditionalProcessExclusionReasonV2 {
    MissingOrNonfiniteModelValue,
    MissingGroupingValue,
    UnselectedGroupingValue,
    MissingOrInvalidWeight,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AnalysisMaskIdentityV2<'a> {
    contract: &'static str,
    dataset_fingerprint: &'a str,
    strata: Vec<AnalysisMaskStratumV2<'a>>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct AnalysisMaskStratumV2<'a> {
    group_id: Option<&'a str>,
    source_rows: &'a [u32],
}

/// Public helper used by authoring and execution to bind raw-unit probe
/// receipts to the exact listwise/grouped/weighted analysis frame.
pub fn conditional_process_analysis_row_mask_sha256_v2(
    dataset_fingerprint: &str,
    strata: &[ConditionalProcessAnalysisStratumV2],
) -> String {
    sha256_serialized(&AnalysisMaskIdentityV2 {
        contract: CONDITIONAL_PROCESS_ANALYSIS_MASK_CONTRACT_V2,
        dataset_fingerprint,
        strata: strata
            .iter()
            .map(|stratum| AnalysisMaskStratumV2 {
                group_id: stratum.group_id.as_deref(),
                source_rows: &stratum.source_rows,
            })
            .collect(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ConditionalProcessRefitSampleV2 {
    CaseRows {
        /// Dataset source rows in sampled-position order. Duplicates are
        /// intentional for bootstrap and absent only in the observed fit.
        source_rows: Vec<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        normalized_case_weights: Option<Vec<f64>>,
    },
    FrequencyCounts {
        /// Unique compact source rows paired one-to-one with `counts`.
        source_rows: Vec<u32>,
        counts: Vec<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessFullRefitRequestV2 {
    pub contract: String,
    pub profile: ConditionalProcessProfileV2,
    pub group_id: Option<String>,
    pub observed_fit: bool,
    pub sample: ConditionalProcessRefitSampleV2,
}

/// A strict capability receipt returned by every point/refit implementation.
/// The runner checks these flags against the selected frozen profile; a false
/// flag is a blocker, never permission to publish a simplified estimate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessFullRefitReceiptV2 {
    pub contract: String,
    pub measurement_and_scores_refit: bool,
    pub deterministic_score_orientation: bool,
    pub all_joint_structural_equations_refit: bool,
    pub interaction_products_rebuilt: bool,
    pub raw_scientific_gamma_and_delta: bool,
    pub hoc_dependency_stages_refit: bool,
    pub group_isolation_preserved: bool,
    pub positive_case_weights_applied_to_all_stages: bool,
    pub frequency_counts_applied_without_physical_expansion: bool,
    pub frequency_fit_exactly_equivalent_to_row_expansion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConditionalProcessFullRefitPointV2 {
    pub edges: Vec<ConditionalEdgeFunctionV2>,
    /// Sign of the fitted standardized score relative to the directly observed
    /// raw moderator column. Required only to verify raw-unit probe receipts.
    #[serde(default)]
    pub observed_moderator_orientation_signs: BTreeMap<String, i8>,
    pub receipt: ConditionalProcessFullRefitReceiptV2,
}

/// Reusable seam for weighted/count-space and multi-HOC point authorities.
/// Implementations receive the original dataset rows, not a pre-expanded
/// matrix, and must return stable failures on any unavailable exact fit.
pub trait ConditionalProcessFullRefitterV2 {
    fn full_refit(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1>;
}

/// Small construction hook for production authorities that already expose one
/// complete point-refit callback. It deliberately does not adapt partial stage
/// results: the callback must return the full typed point and strict receipt.
pub struct CallbackConditionalProcessFullRefitterV2<F> {
    callback: F,
}

impl<F> CallbackConditionalProcessFullRefitterV2<F> {
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> ConditionalProcessFullRefitterV2 for CallbackConditionalProcessFullRefitterV2<F>
where
    F: FnMut(
        &ConditionalProcessFullRefitRequestV2,
        &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1>,
{
    fn full_refit(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1> {
        (self.callback)(request, is_cancelled)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GroupConditionalCaseLedgerV2 {
    pub group_id: String,
    pub ledger: ConditionalCaseBootstrapLedgerV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RawConditionalProcessEvidenceV2 {
    PercentileCase {
        bootstrap: ConditionalCaseBootstrapLedgerV2,
    },
    BcaCase {
        bootstrap: ConditionalCaseBootstrapLedgerV2,
        delete_one: ConditionalDeleteOneLedgerV2,
    },
    StudentizedCase {
        nested: ConditionalStudentizedLedgerV2,
        observed_inner: ConditionalCaseBootstrapLedgerV2,
    },
    GroupedStratified {
        groups: Vec<GroupConditionalCaseLedgerV2>,
    },
    FrequencyCountSpace {
        bootstrap: ConditionalFrequencyBootstrapLedgerV2,
    },
}

#[derive(Debug, Clone)]
pub struct RawConditionalProcessRunV2 {
    pub output: MultiModRunOutputV1,
    pub preparation: ConditionalProcessAnalysisFrameV2,
    /// The validated original-sample full-refit edge functions, retained by
    /// stratum so qualification and diagnostic callers can independently
    /// reconstruct every published target from the frozen probes. These are
    /// point-fit evidence only; they do not widen the canonical result or
    /// archive serialization contracts.
    pub point_fits: Vec<RawConditionalProcessPointFitV2>,
    pub raw_evidence: RawConditionalProcessEvidenceV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawConditionalProcessPointFitV2 {
    pub group_id: Option<String>,
    pub point: ConditionalProcessFullRefitPointV2,
}

#[derive(Debug, Clone)]
struct RelationBindingV2 {
    relation_id: String,
    source_id: String,
    target_id: String,
    role: StructuralRelationRoleV4,
}

fn structural_bindings_v2(model: &SemModelV4) -> Vec<RelationBindingV2> {
    model
        .relations
        .iter()
        .filter_map(|relation| match relation {
            SemRelationV4::Structural {
                id,
                source,
                target,
                role,
                ..
            } => Some(RelationBindingV2 {
                relation_id: id.clone(),
                source_id: source.clone(),
                target_id: target.clone(),
                role: *role,
            }),
            _ => None,
        })
        .collect()
}

fn conditional_plan_parts_v2(
    artifact: &CompiledMultiModRecipeV1,
) -> Result<
    (
        &[qpls_core::ConditionalProcessPathV2],
        &[CompiledInteractionIdentityV1],
        usize,
    ),
    MultiModRunnerErrorV1,
> {
    let CompiledMultiModPlanV1::GeneralSemConditionalProcessV2 {
        paths,
        interactions,
        compiled_target_upper_bound,
        ..
    } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not General SEM conditional-process V2".into(),
        ));
    };
    Ok((paths, interactions, *compiled_target_upper_bound))
}

fn profile_id_v2(profile: ConditionalProcessProfileV2) -> &'static str {
    match profile {
        ConditionalProcessProfileV2::MultiTwoWayPercentile => "multi_two_way_percentile",
        ConditionalProcessProfileV2::MultiTwoWayBca => "multi_two_way_bca",
        ConditionalProcessProfileV2::MultiTwoWayStudentized => "multi_two_way_studentized",
        ConditionalProcessProfileV2::BoundedThreeWayPercentile => "bounded_three_way_percentile",
        ConditionalProcessProfileV2::MultipleHocPercentile => "multiple_hoc_percentile",
        ConditionalProcessProfileV2::GroupedPercentile => "grouped_percentile",
        ConditionalProcessProfileV2::CaseWeightedPercentile => "case_weighted_percentile",
        ConditionalProcessProfileV2::FrequencyWeightedPercentile => "frequency_weighted_percentile",
    }
}

fn conditional_alternative_v2(value: InferenceAlternativeV1) -> ConditionalAlternativeV2 {
    match value {
        InferenceAlternativeV1::TwoSided => ConditionalAlternativeV2::TwoSided,
        InferenceAlternativeV1::Less => ConditionalAlternativeV2::Less,
        InferenceAlternativeV1::Greater => ConditionalAlternativeV2::Greater,
    }
}

fn interval_v2(
    family: &str,
    confidence_level: f64,
    alternative: InferenceAlternativeV1,
    lower: Option<f64>,
    upper: Option<f64>,
) -> Result<MultimodIntervalV1, MultiModRunnerErrorV1> {
    let shape_valid = match alternative {
        InferenceAlternativeV1::TwoSided => lower.is_some() && upper.is_some(),
        InferenceAlternativeV1::Less => lower.is_none() && upper.is_some(),
        InferenceAlternativeV1::Greater => lower.is_some() && upper.is_none(),
    };
    if !shape_valid
        || lower.is_some_and(|value| !value.is_finite())
        || upper.is_some_and(|value| !value.is_finite())
        || lower.zip(upper).is_some_and(|(left, right)| left > right)
    {
        return Err(MultiModRunnerErrorV1::ResultContract(
            "conditional interval endpoints do not match the declared alternative".into(),
        ));
    }
    Ok(MultimodIntervalV1 {
        confidence_level,
        lower,
        upper,
        family: family.into(),
        alternative,
    })
}

fn empirical_zero_probability_v2(values: &[f64], alternative: InferenceAlternativeV1) -> f64 {
    let denominator = values.len() as f64 + 1.0;
    let lower = (values.iter().filter(|value| **value <= 0.0).count() as f64 + 1.0) / denominator;
    let upper = (values.iter().filter(|value| **value >= 0.0).count() as f64 + 1.0) / denominator;
    match alternative {
        InferenceAlternativeV1::TwoSided => (2.0 * lower.min(upper)).min(1.0),
        InferenceAlternativeV1::Less => upper,
        InferenceAlternativeV1::Greater => lower,
    }
}

fn classify_failure_v2(code: &str) -> MultimodReplicateFailureKindV1 {
    if code.contains("cancel") {
        MultimodReplicateFailureKindV1::Cancelled
    } else if code.contains("rank") || code.contains("singular") {
        MultimodReplicateFailureKindV1::RankDeficient
    } else if code.contains("constant_product") {
        MultimodReplicateFailureKindV1::ConstantProduct
    } else if code.contains("constant") {
        MultimodReplicateFailureKindV1::ConstantScore
    } else if code.contains("nonfinite") {
        MultimodReplicateFailureKindV1::NonfiniteEstimate
    } else if code.contains("converg") {
        MultimodReplicateFailureKindV1::EstimatorDidNotConverge
    } else if code.contains("target") {
        MultimodReplicateFailureKindV1::TargetInventoryMismatch
    } else if code.contains("inner") || code.contains("standard_error") {
        MultimodReplicateFailureKindV1::InnerStandardErrorUnavailable
    } else if code.contains("insufficient") || code.contains("observation") {
        MultimodReplicateFailureKindV1::InsufficientCases
    } else {
        MultimodReplicateFailureKindV1::Other
    }
}

fn prepared_failure_v2(index: u32, failure: &MultiModRefitFailureV1) -> PreparedReplicateEntryV1 {
    PreparedReplicateEntryV1 {
        replicate_index: index,
        seed: 0,
        status: PreparedReplicateStatusV1::Failed {
            kind: classify_failure_v2(&failure.code),
            stable_code: failure.code.clone(),
            detail: failure.message.trim().to_owned(),
        },
    }
}

fn ledger_summary_v2(
    ledger: &PreparedSharedReplicateLedgerV1,
    minimum_required: u32,
) -> MultimodReplicateLedgerSummaryV1 {
    let mut failure_counts = BTreeMap::new();
    let mut failures = Vec::new();
    for entry in &ledger.entries {
        let PreparedReplicateStatusV1::Failed {
            kind,
            stable_code,
            detail,
        } = &entry.status
        else {
            continue;
        };
        *failure_counts.entry(stable_code.clone()).or_insert(0) += 1;
        failures.push(MultimodReplicateFailureV1 {
            replicate_index: entry.replicate_index,
            kind: kind.clone(),
            stable_code: stable_code.clone(),
            detail: detail.clone(),
        });
    }
    let usable = ledger.requested - failures.len() as u32;
    MultimodReplicateLedgerSummaryV1 {
        requested: ledger.requested,
        usable,
        minimum_required,
        usable_fraction: f64::from(usable) / f64::from(ledger.requested),
        complete: usable >= minimum_required,
        ledger_sha256: sha256_serialized(ledger),
        failure_counts,
        failures,
    }
}

fn minimum_usable_v2(requested: u32) -> u32 {
    (f64::from(requested) * 0.90).ceil() as u32
}

fn sample_standard_deviation_v2(values: &[f64]) -> Option<f64> {
    if values.len() < 2 || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let standard_deviation = variance.sqrt();
    (standard_deviation.is_finite() && standard_deviation > 0.0).then_some(standard_deviation)
}

fn observed_source_columns_v2(model: &SemModelV4) -> BTreeMap<String, String> {
    model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } => Some((id.clone(), source_column.clone())),
            _ => None,
        })
        .collect()
}

fn required_conditional_source_columns_v2(
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
) -> Vec<String> {
    let observed = observed_source_columns_v2(model);
    let mut variable_ids = BTreeSet::new();
    for relation in &model.relations {
        match relation {
            SemRelationV4::MeasurementEffect { indicator, .. }
            | SemRelationV4::MeasurementCausal { indicator, .. } => {
                variable_ids.insert(indicator.clone());
            }
            SemRelationV4::Structural { source, target, .. } => {
                variable_ids.insert(source.clone());
                variable_ids.insert(target.clone());
            }
            SemRelationV4::Covariance { .. } => {}
        }
    }
    for interaction in &model.derived_terms {
        match interaction {
            qpls_core::SemDerivedTermV4::Interaction {
                predictor,
                moderator,
                ..
            } => {
                variable_ids.insert(predictor.clone());
                variable_ids.insert(moderator.clone());
            }
            qpls_core::SemDerivedTermV4::InteractionV2 { operands, .. } => {
                variable_ids.extend(operands.iter().cloned());
            }
            qpls_core::SemDerivedTermV4::HigherOrder { components, .. } => {
                variable_ids.extend(components.iter().cloned());
            }
            qpls_core::SemDerivedTermV4::Polynomial { source, .. } => {
                variable_ids.insert(source.clone());
            }
        }
    }
    let mut columns = variable_ids
        .iter()
        .filter_map(|id| observed.get(id).cloned())
        .collect::<BTreeSet<_>>();
    for probe in &config.probes {
        if let Some(receipt) = &probe.raw_transformation_receipt {
            columns.insert(receipt.source_column.clone());
        }
        columns.extend(
            probe
                .raw_fit_metric_receipts
                .iter()
                .map(|receipt| receipt.source_column.clone()),
        );
    }
    if let Some(grouping) = &config.grouping_column {
        columns.insert(grouping.clone());
    }
    if let Some(weight) = &config.weight {
        columns.insert(match weight {
            AnalysisWeightBindingV1::Case { column }
            | AnalysisWeightBindingV1::Frequency { column } => column.clone(),
        });
    }
    columns.into_iter().collect()
}

fn raw_probe_source_bindings_v2(
    config: &GeneralSemConditionalProcessConfigV2,
) -> Result<BTreeMap<String, String>, MultiModRunnerErrorV1> {
    config
        .probes
        .iter()
        .filter(|probe| {
            probe.scale == ConditionalProbeScaleV2::RawObservedWithTransformationReceipt
        })
        .map(|probe| {
            let sources = probe
                .raw_transformation_receipt
                .iter()
                .map(|receipt| receipt.source_column.as_str())
                .chain(
                    probe
                        .raw_fit_metric_receipts
                        .iter()
                        .map(|receipt| receipt.source_column.as_str()),
                )
                .collect::<BTreeSet<_>>();
            if sources.len() != 1 {
                return Err(MultiModRunnerErrorV1::Authority(format!(
                    "raw probe {} lacks one authoritative observed source column",
                    probe.moderator_id
                )));
            }
            Ok((
                probe.moderator_id.clone(),
                (*sources
                    .iter()
                    .next()
                    .expect("validated singleton source binding"))
                .to_owned(),
            ))
        })
        .collect()
}

fn typed_group_value_matches_v2(value: &str, expected: &TypedGroupValueV1) -> bool {
    match expected {
        TypedGroupValueV1::Text { value: expected } => value == expected,
        TypedGroupValueV1::Integer { value: expected } => {
            value.parse::<i64>().ok().as_ref() == Some(expected)
        }
        TypedGroupValueV1::Number { value: expected } => value
            .parse::<f64>()
            .ok()
            .filter(|parsed| parsed.is_finite())
            .is_some_and(|parsed| {
                let parsed = if parsed == 0.0 { 0.0 } else { parsed };
                let expected = if *expected == 0.0 { 0.0 } else { *expected };
                parsed.to_bits() == expected.to_bits()
            }),
        TypedGroupValueV1::Boolean { value: expected } => match value {
            "true" | "TRUE" | "True" | "1" => *expected,
            "false" | "FALSE" | "False" | "0" => !*expected,
            _ => false,
        },
    }
}

fn parsed_finite_v2(row: &BTreeMap<String, Option<String>>, column: &str) -> Option<f64> {
    row.get(column)
        .and_then(Option::as_deref)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
}

/// Derives the one frozen complete-case frame used by point estimation and all
/// resampling. Unselected group levels and unusable rows are retained in the
/// exclusion receipt rather than silently disappearing.
pub fn prepare_conditional_process_analysis_frame_v2(
    dataset: &Dataset,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
) -> Result<ConditionalProcessAnalysisFrameV2, MultiModRunnerErrorV1> {
    if dataset.fingerprint.0.trim().is_empty() {
        return Err(MultiModRunnerErrorV1::Authority(
            "conditional-process dataset fingerprint is empty".into(),
        ));
    }
    let required_source_columns = required_conditional_source_columns_v2(model, config);
    for column in &required_source_columns {
        dataset.batch.schema().index_of(column).map_err(|_| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "conditional-process source column {column} is absent"
            ))
        })?;
    }
    let grouping_column = config.grouping_column.as_deref();
    let weight_column = config.weight.as_ref().map(|weight| match weight {
        AnalysisWeightBindingV1::Case { column }
        | AnalysisWeightBindingV1::Frequency { column } => column.as_str(),
    });
    let numeric_model_columns = required_source_columns
        .iter()
        .filter(|column| {
            Some(column.as_str()) != grouping_column && Some(column.as_str()) != weight_column
        })
        .collect::<Vec<_>>();
    let rows = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
    let mut strata = if config.profile == ConditionalProcessProfileV2::GroupedPercentile {
        config
            .groups
            .iter()
            .map(|group| ConditionalProcessAnalysisStratumV2 {
                group_id: Some(group.group_id.clone()),
                source_rows: Vec::new(),
                case_weights: None,
                frequencies: None,
            })
            .collect::<Vec<_>>()
    } else {
        vec![ConditionalProcessAnalysisStratumV2 {
            group_id: None,
            source_rows: Vec::new(),
            case_weights: config.weight.as_ref().and_then(|weight| {
                matches!(weight, AnalysisWeightBindingV1::Case { .. }).then(Vec::new)
            }),
            frequencies: config.weight.as_ref().and_then(|weight| {
                matches!(weight, AnalysisWeightBindingV1::Frequency { .. }).then(Vec::new)
            }),
        }]
    };
    let mut excluded_rows = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        let model_complete = numeric_model_columns
            .iter()
            .all(|column| parsed_finite_v2(row, column).is_some());
        if !model_complete {
            excluded_rows.push(ConditionalProcessExcludedRowV2 {
                source_row: row_index as u32,
                reason: ConditionalProcessExclusionReasonV2::MissingOrNonfiniteModelValue,
            });
            continue;
        }
        let stratum_index = if let Some(grouping_column) = grouping_column {
            let Some(value) = row.get(grouping_column).and_then(Option::as_deref) else {
                excluded_rows.push(ConditionalProcessExcludedRowV2 {
                    source_row: row_index as u32,
                    reason: ConditionalProcessExclusionReasonV2::MissingGroupingValue,
                });
                continue;
            };
            let Some(index) = config
                .groups
                .iter()
                .position(|group| typed_group_value_matches_v2(value, &group.value))
            else {
                excluded_rows.push(ConditionalProcessExcludedRowV2 {
                    source_row: row_index as u32,
                    reason: ConditionalProcessExclusionReasonV2::UnselectedGroupingValue,
                });
                continue;
            };
            index
        } else {
            0
        };
        let mut case_weight = None;
        let mut frequency = None;
        if let Some(weight) = &config.weight {
            let column = match weight {
                AnalysisWeightBindingV1::Case { column }
                | AnalysisWeightBindingV1::Frequency { column } => column,
            };
            let Some(value) = parsed_finite_v2(row, column) else {
                excluded_rows.push(ConditionalProcessExcludedRowV2 {
                    source_row: row_index as u32,
                    reason: ConditionalProcessExclusionReasonV2::MissingOrInvalidWeight,
                });
                continue;
            };
            match weight {
                AnalysisWeightBindingV1::Case { .. } if value > 0.0 => {
                    case_weight = Some(value);
                }
                AnalysisWeightBindingV1::Frequency { .. }
                    if value >= 1.0 && value.fract() == 0.0 && value <= u64::MAX as f64 =>
                {
                    frequency = Some(value as u64);
                }
                _ => {
                    excluded_rows.push(ConditionalProcessExcludedRowV2 {
                        source_row: row_index as u32,
                        reason: ConditionalProcessExclusionReasonV2::MissingOrInvalidWeight,
                    });
                    continue;
                }
            }
        }
        let stratum = &mut strata[stratum_index];
        stratum.source_rows.push(row_index as u32);
        if let Some(value) = case_weight {
            stratum
                .case_weights
                .as_mut()
                .expect("case-weight profile initialized above")
                .push(value);
        }
        if let Some(value) = frequency {
            stratum
                .frequencies
                .as_mut()
                .expect("frequency-weight profile initialized above")
                .push(value);
        }
    }
    if strata.iter().any(|stratum| stratum.source_rows.len() < 2) {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "every conditional-process analysis stratum requires at least two complete rows".into(),
        ));
    }
    if config.profile == ConditionalProcessProfileV2::GroupedPercentile {
        if strata.iter().any(|stratum| stratum.source_rows.len() < 10) {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "multimod.conditional.group.minimum_complete_cases: every group requires at least 10 complete cases"
                    .into(),
            ));
        }
        let minimum = strata
            .iter()
            .map(|stratum| stratum.source_rows.len())
            .min()
            .unwrap_or(0);
        let maximum = strata
            .iter()
            .map(|stratum| stratum.source_rows.len())
            .max()
            .unwrap_or(0);
        if maximum > minimum.saturating_mul(10) {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "multimod.conditional.group.imbalance_above_10_to_1".into(),
            ));
        }
    }
    for stratum in &strata {
        if let Some(weights) = &stratum.case_weights {
            normalize_positive_case_weights_v2(weights).map_err(|error| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "conditional-process case weights are invalid: {error}"
                ))
            })?;
        }
        if let Some(frequencies) = &stratum.frequencies {
            validate_positive_frequency_weights_v2(frequencies).map_err(|error| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "conditional-process frequency weights are invalid: {error}"
                ))
            })?;
        }
    }
    let analysis_row_mask_sha256 =
        conditional_process_analysis_row_mask_sha256_v2(&dataset.fingerprint.0, &strata);
    Ok(ConditionalProcessAnalysisFrameV2 {
        method_version: CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2.into(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        required_source_columns,
        strata,
        analysis_row_mask_sha256,
        excluded_rows,
    })
}

fn validate_analysis_frame_v2(
    dataset: &Dataset,
    config: &GeneralSemConditionalProcessConfigV2,
    frame: &ConditionalProcessAnalysisFrameV2,
) -> Result<(), MultiModRunnerErrorV1> {
    if frame.method_version != CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2
        || frame.dataset_fingerprint != dataset.fingerprint.0
        || frame.analysis_row_mask_sha256
            != conditional_process_analysis_row_mask_sha256_v2(
                &frame.dataset_fingerprint,
                &frame.strata,
            )
        || frame.required_source_columns.is_empty()
        || frame
            .required_source_columns
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "conditional-process analysis-frame identity is invalid".into(),
        ));
    }
    let expected_group_ids = if config.profile == ConditionalProcessProfileV2::GroupedPercentile {
        config
            .groups
            .iter()
            .map(|group| group.group_id.as_str())
            .collect::<BTreeSet<_>>()
    } else {
        BTreeSet::new()
    };
    let actual_group_ids = frame
        .strata
        .iter()
        .filter_map(|stratum| stratum.group_id.as_deref())
        .collect::<BTreeSet<_>>();
    if (config.profile == ConditionalProcessProfileV2::GroupedPercentile
        && (frame.strata.len() != config.groups.len() || actual_group_ids != expected_group_ids))
        || (config.profile != ConditionalProcessProfileV2::GroupedPercentile
            && (frame.strata.len() != 1 || frame.strata[0].group_id.is_some()))
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "conditional-process frame strata differ from the configured groups".into(),
        ));
    }
    let mut all_rows = BTreeSet::new();
    for stratum in &frame.strata {
        if stratum.source_rows.len() < 2
            || stratum
                .source_rows
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || stratum
                .source_rows
                .iter()
                .any(|row| *row as usize >= dataset.batch.num_rows() || !all_rows.insert(*row))
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "analysis strata require unique ascending in-range source rows".into(),
            ));
        }
        match config.profile {
            ConditionalProcessProfileV2::CaseWeightedPercentile => {
                let Some(weights) = &stratum.case_weights else {
                    return Err(MultiModRunnerErrorV1::PreparedInput(
                        "case-weighted frame omitted case weights".into(),
                    ));
                };
                if weights.len() != stratum.source_rows.len() {
                    return Err(MultiModRunnerErrorV1::PreparedInput(
                        "case weights differ from the compact source-row count".into(),
                    ));
                }
                normalize_positive_case_weights_v2(weights)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?;
            }
            ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
                let Some(frequencies) = &stratum.frequencies else {
                    return Err(MultiModRunnerErrorV1::PreparedInput(
                        "frequency-weighted frame omitted frequencies".into(),
                    ));
                };
                if frequencies.len() != stratum.source_rows.len() {
                    return Err(MultiModRunnerErrorV1::PreparedInput(
                        "frequency counts differ from the compact source-row count".into(),
                    ));
                }
                validate_positive_frequency_weights_v2(frequencies)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?;
            }
            _ if stratum.case_weights.is_some() || stratum.frequencies.is_some() => {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "an unweighted conditional profile carried weights".into(),
                ));
            }
            _ => {}
        }
    }
    if frame
        .excluded_rows
        .windows(2)
        .any(|pair| pair[0].source_row >= pair[1].source_row)
        || frame
            .excluded_rows
            .iter()
            .any(|row| row.source_row as usize >= dataset.batch.num_rows())
        || frame
            .excluded_rows
            .iter()
            .any(|row| all_rows.contains(&row.source_row))
        || all_rows.len() + frame.excluded_rows.len() != dataset.batch.num_rows()
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "analysis exclusions must be unique, ordered, in range, and partition the dataset with admitted rows"
                .into(),
        ));
    }
    Ok(())
}

fn approximately_equal_v2(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-12 * scale
}

fn raw_probe_source_values_for_stratum_v2(
    dataset: &Dataset,
    stratum: &ConditionalProcessAnalysisStratumV2,
    source_column: &str,
) -> Result<Vec<f64>, MultiModRunnerErrorV1> {
    let rows = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
    stratum
        .source_rows
        .iter()
        .map(|source_row| {
            parsed_finite_v2(&rows[*source_row as usize], source_column).ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "raw probe source {source_column} is nonnumeric on a retained analysis row"
                ))
            })
        })
        .collect()
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ConditionalRawProbeRowMassIdentityV2<'a> {
    Unweighted {
        source_rows: &'a [u32],
    },
    CaseWeighted {
        source_rows: &'a [u32],
        weight_bits: Vec<u64>,
    },
    FrequencyWeighted {
        source_rows: &'a [u32],
        counts: &'a [u64],
    },
}

fn expected_raw_probe_fit_metric_receipt_v2(
    dataset: &Dataset,
    frame: &ConditionalProcessAnalysisFrameV2,
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    moderator_id: &str,
    source_column: &str,
    orientation_sign: i8,
) -> Result<ConditionalRawProbeFitMetricReceiptV2, MultiModRunnerErrorV1> {
    if !matches!(orientation_sign, -1 | 1) {
        return Err(MultiModRunnerErrorV1::Authority(format!(
            "raw probe {moderator_id} has an invalid score-orientation sign"
        )));
    }
    let values = raw_probe_source_values_for_stratum_v2(dataset, stratum, source_column)?;
    if values.len() != stratum.source_rows.len() || values.len() < 2 {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "raw probe source {source_column} has insufficient retained values"
        )));
    }
    let compact_row_count = u32::try_from(values.len()).map_err(|_| {
        MultiModRunnerErrorV1::PreparedInput(
            "raw probe compact row count exceeds the receipt representation".into(),
        )
    })?;
    let fit_scope = stratum
        .group_id
        .as_ref()
        .map(|group_id| ConditionalRawProbeFitScopeV2::GroupFit {
            group_id: group_id.clone(),
        })
        .unwrap_or(ConditionalRawProbeFitScopeV2::AnalysisFit);
    let (
        metric_basis,
        weight_column,
        row_mass_sha256,
        mass_sum,
        mass_squared_sum,
        effective_degrees_of_freedom,
        frequency_total,
        masses,
    ) = match config.profile {
        ConditionalProcessProfileV2::CaseWeightedPercentile => {
            let weights = stratum.case_weights.as_deref().ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(
                    "case-weighted raw-probe metric lacks original case weights".into(),
                )
            })?;
            if weights.len() != values.len()
                || weights
                    .iter()
                    .any(|weight| !weight.is_finite() || *weight <= 0.0)
            {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "case-weighted raw-probe metric has invalid row masses".into(),
                ));
            }
            let mass_sum = weights.iter().sum::<f64>();
            let mass_squared_sum = weights.iter().map(|weight| weight * weight).sum::<f64>();
            let effective_df = mass_sum - mass_squared_sum / mass_sum;
            let column = match config.weight.as_ref() {
                Some(AnalysisWeightBindingV1::Case { column }) => column.clone(),
                _ => {
                    return Err(MultiModRunnerErrorV1::Authority(
                        "case-weighted raw-probe metric lacks its configured binding".into(),
                    ));
                }
            };
            let identity = ConditionalRawProbeRowMassIdentityV2::CaseWeighted {
                source_rows: &stratum.source_rows,
                weight_bits: weights.iter().map(|weight| weight.to_bits()).collect(),
            };
            (
                ConditionalRawProbeMetricBasisV2::CaseWeightedEffectiveDf,
                Some(column),
                sha256_serialized(&identity),
                mass_sum,
                mass_squared_sum,
                effective_df,
                None,
                weights.to_vec(),
            )
        }
        ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
            let counts = stratum.frequencies.as_deref().ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(
                    "frequency raw-probe metric lacks original counts".into(),
                )
            })?;
            if counts.len() != values.len() || counts.iter().any(|count| *count == 0) {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "frequency raw-probe metric has invalid compact counts".into(),
                ));
            }
            let total = counts.iter().try_fold(0_u64, |sum, count| {
                sum.checked_add(*count).ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "frequency raw-probe metric total overflowed".into(),
                    )
                })
            })?;
            if total > (1_u64 << 53) - 1 || total < 2 {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "frequency raw-probe metric total is outside the exact count-space envelope"
                        .into(),
                ));
            }
            let masses = counts.iter().map(|count| *count as f64).collect::<Vec<_>>();
            let mass_sum = total as f64;
            let mass_squared_sum = masses.iter().map(|mass| mass * mass).sum::<f64>();
            let column = match config.weight.as_ref() {
                Some(AnalysisWeightBindingV1::Frequency { column }) => column.clone(),
                _ => {
                    return Err(MultiModRunnerErrorV1::Authority(
                        "frequency raw-probe metric lacks its configured binding".into(),
                    ));
                }
            };
            let identity = ConditionalRawProbeRowMassIdentityV2::FrequencyWeighted {
                source_rows: &stratum.source_rows,
                counts,
            };
            (
                ConditionalRawProbeMetricBasisV2::FrequencyExpandedSample,
                Some(column),
                sha256_serialized(&identity),
                mass_sum,
                mass_squared_sum,
                mass_sum - 1.0,
                Some(total),
                masses,
            )
        }
        _ => {
            let mass_sum = values.len() as f64;
            let identity = ConditionalRawProbeRowMassIdentityV2::Unweighted {
                source_rows: &stratum.source_rows,
            };
            (
                ConditionalRawProbeMetricBasisV2::UnweightedSample,
                None,
                sha256_serialized(&identity),
                mass_sum,
                mass_sum,
                mass_sum - 1.0,
                None,
                vec![1.0; values.len()],
            )
        }
    };
    if !mass_sum.is_finite()
        || mass_sum <= 0.0
        || !mass_squared_sum.is_finite()
        || !effective_degrees_of_freedom.is_finite()
        || effective_degrees_of_freedom <= 0.0
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "raw probe source {source_column} has an unusable effective sample mass"
        )));
    }
    let center = values
        .iter()
        .zip(&masses)
        .map(|(value, mass)| value * mass)
        .sum::<f64>()
        / mass_sum;
    let variance = values
        .iter()
        .zip(&masses)
        .map(|(value, mass)| mass * (value - center).powi(2))
        .sum::<f64>()
        / effective_degrees_of_freedom;
    let standard_deviation = variance.sqrt();
    if !center.is_finite() || !standard_deviation.is_finite() || standard_deviation <= 0.0 {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "raw probe source {source_column} is constant or has an invalid weighted scale"
        )));
    }
    let receipt = ConditionalRawProbeFitMetricReceiptV2 {
        contract: CONDITIONAL_RAW_PROBE_FIT_METRIC_CONTRACT_V2.into(),
        moderator_id: moderator_id.into(),
        source_column: source_column.into(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        analysis_row_mask_sha256: frame.analysis_row_mask_sha256.clone(),
        fit_scope,
        metric_basis,
        weight_column,
        row_mass_sha256,
        compact_row_count,
        mass_sum,
        mass_squared_sum,
        effective_degrees_of_freedom,
        frequency_total,
        center,
        standard_deviation,
        orientation_sign,
    };
    receipt
        .ensure_valid("conditional_process.raw_probe_fit_metric")
        .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?;
    Ok(receipt)
}

/// Creates the exact original-sample anchor inventory required by a raw-unit
/// moderator. Authoring calls this after the complete-case frame and observed
/// point orientation are known, then persists the returned receipts in the V2
/// configuration. No bootstrap/refit may replace these anchors.
pub fn prepare_conditional_raw_probe_fit_metric_receipts_v2(
    dataset: &Dataset,
    model: &SemModelV4,
    frame: &ConditionalProcessAnalysisFrameV2,
    config: &GeneralSemConditionalProcessConfigV2,
    moderator_id: &str,
    orientation_sign: i8,
) -> Result<Vec<ConditionalRawProbeFitMetricReceiptV2>, MultiModRunnerErrorV1> {
    validate_analysis_frame_v2(dataset, config, frame)?;
    let source_columns = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                source_column,
                scale: ObservedScaleV4::Continuous | ObservedScaleV4::Binary,
                ..
            } if id == moderator_id => Some(source_column.as_str()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    if source_columns.len() != 1 {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.raw_probe_observed_single_indicator_required:{moderator_id}"
        )));
    }
    let source_column = *source_columns
        .iter()
        .next()
        .expect("validated singleton raw-probe source");
    let probe = config
        .probes
        .iter()
        .find(|probe| probe.moderator_id == moderator_id)
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(format!(
                "raw-probe receipt authoring lacks moderator {moderator_id}"
            ))
        })?;
    if probe.scale != ConditionalProbeScaleV2::RawObservedWithTransformationReceipt {
        return Err(MultiModRunnerErrorV1::Authority(format!(
            "moderator {moderator_id} is not configured for raw-unit probes"
        )));
    }
    let mut receipts = frame
        .strata
        .iter()
        .map(|stratum| {
            expected_raw_probe_fit_metric_receipt_v2(
                dataset,
                frame,
                config,
                stratum,
                moderator_id,
                source_column,
                orientation_sign,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    receipts.sort_by(|left, right| left.fit_scope.cmp(&right.fit_scope));
    Ok(receipts)
}

fn raw_probe_fit_metric_receipt_for_stratum_v2<'a>(
    probe: &'a qpls_core::ConditionalModeratorProbeV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
) -> Result<&'a ConditionalRawProbeFitMetricReceiptV2, MultiModRunnerErrorV1> {
    let expected_scope = stratum
        .group_id
        .as_ref()
        .map(|group_id| ConditionalRawProbeFitScopeV2::GroupFit {
            group_id: group_id.clone(),
        })
        .unwrap_or(ConditionalRawProbeFitScopeV2::AnalysisFit);
    let matches = probe
        .raw_fit_metric_receipts
        .iter()
        .filter(|receipt| receipt.fit_scope == expected_scope)
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        Ok(matches[0])
    } else {
        Err(MultiModRunnerErrorV1::Authority(format!(
            "raw probe {} requires exactly one receipt for its original-sample fit scope",
            probe.moderator_id
        )))
    }
}

fn standardized_probe_value_v2(
    dataset: &Dataset,
    frame: &ConditionalProcessAnalysisFrameV2,
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    moderator_id: &str,
    authored_value: f64,
) -> Result<f64, MultiModRunnerErrorV1> {
    let probe = config
        .probes
        .iter()
        .find(|probe| probe.moderator_id == moderator_id)
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(format!(
                "probe conversion lacks moderator {moderator_id}"
            ))
        })?;
    if probe.scale == ConditionalProbeScaleV2::StandardizedScore {
        return authored_value
            .is_finite()
            .then_some(authored_value)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput("standardized probe is nonfinite".into())
            });
    }
    if !probe.raw_fit_metric_receipts.is_empty() {
        let receipt = raw_probe_fit_metric_receipt_for_stratum_v2(probe, stratum)?;
        if receipt.moderator_id != moderator_id
            || receipt.dataset_fingerprint != dataset.fingerprint.0
            || receipt.analysis_row_mask_sha256 != frame.analysis_row_mask_sha256
        {
            return Err(MultiModRunnerErrorV1::Authority(format!(
                "raw probe {moderator_id} fit-metric receipt differs from the execution data or row mask"
            )));
        }
        let expected = expected_raw_probe_fit_metric_receipt_v2(
            dataset,
            frame,
            config,
            stratum,
            moderator_id,
            &receipt.source_column,
            receipt.orientation_sign,
        )?;
        if receipt != &expected {
            return Err(MultiModRunnerErrorV1::Authority(format!(
                "raw probe {moderator_id} fit-metric receipt does not reproduce from its frozen original-sample rows and masses"
            )));
        }
        return receipt
            .standardize(authored_value)
            .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()));
    }
    let receipt = probe.raw_transformation_receipt.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(format!(
            "raw probe {moderator_id} lacks its numerical transformation receipt"
        ))
    })?;
    if receipt.moderator_id != moderator_id
        || receipt.dataset_fingerprint != dataset.fingerprint.0
        || receipt.analysis_row_mask_sha256 != frame.analysis_row_mask_sha256
    {
        return Err(MultiModRunnerErrorV1::Authority(format!(
            "raw probe {moderator_id} receipt differs from the execution data or row mask"
        )));
    }
    let values = raw_probe_source_values_for_stratum_v2(dataset, stratum, &receipt.source_column)?;
    let center = values.iter().sum::<f64>() / values.len() as f64;
    let sample_standard_deviation = sample_standard_deviation_v2(&values).ok_or_else(|| {
        MultiModRunnerErrorV1::PreparedInput(format!(
            "raw probe source {} is constant or has insufficient rows",
            receipt.source_column
        ))
    })?;
    if !approximately_equal_v2(center, receipt.center)
        || !approximately_equal_v2(sample_standard_deviation, receipt.sample_standard_deviation)
    {
        return Err(MultiModRunnerErrorV1::Authority(format!(
            "raw probe {moderator_id} center/sample-SD receipt does not reproduce from the frozen analysis rows"
        )));
    }
    receipt
        .standardize(authored_value)
        .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))
}

#[derive(Debug, Clone, PartialEq)]
struct FrozenConditionalProbePointV2 {
    calculation: ConditionalProbePointV2,
    authored_values: BTreeMap<String, f64>,
}

fn cartesian_probe_points_v2(
    axes: &[(String, Vec<(f64, f64)>)],
    axis: usize,
    authored: &mut BTreeMap<String, f64>,
    standardized: &mut BTreeMap<String, f64>,
    output: &mut Vec<FrozenConditionalProbePointV2>,
) {
    if axis == axes.len() {
        let digest = sha256_serialized(&(
            CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2,
            "cartesian_probe",
            &*authored,
        ));
        output.push(FrozenConditionalProbePointV2 {
            calculation: ConditionalProbePointV2 {
                probe_id: format!("qpls.cp.probe.v2.{digest}"),
                standardized_values: standardized.clone(),
            },
            authored_values: authored.clone(),
        });
        return;
    }
    let (moderator_id, values) = &axes[axis];
    for (authored_value, standardized_value) in values {
        authored.insert(moderator_id.clone(), *authored_value);
        standardized.insert(moderator_id.clone(), *standardized_value);
        cartesian_probe_points_v2(axes, axis + 1, authored, standardized, output);
    }
    authored.remove(moderator_id);
    standardized.remove(moderator_id);
}

fn frozen_probe_points_v2(
    dataset: &Dataset,
    frame: &ConditionalProcessAnalysisFrameV2,
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
) -> Result<Vec<FrozenConditionalProbePointV2>, MultiModRunnerErrorV1> {
    let mut points = if config.explicit_joint_tuples.is_empty() {
        let mut axes = Vec::with_capacity(config.probes.len());
        for probe in &config.probes {
            let values = probe
                .values
                .iter()
                .map(|value| {
                    Ok((
                        *value,
                        standardized_probe_value_v2(
                            dataset,
                            frame,
                            config,
                            stratum,
                            &probe.moderator_id,
                            *value,
                        )?,
                    ))
                })
                .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?;
            axes.push((probe.moderator_id.clone(), values));
        }
        axes.sort_by(|left, right| left.0.cmp(&right.0));
        let mut points = Vec::new();
        cartesian_probe_points_v2(
            &axes,
            0,
            &mut BTreeMap::new(),
            &mut BTreeMap::new(),
            &mut points,
        );
        points
    } else {
        config
            .explicit_joint_tuples
            .iter()
            .map(|tuple| {
                let standardized_values = tuple
                    .values_by_moderator
                    .iter()
                    .map(|(moderator_id, value)| {
                        Ok((
                            moderator_id.clone(),
                            standardized_probe_value_v2(
                                dataset,
                                frame,
                                config,
                                stratum,
                                moderator_id,
                                *value,
                            )?,
                        ))
                    })
                    .collect::<Result<BTreeMap<_, _>, MultiModRunnerErrorV1>>()?;
                Ok(FrozenConditionalProbePointV2 {
                    calculation: ConditionalProbePointV2 {
                        probe_id: tuple.tuple_id.clone(),
                        standardized_values,
                    },
                    authored_values: tuple.values_by_moderator.clone(),
                })
            })
            .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?
    };
    points.sort_by(|left, right| left.calculation.probe_id.cmp(&right.calculation.probe_id));
    if points.is_empty()
        || points
            .windows(2)
            .any(|pair| pair[0].calculation.probe_id == pair[1].calculation.probe_id)
    {
        return Err(MultiModRunnerErrorV1::Authority(
            "frozen probe expansion is empty or has duplicate identities".into(),
        ));
    }
    Ok(points)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ConditionalTargetOperationV2 {
    Specific {
        path_id: String,
        probe: ConditionalProbePointV2,
    },
    TotalIndirect {
        path_ids: Vec<String>,
        probe: ConditionalProbePointV2,
    },
    TotalEffect {
        path_ids: Vec<String>,
        direct_relation_id: String,
        probe: ConditionalProbePointV2,
    },
    ScalarIndex {
        path_id: String,
        moderator_id: String,
    },
    Derivative {
        path_id: String,
        probe: ConditionalProbePointV2,
        derivative_variables: Vec<String>,
    },
    ProbeContrast {
        path_id: String,
        contrast_id: String,
        left: ConditionalProbePointV2,
        right: ConditionalProbePointV2,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct ConditionalTargetTemplateV2 {
    target_id: String,
    kind: ConditionalProcessTargetKindV2,
    path_id: String,
    probe_values: BTreeMap<String, f64>,
    derivative_variables: Vec<String>,
    operation: ConditionalTargetOperationV2,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ConditionalTargetOperationIdentityV2<'a> {
    Specific {
        path_id: &'a str,
        probe_id: &'a str,
    },
    TotalIndirect {
        path_ids: &'a [String],
        probe_id: &'a str,
    },
    TotalEffect {
        path_ids: &'a [String],
        direct_relation_id: &'a str,
        probe_id: &'a str,
    },
    ScalarIndex {
        path_id: &'a str,
        moderator_id: &'a str,
    },
    Derivative {
        path_id: &'a str,
        probe_id: &'a str,
        derivative_variables: &'a [String],
    },
    ProbeContrast {
        path_id: &'a str,
        contrast_id: &'a str,
        left_probe_id: &'a str,
        right_probe_id: &'a str,
    },
}

fn target_id_v2(
    kind: ConditionalProcessTargetKindV2,
    path_id: &str,
    operation: &ConditionalTargetOperationV2,
) -> String {
    let operation_identity = match operation {
        ConditionalTargetOperationV2::Specific { path_id, probe } => {
            ConditionalTargetOperationIdentityV2::Specific {
                path_id,
                probe_id: &probe.probe_id,
            }
        }
        ConditionalTargetOperationV2::TotalIndirect { path_ids, probe } => {
            ConditionalTargetOperationIdentityV2::TotalIndirect {
                path_ids,
                probe_id: &probe.probe_id,
            }
        }
        ConditionalTargetOperationV2::TotalEffect {
            path_ids,
            direct_relation_id,
            probe,
        } => ConditionalTargetOperationIdentityV2::TotalEffect {
            path_ids,
            direct_relation_id,
            probe_id: &probe.probe_id,
        },
        ConditionalTargetOperationV2::ScalarIndex {
            path_id,
            moderator_id,
        } => ConditionalTargetOperationIdentityV2::ScalarIndex {
            path_id,
            moderator_id,
        },
        ConditionalTargetOperationV2::Derivative {
            path_id,
            probe,
            derivative_variables,
        } => ConditionalTargetOperationIdentityV2::Derivative {
            path_id,
            probe_id: &probe.probe_id,
            derivative_variables,
        },
        ConditionalTargetOperationV2::ProbeContrast {
            path_id,
            contrast_id,
            left,
            right,
        } => ConditionalTargetOperationIdentityV2::ProbeContrast {
            path_id,
            contrast_id,
            left_probe_id: &left.probe_id,
            right_probe_id: &right.probe_id,
        },
    };
    let digest = sha256_serialized(&(
        CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2,
        &kind,
        path_id,
        operation_identity,
    ));
    format!("qpls.cp.target.v2.{digest}")
}

fn local_target_id_v2(group_id: Option<&str>, base_target_id: &str) -> String {
    group_id
        .map(|group| {
            format!(
                "qpls.cp.target.v2.{}",
                sha256_serialized(&("group_local", group, base_target_id))
            )
        })
        .unwrap_or_else(|| base_target_id.to_owned())
}

fn group_contrast_target_id_v2(
    contrast_id: &str,
    left_group_id: &str,
    right_group_id: &str,
    base_target_id: &str,
) -> String {
    format!(
        "qpls.cp.target.v2.{}",
        sha256_serialized(&(
            "group_contrast_left_minus_right",
            contrast_id,
            left_group_id,
            right_group_id,
            base_target_id,
        ))
    )
}

type MonomialShapeV2 = BTreeMap<String, u8>;

fn multiply_shape_v2(left: &[MonomialShapeV2], right: &[MonomialShapeV2]) -> Vec<MonomialShapeV2> {
    let mut output = BTreeSet::<Vec<(String, u8)>>::new();
    for left_term in left {
        for right_term in right {
            let mut term = left_term.clone();
            for (moderator, exponent) in right_term {
                *term.entry(moderator.clone()).or_default() += *exponent;
            }
            output.insert(term.into_iter().collect());
        }
    }
    output
        .into_iter()
        .map(|term| term.into_iter().collect())
        .collect()
}

fn conditional_path_shape_v2(
    path: &qpls_core::ConditionalProcessPathV2,
    interactions: &[CompiledInteractionIdentityV1],
) -> Option<Vec<MonomialShapeV2>> {
    let mut path_shape = vec![BTreeMap::new()];
    for relation_id in &path.ordered_relation_ids {
        let mut edge_shape = vec![BTreeMap::new()];
        for interaction in interactions
            .iter()
            .filter(|interaction| &interaction.focal_relation_id == relation_id)
        {
            match interaction.operands.as_slice() {
                [_, moderator] => {
                    edge_shape.push(BTreeMap::from([(moderator.clone(), 1)]));
                }
                [_, first, second] => {
                    edge_shape.push(BTreeMap::from([(first.clone(), 1), (second.clone(), 1)]));
                }
                _ => return None,
            }
        }
        path_shape = multiply_shape_v2(&path_shape, &edge_shape);
    }
    Some(path_shape)
}

fn derivative_structurally_nonzero_v2(
    path: &qpls_core::ConditionalProcessPathV2,
    interactions: &[CompiledInteractionIdentityV1],
    derivative_variables: &[String],
) -> Option<bool> {
    let required_powers =
        derivative_variables
            .iter()
            .fold(BTreeMap::<String, u8>::new(), |mut powers, moderator| {
                *powers.entry(moderator.clone()).or_default() += 1;
                powers
            });
    conditional_path_shape_v2(path, interactions).map(|path_shape| {
        path_shape.iter().any(|term| {
            required_powers.iter().all(|(moderator, required)| {
                term.get(moderator).copied().unwrap_or_default() >= *required
            })
        })
    })
}

fn eligible_derivative_targets_v2(
    path: &qpls_core::ConditionalProcessPathV2,
    moderator_ids: &[String],
    include_first: bool,
    include_second_and_cross: bool,
    interactions: &[CompiledInteractionIdentityV1],
) -> Result<Vec<(ConditionalProcessTargetKindV2, Vec<String>)>, MultiModRunnerErrorV1> {
    let mut eligible = Vec::new();
    let mut admit = |kind, variables: Vec<String>| -> Result<(), MultiModRunnerErrorV1> {
        if derivative_structurally_nonzero_v2(path, interactions, &variables).ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(format!(
                "path {} contains an interaction outside the bounded conditional polynomial",
                path.path_id
            ))
        })? {
            eligible.push((kind, variables));
        }
        Ok(())
    };
    if include_first {
        for moderator in moderator_ids {
            admit(
                ConditionalProcessTargetKindV2::LocalFirstDerivative,
                vec![moderator.clone()],
            )?;
        }
    }
    if include_second_and_cross {
        for (index, first) in moderator_ids.iter().enumerate() {
            admit(
                ConditionalProcessTargetKindV2::LocalSecondDerivative,
                vec![first.clone(), first.clone()],
            )?;
            for second in moderator_ids.iter().skip(index + 1) {
                admit(
                    ConditionalProcessTargetKindV2::LocalCrossDerivative,
                    vec![first.clone(), second.clone()],
                )?;
            }
        }
    }
    Ok(eligible)
}

fn affine_path_moderator_v2(
    path: &qpls_core::ConditionalProcessPathV2,
    interactions: &[CompiledInteractionIdentityV1],
) -> Option<String> {
    let path_shape = conditional_path_shape_v2(path, interactions)?;
    let moderators = path_shape
        .iter()
        .flat_map(|term| term.keys().cloned())
        .collect::<BTreeSet<_>>();
    if moderators.len() != 1
        || path_shape.iter().any(|term| {
            term.values()
                .map(|value| usize::from(*value))
                .sum::<usize>()
                > 1
        })
    {
        return None;
    }
    moderators.into_iter().next()
}

fn path_endpoints_v2(
    path: &qpls_core::ConditionalProcessPathV2,
    relations: &BTreeMap<String, RelationBindingV2>,
) -> Result<(String, String), MultiModRunnerErrorV1> {
    let first = relations
        .get(&path.ordered_relation_ids[0])
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("path relation disappeared".into()))?;
    let last = relations
        .get(
            path.ordered_relation_ids
                .last()
                .expect("validated nonempty path"),
        )
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("path relation disappeared".into()))?;
    Ok((first.source_id.clone(), last.target_id.clone()))
}

fn push_target_template_v2(
    output: &mut Vec<ConditionalTargetTemplateV2>,
    kind: ConditionalProcessTargetKindV2,
    path_id: String,
    probe_values: BTreeMap<String, f64>,
    derivative_variables: Vec<String>,
    operation: ConditionalTargetOperationV2,
) {
    let target_id = target_id_v2(kind.clone(), &path_id, &operation);
    output.push(ConditionalTargetTemplateV2 {
        target_id,
        kind,
        path_id,
        probe_values,
        derivative_variables,
        operation,
    });
}

fn target_templates_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    model: &SemModelV4,
    probes: &[FrozenConditionalProbePointV2],
) -> Result<(Vec<ConditionalTargetTemplateV2>, Vec<String>), MultiModRunnerErrorV1> {
    let (compiled_paths, interactions, compiled_upper_bound) = conditional_plan_parts_v2(artifact)?;
    let relations = structural_bindings_v2(model)
        .into_iter()
        .map(|relation| (relation.relation_id.clone(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut paths = compiled_paths.iter().collect::<Vec<_>>();
    paths.sort_by(|left, right| left.path_id.cmp(&right.path_id));
    let mut endpoint_paths = BTreeMap::<(String, String), Vec<String>>::new();
    for path in &paths {
        let endpoints = path_endpoints_v2(path, &relations)?;
        endpoint_paths
            .entry(endpoints)
            .or_default()
            .push(path.path_id.clone());
    }
    for path_ids in endpoint_paths.values_mut() {
        path_ids.sort();
    }
    let mut output = Vec::new();
    let mut warnings = Vec::new();
    for probe in probes {
        if config.estimands.conditional_specific_indirect {
            for path in &paths {
                push_target_template_v2(
                    &mut output,
                    ConditionalProcessTargetKindV2::ConditionalSpecificIndirect,
                    path.path_id.clone(),
                    probe.authored_values.clone(),
                    Vec::new(),
                    ConditionalTargetOperationV2::Specific {
                        path_id: path.path_id.clone(),
                        probe: probe.calculation.clone(),
                    },
                );
            }
        }
        for ((source, target), path_ids) in &endpoint_paths {
            let representative = path_ids[0].clone();
            if config.estimands.conditional_total_indirect {
                push_target_template_v2(
                    &mut output,
                    ConditionalProcessTargetKindV2::ConditionalTotalIndirect,
                    representative.clone(),
                    probe.authored_values.clone(),
                    Vec::new(),
                    ConditionalTargetOperationV2::TotalIndirect {
                        path_ids: path_ids.clone(),
                        probe: probe.calculation.clone(),
                    },
                );
            }
            if config.estimands.conditional_total_effect {
                let direct = relations
                    .values()
                    .filter(|relation| {
                        relation.role == StructuralRelationRoleV4::Structural
                            && relation.source_id == *source
                            && relation.target_id == *target
                    })
                    .map(|relation| relation.relation_id.clone())
                    .collect::<Vec<_>>();
                if direct.len() != 1 {
                    return Err(MultiModRunnerErrorV1::Authority(format!(
                        "total effect {source}->{target} requires exactly one authored direct structural relation; found {}",
                        direct.len()
                    )));
                }
                push_target_template_v2(
                    &mut output,
                    ConditionalProcessTargetKindV2::ConditionalTotalEffect,
                    representative,
                    probe.authored_values.clone(),
                    Vec::new(),
                    ConditionalTargetOperationV2::TotalEffect {
                        path_ids: path_ids.clone(),
                        direct_relation_id: direct[0].clone(),
                        probe: probe.calculation.clone(),
                    },
                );
            }
        }
        for path in &paths {
            for (kind, derivative_variables) in eligible_derivative_targets_v2(
                path,
                &config.moderator_ids,
                config.estimands.local_first_derivatives,
                config.estimands.local_second_and_cross_derivatives,
                interactions,
            )? {
                push_target_template_v2(
                    &mut output,
                    kind,
                    path.path_id.clone(),
                    probe.authored_values.clone(),
                    derivative_variables.clone(),
                    ConditionalTargetOperationV2::Derivative {
                        path_id: path.path_id.clone(),
                        probe: probe.calculation.clone(),
                        derivative_variables,
                    },
                );
            }
        }
    }
    if config.estimands.scalar_index_when_affine {
        for path in &paths {
            if let Some(moderator_id) = affine_path_moderator_v2(path, interactions) {
                push_target_template_v2(
                    &mut output,
                    ConditionalProcessTargetKindV2::ScalarIndexOfModeratedMediation,
                    path.path_id.clone(),
                    BTreeMap::new(),
                    vec![moderator_id.clone()],
                    ConditionalTargetOperationV2::ScalarIndex {
                        path_id: path.path_id.clone(),
                        moderator_id,
                    },
                );
            } else {
                warnings.push(format!(
                    "path {} is not affine in exactly one moderator; no constant Hayes index was reported",
                    path.path_id
                ));
            }
        }
    }
    if config.estimands.finite_probe_contrasts {
        let probes_by_id = probes
            .iter()
            .map(|probe| (probe.calculation.probe_id.as_str(), probe))
            .collect::<BTreeMap<_, _>>();
        let mut contrasts = config.probe_contrasts.iter().collect::<Vec<_>>();
        contrasts.sort_by(|left, right| left.contrast_id.cmp(&right.contrast_id));
        for path in &paths {
            for contrast in &contrasts {
                let left = probes_by_id
                    .get(contrast.left_tuple_id.as_str())
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::Authority(format!(
                            "probe contrast {} lacks left tuple {}",
                            contrast.contrast_id, contrast.left_tuple_id
                        ))
                    })?;
                let right = probes_by_id
                    .get(contrast.right_tuple_id.as_str())
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::Authority(format!(
                            "probe contrast {} lacks right tuple {}",
                            contrast.contrast_id, contrast.right_tuple_id
                        ))
                    })?;
                push_target_template_v2(
                    &mut output,
                    ConditionalProcessTargetKindV2::ProbeContrast,
                    path.path_id.clone(),
                    BTreeMap::new(),
                    Vec::new(),
                    ConditionalTargetOperationV2::ProbeContrast {
                        path_id: path.path_id.clone(),
                        contrast_id: contrast.contrast_id.clone(),
                        left: left.calculation.clone(),
                        right: right.calculation.clone(),
                    },
                );
            }
        }
    }
    output.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    if output.is_empty()
        || output
            .windows(2)
            .any(|pair| pair[0].target_id == pair[1].target_id)
    {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.no_eligible_requested_target".into(),
        ));
    }
    let group_factor = config.groups.len().max(1);
    let local_count = output.len().saturating_mul(group_factor);
    let group_contrast_count = if config.profile == ConditionalProcessProfileV2::GroupedPercentile {
        config.group_contrasts.len().saturating_mul(
            output
                .iter()
                .filter(|target| target.kind != ConditionalProcessTargetKindV2::ProbeContrast)
                .count(),
        )
    } else {
        0
    };
    if local_count.saturating_add(group_contrast_count) > compiled_upper_bound {
        return Err(MultiModRunnerErrorV1::Authority(format!(
            "assembled target inventory exceeds compiled upper bound {compiled_upper_bound}"
        )));
    }
    Ok((output, warnings))
}

fn validate_refit_receipt_v2(
    profile: ConditionalProcessProfileV2,
    receipt: &ConditionalProcessFullRefitReceiptV2,
) -> Result<(), MultiModRefitFailureV1> {
    if receipt.contract != CONDITIONAL_PROCESS_REFIT_CONTRACT_V2
        || !receipt.measurement_and_scores_refit
        || !receipt.deterministic_score_orientation
        || !receipt.all_joint_structural_equations_refit
        || !receipt.interaction_products_rebuilt
        || !receipt.raw_scientific_gamma_and_delta
    {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.receipt_incomplete".into(),
            message: "the full-refit receipt omitted a mandatory PLS/scoring/joint-product stage"
                .into(),
        });
    }
    match profile {
        ConditionalProcessProfileV2::MultipleHocPercentile
            if !receipt.hoc_dependency_stages_refit =>
        {
            Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.hoc_dependency_stages".into(),
                message: "the multi-HOC refit did not repeat every dependency stage".into(),
            })
        }
        ConditionalProcessProfileV2::GroupedPercentile
            if !receipt.group_isolation_preserved =>
        {
            Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.group_isolation".into(),
                message: "the grouped refit did not preserve its configured group isolation"
                    .into(),
            })
        }
        ConditionalProcessProfileV2::CaseWeightedPercentile
            if !receipt.positive_case_weights_applied_to_all_stages =>
        {
            Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.case_weight_semantics".into(),
                message: "positive case weights were not applied to every scoring/product/structural stage"
                    .into(),
            })
        }
        ConditionalProcessProfileV2::FrequencyWeightedPercentile
            if !receipt.frequency_counts_applied_without_physical_expansion
                || !receipt.frequency_fit_exactly_equivalent_to_row_expansion =>
        {
            Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.frequency_count_space_semantics".into(),
                message: "frequency refit lacks exact compact count-space row-expansion equivalence"
                    .into(),
            })
        }
        _ => Ok(()),
    }
}

fn validate_edge_interaction_inventory_v2(
    edges: &[ConditionalEdgeFunctionV2],
    interactions: &[CompiledInteractionIdentityV1],
) -> Result<(), MultiModRefitFailureV1> {
    type PairKey = (String, String);
    let edge_by_relation = edges
        .iter()
        .map(|edge| (edge.relation_id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    if edge_by_relation.len() != edges.len() {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.interaction_inventory_mismatch".into(),
            message: "interaction inventory cannot bind duplicate relation edges".into(),
        });
    }
    let mut expected_linear = BTreeMap::<String, BTreeSet<String>>::new();
    let mut expected_pairwise = BTreeMap::<String, BTreeSet<PairKey>>::new();
    for interaction in interactions {
        let edge = edge_by_relation
            .get(interaction.focal_relation_id.as_str())
            .ok_or_else(|| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.interaction_inventory_mismatch".into(),
                message: format!(
                    "compiled interaction {} lacks focal relation {}",
                    interaction.term_id, interaction.focal_relation_id
                ),
            })?;
        match interaction.operands.as_slice() {
            [focal, moderator] if &edge.source_id == focal => {
                expected_linear
                    .entry(interaction.focal_relation_id.clone())
                    .or_default()
                    .insert(moderator.clone());
            }
            [focal, first, second] if &edge.source_id == focal && first != second => {
                let mut pair = [first.clone(), second.clone()];
                pair.sort();
                expected_pairwise
                    .entry(interaction.focal_relation_id.clone())
                    .or_default()
                    .insert((pair[0].clone(), pair[1].clone()));
            }
            _ => {
                return Err(MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.interaction_inventory_mismatch".into(),
                    message: format!(
                        "compiled interaction {} has an invalid focal source or bounded order",
                        interaction.term_id
                    ),
                });
            }
        }
    }
    for edge in edges {
        let actual_linear = edge
            .linear_coefficients
            .iter()
            .map(|coefficient| coefficient.moderator_id.clone())
            .collect::<BTreeSet<_>>();
        let actual_pairwise = edge
            .pairwise_coefficients
            .iter()
            .map(|coefficient| {
                let mut pair = [
                    coefficient.first_moderator_id.clone(),
                    coefficient.second_moderator_id.clone(),
                ];
                pair.sort();
                (pair[0].clone(), pair[1].clone())
            })
            .collect::<BTreeSet<_>>();
        let expected_linear_for_edge = expected_linear
            .get(&edge.relation_id)
            .cloned()
            .unwrap_or_default();
        let expected_pairwise_for_edge = expected_pairwise
            .get(&edge.relation_id)
            .cloned()
            .unwrap_or_default();
        if actual_linear != expected_linear_for_edge
            || actual_pairwise != expected_pairwise_for_edge
        {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.interaction_inventory_mismatch".into(),
                message: format!(
                    "relation {} returned gamma/delta keys outside the exact compiled interaction inventory",
                    edge.relation_id
                ),
            });
        }
    }
    Ok(())
}

fn validate_refit_point_v2(
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    point: &ConditionalProcessFullRefitPointV2,
) -> Result<(), MultiModRefitFailureV1> {
    validate_refit_receipt_v2(config.profile, &point.receipt)?;
    let (paths, interactions, _) =
        conditional_plan_parts_v2(artifact).map_err(|error| MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.authority".into(),
            message: error.to_string(),
        })?;
    let required_relations = paths
        .iter()
        .flat_map(|path| path.ordered_relation_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    let model_relations = structural_bindings_v2(model)
        .into_iter()
        .map(|relation| (relation.relation_id.clone(), relation))
        .collect::<BTreeMap<_, _>>();
    let mut actual_relations = BTreeMap::new();
    for edge in &point.edges {
        let Some(binding) = model_relations.get(&edge.relation_id) else {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.unknown_relation".into(),
                message: format!("refit returned unknown relation {}", edge.relation_id),
            });
        };
        if edge.source_id != binding.source_id
            || edge.target_id != binding.target_id
            || !edge.intercept.is_finite()
            || edge
                .linear_coefficients
                .iter()
                .any(|coefficient| !coefficient.estimate.is_finite())
            || edge
                .pairwise_coefficients
                .iter()
                .any(|coefficient| !coefficient.estimate.is_finite())
            || actual_relations
                .insert(edge.relation_id.as_str(), edge)
                .is_some()
        {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.edge_contract".into(),
                message: format!(
                    "refit edge {} has a duplicate, nonfinite value, or endpoint mismatch",
                    edge.relation_id
                ),
            });
        }
        let mut linear = BTreeSet::new();
        if edge
            .linear_coefficients
            .iter()
            .any(|coefficient| !linear.insert(coefficient.moderator_id.as_str()))
        {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.duplicate_gamma".into(),
                message: format!(
                    "relation {} has duplicate moderator gammas",
                    edge.relation_id
                ),
            });
        }
        let mut pairs = BTreeSet::new();
        if edge.pairwise_coefficients.iter().any(|coefficient| {
            let mut pair = [
                coefficient.first_moderator_id.as_str(),
                coefficient.second_moderator_id.as_str(),
            ];
            pair.sort();
            pair[0] == pair[1] || !pairs.insert(pair)
        }) {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.duplicate_delta".into(),
                message: format!(
                    "relation {} has duplicate moderator deltas",
                    edge.relation_id
                ),
            });
        }
    }
    if required_relations
        .iter()
        .any(|relation_id| !actual_relations.contains_key(relation_id.as_str()))
    {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.target_relation_missing".into(),
            message: "refit omitted a relation used by an explicitly selected path".into(),
        });
    }
    validate_edge_interaction_inventory_v2(&point.edges, interactions)?;
    Ok(())
}

fn verify_raw_probe_orientations_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    point: &ConditionalProcessFullRefitPointV2,
) -> Result<(), MultiModRunnerErrorV1> {
    for probe in &config.probes {
        if probe.scale != ConditionalProbeScaleV2::RawObservedWithTransformationReceipt {
            continue;
        }
        let orientation_sign = if probe.raw_fit_metric_receipts.is_empty() {
            probe
                .raw_transformation_receipt
                .as_ref()
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::Authority("raw probe receipt disappeared".into())
                })?
                .orientation_sign
        } else {
            raw_probe_fit_metric_receipt_for_stratum_v2(probe, stratum)?.orientation_sign
        };
        if point
            .observed_moderator_orientation_signs
            .get(&probe.moderator_id)
            != Some(&orientation_sign)
        {
            return Err(MultiModRunnerErrorV1::Authority(format!(
                "raw probe {} orientation sign does not reproduce from the observed point fit",
                probe.moderator_id
            )));
        }
    }
    Ok(())
}

fn compile_path_polynomials_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    edges: &[ConditionalEdgeFunctionV2],
) -> Result<BTreeMap<String, ConditionalPathPolynomialV2>, MultiModRefitFailureV1> {
    let edges = edges
        .iter()
        .map(|edge| (edge.relation_id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    config
        .paths
        .iter()
        .map(|path| {
            let path_edges = path
                .ordered_relation_ids
                .iter()
                .map(|relation_id| {
                    edges
                        .get(relation_id.as_str())
                        .copied()
                        .cloned()
                        .ok_or_else(|| MultiModRefitFailureV1 {
                            code: "multimod.conditional.refit.target_relation_missing".into(),
                            message: format!(
                                "selected path {} lacks relation {relation_id}",
                                path.path_id
                            ),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let polynomial = compile_explicit_conditional_path_v2(&ExplicitConditionalPathV2 {
                path_id: path.path_id.clone(),
                edges: path_edges,
            })
            .map_err(|error| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.polynomial".into(),
                message: error.to_string(),
            })?;
            Ok((path.path_id.clone(), polynomial))
        })
        .collect()
}

fn evaluate_polynomial_derivative_v2(
    polynomial: &ConditionalPathPolynomialV2,
    probe: &ConditionalProbePointV2,
    derivative_variables: &[String],
) -> Result<f64, MultiModRefitFailureV1> {
    let mut derivative_orders = BTreeMap::<String, u8>::new();
    for moderator in derivative_variables {
        *derivative_orders.entry(moderator.clone()).or_default() += 1;
    }
    let mut total = 0.0;
    for term in &polynomial.terms {
        let powers = term
            .powers
            .iter()
            .map(|power| (power.moderator_id.as_str(), power.exponent))
            .collect::<BTreeMap<_, _>>();
        let mut value = term.coefficient;
        let mut eliminated = false;
        let moderators = powers
            .keys()
            .copied()
            .chain(derivative_orders.keys().map(String::as_str))
            .collect::<BTreeSet<_>>();
        for moderator in moderators {
            let exponent = powers.get(moderator).copied().unwrap_or(0);
            let derivative_order = derivative_orders.get(moderator).copied().unwrap_or(0);
            if derivative_order > exponent {
                eliminated = true;
                break;
            }
            for factor in 0..derivative_order {
                value *= f64::from(exponent - factor);
            }
            let remaining = exponent - derivative_order;
            if remaining > 0 {
                let probe_value = probe
                    .standardized_values
                    .get(moderator)
                    .copied()
                    .ok_or_else(|| MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.probe_value_missing".into(),
                        message: format!("probe {} lacks {moderator}", probe.probe_id),
                    })?;
                value *= probe_value.powi(i32::from(remaining));
            }
        }
        if !eliminated {
            total += value;
        }
    }
    if total.is_finite() {
        Ok(total)
    } else {
        Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.nonfinite_target".into(),
            message: "conditional derivative is nonfinite".into(),
        })
    }
}

fn evaluate_polynomial_v2(
    polynomial: &ConditionalPathPolynomialV2,
    probe: &ConditionalProbePointV2,
) -> Result<f64, MultiModRefitFailureV1> {
    conditional_effect_v2(polynomial, probe)
        .map(|effect| effect.estimate)
        .map_err(|error| MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.conditional_effect".into(),
            message: error.to_string(),
        })
}

fn evaluate_target_templates_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    edges: &[ConditionalEdgeFunctionV2],
    templates: &[ConditionalTargetTemplateV2],
) -> Result<Vec<f64>, MultiModRefitFailureV1> {
    let polynomials = compile_path_polynomials_v2(config, edges)?;
    let edges_by_id = edges
        .iter()
        .map(|edge| (edge.relation_id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    let values = templates
        .iter()
        .map(|target| match &target.operation {
            ConditionalTargetOperationV2::Specific { path_id, probe } => {
                evaluate_polynomial_v2(&polynomials[path_id], probe)
            }
            ConditionalTargetOperationV2::TotalIndirect { path_ids, probe } => {
                path_ids.iter().try_fold(0.0, |sum, path_id| {
                    Ok(sum + evaluate_polynomial_v2(&polynomials[path_id], probe)?)
                })
            }
            ConditionalTargetOperationV2::TotalEffect {
                path_ids,
                direct_relation_id,
                probe,
            } => {
                let indirect = path_ids.iter().try_fold(0.0, |sum, path_id| {
                    Ok(sum + evaluate_polynomial_v2(&polynomials[path_id], probe)?)
                })?;
                let edge = edges_by_id
                    .get(direct_relation_id.as_str())
                    .ok_or_else(|| MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.direct_relation_missing".into(),
                        message: format!(
                            "total effect lacks authored direct relation {direct_relation_id}"
                        ),
                    })?;
                let polynomial = compile_explicit_conditional_path_v2(&ExplicitConditionalPathV2 {
                    path_id: format!("direct:{direct_relation_id}"),
                    edges: vec![(*edge).clone()],
                })
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.direct_polynomial".into(),
                    message: error.to_string(),
                })?;
                let direct = evaluate_polynomial_v2(&polynomial, probe)?;
                Ok(indirect + direct)
            }
            ConditionalTargetOperationV2::ScalarIndex {
                path_id,
                moderator_id,
            } => scalar_index_of_moderated_mediation_v2(&polynomials[path_id], moderator_id)
                .map(|index| index.estimate)
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.scalar_index".into(),
                    message: error.to_string(),
                }),
            ConditionalTargetOperationV2::Derivative {
                path_id,
                probe,
                derivative_variables,
            } => evaluate_polynomial_derivative_v2(
                &polynomials[path_id],
                probe,
                derivative_variables,
            ),
            ConditionalTargetOperationV2::ProbeContrast {
                path_id,
                left,
                right,
                ..
            } => Ok(evaluate_polynomial_v2(&polynomials[path_id], left)?
                - evaluate_polynomial_v2(&polynomials[path_id], right)?),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != templates.len() || values.iter().any(|value| !value.is_finite()) {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.target_inventory".into(),
            message: "refit returned a nonfinite or mis-sized ordered target vector".into(),
        });
    }
    Ok(values)
}

fn combine_group_vectors_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    templates: &[ConditionalTargetTemplateV2],
    vectors: &BTreeMap<String, Vec<f64>>,
) -> Result<Vec<f64>, MultiModRefitFailureV1> {
    let mut output = Vec::new();
    let mut group_ids = config
        .groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<Vec<_>>();
    group_ids.sort();
    for group_id in &group_ids {
        let vector = vectors
            .get(*group_id)
            .ok_or_else(|| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.group_vector_missing".into(),
                message: format!("group {group_id} has no target vector"),
            })?;
        if vector.len() != templates.len() {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.target_inventory".into(),
                message: format!("group {group_id} returned the wrong target dimension"),
            });
        }
        output.extend(vector);
    }
    let mut contrasts = config.group_contrasts.iter().collect::<Vec<_>>();
    contrasts.sort_by(|left, right| left.contrast_id.cmp(&right.contrast_id));
    for contrast in contrasts {
        let left = &vectors[&contrast.left_group_id];
        let right = &vectors[&contrast.right_group_id];
        for (index, template) in templates.iter().enumerate() {
            if template.kind == ConditionalProcessTargetKindV2::ProbeContrast {
                continue;
            }
            output.push(left[index] - right[index]);
        }
    }
    if output.iter().all(|value| value.is_finite()) {
        Ok(output)
    } else {
        Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.nonfinite_group_contrast".into(),
            message: "group contrast vector is nonfinite".into(),
        })
    }
}

fn point_targets_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    templates: &[ConditionalTargetTemplateV2],
    point_vectors: &BTreeMap<Option<String>, Vec<f64>>,
) -> Result<Vec<ConditionalProcessTargetResultV2>, MultiModRunnerErrorV1> {
    let mut output = Vec::new();
    if config.profile == ConditionalProcessProfileV2::GroupedPercentile {
        let mut group_ids = config
            .groups
            .iter()
            .map(|group| group.group_id.as_str())
            .collect::<Vec<_>>();
        group_ids.sort();
        for group_id in &group_ids {
            let values = point_vectors
                .get(&Some((*group_id).to_owned()))
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::Kernel(format!(
                        "observed point vector is absent for group {group_id}"
                    ))
                })?;
            for (template, estimate) in templates.iter().zip(values) {
                output.push(ConditionalProcessTargetResultV2 {
                    target_id: local_target_id_v2(Some(group_id), &template.target_id),
                    kind: template.kind.clone(),
                    path_id: template.path_id.clone(),
                    group_id: Some((*group_id).to_owned()),
                    probe_values: template.probe_values.clone(),
                    derivative_variables: template.derivative_variables.clone(),
                    estimate: *estimate,
                    p_value: None,
                    interval: None,
                    usable_replicates: 0,
                });
            }
        }
        let vectors = point_vectors
            .iter()
            .filter_map(|(group, values)| {
                group.as_ref().map(|group| (group.clone(), values.clone()))
            })
            .collect::<BTreeMap<_, _>>();
        let mut contrasts = config.group_contrasts.iter().collect::<Vec<_>>();
        contrasts.sort_by(|left, right| left.contrast_id.cmp(&right.contrast_id));
        for contrast in contrasts {
            let left = &vectors[&contrast.left_group_id];
            let right = &vectors[&contrast.right_group_id];
            for (index, template) in templates.iter().enumerate() {
                if template.kind == ConditionalProcessTargetKindV2::ProbeContrast {
                    continue;
                }
                output.push(ConditionalProcessTargetResultV2 {
                    target_id: group_contrast_target_id_v2(
                        &contrast.contrast_id,
                        &contrast.left_group_id,
                        &contrast.right_group_id,
                        &template.target_id,
                    ),
                    kind: ConditionalProcessTargetKindV2::GroupContrast,
                    path_id: template.path_id.clone(),
                    group_id: None,
                    probe_values: template.probe_values.clone(),
                    derivative_variables: template.derivative_variables.clone(),
                    estimate: left[index] - right[index],
                    p_value: None,
                    interval: None,
                    usable_replicates: 0,
                });
            }
        }
    } else {
        let values = point_vectors.get(&None).ok_or_else(|| {
            MultiModRunnerErrorV1::Kernel("observed point vector is absent".into())
        })?;
        for (template, estimate) in templates.iter().zip(values) {
            output.push(ConditionalProcessTargetResultV2 {
                target_id: template.target_id.clone(),
                kind: template.kind.clone(),
                path_id: template.path_id.clone(),
                group_id: None,
                probe_values: template.probe_values.clone(),
                derivative_variables: template.derivative_variables.clone(),
                estimate: *estimate,
                p_value: None,
                interval: None,
                usable_replicates: 0,
            });
        }
    }
    if output.is_empty()
        || output.iter().any(|target| !target.estimate.is_finite())
        || output
            .iter()
            .map(|target| target.target_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != output.len()
    {
        return Err(MultiModRunnerErrorV1::Kernel(
            "observed target inventory is empty, duplicated, or nonfinite".into(),
        ));
    }
    Ok(output)
}

#[derive(Debug, Clone)]
struct BuiltInConditionalAuthorityV2 {
    point_recipe: AnalysisRecipeV4,
    point_model: SemModelV4,
    point_artifact: CompiledAnalysisRecipeV4,
    plan: CompiledPlsPlanV3,
    source_columns: Vec<String>,
    raw_probe_sources: BTreeMap<String, String>,
}

fn remove_runtime_binding_variable_v2(
    model: &mut SemModelV4,
    source_column: &str,
) -> Result<(), MultiModRunnerErrorV1> {
    let Some(variable_id) = model.variables.iter().find_map(|variable| match variable {
        SemVariableV4::Observed {
            id,
            source_column: candidate,
            ..
        } if candidate == source_column => Some(id.clone()),
        _ => None,
    }) else {
        return Err(MultiModRunnerErrorV1::Authority(format!(
            "runtime binding column {source_column} has no observed-variable identity"
        )));
    };
    let used_by_relation = model.relations.iter().any(|relation| match relation {
        SemRelationV4::MeasurementEffect {
            construct,
            indicator,
            ..
        } => construct == &variable_id || indicator == &variable_id,
        SemRelationV4::MeasurementCausal {
            indicator,
            composite,
            ..
        } => indicator == &variable_id || composite == &variable_id,
        SemRelationV4::Structural { source, target, .. } => {
            source == &variable_id || target == &variable_id
        }
        SemRelationV4::Covariance { left, right, .. } => {
            left.variable_id() == variable_id.as_str()
                || right.variable_id() == variable_id.as_str()
        }
    });
    let used_by_derived = model.derived_terms.iter().any(|term| match term {
        qpls_core::SemDerivedTermV4::Interaction {
            predictor,
            moderator,
            output,
            ..
        } => predictor == &variable_id || moderator == &variable_id || output == &variable_id,
        qpls_core::SemDerivedTermV4::InteractionV2 {
            operands, output, ..
        } => operands.contains(&variable_id) || output == &variable_id,
        qpls_core::SemDerivedTermV4::HigherOrder {
            components, output, ..
        } => components.contains(&variable_id) || output == &variable_id,
        qpls_core::SemDerivedTermV4::Polynomial { source, output, .. } => {
            source == &variable_id || output == &variable_id
        }
    });
    if used_by_relation || used_by_derived {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.runtime_binding_is_scientific_variable:{source_column}"
        )));
    }
    model
        .variables
        .retain(|variable| variable.id() != variable_id.as_str());
    Ok(())
}

fn built_in_conditional_authority_v2(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<BuiltInConditionalAuthorityV2, MultiModRunnerErrorV1> {
    let config = recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("conditional config absent".into()))?;
    if matches!(
        config.profile,
        ConditionalProcessProfileV2::MultipleHocPercentile
            | ConditionalProcessProfileV2::CaseWeightedPercentile
            | ConditionalProcessProfileV2::FrequencyWeightedPercentile
    ) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(match config.profile {
            ConditionalProcessProfileV2::MultipleHocPercentile =>
                "multimod.runner.conditional.external_disjoint_hoc_full_refitter_required",
            ConditionalProcessProfileV2::CaseWeightedPercentile =>
                "multimod.runner.conditional.external_positive_case_weight_full_refitter_required",
            ConditionalProcessProfileV2::FrequencyWeightedPercentile =>
                "multimod.runner.conditional.external_frequency_count_space_full_refitter_required",
            _ => unreachable!(),
        }
        .into()));
    }
    let general = recipe.general_sem_config.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.general_sem_config_required".into(),
        )
    })?;
    let mut scientific_model = model.clone();
    if let Some(grouping_column) = &config.grouping_column {
        remove_runtime_binding_variable_v2(&mut scientific_model, grouping_column)?;
        scientific_model.group = SemGroupV4::SingleGroup;
    }
    if let Some(weight) = &config.weight {
        let column = match weight {
            AnalysisWeightBindingV1::Case { column }
            | AnalysisWeightBindingV1::Frequency { column } => column,
        };
        remove_runtime_binding_variable_v2(&mut scientific_model, column)?;
    }
    if let SemDataBindingV4::Raw { weight, .. } = &mut scientific_model.data_binding {
        *weight = None;
    }
    scientific_model
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    let plan = compile_pls_plan_v3(&scientific_model, general).map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.general_sem_plan_rejected:{error}"
        ))
    })?;
    if plan.higher_order_stage_plans().len() > 0 {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.external_disjoint_hoc_full_refitter_required".into(),
        ));
    }
    let (mut point_recipe, point_model) =
        project_general_sem_pls_stage_one_recipe_v1(recipe, &scientific_model).map_err(
            |error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.conditional.stage_one_projection_rejected:{error}"
                ))
            },
        )?;
    point_recipe.settings.method = AnalysisMethod::PlsPm;
    point_recipe.settings.bootstrap_samples = 0;
    point_recipe.settings.permutation_samples = 0;
    point_recipe.settings.studentized_inner_samples = 0;
    point_recipe.settings.case_weight_column = None;
    point_recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    point_recipe.general_sem_config = None;
    point_recipe.mga_multigroup = None;
    point_recipe.pls_heterogeneity = None;
    point_recipe.general_sem_conditional_process = None;
    point_recipe.interventional_causal_mediation = None;
    let point_digest = point_model
        .scientific_sha256()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    point_recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        model: point_model.clone(),
        scientific_sha256: point_digest,
    };
    point_recipe
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let point_artifact = compile_analysis_recipe_v4(
        &point_recipe,
        Some(&point_model),
        target,
        target.capability_cell_for_recipe(&point_recipe),
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.stage_one_compilation_rejected:{error}"
        ))
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan: base_plan } = point_artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "conditional stage-one projection did not emit a PLS plan".into(),
        ));
    };
    if base_plan != plan.base_plan() {
        return Err(MultiModRunnerErrorV1::Authority(
            "conditional stage-one artifact differs from the compiled General SEM base plan".into(),
        ));
    }
    let mut source_columns = base_plan
        .blocks()
        .iter()
        .flat_map(|block| {
            block
                .indicators()
                .iter()
                .map(|indicator| indicator.source_column().to_owned())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let raw_probe_sources = raw_probe_source_bindings_v2(config)?;
    source_columns.extend(raw_probe_sources.values().cloned());
    source_columns.sort();
    source_columns.dedup();
    Ok(BuiltInConditionalAuthorityV2 {
        point_recipe,
        point_model,
        point_artifact,
        plan,
        source_columns,
        raw_probe_sources,
    })
}

fn edges_from_two_way_point_v2(
    point: &GeneralSemPlsMultipleInteractionPointResultV1,
) -> Vec<ConditionalEdgeFunctionV2> {
    point
        .structural_coefficients()
        .iter()
        .map(|coefficient| {
            let mut linear_coefficients = point
                .interaction_coefficients()
                .iter()
                .filter(|interaction| interaction.focal_relation_id() == coefficient.relation_id())
                .map(|interaction| ConditionalLinearCoefficientV2 {
                    moderator_id: interaction.moderator_id().to_owned(),
                    estimate: interaction.raw_product_estimate(),
                })
                .collect::<Vec<_>>();
            linear_coefficients.sort_by(|left, right| left.moderator_id.cmp(&right.moderator_id));
            ConditionalEdgeFunctionV2 {
                relation_id: coefficient.relation_id().into(),
                source_id: coefficient.source_id().into(),
                target_id: coefficient.target_id().into(),
                intercept: coefficient.estimate(),
                linear_coefficients,
                pairwise_coefficients: Vec::new(),
            }
        })
        .collect()
}

fn edges_from_three_way_point_v2(
    point: &GeneralSemPlsThreeWayPointResultV1,
) -> Vec<ConditionalEdgeFunctionV2> {
    point
        .structural_coefficients
        .iter()
        .map(|coefficient| {
            let mut linear_coefficients = point
                .lower_order_interaction_coefficients
                .iter()
                .filter(|interaction| interaction.focal_relation_id() == coefficient.relation_id())
                .map(|interaction| ConditionalLinearCoefficientV2 {
                    moderator_id: interaction.moderator_id().to_owned(),
                    estimate: interaction.raw_product_estimate(),
                })
                .collect::<Vec<_>>();
            linear_coefficients.sort_by(|left, right| left.moderator_id.cmp(&right.moderator_id));
            let pairwise_coefficients = (point.three_way_coefficient.focal_relation_id
                == coefficient.relation_id())
            .then(|| ConditionalPairwiseCoefficientV2 {
                first_moderator_id: point.three_way_coefficient.operand_ids[1].clone(),
                second_moderator_id: point.three_way_coefficient.operand_ids[2].clone(),
                estimate: point.three_way_coefficient.scientific_rescaled_delta,
            })
            .into_iter()
            .collect();
            ConditionalEdgeFunctionV2 {
                relation_id: coefficient.relation_id().into(),
                source_id: coefficient.source_id().into(),
                target_id: coefficient.target_id().into(),
                intercept: coefficient.estimate(),
                linear_coefficients,
                pairwise_coefficients,
            }
        })
        .collect()
}

fn compiled_two_way_contracts_v2(
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
) -> Result<Vec<MultimodConditionalTwoWayInteractionV2>, MultiModRunnerErrorV1> {
    let CompiledMultiModPlanV1::GeneralSemConditionalProcessV2 { interactions, .. } =
        artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not conditional process V2".into(),
        ));
    };
    let mut output = Vec::with_capacity(interactions.len());
    for interaction in interactions {
        let [focal_predictor_id, moderator_id] = interaction.operands.as_slice() else {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.conditional.two_way_contract_order:{}",
                interaction.term_id
            )));
        };
        let focal = model
            .relations
            .iter()
            .filter_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source,
                    target,
                    role: StructuralRelationRoleV4::Structural,
                    ..
                } if id == &interaction.focal_relation_id => {
                    Some((source.as_str(), target.as_str()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let [(focal_source, outcome_id)] = focal.as_slice() else {
            return Err(MultiModRunnerErrorV1::Authority(format!(
                "interaction {} has no unique authored focal structural relation",
                interaction.term_id
            )));
        };
        if *focal_source != focal_predictor_id {
            return Err(MultiModRunnerErrorV1::Authority(format!(
                "interaction {} focal operand differs from its authored relation",
                interaction.term_id
            )));
        }
        let effects = model
            .relations
            .iter()
            .filter_map(|relation| match relation {
                SemRelationV4::Structural {
                    id,
                    source,
                    target,
                    parameter,
                    role: StructuralRelationRoleV4::Structural,
                    intercept_parameter: None,
                } if source == &interaction.output_id && target == *outcome_id => {
                    Some((id.clone(), parameter.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let [(effect_relation_id, effect_parameter_id)] = effects.as_slice() else {
            return Err(MultiModRunnerErrorV1::Authority(format!(
                "interaction {} has no unique authored effect relation",
                interaction.term_id
            )));
        };
        output.push(MultimodConditionalTwoWayInteractionV2 {
            interaction_id: interaction.term_id.clone(),
            output_id: interaction.output_id.clone(),
            focal_relation_id: interaction.focal_relation_id.clone(),
            interaction_effect_relation_id: effect_relation_id.clone(),
            interaction_effect_parameter_id: effect_parameter_id.clone(),
            focal_predictor_id: focal_predictor_id.clone(),
            moderator_id: moderator_id.clone(),
            outcome_id: (*outcome_id).to_string(),
        });
    }
    output.sort_by(|left, right| left.interaction_id.cmp(&right.interaction_id));
    Ok(output)
}

fn covariance_sign_v2(left: &[f64], right: &[f64]) -> Option<i8> {
    if left.len() != right.len()
        || left.len() < 2
        || left.iter().chain(right).any(|value| !value.is_finite())
    {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let covariance = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - left_mean) * (right - right_mean))
        .sum::<f64>();
    if !covariance.is_finite() || covariance.abs() <= f64::EPSILON {
        None
    } else {
        Some(if covariance > 0.0 { 1 } else { -1 })
    }
}

fn row_mass_covariance_sign_v2(
    left: &[f64],
    right: &[f64],
    row_mass: MultimodConditionalRowMassV2<'_>,
) -> Option<i8> {
    if left.len() != right.len()
        || left.len() < 2
        || left.iter().chain(right).any(|value| !value.is_finite())
    {
        return None;
    }
    let masses = match row_mass {
        MultimodConditionalRowMassV2::Unweighted => vec![1.0; left.len()],
        MultimodConditionalRowMassV2::PositiveCase(weights) => {
            if weights.len() != left.len()
                || weights
                    .iter()
                    .any(|weight| !weight.is_finite() || *weight <= 0.0)
            {
                return None;
            }
            weights.to_vec()
        }
        MultimodConditionalRowMassV2::PositiveIntegerFrequency(counts) => {
            if counts.len() != left.len() || counts.iter().any(|count| *count == 0) {
                return None;
            }
            counts.iter().map(|count| *count as f64).collect()
        }
    };
    let mass_sum = masses.iter().sum::<f64>();
    if !mass_sum.is_finite() || mass_sum <= 0.0 {
        return None;
    }
    let left_mean = left
        .iter()
        .zip(&masses)
        .map(|(value, mass)| value * mass)
        .sum::<f64>()
        / mass_sum;
    let right_mean = right
        .iter()
        .zip(&masses)
        .map(|(value, mass)| value * mass)
        .sum::<f64>()
        / mass_sum;
    let covariance_numerator = left
        .iter()
        .zip(right)
        .zip(&masses)
        .map(|((left, right), mass)| mass * (left - left_mean) * (right - right_mean))
        .sum::<f64>();
    if !covariance_numerator.is_finite() || covariance_numerator.abs() <= f64::EPSILON {
        None
    } else {
        Some(if covariance_numerator > 0.0 { 1 } else { -1 })
    }
}

struct BuiltInConditionalRefitterV2<'a> {
    dataset: &'a Dataset,
    authority: BuiltInConditionalAuthorityV2,
}

impl ConditionalProcessFullRefitterV2 for BuiltInConditionalRefitterV2<'_> {
    fn full_refit(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1> {
        let ConditionalProcessRefitSampleV2::CaseRows {
            source_rows,
            normalized_case_weights,
        } = &request.sample
        else {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.builtin.frequency_unavailable".into(),
                message: "built-in unweighted PLS refitter does not accept frequency counts".into(),
            });
        };
        if normalized_case_weights.is_some() {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.builtin.case_weights_unavailable".into(),
                message: "built-in unweighted interaction refitter does not accept weights".into(),
            });
        }
        if is_cancelled() {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.cancelled".into(),
                message: "conditional point refit was cancelled".into(),
            });
        }
        let source_rows = source_rows
            .iter()
            .map(|row| *row as usize)
            .collect::<Vec<_>>();
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &source_rows,
            || is_cancelled(),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if error.to_string().contains("cancel") {
                "multimod.conditional.refit.cancelled"
            } else {
                "multimod.conditional.refit.row_projection"
            }
            .into(),
            message: error.to_string(),
        })?;
        let stage_one = run_compiled_pls_recipe_v4(
            &sampled,
            &self.authority.point_recipe,
            &self.authority.point_model,
            &self.authority.point_artifact,
            None,
            || is_cancelled(),
            |_| {},
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if error.to_string().contains("cancel") {
                "multimod.conditional.refit.cancelled"
            } else {
                "multimod.conditional.refit.pls_point"
            }
            .into(),
            message: error.to_string(),
        })?
        .estimation()
        .clone();
        if !stage_one.converged
            || stage_one.used_observations != sampled.batch.num_rows()
            || stage_one.omitted_observations != 0
        {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.estimator_did_not_converge".into(),
                message: "stage-one PLS did not converge on exactly the supplied complete rows"
                    .into(),
            });
        }
        let edges = match request.profile {
            ConditionalProcessProfileV2::BoundedThreeWayPercentile => {
                let point = estimate_general_sem_pls_three_way_moderation_v1_with_control(
                    &self.authority.plan,
                    &stage_one.construct_scores,
                    || !is_cancelled(),
                )
                .map_err(|error| match error {
                    GeneralSemPlsThreeWayPointErrorV1::Cancelled => MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.cancelled".into(),
                        message: error.to_string(),
                    },
                    _ => MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.three_way_joint".into(),
                        message: error.to_string(),
                    },
                })?;
                point
                    .ensure_valid_against_plan_v1(&self.authority.plan)
                    .map_err(|error| MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.three_way_receipt".into(),
                        message: error.to_string(),
                    })?;
                edges_from_three_way_point_v2(&point)
            }
            _ => {
                let point = estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
                    &self.authority.plan,
                    &stage_one.construct_scores,
                    || !is_cancelled(),
                )
                .map_err(|error| match error {
                    GeneralSemPlsInteractionPointErrorV1::Cancelled => MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.cancelled".into(),
                        message: error.to_string(),
                    },
                    _ => MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.two_way_joint".into(),
                        message: error.to_string(),
                    },
                })?;
                point
                    .ensure_valid_against_plan_v1(&self.authority.plan)
                    .map_err(|error| MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.two_way_receipt".into(),
                        message: error.to_string(),
                    })?;
                edges_from_two_way_point_v2(&point)
            }
        };
        let sampled_rows = qpls_data::preview_page(&sampled, 0, sampled.batch.num_rows());
        let mut observed_moderator_orientation_signs = BTreeMap::new();
        for (moderator_id, source_column) in &self.authority.raw_probe_sources {
            let raw = sampled_rows
                .iter()
                .map(|row| parsed_finite_v2(row, source_column))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.raw_probe_source".into(),
                    message: format!("raw moderator column {source_column} is invalid"),
                })?;
            let scores = stage_one
                .construct_scores
                .get(moderator_id)
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.raw_probe_score".into(),
                    message: format!("PLS fit omitted raw moderator score {moderator_id}"),
                })?;
            let sign = covariance_sign_v2(&raw, scores).ok_or_else(|| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.raw_probe_orientation".into(),
                message: format!("raw moderator {moderator_id} has undefined score orientation"),
            })?;
            observed_moderator_orientation_signs.insert(moderator_id.clone(), sign);
        }
        Ok(ConditionalProcessFullRefitPointV2 {
            edges,
            observed_moderator_orientation_signs,
            receipt: ConditionalProcessFullRefitReceiptV2 {
                contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
                measurement_and_scores_refit: true,
                deterministic_score_orientation: true,
                all_joint_structural_equations_refit: true,
                interaction_products_rebuilt: true,
                raw_scientific_gamma_and_delta: true,
                hoc_dependency_stages_refit: false,
                group_isolation_preserved: request.group_id.is_some(),
                positive_case_weights_applied_to_all_stages: false,
                frequency_counts_applied_without_physical_expansion: false,
                frequency_fit_exactly_equivalent_to_row_expansion: false,
            },
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltInWeightedConditionalAuthorityV2 {
    source_recipe: AnalysisRecipeV4,
    source_model: SemModelV4,
    requested_weight: AnalysisWeightBindingV1,
    point_authority: MultimodCompiledWeightedPlsRecipeV1,
    interactions: Vec<MultimodConditionalTwoWayInteractionV2>,
    source_columns: Vec<String>,
    raw_probe_sources: BTreeMap<String, String>,
}

fn built_in_weighted_conditional_authority_v2(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
) -> Result<BuiltInWeightedConditionalAuthorityV2, MultiModRunnerErrorV1> {
    let config = recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("conditional config absent".into()))?;
    if !matches!(
        config.profile,
        ConditionalProcessProfileV2::CaseWeightedPercentile
            | ConditionalProcessProfileV2::FrequencyWeightedPercentile
    ) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.weighted_profile_required".into(),
        ));
    }
    let requested_weight = config.weight.clone().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "weighted conditional profile lost its typed weight binding".into(),
        )
    })?;
    let point_authority =
        compile_multimod_weighted_pls_recipe_v4_v1(recipe, model, &requested_weight).map_err(
            |error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.conditional.weighted_point_compilation:{error}"
                ))
            },
        )?;
    let interactions = compiled_two_way_contracts_v2(model, artifact)?;
    let mut source_columns = point_authority
        .plan()
        .blocks()
        .iter()
        .flat_map(|block| {
            block
                .indicators()
                .iter()
                .map(|indicator| indicator.source_column().to_owned())
        })
        .collect::<BTreeSet<_>>();
    source_columns.insert(point_authority.receipt().weight_source_column().to_string());
    let raw_probe_sources = raw_probe_source_bindings_v2(config)?;
    source_columns.extend(raw_probe_sources.values().cloned());
    Ok(BuiltInWeightedConditionalAuthorityV2 {
        source_recipe: recipe.clone(),
        source_model: model.clone(),
        requested_weight,
        point_authority,
        interactions,
        source_columns: source_columns.into_iter().collect(),
        raw_probe_sources,
    })
}

struct BuiltInWeightedConditionalRefitterV2<'a> {
    dataset: &'a Dataset,
    authority: BuiltInWeightedConditionalAuthorityV2,
}

impl BuiltInWeightedConditionalRefitterV2<'_> {
    fn project_rows_v2(
        &self,
        source_rows: &[u32],
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<Dataset, MultiModRefitFailureV1> {
        let indices = source_rows
            .iter()
            .map(|row| {
                usize::try_from(*row)
                    .ok()
                    .filter(|row| *row < self.dataset.batch.num_rows())
                    .ok_or_else(|| MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.row_identity".into(),
                        message: "weighted refit row is outside the execution dataset".into(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &indices,
            || is_cancelled(),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if error.to_string().contains("cancel") {
                "multimod.conditional.refit.cancelled"
            } else {
                "multimod.conditional.refit.row_projection"
            }
            .into(),
            message: error.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
struct HocConditionalStageAuthorityV2 {
    recipe: AnalysisRecipeV4,
    model: SemModelV4,
    artifact: CompiledAnalysisRecipeV4,
    plan: CompiledPlsPlanV2,
}

#[derive(Debug, Clone)]
struct BuiltInHocConditionalAuthorityV2 {
    plan: CompiledPlsPlanV3,
    stages: Vec<HocConditionalStageAuthorityV2>,
    repeated_stage_index: Option<usize>,
    score_stage_index: Option<usize>,
    source_columns: Vec<String>,
    interactions: Vec<MultimodConditionalTwoWayInteractionV2>,
    raw_probe_sources: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct HocConditionalFitV2 {
    stages: Vec<PlsResult>,
}

impl HocConditionalFitV2 {
    fn final_result(&self) -> &PlsResult {
        self.stages
            .last()
            .expect("validated HOC conditional fit has a final stage")
    }
}

fn interaction_free_hoc_model_v2(
    model: &SemModelV4,
    interactions: &[MultimodConditionalTwoWayInteractionV2],
) -> Result<SemModelV4, MultiModRunnerErrorV1> {
    let outputs = interactions
        .iter()
        .map(|interaction| interaction.output_id.as_str())
        .collect::<BTreeSet<_>>();
    let effect_relations = interactions
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id.as_str())
        .collect::<BTreeSet<_>>();
    let effect_parameters = interactions
        .iter()
        .map(|interaction| interaction.interaction_effect_parameter_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut projected = model.clone();
    projected
        .variables
        .retain(|variable| !outputs.contains(variable.id()));
    projected
        .relations
        .retain(|relation| !effect_relations.contains(relation.id()));
    projected
        .parameters
        .retain(|parameter| !effect_parameters.contains(parameter.id()));
    projected
        .derived_terms
        .retain(|term| matches!(term, qpls_core::SemDerivedTermV4::HigherOrder { .. }));
    projected.annotations.clear();
    projected.presentation = Default::default();
    projected = projected.canonicalized();
    projected.ensure_valid().map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.hoc_interaction_projection:{error}"
        ))
    })?;
    Ok(projected)
}

fn compile_hoc_conditional_stage_v2(
    source_recipe: &AnalysisRecipeV4,
    model: SemModelV4,
) -> Result<HocConditionalStageAuthorityV2, MultiModRunnerErrorV1> {
    let mut recipe = source_recipe.clone();
    recipe.settings.method = AnalysisMethod::PlsPm;
    recipe.settings.bootstrap_samples = 0;
    recipe.settings.permutation_samples = 0;
    recipe.settings.studentized_inner_samples = 0;
    recipe.settings.case_weight_column = None;
    recipe.method_config = Some(MethodConfig::PlsAlgorithm);
    recipe.general_sem_config = None;
    recipe.mga_multigroup = None;
    recipe.pls_heterogeneity = None;
    recipe.general_sem_conditional_process = None;
    recipe.interventional_causal_mediation = None;
    recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: model
            .scientific_sha256()
            .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?,
        model: model.clone(),
    };
    recipe
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let artifact = compile_analysis_recipe_v4(
        &recipe,
        Some(&model),
        target,
        target.capability_cell_for_recipe(&recipe),
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.hoc_stage_compilation:{error}"
        ))
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "HOC conditional stage did not compile a PLS plan".into(),
        ));
    };
    Ok(HocConditionalStageAuthorityV2 {
        recipe,
        model,
        plan: plan.clone(),
        artifact,
    })
}

fn project_hoc_conditional_base_model_v2(
    scientific_model: &SemModelV4,
) -> Result<SemModelV4, MultiModRunnerErrorV1> {
    compile_pls_higher_order_lower_order_projection_multimod_v2(scientific_model)
        .map(|projection| projection.projected_model().clone())
        .map_err(|error| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.conditional.hoc_base_projection:{error}"
            ))
        })
}

fn built_in_hoc_conditional_authority_v2(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
) -> Result<BuiltInHocConditionalAuthorityV2, MultiModRunnerErrorV1> {
    let config = recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("conditional config absent".into()))?;
    if config.profile != ConditionalProcessProfileV2::MultipleHocPercentile {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.multiple_hoc_profile_required".into(),
        ));
    }
    let interactions = compiled_two_way_contracts_v2(model, artifact)?;
    let scientific_model = interaction_free_hoc_model_v2(model, &interactions)?;
    let general = recipe.general_sem_config.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.hoc_general_sem_config_required".into(),
        )
    })?;
    let plan = compile_pls_plan_v3_multimod_multiple_hoc_v2(&scientific_model, general).map_err(
        |error| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.conditional.hoc_plan_rejected:{error}"
            ))
        },
    )?;
    let hocs = plan.higher_order_stage_plans();
    if !(1..=4).contains(&hocs.len()) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.hoc_count_outside_one_to_four".into(),
        ));
    }
    let approach = hocs[0].approach();
    if hocs.iter().any(|hoc| hoc.approach() != approach) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.hoc_mixed_approaches".into(),
        ));
    }
    let CompiledMultiModPlanV1::GeneralSemConditionalProcessV2 {
        hocs: compiled_hocs,
        ..
    } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not conditional process V2".into(),
        ));
    };
    if compiled_hocs.len() != hocs.len() {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled HOC inventory differs from the HOC point plan".into(),
        ));
    }

    let base_model = project_hoc_conditional_base_model_v2(&scientific_model)?;
    let mut base_recipe = project_general_sem_pls_base_recipe_v1(recipe).map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.conditional.hoc_base_recipe:{error}"
        ))
    })?;
    base_recipe.general_sem_conditional_process = None;
    let base_stage = compile_hoc_conditional_stage_v2(&base_recipe, base_model)?;
    if &base_stage.plan != plan.base_plan() {
        return Err(MultiModRunnerErrorV1::Authority(
            "HOC conditional base stage differs from the compiled plan".into(),
        ));
    }
    let mut stages = vec![base_stage];
    let mut repeated_stage_index = None;
    let mut score_stage_index = None;
    if matches!(
        approach,
        HigherOrderConstructionApproachV4::RepeatedIndicators
            | HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators
            | HigherOrderConstructionApproachV4::EmbeddedTwoStage
    ) {
        let projection = compile_pls_higher_order_repeated_stage_projection_multimod_v2(
            &scientific_model,
            &plan,
        )
        .map_err(|error| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.conditional.hoc_repeated_projection:{error}"
            ))
        })?;
        repeated_stage_index = Some(stages.len());
        stages.push(compile_hoc_conditional_stage_v2(
            recipe,
            projection.projected_model().clone(),
        )?);
    }
    if matches!(
        approach,
        HigherOrderConstructionApproachV4::EmbeddedTwoStage
            | HigherOrderConstructionApproachV4::DisjointTwoStage
    ) {
        let projection =
            compile_pls_higher_order_score_stage_projection_multimod_v2(&scientific_model, &plan)
                .map_err(|error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.conditional.hoc_score_projection:{error}"
                ))
            })?;
        score_stage_index = Some(stages.len());
        stages.push(compile_hoc_conditional_stage_v2(
            recipe,
            projection.projected_model().clone(),
        )?);
    }
    let final_stage = stages.last().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("HOC conditional plan has no final stage".into())
    })?;
    for interaction in &interactions {
        if !final_stage.plan.paths().iter().any(|path| {
            path.relation_id() == interaction.focal_relation_id
                && path.source() == interaction.focal_predictor_id
                && path.target() == interaction.outcome_id
        }) {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.conditional.hoc_final_stage_lacks_focal_relation:{}",
                interaction.interaction_id
            )));
        }
    }
    let mut source_columns = plan
        .base_plan()
        .blocks()
        .iter()
        .flat_map(|block| {
            block
                .indicators()
                .iter()
                .map(|indicator| indicator.source_column().to_owned())
        })
        .collect::<BTreeSet<_>>();
    let raw_probe_sources = raw_probe_source_bindings_v2(config)?;
    source_columns.extend(raw_probe_sources.values().cloned());
    Ok(BuiltInHocConditionalAuthorityV2 {
        plan,
        stages,
        repeated_stage_index,
        score_stage_index,
        source_columns: source_columns.into_iter().collect(),
        interactions,
        raw_probe_sources,
    })
}

fn hoc_conditional_alias_specs_v2(
    authority: &BuiltInHocConditionalAuthorityV2,
) -> Vec<PlsAliasColumnSpecV1> {
    authority
        .plan
        .higher_order_stage_plans()
        .iter()
        .flat_map(|hoc| {
            hoc.component_mappings()
                .iter()
                .map(move |mapping| (hoc.output_variable_id(), mapping))
        })
        .flat_map(|(hoc_id, mapping)| {
            mapping
                .virtual_indicators()
                .iter()
                .map(move |indicator| PlsAliasColumnSpecV1 {
                    source_column_id: indicator.source_column().to_owned(),
                    generated_column_id: indicator.generated_source_column_id().to_owned(),
                    label: format!(
                        "Conditional-process repeated HOC indicator: {} <- {}",
                        hoc_id,
                        mapping.component_id()
                    ),
                })
        })
        .collect()
}

fn run_hoc_conditional_stage_v2(
    dataset: &Dataset,
    stage: &HocConditionalStageAuthorityV2,
    is_cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<PlsResult, MultiModRefitFailureV1> {
    let execution = run_compiled_pls_recipe_v4_allowing_isolated(
        dataset,
        &stage.recipe,
        &stage.model,
        &stage.artifact,
        || is_cancelled(),
        |_| {},
    )
    .map_err(|error| MultiModRefitFailureV1 {
        code: if error.to_string().contains("cancel") {
            "multimod.conditional.refit.cancelled"
        } else {
            "multimod.conditional.refit.hoc_stage"
        }
        .into(),
        message: error.to_string(),
    })?;
    let result = execution.estimation().clone();
    if !result.converged
        || result.used_observations != dataset.batch.num_rows()
        || result.omitted_observations != 0
    {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.hoc_nonconvergence".into(),
            message: "HOC dependency stage did not converge on all supplied rows".into(),
        });
    }
    Ok(result)
}

struct BuiltInHocConditionalRefitterV2<'a> {
    dataset: &'a Dataset,
    authority: BuiltInHocConditionalAuthorityV2,
    observed_source_rows: Vec<u32>,
    observed_positions: BTreeMap<u32, usize>,
    observed_reference: Option<HocConditionalFitV2>,
}

impl BuiltInHocConditionalRefitterV2<'_> {
    fn fit_sample_v2(
        &self,
        sampled: &Dataset,
        sampled_positions: Option<&[usize]>,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<HocConditionalFitV2, MultiModRefitFailureV1> {
        let mut stages = Vec::with_capacity(self.authority.stages.len());
        stages.push(run_hoc_conditional_stage_v2(
            sampled,
            &self.authority.stages[0],
            is_cancelled,
        )?);
        if let Some(stage_index) = self.authority.repeated_stage_index {
            let aliases = append_pls_alias_columns_v1(
                sampled,
                &hoc_conditional_alias_specs_v2(&self.authority),
            )
            .map_err(|error| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.hoc_aliases".into(),
                message: error.to_string(),
            })?;
            let mut repeated = run_hoc_conditional_stage_v2(
                &aliases,
                &self.authority.stages[stage_index],
                is_cancelled,
            )?;
            if let (Some(reference), Some(positions)) =
                (&self.observed_reference, sampled_positions)
            {
                align_general_sem_pls_hoc_result_signs_v1(
                    &mut repeated,
                    GeneralSemPlsHocScoreAlignmentReferenceV1::new(
                        &reference.stages[stage_index].construct_scores,
                        positions,
                    ),
                    &|| is_cancelled(),
                )
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.hoc_orientation".into(),
                    message: error.to_string(),
                })?;
            }
            stages.push(repeated);
        }
        if let Some(stage_index) = self.authority.score_stage_index {
            let score_source_index = self.authority.repeated_stage_index.unwrap_or(0);
            let prepared = prepare_general_sem_pls_disjoint_hoc_score_dataset_multimod_v2(
                sampled,
                &self.authority.plan,
                &stages[score_source_index],
                || !is_cancelled(),
            )
            .map_err(|error| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.hoc_score_dataset".into(),
                message: error.to_string(),
            })?;
            let mut score = run_hoc_conditional_stage_v2(
                prepared.dataset(),
                &self.authority.stages[stage_index],
                is_cancelled,
            )?;
            if let (Some(reference), Some(positions)) =
                (&self.observed_reference, sampled_positions)
            {
                align_general_sem_pls_hoc_result_signs_v1(
                    &mut score,
                    GeneralSemPlsHocScoreAlignmentReferenceV1::new(
                        &reference.stages[stage_index].construct_scores,
                        positions,
                    ),
                    &|| is_cancelled(),
                )
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.hoc_orientation".into(),
                    message: error.to_string(),
                })?;
            }
            stages.push(score);
        }
        if stages.len() != self.authority.stages.len() {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.hoc_stage_cardinality".into(),
                message: "HOC refit omitted a dependency stage".into(),
            });
        }
        Ok(HocConditionalFitV2 { stages })
    }
}

impl ConditionalProcessFullRefitterV2 for BuiltInHocConditionalRefitterV2<'_> {
    fn full_refit(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1> {
        let ConditionalProcessRefitSampleV2::CaseRows {
            source_rows,
            normalized_case_weights: None,
        } = &request.sample
        else {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.hoc_sample_contract".into(),
                message: "multiple-HOC profile accepts unweighted case rows only".into(),
            });
        };
        let sampled_positions = if request.observed_fit {
            if self.observed_reference.is_some() {
                return Err(MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.hoc_observed_duplicate".into(),
                    message: "HOC observed reference was requested more than once".into(),
                });
            }
            None
        } else {
            Some(
                source_rows
                    .iter()
                    .map(|row| {
                        self.observed_positions.get(row).copied().ok_or_else(|| {
                            MultiModRefitFailureV1 {
                                code: "multimod.conditional.refit.hoc_row_universe".into(),
                                message: "HOC resample row is outside its observed universe".into(),
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
        };
        let indices = source_rows
            .iter()
            .map(|row| {
                usize::try_from(*row)
                    .ok()
                    .filter(|row| *row < self.dataset.batch.num_rows())
                    .ok_or_else(|| MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.hoc_row_identity".into(),
                        message: "HOC source row is outside the execution dataset".into(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &indices,
            || is_cancelled(),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.hoc_row_projection".into(),
            message: error.to_string(),
        })?;
        let fit = self.fit_sample_v2(&sampled, sampled_positions.as_deref(), is_cancelled)?;
        let final_plan = &self
            .authority
            .stages
            .last()
            .expect("HOC authority has a final stage")
            .plan;
        let interaction_point = estimate_multimod_conditional_interactions_v2_with_control(
            final_plan,
            &self.authority.interactions,
            &fit.final_result().construct_scores,
            MultimodConditionalRowMassV2::Unweighted,
            || !is_cancelled(),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.hoc_joint_interactions".into(),
            message: error.to_string(),
        })?;
        let sampled_rows = qpls_data::preview_page(&sampled, 0, sampled.batch.num_rows());
        let mut observed_moderator_orientation_signs = BTreeMap::new();
        for (moderator_id, source_column) in &self.authority.raw_probe_sources {
            let raw = sampled_rows
                .iter()
                .map(|row| parsed_finite_v2(row, source_column))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.hoc_raw_probe_source".into(),
                    message: format!("raw moderator column {source_column} is invalid"),
                })?;
            let scores = fit
                .final_result()
                .construct_scores
                .get(moderator_id)
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.hoc_raw_probe_score".into(),
                    message: format!("HOC fit omitted raw moderator score {moderator_id}"),
                })?;
            let sign = covariance_sign_v2(&raw, scores).ok_or_else(|| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.hoc_raw_probe_orientation".into(),
                message: format!("raw moderator {moderator_id} has undefined orientation"),
            })?;
            observed_moderator_orientation_signs.insert(moderator_id.clone(), sign);
        }
        if request.observed_fit {
            self.observed_source_rows = source_rows.clone();
            self.observed_positions = source_rows
                .iter()
                .enumerate()
                .map(|(position, row)| (*row, position))
                .collect();
            if self.observed_positions.len() != source_rows.len() {
                return Err(MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.hoc_observed_rows".into(),
                    message: "HOC observed source rows are not unique".into(),
                });
            }
            self.observed_reference = Some(fit);
        }
        Ok(ConditionalProcessFullRefitPointV2 {
            edges: interaction_point.edges,
            observed_moderator_orientation_signs,
            receipt: ConditionalProcessFullRefitReceiptV2 {
                contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
                measurement_and_scores_refit: true,
                deterministic_score_orientation: true,
                all_joint_structural_equations_refit: true,
                interaction_products_rebuilt: true,
                raw_scientific_gamma_and_delta: true,
                hoc_dependency_stages_refit: true,
                group_isolation_preserved: false,
                positive_case_weights_applied_to_all_stages: false,
                frequency_counts_applied_without_physical_expansion: false,
                frequency_fit_exactly_equivalent_to_row_expansion: false,
            },
        })
    }
}

impl ConditionalProcessFullRefitterV2 for BuiltInWeightedConditionalRefitterV2<'_> {
    fn full_refit(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1> {
        if is_cancelled() {
            return Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.cancelled".into(),
                message: "weighted conditional refit was cancelled".into(),
            });
        }
        let (prepared, row_mass, case_weighted, frequency_weighted) = match &request.sample {
            ConditionalProcessRefitSampleV2::CaseRows {
                source_rows,
                normalized_case_weights: Some(weights),
            } if request.profile == ConditionalProcessProfileV2::CaseWeightedPercentile => {
                if weights.len() != source_rows.len()
                    || weights
                        .iter()
                        .any(|weight| !weight.is_finite() || *weight <= 0.0)
                {
                    return Err(MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.case_weight_identity".into(),
                        message: "case-weight vector differs from sampled rows".into(),
                    });
                }
                let sampled = self.project_rows_v2(source_rows, is_cancelled)?;
                let prepared = prepare_multimod_case_weight_dataset_v1(
                    &sampled,
                    self.authority
                        .point_authority
                        .receipt()
                        .weight_source_column(),
                )
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.case_weight_preparation".into(),
                    message: error.to_string(),
                })?
                .0;
                (
                    prepared,
                    MultimodConditionalRowMassV2::PositiveCase(weights),
                    true,
                    false,
                )
            }
            ConditionalProcessRefitSampleV2::FrequencyCounts {
                source_rows,
                counts,
            } if request.profile == ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
                let (positive_rows, positive_counts, represented_total) =
                    compact_positive_frequency_cells_v2(source_rows, counts)?;
                let sampled = self.project_rows_v2(&positive_rows, is_cancelled)?;
                let (prepared, preparation) = prepare_multimod_frequency_count_dataset_v1(
                    &sampled,
                    self.authority
                        .point_authority
                        .receipt()
                        .weight_source_column(),
                    &positive_counts,
                )
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.frequency_preparation".into(),
                    message: error.to_string(),
                })?;
                if preparation.frequency_total != Some(represented_total)
                    || !preparation.exact_integer_count_space
                {
                    return Err(MultiModRefitFailureV1 {
                        code: "multimod.conditional.refit.frequency_preparation_receipt".into(),
                        message:
                            "frequency preparation receipt changed the represented count total"
                                .into(),
                    });
                }
                // Store counts beside the prepared dataset for the joint
                // interaction equation without ever materializing rows.
                return self.finish_frequency_refit_v2(
                    request,
                    prepared,
                    &positive_counts,
                    is_cancelled,
                );
            }
            _ => {
                return Err(MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.weight_sample_contract".into(),
                    message: "weighted profile received another sample representation".into(),
                });
            }
        };
        self.finish_weighted_refit_v2(
            request,
            prepared,
            row_mass,
            case_weighted,
            frequency_weighted,
            is_cancelled,
        )
    }
}

impl BuiltInWeightedConditionalRefitterV2<'_> {
    fn finish_frequency_refit_v2(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        prepared: Dataset,
        counts: &[u64],
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1> {
        self.finish_weighted_refit_v2(
            request,
            prepared,
            MultimodConditionalRowMassV2::PositiveIntegerFrequency(counts),
            false,
            true,
            is_cancelled,
        )
    }

    fn finish_weighted_refit_v2(
        &mut self,
        request: &ConditionalProcessFullRefitRequestV2,
        prepared: Dataset,
        row_mass: MultimodConditionalRowMassV2<'_>,
        case_weighted: bool,
        frequency_weighted: bool,
        is_cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<ConditionalProcessFullRefitPointV2, MultiModRefitFailureV1> {
        let weighted = run_compiled_multimod_weighted_pls_point_v1(
            &prepared,
            &self.authority.source_recipe,
            &self.authority.source_model,
            &self.authority.requested_weight,
            &self.authority.point_authority,
            || is_cancelled(),
            |_| {},
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if error.to_string().contains("cancel") {
                "multimod.conditional.refit.cancelled"
            } else {
                "multimod.conditional.refit.weighted_pls_point"
            }
            .into(),
            message: error.to_string(),
        })?;
        let sampled_rows = qpls_data::preview_page(&prepared, 0, prepared.batch.num_rows());
        let mut observed_moderator_orientation_signs = BTreeMap::new();
        for (moderator_id, source_column) in &self.authority.raw_probe_sources {
            let raw = sampled_rows
                .iter()
                .map(|row| parsed_finite_v2(row, source_column))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.weighted_raw_probe_source".into(),
                    message: format!("raw moderator column {source_column} is invalid"),
                })?;
            let scores = weighted
                .estimation
                .construct_scores
                .get(moderator_id)
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.weighted_raw_probe_score".into(),
                    message: format!("weighted PLS fit omitted raw moderator {moderator_id}"),
                })?;
            let sign = row_mass_covariance_sign_v2(&raw, scores, row_mass).ok_or_else(|| {
                MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.weighted_raw_probe_orientation".into(),
                    message: format!(
                        "weighted raw moderator {moderator_id} has undefined score orientation"
                    ),
                }
            })?;
            observed_moderator_orientation_signs.insert(moderator_id.clone(), sign);
        }
        let point = estimate_multimod_conditional_interactions_v2_with_control(
            self.authority.point_authority.plan(),
            &self.authority.interactions,
            &weighted.estimation.construct_scores,
            row_mass,
            || !is_cancelled(),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if error.to_string().contains("cancel") {
                "multimod.conditional.refit.cancelled"
            } else {
                "multimod.conditional.refit.weighted_joint_interactions"
            }
            .into(),
            message: error.to_string(),
        })?;
        Ok(ConditionalProcessFullRefitPointV2 {
            edges: point.edges,
            observed_moderator_orientation_signs,
            receipt: ConditionalProcessFullRefitReceiptV2 {
                contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
                measurement_and_scores_refit: true,
                deterministic_score_orientation: true,
                all_joint_structural_equations_refit: true,
                interaction_products_rebuilt: true,
                raw_scientific_gamma_and_delta: true,
                hoc_dependency_stages_refit: false,
                group_isolation_preserved: request.group_id.is_some(),
                positive_case_weights_applied_to_all_stages: case_weighted,
                frequency_counts_applied_without_physical_expansion: frequency_weighted,
                frequency_fit_exactly_equivalent_to_row_expansion: frequency_weighted,
            },
        })
    }
}

fn observed_refit_request_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
) -> Result<ConditionalProcessFullRefitRequestV2, MultiModRunnerErrorV1> {
    let sample = match config.profile {
        ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
            ConditionalProcessRefitSampleV2::FrequencyCounts {
                source_rows: stratum.source_rows.clone(),
                counts: stratum.frequencies.clone().ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "frequency analysis stratum lacks original counts".into(),
                    )
                })?,
            }
        }
        ConditionalProcessProfileV2::CaseWeightedPercentile => {
            let normalized = normalize_positive_case_weights_v2(
                stratum.case_weights.as_deref().ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "case-weight analysis stratum lacks weights".into(),
                    )
                })?,
            )
            .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?;
            ConditionalProcessRefitSampleV2::CaseRows {
                source_rows: stratum.source_rows.clone(),
                normalized_case_weights: Some(normalized.normalized_weights),
            }
        }
        _ => ConditionalProcessRefitSampleV2::CaseRows {
            source_rows: stratum.source_rows.clone(),
            normalized_case_weights: None,
        },
    };
    Ok(ConditionalProcessFullRefitRequestV2 {
        contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
        profile: config.profile,
        group_id: stratum.group_id.clone(),
        observed_fit: true,
        sample,
    })
}

fn full_refit_vector_v2<R: ConditionalProcessFullRefitterV2 + ?Sized>(
    refitter: &mut R,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    templates: &[ConditionalTargetTemplateV2],
    request: &ConditionalProcessFullRefitRequestV2,
    is_cancelled: &(dyn Fn() -> bool + Sync),
) -> Result<(ConditionalProcessFullRefitPointV2, Vec<f64>), MultiModRefitFailureV1> {
    if request.contract != CONDITIONAL_PROCESS_REFIT_CONTRACT_V2
        || request.profile != config.profile
    {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.request_identity".into(),
            message: "full-refit request differs from the selected profile".into(),
        });
    }
    let point = refitter.full_refit(request, is_cancelled)?;
    validate_refit_point_v2(model, config, artifact, &point)?;
    let vector = evaluate_target_templates_v2(config, &point.edges, templates)?;
    Ok((point, vector))
}

fn mapped_case_request_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    draw: &MultiModCaseBootstrapDrawV1,
) -> Result<ConditionalProcessFullRefitRequestV2, MultiModRefitFailureV1> {
    let source_rows = draw
        .source_rows
        .iter()
        .map(|position| {
            stratum
                .source_rows
                .get(*position as usize)
                .copied()
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.draw_identity".into(),
                    message: "case-bootstrap position is outside its frozen stratum".into(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ConditionalProcessFullRefitRequestV2 {
        contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
        profile: config.profile,
        group_id: stratum.group_id.clone(),
        observed_fit: false,
        sample: ConditionalProcessRefitSampleV2::CaseRows {
            source_rows,
            normalized_case_weights: draw.case_weights.clone(),
        },
    })
}

fn mapped_jackknife_request_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    draw: &MultiModDeleteOneJackknifeDrawV1,
) -> Result<ConditionalProcessFullRefitRequestV2, MultiModRefitFailureV1> {
    let source_rows = draw
        .retained_source_rows
        .iter()
        .map(|position| {
            stratum
                .source_rows
                .get(*position as usize)
                .copied()
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.jackknife_identity".into(),
                    message: "jackknife position is outside its frozen stratum".into(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ConditionalProcessFullRefitRequestV2 {
        contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
        profile: config.profile,
        group_id: stratum.group_id.clone(),
        observed_fit: false,
        sample: ConditionalProcessRefitSampleV2::CaseRows {
            source_rows,
            normalized_case_weights: draw.case_weights.clone(),
        },
    })
}

fn mapped_inner_request_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    draw: &MultiModStudentizedInnerDrawV1,
) -> Result<ConditionalProcessFullRefitRequestV2, MultiModRefitFailureV1> {
    let source_rows = draw
        .source_rows
        .iter()
        .map(|position| {
            stratum
                .source_rows
                .get(*position as usize)
                .copied()
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.inner_draw_identity".into(),
                    message: "studentized inner position is outside its frozen stratum".into(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ConditionalProcessFullRefitRequestV2 {
        contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
        profile: config.profile,
        group_id: stratum.group_id.clone(),
        observed_fit: false,
        sample: ConditionalProcessRefitSampleV2::CaseRows {
            source_rows,
            normalized_case_weights: draw.case_weights.clone(),
        },
    })
}

fn mapped_frequency_request_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    draw: &MultiModFrequencyBootstrapDrawV1,
) -> Result<ConditionalProcessFullRefitRequestV2, MultiModRefitFailureV1> {
    if draw.counts.len() != stratum.source_rows.len() {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.frequency_draw_identity".into(),
            message: "frequency draw differs from the compact source frame".into(),
        });
    }
    Ok(ConditionalProcessFullRefitRequestV2 {
        contract: CONDITIONAL_PROCESS_REFIT_CONTRACT_V2.into(),
        profile: config.profile,
        group_id: stratum.group_id.clone(),
        observed_fit: false,
        sample: ConditionalProcessRefitSampleV2::FrequencyCounts {
            source_rows: stratum.source_rows.clone(),
            counts: draw.counts.clone(),
        },
    })
}

fn compact_positive_frequency_cells_v2(
    source_rows: &[u32],
    counts: &[u64],
) -> Result<(Vec<u32>, Vec<u64>, u64), MultiModRefitFailureV1> {
    if counts.len() != source_rows.len() {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.frequency_identity".into(),
            message: "frequency vector differs from compact source rows".into(),
        });
    }
    let original_total = counts.iter().try_fold(0_u64, |total, count| {
        total
            .checked_add(*count)
            .ok_or_else(|| MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.frequency_total".into(),
                message: "frequency bootstrap total overflowed".into(),
            })
    })?;
    let positive = source_rows
        .iter()
        .copied()
        .zip(counts.iter().copied())
        .filter(|(_, count)| *count > 0)
        .collect::<Vec<_>>();
    if positive.len() < 3 {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.frequency_insufficient_rows".into(),
            message: "frequency refit has fewer than three positive-count rows".into(),
        });
    }
    let positive_rows = positive.iter().map(|(row, _)| *row).collect::<Vec<_>>();
    let positive_counts = positive.iter().map(|(_, count)| *count).collect::<Vec<_>>();
    let receipt = validate_positive_frequency_weights_v2(&positive_counts).map_err(|error| {
        MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.frequency_total".into(),
            message: error.to_string(),
        }
    })?;
    if receipt.total_expanded_count != original_total {
        return Err(MultiModRefitFailureV1 {
            code: "multimod.conditional.refit.frequency_total_identity".into(),
            message: "zero-count compaction changed the represented frequency total".into(),
        });
    }
    Ok((positive_rows, positive_counts, original_total))
}

fn scientific_refit_identity_v2(
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    target_ids: &[String],
    stratum: Option<&ConditionalProcessAnalysisStratumV2>,
    phase: &str,
) -> String {
    sha256_serialized(&(
        CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2,
        artifact.receipt().analytical_identity_sha256.as_str(),
        frame.dataset_fingerprint.as_str(),
        frame.analysis_row_mask_sha256.as_str(),
        target_ids,
        stratum.and_then(|value| value.group_id.as_deref()),
        stratum.map(|value| value.source_rows.as_slice()),
        phase,
    ))
}

fn run_case_bootstrap_v2<R, C>(
    refitter: &RefCell<&mut R>,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    templates: &[ConditionalTargetTemplateV2],
    should_cancel: &C,
) -> Result<ConditionalCaseBootstrapLedgerV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
{
    // Stratified groups require independent random streams. Keying the stream
    // by the typed group value (rather than its display label) also preserves
    // the scientific draw bank when labels or contrast directions are changed.
    let master_seed = if let Some(group_id) = stratum.group_id.as_deref() {
        let group = config
            .groups
            .iter()
            .find(|group| group.group_id == group_id)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(format!(
                    "grouped bootstrap stratum {group_id} is not configured"
                ))
            })?;
        let domain = format!(
            "general_sem_conditional_process_v2:group:{}",
            sha256_serialized(&group.value)
        );
        multimod_replicate_seed_v1(config.inference.seed, &domain, 0)
    } else {
        config.inference.seed
    };
    run_case_bootstrap_with_plan_v2(
        refitter,
        model,
        config,
        artifact,
        frame,
        stratum,
        templates,
        config.inference.outer_resamples,
        master_seed,
        "case_bootstrap",
        should_cancel,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_case_bootstrap_with_plan_v2<R, C>(
    refitter: &RefCell<&mut R>,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    templates: &[ConditionalTargetTemplateV2],
    requested_replicates: u32,
    master_seed: u64,
    phase: &str,
    should_cancel: &C,
) -> Result<ConditionalCaseBootstrapLedgerV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
{
    let target_ids = templates
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<Vec<_>>();
    let plan = MultiModBootstrapPlanV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        scientific_refit_identity_sha256: scientific_refit_identity_v2(
            artifact,
            frame,
            &target_ids,
            Some(stratum),
            phase,
        ),
        requested_replicates,
        master_seed,
        minimum_usable_fraction: 0.90,
    };
    let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
        let request = mapped_case_request_v2(config, stratum, draw)?;
        let (_, vector) = full_refit_vector_v2(
            &mut **refitter.borrow_mut(),
            model,
            config,
            artifact,
            templates,
            &request,
            should_cancel,
        )?;
        Ok(vector)
    };
    let cache = run_multimod_case_bootstrap_shard_v1(
        &plan,
        stratum.source_rows.len(),
        stratum.case_weights.as_deref(),
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &mut callback,
        should_cancel,
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    if cache.cancelled || should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    finalize_multimod_case_bootstrap_v1(
        &plan,
        stratum.source_rows.len(),
        stratum.case_weights.as_deref(),
        vec![cache],
    )
    .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))
}

fn run_observed_studentized_inner_v2<R, C>(
    refitter: &RefCell<&mut R>,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    templates: &[ConditionalTargetTemplateV2],
    should_cancel: &C,
) -> Result<ConditionalCaseBootstrapLedgerV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
{
    let seed = multimod_replicate_seed_v1(
        config.inference.seed,
        "general_sem_conditional_process_v2:studentized_observed_inner",
        0,
    );
    run_case_bootstrap_with_plan_v2(
        refitter,
        model,
        config,
        artifact,
        frame,
        stratum,
        templates,
        config.inference.inner_resamples,
        seed,
        "studentized_observed_inner",
        should_cancel,
    )
}

fn run_delete_one_v2<R, C>(
    refitter: &RefCell<&mut R>,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    templates: &[ConditionalTargetTemplateV2],
    should_cancel: &C,
) -> Result<ConditionalDeleteOneLedgerV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
{
    let target_ids = templates
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<Vec<_>>();
    let plan = MultiModJackknifePlanV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        scientific_refit_identity_sha256: scientific_refit_identity_v2(
            artifact,
            frame,
            &target_ids,
            Some(stratum),
            "delete_one_bca",
        ),
    };
    let mut callback = |draw: &MultiModDeleteOneJackknifeDrawV1| {
        let request = mapped_jackknife_request_v2(config, stratum, draw)?;
        let (_, vector) = full_refit_vector_v2(
            &mut **refitter.borrow_mut(),
            model,
            config,
            artifact,
            templates,
            &request,
            should_cancel,
        )?;
        Ok(vector)
    };
    let cache = run_multimod_delete_one_jackknife_shard_v1(
        &plan,
        stratum.source_rows.len(),
        stratum.case_weights.as_deref(),
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &mut callback,
        should_cancel,
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    if cache.cancelled || should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    finalize_multimod_delete_one_jackknife_v1(
        &plan,
        stratum.source_rows.len(),
        stratum.case_weights.as_deref(),
        vec![cache],
    )
    .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))
}

fn run_frequency_bootstrap_v2<R, C>(
    refitter: &RefCell<&mut R>,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    templates: &[ConditionalTargetTemplateV2],
    should_cancel: &C,
) -> Result<ConditionalFrequencyBootstrapLedgerV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
{
    let target_ids = templates
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<Vec<_>>();
    let frequencies = stratum.frequencies.as_deref().ok_or_else(|| {
        MultiModRunnerErrorV1::PreparedInput("frequency frame lacks counts".into())
    })?;
    let plan = MultiModBootstrapPlanV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        scientific_refit_identity_sha256: scientific_refit_identity_v2(
            artifact,
            frame,
            &target_ids,
            Some(stratum),
            "frequency_count_space_bootstrap",
        ),
        requested_replicates: config.inference.outer_resamples,
        master_seed: config.inference.seed,
        minimum_usable_fraction: 0.90,
    };
    let mut callback = |draw: &MultiModFrequencyBootstrapDrawV1| {
        let request = mapped_frequency_request_v2(config, stratum, draw)?;
        let (_, vector) = full_refit_vector_v2(
            &mut **refitter.borrow_mut(),
            model,
            config,
            artifact,
            templates,
            &request,
            should_cancel,
        )?;
        Ok(vector)
    };
    let cache = run_multimod_frequency_bootstrap_shard_v1(
        &plan,
        frequencies,
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &mut callback,
        should_cancel,
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    if cache.cancelled || should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    finalize_multimod_frequency_bootstrap_v1(&plan, frequencies, vec![cache])
        .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))
}

fn run_studentized_v2<R, C>(
    refitter: &RefCell<&mut R>,
    model: &SemModelV4,
    config: &GeneralSemConditionalProcessConfigV2,
    artifact: &CompiledMultiModRecipeV1,
    frame: &ConditionalProcessAnalysisFrameV2,
    stratum: &ConditionalProcessAnalysisStratumV2,
    templates: &[ConditionalTargetTemplateV2],
    should_cancel: &C,
) -> Result<ConditionalStudentizedLedgerV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
{
    let target_ids = templates
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<Vec<_>>();
    let plan = MultiModStudentizedPlanV1 {
        schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
        scientific_refit_identity_sha256: scientific_refit_identity_v2(
            artifact,
            frame,
            &target_ids,
            Some(stratum),
            "nested_studentized",
        ),
        outer_replicates: config.inference.outer_resamples,
        inner_replicates: config.inference.inner_resamples,
        master_seed: config.inference.seed,
        minimum_outer_usable_fraction: 0.90,
        minimum_inner_usable_fraction: 0.90,
    };
    let mut outer_callback = |draw: &MultiModCaseBootstrapDrawV1| {
        let request = mapped_case_request_v2(config, stratum, draw)?;
        let (_, vector) = full_refit_vector_v2(
            &mut **refitter.borrow_mut(),
            model,
            config,
            artifact,
            templates,
            &request,
            should_cancel,
        )?;
        Ok(vector)
    };
    let mut inner_callback = |draw: &MultiModStudentizedInnerDrawV1| {
        let request = mapped_inner_request_v2(config, stratum, draw)?;
        let (_, vector) = full_refit_vector_v2(
            &mut **refitter.borrow_mut(),
            model,
            config,
            artifact,
            templates,
            &request,
            should_cancel,
        )?;
        Ok(vector)
    };
    let cache = run_multimod_studentized_shard_v1(
        &plan,
        stratum.source_rows.len(),
        stratum.case_weights.as_deref(),
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &mut outer_callback,
        &mut inner_callback,
        should_cancel,
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    if cache.cancelled || should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    finalize_multimod_studentized_v1(
        &plan,
        stratum.source_rows.len(),
        stratum.case_weights.as_deref(),
        vec![cache],
    )
    .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))
}

fn success_entry_v2(index: u32, seed: u64) -> PreparedReplicateEntryV1 {
    PreparedReplicateEntryV1 {
        replicate_index: index,
        seed,
        status: PreparedReplicateStatusV1::Usable,
    }
}

fn failed_entry_v2(
    index: u32,
    seed: u64,
    failure: &MultiModRefitFailureV1,
) -> PreparedReplicateEntryV1 {
    let mut entry = prepared_failure_v2(index, failure);
    entry.seed = seed;
    entry
}

fn prepared_targets_from_columns_v2(
    target_ids: &[String],
    columns: Vec<Vec<Option<f64>>>,
) -> Vec<PreparedTargetReplicatesV1> {
    target_ids
        .iter()
        .cloned()
        .zip(columns)
        .map(|(target_id, estimates)| PreparedTargetReplicatesV1 {
            target_id,
            estimates,
            delete_one_jackknife_estimates: Vec::new(),
            observed_standard_error: None,
            outer_standard_errors: Vec::new(),
        })
        .collect()
}

fn prepared_from_case_ledger_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    target_ids: &[String],
    analysis_observations: usize,
    ledger: &ConditionalCaseBootstrapLedgerV2,
) -> Result<PreparedConditionalInferenceV2, MultiModRunnerErrorV1> {
    let mut entries = Vec::with_capacity(ledger.records.len());
    let mut columns = vec![Vec::with_capacity(ledger.records.len()); target_ids.len()];
    for record in &ledger.records {
        let seed = multimod_replicate_seed_v1(
            config.inference.seed,
            "general_sem_conditional_process_v2",
            record.index,
        );
        match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. }
                if value.len() == target_ids.len()
                    && value.iter().all(|estimate| estimate.is_finite()) =>
            {
                entries.push(success_entry_v2(record.index, seed));
                for (column, estimate) in columns.iter_mut().zip(value) {
                    column.push(Some(*estimate));
                }
            }
            MultiModRefitOutcomeV1::Success { .. } => {
                let failure = MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.target_inventory".into(),
                    message: "successful raw ledger row has the wrong target vector".into(),
                };
                entries.push(failed_entry_v2(record.index, seed, &failure));
                for column in &mut columns {
                    column.push(None);
                }
            }
            MultiModRefitOutcomeV1::Failed { failure, .. } => {
                entries.push(failed_entry_v2(record.index, seed, failure));
                for column in &mut columns {
                    column.push(None);
                }
            }
        }
    }
    let usable = entries
        .iter()
        .filter(|entry| matches!(&entry.status, PreparedReplicateStatusV1::Usable))
        .count() as u32;
    let minimum = minimum_usable_v2(config.inference.outer_resamples);
    if usable < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "conditional shared target ledger has {usable} usable draws; {minimum} required"
        )));
    }
    Ok(PreparedConditionalInferenceV2 {
        ledger: PreparedSharedReplicateLedgerV1 {
            master_seed: config.inference.seed,
            domain: "general_sem_conditional_process_v2".into(),
            requested: config.inference.outer_resamples,
            entries,
        },
        targets: prepared_targets_from_columns_v2(target_ids, columns),
        analysis_observations,
        complete_model_refit_per_replicate: true,
        original_sample_probe_anchors_frozen: true,
        hoc_dependency_stages_repeated: config.profile
            == ConditionalProcessProfileV2::MultipleHocPercentile,
        stratified_group_resampling: false,
        weights_travel_with_resampled_rows: config.profile
            == ConditionalProcessProfileV2::CaseWeightedPercentile,
        frequency_count_space_resampling: false,
        nested_inner_refits_complete: false,
    })
}

fn prepared_from_frequency_ledger_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    target_ids: &[String],
    analysis_observations: usize,
    ledger: &ConditionalFrequencyBootstrapLedgerV2,
) -> Result<PreparedConditionalInferenceV2, MultiModRunnerErrorV1> {
    let mut entries = Vec::with_capacity(ledger.records.len());
    let mut columns = vec![Vec::with_capacity(ledger.records.len()); target_ids.len()];
    for record in &ledger.records {
        let seed = multimod_replicate_seed_v1(
            config.inference.seed,
            "general_sem_conditional_process_v2",
            record.index,
        );
        match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. }
                if value.len() == target_ids.len()
                    && value.iter().all(|estimate| estimate.is_finite()) =>
            {
                entries.push(success_entry_v2(record.index, seed));
                for (column, estimate) in columns.iter_mut().zip(value) {
                    column.push(Some(*estimate));
                }
            }
            MultiModRefitOutcomeV1::Success { .. } => {
                let failure = MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.target_inventory".into(),
                    message: "frequency refit returned the wrong target dimension".into(),
                };
                entries.push(failed_entry_v2(record.index, seed, &failure));
                for column in &mut columns {
                    column.push(None);
                }
            }
            MultiModRefitOutcomeV1::Failed { failure, .. } => {
                entries.push(failed_entry_v2(record.index, seed, failure));
                for column in &mut columns {
                    column.push(None);
                }
            }
        }
    }
    let usable = entries
        .iter()
        .filter(|entry| matches!(&entry.status, PreparedReplicateStatusV1::Usable))
        .count() as u32;
    let minimum = minimum_usable_v2(config.inference.outer_resamples);
    if usable < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "frequency shared target ledger has {usable} usable draws; {minimum} required"
        )));
    }
    Ok(PreparedConditionalInferenceV2 {
        ledger: PreparedSharedReplicateLedgerV1 {
            master_seed: config.inference.seed,
            domain: "general_sem_conditional_process_v2".into(),
            requested: config.inference.outer_resamples,
            entries,
        },
        targets: prepared_targets_from_columns_v2(target_ids, columns),
        analysis_observations,
        complete_model_refit_per_replicate: true,
        original_sample_probe_anchors_frozen: true,
        hoc_dependency_stages_repeated: false,
        stratified_group_resampling: false,
        weights_travel_with_resampled_rows: false,
        frequency_count_space_resampling: true,
        nested_inner_refits_complete: false,
    })
}

fn prepared_from_group_ledgers_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    templates: &[ConditionalTargetTemplateV2],
    target_ids: &[String],
    analysis_observations: usize,
    ledgers: &[GroupConditionalCaseLedgerV2],
) -> Result<PreparedConditionalInferenceV2, MultiModRunnerErrorV1> {
    let by_group = ledgers
        .iter()
        .map(|entry| (entry.group_id.as_str(), &entry.ledger))
        .collect::<BTreeMap<_, _>>();
    let mut entries = Vec::with_capacity(config.inference.outer_resamples as usize);
    let mut columns =
        vec![Vec::with_capacity(config.inference.outer_resamples as usize); target_ids.len()];
    for index in 0..config.inference.outer_resamples {
        let seed = multimod_replicate_seed_v1(
            config.inference.seed,
            "general_sem_conditional_process_v2",
            index,
        );
        let mut vectors = BTreeMap::new();
        let mut failures = Vec::new();
        for group in &config.groups {
            let ledger = by_group.get(group.group_id.as_str()).ok_or_else(|| {
                MultiModRunnerErrorV1::InvalidLedger(format!(
                    "group ledger {} is absent",
                    group.group_id
                ))
            })?;
            let record = ledger.records.get(index as usize).ok_or_else(|| {
                MultiModRunnerErrorV1::InvalidLedger(format!(
                    "group ledger {} lacks replicate {index}",
                    group.group_id
                ))
            })?;
            match &record.outcome {
                MultiModRefitOutcomeV1::Success { value, .. }
                    if value.len() == templates.len()
                        && value.iter().all(|estimate| estimate.is_finite()) =>
                {
                    vectors.insert(group.group_id.clone(), value.clone());
                }
                MultiModRefitOutcomeV1::Success { .. } => {
                    failures.push(format!("{}:target_inventory", group.group_id))
                }
                MultiModRefitOutcomeV1::Failed { failure, .. } => {
                    failures.push(format!("{}:{}", group.group_id, failure.code));
                }
            }
        }
        let combined = if failures.is_empty() {
            combine_group_vectors_v2(config, templates, &vectors)
        } else {
            Err(MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.group_stratum_failed".into(),
                message: failures.join(";"),
            })
        };
        match combined {
            Ok(vector) if vector.len() == target_ids.len() => {
                entries.push(success_entry_v2(index, seed));
                for (column, estimate) in columns.iter_mut().zip(vector) {
                    column.push(Some(estimate));
                }
            }
            Ok(_) => {
                let failure = MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.target_inventory".into(),
                    message: "combined group vector has the wrong target dimension".into(),
                };
                entries.push(failed_entry_v2(index, seed, &failure));
                for column in &mut columns {
                    column.push(None);
                }
            }
            Err(failure) => {
                entries.push(failed_entry_v2(index, seed, &failure));
                for column in &mut columns {
                    column.push(None);
                }
            }
        }
    }
    let usable = entries
        .iter()
        .filter(|entry| matches!(&entry.status, PreparedReplicateStatusV1::Usable))
        .count() as u32;
    let minimum = minimum_usable_v2(config.inference.outer_resamples);
    if usable < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "stratified shared group ledger has {usable} usable draws; {minimum} required"
        )));
    }
    Ok(PreparedConditionalInferenceV2 {
        ledger: PreparedSharedReplicateLedgerV1 {
            master_seed: config.inference.seed,
            domain: "general_sem_conditional_process_v2".into(),
            requested: config.inference.outer_resamples,
            entries,
        },
        targets: prepared_targets_from_columns_v2(target_ids, columns),
        analysis_observations,
        complete_model_refit_per_replicate: true,
        original_sample_probe_anchors_frozen: true,
        hoc_dependency_stages_repeated: false,
        stratified_group_resampling: true,
        weights_travel_with_resampled_rows: false,
        frequency_count_space_resampling: false,
        nested_inner_refits_complete: false,
    })
}

fn prepared_from_studentized_ledger_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    target_ids: &[String],
    analysis_observations: usize,
    ledger: &ConditionalStudentizedLedgerV2,
    observed_inner: &ConditionalCaseBootstrapLedgerV2,
) -> Result<PreparedConditionalInferenceV2, MultiModRunnerErrorV1> {
    let requested = config.inference.outer_resamples as usize;
    let minimum_inner = ledger.minimum_inner_required as usize;
    let mut entries = Vec::with_capacity(requested);
    let mut estimates = vec![vec![None; requested]; target_ids.len()];
    let mut outer_standard_errors = vec![vec![None; requested]; target_ids.len()];
    for record in &ledger.records {
        let index = record.outer.index as usize;
        let seed = multimod_replicate_seed_v1(
            config.inference.seed,
            "general_sem_conditional_process_v2",
            record.outer.index,
        );
        let outer = match &record.outer.outcome {
            MultiModRefitOutcomeV1::Success { value, .. }
                if value.len() == target_ids.len()
                    && value.iter().all(|estimate| estimate.is_finite()) =>
            {
                value
            }
            MultiModRefitOutcomeV1::Success { .. } => {
                let failure = MultiModRefitFailureV1 {
                    code: "multimod.conditional.refit.target_inventory".into(),
                    message: "studentized outer vector has the wrong dimension".into(),
                };
                entries.push(failed_entry_v2(record.outer.index, seed, &failure));
                continue;
            }
            MultiModRefitOutcomeV1::Failed { failure, .. } => {
                entries.push(failed_entry_v2(record.outer.index, seed, failure));
                continue;
            }
        };
        let inner = record
            .inner_records
            .iter()
            .filter_map(|inner| match &inner.outcome {
                MultiModRefitOutcomeV1::Success { value, .. }
                    if value.len() == target_ids.len()
                        && value.iter().all(|estimate| estimate.is_finite()) =>
                {
                    Some(value)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if inner.len() < minimum_inner {
            let failure = MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.inner_usable_gate".into(),
                message: format!(
                    "studentized outer {} has {} usable inner refits; {minimum_inner} required",
                    record.outer.index,
                    inner.len()
                ),
            };
            entries.push(failed_entry_v2(record.outer.index, seed, &failure));
            continue;
        }
        let standard_errors = (0..target_ids.len())
            .map(|target| {
                sample_standard_deviation_v2(
                    &inner
                        .iter()
                        .map(|vector| vector[target])
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Option<Vec<_>>>();
        let Some(standard_errors) = standard_errors else {
            let failure = MultiModRefitFailureV1 {
                code: "multimod.conditional.refit.inner_standard_error_unavailable".into(),
                message: format!(
                    "studentized outer {} has a zero or nonfinite nested standard error",
                    record.outer.index
                ),
            };
            entries.push(failed_entry_v2(record.outer.index, seed, &failure));
            continue;
        };
        entries.push(success_entry_v2(record.outer.index, seed));
        for target in 0..target_ids.len() {
            estimates[target][index] = Some(outer[target]);
            outer_standard_errors[target][index] = Some(standard_errors[target]);
        }
    }
    entries.sort_by_key(|entry| entry.replicate_index);
    if entries.len() != requested
        || entries
            .iter()
            .enumerate()
            .any(|(index, entry)| entry.replicate_index as usize != index)
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "studentized prepared ledger is not complete and ordered".into(),
        ));
    }
    let usable_indices = entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            matches!(&entry.status, PreparedReplicateStatusV1::Usable).then_some(index)
        })
        .collect::<Vec<_>>();
    let minimum = minimum_usable_v2(config.inference.outer_resamples) as usize;
    if usable_indices.len() < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "studentized shared target ledger has {} usable outer draws; {minimum} required",
            usable_indices.len()
        )));
    }
    if observed_inner.requested != config.inference.inner_resamples
        || observed_inner.records.len() != observed_inner.requested as usize
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "studentized original-sample inner ledger identity is incomplete".into(),
        ));
    }
    let observed_inner_vectors = observed_inner
        .records
        .iter()
        .filter_map(|record| match &record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. }
                if value.len() == target_ids.len()
                    && value.iter().all(|estimate| estimate.is_finite()) =>
            {
                Some(value)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let minimum_observed_inner = minimum_usable_v2(config.inference.inner_resamples) as usize;
    if observed_inner_vectors.len() < minimum_observed_inner {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "studentized original sample has {} usable inner refits; {minimum_observed_inner} required",
            observed_inner_vectors.len()
        )));
    }
    let observed_standard_errors = (0..target_ids.len())
        .map(|target| {
            sample_standard_deviation_v2(
                &observed_inner_vectors
                    .iter()
                    .map(|vector| vector[target])
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            MultiModRunnerErrorV1::InvalidLedger(
                "studentized original-sample nested standard error is zero or nonfinite; no percentile fallback is permitted"
                    .into(),
            )
        })?;
    let targets = target_ids
        .iter()
        .enumerate()
        .map(|(target, target_id)| PreparedTargetReplicatesV1 {
            target_id: target_id.clone(),
            estimates: estimates[target].clone(),
            delete_one_jackknife_estimates: Vec::new(),
            observed_standard_error: Some(observed_standard_errors[target]),
            outer_standard_errors: outer_standard_errors[target].clone(),
        })
        .collect();
    Ok(PreparedConditionalInferenceV2 {
        ledger: PreparedSharedReplicateLedgerV1 {
            master_seed: config.inference.seed,
            domain: "general_sem_conditional_process_v2".into(),
            requested: config.inference.outer_resamples,
            entries,
        },
        targets,
        analysis_observations,
        complete_model_refit_per_replicate: true,
        original_sample_probe_anchors_frozen: true,
        hoc_dependency_stages_repeated: false,
        stratified_group_resampling: false,
        weights_travel_with_resampled_rows: false,
        frequency_count_space_resampling: false,
        nested_inner_refits_complete: true,
    })
}

fn attach_delete_one_v2(
    prepared: &mut PreparedConditionalInferenceV2,
    delete_one: &ConditionalDeleteOneLedgerV2,
) -> Result<(), MultiModRunnerErrorV1> {
    if delete_one.usable != delete_one.requested
        || delete_one.records.len() != delete_one.requested as usize
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "BCa requires a complete successful delete-one jackknife".into(),
        ));
    }
    for (target_index, target) in prepared.targets.iter_mut().enumerate() {
        target.delete_one_jackknife_estimates = delete_one
            .records
            .iter()
            .map(|record| match &record.outcome {
                MultiModRefitOutcomeV1::Success { value, .. } => value
                    .get(target_index)
                    .copied()
                    .filter(|value| value.is_finite())
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::InvalidLedger(
                            "BCa jackknife target vector is invalid".into(),
                        )
                    }),
                MultiModRefitOutcomeV1::Failed { failure, .. } => {
                    Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                        "BCa jackknife failed: {}",
                        failure.message
                    )))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
    }
    Ok(())
}

fn apply_prepared_inference_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    prepared: &PreparedConditionalInferenceV2,
    targets: &mut [ConditionalProcessTargetResultV2],
) -> Result<MultimodReplicateLedgerSummaryV1, MultiModRunnerErrorV1> {
    if prepared.ledger.requested != config.inference.outer_resamples
        || prepared.ledger.master_seed != config.inference.seed
        || prepared.ledger.domain != "general_sem_conditional_process_v2"
        || prepared.ledger.entries.len() != prepared.ledger.requested as usize
        || prepared.targets.len() != targets.len()
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "conditional prepared ledger identity or target dimension is invalid".into(),
        ));
    }
    for (index, entry) in prepared.ledger.entries.iter().enumerate() {
        if entry.replicate_index as usize != index
            || entry.seed
                != multimod_replicate_seed_v1(
                    prepared.ledger.master_seed,
                    &prepared.ledger.domain,
                    entry.replicate_index,
                )
            || matches!(
                &entry.status,
                PreparedReplicateStatusV1::Failed {
                    stable_code,
                    detail,
                    ..
                } if stable_code.trim().is_empty() || detail.trim().is_empty()
            )
        {
            return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                "conditional replicate entry {index} has an invalid index, seed, or failure identity"
            )));
        }
    }
    let usable_indices = prepared
        .ledger
        .entries
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            matches!(&entry.status, PreparedReplicateStatusV1::Usable).then_some(index)
        })
        .collect::<Vec<_>>();
    let minimum = minimum_usable_v2(config.inference.outer_resamples) as usize;
    if usable_indices.len() < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "conditional inference has {} usable draws; {minimum} required",
            usable_indices.len()
        )));
    }
    for (target_index, target) in targets.iter_mut().enumerate() {
        let prepared_target = &prepared.targets[target_index];
        if prepared_target.target_id != target.target_id
            || prepared_target.estimates.len() != prepared.ledger.requested as usize
            || prepared_target
                .estimates
                .iter()
                .enumerate()
                .any(|(index, estimate)| {
                    let usable = usable_indices.binary_search(&index).is_ok();
                    usable != estimate.is_some()
                        || estimate.is_some_and(|estimate| !estimate.is_finite())
                })
        {
            return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                "target {} does not share the exact ordered validity bitmap",
                target.target_id
            )));
        }
        let draws = usable_indices
            .iter()
            .map(|index| prepared_target.estimates[*index].expect("bitmap checked above"))
            .collect::<Vec<_>>();
        let alternative = conditional_alternative_v2(config.inference.alternative);
        let resolved = match config.inference.interval {
            ConditionalProcessIntervalV2::Percentile => percentile_interval_v2(
                &draws,
                config.inference.outer_resamples as usize,
                config.inference.confidence_level,
                alternative,
            ),
            ConditionalProcessIntervalV2::Bca => {
                if prepared_target.delete_one_jackknife_estimates.len()
                    != prepared.analysis_observations
                {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                        "BCa target {} lacks the full delete-one jackknife",
                        target.target_id
                    )));
                }
                bca_interval_v2(
                    target.estimate,
                    &draws,
                    &prepared_target.delete_one_jackknife_estimates,
                    config.inference.outer_resamples as usize,
                    config.inference.confidence_level,
                    alternative,
                )
            }
            ConditionalProcessIntervalV2::Studentized => {
                let observed_standard_error =
                    prepared_target.observed_standard_error.ok_or_else(|| {
                        MultiModRunnerErrorV1::InvalidLedger(format!(
                            "studentized target {} lacks observed bootstrap SE",
                            target.target_id
                        ))
                    })?;
                if prepared_target.outer_standard_errors.len() != prepared.ledger.requested as usize
                {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                        "studentized target {} has the wrong outer-SE vector",
                        target.target_id
                    )));
                }
                let outer = usable_indices
                    .iter()
                    .map(|index| {
                        Ok(StudentizedOuterReplicateV2 {
                            estimate: prepared_target.estimates[*index]
                                .expect("bitmap checked above"),
                            standard_error: prepared_target.outer_standard_errors[*index]
                                .ok_or_else(|| {
                                    MultiModRunnerErrorV1::InvalidLedger(format!(
                                        "studentized target {} lacks a usable outer SE",
                                        target.target_id
                                    ))
                                })?,
                        })
                    })
                    .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?;
                studentized_interval_v2(
                    target.estimate,
                    observed_standard_error,
                    &outer,
                    config.inference.outer_resamples as usize,
                    config.inference.confidence_level,
                    alternative,
                )
            }
        }
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        let family = match config.inference.interval {
            ConditionalProcessIntervalV2::Percentile => "type_7_percentile",
            ConditionalProcessIntervalV2::Bca => "full_delete_one_bca",
            ConditionalProcessIntervalV2::Studentized => "complete_nested_studentized",
        };
        target.interval = Some(interval_v2(
            family,
            config.inference.confidence_level,
            config.inference.alternative,
            resolved.lower,
            resolved.upper,
        )?);
        target.p_value = Some(empirical_zero_probability_v2(
            &draws,
            config.inference.alternative,
        ));
        target.usable_replicates = usable_indices.len() as u32;
    }
    Ok(ledger_summary_v2(
        &prepared.ledger,
        minimum_usable_v2(config.inference.outer_resamples),
    ))
}

fn predicted_conditional_evidence_bytes_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    frame: &ConditionalProcessAnalysisFrameV2,
    target_count: usize,
) -> u64 {
    let rows = frame
        .strata
        .iter()
        .map(|stratum| stratum.source_rows.len() as u64)
        .sum::<u64>();
    let groups = frame.strata.len() as u64;
    let outer = u64::from(config.inference.outer_resamples);
    let draw_bytes_per_compact_row = match config.profile {
        // A case-weight draw persists both its u32 source-row position and its
        // f64 row-travelling normalized weight.
        ConditionalProcessProfileV2::CaseWeightedPercentile => 12_u64,
        // A frequency draw persists one exact u64 count per compact source row.
        ConditionalProcessProfileV2::FrequencyWeightedPercentile => 8_u64,
        _ => 4_u64,
    };
    let per_outer = rows
        .saturating_mul(draw_bytes_per_compact_row)
        .saturating_add((target_count as u64).saturating_mul(8))
        .saturating_add(768_u64.saturating_mul(groups));
    let nested = if config.inference.interval == ConditionalProcessIntervalV2::Studentized {
        outer
            .saturating_add(1)
            .saturating_mul(u64::from(config.inference.inner_resamples))
            .saturating_mul(per_outer)
    } else {
        0
    };
    let jackknife = if config.inference.interval == ConditionalProcessIntervalV2::Bca {
        rows.saturating_mul(per_outer)
    } else {
        0
    };
    outer
        .saturating_mul(per_outer)
        .saturating_add(nested)
        .saturating_add(jackknife)
        .saturating_add(rows.saturating_mul(128))
}

fn conditional_analysis_observations_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    frame: &ConditionalProcessAnalysisFrameV2,
) -> Result<usize, MultiModRunnerErrorV1> {
    if config.profile == ConditionalProcessProfileV2::FrequencyWeightedPercentile {
        let represented = frame
            .strata
            .iter()
            .flat_map(|stratum| stratum.frequencies.iter().flatten())
            .try_fold(0_u64, |total, count| {
                total.checked_add(*count).ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "frequency represented observation count overflowed".into(),
                    )
                })
            })?;
        return usize::try_from(represented).map_err(|_| {
            MultiModRunnerErrorV1::PreparedInput(
                "frequency represented observation count exceeds the platform range".into(),
            )
        });
    }
    frame.strata.iter().try_fold(0_usize, |total, stratum| {
        total.checked_add(stratum.source_rows.len()).ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(
                "conditional compact observation count overflowed".into(),
            )
        })
    })
}

fn execution_warnings_v2(
    config: &GeneralSemConditionalProcessConfigV2,
    frame: &ConditionalProcessAnalysisFrameV2,
    predicted_bytes: u64,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if predicted_bytes > MULTIMOD_SIDECAR_WARN_BYTES_V1 {
        warnings.push(format!(
            "predicted conditional-process evidence sidecar is {predicted_bytes} bytes, above the 128 MiB warning threshold"
        ));
    }
    if config.profile == ConditionalProcessProfileV2::GroupedPercentile {
        if frame
            .strata
            .iter()
            .any(|stratum| stratum.source_rows.len() < 30)
        {
            warnings
                .push("one or more selected groups contain fewer than 30 complete cases".into());
        }
        let minimum = frame
            .strata
            .iter()
            .map(|stratum| stratum.source_rows.len())
            .min()
            .unwrap_or(1);
        let maximum = frame
            .strata
            .iter()
            .map(|stratum| stratum.source_rows.len())
            .max()
            .unwrap_or(1);
        if maximum > minimum.saturating_mul(2) {
            warnings.push("selected-group imbalance exceeds 2:1".into());
        }
    }
    if config.profile == ConditionalProcessProfileV2::CaseWeightedPercentile {
        if let Some(weights) = frame.strata[0].case_weights.as_deref() {
            if let Ok(receipt) = normalize_positive_case_weights_v2(weights) {
                warnings.push(format!(
                    "case-weight Kish effective sample size is {:.6}",
                    receipt.kish_effective_sample_size
                ));
                if receipt.effective_sample_size_below_twenty_five_percent {
                    warnings.push("case-weight Kish ESS is below 25% of raw n".into());
                }
            }
        }
    }
    warnings
}

/// Executes conditional-process V2 through a faithful reusable point/refit
/// authority. This is the integration entry point for positive case-weighted,
/// exact compact frequency-weighted, and disjoint multi-HOC adapters.
pub fn run_compiled_general_sem_conditional_process_raw_with_refitter_v2<R, C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    refitter: &mut R,
    should_cancel: C,
    progress: P,
) -> Result<RawConditionalProcessRunV2, MultiModRunnerErrorV1>
where
    R: ConditionalProcessFullRefitterV2 + ?Sized,
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::ValidatingAuthority,
        0,
        1,
        "conditional_raw:authority",
    );
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2,
    )?;
    let config = recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "conditional-process configuration disappeared after compilation".into(),
            )
        })?;
    if recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.listwise_complete_rows_required".into(),
        ));
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::PreparingPointInputs,
        0,
        1,
        "conditional_raw:analysis_frame",
    );
    let frame = prepare_conditional_process_analysis_frame_v2(dataset, model, config)?;
    validate_analysis_frame_v2(dataset, config, &frame)?;
    let mut templates_by_stratum =
        BTreeMap::<Option<String>, Vec<ConditionalTargetTemplateV2>>::new();
    let mut warning_set = BTreeSet::new();
    let mut template_authority = None::<
        Vec<(
            String,
            ConditionalProcessTargetKindV2,
            String,
            BTreeMap<String, f64>,
            Vec<String>,
        )>,
    >;
    for stratum in &frame.strata {
        let probes = frozen_probe_points_v2(dataset, &frame, config, stratum)?;
        let (templates, stratum_warnings) = target_templates_v2(config, artifact, model, &probes)?;
        let authority = templates
            .iter()
            .map(|template| {
                (
                    template.target_id.clone(),
                    template.kind.clone(),
                    template.path_id.clone(),
                    template.probe_values.clone(),
                    template.derivative_variables.clone(),
                )
            })
            .collect::<Vec<_>>();
        if template_authority
            .as_ref()
            .is_some_and(|expected| expected != &authority)
        {
            return Err(MultiModRunnerErrorV1::Authority(
                "group-specific probe metrics changed the authored target identity or inventory"
                    .into(),
            ));
        }
        template_authority.get_or_insert(authority);
        warning_set.extend(stratum_warnings);
        if templates_by_stratum
            .insert(stratum.group_id.clone(), templates)
            .is_some()
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "analysis frame contains a duplicate stratum identity".into(),
            ));
        }
    }
    let templates = templates_by_stratum
        .values()
        .next()
        .cloned()
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority("target template inventory is empty".into())
        })?;
    let mut warnings = warning_set.into_iter().collect::<Vec<_>>();
    let predicted_bytes = predicted_conditional_evidence_bytes_v2(
        config,
        &frame,
        templates
            .len()
            .saturating_mul(frame.strata.len())
            .saturating_add(config.group_contrasts.len().saturating_mul(templates.len())),
    );
    if predicted_bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1 {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.archive.sidecar_limit: predicted conditional evidence is {predicted_bytes} bytes"
        )));
    }
    warnings.extend(execution_warnings_v2(config, &frame, predicted_bytes));
    report(
        &progress,
        MultiModRunnerPhaseV1::PointEstimation,
        0,
        frame.strata.len() as u64,
        "conditional_raw:point",
    );
    let refitter = RefCell::new(refitter);
    let mut point_vectors = BTreeMap::<Option<String>, Vec<f64>>::new();
    let mut point_fits = Vec::with_capacity(frame.strata.len());
    for (index, stratum) in frame.strata.iter().enumerate() {
        let request = observed_refit_request_v2(config, stratum)?;
        let stratum_templates = templates_by_stratum.get(&stratum.group_id).ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "observed fit lacks its group-specific frozen probe template".into(),
            )
        })?;
        let (point, vector) = full_refit_vector_v2(
            &mut **refitter.borrow_mut(),
            model,
            config,
            artifact,
            stratum_templates,
            &request,
            &should_cancel,
        )
        .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.message))?;
        verify_raw_probe_orientations_v2(config, stratum, &point)?;
        point_fits.push(RawConditionalProcessPointFitV2 {
            group_id: stratum.group_id.clone(),
            point,
        });
        if point_vectors
            .insert(stratum.group_id.clone(), vector)
            .is_some()
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "analysis frame contains a duplicate stratum identity".into(),
            ));
        }
        report(
            &progress,
            MultiModRunnerPhaseV1::PointEstimation,
            (index + 1) as u64,
            frame.strata.len() as u64,
            stratum
                .group_id
                .as_ref()
                .map(|group| format!("conditional_raw:point:{group}"))
                .unwrap_or_else(|| "conditional_raw:point:single".into()),
        );
    }
    let mut targets = point_targets_v2(config, &templates, &point_vectors)?;
    let target_ids = targets
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<Vec<_>>();
    let analysis_observations = conditional_analysis_observations_v2(config, &frame)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::Resampling,
        0,
        u64::from(config.inference.outer_resamples),
        "conditional_raw:shared_full_refit_ledger",
    );
    let (prepared, raw_evidence) = match config.profile {
        ConditionalProcessProfileV2::GroupedPercentile => {
            let mut group_ledgers = Vec::with_capacity(frame.strata.len());
            for stratum in &frame.strata {
                let group_id = stratum.group_id.clone().ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "grouped analysis stratum lacks a group id".into(),
                    )
                })?;
                let ledger = run_case_bootstrap_v2(
                    &refitter,
                    model,
                    config,
                    artifact,
                    &frame,
                    stratum,
                    templates_by_stratum.get(&stratum.group_id).ok_or_else(|| {
                        MultiModRunnerErrorV1::Authority(
                            "group bootstrap lacks its frozen probe template".into(),
                        )
                    })?,
                    &should_cancel,
                )?;
                group_ledgers.push(GroupConditionalCaseLedgerV2 { group_id, ledger });
            }
            let prepared = prepared_from_group_ledgers_v2(
                config,
                &templates,
                &target_ids,
                analysis_observations,
                &group_ledgers,
            )?;
            (
                prepared,
                RawConditionalProcessEvidenceV2::GroupedStratified {
                    groups: group_ledgers,
                },
            )
        }
        ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
            let ledger = run_frequency_bootstrap_v2(
                &refitter,
                model,
                config,
                artifact,
                &frame,
                &frame.strata[0],
                &templates,
                &should_cancel,
            )?;
            let prepared = prepared_from_frequency_ledger_v2(
                config,
                &target_ids,
                analysis_observations,
                &ledger,
            )?;
            (
                prepared,
                RawConditionalProcessEvidenceV2::FrequencyCountSpace { bootstrap: ledger },
            )
        }
        ConditionalProcessProfileV2::MultiTwoWayStudentized => {
            let ledger = run_studentized_v2(
                &refitter,
                model,
                config,
                artifact,
                &frame,
                &frame.strata[0],
                &templates,
                &should_cancel,
            )?;
            let observed_inner = run_observed_studentized_inner_v2(
                &refitter,
                model,
                config,
                artifact,
                &frame,
                &frame.strata[0],
                &templates,
                &should_cancel,
            )?;
            let prepared = prepared_from_studentized_ledger_v2(
                config,
                &target_ids,
                analysis_observations,
                &ledger,
                &observed_inner,
            )?;
            (
                prepared,
                RawConditionalProcessEvidenceV2::StudentizedCase {
                    nested: ledger,
                    observed_inner,
                },
            )
        }
        ConditionalProcessProfileV2::MultiTwoWayBca => {
            let bootstrap = run_case_bootstrap_v2(
                &refitter,
                model,
                config,
                artifact,
                &frame,
                &frame.strata[0],
                &templates,
                &should_cancel,
            )?;
            let delete_one = run_delete_one_v2(
                &refitter,
                model,
                config,
                artifact,
                &frame,
                &frame.strata[0],
                &templates,
                &should_cancel,
            )?;
            let mut prepared = prepared_from_case_ledger_v2(
                config,
                &target_ids,
                analysis_observations,
                &bootstrap,
            )?;
            attach_delete_one_v2(&mut prepared, &delete_one)?;
            (
                prepared,
                RawConditionalProcessEvidenceV2::BcaCase {
                    bootstrap,
                    delete_one,
                },
            )
        }
        _ => {
            let bootstrap = run_case_bootstrap_v2(
                &refitter,
                model,
                config,
                artifact,
                &frame,
                &frame.strata[0],
                &templates,
                &should_cancel,
            )?;
            let prepared = prepared_from_case_ledger_v2(
                config,
                &target_ids,
                analysis_observations,
                &bootstrap,
            )?;
            (
                prepared,
                RawConditionalProcessEvidenceV2::PercentileCase { bootstrap },
            )
        }
    };
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    let replicate_ledger = apply_prepared_inference_v2(config, &prepared, &mut targets)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::AssemblingResult,
        1,
        1,
        "conditional_raw:result",
    );
    let analysis = GeneralSemConditionalProcessResultV2 {
        schema_version: GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION,
        provenance: provenance(artifact.receipt(), config.inference.seed),
        profile_id: profile_id_v2(config.profile).into(),
        targets,
        replicate_ledger,
        sidecars: Vec::new(),
        warnings,
    };
    let result = MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(analysis);
    result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    report(
        &progress,
        MultiModRunnerPhaseV1::Completed,
        1,
        1,
        "conditional_raw:complete",
    );
    Ok(RawConditionalProcessRunV2 {
        output: MultiModRunOutputV1 {
            compilation_receipt: artifact.receipt().clone(),
            result,
            evidence: vec![
                MultiModRunnerEvidenceV1::ConditionalInference(prepared.clone()),
                MultiModRunnerEvidenceV1::ConditionalRawPreparation(frame.clone()),
                MultiModRunnerEvidenceV1::ConditionalRawFullRefit(raw_evidence.clone()),
            ],
        },
        preparation: frame,
        point_fits,
        raw_evidence,
    })
}

/// Runtime backend required by each frozen profile. Both variants execute the
/// same target compiler, shared ledgers, inference, and result assembler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalProcessRawBackendRequirementV2 {
    BuiltInRecipeV4Pls,
    ExactProductionFullRefitter,
}

pub const fn conditional_process_raw_backend_requirement_v2(
    _profile: ConditionalProcessProfileV2,
) -> ConditionalProcessRawBackendRequirementV2 {
    ConditionalProcessRawBackendRequirementV2::BuiltInRecipeV4Pls
}

/// One explicit dispatcher for runner/native integration. Production
/// All frozen profiles have built-in production authorities. `ExactProduction`
/// remains an explicit validation/oracle seam and is never selected silently.
pub enum ConditionalProcessRawAuthorityV2<'a> {
    BuiltIn,
    ExactProduction(&'a mut dyn ConditionalProcessFullRefitterV2),
}

pub fn run_compiled_general_sem_conditional_process_raw_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    authority: ConditionalProcessRawAuthorityV2<'_>,
    should_cancel: C,
    progress: P,
) -> Result<RawConditionalProcessRunV2, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    match authority {
        ConditionalProcessRawAuthorityV2::BuiltIn => {
            run_compiled_general_sem_conditional_process_raw_builtin_v2(
                dataset,
                recipe,
                model,
                artifact,
                should_cancel,
                progress,
            )
        }
        ConditionalProcessRawAuthorityV2::ExactProduction(refitter) => {
            run_compiled_general_sem_conditional_process_raw_with_refitter_v2(
                dataset,
                recipe,
                model,
                artifact,
                refitter,
                should_cancel,
                progress,
            )
        }
    }
}

/// Native/runner convenience boundary when the caller persists raw ledgers
/// through its sidecar sink and needs the canonical MultiMod output value.
pub fn run_compiled_general_sem_conditional_process_raw_output_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    authority: ConditionalProcessRawAuthorityV2<'_>,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_general_sem_conditional_process_raw_v2(
        dataset,
        recipe,
        model,
        artifact,
        authority,
        should_cancel,
        progress,
    )
    .map(|run| run.output)
}

/// Convenience wrapper used by the dispatcher for profiles backed entirely by
/// the current Recipe V4 PLS and joint interaction point authorities.
pub fn run_compiled_general_sem_conditional_process_raw_builtin_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    should_cancel: C,
    progress: P,
) -> Result<RawConditionalProcessRunV2, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    let config = recipe
        .general_sem_conditional_process
        .as_ref()
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("conditional config absent".into()))?;
    match config.profile {
        ConditionalProcessProfileV2::CaseWeightedPercentile
        | ConditionalProcessProfileV2::FrequencyWeightedPercentile => {
            let authority = built_in_weighted_conditional_authority_v2(recipe, model, artifact)?;
            let mut refitter = BuiltInWeightedConditionalRefitterV2 { dataset, authority };
            run_compiled_general_sem_conditional_process_raw_with_refitter_v2(
                dataset,
                recipe,
                model,
                artifact,
                &mut refitter,
                should_cancel,
                progress,
            )
        }
        ConditionalProcessProfileV2::MultipleHocPercentile => {
            let authority = built_in_hoc_conditional_authority_v2(recipe, model, artifact)?;
            let mut refitter = BuiltInHocConditionalRefitterV2 {
                dataset,
                authority,
                observed_source_rows: Vec::new(),
                observed_positions: BTreeMap::new(),
                observed_reference: None,
            };
            run_compiled_general_sem_conditional_process_raw_with_refitter_v2(
                dataset,
                recipe,
                model,
                artifact,
                &mut refitter,
                should_cancel,
                progress,
            )
        }
        ConditionalProcessProfileV2::MultiTwoWayPercentile
        | ConditionalProcessProfileV2::MultiTwoWayBca
        | ConditionalProcessProfileV2::MultiTwoWayStudentized
        | ConditionalProcessProfileV2::BoundedThreeWayPercentile
        | ConditionalProcessProfileV2::GroupedPercentile => {
            let authority = built_in_conditional_authority_v2(recipe, model)?;
            let mut refitter = BuiltInConditionalRefitterV2 { dataset, authority };
            run_compiled_general_sem_conditional_process_raw_with_refitter_v2(
                dataset,
                recipe,
                model,
                artifact,
                &mut refitter,
                should_cancel,
                progress,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_core::{
        ConditionalGroupContrastV2, ConditionalProcessEstimandsV2, ConditionalProcessInferenceV2,
        Construct, HigherOrderMeasurementTypeV4, LegacyBasicModelInterpretationV4, MeasurementMode,
        ModelSpec, SelectedGroupV1, SemDerivedTermV4, convert_legacy_basic_model_v4,
    };
    use qpls_estimation::{ConditionalPolynomialTermV2, ModeratorPowerV2};

    fn probe(values: &[(&str, f64)]) -> ConditionalProbePointV2 {
        ConditionalProbePointV2 {
            probe_id: "probe".into(),
            standardized_values: values
                .iter()
                .map(|(id, value)| ((*id).to_owned(), *value))
                .collect(),
        }
    }

    fn template(kind: ConditionalProcessTargetKindV2, id: &str) -> ConditionalTargetTemplateV2 {
        let point = probe(&[("z", 0.0)]);
        ConditionalTargetTemplateV2 {
            target_id: id.into(),
            kind,
            path_id: "path".into(),
            probe_values: point.standardized_values.clone(),
            derivative_variables: Vec::new(),
            operation: ConditionalTargetOperationV2::Specific {
                path_id: "path".into(),
                probe: point,
            },
        }
    }

    fn grouped_config() -> GeneralSemConditionalProcessConfigV2 {
        GeneralSemConditionalProcessConfigV2 {
            schema_version: 2,
            profile: ConditionalProcessProfileV2::GroupedPercentile,
            paths: Vec::new(),
            declared_interaction_ids: Vec::new(),
            three_way_interaction_id: None,
            hoc_ids: Vec::new(),
            moderator_ids: Vec::new(),
            probes: Vec::new(),
            explicit_joint_tuples: Vec::new(),
            probe_contrasts: Vec::new(),
            grouping_column: Some("group".into()),
            groups: vec![
                SelectedGroupV1 {
                    group_id: "b".into(),
                    label: "B".into(),
                    value: TypedGroupValueV1::Text { value: "b".into() },
                },
                SelectedGroupV1 {
                    group_id: "a".into(),
                    label: "A".into(),
                    value: TypedGroupValueV1::Text { value: "a".into() },
                },
            ],
            group_contrasts: vec![ConditionalGroupContrastV2 {
                contrast_id: "a-minus-b".into(),
                left_group_id: "a".into(),
                right_group_id: "b".into(),
            }],
            weight: None,
            estimands: ConditionalProcessEstimandsV2 {
                conditional_specific_indirect: true,
                conditional_total_indirect: false,
                conditional_total_effect: false,
                scalar_index_when_affine: false,
                local_first_derivatives: false,
                local_second_and_cross_derivatives: false,
                finite_probe_contrasts: false,
            },
            inference: ConditionalProcessInferenceV2 {
                interval: ConditionalProcessIntervalV2::Percentile,
                alternative: InferenceAlternativeV1::TwoSided,
                outer_resamples: 500,
                inner_resamples: 0,
                seed: 42,
                confidence_level: 0.95,
            },
        }
    }

    fn four_disjoint_hoc_projection_fixture() -> SemModelV4 {
        let construct_ids = ["a1", "a2", "b1", "b2", "c1", "c2", "d1", "d2"];
        let constructs = construct_ids
            .iter()
            .map(|id| Construct {
                id: (*id).into(),
                name: (*id).to_uppercase(),
                short_name: (*id).to_uppercase(),
                mode: MeasurementMode::Reflective,
                indicators: vec![format!("{id}_1"), format!("{id}_2")],
            })
            .collect();
        let mut model = convert_legacy_basic_model_v4(
            &ModelSpec {
                id: uuid::Uuid::from_u128(0x434f_4e44_484f_435f_4241_5345_0004),
                name: "Conditional four disjoint HOCs".into(),
                constructs,
                paths: Vec::new(),
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        for (index, components) in [["a1", "a2"], ["b1", "b2"], ["c1", "c2"], ["d1", "d2"]]
            .into_iter()
            .enumerate()
        {
            let output = format!("derived:hoc:{index}");
            model.variables.push(SemVariableV4::Derived {
                id: output.clone(),
                label: format!("HOC {index}"),
            });
            model.derived_terms.push(SemDerivedTermV4::HigherOrder {
                id: format!("term:hoc:{index}"),
                output,
                components: components
                    .into_iter()
                    .map(|id| format!("construct:{id}"))
                    .collect(),
                approach: HigherOrderConstructionApproachV4::DisjointTwoStage,
                measurement_type: HigherOrderMeasurementTypeV4::ReflectiveReflective,
            });
        }
        model.ensure_valid().unwrap();
        model
    }

    #[test]
    fn conditional_hoc_base_projection_keeps_the_four_hoc_envelope() {
        let scientific_model = four_disjoint_hoc_projection_fixture();
        assert_eq!(
            scientific_model
                .derived_terms
                .iter()
                .filter(|term| matches!(term, SemDerivedTermV4::HigherOrder { .. }))
                .count(),
            4
        );

        let base_model = project_hoc_conditional_base_model_v2(&scientific_model).unwrap();

        assert!(base_model.derived_terms.is_empty());
        assert!(
            base_model
                .variables
                .iter()
                .all(|variable| !variable.id().starts_with("derived:hoc:"))
        );
        assert_eq!(
            base_model
                .variables
                .iter()
                .filter(|variable| variable.id().starts_with("construct:"))
                .count(),
            8
        );
    }

    #[test]
    fn row_mask_binds_membership_not_runtime_weight_representation() {
        let first = vec![ConditionalProcessAnalysisStratumV2 {
            group_id: None,
            source_rows: vec![1, 3, 8],
            case_weights: Some(vec![1.0, 2.0, 3.0]),
            frequencies: None,
        }];
        let mut changed_weights = first.clone();
        changed_weights[0].case_weights = Some(vec![4.0, 5.0, 6.0]);
        let mut changed_rows = first.clone();
        changed_rows[0].source_rows[2] = 9;
        assert_eq!(
            conditional_process_analysis_row_mask_sha256_v2("dataset", &first),
            conditional_process_analysis_row_mask_sha256_v2("dataset", &changed_weights)
        );
        assert_ne!(
            conditional_process_analysis_row_mask_sha256_v2("dataset", &first),
            conditional_process_analysis_row_mask_sha256_v2("dataset", &changed_rows)
        );
    }

    #[test]
    fn grouped_raw_probe_anchors_are_group_specific_but_target_identity_is_stable() {
        let dataset = qpls_data::import_delimited_bytes(
            b"z,group\n0,a\n2,a\n10,b\n14,b\n",
            "grouped-raw-probe.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let mut config = grouped_config();
        config.moderator_ids = vec!["z".into()];
        config.probes = vec![qpls_core::ConditionalModeratorProbeV2 {
            probe_id: "z-probe".into(),
            moderator_id: "z".into(),
            scale: ConditionalProbeScaleV2::RawObservedWithTransformationReceipt,
            values: vec![2.0],
            raw_transformation_receipt: None,
            raw_fit_metric_receipts: Vec::new(),
        }];
        let mut frame = ConditionalProcessAnalysisFrameV2 {
            method_version: CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2.into(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            required_source_columns: vec!["group".into(), "z".into()],
            strata: vec![
                ConditionalProcessAnalysisStratumV2 {
                    group_id: Some("b".into()),
                    source_rows: vec![2, 3],
                    case_weights: None,
                    frequencies: None,
                },
                ConditionalProcessAnalysisStratumV2 {
                    group_id: Some("a".into()),
                    source_rows: vec![0, 1],
                    case_weights: None,
                    frequencies: None,
                },
            ],
            analysis_row_mask_sha256: String::new(),
            excluded_rows: Vec::new(),
        };
        frame.analysis_row_mask_sha256 = conditional_process_analysis_row_mask_sha256_v2(
            &frame.dataset_fingerprint,
            &frame.strata,
        );
        let b_receipt = expected_raw_probe_fit_metric_receipt_v2(
            &dataset,
            &frame,
            &config,
            &frame.strata[0],
            "z",
            "z",
            1,
        )
        .unwrap();
        let a_receipt = expected_raw_probe_fit_metric_receipt_v2(
            &dataset,
            &frame,
            &config,
            &frame.strata[1],
            "z",
            "z",
            1,
        )
        .unwrap();
        assert_eq!(a_receipt.center, 1.0);
        assert_eq!(b_receipt.center, 12.0);
        config.probes[0].raw_fit_metric_receipts = vec![a_receipt, b_receipt];

        let a_standardized =
            standardized_probe_value_v2(&dataset, &frame, &config, &frame.strata[1], "z", 2.0)
                .unwrap();
        let b_standardized =
            standardized_probe_value_v2(&dataset, &frame, &config, &frame.strata[0], "z", 2.0)
                .unwrap();
        assert_ne!(a_standardized.to_bits(), b_standardized.to_bits());

        let left = ConditionalTargetOperationV2::Specific {
            path_id: "path".into(),
            probe: ConditionalProbePointV2 {
                probe_id: "authored-probe".into(),
                standardized_values: BTreeMap::from([("z".into(), a_standardized)]),
            },
        };
        let right = ConditionalTargetOperationV2::Specific {
            path_id: "path".into(),
            probe: ConditionalProbePointV2 {
                probe_id: "authored-probe".into(),
                standardized_values: BTreeMap::from([("z".into(), b_standardized)]),
            },
        };
        assert_eq!(
            target_id_v2(
                ConditionalProcessTargetKindV2::ConditionalSpecificIndirect,
                "path",
                &left,
            ),
            target_id_v2(
                ConditionalProcessTargetKindV2::ConditionalSpecificIndirect,
                "path",
                &right,
            )
        );
    }

    #[test]
    fn weighted_raw_probe_metrics_use_case_effective_df_and_frequency_expansion_df() {
        let dataset = qpls_data::import_delimited_bytes(
            b"z,w,f\n0,1,1\n10,2,2\n20,3,3\n",
            "weighted-raw-probe.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let base_stratum = ConditionalProcessAnalysisStratumV2 {
            group_id: None,
            source_rows: vec![0, 1, 2],
            case_weights: Some(vec![1.0, 2.0, 3.0]),
            frequencies: None,
        };
        let mut case_frame = ConditionalProcessAnalysisFrameV2 {
            method_version: CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2.into(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            required_source_columns: vec!["w".into(), "z".into()],
            strata: vec![base_stratum],
            analysis_row_mask_sha256: String::new(),
            excluded_rows: Vec::new(),
        };
        case_frame.analysis_row_mask_sha256 = conditional_process_analysis_row_mask_sha256_v2(
            &case_frame.dataset_fingerprint,
            &case_frame.strata,
        );
        let mut case_config = grouped_config();
        case_config.profile = ConditionalProcessProfileV2::CaseWeightedPercentile;
        case_config.grouping_column = None;
        case_config.groups.clear();
        case_config.group_contrasts.clear();
        case_config.weight = Some(AnalysisWeightBindingV1::Case { column: "w".into() });
        let case = expected_raw_probe_fit_metric_receipt_v2(
            &dataset,
            &case_frame,
            &case_config,
            &case_frame.strata[0],
            "z",
            "z",
            1,
        )
        .unwrap();
        assert!(approximately_equal_v2(
            case.effective_degrees_of_freedom,
            6.0 - 14.0 / 6.0
        ));

        let mut frequency_frame = case_frame.clone();
        frequency_frame.required_source_columns = vec!["f".into(), "z".into()];
        frequency_frame.strata[0].case_weights = None;
        frequency_frame.strata[0].frequencies = Some(vec![1, 2, 3]);
        let mut frequency_config = case_config;
        frequency_config.profile = ConditionalProcessProfileV2::FrequencyWeightedPercentile;
        frequency_config.weight = Some(AnalysisWeightBindingV1::Frequency { column: "f".into() });
        let frequency = expected_raw_probe_fit_metric_receipt_v2(
            &dataset,
            &frequency_frame,
            &frequency_config,
            &frequency_frame.strata[0],
            "z",
            "z",
            1,
        )
        .unwrap();
        let expanded = [0.0_f64, 10.0, 10.0, 20.0, 20.0, 20.0];
        assert_eq!(frequency.frequency_total, Some(expanded.len() as u64));
        assert_eq!(frequency.effective_degrees_of_freedom, 5.0);
        assert!(approximately_equal_v2(
            frequency.standard_deviation,
            sample_standard_deviation_v2(&expanded).unwrap()
        ));
        assert_ne!(
            case.standard_deviation.to_bits(),
            frequency.standard_deviation.to_bits()
        );
        assert_ne!(case.row_mass_sha256, frequency.row_mass_sha256);
    }

    #[test]
    fn derivative_evaluator_keeps_requested_zero_and_cross_targets() {
        let polynomial = ConditionalPathPolynomialV2 {
            method_version: "fixture".into(),
            path_id: "path".into(),
            relation_ids: vec!["r1".into(), "r2".into()],
            moderator_ids: vec!["w".into(), "z".into()],
            terms: vec![
                ConditionalPolynomialTermV2 {
                    powers: Vec::new(),
                    coefficient: 3.0,
                },
                ConditionalPolynomialTermV2 {
                    powers: vec![ModeratorPowerV2 {
                        moderator_id: "z".into(),
                        exponent: 1,
                    }],
                    coefficient: 2.0,
                },
                ConditionalPolynomialTermV2 {
                    powers: vec![
                        ModeratorPowerV2 {
                            moderator_id: "w".into(),
                            exponent: 1,
                        },
                        ModeratorPowerV2 {
                            moderator_id: "z".into(),
                            exponent: 1,
                        },
                    ],
                    coefficient: 4.0,
                },
                ConditionalPolynomialTermV2 {
                    powers: vec![ModeratorPowerV2 {
                        moderator_id: "z".into(),
                        exponent: 2,
                    }],
                    coefficient: 5.0,
                },
            ],
        };
        let at = probe(&[("w", 3.0), ("z", 2.0)]);
        assert_eq!(
            evaluate_polynomial_derivative_v2(&polynomial, &at, &["z".into()]).unwrap(),
            34.0
        );
        assert_eq!(
            evaluate_polynomial_derivative_v2(&polynomial, &at, &["z".into(), "w".into()]).unwrap(),
            4.0
        );
        assert_eq!(
            evaluate_polynomial_derivative_v2(&polynomial, &at, &["z".into(), "z".into()]).unwrap(),
            10.0
        );
        assert_eq!(
            evaluate_polynomial_derivative_v2(&polynomial, &at, &["w".into(), "w".into()]).unwrap(),
            0.0
        );
    }

    fn compiled_interaction(
        term_id: &str,
        relation_id: &str,
        operands: &[&str],
    ) -> CompiledInteractionIdentityV1 {
        CompiledInteractionIdentityV1 {
            term_id: term_id.into(),
            output_id: format!("{term_id}-output"),
            operands: operands.iter().map(|operand| (*operand).into()).collect(),
            focal_relation_id: relation_id.into(),
        }
    }

    #[test]
    fn derivative_inventory_omits_structural_zeros_and_keeps_nonzero_identities_stable() {
        let path = qpls_core::ConditionalProcessPathV2 {
            path_id: "x-m-y".into(),
            ordered_relation_ids: vec!["x-m".into(), "m-y".into()],
        };
        let interactions = vec![
            compiled_interaction("xz", "x-m", &["x", "z"]),
            compiled_interaction("mz", "m-y", &["m", "z"]),
            compiled_interaction("mw", "m-y", &["m", "w"]),
        ];
        let moderators = vec!["w".to_owned(), "z".to_owned()];
        let eligible =
            eligible_derivative_targets_v2(&path, &moderators, true, true, &interactions).unwrap();
        assert_eq!(
            eligible,
            vec![
                (
                    ConditionalProcessTargetKindV2::LocalFirstDerivative,
                    vec!["w".into()]
                ),
                (
                    ConditionalProcessTargetKindV2::LocalFirstDerivative,
                    vec!["z".into()]
                ),
                (
                    ConditionalProcessTargetKindV2::LocalCrossDerivative,
                    vec!["w".into(), "z".into()]
                ),
                (
                    ConditionalProcessTargetKindV2::LocalSecondDerivative,
                    vec!["z".into(), "z".into()]
                ),
            ]
        );
        assert!(!eligible.iter().any(|(kind, variables)| {
            *kind == ConditionalProcessTargetKindV2::LocalSecondDerivative
                && variables.as_slice() == ["w".to_owned(), "w".to_owned()].as_slice()
        }));

        let mut reordered = interactions.clone();
        reordered.reverse();
        assert_eq!(
            eligible_derivative_targets_v2(&path, &moderators, true, true, &reordered).unwrap(),
            eligible
        );
    }

    #[test]
    fn edge_interaction_keys_require_exact_order_insensitive_compiled_inventory() {
        let interactions = vec![
            compiled_interaction("xz", "r", &["x", "z"]),
            compiled_interaction("xw", "r", &["x", "w"]),
            compiled_interaction("xzw", "r", &["x", "z", "w"]),
        ];
        let exact = ConditionalEdgeFunctionV2 {
            relation_id: "r".into(),
            source_id: "x".into(),
            target_id: "y".into(),
            intercept: 0.5,
            linear_coefficients: vec![
                ConditionalLinearCoefficientV2 {
                    moderator_id: "w".into(),
                    estimate: 0.2,
                },
                ConditionalLinearCoefficientV2 {
                    moderator_id: "z".into(),
                    estimate: 0.1,
                },
            ],
            pairwise_coefficients: vec![ConditionalPairwiseCoefficientV2 {
                first_moderator_id: "w".into(),
                second_moderator_id: "z".into(),
                estimate: 0.3,
            }],
        };
        validate_edge_interaction_inventory_v2(&[exact.clone()], &interactions).unwrap();
        let mut reordered_interactions = interactions.clone();
        reordered_interactions.reverse();
        validate_edge_interaction_inventory_v2(&[exact.clone()], &reordered_interactions).unwrap();

        let mut extra_gamma = exact.clone();
        extra_gamma
            .linear_coefficients
            .push(ConditionalLinearCoefficientV2 {
                moderator_id: "q".into(),
                estimate: 0.4,
            });
        let gamma_error =
            validate_edge_interaction_inventory_v2(&[extra_gamma], &interactions).unwrap_err();
        assert_eq!(
            gamma_error.code,
            "multimod.conditional.refit.interaction_inventory_mismatch"
        );

        let mut extra_delta = exact;
        extra_delta
            .pairwise_coefficients
            .push(ConditionalPairwiseCoefficientV2 {
                first_moderator_id: "z".into(),
                second_moderator_id: "q".into(),
                estimate: 0.5,
            });
        let delta_error =
            validate_edge_interaction_inventory_v2(&[extra_delta], &interactions).unwrap_err();
        assert_eq!(
            delta_error.code,
            "multimod.conditional.refit.interaction_inventory_mismatch"
        );
    }

    #[test]
    fn grouped_vector_is_sorted_and_uses_left_minus_right_once() {
        let config = grouped_config();
        let templates = vec![
            template(
                ConditionalProcessTargetKindV2::ConditionalSpecificIndirect,
                "effect",
            ),
            template(
                ConditionalProcessTargetKindV2::ProbeContrast,
                "probe-contrast",
            ),
        ];
        let vectors = BTreeMap::from([("a".into(), vec![1.0, 10.0]), ("b".into(), vec![4.0, 7.0])]);
        assert_eq!(
            combine_group_vectors_v2(&config, &templates, &vectors).unwrap(),
            vec![1.0, 10.0, 4.0, 7.0, -3.0]
        );
    }

    #[test]
    fn totals_sum_only_selected_paths_by_endpoint_and_require_authored_direct_edge() {
        let mut config = grouped_config();
        config.paths = vec![
            qpls_core::ConditionalProcessPathV2 {
                path_id: "x_m_y".into(),
                ordered_relation_ids: vec!["x_m".into(), "m_y".into()],
            },
            qpls_core::ConditionalProcessPathV2 {
                path_id: "x_n_y".into(),
                ordered_relation_ids: vec!["x_n".into(), "n_y".into()],
            },
            qpls_core::ConditionalProcessPathV2 {
                path_id: "q_r_s".into(),
                ordered_relation_ids: vec!["q_r".into(), "r_s".into()],
            },
        ];
        let constant_edge = |relation_id: &str, source_id: &str, target_id: &str, estimate| {
            ConditionalEdgeFunctionV2 {
                relation_id: relation_id.into(),
                source_id: source_id.into(),
                target_id: target_id.into(),
                intercept: estimate,
                linear_coefficients: Vec::new(),
                pairwise_coefficients: Vec::new(),
            }
        };
        let edges = vec![
            constant_edge("x_m", "x", "m", 2.0),
            constant_edge("m_y", "m", "y", 3.0),
            constant_edge("x_n", "x", "n", 5.0),
            constant_edge("n_y", "n", "y", 7.0),
            constant_edge("x_y", "x", "y", 11.0),
            constant_edge("q_r", "q", "r", 13.0),
            constant_edge("r_s", "r", "s", 17.0),
        ];
        let point = probe(&[]);
        let templates = vec![
            ConditionalTargetTemplateV2 {
                target_id: "x_y_total_indirect".into(),
                kind: ConditionalProcessTargetKindV2::ConditionalTotalIndirect,
                path_id: "x_m_y".into(),
                probe_values: BTreeMap::new(),
                derivative_variables: Vec::new(),
                operation: ConditionalTargetOperationV2::TotalIndirect {
                    path_ids: vec!["x_m_y".into(), "x_n_y".into()],
                    probe: point.clone(),
                },
            },
            ConditionalTargetTemplateV2 {
                target_id: "x_y_total".into(),
                kind: ConditionalProcessTargetKindV2::ConditionalTotalEffect,
                path_id: "x_m_y".into(),
                probe_values: BTreeMap::new(),
                derivative_variables: Vec::new(),
                operation: ConditionalTargetOperationV2::TotalEffect {
                    path_ids: vec!["x_m_y".into(), "x_n_y".into()],
                    direct_relation_id: "x_y".into(),
                    probe: point.clone(),
                },
            },
            ConditionalTargetTemplateV2 {
                target_id: "q_s_total_indirect".into(),
                kind: ConditionalProcessTargetKindV2::ConditionalTotalIndirect,
                path_id: "q_r_s".into(),
                probe_values: BTreeMap::new(),
                derivative_variables: Vec::new(),
                operation: ConditionalTargetOperationV2::TotalIndirect {
                    path_ids: vec!["q_r_s".into()],
                    probe: point.clone(),
                },
            },
        ];
        assert_eq!(
            evaluate_target_templates_v2(&config, &edges, &templates).unwrap(),
            vec![41.0, 52.0, 221.0]
        );

        let missing_direct = vec![ConditionalTargetTemplateV2 {
            target_id: "missing_direct".into(),
            kind: ConditionalProcessTargetKindV2::ConditionalTotalEffect,
            path_id: "x_m_y".into(),
            probe_values: BTreeMap::new(),
            derivative_variables: Vec::new(),
            operation: ConditionalTargetOperationV2::TotalEffect {
                path_ids: vec!["x_m_y".into(), "x_n_y".into()],
                direct_relation_id: "not_authored".into(),
                probe: point,
            },
        }];
        assert!(evaluate_target_templates_v2(&config, &edges, &missing_direct).is_err());
    }

    #[test]
    fn zero_frequency_cells_compact_without_changing_the_expanded_estimand() {
        let source_rows = vec![0_u32, 1, 2, 3];
        let draw_counts = vec![2_u64, 0, 1, 3];
        let (positive_rows, positive_counts, represented_total) =
            compact_positive_frequency_cells_v2(&source_rows, &draw_counts).unwrap();
        assert_eq!(positive_rows, vec![0, 2, 3]);
        assert_eq!(positive_counts, vec![2, 1, 3]);
        assert_eq!(represented_total, 6);

        let dataset = qpls_data::import_delimited_bytes(
            b"x,f\n1,1\n99,1\n4,1\n7,1\n",
            "zero-frequency-cell.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let sampled = resample_dataset_columns_v1(
            &dataset,
            &["x".into(), "f".into()],
            &positive_rows
                .iter()
                .map(|row| *row as usize)
                .collect::<Vec<_>>(),
            || false,
        )
        .unwrap();
        let (prepared, receipt) =
            prepare_multimod_frequency_count_dataset_v1(&sampled, "f", &positive_counts).unwrap();
        assert_eq!(prepared.batch.num_rows(), 3);
        assert_eq!(receipt.frequency_total, Some(represented_total));
        assert!(receipt.exact_integer_count_space);

        let compact_mean =
            qpls_estimation::frequency_weighted_mean_v2(&[1.0, 4.0, 7.0], &positive_counts)
                .unwrap();
        let expanded = [1.0, 1.0, 4.0, 7.0, 7.0, 7.0];
        let expanded_mean = expanded.iter().sum::<f64>() / expanded.len() as f64;
        assert_eq!(compact_mean.to_bits(), expanded_mean.to_bits());
    }

    #[test]
    fn frequency_reports_represented_n_while_evidence_keeps_compact_rows() {
        let mut config = grouped_config();
        config.profile = ConditionalProcessProfileV2::FrequencyWeightedPercentile;
        config.grouping_column = None;
        config.groups.clear();
        config.group_contrasts.clear();
        config.weight = Some(AnalysisWeightBindingV1::Frequency { column: "f".into() });
        let frame = ConditionalProcessAnalysisFrameV2 {
            method_version: CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2.into(),
            dataset_fingerprint: "fixture".into(),
            required_source_columns: vec!["f".into(), "x".into()],
            strata: vec![ConditionalProcessAnalysisStratumV2 {
                group_id: None,
                source_rows: vec![0, 2, 3],
                case_weights: None,
                frequencies: Some(vec![2, 1, 3]),
            }],
            analysis_row_mask_sha256: "fixture-mask".into(),
            excluded_rows: Vec::new(),
        };
        assert_eq!(frame.strata[0].source_rows.len(), 3);
        assert_eq!(
            conditional_analysis_observations_v2(&config, &frame).unwrap(),
            6
        );

        let mut overflow = frame.clone();
        overflow.strata[0].source_rows = vec![0, 1];
        overflow.strata[0].frequencies = Some(vec![u64::MAX, 1]);
        assert!(conditional_analysis_observations_v2(&config, &overflow).is_err());
    }

    #[test]
    fn sidecar_prediction_budgets_weight_and_count_payloads_conservatively() {
        let mut unweighted = grouped_config();
        unweighted.profile = ConditionalProcessProfileV2::MultiTwoWayPercentile;
        unweighted.grouping_column = None;
        unweighted.groups.clear();
        unweighted.group_contrasts.clear();
        let base_frame = ConditionalProcessAnalysisFrameV2 {
            method_version: CONDITIONAL_PROCESS_RAW_RUNNER_METHOD_V2.into(),
            dataset_fingerprint: "fixture".into(),
            required_source_columns: vec!["x".into()],
            strata: vec![ConditionalProcessAnalysisStratumV2 {
                group_id: None,
                source_rows: vec![0, 1, 2],
                case_weights: None,
                frequencies: None,
            }],
            analysis_row_mask_sha256: "fixture-mask".into(),
            excluded_rows: Vec::new(),
        };
        let ordinary_bytes = predicted_conditional_evidence_bytes_v2(&unweighted, &base_frame, 2);

        let mut case_config = unweighted.clone();
        case_config.profile = ConditionalProcessProfileV2::CaseWeightedPercentile;
        case_config.weight = Some(AnalysisWeightBindingV1::Case { column: "w".into() });
        let mut case_frame = base_frame.clone();
        case_frame.strata[0].case_weights = Some(vec![1.0, 2.0, 3.0]);
        let case_bytes = predicted_conditional_evidence_bytes_v2(&case_config, &case_frame, 2);
        assert_eq!(
            case_bytes - ordinary_bytes,
            u64::from(case_config.inference.outer_resamples) * 3 * 8
        );

        let mut frequency_config = unweighted.clone();
        frequency_config.profile = ConditionalProcessProfileV2::FrequencyWeightedPercentile;
        frequency_config.weight = Some(AnalysisWeightBindingV1::Frequency { column: "f".into() });
        let mut frequency_frame = base_frame;
        frequency_frame.strata[0].frequencies = Some(vec![2, 1, 3]);
        let frequency_bytes =
            predicted_conditional_evidence_bytes_v2(&frequency_config, &frequency_frame, 2);
        assert_eq!(
            frequency_bytes - ordinary_bytes,
            u64::from(frequency_config.inference.outer_resamples) * 3 * 4
        );
        assert!(
            predicted_conditional_evidence_bytes_v2(
                &frequency_config,
                &frequency_frame,
                usize::MAX
            ) > MULTIMOD_SIDECAR_MAX_BYTES_V1
        );
    }
}
