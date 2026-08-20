use crate::{EstimationError, PlsResult, pls::subset_array};
use arrow::{
    array::{Array, ArrayRef, Float64Array, Int64Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use qpls_core::{CompiledPlsPlanV2, CompiledPlsPlanV3, HigherOrderConstructionApproachV4};
use qpls_data::{ColumnMetadata, ColumnType, Dataset, ScaleType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, sync::Arc};

pub const GENERAL_SEM_PLS_DISJOINT_HOC_SCORE_DATASET_RECEIPT_VERSION_V1: &str =
    "general_sem_pls_disjoint_hoc_score_dataset_receipt_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsHocGeneratedScoreColumnReceiptV1 {
    component_id: String,
    generated_score_variable_id: String,
    observation_count: usize,
    values_sha256: String,
}

impl GeneralSemPlsHocGeneratedScoreColumnReceiptV1 {
    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn generated_score_variable_id(&self) -> &str {
        &self.generated_score_variable_id
    }

    pub fn observation_count(&self) -> usize {
        self.observation_count
    }

    pub fn values_sha256(&self) -> &str {
        &self.values_sha256
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneralSemPlsDisjointHocScoreDatasetReceiptV1 {
    receipt_version: String,
    source_dataset_fingerprint: String,
    complete_case_row_count: usize,
    omitted_row_count: usize,
    complete_case_rows_sha256: String,
    generated_score_columns: Vec<GeneralSemPlsHocGeneratedScoreColumnReceiptV1>,
}

impl GeneralSemPlsDisjointHocScoreDatasetReceiptV1 {
    pub fn receipt_version(&self) -> &str {
        &self.receipt_version
    }

    pub fn source_dataset_fingerprint(&self) -> &str {
        &self.source_dataset_fingerprint
    }

    pub fn complete_case_row_count(&self) -> usize {
        self.complete_case_row_count
    }

    pub fn omitted_row_count(&self) -> usize {
        self.omitted_row_count
    }

    pub fn complete_case_rows_sha256(&self) -> &str {
        &self.complete_case_rows_sha256
    }

    pub fn generated_score_columns(&self) -> &[GeneralSemPlsHocGeneratedScoreColumnReceiptV1] {
        &self.generated_score_columns
    }
}

#[derive(Debug, Clone)]
pub struct PreparedGeneralSemPlsDisjointHocScoreDatasetV1 {
    dataset: Dataset,
    receipt: GeneralSemPlsDisjointHocScoreDatasetReceiptV1,
}

impl PreparedGeneralSemPlsDisjointHocScoreDatasetV1 {
    pub fn dataset(&self) -> &Dataset {
        &self.dataset
    }

    pub fn receipt(&self) -> &GeneralSemPlsDisjointHocScoreDatasetReceiptV1 {
        &self.receipt
    }

    pub fn into_parts(self) -> (Dataset, GeneralSemPlsDisjointHocScoreDatasetReceiptV1) {
        (self.dataset, self.receipt)
    }
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum GeneralSemPlsDisjointHocScoreDatasetErrorV1 {
    #[error("higher-order point-stage preparation was cancelled")]
    Cancelled,
    #[error("compiled PLS v3 plan must contain exactly one higher-order stage plan")]
    HigherOrderPlanCardinality,
    #[error("higher-order point-stage preparation currently requires disjoint_two_stage")]
    DisjointTwoStageRequired,
    #[error(
        "higher-order score-stage preparation requires embedded_two_stage or disjoint_two_stage"
    )]
    ScoreStageRequired,
    #[error("stage-one dataset is missing numeric source column {source_column}")]
    MissingOrNonNumericSourceColumn { source_column: String },
    #[error("fewer than three complete observations remain for higher-order estimation")]
    InsufficientObservations,
    #[error(
        "stage-one result reports {reported} used observations, but the compiled plan resolves {resolved} complete rows"
    )]
    CompleteCaseReceiptMismatch { reported: usize, resolved: usize },
    #[error("stage-one score is missing for lower-order component {component_id}")]
    MissingComponentScore { component_id: String },
    #[error(
        "stage-one score for lower-order component {component_id} has {actual} observations; expected {expected}"
    )]
    ComponentScoreLengthMismatch {
        component_id: String,
        expected: usize,
        actual: usize,
    },
    #[error(
        "stage-one score for lower-order component {component_id} is non-finite at row {row_index}"
    )]
    NonFiniteComponentScore {
        component_id: String,
        row_index: usize,
    },
    #[error("generated score column collides with existing dataset column {column_id}")]
    GeneratedColumnCollision { column_id: String },
    #[error("unsupported Arrow column cannot be subset for higher-order stage two")]
    UnsupportedArrowColumn,
    #[error("higher-order score dataset could not be constructed: {0}")]
    Arrow(String),
}

