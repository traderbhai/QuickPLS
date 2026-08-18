use crate::{
    CanonicalMissingReasonV2, CanonicalResultCellV2, CanonicalResultDocumentV2,
    CanonicalResultTableV2,
};
use qpls_core::{
    AnalysisRecipeV4, CompiledPlsBlockModeV2, CompiledPlsFixedScoringV2, CompiledPlsPlanV2,
    CompiledRecipePlanV4, CompositeWeightNormalizationV4, MethodConfig, PlsInitialOuterWeightsV2,
    Preprocessing, RecipeV4CompilerTarget, SemModelV4, SemParameterV4, StructuralRelationRoleV4,
    WeightingScheme, compile_analysis_recipe_v4,
};
use qpls_estimation::{
    PLS_ALGORITHM_CONVERGENCE_RECEIPT_CONTRACT_VERSION_V1,
    PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1, PLS_METHOD_VERSION,
    PLS_POINT_ESTIMATE_ATTRIBUTION_CONTRACT_VERSION_V1, PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2,
    PLS_SCORE_EXECUTION_METHOD_VERSION_V2, PlsPointEstimateAttributionV1,
};
use std::collections::BTreeMap;

pub const PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2: &str = "score_execution_summary";
pub const PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2: &str = "score_execution_weights";
pub const PLS_CONTROL_ESTIMATES_TABLE_ID_V2: &str = "control_estimates";
pub const PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1: &str = "point_estimate_attribution";
pub const PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1: &str = "algorithm_convergence_receipt";
pub const PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1: &str = "algorithm_block_order";
pub const PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1: &str = "fixed_score_scale_receipt";
pub const LEGACY_RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v3";
pub const LEGACY_RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v4";
pub const RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v5";
pub const RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2: &str =
    "compiled_recipe_v4_pls_plan_v2_execution_v6";

const POINT_ATTRIBUTION_COLUMNS: &[&str] = &[
    "contract_version",
    "preprocessing",
    "indicator_centering",
    "indicator_scaling",
    "outer_weights",
    "outer_loadings",
    "construct_scores",
    "structural_paths",
    "effects",
];
const ALGORITHM_CONVERGENCE_COLUMNS: &[&str] = &[
    "contract_version",
    "weighting_scheme",
    "maximum_iterations",
    "stop_criterion",
    "comparison",
    "performed_iterations",
    "estimated_block_updates",
    "termination_reason",
    "final_max_outer_weight_change",
];
const ALGORITHM_BLOCK_COLUMNS: &[&str] = &[
    "block_ordinal",
    "construct_id",
    "indicator_ordinal",
    "indicator_id",
    "update_rule",
    "initialization",
];

const SCORE_SUMMARY_COLUMNS: &[&str] = &[
    "contract_version",
    "maximum_iterations",
    "stop_criterion",
    "estimated_block_count",
    "fixed_block_count",
    "performed_iterations",
    "estimated_block_updates",
];
const SCORE_WEIGHT_COLUMNS: &[&str] = &[
    "construct_id",
    "indicator_id",
    "block_kind",
    "estimated_mode",
    "requested_initialization",
    "normalization",
    "requested_weight",
    "resolved_initial_or_fixed_weight",
    "final_outer_weight",
];
const FIXED_SCORE_SCALE_COLUMNS: &[&str] = &[
    "contract_version",
    "construct_id",
    "indicator_id",
    "pre_standardization_center",
    "pre_standardization_scale",
    "resolved_scoring_coefficient",
    "effective_unit_score_weight",
];

#[derive(Debug, thiserror::Error)]
pub enum PlsScoreExecutionDocumentV2Error {
    #[error("invalid recipe-v4 PLS score-execution document: {0}")]
    Invalid(String),
    #[error("recipe-v4 PLS recompilation failed: {0}")]
    Recompilation(String),
}

#[derive(Debug)]
struct ExpectedWeightRow {
    construct_id: String,
    indicator_id: String,
    source_column: String,
    block_kind: &'static str,
    estimated_mode: Option<&'static str>,
    requested_initialization: Option<&'static str>,
    normalization: Option<&'static str>,
    requested_weight: f64,
    estimated: bool,
}

