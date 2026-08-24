use crate::{
    CbsemCompiledMomentErrorV2, CbsemCompiledMomentProgressV2, CbsemCompiledMomentResultV2,
    estimate_cbsem_ml_compiled_moments_v2_with_control,
};
use arrow::{
    array::{Array, ArrayRef, Float64Array, Int64Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use qpls_core::{
    AnalysisMethod, AnalysisRecipeModelBindingV4, AnalysisRecipeV4,
    CBSEM_PRODUCT_INDICATOR_ESTIMATED_PEAK_BYTES_PER_CELL_V1,
    CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1,
    CBSEM_PRODUCT_INDICATOR_MAX_MATERIALIZED_PRODUCT_CELLS_V1,
    CBSEM_PRODUCT_INDICATOR_PEAK_WORK_MEMORY_CEILING_BYTES_V1,
    CBSEM_PRODUCT_INDICATOR_RAW_BYTES_PER_CELL_V1, CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1,
    CbsemEstimator, CbsemInput, CbsemModelType, CompiledAnalysisRecipeV4,
    CompiledCbsemProductIndicatorErrorV1, CompiledCbsemProductIndicatorPlanV1,
    CompiledRecipePlanV4, MethodConfig, ObservedScaleV4, Preprocessing,
    ProductIndicatorCenteringV4, ProductIndicatorStandardizationV4, RecipeV4CompilationError,
    RecipeV4CompilationReceipt, RecipeV4CompilerTarget, SemModelV4, SemModelV4ValidationError,
    SemVariableV4, compile_analysis_recipe_v4, validate_compiled_analysis_recipe_v4,
};
use qpls_data::{
    ColumnMetadata, ColumnType, DataFingerprint, DataKind, Dataset, DatasetDescriptor,
    DatasetSchema, ScaleType, dataset_from_descriptor, write_arrow,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};

pub const CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1: u32 = 1;
pub const CBSEM_PRODUCT_INDICATOR_TRANSFORMATION_VERSION_V1: &str =
    "cbsem_product_indicator_all_pairs_transform_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorProgressV1 {
    pub phase: String,
    pub completed_units: u64,
    pub total_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorConstituentStatisticV1 {
    pub observed_variable_id: String,
    pub source_column: String,
    pub mean: f64,
    pub sample_standard_deviation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorColumnStatisticV1 {
    pub observed_variable_id: String,
    pub source_column: String,
    pub mean_before_second_centering: f64,
    pub final_mean: f64,
    pub final_sample_standard_deviation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorTransformationProvenanceV1 {
    pub transformation_version: String,
    pub source_dataset_id: String,
    pub source_dataset_fingerprint: String,
    pub transformed_dataset_fingerprint: String,
    pub source_observations: usize,
    pub used_observations: usize,
    pub omitted_observations: usize,
    pub centering: ProductIndicatorCenteringV4,
    pub standardization: ProductIndicatorStandardizationV4,
    pub constituent_statistics: Vec<CbsemProductIndicatorConstituentStatisticV1>,
    pub product_statistics: Vec<CbsemProductIndicatorColumnStatisticV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorExecutionProvenanceV1 {
    pub source_compilation_receipt: RecipeV4CompilationReceipt,
    pub expanded_compilation_receipt: RecipeV4CompilationReceipt,
    pub compiled_product_plan_sha256: String,
    pub transformation: CbsemProductIndicatorTransformationProvenanceV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorOutputExclusionV1 {
    pub code: String,
    pub nested_result_paths: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CbsemProductIndicatorModerationResultV1 {
    pub schema_version: u32,
    pub method_version: String,
    pub provenance: CbsemProductIndicatorExecutionProvenanceV1,
    pub compiled_product_plan: CompiledCbsemProductIndicatorPlanV1,
    pub estimation: CbsemCompiledMomentResultV2,
    /// Machine-readable guardrails for fields produced by the delegated generic
    /// exact engine but not scientifically qualified for product-indicator
    /// moderation. Results/export surfaces must honor these exclusions.
    pub output_exclusions: Vec<CbsemProductIndicatorOutputExclusionV1>,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum CbsemProductIndicatorExecutionErrorV1 {
    #[error("CB-SEM product-indicator moderation was cancelled")]
    Cancelled,
    #[error(transparent)]
    CompiledArtifact(#[from] RecipeV4CompilationError),
    #[error("compiled recipe does not contain the exact CB-SEM product-indicator plan")]
    WrongCompiledPlan,
    #[error("source dataset fingerprint differs from the compiled recipe receipt")]
    DatasetFingerprintMismatch,
    #[error("source dataset bytes or schema fail their declared fingerprint: {0}")]
    DatasetIntegrity(String),
    #[error(
        "product-indicator execution requires the Internal-only CB-SEM raw ML point-estimate recipe: {code}: {message}"
    )]
    UnsupportedRecipe { code: String, message: String },
    #[error(transparent)]
    ProductCompiler(#[from] CompiledCbsemProductIndicatorErrorV1),
    #[error(transparent)]
    InvalidExpandedModel(#[from] SemModelV4ValidationError),
    #[error("required source column is missing or duplicated: {0}")]
    SourceColumnBinding(String),
    #[error("source column {column} contains a nonnumeric value at row {row}")]
    SourceValueInvalid { column: String, row: usize },
    #[error("source column {column} contains a non-finite value at row {row}")]
    SourceValueNonFinite { column: String, row: usize },
    #[error("at least ten listwise-complete observations are required (found {0})")]
    InsufficientObservations(usize),
    #[error("product-indicator execution requires positive variance for {0}")]
    ZeroVariance(String),
    #[error(
        "product-indicator materialization size overflowed for {complete_rows} complete rows and {product_count} product columns"
    )]
    MaterializationSizeOverflow {
        complete_rows: usize,
        product_count: usize,
    },
    #[error(
        "product-indicator materialization exceeds the Internal v1 safety envelope: rows={complete_rows}, product_columns={product_count}, product_cells={materialized_product_cells} (max {max_materialized_product_cells}), raw_bytes={estimated_raw_bytes}, estimated_peak_bytes={estimated_peak_bytes} (max {max_peak_bytes})"
    )]
    MaterializationLimitExceeded {
        complete_rows: usize,
        product_count: usize,
        materialized_product_cells: u64,
        max_materialized_product_cells: u64,
        estimated_raw_bytes: u64,
        estimated_peak_bytes: u64,
        max_peak_bytes: u64,
    },
    #[error(
        "product-indicator transformation produced a non-finite value or statistic for {column} during {stage}{row_suffix}",
        row_suffix = .row.map(|row| format!(" at row {row}")).unwrap_or_default()
    )]
    NonFiniteTransformation {
        column: String,
        stage: &'static str,
        row: Option<usize>,
    },
    #[error("the transformed Arrow dataset could not be constructed: {0}")]
    TransformedDataset(String),
    #[error("the compiled expanded plan differs from the immutable product materialization")]
    ExpandedPlanMismatch,
    #[error("expanded exact CB-SEM estimation failed: {0}")]
    Estimation(CbsemCompiledMomentErrorV2),
}

struct PreparedProductDatasetV1 {
    dataset: Dataset,
    provenance: CbsemProductIndicatorTransformationProvenanceV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProductMaterializationEnvelopeV1 {
    materialized_product_cells: u64,
    estimated_raw_bytes: u64,
    estimated_peak_bytes: u64,
}

pub fn estimate_internal_cbsem_product_indicator_moderation_v1(
    dataset: &Dataset,
    source_artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
) -> Result<CbsemProductIndicatorModerationResultV1, CbsemProductIndicatorExecutionErrorV1> {
    estimate_internal_cbsem_product_indicator_moderation_v1_with_control(
        dataset,
        source_artifact,
        source_recipe,
        resolved_model,
        || false,
        |_| {},
    )
}

/// Internal-only point-estimate execution path. It revalidates the source
/// Recipe-v4 artifact, materializes an immutable product-indicator factor,
/// creates a deterministic listwise-complete derived dataset, and delegates
/// the expanded model to the exact CB-SEM parameter-table engine.
pub fn estimate_internal_cbsem_product_indicator_moderation_v1_with_control(
    dataset: &Dataset,
    source_artifact: &CompiledAnalysisRecipeV4,
    source_recipe: &AnalysisRecipeV4,
    resolved_model: &SemModelV4,
    should_cancel: impl Fn() -> bool + Sync,
    progress: impl Fn(CbsemProductIndicatorProgressV1) + Sync,
) -> Result<CbsemProductIndicatorModerationResultV1, CbsemProductIndicatorExecutionErrorV1> {
    checkpoint(&should_cancel, &progress, "integrity", 0, 5)?;
    validate_compiled_analysis_recipe_v4(source_artifact, source_recipe, Some(resolved_model))?;
    validate_recipe_contract(source_recipe)?;
    if source_artifact.receipt().dataset_fingerprint() != dataset.fingerprint.0 {
        return Err(CbsemProductIndicatorExecutionErrorV1::DatasetFingerprintMismatch);
    }
    validate_dataset_integrity(dataset)?;
    let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 {
        plan: compiled_product_plan,
    } = source_artifact.plan()
    else {
        return Err(CbsemProductIndicatorExecutionErrorV1::WrongCompiledPlan);
    };
    checkpoint(&should_cancel, &progress, "integrity", 1, 5)?;
    let compiled_product_plan = compiled_product_plan.clone();
    checkpoint(&should_cancel, &progress, "materialization", 2, 5)?;
    let prepared =
        prepare_product_dataset(dataset, &compiled_product_plan, &should_cancel, &progress)?;
    checkpoint(&should_cancel, &progress, "transformation", 3, 5)?;

    let expanded_model = compiled_product_plan
        .expanded_plan()
        .to_scientific_sem_model_v4();
    let mut expanded_recipe = source_recipe.clone();
    expanded_recipe.dataset_fingerprint = prepared.dataset.fingerprint.0.clone();
    expanded_recipe.model_binding = AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
        scientific_sha256: expanded_model.scientific_sha256()?,
        model: expanded_model.clone(),
    };
    expanded_recipe.metadata.insert(
        "internal_product_indicator_source_plan_sha256".into(),
        compiled_product_plan.source_plan_sha256().into(),
    );
    let target = RecipeV4CompilerTarget::CbsemPlanV2;
    let expanded_artifact = compile_analysis_recipe_v4(
        &expanded_recipe,
        Some(&expanded_model),
        target,
        target.capability_cell(),
    )?;
    let CompiledRecipePlanV4::CbsemPlanV2 {
        plan: recompiled_expanded,
    } = expanded_artifact.plan()
    else {
        unreachable!()
    };
    if recompiled_expanded != compiled_product_plan.expanded_plan() {
        return Err(CbsemProductIndicatorExecutionErrorV1::ExpandedPlanMismatch);
    }
    checkpoint(&should_cancel, &progress, "expanded_compilation", 4, 5)?;
    let estimation_progress = |update: CbsemCompiledMomentProgressV2| {
        progress(CbsemProductIndicatorProgressV1 {
            phase: format!("estimation:{}", update.phase),
            completed_units: update.completed_units,
            total_units: update.total_units,
        });
    };
    let estimation = estimate_cbsem_ml_compiled_moments_v2_with_control(
        &prepared.dataset,
        &expanded_artifact,
        &expanded_recipe,
        &expanded_model,
        &should_cancel,
        &estimation_progress,
    )
    .map_err(|error| match error {
        CbsemCompiledMomentErrorV2::Cancelled => CbsemProductIndicatorExecutionErrorV1::Cancelled,
        other => CbsemProductIndicatorExecutionErrorV1::Estimation(other),
    })?;
    checkpoint(&should_cancel, &progress, "result", 5, 5)?;
    Ok(CbsemProductIndicatorModerationResultV1 {
        schema_version: CBSEM_PRODUCT_INDICATOR_RESULT_SCHEMA_VERSION_V1,
        method_version: compiled_product_plan.method_version().into(),
        provenance: CbsemProductIndicatorExecutionProvenanceV1 {
            source_compilation_receipt: source_artifact.receipt().clone(),
            expanded_compilation_receipt: expanded_artifact.receipt().clone(),
            compiled_product_plan_sha256: compiled_product_plan.deterministic_sha256(),
            transformation: prepared.provenance,
        },
        compiled_product_plan,
        estimation,
        output_exclusions: product_indicator_output_exclusions(),
        warnings: vec![
            "This unregistered Internal source slice places only the latent interaction point estimate and transformation provenance in scope. Do not present nested normal-theory inference or global-fit statistics as product-indicator moderation results.".into(),
        ],
    })
}

fn product_indicator_output_exclusions() -> Vec<CbsemProductIndicatorOutputExclusionV1> {
    vec![
        CbsemProductIndicatorOutputExclusionV1 {
            code: "normal_theory_inference_unqualified".into(),
            nested_result_paths: vec![
                "estimation.analysis.parameters[*].standard_error".into(),
                "estimation.analysis.parameters[*].z_statistic".into(),
                "estimation.analysis.parameters[*].p_value_two_sided".into(),
            ],
            message: "Product indicators are nonnormal; the delegated generic engine's normal-theory standard errors, z statistics, and p values are excluded from this point-estimate contract.".into(),
        },
        CbsemProductIndicatorOutputExclusionV1 {
            code: "interaction_aware_global_fit_unqualified".into(),
            nested_result_paths: vec!["estimation.analysis.fit".into()],
            message: "The delegated generic covariance-model fit block is not qualified as interaction-aware global fit for product-indicator moderation.".into(),
        },
    ]
}

fn validate_recipe_contract(
    recipe: &AnalysisRecipeV4,
) -> Result<(), CbsemProductIndicatorExecutionErrorV1> {
    if recipe.settings.method != AnalysisMethod::Cbsem {
        return unsupported_recipe(
            "cbsem_method_required",
            "Select the CB-SEM analysis family for this latent interaction.",
        );
    }
    if recipe.settings.preprocessing != Preprocessing::Unstandardized {
        return unsupported_recipe(
            "unstandardized_base_preprocessing_required",
            "Keep base CB-SEM preprocessing unstandardized; product centering and scaling are explicit in the interaction specification.",
        );
    }
    if recipe.settings.bootstrap_samples != 0
        || recipe.settings.studentized_inner_samples != 0
        || recipe.settings.permutation_samples != 0
    {
        return unsupported_recipe(
            "resampling_settings_unsupported",
            "Set bootstrap, studentized-inner, and permutation sample counts to zero. Product-indicator inference is a separate unqualified option cell.",
        );
    }
    if recipe.settings.max_iterations != CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1
        || recipe.settings.tolerance.to_bits() != CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1.to_bits()
    {
        return unsupported_recipe(
            "exact_optimizer_contract_required",
            format!(
                "Set max_iterations={} and tolerance={}. These fields identify the fixed exact-engine convergence contract for method v1.",
                CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1, CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1
            ),
        );
    }
    let Some(MethodConfig::Cbsem {
        model_type: CbsemModelType::Sem,
        estimator: CbsemEstimator::Ml,
        input: CbsemInput::Raw,
        mean_structure: false,
        bootstrap_samples: 0,
        bootstrap_v2: None,
        group_column: None,
        invariance_steps,
    }) = recipe.method_config.as_ref()
    else {
        return unsupported_recipe(
            "raw_ml_point_estimate_required",
            "Use single-group raw-data CB-SEM ML without mean structure or bootstrap. Inference is a separate unqualified cell.",
        );
    };
    if !invariance_steps.is_empty() {
        return unsupported_recipe(
            "invariance_options_unsupported",
            "Remove measurement-invariance steps from the product-indicator point estimate.",
        );
    }
    Ok(())
}

fn ensure_product_materialization_envelope_v1(
    complete_rows: usize,
    product_count: usize,
) -> Result<ProductMaterializationEnvelopeV1, CbsemProductIndicatorExecutionErrorV1> {
    let complete_rows_u64 = u64::try_from(complete_rows).map_err(|_| {
        CbsemProductIndicatorExecutionErrorV1::MaterializationSizeOverflow {
            complete_rows,
            product_count,
        }
    })?;
    let product_count_u64 = u64::try_from(product_count).map_err(|_| {
        CbsemProductIndicatorExecutionErrorV1::MaterializationSizeOverflow {
            complete_rows,
            product_count,
        }
    })?;
    let materialized_product_cells = complete_rows_u64.checked_mul(product_count_u64).ok_or(
        CbsemProductIndicatorExecutionErrorV1::MaterializationSizeOverflow {
            complete_rows,
            product_count,
        },
    )?;
    let estimated_raw_bytes = materialized_product_cells
        .checked_mul(CBSEM_PRODUCT_INDICATOR_RAW_BYTES_PER_CELL_V1)
        .ok_or(
            CbsemProductIndicatorExecutionErrorV1::MaterializationSizeOverflow {
                complete_rows,
                product_count,
            },
        )?;
    let estimated_peak_bytes = materialized_product_cells
        .checked_mul(CBSEM_PRODUCT_INDICATOR_ESTIMATED_PEAK_BYTES_PER_CELL_V1)
        .ok_or(
            CbsemProductIndicatorExecutionErrorV1::MaterializationSizeOverflow {
                complete_rows,
                product_count,
            },
        )?;
    if materialized_product_cells > CBSEM_PRODUCT_INDICATOR_MAX_MATERIALIZED_PRODUCT_CELLS_V1
        || estimated_peak_bytes > CBSEM_PRODUCT_INDICATOR_PEAK_WORK_MEMORY_CEILING_BYTES_V1
    {
        return Err(
            CbsemProductIndicatorExecutionErrorV1::MaterializationLimitExceeded {
                complete_rows,
                product_count,
                materialized_product_cells,
                max_materialized_product_cells:
                    CBSEM_PRODUCT_INDICATOR_MAX_MATERIALIZED_PRODUCT_CELLS_V1,
                estimated_raw_bytes,
                estimated_peak_bytes,
                max_peak_bytes: CBSEM_PRODUCT_INDICATOR_PEAK_WORK_MEMORY_CEILING_BYTES_V1,
            },
        );
    }
    Ok(ProductMaterializationEnvelopeV1 {
        materialized_product_cells,
        estimated_raw_bytes,
        estimated_peak_bytes,
    })
}

fn prepare_product_dataset(
    source: &Dataset,
    plan: &CompiledCbsemProductIndicatorPlanV1,
    should_cancel: &impl Fn() -> bool,
    progress: &impl Fn(CbsemProductIndicatorProgressV1),
) -> Result<PreparedProductDatasetV1, CbsemProductIndicatorExecutionErrorV1> {
    if source.schema.kind != DataKind::Raw {
        return unsupported_recipe(
            "raw_dataset_required",
            "Product indicators require case-level raw observations.",
        );
    }
    let product_columns = plan
        .product_indicators()
        .iter()
        .map(|indicator| indicator.source_column())
        .collect::<HashSet<_>>();
    let mut base_columns = plan
        .expanded_plan()
        .variables()
        .iter()
        .filter_map(|variable| match variable {
            SemVariableV4::Observed {
                source_column,
                scale: ObservedScaleV4::Continuous,
                ..
            } if !product_columns.contains(source_column.as_str()) => Some(source_column.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    base_columns.sort();
    base_columns.dedup();
    let positions = resolve_source_positions(source, &base_columns)?;
    let total_rows = source.batch.num_rows();
    let mut complete_row_indices = Vec::new();
    for row in 0..total_rows {
        if row % 128 == 0 {
            checkpoint(
                should_cancel,
                progress,
                "listwise_rows",
                row as u64,
                total_rows as u64,
            )?;
        }
        if positions
            .values()
            .any(|position| source.batch.column(*position).is_null(row))
        {
            continue;
        }
        for column in &base_columns {
            let position = positions[column];
            let value =
                numeric_cell(source.batch.column(position).as_ref(), row).ok_or_else(|| {
                    CbsemProductIndicatorExecutionErrorV1::SourceValueInvalid {
                        column: column.clone(),
                        row,
                    }
                })?;
            if !value.is_finite() {
                return Err(
                    CbsemProductIndicatorExecutionErrorV1::SourceValueNonFinite {
                        column: column.clone(),
                        row,
                    },
                );
            }
        }
        complete_row_indices.push(row);
    }
    let used = complete_row_indices.len();
    if used < 10 {
        return Err(CbsemProductIndicatorExecutionErrorV1::InsufficientObservations(used));
    }
    ensure_product_materialization_envelope_v1(used, plan.product_indicators().len())?;
    checkpoint(
        should_cancel,
        progress,
        "listwise_rows",
        total_rows as u64,
        total_rows as u64,
    )?;

    let mut complete = base_columns
        .iter()
        .map(|column| (column.clone(), Vec::with_capacity(used)))
        .collect::<BTreeMap<_, Vec<f64>>>();
    let base_materialization_units = used.saturating_mul(base_columns.len()) as u64;
    for (complete_index, row) in complete_row_indices.into_iter().enumerate() {
        for (column_index, column) in base_columns.iter().enumerate() {
            if column_index == 0 && complete_index % 128 == 0 {
                checkpoint(
                    should_cancel,
                    progress,
                    "base_materialization",
                    complete_index.saturating_mul(base_columns.len()) as u64,
                    base_materialization_units,
                )?;
            }
            let position = positions[column];
            let value = numeric_cell(source.batch.column(position).as_ref(), row)
                .expect("the validated complete row remains numeric");
            complete
                .get_mut(column)
                .expect("base column was initialized")
                .push(value);
        }
    }
    checkpoint(
        should_cancel,
        progress,
        "base_materialization",
        base_materialization_units,
        base_materialization_units,
    )?;

    let constituent_sources = plan
        .predictor_indicators()
        .iter()
        .chain(plan.moderator_indicators())
        .map(|source| {
            (
                source.observed_variable_id().to_owned(),
                source.source_column().to_owned(),
            )
        })
        .collect::<BTreeSet<_>>();
    let mut constituent_statistics = Vec::new();
    let mut constituent_values = HashMap::<String, Vec<f64>>::new();
    let constituent_total_units = constituent_sources.len().saturating_mul(used) as u64;
    for (constituent_index, (observed_variable_id, source_column)) in
        constituent_sources.into_iter().enumerate()
    {
        let values = &complete[&source_column];
        let statistics =
            stable_sample_statistics(values, &source_column, "constituent_statistics")?;
        if !statistics.has_positive_variance {
            return Err(CbsemProductIndicatorExecutionErrorV1::ZeroVariance(
                source_column,
            ));
        }
        let mean = statistics.mean;
        let standard_deviation = statistics.sample_standard_deviation;
        let mut transformed = Vec::with_capacity(values.len());
        for (row, value) in values.iter().enumerate() {
            if row % 1_024 == 0 {
                checkpoint(
                    should_cancel,
                    progress,
                    "constituent_rows",
                    constituent_index.saturating_mul(used).saturating_add(row) as u64,
                    constituent_total_units,
                )?;
            }
            let centered = match plan.specification().centering {
                ProductIndicatorCenteringV4::None => *value,
                ProductIndicatorCenteringV4::MeanCenter
                | ProductIndicatorCenteringV4::DoubleMeanCenter => *value - mean,
            };
            let transformed_value = match plan.specification().standardization {
                ProductIndicatorStandardizationV4::None => centered,
                ProductIndicatorStandardizationV4::SampleStandardDeviation => {
                    statistics.standardized(*value)
                }
            };
            if !transformed_value.is_finite() {
                return Err(
                    CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                        column: source_column.clone(),
                        stage: "constituent_transformation",
                        row: Some(row),
                    },
                );
            }
            transformed.push(transformed_value);
        }
        constituent_values.insert(source_column.clone(), transformed);
        constituent_statistics.push(CbsemProductIndicatorConstituentStatisticV1 {
            observed_variable_id,
            source_column,
            mean,
            sample_standard_deviation: standard_deviation,
        });
    }
    constituent_statistics.sort_by(|left, right| {
        (&left.observed_variable_id, &left.source_column)
            .cmp(&(&right.observed_variable_id, &right.source_column))
    });

    let mut products = BTreeMap::<String, Vec<f64>>::new();
    let mut product_statistics = Vec::new();
    let product_total_units = plan.product_indicators().len().saturating_mul(used) as u64;
    for (index, product) in plan.product_indicators().iter().enumerate() {
        let predictor_values = &constituent_values[product.predictor_source_column()];
        let moderator_values = &constituent_values[product.moderator_source_column()];
        let mut values = Vec::with_capacity(used);
        for row in 0..used {
            if row % 1_024 == 0 {
                checkpoint(
                    should_cancel,
                    progress,
                    "product_rows",
                    index.saturating_mul(used).saturating_add(row) as u64,
                    product_total_units,
                )?;
            }
            let value = predictor_values[row] * moderator_values[row];
            if !value.is_finite() {
                return Err(
                    CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                        column: product.source_column().into(),
                        stage: "product_multiplication",
                        row: Some(row),
                    },
                );
            }
            values.push(value);
        }
        let mean_before_second_centering = stable_mean(&values);
        if !mean_before_second_centering.is_finite() {
            return Err(
                CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                    column: product.source_column().into(),
                    stage: "product_mean",
                    row: None,
                },
            );
        }
        if plan.specification().centering == ProductIndicatorCenteringV4::DoubleMeanCenter {
            for (row, value) in values.iter_mut().enumerate() {
                if row % 1_024 == 0 {
                    checkpoint(
                        should_cancel,
                        progress,
                        "product_second_centering_rows",
                        index.saturating_mul(used).saturating_add(row) as u64,
                        product_total_units,
                    )?;
                }
                *value -= mean_before_second_centering;
                if !value.is_finite() {
                    return Err(
                        CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                            column: product.source_column().into(),
                            stage: "product_second_centering",
                            row: Some(row),
                        },
                    );
                }
            }
        }
        let statistics =
            stable_sample_statistics(&values, product.source_column(), "product_statistics")?;
        if !statistics.has_positive_variance {
            return Err(CbsemProductIndicatorExecutionErrorV1::ZeroVariance(
                product.source_column().into(),
            ));
        }
        let final_mean = statistics.mean;
        let final_sample_standard_deviation = statistics.sample_standard_deviation;
        product_statistics.push(CbsemProductIndicatorColumnStatisticV1 {
            observed_variable_id: product.observed_variable_id().into(),
            source_column: product.source_column().into(),
            mean_before_second_centering,
            final_mean,
            final_sample_standard_deviation,
        });
        products.insert(product.source_column().into(), values);
    }
    checkpoint(
        should_cancel,
        progress,
        "product_rows",
        product_total_units,
        product_total_units,
    )?;

    let mut output_columns = base_columns;
    output_columns.extend(
        plan.product_indicators()
            .iter()
            .map(|indicator| indicator.source_column().to_owned()),
    );
    let fields = output_columns
        .iter()
        .map(|name| Field::new(name, DataType::Float64, false))
        .collect::<Vec<_>>();
    let arrays = output_columns
        .iter()
        .enumerate()
        .map(|(index, name)| {
            checkpoint(
                should_cancel,
                progress,
                "arrow_columns",
                index as u64,
                output_columns.len() as u64,
            )?;
            let values = complete
                .get(name)
                .or_else(|| products.get(name))
                .ok_or_else(|| {
                    CbsemProductIndicatorExecutionErrorV1::TransformedDataset(format!(
                        "missing materialized column {name}"
                    ))
                })?;
            Ok(Arc::new(Float64Array::from(values.clone())) as ArrayRef)
        })
        .collect::<Result<Vec<_>, CbsemProductIndicatorExecutionErrorV1>>()?;
    checkpoint(
        should_cancel,
        progress,
        "arrow_columns",
        output_columns.len() as u64,
        output_columns.len() as u64,
    )?;
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays).map_err(|error| {
        CbsemProductIndicatorExecutionErrorV1::TransformedDataset(error.to_string())
    })?;
    let schema = DatasetSchema {
        version: source.schema.version,
        kind: DataKind::Raw,
        columns: output_columns
            .iter()
            .map(|name| ColumnMetadata {
                name: name.clone(),
                label: None,
                column_type: ColumnType::Numeric,
                scale_type: ScaleType::Continuous,
                missing_markers: Vec::new(),
                theoretical_min: None,
                theoretical_max: None,
                value_labels: BTreeMap::new(),
            })
            .collect(),
        case_count: used,
        sample_size: None,
    };
    checkpoint(should_cancel, progress, "fingerprint", 0, 1)?;
    let fingerprint = fingerprint_v2(&batch, &schema)?;
    checkpoint(should_cancel, progress, "fingerprint", 1, 1)?;
    let dataset = Dataset {
        id: source.id,
        name: format!("{} [product-indicator materialization]", source.name),
        schema,
        batch,
        fingerprint: fingerprint.clone(),
    };
    validate_dataset_integrity(&dataset)?;
    Ok(PreparedProductDatasetV1 {
        dataset,
        provenance: CbsemProductIndicatorTransformationProvenanceV1 {
            transformation_version: CBSEM_PRODUCT_INDICATOR_TRANSFORMATION_VERSION_V1.into(),
            source_dataset_id: source.id.to_string(),
            source_dataset_fingerprint: source.fingerprint.0.clone(),
            transformed_dataset_fingerprint: fingerprint.0,
            source_observations: total_rows,
            used_observations: used,
            omitted_observations: total_rows - used,
            centering: plan.specification().centering,
            standardization: plan.specification().standardization,
            constituent_statistics,
            product_statistics,
        },
    })
}

fn resolve_source_positions(
    dataset: &Dataset,
    names: &[String],
) -> Result<HashMap<String, usize>, CbsemProductIndicatorExecutionErrorV1> {
    let mut positions = HashMap::new();
    for name in names {
        let matches = dataset
            .schema
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.name == *name)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0] >= dataset.batch.num_columns() {
            return Err(CbsemProductIndicatorExecutionErrorV1::SourceColumnBinding(
                name.clone(),
            ));
        }
        positions.insert(name.clone(), matches[0]);
    }
    Ok(positions)
}

