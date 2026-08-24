use arrow::{
    array::{Array, Float64Array, Int64Array},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use qpls_data::Dataset;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const MULTIMOD_WEIGHT_COLUMN_PREPARATION_METHOD_VERSION_V1: &str =
    "multimod_weight_column_preparation_v1";
pub const MULTIMOD_MAX_FREQUENCY_TOTAL_V1: u64 = (1_u64 << 53) - 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MultimodWeightSemanticsV1 {
    PositiveCase,
    PositiveIntegerFrequencyCountSpace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MultimodWeightPreparationReceiptV1 {
    pub method_version: String,
    pub semantics: MultimodWeightSemanticsV1,
    pub column: String,
    pub row_count: usize,
    pub original_sum: f64,
    pub normalized_sum: f64,
    pub kish_effective_sample_size: f64,
    pub normalized_maximum_to_minimum_ratio: f64,
    pub frequency_total: Option<u64>,
    pub exact_integer_count_space: bool,
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum MultimodWeightPreparationErrorV1 {
    #[error("weight column {0} is absent")]
    MissingColumn(String),
    #[error("weight column {0} is not numeric")]
    NonNumericColumn(String),
    #[error("weight column {column} has a missing, nonfinite, or nonpositive value at row {row}")]
    InvalidCaseWeight { column: String, row: usize },
    #[error("case-weight maximum/minimum ratio exceeds 1e6")]
    CaseWeightRatio,
    #[error("frequency column {column} is not a positive exact integer at row {row}")]
    InvalidFrequency { column: String, row: usize },
    #[error("frequency total exceeds 2^53-1")]
    FrequencyTotal,
    #[error("replacement weight column could not be encoded: {0}")]
    Arrow(String),
}

fn numeric_column(
    dataset: &Dataset,
    column: &str,
) -> Result<(usize, Vec<f64>), MultimodWeightPreparationErrorV1> {
    let position = dataset
        .batch
        .schema()
        .fields()
        .iter()
        .position(|field| field.name() == column)
        .ok_or_else(|| MultimodWeightPreparationErrorV1::MissingColumn(column.into()))?;
    let array = dataset.batch.column(position);
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Ok((
            position,
            (0..values.len())
                .map(|row| {
                    (!values.is_null(row))
                        .then(|| values.value(row))
                        .ok_or_else(|| MultimodWeightPreparationErrorV1::InvalidCaseWeight {
                            column: column.into(),
                            row,
                        })
                })
                .collect::<Result<Vec<_>, _>>()?,
        ));
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Ok((
            position,
            (0..values.len())
                .map(|row| {
                    (!values.is_null(row))
                        .then(|| values.value(row) as f64)
                        .ok_or_else(|| MultimodWeightPreparationErrorV1::InvalidCaseWeight {
                            column: column.into(),
                            row,
                        })
                })
                .collect::<Result<Vec<_>, _>>()?,
        ));
    }
    Err(MultimodWeightPreparationErrorV1::NonNumericColumn(
        column.into(),
    ))
}

fn replace_numeric_column(
    dataset: &Dataset,
    position: usize,
    values: Vec<f64>,
) -> Result<Dataset, MultimodWeightPreparationErrorV1> {
    let mut fields = dataset
        .batch
        .schema()
        .fields()
        .iter()
        .map(|field| field.as_ref().clone())
        .collect::<Vec<_>>();
    let source = &fields[position];
    fields[position] = Field::new(source.name(), DataType::Float64, false)
        .with_metadata(source.metadata().clone());
    let mut columns = dataset.batch.columns().to_vec();
    columns[position] = Arc::new(Float64Array::from(values));
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), columns)
        .map_err(|error| MultimodWeightPreparationErrorV1::Arrow(error.to_string()))?;
    Ok(Dataset {
        id: dataset.id,
        name: dataset.name.clone(),
        schema: dataset.schema.clone(),
        batch,
        // This is a deterministic execution projection of the same admitted
        // rows, not a new scientific dataset. Keeping the source identity is
        // required by the already-bound compiled Recipe V4 artifact.
        fingerprint: dataset.fingerprint.clone(),
    })
}