pub fn validate_recipe_v4_pls_score_execution_document_v2(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    model: &SemModelV4,
) -> Result<(), PlsScoreExecutionDocumentV2Error> {
    let target = RecipeV4CompilerTarget::PlsPlanV2;
    let artifact =
        compile_analysis_recipe_v4(recipe, Some(model), target, target.capability_cell())
            .map_err(|error| PlsScoreExecutionDocumentV2Error::Recompilation(error.to_string()))?;
    let CompiledRecipePlanV4::PlsPlanV2 { plan } = artifact.plan() else {
        unreachable!("the exact PLS target must return a PLS plan")
    };
    let receipt = artifact.receipt();
    if document.provenance.recipe_id != receipt.recipe_id().to_string()
        || document.provenance.recipe_digest != receipt.recipe_analytical_sha256()
        || document.provenance.model_id != receipt.model_id()
        || document.provenance.model_digest != receipt.model_scientific_sha256()
        || document.provenance.dataset_id != plan.dataset_id()
        || document.provenance.dataset_fingerprint
            != recorded_sha256(receipt.dataset_fingerprint()).unwrap_or_default()
        || document.provenance.capability_cell.registry_schema_version
            != receipt.capability_cell().registry_schema_version
        || document.provenance.capability_cell.capability_id
            != receipt.capability_cell().capability_id
        || document.provenance.capability_cell.cell_id != receipt.capability_cell().cell_id
        || document.provenance.capability_cell.capability_version
            != receipt.capability_cell().capability_version
    {
        return invalid("canonical provenance differs from deterministic Recipe-v4 recompilation");
    }
    let configured = matches!(
        recipe.method_config.as_ref(),
        Some(MethodConfig::PlsAlgorithmConfiguredV2(_))
    );
    let fixed = plan
        .blocks()
        .iter()
        .any(|block| block.fixed_scoring().is_some());
    let versioned = configured || fixed;
    let (legacy_adapter, current_adapter) = if versioned {
        (
            LEGACY_RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2,
            RECIPE_V4_PLS_SCORE_EXECUTION_ADAPTER_VERSION_V2,
        )
    } else {
        (
            LEGACY_RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1,
            RECIPE_V4_PLS_EXECUTION_ADAPTER_VERSION_V1,
        )
    };
    let current_generation = match document.provenance.engine_version.as_str() {
        engine if engine == current_adapter => true,
        engine if engine == legacy_adapter => false,
        _ => {
            return invalid("canonical PLS adapter identity is not explicitly allowlisted");
        }
    };
    validate_point_estimate_attribution(document, recipe, current_generation)?;
    validate_algorithm_convergence_receipt(document, recipe, plan, current_generation)?;
    validate_fixed_score_scale_receipt(document, plan, current_generation)?;
    validate_control_result_family(document, model, plan)?;

    let score_tables = (
        table(document, PLS_SCORE_EXECUTION_SUMMARY_TABLE_ID_V2),
        table(document, PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2),
    );
    if !versioned {
        if document.provenance.method_version != PLS_METHOD_VERSION
            || score_tables.0.is_some()
            || score_tables.1.is_some()
        {
            return invalid(
                "legacy all-estimated PLS must retain pls_pm_v1 and omit score-execution-v2 tables",
            );
        }
        return Ok(());
    }
    if document.provenance.method_version != PLS_SCORE_EXECUTION_METHOD_VERSION_V2 {
        return invalid("versioned scoring has a drifted estimator or adapter identity");
    }
    let summary = score_tables
        .0
        .ok_or_else(|| invalid_error("versioned scoring omitted score_execution_summary"))?;
    let weights = score_tables
        .1
        .ok_or_else(|| invalid_error("versioned scoring omitted score_execution_weights"))?;
    validate_exact_columns(summary, SCORE_SUMMARY_COLUMNS)?;
    validate_exact_columns(weights, SCORE_WEIGHT_COLUMNS)?;
    if summary.rows.len() != 1 || summary.rows[0].id != "execution" {
        return invalid("score_execution_summary must contain exactly the execution row");
    }
    let summary_cells = &summary.rows[0].cells;
    if text_cell(&summary_cells[0]) != Some(PLS_SCORE_EXECUTION_CONTRACT_VERSION_V2) {
        return invalid("score-execution contract version differs from pls_score_execution_v2");
    }
    let maximum_iterations = integer_cell(&summary_cells[1])?;
    let stop_criterion = number_cell(&summary_cells[2])?;
    let estimated_block_count = integer_cell(&summary_cells[3])?;
    let fixed_block_count = integer_cell(&summary_cells[4])?;
    let performed_iterations = integer_cell(&summary_cells[5])?;
    let estimated_block_updates = integer_cell_u64(&summary_cells[6])?;
    let expected_estimated = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_none())
        .count() as u32;
    let expected_fixed = plan.blocks().len() as u32 - expected_estimated;
    if maximum_iterations != qpls_core::PLS_ALGORITHM_FIXED_MAX_ITERATIONS
        || stop_criterion.to_bits() != qpls_core::PLS_ALGORITHM_FIXED_STOP_CRITERION.to_bits()
        || estimated_block_count != expected_estimated
        || fixed_block_count != expected_fixed
        || estimated_block_updates
            != u64::from(performed_iterations) * u64::from(estimated_block_count)
        || (estimated_block_count == 0 && performed_iterations != 0)
        || (estimated_block_count > 0
            && !(1..=qpls_core::PLS_ALGORITHM_FIXED_MAX_ITERATIONS).contains(&performed_iterations))
    {
        return invalid("score-execution iteration accounting is not the exact compiled contract");
    }
    let estimation_iterations = estimation_summary_iterations(document)?;
    if estimation_iterations != performed_iterations {
        return invalid("score-execution iterations differ from the estimation summary");
    }

    let expected = expected_weight_rows(recipe, plan)?;
    if weights.rows.len() != expected.len() {
        return invalid("score-execution weight row count differs from the compiled plan");
    }
    let outer = canonical_outer_weights(document)?;
    let mut resolved_by_block = BTreeMap::<String, (Option<&'static str>, Vec<(f64, f64)>)>::new();
    for (index, (row, expected)) in weights.rows.iter().zip(&expected).enumerate() {
        if row.id != format!("score_weight_{index:04}")
            || row.cells.len() != SCORE_WEIGHT_COLUMNS.len()
        {
            return invalid("score-execution weight row identity or width is non-canonical");
        }
        if text_cell(&row.cells[0]) != Some(expected.construct_id.as_str())
            || text_cell(&row.cells[1]) != Some(expected.indicator_id.as_str())
            || text_cell(&row.cells[2]) != Some(expected.block_kind)
            || optional_text_cell(&row.cells[3])? != expected.estimated_mode
            || optional_text_cell(&row.cells[4])? != expected.requested_initialization
            || optional_text_cell(&row.cells[5])? != expected.normalization
            || number_cell(&row.cells[6])?.to_bits() != expected.requested_weight.to_bits()
        {
            return invalid(
                "score-execution requested semantics differ from recipe/model recompilation",
            );
        }
        let resolved = number_cell(&row.cells[7])?;
        let final_outer = number_cell(&row.cells[8])?;
        let Some(stored_outer) = outer.get(&(
            expected.construct_id.clone(),
            expected.source_column.clone(),
        )) else {
            return invalid("outer_model omitted a compiled score-execution indicator");
        };
        if stored_outer.to_bits() != final_outer.to_bits() {
            return invalid("score-execution final outer weight differs from outer_model");
        }
        if !expected.estimated && resolved.to_bits() != final_outer.to_bits() {
            return invalid(
                "fixed scoring changed between resolved execution and final outer output",
            );
        }
        let entry = resolved_by_block
            .entry(expected.construct_id.clone())
            .or_insert_with(|| (expected.normalization, Vec::new()));
        if entry.0 != expected.normalization {
            return invalid("compiled score block has inconsistent normalization metadata");
        }
        entry.1.push((expected.requested_weight, resolved));
    }
    if resolved_by_block
        .values()
        .any(|(normalization, weights)| !resolved_weights_match(*normalization, weights))
    {
        return invalid(
            "resolved initial/fixed weights differ from the compiled normalization contract",
        );
    }
    Ok(())
}