fn numeric_cell(array: &dyn Array, row: usize) -> Option<f64> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(row));
    }
    array
        .as_any()
        .downcast_ref::<Int64Array>()
        .map(|values| values.value(row) as f64)
}

fn stable_mean(values: &[f64]) -> f64 {
    let scale = values
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    let denominator = values.len() as f64;
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for value in values {
        let term = (*value / scale) / denominator;
        let corrected = term - compensation;
        let next = sum + corrected;
        compensation = (next - sum) - corrected;
        sum = next;
    }
    sum * scale
}

struct StableSampleStatistics {
    mean: f64,
    sample_standard_deviation: f64,
    scale: f64,
    scaled_mean: f64,
    scaled_sample_standard_deviation: f64,
    has_positive_variance: bool,
}

impl StableSampleStatistics {
    fn standardized(&self, value: f64) -> f64 {
        (value / self.scale - self.scaled_mean) / self.scaled_sample_standard_deviation
    }
}

fn stable_sample_statistics(
    values: &[f64],
    column: &str,
    stage: &'static str,
) -> Result<StableSampleStatistics, CbsemProductIndicatorExecutionErrorV1> {
    let scale = values
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() {
        return Err(
            CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                column: column.into(),
                stage,
                row: None,
            },
        );
    }
    if scale == 0.0 {
        return Ok(StableSampleStatistics {
            mean: 0.0,
            sample_standard_deviation: 0.0,
            scale: 1.0,
            scaled_mean: 0.0,
            scaled_sample_standard_deviation: 0.0,
            has_positive_variance: false,
        });
    }

    let denominator = values.len() as f64;
    let mut scaled_mean = 0.0;
    let mut mean_compensation = 0.0;
    for value in values {
        let term = (*value / scale) / denominator;
        let corrected = term - mean_compensation;
        let next = scaled_mean + corrected;
        mean_compensation = (next - scaled_mean) - corrected;
        scaled_mean = next;
    }

    let mut scaled_sum_squares = 0.0;
    let mut square_compensation = 0.0;
    for value in values {
        let deviation = *value / scale - scaled_mean;
        let term = deviation * deviation;
        let corrected = term - square_compensation;
        let next = scaled_sum_squares + corrected;
        square_compensation = (next - scaled_sum_squares) - corrected;
        scaled_sum_squares = next;
    }
    let has_positive_variance = scaled_sum_squares > 0.0;
    let scaled_sample_standard_deviation = (scaled_sum_squares / (values.len() - 1) as f64).sqrt();
    let mean = scaled_mean * scale;
    let sample_standard_deviation = scaled_sample_standard_deviation * scale;
    if !mean.is_finite()
        || !scaled_sample_standard_deviation.is_finite()
        || (has_positive_variance
            && (!sample_standard_deviation.is_finite() || sample_standard_deviation == 0.0))
    {
        return Err(
            CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                column: column.into(),
                stage,
                row: None,
            },
        );
    }
    Ok(StableSampleStatistics {
        mean,
        sample_standard_deviation,
        scale,
        scaled_mean,
        scaled_sample_standard_deviation,
        has_positive_variance,
    })
}

