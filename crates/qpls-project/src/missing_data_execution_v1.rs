use crate::{
    CanonicalMissingReasonV2, CanonicalResultCellV2, CanonicalResultDocumentV2,
    CanonicalResultTableV2,
};
use qpls_core::{
    AnalysisRecipeV4, CbsemEstimator, CbsemInput, CbsemModelType, CompiledAnalysisRecipeV4,
    CompiledCbsemInputV2, CompiledRecipePlanV4, MethodConfig, MissingDataPolicy,
    MissingDataPolicyV4, RecipeV4CompilerTarget, SemDataBindingV4, SemModelV4, SemVariableV4,
    compile_analysis_recipe_v4,
};
use qpls_data::{ColumnType, DataKind, DatasetDescriptor, ScaleType};
use qpls_estimation::{
    CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
    CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3, CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2, CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3,
    CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4, CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
    CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4, MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1,
    MEAN_REPLACEMENT_METHOD_VERSION_V1, MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1,
    MeanReplacementCaseReceiptV1, MeanReplacementPolicyV1, MeanReplacementReceiptV1,
    MeanReplacementVariableReceiptV1, MeanReplacementWarningLevelV1,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MISSING_DATA_EXECUTION_TABLE_ID_V1: &str = "missing_data_execution";
pub const MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1: &str = "mean_replacement_variables";
pub const MEAN_REPLACEMENT_CELLS_TABLE_ID_V1: &str = "mean_replacement_cells";
pub const MISSING_DATA_SECTION_ID_V1: &str = "missing_data";
pub const CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v4";
pub const CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v7";
pub const CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v5";
pub const CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v8";
pub const CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v9";
pub const CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v10";
pub const CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v11";
pub const CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v12";
pub const CBSEM_CURRENT_MEAN_STRUCTURE_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_cbsem_plan_v2_execution_v6";
pub const SCHEMA6_MISSING_DATA_VALIDATION_SCOPE_V1: &str =
    "descriptor_identity_shape_and_receipt_only";

pub const MISSING_DATA_EXECUTION_COLUMNS_V1: &[&str] = &[
    "method_version",
    "policy",
    "archive_validation_scope",
    "raw_replay_performed",
    "source_dataset_id",
    "source_dataset_fingerprint",
    "source_row_count",
    "retained_row_count",
    "omitted_row_count",
    "modeled_variable_count",
    "imputed_cell_count",
    "affected_case_count",
    "variable_warning_threshold",
    "high_missingness_threshold",
    "missingness_sha256",
    "completed_matrix_sha256",
    "receipt_sha256",
];

pub const MEAN_REPLACEMENT_VARIABLE_COLUMNS_V1: &[&str] = &[
    "variable_order",
    "variable_id",
    "source_column",
    "canonical_missing_markers_json",
    "observed_count",
    "missing_count",
    "replacement_mean",
    "missing_fraction",
    "warning_level",
];

pub const MEAN_REPLACEMENT_CELL_COLUMNS_V1: &[&str] = &[
    "row_index_zero_based",
    "variable_order",
    "variable_id",
    "source_column",
    "replacement_mean",
    "case_missing_fraction",
    "high_missingness_warning",
];

#[derive(Debug, thiserror::Error)]
pub enum MissingDataExecutionDocumentV1Error {
    #[error("invalid CB-SEM mean-replacement persistence contract: {0}")]
    Invalid(String),
    #[error("CB-SEM Recipe-v4 recompilation failed: {0}")]
    Recompilation(String),
}

#[derive(Serialize)]
struct MeanReplacementReceiptHashInputV1<'a> {
    method_version: &'a str,
    policy: MeanReplacementPolicyV1,
    source_dataset_id: &'a str,
    source_dataset_fingerprint: &'a str,
    source_row_count: usize,
    retained_row_count: usize,
    omitted_row_count: usize,
    modeled_variable_count: usize,
    imputed_cell_count: usize,
    affected_case_count: usize,
    variable_warning_threshold: f64,
    high_missingness_threshold: f64,
    variables: &'a [MeanReplacementVariableReceiptV1],
    cases: &'a [MeanReplacementCaseReceiptV1],
    missingness_sha256: &'a str,
    completed_matrix_sha256: &'a str,
}