fn validate_point_estimate_attribution(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    current_generation: bool,
) -> Result<(), PlsScoreExecutionDocumentV2Error> {
    // The table is additive so historical v3/v4 documents remain readable.
    // Whenever present, every token and its section ownership are strict.
    let Some(table) = table(document, PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1) else {
        return if current_generation {
            invalid("current PLS adapter omitted point_estimate_attribution")
        } else {
            Ok(())
        };
    };
    let attribution_references = document
        .sections
        .iter()
        .filter(|section| section.id == "run_details")
        .flat_map(|section| &section.table_ids)
        .filter(|table_id| table_id.as_str() == PLS_POINT_ESTIMATE_ATTRIBUTION_TABLE_ID_V1)
        .count();
    if attribution_references != 1 {
        return invalid(
            "point_estimate_attribution must belong exactly once to the run_details section",
        );
    }
    validate_exact_columns(table, POINT_ATTRIBUTION_COLUMNS)?;
    if table.rows.len() != 1
        || table.rows[0].id != "attribution"
        || table.rows[0].cells.len() != POINT_ATTRIBUTION_COLUMNS.len()
    {
        return invalid("point_estimate_attribution must contain exactly its canonical row");
    }
    let expected =
        PlsPointEstimateAttributionV1::for_preprocessing(recipe.settings.preprocessing.clone());
    let expected_cells = [
        PLS_POINT_ESTIMATE_ATTRIBUTION_CONTRACT_VERSION_V1,
        preprocessing_name(&expected.preprocessing),
        expected.indicator_centering.as_str(),
        expected.indicator_scaling.as_str(),
        expected.outer_weights.as_str(),
        expected.outer_loadings.as_str(),
        expected.construct_scores.as_str(),
        expected.structural_paths.as_str(),
        expected.effects.as_str(),
    ];
    if table.rows[0]
        .cells
        .iter()
        .zip(expected_cells)
        .any(|(cell, expected)| text_cell(cell) != Some(expected))
    {
        return invalid(
            "point-estimate attribution differs from the exact Recipe-v4 preprocessing contract",
        );
    }
    Ok(())
}