fn normalized_receipt(
    dataset: &Dataset,
    column: &str,
    semantics: MultimodWeightSemanticsV1,
    frequency_total: Option<u64>,
) -> Result<(Dataset, MultimodWeightPreparationReceiptV1), MultimodWeightPreparationErrorV1> {
    let (position, weights) = numeric_column(dataset, column)?;
    for (row, weight) in weights.iter().enumerate() {
        if !weight.is_finite() || *weight <= 0.0 {
            return Err(MultimodWeightPreparationErrorV1::InvalidCaseWeight {
                column: column.into(),
                row,
            });
        }
    }
    let original_sum = weights.iter().sum::<f64>();
    let mean = original_sum / weights.len() as f64;
    let normalized = weights
        .iter()
        .map(|weight| weight / mean)
        .collect::<Vec<_>>();
    let minimum = normalized.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = normalized.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let ratio = maximum / minimum;
    if !ratio.is_finite() || (semantics == MultimodWeightSemanticsV1::PositiveCase && ratio > 1.0e6)
    {
        return Err(MultimodWeightPreparationErrorV1::CaseWeightRatio);
    }
    let sum_squared = normalized.iter().map(|weight| weight * weight).sum::<f64>();
    let normalized_sum = normalized.iter().sum::<f64>();
    let receipt = MultimodWeightPreparationReceiptV1 {
        method_version: MULTIMOD_WEIGHT_COLUMN_PREPARATION_METHOD_VERSION_V1.into(),
        semantics,
        column: column.into(),
        row_count: normalized.len(),
        original_sum,
        normalized_sum,
        kish_effective_sample_size: normalized_sum * normalized_sum / sum_squared,
        normalized_maximum_to_minimum_ratio: ratio,
        frequency_total,
        exact_integer_count_space: semantics
            == MultimodWeightSemanticsV1::PositiveIntegerFrequencyCountSpace,
    };
    Ok((
        replace_numeric_column(dataset, position, normalized)?,
        receipt,
    ))
}

/// Normalizes strictly positive finite case weights to mean one for one PLS
/// fit. Row sampling must occur before this function so weights travel with
/// their rows and duplicate bootstrap rows retain their original weights.
pub fn prepare_multimod_case_weight_dataset_v1(
    dataset: &Dataset,
    column: &str,
) -> Result<(Dataset, MultimodWeightPreparationReceiptV1), MultimodWeightPreparationErrorV1> {
    normalized_receipt(
        dataset,
        column,
        MultimodWeightSemanticsV1::PositiveCase,
        None,
    )
}

/// Reads finite positive case weights for an already-admitted stable row
/// universe. The returned vector preserves the supplied row order and is not
/// normalized; each downstream fit normalizes its own selected sample.
pub fn multimod_case_weights_for_source_rows_v1(
    dataset: &Dataset,
    column: &str,
    source_rows: &[u64],
) -> Result<Vec<f64>, MultimodWeightPreparationErrorV1> {
    let (_, values) = numeric_column(dataset, column)?;
    let mut weights = Vec::with_capacity(source_rows.len());
    for source_row in source_rows {
        let row = usize::try_from(*source_row).map_err(|_| {
            MultimodWeightPreparationErrorV1::Arrow(
                "case-weight source-row token exceeds the platform row range".into(),
            )
        })?;
        let weight = values.get(row).copied().ok_or_else(|| {
            MultimodWeightPreparationErrorV1::Arrow(
                "case-weight source-row token lies outside the dataset".into(),
            )
        })?;
        if !weight.is_finite() || weight <= 0.0 {
            return Err(MultimodWeightPreparationErrorV1::InvalidCaseWeight {
                column: column.into(),
                row,
            });
        }
        weights.push(weight);
    }
    let minimum = weights.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = weights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !minimum.is_finite() || !maximum.is_finite() || maximum / minimum > 1.0e6 {
        return Err(MultimodWeightPreparationErrorV1::CaseWeightRatio);
    }
    Ok(weights)
}

/// Converts positive exact integer frequencies into the normalized weight
/// representation consumed by the production weighted standardized-PLS
/// engine. For standardized PLS paths, loadings, outer weights, and R-squared,
/// this count-space representation is algebraically identical to duplicating
/// rows, while retaining only one physical row per distinct record.
pub fn prepare_multimod_frequency_weight_dataset_v1(
    dataset: &Dataset,
    column: &str,
) -> Result<(Dataset, MultimodWeightPreparationReceiptV1), MultimodWeightPreparationErrorV1> {
    let counts = multimod_frequency_counts_v1(dataset, column)?;
    prepare_multimod_frequency_count_dataset_v1(dataset, column, &counts)
}

