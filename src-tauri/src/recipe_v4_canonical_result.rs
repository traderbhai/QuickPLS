use crate::InternalRecipeV4PlsExecutionRequestV1;
use qpls_core::{
    AnalysisMethod, CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION, CanonicalChartDisplayOptions,
    CanonicalColumnRole, CanonicalColumnType, CanonicalMissingReason, CanonicalNoticeSeverity,
    CanonicalResultCell, CanonicalResultColumn, CanonicalResultDocumentV2,
    CanonicalResultExclusion, CanonicalResultNotice, CanonicalResultPresentationV2,
    CanonicalResultProvenanceV2, CanonicalResultRow, CanonicalResultSection, CanonicalResultTable,
    CapabilityCellReferenceV2, CapabilityRegistryV2, CompiledRecipePlanV4,
    CompositeWeightNormalizationV4, MethodConfig, PLS_ALGORITHM_CAPABILITY_ID,
    PLS_ALGORITHM_CAPABILITY_VERSION, PLS_ALGORITHM_CELL_ID, PLS_NONLINEAR_EFFECTS_CAPABILITY_ID,
    PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION, PLS_NONLINEAR_EFFECTS_CELL_ID, Preprocessing,
    RecipeV4CompilerTarget, WeightingScheme, compile_analysis_recipe_v4,
    validate_canonical_result_document_v2,
};
use qpls_estimation::{
    NONLINEAR_EFFECTS_METHOD_VERSION, PLS_ALGORITHM_CONVERGENCE_RECEIPT_CONTRACT_VERSION_V1,
    PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1, PLS_METHOD_VERSION,
    PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2, PLS_SCORE_EXECUTION_METHOD_VERSION_V2,
    PlsEstimatedScoreModeV2, PlsPointEstimateAttributionV1, PlsPosthocMinimumSampleSize,
    PlsPosthocMinimumSampleSizeStatus, PlsResolvedInitialOuterWeightsV2,
    PlsResolvedScoreBlockKindV2, PlsResolvedScoreWeightV2, PlsResult,
};
use qpls_runner::{
    RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7, RecipeV4PlsExecutionResultV1,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

const JAVASCRIPT_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const NONLINEAR_SECTION_ID: &str = "nonlinear_relationships";
const NONLINEAR_DIAGNOSTICS_TABLE_ID: &str = "nonlinear_quadratic_diagnostics";
const NONLINEAR_EQUATION_FIT_TABLE_ID: &str = "nonlinear_equation_fit";
const NONLINEAR_METHOD_SCOPE_TABLE_ID: &str = "nonlinear_method_scope";
const NONLINEAR_TERM_V1: &str = "centered_squared_construct_score_v1";
const NONLINEAR_DIAGNOSTIC_COLUMNS: &[&str] = &[
    "source",
    "target",
    "linear_coefficient",
    "quadratic_coefficient",
    "standard_error",
    "t_statistic",
    "p_value_two_sided",
    "warning",
];
const NONLINEAR_EQUATION_FIT_COLUMNS: &[&str] = &[
    "target",
    "linear_r_squared",
    "augmented_r_squared",
    "delta_r_squared",
];
const NONLINEAR_METHOD_SCOPE_COLUMNS: &[&str] = &["method_version", "term", "warning"];
const NONLINEAR_ENGINE_WARNING_V1: &str = "Nonlinear effects are validated for the documented QuickPLS v1.2.3 fixed-score quadratic diagnostic scope; diagnostics use fixed PLS construct scores and centered squared score terms.";

pub(crate) fn validate_archived_recipe_v4_pls_method_identity(
    document: &qpls_project::CanonicalResultDocumentV2,
) -> Result<(), String> {
    let cell = &document.provenance.capability_cell;
    let base = archived_base_capability_cell();
    let nonlinear = archived_nonlinear_capability_cell();
    let is_base = cell == &base;
    let is_nonlinear = cell == &nonlinear;
    let has_nonlinear_artifact = document.provenance.method_version
        == NONLINEAR_EFFECTS_METHOD_VERSION
        || document.provenance.engine_version
            == RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        || document.sections.iter().any(|section| {
            section.id == NONLINEAR_SECTION_ID
                || section
                    .capability_cells
                    .as_ref()
                    .is_some_and(|cells| cells.contains(&nonlinear))
        })
        || document.tables.iter().any(|table| {
            matches!(
                table.id.as_str(),
                NONLINEAR_DIAGNOSTICS_TABLE_ID
                    | NONLINEAR_EQUATION_FIT_TABLE_ID
                    | NONLINEAR_METHOD_SCOPE_TABLE_ID
            ) || table
                .capability_cells
                .as_ref()
                .is_some_and(|cells| cells.contains(&nonlinear))
        })
        || document
            .capability_cells
            .as_ref()
            .is_some_and(|cells| cells.contains(&nonlinear));
    if is_nonlinear {
        return validate_archived_nonlinear_document(document, &base, &nonlinear);
    }
    if has_nonlinear_artifact {
        return Err(
            "archived non-nonlinear document contains injected Recipe-v4 PLS nonlinear artifacts"
                .into(),
        );
    }
    if !is_base {
        return Ok(());
    }
    let has_summary = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2);
    let has_weights = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2);
    let has_attribution = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1);
    let has_convergence = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1);
    let has_block_order = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1);
    let legacy_additions_coherent =
        has_convergence == has_block_order && (!has_convergence || has_attribution);
    let current_families = has_attribution && has_convergence && has_block_order;
    match (
        document.provenance.method_version.as_str(),
        document.provenance.engine_version.as_str(),
    ) {
        (PLS_METHOD_VERSION, qpls_project::LEGACY_RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1)
            if !has_summary && !has_weights && legacy_additions_coherent =>
        {
            Ok(())
        }
        (PLS_METHOD_VERSION, qpls_project::RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1)
            if !has_summary && !has_weights && current_families =>
        {
            Ok(())
        }
        (
            PLS_SCORE_EXECUTION_METHOD_VERSION_V2,
            qpls_project::LEGACY_RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2,
        ) if has_summary && has_weights && legacy_additions_coherent => Ok(()),
        (
            PLS_SCORE_EXECUTION_METHOD_VERSION_V2,
            qpls_project::RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2,
        ) if has_summary && has_weights && current_families => Ok(()),
        (PLS_METHOD_VERSION | PLS_SCORE_EXECUTION_METHOD_VERSION_V2, _) => Err(
            "archived Recipe-v4 PLS method, adapter generation, and typed table identities differ"
                .into(),
        ),
        (method_version, engine_version) => Err(format!(
            "archived Recipe-v4 PLS method identity is unsupported: method={method_version}, engine={engine_version}"
        )),
    }
}

fn archived_base_capability_cell() -> qpls_project::CapabilityCellReferenceV2 {
    qpls_project::CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_ALGORITHM_CAPABILITY_ID.into(),
        cell_id: PLS_ALGORITHM_CELL_ID.into(),
        capability_version: PLS_ALGORITHM_CAPABILITY_VERSION.into(),
    }
}

fn archived_nonlinear_capability_cell() -> qpls_project::CapabilityCellReferenceV2 {
    qpls_project::CapabilityCellReferenceV2 {
        registry_schema_version: 2,
        capability_id: PLS_NONLINEAR_EFFECTS_CAPABILITY_ID.into(),
        cell_id: PLS_NONLINEAR_EFFECTS_CELL_ID.into(),
        capability_version: PLS_NONLINEAR_EFFECTS_CAPABILITY_VERSION.into(),
    }
}