/// Reconstructs the exact typed receipt from its three canonical tables. This
/// gate proves table/receipt self-consistency and does not claim access to the
/// schema-v5 Arrow buffers that produced the execution.
pub fn mean_replacement_receipt_from_document_v1(
    document: &CanonicalResultDocumentV2,
) -> Result<MeanReplacementReceiptV1, MissingDataExecutionDocumentV1Error> {
    let execution = exact_table(document, MISSING_DATA_EXECUTION_TABLE_ID_V1)?;
    let variables = exact_table(document, MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1)?;
    let cells = exact_table(document, MEAN_REPLACEMENT_CELLS_TABLE_ID_V1)?;
    validate_exact_columns(execution, MISSING_DATA_EXECUTION_COLUMNS_V1)?;
    validate_exact_columns(variables, MEAN_REPLACEMENT_VARIABLE_COLUMNS_V1)?;
    validate_exact_columns(cells, MEAN_REPLACEMENT_CELL_COLUMNS_V1)?;
    if execution.rows.len() != 1 || execution.rows[0].id != "execution" {
        return invalid("missing_data_execution must contain exactly the execution row");
    }
    let summary = &execution.rows[0].cells;
    let method_version = text_cell(&summary[0], "method_version")?.to_owned();
    if method_version != MEAN_REPLACEMENT_METHOD_VERSION_V1 {
        return invalid("method_version must equal mean_replacement_v1");
    }
    if text_cell(&summary[1], "policy")? != "mean_replacement" {
        return invalid("policy must equal mean_replacement");
    }
    if text_cell(&summary[2], "archive_validation_scope")?
        != SCHEMA6_MISSING_DATA_VALIDATION_SCOPE_V1
        || boolean_cell(&summary[3], "raw_replay_performed")?
    {
        return invalid(
            "schema-6 persistence must disclose descriptor-only validation without Arrow replay",
        );
    }
    let source_dataset_id = text_cell(&summary[4], "source_dataset_id")?.to_owned();
    let source_dataset_fingerprint =
        dataset_fingerprint_cell(&summary[5], "source_dataset_fingerprint")?;
    let source_row_count = count_cell(&summary[6], "source_row_count")?;
    let retained_row_count = count_cell(&summary[7], "retained_row_count")?;
    let omitted_row_count = count_cell(&summary[8], "omitted_row_count")?;
    let modeled_variable_count = count_cell(&summary[9], "modeled_variable_count")?;
    let imputed_cell_count = count_cell(&summary[10], "imputed_cell_count")?;
    let affected_case_count = count_cell(&summary[11], "affected_case_count")?;
    let variable_warning_threshold = number_cell(&summary[12], "variable_warning_threshold")?;
    let high_missingness_threshold = number_cell(&summary[13], "high_missingness_threshold")?;
    let missingness_sha256 = sha256_cell(&summary[14], "missingness_sha256")?;
    let completed_matrix_sha256 = sha256_cell(&summary[15], "completed_matrix_sha256")?;
    let receipt_sha256 = sha256_cell(&summary[16], "receipt_sha256")?;

    if retained_row_count != source_row_count
        || omitted_row_count != 0
        || source_row_count < 10
        || modeled_variable_count == 0
        || modeled_variable_count != variables.rows.len()
        || imputed_cell_count != cells.rows.len()
        || variable_warning_threshold.to_bits()
            != MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1.to_bits()
        || high_missingness_threshold.to_bits()
            != MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1.to_bits()
    {
        return invalid("execution counts or frozen warning thresholds are incoherent");
    }

    let mut variable_receipts = Vec::with_capacity(variables.rows.len());
    let mut variable_index = BTreeMap::<String, usize>::new();
    for (index, row) in variables.rows.iter().enumerate() {
        if row.id != format!("mean_replacement_variable_{index:04}") {
            return invalid("mean_replacement_variables row order or identity is non-canonical");
        }
        let order = count_cell(&row.cells[0], "variable_order")?;
        let variable_id = text_cell(&row.cells[1], "variable_id")?.to_owned();
        let source_column = text_cell(&row.cells[2], "source_column")?.to_owned();
        if order != index
            || variable_id.trim().is_empty()
            || source_column.trim().is_empty()
            || variable_index.insert(variable_id.clone(), index).is_some()
        {
            return invalid(
                "mean-replacement variable identity is empty, duplicated, or reordered",
            );
        }
        let markers_text = text_cell(&row.cells[3], "canonical_missing_markers_json")?;
        let stored_missing_markers = serde_json::from_str::<Vec<String>>(markers_text)
            .map_err(|error| invalid_error(format!("missing-marker JSON is invalid: {error}")))?;
        if serde_json::to_string(&stored_missing_markers)
            .map_err(|error| invalid_error(error.to_string()))?
            != markers_text
            || canonical_missing_markers(&stored_missing_markers) != stored_missing_markers
        {
            return invalid("missing-marker JSON is not canonical");
        }
        let observed_count = count_cell(&row.cells[4], "observed_count")?;
        let missing_count = count_cell(&row.cells[5], "missing_count")?;
        let replacement_mean = number_cell(&row.cells[6], "replacement_mean")?;
        let missing_fraction = number_cell(&row.cells[7], "missing_fraction")?;
        let warning_level = match text_cell(&row.cells[8], "warning_level")? {
            "none" => MeanReplacementWarningLevelV1::None,
            "at_least_five_percent" => MeanReplacementWarningLevelV1::AtLeastFivePercent,
            "above_fifteen_percent" => MeanReplacementWarningLevelV1::AboveFifteenPercent,
            _ => return invalid("mean-replacement warning level is unsupported"),
        };
        if observed_count.checked_add(missing_count) != Some(source_row_count)
            || missing_fraction.to_bits() != fraction(missing_count, source_row_count).to_bits()
            || warning_level != variable_warning_level(missing_count, source_row_count)
        {
            return invalid("mean-replacement variable counts, fraction, or warning drifted");
        }
        variable_receipts.push(MeanReplacementVariableReceiptV1 {
            variable_order: order,
            variable_id,
            source_column,
            canonical_missing_markers: stored_missing_markers,
            observed_count,
            missing_count,
            replacement_mean,
            missing_fraction,
            warning_level,
        });
    }

    let mut case_rows = BTreeMap::<usize, Vec<(usize, String)>>::new();
    let mut case_metadata = BTreeMap::<usize, (f64, bool)>::new();
    let mut previous_identity = None::<(usize, usize)>;
    let mut missing_by_variable = vec![0usize; variable_receipts.len()];
    for (index, row) in cells.rows.iter().enumerate() {
        if row.id != format!("mean_replacement_cell_{index:06}") {
            return invalid("mean_replacement_cells row identity is non-canonical");
        }
        let row_index = count_cell(&row.cells[0], "row_index_zero_based")?;
        let variable_order = count_cell(&row.cells[1], "variable_order")?;
        let variable_id = text_cell(&row.cells[2], "variable_id")?.to_owned();
        let source_column = text_cell(&row.cells[3], "source_column")?;
        let replacement_mean = number_cell(&row.cells[4], "replacement_mean")?;
        let case_missing_fraction = number_cell(&row.cells[5], "case_missing_fraction")?;
        let high_missingness_warning = boolean_cell(&row.cells[6], "high_missingness_warning")?;
        if row_index >= source_row_count || variable_order >= variable_receipts.len() {
            return invalid("mean-replacement cell points outside the declared matrix shape");
        }
        if previous_identity.is_some_and(|previous| previous >= (row_index, variable_order)) {
            return invalid("mean-replacement cells are duplicated or reordered");
        }
        previous_identity = Some((row_index, variable_order));
        let variable = &variable_receipts[variable_order];
        if variable.variable_id != variable_id
            || variable.source_column != source_column
            || variable.replacement_mean.to_bits() != replacement_mean.to_bits()
        {
            return invalid("mean-replacement cell identity or replacement mean drifted");
        }
        missing_by_variable[variable_order] += 1;
        case_rows
            .entry(row_index)
            .or_default()
            .push((variable_order, variable_id));
        match case_metadata.entry(row_index) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert((case_missing_fraction, high_missingness_warning));
            }
            std::collections::btree_map::Entry::Occupied(entry)
                if entry.get().0.to_bits() == case_missing_fraction.to_bits()
                    && entry.get().1 == high_missingness_warning => {}
            std::collections::btree_map::Entry::Occupied(_) => {
                return invalid("cells from one case carry inconsistent case metadata");
            }
        }
    }
    if variable_receipts
        .iter()
        .zip(&missing_by_variable)
        .any(|(variable, count)| variable.missing_count != *count)
    {
        return invalid("variable missing counts differ from canonical imputed cells");
    }
    let cases = case_rows
        .into_iter()
        .map(|(row_index_zero_based, cells)| {
            let (recorded_fraction, recorded_warning) = case_metadata[&row_index_zero_based];
            let expected_fraction = fraction(cells.len(), modeled_variable_count);
            let expected_warning = above_percent(cells.len(), modeled_variable_count, 15);
            if recorded_fraction.to_bits() != expected_fraction.to_bits()
                || recorded_warning != expected_warning
            {
                return invalid(
                    "case missing fraction or high-missingness warning differs from its cells",
                );
            }
            Ok(MeanReplacementCaseReceiptV1 {
                row_index_zero_based,
                imputed_variable_ids: cells.into_iter().map(|(_, id)| id).collect(),
                missing_fraction: recorded_fraction,
                high_missingness_warning: recorded_warning,
            })
        })
        .collect::<Result<Vec<_>, MissingDataExecutionDocumentV1Error>>()?;
    if cases.len() != affected_case_count {
        return invalid("affected_case_count differs from canonical imputed cases");
    }

    let receipt = MeanReplacementReceiptV1 {
        method_version,
        policy: MeanReplacementPolicyV1::MeanReplacement,
        source_dataset_id,
        source_dataset_fingerprint,
        source_row_count,
        retained_row_count,
        omitted_row_count,
        modeled_variable_count,
        imputed_cell_count,
        affected_case_count,
        variable_warning_threshold,
        high_missingness_threshold,
        variables: variable_receipts,
        cases,
        missingness_sha256,
        completed_matrix_sha256,
        receipt_sha256,
    };
    if receipt_sha256_v1(&receipt)? != receipt.receipt_sha256 {
        return invalid("receipt_sha256 does not match the reconstructed typed receipt");
    }
    let expected_missingness = missingness_sha256_from_receipt_v1(&receipt);
    if expected_missingness != receipt.missingness_sha256 {
        return invalid("missingness_sha256 differs from the canonical imputed-cell mask");
    }
    Ok(receipt)
}

