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
    GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1,
    RecipeV4CompilerTarget, SemModelV4, SemParameterV4, StructuralRelationRoleV4,
    canonical_general_sem_effect_identities_v1, compile_general_sem_pls_recipe_v1,
    general_sem_effect_identity_set_sha256_v1, pls_general_bootstrap_capability_cell_v1,
    pls_general_higher_order_bootstrap_capability_cell_v1,
    pls_general_higher_order_point_capability_cell_v1,
    pls_general_multiple_moderation_bootstrap_capability_cell_v1,
    pls_general_multiple_moderation_point_capability_cell_v1,
    pls_general_recursive_effects_capability_cell_v1,
    pls_general_single_mediation_bootstrap_capability_cell_v1,
    pls_general_three_way_moderation_bootstrap_capability_cell_v1,
    pls_general_three_way_moderation_point_capability_cell_v1,
    pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1,
    project_general_sem_pls_stage_one_recipe_v1, validate_canonical_result_document_v2,
};
use qpls_runner::{
    GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1,
    GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_POINT_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
    RECIPE_V4_GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
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
const GENERAL_SEM_MODERATED_MEDIATION_BOOTSTRAP_SECTION_ID_V1: &str =
    "general_sem_moderated_mediation_bootstrap";
const GENERAL_SEM_MODERATED_MEDIATION_GAMMA_TABLE_ID_V1: &str =
    "general_sem_moderated_mediation_gamma_inference";
const GENERAL_SEM_CONDITIONAL_INDIRECT_TABLE_ID_V1: &str =
    "general_sem_conditional_indirect_effects";
const GENERAL_SEM_MODERATED_MEDIATION_INDEX_TABLE_ID_V1: &str =
    "general_sem_moderated_mediation_indices";
const GENERAL_SEM_MODERATED_MEDIATION_RECEIPT_TABLE_ID_V1: &str =
    "general_sem_moderated_mediation_bootstrap_receipt";
const GENERAL_SEM_HIGHER_ORDER_SECTION_ID_V1: &str = "general_sem_higher_order";
const GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_SECTION_ID_V1: &str = "general_sem_higher_order_bootstrap";
const GENERAL_SEM_HIGHER_ORDER_STAGES_TABLE_ID_V1: &str = "general_sem_higher_order_stages";
const GENERAL_SEM_HIGHER_ORDER_TARGETS_TABLE_ID_V1: &str = "general_sem_higher_order_targets";
const GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_RECEIPT_TABLE_ID_V1: &str =
    "general_sem_higher_order_bootstrap_receipt";
const GENERAL_SEM_THREE_WAY_SECTION_ID_V1: &str = "general_sem_three_way_moderation";
const GENERAL_SEM_THREE_WAY_EFFECT_TABLE_ID_V1: &str = "general_sem_three_way_effect";
const GENERAL_SEM_THREE_WAY_CONDITIONAL_TABLE_ID_V1: &str =
    "general_sem_three_way_conditional_effects";
const GENERAL_SEM_THREE_WAY_SIMPLE_SLOPES_TABLE_ID_V1: &str = "general_sem_three_way_simple_slopes";
const GENERAL_SEM_THREE_WAY_RECEIPT_TABLE_ID_V1: &str = "general_sem_three_way_bootstrap_receipt";

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

fn validate_archived_moderated_mediation_method_identity_v1(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let point_cell = general_sem_multiple_moderation_point_capability_cell_v1();
    let supplemental_cell = pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    let project_cell_matches =
        |candidate: &qpls_project::CapabilityCellReferenceV2,
         expected: &CapabilityCellReferenceV2| {
            candidate.registry_schema_version == expected.registry_schema_version
                && candidate.capability_id == expected.capability_id
                && candidate.cell_id == expected.cell_id
                && candidate.capability_version == expected.capability_version
        };
    if !project_cell_matches(&document.provenance.capability_cell, &point_cell)
        || document.provenance.method_version
            != GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
        || document.provenance.engine_version
            != RECIPE_V4_GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        || document.title != "General SEM two-way PLS moderated-mediation bootstrap inference"
    {
        return Err(
            "archived moderated-mediation document has a stale primary cell, method, adapter, or title"
                .into(),
        );
    }
    let results = document.general_sem_results.as_ref().ok_or_else(|| {
        "archived moderated-mediation document omits its typed scientific payload".to_string()
    })?;
    let receipt = results.inference_receipt.as_ref().ok_or_else(|| {
        "archived moderated-mediation document omits its combined bootstrap receipt".to_string()
    })?;
    if receipt.capability_cell != supplemental_cell
        || receipt.method_version
            != GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
        || receipt.capability_dependencies.len() != 2
        || results.interaction_effects.len() != 1
        || results.conditional_indirect_effects.len() != 3
        || results.moderated_mediation_indices.len() != 1
        || results.joint_stage_structural_coefficients.is_empty()
        || !results.specific_indirect_effects.is_empty()
        || !results.aggregate_effects.is_empty()
    {
        return Err(
            "archived moderated-mediation typed payload or exact three-cell receipt is incomplete"
                .into(),
        );
    }
    let expected_section_tables = [
        GENERAL_SEM_MODERATED_MEDIATION_GAMMA_TABLE_ID_V1,
        GENERAL_SEM_CONDITIONAL_INDIRECT_TABLE_ID_V1,
        GENERAL_SEM_MODERATED_MEDIATION_INDEX_TABLE_ID_V1,
        GENERAL_SEM_MODERATED_MEDIATION_RECEIPT_TABLE_ID_V1,
    ];
    let section = document
        .sections
        .iter()
        .find(|section| section.id == GENERAL_SEM_MODERATED_MEDIATION_BOOTSTRAP_SECTION_ID_V1)
        .ok_or_else(|| {
            "archived moderated-mediation document omits its result section".to_string()
        })?;
    let table_suffix = document
        .tables
        .iter()
        .rev()
        .take(expected_section_tables.len())
        .map(|table| table.id.as_str())
        .collect::<Vec<_>>();
    if document.sections.last().map(|section| section.id.as_str())
        != Some(GENERAL_SEM_MODERATED_MEDIATION_BOOTSTRAP_SECTION_ID_V1)
        || section
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_section_tables)
        || !section.chart_ids.is_empty()
        || section.capability_cells.as_deref().is_none_or(
            |cells| !matches!(cells, [cell] if project_cell_matches(cell, &supplemental_cell)),
        )
        || table_suffix
            != expected_section_tables
                .iter()
                .rev()
                .copied()
                .collect::<Vec<_>>()
        || document.presentation.default_section_id.as_deref()
            != Some(GENERAL_SEM_MODERATED_MEDIATION_BOOTSTRAP_SECTION_ID_V1)
        || document.presentation.default_table_id.as_deref()
            != Some(GENERAL_SEM_CONDITIONAL_INDIRECT_TABLE_ID_V1)
        || expected_section_tables.iter().any(|table_id| {
            let tables = document
                .tables
                .iter()
                .filter(|table| table.id == *table_id)
                .collect::<Vec<_>>();
            !matches!(tables.as_slice(), [table] if table.capability_cells.as_deref().is_some_and(
                |cells| matches!(cells, [cell] if project_cell_matches(cell, &supplemental_cell))
            ))
        })
    {
        return Err(
            "archived moderated-mediation section, table order, ownership, or presentation binding is invalid"
                .into(),
        );
    }
    document.ensure_valid().map_err(|error| error.to_string())
}

fn validate_archived_three_way_method_identity_v1(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let point_cell = pls_general_three_way_moderation_point_capability_cell_v1();
    let bootstrap_cell = pls_general_three_way_moderation_bootstrap_capability_cell_v1();
    let same_cell = |candidate: &qpls_project::CapabilityCellReferenceV2,
                     expected: &CapabilityCellReferenceV2| {
        candidate.registry_schema_version == expected.registry_schema_version
            && candidate.capability_id == expected.capability_id
            && candidate.cell_id == expected.cell_id
            && candidate.capability_version == expected.capability_version
    };
    let results = document.general_sem_results.as_ref().ok_or_else(|| {
        "archived three-way document omits its typed scientific payload".to_string()
    })?;
    let inferred = results.three_way_moderation_bootstrap_receipt.is_some();
    let (method, adapter, title) = if inferred {
        (
            GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1,
            RECIPE_V4_GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            "General SEM three-way PLS moderation bootstrap inference",
        )
    } else {
        (
            GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1,
            RECIPE_V4_GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1,
            "General SEM three-way PLS moderation point estimates",
        )
    };
    if !same_cell(&document.provenance.capability_cell, &point_cell)
        || document.provenance.method_version != method
        || document.provenance.engine_version != adapter
        || document.title != title
        || results.three_way_interaction_effects.len() != 1
        || results.three_way_conditional_interaction_effects.is_empty()
        || results.three_way_simple_slopes.is_empty()
        || document
            .sections
            .iter()
            .filter(|section| section.id == GENERAL_SEM_THREE_WAY_SECTION_ID_V1)
            .count()
            != 1
        || [
            GENERAL_SEM_THREE_WAY_EFFECT_TABLE_ID_V1,
            GENERAL_SEM_THREE_WAY_CONDITIONAL_TABLE_ID_V1,
            GENERAL_SEM_THREE_WAY_SIMPLE_SLOPES_TABLE_ID_V1,
        ]
        .iter()
        .any(|id| {
            document
                .tables
                .iter()
                .filter(|table| table.id == **id)
                .count()
                != 1
        })
    {
        return Err(
            "archived three-way method, typed inventory, or result projection has drifted".into(),
        );
    }
    if inferred
        && results
            .three_way_moderation_bootstrap_receipt
            .as_ref()
            .is_none_or(|receipt| receipt.capability_cell != bootstrap_cell)
    {
        return Err("archived three-way bootstrap receipt has a stale supplemental cell".into());
    }
    document.ensure_valid().map_err(|error| error.to_string())
}

fn validate_archived_single_mediation_method_identity_v1(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let point_cell = pls_general_recursive_effects_capability_cell_v1();
    let bootstrap_cell = pls_general_single_mediation_bootstrap_capability_cell_v1();
    let same_cell = |candidate: &qpls_project::CapabilityCellReferenceV2,
                     expected: &CapabilityCellReferenceV2| {
        candidate.registry_schema_version == expected.registry_schema_version
            && candidate.capability_id == expected.capability_id
            && candidate.cell_id == expected.cell_id
            && candidate.capability_version == expected.capability_version
    };
    let results = document.general_sem_results.as_ref().ok_or_else(|| {
        "archived single-mediation document omits its typed scientific payload".to_string()
    })?;
    let receipt = results.inference_receipt.as_ref().ok_or_else(|| {
        "archived single-mediation bootstrap document omits its inference receipt".to_string()
    })?;
    if !same_cell(&document.provenance.capability_cell, &point_cell)
        || document.provenance.method_version
            != GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || document.provenance.engine_version
            != RECIPE_V4_GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        || document.title != "PLS-SEM single mediation with full-model bootstrap"
        || receipt.capability_cell != bootstrap_cell
        || receipt.method_version
            != GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || results.specific_indirect_effects.len() != 1
        || !document
            .capability_cells
            .as_ref()
            .is_some_and(|cells| cells.iter().any(|cell| same_cell(cell, &bootstrap_cell)))
    {
        return Err(
            "archived single-mediation bootstrap identity or typed inventory has drifted".into(),
        );
    }
    document.ensure_valid().map_err(|error| error.to_string())
}

