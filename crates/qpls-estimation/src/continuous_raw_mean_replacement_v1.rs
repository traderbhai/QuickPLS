use arrow::array::{Array, Float64Array, Int64Array};
use qpls_data::{
    ColumnType, DataKind, Dataset, DatasetDescriptor, ScaleType, dataset_from_descriptor,
    write_arrow,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::io::{self, Write};

pub const MEAN_REPLACEMENT_METHOD_VERSION_V1: &str = "mean_replacement_v1";
pub const MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1: f64 = 0.05;
pub const MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1: f64 = 0.15;
/// Internal execution-safety bounds for the v1 workload. These are not a
/// maximum-axis scientific or performance qualification claim.
pub const MEAN_REPLACEMENT_MAX_ROWS_V1: usize = 100_000;
pub const MEAN_REPLACEMENT_MAX_VARIABLES_V1: usize = 300;
pub const MEAN_REPLACEMENT_MAX_MODELED_CELLS_V1: usize = 10_000_000;
pub const MEAN_REPLACEMENT_MAX_IMPUTED_CELLS_V1: usize = 1_000_000;
const MAX_EXACT_F64_INTEGER_V1: i64 = 1_i64 << 53;
const CANCELLATION_POLL_INTERVAL_V1: usize = 1_024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContinuousRawMeanReplacementVariableBindingV1 {
    pub variable_id: String,
    pub source_column: String,
    #[serde(default)]
    pub missing_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeanReplacementVariableReceiptV1 {
    pub variable_order: usize,
    pub variable_id: String,
    pub source_column: String,
    pub canonical_missing_markers: Vec<String>,
    pub observed_count: usize,
    pub missing_count: usize,
    pub replacement_mean: f64,
    pub missing_fraction: f64,
    pub warning_level: MeanReplacementWarningLevelV1,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeanReplacementPolicyV1 {
    MeanReplacement,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeanReplacementWarningLevelV1 {
    None,
    AtLeastFivePercent,
    AboveFifteenPercent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeanReplacementCaseReceiptV1 {
    pub row_index_zero_based: usize,
    pub imputed_variable_ids: Vec<String>,
    pub missing_fraction: f64,
    pub high_missingness_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MeanReplacementReceiptV1 {
    pub method_version: String,
    pub policy: MeanReplacementPolicyV1,
    pub source_dataset_id: String,
    pub source_dataset_fingerprint: String,
    pub source_row_count: usize,
    pub retained_row_count: usize,
    pub omitted_row_count: usize,
    pub modeled_variable_count: usize,
    pub imputed_cell_count: usize,
    pub affected_case_count: usize,
    pub variable_warning_threshold: f64,
    pub high_missingness_threshold: f64,
    pub variables: Vec<MeanReplacementVariableReceiptV1>,
    pub cases: Vec<MeanReplacementCaseReceiptV1>,
    pub missingness_sha256: String,
    pub completed_matrix_sha256: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedContinuousRawMeanReplacementV1 {
    rows: Vec<Vec<f64>>,
    receipt: MeanReplacementReceiptV1,
}

impl PreparedContinuousRawMeanReplacementV1 {
    pub fn rows(&self) -> &[Vec<f64>] {
        &self.rows
    }

    pub fn receipt(&self) -> &MeanReplacementReceiptV1 {
        &self.receipt
    }

    pub fn into_parts(self) -> (Vec<Vec<f64>>, MeanReplacementReceiptV1) {
        (self.rows, self.receipt)
    }

    pub fn covariance_ml_with_control(
        &self,
        should_cancel: &impl Fn() -> bool,
    ) -> Result<Vec<Vec<f64>>, ContinuousRawMeanReplacementErrorV1> {
        covariance_ml_from_receipt_means(&self.rows, &self.receipt, should_cancel)
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ContinuousRawMeanReplacementErrorV1 {
    #[error("continuous raw mean replacement was cancelled")]
    Cancelled,
    #[error("dataset bytes or schema do not reproduce their declared fingerprint: {0}")]
    DatasetIntegrity(String),
    #[error("continuous raw mean replacement requires a raw dataset")]
    RawDataRequired,
    #[error("raw dataset sample_size metadata must be absent")]
    RawSampleSizeMetadata,
    #[error("dataset schema and Arrow record batch have inconsistent shape")]
    DatasetShape,
    #[error("continuous raw mean replacement requires at least one modeled variable")]
    EmptyVariableSet,
    #[error(
        "continuous raw mean replacement supports at most {maximum} rows in v1 (found {actual})"
    )]
    RowLimitExceeded { actual: usize, maximum: usize },
    #[error(
        "continuous raw mean replacement supports at most {maximum} modeled variables in v1 (found {actual})"
    )]
    VariableLimitExceeded { actual: usize, maximum: usize },
    #[error(
        "continuous raw mean replacement row-by-variable cell count overflowed for {row_count} rows and {variable_count} variables"
    )]
    ModeledCellCountOverflow {
        row_count: usize,
        variable_count: usize,
    },
    #[error(
        "continuous raw mean replacement supports at most {maximum} modeled cells in v1 (found {actual})"
    )]
    ModeledCellLimitExceeded { actual: usize, maximum: usize },
    #[error("continuous raw mean replacement imputed-cell count overflowed")]
    ImputedCellCountOverflow,
    #[error(
        "continuous raw mean replacement supports at most {maximum} imputed cells in v1 (found {actual})"
    )]
    ImputedCellLimitExceeded { actual: usize, maximum: usize },
    #[error(
        "continuous raw mean replacement could not reserve {requested_capacity} entries for {buffer}"
    )]
    AllocationFailed {
        buffer: &'static str,
        requested_capacity: usize,
    },
    #[error("modeled variable or source-column identities are duplicated")]
    DuplicateVariableBinding,
    #[error("modeled source column {0} is unavailable")]
    UnknownSourceColumn(String),
    #[error("modeled source column {0} is not continuous numeric data")]
    UnsupportedSourceColumn(String),
    #[error("missing-marker metadata differs for modeled variable {variable_id}")]
    MissingMarkerMetadataMismatch { variable_id: String },
    #[error(
        "non-null value in {source_column} at row {row_index_zero_based} still equals declared missing marker {marker}"
    )]
    UnresolvedMissingMarker {
        source_column: String,
        row_index_zero_based: usize,
        marker: String,
    },
    #[error("non-finite value in {source_column} at row {row_index_zero_based}")]
    NonFiniteValue {
        source_column: String,
        row_index_zero_based: usize,
    },
    #[error(
        "Int64 value {value} in {source_column} at row {row_index_zero_based} is outside the exactly representable f64 integer interval [-2^53, +2^53]"
    )]
    Int64ValueOutsideExactF64Range {
        source_column: String,
        row_index_zero_based: usize,
        value: i64,
    },
    #[error("modeled variable {0} has no observed finite values")]
    AllMissingVariable(String),
    #[error("replacement mean for modeled variable {0} is not finite")]
    NonFiniteMean(String),
    #[error("modeled variable {0} is constant after mean replacement")]
    ConstantVariable(String),
    #[error("completed variance for modeled variable {0} is not finite")]
    NonFiniteVariance(String),
    #[error(
        "completed cross-product for modeled variables {left_variable_id} and {right_variable_id} is not finite at row {row_index_zero_based}"
    )]
    NonFiniteCrossProduct {
        left_variable_id: String,
        right_variable_id: String,
        row_index_zero_based: usize,
    },
    #[error(
        "completed covariance for modeled variables {left_variable_id} and {right_variable_id} is not finite"
    )]
    NonFiniteCovariance {
        left_variable_id: String,
        right_variable_id: String,
    },
    #[error("mean-replacement receipt hash serialization failed: {0}")]
    ReceiptHashSerialization(String),
}