/// Validates the canonical tables against the exact in-memory execution
/// receipt before they enter a schema-v6 attachment.
pub fn validate_mean_replacement_tables_against_receipt_v1(
    document: &CanonicalResultDocumentV2,
    expected: &MeanReplacementReceiptV1,
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    let reconstructed = mean_replacement_receipt_from_document_v1(document)?;
    if &reconstructed != expected {
        return invalid("canonical missing-data tables differ from the execution receipt");
    }
    validate_missing_data_section(document)
}

#[derive(Debug, Clone, Copy)]
struct CurrentCbsemExecutionIdentityV1 {
    adapter_version: &'static str,
    estimator_method_version: &'static str,
    moment_input_method_version: &'static str,
    compiled_moment_schema_version: usize,
    mean_structure: bool,
    mean_replacement: bool,
}

fn current_cbsem_execution_identity_v1(
    document: &CanonicalResultDocumentV2,
) -> Result<CurrentCbsemExecutionIdentityV1, MissingDataExecutionDocumentV1Error> {
    match document.provenance.engine_version.as_str() {
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V1 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V1,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
                    as usize,
                mean_structure: false,
                mean_replacement: false,
            })
        }
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
                    as usize,
                mean_structure: false,
                mean_replacement: false,
            })
        }
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
                    as usize,
                mean_structure: false,
                mean_replacement: false,
            })
        }
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
                    as usize,
                mean_structure: false,
                mean_replacement: false,
            })
        }
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
                    as usize,
                mean_structure: false,
                mean_replacement: false,
            })
        }
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V3,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V2
                    as usize,
                mean_structure: false,
                mean_replacement: false,
            })
        }
        CBSEM_CURRENT_MEAN_STRUCTURE_EXECUTION_ADAPTER_VERSION_V1 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_CURRENT_MEAN_STRUCTURE_EXECUTION_ADAPTER_VERSION_V1,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V4,
                moment_input_method_version: CBSEM_COMPILED_MOMENT_INPUT_METHOD_VERSION_V4,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V3
                    as usize,
                mean_structure: true,
                mean_replacement: false,
            })
        }
        CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2 => {
            Ok(CurrentCbsemExecutionIdentityV1 {
                adapter_version: CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2,
                estimator_method_version: CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3,
                moment_input_method_version:
                    CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1,
                compiled_moment_schema_version: CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4
                    as usize,
                mean_structure: false,
                mean_replacement: true,
            })
        }
        _ => invalid(
            "CB-SEM execution adapter is not a current v5, v6, v7, v8, v9, v10, v11, or v12 identity",
        ),
    }
}