fn validate_algorithm_convergence_receipt(
    document: &CanonicalResultDocumentV2,
    recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
    current_generation: bool,
) -> Result<(), PlsScoreExecutionDocumentV2Error> {
    let summary = table(document, PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1);
    let blocks = table(document, PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1);
    if matches!(recipe.settings.weighting_scheme, WeightingScheme::Pca) {
        return if summary.is_none() && blocks.is_none() {
            if current_generation {
                invalid("current PLS adapter requires the algorithm convergence receipt")
            } else {
                Ok(())
            }
        } else {
            invalid("PCA weighting cannot claim the Mode A/B convergence receipt")
        };
    }
    if summary.is_none() || blocks.is_none() {
        return if summary.is_none() && blocks.is_none() {
            if current_generation {
                invalid("current PLS adapter omitted the algorithm convergence receipt")
            } else {
                // Additive compatibility for historical v3/v4 canonical results.
                Ok(())
            }
        } else {
            invalid("algorithm convergence receipt tables must be present as one exact family")
        };
    }
    let summary = summary.unwrap();
    let blocks = blocks.unwrap();
    for table_id in [
        PLS_ALGORITHM_CONVERGENCE_SUMMARY_TABLE_ID_V1,
        PLS_ALGORITHM_BLOCK_ORDER_TABLE_ID_V1,
    ] {
        let references = document
            .sections
            .iter()
            .filter(|section| section.id == "run_details")
            .flat_map(|section| &section.table_ids)
            .filter(|candidate| candidate.as_str() == table_id)
            .count();
        if references != 1 {
            return invalid("algorithm receipt tables must belong exactly once to run_details");
        }
    }
    validate_exact_columns(summary, ALGORITHM_CONVERGENCE_COLUMNS)?;
    validate_exact_columns(blocks, ALGORITHM_BLOCK_COLUMNS)?;
    if summary.rows.len() != 1
        || summary.rows[0].id != "convergence"
        || summary.rows[0].cells.len() != ALGORITHM_CONVERGENCE_COLUMNS.len()
    {
        return invalid("algorithm convergence summary is non-canonical");
    }
    let cells = &summary.rows[0].cells;
    let maximum_iterations = integer_cell(&cells[2])?;
    let stop_criterion = number_cell(&cells[3])?;
    let performed_iterations = integer_cell(&cells[5])?;
    let estimated_block_updates = integer_cell_u64(&cells[6])?;
    let estimated_blocks = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_none())
        .count() as u64;
    if text_cell(&cells[0]) != Some(PLS_ALGORITHM_CONVERGENCE_RECEIPT_CONTRACT_VERSION_V1)
        || text_cell(&cells[1]) != Some(weighting_scheme_name(&recipe.settings.weighting_scheme))
        || maximum_iterations != recipe.settings.max_iterations
        || stop_criterion.to_bits() != recipe.settings.tolerance.to_bits()
        || text_cell(&cells[4]) != Some("less_than_or_equal")
        || performed_iterations != estimation_summary_iterations(document)?
        || estimated_block_updates != u64::from(performed_iterations) * estimated_blocks
    {
        return invalid("algorithm convergence settings or accounting differ from the recipe");
    }
    if estimated_blocks == 0 {
        if performed_iterations != 0
            || text_cell(&cells[7]) != Some("all_blocks_fixed")
            || !matches!(
                &cells[8],
                CanonicalResultCellV2::Missing {
                    reason: CanonicalMissingReasonV2::NotApplicable,
                    ..
                }
            )
        {
            return invalid("all-fixed algorithm termination receipt is incoherent");
        }
    } else {
        let final_change = number_cell(&cells[8])?;
        if performed_iterations == 0
            || performed_iterations > maximum_iterations
            || text_cell(&cells[7]) != Some("converged_tolerance")
            || final_change < 0.0
            || final_change > stop_criterion
        {
            return invalid("estimated algorithm termination receipt is incoherent");
        }
    }

    let individual_initialization = matches!(
        recipe.method_config.as_ref(),
        Some(MethodConfig::PlsAlgorithmConfiguredV2(config))
            if matches!(&config.initial_outer_weights, PlsInitialOuterWeightsV2::Individual { .. })
    );
    let expected_rows = plan
        .blocks()
        .iter()
        .map(|block| block.indicators().len())
        .sum::<usize>();
    if blocks.rows.len() != expected_rows {
        return invalid("algorithm block-order row count differs from the compiled PLS plan");
    }
    let mut row_index = 0;
    for (block_index, block) in plan.blocks().iter().enumerate() {
        let (update_rule, initialization) = match block.fixed_scoring() {
            Some(CompiledPlsFixedScoringV2::Unit { .. }) => {
                ("fixed_no_update", "fixed_unit_weights")
            }
            Some(CompiledPlsFixedScoringV2::Custom { .. }) => {
                ("fixed_no_update", "fixed_custom_weights")
            }
            None => (
                match block.mode() {
                    CompiledPlsBlockModeV2::ModeA => "mode_a_covariance",
                    CompiledPlsBlockModeV2::ModeB => "mode_b_ols",
                },
                if individual_initialization {
                    "individual_requested_weights"
                } else {
                    "standard_unit_weights"
                },
            ),
        };
        for (indicator_index, indicator) in block.indicators().iter().enumerate() {
            let row = &blocks.rows[row_index];
            if row.cells.len() != ALGORITHM_BLOCK_COLUMNS.len()
                || row.id
                    != format!("algorithm_block_{block_index:04}_indicator_{indicator_index:04}")
                || integer_cell(&row.cells[0])? != block_index as u32
                || text_cell(&row.cells[1]) != Some(block.construct_id())
                || integer_cell(&row.cells[2])? != indicator_index as u32
                || text_cell(&row.cells[3]) != Some(indicator.source_column())
                || text_cell(&row.cells[4]) != Some(update_rule)
                || text_cell(&row.cells[5]) != Some(initialization)
            {
                return invalid(
                    "algorithm block or indicator order differs from the compiled PLS plan",
                );
            }
            row_index += 1;
        }
    }
    Ok(())
}

