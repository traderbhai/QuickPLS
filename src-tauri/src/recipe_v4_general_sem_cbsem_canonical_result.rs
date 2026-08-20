//! Canonical projection for bounded CB-SEM V3 Experimental Labs execution.

use qpls_core::{
    AnalysisRecipeV4, CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
    CBSEM_GENERAL_SEM_FIT_TABLE_ID_V1, CBSEM_GENERAL_SEM_IDENTIFICATION_TABLE_ID_V1,
    CBSEM_GENERAL_SEM_ML_CAPABILITY_VERSION_V1, CBSEM_GENERAL_SEM_PARAMETERS_TABLE_ID_V1,
    CBSEM_RECURSIVE_SEM_BOOTSTRAP_FAILURES_TABLE_ID_V1,
    CBSEM_RECURSIVE_SEM_BOOTSTRAP_INFERENCE_TABLE_ID_V1,
    CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1,
    CBSEM_RECURSIVE_SEM_BOOTSTRAP_RECEIPT_TABLE_ID_V1, CanonicalChartDisplayOptions,
    CanonicalResultDocumentV2, CanonicalResultExclusion, CanonicalResultPresentationV2,
    CanonicalResultProvenanceV2, CanonicalResultSection, GeneralSemInferenceV1, SemModelV4,
    canonical_cbsem_general_sem_tables_v1, cbsem_general_sem_ml_capability_cell_v1,
    cbsem_recursive_sem_bootstrap_capability_cell_v1, validate_canonical_result_document_v2,
};
use qpls_data::Dataset;
use qpls_runner::internal_cbsem_general_sem_execution::{
    INTERNAL_CBSEM_GENERAL_SEM_EXECUTION_RESULT_SCHEMA_VERSION_V1,
    InternalCbsemGeneralSemExecutionResultV1,
};
use uuid::Uuid;

const CBSEM_POINT_SECTION_ID_V1: &str = "cbsem_general_sem_point";
const CBSEM_BOOTSTRAP_SECTION_ID_V1: &str = "cbsem_recursive_sem_bootstrap";