fn fingerprint_v2(
    batch: &RecordBatch,
    schema: &DatasetSchema,
) -> Result<DataFingerprint, CbsemProductIndicatorExecutionErrorV1> {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-dataset-fingerprint-v2\0");
    digest.update(serde_json::to_vec(schema).map_err(|error| {
        CbsemProductIndicatorExecutionErrorV1::TransformedDataset(error.to_string())
    })?);
    digest.update(write_arrow(batch).map_err(|error| {
        CbsemProductIndicatorExecutionErrorV1::TransformedDataset(error.to_string())
    })?);
    Ok(DataFingerprint(format!("v2:{:x}", digest.finalize())))
}

fn validate_dataset_integrity(
    dataset: &Dataset,
) -> Result<(), CbsemProductIndicatorExecutionErrorV1> {
    let bytes = write_arrow(&dataset.batch).map_err(|error| {
        CbsemProductIndicatorExecutionErrorV1::DatasetIntegrity(error.to_string())
    })?;
    dataset_from_descriptor(DatasetDescriptor::from(dataset), &bytes)
        .map(|_| ())
        .map_err(|error| CbsemProductIndicatorExecutionErrorV1::DatasetIntegrity(error.to_string()))
}

fn checkpoint(
    should_cancel: &impl Fn() -> bool,
    progress: &impl Fn(CbsemProductIndicatorProgressV1),
    phase: &str,
    completed_units: u64,
    total_units: u64,
) -> Result<(), CbsemProductIndicatorExecutionErrorV1> {
    if should_cancel() {
        return Err(CbsemProductIndicatorExecutionErrorV1::Cancelled);
    }
    progress(CbsemProductIndicatorProgressV1 {
        phase: phase.into(),
        completed_units: completed_units.min(total_units),
        total_units: total_units.max(1),
    });
    Ok(())
}