enum ResolvedNumericColumnV1<'a> {
    Float64(&'a Float64Array),
    Int64(&'a Int64Array),
}

impl ResolvedNumericColumnV1<'_> {
    fn is_null(&self, row: usize) -> bool {
        match self {
            Self::Float64(values) => values.is_null(row),
            Self::Int64(values) => values.is_null(row),
        }
    }

    fn null_count(&self) -> usize {
        match self {
            Self::Float64(values) => values.null_count(),
            Self::Int64(values) => values.null_count(),
        }
    }

    fn exact_value(
        &self,
        row: usize,
        source_column: &str,
    ) -> Result<ExactNumericCellV1, ContinuousRawMeanReplacementErrorV1> {
        match self {
            Self::Float64(values) => {
                let value = values.value(row);
                if !value.is_finite() {
                    return Err(ContinuousRawMeanReplacementErrorV1::NonFiniteValue {
                        source_column: source_column.into(),
                        row_index_zero_based: row,
                    });
                }
                Ok(ExactNumericCellV1::Float64(value))
            }
            Self::Int64(values) => {
                let value = values.value(row);
                if !(-MAX_EXACT_F64_INTEGER_V1..=MAX_EXACT_F64_INTEGER_V1).contains(&value) {
                    return Err(
                        ContinuousRawMeanReplacementErrorV1::Int64ValueOutsideExactF64Range {
                            source_column: source_column.into(),
                            row_index_zero_based: row,
                            value,
                        },
                    );
                }
                Ok(ExactNumericCellV1::Int64(value))
            }
        }
    }
}

enum ExactNumericCellV1 {
    Float64(f64),
    Int64(i64),
}

impl ExactNumericCellV1 {
    fn as_f64(&self) -> f64 {
        match self {
            Self::Float64(value) => *value,
            Self::Int64(value) => *value as f64,
        }
    }
}

enum ResolvedNumericMarkersV1 {
    Float64(Vec<(String, f64)>),
    Int64(Vec<(String, i64)>),
}

impl ResolvedNumericMarkersV1 {
    fn matching_marker<'a>(&'a self, cell: &ExactNumericCellV1) -> Option<&'a str> {
        match (self, cell) {
            (Self::Float64(markers), ExactNumericCellV1::Float64(value)) => markers
                .iter()
                .find(|(_, marker_value)| value == marker_value)
                .map(|(marker, _)| marker.as_str()),
            (Self::Int64(markers), ExactNumericCellV1::Int64(value)) => markers
                .iter()
                .find(|(_, marker_value)| value == marker_value)
                .map(|(marker, _)| marker.as_str()),
            _ => None,
        }
    }
}

struct ResolvedColumnV1<'a> {
    column: ResolvedNumericColumnV1<'a>,
    canonical_missing_markers: Vec<String>,
    numeric_markers: ResolvedNumericMarkersV1,
}

#[derive(Default)]
struct CompensatedSumV1 {
    sum: f64,
    compensation: f64,
}

impl CompensatedSumV1 {
    fn add(&mut self, value: f64) -> bool {
        let next = self.sum + value;
        self.compensation += if self.sum.abs() >= value.abs() {
            (self.sum - next) + value
        } else {
            (value - next) + self.sum
        };
        if !next.is_finite() || !self.compensation.is_finite() {
            return false;
        }
        self.sum = next;
        true
    }

    fn total(&self) -> Option<f64> {
        let result = self.sum + self.compensation;
        result.is_finite().then_some(result)
    }
}

/// Prepares a temporary dense row matrix for the exact continuous-raw,
/// cell-wise mean-replacement contract. The source Dataset is never mutated.
/// A row missing every modeled variable is retained and filled cell by cell;
/// only a variable with no observed value is non-executable.
pub fn prepare_continuous_raw_mean_replacement_v1(
    dataset: &Dataset,
    bindings: &[ContinuousRawMeanReplacementVariableBindingV1],
) -> Result<PreparedContinuousRawMeanReplacementV1, ContinuousRawMeanReplacementErrorV1> {
    prepare_continuous_raw_mean_replacement_v1_with_control(dataset, bindings, || false)
}

pub fn prepare_continuous_raw_mean_replacement_v1_with_control(
    dataset: &Dataset,
    bindings: &[ContinuousRawMeanReplacementVariableBindingV1],
    should_cancel: impl Fn() -> bool,
) -> Result<PreparedContinuousRawMeanReplacementV1, ContinuousRawMeanReplacementErrorV1> {
    cancellation_checkpoint(&should_cancel)?;
    validate_workload_dimensions_v1(dataset.batch.num_rows(), bindings.len())?;
    validate_dataset_integrity(dataset)?;
    cancellation_checkpoint(&should_cancel)?;
    prepare_continuous_raw_mean_replacement_v1_after_integrity_with_control(
        dataset,
        bindings,
        &should_cancel,
    )
}

