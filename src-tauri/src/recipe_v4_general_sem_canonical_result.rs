//! Canonical projection for the archive-bound General SEM PLS Labs slice.

use crate::{
    InternalRecipeV4ExecutionSurfaceV1, InternalRecipeV4PlsExecutionRequestV1,
    InternalRecipeV4ResidentDataV1,
    recipe_v4_canonical_result::build_recipe_v4_pls_canonical_result,
};
use qpls_core::{
    AnalysisRecipeV4, CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION, CanonicalAggregateEffectKindV1,
    CanonicalChartDisplayOptions, CanonicalChartKind, CanonicalChartPoint, CanonicalChartSeries,
    CanonicalChartX, CanonicalColumnRole, CanonicalColumnType, CanonicalGeneralSemEstimateV1,
    CanonicalGeneralSemResultTraceV1, CanonicalGeneralSemResultsV1,
    CanonicalJointStageStructuralCoefficientResultV1, CanonicalMissingReason, CanonicalResultCell,
    CanonicalResultChart, CanonicalResultColumn, CanonicalResultDocumentV2,
    CanonicalResultExclusion, CanonicalResultRow, CanonicalResultSection, CanonicalResultTable,
    CanonicalStructuralEstimateStageV1, CanonicalStructuralRelationRoleV1,
    CapabilityCellReferenceV2, GENERAL_SEM_EFFECTS_V1_METHOD_VERSION,
    GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1,
    GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1, RecipeV4CompilerTarget, SemModelV4,
    SemParameterV4, StructuralRelationRoleV4, canonical_general_sem_effect_identities_v1,
    compile_general_sem_pls_recipe_v1, general_sem_effect_identity_set_sha256_v1,
    pls_general_bootstrap_capability_cell_v1,
    pls_general_multiple_moderation_bootstrap_capability_cell_v1,
    pls_general_multiple_moderation_point_capability_cell_v1,
    pls_general_recursive_effects_capability_cell_v1, project_general_sem_pls_stage_one_recipe_v1,
    validate_canonical_result_document_v2,
};
use qpls_runner::{
    RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
    RecipeV4GeneralSemPlsExecutionResultV1,
};
use uuid::Uuid;

const GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1: &str = "general_sem_interaction_effects";
const GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1: &str = "general_sem_conditional_slopes";
const GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1: &str = "general_sem_interaction_plots";
const GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1: &str = "general_sem_moderation_bootstrap";
const GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1: &str =
    "general_sem_moderation_gamma_inference";
const GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1: &str =
    "general_sem_moderation_bootstrap_receipt";

pub(crate) fn general_sem_multiple_mediation_bootstrap_capability_cell_v1()
-> CapabilityCellReferenceV2 {
    pls_general_bootstrap_capability_cell_v1()
}

pub(crate) fn general_sem_multiple_moderation_point_capability_cell_v1() -> CapabilityCellReferenceV2
{
    pls_general_multiple_moderation_point_capability_cell_v1()
}

pub(crate) fn general_sem_multiple_moderation_bootstrap_capability_cell_v1()
-> CapabilityCellReferenceV2 {
    pls_general_multiple_moderation_bootstrap_capability_cell_v1()
}

pub(crate) fn validate_archived_general_sem_pls_method_identity_v1(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let moderation_cell = general_sem_multiple_moderation_point_capability_cell_v1();
    let bootstrap_cell = general_sem_multiple_moderation_bootstrap_capability_cell_v1();
    let project_cell_matches =
        |candidate: &qpls_project::CapabilityCellReferenceV2,
         expected: &CapabilityCellReferenceV2| {
            candidate.registry_schema_version == expected.registry_schema_version
                && candidate.capability_id == expected.capability_id
                && candidate.cell_id == expected.cell_id
                && candidate.capability_version == expected.capability_version
        };
    let project_cell_is_moderation = |candidate: &qpls_project::CapabilityCellReferenceV2| {
        project_cell_matches(candidate, &moderation_cell)
    };
    let project_cell_is_bootstrap = |candidate: &qpls_project::CapabilityCellReferenceV2| {
        project_cell_matches(candidate, &bootstrap_cell)
    };
    let cell = &document.provenance.capability_cell;
    let is_moderation_cell = project_cell_is_moderation(cell);
    let has_moderation_artifact = document
        .general_sem_results
        .as_ref()
        .is_some_and(|results| {
            results
                .joint_stage_structural_coefficients
                .iter()
                .any(|coefficient| coefficient.trace.capability_cell == moderation_cell)
                || results
                    .interaction_effects
                    .iter()
                    .any(|effect| effect.trace.capability_cell == moderation_cell)
                || results
                    .conditional_effect_probes
                    .iter()
                    .any(|probe| probe.trace.capability_cell == moderation_cell)
                || results
                    .conditional_effects
                    .iter()
                    .any(|effect| effect.trace.capability_cell == moderation_cell)
                || results
                    .interaction_plots
                    .iter()
                    .any(|plot| plot.trace.capability_cell == moderation_cell)
        })
        || document.sections.iter().any(|section| {
            matches!(
                section.id.as_str(),
                "general_sem_moderation" | GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1
            ) && section.capability_cells.as_ref().is_some_and(|cells| {
                cells
                    .iter()
                    .any(|cell| project_cell_is_moderation(cell) || project_cell_is_bootstrap(cell))
            })
        })
        || document.tables.iter().any(|table| {
            matches!(
                table.id.as_str(),
                GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1
                    | GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1
                    | GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1
                    | GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1
                    | GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1
            ) && table.capability_cells.as_ref().is_some_and(|cells| {
                cells
                    .iter()
                    .any(|cell| project_cell_is_moderation(cell) || project_cell_is_bootstrap(cell))
            })
        })
        || document.exclusions.iter().any(|exclusion| {
            matches!(
                exclusion.id.as_str(),
                "moderation_point_estimation_only"
                    | "moderation_bootstrap_scientific_gamma_only"
                    | "moderation_beta_joint_coefficients_slopes_plots_point_only"
                    | "joint_stage_two_effects_and_fit_not_available"
            ) && exclusion.capability_cell.as_ref().is_some_and(|cell| {
                project_cell_is_moderation(cell) || project_cell_is_bootstrap(cell)
            })
        })
        || document.provenance.method_version
            == GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1
        || document.provenance.method_version
            == GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
        || document.provenance.engine_version
            == RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1
        || document.provenance.engine_version
            == RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1;
    if !is_moderation_cell && !has_moderation_artifact {
        return Ok(());
    }
    if !is_moderation_cell {
        return Err(
            "archived non-moderation document contains injected General SEM moderation artifacts"
                .into(),
        );
    }
    let results = document.general_sem_results.as_ref().ok_or_else(|| {
        "archived General SEM moderation document omits its typed scientific payload".to_string()
    })?;
    let is_bootstrap = results.inference_receipt.is_some();
    let (expected_method, expected_adapter, expected_title) = if is_bootstrap {
        (
            GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
            RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            "General SEM simultaneous two-way PLS moderation bootstrap inference",
        )
    } else {
        (
            GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
            RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
            "General SEM simultaneous two-way PLS moderation point estimates",
        )
    };
    if document.provenance.method_version != expected_method
        || document.provenance.engine_version != expected_adapter
    {
        return Err(
            "archived General SEM moderation document has a stale method or execution-adapter identity"
                .into(),
        );
    }
    if document.title != expected_title {
        return Err("archived General SEM moderation document has an unexpected title".into());
    }
    if results.joint_stage_structural_coefficients.is_empty()
        || results.interaction_effects.is_empty()
        || results.conditional_effect_probes.len() != results.interaction_effects.len()
        || results.conditional_effects.len() != results.interaction_effects.len() * 3
        || results.interaction_plots.len() != results.interaction_effects.len()
        || !results.specific_indirect_effects.is_empty()
        || !results.aggregate_effects.is_empty()
    {
        return Err(
            "archived General SEM moderation typed payload is incomplete or exceeds the exact point/bootstrap inventories"
                .into(),
        );
    }
    if is_bootstrap
        && results.inference_receipt.as_ref().is_none_or(|receipt| {
            receipt.capability_cell != bootstrap_cell
                || receipt.method_version
                    != GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1
        })
    {
        return Err(
            "archived General SEM moderation-bootstrap receipt has a stale supplemental cell or method identity"
                .into(),
        );
    }
    let point_table_ids = [
        GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1,
        GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1,
        GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1,
    ];
    let point_section = document
        .sections
        .iter()
        .find(|section| section.id == "general_sem_moderation")
        .ok_or_else(|| {
            "archived General SEM moderation document omits its result section".to_string()
        })?;
    let bootstrap_section = document
        .sections
        .iter()
        .find(|section| section.id == GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1);
    let expected_last_section = if is_bootstrap {
        GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1
    } else {
        "general_sem_moderation"
    };
    let expected_default_table = if is_bootstrap {
        GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1
    } else {
        GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1
    };
    if document.sections.last().map(|section| section.id.as_str()) != Some(expected_last_section)
        || point_section
            .table_ids
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            != point_table_ids
        || point_section.chart_ids.len() != results.interaction_plots.len()
        || document.presentation.default_section_id.as_deref() != Some(expected_last_section)
        || document.presentation.default_table_id.as_deref() != Some(expected_default_table)
        || (is_bootstrap
            && bootstrap_section.is_none_or(|section| {
                section.table_ids.iter().map(String::as_str).ne([
                    GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
                    GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
                ]) || !section.chart_ids.is_empty()
                    || section.capability_cells.as_deref().is_none_or(
                        |cells| !matches!(cells, [cell] if project_cell_is_bootstrap(cell)),
                    )
            }))
        || (!is_bootstrap && bootstrap_section.is_some())
    {
        return Err(
            "archived General SEM moderation section, ordering, or presentation binding is invalid"
                .into(),
        );
    }
    let mut expected_table_suffix = point_table_ids.to_vec();
    if is_bootstrap {
        expected_table_suffix.extend([
            GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
            GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
        ]);
    }
    let table_suffix = document
        .tables
        .iter()
        .rev()
        .take(expected_table_suffix.len())
        .map(|table| table.id.as_str())
        .collect::<Vec<_>>();
    if table_suffix
        != expected_table_suffix
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>()
    {
        return Err("archived General SEM moderation table order is invalid".into());
    }
    let mut expected_columns: Vec<(&str, &[&str])> = vec![
        (
            GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1,
            &[
                "effect_id",
                "interaction_id",
                "focal_relation_id",
                "interaction_effect_relation_id",
                "interaction_effect_parameter_id",
                "focal_predictor_id",
                "moderator_id",
                "outcome_id",
                "generated_product_column_id",
                "stage_one_model_scientific_sha256",
                "observation_count",
                "standardized_product_coefficient",
                "scientific_rescaled_gamma",
                "product_mean",
                "product_sample_sd",
                "construction_method",
                "product_scale_version",
                "hierarchy_policy",
                "hierarchy_policy_version",
                "conditioning_policy_version",
                "method_version",
            ],
        ),
        (
            GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1,
            &[
                "effect_id",
                "interaction_id",
                "interaction_effect_id",
                "focal_relation_id",
                "probe_id",
                "probe_value_index",
                "moderator_id",
                "outcome_id",
                "moderator_value",
                "estimate",
                "conditioning_policy_version",
            ],
        ),
        (
            GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1,
            &[
                "plot_id",
                "interaction_id",
                "interaction_effect_id",
                "focal_relation_id",
                "focal_predictor_id",
                "moderator_id",
                "outcome_id",
                "series_id",
                "probe_id",
                "probe_value_index",
                "moderator_value",
                "focal_value",
                "predicted_value",
                "lower",
                "upper",
            ],
        ),
    ];
    if is_bootstrap {
        expected_columns.extend([
            (
                GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1,
                &[
                    "effect_id",
                    "interaction_id",
                    "focal_relation_id",
                    "interaction_effect_relation_id",
                    "interaction_effect_parameter_id",
                    "generated_product_column_id",
                    "focal_predictor_id",
                    "moderator_id",
                    "outcome_id",
                    "stage_one_model_scientific_sha256",
                    "product_scale_version",
                    "point_method_version",
                    "estimate",
                    "bootstrap_mean",
                    "bootstrap_bias",
                    "standard_error",
                    "lower",
                    "upper",
                    "p_value",
                    "bootstrap_usable_replicates",
                    "bootstrap_two_sided_exceedances",
                ] as &[&str],
            ),
            (
                GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1,
                &[
                    "capability_id",
                    "cell_id",
                    "capability_version",
                    "method_version",
                    "point_method_version",
                    "resampling_operation_version",
                    "resampling_stream_version",
                    "quantile_method_version",
                    "standard_error_method_version",
                    "summation_method_version",
                    "p_value_method_version",
                    "failure_policy_version",
                    "sign_alignment_method_version",
                    "product_scale_version",
                    "gamma_target_version",
                    "compilation_artifact_identity_sha256",
                    "compiled_plan_sha256",
                    "general_sem_config_sha256",
                    "recipe_analytical_sha256",
                    "model_scientific_sha256",
                    "stage_one_model_scientific_sha256",
                    "source_dataset_fingerprint",
                    "complete_case_frame_sha256",
                    "usable_replicate_indices_sha256",
                    "gamma_target_identity_set_sha256",
                    "interval",
                    "tail",
                    "confidence_level",
                    "resamples_requested",
                    "resamples_usable",
                    "minimum_usable_resamples",
                    "seed",
                    "workers",
                    "complete_model_reestimated_per_replicate",
                    "shared_stage_one_reestimated_per_replicate",
                    "score_vectors_sign_aligned_before_products",
                    "product_scaling_recomputed_per_replicate",
                    "joint_stage_two_reestimated_per_replicate",
                    "complete_joint_point_contract_validated_per_replicate",
                    "failed_replicate_count",
                ] as &[&str],
            ),
        ]);
    }
    for (table_id, columns) in expected_columns {
        let matches = document
            .tables
            .iter()
            .filter(|table| table.id == table_id)
            .collect::<Vec<_>>();
        let [table] = matches.as_slice() else {
            return Err(format!(
                "archived General SEM moderation document requires exactly one {table_id} table"
            ));
        };
        if table
            .columns
            .iter()
            .map(|column| column.id.as_str())
            .collect::<Vec<_>>()
            != columns
        {
            return Err(format!(
                "archived General SEM moderation table {table_id} has an invalid column contract"
            ));
        }
    }
    let expected_chart_ids = results
        .interaction_plots
        .iter()
        .enumerate()
        .map(|(index, _)| format!("general_sem_interaction_chart_{index:04}"))
        .collect::<Vec<_>>();
    if document.charts.len() != expected_chart_ids.len()
        || document
            .charts
            .iter()
            .map(|chart| chart.id.as_str())
            .ne(expected_chart_ids.iter().map(String::as_str))
        || point_section.chart_ids != expected_chart_ids
        || document.charts.iter().any(|chart| {
            chart.source_table_id.as_deref() != Some(GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1)
                || chart.series.iter().any(|series| {
                    series
                        .points
                        .iter()
                        .any(|point| point.lower.is_some() || point.upper.is_some())
                })
        })
        || results.interaction_plots.iter().any(|plot| {
            plot.series.iter().any(|series| {
                series
                    .points
                    .iter()
                    .any(|point| point.lower.is_some() || point.upper.is_some())
            })
        })
    {
        return Err("archived General SEM moderation chart binding is invalid".into());
    }
    let expected_exclusions: &[&str] = if is_bootstrap {
        &[
            "moderation_bootstrap_scientific_gamma_only",
            "moderation_beta_joint_coefficients_slopes_plots_point_only",
            "joint_stage_two_effects_and_fit_not_available",
        ]
    } else {
        &[
            "moderation_point_estimation_only",
            "joint_stage_two_effects_and_fit_not_available",
        ]
    };
    if document
        .exclusions
        .iter()
        .map(|exclusion| exclusion.id.as_str())
        .collect::<Vec<_>>()
        != expected_exclusions
        || document.exclusions.iter().any(|exclusion| {
            exclusion.capability_cell.as_ref().is_none_or(|cell| {
                if is_bootstrap {
                    !project_cell_is_bootstrap(cell)
                } else {
                    !project_cell_is_moderation(cell)
                }
            })
        })
    {
        return Err("archived General SEM moderation exclusions are incomplete".into());
    }
    document.ensure_valid().map_err(|error| error.to_string())
}

