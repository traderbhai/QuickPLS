//! Canonical projection for the archive-bound General SEM PLS Labs slice.

use crate::{
    InternalRecipeV4ExecutionSurfaceV1, InternalRecipeV4PlsExecutionRequestV1,
    InternalRecipeV4ResidentDataV1,
    recipe_v4_canonical_result::build_recipe_v4_pls_canonical_result,
};
use qpls_core::{
    AnalysisRecipeV4, CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION, CanonicalAggregateEffectKindV1,
    CanonicalColumnRole, CanonicalColumnType, CanonicalGeneralSemEstimateV1,
    CanonicalGeneralSemResultsV1, CanonicalMissingReason, CanonicalResultCell,
    CanonicalResultColumn, CanonicalResultDocumentV2, CanonicalResultExclusion, CanonicalResultRow,
    CanonicalResultSection, CanonicalResultTable, CapabilityCellReferenceV2,
    GENERAL_SEM_EFFECTS_V1_METHOD_VERSION, GENERAL_SEM_PLS_CASE_BOOTSTRAP_METHOD_VERSION_V1,
    RecipeV4CompilerTarget, SemModelV4, pls_general_bootstrap_capability_cell_v1,
    pls_general_recursive_effects_capability_cell_v1, project_general_sem_pls_base_recipe_v1,
    validate_canonical_result_document_v2,
};
use qpls_runner::RecipeV4GeneralSemPlsExecutionResultV1;
use uuid::Uuid;

pub(crate) fn general_sem_multiple_mediation_bootstrap_capability_cell_v1()
-> CapabilityCellReferenceV2 {
    pls_general_bootstrap_capability_cell_v1()
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
    let base_recipe = project_general_sem_pls_base_recipe_v1(recipe).map_err(|error| {
        vec![format!(
            "General SEM base-recipe projection failed: {error}"
        )]
    })?;
    let base_request = InternalRecipeV4PlsExecutionRequestV1 {
        surface: InternalRecipeV4ExecutionSurfaceV1::InternalLabs,
        experimental_labs_enabled: true,
        resident_data: InternalRecipeV4ResidentDataV1::ProjectResident,
        dataset_id: dataset_id.to_string(),
        dataset_fingerprint: result.source_dataset_fingerprint().into(),
        recipe: base_recipe,
        model: model.clone(),
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
        .map_err(|error| vec![format!("General SEM canonical projection failed: {error}")])?;
    let inferred = general_sem_results.inference_receipt.is_some();

    document.schema_version = CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
    document.title = if inferred {
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
    document.provenance.capability_cell = pls_general_recursive_effects_capability_cell_v1();
    document.provenance.method_version = if inferred {
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
    let point_cell = pls_general_recursive_effects_capability_cell_v1();
    let bootstrap_combination_cell = general_sem_multiple_mediation_bootstrap_capability_cell_v1();
    let mut document_cells = vec![base_cell, point_cell.clone()];
    if inferred {
        document_cells.push(bootstrap_combination_cell.clone());
    }
    sort_and_deduplicate_cells(&mut document_cells);
    document.capability_cells = Some(document_cells);

    let mut general_sem_table_cells = vec![point_cell.clone()];
    if inferred {
        general_sem_table_cells.push(bootstrap_combination_cell);
    }
    sort_and_deduplicate_cells(&mut general_sem_table_cells);
    let general_sem_tables = general_sem_tables(&general_sem_results, &general_sem_table_cells);
    let table_ids = general_sem_tables
        .iter()
        .map(|table| table.id.clone())
        .collect::<Vec<_>>();
    document.tables.extend(general_sem_tables);
    document.sections.push(CanonicalResultSection {
        id: "general_sem_effects".into(),
        title: "Mediation effects".into(),
        description: Some(
            "Stable specific-indirect, total-indirect, and total-effect estimands from the authored recursive SEM topology."
                .into(),
        ),
        table_ids,
        chart_ids: Vec::new(),
        capability_cells: Some(general_sem_table_cells),
    });
    document.general_sem_results = Some(general_sem_results);
    document.exclusions = if inferred {
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
    document.presentation.default_section_id = Some("general_sem_effects".into());
    document.presentation.default_table_id = document
        .sections
        .last()
        .and_then(|section| section.table_ids.first().cloned());

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