fn current_cbsem_execution_recipe_seed_v1(adapter_version: &str, recipe_seed: u64) -> Option<u64> {
    matches!(
        adapter_version,
        CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3
            | CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4
            | CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5
            | CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6
    )
    .then_some(recipe_seed)
}

fn compile_and_validate_current_cbsem_execution_binding_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<CompiledAnalysisRecipeV4, MissingDataExecutionDocumentV1Error> {
    let identity = current_cbsem_execution_identity_v1(document)?;
    if document.provenance.method_version != identity.estimator_method_version {
        return invalid("canonical CB-SEM method version differs from its current adapter");
    }

    let Some(MethodConfig::Cbsem {
        model_type,
        estimator,
        input,
        mean_structure,
        ..
    }) = recipe.method_config.as_ref()
    else {
        return invalid("resident Recipe-v4 does not carry a CB-SEM method configuration");
    };
    if *estimator != CbsemEstimator::Ml || *mean_structure != identity.mean_structure {
        return invalid(
            "resident CB-SEM method or mean-structure setting differs from the adapter",
        );
    }

    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let artifact = compile_analysis_recipe_v4(
        recipe,
        Some(model),
        target,
        qpls_core::CapabilityCellReferenceV2 {
            registry_schema_version: document.provenance.capability_cell.registry_schema_version,
            capability_id: document.provenance.capability_cell.capability_id.clone(),
            cell_id: document.provenance.capability_cell.cell_id.clone(),
            capability_version: document
                .provenance
                .capability_cell
                .capability_version
                .clone(),
        },
    )
    .map_err(|error| MissingDataExecutionDocumentV1Error::Recompilation(error.to_string()))?;
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        unreachable!("the exact CB-SEM target must return a CB-SEM plan")
    };
    let (compiled_input, compiled_kind, compiled_mean_replacement) = match plan.input() {
        CompiledCbsemInputV2::Raw { missing_data, .. } => (
            CbsemInput::Raw,
            DataKind::Raw,
            *missing_data == MissingDataPolicyV4::MeanReplacement,
        ),
        CompiledCbsemInputV2::Covariance { .. } => {
            (CbsemInput::Covariance, DataKind::Covariance, false)
        }
        CompiledCbsemInputV2::Correlation { .. } => {
            (CbsemInput::Correlation, DataKind::Correlation, false)
        }
    };
    if *input != compiled_input || identity.mean_replacement != compiled_mean_replacement {
        return invalid("compiled CB-SEM input or missing-data policy differs from the adapter");
    }
    let input_name = match compiled_input {
        CbsemInput::Raw => "raw",
        CbsemInput::Covariance => "covariance",
        CbsemInput::Correlation => "correlation",
    };
    let model_type_name = match model_type {
        CbsemModelType::Cfa => "cfa",
        CbsemModelType::Sem => "sem",
    };

    let compilation = artifact.receipt();
    let compiled_fingerprint = recorded_sha256(compilation.dataset_fingerprint())
        .ok_or_else(|| invalid_error("compiled dataset fingerprint is not lowercase SHA-256"))?;
    let resident_fingerprint = recorded_sha256(&dataset.fingerprint.0)
        .ok_or_else(|| invalid_error("resident dataset fingerprint is not lowercase SHA-256"))?;
    if document.provenance.recipe_id != compilation.recipe_id().to_string()
        || document.provenance.recipe_digest != compilation.recipe_analytical_sha256()
        || document.provenance.model_id != compilation.model_id()
        || document.provenance.model_digest != compilation.model_scientific_sha256()
        || document.provenance.dataset_id != plan.input().dataset_id()
        || document.provenance.dataset_id != dataset.id.to_string()
        || document.provenance.dataset_fingerprint != compiled_fingerprint
        || document.provenance.dataset_fingerprint != resident_fingerprint
        || dataset.schema.kind != compiled_kind
        || document.provenance.capability_cell.registry_schema_version
            != compilation.capability_cell().registry_schema_version
        || document.provenance.capability_cell.capability_id
            != compilation.capability_cell().capability_id
        || document.provenance.capability_cell.cell_id != compilation.capability_cell().cell_id
        || document.provenance.capability_cell.capability_version
            != compilation.capability_cell().capability_version
        || document.provenance.seed
            != current_cbsem_execution_recipe_seed_v1(
                identity.adapter_version,
                recipe.settings.seed,
            )
        || usize::try_from(document.provenance.workers).ok() != Some(recipe.settings.workers)
    {
        return invalid(
            "canonical provenance differs from resident Recipe-v4/model/dataset recompilation",
        );
    }

    let summary = exact_table(document, "estimation_summary")?;
    if summary.rows.len() != 1 || summary.rows[0].id != "run" {
        return invalid("estimation_summary must contain exactly the run row");
    }
    let row = &summary.rows[0];
    let get = |id: &str| -> Result<&CanonicalResultCellV2, MissingDataExecutionDocumentV1Error> {
        let positions = summary
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| (column.id == id).then_some(index))
            .collect::<Vec<_>>();
        if positions.len() != 1 {
            return invalid(format!(
                "estimation_summary column {id} is missing or duplicated"
            ));
        }
        row.cells
            .get(positions[0])
            .ok_or_else(|| invalid_error("estimation_summary row width drifted"))
    };
    if text_cell(get("model_type")?, "model_type")? != model_type_name
        || text_cell(get("estimator")?, "estimator")? != "ml"
        || text_cell(
            get("execution_adapter_version")?,
            "execution_adapter_version",
        )? != identity.adapter_version
        || text_cell(get("estimator_method_version")?, "estimator_method_version")?
            != identity.estimator_method_version
        || text_cell(
            get("moment_input_method_version")?,
            "moment_input_method_version",
        )? != identity.moment_input_method_version
        || count_cell(
            get("compiled_moment_schema_version")?,
            "compiled_moment_schema_version",
        )? != identity.compiled_moment_schema_version
        || boolean_cell(get("mean_structure")?, "mean_structure")? != identity.mean_structure
        || text_cell(get("input")?, "input")? != input_name
    {
        return invalid(
            "estimation_summary differs from the resident compiled CB-SEM execution identity",
        );
    }
    Ok(artifact)
}