pub(crate) fn build_recipe_v4_general_sem_pls_canonical_result_v1(
    job_id: Uuid,
    project_id: Uuid,
    dataset_id: Uuid,
    started_at: &str,
    completed_at: &str,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    result: &RecipeV4GeneralSemPlsExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, Vec<String>> {
    let interaction_point = result.interaction_point_estimation();
    let moderation_bootstrap = result.moderation_bootstrap_inference();
    let moderation_artifact = interaction_point
        .map(|interactions| {
            let artifact = compile_general_sem_pls_recipe_v1(recipe, Some(model)).map_err(
                |error| vec![format!("General SEM moderation recompilation failed: {error}")],
            )?;
            let moderation_cell = general_sem_multiple_moderation_point_capability_cell_v1();
            if artifact.plan().two_way_interactions().is_empty()
                || artifact.capability_cell() != &moderation_cell
                || result.capability_cell() != &moderation_cell
                || result.compiled_plan_sha256() != artifact.plan().deterministic_sha256()
                || result.model_scientific_sha256() != artifact.plan().scientific_hash()
            {
                return Err(vec![
                    "General SEM moderation result differs from its exact compiled plan or capability cell"
                        .into(),
                ]);
            }
            interactions
                .ensure_valid_against_plan_v1(artifact.plan())
                .map_err(|error| {
                    vec![format!(
                        "General SEM moderation point result failed compiled-plan validation: {error}"
                    )]
                })?;
            if let Some(bootstrap) = moderation_bootstrap {
                if artifact.compiler_version()
                    != GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_RECIPE_COMPILER_VERSION_V1
                    || !matches!(
                        recipe
                            .general_sem_config
                            .as_ref()
                            .map(|config| &config.inference),
                        Some(qpls_core::GeneralSemInferenceV1::CaseBootstrap { .. })
                    )
                {
                    return Err(vec![
                        "General SEM moderation bootstrap result lacks its exact resident compiler/config authority"
                            .into(),
                    ]);
                }
                bootstrap
                    .ensure_valid_against_plan_v1(artifact.plan(), interactions)
                    .map_err(|error| {
                        vec![format!(
                            "General SEM moderation bootstrap failed compiled-plan validation: {error}"
                        )]
                    })?;
            }
            Ok(artifact)
        })
        .transpose()?;
    let moderation = moderation_artifact.is_some();
    let (base_recipe, stage_one_model) = project_general_sem_pls_stage_one_recipe_v1(recipe, model)
        .map_err(|error| vec![format!("General SEM stage-one projection failed: {error}")])?;
    let base_request = InternalRecipeV4PlsExecutionRequestV1 {
        surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
        experimental_labs_enabled: true,
        resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
        dataset_id: dataset_id.to_string(),
        dataset_fingerprint: result.source_dataset_fingerprint().into(),
        recipe: base_recipe,
        model: stage_one_model,
        compiler_target: RecipeV4CompilerTarget::PlsPlanV2,
        capability_cell: RecipeV4CompilerTarget::PlsPlanV2
            .capability_cell_for_method(recipe.settings.method),
        posthoc_technical_minimum_sample_size: None,
    };
    let mut document = build_recipe_v4_pls_canonical_result(
        job_id,
        project_id,
        started_at,
        completed_at,
        &base_request,
        result.point_estimation(),
    )?;
    let mut general_sem_results = result
        .canonical_general_sem_results_v1()
        .map_err(|error| vec![format!("General SEM canonical projection failed: {error}")])?;
    if let (Some(artifact), Some(interactions)) = (moderation_artifact.as_ref(), interaction_point)
    {
        general_sem_results.joint_stage_structural_coefficients =
            canonical_joint_stage_structural_coefficients_v1(
                artifact.plan(),
                interactions,
                &model.id,
                &general_sem_multiple_moderation_point_capability_cell_v1(),
            )?;
    }
    let inferred = general_sem_results.inference_receipt.is_some();
    if moderation && inferred != moderation_bootstrap.is_some() {
        return Err(vec![
            "The typed moderation inference receipt and raw bootstrap result disagree".into(),
        ]);
    }

    document.schema_version = CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
    document.title = if moderation && inferred {
        "General SEM simultaneous two-way PLS moderation bootstrap inference".into()
    } else if moderation {
        "General SEM simultaneous two-way PLS moderation point estimates".into()
    } else if inferred {
        "PLS-SEM multiple mediation with full-model bootstrap".into()
    } else {
        "PLS-SEM mediation effects".into()
    };
    document.provenance.model_id = model.id.clone();
    document.provenance.model_digest = result.model_scientific_sha256().into();
    document.provenance.dataset_id = dataset_id.to_string();
    document.provenance.dataset_fingerprint = result.source_dataset_fingerprint().into();
    document.provenance.recipe_id = recipe.id.to_string();
    document.provenance.recipe_digest = result.recipe_analytical_sha256().into();
    let point_cell = pls_general_recursive_effects_capability_cell_v1();
    let moderation_cell = general_sem_multiple_moderation_point_capability_cell_v1();
    document.provenance.capability_cell = if moderation {
        moderation_cell.clone()
    } else {
        point_cell.clone()
    };
    document.provenance.method_version = if moderation && inferred {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1.into()
    } else if moderation {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.into()
    } else if inferred {
        GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1.into()
    } else {
        GENERAL_SEM_EFFECTS_V1_METHOD_VERSION.into()
    };
    document.provenance.engine_version = result.adapter_version().into();
    document.provenance.seed = Some(i64::try_from(recipe.settings.seed).map_err(|_| {
        vec!["General SEM recipe seed exceeds the canonical safe-integer boundary".into()]
    })?);
    document.provenance.workers = i64::try_from(recipe.settings.workers)
        .ok()
        .filter(|workers| *workers > 0)
        .ok_or_else(|| vec!["General SEM recipe worker count is invalid".into()])?;

    let base_cell =
        RecipeV4CompilerTarget::PlsPlanV2.capability_cell_for_method(recipe.settings.method);
    let bootstrap_combination_cell = if moderation {
        general_sem_multiple_moderation_bootstrap_capability_cell_v1()
    } else {
        general_sem_multiple_mediation_bootstrap_capability_cell_v1()
    };
    let primary_cell = if moderation {
        moderation_cell.clone()
    } else {
        point_cell.clone()
    };
    let mut document_cells = vec![base_cell, primary_cell.clone()];
    if inferred {
        document_cells.push(bootstrap_combination_cell.clone());
    }
    sort_and_deduplicate_cells(&mut document_cells);
    document.capability_cells = Some(document_cells);

    let mut general_sem_table_cells = vec![primary_cell.clone()];
    if inferred && !moderation {
        general_sem_table_cells.push(bootstrap_combination_cell.clone());
        sort_and_deduplicate_cells(&mut general_sem_table_cells);
    }
    if let (Some(artifact), Some(interactions)) = (moderation_artifact.as_ref(), interaction_point)
    {
        project_joint_stage_two_structural_tables_v1(
            &mut document,
            artifact.plan(),
            interactions,
            model,
            &moderation_cell,
        )?;
    }
    let general_sem_tables = if moderation {
        moderation_tables(&general_sem_results, &general_sem_table_cells)?
    } else {
        general_sem_tables(&general_sem_results, &general_sem_table_cells)
    };
    let table_ids = general_sem_tables
        .iter()
        .map(|table| table.id.clone())
        .collect::<Vec<_>>();
    document.tables.extend(general_sem_tables);
    let moderation_charts = if moderation {
        moderation_charts(&general_sem_results)
    } else {
        Vec::new()
    };
    let chart_ids = moderation_charts
        .iter()
        .map(|chart| chart.id.clone())
        .collect::<Vec<_>>();
    document.charts.extend(moderation_charts);
    document.sections.push(CanonicalResultSection {
        id: if moderation {
            "general_sem_moderation".into()
        } else {
            "general_sem_effects".into()
        },
        title: if moderation {
            "Moderation effects".into()
        } else {
            "Mediation effects".into()
        },
        description: Some(if moderation {
            "Exact simultaneous two-way interaction coefficients, product scaling receipts, conditional slopes, and typed interaction plots from the final joint stage-two solve."
                .into()
        } else {
            "Stable specific-indirect, total-indirect, and total-effect estimands from the authored recursive SEM topology."
                .into()
        }),
        table_ids,
        chart_ids,
        capability_cells: Some(general_sem_table_cells),
    });
    if let (Some(bootstrap), true) = (moderation_bootstrap, moderation) {
        let bootstrap_tables = moderation_bootstrap_tables(
            &general_sem_results,
            bootstrap,
            result,
            &bootstrap_combination_cell,
        )?;
        let bootstrap_table_ids = bootstrap_tables
            .iter()
            .map(|table| table.id.clone())
            .collect::<Vec<_>>();
        document.tables.extend(bootstrap_tables);
        document.sections.push(CanonicalResultSection {
            id: GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1.into(),
            title: "Moderation bootstrap inference".into(),
            description: Some(
                "Gamma-only percentile case-bootstrap inference for every simultaneous two-way interaction; all ordinary coefficients, standardized product betas, conditional slopes, and plots remain point estimates."
                    .into(),
            ),
            table_ids: bootstrap_table_ids,
            chart_ids: Vec::new(),
            capability_cells: Some(vec![bootstrap_combination_cell.clone()]),
        });
    }
    document.general_sem_results = Some(general_sem_results);
    document.exclusions = if moderation && inferred {
        vec![
            CanonicalResultExclusion {
                id: "moderation_bootstrap_scientific_gamma_only".into(),
                capability_cell: Some(bootstrap_combination_cell.clone()),
                title: "Bootstrap inference is limited to scientific gamma".into(),
                reason: "This exact supplemental cell infers only the scientific rescaled interaction gamma for every compiled two-way interaction."
                    .into(),
            },
            CanonicalResultExclusion {
                id: "moderation_beta_joint_coefficients_slopes_plots_point_only".into(),
                capability_cell: Some(bootstrap_combination_cell.clone()),
                title: "Other moderation surfaces remain point-only".into(),
                reason: "Standardized product betas, joint ordinary/control coefficients, conditional slopes, and interaction plots remain point estimates without interval bands."
                    .into(),
            },
            CanonicalResultExclusion {
                id: "joint_stage_two_effects_and_fit_not_available".into(),
                capability_cell: Some(bootstrap_combination_cell),
                title: "Joint-model effects and fit not estimated".into(),
                reason: "Final joint stage-two coefficients are published, but total effects and R-squared remain omitted without a qualified joint-model decomposition or fit receipt."
                    .into(),
            },
        ]
    } else if moderation {
        vec![
            CanonicalResultExclusion {
                id: "moderation_point_estimation_only".into(),
                capability_cell: Some(moderation_cell.clone()),
                title: "Moderation inference not qualified".into(),
                reason: "The exact Labs cell publishes final joint stage-two point estimates only; interaction bootstrap, three-way moderation, moderated mediation, and authored arbitrary probes remain blocked."
                    .into(),
            },
            CanonicalResultExclusion {
                id: "joint_stage_two_effects_and_fit_not_available".into(),
                capability_cell: Some(moderation_cell),
                title: "Joint-model effects and fit not estimated".into(),
                reason: "Final joint stage-two ordinary and control coefficients are published, but total effects and R-squared are omitted because this point cell has no qualified joint-model decomposition or fit receipt."
                    .into(),
            },
        ]
    } else if inferred {
        Vec::new()
    } else {
        vec![CanonicalResultExclusion {
            id: "inference_not_requested".into(),
            capability_cell: Some(point_cell),
            title: "Inference not requested".into(),
            reason: "The resident GeneralSemConfigV1 requested point effects without case-bootstrap inference."
                .into(),
        }]
    };
    document.presentation.default_section_id = Some(if moderation && inferred {
        GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1.into()
    } else if moderation {
        "general_sem_moderation".into()
    } else {
        "general_sem_effects".into()
    });
    document.presentation.default_table_id = Some(if moderation && inferred {
        GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1.into()
    } else {
        document
            .sections
            .last()
            .and_then(|section| section.table_ids.first().cloned())
            .ok_or_else(|| vec!["General SEM result section has no default table".into()])?
    });

    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(validation.errors);
    }
    let archive_value = serde_json::to_value(&document)
        .map_err(|error| vec![format!("canonical result serialization failed: {error}")])?;
    let archive_document =
        serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(archive_value)
            .map_err(|error| vec![format!("schema-6 canonical result parsing failed: {error}")])?;
    archive_document.ensure_valid().map_err(|error| {
        vec![format!(
            "schema-6 General SEM canonical validation failed: {error}"
        )]
    })?;
    Ok(document)
}