#[derive(Debug, Clone)]
pub(crate) struct PlsGeneratedScoreColumnSpecV1 {
    pub source_score_id: String,
    pub generated_column_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlsAliasColumnSpecV1 {
    pub source_column_id: String,
    pub generated_column_id: String,
    pub label: String,
}

/// Adds deterministic virtual aliases without changing source values or the
/// resident dataset fingerprint. Repeated-indicator HOCs use this to satisfy
/// the ordinary PLS engine's one-source-column-per-indicator invariant.
pub fn append_pls_alias_columns_v1(
    dataset: &Dataset,
    specs: &[PlsAliasColumnSpecV1],
) -> Result<Dataset, GeneralSemPlsDisjointHocScoreDatasetErrorV1> {
    let mut arrays = dataset.batch.columns().to_vec();
    let batch_schema = dataset.batch.schema();
    let mut fields = batch_schema
        .fields()
        .iter()
        .map(|field| Field::new(field.name(), field.data_type().clone(), field.is_nullable()))
        .collect::<Vec<_>>();
    let mut schema = dataset.schema.clone();
    let mut occupied = fields
        .iter()
        .map(|field| field.name().to_string())
        .collect::<BTreeSet<_>>();
    for spec in specs {
        if !occupied.insert(spec.generated_column_id.clone()) {
            return Err(
                GeneralSemPlsDisjointHocScoreDatasetErrorV1::GeneratedColumnCollision {
                    column_id: spec.generated_column_id.clone(),
                },
            );
        }
        let source_index = batch_schema.index_of(&spec.source_column_id).map_err(|_| {
            GeneralSemPlsDisjointHocScoreDatasetErrorV1::MissingOrNonNumericSourceColumn {
                source_column: spec.source_column_id.clone(),
            }
        })?;
        let source = dataset.batch.column(source_index);
        let source_field = batch_schema.field(source_index);
        if source.as_any().downcast_ref::<Float64Array>().is_none()
            && source.as_any().downcast_ref::<Int64Array>().is_none()
        {
            return Err(
                GeneralSemPlsDisjointHocScoreDatasetErrorV1::MissingOrNonNumericSourceColumn {
                    source_column: spec.source_column_id.clone(),
                },
            );
        }
        arrays.push(source.clone());
        fields.push(Field::new(
            &spec.generated_column_id,
            source.data_type().clone(),
            source_field.is_nullable(),
        ));
        schema.columns.push(ColumnMetadata {
            name: spec.generated_column_id.clone(),
            label: Some(spec.label.clone()),
            column_type: ColumnType::Numeric,
            scale_type: ScaleType::Continuous,
            missing_markers: Vec::new(),
            theoretical_min: None,
            theoretical_max: None,
            value_labels: Default::default(),
        });
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|error| GeneralSemPlsDisjointHocScoreDatasetErrorV1::Arrow(error.to_string()))?;
    Ok(Dataset {
        id: dataset.id,
        name: dataset.name.clone(),
        schema,
        batch,
        fingerprint: dataset.fingerprint.clone(),
    })
}

pub(crate) fn append_pls_generated_score_columns_v1(
    dataset: &Dataset,
    scores: &std::collections::BTreeMap<String, Vec<f64>>,
    used_rows: &[usize],
    specs: &[PlsGeneratedScoreColumnSpecV1],
) -> Result<Dataset, EstimationError> {
    let mut arrays = dataset
        .batch
        .columns()
        .iter()
        .map(|column| subset_array(column.as_ref(), used_rows))
        .collect::<Result<Vec<_>, _>>()?;
    let mut fields = dataset
        .batch
        .schema()
        .fields()
        .iter()
        .map(|field| Field::new(field.name(), field.data_type().clone(), field.is_nullable()))
        .collect::<Vec<_>>();
    let mut schema = dataset.schema.clone();
    let existing_fields = fields
        .iter()
        .map(|field| field.name().to_string())
        .collect::<BTreeSet<_>>();
    let mut generated_names = BTreeSet::new();
    for spec in specs {
        let values = scores.get(&spec.source_score_id).ok_or_else(|| {
            EstimationError::Numerical(format!(
                "missing stage-1 component scores for {}",
                spec.source_score_id
            ))
        })?;
        if values.len() != used_rows.len() {
            return Err(EstimationError::Numerical(
                "stage-1 score length does not match the complete-case rows".into(),
            ));
        }
        if existing_fields.contains(&spec.generated_column_id)
            || !generated_names.insert(spec.generated_column_id.clone())
        {
            return Err(EstimationError::DuplicateIndicator(
                spec.generated_column_id.clone(),
            ));
        }
        arrays.push(Arc::new(Float64Array::from(values.clone())) as ArrayRef);
        fields.push(Field::new(
            &spec.generated_column_id,
            DataType::Float64,
            false,
        ));
        schema.columns.push(ColumnMetadata {
            name: spec.generated_column_id.clone(),
            label: Some(spec.label.clone()),
            column_type: ColumnType::Numeric,
            scale_type: ScaleType::Continuous,
            missing_markers: Vec::new(),
            theoretical_min: None,
            theoretical_max: None,
            value_labels: Default::default(),
        });
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|error| EstimationError::Numerical(error.to_string()))?;
    schema.case_count = batch.num_rows();
    Ok(Dataset {
        id: dataset.id,
        name: dataset.name.clone(),
        schema,
        batch,
        fingerprint: dataset.fingerprint.clone(),
    })
}

/// Prepares the ephemeral stage-two dataset directly from the typed schema-6
/// plan. This is shared by point execution now and by full-model case
/// resampling later; it never creates a legacy HOC `ModelSpec`.
pub fn prepare_general_sem_pls_disjoint_hoc_score_dataset_v1(
    dataset: &Dataset,
    plan: &CompiledPlsPlanV3,
    stage_one: &PlsResult,
    mut should_continue: impl FnMut() -> bool,
) -> Result<
    PreparedGeneralSemPlsDisjointHocScoreDatasetV1,
    GeneralSemPlsDisjointHocScoreDatasetErrorV1,
> {
    let [hoc] = plan.higher_order_stage_plans() else {
        return Err(GeneralSemPlsDisjointHocScoreDatasetErrorV1::HigherOrderPlanCardinality);
    };
    if !matches!(
        hoc.approach(),
        HigherOrderConstructionApproachV4::EmbeddedTwoStage
            | HigherOrderConstructionApproachV4::DisjointTwoStage
    ) {
        return Err(GeneralSemPlsDisjointHocScoreDatasetErrorV1::ScoreStageRequired);
    }
    let used_rows =
        general_sem_pls_hoc_complete_case_rows_v1(dataset, plan.base_plan(), &mut should_continue)?;
    if stage_one.used_observations != used_rows.len()
        || stage_one.omitted_observations
            != dataset.batch.num_rows().saturating_sub(used_rows.len())
    {
        return Err(
            GeneralSemPlsDisjointHocScoreDatasetErrorV1::CompleteCaseReceiptMismatch {
                reported: stage_one.used_observations,
                resolved: used_rows.len(),
            },
        );
    }
    let specs = hoc
        .component_mappings()
        .iter()
        .map(|mapping| {
            let values = stage_one
                .construct_scores
                .get(mapping.component_id())
                .ok_or_else(|| {
                    GeneralSemPlsDisjointHocScoreDatasetErrorV1::MissingComponentScore {
                        component_id: mapping.component_id().to_string(),
                    }
                })?;
            if values.len() != used_rows.len() {
                return Err(
                    GeneralSemPlsDisjointHocScoreDatasetErrorV1::ComponentScoreLengthMismatch {
                        component_id: mapping.component_id().to_string(),
                        expected: used_rows.len(),
                        actual: values.len(),
                    },
                );
            }
            if let Some((row_index, _)) = values
                .iter()
                .enumerate()
                .find(|(_, value)| !value.is_finite())
            {
                return Err(
                    GeneralSemPlsDisjointHocScoreDatasetErrorV1::NonFiniteComponentScore {
                        component_id: mapping.component_id().to_string(),
                        row_index,
                    },
                );
            }
            Ok(PlsGeneratedScoreColumnSpecV1 {
                source_score_id: mapping.component_id().to_string(),
                generated_column_id: mapping.generated_score_variable_id().to_string(),
                label: format!(
                    "HOC component score: {} <- {}",
                    hoc.output_variable_id(),
                    mapping.component_id()
                ),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expanded = append_pls_generated_score_columns_v1(
        dataset,
        &stage_one.construct_scores,
        &used_rows,
        &specs,
    )
    .map_err(map_generated_dataset_error)?;
    let generated_score_columns = hoc
        .component_mappings()
        .iter()
        .map(|mapping| {
            let values = &stage_one.construct_scores[mapping.component_id()];
            GeneralSemPlsHocGeneratedScoreColumnReceiptV1 {
                component_id: mapping.component_id().to_string(),
                generated_score_variable_id: mapping.generated_score_variable_id().to_string(),
                observation_count: values.len(),
                values_sha256: f64_values_sha256(values),
            }
        })
        .collect();
    Ok(PreparedGeneralSemPlsDisjointHocScoreDatasetV1 {
        dataset: expanded,
        receipt: GeneralSemPlsDisjointHocScoreDatasetReceiptV1 {
            receipt_version: GENERAL_SEM_PLS_DISJOINT_HOC_SCORE_DATASET_RECEIPT_VERSION_V1.into(),
            source_dataset_fingerprint: dataset.fingerprint.0.clone(),
            complete_case_row_count: used_rows.len(),
            omitted_row_count: dataset.batch.num_rows().saturating_sub(used_rows.len()),
            complete_case_rows_sha256: row_indices_sha256(&used_rows),
            generated_score_columns,
        },
    })
}

/// Resolves the raw-case frame shared by HOC point preparation and indexed
/// full-model bootstrap. Missing source columns fail closed rather than being
/// silently omitted from the listwise predicate.
pub fn general_sem_pls_hoc_complete_case_rows_v1(
    dataset: &Dataset,
    plan: &CompiledPlsPlanV2,
    mut should_continue: impl FnMut() -> bool,
) -> Result<Vec<usize>, GeneralSemPlsDisjointHocScoreDatasetErrorV1> {
    let source_columns = plan
        .blocks()
        .iter()
        .flat_map(|block| block.indicators())
        .map(|indicator| indicator.source_column())
        .collect::<BTreeSet<_>>();
    let schema = dataset.batch.schema();
    let positions = source_columns
        .iter()
        .map(|source_column| {
            let position = schema.index_of(source_column).map_err(|_| {
                GeneralSemPlsDisjointHocScoreDatasetErrorV1::MissingOrNonNumericSourceColumn {
                    source_column: (*source_column).to_string(),
                }
            })?;
            let array = dataset.batch.column(position);
            if array.as_any().downcast_ref::<Float64Array>().is_none()
                && array.as_any().downcast_ref::<Int64Array>().is_none()
            {
                return Err(
                    GeneralSemPlsDisjointHocScoreDatasetErrorV1::MissingOrNonNumericSourceColumn {
                        source_column: (*source_column).to_string(),
                    },
                );
            }
            Ok(position)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut used_rows = Vec::new();
    for row in 0..dataset.batch.num_rows() {
        if row % 1024 == 0 && !should_continue() {
            return Err(GeneralSemPlsDisjointHocScoreDatasetErrorV1::Cancelled);
        }
        if positions.iter().all(|position| {
            let array = dataset.batch.column(*position);
            !array.is_null(row) && numeric_value(array.as_ref(), row).is_some_and(f64::is_finite)
        }) {
            used_rows.push(row);
        }
    }
    if !should_continue() {
        return Err(GeneralSemPlsDisjointHocScoreDatasetErrorV1::Cancelled);
    }
    if used_rows.len() < 3 {
        return Err(GeneralSemPlsDisjointHocScoreDatasetErrorV1::InsufficientObservations);
    }
    Ok(used_rows)
}

fn map_generated_dataset_error(
    error: EstimationError,
) -> GeneralSemPlsDisjointHocScoreDatasetErrorV1 {
    match error {
        EstimationError::DuplicateIndicator(column_id) => {
            GeneralSemPlsDisjointHocScoreDatasetErrorV1::GeneratedColumnCollision { column_id }
        }
        EstimationError::Numerical(message) if message.contains("unsupported Arrow") => {
            GeneralSemPlsDisjointHocScoreDatasetErrorV1::UnsupportedArrowColumn
        }
        EstimationError::Numerical(message) => {
            GeneralSemPlsDisjointHocScoreDatasetErrorV1::Arrow(message)
        }
        other => GeneralSemPlsDisjointHocScoreDatasetErrorV1::Arrow(other.to_string()),
    }
}

fn numeric_value(array: &dyn Array, row: usize) -> Option<f64> {
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        Some(values.value(row))
    } else {
        array
            .as_any()
            .downcast_ref::<Int64Array>()
            .map(|values| values.value(row) as f64)
    }
}

fn row_indices_sha256(rows: &[usize]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.general-sem-pls-hoc.complete-case-rows.v1\0");
    for row in rows {
        digest.update((*row as u64).to_be_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn f64_values_sha256(values: &[f64]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"qpls.general-sem-pls-hoc.generated-score-values.v1\0");
    for value in values {
        digest.update(value.to_bits().to_be_bytes());
    }
    format!("{:x}", digest.finalize())
}