#[allow(clippy::too_many_arguments)]
pub(crate) fn build_internal_cbsem_general_sem_canonical_result_v1(
    job_id: Uuid,
    project_id: Uuid,
    started_at: &str,
    completed_at: &str,
    dataset: &Dataset,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    execution: &InternalCbsemGeneralSemExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, Vec<String>> {
    if execution.schema_version() != INTERNAL_CBSEM_GENERAL_SEM_EXECUTION_RESULT_SCHEMA_VERSION_V1 {
        return Err(vec![
            "unexpected internal CB-SEM V3 execution schema".into(),
        ]);
    }
    let receipt = execution.compilation_receipt();
    let model_digest = model
        .scientific_sha256()
        .map_err(|error| vec![error.to_string()])?;
    if receipt.recipe_id() != recipe.id
        || receipt.model_id() != model.id.as_str()
        || receipt.model_scientific_sha256() != model_digest.as_str()
        || receipt.dataset_fingerprint() != dataset.fingerprint.0.as_str()
        || recipe.dataset_fingerprint.as_str() != dataset.fingerprint.0.as_str()
        || execution.capability_cells().is_empty()
        || !execution
            .capability_cells()
            .contains(receipt.capability_cell())
    {
        return Err(vec![
            "internal CB-SEM V3 execution differs from its resident model, recipe, dataset, or capability authority"
                .into(),
        ]);
    }
    let inference = recipe
        .general_sem_config
        .as_ref()
        .ok_or_else(|| vec!["GeneralSemConfigV1 is required".into()])?
        .inference;
    let is_bootstrap = matches!(inference, GeneralSemInferenceV1::CaseBootstrap { .. });
    let expected_primary = if is_bootstrap {
        cbsem_recursive_sem_bootstrap_capability_cell_v1()
    } else {
        cbsem_general_sem_ml_capability_cell_v1()
    };
    if receipt.capability_cell() != &expected_primary
        || execution
            .general_sem_results()
            .cbsem_bootstrap_receipt
            .is_some()
            != is_bootstrap
    {
        return Err(vec![
            "internal CB-SEM V3 primary cell or bootstrap payload differs from the resident inference request"
                .into(),
        ]);
    }

    let seed = match inference {
        GeneralSemInferenceV1::None => None,
        GeneralSemInferenceV1::CaseBootstrap { seed, .. } => {
            Some(i64::try_from(seed).map_err(|_| {
                vec!["bootstrap seed exceeds canonical i64 provenance range".into()]
            })?)
        }
    };
    let point_cell = cbsem_general_sem_ml_capability_cell_v1();
    let mut sections = vec![CanonicalResultSection {
        id: CBSEM_POINT_SECTION_ID_V1.into(),
        title: "CB-SEM ML estimates".into(),
        description: Some(
            "Strict resident parameter-table estimates, fit, and identification evidence.".into(),
        ),
        table_ids: vec![
            CBSEM_GENERAL_SEM_PARAMETERS_TABLE_ID_V1.into(),
            CBSEM_GENERAL_SEM_FIT_TABLE_ID_V1.into(),
            CBSEM_GENERAL_SEM_IDENTIFICATION_TABLE_ID_V1.into(),
        ],
        chart_ids: Vec::new(),
        capability_cells: Some(vec![point_cell.clone()]),
    }];
    if is_bootstrap {
        let bootstrap_cell = cbsem_recursive_sem_bootstrap_capability_cell_v1();
        sections.push(CanonicalResultSection {
            id: CBSEM_BOOTSTRAP_SECTION_ID_V1.into(),
            title: "Recursive-SEM case bootstrap".into(),
            description: Some(
                "Full-refit percentile Type-7 inference and the ordered failure ledger.".into(),
            ),
            table_ids: vec![
                CBSEM_RECURSIVE_SEM_BOOTSTRAP_INFERENCE_TABLE_ID_V1.into(),
                CBSEM_RECURSIVE_SEM_BOOTSTRAP_RECEIPT_TABLE_ID_V1.into(),
                CBSEM_RECURSIVE_SEM_BOOTSTRAP_FAILURES_TABLE_ID_V1.into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: Some(vec![bootstrap_cell]),
        });
    }

    let default_section = if is_bootstrap {
        CBSEM_BOOTSTRAP_SECTION_ID_V1
    } else {
        CBSEM_POINT_SECTION_ID_V1
    };
    let default_table = if is_bootstrap {
        CBSEM_RECURSIVE_SEM_BOOTSTRAP_INFERENCE_TABLE_ID_V1
    } else {
        CBSEM_GENERAL_SEM_PARAMETERS_TABLE_ID_V1
    };
    let document = CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: format!("result_{job_id}"),
        title: if is_bootstrap {
            "CB-SEM recursive-SEM bootstrap results".into()
        } else {
            "CB-SEM General SEM ML results".into()
        },
        provenance: CanonicalResultProvenanceV2 {
            run_id: job_id.to_string(),
            project_id: project_id.to_string(),
            model_id: model.id.clone(),
            model_digest,
            dataset_id: dataset.id.to_string(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            recipe_id: recipe.id.to_string(),
            recipe_digest: receipt.recipe_analytical_sha256().into(),
            capability_cell: expected_primary,
            method_version: if is_bootstrap {
                CBSEM_RECURSIVE_SEM_BOOTSTRAP_METHOD_VERSION_V1.into()
            } else {
                CBSEM_GENERAL_SEM_ML_CAPABILITY_VERSION_V1.into()
            },
            engine_version: execution.adapter_version().into(),
            seed,
            workers: if is_bootstrap {
                i64::try_from(recipe.settings.workers)
                    .map_err(|_| vec!["worker count exceeds canonical i64 range".into()])?
            } else {
                1
            },
            started_at: started_at.into(),
            completed_at: completed_at.into(),
        },
        capability_cells: Some(execution.capability_cells().to_vec()),
        general_sem_results: Some(execution.general_sem_results().clone()),
        sections,
        tables: canonical_cbsem_general_sem_tables_v1(execution.general_sem_results()),
        charts: Vec::new(),
        notices: Vec::new(),
        exclusions: vec![CanonicalResultExclusion {
            id: "bounded_cbsem_general_sem_v1".into(),
            capability_cell: Some(point_cell),
            title: "Bounded CB-SEM General SEM scope".into(),
            reason: "Raw continuous listwise single-group reflective CFA or recursive SEM without means, feedback, robust corrections, or derived indirect-effect inference."
                .into(),
        }],
        footnotes: Vec::new(),
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some(default_section.into()),
            default_table_id: Some(default_table.into()),
            precision: 4,
            missing_value_label: "—".into(),
            chart_defaults: CanonicalChartDisplayOptions::default(),
        },
    };
    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(validation.errors);
    }
    let archive_document = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
        serde_json::to_value(&document)
            .map_err(|error| vec![format!("canonical serialization failed: {error}")])?,
    )
    .map_err(|error| vec![format!("schema-6 canonical parsing failed: {error}")])?;
    archive_document
        .ensure_valid()
        .map_err(|error| vec![format!("schema-6 canonical validation failed: {error}")])?;
    Ok(document)
}