fn canonical_joint_stage_structural_coefficients_v1(
    plan: &qpls_core::CompiledPlsPlanV3,
    interactions: &qpls_estimation::GeneralSemPlsMultipleInteractionPointResultV1,
    model_id: &str,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<Vec<CanonicalJointStageStructuralCoefficientResultV1>, Vec<String>> {
    let relations = plan
        .topology()
        .structural_relations()
        .iter()
        .map(|relation| (relation.relation_id(), relation))
        .collect::<std::collections::BTreeMap<_, _>>();
    let interaction_relation_ids = plan
        .two_way_interactions()
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id())
        .collect::<std::collections::BTreeSet<_>>();
    let expected_relation_ids = relations
        .keys()
        .copied()
        .filter(|relation_id| !interaction_relation_ids.contains(relation_id))
        .collect::<std::collections::BTreeSet<_>>();
    let mut observed_relation_ids = std::collections::BTreeSet::new();
    let mut coefficients = Vec::with_capacity(interactions.structural_coefficients().len());
    for coefficient in interactions.structural_coefficients() {
        let relation = relations.get(coefficient.relation_id()).ok_or_else(|| {
            vec![format!(
                "Final joint-stage coefficient {} is absent from the compiled topology",
                coefficient.relation_id()
            )]
        })?;
        if interaction_relation_ids.contains(coefficient.relation_id())
            || coefficient.source_id() != relation.source()
            || coefficient.target_id() != relation.target()
            || !coefficient.estimate().is_finite()
            || !observed_relation_ids.insert(coefficient.relation_id())
        {
            return Err(vec![format!(
                "Final joint-stage coefficient {} contradicts or duplicates its compiled ordinary relation",
                coefficient.relation_id()
            )]);
        }
        coefficients.push(CanonicalJointStageStructuralCoefficientResultV1 {
            relation_id: coefficient.relation_id().into(),
            parameter_id: relation.parameter_id().into(),
            trace: CanonicalGeneralSemResultTraceV1 {
                model_id: model_id.into(),
                capability_cell: capability_cell.clone(),
            },
            source_id: coefficient.source_id().into(),
            target_id: coefficient.target_id().into(),
            role: match relation.role() {
                StructuralRelationRoleV4::Structural => {
                    CanonicalStructuralRelationRoleV1::Structural
                }
                StructuralRelationRoleV4::Control => CanonicalStructuralRelationRoleV1::Control,
            },
            estimate: CanonicalGeneralSemEstimateV1 {
                estimate: coefficient.estimate(),
                bootstrap_mean: None,
                bootstrap_bias: None,
                standard_error: None,
                lower: None,
                upper: None,
                p_value: None,
                bootstrap_usable_replicates: None,
                bootstrap_two_sided_exceedances: None,
            },
            stage: CanonicalStructuralEstimateStageV1::JointStageTwo,
            method_version: GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.into(),
        });
    }
    if observed_relation_ids != expected_relation_ids {
        return Err(vec![
            "The final joint-stage structural coefficient ledger does not cover every compiled ordinary structural/control relation exactly once"
                .into(),
        ]);
    }
    coefficients.sort_by(|left, right| left.relation_id.cmp(&right.relation_id));
    Ok(coefficients)
}