fn unsupported_recipe<T>(
    code: impl Into<String>,
    message: impl Into<String>,
) -> Result<T, CbsemProductIndicatorExecutionErrorV1> {
    Err(CbsemProductIndicatorExecutionErrorV1::UnsupportedRecipe {
        code: code.into(),
        message: message.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_V4_SCHEMA_VERSION, AnalysisRecipeModelBindingV4, AnalysisSettings,
        Construct, InteractionMethodV4, LegacyBasicModelInterpretationV4,
        LegacyEstimandConfirmationV4, MeasurementMode, ModelSpec, ProductIndicatorPairingV4,
        ProductIndicatorSpecificationV4, SemDerivedTermV4, SemEndpointV4, SemParameterTargetV4,
        SemParameterV4, SemRelationV4, SemVariableV4, StructuralPath, StructuralRelationRoleV4,
        convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use std::sync::atomic::{AtomicBool, Ordering};
    use uuid::Uuid;

    fn fixture(
        reverse_rows: bool,
        constant_predictor_indicator: bool,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        fixture_with_options(
            reverse_rows,
            constant_predictor_indicator,
            1.0,
            1.0,
            ProductIndicatorStandardizationV4::None,
        )
    }

    fn fixture_with_options(
        reverse_rows: bool,
        constant_predictor_indicator: bool,
        predictor_scale: f64,
        moderator_scale: f64,
        standardization: ProductIndicatorStandardizationV4,
    ) -> (
        Dataset,
        AnalysisRecipeV4,
        SemModelV4,
        CompiledAnalysisRecipeV4,
    ) {
        let mut rows = (0..180)
            .map(|index| {
                let a = ((index * 37 + 11) % 101) as f64 / 50.0 - 1.0;
                let b = ((index * 53 + 17) % 103) as f64 / 51.0 - 1.0;
                let c = ((index * 29 + 7) % 97) as f64 / 48.0 - 1.0;
                let e_x1 = ((index * 19 + 3) % 89) as f64 / 44.0 - 1.0;
                let e_x2 = ((index * 23 + 5) % 83) as f64 / 41.0 - 1.0;
                let e_m1 = ((index * 31 + 9) % 79) as f64 / 39.0 - 1.0;
                let e_m2 = ((index * 17 + 13) % 73) as f64 / 36.0 - 1.0;
                let e_y1 = ((index * 27 + 15) % 71) as f64 / 35.0 - 1.0;
                let e_y2 = ((index * 11 + 19) % 67) as f64 / 33.0 - 1.0;
                let e_y3 = ((index * 7 + 21) % 61) as f64 / 30.0 - 1.0;
                // Keep the latent, disturbance, and indicator-error scales close
                // to the exact engine's declared generic starts. The prior tiny
                // residuals produced a full-rank but 26,000:1 covariance
                // condition number and tested optimizer scale recovery rather
                // than product-indicator semantics.
                let x = 1.5 * a + 0.18 * b;
                let m = 1.5 * b + 0.12 * a;
                let y = 0.45 * x + 0.30 * m + 0.35 * x * m + 0.90 * c;
                let mut row = [
                    x + 1.1 * e_x1,
                    if constant_predictor_indicator {
                        1.0
                    } else {
                        0.82 * x + 1.1 * e_x2
                    },
                    m + 1.1 * e_m1,
                    0.86 * m - 1.1 * e_m2,
                    y + 1.1 * e_y1,
                    0.83 * y + 1.1 * e_y2,
                    0.68 * y - 1.1 * e_y3,
                ];
                row[0] *= predictor_scale;
                row[1] *= predictor_scale;
                row[2] *= moderator_scale;
                row[3] *= moderator_scale;
                row
            })
            .collect::<Vec<_>>();
        if reverse_rows {
            rows.reverse();
        }
        let mut csv = String::from("x1,x2,m1,m2,y1,y2,y3\n");
        for row in rows {
            csv.push_str(&format!(
                "{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}\n",
                row[0], row[1], row[2], row[3], row[4], row[5], row[6]
            ));
        }
        let dataset = import_delimited_bytes(
            csv.as_bytes(),
            "cbsem-product-indicator-fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let legacy = ModelSpec {
            id: Uuid::from_u128(0xCB5E_5100),
            name: "CB-SEM product indicator fixture".into(),
            constructs: vec![
                construct("x", &["x1", "x2"]),
                construct("m", &["m1", "m2"]),
                construct("y", &["y1", "y2", "y3"]),
            ],
            paths: vec![path("x", "y"), path("m", "y")],
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let mut model = convert_legacy_basic_model_v4(
            &legacy,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap();
        model.data_binding = qpls_core::SemDataBindingV4::Raw {
            dataset_id: dataset.id.to_string(),
            missing_data: qpls_core::MissingDataPolicyV4::ListwiseDeletion,
            weight: None,
            cluster_variable: None,
            strata_variable: None,
        };
        let (left, right) = (
            SemEndpointV4::Variable("construct:m".into()),
            SemEndpointV4::Variable("construct:x".into()),
        );
        model.relations.push(SemRelationV4::Covariance {
            id: "relation:covariance:m:x".into(),
            left: left.clone(),
            right: right.clone(),
            parameter: "parameter:covariance:m:x".into(),
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:covariance:m:x".into(),
            label: "Cov(M,X)".into(),
            target: SemParameterTargetV4::Covariance { left, right },
            start: Some(0.1),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        model.variables.push(SemVariableV4::Derived {
            id: "derived:x_by_m".into(),
            label: "X × M".into(),
        });
        model.relations.push(SemRelationV4::Structural {
            id: "relation:interaction_effect".into(),
            source: "derived:x_by_m".into(),
            target: "construct:y".into(),
            parameter: "parameter:interaction_effect".into(),
            intercept_parameter: None,
            role: StructuralRelationRoleV4::Structural,
        });
        model.parameters.push(SemParameterV4::Free {
            id: "parameter:interaction_effect".into(),
            label: "X × M -> Y".into(),
            target: SemParameterTargetV4::Regression {
                source: "derived:x_by_m".into(),
                target: "construct:y".into(),
            },
            start: Some(0.3),
            lower: None,
            upper: None,
            equality_label: None,
            group_overrides: Vec::new(),
        });
        let focal_relation = model
            .relations
            .iter()
            .find_map(|relation| match relation {
                SemRelationV4::Structural {
                    id, source, target, ..
                } if source == "construct:x" && target == "construct:y" => Some(id.clone()),
                _ => None,
            })
            .unwrap();
        model.derived_terms.push(SemDerivedTermV4::Interaction {
            id: "term:x_by_m".into(),
            output: "derived:x_by_m".into(),
            predictor: "construct:x".into(),
            moderator: "construct:m".into(),
            focal_relation,
            method: InteractionMethodV4::ProductIndicator,
            product_indicator: Some(ProductIndicatorSpecificationV4 {
                centering: ProductIndicatorCenteringV4::DoubleMeanCenter,
                standardization,
                pairing: ProductIndicatorPairingV4::AllPairs,
            }),
        });
        model.ensure_valid().unwrap();
        let mut settings = AnalysisSettings::default();
        settings.method = AnalysisMethod::Cbsem;
        settings.preprocessing = Preprocessing::Unstandardized;
        settings.max_iterations = CBSEM_PRODUCT_INDICATOR_MAX_ITERATIONS_V1;
        settings.tolerance = CBSEM_PRODUCT_INDICATOR_TOLERANCE_V1;
        settings.workers = 1;
        let recipe = AnalysisRecipeV4 {
            schema_version: ANALYSIS_RECIPE_V4_SCHEMA_VERSION,
            id: Uuid::from_u128(0xCB5E_5101),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model_binding: AnalysisRecipeModelBindingV4::EmbeddedSemModelV4 {
                scientific_sha256: model.scientific_sha256().unwrap(),
                model: model.clone(),
            },
            estimand_confirmation: LegacyEstimandConfirmationV4::ConfirmedCommonFactor,
            settings,
            method_config: Some(MethodConfig::Cbsem {
                model_type: CbsemModelType::Sem,
                estimator: CbsemEstimator::Ml,
                input: CbsemInput::Raw,
                mean_structure: false,
                bootstrap_samples: 0,
                bootstrap_v2: None,
                group_column: None,
                invariance_steps: Vec::new(),
            }),
            general_sem_config: None,
            mga_multigroup: None,
            pls_heterogeneity: None,
            general_sem_conditional_process: None,
            interventional_causal_mediation: None,
            metadata: BTreeMap::new(),
            legacy_source: None,
        };
        let target = RecipeV4CompilerTarget::CbsemProductIndicatorPlanV1;
        let artifact =
            compile_analysis_recipe_v4(&recipe, Some(&model), target, target.capability_cell())
                .unwrap();
        (dataset, recipe, model, artifact)
    }

    fn construct(id: &str, indicators: &[&str]) -> Construct {
        Construct {
            id: id.into(),
            name: id.to_uppercase(),
            short_name: id.to_uppercase(),
            mode: MeasurementMode::Reflective,
            indicators: indicators.iter().map(|value| (*value).into()).collect(),
        }
    }

    fn path(source: &str, target: &str) -> StructuralPath {
        StructuralPath {
            source: source.into(),
            target: target.into(),
        }
    }

    fn assert_transformation_statistics_equivalent(
        left: &CbsemProductIndicatorTransformationProvenanceV1,
        right: &CbsemProductIndicatorTransformationProvenanceV1,
    ) {
        assert_eq!(
            left.constituent_statistics.len(),
            right.constituent_statistics.len()
        );
        for (left, right) in left
            .constituent_statistics
            .iter()
            .zip(&right.constituent_statistics)
        {
            assert_eq!(left.observed_variable_id, right.observed_variable_id);
            assert_eq!(left.source_column, right.source_column);
            assert_abs_diff_eq!(left.mean, right.mean, epsilon = 1e-14);
            assert_abs_diff_eq!(
                left.sample_standard_deviation,
                right.sample_standard_deviation,
                epsilon = 1e-14
            );
        }
        assert_eq!(
            left.product_statistics.len(),
            right.product_statistics.len()
        );
        for (left, right) in left
            .product_statistics
            .iter()
            .zip(&right.product_statistics)
        {
            assert_eq!(left.observed_variable_id, right.observed_variable_id);
            assert_eq!(left.source_column, right.source_column);
            assert_abs_diff_eq!(
                left.mean_before_second_centering,
                right.mean_before_second_centering,
                epsilon = 1e-14
            );
            assert_abs_diff_eq!(left.final_mean, right.final_mean, epsilon = 1e-14);
            assert_abs_diff_eq!(
                left.final_sample_standard_deviation,
                right.final_sample_standard_deviation,
                epsilon = 1e-14
            );
        }
    }

    fn minimum_sample_covariance_cholesky_pivot(dataset: &Dataset) -> f64 {
        let columns = dataset
            .batch
            .columns()
            .iter()
            .map(|column| {
                column
                    .as_any()
                    .downcast_ref::<Float64Array>()
                    .expect("fixture columns are Float64")
            })
            .collect::<Vec<_>>();
        let observations = dataset.batch.num_rows();
        let means = columns
            .iter()
            .map(|column| {
                (0..observations).map(|row| column.value(row)).sum::<f64>() / observations as f64
            })
            .collect::<Vec<_>>();
        let dimension = columns.len();
        let mut covariance = vec![vec![0.0; dimension]; dimension];
        for left in 0..dimension {
            for right in 0..=left {
                let value = (0..observations)
                    .map(|row| {
                        (columns[left].value(row) - means[left])
                            * (columns[right].value(row) - means[right])
                    })
                    .sum::<f64>()
                    / (observations - 1) as f64;
                covariance[left][right] = value;
                covariance[right][left] = value;
            }
        }
        let mut cholesky = vec![vec![0.0; dimension]; dimension];
        let mut minimum_pivot = f64::INFINITY;
        for row in 0..dimension {
            for column in 0..=row {
                let adjustment = (0..column)
                    .map(|index| cholesky[row][index] * cholesky[column][index])
                    .sum::<f64>();
                if row == column {
                    let pivot = covariance[row][row] - adjustment;
                    assert!(
                        pivot.is_finite() && pivot > 0.0,
                        "fixture sample covariance must be strictly positive definite at pivot {row}; found {pivot}"
                    );
                    minimum_pivot = minimum_pivot.min(pivot);
                    cholesky[row][column] = pivot.sqrt();
                } else {
                    cholesky[row][column] =
                        (covariance[row][column] - adjustment) / cholesky[column][column];
                }
            }
        }
        minimum_pivot
    }

    #[test]
    fn double_mean_centering_matches_the_hand_formula_and_is_row_order_invariant() {
        let (dataset, _, _, artifact) = fixture(false, false);
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 { plan } = artifact.plan() else {
            unreachable!()
        };
        let prepared = prepare_product_dataset(&dataset, plan, &|| false, &|_| {}).unwrap();
        let product = &plan.product_indicators()[0];
        let x_position = dataset
            .schema
            .columns
            .iter()
            .position(|column| column.name == product.predictor_source_column())
            .unwrap();
        let m_position = dataset
            .schema
            .columns
            .iter()
            .position(|column| column.name == product.moderator_source_column())
            .unwrap();
        let output_position = prepared
            .dataset
            .schema
            .columns
            .iter()
            .position(|column| column.name == product.source_column())
            .unwrap();
        let x = dataset.batch.column(x_position);
        let m = dataset.batch.column(m_position);
        let output = prepared
            .dataset
            .batch
            .column(output_position)
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        let xs = x.as_any().downcast_ref::<Float64Array>().unwrap();
        let ms = m.as_any().downcast_ref::<Float64Array>().unwrap();
        let mean_x = (0..xs.len()).map(|row| xs.value(row)).sum::<f64>() / xs.len() as f64;
        let mean_m = (0..ms.len()).map(|row| ms.value(row)).sum::<f64>() / ms.len() as f64;
        let raw_products = (0..xs.len())
            .map(|row| (xs.value(row) - mean_x) * (ms.value(row) - mean_m))
            .collect::<Vec<_>>();
        let product_mean = stable_mean(&raw_products);
        for row in 0..output.len() {
            assert_abs_diff_eq!(
                output.value(row),
                raw_products[row] - product_mean,
                epsilon = 1e-12
            );
        }
        assert_abs_diff_eq!(
            (0..output.len()).map(|row| output.value(row)).sum::<f64>() / output.len() as f64,
            0.0,
            epsilon = 1e-14
        );

        let (reversed, _, _, reversed_artifact) = fixture(true, false);
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 {
            plan: reversed_plan,
        } = reversed_artifact.plan()
        else {
            unreachable!()
        };
        let reversed_prepared =
            prepare_product_dataset(&reversed, reversed_plan, &|| false, &|_| {}).unwrap();
        assert_eq!(
            prepared.provenance.product_statistics.len(),
            plan.product_indicators().len()
        );
        assert_transformation_statistics_equivalent(
            &prepared.provenance,
            &reversed_prepared.provenance,
        );

        // Frozen independently by validation/cbsem_product_indicator_moderation_v1_oracle.py.
        let oracle_rows = [
            [
                4.328537228866939,
                0.3601707413218702,
                3.698191329089379,
                0.34538684642123807,
            ],
            [
                -0.4203458152679607,
                -0.5229562769078702,
                -0.3659101416675321,
                -0.3356598200864455,
            ],
            [
                -0.012103656676097738,
                -1.0695109921684964,
                -0.06269886258600131,
                -1.2003028726861573,
            ],
            [
                -0.15377267976583633,
                -0.02981382526738996,
                -0.12800536547540892,
                -0.19743142325844182,
            ],
        ];
        for (product_index, product) in plan.product_indicators().iter().enumerate() {
            let position = prepared
                .dataset
                .schema
                .columns
                .iter()
                .position(|column| column.name == product.source_column())
                .unwrap();
            let values = prepared.dataset.batch.column(position);
            let values = values.as_any().downcast_ref::<Float64Array>().unwrap();
            for (row, expected) in oracle_rows.iter().enumerate() {
                assert_abs_diff_eq!(values.value(row), expected[product_index], epsilon = 2e-14);
            }
        }
    }

    #[test]
    fn internal_path_uses_exact_parameter_table_and_preserves_stable_interaction_identity() {
        let (dataset, recipe, model, artifact) = fixture(false, false);
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 { plan: product_plan } =
            artifact.plan()
        else {
            unreachable!()
        };
        let prepared = prepare_product_dataset(&dataset, product_plan, &|| false, &|_| {}).unwrap();
        assert!(
            minimum_sample_covariance_cholesky_pivot(&prepared.dataset) > 1e-8,
            "the independent deterministic fixture must remain numerically admissible"
        );
        let result = estimate_internal_cbsem_product_indicator_moderation_v1(
            &dataset, &artifact, &recipe, &model,
        )
        .unwrap();
        assert!(result.estimation.analysis.converged);
        assert_eq!(
            result.provenance.transformation.used_observations,
            dataset.batch.num_rows()
        );
        assert_eq!(result.compiled_product_plan.product_indicators().len(), 4);
        assert!(
            result
                .estimation
                .parameter_ids
                .values()
                .any(|id| id == "parameter:interaction_effect")
        );
        assert_eq!(
            result.provenance.source_compilation_receipt,
            *artifact.receipt()
        );
        assert_ne!(
            result.provenance.source_compilation_receipt.plan_sha256(),
            result.provenance.expanded_compilation_receipt.plan_sha256()
        );
        assert!(result.output_exclusions.iter().any(|exclusion| {
            exclusion.code == "normal_theory_inference_unqualified"
                && exclusion
                    .nested_result_paths
                    .iter()
                    .any(|path| path.ends_with("p_value_two_sided"))
        }));
        assert!(result.output_exclusions.iter().any(|exclusion| {
            exclusion.code == "interaction_aware_global_fit_unqualified"
                && exclusion.nested_result_paths == vec!["estimation.analysis.fit".to_string()]
        }));
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn standardization_accepts_valid_small_scale_positive_rescaling() {
        let (base, _, _, base_artifact) = fixture_with_options(
            false,
            false,
            1.0,
            1.0,
            ProductIndicatorStandardizationV4::SampleStandardDeviation,
        );
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 { plan: base_plan } =
            base_artifact.plan()
        else {
            unreachable!()
        };
        let base_prepared = prepare_product_dataset(&base, base_plan, &|| false, &|_| {}).unwrap();

        let (tiny, _, _, tiny_artifact) = fixture_with_options(
            false,
            false,
            1e-20,
            1e-20,
            ProductIndicatorStandardizationV4::SampleStandardDeviation,
        );
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 { plan: tiny_plan } =
            tiny_artifact.plan()
        else {
            unreachable!()
        };
        let tiny_prepared = prepare_product_dataset(&tiny, tiny_plan, &|| false, &|_| {}).unwrap();
        assert!(
            tiny_prepared
                .provenance
                .constituent_statistics
                .iter()
                .any(|statistic| statistic.sample_standard_deviation > 0.0
                    && statistic.sample_standard_deviation < f64::EPSILON)
        );

        for (base_product, tiny_product) in base_plan
            .product_indicators()
            .iter()
            .zip(tiny_plan.product_indicators())
        {
            assert_eq!(base_product.source_column(), tiny_product.source_column());
            let base_position = base_prepared
                .dataset
                .schema
                .columns
                .iter()
                .position(|column| column.name == base_product.source_column())
                .unwrap();
            let tiny_position = tiny_prepared
                .dataset
                .schema
                .columns
                .iter()
                .position(|column| column.name == tiny_product.source_column())
                .unwrap();
            let base_values = base_prepared.dataset.batch.column(base_position);
            let tiny_values = tiny_prepared.dataset.batch.column(tiny_position);
            let base_values = base_values.as_any().downcast_ref::<Float64Array>().unwrap();
            let tiny_values = tiny_values.as_any().downcast_ref::<Float64Array>().unwrap();
            for row in 0..base_values.len() {
                assert_abs_diff_eq!(
                    base_values.value(row),
                    tiny_values.value(row),
                    epsilon = 5e-13
                );
            }
        }
    }

    #[test]
    fn cancellation_zero_variance_and_product_overflow_have_distinct_typed_errors() {
        let (dataset, recipe, model, artifact) = fixture(false, false);
        assert!(matches!(
            estimate_internal_cbsem_product_indicator_moderation_v1_with_control(
                &dataset,
                &artifact,
                &recipe,
                &model,
                || true,
                |_| {},
            ),
            Err(CbsemProductIndicatorExecutionErrorV1::Cancelled)
        ));

        let (constant, recipe, model, artifact) = fixture(false, true);
        assert!(matches!(
            estimate_internal_cbsem_product_indicator_moderation_v1(
                &constant, &artifact, &recipe, &model,
            ),
            Err(CbsemProductIndicatorExecutionErrorV1::ZeroVariance(column)) if column == "x2"
        ));

        let (overflow, _, _, overflow_artifact) = fixture_with_options(
            false,
            false,
            1e200,
            1e200,
            ProductIndicatorStandardizationV4::None,
        );
        let CompiledRecipePlanV4::CbsemProductIndicatorPlanV1 {
            plan: overflow_plan,
        } = overflow_artifact.plan()
        else {
            unreachable!()
        };
        assert!(matches!(
            prepare_product_dataset(&overflow, overflow_plan, &|| false, &|_| {}),
            Err(
                CbsemProductIndicatorExecutionErrorV1::NonFiniteTransformation {
                    stage: "product_multiplication",
                    row: Some(_),
                    ..
                }
            )
        ));
    }

    #[test]
    fn cancellation_is_observed_during_row_materialization() {
        let (dataset, recipe, model, artifact) = fixture(false, false);
        let cancel = AtomicBool::new(false);
        let saw_row_progress = AtomicBool::new(false);
        let result = estimate_internal_cbsem_product_indicator_moderation_v1_with_control(
            &dataset,
            &artifact,
            &recipe,
            &model,
            || cancel.load(Ordering::SeqCst),
            |update| {
                if update.phase == "listwise_rows" && update.completed_units >= 128 {
                    saw_row_progress.store(true, Ordering::SeqCst);
                    cancel.store(true, Ordering::SeqCst);
                }
            },
        );
        assert!(saw_row_progress.load(Ordering::SeqCst));
        assert!(matches!(
            result,
            Err(CbsemProductIndicatorExecutionErrorV1::Cancelled)
        ));
    }

    #[test]
    fn materialization_envelope_is_checked_before_allocation() {
        let inside = ensure_product_materialization_envelope_v1(9_999_999, 1).unwrap();
        assert_eq!(inside.materialized_product_cells, 9_999_999);

        let boundary = ensure_product_materialization_envelope_v1(2_500_000, 4).unwrap();
        assert_eq!(
            boundary.materialized_product_cells,
            CBSEM_PRODUCT_INDICATOR_MAX_MATERIALIZED_PRODUCT_CELLS_V1
        );
        assert_eq!(boundary.estimated_raw_bytes, 80_000_000);
        assert_eq!(boundary.estimated_peak_bytes, 240_000_000);

        assert!(matches!(
            ensure_product_materialization_envelope_v1(2_500_001, 4),
            Err(
                CbsemProductIndicatorExecutionErrorV1::MaterializationLimitExceeded {
                    complete_rows: 2_500_001,
                    product_count: 4,
                    materialized_product_cells: 10_000_004,
                    estimated_raw_bytes: 80_000_032,
                    estimated_peak_bytes: 240_000_096,
                    max_materialized_product_cells:
                        CBSEM_PRODUCT_INDICATOR_MAX_MATERIALIZED_PRODUCT_CELLS_V1,
                    max_peak_bytes: CBSEM_PRODUCT_INDICATOR_PEAK_WORK_MEMORY_CEILING_BYTES_V1,
                }
            )
        ));

        assert!(matches!(
            ensure_product_materialization_envelope_v1(usize::MAX, usize::MAX),
            Err(
                CbsemProductIndicatorExecutionErrorV1::MaterializationSizeOverflow {
                    complete_rows: usize::MAX,
                    product_count: usize::MAX,
                }
            )
        ));
    }
}
