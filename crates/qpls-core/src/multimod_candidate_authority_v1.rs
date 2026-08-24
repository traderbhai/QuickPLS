//! Exact runtime promotion cells for the additive MultiMod contracts.
//!
//! Qualification is intentionally not inferred from a family capability row.
//! This module maps one validated typed recipe, compiled artifact, and result
//! to the smallest exact profile/procedure-cell set the result consumes.

use crate::{
    AnalysisRecipeV4, CompiledMultiModPlanV1, CompiledMultiModRecipeV1,
    ConditionalProcessProfileV2, HeterogeneityAlgorithmV2, HeterogeneityCandidateMethodV2,
    HeterogeneityInteractionProfileV2, HeterogeneityPhaseV2, MgaModelProfileV1, MgaProcedureV1,
    MultiModAnalysisResultV1, MultiModCompilerTargetV1, MultimodCandidateQualificationReceiptV1,
    MultimodQualificationStateV1, PlsUnobservedHeterogeneityConfigV2,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum MultiModCandidateAuthorityErrorV1 {
    #[error("typed recipe, compiled artifact, and result family do not match")]
    FamilyMismatch,
    #[error("typed recipe, compiled artifact, and result provenance do not share one authority")]
    AuthorityMismatch,
    #[error("typed result inventory differs from the compiled profile or algorithms")]
    ArtifactMismatch,
    #[error("candidate qualification has no exact cell for {0}")]
    CellNotQualified(String),
    #[error("candidate qualification receipt does not match the exact required cell set")]
    ReceiptCellMismatch,
}

fn cell(profile: &str, procedure: &str) -> String {
    format!("{profile}::{procedure}")
}

fn insert_profile_cells(cells: &mut BTreeSet<String>, profile: &str, procedures: &[&str]) {
    cells.extend(procedures.iter().map(|procedure| cell(profile, procedure)));
}

fn result_matches_compilation_authority_v1(
    artifact: &CompiledMultiModRecipeV1,
    result: &MultiModAnalysisResultV1,
) -> bool {
    let compiled = artifact.receipt();
    let provenance = result.provenance();
    provenance.recipe_id == compiled.recipe_id
        && provenance.recipe_analytical_sha256 == compiled.recipe_analytical_sha256
        && provenance.config_sha256 == compiled.config_sha256
        && provenance.model_id == compiled.model_id
        && provenance.model_scientific_sha256 == compiled.model_scientific_sha256
        && provenance.dataset_id == compiled.dataset_id
        && provenance.dataset_fingerprint == compiled.dataset_fingerprint
        && provenance.capability_cell == compiled.capability_cell
}

fn mga_profile_identity_v1(profile: MgaModelProfileV1) -> (&'static str, &'static [&'static str]) {
    match profile {
        MgaModelProfileV1::GeneralSemPls => ("mga.general_sem_pls.v1", &["point_fit"]),
        MgaModelProfileV1::MultipleTwoWayModeration => (
            "mga.multiple_two_way_moderation.v1",
            &["point_fit_path_gamma_slopes"],
        ),
        MgaModelProfileV1::BoundedThreeWayModeration => (
            "mga.bounded_three_way_moderation.v1",
            &["point_fit_path_gamma_delta_slopes"],
        ),
        MgaModelProfileV1::BoundedTwoWayModeratedMediation => (
            "mga.bounded_two_way_moderated_mediation.v1",
            &["point_fit_bounded_conditional_targets"],
        ),
        MgaModelProfileV1::MultipleNonnestedHoc => {
            ("mga.multiple_nonnested_hoc.v1", &["point_fit_hoc_stages"])
        }
        MgaModelProfileV1::CaseWeightedPls => ("mga.case_weighted_pls.v1", &["weighted_point_fit"]),
        MgaModelProfileV1::FrequencyWeightedPls => (
            "mga.frequency_weighted_pls.v1",
            &["count_space_point_fit", "expanded_row_equivalence"],
        ),
        MgaModelProfileV1::ReflectivePlsc => ("mga.reflective_plsc.v1", &["plsc_point_fit"]),
    }
}

fn mga_procedure_cell_v1(
    profile: MgaModelProfileV1,
    procedure: MgaProcedureV1,
) -> Result<&'static str, MultiModCandidateAuthorityErrorV1> {
    let ordinary = match procedure {
        MgaProcedureV1::MicomPairwise => Some("micom_pairwise_steps_2_3"),
        MgaProcedureV1::PairwisePermutation => Some("pairwise_permutation"),
        MgaProcedureV1::OmnibusMaxSpreadPermutation => Some("max_spread_omnibus"),
        MgaProcedureV1::BootstrapDifferenceBc => Some("bootstrap_bc"),
        _ => None,
    };
    if profile == MgaModelProfileV1::ReflectivePlsc {
        return match procedure {
            MgaProcedureV1::MicomPairwise => Ok("composite_micom_pairwise_steps_2_3"),
            MgaProcedureV1::PairwisePermutation => Ok("consistent_pairwise_permutation"),
            MgaProcedureV1::OmnibusMaxSpreadPermutation => Ok("consistent_max_spread_omnibus"),
            MgaProcedureV1::BootstrapDifferenceBc => Ok("consistent_bootstrap_bc"),
            unsupported => Err(MultiModCandidateAuthorityErrorV1::CellNotQualified(
                format!("reflective PLSc {unsupported:?}"),
            )),
        };
    }
    if let Some(cell) = ordinary {
        return Ok(cell);
    }
    match procedure {
        MgaProcedureV1::HenselerPlsMga
            if matches!(
                profile,
                MgaModelProfileV1::GeneralSemPls | MgaModelProfileV1::MultipleTwoWayModeration
            ) =>
        {
            Ok("henseler_directional_probability")
        }
        MgaProcedureV1::ParametricPooledVariance | MgaProcedureV1::ParametricWelchSatterthwaite
            if profile == MgaModelProfileV1::GeneralSemPls =>
        {
            Ok("pooled_and_welch")
        }
        MgaProcedureV1::ParametricWaldOmnibus if profile == MgaModelProfileV1::GeneralSemPls => {
            Ok("k_group_wald")
        }
        unsupported => Err(MultiModCandidateAuthorityErrorV1::CellNotQualified(
            format!("{profile:?} {unsupported:?}"),
        )),
    }
}

fn heterogeneity_profile_identity_v1(
    algorithm: HeterogeneityAlgorithmV2,
    profile: HeterogeneityInteractionProfileV2,
) -> Result<(&'static str, &'static [&'static str]), MultiModCandidateAuthorityErrorV1> {
    match (algorithm, profile) {
        (HeterogeneityAlgorithmV2::FimixPlsV2, HeterogeneityInteractionProfileV2::P0Structural) => {
            Ok((
                "fimix.p0_structural.v2",
                &["em_multistart_point", "likelihood_criteria_posteriors"],
            ))
        }
        (
            HeterogeneityAlgorithmV2::FimixPlsV2,
            HeterogeneityInteractionProfileV2::P2MultiTwoWay,
        ) => Ok((
            "fimix.p2_multi_two_way.v2",
            &[
                "pooled_metric_products",
                "em_multistart_point",
                "gamma_and_slopes",
            ],
        )),
        (
            HeterogeneityAlgorithmV2::FimixPlsV2,
            HeterogeneityInteractionProfileV2::P23AllCurrent,
        ) => Ok((
            "fimix.p23_all_current.v2",
            &[
                "pooled_metric_three_way_products",
                "em_multistart_point",
                "gamma_delta_and_slopes",
            ],
        )),
        (
            HeterogeneityAlgorithmV2::PlsPosPublishedV2,
            HeterogeneityInteractionProfileV2::P0Structural,
        ) => Ok((
            "pos.published.p0_structural.v2",
            &[
                "ten_start_full_refit_point",
                "strict_objective_monotonicity",
            ],
        )),
        (
            HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
            HeterogeneityInteractionProfileV2::P2MultiTwoWay,
        ) => Ok((
            "pos.destination_scored.p2_multi_two_way.v2",
            &[
                "ten_start_full_refit_point",
                "destination_product_rescaling",
            ],
        )),
        (
            HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
            HeterogeneityInteractionProfileV2::P23AllCurrent,
        ) => Ok((
            "pos.destination_scored.p23_all_current.v2",
            &[
                "ten_start_full_refit_point",
                "destination_three_way_product_rescaling",
            ],
        )),
        combination => Err(MultiModCandidateAuthorityErrorV1::CellNotQualified(
            format!("heterogeneity {combination:?}"),
        )),
    }
}

fn conditional_profile_identity_v1(
    profile: ConditionalProcessProfileV2,
) -> (&'static str, &'static [&'static str]) {
    match profile {
        ConditionalProcessProfileV2::MultiTwoWayPercentile => (
            "conditional.multi_two_way_percentile.v2",
            &[
                "explicit_path_target_math",
                "shared_ledger_percentile_type7",
                "both_stage_multiple_long_path",
                "all_predeclared_alternatives",
            ],
        ),
        ConditionalProcessProfileV2::MultiTwoWayBca => (
            "conditional.multi_two_way_bca.v2",
            &[
                "explicit_path_target_math",
                "complete_delete_one_bca",
                "all_predeclared_alternatives",
                "incomplete_jackknife_fail_closed",
            ],
        ),
        ConditionalProcessProfileV2::MultiTwoWayStudentized => (
            "conditional.studentized.v2",
            &[
                "nested_studentized",
                "outer_inner_budget_limits",
                "no_percentile_fallback",
                "all_predeclared_alternatives",
            ],
        ),
        ConditionalProcessProfileV2::BoundedThreeWayPercentile => (
            "conditional.bounded_three_way_percentile.v2",
            &[
                "complete_lower_order_closure",
                "derivatives_and_cross_derivatives",
                "shared_ledger_percentile_type7",
                "all_predeclared_alternatives",
            ],
        ),
        ConditionalProcessProfileV2::MultipleHocPercentile => (
            "conditional.multiple_hoc_percentile.v2",
            &[
                "hoc_dependency_before_products",
                "disjoint_nonnested_single_approach",
                "shared_ledger_percentile_type7_two_sided",
            ],
        ),
        ConditionalProcessProfileV2::GroupedPercentile => (
            "conditional.grouped_percentile.v2",
            &[
                "group_stratified_shared_ledger",
                "two_to_twenty_group_bounds",
                "percentile_type7_two_sided",
            ],
        ),
        ConditionalProcessProfileV2::CaseWeightedPercentile => (
            "conditional.case_weighted_percentile.v2",
            &[
                "positive_normalized_case_weights",
                "kish_ess_and_ratio_guards",
                "row_weight_resampling",
                "percentile_type7_two_sided",
            ],
        ),
        ConditionalProcessProfileV2::FrequencyWeightedPercentile => (
            "conditional.frequency_weighted_percentile.v2",
            &[
                "count_space_point_equivalence",
                "multinomial_count_bootstrap_equivalence",
                "exact_integer_total_guard",
                "percentile_type7_two_sided",
            ],
        ),
    }
}

fn insert_heterogeneity_required_cells_v1(
    cells: &mut BTreeSet<String>,
    config: &PlsUnobservedHeterogeneityConfigV2,
    algorithms: &[HeterogeneityAlgorithmV2],
) -> Result<(), MultiModCandidateAuthorityErrorV1> {
    for algorithm in algorithms {
        let (profile_id, procedures) =
            heterogeneity_profile_identity_v1(*algorithm, config.profile)?;
        insert_profile_cells(cells, profile_id, procedures);
    }
    let HeterogeneityPhaseV2::Inference { lock } = &config.phase else {
        // Discovery publishes point estimates and diagnostics only. Its config
        // cannot carry a bootstrap, so it must never consume or claim a fixed-K
        // inference cell.
        if config.bootstrap.is_some()
            || config
                .pos_common_metric
                .as_ref()
                .is_some_and(|gate| gate.request_segment_contrasts)
        {
            return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
        }
        return Ok(());
    };
    if config.bootstrap.is_none() {
        return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
    }
    let (selected_profile_id, _) =
        heterogeneity_profile_identity_v1(lock.selected_algorithm, config.profile)?;
    cells.insert(cell(selected_profile_id, "fixed_k_label_aligned_bootstrap"));
    if config
        .pos_common_metric
        .as_ref()
        .is_some_and(|gate| gate.request_segment_contrasts)
    {
        if lock.selected_algorithm
            != HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2
        {
            return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
        }
        cells.insert(cell(selected_profile_id, "common_metric_inferential_refit"));
        let profile_id = match config.profile {
            HeterogeneityInteractionProfileV2::P2MultiTwoWay => {
                "pos.common_metric.p2_multi_two_way.v1"
            }
            HeterogeneityInteractionProfileV2::P23AllCurrent => {
                "pos.common_metric.p23_all_current.v1"
            }
            HeterogeneityInteractionProfileV2::P0Structural => {
                return Err(MultiModCandidateAuthorityErrorV1::CellNotQualified(
                    "POS common metric P0".into(),
                ));
            }
        };
        insert_profile_cells(
            cells,
            profile_id,
            &[
                "configural_and_pairwise_compositional_gate",
                "failed_gate_descriptive_only_suppression",
            ],
        );
    }
    Ok(())
}

/// Return the exact, sorted profile/procedure cells required to promote this
/// typed completed result. Heterogeneity deliberately returns multiple profile
/// cell groups when its compiled discovery inventory requested multiple
/// algorithms.
pub fn required_multimod_candidate_profile_cells_v1(
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledMultiModRecipeV1,
    result: &MultiModAnalysisResultV1,
) -> Result<Vec<String>, MultiModCandidateAuthorityErrorV1> {
    if !result_matches_compilation_authority_v1(artifact, result) {
        return Err(MultiModCandidateAuthorityErrorV1::AuthorityMismatch);
    }
    let mut cells = BTreeSet::new();
    match (artifact.receipt().target, artifact.plan(), result) {
        (
            MultiModCompilerTargetV1::MgaMultigroupV1,
            CompiledMultiModPlanV1::MgaMultigroupV1 { profile, .. },
            MultiModAnalysisResultV1::PlsMultigroupAnalysisV1(_),
        ) => {
            let config = recipe
                .mga_multigroup
                .as_ref()
                .ok_or(MultiModCandidateAuthorityErrorV1::FamilyMismatch)?;
            if config.profile != *profile {
                return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
            }
            let (profile_id, point_cells) = mga_profile_identity_v1(*profile);
            insert_profile_cells(&mut cells, profile_id, point_cells);
            for procedure in &config.procedures {
                cells.insert(cell(
                    profile_id,
                    mga_procedure_cell_v1(*profile, *procedure)?,
                ));
            }
            cells.insert(cell(profile_id, "multiplicity_adjustment"));
        }
        (
            MultiModCompilerTargetV1::PlsHeterogeneityV2,
            CompiledMultiModPlanV1::PlsHeterogeneityV2 {
                profile,
                algorithms,
                ..
            },
            MultiModAnalysisResultV1::PlsHeterogeneityAnalysisV2(value),
        ) => {
            let config = recipe
                .pls_heterogeneity
                .as_ref()
                .ok_or(MultiModCandidateAuthorityErrorV1::FamilyMismatch)?;
            if config.profile != *profile || value.profile != *profile {
                return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
            }
            let observed = value
                .candidates
                .iter()
                .filter_map(|candidate| match candidate.method {
                    HeterogeneityCandidateMethodV2::Segmentation { algorithm } => Some(algorithm),
                    HeterogeneityCandidateMethodV2::PooledBaselineV1 => None,
                })
                .collect::<BTreeSet<_>>();
            let compiled = algorithms.iter().copied().collect::<BTreeSet<_>>();
            if observed != compiled {
                return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
            }
            insert_heterogeneity_required_cells_v1(&mut cells, config, algorithms)?;
        }
        (
            MultiModCompilerTargetV1::GeneralSemConditionalProcessV2,
            CompiledMultiModPlanV1::GeneralSemConditionalProcessV2 { profile, .. },
            MultiModAnalysisResultV1::GeneralSemConditionalProcessResultV2(value),
        ) => {
            let config = recipe
                .general_sem_conditional_process
                .as_ref()
                .ok_or(MultiModCandidateAuthorityErrorV1::FamilyMismatch)?;
            let (profile_id, procedures) = conditional_profile_identity_v1(*profile);
            if config.profile != *profile || value.profile_id != profile_id {
                return Err(MultiModCandidateAuthorityErrorV1::ArtifactMismatch);
            }
            insert_profile_cells(&mut cells, profile_id, procedures);
        }
        (
            MultiModCompilerTargetV1::InterventionalCausalMediationV1,
            CompiledMultiModPlanV1::InterventionalCausalMediationV1 { .. },
            MultiModAnalysisResultV1::InterventionalMediationResultV1(_),
        ) if recipe.interventional_causal_mediation.is_some() => insert_profile_cells(
            &mut cells,
            "interventional.observed_gcomp.v1",
            &[
                "observed_equation_point_fit",
                "parametric_g_computation",
                "known_target_simulation",
                "positivity_diagnostics",
                "identification_failure_guards",
                "causal_wording_guard",
            ],
        ),
        _ => return Err(MultiModCandidateAuthorityErrorV1::FamilyMismatch),
    }
    Ok(cells.into_iter().collect())
}

/// Apply a receipt only after native code has checked it against the immutable
/// embedded authority. This function enforces exact typed-cell equality and
/// validates the resulting state/receipt coupling.
pub fn apply_multimod_candidate_qualification_v1(
    recipe: &AnalysisRecipeV4,
    artifact: &CompiledMultiModRecipeV1,
    result: &mut MultiModAnalysisResultV1,
    receipt: MultimodCandidateQualificationReceiptV1,
) -> Result<(), MultiModCandidateAuthorityErrorV1> {
    let required = required_multimod_candidate_profile_cells_v1(recipe, artifact, result)?;
    if receipt.required_profile_cells != required {
        return Err(MultiModCandidateAuthorityErrorV1::ReceiptCellMismatch);
    }
    let provenance = result.provenance_mut();
    provenance.qualification = MultimodQualificationStateV1::ReleaseQualifiedCandidate;
    provenance.candidate_qualification_receipt = Some(receipt);
    result
        .ensure_valid()
        .map_err(|_| MultiModCandidateAuthorityErrorV1::AuthorityMismatch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FimixSettingsV2, HeterogeneityInferenceLockReceiptV2, MicomConfiguralChecklistV1,
        PlsPosSettingsV2, PosCommonMetricComparabilityV1, SegmentationBootstrapV2,
    };

    fn complete_checklist() -> MicomConfiguralChecklistV1 {
        MicomConfiguralChecklistV1 {
            identical_indicators_and_coding: true,
            identical_data_treatment: true,
            identical_algorithm_settings: true,
            identical_model_specification: true,
            deterministic_sign_orientation_reviewed: true,
            analyst_review_confirmed: true,
        }
    }

    fn discovery_config() -> PlsUnobservedHeterogeneityConfigV2 {
        PlsUnobservedHeterogeneityConfigV2 {
            schema_version: 2,
            profile: HeterogeneityInteractionProfileV2::P2MultiTwoWay,
            phase: HeterogeneityPhaseV2::Discovery {
                candidate_k: vec![2, 3],
                algorithms: vec![
                    HeterogeneityAlgorithmV2::FimixPlsV2,
                    HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                ],
            },
            seed: 42,
            fimix: FimixSettingsV2::default(),
            pls_pos: PlsPosSettingsV2::default(),
            pos_common_metric: None,
            bootstrap: None,
        }
    }

    #[test]
    fn discovery_maps_each_algorithm_point_cells_without_bootstrap() {
        let config = discovery_config();
        config.ensure_valid().unwrap();
        let algorithms = match &config.phase {
            HeterogeneityPhaseV2::Discovery { algorithms, .. } => algorithms,
            _ => unreachable!(),
        };
        let mut cells = BTreeSet::new();
        insert_heterogeneity_required_cells_v1(&mut cells, &config, algorithms).unwrap();
        assert!(cells.contains("fimix.p2_multi_two_way.v2::em_multistart_point"));
        assert!(
            cells
                .contains("pos.destination_scored.p2_multi_two_way.v2::ten_start_full_refit_point")
        );
        assert!(
            cells
                .iter()
                .all(|cell| !cell.ends_with("::fixed_k_label_aligned_bootstrap"))
        );
        assert!(cells.iter().all(|cell| !cell.contains("common_metric")));
    }

    #[test]
    fn locked_pos_inference_adds_bootstrap_only_to_selected_algorithm_and_gates_contrasts() {
        let algorithms = vec![
            HeterogeneityAlgorithmV2::FimixPlsV2,
            HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
        ];
        let config = PlsUnobservedHeterogeneityConfigV2 {
            phase: HeterogeneityPhaseV2::Inference {
                lock: HeterogeneityInferenceLockReceiptV2 {
                    schema_version: 1,
                    discovery_result_identity_sha256: "a".repeat(64),
                    discovery_candidate_k: vec![2, 3],
                    discovery_algorithms: algorithms.clone(),
                    selected_algorithm:
                        HeterogeneityAlgorithmV2::PlsPosDestinationScoredInteractionsV2,
                    selected_k: 2,
                    analyst_lock_confirmed: true,
                    tandem_fimix_same_k_start_required: true,
                },
            },
            pos_common_metric: Some(PosCommonMetricComparabilityV1 {
                schema_version: 1,
                request_segment_contrasts: true,
                permutation_samples: 5_000,
                configural_checklist: complete_checklist(),
                require_partial_compositional_invariance: true,
            }),
            bootstrap: Some(SegmentationBootstrapV2 {
                resamples: 1_000,
                seed: 42,
                confidence_level: 0.95,
            }),
            ..discovery_config()
        };
        config.ensure_valid().unwrap();
        let mut cells = BTreeSet::new();
        insert_heterogeneity_required_cells_v1(&mut cells, &config, &algorithms).unwrap();
        assert!(cells.contains(
            "pos.destination_scored.p2_multi_two_way.v2::fixed_k_label_aligned_bootstrap"
        ));
        assert!(!cells.contains("fimix.p2_multi_two_way.v2::fixed_k_label_aligned_bootstrap"));
        assert!(cells.contains(
            "pos.destination_scored.p2_multi_two_way.v2::common_metric_inferential_refit"
        ));
        assert!(cells.contains(
            "pos.common_metric.p2_multi_two_way.v1::configural_and_pairwise_compositional_gate"
        ));
    }
}