/// Internal execution seam for callers that have already reproduced the
/// dataset fingerprint from the exact schema and Arrow bytes. Keeping this
/// precondition explicit avoids a second full Arrow integrity round-trip.
pub(crate) fn prepare_continuous_raw_mean_replacement_v1_after_integrity_with_control(
    dataset: &Dataset,
    bindings: &[ContinuousRawMeanReplacementVariableBindingV1],
    should_cancel: &impl Fn() -> bool,
) -> Result<PreparedContinuousRawMeanReplacementV1, ContinuousRawMeanReplacementErrorV1> {
    cancellation_checkpoint(should_cancel)?;
    if dataset.schema.kind != DataKind::Raw {
        return Err(ContinuousRawMeanReplacementErrorV1::RawDataRequired);
    }
    if dataset.schema.sample_size.is_some() {
        return Err(ContinuousRawMeanReplacementErrorV1::RawSampleSizeMetadata);
    }
    if dataset.schema.case_count != dataset.batch.num_rows()
        || dataset.schema.columns.len() != dataset.batch.num_columns()
        || dataset
            .schema
            .columns
            .iter()
            .zip(dataset.batch.schema().fields())
            .any(|(metadata, field)| metadata.name != field.name().as_str())
    {
        return Err(ContinuousRawMeanReplacementErrorV1::DatasetShape);
    }
    if bindings.is_empty() {
        return Err(ContinuousRawMeanReplacementErrorV1::EmptyVariableSet);
    }
    let row_count = dataset.batch.num_rows();
    validate_workload_dimensions_v1(row_count, bindings.len())?;

    let mut variable_ids = HashSet::new();
    variable_ids.try_reserve(bindings.len()).map_err(|_| {
        ContinuousRawMeanReplacementErrorV1::AllocationFailed {
            buffer: "modeled variable identities",
            requested_capacity: bindings.len(),
        }
    })?;
    let mut source_columns = HashSet::new();
    source_columns.try_reserve(bindings.len()).map_err(|_| {
        ContinuousRawMeanReplacementErrorV1::AllocationFailed {
            buffer: "modeled source-column identities",
            requested_capacity: bindings.len(),
        }
    })?;
    if bindings.iter().any(|binding| {
        binding.variable_id.trim().is_empty()
            || binding.source_column.trim().is_empty()
            || !variable_ids.insert(binding.variable_id.as_str())
            || !source_columns.insert(binding.source_column.as_str())
    }) {
        return Err(ContinuousRawMeanReplacementErrorV1::DuplicateVariableBinding);
    }

    let mut resolved_columns = Vec::new();
    try_reserve_exact(
        &mut resolved_columns,
        bindings.len(),
        "resolved modeled columns",
    )?;
    let mut imputed_cell_count_preflight = 0usize;
    for binding in bindings {
        cancellation_checkpoint(should_cancel)?;
        let position = dataset
            .schema
            .columns
            .iter()
            .position(|column| column.name == binding.source_column)
            .ok_or_else(|| {
                ContinuousRawMeanReplacementErrorV1::UnknownSourceColumn(
                    binding.source_column.clone(),
                )
            })?;
        let metadata = &dataset.schema.columns[position];
        let array = dataset.batch.column(position);
        if metadata.column_type != ColumnType::Numeric
            || metadata.scale_type != ScaleType::Continuous
        {
            return Err(
                ContinuousRawMeanReplacementErrorV1::UnsupportedSourceColumn(
                    binding.source_column.clone(),
                ),
            );
        }
        let model_markers = canonical_missing_markers(&binding.missing_markers);
        let dataset_markers = canonical_missing_markers(&metadata.missing_markers);
        if model_markers != dataset_markers {
            return Err(
                ContinuousRawMeanReplacementErrorV1::MissingMarkerMetadataMismatch {
                    variable_id: binding.variable_id.clone(),
                },
            );
        }
        let (column, numeric_markers) =
            if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
                let mut numeric_markers = Vec::new();
                try_reserve_exact(
                    &mut numeric_markers,
                    dataset_markers.len(),
                    "Float64 missing markers",
                )?;
                numeric_markers.extend(dataset_markers.iter().filter_map(|marker| {
                    marker
                        .parse::<f64>()
                        .ok()
                        .filter(|value| value.is_finite())
                        .map(|value| (marker.clone(), value))
                }));
                (
                    ResolvedNumericColumnV1::Float64(values),
                    ResolvedNumericMarkersV1::Float64(numeric_markers),
                )
            } else if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
                let mut numeric_markers = Vec::new();
                try_reserve_exact(
                    &mut numeric_markers,
                    dataset_markers.len(),
                    "Int64 missing markers",
                )?;
                numeric_markers.extend(dataset_markers.iter().filter_map(|marker| {
                    marker
                        .parse::<i64>()
                        .ok()
                        .map(|value| (marker.clone(), value))
                }));
                (
                    ResolvedNumericColumnV1::Int64(values),
                    ResolvedNumericMarkersV1::Int64(numeric_markers),
                )
            } else {
                return Err(
                    ContinuousRawMeanReplacementErrorV1::UnsupportedSourceColumn(
                        binding.source_column.clone(),
                    ),
                );
            };
        imputed_cell_count_preflight = imputed_cell_count_preflight
            .checked_add(column.null_count())
            .ok_or(ContinuousRawMeanReplacementErrorV1::ImputedCellCountOverflow)?;
        validate_imputed_cell_count_v1(imputed_cell_count_preflight)?;
        resolved_columns.push(ResolvedColumnV1 {
            column,
            canonical_missing_markers: dataset_markers,
            numeric_markers,
        });
    }
    cancellation_checkpoint(should_cancel)?;

    let mut means = Vec::new();
    try_reserve_exact(&mut means, bindings.len(), "replacement means")?;
    let mut variables = Vec::new();
    try_reserve_exact(&mut variables, bindings.len(), "variable receipts")?;
    let mut work_units = 0usize;
    for (variable_order, (binding, resolved)) in bindings.iter().zip(&resolved_columns).enumerate()
    {
        let observed_count = row_count - resolved.column.null_count();
        if observed_count == 0 {
            return Err(ContinuousRawMeanReplacementErrorV1::AllMissingVariable(
                binding.variable_id.clone(),
            ));
        }
        let mut sum = CompensatedSumV1::default();
        for row in 0..row_count {
            poll_cancellation(should_cancel, &mut work_units)?;
            if resolved.column.is_null(row) {
                continue;
            }
            let cell = resolved.column.exact_value(row, &binding.source_column)?;
            if let Some(marker) = resolved.numeric_markers.matching_marker(&cell) {
                return Err(
                    ContinuousRawMeanReplacementErrorV1::UnresolvedMissingMarker {
                        source_column: binding.source_column.clone(),
                        row_index_zero_based: row,
                        marker: marker.into(),
                    },
                );
            }
            if !sum.add(cell.as_f64()) {
                return Err(ContinuousRawMeanReplacementErrorV1::NonFiniteMean(
                    binding.variable_id.clone(),
                ));
            }
        }
        let mean = sum
            .total()
            .filter(|value| value.is_finite())
            .map(|sum| sum / observed_count as f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| {
                ContinuousRawMeanReplacementErrorV1::NonFiniteMean(binding.variable_id.clone())
            })?;
        let missing_count = row_count - observed_count;
        let missing_fraction = fraction(missing_count, row_count);
        let warning_level = if above_percent(missing_count, row_count, 15) {
            MeanReplacementWarningLevelV1::AboveFifteenPercent
        } else if at_least_percent(missing_count, row_count, 5) {
            MeanReplacementWarningLevelV1::AtLeastFivePercent
        } else {
            MeanReplacementWarningLevelV1::None
        };
        means.push(mean);
        variables.push(MeanReplacementVariableReceiptV1 {
            variable_order,
            variable_id: binding.variable_id.clone(),
            source_column: binding.source_column.clone(),
            canonical_missing_markers: resolved.canonical_missing_markers.clone(),
            observed_count,
            missing_count,
            replacement_mean: mean,
            missing_fraction,
            warning_level,
        });
    }
    cancellation_checkpoint(should_cancel)?;

    let mut rows = Vec::new();
    try_reserve_exact(&mut rows, row_count, "completed rows")?;
    let mut cases = Vec::new();
    try_reserve_exact(
        &mut cases,
        row_count.min(imputed_cell_count_preflight),
        "affected-case receipts",
    )?;
    let mut imputed_cell_count = 0usize;
    for row_index in 0..row_count {
        let mut missing_in_row = 0usize;
        for resolved in &resolved_columns {
            poll_cancellation(should_cancel, &mut work_units)?;
            if resolved.column.is_null(row_index) {
                missing_in_row += 1;
            }
        }
        let mut row = Vec::new();
        try_reserve_exact(&mut row, bindings.len(), "completed row values")?;
        let mut imputed_variable_ids = Vec::new();
        try_reserve_exact(
            &mut imputed_variable_ids,
            missing_in_row,
            "case imputed-variable identities",
        )?;
        for (variable_order, (binding, resolved)) in
            bindings.iter().zip(&resolved_columns).enumerate()
        {
            poll_cancellation(should_cancel, &mut work_units)?;
            if resolved.column.is_null(row_index) {
                row.push(means[variable_order]);
                imputed_variable_ids.push(binding.variable_id.clone());
                imputed_cell_count = imputed_cell_count
                    .checked_add(1)
                    .ok_or(ContinuousRawMeanReplacementErrorV1::ImputedCellCountOverflow)?;
            } else {
                row.push(
                    resolved
                        .column
                        .exact_value(row_index, &binding.source_column)?
                        .as_f64(),
                );
            }
        }
        if !imputed_variable_ids.is_empty() {
            let missing_fraction = fraction(imputed_variable_ids.len(), bindings.len());
            cases.push(MeanReplacementCaseReceiptV1 {
                row_index_zero_based: row_index,
                high_missingness_warning: above_percent(
                    imputed_variable_ids.len(),
                    bindings.len(),
                    15,
                ),
                imputed_variable_ids,
                missing_fraction,
            });
        }
        rows.push(row);
    }
    if imputed_cell_count != imputed_cell_count_preflight {
        return Err(ContinuousRawMeanReplacementErrorV1::DatasetShape);
    }
    cancellation_checkpoint(should_cancel)?;

    for (column, binding) in bindings.iter().enumerate() {
        let first = rows[0][column];
        if rows.iter().all(|row| row[column] == first) {
            return Err(ContinuousRawMeanReplacementErrorV1::ConstantVariable(
                binding.variable_id.clone(),
            ));
        }
        let mut variance = CompensatedSumV1::default();
        for row in &rows {
            poll_cancellation(should_cancel, &mut work_units)?;
            let deviation = row[column] - means[column];
            let squared = deviation * deviation;
            if !squared.is_finite() || !variance.add(squared) {
                return Err(ContinuousRawMeanReplacementErrorV1::NonFiniteVariance(
                    binding.variable_id.clone(),
                ));
            }
        }
        let variance_sum = variance.total().ok_or_else(|| {
            ContinuousRawMeanReplacementErrorV1::NonFiniteVariance(binding.variable_id.clone())
        })?;
        if !variance_sum.is_finite() {
            return Err(ContinuousRawMeanReplacementErrorV1::NonFiniteVariance(
                binding.variable_id.clone(),
            ));
        }
        if variance_sum <= 0.0 {
            return Err(ContinuousRawMeanReplacementErrorV1::ConstantVariable(
                binding.variable_id.clone(),
            ));
        }
    }
    cancellation_checkpoint(should_cancel)?;

    let missingness_sha256 =
        hash_missingness_with_control(dataset, &variables, &resolved_columns, should_cancel)?;
    let completed_matrix_sha256 =
        hash_completed_matrix_with_control(dataset, &variables, &rows, should_cancel)?;
    let mut receipt = MeanReplacementReceiptV1 {
        method_version: MEAN_REPLACEMENT_METHOD_VERSION_V1.into(),
        policy: MeanReplacementPolicyV1::MeanReplacement,
        source_dataset_id: dataset.id.to_string(),
        source_dataset_fingerprint: dataset.fingerprint.0.clone(),
        source_row_count: row_count,
        retained_row_count: row_count,
        omitted_row_count: 0,
        modeled_variable_count: bindings.len(),
        imputed_cell_count,
        affected_case_count: cases.len(),
        variable_warning_threshold: MEAN_REPLACEMENT_VARIABLE_WARNING_THRESHOLD_V1,
        high_missingness_threshold: MEAN_REPLACEMENT_HIGH_MISSINGNESS_THRESHOLD_V1,
        variables,
        cases,
        missingness_sha256,
        completed_matrix_sha256,
        receipt_sha256: String::new(),
    };
    receipt.receipt_sha256 = receipt_sha256_with_control(&receipt, should_cancel)?;
    cancellation_checkpoint(should_cancel)?;
    Ok(PreparedContinuousRawMeanReplacementV1 { receipt, rows })
}