/// Cross-binds current schema-v6 CB-SEM adapters to the exact resident
/// Recipe-v4, SemModelV4, and dataset descriptor. Historical v2/v3/v4
/// attachments deliberately remain outside this stricter current-generation
/// contract.
pub fn validate_recipe_v4_cbsem_current_execution_document_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    compile_and_validate_current_cbsem_execution_binding_v1(document, recipe, model, dataset)?;
    Ok(())
}

/// Schema-v6 descriptor-only validation. It deterministically recompiles the
/// recipe/model and binds the receipt to the exact resident descriptor, while
/// explicitly avoiding any claim that absent Arrow rows were replayed.
pub fn validate_recipe_v4_cbsem_missing_data_execution_document_v1(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
    dataset: &DatasetDescriptor,
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    let table_presence = [
        MISSING_DATA_EXECUTION_TABLE_ID_V1,
        MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1,
        MEAN_REPLACEMENT_CELLS_TABLE_ID_V1,
    ]
    .map(|id| document.tables.iter().any(|table| table.id == id));
    let recipe_mean = recipe.settings.missing_data == MissingDataPolicy::MeanReplacement;
    let model_mean = matches!(
        &model.data_binding,
        SemDataBindingV4::Raw {
            missing_data: MissingDataPolicyV4::MeanReplacement,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
            ..
        }
    );
    if recipe_mean != model_mean {
        return invalid("recipe and SemModelV4 mean-replacement policies differ");
    }
    if !recipe_mean {
        if table_presence.into_iter().any(|present| present) {
            return invalid("non-mean-replacement result carries missing-data execution tables");
        }
        return Ok(());
    }
    if table_presence.into_iter().any(|present| !present) {
        return invalid("mean-replacement result omitted one or more canonical receipt tables");
    }
    if document.provenance.method_version != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || !matches!(
            document.provenance.engine_version.as_str(),
            CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V1
                | CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2
        )
    {
        return invalid("mean-replacement estimator or adapter identity drifted");
    }

    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let artifact = if document.provenance.engine_version
        == CBSEM_MEAN_REPLACEMENT_EXECUTION_ADAPTER_VERSION_V2
    {
        compile_and_validate_current_cbsem_execution_binding_v1(document, recipe, model, dataset)?
    } else {
        compile_analysis_recipe_v4(recipe, Some(model), target, target.capability_cell()).map_err(
            |error| MissingDataExecutionDocumentV1Error::Recompilation(error.to_string()),
        )?
    };
    let CompiledRecipePlanV4::CbsemPlanV2 { plan } = artifact.plan() else {
        unreachable!("the exact CB-SEM target must return a CB-SEM plan")
    };
    if !matches!(
        plan.input(),
        CompiledCbsemInputV2::Raw {
            missing_data: MissingDataPolicyV4::MeanReplacement,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
            ..
        }
    ) {
        return invalid("recompiled CB-SEM plan is outside the frozen raw mean-replacement slice");
    }
    let compilation = artifact.receipt();
    let recorded_fingerprint = recorded_sha256(compilation.dataset_fingerprint())
        .ok_or_else(|| invalid_error("compiled dataset fingerprint is not lowercase SHA-256"))?;
    if document.provenance.recipe_id != compilation.recipe_id().to_string()
        || document.provenance.recipe_digest != compilation.recipe_analytical_sha256()
        || document.provenance.model_id != compilation.model_id()
        || document.provenance.model_digest != compilation.model_scientific_sha256()
        || document.provenance.dataset_id != plan.input().dataset_id()
        || document.provenance.dataset_fingerprint != recorded_fingerprint
        || document.provenance.capability_cell.registry_schema_version
            != compilation.capability_cell().registry_schema_version
        || document.provenance.capability_cell.capability_id
            != compilation.capability_cell().capability_id
        || document.provenance.capability_cell.cell_id != compilation.capability_cell().cell_id
        || document.provenance.capability_cell.capability_version
            != compilation.capability_cell().capability_version
    {
        return invalid("canonical provenance differs from deterministic Recipe-v4 recompilation");
    }

    let receipt = mean_replacement_receipt_from_document_v1(document)?;
    validate_missing_data_section(document)?;
    if dataset.id.to_string() != receipt.source_dataset_id
        || dataset.id.to_string() != document.provenance.dataset_id
        || dataset.fingerprint.0 != receipt.source_dataset_fingerprint
        || recorded_sha256(&dataset.fingerprint.0)
            != Some(document.provenance.dataset_fingerprint.as_str())
        || dataset.schema.kind != DataKind::Raw
        || dataset.schema.sample_size.is_some()
        || dataset.schema.case_count != receipt.source_row_count
    {
        return invalid(
            "schema-v6 dataset descriptor differs from the archived receipt identity or shape",
        );
    }

    let expected_variables = plan
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                id,
                source_column,
                missing_markers,
                ..
            } => Some((
                id.clone(),
                (
                    source_column.clone(),
                    canonical_missing_markers(missing_markers),
                ),
            )),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    if expected_variables.len() != receipt.variables.len() {
        return invalid("modeled observed-variable count differs from deterministic recompilation");
    }
    for (index, ((expected_id, (expected_source, expected_markers)), observed)) in
        expected_variables
            .iter()
            .zip(&receipt.variables)
            .enumerate()
    {
        let matching_columns = dataset
            .schema
            .columns
            .iter()
            .filter(|column| column.name == *expected_source)
            .collect::<Vec<_>>();
        let [column] = matching_columns.as_slice() else {
            return invalid("schema-v6 descriptor omits a modeled source column");
        };
        if observed.variable_order != index
            || observed.variable_id != *expected_id
            || observed.source_column != *expected_source
            || observed.canonical_missing_markers != *expected_markers
            || canonical_missing_markers(&column.missing_markers) != *expected_markers
            || column.column_type != ColumnType::Numeric
            || column.scale_type != ScaleType::Continuous
        {
            return invalid(
                "receipt variable order, identity, markers, or continuous scale differs from model/descriptor bindings",
            );
        }
    }

    validate_estimation_summary(document, &receipt)?;
    validate_canonical_covariance(document, &receipt)?;
    Ok(())
}