/// Reads the authoritative positive integer frequency vector without
/// converting it to floating-point execution weights.
pub fn multimod_frequency_counts_v1(
    dataset: &Dataset,
    column: &str,
) -> Result<Vec<u64>, MultimodWeightPreparationErrorV1> {
    let (_, values) = numeric_column(dataset, column)?;
    let mut total = 0_u64;
    let mut counts = Vec::with_capacity(values.len());
    for (row, value) in values.iter().enumerate() {
        if !value.is_finite()
            || *value < 1.0
            || value.fract() != 0.0
            || *value > MULTIMOD_MAX_FREQUENCY_TOTAL_V1 as f64
        {
            return Err(MultimodWeightPreparationErrorV1::InvalidFrequency {
                column: column.into(),
                row,
            });
        }
        total = total
            .checked_add(*value as u64)
            .filter(|total| *total <= MULTIMOD_MAX_FREQUENCY_TOTAL_V1)
            .ok_or(MultimodWeightPreparationErrorV1::FrequencyTotal)?;
        counts.push(*value as u64);
    }
    Ok(counts)
}

/// Reads exact positive integer frequencies only for the already-admitted
/// stable source-row universe. Invalid values outside that universe remain an
/// upstream exclusion concern and cannot contaminate the scientific fit.
pub fn multimod_frequency_counts_for_source_rows_v1(
    dataset: &Dataset,
    column: &str,
    source_rows: &[u64],
) -> Result<Vec<u64>, MultimodWeightPreparationErrorV1> {
    let (_, values) = numeric_column(dataset, column)?;
    let mut total = 0_u64;
    let mut counts = Vec::with_capacity(source_rows.len());
    for source_row in source_rows {
        let row = usize::try_from(*source_row).map_err(|_| {
            MultimodWeightPreparationErrorV1::Arrow(
                "frequency source-row token exceeds the platform row range".into(),
            )
        })?;
        let value = values.get(row).copied().ok_or_else(|| {
            MultimodWeightPreparationErrorV1::Arrow(
                "frequency source-row token lies outside the dataset".into(),
            )
        })?;
        if !value.is_finite()
            || value < 1.0
            || value.fract() != 0.0
            || value > MULTIMOD_MAX_FREQUENCY_TOTAL_V1 as f64
        {
            return Err(MultimodWeightPreparationErrorV1::InvalidFrequency {
                column: column.into(),
                row,
            });
        }
        let count = value as u64;
        total = total
            .checked_add(count)
            .filter(|total| *total <= MULTIMOD_MAX_FREQUENCY_TOTAL_V1)
            .ok_or(MultimodWeightPreparationErrorV1::FrequencyTotal)?;
        counts.push(count);
    }
    Ok(counts)
}

/// Installs one already-generated positive count-space sample into the
/// production WPLS input. Callers must omit zero-count rows before invoking
/// this function; no physical row expansion occurs.
pub fn prepare_multimod_frequency_count_dataset_v1(
    dataset: &Dataset,
    column: &str,
    counts: &[u64],
) -> Result<(Dataset, MultimodWeightPreparationReceiptV1), MultimodWeightPreparationErrorV1> {
    if counts.len() != dataset.batch.num_rows() {
        return Err(MultimodWeightPreparationErrorV1::Arrow(
            "frequency count vector length differs from the dataset row count".into(),
        ));
    }
    let mut total = 0_u64;
    for (row, count) in counts.iter().enumerate() {
        if *count == 0 {
            return Err(MultimodWeightPreparationErrorV1::InvalidFrequency {
                column: column.into(),
                row,
            });
        }
        total = total
            .checked_add(*count)
            .filter(|total| *total <= MULTIMOD_MAX_FREQUENCY_TOTAL_V1)
            .ok_or(MultimodWeightPreparationErrorV1::FrequencyTotal)?;
    }
    let (position, _) = numeric_column(dataset, column)?;
    let counted = replace_numeric_column(
        dataset,
        position,
        counts.iter().map(|count| *count as f64).collect(),
    )?;
    normalized_receipt(
        &counted,
        column,
        MultimodWeightSemanticsV1::PositiveIntegerFrequencyCountSpace,
        Some(total),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_limit_is_exact() {
        assert_eq!(MULTIMOD_MAX_FREQUENCY_TOTAL_V1, 9_007_199_254_740_991);
    }
}