fn validate_archived_nonlinear_document(
    document: &qpls_project::CanonicalResultDocumentV2,
    base: &qpls_project::CapabilityCellReferenceV2,
    nonlinear: &qpls_project::CapabilityCellReferenceV2,
) -> Result<(), String> {
    if document.provenance.method_version != NONLINEAR_EFFECTS_METHOD_VERSION
        || document.provenance.engine_version
            != RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        || document.title != "PLS nonlinear quadratic diagnostics"
        || document.presentation.default_section_id.as_deref() != Some(NONLINEAR_SECTION_ID)
        || document.presentation.default_table_id.as_deref() != Some(NONLINEAR_DIAGNOSTICS_TABLE_ID)
    {
        return Err("archived nonlinear method, adapter, title, or default view differs".into());
    }
    if !document.charts.is_empty()
        || document
            .sections
            .iter()
            .any(|section| !section.chart_ids.is_empty())
    {
        return Err("archived nonlinear v7 document contains an unsupported chart surface".into());
    }
    let capability_cells = document
        .capability_cells
        .as_ref()
        .ok_or_else(|| "archived nonlinear document omitted capability_cells".to_string())?;
    if capability_cells.as_slice() != [nonlinear.clone(), base.clone()].as_slice() {
        return Err(
            "archived nonlinear capability_cells must be ordered [primary nonlinear, base PLS]"
                .into(),
        );
    }
    let section_ids = document
        .sections
        .iter()
        .map(|section| section.id.as_str())
        .collect::<Vec<_>>();
    if section_ids
        != [
            "run_details",
            "measurement_model",
            "structural_model",
            NONLINEAR_SECTION_ID,
        ]
    {
        return Err("archived nonlinear section order or ownership boundary differs".into());
    }
    for section in &document.sections[..3] {
        if section.capability_cells.as_deref() != Some(std::slice::from_ref(base)) {
            return Err("base PLS sections must be owned only by the base PLS cell".into());
        }
    }
    let has_controls = document
        .tables
        .iter()
        .any(|table| table.id == qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    let expected_run_details = [
        "estimation_summary",
        qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
    ];
    let expected_measurement = ["outer_model"];
    let mut expected_structural = vec!["structural_paths", "effects", "r_squared"];
    if has_controls {
        expected_structural.push(qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    }
    if document.sections[0]
        .table_ids
        .iter()
        .map(String::as_str)
        .ne(expected_run_details)
        || document.sections[1]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_measurement)
        || document.sections[2]
            .table_ids
            .iter()
            .map(String::as_str)
            .ne(expected_structural.iter().copied())
    {
        return Err("nonlinear base PLS section table membership or order differs".into());
    }
    let nonlinear_section = &document.sections[3];
    if nonlinear_section.capability_cells.as_deref() != Some(std::slice::from_ref(nonlinear))
        || !nonlinear_section.table_ids.iter().map(String::as_str).eq([
            NONLINEAR_DIAGNOSTICS_TABLE_ID,
            NONLINEAR_EQUATION_FIT_TABLE_ID,
            NONLINEAR_METHOD_SCOPE_TABLE_ID,
        ])
    {
        return Err("nonlinear_relationships has a drifted table order or capability owner".into());
    }
    for table_id in [
        NONLINEAR_DIAGNOSTICS_TABLE_ID,
        NONLINEAR_EQUATION_FIT_TABLE_ID,
        NONLINEAR_METHOD_SCOPE_TABLE_ID,
    ] {
        let references = document
            .sections
            .iter()
            .flat_map(|section| &section.table_ids)
            .filter(|candidate| candidate.as_str() == table_id)
            .count();
        if references != 1 {
            return Err(format!(
                "nonlinear table {table_id} must belong exactly once to nonlinear_relationships"
            ));
        }
    }
    let nonlinear_ids = document
        .tables
        .iter()
        .filter(|table| {
            matches!(
                table.id.as_str(),
                NONLINEAR_DIAGNOSTICS_TABLE_ID
                    | NONLINEAR_EQUATION_FIT_TABLE_ID
                    | NONLINEAR_METHOD_SCOPE_TABLE_ID
            )
        })
        .map(|table| table.id.as_str())
        .collect::<Vec<_>>();
    let table_ids = document
        .tables
        .iter()
        .map(|table| table.id.as_str())
        .collect::<Vec<_>>();
    let mut expected_table_ids = vec![
        "estimation_summary",
        "outer_model",
        "structural_paths",
        "effects",
        "r_squared",
    ];
    if has_controls {
        expected_table_ids.push(qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    }
    expected_table_ids.extend([
        qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
        NONLINEAR_DIAGNOSTICS_TABLE_ID,
        NONLINEAR_EQUATION_FIT_TABLE_ID,
        NONLINEAR_METHOD_SCOPE_TABLE_ID,
    ]);
    if !nonlinear_ids.iter().copied().eq([
        NONLINEAR_DIAGNOSTICS_TABLE_ID,
        NONLINEAR_EQUATION_FIT_TABLE_ID,
        NONLINEAR_METHOD_SCOPE_TABLE_ID,
    ]) || table_ids != expected_table_ids
    {
        return Err("nonlinear tables must occur exactly once at the canonical table tail".into());
    }
    for table in &document.tables {
        let nonlinear_table = matches!(
            table.id.as_str(),
            NONLINEAR_DIAGNOSTICS_TABLE_ID
                | NONLINEAR_EQUATION_FIT_TABLE_ID
                | NONLINEAR_METHOD_SCOPE_TABLE_ID
        );
        let expected = if nonlinear_table { nonlinear } else { base };
        if table.capability_cells.as_deref() != Some(std::slice::from_ref(expected)) {
            return Err(format!(
                "canonical table {} has a drifted capability owner",
                table.id
            ));
        }
    }

    let diagnostics = archived_table(document, NONLINEAR_DIAGNOSTICS_TABLE_ID)?;
    let equations = archived_table(document, NONLINEAR_EQUATION_FIT_TABLE_ID)?;
    let scope = archived_table(document, NONLINEAR_METHOD_SCOPE_TABLE_ID)?;
    archived_exact_columns(diagnostics, NONLINEAR_DIAGNOSTIC_COLUMNS)?;
    archived_exact_columns(equations, NONLINEAR_EQUATION_FIT_COLUMNS)?;
    archived_exact_columns(scope, NONLINEAR_METHOD_SCOPE_COLUMNS)?;
    if diagnostics.rows.is_empty() {
        return Err("nonlinear_quadratic_diagnostics must not be empty".into());
    }

    let structural = archived_table(document, "structural_paths")?;
    archived_exact_columns(structural, &["source", "target", "coefficient"])?;
    let mut structural_coefficients = BTreeMap::new();
    for row in &structural.rows {
        if row.cells.len() != 3 {
            return Err("structural_paths row width is non-canonical".into());
        }
        let source = archived_text(&row.cells[0])?.to_owned();
        let target = archived_text(&row.cells[1])?.to_owned();
        if structural_coefficients
            .insert((target, source), archived_number(&row.cells[2])?)
            .is_some()
        {
            return Err("structural_paths contains duplicate endpoints".into());
        }
    }

    let mut diagnostic_keys = Vec::with_capacity(diagnostics.rows.len());
    let mut targets = BTreeSet::new();
    for (index, row) in diagnostics.rows.iter().enumerate() {
        if row.id != format!("nonlinear_quadratic_diagnostic_{index:04}")
            || row.cells.len() != NONLINEAR_DIAGNOSTIC_COLUMNS.len()
        {
            return Err("nonlinear diagnostic row identity or width is non-canonical".into());
        }
        let source = archived_text(&row.cells[0])?.to_owned();
        let target = archived_text(&row.cells[1])?.to_owned();
        let key = (target.clone(), source.clone());
        if diagnostic_keys
            .last()
            .is_some_and(|previous| previous >= &key)
        {
            return Err(
                "nonlinear diagnostics are not strictly ordered by (target, source)".into(),
            );
        }
        let linear = archived_number(&row.cells[2])?;
        let quadratic = archived_number(&row.cells[3])?;
        let standard_error = archived_number(&row.cells[4])?;
        let t_statistic = archived_number(&row.cells[5])?;
        let p_value = archived_number(&row.cells[6])?;
        archived_optional_warning(&row.cells[7])?;
        if standard_error <= 0.0
            || t_statistic.to_bits() != (quadratic / standard_error).to_bits()
            || !(0.0..=1.0).contains(&p_value)
            || structural_coefficients
                .get(&key)
                .is_none_or(|coefficient| coefficient.to_bits() != linear.to_bits())
        {
            return Err("nonlinear diagnostic numerical invariants differ".into());
        }
        diagnostic_keys.push(key);
        targets.insert(target);
    }
    if diagnostic_keys.len() != structural_coefficients.len()
        || diagnostic_keys
            .iter()
            .any(|key| !structural_coefficients.contains_key(key))
    {
        return Err("nonlinear diagnostic endpoints differ from structural_paths".into());
    }

    if equations.rows.len() != targets.len() {
        return Err("nonlinear equation-fit targets differ from diagnostics".into());
    }
    for (index, (row, expected_target)) in equations.rows.iter().zip(targets).enumerate() {
        if row.id != format!("nonlinear_equation_fit_{index:04}")
            || row.cells.len() != NONLINEAR_EQUATION_FIT_COLUMNS.len()
            || archived_text(&row.cells[0])? != expected_target.as_str()
        {
            return Err("nonlinear equation-fit row identity or target order differs".into());
        }
        let linear = archived_number(&row.cells[1])?;
        let augmented = archived_number(&row.cells[2])?;
        let delta = archived_number(&row.cells[3])?;
        if !(0.0..=1.0).contains(&linear)
            || !(0.0..=1.0).contains(&augmented)
            || delta.to_bits() != (augmented - linear).max(0.0).to_bits()
        {
            return Err("nonlinear equation-fit R-squared invariants differ".into());
        }
    }
    if scope.rows.len() != 1
        || scope.rows[0].id != NONLINEAR_METHOD_SCOPE_TABLE_ID
        || scope.rows[0].cells.len() != NONLINEAR_METHOD_SCOPE_COLUMNS.len()
        || archived_text(&scope.rows[0].cells[0])? != NONLINEAR_EFFECTS_METHOD_VERSION
        || archived_text(&scope.rows[0].cells[1])? != NONLINEAR_TERM_V1
        || archived_text(&scope.rows[0].cells[2])? != NONLINEAR_ENGINE_WARNING_V1
    {
        return Err("nonlinear_method_scope differs from the exact method-v1 contract".into());
    }
    Ok(())
}

fn archived_table<'a>(
    document: &'a qpls_project::CanonicalResultDocumentV2,
    id: &str,
) -> Result<&'a qpls_project::CanonicalResultTableV2, String> {
    let matches = document
        .tables
        .iter()
        .filter(|table| table.id == id)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [table] => Ok(*table),
        _ => Err(format!(
            "canonical result must contain exactly one {id} table"
        )),
    }
}

fn archived_exact_columns(
    table: &qpls_project::CanonicalResultTableV2,
    expected: &[&str],
) -> Result<(), String> {
    if table
        .columns
        .iter()
        .map(|column| column.id.as_str())
        .eq(expected.iter().copied())
    {
        Ok(())
    } else {
        Err(format!("canonical table {} has drifted columns", table.id))
    }
}

fn archived_text(cell: &qpls_project::CanonicalResultCellV2) -> Result<&str, String> {
    match cell {
        qpls_project::CanonicalResultCellV2::Text { value } if !value.trim().is_empty() => {
            Ok(value)
        }
        _ => Err("canonical nonlinear text cell is empty or not text".into()),
    }
}

fn archived_number(cell: &qpls_project::CanonicalResultCellV2) -> Result<f64, String> {
    match cell {
        qpls_project::CanonicalResultCellV2::Number { value, .. } if value.is_finite() => {
            Ok(*value)
        }
        _ => Err("canonical nonlinear number cell is non-finite or not numeric".into()),
    }
}

fn archived_optional_warning(cell: &qpls_project::CanonicalResultCellV2) -> Result<(), String> {
    match cell {
        qpls_project::CanonicalResultCellV2::Text { value } if !value.trim().is_empty() => Ok(()),
        qpls_project::CanonicalResultCellV2::Missing {
            reason: qpls_project::CanonicalMissingReasonV2::NotEstimated,
            display: None,
        } => Ok(()),
        _ => Err("nonlinear diagnostic warning must be text or not_estimated".into()),
    }
}

fn text_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Text,
        description: description.into(),
        role: Some(CanonicalColumnRole::Label),
        unit: None,
        default_precision: None,
    }
}

fn number_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Number,
        description: description.into(),
        role: Some(CanonicalColumnRole::Estimate),
        unit: None,
        default_precision: Some(6),
    }
}