fn weighting_scheme_name(weighting_scheme: &WeightingScheme) -> &'static str {
    match weighting_scheme {
        WeightingScheme::Path => "path",
        WeightingScheme::Factor => "factor",
        WeightingScheme::Pca => "pca",
    }
}

fn preprocessing_name(preprocessing: &Preprocessing) -> &'static str {
    match preprocessing {
        Preprocessing::Standardized => "standardized",
        Preprocessing::MeanCentered => "mean_centered",
        Preprocessing::Unstandardized => "unstandardized",
    }
}

fn validate_control_result_family(
    document: &CanonicalResultDocumentV2,
    model: &SemModelV4,
    plan: &CompiledPlsPlanV2,
) -> Result<(), PlsScoreExecutionDocumentV2Error> {
    let labels = model
        .parameters
        .iter()
        .map(|parameter| {
            let label = match parameter {
                SemParameterV4::Free { label, .. }
                | SemParameterV4::Fixed { label, .. }
                | SemParameterV4::Derived { label, .. } => label,
            };
            (parameter.id(), label.as_str())
        })
        .collect::<BTreeMap<_, _>>();
    let mut expected = plan
        .paths()
        .iter()
        .filter(|path| path.role() == StructuralRelationRoleV4::Control)
        .map(|path| {
            let label = labels.get(path.parameter_id()).ok_or_else(|| {
                invalid_error("compiled control path references an unknown parameter")
            })?;
            Ok((path.source(), path.target(), *label))
        })
        .collect::<Result<Vec<_>, PlsScoreExecutionDocumentV2Error>>()?;
    expected.sort();
    let controls = table(document, PLS_CONTROL_ESTIMATES_TABLE_ID_V2);
    let control_references = document
        .sections
        .iter()
        .flat_map(|section| {
            section
                .table_ids
                .iter()
                .map(move |table_id| (section.id.as_str(), table_id.as_str()))
        })
        .filter(|(_, table_id)| *table_id == PLS_CONTROL_ESTIMATES_TABLE_ID_V2)
        .collect::<Vec<_>>();
    if expected.is_empty() {
        return if controls.is_none() && control_references.is_empty() {
            Ok(())
        } else {
            invalid("canonical result contains controls absent from the compiled PLS plan")
        };
    }
    let controls = controls.ok_or_else(|| {
        invalid_error("compiled PLS controls require the canonical control_estimates table")
    })?;
    if control_references.len() != 1
        || control_references[0] != ("structural_model", PLS_CONTROL_ESTIMATES_TABLE_ID_V2)
    {
        return invalid("control_estimates must belong exactly once to structural_model");
    }
    validate_exact_columns(controls, &["source", "target", "label", "coefficient"])?;
    if controls.rows.len() != expected.len() {
        return invalid("control_estimates row count differs from the compiled PLS plan");
    }
    let structural = table(document, "structural_paths")
        .ok_or_else(|| invalid_error("canonical result omitted structural_paths"))?;
    validate_exact_columns(structural, &["source", "target", "coefficient"])?;
    let mut coefficients = BTreeMap::new();
    for row in &structural.rows {
        if row.cells.len() != 3 {
            return invalid("structural_paths row width is non-canonical");
        }
        let source = text_cell(&row.cells[0])
            .ok_or_else(|| invalid_error("structural path source must be text"))?;
        let target = text_cell(&row.cells[1])
            .ok_or_else(|| invalid_error("structural path target must be text"))?;
        if coefficients
            .insert((source, target), number_cell(&row.cells[2])?)
            .is_some()
        {
            return invalid("structural_paths contains a duplicate endpoint pair");
        }
    }
    for (index, (row, (source, target, label))) in controls.rows.iter().zip(expected).enumerate() {
        if row.cells.len() != 4
            || row.id != format!("control_{index:04}")
            || text_cell(&row.cells[0]) != Some(source)
            || text_cell(&row.cells[1]) != Some(target)
            || text_cell(&row.cells[2]) != Some(label)
        {
            return invalid(
                "control_estimates identity or label differs from the compiled PLS plan",
            );
        }
        let coefficient = number_cell(&row.cells[3])?;
        if coefficients
            .get(&(source, target))
            .is_none_or(|structural| structural.to_bits() != coefficient.to_bits())
        {
            return invalid("control estimate differs from its structural path coefficient");
        }
    }
    Ok(())
}

