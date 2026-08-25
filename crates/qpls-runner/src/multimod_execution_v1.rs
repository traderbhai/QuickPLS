//! Additive runtime adapters for the versioned MultiMod contracts.
//!
//! The statistical kernels deliberately accept prepared score/model inputs.
//! This runner owns authority validation, deterministic run control, conversion
//! into the public result families, and fail-closed publication. Existing V1
//! runners are not called or reinterpreted by this module.

use crate::{
    MgaExecutionCacheErrorV1, MgaExecutionCacheV1, MgaExecutionPlanV1, MgaExecutionShardKindV1,
    MgaExecutionShardPayloadV1, RunnerProgress, ValidatedMgaExecutionCacheSessionV1,
    build_mga_execution_plan_v1,
    multimod_row_order_v1::canonical_multimod_row_permutation_v1,
    multimod_weighted_pls_point_v1::{
        PreparedMultimodWeightedPlsPointV1, prepare_compiled_multimod_weighted_pls_point_v1,
        run_prepared_multimod_weighted_pls_point_v1,
    },
    recipe_v4_general_sem_hoc_point_execution::{
        GeneralSemPlsHocScoreAlignmentReferenceV1, align_general_sem_pls_hoc_result_signs_v1,
    },
    recipe_v4_pls_execution::project_pls_plan_to_current_recipe,
    run_compiled_pls_recipe_v4, run_compiled_pls_recipe_v4_allowing_isolated,
};
use qpls_core::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4, CompiledMultiModPlanV1,
    CompiledMultiModRecipeV1, CompiledPlsBlockModeV2, CompiledPlsPlanV2, CompiledPlsPlanV3,
    CompiledRecipePlanV4, CompositeWeightNormalizationV4, CompositeWeightingV4,
    ConditionalProcessIntervalV2 as CoreConditionalIntervalV2,
    ConditionalProcessProfileV2 as CoreConditionalProfileV2, ConditionalProcessTargetKindV2,
    ConditionalProcessTargetResultV2, DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION, ExcludedRowReceiptV1,
    GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION, GeneralSemConditionalProcessResultV2,
    GeneralSemInferenceV1, HeterogeneityAlgorithmV2 as CoreHeterogeneityAlgorithmV2,
    HeterogeneityCandidateMethodV2, HeterogeneityCandidateStateV2, HeterogeneityCandidateV2,
    HeterogeneityClassContrastV2, HeterogeneityClassParameterV2,
    HeterogeneityInferenceLockReceiptV2,
    HeterogeneityInteractionProfileV2 as CoreHeterogeneityProfileV2,
    HigherOrderConstructionApproachV4, INTERVENTIONAL_MEDIATION_RESULT_INTERPRETATION_LABEL_V1,
    INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION, InferenceAlternativeV1,
    InterventionalEffectResultV1,
    InterventionalMediationResultV1 as CoreInterventionalMediationResultV1,
    MULTIMOD_SIDECAR_MAX_BYTES_V1, MULTIMOD_SIDECAR_WARN_BYTES_V1, MethodConfig,
    MgaComparisonPlanV1, MgaGroupEligibilityV1, MgaGroupParameterV1, MgaMultigroupV1,
    MgaOmnibusComparisonV1, MgaPairwiseComparisonV1, MgaProcedureV1,
    MicomInvarianceInterpretationV1, MicomPairResultV1, MissingDataPolicy,
    MultiModAnalysisResultV1, MultiModCompilationReceiptV1, MultiModCompilerTargetV1,
    MultimodIntervalV1, MultimodParameterEstimateV1, MultimodProvenanceV1,
    MultimodQualificationStateV1, MultimodReplicateFailureKindV1, MultimodReplicateFailureV1,
    MultimodReplicateLedgerSummaryV1, MultiplicityAdjustmentV1, ObservedRoleV4, ObservedScaleV4,
    PLS_HETEROGENEITY_ANALYSIS_V2_SCHEMA_VERSION, PLS_MULTIGROUP_ANALYSIS_V1_SCHEMA_VERSION,
    PlsHeterogeneityAnalysisV2, PlsMultigroupAnalysisV1, RecipeV4CompilerTarget, SemGroupV4,
    SemModelV4, SemParameterTargetV4, SemParameterV4, SemRelationV4, SemVariableV4,
    StructuralRelationRoleV4, ValidatedExecutionRecipe, compile_analysis_recipe_v4,
    compile_multimod_recipe_v1, compile_multimod_weighted_pls_recipe_v4_v1,
    compile_pls_higher_order_lower_order_projection_multimod_v2,
    compile_pls_higher_order_repeated_stage_projection_multimod_v2,
    compile_pls_higher_order_score_stage_projection_multimod_v2, compile_pls_plan_v3,
    compile_pls_plan_v3_multimod_multiple_hoc_v2, project_general_sem_pls_base_recipe_v1,
    project_general_sem_pls_stage_one_recipe_v1, sha256_serialized,
    validate_compiled_analysis_recipe_v4, validate_compiled_multimod_recipe_v1,
};
use qpls_data::Dataset;
use qpls_estimation::{
    AlternativeHypothesisV1, ConditionalAlternativeV2, ConditionalDerivativeKindV2,
    ConditionalEdgeFunctionV2, ConditionalProbePointV2, ConditionalProcessMathErrorV2,
    EstimationError, ExplicitConditionalPathV2, FREQUENCY_MICOM_PAIRWISE_METHOD_VERSION_V1,
    FREQUENCY_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1, FimixPlsV2Config, FimixPlsV2Result,
    FitSampleKindV1, FrequencyMicomFitRequestV1, FrequencyMicomRefitterV1,
    FrequencyMultigroupDesignV1, FrequencyMultigroupFitRequestV1, FrequencyMultigroupRefitterV1,
    FrequencyPairwisePartitionPlanV1, FrequencySelectedGroupRowV1,
    GeneralSemPlsInteractionPointErrorV1, GeneralSemPlsMultipleInteractionPointResultV1,
    GeneralSemPlsThreeWayPointErrorV1, GeneralSemPlsThreeWayPointResultV1, GroupBootstrapBanksV1,
    GroupIndexV1, GroupParameterVectorV1, HeterogeneityBootstrapAlgorithmV2,
    HeterogeneityBootstrapLedgerEntryV2, HeterogeneityBootstrapPlanV2,
    HeterogeneityBootstrapQualificationV2,
    HeterogeneityInteractionProfileV2 as EstimationHeterogeneityProfileV2, HeterogeneityV2Error,
    InferenceAvailabilityV1, InterventionalCausalMediationInputV1,
    InterventionalMediationResultV1 as EstimationInterventionalMediationResultV1, LabelAlignmentV2,
    MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1, MGA_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1,
    MGA_MULTIGROUP_PAIRWISE_PERMUTATION_VERSION_V1, MICOM_CASE_WEIGHTED_PAIRWISE_METHOD_VERSION_V1,
    MICOM_PAIRWISE_METHOD_VERSION_V1, MicomConfiguralReceiptV1, MicomFitKindV1, MicomFitRequestV1,
    MicomFitV1, MicomGroupConstructScoresV1, MicomPairwiseResultV1, MicomPermutationConfigV1,
    MicomPermutationStatusV1, MicomPooledConstructScoresV1, MicomRefitterV1, MultigroupDesignV1,
    MultigroupEligibilityV1, MultigroupFitRequestV1, MultigroupRefitterV1,
    MultigroupResamplingConfigV1, OmnibusPermutationResultV1, OrderedGroupPairV1,
    PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2, PLS_POS_PUBLISHED_METHOD_VERSION_V2,
    PLSC_METHOD_VERSION, POS_STANDARDIZED_OUTCOME_MEAN_TOLERANCE_V2, PairwisePartitionPlanV1,
    PairwisePermutationResultV1, ParameterEstimateV1, ParameterFamilyV1, ParameterIdentityV1,
    ParameterVectorV1, ParametricGroupEstimateV1, ParametricGroupSeMethodV1, PlsAliasColumnSpecV1,
    PlsPointEstimateAttributionV1, PlsPosFullRefitterV2, PlsPosV2Config, PlsPosV2Result, PlsResult,
    PooledStandardizedMetricReceiptV2, PooledStructuralBaselineV2, PosCommonMetricGateInputV1,
    PosCommonMetricGateResultV1, PosCommonMetricGateStatusV1, PosConstructComparabilityEvidenceV1,
    PosFullRefitReceiptV2, PosOutcomeFitAuditV2, PosOutcomeR2V2,
    PosPairwiseCompositionalInvarianceV1, PosPairwiseStep3EqualityV1, PosScoringContractV2,
    PosSegmentFullFitV2, PosSegmentRefitRequestV2, RefitFailureCodeV1, RefitFailureV1,
    ResampleFitStatusV1, SelectedGroupRowV1, StandardizedFimixInputV2,
    StandardizedStructuralEquationV2, StudentizedOuterReplicateV2, WaldGroupEstimateV1,
    adjust_probabilities_v1, align_labels_exhaustive_v2, append_pls_alias_columns_v1,
    assess_frequency_multigroup_design_v1, assess_multigroup_design_v1, bca_interval_v2,
    bias_corrected_interval_for_alternative_v1, build_frequency_pairwise_partition_plan_v1,
    build_pairwise_partition_plan_from_rows_v1, build_pairwise_partition_plan_v1,
    build_pls_pos_start_plan_v2, compile_explicit_conditional_path_v2, conditional_derivatives_v2,
    conditional_effect_v2, conditional_probe_contrast_v2,
    estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control,
    estimate_general_sem_pls_three_way_moderation_v1_with_control,
    estimate_interventional_mediation_v1, estimate_pls_validated_with_control,
    evaluate_pos_common_metric_gate_v1, fit_fimix_pls_v2,
    fit_pls_pos_destination_scored_interactions_with_scientific_row_features_v2,
    fit_pls_pos_published_with_scientific_row_features_v2, fit_pooled_metric_segment_baselines_v2,
    fit_pooled_structural_baseline_v2, henseler_directional_probabilities_v1,
    heterogeneity_bootstrap_replicate_seed_v2, heterogeneity_target_payload_sha256_v2,
    inverse_variance_wald_test_v1, multimod_case_weights_for_source_rows_v1,
    multimod_frequency_counts_for_source_rows_v1, ordinary_pls_path_standard_error_v1,
    percentile_interval_v2, pooled_variance_parameter_test_v1,
    prepare_general_sem_pls_disjoint_hoc_score_dataset_multimod_v2,
    prepare_multimod_case_weight_dataset_v1, prepare_multimod_frequency_count_dataset_v1,
    run_frequency_group_bootstrap_banks_v1, run_frequency_max_spread_omnibus_permutation_v1,
    run_frequency_pairwise_micom_with_partition_plan_v1,
    run_frequency_pairwise_permutation_with_plan_v1, run_group_bootstrap_banks_v1,
    run_max_spread_omnibus_permutation_v1, run_pairwise_case_weighted_micom_with_partition_plan_v1,
    run_pairwise_micom_with_partition_plan_v1, run_pairwise_permutation_with_plan_v1,
    scalar_index_of_moderated_mediation_v2, studentized_interval_v2,
    summarize_heterogeneity_bootstrap_ledger_v2, validate_fimix_multistart_evidence_v2,
    validate_pos_multistart_evidence_v2, validate_retained_label_alignment_v2,
    welch_satterthwaite_parameter_test_v1,
};
use qpls_resampling::{
    MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1, MultiModBootstrapPlanV1, MultiModCaseBootstrapDrawV1,
    MultiModFinalLedgerV1, MultiModInterruptibleFullRefitCallbackV1, MultiModRefitAttemptV1,
    MultiModRefitFailureV1, MultiModRefitOutcomeV1, MultiModShardCacheV1, MultiModShardSpecV1,
    finalize_multimod_case_bootstrap_v1, resample_dataset_columns_v1,
    run_multimod_case_bootstrap_shard_interruptible_v1,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

pub const MULTIMOD_RUNNER_METHOD_VERSION_V1: &str = "multimod_runner_v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiModSidecarCostStateV1 {
    WithinLimit,
    Warning,
    Blocked,
}

/// Classifies one conservative uncompressed-evidence prediction against the
/// archive V6 thresholds. Exactly 128 MiB does not warn and exactly 512 MiB
/// remains executable.
pub const fn multimod_sidecar_cost_state_v1(bytes: u64) -> MultiModSidecarCostStateV1 {
    if bytes > MULTIMOD_SIDECAR_MAX_BYTES_V1 {
        MultiModSidecarCostStateV1::Blocked
    } else if bytes > MULTIMOD_SIDECAR_WARN_BYTES_V1 {
        MultiModSidecarCostStateV1::Warning
    } else {
        MultiModSidecarCostStateV1::WithinLimit
    }
}

const fn sidecar_cost_add_v1(left: u64, right: u64) -> u64 {
    match left.checked_add(right) {
        Some(value) => value,
        None => u64::MAX,
    }
}

const fn sidecar_cost_mul_v1(left: u64, right: u64) -> u64 {
    match left.checked_mul(right) {
        Some(value) => value,
        None => u64::MAX,
    }
}

fn sidecar_cost_sum_v1(values: impl IntoIterator<Item = u64>) -> u64 {
    values.into_iter().fold(0, sidecar_cost_add_v1)
}

// These bounds match the trusted Archive V6 Arrow layouts. Dictionary-backed
// target ledgers retain one UInt32 key instead of repeated UTF-8 target IDs.
// MICOM null rows use replicate/construct ordinals, a UInt8 statistic kind,
// and one Float64 value (17 physical data bytes per row); rounding to 18 plus
// a 4 KiB stream allowance covers buffer alignment and IPC metadata.
const MGA_ARROW_STREAM_FIXED_BYTES_V1: u64 = 4 * 1024;
const MGA_DICTIONARY_TARGET_LEDGER_ROW_BYTES_V1: u64 = 21;
const MGA_MICOM_NULL_STATISTIC_ROW_BYTES_V1: u64 = 18;

fn dictionary_value_bytes_upper_bound_v1(unique_values: u64, maximum_value_bytes: u64) -> u64 {
    sidecar_cost_add_v1(
        MGA_ARROW_STREAM_FIXED_BYTES_V1,
        sidecar_cost_mul_v1(unique_values, sidecar_cost_add_v1(maximum_value_bytes, 4)),
    )
}

pub fn predict_mga_micom_null_statistics_arrow_bytes_v1(rows: u64) -> u64 {
    sidecar_cost_add_v1(
        MGA_ARROW_STREAM_FIXED_BYTES_V1,
        sidecar_cost_mul_v1(rows, MGA_MICOM_NULL_STATISTIC_ROW_BYTES_V1),
    )
}

fn mga_pair_count_v1(config: &MgaMultigroupV1) -> u64 {
    let groups = config.groups.len() as u64;
    match &config.comparison_plan {
        MgaComparisonPlanV1::ReferenceVsRest { .. } => groups.saturating_sub(1),
        MgaComparisonPlanV1::SelectedPairs { pairs } => pairs.len() as u64,
        MgaComparisonPlanV1::AllPairs { .. } => {
            sidecar_cost_mul_v1(groups, groups.saturating_sub(1)) / 2
        }
    }
}

/// Conservative uncompressed Arrow-sidecar prediction for raw MGA.
/// `compact_group_rows` contains the physically stored complete rows in each
/// selected group. Case weights and frequency counts add one f64/u64
/// coordinate per draw; frequency expansion itself is never materialized.
/// A shared pairwise partition plan is charged once even when both MICOM and
/// permutation MGA consume it. Repeated target/construct identifiers are
/// charged at their compressed coordinate cost, while random partition hashes
/// and numeric payloads are charged in full.
pub fn predict_mga_sidecar_bytes_v1(
    config: &MgaMultigroupV1,
    compact_group_rows: &[u64],
    target_count: usize,
    maximum_target_id_bytes: usize,
    micom_construct_count: usize,
) -> u64 {
    if compact_group_rows.len() != config.groups.len() {
        return u64::MAX;
    }
    let rows = sidecar_cost_sum_v1(compact_group_rows.iter().copied());
    let groups = config.groups.len() as u64;
    let targets = target_count.max(1) as u64;
    let maximum_target_id_bytes = maximum_target_id_bytes.max(1) as u64;
    let micom_constructs = micom_construct_count.max(1) as u64;
    let pairs = mga_pair_count_v1(config);
    let permutations = u64::from(config.permutation_samples);
    let bootstraps = u64::from(config.bootstrap_samples);
    let extra_draw_coordinate = if matches!(
        config.profile,
        qpls_core::MgaModelProfileV1::CaseWeightedPls
            | qpls_core::MgaModelProfileV1::FrequencyWeightedPls
    ) {
        8
    } else {
        0
    };
    let draw_row_bytes = 8 + extra_draw_coordinate;
    let mut total = sidecar_cost_add_v1(
        sidecar_cost_mul_v1(rows, 112 + extra_draw_coordinate),
        sidecar_cost_mul_v1(sidecar_cost_mul_v1(groups, targets), 64),
    );

    let uses_pairwise_plan = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::MicomPairwise | MgaProcedureV1::PairwisePermutation
        )
    });
    if uses_pairwise_plan {
        // The plan sidecar retains one replicate identity and one SHA-256
        // partition digest, never the permuted row vector itself.
        let per_draw = 80;
        total = sidecar_cost_add_v1(
            total,
            sidecar_cost_mul_v1(sidecar_cost_mul_v1(pairs, permutations), per_draw),
        );
    }
    if config
        .procedures
        .contains(&MgaProcedureV1::PairwisePermutation)
    {
        // Two group-fit ledger rows plus the single bounded raw null target
        // admitted for independent probability reconstruction.
        let per_draw = 224;
        total = sidecar_cost_add_v1(
            total,
            sidecar_cost_mul_v1(sidecar_cost_mul_v1(pairs, permutations), per_draw),
        );
        total = sidecar_cost_add_v1(
            total,
            sidecar_cost_mul_v1(
                pairs,
                dictionary_value_bytes_upper_bound_v1(1, maximum_target_id_bytes),
            ),
        );
    }
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        // One MICOM ledger row plus the ordinal Arrow table containing all
        // Step-2 construct correlations and the single bounded Step-3
        // mean/variance audit construct.
        let null_rows_per_pair =
            sidecar_cost_mul_v1(permutations, sidecar_cost_add_v1(micom_constructs, 2));
        total = sidecar_cost_add_v1(
            total,
            sidecar_cost_mul_v1(
                pairs,
                sidecar_cost_add_v1(
                    sidecar_cost_mul_v1(permutations, 96),
                    predict_mga_micom_null_statistics_arrow_bytes_v1(null_rows_per_pair),
                ),
            ),
        );
    }
    if config
        .procedures
        .contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation)
    {
        let per_draw = sidecar_cost_add_v1(
            sidecar_cost_mul_v1(rows, draw_row_bytes),
            sidecar_cost_add_v1(
                sidecar_cost_mul_v1(targets, MGA_DICTIONARY_TARGET_LEDGER_ROW_BYTES_V1),
                128,
            ),
        );
        total = sidecar_cost_add_v1(total, sidecar_cost_mul_v1(permutations, per_draw));
        total = sidecar_cost_add_v1(
            total,
            dictionary_value_bytes_upper_bound_v1(targets, maximum_target_id_bytes),
        );
    }
    let uses_bootstrap_bank = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::HenselerPlsMga | MgaProcedureV1::BootstrapDifferenceBc
        )
    });
    if uses_bootstrap_bank {
        let target_cells = sidecar_cost_mul_v1(groups, targets);
        let per_draw = sidecar_cost_add_v1(
            sidecar_cost_mul_v1(target_cells, MGA_DICTIONARY_TARGET_LEDGER_ROW_BYTES_V1),
            sidecar_cost_mul_v1(groups, 112),
        );
        total = sidecar_cost_add_v1(total, sidecar_cost_mul_v1(bootstraps, per_draw));
        // `group:<u8>:` is at most nine UTF-8 bytes for the admitted 20 groups.
        total = sidecar_cost_add_v1(
            total,
            dictionary_value_bytes_upper_bound_v1(
                target_cells,
                sidecar_cost_add_v1(maximum_target_id_bytes, 9),
            ),
        );
    }
    total
}

/// Conservative Arrow-sidecar prediction for raw FIMIX/PLS-POS V2. It covers
/// the pooled row map, all discovery candidates, complete FIMIX start traces
/// and posterior matrices, POS memberships/objective histories, fixed-K case
/// bootstrap targets and label alignment, and optional common-metric MICOM.
pub fn predict_heterogeneity_sidecar_bytes_v2(
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    rows: usize,
    target_count: usize,
) -> u64 {
    let rows = rows as u64;
    let targets = target_count.max(1) as u64;
    let (candidate_k, algorithms) = match &config.phase {
        qpls_core::HeterogeneityPhaseV2::Discovery {
            candidate_k,
            algorithms,
        } => (candidate_k.as_slice(), algorithms.as_slice()),
        qpls_core::HeterogeneityPhaseV2::Inference { lock } => (
            lock.discovery_candidate_k.as_slice(),
            lock.discovery_algorithms.as_slice(),
        ),
    };
    let mut total = sidecar_cost_add_v1(
        sidecar_cost_mul_v1(rows, 112),
        sidecar_cost_add_v1(
            sidecar_cost_mul_v1(targets, 128),
            sidecar_cost_mul_v1(sidecar_cost_mul_v1(rows, targets), 64),
        ),
    );
    for k in candidate_k.iter().copied().map(u64::from) {
        for algorithm in algorithms {
            let candidate = match algorithm {
                CoreHeterogeneityAlgorithmV2::FimixPlsV2 => {
                    let posterior = sidecar_cost_mul_v1(sidecar_cost_mul_v1(rows, k), 8);
                    let membership = sidecar_cost_mul_v1(rows, 8);
                    let trace_row = sidecar_cost_add_v1(40, sidecar_cost_mul_v1(k, 8));
                    let traces = sidecar_cost_mul_v1(
                        sidecar_cost_mul_v1(
                            u64::from(config.fimix.starts),
                            u64::from(config.fimix.max_iterations),
                        ),
                        trace_row,
                    );
                    let parameters = sidecar_cost_mul_v1(sidecar_cost_mul_v1(k, targets), 48);
                    sidecar_cost_sum_v1([posterior, membership, traces, parameters, 2_048])
                }
                CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2
                | CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
                    let starts = u64::from(config.pls_pos.starts);
                    let move_cap = 1_000_u64.max(sidecar_cost_mul_v1(2, rows));
                    let start_memberships =
                        sidecar_cost_mul_v1(sidecar_cost_mul_v1(starts, rows), 8);
                    let objective_and_failure_traces =
                        sidecar_cost_mul_v1(sidecar_cost_mul_v1(starts, move_cap), 80);
                    let final_membership = sidecar_cost_mul_v1(rows, 8);
                    let parameters = sidecar_cost_mul_v1(sidecar_cost_mul_v1(k, targets), 48);
                    let final_outcome_audit =
                        sidecar_cost_mul_v1(sidecar_cost_mul_v1(rows, targets), 48);
                    sidecar_cost_sum_v1([
                        start_memberships,
                        objective_and_failure_traces,
                        final_membership,
                        parameters,
                        final_outcome_audit,
                        2_048,
                    ])
                }
            };
            total = sidecar_cost_add_v1(total, candidate);
        }
    }

    if let (qpls_core::HeterogeneityPhaseV2::Inference { lock }, Some(bootstrap)) =
        (&config.phase, &config.bootstrap)
    {
        let k = u64::from(lock.selected_k);
        let per_draw = sidecar_cost_sum_v1([
            sidecar_cost_mul_v1(rows, 8),
            sidecar_cost_mul_v1(sidecar_cost_mul_v1(k, targets), 8),
            sidecar_cost_mul_v1(sidecar_cost_mul_v1(k, k), 8),
            192,
        ]);
        total = sidecar_cost_add_v1(
            total,
            sidecar_cost_mul_v1(u64::from(bootstrap.resamples), per_draw),
        );
    }

    if let (qpls_core::HeterogeneityPhaseV2::Inference { lock }, Some(gate)) =
        (&config.phase, &config.pos_common_metric)
    {
        if gate.request_segment_contrasts {
            let k = u64::from(lock.selected_k);
            let pairs = sidecar_cost_mul_v1(k, k.saturating_sub(1)) / 2;
            let per_draw = sidecar_cost_sum_v1([
                sidecar_cost_mul_v1(rows, 8),
                sidecar_cost_mul_v1(targets, 24),
                160,
            ]);
            total = sidecar_cost_add_v1(
                total,
                sidecar_cost_mul_v1(
                    sidecar_cost_mul_v1(pairs, u64::from(gate.permutation_samples)),
                    per_draw,
                ),
            );
        }
    }
    total
}

fn enforce_multimod_sidecar_cost_v1<P>(
    family: &str,
    predicted_bytes: u64,
    progress: &P,
) -> Result<(), MultiModRunnerErrorV1>
where
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    match multimod_sidecar_cost_state_v1(predicted_bytes) {
        MultiModSidecarCostStateV1::Blocked => {
            Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.archive.sidecar_limit: predicted {family} evidence is {predicted_bytes} bytes"
            )))
        }
        MultiModSidecarCostStateV1::Warning => {
            report(
                progress,
                MultiModRunnerPhaseV1::PreparingPointInputs,
                0,
                1,
                format!("multimod.archive.sidecar_warning:{family}:{predicted_bytes}"),
            );
            Ok(())
        }
        MultiModSidecarCostStateV1::WithinLimit => Ok(()),
    }
}

fn multimod_model_target_upper_bound_v1(
    config_selected_targets: &[String],
    model: &SemModelV4,
) -> usize {
    if !config_selected_targets.is_empty() {
        return config_selected_targets.len();
    }
    let modeled_variables = model
        .variables
        .iter()
        .filter(|variable| !matches!(variable, SemVariableV4::Observed { .. }))
        .count();
    model
        .parameters
        .len()
        .saturating_add(model.relations.len())
        .saturating_add(modeled_variables.saturating_mul(2))
        .saturating_add(model.derived_terms.len().saturating_mul(8))
        .max(1)
}

fn multimod_model_target_id_maximum_bytes_v1(
    config_selected_targets: &[String],
    model: &SemModelV4,
) -> usize {
    if !config_selected_targets.is_empty() {
        return config_selected_targets
            .iter()
            .map(String::len)
            .max()
            .unwrap_or(1)
            .max(1);
    }
    let maximum_model_atom = std::iter::once(model.id.as_str())
        .chain(model.variables.iter().map(SemVariableV4::id))
        .chain(
            model
                .relations
                .iter()
                .flat_map(|relation| [relation.id(), relation.parameter()].into_iter()),
        )
        .chain(model.parameters.iter().map(|parameter| parameter.id()))
        .chain(
            model
                .derived_terms
                .iter()
                .flat_map(|term| [term.id(), term.output()].into_iter()),
        )
        .map(str::len)
        .max()
        .unwrap_or(1)
        .max(1);
    // Every admitted MGA target identity is either an authored parameter ID
    // or a bounded derived identity composed from no more than six model IDs
    // plus stable separators/probe labels. Saturation fails the byte preflight
    // closed for pathological authoring documents.
    maximum_model_atom.saturating_mul(6).saturating_add(192)
}

fn multimod_model_micom_construct_upper_bound_v1(model: &SemModelV4) -> usize {
    model
        .variables
        .iter()
        .filter(|variable| !matches!(variable, SemVariableV4::Observed { .. }))
        .count()
        .max(1)
}

fn compact_group_row_counts_v1(design: &MultigroupDesignV1) -> Vec<u64> {
    design
        .groups
        .iter()
        .map(|group| {
            design
                .rows
                .iter()
                .filter(|row| row.group == group.index)
                .count() as u64
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiModRunnerPhaseV1 {
    ValidatingAuthority,
    PreparingPointInputs,
    PointEstimation,
    Resampling,
    Multiplicity,
    AssemblingResult,
    Completed,
}

impl MultiModRunnerPhaseV1 {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::ValidatingAuthority => "multimod_validating_authority",
            Self::PreparingPointInputs => "multimod_preparing_point_inputs",
            Self::PointEstimation => "multimod_point_estimation",
            Self::Resampling => "multimod_resampling",
            Self::Multiplicity => "multimod_multiplicity",
            Self::AssemblingResult => "multimod_assembling_result",
            Self::Completed => "multimod_completed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModRunnerProgressV1 {
    pub phase: MultiModRunnerPhaseV1,
    pub completed_units: u64,
    pub total_units: u64,
    pub shard_id: String,
}

impl From<MultiModRunnerProgressV1> for RunnerProgress {
    fn from(value: MultiModRunnerProgressV1) -> Self {
        Self {
            phase: value.phase.stable_id().to_owned(),
            completed_units: value.completed_units,
            total_units: value.total_units,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MultiModRunnerErrorV1 {
    #[error("analysis was cancelled")]
    Cancelled,
    #[error("MultiMod authority validation failed: {0}")]
    Authority(String),
    #[error("prepared execution input is incompatible with the compiled plan: {0}")]
    PreparedInput(String),
    #[error("the requested profile has no qualified runtime adapter: {0}")]
    UnsupportedProfile(String),
    #[error("MultiMod statistical kernel failed: {0}")]
    Kernel(String),
    #[error("the shared resampling ledger is invalid: {0}")]
    InvalidLedger(String),
    #[error("the public result contract cannot represent the requested inference: {0}")]
    ResultContract(String),
    #[error("the resumable MGA execution cache is invalid: {0}")]
    ExecutionCache(String),
}

impl MultiModRunnerErrorV1 {
    pub const fn stable_code(&self) -> &'static str {
        match self {
            Self::Cancelled => "multimod.runner.cancelled",
            Self::Authority(_) => "multimod.runner.authority",
            Self::PreparedInput(_) => "multimod.runner.prepared_input",
            Self::UnsupportedProfile(_) => "multimod.runner.unsupported_profile",
            Self::Kernel(_) => "multimod.runner.kernel",
            Self::InvalidLedger(_) => "multimod.runner.ledger",
            Self::ResultContract(_) => "multimod.runner.result_contract",
            Self::ExecutionCache(_) => "multimod.runner.mga.execution_cache",
        }
    }
}

fn map_mga_execution_cache_error_v1(error: MgaExecutionCacheErrorV1) -> MultiModRunnerErrorV1 {
    match error {
        MgaExecutionCacheErrorV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
        other => MultiModRunnerErrorV1::ExecutionCache(other.to_string()),
    }
}

type MgaShardCheckpointCallbackV1<'a> =
    dyn FnMut(&MgaExecutionPlanV1, &MgaExecutionCacheV1) -> Result<(), String> + 'a;

fn execute_or_reuse_mga_shard_checkpointed_v1<F, C>(
    session: &mut ValidatedMgaExecutionCacheSessionV1<'_>,
    kind: &MgaExecutionShardKindV1,
    should_cancel: C,
    execute: F,
    checkpoint: Option<&mut MgaShardCheckpointCallbackV1<'_>>,
) -> Result<MgaExecutionShardPayloadV1, MgaExecutionCacheErrorV1>
where
    F: FnOnce() -> Result<MgaExecutionShardPayloadV1, MgaExecutionCacheErrorV1>,
    C: Fn() -> bool,
{
    if should_cancel() {
        return Err(MgaExecutionCacheErrorV1::Cancelled);
    }
    if let Some(payload) = session.payload(kind)? {
        return Ok(payload.clone());
    }
    let payload = execute()?;
    session.insert(kind, payload.clone())?;
    if let Some(checkpoint) = checkpoint {
        checkpoint(session.plan(), session.cache())
            .map_err(MgaExecutionCacheErrorV1::CheckpointFailed)?;
    }
    if should_cancel() {
        return Err(MgaExecutionCacheErrorV1::Cancelled);
    }
    Ok(payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiModRunOutputV1 {
    pub compilation_receipt: MultiModCompilationReceiptV1,
    pub result: MultiModAnalysisResultV1,
    /// Large in-memory evidence is deliberately excluded from JSON. The
    /// project layer must encode it into checked Arrow sidecars before a
    /// scientific result can be persisted.
    #[serde(skip)]
    pub evidence: Vec<MultiModRunnerEvidenceV1>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MultiModRunnerEvidenceV1 {
    MgaPairwisePartitionPlan(PairwisePartitionPlanV1),
    MgaFrequencyPairwisePartitionPlan(FrequencyPairwisePartitionPlanV1),
    MgaPairwisePermutation(PairwisePermutationResultV1),
    MgaOmnibusPermutation(OmnibusPermutationResultV1),
    MgaBootstrapBanks(qpls_estimation::GroupBootstrapBanksV1),
    MgaMicomPair(MicomPairwiseResultV1),
    MgaOrdinaryPlsPathStandardError {
        parameter: ParameterIdentityV1,
        group: GroupIndexV1,
        receipt: qpls_estimation::OrdinaryPlsPathStandardErrorV1,
    },
    MgaPairwiseParametric(qpls_estimation::PairwiseParametricTestV1),
    MgaParametricWald(qpls_estimation::InverseVarianceWaldResultV1),
    FimixCandidate {
        k: u8,
        result: FimixPlsV2Result,
    },
    PlsPosCandidate {
        k: u8,
        result: PlsPosV2Result,
    },
    HeterogeneityPooledBaseline(PooledStructuralBaselineV2),
    HeterogeneityRawPreparation(RawHeterogeneityPreparationReceiptV2),
    HeterogeneityPosCommonMetric(PreparedPosCommonMetricEvidenceV1),
    HeterogeneityBootstrap(PreparedHeterogeneityBootstrapV2),
    ConditionalInference(PreparedConditionalInferenceV2),
    ConditionalRawPreparation(crate::ConditionalProcessAnalysisFrameV2),
    ConditionalRawFullRefit(crate::RawConditionalProcessEvidenceV2),
    InterventionalBootstrap(PreparedInterventionalBootstrapV1),
    InterventionalFullRefitLedger(
        qpls_resampling::MultiModFinalLedgerV1<
            qpls_resampling::MultiModCaseBootstrapDrawV1,
            Vec<f64>,
        >,
    ),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultiModRuntimeReadinessV1 {
    /// Dataset, recipe, and model are sufficient for the runner itself.
    BuiltInFromDataset,
    /// A typed point/refit or complete resampling adapter is still required.
    PreparedAdapterRequired,
    /// The requested procedure has no safe publication adapter.
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiModRuntimeSupportV1 {
    pub target: MultiModCompilerTargetV1,
    pub readiness: MultiModRuntimeReadinessV1,
    pub stable_reason_codes: Vec<String>,
}

/// Reports runtime truth independently from compiler eligibility. A compiled
/// scientific profile is not advertised as executable merely because its
/// configuration and point math are valid.
pub fn multimod_runtime_support_v1(
    recipe: &AnalysisRecipeV4,
    target: MultiModCompilerTargetV1,
) -> MultiModRuntimeSupportV1 {
    let mut reasons = Vec::new();
    let readiness = match target {
        MultiModCompilerTargetV1::MgaMultigroupV1 => match recipe.mga_multigroup.as_ref() {
            None => {
                reasons.push("multimod.runner.mga.config_absent".into());
                MultiModRuntimeReadinessV1::Blocked
            }
            Some(config) => {
                let requests_parametric = config.procedures.iter().any(|procedure| {
                    matches!(
                        procedure,
                        MgaProcedureV1::ParametricPooledVariance
                            | MgaProcedureV1::ParametricWelchSatterthwaite
                            | MgaProcedureV1::ParametricWaldOmnibus
                    )
                });
                match (config.profile, requests_parametric) {
                    (qpls_core::MgaModelProfileV1::GeneralSemPls, false) => {
                        MultiModRuntimeReadinessV1::BuiltInFromDataset
                    }
                    (qpls_core::MgaModelProfileV1::GeneralSemPls, true)
                        if config.selected_parameter_ids.is_empty() =>
                    {
                        reasons.push(
                            "multimod.runner.mga.parametric_explicit_path_targets_required".into(),
                        );
                        MultiModRuntimeReadinessV1::Blocked
                    }
                    (qpls_core::MgaModelProfileV1::GeneralSemPls, true) => {
                        reasons.push(
                            "multimod.runner.mga.parametric_structural_path_targets_only".into(),
                        );
                        MultiModRuntimeReadinessV1::BuiltInFromDataset
                    }
                    (
                        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
                        | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
                        | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation,
                        false,
                    ) => MultiModRuntimeReadinessV1::BuiltInFromDataset,
                    (
                        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
                        | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
                        | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation,
                        true,
                    ) => {
                        reasons.push(
                            "multimod.runner.mga.interaction_parametric_se_semantics_unavailable"
                                .into(),
                        );
                        MultiModRuntimeReadinessV1::Blocked
                    }
                    (
                        qpls_core::MgaModelProfileV1::ReflectivePlsc
                        | qpls_core::MgaModelProfileV1::CaseWeightedPls,
                        false,
                    ) => MultiModRuntimeReadinessV1::BuiltInFromDataset,
                    (
                        qpls_core::MgaModelProfileV1::ReflectivePlsc
                        | qpls_core::MgaModelProfileV1::CaseWeightedPls,
                        true,
                    ) => {
                        reasons.push(
                            "multimod.runner.mga.parametric_se_semantics_unavailable_for_profile"
                                .into(),
                        );
                        MultiModRuntimeReadinessV1::Blocked
                    }
                    (qpls_core::MgaModelProfileV1::MultipleNonnestedHoc, false) => {
                        MultiModRuntimeReadinessV1::BuiltInFromDataset
                    }
                    (qpls_core::MgaModelProfileV1::MultipleNonnestedHoc, true) => {
                        reasons.push(
                            "multimod.runner.mga.hoc_parametric_se_semantics_unavailable".into(),
                        );
                        MultiModRuntimeReadinessV1::Blocked
                    }
                    (qpls_core::MgaModelProfileV1::FrequencyWeightedPls, false) => {
                        MultiModRuntimeReadinessV1::BuiltInFromDataset
                    }
                    (qpls_core::MgaModelProfileV1::FrequencyWeightedPls, true) => {
                        reasons.push(
                            "multimod.runner.mga.frequency_parametric_se_semantics_unavailable"
                                .into(),
                        );
                        MultiModRuntimeReadinessV1::Blocked
                    }
                }
            }
        },
        MultiModCompilerTargetV1::PlsHeterogeneityV2 => {
            if recipe.pls_heterogeneity.is_some() {
                if recipe.general_sem_config.is_none() {
                    reasons
                        .push("multimod.runner.heterogeneity.general_sem_config_required".into());
                    MultiModRuntimeReadinessV1::Blocked
                } else if recipe
                    .general_sem_config
                    .as_ref()
                    .is_some_and(|general| general.inference != GeneralSemInferenceV1::None)
                {
                    reasons.push(
                        "multimod.runner.heterogeneity.point_general_sem_authority_required".into(),
                    );
                    MultiModRuntimeReadinessV1::Blocked
                } else if recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion {
                    reasons.push(
                        "multimod.runner.heterogeneity.listwise_complete_scores_required".into(),
                    );
                    MultiModRuntimeReadinessV1::Blocked
                } else {
                    MultiModRuntimeReadinessV1::BuiltInFromDataset
                }
            } else {
                reasons.push("multimod.runner.heterogeneity.config_absent".into());
                MultiModRuntimeReadinessV1::Blocked
            }
        }
        MultiModCompilerTargetV1::GeneralSemConditionalProcessV2 => {
            if let Some(config) = recipe.general_sem_conditional_process.as_ref() {
                match config.profile {
                    CoreConditionalProfileV2::MultiTwoWayPercentile
                    | CoreConditionalProfileV2::MultiTwoWayBca
                    | CoreConditionalProfileV2::MultiTwoWayStudentized
                    | CoreConditionalProfileV2::BoundedThreeWayPercentile
                    | CoreConditionalProfileV2::MultipleHocPercentile
                    | CoreConditionalProfileV2::GroupedPercentile
                    | CoreConditionalProfileV2::CaseWeightedPercentile
                    | CoreConditionalProfileV2::FrequencyWeightedPercentile => {
                        MultiModRuntimeReadinessV1::BuiltInFromDataset
                    }
                }
            } else {
                reasons.push("multimod.runner.conditional.config_absent".into());
                MultiModRuntimeReadinessV1::Blocked
            }
        }
        MultiModCompilerTargetV1::InterventionalCausalMediationV1 => {
            if recipe.interventional_causal_mediation.is_some() {
                MultiModRuntimeReadinessV1::BuiltInFromDataset
            } else {
                reasons.push("multimod.runner.causal.config_absent".into());
                MultiModRuntimeReadinessV1::Blocked
            }
        }
    };
    MultiModRuntimeSupportV1 {
        target,
        readiness,
        stable_reason_codes: reasons,
    }
}

/// Resolves the exact typed target inventory that the raw MGA runner can
/// publish for this compiled authority and selected-row design. Interaction
/// profiles deliberately execute the real pooled joint point seam so native
/// preflight cannot drift from gamma/delta/slope target identities.
pub fn multimod_mga_publishable_parameter_identities_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
) -> Result<Vec<ParameterIdentityV1>, MultiModRunnerErrorV1> {
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    validate_prepared_group_membership_v1(dataset, config, design)?;
    let ordinary_authority = matches!(
        config.profile,
        qpls_core::MgaModelProfileV1::GeneralSemPls
            | qpls_core::MgaModelProfileV1::ReflectivePlsc
            | qpls_core::MgaModelProfileV1::CaseWeightedPls
            | qpls_core::MgaModelProfileV1::FrequencyWeightedPls
    )
    .then(|| projected_ordinary_pls_authority_v1(recipe, model, config))
    .transpose()?;
    let eligible = if config.profile == qpls_core::MgaModelProfileV1::FrequencyWeightedPls {
        let weight_source_column = ordinary_authority
            .as_ref()
            .and_then(OrdinaryPlsPointAuthorityV1::weight_source_column)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(
                    "frequency-weighted MGA target resolution lost its resolved weight source column"
                        .into(),
                )
            })?;
        let (frequency_design, _, _) =
            frequency_multigroup_design_from_raw_v1(dataset, weight_source_column, design)?;
        assess_frequency_multigroup_design_v1(&frequency_design).eligible
    } else {
        assess_multigroup_design_v1(design).eligible
    };
    if !eligible {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "MGA target resolution requires an eligible selected-row design".into(),
        ));
    }
    match config.profile {
        qpls_core::MgaModelProfileV1::GeneralSemPls
        | qpls_core::MgaModelProfileV1::ReflectivePlsc
        | qpls_core::MgaModelProfileV1::CaseWeightedPls
        | qpls_core::MgaModelProfileV1::FrequencyWeightedPls => {
            let authority = ordinary_authority.expect("ordinary profile authority prepared above");
            Ok(ordinary_pls_parameter_projections_with_technical_v1(
                config,
                authority.point_model(),
                authority.plan(),
                authority.technical_construct_ids(),
            )?
            .into_iter()
            .map(|projection| projection.identity)
            .collect())
        }
        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
        | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
        | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation => {
            let authority =
                projected_interaction_mga_authority_v1(dataset, recipe, model, artifact)?;
            let mut rows = design
                .rows
                .iter()
                .map(|row| row.source_row)
                .collect::<Vec<_>>();
            rows.sort_unstable();
            rows.dedup();
            if rows.len() != design.rows.len() {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "interaction MGA target resolution found duplicate source rows".into(),
                ));
            }
            let pooled_stage_one = pooled_ordinary_pls_fit_v1(
                dataset,
                &authority.source_columns,
                &rows,
                &authority.point_recipe,
                &authority.point_model,
                &authority.point_artifact,
                qpls_core::MgaModelProfileV1::GeneralSemPls,
                None,
                &|| false,
            )?;
            let pooled_joint = interaction_mga_joint_point_v1(
                &authority,
                &pooled_stage_one,
                config.profile,
                &|| false,
            )
            .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.detail))?;
            Ok(
                interaction_mga_parameter_projections_v1(config, &authority, &pooled_joint)?
                    .into_iter()
                    .map(|projection| projection.identity)
                    .collect(),
            )
        }
        qpls_core::MgaModelProfileV1::MultipleNonnestedHoc => {
            let authority = projected_hoc_mga_authority_v1(dataset, recipe, model, artifact)?;
            let final_stage = authority.stages.last().ok_or_else(|| {
                MultiModRunnerErrorV1::Authority("multiple-HOC authority has no final stage".into())
            })?;
            Ok(ordinary_pls_parameter_projections_v1(
                config,
                &authority.scientific_model,
                &final_stage.plan,
            )?
            .into_iter()
            .map(|projection| projection.identity)
            .collect())
        }
    }
}

/// Freezes the top-level raw MGA scientific task graph. The compiled
/// analytical identity binds the full recipe/model authority, the dataset
/// fingerprint binds indicator/group/weight values, and the design/target
/// digests bind selected rows, typed groups, and publishable parameters.
fn bind_mga_stable_row_tokens_v1(
    dataset: &Dataset,
    model: &SemModelV4,
    config: &MgaMultigroupV1,
    design: &MultigroupDesignV1,
) -> Result<MultigroupDesignV1, MultiModRunnerErrorV1> {
    let mut scientific_columns = model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed { source_column, .. } => Some(source_column.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    scientific_columns.insert(config.grouping_column.clone());
    if let Some(weight) = &config.weight {
        let column = match weight {
            qpls_core::AnalysisWeightBindingV1::Case { column }
            | qpls_core::AnalysisWeightBindingV1::Frequency { column } => column,
        };
        scientific_columns.insert(column.clone());
    }
    bind_mga_stable_row_tokens_for_columns_v1(
        dataset,
        &scientific_columns.into_iter().collect::<Vec<_>>(),
        design,
    )
}

fn bind_mga_stable_row_tokens_for_columns_v1(
    dataset: &Dataset,
    scientific_columns: &[String],
    design: &MultigroupDesignV1,
) -> Result<MultigroupDesignV1, MultiModRunnerErrorV1> {
    let row_count = dataset.batch.num_rows();
    let rows = qpls_data::preview_page(dataset, 0, row_count);
    if rows.len() != row_count {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "the resident dataset could not provide a complete MGA row-token frame".into(),
        ));
    }
    let mut scientific_columns = scientific_columns.to_vec();
    scientific_columns.sort();
    scientific_columns.dedup();
    if scientific_columns.is_empty() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "MGA stable-row binding requires at least one scientific input column".into(),
        ));
    }
    let schema_by_name = dataset
        .schema
        .columns
        .iter()
        .map(|column| (column.name.clone(), column.column_type))
        .collect::<BTreeMap<_, _>>();
    let schema_identity = scientific_columns
        .iter()
        .map(|column| {
            schema_by_name
                .get(column)
                .copied()
                .map(|column_type| (column.clone(), column_type))
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(format!(
                        "MGA scientific row-token column {column} is absent from the resident dataset"
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut physical_rows = BTreeSet::new();
    let mut ranked = Vec::with_capacity(design.rows.len());
    for selected in &design.rows {
        let physical = usize::try_from(selected.source_row).map_err(|_| {
            MultiModRunnerErrorV1::PreparedInput(
                "an MGA source-row address exceeds the platform row range".into(),
            )
        })?;
        let values = rows.get(physical).ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "MGA source row {} is outside the resident dataset",
                selected.source_row
            ))
        })?;
        if !physical_rows.insert(selected.source_row) {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "the MGA design contains a duplicate physical source-row address".into(),
            ));
        }
        let scientific_values = scientific_columns
            .iter()
            .map(|column| {
                values
                    .get(column)
                    .cloned()
                    .map(|value| (column.clone(), value))
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::PreparedInput(format!(
                            "MGA scientific row-token column {column} is absent from source row {}",
                            selected.source_row
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        ranked.push((
            selected.group,
            sha256_serialized(&(
                "qpls.mga.multigroup.stable-row-token.v1",
                &schema_identity,
                scientific_values,
            )),
            selected.source_row,
        ));
    }
    // Group-major ordering keeps the existing MICOM pair adapter compact.
    // Exact duplicate rows are scientifically exchangeable; physical address
    // is used only as their deterministic within-dataset tie-breaker.
    ranked.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
    });
    let tokens = ranked
        .into_iter()
        .enumerate()
        .map(|(token, (_, _, source_row))| (source_row, token as u64))
        .collect::<BTreeMap<_, _>>();
    let mut bound = design.clone();
    for row in &mut bound.rows {
        row.stable_row_token = *tokens.get(&row.source_row).ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(
                "the MGA stable-row binding omitted a selected physical row".into(),
            )
        })?;
    }
    Ok(bound)
}

pub fn prepare_compiled_raw_mga_execution_plan_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
) -> Result<MgaExecutionPlanV1, MultiModRunnerErrorV1> {
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    let design = bind_mga_stable_row_tokens_v1(dataset, model, config, design)?;
    let parameters = multimod_mga_publishable_parameter_identities_v1(
        dataset, recipe, model, artifact, &design,
    )?;
    let pairs = selected_mga_pairs(config)?;
    build_mga_execution_plan_v1(
        artifact.receipt(),
        &dataset.fingerprint.0,
        config,
        &design,
        &parameters,
        &pairs,
    )
    .map_err(map_mga_execution_cache_error_v1)
}

/// Top-level raw-data MGA dispatcher with an external, serializable execution
/// cache. Only completed immutable shard payloads are committed. Cancellation
/// leaves the cache resumable and cannot return a partial scientific result.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_raw_mga_resumable_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    cache: &mut MgaExecutionCacheV1,
    should_cancel: C,
    progress: P,
) -> Result<ResumableMgaRunV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_raw_mga_resumable_with_checkpoint_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        excluded_rows,
        cache,
        should_cancel,
        progress,
        |_, _| Ok(()),
    )
}

/// Raw-data resumable MGA with an app-owned durable-cache checkpoint. The
/// callback runs sequentially after each new immutable shard is inserted and
/// fully validated; it is not called for cache hits. A callback error aborts
/// before result publication while retaining the valid in-memory shard.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_raw_mga_resumable_with_checkpoint_v1<C, P, Q>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    cache: &mut MgaExecutionCacheV1,
    should_cancel: C,
    progress: P,
    mut checkpoint: Q,
) -> Result<ResumableMgaRunV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
    Q: FnMut(&MgaExecutionPlanV1, &MgaExecutionCacheV1) -> Result<(), String>,
{
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    let design = bind_mga_stable_row_tokens_v1(dataset, model, config, design)?;
    let execution_plan =
        prepare_compiled_raw_mga_execution_plan_v1(dataset, recipe, model, artifact, &design)?;
    let mut execution_session = ValidatedMgaExecutionCacheSessionV1::open(&execution_plan, cache)
        .map_err(map_mga_execution_cache_error_v1)?;
    let output = match config.profile {
        qpls_core::MgaModelProfileV1::GeneralSemPls
        | qpls_core::MgaModelProfileV1::ReflectivePlsc
        | qpls_core::MgaModelProfileV1::CaseWeightedPls => {
            run_compiled_ordinary_pls_mga_internal_v1(
                dataset,
                recipe,
                model,
                artifact,
                &design,
                excluded_rows,
                Some(&mut execution_session),
                Some(&mut checkpoint),
                &should_cancel,
                &progress,
            )
        }
        qpls_core::MgaModelProfileV1::FrequencyWeightedPls => {
            run_compiled_frequency_weighted_pls_mga_internal_v1(
                dataset,
                recipe,
                model,
                artifact,
                &design,
                excluded_rows,
                Some(&mut execution_session),
                Some(&mut checkpoint),
                &should_cancel,
                &progress,
            )
        }
        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
        | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
        | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation => {
            run_compiled_interaction_pls_mga_internal_v1(
                dataset,
                recipe,
                model,
                artifact,
                &design,
                excluded_rows,
                Some(&mut execution_session),
                Some(&mut checkpoint),
                &should_cancel,
                &progress,
            )
        }
        qpls_core::MgaModelProfileV1::MultipleNonnestedHoc => run_compiled_hoc_pls_mga_internal_v1(
            dataset,
            recipe,
            model,
            artifact,
            &design,
            excluded_rows,
            Some(&mut execution_session),
            Some(&mut checkpoint),
            &should_cancel,
            &progress,
        ),
    }?;
    let finalized_cache_sha256 = execution_session
        .finalized_identity_sha256()
        .map_err(map_mga_execution_cache_error_v1)?;
    drop(execution_session);
    Ok(ResumableMgaRunV1 {
        output,
        execution_plan,
        finalized_cache_sha256,
    })
}

pub(crate) fn report(
    progress: &(impl Fn(MultiModRunnerProgressV1) + Sync),
    phase: MultiModRunnerPhaseV1,
    completed_units: u64,
    total_units: u64,
    shard_id: impl Into<String>,
) {
    progress(MultiModRunnerProgressV1 {
        phase,
        completed_units,
        total_units,
        shard_id: shard_id.into(),
    });
}

pub(crate) fn validate_authority(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    target: MultiModCompilerTargetV1,
) -> Result<(), MultiModRunnerErrorV1> {
    if dataset.fingerprint.0 != artifact.receipt().dataset_fingerprint
        || recipe.dataset_fingerprint != dataset.fingerprint.0
    {
        return Err(MultiModRunnerErrorV1::Authority(
            "dataset fingerprint differs from the compiled receipt".into(),
        ));
    }
    if artifact.receipt().target != target {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiler target differs from the runtime adapter".into(),
        ));
    }
    validate_compiled_multimod_recipe_v1(artifact, recipe, model)
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))
}

pub fn prepare_multimod_recipe_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    target: MultiModCompilerTargetV1,
) -> Result<CompiledMultiModRecipeV1, MultiModRunnerErrorV1> {
    if recipe.dataset_fingerprint != dataset.fingerprint.0 {
        return Err(MultiModRunnerErrorV1::Authority(
            "recipe dataset fingerprint differs from the execution dataset".into(),
        ));
    }
    compile_multimod_recipe_v1(recipe, model, target)
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))
}

pub(crate) fn provenance(
    receipt: &MultiModCompilationReceiptV1,
    seed: u64,
) -> MultimodProvenanceV1 {
    MultimodProvenanceV1 {
        method_version: receipt.method_version.clone(),
        recipe_id: receipt.recipe_id.clone(),
        recipe_analytical_sha256: receipt.recipe_analytical_sha256.clone(),
        config_sha256: receipt.config_sha256.clone(),
        model_id: receipt.model_id.clone(),
        model_scientific_sha256: receipt.model_scientific_sha256.clone(),
        dataset_id: receipt.dataset_id.clone(),
        dataset_fingerprint: receipt.dataset_fingerprint.clone(),
        engine_version: MULTIMOD_RUNNER_METHOD_VERSION_V1.into(),
        seed,
        capability_cell: receipt.capability_cell.clone(),
        qualification: MultimodQualificationStateV1::UnqualifiedLabs,
        candidate_qualification_receipt: None,
    }
}

fn interval(
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
        || lower.zip(upper).is_some_and(|(lower, upper)| lower > upper)
    {
        return Err(MultiModRunnerErrorV1::ResultContract(
            "interval bound presence does not match its alternative, or a bound is nonfinite/reversed"
                .into(),
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

fn core_alternative(value: InferenceAlternativeV1) -> AlternativeHypothesisV1 {
    match value {
        InferenceAlternativeV1::TwoSided => AlternativeHypothesisV1::TwoSided,
        InferenceAlternativeV1::Less => AlternativeHypothesisV1::Less,
        InferenceAlternativeV1::Greater => AlternativeHypothesisV1::Greater,
    }
}

fn conditional_alternative(value: InferenceAlternativeV1) -> ConditionalAlternativeV2 {
    match value {
        InferenceAlternativeV1::TwoSided => ConditionalAlternativeV2::TwoSided,
        InferenceAlternativeV1::Less => ConditionalAlternativeV2::Less,
        InferenceAlternativeV1::Greater => ConditionalAlternativeV2::Greater,
    }
}

fn multiplicity_method(value: MultiplicityAdjustmentV1) -> qpls_estimation::MultiplicityMethodV1 {
    match value {
        MultiplicityAdjustmentV1::Holm => qpls_estimation::MultiplicityMethodV1::Holm,
        MultiplicityAdjustmentV1::Bonferroni => qpls_estimation::MultiplicityMethodV1::Bonferroni,
        MultiplicityAdjustmentV1::Sidak => qpls_estimation::MultiplicityMethodV1::Sidak,
        MultiplicityAdjustmentV1::BenjaminiHochbergExploratory => {
            qpls_estimation::MultiplicityMethodV1::BenjaminiHochberg
        }
        MultiplicityAdjustmentV1::NoneExplicit => qpls_estimation::MultiplicityMethodV1::None,
    }
}

fn parameter_family_id(family: ParameterFamilyV1) -> &'static str {
    match family {
        ParameterFamilyV1::StructuralPath => "structural_path",
        ParameterFamilyV1::OuterLoading => "outer_loading",
        ParameterFamilyV1::OuterWeight => "outer_weight",
        ParameterFamilyV1::RSquared => "r_squared",
        ParameterFamilyV1::SpecificIndirect => "specific_indirect",
        ParameterFamilyV1::TotalIndirect => "total_indirect",
        ParameterFamilyV1::InteractionGamma => "interaction_gamma",
        ParameterFamilyV1::ThreeWayDelta => "three_way_delta",
        ParameterFamilyV1::SimpleSlope => "simple_slope",
        ParameterFamilyV1::Other => "other",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreparedReplicateStatusV1 {
    Usable,
    Failed {
        kind: MultimodReplicateFailureKindV1,
        stable_code: String,
        detail: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedReplicateEntryV1 {
    pub replicate_index: u32,
    pub seed: u64,
    pub status: PreparedReplicateStatusV1,
}

/// One no-retry, deterministic ledger shared by every target in a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedSharedReplicateLedgerV1 {
    pub master_seed: u64,
    pub domain: String,
    pub requested: u32,
    pub entries: Vec<PreparedReplicateEntryV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedTargetReplicatesV1 {
    pub target_id: String,
    /// Exactly one cell per requested replicate. Failed-ledger cells are None;
    /// usable-ledger cells are finite Some values.
    pub estimates: Vec<Option<f64>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delete_one_jackknife_estimates: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_standard_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outer_standard_errors: Vec<Option<f64>>,
}

fn stable_domain_hash(domain: &str) -> u64 {
    // FNV-1a has a frozen byte-level definition and is used only to split a
    // declared master seed into independent deterministic ledgers.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in domain.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn multimod_replicate_seed_v1(master_seed: u64, domain: &str, index: u32) -> u64 {
    let mut state = master_seed
        ^ stable_domain_hash(domain)
        ^ u64::from(index).wrapping_mul(0x9e3779b97f4a7c15);
    state = (state ^ (state >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    state = (state ^ (state >> 27)).wrapping_mul(0x94d049bb133111eb);
    state ^ (state >> 31)
}

/// Converts an outer additive MultiMod Recipe V4 into the internal ordinary
/// PLS point recipe used only to compile a lower-order scoring plan. The outer
/// compiled MultiMod artifact remains the scientific authority.
fn stage_internal_pls_point_recipe_v1(recipe: &mut AnalysisRecipeV4) {
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
}

fn validate_shared_ledger(
    ledger: &PreparedSharedReplicateLedgerV1,
) -> Result<Vec<usize>, MultiModRunnerErrorV1> {
    if ledger.requested == 0
        || ledger.domain.trim().is_empty()
        || ledger.entries.len() != ledger.requested as usize
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "ledger requires a nonempty domain and exactly requested entries".into(),
        ));
    }
    let mut usable = Vec::new();
    for (position, entry) in ledger.entries.iter().enumerate() {
        if entry.replicate_index as usize != position
            || entry.seed
                != multimod_replicate_seed_v1(
                    ledger.master_seed,
                    &ledger.domain,
                    entry.replicate_index,
                )
        {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "replicate indices or derived seeds are not canonical".into(),
            ));
        }
        match &entry.status {
            PreparedReplicateStatusV1::Usable => usable.push(position),
            PreparedReplicateStatusV1::Failed {
                stable_code,
                detail,
                ..
            } if stable_code.trim().is_empty() || detail.trim().is_empty() => {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "failed replicates require a stable code and detail".into(),
                ));
            }
            PreparedReplicateStatusV1::Failed { .. } => {}
        }
    }
    Ok(usable)
}

fn validate_target_replicates(
    ledger: &PreparedSharedReplicateLedgerV1,
    targets: &[PreparedTargetReplicatesV1],
    expected_target_ids: &BTreeSet<String>,
) -> Result<Vec<usize>, MultiModRunnerErrorV1> {
    let usable = validate_shared_ledger(ledger)?;
    let mut actual_ids = BTreeSet::new();
    for target in targets {
        if !actual_ids.insert(target.target_id.clone())
            || target.estimates.len() != ledger.requested as usize
        {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "target identities must be unique and have one estimate cell per replicate".into(),
            ));
        }
        for (index, value) in target.estimates.iter().enumerate() {
            let ledger_usable = matches!(
                &ledger.entries[index].status,
                PreparedReplicateStatusV1::Usable
            );
            if ledger_usable != value.is_some() || value.is_some_and(|value| !value.is_finite()) {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "all targets must use the exact same validity bitmap and finite values".into(),
                ));
            }
        }
    }
    if &actual_ids != expected_target_ids {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "prepared target inventory differs from the point-result inventory".into(),
        ));
    }
    Ok(usable)
}

fn shared_ledger_summary(
    ledger: &PreparedSharedReplicateLedgerV1,
    minimum_required: usize,
) -> MultimodReplicateLedgerSummaryV1 {
    let usable = ledger
        .entries
        .iter()
        .filter(|entry| matches!(&entry.status, PreparedReplicateStatusV1::Usable))
        .count();
    let mut failure_counts = BTreeMap::<String, u32>::new();
    let failures = ledger
        .entries
        .iter()
        .filter_map(|entry| match &entry.status {
            PreparedReplicateStatusV1::Usable => None,
            PreparedReplicateStatusV1::Failed {
                kind,
                stable_code,
                detail,
            } => {
                *failure_counts.entry(stable_code.clone()).or_default() += 1;
                Some(MultimodReplicateFailureV1 {
                    replicate_index: entry.replicate_index,
                    kind: kind.clone(),
                    stable_code: stable_code.clone(),
                    detail: detail.clone(),
                })
            }
        })
        .collect::<Vec<_>>();
    MultimodReplicateLedgerSummaryV1 {
        requested: ledger.requested,
        usable: usable as u32,
        minimum_required: minimum_required as u32,
        usable_fraction: usable as f64 / f64::from(ledger.requested),
        complete: ledger.entries.len() == ledger.requested as usize && usable >= minimum_required,
        ledger_sha256: sha256_serialized(ledger),
        failure_counts,
        failures,
    }
}

fn empirical_zero_probability(estimates: &[f64], alternative: InferenceAlternativeV1) -> f64 {
    let nonnegative = estimates.iter().filter(|value| **value >= 0.0).count();
    let nonpositive = estimates.iter().filter(|value| **value <= 0.0).count();
    let denominator = estimates.len() as f64 + 1.0;
    let greater = (nonpositive as f64 + 1.0) / denominator;
    let less = (nonnegative as f64 + 1.0) / denominator;
    match alternative {
        InferenceAlternativeV1::TwoSided => (2.0 * greater.min(less)).min(1.0),
        InferenceAlternativeV1::Less => less,
        InferenceAlternativeV1::Greater => greater,
    }
}

#[derive(Debug, Clone)]
struct OrdinaryPlsScoringBlockV1 {
    construct_id: String,
    indicators: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
enum OrdinaryPlsParameterSourceV1 {
    StructuralPath {
        source: String,
        target: String,
        role: StructuralRelationRoleV4,
    },
    OuterLoading {
        construct: String,
        source_column: String,
    },
    OuterWeight {
        construct: String,
        source_column: String,
    },
    RSquared {
        construct: String,
    },
}

#[derive(Debug, Clone)]
struct OrdinaryPlsParameterProjectionV1 {
    identity: ParameterIdentityV1,
    source: OrdinaryPlsParameterSourceV1,
    micom_required_constructs: BTreeSet<String>,
}

impl OrdinaryPlsParameterProjectionV1 {
    fn required_constructs(&self) -> BTreeSet<String> {
        self.micom_required_constructs.clone()
    }
}

const MGA_OBSERVED_CONTROL_LOWERING_MAX_V1: usize = 50;
const MGA_OBSERVED_CONTROL_LOWERING_IDENTITY_V1: &str =
    "qpls.mga.observed-control-fixed-unit-lowering.v1";

#[derive(Debug, Clone)]
struct MgaObservedControlLoweringIdsV1 {
    indicator_id: String,
    measurement_relation_id: String,
    measurement_parameter_id: String,
}

fn mga_observed_control_lowering_ids_v1(
    observed_id: &str,
    source_column: &str,
) -> MgaObservedControlLoweringIdsV1 {
    let digest = sha256_serialized(&(
        MGA_OBSERVED_CONTROL_LOWERING_IDENTITY_V1,
        observed_id,
        source_column,
    ));
    MgaObservedControlLoweringIdsV1 {
        indicator_id: format!("qpls_mga_observed_control_indicator_v1_{digest}"),
        measurement_relation_id: format!("qpls_mga_observed_control_relation_v1_{digest}"),
        measurement_parameter_id: format!("qpls_mga_observed_control_parameter_v1_{digest}"),
    }
}

#[derive(Debug, Clone)]
struct MgaObservedControlLoweringSpecV1 {
    observed_id: String,
    label: String,
    source_column: String,
    generated: MgaObservedControlLoweringIdsV1,
}

/// Lowers only the admitted observed-control shape into the fixed-score
/// representation already supported by the PLS V2 point engine. The source
/// SemModelV4 remains the outer scientific authority: its observed variable
/// id becomes the internal composite id, its source column moves unchanged to
/// one technical indicator, and every authored control relation/parameter id
/// remains untouched.
fn lower_observed_controls_for_mga_pls_v1(
    point_model: &mut SemModelV4,
    profile: qpls_core::MgaModelProfileV1,
    grouping_variable_id: &str,
    grouping_column: &str,
) -> Result<BTreeSet<String>, MultiModRunnerErrorV1> {
    let mut observed = point_model
        .variables
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                label,
                source_column,
                scale,
                role,
                missing_markers,
                transformation_lineage,
                ..
            } => Some((
                id.clone(),
                label.clone(),
                source_column.clone(),
                *scale,
                role.clone(),
                missing_markers.is_empty(),
                transformation_lineage.is_empty(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    observed.sort_by(|left, right| left.0.cmp(&right.0));

    let mut lowering = Vec::new();
    for (
        observed_id,
        label,
        source_column,
        scale,
        role,
        missing_markers_empty,
        transformation_lineage_empty,
    ) in observed
    {
        let structural_sources = point_model
            .relations
            .iter()
            .filter_map(|relation| match relation {
                SemRelationV4::Structural {
                    source,
                    target,
                    role,
                    ..
                } if source == &observed_id => Some((target.as_str(), *role)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let used_as_structural_target = point_model.relations.iter().any(|relation| {
            matches!(relation, SemRelationV4::Structural { target, .. } if target == &observed_id)
        });
        if used_as_structural_target {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.observed_control_target_unsupported:{observed_id}"
            )));
        }
        if structural_sources.is_empty() {
            continue;
        }
        if role != ObservedRoleV4::Control
            || structural_sources
                .iter()
                .any(|(_, relation_role)| *relation_role != StructuralRelationRoleV4::Control)
        {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.observed_control_role_or_relation_unsupported:{observed_id}"
            )));
        }
        if scale != ObservedScaleV4::Continuous
            || !missing_markers_empty
            || !transformation_lineage_empty
        {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.observed_control_scale_or_metadata_unsupported:{observed_id}"
            )));
        }
        if structural_sources.iter().any(|(target, _)| {
            !point_model.variables.iter().any(|variable| {
                matches!(variable, SemVariableV4::Composite { id, .. } if id.as_str() == *target)
            })
        }) {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.observed_control_target_kind_unsupported:{observed_id}"
            )));
        }
        let has_mixed_relation_use = point_model.relations.iter().any(|relation| match relation {
            SemRelationV4::Structural { source, role, .. } if source == &observed_id => {
                *role != StructuralRelationRoleV4::Control
            }
            SemRelationV4::Structural { target, .. } => target == &observed_id,
            SemRelationV4::MeasurementEffect {
                construct,
                indicator,
                ..
            } => construct == &observed_id || indicator == &observed_id,
            SemRelationV4::MeasurementCausal {
                indicator,
                composite,
                ..
            } => indicator == &observed_id || composite == &observed_id,
            SemRelationV4::Covariance { left, right, .. } => {
                left.variable_id() == observed_id.as_str()
                    || right.variable_id() == observed_id.as_str()
            }
        });
        let has_derived_use = point_model.derived_terms.iter().any(|term| match term {
            qpls_core::SemDerivedTermV4::Interaction {
                output,
                predictor,
                moderator,
                ..
            } => [output, predictor, moderator]
                .iter()
                .any(|candidate| *candidate == &observed_id),
            qpls_core::SemDerivedTermV4::InteractionV2 {
                output, operands, ..
            } => output == &observed_id || operands.iter().any(|value| value == &observed_id),
            qpls_core::SemDerivedTermV4::HigherOrder {
                output, components, ..
            } => output == &observed_id || components.iter().any(|value| value == &observed_id),
            qpls_core::SemDerivedTermV4::Polynomial { output, source, .. } => {
                output == &observed_id || source == &observed_id
            }
        });
        if has_mixed_relation_use || has_derived_use {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.observed_control_mixed_use_unsupported:{observed_id}"
            )));
        }
        if source_column == grouping_column
            || point_model
            .variables
            .iter()
            .filter(|variable| {
                matches!(variable, SemVariableV4::Observed { source_column: candidate, .. } if candidate == &source_column)
            })
            .count()
            != 1
        {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.observed_control_source_collision:{observed_id}"
            )));
        }
        lowering.push(MgaObservedControlLoweringSpecV1 {
            generated: mga_observed_control_lowering_ids_v1(&observed_id, &source_column),
            observed_id,
            label,
            source_column,
        });
    }

    if lowering.is_empty() {
        return Ok(BTreeSet::new());
    }
    if profile != qpls_core::MgaModelProfileV1::GeneralSemPls {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.observed_control_general_sem_pls_only".into(),
        ));
    }
    if lowering.len() > MGA_OBSERVED_CONTROL_LOWERING_MAX_V1 {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.mga.observed_control_count_unsupported:{}>{MGA_OBSERVED_CONTROL_LOWERING_MAX_V1}",
            lowering.len()
        )));
    }

    let mut occupied = point_model
        .variables
        .iter()
        .map(|value| value.id().to_owned())
        .chain(
            point_model
                .relations
                .iter()
                .map(|value| value.id().to_owned()),
        )
        .chain(
            point_model
                .parameters
                .iter()
                .map(|value| value.id().to_owned()),
        )
        .chain(
            point_model
                .constraints
                .iter()
                .map(|value| value.id().to_owned()),
        )
        .chain(
            point_model
                .derived_terms
                .iter()
                .map(|value| value.id().to_owned()),
        )
        .chain(
            point_model
                .annotations
                .iter()
                .map(|value| value.id().to_owned()),
        )
        .chain(
            point_model
                .variables
                .iter()
                .filter_map(|variable| match variable {
                    SemVariableV4::Observed { source_column, .. } => Some(source_column.clone()),
                    _ => None,
                }),
        )
        .collect::<BTreeSet<_>>();
    occupied.insert(grouping_variable_id.to_owned());
    occupied.insert(grouping_column.to_owned());
    for spec in &lowering {
        for identity in [
            &spec.generated.indicator_id,
            &spec.generated.measurement_relation_id,
            &spec.generated.measurement_parameter_id,
        ] {
            if !occupied.insert(identity.clone()) {
                return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.mga.observed_control_generated_identity_collision:{identity}"
                )));
            }
        }
    }

    let technical_construct_ids = lowering
        .iter()
        .map(|spec| spec.observed_id.clone())
        .collect::<BTreeSet<_>>();
    for spec in lowering {
        let variable = point_model
            .variables
            .iter_mut()
            .find(|variable| variable.id() == spec.observed_id)
            .expect("lowering specifications come from the point model");
        *variable = SemVariableV4::Composite {
            id: spec.observed_id.clone(),
            label: spec.label.clone(),
            weighting: CompositeWeightingV4::Unit {
                normalization: CompositeWeightNormalizationV4::None,
            },
        };
        point_model.variables.push(SemVariableV4::Observed {
            id: spec.generated.indicator_id.clone(),
            label: format!("Internal MGA observed-control input: {}", spec.label),
            source_column: spec.source_column,
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Indicator,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        point_model
            .relations
            .push(SemRelationV4::MeasurementCausal {
                id: spec.generated.measurement_relation_id,
                indicator: spec.generated.indicator_id.clone(),
                composite: spec.observed_id.clone(),
                parameter: spec.generated.measurement_parameter_id.clone(),
            });
        point_model.parameters.push(SemParameterV4::Free {
            id: spec.generated.measurement_parameter_id,
            label: format!("Internal fixed unit score for {}", spec.label),
            target: SemParameterTargetV4::Weight {
                indicator: spec.generated.indicator_id,
                composite: spec.observed_id,
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
    }
    *point_model = point_model.canonicalized();
    Ok(technical_construct_ids)
}

#[derive(Debug, Clone)]
enum OrdinaryPlsPointExecutionAuthorityV1 {
    Standard {
        point_recipe: AnalysisRecipeV4,
        point_model: SemModelV4,
        point_artifact: qpls_core::CompiledAnalysisRecipeV4,
    },
    ReflectivePlsc {
        point_recipe: AnalysisRecipeV4,
        point_model: SemModelV4,
        point_artifact: qpls_core::CompiledAnalysisRecipeV4,
        execution: ValidatedExecutionRecipe,
    },
    Weighted {
        prepared: PreparedMultimodWeightedPlsPointV1,
        weight_source_column: String,
    },
}

#[derive(Debug, Clone)]
struct OrdinaryPlsPointAuthorityV1 {
    execution: OrdinaryPlsPointExecutionAuthorityV1,
    technical_observed_control_construct_ids: BTreeSet<String>,
}

impl OrdinaryPlsPointAuthorityV1 {
    fn point_recipe(&self) -> &AnalysisRecipeV4 {
        match &self.execution {
            OrdinaryPlsPointExecutionAuthorityV1::Standard { point_recipe, .. }
            | OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { point_recipe, .. } => {
                point_recipe
            }
            OrdinaryPlsPointExecutionAuthorityV1::Weighted { prepared, .. } => {
                prepared.point_recipe()
            }
        }
    }

    fn point_model(&self) -> &SemModelV4 {
        match &self.execution {
            OrdinaryPlsPointExecutionAuthorityV1::Standard { point_model, .. }
            | OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { point_model, .. } => {
                point_model
            }
            OrdinaryPlsPointExecutionAuthorityV1::Weighted { prepared, .. } => {
                prepared.point_model()
            }
        }
    }

    fn plan(&self) -> &CompiledPlsPlanV2 {
        match &self.execution {
            OrdinaryPlsPointExecutionAuthorityV1::Weighted { prepared, .. } => prepared.plan(),
            OrdinaryPlsPointExecutionAuthorityV1::Standard { point_artifact, .. }
            | OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { point_artifact, .. } => {
                let CompiledRecipePlanV4::PlsPlanV2 { plan } = point_artifact.plan() else {
                    unreachable!("ordinary MGA point authority always embeds a PLS v2 plan")
                };
                plan
            }
        }
    }

    fn technical_construct_ids(&self) -> &BTreeSet<String> {
        &self.technical_observed_control_construct_ids
    }

    fn weight_source_column(&self) -> Option<&str> {
        match &self.execution {
            OrdinaryPlsPointExecutionAuthorityV1::Weighted {
                weight_source_column,
                ..
            } => Some(weight_source_column),
            OrdinaryPlsPointExecutionAuthorityV1::Standard { .. }
            | OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { .. } => None,
        }
    }

    fn repeats_plsc_correction(&self) -> bool {
        matches!(
            &self.execution,
            OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { .. }
        )
    }

    fn execute<C>(&self, dataset: &Dataset, should_cancel: &C) -> Result<PlsResult, RefitFailureV1>
    where
        C: Fn() -> bool + Sync,
    {
        match &self.execution {
            OrdinaryPlsPointExecutionAuthorityV1::Standard {
                point_recipe,
                point_model,
                point_artifact,
            } => run_compiled_pls_recipe_v4(
                dataset,
                point_recipe,
                point_model,
                point_artifact,
                None,
                || should_cancel(),
                |_| {},
            )
            .map(|execution| execution.estimation().clone())
            .map_err(refit_execution_failure_v1),
            OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { execution, .. } => {
                run_prepared_multimod_reflective_plsc_point_v1(
                    dataset,
                    self.point_recipe(),
                    self.plan(),
                    execution,
                    should_cancel,
                )
            }
            OrdinaryPlsPointExecutionAuthorityV1::Weighted { prepared, .. } => {
                run_prepared_multimod_weighted_pls_point_v1(
                    dataset,
                    prepared,
                    || should_cancel(),
                    |_| {},
                )
                .map(|execution| execution.estimation)
                .map_err(weighted_point_refit_failure_v1)
            }
        }
    }
}

fn prepare_multimod_reflective_plsc_execution_v1(
    point_recipe: &AnalysisRecipeV4,
    point_model: &SemModelV4,
    point_artifact: &qpls_core::CompiledAnalysisRecipeV4,
) -> Result<ValidatedExecutionRecipe, MultiModRunnerErrorV1> {
    validate_compiled_analysis_recipe_v4(point_artifact, point_recipe, Some(point_model)).map_err(
        |error| {
            MultiModRunnerErrorV1::Authority(format!(
                "multimod.runner.mga.plsc_point_authority_invalid: {error}"
            ))
        },
    )?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = point_artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "multimod.runner.mga.plsc_point_plan_missing".into(),
        ));
    };
    if plan.blocks().iter().any(|block| {
        block.mode() != CompiledPlsBlockModeV2::ModeA
            || block.indicators().len() < 2
            || block.fixed_scoring().is_some()
    }) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.plsc_reflective_plan_required: PLSc MGA requires estimated Mode-A blocks with at least two indicators"
                .into(),
        ));
    }
    let mut projected = crate::recipe_v4_pls_execution::project_pls_plan_to_current_recipe(
        point_recipe,
        point_model,
        plan,
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::Authority(format!(
            "multimod.runner.mga.plsc_point_projection_failed: {error}"
        ))
    })?;
    projected.settings.method = AnalysisMethod::Plsc;
    projected.settings.case_weight_column = None;
    projected.method_config = Some(MethodConfig::Plsc);
    ValidatedExecutionRecipe::for_dataset(&projected, &point_recipe.dataset_fingerprint).map_err(
        |error| {
            MultiModRunnerErrorV1::Authority(format!(
                "multimod.runner.mga.plsc_execution_recipe_invalid: {error}"
            ))
        },
    )
}

fn projected_ordinary_pls_authority_v1(
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    config: &MgaMultigroupV1,
) -> Result<OrdinaryPlsPointAuthorityV1, MultiModRunnerErrorV1> {
    if !matches!(
        config.profile,
        qpls_core::MgaModelProfileV1::GeneralSemPls
            | qpls_core::MgaModelProfileV1::ReflectivePlsc
            | qpls_core::MgaModelProfileV1::CaseWeightedPls
            | qpls_core::MgaModelProfileV1::FrequencyWeightedPls
    ) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.base_pls_profile_required: the built-in base refitter executes general_sem_pls, reflective_plsc, case_weighted_pls, or frequency_weighted_pls"
                .into(),
        ));
    }
    let grouping_variable = match &model.group {
        SemGroupV4::ObservedGroups {
            grouping_variable, ..
        } => grouping_variable.clone(),
        SemGroupV4::SingleGroup => {
            return Err(MultiModRunnerErrorV1::Authority(
                "ordinary multigroup PLS requires an observed-group SemModelV4 authority".into(),
            ));
        }
    };
    let grouping_source = model
        .variables
        .iter()
        .find_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } if id == &grouping_variable => Some(source_column.as_str()),
            _ => None,
        })
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "the SemModelV4 grouping variable is not a directly observed column".into(),
            )
        })?;
    if grouping_source != config.grouping_column {
        return Err(MultiModRunnerErrorV1::Authority(
            "the MGA grouping column differs from the SemModelV4 group binding".into(),
        ));
    }

    // The outer MultiMod artifact remains authoritative. This internal point
    // projection removes only the non-indicator grouping variable and its
    // group declaration so the existing single-group Recipe V4 PLS compiler
    // can prove every row-specific refit.
    let mut point_model = model.clone();
    point_model.group = SemGroupV4::SingleGroup;
    point_model
        .variables
        .retain(|variable| variable.id() != grouping_variable);
    let technical_observed_control_construct_ids = lower_observed_controls_for_mga_pls_v1(
        &mut point_model,
        config.profile,
        &grouping_variable,
        &config.grouping_column,
    )?;
    point_model.annotations.clear();
    point_model.presentation = Default::default();
    point_model
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    let scientific_sha256 = point_model
        .scientific_sha256()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;

    let mut point_recipe = recipe.clone();
    let (method, method_config, case_weight_column) = match config.profile {
        qpls_core::MgaModelProfileV1::GeneralSemPls => {
            (AnalysisMethod::PlsPm, MethodConfig::PlsAlgorithm, None)
        }
        // PLSc uses an ordinary compiled score/path plan. The MultiMod-only
        // execution adapter below switches only the projected legacy recipe
        // to PLSc and re-applies the correction in every sampled fit.
        qpls_core::MgaModelProfileV1::ReflectivePlsc => {
            (AnalysisMethod::PlsPm, MethodConfig::PlsAlgorithm, None)
        }
        qpls_core::MgaModelProfileV1::CaseWeightedPls => {
            let Some(qpls_core::AnalysisWeightBindingV1::Case { column }) = &config.weight else {
                return Err(MultiModRunnerErrorV1::Authority(
                    "case-weighted MGA lost its typed case-weight binding".into(),
                ));
            };
            (
                AnalysisMethod::Wpls,
                MethodConfig::Wpls,
                Some(column.clone()),
            )
        }
        qpls_core::MgaModelProfileV1::FrequencyWeightedPls => {
            let Some(qpls_core::AnalysisWeightBindingV1::Frequency { column }) = &config.weight
            else {
                return Err(MultiModRunnerErrorV1::Authority(
                    "frequency-weighted MGA lost its typed frequency binding".into(),
                ));
            };
            (
                AnalysisMethod::Wpls,
                MethodConfig::Wpls,
                Some(column.clone()),
            )
        }
        _ => unreachable!("profile admission checked above"),
    };
    point_recipe.settings.method = method;
    point_recipe.settings.bootstrap_samples = 0;
    point_recipe.settings.permutation_samples = 0;
    point_recipe.settings.studentized_inner_samples = 0;
    point_recipe.settings.case_weight_column = case_weight_column;
    point_recipe.method_config = Some(method_config);
    point_recipe.mga_multigroup = None;
    point_recipe.model_binding = match &recipe.model_binding {
        AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 { .. } => {
            AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: point_model.clone(),
                scientific_sha256,
            }
        }
        AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference { model_id, .. } => {
            AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: model_id.clone(),
                scientific_sha256,
            }
        }
        AnalysisRecipeModelBindingV4::LegacyEstimandUnspecified { .. } => {
            return Err(MultiModRunnerErrorV1::Authority(
                "MultiMod ordinary PLS cannot project an unresolved legacy estimand".into(),
            ));
        }
    };
    let execution = match config.profile {
        qpls_core::MgaModelProfileV1::CaseWeightedPls
        | qpls_core::MgaModelProfileV1::FrequencyWeightedPls => {
            let requested_weight = config.weight.as_ref().ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(
                    "weighted MGA lost its typed weight binding".into(),
                )
            })?;
            let authority = compile_multimod_weighted_pls_recipe_v4_v1(
                &point_recipe,
                &point_model,
                requested_weight,
            )
            .map_err(|error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.mga.weighted_pls_projection_rejected: {error}"
                ))
            })?;
            let expected_weight_semantics = match config.profile {
                qpls_core::MgaModelProfileV1::CaseWeightedPls => {
                    qpls_core::MultimodCompiledWeightSemanticsV1::PositiveCase
                }
                qpls_core::MgaModelProfileV1::FrequencyWeightedPls => {
                    qpls_core::MultimodCompiledWeightSemanticsV1::PositiveIntegerFrequencyCountSpace
                }
                _ => unreachable!("weighted authority branch is profile-guarded"),
            };
            if authority.receipt().weight_semantics() != expected_weight_semantics {
                return Err(MultiModRunnerErrorV1::Authority(
                    "multimod.runner.mga.weighted_point_semantics_mismatch".into(),
                ));
            }
            let weight_source_column = authority.receipt().weight_source_column().to_owned();
            let prepared = prepare_compiled_multimod_weighted_pls_point_v1(
                &point_recipe,
                &point_model,
                requested_weight,
                &authority,
            )
            .map_err(|error| {
                MultiModRunnerErrorV1::Authority(format!(
                    "multimod.runner.mga.weighted_point_authority_invalid: {error}"
                ))
            })?;
            OrdinaryPlsPointExecutionAuthorityV1::Weighted {
                prepared,
                weight_source_column,
            }
        }
        qpls_core::MgaModelProfileV1::GeneralSemPls
        | qpls_core::MgaModelProfileV1::ReflectivePlsc => {
            let target = RecipeV4CompilerTarget::PlsPlanV2;
            let point_artifact = compile_analysis_recipe_v4(
                &point_recipe,
                Some(&point_model),
                target,
                target.capability_cell_for_recipe(&point_recipe),
            )
            .map_err(|error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.mga.ordinary_pls_projection_rejected: {error}"
                ))
            })?;
            if config.profile == qpls_core::MgaModelProfileV1::ReflectivePlsc {
                let execution = prepare_multimod_reflective_plsc_execution_v1(
                    &point_recipe,
                    &point_model,
                    &point_artifact,
                )?;
                OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc {
                    point_recipe,
                    point_model,
                    point_artifact,
                    execution,
                }
            } else {
                OrdinaryPlsPointExecutionAuthorityV1::Standard {
                    point_recipe,
                    point_model,
                    point_artifact,
                }
            }
        }
        _ => unreachable!("profile admission checked above"),
    };
    Ok(OrdinaryPlsPointAuthorityV1 {
        execution,
        technical_observed_control_construct_ids,
    })
}

fn ordinary_pls_scoring_blocks_v1(plan: &CompiledPlsPlanV2) -> Vec<OrdinaryPlsScoringBlockV1> {
    plan.blocks()
        .iter()
        .map(|block| OrdinaryPlsScoringBlockV1 {
            construct_id: block.construct_id().into(),
            indicators: block
                .indicators()
                .iter()
                .map(|indicator| {
                    (
                        indicator.variable_id().into(),
                        indicator.source_column().into(),
                    )
                })
                .collect(),
        })
        .collect()
}

fn ordinary_pls_micom_construct_ids_v1(
    blocks: &[OrdinaryPlsScoringBlockV1],
    technical_construct_ids: &BTreeSet<String>,
) -> Vec<String> {
    blocks
        .iter()
        .filter(|block| !technical_construct_ids.contains(&block.construct_id))
        .map(|block| block.construct_id.clone())
        .collect()
}

fn synthetic_ordinary_pls_parameter_projections_v1(
    plan: &CompiledPlsPlanV2,
    technical_construct_ids: &BTreeSet<String>,
) -> Vec<OrdinaryPlsParameterProjectionV1> {
    let mut projections = Vec::new();
    for path in plan.paths() {
        let micom_required_constructs = [path.source(), path.target()]
            .into_iter()
            .filter(|construct| !technical_construct_ids.contains(*construct))
            .map(str::to_owned)
            .collect();
        projections.push(OrdinaryPlsParameterProjectionV1 {
            identity: ParameterIdentityV1 {
                stable_id: path.parameter_id().into(),
                family: ParameterFamilyV1::StructuralPath,
            },
            source: OrdinaryPlsParameterSourceV1::StructuralPath {
                source: path.source().into(),
                target: path.target().into(),
                role: path.role(),
            },
            micom_required_constructs,
        });
    }
    let mut endogenous = BTreeSet::new();
    for path in plan.paths() {
        endogenous.insert(path.target().to_owned());
    }
    for block in plan.blocks() {
        if technical_construct_ids.contains(block.construct_id()) {
            continue;
        }
        for indicator in block.indicators() {
            for (prefix, family) in [
                ("outer_loading", ParameterFamilyV1::OuterLoading),
                ("outer_weight", ParameterFamilyV1::OuterWeight),
            ] {
                let source = match family {
                    ParameterFamilyV1::OuterLoading => OrdinaryPlsParameterSourceV1::OuterLoading {
                        construct: block.construct_id().into(),
                        source_column: indicator.source_column().into(),
                    },
                    ParameterFamilyV1::OuterWeight => OrdinaryPlsParameterSourceV1::OuterWeight {
                        construct: block.construct_id().into(),
                        source_column: indicator.source_column().into(),
                    },
                    _ => unreachable!(),
                };
                projections.push(OrdinaryPlsParameterProjectionV1 {
                    identity: ParameterIdentityV1 {
                        stable_id: format!(
                            "{prefix}:{}:{}",
                            block.construct_id(),
                            indicator.variable_id()
                        ),
                        family,
                    },
                    source,
                    micom_required_constructs: BTreeSet::from([block.construct_id().to_owned()]),
                });
            }
        }
    }
    for construct in endogenous {
        if technical_construct_ids.contains(&construct) {
            continue;
        }
        projections.push(OrdinaryPlsParameterProjectionV1 {
            identity: ParameterIdentityV1 {
                stable_id: format!("r_squared:{construct}"),
                family: ParameterFamilyV1::RSquared,
            },
            source: OrdinaryPlsParameterSourceV1::RSquared {
                construct: construct.clone(),
            },
            micom_required_constructs: BTreeSet::from([construct]),
        });
    }
    projections.sort_by(|left, right| left.identity.stable_id.cmp(&right.identity.stable_id));
    projections
}

fn explicit_ordinary_pls_parameter_projection_v1(
    parameter_id: &str,
    model: &SemModelV4,
    plan: &CompiledPlsPlanV2,
    synthetic: &[OrdinaryPlsParameterProjectionV1],
    technical_construct_ids: &BTreeSet<String>,
) -> Result<OrdinaryPlsParameterProjectionV1, MultiModRunnerErrorV1> {
    if let Some(candidate) = synthetic
        .iter()
        .find(|candidate| candidate.identity.stable_id == parameter_id)
    {
        return Ok(candidate.clone());
    }
    let parameter = model
        .parameters
        .iter()
        .find(|parameter| parameter.id() == parameter_id)
        .ok_or_else(|| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.parameter_unmapped: selected target {parameter_id} is neither an ordinary PLS parameter nor a supported synthetic loading/weight/R-squared identity"
            ))
        })?;
    match parameter.target() {
        SemParameterTargetV4::Regression { .. } => {
            let path = plan
                .paths()
                .iter()
                .find(|path| path.parameter_id() == parameter_id)
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::UnsupportedProfile(format!(
                        "multimod.runner.mga.parameter_unmapped: regression parameter {parameter_id} is absent from the compiled PLS plan"
                    ))
                })?;
            Ok(OrdinaryPlsParameterProjectionV1 {
                identity: ParameterIdentityV1 {
                    stable_id: parameter_id.into(),
                    family: ParameterFamilyV1::StructuralPath,
                },
                source: OrdinaryPlsParameterSourceV1::StructuralPath {
                    source: path.source().into(),
                    target: path.target().into(),
                    role: path.role(),
                },
                micom_required_constructs: [path.source(), path.target()]
                    .into_iter()
                    .filter(|construct| !technical_construct_ids.contains(*construct))
                    .map(str::to_owned)
                    .collect(),
            })
        }
        SemParameterTargetV4::Loading { .. } | SemParameterTargetV4::Weight { .. } => {
            let (block, indicator) = plan
                .blocks()
                .iter()
                .find_map(|block| {
                    block
                        .indicators()
                        .iter()
                        .find(|indicator| indicator.parameter_id() == parameter_id)
                        .map(|indicator| (block, indicator))
                })
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::UnsupportedProfile(format!(
                        "multimod.runner.mga.parameter_unmapped: measurement parameter {parameter_id} is absent from the compiled PLS plan"
                    ))
                })?;
            if technical_construct_ids.contains(block.construct_id()) {
                return Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.mga.technical_observed_control_measurement_target_unsupported:{parameter_id}"
                )));
            }
            let (family, source) = match parameter.target() {
                SemParameterTargetV4::Loading { .. } => (
                    ParameterFamilyV1::OuterLoading,
                    OrdinaryPlsParameterSourceV1::OuterLoading {
                        construct: block.construct_id().into(),
                        source_column: indicator.source_column().into(),
                    },
                ),
                SemParameterTargetV4::Weight { .. } => (
                    ParameterFamilyV1::OuterWeight,
                    OrdinaryPlsParameterSourceV1::OuterWeight {
                        construct: block.construct_id().into(),
                        source_column: indicator.source_column().into(),
                    },
                ),
                _ => unreachable!(),
            };
            Ok(OrdinaryPlsParameterProjectionV1 {
                identity: ParameterIdentityV1 {
                    stable_id: parameter_id.into(),
                    family,
                },
                source,
                micom_required_constructs: BTreeSet::from([block.construct_id().to_owned()]),
            })
        }
        _ => Err(MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.mga.parameter_family_unsupported: selected target {parameter_id} is not a path, loading, weight, or R-squared parameter"
        ))),
    }
}

fn ordinary_pls_parameter_projections_v1(
    config: &MgaMultigroupV1,
    model: &SemModelV4,
    plan: &CompiledPlsPlanV2,
) -> Result<Vec<OrdinaryPlsParameterProjectionV1>, MultiModRunnerErrorV1> {
    ordinary_pls_parameter_projections_with_technical_v1(config, model, plan, &BTreeSet::new())
}

fn ordinary_pls_parameter_projections_with_technical_v1(
    config: &MgaMultigroupV1,
    model: &SemModelV4,
    plan: &CompiledPlsPlanV2,
    technical_construct_ids: &BTreeSet<String>,
) -> Result<Vec<OrdinaryPlsParameterProjectionV1>, MultiModRunnerErrorV1> {
    let synthetic = synthetic_ordinary_pls_parameter_projections_v1(plan, technical_construct_ids);
    let projections = if config.selected_parameter_ids.is_empty() {
        synthetic
    } else {
        config
            .selected_parameter_ids
            .iter()
            .map(|parameter_id| {
                explicit_ordinary_pls_parameter_projection_v1(
                    parameter_id,
                    model,
                    plan,
                    &synthetic,
                    technical_construct_ids,
                )
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    if projections.is_empty()
        || projections
            .iter()
            .map(|projection| projection.identity.stable_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != projections.len()
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "ordinary PLS MGA parameter projections are empty or duplicated".into(),
        ));
    }
    Ok(projections)
}

fn ordinary_pls_source_columns_v1(
    dataset: &Dataset,
    blocks: &[OrdinaryPlsScoringBlockV1],
) -> Result<Vec<String>, MultiModRunnerErrorV1> {
    let required = blocks
        .iter()
        .flat_map(|block| block.indicators.iter().map(|(_, column)| column.as_str()))
        .collect::<BTreeSet<_>>();
    let columns = dataset
        .schema
        .columns
        .iter()
        .filter(|column| required.contains(column.name.as_str()))
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    if columns.len() != required.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "the execution dataset omits a compiled ordinary PLS indicator column".into(),
        ));
    }
    Ok(columns)
}

#[derive(Debug, Clone)]
struct OrdinaryPlsRawScoreCacheV1 {
    row_positions: BTreeMap<u64, usize>,
    values: BTreeMap<String, Vec<f64>>,
}

impl OrdinaryPlsRawScoreCacheV1 {
    fn build<C>(
        dataset: &Dataset,
        source_columns: &[String],
        source_rows: &[u64],
        should_cancel: &C,
    ) -> Result<Self, MultiModRunnerErrorV1>
    where
        C: Fn() -> bool + Sync,
    {
        let indices = checked_source_row_indices_v1(dataset, source_rows)
            .map_err(|failure| MultiModRunnerErrorV1::PreparedInput(failure.detail))?;
        let sampled =
            resample_dataset_columns_v1(dataset, source_columns, &indices, || should_cancel())
                .map_err(|error| match error {
                    EstimationError::Cancelled => MultiModRunnerErrorV1::Cancelled,
                    other => MultiModRunnerErrorV1::Kernel(other.to_string()),
                })?;
        let preview = qpls_data::preview(&sampled, source_rows.len());
        if preview.len() != source_rows.len() {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "ordinary PLS raw-score cache has the wrong row count".into(),
            ));
        }
        let mut values = BTreeMap::new();
        for column in source_columns {
            let column_values = preview
                .iter()
                .map(|row| {
                    row.get(column)
                        .and_then(|value| value.as_deref())
                        .and_then(|value| value.parse::<f64>().ok())
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| {
                            MultiModRunnerErrorV1::PreparedInput(format!(
                                "selected MGA row has a missing or nonnumeric value in {column}"
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            values.insert(column.clone(), column_values);
        }
        Ok(Self {
            row_positions: source_rows
                .iter()
                .enumerate()
                .map(|(position, row)| (*row, position))
                .collect(),
            values,
        })
    }

    fn composite_scores(
        &self,
        block: &OrdinaryPlsScoringBlockV1,
        result: &PlsResult,
        scoring_rows: &[u64],
    ) -> Result<Vec<f64>, RefitFailureV1> {
        let positions = scoring_rows
            .iter()
            .map(|row| {
                self.row_positions.get(row).copied().ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        "MICOM/orientation scoring row is outside the selected MGA row universe",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut scores = vec![0.0; positions.len()];
        for (_, source_column) in &block.indicators {
            let estimate = result
                .outer_estimates
                .iter()
                .find(|estimate| {
                    estimate.construct == block.construct_id && estimate.indicator == *source_column
                })
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!(
                            "ordinary PLS fit omitted outer weight {}:{}",
                            block.construct_id, source_column
                        ),
                    )
                })?;
            let transform = result
                .transforms
                .iter()
                .find(|transform| transform.indicator == *source_column)
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!("ordinary PLS fit omitted transform {source_column}"),
                    )
                })?;
            if !estimate.weight.is_finite()
                || !transform.scale.is_finite()
                || transform.scale.abs() <= f64::EPSILON
            {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::OrientationUndefined,
                    format!(
                        "ordinary PLS scoring rule is degenerate for {}:{}",
                        block.construct_id, source_column
                    ),
                ));
            }
            let raw_weight = estimate.weight / transform.scale;
            let values = self.values.get(source_column).ok_or_else(|| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    format!("raw-score cache omitted {source_column}"),
                )
            })?;
            for (score, position) in scores.iter_mut().zip(&positions) {
                *score += raw_weight * values[*position];
            }
        }
        if sample_standard_deviation_v1(&scores) <= f64::EPSILON {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::OrientationUndefined,
                format!(
                    "pooled scoring proxy has zero variance for {}",
                    block.construct_id
                ),
            ));
        }
        Ok(scores)
    }
}

fn checked_source_row_indices_v1(
    dataset: &Dataset,
    source_rows: &[u64],
) -> Result<Vec<usize>, RefitFailureV1> {
    source_rows
        .iter()
        .map(|row| {
            usize::try_from(*row)
                .ok()
                .filter(|row| *row < dataset.batch.num_rows())
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        "MGA refit row lies outside the execution dataset",
                    )
                })
        })
        .collect()
}

fn checked_source_rows_v1(
    dataset: &Dataset,
    source_rows: &[u64],
) -> Result<Vec<usize>, RefitFailureV1> {
    if source_rows.len() < 10 {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::InsufficientRows,
            "ordinary PLS MGA refits require at least ten rows",
        ));
    }
    checked_source_row_indices_v1(dataset, source_rows)
}

fn checked_frequency_source_rows_v1(
    dataset: &Dataset,
    source_rows: &[u64],
    counts: &[u64],
) -> Result<Vec<usize>, RefitFailureV1> {
    if source_rows.len() != counts.len()
        || source_rows.is_empty()
        || counts.iter().any(|count| *count == 0)
    {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::InsufficientRows,
            "frequency PLS refit requires aligned nonempty positive counts",
        ));
    }
    let represented = counts.iter().try_fold(0_u64, |total, count| {
        total.checked_add(*count).ok_or_else(|| {
            RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "frequency PLS represented-case count overflowed",
            )
        })
    })?;
    if represented < 10 {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::InsufficientRows,
            "frequency PLS MGA refits require at least ten represented cases",
        ));
    }
    checked_source_row_indices_v1(dataset, source_rows)
}

fn refit_execution_failure_v1(error: crate::RecipeV4PlsExecutionError) -> RefitFailureV1 {
    let code = match &error {
        crate::RecipeV4PlsExecutionError::Cancelled => RefitFailureCodeV1::Cancelled,
        crate::RecipeV4PlsExecutionError::Estimation(EstimationError::InsufficientObservations) => {
            RefitFailureCodeV1::InsufficientRows
        }
        crate::RecipeV4PlsExecutionError::Estimation(EstimationError::NonConvergence(_)) => {
            RefitFailureCodeV1::Nonconvergence
        }
        crate::RecipeV4PlsExecutionError::Estimation(
            EstimationError::RankDeficient(_)
            | EstimationError::ConstantIndicator(_)
            | EstimationError::OlsNonPositiveResidualDegreesOfFreedom { .. },
        ) => RefitFailureCodeV1::SingularModel,
        crate::RecipeV4PlsExecutionError::Estimation(EstimationError::Numerical(_)) => {
            RefitFailureCodeV1::NonFiniteEstimate
        }
        _ => RefitFailureCodeV1::EngineFailure,
    };
    RefitFailureV1::new(code, error.to_string())
}

fn weighted_point_refit_failure_v1(
    error: crate::MultimodWeightedPlsPointErrorV1,
) -> RefitFailureV1 {
    let code = match &error {
        crate::MultimodWeightedPlsPointErrorV1::Cancelled => RefitFailureCodeV1::Cancelled,
        crate::MultimodWeightedPlsPointErrorV1::DatasetIdentity
        | crate::MultimodWeightedPlsPointErrorV1::ResultContract(_)
        | crate::MultimodWeightedPlsPointErrorV1::Authority(_)
        | crate::MultimodWeightedPlsPointErrorV1::Projection(_)
        | crate::MultimodWeightedPlsPointErrorV1::RawDataRequired => {
            RefitFailureCodeV1::ParameterContractMismatch
        }
        crate::MultimodWeightedPlsPointErrorV1::Estimation(_) => RefitFailureCodeV1::EngineFailure,
    };
    RefitFailureV1::new(code, error.to_string())
}

fn run_prepared_multimod_reflective_plsc_point_v1<C>(
    dataset: &Dataset,
    point_recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
    execution: &ValidatedExecutionRecipe,
    should_cancel: &C,
) -> Result<PlsResult, RefitFailureV1>
where
    C: Fn() -> bool + Sync,
{
    if should_cancel() {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::Cancelled,
            "cancelled before reflective PLSc refit",
        ));
    }
    if dataset.fingerprint.0 != point_recipe.dataset_fingerprint
        || dataset.id.to_string() != plan.dataset_id()
    {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::ParameterContractMismatch,
            "reflective PLSc execution dataset differs from the compiled point authority",
        ));
    }
    let mut report_progress = |_: qpls_estimation::EstimationProgress| !should_cancel();
    let result = estimate_pls_validated_with_control(dataset, execution, &mut report_progress)
        .map_err(|error| {
            let code = match &error {
                EstimationError::Cancelled => RefitFailureCodeV1::Cancelled,
                EstimationError::InsufficientObservations => RefitFailureCodeV1::InsufficientRows,
                EstimationError::NonConvergence(_) => RefitFailureCodeV1::Nonconvergence,
                EstimationError::RankDeficient(_)
                | EstimationError::ConstantIndicator(_)
                | EstimationError::OlsNonPositiveResidualDegreesOfFreedom { .. } => {
                    RefitFailureCodeV1::SingularModel
                }
                EstimationError::Numerical(_) => RefitFailureCodeV1::NonFiniteEstimate,
                _ => RefitFailureCodeV1::EngineFailure,
            };
            RefitFailureV1::new(code, error.to_string())
        })?;
    validate_multimod_reflective_plsc_point_v1(dataset, point_recipe, plan, &result)?;
    Ok(result)
}

fn validate_multimod_reflective_plsc_point_v1(
    dataset: &Dataset,
    point_recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
    result: &PlsResult,
) -> Result<(), RefitFailureV1> {
    let invalid = |detail: String| {
        RefitFailureV1::new(
            RefitFailureCodeV1::ParameterContractMismatch,
            format!("reflective PLSc point contract failed: {detail}"),
        )
    };
    if !result.converged
        || result.method_version != PLSC_METHOD_VERSION
        || result.used_observations != dataset.batch.num_rows()
        || result.omitted_observations != 0
        || result.wpls.is_some()
        || result.point_estimate_attribution.as_ref()
            != Some(&PlsPointEstimateAttributionV1::for_preprocessing(
                point_recipe.settings.preprocessing.clone(),
            ))
    {
        return Err(invalid(
            "method identity, convergence, row accounting, weight state, or point scale is invalid"
                .into(),
        ));
    }
    let Some(plsc) = result.plsc.as_ref() else {
        return Err(invalid("plsc_v2 payload is absent".into()));
    };
    if plsc.method_version != PLSC_METHOD_VERSION
        || plsc.reliability_method_version != DIJKSTRA_HENSELER_RHO_A_METHOD_VERSION
        || plsc.corrected_paths != result.paths
        || plsc.corrected_r_squared != result.r_squared
    {
        return Err(invalid(
            "plsc_v2 identity or corrected structural payload differs from the published point"
                .into(),
        ));
    }

    let expected_constructs = plan
        .blocks()
        .iter()
        .map(|block| block.construct_id().to_owned())
        .collect::<BTreeSet<_>>();
    let score_constructs = result
        .construct_scores
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let reliability_constructs = plsc
        .reliabilities
        .iter()
        .map(|entry| entry.construct.clone())
        .collect::<BTreeSet<_>>();
    if expected_constructs != score_constructs
        || expected_constructs != reliability_constructs
        || plsc.reliabilities.len() != expected_constructs.len()
        || result.construct_scores.values().any(|scores| {
            scores.len() != dataset.batch.num_rows()
                || scores.iter().any(|value| !value.is_finite())
        })
        || plsc
            .reliabilities
            .iter()
            .any(|entry| !entry.rho_a.is_finite() || entry.rho_a <= 0.0 || entry.rho_a > 1.0)
    {
        return Err(invalid(
            "construct-score or reliability inventory differs from the compiled plan".into(),
        ));
    }

    let expected_outer = plan
        .blocks()
        .iter()
        .flat_map(|block| {
            block.indicators().iter().map(|indicator| {
                (
                    block.construct_id().to_owned(),
                    indicator.source_column().to_owned(),
                )
            })
        })
        .collect::<BTreeSet<_>>();
    let ordinary_outer = result
        .outer_estimates
        .iter()
        .map(|entry| (entry.construct.clone(), entry.indicator.clone()))
        .collect::<BTreeSet<_>>();
    let corrected_outer = plsc
        .corrected_outer_loadings
        .iter()
        .map(|entry| (entry.construct.clone(), entry.indicator.clone()))
        .collect::<BTreeSet<_>>();
    if expected_outer != ordinary_outer
        || expected_outer != corrected_outer
        || result.outer_estimates.len() != expected_outer.len()
        || plsc.corrected_outer_loadings.len() != expected_outer.len()
        || result
            .outer_estimates
            .iter()
            .chain(&plsc.corrected_outer_loadings)
            .any(|entry| !entry.weight.is_finite() || !entry.loading.is_finite())
    {
        return Err(invalid(
            "ordinary or corrected outer inventory differs from the compiled plan".into(),
        ));
    }

    let expected_paths = plan
        .paths()
        .iter()
        .map(|path| (path.source().to_owned(), path.target().to_owned()))
        .collect::<BTreeSet<_>>();
    let actual_paths = result
        .paths
        .iter()
        .map(|path| (path.source.clone(), path.target.clone()))
        .collect::<BTreeSet<_>>();
    let expected_controls = plan
        .paths()
        .iter()
        .filter(|path| path.role() == StructuralRelationRoleV4::Control)
        .map(|path| (path.source().to_owned(), path.target().to_owned()))
        .collect::<BTreeSet<_>>();
    let actual_controls = result
        .control_estimates
        .iter()
        .map(|path| (path.source.clone(), path.target.clone()))
        .collect::<BTreeSet<_>>();
    let expected_endogenous = plan
        .paths()
        .iter()
        .map(|path| path.target().to_owned())
        .collect::<BTreeSet<_>>();
    let actual_endogenous = result.r_squared.keys().cloned().collect::<BTreeSet<_>>();
    if expected_paths != actual_paths
        || result.paths.len() != expected_paths.len()
        || expected_controls != actual_controls
        || result.control_estimates.len() != expected_controls.len()
        || expected_endogenous != actual_endogenous
        || result
            .paths
            .iter()
            .any(|path| !path.coefficient.is_finite())
        || result
            .control_estimates
            .iter()
            .any(|path| !path.coefficient.is_finite())
        || result.r_squared.values().any(|value| !value.is_finite())
    {
        return Err(invalid(format!(
            "corrected path, control, or R-squared inventory differs from the compiled plan: expected_paths={expected_paths:?}, actual_paths={actual_paths:?}, expected_controls={expected_controls:?}, actual_controls={actual_controls:?}, expected_endogenous={expected_endogenous:?}, actual_endogenous={actual_endogenous:?}"
        )));
    }

    let expected_correlations = expected_constructs
        .iter()
        .enumerate()
        .flat_map(|(left_index, left)| {
            expected_constructs
                .iter()
                .skip(left_index + 1)
                .map(move |right| (left.clone(), right.clone()))
        })
        .collect::<BTreeSet<_>>();
    let actual_correlations = plsc
        .construct_correlations
        .iter()
        .map(|entry| {
            if entry.left <= entry.right {
                (entry.left.clone(), entry.right.clone())
            } else {
                (entry.right.clone(), entry.left.clone())
            }
        })
        .collect::<BTreeSet<_>>();
    if expected_correlations != actual_correlations
        || plsc.construct_correlations.len() != expected_correlations.len()
        || plsc.construct_correlations.iter().any(|entry| {
            !entry.original.is_finite()
                || !entry.corrected.is_finite()
                || entry.corrected.abs() > 1.0
        })
        || result.effects.iter().any(|effect| {
            !effect.direct.is_finite() || !effect.indirect.is_finite() || !effect.total.is_finite()
        })
    {
        return Err(invalid(
            "corrected correlation or effect inventory contains missing or nonfinite values".into(),
        ));
    }
    Ok(())
}

fn sample_standard_deviation_v1(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64)
        .sqrt()
}

fn correlation_v1(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
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
    (denominator > f64::EPSILON && denominator.is_finite())
        .then_some((cross / denominator).clamp(-1.0, 1.0))
        .filter(|value| value.is_finite())
}

fn frequency_correlation_v1(left: &[f64], right: &[f64], counts: &[u64]) -> Option<f64> {
    if left.len() != right.len() || left.len() != counts.len() || left.is_empty() {
        return None;
    }
    let total = counts.iter().sum::<u64>() as f64;
    if total <= 0.0 || !total.is_finite() {
        return None;
    }
    let left_mean = left
        .iter()
        .zip(counts)
        .map(|(value, count)| value * *count as f64)
        .sum::<f64>()
        / total;
    let right_mean = right
        .iter()
        .zip(counts)
        .map(|(value, count)| value * *count as f64)
        .sum::<f64>()
        / total;
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
    (denominator > f64::EPSILON && denominator.is_finite())
        .then_some((cross / denominator).clamp(-1.0, 1.0))
        .filter(|value| value.is_finite())
}

fn apply_pls_fit_signs_v1(result: &mut PlsResult, signs: &BTreeMap<String, f64>) {
    for estimate in &mut result.outer_estimates {
        let sign = signs.get(&estimate.construct).copied().unwrap_or(1.0);
        estimate.weight *= sign;
        estimate.loading *= sign;
    }
    for (construct, scores) in &mut result.construct_scores {
        if signs.get(construct).copied().unwrap_or(1.0) < 0.0 {
            for score in scores {
                *score = -*score;
            }
        }
    }
    for path in &mut result.paths {
        path.coefficient *= signs.get(&path.source).copied().unwrap_or(1.0)
            * signs.get(&path.target).copied().unwrap_or(1.0);
    }
    for control in &mut result.control_estimates {
        control.coefficient *= signs.get(&control.source).copied().unwrap_or(1.0)
            * signs.get(&control.target).copied().unwrap_or(1.0);
    }
    for effect in &mut result.effects {
        let sign = signs.get(&effect.source).copied().unwrap_or(1.0)
            * signs.get(&effect.target).copied().unwrap_or(1.0);
        effect.direct *= sign;
        effect.indirect *= sign;
        effect.total *= sign;
    }
    if let Some(plsc) = &mut result.plsc {
        for estimate in &mut plsc.corrected_outer_loadings {
            let sign = signs.get(&estimate.construct).copied().unwrap_or(1.0);
            estimate.weight *= sign;
            estimate.loading *= sign;
        }
        for path in &mut plsc.corrected_paths {
            path.coefficient *= signs.get(&path.source).copied().unwrap_or(1.0)
                * signs.get(&path.target).copied().unwrap_or(1.0);
        }
        for correlation in &mut plsc.construct_correlations {
            let sign = signs.get(&correlation.left).copied().unwrap_or(1.0)
                * signs.get(&correlation.right).copied().unwrap_or(1.0);
            correlation.original *= sign;
            correlation.corrected *= sign;
        }
    }
}

fn align_pls_fit_to_reference_v1(
    blocks: &[OrdinaryPlsScoringBlockV1],
    raw_scores: &OrdinaryPlsRawScoreCacheV1,
    pooled_fit: &PlsResult,
    orientation_rows: &[u64],
    result: &mut PlsResult,
) -> Result<(), RefitFailureV1> {
    let mut signs = BTreeMap::<String, f64>::new();
    for block in blocks {
        let reference = raw_scores.composite_scores(block, pooled_fit, orientation_rows)?;
        let candidate = raw_scores.composite_scores(block, result, orientation_rows)?;
        let correlation = correlation_v1(&reference, &candidate).ok_or_else(|| {
            RefitFailureV1::new(
                RefitFailureCodeV1::OrientationUndefined,
                format!(
                    "pooled-reference sign orientation is undefined for {}",
                    block.construct_id
                ),
            )
        })?;
        signs.insert(
            block.construct_id.clone(),
            if correlation < 0.0 { -1.0 } else { 1.0 },
        );
    }
    apply_pls_fit_signs_v1(result, &signs);
    Ok(())
}

struct OrdinaryPlsMgaRefitterV1<'a, C, P> {
    dataset: &'a Dataset,
    authority: OrdinaryPlsPointAuthorityV1,
    source_columns: Vec<String>,
    profile: qpls_core::MgaModelProfileV1,
    weight_column: Option<String>,
    blocks: Vec<OrdinaryPlsScoringBlockV1>,
    projections: Vec<OrdinaryPlsParameterProjectionV1>,
    orientation_rows: Vec<u64>,
    raw_scores: OrdinaryPlsRawScoreCacheV1,
    pooled_fit: PlsResult,
    should_cancel: &'a C,
    progress: &'a P,
    micom_completed: u64,
    micom_total: u64,
}

impl<C, P> OrdinaryPlsMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_rows(&self, source_rows: &[u64]) -> Result<PlsResult, RefitFailureV1> {
        if (self.should_cancel)() {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::Cancelled,
                "cancelled before ordinary PLS refit",
            ));
        }
        // PLS point estimates are functions of the row multiset, not of the
        // incidental order in which a permutation/bootstrap planner emitted
        // that multiset. Sorting preserves repeated bootstrap counts and
        // freezes floating-point accumulation order for shard/worker replay.
        let mut canonical_rows = source_rows.to_vec();
        canonical_rows.sort_unstable();
        let indices = checked_source_rows_v1(self.dataset, &canonical_rows)?;
        let sampled =
            resample_dataset_columns_v1(self.dataset, &self.source_columns, &indices, || {
                (self.should_cancel)()
            })
            .map_err(|error| match error {
                EstimationError::Cancelled => {
                    RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
                }
                other => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, other.to_string()),
            })?;
        let sampled = match self.profile {
            qpls_core::MgaModelProfileV1::CaseWeightedPls => {
                let column = self.weight_column.as_deref().ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        "case-weighted refitter omitted its weight-column identity",
                    )
                })?;
                prepare_multimod_case_weight_dataset_v1(&sampled, column)
                    .map_err(|error| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            format!("case-weight normalization failed: {error}"),
                        )
                    })?
                    .0
            }
            _ => sampled,
        };
        let mut result = self.authority.execute(&sampled, self.should_cancel)?;
        if !result.converged {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::Nonconvergence,
                "ordinary PLS refit did not converge",
            ));
        }
        if result.used_observations != canonical_rows.len() || result.omitted_observations != 0 {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "prepared MGA rows were not complete cases for the projected PLS model",
            ));
        }
        self.align_to_pooled(&mut result)?;
        Ok(result)
    }

    fn align_to_pooled(&self, result: &mut PlsResult) -> Result<(), RefitFailureV1> {
        align_pls_fit_to_reference_v1(
            &self.blocks,
            &self.raw_scores,
            &self.pooled_fit,
            &self.orientation_rows,
            result,
        )
    }

    fn parameter_vector(&self, result: &PlsResult) -> Result<ParameterVectorV1, RefitFailureV1> {
        let parameters = self
            .projections
            .iter()
            .map(|projection| {
                let estimate = match &projection.source {
                    OrdinaryPlsParameterSourceV1::StructuralPath {
                        source,
                        target,
                        role,
                    } => result
                        .paths
                        .iter()
                        .find(|estimate| estimate.source == *source && estimate.target == *target)
                        .map(|estimate| estimate.coefficient)
                        .or_else(|| {
                            (*role == StructuralRelationRoleV4::Control).then(|| {
                                result
                                    .control_estimates
                                    .iter()
                                    .find(|estimate| {
                                        estimate.source == *source && estimate.target == *target
                                    })
                                    .map(|estimate| estimate.coefficient)
                            })?
                        }),
                    OrdinaryPlsParameterSourceV1::OuterLoading {
                        construct,
                        source_column,
                    } => result
                        .plsc
                        .as_ref()
                        .filter(|_| self.profile == qpls_core::MgaModelProfileV1::ReflectivePlsc)
                        .map(|plsc| plsc.corrected_outer_loadings.as_slice())
                        .unwrap_or(result.outer_estimates.as_slice())
                        .iter()
                        .find(|estimate| {
                            estimate.construct == *construct && estimate.indicator == *source_column
                        })
                        .map(|estimate| estimate.loading),
                    OrdinaryPlsParameterSourceV1::OuterWeight {
                        construct,
                        source_column,
                    } => result
                        .outer_estimates
                        .iter()
                        .find(|estimate| {
                            estimate.construct == *construct && estimate.indicator == *source_column
                        })
                        .map(|estimate| estimate.weight),
                    OrdinaryPlsParameterSourceV1::RSquared { construct } => {
                        result.r_squared.get(construct).copied()
                    }
                }
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!(
                            "ordinary PLS refit omitted selected target {}",
                            projection.identity.stable_id
                        ),
                    )
                })?;
                if !estimate.is_finite() {
                    return Err(RefitFailureV1::new(
                        RefitFailureCodeV1::NonFiniteEstimate,
                        format!(
                            "ordinary PLS refit produced a nonfinite value for {}",
                            projection.identity.stable_id
                        ),
                    ));
                }
                Ok(ParameterEstimateV1 {
                    parameter: projection.identity.clone(),
                    estimate,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ParameterVectorV1 { parameters })
    }
}

impl<C, P> MultigroupRefitterV1 for OrdinaryPlsMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit(
        &mut self,
        request: &MultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        let result = self.fit_rows(&request.source_rows)?;
        self.parameter_vector(&result)
    }
}

impl<C, P> MicomRefitterV1 for OrdinaryPlsMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_micom(&mut self, request: &MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1> {
        let scientific_block_count = self
            .blocks
            .iter()
            .filter(|block| {
                !self
                    .authority
                    .technical_construct_ids()
                    .contains(&block.construct_id)
            })
            .count();
        let mut scores = Vec::with_capacity(request.training_groups.len() * scientific_block_count);
        for training in &request.training_groups {
            report(
                self.progress,
                MultiModRunnerPhaseV1::Resampling,
                self.micom_completed.min(self.micom_total),
                self.micom_total.max(1),
                format!(
                    "mga:micom:{:?}:g{}:r{}",
                    request.kind,
                    training.group.get(),
                    request
                        .replicate
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "point".into())
                ),
            );
            let fit = self.fit_rows(&training.source_rows)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            for block in &self.blocks {
                if self
                    .authority
                    .technical_construct_ids()
                    .contains(&block.construct_id)
                {
                    continue;
                }
                scores.push(MicomGroupConstructScoresV1 {
                    group: training.group,
                    construct_id: block.construct_id.clone(),
                    pooled_scores: self.raw_scores.composite_scores(
                        block,
                        &fit,
                        &request.scoring_rows,
                    )?,
                });
            }
        }
        let pooled_reference_scores = if request.kind == qpls_estimation::MicomFitKindV1::Observed {
            report(
                self.progress,
                MultiModRunnerPhaseV1::Resampling,
                self.micom_completed.min(self.micom_total),
                self.micom_total.max(1),
                "mga:micom:observed:pair_pooled_reference",
            );
            let fit = self.fit_rows(&request.scoring_rows)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            self.blocks
                .iter()
                .filter(|block| {
                    !self
                        .authority
                        .technical_construct_ids()
                        .contains(&block.construct_id)
                })
                .map(|block| {
                    let pooled_scores = fit
                        .construct_scores
                        .get(&block.construct_id)
                        .cloned()
                        .ok_or_else(|| {
                            RefitFailureV1::new(
                                RefitFailureCodeV1::ParameterContractMismatch,
                                format!(
                                    "pair-pooled MICOM fit omitted construct {}",
                                    block.construct_id
                                ),
                            )
                        })?;
                    if pooled_scores.len() != request.scoring_rows.len()
                        || pooled_scores.iter().any(|value| !value.is_finite())
                    {
                        return Err(RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            format!(
                                "pair-pooled MICOM scores are invalid for {}",
                                block.construct_id
                            ),
                        ));
                    }
                    Ok(MicomPooledConstructScoresV1 {
                        construct_id: block.construct_id.clone(),
                        pooled_scores,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        Ok(MicomFitV1 {
            scores,
            pooled_reference_scores,
        })
    }
}

fn run_frequency_pls_sample_v1<C>(
    dataset: &Dataset,
    source_columns: &[String],
    source_rows: &[u64],
    counts: &[u64],
    weight_column: &str,
    authority: &OrdinaryPlsPointAuthorityV1,
    should_cancel: &C,
) -> Result<PlsResult, RefitFailureV1>
where
    C: Fn() -> bool + Sync,
{
    if source_rows.len() != counts.len()
        || source_rows.is_empty()
        || counts.iter().any(|count| *count == 0)
    {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::InsufficientRows,
            "frequency PLS refit requires aligned nonempty positive counts",
        ));
    }
    let mut by_row = BTreeMap::<u64, u64>::new();
    for (row, count) in source_rows.iter().zip(counts) {
        let entry = by_row.entry(*row).or_default();
        *entry = entry.checked_add(*count).ok_or_else(|| {
            RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "frequency PLS refit count overflowed",
            )
        })?;
    }
    let canonical_rows = by_row.keys().copied().collect::<Vec<_>>();
    let canonical_counts = by_row.values().copied().collect::<Vec<_>>();
    let indices = checked_frequency_source_rows_v1(dataset, &canonical_rows, &canonical_counts)?;
    let sampled =
        resample_dataset_columns_v1(dataset, source_columns, &indices, || should_cancel())
            .map_err(|error| match error {
                EstimationError::Cancelled => {
                    RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
                }
                other => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, other.to_string()),
            })?;
    let sampled =
        prepare_multimod_frequency_count_dataset_v1(&sampled, weight_column, &canonical_counts)
            .map_err(|error| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    format!("frequency count preparation failed: {error}"),
                )
            })?
            .0;
    let result = authority.execute(&sampled, should_cancel)?;
    if !result.converged {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::Nonconvergence,
            "frequency PLS refit did not converge",
        ));
    }
    if result.used_observations != canonical_rows.len() || result.omitted_observations != 0 {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::ParameterContractMismatch,
            "frequency MGA rows were not complete cases for the projected PLS model",
        ));
    }
    Ok(result)
}

struct FrequencyOrdinaryPlsMgaRefitterV1<'a, C, P> {
    dataset: &'a Dataset,
    authority: OrdinaryPlsPointAuthorityV1,
    source_columns: Vec<String>,
    weight_column: String,
    blocks: Vec<OrdinaryPlsScoringBlockV1>,
    projections: Vec<OrdinaryPlsParameterProjectionV1>,
    orientation_rows: Vec<u64>,
    orientation_counts: Vec<u64>,
    raw_scores: OrdinaryPlsRawScoreCacheV1,
    pooled_fit: PlsResult,
    should_cancel: &'a C,
    progress: &'a P,
    micom_completed: u64,
    micom_total: u64,
}

impl<C, P> FrequencyOrdinaryPlsMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_counts(&self, rows: &[u64], counts: &[u64]) -> Result<PlsResult, RefitFailureV1> {
        let mut result = run_frequency_pls_sample_v1(
            self.dataset,
            &self.source_columns,
            rows,
            counts,
            &self.weight_column,
            &self.authority,
            self.should_cancel,
        )?;
        let mut signs = BTreeMap::<String, f64>::new();
        for block in &self.blocks {
            let reference = self.raw_scores.composite_scores(
                block,
                &self.pooled_fit,
                &self.orientation_rows,
            )?;
            let candidate =
                self.raw_scores
                    .composite_scores(block, &result, &self.orientation_rows)?;
            let correlation =
                frequency_correlation_v1(&reference, &candidate, &self.orientation_counts)
                    .ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::OrientationUndefined,
                            format!(
                                "frequency pooled-reference orientation is undefined for {}",
                                block.construct_id
                            ),
                        )
                    })?;
            signs.insert(
                block.construct_id.clone(),
                if correlation < 0.0 { -1.0 } else { 1.0 },
            );
        }
        apply_pls_fit_signs_v1(&mut result, &signs);
        Ok(result)
    }

    fn parameter_vector(&self, result: &PlsResult) -> Result<ParameterVectorV1, RefitFailureV1> {
        let parameters = self
            .projections
            .iter()
            .map(|projection| {
                let estimate = match &projection.source {
                    OrdinaryPlsParameterSourceV1::StructuralPath {
                        source,
                        target,
                        role,
                    } => result
                        .paths
                        .iter()
                        .find(|estimate| estimate.source == *source && estimate.target == *target)
                        .map(|estimate| estimate.coefficient)
                        .or_else(|| {
                            (*role == StructuralRelationRoleV4::Control).then(|| {
                                result
                                    .control_estimates
                                    .iter()
                                    .find(|estimate| {
                                        estimate.source == *source && estimate.target == *target
                                    })
                                    .map(|estimate| estimate.coefficient)
                            })?
                        }),
                    OrdinaryPlsParameterSourceV1::OuterLoading {
                        construct,
                        source_column,
                    } => result
                        .outer_estimates
                        .iter()
                        .find(|estimate| {
                            estimate.construct == *construct && estimate.indicator == *source_column
                        })
                        .map(|estimate| estimate.loading),
                    OrdinaryPlsParameterSourceV1::OuterWeight {
                        construct,
                        source_column,
                    } => result
                        .outer_estimates
                        .iter()
                        .find(|estimate| {
                            estimate.construct == *construct && estimate.indicator == *source_column
                        })
                        .map(|estimate| estimate.weight),
                    OrdinaryPlsParameterSourceV1::RSquared { construct } => {
                        result.r_squared.get(construct).copied()
                    }
                }
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!(
                            "frequency PLS refit omitted selected target {}",
                            projection.identity.stable_id
                        ),
                    )
                })?;
                if !estimate.is_finite() {
                    return Err(RefitFailureV1::new(
                        RefitFailureCodeV1::NonFiniteEstimate,
                        format!(
                            "frequency PLS refit produced a nonfinite value for {}",
                            projection.identity.stable_id
                        ),
                    ));
                }
                Ok(ParameterEstimateV1 {
                    parameter: projection.identity.clone(),
                    estimate,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ParameterVectorV1 { parameters })
    }
}

impl<C, P> FrequencyMultigroupRefitterV1 for FrequencyOrdinaryPlsMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_frequency(
        &mut self,
        request: &FrequencyMultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        report(
            self.progress,
            match request.sample_kind {
                FitSampleKindV1::ObservedGroup => MultiModRunnerPhaseV1::PointEstimation,
                _ => MultiModRunnerPhaseV1::Resampling,
            },
            request.replicate.unwrap_or(0) as u64,
            1,
            format!(
                "mga:frequency:{:?}:g{}:r{}",
                request.sample_kind,
                request.group.get(),
                request
                    .replicate
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "point".into())
            ),
        );
        let fit = self.fit_counts(&request.source_rows, &request.counts)?;
        self.parameter_vector(&fit)
    }
}

impl<C, P> FrequencyMicomRefitterV1 for FrequencyOrdinaryPlsMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_frequency_micom(
        &mut self,
        request: &FrequencyMicomFitRequestV1,
    ) -> Result<MicomFitV1, RefitFailureV1> {
        let scientific_block_count = self
            .blocks
            .iter()
            .filter(|block| {
                !self
                    .authority
                    .technical_construct_ids()
                    .contains(&block.construct_id)
            })
            .count();
        let mut scores = Vec::with_capacity(request.training_groups.len() * scientific_block_count);
        for training in &request.training_groups {
            report(
                self.progress,
                MultiModRunnerPhaseV1::Resampling,
                self.micom_completed.min(self.micom_total),
                self.micom_total.max(1),
                format!(
                    "mga:frequency:micom:{:?}:g{}:r{}",
                    request.kind,
                    training.group.get(),
                    request
                        .replicate
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "point".into())
                ),
            );
            let fit = self.fit_counts(&training.source_rows, &training.counts)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            for block in &self.blocks {
                if self
                    .authority
                    .technical_construct_ids()
                    .contains(&block.construct_id)
                {
                    continue;
                }
                scores.push(MicomGroupConstructScoresV1 {
                    group: training.group,
                    construct_id: block.construct_id.clone(),
                    pooled_scores: self.raw_scores.composite_scores(
                        block,
                        &fit,
                        &request.scoring_rows,
                    )?,
                });
            }
        }
        let pooled_reference_scores = if request.kind == MicomFitKindV1::Observed {
            let mut counts_by_row = BTreeMap::<u64, u64>::new();
            for training in &request.training_groups {
                for (row, count) in training.source_rows.iter().zip(&training.counts) {
                    let entry = counts_by_row.entry(*row).or_default();
                    *entry = entry.checked_add(*count).ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            "frequency MICOM pooled count overflowed",
                        )
                    })?;
                }
            }
            let pooled_rows = counts_by_row.keys().copied().collect::<Vec<_>>();
            let pooled_counts = counts_by_row.values().copied().collect::<Vec<_>>();
            let fit = self.fit_counts(&pooled_rows, &pooled_counts)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            self.blocks
                .iter()
                .filter(|block| {
                    !self
                        .authority
                        .technical_construct_ids()
                        .contains(&block.construct_id)
                })
                .map(|block| {
                    Ok(MicomPooledConstructScoresV1 {
                        construct_id: block.construct_id.clone(),
                        pooled_scores: self.raw_scores.composite_scores(
                            block,
                            &fit,
                            &request.scoring_rows,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, RefitFailureV1>>()?
        } else {
            Vec::new()
        };
        Ok(MicomFitV1 {
            scores,
            pooled_reference_scores,
        })
    }
}

#[derive(Debug, Clone)]
struct InteractionMgaAuthorityV1 {
    point_recipe: AnalysisRecipeV4,
    point_model: SemModelV4,
    point_artifact: qpls_core::CompiledAnalysisRecipeV4,
    plan: CompiledPlsPlanV3,
    source_columns: Vec<String>,
    blocks: Vec<OrdinaryPlsScoringBlockV1>,
}

fn projected_interaction_mga_authority_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
) -> Result<InteractionMgaAuthorityV1, MultiModRunnerErrorV1> {
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    if !matches!(
        config.profile,
        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
            | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
            | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation
    ) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.interaction_profile_required".into(),
        ));
    }
    let grouping_variable = match &model.group {
        SemGroupV4::ObservedGroups {
            grouping_variable, ..
        } => grouping_variable.clone(),
        SemGroupV4::SingleGroup => {
            return Err(MultiModRunnerErrorV1::Authority(
                "interaction MGA requires an observed-group SemModelV4 authority".into(),
            ));
        }
    };
    let grouping_source = model
        .variables
        .iter()
        .find_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } if id == &grouping_variable => Some(source_column.as_str()),
            _ => None,
        })
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "the interaction-MGA grouping variable is not a directly observed column".into(),
            )
        })?;
    if grouping_source != config.grouping_column {
        return Err(MultiModRunnerErrorV1::Authority(
            "the interaction-MGA grouping column differs from the SemModelV4 binding".into(),
        ));
    }
    let mut scientific_model = model.clone();
    scientific_model.group = SemGroupV4::SingleGroup;
    scientific_model
        .variables
        .retain(|variable| variable.id() != grouping_variable);
    scientific_model.annotations.clear();
    scientific_model.presentation = Default::default();
    scientific_model
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    let general = recipe.general_sem_config.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.interaction_general_sem_config_required".into(),
        )
    })?;
    let plan = compile_pls_plan_v3(&scientific_model, general).map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.mga.interaction_plan_rejected:{error}"
        ))
    })?;
    let CompiledMultiModPlanV1::MgaMultigroupV1 { interactions, .. } = artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not MGA multigroup V1".into(),
        ));
    };
    let expected_interaction_count =
        plan.two_way_interactions().len() + usize::from(plan.three_way_interaction().is_some());
    if interactions.len() != expected_interaction_count
        || match config.profile {
            qpls_core::MgaModelProfileV1::MultipleTwoWayModeration => {
                plan.two_way_interactions().is_empty() || plan.three_way_interaction().is_some()
            }
            qpls_core::MgaModelProfileV1::BoundedThreeWayModeration => {
                plan.three_way_interaction().is_none()
            }
            qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation => {
                plan.two_way_interactions().len() != 1
                    || plan.three_way_interaction().is_some()
                    || plan.two_way_moderated_mediation_target().is_none()
            }
            _ => true,
        }
    {
        return Err(MultiModRunnerErrorV1::Authority(
            "the compiled General SEM interaction inventory differs from the MGA profile".into(),
        ));
    }
    let (mut point_recipe, point_model) =
        project_general_sem_pls_stage_one_recipe_v1(recipe, &scientific_model).map_err(
            |error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.mga.interaction_stage_one_projection_rejected:{error}"
                ))
            },
        )?;
    stage_internal_pls_point_recipe_v1(&mut point_recipe);
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let point_artifact = compile_analysis_recipe_v4(
        &point_recipe,
        Some(&point_model),
        target,
        target.capability_cell_for_recipe(&point_recipe),
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.mga.interaction_stage_one_compilation_rejected:{error}"
        ))
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan: base_plan } = point_artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "interaction MGA stage-one projection did not emit a PLS plan".into(),
        ));
    };
    if base_plan != plan.base_plan() {
        return Err(MultiModRunnerErrorV1::Authority(
            "interaction MGA stage-one artifact differs from the General SEM base plan".into(),
        ));
    }
    let blocks = ordinary_pls_scoring_blocks_v1(base_plan);
    let source_columns = ordinary_pls_source_columns_v1(dataset, &blocks)?;
    Ok(InteractionMgaAuthorityV1 {
        point_recipe,
        point_model,
        point_artifact,
        plan,
        source_columns,
        blocks,
    })
}

#[derive(Debug, Clone)]
struct InteractionMgaFitV1 {
    stage_one: PlsResult,
    joint: RawJointStructuralPointV2,
}

fn interaction_mga_joint_point_v1<C>(
    authority: &InteractionMgaAuthorityV1,
    stage_one: &PlsResult,
    profile: qpls_core::MgaModelProfileV1,
    should_cancel: &C,
) -> Result<RawJointStructuralPointV2, RefitFailureV1>
where
    C: Fn() -> bool + Sync,
{
    match profile {
        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
        | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation => {
            let point = estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
                &authority.plan,
                &stage_one.construct_scores,
                || !should_cancel(),
            )
            .map_err(|error| match error {
                GeneralSemPlsInteractionPointErrorV1::Cancelled => {
                    RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
                }
                _ => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, error.to_string()),
            })?;
            point
                .ensure_valid_against_plan_v1(&authority.plan)
                .map_err(|error| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        error.to_string(),
                    )
                })?;
            Ok(RawJointStructuralPointV2::P2(point))
        }
        qpls_core::MgaModelProfileV1::BoundedThreeWayModeration => {
            let point = estimate_general_sem_pls_three_way_moderation_v1_with_control(
                &authority.plan,
                &stage_one.construct_scores,
                || !should_cancel(),
            )
            .map_err(|error| match error {
                GeneralSemPlsThreeWayPointErrorV1::Cancelled => {
                    RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
                }
                _ => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, error.to_string()),
            })?;
            point
                .ensure_valid_against_plan_v1(&authority.plan)
                .map_err(|error| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        error.to_string(),
                    )
                })?;
            Ok(RawJointStructuralPointV2::P23(point))
        }
        _ => Err(RefitFailureV1::new(
            RefitFailureCodeV1::UnsupportedProfile,
            "interaction joint refitter received a noninteraction profile",
        )),
    }
}

fn interaction_required_constructs_v1(
    authority: &InteractionMgaAuthorityV1,
    interaction_id: &str,
) -> BTreeSet<String> {
    if let Some(interaction) = authority
        .plan
        .two_way_interactions()
        .iter()
        .find(|interaction| interaction.interaction_id() == interaction_id)
    {
        return BTreeSet::from([
            interaction.focal_predictor_id().into(),
            interaction.moderator_id().into(),
            interaction.outcome_id().into(),
        ]);
    }
    if let Some(interaction) = authority
        .plan
        .three_way_interaction()
        .filter(|interaction| interaction.interaction_id() == interaction_id)
    {
        return interaction
            .operand_ids()
            .iter()
            .cloned()
            .chain(std::iter::once(interaction.outcome_id().into()))
            .collect();
    }
    BTreeSet::new()
}

fn interaction_mga_scientific_values_v1(
    authority: &InteractionMgaAuthorityV1,
    profile: qpls_core::MgaModelProfileV1,
    joint: &RawJointStructuralPointV2,
) -> Result<BTreeMap<String, (ParameterFamilyV1, f64, BTreeSet<String>)>, RefitFailureV1> {
    let mut values = BTreeMap::new();
    let insert_path = |values: &mut BTreeMap<_, _>, relation_id: &str, estimate: f64| {
        let path = authority
            .plan
            .base_plan()
            .paths()
            .iter()
            .find(|path| path.relation_id() == relation_id)
            .ok_or_else(|| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    format!("joint result returned unknown relation {relation_id}"),
                )
            })?;
        values.insert(
            path.parameter_id().to_owned(),
            (
                ParameterFamilyV1::StructuralPath,
                estimate,
                BTreeSet::from([path.source().into(), path.target().into()]),
            ),
        );
        Ok::<(), RefitFailureV1>(())
    };
    match joint {
        RawJointStructuralPointV2::P0 => {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "interaction MGA received a P0 joint result",
            ));
        }
        RawJointStructuralPointV2::P2(point) => {
            for row in point.structural_coefficients() {
                insert_path(&mut values, row.relation_id(), row.estimate())?;
            }
            for row in point.interaction_coefficients() {
                values.insert(
                    row.interaction_effect_relation_id().into(),
                    (
                        ParameterFamilyV1::InteractionGamma,
                        row.raw_product_estimate(),
                        interaction_required_constructs_v1(authority, row.interaction_id()),
                    ),
                );
            }
            for row in point.simple_slopes() {
                let probe = match row.moderator_value_standardized() as i32 {
                    -1 => "minus_1",
                    0 => "zero",
                    1 => "plus_1",
                    _ => {
                        return Err(RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            "two-way engine returned a noncanonical simple-slope probe",
                        ));
                    }
                };
                values.insert(
                    format!(
                        "simple_slope:{}:{}:{probe}",
                        row.interaction_id(),
                        row.moderator_id()
                    ),
                    (
                        ParameterFamilyV1::SimpleSlope,
                        row.estimate(),
                        interaction_required_constructs_v1(authority, row.interaction_id()),
                    ),
                );
            }
            if profile == qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation {
                let target = authority
                    .plan
                    .two_way_moderated_mediation_target()
                    .ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            "bounded moderated-mediation profile omitted its compiled target",
                        )
                    })?;
                let moderated = point
                    .structural_coefficients()
                    .iter()
                    .find(|row| row.relation_id() == target.moderated_relation_id())
                    .ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            "moderated-stage coefficient is absent",
                        )
                    })?;
                let other = point
                    .structural_coefficients()
                    .iter()
                    .find(|row| row.relation_id() == target.other_stage_relation_id())
                    .ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            "other-stage coefficient is absent",
                        )
                    })?;
                let gamma = point
                    .interaction_coefficients()
                    .iter()
                    .find(|row| row.interaction_id() == target.interaction_id())
                    .ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            "moderated-mediation gamma is absent",
                        )
                    })?;
                let derived =
                    qpls_core::calculate_general_sem_pls_two_way_moderated_mediation_point_v1(
                        target,
                        moderated.estimate(),
                        other.estimate(),
                        gamma.raw_product_estimate(),
                    )
                    .map_err(|error| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::NonFiniteEstimate,
                            error.to_string(),
                        )
                    })?;
                let required = BTreeSet::from([
                    target.x_id().into(),
                    target.mediator_id().into(),
                    target.y_id().into(),
                    target.moderator_id().into(),
                ]);
                for effect in derived.conditional_indirect_effects {
                    values.insert(
                        effect.effect_id,
                        (
                            ParameterFamilyV1::SpecificIndirect,
                            effect.estimate,
                            required.clone(),
                        ),
                    );
                }
                values.insert(
                    derived.moderated_mediation_index.effect_id,
                    (
                        ParameterFamilyV1::Other,
                        derived.moderated_mediation_index.estimate,
                        required,
                    ),
                );
            }
        }
        RawJointStructuralPointV2::P23(point) => {
            for row in &point.structural_coefficients {
                insert_path(&mut values, row.relation_id(), row.estimate())?;
            }
            for row in &point.lower_order_interaction_coefficients {
                values.insert(
                    row.interaction_effect_relation_id().into(),
                    (
                        ParameterFamilyV1::InteractionGamma,
                        row.raw_product_estimate(),
                        interaction_required_constructs_v1(authority, row.interaction_id()),
                    ),
                );
            }
            values.insert(
                qpls_estimation::three_way_delta_target_id(
                    &point.three_way_coefficient.interaction_id,
                ),
                (
                    ParameterFamilyV1::ThreeWayDelta,
                    point.three_way_coefficient.scientific_rescaled_delta,
                    interaction_required_constructs_v1(
                        authority,
                        &point.three_way_coefficient.interaction_id,
                    ),
                ),
            );
            for row in &point.conditional_interaction_effects {
                values.insert(
                    row.target_id.clone(),
                    (
                        ParameterFamilyV1::Other,
                        row.estimate,
                        interaction_required_constructs_v1(authority, &row.interaction_id),
                    ),
                );
            }
            for row in &point.simple_slopes {
                values.insert(
                    row.target_id.clone(),
                    (
                        ParameterFamilyV1::SimpleSlope,
                        row.estimate,
                        interaction_required_constructs_v1(authority, &row.interaction_id),
                    ),
                );
            }
        }
    }
    if values.is_empty() || values.values().any(|(_, value, _)| !value.is_finite()) {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::NonFiniteEstimate,
            "interaction MGA scientific target inventory is empty or nonfinite",
        ));
    }
    Ok(values)
}

#[derive(Debug, Clone)]
struct InteractionMgaParameterProjectionV1 {
    identity: ParameterIdentityV1,
    required_constructs: BTreeSet<String>,
}

fn interaction_mga_parameter_projections_v1(
    config: &MgaMultigroupV1,
    authority: &InteractionMgaAuthorityV1,
    pooled_joint: &RawJointStructuralPointV2,
) -> Result<Vec<InteractionMgaParameterProjectionV1>, MultiModRunnerErrorV1> {
    let inventory = interaction_mga_scientific_values_v1(authority, config.profile, pooled_joint)
        .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.detail))?;
    let selected = if config.selected_parameter_ids.is_empty() {
        inventory.keys().cloned().collect::<Vec<_>>()
    } else {
        config.selected_parameter_ids.clone()
    };
    let projections = selected
        .iter()
        .map(|target_id| {
            let (family, _, required_constructs) = inventory.get(target_id).ok_or_else(|| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.mga.interaction_target_unmapped:{target_id}"
                ))
            })?;
            Ok(InteractionMgaParameterProjectionV1 {
                identity: ParameterIdentityV1 {
                    stable_id: target_id.clone(),
                    family: *family,
                },
                required_constructs: required_constructs.clone(),
            })
        })
        .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?;
    if projections.is_empty()
        || projections
            .iter()
            .map(|projection| projection.identity.stable_id.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != projections.len()
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "interaction MGA target projections are empty or duplicated".into(),
        ));
    }
    Ok(projections)
}

struct InteractionMgaRefitterV1<'a, C, P> {
    dataset: &'a Dataset,
    authority: InteractionMgaAuthorityV1,
    profile: qpls_core::MgaModelProfileV1,
    projections: Vec<InteractionMgaParameterProjectionV1>,
    orientation_rows: Vec<u64>,
    raw_scores: OrdinaryPlsRawScoreCacheV1,
    pooled_stage_one: PlsResult,
    should_cancel: &'a C,
    progress: &'a P,
    micom_completed: u64,
    micom_total: u64,
}

impl<C, P> InteractionMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_rows(&self, source_rows: &[u64]) -> Result<InteractionMgaFitV1, RefitFailureV1> {
        if (self.should_cancel)() {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::Cancelled,
                "cancelled before interaction MGA refit",
            ));
        }
        let mut canonical_rows = source_rows.to_vec();
        canonical_rows.sort_unstable();
        let indices = checked_source_rows_v1(self.dataset, &canonical_rows)?;
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &indices,
            || (self.should_cancel)(),
        )
        .map_err(|error| match error {
            EstimationError::Cancelled => {
                RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
            }
            other => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, other.to_string()),
        })?;
        let execution = run_compiled_pls_recipe_v4(
            &sampled,
            &self.authority.point_recipe,
            &self.authority.point_model,
            &self.authority.point_artifact,
            None,
            || (self.should_cancel)(),
            |_| {},
        )
        .map_err(refit_execution_failure_v1)?;
        let mut stage_one = execution.estimation().clone();
        if !stage_one.converged
            || stage_one.used_observations != canonical_rows.len()
            || stage_one.omitted_observations != 0
        {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::Nonconvergence,
                "interaction MGA stage-one refit did not converge on exactly the requested rows",
            ));
        }
        align_pls_fit_to_reference_v1(
            &self.authority.blocks,
            &self.raw_scores,
            &self.pooled_stage_one,
            &self.orientation_rows,
            &mut stage_one,
        )?;
        let joint = interaction_mga_joint_point_v1(
            &self.authority,
            &stage_one,
            self.profile,
            self.should_cancel,
        )?;
        Ok(InteractionMgaFitV1 { stage_one, joint })
    }

    fn parameter_vector(
        &self,
        fit: &InteractionMgaFitV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        let values =
            interaction_mga_scientific_values_v1(&self.authority, self.profile, &fit.joint)?;
        Ok(ParameterVectorV1 {
            parameters: self
                .projections
                .iter()
                .map(|projection| {
                    values
                        .get(&projection.identity.stable_id)
                        .map(|(_, estimate, _)| ParameterEstimateV1 {
                            parameter: projection.identity.clone(),
                            estimate: *estimate,
                        })
                        .ok_or_else(|| {
                            RefitFailureV1::new(
                                RefitFailureCodeV1::ParameterContractMismatch,
                                format!(
                                    "interaction refit omitted target {}",
                                    projection.identity.stable_id
                                ),
                            )
                        })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

impl<C, P> MultigroupRefitterV1 for InteractionMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit(
        &mut self,
        request: &MultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        let fit = self.fit_rows(&request.source_rows)?;
        self.parameter_vector(&fit)
    }
}

impl<C, P> MicomRefitterV1 for InteractionMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_micom(&mut self, request: &MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1> {
        let mut scores =
            Vec::with_capacity(request.training_groups.len() * self.authority.blocks.len());
        for training in &request.training_groups {
            report(
                self.progress,
                MultiModRunnerPhaseV1::Resampling,
                self.micom_completed.min(self.micom_total),
                self.micom_total.max(1),
                format!(
                    "mga:interaction:micom:{:?}:g{}:r{}",
                    request.kind,
                    training.group.get(),
                    request
                        .replicate
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "point".into())
                ),
            );
            let fit = self.fit_rows(&training.source_rows)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            for block in &self.authority.blocks {
                scores.push(MicomGroupConstructScoresV1 {
                    group: training.group,
                    construct_id: block.construct_id.clone(),
                    pooled_scores: self.raw_scores.composite_scores(
                        block,
                        &fit.stage_one,
                        &request.scoring_rows,
                    )?,
                });
            }
        }
        let pooled_reference_scores = if request.kind == qpls_estimation::MicomFitKindV1::Observed {
            let fit = self.fit_rows(&request.scoring_rows)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            self.authority
                .blocks
                .iter()
                .map(|block| {
                    let pooled_scores = fit
                        .stage_one
                        .construct_scores
                        .get(&block.construct_id)
                        .cloned()
                        .ok_or_else(|| {
                            RefitFailureV1::new(
                                RefitFailureCodeV1::ParameterContractMismatch,
                                format!(
                                    "pair-pooled interaction MICOM fit omitted {}",
                                    block.construct_id
                                ),
                            )
                        })?;
                    Ok(MicomPooledConstructScoresV1 {
                        construct_id: block.construct_id.clone(),
                        pooled_scores,
                    })
                })
                .collect::<Result<Vec<_>, RefitFailureV1>>()?
        } else {
            Vec::new()
        };
        Ok(MicomFitV1 {
            scores,
            pooled_reference_scores,
        })
    }
}

#[derive(Debug, Clone)]
struct HocMgaStageAuthorityV1 {
    recipe: AnalysisRecipeV4,
    model: SemModelV4,
    artifact: qpls_core::CompiledAnalysisRecipeV4,
    plan: CompiledPlsPlanV2,
}

#[derive(Debug, Clone)]
struct HocMgaAuthorityV1 {
    scientific_model: SemModelV4,
    plan: CompiledPlsPlanV3,
    stages: Vec<HocMgaStageAuthorityV1>,
    repeated_stage_index: Option<usize>,
    score_stage_index: Option<usize>,
    source_columns: Vec<String>,
    raw_blocks: Vec<OrdinaryPlsScoringBlockV1>,
    final_blocks: Vec<OrdinaryPlsScoringBlockV1>,
    virtual_alias_sources: BTreeMap<String, String>,
    generated_score_sources: BTreeMap<String, String>,
}

fn compile_hoc_mga_stage_v1(
    recipe: &AnalysisRecipeV4,
    model: SemModelV4,
) -> Result<HocMgaStageAuthorityV1, MultiModRunnerErrorV1> {
    let mut stage_recipe = project_general_sem_pls_base_recipe_v1(recipe).map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.mga.hoc_stage_recipe_rejected:{error}"
        ))
    })?;
    stage_internal_pls_point_recipe_v1(&mut stage_recipe);
    stage_recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: model
            .scientific_sha256()
            .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?,
        model: model.clone(),
    };
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let artifact = compile_analysis_recipe_v4(
        &stage_recipe,
        Some(&model),
        target,
        target.capability_cell_for_recipe(&stage_recipe),
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.mga.hoc_stage_compilation_rejected:{error}"
        ))
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "HOC stage projection did not emit a PLS plan".into(),
        ));
    };
    let plan = plan.clone();
    Ok(HocMgaStageAuthorityV1 {
        recipe: stage_recipe,
        model,
        artifact,
        plan,
    })
}

fn projected_hoc_mga_authority_v1(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
) -> Result<HocMgaAuthorityV1, MultiModRunnerErrorV1> {
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    if config.profile != qpls_core::MgaModelProfileV1::MultipleNonnestedHoc {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.multiple_hoc_profile_required".into(),
        ));
    }
    let grouping_variable = match &model.group {
        SemGroupV4::ObservedGroups {
            grouping_variable, ..
        } => grouping_variable.clone(),
        SemGroupV4::SingleGroup => {
            return Err(MultiModRunnerErrorV1::Authority(
                "multiple-HOC MGA requires an observed-group SemModelV4 authority".into(),
            ));
        }
    };
    let grouping_source = model
        .variables
        .iter()
        .find_map(|variable| match variable {
            SemVariableV4::Observed {
                id, source_column, ..
            } if id == &grouping_variable => Some(source_column.as_str()),
            _ => None,
        })
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "the HOC-MGA grouping variable is not a directly observed column".into(),
            )
        })?;
    if grouping_source != config.grouping_column {
        return Err(MultiModRunnerErrorV1::Authority(
            "the HOC-MGA grouping column differs from the SemModelV4 binding".into(),
        ));
    }
    let mut scientific_model = model.clone();
    scientific_model.group = SemGroupV4::SingleGroup;
    scientific_model
        .variables
        .retain(|variable| variable.id() != grouping_variable);
    scientific_model.annotations.clear();
    scientific_model.presentation = Default::default();
    scientific_model
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::Authority(error.to_string()))?;
    let general = recipe.general_sem_config.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.hoc_general_sem_config_required".into(),
        )
    })?;
    let plan = compile_pls_plan_v3_multimod_multiple_hoc_v2(&scientific_model, general).map_err(
        |error| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.hoc_plan_rejected:{error}"
            ))
        },
    )?;
    let hocs = plan.higher_order_stage_plans();
    if !(1..=4).contains(&hocs.len()) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.hoc_count_outside_one_to_four".into(),
        ));
    }
    let approach = hocs[0].approach();
    if hocs.iter().any(|hoc| hoc.approach() != approach) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.hoc_mixed_approaches_blocked".into(),
        ));
    }
    let CompiledMultiModPlanV1::MgaMultigroupV1 {
        hocs: compiled_hocs,
        ..
    } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not MGA multigroup V1".into(),
        ));
    };
    if compiled_hocs.len() != hocs.len() {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled HOC inventory differs from the General SEM plan".into(),
        ));
    }

    let base_model = compile_pls_higher_order_lower_order_projection_multimod_v2(&scientific_model)
        .map(|projection| projection.projected_model().clone())
        .map_err(|error| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.mga.hoc_base_projection_rejected:{error}"
            ))
        })?;
    let base_stage = compile_hoc_mga_stage_v1(recipe, base_model)?;
    if &base_stage.plan != plan.base_plan() {
        return Err(MultiModRunnerErrorV1::Authority(
            "multiple-HOC base-stage artifact differs from the compiled plan".into(),
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
                "multimod.runner.mga.hoc_repeated_projection_rejected:{error}"
            ))
        })?;
        repeated_stage_index = Some(stages.len());
        stages.push(compile_hoc_mga_stage_v1(
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
                    "multimod.runner.mga.hoc_score_projection_rejected:{error}"
                ))
            })?;
        score_stage_index = Some(stages.len());
        stages.push(compile_hoc_mga_stage_v1(
            recipe,
            projection.projected_model().clone(),
        )?);
    }
    let final_stage = stages.last().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("multiple-HOC execution has no final stage".into())
    })?;
    let raw_blocks = ordinary_pls_scoring_blocks_v1(plan.base_plan());
    let final_blocks = ordinary_pls_scoring_blocks_v1(&final_stage.plan);
    let source_columns = ordinary_pls_source_columns_v1(dataset, &raw_blocks)?;
    let virtual_alias_sources = hocs
        .iter()
        .flat_map(|hoc| hoc.component_mappings())
        .flat_map(|mapping| mapping.virtual_indicators())
        .map(|indicator| {
            (
                indicator.generated_source_column_id().to_owned(),
                indicator.source_column().to_owned(),
            )
        })
        .collect();
    let generated_score_sources = hocs
        .iter()
        .flat_map(|hoc| hoc.component_mappings())
        .map(|mapping| {
            (
                mapping.generated_score_variable_id().to_owned(),
                mapping.component_id().to_owned(),
            )
        })
        .collect();
    Ok(HocMgaAuthorityV1 {
        scientific_model,
        plan,
        stages,
        repeated_stage_index,
        score_stage_index,
        source_columns,
        raw_blocks,
        final_blocks,
        virtual_alias_sources,
        generated_score_sources,
    })
}

#[derive(Debug, Clone)]
struct HocMgaFitV1 {
    stages: Vec<PlsResult>,
}

impl HocMgaFitV1 {
    fn final_result(&self) -> &PlsResult {
        self.stages
            .last()
            .expect("validated HOC fit always contains a stage")
    }
}

fn run_hoc_mga_stage_v1<C>(
    dataset: &Dataset,
    stage: &HocMgaStageAuthorityV1,
    should_cancel: &C,
) -> Result<PlsResult, RefitFailureV1>
where
    C: Fn() -> bool + Sync,
{
    let execution = run_compiled_pls_recipe_v4_allowing_isolated(
        dataset,
        &stage.recipe,
        &stage.model,
        &stage.artifact,
        || should_cancel(),
        |_| {},
    )
    .map_err(refit_execution_failure_v1)?;
    let result = execution.estimation().clone();
    if !result.converged
        || result.used_observations != dataset.batch.num_rows()
        || result.omitted_observations != 0
    {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::Nonconvergence,
            "multiple-HOC stage did not converge on exactly the requested complete rows",
        ));
    }
    Ok(result)
}

fn hoc_alias_specs_v1(authority: &HocMgaAuthorityV1) -> Vec<PlsAliasColumnSpecV1> {
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
                        "Repeated HOC indicator: {} <- {}",
                        hoc_id,
                        mapping.component_id()
                    ),
                })
        })
        .collect()
}

fn hoc_mga_fit_sample_v1<C>(
    sampled: &Dataset,
    authority: &HocMgaAuthorityV1,
    pooled_reference: Option<&HocMgaFitV1>,
    sampled_positions: &[usize],
    raw_scores: Option<(&OrdinaryPlsRawScoreCacheV1, &[u64])>,
    should_cancel: &C,
) -> Result<HocMgaFitV1, RefitFailureV1>
where
    C: Fn() -> bool + Sync,
{
    let mut results = Vec::with_capacity(authority.stages.len());
    let mut base = run_hoc_mga_stage_v1(sampled, &authority.stages[0], should_cancel)?;
    if let Some((raw_scores, orientation_rows)) = raw_scores {
        align_pls_fit_to_reference_v1(
            &authority.raw_blocks,
            raw_scores,
            &pooled_reference
                .expect("raw reference accompanies pooled HOC reference")
                .stages[0],
            orientation_rows,
            &mut base,
        )?;
    }
    results.push(base);

    if let Some(stage_index) = authority.repeated_stage_index {
        let aliases = append_pls_alias_columns_v1(sampled, &hoc_alias_specs_v1(authority))
            .map_err(|error| {
                RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, error.to_string())
            })?;
        let mut repeated =
            run_hoc_mga_stage_v1(&aliases, &authority.stages[stage_index], should_cancel)?;
        if let Some(reference) = pooled_reference {
            align_general_sem_pls_hoc_result_signs_v1(
                &mut repeated,
                GeneralSemPlsHocScoreAlignmentReferenceV1::new(
                    &reference.stages[stage_index].construct_scores,
                    sampled_positions,
                ),
                &|| should_cancel(),
            )
            .map_err(|error| {
                RefitFailureV1::new(RefitFailureCodeV1::OrientationUndefined, error.to_string())
            })?;
        }
        results.push(repeated);
    }

    if let Some(stage_index) = authority.score_stage_index {
        let score_source_index = authority.repeated_stage_index.unwrap_or(0);
        let prepared = prepare_general_sem_pls_disjoint_hoc_score_dataset_multimod_v2(
            sampled,
            &authority.plan,
            &results[score_source_index],
            || !should_cancel(),
        )
        .map_err(|error| {
            RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, error.to_string())
        })?;
        let mut score = run_hoc_mga_stage_v1(
            prepared.dataset(),
            &authority.stages[stage_index],
            should_cancel,
        )?;
        if let Some(reference) = pooled_reference {
            align_general_sem_pls_hoc_result_signs_v1(
                &mut score,
                GeneralSemPlsHocScoreAlignmentReferenceV1::new(
                    &reference.stages[stage_index].construct_scores,
                    sampled_positions,
                ),
                &|| should_cancel(),
            )
            .map_err(|error| {
                RefitFailureV1::new(RefitFailureCodeV1::OrientationUndefined, error.to_string())
            })?;
        }
        results.push(score);
    }
    if results.len() != authority.stages.len() {
        return Err(RefitFailureV1::new(
            RefitFailureCodeV1::ParameterContractMismatch,
            "multiple-HOC execution omitted a dependency stage",
        ));
    }
    Ok(HocMgaFitV1 { stages: results })
}

struct HocMgaRefitterV1<'a, C, P> {
    dataset: &'a Dataset,
    authority: HocMgaAuthorityV1,
    projections: Vec<OrdinaryPlsParameterProjectionV1>,
    orientation_rows: Vec<u64>,
    orientation_positions: BTreeMap<u64, usize>,
    raw_scores: OrdinaryPlsRawScoreCacheV1,
    pooled_fit: HocMgaFitV1,
    should_cancel: &'a C,
    progress: &'a P,
    micom_completed: u64,
    micom_total: u64,
}

impl<C, P> HocMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_rows(&self, source_rows: &[u64]) -> Result<HocMgaFitV1, RefitFailureV1> {
        let mut canonical_rows = source_rows.to_vec();
        canonical_rows.sort_unstable();
        let indices = checked_source_rows_v1(self.dataset, &canonical_rows)?;
        let sampled_positions = canonical_rows
            .iter()
            .map(|row| {
                self.orientation_positions.get(row).copied().ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        "HOC refit row is outside the pooled orientation universe",
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &indices,
            || (self.should_cancel)(),
        )
        .map_err(|error| match error {
            EstimationError::Cancelled => {
                RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
            }
            other => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, other.to_string()),
        })?;
        hoc_mga_fit_sample_v1(
            &sampled,
            &self.authority,
            Some(&self.pooled_fit),
            &sampled_positions,
            Some((&self.raw_scores, &self.orientation_rows)),
            self.should_cancel,
        )
    }

    fn parameter_vector(&self, fit: &HocMgaFitV1) -> Result<ParameterVectorV1, RefitFailureV1> {
        let result = fit.final_result();
        let parameters = self
            .projections
            .iter()
            .map(|projection| {
                let estimate = match &projection.source {
                    OrdinaryPlsParameterSourceV1::StructuralPath {
                        source,
                        target,
                        role,
                    } => result
                        .paths
                        .iter()
                        .find(|row| row.source == *source && row.target == *target)
                        .map(|row| row.coefficient)
                        .or_else(|| {
                            (*role == StructuralRelationRoleV4::Control).then(|| {
                                result
                                    .control_estimates
                                    .iter()
                                    .find(|row| row.source == *source && row.target == *target)
                                    .map(|row| row.coefficient)
                            })?
                        }),
                    OrdinaryPlsParameterSourceV1::OuterLoading {
                        construct,
                        source_column,
                    } => result
                        .outer_estimates
                        .iter()
                        .find(|row| row.construct == *construct && row.indicator == *source_column)
                        .map(|row| row.loading),
                    OrdinaryPlsParameterSourceV1::OuterWeight {
                        construct,
                        source_column,
                    } => result
                        .outer_estimates
                        .iter()
                        .find(|row| row.construct == *construct && row.indicator == *source_column)
                        .map(|row| row.weight),
                    OrdinaryPlsParameterSourceV1::RSquared { construct } => {
                        result.r_squared.get(construct).copied()
                    }
                }
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!("HOC refit omitted target {}", projection.identity.stable_id),
                    )
                })?;
                Ok(ParameterEstimateV1 {
                    parameter: projection.identity.clone(),
                    estimate,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ParameterVectorV1 { parameters })
    }

    fn score_construct_on_rows(
        &self,
        fit: &HocMgaFitV1,
        stage_index: usize,
        construct_id: &str,
        scoring_rows: &[u64],
    ) -> Result<Vec<f64>, RefitFailureV1> {
        let stage = self.authority.stages.get(stage_index).ok_or_else(|| {
            RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "HOC scoring stage is absent",
            )
        })?;
        let block = stage
            .plan
            .blocks()
            .iter()
            .find(|block| block.construct_id() == construct_id)
            .ok_or_else(|| {
                RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    format!("HOC scoring block {construct_id} is absent"),
                )
            })?;
        let result = &fit.stages[stage_index];
        let mut output = vec![0.0; scoring_rows.len()];
        for indicator in block.indicators() {
            let source_column = indicator.source_column();
            let outer = result
                .outer_estimates
                .iter()
                .find(|row| row.construct == construct_id && row.indicator == source_column)
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!("HOC scoring rule omitted {construct_id}:{source_column}"),
                    )
                })?;
            let transform = result
                .transforms
                .iter()
                .find(|row| row.indicator == source_column)
                .ok_or_else(|| {
                    RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        format!("HOC scoring transform omitted {source_column}"),
                    )
                })?;
            if !outer.weight.is_finite()
                || !transform.scale.is_finite()
                || transform.scale.abs() <= f64::EPSILON
            {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::OrientationUndefined,
                    format!("HOC scoring rule is degenerate for {construct_id}:{source_column}"),
                ));
            }
            let effective_source = self
                .authority
                .virtual_alias_sources
                .get(source_column)
                .map(String::as_str)
                .unwrap_or(source_column);
            let source_values = if let Some(component_id) =
                self.authority.generated_score_sources.get(effective_source)
            {
                if stage_index == 0 {
                    return Err(RefitFailureV1::new(
                        RefitFailureCodeV1::ParameterContractMismatch,
                        "generated HOC score appeared in the base stage",
                    ));
                }
                self.score_construct_on_rows(fit, stage_index - 1, component_id, scoring_rows)?
            } else {
                let values = self
                    .raw_scores
                    .values
                    .get(effective_source)
                    .ok_or_else(|| {
                        RefitFailureV1::new(
                            RefitFailureCodeV1::ParameterContractMismatch,
                            format!("HOC raw scoring cache omitted {effective_source}"),
                        )
                    })?;
                scoring_rows
                    .iter()
                    .map(|row| {
                        self.raw_scores
                            .row_positions
                            .get(row)
                            .map(|position| values[*position])
                            .ok_or_else(|| {
                                RefitFailureV1::new(
                                    RefitFailureCodeV1::ParameterContractMismatch,
                                    "HOC scoring row is outside the pooled row universe",
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?
            };
            let coefficient = outer.weight / transform.scale;
            for (target, source) in output.iter_mut().zip(source_values) {
                *target += coefficient * source;
            }
        }
        if sample_standard_deviation_v1(&output) <= f64::EPSILON
            || output.iter().any(|value| !value.is_finite())
        {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::OrientationUndefined,
                format!("HOC pooled scoring proxy has zero variance for {construct_id}"),
            ));
        }
        Ok(output)
    }
}

impl<C, P> MultigroupRefitterV1 for HocMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit(
        &mut self,
        request: &MultigroupFitRequestV1,
    ) -> Result<ParameterVectorV1, RefitFailureV1> {
        let fit = self.fit_rows(&request.source_rows)?;
        self.parameter_vector(&fit)
    }
}

impl<C, P> MicomRefitterV1 for HocMgaRefitterV1<'_, C, P>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit_micom(&mut self, request: &MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1> {
        let final_stage_index = self.authority.stages.len() - 1;
        let mut scores =
            Vec::with_capacity(request.training_groups.len() * self.authority.final_blocks.len());
        for training in &request.training_groups {
            report(
                self.progress,
                MultiModRunnerPhaseV1::Resampling,
                self.micom_completed.min(self.micom_total),
                self.micom_total.max(1),
                format!(
                    "mga:hoc:micom:{:?}:g{}:r{}",
                    request.kind,
                    training.group.get(),
                    request
                        .replicate
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "point".into())
                ),
            );
            let fit = self.fit_rows(&training.source_rows)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            for block in &self.authority.final_blocks {
                scores.push(MicomGroupConstructScoresV1 {
                    group: training.group,
                    construct_id: block.construct_id.clone(),
                    pooled_scores: self.score_construct_on_rows(
                        &fit,
                        final_stage_index,
                        &block.construct_id,
                        &request.scoring_rows,
                    )?,
                });
            }
        }
        let pooled_reference_scores = if request.kind == qpls_estimation::MicomFitKindV1::Observed {
            let fit = self.fit_rows(&request.scoring_rows)?;
            self.micom_completed = self.micom_completed.saturating_add(1);
            self.authority
                .final_blocks
                .iter()
                .map(|block| {
                    let values = fit
                        .final_result()
                        .construct_scores
                        .get(&block.construct_id)
                        .cloned()
                        .ok_or_else(|| {
                            RefitFailureV1::new(
                                RefitFailureCodeV1::ParameterContractMismatch,
                                format!("pair-pooled HOC fit omitted {}", block.construct_id),
                            )
                        })?;
                    Ok(MicomPooledConstructScoresV1 {
                        construct_id: block.construct_id.clone(),
                        pooled_scores: values,
                    })
                })
                .collect::<Result<Vec<_>, RefitFailureV1>>()?
        } else {
            Vec::new()
        };
        Ok(MicomFitV1 {
            scores,
            pooled_reference_scores,
        })
    }
}

/// Executes the qualified one-through-four disjoint, nonnested, homogeneous-
/// approach HOC MGA envelope. Every observed and resampled request reruns all
/// dependency stages before extracting the final scientific targets.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_hoc_pls_mga_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_hoc_pls_mga_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        excluded_rows,
        None,
        None,
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_compiled_hoc_pls_mga_internal_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    mut execution_cache: Option<&mut ValidatedMgaExecutionCacheSessionV1<'_>>,
    mut checkpoint: Option<&mut MgaShardCheckpointCallbackV1<'_>>,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    if config.profile != qpls_core::MgaModelProfileV1::MultipleNonnestedHoc {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.multiple_hoc_profile_required".into(),
        ));
    }
    if config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::ParametricPooledVariance
                | MgaProcedureV1::ParametricWelchSatterthwaite
                | MgaProcedureV1::ParametricWaldOmnibus
        )
    }) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.hoc_parametric_se_semantics_unavailable".into(),
        ));
    }
    let predicted_sidecar_bytes = predict_mga_sidecar_bytes_v1(
        config,
        &compact_group_row_counts_v1(design),
        multimod_model_target_upper_bound_v1(&config.selected_parameter_ids, model),
        multimod_model_target_id_maximum_bytes_v1(&config.selected_parameter_ids, model),
        multimod_model_micom_construct_upper_bound_v1(model),
    );
    enforce_multimod_sidecar_cost_v1("mga", predicted_sidecar_bytes, &progress)?;
    let eligibility = assess_multigroup_design_v1(design);
    if !eligibility.eligible {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "multiple-HOC PLS MGA design is ineligible: {:?}",
            eligibility.blockers
        )));
    }
    validate_prepared_group_membership_v1(dataset, config, design)?;
    let authority = projected_hoc_mga_authority_v1(dataset, recipe, model, artifact)?;
    let final_stage = authority.stages.last().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("multiple-HOC execution has no final stage".into())
    })?;
    let projections = ordinary_pls_parameter_projections_v1(
        config,
        &authority.scientific_model,
        &final_stage.plan,
    )?;
    let parameters = projections
        .iter()
        .map(|projection| projection.identity.clone())
        .collect::<Vec<_>>();
    let mut orientation_rows = design
        .rows
        .iter()
        .map(|row| row.source_row)
        .collect::<Vec<_>>();
    orientation_rows.sort_unstable();
    orientation_rows.dedup();
    if orientation_rows.len() != design.rows.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "multiple-HOC PLS MGA design contains duplicate source rows".into(),
        ));
    }
    let orientation_positions = orientation_rows
        .iter()
        .enumerate()
        .map(|(position, source_row)| (*source_row, position))
        .collect::<BTreeMap<_, _>>();
    let raw_scores = OrdinaryPlsRawScoreCacheV1::build(
        dataset,
        &authority.source_columns,
        &orientation_rows,
        &should_cancel,
    )?;
    let pooled_indices = checked_source_rows_v1(dataset, &orientation_rows)
        .map_err(|failure| MultiModRunnerErrorV1::PreparedInput(failure.detail))?;
    let pooled_dataset =
        resample_dataset_columns_v1(dataset, &authority.source_columns, &pooled_indices, || {
            should_cancel()
        })
        .map_err(|error| match error {
            EstimationError::Cancelled => MultiModRunnerErrorV1::Cancelled,
            other => MultiModRunnerErrorV1::Kernel(format!(
                "multimod.runner.mga.hoc_pooled_dataset_failed:{other}"
            )),
        })?;
    report(
        &progress,
        MultiModRunnerPhaseV1::PointEstimation,
        0,
        1,
        "mga:hoc:pooled_reference",
    );
    let pooled_positions = (0..orientation_rows.len()).collect::<Vec<_>>();
    let pooled_fit = hoc_mga_fit_sample_v1(
        &pooled_dataset,
        &authority,
        None,
        &pooled_positions,
        None,
        &should_cancel,
    )
    .map_err(|failure| match failure.code {
        RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
        _ => MultiModRunnerErrorV1::Kernel(format!(
            "multimod.runner.mga.hoc_pooled_fit_failed:{}",
            failure.detail
        )),
    })?;
    let pairs = selected_mga_pairs(config)?;
    let execution_plan = if execution_cache.is_some() {
        let plan = build_mga_execution_plan_v1(
            artifact.receipt(),
            &artifact.receipt().dataset_fingerprint,
            config,
            design,
            &parameters,
            &pairs,
        )
        .map_err(map_mga_execution_cache_error_v1)?;
        if execution_cache
            .as_deref()
            .expect("raw HOC cache presence checked above")
            .plan()
            .plan_sha256
            != plan.plan_sha256
        {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "validated session plan differs from the reconstructed HOC plan".into(),
            ));
        }
        Some(plan)
    } else {
        None
    };
    let rows_by_group = observed_rows_by_group_v1(design);
    let uses_pairwise_partition_plan = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::MicomPairwise | MgaProcedureV1::PairwisePermutation
        )
    });
    let pairwise_partition_plans = if uses_pairwise_partition_plan {
        pairs
            .iter()
            .map(|pair| {
                build_pairwise_partition_plan_v1(
                    design,
                    *pair,
                    config.permutation_samples as usize,
                    config.seed,
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let micom_total = if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        pairs.len() as u64 * ((u64::from(config.permutation_samples) + 1) * 2 + 1)
    } else {
        0
    };
    let mut refitter = HocMgaRefitterV1 {
        dataset,
        authority,
        projections: projections.clone(),
        orientation_rows,
        orientation_positions,
        raw_scores,
        pooled_fit,
        should_cancel: &should_cancel,
        progress: &progress,
        micom_completed: 0,
        micom_total,
    };
    let mut observed_group_parameters = Vec::with_capacity(design.groups.len());
    for group in &design.groups {
        report(
            &progress,
            MultiModRunnerPhaseV1::PointEstimation,
            observed_group_parameters.len() as u64,
            design.groups.len() as u64,
            format!("mga:hoc:observed:g{}", group.index.get()),
        );
        let compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
            let fit = refitter
                .fit_rows(&rows_by_group[&group.index])
                .map_err(|failure| match failure.code {
                    RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
                    _ => MultiModRunnerErrorV1::Kernel(format!(
                        "multimod.runner.mga.hoc_observed_fit_failed:{}",
                        failure.detail
                    )),
                })?;
            let vector = refitter
                .parameter_vector(&fit)
                .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.detail))?;
            Ok(MgaExecutionShardPayloadV1::PointFit {
                value: GroupParameterVectorV1 {
                    group: group.index,
                    values: vector
                        .parameters
                        .into_iter()
                        .map(|parameter| parameter.estimate)
                        .collect(),
                },
                ordinary_path_standard_errors: Vec::new(),
            })
        };
        let payload =
            if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::PointFit { group: group.index },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
        let MgaExecutionShardPayloadV1::PointFit { value, .. } = payload else {
            unreachable!("cache validates HOC point payload against shard kind")
        };
        observed_group_parameters.push(value);
    }

    let mut micom_results = Vec::new();
    let mut micom_public = Vec::new();
    let mut micom_ledgers = Vec::new();
    let mut micom_partition_plan_receipts = BTreeMap::new();
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        let construct_ids = refitter
            .authority
            .final_blocks
            .iter()
            .map(|block| block.construct_id.clone())
            .collect::<Vec<_>>();
        let receipt = MicomConfiguralReceiptV1 {
            identical_indicators_and_coding: config
                .configural_checklist
                .identical_indicators_and_coding,
            identical_data_treatment: config.configural_checklist.identical_data_treatment,
            identical_algorithm_settings: config.configural_checklist.identical_algorithm_settings,
            identical_model_specification: config
                .configural_checklist
                .identical_model_specification,
            deterministic_orientation_reviewed: config
                .configural_checklist
                .deterministic_sign_orientation_reviewed,
            analyst_review_confirmed: config.configural_checklist.analyst_review_confirmed,
        };
        for pair in &pairs {
            let partition_plan = pairwise_partition_plans
                .iter()
                .find(|plan| {
                    plan.pair.group_a == pair.group_a.min(pair.group_b)
                        && plan.pair.group_b == pair.group_a.max(pair.group_b)
                })
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::InvalidLedger(format!(
                        "shared multiple-HOC partition plan is missing for {}",
                        pairwise_plan_key_v1(*pair)
                    ))
                })?;
            let micom_config = MicomPermutationConfigV1 {
                requested: config.permutation_samples as usize,
                seed: config.seed,
                alpha: config.alpha,
            };
            let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
                let result = run_pairwise_micom_with_partition_plan_v1(
                    &mut refitter,
                    *pair,
                    &rows_by_group,
                    &design.rows,
                    &construct_ids,
                    receipt.clone(),
                    micom_config.clone(),
                    partition_plan,
                    || should_cancel(),
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(
                        "multiple-HOC MICOM changed the shared partition-plan identity".into(),
                    ));
                }
                let rows = micom_public_rows_v1(config, &result)?;
                Ok(MgaExecutionShardPayloadV1::MicomPair {
                    value: result,
                    rows,
                })
            };
            let payload = if let (Some(_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::MicomPair { pair: *pair },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
            let MgaExecutionShardPayloadV1::MicomPair {
                value: result,
                rows,
            } = payload
            else {
                unreachable!("cache validates HOC MICOM payload against shard kind")
            };
            if rows != micom_public_rows_v1(config, &result)? {
                return Err(cached_payload_error_v1(
                    "multiple-HOC MICOM public rows differ from their retained kernel result",
                ));
            }
            validate_cached_micom_result_v1(
                &result,
                MICOM_PAIRWISE_METHOD_VERSION_V1,
                *pair,
                &construct_ids,
                &receipt,
                &micom_config,
                &partition_plan.plan_sha256,
                partition_plan
                    .entries
                    .iter()
                    .map(|entry| (entry.replicate, entry.partition_sha256.as_str())),
            )?;
            if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "multiple-HOC MICOM changed the shared partition-plan identity".into(),
                ));
            }
            micom_partition_plan_receipts.insert(
                pairwise_plan_key_v1(*pair),
                result.partition_plan_sha256.clone(),
            );
            micom_public.extend(rows);
            micom_ledgers.push(micom_ledger_summary_v1(&result));
            micom_results.push(result);
        }
    }
    let comparability = comparable_ordinary_pls_targets_v1(
        &projections,
        &micom_results,
        &pairs,
        config.procedures.contains(&MgaProcedureV1::MicomPairwise),
    )?;
    let prepared = PreparedMgaExecutionV1 {
        design: design.clone(),
        parameters,
        refit_receipt: PreparedMgaRefitReceiptV1 {
            complete_model_refit_per_request: true,
            deterministic_sign_orientation: true,
            interaction_products_rebuilt_per_request: false,
            hoc_dependency_stages_refit_per_request: true,
            plsc_correction_repeated_per_request: false,
            positive_case_weights_applied_per_request: false,
            integer_frequency_count_space_equivalent: false,
        },
        observed_group_parameters,
        pairwise_partition_plans: pairwise_partition_plans.clone(),
        micom_partition_plan_receipts,
        micom_pairs: micom_public,
        comparable_target_ids_by_canonical_pair: comparability.by_canonical_pair,
        comparable_target_ids: comparability.all_pairs,
        parametric_cells: Vec::new(),
        excluded_rows: excluded_rows.to_vec(),
    };
    let mut output = run_compiled_mga_multigroup_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        &prepared,
        &mut refitter,
        None,
        execution_cache.as_deref_mut(),
        checkpoint.as_deref_mut(),
        &should_cancel,
        &progress,
    )?;
    if let MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(analysis) = &mut output.result {
        analysis.replicate_ledgers.extend(micom_ledgers);
    }
    output.evidence.extend(
        micom_results
            .into_iter()
            .map(MultiModRunnerEvidenceV1::MgaMicomPair),
    );
    output
        .result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    Ok(output)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedMgaRefitReceiptV1 {
    pub complete_model_refit_per_request: bool,
    pub deterministic_sign_orientation: bool,
    pub interaction_products_rebuilt_per_request: bool,
    pub hoc_dependency_stages_refit_per_request: bool,
    pub plsc_correction_repeated_per_request: bool,
    pub positive_case_weights_applied_per_request: bool,
    pub integer_frequency_count_space_equivalent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedMgaParametricCellV1 {
    pub parameter: ParameterIdentityV1,
    pub group_estimates: Vec<ParametricGroupEstimateV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedMgaExecutionV1 {
    pub design: MultigroupDesignV1,
    pub parameters: Vec<ParameterIdentityV1>,
    pub refit_receipt: PreparedMgaRefitReceiptV1,
    /// Optional precomputed observed fits. They are mandatory only when the
    /// selected procedures do not otherwise execute an observed group fit.
    #[serde(default)]
    pub observed_group_parameters: Vec<GroupParameterVectorV1>,
    /// Optional immutable plans for pairwise procedures. When MICOM is
    /// selected, every requested pair requires a matching plan and receipt so
    /// its partitions can be proven identical to permutation MGA.
    #[serde(default)]
    pub pairwise_partition_plans: Vec<PairwisePartitionPlanV1>,
    #[serde(default)]
    pub micom_partition_plan_receipts: BTreeMap<String, String>,
    #[serde(default)]
    pub micom_pairs: Vec<MicomPairResultV1>,
    /// Targets satisfying the configured comparability gate for each exact
    /// unordered group pair. Keys use `multimod_mga_canonical_pair_key_v1`, so
    /// reversing the reported A-minus-B direction cannot change the gate.
    #[serde(default)]
    pub comparable_target_ids_by_canonical_pair: BTreeMap<String, BTreeSet<String>>,
    /// Intersection of the pair-specific sets. This is retained only as the
    /// all-pairs authority for K-group interpretation; A/B result rows must use
    /// `comparable_target_ids_by_canonical_pair`.
    #[serde(default)]
    pub comparable_target_ids: BTreeSet<String>,
    #[serde(default)]
    pub parametric_cells: Vec<PreparedMgaParametricCellV1>,
    #[serde(default)]
    pub excluded_rows: Vec<ExcludedRowReceiptV1>,
}

#[derive(Debug, Clone)]
struct MgaKernelOverridesV1 {
    frequency_pairwise_plans: Vec<FrequencyPairwisePartitionPlanV1>,
    pairwise_permutations: BTreeMap<String, PairwisePermutationResultV1>,
    omnibus_permutation: Option<OmnibusPermutationResultV1>,
    bootstrap_banks: Option<GroupBootstrapBanksV1>,
    eligibility: MultigroupEligibilityV1,
}

impl MgaKernelOverridesV1 {
    fn pairwise(&self, pair: OrderedGroupPairV1) -> Option<&PairwisePermutationResultV1> {
        self.pairwise_permutations.get(&pairwise_plan_key_v1(pair))
    }

    fn frequency_plan(
        &self,
        pair: OrderedGroupPairV1,
    ) -> Option<&FrequencyPairwisePartitionPlanV1> {
        let pair = if pair.group_a < pair.group_b {
            pair
        } else {
            OrderedGroupPairV1 {
                group_a: pair.group_b,
                group_b: pair.group_a,
            }
        };
        self.frequency_pairwise_plans
            .iter()
            .find(|plan| plan.pair == pair)
    }
}

fn validate_frequency_kernel_overrides_v1(
    config: &MgaMultigroupV1,
    pairs: &[OrderedGroupPairV1],
    prepared: &PreparedMgaExecutionV1,
    overrides: &MgaKernelOverridesV1,
) -> Result<(), MultiModRunnerErrorV1> {
    if config.profile != qpls_core::MgaModelProfileV1::FrequencyWeightedPls
        || !overrides.eligibility.eligible
        || overrides.eligibility.group_counts.len() != prepared.design.groups.len()
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency kernel override has the wrong profile or eligibility inventory".into(),
        ));
    }
    let permutation = config
        .procedures
        .contains(&MgaProcedureV1::PairwisePermutation);
    if permutation != !overrides.pairwise_permutations.is_empty()
        || (permutation && overrides.pairwise_permutations.len() != pairs.len())
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency pairwise result inventory differs from the selected procedures".into(),
        ));
    }
    for pair in pairs {
        let Some(plan) = overrides.frequency_plan(*pair) else {
            if config.procedures.iter().any(|procedure| {
                matches!(
                    procedure,
                    MgaProcedureV1::PairwisePermutation | MgaProcedureV1::MicomPairwise
                )
            }) {
                return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                    "frequency count-space plan is missing for {}",
                    pairwise_plan_key_v1(*pair)
                )));
            }
            continue;
        };
        if let Some(result) = overrides.pairwise(*pair)
            && (result.pair != *pair
                || result.seed != config.seed
                || result.requested != config.permutation_samples as usize
                || result.plan_sha256 != plan.plan_sha256
                || result.point_estimates.len() != prepared.parameters.len()
                || result
                    .point_estimates
                    .iter()
                    .zip(&prepared.parameters)
                    .any(|(actual, expected)| actual.parameter != *expected)
                || (result.availability == InferenceAvailabilityV1::Available
                    && (result.parameters.len() != prepared.parameters.len()
                        || result
                            .parameters
                            .iter()
                            .zip(&prepared.parameters)
                            .any(|(actual, expected)| actual.parameter != *expected))))
        {
            return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                "frequency pairwise result authority differs for {}",
                pairwise_plan_key_v1(*pair)
            )));
        }
    }
    let omnibus_selected = config
        .procedures
        .contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation);
    if omnibus_selected != overrides.omnibus_permutation.is_some() {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency omnibus result presence differs from the selected procedure".into(),
        ));
    }
    if let Some(result) = &overrides.omnibus_permutation
        && (result.seed != config.seed
            || result.requested != config.permutation_samples as usize
            || result.group_point_estimates.len() != prepared.design.groups.len()
            || (result.availability == InferenceAvailabilityV1::Available
                && (result.parameters.len() != prepared.parameters.len()
                    || result
                        .parameters
                        .iter()
                        .zip(&prepared.parameters)
                        .any(|(actual, expected)| actual.parameter != *expected))))
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency omnibus result authority differs from the compiled request".into(),
        ));
    }
    let bootstrap_selected = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::HenselerPlsMga | MgaProcedureV1::BootstrapDifferenceBc
        )
    });
    if bootstrap_selected != overrides.bootstrap_banks.is_some() {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency bootstrap-bank presence differs from the selected procedures".into(),
        ));
    }
    if let Some(banks) = &overrides.bootstrap_banks
        && (banks.seed != config.seed
            || banks.requested != config.bootstrap_samples as usize
            || banks.parameters != prepared.parameters
            || banks.groups.len() != prepared.design.groups.len())
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency bootstrap-bank authority differs from the compiled request".into(),
        ));
    }
    Ok(())
}

fn observed_rows_by_group_v1(design: &MultigroupDesignV1) -> BTreeMap<GroupIndexV1, Vec<u64>> {
    let mut rows = design
        .groups
        .iter()
        .map(|group| (group.index, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let mut selected_rows = design.rows.clone();
    selected_rows.sort_by_key(|row| row.stable_row_token);
    for selected in &selected_rows {
        if let Some(group_rows) = rows.get_mut(&selected.group) {
            group_rows.push(selected.source_row);
        }
    }
    rows
}

fn pairwise_plan_key_v1(pair: OrderedGroupPairV1) -> String {
    multimod_mga_canonical_pair_key_v1(pair)
}

/// Stable serialized key for pair-specific MGA authority. The key is
/// invariant to A/B reporting direction.
pub fn multimod_mga_canonical_pair_key_v1(pair: OrderedGroupPairV1) -> String {
    let (low, high) = if pair.group_a < pair.group_b {
        (pair.group_a, pair.group_b)
    } else {
        (pair.group_b, pair.group_a)
    };
    format!("g{}:g{}", low.get(), high.get())
}

fn prepared_pair_comparable_targets_v1<'a>(
    prepared: &'a PreparedMgaExecutionV1,
    pair: OrderedGroupPairV1,
) -> Result<&'a BTreeSet<String>, MultiModRunnerErrorV1> {
    prepared
        .comparable_target_ids_by_canonical_pair
        .get(&pairwise_plan_key_v1(pair))
        .ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "pair-specific comparability authority is missing for {}",
                pairwise_plan_key_v1(pair)
            ))
        })
}

fn validate_prepared_pair_comparability_v1(
    config: &MgaMultigroupV1,
    pairs: &[OrderedGroupPairV1],
    prepared: &PreparedMgaExecutionV1,
) -> Result<(), MultiModRunnerErrorV1> {
    let target_ids = prepared
        .parameters
        .iter()
        .map(|parameter| parameter.stable_id.clone())
        .collect::<BTreeSet<_>>();
    let expected_pair_keys = pairs
        .iter()
        .map(|pair| pairwise_plan_key_v1(*pair))
        .collect::<BTreeSet<_>>();
    let actual_pair_keys = prepared
        .comparable_target_ids_by_canonical_pair
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if actual_pair_keys != expected_pair_keys
        || prepared
            .comparable_target_ids_by_canonical_pair
            .values()
            .any(|ids| !ids.is_subset(&target_ids))
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "pair-specific comparability authority must contain exactly the selected canonical pairs and selected targets"
                .into(),
        ));
    }
    let all_pairs = target_ids
        .iter()
        .filter(|target_id| {
            prepared
                .comparable_target_ids_by_canonical_pair
                .values()
                .all(|ids| ids.contains(*target_id))
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    if prepared.comparable_target_ids != all_pairs {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "global MGA comparability authority must equal the exact intersection of every selected pair"
                .into(),
        ));
    }

    let micom_selected = config.procedures.contains(&MgaProcedureV1::MicomPairwise);
    if micom_selected {
        let group_index_by_id = config
            .groups
            .iter()
            .enumerate()
            .map(|(index, group)| {
                Ok((
                    group.group_id.as_str(),
                    GroupIndexV1::new(index)
                        .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, MultiModRunnerErrorV1>>()?;
        let mut public_pair_keys = BTreeSet::new();
        for row in &prepared.micom_pairs {
            let left = group_index_by_id
                .get(row.left_group_id.as_str())
                .copied()
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "MICOM public comparability row references an unknown left group".into(),
                    )
                })?;
            let right = group_index_by_id
                .get(row.right_group_id.as_str())
                .copied()
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "MICOM public comparability row references an unknown right group".into(),
                    )
                })?;
            if left == right {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "MICOM public comparability row cannot compare a group with itself".into(),
                ));
            }
            let key = pairwise_plan_key_v1(OrderedGroupPairV1 {
                group_a: left,
                group_b: right,
            });
            if !expected_pair_keys.contains(&key) {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "MICOM public comparability row references an unselected canonical pair".into(),
                ));
            }
            public_pair_keys.insert(key);
        }
        if public_pair_keys != expected_pair_keys {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "MICOM public comparability rows must cover every selected canonical pair".into(),
            ));
        }
    } else if !prepared.micom_pairs.is_empty()
        || prepared
            .comparable_target_ids_by_canonical_pair
            .values()
            .any(|ids| !ids.is_empty())
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "comparability cannot be satisfied without the selected MICOM procedure".into(),
        ));
    }
    Ok(())
}

fn validate_pairwise_result_comparability_v1(
    config: &MgaMultigroupV1,
    prepared: &PreparedMgaExecutionV1,
    rows: &[MgaPairwiseComparisonV1],
) -> Result<(), MultiModRunnerErrorV1> {
    let group_index_by_id = config
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            Ok((
                group.group_id.as_str(),
                GroupIndexV1::new(index)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, MultiModRunnerErrorV1>>()?;
    for row in rows {
        let left = group_index_by_id
            .get(row.left_group_id.as_str())
            .copied()
            .ok_or_else(|| {
                MultiModRunnerErrorV1::ResultContract(
                    "pairwise result comparability row references an unknown left group".into(),
                )
            })?;
        let right = group_index_by_id
            .get(row.right_group_id.as_str())
            .copied()
            .ok_or_else(|| {
                MultiModRunnerErrorV1::ResultContract(
                    "pairwise result comparability row references an unknown right group".into(),
                )
            })?;
        if left == right {
            return Err(MultiModRunnerErrorV1::ResultContract(
                "pairwise result comparability row cannot compare a group with itself".into(),
            ));
        }
        let comparable = prepared_pair_comparable_targets_v1(
            prepared,
            OrderedGroupPairV1 {
                group_a: left,
                group_b: right,
            },
        )?
        .contains(&row.target_id);
        if row.measurement_comparability_satisfied != comparable
            || row.interpretation_blocked == comparable
        {
            return Err(MultiModRunnerErrorV1::ResultContract(format!(
                "pairwise result comparability differs from canonical pair authority for {}:{}",
                pairwise_plan_key_v1(OrderedGroupPairV1 {
                    group_a: left,
                    group_b: right,
                }),
                row.target_id
            )));
        }
    }
    Ok(())
}

fn prepared_pairwise_plan_v1<'a>(
    prepared: &'a PreparedMgaExecutionV1,
    pair: OrderedGroupPairV1,
) -> Option<&'a PairwisePartitionPlanV1> {
    let (low, high) = if pair.group_a < pair.group_b {
        (pair.group_a, pair.group_b)
    } else {
        (pair.group_b, pair.group_a)
    };
    prepared
        .pairwise_partition_plans
        .iter()
        .find(|plan| plan.pair.group_a == low && plan.pair.group_b == high)
}

fn validate_prepared_group_membership_v1(
    dataset: &Dataset,
    config: &MgaMultigroupV1,
    design: &MultigroupDesignV1,
) -> Result<(), MultiModRunnerErrorV1> {
    let metadata = dataset
        .schema
        .columns
        .iter()
        .find(|column| column.name == config.grouping_column)
        .ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(
                "the configured grouping column is absent from the execution dataset".into(),
            )
        })?;
    let rows = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
    for selected in &design.rows {
        let configured = config.groups.get(selected.group.get()).ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(
                "prepared row references a group outside the configured inventory".into(),
            )
        })?;
        let displayed = rows
            .get(selected.source_row as usize)
            .and_then(|row| row.get(&config.grouping_column))
            .and_then(Option::as_deref)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "selected source row {} has a missing grouping value",
                    selected.source_row
                ))
            })?;
        let matches = match (&configured.value, metadata.column_type) {
            (qpls_core::TypedGroupValueV1::Text { value }, qpls_data::ColumnType::Text) => {
                displayed == value
            }
            (qpls_core::TypedGroupValueV1::Boolean { value }, qpls_data::ColumnType::Boolean) => {
                displayed.parse::<bool>().ok() == Some(*value)
            }
            (qpls_core::TypedGroupValueV1::Integer { value }, qpls_data::ColumnType::Numeric) => {
                displayed.parse::<i64>().ok() == Some(*value)
            }
            (qpls_core::TypedGroupValueV1::Number { value }, qpls_data::ColumnType::Numeric) => {
                displayed
                    .parse::<f64>()
                    .ok()
                    .is_some_and(|actual| actual.to_bits() == value.to_bits())
            }
            _ => false,
        };
        if !matches {
            return Err(MultiModRunnerErrorV1::PreparedInput(format!(
                "selected source row {} does not belong to configured group {} in the raw grouping column",
                selected.source_row, configured.group_id
            )));
        }
    }
    Ok(())
}

fn frequency_multigroup_design_from_raw_v1(
    dataset: &Dataset,
    weight_column: &str,
    design: &MultigroupDesignV1,
) -> Result<(FrequencyMultigroupDesignV1, Vec<u64>, Vec<u64>), MultiModRunnerErrorV1> {
    let mut canonical_rows = design
        .rows
        .iter()
        .map(|row| row.source_row)
        .collect::<Vec<_>>();
    canonical_rows.sort_unstable();
    canonical_rows.dedup();
    if canonical_rows.len() != design.rows.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "frequency MGA design contains duplicate source rows".into(),
        ));
    }
    let counts =
        multimod_frequency_counts_for_source_rows_v1(dataset, weight_column, &canonical_rows)
            .map_err(|error| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "multimod.runner.mga.frequency_count_contract:{error}"
                ))
            })?;
    let count_by_row = canonical_rows
        .iter()
        .copied()
        .zip(counts.iter().copied())
        .collect::<BTreeMap<_, _>>();
    let frequency_design = FrequencyMultigroupDesignV1 {
        groups: design.groups.clone(),
        rows: design
            .rows
            .iter()
            .map(|row| {
                Ok(FrequencySelectedGroupRowV1 {
                    source_row: row.source_row,
                    stable_row_token: row.stable_row_token,
                    group: row.group,
                    frequency: *count_by_row.get(&row.source_row).ok_or_else(|| {
                        MultiModRunnerErrorV1::PreparedInput(
                            "frequency row is absent from the canonical count inventory".into(),
                        )
                    })?,
                })
            })
            .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?,
    };
    Ok((frequency_design, canonical_rows, counts))
}

fn pooled_ordinary_pls_fit_v1<C>(
    dataset: &Dataset,
    source_columns: &[String],
    source_rows: &[u64],
    point_recipe: &AnalysisRecipeV4,
    point_model: &SemModelV4,
    point_artifact: &qpls_core::CompiledAnalysisRecipeV4,
    profile: qpls_core::MgaModelProfileV1,
    weight_column: Option<&str>,
    should_cancel: &C,
) -> Result<PlsResult, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    let indices = checked_source_rows_v1(dataset, source_rows)
        .map_err(|failure| MultiModRunnerErrorV1::PreparedInput(failure.detail))?;
    let sampled =
        resample_dataset_columns_v1(dataset, source_columns, &indices, || should_cancel())
            .map_err(|error| match error {
                EstimationError::Cancelled => MultiModRunnerErrorV1::Cancelled,
                other => MultiModRunnerErrorV1::Kernel(other.to_string()),
            })?;
    let sampled = if profile == qpls_core::MgaModelProfileV1::CaseWeightedPls {
        prepare_multimod_case_weight_dataset_v1(
            &sampled,
            weight_column.ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(
                    "case-weighted pooled fit omitted its weight column".into(),
                )
            })?,
        )
        .map_err(|error| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "case-weight normalization failed for the pooled fit: {error}"
            ))
        })?
        .0
    } else {
        sampled
    };
    let result = run_compiled_pls_recipe_v4(
        &sampled,
        point_recipe,
        point_model,
        point_artifact,
        None,
        || should_cancel(),
        |_| {},
    )
    .map_err(|error| match error {
        crate::RecipeV4PlsExecutionError::Cancelled => MultiModRunnerErrorV1::Cancelled,
        other => MultiModRunnerErrorV1::Kernel(format!(
            "multimod.runner.mga.pooled_reference_fit_failed: {other}"
        )),
    })?
    .estimation()
    .clone();
    if !result.converged
        || result.used_observations != source_rows.len()
        || result.omitted_observations != 0
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "the selected MGA row universe is not a complete, converged pooled PLS sample".into(),
        ));
    }
    Ok(result)
}

fn pooled_mga_point_fit_v1<C>(
    dataset: &Dataset,
    source_columns: &[String],
    source_rows: &[u64],
    authority: &OrdinaryPlsPointAuthorityV1,
    profile: qpls_core::MgaModelProfileV1,
    weight_column: Option<&str>,
    should_cancel: &C,
) -> Result<PlsResult, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    let indices = checked_source_rows_v1(dataset, source_rows)
        .map_err(|failure| MultiModRunnerErrorV1::PreparedInput(failure.detail))?;
    let sampled =
        resample_dataset_columns_v1(dataset, source_columns, &indices, || should_cancel())
            .map_err(|error| match error {
                EstimationError::Cancelled => MultiModRunnerErrorV1::Cancelled,
                other => MultiModRunnerErrorV1::Kernel(other.to_string()),
            })?;
    let sampled = if profile == qpls_core::MgaModelProfileV1::CaseWeightedPls {
        prepare_multimod_case_weight_dataset_v1(
            &sampled,
            weight_column.ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(
                    "case-weighted pooled fit omitted its weight column".into(),
                )
            })?,
        )
        .map_err(|error| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "case-weight normalization failed for the pooled fit: {error}"
            ))
        })?
        .0
    } else {
        sampled
    };
    let result = authority
        .execute(&sampled, should_cancel)
        .map_err(|failure| match failure.code {
            RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
            _ => MultiModRunnerErrorV1::Kernel(format!(
                "multimod.runner.mga.pooled_reference_fit_failed: {}",
                failure.detail
            )),
        })?;
    if !result.converged
        || result.used_observations != source_rows.len()
        || result.omitted_observations != 0
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "the selected MGA row universe is not a complete, converged pooled PLS sample".into(),
        ));
    }
    Ok(result)
}

fn micom_public_rows_v1(
    config: &MgaMultigroupV1,
    result: &MicomPairwiseResultV1,
) -> Result<Vec<MicomPairResultV1>, MultiModRunnerErrorV1> {
    if !result.complete {
        return Err(MultiModRunnerErrorV1::Kernel(format!(
            "MICOM pair {}-{} has {} usable permutations; {} required",
            result.pair.group_a.get(),
            result.pair.group_b.get(),
            result.usable_permutations,
            result.minimum_usable_permutations
        )));
    }
    let left_group_id = config.groups[result.pair.group_a.get()].group_id.clone();
    let right_group_id = config.groups[result.pair.group_b.get()].group_id.clone();
    result
        .constructs
        .iter()
        .map(|construct| {
            Ok(MicomPairResultV1 {
                left_group_id: left_group_id.clone(),
                right_group_id: right_group_id.clone(),
                construct_id: construct.construct_id.clone(),
                interpretation: MicomInvarianceInterpretationV1::CompositeInvariance,
                configural_invariance_confirmed: result.configural_receipt.complete(),
                compositional_correlation: construct.observed_compositional_correlation,
                compositional_lower_quantile: construct.compositional_lower_quantile.ok_or_else(
                    || {
                        MultiModRunnerErrorV1::Kernel(
                            "complete MICOM result omitted its Step-2 lower quantile".into(),
                        )
                    },
                )?,
                compositional_p_value: construct.compositional_invariance_probability.ok_or_else(
                    || {
                        MultiModRunnerErrorV1::Kernel(
                            "complete MICOM result omitted its Step-2 directional probability"
                                .into(),
                        )
                    },
                )?,
                compositional_invariance: construct.compositional_invariance,
                partial_invariance: construct.partial_measurement_invariance,
                equal_mean_p_value: construct.mean_difference_two_sided_probability.ok_or_else(
                    || {
                        MultiModRunnerErrorV1::Kernel(
                            "complete MICOM result omitted its Step-3 mean probability".into(),
                        )
                    },
                )?,
                equal_variance_p_value: construct
                    .variance_difference_two_sided_probability
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::Kernel(
                            "complete MICOM result omitted its Step-3 variance probability".into(),
                        )
                    })?,
            })
        })
        .collect()
}

fn validate_cached_micom_result_v1<'a>(
    result: &MicomPairwiseResultV1,
    expected_method_version: &str,
    pair: OrderedGroupPairV1,
    construct_ids: &[String],
    receipt: &MicomConfiguralReceiptV1,
    config: &MicomPermutationConfigV1,
    expected_plan_sha256: &str,
    expected_partitions: impl ExactSizeIterator<Item = (usize, &'a str)>,
) -> Result<(), MultiModRunnerErrorV1> {
    let usable = result
        .ledger
        .iter()
        .filter(|entry| matches!(&entry.status, MicomPermutationStatusV1::Usable))
        .count();
    let expected_partition_count = expected_partitions.len();
    let ledger_valid = result
        .ledger
        .iter()
        .zip(expected_partitions)
        .enumerate()
        .all(
            |(replicate, (entry, (planned_replicate, planned_sha256)))| {
                entry.replicate == replicate
                    && planned_replicate == replicate
                    && entry.seed == config.seed
                    && entry.partition_sha256 == planned_sha256
            },
        );
    let constructs_valid = result.constructs.len() == construct_ids.len()
        && result
            .constructs
            .iter()
            .zip(construct_ids)
            .all(|(construct, expected_id)| {
                let Some(lower) = construct.compositional_lower_quantile else {
                    return false;
                };
                let Some(compositional_probability) =
                    construct.compositional_invariance_probability
                else {
                    return false;
                };
                let Some(mean_probability) = construct.mean_difference_two_sided_probability else {
                    return false;
                };
                let Some(variance_probability) =
                    construct.variance_difference_two_sided_probability
                else {
                    return false;
                };
                construct.construct_id == *expected_id
                    && construct.observed_compositional_correlation.is_finite()
                    && (-1.0..=1.0).contains(&construct.observed_compositional_correlation)
                    && lower.is_finite()
                    && (-1.0..=1.0).contains(&lower)
                    && construct.observed_mean_difference_a_minus_b.is_finite()
                    && construct.observed_log_variance_ratio_a_minus_b.is_finite()
                    && [
                        compositional_probability,
                        mean_probability,
                        variance_probability,
                    ]
                    .into_iter()
                    .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
                    && construct.compositional_invariance
                        == (construct.observed_compositional_correlation >= lower)
                    && construct.equal_means == (mean_probability >= config.alpha)
                    && construct.equal_variances == (variance_probability >= config.alpha)
                    && construct.partial_measurement_invariance
                        == (receipt.complete() && construct.compositional_invariance)
                    && construct.full_measurement_invariance
                        == (construct.partial_measurement_invariance
                            && construct.equal_means
                            && construct.equal_variances)
            });
    if result.method_version != expected_method_version
        || result.pair != pair
        || result.configural_receipt != *receipt
        || result.requested_permutations != config.requested
        || result.minimum_usable_permutations != config.minimum_usable()
        || result.usable_permutations != usable
        || result.ledger.len() != config.requested
        || expected_partition_count != config.requested
        || result.partition_plan_sha256 != expected_plan_sha256
        || result.ledger_sha256 != sha256_serialized(&result.ledger)
        || result.complete != (usable >= config.minimum_usable())
        || !result.complete
        || !ledger_valid
        || !constructs_valid
    {
        return Err(cached_payload_error_v1(
            "MICOM identity, ledger, construct inventory, or invariance decisions differ from the frozen pairwise request",
        ));
    }
    Ok(())
}

fn micom_ledger_summary_v1(result: &MicomPairwiseResultV1) -> MultimodReplicateLedgerSummaryV1 {
    let failures = result
        .ledger
        .iter()
        .filter_map(|entry| match &entry.status {
            MicomPermutationStatusV1::Usable => None,
            MicomPermutationStatusV1::Failed { code, .. } => Some(refit_failure_code(
                &RefitFailureV1::new(*code, "MICOM permutation refit failed"),
            )),
        });
    mga_ledger_summary(
        result.requested_permutations,
        result.usable_permutations,
        result.minimum_usable_permutations,
        &result.ledger,
        failures,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MgaComparabilityAuthorityV1 {
    by_canonical_pair: BTreeMap<String, BTreeSet<String>>,
    all_pairs: BTreeSet<String>,
}

fn build_mga_comparability_authority_v1(
    target_requirements: &[(String, BTreeSet<String>)],
    micom: &[MicomPairwiseResultV1],
    pairs: &[OrderedGroupPairV1],
    micom_selected: bool,
) -> Result<MgaComparabilityAuthorityV1, MultiModRunnerErrorV1> {
    let target_ids = target_requirements
        .iter()
        .map(|(target_id, _)| target_id.clone())
        .collect::<BTreeSet<_>>();
    if target_ids.len() != target_requirements.len()
        || target_requirements
            .iter()
            .any(|(target_id, required)| target_id.trim().is_empty() || required.is_empty())
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "MGA comparability target identities must be unique and require at least one construct"
                .into(),
        ));
    }
    let expected_pair_keys = pairs
        .iter()
        .map(|pair| pairwise_plan_key_v1(*pair))
        .collect::<BTreeSet<_>>();
    if expected_pair_keys.len() != pairs.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "MGA comparability pairs must be unique after canonicalization".into(),
        ));
    }

    let by_canonical_pair = if micom_selected {
        let mut micom_by_pair = BTreeMap::new();
        for result in micom {
            let key = pairwise_plan_key_v1(result.pair);
            let unique_constructs = result
                .constructs
                .iter()
                .map(|construct| construct.construct_id.as_str())
                .collect::<BTreeSet<_>>();
            if !result.complete
                || unique_constructs.len() != result.constructs.len()
                || !expected_pair_keys.contains(&key)
                || micom_by_pair.insert(key, result).is_some()
            {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "MICOM comparability authority is incomplete, duplicated, or references an unselected canonical pair"
                        .into(),
                ));
            }
        }
        if micom_by_pair.len() != expected_pair_keys.len() {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "MICOM comparability authority must contain exactly one complete result per selected canonical pair"
                    .into(),
            ));
        }
        pairs
            .iter()
            .map(|pair| {
                let key = pairwise_plan_key_v1(*pair);
                let result = micom_by_pair
                    .get(&key)
                    .expect("canonical MICOM inventory validated above");
                let comparable = target_requirements
                    .iter()
                    .filter(|(_, required)| {
                        required.iter().all(|construct_id| {
                            result.constructs.iter().any(|construct| {
                                construct.construct_id == *construct_id
                                    && construct.partial_measurement_invariance
                            })
                        })
                    })
                    .map(|(target_id, _)| target_id.clone())
                    .collect::<BTreeSet<_>>();
                (key, comparable)
            })
            .collect::<BTreeMap<_, _>>()
    } else {
        if !micom.is_empty() {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "MICOM results were supplied without selecting the MICOM procedure".into(),
            ));
        }
        pairs
            .iter()
            .map(|pair| (pairwise_plan_key_v1(*pair), BTreeSet::new()))
            .collect::<BTreeMap<_, _>>()
    };
    let all_pairs = target_ids
        .into_iter()
        .filter(|target_id| {
            by_canonical_pair
                .values()
                .all(|comparable| comparable.contains(target_id))
        })
        .collect();
    Ok(MgaComparabilityAuthorityV1 {
        by_canonical_pair,
        all_pairs,
    })
}

fn comparable_ordinary_pls_targets_v1(
    projections: &[OrdinaryPlsParameterProjectionV1],
    micom: &[MicomPairwiseResultV1],
    pairs: &[OrderedGroupPairV1],
    micom_selected: bool,
) -> Result<MgaComparabilityAuthorityV1, MultiModRunnerErrorV1> {
    let requirements = projections
        .iter()
        .map(|projection| {
            (
                projection.identity.stable_id.clone(),
                projection.required_constructs(),
            )
        })
        .collect::<Vec<_>>();
    build_mga_comparability_authority_v1(&requirements, micom, pairs, micom_selected)
}

/// Executes the raw-data ordinary General SEM PLS MGA profile without an
/// external scientific refitter. The caller supplies the already classified
/// complete-case group design so row exclusions retain their native stable
/// tokens; every observed, permutation, bootstrap, and MICOM fit is performed
/// here through the existing Recipe V4 PLS point-estimation boundary.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_ordinary_pls_mga_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_ordinary_pls_mga_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        excluded_rows,
        None,
        None,
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_compiled_ordinary_pls_mga_internal_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    mut execution_cache: Option<&mut ValidatedMgaExecutionCacheSessionV1<'_>>,
    mut checkpoint: Option<&mut MgaShardCheckpointCallbackV1<'_>>,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
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
        "mga:ordinary_pls:authority",
    );
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    if !matches!(
        config.profile,
        qpls_core::MgaModelProfileV1::GeneralSemPls
            | qpls_core::MgaModelProfileV1::ReflectivePlsc
            | qpls_core::MgaModelProfileV1::CaseWeightedPls
    ) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.base_pls_profile_required: this raw refitter accepts general_sem_pls, reflective_plsc, or case_weighted_pls"
                .into(),
        ));
    }
    let eligibility = assess_multigroup_design_v1(design);
    if !eligibility.eligible {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "ordinary PLS MGA design is ineligible: {:?}",
            eligibility.blockers
        )));
    }
    validate_prepared_group_membership_v1(dataset, config, design)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::PreparingPointInputs,
        0,
        1,
        "mga:ordinary_pls:projection",
    );
    let authority = projected_ordinary_pls_authority_v1(recipe, model, config)?;
    let point_plan = authority.plan();
    let blocks = ordinary_pls_scoring_blocks_v1(point_plan);
    let projections = ordinary_pls_parameter_projections_with_technical_v1(
        config,
        authority.point_model(),
        point_plan,
        authority.technical_construct_ids(),
    )?;
    // The projected ordinary-PLS authority is the source of truth for both
    // publishable targets and MICOM constructs. Using the authored model's
    // deliberately loose upper bound here falsely rejected the admitted
    // 20-group/190-pair fixture even though its exact Arrow evidence remains
    // below the 512 MiB cap.
    let predicted_sidecar_bytes = predict_mga_sidecar_bytes_v1(
        config,
        &compact_group_row_counts_v1(design),
        projections.len(),
        projections
            .iter()
            .map(|projection| projection.identity.stable_id.len())
            .max()
            .unwrap_or(1),
        ordinary_pls_micom_construct_ids_v1(&blocks, authority.technical_construct_ids()).len(),
    );
    enforce_multimod_sidecar_cost_v1("mga", predicted_sidecar_bytes, &progress)?;
    let mut equation_predecessors = BTreeMap::<String, Vec<String>>::new();
    for path in point_plan.paths() {
        equation_predecessors
            .entry(path.target().to_owned())
            .or_default()
            .push(path.source().to_owned());
    }
    let needs_parametric = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::ParametricPooledVariance
                | MgaProcedureV1::ParametricWelchSatterthwaite
                | MgaProcedureV1::ParametricWaldOmnibus
        )
    });
    if needs_parametric
        && (config.selected_parameter_ids.is_empty()
            || projections.iter().any(|projection| {
                !matches!(
                    &projection.source,
                    OrdinaryPlsParameterSourceV1::StructuralPath { .. }
                )
            }))
    {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.parametric_structural_path_targets_only: parametric pooled/Welch/Wald sensitivity requires an explicit nonempty selection containing only ordinary PLS structural or control paths"
                .into(),
        ));
    }
    let parameters = projections
        .iter()
        .map(|projection| projection.identity.clone())
        .collect::<Vec<_>>();
    let mut source_columns = ordinary_pls_source_columns_v1(dataset, &blocks)?;
    let weight_column = match config.profile {
        qpls_core::MgaModelProfileV1::CaseWeightedPls => Some(
            authority
                .weight_source_column()
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::Authority(
                        "case-weighted MGA lost its resolved weight source column".into(),
                    )
                })?
                .to_owned(),
        ),
        _ => None,
    };
    if let Some(column) = &weight_column {
        if !source_columns.contains(column) {
            source_columns.push(column.clone());
        }
    }
    let mut orientation_rows = design
        .rows
        .iter()
        .map(|row| row.source_row)
        .collect::<Vec<_>>();
    orientation_rows.sort_unstable();
    orientation_rows.dedup();
    if orientation_rows.len() != design.rows.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "ordinary PLS MGA design contains duplicate source rows".into(),
        ));
    }
    let raw_scores = OrdinaryPlsRawScoreCacheV1::build(
        dataset,
        &source_columns,
        &orientation_rows,
        &should_cancel,
    )?;
    let case_weights_by_row = if config.profile == qpls_core::MgaModelProfileV1::CaseWeightedPls {
        let column = weight_column.as_deref().ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "multimod.runner.mga.case_weight_column_missing: case-weighted PLS requires a bound weight column"
                    .into(),
            )
        })?;
        let weights = multimod_case_weights_for_source_rows_v1(dataset, column, &orientation_rows)
            .map_err(|error| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "multimod.runner.mga.case_weight_unusable: {error}"
                ))
            })?;
        Some(
            orientation_rows
                .iter()
                .copied()
                .zip(weights)
                .collect::<BTreeMap<_, _>>(),
        )
    } else {
        None
    };
    report(
        &progress,
        MultiModRunnerPhaseV1::PointEstimation,
        0,
        1,
        "mga:ordinary_pls:pooled_reference",
    );
    let pooled_fit = pooled_mga_point_fit_v1(
        dataset,
        &source_columns,
        &orientation_rows,
        &authority,
        config.profile,
        weight_column.as_deref(),
        &should_cancel,
    )?;
    let pairs = selected_mga_pairs(config)?;
    let execution_plan = if execution_cache.is_some() {
        let plan = build_mga_execution_plan_v1(
            artifact.receipt(),
            &artifact.receipt().dataset_fingerprint,
            config,
            design,
            &parameters,
            &pairs,
        )
        .map_err(map_mga_execution_cache_error_v1)?;
        if execution_cache
            .as_deref()
            .expect("raw ordinary cache presence checked above")
            .plan()
            .plan_sha256
            != plan.plan_sha256
        {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "validated session plan differs from the reconstructed ordinary plan".into(),
            ));
        }
        Some(plan)
    } else {
        None
    };
    let micom_total = if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        pairs.len() as u64 * ((u64::from(config.permutation_samples) + 1) * 2 + 1)
    } else {
        0
    };
    let rows_by_group = observed_rows_by_group_v1(design);
    let uses_pairwise_partition_plan = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::MicomPairwise | MgaProcedureV1::PairwisePermutation
        )
    });
    let pairwise_partition_plans = if uses_pairwise_partition_plan {
        pairs
            .iter()
            .map(|pair| {
                build_pairwise_partition_plan_v1(
                    design,
                    *pair,
                    config.permutation_samples as usize,
                    config.seed,
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let mut refitter = OrdinaryPlsMgaRefitterV1 {
        dataset,
        authority,
        source_columns,
        profile: config.profile,
        weight_column,
        blocks,
        projections: projections.clone(),
        orientation_rows,
        raw_scores,
        pooled_fit,
        should_cancel: &should_cancel,
        progress: &progress,
        micom_completed: 0,
        micom_total,
    };
    let mut observed_group_parameters = Vec::with_capacity(design.groups.len());
    let mut parametric_by_parameter =
        BTreeMap::<ParameterIdentityV1, Vec<ParametricGroupEstimateV1>>::new();
    let mut parametric_se_evidence = Vec::new();
    for group in &design.groups {
        report(
            &progress,
            MultiModRunnerPhaseV1::PointEstimation,
            observed_group_parameters.len() as u64,
            design.groups.len() as u64,
            format!("mga:ordinary_pls:observed:g{}", group.index.get()),
        );
        let compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
            let fit = refitter
                .fit_rows(&rows_by_group[&group.index])
                .map_err(|failure| match failure.code {
                    RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
                    _ => MultiModRunnerErrorV1::Kernel(format!(
                        "observed ordinary PLS group refit failed: {}",
                        failure.detail
                    )),
                })?;
            let vector = refitter
                .parameter_vector(&fit)
                .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.detail))?;
            let mut standard_errors = Vec::new();
            if needs_parametric {
                for projection in &projections {
                    let OrdinaryPlsParameterSourceV1::StructuralPath { source, target, .. } =
                        &projection.source
                    else {
                        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
                            "multimod.runner.mga.parametric_structural_path_targets_only".into(),
                        ));
                    };
                    let predecessor_ids = equation_predecessors.get(target).ok_or_else(|| {
                        MultiModRunnerErrorV1::UnsupportedProfile(format!(
                            "multimod.runner.mga.parametric_equation_missing:{}",
                            projection.identity.stable_id
                        ))
                    })?;
                    let receipt =
                        ordinary_pls_path_standard_error_v1(&fit, source, target, predecessor_ids)
                            .map_err(|error| {
                                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                                    "multimod.runner.mga.parametric_path_se_unavailable:{}:{error}",
                                    projection.identity.stable_id
                                ))
                            })?;
                    standard_errors.push(crate::MgaOrdinaryPointSeReceiptV1 {
                        parameter: projection.identity.clone(),
                        receipt,
                    });
                }
            }
            Ok(MgaExecutionShardPayloadV1::PointFit {
                value: GroupParameterVectorV1 {
                    group: group.index,
                    values: vector
                        .parameters
                        .into_iter()
                        .map(|parameter| parameter.estimate)
                        .collect(),
                },
                ordinary_path_standard_errors: standard_errors,
            })
        };
        let payload =
            if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::PointFit { group: group.index },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
        let MgaExecutionShardPayloadV1::PointFit {
            value,
            ordinary_path_standard_errors,
        } = payload
        else {
            unreachable!("cache validates point payload against shard kind")
        };
        let expected_standard_errors = if needs_parametric {
            projections.len()
        } else {
            0
        };
        if value.values.len() != parameters.len()
            || ordinary_path_standard_errors.len() != expected_standard_errors
        {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "ordinary point shard dimension or SE receipt inventory differs from the compiled target inventory"
                    .into(),
            ));
        }
        for item in ordinary_path_standard_errors {
            let receipt = item.receipt;
            parametric_by_parameter
                .entry(item.parameter.clone())
                .or_default()
                .push(ParametricGroupEstimateV1 {
                    group: group.index,
                    estimate: receipt.estimate,
                    standard_error_method:
                        ParametricGroupSeMethodV1::OrdinaryPlsScoreConditionalCenteredOls,
                    standard_error: receipt.standard_error,
                    observations: receipt.observations,
                    predictor_count: receipt.predictor_count,
                    variance_degrees_of_freedom: receipt.variance_degrees_of_freedom,
                    residual_sum_of_squares: receipt.residual_sum_of_squares,
                    coefficient_variance_factor: receipt.coefficient_variance_factor,
                });
            parametric_se_evidence.push(
                MultiModRunnerEvidenceV1::MgaOrdinaryPlsPathStandardError {
                    parameter: item.parameter,
                    group: group.index,
                    receipt,
                },
            );
        }
        observed_group_parameters.push(value);
    }
    let parametric_cells = if needs_parametric {
        projections
            .iter()
            .map(|projection| {
                let group_estimates = parametric_by_parameter
                    .remove(&projection.identity)
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::ExecutionCache(format!(
                            "point shards omitted SE receipts for {}",
                            projection.identity.stable_id
                        ))
                    })?;
                Ok(PreparedMgaParametricCellV1 {
                    parameter: projection.identity.clone(),
                    group_estimates,
                })
            })
            .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?
    } else {
        Vec::new()
    };

    let mut micom_results = Vec::new();
    let mut micom_public = Vec::new();
    let mut micom_ledgers = Vec::new();
    let mut micom_partition_plan_receipts = BTreeMap::new();
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        let construct_ids = ordinary_pls_micom_construct_ids_v1(
            &refitter.blocks,
            refitter.authority.technical_construct_ids(),
        );
        let receipt = MicomConfiguralReceiptV1 {
            identical_indicators_and_coding: config
                .configural_checklist
                .identical_indicators_and_coding,
            identical_data_treatment: config.configural_checklist.identical_data_treatment,
            identical_algorithm_settings: config.configural_checklist.identical_algorithm_settings,
            identical_model_specification: config
                .configural_checklist
                .identical_model_specification,
            deterministic_orientation_reviewed: config
                .configural_checklist
                .deterministic_sign_orientation_reviewed,
            analyst_review_confirmed: config.configural_checklist.analyst_review_confirmed,
        };
        for pair in &pairs {
            let partition_plan = pairwise_partition_plans
                .iter()
                .find(|plan| {
                    plan.pair.group_a == pair.group_a.min(pair.group_b)
                        && plan.pair.group_b == pair.group_a.max(pair.group_b)
                })
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::InvalidLedger(format!(
                        "shared partition plan is missing for {}",
                        pairwise_plan_key_v1(*pair)
                    ))
                })?;
            let permutation_config = MicomPermutationConfigV1 {
                requested: config.permutation_samples as usize,
                seed: config.seed,
                alpha: config.alpha,
            };
            let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
                let result = if let Some(case_weights) = &case_weights_by_row {
                    run_pairwise_case_weighted_micom_with_partition_plan_v1(
                        &mut refitter,
                        *pair,
                        &rows_by_group,
                        &design.rows,
                        &construct_ids,
                        receipt.clone(),
                        permutation_config.clone(),
                        partition_plan,
                        case_weights,
                        || should_cancel(),
                    )
                } else {
                    run_pairwise_micom_with_partition_plan_v1(
                        &mut refitter,
                        *pair,
                        &rows_by_group,
                        &design.rows,
                        &construct_ids,
                        receipt.clone(),
                        permutation_config.clone(),
                        partition_plan,
                        || should_cancel(),
                    )
                }
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                if should_cancel() {
                    return Err(MultiModRunnerErrorV1::Cancelled);
                }
                if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(
                        "MICOM result changed the shared pairwise partition-plan identity".into(),
                    ));
                }
                let rows = micom_public_rows_v1(config, &result)?;
                Ok(MgaExecutionShardPayloadV1::MicomPair {
                    value: result,
                    rows,
                })
            };
            let payload = if let (Some(_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::MicomPair { pair: *pair },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
            let MgaExecutionShardPayloadV1::MicomPair {
                value: result,
                rows,
            } = payload
            else {
                unreachable!("cache validates MICOM payload against shard kind")
            };
            if rows != micom_public_rows_v1(config, &result)? {
                return Err(cached_payload_error_v1(
                    "ordinary/weighted/PLSc MICOM public rows differ from their retained kernel result",
                ));
            }
            validate_cached_micom_result_v1(
                &result,
                if config.profile == qpls_core::MgaModelProfileV1::CaseWeightedPls {
                    MICOM_CASE_WEIGHTED_PAIRWISE_METHOD_VERSION_V1
                } else {
                    MICOM_PAIRWISE_METHOD_VERSION_V1
                },
                *pair,
                &construct_ids,
                &receipt,
                &permutation_config,
                &partition_plan.plan_sha256,
                partition_plan
                    .entries
                    .iter()
                    .map(|entry| (entry.replicate, entry.partition_sha256.as_str())),
            )?;
            if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "MICOM result changed the shared pairwise partition-plan identity".into(),
                ));
            }
            micom_partition_plan_receipts.insert(
                pairwise_plan_key_v1(*pair),
                result.partition_plan_sha256.clone(),
            );
            micom_public.extend(rows);
            micom_ledgers.push(micom_ledger_summary_v1(&result));
            micom_results.push(result);
        }
    }
    let comparability = comparable_ordinary_pls_targets_v1(
        &projections,
        &micom_results,
        &pairs,
        config.procedures.contains(&MgaProcedureV1::MicomPairwise),
    )?;
    let prepared = PreparedMgaExecutionV1 {
        design: design.clone(),
        parameters,
        refit_receipt: PreparedMgaRefitReceiptV1 {
            complete_model_refit_per_request: true,
            deterministic_sign_orientation: true,
            interaction_products_rebuilt_per_request: false,
            hoc_dependency_stages_refit_per_request: false,
            plsc_correction_repeated_per_request: refitter.authority.repeats_plsc_correction(),
            positive_case_weights_applied_per_request: config.profile
                == qpls_core::MgaModelProfileV1::CaseWeightedPls,
            integer_frequency_count_space_equivalent: false,
        },
        observed_group_parameters,
        pairwise_partition_plans: pairwise_partition_plans.clone(),
        micom_partition_plan_receipts,
        micom_pairs: micom_public,
        comparable_target_ids_by_canonical_pair: comparability.by_canonical_pair,
        comparable_target_ids: comparability.all_pairs,
        parametric_cells,
        excluded_rows: excluded_rows.to_vec(),
    };
    let mut output = run_compiled_mga_multigroup_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        &prepared,
        &mut refitter,
        None,
        execution_cache.as_deref_mut(),
        checkpoint.as_deref_mut(),
        &should_cancel,
        &progress,
    )?;
    if let MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(analysis) = &mut output.result {
        analysis.replicate_ledgers.extend(micom_ledgers);
    }
    output.evidence.extend(
        micom_results
            .into_iter()
            .map(MultiModRunnerEvidenceV1::MgaMicomPair),
    );
    output.evidence.extend(parametric_se_evidence);
    output
        .result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    Ok(output)
}

/// Executes positive-integer frequency-weighted MGA entirely in count space.
/// The production WPLS point engine receives one physical row per positive
/// count; permutation and bootstrap planners never materialize row expansion.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_frequency_weighted_pls_mga_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_frequency_weighted_pls_mga_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        excluded_rows,
        None,
        None,
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_compiled_frequency_weighted_pls_mga_internal_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    mut execution_cache: Option<&mut ValidatedMgaExecutionCacheSessionV1<'_>>,
    mut checkpoint: Option<&mut MgaShardCheckpointCallbackV1<'_>>,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    if config.profile != qpls_core::MgaModelProfileV1::FrequencyWeightedPls {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.frequency_weighted_profile_required".into(),
        ));
    }
    if config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::ParametricPooledVariance
                | MgaProcedureV1::ParametricWelchSatterthwaite
                | MgaProcedureV1::ParametricWaldOmnibus
        )
    }) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.frequency_parametric_se_semantics_unavailable".into(),
        ));
    }
    let predicted_sidecar_bytes = predict_mga_sidecar_bytes_v1(
        config,
        &compact_group_row_counts_v1(design),
        multimod_model_target_upper_bound_v1(&config.selected_parameter_ids, model),
        multimod_model_target_id_maximum_bytes_v1(&config.selected_parameter_ids, model),
        multimod_model_micom_construct_upper_bound_v1(model),
    );
    enforce_multimod_sidecar_cost_v1("mga", predicted_sidecar_bytes, &progress)?;
    validate_prepared_group_membership_v1(dataset, config, design)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::PreparingPointInputs,
        0,
        1,
        "mga:frequency:projection",
    );
    let authority = projected_ordinary_pls_authority_v1(recipe, model, config)?;
    let weight_column = authority
        .weight_source_column()
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "frequency-weighted MGA lost its resolved weight source column".into(),
            )
        })?
        .to_owned();
    let (frequency_design, orientation_rows, orientation_counts) =
        frequency_multigroup_design_from_raw_v1(dataset, &weight_column, design)?;
    let frequency_eligibility = assess_frequency_multigroup_design_v1(&frequency_design);
    if !frequency_eligibility.eligible {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "frequency MGA design is ineligible: {:?}",
            frequency_eligibility.blockers
        )));
    }
    let point_plan = authority.plan();
    let blocks = ordinary_pls_scoring_blocks_v1(point_plan);
    let projections = ordinary_pls_parameter_projections_with_technical_v1(
        config,
        authority.point_model(),
        point_plan,
        authority.technical_construct_ids(),
    )?;
    let parameters = projections
        .iter()
        .map(|projection| projection.identity.clone())
        .collect::<Vec<_>>();
    let mut source_columns = ordinary_pls_source_columns_v1(dataset, &blocks)?;
    if !source_columns.contains(&weight_column) {
        source_columns.push(weight_column.clone());
    }
    let raw_scores = OrdinaryPlsRawScoreCacheV1::build(
        dataset,
        &source_columns,
        &orientation_rows,
        &should_cancel,
    )?;
    report(
        &progress,
        MultiModRunnerPhaseV1::PointEstimation,
        0,
        1,
        "mga:frequency:pooled_reference",
    );
    let pooled_fit = run_frequency_pls_sample_v1(
        dataset,
        &source_columns,
        &orientation_rows,
        &orientation_counts,
        &weight_column,
        &authority,
        &should_cancel,
    )
    .map_err(|failure| match failure.code {
        RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
        _ => MultiModRunnerErrorV1::Kernel(format!(
            "multimod.runner.mga.frequency_pooled_fit_failed:{}",
            failure.detail
        )),
    })?;
    let pairs = selected_mga_pairs(config)?;
    let execution_plan = if execution_cache.is_some() {
        let plan = build_mga_execution_plan_v1(
            artifact.receipt(),
            &artifact.receipt().dataset_fingerprint,
            config,
            design,
            &parameters,
            &pairs,
        )
        .map_err(map_mga_execution_cache_error_v1)?;
        if execution_cache
            .as_deref()
            .expect("raw frequency cache presence checked above")
            .plan()
            .plan_sha256
            != plan.plan_sha256
        {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "validated session plan differs from the reconstructed frequency plan".into(),
            ));
        }
        Some(plan)
    } else {
        None
    };
    let uses_pairwise_plan = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::MicomPairwise | MgaProcedureV1::PairwisePermutation
        )
    });
    let frequency_pairwise_plans = if uses_pairwise_plan {
        pairs
            .iter()
            .map(|pair| {
                build_frequency_pairwise_partition_plan_v1(
                    &frequency_design,
                    *pair,
                    config.permutation_samples as usize,
                    config.seed,
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let micom_total = if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        pairs.len() as u64 * ((u64::from(config.permutation_samples) + 1) * 2 + 1)
    } else {
        0
    };
    let mut refitter = FrequencyOrdinaryPlsMgaRefitterV1 {
        dataset,
        authority,
        source_columns,
        weight_column: weight_column.clone(),
        blocks,
        projections: projections.clone(),
        orientation_rows,
        orientation_counts,
        raw_scores,
        pooled_fit,
        should_cancel: &should_cancel,
        progress: &progress,
        micom_completed: 0,
        micom_total,
    };
    let mut observed_group_parameters = Vec::with_capacity(frequency_design.groups.len());
    for group in &frequency_design.groups {
        let selected = frequency_design
            .rows
            .iter()
            .filter(|row| row.group == group.index)
            .collect::<Vec<_>>();
        let rows = selected
            .iter()
            .map(|row| row.source_row)
            .collect::<Vec<_>>();
        let counts = selected.iter().map(|row| row.frequency).collect::<Vec<_>>();
        let compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
            let fit =
                refitter
                    .fit_counts(&rows, &counts)
                    .map_err(|failure| match failure.code {
                        RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
                        _ => MultiModRunnerErrorV1::Kernel(format!(
                            "multimod.runner.mga.frequency_observed_fit_failed:{}",
                            failure.detail
                        )),
                    })?;
            let vector = refitter
                .parameter_vector(&fit)
                .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.detail))?;
            Ok(MgaExecutionShardPayloadV1::PointFit {
                value: GroupParameterVectorV1 {
                    group: group.index,
                    values: vector
                        .parameters
                        .into_iter()
                        .map(|parameter| parameter.estimate)
                        .collect(),
                },
                ordinary_path_standard_errors: Vec::new(),
            })
        };
        let payload =
            if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::PointFit { group: group.index },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
        let MgaExecutionShardPayloadV1::PointFit { value, .. } = payload else {
            unreachable!("cache validates frequency point payload against shard kind")
        };
        observed_group_parameters.push(value);
    }

    let mut micom_results = Vec::new();
    let mut micom_public = Vec::new();
    let mut micom_ledgers = Vec::new();
    let mut micom_partition_plan_receipts = BTreeMap::new();
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        let construct_ids = ordinary_pls_micom_construct_ids_v1(
            &refitter.blocks,
            refitter.authority.technical_construct_ids(),
        );
        let receipt = MicomConfiguralReceiptV1 {
            identical_indicators_and_coding: config
                .configural_checklist
                .identical_indicators_and_coding,
            identical_data_treatment: config.configural_checklist.identical_data_treatment,
            identical_algorithm_settings: config.configural_checklist.identical_algorithm_settings,
            identical_model_specification: config
                .configural_checklist
                .identical_model_specification,
            deterministic_orientation_reviewed: config
                .configural_checklist
                .deterministic_sign_orientation_reviewed,
            analyst_review_confirmed: config.configural_checklist.analyst_review_confirmed,
        };
        for pair in &pairs {
            let partition_plan = frequency_pairwise_plans
                .iter()
                .find(|plan| {
                    plan.pair.group_a == pair.group_a.min(pair.group_b)
                        && plan.pair.group_b == pair.group_a.max(pair.group_b)
                })
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::InvalidLedger(format!(
                        "frequency shared plan is missing for {}",
                        pairwise_plan_key_v1(*pair)
                    ))
                })?;
            let micom_config = MicomPermutationConfigV1 {
                requested: config.permutation_samples as usize,
                seed: config.seed,
                alpha: config.alpha,
            };
            let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
                let result = run_frequency_pairwise_micom_with_partition_plan_v1(
                    &mut refitter,
                    &frequency_design,
                    *pair,
                    &construct_ids,
                    receipt.clone(),
                    micom_config.clone(),
                    partition_plan,
                    || should_cancel(),
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                if should_cancel() {
                    return Err(MultiModRunnerErrorV1::Cancelled);
                }
                if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(
                        "frequency MICOM changed the shared count-space plan identity".into(),
                    ));
                }
                let rows = micom_public_rows_v1(config, &result)?;
                Ok(MgaExecutionShardPayloadV1::MicomPair {
                    value: result,
                    rows,
                })
            };
            let payload = if let (Some(_execution_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::MicomPair { pair: *pair },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
            let MgaExecutionShardPayloadV1::MicomPair {
                value: result,
                rows,
            } = payload
            else {
                unreachable!("cache validates frequency MICOM payload against shard kind")
            };
            if rows != micom_public_rows_v1(config, &result)? {
                return Err(cached_payload_error_v1(
                    "frequency MICOM public rows differ from their retained kernel result",
                ));
            }
            validate_cached_micom_result_v1(
                &result,
                FREQUENCY_MICOM_PAIRWISE_METHOD_VERSION_V1,
                *pair,
                &construct_ids,
                &receipt,
                &micom_config,
                &partition_plan.plan_sha256,
                partition_plan
                    .entries
                    .iter()
                    .map(|entry| (entry.replicate, entry.partition_sha256.as_str())),
            )?;
            if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "frequency MICOM changed the shared count-space plan identity".into(),
                ));
            }
            micom_partition_plan_receipts.insert(
                pairwise_plan_key_v1(*pair),
                result.partition_plan_sha256.clone(),
            );
            micom_public.extend(rows);
            micom_ledgers.push(micom_ledger_summary_v1(&result));
            micom_results.push(result);
        }
    }
    let comparability = comparable_ordinary_pls_targets_v1(
        &projections,
        &micom_results,
        &pairs,
        config.procedures.contains(&MgaProcedureV1::MicomPairwise),
    )?;
    let permutation_config = MultigroupResamplingConfigV1 {
        requested: config.permutation_samples as usize,
        seed: config.seed,
        confidence_level: config.confidence_level,
        alpha: config.alpha,
        alternative: core_alternative(config.alternative),
    };
    let bootstrap_config = MultigroupResamplingConfigV1 {
        requested: config.bootstrap_samples as usize,
        ..permutation_config
    };
    let mut pairwise_permutations = BTreeMap::new();
    if config
        .procedures
        .contains(&MgaProcedureV1::PairwisePermutation)
    {
        for pair in &pairs {
            let partition_plan = frequency_pairwise_plans
                .iter()
                .find(|plan| {
                    plan.pair.group_a == pair.group_a.min(pair.group_b)
                        && plan.pair.group_b == pair.group_a.max(pair.group_b)
                })
                .expect("frequency pairwise plan inventory validated above");
            let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
                let result = run_frequency_pairwise_permutation_with_plan_v1(
                    &frequency_design,
                    *pair,
                    &parameters,
                    permutation_config,
                    partition_plan,
                    &mut refitter,
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                if should_cancel() {
                    return Err(MultiModRunnerErrorV1::Cancelled);
                }
                Ok(MgaExecutionShardPayloadV1::PairwisePermutation { value: result })
            };
            let payload = if let (Some(_execution_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::PairwisePermutation { pair: *pair },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
            let MgaExecutionShardPayloadV1::PairwisePermutation { value: result } = payload else {
                unreachable!("cache validates frequency permutation payload against shard kind")
            };
            pairwise_permutations.insert(pairwise_plan_key_v1(*pair), result);
        }
    }
    let omnibus_permutation = if config
        .procedures
        .contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation)
    {
        let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
            run_frequency_max_spread_omnibus_permutation_v1(
                &frequency_design,
                &parameters,
                permutation_config,
                &mut refitter,
            )
            .map(|value| MgaExecutionShardPayloadV1::OmnibusPermutation { value })
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))
        };
        let payload =
            if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::OmnibusPermutation,
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
        let MgaExecutionShardPayloadV1::OmnibusPermutation { value } = payload else {
            unreachable!("cache validates frequency omnibus payload against shard kind")
        };
        Some(value)
    } else {
        None
    };
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    let bootstrap_selected = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::HenselerPlsMga | MgaProcedureV1::BootstrapDifferenceBc
        )
    });
    let bootstrap_banks = if bootstrap_selected {
        let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
            run_frequency_group_bootstrap_banks_v1(
                &frequency_design,
                &parameters,
                bootstrap_config,
                &mut refitter,
            )
            .map(|value| MgaExecutionShardPayloadV1::SharedGroupBootstrapBanks { value })
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))
        };
        let payload =
            if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::SharedGroupBootstrapBanks,
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
        let MgaExecutionShardPayloadV1::SharedGroupBootstrapBanks { value } = payload else {
            unreachable!("cache validates frequency bootstrap payload against shard kind")
        };
        Some(value)
    } else {
        None
    };
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    let prepared = PreparedMgaExecutionV1 {
        design: design.clone(),
        parameters,
        refit_receipt: PreparedMgaRefitReceiptV1 {
            complete_model_refit_per_request: true,
            deterministic_sign_orientation: true,
            interaction_products_rebuilt_per_request: false,
            hoc_dependency_stages_refit_per_request: false,
            plsc_correction_repeated_per_request: false,
            positive_case_weights_applied_per_request: false,
            integer_frequency_count_space_equivalent: true,
        },
        observed_group_parameters,
        pairwise_partition_plans: Vec::new(),
        micom_partition_plan_receipts,
        micom_pairs: micom_public,
        comparable_target_ids_by_canonical_pair: comparability.by_canonical_pair,
        comparable_target_ids: comparability.all_pairs,
        parametric_cells: Vec::new(),
        excluded_rows: excluded_rows.to_vec(),
    };
    let overrides = MgaKernelOverridesV1 {
        frequency_pairwise_plans,
        pairwise_permutations,
        omnibus_permutation,
        bootstrap_banks,
        eligibility: frequency_eligibility,
    };
    let mut rejecting_refitter = |_request: &MultigroupFitRequestV1| {
        Err(RefitFailureV1::new(
            RefitFailureCodeV1::UnsupportedProfile,
            "standard row-list refitter cannot execute frequency count-space MGA",
        ))
    };
    let mut output = run_compiled_mga_multigroup_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        &prepared,
        &mut rejecting_refitter,
        Some(&overrides),
        execution_cache.as_deref_mut(),
        checkpoint.as_deref_mut(),
        &should_cancel,
        &progress,
    )?;
    if let MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(analysis) = &mut output.result {
        analysis.replicate_ledgers.extend(micom_ledgers);
    }
    output.evidence.extend(
        micom_results
            .into_iter()
            .map(MultiModRunnerEvidenceV1::MgaMicomPair),
    );
    output
        .result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    Ok(output)
}

fn comparable_interaction_targets_v1(
    projections: &[InteractionMgaParameterProjectionV1],
    micom: &[MicomPairwiseResultV1],
    pairs: &[OrderedGroupPairV1],
    micom_selected: bool,
) -> Result<MgaComparabilityAuthorityV1, MultiModRunnerErrorV1> {
    let requirements = projections
        .iter()
        .map(|projection| {
            (
                projection.identity.stable_id.clone(),
                projection.required_constructs.clone(),
            )
        })
        .collect::<Vec<_>>();
    build_mga_comparability_authority_v1(&requirements, micom, pairs, micom_selected)
}

/// Executes the raw-data TwoStage + Strong interaction MGA profiles. Every
/// point, permutation, bootstrap, and MICOM training request re-estimates the
/// stage-one PLS scores, reorients them to the frozen pooled reference,
/// rebuilds all standardized products, and solves the complete joint stage.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_interaction_pls_mga_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_interaction_pls_mga_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        design,
        excluded_rows,
        None,
        None,
        should_cancel,
        progress,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_compiled_interaction_pls_mga_internal_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    design: &MultigroupDesignV1,
    excluded_rows: &[ExcludedRowReceiptV1],
    mut execution_cache: Option<&mut ValidatedMgaExecutionCacheSessionV1<'_>>,
    mut checkpoint: Option<&mut MgaShardCheckpointCallbackV1<'_>>,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    if !matches!(
        config.profile,
        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
            | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
            | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation
    ) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.interaction_profile_required".into(),
        ));
    }
    if config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::ParametricPooledVariance
                | MgaProcedureV1::ParametricWelchSatterthwaite
                | MgaProcedureV1::ParametricWaldOmnibus
        )
    }) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.interaction_parametric_se_semantics_unavailable".into(),
        ));
    }
    let predicted_sidecar_bytes = predict_mga_sidecar_bytes_v1(
        config,
        &compact_group_row_counts_v1(design),
        multimod_model_target_upper_bound_v1(&config.selected_parameter_ids, model),
        multimod_model_target_id_maximum_bytes_v1(&config.selected_parameter_ids, model),
        multimod_model_micom_construct_upper_bound_v1(model),
    );
    enforce_multimod_sidecar_cost_v1("mga", predicted_sidecar_bytes, &progress)?;
    let eligibility = assess_multigroup_design_v1(design);
    if !eligibility.eligible {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "interaction PLS MGA design is ineligible: {:?}",
            eligibility.blockers
        )));
    }
    validate_prepared_group_membership_v1(dataset, config, design)?;
    let authority = projected_interaction_mga_authority_v1(dataset, recipe, model, artifact)?;
    let mut orientation_rows = design
        .rows
        .iter()
        .map(|row| row.source_row)
        .collect::<Vec<_>>();
    orientation_rows.sort_unstable();
    orientation_rows.dedup();
    if orientation_rows.len() != design.rows.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "interaction PLS MGA design contains duplicate source rows".into(),
        ));
    }
    let raw_scores = OrdinaryPlsRawScoreCacheV1::build(
        dataset,
        &authority.source_columns,
        &orientation_rows,
        &should_cancel,
    )?;
    let pooled_stage_one = pooled_ordinary_pls_fit_v1(
        dataset,
        &authority.source_columns,
        &orientation_rows,
        &authority.point_recipe,
        &authority.point_model,
        &authority.point_artifact,
        qpls_core::MgaModelProfileV1::GeneralSemPls,
        None,
        &should_cancel,
    )?;
    let pooled_joint = interaction_mga_joint_point_v1(
        &authority,
        &pooled_stage_one,
        config.profile,
        &should_cancel,
    )
    .map_err(|failure| match failure.code {
        RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
        _ => MultiModRunnerErrorV1::Kernel(format!(
            "pooled interaction joint fit failed: {}",
            failure.detail
        )),
    })?;
    let projections = interaction_mga_parameter_projections_v1(config, &authority, &pooled_joint)?;
    let parameters = projections
        .iter()
        .map(|projection| projection.identity.clone())
        .collect::<Vec<_>>();
    let pairs = selected_mga_pairs(config)?;
    let execution_plan = if execution_cache.is_some() {
        let plan = build_mga_execution_plan_v1(
            artifact.receipt(),
            &artifact.receipt().dataset_fingerprint,
            config,
            design,
            &parameters,
            &pairs,
        )
        .map_err(map_mga_execution_cache_error_v1)?;
        if execution_cache
            .as_deref()
            .expect("raw interaction cache presence checked above")
            .plan()
            .plan_sha256
            != plan.plan_sha256
        {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "validated session plan differs from the reconstructed interaction plan".into(),
            ));
        }
        Some(plan)
    } else {
        None
    };
    let rows_by_group = observed_rows_by_group_v1(design);
    let uses_pairwise_partition_plan = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::MicomPairwise | MgaProcedureV1::PairwisePermutation
        )
    });
    let pairwise_partition_plans = if uses_pairwise_partition_plan {
        pairs
            .iter()
            .map(|pair| {
                build_pairwise_partition_plan_v1(
                    design,
                    *pair,
                    config.permutation_samples as usize,
                    config.seed,
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let micom_total = if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        pairs.len() as u64 * ((u64::from(config.permutation_samples) + 1) * 2 + 1)
    } else {
        0
    };
    let mut refitter = InteractionMgaRefitterV1 {
        dataset,
        authority,
        profile: config.profile,
        projections: projections.clone(),
        orientation_rows,
        raw_scores,
        pooled_stage_one,
        should_cancel: &should_cancel,
        progress: &progress,
        micom_completed: 0,
        micom_total,
    };
    let mut observed_group_parameters = Vec::with_capacity(design.groups.len());
    for group in &design.groups {
        report(
            &progress,
            MultiModRunnerPhaseV1::PointEstimation,
            observed_group_parameters.len() as u64,
            design.groups.len() as u64,
            format!("mga:interaction:observed:g{}", group.index.get()),
        );
        let compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
            let fit = refitter
                .fit_rows(&rows_by_group[&group.index])
                .map_err(|failure| match failure.code {
                    RefitFailureCodeV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
                    _ => MultiModRunnerErrorV1::Kernel(format!(
                        "observed interaction group refit failed: {}",
                        failure.detail
                    )),
                })?;
            let vector = refitter
                .parameter_vector(&fit)
                .map_err(|failure| MultiModRunnerErrorV1::Kernel(failure.detail))?;
            Ok(MgaExecutionShardPayloadV1::PointFit {
                value: GroupParameterVectorV1 {
                    group: group.index,
                    values: vector
                        .parameters
                        .into_iter()
                        .map(|parameter| parameter.estimate)
                        .collect(),
                },
                ordinary_path_standard_errors: Vec::new(),
            })
        };
        let payload =
            if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::PointFit { group: group.index },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
        let MgaExecutionShardPayloadV1::PointFit { value, .. } = payload else {
            unreachable!("cache validates interaction point payload against shard kind")
        };
        observed_group_parameters.push(value);
    }
    let mut micom_results = Vec::new();
    let mut micom_public = Vec::new();
    let mut micom_ledgers = Vec::new();
    let mut micom_partition_plan_receipts = BTreeMap::new();
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        let construct_ids = refitter
            .authority
            .blocks
            .iter()
            .map(|block| block.construct_id.clone())
            .collect::<Vec<_>>();
        let receipt = MicomConfiguralReceiptV1 {
            identical_indicators_and_coding: config
                .configural_checklist
                .identical_indicators_and_coding,
            identical_data_treatment: config.configural_checklist.identical_data_treatment,
            identical_algorithm_settings: config.configural_checklist.identical_algorithm_settings,
            identical_model_specification: config
                .configural_checklist
                .identical_model_specification,
            deterministic_orientation_reviewed: config
                .configural_checklist
                .deterministic_sign_orientation_reviewed,
            analyst_review_confirmed: config.configural_checklist.analyst_review_confirmed,
        };
        for pair in &pairs {
            let partition_plan = pairwise_partition_plans
                .iter()
                .find(|plan| {
                    plan.pair.group_a == pair.group_a.min(pair.group_b)
                        && plan.pair.group_b == pair.group_a.max(pair.group_b)
                })
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::InvalidLedger(format!(
                        "shared interaction partition plan is missing for {}",
                        pairwise_plan_key_v1(*pair)
                    ))
                })?;
            let micom_config = MicomPermutationConfigV1 {
                requested: config.permutation_samples as usize,
                seed: config.seed,
                alpha: config.alpha,
            };
            let mut compute = || -> Result<MgaExecutionShardPayloadV1, MultiModRunnerErrorV1> {
                let result = run_pairwise_micom_with_partition_plan_v1(
                    &mut refitter,
                    *pair,
                    &rows_by_group,
                    &design.rows,
                    &construct_ids,
                    receipt.clone(),
                    micom_config.clone(),
                    partition_plan,
                    || should_cancel(),
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(
                        "interaction MICOM changed the shared partition-plan identity".into(),
                    ));
                }
                let rows = micom_public_rows_v1(config, &result)?;
                Ok(MgaExecutionShardPayloadV1::MicomPair {
                    value: result,
                    rows,
                })
            };
            let payload = if let (Some(_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &MgaExecutionShardKindV1::MicomPair { pair: *pair },
                    || should_cancel(),
                    || {
                        compute().map_err(|error| match error {
                            MultiModRunnerErrorV1::Cancelled => MgaExecutionCacheErrorV1::Cancelled,
                            other => MgaExecutionCacheErrorV1::ExecutionFailed(other.to_string()),
                        })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?
            } else {
                compute()?
            };
            let MgaExecutionShardPayloadV1::MicomPair {
                value: result,
                rows,
            } = payload
            else {
                unreachable!("cache validates interaction MICOM payload against shard kind")
            };
            if rows != micom_public_rows_v1(config, &result)? {
                return Err(cached_payload_error_v1(
                    "interaction MICOM public rows differ from their retained kernel result",
                ));
            }
            validate_cached_micom_result_v1(
                &result,
                MICOM_PAIRWISE_METHOD_VERSION_V1,
                *pair,
                &construct_ids,
                &receipt,
                &micom_config,
                &partition_plan.plan_sha256,
                partition_plan
                    .entries
                    .iter()
                    .map(|entry| (entry.replicate, entry.partition_sha256.as_str())),
            )?;
            if result.partition_plan_sha256 != partition_plan.plan_sha256 {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "interaction MICOM changed the shared partition-plan identity".into(),
                ));
            }
            micom_partition_plan_receipts.insert(
                pairwise_plan_key_v1(*pair),
                result.partition_plan_sha256.clone(),
            );
            micom_public.extend(rows);
            micom_ledgers.push(micom_ledger_summary_v1(&result));
            micom_results.push(result);
        }
    }
    let comparability = comparable_interaction_targets_v1(
        &projections,
        &micom_results,
        &pairs,
        config.procedures.contains(&MgaProcedureV1::MicomPairwise),
    )?;
    let prepared = PreparedMgaExecutionV1 {
        design: design.clone(),
        parameters,
        refit_receipt: PreparedMgaRefitReceiptV1 {
            complete_model_refit_per_request: true,
            deterministic_sign_orientation: true,
            interaction_products_rebuilt_per_request: true,
            hoc_dependency_stages_refit_per_request: false,
            plsc_correction_repeated_per_request: false,
            positive_case_weights_applied_per_request: false,
            integer_frequency_count_space_equivalent: false,
        },
        observed_group_parameters,
        pairwise_partition_plans: pairwise_partition_plans.clone(),
        micom_partition_plan_receipts,
        micom_pairs: micom_public,
        comparable_target_ids_by_canonical_pair: comparability.by_canonical_pair,
        comparable_target_ids: comparability.all_pairs,
        parametric_cells: Vec::new(),
        excluded_rows: excluded_rows.to_vec(),
    };
    let mut output = run_compiled_mga_multigroup_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        &prepared,
        &mut refitter,
        None,
        execution_cache.as_deref_mut(),
        checkpoint.as_deref_mut(),
        &should_cancel,
        &progress,
    )?;
    if let MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(analysis) = &mut output.result {
        analysis.replicate_ledgers.extend(micom_ledgers);
    }
    output.evidence.extend(
        micom_results
            .into_iter()
            .map(MultiModRunnerEvidenceV1::MgaMicomPair),
    );
    output
        .result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    Ok(output)
}

struct ControlledMgaRefitter<'a, R, C, P> {
    inner: &'a mut R,
    should_cancel: &'a C,
    progress: &'a P,
    completed: u64,
    total: u64,
    cancelled: bool,
}

impl<R, C, P> MultigroupRefitterV1 for ControlledMgaRefitter<'_, R, C, P>
where
    R: MultigroupRefitterV1,
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn fit(
        &mut self,
        request: &MultigroupFitRequestV1,
    ) -> Result<qpls_estimation::ParameterVectorV1, RefitFailureV1> {
        if (self.should_cancel)() {
            self.cancelled = true;
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::Cancelled,
                "cancelled by the MultiMod runner",
            ));
        }
        let phase = match request.sample_kind {
            FitSampleKindV1::ObservedGroup => MultiModRunnerPhaseV1::PointEstimation,
            FitSampleKindV1::PairwisePermutation
            | FitSampleKindV1::OmnibusPermutation
            | FitSampleKindV1::GroupBootstrap => MultiModRunnerPhaseV1::Resampling,
        };
        report(
            self.progress,
            phase,
            self.completed.min(self.total),
            self.total,
            format!(
                "mga:{:?}:g{}:r{}",
                request.sample_kind,
                request.group.get(),
                request
                    .replicate
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "point".into())
            ),
        );
        let result = self.inner.fit(request);
        self.completed = self.completed.saturating_add(1);
        result
    }
}

fn validate_mga_prepared(
    dataset: &Dataset,
    config: &MgaMultigroupV1,
    plan: &CompiledMultiModPlanV1,
    prepared: &PreparedMgaExecutionV1,
    kernel_overrides: Option<&MgaKernelOverridesV1>,
) -> Result<(), MultiModRunnerErrorV1> {
    let CompiledMultiModPlanV1::MgaMultigroupV1 {
        group_ids,
        selected_parameter_ids,
        ..
    } = plan
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not MGA multigroup V1".into(),
        ));
    };
    if !prepared.refit_receipt.complete_model_refit_per_request
        || !prepared.refit_receipt.deterministic_sign_orientation
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "MGA requires a complete deterministic model refit for every point and resample".into(),
        ));
    }
    match config.profile {
        qpls_core::MgaModelProfileV1::MultipleTwoWayModeration
        | qpls_core::MgaModelProfileV1::BoundedThreeWayModeration
        | qpls_core::MgaModelProfileV1::BoundedTwoWayModeratedMediation
            if !prepared
                .refit_receipt
                .interaction_products_rebuilt_per_request =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "moderation MGA must rebuild every interaction product in every refit".into(),
            ));
        }
        qpls_core::MgaModelProfileV1::MultipleNonnestedHoc
            if !prepared
                .refit_receipt
                .hoc_dependency_stages_refit_per_request =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "HOC MGA must rerun dependency stages in every refit".into(),
            ));
        }
        qpls_core::MgaModelProfileV1::ReflectivePlsc
            if !prepared.refit_receipt.plsc_correction_repeated_per_request =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "PLSc correction must be repeated in every refit".into(),
            ));
        }
        qpls_core::MgaModelProfileV1::CaseWeightedPls
            if !prepared
                .refit_receipt
                .positive_case_weights_applied_per_request =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "case weights must travel with rows and be applied in every refit".into(),
            ));
        }
        qpls_core::MgaModelProfileV1::FrequencyWeightedPls
            if !prepared
                .refit_receipt
                .integer_frequency_count_space_equivalent =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "frequency-weighted refits require exact row-expansion-equivalent count space"
                    .into(),
            ));
        }
        _ => {}
    }
    if prepared.design.groups.len() != config.groups.len()
        || prepared.design.groups.len() != group_ids.len()
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "prepared groups differ from the compiled group inventory".into(),
        ));
    }
    for (index, ((actual, configured), group_id)) in prepared
        .design
        .groups
        .iter()
        .zip(&config.groups)
        .zip(group_ids)
        .enumerate()
    {
        let expected_value = match &configured.value {
            qpls_core::TypedGroupValueV1::Text { value } => {
                qpls_estimation::TypedGroupValueV1::Text {
                    value: value.clone(),
                }
            }
            qpls_core::TypedGroupValueV1::Boolean { value } => {
                qpls_estimation::TypedGroupValueV1::Boolean { value: *value }
            }
            qpls_core::TypedGroupValueV1::Integer { value } => {
                qpls_estimation::TypedGroupValueV1::Integer { value: *value }
            }
            qpls_core::TypedGroupValueV1::Number { value } => {
                qpls_estimation::TypedGroupValueV1::finite_number(*value)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?
            }
        };
        if actual.index.get() != index
            || &actual.value != &expected_value
            || actual.display_label != configured.label
            || group_id != &configured.group_id
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "prepared typed group identity or ordering differs from the recipe".into(),
            ));
        }
    }
    if prepared
        .design
        .rows
        .iter()
        .any(|row| row.source_row as usize >= dataset.batch.num_rows())
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "prepared group row lies outside the execution dataset".into(),
        ));
    }
    validate_prepared_group_membership_v1(dataset, config, &prepared.design)?;
    if kernel_overrides.is_some()
        != (config.profile == qpls_core::MgaModelProfileV1::FrequencyWeightedPls)
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "count-space kernel overrides must be present exactly for frequency-weighted MGA"
                .into(),
        ));
    }
    let eligibility = kernel_overrides
        .map(|overrides| overrides.eligibility.clone())
        .unwrap_or_else(|| assess_multigroup_design_v1(&prepared.design));
    if !eligibility.eligible {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "MGA design is ineligible: {:?}",
            eligibility.blockers
        )));
    }
    let actual_parameters = prepared
        .parameters
        .iter()
        .map(|parameter| parameter.stable_id.clone())
        .collect::<Vec<_>>();
    if prepared.parameters.is_empty()
        || actual_parameters.iter().collect::<BTreeSet<_>>().len() != actual_parameters.len()
        || (!selected_parameter_ids.is_empty() && &actual_parameters != selected_parameter_ids)
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "prepared parameter identities are empty, duplicated, or differ from the compiled selection"
            .into(),
        ));
    }
    let pairs = selected_mga_pairs(config)?;
    validate_prepared_pair_comparability_v1(config, &pairs, prepared)?;
    let consumes_pairwise_partitions = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::MicomPairwise | MgaProcedureV1::PairwisePermutation
        )
    });
    if consumes_pairwise_partitions
        && let Some(overrides) = kernel_overrides
        && (!prepared.pairwise_partition_plans.is_empty()
            || overrides.frequency_pairwise_plans.len() != pairs.len()
            || pairs
                .iter()
                .any(|pair| overrides.frequency_plan(*pair).is_none()))
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "frequency pairwise procedures require exactly one typed count-space plan per selected pair"
                .into(),
        ));
    }
    if consumes_pairwise_partitions && kernel_overrides.is_none() {
        if prepared.pairwise_partition_plans.len() != pairs.len() {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "the prepared pairwise partition-plan inventory differs from the comparison plan"
                    .into(),
            ));
        }
        for pair in &pairs {
            if let Some(partition_plan) = prepared_pairwise_plan_v1(prepared, *pair) {
                qpls_estimation::validate_pairwise_partition_plan_for_rows_v1(
                    &prepared.design.rows,
                    *pair,
                    config.permutation_samples as usize,
                    config.seed,
                    partition_plan,
                )
                .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))?;
            } else {
                return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                    "prepared partition-plan inventory omits {}",
                    pairwise_plan_key_v1(*pair)
                )));
            }
        }
    } else if kernel_overrides.is_none() && !prepared.pairwise_partition_plans.is_empty() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "pairwise partition plans were supplied without a pairwise procedure".into(),
        ));
    }
    if config.procedures.contains(&MgaProcedureV1::MicomPairwise)
        && let Some(overrides) = kernel_overrides
    {
        if !prepared.pairwise_partition_plans.is_empty()
            || overrides.frequency_pairwise_plans.len() != pairs.len()
        {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "frequency MICOM requires exactly one typed count-space plan per selected pair"
                    .into(),
            ));
        }
        for pair in &pairs {
            let partition_plan = overrides.frequency_plan(*pair).ok_or_else(|| {
                MultiModRunnerErrorV1::InvalidLedger(format!(
                    "frequency MICOM is missing the shared count-space plan for {}",
                    pairwise_plan_key_v1(*pair)
                ))
            })?;
            if prepared
                .micom_partition_plan_receipts
                .get(&pairwise_plan_key_v1(*pair))
                != Some(&partition_plan.plan_sha256)
            {
                return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                    "frequency MICOM partition receipt differs for {}",
                    pairwise_plan_key_v1(*pair)
                )));
            }
        }
        if prepared.micom_partition_plan_receipts.len() != pairs.len() {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "frequency MICOM partition-receipt inventory differs from the comparison plan"
                    .into(),
            ));
        }
    } else if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        for pair in &pairs {
            let partition_plan = prepared_pairwise_plan_v1(prepared, *pair).ok_or_else(|| {
                MultiModRunnerErrorV1::InvalidLedger(format!(
                    "MICOM is missing the shared partition plan for {}",
                    pairwise_plan_key_v1(*pair)
                ))
            })?;
            if prepared
                .micom_partition_plan_receipts
                .get(&pairwise_plan_key_v1(*pair))
                != Some(&partition_plan.plan_sha256)
            {
                return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                    "MICOM partition receipt differs for {}",
                    pairwise_plan_key_v1(*pair)
                )));
            }
        }
        if prepared.micom_partition_plan_receipts.len() != pairs.len() {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "MICOM partition-receipt inventory differs from the comparison plan".into(),
            ));
        }
    } else if !prepared.micom_partition_plan_receipts.is_empty() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "MICOM partition receipts were supplied without selecting MICOM".into(),
        ));
    }
    Ok(())
}

fn selected_mga_pairs(
    config: &MgaMultigroupV1,
) -> Result<Vec<OrderedGroupPairV1>, MultiModRunnerErrorV1> {
    let by_id = config
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| (group.group_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let raw_pairs = match &config.comparison_plan {
        MgaComparisonPlanV1::AllPairs { .. } => {
            let mut pairs = Vec::new();
            for left in 0..config.groups.len() {
                for right in (left + 1)..config.groups.len() {
                    pairs.push((left, right));
                }
            }
            pairs
        }
        MgaComparisonPlanV1::ReferenceVsRest { reference_group_id } => {
            let reference = *by_id.get(reference_group_id.as_str()).ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput("unknown reference group".into())
            })?;
            (0..config.groups.len())
                .filter(|index| *index != reference)
                .map(|index| (reference, index))
                .collect()
        }
        MgaComparisonPlanV1::SelectedPairs { pairs } => pairs
            .iter()
            .map(|pair| {
                Ok((
                    *by_id.get(pair.left_group_id.as_str()).ok_or_else(|| {
                        MultiModRunnerErrorV1::PreparedInput("unknown left group".into())
                    })?,
                    *by_id.get(pair.right_group_id.as_str()).ok_or_else(|| {
                        MultiModRunnerErrorV1::PreparedInput("unknown right group".into())
                    })?,
                ))
            })
            .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?,
    };
    raw_pairs
        .into_iter()
        .map(|(left, right)| {
            OrderedGroupPairV1::new(
                GroupIndexV1::new(left)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?,
                GroupIndexV1::new(right)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?,
            )
            .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))
        })
        .collect()
}

fn insert_group_point(
    points: &mut BTreeMap<(usize, String), f64>,
    group: GroupIndexV1,
    parameter: &ParameterIdentityV1,
    estimate: f64,
) -> Result<(), MultiModRunnerErrorV1> {
    if !estimate.is_finite() {
        return Err(MultiModRunnerErrorV1::Kernel(
            "group point estimate is nonfinite".into(),
        ));
    }
    let key = (group.get(), parameter.stable_id.clone());
    if let Some(existing) = points.insert(key, estimate) {
        if existing.to_bits() != estimate.to_bits() {
            return Err(MultiModRunnerErrorV1::Kernel(
                "repeated point fits disagree for the same group parameter".into(),
            ));
        }
    }
    Ok(())
}

fn cached_payload_error_v1(detail: impl Into<String>) -> MultiModRunnerErrorV1 {
    MultiModRunnerErrorV1::ExecutionCache(format!(
        "cached MGA shard failed semantic validation: {}",
        detail.into()
    ))
}

fn validate_cached_pairwise_permutation_v1(
    result: &PairwisePermutationResultV1,
    pair: OrderedGroupPairV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    partition_plan: &PairwisePartitionPlanV1,
    expected_group_counts: &[qpls_estimation::GroupEligibilitySummaryV1],
) -> Result<(), MultiModRunnerErrorV1> {
    let accounting = result.usable.checked_add(result.failed);
    let usable_ledger = result
        .ledger
        .iter()
        .filter(|entry| entry.status == ResampleFitStatusV1::Usable)
        .count();
    let ledger_valid = result
        .ledger
        .iter()
        .zip(&partition_plan.entries)
        .enumerate()
        .all(|(replicate, (entry, planned))| {
            let fits_usable = entry
                .group_fits
                .iter()
                .all(|fit| fit.status == ResampleFitStatusV1::Usable);
            entry.replicate == replicate
                && planned.replicate == replicate
                && entry.partition_sha256 == planned.partition_sha256
                && entry.group_fits.len() == 2
                && entry.group_fits[0].group == pair.group_a
                && entry.group_fits[1].group == pair.group_b
                && entry
                    .group_fits
                    .iter()
                    .all(|fit| (fit.status == ResampleFitStatusV1::Usable) == fit.failure.is_none())
                && (entry.status == ResampleFitStatusV1::Usable) == fits_usable
        });
    let points_valid = result.point_estimates.len() == parameters.len()
        && result
            .point_estimates
            .iter()
            .zip(parameters)
            .all(|(point, parameter)| {
                point.parameter == *parameter
                    && point.estimate_a.is_finite()
                    && point.estimate_b.is_finite()
                    && point.difference_a_minus_b.is_finite()
                    && point.difference_a_minus_b.to_bits()
                        == (point.estimate_a - point.estimate_b).to_bits()
            });
    let available = result.usable >= config.minimum_usable();
    let inference_valid = points_valid
        && if available {
            result.parameters.len() == parameters.len()
                && result.parameters.iter().zip(&result.point_estimates).all(
                    |(inference, point)| {
                        let selected = match config.alternative {
                            AlternativeHypothesisV1::TwoSided => inference.p_value_two_sided,
                            AlternativeHypothesisV1::Greater => inference.p_value_greater,
                            AlternativeHypothesisV1::Less => inference.p_value_less,
                        };
                        inference.parameter == point.parameter
                            && inference.estimate_a.to_bits() == point.estimate_a.to_bits()
                            && inference.estimate_b.to_bits() == point.estimate_b.to_bits()
                            && inference.difference_a_minus_b.to_bits()
                                == point.difference_a_minus_b.to_bits()
                            && inference.selected_alternative == config.alternative
                            && inference.selected_probability.to_bits() == selected.to_bits()
                            && [
                                inference.p_value_two_sided,
                                inference.p_value_greater,
                                inference.p_value_less,
                                inference.selected_probability,
                            ]
                            .into_iter()
                            .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
                    },
                )
        } else {
            result.parameters.is_empty()
        };
    if result.method_version != MGA_MULTIGROUP_PAIRWISE_PERMUTATION_VERSION_V1
        || result.pair != pair
        || result.seed != config.seed
        || result.requested != config.requested
        || result.attempted != config.requested
        || result.ledger.len() != config.requested
        || accounting != Some(config.requested)
        || result.minimum_usable != config.minimum_usable()
        || result.retry_policy != "none"
        || result.plan_sha256 != partition_plan.plan_sha256
        || (result.availability == InferenceAvailabilityV1::Available) != available
        || usable_ledger != result.usable
        || result.group_counts != expected_group_counts
        || !ledger_valid
        || !points_valid
        || !inference_valid
    {
        return Err(cached_payload_error_v1(
            "pairwise permutation identity, accounting, ledger, point estimates, or target inventory differs from the frozen request",
        ));
    }
    Ok(())
}

fn validate_cached_omnibus_permutation_v1(
    result: &OmnibusPermutationResultV1,
    design: &MultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    expected_group_counts: &[qpls_estimation::GroupEligibilitySummaryV1],
) -> Result<(), MultiModRunnerErrorV1> {
    let accounting = result.usable.checked_add(result.failed);
    let usable_ledger = result
        .ledger
        .iter()
        .filter(|entry| entry.status == ResampleFitStatusV1::Usable)
        .count();
    let ledger_valid = result.ledger.iter().enumerate().all(|(replicate, entry)| {
        let fits_usable = entry
            .group_fits
            .iter()
            .all(|fit| fit.status == ResampleFitStatusV1::Usable);
        entry.replicate == replicate
            && !entry.partition_sha256.trim().is_empty()
            && entry.group_fits.len() == design.groups.len()
            && entry
                .group_fits
                .iter()
                .zip(&design.groups)
                .all(|(fit, group)| {
                    fit.group == group.index
                        && (fit.status == ResampleFitStatusV1::Usable) == fit.failure.is_none()
                })
            && (entry.status == ResampleFitStatusV1::Usable) == fits_usable
    });
    let points_valid = result.group_point_estimates.len() == design.groups.len()
        && result
            .group_point_estimates
            .iter()
            .zip(&design.groups)
            .all(|(point, group)| {
                point.group == group.index
                    && point.values.len() == parameters.len()
                    && point.values.iter().all(|value| value.is_finite())
            });
    let available = result.usable >= config.minimum_usable();
    let inference_valid = points_valid
        && if available {
            result.parameters.len() == parameters.len()
                && result
                    .parameters
                    .iter()
                    .enumerate()
                    .all(|(parameter_index, inference)| {
                        let mut minimum = f64::INFINITY;
                        let mut maximum = f64::NEG_INFINITY;
                        for point in &result.group_point_estimates {
                            minimum = minimum.min(point.values[parameter_index]);
                            maximum = maximum.max(point.values[parameter_index]);
                        }
                        let reconstructed_probability = (1 + inference
                            .null_maximum_pairwise_spreads
                            .iter()
                            .filter(|value| **value >= inference.observed_maximum_pairwise_spread)
                            .count())
                            as f64
                            / (result.usable + 1) as f64;
                        inference.parameter == parameters[parameter_index]
                            && inference.observed_maximum_pairwise_spread.is_finite()
                            && inference.observed_maximum_pairwise_spread.to_bits()
                                == (maximum - minimum).to_bits()
                            && inference.p_value_right_tailed.is_finite()
                            && (0.0..=1.0).contains(&inference.p_value_right_tailed)
                            && inference.null_maximum_pairwise_spreads.len() == result.usable
                            && inference
                                .null_maximum_pairwise_spreads
                                .iter()
                                .all(|value| value.is_finite() && *value >= 0.0)
                            && inference.p_value_right_tailed.to_bits()
                                == reconstructed_probability.to_bits()
                    })
        } else {
            result.parameters.is_empty()
        };
    if result.method_version != MGA_MULTIGROUP_OMNIBUS_PERMUTATION_VERSION_V1
        || result.seed != config.seed
        || result.requested != config.requested
        || result.attempted != config.requested
        || result.ledger.len() != config.requested
        || accounting != Some(config.requested)
        || result.minimum_usable != config.minimum_usable()
        || result.retry_policy != "none"
        || result.plan_sha256.trim().is_empty()
        || (result.availability == InferenceAvailabilityV1::Available) != available
        || usable_ledger != result.usable
        || result.group_counts != expected_group_counts
        || !ledger_valid
        || !points_valid
        || !inference_valid
    {
        return Err(cached_payload_error_v1(
            "omnibus permutation identity, accounting, ledger, point estimates, or target inventory differs from the frozen request",
        ));
    }
    Ok(())
}

fn validate_cached_bootstrap_banks_v1(
    banks: &GroupBootstrapBanksV1,
    expected_method_version: &str,
    design: &MultigroupDesignV1,
    parameters: &[ParameterIdentityV1],
    config: MultigroupResamplingConfigV1,
    expected_group_counts: &[qpls_estimation::GroupEligibilitySummaryV1],
) -> Result<(), MultiModRunnerErrorV1> {
    let groups_valid = banks.groups.len() == design.groups.len()
        && banks
            .groups
            .iter()
            .zip(&design.groups)
            .all(|(bank, group)| {
                let actual_usable = bank
                    .replicate_estimates
                    .iter()
                    .filter(|values| values.is_some())
                    .count();
                bank.group == group.index
                    && bank.point_estimates.len() == parameters.len()
                    && bank.point_estimates.iter().all(|value| value.is_finite())
                    && bank.replicate_estimates.len() == config.requested
                    && bank.usable.checked_add(bank.failed) == Some(config.requested)
                    && bank.usable == actual_usable
                    && bank.replicate_estimates.iter().flatten().all(|values| {
                        values.len() == parameters.len()
                            && values.iter().all(|value| value.is_finite())
                    })
            });
    let ledger_valid = groups_valid
        && banks.ledger.len() == config.requested
        && banks.ledger.iter().enumerate().all(|(replicate, entry)| {
            let all_usable = entry
                .groups
                .iter()
                .all(|group| group.status == ResampleFitStatusV1::Usable);
            entry.replicate == replicate
                && entry.groups.len() == design.groups.len()
                && entry.groups.iter().zip(&design.groups).enumerate().all(
                    |(group_index, (item, group))| {
                        item.group == group.index
                            && !item.sample_sha256.trim().is_empty()
                            && (item.status == ResampleFitStatusV1::Usable)
                                == item.failure.is_none()
                            && (item.status == ResampleFitStatusV1::Usable)
                                == banks.groups[group_index].replicate_estimates[replicate]
                                    .is_some()
                    },
                )
                && (entry.status == ResampleFitStatusV1::Usable) == all_usable
        });
    let available = banks
        .groups
        .iter()
        .all(|bank| bank.usable >= config.minimum_usable());
    if banks.method_version != expected_method_version
        || banks.parameters != parameters
        || banks.seed != config.seed
        || banks.requested != config.requested
        || banks.attempted != config.requested
        || banks.minimum_usable != config.minimum_usable()
        || banks.retry_policy != "none"
        || banks.plan_sha256.trim().is_empty()
        || (banks.availability == InferenceAvailabilityV1::Available) != available
        || banks.ledger.len() != config.requested
        || banks.group_counts != expected_group_counts
        || !groups_valid
        || !ledger_valid
    {
        return Err(cached_payload_error_v1(
            "shared bootstrap-bank identity, accounting, ledger, group vectors, or target inventory differs from the frozen request",
        ));
    }
    Ok(())
}

fn mga_ledger_summary<T: Serialize>(
    requested: usize,
    usable: usize,
    minimum_required: usize,
    payload: &T,
    failure_codes: impl IntoIterator<Item = String>,
) -> MultimodReplicateLedgerSummaryV1 {
    let mut counts = BTreeMap::<String, u32>::new();
    for code in failure_codes {
        *counts.entry(code).or_default() += 1;
    }
    MultimodReplicateLedgerSummaryV1 {
        requested: requested as u32,
        usable: usable as u32,
        minimum_required: minimum_required as u32,
        usable_fraction: usable as f64 / requested as f64,
        complete: usable >= minimum_required,
        ledger_sha256: sha256_serialized(payload),
        failure_counts: counts,
        failures: Vec::new(),
    }
}

fn refit_failure_code(failure: &RefitFailureV1) -> String {
    match failure.code {
        RefitFailureCodeV1::Cancelled => "mga.refit.cancelled",
        RefitFailureCodeV1::UnsupportedProfile => "mga.refit.unsupported_profile",
        RefitFailureCodeV1::InsufficientRows => "mga.refit.insufficient_rows",
        RefitFailureCodeV1::SingularModel => "mga.refit.singular_model",
        RefitFailureCodeV1::Nonconvergence => "mga.refit.nonconvergence",
        RefitFailureCodeV1::NonFiniteEstimate => "mga.refit.nonfinite_estimate",
        RefitFailureCodeV1::OrientationUndefined => "mga.refit.orientation_undefined",
        RefitFailureCodeV1::ParameterContractMismatch => "mga.refit.parameter_contract",
        RefitFailureCodeV1::EngineFailure => "mga.refit.engine_failure",
    }
    .into()
}

fn apply_mga_multiplicity(
    rows: &mut [MgaPairwiseComparisonV1],
    method: MultiplicityAdjustmentV1,
) -> Result<(), MultiModRunnerErrorV1> {
    let hypotheses = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            row.raw_p_value.map(|raw_probability| {
                (
                    index,
                    qpls_estimation::HypothesisProbabilityV1 {
                        hypothesis_id: format!(
                            "{}:{}:{}:{}:{}",
                            row.procedure,
                            row.left_group_id,
                            row.right_group_id,
                            row.target_id,
                            index
                        ),
                        raw_probability,
                    },
                )
            })
        })
        .collect::<Vec<_>>();
    if hypotheses.is_empty() {
        return Ok(());
    }
    let adjusted = adjust_probabilities_v1(
        &hypotheses
            .iter()
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>(),
        multiplicity_method(method),
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    for ((row_index, _), value) in hypotheses.into_iter().zip(adjusted) {
        rows[row_index].adjusted_p_value = Some(value.adjusted_probability);
    }
    Ok(())
}

pub fn run_compiled_mga_multigroup_v1<R, C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    prepared: &PreparedMgaExecutionV1,
    refitter: &mut R,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    R: MultigroupRefitterV1,
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    run_compiled_mga_multigroup_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        prepared,
        refitter,
        None,
        None,
        None,
        should_cancel,
        progress,
    )
}

/// Builds the immutable prepared-kernel execution graph used by the external
/// MGA resume cache. Dataset/model authority is revalidated by the run call.
pub fn prepare_compiled_mga_execution_plan_v1(
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledMultiModRecipeV1,
    prepared: &PreparedMgaExecutionV1,
) -> Result<MgaExecutionPlanV1, MultiModRunnerErrorV1> {
    let config = recipe
        .mga_multigroup
        .as_ref()
        .ok_or_else(|| MultiModRunnerErrorV1::Authority("MGA configuration is absent".into()))?;
    let pairs = selected_mga_pairs(config)?;
    build_mga_execution_plan_v1(
        artifact.receipt(),
        &artifact.receipt().dataset_fingerprint,
        config,
        &prepared.design,
        &prepared.parameters,
        &pairs,
    )
    .map_err(map_mga_execution_cache_error_v1)
}

#[derive(Debug)]
pub struct ResumableMgaRunV1 {
    pub output: MultiModRunOutputV1,
    pub execution_plan: MgaExecutionPlanV1,
    pub finalized_cache_sha256: String,
}

/// Executes or resumes the prepared scientific-kernel graph. The public result
/// is returned only after every immutable shard, including multiplicity
/// aggregation, has a validated completed payload.
#[allow(clippy::too_many_arguments)]
pub fn run_compiled_mga_multigroup_resumable_v1<R, C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    prepared: &PreparedMgaExecutionV1,
    refitter: &mut R,
    cache: &mut MgaExecutionCacheV1,
    should_cancel: C,
    progress: P,
) -> Result<ResumableMgaRunV1, MultiModRunnerErrorV1>
where
    R: MultigroupRefitterV1,
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    let execution_plan = prepare_compiled_mga_execution_plan_v1(recipe, artifact, prepared)?;
    let mut execution_session = ValidatedMgaExecutionCacheSessionV1::open(&execution_plan, cache)
        .map_err(map_mga_execution_cache_error_v1)?;
    let output = run_compiled_mga_multigroup_internal_v1(
        dataset,
        recipe,
        model,
        artifact,
        prepared,
        refitter,
        None,
        Some(&mut execution_session),
        None,
        should_cancel,
        progress,
    )?;
    let finalized_cache_sha256 = execution_session
        .finalized_identity_sha256()
        .map_err(map_mga_execution_cache_error_v1)?;
    drop(execution_session);
    Ok(ResumableMgaRunV1 {
        output,
        execution_plan,
        finalized_cache_sha256,
    })
}

#[allow(clippy::too_many_arguments)]
fn run_compiled_mga_multigroup_internal_v1<R, C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    prepared: &PreparedMgaExecutionV1,
    refitter: &mut R,
    kernel_overrides: Option<&MgaKernelOverridesV1>,
    mut execution_cache: Option<&mut ValidatedMgaExecutionCacheSessionV1<'_>>,
    mut checkpoint: Option<&mut MgaShardCheckpointCallbackV1<'_>>,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    R: MultigroupRefitterV1,
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
        "mga:authority",
    );
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::MgaMultigroupV1,
    )?;
    let config = recipe.mga_multigroup.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority("MGA configuration disappeared after compilation".into())
    })?;
    validate_mga_prepared(dataset, config, artifact.plan(), prepared, kernel_overrides)?;
    let pairs = selected_mga_pairs(config)?;
    if let Some(overrides) = kernel_overrides {
        validate_frequency_kernel_overrides_v1(config, &pairs, prepared, overrides)?;
    }
    let frozen_eligibility = kernel_overrides
        .map(|overrides| overrides.eligibility.clone())
        .unwrap_or_else(|| assess_multigroup_design_v1(&prepared.design));
    let execution_plan = if execution_cache.is_some() {
        let plan = build_mga_execution_plan_v1(
            artifact.receipt(),
            &artifact.receipt().dataset_fingerprint,
            config,
            &prepared.design,
            &prepared.parameters,
            &pairs,
        )
        .map_err(map_mga_execution_cache_error_v1)?;
        if execution_cache
            .as_deref()
            .expect("execution cache presence checked above")
            .plan()
            .plan_sha256
            != plan.plan_sha256
        {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "validated session plan differs from the reconstructed prepared plan".into(),
            ));
        }
        Some(plan)
    } else {
        None
    };
    let permutation = config
        .procedures
        .contains(&MgaProcedureV1::PairwisePermutation);
    let omnibus = config
        .procedures
        .contains(&MgaProcedureV1::OmnibusMaxSpreadPermutation);
    let bootstrap = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::HenselerPlsMga | MgaProcedureV1::BootstrapDifferenceBc
        )
    });
    let requests_parametric = config.procedures.iter().any(|procedure| {
        matches!(
            procedure,
            MgaProcedureV1::ParametricPooledVariance
                | MgaProcedureV1::ParametricWelchSatterthwaite
                | MgaProcedureV1::ParametricWaldOmnibus
        )
    });
    if requests_parametric && config.profile != qpls_core::MgaModelProfileV1::GeneralSemPls {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.mga.parametric_se_semantics_unavailable_for_profile: pooled/Welch/Wald sensitivity is qualified only for ordinary General SEM PLS structural paths"
                .into(),
        ));
    }
    if requests_parametric {
        if config.selected_parameter_ids.is_empty() {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(
                "multimod.runner.mga.parametric_explicit_path_targets_required".into(),
            ));
        }
        let authority = projected_ordinary_pls_authority_v1(recipe, model, config)?;
        let authoritative = ordinary_pls_parameter_projections_with_technical_v1(
            config,
            authority.point_model(),
            authority.plan(),
            authority.technical_construct_ids(),
        )?;
        if authoritative
            .iter()
            .any(|projection| projection.identity.family != ParameterFamilyV1::StructuralPath)
            || authoritative
                .iter()
                .map(|projection| &projection.identity)
                .ne(prepared.parameters.iter())
        {
            return Err(MultiModRunnerErrorV1::UnsupportedProfile(
                "multimod.runner.mga.parametric_structural_path_targets_only: selected target identities are not compiler-proven ordinary PLS paths"
                    .into(),
            ));
        }
    }
    let group_count = prepared.design.groups.len() as u64;
    let total_fit_units = (if permutation {
        pairs.len() as u64 * (2 + 2 * u64::from(config.permutation_samples))
    } else {
        0
    }) + (if omnibus {
        group_count * (1 + u64::from(config.permutation_samples))
    } else {
        0
    }) + (if bootstrap {
        group_count * (1 + u64::from(config.bootstrap_samples))
    } else {
        0
    });
    let mut controlled = ControlledMgaRefitter {
        inner: refitter,
        should_cancel: &should_cancel,
        progress: &progress,
        completed: 0,
        total: total_fit_units.max(1),
        cancelled: false,
    };
    let permutation_config = MultigroupResamplingConfigV1 {
        requested: config.permutation_samples as usize,
        seed: config.seed,
        confidence_level: config.confidence_level,
        alpha: config.alpha,
        alternative: core_alternative(config.alternative),
    };
    let bootstrap_config = MultigroupResamplingConfigV1 {
        requested: config.bootstrap_samples as usize,
        ..permutation_config
    };
    let mut group_points = BTreeMap::<(usize, String), f64>::new();
    let mut pairwise = Vec::<MgaPairwiseComparisonV1>::new();
    let mut omnibus_rows = Vec::<MgaOmnibusComparisonV1>::new();
    let mut ledgers = Vec::<MultimodReplicateLedgerSummaryV1>::new();
    let mut evidence = Vec::<MultiModRunnerEvidenceV1>::new();
    evidence.extend(
        prepared
            .pairwise_partition_plans
            .iter()
            .cloned()
            .map(MultiModRunnerEvidenceV1::MgaPairwisePartitionPlan),
    );
    if let Some(overrides) = kernel_overrides {
        evidence.extend(
            overrides
                .frequency_pairwise_plans
                .iter()
                .cloned()
                .map(MultiModRunnerEvidenceV1::MgaFrequencyPairwisePartitionPlan),
        );
    }
    if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
        for group in &prepared.design.groups {
            let expected = prepared
                .observed_group_parameters
                .iter()
                .find(|vector| vector.group == group.index)
                .cloned();
            let kind = MgaExecutionShardKindV1::PointFit { group: group.index };
            let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                cache,
                &kind,
                || should_cancel(),
                || {
                    expected
                        .clone()
                        .map(|value| MgaExecutionShardPayloadV1::PointFit {
                            value,
                            ordinary_path_standard_errors: Vec::new(),
                        })
                        .ok_or_else(|| {
                            MgaExecutionCacheErrorV1::ExecutionFailed(format!(
                                "raw/prepared point fit for group {} was not supplied to its planned shard",
                                group.index.get()
                            ))
                        })
                },
                checkpoint.as_deref_mut(),
            )
            .map_err(map_mga_execution_cache_error_v1)?;
            let MgaExecutionShardPayloadV1::PointFit { value, .. } = payload else {
                unreachable!("cache validates point payload against shard kind")
            };
            if expected.as_ref().is_some_and(|expected| expected != &value) {
                return Err(MultiModRunnerErrorV1::ExecutionCache(format!(
                    "point-fit payload differs from the deterministic raw fit for group {}",
                    group.index.get()
                )));
            }
            if value.values.len() != prepared.parameters.len() {
                return Err(MultiModRunnerErrorV1::ExecutionCache(
                    "cached point-fit dimension differs from the target inventory".into(),
                ));
            }
            for (parameter, estimate) in prepared.parameters.iter().zip(&value.values) {
                insert_group_point(&mut group_points, value.group, parameter, *estimate)?;
            }
        }
    } else {
        for vector in &prepared.observed_group_parameters {
            if vector.group.get() >= prepared.design.groups.len()
                || vector.values.len() != prepared.parameters.len()
            {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "prepared observed group point vector has an invalid group or dimension".into(),
                ));
            }
            for (parameter, estimate) in prepared.parameters.iter().zip(&vector.values) {
                insert_group_point(&mut group_points, vector.group, parameter, *estimate)?;
            }
        }
    }

    if permutation {
        for pair in &pairs {
            let comparable_targets = prepared_pair_comparable_targets_v1(prepared, *pair)?;
            let result = if let Some(overrides) = kernel_overrides {
                overrides.pairwise(*pair).cloned().ok_or_else(|| {
                    MultiModRunnerErrorV1::InvalidLedger(format!(
                        "frequency pairwise result is missing for {}",
                        pairwise_plan_key_v1(*pair)
                    ))
                })?
            } else if let (Some(_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                let kind = MgaExecutionShardKindV1::PairwisePermutation { pair: *pair };
                let partition_plan = prepared_pairwise_plan_v1(prepared, *pair)
                    .expect("prepared pairwise plan inventory validated above");
                let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &kind,
                    || should_cancel(),
                    || {
                        let result = run_pairwise_permutation_with_plan_v1(
                            &prepared.design,
                            *pair,
                            &prepared.parameters,
                            permutation_config,
                            partition_plan,
                            &mut controlled,
                        );
                        if controlled.cancelled || should_cancel() {
                            return Err(MgaExecutionCacheErrorV1::Cancelled);
                        }
                        result
                            .map(|value| MgaExecutionShardPayloadV1::PairwisePermutation { value })
                            .map_err(|error| {
                                MgaExecutionCacheErrorV1::ExecutionFailed(error.to_string())
                            })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?;
                let MgaExecutionShardPayloadV1::PairwisePermutation { value } = payload else {
                    unreachable!("cache validates permutation payload against shard kind")
                };
                value
            } else {
                let plan = prepared_pairwise_plan_v1(prepared, *pair)
                    .expect("prepared pairwise plan inventory validated above");
                let result = run_pairwise_permutation_with_plan_v1(
                    &prepared.design,
                    *pair,
                    &prepared.parameters,
                    permutation_config,
                    plan,
                    &mut controlled,
                );
                if controlled.cancelled || should_cancel() {
                    return Err(MultiModRunnerErrorV1::Cancelled);
                }
                result.map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?
            };
            if kernel_overrides.is_none() {
                let partition_plan = prepared_pairwise_plan_v1(prepared, *pair)
                    .expect("prepared pairwise plan inventory validated above");
                validate_cached_pairwise_permutation_v1(
                    &result,
                    *pair,
                    &prepared.parameters,
                    permutation_config,
                    partition_plan,
                    &frozen_eligibility.group_counts,
                )?;
            }
            if result.availability != InferenceAvailabilityV1::Available {
                return Err(MultiModRunnerErrorV1::Kernel(format!(
                    "pairwise permutation has {} usable draws; {} required",
                    result.usable, result.minimum_usable
                )));
            }
            evidence.push(MultiModRunnerEvidenceV1::MgaPairwisePermutation(
                result.clone(),
            ));
            let left_id = &config.groups[pair.group_a.get()].group_id;
            let right_id = &config.groups[pair.group_b.get()].group_id;
            for point in &result.point_estimates {
                insert_group_point(
                    &mut group_points,
                    pair.group_a,
                    &point.parameter,
                    point.estimate_a,
                )?;
                insert_group_point(
                    &mut group_points,
                    pair.group_b,
                    &point.parameter,
                    point.estimate_b,
                )?;
            }
            for parameter in &result.parameters {
                let comparable = comparable_targets.contains(&parameter.parameter.stable_id);
                pairwise.push(MgaPairwiseComparisonV1 {
                    procedure: if kernel_overrides.is_some() {
                        "frequency_count_space_pairwise_permutation_v1"
                    } else {
                        "pairwise_permutation_two_tailed_or_predeclared_direction_v1"
                    }
                    .into(),
                    left_group_id: left_id.clone(),
                    right_group_id: right_id.clone(),
                    target_id: parameter.parameter.stable_id.clone(),
                    difference_left_minus_right: parameter.difference_a_minus_b,
                    raw_p_value: Some(parameter.selected_probability),
                    adjusted_p_value: None,
                    directional_probability: None,
                    interval: None,
                    measurement_comparability_satisfied: comparable,
                    interpretation_blocked: !comparable,
                });
            }
            let failures = result.ledger.iter().flat_map(|entry| {
                entry
                    .group_fits
                    .iter()
                    .filter_map(|fit| fit.failure.as_ref().map(refit_failure_code))
            });
            ledgers.push(mga_ledger_summary(
                result.requested,
                result.usable,
                result.minimum_usable,
                &result.ledger,
                failures,
            ));
        }
    }

    if omnibus {
        let result = if let Some(overrides) = kernel_overrides {
            overrides.omnibus_permutation.clone().ok_or_else(|| {
                MultiModRunnerErrorV1::InvalidLedger(
                    "frequency omnibus result is missing from the kernel override".into(),
                )
            })?
        } else if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut())
        {
            let kind = MgaExecutionShardKindV1::OmnibusPermutation;
            let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                cache,
                &kind,
                || should_cancel(),
                || {
                    let result = run_max_spread_omnibus_permutation_v1(
                        &prepared.design,
                        &prepared.parameters,
                        permutation_config,
                        &mut controlled,
                    );
                    if controlled.cancelled || should_cancel() {
                        return Err(MgaExecutionCacheErrorV1::Cancelled);
                    }
                    result
                        .map(|value| MgaExecutionShardPayloadV1::OmnibusPermutation { value })
                        .map_err(|error| {
                            MgaExecutionCacheErrorV1::ExecutionFailed(error.to_string())
                        })
                },
                checkpoint.as_deref_mut(),
            )
            .map_err(map_mga_execution_cache_error_v1)?;
            let MgaExecutionShardPayloadV1::OmnibusPermutation { value } = payload else {
                unreachable!("cache validates omnibus payload against shard kind")
            };
            value
        } else {
            let result = run_max_spread_omnibus_permutation_v1(
                &prepared.design,
                &prepared.parameters,
                permutation_config,
                &mut controlled,
            );
            if controlled.cancelled || should_cancel() {
                return Err(MultiModRunnerErrorV1::Cancelled);
            }
            result.map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?
        };
        if kernel_overrides.is_none() {
            validate_cached_omnibus_permutation_v1(
                &result,
                &prepared.design,
                &prepared.parameters,
                permutation_config,
                &frozen_eligibility.group_counts,
            )?;
        }
        if result.availability != InferenceAvailabilityV1::Available {
            return Err(MultiModRunnerErrorV1::Kernel(format!(
                "omnibus permutation has {} usable draws; {} required",
                result.usable, result.minimum_usable
            )));
        }
        evidence.push(MultiModRunnerEvidenceV1::MgaOmnibusPermutation(
            result.clone(),
        ));
        for vector in &result.group_point_estimates {
            for (parameter, estimate) in prepared.parameters.iter().zip(&vector.values) {
                insert_group_point(&mut group_points, vector.group, parameter, *estimate)?;
            }
        }
        for parameter in &result.parameters {
            omnibus_rows.push(MgaOmnibusComparisonV1 {
                procedure: if kernel_overrides.is_some() {
                    "frequency_count_space_max_spread_omnibus_permutation_v1"
                } else {
                    "max_spread_omnibus_permutation_v1"
                }
                .into(),
                target_id: parameter.parameter.stable_id.clone(),
                statistic: parameter.observed_maximum_pairwise_spread,
                degrees_of_freedom: (prepared.design.groups.len() - 1) as u32,
                p_value: parameter.p_value_right_tailed,
            });
        }
        let failures = result.ledger.iter().flat_map(|entry| {
            entry
                .group_fits
                .iter()
                .filter_map(|fit| fit.failure.as_ref().map(refit_failure_code))
        });
        ledgers.push(mga_ledger_summary(
            result.requested,
            result.usable,
            result.minimum_usable,
            &result.ledger,
            failures,
        ));
    }

    let bootstrap_banks = if bootstrap {
        let banks = if let Some(overrides) = kernel_overrides {
            overrides.bootstrap_banks.clone().ok_or_else(|| {
                MultiModRunnerErrorV1::InvalidLedger(
                    "frequency bootstrap banks are missing from the kernel override".into(),
                )
            })?
        } else if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut())
        {
            let kind = MgaExecutionShardKindV1::SharedGroupBootstrapBanks;
            let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                cache,
                &kind,
                || should_cancel(),
                || {
                    let result = run_group_bootstrap_banks_v1(
                        &prepared.design,
                        &prepared.parameters,
                        bootstrap_config,
                        &mut controlled,
                    );
                    if controlled.cancelled || should_cancel() {
                        return Err(MgaExecutionCacheErrorV1::Cancelled);
                    }
                    result
                        .map(
                            |value| MgaExecutionShardPayloadV1::SharedGroupBootstrapBanks { value },
                        )
                        .map_err(|error| {
                            MgaExecutionCacheErrorV1::ExecutionFailed(error.to_string())
                        })
                },
                checkpoint.as_deref_mut(),
            )
            .map_err(map_mga_execution_cache_error_v1)?;
            let MgaExecutionShardPayloadV1::SharedGroupBootstrapBanks { value } = payload else {
                unreachable!("cache validates bootstrap payload against shard kind")
            };
            value
        } else {
            let banks = run_group_bootstrap_banks_v1(
                &prepared.design,
                &prepared.parameters,
                bootstrap_config,
                &mut controlled,
            );
            if controlled.cancelled || should_cancel() {
                return Err(MultiModRunnerErrorV1::Cancelled);
            }
            banks.map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?
        };
        validate_cached_bootstrap_banks_v1(
            &banks,
            if kernel_overrides.is_some() {
                FREQUENCY_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1
            } else {
                MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1
            },
            &prepared.design,
            &prepared.parameters,
            bootstrap_config,
            &frozen_eligibility.group_counts,
        )?;
        if banks.availability != InferenceAvailabilityV1::Available {
            return Err(MultiModRunnerErrorV1::Kernel(
                "group bootstrap banks did not meet the minimum usable-draw requirement".into(),
            ));
        }
        evidence.push(MultiModRunnerEvidenceV1::MgaBootstrapBanks(banks.clone()));
        for bank in &banks.groups {
            for (parameter, estimate) in prepared.parameters.iter().zip(&bank.point_estimates) {
                insert_group_point(&mut group_points, bank.group, parameter, *estimate)?;
            }
        }
        let failures = banks.ledger.iter().flat_map(|entry| {
            entry
                .groups
                .iter()
                .filter_map(|fit| fit.failure.as_ref().map(refit_failure_code))
        });
        let usable = banks
            .ledger
            .iter()
            .filter(|entry| entry.status == ResampleFitStatusV1::Usable)
            .count();
        ledgers.push(mga_ledger_summary(
            banks.requested,
            usable,
            banks.minimum_usable,
            &banks.ledger,
            failures,
        ));
        Some(banks)
    } else {
        None
    };

    if config.procedures.contains(&MgaProcedureV1::HenselerPlsMga) {
        let banks = bootstrap_banks
            .as_ref()
            .expect("bootstrap bank selected above");
        for pair in &pairs {
            let comparable_targets = prepared_pair_comparable_targets_v1(prepared, *pair)?;
            let compute_rows = || {
                henseler_directional_probabilities_v1(banks, *pair, config.alpha)
                    .map_err(|error| error.to_string())
                    .map(|parameters| {
                        parameters
                            .into_iter()
                            .map(|parameter| {
                                let comparable =
                                    comparable_targets.contains(&parameter.parameter.stable_id);
                                MgaPairwiseComparisonV1 {
                                    procedure: if kernel_overrides.is_some() {
                                        "frequency_count_space_henseler_pls_mga_directional_probability_v1"
                                    } else {
                                        "henseler_pls_mga_directional_probability_v1"
                                    }
                                    .into(),
                                    left_group_id: config.groups[pair.group_a.get()]
                                        .group_id
                                        .clone(),
                                    right_group_id: config.groups[pair.group_b.get()]
                                        .group_id
                                        .clone(),
                                    target_id: parameter.parameter.stable_id,
                                    difference_left_minus_right: parameter
                                        .point_difference_a_minus_b,
                                    raw_p_value: None,
                                    adjusted_p_value: None,
                                    directional_probability: Some(
                                        parameter.directional_probability_a_greater,
                                    ),
                                    interval: None,
                                    measurement_comparability_satisfied: comparable,
                                    interpretation_blocked: !comparable,
                                }
                            })
                            .collect::<Vec<_>>()
                    })
            };
            let rows = if let (Some(_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                let kind = MgaExecutionShardKindV1::PairwiseBootstrapDerived {
                    procedure: MgaProcedureV1::HenselerPlsMga,
                    pair: *pair,
                };
                let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &kind,
                    || should_cancel(),
                    || {
                        compute_rows()
                            .map(|rows| MgaExecutionShardPayloadV1::PairwiseRows {
                                procedure: MgaProcedureV1::HenselerPlsMga,
                                pair: *pair,
                                rows,
                            })
                            .map_err(MgaExecutionCacheErrorV1::ExecutionFailed)
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?;
                let MgaExecutionShardPayloadV1::PairwiseRows { rows, .. } = payload else {
                    unreachable!("cache validates Henseler payload against shard kind")
                };
                rows
            } else {
                compute_rows().map_err(MultiModRunnerErrorV1::Kernel)?
            };
            let expected_rows = compute_rows().map_err(MultiModRunnerErrorV1::Kernel)?;
            if rows != expected_rows {
                return Err(cached_payload_error_v1(
                    "Henseler pairwise rows differ from the retained shared bootstrap bank",
                ));
            }
            pairwise.extend(rows);
        }
    }

    if config
        .procedures
        .contains(&MgaProcedureV1::BootstrapDifferenceBc)
    {
        let banks = bootstrap_banks
            .as_ref()
            .expect("bootstrap bank selected above");
        for pair in &pairs {
            let comparable_targets = prepared_pair_comparable_targets_v1(prepared, *pair)?;
            let left = &banks.groups[pair.group_a.get()];
            let right = &banks.groups[pair.group_b.get()];
            let compute_rows = || -> Result<Vec<MgaPairwiseComparisonV1>, MultiModRunnerErrorV1> {
                let mut rows = Vec::with_capacity(prepared.parameters.len());
                for (parameter_index, parameter) in prepared.parameters.iter().enumerate() {
                    let draws = left
                        .replicate_estimates
                        .iter()
                        .zip(&right.replicate_estimates)
                        .filter_map(|(left, right)| {
                            Some(left.as_ref()?[parameter_index] - right.as_ref()?[parameter_index])
                        })
                        .collect::<Vec<_>>();
                    if draws.len() < banks.minimum_usable {
                        return Err(MultiModRunnerErrorV1::Kernel(format!(
                            "BC difference {} has {} usable draws; {} required",
                            parameter.stable_id,
                            draws.len(),
                            banks.minimum_usable
                        )));
                    }
                    let point = left.point_estimates[parameter_index]
                        - right.point_estimates[parameter_index];
                    let bc = bias_corrected_interval_for_alternative_v1(
                        point,
                        &draws,
                        config.confidence_level,
                        core_alternative(config.alternative),
                    )
                    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                    let comparable = comparable_targets.contains(&parameter.stable_id);
                    rows.push(MgaPairwiseComparisonV1 {
                        procedure: if kernel_overrides.is_some() {
                            "frequency_count_space_bootstrap_difference_bc_zero_acceleration_v1"
                        } else {
                            "bootstrap_difference_bc_zero_acceleration_v1"
                        }
                        .into(),
                        left_group_id: config.groups[pair.group_a.get()].group_id.clone(),
                        right_group_id: config.groups[pair.group_b.get()].group_id.clone(),
                        target_id: parameter.stable_id.clone(),
                        difference_left_minus_right: point,
                        raw_p_value: Some(empirical_zero_probability(&draws, config.alternative)),
                        adjusted_p_value: None,
                        directional_probability: None,
                        interval: Some(interval(
                            "bias_corrected_zero_acceleration_bc",
                            config.confidence_level,
                            config.alternative,
                            bc.lower,
                            bc.upper,
                        )?),
                        measurement_comparability_satisfied: comparable,
                        interpretation_blocked: !comparable,
                    });
                }
                Ok(rows)
            };
            let rows = if let (Some(_plan), Some(cache)) =
                (&execution_plan, execution_cache.as_deref_mut())
            {
                let kind = MgaExecutionShardKindV1::PairwiseBootstrapDerived {
                    procedure: MgaProcedureV1::BootstrapDifferenceBc,
                    pair: *pair,
                };
                let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &kind,
                    || should_cancel(),
                    || {
                        compute_rows()
                            .map(|rows| MgaExecutionShardPayloadV1::PairwiseRows {
                                procedure: MgaProcedureV1::BootstrapDifferenceBc,
                                pair: *pair,
                                rows,
                            })
                            .map_err(|error| {
                                if matches!(error, MultiModRunnerErrorV1::Cancelled) {
                                    MgaExecutionCacheErrorV1::Cancelled
                                } else {
                                    MgaExecutionCacheErrorV1::ExecutionFailed(error.to_string())
                                }
                            })
                    },
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?;
                let MgaExecutionShardPayloadV1::PairwiseRows { rows, .. } = payload else {
                    unreachable!("cache validates BC payload against shard kind")
                };
                rows
            } else {
                compute_rows()?
            };
            let expected_rows = compute_rows()?;
            if rows != expected_rows {
                return Err(cached_payload_error_v1(
                    "BC pairwise rows differ from the retained shared bootstrap bank",
                ));
            }
            pairwise.extend(rows);
        }
    }

    if requests_parametric {
        let expected = prepared
            .parameters
            .iter()
            .map(|parameter| parameter.stable_id.as_str())
            .collect::<BTreeSet<_>>();
        let actual = prepared
            .parametric_cells
            .iter()
            .map(|cell| cell.parameter.stable_id.as_str())
            .collect::<BTreeSet<_>>();
        if prepared
            .parameters
            .iter()
            .any(|parameter| parameter.family != ParameterFamilyV1::StructuralPath)
            || actual != expected
            || prepared.parametric_cells.len() != expected.len()
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "parametric sensitivity requires exactly one structural-path SE cell for every selected parameter"
                    .into(),
            ));
        }
        for cell in &prepared.parametric_cells {
            for estimate in &cell.group_estimates {
                qpls_estimation::validate_parametric_group_estimate_v1(*estimate)
                    .map_err(|error| MultiModRunnerErrorV1::PreparedInput(error.to_string()))?;
            }
        }
    } else if !prepared.parametric_cells.is_empty() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "parametric sensitivity cells were supplied without a parametric procedure".into(),
        ));
    }

    for cell in &prepared.parametric_cells {
        if !prepared.parameters.contains(&cell.parameter)
            || cell.group_estimates.len() != prepared.design.groups.len()
            || cell
                .group_estimates
                .iter()
                .enumerate()
                .any(|(index, estimate)| estimate.group.get() != index)
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "parametric sensitivity cells must contain every group in canonical order".into(),
            ));
        }
        for estimate in &cell.group_estimates {
            insert_group_point(
                &mut group_points,
                estimate.group,
                &cell.parameter,
                estimate.estimate,
            )?;
        }
        for pair in &pairs {
            let comparable_targets = prepared_pair_comparable_targets_v1(prepared, *pair)?;
            for procedure in &config.procedures {
                let test = match procedure {
                    MgaProcedureV1::ParametricPooledVariance => {
                        Some(pooled_variance_parameter_test_v1(
                            cell.group_estimates[pair.group_a.get()],
                            cell.group_estimates[pair.group_b.get()],
                            core_alternative(config.alternative),
                        ))
                    }
                    MgaProcedureV1::ParametricWelchSatterthwaite => {
                        Some(welch_satterthwaite_parameter_test_v1(
                            cell.group_estimates[pair.group_a.get()],
                            cell.group_estimates[pair.group_b.get()],
                            core_alternative(config.alternative),
                        ))
                    }
                    _ => None,
                };
                if let Some(test) = test {
                    let test =
                        test.map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                    evidence.push(MultiModRunnerEvidenceV1::MgaPairwiseParametric(
                        test.clone(),
                    ));
                    let comparable = comparable_targets.contains(&cell.parameter.stable_id);
                    pairwise.push(MgaPairwiseComparisonV1 {
                        procedure: match procedure {
                            MgaProcedureV1::ParametricPooledVariance => {
                                "parametric_pooled_variance_sensitivity_v1"
                            }
                            MgaProcedureV1::ParametricWelchSatterthwaite => {
                                "parametric_welch_satterthwaite_sensitivity_v1"
                            }
                            _ => unreachable!(),
                        }
                        .into(),
                        left_group_id: config.groups[pair.group_a.get()].group_id.clone(),
                        right_group_id: config.groups[pair.group_b.get()].group_id.clone(),
                        target_id: cell.parameter.stable_id.clone(),
                        difference_left_minus_right: test.difference_a_minus_b,
                        raw_p_value: Some(test.selected_probability),
                        adjusted_p_value: None,
                        directional_probability: None,
                        interval: None,
                        measurement_comparability_satisfied: comparable,
                        interpretation_blocked: !comparable,
                    });
                }
            }
        }
        if config
            .procedures
            .contains(&MgaProcedureV1::ParametricWaldOmnibus)
        {
            let wald_inputs = cell
                .group_estimates
                .iter()
                .map(|estimate| WaldGroupEstimateV1 {
                    group: estimate.group,
                    estimate: estimate.estimate,
                    standard_error: estimate.standard_error,
                })
                .collect::<Vec<_>>();
            let wald = inverse_variance_wald_test_v1(&wald_inputs)
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            evidence.push(MultiModRunnerEvidenceV1::MgaParametricWald(wald.clone()));
            omnibus_rows.push(MgaOmnibusComparisonV1 {
                procedure: "inverse_variance_wald_sensitivity_v1".into(),
                target_id: cell.parameter.stable_id.clone(),
                statistic: wald.chi_square,
                degrees_of_freedom: wald.degrees_of_freedom as u32,
                p_value: wald.p_value_right_tailed,
            });
        }
    }
    if requests_parametric && prepared.parametric_cells.len() != prepared.parameters.len() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "each selected parameter requires a qualified parametric sensitivity cell".into(),
        ));
    }
    if requests_parametric
        && let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut())
    {
        for pair in &pairs {
            for procedure in config.procedures.iter().copied().filter(|procedure| {
                matches!(
                    procedure,
                    MgaProcedureV1::ParametricPooledVariance
                        | MgaProcedureV1::ParametricWelchSatterthwaite
                )
            }) {
                let procedure_id = match procedure {
                    MgaProcedureV1::ParametricPooledVariance => {
                        "parametric_pooled_variance_sensitivity_v1"
                    }
                    MgaProcedureV1::ParametricWelchSatterthwaite => {
                        "parametric_welch_satterthwaite_sensitivity_v1"
                    }
                    _ => unreachable!(),
                };
                let rows = pairwise
                    .iter()
                    .filter(|row| {
                        row.procedure == procedure_id
                            && row.left_group_id == config.groups[pair.group_a.get()].group_id
                            && row.right_group_id == config.groups[pair.group_b.get()].group_id
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let tests = evidence
                    .iter()
                    .filter_map(|item| match item {
                        MultiModRunnerEvidenceV1::MgaPairwiseParametric(test)
                            if test.pair == *pair
                                && match procedure {
                                    MgaProcedureV1::ParametricPooledVariance => matches!(
                                        test.method,
                                        qpls_estimation::PairwiseParametricMethodV1::PooledEqualResidualVariance
                                    ),
                                    MgaProcedureV1::ParametricWelchSatterthwaite => matches!(
                                        test.method,
                                        qpls_estimation::PairwiseParametricMethodV1::WelchSatterthwaite
                                    ),
                                    _ => false,
                                } => Some(test.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let kind = MgaExecutionShardKindV1::ParametricPair {
                    procedure,
                    pair: *pair,
                };
                let expected = MgaExecutionShardPayloadV1::ParametricPairRows {
                    procedure,
                    pair: *pair,
                    rows,
                    tests,
                };
                let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                    cache,
                    &kind,
                    || should_cancel(),
                    || Ok(expected.clone()),
                    checkpoint.as_deref_mut(),
                )
                .map_err(map_mga_execution_cache_error_v1)?;
                if payload != expected {
                    return Err(MultiModRunnerErrorV1::ExecutionCache(format!(
                        "cached parametric shard differs for {}",
                        pairwise_plan_key_v1(*pair)
                    )));
                }
            }
        }
        if config
            .procedures
            .contains(&MgaProcedureV1::ParametricWaldOmnibus)
        {
            let rows = omnibus_rows
                .iter()
                .filter(|row| row.procedure == "inverse_variance_wald_sensitivity_v1")
                .cloned()
                .collect::<Vec<_>>();
            let tests = evidence
                .iter()
                .filter_map(|item| match item {
                    MultiModRunnerEvidenceV1::MgaParametricWald(test) => Some(test.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let expected = MgaExecutionShardPayloadV1::ParametricWaldOmnibus {
                output_identity_sha256: sha256_serialized(&rows),
                tests,
            };
            let payload = execute_or_reuse_mga_shard_checkpointed_v1(
                cache,
                &MgaExecutionShardKindV1::ParametricWaldOmnibus,
                || should_cancel(),
                || Ok(expected.clone()),
                checkpoint.as_deref_mut(),
            )
            .map_err(map_mga_execution_cache_error_v1)?;
            if payload != expected {
                return Err(MultiModRunnerErrorV1::ExecutionCache(
                    "cached Wald omnibus shard differs from the deterministic sensitivity cells"
                        .into(),
                ));
            }
        }
    }

    let effective_micom_pairs = if config.procedures.contains(&MgaProcedureV1::MicomPairwise) {
        let rows = if let (Some(_plan), Some(cache)) =
            (&execution_plan, execution_cache.as_deref_mut())
        {
            let mut cached_rows = Vec::new();
            for pair in &pairs {
                let kind = MgaExecutionShardKindV1::MicomPair { pair: *pair };
                let payload = cache
                    .payload(&kind)
                    .map_err(map_mga_execution_cache_error_v1)?
                    .cloned()
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::ExecutionCache(format!(
                            "MICOM shard {} was not completed by the raw pairwise refit stage",
                            pairwise_plan_key_v1(*pair)
                        ))
                    })?;
                let MgaExecutionShardPayloadV1::MicomPair { rows, .. } = payload else {
                    unreachable!("cache validates MICOM payload against shard kind")
                };
                cached_rows.extend(rows);
            }
            if !prepared.micom_pairs.is_empty() && prepared.micom_pairs != cached_rows {
                return Err(MultiModRunnerErrorV1::ExecutionCache(
                    "cached MICOM public rows differ from the deterministic raw preparation".into(),
                ));
            }
            cached_rows
        } else {
            prepared.micom_pairs.clone()
        };
        if rows.is_empty()
            || rows.iter().any(|row| {
                !row.configural_invariance_confirmed
                    || !row.compositional_correlation.is_finite()
                    || !row.compositional_lower_quantile.is_finite()
                    || !row.compositional_p_value.is_finite()
                    || !(0.0..=1.0).contains(&row.compositional_p_value)
                    || !row.equal_mean_p_value.is_finite()
                    || !(0.0..=1.0).contains(&row.equal_mean_p_value)
                    || !row.equal_variance_p_value.is_finite()
                    || !(0.0..=1.0).contains(&row.equal_variance_p_value)
                    || row.compositional_invariance
                        != (row.compositional_correlation >= row.compositional_lower_quantile)
                    || row.partial_invariance
                        != (row.configural_invariance_confirmed && row.compositional_invariance)
            })
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "MICOM procedure requires finite pairwise Steps 2-3 rows bound to confirmed Step 1"
                    .into(),
            ));
        }
        rows
    } else if !prepared.micom_pairs.is_empty() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "prepared MICOM rows were supplied without selecting MICOM".into(),
        ));
    } else {
        Vec::new()
    };

    // This final authority check also covers every resumed pairwise-row shard:
    // a cached AB row cannot inherit the all-pairs intersection or an AC gate.
    validate_pairwise_result_comparability_v1(config, prepared, &pairwise)?;

    report(
        &progress,
        MultiModRunnerPhaseV1::Multiplicity,
        0,
        1,
        "mga:multiplicity",
    );
    if let (Some(_plan), Some(cache)) = (&execution_plan, execution_cache.as_deref_mut()) {
        let kind = MgaExecutionShardKindV1::MultiplicityAggregation;
        let input_rows_sha256 = sha256_serialized(&pairwise);
        let payload = execute_or_reuse_mga_shard_checkpointed_v1(
            cache,
            &kind,
            || should_cancel(),
            || {
                let mut rows = pairwise.clone();
                apply_mga_multiplicity(&mut rows, config.multiplicity).map_err(|error| {
                    MgaExecutionCacheErrorV1::ExecutionFailed(error.to_string())
                })?;
                Ok(MgaExecutionShardPayloadV1::MultiplicityAggregation {
                    input_rows_sha256: input_rows_sha256.clone(),
                    rows,
                })
            },
            checkpoint.as_deref_mut(),
        )
        .map_err(map_mga_execution_cache_error_v1)?;
        let MgaExecutionShardPayloadV1::MultiplicityAggregation {
            input_rows_sha256: cached_input,
            rows,
        } = payload
        else {
            unreachable!("cache validates multiplicity payload against shard kind")
        };
        let mut expected_rows = pairwise.clone();
        apply_mga_multiplicity(&mut expected_rows, config.multiplicity)?;
        if cached_input != input_rows_sha256 || rows != expected_rows {
            return Err(MultiModRunnerErrorV1::ExecutionCache(
                "multiplicity shard input digest or adjusted rows differ from the current unadjusted comparisons"
                    .into(),
            ));
        }
        pairwise = rows;
    } else {
        apply_mga_multiplicity(&mut pairwise, config.multiplicity)?;
    }
    let eligibility = frozen_eligibility;
    let group_eligibility = config
        .groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            let count = eligibility
                .group_counts
                .iter()
                .find(|count| count.group.get() == index)
                .map(|count| count.complete_cases)
                .unwrap_or(0);
            let warnings = eligibility
                .warnings
                .iter()
                .filter(|warning| warning.group.is_none_or(|value| value.get() == index))
                .map(|warning| warning.detail.clone())
                .collect();
            MgaGroupEligibilityV1 {
                group_id: group.group_id.clone(),
                label: group.label.clone(),
                complete_cases: count as u64,
                selected_rows: count as u64,
                eligible: true,
                warnings,
                blockers: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    let mut group_parameters = Vec::new();
    for (group_index, group) in config.groups.iter().enumerate() {
        for parameter in &prepared.parameters {
            let estimate = group_points
                .get(&(group_index, parameter.stable_id.clone()))
                .copied()
                .ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(format!(
                        "no point estimate was produced for group {} parameter {}",
                        group.group_id, parameter.stable_id
                    ))
                })?;
            let standard_error = prepared
                .parametric_cells
                .iter()
                .find(|cell| cell.parameter == *parameter)
                .and_then(|cell| cell.group_estimates.get(group_index))
                .map(|group| group.standard_error);
            group_parameters.push(MgaGroupParameterV1 {
                group_id: group.group_id.clone(),
                parameter: MultimodParameterEstimateV1 {
                    target_id: parameter.stable_id.clone(),
                    target_kind: parameter_family_id(parameter.family).into(),
                    estimate,
                    standard_error,
                    p_value: None,
                    interval: None,
                },
            });
        }
    }
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::Completed,
        1,
        1,
        "mga:complete",
    );
    let analysis = PlsMultigroupAnalysisV1 {
        schema_version: PLS_MULTIGROUP_ANALYSIS_V1_SCHEMA_VERSION,
        provenance: provenance(artifact.receipt(), config.seed),
        profile: config.profile,
        group_eligibility,
        group_parameters,
        micom_pairs: effective_micom_pairs,
        omnibus: omnibus_rows,
        pairwise,
        multiplicity: config.multiplicity,
        replicate_ledgers: ledgers,
        excluded_rows: prepared.excluded_rows.clone(),
        sidecars: Vec::new(),
    };
    let result = MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(analysis);
    result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    Ok(MultiModRunOutputV1 {
        compilation_receipt: artifact.receipt().clone(),
        result,
        evidence,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawHeterogeneityPreparationReceiptV2 {
    pub method_version: String,
    pub general_sem_plan_sha256: String,
    pub pooled_metric_sha256: String,
    pub source_row_tokens: Vec<u64>,
    pub omitted_source_rows: usize,
    pub unique_analysis_positions: bool,
    /// Exact pooled standardized equations supplied to FIMIX. This is
    /// retained evidence, not a second estimator input.
    pub fimix_input: StandardizedFimixInputV2,
}

fn is_lower_hex_sha256_v1(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

impl RawHeterogeneityPreparationReceiptV2 {
    /// Validates the retained raw authority without invoking either FIMIX or
    /// POS. Sidecar publication calls this again so malformed evidence cannot
    /// be serialized even if a caller bypasses the raw runner.
    pub fn ensure_valid(&self) -> Result<(), String> {
        if self.method_version != "qpls.heterogeneity.raw-preparation.v2"
            || !is_lower_hex_sha256_v1(&self.general_sem_plan_sha256)
            || !is_lower_hex_sha256_v1(&self.pooled_metric_sha256)
            || !is_lower_hex_sha256_v1(&self.fimix_input.metric.source_sha256)
            || self.pooled_metric_sha256 != self.fimix_input.metric.source_sha256
            || self.fimix_input.metric.metric_id.trim().is_empty()
            || !self
                .fimix_input
                .metric
                .scores_standardized_once_on_pooled_rows
            || (self.fimix_input.interaction_profile
                != EstimationHeterogeneityProfileV2::P0Structural
                && !self
                    .fimix_input
                    .metric
                    .products_standardized_once_on_pooled_rows)
        {
            return Err(
                "raw heterogeneity receipt has an invalid method, metric, or SHA-256 identity"
                    .to_string(),
            );
        }
        let observations = self.source_row_tokens.len();
        if observations < 2
            || !self.unique_analysis_positions
            || self.source_row_tokens.iter().collect::<BTreeSet<_>>().len() != observations
            || self.fimix_input.metric.observation_count != observations
            || self.omitted_source_rows.checked_add(observations).is_none()
            || self.fimix_input.equations.is_empty()
        {
            return Err(
                "raw heterogeneity receipt has invalid row identity or observation cardinality"
                    .to_string(),
            );
        }
        let mut equation_ids = BTreeSet::new();
        for equation in &self.fimix_input.equations {
            let mut predictors = BTreeSet::new();
            if equation.equation_id.trim().is_empty()
                || equation.outcome_id.trim().is_empty()
                || !equation_ids.insert(equation.equation_id.as_str())
                || equation.predictor_ids.is_empty()
                || equation.predictor_ids.iter().any(|predictor| {
                    predictor.trim().is_empty()
                        || predictor == "(intercept)"
                        || !predictors.insert(predictor.as_str())
                })
                || equation.design.len() != observations
                || equation.outcome.len() != observations
                || equation
                    .design
                    .iter()
                    .any(|row| row.len() != equation.predictor_ids.len())
                || equation
                    .design
                    .iter()
                    .flatten()
                    .chain(&equation.outcome)
                    .any(|value| !value.is_finite())
            {
                return Err(format!(
                    "raw heterogeneity equation {} has invalid identity, cardinality, or finite values",
                    equation.equation_id
                ));
            }
        }
        Ok(())
    }

    fn ensure_matches_live_authority(
        &self,
        dataset_observations: usize,
        source_row_tokens: &[u64],
        general_sem_plan_sha256: &str,
    ) -> Result<(), String> {
        let omitted_source_rows = dataset_observations
            .checked_sub(source_row_tokens.len())
            .ok_or_else(|| {
                "raw heterogeneity receipt contains more source rows than the live dataset"
                    .to_string()
            })?;
        if self.general_sem_plan_sha256 != general_sem_plan_sha256
            || self.source_row_tokens != source_row_tokens
            || self.omitted_source_rows != omitted_source_rows
        {
            return Err(
                "raw heterogeneity receipt differs from the live plan or row-exclusion authority"
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct RawHeterogeneityAuthorityV2 {
    point_recipe: AnalysisRecipeV4,
    point_model: SemModelV4,
    point_artifact: qpls_core::CompiledAnalysisRecipeV4,
    stage_one_execution: ValidatedExecutionRecipe,
    plan: CompiledPlsPlanV3,
    source_columns: Vec<String>,
    blocks: Vec<OrdinaryPlsScoringBlockV1>,
}

fn projected_heterogeneity_authority_v2(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
) -> Result<RawHeterogeneityAuthorityV2, MultiModRunnerErrorV1> {
    let config = recipe.pls_heterogeneity.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "heterogeneity configuration disappeared after compilation".into(),
        )
    })?;
    if !matches!(model.group, SemGroupV4::SingleGroup) {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.heterogeneity.single_group_required".into(),
        ));
    }
    if recipe.settings.case_weight_column.is_some() {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.heterogeneity.weights_unsupported".into(),
        ));
    }
    if recipe.settings.missing_data != MissingDataPolicy::ListwiseDeletion {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.heterogeneity.listwise_complete_scores_required".into(),
        ));
    }
    let general = recipe.general_sem_config.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.heterogeneity.general_sem_config_required".into(),
        )
    })?;
    if general.inference != GeneralSemInferenceV1::None {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.heterogeneity.point_general_sem_authority_required".into(),
        ));
    }
    let plan = compile_pls_plan_v3(model, general).map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.heterogeneity.general_sem_plan_rejected:{error}"
        ))
    })?;
    let CompiledMultiModPlanV1::PlsHeterogeneityV2 {
        profile,
        interactions,
        ..
    } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not PLS heterogeneity V2".into(),
        ));
    };
    let expected_two_way = plan.two_way_interactions().len();
    let expected_three_way = usize::from(plan.three_way_interaction().is_some());
    if profile != &config.profile
        || interactions.len() != expected_two_way + expected_three_way
        || match config.profile {
            CoreHeterogeneityProfileV2::P0Structural => {
                expected_two_way != 0 || expected_three_way != 0
            }
            CoreHeterogeneityProfileV2::P2MultiTwoWay => {
                expected_two_way == 0 || expected_three_way != 0
            }
            CoreHeterogeneityProfileV2::P23AllCurrent => expected_three_way != 1,
        }
    {
        return Err(MultiModRunnerErrorV1::Authority(
            "the compiled General SEM interaction inventory differs from the MultiMod profile"
                .into(),
        ));
    }

    let (mut point_recipe, point_model) =
        project_general_sem_pls_stage_one_recipe_v1(recipe, model).map_err(|error| {
            MultiModRunnerErrorV1::UnsupportedProfile(format!(
                "multimod.runner.heterogeneity.stage_one_projection_rejected:{error}"
            ))
        })?;
    stage_internal_pls_point_recipe_v1(&mut point_recipe);
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let point_artifact = compile_analysis_recipe_v4(
        &point_recipe,
        Some(&point_model),
        target,
        target.capability_cell_for_recipe(&point_recipe),
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::UnsupportedProfile(format!(
            "multimod.runner.heterogeneity.stage_one_compilation_rejected:{error}"
        ))
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan: base_plan } = point_artifact.plan() else {
        return Err(MultiModRunnerErrorV1::Authority(
            "heterogeneity stage-one projection did not emit a PLS plan".into(),
        ));
    };
    if base_plan != plan.base_plan() {
        return Err(MultiModRunnerErrorV1::Authority(
            "heterogeneity stage-one artifact differs from the General SEM base plan".into(),
        ));
    }
    let projected_point_recipe =
        project_pls_plan_to_current_recipe(&point_recipe, &point_model, base_plan).map_err(
            |error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.heterogeneity.stage_one_execution_projection_rejected:{error}"
                ))
            },
        )?;
    let stage_one_execution =
        ValidatedExecutionRecipe::for_dataset(&projected_point_recipe, &dataset.fingerprint.0)
            .map_err(|error| {
                MultiModRunnerErrorV1::UnsupportedProfile(format!(
                    "multimod.runner.heterogeneity.stage_one_execution_rejected:{error}"
                ))
            })?;
    let blocks = ordinary_pls_scoring_blocks_v1(base_plan);
    let source_columns = ordinary_pls_source_columns_v1(dataset, &blocks)?;
    Ok(RawHeterogeneityAuthorityV2 {
        point_recipe,
        point_model,
        point_artifact,
        stage_one_execution,
        plan,
        source_columns,
        blocks,
    })
}

fn complete_heterogeneity_dataset_v2<C>(
    dataset: &Dataset,
    source_columns: &[String],
    should_cancel: &C,
) -> Result<(Dataset, Vec<u64>), MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    let rows = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
    let complete = rows
        .iter()
        .enumerate()
        .filter_map(|(row_index, row)| {
            source_columns
                .iter()
                .all(|column| {
                    row.get(column)
                        .and_then(Option::as_deref)
                        .and_then(|value| value.parse::<f64>().ok())
                        .is_some_and(f64::is_finite)
                })
                .then_some(row_index)
        })
        .collect::<Vec<_>>();
    if complete.len() < 20 {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "heterogeneity V2 requires at least 20 listwise-complete indicator rows".into(),
        ));
    }
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    let complete_dataset =
        resample_dataset_columns_v1(dataset, source_columns, &complete, || should_cancel())
            .map_err(|error| match error {
                EstimationError::Cancelled => MultiModRunnerErrorV1::Cancelled,
                other => MultiModRunnerErrorV1::Kernel(format!(
                    "heterogeneity complete-case projection failed: {other}"
                )),
            })?;
    Ok((
        complete_dataset,
        complete.into_iter().map(|row| row as u64).collect(),
    ))
}

fn raw_heterogeneity_scientific_row_features_v2(
    dataset: &Dataset,
    source_columns: &[String],
) -> Result<Vec<Vec<f64>>, MultiModRunnerErrorV1> {
    let mut columns = source_columns.to_vec();
    columns.sort();
    columns.dedup();
    if columns.is_empty() {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "heterogeneity scientific row features require source columns".into(),
        ));
    }
    qpls_data::preview_page(dataset, 0, dataset.batch.num_rows())
        .into_iter()
        .enumerate()
        .map(|(row_index, row)| {
            columns
                .iter()
                .map(|column| {
                    row.get(column)
                        .and_then(Option::as_deref)
                        .and_then(|value| value.parse::<f64>().ok())
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| {
                            MultiModRunnerErrorV1::PreparedInput(format!(
                                "heterogeneity scientific row {row_index} lacks finite source column {column}"
                            ))
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect()
}

#[derive(Debug, Clone)]
struct RawHeterogeneityMetricV2 {
    fimix_input: StandardizedFimixInputV2,
    pos_start_features: Vec<Vec<f64>>,
    product_sample_standard_deviations: BTreeMap<String, f64>,
}

fn standardize_heterogeneity_vector_v2(
    values: &[f64],
    identity: &str,
) -> Result<(Vec<f64>, f64), MultiModRunnerErrorV1> {
    if values.len() < 3 || values.iter().any(|value| !value.is_finite()) {
        return Err(MultiModRunnerErrorV1::Kernel(format!(
            "heterogeneity score {identity} is nonfinite or has fewer than three observations"
        )));
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64;
    let sample_sd = variance.sqrt();
    if !sample_sd.is_finite() || sample_sd <= f64::EPSILON {
        return Err(MultiModRunnerErrorV1::Kernel(format!(
            "heterogeneity score {identity} is constant"
        )));
    }
    Ok((
        values
            .iter()
            .map(|value| (value - mean) / sample_sd)
            .collect(),
        sample_sd,
    ))
}

fn raw_heterogeneity_metric_v2(
    plan: &CompiledPlsPlanV3,
    stage_one: &PlsResult,
    profile: CoreHeterogeneityProfileV2,
) -> Result<RawHeterogeneityMetricV2, MultiModRunnerErrorV1> {
    let mut standardized_scores = BTreeMap::<String, Vec<f64>>::new();
    for block in plan.base_plan().blocks() {
        let construct_id = block.construct_id();
        let scores = stage_one
            .construct_scores
            .get(construct_id)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Kernel(format!(
                    "stage-one PLS omitted construct score {construct_id}"
                ))
            })?;
        let (standardized, _) = standardize_heterogeneity_vector_v2(scores, construct_id)?;
        standardized_scores.insert(construct_id.to_owned(), standardized);
    }
    let observations = standardized_scores
        .values()
        .next()
        .map(Vec::len)
        .ok_or_else(|| MultiModRunnerErrorV1::Kernel("PLS plan has no score blocks".into()))?;
    if standardized_scores
        .values()
        .any(|values| values.len() != observations)
    {
        return Err(MultiModRunnerErrorV1::Kernel(
            "stage-one construct scores have inconsistent row counts".into(),
        ));
    }

    let mut products = BTreeMap::<String, Vec<f64>>::new();
    let mut product_sample_standard_deviations = BTreeMap::<String, f64>::new();
    for interaction in plan.two_way_interactions() {
        let left = standardized_scores
            .get(interaction.focal_predictor_id())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Kernel(format!(
                    "interaction {} omitted focal score {}",
                    interaction.interaction_id(),
                    interaction.focal_predictor_id()
                ))
            })?;
        let right = standardized_scores
            .get(interaction.moderator_id())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Kernel(format!(
                    "interaction {} omitted moderator score {}",
                    interaction.interaction_id(),
                    interaction.moderator_id()
                ))
            })?;
        let raw = left
            .iter()
            .zip(right)
            .map(|(left, right)| left * right)
            .collect::<Vec<_>>();
        let (standardized, sample_sd) =
            standardize_heterogeneity_vector_v2(&raw, interaction.interaction_id())?;
        products.insert(interaction.interaction_id().into(), standardized);
        product_sample_standard_deviations.insert(interaction.interaction_id().into(), sample_sd);
    }
    if let Some(interaction) = plan.three_way_interaction() {
        let operands = interaction.operand_ids();
        let left = &standardized_scores[&operands[0]];
        let middle = &standardized_scores[&operands[1]];
        let right = &standardized_scores[&operands[2]];
        let raw = left
            .iter()
            .zip(middle)
            .zip(right)
            .map(|((left, middle), right)| left * middle * right)
            .collect::<Vec<_>>();
        let (standardized, sample_sd) =
            standardize_heterogeneity_vector_v2(&raw, interaction.interaction_id())?;
        products.insert(interaction.interaction_id().into(), standardized);
        product_sample_standard_deviations.insert(interaction.interaction_id().into(), sample_sd);
    }

    let outcomes = plan
        .base_plan()
        .paths()
        .iter()
        .map(|path| path.target().to_owned())
        .chain(
            plan.two_way_interactions()
                .iter()
                .map(|interaction| interaction.outcome_id().to_owned()),
        )
        .chain(
            plan.three_way_interaction()
                .into_iter()
                .map(|interaction| interaction.outcome_id().to_owned()),
        )
        .collect::<BTreeSet<_>>();
    let mut equations = Vec::with_capacity(outcomes.len());
    for outcome_id in outcomes {
        let mut predictors = plan
            .base_plan()
            .paths()
            .iter()
            .filter(|path| path.target() == outcome_id)
            .map(|path| {
                (
                    path.parameter_id().to_owned(),
                    standardized_scores[path.source()].clone(),
                )
            })
            .collect::<Vec<_>>();
        predictors.extend(
            plan.two_way_interactions()
                .iter()
                .filter(|interaction| interaction.outcome_id() == outcome_id)
                .map(|interaction| {
                    (
                        interaction.interaction_effect_parameter_id().to_owned(),
                        products[interaction.interaction_id()].clone(),
                    )
                }),
        );
        if let Some(interaction) = plan
            .three_way_interaction()
            .filter(|interaction| interaction.outcome_id() == outcome_id)
        {
            predictors.push((
                interaction.interaction_effect_parameter_id().to_owned(),
                products[interaction.interaction_id()].clone(),
            ));
        }
        predictors.sort_by(|left, right| left.0.cmp(&right.0));
        if predictors.is_empty() {
            continue;
        }
        let predictor_ids = predictors
            .iter()
            .map(|(identity, _)| identity.clone())
            .collect::<Vec<_>>();
        let design = (0..observations)
            .map(|row| predictors.iter().map(|(_, values)| values[row]).collect())
            .collect::<Vec<Vec<f64>>>();
        equations.push(StandardizedStructuralEquationV2 {
            equation_id: format!("equation:{outcome_id}"),
            outcome_id: outcome_id.clone(),
            predictor_ids,
            design,
            outcome: standardized_scores[&outcome_id].clone(),
            include_intercept: true,
        });
    }
    if equations.is_empty() {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.heterogeneity.structural_equation_required".into(),
        ));
    }
    let score_ids = standardized_scores.keys().cloned().collect::<Vec<_>>();
    let pos_start_features = (0..observations)
        .map(|row| {
            score_ids
                .iter()
                .map(|identity| standardized_scores[identity][row])
                .collect()
        })
        .collect::<Vec<Vec<f64>>>();
    let source_sha256 =
        sha256_serialized(&(plan.deterministic_sha256(), &standardized_scores, &products));
    Ok(RawHeterogeneityMetricV2 {
        fimix_input: StandardizedFimixInputV2 {
            interaction_profile: estimation_heterogeneity_profile(profile),
            metric: PooledStandardizedMetricReceiptV2 {
                metric_id: format!(
                    "qpls.heterogeneity.pooled-standardized-metric.v2:{source_sha256}"
                ),
                source_sha256,
                observation_count: observations,
                scores_standardized_once_on_pooled_rows: true,
                products_standardized_once_on_pooled_rows: profile
                    != CoreHeterogeneityProfileV2::P0Structural,
            },
            equations,
        },
        pos_start_features,
        product_sample_standard_deviations,
    })
}

fn raw_heterogeneity_stage_one_fit_v2<C>(
    dataset: &Dataset,
    authority: &RawHeterogeneityAuthorityV2,
    should_cancel: &C,
) -> Result<PlsResult, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    let result = run_compiled_pls_recipe_v4(
        dataset,
        &authority.point_recipe,
        &authority.point_model,
        &authority.point_artifact,
        None,
        || should_cancel(),
        |_| {},
    )
    .map_err(|error| match error {
        crate::RecipeV4PlsExecutionError::Cancelled => MultiModRunnerErrorV1::Cancelled,
        other => MultiModRunnerErrorV1::Kernel(format!(
            "heterogeneity stage-one PLS refit failed: {other}"
        )),
    })?
    .estimation()
    .clone();
    if !result.converged
        || result.used_observations != dataset.batch.num_rows()
        || result.omitted_observations != 0
    {
        return Err(MultiModRunnerErrorV1::Kernel(
            "heterogeneity stage-one PLS did not converge on exactly the requested complete rows"
                .into(),
        ));
    }
    Ok(result)
}

/// Executes the already-authorized stage-one PLS recipe without repeating the
/// immutable Recipe V4 artifact validation and projection for every POS
/// candidate segment. `projected_heterogeneity_authority_v2` constructs this
/// opaque execution capability once, and the sampled datasets retain its
/// bound id/fingerprint. The numerical estimator and its convergence contract
/// are therefore identical to `raw_heterogeneity_stage_one_fit_v2`.
fn raw_heterogeneity_stage_one_refit_v2<C>(
    dataset: &Dataset,
    authority: &RawHeterogeneityAuthorityV2,
    should_cancel: &C,
) -> Result<PlsResult, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    if dataset.id.to_string() != authority.plan.base_plan().dataset_id()
        || dataset.fingerprint.0 != authority.point_recipe.dataset_fingerprint
    {
        return Err(MultiModRunnerErrorV1::Authority(
            "heterogeneity segment refit dataset differs from its prepared stage-one authority"
                .into(),
        ));
    }
    let mut report_progress = |_: qpls_estimation::EstimationProgress| !should_cancel();
    let result = estimate_pls_validated_with_control(
        dataset,
        &authority.stage_one_execution,
        &mut report_progress,
    )
    .map_err(|error| match error {
        EstimationError::Cancelled => MultiModRunnerErrorV1::Cancelled,
        other => MultiModRunnerErrorV1::Kernel(format!(
            "heterogeneity prepared stage-one PLS refit failed: {other}"
        )),
    })?;
    if !result.converged
        || result.method_version != qpls_estimation::PLS_METHOD_VERSION
        || result.used_observations != dataset.batch.num_rows()
        || result.omitted_observations != 0
        || result.wpls.is_some()
        || result.plsc.is_some()
        || result.score_execution.is_some()
        || result.nonlinear_effects.is_some()
        || result.posthoc_minimum_sample_size.is_some()
        || result.point_estimate_attribution.as_ref()
            != Some(&PlsPointEstimateAttributionV1::for_preprocessing(
                authority.point_recipe.settings.preprocessing.clone(),
            ))
    {
        return Err(MultiModRunnerErrorV1::Kernel(
            "heterogeneity prepared stage-one PLS refit violated its method, convergence, row, or scale contract"
                .into(),
        ));
    }
    Ok(result)
}

#[derive(Debug, Clone)]
enum RawJointStructuralPointV2 {
    P0,
    P2(GeneralSemPlsMultipleInteractionPointResultV1),
    P23(GeneralSemPlsThreeWayPointResultV1),
}

fn raw_joint_structural_point_v2<C>(
    authority: &RawHeterogeneityAuthorityV2,
    stage_one: &PlsResult,
    profile: CoreHeterogeneityProfileV2,
    should_cancel: &C,
) -> Result<RawJointStructuralPointV2, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    match profile {
        CoreHeterogeneityProfileV2::P0Structural => Ok(RawJointStructuralPointV2::P0),
        CoreHeterogeneityProfileV2::P2MultiTwoWay => {
            let point = estimate_general_sem_pls_multiple_two_way_interactions_v1_with_control(
                &authority.plan,
                &stage_one.construct_scores,
                || !should_cancel(),
            )
            .map_err(|error| match error {
                GeneralSemPlsInteractionPointErrorV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
                other => MultiModRunnerErrorV1::Kernel(format!(
                    "destination-local two-way joint refit failed: {other}"
                )),
            })?;
            point
                .ensure_valid_against_plan_v1(&authority.plan)
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            Ok(RawJointStructuralPointV2::P2(point))
        }
        CoreHeterogeneityProfileV2::P23AllCurrent => {
            let point = estimate_general_sem_pls_three_way_moderation_v1_with_control(
                &authority.plan,
                &stage_one.construct_scores,
                || !should_cancel(),
            )
            .map_err(|error| match error {
                GeneralSemPlsThreeWayPointErrorV1::Cancelled => MultiModRunnerErrorV1::Cancelled,
                other => MultiModRunnerErrorV1::Kernel(format!(
                    "destination-local three-way joint refit failed: {other}"
                )),
            })?;
            point
                .ensure_valid_against_plan_v1(&authority.plan)
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            Ok(RawJointStructuralPointV2::P23(point))
        }
    }
}

fn p0_path_coefficient_v2(
    plan: &CompiledPlsPlanV3,
    result: &PlsResult,
    parameter_id: &str,
) -> Option<f64> {
    let path = plan
        .base_plan()
        .paths()
        .iter()
        .find(|path| path.parameter_id() == parameter_id)?;
    result
        .paths
        .iter()
        .find(|estimate| estimate.source == path.source() && estimate.target == path.target())
        .map(|estimate| estimate.coefficient)
        .or_else(|| {
            (path.role() == StructuralRelationRoleV4::Control).then(|| {
                result
                    .control_estimates
                    .iter()
                    .find(|estimate| {
                        estimate.source == path.source() && estimate.target == path.target()
                    })
                    .map(|estimate| estimate.coefficient)
            })?
        })
}

fn raw_joint_standardized_coefficients_v2(
    authority: &RawHeterogeneityAuthorityV2,
    stage_one: &PlsResult,
    joint: &RawJointStructuralPointV2,
) -> Result<BTreeMap<String, f64>, MultiModRunnerErrorV1> {
    let mut coefficients = BTreeMap::new();
    match joint {
        RawJointStructuralPointV2::P0 => {
            for path in authority.plan.base_plan().paths() {
                let value = p0_path_coefficient_v2(&authority.plan, stage_one, path.parameter_id())
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::Kernel(format!(
                            "stage-one PLS omitted path parameter {}",
                            path.parameter_id()
                        ))
                    })?;
                coefficients.insert(path.parameter_id().into(), value);
            }
        }
        RawJointStructuralPointV2::P2(point) => {
            for row in point.structural_coefficients() {
                let path = authority
                    .plan
                    .base_plan()
                    .paths()
                    .iter()
                    .find(|path| path.relation_id() == row.relation_id())
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::Kernel(format!(
                            "joint two-way result returned unknown relation {}",
                            row.relation_id()
                        ))
                    })?;
                coefficients.insert(path.parameter_id().into(), row.estimate());
            }
            for row in point.interaction_coefficients() {
                coefficients.insert(
                    row.interaction_effect_parameter_id().into(),
                    row.standardized_product_estimate(),
                );
            }
        }
        RawJointStructuralPointV2::P23(point) => {
            for row in &point.structural_coefficients {
                let path = authority
                    .plan
                    .base_plan()
                    .paths()
                    .iter()
                    .find(|path| path.relation_id() == row.relation_id())
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::Kernel(format!(
                            "joint three-way result returned unknown relation {}",
                            row.relation_id()
                        ))
                    })?;
                coefficients.insert(path.parameter_id().into(), row.estimate());
            }
            for row in &point.lower_order_interaction_coefficients {
                coefficients.insert(
                    row.interaction_effect_parameter_id().into(),
                    row.standardized_product_estimate(),
                );
            }
            coefficients.insert(
                point
                    .three_way_coefficient
                    .interaction_effect_parameter_id
                    .clone(),
                point.three_way_coefficient.standardized_product_estimate,
            );
        }
    }
    if coefficients.values().any(|value| !value.is_finite()) {
        return Err(MultiModRunnerErrorV1::Kernel(
            "joint structural refit produced a nonfinite coefficient".into(),
        ));
    }
    Ok(coefficients)
}

fn raw_joint_r_squared_v2(
    metric: &RawHeterogeneityMetricV2,
    coefficients: &BTreeMap<String, f64>,
    source_row_indices: &[usize],
    retain_outcome_audits: bool,
) -> Result<(Vec<PosOutcomeR2V2>, Vec<PosOutcomeFitAuditV2>), MultiModRunnerErrorV1> {
    let mut r_squared = Vec::with_capacity(metric.fimix_input.equations.len());
    let mut audits = if retain_outcome_audits {
        Vec::with_capacity(metric.fimix_input.equations.len())
    } else {
        Vec::new()
    };
    for equation in &metric.fimix_input.equations {
        let predictions = equation
            .design
            .iter()
            .map(|row| {
                equation
                    .predictor_ids
                    .iter()
                    .zip(row)
                    .map(|(identity, value)| {
                        coefficients
                            .get(identity)
                            .copied()
                            .ok_or_else(|| {
                                MultiModRunnerErrorV1::Kernel(format!(
                                    "joint result omitted predictor {identity}"
                                ))
                            })
                            .map(|coefficient| coefficient * value)
                    })
                    .sum::<Result<f64, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let residual_sum_of_squares = equation
            .outcome
            .iter()
            .zip(&predictions)
            .map(|(actual, predicted)| (actual - predicted).powi(2))
            .sum::<f64>();
        let observed_mean = equation.outcome.iter().sum::<f64>() / equation.outcome.len() as f64;
        let total_sum_of_squares = equation
            .outcome
            .iter()
            .map(|value| (value - observed_mean).powi(2))
            .sum::<f64>();
        if !residual_sum_of_squares.is_finite()
            || !total_sum_of_squares.is_finite()
            || total_sum_of_squares <= f64::EPSILON
            || observed_mean.abs() > POS_STANDARDIZED_OUTCOME_MEAN_TOLERANCE_V2
        {
            return Err(MultiModRunnerErrorV1::Kernel(format!(
                "joint equation {} lacks centered standardized outcome scores for R-squared (mean {observed_mean})",
                equation.equation_id,
            )));
        }
        let value = 1.0 - residual_sum_of_squares / total_sum_of_squares;
        if !(-1.0e-10..=1.0 + 1.0e-10).contains(&value) {
            return Err(MultiModRunnerErrorV1::Kernel(format!(
                "joint equation {} produced out-of-range R-squared {value}",
                equation.equation_id
            )));
        }
        if source_row_indices.len() != equation.outcome.len() {
            return Err(MultiModRunnerErrorV1::Kernel(format!(
                "joint equation {} source-row audit has the wrong length",
                equation.equation_id
            )));
        }
        r_squared.push(PosOutcomeR2V2 {
            outcome_id: equation.outcome_id.clone(),
            r_squared: value.clamp(0.0, 1.0),
        });
        if retain_outcome_audits {
            audits.push(PosOutcomeFitAuditV2 {
                outcome_id: equation.outcome_id.clone(),
                source_row_indices: source_row_indices.to_vec(),
                observed_scores: equation.outcome.clone(),
                fitted_scores: predictions,
                observed_mean,
                centered_total_sum_of_squares: total_sum_of_squares,
            });
        }
    }
    Ok((r_squared, audits))
}

fn scientific_parameter_signature_v2(
    authority: &RawHeterogeneityAuthorityV2,
    stage_one: &PlsResult,
    joint: &RawJointStructuralPointV2,
) -> Result<(Vec<String>, Vec<f64>), MultiModRunnerErrorV1> {
    let standardized = raw_joint_standardized_coefficients_v2(authority, stage_one, joint)?;
    let mut scientific = BTreeMap::<String, f64>::new();
    for path in authority.plan.base_plan().paths() {
        scientific.insert(
            path.parameter_id().into(),
            standardized[path.parameter_id()],
        );
    }
    match joint {
        RawJointStructuralPointV2::P0 => {}
        RawJointStructuralPointV2::P2(point) => {
            for row in point.interaction_coefficients() {
                scientific.insert(
                    row.interaction_effect_parameter_id().into(),
                    row.raw_product_estimate(),
                );
            }
        }
        RawJointStructuralPointV2::P23(point) => {
            for row in &point.lower_order_interaction_coefficients {
                scientific.insert(
                    row.interaction_effect_parameter_id().into(),
                    row.raw_product_estimate(),
                );
            }
            scientific.insert(
                point
                    .three_way_coefficient
                    .interaction_effect_parameter_id
                    .clone(),
                point.three_way_coefficient.scientific_rescaled_delta,
            );
        }
    }

    let probe_slug = |probe: f64| match probe as i32 {
        -1 => "minus_1",
        0 => "zero",
        1 => "plus_1",
        _ => unreachable!("fixed POS probes are -1, 0, +1"),
    };
    for interaction in authority.plan.two_way_interactions() {
        let focal_path = authority
            .plan
            .base_plan()
            .paths()
            .iter()
            .find(|path| path.relation_id() == interaction.focal_relation_id())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(format!(
                    "interaction {} omitted its focal path",
                    interaction.interaction_id()
                ))
            })?;
        let focal = scientific[focal_path.parameter_id()];
        let gamma = scientific[interaction.interaction_effect_parameter_id()];
        for probe in [-1.0_f64, 0.0, 1.0] {
            scientific.insert(
                format!(
                    "simple_slope:{}:{}:{}",
                    interaction.interaction_id(),
                    interaction.moderator_id(),
                    probe_slug(probe)
                ),
                focal + gamma * probe,
            );
        }
    }
    if let Some(interaction) = authority.plan.three_way_interaction() {
        let focal_path = authority
            .plan
            .base_plan()
            .paths()
            .iter()
            .find(|path| path.relation_id() == interaction.focal_relation_id())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(
                    "three-way interaction omitted its focal path".into(),
                )
            })?;
        let focal = scientific[focal_path.parameter_id()];
        let pair_coefficient = |left: &str, right: &str| {
            authority
                .plan
                .two_way_interactions()
                .iter()
                .find(|candidate| {
                    (candidate.focal_predictor_id() == left && candidate.moderator_id() == right)
                        || (candidate.focal_predictor_id() == right
                            && candidate.moderator_id() == left)
                })
                .and_then(|candidate| {
                    scientific
                        .get(candidate.interaction_effect_parameter_id())
                        .copied()
                })
        };
        let xw = pair_coefficient(
            interaction.focal_predictor_id(),
            interaction.first_moderator_id(),
        )
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "three-way strong hierarchy omitted the focal-by-first-moderator gamma".into(),
            )
        })?;
        let xz = pair_coefficient(
            interaction.focal_predictor_id(),
            interaction.second_moderator_id(),
        )
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "three-way strong hierarchy omitted the focal-by-second-moderator gamma".into(),
            )
        })?;
        let delta = scientific[interaction.interaction_effect_parameter_id()];
        for first in [-1.0_f64, 0.0, 1.0] {
            for second in [-1.0_f64, 0.0, 1.0] {
                scientific.insert(
                    format!(
                        "three_way_simple_slope:{}:{}:{}:{}:{}",
                        interaction.interaction_id(),
                        interaction.first_moderator_id(),
                        probe_slug(first),
                        interaction.second_moderator_id(),
                        probe_slug(second)
                    ),
                    focal + xw * first + xz * second + delta * first * second,
                );
            }
        }
    }
    if scientific.values().any(|value| !value.is_finite()) {
        return Err(MultiModRunnerErrorV1::Kernel(
            "scientific POS parameter signature contains a nonfinite value".into(),
        ));
    }
    Ok(scientific.into_iter().unzip())
}

/// Qualification executes at most four producer shards concurrently. Giving
/// each POS producer approximately one quarter of the machine keeps the exact
/// candidate-refit batches parallel without oversubscribing the shared host.
/// `IndexedParallelIterator::collect` retains request order, so this affects
/// wall time only and cannot change the deterministic steepest-move decision.
static POS_REFIT_POOL_V2: LazyLock<rayon::ThreadPool> = LazyLock::new(|| {
    let logical = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let workers = (logical.saturating_add(3) / 4).clamp(1, 4);
    rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .thread_name(|index| format!("qpls-pos-refit-{index}"))
        .build()
        .expect("bounded POS refit thread pool must initialize")
});

struct RawPlsPosRefitterV2<'a, C> {
    dataset: &'a Dataset,
    authority: &'a RawHeterogeneityAuthorityV2,
    profile: CoreHeterogeneityProfileV2,
    pooled_fit: &'a PlsResult,
    raw_scores: &'a OrdinaryPlsRawScoreCacheV1,
    orientation_rows: &'a [u64],
    retain_outcome_audits: bool,
    should_cancel: &'a C,
}

impl<C> RawPlsPosRefitterV2<'_, C>
where
    C: Fn() -> bool + Sync,
{
    fn stage_one_for_source_rows_v2(
        &self,
        source_rows: &[u64],
    ) -> Result<PlsResult, RefitFailureV1> {
        if (self.should_cancel)() {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::Cancelled,
                "cancelled before heterogeneity common-metric refit",
            ));
        }
        let mut canonical_rows = source_rows.to_vec();
        canonical_rows.sort_unstable();
        if canonical_rows.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(RefitFailureV1::new(
                RefitFailureCodeV1::ParameterContractMismatch,
                "heterogeneity common-metric membership duplicates a source row",
            ));
        }
        let indices = checked_source_rows_v1(self.dataset, &canonical_rows)?;
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &indices,
            || (self.should_cancel)(),
        )
        .map_err(|error| match error {
            EstimationError::Cancelled => {
                RefitFailureV1::new(RefitFailureCodeV1::Cancelled, error.to_string())
            }
            other => RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, other.to_string()),
        })?;
        let mut fit =
            raw_heterogeneity_stage_one_refit_v2(&sampled, self.authority, self.should_cancel)
                .map_err(|error| match error {
                    MultiModRunnerErrorV1::Cancelled => RefitFailureV1::new(
                        RefitFailureCodeV1::Cancelled,
                        "heterogeneity common-metric refit was cancelled",
                    ),
                    other => {
                        RefitFailureV1::new(RefitFailureCodeV1::EngineFailure, other.to_string())
                    }
                })?;
        align_pls_fit_to_reference_v1(
            &self.authority.blocks,
            self.raw_scores,
            self.pooled_fit,
            self.orientation_rows,
            &mut fit,
        )?;
        Ok(fit)
    }
}

fn raw_fimix_scientific_targets_v2(
    authority: &RawHeterogeneityAuthorityV2,
    metric: &RawHeterogeneityMetricV2,
) -> Result<Vec<PreparedFimixScientificTargetV2>, MultiModRunnerErrorV1> {
    let mut targets = Vec::new();
    for equation in &metric.fimix_input.equations {
        for coefficient_id in &equation.predictor_ids {
            let (target_kind, scale_divisor) = if authority
                .plan
                .base_plan()
                .paths()
                .iter()
                .any(|path| path.parameter_id() == coefficient_id)
            {
                ("class_specific_structural_path", 1.0)
            } else if let Some(interaction) = authority
                .plan
                .two_way_interactions()
                .iter()
                .find(|interaction| interaction.interaction_effect_parameter_id() == coefficient_id)
            {
                (
                    "class_specific_scientific_gamma",
                    *metric
                        .product_sample_standard_deviations
                        .get(interaction.interaction_id())
                        .ok_or_else(|| {
                            MultiModRunnerErrorV1::Kernel(format!(
                                "pooled metric omitted product scale for {}",
                                interaction.interaction_id()
                            ))
                        })?,
                )
            } else if let Some(interaction) =
                authority
                    .plan
                    .three_way_interaction()
                    .filter(|interaction| {
                        interaction.interaction_effect_parameter_id() == coefficient_id
                    })
            {
                (
                    "class_specific_scientific_delta",
                    *metric
                        .product_sample_standard_deviations
                        .get(interaction.interaction_id())
                        .ok_or_else(|| {
                            MultiModRunnerErrorV1::Kernel(format!(
                                "pooled metric omitted product scale for {}",
                                interaction.interaction_id()
                            ))
                        })?,
                )
            } else {
                return Err(MultiModRunnerErrorV1::Authority(format!(
                    "FIMIX equation {} has unknown coefficient {coefficient_id}",
                    equation.equation_id
                )));
            };
            targets.push(PreparedFimixScientificTargetV2 {
                equation_id: equation.equation_id.clone(),
                target_id: coefficient_id.clone(),
                target_kind: target_kind.into(),
                primary_coefficient_target: true,
                terms: vec![PreparedFimixScientificTermV2 {
                    coefficient_id: coefficient_id.clone(),
                    multiplier: 1.0,
                    scale_divisor,
                }],
            });
        }
    }

    let probe_slug = |probe: f64| match probe as i32 {
        -1 => "minus_1",
        0 => "zero",
        1 => "plus_1",
        _ => unreachable!("fixed probes are -1, 0, +1"),
    };
    for interaction in authority.plan.two_way_interactions() {
        let equation_id = format!("equation:{}", interaction.outcome_id());
        let focal = authority
            .plan
            .base_plan()
            .paths()
            .iter()
            .find(|path| path.relation_id() == interaction.focal_relation_id())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(format!(
                    "interaction {} omitted its focal path",
                    interaction.interaction_id()
                ))
            })?;
        let divisor = metric.product_sample_standard_deviations[interaction.interaction_id()];
        for probe in [-1.0_f64, 0.0, 1.0] {
            targets.push(PreparedFimixScientificTargetV2 {
                equation_id: equation_id.clone(),
                target_id: format!(
                    "simple_slope:{}:{}:{}",
                    interaction.interaction_id(),
                    interaction.moderator_id(),
                    probe_slug(probe)
                ),
                target_kind: "class_specific_fixed_simple_slope".into(),
                primary_coefficient_target: false,
                terms: vec![
                    PreparedFimixScientificTermV2 {
                        coefficient_id: focal.parameter_id().into(),
                        multiplier: 1.0,
                        scale_divisor: 1.0,
                    },
                    PreparedFimixScientificTermV2 {
                        coefficient_id: interaction.interaction_effect_parameter_id().into(),
                        multiplier: probe,
                        scale_divisor: divisor,
                    },
                ],
            });
        }
    }
    if let Some(interaction) = authority.plan.three_way_interaction() {
        let equation_id = format!("equation:{}", interaction.outcome_id());
        let focal = authority
            .plan
            .base_plan()
            .paths()
            .iter()
            .find(|path| path.relation_id() == interaction.focal_relation_id())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::Authority(
                    "three-way interaction omitted its focal path".into(),
                )
            })?;
        let pair = |moderator_id: &str| {
            authority
                .plan
                .two_way_interactions()
                .iter()
                .find(|candidate| {
                    (candidate.focal_predictor_id() == interaction.focal_predictor_id()
                        && candidate.moderator_id() == moderator_id)
                        || (candidate.moderator_id() == interaction.focal_predictor_id()
                            && candidate.focal_predictor_id() == moderator_id)
                })
        };
        let first_pair = pair(interaction.first_moderator_id()).ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "three-way strong hierarchy omitted the focal-by-first-moderator term".into(),
            )
        })?;
        let second_pair = pair(interaction.second_moderator_id()).ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "three-way strong hierarchy omitted the focal-by-second-moderator term".into(),
            )
        })?;
        for first in [-1.0_f64, 0.0, 1.0] {
            for second in [-1.0_f64, 0.0, 1.0] {
                targets.push(PreparedFimixScientificTargetV2 {
                    equation_id: equation_id.clone(),
                    target_id: format!(
                        "three_way_simple_slope:{}:{}:{}:{}:{}",
                        interaction.interaction_id(),
                        interaction.first_moderator_id(),
                        probe_slug(first),
                        interaction.second_moderator_id(),
                        probe_slug(second)
                    ),
                    target_kind: "class_specific_fixed_three_way_simple_slope".into(),
                    primary_coefficient_target: false,
                    terms: vec![
                        PreparedFimixScientificTermV2 {
                            coefficient_id: focal.parameter_id().into(),
                            multiplier: 1.0,
                            scale_divisor: 1.0,
                        },
                        PreparedFimixScientificTermV2 {
                            coefficient_id: first_pair.interaction_effect_parameter_id().into(),
                            multiplier: first,
                            scale_divisor: metric.product_sample_standard_deviations
                                [first_pair.interaction_id()],
                        },
                        PreparedFimixScientificTermV2 {
                            coefficient_id: second_pair.interaction_effect_parameter_id().into(),
                            multiplier: second,
                            scale_divisor: metric.product_sample_standard_deviations
                                [second_pair.interaction_id()],
                        },
                        PreparedFimixScientificTermV2 {
                            coefficient_id: interaction.interaction_effect_parameter_id().into(),
                            multiplier: first * second,
                            scale_divisor: metric.product_sample_standard_deviations
                                [interaction.interaction_id()],
                        },
                    ],
                });
            }
        }
    }
    targets.sort_by(|left, right| {
        left.equation_id
            .cmp(&right.equation_id)
            .then(left.target_id.cmp(&right.target_id))
    });
    Ok(targets)
}

impl<C> PlsPosFullRefitterV2 for RawPlsPosRefitterV2<'_, C>
where
    C: Fn() -> bool + Sync,
{
    fn refit_segment(
        &mut self,
        _segment_index: usize,
        row_indices: &[usize],
        scoring: PosScoringContractV2,
    ) -> Result<PosSegmentFullFitV2, String> {
        if (self.should_cancel)() {
            return Err("multimod.runner.cancelled".into());
        }
        let scoring_matches = matches!(
            (self.profile, scoring),
            (
                CoreHeterogeneityProfileV2::P0Structural,
                PosScoringContractV2::PublishedP0FullSegmentPls
            ) | (
                CoreHeterogeneityProfileV2::P2MultiTwoWay,
                PosScoringContractV2::DestinationScoredInteractions {
                    profile: EstimationHeterogeneityProfileV2::P2MultiTwoWay
                }
            ) | (
                CoreHeterogeneityProfileV2::P23AllCurrent,
                PosScoringContractV2::DestinationScoredInteractions {
                    profile: EstimationHeterogeneityProfileV2::P23AllCurrent
                }
            )
        );
        if !scoring_matches {
            return Err("multimod.runner.heterogeneity.pos_scoring_contract_mismatch".into());
        }
        if row_indices.len() < 20
            || row_indices
                .iter()
                .any(|row| *row >= self.dataset.batch.num_rows())
        {
            return Err("multimod.runner.heterogeneity.pos_segment_rows_invalid".into());
        }
        let mut canonical_rows = row_indices.to_vec();
        canonical_rows.sort_unstable();
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &canonical_rows,
            || (self.should_cancel)(),
        )
        .map_err(|error| format!("multimod.runner.heterogeneity.pos_resample:{error}"))?;
        let mut stage_one =
            raw_heterogeneity_stage_one_refit_v2(&sampled, self.authority, self.should_cancel)
                .map_err(|error| error.to_string())?;
        align_pls_fit_to_reference_v1(
            &self.authority.blocks,
            self.raw_scores,
            self.pooled_fit,
            self.orientation_rows,
            &mut stage_one,
        )
        .map_err(|error| {
            let code = match error.code {
                RefitFailureCodeV1::Cancelled => "cancelled",
                RefitFailureCodeV1::UnsupportedProfile => "unsupported_profile",
                RefitFailureCodeV1::InsufficientRows => "insufficient_rows",
                RefitFailureCodeV1::SingularModel => "singular_model",
                RefitFailureCodeV1::Nonconvergence => "nonconvergence",
                RefitFailureCodeV1::NonFiniteEstimate => "nonfinite_estimate",
                RefitFailureCodeV1::OrientationUndefined => "orientation_undefined",
                RefitFailureCodeV1::ParameterContractMismatch => "parameter_contract",
                RefitFailureCodeV1::EngineFailure => "engine_failure",
            };
            format!(
                "multimod.runner.heterogeneity.pos_orientation.{code}:{}",
                error.detail
            )
        })?;
        let metric = raw_heterogeneity_metric_v2(&self.authority.plan, &stage_one, self.profile)
            .map_err(|error| error.to_string())?;
        let joint = raw_joint_structural_point_v2(
            self.authority,
            &stage_one,
            self.profile,
            self.should_cancel,
        )
        .map_err(|error| error.to_string())?;
        let coefficients =
            raw_joint_standardized_coefficients_v2(self.authority, &stage_one, &joint)
                .map_err(|error| error.to_string())?;
        let (r_squared, outcome_fit_audits) = raw_joint_r_squared_v2(
            &metric,
            &coefficients,
            &canonical_rows,
            self.retain_outcome_audits,
        )
        .map_err(|error| error.to_string())?;
        if self.profile == CoreHeterogeneityProfileV2::P0Structural {
            for outcome in &r_squared {
                let expected = stage_one
                    .r_squared
                    .get(&outcome.outcome_id)
                    .ok_or_else(|| {
                        format!("stage-one PLS omitted R-squared for {}", outcome.outcome_id)
                    })?;
                if (expected - outcome.r_squared).abs() > 1.0e-8 {
                    return Err(format!(
                        "P0 joint R-squared {} differs from the full PLS refit {} for {}",
                        outcome.r_squared, expected, outcome.outcome_id
                    ));
                }
            }
        }
        let (_, parameter_signature) =
            scientific_parameter_signature_v2(self.authority, &stage_one, &joint)
                .map_err(|error| error.to_string())?;
        let interaction_profile = self.profile != CoreHeterogeneityProfileV2::P0Structural;
        Ok(PosSegmentFullFitV2 {
            r_squared,
            outcome_fit_audits,
            parameter_signature,
            receipt: PosFullRefitReceiptV2 {
                method_version: if interaction_profile {
                    PLS_POS_DESTINATION_SCORED_INTERACTIONS_METHOD_VERSION_V2.into()
                } else {
                    PLS_POS_PUBLISHED_METHOD_VERSION_V2.into()
                },
                full_segment_pls_refit: true,
                measurement_scores_reestimated: true,
                score_orientation_reapplied: true,
                interaction_stage_one_refit: interaction_profile,
                interaction_operands_restandardized_within_destination: interaction_profile,
                interaction_products_rebuilt_within_destination: interaction_profile,
                joint_structural_equations_refit: true,
            },
        })
    }

    fn refit_segments_batch(
        &mut self,
        requests: &[PosSegmentRefitRequestV2],
    ) -> Vec<Result<PosSegmentFullFitV2, String>> {
        let dataset = self.dataset;
        let authority = self.authority;
        let profile = self.profile;
        let pooled_fit = self.pooled_fit;
        let raw_scores = self.raw_scores;
        let orientation_rows = self.orientation_rows;
        let should_cancel = self.should_cancel;
        POS_REFIT_POOL_V2.install(|| {
            requests
                .par_iter()
                .map(|request| {
                    let mut isolated = RawPlsPosRefitterV2 {
                        dataset,
                        authority,
                        profile,
                        pooled_fit,
                        raw_scores,
                        orientation_rows,
                        retain_outcome_audits: false,
                        should_cancel,
                    };
                    isolated.refit_segment(
                        request.segment_index,
                        &request.row_indices,
                        request.scoring,
                    )
                })
                .collect()
        })
    }
}

impl<C> MicomRefitterV1 for RawPlsPosRefitterV2<'_, C>
where
    C: Fn() -> bool + Sync,
{
    fn fit_micom(&mut self, request: &MicomFitRequestV1) -> Result<MicomFitV1, RefitFailureV1> {
        let mut scores = Vec::with_capacity(
            request
                .training_groups
                .len()
                .saturating_mul(self.authority.blocks.len()),
        );
        for training in &request.training_groups {
            let fit = self.stage_one_for_source_rows_v2(&training.source_rows)?;
            for block in &self.authority.blocks {
                scores.push(MicomGroupConstructScoresV1 {
                    group: training.group,
                    construct_id: block.construct_id.clone(),
                    pooled_scores: self.raw_scores.composite_scores(
                        block,
                        &fit,
                        &request.scoring_rows,
                    )?,
                });
            }
        }
        let pooled_reference_scores = if request.kind == MicomFitKindV1::Observed {
            let mut pooled_rows = request
                .training_groups
                .iter()
                .flat_map(|group| group.source_rows.iter().copied())
                .collect::<Vec<_>>();
            pooled_rows.sort_unstable();
            if pooled_rows.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(RefitFailureV1::new(
                    RefitFailureCodeV1::ParameterContractMismatch,
                    "pair-pooled POS MICOM membership duplicates a source row",
                ));
            }
            let fit = self.stage_one_for_source_rows_v2(&pooled_rows)?;
            self.authority
                .blocks
                .iter()
                .map(|block| {
                    Ok(MicomPooledConstructScoresV1 {
                        construct_id: block.construct_id.clone(),
                        pooled_scores: self.raw_scores.composite_scores(
                            block,
                            &fit,
                            &request.scoring_rows,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, RefitFailureV1>>()?
        } else {
            Vec::new()
        };
        Ok(MicomFitV1 {
            scores,
            pooled_reference_scores,
        })
    }
}

fn prepare_raw_pos_common_metric_v1<C>(
    assignments: &[usize],
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    prepared: &PreparedHeterogeneityExecutionV2,
    refitter: &mut RawPlsPosRefitterV2<'_, C>,
) -> Result<PreparedPosCommonMetricEvidenceV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
{
    let gate_config = config
        .pos_common_metric
        .as_ref()
        .filter(|gate| gate.request_segment_contrasts)
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "POS common-metric preparation requires an explicit contrast request".into(),
            )
        })?;
    let lock = match &config.phase {
        qpls_core::HeterogeneityPhaseV2::Inference { lock } => lock,
        qpls_core::HeterogeneityPhaseV2::Discovery { .. } => {
            return Err(MultiModRunnerErrorV1::Authority(
                "POS common-metric contrasts require a locked segmentation".into(),
            ));
        }
    };
    let segments = lock.selected_k as usize;
    if lock.selected_algorithm == CoreHeterogeneityAlgorithmV2::FimixPlsV2
        || assignments.len() != prepared.fimix_input.metric.observation_count
        || assignments.iter().any(|segment| *segment >= segments)
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "POS common-metric assignments differ from the locked POS segmentation".into(),
        ));
    }
    let rows_by_group = (0..segments)
        .map(|segment| {
            let group = GroupIndexV1::new(segment).expect("K is bounded by five");
            let rows = assignments
                .iter()
                .enumerate()
                .filter_map(|(row, assignment)| (*assignment == segment).then_some(row as u64))
                .collect::<Vec<_>>();
            (group, rows)
        })
        .collect::<BTreeMap<_, _>>();
    if rows_by_group.values().any(|rows| rows.len() < 20) {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "POS common-metric MICOM requires at least 20 rows in every locked segment".into(),
        ));
    }
    let selected_rows = assignments
        .iter()
        .enumerate()
        .map(|(row, segment)| SelectedGroupRowV1 {
            source_row: row as u64,
            stable_row_token: row as u64,
            group: GroupIndexV1::new(*segment).expect("validated segment index"),
        })
        .collect::<Vec<_>>();
    let construct_ids = refitter
        .authority
        .blocks
        .iter()
        .map(|block| block.construct_id.clone())
        .collect::<Vec<_>>();
    let receipt = MicomConfiguralReceiptV1 {
        identical_indicators_and_coding: gate_config
            .configural_checklist
            .identical_indicators_and_coding,
        identical_data_treatment: gate_config.configural_checklist.identical_data_treatment,
        identical_algorithm_settings: gate_config
            .configural_checklist
            .identical_algorithm_settings,
        identical_model_specification: gate_config
            .configural_checklist
            .identical_model_specification,
        deterministic_orientation_reviewed: gate_config
            .configural_checklist
            .deterministic_sign_orientation_reviewed,
        analyst_review_confirmed: gate_config.configural_checklist.analyst_review_confirmed,
    };
    let mut micom_pairs = Vec::with_capacity(segments * (segments - 1) / 2);
    let should_cancel = refitter.should_cancel;
    for left in 0..segments {
        for right in left + 1..segments {
            let pair = OrderedGroupPairV1::new(
                GroupIndexV1::new(left).expect("bounded segment"),
                GroupIndexV1::new(right).expect("bounded segment"),
            )
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            let partition_plan = build_pairwise_partition_plan_from_rows_v1(
                &selected_rows,
                pair,
                gate_config.permutation_samples as usize,
                config.seed,
            )
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            let result = run_pairwise_micom_with_partition_plan_v1(
                refitter,
                pair,
                &rows_by_group,
                &selected_rows,
                &construct_ids,
                receipt.clone(),
                MicomPermutationConfigV1 {
                    requested: gate_config.permutation_samples as usize,
                    seed: config.seed,
                    alpha: 0.05,
                },
                &partition_plan,
                || should_cancel(),
            )
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            micom_pairs.push(result);
        }
    }
    let evidence = construct_ids
        .iter()
        .map(|construct_id| {
            let compositional_invariance = micom_pairs
                .iter()
                .map(|pair| {
                    let construct = pair
                        .constructs
                        .iter()
                        .find(|row| row.construct_id == *construct_id)
                        .ok_or_else(|| {
                            MultiModRunnerErrorV1::ResultContract(format!(
                                "POS MICOM pair omitted construct {construct_id}"
                            ))
                        })?;
                    Ok(PosPairwiseCompositionalInvarianceV1 {
                        left_segment: pair.pair.group_a.get(),
                        right_segment: pair.pair.group_b.get(),
                        passed: pair.complete && construct.compositional_invariance,
                        permutation_p_value: construct.compositional_invariance_probability,
                    })
                })
                .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?;
            let step3_equality = micom_pairs
                .iter()
                .map(|pair| {
                    let construct = pair
                        .constructs
                        .iter()
                        .find(|row| row.construct_id == *construct_id)
                        .expect("construct inventory checked above");
                    PosPairwiseStep3EqualityV1 {
                        left_segment: pair.pair.group_a.get(),
                        right_segment: pair.pair.group_b.get(),
                        mean_equality_passed: pair.complete && construct.equal_means,
                        variance_equality_passed: pair.complete && construct.equal_variances,
                    }
                })
                .collect();
            Ok(PosConstructComparabilityEvidenceV1 {
                construct_id: construct_id.clone(),
                configural_identity_passed: receipt.identical_indicators_and_coding
                    && receipt.identical_data_treatment
                    && receipt.identical_algorithm_settings
                    && receipt.identical_model_specification
                    && receipt.deterministic_orientation_reviewed
                    && receipt.analyst_review_confirmed,
                compositional_invariance,
                step3_equality,
            })
        })
        .collect::<Result<Vec<_>, MultiModRunnerErrorV1>>()?;
    let gate_input = PosCommonMetricGateInputV1 {
        pooled_metric_id: prepared.fimix_input.metric.metric_id.clone(),
        pooled_metric_sha256: prepared.fimix_input.metric.source_sha256.clone(),
        segments,
        applied_identically_to_all_segments: true,
        required_construct_ids: construct_ids,
        evidence,
    };
    let gate_result = evaluate_pos_common_metric_gate_v1(&gate_input);
    let baselines = fit_pooled_metric_segment_baselines_v2(
        &prepared.fimix_input,
        assignments,
        segments,
        config.fimix.rank_tolerance,
    )
    .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    let common_metric_parameters =
        common_metric_segment_parameters_v2(&baselines, &prepared.fimix_scientific_targets)?;
    let prepared = PreparedPosCommonMetricEvidenceV1 {
        method_version: "qpls.pos-common-metric.runner.v1".into(),
        gate_input,
        gate_result,
        micom_pairs,
        common_metric_parameters,
    };
    prepared
        .ensure_valid()
        .map_err(MultiModRunnerErrorV1::ResultContract)?;
    Ok(prepared)
}

fn prepare_raw_heterogeneity_execution_v2<C>(
    dataset: &Dataset,
    authority: &RawHeterogeneityAuthorityV2,
    profile: CoreHeterogeneityProfileV2,
    should_cancel: &C,
) -> Result<
    (
        PreparedHeterogeneityExecutionV2,
        PlsResult,
        OrdinaryPlsRawScoreCacheV1,
        Vec<u64>,
    ),
    MultiModRunnerErrorV1,
>
where
    C: Fn() -> bool + Sync,
{
    let pooled_fit = raw_heterogeneity_stage_one_fit_v2(dataset, authority, should_cancel)?;
    let metric = raw_heterogeneity_metric_v2(&authority.plan, &pooled_fit, profile)?;
    let fimix_scientific_targets = raw_fimix_scientific_targets_v2(authority, &metric)?;
    let joint = raw_joint_structural_point_v2(authority, &pooled_fit, profile, should_cancel)?;
    let (pos_parameter_ids, _) = scientific_parameter_signature_v2(authority, &pooled_fit, &joint)?;
    let orientation_rows = (0..dataset.batch.num_rows())
        .map(|row| row as u64)
        .collect::<Vec<_>>();
    let raw_scores = OrdinaryPlsRawScoreCacheV1::build(
        dataset,
        &authority.source_columns,
        &orientation_rows,
        should_cancel,
    )?;
    Ok((
        PreparedHeterogeneityExecutionV2 {
            fimix_input: metric.fimix_input,
            fimix_scientific_targets,
            pos_start_features: metric.pos_start_features,
            pos_parameter_ids,
            pos_common_metric_gate: None,
            pos_common_metric_parameters: Vec::new(),
            pos_common_metric_contrasts: Vec::new(),
            pos_common_metric_micom_pairs: Vec::new(),
            bootstrap: None,
        },
        pooled_fit,
        raw_scores,
        orientation_rows,
    ))
}

#[derive(Debug, Clone)]
enum RawLockedHeterogeneityFitV2 {
    Fimix(FimixPlsV2Result),
    Pos(PlsPosV2Result),
}

impl RawLockedHeterogeneityFitV2 {
    fn assignments(&self) -> &[usize] {
        match self {
            Self::Fimix(result) => &result.hard_assignments,
            Self::Pos(result) => &result.assignments,
        }
    }

    fn fit_statistic(&self) -> f64 {
        match self {
            Self::Fimix(result) => result.log_likelihood,
            Self::Pos(result) => result.objective,
        }
    }
}

fn fit_raw_locked_heterogeneity_v2<C>(
    dataset: &Dataset,
    authority: &RawHeterogeneityAuthorityV2,
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    algorithm: CoreHeterogeneityAlgorithmV2,
    k: u8,
    fit_seed: u64,
    use_pooled_common_metric: bool,
    should_cancel: &C,
) -> Result<
    (
        RawLockedHeterogeneityFitV2,
        Vec<HeterogeneityClassParameterV2>,
    ),
    MultiModRunnerErrorV1,
>
where
    C: Fn() -> bool + Sync,
{
    let (prepared, pooled_fit, raw_scores, orientation_rows) =
        prepare_raw_heterogeneity_execution_v2(dataset, authority, config.profile, should_cancel)?;
    match algorithm {
        CoreHeterogeneityAlgorithmV2::FimixPlsV2 => {
            let mut settings = fimix_config(config, k as usize);
            settings.seed = fit_seed;
            let result = fit_fimix_pls_v2(&prepared.fimix_input, &settings)
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            validate_fimix_multistart_evidence_v2(&result)
                .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
            let parameters = fimix_parameters(&result, &prepared.fimix_scientific_targets)?;
            Ok((RawLockedHeterogeneityFitV2::Fimix(result), parameters))
        }
        CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2
        | CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
            let tandem_fimix_required = matches!(
                &config.phase,
                qpls_core::HeterogeneityPhaseV2::Inference { lock }
                    if lock.tandem_fimix_same_k_start_required
            );
            let tandem_fimix = if tandem_fimix_required {
                let mut settings = fimix_config(config, k as usize);
                settings.seed = fit_seed;
                let result =
                    fit_fimix_pls_v2(&prepared.fimix_input, &settings).map_err(|error| {
                        MultiModRunnerErrorV1::Kernel(format!(
                            "multimod.runner.heterogeneity.tandem_fimix_refit_failed: {error}"
                        ))
                    })?;
                validate_fimix_multistart_evidence_v2(&result)
                    .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
                Some(result)
            } else {
                None
            };
            let starts = build_pls_pos_start_plan_v2(
                &prepared.pos_start_features,
                k as usize,
                fit_seed,
                tandem_fimix
                    .as_ref()
                    .map(|result| result.hard_assignments.as_slice()),
            )
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            let settings = pos_config(config, k as usize, prepared.pos_start_features.len());
            let scientific_row_features =
                raw_heterogeneity_scientific_row_features_v2(dataset, &authority.source_columns)?;
            let mut refitter = RawPlsPosRefitterV2 {
                dataset,
                authority,
                profile: config.profile,
                pooled_fit: &pooled_fit,
                raw_scores: &raw_scores,
                orientation_rows: &orientation_rows,
                retain_outcome_audits: true,
                should_cancel,
            };
            let result = match algorithm {
                CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2 => {
                    fit_pls_pos_published_with_scientific_row_features_v2(
                        &starts,
                        &scientific_row_features,
                        &settings,
                        &mut refitter,
                    )
                }
                CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
                    fit_pls_pos_destination_scored_interactions_with_scientific_row_features_v2(
                        &starts,
                        &scientific_row_features,
                        estimation_heterogeneity_profile(config.profile),
                        &settings,
                        &mut refitter,
                    )
                }
                CoreHeterogeneityAlgorithmV2::FimixPlsV2 => unreachable!(),
            }
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            validate_pos_multistart_evidence_v2(&result)
                .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
            let parameters = if use_pooled_common_metric {
                let baselines = fit_pooled_metric_segment_baselines_v2(
                    &prepared.fimix_input,
                    &result.assignments,
                    k as usize,
                    config.fimix.rank_tolerance,
                )
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
                common_metric_segment_parameters_v2(&baselines, &prepared.fimix_scientific_targets)?
            } else {
                pos_parameters(&result, &prepared.pos_parameter_ids)?
            };
            Ok((RawLockedHeterogeneityFitV2::Pos(result), parameters))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawHeterogeneityBootstrapEstimateV2 {
    fit_statistic: f64,
    alignment: LabelAlignmentV2,
    target_values: Vec<f64>,
}

pub type RawHeterogeneityBootstrapShardCacheV2 =
    MultiModShardCacheV1<MultiModCaseBootstrapDrawV1, RawHeterogeneityBootstrapEstimateV2>;

fn target_suffix_v2(target_id: &str) -> Option<&str> {
    target_id.split_once(':').map(|(_, suffix)| suffix)
}

fn aligned_heterogeneity_target_values_v2(
    reference_target_ids: &[String],
    candidate: &[HeterogeneityClassParameterV2],
    alignment: &LabelAlignmentV2,
) -> Result<Vec<f64>, MultiModRefitFailureV1> {
    reference_target_ids
        .iter()
        .map(|reference_id| {
            let prefix = reference_id
                .split_once(':')
                .map(|(prefix, _)| prefix)
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "target_identity".into(),
                    message: format!("reference target {reference_id} has no class prefix"),
                })?;
            let reference_class = prefix
                .rsplit_once('_')
                .and_then(|(_, value)| value.parse::<usize>().ok())
                .and_then(|value| value.checked_sub(1))
                .filter(|value| *value < alignment.candidate_to_reference.len())
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "target_identity".into(),
                    message: format!("reference target {reference_id} has an invalid class"),
                })?;
            let candidate_class = alignment
                .candidate_to_reference
                .iter()
                .position(|mapped| *mapped == reference_class)
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "target_identity".into(),
                    message: format!(
                        "no candidate label maps to reference class {reference_class}"
                    ),
                })?;
            let suffix = target_suffix_v2(reference_id).ok_or_else(|| MultiModRefitFailureV1 {
                code: "target_identity".into(),
                message: format!("reference target {reference_id} has no suffix"),
            })?;
            candidate
                .iter()
                .find(|row| {
                    row.class_id as usize == candidate_class + 1
                        && target_suffix_v2(&row.parameter.target_id) == Some(suffix)
                })
                .map(|row| row.parameter.estimate)
                .filter(|value| value.is_finite())
                .ok_or_else(|| MultiModRefitFailureV1 {
                    code: "nonfinite_target".into(),
                    message: format!(
                        "candidate class {} omitted finite aligned target {suffix}",
                        candidate_class + 1
                    ),
                })
        })
        .collect()
}

struct RawHeterogeneityBootstrapCallbackV2<'a, C> {
    dataset: &'a Dataset,
    authority: &'a RawHeterogeneityAuthorityV2,
    sampling_positions: &'a [usize],
    config: &'a qpls_core::PlsUnobservedHeterogeneityConfigV2,
    algorithm: CoreHeterogeneityAlgorithmV2,
    k: u8,
    bootstrap_plan: &'a HeterogeneityBootstrapPlanV2,
    reference_assignments: &'a [usize],
    reference_target_ids: &'a [String],
    use_pooled_common_metric: bool,
    should_cancel: &'a C,
}

impl<C>
    MultiModInterruptibleFullRefitCallbackV1<
        MultiModCaseBootstrapDrawV1,
        RawHeterogeneityBootstrapEstimateV2,
    > for RawHeterogeneityBootstrapCallbackV2<'_, C>
where
    C: Fn() -> bool + Sync,
{
    fn full_refit_attempt(
        &mut self,
        draw: &MultiModCaseBootstrapDrawV1,
    ) -> MultiModRefitAttemptV1<RawHeterogeneityBootstrapEstimateV2> {
        if (self.should_cancel)() {
            return MultiModRefitAttemptV1::Interrupted;
        }
        let result = self.full_refit_scientific(draw);
        if result
            .as_ref()
            .is_err_and(|failure| failure.code == "cancelled")
        {
            MultiModRefitAttemptV1::Interrupted
        } else {
            MultiModRefitAttemptV1::Completed(result)
        }
    }
}

impl<C> RawHeterogeneityBootstrapCallbackV2<'_, C>
where
    C: Fn() -> bool + Sync,
{
    fn full_refit_scientific(
        &mut self,
        draw: &MultiModCaseBootstrapDrawV1,
    ) -> Result<RawHeterogeneityBootstrapEstimateV2, MultiModRefitFailureV1> {
        let indices = draw
            .source_rows
            .iter()
            .map(|row| {
                self.sampling_positions
                    .get(*row as usize)
                    .copied()
                    .ok_or_else(|| MultiModRefitFailureV1 {
                        code: "draw_identity".into(),
                        message: "heterogeneity bootstrap position is outside the frozen scientific row order"
                            .into(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sampled = resample_dataset_columns_v1(
            self.dataset,
            &self.authority.source_columns,
            &indices,
            || (self.should_cancel)(),
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if matches!(&error, EstimationError::Cancelled) {
                "cancelled".into()
            } else {
                "fit_failed".into()
            },
            message: format!("bootstrap dataset projection failed: {error}"),
        })?;
        let seed = heterogeneity_bootstrap_replicate_seed_v2(
            self.bootstrap_plan,
            draw.replicate_index as usize,
        );
        let (fit, parameters) = fit_raw_locked_heterogeneity_v2(
            &sampled,
            self.authority,
            self.config,
            self.algorithm,
            self.k,
            seed,
            self.use_pooled_common_metric,
            self.should_cancel,
        )
        .map_err(|error| MultiModRefitFailureV1 {
            code: if matches!(&error, MultiModRunnerErrorV1::Cancelled) || (self.should_cancel)() {
                "cancelled".into()
            } else {
                "fit_failed".into()
            },
            message: error.to_string(),
        })?;
        let reference_on_draw = indices
            .iter()
            .map(|row| self.reference_assignments[*row])
            .collect::<Vec<_>>();
        let alignment =
            align_labels_exhaustive_v2(&reference_on_draw, fit.assignments(), self.k as usize)
                .map_err(|error| MultiModRefitFailureV1 {
                    code: "label_ambiguous".into(),
                    message: error.to_string(),
                })?;
        if alignment.ambiguous {
            return Err(MultiModRefitFailureV1 {
                code: "label_ambiguous".into(),
                message: "bootstrap class/segment label match was ambiguous".into(),
            });
        }
        if !alignment.mutual_majority {
            return Err(MultiModRefitFailureV1 {
                code: "label_not_mutual_majority".into(),
                message: "bootstrap class/segment match failed mutual-majority overlap".into(),
            });
        }
        let target_values = aligned_heterogeneity_target_values_v2(
            self.reference_target_ids,
            &parameters,
            &alignment,
        )?;
        Ok(RawHeterogeneityBootstrapEstimateV2 {
            fit_statistic: fit.fit_statistic(),
            alignment,
            target_values,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedRawHeterogeneityBootstrapReferenceV2 {
    pub method_version: String,
    pub dataset_fingerprint: String,
    pub compilation_identity_sha256: String,
    pub config_identity_sha256: String,
    pub point_pass_identity_sha256: String,
    pub pooled_metric_sha256: String,
    pub complete_source_row_tokens: Vec<u64>,
    pub algorithm: CoreHeterogeneityAlgorithmV2,
    pub k: u8,
    pub use_pooled_common_metric: bool,
    pub heterogeneity_plan: HeterogeneityBootstrapPlanV2,
    pub orchestrator_plan: MultiModBootstrapPlanV1,
    pub reference_assignments: Vec<usize>,
    pub reference_target_ids: Vec<String>,
    pub reference_parameter_identity_sha256: String,
    pub reference_fit_statistic: f64,
    pub reference_identity_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedRawHeterogeneityBootstrapExecutionV2 {
    pub method_version: String,
    pub dataset_fingerprint: String,
    pub compilation_identity_sha256: String,
    pub raw_preparation_receipt: RawHeterogeneityPreparationReceiptV2,
    pub prepared_point: PreparedHeterogeneityExecutionV2,
    pub point_pass: PreparedHeterogeneityPointPassV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub common_metric_evidence: Option<PreparedPosCommonMetricEvidenceV1>,
    pub reference: PreparedRawHeterogeneityBootstrapReferenceV2,
    pub execution_identity_sha256: String,
}

fn raw_heterogeneity_reference_scientific_identity_v2(
    reference: &PreparedRawHeterogeneityBootstrapReferenceV2,
) -> String {
    sha256_serialized(&(
        reference.method_version.as_str(),
        reference.dataset_fingerprint.as_str(),
        reference.compilation_identity_sha256.as_str(),
        reference.config_identity_sha256.as_str(),
        reference.point_pass_identity_sha256.as_str(),
        reference.pooled_metric_sha256.as_str(),
        reference.complete_source_row_tokens.as_slice(),
        reference.algorithm,
        reference.k,
        reference.use_pooled_common_metric,
        reference.reference_assignments.as_slice(),
        reference.reference_target_ids.as_slice(),
        reference.reference_parameter_identity_sha256.as_str(),
        reference.reference_fit_statistic.to_bits(),
    ))
}

fn raw_heterogeneity_reference_identity_v2(
    reference: &PreparedRawHeterogeneityBootstrapReferenceV2,
) -> String {
    sha256_serialized(&(
        raw_heterogeneity_reference_scientific_identity_v2(reference),
        &reference.heterogeneity_plan,
        &reference.orchestrator_plan,
    ))
}

impl PreparedRawHeterogeneityBootstrapReferenceV2 {
    fn ensure_valid(
        &self,
        dataset: &Dataset,
        artifact: &CompiledMultiModRecipeV1,
        config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
        source_row_tokens: &[u64],
    ) -> Result<(), String> {
        let bootstrap = config.bootstrap.as_ref().ok_or_else(|| {
            "resumable heterogeneity reference requires bootstrap settings".to_string()
        })?;
        let lock = match &config.phase {
            qpls_core::HeterogeneityPhaseV2::Inference { lock } => lock,
            qpls_core::HeterogeneityPhaseV2::Discovery { .. } => {
                return Err(
                    "resumable heterogeneity reference requires a locked inference phase".into(),
                );
            }
        };
        if self.method_version != "qpls.heterogeneity.bootstrap-reference.v2"
            || self.dataset_fingerprint != dataset.fingerprint.0
            || self.compilation_identity_sha256 != artifact.receipt().analytical_identity_sha256
            || self.config_identity_sha256 != sha256_serialized(config)
            || !is_lower_hex_sha256_v1(&self.point_pass_identity_sha256)
            || !is_lower_hex_sha256_v1(&self.pooled_metric_sha256)
            || !is_lower_hex_sha256_v1(&self.reference_parameter_identity_sha256)
            || self.complete_source_row_tokens != source_row_tokens
            || self.algorithm != lock.selected_algorithm
            || self.k != lock.selected_k
            || self.reference_assignments.len() != source_row_tokens.len()
            || self
                .reference_assignments
                .iter()
                .any(|class| *class >= usize::from(self.k))
            || (0..usize::from(self.k)).any(|class| !self.reference_assignments.contains(&class))
            || !self.reference_fit_statistic.is_finite()
            || self.reference_target_ids.is_empty()
            || self
                .reference_target_ids
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.reference_target_ids.len()
            || self.reference_target_ids.iter().any(|target| {
                target_suffix_v2(target).is_none()
                    || target
                        .split_once(':')
                        .and_then(|(prefix, _)| prefix.rsplit_once('_'))
                        .and_then(|(_, class)| class.parse::<u8>().ok())
                        .is_none_or(|class| class == 0 || class > self.k)
            })
        {
            return Err("resumable heterogeneity reference differs from its compiler, data, lock, row, or target authority".into());
        }
        let expected_plan = HeterogeneityBootstrapPlanV2 {
            algorithm: heterogeneity_bootstrap_algorithm(self.algorithm),
            fixed_classes_or_segments: usize::from(self.k),
            requested_replicates: bootstrap.resamples as usize,
            master_seed: bootstrap.seed,
            confidence_level: bootstrap.confidence_level,
            minimum_usable_share: 0.90,
        };
        if self.heterogeneity_plan != expected_plan
            || self.orchestrator_plan.schema_version != MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1
            || self.orchestrator_plan.requested_replicates != bootstrap.resamples
            || self.orchestrator_plan.master_seed != bootstrap.seed
            || self.orchestrator_plan.minimum_usable_fraction.to_bits() != 0.90f64.to_bits()
            || self.orchestrator_plan.scientific_refit_identity_sha256
                != raw_heterogeneity_reference_scientific_identity_v2(self)
            || self.orchestrator_plan.ensure_valid().is_err()
            || !is_lower_hex_sha256_v1(&self.reference_identity_sha256)
            || self.reference_identity_sha256 != raw_heterogeneity_reference_identity_v2(self)
        {
            return Err(
                "resumable heterogeneity reference has an invalid bootstrap plan or identity"
                    .into(),
            );
        }
        Ok(())
    }
}

fn raw_heterogeneity_execution_identity_v2(
    execution: &PreparedRawHeterogeneityBootstrapExecutionV2,
) -> String {
    sha256_serialized(&(
        execution.method_version.as_str(),
        execution.dataset_fingerprint.as_str(),
        execution.compilation_identity_sha256.as_str(),
        &execution.raw_preparation_receipt,
        &execution.prepared_point,
        &execution.point_pass,
        execution.common_metric_evidence.as_ref(),
        &execution.reference,
    ))
}

impl PreparedRawHeterogeneityBootstrapExecutionV2 {
    pub fn ensure_valid(
        &self,
        dataset: &Dataset,
        artifact: &CompiledMultiModRecipeV1,
        config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
        source_row_tokens: &[u64],
        general_sem_plan_sha256: &str,
    ) -> Result<(), String> {
        if self.method_version != "qpls.heterogeneity.raw-bootstrap-execution.v2"
            || self.dataset_fingerprint != dataset.fingerprint.0
            || self.compilation_identity_sha256 != artifact.receipt().analytical_identity_sha256
            || !is_lower_hex_sha256_v1(&self.execution_identity_sha256)
            || self.execution_identity_sha256 != raw_heterogeneity_execution_identity_v2(self)
            || self.raw_preparation_receipt.pooled_metric_sha256
                != self.prepared_point.fimix_input.metric.source_sha256
            || self.raw_preparation_receipt.fimix_input != self.prepared_point.fimix_input
        {
            return Err("prepared raw heterogeneity bootstrap execution differs from its data, compiler, metric, or identity".into());
        }
        self.raw_preparation_receipt.ensure_valid()?;
        self.raw_preparation_receipt.ensure_matches_live_authority(
            dataset.batch.num_rows(),
            source_row_tokens,
            general_sem_plan_sha256,
        )?;
        self.point_pass
            .ensure_valid(artifact, config, &self.prepared_point)?;
        self.reference
            .ensure_valid(dataset, artifact, config, source_row_tokens)?;
        let locked = self.point_pass.locked.as_ref().ok_or_else(|| {
            "prepared raw bootstrap execution omitted its locked point".to_string()
        })?;
        let expected_common = self
            .common_metric_evidence
            .as_ref()
            .map(|evidence| {
                evidence.ensure_valid()?;
                Ok::<_, String>((
                    evidence.gate_input.clone(),
                    evidence.common_metric_parameters.clone(),
                    evidence.micom_pairs.clone(),
                ))
            })
            .transpose()?;
        let prepared_common = self
            .prepared_point
            .pos_common_metric_gate
            .clone()
            .map(|gate| {
                (
                    gate,
                    self.prepared_point.pos_common_metric_parameters.clone(),
                    self.prepared_point.pos_common_metric_micom_pairs.clone(),
                )
            });
        if expected_common != prepared_common
            || self.reference.point_pass_identity_sha256
                != self.point_pass.point_pass_identity_sha256
            || self.reference.pooled_metric_sha256
                != self.prepared_point.fimix_input.metric.source_sha256
            || self.reference.algorithm != locked.algorithm
            || self.reference.k != locked.k
            || self.reference.reference_assignments != locked.assignments
            || self.reference.reference_fit_statistic.to_bits() != locked.fit_statistic.to_bits()
        {
            return Err("prepared raw bootstrap reference is not owned by the retained point/common-metric pass".into());
        }
        let use_common = self
            .prepared_point
            .pos_common_metric_gate
            .as_ref()
            .is_some_and(|gate| {
                evaluate_pos_common_metric_gate_v1(gate).status
                    == PosCommonMetricGateStatusV1::Passed
            });
        let expected_targets = if use_common {
            &self.prepared_point.pos_common_metric_parameters
        } else {
            &locked.local_parameters
        }
        .iter()
        .map(|row| row.parameter.target_id.clone())
        .collect::<Vec<_>>();
        if self.reference.use_pooled_common_metric != use_common
            || self.reference.reference_target_ids != expected_targets
            || self.reference.reference_parameter_identity_sha256
                != sha256_serialized(if use_common {
                    &self.prepared_point.pos_common_metric_parameters
                } else {
                    &locked.local_parameters
                })
        {
            return Err("prepared raw bootstrap reference target metric differs from the common-metric gate".into());
        }
        Ok(())
    }
}

/// Executes and freezes the single point pass used by common-metric evidence,
/// every bootstrap shard, and final result assembly. No bootstrap draw is run
/// by this preparation step.
pub fn prepare_compiled_raw_pls_heterogeneity_bootstrap_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    should_cancel: C,
    progress: P,
) -> Result<PreparedRawHeterogeneityBootstrapExecutionV2, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    let config = recipe.pls_heterogeneity.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "heterogeneity configuration disappeared after compilation".into(),
        )
    })?;
    let bootstrap = config.bootstrap.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "resumable heterogeneity preparation requires bootstrap settings".into(),
        )
    })?;
    let lock = match &config.phase {
        qpls_core::HeterogeneityPhaseV2::Inference { lock } => lock,
        qpls_core::HeterogeneityPhaseV2::Discovery { .. } => {
            return Err(MultiModRunnerErrorV1::Authority(
                "resumable heterogeneity preparation requires a locked inference phase".into(),
            ));
        }
    };
    let CompiledMultiModPlanV1::PlsHeterogeneityV2 {
        profile,
        algorithms,
        candidate_k,
        ..
    } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not PLS heterogeneity V2".into(),
        ));
    };
    if profile != &config.profile
        || algorithms != &lock.discovery_algorithms
        || candidate_k != &lock.discovery_candidate_k
    {
        return Err(MultiModRunnerErrorV1::Authority(
            "multimod.runner.heterogeneity.lock_compiler_inventory_mismatch".into(),
        ));
    }
    let predicted_sidecar_bytes = predict_heterogeneity_sidecar_bytes_v2(
        config,
        dataset.batch.num_rows(),
        multimod_model_target_upper_bound_v1(&[], model),
    );
    enforce_multimod_sidecar_cost_v1("heterogeneity", predicted_sidecar_bytes, &progress)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::PreparingPointInputs,
        0,
        1,
        "heterogeneity:raw_projection",
    );
    let authority = projected_heterogeneity_authority_v2(dataset, recipe, model, artifact)?;
    let (complete_dataset, source_row_tokens) =
        complete_heterogeneity_dataset_v2(dataset, &authority.source_columns, &should_cancel)?;
    let (mut prepared_point, pooled_fit, raw_scores, orientation_rows) =
        prepare_raw_heterogeneity_execution_v2(
            &complete_dataset,
            &authority,
            config.profile,
            &should_cancel,
        )?;
    let pos_scientific_row_features =
        raw_heterogeneity_scientific_row_features_v2(&complete_dataset, &authority.source_columns)?;
    let mut refitter = RawPlsPosRefitterV2 {
        dataset: &complete_dataset,
        authority: &authority,
        profile: config.profile,
        pooled_fit: &pooled_fit,
        raw_scores: &raw_scores,
        orientation_rows: &orientation_rows,
        retain_outcome_audits: true,
        should_cancel: &should_cancel,
    };
    let point_pass = execute_heterogeneity_point_pass_v2(
        artifact,
        config,
        algorithms,
        candidate_k,
        &prepared_point,
        &pos_scientific_row_features,
        &mut refitter,
        &should_cancel,
        &progress,
    )?;
    let locked = point_pass.locked.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::ResultContract(
            "locked heterogeneity point pass did not retain its selected candidate".into(),
        )
    })?;

    let common_metric_evidence = if config
        .pos_common_metric
        .as_ref()
        .is_some_and(|gate| gate.request_segment_contrasts)
    {
        if matches!(locked.algorithm, CoreHeterogeneityAlgorithmV2::FimixPlsV2) {
            return Err(MultiModRunnerErrorV1::Authority(
                "POS common-metric contrasts cannot use a FIMIX lock".into(),
            ));
        }
        let mut common_refitter = RawPlsPosRefitterV2 {
            dataset: &complete_dataset,
            authority: &authority,
            profile: config.profile,
            pooled_fit: &pooled_fit,
            raw_scores: &raw_scores,
            orientation_rows: &orientation_rows,
            retain_outcome_audits: true,
            should_cancel: &should_cancel,
        };
        let evidence = prepare_raw_pos_common_metric_v1(
            &locked.assignments,
            config,
            &prepared_point,
            &mut common_refitter,
        )?;
        prepared_point.pos_common_metric_gate = Some(evidence.gate_input.clone());
        prepared_point.pos_common_metric_parameters = evidence.common_metric_parameters.clone();
        prepared_point.pos_common_metric_micom_pairs = evidence.micom_pairs.clone();
        Some(evidence)
    } else {
        None
    };
    let use_pooled_common_metric =
        prepared_point
            .pos_common_metric_gate
            .as_ref()
            .is_some_and(|gate| {
                evaluate_pos_common_metric_gate_v1(gate).status
                    == PosCommonMetricGateStatusV1::Passed
            });
    let reference_parameters = if use_pooled_common_metric {
        &prepared_point.pos_common_metric_parameters
    } else {
        &locked.local_parameters
    };
    let reference_target_ids = reference_parameters
        .iter()
        .map(|row| row.parameter.target_id.clone())
        .collect::<Vec<_>>();
    let heterogeneity_plan = HeterogeneityBootstrapPlanV2 {
        algorithm: heterogeneity_bootstrap_algorithm(locked.algorithm),
        fixed_classes_or_segments: usize::from(locked.k),
        requested_replicates: bootstrap.resamples as usize,
        master_seed: bootstrap.seed,
        confidence_level: bootstrap.confidence_level,
        minimum_usable_share: 0.90,
    };
    let mut reference = PreparedRawHeterogeneityBootstrapReferenceV2 {
        method_version: "qpls.heterogeneity.bootstrap-reference.v2".into(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        compilation_identity_sha256: artifact.receipt().analytical_identity_sha256.clone(),
        config_identity_sha256: sha256_serialized(config),
        point_pass_identity_sha256: point_pass.point_pass_identity_sha256.clone(),
        pooled_metric_sha256: prepared_point.fimix_input.metric.source_sha256.clone(),
        complete_source_row_tokens: source_row_tokens.clone(),
        algorithm: locked.algorithm,
        k: locked.k,
        use_pooled_common_metric,
        heterogeneity_plan,
        orchestrator_plan: MultiModBootstrapPlanV1 {
            schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
            scientific_refit_identity_sha256: "0".repeat(64),
            requested_replicates: bootstrap.resamples,
            master_seed: bootstrap.seed,
            minimum_usable_fraction: 0.90,
        },
        reference_assignments: locked.assignments.clone(),
        reference_target_ids,
        reference_parameter_identity_sha256: sha256_serialized(reference_parameters),
        reference_fit_statistic: locked.fit_statistic,
        reference_identity_sha256: String::new(),
    };
    reference.orchestrator_plan.scientific_refit_identity_sha256 =
        raw_heterogeneity_reference_scientific_identity_v2(&reference);
    reference.reference_identity_sha256 = raw_heterogeneity_reference_identity_v2(&reference);
    let raw_preparation_receipt = RawHeterogeneityPreparationReceiptV2 {
        method_version: "qpls.heterogeneity.raw-preparation.v2".into(),
        general_sem_plan_sha256: authority.plan.deterministic_sha256(),
        pooled_metric_sha256: prepared_point.fimix_input.metric.source_sha256.clone(),
        omitted_source_rows: dataset
            .batch
            .num_rows()
            .saturating_sub(source_row_tokens.len()),
        source_row_tokens,
        unique_analysis_positions: true,
        fimix_input: prepared_point.fimix_input.clone(),
    };
    let mut execution = PreparedRawHeterogeneityBootstrapExecutionV2 {
        method_version: "qpls.heterogeneity.raw-bootstrap-execution.v2".into(),
        dataset_fingerprint: dataset.fingerprint.0.clone(),
        compilation_identity_sha256: artifact.receipt().analytical_identity_sha256.clone(),
        raw_preparation_receipt,
        prepared_point,
        point_pass,
        common_metric_evidence,
        reference,
        execution_identity_sha256: String::new(),
    };
    execution.execution_identity_sha256 = raw_heterogeneity_execution_identity_v2(&execution);
    execution
        .ensure_valid(
            dataset,
            artifact,
            config,
            &execution.reference.complete_source_row_tokens,
            &authority.plan.deterministic_sha256(),
        )
        .map_err(MultiModRunnerErrorV1::ResultContract)?;
    Ok(execution)
}

/// Executes or resumes one exact modulo-owned bootstrap shard. The returned
/// cache is always app-owned and serializable; the runner performs no durable
/// writes. `cancelled=true` means an in-flight attempt was interrupted and no
/// record was committed for that replicate.
#[allow(clippy::too_many_arguments)]
pub fn run_prepared_raw_pls_heterogeneity_bootstrap_shard_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    execution: &PreparedRawHeterogeneityBootstrapExecutionV2,
    shard: MultiModShardSpecV1,
    resume: Option<RawHeterogeneityBootstrapShardCacheV2>,
    should_cancel: C,
    progress: P,
) -> Result<RawHeterogeneityBootstrapShardCacheV2, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    let config = recipe.pls_heterogeneity.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "heterogeneity configuration disappeared after compilation".into(),
        )
    })?;
    let authority = projected_heterogeneity_authority_v2(dataset, recipe, model, artifact)?;
    let (complete_dataset, source_row_tokens) =
        complete_heterogeneity_dataset_v2(dataset, &authority.source_columns, &should_cancel)?;
    let complete_row_count = u32::try_from(complete_dataset.batch.num_rows()).map_err(|_| {
        MultiModRunnerErrorV1::PreparedInput(
            "heterogeneity complete-case rows exceed the bootstrap row-index contract".into(),
        )
    })?;
    let complete_source_rows = (0..complete_row_count).collect::<Vec<_>>();
    let sampling_positions = canonical_multimod_row_permutation_v1(
        &complete_dataset,
        &complete_source_rows,
        &authority.source_columns,
        &BTreeSet::new(),
    )
    .map_err(|error| {
        MultiModRunnerErrorV1::PreparedInput(format!(
            "heterogeneity scientific row-order preparation failed: {error}"
        ))
    })?;
    execution
        .ensure_valid(
            dataset,
            artifact,
            config,
            &source_row_tokens,
            &authority.plan.deterministic_sha256(),
        )
        .map_err(MultiModRunnerErrorV1::PreparedInput)?;
    let initial_completed = resume.as_ref().map_or(0, |cache| cache.records.len()) as u64;
    report(
        &progress,
        MultiModRunnerPhaseV1::Resampling,
        initial_completed,
        u64::from(execution.reference.orchestrator_plan.requested_replicates),
        format!(
            "heterogeneity:fixed_k_bootstrap:shard_{}/{}",
            shard.shard_index, shard.shard_count
        ),
    );
    let mut callback = RawHeterogeneityBootstrapCallbackV2 {
        dataset: &complete_dataset,
        authority: &authority,
        sampling_positions: &sampling_positions,
        config,
        algorithm: execution.reference.algorithm,
        k: execution.reference.k,
        bootstrap_plan: &execution.reference.heterogeneity_plan,
        reference_assignments: &execution.reference.reference_assignments,
        reference_target_ids: &execution.reference.reference_target_ids,
        use_pooled_common_metric: execution.reference.use_pooled_common_metric,
        should_cancel: &should_cancel,
    };
    let cache = run_multimod_case_bootstrap_shard_interruptible_v1(
        &execution.reference.orchestrator_plan,
        complete_dataset.batch.num_rows(),
        None,
        shard,
        resume,
        &mut callback,
        || should_cancel(),
    )
    .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))?;
    report(
        &progress,
        MultiModRunnerPhaseV1::Resampling,
        cache.records.len() as u64,
        u64::from(execution.reference.orchestrator_plan.requested_replicates),
        format!(
            "heterogeneity:fixed_k_bootstrap:shard_{}/{}",
            shard.shard_index, shard.shard_count
        ),
    );
    Ok(cache)
}

fn prepared_heterogeneity_bootstrap_from_final_ledger_v2(
    reference: &PreparedRawHeterogeneityBootstrapReferenceV2,
    ledger: MultiModFinalLedgerV1<MultiModCaseBootstrapDrawV1, RawHeterogeneityBootstrapEstimateV2>,
) -> Result<PreparedHeterogeneityBootstrapV2, MultiModRunnerErrorV1> {
    let requested = reference.orchestrator_plan.requested_replicates as usize;
    if !ledger.complete
        || ledger.requested as usize != requested
        || ledger.records.len() != requested
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "global heterogeneity bootstrap ledger is incomplete".into(),
        ));
    }
    let mut targets = reference
        .reference_target_ids
        .iter()
        .map(|target_id| PreparedHeterogeneityBootstrapTargetV2 {
            target_id: target_id.clone(),
            estimates: vec![None; requested],
        })
        .collect::<Vec<_>>();
    let mut entries = Vec::with_capacity(requested);
    for record in ledger.records {
        let replicate_index = record.index as usize;
        let seed = heterogeneity_bootstrap_replicate_seed_v2(
            &reference.heterogeneity_plan,
            replicate_index,
        );
        match record.outcome {
            MultiModRefitOutcomeV1::Success { value, .. } => {
                if value.target_values.len() != targets.len()
                    || value.target_values.iter().any(|value| !value.is_finite())
                {
                    entries.push(HeterogeneityBootstrapLedgerEntryV2 {
                        replicate_index,
                        seed,
                        status: qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::NonFiniteTarget,
                        fit_statistic: None,
                        label_alignment: None,
                        target_payload_sha256: None,
                        failure_reason: Some(
                            "bootstrap target vector was nonfinite or had the wrong cardinality"
                                .into(),
                        ),
                    });
                    continue;
                }
                for (target, estimate) in targets.iter_mut().zip(&value.target_values) {
                    target.estimates[replicate_index] = Some(*estimate);
                }
                entries.push(HeterogeneityBootstrapLedgerEntryV2 {
                    replicate_index,
                    seed,
                    status: qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable,
                    fit_statistic: Some(value.fit_statistic),
                    label_alignment: Some(value.alignment),
                    target_payload_sha256: Some(
                        heterogeneity_target_payload_sha256_v2(&value.target_values).map_err(
                            |error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()),
                        )?,
                    ),
                    failure_reason: None,
                });
            }
            MultiModRefitOutcomeV1::Failed { failure, .. } => {
                let status = match failure.code.as_str() {
                    "label_ambiguous" => {
                        qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::LabelAmbiguous
                    }
                    "label_not_mutual_majority" => qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::LabelNotMutualMajority,
                    "nonfinite_target" | "target_identity" => qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::NonFiniteTarget,
                    "cancelled" => qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Cancelled,
                    _ => qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::FitFailed,
                };
                entries.push(HeterogeneityBootstrapLedgerEntryV2 {
                    replicate_index,
                    seed,
                    status,
                    fit_statistic: None,
                    label_alignment: None,
                    target_payload_sha256: None,
                    failure_reason: Some(failure.message),
                });
            }
        }
    }
    entries.sort_by_key(|entry| entry.replicate_index);
    let prepared = PreparedHeterogeneityBootstrapV2 {
        entries,
        targets,
        complete_stage_one_and_segmentation_rerun: true,
        pooled_common_metric_refit_repeated: reference.use_pooled_common_metric,
        exhaustive_label_alignment_applied: true,
    };
    let retained_k = prepared
        .ensure_valid()
        .map_err(MultiModRunnerErrorV1::InvalidLedger)?;
    if retained_k != usize::from(reference.k) {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "heterogeneity bootstrap retained K={retained_k}, expected K={}",
            reference.k
        )));
    }
    Ok(prepared)
}

/// Finalizes all exact shards through the global V1 ledger gate, then assembles
/// the existing public V2 result from retained point/common-metric evidence.
/// This function performs no FIMIX, PLS-POS, MICOM, or bootstrap estimator fit.
#[allow(clippy::too_many_arguments)]
pub fn finalize_prepared_raw_pls_heterogeneity_bootstrap_v2<P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    execution: &PreparedRawHeterogeneityBootstrapExecutionV2,
    shards: Vec<RawHeterogeneityBootstrapShardCacheV2>,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    let config = recipe.pls_heterogeneity.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "heterogeneity configuration disappeared after compilation".into(),
        )
    })?;
    let authority = projected_heterogeneity_authority_v2(dataset, recipe, model, artifact)?;
    let (complete_dataset, source_row_tokens) =
        complete_heterogeneity_dataset_v2(dataset, &authority.source_columns, &|| false)?;
    execution
        .ensure_valid(
            dataset,
            artifact,
            config,
            &source_row_tokens,
            &authority.plan.deterministic_sha256(),
        )
        .map_err(MultiModRunnerErrorV1::PreparedInput)?;
    let ledger = finalize_multimod_case_bootstrap_v1(
        &execution.reference.orchestrator_plan,
        complete_dataset.batch.num_rows(),
        None,
        shards,
    )
    .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))?;
    let prepared_bootstrap =
        prepared_heterogeneity_bootstrap_from_final_ledger_v2(&execution.reference, ledger)?;
    let mut prepared_point = execution.prepared_point.clone();
    prepared_point.bootstrap = Some(prepared_bootstrap);
    let mut output = assemble_heterogeneity_result_from_point_pass_v2(
        artifact,
        config,
        &prepared_point,
        &execution.point_pass,
        &progress,
    )?;
    output.evidence.insert(
        0,
        MultiModRunnerEvidenceV1::HeterogeneityRawPreparation(
            execution.raw_preparation_receipt.clone(),
        ),
    );
    if let Some(evidence) = &execution.common_metric_evidence {
        output
            .evidence
            .push(MultiModRunnerEvidenceV1::HeterogeneityPosCommonMetric(
                evidence.clone(),
            ));
    }
    Ok(output)
}

/// Executes the complete raw-data FIMIX/PLS-POS V2 point pipeline. The
/// lower-level prepared entry point remains available for independent oracle
/// adapters, while this path derives every score, product, signature, and
/// fixed-K resample from the compiled Recipe V4 authority itself.
fn run_compiled_raw_pls_heterogeneity_point_only_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    should_cancel: &C,
    progress: &P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    let config = recipe.pls_heterogeneity.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "heterogeneity configuration disappeared after compilation".into(),
        )
    })?;
    let predicted_sidecar_bytes = predict_heterogeneity_sidecar_bytes_v2(
        config,
        dataset.batch.num_rows(),
        multimod_model_target_upper_bound_v1(&[], model),
    );
    enforce_multimod_sidecar_cost_v1("heterogeneity", predicted_sidecar_bytes, progress)?;
    report(
        progress,
        MultiModRunnerPhaseV1::PreparingPointInputs,
        0,
        1,
        "heterogeneity:raw_projection",
    );
    let authority = projected_heterogeneity_authority_v2(dataset, recipe, model, artifact)?;
    let (complete_dataset, source_row_tokens) =
        complete_heterogeneity_dataset_v2(dataset, &authority.source_columns, should_cancel)?;
    let (prepared, pooled_fit, raw_scores, orientation_rows) =
        prepare_raw_heterogeneity_execution_v2(
            &complete_dataset,
            &authority,
            config.profile,
            should_cancel,
        )?;
    let mut refitter = RawPlsPosRefitterV2 {
        dataset: &complete_dataset,
        authority: &authority,
        profile: config.profile,
        pooled_fit: &pooled_fit,
        raw_scores: &raw_scores,
        orientation_rows: &orientation_rows,
        retain_outcome_audits: true,
        should_cancel,
    };
    let mut output = run_compiled_pls_heterogeneity_v2(
        dataset,
        recipe,
        model,
        artifact,
        &prepared,
        &mut refitter,
        should_cancel,
        progress,
    )?;
    let raw_receipt = RawHeterogeneityPreparationReceiptV2 {
        method_version: "qpls.heterogeneity.raw-preparation.v2".into(),
        general_sem_plan_sha256: authority.plan.deterministic_sha256(),
        pooled_metric_sha256: prepared.fimix_input.metric.source_sha256.clone(),
        omitted_source_rows: dataset
            .batch
            .num_rows()
            .saturating_sub(source_row_tokens.len()),
        source_row_tokens,
        unique_analysis_positions: true,
        fimix_input: prepared.fimix_input.clone(),
    };
    raw_receipt
        .ensure_valid()
        .map_err(MultiModRunnerErrorV1::ResultContract)?;
    output.evidence.insert(
        0,
        MultiModRunnerEvidenceV1::HeterogeneityRawPreparation(raw_receipt),
    );
    Ok(output)
}

pub fn run_compiled_raw_pls_heterogeneity_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    if recipe
        .pls_heterogeneity
        .as_ref()
        .is_some_and(|config| config.bootstrap.is_none())
    {
        return run_compiled_raw_pls_heterogeneity_point_only_v2(
            dataset,
            recipe,
            model,
            artifact,
            &should_cancel,
            &progress,
        );
    }
    let execution = prepare_compiled_raw_pls_heterogeneity_bootstrap_v2(
        dataset,
        recipe,
        model,
        artifact,
        &should_cancel,
        &progress,
    )?;
    let cache = run_prepared_raw_pls_heterogeneity_bootstrap_shard_v2(
        dataset,
        recipe,
        model,
        artifact,
        &execution,
        MultiModShardSpecV1 {
            shard_index: 0,
            shard_count: 1,
        },
        None,
        &should_cancel,
        &progress,
    );
    let cache = cache?;
    if cache.cancelled || should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    finalize_prepared_raw_pls_heterogeneity_bootstrap_v2(
        dataset,
        recipe,
        model,
        artifact,
        &execution,
        vec![cache],
        progress,
    )
}

#[derive(Debug, Default)]
pub struct NoopPlsPosRefitterV2;

impl PlsPosFullRefitterV2 for NoopPlsPosRefitterV2 {
    fn refit_segment(
        &mut self,
        _segment_index: usize,
        _row_indices: &[usize],
        _scoring: PosScoringContractV2,
    ) -> Result<qpls_estimation::PosSegmentFullFitV2, String> {
        Err("PLS-POS refitter was not configured".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedHeterogeneityBootstrapTargetV2 {
    pub target_id: String,
    pub estimates: Vec<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedHeterogeneityBootstrapV2 {
    pub entries: Vec<HeterogeneityBootstrapLedgerEntryV2>,
    pub targets: Vec<PreparedHeterogeneityBootstrapTargetV2>,
    pub complete_stage_one_and_segmentation_rerun: bool,
    pub pooled_common_metric_refit_repeated: bool,
    pub exhaustive_label_alignment_applied: bool,
}

impl PreparedHeterogeneityBootstrapV2 {
    /// Reconstructs target identities and exhaustive label claims from the
    /// retained ledger/target evidence. Returns the common retained K.
    pub fn ensure_valid(&self) -> Result<usize, String> {
        if !self.complete_stage_one_and_segmentation_rerun
            || !self.exhaustive_label_alignment_applied
            || self.entries.is_empty()
            || self.targets.is_empty()
        {
            return Err(
                "heterogeneity bootstrap omitted its full-pipeline, alignment, ledger, or target evidence"
                    .to_string(),
            );
        }
        let mut target_ids = BTreeSet::new();
        for target in &self.targets {
            if target.target_id.trim().is_empty()
                || !target_ids.insert(target.target_id.as_str())
                || target.estimates.len() != self.entries.len()
            {
                return Err(
                    "heterogeneity bootstrap target identities are duplicate/empty or do not share the ledger"
                        .to_string(),
                );
            }
        }
        let mut retained_k = None;
        let mut retained_observations = None;
        let mut usable = 0usize;
        for (position, entry) in self.entries.iter().enumerate() {
            if entry.replicate_index != position {
                return Err(
                    "heterogeneity bootstrap ledger is not in exact replicate-index order"
                        .to_string(),
                );
            }
            let values = self
                .targets
                .iter()
                .map(|target| target.estimates[position])
                .collect::<Vec<_>>();
            if entry.status == qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable {
                let finite_values = values
                    .iter()
                    .copied()
                    .collect::<Option<Vec<_>>>()
                    .filter(|values| values.iter().all(|value| value.is_finite()))
                    .ok_or_else(|| {
                        format!(
                            "usable heterogeneity replicate {position} lacks a complete finite target vector"
                        )
                    })?;
                let alignment = entry.label_alignment.as_ref().ok_or_else(|| {
                    format!("usable heterogeneity replicate {position} lacks label evidence")
                })?;
                let observations = validate_retained_label_alignment_v2(alignment)
                    .map_err(|error| error.to_string())?;
                let k = alignment.candidate_to_reference.len();
                if alignment.ambiguous || !alignment.mutual_majority {
                    return Err(format!(
                        "usable heterogeneity replicate {position} has an ineligible alignment"
                    ));
                }
                if retained_k.replace(k).is_some_and(|expected| expected != k)
                    || retained_observations
                        .replace(observations)
                        .is_some_and(|expected| expected != observations)
                {
                    return Err(
                        "heterogeneity bootstrap alignments disagree on K or observation count"
                            .to_string(),
                    );
                }
                let digest = heterogeneity_target_payload_sha256_v2(&finite_values)
                    .map_err(|error| error.to_string())?;
                if entry.fit_statistic.is_none_or(|value| !value.is_finite())
                    || entry.target_payload_sha256.as_deref() != Some(digest.as_str())
                    || entry.failure_reason.is_some()
                {
                    return Err(format!(
                        "usable heterogeneity replicate {position} has an invalid fit statistic, target digest, or failure field"
                    ));
                }
                usable += 1;
            } else if values.iter().any(Option::is_some)
                || entry.fit_statistic.is_some()
                || entry.label_alignment.is_some()
                || entry.target_payload_sha256.is_some()
                || entry
                    .failure_reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
            {
                return Err(format!(
                    "failed heterogeneity replicate {position} has incoherent retained evidence"
                ));
            }
        }
        if usable == 0 {
            return Err(
                "heterogeneity bootstrap contains no usable retained replicate".to_string(),
            );
        }
        retained_k.ok_or_else(|| "heterogeneity bootstrap retained no label K".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedFimixScientificTermV2 {
    pub coefficient_id: String,
    pub multiplier: f64,
    pub scale_divisor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedFimixScientificTargetV2 {
    pub equation_id: String,
    pub target_id: String,
    pub target_kind: String,
    pub primary_coefficient_target: bool,
    pub terms: Vec<PreparedFimixScientificTermV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedPosCommonMetricEvidenceV1 {
    pub method_version: String,
    pub gate_input: PosCommonMetricGateInputV1,
    pub gate_result: PosCommonMetricGateResultV1,
    pub micom_pairs: Vec<MicomPairwiseResultV1>,
    pub common_metric_parameters: Vec<HeterogeneityClassParameterV2>,
}

impl PreparedPosCommonMetricEvidenceV1 {
    /// Proves exact construct and pair coverage across the gate input, MICOM
    /// results, and pooled-metric parameter inventory.
    pub fn ensure_valid(&self) -> Result<(), String> {
        if self.method_version != "qpls.pos-common-metric.runner.v1"
            || !is_lower_hex_sha256_v1(&self.gate_input.pooled_metric_sha256)
            || !(2..=5).contains(&self.gate_input.segments)
            || self.gate_result != evaluate_pos_common_metric_gate_v1(&self.gate_input)
        {
            return Err(
                "POS common-metric evidence has an invalid method, metric, segment count, or derived gate result"
                    .to_string(),
            );
        }
        let segments = self.gate_input.segments;
        let required = self
            .gate_input
            .required_construct_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let result_required = self
            .gate_result
            .required_construct_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let evidence_constructs = self
            .gate_input
            .evidence
            .iter()
            .map(|row| row.construct_id.as_str())
            .collect::<BTreeSet<_>>();
        if required.is_empty()
            || required.len() != self.gate_input.required_construct_ids.len()
            || result_required != required
            || self.gate_result.required_construct_ids.len() != required.len()
            || evidence_constructs != required
            || self.gate_input.evidence.len() != required.len()
        {
            return Err(
                "POS common-metric required construct inventory is not exact and unique"
                    .to_string(),
            );
        }
        let expected_pairs = (0..segments)
            .flat_map(|left| (left + 1..segments).map(move |right| (left, right)))
            .collect::<BTreeSet<_>>();
        for evidence in &self.gate_input.evidence {
            let compositional_pairs = evidence
                .compositional_invariance
                .iter()
                .map(|pair| (pair.left_segment, pair.right_segment))
                .collect::<BTreeSet<_>>();
            let step3_pairs = evidence
                .step3_equality
                .iter()
                .map(|pair| (pair.left_segment, pair.right_segment))
                .collect::<BTreeSet<_>>();
            if compositional_pairs != expected_pairs
                || compositional_pairs.len() != evidence.compositional_invariance.len()
                || step3_pairs != expected_pairs
                || step3_pairs.len() != evidence.step3_equality.len()
                || evidence.compositional_invariance.iter().any(|pair| {
                    pair.permutation_p_value
                        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
                })
            {
                return Err(format!(
                    "POS common-metric construct {} lacks the exact compositional/Step-3 pair inventory",
                    evidence.construct_id
                ));
            }
        }
        let micom_pairs = self
            .micom_pairs
            .iter()
            .map(|pair| (pair.pair.group_a.get(), pair.pair.group_b.get()))
            .collect::<BTreeSet<_>>();
        if micom_pairs != expected_pairs || micom_pairs.len() != self.micom_pairs.len() {
            return Err("POS common-metric MICOM pair inventory is not exact".to_string());
        }
        for pair in &self.micom_pairs {
            let pair_id = (pair.pair.group_a.get(), pair.pair.group_b.get());
            let constructs = pair
                .constructs
                .iter()
                .map(|construct| construct.construct_id.as_str())
                .collect::<BTreeSet<_>>();
            if constructs != required || constructs.len() != pair.constructs.len() {
                return Err(format!(
                    "POS common-metric MICOM pair {:?} lacks the exact construct inventory",
                    pair_id
                ));
            }
            for construct in &pair.constructs {
                let evidence = self
                    .gate_input
                    .evidence
                    .iter()
                    .find(|row| row.construct_id == construct.construct_id)
                    .expect("construct inventories checked above");
                let compositional = evidence
                    .compositional_invariance
                    .iter()
                    .find(|row| (row.left_segment, row.right_segment) == pair_id)
                    .expect("pair inventories checked above");
                let step3 = evidence
                    .step3_equality
                    .iter()
                    .find(|row| (row.left_segment, row.right_segment) == pair_id)
                    .expect("pair inventories checked above");
                if evidence.configural_identity_passed != pair.configural_receipt.complete()
                    || compositional.passed != (pair.complete && construct.compositional_invariance)
                    || compositional.permutation_p_value
                        != construct.compositional_invariance_probability
                    || step3.mean_equality_passed != (pair.complete && construct.equal_means)
                    || step3.variance_equality_passed
                        != (pair.complete && construct.equal_variances)
                {
                    return Err(format!(
                        "POS common-metric construct {} pair {:?} disagrees with retained MICOM",
                        construct.construct_id, pair_id
                    ));
                }
            }
        }
        let mut suffixes_by_segment = BTreeMap::<u8, BTreeSet<String>>::new();
        let expected_metric = format!(
            "qpls.pos.pooled-common-metric.v1:{}",
            self.gate_input.pooled_metric_sha256
        );
        for row in &self.common_metric_parameters {
            let prefix = format!("class_{}:", row.class_id);
            let suffix = row
                .parameter
                .target_id
                .strip_prefix(&prefix)
                .filter(|suffix| !suffix.trim().is_empty())
                .ok_or_else(|| {
                    format!(
                        "POS common-metric parameter {} has the wrong segment identity",
                        row.parameter.target_id
                    )
                })?;
            if row.class_id == 0
                || usize::from(row.class_id) > segments
                || row.metric != expected_metric
                || row.parameter.target_kind.trim().is_empty()
                || !row.parameter.estimate.is_finite()
                || row
                    .parameter
                    .standard_error
                    .is_some_and(|value| !value.is_finite() || value < 0.0)
                || row
                    .parameter
                    .p_value
                    .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
                || row.parameter.interval.as_ref().is_some_and(|interval| {
                    interval.family.trim().is_empty()
                        || !interval.confidence_level.is_finite()
                        || !(0.0..1.0).contains(&interval.confidence_level)
                        || interval.lower.is_some_and(|value| !value.is_finite())
                        || interval.upper.is_some_and(|value| !value.is_finite())
                })
                || !suffixes_by_segment
                    .entry(row.class_id)
                    .or_default()
                    .insert(suffix.to_string())
            {
                return Err(
                    "POS common-metric parameters have invalid identities, metrics, or values"
                        .to_string(),
                );
            }
        }
        let expected_segments = (1..=segments as u8).collect::<BTreeSet<_>>();
        if suffixes_by_segment.keys().copied().collect::<BTreeSet<_>>() != expected_segments
            || suffixes_by_segment.values().next().is_none_or(|expected| {
                expected.is_empty()
                    || suffixes_by_segment
                        .values()
                        .any(|observed| observed != expected)
            })
        {
            return Err(
                "POS common-metric parameter target inventory differs across segments".to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedHeterogeneityExecutionV2 {
    pub fimix_input: StandardizedFimixInputV2,
    #[serde(default)]
    pub fimix_scientific_targets: Vec<PreparedFimixScientificTargetV2>,
    #[serde(default)]
    pub pos_start_features: Vec<Vec<f64>>,
    #[serde(default)]
    pub pos_parameter_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos_common_metric_gate: Option<PosCommonMetricGateInputV1>,
    #[serde(default)]
    pub pos_common_metric_parameters: Vec<HeterogeneityClassParameterV2>,
    #[serde(default)]
    pub pos_common_metric_contrasts: Vec<HeterogeneityClassContrastV2>,
    #[serde(default)]
    pub pos_common_metric_micom_pairs: Vec<MicomPairwiseResultV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<PreparedHeterogeneityBootstrapV2>,
}

struct ControlledPosRefitter<'a, R, C, P> {
    inner: &'a mut R,
    should_cancel: &'a C,
    progress: &'a P,
    calls: u64,
    cancelled: bool,
}

impl<R, C, P> PlsPosFullRefitterV2 for ControlledPosRefitter<'_, R, C, P>
where
    R: PlsPosFullRefitterV2,
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    fn refit_segment(
        &mut self,
        segment_index: usize,
        row_indices: &[usize],
        scoring: PosScoringContractV2,
    ) -> Result<qpls_estimation::PosSegmentFullFitV2, String> {
        if (self.should_cancel)() {
            self.cancelled = true;
            return Err("multimod.runner.cancelled".into());
        }
        report(
            self.progress,
            MultiModRunnerPhaseV1::PointEstimation,
            self.calls,
            self.calls.saturating_add(1),
            format!("pls_pos:segment:{segment_index}"),
        );
        self.calls = self.calls.saturating_add(1);
        self.inner
            .refit_segment(segment_index, row_indices, scoring)
    }

    fn refit_segments_batch(
        &mut self,
        requests: &[PosSegmentRefitRequestV2],
    ) -> Vec<Result<PosSegmentFullFitV2, String>> {
        if (self.should_cancel)() {
            self.cancelled = true;
            return requests
                .iter()
                .map(|_| Err("multimod.runner.cancelled".into()))
                .collect();
        }
        if !requests.is_empty() {
            report(
                self.progress,
                MultiModRunnerPhaseV1::PointEstimation,
                self.calls,
                self.calls.saturating_add(requests.len() as u64),
                format!("pls_pos:ordered_refit_batch:{}", requests.len()),
            );
            self.calls = self.calls.saturating_add(requests.len() as u64);
        }
        let results = self.inner.refit_segments_batch(requests);
        if results.iter().any(|result| {
            result
                .as_ref()
                .err()
                .is_some_and(|reason| reason == "multimod.runner.cancelled")
        }) {
            self.cancelled = true;
        }
        results
    }
}

fn estimation_heterogeneity_profile(
    profile: CoreHeterogeneityProfileV2,
) -> EstimationHeterogeneityProfileV2 {
    match profile {
        CoreHeterogeneityProfileV2::P0Structural => EstimationHeterogeneityProfileV2::P0Structural,
        CoreHeterogeneityProfileV2::P2MultiTwoWay => {
            EstimationHeterogeneityProfileV2::P2MultiTwoWay
        }
        CoreHeterogeneityProfileV2::P23AllCurrent => {
            EstimationHeterogeneityProfileV2::P23AllCurrent
        }
    }
}

fn heterogeneity_bootstrap_algorithm(
    algorithm: CoreHeterogeneityAlgorithmV2,
) -> HeterogeneityBootstrapAlgorithmV2 {
    match algorithm {
        CoreHeterogeneityAlgorithmV2::FimixPlsV2 => HeterogeneityBootstrapAlgorithmV2::FimixPlsV2,
        CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2 => {
            HeterogeneityBootstrapAlgorithmV2::PlsPosPublishedV2
        }
        CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
            HeterogeneityBootstrapAlgorithmV2::PlsPosDestinationScoredInteractionsV2
        }
    }
}

fn fimix_config(
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    k: usize,
) -> FimixPlsV2Config {
    let mut value = FimixPlsV2Config::for_classes(k);
    value.starts = config.fimix.starts as usize;
    value.seed = config.seed;
    value.max_iterations = config.fimix.max_iterations as usize;
    value.relative_log_likelihood_tolerance = config.fimix.relative_log_likelihood_tolerance;
    value.consecutive_stable_iterations = config.fimix.consecutive_converged_iterations as usize;
    value.likelihood_decrease_tolerance = config.fimix.likelihood_decrease_tolerance;
    value.residual_variance_floor = config.fimix.residual_variance_floor;
    value.rank_tolerance = config.fimix.rank_tolerance;
    value.minimum_class_share = config.fimix.minimum_class_share;
    value.required_reproducing_starts = config.fimix.required_reproducing_starts as usize;
    value.optimum_relative_log_likelihood_tolerance =
        config.fimix.optimum_relative_log_likelihood_tolerance;
    value.optimum_maximum_coefficient_difference =
        config.fimix.optimum_maximum_coefficient_difference;
    value.optimum_mean_posterior_difference = config.fimix.optimum_mean_posterior_difference;
    value
}

fn pos_config(
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    k: usize,
    observations: usize,
) -> PlsPosV2Config {
    let mut value = PlsPosV2Config::for_segments(k, observations);
    value.strict_improvement_tolerance = config.pls_pos.strict_improvement_tolerance;
    value.stable_objective_tolerance = config.pls_pos.stable_objective_tolerance;
    value.required_reproducing_starts = config.pls_pos.minimum_reproducing_starts as usize;
    value
}

fn fimix_candidate(
    algorithm: CoreHeterogeneityAlgorithmV2,
    k: u8,
    result: &FimixPlsV2Result,
) -> HeterogeneityCandidateV2 {
    let converged_starts = result.starts.iter().filter(|start| start.converged).count();
    HeterogeneityCandidateV2 {
        method: HeterogeneityCandidateMethodV2::Segmentation { algorithm },
        k,
        state: if result.stability.stable {
            HeterogeneityCandidateStateV2::ConvergedStable
        } else {
            HeterogeneityCandidateStateV2::Unstable
        },
        converged_starts: converged_starts as u32,
        stable_starts: result.stability.reproducing_start_indices.len() as u32,
        log_likelihood: Some(result.log_likelihood),
        objective: None,
        criteria: BTreeMap::from([
            ("aic".into(), result.criteria.aic),
            ("aic3".into(), result.criteria.aic3),
            ("aic4".into(), result.criteria.aic4),
            ("bic".into(), result.criteria.bic),
            ("caic".into(), result.criteria.caic),
            ("hq".into(), result.criteria.hq),
            ("entropy".into(), result.entropy.raw),
            (
                "entropy_normalized_certainty".into(),
                result.entropy.normalized_certainty,
            ),
        ]),
        class_or_segment_shares: result
            .classes
            .iter()
            .map(|class| class.proportion)
            .collect(),
        pooled_parameters: Vec::new(),
        blockers: Vec::new(),
    }
}

fn pos_candidate(
    algorithm: CoreHeterogeneityAlgorithmV2,
    k: u8,
    result: &PlsPosV2Result,
) -> HeterogeneityCandidateV2 {
    HeterogeneityCandidateV2 {
        method: HeterogeneityCandidateMethodV2::Segmentation { algorithm },
        k,
        state: if result.reproducing_start_indices.len() >= 2 {
            HeterogeneityCandidateStateV2::ConvergedStable
        } else {
            HeterogeneityCandidateStateV2::Unstable
        },
        converged_starts: result.starts.iter().filter(|start| start.completed).count() as u32,
        stable_starts: result.reproducing_start_indices.len() as u32,
        log_likelihood: None,
        objective: Some(result.objective),
        criteria: BTreeMap::new(),
        class_or_segment_shares: result
            .segments
            .iter()
            .map(|segment| segment.observations as f64 / result.observations as f64)
            .collect(),
        pooled_parameters: Vec::new(),
        blockers: Vec::new(),
    }
}

fn candidate_from_heterogeneity_error(
    algorithm: CoreHeterogeneityAlgorithmV2,
    k: u8,
    error: &HeterogeneityV2Error,
) -> HeterogeneityCandidateV2 {
    let (state, converged_starts, stable_starts) = match error {
        HeterogeneityV2Error::UnstableFimixOptimum {
            reproducing_starts,
            diagnostics,
            ..
        } => (
            HeterogeneityCandidateStateV2::Unstable,
            diagnostics.iter().filter(|start| start.converged).count() as u32,
            *reproducing_starts as u32,
        ),
        HeterogeneityV2Error::UnstablePosOptimum {
            reproducing_starts,
            diagnostics,
            ..
        } => (
            HeterogeneityCandidateStateV2::Unstable,
            diagnostics.iter().filter(|start| start.completed).count() as u32,
            *reproducing_starts as u32,
        ),
        _ => (HeterogeneityCandidateStateV2::Failed, 0, 0),
    };
    HeterogeneityCandidateV2 {
        method: HeterogeneityCandidateMethodV2::Segmentation { algorithm },
        k,
        state,
        converged_starts,
        stable_starts,
        log_likelihood: None,
        objective: None,
        criteria: BTreeMap::new(),
        class_or_segment_shares: Vec::new(),
        pooled_parameters: Vec::new(),
        blockers: vec![error.to_string()],
    }
}

fn pooled_baseline_candidate_v2(
    baseline: &PooledStructuralBaselineV2,
    scientific_targets: &[PreparedFimixScientificTargetV2],
) -> Result<HeterogeneityCandidateV2, MultiModRunnerErrorV1> {
    let equation_coefficients = baseline
        .equations
        .iter()
        .map(|equation| {
            (
                equation.equation_id.as_str(),
                equation
                    .coefficients
                    .iter()
                    .map(|coefficient| (coefficient.parameter_id.as_str(), coefficient.estimate))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut pooled_parameters = Vec::with_capacity(
        scientific_targets
            .len()
            .saturating_add(baseline.equations.len()),
    );
    for target in scientific_targets {
        let coefficients = equation_coefficients
            .get(target.equation_id.as_str())
            .ok_or_else(|| {
                MultiModRunnerErrorV1::ResultContract(format!(
                    "pooled baseline omitted equation {}",
                    target.equation_id
                ))
            })?;
        let estimate = target
            .terms
            .iter()
            .map(|term| {
                coefficients
                    .get(term.coefficient_id.as_str())
                    .copied()
                    .ok_or_else(|| {
                        MultiModRunnerErrorV1::ResultContract(format!(
                            "pooled baseline equation {} omitted coefficient {}",
                            target.equation_id, term.coefficient_id
                        ))
                    })
                    .map(|coefficient| coefficient * term.multiplier / term.scale_divisor)
            })
            .sum::<Result<f64, _>>()?;
        if !estimate.is_finite() {
            return Err(MultiModRunnerErrorV1::ResultContract(format!(
                "pooled baseline target {} is nonfinite",
                target.target_id
            )));
        }
        pooled_parameters.push(MultimodParameterEstimateV1 {
            target_id: target.target_id.clone(),
            target_kind: target
                .target_kind
                .strip_prefix("class_specific_")
                .map(|suffix| format!("pooled_{suffix}"))
                .unwrap_or_else(|| format!("pooled_{}", target.target_kind)),
            estimate,
            standard_error: None,
            p_value: None,
            interval: None,
        });
    }
    for equation in &baseline.equations {
        pooled_parameters.push(MultimodParameterEstimateV1 {
            target_id: format!("pooled_residual_variance:{}", equation.outcome_id),
            target_kind: "pooled_residual_variance".into(),
            estimate: equation.residual_variance,
            standard_error: None,
            p_value: None,
            interval: None,
        });
    }
    pooled_parameters.sort_by(|left, right| left.target_id.cmp(&right.target_id));
    if pooled_parameters
        .windows(2)
        .any(|pair| pair[0].target_id == pair[1].target_id)
    {
        return Err(MultiModRunnerErrorV1::ResultContract(
            "pooled baseline target identities are duplicated".into(),
        ));
    }
    let mut criteria = BTreeMap::from([("observation_count".into(), baseline.observations as f64)]);
    for equation in &baseline.equations {
        criteria.insert(
            format!("r_squared:{}", equation.outcome_id),
            equation.r_squared,
        );
        criteria.insert(
            format!("residual_variance:{}", equation.outcome_id),
            equation.residual_variance,
        );
    }
    Ok(HeterogeneityCandidateV2 {
        method: HeterogeneityCandidateMethodV2::PooledBaselineV1,
        k: 1,
        state: HeterogeneityCandidateStateV2::Eligible,
        converged_starts: 0,
        stable_starts: 0,
        log_likelihood: None,
        objective: None,
        criteria,
        class_or_segment_shares: Vec::new(),
        pooled_parameters,
        blockers: Vec::new(),
    })
}

fn heterogeneity_discovery_result_identity_v2(
    receipt: &MultiModCompilationReceiptV1,
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    candidates: &[HeterogeneityCandidateV2],
) -> String {
    sha256_serialized(&(
        "qpls.heterogeneity.discovery-result-identity.v2",
        receipt.model_scientific_sha256.as_str(),
        receipt.dataset_fingerprint.as_str(),
        config.profile,
        config.seed,
        &config.fimix,
        &config.pls_pos,
        candidates,
    ))
}

fn common_metric_segment_parameters_v2(
    baselines: &[PooledStructuralBaselineV2],
    scientific_targets: &[PreparedFimixScientificTargetV2],
) -> Result<Vec<HeterogeneityClassParameterV2>, MultiModRunnerErrorV1> {
    let mut rows = Vec::with_capacity(baselines.len().saturating_mul(scientific_targets.len()));
    for (segment, baseline) in baselines.iter().enumerate() {
        let equations = baseline
            .equations
            .iter()
            .map(|equation| {
                (
                    equation.equation_id.as_str(),
                    equation
                        .coefficients
                        .iter()
                        .map(|coefficient| {
                            (coefficient.parameter_id.as_str(), coefficient.estimate)
                        })
                        .collect::<BTreeMap<_, _>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for target in scientific_targets {
            let coefficients = equations.get(target.equation_id.as_str()).ok_or_else(|| {
                MultiModRunnerErrorV1::ResultContract(format!(
                    "pooled common metric omitted equation {}",
                    target.equation_id
                ))
            })?;
            let estimate = target
                .terms
                .iter()
                .map(|term| {
                    coefficients
                        .get(term.coefficient_id.as_str())
                        .copied()
                        .ok_or_else(|| {
                            MultiModRunnerErrorV1::ResultContract(format!(
                                "pooled common metric equation {} omitted coefficient {}",
                                target.equation_id, term.coefficient_id
                            ))
                        })
                        .map(|value| value * term.multiplier / term.scale_divisor)
                })
                .sum::<Result<f64, _>>()?;
            if !estimate.is_finite() {
                return Err(MultiModRunnerErrorV1::ResultContract(format!(
                    "pooled common-metric target {} is nonfinite",
                    target.target_id
                )));
            }
            rows.push(HeterogeneityClassParameterV2 {
                class_id: (segment + 1) as u8,
                parameter: MultimodParameterEstimateV1 {
                    target_id: format!("class_{}:{}", segment + 1, target.target_id),
                    target_kind: target.target_kind.clone(),
                    estimate,
                    standard_error: None,
                    p_value: None,
                    interval: None,
                },
                metric: format!(
                    "qpls.pos.pooled-common-metric.v1:{}",
                    baseline.metric_source_sha256
                ),
            });
        }
    }
    rows.sort_by(|left, right| {
        left.class_id
            .cmp(&right.class_id)
            .then(left.parameter.target_id.cmp(&right.parameter.target_id))
    });
    if rows.windows(2).any(|pair| {
        pair[0].class_id == pair[1].class_id
            && pair[0].parameter.target_id == pair[1].parameter.target_id
    }) {
        return Err(MultiModRunnerErrorV1::ResultContract(
            "pooled common-metric target identities are duplicated".into(),
        ));
    }
    Ok(rows)
}

fn common_metric_point_contrasts_v2(
    parameters: &[HeterogeneityClassParameterV2],
    segments: u8,
) -> Result<Vec<HeterogeneityClassContrastV2>, MultiModRunnerErrorV1> {
    let mut by_key = BTreeMap::<(u8, String), f64>::new();
    for row in parameters {
        let suffix = target_suffix_v2(&row.parameter.target_id).ok_or_else(|| {
            MultiModRunnerErrorV1::ResultContract(format!(
                "common-metric target {} has no class prefix",
                row.parameter.target_id
            ))
        })?;
        if row.class_id == 0
            || row.class_id > segments
            || by_key
                .insert((row.class_id, suffix.to_owned()), row.parameter.estimate)
                .is_some()
        {
            return Err(MultiModRunnerErrorV1::ResultContract(
                "common-metric class/target inventory is invalid or duplicated".into(),
            ));
        }
    }
    let target_ids = by_key
        .keys()
        .map(|(_, target)| target.clone())
        .collect::<BTreeSet<_>>();
    if target_ids.is_empty()
        || (1..=segments).any(|class| {
            target_ids
                .iter()
                .any(|target| !by_key.contains_key(&(class, target.clone())))
        })
    {
        return Err(MultiModRunnerErrorV1::ResultContract(
            "common-metric target inventory is incomplete across segments".into(),
        ));
    }
    let mut contrasts = Vec::new();
    for left in 1..=segments {
        for right in left + 1..=segments {
            for target_id in &target_ids {
                contrasts.push(HeterogeneityClassContrastV2 {
                    left_class_id: left,
                    right_class_id: right,
                    target_id: target_id.clone(),
                    difference: by_key[&(left, target_id.clone())]
                        - by_key[&(right, target_id.clone())],
                    p_value: None,
                    interval: None,
                    common_metric_comparability_satisfied: true,
                    inferential_interpretation_blocked: false,
                });
            }
        }
    }
    Ok(contrasts)
}

fn fimix_parameters(
    result: &FimixPlsV2Result,
    scientific_targets: &[PreparedFimixScientificTargetV2],
) -> Result<Vec<HeterogeneityClassParameterV2>, MultiModRunnerErrorV1> {
    validate_fimix_scientific_targets_v2(result, scientific_targets)?;
    let mut parameters = Vec::new();
    for (class_index, class) in result.classes.iter().enumerate() {
        for equation in &class.equations {
            let by_id = equation
                .coefficients
                .iter()
                .map(|coefficient| (coefficient.parameter_id.as_str(), coefficient.estimate))
                .collect::<BTreeMap<_, _>>();
            if scientific_targets.is_empty() {
                for coefficient in &equation.coefficients {
                    parameters.push(HeterogeneityClassParameterV2 {
                        class_id: (class_index + 1) as u8,
                        parameter: MultimodParameterEstimateV1 {
                            target_id: format!(
                                "class_{}:{}:{}",
                                class_index + 1,
                                equation.equation_id,
                                coefficient.parameter_id
                            ),
                            target_kind: if coefficient.parameter_id == "(intercept)" {
                                "class_specific_intercept".into()
                            } else {
                                "class_specific_standardized_predictor_coefficient".into()
                            },
                            estimate: coefficient.estimate,
                            standard_error: None,
                            p_value: None,
                            interval: None,
                        },
                        metric: "pooled_globally_standardized_metric".into(),
                    });
                }
            } else {
                if let Some(intercept) = by_id.get("(intercept)") {
                    parameters.push(HeterogeneityClassParameterV2 {
                        class_id: (class_index + 1) as u8,
                        parameter: MultimodParameterEstimateV1 {
                            target_id: format!(
                                "class_{}:{}:(intercept)",
                                class_index + 1,
                                equation.equation_id
                            ),
                            target_kind: "class_specific_intercept".into(),
                            estimate: *intercept,
                            standard_error: None,
                            p_value: None,
                            interval: None,
                        },
                        metric: "pooled_globally_standardized_metric".into(),
                    });
                }
                for target in scientific_targets
                    .iter()
                    .filter(|target| target.equation_id == equation.equation_id)
                {
                    let estimate = target
                        .terms
                        .iter()
                        .map(|term| {
                            by_id
                                .get(term.coefficient_id.as_str())
                                .copied()
                                .map(|value| value * term.multiplier / term.scale_divisor)
                                .ok_or_else(|| {
                                    MultiModRunnerErrorV1::PreparedInput(format!(
                                        "FIMIX scientific target {} references missing coefficient {}",
                                        target.target_id, term.coefficient_id
                                    ))
                                })
                        })
                        .sum::<Result<f64, _>>()?;
                    parameters.push(HeterogeneityClassParameterV2 {
                        class_id: (class_index + 1) as u8,
                        parameter: MultimodParameterEstimateV1 {
                            target_id: format!(
                                "class_{}:{}:{}",
                                class_index + 1,
                                equation.equation_id,
                                target.target_id
                            ),
                            target_kind: target.target_kind.clone(),
                            estimate,
                            standard_error: None,
                            p_value: None,
                            interval: None,
                        },
                        metric: "pooled_globally_standardized_scientific_metric".into(),
                    });
                }
            }
            parameters.push(HeterogeneityClassParameterV2 {
                class_id: (class_index + 1) as u8,
                parameter: MultimodParameterEstimateV1 {
                    target_id: format!(
                        "class_{}:{}:residual_variance",
                        class_index + 1,
                        equation.equation_id
                    ),
                    target_kind: "class_specific_residual_variance".into(),
                    estimate: equation.residual_variance,
                    standard_error: None,
                    p_value: None,
                    interval: None,
                },
                metric: "pooled_globally_standardized_metric".into(),
            });
        }
    }
    Ok(parameters)
}

fn validate_fimix_scientific_targets_v2(
    result: &FimixPlsV2Result,
    targets: &[PreparedFimixScientificTargetV2],
) -> Result<(), MultiModRunnerErrorV1> {
    if targets.is_empty() {
        return Ok(());
    }
    let equations = result
        .classes
        .first()
        .map(|class| {
            class
                .equations
                .iter()
                .map(|equation| {
                    (
                        equation.equation_id.as_str(),
                        equation
                            .coefficients
                            .iter()
                            .filter(|coefficient| coefficient.parameter_id != "(intercept)")
                            .map(|coefficient| coefficient.parameter_id.as_str())
                            .collect::<BTreeSet<_>>(),
                    )
                })
                .collect::<BTreeMap<_, _>>()
        })
        .ok_or_else(|| MultiModRunnerErrorV1::Kernel("FIMIX returned no classes".into()))?;
    let mut target_keys = BTreeSet::new();
    let mut primary = BTreeSet::new();
    for target in targets {
        let coefficients = equations.get(target.equation_id.as_str()).ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "FIMIX scientific target {} references unknown equation {}",
                target.target_id, target.equation_id
            ))
        })?;
        if target.target_id.trim().is_empty()
            || target.target_kind.trim().is_empty()
            || target.terms.is_empty()
            || !target_keys.insert((target.equation_id.as_str(), target.target_id.as_str()))
            || target.terms.iter().any(|term| {
                term.coefficient_id.trim().is_empty()
                    || !term.multiplier.is_finite()
                    || !term.scale_divisor.is_finite()
                    || term.scale_divisor <= 0.0
                    || !coefficients.contains(term.coefficient_id.as_str())
            })
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "FIMIX scientific target projection is incomplete, duplicated, or nonfinite".into(),
            ));
        }
        if target.primary_coefficient_target {
            if target.terms.len() != 1
                || target.terms[0].multiplier.to_bits() != 1.0f64.to_bits()
                || !primary.insert((
                    target.equation_id.as_str(),
                    target.terms[0].coefficient_id.as_str(),
                ))
            {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "each primary FIMIX coefficient projection must be a unique one-term target"
                        .into(),
                ));
            }
        }
    }
    let expected_primary = equations
        .iter()
        .flat_map(|(equation, coefficients)| {
            coefficients
                .iter()
                .map(move |coefficient| (*equation, *coefficient))
        })
        .collect::<BTreeSet<_>>();
    if primary != expected_primary {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "FIMIX scientific projections do not cover every fitted non-intercept coefficient exactly once"
                .into(),
        ));
    }
    Ok(())
}

fn pos_parameters(
    result: &PlsPosV2Result,
    parameter_ids: &[String],
) -> Result<Vec<HeterogeneityClassParameterV2>, MultiModRunnerErrorV1> {
    if parameter_ids.is_empty()
        || parameter_ids.iter().collect::<BTreeSet<_>>().len() != parameter_ids.len()
        || result
            .segments
            .iter()
            .any(|segment| segment.fit.parameter_signature.len() != parameter_ids.len())
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "PLS-POS parameter identities must uniquely cover the full-refit signature".into(),
        ));
    }
    Ok(result
        .segments
        .iter()
        .enumerate()
        .flat_map(|(segment_index, segment)| {
            parameter_ids
                .iter()
                .zip(&segment.fit.parameter_signature)
                .map(
                    move |(parameter_id, estimate)| HeterogeneityClassParameterV2 {
                        class_id: (segment_index + 1) as u8,
                        parameter: MultimodParameterEstimateV1 {
                            target_id: format!("segment_{}:{parameter_id}", segment_index + 1),
                            target_kind: "segment_local_structural_parameter".into(),
                            estimate: *estimate,
                            standard_error: None,
                            p_value: None,
                            interval: None,
                        },
                        metric: "destination_local_descriptive_metric".into(),
                    },
                )
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedFimixPointCandidateV2 {
    pub k: u8,
    pub result: FimixPlsV2Result,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedPosPointCandidateV2 {
    pub algorithm: CoreHeterogeneityAlgorithmV2,
    pub k: u8,
    pub result: PlsPosV2Result,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PreparedHeterogeneityPointEvidenceOrderV2 {
    Fimix {
        k: u8,
    },
    PlsPos {
        algorithm: CoreHeterogeneityAlgorithmV2,
        k: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedLockedHeterogeneityPointV2 {
    pub algorithm: CoreHeterogeneityAlgorithmV2,
    pub k: u8,
    pub assignments: Vec<usize>,
    pub fit_statistic: f64,
    pub local_parameters: Vec<HeterogeneityClassParameterV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedHeterogeneityPointPassV2 {
    pub method_version: String,
    pub candidates: Vec<HeterogeneityCandidateV2>,
    pub discovery_result_identity_sha256: String,
    pub pooled_baseline: PooledStructuralBaselineV2,
    pub fimix_candidates: Vec<PreparedFimixPointCandidateV2>,
    pub pos_candidates: Vec<PreparedPosPointCandidateV2>,
    pub evidence_order: Vec<PreparedHeterogeneityPointEvidenceOrderV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked: Option<PreparedLockedHeterogeneityPointV2>,
    pub point_pass_identity_sha256: String,
}

fn run_pos_candidate<R: PlsPosFullRefitterV2>(
    algorithm: CoreHeterogeneityAlgorithmV2,
    profile: CoreHeterogeneityProfileV2,
    k: usize,
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    prepared: &PreparedHeterogeneityExecutionV2,
    scientific_row_features: &[Vec<f64>],
    same_k_fimix: Option<&[usize]>,
    refitter: &mut R,
) -> Result<PlsPosV2Result, HeterogeneityV2Error> {
    let starts =
        build_pls_pos_start_plan_v2(&prepared.pos_start_features, k, config.seed, same_k_fimix)?;
    let settings = pos_config(config, k, prepared.pos_start_features.len());
    let result = match algorithm {
        CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2 => {
            fit_pls_pos_published_with_scientific_row_features_v2(
                &starts,
                scientific_row_features,
                &settings,
                refitter,
            )
        }
        CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
            fit_pls_pos_destination_scored_interactions_with_scientific_row_features_v2(
                &starts,
                scientific_row_features,
                estimation_heterogeneity_profile(profile),
                &settings,
                refitter,
            )
        }
        CoreHeterogeneityAlgorithmV2::FimixPlsV2 => Err(HeterogeneityV2Error::InvalidContract(
            "FIMIX was routed to the PLS-POS adapter".into(),
        )),
    }?;
    validate_pos_multistart_evidence_v2(&result)?;
    Ok(result)
}

fn heterogeneity_point_pass_identity_v2(point: &PreparedHeterogeneityPointPassV2) -> String {
    sha256_serialized(&(
        point.method_version.as_str(),
        point.candidates.as_slice(),
        point.discovery_result_identity_sha256.as_str(),
        &point.pooled_baseline,
        point.fimix_candidates.as_slice(),
        point.pos_candidates.as_slice(),
        point.evidence_order.as_slice(),
        point.locked.as_ref(),
    ))
}

impl PreparedHeterogeneityPointPassV2 {
    fn evidence(&self) -> Vec<MultiModRunnerEvidenceV1> {
        let mut evidence = vec![MultiModRunnerEvidenceV1::HeterogeneityPooledBaseline(
            self.pooled_baseline.clone(),
        )];
        evidence.extend(self.evidence_order.iter().map(|identity| match identity {
            PreparedHeterogeneityPointEvidenceOrderV2::Fimix { k } => {
                let candidate = self
                    .fimix_candidates
                    .iter()
                    .find(|candidate| candidate.k == *k)
                    .expect("validated point evidence order");
                MultiModRunnerEvidenceV1::FimixCandidate {
                    k: *k,
                    result: candidate.result.clone(),
                }
            }
            PreparedHeterogeneityPointEvidenceOrderV2::PlsPos { algorithm, k } => {
                let candidate = self
                    .pos_candidates
                    .iter()
                    .find(|candidate| candidate.algorithm == *algorithm && candidate.k == *k)
                    .expect("validated point evidence order");
                MultiModRunnerEvidenceV1::PlsPosCandidate {
                    k: *k,
                    result: candidate.result.clone(),
                }
            }
        }));
        evidence
    }

    pub fn ensure_valid(
        &self,
        artifact: &CompiledMultiModRecipeV1,
        config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
        prepared: &PreparedHeterogeneityExecutionV2,
    ) -> Result<(), String> {
        if self.method_version != "qpls.heterogeneity.point-pass.v2"
            || !is_lower_hex_sha256_v1(&self.discovery_result_identity_sha256)
            || !is_lower_hex_sha256_v1(&self.point_pass_identity_sha256)
            || self.point_pass_identity_sha256 != heterogeneity_point_pass_identity_v2(self)
            || self.candidates.is_empty()
        {
            return Err("heterogeneity point pass has an invalid method or identity".into());
        }
        let CompiledMultiModPlanV1::PlsHeterogeneityV2 {
            algorithms,
            candidate_k,
            ..
        } = artifact.plan()
        else {
            return Err("heterogeneity point pass requires a heterogeneity artifact".into());
        };
        if self.candidates.len() != 1 + algorithms.len() * candidate_k.len()
            || self.discovery_result_identity_sha256
                != heterogeneity_discovery_result_identity_v2(
                    artifact.receipt(),
                    config,
                    &self.candidates,
                )
        {
            return Err("heterogeneity point pass candidate inventory or discovery identity differs from the compiled authority".into());
        }
        let expected_pooled =
            pooled_baseline_candidate_v2(&self.pooled_baseline, &prepared.fimix_scientific_targets)
                .map_err(|error| error.to_string())?;
        if self.candidates.first() != Some(&expected_pooled) {
            return Err("heterogeneity point pass pooled baseline evidence is inconsistent".into());
        }

        let requested = algorithms
            .iter()
            .flat_map(|algorithm| candidate_k.iter().map(move |k| (*algorithm, *k)))
            .collect::<BTreeSet<_>>();
        let mut evidenced = BTreeSet::new();
        for candidate in &self.fimix_candidates {
            let key = (CoreHeterogeneityAlgorithmV2::FimixPlsV2, candidate.k);
            if !requested.contains(&key)
                || !evidenced.insert(key)
                || validate_fimix_multistart_evidence_v2(&candidate.result).is_err()
                || !self.candidates.contains(&fimix_candidate(
                    CoreHeterogeneityAlgorithmV2::FimixPlsV2,
                    candidate.k,
                    &candidate.result,
                ))
            {
                return Err(
                    "heterogeneity point pass has invalid or duplicate FIMIX evidence".into(),
                );
            }
        }
        for candidate in &self.pos_candidates {
            let key = (candidate.algorithm, candidate.k);
            if matches!(
                candidate.algorithm,
                CoreHeterogeneityAlgorithmV2::FimixPlsV2
            ) || !requested.contains(&key)
                || !evidenced.insert(key)
                || validate_pos_multistart_evidence_v2(&candidate.result).is_err()
                || !self.candidates.contains(&pos_candidate(
                    candidate.algorithm,
                    candidate.k,
                    &candidate.result,
                ))
            {
                return Err(
                    "heterogeneity point pass has invalid or duplicate PLS-POS evidence".into(),
                );
            }
        }
        let stable = self
            .candidates
            .iter()
            .filter_map(|candidate| match candidate.method {
                HeterogeneityCandidateMethodV2::Segmentation { algorithm }
                    if candidate.state == HeterogeneityCandidateStateV2::ConvergedStable =>
                {
                    Some((algorithm, candidate.k))
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let ordered_evidence = self
            .evidence_order
            .iter()
            .map(|identity| match identity {
                PreparedHeterogeneityPointEvidenceOrderV2::Fimix { k } => {
                    (CoreHeterogeneityAlgorithmV2::FimixPlsV2, *k)
                }
                PreparedHeterogeneityPointEvidenceOrderV2::PlsPos { algorithm, k } => {
                    (*algorithm, *k)
                }
            })
            .collect::<Vec<_>>();
        if stable != evidenced
            || ordered_evidence.len() != evidenced.len()
            || ordered_evidence.iter().copied().collect::<BTreeSet<_>>() != evidenced
        {
            return Err(
                "heterogeneity point pass stable candidates and point evidence differ".into(),
            );
        }

        match (&config.phase, &self.locked) {
            (qpls_core::HeterogeneityPhaseV2::Discovery { .. }, None) => {}
            (qpls_core::HeterogeneityPhaseV2::Inference { lock }, Some(locked)) => {
                if locked.algorithm != lock.selected_algorithm
                    || locked.k != lock.selected_k
                    || locked.assignments.len() != prepared.fimix_input.metric.observation_count
                    || locked
                        .assignments
                        .iter()
                        .any(|class| *class >= usize::from(locked.k))
                    || !locked.fit_statistic.is_finite()
                    || locked.local_parameters.is_empty()
                {
                    return Err(
                        "heterogeneity locked point receipt differs from its inference lock".into(),
                    );
                }
                let (assignments, fit_statistic, parameters) = match locked.algorithm {
                    CoreHeterogeneityAlgorithmV2::FimixPlsV2 => {
                        let point = self
                            .fimix_candidates
                            .iter()
                            .find(|candidate| candidate.k == locked.k)
                            .ok_or_else(|| "locked FIMIX point evidence is missing".to_string())?;
                        (
                            point.result.hard_assignments.as_slice(),
                            point.result.log_likelihood,
                            fimix_parameters(&point.result, &prepared.fimix_scientific_targets)
                                .map_err(|error| error.to_string())?,
                        )
                    }
                    algorithm => {
                        let point = self
                            .pos_candidates
                            .iter()
                            .find(|candidate| {
                                candidate.algorithm == algorithm && candidate.k == locked.k
                            })
                            .ok_or_else(|| {
                                "locked PLS-POS point evidence is missing".to_string()
                            })?;
                        (
                            point.result.assignments.as_slice(),
                            point.result.objective,
                            pos_parameters(&point.result, &prepared.pos_parameter_ids)
                                .map_err(|error| error.to_string())?,
                        )
                    }
                };
                if locked.assignments != assignments
                    || locked.fit_statistic.to_bits() != fit_statistic.to_bits()
                    || locked.local_parameters != parameters
                {
                    return Err("heterogeneity locked point receipt is not reproduced by retained point evidence".into());
                }
            }
            _ => {
                return Err(
                    "heterogeneity point pass lock presence differs from the configured phase"
                        .into(),
                );
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_heterogeneity_point_pass_v2<R, C, P>(
    artifact: &CompiledMultiModRecipeV1,
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    algorithms: &[CoreHeterogeneityAlgorithmV2],
    candidate_k: &[u8],
    prepared: &PreparedHeterogeneityExecutionV2,
    pos_scientific_row_features: &[Vec<f64>],
    pos_refitter: &mut R,
    should_cancel: &C,
    progress: &P,
) -> Result<PreparedHeterogeneityPointPassV2, MultiModRunnerErrorV1>
where
    R: PlsPosFullRefitterV2,
    C: Fn() -> bool + Sync,
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    let inference_lock = match &config.phase {
        qpls_core::HeterogeneityPhaseV2::Discovery { .. } => None,
        qpls_core::HeterogeneityPhaseV2::Inference { lock } => Some(lock),
    };
    let mut controlled = ControlledPosRefitter {
        inner: pos_refitter,
        should_cancel,
        progress,
        calls: 0,
        cancelled: false,
    };
    let mut ordered_algorithms = algorithms.to_vec();
    ordered_algorithms.sort_by_key(|algorithm| match algorithm {
        CoreHeterogeneityAlgorithmV2::FimixPlsV2 => 0,
        CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2 => 1,
        CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => 2,
    });
    let pooled_baseline =
        fit_pooled_structural_baseline_v2(&prepared.fimix_input, config.fimix.rank_tolerance)
            .map_err(|error| {
                MultiModRunnerErrorV1::Kernel(format!(
                    "multimod.runner.heterogeneity.pooled_baseline_failed: {error}"
                ))
            })?;
    let mut candidates = vec![pooled_baseline_candidate_v2(
        &pooled_baseline,
        &prepared.fimix_scientific_targets,
    )?];
    let mut fimix_by_k = BTreeMap::<u8, FimixPlsV2Result>::new();
    let mut fimix_candidates = Vec::new();
    let mut pos_candidates = Vec::new();
    let mut evidence_order = Vec::new();
    for k in candidate_k {
        for algorithm in &ordered_algorithms {
            if should_cancel() || controlled.cancelled {
                return Err(MultiModRunnerErrorV1::Cancelled);
            }
            report(
                progress,
                MultiModRunnerPhaseV1::PointEstimation,
                candidates.len().saturating_sub(1) as u64,
                (candidate_k.len() * ordered_algorithms.len()) as u64,
                format!("heterogeneity:{algorithm:?}:k{k}"),
            );
            match algorithm {
                CoreHeterogeneityAlgorithmV2::FimixPlsV2 => {
                    let point =
                        fit_fimix_pls_v2(&prepared.fimix_input, &fimix_config(config, *k as usize));
                    match point {
                        Ok(point) => {
                            validate_fimix_multistart_evidence_v2(&point).map_err(|error| {
                                MultiModRunnerErrorV1::ResultContract(error.to_string())
                            })?;
                            candidates.push(fimix_candidate(*algorithm, *k, &point));
                            fimix_candidates.push(PreparedFimixPointCandidateV2 {
                                k: *k,
                                result: point.clone(),
                            });
                            evidence_order
                                .push(PreparedHeterogeneityPointEvidenceOrderV2::Fimix { k: *k });
                            fimix_by_k.insert(*k, point);
                        }
                        Err(error) => {
                            if controlled.cancelled || should_cancel() {
                                return Err(MultiModRunnerErrorV1::Cancelled);
                            }
                            candidates
                                .push(candidate_from_heterogeneity_error(*algorithm, *k, &error));
                            if inference_lock.is_some_and(|lock| {
                                lock.selected_algorithm == *algorithm && lock.selected_k == *k
                            }) {
                                return Err(MultiModRunnerErrorV1::Kernel(error.to_string()));
                            }
                        }
                    }
                }
                CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2
                | CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2 => {
                    let tandem_fimix_requested =
                        ordered_algorithms.contains(&CoreHeterogeneityAlgorithmV2::FimixPlsV2);
                    let same_k = fimix_by_k
                        .get(k)
                        .map(|result| result.hard_assignments.as_slice());
                    let point = if tandem_fimix_requested && same_k.is_none() {
                        Err(HeterogeneityV2Error::InvalidContract(format!(
                            "same-K FIMIX start is unavailable for tandem POS candidate K={k}"
                        )))
                    } else {
                        run_pos_candidate(
                            *algorithm,
                            config.profile,
                            *k as usize,
                            config,
                            prepared,
                            pos_scientific_row_features,
                            same_k,
                            &mut controlled,
                        )
                    };
                    match point {
                        Ok(point) => {
                            validate_pos_multistart_evidence_v2(&point).map_err(|error| {
                                MultiModRunnerErrorV1::ResultContract(error.to_string())
                            })?;
                            candidates.push(pos_candidate(*algorithm, *k, &point));
                            pos_candidates.push(PreparedPosPointCandidateV2 {
                                algorithm: *algorithm,
                                k: *k,
                                result: point,
                            });
                            evidence_order.push(
                                PreparedHeterogeneityPointEvidenceOrderV2::PlsPos {
                                    algorithm: *algorithm,
                                    k: *k,
                                },
                            );
                        }
                        Err(error) => {
                            if controlled.cancelled || should_cancel() {
                                return Err(MultiModRunnerErrorV1::Cancelled);
                            }
                            candidates
                                .push(candidate_from_heterogeneity_error(*algorithm, *k, &error));
                            if inference_lock.is_some_and(|lock| {
                                lock.selected_algorithm == *algorithm && lock.selected_k == *k
                            }) {
                                return Err(MultiModRunnerErrorV1::Kernel(error.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }
    if should_cancel() || controlled.cancelled {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    let discovery_result_identity_sha256 =
        heterogeneity_discovery_result_identity_v2(artifact.receipt(), config, &candidates);
    if inference_lock.is_some_and(|lock| {
        discovery_result_identity_sha256 != lock.discovery_result_identity_sha256
    }) {
        return Err(MultiModRunnerErrorV1::Authority(
            "multimod.runner.heterogeneity.discovery_lock_identity_mismatch".into(),
        ));
    }
    let locked = inference_lock
        .map(
            |lock| -> Result<PreparedLockedHeterogeneityPointV2, MultiModRunnerErrorV1> {
                match lock.selected_algorithm {
                    CoreHeterogeneityAlgorithmV2::FimixPlsV2 => {
                        let point = fimix_candidates
                            .iter()
                            .find(|candidate| candidate.k == lock.selected_k)
                            .ok_or_else(|| {
                                MultiModRunnerErrorV1::Kernel(
                                    "locked FIMIX candidate did not complete".into(),
                                )
                            })?;
                        Ok(PreparedLockedHeterogeneityPointV2 {
                            algorithm: lock.selected_algorithm,
                            k: lock.selected_k,
                            assignments: point.result.hard_assignments.clone(),
                            fit_statistic: point.result.log_likelihood,
                            local_parameters: fimix_parameters(
                                &point.result,
                                &prepared.fimix_scientific_targets,
                            )?,
                        })
                    }
                    algorithm => {
                        let point = pos_candidates
                            .iter()
                            .find(|candidate| {
                                candidate.algorithm == algorithm && candidate.k == lock.selected_k
                            })
                            .ok_or_else(|| {
                                MultiModRunnerErrorV1::Kernel(
                                    "locked PLS-POS candidate did not complete".into(),
                                )
                            })?;
                        Ok(PreparedLockedHeterogeneityPointV2 {
                            algorithm,
                            k: lock.selected_k,
                            assignments: point.result.assignments.clone(),
                            fit_statistic: point.result.objective,
                            local_parameters: pos_parameters(
                                &point.result,
                                &prepared.pos_parameter_ids,
                            )?,
                        })
                    }
                }
            },
        )
        .transpose()?;
    let mut point = PreparedHeterogeneityPointPassV2 {
        method_version: "qpls.heterogeneity.point-pass.v2".into(),
        candidates,
        discovery_result_identity_sha256,
        pooled_baseline,
        fimix_candidates,
        pos_candidates,
        evidence_order,
        locked,
        point_pass_identity_sha256: String::new(),
    };
    point.point_pass_identity_sha256 = heterogeneity_point_pass_identity_v2(&point);
    point
        .ensure_valid(artifact, config, prepared)
        .map_err(MultiModRunnerErrorV1::ResultContract)?;
    Ok(point)
}

fn apply_heterogeneity_bootstrap(
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    algorithm: CoreHeterogeneityAlgorithmV2,
    k: u8,
    prepared: &PreparedHeterogeneityBootstrapV2,
    parameters: &mut [HeterogeneityClassParameterV2],
    contrasts: &mut [HeterogeneityClassContrastV2],
) -> Result<MultimodReplicateLedgerSummaryV1, MultiModRunnerErrorV1> {
    let bootstrap = config.bootstrap.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::PreparedInput(
            "prepared bootstrap was supplied without a bootstrap configuration".into(),
        )
    })?;
    let retained_k = prepared
        .ensure_valid()
        .map_err(MultiModRunnerErrorV1::InvalidLedger)?;
    if retained_k != usize::from(k) {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "segmentation bootstrap retained K={retained_k}, expected K={k}"
        )));
    }
    if !prepared.complete_stage_one_and_segmentation_rerun
        || !prepared.exhaustive_label_alignment_applied
        || (!contrasts.is_empty() && !prepared.pooled_common_metric_refit_repeated)
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "segmentation bootstrap must rerun the complete pipeline, common metric where applicable, and exhaustive label alignment"
                .into(),
        ));
    }
    let plan = HeterogeneityBootstrapPlanV2 {
        algorithm: heterogeneity_bootstrap_algorithm(algorithm),
        fixed_classes_or_segments: k as usize,
        requested_replicates: bootstrap.resamples as usize,
        master_seed: bootstrap.seed,
        confidence_level: bootstrap.confidence_level,
        minimum_usable_share: 0.90,
    };
    let summary = summarize_heterogeneity_bootstrap_ledger_v2(&plan, &prepared.entries)
        .map_err(|error| MultiModRunnerErrorV1::InvalidLedger(error.to_string()))?;
    if prepared
        .entries
        .iter()
        .enumerate()
        .any(|(index, entry)| entry.replicate_index != index)
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "segmentation bootstrap ledger must be stored in replicate-index order".into(),
        ));
    }
    if summary.qualification != HeterogeneityBootstrapQualificationV2::Qualified {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "fixed-K bootstrap did not qualify: {:?}",
            summary.qualification
        )));
    }
    let expected_ids = parameters
        .iter()
        .map(|parameter| parameter.parameter.target_id.clone())
        .collect::<BTreeSet<_>>();
    let actual_ids = prepared
        .targets
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<BTreeSet<_>>();
    if expected_ids != actual_ids || actual_ids.len() != prepared.targets.len() {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "segmentation bootstrap target inventory differs from the locked point fit".into(),
        ));
    }
    for target in &prepared.targets {
        if target.estimates.len() != plan.requested_replicates {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "segmentation bootstrap target vector has the wrong length".into(),
            ));
        }
        let mut draws = Vec::new();
        for (index, value) in target.estimates.iter().enumerate() {
            let usable = prepared.entries[index].status
                == qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable;
            if usable != value.is_some() || value.is_some_and(|value| !value.is_finite()) {
                return Err(MultiModRunnerErrorV1::InvalidLedger(
                    "segmentation targets disagree with the shared bootstrap validity bitmap"
                        .into(),
                ));
            }
            if let Some(value) = value {
                draws.push(*value);
            }
        }
        let estimated = parameters
            .iter_mut()
            .find(|parameter| parameter.parameter.target_id == target.target_id)
            .expect("target inventory checked above");
        let result = percentile_interval_v2(
            &draws,
            plan.requested_replicates,
            plan.confidence_level,
            ConditionalAlternativeV2::TwoSided,
        )
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        estimated.parameter.interval = Some(interval(
            "type_7_two_sided_percentile",
            plan.confidence_level,
            InferenceAlternativeV1::TwoSided,
            result.lower,
            result.upper,
        )?);
        // The qualified segmentation contract is percentile-interval only.
        // A zero-crossing tail area is not a separately qualified p-value.
        estimated.parameter.p_value = None;
    }
    let target_draws = prepared
        .targets
        .iter()
        .map(|target| (target.target_id.as_str(), target.estimates.as_slice()))
        .collect::<BTreeMap<_, _>>();
    for contrast in contrasts {
        let left_id = format!("class_{}:{}", contrast.left_class_id, contrast.target_id);
        let right_id = format!("class_{}:{}", contrast.right_class_id, contrast.target_id);
        let left = target_draws.get(left_id.as_str()).ok_or_else(|| {
            MultiModRunnerErrorV1::InvalidLedger(format!(
                "common-metric bootstrap omitted contrast endpoint {left_id}"
            ))
        })?;
        let right = target_draws.get(right_id.as_str()).ok_or_else(|| {
            MultiModRunnerErrorV1::InvalidLedger(format!(
                "common-metric bootstrap omitted contrast endpoint {right_id}"
            ))
        })?;
        let draws = left
            .iter()
            .zip(*right)
            .enumerate()
            .filter_map(|(index, (left, right))| {
                if prepared.entries[index].status
                    == qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable
                {
                    match (*left, *right) {
                        (Some(left), Some(right)) => Some(left - right),
                        _ => None,
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if draws.len() != summary.usable_replicates {
            return Err(MultiModRunnerErrorV1::InvalidLedger(
                "common-metric contrast draws disagree with the shared validity bitmap".into(),
            ));
        }
        let result = percentile_interval_v2(
            &draws,
            plan.requested_replicates,
            plan.confidence_level,
            ConditionalAlternativeV2::TwoSided,
        )
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        contrast.interval = Some(interval(
            "type_7_two_sided_percentile",
            plan.confidence_level,
            InferenceAlternativeV1::TwoSided,
            result.lower,
            result.upper,
        )?);
        contrast.p_value = None;
    }
    let mut failure_counts = BTreeMap::<String, u32>::new();
    let failures = prepared
        .entries
        .iter()
        .filter(|entry| {
            entry.status != qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable
        })
        .map(|entry| {
            let (kind, code) = match entry.status {
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable => unreachable!(),
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::FitFailed => (
                    MultimodReplicateFailureKindV1::EstimatorDidNotConverge,
                    "heterogeneity.bootstrap.fit_failed",
                ),
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::LabelAmbiguous => (
                    MultimodReplicateFailureKindV1::AmbiguousLabelAlignment,
                    "heterogeneity.bootstrap.label_ambiguous",
                ),
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::LabelNotMutualMajority => (
                    MultimodReplicateFailureKindV1::AmbiguousLabelAlignment,
                    "heterogeneity.bootstrap.label_not_mutual_majority",
                ),
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::ComparabilityFailed => (
                    MultimodReplicateFailureKindV1::ComparabilityFailed,
                    "heterogeneity.bootstrap.comparability_failed",
                ),
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::NonFiniteTarget => (
                    MultimodReplicateFailureKindV1::NonfiniteEstimate,
                    "heterogeneity.bootstrap.nonfinite_target",
                ),
                qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Cancelled => (
                    MultimodReplicateFailureKindV1::Cancelled,
                    "heterogeneity.bootstrap.cancelled",
                ),
            };
            *failure_counts.entry(code.into()).or_default() += 1;
            MultimodReplicateFailureV1 {
                replicate_index: entry.replicate_index as u32,
                kind,
                stable_code: code.into(),
                detail: entry
                    .failure_reason
                    .clone()
                    .unwrap_or_else(|| "bootstrap replicate failed".into()),
            }
        })
        .collect::<Vec<_>>();
    Ok(MultimodReplicateLedgerSummaryV1 {
        requested: summary.requested_replicates as u32,
        usable: summary.usable_replicates as u32,
        minimum_required: summary.required_usable_replicates as u32,
        usable_fraction: summary.usable_replicates as f64 / summary.requested_replicates as f64,
        complete: true,
        ledger_sha256: sha256_serialized(&prepared.entries),
        failure_counts,
        failures,
    })
}

fn assemble_heterogeneity_result_from_point_pass_v2<P>(
    artifact: &CompiledMultiModRecipeV1,
    config: &qpls_core::PlsUnobservedHeterogeneityConfigV2,
    prepared: &PreparedHeterogeneityExecutionV2,
    point_pass: &PreparedHeterogeneityPointPassV2,
    progress: &P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    P: Fn(MultiModRunnerProgressV1) + Sync,
{
    point_pass
        .ensure_valid(artifact, config, prepared)
        .map_err(MultiModRunnerErrorV1::PreparedInput)?;
    let inference_lock = match &config.phase {
        qpls_core::HeterogeneityPhaseV2::Discovery { .. } => None,
        qpls_core::HeterogeneityPhaseV2::Inference { lock } => Some(lock),
    };
    let (locked_algorithm, locked_k, mut parameters) =
        point_pass
            .locked
            .as_ref()
            .map_or((None, None, Vec::new()), |locked| {
                (
                    Some(locked.algorithm),
                    Some(locked.k),
                    locked.local_parameters.clone(),
                )
            });
    let mut evidence = point_pass.evidence();
    let mut descriptive_only = locked_algorithm.is_none_or(|algorithm| {
        matches!(
            algorithm,
            CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2
                | CoreHeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2
        )
    });
    let mut contrasts = if let Some(gate_config) = config.pos_common_metric.as_ref() {
        if gate_config.request_segment_contrasts {
            let gate_input = prepared.pos_common_metric_gate.as_ref().ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(
                    "requested POS contrasts require prepared common-metric evidence".into(),
                )
            })?;
            if Some(gate_input.segments as u8) != locked_k {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "common-metric evidence is bound to a different locked K".into(),
                ));
            }
            let gate = evaluate_pos_common_metric_gate_v1(gate_input);
            descriptive_only = gate.status == PosCommonMetricGateStatusV1::DescriptiveOnly;
            let expected_pairs =
                usize::from(locked_k.expect("common metric has locked K")).saturating_mul(
                    usize::from(locked_k.expect("common metric has locked K") - 1),
                ) / 2;
            if !prepared.pos_common_metric_contrasts.is_empty()
                || prepared.pos_common_metric_micom_pairs.len() != expected_pairs
            {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "POS common-metric point contrasts must be runner-derived from a complete MICOM pair inventory"
                        .into(),
                ));
            }
            if descriptive_only {
                Vec::new()
            } else {
                if prepared.pos_common_metric_parameters.is_empty()
                    || prepared
                        .pos_common_metric_parameters
                        .iter()
                        .any(|row| !row.metric.starts_with("qpls.pos.pooled-common-metric.v1:"))
                {
                    return Err(MultiModRunnerErrorV1::PreparedInput(
                        "passed POS common-metric gate omitted pooled-metric segment parameters"
                            .into(),
                    ));
                }
                parameters = prepared.pos_common_metric_parameters.clone();
                common_metric_point_contrasts_v2(
                    &parameters,
                    locked_k.expect("common metric has locked K"),
                )?
            }
        } else {
            if !prepared.pos_common_metric_contrasts.is_empty()
                || !prepared.pos_common_metric_parameters.is_empty()
                || !prepared.pos_common_metric_micom_pairs.is_empty()
                || prepared.pos_common_metric_gate.is_some()
            {
                return Err(MultiModRunnerErrorV1::PreparedInput(
                    "POS common-metric material was supplied without requesting contrasts".into(),
                ));
            }
            Vec::new()
        }
    } else {
        if !prepared.pos_common_metric_contrasts.is_empty()
            || !prepared.pos_common_metric_parameters.is_empty()
            || !prepared.pos_common_metric_micom_pairs.is_empty()
            || prepared.pos_common_metric_gate.is_some()
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "common-metric material was supplied outside the POS contrast profile".into(),
            ));
        }
        Vec::new()
    };
    let bootstrap_ledger = match (&config.bootstrap, &prepared.bootstrap) {
        (Some(_), Some(prepared_bootstrap)) => {
            let ledger = apply_heterogeneity_bootstrap(
                config,
                locked_algorithm.ok_or_else(|| {
                    MultiModRunnerErrorV1::PreparedInput(
                        "bootstrap requires an explicitly locked algorithm".into(),
                    )
                })?,
                locked_k.expect("locked algorithm has K"),
                prepared_bootstrap,
                &mut parameters,
                &mut contrasts,
            )?;
            evidence.push(MultiModRunnerEvidenceV1::HeterogeneityBootstrap(
                prepared_bootstrap.clone(),
            ));
            Some(ledger)
        }
        (Some(_), None) => {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "locked segmentation inference requires its complete bootstrap ledger".into(),
            ));
        }
        (None, Some(_)) => {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "prepared bootstrap was supplied during discovery".into(),
            ));
        }
        (None, None) => None,
    };
    report(
        progress,
        MultiModRunnerPhaseV1::Completed,
        1,
        1,
        "heterogeneity:complete",
    );
    let analysis = PlsHeterogeneityAnalysisV2 {
        schema_version: PLS_HETEROGENEITY_ANALYSIS_V2_SCHEMA_VERSION,
        provenance: provenance(artifact.receipt(), config.seed),
        profile: config.profile,
        candidates: point_pass.candidates.clone(),
        discovery_result_identity_sha256: point_pass.discovery_result_identity_sha256.clone(),
        inference_lock: inference_lock.cloned(),
        locked_algorithm,
        locked_k,
        parameters,
        contrasts,
        bootstrap_ledger,
        sidecars: Vec::new(),
        descriptive_only,
    };
    let result = MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(analysis);
    result
        .ensure_valid()
        .map_err(|error| MultiModRunnerErrorV1::ResultContract(error.to_string()))?;
    Ok(MultiModRunOutputV1 {
        compilation_receipt: artifact.receipt().clone(),
        result,
        evidence,
    })
}

pub fn run_compiled_pls_heterogeneity_v2<R, C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    prepared: &PreparedHeterogeneityExecutionV2,
    pos_refitter: &mut R,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
    R: PlsPosFullRefitterV2,
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
        "heterogeneity:authority",
    );
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::PlsHeterogeneityV2,
    )?;
    let config = recipe.pls_heterogeneity.as_ref().ok_or_else(|| {
        MultiModRunnerErrorV1::Authority(
            "heterogeneity configuration disappeared after compilation".into(),
        )
    })?;
    let CompiledMultiModPlanV1::PlsHeterogeneityV2 {
        profile,
        algorithms,
        candidate_k,
        ..
    } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not PLS heterogeneity V2".into(),
        ));
    };
    let inference_lock: Option<&HeterogeneityInferenceLockReceiptV2> = match &config.phase {
        qpls_core::HeterogeneityPhaseV2::Discovery { .. } => None,
        qpls_core::HeterogeneityPhaseV2::Inference { lock } => Some(lock),
    };
    if let Some(lock) = inference_lock {
        if algorithms != &lock.discovery_algorithms || candidate_k != &lock.discovery_candidate_k {
            return Err(MultiModRunnerErrorV1::Authority(
                "multimod.runner.heterogeneity.lock_compiler_inventory_mismatch".into(),
            ));
        }
    }
    if profile != &config.profile
        || prepared.fimix_input.interaction_profile
            != estimation_heterogeneity_profile(config.profile)
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "prepared standardized input has the wrong interaction profile".into(),
        ));
    }
    if config.profile != CoreHeterogeneityProfileV2::P0Structural
        && prepared.fimix_scientific_targets.is_empty()
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "interaction FIMIX requires explicit product-scale gamma/delta and fixed-slope projections"
                .into(),
        ));
    }
    let uses_pos = algorithms
        .iter()
        .any(|algorithm| !matches!(algorithm, CoreHeterogeneityAlgorithmV2::FimixPlsV2));
    if prepared.fimix_input.metric.observation_count > dataset.batch.num_rows()
        || (uses_pos
            && (prepared.pos_start_features.len() != prepared.fimix_input.metric.observation_count
                || prepared.pos_start_features.is_empty()))
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "PLS-POS requires one finite start-feature row per source observation".into(),
        ));
    }
    let point_pass = execute_heterogeneity_point_pass_v2(
        artifact,
        config,
        algorithms,
        candidate_k,
        prepared,
        &prepared.pos_start_features,
        pos_refitter,
        &should_cancel,
        &progress,
    )?;
    assemble_heterogeneity_result_from_point_pass_v2(
        artifact,
        config,
        prepared,
        &point_pass,
        &progress,
    )
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedConditionalProbeContrastBindingV2 {
    pub left_probe_id: String,
    pub right_probe_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedConditionalPointBatchV2 {
    pub targets: Vec<ConditionalProcessTargetResultV2>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparedConditionalEdgeBindingV2 {
    pub relation_id: String,
    pub source_id: String,
    pub target_id: String,
}

/// Converts the existing General-SEM PLS point authority into V2 conditional
/// edge functions. Scientific product coefficients use the already validated
/// raw-product gamma and rescaled three-way delta, never the internal
/// standardized generated-column coefficient.
pub fn conditional_edges_from_general_sem_pls_point_v2(
    result: &crate::RecipeV4GeneralSemPlsExecutionResultV1,
    bindings: &[PreparedConditionalEdgeBindingV2],
) -> Result<Vec<ConditionalEdgeFunctionV2>, MultiModRunnerErrorV1> {
    let mut binding_ids = BTreeSet::new();
    if bindings.is_empty()
        || bindings.iter().any(|binding| {
            binding.relation_id.trim().is_empty()
                || binding.source_id.trim().is_empty()
                || binding.target_id.trim().is_empty()
                || !binding_ids.insert(binding.relation_id.clone())
        })
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "conditional edge bindings require unique nonempty relation identities".into(),
        ));
    }
    let interaction_point = result.interaction_point_estimation();
    let three_way_point = result.three_way_point_estimation();
    if interaction_point.is_some() && three_way_point.is_some() {
        return Err(MultiModRunnerErrorV1::Authority(
            "General-SEM point authority contains two competing interaction point families".into(),
        ));
    }
    let mut output = Vec::with_capacity(bindings.len());
    for binding in bindings {
        let intercept = if let Some(point) = interaction_point {
            point
                .structural_coefficients()
                .iter()
                .find(|coefficient| coefficient.relation_id() == binding.relation_id)
                .map(|coefficient| coefficient.estimate())
        } else if let Some(point) = three_way_point {
            point
                .structural_coefficients
                .iter()
                .find(|coefficient| coefficient.relation_id() == binding.relation_id)
                .map(|coefficient| coefficient.estimate())
        } else {
            let matching = result
                .point_estimation()
                .estimation()
                .paths
                .iter()
                .filter(|path| path.source == binding.source_id && path.target == binding.target_id)
                .collect::<Vec<_>>();
            if matching.len() == 1 {
                Some(matching[0].coefficient)
            } else {
                None
            }
        }
        .ok_or_else(|| {
            MultiModRunnerErrorV1::PreparedInput(format!(
                "General-SEM point result lacks a unique coefficient for relation {}",
                binding.relation_id
            ))
        })?;
        let interaction_coefficients = if let Some(point) = interaction_point {
            point.interaction_coefficients()
        } else if let Some(point) = three_way_point {
            point.lower_order_interaction_coefficients.as_slice()
        } else {
            &[]
        };
        let mut linear_coefficients = interaction_coefficients
            .iter()
            .filter(|coefficient| {
                coefficient.focal_relation_id() == binding.relation_id
                    && coefficient.focal_predictor_id() == binding.source_id
                    && coefficient.outcome_id() == binding.target_id
            })
            .map(
                |coefficient| qpls_estimation::ConditionalLinearCoefficientV2 {
                    moderator_id: coefficient.moderator_id().to_owned(),
                    estimate: coefficient.raw_product_estimate(),
                },
            )
            .collect::<Vec<_>>();
        linear_coefficients.sort_by(|left, right| left.moderator_id.cmp(&right.moderator_id));
        if linear_coefficients
            .windows(2)
            .any(|pair| pair[0].moderator_id == pair[1].moderator_id)
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(format!(
                "relation {} has duplicate scientific gamma bindings",
                binding.relation_id
            )));
        }
        let mut pairwise_coefficients = Vec::new();
        if let Some(point) = three_way_point {
            let three_way = &point.three_way_coefficient;
            if three_way.focal_relation_id == binding.relation_id {
                let moderators = three_way
                    .operand_ids
                    .iter()
                    .filter(|operand| *operand != &binding.source_id)
                    .cloned()
                    .collect::<Vec<_>>();
                if moderators.len() != 2 || three_way.outcome_id != binding.target_id {
                    return Err(MultiModRunnerErrorV1::PreparedInput(format!(
                        "three-way coefficient for relation {} does not identify exactly two moderators",
                        binding.relation_id
                    )));
                }
                pairwise_coefficients.push(qpls_estimation::ConditionalPairwiseCoefficientV2 {
                    first_moderator_id: moderators[0].clone(),
                    second_moderator_id: moderators[1].clone(),
                    estimate: three_way.scientific_rescaled_delta,
                });
            }
        }
        output.push(ConditionalEdgeFunctionV2 {
            relation_id: binding.relation_id.clone(),
            source_id: binding.source_id.clone(),
            target_id: binding.target_id.clone(),
            intercept,
            linear_coefficients,
            pairwise_coefficients,
        });
    }
    Ok(output)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedConditionalInferenceV2 {
    pub ledger: PreparedSharedReplicateLedgerV1,
    pub targets: Vec<PreparedTargetReplicatesV1>,
    pub analysis_observations: usize,
    pub complete_model_refit_per_replicate: bool,
    pub original_sample_probe_anchors_frozen: bool,
    pub hoc_dependency_stages_repeated: bool,
    pub stratified_group_resampling: bool,
    pub weights_travel_with_resampled_rows: bool,
    pub frequency_count_space_resampling: bool,
    pub nested_inner_refits_complete: bool,
}

fn namespaced_conditional_target_id(group_id: Option<&str>, target_id: &str) -> String {
    group_id
        .map(|group_id| format!("group:{group_id}:{target_id}"))
        .unwrap_or_else(|| target_id.to_owned())
}

/// Evaluates one explicitly ordered conditional path through the estimation
/// kernel. Total-indirect/total-effect and grouped contrasts require a caller
/// to combine complete point batches explicitly; this helper never guesses
/// omitted paths or direct effects.
pub fn prepare_conditional_path_point_targets_v2(
    path: &ExplicitConditionalPathV2,
    probes: &[ConditionalProbePointV2],
    group_id: Option<&str>,
    include_conditional_specific_indirect: bool,
    include_scalar_index_when_affine: bool,
    include_first_derivatives: bool,
    include_second_and_cross_derivatives: bool,
    contrasts: &[PreparedConditionalProbeContrastBindingV2],
) -> Result<PreparedConditionalPointBatchV2, MultiModRunnerErrorV1> {
    let polynomial = compile_explicit_conditional_path_v2(path)
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
    let mut targets = Vec::new();
    let mut warnings = Vec::new();
    for probe in probes {
        if include_conditional_specific_indirect {
            let effect = conditional_effect_v2(&polynomial, probe)
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
            targets.push(ConditionalProcessTargetResultV2 {
                target_id: namespaced_conditional_target_id(group_id, &effect.target_id),
                kind: ConditionalProcessTargetKindV2::ConditionalSpecificIndirect,
                path_id: effect.path_id,
                group_id: group_id.map(str::to_owned),
                probe_values: probe.standardized_values.clone(),
                derivative_variables: Vec::new(),
                estimate: effect.estimate,
                p_value: None,
                interval: None,
                usable_replicates: 0,
            });
        }
        if include_first_derivatives || include_second_and_cross_derivatives {
            for derivative in conditional_derivatives_v2(&polynomial, probe)
                .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?
            {
                let (include, kind) = match derivative.kind {
                    ConditionalDerivativeKindV2::First => (
                        include_first_derivatives,
                        ConditionalProcessTargetKindV2::LocalFirstDerivative,
                    ),
                    ConditionalDerivativeKindV2::Second => (
                        include_second_and_cross_derivatives,
                        ConditionalProcessTargetKindV2::LocalSecondDerivative,
                    ),
                    ConditionalDerivativeKindV2::Cross => (
                        include_second_and_cross_derivatives,
                        ConditionalProcessTargetKindV2::LocalCrossDerivative,
                    ),
                };
                if include {
                    let mut variables = vec![derivative.first_moderator_id];
                    if let Some(second) = derivative.second_moderator_id {
                        variables.push(second);
                    }
                    targets.push(ConditionalProcessTargetResultV2 {
                        target_id: namespaced_conditional_target_id(
                            group_id,
                            &derivative.target_id,
                        ),
                        kind,
                        path_id: derivative.path_id,
                        group_id: group_id.map(str::to_owned),
                        probe_values: probe.standardized_values.clone(),
                        derivative_variables: variables,
                        estimate: derivative.estimate,
                        p_value: None,
                        interval: None,
                        usable_replicates: 0,
                    });
                }
            }
        }
    }
    if include_scalar_index_when_affine {
        if polynomial.moderator_ids.len() == 1 {
            match scalar_index_of_moderated_mediation_v2(&polynomial, &polynomial.moderator_ids[0])
            {
                Ok(index) => targets.push(ConditionalProcessTargetResultV2 {
                    target_id: namespaced_conditional_target_id(group_id, &index.target_id),
                    kind: ConditionalProcessTargetKindV2::ScalarIndexOfModeratedMediation,
                    path_id: index.path_id,
                    group_id: group_id.map(str::to_owned),
                    probe_values: BTreeMap::new(),
                    derivative_variables: vec![index.moderator_id],
                    estimate: index.estimate,
                    p_value: None,
                    interval: None,
                    usable_replicates: 0,
                }),
                Err(ConditionalProcessMathErrorV2::ScalarIndexNotAffine) => warnings.push(format!(
                    "path {} is not affine in one moderator; no constant Hayes index was reported",
                    polynomial.path_id
                )),
                Err(error) => return Err(MultiModRunnerErrorV1::Kernel(error.to_string())),
            }
        } else {
            warnings.push(format!(
                "path {} depends on {} moderators; no constant Hayes index was reported",
                polynomial.path_id,
                polynomial.moderator_ids.len()
            ));
        }
    }
    for binding in contrasts {
        let left = probes
            .iter()
            .find(|probe| probe.probe_id == binding.left_probe_id)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "unknown left probe {}",
                    binding.left_probe_id
                ))
            })?;
        let right = probes
            .iter()
            .find(|probe| probe.probe_id == binding.right_probe_id)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(format!(
                    "unknown right probe {}",
                    binding.right_probe_id
                ))
            })?;
        let contrast = conditional_probe_contrast_v2(&polynomial, left, right)
            .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        targets.push(ConditionalProcessTargetResultV2 {
            target_id: namespaced_conditional_target_id(group_id, &contrast.target_id),
            kind: ConditionalProcessTargetKindV2::ProbeContrast,
            path_id: contrast.path_id,
            group_id: group_id.map(str::to_owned),
            probe_values: BTreeMap::new(),
            derivative_variables: Vec::new(),
            estimate: contrast.estimate,
            p_value: None,
            interval: None,
            usable_replicates: 0,
        });
    }
    Ok(PreparedConditionalPointBatchV2 { targets, warnings })
}

fn conditional_profile_id(profile: CoreConditionalProfileV2) -> &'static str {
    match profile {
        CoreConditionalProfileV2::MultiTwoWayPercentile => "multi_two_way_percentile",
        CoreConditionalProfileV2::MultiTwoWayBca => "multi_two_way_bca",
        CoreConditionalProfileV2::MultiTwoWayStudentized => "multi_two_way_studentized",
        CoreConditionalProfileV2::BoundedThreeWayPercentile => "bounded_three_way_percentile",
        CoreConditionalProfileV2::MultipleHocPercentile => "multiple_hoc_percentile",
        CoreConditionalProfileV2::GroupedPercentile => "grouped_percentile",
        CoreConditionalProfileV2::CaseWeightedPercentile => "case_weighted_percentile",
        CoreConditionalProfileV2::FrequencyWeightedPercentile => "frequency_weighted_percentile",
    }
}

fn conditional_kind_requested(
    config: &qpls_core::GeneralSemConditionalProcessConfigV2,
    kind: &ConditionalProcessTargetKindV2,
) -> bool {
    match kind {
        ConditionalProcessTargetKindV2::ConditionalSpecificIndirect => {
            config.estimands.conditional_specific_indirect
        }
        ConditionalProcessTargetKindV2::ConditionalTotalIndirect => {
            config.estimands.conditional_total_indirect
        }
        ConditionalProcessTargetKindV2::ConditionalTotalEffect => {
            config.estimands.conditional_total_effect
        }
        ConditionalProcessTargetKindV2::ScalarIndexOfModeratedMediation => {
            config.estimands.scalar_index_when_affine
        }
        ConditionalProcessTargetKindV2::LocalFirstDerivative => {
            config.estimands.local_first_derivatives
        }
        ConditionalProcessTargetKindV2::LocalSecondDerivative
        | ConditionalProcessTargetKindV2::LocalCrossDerivative => {
            config.estimands.local_second_and_cross_derivatives
        }
        ConditionalProcessTargetKindV2::ProbeContrast => config.estimands.finite_probe_contrasts,
        ConditionalProcessTargetKindV2::GroupContrast => !config.group_contrasts.is_empty(),
    }
}

fn validate_conditional_point_batch(
    config: &qpls_core::GeneralSemConditionalProcessConfigV2,
    plan: &CompiledMultiModPlanV1,
    point: &PreparedConditionalPointBatchV2,
) -> Result<(), MultiModRunnerErrorV1> {
    let CompiledMultiModPlanV1::GeneralSemConditionalProcessV2 {
        paths,
        compiled_target_upper_bound,
        ..
    } = plan
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not conditional-process V2".into(),
        ));
    };
    let path_ids = paths
        .iter()
        .map(|path| path.path_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    if point.targets.is_empty()
        || point.targets.len() > *compiled_target_upper_bound
        || point.targets.iter().any(|target| {
            !ids.insert(target.target_id.clone())
                || !path_ids.contains(target.path_id.as_str())
                || !target.estimate.is_finite()
                || target.p_value.is_some()
                || target.interval.is_some()
                || target.usable_replicates != 0
                || !conditional_kind_requested(config, &target.kind)
        })
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "conditional-process point targets are empty, duplicated, nonfinite, unrequested, or outside the compiled inventory"
                .into(),
        ));
    }
    let configured_groups = config
        .groups
        .iter()
        .map(|group| group.group_id.as_str())
        .collect::<BTreeSet<_>>();
    if config.profile == CoreConditionalProfileV2::GroupedPercentile {
        if point.targets.iter().any(|target| {
            target
                .group_id
                .as_deref()
                .is_none_or(|group| !configured_groups.contains(group))
        }) {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "grouped conditional targets require a configured group identity".into(),
            ));
        }
    } else if point.targets.iter().any(|target| target.group_id.is_some()) {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "group identities are allowed only in the grouped profile".into(),
        ));
    }
    Ok(())
}

fn validate_conditional_inference_receipt(
    config: &qpls_core::GeneralSemConditionalProcessConfigV2,
    inference: &PreparedConditionalInferenceV2,
) -> Result<(), MultiModRunnerErrorV1> {
    if !inference.complete_model_refit_per_replicate
        || !inference.original_sample_probe_anchors_frozen
        || inference.analysis_observations == 0
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "conditional inference requires complete refits, frozen original anchors, and a positive analysis-sample size"
                .into(),
        ));
    }
    match config.profile {
        CoreConditionalProfileV2::MultipleHocPercentile
            if !inference.hoc_dependency_stages_repeated =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "HOC dependency stages must be repeated in every resample".into(),
            ));
        }
        CoreConditionalProfileV2::GroupedPercentile if !inference.stratified_group_resampling => {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "grouped conditional process requires stratified resampling".into(),
            ));
        }
        CoreConditionalProfileV2::CaseWeightedPercentile
            if !inference.weights_travel_with_resampled_rows =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "case weights must travel with resampled rows".into(),
            ));
        }
        CoreConditionalProfileV2::FrequencyWeightedPercentile
            if !inference.frequency_count_space_resampling =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "frequency weights require count-space bootstrap equivalent to expansion".into(),
            ));
        }
        CoreConditionalProfileV2::MultiTwoWayStudentized
            if !inference.nested_inner_refits_complete =>
        {
            return Err(MultiModRunnerErrorV1::PreparedInput(
                "studentized inference requires complete nested inner refits with no percentile fallback"
                    .into(),
            ));
        }
        _ => {}
    }
    Ok(())
}

fn apply_conditional_inference(
    dataset: &Dataset,
    config: &qpls_core::GeneralSemConditionalProcessConfigV2,
    prepared: &PreparedConditionalInferenceV2,
    targets: &mut [ConditionalProcessTargetResultV2],
) -> Result<MultimodReplicateLedgerSummaryV1, MultiModRunnerErrorV1> {
    validate_conditional_inference_receipt(config, prepared)?;
    if prepared.analysis_observations > dataset.batch.num_rows()
        || prepared.ledger.master_seed != config.inference.seed
        || prepared.ledger.requested != config.inference.outer_resamples
        || prepared.ledger.domain != "general_sem_conditional_process_v2"
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "conditional ledger seed, count, domain, or analysis-sample size differs from the recipe"
                .into(),
        ));
    }
    let expected = targets
        .iter()
        .map(|target| target.target_id.clone())
        .collect::<BTreeSet<_>>();
    let usable_indices =
        validate_target_replicates(&prepared.ledger, &prepared.targets, &expected)?;
    let minimum =
        qpls_estimation::minimum_usable_resamples_v2(config.inference.outer_resamples as usize);
    if usable_indices.len() < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "conditional inference has {} usable draws; {minimum} required",
            usable_indices.len()
        )));
    }
    for target in targets {
        let prepared_target = prepared
            .targets
            .iter()
            .find(|candidate| candidate.target_id == target.target_id)
            .expect("target inventory validated above");
        let draws = usable_indices
            .iter()
            .map(|index| {
                prepared_target.estimates[*index].expect("shared usable bitmap validated above")
            })
            .collect::<Vec<_>>();
        let alternative = conditional_alternative(config.inference.alternative);
        let resolved = match config.inference.interval {
            CoreConditionalIntervalV2::Percentile => percentile_interval_v2(
                &draws,
                config.inference.outer_resamples as usize,
                config.inference.confidence_level,
                alternative,
            ),
            CoreConditionalIntervalV2::Bca => {
                if prepared_target.delete_one_jackknife_estimates.len()
                    != prepared.analysis_observations
                {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                        "BCa target {} does not contain the complete delete-one jackknife",
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
            CoreConditionalIntervalV2::Studentized => {
                let observed_se = prepared_target.observed_standard_error.ok_or_else(|| {
                    MultiModRunnerErrorV1::InvalidLedger(format!(
                        "studentized target {} lacks its observed standard error",
                        target.target_id
                    ))
                })?;
                if prepared_target.outer_standard_errors.len()
                    != config.inference.outer_resamples as usize
                {
                    return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
                        "studentized target {} has the wrong outer-SE vector length",
                        target.target_id
                    )));
                }
                let mut outer = Vec::with_capacity(usable_indices.len());
                for index in &usable_indices {
                    let standard_error =
                        prepared_target.outer_standard_errors[*index].ok_or_else(|| {
                            MultiModRunnerErrorV1::InvalidLedger(format!(
                                "studentized target {} is missing a usable outer standard error",
                                target.target_id
                            ))
                        })?;
                    outer.push(StudentizedOuterReplicateV2 {
                        estimate: prepared_target.estimates[*index]
                            .expect("shared usable bitmap validated"),
                        standard_error,
                    });
                }
                for (index, standard_error) in
                    prepared_target.outer_standard_errors.iter().enumerate()
                {
                    let usable = usable_indices.binary_search(&index).is_ok();
                    if usable != standard_error.is_some() {
                        return Err(MultiModRunnerErrorV1::InvalidLedger(
                            "outer standard errors do not share the replicate validity bitmap"
                                .into(),
                        ));
                    }
                }
                studentized_interval_v2(
                    target.estimate,
                    observed_se,
                    &outer,
                    config.inference.outer_resamples as usize,
                    config.inference.confidence_level,
                    alternative,
                )
            }
        }
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        let family = match config.inference.interval {
            CoreConditionalIntervalV2::Percentile => "type_7_percentile",
            CoreConditionalIntervalV2::Bca => "full_delete_one_bca",
            CoreConditionalIntervalV2::Studentized => "nested_studentized",
        };
        target.interval = Some(interval(
            family,
            config.inference.confidence_level,
            config.inference.alternative,
            resolved.lower,
            resolved.upper,
        )?);
        target.p_value = Some(empirical_zero_probability(
            &draws,
            config.inference.alternative,
        ));
        target.usable_replicates = usable_indices.len() as u32;
    }
    Ok(shared_ledger_summary(&prepared.ledger, minimum))
}

pub fn run_compiled_general_sem_conditional_process_v2<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    point: &PreparedConditionalPointBatchV2,
    inference: &PreparedConditionalInferenceV2,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
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
        "conditional_process:authority",
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
    if config.estimands.conditional_total_indirect
        || config.estimands.conditional_total_effect
        || !config.group_contrasts.is_empty()
    {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "multimod.runner.conditional.total_or_group_target_assembler_absent: specific paths are executable, but totals and group contrasts require an explicit complete-model target assembler"
                .into(),
        ));
    }
    validate_conditional_point_batch(config, artifact.plan(), point)?;
    report(
        &progress,
        MultiModRunnerPhaseV1::Resampling,
        0,
        u64::from(config.inference.outer_resamples),
        "conditional_process:shared_ledger",
    );
    let mut targets = point.targets.clone();
    let ledger = apply_conditional_inference(dataset, config, inference, &mut targets)?;
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::Completed,
        1,
        1,
        "conditional_process:complete",
    );
    let analysis = GeneralSemConditionalProcessResultV2 {
        schema_version: GENERAL_SEM_CONDITIONAL_PROCESS_RESULT_V2_SCHEMA_VERSION,
        provenance: provenance(artifact.receipt(), config.inference.seed),
        profile_id: conditional_profile_id(config.profile).into(),
        targets,
        replicate_ledger: ledger,
        sidecars: Vec::new(),
        warnings: point.warnings.clone(),
    };
    Ok(MultiModRunOutputV1 {
        compilation_receipt: artifact.receipt().clone(),
        result: MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(analysis),
        evidence: vec![MultiModRunnerEvidenceV1::ConditionalInference(
            inference.clone(),
        )],
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedInterventionalPathV1 {
    pub path_id: String,
    pub input: InterventionalCausalMediationInputV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedInterventionalBootstrapV1 {
    pub ledger: PreparedSharedReplicateLedgerV1,
    pub targets: Vec<PreparedTargetReplicatesV1>,
    pub complete_observed_equation_refit_per_path_and_replicate: bool,
    pub g_computation_repeated_per_replicate: bool,
    pub fixed_identification_and_positivity_contract: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreparedInterventionalExecutionV1 {
    pub paths: Vec<PreparedInterventionalPathV1>,
    pub bootstrap: PreparedInterventionalBootstrapV1,
}

fn treatment_endpoints(
    contrast: &qpls_core::ObservedTreatmentContrastV1,
) -> (f64, f64, qpls_estimation::ObservedTreatmentKindV1) {
    match contrast {
        qpls_core::ObservedTreatmentContrastV1::Binary { control, treated } => (
            *control,
            *treated,
            qpls_estimation::ObservedTreatmentKindV1::Binary,
        ),
        qpls_core::ObservedTreatmentContrastV1::Continuous { x0, x1 } => (
            *x0,
            *x1,
            qpls_estimation::ObservedTreatmentKindV1::ContinuousContrast,
        ),
    }
}

fn validate_interventional_prepared_path(
    config: &qpls_core::InterventionalCausalMediationConfigV1,
    configured_path: &qpls_core::ObservedCausalPathV1,
    prepared: &PreparedInterventionalPathV1,
) -> Result<(), MultiModRunnerErrorV1> {
    let input = &prepared.input;
    let (x0, x1, kind) = treatment_endpoints(&config.treatment_contrast);
    let expected_mediators = configured_path.ordered_variable_ids
        [1..configured_path.ordered_variable_ids.len() - 1]
        .to_vec();
    let mut expected_adjustment = config.adjustment_covariates.clone();
    let mut actual_adjustment = input.adjustment_covariate_variable_ids.clone();
    expected_adjustment.sort();
    actual_adjustment.sort();
    let mut expected_moderators = config.baseline_moderators.clone();
    let mut actual_moderators = input.baseline_moderator_variable_ids.clone();
    expected_moderators.sort();
    actual_moderators.sort();
    let equations_match = input.equations.len() == configured_path.equations.len()
        && input
            .equations
            .iter()
            .zip(&configured_path.equations)
            .all(|(actual, expected)| {
                actual.equation_id == expected.equation_id
                    && actual.outcome_variable_id == expected.outcome_variable_id
                    && actual.terms.len() == expected.terms.len()
                    && actual.terms.iter().zip(&expected.terms).all(
                        |(actual_term, expected_term)| {
                            actual_term.term_id == expected_term.term_id
                                && actual_term.factor_variable_ids
                                    == expected_term.factor_variable_ids
                        },
                    )
            });
    let positivity_matches = input.positivity_policy.minimum_binary_arm_count
        == config.positivity_policy.minimum_binary_arm_count as usize
        && input.positivity_policy.maximum_binary_arm_ratio.to_bits()
            == config.positivity_policy.maximum_binary_arm_ratio.to_bits()
        && input.positivity_policy.positivity_strata_variable_ids
            == config.positivity_policy.positivity_strata_variable_ids
        && input.positivity_policy.minimum_count_per_binary_stratum_arm
            == config
                .positivity_policy
                .minimum_count_per_binary_stratum_arm as usize
        && input
            .positivity_policy
            .continuous_neighborhood_fraction_of_range
            .to_bits()
            == config
                .positivity_policy
                .continuous_neighborhood_fraction_of_range
                .to_bits()
        && input
            .positivity_policy
            .minimum_continuous_neighborhood_count
            == config
                .positivity_policy
                .minimum_continuous_neighborhood_count as usize;
    if prepared.path_id != configured_path.path_id
        || input.analysis_id != configured_path.path_id
        || input.treatment_variable_id != config.treatment
        || input.outcome_variable_id != config.outcome
        || input.ordered_mediator_variable_ids != expected_mediators
        || actual_adjustment != expected_adjustment
        || actual_moderators != expected_moderators
        || input.treatment_contrast.kind != kind
        || input.treatment_contrast.x0.to_bits() != x0.to_bits()
        || input.treatment_contrast.x1.to_bits() != x1.to_bits()
        || !equations_match
        || !positivity_matches
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(format!(
            "prepared observed-data input differs from causal path {}",
            configured_path.path_id
        )));
    }
    if !input.identification_checklist.temporal_order_reviewed
        || !input.identification_checklist.consistency_reviewed
        || !input
            .identification_checklist
            .treatment_outcome_exchangeability_reviewed
        || !input
            .identification_checklist
            .treatment_mediator_exchangeability_reviewed
        || !input
            .identification_checklist
            .mediator_outcome_exchangeability_reviewed
        || !input
            .identification_checklist
            .no_exposure_induced_mediator_outcome_confounder_reviewed
        || !input.identification_checklist.no_recanting_witness_reviewed
        || !input
            .identification_checklist
            .linear_model_specification_reviewed
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "the observed-data engine identification checklist is incomplete".into(),
        ));
    }
    if input.unsupported_features.latent_composite_or_hoc_roles
        || input.unsupported_features.groups
        || input.unsupported_features.weights
        || input.unsupported_features.natural_or_cross_world_effects
        || input
            .unsupported_features
            .exposure_induced_mediator_outcome_confounder_present
        || input.unsupported_features.recanting_witness_present
    {
        return Err(MultiModRunnerErrorV1::UnsupportedProfile(
            "causal V1 excludes latent roles, groups, weights, cross-world effects, exposure-induced confounding, and recanting witnesses"
                .into(),
        ));
    }
    Ok(())
}

fn causal_effect_target_id(path_id: &str, estimand: &str) -> String {
    format!("qpls.interventional.v1:{path_id}:{estimand}")
}

pub(crate) fn causal_effects(
    path_id: &str,
    result: &EstimationInterventionalMediationResultV1,
) -> Vec<InterventionalEffectResultV1> {
    [
        (
            "interventional_direct_effect",
            result.interventional_direct_effect,
        ),
        (
            "joint_interventional_indirect_effect",
            result.joint_interventional_indirect_effect,
        ),
        (
            "total_interventional_contrast",
            result.total_interventional_contrast,
        ),
    ]
    .into_iter()
    .map(|(estimand, estimate)| InterventionalEffectResultV1 {
        target_id: causal_effect_target_id(path_id, estimand),
        path_id: path_id.into(),
        estimand: estimand.into(),
        estimate,
        p_value: None,
        interval: None,
    })
    .collect()
}

fn apply_interventional_bootstrap(
    config: &qpls_core::InterventionalCausalMediationConfigV1,
    prepared: &PreparedInterventionalBootstrapV1,
    effects: &mut [InterventionalEffectResultV1],
) -> Result<MultimodReplicateLedgerSummaryV1, MultiModRunnerErrorV1> {
    if !prepared.complete_observed_equation_refit_per_path_and_replicate
        || !prepared.g_computation_repeated_per_replicate
        || !prepared.fixed_identification_and_positivity_contract
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "causal bootstrap must refit every observed equation and rerun g-computation under the fixed declared contract"
                .into(),
        ));
    }
    if prepared.ledger.master_seed != config.seed
        || prepared.ledger.requested != config.bootstrap_resamples
        || prepared.ledger.domain != "interventional_causal_mediation_v1"
    {
        return Err(MultiModRunnerErrorV1::InvalidLedger(
            "causal bootstrap seed, count, or domain differs from the recipe".into(),
        ));
    }
    let expected = effects
        .iter()
        .map(|effect| effect.target_id.clone())
        .collect::<BTreeSet<_>>();
    let usable_indices =
        validate_target_replicates(&prepared.ledger, &prepared.targets, &expected)?;
    let minimum = qpls_estimation::minimum_usable_resamples_v2(config.bootstrap_resamples as usize);
    if usable_indices.len() < minimum {
        return Err(MultiModRunnerErrorV1::InvalidLedger(format!(
            "causal bootstrap has {} usable draws; {minimum} required",
            usable_indices.len()
        )));
    }
    for effect in effects {
        let target = prepared
            .targets
            .iter()
            .find(|target| target.target_id == effect.target_id)
            .expect("target inventory validated above");
        let draws = usable_indices
            .iter()
            .map(|index| target.estimates[*index].expect("shared validity bitmap validated"))
            .collect::<Vec<_>>();
        let inferred = percentile_interval_v2(
            &draws,
            config.bootstrap_resamples as usize,
            config.confidence_level,
            ConditionalAlternativeV2::TwoSided,
        )
        .map_err(|error| MultiModRunnerErrorV1::Kernel(error.to_string()))?;
        effect.interval = Some(interval(
            "type_7_two_sided_percentile",
            config.confidence_level,
            InferenceAlternativeV1::TwoSided,
            inferred.lower,
            inferred.upper,
        )?);
        effect.p_value = Some(empirical_zero_probability(
            &draws,
            InferenceAlternativeV1::TwoSided,
        ));
    }
    Ok(shared_ledger_summary(&prepared.ledger, minimum))
}

pub fn run_compiled_interventional_causal_mediation_v1<C, P>(
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    artifact: &CompiledMultiModRecipeV1,
    prepared: &PreparedInterventionalExecutionV1,
    should_cancel: C,
    progress: P,
) -> Result<MultiModRunOutputV1, MultiModRunnerErrorV1>
where
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
        "interventional:authority",
    );
    validate_authority(
        dataset,
        recipe,
        model,
        artifact,
        MultiModCompilerTargetV1::InterventionalCausalMediationV1,
    )?;
    let config = recipe
        .interventional_causal_mediation
        .as_ref()
        .ok_or_else(|| {
            MultiModRunnerErrorV1::Authority(
                "interventional configuration disappeared after compilation".into(),
            )
        })?;
    let CompiledMultiModPlanV1::InterventionalCausalMediationV1 { path_ids, .. } = artifact.plan()
    else {
        return Err(MultiModRunnerErrorV1::Authority(
            "compiled plan is not interventional causal mediation V1".into(),
        ));
    };
    if prepared.paths.len() != config.paths.len()
        || prepared
            .paths
            .iter()
            .map(|path| path.path_id.as_str())
            .collect::<Vec<_>>()
            != path_ids.iter().map(String::as_str).collect::<Vec<_>>()
    {
        return Err(MultiModRunnerErrorV1::PreparedInput(
            "prepared causal paths differ from the exact compiled selection".into(),
        ));
    }
    let mut effects = Vec::new();
    let mut positivity = Vec::new();
    for (index, (configured_path, prepared_path)) in
        config.paths.iter().zip(&prepared.paths).enumerate()
    {
        if should_cancel() {
            return Err(MultiModRunnerErrorV1::Cancelled);
        }
        report(
            &progress,
            MultiModRunnerPhaseV1::PointEstimation,
            index as u64,
            config.paths.len() as u64,
            format!("interventional:path:{}", configured_path.path_id),
        );
        validate_interventional_prepared_path(config, configured_path, prepared_path)?;
        let result = estimate_interventional_mediation_v1(&prepared_path.input)
            .map_err(|blockers| MultiModRunnerErrorV1::Kernel(format!("{blockers:?}")))?;
        if result.interpretation != qpls_estimation::INTERVENTIONAL_MEDIATION_INTERPRETATION_V1
            || !result.additive_decomposition_residual.is_finite()
            || result.additive_decomposition_residual.abs() > 1.0e-8
        {
            return Err(MultiModRunnerErrorV1::Kernel(
                "interventional engine returned drifted wording or decomposition".into(),
            ));
        }
        effects.extend(causal_effects(&configured_path.path_id, &result));
        let treatment_column = prepared_path
            .input
            .columns
            .iter()
            .find(|column| column.variable_id == config.treatment)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput(
                    "prepared causal input lacks its treatment column".into(),
                )
            })?;
        let observed_minimum = treatment_column
            .values
            .iter()
            .copied()
            .min_by(f64::total_cmp)
            .ok_or_else(|| {
                MultiModRunnerErrorV1::PreparedInput("treatment column is empty".into())
            })?;
        let observed_maximum = treatment_column
            .values
            .iter()
            .copied()
            .max_by(f64::total_cmp)
            .expect("nonempty treatment column");
        let (minimum_required_count, support_rule) = match config.treatment_contrast {
            qpls_core::ObservedTreatmentContrastV1::Binary { .. } => (
                u64::from(config.positivity_policy.minimum_binary_arm_count),
                "binary_arm_count_and_declared_strata",
            ),
            qpls_core::ObservedTreatmentContrastV1::Continuous { .. } => (
                u64::from(
                    config
                        .positivity_policy
                        .minimum_continuous_neighborhood_count,
                ),
                "continuous_neighborhood_count",
            ),
        };
        for (label, requested_value, support_count) in [
            ("x0", result.x0, result.positivity.x0_support_count as u64),
            ("x1", result.x1, result.positivity.x1_support_count as u64),
        ] {
            positivity.push(qpls_core::CausalPositivityDiagnosticV1 {
                variable_id: format!("{}:{}:{}", configured_path.path_id, config.treatment, label),
                observed_minimum,
                observed_maximum,
                requested_value,
                support_count,
                minimum_required_count,
                support_rule: support_rule.into(),
                supported: requested_value >= observed_minimum
                    && requested_value <= observed_maximum
                    && support_count >= minimum_required_count,
            });
        }
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::Resampling,
        0,
        u64::from(config.bootstrap_resamples),
        "interventional:shared_ledger",
    );
    let ledger = apply_interventional_bootstrap(config, &prepared.bootstrap, &mut effects)?;
    if should_cancel() {
        return Err(MultiModRunnerErrorV1::Cancelled);
    }
    report(
        &progress,
        MultiModRunnerPhaseV1::Completed,
        1,
        1,
        "interventional:complete",
    );
    let analysis = CoreInterventionalMediationResultV1 {
        schema_version: INTERVENTIONAL_MEDIATION_RESULT_V1_SCHEMA_VERSION,
        provenance: provenance(artifact.receipt(), config.seed),
        interpretation_label: INTERVENTIONAL_MEDIATION_RESULT_INTERPRETATION_LABEL_V1.into(),
        identification_assumptions: vec![
            "temporal order and consistency were declared".into(),
            "the adjustment set was declared sufficient for treatment-outcome, treatment-mediator, and mediator-outcome exchangeability".into(),
            "no exposure-induced mediator-outcome confounder or recanting witness was declared".into(),
            "positivity and the observed linear equation specification were reviewed".into(),
        ],
        positivity,
        effects,
        replicate_ledger: ledger,
        sidecars: Vec::new(),
    };
    Ok(MultiModRunOutputV1 {
        compilation_receipt: artifact.receipt().clone(),
        result: MultiModAnalysisResultV1::InterventionalMediationResultV1(analysis),
        evidence: vec![MultiModRunnerEvidenceV1::InterventionalBootstrap(
            prepared.bootstrap.clone(),
        )],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_estimation::{ConditionalLinearCoefficientV2, GroupIdentityV1, TypedGroupValueV1};

    #[test]
    fn raw_mga_row_tokens_are_stable_across_row_and_column_reordering() {
        fn dataset(source: &str, name: &str) -> Dataset {
            qpls_data::import_delimited_bytes(
                source.as_bytes(),
                name,
                b',',
                &qpls_data::ImportOptions::default(),
            )
            .unwrap()
        }
        fn design(dataset: &Dataset) -> MultigroupDesignV1 {
            let preview = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
            MultigroupDesignV1 {
                groups: ["A", "B"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| GroupIdentityV1 {
                        index: GroupIndexV1::new(index).unwrap(),
                        value: TypedGroupValueV1::Text {
                            value: value.into(),
                        },
                        display_label: value.into(),
                    })
                    .collect(),
                rows: preview
                    .iter()
                    .enumerate()
                    .map(|(source_row, values)| SelectedGroupRowV1 {
                        source_row: source_row as u64,
                        stable_row_token: source_row as u64,
                        group: GroupIndexV1::new(usize::from(
                            values["group"].as_deref() == Some("B"),
                        ))
                        .unwrap(),
                    })
                    .collect(),
            }
        }
        fn tokens_by_x(
            dataset: &Dataset,
            design: &MultigroupDesignV1,
        ) -> BTreeMap<String, (u64, u64)> {
            let preview = qpls_data::preview_page(dataset, 0, dataset.batch.num_rows());
            design
                .rows
                .iter()
                .map(|row| {
                    (
                        preview[row.source_row as usize]["x"].clone().unwrap(),
                        (row.stable_row_token, row.source_row),
                    )
                })
                .collect()
        }

        let scientific_columns = vec!["group".into(), "x".into(), "y".into()];
        let baseline = dataset(
            "group,x,y,unused\nA,1,11,left\nA,2,12,left\nB,3,13,left\nB,4,14,left\n",
            "mga-token-baseline.csv",
        );
        let reversed = dataset(
            "group,x,y,unused\nB,4,14,right\nB,3,13,right\nA,2,12,right\nA,1,11,right\n",
            "mga-token-row-reversed.csv",
        );
        let columns_reversed = dataset(
            "unused,y,x,group\nchanged,11,1,A\nchanged,12,2,A\nchanged,13,3,B\nchanged,14,4,B\n",
            "mga-token-columns-reversed.csv",
        );
        let baseline_bound = bind_mga_stable_row_tokens_for_columns_v1(
            &baseline,
            &scientific_columns,
            &design(&baseline),
        )
        .unwrap();
        let reversed_bound = bind_mga_stable_row_tokens_for_columns_v1(
            &reversed,
            &scientific_columns,
            &design(&reversed),
        )
        .unwrap();
        let columns_bound = bind_mga_stable_row_tokens_for_columns_v1(
            &columns_reversed,
            &["y".into(), "group".into(), "x".into()],
            &design(&columns_reversed),
        )
        .unwrap();
        let baseline_tokens = tokens_by_x(&baseline, &baseline_bound);
        let reversed_tokens = tokens_by_x(&reversed, &reversed_bound);
        let columns_tokens = tokens_by_x(&columns_reversed, &columns_bound);
        assert_eq!(
            baseline_tokens
                .iter()
                .map(|(x, (token, _))| (x, token))
                .collect::<BTreeMap<_, _>>(),
            reversed_tokens
                .iter()
                .map(|(x, (token, _))| (x, token))
                .collect::<BTreeMap<_, _>>()
        );
        assert_eq!(
            baseline_tokens
                .iter()
                .map(|(x, (token, _))| (x, token))
                .collect::<BTreeMap<_, _>>(),
            columns_tokens
                .iter()
                .map(|(x, (token, _))| (x, token))
                .collect::<BTreeMap<_, _>>()
        );
        for (x, (_, source_row)) in &baseline_tokens {
            assert_eq!(*source_row + reversed_tokens[x].1, 3);
        }
    }

    fn observed_control_lowering_model_v1() -> SemModelV4 {
        let legacy = qpls_core::ModelSpec {
            id: uuid::Uuid::from_u128(0x4d47_415f_434f_4e54_524f_4c00_0001),
            name: "MGA observed-control lowering".into(),
            constructs: vec![
                qpls_core::Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: qpls_core::MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                qpls_core::Construct {
                    id: "y".into(),
                    name: "Y".into(),
                    short_name: "Y".into(),
                    mode: qpls_core::MeasurementMode::Reflective,
                    indicators: vec!["y1".into(), "y2".into()],
                },
            ],
            paths: vec![qpls_core::StructuralPath {
                source: "x".into(),
                target: "y".into(),
            }],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = qpls_core::convert_legacy_basic_model_v4(
            &legacy,
            qpls_core::LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        model.variables.push(SemVariableV4::Observed {
            id: "observed:qualification_control".into(),
            label: "Qualification control".into(),
            source_column: "w1".into(),
            scale: ObservedScaleV4::Continuous,
            role: ObservedRoleV4::Control,
            categories: Vec::new(),
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "relation:qualification_control:to:y".into(),
            source: "observed:qualification_control".into(),
            target: "construct:y".into(),
            parameter: "parameter:qualification_control:to:y".into(),
            role: StructuralRelationRoleV4::Control,
            intercept_parameter: None,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:qualification_control:to:y".into(),
            label: "Qualification control -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "observed:qualification_control".into(),
                target: "construct:y".into(),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.ensure_valid().unwrap();
        model
    }

    fn multimod_point_authority_fixture_v1(
        weight: Option<qpls_core::SemWeightBindingV4>,
        composite_control: bool,
    ) -> (Dataset, SemModelV4, AnalysisRecipeV4) {
        let source = include_str!("../../../validation/results/plsc_reference.csv");
        let weighted_source = weight.as_ref().map(|_| {
            let mut rows = String::new();
            for (index, row) in source.lines().enumerate() {
                rows.push_str(row);
                rows.push_str(if index == 0 { ",w\n" } else { ",1\n" });
            }
            rows
        });
        let bytes = weighted_source.as_deref().unwrap_or(source).as_bytes();
        let dataset = qpls_data::import_delimited_bytes(
            bytes,
            "multimod-point-authority.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let legacy: qpls_core::AnalysisRecipe = serde_json::from_slice(include_bytes!(
            "../../../validation/results/plsc_reference.recipe.json"
        ))
        .unwrap();
        let mut model = qpls_core::convert_legacy_basic_model_v4(
            &legacy.model,
            qpls_core::LegacyBasicModelInterpretationV4::PlsComposite,
            &[],
        )
        .unwrap();
        if composite_control {
            let relation = model
                .relations
                .iter_mut()
                .find(|relation| {
                    matches!(relation, SemRelationV4::Structural { source, target, .. }
                        if source == "construct:z" && target == "construct:y")
                })
                .unwrap();
            let SemRelationV4::Structural { role, .. } = relation else {
                unreachable!()
            };
            *role = StructuralRelationRoleV4::Control;
        }
        if weight.is_some() {
            model.variables.push(SemVariableV4::Observed {
                id: "observed:weight".into(),
                label: "Weight".into(),
                source_column: "w".into(),
                scale: ObservedScaleV4::Continuous,
                role: ObservedRoleV4::Control,
                categories: Vec::new(),
                value_labels: BTreeMap::new(),
                missing_markers: Vec::new(),
                transformation_lineage: Vec::new(),
            });
        }
        model.data_binding = qpls_core::SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: qpls_core::MissingDataPolicyV4::ListwiseDeletion,
            weight,
            cluster_variable: None,
            strata_variable: None,
        };
        model.ensure_valid().unwrap();
        let scientific_sha256 = model.scientific_sha256().unwrap();
        let weighted = matches!(
            &model.data_binding,
            qpls_core::SemDataBindingV4::Raw {
                weight: Some(_),
                ..
            }
        );
        let recipe = AnalysisRecipeV4 {
            schema_version: qpls_core::ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: uuid::Uuid::from_u128(if weighted { 0x5750_4c53 } else { 0x504c_5343 }),
            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                scientific_sha256,
                model: model.clone(),
            },
            estimand_confirmation: qpls_core::LegacyEstimandConfirmationV4::NotLegacy,
            settings: qpls_core::AnalysisSettings {
                method: if weighted {
                    AnalysisMethod::Wpls
                } else {
                    AnalysisMethod::PlsPm
                },
                case_weight_column: weighted.then(|| "w".to_owned()),
                ..qpls_core::AnalysisSettings::default()
            },
            method_config: Some(if weighted {
                MethodConfig::Wpls
            } else {
                MethodConfig::PlsAlgorithm
            }),
            general_sem_config: None,
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        recipe.ensure_valid().unwrap();
        (dataset, model, recipe)
    }

    fn attach_two_group_mga_v1(
        model: &mut SemModelV4,
        recipe: &mut AnalysisRecipeV4,
        profile: qpls_core::MgaModelProfileV1,
        weight: Option<qpls_core::AnalysisWeightBindingV1>,
    ) {
        model.variables.push(SemVariableV4::Observed {
            id: "observed:group".into(),
            label: "Group".into(),
            source_column: "group".into(),
            scale: ObservedScaleV4::Nominal,
            role: ObservedRoleV4::Control,
            categories: vec!["a".into(), "b".into()],
            value_labels: BTreeMap::new(),
            missing_markers: Vec::new(),
            transformation_lineage: Vec::new(),
        });
        model.group = SemGroupV4::ObservedGroups {
            grouping_variable: "observed:group".into(),
            levels: vec![
                qpls_core::SemGroupLevelV4 {
                    id: "a".into(),
                    value: "a".into(),
                    label: "A".into(),
                },
                qpls_core::SemGroupLevelV4 {
                    id: "b".into(),
                    value: "b".into(),
                    label: "B".into(),
                },
            ],
        };
        model.ensure_valid().unwrap();
        recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            scientific_sha256: model.scientific_sha256().unwrap(),
            model: model.clone(),
        };
        recipe.settings.method = AnalysisMethod::Mga;
        recipe.method_config = None;
        recipe.mga_multigroup = Some(MgaMultigroupV1 {
            schema_version: qpls_core::MGA_MULTIGROUP_V1_SCHEMA_VERSION,
            profile,
            grouping_column: "group".into(),
            groups: vec![
                qpls_core::SelectedGroupV1 {
                    group_id: "a".into(),
                    label: "A".into(),
                    value: qpls_core::TypedGroupValueV1::Text { value: "a".into() },
                },
                qpls_core::SelectedGroupV1 {
                    group_id: "b".into(),
                    label: "B".into(),
                    value: qpls_core::TypedGroupValueV1::Text { value: "b".into() },
                },
            ],
            comparison_plan: qpls_core::MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: false,
            },
            procedures: vec![MgaProcedureV1::PairwisePermutation],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: qpls_core::MicomConfiguralChecklistV1 {
                identical_indicators_and_coding: true,
                identical_data_treatment: true,
                identical_algorithm_settings: true,
                identical_model_specification: true,
                deterministic_sign_orientation_reviewed: true,
                analyst_review_confirmed: true,
            },
            weight,
            selected_parameter_ids: Vec::new(),
        });
        recipe.ensure_valid().unwrap();
    }

    #[test]
    fn multimod_plsc_authority_compiles_plspm_but_executes_plsc_v2_with_exact_inventories() {
        let (dataset, mut model, mut recipe) = multimod_point_authority_fixture_v1(None, true);
        attach_two_group_mga_v1(
            &mut model,
            &mut recipe,
            qpls_core::MgaModelProfileV1::ReflectivePlsc,
            None,
        );
        let authority = projected_ordinary_pls_authority_v1(
            &recipe,
            &model,
            recipe.mga_multigroup.as_ref().unwrap(),
        )
        .unwrap();
        let target = RecipeV4CompilerTarget::PlsPlanV2;
        let mut forbidden_direct = authority.point_recipe().clone();
        forbidden_direct.settings.method = AnalysisMethod::Plsc;
        forbidden_direct.method_config = Some(MethodConfig::Plsc);
        assert!(
            compile_analysis_recipe_v4(
                &forbidden_direct,
                Some(authority.point_model()),
                target,
                target.capability_cell_for_recipe(&forbidden_direct),
            )
            .is_err()
        );
        assert_eq!(
            authority.point_recipe().settings.method,
            AnalysisMethod::PlsPm
        );
        assert!(matches!(
            &authority.execution,
            OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { .. }
        ));
        assert!(authority.repeats_plsc_correction());
        let OrdinaryPlsPointExecutionAuthorityV1::ReflectivePlsc { point_artifact, .. } =
            &authority.execution
        else {
            unreachable!("the authority variant was asserted above")
        };
        assert_eq!(
            qpls_core::sha256_serialized(point_artifact.plan()),
            point_artifact.receipt().plan_sha256()
        );
        assert_eq!(
            authority.point_model().scientific_sha256().unwrap(),
            point_artifact.receipt().model_scientific_sha256()
        );
        let result = authority.execute(&dataset, &|| false).unwrap();
        assert_eq!(result.method_version, PLSC_METHOD_VERSION);
        assert_eq!(result.plsc.as_ref().unwrap().corrected_paths, result.paths);
        let expected_paths = authority
            .plan()
            .paths()
            .iter()
            .map(|path| (path.source(), path.target()))
            .collect::<BTreeSet<_>>();
        let actual_paths = result
            .paths
            .iter()
            .map(|path| (path.source.as_str(), path.target.as_str()))
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_paths, expected_paths);
        assert_eq!(result.control_estimates.len(), 1);
    }

    #[test]
    fn multimod_weighted_authority_is_prepared_once_and_routes_case_and_frequency_points() {
        for (resident_weight, requested_weight) in [
            (
                qpls_core::SemWeightBindingV4::Case {
                    variable: "observed:weight".into(),
                },
                qpls_core::AnalysisWeightBindingV1::Case { column: "w".into() },
            ),
            (
                qpls_core::SemWeightBindingV4::Case {
                    variable: "observed:weight".into(),
                },
                qpls_core::AnalysisWeightBindingV1::Case {
                    column: "observed:weight".into(),
                },
            ),
            (
                qpls_core::SemWeightBindingV4::Frequency {
                    variable: "observed:weight".into(),
                },
                qpls_core::AnalysisWeightBindingV1::Frequency { column: "w".into() },
            ),
            (
                qpls_core::SemWeightBindingV4::Frequency {
                    variable: "observed:weight".into(),
                },
                qpls_core::AnalysisWeightBindingV1::Frequency {
                    column: "observed:weight".into(),
                },
            ),
        ] {
            let (dataset, mut model, mut recipe) =
                multimod_point_authority_fixture_v1(Some(resident_weight), false);
            let profile = match &requested_weight {
                qpls_core::AnalysisWeightBindingV1::Case { .. } => {
                    qpls_core::MgaModelProfileV1::CaseWeightedPls
                }
                qpls_core::AnalysisWeightBindingV1::Frequency { .. } => {
                    qpls_core::MgaModelProfileV1::FrequencyWeightedPls
                }
            };
            attach_two_group_mga_v1(
                &mut model,
                &mut recipe,
                profile,
                Some(requested_weight.clone()),
            );
            let authority = projected_ordinary_pls_authority_v1(
                &recipe,
                &model,
                recipe.mga_multigroup.as_ref().unwrap(),
            )
            .unwrap();
            assert!(matches!(
                &authority.execution,
                OrdinaryPlsPointExecutionAuthorityV1::Weighted { .. }
            ));
            assert_eq!(
                authority.point_recipe().settings.method,
                AnalysisMethod::PlsPm
            );
            let OrdinaryPlsPointExecutionAuthorityV1::Weighted { prepared, .. } =
                &authority.execution
            else {
                unreachable!("the authority variant was asserted above")
            };
            let compiled_authority = prepared.compiled_authority_for_test();
            assert_eq!(authority.plan(), compiled_authority.plan());
            assert_eq!(
                qpls_core::sha256_serialized(compiled_authority.point_artifact().plan()),
                compiled_authority.point_artifact().receipt().plan_sha256()
            );
            assert_eq!(
                authority.point_model().scientific_sha256().unwrap(),
                compiled_authority.receipt().point_model_scientific_sha256()
            );
            assert!(matches!(
                &authority.point_model().data_binding,
                qpls_core::SemDataBindingV4::Raw { weight: None, .. }
            ));
            assert_eq!(authority.weight_source_column(), Some("w"));
            let blocks = ordinary_pls_scoring_blocks_v1(authority.plan());
            let mut source_columns = ordinary_pls_source_columns_v1(&dataset, &blocks).unwrap();
            source_columns.push(authority.weight_source_column().unwrap().to_owned());
            let all_rows = (0..dataset.batch.num_rows() as u64).collect::<Vec<_>>();
            let result = match profile {
                qpls_core::MgaModelProfileV1::CaseWeightedPls => pooled_mga_point_fit_v1(
                    &dataset,
                    &source_columns,
                    &all_rows,
                    &authority,
                    profile,
                    authority.weight_source_column(),
                    &|| false,
                )
                .unwrap(),
                qpls_core::MgaModelProfileV1::FrequencyWeightedPls => {
                    let compact_rows = (0..9)
                        .map(|index| index as u64 * (dataset.batch.num_rows() as u64 - 1) / 8)
                        .collect::<Vec<_>>();
                    run_frequency_pls_sample_v1(
                        &dataset,
                        &source_columns,
                        &compact_rows,
                        &vec![2; compact_rows.len()],
                        authority.weight_source_column().unwrap(),
                        &authority,
                        &|| false,
                    )
                    .unwrap()
                }
                _ => unreachable!(),
            };
            assert_eq!(result.method_version, qpls_estimation::WPLS_METHOD_VERSION);
            assert!(result.wpls.is_some());
            assert!(result.plsc.is_none());
        }
    }

    #[test]
    fn mga_observed_control_lowering_preserves_scientific_ids_and_hides_technical_targets() {
        let source = observed_control_lowering_model_v1();
        let source_relation = source
            .relations
            .iter()
            .find(|relation| relation.id() == "relation:qualification_control:to:y")
            .unwrap()
            .clone();
        let source_parameter = source
            .parameters
            .iter()
            .find(|parameter| parameter.id() == "parameter:qualification_control:to:y")
            .unwrap()
            .clone();
        let mut lowered = source.clone();
        let technical = lower_observed_controls_for_mga_pls_v1(
            &mut lowered,
            qpls_core::MgaModelProfileV1::GeneralSemPls,
            "observed:group",
            "group",
        )
        .unwrap();
        assert_eq!(
            technical,
            BTreeSet::from(["observed:qualification_control".to_owned()])
        );
        assert!(matches!(
            source
                .variables
                .iter()
                .find(|variable| variable.id() == "observed:qualification_control"),
            Some(SemVariableV4::Observed {
                source_column,
                role: ObservedRoleV4::Control,
                ..
            }) if source_column == "w1"
        ));
        assert!(matches!(
            lowered
                .variables
                .iter()
                .find(|variable| variable.id() == "observed:qualification_control"),
            Some(SemVariableV4::Composite {
                weighting: CompositeWeightingV4::Unit {
                    normalization: CompositeWeightNormalizationV4::None,
                },
                ..
            })
        ));
        let generated =
            mga_observed_control_lowering_ids_v1("observed:qualification_control", "w1");
        assert!(matches!(
            lowered
                .variables
                .iter()
                .find(|variable| variable.id() == generated.indicator_id),
            Some(SemVariableV4::Observed {
                source_column,
                role: ObservedRoleV4::Indicator,
                ..
            }) if source_column == "w1"
        ));
        assert_eq!(
            lowered
                .relations
                .iter()
                .find(|relation| relation.id() == "relation:qualification_control:to:y")
                .unwrap(),
            &source_relation
        );
        assert_eq!(
            lowered
                .parameters
                .iter()
                .find(|parameter| parameter.id() == "parameter:qualification_control:to:y")
                .unwrap(),
            &source_parameter
        );
        lowered.ensure_valid().unwrap();
        let plan = qpls_core::compile_pls_plan_v2(&lowered).unwrap();
        let technical_block = plan
            .blocks()
            .iter()
            .find(|block| block.construct_id() == "observed:qualification_control")
            .unwrap();
        assert_eq!(
            technical_block.fixed_scoring(),
            Some(&qpls_core::CompiledPlsFixedScoringV2::Unit {
                normalization: CompositeWeightNormalizationV4::None,
            })
        );
        let blocks = ordinary_pls_scoring_blocks_v1(&plan);
        let micom_construct_ids = ordinary_pls_micom_construct_ids_v1(&blocks, &technical);
        assert!(!micom_construct_ids.contains(&"observed:qualification_control".to_owned()));
        assert_eq!(
            micom_construct_ids.into_iter().collect::<BTreeSet<_>>(),
            BTreeSet::from(["construct:x".to_owned(), "construct:y".to_owned(),])
        );
        let projections = synthetic_ordinary_pls_parameter_projections_v1(&plan, &technical);
        assert!(!projections.iter().any(|projection| matches!(
            &projection.source,
            OrdinaryPlsParameterSourceV1::OuterLoading { construct, .. }
                | OrdinaryPlsParameterSourceV1::OuterWeight { construct, .. }
                if technical.contains(construct)
        )));
        let control = projections
            .iter()
            .find(|projection| {
                projection.identity.stable_id == "parameter:qualification_control:to:y"
            })
            .unwrap();
        assert_eq!(
            control.required_constructs(),
            BTreeSet::from(["construct:y".to_owned()])
        );
    }

    #[test]
    fn mga_observed_control_lowering_fails_closed_for_unsupported_semantics_and_collisions() {
        let mut weighted = observed_control_lowering_model_v1();
        assert!(matches!(
            lower_observed_controls_for_mga_pls_v1(
                &mut weighted,
                qpls_core::MgaModelProfileV1::CaseWeightedPls,
                "observed:group",
                "group",
            ),
            Err(MultiModRunnerErrorV1::UnsupportedProfile(code))
                if code == "multimod.runner.mga.observed_control_general_sem_pls_only"
        ));
        let mut plsc = observed_control_lowering_model_v1();
        assert!(matches!(
            lower_observed_controls_for_mga_pls_v1(
                &mut plsc,
                qpls_core::MgaModelProfileV1::ReflectivePlsc,
                "observed:group",
                "group",
            ),
            Err(MultiModRunnerErrorV1::UnsupportedProfile(code))
                if code == "multimod.runner.mga.observed_control_general_sem_pls_only"
        ));

        let mut categorical = observed_control_lowering_model_v1();
        let SemVariableV4::Observed {
            scale, categories, ..
        } = categorical
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "observed:qualification_control")
            .unwrap()
        else {
            unreachable!()
        };
        *scale = ObservedScaleV4::Binary;
        *categories = vec!["0".into(), "1".into()];
        categorical.ensure_valid().unwrap();
        assert!(matches!(
            lower_observed_controls_for_mga_pls_v1(
                &mut categorical,
                qpls_core::MgaModelProfileV1::GeneralSemPls,
                "observed:group",
                "group",
            ),
            Err(MultiModRunnerErrorV1::UnsupportedProfile(code))
                if code.starts_with("multimod.runner.mga.observed_control_scale_or_metadata_unsupported:")
        ));

        let mut wrong_role = observed_control_lowering_model_v1();
        let SemVariableV4::Observed { role, .. } = wrong_role
            .variables
            .iter_mut()
            .find(|variable| variable.id() == "observed:qualification_control")
            .unwrap()
        else {
            unreachable!()
        };
        *role = ObservedRoleV4::Structural;
        wrong_role.ensure_valid().unwrap();
        assert!(matches!(
            lower_observed_controls_for_mga_pls_v1(
                &mut wrong_role,
                qpls_core::MgaModelProfileV1::GeneralSemPls,
                "observed:group",
                "group",
            ),
            Err(MultiModRunnerErrorV1::UnsupportedProfile(code))
                if code.starts_with("multimod.runner.mga.observed_control_role_or_relation_unsupported:")
        ));

        let mut mixed = observed_control_lowering_model_v1();
        mixed.relations.push(SemRelationV4::Covariance {
            id: "relation:qualification_control:with:x".into(),
            left: qpls_core::SemEndpointV4::Variable("observed:qualification_control".into()),
            right: qpls_core::SemEndpointV4::Variable("construct:x".into()),
            parameter: "parameter:qualification_control:with:x".into(),
        });
        mixed.parameters.push(SemParameterV4::Free {
            id: "parameter:qualification_control:with:x".into(),
            label: "Qualification control with X".into(),
            target: SemParameterTargetV4::Covariance {
                left: qpls_core::SemEndpointV4::Variable("observed:qualification_control".into()),
                right: qpls_core::SemEndpointV4::Variable("construct:x".into()),
            },
            start: None,
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        mixed.ensure_valid().unwrap();
        assert!(matches!(
            lower_observed_controls_for_mga_pls_v1(
                &mut mixed,
                qpls_core::MgaModelProfileV1::GeneralSemPls,
                "observed:group",
                "group",
            ),
            Err(MultiModRunnerErrorV1::UnsupportedProfile(code))
                if code.starts_with("multimod.runner.mga.observed_control_mixed_use_unsupported:")
        ));

        let mut collision = observed_control_lowering_model_v1();
        let generated =
            mga_observed_control_lowering_ids_v1("observed:qualification_control", "w1");
        let SemRelationV4::MeasurementEffect { id, .. } = collision
            .relations
            .iter_mut()
            .find(|relation| matches!(relation, SemRelationV4::MeasurementEffect { .. }))
            .unwrap()
        else {
            unreachable!()
        };
        *id = generated.measurement_relation_id;
        collision.ensure_valid().unwrap();
        assert!(matches!(
            lower_observed_controls_for_mga_pls_v1(
                &mut collision,
                qpls_core::MgaModelProfileV1::GeneralSemPls,
                "observed:group",
                "group",
            ),
            Err(MultiModRunnerErrorV1::UnsupportedProfile(code))
                if code.starts_with("multimod.runner.mga.observed_control_generated_identity_collision:")
        ));
    }

    #[test]
    fn internal_pls_point_staging_replaces_outer_additive_method_authority() {
        let mut recipe = AnalysisRecipeV4 {
            schema_version: qpls_core::ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: uuid::Uuid::from_u128(1),
            created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "fixture-dataset".into(),
            model_binding: AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
                model_id: "fixture-model".into(),
                scientific_sha256: "0".repeat(64),
            },
            estimand_confirmation: qpls_core::LegacyEstimandConfirmationV4::NotLegacy,
            settings: qpls_core::AnalysisSettings {
                method: AnalysisMethod::Predict,
                bootstrap_samples: 500,
                permutation_samples: 5_000,
                studentized_inner_samples: 200,
                case_weight_column: Some("outer-weight".into()),
                ..qpls_core::AnalysisSettings::default()
            },
            method_config: None,
            general_sem_config: Some(qpls_core::GeneralSemConfigV1::default()),
            mga_multigroup: None,
            pls_heterogeneity: Some(qpls_core::PlsUnobservedHeterogeneityConfigV2 {
                schema_version: qpls_core::PLS_HETEROGENEITY_V2_SCHEMA_VERSION,
                profile: qpls_core::HeterogeneityInteractionProfileV2::P0Structural,
                phase: qpls_core::HeterogeneityPhaseV2::Discovery {
                    candidate_k: vec![2],
                    algorithms: vec![qpls_core::HeterogeneityAlgorithmV2::FimixPlsV2],
                },
                seed: 42,
                fimix: qpls_core::FimixSettingsV2::default(),
                pls_pos: qpls_core::PlsPosSettingsV2::default(),
                pos_common_metric: None,
                bootstrap: None,
            }),
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: std::collections::BTreeMap::new(),
            legacy_source: None,
        };
        recipe.ensure_valid().unwrap();
        let outer = recipe.clone();
        let mut conflicting_outer = outer.clone();
        conflicting_outer.method_config = Some(MethodConfig::PlsAlgorithm);
        assert_eq!(
            conflicting_outer.ensure_valid().unwrap_err(),
            qpls_core::AnalysisRecipeV4Error::MultiModLegacyMethodConfigConflict
        );

        stage_internal_pls_point_recipe_v1(&mut recipe);

        assert!(outer.pls_heterogeneity.is_some());
        assert!(outer.method_config.is_none());
        assert_eq!(recipe.settings.method, AnalysisMethod::PlsPm);
        assert_eq!(recipe.settings.bootstrap_samples, 0);
        assert_eq!(recipe.settings.permutation_samples, 0);
        assert_eq!(recipe.settings.studentized_inner_samples, 0);
        assert!(recipe.settings.case_weight_column.is_none());
        assert_eq!(recipe.method_config, Some(MethodConfig::PlsAlgorithm));
        assert!(recipe.general_sem_config.is_none());
        assert!(recipe.mga_multigroup.is_none());
        assert!(recipe.pls_heterogeneity.is_none());
        assert!(recipe.general_sem_conditional_process.is_none());
        assert!(recipe.interventional_causal_mediation.is_none());
        recipe.ensure_valid().unwrap();
    }

    fn test_group(index: usize) -> GroupIndexV1 {
        GroupIndexV1::new(index).unwrap()
    }

    fn test_micom_pair(
        left: usize,
        right: usize,
        constructs: &[(&str, bool)],
    ) -> MicomPairwiseResultV1 {
        MicomPairwiseResultV1 {
            method_version: MICOM_PAIRWISE_METHOD_VERSION_V1.into(),
            pair: OrderedGroupPairV1 {
                group_a: test_group(left),
                group_b: test_group(right),
            },
            configural_receipt: MicomConfiguralReceiptV1 {
                identical_indicators_and_coding: true,
                identical_data_treatment: true,
                identical_algorithm_settings: true,
                identical_model_specification: true,
                deterministic_orientation_reviewed: true,
                analyst_review_confirmed: true,
            },
            requested_permutations: 5_000,
            usable_permutations: 5_000,
            minimum_usable_permutations: 4_500,
            partition_plan_sha256: "a".repeat(64),
            ledger_sha256: "b".repeat(64),
            ledger: Vec::new(),
            constructs: constructs
                .iter()
                .map(
                    |(construct_id, invariant)| qpls_estimation::MicomConstructResultV1 {
                        construct_id: (*construct_id).into(),
                        observed_compositional_correlation: if *invariant { 0.99 } else { 0.8 },
                        compositional_lower_quantile: Some(0.95),
                        compositional_invariance_probability: Some(if *invariant {
                            0.99
                        } else {
                            0.01
                        }),
                        compositional_invariance: *invariant,
                        observed_mean_difference_a_minus_b: 0.0,
                        mean_difference_two_sided_probability: Some(1.0),
                        equal_means: true,
                        observed_log_variance_ratio_a_minus_b: 0.0,
                        variance_difference_two_sided_probability: Some(1.0),
                        equal_variances: true,
                        partial_measurement_invariance: *invariant,
                        full_measurement_invariance: *invariant,
                        permutation_compositional_correlations: Vec::new(),
                        permutation_mean_differences: Vec::new(),
                        permutation_log_variance_ratios: Vec::new(),
                    },
                )
                .collect(),
            complete: true,
        }
    }

    fn ledger(requested: u32) -> PreparedSharedReplicateLedgerV1 {
        let domain = "general_sem_conditional_process_v2".to_owned();
        PreparedSharedReplicateLedgerV1 {
            master_seed: 42,
            domain: domain.clone(),
            requested,
            entries: (0..requested)
                .map(|replicate_index| PreparedReplicateEntryV1 {
                    replicate_index,
                    seed: multimod_replicate_seed_v1(42, &domain, replicate_index),
                    status: PreparedReplicateStatusV1::Usable,
                })
                .collect(),
        }
    }

    fn raw_heterogeneity_receipt_fixture() -> RawHeterogeneityPreparationReceiptV2 {
        RawHeterogeneityPreparationReceiptV2 {
            method_version: "qpls.heterogeneity.raw-preparation.v2".into(),
            general_sem_plan_sha256: "a".repeat(64),
            pooled_metric_sha256: "b".repeat(64),
            source_row_tokens: vec![10, 11, 12, 13],
            omitted_source_rows: 0,
            unique_analysis_positions: true,
            fimix_input: StandardizedFimixInputV2 {
                interaction_profile: EstimationHeterogeneityProfileV2::P0Structural,
                metric: PooledStandardizedMetricReceiptV2 {
                    metric_id: "pooled".into(),
                    source_sha256: "b".repeat(64),
                    observation_count: 4,
                    scores_standardized_once_on_pooled_rows: true,
                    products_standardized_once_on_pooled_rows: false,
                },
                equations: vec![StandardizedStructuralEquationV2 {
                    equation_id: "eq:y".into(),
                    outcome_id: "y".into(),
                    predictor_ids: vec!["x".into()],
                    design: vec![vec![-1.0], vec![-0.5], vec![0.5], vec![1.0]],
                    outcome: vec![-1.0, -0.5, 0.5, 1.0],
                    include_intercept: false,
                }],
            },
        }
    }

    #[test]
    fn raw_heterogeneity_receipt_rejects_identity_cardinality_and_finite_tampering() {
        let fixture = raw_heterogeneity_receipt_fixture();
        fixture.ensure_valid().unwrap();
        fixture
            .ensure_matches_live_authority(4, &[10, 11, 12, 13], &"a".repeat(64))
            .unwrap();
        assert!(
            fixture
                .ensure_matches_live_authority(4, &[10, 11, 12, 13], &"c".repeat(64))
                .is_err()
        );
        assert!(
            fixture
                .ensure_matches_live_authority(5, &[10, 11, 12, 13], &"a".repeat(64))
                .is_err()
        );

        let mut duplicate_row = fixture.clone();
        duplicate_row.source_row_tokens[3] = 12;
        assert!(duplicate_row.ensure_valid().is_err());

        let mut wrong_cardinality = fixture.clone();
        wrong_cardinality.fimix_input.metric.observation_count = 3;
        assert!(wrong_cardinality.ensure_valid().is_err());

        let mut nonfinite = fixture;
        nonfinite.fimix_input.equations[0].design[0][0] = f64::NAN;
        assert!(nonfinite.ensure_valid().is_err());
    }

    fn retained_alignment_for_mapping(mapping: Vec<usize>) -> LabelAlignmentV2 {
        let k = mapping.len();
        let mut overlap = vec![vec![0; k]; k];
        for (candidate, reference) in mapping.iter().copied().enumerate() {
            overlap[reference][candidate] = 2;
        }
        LabelAlignmentV2 {
            candidate_to_reference: mapping,
            matched_observations: k * 2,
            match_share: 1.0,
            ambiguous: false,
            mutual_majority: true,
            overlap,
        }
    }

    fn retained_alignment_fixture() -> LabelAlignmentV2 {
        retained_alignment_for_mapping(vec![1, 0])
    }

    fn raw_bootstrap_reference_fixture() -> PreparedRawHeterogeneityBootstrapReferenceV2 {
        let mut reference = PreparedRawHeterogeneityBootstrapReferenceV2 {
            method_version: "qpls.heterogeneity.bootstrap-reference.v2".into(),
            dataset_fingerprint: "fixture-dataset".into(),
            compilation_identity_sha256: "a".repeat(64),
            config_identity_sha256: "b".repeat(64),
            point_pass_identity_sha256: "c".repeat(64),
            pooled_metric_sha256: "d".repeat(64),
            complete_source_row_tokens: (0..6).collect(),
            algorithm: CoreHeterogeneityAlgorithmV2::FimixPlsV2,
            k: 2,
            use_pooled_common_metric: false,
            heterogeneity_plan: HeterogeneityBootstrapPlanV2 {
                algorithm: HeterogeneityBootstrapAlgorithmV2::FimixPlsV2,
                fixed_classes_or_segments: 2,
                requested_replicates: 10,
                master_seed: 42,
                confidence_level: 0.95,
                minimum_usable_share: 0.90,
            },
            orchestrator_plan: MultiModBootstrapPlanV1 {
                schema_version: MULTIMOD_FULL_REFIT_SCHEMA_VERSION_V1,
                scientific_refit_identity_sha256: "0".repeat(64),
                requested_replicates: 10,
                master_seed: 42,
                minimum_usable_fraction: 0.90,
            },
            reference_assignments: vec![0, 0, 0, 1, 1, 1],
            reference_target_ids: vec!["class_1:path:x->y".into(), "class_2:path:x->y".into()],
            reference_parameter_identity_sha256: "e".repeat(64),
            reference_fit_statistic: -10.0,
            reference_identity_sha256: String::new(),
        };
        reference.orchestrator_plan.scientific_refit_identity_sha256 =
            raw_heterogeneity_reference_scientific_identity_v2(&reference);
        reference.reference_identity_sha256 = raw_heterogeneity_reference_identity_v2(&reference);
        reference
    }

    #[test]
    fn raw_bootstrap_reference_identity_binds_point_rows_targets_and_plans() {
        let reference = raw_bootstrap_reference_fixture();
        assert_eq!(
            reference.reference_identity_sha256,
            raw_heterogeneity_reference_identity_v2(&reference)
        );

        let mut point_tamper = reference.clone();
        point_tamper.point_pass_identity_sha256 = "f".repeat(64);
        assert_ne!(
            reference.reference_identity_sha256,
            raw_heterogeneity_reference_identity_v2(&point_tamper)
        );

        let mut target_tamper = reference.clone();
        target_tamper.reference_target_ids[0] = "class_1:path:z->y".into();
        assert_ne!(
            reference.reference_identity_sha256,
            raw_heterogeneity_reference_identity_v2(&target_tamper)
        );

        let mut row_tamper = reference.clone();
        row_tamper.reference_assignments.swap(0, 5);
        assert_ne!(
            reference.reference_identity_sha256,
            raw_heterogeneity_reference_identity_v2(&row_tamper)
        );

        let mut parameter_tamper = reference.clone();
        parameter_tamper.reference_parameter_identity_sha256 = "0".repeat(64);
        assert_ne!(
            reference.reference_identity_sha256,
            raw_heterogeneity_reference_identity_v2(&parameter_tamper)
        );

        let mut plan_tamper = reference;
        plan_tamper.heterogeneity_plan.master_seed ^= 1;
        assert_ne!(
            plan_tamper.reference_identity_sha256,
            raw_heterogeneity_reference_identity_v2(&plan_tamper)
        );
    }

    #[test]
    fn heterogeneity_shards_publish_only_through_the_global_final_ledger() {
        let reference = raw_bootstrap_reference_fixture();
        let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
            MultiModRefitAttemptV1::Completed(Ok(RawHeterogeneityBootstrapEstimateV2 {
                fit_statistic: -10.0 - f64::from(draw.replicate_index),
                alignment: retained_alignment_fixture(),
                target_values: vec![
                    f64::from(draw.replicate_index),
                    -f64::from(draw.replicate_index),
                ],
            }))
        };
        let cache = run_multimod_case_bootstrap_shard_interruptible_v1(
            &reference.orchestrator_plan,
            reference.reference_assignments.len(),
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
        let ledger = finalize_multimod_case_bootstrap_v1(
            &reference.orchestrator_plan,
            reference.reference_assignments.len(),
            None,
            vec![cache],
        )
        .unwrap();
        assert_eq!(ledger.usable, 10);
        let prepared =
            prepared_heterogeneity_bootstrap_from_final_ledger_v2(&reference, ledger).unwrap();
        assert_eq!(prepared.entries.len(), 10);
        assert_eq!(prepared.targets.len(), 2);
        assert!(prepared.entries.iter().all(|entry| {
            entry.status == qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable
        }));
    }

    #[test]
    fn k3_through_k5_relabeling_survives_global_ledger_and_interval_assembly() {
        for k in 3usize..=5 {
            let mapping = (0..k)
                .map(|candidate| (candidate + 1) % k)
                .collect::<Vec<_>>();
            let alignment = retained_alignment_for_mapping(mapping.clone());
            validate_retained_label_alignment_v2(&alignment).unwrap();
            let reference_target_ids = (1..=k)
                .map(|class_id| format!("class_{class_id}:path:x->y"))
                .collect::<Vec<_>>();
            let candidate_parameters = (0..k)
                .map(|candidate| HeterogeneityClassParameterV2 {
                    class_id: (candidate + 1) as u8,
                    parameter: MultimodParameterEstimateV1 {
                        target_id: format!("class_{}:path:x->y", candidate + 1),
                        target_kind: "class_specific_path".into(),
                        estimate: 10.0 + candidate as f64,
                        standard_error: None,
                        p_value: None,
                        interval: None,
                    },
                    metric: "pooled".into(),
                })
                .collect::<Vec<_>>();
            let aligned = aligned_heterogeneity_target_values_v2(
                &reference_target_ids,
                &candidate_parameters,
                &alignment,
            )
            .unwrap();
            let expected = (0..k)
                .map(|reference| {
                    let candidate = mapping
                        .iter()
                        .position(|mapped| *mapped == reference)
                        .unwrap();
                    10.0 + candidate as f64
                })
                .collect::<Vec<_>>();
            assert_eq!(aligned, expected);

            let mut reference = raw_bootstrap_reference_fixture();
            reference.k = k as u8;
            reference.reference_assignments = (0..k).collect();
            reference.reference_target_ids = reference_target_ids.clone();
            reference.reference_parameter_identity_sha256 =
                sha256_serialized(&candidate_parameters);
            reference.heterogeneity_plan.fixed_classes_or_segments = k;
            reference.heterogeneity_plan.requested_replicates = 500;
            reference.orchestrator_plan.requested_replicates = 500;
            reference.orchestrator_plan.scientific_refit_identity_sha256 =
                raw_heterogeneity_reference_scientific_identity_v2(&reference);
            reference.reference_identity_sha256 =
                raw_heterogeneity_reference_identity_v2(&reference);

            let mut callback = |draw: &MultiModCaseBootstrapDrawV1| {
                let offset = f64::from(draw.replicate_index) / 1_000.0;
                MultiModRefitAttemptV1::Completed(Ok(RawHeterogeneityBootstrapEstimateV2 {
                    fit_statistic: -10.0 - offset,
                    alignment: alignment.clone(),
                    target_values: aligned.iter().map(|value| value + offset).collect(),
                }))
            };
            let cache = run_multimod_case_bootstrap_shard_interruptible_v1(
                &reference.orchestrator_plan,
                k,
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
            let ledger = finalize_multimod_case_bootstrap_v1(
                &reference.orchestrator_plan,
                k,
                None,
                vec![cache],
            )
            .unwrap();
            let prepared =
                prepared_heterogeneity_bootstrap_from_final_ledger_v2(&reference, ledger).unwrap();
            assert_eq!(prepared.ensure_valid(), Ok(k));

            let config = qpls_core::PlsUnobservedHeterogeneityConfigV2 {
                schema_version: qpls_core::PLS_HETEROGENEITY_V2_SCHEMA_VERSION,
                profile: CoreHeterogeneityProfileV2::P0Structural,
                phase: qpls_core::HeterogeneityPhaseV2::Inference {
                    lock: qpls_core::HeterogeneityInferenceLockReceiptV2 {
                        schema_version: qpls_core::HETEROGENEITY_INFERENCE_LOCK_V2_SCHEMA_VERSION,
                        discovery_result_identity_sha256: "a".repeat(64),
                        discovery_candidate_k: vec![k as u8],
                        discovery_algorithms: vec![CoreHeterogeneityAlgorithmV2::FimixPlsV2],
                        selected_algorithm: CoreHeterogeneityAlgorithmV2::FimixPlsV2,
                        selected_k: k as u8,
                        analyst_lock_confirmed: true,
                        tandem_fimix_same_k_start_required: false,
                    },
                },
                seed: 42,
                fimix: qpls_core::FimixSettingsV2::default(),
                pls_pos: qpls_core::PlsPosSettingsV2::default(),
                pos_common_metric: None,
                bootstrap: Some(qpls_core::SegmentationBootstrapV2 {
                    resamples: 500,
                    seed: 42,
                    confidence_level: 0.95,
                }),
            };
            config.ensure_valid().unwrap();
            let mut parameters = (1..=k)
                .map(|class_id| HeterogeneityClassParameterV2 {
                    class_id: class_id as u8,
                    parameter: MultimodParameterEstimateV1 {
                        target_id: format!("class_{class_id}:path:x->y"),
                        target_kind: "class_specific_path".into(),
                        estimate: expected[class_id - 1],
                        standard_error: None,
                        p_value: None,
                        interval: None,
                    },
                    metric: "pooled".into(),
                })
                .collect::<Vec<_>>();
            let summary = apply_heterogeneity_bootstrap(
                &config,
                CoreHeterogeneityAlgorithmV2::FimixPlsV2,
                k as u8,
                &prepared,
                &mut parameters,
                &mut [],
            )
            .unwrap();
            assert_eq!(summary.usable, 500);
            assert!(
                parameters
                    .iter()
                    .all(|row| row.parameter.interval.is_some())
            );
        }
    }

    #[test]
    fn prepared_heterogeneity_bootstrap_recomputes_target_and_label_receipts() {
        let values = [0.25, 0.75];
        let mut prepared = PreparedHeterogeneityBootstrapV2 {
            entries: values
                .iter()
                .enumerate()
                .map(
                    |(replicate_index, value)| HeterogeneityBootstrapLedgerEntryV2 {
                        replicate_index,
                        seed: replicate_index as u64,
                        status: qpls_estimation::HeterogeneityBootstrapReplicateStatusV2::Usable,
                        fit_statistic: Some(-10.0),
                        label_alignment: Some(retained_alignment_fixture()),
                        target_payload_sha256: Some(
                            heterogeneity_target_payload_sha256_v2(&[*value]).unwrap(),
                        ),
                        failure_reason: None,
                    },
                )
                .collect(),
            targets: vec![PreparedHeterogeneityBootstrapTargetV2 {
                target_id: "class_1:path:x->y".into(),
                estimates: values.into_iter().map(Some).collect(),
            }],
            complete_stage_one_and_segmentation_rerun: true,
            pooled_common_metric_refit_repeated: false,
            exhaustive_label_alignment_applied: true,
        };
        assert_eq!(prepared.ensure_valid(), Ok(2));

        prepared.entries[0].target_payload_sha256 = Some("0".repeat(64));
        assert!(prepared.ensure_valid().is_err());
        prepared.entries[0].target_payload_sha256 =
            Some(heterogeneity_target_payload_sha256_v2(&[values[0]]).unwrap());
        prepared.entries[0]
            .label_alignment
            .as_mut()
            .unwrap()
            .candidate_to_reference = vec![0, 1];
        assert!(prepared.ensure_valid().is_err());
    }

    #[test]
    fn prepared_pos_common_metric_requires_exact_construct_and_pair_inventory() {
        let pairs = [(0usize, 1usize), (0, 2), (1, 2)];
        let gate_input = PosCommonMetricGateInputV1 {
            pooled_metric_id: "pooled".into(),
            pooled_metric_sha256: "c".repeat(64),
            segments: 3,
            applied_identically_to_all_segments: true,
            required_construct_ids: vec!["x".into()],
            evidence: vec![PosConstructComparabilityEvidenceV1 {
                construct_id: "x".into(),
                configural_identity_passed: true,
                compositional_invariance: pairs
                    .iter()
                    .map(|(left, right)| PosPairwiseCompositionalInvarianceV1 {
                        left_segment: *left,
                        right_segment: *right,
                        passed: true,
                        permutation_p_value: Some(0.99),
                    })
                    .collect(),
                step3_equality: pairs
                    .iter()
                    .map(|(left, right)| PosPairwiseStep3EqualityV1 {
                        left_segment: *left,
                        right_segment: *right,
                        mean_equality_passed: true,
                        variance_equality_passed: true,
                    })
                    .collect(),
            }],
        };
        let gate_result = evaluate_pos_common_metric_gate_v1(&gate_input);
        let metric = format!(
            "qpls.pos.pooled-common-metric.v1:{}",
            gate_input.pooled_metric_sha256
        );
        let mut prepared = PreparedPosCommonMetricEvidenceV1 {
            method_version: "qpls.pos-common-metric.runner.v1".into(),
            gate_input,
            gate_result,
            micom_pairs: pairs
                .iter()
                .map(|(left, right)| test_micom_pair(*left, *right, &[("x", true)]))
                .collect(),
            common_metric_parameters: (1..=3)
                .map(|class_id| HeterogeneityClassParameterV2 {
                    class_id,
                    parameter: MultimodParameterEstimateV1 {
                        target_id: format!("class_{class_id}:path:x->y"),
                        target_kind: "class_specific_path".into(),
                        estimate: class_id as f64,
                        standard_error: None,
                        p_value: None,
                        interval: None,
                    },
                    metric: metric.clone(),
                })
                .collect(),
        };
        prepared.ensure_valid().unwrap();
        prepared.micom_pairs.pop();
        assert!(prepared.ensure_valid().is_err());
    }

    #[test]
    fn shared_ledger_seed_is_stable_and_domain_separated() {
        assert_eq!(
            multimod_replicate_seed_v1(42, "a", 7),
            multimod_replicate_seed_v1(42, "a", 7)
        );
        assert_ne!(
            multimod_replicate_seed_v1(42, "a", 7),
            multimod_replicate_seed_v1(42, "b", 7)
        );
        assert_ne!(
            multimod_replicate_seed_v1(42, "a", 7),
            multimod_replicate_seed_v1(42, "a", 8)
        );
    }

    #[test]
    fn pair_specific_micom_gate_keeps_ab_interpretable_when_ac_fails() {
        let pairs = vec![
            OrderedGroupPairV1 {
                group_a: test_group(0),
                group_b: test_group(1),
            },
            OrderedGroupPairV1 {
                group_a: test_group(0),
                group_b: test_group(2),
            },
        ];
        let micom = vec![
            test_micom_pair(0, 1, &[("x", true), ("y", true), ("z", true)]),
            test_micom_pair(0, 2, &[("x", true), ("y", false), ("z", true)]),
        ];
        let ordinary = OrdinaryPlsParameterProjectionV1 {
            identity: ParameterIdentityV1 {
                stable_id: "path:x->y".into(),
                family: ParameterFamilyV1::StructuralPath,
            },
            source: OrdinaryPlsParameterSourceV1::StructuralPath {
                source: "x".into(),
                target: "y".into(),
                role: StructuralRelationRoleV4::Structural,
            },
            micom_required_constructs: BTreeSet::from(["x".into(), "y".into()]),
        };
        let authority =
            comparable_ordinary_pls_targets_v1(&[ordinary], &micom, &pairs, true).unwrap();
        let ab = pairwise_plan_key_v1(pairs[0]);
        let ba = pairwise_plan_key_v1(OrderedGroupPairV1 {
            group_a: test_group(1),
            group_b: test_group(0),
        });
        let ac = pairwise_plan_key_v1(pairs[1]);
        assert_eq!(ab, ba);
        assert!(authority.by_canonical_pair[&ab].contains("path:x->y"));
        assert!(authority.by_canonical_pair[&ba].contains("path:x->y"));
        assert!(!authority.by_canonical_pair[&ac].contains("path:x->y"));
        assert!(!authority.all_pairs.contains("path:x->y"));
    }

    #[test]
    fn pairwise_row_and_cached_payload_validation_use_the_canonical_pair_gate() {
        let selected_groups = ["a", "b", "c"]
            .into_iter()
            .map(|group_id| qpls_core::SelectedGroupV1 {
                group_id: group_id.into(),
                label: group_id.to_uppercase(),
                value: qpls_core::TypedGroupValueV1::Text {
                    value: group_id.into(),
                },
            })
            .collect::<Vec<_>>();
        let config = MgaMultigroupV1 {
            schema_version: 1,
            profile: qpls_core::MgaModelProfileV1::GeneralSemPls,
            grouping_column: "group".into(),
            groups: selected_groups,
            comparison_plan: qpls_core::MgaComparisonPlanV1::ReferenceVsRest {
                reference_group_id: "a".into(),
            },
            procedures: vec![
                MgaProcedureV1::MicomPairwise,
                MgaProcedureV1::OmnibusMaxSpreadPermutation,
            ],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: qpls_core::MicomConfiguralChecklistV1 {
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
        let prepared = PreparedMgaExecutionV1 {
            design: MultigroupDesignV1 {
                groups: ["a", "b", "c"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, value)| qpls_estimation::GroupIdentityV1 {
                        index: test_group(index),
                        value: qpls_estimation::TypedGroupValueV1::Text {
                            value: value.into(),
                        },
                        display_label: value.to_uppercase(),
                    })
                    .collect(),
                rows: Vec::new(),
            },
            parameters: vec![ParameterIdentityV1 {
                stable_id: "path:x->y".into(),
                family: ParameterFamilyV1::StructuralPath,
            }],
            refit_receipt: PreparedMgaRefitReceiptV1 {
                complete_model_refit_per_request: true,
                deterministic_sign_orientation: true,
                interaction_products_rebuilt_per_request: false,
                hoc_dependency_stages_refit_per_request: false,
                plsc_correction_repeated_per_request: false,
                positive_case_weights_applied_per_request: false,
                integer_frequency_count_space_equivalent: false,
            },
            observed_group_parameters: Vec::new(),
            pairwise_partition_plans: Vec::new(),
            micom_partition_plan_receipts: BTreeMap::new(),
            micom_pairs: Vec::new(),
            comparable_target_ids_by_canonical_pair: BTreeMap::from([
                ("g0:g1".into(), BTreeSet::from(["path:x->y".into()])),
                ("g0:g2".into(), BTreeSet::new()),
            ]),
            comparable_target_ids: BTreeSet::new(),
            parametric_cells: Vec::new(),
            excluded_rows: Vec::new(),
        };
        let row = |left: &str, right: &str, comparable: bool| MgaPairwiseComparisonV1 {
            procedure: "cached_pairwise_test".into(),
            left_group_id: left.into(),
            right_group_id: right.into(),
            target_id: "path:x->y".into(),
            difference_left_minus_right: 0.1,
            raw_p_value: Some(0.5),
            adjusted_p_value: None,
            directional_probability: None,
            interval: None,
            measurement_comparability_satisfied: comparable,
            interpretation_blocked: !comparable,
        };
        let rows = vec![
            row("a", "b", true),
            row("a", "c", false),
            row("b", "a", true),
        ];
        validate_pairwise_result_comparability_v1(&config, &prepared, &rows).unwrap();

        let stale_cached_rows = vec![row("a", "b", true), row("a", "c", true)];
        assert!(matches!(
            validate_pairwise_result_comparability_v1(&config, &prepared, &stale_cached_rows),
            Err(MultiModRunnerErrorV1::ResultContract(_))
        ));
    }

    #[test]
    fn pair_specific_gate_covers_interaction_and_hoc_weighted_target_shapes() {
        let pairs = vec![
            OrderedGroupPairV1 {
                group_a: test_group(0),
                group_b: test_group(1),
            },
            OrderedGroupPairV1 {
                group_a: test_group(0),
                group_b: test_group(2),
            },
        ];
        let micom = vec![
            test_micom_pair(0, 1, &[("hoc_x", true), ("y", true), ("z", true)]),
            test_micom_pair(0, 2, &[("hoc_x", true), ("y", false), ("z", true)]),
        ];
        let hoc_or_weighted_path = OrdinaryPlsParameterProjectionV1 {
            identity: ParameterIdentityV1 {
                stable_id: "path:hoc_x->y".into(),
                family: ParameterFamilyV1::StructuralPath,
            },
            source: OrdinaryPlsParameterSourceV1::StructuralPath {
                source: "hoc_x".into(),
                target: "y".into(),
                role: StructuralRelationRoleV4::Structural,
            },
            micom_required_constructs: BTreeSet::from(["hoc_x".into(), "y".into()]),
        };
        let interaction = InteractionMgaParameterProjectionV1 {
            identity: ParameterIdentityV1 {
                stable_id: "gamma:hoc_x*z->y".into(),
                family: ParameterFamilyV1::InteractionGamma,
            },
            required_constructs: BTreeSet::from(["hoc_x".into(), "y".into(), "z".into()]),
        };
        let ordinary_authority =
            comparable_ordinary_pls_targets_v1(&[hoc_or_weighted_path], &micom, &pairs, true)
                .unwrap();
        let interaction_authority =
            comparable_interaction_targets_v1(&[interaction], &micom, &pairs, true).unwrap();
        let ab = pairwise_plan_key_v1(pairs[0]);
        let ac = pairwise_plan_key_v1(pairs[1]);
        assert!(ordinary_authority.by_canonical_pair[&ab].contains("path:hoc_x->y"));
        assert!(!ordinary_authority.by_canonical_pair[&ac].contains("path:hoc_x->y"));
        assert!(interaction_authority.by_canonical_pair[&ab].contains("gamma:hoc_x*z->y"));
        assert!(!interaction_authority.by_canonical_pair[&ac].contains("gamma:hoc_x*z->y"));
    }

    #[test]
    fn shared_ledger_rejects_per_target_validity_drift() {
        let mut source = ledger(4);
        source.entries[2].status = PreparedReplicateStatusV1::Failed {
            kind: MultimodReplicateFailureKindV1::RankDeficient,
            stable_code: "fit.rank".into(),
            detail: "rank deficient".into(),
        };
        let targets = vec![PreparedTargetReplicatesV1 {
            target_id: "t".into(),
            estimates: vec![Some(1.0), Some(2.0), Some(3.0), Some(4.0)],
            delete_one_jackknife_estimates: Vec::new(),
            observed_standard_error: None,
            outer_standard_errors: Vec::new(),
        }];
        let outcome =
            validate_target_replicates(&source, &targets, &BTreeSet::from(["t".to_owned()]));
        assert!(matches!(
            outcome,
            Err(MultiModRunnerErrorV1::InvalidLedger(_))
        ));
    }

    #[test]
    fn conditional_helper_reports_both_stage_effect_and_nonconstant_index() {
        let path = ExplicitConditionalPathV2 {
            path_id: "x_m_y".into(),
            edges: vec![
                ConditionalEdgeFunctionV2 {
                    relation_id: "x_m".into(),
                    source_id: "x".into(),
                    target_id: "m".into(),
                    intercept: 0.5,
                    linear_coefficients: vec![ConditionalLinearCoefficientV2 {
                        moderator_id: "z".into(),
                        estimate: 0.2,
                    }],
                    pairwise_coefficients: Vec::new(),
                },
                ConditionalEdgeFunctionV2 {
                    relation_id: "m_y".into(),
                    source_id: "m".into(),
                    target_id: "y".into(),
                    intercept: 0.7,
                    linear_coefficients: vec![ConditionalLinearCoefficientV2 {
                        moderator_id: "z".into(),
                        estimate: -0.1,
                    }],
                    pairwise_coefficients: Vec::new(),
                },
            ],
        };
        let probes = vec![ConditionalProbePointV2 {
            probe_id: "zero".into(),
            standardized_values: BTreeMap::from([("z".into(), 0.0)]),
        }];
        let result = prepare_conditional_path_point_targets_v2(
            &path,
            &probes,
            None,
            true,
            true,
            true,
            true,
            &[],
        )
        .unwrap();
        assert!(result.targets.iter().any(|target| {
            target.kind == ConditionalProcessTargetKindV2::ConditionalSpecificIndirect
                && (target.estimate - 0.35).abs() < 1.0e-12
        }));
        assert!(result.targets.iter().any(|target| {
            target.kind == ConditionalProcessTargetKindV2::LocalSecondDerivative
        }));
        assert_eq!(result.warnings.len(), 1);
        assert!(!result.targets.iter().any(|target| {
            target.kind == ConditionalProcessTargetKindV2::ScalarIndexOfModeratedMediation
        }));
    }

    #[test]
    fn public_interval_contract_enforces_alternative_specific_bounds() {
        let greater = interval(
            "percentile",
            0.95,
            InferenceAlternativeV1::Greater,
            Some(0.1),
            None,
        )
        .unwrap();
        assert_eq!(greater.lower, Some(0.1));
        assert_eq!(greater.upper, None);

        let less = interval(
            "percentile",
            0.95,
            InferenceAlternativeV1::Less,
            None,
            Some(0.8),
        )
        .unwrap();
        assert_eq!(less.lower, None);
        assert_eq!(less.upper, Some(0.8));

        let invalid = interval(
            "percentile",
            0.95,
            InferenceAlternativeV1::Greater,
            None,
            Some(0.8),
        );
        assert!(matches!(
            invalid,
            Err(MultiModRunnerErrorV1::ResultContract(_))
        ));
    }

    #[test]
    fn pooled_reference_orientation_detects_exact_sign_reversal() {
        let reference = [-2.0, -0.5, 0.25, 3.0];
        let reversed = reference.map(|value| -value);
        let correlation = correlation_v1(&reference, &reversed).unwrap();
        assert!((correlation + 1.0).abs() < 1.0e-12);
        assert_eq!(sample_standard_deviation_v1(&[4.0, 4.0, 4.0]), 0.0);
    }

    #[test]
    fn heterogeneity_unstable_candidates_retain_truthful_start_counts() {
        let fimix_diagnostics = (0..4)
            .map(|start_index| {
                let converged = start_index < 3;
                qpls_estimation::FimixStartDiagnosticV2 {
                    start_index,
                    start_seed: 100 + start_index as u64,
                    converged,
                    iterations: 12,
                    final_log_likelihood: converged.then_some(-25.0 - start_index as f64),
                    maximum_likelihood_decrease: 0.0,
                    final_effective_class_sizes: vec![50.0, 50.0],
                    failure_code: (!converged)
                        .then_some(qpls_estimation::FimixStartFailureCodeV2::MaximumIterations),
                    failure_message: (!converged).then_some("fixture did not converge".into()),
                    trace: Vec::new(),
                }
            })
            .collect::<Vec<_>>();
        let fimix_error = HeterogeneityV2Error::UnstableFimixOptimum {
            reproducing_starts: 1,
            required_starts: 2,
            diagnostics: fimix_diagnostics,
        };
        let fimix_candidate = candidate_from_heterogeneity_error(
            CoreHeterogeneityAlgorithmV2::FimixPlsV2,
            2,
            &fimix_error,
        );
        assert_eq!(
            fimix_candidate.state,
            HeterogeneityCandidateStateV2::Unstable
        );
        assert_eq!(fimix_candidate.converged_starts, 3);
        assert_eq!(fimix_candidate.stable_starts, 1);
        assert_eq!(
            fimix_candidate.blockers,
            vec!["FIMIX optimum was reproduced by 1 starts; 2 required"]
        );

        let pos_diagnostics = (0..5)
            .map(|start_index| {
                let completed = start_index != 4;
                qpls_estimation::PosStartDiagnosticV2 {
                    start_index,
                    completed,
                    accepted_moves: if completed { 1 } else { 0 },
                    final_objective: completed.then_some(1.5),
                    failure_reason: (!completed).then_some("fixture refit failed".into()),
                    candidate_refit_failures: Vec::new(),
                    objective_history: if completed {
                        vec![1.0, 1.5]
                    } else {
                        Vec::new()
                    },
                }
            })
            .collect::<Vec<_>>();
        let pos_error = HeterogeneityV2Error::UnstablePosOptimum {
            reproducing_starts: 1,
            required_starts: 2,
            diagnostics: pos_diagnostics,
        };
        let pos_candidate = candidate_from_heterogeneity_error(
            CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2,
            3,
            &pos_error,
        );
        assert_eq!(pos_candidate.state, HeterogeneityCandidateStateV2::Unstable);
        assert_eq!(pos_candidate.converged_starts, 4);
        assert_eq!(pos_candidate.stable_starts, 1);
        assert_eq!(
            pos_candidate.blockers,
            vec!["PLS-POS optimum was reproduced by 1 starts; 2 required"]
        );

        let hard_failure = candidate_from_heterogeneity_error(
            CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2,
            2,
            &HeterogeneityV2Error::PosRefit("fixture hard failure".into()),
        );
        assert_eq!(hard_failure.state, HeterogeneityCandidateStateV2::Failed);
        assert_eq!(hard_failure.converged_starts, 0);
        assert_eq!(hard_failure.stable_starts, 0);
    }

    #[test]
    fn heterogeneity_settings_map_every_frozen_stability_tolerance() {
        let mut fimix = qpls_core::FimixSettingsV2::default();
        fimix.rank_tolerance = 3.0e-11;
        fimix.required_reproducing_starts = 3;
        fimix.optimum_relative_log_likelihood_tolerance = 4.0e-8;
        fimix.optimum_maximum_coefficient_difference = 5.0e-6;
        fimix.optimum_mean_posterior_difference = 6.0e-4;
        let mut pls_pos = qpls_core::PlsPosSettingsV2::default();
        pls_pos.stable_objective_tolerance = 7.0e-10;
        pls_pos.minimum_reproducing_starts = 2;
        let config = qpls_core::PlsUnobservedHeterogeneityConfigV2 {
            schema_version: qpls_core::PLS_HETEROGENEITY_V2_SCHEMA_VERSION,
            profile: CoreHeterogeneityProfileV2::P0Structural,
            phase: qpls_core::HeterogeneityPhaseV2::Discovery {
                candidate_k: vec![2],
                algorithms: vec![CoreHeterogeneityAlgorithmV2::FimixPlsV2],
            },
            seed: 99,
            fimix,
            pls_pos,
            pos_common_metric: None,
            bootstrap: None,
        };
        let mapped_fimix = fimix_config(&config, 2);
        assert_eq!(mapped_fimix.seed, 99);
        assert_eq!(
            mapped_fimix.likelihood_decrease_tolerance,
            qpls_core::FIMIX_LIKELIHOOD_DECREASE_RELATIVE_TOLERANCE_V2
        );
        assert_eq!(mapped_fimix.rank_tolerance, 3.0e-11);
        assert_eq!(mapped_fimix.required_reproducing_starts, 3);
        assert_eq!(
            mapped_fimix.optimum_relative_log_likelihood_tolerance,
            4.0e-8
        );
        assert_eq!(mapped_fimix.optimum_maximum_coefficient_difference, 5.0e-6);
        assert_eq!(mapped_fimix.optimum_mean_posterior_difference, 6.0e-4);
        let mapped_pos = pos_config(&config, 2, 100);
        assert_eq!(mapped_pos.stable_objective_tolerance, 7.0e-10);
        assert_eq!(mapped_pos.required_reproducing_starts, 2);
    }

    #[test]
    fn prepared_heterogeneity_stage_one_refit_is_bit_exact_to_checked_recipe_v4_path() {
        let (dataset, model, mut recipe) = multimod_point_authority_fixture_v1(None, false);
        recipe.settings.method = AnalysisMethod::Predict;
        recipe.method_config = None;
        recipe.general_sem_config = Some(qpls_core::GeneralSemConfigV1::default());
        recipe.pls_heterogeneity = Some(qpls_core::PlsUnobservedHeterogeneityConfigV2 {
            schema_version: qpls_core::PLS_HETEROGENEITY_V2_SCHEMA_VERSION,
            profile: CoreHeterogeneityProfileV2::P0Structural,
            phase: qpls_core::HeterogeneityPhaseV2::Discovery {
                candidate_k: vec![2],
                algorithms: vec![CoreHeterogeneityAlgorithmV2::PlsPosPublishedV2],
            },
            seed: 42,
            fimix: qpls_core::FimixSettingsV2::default(),
            pls_pos: qpls_core::PlsPosSettingsV2::default(),
            pos_common_metric: None,
            bootstrap: None,
        });
        recipe.ensure_valid().unwrap();
        let artifact = prepare_multimod_recipe_v1(
            &dataset,
            &recipe,
            &model,
            MultiModCompilerTargetV1::PlsHeterogeneityV2,
        )
        .unwrap();
        let authority =
            projected_heterogeneity_authority_v2(&dataset, &recipe, &model, &artifact).unwrap();
        let rows = (0..dataset.batch.num_rows().min(48)).collect::<Vec<_>>();
        assert!(rows.len() >= 20);
        let sampled =
            resample_dataset_columns_v1(&dataset, &authority.source_columns, &rows, || false)
                .unwrap();

        let checked = raw_heterogeneity_stage_one_fit_v2(&sampled, &authority, &|| false).unwrap();
        let prepared =
            raw_heterogeneity_stage_one_refit_v2(&sampled, &authority, &|| false).unwrap();
        assert_eq!(prepared, checked);
        assert_eq!(
            qpls_core::sha256_serialized(&prepared),
            qpls_core::sha256_serialized(&checked)
        );
    }

    #[test]
    fn sidecar_cost_thresholds_are_exact_and_overflow_fails_closed() {
        assert_eq!(
            multimod_sidecar_cost_state_v1(MULTIMOD_SIDECAR_WARN_BYTES_V1),
            MultiModSidecarCostStateV1::WithinLimit
        );
        assert_eq!(
            multimod_sidecar_cost_state_v1(MULTIMOD_SIDECAR_WARN_BYTES_V1 + 1),
            MultiModSidecarCostStateV1::Warning
        );
        assert_eq!(
            multimod_sidecar_cost_state_v1(MULTIMOD_SIDECAR_MAX_BYTES_V1),
            MultiModSidecarCostStateV1::Warning
        );
        assert_eq!(
            multimod_sidecar_cost_state_v1(MULTIMOD_SIDECAR_MAX_BYTES_V1 + 1),
            MultiModSidecarCostStateV1::Blocked
        );
        assert_eq!(sidecar_cost_add_v1(u64::MAX, 1), u64::MAX);
        assert_eq!(sidecar_cost_mul_v1(u64::MAX, 2), u64::MAX);
    }

    fn mga_sidecar_cost_fixture_v1(targets: usize) -> MgaMultigroupV1 {
        MgaMultigroupV1 {
            schema_version: 1,
            profile: qpls_core::MgaModelProfileV1::GeneralSemPls,
            grouping_column: "group".into(),
            groups: (0..20)
                .map(|index| qpls_core::SelectedGroupV1 {
                    group_id: format!("group_{:02}", index + 1),
                    label: format!("Group {}", index + 1),
                    value: qpls_core::TypedGroupValueV1::Integer {
                        value: i64::try_from(index).unwrap(),
                    },
                })
                .collect(),
            comparison_plan: qpls_core::MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: true,
            },
            procedures: vec![
                MgaProcedureV1::MicomPairwise,
                MgaProcedureV1::PairwisePermutation,
                MgaProcedureV1::HenselerPlsMga,
                MgaProcedureV1::BootstrapDifferenceBc,
                MgaProcedureV1::OmnibusMaxSpreadPermutation,
            ],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: qpls_core::MicomConfiguralChecklistV1 {
                identical_indicators_and_coding: true,
                identical_data_treatment: true,
                identical_algorithm_settings: true,
                identical_model_specification: true,
                deterministic_sign_orientation_reviewed: true,
                analyst_review_confirmed: true,
            },
            weight: None,
            selected_parameter_ids: (0..targets.max(1))
                .map(|index| format!("target_{index}"))
                .collect(),
        }
    }

    #[test]
    fn mga_sidecar_cost_accounts_for_bounded_null_and_micom_distributions() {
        let config = mga_sidecar_cost_fixture_v1(30);
        let rows = vec![10; 20];
        let maximum_target_id_bytes = 64;
        let one_construct =
            predict_mga_sidecar_bytes_v1(&config, &rows, 30, maximum_target_id_bytes, 1);
        let two_constructs =
            predict_mga_sidecar_bytes_v1(&config, &rows, 30, maximum_target_id_bytes, 2);
        let three_constructs =
            predict_mga_sidecar_bytes_v1(&config, &rows, 30, maximum_target_id_bytes, 3);
        let exact_general_sem_inventory =
            predict_mga_sidecar_bytes_v1(&config, &rows, 18, maximum_target_id_bytes, 3);
        let pair_draws = 190_u64 * 5_000;
        assert_eq!(two_constructs - one_construct, pair_draws * 18);
        assert_eq!(
            multimod_sidecar_cost_state_v1(one_construct),
            MultiModSidecarCostStateV1::Warning
        );
        assert_eq!(
            multimod_sidecar_cost_state_v1(two_constructs),
            MultiModSidecarCostStateV1::Warning
        );
        assert_eq!(
            multimod_sidecar_cost_state_v1(three_constructs),
            MultiModSidecarCostStateV1::Blocked
        );
        assert_eq!(
            multimod_sidecar_cost_state_v1(exact_general_sem_inventory),
            MultiModSidecarCostStateV1::Warning
        );

        let mut without_micom = config.clone();
        without_micom
            .procedures
            .retain(|procedure| *procedure != MgaProcedureV1::MicomPairwise);
        let without_micom_bytes =
            predict_mga_sidecar_bytes_v1(&without_micom, &rows, 30, maximum_target_id_bytes, 2);
        assert_eq!(
            two_constructs - without_micom_bytes,
            190 * (5_000 * 96 + 4_096 + 5_000 * (2 + 2) * 18)
        );

        let mut without_pairwise_null = config.clone();
        without_pairwise_null
            .procedures
            .retain(|procedure| *procedure != MgaProcedureV1::PairwisePermutation);
        let without_pairwise_null_bytes = predict_mga_sidecar_bytes_v1(
            &without_pairwise_null,
            &rows,
            30,
            maximum_target_id_bytes,
            2,
        );
        assert_eq!(
            two_constructs - without_pairwise_null_bytes,
            pair_draws * 224 + 190 * (4_096 + maximum_target_id_bytes as u64 + 4)
        );

        let mut without_omnibus = config;
        without_omnibus
            .procedures
            .retain(|procedure| *procedure != MgaProcedureV1::OmnibusMaxSpreadPermutation);
        let without_omnibus_bytes =
            predict_mga_sidecar_bytes_v1(&without_omnibus, &rows, 30, maximum_target_id_bytes, 2);
        assert_eq!(
            two_constructs - without_omnibus_bytes,
            5_000 * (200 * 8 + 30 * 21 + 128) + 4_096 + 30 * (maximum_target_id_bytes as u64 + 4)
        );
    }

    #[test]
    fn micom_ordinal_arrow_prediction_bounds_the_trusted_stream() {
        let rows = 25_000_u32;
        let batch = qpls_project::multimod_micom_null_statistics_batch_v1(
            (0..rows).map(|row| row / 5).collect(),
            (0..rows)
                .map(|row| if row % 5 < 3 { row % 5 } else { 0 })
                .collect(),
            (0..rows)
                .map(|row| if row % 5 < 3 { 0 } else { (row % 5 - 2) as u8 })
                .collect(),
            (0..rows).map(|row| f64::from(row) / 10_000.0).collect(),
        )
        .unwrap();
        let payload = qpls_project::encode_multimod_arrow_sidecar_v1(
            "result:micom-preflight-boundary",
            "mga-micom-pair-null-statistics.arrow",
            &"a".repeat(64),
            "mga-micom-pair:null-statistics",
            &batch,
        )
        .unwrap();
        assert!(
            payload.descriptor.uncompressed_bytes
                <= predict_mga_micom_null_statistics_arrow_bytes_v1(u64::from(rows))
        );
    }

    #[test]
    fn frequency_mga_eligibility_uses_represented_cases_not_compact_rows() {
        let dataset = qpls_data::import_delimited_bytes(
            b"group,f\na,2\na,2\na,2\na,2\na,2\na,2\nb,2\nb,2\nb,2\nb,2\nb,2\nb,2\n",
            "frequency-mga-preflight.csv",
            b',',
            &qpls_data::ImportOptions::default(),
        )
        .unwrap();
        let groups = vec![
            qpls_estimation::GroupIdentityV1 {
                index: GroupIndexV1::new(0).unwrap(),
                value: qpls_estimation::TypedGroupValueV1::Text { value: "a".into() },
                display_label: "A".into(),
            },
            qpls_estimation::GroupIdentityV1 {
                index: GroupIndexV1::new(1).unwrap(),
                value: qpls_estimation::TypedGroupValueV1::Text { value: "b".into() },
                display_label: "B".into(),
            },
        ];
        let design = MultigroupDesignV1 {
            groups,
            rows: (0..12)
                .map(|source_row| SelectedGroupRowV1 {
                    source_row,
                    stable_row_token: source_row,
                    group: GroupIndexV1::new(if source_row < 6 { 0 } else { 1 }).unwrap(),
                })
                .collect(),
        };
        assert!(!assess_multigroup_design_v1(&design).eligible);
        let config = MgaMultigroupV1 {
            schema_version: 1,
            profile: qpls_core::MgaModelProfileV1::FrequencyWeightedPls,
            grouping_column: "group".into(),
            groups: vec![
                qpls_core::SelectedGroupV1 {
                    group_id: "a".into(),
                    label: "A".into(),
                    value: qpls_core::TypedGroupValueV1::Text { value: "a".into() },
                },
                qpls_core::SelectedGroupV1 {
                    group_id: "b".into(),
                    label: "B".into(),
                    value: qpls_core::TypedGroupValueV1::Text { value: "b".into() },
                },
            ],
            comparison_plan: qpls_core::MgaComparisonPlanV1::AllPairs {
                heavy_run_confirmed: false,
            },
            procedures: vec![MgaProcedureV1::PairwisePermutation],
            permutation_samples: 5_000,
            bootstrap_samples: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: InferenceAlternativeV1::TwoSided,
            multiplicity: MultiplicityAdjustmentV1::Holm,
            configural_checklist: qpls_core::MicomConfiguralChecklistV1 {
                identical_indicators_and_coding: true,
                identical_data_treatment: true,
                identical_algorithm_settings: true,
                identical_model_specification: true,
                deterministic_sign_orientation_reviewed: true,
                analyst_review_confirmed: true,
            },
            weight: Some(qpls_core::AnalysisWeightBindingV1::Frequency { column: "f".into() }),
            selected_parameter_ids: vec!["path:x:y".into()],
        };
        let configured_weight = match config.weight.as_ref().unwrap() {
            qpls_core::AnalysisWeightBindingV1::Frequency { column } => column.as_str(),
            qpls_core::AnalysisWeightBindingV1::Case { .. } => unreachable!(),
        };
        let (frequency_design, canonical_rows, counts) =
            frequency_multigroup_design_from_raw_v1(&dataset, configured_weight, &design).unwrap();
        assert_eq!(canonical_rows.len(), 12);
        assert_eq!(counts, vec![2; 12]);
        let eligibility = assess_frequency_multigroup_design_v1(&frequency_design);
        assert!(eligibility.eligible);
        assert!(
            eligibility
                .group_counts
                .iter()
                .all(|group| group.complete_cases == 12)
        );
        let compact_group_rows = (0..6).collect::<Vec<_>>();
        assert_eq!(
            checked_frequency_source_rows_v1(
                &dataset,
                &compact_group_rows,
                &vec![2; compact_group_rows.len()],
            )
            .unwrap()
            .len(),
            compact_group_rows.len()
        );
        assert!(matches!(
            checked_source_rows_v1(&dataset, &compact_group_rows),
            Err(RefitFailureV1 {
                code: RefitFailureCodeV1::InsufficientRows,
                ..
            })
        ));
        assert!(matches!(
            checked_frequency_source_rows_v1(
                &dataset,
                &compact_group_rows,
                &vec![1; compact_group_rows.len()],
            ),
            Err(RefitFailureV1 {
                code: RefitFailureCodeV1::InsufficientRows,
                ..
            })
        ));
    }

    #[test]
    fn malformed_cached_bootstrap_bank_fails_closed_before_indexing() {
        let groups = vec![
            qpls_estimation::GroupIdentityV1 {
                index: GroupIndexV1::new(0).unwrap(),
                value: qpls_estimation::TypedGroupValueV1::Text { value: "a".into() },
                display_label: "A".into(),
            },
            qpls_estimation::GroupIdentityV1 {
                index: GroupIndexV1::new(1).unwrap(),
                value: qpls_estimation::TypedGroupValueV1::Text { value: "b".into() },
                display_label: "B".into(),
            },
        ];
        let design = MultigroupDesignV1 {
            groups,
            rows: (0..20)
                .map(|source_row| SelectedGroupRowV1 {
                    source_row,
                    stable_row_token: source_row,
                    group: GroupIndexV1::new(if source_row < 10 { 0 } else { 1 }).unwrap(),
                })
                .collect(),
        };
        let parameters = vec![ParameterIdentityV1 {
            stable_id: "path:x->y".into(),
            family: ParameterFamilyV1::StructuralPath,
        }];
        let config = MultigroupResamplingConfigV1 {
            requested: 5_000,
            seed: 42,
            confidence_level: 0.95,
            alpha: 0.05,
            alternative: AlternativeHypothesisV1::TwoSided,
        };
        let eligibility = assess_multigroup_design_v1(&design);
        let malformed = GroupBootstrapBanksV1 {
            method_version: "mga_multigroup_group_bootstrap_bank_v1".into(),
            parameters: parameters.clone(),
            seed: 42,
            requested: 5_000,
            attempted: 5_000,
            minimum_usable: 4_500,
            retry_policy: "none".into(),
            plan_sha256: "sha256:test".into(),
            availability: InferenceAvailabilityV1::Available,
            groups: Vec::new(),
            ledger: Vec::new(),
            group_counts: eligibility.group_counts.clone(),
            eligibility_warnings: eligibility.warnings,
        };
        assert!(matches!(
            validate_cached_bootstrap_banks_v1(
                &malformed,
                MGA_MULTIGROUP_BOOTSTRAP_BANK_VERSION_V1,
                &design,
                &parameters,
                config,
                &eligibility.group_counts,
            ),
            Err(MultiModRunnerErrorV1::ExecutionCache(_))
        ));
    }
}