fn validate_dataset_integrity(
    dataset: &Dataset,
) -> Result<(), ContinuousRawMeanReplacementErrorV1> {
    let bytes = write_arrow(&dataset.batch).map_err(|error| {
        ContinuousRawMeanReplacementErrorV1::DatasetIntegrity(error.to_string())
    })?;
    dataset_from_descriptor(DatasetDescriptor::from(dataset), &bytes)
        .map(|_| ())
        .map_err(|error| ContinuousRawMeanReplacementErrorV1::DatasetIntegrity(error.to_string()))
}

fn validate_workload_dimensions_v1(
    row_count: usize,
    variable_count: usize,
) -> Result<usize, ContinuousRawMeanReplacementErrorV1> {
    let modeled_cell_count = row_count.checked_mul(variable_count).ok_or(
        ContinuousRawMeanReplacementErrorV1::ModeledCellCountOverflow {
            row_count,
            variable_count,
        },
    )?;
    if row_count > MEAN_REPLACEMENT_MAX_ROWS_V1 {
        return Err(ContinuousRawMeanReplacementErrorV1::RowLimitExceeded {
            actual: row_count,
            maximum: MEAN_REPLACEMENT_MAX_ROWS_V1,
        });
    }
    if variable_count > MEAN_REPLACEMENT_MAX_VARIABLES_V1 {
        return Err(ContinuousRawMeanReplacementErrorV1::VariableLimitExceeded {
            actual: variable_count,
            maximum: MEAN_REPLACEMENT_MAX_VARIABLES_V1,
        });
    }
    if modeled_cell_count > MEAN_REPLACEMENT_MAX_MODELED_CELLS_V1 {
        return Err(
            ContinuousRawMeanReplacementErrorV1::ModeledCellLimitExceeded {
                actual: modeled_cell_count,
                maximum: MEAN_REPLACEMENT_MAX_MODELED_CELLS_V1,
            },
        );
    }
    Ok(modeled_cell_count)
}