fn validate_fixed_score_scale_receipt(
    document: &CanonicalResultDocumentV2,
    plan: &CompiledPlsPlanV2,
    current_generation: bool,
) -> Result<(), PlsScoreExecutionDocumentV2Error> {
    let fixed = plan
        .blocks()
        .iter()
        .filter(|block| block.fixed_scoring().is_some())
        .collect::<Vec<_>>();
    let Some(receipt) = table(document, PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1) else {
        return if current_generation && !fixed.is_empty() {
            invalid("current fixed-score PLS result omitted fixed_score_scale_receipt")
        } else {
            Ok(())
        };
    };
    if fixed.is_empty() {
        return invalid("fixed_score_scale_receipt exists without compiled fixed blocks");
    }
    let references = document
        .sections
        .iter()
        .flat_map(|section| {
            section
                .table_ids
                .iter()
                .filter(|id| *id == PLS_FIXED_SCORE_SCALE_RECEIPT_TABLE_ID_V1)
                .map(move |_| section.id.as_str())
        })
        .collect::<Vec<_>>();
    if references.as_slice() != ["run_details"] {
        return invalid("fixed_score_scale_receipt must belong exactly once to run_details");
    }
    validate_exact_columns(receipt, FIXED_SCORE_SCALE_COLUMNS)?;
    let score_weights = table(document, PLS_SCORE_EXECUTION_WEIGHTS_TABLE_ID_V2)
        .ok_or_else(|| invalid_error("fixed-score scale receipt requires score weights"))?;
    let resolved = score_weights
        .rows
        .iter()
        .filter_map(|row| {
            Some((
                (
                    text_cell(row.cells.first()?)?,
                    text_cell(row.cells.get(1)?)?,
                ),
                number_cell(row.cells.get(7)?).ok()?,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let expected_rows = fixed
        .iter()
        .map(|block| block.indicators().len())
        .sum::<usize>();
    if receipt.rows.len() != expected_rows {
        return invalid("fixed_score_scale_receipt row count differs from compiled fixed blocks");
    }
    let mut row_index = 0;
    for block in fixed {
        let mut block_scale = None;
        let mut block_center = None;
        for indicator in block.indicators() {
            let row = &receipt.rows[row_index];
            if row.id != format!("fixed_score_scale_{row_index:04}")
                || text_cell(&row.cells[0])
                    != Some(PLS_FIXED_SCORE_SCALE_RECEIPT_CONTRACT_VERSION_V1)
                || text_cell(&row.cells[1]) != Some(block.construct_id())
                || text_cell(&row.cells[2]) != Some(indicator.variable_id())
            {
                return invalid("fixed_score_scale_receipt identity or order is non-canonical");
            }
            let center = number_cell(&row.cells[3])?;
            let scale = number_cell(&row.cells[4])?;
            let coefficient = number_cell(&row.cells[5])?;
            let effective = number_cell(&row.cells[6])?;
            if scale <= f64::EPSILON
                || block_scale.is_some_and(|value: f64| value.to_bits() != scale.to_bits())
                || block_center.is_some_and(|value: f64| value.to_bits() != center.to_bits())
                || resolved
                    .get(&(block.construct_id(), indicator.variable_id()))
                    .is_none_or(|stored| stored.to_bits() != coefficient.to_bits())
                || effective.to_bits() != (coefficient / scale).to_bits()
            {
                return invalid("fixed_score_scale_receipt arithmetic or block scale was tampered");
            }
            block_scale = Some(scale);
            block_center = Some(center);
            row_index += 1;
        }
    }
    Ok(())
}

fn expected_weight_rows(
    recipe: &AnalysisRecipeV4,
    plan: &CompiledPlsPlanV2,
) -> Result<Vec<ExpectedWeightRow>, PlsScoreExecutionDocumentV2Error> {
    let configured = match recipe.method_config.as_ref() {
        Some(MethodConfig::PlsAlgorithmConfiguredV2(config)) => Some(config),
        _ => None,
    };
    let individual = configured.and_then(|config| match &config.initial_outer_weights {
        PlsInitialOuterWeightsV2::Individual { weights } => Some(
            weights
                .iter()
                .map(|weight| {
                    (
                        (weight.construct_id.as_str(), weight.indicator_id.as_str()),
                        weight.value,
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        ),
        PlsInitialOuterWeightsV2::Standard => None,
    });
    let initialization_kind = individual.as_ref().map_or("standard", |_| "individual");
    let mut rows = Vec::new();
    for block in plan.blocks() {
        let mode = match block.mode() {
            CompiledPlsBlockModeV2::ModeA => "mode_a",
            CompiledPlsBlockModeV2::ModeB => "mode_b",
        };
        for indicator in block.indicators() {
            let (block_kind, estimated_mode, initialization, normalization, requested, estimated) =
                match block.fixed_scoring() {
                    None => (
                        "estimated",
                        Some(mode),
                        Some(initialization_kind),
                        None,
                        individual.as_ref().map_or(1.0, |weights| {
                            weights[&(block.construct_id(), indicator.variable_id())]
                        }),
                        true,
                    ),
                    Some(CompiledPlsFixedScoringV2::Unit { normalization }) => (
                        "fixed_unit",
                        None,
                        None,
                        Some(normalization_name(*normalization)?),
                        1.0,
                        false,
                    ),
                    Some(CompiledPlsFixedScoringV2::Custom {
                        weights,
                        normalization,
                    }) => (
                        "fixed_custom",
                        None,
                        None,
                        Some(normalization_name(*normalization)?),
                        weights[indicator.variable_id()],
                        false,
                    ),
                };
            rows.push(ExpectedWeightRow {
                construct_id: block.construct_id().into(),
                indicator_id: indicator.variable_id().into(),
                source_column: indicator.source_column().into(),
                block_kind,
                estimated_mode,
                requested_initialization: initialization,
                normalization,
                requested_weight: requested,
                estimated,
            });
        }
    }
    Ok(rows)
}

fn normalization_name(
    normalization: CompositeWeightNormalizationV4,
) -> Result<&'static str, PlsScoreExecutionDocumentV2Error> {
    match normalization {
        CompositeWeightNormalizationV4::UnitVariance => Ok("unit_variance"),
        CompositeWeightNormalizationV4::None => Ok("none"),
        CompositeWeightNormalizationV4::SumToOne => Ok("sum_to_one"),
    }
}

fn canonical_outer_weights(
    document: &CanonicalResultDocumentV2,
) -> Result<BTreeMap<(String, String), f64>, PlsScoreExecutionDocumentV2Error> {
    let outer = table(document, "outer_model")
        .ok_or_else(|| invalid_error("canonical document omitted outer_model"))?;
    validate_exact_columns(outer, &["construct", "indicator", "weight", "loading"])?;
    let mut weights = BTreeMap::new();
    for row in &outer.rows {
        let construct = text_cell(&row.cells[0])
            .ok_or_else(|| invalid_error("outer_model construct must be text"))?;
        let indicator = text_cell(&row.cells[1])
            .ok_or_else(|| invalid_error("outer_model indicator must be text"))?;
        let weight = number_cell(&row.cells[2])?;
        if weights
            .insert((construct.to_owned(), indicator.to_owned()), weight)
            .is_some()
        {
            return invalid("outer_model contains a duplicate construct/indicator identity");
        }
    }
    Ok(weights)
}

fn estimation_summary_iterations(
    document: &CanonicalResultDocumentV2,
) -> Result<u32, PlsScoreExecutionDocumentV2Error> {
    let summary = table(document, "estimation_summary")
        .ok_or_else(|| invalid_error("canonical document omitted estimation_summary"))?;
    validate_exact_columns(
        summary,
        &[
            "converged",
            "iterations",
            "used_observations",
            "omitted_observations",
        ],
    )?;
    if summary.rows.len() != 1 || summary.rows[0].id != "run" {
        return invalid("estimation_summary must contain exactly the run row");
    }
    if !matches!(
        &summary.rows[0].cells[0],
        CanonicalResultCellV2::Boolean { value: true }
    ) {
        return invalid("versioned score execution must be converged");
    }
    integer_cell(&summary.rows[0].cells[1])
}

fn positive_scalar_proportional(weights: &[(f64, f64)]) -> bool {
    let Some((requested_anchor, resolved_anchor)) = weights
        .iter()
        .copied()
        .find(|(requested, _)| *requested != 0.0)
    else {
        return false;
    };
    let scale = resolved_anchor / requested_anchor;
    scale.is_finite()
        && scale > 0.0
        && weights.iter().all(|(requested, resolved)| {
            let expected = requested * scale;
            resolved.is_finite()
                && (resolved - expected).abs()
                    <= 1e-12 * resolved.abs().max(expected.abs()).max(1.0)
        })
}

fn resolved_weights_match(normalization: Option<&str>, weights: &[(f64, f64)]) -> bool {
    match normalization {
        Some("none") => weights
            .iter()
            .all(|(requested, resolved)| requested.to_bits() == resolved.to_bits()),
        Some("sum_to_one") => {
            let sum = weights.iter().map(|(requested, _)| requested).sum::<f64>();
            sum != 0.0
                && sum.is_finite()
                && weights
                    .iter()
                    .all(|(requested, resolved)| (requested / sum).to_bits() == resolved.to_bits())
        }
        Some("unit_variance") | None => positive_scalar_proportional(weights),
        Some(_) => false,
    }
}

fn table<'a>(
    document: &'a CanonicalResultDocumentV2,
    id: &str,
) -> Option<&'a CanonicalResultTableV2> {
    document.tables.iter().find(|table| table.id == id)
}

fn validate_exact_columns(
    table: &CanonicalResultTableV2,
    expected: &[&str],
) -> Result<(), PlsScoreExecutionDocumentV2Error> {
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
        return invalid(format!(
            "table {} has a non-canonical column contract",
            table.id
        ));
    }
    Ok(())
}

fn recorded_sha256(value: &str) -> Option<&str> {
    let candidate = value.rsplit_once(':').map_or(value, |(_, suffix)| suffix);
    (candidate.len() == 64
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    .then_some(candidate)
}

fn text_cell(cell: &CanonicalResultCellV2) -> Option<&str> {
    match cell {
        CanonicalResultCellV2::Text { value } => Some(value),
        _ => None,
    }
}

fn optional_text_cell(
    cell: &CanonicalResultCellV2,
) -> Result<Option<&str>, PlsScoreExecutionDocumentV2Error> {
    match cell {
        CanonicalResultCellV2::Text { value } => Ok(Some(value)),
        CanonicalResultCellV2::Missing {
            reason: CanonicalMissingReasonV2::NotApplicable,
            display: None,
        } => Ok(None),
        _ => invalid("optional score-execution text must be text or not_applicable"),
    }
}

fn number_cell(cell: &CanonicalResultCellV2) -> Result<f64, PlsScoreExecutionDocumentV2Error> {
    match cell {
        CanonicalResultCellV2::Number { value, .. } if value.is_finite() => Ok(*value),
        _ => invalid("score-execution numeric cell must be finite"),
    }
}

fn integer_cell(cell: &CanonicalResultCellV2) -> Result<u32, PlsScoreExecutionDocumentV2Error> {
    let value = number_cell(cell)?;
    if value.fract() != 0.0 || !(0.0..=f64::from(u32::MAX)).contains(&value) {
        return invalid("score-execution count must be an exact nonnegative u32");
    }
    Ok(value as u32)
}

fn integer_cell_u64(cell: &CanonicalResultCellV2) -> Result<u64, PlsScoreExecutionDocumentV2Error> {
    let value = number_cell(cell)?;
    if value.fract() != 0.0 || !(0.0..=9_007_199_254_740_991.0).contains(&value) {
        return invalid("score-execution update count must be an exact safe integer");
    }
    Ok(value as u64)
}

fn invalid<T>(message: impl Into<String>) -> Result<T, PlsScoreExecutionDocumentV2Error> {
    Err(invalid_error(message))
}

fn invalid_error(message: impl Into<String>) -> PlsScoreExecutionDocumentV2Error {
    PlsScoreExecutionDocumentV2Error::Invalid(message.into())
}

#[cfg(test)]
mod tests {
    use super::resolved_weights_match;

    #[test]
    fn fixed_score_normalization_receipts_are_exact_and_tamper_evident() {
        let requested = [(-0.25, -0.25), (0.75, 0.75)];
        assert!(resolved_weights_match(Some("none"), &requested));
        let none_tamper = [
            (-0.25, f64::from_bits((-0.25_f64).to_bits() + 1)),
            (0.75, 0.75),
        ];
        assert!(!resolved_weights_match(Some("none"), &none_tamper));

        let sum_to_one = [(-0.25, -0.5), (0.75, 1.5)];
        assert!(resolved_weights_match(Some("sum_to_one"), &sum_to_one));
        let negative_sum = [(-0.25, 0.25), (-0.75, 0.75)];
        assert!(resolved_weights_match(Some("sum_to_one"), &negative_sum));
        let sum_tamper = [(-0.25, -0.5), (0.75, 1.500_000_000_000_000_2)];
        assert!(!resolved_weights_match(Some("sum_to_one"), &sum_tamper));
        assert!(!resolved_weights_match(
            Some("sum_to_one"),
            &[(-1.0, -1.0), (1.0, 1.0)],
        ));

        let unit_variance = [(-0.25, -0.5), (0.75, 1.5)];
        assert!(resolved_weights_match(
            Some("unit_variance"),
            &unit_variance,
        ));
        assert!(!resolved_weights_match(
            Some("unit_variance"),
            &[(-0.25, -0.5), (0.75, 1.4)],
        ));
    }
}