fn project_joint_stage_two_structural_tables_v1(
    document: &mut CanonicalResultDocumentV2,
    plan: &qpls_core::CompiledPlsPlanV3,
    interactions: &qpls_estimation::GeneralSemPlsMultipleInteractionPointResultV1,
    model: &SemModelV4,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), Vec<String>> {
    let relations = plan
        .topology()
        .structural_relations()
        .iter()
        .map(|relation| (relation.relation_id(), relation))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut ordinary_rows = Vec::new();
    let mut control_rows = Vec::new();
    for coefficient in interactions.structural_coefficients() {
        let relation = relations.get(coefficient.relation_id()).ok_or_else(|| {
            vec![format!(
                "Final joint-stage coefficient {} is absent from the compiled topology",
                coefficient.relation_id()
            )]
        })?;
        if coefficient.source_id() != relation.source()
            || coefficient.target_id() != relation.target()
            || !coefficient.estimate().is_finite()
        {
            return Err(vec![format!(
                "Final joint-stage coefficient {} contradicts its compiled relation",
                coefficient.relation_id()
            )]);
        }
        match relation.role() {
            StructuralRelationRoleV4::Structural => ordinary_rows.push(CanonicalResultRow {
                id: format!("joint_path_{:04}", ordinary_rows.len()),
                cells: vec![
                    text(coefficient.relation_id()),
                    text(relation.parameter_id()),
                    text(coefficient.source_id()),
                    text(coefficient.target_id()),
                    number(coefficient.estimate()),
                ],
            }),
            StructuralRelationRoleV4::Control => control_rows.push(CanonicalResultRow {
                id: format!("joint_control_{:04}", control_rows.len()),
                cells: vec![
                    text(coefficient.relation_id()),
                    text(relation.parameter_id()),
                    text(coefficient.source_id()),
                    text(coefficient.target_id()),
                    text(parameter_label(model, relation.parameter_id()).unwrap_or("Control")),
                    number(coefficient.estimate()),
                ],
            }),
        }
    }
    if ordinary_rows.is_empty() {
        return Err(vec![
            "The final joint stage-two result contains no ordinary structural coefficients".into(),
        ]);
    }

    let structural_table = document
        .tables
        .iter_mut()
        .find(|table| table.id == "structural_paths")
        .ok_or_else(|| vec!["Base canonical result omitted structural_paths".into()])?;
    structural_table.title = "Joint stage-two structural paths".into();
    structural_table.description = Some(
        "Final ordinary coefficients from the simultaneous joint stage-two solve; stage-one structural coefficients are not published as final moderation results."
            .into(),
    );
    structural_table.columns = vec![
        text_column(
            "relation_id",
            "Relation ID",
            "Stable authored structural relation identity.",
        ),
        text_column(
            "parameter_id",
            "Parameter ID",
            "Stable authored regression parameter identity.",
        ),
        text_column("source", "Source", "Predictor construct identifier."),
        text_column("target", "Target", "Outcome construct identifier."),
        number_column(
            "coefficient",
            "Coefficient",
            "Final joint stage-two ordinary coefficient.",
        ),
    ];
    structural_table.rows = ordinary_rows;
    structural_table.capability_cells = Some(vec![capability_cell.clone()]);

    let stale_table_ids = [
        "effects",
        "r_squared",
        qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2,
        qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
    ];
    document
        .tables
        .retain(|table| !stale_table_ids.contains(&table.id.as_str()));

    if !control_rows.is_empty() {
        document.tables.push(CanonicalResultTable {
            id: qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2.into(),
            title: "Joint stage-two control effects".into(),
            description: Some(
                "Final control-role coefficients from the simultaneous joint stage-two solve."
                    .into(),
            ),
            columns: vec![
                text_column(
                    "relation_id",
                    "Relation ID",
                    "Stable authored control relation identity.",
                ),
                text_column(
                    "parameter_id",
                    "Parameter ID",
                    "Stable authored control parameter identity.",
                ),
                text_column("source", "Source", "Control construct identifier."),
                text_column(
                    "target",
                    "Target",
                    "Controlled outcome construct identifier.",
                ),
                text_column("label", "Label", "Canonical control parameter label."),
                number_column(
                    "coefficient",
                    "Coefficient",
                    "Final joint stage-two control coefficient.",
                ),
            ],
            rows: control_rows,
            footnote_ids: Vec::new(),
            capability_cells: Some(vec![capability_cell.clone()]),
        });
    }

    let stage_one_table_ids = document
        .sections
        .iter()
        .filter(|section| matches!(section.id.as_str(), "run_details" | "measurement_model"))
        .flat_map(|section| section.table_ids.iter().cloned())
        .filter(|table_id| table_id != qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1)
        .collect::<std::collections::BTreeSet<_>>();
    for table in &mut document.tables {
        if stage_one_table_ids.contains(&table.id) {
            table.title = format!("Stage-one {}", table.title.to_lowercase());
            table.description = Some(format!(
                "Stage-one construct-score estimation only; not a final joint moderation coefficient or fit result. {}",
                table.description.as_deref().unwrap_or_default()
            ));
        }
    }
    let has_joint_control_table = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    for section in &mut document.sections {
        section
            .table_ids
            .retain(|table_id| !stale_table_ids.contains(&table_id.as_str()));
        match section.id.as_str() {
            "run_details" => {
                section.title = "Stage-one score estimation".into();
                section.description = Some(
                    "Convergence and observation accounting for the stage-one construct-score model."
                        .into(),
                );
            }
            "measurement_model" => {
                section.title = "Stage-one measurement model".into();
                section.description = Some(
                    "Outer-model estimates used to produce the construct scores consumed by the joint stage-two moderation solve."
                        .into(),
                );
            }
            "structural_model" => {
                section.title = "Joint stage-two structural model".into();
                section.description = Some(
                    "Final ordinary and control coefficients from the simultaneous joint stage-two solve. Effects and R-squared are omitted without a qualified joint-model receipt."
                        .into(),
                );
                section.table_ids = vec!["structural_paths".into()];
                if has_joint_control_table {
                    section
                        .table_ids
                        .push(qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2.into());
                }
                section.capability_cells = Some(vec![capability_cell.clone()]);
            }
            _ => {}
        }
    }
    Ok(())
}

fn parameter_label<'a>(model: &'a SemModelV4, parameter_id: &str) -> Option<&'a str> {
    model
        .parameters
        .iter()
        .find(|parameter| parameter.id() == parameter_id)
        .map(|parameter| match parameter {
            SemParameterV4::Free { label, .. }
            | SemParameterV4::Fixed { label, .. }
            | SemParameterV4::Derived { label, .. } => label.as_str(),
        })
}