fn validate_missing_data_section(
    document: &CanonicalResultDocumentV2,
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    let matching = document
        .sections
        .iter()
        .filter(|section| section.id == MISSING_DATA_SECTION_ID_V1)
        .collect::<Vec<_>>();
    if matching.len() != 1
        || matching[0].table_ids
            != [
                MISSING_DATA_EXECUTION_TABLE_ID_V1.to_owned(),
                MEAN_REPLACEMENT_VARIABLES_TABLE_ID_V1.to_owned(),
                MEAN_REPLACEMENT_CELLS_TABLE_ID_V1.to_owned(),
            ]
    {
        return invalid("missing_data section is absent, duplicated, or reordered");
    }
    Ok(())
}

fn validate_estimation_summary(
    document: &CanonicalResultDocumentV2,
    receipt: &MeanReplacementReceiptV1,
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    let table = exact_table(document, "estimation_summary")?;
    if table.rows.len() != 1 || table.rows[0].id != "run" {
        return invalid("estimation_summary must contain exactly the run row");
    }
    let row = &table.rows[0];
    let get = |id: &str| -> Result<&CanonicalResultCellV2, MissingDataExecutionDocumentV1Error> {
        let positions = table
            .columns
            .iter()
            .enumerate()
            .filter_map(|(index, column)| (column.id == id).then_some(index))
            .collect::<Vec<_>>();
        if positions.len() != 1 {
            return invalid(format!(
                "estimation_summary column {id} is missing or duplicated"
            ));
        }
        row.cells
            .get(positions[0])
            .ok_or_else(|| invalid_error("estimation_summary row width drifted"))
    };
    if text_cell(
        get("execution_adapter_version")?,
        "execution_adapter_version",
    )? != document.provenance.engine_version.as_str()
        || text_cell(get("estimator_method_version")?, "estimator_method_version")?
            != CBSEM_EXACT_PARAMETER_TABLE_METHOD_VERSION_V3
        || text_cell(
            get("moment_input_method_version")?,
            "moment_input_method_version",
        )? != CBSEM_COMPILED_MOMENT_INPUT_MEAN_REPLACEMENT_METHOD_VERSION_V1
        || count_cell(
            get("compiled_moment_schema_version")?,
            "compiled_moment_schema_version",
        )? != CBSEM_COMPILED_MOMENT_RESULT_SCHEMA_VERSION_V4 as usize
        || boolean_cell(get("mean_structure")?, "mean_structure")?
        || text_cell(get("input")?, "input")? != "raw"
        || !boolean_cell(get("converged")?, "converged")?
        || count_cell(get("sample_size")?, "sample_size")? != receipt.retained_row_count
        || count_cell(get("omitted_observations")?, "omitted_observations")?
            != receipt.omitted_row_count
        || text_cell(get("covariance_denominator")?, "covariance_denominator")?
            != "maximum_likelihood_n"
        || !matches!(
            get("declared_sample_size")?,
            CanonicalResultCellV2::Missing {
                reason: CanonicalMissingReasonV2::NotEstimated,
                display: None,
            }
        )
        || !matches!(
            get("canonical_observed_means_sha256")?,
            CanonicalResultCellV2::Missing {
                reason: CanonicalMissingReasonV2::NotEstimated,
                display: None,
            }
        )
    {
        return invalid(
            "estimation_summary differs from frozen mean-replacement execution identity",
        );
    }
    Ok(())
}