fn boolean_column(id: &str, label: &str, description: &str) -> CanonicalResultColumn {
    CanonicalResultColumn {
        id: id.into(),
        label: label.into(),
        data_type: CanonicalColumnType::Boolean,
        description: description.into(),
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

fn missing() -> CanonicalResultCell {
    CanonicalResultCell::Missing {
        reason: CanonicalMissingReason::NotEstimated,
        display: None,
    }
}

fn not_applicable() -> CanonicalResultCell {
    CanonicalResultCell::Missing {
        reason: CanonicalMissingReason::NotApplicable,
        display: None,
    }
}

fn optional_text(value: Option<&str>) -> CanonicalResultCell {
    value.map_or_else(missing, text)
}

fn optional_number(value: Option<f64>) -> CanonicalResultCell {
    value.map_or_else(missing, number)
}

fn optional_boolean(value: Option<bool>) -> CanonicalResultCell {
    value.map_or_else(missing, boolean)
}

fn score_optional_text(value: Option<&str>) -> CanonicalResultCell {
    value.map_or_else(not_applicable, text)
}

fn posthoc_status(status: &PlsPosthocMinimumSampleSizeStatus) -> &'static str {
    match status {
        PlsPosthocMinimumSampleSizeStatus::Available => "available",
        PlsPosthocMinimumSampleSizeStatus::NotApplicableNoStructuralPath => {
            "not_applicable_no_structural_path"
        }
        PlsPosthocMinimumSampleSizeStatus::InferenceUnavailable => "inference_unavailable",
        PlsPosthocMinimumSampleSizeStatus::InferenceIncomplete => "inference_incomplete",
        PlsPosthocMinimumSampleSizeStatus::NoStatisticallySignificantPath => {
            "no_statistically_significant_path"
        }
        PlsPosthocMinimumSampleSizeStatus::UndefinedZeroPath => "undefined_zero_path",
        PlsPosthocMinimumSampleSizeStatus::ExceedsSupportedIntegerRange => {
            "exceeds_supported_integer_range"
        }
    }
}

fn posthoc_capability_cell() -> Result<CapabilityCellReferenceV2, Vec<String>> {
    const CAPABILITY_ID: &str = "smartpls.pls_power_analysis";
    const CELL_ID: &str = "qpls3.pls.posthoc_technical_minimum_sample_size";
    let registry = CapabilityRegistryV2::embedded()
        .map_err(|error| vec![format!("Capability Registry V2 is invalid: {error}")])?;
    let matches = registry
        .option_cells()
        .filter(|cell| cell.capability_id == CAPABILITY_ID && cell.cell_id == CELL_ID)
        .collect::<Vec<_>>();
    let [cell] = matches.as_slice() else {
        return Err(vec![format!(
            "expected one exact post-hoc technical sample-size option cell, found {}",
            matches.len()
        )]);
    };
    cell.qualification_spec
        .links
        .as_slice()
        .first()
        .cloned()
        .ok_or_else(|| {
            vec!["post-hoc technical sample-size option cell has no qualification link".into()]
        })
}

fn posthoc_table(
    result: &PlsPosthocMinimumSampleSize,
    capability_cell: CapabilityCellReferenceV2,
) -> CanonicalResultTable {
    CanonicalResultTable {
        id: "posthoc_minimum_sample_size".into(),
        title: "Post-hoc minimum sample size".into(),
        description: Some(
            "Retrospective inverse-square-root technical sample-size diagnostic. The formula uses a directional-test assumption; significance-aware driver selection uses the separately recorded two-sided bootstrap probability contract."
                .into(),
        ),
        columns: vec![
            text_column("status", "Status", "Typed availability or boundary state."),
            text_column("method_version", "Method version", "Frozen analytical method identity."),
            text_column("formula_test", "Formula test", "Test-direction assumption used by the inverse-square-root constant."),
            number_column("alpha", "Formula alpha", "Significance level embedded in the inverse-square-root constant."),
            number_column("power", "Formula power", "Statistical power embedded in the inverse-square-root constant."),
            number_column("constant", "Formula constant", "Inverse-square-root constant."),
            text_column("selection_rule", "Driver selection", "Rule used to select the path coefficient that drives the calculation."),
            text_column("significance_source", "Selection inference", "Inference source used only for statistically significant path selection."),
            number_column("significance_alpha", "Selection alpha", "Two-sided bootstrap probability threshold used for driver selection."),
            number_column("eligible_paths", "Eligible paths", "Number of structural paths eligible for selection."),
            number_column("significant_paths", "Significant paths", "Number of structural paths meeting the recorded selection threshold."),
            text_column("driver_source", "Driver source", "Source construct of the selected path."),
            text_column("driver_target", "Driver target", "Target construct of the selected path."),
            number_column("driver_p_two_sided", "Driver p value (two-sided)", "Two-sided bootstrap probability of the selected path."),
            number_column("absolute_coefficient", "Absolute coefficient", "Absolute magnitude of the selected path coefficient."),
            number_column("required_sample_size", "Technically required sample size", "Ceiling of (2.486 divided by the absolute driver coefficient) squared."),
            number_column("analytical_sample_size", "Analytical sample size", "Valid observations used by the linked PLS estimate."),
            boolean_column("meets_requirement", "Meets technical requirement", "Whether the analytical sample is at least the calculated technical requirement."),
            text_column("caution", "Interpretation caution", "Required interpretation boundary for the retrospective diagnostic."),
        ],
        rows: vec![CanonicalResultRow {
            id: "technical_minimum".into(),
            cells: vec![
                text(posthoc_status(&result.status)),
                text(result.method_version.clone()),
                text(result.test.clone()),
                number(result.alpha),
                number(result.power),
                number(result.inverse_square_root_constant),
                text(result.selection_rule.clone()),
                optional_text(result.significance_source.as_deref()),
                optional_number(result.significance_alpha),
                number(result.eligible_path_count as f64),
                optional_number(result.significant_path_count.map(|value| value as f64)),
                optional_text(result.driver_source.as_deref()),
                optional_text(result.driver_target.as_deref()),
                optional_number(result.driver_p_value_two_sided),
                optional_number(result.minimum_absolute_path_coefficient),
                optional_number(result.technically_required_sample_size.map(|value| value as f64)),
                number(result.analytical_sample_size as f64),
                optional_boolean(result.meets_technical_requirement),
                text(result.caution.clone()),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: Some(vec![capability_cell]),
    }
}

fn recorded_sha256(value: &str) -> Option<String> {
    let candidate = value.rsplit_once(':').map_or(value, |(_, suffix)| suffix);
    (candidate.len() == 64
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    .then(|| candidate.to_owned())
}

fn score_execution_tables(
    request: &InternalRecipeV4PlsExecutionRequestV1,
    estimation: &PlsResult,
    capability_cells: Option<Vec<CapabilityCellReferenceV2>>,
) -> Result<
    Option<(
        CanonicalResultTable,
        CanonicalResultTable,
        Option<CanonicalResultTable>,
    )>,
    Vec<String>,
> {
    let Some(execution) = estimation.score_execution.as_ref() else {
        if !matches!(
            estimation.method_version.as_str(),
            PLS_METHOD_VERSION | NONLINEAR_EFFECTS_METHOD_VERSION
        ) {
            return Err(vec![format!(
                "PLS result method {} omitted its typed score-execution contract",
                estimation.method_version
            )]);
        }
        return Ok(None);
    };
    if !matches!(
        estimation.method_version.as_str(),
        PLS_SCORE_EXECUTION_METHOD_VERSION_V2 | NONLINEAR_EFFECTS_METHOD_VERSION
    ) || execution.contract_version != PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2
    {
        return Err(vec![
            "typed score-execution payload has a drifted method or contract identity".into(),
        ]);
    }
    let artifact = compile_analysis_recipe_v4(
        &request.recipe,
        Some(&request.model),
        request.compiler_target,
        request.capability_cell.clone(),
    )
    .map_err(|error| {
        vec![format!(
            "score-execution plan recompilation failed: {error}"
        )]
    })?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        return Err(vec![
            "score-execution canonical projection requires a compiled PLS plan".into(),
        ]);
    };
    if execution.blocks.len() != plan.blocks().len() {
        return Err(vec![
            "typed score-execution block count differs from the compiled PLS plan".into(),
        ]);
    }

    let mut outer_weights = BTreeMap::new();
    for estimate in &estimation.outer_estimates {
        if !estimate.weight.is_finite()
            || outer_weights
                .insert(
                    (estimate.construct.clone(), estimate.indicator.clone()),
                    estimate.weight,
                )
                .is_some()
        {
            return Err(vec![
                "outer estimates contain a duplicate identity or non-finite score weight".into(),
            ]);
        }
    }

    let accounting = &execution.iteration_accounting;
    let summary = CanonicalResultTable {
        id: qpls_project::PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2.into(),
        title: "Score execution".into(),
        description: Some(
            "Resolved PLS score initialization, fixed-scoring, and iteration accounting contract."
                .into(),
        ),
        columns: vec![
            text_column(
                "contract_version",
                "Contract version",
                "Frozen resolved score-execution contract identity.",
            ),
            number_column(
                "maximum_iterations",
                "Maximum iterations",
                "Configured maximum outer iterations.",
            ),
            number_column(
                "stop_criterion",
                "Stop criterion",
                "Configured outer-weight convergence threshold.",
            ),
            number_column(
                "estimated_block_count",
                "Estimated blocks",
                "Number of blocks whose outer weights are iteratively estimated.",
            ),
            number_column(
                "fixed_block_count",
                "Fixed blocks",
                "Number of blocks whose scoring weights remain fixed.",
            ),
            number_column(
                "performed_iterations",
                "Performed iterations",
                "Number of outer iterations performed.",
            ),
            number_column(
                "estimated_block_updates",
                "Estimated block updates",
                "Performed iterations multiplied by estimated block count.",
            ),
        ],
        rows: vec![CanonicalResultRow {
            id: "execution".into(),
            cells: vec![
                text(execution.contract_version.clone()),
                number(f64::from(accounting.maximum_iterations)),
                number(accounting.stop_criterion),
                number(accounting.estimated_block_count as f64),
                number(accounting.fixed_block_count as f64),
                number(f64::from(accounting.performed_iterations)),
                number(accounting.estimated_block_updates as f64),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let mut rows = Vec::new();
    for (block, resolved_block) in plan.blocks().iter().zip(&execution.blocks) {
        let expected_indicator_ids = block
            .indicators()
            .iter()
            .map(|indicator| indicator.variable_id())
            .collect::<Vec<_>>();
        if resolved_block.construct_id != block.construct_id()
            || resolved_block
                .indicator_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                != expected_indicator_ids
        {
            return Err(vec![
                "typed score-execution stable block identities differ from the compiled PLS plan"
                    .into(),
            ]);
        }
        let (block_kind, estimated_mode, initialization, normalization, requested, resolved) =
            match &resolved_block.scoring {
                PlsResolvedScoreBlockKindV2::Estimated {
                    mode,
                    requested_initialization,
                    resolved_initial_weights,
                } => {
                    let (initialization, requested_weights) = match requested_initialization {
                        PlsResolvedInitialOuterWeightsV2::Standard { weights } => {
                            ("standard", weights)
                        }
                        PlsResolvedInitialOuterWeightsV2::Individual { weights } => {
                            ("individual", weights)
                        }
                    };
                    (
                        "estimated",
                        Some(match mode {
                            PlsEstimatedScoreModeV2::ModeA => "mode_a",
                            PlsEstimatedScoreModeV2::ModeB => "mode_b",
                        }),
                        Some(initialization),
                        None,
                        exact_score_weights(requested_weights, &resolved_block.indicator_ids)?,
                        exact_score_weights(
                            resolved_initial_weights,
                            &resolved_block.indicator_ids,
                        )?,
                    )
                }
                PlsResolvedScoreBlockKindV2::FixedUnit {
                    normalization,
                    requested_weights,
                    resolved_effective_weights,
                } => (
                    "fixed_unit",
                    None,
                    None,
                    Some(score_normalization(*normalization)?),
                    exact_score_weights(requested_weights, &resolved_block.indicator_ids)?,
                    exact_score_weights(resolved_effective_weights, &resolved_block.indicator_ids)?,
                ),
                PlsResolvedScoreBlockKindV2::FixedCustom {
                    normalization,
                    requested_weights,
                    resolved_effective_weights,
                } => (
                    "fixed_custom",
                    None,
                    None,
                    Some(score_normalization(*normalization)?),
                    exact_score_weights(requested_weights, &resolved_block.indicator_ids)?,
                    exact_score_weights(resolved_effective_weights, &resolved_block.indicator_ids)?,
                ),
            };
        for indicator in block.indicators() {
            let stable_id = indicator.variable_id();
            let final_weight = outer_weights
                .get(&(
                    block.construct_id().to_owned(),
                    indicator.source_column().to_owned(),
                ))
                .copied()
                .ok_or_else(|| {
                    vec![format!(
                        "outer estimates omitted compiled score indicator {}:{}",
                        block.construct_id(),
                        indicator.source_column()
                    )]
                })?;
            rows.push(CanonicalResultRow {
                id: format!("score_weight_{:04}", rows.len()),
                cells: vec![
                    text(block.construct_id()),
                    text(stable_id),
                    text(block_kind),
                    score_optional_text(estimated_mode),
                    score_optional_text(initialization),
                    score_optional_text(normalization),
                    number(requested[stable_id]),
                    number(resolved[stable_id]),
                    number(final_weight),
                ],
            });
        }
    }
    let weights = CanonicalResultTable {
        id: qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2.into(),
        title: "Score weights".into(),
        description: Some(
            "Requested, resolved starting or fixed, and final outer weights by stable model identity."
                .into(),
        ),
        columns: vec![
            text_column("construct_id", "Construct", "Stable construct identifier."),
            text_column("indicator_id", "Indicator", "Stable SemModel indicator identifier."),
            text_column("block_kind", "Block kind", "Estimated or fixed scoring contract."),
            text_column("estimated_mode", "Estimated mode", "Mode A or Mode B when estimated."),
            text_column(
                "requested_initialization",
                "Requested initialization",
                "Standard or individual initialization when estimated.",
            ),
            text_column(
                "normalization",
                "Normalization",
                "Fixed-score normalization when applicable.",
            ),
            number_column("requested_weight", "Requested weight", "Requested initial or fixed weight."),
            number_column(
                "resolved_initial_or_fixed_weight",
                "Resolved initial or fixed weight",
                "Effective starting or fixed score weight after normalization.",
            ),
            number_column(
                "final_outer_weight",
                "Final outer weight",
                "Converged estimated weight or unchanged fixed effective weight.",
            ),
        ],
        rows,
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let fixed_scale = estimation
        .fixed_score_scale_receipt
        .as_ref()
        .map(|receipt| {
            if receipt.contract_version != PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1 {
                return Err(vec!["fixed-score scale receipt contract drifted".into()]);
            }
            let mut rows = Vec::new();
            for block in &receipt.blocks {
                if !block.pre_standardization_center.is_finite()
                    || !block.pre_standardization_scale.is_finite()
                    || block.pre_standardization_scale <= f64::EPSILON
                    || block.indicator_ids.len() != block.effective_unit_score_weights.len()
                {
                    return Err(vec!["fixed-score scale receipt is nonfinite or incomplete".into()]);
                }
                let resolved_block = execution
                    .blocks
                    .iter()
                    .find(|candidate| candidate.construct_id == block.construct_id)
                    .ok_or_else(|| vec!["fixed-score scale block is unknown".into()])?;
                let resolved = match &resolved_block.scoring {
                    PlsResolvedScoreBlockKindV2::FixedUnit {
                        resolved_effective_weights,
                        ..
                    }
                    | PlsResolvedScoreBlockKindV2::FixedCustom {
                        resolved_effective_weights,
                        ..
                    } => resolved_effective_weights,
                    _ => return Err(vec!["fixed-score scale block is not fixed".into()]),
                };
                if block.indicator_ids != resolved_block.indicator_ids
                    || resolved.len() != block.indicator_ids.len()
                {
                    return Err(vec!["fixed-score scale block coverage drifted".into()]);
                }
                for ((indicator_id, effective), coefficient) in block
                    .indicator_ids
                    .iter()
                    .zip(&block.effective_unit_score_weights)
                    .zip(resolved)
                {
                    if effective.indicator_id != *indicator_id
                        || coefficient.indicator_id != *indicator_id
                        || effective.value.to_bits()
                            != (coefficient.value / block.pre_standardization_scale).to_bits()
                    {
                        return Err(vec!["fixed-score scale arithmetic or order drifted".into()]);
                    }
                    rows.push(CanonicalResultRow {
                        id: format!("fixed_score_scale_{:04}", rows.len()),
                        cells: vec![
                            text(PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1),
                            text(block.construct_id.clone()),
                            text(indicator_id.clone()),
                            number(block.pre_standardization_center),
                            number(block.pre_standardization_scale),
                            number(coefficient.value),
                            number(effective.value),
                        ],
                    });
                }
            }
            Ok(CanonicalResultTable {
                id: qpls_project::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1.into(),
                title: "Fixed-score scale receipt".into(),
                description: Some("Exact fixed scoring coefficients and the subsequent centering/scaling used for emitted unit-variance construct scores.".into()),
                columns: vec![
                    text_column("contract_version", "Contract", "Scale receipt contract."),
                    text_column("construct_id", "Construct", "Stable construct identifier."),
                    text_column("indicator_id", "Indicator", "Stable indicator identifier."),
                    number_column("pre_standardization_center", "Score center", "Center subtracted from the fixed linear score."),
                    number_column("pre_standardization_scale", "Score scale", "Sample-SD divisor applied to the fixed linear score."),
                    number_column("resolved_scoring_coefficient", "Scoring coefficient", "Resolved fixed coefficient before score standardization."),
                    number_column("effective_unit_score_weight", "Effective unit-score weight", "Coefficient divided by the score scale."),
                ],
                rows,
                footnote_ids: Vec::new(),
                capability_cells: capability_cells.clone(),
            })
        })
        .transpose()?;
    Ok(Some((summary, weights, fixed_scale)))
}

fn exact_score_weights(
    weights: &[PlsResolvedScoreWeightV2],
    indicator_ids: &[String],
) -> Result<BTreeMap<String, f64>, Vec<String>> {
    if weights.len() != indicator_ids.len()
        || weights
            .iter()
            .zip(indicator_ids)
            .any(|(weight, expected)| weight.indicator_id != *expected || !weight.value.is_finite())
    {
        return Err(vec![
            "typed score weights do not exactly cover their stable indicator identities".into(),
        ]);
    }
    Ok(weights
        .iter()
        .map(|weight| (weight.indicator_id.clone(), weight.value))
        .collect())
}

fn score_normalization(
    normalization: CompositeWeightNormalizationV4,
) -> Result<&'static str, Vec<String>> {
    match normalization {
        CompositeWeightNormalizationV4::None => Ok("none"),
        CompositeWeightNormalizationV4::SumToOne => Ok("sum_to_one"),
        CompositeWeightNormalizationV4::UnitVariance => Ok("unit_variance"),
    }
}

fn preprocessing_name(preprocessing: &Preprocessing) -> &'static str {
    match preprocessing {
        Preprocessing::Standardized => "standardized",
        Preprocessing::MeanCentered => "mean_centered",
        Preprocessing::Unstandardized => "unstandardized",
    }
}

fn weighting_scheme_name(weighting_scheme: &WeightingScheme) -> &'static str {
    match weighting_scheme {
        WeightingScheme::Path => "path",
        WeightingScheme::Factor => "factor",
        WeightingScheme::Pca => "pca",
    }
}

fn nonlinear_capability_cell() -> CapabilityCellReferenceV2 {
    RecipeV4CompilerTarget::PlsPlanV2.capability_cell_for_method(AnalysisMethod::NonlinearEffects)
}

fn nonlinear_tables(
    request: &InternalRecipeV4PlsExecutionRequestV1,
    result: &RecipeV4PlsExecutionResultV1,
) -> Result<Option<[CanonicalResultTable; 3]>, Vec<String>> {
    let estimation = result.estimation();
    let requested = request.recipe.settings.method == AnalysisMethod::NonlinearEffects
        && matches!(
            request.recipe.method_config.as_ref(),
            Some(MethodConfig::NonlinearEffects)
        );
    if !requested {
        if estimation.nonlinear_effects.is_some()
            || estimation.method_version == NONLINEAR_EFFECTS_METHOD_VERSION
            || result.provenance().adapter_version()
                == RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        {
            return Err(vec![
                "nonlinear payload or v7 identity was injected into a non-nonlinear Recipe-v4 result"
                    .into(),
            ]);
        }
        return Ok(None);
    }
    if request.posthoc_technical_minimum_sample_size.is_some()
        || request.capability_cell != nonlinear_capability_cell()
        || result.provenance().compilation_receipt().capability_cell()
            != &nonlinear_capability_cell()
        || result.provenance().adapter_version()
            != RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        || result.provenance().estimator_method_version() != NONLINEAR_EFFECTS_METHOD_VERSION
        || estimation.method_version != NONLINEAR_EFFECTS_METHOD_VERSION
    {
        return Err(vec![
            "Recipe-v4 nonlinear request, primary cell, adapter, and estimator identity differ"
                .into(),
        ]);
    }
    let analysis = estimation.nonlinear_effects.as_ref().ok_or_else(|| {
        vec!["Recipe-v4 nonlinear result omitted its typed nonlinear payload".into()]
    })?;
    if analysis.method_version != NONLINEAR_EFFECTS_METHOD_VERSION
        || analysis.term != NONLINEAR_TERM_V1
        || analysis.warnings.len() != 1
        || analysis.warnings[0] != NONLINEAR_ENGINE_WARNING_V1
        || analysis.estimates.is_empty()
    {
        return Err(vec![
            "typed nonlinear payload has a drifted method, term, warning, or empty estimate family"
                .into(),
        ]);
    }
    let artifact = compile_analysis_recipe_v4(
        &request.recipe,
        Some(&request.model),
        request.compiler_target,
        request.capability_cell.clone(),
    )
    .map_err(|error| vec![format!("nonlinear plan recompilation failed: {error}")])?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        return Err(vec![
            "nonlinear canonical projection requires a compiled PLS plan".into(),
        ]);
    };
    let mut expected_paths = plan
        .paths()
        .iter()
        .map(|path| (path.target().to_owned(), path.source().to_owned()))
        .collect::<Vec<_>>();
    expected_paths.sort();
    if expected_paths.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(vec![
            "compiled PLS plan contains duplicate nonlinear endpoint identities".into(),
        ]);
    }
    let path_coefficients = estimation
        .paths
        .iter()
        .map(|path| ((path.target.clone(), path.source.clone()), path.coefficient))
        .collect::<BTreeMap<_, _>>();
    if path_coefficients.len() != estimation.paths.len() {
        return Err(vec![
            "PLS structural paths contain duplicate nonlinear endpoint identities".into(),
        ]);
    }

    let mut estimates = analysis.estimates.clone();
    estimates
        .sort_by(|left, right| (&left.target, &left.source).cmp(&(&right.target, &right.source)));
    let actual_paths = estimates
        .iter()
        .map(|estimate| (estimate.target.clone(), estimate.source.clone()))
        .collect::<Vec<_>>();
    if actual_paths != expected_paths {
        return Err(vec![
            "nonlinear diagnostic endpoints differ from the compiled PLS path family".into(),
        ]);
    }
    let mut equations = BTreeMap::<String, (f64, f64, f64)>::new();
    let mut diagnostic_rows = Vec::with_capacity(estimates.len());
    for (index, estimate) in estimates.into_iter().enumerate() {
        let key = (estimate.target.clone(), estimate.source.clone());
        let finite = [
            estimate.linear_coefficient,
            estimate.quadratic_coefficient,
            estimate.standard_error,
            estimate.t_statistic,
            estimate.p_value_two_sided,
            estimate.linear_r_squared,
            estimate.augmented_r_squared,
            estimate.delta_r_squared,
        ]
        .into_iter()
        .all(f64::is_finite);
        if !finite
            || estimate.standard_error <= 0.0
            || estimate.t_statistic.to_bits()
                != (estimate.quadratic_coefficient / estimate.standard_error).to_bits()
            || !(0.0..=1.0).contains(&estimate.p_value_two_sided)
            || !(0.0..=1.0).contains(&estimate.linear_r_squared)
            || !(0.0..=1.0).contains(&estimate.augmented_r_squared)
            || estimate.delta_r_squared.to_bits()
                != (estimate.augmented_r_squared - estimate.linear_r_squared)
                    .max(0.0)
                    .to_bits()
            || estimate
                .warning
                .as_deref()
                .is_some_and(|warning| warning.trim().is_empty())
            || path_coefficients.get(&key).is_none_or(|coefficient| {
                coefficient.to_bits() != estimate.linear_coefficient.to_bits()
            })
        {
            return Err(vec![
                "nonlinear diagnostic numerical or structural invariants differ".into(),
            ]);
        }
        let equation = (
            estimate.linear_r_squared,
            estimate.augmented_r_squared,
            estimate.delta_r_squared,
        );
        if equations.get(&estimate.target).is_some_and(|existing| {
            existing.0.to_bits() != equation.0.to_bits()
                || existing.1.to_bits() != equation.1.to_bits()
                || existing.2.to_bits() != equation.2.to_bits()
        }) {
            return Err(vec![
                "nonlinear estimates disagree about their shared target equation fit".into(),
            ]);
        }
        equations.insert(estimate.target.clone(), equation);
        diagnostic_rows.push(CanonicalResultRow {
            id: format!("nonlinear_quadratic_diagnostic_{index:04}"),
            cells: vec![
                text(estimate.source),
                text(estimate.target),
                number(estimate.linear_coefficient),
                number(estimate.quadratic_coefficient),
                number(estimate.standard_error),
                number(estimate.t_statistic),
                number(estimate.p_value_two_sided),
                optional_text(estimate.warning.as_deref()),
            ],
        });
    }
    let primary_owner = Some(vec![nonlinear_capability_cell()]);
    let diagnostics = CanonicalResultTable {
        id: NONLINEAR_DIAGNOSTICS_TABLE_ID.into(),
        title: "Quadratic diagnostics".into(),
        description: Some(
            "Fixed-score centered-quadratic diagnostics ordered by target then source.".into(),
        ),
        columns: vec![
            text_column("source", "Source", "Predictor construct identifier."),
            text_column("target", "Target", "Outcome construct identifier."),
            number_column(
                "linear_coefficient",
                "Linear",
                "Base linear path coefficient.",
            ),
            number_column(
                "quadratic_coefficient",
                "Quadratic",
                "Centered squared-score coefficient.",
            ),
            number_column(
                "standard_error",
                "Standard error",
                "Quadratic-term standard error.",
            ),
            number_column("t_statistic", "t statistic", "Quadratic-term t statistic."),
            number_column(
                "p_value_two_sided",
                "p value",
                "Two-sided quadratic-term p value.",
            ),
            text_column("warning", "Warning", "Optional estimate-level warning."),
        ],
        rows: diagnostic_rows,
        footnote_ids: Vec::new(),
        capability_cells: primary_owner.clone(),
    };
    let equation_fit = CanonicalResultTable {
        id: NONLINEAR_EQUATION_FIT_TABLE_ID.into(),
        title: "Nonlinear equation fit".into(),
        description: Some("Linear and quadratic fixed-score equation fit by target.".into()),
        columns: vec![
            text_column("target", "Target", "Outcome construct identifier."),
            number_column(
                "linear_r_squared",
                "Linear R-squared",
                "Linear equation R-squared.",
            ),
            number_column(
                "augmented_r_squared",
                "Augmented R-squared",
                "R-squared after centered squared-score terms are added.",
            ),
            number_column(
                "delta_r_squared",
                "Delta R-squared",
                "Nonnegative augmented-minus-linear R-squared change.",
            ),
        ],
        rows: equations
            .into_iter()
            .enumerate()
            .map(
                |(index, (target, (linear, augmented, delta)))| CanonicalResultRow {
                    id: format!("nonlinear_equation_fit_{index:04}"),
                    cells: vec![
                        text(target),
                        number(linear),
                        number(augmented),
                        number(delta),
                    ],
                },
            )
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: primary_owner.clone(),
    };
    let method_scope = CanonicalResultTable {
        id: NONLINEAR_METHOD_SCOPE_TABLE_ID.into(),
        title: "Nonlinear method scope".into(),
        description: Some("Exact bounded method, term, and engine-level scope warning.".into()),
        columns: vec![
            text_column(
                "method_version",
                "Method",
                "Exact nonlinear method version.",
            ),
            text_column("term", "Term", "Exact nonlinear basis term."),
            text_column(
                "warning",
                "Warning",
                "Sole engine-level method-scope warning.",
            ),
        ],
        rows: vec![CanonicalResultRow {
            id: NONLINEAR_METHOD_SCOPE_TABLE_ID.into(),
            cells: vec![
                text(analysis.method_version.clone()),
                text(analysis.term.clone()),
                text(analysis.warnings[0].clone()),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: primary_owner,
    };
    Ok(Some([diagnostics, equation_fit, method_scope]))
}

pub(crate) fn build_recipe_v4_pls_canonical_result(
    job_id: Uuid,
    project_id: Uuid,
    started_at: &str,
    completed_at: &str,
    request: &InternalRecipeV4PlsExecutionRequestV1,
    result: &RecipeV4PlsExecutionResultV1,
) -> Result<CanonicalResultDocumentV2, Vec<String>> {
    let receipt = result.provenance().compilation_receipt();
    let dataset_fingerprint = recorded_sha256(receipt.dataset_fingerprint())
        .ok_or_else(|| vec!["dataset fingerprint is not a recorded lowercase SHA-256".into()])?;
    if request.recipe.settings.seed > JAVASCRIPT_MAX_SAFE_INTEGER {
        return Err(vec![
            "recipe seed exceeds the canonical result safe-integer boundary".into(),
        ]);
    }
    let workers = i64::try_from(request.recipe.settings.workers)
        .ok()
        .filter(|workers| *workers > 0)
        .ok_or_else(|| {
            vec!["recipe worker count is outside the canonical result boundary".into()]
        })?;
    let nonlinear_tables = nonlinear_tables(request, result)?;
    let nonlinear_result = nonlinear_tables.is_some();
    let provenance_capability_cell = receipt.capability_cell().clone();
    let capability_cell = if nonlinear_result {
        RecipeV4CompilerTarget::PlsPlanV2.capability_cell()
    } else {
        provenance_capability_cell.clone()
    };
    if request.posthoc_technical_minimum_sample_size.as_ref()
        != result.provenance().posthoc_technical_minimum_sample_size()
    {
        return Err(vec![
            "Recipe-v4 PLS post-hoc opt-in differs from immutable runner provenance".into(),
        ]);
    }
    let estimation = result.estimation();
    if nonlinear_result && estimation.posthoc_minimum_sample_size.is_some() {
        return Err(vec![
            "Recipe-v4 nonlinear result unexpectedly contains a post-hoc analytical payload".into(),
        ]);
    }
    let expected_attribution = PlsPointEstimateAttributionV1::for_preprocessing(
        request.recipe.settings.preprocessing.clone(),
    );
    let attribution = estimation
        .point_estimate_attribution
        .as_ref()
        .filter(|attribution| *attribution == &expected_attribution)
        .ok_or_else(|| {
            vec![
                "Recipe-v4 PLS result omitted or changed its exact point-estimate attribution"
                    .into(),
            ]
        })?;
    let posthoc = match (
        request.posthoc_technical_minimum_sample_size.as_ref(),
        estimation.posthoc_minimum_sample_size.as_ref(),
    ) {
        (None, None) => None,
        (Some(config), Some(technical)) => {
            let registry_cell = posthoc_capability_cell()?;
            if !config.is_exact_v2()
                || !config.has_coherent_base_and_inference()
                || config.capability_cell != registry_cell
                || config.method_version != technical.method_version
            {
                return Err(vec![
                    "Recipe-v4 PLS post-hoc option, registry cell, provenance, and payload versions differ"
                        .into(),
                ]);
            }
            Some((technical, registry_cell))
        }
        (None, Some(_)) => {
            return Err(vec![
                "Recipe-v4 PLS result contains a post-hoc payload without an explicit opt-in"
                    .into(),
            ]);
        }
        (Some(_), None) => {
            return Err(vec![
                "Recipe-v4 PLS result omitted its explicitly requested post-hoc payload".into(),
            ]);
        }
    };
    let mut all_capability_cells =
        vec![capability_cell.clone(), provenance_capability_cell.clone()];
    if let Some((_, posthoc_cell)) = &posthoc {
        all_capability_cells.push(posthoc_cell.clone());
    }
    all_capability_cells.sort_by(|left, right| {
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
    all_capability_cells.dedup();
    let capability_cells = Some(vec![capability_cell.clone()]);
    let structural_capability_cells = if nonlinear_result {
        capability_cells.clone()
    } else {
        Some(all_capability_cells.clone())
    };

    let summary = CanonicalResultTable {
        id: "estimation_summary".into(),
        title: "Estimation summary".into(),
        description: Some("Core convergence and observation accounting for this run.".into()),
        columns: vec![
            boolean_column(
                "converged",
                "Converged",
                "Whether the production PLS estimator converged.",
            ),
            number_column(
                "iterations",
                "Iterations",
                "Number of outer iterations completed.",
            ),
            number_column(
                "used_observations",
                "Used observations",
                "Rows retained for estimation.",
            ),
            number_column(
                "omitted_observations",
                "Omitted observations",
                "Rows omitted by the configured missing-data policy.",
            ),
        ],
        rows: vec![CanonicalResultRow {
            id: "run".into(),
            cells: vec![
                boolean(estimation.converged),
                number(f64::from(estimation.iterations)),
                number(estimation.used_observations as f64),
                number(estimation.omitted_observations as f64),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let point_estimate_attribution = CanonicalResultTable {
        id: qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1.into(),
        title: "Point-estimate attribution".into(),
        description: Some(
            "Exact preprocessing and scale interpretation for every PLS point-estimate family."
                .into(),
        ),
        columns: vec![
            text_column(
                "contract_version",
                "Contract",
                "Attribution contract version.",
            ),
            text_column(
                "preprocessing",
                "Preprocessing",
                "Requested indicator preprocessing.",
            ),
            text_column(
                "indicator_centering",
                "Centering",
                "Indicator centering operation.",
            ),
            text_column("indicator_scaling", "Scaling", "Indicator scale operation."),
            text_column(
                "outer_weights",
                "Outer weights",
                "Outer-weight scale attribution.",
            ),
            text_column(
                "outer_loadings",
                "Outer loadings",
                "Outer-loading scale attribution.",
            ),
            text_column(
                "construct_scores",
                "Construct scores",
                "Construct-score scale attribution.",
            ),
            text_column(
                "structural_paths",
                "Structural paths",
                "Structural-path scale attribution.",
            ),
            text_column(
                "effects",
                "Effects",
                "Direct, indirect, and total-effect scale attribution.",
            ),
        ],
        rows: vec![CanonicalResultRow {
            id: "attribution".into(),
            cells: vec![
                text(attribution.contract_version.clone()),
                text(preprocessing_name(&attribution.preprocessing)),
                text(attribution.indicator_centering.as_str()),
                text(attribution.indicator_scaling.as_str()),
                text(attribution.outer_weights.as_str()),
                text(attribution.outer_loadings.as_str()),
                text(attribution.construct_scores.as_str()),
                text(attribution.structural_paths.as_str()),
                text(attribution.effects.as_str()),
            ],
        }],
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    if nonlinear_result && estimation.algorithm_convergence_receipt.is_some() {
        return Err(vec![
            "Recipe-v4 nonlinear result unexpectedly contains a PlsPm-only convergence receipt"
                .into(),
        ]);
    }
    let algorithm_convergence_tables = match (
        &request.recipe.settings.weighting_scheme,
        estimation.algorithm_convergence_receipt.as_ref(),
    ) {
        (WeightingScheme::Path | WeightingScheme::Factor, Some(receipt))
            if receipt.contract_version
                == PLS_ALGORITHM_CONVERGENCE_RECEIPT_CONTRACT_VERSION_V1 =>
        {
            let summary = CanonicalResultTable {
                id: qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1.into(),
                title: "PLS algorithm convergence".into(),
                description: Some(
                    "Exact stopping rule, observed termination, and iteration accounting.".into(),
                ),
                columns: vec![
                    text_column(
                        "contract_version",
                        "Contract",
                        "Convergence receipt version.",
                    ),
                    text_column("weighting_scheme", "Weighting", "Inner weighting scheme."),
                    number_column("maximum_iterations", "Maximum iterations", "Iteration cap."),
                    number_column(
                        "stop_criterion",
                        "Tolerance",
                        "Outer-weight stop tolerance.",
                    ),
                    text_column(
                        "comparison",
                        "Comparison",
                        "Convergence comparison operator.",
                    ),
                    number_column(
                        "performed_iterations",
                        "Performed",
                        "Outer iterations performed.",
                    ),
                    number_column(
                        "estimated_block_updates",
                        "Block updates",
                        "Estimated block updates performed.",
                    ),
                    text_column(
                        "termination_reason",
                        "Termination",
                        "Observed termination reason.",
                    ),
                    number_column(
                        "final_max_outer_weight_change",
                        "Final change",
                        "Maximum absolute outer-weight change in the terminal iteration.",
                    ),
                ],
                rows: vec![CanonicalResultRow {
                    id: "convergence".into(),
                    cells: vec![
                        text(receipt.contract_version.clone()),
                        text(weighting_scheme_name(&receipt.weighting_scheme)),
                        number(f64::from(receipt.maximum_iterations)),
                        number(receipt.stop_criterion),
                        text(receipt.comparison.as_str()),
                        number(f64::from(receipt.performed_iterations)),
                        number(receipt.estimated_block_updates as f64),
                        text(receipt.termination_reason.as_str()),
                        receipt
                            .final_max_outer_weight_change
                            .map_or_else(not_applicable, number),
                    ],
                }],
                footnote_ids: Vec::new(),
                capability_cells: capability_cells.clone(),
            };
            let rows = receipt
                .blocks
                .iter()
                .enumerate()
                .flat_map(|(block_index, block)| {
                    block.indicator_order.iter().enumerate().map(
                        move |(indicator_index, indicator)| CanonicalResultRow {
                            id: format!(
                                "algorithm_block_{block_index:04}_indicator_{indicator_index:04}"
                            ),
                            cells: vec![
                                number(block_index as f64),
                                text(block.construct_id.clone()),
                                number(indicator_index as f64),
                                text(indicator.clone()),
                                text(block.update_rule.as_str()),
                                text(block.initialization.as_str()),
                            ],
                        },
                    )
                })
                .collect();
            let blocks = CanonicalResultTable {
                id: qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1.into(),
                title: "PLS algorithm block order".into(),
                description: Some(
                    "Deterministic construct and indicator iteration order with exact Mode A, Mode B, or fixed update semantics."
                        .into(),
                ),
                columns: vec![
                    number_column("block_ordinal", "Block order", "Zero-based block iteration order."),
                    text_column("construct_id", "Construct", "Compiled construct identifier."),
                    number_column("indicator_ordinal", "Indicator order", "Zero-based within-block indicator order."),
                    text_column("indicator_id", "Indicator", "Projected source-column identity."),
                    text_column("update_rule", "Update rule", "Mode A, Mode B, or fixed update rule."),
                    text_column("initialization", "Initialization", "Exact initialization family."),
                ],
                rows,
                footnote_ids: Vec::new(),
                capability_cells: capability_cells.clone(),
            };
            Some((summary, blocks))
        }
        (WeightingScheme::Path | WeightingScheme::Factor, None) if nonlinear_result => None,
        (WeightingScheme::Path | WeightingScheme::Factor, _) => {
            return Err(vec![
                "Recipe-v4 PLS result omitted its exact algorithm convergence receipt".into(),
            ]);
        }
        (WeightingScheme::Pca, None) => None,
        (WeightingScheme::Pca, Some(_)) => {
            return Err(vec![
                "PCA-weighted result unexpectedly contains a Mode A/B convergence receipt".into(),
            ]);
        }
    };

    let mut outer = estimation.outer_estimates.clone();
    outer.sort_by(|left, right| {
        (&left.construct, &left.indicator).cmp(&(&right.construct, &right.indicator))
    });
    let outer_model = CanonicalResultTable {
        id: "outer_model".into(),
        title: "Outer model".into(),
        description: Some(
            "Composite weights and loadings from the converged point estimate.".into(),
        ),
        columns: vec![
            text_column("construct", "Construct", "Composite construct identifier."),
            text_column("indicator", "Indicator", "Observed indicator identifier."),
            number_column("weight", "Weight", "Estimated outer weight."),
            number_column("loading", "Loading", "Estimated outer loading."),
        ],
        rows: outer
            .into_iter()
            .enumerate()
            .map(|(index, estimate)| CanonicalResultRow {
                id: format!("outer_{index:04}"),
                cells: vec![
                    text(estimate.construct),
                    text(estimate.indicator),
                    number(estimate.weight),
                    number(estimate.loading),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let mut paths = estimation.paths.clone();
    paths.sort_by(|left, right| (&left.source, &left.target).cmp(&(&right.source, &right.target)));
    let structural_paths = CanonicalResultTable {
        id: "structural_paths".into(),
        title: "Structural paths".into(),
        description: Some("Estimated direct structural path coefficients.".into()),
        columns: vec![
            text_column("source", "Source", "Predictor construct identifier."),
            text_column("target", "Target", "Outcome construct identifier."),
            number_column(
                "coefficient",
                "Coefficient",
                "Estimated direct path coefficient.",
            ),
        ],
        rows: paths
            .into_iter()
            .enumerate()
            .map(|(index, estimate)| CanonicalResultRow {
                id: format!("path_{index:04}"),
                cells: vec![
                    text(estimate.source),
                    text(estimate.target),
                    number(estimate.coefficient),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let control_estimates = if estimation.control_estimates.is_empty() {
        None
    } else {
        let mut controls = estimation.control_estimates.clone();
        controls.sort_by(|left, right| {
            (&left.source, &left.target).cmp(&(&right.source, &right.target))
        });
        Some(CanonicalResultTable {
            id: qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2.into(),
            title: "Control effects".into(),
            description: Some(
                "Structural coefficients explicitly authored with the canonical control role."
                    .into(),
            ),
            columns: vec![
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
                    "Estimated control coefficient.",
                ),
            ],
            rows: controls
                .into_iter()
                .enumerate()
                .map(|(index, estimate)| CanonicalResultRow {
                    id: format!("control_{index:04}"),
                    cells: vec![
                        text(estimate.source),
                        text(estimate.target),
                        text(estimate.label.unwrap_or_else(|| "Control".into())),
                        number(estimate.coefficient),
                    ],
                })
                .collect(),
            footnote_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        })
    };

    let mut effects = estimation.effects.clone();
    effects
        .sort_by(|left, right| (&left.source, &left.target).cmp(&(&right.source, &right.target)));
    let effects_table = CanonicalResultTable {
        id: "effects".into(),
        title: "Effects".into(),
        description: Some(
            "Direct, indirect, and total effects implied by the structural model.".into(),
        ),
        columns: vec![
            text_column("source", "Source", "Predictor construct identifier."),
            text_column("target", "Target", "Outcome construct identifier."),
            number_column("direct", "Direct", "Direct effect."),
            number_column("indirect", "Indirect", "Total indirect effect."),
            number_column("total", "Total", "Direct plus indirect effect."),
        ],
        rows: effects
            .into_iter()
            .enumerate()
            .map(|(index, estimate)| CanonicalResultRow {
                id: format!("effect_{index:04}"),
                cells: vec![
                    text(estimate.source),
                    text(estimate.target),
                    number(estimate.direct),
                    number(estimate.indirect),
                    number(estimate.total),
                ],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };

    let r_squared = CanonicalResultTable {
        id: "r_squared".into(),
        title: "R-squared".into(),
        description: Some("Explained variance for endogenous constructs.".into()),
        columns: vec![
            text_column("construct", "Construct", "Endogenous construct identifier."),
            number_column("r_squared", "R-squared", "Coefficient of determination."),
        ],
        rows: estimation
            .r_squared
            .iter()
            .enumerate()
            .map(|(index, (construct, value))| CanonicalResultRow {
                id: format!("r_squared_{index:04}"),
                cells: vec![text(construct), number(*value)],
            })
            .collect(),
        footnote_ids: Vec::new(),
        capability_cells: capability_cells.clone(),
    };
    let posthoc_sample_size =
        posthoc.map(|(technical, capability_cell)| posthoc_table(technical, capability_cell));
    let score_execution = score_execution_tables(request, estimation, capability_cells.clone())?;

    let mut structural_table_ids = vec![
        "structural_paths".into(),
        "effects".into(),
        "r_squared".into(),
    ];
    if control_estimates.is_some() {
        structural_table_ids.push(qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2.into());
    }
    if posthoc_sample_size.is_some() {
        structural_table_ids.push("posthoc_minimum_sample_size".into());
    }
    let mut tables = vec![
        summary,
        outer_model,
        structural_paths,
        effects_table,
        r_squared,
    ];
    if let Some(control_estimates) = control_estimates {
        tables.push(control_estimates);
    }
    if let Some(posthoc_sample_size) = posthoc_sample_size {
        tables.push(posthoc_sample_size);
    }
    if let Some((score_summary, score_weights, fixed_scale)) = score_execution {
        tables.push(score_summary);
        tables.push(score_weights);
        if let Some(fixed_scale) = fixed_scale {
            tables.push(fixed_scale);
        }
    }
    tables.push(point_estimate_attribution);
    if let Some((convergence, blocks)) = algorithm_convergence_tables {
        tables.push(convergence);
        tables.push(blocks);
    }
    if let Some(nonlinear_tables) = nonlinear_tables {
        tables.extend(nonlinear_tables);
    }

    let versioned_score_execution = estimation.score_execution.is_some();
    let mut run_detail_table_ids = vec![
        "estimation_summary".into(),
        qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1.into(),
    ];
    if !nonlinear_result
        && matches!(
            request.recipe.settings.weighting_scheme,
            WeightingScheme::Path | WeightingScheme::Factor
        )
    {
        run_detail_table_ids
            .push(qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1.into());
        run_detail_table_ids.push(qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1.into());
    }
    let mut measurement_table_ids = vec!["outer_model".into()];
    if versioned_score_execution {
        run_detail_table_ids.push(qpls_project::PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2.into());
        measurement_table_ids.push(qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2.into());
    }
    if estimation.fixed_score_scale_receipt.as_ref().is_some() {
        run_detail_table_ids.push(qpls_project::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1.into());
    }

    let mut sections = vec![
        CanonicalResultSection {
            id: "run_details".into(),
            title: "Run details".into(),
            description: Some("Convergence and observation accounting.".into()),
            table_ids: run_detail_table_ids,
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        },
        CanonicalResultSection {
            id: "measurement_model".into(),
            title: "Measurement model".into(),
            description: Some("Composite outer-model estimates.".into()),
            table_ids: measurement_table_ids,
            chart_ids: Vec::new(),
            capability_cells: capability_cells.clone(),
        },
        CanonicalResultSection {
            id: "structural_model".into(),
            title: "Structural model".into(),
            description: Some("Structural coefficients, effects, and explained variance.".into()),
            table_ids: structural_table_ids,
            chart_ids: Vec::new(),
            capability_cells: structural_capability_cells,
        },
    ];
    if nonlinear_result {
        sections.push(CanonicalResultSection {
            id: NONLINEAR_SECTION_ID.into(),
            title: "Nonlinear relationships".into(),
            description: Some(
                "Fixed-score centered-quadratic diagnostics and equation fit.".into(),
            ),
            table_ids: vec![
                NONLINEAR_DIAGNOSTICS_TABLE_ID.into(),
                NONLINEAR_EQUATION_FIT_TABLE_ID.into(),
                NONLINEAR_METHOD_SCOPE_TABLE_ID.into(),
            ],
            chart_ids: Vec::new(),
            capability_cells: Some(vec![provenance_capability_cell.clone()]),
        });
    }

    let document = CanonicalResultDocumentV2 {
        schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
        document_id: format!("result_{job_id}"),
        title: if nonlinear_result {
            "PLS nonlinear quadratic diagnostics".into()
        } else {
            "PLS-SEM results".into()
        },
        provenance: CanonicalResultProvenanceV2 {
            run_id: job_id.to_string(),
            project_id: project_id.to_string(),
            model_id: receipt.model_id().into(),
            model_digest: receipt.model_scientific_sha256().into(),
            dataset_id: result.provenance().dataset_id().into(),
            dataset_fingerprint,
            recipe_id: receipt.recipe_id().to_string(),
            recipe_digest: receipt.recipe_analytical_sha256().into(),
            capability_cell: provenance_capability_cell.clone(),
            method_version: result.provenance().estimator_method_version().into(),
            engine_version: result.provenance().adapter_version().into(),
            seed: Some(request.recipe.settings.seed as i64),
            workers,
            started_at: started_at.into(),
            completed_at: completed_at.into(),
        },
        capability_cells: Some(all_capability_cells),
        general_sem_results: None,
        sections,
        tables,
        charts: Vec::new(),
        notices: estimation
            .warnings
            .iter()
            .enumerate()
            .map(|(index, warning)| CanonicalResultNotice {
                id: format!("warning_{index:04}"),
                code: "pls_estimation_warning".into(),
                severity: CanonicalNoticeSeverity::Warning,
                message: warning.clone(),
                section_ids: Vec::new(),
                table_ids: Vec::new(),
            })
            .collect(),
        exclusions: vec![CanonicalResultExclusion {
            id: "point_estimation_only".into(),
            capability_cell: Some(provenance_capability_cell),
            title: "Point estimation only".into(),
            reason:
                "This Recipe-v4 execution slice does not include resampling or assessment add-ons."
                    .into(),
        }],
        footnotes: Vec::new(),
        presentation: CanonicalResultPresentationV2 {
            default_section_id: Some(if nonlinear_result {
                NONLINEAR_SECTION_ID.into()
            } else {
                "structural_model".into()
            }),
            default_table_id: Some(if nonlinear_result {
                NONLINEAR_DIAGNOSTICS_TABLE_ID.into()
            } else {
                "structural_paths".into()
            }),
            precision: 4,
            missing_value_label: "—".into(),
            chart_defaults: CanonicalChartDisplayOptions::default(),
        },
    };

    let validation = validate_canonical_result_document_v2(&document);
    if !validation.passed {
        return Err(validation.errors);
    }

    // The live canonical document and the schema-6 archive attachment are
    // intentionally separate Rust types. Verify their shared strict wire
    // contract before a completed job is ever made visible to callers.
    let archive_value = serde_json::to_value(&document)
        .map_err(|error| vec![format!("canonical result serialization failed: {error}")])?;
    let archive_document =
        serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(archive_value)
            .map_err(|error| vec![format!("schema-6 canonical result parsing failed: {error}")])?;
    archive_document.ensure_valid().map_err(|error| {
        vec![format!(
            "schema-6 canonical result validation failed: {error}"
        )]
    })?;
    if nonlinear_result {
        validate_archived_recipe_v4_pls_method_identity(&archive_document)
            .map_err(|error| vec![format!("Recipe-v4 nonlinear validation failed: {error}")])?;
    } else {
        qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
            &archive_document,
            &request.recipe,
            &request.model,
        )
        .map_err(|error| {
            vec![format!(
                "Recipe-v4 PLS score-execution validation failed: {error}"
            )]
        })?;
    }

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{execute_internal_recipe_v4_pls, resolve_internal_recipe_v4_dataset};
    use qpls_core::{
        AnalysisRecipeModelBindingV4, CompositeWeightingV4, MethodConfig, PlsAlgorithmConfigV2,
        Preprocessing, SemVariableV4, StructuralRelationRoleV4, WeightingScheme,
    };

    #[test]
    fn recipe_v4_point_result_maps_to_a_valid_typed_canonical_document() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();

        let document = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000101").unwrap(),
            project.manifest.project_id,
            "2026-08-14T00:00:00.000Z",
            "2026-08-14T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();

        assert!(validate_canonical_result_document_v2(&document).passed);
        assert_eq!(document.tables.len(), 9);
        assert_eq!(document.tables[2].id, "structural_paths");
        assert_eq!(document.tables[2].rows.len(), 1);
        let technical = document
            .tables
            .iter()
            .find(|table| table.id == "posthoc_minimum_sample_size")
            .expect("canonical result must contain the post-hoc table");
        assert_eq!(technical.rows.len(), 1);
        assert_eq!(
            technical.capability_cells.as_ref().unwrap()[0].cell_id,
            "qpls3.pls.posthoc_technical_minimum_sample_size"
        );
        assert_eq!(
            technical.rows[0].cells[0],
            CanonicalResultCell::Text {
                value: "inference_unavailable".into()
            }
        );
        assert_eq!(
            technical.rows[0].cells[2],
            CanonicalResultCell::Text {
                value: "directional".into()
            }
        );
        assert!(matches!(
            technical.rows[0].cells[7],
            CanonicalResultCell::Missing {
                reason: CanonicalMissingReason::NotEstimated,
                ..
            }
        ));
        assert!(
            document
                .capability_cells
                .as_ref()
                .unwrap()
                .iter()
                .any(|cell| cell.cell_id == "qpls3.pls.posthoc_technical_minimum_sample_size")
        );
        assert_eq!(document.provenance.capability_cell, request.capability_cell);
        assert_eq!(document.provenance.recipe_id, request.recipe.id.to_string());
        let archive_document = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(&document).unwrap(),
        )
        .unwrap();
        archive_document.ensure_valid().unwrap();
    }

    #[test]
    fn nonlinear_v7_maps_to_the_exact_owned_tables_and_rejects_tampering() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::nonlinear_fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        let document = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000171").unwrap(),
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();
        assert_eq!(document.title, "PLS nonlinear quadratic diagnostics");
        assert_eq!(
            document.provenance.engine_version,
            RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7
        );
        assert_eq!(
            document.provenance.method_version,
            NONLINEAR_EFFECTS_METHOD_VERSION
        );
        assert_eq!(
            document
                .capability_cells
                .as_ref()
                .unwrap()
                .iter()
                .map(|cell| cell.cell_id.as_str())
                .collect::<Vec<_>>(),
            vec![PLS_NONLINEAR_EFFECTS_CELL_ID, PLS_ALGORITHM_CELL_ID]
        );
        assert_eq!(
            document.sections[0].table_ids,
            vec![
                "estimation_summary".to_string(),
                qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1.to_string(),
            ]
        );
        assert_eq!(
            document
                .tables
                .iter()
                .map(|table| table.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "estimation_summary",
                "outer_model",
                "structural_paths",
                "effects",
                "r_squared",
                qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1,
                NONLINEAR_DIAGNOSTICS_TABLE_ID,
                NONLINEAR_EQUATION_FIT_TABLE_ID,
                NONLINEAR_METHOD_SCOPE_TABLE_ID,
            ]
        );
        assert!(document.tables.iter().all(|table| {
            !matches!(
                table.id.as_str(),
                qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1
                    | qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1
            )
        }));
        let nonlinear_section = document
            .sections
            .iter()
            .find(|section| section.id == NONLINEAR_SECTION_ID)
            .unwrap();
        assert_eq!(
            nonlinear_section.table_ids,
            vec![
                NONLINEAR_DIAGNOSTICS_TABLE_ID.to_string(),
                NONLINEAR_EQUATION_FIT_TABLE_ID.to_string(),
                NONLINEAR_METHOD_SCOPE_TABLE_ID.to_string(),
            ]
        );
        let nonlinear_ids = document
            .tables
            .iter()
            .rev()
            .take(3)
            .map(|table| table.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            nonlinear_ids,
            vec![
                NONLINEAR_METHOD_SCOPE_TABLE_ID,
                NONLINEAR_EQUATION_FIT_TABLE_ID,
                NONLINEAR_DIAGNOSTICS_TABLE_ID,
            ]
        );
        let diagnostics = document
            .tables
            .iter()
            .find(|table| table.id == NONLINEAR_DIAGNOSTICS_TABLE_ID)
            .unwrap();
        assert_eq!(
            diagnostics
                .columns
                .iter()
                .map(|column| column.id.as_str())
                .collect::<Vec<_>>(),
            NONLINEAR_DIAGNOSTIC_COLUMNS
        );
        assert_eq!(
            diagnostics.rows[0].id,
            "nonlinear_quadratic_diagnostic_0000"
        );
        let archive: qpls_project::CanonicalResultDocumentV2 =
            serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();
        validate_archived_recipe_v4_pls_method_identity(&archive).unwrap();

        let mut fake_primary = archive.clone();
        fake_primary.provenance.capability_cell.capability_id = "unknown.capability".into();
        fake_primary.provenance.capability_cell.cell_id = "unknown.cell".into();
        fake_primary.provenance.capability_cell.capability_version = "unknown_v1".into();
        assert!(validate_archived_recipe_v4_pls_method_identity(&fake_primary).is_err());

        let unknown_cell = fake_primary.provenance.capability_cell.clone();
        let mut unrelated_primary = archive.clone();
        unrelated_primary.sections.pop();
        unrelated_primary
            .tables
            .truncate(unrelated_primary.tables.len() - 3);
        unrelated_primary.provenance.capability_cell = unknown_cell.clone();
        unrelated_primary.provenance.method_version = "unrelated_method_v1".into();
        unrelated_primary.provenance.engine_version = "unrelated_adapter_v1".into();
        let mut unrelated_capability_cells = vec![unknown_cell, archived_base_capability_cell()];
        unrelated_capability_cells.sort_by(|left, right| {
            (
                left.registry_schema_version,
                left.capability_id.as_str(),
                left.cell_id.as_str(),
                left.capability_version.as_str(),
            )
                .cmp(&(
                    right.registry_schema_version,
                    right.capability_id.as_str(),
                    right.cell_id.as_str(),
                    right.capability_version.as_str(),
                ))
        });
        unrelated_primary.capability_cells = Some(unrelated_capability_cells);
        unrelated_primary.presentation.default_section_id = Some("structural_model".into());
        unrelated_primary.presentation.default_table_id = Some("structural_paths".into());
        unrelated_primary.ensure_valid().unwrap();
        validate_archived_recipe_v4_pls_method_identity(&unrelated_primary).unwrap();

        let mut method_only = unrelated_primary.clone();
        method_only.provenance.method_version = NONLINEAR_EFFECTS_METHOD_VERSION.into();
        assert!(validate_archived_recipe_v4_pls_method_identity(&method_only).is_err());

        let mut adapter_only = unrelated_primary;
        adapter_only.provenance.engine_version =
            RECIPE_V4_PLS_NONLINEAR_EXECUTION_ADAPTER_VERSION_V7.into();
        assert!(validate_archived_recipe_v4_pls_method_identity(&adapter_only).is_err());

        let injected_chart = qpls_project::CanonicalResultChartV2 {
            id: "nonlinear_chart_injection".into(),
            title: "Injected nonlinear chart".into(),
            description: "Chart output is outside the exact nonlinear v7 surface.".into(),
            kind: qpls_project::CanonicalChartKindV2::Scatter,
            series: vec![qpls_project::CanonicalChartSeriesV2 {
                id: "injected_series".into(),
                label: "Injected series".into(),
                group: None,
                points: vec![qpls_project::CanonicalChartPointV2 {
                    x: qpls_project::CanonicalChartXValueV2::Text("injected".into()),
                    y: 0.0,
                    lower: None,
                    upper: None,
                    label: None,
                }],
            }],
            source_table_id: Some(NONLINEAR_DIAGNOSTICS_TABLE_ID.into()),
            display: qpls_project::CanonicalChartDisplayOptionsV2::default(),
        };
        let mut unreferenced_chart = archive.clone();
        unreferenced_chart.charts.push(injected_chart.clone());
        unreferenced_chart.ensure_valid().unwrap();
        assert!(validate_archived_recipe_v4_pls_method_identity(&unreferenced_chart).is_err());

        let mut referenced_chart = archive.clone();
        referenced_chart.charts.push(injected_chart);
        referenced_chart
            .sections
            .iter_mut()
            .find(|section| section.id == NONLINEAR_SECTION_ID)
            .unwrap()
            .chart_ids
            .push("nonlinear_chart_injection".into());
        referenced_chart.ensure_valid().unwrap();
        assert!(validate_archived_recipe_v4_pls_method_identity(&referenced_chart).is_err());

        let mut posthoc_payload_tamper = serde_json::to_value(&result).unwrap();
        posthoc_payload_tamper["estimation"]["posthoc_minimum_sample_size"] =
            serde_json::to_value(qpls_estimation::pls_posthoc_minimum_sample_size_v2(
                &result.estimation().paths,
                result.estimation().used_observations,
                None,
            ))
            .unwrap();
        let posthoc_payload_tamper: RecipeV4PlsExecutionResultV1 =
            serde_json::from_value(posthoc_payload_tamper).unwrap();
        let errors = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000173").unwrap(),
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &request,
            &posthoc_payload_tamper,
        )
        .unwrap_err();
        assert!(errors.iter().any(|error| {
            error.contains("nonlinear result unexpectedly contains a post-hoc analytical payload")
        }));

        let mut numerical_tamper = archive.clone();
        let table = numerical_tamper
            .tables
            .iter_mut()
            .find(|table| table.id == NONLINEAR_DIAGNOSTICS_TABLE_ID)
            .unwrap();
        let qpls_project::CanonicalResultCellV2::Number { value, .. } = &mut table.rows[0].cells[3]
        else {
            unreachable!()
        };
        *value += 0.125;
        assert!(validate_archived_recipe_v4_pls_method_identity(&numerical_tamper).is_err());

        let mut ownership_tamper = archive.clone();
        ownership_tamper
            .sections
            .iter_mut()
            .find(|section| section.id == "structural_model")
            .unwrap()
            .table_ids
            .push(NONLINEAR_DIAGNOSTICS_TABLE_ID.into());
        assert!(validate_archived_recipe_v4_pls_method_identity(&ownership_tamper).is_err());

        let (base_project, base_request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let base_dataset =
            resolve_internal_recipe_v4_dataset(&base_project, &base_request).unwrap();
        let base_result = execute_internal_recipe_v4_pls(&base_dataset, &base_request).unwrap();
        let base_document = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000172").unwrap(),
            base_project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &base_request,
            &base_result,
        )
        .unwrap();
        let mut base_archive: qpls_project::CanonicalResultDocumentV2 =
            serde_json::from_value(serde_json::to_value(base_document).unwrap()).unwrap();
        base_archive.tables.push(
            archive
                .tables
                .iter()
                .find(|table| table.id == NONLINEAR_DIAGNOSTICS_TABLE_ID)
                .unwrap()
                .clone(),
        );
        assert!(validate_archived_recipe_v4_pls_method_identity(&base_archive).is_err());
    }

    #[test]
    fn canonical_point_attribution_is_recipe_bound_and_archive_tamper_evident() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        let document = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000112").unwrap(),
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();
        let attribution = document
            .tables
            .iter()
            .find(|table| table.id == qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1)
            .expect("canonical point attribution table");
        assert_eq!(attribution.rows[0].cells[1], text("standardized"));
        assert_eq!(
            attribution.rows[0].cells[6],
            text("zero_mean_unit_variance_construct_score")
        );

        let mut archive: qpls_project::CanonicalResultDocumentV2 =
            serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();
        qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
            &archive,
            &request.recipe,
            &request.model,
        )
        .unwrap();
        let mut missing = archive.clone();
        missing
            .tables
            .retain(|table| table.id != qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1);
        for section in &mut missing.sections {
            section.table_ids.retain(|table_id| {
                table_id != qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1
            });
        }
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &missing,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );
        let table = archive
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1)
            .unwrap();
        table.rows[0].cells[1] = qpls_project::CanonicalResultCellV2::Text {
            value: "mean_centered".into(),
        };
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &archive,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_algorithm_receipt_rejects_stopping_and_order_tampering() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        let document = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000113").unwrap(),
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();
        let archive: qpls_project::CanonicalResultDocumentV2 =
            serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();
        qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
            &archive,
            &request.recipe,
            &request.model,
        )
        .unwrap();
        let mut missing = archive.clone();
        missing.tables.retain(|table| {
            !matches!(
                table.id.as_str(),
                qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1
                    | qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1
            )
        });
        for section in &mut missing.sections {
            section.table_ids.retain(|table_id| {
                !matches!(
                    table_id.as_str(),
                    qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1
                        | qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1
                )
            });
        }
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &missing,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );

        let mut stopping_tamper = archive.clone();
        let summary = stopping_tamper
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1)
            .unwrap();
        summary.rows[0].cells[8] = qpls_project::CanonicalResultCellV2::Number {
            value: request.recipe.settings.tolerance * 2.0,
            display: None,
        };
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &stopping_tamper,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );

        let mut order_tamper = archive;
        let blocks = order_tamper
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1)
            .unwrap();
        blocks.rows.swap(0, 1);
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &order_tamper,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );
    }

    #[test]
    fn canonical_control_family_is_compiled_role_bound_and_archive_tamper_evident() {
        let (project, mut request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let relation = request
            .model
            .relations
            .iter_mut()
            .find(|relation| matches!(relation, qpls_core::SemRelationV4::Structural { .. }))
            .expect("fixture structural path");
        let qpls_core::SemRelationV4::Structural { role, .. } = relation else {
            unreachable!()
        };
        *role = StructuralRelationRoleV4::Control;
        request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
            model: request.model.clone(),
            scientific_sha256: request.model.scientific_sha256().unwrap(),
        };
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        let document = build_recipe_v4_pls_canonical_result(
            Uuid::parse_str("00000000-0000-0000-0000-000000000111").unwrap(),
            project.manifest.project_id,
            "2026-08-16T00:00:00.000Z",
            "2026-08-16T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();
        let controls = document
            .tables
            .iter()
            .find(|table| table.id == qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2)
            .expect("canonical control table");
        assert_eq!(controls.rows.len(), 1);
        assert_eq!(controls.rows[0].cells[0], text("construct:x"));
        assert!(
            document
                .sections
                .iter()
                .find(|section| section.id == "structural_model")
                .unwrap()
                .table_ids
                .contains(&qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2.into())
        );

        let mut archive: qpls_project::CanonicalResultDocumentV2 =
            serde_json::from_value(serde_json::to_value(&document).unwrap()).unwrap();
        qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
            &archive,
            &request.recipe,
            &request.model,
        )
        .unwrap();
        let mut duplicate_section = archive.clone();
        duplicate_section
            .sections
            .iter_mut()
            .find(|section| section.id == "run_details")
            .unwrap()
            .table_ids
            .push(qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2.into());
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &duplicate_section,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );
        let table = archive
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::PLS_CONTROL_ESTIMATES_TABLE_ID_V2)
            .unwrap();
        let qpls_project::CanonicalResultCellV2::Number { value, .. } = &mut table.rows[0].cells[3]
        else {
            unreachable!()
        };
        *value += 0.125;
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &archive,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );
    }

    #[test]
    fn unsafe_seed_is_rejected_before_document_construction() {
        let (project, mut request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        request.recipe.settings.seed = JAVASCRIPT_MAX_SAFE_INTEGER + 1;

        let errors = build_recipe_v4_pls_canonical_result(
            Uuid::new_v4(),
            project.manifest.project_id,
            "2026-08-14T00:00:00.000Z",
            "2026-08-14T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap_err();

        assert!(errors.iter().any(|error| error.contains("safe-integer")));
    }

    #[test]
    fn unopted_result_omits_posthoc_capability_table_and_claim() {
        let (project, mut request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        request.posthoc_technical_minimum_sample_size = None;
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();

        let document = build_recipe_v4_pls_canonical_result(
            Uuid::new_v4(),
            project.manifest.project_id,
            "2026-08-14T00:00:00.000Z",
            "2026-08-14T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();

        assert_eq!(document.tables.len(), 8);
        assert!(
            document
                .tables
                .iter()
                .all(|table| table.id != "posthoc_minimum_sample_size")
        );
        assert!(
            document
                .capability_cells
                .as_ref()
                .unwrap()
                .iter()
                .all(|cell| { cell.cell_id != "qpls3.pls.posthoc_technical_minimum_sample_size" })
        );
    }

    #[test]
    fn canonical_mapping_rejects_request_and_runner_optin_mismatch() {
        let (project, request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        let mut unopted_request = request.clone();
        unopted_request.posthoc_technical_minimum_sample_size = None;

        let errors = build_recipe_v4_pls_canonical_result(
            Uuid::new_v4(),
            project.manifest.project_id,
            "2026-08-14T00:00:00.000Z",
            "2026-08-14T00:00:01.000Z",
            &unopted_request,
            &result,
        )
        .unwrap_err();

        assert!(
            errors
                .iter()
                .any(|error| error.contains("immutable runner provenance"))
        );
    }

    #[test]
    fn configured_initialization_emits_typed_tables_and_rejects_coordinated_weight_tampering() {
        let (project, mut request) = crate::internal_recipe_v4_pls_command_tests::fixture();
        request.posthoc_technical_minimum_sample_size = None;
        request.recipe.method_config = Some(MethodConfig::PlsAlgorithmConfiguredV2(
            PlsAlgorithmConfigV2::standard(),
        ));
        let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
        let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
        let document = build_recipe_v4_pls_canonical_result(
            Uuid::new_v4(),
            project.manifest.project_id,
            "2026-08-15T00:00:00.000Z",
            "2026-08-15T00:00:01.000Z",
            &request,
            &result,
        )
        .unwrap();

        assert_eq!(
            document.provenance.method_version,
            PLS_SCORE_EXECUTION_METHOD_VERSION_V2
        );
        assert_eq!(document.tables.len(), 10);
        assert!(
            document
                .tables
                .iter()
                .any(|table| { table.id == qpls_project::PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2 })
        );
        assert!(
            document
                .tables
                .iter()
                .any(|table| { table.id == qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2 })
        );

        let mut archive = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
            serde_json::to_value(document).unwrap(),
        )
        .unwrap();
        qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
            &archive,
            &request.recipe,
            &request.model,
        )
        .unwrap();
        let weights = archive
            .tables
            .iter_mut()
            .find(|table| table.id == qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2)
            .unwrap();
        let qpls_project::CanonicalResultCellV2::Number { value, .. } =
            &mut weights.rows[0].cells[7]
        else {
            unreachable!()
        };
        *value *= 0.5;
        assert!(
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &archive,
                &request.recipe,
                &request.model,
            )
            .is_err()
        );
    }

    #[test]
    fn configured_fixed_nonstandard_scales_bind_canonical_and_archive_receipts() {
        for preprocessing in [Preprocessing::MeanCentered, Preprocessing::Unstandardized] {
            for weighting_scheme in [WeightingScheme::Path, WeightingScheme::Factor] {
                let (project, mut request) =
                    crate::internal_recipe_v4_pls_command_tests::fixed_custom_fixture();
                request.recipe.settings.preprocessing = preprocessing.clone();
                request.recipe.settings.weighting_scheme = weighting_scheme;
                request.recipe.method_config = Some(MethodConfig::PlsAlgorithmConfiguredV2(
                    PlsAlgorithmConfigV2::standard(),
                ));
                let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
                let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
                let document = build_recipe_v4_pls_canonical_result(
                    Uuid::parse_str("00000000-0000-0000-0000-000000000114").unwrap(),
                    project.manifest.project_id,
                    "2026-08-16T00:00:00.000Z",
                    "2026-08-16T00:00:01.000Z",
                    &request,
                    &result,
                )
                .unwrap();
                let expected_preprocessing = match &preprocessing {
                    Preprocessing::MeanCentered => "mean_centered",
                    Preprocessing::Unstandardized => "unstandardized",
                    Preprocessing::Standardized => unreachable!(),
                };
                let attribution = document
                    .tables
                    .iter()
                    .find(|table| {
                        table.id == qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1
                    })
                    .unwrap();
                assert_eq!(attribution.rows[0].cells[1], text(expected_preprocessing));
                assert!(document.tables.iter().any(
                    |table| table.id == qpls_project::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1
                ));

                let archive: qpls_project::CanonicalResultDocumentV2 =
                    serde_json::from_value(serde_json::to_value(document).unwrap()).unwrap();
                qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                    &archive,
                    &request.recipe,
                    &request.model,
                )
                .unwrap();
                let mut tampered = archive;
                let table = tampered
                    .tables
                    .iter_mut()
                    .find(|table| {
                        table.id == qpls_project::PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1
                    })
                    .unwrap();
                let qpls_project::CanonicalResultCellV2::Text { value } =
                    &mut table.rows[0].cells[1]
                else {
                    unreachable!()
                };
                *value = "standardized".into();
                assert!(
                    qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                        &tampered,
                        &request.recipe,
                        &request.model,
                    )
                    .is_err()
                );
            }
        }
    }

    #[test]
    fn fixed_custom_normalizations_compile_execute_and_emit_exact_canonical_tokens() {
        for (normalization, token) in [
            (CompositeWeightNormalizationV4::None, "none"),
            (CompositeWeightNormalizationV4::SumToOne, "sum_to_one"),
            (
                CompositeWeightNormalizationV4::UnitVariance,
                "unit_variance",
            ),
        ] {
            let (project, mut request) =
                crate::internal_recipe_v4_pls_command_tests::fixed_custom_fixture();
            let variable = request
                .model
                .variables
                .iter_mut()
                .find(|variable| variable.id() == "construct:x")
                .unwrap();
            let SemVariableV4::Composite { weighting, .. } = variable else {
                unreachable!("fixed-score fixture must contain a composite")
            };
            *weighting = CompositeWeightingV4::Custom {
                weights: std::collections::BTreeMap::from([
                    ("observed:x1".into(), -0.25),
                    ("observed:x2".into(), 0.75),
                ]),
                normalization,
            };
            request.recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                model: request.model.clone(),
                scientific_sha256: request.model.scientific_sha256().unwrap(),
            };

            let dataset = resolve_internal_recipe_v4_dataset(&project, &request).unwrap();
            let result = execute_internal_recipe_v4_pls(&dataset, &request).unwrap();
            let execution = result.estimation().score_execution.as_ref().unwrap();
            assert!(execution.blocks.iter().any(|block| {
                matches!(
                    &block.scoring,
                    PlsResolvedScoreBlockKindV2::FixedCustom {
                        normalization: actual,
                        ..
                    } if *actual == normalization
                )
            }));

            let document = build_recipe_v4_pls_canonical_result(
                Uuid::new_v4(),
                project.manifest.project_id,
                "2026-08-15T00:00:00.000Z",
                "2026-08-15T00:00:01.000Z",
                &request,
                &result,
            )
            .unwrap();
            let table = document
                .tables
                .iter()
                .find(|table| table.id == qpls_project::PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2)
                .unwrap();
            let fixed_rows = table
                .rows
                .iter()
                .filter(|row| {
                    matches!(
                        &row.cells[2],
                        CanonicalResultCell::Text { value } if value == "fixed_custom"
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(fixed_rows.len(), 2);
            let requested_sum = fixed_rows
                .iter()
                .map(|row| match &row.cells[6] {
                    CanonicalResultCell::Number { value, .. } => *value,
                    _ => unreachable!(),
                })
                .sum::<f64>();
            for row in fixed_rows {
                assert_eq!(row.cells[5], text(token));
                assert_eq!(row.cells[7], row.cells[8]);
                let CanonicalResultCell::Number {
                    value: requested, ..
                } = &row.cells[6]
                else {
                    unreachable!()
                };
                let CanonicalResultCell::Number {
                    value: resolved, ..
                } = &row.cells[7]
                else {
                    unreachable!()
                };
                match normalization {
                    CompositeWeightNormalizationV4::None => {
                        assert_eq!(requested.to_bits(), resolved.to_bits())
                    }
                    CompositeWeightNormalizationV4::SumToOne => {
                        assert_eq!((*requested / requested_sum).to_bits(), resolved.to_bits())
                    }
                    CompositeWeightNormalizationV4::UnitVariance => {
                        assert!(resolved.is_finite())
                    }
                }
            }
            let mut archive = serde_json::from_value::<qpls_project::CanonicalResultDocumentV2>(
                serde_json::to_value(document).unwrap(),
            )
            .unwrap();
            qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                &archive,
                &request.recipe,
                &request.model,
            )
            .unwrap();
            let mut missing_scale = archive.clone();
            missing_scale.tables.retain(|table| {
                table.id != qpls_project::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1
            });
            for section in &mut missing_scale.sections {
                section.table_ids.retain(|table_id| {
                    table_id != qpls_project::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1
                });
            }
            assert!(
                qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                    &missing_scale,
                    &request.recipe,
                    &request.model,
                )
                .is_err()
            );
            let scale = archive
                .tables
                .iter_mut()
                .find(|table| table.id == qpls_project::PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1)
                .expect("current fixed result must include its scale receipt");
            assert_eq!(scale.rows.len(), 2);
            let qpls_project::CanonicalResultCellV2::Number { value, .. } =
                &mut scale.rows[0].cells[6]
            else {
                unreachable!()
            };
            *value = f64::from_bits(value.to_bits() + 1);
            assert!(
                qpls_project::validate_recipe_v4_pls_score_execution_document_v2(
                    &archive,
                    &request.recipe,
                    &request.model,
                )
                .is_err()
            );
        }
    }
}