fn moderation_tables(
    results: &CanonicalGeneralSemResultsV1,
    capability_cells: &[CapabilityCellReferenceV2],
) -> Result<Vec<CanonicalResultTable>, Vec<String>> {
    let capability_cells = Some(capability_cells.to_vec());
    let interactions_by_id = results
        .interaction_effects
        .iter()
        .map(|effect| (effect.interaction_id.as_str(), effect))
        .collect::<std::collections::BTreeMap<_, _>>();
    if interactions_by_id.is_empty()
        || results
            .interaction_plots
            .iter()
            .any(|plot| plot.interaction_effect_id.is_none())
    {
        return Err(vec![
            "The moderation canonical payload omits an interaction effect or plot binding".into(),
        ]);
    }
    let interaction_effects = CanonicalResultTable {
        id: GENERAL_SEM_INTERACTION_EFFECTS_TABLE_ID_V1.into(),
        title: "Interaction effects and product scaling".into(),
        description: Some(
            "Final joint stage-two interaction coefficients with their exact product-scaling receipts."
                .into(),
        ),
        columns: vec![
            text_column("effect_id", "Effect ID", "Stable canonical interaction-effect identity."),
            text_column("interaction_id", "Interaction", "Stable authored interaction identity."),
            text_column("focal_relation_id", "Focal path", "Stable focal structural relation identity."),
            text_column("interaction_effect_relation_id", "Effect relation", "Stable authored interaction-effect relation identity."),
            text_column("interaction_effect_parameter_id", "Effect parameter", "Stable authored interaction-effect parameter identity."),
            text_column("focal_predictor_id", "Focal predictor", "Focal predictor construct identity."),
            text_column("moderator_id", "Moderator", "Moderator construct identity."),
            text_column("outcome_id", "Outcome", "Outcome construct identity."),
            text_column("generated_product_column_id", "Product column", "Stable generated product-column identity."),
            text_column("stage_one_model_scientific_sha256", "Stage-one digest", "Scientific digest of the projected stage-one score model."),
            number_column("observation_count", "N", "Complete observations used by the joint stage-two solve."),
            number_column("standardized_product_coefficient", "Standardized product", "Coefficient fitted to the standardized product column."),
            number_column("scientific_rescaled_gamma", "Rescaled gamma", "Slope change per one standardized moderator unit."),
            number_column("product_mean", "Product mean", "Mean of the unstandardized product of standardized operands."),
            number_column("product_sample_sd", "Product sample SD", "Sample standard deviation used to standardize the product."),
            text_column("construction_method", "Construction", "Frozen interaction construction method."),
            text_column("product_scale_version", "Scale policy", "Exact product-scale policy version."),
            text_column("hierarchy_policy", "Hierarchy", "Frozen hierarchy policy."),
            text_column("hierarchy_policy_version", "Hierarchy version", "Exact hierarchy-policy version."),
            text_column("conditioning_policy_version", "Conditioning version", "Exact simple-slope conditioning policy version."),
            text_column("method_version", "Method", "Exact simultaneous moderation method version."),
        ],
        rows: results
            .interaction_effects
            .iter()
            .enumerate()
            .map(|(index, effect)| CanonicalResultRow {
                id: format!("interaction_effect_{index:04}"),
                cells: vec![
                    text(&effect.effect_id),
                    text(&effect.interaction_id),
                    text(&effect.focal_relation_id),
                    text(&effect.interaction_effect_relation_id),
                    text(&effect.interaction_effect_parameter_id),
                    text(&effect.focal_predictor_id),
                    text(&effect.moderator_id),
                    text(&effect.outcome_id),
                    text(&effect.generated_product_column_id),
                    text(&effect.stage_one_model_scientific_sha256),
                    number(f64::from(effect.observation_count)),
                    number(effect.standardized_product_coefficient.estimate),
                    number(effect.scientific_rescaled_gamma.estimate),
                    number(effect.unstandardized_product_mean),
                    number(effect.unstandardized_product_sample_standard_deviation),
                    text("two_stage"),
                    text(&effect.product_scale_version),
                    text("strong"),
                    text(&effect.hierarchy_policy_version),
                    text(&effect.conditioning_policy_version),
                    text(&effect.method_version),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let conditional_slopes = CanonicalResultTable {
        id: GENERAL_SEM_CONDITIONAL_SLOPES_TABLE_ID_V1.into(),
        title: "Conditional focal slopes".into(),
        description: Some(
            "Frozen minus-one, zero, and plus-one standardized moderator probes from the final joint stage-two coefficients."
                .into(),
        ),
        columns: vec![
            text_column("effect_id", "Effect ID", "Stable conditional-effect identity."),
            text_column("interaction_id", "Interaction", "Stable authored interaction identity."),
            text_column("interaction_effect_id", "Interaction effect", "Canonical interaction-effect identity that owns this slope."),
            text_column("focal_relation_id", "Focal path", "Stable focal structural relation identity."),
            text_column("probe_id", "Probe ID", "Stable frozen probe identity."),
            number_column("probe_value_index", "Probe index", "Zero-based index in the frozen probe policy."),
            text_column("moderator_id", "Moderator", "Moderator construct identity."),
            text_column("outcome_id", "Outcome", "Outcome construct identity."),
            number_column("moderator_value", "Moderator value", "Standardized moderator probe value."),
            number_column("estimate", "Conditional slope", "Conditional focal slope estimate."),
            text_column("conditioning_policy_version", "Conditioning version", "Exact simple-slope conditioning policy version."),
        ],
        rows: results
            .conditional_effects
            .iter()
            .enumerate()
            .map(|(index, effect)| {
                let interaction = interactions_by_id.get(effect.interaction_id.as_str()).ok_or_else(|| {
                    vec![format!(
                        "Conditional effect {} has no matching interaction-effect receipt",
                        effect.effect_id
                    )]
                })?;
                let interaction_effect_id = effect.interaction_effect_id.as_deref().ok_or_else(|| {
                    vec![format!(
                        "Conditional effect {} omits its interaction-effect identity",
                        effect.effect_id
                    )]
                })?;
                Ok(CanonicalResultRow {
                    id: format!("conditional_slope_{index:04}"),
                    cells: vec![
                        text(&effect.effect_id),
                        text(&effect.interaction_id),
                        text(interaction_effect_id),
                        text(&effect.focal_relation_id),
                        text(&effect.probe_id),
                        number(f64::from(effect.probe_value_index)),
                        text(&effect.moderator_id),
                        text(&interaction.outcome_id),
                        number(effect.moderator_value),
                        number(effect.value.estimate),
                        text(&interaction.conditioning_policy_version),
                    ],
                })
            })
            .collect::<Result<Vec<_>, Vec<String>>>()?,
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let interaction_plots = CanonicalResultTable {
        id: GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1.into(),
        title: "Interaction plot points".into(),
        description: Some(
            "Every point from the typed canonical interaction plots, materialized for viewer and tabular export parity."
                .into(),
        ),
        columns: vec![
            text_column("plot_id", "Plot ID", "Stable canonical plot identity."),
            text_column("interaction_id", "Interaction", "Stable authored interaction identity."),
            text_column("interaction_effect_id", "Interaction effect", "Canonical interaction-effect identity that owns this plot."),
            text_column("focal_relation_id", "Focal path", "Stable focal structural relation identity."),
            text_column("focal_predictor_id", "Focal predictor", "Focal predictor construct identity."),
            text_column("moderator_id", "Moderator", "Moderator construct identity."),
            text_column("outcome_id", "Outcome", "Outcome construct identity."),
            text_column("series_id", "Series ID", "Stable moderator-probe series identity."),
            text_column("probe_id", "Probe ID", "Stable frozen probe identity."),
            number_column("probe_value_index", "Probe index", "Zero-based index in the frozen probe policy."),
            number_column("moderator_value", "Moderator value", "Standardized moderator probe value."),
            number_column("focal_value", "Focal value", "Standardized focal predictor value."),
            number_column("predicted_value", "Predicted outcome", "Final joint stage-two standardized linear prediction."),
            number_column("lower", "Lower", "Optional lower interval bound."),
            number_column("upper", "Upper", "Optional upper interval bound."),
        ],
        rows: results
            .interaction_plots
            .iter()
            .flat_map(|plot| {
                plot.series.iter().flat_map(move |series| {
                    series.points.iter().map(move |point| (plot, series, point))
                })
            })
            .enumerate()
            .map(|(index, (plot, series, point))| CanonicalResultRow {
                id: format!("interaction_plot_point_{index:04}"),
                cells: vec![
                    text(&plot.plot_id),
                    text(&plot.interaction_id),
                    text(plot.interaction_effect_id.as_deref().unwrap_or_default()),
                    text(&plot.focal_relation_id),
                    text(&plot.focal_predictor_id),
                    text(&plot.moderator_id),
                    text(&plot.outcome_id),
                    text(&series.series_id),
                    text(&series.probe_id),
                    number(f64::from(series.probe_value_index)),
                    number(series.moderator_value),
                    number(point.focal_value),
                    number(point.predicted_value),
                    optional_number(point.lower),
                    optional_number(point.upper),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells,
    };
    Ok(vec![
        interaction_effects,
        conditional_slopes,
        interaction_plots,
    ])
}

fn moderation_bootstrap_tables(
    results: &CanonicalGeneralSemResultsV1,
    bootstrap: &qpls_resampling::GeneralSemPlsMultipleModerationBootstrapResultV1,
    execution: &RecipeV4GeneralSemPlsExecutionResultV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<Vec<CanonicalResultTable>, Vec<String>> {
    bootstrap.ensure_valid().map_err(|error| {
        vec![format!(
            "General SEM moderation bootstrap result is invalid: {error}"
        )]
    })?;
    let receipt = results.inference_receipt.as_ref().ok_or_else(|| {
        vec!["General SEM moderation bootstrap omitted its typed inference receipt".into()]
    })?;
    let expected_cell = general_sem_multiple_moderation_bootstrap_capability_cell_v1();
    let canonical_identity_digest = general_sem_effect_identity_set_sha256_v1(
        &canonical_general_sem_effect_identities_v1(results),
    );
    let failed_replicates_match = receipt.failed_replicates.len()
        == bootstrap.failed_replicates.len()
        && receipt
            .failed_replicates
            .iter()
            .zip(&bootstrap.failed_replicates)
            .all(|(canonical, raw)| {
                canonical.replicate_index == raw.replicate_index
                    && canonical.message == raw.message
                    && serde_json::to_value(canonical.reason_code).ok()
                        == serde_json::to_value(&raw.reason_code).ok()
            });
    if capability_cell != &expected_cell
        || receipt.capability_cell != expected_cell
        || receipt.method_version != bootstrap.method_version
        || receipt.resampling_operation_version != bootstrap.resampling_operation_version
        || receipt.resampling_stream_version != bootstrap.resampling_stream_version
        || receipt.quantile_method_version != bootstrap.quantile_method_version
        || receipt.standard_error_method_version != bootstrap.standard_error_method_version
        || receipt.summation_method_version != bootstrap.summation_method_version
        || receipt.p_value_method_version != bootstrap.p_value_method_version
        || receipt.failure_policy_version != bootstrap.failure_policy_version
        || receipt.compilation_artifact_identity_sha256
            != execution.compilation_artifact_identity_sha256()
        || receipt.compiled_plan_sha256 != bootstrap.compiled_plan_sha256
        || receipt.general_sem_config_sha256 != bootstrap.general_sem_config_sha256
        || receipt.recipe_analytical_sha256 != execution.recipe_analytical_sha256()
        || receipt.model_scientific_sha256 != bootstrap.model_scientific_sha256
        || receipt.source_dataset_fingerprint != bootstrap.source_dataset_fingerprint
        || receipt.complete_case_frame_sha256 != bootstrap.complete_case_frame_sha256
        || receipt.usable_replicate_indices_sha256 != bootstrap.usable_replicate_indices_sha256
        || receipt.effect_identity_set_sha256 != canonical_identity_digest
        || receipt.effect_ids != bootstrap.gamma_target_ids
        || !matches!(
            receipt.interval,
            qpls_core::CanonicalGeneralSemBootstrapIntervalV1::PercentileType7
        )
        || !matches!(
            receipt.tail,
            qpls_core::CanonicalGeneralSemInferenceTailV1::TwoSided
        )
        || receipt.confidence_level.to_bits() != bootstrap.confidence_level.to_bits()
        || receipt.resamples_requested != bootstrap.resamples_requested
        || receipt.resamples_usable != bootstrap.resamples_usable
        || receipt.minimum_usable_resamples != bootstrap.minimum_usable_resamples
        || receipt.seed != bootstrap.seed
        || receipt.workers != bootstrap.workers
        || receipt.complete_model_reestimated_per_replicate
            != bootstrap.complete_model_reestimated_per_replicate
        || !failed_replicates_match
        || execution.general_sem_config_sha256() != bootstrap.general_sem_config_sha256
        || execution.compiled_plan_sha256() != bootstrap.compiled_plan_sha256
        || execution.model_scientific_sha256() != bootstrap.model_scientific_sha256
        || execution.stage_one_model_scientific_sha256()
            != bootstrap.stage_one_model_scientific_sha256
        || execution.source_dataset_fingerprint() != bootstrap.source_dataset_fingerprint
    {
        return Err(vec![
            "General SEM moderation bootstrap raw/canonical receipt or execution provenance has drifted"
                .into(),
        ]);
    }

    let point_only_estimate = |estimate: &CanonicalGeneralSemEstimateV1| {
        estimate.bootstrap_mean.is_none()
            && estimate.bootstrap_bias.is_none()
            && estimate.standard_error.is_none()
            && estimate.lower.is_none()
            && estimate.upper.is_none()
            && estimate.p_value.is_none()
            && estimate.bootstrap_usable_replicates.is_none()
            && estimate.bootstrap_two_sided_exceedances.is_none()
    };
    if results
        .joint_stage_structural_coefficients
        .iter()
        .any(|coefficient| !point_only_estimate(&coefficient.estimate))
        || results
            .interaction_effects
            .iter()
            .any(|effect| !point_only_estimate(&effect.standardized_product_coefficient))
        || results
            .conditional_effects
            .iter()
            .any(|effect| !point_only_estimate(&effect.value))
        || results.interaction_plots.iter().any(|plot| {
            plot.series.iter().any(|series| {
                series
                    .points
                    .iter()
                    .any(|point| point.lower.is_some() || point.upper.is_some())
            })
        })
    {
        return Err(vec![
            "Moderation bootstrap beta, joint coefficients, slopes, and plots must remain point-only"
                .into(),
        ]);
    }

    if results.interaction_effects.len() != bootstrap.interaction_gammas.len() {
        return Err(vec![
            "Moderation bootstrap gamma inventory differs from the canonical interaction inventory"
                .into(),
        ]);
    }
    let mut gamma_rows = Vec::with_capacity(results.interaction_effects.len());
    for (index, (effect, gamma)) in results
        .interaction_effects
        .iter()
        .zip(&bootstrap.interaction_gammas)
        .enumerate()
    {
        let target = &gamma.target;
        let estimate = &effect.scientific_rescaled_gamma;
        if effect.effect_id != target.target_id
            || effect.interaction_id != target.interaction_id
            || effect.focal_relation_id != target.focal_relation_id
            || effect.interaction_effect_relation_id != target.interaction_effect_relation_id
            || effect.interaction_effect_parameter_id != target.interaction_effect_parameter_id
            || effect.generated_product_column_id != target.generated_product_column_id
            || effect.focal_predictor_id != target.focal_predictor_id
            || effect.moderator_id != target.moderator_id
            || effect.outcome_id != target.outcome_id
            || effect.stage_one_model_scientific_sha256 != target.stage_one_model_scientific_sha256
            || effect.product_scale_version != target.product_scale_version
            || effect.method_version != target.method_version
            || estimate.estimate.to_bits() != gamma.original.to_bits()
            || estimate.bootstrap_mean.map(f64::to_bits) != Some(gamma.bootstrap_mean.to_bits())
            || estimate.bootstrap_bias.map(f64::to_bits) != Some(gamma.bootstrap_bias.to_bits())
            || estimate.standard_error.map(f64::to_bits) != Some(gamma.standard_error.to_bits())
            || estimate.lower.map(f64::to_bits) != Some(gamma.lower.to_bits())
            || estimate.upper.map(f64::to_bits) != Some(gamma.upper.to_bits())
            || estimate.p_value.map(f64::to_bits) != Some(gamma.p_value_two_sided.to_bits())
            || estimate.bootstrap_usable_replicates != Some(gamma.usable_replicates)
            || estimate.bootstrap_two_sided_exceedances != Some(gamma.two_sided_exceedances)
        {
            return Err(vec![format!(
                "Moderation bootstrap gamma {} differs from its complete raw/canonical target identity or inference row",
                target.target_id
            )]);
        }
        gamma_rows.push(CanonicalResultRow {
            id: format!("moderation_gamma_inference_{index:04}"),
            cells: vec![
                text(&effect.effect_id),
                text(&effect.interaction_id),
                text(&effect.focal_relation_id),
                text(&effect.interaction_effect_relation_id),
                text(&effect.interaction_effect_parameter_id),
                text(&effect.generated_product_column_id),
                text(&effect.focal_predictor_id),
                text(&effect.moderator_id),
                text(&effect.outcome_id),
                text(&effect.stage_one_model_scientific_sha256),
                text(&effect.product_scale_version),
                text(&effect.method_version),
                number(estimate.estimate),
                number(gamma.bootstrap_mean),
                number(gamma.bootstrap_bias),
                number(gamma.standard_error),
                number(gamma.lower),
                number(gamma.upper),
                number(gamma.p_value_two_sided),
                number(f64::from(gamma.usable_replicates)),
                number(f64::from(gamma.two_sided_exceedances)),
            ],
        });
    }

    let gamma_table = CanonicalResultTable {
        id: GENERAL_SEM_MODERATION_GAMMA_INFERENCE_TABLE_ID_V1.into(),
        title: "Scientific gamma bootstrap inference".into(),
        description: Some(
            "Percentile full-pipeline case-bootstrap inference for every compiled scientific rescaled interaction gamma."
                .into(),
        ),
        columns: vec![
            text_column("effect_id", "Effect ID", "Canonical scientific gamma identity."),
            text_column("interaction_id", "Interaction", "Authored interaction identity."),
            text_column("focal_relation_id", "Focal path", "Focal relation identity."),
            text_column("interaction_effect_relation_id", "Effect relation", "Interaction-effect relation identity."),
            text_column("interaction_effect_parameter_id", "Effect parameter", "Interaction-effect parameter identity."),
            text_column("generated_product_column_id", "Product column", "Generated standardized product-column identity."),
            text_column("focal_predictor_id", "Focal predictor", "Focal predictor construct identity."),
            text_column("moderator_id", "Moderator", "Moderator construct identity."),
            text_column("outcome_id", "Outcome", "Outcome construct identity."),
            text_column("stage_one_model_scientific_sha256", "Stage-one digest", "Scientific digest of the shared interaction-free score model."),
            text_column("product_scale_version", "Product scale", "Exact product scaling policy."),
            text_column("point_method_version", "Point method", "Exact simultaneous moderation point method."),
            number_column("estimate", "Estimate", "Scientific rescaled gamma point estimate."),
            number_column("bootstrap_mean", "Bootstrap mean", "Mean across usable full-pipeline bootstrap replicates."),
            number_column("bootstrap_bias", "Bias", "Bootstrap mean minus point gamma."),
            number_column("standard_error", "Standard error", "Sample standard error across usable gamma replicates."),
            number_column("lower", "Lower", "Lower type-7 percentile bound."),
            number_column("upper", "Upper", "Upper type-7 percentile bound."),
            number_column("p_value", "P value", "Two-sided null-centered plus-one probability."),
            number_column("bootstrap_usable_replicates", "Usable", "Usable full-pipeline replicate count."),
            number_column("bootstrap_two_sided_exceedances", "Exceedances", "Two-sided null exceedance count."),
        ],
        rows: gamma_rows,
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    };

    let receipt_table = CanonicalResultTable {
        id: GENERAL_SEM_MODERATION_BOOTSTRAP_RECEIPT_TABLE_ID_V1.into(),
        title: "Moderation bootstrap pipeline receipt".into(),
        description: Some(
            "Exact supplemental cell, algorithm, provenance, full-pipeline, and failed-replicate receipt."
                .into(),
        ),
        columns: vec![
            text_column("capability_id", "Capability", "Supplemental capability identity."),
            text_column("cell_id", "Cell", "Supplemental cell identity."),
            text_column("capability_version", "Capability version", "Supplemental capability version."),
            text_column("method_version", "Method", "Outer moderation-bootstrap method."),
            text_column("point_method_version", "Point method", "Point estimator method."),
            text_column("resampling_operation_version", "Operation", "Case-resampling operation."),
            text_column("resampling_stream_version", "Stream", "Indexed resampling stream."),
            text_column("quantile_method_version", "Quantile", "Interval quantile method."),
            text_column("standard_error_method_version", "Standard error", "Standard-error method."),
            text_column("summation_method_version", "Summation", "Stable summation method."),
            text_column("p_value_method_version", "P value", "Bootstrap p-value method."),
            text_column("failure_policy_version", "Failure policy", "Minimum usable policy."),
            text_column("sign_alignment_method_version", "Sign alignment", "Construct-score sign alignment."),
            text_column("product_scale_version", "Product scale", "Per-replicate product scaling."),
            text_column("gamma_target_version", "Gamma target", "Scientific gamma target schema."),
            text_column("compilation_artifact_identity_sha256", "Artifact digest", "Compiled artifact identity digest."),
            text_column("compiled_plan_sha256", "Plan digest", "Compiled plan digest."),
            text_column("general_sem_config_sha256", "Config digest", "General SEM config digest."),
            text_column("recipe_analytical_sha256", "Recipe digest", "Analytical recipe digest."),
            text_column("model_scientific_sha256", "Model digest", "Full model scientific digest."),
            text_column("stage_one_model_scientific_sha256", "Stage-one digest", "Stage-one projection digest."),
            text_column("source_dataset_fingerprint", "Dataset", "Source dataset fingerprint."),
            text_column("complete_case_frame_sha256", "Complete-case digest", "Complete-case frame digest."),
            text_column("usable_replicate_indices_sha256", "Usable-index digest", "Usable replicate ledger digest."),
            text_column("gamma_target_identity_set_sha256", "Gamma identity digest", "Canonical gamma identity-set digest."),
            text_column("interval", "Interval", "Canonical interval family."),
            text_column("tail", "Tail", "Canonical hypothesis tail."),
            number_column("confidence_level", "Confidence", "Confidence level."),
            number_column("resamples_requested", "Requested", "Requested replicates."),
            number_column("resamples_usable", "Usable", "Usable replicates."),
            number_column("minimum_usable_resamples", "Minimum usable", "Required usable replicates."),
            text_column("seed", "Seed", "Exact decimal resampling seed."),
            number_column("workers", "Workers", "Configured worker count."),
            boolean_column("complete_model_reestimated_per_replicate", "Complete model", "Complete model re-estimated per replicate."),
            boolean_column("shared_stage_one_reestimated_per_replicate", "Stage one", "Shared stage one re-estimated per replicate."),
            boolean_column("score_vectors_sign_aligned_before_products", "Sign aligned", "Score vectors sign aligned before products."),
            boolean_column("product_scaling_recomputed_per_replicate", "Product scaling", "Product scaling recomputed per replicate."),
            boolean_column("joint_stage_two_reestimated_per_replicate", "Joint stage two", "Joint stage two re-estimated per replicate."),
            boolean_column("complete_joint_point_contract_validated_per_replicate", "Point contract", "Complete joint point contract validated per replicate."),
            number_column("failed_replicate_count", "Failed", "Typed failed replicate count."),
        ],
        rows: vec![CanonicalResultRow {
            id: "moderation_bootstrap_receipt".into(),
            cells: vec![
                text(&receipt.capability_cell.capability_id),
                text(&receipt.capability_cell.cell_id),
                text(&receipt.capability_cell.capability_version),
                text(&receipt.method_version),
                text(&bootstrap.point_method_version),
                text(&receipt.resampling_operation_version),
                text(&receipt.resampling_stream_version),
                text(&receipt.quantile_method_version),
                text(&receipt.standard_error_method_version),
                text(&receipt.summation_method_version),
                text(&receipt.p_value_method_version),
                text(&receipt.failure_policy_version),
                text(&bootstrap.sign_alignment_method_version),
                text(&bootstrap.product_scale_version),
                text(&bootstrap.gamma_target_version),
                text(&receipt.compilation_artifact_identity_sha256),
                text(&receipt.compiled_plan_sha256),
                text(&receipt.general_sem_config_sha256),
                text(&receipt.recipe_analytical_sha256),
                text(&receipt.model_scientific_sha256),
                text(&bootstrap.stage_one_model_scientific_sha256),
                text(&receipt.source_dataset_fingerprint),
                text(&receipt.complete_case_frame_sha256),
                text(&receipt.usable_replicate_indices_sha256),
                text(&receipt.effect_identity_set_sha256),
                text("percentile_type7"),
                text("two_sided"),
                number(receipt.confidence_level),
                number(f64::from(receipt.resamples_requested)),
                number(f64::from(receipt.resamples_usable)),
                number(f64::from(receipt.minimum_usable_resamples)),
                text(&receipt.seed),
                number(f64::from(receipt.workers)),
                boolean(bootstrap.complete_model_reestimated_per_replicate),
                boolean(bootstrap.shared_stage_one_reestimated_per_replicate),
                boolean(bootstrap.score_vectors_sign_aligned_before_products),
                boolean(bootstrap.product_scaling_recomputed_per_replicate),
                boolean(bootstrap.joint_stage_two_reestimated_per_replicate),
                boolean(bootstrap.complete_joint_point_contract_validated_per_replicate),
                number(bootstrap.failed_replicates.len() as f64),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    };
    Ok(vec![gamma_table, receipt_table])
}

fn moderation_charts(results: &CanonicalGeneralSemResultsV1) -> Vec<CanonicalResultChart> {
    results
        .interaction_plots
        .iter()
        .enumerate()
        .map(|(plot_index, plot)| CanonicalResultChart {
            id: format!("general_sem_interaction_chart_{plot_index:04}"),
            title: format!("Interaction {}", plot.interaction_id),
            description: format!(
                "Final joint stage-two predicted {} across standardized {} values at frozen standardized {} probes.",
                plot.outcome_id, plot.focal_predictor_id, plot.moderator_id
            ),
            kind: CanonicalChartKind::Line,
            series: plot
                .series
                .iter()
                .map(|series| CanonicalChartSeries {
                    id: series.series_id.clone(),
                    label: format!("{} = {:.4}", plot.moderator_id, series.moderator_value),
                    group: Some(plot.interaction_id.clone()),
                    points: series
                        .points
                        .iter()
                        .map(|point| CanonicalChartPoint {
                            x: CanonicalChartX::Number(point.focal_value),
                            y: point.predicted_value,
                            lower: point.lower,
                            upper: point.upper,
                            label: None,
                        })
                        .collect(),
                })
                .collect(),
            source_table_id: Some(GENERAL_SEM_INTERACTION_PLOTS_TABLE_ID_V1.into()),
            display: CanonicalChartDisplayOptions {
                palette: None,
                show_legend: Some(true),
                show_values: Some(false),
                x_axis_label: Some(format!("{} (standardized)", plot.focal_predictor_id)),
                y_axis_label: Some(format!("{} (predicted standardized)", plot.outcome_id)),
            },
        })
        .collect()
}

fn general_sem_tables(
    results: &CanonicalGeneralSemResultsV1,
    capability_cells: &[CapabilityCellReferenceV2],
) -> Vec<CanonicalResultTable> {
    let mut tables = Vec::new();
    if !results.specific_indirect_effects.is_empty() {
        tables.push(CanonicalResultTable {
            id: "general_sem_specific_indirect_effects".into(),
            title: "Specific indirect effects".into(),
            description: Some(
                "Requested ordered mediation paths with stable relation identities and optional full-model bootstrap inference."
                    .into(),
            ),
            columns: effect_columns(true),
            rows: results
                .specific_indirect_effects
                .iter()
                .enumerate()
                .map(|(index, effect)| CanonicalResultRow {
                    id: format!("specific_indirect_{index:04}"),
                    cells: effect_cells(
                        &effect.effect_id,
                        &effect.estimand_id,
                        "specific_indirect",
                        &effect.source_id,
                        &effect.target_id,
                        &effect.ordered_relation_ids.join(" -> "),
                        &effect.value,
                    ),
                })
                .collect(),
            footnote_ids: Vec::new(),
            capability_cells: Some(capability_cells.to_vec()),
        });
    }
    if !results.aggregate_effects.is_empty() {
        tables.push(CanonicalResultTable {
            id: "general_sem_aggregate_effects".into(),
            title: "Aggregate effects".into(),
            description: Some(
                "Requested total-indirect and total effects with their contributing stable path identities."
                    .into(),
            ),
            columns: effect_columns(true),
            rows: results
                .aggregate_effects
                .iter()
                .enumerate()
                .map(|(index, effect)| {
                    let kind = match effect.kind {
                        CanonicalAggregateEffectKindV1::TotalIndirect => "total_indirect",
                        CanonicalAggregateEffectKindV1::TotalEffect => "total_effect",
                    };
                    CanonicalResultRow {
                        id: format!("aggregate_effect_{index:04}"),
                        cells: effect_cells(
                            &effect.effect_id,
                            &effect.estimand_id,
                            kind,
                            &effect.source_id,
                            &effect.target_id,
                            &effect.contributing_path_identities.join("; "),
                            &effect.value,
                        ),
                    }
                })
                .collect(),
            footnote_ids: Vec::new(),
            capability_cells: Some(capability_cells.to_vec()),
        });
    }
    if let Some(receipt) = &results.inference_receipt {
        tables.push(CanonicalResultTable {
            id: "general_sem_bootstrap_receipt".into(),
            title: "Full-model bootstrap receipt".into(),
            description: Some(
                "Exact case-resampling, usable-replicate, worker, and failure-ledger identity for inferred mediation effects."
                    .into(),
            ),
            columns: vec![
                text_column("method_version", "Method", "Frozen General SEM bootstrap method version."),
                number_column("resamples_requested", "Requested", "Number of requested case-bootstrap replicates."),
                number_column("resamples_usable", "Usable", "Number of usable complete-model refits."),
                number_column("minimum_usable", "Minimum usable", "Minimum usable-replicate acceptance threshold."),
                text_column("seed", "Seed", "Exact decimal unsigned resampling seed."),
                number_column("workers", "Workers", "Configured resampling worker count."),
                boolean_column("complete_model_refit", "Complete refit", "Whether every usable replicate re-estimated the complete model."),
                number_column("failed_replicates", "Failed", "Number of typed failed replicates."),
            ],
            rows: vec![CanonicalResultRow {
                id: "bootstrap_receipt".into(),
                cells: vec![
                    text(&receipt.method_version),
                    number(f64::from(receipt.resamples_requested)),
                    number(f64::from(receipt.resamples_usable)),
                    number(f64::from(receipt.minimum_usable_resamples)),
                    text(&receipt.seed),
                    number(f64::from(receipt.workers)),
                    boolean(receipt.complete_model_reestimated_per_replicate),
                    number(receipt.failed_replicates.len() as f64),
                ],
            }],
            footnote_ids: Vec::new(),
            capability_cells: Some(capability_cells.to_vec()),
        });
    }
    tables
}

fn effect_columns(include_identity_paths: bool) -> Vec<CanonicalResultColumn> {
    let mut columns = vec![
        text_column(
            "effect_id",
            "Effect ID",
            "Stable canonical effect identity.",
        ),
        text_column(
            "estimand_id",
            "Estimand ID",
            "Stable authored estimand request identity.",
        ),
        text_column(
            "kind",
            "Kind",
            "Specific-indirect, total-indirect, or total-effect family.",
        ),
        text_column("source", "Source", "Source construct identity."),
        text_column("target", "Target", "Target construct identity."),
    ];
    if include_identity_paths {
        columns.push(text_column(
            "path_identities",
            "Paths",
            "Ordered relation path or contributing canonical path identities.",
        ));
    }
    columns.extend([
        number_column("estimate", "Estimate", "PLS point estimate."),
        number_column(
            "bootstrap_mean",
            "Bootstrap mean",
            "Mean across usable full-model bootstrap replicates.",
        ),
        number_column(
            "bootstrap_bias",
            "Bias",
            "Bootstrap mean minus the point estimate.",
        ),
        number_column(
            "standard_error",
            "Standard error",
            "Bootstrap standard error.",
        ),
        number_column("lower", "Lower", "Lower percentile confidence bound."),
        number_column("upper", "Upper", "Upper percentile confidence bound."),
        number_column(
            "p_value",
            "P value",
            "Two-sided plus-one bootstrap probability.",
        ),
        number_column(
            "usable_replicates",
            "Usable replicates",
            "Usable full-model bootstrap replicate count.",
        ),
    ]);
    columns
}

fn effect_cells(
    effect_id: &str,
    estimand_id: &str,
    kind: &str,
    source: &str,
    target: &str,
    paths: &str,
    estimate: &CanonicalGeneralSemEstimateV1,
) -> Vec<CanonicalResultCell> {
    vec![
        text(effect_id),
        text(estimand_id),
        text(kind),
        text(source),
        text(target),
        text(paths),
        number(estimate.estimate),
        optional_number(estimate.bootstrap_mean),
        optional_number(estimate.bootstrap_bias),
        optional_number(estimate.standard_error),
        optional_number(estimate.lower),
        optional_number(estimate.upper),
        optional_number(estimate.p_value),
        optional_number(estimate.bootstrap_usable_replicates.map(f64::from)),
    ]
}

fn text_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        description: description.into(),
        data_type: CanonicalColumnType::Text,
        role: Some(CanonicalColumnRole::Label),
        unit: None,
        default_precision: None,
    }
}

fn number_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        description: description.into(),
        data_type: CanonicalColumnType::Number,
        role: Some(CanonicalColumnRole::Estimate),
        unit: None,
        default_precision: Some(4),
    }
}

fn boolean_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        description: description.into(),
        data_type: CanonicalColumnType::Boolean,
        role: Some(CanonicalColumnRole::Diagnostic),
        unit: None,
        default_precision: None,
    }
}

fn text(value: impl Into<String>) -> CanonicalResultCell {
    CanonicalResultCell::Text {
        value: value.into(),
    }
}

fn number(value: f64) -> CanonicalResultCell {
    CanonicalResultCell::Number {
        value,
        display: None,
    }
}

fn boolean(value: bool) -> CanonicalResultCell {
    CanonicalResultCell::Boolean { value }
}

fn optional_number(value: Option<f64>) -> CanonicalResultCell {
    value.map_or(
        CanonicalResultCell::Missing {
            reason: CanonicalMissingReason::NotEstimated,
            display: None,
        },
        number,
    )
}

fn sort_and_deduplicate_cells(cells: &mut Vec<CapabilityCellReferenceV2>) {
    cells.sort_by(|left, right| {
        (
            left.capability_id.as_str(),
            left.cell_id.as_str(),
            left.capability_version.as_str(),
        )
            .cmp(&(
                right.capability_id.as_str(),
                right.cell_id.as_str(),
                right.capability_version.as_str(),
            ))
    });
    cells.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_archive_v6_general_sem_bootstrap::tests::general_sem_native_fixture_v1;
    use qpls_core::{
        GeneralSemBootstrapIntervalV1, GeneralSemConfigV1, GeneralSemInferenceTailV1,
        GeneralSemInferenceV1, PlsBootstrapTestTail, compile_general_sem_pls_recipe_v1,
    };
    use qpls_runner::run_compiled_general_sem_pls_recipe_v1;

    fn canonical_document(bootstrap: bool) -> CanonicalResultDocumentV2 {
        let fixture = general_sem_native_fixture_v1();
        let mut recipe = fixture.recipe;
        if bootstrap {
            recipe.settings.bootstrap_samples = 20;
            recipe.settings.seed = qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1;
            recipe.settings.confidence_level = 0.95;
            recipe.settings.bootstrap_test_tail = PlsBootstrapTestTail::TwoSided;
            recipe.settings.studentized_inner_samples = 0;
            recipe.settings.workers = 1;
            recipe.general_sem_config = Some(GeneralSemConfigV1 {
                inference: GeneralSemInferenceV1::CaseBootstrap {
                    resamples: 20,
                    seed: qpls_core::GENERAL_SEM_CASE_BOOTSTRAP_MAX_SEED_V1,
                    confidence_level: 0.95,
                    interval: GeneralSemBootstrapIntervalV1::Percentile,
                    tail: GeneralSemInferenceTailV1::TwoSided,
                },
                ..GeneralSemConfigV1::default()
            });
        }
        let artifact = compile_general_sem_pls_recipe_v1(&recipe, Some(&fixture.model)).unwrap();
        let analytical = run_compiled_general_sem_pls_recipe_v1(
            &fixture.dataset,
            &recipe,
            &fixture.model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        build_recipe_v4_general_sem_pls_canonical_result_v1(
            Uuid::from_u128(if bootstrap { 0x7402 } else { 0x7401 }),
            Uuid::from_u128(0x7400),
            fixture.dataset.id,
            "2026-08-18T09:30:00.000Z",
            "2026-08-18T09:30:01.000Z",
            &recipe,
            &fixture.model,
            &analytical,
        )
        .unwrap()
    }

    #[test]
    fn point_and_bootstrap_documents_retain_exact_capability_cell_identities() {
        let point_cell = pls_general_recursive_effects_capability_cell_v1();
        let bootstrap_cell = general_sem_multiple_mediation_bootstrap_capability_cell_v1();

        let point = canonical_document(false);
        assert_eq!(point.provenance.capability_cell, point_cell);
        assert_eq!(
            point.provenance.method_version,
            GENERAL_SEM_EFFECTS_V1_METHOD_VERSION
        );
        let point_cells = point.capability_cells.as_ref().unwrap();
        assert!(point_cells.contains(&point_cell));
        assert!(!point_cells.contains(&bootstrap_cell));
        let point_results = point.general_sem_results.as_ref().unwrap();
        assert!(point_results.inference_receipt.is_none());
        assert!(
            point_results
                .specific_indirect_effects
                .iter()
                .all(|effect| effect.trace.capability_cell == point_cell)
        );

        let bootstrap = canonical_document(true);
        assert_eq!(bootstrap.provenance.capability_cell, point_cell);
        assert_eq!(
            bootstrap.provenance.method_version,
            GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1
        );
        let bootstrap_cells = bootstrap.capability_cells.as_ref().unwrap();
        assert!(bootstrap_cells.contains(&point_cell));
        assert!(bootstrap_cells.contains(&bootstrap_cell));
        let bootstrap_results = bootstrap.general_sem_results.as_ref().unwrap();
        let receipt = bootstrap_results.inference_receipt.as_ref().unwrap();
        assert_eq!(receipt.capability_cell, bootstrap_cell);
        assert!(
            bootstrap_results
                .specific_indirect_effects
                .iter()
                .all(|effect| effect.trace.capability_cell == point_cell)
        );
        let receipt_table = bootstrap
            .tables
            .iter()
            .find(|table| table.id == "general_sem_bootstrap_receipt")
            .unwrap();
        assert!(
            receipt_table
                .capability_cells
                .as_ref()
                .is_some_and(|cells| cells.contains(&bootstrap_cell))
        );
    }
}