fn validate_canonical_covariance(
    document: &CanonicalResultDocumentV2,
    receipt: &MeanReplacementReceiptV1,
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    let table = exact_table(document, "canonical_ml_covariance")?;
    let sources = receipt
        .variables
        .iter()
        .map(|variable| variable.source_column.as_str())
        .collect::<Vec<_>>();
    if table.columns.len() != sources.len() + 1
        || table.columns[0].id != "row"
        || table.rows.len() != sources.len()
    {
        return invalid("canonical ML covariance shape differs from modeled variables");
    }
    for (index, source) in sources.iter().enumerate() {
        if table.columns[index + 1].id != format!("column_{index:04}")
            || table.columns[index + 1].label != *source
            || table.rows[index].id != format!("row_{index:04}")
            || text_cell(&table.rows[index].cells[0], "covariance row")? != *source
            || table.rows[index].cells.len() != sources.len() + 1
        {
            return invalid("canonical ML covariance variable identity or order drifted");
        }
    }
    let matrix = table
        .rows
        .iter()
        .map(|row| {
            row.cells[1..]
                .iter()
                .map(|cell| number_cell(cell, "canonical covariance cell"))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, MissingDataExecutionDocumentV1Error>>()?;
    let summary = exact_table(document, "estimation_summary")?;
    let digest_index = summary
        .columns
        .iter()
        .position(|column| column.id == "canonical_covariance_sha256")
        .ok_or_else(|| invalid_error("estimation_summary omits canonical covariance digest"))?;
    let recorded = text_cell(
        &summary.rows[0].cells[digest_index],
        "canonical_covariance_sha256",
    )?;
    if covariance_sha256(&receipt.variables, receipt.retained_row_count, &matrix) != recorded {
        return invalid("canonical covariance digest differs from its archived matrix");
    }
    Ok(())
}

fn covariance_sha256(
    variables: &[MeanReplacementVariableReceiptV1],
    sample_size: usize,
    matrix: &[Vec<f64>],
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-cbsem-canonical-ml-covariance-v2\0");
    digest.update(b"raw");
    digest.update((sample_size as u64).to_le_bytes());
    digest.update(b"maximum_likelihood_n");
    for variable in variables {
        digest.update((variable.variable_id.len() as u64).to_le_bytes());
        digest.update(variable.variable_id.as_bytes());
    }
    for row in matrix {
        for value in row {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn missingness_sha256_from_receipt_v1(receipt: &MeanReplacementReceiptV1) -> String {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-continuous-raw-mean-replacement-v1:missingness\0");
    hash_string(&mut digest, &receipt.source_dataset_fingerprint);
    digest.update((receipt.source_row_count as u64).to_be_bytes());
    digest.update((receipt.variables.len() as u64).to_be_bytes());
    for variable in &receipt.variables {
        digest.update((variable.variable_order as u64).to_be_bytes());
        hash_string(&mut digest, &variable.variable_id);
        hash_string(&mut digest, &variable.source_column);
        digest.update((variable.canonical_missing_markers.len() as u64).to_be_bytes());
        for marker in &variable.canonical_missing_markers {
            hash_string(&mut digest, marker);
        }
    }
    let missing = receipt
        .cases
        .iter()
        .flat_map(|case| {
            case.imputed_variable_ids
                .iter()
                .map(move |variable| (case.row_index_zero_based, variable.as_str()))
        })
        .collect::<BTreeSet<_>>();
    for row in 0..receipt.source_row_count {
        for variable in &receipt.variables {
            digest.update([u8::from(
                missing.contains(&(row, variable.variable_id.as_str())),
            )])
        }
    }
    format!("{:x}", digest.finalize())
}

fn receipt_sha256_v1(
    receipt: &MeanReplacementReceiptV1,
) -> Result<String, MissingDataExecutionDocumentV1Error> {
    let input = MeanReplacementReceiptHashInputV1 {
        method_version: &receipt.method_version,
        policy: receipt.policy,
        source_dataset_id: &receipt.source_dataset_id,
        source_dataset_fingerprint: &receipt.source_dataset_fingerprint,
        source_row_count: receipt.source_row_count,
        retained_row_count: receipt.retained_row_count,
        omitted_row_count: receipt.omitted_row_count,
        modeled_variable_count: receipt.modeled_variable_count,
        imputed_cell_count: receipt.imputed_cell_count,
        affected_case_count: receipt.affected_case_count,
        variable_warning_threshold: receipt.variable_warning_threshold,
        high_missingness_threshold: receipt.high_missingness_threshold,
        variables: &receipt.variables,
        cases: &receipt.cases,
        missingness_sha256: &receipt.missingness_sha256,
        completed_matrix_sha256: &receipt.completed_matrix_sha256,
    };
    let mut digest = Sha256::new();
    digest.update(b"quickpls-mean-replacement-receipt-v1\0");
    digest.update(
        serde_json::to_vec(&input).map_err(|error| {
            invalid_error(format!("receipt hash serialization failed: {error}"))
        })?,
    );
    Ok(format!("{:x}", digest.finalize()))
}

fn exact_table<'a>(
    document: &'a CanonicalResultDocumentV2,
    id: &str,
) -> Result<&'a CanonicalResultTableV2, MissingDataExecutionDocumentV1Error> {
    let matching = document
        .tables
        .iter()
        .filter(|table| table.id == id)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return invalid(format!("canonical table {id} is missing or duplicated"));
    }
    Ok(matching[0])
}

fn validate_exact_columns(
    table: &CanonicalResultTableV2,
    expected: &[&str],
) -> Result<(), MissingDataExecutionDocumentV1Error> {
    if table.columns.len() != expected.len()
        || table
            .columns
            .iter()
            .zip(expected)
            .any(|(column, expected)| column.id != *expected)
        || table
            .rows
            .iter()
            .any(|row| row.cells.len() != expected.len())
    {
        return invalid(format!("table {} has a drifted column contract", table.id));
    }
    Ok(())
}

fn text_cell<'a>(
    cell: &'a CanonicalResultCellV2,
    field: &str,
) -> Result<&'a str, MissingDataExecutionDocumentV1Error> {
    match cell {
        CanonicalResultCellV2::Text { value } => Ok(value),
        _ => invalid(format!("{field} must be text")),
    }
}