pub(crate) fn validate_archived_general_sem_pls_method_identity_v1(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let three_way_point_cell = pls_general_three_way_moderation_point_capability_cell_v1();
    let has_three_way_artifact = document.provenance.capability_cell.cell_id
        == three_way_point_cell.cell_id
        || document
            .general_sem_results
            .as_ref()
            .is_some_and(|results| {
                !results.three_way_interaction_effects.is_empty()
                    || !results.three_way_conditional_interaction_effects.is_empty()
                    || !results.three_way_simple_slopes.is_empty()
                    || results.three_way_moderation_bootstrap_receipt.is_some()
            });
    if has_three_way_artifact {
        return validate_archived_three_way_method_identity_v1(document);
    }
    let single_mediation_cell = pls_general_single_mediation_bootstrap_capability_cell_v1();
    let has_single_mediation_artifact = document
        .general_sem_results
        .as_ref()
        .and_then(|results| results.inference_receipt.as_ref())
        .is_some_and(|receipt| receipt.capability_cell == single_mediation_cell)
        || document.provenance.method_version
            == GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1
        || document.provenance.engine_version
            == RECIPE_V4_GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1;
    if has_single_mediation_artifact {
        return validate_archived_single_mediation_method_identity_v1(document);
    }
    let combined_cell = pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    let has_combined_artifact = document
        .general_sem_results
        .as_ref()
        .and_then(|results| results.inference_receipt.as_ref())
        .is_some_and(|receipt| receipt.capability_cell == combined_cell)
        || document.provenance.method_version
            == GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1
        || document.provenance.engine_version
            == RECIPE_V4_GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        || document.tables.iter().any(|table| {
            matches!(
                table.id.as_str(),
                GENERAL_SEM_MODERATED_MEDIATION_GAMMA_TABLE_ID_V1
                    | GENERAL_SEM_CONDITIONAL_INDIRECT_TABLE_ID_V1
                    | GENERAL_SEM_MODERATED_MEDIATION_INDEX_TABLE_ID_V1
                    | GENERAL_SEM_MODERATED_MEDIATION_RECEIPT_TABLE_ID_V1
            )
        });
    if has_combined_artifact {
        return validate_archived_moderated_mediation_method_identity_v1(document);
    }
    let moderation_cell = general_sem_multiple_moderation_point_capability_cell_v1();
    let bootstrap_cell = general_sem_multiple_moderation_bootstrap_capability_cell_v1();
    let hoc_point_cell = pls_general_higher_order_point_capability_cell_v1();
    let hoc_bootstrap_cell = pls_general_higher_order_bootstrap_capability_cell_v1();
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
    let is_hoc_cell = project_cell_matches(cell, &hoc_point_cell);
    let has_hoc_artifact = document
        .general_sem_results
        .as_ref()
        .is_some_and(|results| {
            results.higher_order_stages.iter().any(|stage| {
                stage.receipt.is_some()
                    || stage.approach.is_some()
                    || !stage.generated_variable_mappings.is_empty()
            }) || results.higher_order_inference_receipt.is_some()
        })
        || document.sections.iter().any(|section| {
            matches!(
                section.id.as_str(),
                GENERAL_SEM_HIGHER_ORDER_SECTION_ID_V1
                    | GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_SECTION_ID_V1
            )
        })
        || document.tables.iter().any(|table| {
            matches!(
                table.id.as_str(),
                GENERAL_SEM_HIGHER_ORDER_STAGES_TABLE_ID_V1
                    | GENERAL_SEM_HIGHER_ORDER_TARGETS_TABLE_ID_V1
                    | GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_RECEIPT_TABLE_ID_V1
            )
        })
        || matches!(
            document.provenance.method_version.as_str(),
            GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1
                | GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1
        )
        || matches!(
            document.provenance.engine_version.as_str(),
            RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_POINT_EXECUTION_ADAPTER_VERSION_V1
                | RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1
        );
    if is_hoc_cell || has_hoc_artifact {
        if !is_hoc_cell {
            return Err(
                "archived non-HOC document contains injected General SEM HOC artifacts".into(),
            );
        }
        let results = document.general_sem_results.as_ref().ok_or_else(|| {
            "archived General SEM HOC document omits its typed scientific payload".to_string()
        })?;
        let approach = results
            .higher_order_stages
            .iter()
            .find_map(|stage| stage.approach.as_ref())
            .ok_or_else(|| {
                "archived General SEM HOC document omits its construction approach".to_string()
            })?;
        let expected_stage_count = match approach {
            qpls_core::HigherOrderConstructionApproachV4::RepeatedIndicators
            | qpls_core::HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators => 1,
            qpls_core::HigherOrderConstructionApproachV4::EmbeddedTwoStage
            | qpls_core::HigherOrderConstructionApproachV4::DisjointTwoStage => 2,
            qpls_core::HigherOrderConstructionApproachV4::Hybrid => {
                return Err(
                    "archived General SEM HOC document uses compatibility-only hybrid execution"
                        .into(),
                );
            }
        };
        let inferred = results.higher_order_inference_receipt.is_some();
        let (expected_method, expected_adapter) = if inferred {
            (
                GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1,
                RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1,
            )
        } else {
            (
                GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1,
                RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_POINT_EXECUTION_ADAPTER_VERSION_V1,
            )
        };
        if document.provenance.method_version != expected_method
            || document.provenance.engine_version != expected_adapter
            || document.title != higher_order_document_title_v1(approach, inferred)
            || results.higher_order_stages.len() != expected_stage_count
            || results.inference_receipt.is_some()
            || !results.specific_indirect_effects.is_empty()
            || !results.aggregate_effects.is_empty()
            || !results.interaction_effects.is_empty()
        {
            return Err(
                "archived General SEM HOC identity or exact typed inventory has drifted".into(),
            );
        }
        if inferred
            && results
                .higher_order_inference_receipt
                .as_ref()
                .is_none_or(|receipt| {
                    receipt.capability_cell != hoc_bootstrap_cell
                        || receipt.method_version
                            != GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1
                })
        {
            return Err(
                "archived General SEM HOC bootstrap receipt has a stale supplemental identity"
                    .into(),
            );
        }
        return Ok(());
    }
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
    if result.higher_order_point_estimation().is_some() {
        return build_recipe_v4_general_sem_pls_hoc_canonical_result_v1(
            job_id,
            project_id,
            dataset_id,
            started_at,
            completed_at,
            recipe,
            model,
            result,
        );
    }
    if result.three_way_point_estimation().is_some() {
        return build_recipe_v4_general_sem_pls_three_way_canonical_result_v1(
            job_id,
            project_id,
            dataset_id,
            started_at,
            completed_at,
            recipe,
            model,
            result,
        );
    }
    let interaction_point = result.interaction_point_estimation();
    let moderation_bootstrap = result.moderation_bootstrap_inference();
    let moderated_mediation_bootstrap = result.moderated_mediation_bootstrap_inference();
    let moderation_artifact = interaction_point
        .map(|interactions| {
            let artifact = compile_general_sem_pls_recipe_v1(recipe, Some(model)).map_err(|error| {
                vec![format!("General SEM moderation recompilation failed: {error}")]
            })?;
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
            if let Some(bootstrap) = moderated_mediation_bootstrap {
                let supplemental =
                    pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
                if artifact.supplemental_capability_admission() != Some(&supplemental)
                    || artifact.plan().two_way_moderated_mediation_target().is_none()
                    || !matches!(
                        recipe
                            .general_sem_config
                            .as_ref()
                            .map(|config| &config.inference),
                        Some(qpls_core::GeneralSemInferenceV1::CaseBootstrap { .. })
                    )
                {
                    return Err(vec![
                        "General SEM moderated-mediation result lacks its exact Registry-derived compiler/config authority"
                            .into(),
                    ]);
                }
                bootstrap
                    .ensure_valid_against_plan_v1(artifact.plan(), interactions)
                    .map_err(|error| {
                        vec![format!(
                            "General SEM moderated-mediation bootstrap failed compiled-plan validation: {error}"
                        )]
                    })?;
            }
            Ok(artifact)
        })
        .transpose()?;
    let moderation = moderation_artifact.is_some();
    let moderated_mediation = moderated_mediation_bootstrap.is_some();
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
    let single_mediation = general_sem_results
        .inference_receipt
        .as_ref()
        .is_some_and(|receipt| {
            receipt.capability_cell == pls_general_single_mediation_bootstrap_capability_cell_v1()
        });
    if moderation
        && inferred != (moderation_bootstrap.is_some() || moderated_mediation_bootstrap.is_some())
        || moderation_bootstrap.is_some() && moderated_mediation_bootstrap.is_some()
    {
        return Err(vec![
            "The typed moderation inference receipt and the mutually exclusive raw bootstrap results disagree"
                .into(),
        ]);
    }

    document.schema_version = CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
    document.title = if moderated_mediation {
        "General SEM two-way PLS moderated-mediation bootstrap inference".into()
    } else if moderation && inferred {
        "General SEM simultaneous two-way PLS moderation bootstrap inference".into()
    } else if moderation {
        "General SEM simultaneous two-way PLS moderation point estimates".into()
    } else if single_mediation {
        "PLS-SEM single mediation with full-model bootstrap".into()
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
    document.provenance.method_version = if moderated_mediation {
        GENERAL_SEM_PLS_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_METHOD_VERSION_V1.into()
    } else if moderation && inferred {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1.into()
    } else if moderation {
        GENERAL_SEM_PLS_MULTIPLE_TWO_WAY_POINT_METHOD_VERSION_V1.into()
    } else if single_mediation {
        GENERAL_SEM_PLS_SINGLE_MEDIATION_CASE_BOOTSTRAP_METHOD_VERSION_V1.into()
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
    let bootstrap_combination_cell = if moderated_mediation {
        pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1()
    } else if moderation {
        general_sem_multiple_moderation_bootstrap_capability_cell_v1()
    } else if single_mediation {
        pls_general_single_mediation_bootstrap_capability_cell_v1()
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
    if let Some(bootstrap) = moderated_mediation_bootstrap {
        let bootstrap_tables = moderated_mediation_bootstrap_tables(
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
            id: GENERAL_SEM_MODERATED_MEDIATION_BOOTSTRAP_SECTION_ID_V1.into(),
            title: "Moderated-mediation bootstrap inference".into(),
            description: Some(
                "One shared full-model case-bootstrap ledger for scientific gamma, three locked standardized-moderator conditional indirect effects, and the index of moderated mediation."
                    .into(),
            ),
            table_ids: bootstrap_table_ids,
            chart_ids: Vec::new(),
            capability_cells: Some(vec![bootstrap_combination_cell.clone()]),
        });
    } else if let (Some(bootstrap), true) = (moderation_bootstrap, moderation) {
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
    document.exclusions = if moderated_mediation {
        vec![
            CanonicalResultExclusion {
                id: "moderated_mediation_single_two_relation_target_only".into(),
                capability_cell: Some(bootstrap_combination_cell.clone()),
                title: "One bounded conditional-process target".into(),
                reason: "This exact cell supports one selected two-relation X to M to Y path and one first- or second-stage two-way interaction only."
                    .into(),
            },
            CanonicalResultExclusion {
                id: "moderated_mediation_locked_standardized_probes".into(),
                capability_cell: Some(bootstrap_combination_cell.clone()),
                title: "Conditional indirect probes are fixed".into(),
                reason: "Conditional indirect effects are reported only at standardized moderator values -1, 0, and +1; arbitrary probes and both-stage moderation remain deferred."
                    .into(),
            },
            CanonicalResultExclusion {
                id: "moderated_mediation_no_causal_claim".into(),
                capability_cell: Some(bootstrap_combination_cell),
                title: "No causal interpretation is implied".into(),
                reason: "The reported conditional-process estimands and percentile intervals are associational unless the research design independently supports causal interpretation."
                    .into(),
            },
        ]
    } else if moderation && inferred {
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
    document.presentation.default_section_id = Some(if moderated_mediation {
        GENERAL_SEM_MODERATED_MEDIATION_BOOTSTRAP_SECTION_ID_V1.into()
    } else if moderation && inferred {
        GENERAL_SEM_MODERATION_BOOTSTRAP_SECTION_ID_V1.into()
    } else if moderation {
        "general_sem_moderation".into()
    } else {
        "general_sem_effects".into()
    });
    document.presentation.default_table_id = Some(if moderated_mediation {
        GENERAL_SEM_CONDITIONAL_INDIRECT_TABLE_ID_V1.into()
    } else if moderation && inferred {
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

#[allow(clippy::too_many_arguments)]
fn build_recipe_v4_general_sem_pls_three_way_canonical_result_v1(
    job_id: Uuid,
    project_id: Uuid,
    dataset_id: Uuid,
    started_at: &str,
    completed_at: &str,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    result: &RecipeV4GeneralSemPlsExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, Vec<String>> {
    let artifact = compile_general_sem_pls_recipe_v1(recipe, Some(model)).map_err(|error| {
        vec![format!(
            "General SEM three-way recompilation failed: {error}"
        )]
    })?;
    if artifact.plan().three_way_interaction().is_none()
        || artifact.capability_cell()
            != &pls_general_three_way_moderation_point_capability_cell_v1()
        || result.capability_cell() != artifact.capability_cell()
        || result.compiled_plan_sha256() != artifact.plan().deterministic_sha256()
        || result.model_scientific_sha256() != artifact.plan().scientific_hash()
    {
        return Err(vec![
            "The three-way result differs from its exact compiled plan or point capability cell"
                .into(),
        ]);
    }
    let point = result.three_way_point_estimation().ok_or_else(|| {
        vec!["The three-way canonical path requires its typed point result".into()]
    })?;
    point
        .ensure_valid_against_plan_v1(artifact.plan())
        .map_err(|error| vec![format!("Three-way point validation failed: {error}")])?;
    if let Some(bootstrap) = result.three_way_bootstrap_inference() {
        bootstrap
            .ensure_valid_against_plan_v1(artifact.plan(), point)
            .map_err(|error| vec![format!("Three-way bootstrap validation failed: {error}")])?;
    }

    let (base_recipe, stage_one_model) = project_general_sem_pls_stage_one_recipe_v1(recipe, model)
        .map_err(|error| {
            vec![format!(
                "General SEM three-way stage-one projection failed: {error}"
            )]
        })?;
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
    let results = result
        .canonical_general_sem_results_v1()
        .map_err(|error| vec![format!("Three-way canonical projection failed: {error}")])?;
    let inferred = result.three_way_bootstrap_inference().is_some();
    if results.three_way_moderation_bootstrap_receipt.is_some() != inferred
        || results.three_way_interaction_effects.len() != 1
        || results.three_way_conditional_interaction_effects.is_empty()
        || results.three_way_simple_slopes.is_empty()
    {
        return Err(vec![
            "The typed three-way point, fixed-probe, or bootstrap receipt inventory is incomplete"
                .into(),
        ]);
    }

    let point_cell = pls_general_three_way_moderation_point_capability_cell_v1();
    let bootstrap_cell = pls_general_three_way_moderation_bootstrap_capability_cell_v1();
    let base_cell =
        RecipeV4CompilerTarget::PlsPlanV2.capability_cell_for_method(recipe.settings.method);
    let mut cells = vec![base_cell, point_cell.clone()];
    if inferred {
        cells.push(bootstrap_cell.clone());
    }
    sort_and_deduplicate_cells(&mut cells);
    let table_cells = if inferred {
        vec![point_cell.clone(), bootstrap_cell.clone()]
    } else {
        vec![point_cell.clone()]
    };
    project_canonical_joint_stage_two_structural_tables_v1(
        &mut document,
        artifact.plan(),
        &results.joint_stage_structural_coefficients,
        model,
        &point_cell,
    )?;
    let tables = three_way_tables_v1(&results, &table_cells, &bootstrap_cell)?;
    let table_ids = tables
        .iter()
        .map(|table| table.id.clone())
        .collect::<Vec<_>>();
    document.tables.extend(tables);
    let charts = three_way_simple_slope_charts_v1(&results);
    let chart_ids = charts
        .iter()
        .map(|chart| chart.id.clone())
        .collect::<Vec<_>>();
    document.charts.extend(charts);
    document.sections.push(CanonicalResultSection {
        id: GENERAL_SEM_THREE_WAY_SECTION_ID_V1.into(),
        title: "Three-way moderation".into(),
        description: Some(
            "Three-way coefficient, conditional two-way interaction effects, and two-dimensional simple slopes from one strong-hierarchy stage-two model."
                .into(),
        ),
        table_ids,
        chart_ids,
        capability_cells: Some(table_cells),
    });
    document.schema_version = CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
    document.title = if inferred {
        "General SEM three-way PLS moderation bootstrap inference".into()
    } else {
        "General SEM three-way PLS moderation point estimates".into()
    };
    document.provenance.model_id = model.id.clone();
    document.provenance.model_digest = result.model_scientific_sha256().into();
    document.provenance.dataset_id = dataset_id.to_string();
    document.provenance.dataset_fingerprint = result.source_dataset_fingerprint().into();
    document.provenance.recipe_id = recipe.id.to_string();
    document.provenance.recipe_digest = result.recipe_analytical_sha256().into();
    document.provenance.capability_cell = point_cell.clone();
    document.provenance.method_version = if inferred {
        GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_METHOD_VERSION_V1.into()
    } else {
        GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1.into()
    };
    document.provenance.engine_version = if inferred {
        RECIPE_V4_GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1.into()
    } else {
        RECIPE_V4_GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_EXECUTION_ADAPTER_VERSION_V1.into()
    };
    document.provenance.seed = Some(i64::try_from(recipe.settings.seed).map_err(|_| {
        vec!["General SEM recipe seed exceeds the canonical safe-integer boundary".into()]
    })?);
    document.provenance.workers = i64::try_from(recipe.settings.workers)
        .ok()
        .filter(|workers| *workers > 0)
        .ok_or_else(|| vec!["General SEM recipe worker count is invalid".into()])?;
    document.capability_cells = Some(cells);
    document.general_sem_results = Some(results);
    document.exclusions = vec![CanonicalResultExclusion {
        id: "three_way_v1_bounded_scope".into(),
        capability_cell: Some(if inferred { bootstrap_cell } else { point_cell }),
        title: "Bounded three-way scope".into(),
        reason: "This method supports one strong-hierarchy three-way term and fixed standardized or binary moderator probes; fourth-order, HOC interaction, and three-way moderated mediation are outside this method version."
            .into(),
    }];
    document.presentation.default_section_id = Some(GENERAL_SEM_THREE_WAY_SECTION_ID_V1.into());
    document.presentation.default_table_id =
        Some(GENERAL_SEM_THREE_WAY_SIMPLE_SLOPES_TABLE_ID_V1.into());

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
            "schema-6 three-way canonical validation failed: {error}"
        )]
    })?;
    Ok(document)
}

fn three_way_estimate_cells_v1(
    estimate: &CanonicalGeneralSemEstimateV1,
) -> Vec<CanonicalResultCell> {
    vec![
        number(estimate.estimate),
        optional_number(estimate.standard_error),
        optional_number(estimate.lower),
        optional_number(estimate.upper),
        optional_number(estimate.p_value),
    ]
}

fn three_way_estimate_columns_v1() -> Vec<CanonicalResultColumn> {
    vec![
        number_column("estimate", "Estimate", "Point estimate."),
        number_column(
            "standard_error",
            "Std. error",
            "Bootstrap standard error when requested.",
        ),
        number_column("lower", "Lower", "Lower percentile bound when requested."),
        number_column("upper", "Upper", "Upper percentile bound when requested."),
        number_column(
            "p_value",
            "P value",
            "Two-sided bootstrap probability when requested.",
        ),
    ]
}

fn three_way_tables_v1(
    results: &CanonicalGeneralSemResultsV1,
    table_cells: &[CapabilityCellReferenceV2],
    bootstrap_cell: &CapabilityCellReferenceV2,
) -> Result<Vec<CanonicalResultTable>, Vec<String>> {
    let effect_columns = [
        text_column("effect_id", "Effect", "Stable three-way effect identity."),
        text_column(
            "interaction_id",
            "Interaction",
            "Stable interaction term identity.",
        ),
        text_column(
            "focal_relation_id",
            "Focal path",
            "Authored focal structural relation.",
        ),
        text_column(
            "operands",
            "Operands",
            "Ordered focal predictor and two moderators.",
        ),
        text_column("outcome_id", "Outcome", "Focal outcome identity."),
    ]
    .into_iter()
    .chain(three_way_estimate_columns_v1())
    .collect::<Vec<_>>();
    let effect_rows = results
        .three_way_interaction_effects
        .iter()
        .enumerate()
        .map(|(index, effect)| CanonicalResultRow {
            id: format!("three_way_effect_{index:04}"),
            cells: [
                text(&effect.effect_id),
                text(&effect.interaction_id),
                text(&effect.focal_relation_id),
                text(&effect.operand_ids.join(" × ")),
                text(&effect.outcome_id),
            ]
            .into_iter()
            .chain(three_way_estimate_cells_v1(
                &effect.scientific_rescaled_delta,
            ))
            .collect(),
        })
        .collect();
    let conditional_columns = [
        text_column(
            "effect_id",
            "Effect",
            "Stable conditional-interaction identity.",
        ),
        text_column(
            "interaction_id",
            "Interaction",
            "Stable interaction term identity.",
        ),
        text_column(
            "first_moderator",
            "Moderator 1",
            "First moderator identity.",
        ),
        text_column(
            "second_moderator",
            "Moderator 2",
            "Second moderator identity.",
        ),
        number_column(
            "second_value",
            "Moderator 2 value",
            "Fixed standardized or binary probe.",
        ),
    ]
    .into_iter()
    .chain(three_way_estimate_columns_v1())
    .collect::<Vec<_>>();
    let conditional_rows = results
        .three_way_conditional_interaction_effects
        .iter()
        .enumerate()
        .map(|(index, effect)| CanonicalResultRow {
            id: format!("three_way_conditional_{index:04}"),
            cells: [
                text(&effect.effect_id),
                text(&effect.interaction_id),
                text(&effect.first_moderator_id),
                text(&effect.second_moderator_id),
                number(effect.second_moderator_value),
            ]
            .into_iter()
            .chain(three_way_estimate_cells_v1(&effect.value))
            .collect(),
        })
        .collect();
    let slope_columns = [
        text_column("effect_id", "Slope", "Stable simple-slope identity."),
        text_column(
            "interaction_id",
            "Interaction",
            "Stable interaction term identity.",
        ),
        text_column(
            "first_moderator",
            "Moderator 1",
            "First moderator identity.",
        ),
        number_column(
            "first_value",
            "Moderator 1 value",
            "Fixed standardized or binary probe.",
        ),
        text_column(
            "second_moderator",
            "Moderator 2",
            "Second moderator identity.",
        ),
        number_column(
            "second_value",
            "Moderator 2 value",
            "Fixed standardized or binary probe.",
        ),
    ]
    .into_iter()
    .chain(three_way_estimate_columns_v1())
    .collect::<Vec<_>>();
    let slope_rows = results
        .three_way_simple_slopes
        .iter()
        .enumerate()
        .map(|(index, effect)| CanonicalResultRow {
            id: format!("three_way_slope_{index:04}"),
            cells: [
                text(&effect.effect_id),
                text(&effect.interaction_id),
                text(&effect.first_moderator_id),
                number(effect.first_moderator_value),
                text(&effect.second_moderator_id),
                number(effect.second_moderator_value),
            ]
            .into_iter()
            .chain(three_way_estimate_cells_v1(&effect.value))
            .collect(),
        })
        .collect();
    let mut tables = vec![
        CanonicalResultTable {
            id: GENERAL_SEM_THREE_WAY_EFFECT_TABLE_ID_V1.into(),
            title: "Three-way interaction".into(),
            description: Some(
                "Scientific three-way coefficient from the joint strong-hierarchy model.".into(),
            ),
            columns: effect_columns,
            rows: effect_rows,
            footnote_ids: Vec::new(),
            capability_cells: Some(table_cells.to_vec()),
        },
        CanonicalResultTable {
            id: GENERAL_SEM_THREE_WAY_CONDITIONAL_TABLE_ID_V1.into(),
            title: "Conditional two-way interaction".into(),
            description: Some("The focal X-by-W interaction conditional on fixed Z probes.".into()),
            columns: conditional_columns,
            rows: conditional_rows,
            footnote_ids: Vec::new(),
            capability_cells: Some(table_cells.to_vec()),
        },
        CanonicalResultTable {
            id: GENERAL_SEM_THREE_WAY_SIMPLE_SLOPES_TABLE_ID_V1.into(),
            title: "Simple slopes".into(),
            description: Some(
                "Simple slopes of the focal predictor across the two-moderator probe grid.".into(),
            ),
            columns: slope_columns,
            rows: slope_rows,
            footnote_ids: Vec::new(),
            capability_cells: Some(table_cells.to_vec()),
        },
    ];
    if let Some(receipt) = &results.three_way_moderation_bootstrap_receipt {
        tables.push(CanonicalResultTable {
            id: GENERAL_SEM_THREE_WAY_RECEIPT_TABLE_ID_V1.into(),
            title: "Three-way bootstrap receipt".into(),
            description: Some(
                "Shared full-model case-bootstrap and failure-ledger receipt.".into(),
            ),
            columns: vec![
                text_column("method", "Method", "Frozen three-way bootstrap method."),
                number_column("requested", "Requested", "Requested replicates."),
                number_column("usable", "Usable", "Usable full-model refits."),
                text_column("seed", "Seed", "Deterministic resampling seed."),
                number_column("workers", "Workers", "Configured worker count."),
                number_column("failed", "Failed", "Typed failed-replicate count."),
            ],
            rows: vec![CanonicalResultRow {
                id: "three_way_bootstrap_receipt".into(),
                cells: vec![
                    text(&receipt.method_version),
                    number(f64::from(receipt.resamples_requested)),
                    number(f64::from(receipt.resamples_usable)),
                    text(&receipt.seed),
                    number(f64::from(receipt.workers)),
                    number(receipt.failed_replicates.len() as f64),
                ],
            }],
            footnote_ids: Vec::new(),
            capability_cells: Some(vec![bootstrap_cell.clone()]),
        });
    }
    Ok(tables)
}

fn three_way_simple_slope_charts_v1(
    results: &CanonicalGeneralSemResultsV1,
) -> Vec<CanonicalResultChart> {
    let Some(first) = results.three_way_simple_slopes.first() else {
        return Vec::new();
    };
    let mut second_probes = results
        .three_way_simple_slopes
        .iter()
        .map(|row| (row.second_probe_index, row.second_moderator_value))
        .collect::<Vec<_>>();
    second_probes.sort_by_key(|(index, _)| *index);
    second_probes.dedup_by_key(|(index, _)| *index);
    vec![CanonicalResultChart {
        id: "general_sem_three_way_simple_slope_chart".into(),
        title: "Simple slopes across moderator probes".into(),
        description: format!(
            "Simple slope of the focal predictor across {} values, grouped by {}.",
            first.first_moderator_id, first.second_moderator_id
        ),
        kind: CanonicalChartKind::Line,
        series: second_probes
            .into_iter()
            .map(|(probe_index, probe_value)| CanonicalChartSeries {
                id: format!("second_probe_{probe_index:04}"),
                label: format!("{} = {probe_value:.4}", first.second_moderator_id),
                group: Some(first.interaction_id.clone()),
                points: results
                    .three_way_simple_slopes
                    .iter()
                    .filter(|row| row.second_probe_index == probe_index)
                    .map(|row| CanonicalChartPoint {
                        x: CanonicalChartX::Number(row.first_moderator_value),
                        y: row.value.estimate,
                        lower: row.value.lower,
                        upper: row.value.upper,
                        label: None,
                    })
                    .collect(),
            })
            .collect(),
        source_table_id: Some(GENERAL_SEM_THREE_WAY_SIMPLE_SLOPES_TABLE_ID_V1.into()),
        display: CanonicalChartDisplayOptions {
            palette: None,
            show_legend: Some(true),
            show_values: Some(false),
            x_axis_label: Some(first.first_moderator_id.clone()),
            y_axis_label: Some("Simple slope of focal predictor".into()),
        },
    }]
}

#[allow(clippy::too_many_arguments)]
fn build_recipe_v4_general_sem_pls_hoc_canonical_result_v1(
    job_id: Uuid,
    project_id: Uuid,
    dataset_id: Uuid,
    started_at: &str,
    completed_at: &str,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    result: &RecipeV4GeneralSemPlsExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, Vec<String>> {
    let artifact = compile_general_sem_pls_recipe_v1(recipe, Some(model))
        .map_err(|error| vec![format!("General SEM HOC recompilation failed: {error}")])?;
    let [hoc_plan] = artifact.plan().higher_order_stage_plans() else {
        return Err(vec![
            "The internal HOC canonical path requires exactly one compiled higher-order stage plan"
                .into(),
        ]);
    };
    if result.capability_cell() != &pls_general_higher_order_point_capability_cell_v1()
        || artifact.capability_cell() != result.capability_cell()
        || result.compiled_plan_sha256() != artifact.plan().deterministic_sha256()
        || result.model_scientific_sha256() != artifact.plan().scientific_hash()
    {
        return Err(vec![
            "The HOC result differs from the exact compiled plan or point capability cell".into(),
        ]);
    }
    let point = result.higher_order_point_estimation().ok_or_else(|| {
        vec!["The HOC canonical path requires its typed point-stage result".into()]
    })?;
    point
        .ensure_valid_against_plan_v1(artifact.plan())
        .map_err(|error| {
            vec![format!(
                "HOC point result failed recompiled-plan validation: {error}"
            )]
        })?;
    if let Some(bootstrap) = result.higher_order_bootstrap_inference() {
        bootstrap
            .ensure_valid_against_plan_v1(artifact.plan(), point)
            .map_err(|error| {
                vec![format!(
                    "HOC bootstrap result failed recompiled-plan validation: {error}"
                )]
            })?;
    }

    let (base_recipe, stage_one_model) = project_general_sem_pls_stage_one_recipe_v1(recipe, model)
        .map_err(|error| {
            vec![format!(
                "General SEM HOC stage-one projection failed: {error}"
            )]
        })?;
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
    let general_sem_results = result
        .canonical_general_sem_results_v1()
        .map_err(|error| vec![format!("HOC canonical projection failed: {error}")])?;
    if !general_sem_results.specific_indirect_effects.is_empty()
        || !general_sem_results.aggregate_effects.is_empty()
        || !general_sem_results.interaction_effects.is_empty()
        || general_sem_results.inference_receipt.is_some()
        || general_sem_results.higher_order_stages.len() != hoc_plan.stage_projections().len()
        || general_sem_results.higher_order_inference_receipt.is_some()
            != result.higher_order_bootstrap_inference().is_some()
    {
        return Err(vec![
            "The bounded HOC canonical inventory contains non-HOC effects or mismatched inference authority"
                .into(),
        ]);
    }

    let point_cell = pls_general_higher_order_point_capability_cell_v1();
    let bootstrap_cell = pls_general_higher_order_bootstrap_capability_cell_v1();
    let inferred = result.higher_order_bootstrap_inference().is_some();
    document.schema_version = CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
    document.title = higher_order_document_title_v1(hoc_plan.approach(), inferred);
    document.provenance.model_id = model.id.clone();
    document.provenance.model_digest = result.model_scientific_sha256().into();
    document.provenance.dataset_id = dataset_id.to_string();
    document.provenance.dataset_fingerprint = result.source_dataset_fingerprint().into();
    document.provenance.recipe_id = recipe.id.to_string();
    document.provenance.recipe_digest = result.recipe_analytical_sha256().into();
    document.provenance.capability_cell = point_cell.clone();
    document.provenance.method_version = if inferred {
        GENERAL_SEM_PLS_DISJOINT_HOC_BOOTSTRAP_METHOD_VERSION_V1.into()
    } else {
        GENERAL_SEM_PLS_DISJOINT_HIGHER_ORDER_POINT_METHOD_VERSION_V1.into()
    };
    document.provenance.engine_version = if inferred {
        RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_BOOTSTRAP_EXECUTION_ADAPTER_VERSION_V1.into()
    } else {
        RECIPE_V4_GENERAL_SEM_PLS_HIGHER_ORDER_POINT_EXECUTION_ADAPTER_VERSION_V1.into()
    };
    document.provenance.seed = Some(i64::try_from(recipe.settings.seed).map_err(|_| {
        vec!["General SEM HOC recipe seed exceeds the canonical safe-integer boundary".into()]
    })?);
    document.provenance.workers = i64::try_from(recipe.settings.workers)
        .ok()
        .filter(|workers| *workers > 0)
        .ok_or_else(|| vec!["General SEM HOC recipe worker count is invalid".into()])?;

    let base_cell =
        RecipeV4CompilerTarget::PlsPlanV2.capability_cell_for_method(recipe.settings.method);
    let mut document_cells = vec![base_cell, point_cell.clone()];
    if inferred {
        document_cells.push(bootstrap_cell.clone());
    }
    sort_and_deduplicate_cells(&mut document_cells);
    document.capability_cells = Some(document_cells);

    for section in &mut document.sections {
        if matches!(
            section.id.as_str(),
            "run_details" | "measurement_model" | "structural_model"
        ) {
            section.title = format!("Stage-one {}", section.title.to_lowercase());
            section.description = Some(
                "Lower-order construct score estimation only; final HOC coefficients are reported in the higher-order section."
                    .into(),
            );
        }
    }
    let stage_table = higher_order_stage_table_v1(&general_sem_results, &point_cell)?;
    let target_table = higher_order_target_table_v1(&general_sem_results, &point_cell)?;
    document.tables.push(stage_table);
    document.tables.push(target_table);
    document.sections.push(CanonicalResultSection {
        id: GENERAL_SEM_HIGHER_ORDER_SECTION_ID_V1.into(),
        title: "Higher-order construct stages and targets".into(),
        description: Some(
            "Exact approach-specific stage projections, generated mappings, component loadings or weights, authored HOC paths, and separated technical effects."
                .into(),
        ),
        table_ids: vec![
            GENERAL_SEM_HIGHER_ORDER_STAGES_TABLE_ID_V1.into(),
            GENERAL_SEM_HIGHER_ORDER_TARGETS_TABLE_ID_V1.into(),
        ],
        chart_ids: Vec::new(),
        capability_cells: Some(vec![point_cell.clone()]),
    });
    if inferred {
        document
            .tables
            .push(higher_order_bootstrap_receipt_table_v1(
                &general_sem_results,
                &bootstrap_cell,
            )?);
        document.sections.push(CanonicalResultSection {
            id: GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_SECTION_ID_V1.into(),
            title: "Higher-order full-model case bootstrap".into(),
            description: Some(
                "One shared usable/failure ledger for the exact HOC component and HOC structural-path target inventory."
                    .into(),
            ),
            table_ids: vec![GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_RECEIPT_TABLE_ID_V1.into()],
            chart_ids: Vec::new(),
            capability_cells: Some(vec![bootstrap_cell.clone()]),
        });
    }
    document.general_sem_results = Some(general_sem_results);
    document.exclusions = vec![CanonicalResultExclusion {
        id: if inferred {
            "higher_order_bootstrap_bounded_targets_only".into()
        } else {
            "higher_order_point_estimation_only".into()
        },
        capability_cell: Some(if inferred {
            bootstrap_cell.clone()
        } else {
            point_cell.clone()
        }),
        title: if inferred {
            "Bootstrap inference is limited to exact HOC targets".into()
        } else {
            "Higher-order inference was not requested".into()
        },
        reason: if inferred {
            "This checkpoint infers HOC component loadings or weights and authored structural paths touching the HOC; ordinary controls and unrelated paths remain point-only."
                .into()
        } else {
            "The resident GeneralSemConfigV1 requested bounded higher-order PLS point estimation without case bootstrap."
                .into()
        },
    }];
    document.presentation.default_section_id = Some(if inferred {
        GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_SECTION_ID_V1.into()
    } else {
        GENERAL_SEM_HIGHER_ORDER_SECTION_ID_V1.into()
    });
    document.presentation.default_table_id =
        Some(GENERAL_SEM_HIGHER_ORDER_TARGETS_TABLE_ID_V1.into());

    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(validation.errors);
    }
    let archive_value = serde_json::to_value(&document).map_err(|error| {
        vec![format!(
            "HOC canonical result serialization failed: {error}"
        )]
    })?;
    let archive_document =
        serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(archive_value)
            .map_err(|error| vec![format!("schema-6 HOC result parsing failed: {error}")])?;
    archive_document
        .ensure_valid()
        .map_err(|error| vec![format!("schema-6 HOC canonical validation failed: {error}")])?;
    Ok(document)
}

fn higher_order_approach_label_v1(
    approach: &qpls_core::HigherOrderConstructionApproachV4,
) -> &'static str {
    match approach {
        qpls_core::HigherOrderConstructionApproachV4::RepeatedIndicators => "repeated indicators",
        qpls_core::HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators => {
            "extended repeated indicators"
        }
        qpls_core::HigherOrderConstructionApproachV4::EmbeddedTwoStage => "embedded two-stage",
        qpls_core::HigherOrderConstructionApproachV4::DisjointTwoStage => "disjoint two-stage",
        qpls_core::HigherOrderConstructionApproachV4::Hybrid => "hybrid",
    }
}

fn higher_order_approach_wire_label_v1(
    approach: &qpls_core::HigherOrderConstructionApproachV4,
) -> &'static str {
    match approach {
        qpls_core::HigherOrderConstructionApproachV4::RepeatedIndicators => "repeated_indicators",
        qpls_core::HigherOrderConstructionApproachV4::ExtendedRepeatedIndicators => {
            "extended_repeated_indicators"
        }
        qpls_core::HigherOrderConstructionApproachV4::EmbeddedTwoStage => "embedded_two_stage",
        qpls_core::HigherOrderConstructionApproachV4::DisjointTwoStage => "disjoint_two_stage",
        qpls_core::HigherOrderConstructionApproachV4::Hybrid => "hybrid",
    }
}

fn higher_order_document_title_v1(
    approach: &qpls_core::HigherOrderConstructionApproachV4,
    inferred: bool,
) -> String {
    format!(
        "General SEM {} higher-order PLS {}",
        higher_order_approach_label_v1(approach),
        if inferred {
            "bootstrap inference"
        } else {
            "point estimates"
        }
    )
}

fn higher_order_stage_table_v1(
    results: &CanonicalGeneralSemResultsV1,
    point_cell: &CapabilityCellReferenceV2,
) -> Result<CanonicalResultTable, Vec<String>> {
    let rows = results
        .higher_order_stages
        .iter()
        .enumerate()
        .map(|(index, stage)| {
            let receipt = stage.receipt.as_ref().ok_or_else(|| {
                vec![format!(
                    "HOC stage {} omits its point receipt",
                    stage.stage_id
                )]
            })?;
            let generated = receipt.generated_score_dataset.as_ref();
            Ok(CanonicalResultRow {
                id: format!("higher_order_stage_{index:04}"),
                cells: vec![
                    text(&stage.stage_id),
                    text(&stage.higher_order_construct_id),
                    number(f64::from(stage.stage_number)),
                    text(match stage.kind {
                        qpls_core::CanonicalHocStageKindV1::LowerOrderScoreEstimation => {
                            "lower_order_score_estimation"
                        }
                        qpls_core::CanonicalHocStageKindV1::HigherOrderEstimation => {
                            "higher_order_estimation"
                        }
                    }),
                    text(stage.approach.as_ref().map_or("missing", higher_order_approach_wire_label_v1)),
                    text(match stage.measurement_type.as_ref() {
                        Some(qpls_core::HigherOrderMeasurementTypeV4::ReflectiveReflective) => {
                            "reflective_reflective"
                        }
                        Some(qpls_core::HigherOrderMeasurementTypeV4::ReflectiveFormative) => {
                            "reflective_formative"
                        }
                        Some(qpls_core::HigherOrderMeasurementTypeV4::FormativeReflective) => {
                            "formative_reflective"
                        }
                        Some(qpls_core::HigherOrderMeasurementTypeV4::FormativeFormative) => {
                            "formative_formative"
                        }
                        None => "missing",
                    }),
                    text(match receipt.role {
                        qpls_core::CompiledPlsHocStageRoleV1::RepeatedIndicatorEstimation => {
                            "repeated_indicator_estimation"
                        }
                        qpls_core::CompiledPlsHocStageRoleV1::ExtendedRepeatedIndicatorEstimation => {
                            "extended_repeated_indicator_estimation"
                        }
                        qpls_core::CompiledPlsHocStageRoleV1::EmbeddedRepeatedIndicatorEstimation => {
                            "embedded_repeated_indicator_estimation"
                        }
                        qpls_core::CompiledPlsHocStageRoleV1::DisjointLowerOrderScoreEstimation => {
                            "disjoint_lower_order_score_estimation"
                        }
                        qpls_core::CompiledPlsHocStageRoleV1::HigherOrderFromLowerOrderScores => {
                            "higher_order_from_lower_order_scores"
                        }
                    }),
                    text(stage.input_construct_ids.join(";")),
                    text(stage.output_variable_ids.join(";")),
                    text(&receipt.receipt_version),
                    text(&receipt.projection_identity_sha256),
                    text(&receipt.model_scientific_sha256),
                    text(&receipt.compiled_plan_sha256),
                    text(&receipt.dataset_fingerprint),
                    number(f64::from(receipt.used_observations)),
                    number(f64::from(receipt.omitted_observations)),
                    optional_text(generated.map(|value| value.receipt_version.as_str())),
                    optional_text(generated.map(|value| value.complete_case_rows_sha256.as_str())),
                ],
            })
        })
        .collect::<Result<Vec<_>, Vec<String>>>()?;
    Ok(CanonicalResultTable {
        id: GENERAL_SEM_HIGHER_ORDER_STAGES_TABLE_ID_V1.into(),
        title: "Higher-order stage receipts".into(),
        description: Some(
            "Ordered approach-specific stage projections and generated-score dataset identities for the bounded HOC."
                .into(),
        ),
        columns: vec![
            text_column("stage_id", "Stage ID", "Stable compiled stage identity."),
            text_column("higher_order_construct_id", "HOC", "Higher-order construct identity."),
            number_column("stage_number", "Stage", "One-based stage number."),
            text_column("stage_kind", "Kind", "Canonical stage kind."),
            text_column("approach", "Approach", "Frozen HOC construction approach."),
            text_column("measurement_type", "HCM type", "Frozen higher-order measurement type."),
            text_column("stage_role", "Role", "Exact compiled stage role."),
            text_column("input_construct_ids", "Inputs", "Ordered input construct identities."),
            text_column("output_variable_ids", "Outputs", "Ordered output variable identities."),
            text_column("receipt_version", "Receipt", "Point-stage receipt version."),
            text_column("projection_identity_sha256", "Projection digest", "Compiled projection identity."),
            text_column("model_scientific_sha256", "Model digest", "Stage scientific model digest."),
            text_column("compiled_plan_sha256", "Plan digest", "Stage compiled-plan digest."),
            text_column("dataset_fingerprint", "Dataset", "Source dataset fingerprint."),
            number_column("used_observations", "Used", "Complete observations used."),
            number_column("omitted_observations", "Omitted", "Rows omitted by listwise deletion."),
            text_column("generated_score_receipt_version", "Score receipt", "Generated-score receipt version when applicable."),
            text_column("complete_case_rows_sha256", "Row digest", "Complete-case row identity when applicable."),
        ],
        rows,
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![point_cell.clone()]),
    })
}

fn higher_order_target_table_v1(
    results: &CanonicalGeneralSemResultsV1,
    point_cell: &CapabilityCellReferenceV2,
) -> Result<CanonicalResultTable, Vec<String>> {
    let stage = results
        .higher_order_stages
        .iter()
        .find(|stage| stage.kind == qpls_core::CanonicalHocStageKindV1::HigherOrderEstimation)
        .ok_or_else(|| vec!["HOC canonical payload omits its final estimation stage".into()])?;
    Ok(CanonicalResultTable {
        id: GENERAL_SEM_HIGHER_ORDER_TARGETS_TABLE_ID_V1.into(),
        title: "Higher-order component and structural targets".into(),
        description: Some(
            "Component loadings or weights, authored structural paths, collinearity, and optional shared-ledger bootstrap inference."
                .into(),
        ),
        columns: vec![
            text_column("relation_id", "Relation ID", "Stable relation or target identity."),
            text_column("parameter_id", "Parameter ID", "Stable parameter identity."),
            text_column("kind", "Kind", "Loading, weight, structural, or control interpretation."),
            text_column("source_id", "Source", "Relation source identity."),
            text_column("target_id", "Target", "Relation target identity."),
            number_column("estimate", "Estimate", "Point estimate."),
            number_column("collinearity_vif", "VIF", "Formative component collinearity when applicable."),
            number_column("bootstrap_mean", "Bootstrap mean", "Mean across usable replicates."),
            number_column("bootstrap_bias", "Bias", "Bootstrap mean minus point estimate."),
            number_column("standard_error", "SE", "Bootstrap sample standard error."),
            number_column("lower", "Lower", "Percentile Type-7 lower bound."),
            number_column("upper", "Upper", "Percentile Type-7 upper bound."),
            number_column("p_value", "P value", "Two-sided plus-one probability."),
            number_column("usable_replicates", "Usable", "Shared usable replicate count."),
            number_column("two_sided_exceedances", "Exceedances", "Shared-ledger two-sided exceedance count."),
        ],
        rows: stage
            .relation_estimates
            .iter()
            .enumerate()
            .map(|(index, relation)| CanonicalResultRow {
                id: format!("higher_order_target_{index:04}"),
                cells: vec![
                    text(&relation.relation_id),
                    optional_text(relation.parameter_id.as_deref()),
                    text(match relation.kind {
                        Some(qpls_core::CanonicalHocRelationKindV1::ComponentLoading) => {
                            "component_loading"
                        }
                        Some(qpls_core::CanonicalHocRelationKindV1::ComponentWeight) => {
                            "component_weight"
                        }
                        Some(qpls_core::CanonicalHocRelationKindV1::AuthoredStructural) => {
                            "authored_structural"
                        }
                        Some(qpls_core::CanonicalHocRelationKindV1::AuthoredControl) => {
                            "authored_control"
                        }
                        Some(qpls_core::CanonicalHocRelationKindV1::TechnicalStructural) => {
                            "technical_structural"
                        }
                        Some(qpls_core::CanonicalHocRelationKindV1::ExtendedIndirectEffect) => {
                            "extended_indirect_effect"
                        }
                        Some(qpls_core::CanonicalHocRelationKindV1::ExtendedTotalEffect) => {
                            "extended_total_effect"
                        }
                        None => "missing",
                    }),
                    text(&relation.source_id),
                    text(&relation.target_id),
                    number(relation.value.estimate),
                    optional_number(relation.collinearity_vif),
                    optional_number(relation.value.bootstrap_mean),
                    optional_number(relation.value.bootstrap_bias),
                    optional_number(relation.value.standard_error),
                    optional_number(relation.value.lower),
                    optional_number(relation.value.upper),
                    optional_number(relation.value.p_value),
                    optional_number(relation.value.bootstrap_usable_replicates.map(f64::from)),
                    optional_number(
                        relation
                            .value
                            .bootstrap_two_sided_exceedances
                            .map(f64::from),
                    ),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![point_cell.clone()]),
    })
}

fn higher_order_bootstrap_receipt_table_v1(
    results: &CanonicalGeneralSemResultsV1,
    bootstrap_cell: &CapabilityCellReferenceV2,
) -> Result<CanonicalResultTable, Vec<String>> {
    let receipt = results
        .higher_order_inference_receipt
        .as_ref()
        .ok_or_else(|| vec!["HOC bootstrap table requires its typed receipt".into()])?;
    Ok(CanonicalResultTable {
        id: GENERAL_SEM_HIGHER_ORDER_BOOTSTRAP_RECEIPT_TABLE_ID_V1.into(),
        title: "Higher-order bootstrap receipt".into(),
        description: Some(
            "Exact two-stage refit, target, usable-replicate, failure-ledger, seed, and worker authority."
                .into(),
        ),
        columns: vec![
            text_column("capability_id", "Capability", "Capability owner."),
            text_column("cell_id", "Cell", "Exact supplemental bootstrap cell."),
            text_column("capability_version", "Version", "Exact capability version."),
            text_column("method_version", "Method", "Bootstrap method version."),
            text_column("point_method_version", "Point method", "Bound point method version."),
            text_column("resampling_operation_version", "Operation", "Resampling operation version."),
            text_column("resampling_stream_version", "Stream", "Indexed stream version."),
            text_column("quantile_method_version", "Quantile", "Quantile method version."),
            text_column("standard_error_method_version", "SE method", "Standard-error method version."),
            text_column("summation_method_version", "Summation", "Summation method version."),
            text_column("p_value_method_version", "P method", "P-value method version."),
            text_column("failure_policy_version", "Failure policy", "Usable-gate policy version."),
            text_column("sign_alignment_method_version", "Alignment", "Score sign-alignment method."),
            text_column("target_version", "Target version", "Exact target identity version."),
            text_column("general_sem_config_sha256", "Config digest", "General SEM config digest."),
            text_column("compiled_plan_sha256", "Plan digest", "Compiled PLS v3 plan digest."),
            text_column("hoc_stage_plan_sha256", "HOC digest", "Compiled HOC stage-plan digest."),
            text_column("model_scientific_sha256", "Model digest", "Full scientific model digest."),
            text_column("stage_one_model_scientific_sha256", "Stage-one digest", "Stage-one model digest."),
            text_column("stage_two_model_scientific_sha256", "Stage-two digest", "Stage-two model digest."),
            text_column("source_dataset_fingerprint", "Dataset", "Source dataset fingerprint."),
            text_column("complete_case_frame_sha256", "Frame digest", "Complete-case frame identity."),
            text_column("usable_replicate_indices_sha256", "Usable digest", "Usable index ledger digest."),
            text_column("target_identity_set_sha256", "Target digest", "Target identity-set digest."),
            text_column("target_ids", "Targets", "Ordered target identities."),
            number_column("confidence_level", "Confidence", "Confidence level."),
            number_column("resamples_requested", "Requested", "Requested replicates."),
            number_column("resamples_usable", "Usable", "Usable replicates."),
            number_column("minimum_usable_resamples", "Minimum", "Minimum usable gate."),
            text_column("seed", "Seed", "Canonical decimal seed."),
            number_column("workers", "Workers", "Worker count."),
            boolean_column("complete_model_reestimated_per_replicate", "Complete refit", "Complete model refit flag."),
            boolean_column("stage_one_reestimated_per_replicate", "Stage one", "Stage-one refit flag."),
            boolean_column("generated_component_values_recalculated_per_replicate", "Generated values", "Generated-value recalculation flag."),
            boolean_column("stage_one_scores_sign_aligned_per_replicate", "Stage-one alignment", "Stage-one sign-alignment flag."),
            boolean_column("stage_two_reestimated_per_replicate", "Stage two", "Stage-two refit flag."),
            boolean_column("stage_two_scores_sign_aligned_per_replicate", "Stage-two alignment", "Stage-two sign-alignment flag."),
            boolean_column("complete_point_contract_validated_per_replicate", "Point contract", "Per-replicate point-contract validation flag."),
            number_column("failed_replicate_count", "Failed", "Typed failed replicate count."),
        ],
        rows: vec![CanonicalResultRow {
            id: "higher_order_bootstrap_receipt".into(),
            cells: vec![
                text(&receipt.capability_cell.capability_id),
                text(&receipt.capability_cell.cell_id),
                text(&receipt.capability_cell.capability_version),
                text(&receipt.method_version),
                text(&receipt.point_method_version),
                text(&receipt.resampling_operation_version),
                text(&receipt.resampling_stream_version),
                text(&receipt.quantile_method_version),
                text(&receipt.standard_error_method_version),
                text(&receipt.summation_method_version),
                text(&receipt.p_value_method_version),
                text(&receipt.failure_policy_version),
                text(&receipt.sign_alignment_method_version),
                text(&receipt.target_version),
                text(&receipt.general_sem_config_sha256),
                text(&receipt.compiled_plan_sha256),
                text(&receipt.hoc_stage_plan_sha256),
                text(&receipt.model_scientific_sha256),
                text(&receipt.stage_one_model_scientific_sha256),
                text(&receipt.stage_two_model_scientific_sha256),
                text(&receipt.source_dataset_fingerprint),
                text(&receipt.complete_case_frame_sha256),
                text(&receipt.usable_replicate_indices_sha256),
                text(&receipt.target_identity_set_sha256),
                text(receipt.target_ids.join(";")),
                number(receipt.confidence_level),
                number(f64::from(receipt.resamples_requested)),
                number(f64::from(receipt.resamples_usable)),
                number(f64::from(receipt.minimum_usable_resamples)),
                text(&receipt.seed),
                number(f64::from(receipt.workers)),
                boolean(receipt.complete_model_reestimated_per_replicate),
                boolean(receipt.stage_one_reestimated_per_replicate),
                boolean(receipt.generated_component_values_recalculated_per_replicate),
                boolean(receipt.stage_one_scores_sign_aligned_per_replicate),
                boolean(receipt.stage_two_reestimated_per_replicate),
                boolean(receipt.stage_two_scores_sign_aligned_per_replicate),
                boolean(receipt.complete_point_contract_validated_per_replicate),
                number(receipt.failed_replicates.len() as f64),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![bootstrap_cell.clone()]),
    })
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
    replace_joint_stage_two_structural_tables_v1(
        document,
        ordinary_rows,
        control_rows,
        capability_cell,
    )
}

fn project_canonical_joint_stage_two_structural_tables_v1(
    document: &mut CanonicalResultDocumentV2,
    plan: &qpls_core::CompiledPlsPlanV3,
    coefficients: &[CanonicalJointStageStructuralCoefficientResultV1],
    model: &SemModelV4,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), Vec<String>> {
    let relations = plan
        .topology()
        .structural_relations()
        .iter()
        .map(|relation| (relation.relation_id(), relation))
        .collect::<std::collections::BTreeMap<_, _>>();
    let technical_relation_ids = plan
        .two_way_interactions()
        .iter()
        .map(|interaction| interaction.interaction_effect_relation_id())
        .chain(
            plan.three_way_interaction()
                .iter()
                .map(|interaction| interaction.interaction_effect_relation_id()),
        )
        .collect::<std::collections::BTreeSet<_>>();
    let expected_relation_ids = relations
        .keys()
        .copied()
        .filter(|relation_id| !technical_relation_ids.contains(relation_id))
        .collect::<std::collections::BTreeSet<_>>();
    let mut observed_relation_ids = std::collections::BTreeSet::new();
    let mut ordinary_rows = Vec::new();
    let mut control_rows = Vec::new();
    for coefficient in coefficients {
        let relation = relations
            .get(coefficient.relation_id.as_str())
            .ok_or_else(|| {
                vec![format!(
                    "Final joint-stage coefficient {} is absent from the compiled topology",
                    coefficient.relation_id
                )]
            })?;
        let expected_role = match relation.role() {
            StructuralRelationRoleV4::Structural => CanonicalStructuralRelationRoleV1::Structural,
            StructuralRelationRoleV4::Control => CanonicalStructuralRelationRoleV1::Control,
        };
        if technical_relation_ids.contains(coefficient.relation_id.as_str())
            || coefficient.parameter_id != relation.parameter_id()
            || coefficient.source_id != relation.source()
            || coefficient.target_id != relation.target()
            || coefficient.role != expected_role
            || coefficient.stage != CanonicalStructuralEstimateStageV1::JointStageTwo
            || coefficient.method_version
                != GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_METHOD_VERSION_V1
            || coefficient.trace.model_id != model.id
            || coefficient.trace.capability_cell != *capability_cell
            || !coefficient.estimate.estimate.is_finite()
            || !observed_relation_ids.insert(coefficient.relation_id.as_str())
        {
            return Err(vec![format!(
                "Final joint-stage coefficient {} contradicts or duplicates its compiled three-way relation",
                coefficient.relation_id
            )]);
        }
        match relation.role() {
            StructuralRelationRoleV4::Structural => ordinary_rows.push(CanonicalResultRow {
                id: format!("joint_path_{:04}", ordinary_rows.len()),
                cells: vec![
                    text(&coefficient.relation_id),
                    text(&coefficient.parameter_id),
                    text(&coefficient.source_id),
                    text(&coefficient.target_id),
                    number(coefficient.estimate.estimate),
                ],
            }),
            StructuralRelationRoleV4::Control => control_rows.push(CanonicalResultRow {
                id: format!("joint_control_{:04}", control_rows.len()),
                cells: vec![
                    text(&coefficient.relation_id),
                    text(&coefficient.parameter_id),
                    text(&coefficient.source_id),
                    text(&coefficient.target_id),
                    text(parameter_label(model, &coefficient.parameter_id).unwrap_or("Control")),
                    number(coefficient.estimate.estimate),
                ],
            }),
        }
    }
    if observed_relation_ids != expected_relation_ids {
        return Err(vec![
            "The final three-way joint-stage ledger does not cover every compiled ordinary structural/control relation exactly once"
                .into(),
        ]);
    }
    replace_joint_stage_two_structural_tables_v1(
        document,
        ordinary_rows,
        control_rows,
        capability_cell,
    )
}

fn replace_joint_stage_two_structural_tables_v1(
    document: &mut CanonicalResultDocumentV2,
    ordinary_rows: Vec<CanonicalResultRow>,
    control_rows: Vec<CanonicalResultRow>,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<(), Vec<String>> {
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

fn moderated_mediation_stage_label(
    stage: qpls_core::CanonicalModeratedMediationStageV1,
) -> &'static str {
    match stage {
        qpls_core::CanonicalModeratedMediationStageV1::FirstStage => "first_stage",
        qpls_core::CanonicalModeratedMediationStageV1::SecondStage => "second_stage",
    }
}

fn moderated_mediation_inference_cells(
    estimate: &CanonicalGeneralSemEstimateV1,
) -> Result<Vec<CanonicalResultCell>, Vec<String>> {
    let complete = [
        estimate.bootstrap_mean,
        estimate.bootstrap_bias,
        estimate.standard_error,
        estimate.lower,
        estimate.upper,
        estimate.p_value,
    ];
    if complete.iter().any(Option::is_none)
        || estimate.bootstrap_usable_replicates.is_none()
        || estimate.bootstrap_two_sided_exceedances.is_none()
    {
        return Err(vec![
            "Moderated-mediation five-target rows require complete bootstrap inference".into(),
        ]);
    }
    Ok(vec![
        number(estimate.estimate),
        number(estimate.bootstrap_mean.unwrap()),
        number(estimate.bootstrap_bias.unwrap()),
        number(estimate.standard_error.unwrap()),
        number(estimate.lower.unwrap()),
        number(estimate.upper.unwrap()),
        number(estimate.p_value.unwrap()),
        number(f64::from(estimate.bootstrap_usable_replicates.unwrap())),
        number(f64::from(estimate.bootstrap_two_sided_exceedances.unwrap())),
    ])
}

fn moderated_mediation_inference_columns() -> Vec<CanonicalResultColumn> {
    vec![
        number_column("estimate", "Estimate", "Point estimate."),
        number_column(
            "bootstrap_mean",
            "Bootstrap mean",
            "Mean across usable replicates.",
        ),
        number_column(
            "bootstrap_bias",
            "Bias",
            "Bootstrap mean minus point estimate.",
        ),
        number_column(
            "standard_error",
            "Standard error",
            "Sample standard error across usable replicates.",
        ),
        number_column("lower", "Lower", "Lower type-7 percentile bound."),
        number_column("upper", "Upper", "Upper type-7 percentile bound."),
        number_column(
            "p_value",
            "P value",
            "Two-sided null-centered plus-one probability.",
        ),
        number_column(
            "bootstrap_usable_replicates",
            "Usable",
            "Shared usable replicate count.",
        ),
        number_column(
            "bootstrap_two_sided_exceedances",
            "Exceedances",
            "Shared two-sided exceedance count.",
        ),
    ]
}

fn moderated_mediation_bootstrap_tables(
    results: &CanonicalGeneralSemResultsV1,
    bootstrap: &qpls_resampling::GeneralSemPlsTwoWayModeratedMediationBootstrapResultV1,
    execution: &RecipeV4GeneralSemPlsExecutionResultV1,
    capability_cell: &CapabilityCellReferenceV2,
) -> Result<Vec<CanonicalResultTable>, Vec<String>> {
    bootstrap.ensure_valid().map_err(|error| {
        vec![format!(
            "General SEM moderated-mediation bootstrap result is invalid: {error}"
        )]
    })?;
    let expected_cell = pls_general_two_way_moderated_mediation_bootstrap_capability_cell_v1();
    let point_cell = general_sem_multiple_moderation_point_capability_cell_v1();
    let base_cell = bootstrap.base_pls_capability_cell.clone();
    let receipt = results.inference_receipt.as_ref().ok_or_else(|| {
        vec!["General SEM moderated-mediation bootstrap omitted its typed receipt".into()]
    })?;
    let mut expected_dependencies = vec![base_cell.clone(), point_cell.clone()];
    sort_and_deduplicate_cells(&mut expected_dependencies);
    if capability_cell != &expected_cell
        || receipt.capability_cell != expected_cell
        || receipt.capability_dependencies != expected_dependencies
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
        || receipt.effect_ids != bootstrap.five_target_ids
        || receipt.resamples_requested != bootstrap.resamples_requested
        || receipt.resamples_usable != bootstrap.resamples_usable
        || receipt.minimum_usable_resamples != bootstrap.minimum_usable_resamples
        || receipt.seed != bootstrap.seed
        || receipt.workers != bootstrap.workers
        || receipt.complete_model_reestimated_per_replicate
            != bootstrap.complete_model_reestimated_per_replicate
        || !bootstrap.all_five_targets_share_one_replicate_ledger
        || execution.general_sem_config_sha256() != bootstrap.general_sem_config_sha256
        || execution.compiled_plan_sha256() != bootstrap.compiled_plan_sha256
        || execution.model_scientific_sha256() != bootstrap.model_scientific_sha256
        || execution.stage_one_model_scientific_sha256()
            != bootstrap.stage_one_model_scientific_sha256
        || execution.source_dataset_fingerprint() != bootstrap.source_dataset_fingerprint
    {
        return Err(vec![
            "General SEM moderated-mediation raw/canonical receipt or three-cell execution provenance has drifted"
                .into(),
        ]);
    }
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
    if !failed_replicates_match {
        return Err(vec![
            "General SEM moderated-mediation failure ledger differs from its raw bootstrap ledger"
                .into(),
        ]);
    }
    let [gamma] = results.interaction_effects.as_slice() else {
        return Err(vec![
            "Moderated-mediation canonical results require exactly one scientific gamma".into(),
        ]);
    };
    let raw_gamma = &bootstrap.interaction_gamma;
    if gamma.effect_id != raw_gamma.target.target_id
        || gamma.interaction_id != raw_gamma.target.interaction_id
        || gamma.scientific_rescaled_gamma.estimate.to_bits() != raw_gamma.original.to_bits()
        || gamma
            .scientific_rescaled_gamma
            .bootstrap_mean
            .map(f64::to_bits)
            != Some(raw_gamma.bootstrap_mean.to_bits())
        || gamma
            .scientific_rescaled_gamma
            .bootstrap_bias
            .map(f64::to_bits)
            != Some(raw_gamma.bootstrap_bias.to_bits())
        || gamma
            .scientific_rescaled_gamma
            .standard_error
            .map(f64::to_bits)
            != Some(raw_gamma.standard_error.to_bits())
        || gamma.scientific_rescaled_gamma.lower.map(f64::to_bits)
            != Some(raw_gamma.lower.to_bits())
        || gamma.scientific_rescaled_gamma.upper.map(f64::to_bits)
            != Some(raw_gamma.upper.to_bits())
        || gamma.scientific_rescaled_gamma.p_value.map(f64::to_bits)
            != Some(raw_gamma.p_value_two_sided.to_bits())
    {
        return Err(vec![
            "Moderated-mediation scientific gamma differs from its shared five-target ledger"
                .into(),
        ]);
    }

    let mut gamma_cells = vec![
        text(&gamma.effect_id),
        text(&gamma.interaction_id),
        text(&gamma.focal_relation_id),
        text(&gamma.interaction_effect_relation_id),
        text(&gamma.interaction_effect_parameter_id),
        text(&gamma.generated_product_column_id),
        text(&gamma.focal_predictor_id),
        text(&gamma.moderator_id),
        text(&gamma.outcome_id),
        text(&gamma.stage_one_model_scientific_sha256),
        text(&gamma.product_scale_version),
        text(&gamma.method_version),
    ];
    gamma_cells.extend(moderated_mediation_inference_cells(
        &gamma.scientific_rescaled_gamma,
    )?);
    let mut gamma_columns = vec![
        text_column(
            "effect_id",
            "Effect ID",
            "Canonical scientific gamma identity.",
        ),
        text_column(
            "interaction_id",
            "Interaction",
            "Authored interaction identity.",
        ),
        text_column(
            "focal_relation_id",
            "Focal path",
            "Moderated relation identity.",
        ),
        text_column(
            "interaction_effect_relation_id",
            "Effect relation",
            "Interaction-effect relation identity.",
        ),
        text_column(
            "interaction_effect_parameter_id",
            "Effect parameter",
            "Interaction-effect parameter identity.",
        ),
        text_column(
            "generated_product_column_id",
            "Product column",
            "Generated product-column identity.",
        ),
        text_column(
            "focal_predictor_id",
            "Focal predictor",
            "Focal predictor identity.",
        ),
        text_column("moderator_id", "Moderator", "Moderator identity."),
        text_column("outcome_id", "Outcome", "Moderated-stage outcome identity."),
        text_column(
            "stage_one_model_scientific_sha256",
            "Stage-one digest",
            "Shared stage-one scientific digest.",
        ),
        text_column(
            "product_scale_version",
            "Product scale",
            "Scientific gamma scaling policy.",
        ),
        text_column(
            "point_method_version",
            "Point method",
            "Exact interaction point method.",
        ),
    ];
    gamma_columns.extend(moderated_mediation_inference_columns());
    let gamma_table = CanonicalResultTable {
        id: GENERAL_SEM_MODERATED_MEDIATION_GAMMA_TABLE_ID_V1.into(),
        title: "Scientific gamma inference".into(),
        description: Some("The interaction gamma from the same usable replicate ledger as all conditional-process targets.".into()),
        columns: gamma_columns,
        rows: vec![CanonicalResultRow {
            id: "moderated_mediation_gamma_inference_0000".into(),
            cells: gamma_cells,
        }],
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    };

    let mut conditional_rows = Vec::with_capacity(3);
    for (index, effect) in results.conditional_indirect_effects.iter().enumerate() {
        let raw = bootstrap
            .derived_effects
            .iter()
            .find(|candidate| candidate.effect_id() == effect.effect_id)
            .ok_or_else(|| {
                vec![format!(
                    "Conditional indirect {} is absent from the raw shared ledger",
                    effect.effect_id
                )]
            })?;
        if effect.value.estimate.to_bits() != raw.original.to_bits() {
            return Err(vec![format!(
                "Conditional indirect {} differs from its raw point identity",
                effect.effect_id
            )]);
        }
        let ordered_relation_ids = serde_json::to_string(&effect.ordered_relation_ids)
            .map_err(|error| vec![format!("ordered relation serialization failed: {error}")])?;
        let mut cells = vec![
            text(&effect.effect_id),
            text(&effect.target_id),
            text(&effect.estimand_id),
            text(moderated_mediation_stage_label(effect.moderated_stage)),
            text(&effect.interaction_id),
            text(&effect.x_id),
            text(&effect.mediator_id),
            text(&effect.y_id),
            text(&effect.moderator_id),
            text(&ordered_relation_ids),
            number(f64::from(effect.probe_value_index)),
            number(effect.moderator_value),
        ];
        cells.extend(moderated_mediation_inference_cells(&effect.value)?);
        conditional_rows.push(CanonicalResultRow {
            id: format!("conditional_indirect_{index:04}"),
            cells,
        });
    }
    let mut conditional_columns = vec![
        text_column(
            "effect_id",
            "Effect ID",
            "Conditional-indirect effect identity.",
        ),
        text_column(
            "target_id",
            "Target ID",
            "Compiled conditional-process target.",
        ),
        text_column(
            "estimand_id",
            "Estimand",
            "Selected SpecificPath request identity.",
        ),
        text_column(
            "moderated_stage",
            "Stage",
            "First- or second-stage moderation.",
        ),
        text_column(
            "interaction_id",
            "Interaction",
            "Authored interaction identity.",
        ),
        text_column("x_id", "X", "Path origin."),
        text_column("mediator_id", "M", "Selected mediator."),
        text_column("y_id", "Y", "Path outcome."),
        text_column("moderator_id", "W", "Moderator."),
        text_column(
            "ordered_relation_ids",
            "Path relations",
            "Ordered stable relation identities as canonical JSON.",
        ),
        number_column("probe_value_index", "Probe index", "Frozen probe index."),
        number_column(
            "moderator_value",
            "Moderator value",
            "Frozen standardized moderator value.",
        ),
    ];
    conditional_columns.extend(moderated_mediation_inference_columns());
    let conditional_table = CanonicalResultTable {
        id: GENERAL_SEM_CONDITIONAL_INDIRECT_TABLE_ID_V1.into(),
        title: "Conditional indirect effects".into(),
        description: Some("Indirect effects at standardized moderator values -1, 0, and +1, from one full-model case-bootstrap ledger.".into()),
        columns: conditional_columns,
        rows: conditional_rows,
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    };

    let [index_effect] = results.moderated_mediation_indices.as_slice() else {
        return Err(vec![
            "Moderated-mediation canonical results require exactly one index".into(),
        ]);
    };
    let raw_index = bootstrap
        .derived_effects
        .iter()
        .find(|candidate| candidate.effect_id() == index_effect.effect_id)
        .ok_or_else(|| {
            vec!["The moderated-mediation index is absent from the raw ledger".into()]
        })?;
    if index_effect.value.estimate.to_bits() != raw_index.original.to_bits() {
        return Err(vec![
            "The moderated-mediation index differs from its raw point identity".into(),
        ]);
    }
    let ordered_relation_ids = serde_json::to_string(&index_effect.ordered_relation_ids)
        .map_err(|error| vec![format!("ordered relation serialization failed: {error}")])?;
    let mut index_cells = vec![
        text(&index_effect.effect_id),
        text(&index_effect.target_id),
        text(&index_effect.estimand_id),
        text(moderated_mediation_stage_label(
            index_effect.moderated_stage,
        )),
        text(&index_effect.interaction_id),
        text(&index_effect.x_id),
        text(&index_effect.mediator_id),
        text(&index_effect.y_id),
        text(&index_effect.moderator_id),
        text(&ordered_relation_ids),
    ];
    index_cells.extend(moderated_mediation_inference_cells(&index_effect.value)?);
    let mut index_columns = vec![
        text_column("effect_id", "Effect ID", "Index effect identity."),
        text_column(
            "target_id",
            "Target ID",
            "Compiled conditional-process target.",
        ),
        text_column(
            "estimand_id",
            "Estimand",
            "Selected SpecificPath request identity.",
        ),
        text_column(
            "moderated_stage",
            "Stage",
            "First- or second-stage moderation.",
        ),
        text_column(
            "interaction_id",
            "Interaction",
            "Authored interaction identity.",
        ),
        text_column("x_id", "X", "Path origin."),
        text_column("mediator_id", "M", "Selected mediator."),
        text_column("y_id", "Y", "Path outcome."),
        text_column("moderator_id", "W", "Moderator."),
        text_column(
            "ordered_relation_ids",
            "Path relations",
            "Ordered stable relation identities as canonical JSON.",
        ),
    ];
    index_columns.extend(moderated_mediation_inference_columns());
    let index_table = CanonicalResultTable {
        id: GENERAL_SEM_MODERATED_MEDIATION_INDEX_TABLE_ID_V1.into(),
        title: "Index of moderated mediation".into(),
        description: Some(
            "Scientific gamma multiplied by the coefficient of the unmoderated stage.".into(),
        ),
        columns: index_columns,
        rows: vec![CanonicalResultRow {
            id: "moderated_mediation_index_0000".into(),
            cells: index_cells,
        }],
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    };

    let target = &bootstrap.compiled_target;
    let five_target_ids = serde_json::to_string(&bootstrap.five_target_ids)
        .map_err(|error| vec![format!("five-target serialization failed: {error}")])?;
    let receipt_columns = vec![
        text_column(
            "capability_id",
            "Capability",
            "Supplemental capability identity.",
        ),
        text_column("cell_id", "Cell", "Supplemental cell identity."),
        text_column(
            "capability_version",
            "Capability version",
            "Supplemental capability version.",
        ),
        text_column(
            "base_capability_id",
            "Base capability",
            "Base PLS dependency.",
        ),
        text_column("base_cell_id", "Base cell", "Base PLS cell dependency."),
        text_column(
            "base_capability_version",
            "Base version",
            "Base PLS dependency version.",
        ),
        text_column(
            "moderation_capability_id",
            "Moderation capability",
            "Moderation-point dependency.",
        ),
        text_column(
            "moderation_cell_id",
            "Moderation cell",
            "Moderation-point cell dependency.",
        ),
        text_column(
            "moderation_capability_version",
            "Moderation version",
            "Moderation-point dependency version.",
        ),
        text_column("method_version", "Method", "Combined bootstrap method."),
        text_column(
            "point_method_version",
            "Point method",
            "Joint point method.",
        ),
        text_column(
            "resampling_operation_version",
            "Operation",
            "Case-resampling operation.",
        ),
        text_column(
            "resampling_stream_version",
            "Stream",
            "Indexed resampling stream.",
        ),
        text_column(
            "quantile_method_version",
            "Quantile",
            "Interval quantile method.",
        ),
        text_column(
            "standard_error_method_version",
            "Standard error",
            "Standard-error method.",
        ),
        text_column(
            "summation_method_version",
            "Summation",
            "Stable summation method.",
        ),
        text_column(
            "p_value_method_version",
            "P value",
            "Bootstrap p-value method.",
        ),
        text_column(
            "failure_policy_version",
            "Failure policy",
            "Minimum usable policy.",
        ),
        text_column(
            "sign_alignment_method_version",
            "Sign alignment",
            "Score sign-alignment method.",
        ),
        text_column(
            "product_scale_version",
            "Product scale",
            "Per-replicate product scaling.",
        ),
        text_column(
            "gamma_target_version",
            "Gamma target",
            "Scientific gamma target schema.",
        ),
        text_column(
            "target_contract_version",
            "Target contract",
            "Compiled conditional-process target contract.",
        ),
        text_column(
            "target_id",
            "Target ID",
            "Compiled conditional-process target identity.",
        ),
        text_column(
            "compiled_target_sha256",
            "Target digest",
            "Compiled target digest.",
        ),
        text_column(
            "probe_policy_version",
            "Probe policy",
            "Frozen standardized probe policy.",
        ),
        text_column(
            "conditional_target_version",
            "Conditional target",
            "Conditional-indirect target schema.",
        ),
        text_column(
            "index_target_version",
            "Index target",
            "Moderated-mediation index target schema.",
        ),
        text_column(
            "compilation_artifact_identity_sha256",
            "Artifact digest",
            "Compiled artifact identity digest.",
        ),
        text_column(
            "compiled_plan_sha256",
            "Plan digest",
            "Compiled plan digest.",
        ),
        text_column(
            "general_sem_config_sha256",
            "Config digest",
            "General SEM config digest.",
        ),
        text_column(
            "recipe_analytical_sha256",
            "Recipe digest",
            "Analytical recipe digest.",
        ),
        text_column(
            "model_scientific_sha256",
            "Model digest",
            "Full model scientific digest.",
        ),
        text_column(
            "stage_one_model_scientific_sha256",
            "Stage-one digest",
            "Shared stage-one projection digest.",
        ),
        text_column(
            "source_dataset_fingerprint",
            "Dataset",
            "Source dataset fingerprint.",
        ),
        text_column(
            "complete_case_frame_sha256",
            "Complete-case digest",
            "Complete-case frame digest.",
        ),
        text_column(
            "usable_replicate_indices_sha256",
            "Usable-index digest",
            "Usable replicate ledger digest.",
        ),
        text_column(
            "five_target_identity_set_sha256",
            "Raw identity digest",
            "Raw five-target identity digest.",
        ),
        text_column(
            "canonical_effect_identity_set_sha256",
            "Canonical identity digest",
            "Canonical five-target identity digest.",
        ),
        text_column(
            "five_target_ids",
            "Five targets",
            "Ordered five-target identities as canonical JSON.",
        ),
        text_column("interval", "Interval", "Canonical interval family."),
        text_column("tail", "Tail", "Canonical hypothesis tail."),
        number_column("confidence_level", "Confidence", "Confidence level."),
        number_column("resamples_requested", "Requested", "Requested replicates."),
        number_column("resamples_usable", "Usable", "Usable replicates."),
        number_column(
            "minimum_usable_resamples",
            "Minimum usable",
            "Required usable replicates.",
        ),
        text_column("seed", "Seed", "Exact decimal resampling seed."),
        number_column("workers", "Workers", "Configured worker count."),
        boolean_column(
            "complete_model_reestimated_per_replicate",
            "Complete model",
            "Complete model re-estimated per replicate.",
        ),
        boolean_column(
            "shared_stage_one_reestimated_per_replicate",
            "Stage one",
            "Shared stage one re-estimated per replicate.",
        ),
        boolean_column(
            "score_vectors_sign_aligned_before_products",
            "Sign aligned",
            "Scores sign aligned before product regeneration.",
        ),
        boolean_column(
            "product_scaling_recomputed_per_replicate",
            "Product scaling",
            "Product scaling recomputed per replicate.",
        ),
        boolean_column(
            "joint_stage_two_reestimated_per_replicate",
            "Joint stage two",
            "Joint stage two re-estimated per replicate.",
        ),
        boolean_column(
            "complete_joint_point_contract_validated_per_replicate",
            "Point contract",
            "Complete point contract validated per replicate.",
        ),
        boolean_column(
            "all_five_targets_share_one_replicate_ledger",
            "Shared ledger",
            "All five targets share one usable/failure ledger.",
        ),
        number_column(
            "failed_replicate_count",
            "Failed",
            "Typed failed replicate count.",
        ),
    ];
    let receipt_cells = vec![
        text(&receipt.capability_cell.capability_id),
        text(&receipt.capability_cell.cell_id),
        text(&receipt.capability_cell.capability_version),
        text(&base_cell.capability_id),
        text(&base_cell.cell_id),
        text(&base_cell.capability_version),
        text(&point_cell.capability_id),
        text(&point_cell.cell_id),
        text(&point_cell.capability_version),
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
        text(target.contract_version()),
        text(target.target_id()),
        text(&bootstrap.compiled_target_sha256),
        text(target.probe_policy_version()),
        text(target.conditional_target_version()),
        text(target.index_target_version()),
        text(&receipt.compilation_artifact_identity_sha256),
        text(&receipt.compiled_plan_sha256),
        text(&receipt.general_sem_config_sha256),
        text(&receipt.recipe_analytical_sha256),
        text(&receipt.model_scientific_sha256),
        text(&bootstrap.stage_one_model_scientific_sha256),
        text(&receipt.source_dataset_fingerprint),
        text(&receipt.complete_case_frame_sha256),
        text(&receipt.usable_replicate_indices_sha256),
        text(&bootstrap.five_target_identity_set_sha256),
        text(&receipt.effect_identity_set_sha256),
        text(&five_target_ids),
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
        boolean(bootstrap.all_five_targets_share_one_replicate_ledger),
        number(bootstrap.failed_replicates.len() as f64),
    ];
    if receipt_columns.len() != receipt_cells.len() {
        return Err(vec![
            "Moderated-mediation receipt table has an internal column-width mismatch".into(),
        ]);
    }
    let receipt_table = CanonicalResultTable {
        id: GENERAL_SEM_MODERATED_MEDIATION_RECEIPT_TABLE_ID_V1.into(),
        title: "Moderated-mediation bootstrap pipeline receipt".into(),
        description: Some("Exact three-cell ownership, compiled target, provenance, full-refit pipeline, and shared usable/failure ledger receipt.".into()),
        columns: receipt_columns,
        rows: vec![CanonicalResultRow {
            id: "moderated_mediation_bootstrap_receipt".into(),
            cells: receipt_cells,
        }],
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell.clone()]),
    };
    Ok(vec![
        gamma_table,
        conditional_table,
        index_table,
        receipt_table,
    ])
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

fn optional_text(value: Option<&str>) -> CanonicalResultCell {
    value.map_or(
        CanonicalResultCell::Missing {
            reason: CanonicalMissingReason::NotApplicable,
            display: None,
        },
        text,
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

    #[cfg(windows)]
    fn strict_v3_three_way_source_fixture(
    ) -> (qpls_data::Dataset, AnalysisRecipeV4, SemModelV4, [String; 4]) {
        use chrono::{TimeZone, Utc};
        use qpls_core::{
            ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe,
            AnalysisRecipeModelBindingV4, AnalysisSettings, Construct,
            LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig, ModelSpec,
            SemDataBindingV4, SemVariableV4, StructuralPath,
            confirm_legacy_recipe_estimand_v4, migrate_analysis_recipe_to_v4_pending,
        };
        use qpls_data::{ImportOptions, import_delimited_bytes};
        use std::collections::BTreeMap;

        let source_model = ModelSpec {
            id: Uuid::from_u128(0x3254_0000_0000_0000_0000_0000_0000_0001),
            name: "Strict V3 authored-ID moderation source".into(),
            constructs: [
                ("authored:x", "X", ["x1", "x2"]),
                ("authored:w", "W", ["w1", "w2"]),
                ("authored:z", "Z", ["z1", "z2"]),
                ("authored:y", "Y", ["y1", "y2"]),
            ]
            .into_iter()
            .map(|(id, label, indicators)| Construct {
                id: id.into(),
                name: label.into(),
                short_name: label.into(),
                mode: MeasurementMode::Reflective,
                indicators: indicators.into_iter().map(str::to_owned).collect(),
            })
            .collect(),
            // Only the focal effect is authored. Strict hierarchy must create
            // both moderator main-effect relations through the V3 revision
            // allocator, which is where colon-bearing authored IDs previously
            // produced overlong generated schema-6 identities.
            paths: [("authored:x", "authored:y")]
            .into_iter()
            .map(|(source, target)| StructuralPath {
                source: source.into(),
                target: target.into(),
            })
            .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut csv = String::from("x1,x2,w1,w2,z1,z2,y1,y2\n");
        for row in 0..81 {
            let row = f64::from(row);
            let x = (row - 40.0) / 13.0;
            let w = (row * 0.71).sin() + 0.2 * (row * 0.13).cos();
            let z = (row * 0.37).cos() - 0.3 * (row * 0.19).sin();
            let y = 0.25 * x + 0.20 * w - 0.15 * z + 0.65 * x * w
                - 0.30 * x * z
                + 0.20 * w * z
                + 0.45 * x * w * z;
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                x + 0.03 * (row * 0.11).sin(),
                1.02 * x + 0.02 * (row * 0.17).cos(),
                w + 0.03 * (row * 0.23).cos(),
                0.98 * w + 0.02 * (row * 0.29).sin(),
                z + 0.03 * (row * 0.31).sin(),
                1.01 * z + 0.02 * (row * 0.41).cos(),
                y + 0.03 * (row * 0.43).sin(),
                1.01 * y + 0.02 * (row * 0.47).cos(),
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "strict-v3-three-way-authored-ids.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x3254_0000_0000_0000_0000_0000_0000_0002),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: source_model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                workers: 1,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source_model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!("the fixture is raw-data PLS")
        };
        *dataset_id = dataset.id.to_string();
        let authored_id = |label: &str| {
            model
                .variables
                .iter()
                .find_map(|variable| match variable {
                    SemVariableV4::Composite {
                        id,
                        label: actual,
                        ..
                    } if actual == label => Some(id.clone()),
                    _ => None,
                })
                .unwrap()
        };
        let ids = [
            authored_id("X"),
            authored_id("W"),
            authored_id("Z"),
            authored_id("Y"),
        ];
        assert!(ids.iter().all(|id| id.matches(':').count() >= 2));
        recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        recipe.metadata.insert(
            "execution_surface".into(),
            qpls_project::GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1.into(),
        );
        recipe
            .metadata
            .insert("general_sem_generation".into(), "general_sem_v1".into());
        recipe.ensure_valid().unwrap();
        model.ensure_valid().unwrap();
        (dataset, recipe, model, ids)
    }

    #[cfg(windows)]
    fn strict_structural_relation_id(
        model: &SemModelV4,
        source: &str,
        target: &str,
    ) -> String {
        model
            .relations
            .iter()
            .find_map(|relation| match relation {
                qpls_core::SemRelationV4::Structural {
                    id,
                    source: actual_source,
                    target: actual_target,
                    role: qpls_core::StructuralRelationRoleV4::Structural,
                    ..
                } if actual_source == source && actual_target == target => Some(id.clone()),
                _ => None,
            })
            .unwrap()
    }

    #[cfg(windows)]
    #[test]
    fn strict_v3_colon_ids_execute_build_append_and_reopen_three_way_canonical_result() {
        use chrono::{TimeZone, Utc};
        use qpls_core::{
            SemDerivedTermV4, sha256_serialized,
        };
        use qpls_project::{
            GeneralSemExecutionAuthorityRevisionIdentityV1,
            GeneralSemExecutionAuthorityRevisionIntentV1,
            GeneralSemExecutionAuthorityRevisionRequestV1,
            GeneralSemExecutionAuthoritySourcePinV1, GeneralSemModeratingEffectTargetV1,
            GeneralSemRevisionGenerationV1, GeneralSemRevisionHierarchyPolicyV1,
            GeneralSemRevisionInteractionMethodV1, ProjectModelPayloadV6,
            append_canonical_result_document_v2_file_v6,
            create_general_sem_execution_authority_revision_v1,
            create_populated_general_sem_project_archive_v6, load_project_archive_v6,
        };

        let assert_canonical_generated_id = |id: &str, prefix: &str| {
            assert!(id.starts_with(prefix));
            assert!(!id.contains('%'));
            assert!(id.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'_' | b'.' | b':' | b'-')
            }));
        };

        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("strict-v3-source.qpls");
        let two_way_path = directory.path().join("strict-v3-two-way.qpls");
        let three_way_path = directory.path().join("strict-v3-three-way.qpls");
        let (dataset, recipe, model, [x_id, w_id, z_id, y_id]) =
            strict_v3_three_way_source_fixture();
        let source_project_id = Uuid::from_u128(0x3254_0000_0000_0000_0000_0000_0000_0010);
        let source_model_document_sha256 = model.model_document_sha256().unwrap();
        let source_model_scientific_sha256 = model.scientific_sha256().unwrap();
        let source_recipe_document_sha256 = sha256_serialized(&recipe);
        let focal_relation = strict_structural_relation_id(&model, &x_id, &y_id);
        let source_receipt = create_populated_general_sem_project_archive_v6(
            &source_path,
            source_project_id,
            "Strict V3 source",
            Utc.with_ymd_and_hms(2026, 8, 22, 8, 0, 0).unwrap(),
            &dataset,
            model.clone(),
            recipe.clone(),
        )
        .unwrap();
        let two_way_project_id =
            Uuid::from_u128(0x3254_0000_0000_0000_0000_0000_0000_0020);
        let two_way_receipt = create_general_sem_execution_authority_revision_v1(
            &source_path,
            &source_receipt.destination_archive_sha256,
            &two_way_path,
            GeneralSemExecutionAuthorityRevisionRequestV1 {
                source: GeneralSemExecutionAuthoritySourcePinV1 {
                    project_id: source_project_id,
                    model_id: model.id.clone(),
                    model_document_sha256: source_model_document_sha256,
                    model_scientific_sha256: source_model_scientific_sha256,
                    recipe_id: recipe.id,
                    recipe_document_sha256: source_recipe_document_sha256,
                },
                revision: GeneralSemExecutionAuthorityRevisionIdentityV1 {
                    project_id: two_way_project_id,
                    project_name: "Strict V3 two-way revision".into(),
                    created_at: Utc.with_ymd_and_hms(2026, 8, 22, 8, 1, 0).unwrap(),
                    model_id: "model:strict-v3:two-way".into(),
                    model_name: "Strict V3 two-way revision".into(),
                    recipe_id: Uuid::from_u128(
                        0x3254_0000_0000_0000_0000_0000_0000_0021,
                    ),
                },
                intent: GeneralSemExecutionAuthorityRevisionIntentV1::AddModeratingEffectV3 {
                    intent_version: 3,
                    sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                    label: "X × W".into(),
                    operands: vec![x_id.clone(), w_id.clone()],
                    target: GeneralSemModeratingEffectTargetV1::FocalRelation {
                        relation_id: focal_relation,
                    },
                    outcome: y_id.clone(),
                    method: GeneralSemRevisionInteractionMethodV1::TwoStage,
                    hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1::Strong,
                },
                expected_capability_cell:
                    qpls_core::pls_general_multiple_moderation_point_capability_cell_v1(),
                recipe_execution_surface:
                    qpls_project::GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1.into(),
            },
        )
        .unwrap();
        assert_canonical_generated_id(
            &two_way_receipt.interaction_term_id,
            "general_sem_v1_moderation_term_",
        );

        let three_way_project_id =
            Uuid::from_u128(0x3254_0000_0000_0000_0000_0000_0000_0030);
        let three_way_receipt = create_general_sem_execution_authority_revision_v1(
            &two_way_path,
            &two_way_receipt.destination_archive_sha256,
            &three_way_path,
            GeneralSemExecutionAuthorityRevisionRequestV1 {
                source: GeneralSemExecutionAuthoritySourcePinV1 {
                    project_id: two_way_project_id,
                    model_id: two_way_receipt.resident_model_id.clone(),
                    model_document_sha256: two_way_receipt
                        .resident_model_document_sha256
                        .clone(),
                    model_scientific_sha256: two_way_receipt
                        .resident_model_scientific_sha256
                        .clone(),
                    recipe_id: two_way_receipt.resident_recipe_id,
                    recipe_document_sha256: two_way_receipt
                        .resident_recipe_document_sha256
                        .clone(),
                },
                revision: GeneralSemExecutionAuthorityRevisionIdentityV1 {
                    project_id: three_way_project_id,
                    project_name: "Strict V3 three-way revision".into(),
                    created_at: Utc.with_ymd_and_hms(2026, 8, 22, 8, 2, 0).unwrap(),
                    model_id: "model:strict-v3:three-way".into(),
                    model_name: "Strict V3 three-way revision".into(),
                    recipe_id: Uuid::from_u128(
                        0x3254_0000_0000_0000_0000_0000_0000_0031,
                    ),
                },
                intent: GeneralSemExecutionAuthorityRevisionIntentV1::AddModeratingEffectV3 {
                    intent_version: 3,
                    sem_generation: GeneralSemRevisionGenerationV1::GeneralSemV1,
                    label: "X × W × Z".into(),
                    operands: vec![x_id.clone(), w_id.clone(), z_id.clone()],
                    target: GeneralSemModeratingEffectTargetV1::ParentInteraction {
                        interaction_term_id: two_way_receipt.interaction_term_id.clone(),
                    },
                    outcome: y_id.clone(),
                    method: GeneralSemRevisionInteractionMethodV1::TwoStage,
                    hierarchy_policy: GeneralSemRevisionHierarchyPolicyV1::Strong,
                },
                expected_capability_cell:
                    pls_general_three_way_moderation_point_capability_cell_v1(),
                recipe_execution_surface:
                    qpls_project::GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1.into(),
            },
        )
        .unwrap();
        assert_canonical_generated_id(
            &three_way_receipt.interaction_term_id,
            "general_sem_v1_moderation_term_",
        );
        assert_eq!(three_way_receipt.revision_number, 2);

        let loaded = load_project_archive_v6(&three_way_path).unwrap();
        let resident_model = match &loaded.document.models[0].payload {
            ProjectModelPayloadV6::SemModelV4 { model, .. } => model,
            _ => panic!("the strict revision must retain one SemModelV4 authority"),
        };
        let resident_recipe = &loaded.document.recipes[0];
        for moderator in [&w_id, &z_id] {
            let (relation_id, parameter_id) = resident_model
                .relations
                .iter()
                .find_map(|relation| match relation {
                    qpls_core::SemRelationV4::Structural {
                        id,
                        source,
                        target,
                        parameter,
                        role: qpls_core::StructuralRelationRoleV4::Structural,
                        ..
                    } if source == moderator && target == &y_id => {
                        Some((id.as_str(), parameter.as_str()))
                    }
                    _ => None,
                })
                .unwrap();
            assert_canonical_generated_id(
                relation_id,
                "general_sem_v1_moderation_main_relation_",
            );
            assert_canonical_generated_id(
                parameter_id,
                "general_sem_v1_moderation_parameter_",
            );
        }
        assert_eq!(resident_model.derived_terms.len(), 4);
        assert!(resident_model.derived_terms.iter().all(|term| matches!(
            term,
            SemDerivedTermV4::InteractionV2 {
                hierarchy_policy: qpls_core::InteractionHierarchyPolicyV2::Strong,
                ..
            }
        )));
        let artifact = compile_general_sem_pls_recipe_v1(resident_recipe, Some(resident_model))
            .unwrap();
        assert_eq!(
            artifact.capability_cell(),
            &pls_general_three_way_moderation_point_capability_cell_v1()
        );
        let analytical = run_compiled_general_sem_pls_recipe_v1(
            &loaded.datasets[0],
            resident_recipe,
            resident_model,
            &artifact,
            || false,
            |_| {},
        )
        .unwrap();
        assert!(analytical.three_way_point_estimation().is_some());
        assert!(analytical.three_way_bootstrap_inference().is_none());

        let canonical = build_recipe_v4_general_sem_pls_canonical_result_v1(
            Uuid::from_u128(0x3254_0000_0000_0000_0000_0000_0000_0040),
            three_way_project_id,
            loaded.datasets[0].id,
            "2026-08-22T08:03:00.000Z",
            "2026-08-22T08:03:01.000Z",
            resident_recipe,
            resident_model,
            &analytical,
        )
        .unwrap();
        assert_eq!(
            canonical.provenance.capability_cell,
            pls_general_three_way_moderation_point_capability_cell_v1()
        );
        let results = canonical.general_sem_results.as_ref().unwrap();
        assert_eq!(results.three_way_interaction_effects.len(), 1);
        assert_eq!(
            results.three_way_interaction_effects[0].interaction_id,
            three_way_receipt.interaction_term_id
        );
        assert_canonical_generated_id(
            &results.three_way_interaction_effects[0].interaction_id,
            "general_sem_v1_moderation_term_",
        );
        assert!(!results.three_way_conditional_interaction_effects.is_empty());
        assert!(!results.three_way_simple_slopes.is_empty());
        assert!(results.three_way_moderation_bootstrap_receipt.is_none());

        let archive_canonical: qpls_project::CanonicalResultDocumentV2 =
            serde_json::from_value(serde_json::to_value(&canonical).unwrap()).unwrap();
        archive_canonical.ensure_valid().unwrap();
        let append = append_canonical_result_document_v2_file_v6(
            &three_way_path,
            &three_way_receipt.destination_archive_sha256,
            archive_canonical,
        )
        .unwrap();
        assert_eq!(append.canonical_result_document_count, 1);
        let reopened = load_project_archive_v6(&three_way_path).unwrap();
        assert_eq!(reopened.document.canonical_result_documents.len(), 1);
        let reopened_canonical = reopened.document.canonical_result_documents[0]
            .canonical_document();
        reopened_canonical.ensure_valid().unwrap();
        validate_archived_general_sem_pls_method_identity_v1(reopened_canonical).unwrap();
    }
}