fn validate_imputed_cell_count_v1(
    imputed_cell_count: usize,
) -> Result<(), ContinuousRawMeanReplacementErrorV1> {
    if imputed_cell_count > MEAN_REPLACEMENT_MAX_IMPUTED_CELLS_V1 {
        return Err(
            ContinuousRawMeanReplacementErrorV1::ImputedCellLimitExceeded {
                actual: imputed_cell_count,
                maximum: MEAN_REPLACEMENT_MAX_IMPUTED_CELLS_V1,
            },
        );
    }
    Ok(())
}

fn try_reserve_exact<T>(
    values: &mut Vec<T>,
    requested_capacity: usize,
    buffer: &'static str,
) -> Result<(), ContinuousRawMeanReplacementErrorV1> {
    values.try_reserve_exact(requested_capacity).map_err(|_| {
        ContinuousRawMeanReplacementErrorV1::AllocationFailed {
            buffer,
            requested_capacity,
        }
    })
}

fn cancellation_checkpoint(
    should_cancel: &impl Fn() -> bool,
) -> Result<(), ContinuousRawMeanReplacementErrorV1> {
    if should_cancel() {
        Err(ContinuousRawMeanReplacementErrorV1::Cancelled)
    } else {
        Ok(())
    }
}

fn poll_cancellation(
    should_cancel: &impl Fn() -> bool,
    work_units: &mut usize,
) -> Result<(), ContinuousRawMeanReplacementErrorV1> {
    if *work_units % CANCELLATION_POLL_INTERVAL_V1 == 0 {
        cancellation_checkpoint(should_cancel)?;
    }
    *work_units += 1;
    Ok(())
}