fn number_cell(
    cell: &CanonicalResultCellV2,
    field: &str,
) -> Result<f64, MissingDataExecutionDocumentV1Error> {
    match cell {
        CanonicalResultCellV2::Number { value, .. } if value.is_finite() => Ok(*value),
        _ => invalid(format!("{field} must be a finite number")),
    }
}

fn count_cell(
    cell: &CanonicalResultCellV2,
    field: &str,
) -> Result<usize, MissingDataExecutionDocumentV1Error> {
    let value = number_cell(cell, field)?;
    if value.fract() != 0.0 || !(0.0..=9_007_199_254_740_991.0).contains(&value) {
        return invalid(format!("{field} must be a nonnegative safe integer"));
    }
    Ok(value as usize)
}

fn boolean_cell(
    cell: &CanonicalResultCellV2,
    field: &str,
) -> Result<bool, MissingDataExecutionDocumentV1Error> {
    match cell {
        CanonicalResultCellV2::Boolean { value } => Ok(*value),
        _ => invalid(format!("{field} must be boolean")),
    }
}

fn sha256_cell(
    cell: &CanonicalResultCellV2,
    field: &str,
) -> Result<String, MissingDataExecutionDocumentV1Error> {
    let value = text_cell(cell, field)?;
    if !is_lowercase_sha256(value) {
        return invalid(format!("{field} must be lowercase SHA-256"));
    }
    Ok(value.to_owned())
}

fn dataset_fingerprint_cell(
    cell: &CanonicalResultCellV2,
    field: &str,
) -> Result<String, MissingDataExecutionDocumentV1Error> {
    let value = text_cell(cell, field)?;
    let valid = value
        .strip_prefix("v2:")
        .map_or_else(|| is_lowercase_sha256(value), is_lowercase_sha256);
    if !valid {
        return invalid(format!(
            "{field} must be a bare lowercase SHA-256 or v2:<lowercase SHA-256>"
        ));
    }
    Ok(value.to_owned())
}

fn is_lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn recorded_sha256(value: &str) -> Option<&str> {
    let candidate = value.rsplit_once(':').map_or(value, |(_, suffix)| suffix);
    is_lowercase_sha256(candidate).then_some(candidate)
}

fn canonical_missing_markers(markers: &[String]) -> Vec<String> {
    let mut canonical = markers
        .iter()
        .map(|marker| marker.trim())
        .filter(|marker| !marker.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    canonical.sort();
    canonical.dedup();
    canonical
}

fn hash_string(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn fraction(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}

fn at_least_percent(count: usize, total: usize, percent: u128) -> bool {
    total > 0 && (count as u128) * 100 >= (total as u128) * percent
}

fn above_percent(count: usize, total: usize, percent: u128) -> bool {
    total > 0 && (count as u128) * 100 > (total as u128) * percent
}

fn variable_warning_level(count: usize, total: usize) -> MeanReplacementWarningLevelV1 {
    if above_percent(count, total, 15) {
        MeanReplacementWarningLevelV1::AboveFifteenPercent
    } else if at_least_percent(count, total, 5) {
        MeanReplacementWarningLevelV1::AtLeastFivePercent
    } else {
        MeanReplacementWarningLevelV1::None
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, MissingDataExecutionDocumentV1Error> {
    Err(invalid_error(message))
}

fn invalid_error(message: impl Into<String>) -> MissingDataExecutionDocumentV1Error {
    MissingDataExecutionDocumentV1Error::Invalid(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint(value: &str) -> Result<String, MissingDataExecutionDocumentV1Error> {
        dataset_fingerprint_cell(
            &CanonicalResultCellV2::Text {
                value: value.into(),
            },
            "source_dataset_fingerprint",
        )
    }

    #[test]
    fn dataset_fingerprint_parser_preserves_legacy_and_v2_identities_exactly() {
        let legacy = "a".repeat(64);
        let current = format!("v2:{}", "b".repeat(64));
        assert_eq!(fingerprint(&legacy).unwrap(), legacy);
        assert_eq!(fingerprint(&current).unwrap(), current);
        assert_eq!(recorded_sha256(&legacy), Some(legacy.as_str()));
        assert_eq!(recorded_sha256(&current), Some(&current[3..]));
    }

    #[test]
    fn dataset_fingerprint_parser_rejects_unknown_prefix_case_and_length_drift() {
        for invalid in [
            format!("v3:{}", "a".repeat(64)),
            format!("v2:{}", "A".repeat(64)),
            format!("v2:{}", "a".repeat(63)),
            format!("v2:{}", "a".repeat(65)),
            "a".repeat(63),
            "a".repeat(65),
        ] {
            assert!(fingerprint(&invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn current_bootstrap_seed_binding_preserves_v11_and_adds_v12() {
        let seed = 20_260_816;
        for adapter in [
            CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V3,
            CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V4,
            CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V5,
            CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V6,
        ] {
            assert_eq!(
                current_cbsem_execution_recipe_seed_v1(adapter, seed),
                Some(seed)
            );
        }
        for adapter in [
            CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V1,
            CBSEM_CURRENT_LISTWISE_EXECUTION_ADAPTER_VERSION_V2,
        ] {
            assert_eq!(current_cbsem_execution_recipe_seed_v1(adapter, seed), None);
        }
    }
}