fn covariance_ml_from_receipt_means(
    rows: &[Vec<f64>],
    receipt: &MeanReplacementReceiptV1,
    should_cancel: &impl Fn() -> bool,
) -> Result<Vec<Vec<f64>>, ContinuousRawMeanReplacementErrorV1> {
    cancellation_checkpoint(should_cancel)?;
    let variable_count = receipt.variables.len();
    if rows.is_empty() || variable_count == 0 || rows.iter().any(|row| row.len() != variable_count)
    {
        return Err(ContinuousRawMeanReplacementErrorV1::DatasetShape);
    }
    let mut covariance = Vec::new();
    try_reserve_exact(
        &mut covariance,
        variable_count,
        "mean-replacement covariance rows",
    )?;
    for _ in 0..variable_count {
        let mut row = Vec::new();
        try_reserve_exact(
            &mut row,
            variable_count,
            "mean-replacement covariance cells",
        )?;
        row.resize(variable_count, 0.0);
        covariance.push(row);
    }
    let mut work_units = 0usize;
    for left in 0..variable_count {
        let left_variable = &receipt.variables[left];
        for right in left..variable_count {
            let right_variable = &receipt.variables[right];
            let mut cross_products = CompensatedSumV1::default();
            for (row_index, row) in rows.iter().enumerate() {
                poll_cancellation(should_cancel, &mut work_units)?;
                let product = (row[left] - left_variable.replacement_mean)
                    * (row[right] - right_variable.replacement_mean);
                if !product.is_finite() || !cross_products.add(product) {
                    return Err(ContinuousRawMeanReplacementErrorV1::NonFiniteCrossProduct {
                        left_variable_id: left_variable.variable_id.clone(),
                        right_variable_id: right_variable.variable_id.clone(),
                        row_index_zero_based: row_index,
                    });
                }
            }
            let value = cross_products
                .total()
                .map(|sum| sum / rows.len() as f64)
                .filter(|value| value.is_finite())
                .ok_or_else(
                    || ContinuousRawMeanReplacementErrorV1::NonFiniteCovariance {
                        left_variable_id: left_variable.variable_id.clone(),
                        right_variable_id: right_variable.variable_id.clone(),
                    },
                )?;
            covariance[left][right] = value;
            covariance[right][left] = value;
        }
    }
    cancellation_checkpoint(should_cancel)?;
    Ok(covariance)
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

fn at_least_percent(count: usize, total: usize, percent: u128) -> bool {
    total > 0 && (count as u128) * 100 >= (total as u128) * percent
}

fn above_percent(count: usize, total: usize, percent: u128) -> bool {
    total > 0 && (count as u128) * 100 > (total as u128) * percent
}

fn fraction(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
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

fn receipt_sha256_with_control(
    receipt: &MeanReplacementReceiptV1,
    should_cancel: &impl Fn() -> bool,
) -> Result<String, ContinuousRawMeanReplacementErrorV1> {
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
    let mut writer = CancellableHashWriterV1::new(digest, should_cancel);
    if let Err(error) = serde_json::to_writer(&mut writer, &input) {
        if writer.cancelled {
            return Err(ContinuousRawMeanReplacementErrorV1::Cancelled);
        }
        return Err(
            ContinuousRawMeanReplacementErrorV1::ReceiptHashSerialization(error.to_string()),
        );
    }
    cancellation_checkpoint(should_cancel)?;
    Ok(format!("{:x}", writer.digest.finalize()))
}

#[cfg(test)]
fn receipt_sha256(receipt: &MeanReplacementReceiptV1) -> String {
    receipt_sha256_with_control(receipt, &|| false)
        .expect("mean-replacement receipt hash input must serialize")
}

struct CancellableHashWriterV1<'a, F: Fn() -> bool> {
    digest: Sha256,
    should_cancel: &'a F,
    bytes_since_poll: usize,
    cancelled: bool,
}

impl<'a, F: Fn() -> bool> CancellableHashWriterV1<'a, F> {
    fn new(digest: Sha256, should_cancel: &'a F) -> Self {
        Self {
            digest,
            should_cancel,
            bytes_since_poll: 0,
            cancelled: false,
        }
    }
}

impl<F: Fn() -> bool> Write for CancellableHashWriterV1<'_, F> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if self.bytes_since_poll == 0 && (self.should_cancel)() {
            self.cancelled = true;
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "mean-replacement receipt hashing cancelled",
            ));
        }
        let remaining = CANCELLATION_POLL_INTERVAL_V1 - self.bytes_since_poll;
        let written = remaining.min(bytes.len());
        self.digest.update(&bytes[..written]);
        self.bytes_since_poll += written;
        if self.bytes_since_poll == CANCELLATION_POLL_INTERVAL_V1 {
            self.bytes_since_poll = 0;
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash_header_with_control(
    domain: &[u8],
    dataset: &Dataset,
    variables: &[MeanReplacementVariableReceiptV1],
    should_cancel: &impl Fn() -> bool,
) -> Result<Sha256, ContinuousRawMeanReplacementErrorV1> {
    let mut digest = Sha256::new();
    digest.update(domain);
    let mut work_units = 0usize;
    hash_string_with_control(
        &mut digest,
        &dataset.fingerprint.0,
        should_cancel,
        &mut work_units,
    )?;
    digest.update((dataset.batch.num_rows() as u64).to_be_bytes());
    digest.update((variables.len() as u64).to_be_bytes());
    for variable in variables {
        poll_cancellation(should_cancel, &mut work_units)?;
        digest.update((variable.variable_order as u64).to_be_bytes());
        hash_string_with_control(
            &mut digest,
            &variable.variable_id,
            should_cancel,
            &mut work_units,
        )?;
        hash_string_with_control(
            &mut digest,
            &variable.source_column,
            should_cancel,
            &mut work_units,
        )?;
        digest.update((variable.canonical_missing_markers.len() as u64).to_be_bytes());
        for marker in &variable.canonical_missing_markers {
            hash_string_with_control(&mut digest, marker, should_cancel, &mut work_units)?;
        }
    }
    cancellation_checkpoint(should_cancel)?;
    Ok(digest)
}

fn hash_string_with_control(
    digest: &mut Sha256,
    value: &str,
    should_cancel: &impl Fn() -> bool,
    work_units: &mut usize,
) -> Result<(), ContinuousRawMeanReplacementErrorV1> {
    digest.update((value.len() as u64).to_be_bytes());
    for chunk in value.as_bytes().chunks(CANCELLATION_POLL_INTERVAL_V1) {
        poll_cancellation(should_cancel, work_units)?;
        digest.update(chunk);
    }
    Ok(())
}

fn hash_missingness_with_control(
    dataset: &Dataset,
    variables: &[MeanReplacementVariableReceiptV1],
    resolved_columns: &[ResolvedColumnV1<'_>],
    should_cancel: &impl Fn() -> bool,
) -> Result<String, ContinuousRawMeanReplacementErrorV1> {
    let mut digest = hash_header_with_control(
        b"quickpls-continuous-raw-mean-replacement-v1:missingness\0",
        dataset,
        variables,
        should_cancel,
    )?;
    let mut work_units = 0usize;
    for row in 0..dataset.batch.num_rows() {
        for resolved in resolved_columns {
            poll_cancellation(should_cancel, &mut work_units)?;
            digest.update([u8::from(resolved.column.is_null(row))]);
        }
    }
    cancellation_checkpoint(should_cancel)?;
    Ok(format!("{:x}", digest.finalize()))
}

fn hash_completed_matrix_with_control(
    dataset: &Dataset,
    variables: &[MeanReplacementVariableReceiptV1],
    rows: &[Vec<f64>],
    should_cancel: &impl Fn() -> bool,
) -> Result<String, ContinuousRawMeanReplacementErrorV1> {
    let mut digest = hash_header_with_control(
        b"quickpls-continuous-raw-mean-replacement-v1:completed-matrix\0",
        dataset,
        variables,
        should_cancel,
    )?;
    let mut work_units = 0usize;
    for row in rows {
        for value in row {
            poll_cancellation(should_cancel, &mut work_units)?;
            digest.update(value.to_bits().to_be_bytes());
        }
    }
    cancellation_checkpoint(should_cancel)?;
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::{array::ArrayRef, record_batch::RecordBatch};
    use qpls_data::{ImportOptions, import_delimited_bytes, update_column_metadata};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    fn dataset(csv: &str) -> Dataset {
        import_delimited_bytes(
            csv.as_bytes(),
            "mean-replacement.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn binding(
        dataset: &Dataset,
        variable_id: &str,
        source_column: &str,
    ) -> ContinuousRawMeanReplacementVariableBindingV1 {
        let markers = dataset
            .schema
            .columns
            .iter()
            .find(|column| column.name == source_column)
            .unwrap()
            .missing_markers
            .clone();
        ContinuousRawMeanReplacementVariableBindingV1 {
            variable_id: variable_id.into(),
            source_column: source_column.into(),
            missing_markers: canonical_missing_markers(&markers),
        }
    }

    fn prior_to_vec_receipt_sha256(receipt: &MeanReplacementReceiptV1) -> String {
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
        digest.update(serde_json::to_vec(&input).unwrap());
        format!("{:x}", digest.finalize())
    }

    fn int64_dataset(values: Vec<Option<i64>>, missing_markers: &[&str]) -> Dataset {
        let mut csv = String::from("x\n");
        for _ in 0..values.len() {
            csv.push_str("0\n");
        }
        let mut dataset = dataset(&csv);
        dataset.batch =
            RecordBatch::try_from_iter([("x", Arc::new(Int64Array::from(values)) as ArrayRef)])
                .unwrap();
        dataset.schema.case_count = dataset.batch.num_rows();
        let mut metadata = dataset.schema.columns[0].clone();
        metadata.missing_markers = missing_markers
            .iter()
            .map(|marker| (*marker).into())
            .collect();
        update_column_metadata(&mut dataset, "x", metadata).unwrap();
        dataset
    }

    #[test]
    fn int64_values_and_markers_require_exact_f64_integer_representation() {
        let exact = int64_dataset(
            vec![
                Some(-MAX_EXACT_F64_INTEGER_V1),
                Some(0),
                Some(MAX_EXACT_F64_INTEGER_V1),
                None,
            ],
            &["NA"],
        );
        let prepared = prepare_continuous_raw_mean_replacement_v1(
            &exact,
            &[binding(&exact, "observed:x", "x")],
        )
        .unwrap();
        assert_eq!(prepared.receipt.variables[0].replacement_mean, 0.0);
        assert_eq!(prepared.rows[3][0], 0.0);

        for outside in [MAX_EXACT_F64_INTEGER_V1 + 1, -MAX_EXACT_F64_INTEGER_V1 - 1] {
            let dataset = int64_dataset(vec![Some(outside), Some(0), Some(1)], &["NA"]);
            assert!(matches!(
                prepare_continuous_raw_mean_replacement_v1(
                    &dataset,
                    &[binding(&dataset, "observed:x", "x")]
                ),
                Err(
                    ContinuousRawMeanReplacementErrorV1::Int64ValueOutsideExactF64Range {
                        value,
                        ..
                    }
                ) if value == outside
            ));
        }

        let exact_marker = MAX_EXACT_F64_INTEGER_V1.to_string();
        let marker_dataset = int64_dataset(
            vec![Some(MAX_EXACT_F64_INTEGER_V1), Some(0), Some(-1)],
            &[exact_marker.as_str()],
        );
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(
                &marker_dataset,
                &[binding(&marker_dataset, "observed:x", "x")]
            ),
            Err(ContinuousRawMeanReplacementErrorV1::UnresolvedMissingMarker {
                row_index_zero_based: 0,
                marker,
                ..
            }) if marker == exact_marker
        ));

        let outside_marker = (MAX_EXACT_F64_INTEGER_V1 + 1).to_string();
        let outside_marker_dataset = int64_dataset(
            vec![Some(MAX_EXACT_F64_INTEGER_V1 + 1), Some(0), Some(-1)],
            &[outside_marker.as_str()],
        );
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(
                &outside_marker_dataset,
                &[binding(&outside_marker_dataset, "observed:x", "x")]
            ),
            Err(
                ContinuousRawMeanReplacementErrorV1::Int64ValueOutsideExactF64Range {
                    row_index_zero_based: 0,
                    ..
                }
            )
        ));
    }

    #[test]
    fn bounded_workload_preflight_reports_every_v1_limit_before_materialization() {
        assert_eq!(
            validate_workload_dimensions_v1(MEAN_REPLACEMENT_MAX_ROWS_V1, 100),
            Ok(MEAN_REPLACEMENT_MAX_MODELED_CELLS_V1)
        );
        assert!(matches!(
            validate_workload_dimensions_v1(MEAN_REPLACEMENT_MAX_ROWS_V1 + 1, 1),
            Err(ContinuousRawMeanReplacementErrorV1::RowLimitExceeded { .. })
        ));
        assert!(matches!(
            validate_workload_dimensions_v1(1, MEAN_REPLACEMENT_MAX_VARIABLES_V1 + 1),
            Err(ContinuousRawMeanReplacementErrorV1::VariableLimitExceeded { .. })
        ));
        assert!(matches!(
            validate_workload_dimensions_v1(MEAN_REPLACEMENT_MAX_ROWS_V1, 101),
            Err(ContinuousRawMeanReplacementErrorV1::ModeledCellLimitExceeded { .. })
        ));
        assert!(matches!(
            validate_workload_dimensions_v1(usize::MAX, 2),
            Err(ContinuousRawMeanReplacementErrorV1::ModeledCellCountOverflow { .. })
        ));
        assert_eq!(
            validate_imputed_cell_count_v1(MEAN_REPLACEMENT_MAX_IMPUTED_CELLS_V1),
            Ok(())
        );
        assert!(matches!(
            validate_imputed_cell_count_v1(MEAN_REPLACEMENT_MAX_IMPUTED_CELLS_V1 + 1),
            Err(ContinuousRawMeanReplacementErrorV1::ImputedCellLimitExceeded { .. })
        ));
    }

    #[test]
    fn preparation_covariance_and_receipt_hash_poll_cancellation() {
        let mut csv = String::from("x\n");
        for row in 0..2_048 {
            csv.push_str(&format!("{}\n", row as f64 - 1_024.0));
        }
        let dataset = dataset(&csv);
        let bindings = [binding(&dataset, "observed:x", "x")];
        let preparation_checks = AtomicUsize::new(0);
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1_with_control(&dataset, &bindings, || {
                preparation_checks.fetch_add(1, Ordering::SeqCst) >= 6
            }),
            Err(ContinuousRawMeanReplacementErrorV1::Cancelled)
        ));
        assert!(preparation_checks.load(Ordering::SeqCst) >= 7);

        let prepared = prepare_continuous_raw_mean_replacement_v1(&dataset, &bindings).unwrap();
        let covariance_checks = AtomicUsize::new(0);
        assert!(matches!(
            prepared.covariance_ml_with_control(&|| {
                covariance_checks.fetch_add(1, Ordering::SeqCst) >= 2
            }),
            Err(ContinuousRawMeanReplacementErrorV1::Cancelled)
        ));
        assert!(covariance_checks.load(Ordering::SeqCst) >= 3);

        let receipt_checks = AtomicUsize::new(0);
        assert!(matches!(
            receipt_sha256_with_control(&prepared.receipt, &|| {
                receipt_checks.fetch_add(1, Ordering::SeqCst) >= 1
            }),
            Err(ContinuousRawMeanReplacementErrorV1::Cancelled)
        ));
        assert!(receipt_checks.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn receipt_means_and_compensated_cross_products_resist_adversarial_roundoff() {
        let dataset =
            dataset("x,y\n10000000000000000,1\n1,1\n-10000000000000000,1\n-1,-3\nNA,NA\n");
        let bindings = [
            binding(&dataset, "observed:x", "x"),
            binding(&dataset, "observed:y", "y"),
        ];
        let prepared = prepare_continuous_raw_mean_replacement_v1(&dataset, &bindings).unwrap();
        assert_eq!(prepared.receipt.variables[0].replacement_mean, 0.0);
        assert_eq!(prepared.receipt.variables[1].replacement_mean, 0.0);
        let covariance = prepared.covariance_ml_with_control(&|| false).unwrap();
        assert_eq!(covariance[0][1].to_bits(), (4.0_f64 / 5.0).to_bits());
        assert_eq!(covariance[1][0].to_bits(), covariance[0][1].to_bits());

        let naive_means = (0..2)
            .map(|column| {
                prepared.rows.iter().map(|row| row[column]).sum::<f64>()
                    / prepared.rows.len() as f64
            })
            .collect::<Vec<_>>();
        let naive_cross_product = prepared
            .rows
            .iter()
            .map(|row| (row[0] - naive_means[0]) * (row[1] - naive_means[1]))
            .sum::<f64>()
            / prepared.rows.len() as f64;
        assert_ne!(naive_cross_product.to_bits(), covariance[0][1].to_bits());
    }

    #[test]
    fn cellwise_replacement_retains_all_missing_rows_and_emits_exact_threshold_warnings() {
        let mut csv = String::from("x1,x2\n");
        for row in 0..20 {
            let x1 = if row == 0 {
                "NA".into()
            } else {
                (row + 1).to_string()
            };
            let x2 = if row < 4 {
                "NA".into()
            } else {
                (2 * row + 3).to_string()
            };
            csv.push_str(&format!("{x1},{x2}\n"));
        }
        let dataset = dataset(&csv);
        let before_arrow = write_arrow(&dataset.batch).unwrap();
        let before_fingerprint = dataset.fingerprint.clone();
        let bindings = [
            binding(&dataset, "observed:x1", "x1"),
            binding(&dataset, "observed:x2", "x2"),
        ];
        let first = prepare_continuous_raw_mean_replacement_v1(&dataset, &bindings).unwrap();
        let second = prepare_continuous_raw_mean_replacement_v1(&dataset, &bindings).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.receipt.retained_row_count, 20);
        assert_eq!(first.receipt.omitted_row_count, 0);
        assert_eq!(first.receipt.imputed_cell_count, 5);
        assert_eq!(first.receipt.affected_case_count, 4);
        assert_eq!(
            first.rows[0][0],
            first.receipt.variables[0].replacement_mean
        );
        assert_eq!(
            first.rows[0][1],
            first.receipt.variables[1].replacement_mean
        );
        assert_eq!(
            first.receipt.variables[0].warning_level,
            MeanReplacementWarningLevelV1::AtLeastFivePercent
        );
        assert_eq!(
            first.receipt.variables[1].warning_level,
            MeanReplacementWarningLevelV1::AboveFifteenPercent
        );
        assert_eq!(first.receipt.cases[0].row_index_zero_based, 0);
        assert_eq!(
            first.receipt.cases[0].imputed_variable_ids,
            vec!["observed:x1", "observed:x2"]
        );
        assert_eq!(first.receipt.cases[0].missing_fraction, 1.0);
        assert!(first.receipt.cases[0].high_missingness_warning);
        assert_eq!(first.receipt.receipt_sha256.len(), 64);
        assert_eq!(receipt_sha256(&first.receipt), first.receipt.receipt_sha256);
        assert_eq!(
            prior_to_vec_receipt_sha256(&first.receipt),
            first.receipt.receipt_sha256
        );
        assert_eq!(write_arrow(&dataset.batch).unwrap(), before_arrow);
        assert_eq!(dataset.fingerprint, before_fingerprint);
    }

    #[test]
    fn marker_mismatch_all_missing_variable_nonfinite_and_constant_fail_closed() {
        let all_missing_dataset = dataset("x1,x2\n1,NA\n2,NA\n3,NA\n");
        let mut mismatch = binding(&all_missing_dataset, "observed:x1", "x1");
        mismatch.missing_markers = vec!["-99".into()];
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(&all_missing_dataset, &[mismatch]),
            Err(ContinuousRawMeanReplacementErrorV1::MissingMarkerMetadataMismatch { .. })
        ));
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(
                &all_missing_dataset,
                &[binding(&all_missing_dataset, "observed:x2", "x2")]
            ),
            Err(ContinuousRawMeanReplacementErrorV1::AllMissingVariable(_))
        ));

        let constant = dataset("x1\n2\n2\nNA\n");
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(
                &constant,
                &[binding(&constant, "observed:x1", "x1")]
            ),
            Err(ContinuousRawMeanReplacementErrorV1::ConstantVariable(_))
        ));

        let unsupported = dataset("x1\nA\nB\nC\n");
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(
                &unsupported,
                &[binding(&unsupported, "observed:x1", "x1")]
            ),
            Err(ContinuousRawMeanReplacementErrorV1::UnsupportedSourceColumn(_))
        ));

        let nonfinite = dataset("x1\n1\nNaN\n2\n");
        assert!(matches!(
            prepare_continuous_raw_mean_replacement_v1(
                &nonfinite,
                &[binding(&nonfinite, "observed:x1", "x1")]
            ),
            Err(ContinuousRawMeanReplacementErrorV1::NonFiniteValue { .. })
        ));
    }
}
