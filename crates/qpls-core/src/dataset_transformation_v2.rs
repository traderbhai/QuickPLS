use arrow::{
    array::{Array, ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray, StringBuilder},
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
};
use chrono::{DateTime, SecondsFormat, Utc};
use qpls_data::{
    ColumnMetadata, ColumnType, DataFingerprint, DataKind, Dataset, DatasetSchema, ScaleType,
    write_arrow,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    error::Error,
    fmt,
    sync::Arc,
};
use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

pub const DATASET_TRANSFORMATION_SCHEMA_V2: u32 = 2;
pub const DATASET_TRANSFORMATION_ENGINE_V2: &str = "qpls.dataset_transform.v2";

const DEFAULT_PREVIEW_LIMIT: i64 = 20;
const MAX_PREVIEW_LIMIT: i64 = 100;
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// The persisted project-row boundary is deliberately limited to finite
/// numbers, strings, and missing values. In particular, a binary column may
/// have Boolean metadata while its cells remain numeric zero/one values.
#[derive(Debug, Clone, PartialEq)]
pub enum DatasetCellV2 {
    Number(f64),
    Text(String),
    Missing,
}

impl Serialize for DatasetCellV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Number(value) if value.is_finite() => {
                serializer.serialize_f64(normalize_zero(*value))
            }
            Self::Number(_) => Err(serde::ser::Error::custom(
                "dataset cells must contain finite numbers",
            )),
            Self::Text(value) => serializer.serialize_str(value),
            Self::Missing => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for DatasetCellV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DatasetCellV2;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a finite number, string, or null")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(DatasetCellV2::Missing)
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(DatasetCellV2::Missing)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.unsigned_abs() > MAX_SAFE_INTEGER as u64 {
                    return Err(E::custom("dataset numbers must be JavaScript-safe"));
                }
                Ok(DatasetCellV2::Number(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value > MAX_SAFE_INTEGER as u64 {
                    return Err(E::custom("dataset numbers must be JavaScript-safe"));
                }
                Ok(DatasetCellV2::Number(value as f64))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if !value.is_finite() {
                    return Err(E::custom("dataset numbers must be finite"));
                }
                Ok(DatasetCellV2::Number(normalize_zero(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(DatasetCellV2::Text(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                Ok(DatasetCellV2::Text(value))
            }

            fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Err(E::custom(
                    "boolean values are not part of the dataset-row wire contract",
                ))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NonMissingDatasetCellV2 {
    Number(f64),
    Text(String),
}

impl NonMissingDatasetCellV2 {
    fn as_cell(&self) -> DatasetCellV2 {
        match self {
            Self::Number(value) => DatasetCellV2::Number(normalize_zero(*value)),
            Self::Text(value) => DatasetCellV2::Text(value.clone()),
        }
    }

    fn is_finite(&self) -> bool {
        match self {
            Self::Number(value) => value.is_finite(),
            Self::Text(_) => true,
        }
    }
}

impl Serialize for NonMissingDatasetCellV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Number(value) if value.is_finite() => {
                serializer.serialize_f64(normalize_zero(*value))
            }
            Self::Number(_) => Err(serde::ser::Error::custom(
                "dataset cells must contain finite numbers",
            )),
            Self::Text(value) => serializer.serialize_str(value),
        }
    }
}

impl<'de> Deserialize<'de> for NonMissingDatasetCellV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match DatasetCellV2::deserialize(deserializer)? {
            DatasetCellV2::Number(value) => Ok(Self::Number(value)),
            DatasetCellV2::Text(value) => Ok(Self::Text(value)),
            DatasetCellV2::Missing => Err(de::Error::custom("this value cannot be null")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DatasetMissingPolicyV2 {
    Propagate,
    Available,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StandardDeviationDenominatorV2 {
    SampleNMinusOne,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecodeUnmappedV2 {
    Keep,
    Missing,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOperatorV2 {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RowAggregateOperationV2 {
    Sum,
    Mean,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DummyMissingPolicyV2 {
    Missing,
    Zero,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupUnmatchedPolicyV2 {
    Missing,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DatasetRecodeMappingV2 {
    pub source: NonMissingDatasetCellV2,
    pub target: DatasetCellV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DatasetMissingMarkerColumnV2 {
    pub source_column: String,
    pub target_column: String,
    pub markers: Vec<NonMissingDatasetCellV2>,
    pub target_type: ColumnType,
    pub target_scale: ScaleType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_label: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub value_labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DatasetArithmeticRightV2 {
    Column { column: String },
    Constant { value: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DatasetGroupRuleV2 {
    Values {
        output: NonMissingDatasetCellV2,
        values: Vec<NonMissingDatasetCellV2>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    NumericRange {
        output: NonMissingDatasetCellV2,
        minimum: Option<f64>,
        maximum: Option<f64>,
        include_minimum: bool,
        include_maximum: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

impl DatasetGroupRuleV2 {
    fn output(&self) -> &NonMissingDatasetCellV2 {
        match self {
            Self::Values { output, .. } | Self::NumericRange { output, .. } => output,
        }
    }

    fn label(&self) -> Option<&str> {
        match self {
            Self::Values { label, .. } | Self::NumericRange { label, .. } => label.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DatasetTransformationSpecV2 {
    AddColumn {
        target_column: String,
        value: DatasetCellV2,
        target_type: ColumnType,
        target_scale: ScaleType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        value_labels: BTreeMap<String, String>,
    },
    MissingMarkers {
        columns: Vec<DatasetMissingMarkerColumnV2>,
    },
    ReverseScale {
        source_column: String,
        target_column: String,
        scale_min: f64,
        scale_max: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
    },
    Standardize {
        source_column: String,
        target_column: String,
        denominator: StandardDeviationDenominatorV2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
    },
    Recode {
        source_column: String,
        target_column: String,
        mappings: Vec<DatasetRecodeMappingV2>,
        unmapped: RecodeUnmappedV2,
        target_type: ColumnType,
        target_scale: ScaleType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        value_labels: BTreeMap<String, String>,
    },
    Arithmetic {
        left_column: String,
        right: DatasetArithmeticRightV2,
        operator: ArithmeticOperatorV2,
        target_column: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
    },
    RowAggregate {
        source_columns: Vec<String>,
        operation: RowAggregateOperationV2,
        missing_policy: DatasetMissingPolicyV2,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        minimum_non_missing: Option<usize>,
        target_column: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
    },
    Dummy {
        source_column: String,
        match_value: NonMissingDatasetCellV2,
        missing_policy: DummyMissingPolicyV2,
        target_column: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
    },
    Group {
        source_column: String,
        rules: Vec<DatasetGroupRuleV2>,
        unmatched: GroupUnmatchedPolicyV2,
        target_column: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_label: Option<String>,
    },
}

impl DatasetTransformationSpecV2 {
    pub fn input_columns(&self) -> Vec<String> {
        match self {
            Self::AddColumn { .. } => Vec::new(),
            Self::MissingMarkers { columns } => columns
                .iter()
                .map(|column| column.source_column.clone())
                .collect(),
            Self::Arithmetic {
                left_column, right, ..
            } => match right {
                DatasetArithmeticRightV2::Column { column } => {
                    vec![left_column.clone(), column.clone()]
                }
                DatasetArithmeticRightV2::Constant { .. } => vec![left_column.clone()],
            },
            Self::RowAggregate { source_columns, .. } => source_columns.clone(),
            Self::ReverseScale { source_column, .. }
            | Self::Standardize { source_column, .. }
            | Self::Recode { source_column, .. }
            | Self::Dummy { source_column, .. }
            | Self::Group { source_column, .. } => vec![source_column.clone()],
        }
    }

    pub fn target_column(&self) -> &str {
        match self {
            Self::MissingMarkers { columns } => columns
                .first()
                .map(|column| column.target_column.as_str())
                .unwrap_or(""),
            Self::AddColumn { target_column, .. }
            | Self::ReverseScale { target_column, .. }
            | Self::Standardize { target_column, .. }
            | Self::Recode { target_column, .. }
            | Self::Arithmetic { target_column, .. }
            | Self::RowAggregate { target_column, .. }
            | Self::Dummy { target_column, .. }
            | Self::Group { target_column, .. } => target_column,
        }
    }

    pub fn target_columns(&self) -> Vec<String> {
        match self {
            Self::MissingMarkers { columns } => columns
                .iter()
                .map(|column| column.target_column.clone())
                .collect(),
            _ => vec![self.target_column().to_owned()],
        }
    }

    fn target_label(&self) -> Option<&str> {
        match self {
            Self::MissingMarkers { columns } => columns
                .first()
                .and_then(|column| column.target_label.as_deref()),
            Self::AddColumn { target_label, .. }
            | Self::ReverseScale { target_label, .. }
            | Self::Standardize { target_label, .. }
            | Self::Recode { target_label, .. }
            | Self::Arithmetic { target_label, .. }
            | Self::RowAggregate { target_label, .. }
            | Self::Dummy { target_label, .. }
            | Self::Group { target_label, .. } => target_label.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatasetTransformationIssueV2 {
    pub code: String,
    pub field: String,
    pub message: String,
    pub row_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DatasetTransformationPreviewRowV2 {
    pub row_index: usize,
    pub inputs: BTreeMap<String, DatasetCellV2>,
    pub output: DatasetCellV2,
    pub outputs: BTreeMap<String, DatasetCellV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DatasetTransformationPreviewV2 {
    pub schema_version: u32,
    pub source_dataset_id: String,
    pub target_column: String,
    pub output_columns: Vec<String>,
    pub input_columns: Vec<String>,
    pub inspected_rows: usize,
    pub total_rows: usize,
    /// Total missing cells across all derived output columns.
    pub output_missing_count: usize,
    pub rows: Vec<DatasetTransformationPreviewRowV2>,
    pub issues: Vec<DatasetTransformationIssueV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DatasetTransformationLineageV2 {
    pub schema_version: u32,
    pub engine: String,
    pub operation_id: String,
    pub source_dataset_id: String,
    pub source_dataset_fingerprint: String,
    pub output_dataset_id: String,
    pub output_dataset_fingerprint: String,
    pub created_at: String,
    pub spec_sha256: String,
    pub spec: DatasetTransformationSpecV2,
    pub input_columns: Vec<String>,
    pub output_columns: Vec<String>,
    pub source_row_count: usize,
    /// Total missing cells across all derived output columns.
    pub output_missing_count: usize,
}

#[derive(Debug, Clone)]
pub struct DatasetTransformationMutationV2 {
    pub dataset: Dataset,
    pub lineage: DatasetTransformationLineageV2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApplyDatasetTransformationOptionsV2 {
    pub output_dataset_id: String,
    pub output_dataset_name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DatasetTransformationErrorV2 {
    pub issues: Vec<DatasetTransformationIssueV2>,
}

impl fmt::Display for DatasetTransformationErrorV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            self.issues
                .first()
                .map(|issue| issue.message.as_str())
                .unwrap_or("Dataset transformation is invalid."),
        )
    }
}

impl Error for DatasetTransformationErrorV2 {}

fn issue(
    code: impl Into<String>,
    field: impl Into<String>,
    message: impl Into<String>,
    row_index: Option<usize>,
) -> DatasetTransformationIssueV2 {
    DatasetTransformationIssueV2 {
        code: code.into(),
        field: field.into(),
        message: message.into(),
        row_index,
    }
}

fn failure(issue: DatasetTransformationIssueV2) -> DatasetTransformationErrorV2 {
    DatasetTransformationErrorV2 {
        issues: vec![issue],
    }
}

fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn stable_value(value: Value) -> Result<Value, DatasetTransformationErrorV2> {
    match value {
        Value::Array(values) => values
            .into_iter()
            .map(stable_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(values) => {
            let sorted = values.into_iter().collect::<BTreeMap<_, _>>();
            let mut object = Map::new();
            for (key, value) in sorted {
                object.insert(key, stable_value(value)?);
            }
            Ok(Value::Object(object))
        }
        Value::Number(number) => {
            let Some(value) = number.as_f64() else {
                return Ok(Value::Number(number));
            };
            if !value.is_finite() {
                return Err(failure(issue(
                    "canonical.non_finite",
                    "value",
                    "Canonical transformation values must be finite.",
                    None,
                )));
            }
            let value = normalize_zero(value);
            if value.fract() == 0.0
                && value >= -(MAX_SAFE_INTEGER as f64)
                && value <= MAX_SAFE_INTEGER as f64
            {
                return Ok(Value::Number(Number::from(value as i64)));
            }
            Number::from_f64(value).map(Value::Number).ok_or_else(|| {
                failure(issue(
                    "canonical.non_finite",
                    "value",
                    "Canonical transformation values must be finite.",
                    None,
                ))
            })
        }
        other => Ok(other),
    }
}

pub fn canonical_dataset_transformation_json_v2<T: Serialize>(
    value: &T,
) -> Result<String, DatasetTransformationErrorV2> {
    let value = serde_json::to_value(value).map_err(|_| {
        failure(issue(
            "canonical.serialization_failed",
            "value",
            "The transformation value cannot be represented canonically.",
            None,
        ))
    })?;
    serde_json::to_string(&stable_value(value)?).map_err(|_| {
        failure(issue(
            "canonical.serialization_failed",
            "value",
            "The transformation value cannot be represented canonically.",
            None,
        ))
    })
}

fn sha256_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn normalized_column_key(value: &str) -> String {
    value.nfkc().collect::<String>().to_lowercase()
}

fn number_key(value: f64) -> String {
    let normalized = normalize_zero(value);
    if normalized.fract() == 0.0
        && normalized >= -(MAX_SAFE_INTEGER as f64)
        && normalized <= MAX_SAFE_INTEGER as f64
    {
        (normalized as i64).to_string()
    } else {
        normalized.to_string()
    }
}

fn value_key(value: &NonMissingDatasetCellV2) -> String {
    match value {
        NonMissingDatasetCellV2::Number(value) => format!("number:{}", number_key(*value)),
        NonMissingDatasetCellV2::Text(value) => format!("string:{value}"),
    }
}

fn cell_matches(left: &DatasetCellV2, right: &NonMissingDatasetCellV2) -> bool {
    match (left, right) {
        (DatasetCellV2::Number(left), NonMissingDatasetCellV2::Number(right)) => left == right,
        (DatasetCellV2::Text(left), NonMissingDatasetCellV2::Text(right)) => left == right,
        _ => false,
    }
}

fn cell_display(value: &NonMissingDatasetCellV2) -> String {
    match value {
        NonMissingDatasetCellV2::Number(value) => number_key(*value),
        NonMissingDatasetCellV2::Text(value) => value.clone(),
    }
}

fn validate_dataset(dataset: &Dataset) -> Vec<DatasetTransformationIssueV2> {
    let mut issues = Vec::new();
    if dataset.schema.kind != DataKind::Raw {
        issues.push(issue(
            "dataset.raw_required",
            "dataset",
            "Choose a raw-observation dataset before deriving a variable.",
            None,
        ));
    }
    if dataset.schema.case_count != dataset.batch.num_rows() {
        issues.push(issue(
            "dataset.rows_not_resident",
            "dataset",
            "Load all observation rows before deriving a variable.",
            None,
        ));
    }
    if dataset.schema.columns.len() != dataset.batch.num_columns() {
        issues.push(issue(
            "dataset.columns_not_resident",
            "dataset",
            "Load all declared columns before deriving a variable.",
            None,
        ));
    }

    let mut normalized = HashSet::new();
    for (index, column) in dataset.schema.columns.iter().enumerate() {
        if column.name.trim().is_empty() {
            issues.push(issue(
                "dataset.column_blank",
                format!("columns.{index}"),
                "Column names cannot be blank.",
                None,
            ));
        }
        let key = normalized_column_key(&column.name);
        if !normalized.insert(key) {
            issues.push(issue(
                "dataset.column_duplicate",
                format!("columns.{index}"),
                format!("Column {} is duplicated.", column.name),
                None,
            ));
        }
        if dataset
            .batch
            .schema()
            .fields()
            .get(index)
            .is_some_and(|field| field.name() != &column.name)
        {
            issues.push(issue(
                "dataset.column_binding_mismatch",
                format!("columns.{index}"),
                format!(
                    "Column {} is not bound to the resident Arrow field at the same position.",
                    column.name
                ),
                None,
            ));
        }
    }
    issues
}

fn find_column(dataset: &Dataset, column: &str) -> Option<usize> {
    dataset
        .schema
        .columns
        .iter()
        .position(|metadata| metadata.name == column)
}

fn recode_target_is_compatible(cell: &DatasetCellV2, target_type: ColumnType) -> bool {
    match (cell, target_type) {
        (DatasetCellV2::Missing, _) => true,
        (DatasetCellV2::Number(_), ColumnType::Numeric) => true,
        (DatasetCellV2::Text(_), ColumnType::Text) => true,
        (DatasetCellV2::Number(value), ColumnType::Boolean) => *value == 0.0 || *value == 1.0,
        _ => false,
    }
}

fn validate_spec(
    dataset: &Dataset,
    spec: &DatasetTransformationSpecV2,
) -> Vec<DatasetTransformationIssueV2> {
    let mut issues = validate_dataset(dataset);
    let targets = spec.target_columns();
    for (index, candidate) in targets.iter().enumerate() {
        let target = candidate.trim();
        let field = if targets.len() == 1 {
            "target_column".to_owned()
        } else {
            format!("columns.{index}.target_column")
        };
        if target.is_empty() {
            issues.push(issue(
                "target.required",
                field,
                "Enter a name for the derived variable.",
                None,
            ));
        } else if candidate != target {
            issues.push(issue(
                "target.whitespace",
                field,
                "Derived variable names cannot begin or end with whitespace.",
                None,
            ));
        } else if dataset
            .schema
            .columns
            .iter()
            .any(|column| normalized_column_key(&column.name) == normalized_column_key(target))
        {
            issues.push(issue(
                "target.exists",
                field,
                format!("Column {target} already exists. Choose a new variable name."),
                None,
            ));
        }
    }
    if targets
        .iter()
        .map(|target| normalized_column_key(target.trim()))
        .collect::<HashSet<_>>()
        .len()
        != targets.len()
    {
        issues.push(issue(
            "target.duplicate",
            "output_columns",
            "Each derived variable name must be unique.",
            None,
        ));
    }

    let inputs = spec.input_columns();
    for (index, column) in inputs.iter().enumerate() {
        if column != column.trim() {
            issues.push(issue(
                "source.whitespace",
                format!("input_columns.{index}"),
                "Input variable names cannot begin or end with whitespace.",
                None,
            ));
        }
        if find_column(dataset, column).is_none() {
            issues.push(issue(
                "source.unknown",
                format!("input_columns.{index}"),
                format!("Column {column} is not present in this dataset."),
                None,
            ));
        }
    }
    if inputs
        .iter()
        .map(|column| normalized_column_key(column.trim()))
        .collect::<HashSet<_>>()
        .len()
        != inputs.len()
    {
        issues.push(issue(
            "source.duplicate",
            "input_columns",
            "Choose each input variable only once.",
            None,
        ));
    }

    match spec {
        DatasetTransformationSpecV2::AddColumn {
            value, target_type, ..
        } => {
            if matches!(value, DatasetCellV2::Number(number) if !number.is_finite()) {
                issues.push(issue(
                    "add_column.value_invalid",
                    "value",
                    "The constant value must be finite or missing.",
                    None,
                ));
            } else if !recode_target_is_compatible(value, *target_type) {
                issues.push(issue(
                    "add_column.value_type_mismatch",
                    "value",
                    "The constant value must match the declared column type.",
                    None,
                ));
            }
        }
        DatasetTransformationSpecV2::MissingMarkers { columns } => {
            if columns.is_empty() {
                issues.push(issue(
                    "missing_markers.columns_required",
                    "columns",
                    "Choose at least one column to clean.",
                    None,
                ));
            }
            for (column_index, column) in columns.iter().enumerate() {
                if let Some(source_index) = find_column(dataset, &column.source_column) {
                    let source_metadata = &dataset.schema.columns[source_index];
                    if column.target_type != source_metadata.column_type
                        || column.target_scale != source_metadata.scale_type
                        || column.value_labels != source_metadata.value_labels
                    {
                        issues.push(issue(
                            "missing_markers.metadata_mismatch",
                            format!("columns.{column_index}"),
                            "A cleaned column must preserve its source type, scale, and value labels.",
                            None,
                        ));
                    }
                }
                if column.markers.is_empty() {
                    issues.push(issue(
                        "missing_markers.markers_required",
                        format!("columns.{column_index}.markers"),
                        "Add at least one missing-value marker.",
                        None,
                    ));
                }
                let mut markers = HashSet::new();
                for (marker_index, marker) in column.markers.iter().enumerate() {
                    if !marker.is_finite() {
                        issues.push(issue(
                            "missing_markers.marker_invalid",
                            format!("columns.{column_index}.markers.{marker_index}"),
                            "Missing-value markers must be finite numbers or strings.",
                            None,
                        ));
                    }
                    if !markers.insert(value_key(marker)) {
                        issues.push(issue(
                            "missing_markers.marker_duplicate",
                            format!("columns.{column_index}.markers.{marker_index}"),
                            "Each marker may appear only once per source column.",
                            None,
                        ));
                    }
                }
            }
        }
        DatasetTransformationSpecV2::ReverseScale {
            scale_min,
            scale_max,
            ..
        } => {
            if !scale_min.is_finite() || !scale_max.is_finite() || scale_min >= scale_max {
                issues.push(issue(
                    "reverse_scale.range_invalid",
                    "scale_min",
                    "Enter a finite minimum smaller than the maximum.",
                    None,
                ));
            }
        }
        DatasetTransformationSpecV2::Standardize { .. } => {}
        DatasetTransformationSpecV2::Recode {
            mappings,
            target_type,
            ..
        } => {
            if mappings.is_empty() {
                issues.push(issue(
                    "recode.mappings_required",
                    "mappings",
                    "Add at least one recode mapping.",
                    None,
                ));
            }
            let mut sources = HashSet::new();
            for (index, mapping) in mappings.iter().enumerate() {
                if !mapping.source.is_finite() {
                    issues.push(issue(
                        "recode.source_invalid",
                        format!("mappings.{index}.source"),
                        "Recode source values must be finite numbers or strings.",
                        None,
                    ));
                }
                if !sources.insert(value_key(&mapping.source)) {
                    issues.push(issue(
                        "recode.source_duplicate",
                        format!("mappings.{index}.source"),
                        "Each source value can appear only once.",
                        None,
                    ));
                }
                if matches!(&mapping.target, DatasetCellV2::Number(value) if !value.is_finite()) {
                    issues.push(issue(
                        "recode.target_invalid",
                        format!("mappings.{index}.target"),
                        "Recode targets must be finite values or missing.",
                        None,
                    ));
                } else if !recode_target_is_compatible(&mapping.target, *target_type) {
                    let (code, message) = if *target_type == ColumnType::Boolean {
                        (
                            "recode.boolean_target_invalid",
                            "Binary recode targets must be 0, 1, or missing.",
                        )
                    } else {
                        (
                            "recode.target_type_mismatch",
                            "Each recode target must match the declared target type.",
                        )
                    };
                    issues.push(issue(
                        code,
                        format!("mappings.{index}.target"),
                        message,
                        None,
                    ));
                }
            }
        }
        DatasetTransformationSpecV2::Arithmetic { right, .. } => {
            if let DatasetArithmeticRightV2::Constant { value } = right
                && !value.is_finite()
            {
                issues.push(issue(
                    "arithmetic.constant_invalid",
                    "right.value",
                    "Enter a finite arithmetic constant.",
                    None,
                ));
            }
        }
        DatasetTransformationSpecV2::RowAggregate {
            source_columns,
            missing_policy,
            minimum_non_missing,
            ..
        } => {
            if source_columns.len() < 2 {
                issues.push(issue(
                    "aggregate.sources_required",
                    "source_columns",
                    "Choose at least two variables to combine.",
                    None,
                ));
            }
            let default_minimum = match missing_policy {
                DatasetMissingPolicyV2::Available => 1,
                DatasetMissingPolicyV2::Propagate => source_columns.len(),
            };
            let minimum = minimum_non_missing.unwrap_or(default_minimum);
            if minimum < 1 || minimum > source_columns.len() {
                issues.push(issue(
                    "aggregate.minimum_invalid",
                    "minimum_non_missing",
                    "The minimum complete-variable count must be within the selected variables.",
                    None,
                ));
            }
        }
        DatasetTransformationSpecV2::Dummy { match_value, .. } => {
            if !match_value.is_finite() {
                issues.push(issue(
                    "dummy.match_invalid",
                    "match_value",
                    "The dummy match value must be finite or text.",
                    None,
                ));
            }
        }
        DatasetTransformationSpecV2::Group { rules, .. } => {
            if rules.is_empty() {
                issues.push(issue(
                    "group.rules_required",
                    "rules",
                    "Add at least one group rule.",
                    None,
                ));
            }
            let mut outputs = HashSet::new();
            let mut claimed_values = HashSet::new();
            let mut output_kinds = BTreeSet::new();
            for (index, rule) in rules.iter().enumerate() {
                let output = rule.output();
                if !output.is_finite() {
                    issues.push(issue(
                        "group.output_invalid",
                        format!("rules.{index}.output"),
                        "Group outputs must be finite numbers or strings.",
                        None,
                    ));
                }
                if !outputs.insert(value_key(output)) {
                    issues.push(issue(
                        "group.output_duplicate",
                        format!("rules.{index}.output"),
                        "Each group output must be unique.",
                        None,
                    ));
                }
                output_kinds.insert(match output {
                    NonMissingDatasetCellV2::Number(_) => "number",
                    NonMissingDatasetCellV2::Text(_) => "text",
                });

                match rule {
                    DatasetGroupRuleV2::Values { values, .. } => {
                        if values.is_empty() {
                            issues.push(issue(
                                "group.values_required",
                                format!("rules.{index}.values"),
                                "Add at least one source value for this group.",
                                None,
                            ));
                        }
                        let mut local = HashSet::new();
                        for value in values {
                            if !value.is_finite() {
                                issues.push(issue(
                                    "group.value_invalid",
                                    format!("rules.{index}.values"),
                                    "Group source values must be finite numbers or strings.",
                                    None,
                                ));
                            }
                            let key = value_key(value);
                            if !local.insert(key.clone()) {
                                issues.push(issue(
                                    "group.values_duplicate",
                                    format!("rules.{index}.values"),
                                    "A group rule cannot repeat a source value.",
                                    None,
                                ));
                            }
                            if !claimed_values.insert(key) {
                                issues.push(issue(
                                    "group.value_overlap",
                                    format!("rules.{index}.values"),
                                    format!(
                                        "Source value {} belongs to more than one group.",
                                        cell_display(value)
                                    ),
                                    None,
                                ));
                            }
                        }
                    }
                    DatasetGroupRuleV2::NumericRange {
                        minimum, maximum, ..
                    } => {
                        if minimum.is_none() && maximum.is_none() {
                            issues.push(issue(
                                "group.range_unbounded",
                                format!("rules.{index}"),
                                "A numeric group range needs a minimum or maximum.",
                                None,
                            ));
                        }
                        if minimum.is_some_and(|value| !value.is_finite()) {
                            issues.push(issue(
                                "group.range_invalid",
                                format!("rules.{index}.minimum"),
                                "The group minimum must be finite.",
                                None,
                            ));
                        }
                        if maximum.is_some_and(|value| !value.is_finite()) {
                            issues.push(issue(
                                "group.range_invalid",
                                format!("rules.{index}.maximum"),
                                "The group maximum must be finite.",
                                None,
                            ));
                        }
                        if minimum
                            .zip(*maximum)
                            .is_some_and(|(minimum, maximum)| minimum > maximum)
                        {
                            issues.push(issue(
                                "group.range_invalid",
                                format!("rules.{index}"),
                                "The group minimum cannot exceed its maximum.",
                                None,
                            ));
                        }
                    }
                }
            }
            // Arrow-backed repository datasets require a single physical type
            // for the new column. Fail closed instead of coercing cell values.
            if output_kinds.len() > 1 {
                issues.push(issue(
                    "group.output_type_mixed",
                    "rules",
                    "Use either numeric or text outputs consistently across group rules.",
                    None,
                ));
            }
        }
    }
    issues
}

fn raw_cell(dataset: &Dataset, column_index: usize, row_index: usize) -> Result<DatasetCellV2, ()> {
    let Some(array) = dataset.batch.columns().get(column_index) else {
        return Err(());
    };
    if row_index >= array.len() {
        return Err(());
    }
    if array.is_null(row_index) {
        return Ok(DatasetCellV2::Missing);
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        let value = values.value(row_index);
        return value
            .is_finite()
            .then(|| DatasetCellV2::Number(normalize_zero(value)))
            .ok_or(());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        let value = values.value(row_index);
        return (value.unsigned_abs() <= MAX_SAFE_INTEGER as u64)
            .then_some(DatasetCellV2::Number(value as f64))
            .ok_or(());
    }
    if let Some(values) = array.as_any().downcast_ref::<StringArray>() {
        return Ok(DatasetCellV2::Text(values.value(row_index).to_owned()));
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Ok(DatasetCellV2::Text(values.value(row_index).to_string()));
    }
    Err(())
}

fn source_cell(
    dataset: &Dataset,
    column: &str,
    row_index: usize,
) -> Result<DatasetCellV2, DatasetTransformationIssueV2> {
    let Some(column_index) = find_column(dataset, column) else {
        return Err(issue(
            "source.unknown",
            "source",
            format!("Column {column} is not present in this dataset."),
            Some(row_index),
        ));
    };
    match raw_cell(dataset, column_index, row_index) {
        Ok(DatasetCellV2::Text(value)) if value.is_empty() => Ok(DatasetCellV2::Missing),
        Ok(value) => Ok(value),
        Err(()) => Err(issue(
            "source.invalid_value",
            "source",
            "The source data contains an unsupported value.",
            Some(row_index),
        )),
    }
}

fn numeric_cell(
    dataset: &Dataset,
    column: &str,
    row_index: usize,
) -> Result<Option<f64>, DatasetTransformationIssueV2> {
    match source_cell(dataset, column, row_index)? {
        DatasetCellV2::Missing => Ok(None),
        DatasetCellV2::Number(value) if value.is_finite() => Ok(Some(normalize_zero(value))),
        DatasetCellV2::Text(value) if !value.trim().is_empty() => value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(normalize_zero)
            .map(Some)
            .ok_or_else(|| {
                issue(
                    "source.not_numeric",
                    "source",
                    "This transformation requires numeric input values.",
                    Some(row_index),
                )
            }),
        _ => Err(issue(
            "source.not_numeric",
            "source",
            "This transformation requires numeric input values.",
            Some(row_index),
        )),
    }
}

fn numeric_range_matches(value: f64, rule: &DatasetGroupRuleV2) -> bool {
    let DatasetGroupRuleV2::NumericRange {
        minimum,
        maximum,
        include_minimum,
        include_maximum,
        ..
    } = rule
    else {
        return false;
    };
    let above_minimum = minimum.is_none_or(|minimum| {
        if *include_minimum {
            value >= minimum
        } else {
            value > minimum
        }
    });
    let below_maximum = maximum.is_none_or(|maximum| {
        if *include_maximum {
            value <= maximum
        } else {
            value < maximum
        }
    });
    above_minimum && below_maximum
}

fn validate_evaluated_output(
    spec: &DatasetTransformationSpecV2,
    output: &DatasetCellV2,
    row_index: usize,
) -> Result<(), DatasetTransformationIssueV2> {
    if matches!(output, DatasetCellV2::Number(value) if !value.is_finite()) {
        return Err(issue(
            "transformation.non_finite",
            "spec",
            "This transformation produced a non-finite value.",
            Some(row_index),
        ));
    }
    let declared_target_type = match spec {
        DatasetTransformationSpecV2::Recode { target_type, .. }
        | DatasetTransformationSpecV2::AddColumn { target_type, .. } => Some(*target_type),
        _ => None,
    };
    if declared_target_type
        .is_some_and(|target_type| !recode_target_is_compatible(output, target_type))
    {
        return Err(issue(
            "recode.target_type_mismatch",
            "target_type",
            "The recoded value does not match the declared target type.",
            Some(row_index),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct StandardizationParametersV2 {
    mean: f64,
    standard_deviation: f64,
}

fn standardization_parameters(
    dataset: &Dataset,
    source_column: &str,
    denominator: StandardDeviationDenominatorV2,
) -> Result<StandardizationParametersV2, Vec<DatasetTransformationIssueV2>> {
    let mut count = 0_usize;
    let mut mean = 0.0_f64;
    let mut observed = Vec::new();
    let mut issues = Vec::new();
    for row_index in 0..dataset.batch.num_rows() {
        match numeric_cell(dataset, source_column, row_index) {
            Ok(None) => {}
            Ok(Some(value)) => {
                count += 1;
                let delta = value - mean;
                let next_mean = mean + delta / count as f64;
                if !delta.is_finite() || !next_mean.is_finite() {
                    return Err(vec![issue(
                        "standardize.non_finite",
                        "source_column",
                        "The observed values cannot produce finite standardization statistics.",
                        None,
                    )]);
                }
                mean = normalize_zero(next_mean);
                observed.push(value);
            }
            Err(value_issue) => issues.push(value_issue),
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    if count == 0 {
        return Err(vec![issue(
            "standardize.all_missing",
            "source_column",
            "Standardization needs at least two observed numeric values; this column is entirely missing.",
            None,
        )]);
    }
    if count < 2 {
        return Err(vec![issue(
            "standardize.insufficient_observations",
            "source_column",
            "Standardization needs at least two observed numeric values for sample standard deviation.",
            None,
        )]);
    }
    let mut squared_deviation_sum = 0.0_f64;
    let mut compensation = 0.0_f64;
    for value in observed {
        let deviation = value - mean;
        let squared_deviation = deviation * deviation;
        let adjusted = squared_deviation - compensation;
        let next_sum = squared_deviation_sum + adjusted;
        if !deviation.is_finite()
            || !squared_deviation.is_finite()
            || !adjusted.is_finite()
            || !next_sum.is_finite()
        {
            return Err(vec![issue(
                "standardize.non_finite",
                "source_column",
                "The observed values cannot produce a finite sample sum of squares.",
                None,
            )]);
        }
        compensation = (next_sum - squared_deviation_sum) - adjusted;
        squared_deviation_sum = normalize_zero(next_sum);
    }
    let variance = match denominator {
        StandardDeviationDenominatorV2::SampleNMinusOne => {
            squared_deviation_sum / (count - 1) as f64
        }
    };
    if variance <= 0.0 {
        return Err(vec![issue(
            "standardize.zero_variance",
            "source_column",
            "A zero-variance column cannot be standardized.",
            None,
        )]);
    }
    if !mean.is_finite() || !variance.is_finite() {
        return Err(vec![issue(
            "standardize.non_finite",
            "source_column",
            "The observed values cannot produce finite standardization statistics.",
            None,
        )]);
    }
    let standard_deviation = variance.sqrt();
    if !standard_deviation.is_finite() || standard_deviation == 0.0 {
        return Err(vec![issue(
            "standardize.non_finite",
            "source_column",
            "The observed values cannot produce a finite nonzero sample standard deviation.",
            None,
        )]);
    }
    Ok(StandardizationParametersV2 {
        mean,
        standard_deviation,
    })
}

fn evaluate_row(
    dataset: &Dataset,
    spec: &DatasetTransformationSpecV2,
    row_index: usize,
    standardization: Option<StandardizationParametersV2>,
) -> Result<DatasetCellV2, DatasetTransformationIssueV2> {
    let output = match spec {
        DatasetTransformationSpecV2::AddColumn { value, .. } => value.clone(),
        DatasetTransformationSpecV2::MissingMarkers { .. } => {
            return Err(issue(
                "missing_markers.internal",
                "columns",
                "Multi-column marker cleanup must be evaluated atomically.",
                Some(row_index),
            ));
        }
        DatasetTransformationSpecV2::ReverseScale {
            source_column,
            scale_min,
            scale_max,
            ..
        } => {
            let Some(source) = numeric_cell(dataset, source_column, row_index)? else {
                return Ok(DatasetCellV2::Missing);
            };
            let output = scale_min + scale_max - source;
            if !output.is_finite() {
                return Err(issue(
                    "reverse_scale.non_finite",
                    "scale_min",
                    "This reverse-scale calculation produced a non-finite value.",
                    Some(row_index),
                ));
            }
            DatasetCellV2::Number(normalize_zero(output))
        }
        DatasetTransformationSpecV2::Standardize { source_column, .. } => {
            let parameters = standardization.ok_or_else(|| {
                issue(
                    "standardize.parameters_unavailable",
                    "source_column",
                    "Standardization parameters were not available.",
                    None,
                )
            })?;
            let Some(source) = numeric_cell(dataset, source_column, row_index)? else {
                return Ok(DatasetCellV2::Missing);
            };
            let output = (source - parameters.mean) / parameters.standard_deviation;
            if !output.is_finite() {
                return Err(issue(
                    "standardize.non_finite",
                    "source_column",
                    "This standardization produced a non-finite value.",
                    Some(row_index),
                ));
            }
            DatasetCellV2::Number(normalize_zero(output))
        }
        DatasetTransformationSpecV2::Recode {
            source_column,
            mappings,
            unmapped,
            ..
        } => {
            let source = source_cell(dataset, source_column, row_index)?;
            if source == DatasetCellV2::Missing {
                DatasetCellV2::Missing
            } else if let Some(mapping) = mappings
                .iter()
                .find(|candidate| cell_matches(&source, &candidate.source))
            {
                mapping.target.clone()
            } else {
                match unmapped {
                    RecodeUnmappedV2::Keep => source,
                    RecodeUnmappedV2::Missing => DatasetCellV2::Missing,
                    RecodeUnmappedV2::Error => {
                        let display = match &source {
                            DatasetCellV2::Number(value) => number_key(*value),
                            DatasetCellV2::Text(value) => value.clone(),
                            DatasetCellV2::Missing => String::new(),
                        };
                        return Err(issue(
                            "recode.unmapped",
                            "mappings",
                            format!("No recode mapping exists for {display}."),
                            Some(row_index),
                        ));
                    }
                }
            }
        }
        DatasetTransformationSpecV2::Arithmetic {
            left_column,
            right,
            operator,
            ..
        } => {
            let left = numeric_cell(dataset, left_column, row_index)?;
            let right = match right {
                DatasetArithmeticRightV2::Column { column } => {
                    numeric_cell(dataset, column, row_index)?
                }
                DatasetArithmeticRightV2::Constant { value } => Some(*value),
            };
            let (Some(left), Some(right)) = (left, right) else {
                return Ok(DatasetCellV2::Missing);
            };
            if *operator == ArithmeticOperatorV2::Divide && right == 0.0 {
                return Err(issue(
                    "arithmetic.division_by_zero",
                    "right",
                    "Division by zero is not defined.",
                    Some(row_index),
                ));
            }
            let output = match operator {
                ArithmeticOperatorV2::Add => left + right,
                ArithmeticOperatorV2::Subtract => left - right,
                ArithmeticOperatorV2::Multiply => left * right,
                ArithmeticOperatorV2::Divide => left / right,
            };
            if !output.is_finite() {
                return Err(issue(
                    "arithmetic.non_finite",
                    "operator",
                    "This calculation produced a non-finite value.",
                    Some(row_index),
                ));
            }
            DatasetCellV2::Number(normalize_zero(output))
        }
        DatasetTransformationSpecV2::RowAggregate {
            source_columns,
            operation,
            missing_policy,
            minimum_non_missing,
            ..
        } => {
            let mut complete = Vec::with_capacity(source_columns.len());
            for column in source_columns {
                if let Some(value) = numeric_cell(dataset, column, row_index)? {
                    complete.push(value);
                }
            }
            let default_minimum = match missing_policy {
                DatasetMissingPolicyV2::Available => 1,
                DatasetMissingPolicyV2::Propagate => source_columns.len(),
            };
            let minimum = minimum_non_missing.unwrap_or(default_minimum);
            if (*missing_policy == DatasetMissingPolicyV2::Propagate
                && complete.len() != source_columns.len())
                || complete.len() < minimum
            {
                return Ok(DatasetCellV2::Missing);
            }
            let sum = complete.iter().try_fold(0.0_f64, |total, value| {
                let next = total + value;
                next.is_finite().then_some(next)
            });
            let Some(sum) = sum else {
                return Err(issue(
                    "aggregate.non_finite",
                    "operation",
                    "This row aggregation produced a non-finite value.",
                    Some(row_index),
                ));
            };
            let output = match operation {
                RowAggregateOperationV2::Sum => sum,
                RowAggregateOperationV2::Mean => sum / complete.len() as f64,
            };
            if !output.is_finite() {
                return Err(issue(
                    "aggregate.non_finite",
                    "operation",
                    "This row aggregation produced a non-finite value.",
                    Some(row_index),
                ));
            }
            DatasetCellV2::Number(normalize_zero(output))
        }
        DatasetTransformationSpecV2::Dummy {
            source_column,
            match_value,
            missing_policy,
            ..
        } => {
            let source = source_cell(dataset, source_column, row_index)?;
            if source == DatasetCellV2::Missing {
                match missing_policy {
                    DummyMissingPolicyV2::Missing => DatasetCellV2::Missing,
                    DummyMissingPolicyV2::Zero => DatasetCellV2::Number(0.0),
                }
            } else if cell_matches(&source, match_value) {
                DatasetCellV2::Number(1.0)
            } else {
                DatasetCellV2::Number(0.0)
            }
        }
        DatasetTransformationSpecV2::Group {
            source_column,
            rules,
            unmatched,
            ..
        } => {
            let source = source_cell(dataset, source_column, row_index)?;
            if source == DatasetCellV2::Missing {
                return Ok(DatasetCellV2::Missing);
            }
            let numeric_source = if rules
                .iter()
                .any(|rule| matches!(rule, DatasetGroupRuleV2::NumericRange { .. }))
            {
                numeric_cell(dataset, source_column, row_index)?
            } else {
                None
            };
            let matches = rules
                .iter()
                .filter(|rule| match rule {
                    DatasetGroupRuleV2::Values { values, .. } => {
                        values.iter().any(|value| cell_matches(&source, value))
                    }
                    DatasetGroupRuleV2::NumericRange { .. } => {
                        numeric_source.is_some_and(|value| numeric_range_matches(value, rule))
                    }
                })
                .collect::<Vec<_>>();
            if matches.len() > 1 {
                let display = match &source {
                    DatasetCellV2::Number(value) => number_key(*value),
                    DatasetCellV2::Text(value) => value.clone(),
                    DatasetCellV2::Missing => String::new(),
                };
                return Err(issue(
                    "group.rule_overlap",
                    "rules",
                    format!("Value {display} belongs to more than one group."),
                    Some(row_index),
                ));
            }
            if let Some(rule) = matches.first() {
                rule.output().as_cell()
            } else {
                match unmatched {
                    GroupUnmatchedPolicyV2::Missing => DatasetCellV2::Missing,
                    GroupUnmatchedPolicyV2::Error => {
                        let display = match &source {
                            DatasetCellV2::Number(value) => number_key(*value),
                            DatasetCellV2::Text(value) => value.clone(),
                            DatasetCellV2::Missing => String::new(),
                        };
                        return Err(issue(
                            "group.unmatched",
                            "rules",
                            format!("No group rule includes {display}."),
                            Some(row_index),
                        ));
                    }
                }
            }
        }
    };
    validate_evaluated_output(spec, &output, row_index)?;
    Ok(output)
}

#[derive(Debug)]
struct EvaluatedRows {
    outputs: Vec<Vec<DatasetCellV2>>,
    issues: Vec<DatasetTransformationIssueV2>,
}

fn evaluate_rows(dataset: &Dataset, spec: &DatasetTransformationSpecV2) -> EvaluatedRows {
    let standardization = if let DatasetTransformationSpecV2::Standardize {
        source_column,
        denominator,
        ..
    } = spec
    {
        match standardization_parameters(dataset, source_column, *denominator) {
            Ok(parameters) => Some(parameters),
            Err(issues) => {
                return EvaluatedRows {
                    outputs: vec![vec![DatasetCellV2::Missing]; dataset.batch.num_rows()],
                    issues,
                };
            }
        }
    } else {
        None
    };
    let mut outputs = Vec::with_capacity(dataset.batch.num_rows());
    let mut issues = Vec::new();
    for row_index in 0..dataset.batch.num_rows() {
        let evaluated = match spec {
            DatasetTransformationSpecV2::MissingMarkers { columns } => columns
                .iter()
                .enumerate()
                .map(|(column_index, column)| {
                    let source = source_cell(dataset, &column.source_column, row_index)?;
                    let output = if source == DatasetCellV2::Missing
                        || column
                            .markers
                            .iter()
                            .any(|marker| cell_matches(&source, marker))
                    {
                        DatasetCellV2::Missing
                    } else {
                        source
                    };
                    if !recode_target_is_compatible(&output, column.target_type) {
                        return Err(issue(
                            "output.type_mismatch",
                            format!("columns.{column_index}.target_type"),
                            "A cleaned value does not match the declared target type.",
                            Some(row_index),
                        ));
                    }
                    Ok(output)
                })
                .collect::<Result<Vec<_>, _>>(),
            _ => evaluate_row(dataset, spec, row_index, standardization).map(|output| vec![output]),
        };
        match evaluated {
            Ok(output) => outputs.push(output),
            Err(row_issue) => {
                issues.push(row_issue);
                outputs.push(vec![DatasetCellV2::Missing; spec.target_columns().len()]);
            }
        }
    }
    EvaluatedRows { outputs, issues }
}

pub fn preview_dataset_transformation_v2(
    dataset: &Dataset,
    spec: &DatasetTransformationSpecV2,
) -> DatasetTransformationPreviewV2 {
    preview_dataset_transformation_v2_with_limit(dataset, spec, DEFAULT_PREVIEW_LIMIT)
}

pub fn preview_dataset_transformation_v2_with_limit(
    dataset: &Dataset,
    spec: &DatasetTransformationSpecV2,
    preview_limit: i64,
) -> DatasetTransformationPreviewV2 {
    let mut static_issues = validate_spec(dataset, spec);
    if !(1..=MAX_PREVIEW_LIMIT).contains(&preview_limit) {
        static_issues.push(issue(
            "preview.limit_invalid",
            "preview_limit",
            "Preview between 1 and 100 rows.",
            None,
        ));
    }
    let evaluated = if static_issues.is_empty() {
        evaluate_rows(dataset, spec)
    } else {
        EvaluatedRows {
            outputs: Vec::new(),
            issues: Vec::new(),
        }
    };
    let bounded_limit = if preview_limit > 0 {
        preview_limit.min(MAX_PREVIEW_LIMIT) as usize
    } else {
        DEFAULT_PREVIEW_LIMIT as usize
    };
    let input_columns = spec.input_columns();
    let output_columns = spec.target_columns();
    let rows = evaluated
        .outputs
        .iter()
        .take(bounded_limit)
        .enumerate()
        .map(|(row_index, row_outputs)| {
            let inputs = input_columns
                .iter()
                .map(|column| {
                    let value =
                        source_cell(dataset, column, row_index).unwrap_or(DatasetCellV2::Missing);
                    (column.clone(), value)
                })
                .collect::<BTreeMap<_, _>>();
            DatasetTransformationPreviewRowV2 {
                row_index,
                inputs,
                output: row_outputs
                    .first()
                    .cloned()
                    .unwrap_or(DatasetCellV2::Missing),
                outputs: output_columns
                    .iter()
                    .cloned()
                    .zip(row_outputs.iter().cloned())
                    .collect(),
            }
        })
        .collect();
    let output_missing_count = evaluated
        .outputs
        .iter()
        .flatten()
        .filter(|value| **value == DatasetCellV2::Missing)
        .count();
    static_issues.extend(evaluated.issues);
    DatasetTransformationPreviewV2 {
        schema_version: DATASET_TRANSFORMATION_SCHEMA_V2,
        source_dataset_id: dataset.id.to_string(),
        target_column: spec.target_column().to_owned(),
        output_columns,
        input_columns,
        inspected_rows: dataset.batch.num_rows().min(bounded_limit),
        total_rows: dataset.schema.case_count,
        output_missing_count,
        rows,
        issues: static_issues,
    }
}

fn trimmed_label(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn declared_target_metadata(
    name: String,
    target_type: ColumnType,
    target_scale: ScaleType,
    target_label: Option<&str>,
    value_labels: &BTreeMap<String, String>,
) -> ColumnMetadata {
    ColumnMetadata {
        name,
        label: trimmed_label(target_label),
        column_type: target_type,
        scale_type: target_scale,
        missing_markers: Vec::new(),
        theoretical_min: None,
        theoretical_max: None,
        value_labels: value_labels.clone(),
    }
}

fn target_metadata(spec: &DatasetTransformationSpecV2) -> Vec<ColumnMetadata> {
    let name = spec.target_column().to_owned();
    let label = trimmed_label(spec.target_label());
    match spec {
        DatasetTransformationSpecV2::MissingMarkers { columns } => columns
            .iter()
            .map(|column| {
                declared_target_metadata(
                    column.target_column.clone(),
                    column.target_type,
                    column.target_scale,
                    column.target_label.as_deref(),
                    &column.value_labels,
                )
            })
            .collect(),
        DatasetTransformationSpecV2::AddColumn {
            target_type,
            target_scale,
            value_labels,
            ..
        } => vec![declared_target_metadata(
            name,
            *target_type,
            *target_scale,
            spec.target_label(),
            value_labels,
        )],
        DatasetTransformationSpecV2::Recode {
            target_type,
            target_scale,
            value_labels,
            ..
        } => vec![ColumnMetadata {
            name,
            label,
            column_type: *target_type,
            scale_type: *target_scale,
            missing_markers: Vec::new(),
            theoretical_min: None,
            theoretical_max: None,
            value_labels: value_labels.clone(),
        }],
        DatasetTransformationSpecV2::Dummy { .. } => vec![ColumnMetadata {
            name,
            label,
            column_type: ColumnType::Numeric,
            scale_type: ScaleType::Binary,
            missing_markers: Vec::new(),
            theoretical_min: Some(0.0),
            theoretical_max: Some(1.0),
            value_labels: BTreeMap::from([
                ("0".to_owned(), "No".to_owned()),
                ("1".to_owned(), "Yes".to_owned()),
            ]),
        }],
        DatasetTransformationSpecV2::Group { rules, .. } => {
            let numeric = rules
                .iter()
                .all(|rule| matches!(rule.output(), NonMissingDatasetCellV2::Number(_)));
            let value_labels = rules
                .iter()
                .filter_map(|rule| {
                    trimmed_label(rule.label()).map(|label| (cell_display(rule.output()), label))
                })
                .collect();
            vec![ColumnMetadata {
                name,
                label,
                column_type: if numeric {
                    ColumnType::Numeric
                } else {
                    ColumnType::Text
                },
                scale_type: ScaleType::Nominal,
                missing_markers: Vec::new(),
                theoretical_min: None,
                theoretical_max: None,
                value_labels,
            }]
        }
        DatasetTransformationSpecV2::ReverseScale {
            scale_min,
            scale_max,
            ..
        } => vec![ColumnMetadata {
            name,
            label,
            column_type: ColumnType::Numeric,
            scale_type: ScaleType::Continuous,
            missing_markers: Vec::new(),
            theoretical_min: Some(*scale_min),
            theoretical_max: Some(*scale_max),
            value_labels: BTreeMap::new(),
        }],
        DatasetTransformationSpecV2::Standardize { .. }
        | DatasetTransformationSpecV2::Arithmetic { .. }
        | DatasetTransformationSpecV2::RowAggregate { .. } => vec![ColumnMetadata {
            name,
            label,
            column_type: ColumnType::Numeric,
            scale_type: ScaleType::Continuous,
            missing_markers: Vec::new(),
            theoretical_min: None,
            theoretical_max: None,
            value_labels: BTreeMap::new(),
        }],
    }
}

fn materialize_outputs(
    outputs: &[DatasetCellV2],
    metadata: &ColumnMetadata,
) -> Result<(ArrayRef, DataType), DatasetTransformationErrorV2> {
    match metadata.column_type {
        ColumnType::Numeric | ColumnType::Boolean => {
            let values = outputs
                .iter()
                .enumerate()
                .map(|(row_index, value)| match value {
                    DatasetCellV2::Missing => Ok(None),
                    DatasetCellV2::Number(value) if value.is_finite() => {
                        if metadata.column_type == ColumnType::Boolean
                            && *value != 0.0
                            && *value != 1.0
                        {
                            Err(issue(
                                "recode.boolean_target_invalid",
                                "target_type",
                                "Binary recode targets must be 0, 1, or missing.",
                                Some(row_index),
                            ))
                        } else {
                            Ok(Some(normalize_zero(*value)))
                        }
                    }
                    _ => Err(issue(
                        "output.type_mismatch",
                        "target_type",
                        "A derived numeric column contains a nonnumeric value.",
                        Some(row_index),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(failure)?;
            Ok((Arc::new(Float64Array::from(values)), DataType::Float64))
        }
        ColumnType::Text => {
            let mut builder = StringBuilder::new();
            for (row_index, value) in outputs.iter().enumerate() {
                match value {
                    DatasetCellV2::Missing => builder.append_null(),
                    DatasetCellV2::Text(value) => builder.append_value(value),
                    DatasetCellV2::Number(_) => {
                        return Err(failure(issue(
                            "output.type_mismatch",
                            "target_type",
                            "A derived text column contains a numeric value.",
                            Some(row_index),
                        )));
                    }
                }
            }
            Ok((Arc::new(builder.finish()), DataType::Utf8))
        }
    }
}

fn repository_dataset_fingerprint(
    batch: &RecordBatch,
    schema: &DatasetSchema,
) -> Result<DataFingerprint, DatasetTransformationErrorV2> {
    let schema_bytes = serde_json::to_vec(schema).map_err(|_| {
        failure(issue(
            "fingerprint.schema_invalid",
            "dataset",
            "The dataset schema cannot be fingerprinted.",
            None,
        ))
    })?;
    let arrow_bytes = write_arrow(batch).map_err(|_| {
        failure(issue(
            "fingerprint.data_invalid",
            "dataset",
            "The resident dataset cannot be fingerprinted.",
            None,
        ))
    })?;
    let mut digest = Sha256::new();
    digest.update(b"quickpls-dataset-fingerprint-v2\0");
    digest.update(schema_bytes);
    digest.update(arrow_bytes);
    Ok(DataFingerprint(format!("v2:{:x}", digest.finalize())))
}

fn source_fingerprint(dataset: &Dataset) -> Result<String, DatasetTransformationErrorV2> {
    let existing = dataset.fingerprint.0.trim();
    if !existing.is_empty() {
        return Ok(existing.to_owned());
    }
    Ok(repository_dataset_fingerprint(&dataset.batch, &dataset.schema)?.0)
}

fn validate_apply_options(
    dataset: &Dataset,
    options: &ApplyDatasetTransformationOptionsV2,
) -> Result<(Uuid, DateTime<Utc>), DatasetTransformationErrorV2> {
    let mut issues = Vec::new();
    let output_id = if options.output_dataset_id.trim().is_empty() {
        issues.push(issue(
            "output.id_required",
            "output_dataset_id",
            "Provide a stable identifier for the derived dataset.",
            None,
        ));
        None
    } else {
        match Uuid::parse_str(&options.output_dataset_id) {
            Ok(output_id) => {
                if output_id == dataset.id {
                    issues.push(issue(
                        "output.id_conflict",
                        "output_dataset_id",
                        "The derived dataset must have a new identifier.",
                        None,
                    ));
                }
                Some(output_id)
            }
            Err(_) => {
                issues.push(issue(
                    "output.id_invalid",
                    "output_dataset_id",
                    "The Rust dataset store requires a UUID identifier.",
                    None,
                ));
                None
            }
        }
    };
    if options.output_dataset_name.trim().is_empty() {
        issues.push(issue(
            "output.name_required",
            "output_dataset_name",
            "Name the derived dataset.",
            None,
        ));
    }
    let created_at = DateTime::parse_from_rfc3339(&options.created_at)
        .ok()
        .map(|value| value.with_timezone(&Utc));
    if created_at.is_none() {
        issues.push(issue(
            "output.created_at_invalid",
            "created_at",
            "Use an ISO date-time for transformation lineage.",
            None,
        ));
    }
    if issues.is_empty() {
        Ok((output_id.expect("validated output id"), created_at.unwrap()))
    } else {
        Err(DatasetTransformationErrorV2 { issues })
    }
}

pub fn apply_dataset_transformation_v2(
    dataset: &Dataset,
    spec: &DatasetTransformationSpecV2,
    options: &ApplyDatasetTransformationOptionsV2,
) -> Result<DatasetTransformationMutationV2, DatasetTransformationErrorV2> {
    let static_issues = validate_spec(dataset, spec);
    if !static_issues.is_empty() {
        return Err(DatasetTransformationErrorV2 {
            issues: static_issues,
        });
    }
    let (output_dataset_id, created_at) = validate_apply_options(dataset, options)?;
    let evaluated = evaluate_rows(dataset, spec);
    if !evaluated.issues.is_empty() {
        return Err(DatasetTransformationErrorV2 {
            issues: evaluated.issues,
        });
    }

    let metadata = target_metadata(spec);
    let target_columns = spec.target_columns();
    let mut materialized = Vec::with_capacity(metadata.len());
    for (output_index, target_metadata) in metadata.iter().enumerate() {
        let column_outputs = evaluated
            .outputs
            .iter()
            .map(|row| row[output_index].clone())
            .collect::<Vec<_>>();
        materialized.push(materialize_outputs(&column_outputs, target_metadata)?);
    }
    let mut fields = dataset
        .batch
        .schema()
        .fields()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut arrays = dataset.batch.columns().to_vec();
    for (target, (target_array, target_data_type)) in target_columns.iter().zip(materialized) {
        fields.push(Arc::new(Field::new(target, target_data_type, true)));
        arrays.push(target_array);
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays).map_err(|_| {
        failure(issue(
            "output.materialization_failed",
            "target_column",
            "The derived column could not be materialized.",
            None,
        ))
    })?;
    let mut schema = dataset.schema.clone();
    schema.columns.extend(metadata);
    schema.case_count = batch.num_rows();
    let fingerprint = repository_dataset_fingerprint(&batch, &schema)?;
    let next = Dataset {
        id: output_dataset_id,
        name: options.output_dataset_name.trim().to_owned(),
        schema,
        batch,
        fingerprint,
    };

    let source_fingerprint = source_fingerprint(dataset)?;
    let spec_json = canonical_dataset_transformation_json_v2(spec)?;
    let spec_sha256 = sha256_text(&spec_json);
    let canonical_spec = serde_json::from_str(&spec_json).map_err(|_| {
        failure(issue(
            "canonical.spec_invalid",
            "spec",
            "The canonical transformation specification cannot be restored.",
            None,
        ))
    })?;
    let operation_digest = sha256_text(&format!(
        "{source_fingerprint}\0{spec_sha256}\0{}",
        options.output_dataset_id
    ));
    let output_missing_count = evaluated
        .outputs
        .iter()
        .flatten()
        .filter(|value| matches!(value, DatasetCellV2::Missing))
        .count();
    Ok(DatasetTransformationMutationV2 {
        lineage: DatasetTransformationLineageV2 {
            schema_version: DATASET_TRANSFORMATION_SCHEMA_V2,
            engine: DATASET_TRANSFORMATION_ENGINE_V2.to_owned(),
            operation_id: format!("dataset_transform:{}", &operation_digest[..24]),
            source_dataset_id: dataset.id.to_string(),
            source_dataset_fingerprint: source_fingerprint,
            output_dataset_id: next.id.to_string(),
            output_dataset_fingerprint: next.fingerprint.0.clone(),
            created_at: created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            spec_sha256,
            spec: canonical_spec,
            input_columns: spec.input_columns(),
            output_columns: target_columns,
            source_row_count: dataset.batch.num_rows(),
            output_missing_count,
        },
        dataset: next,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use qpls_data::{
        DatasetDescriptor, ImportOptions, dataset_from_descriptor, import_delimited_bytes,
    };

    fn fixture() -> Dataset {
        let mut dataset = import_delimited_bytes(
            b"item,x,y,segment\n1,2,4,A\n3,,2,B\n5,6,0,A\n",
            "Study",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        dataset.id = "00000000-0000-0000-0000-000000000101".parse().unwrap();
        dataset
    }

    fn options() -> ApplyDatasetTransformationOptionsV2 {
        ApplyDatasetTransformationOptionsV2 {
            output_dataset_id: "00000000-0000-0000-0000-000000000102".to_owned(),
            output_dataset_name: " Study - derived ".to_owned(),
            created_at: "2026-08-14T15:30:00+05:30".to_owned(),
        }
    }

    fn reverse_spec() -> DatasetTransformationSpecV2 {
        DatasetTransformationSpecV2::ReverseScale {
            source_column: "item".to_owned(),
            target_column: "item_r".to_owned(),
            scale_min: 1.0,
            scale_max: 5.0,
            target_label: None,
        }
    }

    fn standardize_spec(source_column: &str, target_column: &str) -> DatasetTransformationSpecV2 {
        DatasetTransformationSpecV2::Standardize {
            source_column: source_column.to_owned(),
            target_column: target_column.to_owned(),
            denominator: StandardDeviationDenominatorV2::SampleNMinusOne,
            target_label: Some(format!("Standardized {source_column}")),
        }
    }

    fn replace_numeric_column(
        mut dataset: Dataset,
        column: &str,
        values: Vec<Option<f64>>,
    ) -> Dataset {
        let index = find_column(&dataset, column).unwrap();
        let mut arrays = dataset.batch.columns().to_vec();
        arrays[index] = Arc::new(Float64Array::from(values));
        dataset.batch = RecordBatch::try_new(dataset.batch.schema(), arrays).unwrap();
        dataset
    }

    fn output_cells(dataset: &Dataset) -> Vec<DatasetCellV2> {
        let target = dataset.batch.num_columns() - 1;
        (0..dataset.batch.num_rows())
            .map(|row| raw_cell(dataset, target, row).unwrap())
            .collect()
    }

    fn has_issue(preview: &DatasetTransformationPreviewV2, code: &str) -> bool {
        preview.issues.iter().any(|issue| issue.code == code)
    }

    #[test]
    fn add_column_is_zero_input_immutable_and_supports_constant_or_missing_values() {
        let source = fixture();
        let source_columns = source.batch.num_columns();
        let source_fingerprint = source.fingerprint.clone();
        let constant = DatasetTransformationSpecV2::AddColumn {
            target_column: "cohort".to_owned(),
            value: DatasetCellV2::Text("pilot".to_owned()),
            target_type: ColumnType::Text,
            target_scale: ScaleType::Nominal,
            target_label: Some("Cohort".to_owned()),
            value_labels: BTreeMap::new(),
        };
        let preview = preview_dataset_transformation_v2(&source, &constant);
        assert!(preview.issues.is_empty());
        assert!(preview.input_columns.is_empty());
        assert_eq!(preview.output_columns, vec!["cohort"]);
        assert_eq!(
            preview.rows[0].output,
            DatasetCellV2::Text("pilot".to_owned())
        );
        assert_eq!(
            preview.rows[0].outputs.get("cohort"),
            Some(&DatasetCellV2::Text("pilot".to_owned()))
        );

        let mutation = apply_dataset_transformation_v2(&source, &constant, &options()).unwrap();
        assert_eq!(source.batch.num_columns(), source_columns);
        assert_eq!(source.fingerprint, source_fingerprint);
        assert_eq!(mutation.dataset.batch.num_columns(), source_columns + 1);
        assert_eq!(
            output_cells(&mutation.dataset),
            vec![DatasetCellV2::Text("pilot".to_owned()); 3]
        );
        assert!(mutation.lineage.input_columns.is_empty());
        assert_eq!(mutation.lineage.output_columns, vec!["cohort"]);
        assert_eq!(mutation.lineage.spec, constant);

        let missing = DatasetTransformationSpecV2::AddColumn {
            target_column: "placeholder".to_owned(),
            value: DatasetCellV2::Missing,
            target_type: ColumnType::Numeric,
            target_scale: ScaleType::Continuous,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        let mut missing_options = options();
        missing_options.output_dataset_id = "00000000-0000-0000-0000-000000000103".to_owned();
        let missing_mutation =
            apply_dataset_transformation_v2(&source, &missing, &missing_options).unwrap();
        assert_eq!(
            output_cells(&missing_mutation.dataset),
            vec![DatasetCellV2::Missing; 3]
        );
        assert_eq!(missing_mutation.lineage.output_missing_count, 3);
    }

    #[test]
    fn missing_markers_append_multiple_clean_columns_in_one_atomic_child() {
        let source = fixture();
        let source_columns = source.batch.num_columns();
        let source_fingerprint = source.fingerprint.clone();
        let spec = DatasetTransformationSpecV2::MissingMarkers {
            columns: vec![
                DatasetMissingMarkerColumnV2 {
                    source_column: "item".to_owned(),
                    target_column: "item_clean".to_owned(),
                    markers: vec![NonMissingDatasetCellV2::Number(3.0)],
                    target_type: ColumnType::Numeric,
                    target_scale: ScaleType::Continuous,
                    target_label: Some("Clean item".to_owned()),
                    value_labels: BTreeMap::new(),
                },
                DatasetMissingMarkerColumnV2 {
                    source_column: "segment".to_owned(),
                    target_column: "segment_clean".to_owned(),
                    markers: vec![NonMissingDatasetCellV2::Text("B".to_owned())],
                    target_type: ColumnType::Text,
                    target_scale: ScaleType::Nominal,
                    target_label: Some("Clean segment".to_owned()),
                    value_labels: BTreeMap::new(),
                },
            ],
        };
        let preview = preview_dataset_transformation_v2(&source, &spec);
        assert!(preview.issues.is_empty());
        assert_eq!(preview.input_columns, vec!["item", "segment"]);
        assert_eq!(preview.output_columns, vec!["item_clean", "segment_clean"]);
        assert_eq!(preview.output_missing_count, 2);
        assert_eq!(
            preview.rows[1].outputs.get("item_clean"),
            Some(&DatasetCellV2::Missing)
        );
        assert_eq!(
            preview.rows[1].outputs.get("segment_clean"),
            Some(&DatasetCellV2::Missing)
        );

        let mutation = apply_dataset_transformation_v2(&source, &spec, &options()).unwrap();
        assert_eq!(source.batch.num_columns(), source_columns);
        assert_eq!(source.fingerprint, source_fingerprint);
        assert_eq!(mutation.dataset.batch.num_columns(), source_columns + 2);
        let item_index = find_column(&mutation.dataset, "item_clean").unwrap();
        let segment_index = find_column(&mutation.dataset, "segment_clean").unwrap();
        assert_eq!(
            (0..3)
                .map(|row| raw_cell(&mutation.dataset, item_index, row).unwrap())
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(1.0),
                DatasetCellV2::Missing,
                DatasetCellV2::Number(5.0)
            ]
        );
        assert_eq!(
            (0..3)
                .map(|row| raw_cell(&mutation.dataset, segment_index, row).unwrap())
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Text("A".to_owned()),
                DatasetCellV2::Missing,
                DatasetCellV2::Text("A".to_owned())
            ]
        );
        assert_eq!(mutation.lineage.spec, spec);
        assert_eq!(mutation.lineage.input_columns, vec!["item", "segment"]);
        assert_eq!(
            mutation.lineage.output_columns,
            vec!["item_clean", "segment_clean"]
        );
        assert_eq!(mutation.lineage.output_missing_count, 2);
    }

    #[test]
    fn add_and_missing_marker_specs_fail_closed_on_ambiguous_or_incompatible_declarations() {
        let bad_add = DatasetTransformationSpecV2::AddColumn {
            target_column: "bad".to_owned(),
            value: DatasetCellV2::Text("text".to_owned()),
            target_type: ColumnType::Numeric,
            target_scale: ScaleType::Continuous,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&fixture(), &bad_add),
            "add_column.value_type_mismatch"
        ));
        let whitespace_target = DatasetTransformationSpecV2::AddColumn {
            target_column: " item ".to_owned(),
            value: DatasetCellV2::Missing,
            target_type: ColumnType::Numeric,
            target_scale: ScaleType::Continuous,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&fixture(), &whitespace_target),
            "target.whitespace"
        ));
        let collision = DatasetTransformationSpecV2::AddColumn {
            target_column: "ITEM".to_owned(),
            value: DatasetCellV2::Missing,
            target_type: ColumnType::Numeric,
            target_scale: ScaleType::Continuous,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&fixture(), &collision),
            "target.exists"
        ));

        let duplicate = DatasetTransformationSpecV2::MissingMarkers {
            columns: vec![
                DatasetMissingMarkerColumnV2 {
                    source_column: "item".to_owned(),
                    target_column: "clean".to_owned(),
                    markers: vec![NonMissingDatasetCellV2::Number(3.0)],
                    target_type: ColumnType::Numeric,
                    target_scale: ScaleType::Continuous,
                    target_label: None,
                    value_labels: BTreeMap::new(),
                },
                DatasetMissingMarkerColumnV2 {
                    source_column: "segment".to_owned(),
                    target_column: "CLEAN".to_owned(),
                    markers: vec![NonMissingDatasetCellV2::Text("B".to_owned())],
                    target_type: ColumnType::Text,
                    target_scale: ScaleType::Nominal,
                    target_label: None,
                    value_labels: BTreeMap::new(),
                },
            ],
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&fixture(), &duplicate),
            "target.duplicate"
        ));
        let metadata_mismatch = DatasetTransformationSpecV2::MissingMarkers {
            columns: vec![DatasetMissingMarkerColumnV2 {
                source_column: "item".to_owned(),
                target_column: "item_clean".to_owned(),
                markers: vec![NonMissingDatasetCellV2::Number(3.0)],
                target_type: ColumnType::Text,
                target_scale: ScaleType::Nominal,
                target_label: None,
                value_labels: BTreeMap::new(),
            }],
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&fixture(), &metadata_mismatch),
            "missing_markers.metadata_mismatch"
        ));
        assert!(has_issue(
            &preview_dataset_transformation_v2(
                &fixture(),
                &DatasetTransformationSpecV2::MissingMarkers {
                    columns: Vec::new()
                }
            ),
            "missing_markers.columns_required"
        ));
    }

    #[test]
    fn reverse_scale_preview_and_execution_share_the_evaluator_and_preserve_source() {
        let source = fixture();
        let source_columns = source.batch.num_columns();
        let source_fingerprint = source.fingerprint.clone();
        let preview = preview_dataset_transformation_v2(&source, &reverse_spec());
        assert!(preview.issues.is_empty());
        assert_eq!(preview.output_missing_count, 0);
        assert_eq!(preview.target_column, "item_r");
        assert_eq!(preview.output_columns, vec!["item_r"]);
        assert_eq!(
            preview
                .rows
                .iter()
                .map(|row| row.output.clone())
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(5.0),
                DatasetCellV2::Number(3.0),
                DatasetCellV2::Number(1.0),
            ]
        );
        assert_eq!(
            preview
                .rows
                .iter()
                .map(|row| row.outputs.get("item_r").cloned())
                .collect::<Vec<_>>(),
            vec![
                Some(DatasetCellV2::Number(5.0)),
                Some(DatasetCellV2::Number(3.0)),
                Some(DatasetCellV2::Number(1.0)),
            ]
        );

        let mutation =
            apply_dataset_transformation_v2(&source, &reverse_spec(), &options()).unwrap();
        assert_eq!(
            output_cells(&mutation.dataset),
            preview
                .rows
                .iter()
                .map(|row| row.output.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(source.batch.num_columns(), source_columns);
        assert_eq!(source.fingerprint, source_fingerprint);
        assert_eq!(mutation.dataset.name, "Study - derived");
        assert!(mutation.dataset.fingerprint.0.starts_with("v2:"));
        assert_eq!(mutation.lineage.input_columns, vec!["item"]);
        assert_eq!(mutation.lineage.output_columns, vec!["item_r"]);
        assert_eq!(mutation.lineage.output_missing_count, 0);
        assert_eq!(mutation.lineage.created_at, "2026-08-14T10:00:00.000Z");
        assert_eq!(
            mutation.lineage.output_dataset_fingerprint,
            mutation.dataset.fingerprint.0
        );

        // The new fingerprint uses the repository's v2 SHA-256 envelope and
        // remains readable by the existing qpls-data descriptor boundary.
        let descriptor = DatasetDescriptor::from(&mutation.dataset);
        let bytes = write_arrow(&mutation.dataset.batch).unwrap();
        let reopened = dataset_from_descriptor(descriptor, &bytes).unwrap();
        assert_eq!(reopened.fingerprint, mutation.dataset.fingerprint);
    }

    #[test]
    fn standardize_uses_sample_n_minus_one_once_and_preserves_missing_source_cells() {
        let source = fixture();
        let source_columns = source.batch.num_columns();
        let source_fingerprint = source.fingerprint.clone();
        let spec = standardize_spec("item", "z_item");
        let preview = preview_dataset_transformation_v2(&source, &spec);
        assert!(preview.issues.is_empty());
        assert_eq!(
            preview
                .rows
                .iter()
                .map(|row| row.output.clone())
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(-1.0),
                DatasetCellV2::Number(0.0),
                DatasetCellV2::Number(1.0),
            ]
        );

        let mutation = apply_dataset_transformation_v2(&source, &spec, &options()).unwrap();
        assert_eq!(
            output_cells(&mutation.dataset),
            vec![
                DatasetCellV2::Number(-1.0),
                DatasetCellV2::Number(0.0),
                DatasetCellV2::Number(1.0),
            ]
        );
        assert_eq!(source.batch.num_columns(), source_columns);
        assert_eq!(source.fingerprint, source_fingerprint);
        assert_eq!(mutation.dataset.batch.num_columns(), source_columns + 1);
        let metadata = mutation.dataset.schema.columns.last().unwrap();
        assert_eq!(metadata.name, "z_item");
        assert_eq!(metadata.column_type, ColumnType::Numeric);
        assert_eq!(metadata.scale_type, ScaleType::Continuous);
        assert_eq!(mutation.lineage.spec, spec);
        assert_eq!(mutation.lineage.input_columns, vec!["item"]);
        assert_eq!(mutation.lineage.output_columns, vec!["z_item"]);
        assert_eq!(mutation.lineage.output_missing_count, 0);

        let missing_preview =
            preview_dataset_transformation_v2(&source, &standardize_spec("x", "z_x"));
        assert!(missing_preview.issues.is_empty());
        assert_eq!(missing_preview.output_missing_count, 1);
        let expected = 1.0_f64 / 2.0_f64.sqrt();
        assert!(
            matches!(missing_preview.rows[0].output, DatasetCellV2::Number(value) if (value + expected).abs() < 1e-12)
        );
        assert_eq!(missing_preview.rows[1].output, DatasetCellV2::Missing);
        assert!(
            matches!(missing_preview.rows[2].output, DatasetCellV2::Number(value) if (value - expected).abs() < 1e-12)
        );

        let tiny = replace_numeric_column(
            fixture(),
            "item",
            vec![Some(1e-150), Some(3e-150), Some(5e-150)],
        );
        let tiny_preview = preview_dataset_transformation_v2(&tiny, &spec);
        assert!(tiny_preview.issues.is_empty());
        let tiny_expected = [-1.0_f64, 0.0, 1.0];
        for (row, expected) in tiny_preview.rows.iter().zip(tiny_expected) {
            assert!(
                matches!(row.output, DatasetCellV2::Number(value) if (value - expected).abs() < 1e-12)
            );
        }

        let canonical = canonical_dataset_transformation_json_v2(&spec).unwrap();
        assert!(canonical.contains(r#""denominator":"sample_n_minus_one""#));
    }

    #[test]
    fn standardize_rejects_all_missing_single_observation_zero_variance_nonnumeric_and_nonfinite() {
        let all_missing = replace_numeric_column(fixture(), "x", vec![None, None, None]);
        assert!(has_issue(
            &preview_dataset_transformation_v2(&all_missing, &standardize_spec("x", "z_x")),
            "standardize.all_missing"
        ));

        let one = replace_numeric_column(fixture(), "x", vec![None, Some(4.0), None]);
        assert!(has_issue(
            &preview_dataset_transformation_v2(&one, &standardize_spec("x", "z_x")),
            "standardize.insufficient_observations"
        ));

        let constant =
            replace_numeric_column(fixture(), "x", vec![Some(4.0), Some(4.0), Some(4.0)]);
        let constant_preview =
            preview_dataset_transformation_v2(&constant, &standardize_spec("x", "z_x"));
        assert!(has_issue(&constant_preview, "standardize.zero_variance"));
        assert!(
            apply_dataset_transformation_v2(&constant, &standardize_spec("x", "z_x"), &options())
                .is_err()
        );

        let nonnumeric = import_delimited_bytes(
            b"x\n1\nnot-a-number\n3\n",
            "Nonnumeric",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let nonnumeric_preview =
            preview_dataset_transformation_v2(&nonnumeric, &standardize_spec("x", "z_x"));
        assert!(
            nonnumeric_preview
                .issues
                .iter()
                .any(|issue| issue.code == "source.not_numeric" && issue.row_index == Some(1))
        );

        let overflow =
            replace_numeric_column(fixture(), "x", vec![Some(1e308), Some(-1e308), Some(0.0)]);
        assert!(has_issue(
            &preview_dataset_transformation_v2(&overflow, &standardize_spec("x", "z_x")),
            "standardize.non_finite"
        ));

        let invalid_denominator = r#"{"kind":"standardize","source_column":"x","target_column":"z_x","denominator":"population_n"}"#;
        assert!(serde_json::from_str::<DatasetTransformationSpecV2>(invalid_denominator).is_err());
        let unknown_field = r#"{"kind":"standardize","source_column":"x","target_column":"z_x","denominator":"sample_n_minus_one","mean":2}"#;
        assert!(serde_json::from_str::<DatasetTransformationSpecV2>(unknown_field).is_err());
    }

    #[test]
    fn exact_recode_dummy_and_group_rules_match_typed_values() {
        let dataset = fixture();
        let recode = DatasetTransformationSpecV2::Recode {
            source_column: "segment".to_owned(),
            target_column: "segment_label".to_owned(),
            mappings: vec![
                DatasetRecodeMappingV2 {
                    source: NonMissingDatasetCellV2::Text("A".to_owned()),
                    target: DatasetCellV2::Text("Treatment".to_owned()),
                },
                DatasetRecodeMappingV2 {
                    source: NonMissingDatasetCellV2::Text("B".to_owned()),
                    target: DatasetCellV2::Text("Control".to_owned()),
                },
            ],
            unmapped: RecodeUnmappedV2::Error,
            target_type: ColumnType::Text,
            target_scale: ScaleType::Nominal,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        let mutation = apply_dataset_transformation_v2(&dataset, &recode, &options()).unwrap();
        assert_eq!(
            output_cells(&mutation.dataset),
            vec![
                DatasetCellV2::Text("Treatment".to_owned()),
                DatasetCellV2::Text("Control".to_owned()),
                DatasetCellV2::Text("Treatment".to_owned()),
            ]
        );

        let dummy = DatasetTransformationSpecV2::Dummy {
            source_column: "segment".to_owned(),
            match_value: NonMissingDatasetCellV2::Text("A".to_owned()),
            missing_policy: DummyMissingPolicyV2::Missing,
            target_column: "is_a".to_owned(),
            target_label: None,
        };
        assert_eq!(
            preview_dataset_transformation_v2(&dataset, &dummy)
                .rows
                .into_iter()
                .map(|row| row.output)
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(1.0),
                DatasetCellV2::Number(0.0),
                DatasetCellV2::Number(1.0),
            ]
        );

        let group = DatasetTransformationSpecV2::Group {
            source_column: "item".to_owned(),
            target_column: "item_group".to_owned(),
            unmatched: GroupUnmatchedPolicyV2::Error,
            target_label: None,
            rules: vec![
                DatasetGroupRuleV2::NumericRange {
                    output: NonMissingDatasetCellV2::Text("low".to_owned()),
                    minimum: Some(1.0),
                    maximum: Some(2.0),
                    include_minimum: true,
                    include_maximum: true,
                    label: Some(" Low ".to_owned()),
                },
                DatasetGroupRuleV2::NumericRange {
                    output: NonMissingDatasetCellV2::Text("high".to_owned()),
                    minimum: Some(2.0),
                    maximum: Some(5.0),
                    include_minimum: false,
                    include_maximum: true,
                    label: Some("High".to_owned()),
                },
            ],
        };
        let grouped = apply_dataset_transformation_v2(&dataset, &group, &options()).unwrap();
        assert_eq!(
            output_cells(&grouped.dataset),
            vec![
                DatasetCellV2::Text("low".to_owned()),
                DatasetCellV2::Text("high".to_owned()),
                DatasetCellV2::Text("high".to_owned()),
            ]
        );
        assert_eq!(
            grouped.dataset.schema.columns.last().unwrap().value_labels,
            BTreeMap::from([
                ("high".to_owned(), "High".to_owned()),
                ("low".to_owned(), "Low".to_owned()),
            ])
        );

        let exact_group = DatasetTransformationSpecV2::Group {
            source_column: "segment".to_owned(),
            target_column: "segment_group".to_owned(),
            unmatched: GroupUnmatchedPolicyV2::Error,
            target_label: None,
            rules: vec![
                DatasetGroupRuleV2::Values {
                    output: NonMissingDatasetCellV2::Number(10.0),
                    values: vec![NonMissingDatasetCellV2::Text("A".to_owned())],
                    label: None,
                },
                DatasetGroupRuleV2::Values {
                    output: NonMissingDatasetCellV2::Number(20.0),
                    values: vec![NonMissingDatasetCellV2::Text("B".to_owned())],
                    label: None,
                },
            ],
        };
        assert_eq!(
            preview_dataset_transformation_v2(&dataset, &exact_group)
                .rows
                .into_iter()
                .map(|row| row.output)
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(10.0),
                DatasetCellV2::Number(20.0),
                DatasetCellV2::Number(10.0),
            ]
        );

        let boolean_recode = DatasetTransformationSpecV2::Recode {
            source_column: "segment".to_owned(),
            target_column: "is_treatment".to_owned(),
            mappings: vec![
                DatasetRecodeMappingV2 {
                    source: NonMissingDatasetCellV2::Text("A".to_owned()),
                    target: DatasetCellV2::Number(1.0),
                },
                DatasetRecodeMappingV2 {
                    source: NonMissingDatasetCellV2::Text("B".to_owned()),
                    target: DatasetCellV2::Number(0.0),
                },
            ],
            unmapped: RecodeUnmappedV2::Error,
            target_type: ColumnType::Boolean,
            target_scale: ScaleType::Binary,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        let boolean =
            apply_dataset_transformation_v2(&dataset, &boolean_recode, &options()).unwrap();
        assert_eq!(
            output_cells(&boolean.dataset),
            vec![
                DatasetCellV2::Number(1.0),
                DatasetCellV2::Number(0.0),
                DatasetCellV2::Number(1.0),
            ]
        );
        assert_eq!(
            boolean.dataset.schema.columns.last().unwrap().column_type,
            ColumnType::Boolean
        );
        assert_eq!(
            boolean
                .dataset
                .batch
                .column(boolean.dataset.batch.num_columns() - 1)
                .data_type(),
            &DataType::Float64
        );
    }

    #[test]
    fn arithmetic_sum_and_available_mean_have_explicit_missing_semantics() {
        let dataset = fixture();
        let arithmetic = DatasetTransformationSpecV2::Arithmetic {
            left_column: "x".to_owned(),
            right: DatasetArithmeticRightV2::Column {
                column: "y".to_owned(),
            },
            operator: ArithmeticOperatorV2::Multiply,
            target_column: "xy".to_owned(),
            target_label: None,
        };
        assert_eq!(
            preview_dataset_transformation_v2(&dataset, &arithmetic)
                .rows
                .into_iter()
                .map(|row| row.output)
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(8.0),
                DatasetCellV2::Missing,
                DatasetCellV2::Number(0.0),
            ]
        );

        let sum = DatasetTransformationSpecV2::RowAggregate {
            source_columns: vec!["x".to_owned(), "y".to_owned()],
            operation: RowAggregateOperationV2::Sum,
            missing_policy: DatasetMissingPolicyV2::Propagate,
            minimum_non_missing: None,
            target_column: "total".to_owned(),
            target_label: None,
        };
        assert_eq!(
            preview_dataset_transformation_v2(&dataset, &sum)
                .rows
                .into_iter()
                .map(|row| row.output)
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(6.0),
                DatasetCellV2::Missing,
                DatasetCellV2::Number(6.0),
            ]
        );

        let mean = DatasetTransformationSpecV2::RowAggregate {
            source_columns: vec!["x".to_owned(), "y".to_owned()],
            operation: RowAggregateOperationV2::Mean,
            missing_policy: DatasetMissingPolicyV2::Available,
            minimum_non_missing: Some(1),
            target_column: "average".to_owned(),
            target_label: None,
        };
        assert_eq!(
            preview_dataset_transformation_v2(&dataset, &mean)
                .rows
                .into_iter()
                .map(|row| row.output)
                .collect::<Vec<_>>(),
            vec![
                DatasetCellV2::Number(3.0),
                DatasetCellV2::Number(2.0),
                DatasetCellV2::Number(3.0),
            ]
        );
    }

    #[test]
    fn static_and_row_failures_are_typed_and_fail_closed() {
        let dataset = fixture();

        let mut matrix = dataset.clone();
        matrix.schema.kind = DataKind::Covariance;
        assert!(has_issue(
            &preview_dataset_transformation_v2(&matrix, &reverse_spec()),
            "dataset.raw_required"
        ));

        let mut nonresident = dataset.clone();
        nonresident.schema.case_count = 10;
        assert!(has_issue(
            &preview_dataset_transformation_v2(&nonresident, &reverse_spec()),
            "dataset.rows_not_resident"
        ));

        let overwrite = DatasetTransformationSpecV2::ReverseScale {
            source_column: "item".to_owned(),
            target_column: "x".to_owned(),
            scale_min: 1.0,
            scale_max: 5.0,
            target_label: None,
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&dataset, &overwrite),
            "target.exists"
        ));

        let nonnumeric = import_delimited_bytes(
            b"x,y\nnot-a-number,1\n2,3\n",
            "Invalid numeric",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let add = DatasetTransformationSpecV2::Arithmetic {
            left_column: "x".to_owned(),
            right: DatasetArithmeticRightV2::Constant { value: 2.0 },
            operator: ArithmeticOperatorV2::Add,
            target_column: "z".to_owned(),
            target_label: None,
        };
        let preview = preview_dataset_transformation_v2(&nonnumeric, &add);
        assert!(
            preview
                .issues
                .iter()
                .any(|issue| issue.code == "source.not_numeric" && issue.row_index == Some(0))
        );

        let divide = DatasetTransformationSpecV2::Arithmetic {
            left_column: "x".to_owned(),
            right: DatasetArithmeticRightV2::Column {
                column: "y".to_owned(),
            },
            operator: ArithmeticOperatorV2::Divide,
            target_column: "ratio".to_owned(),
            target_label: None,
        };
        let preview = preview_dataset_transformation_v2(&dataset, &divide);
        assert!(preview.issues.iter().any(|issue| {
            issue.code == "arithmetic.division_by_zero" && issue.row_index == Some(2)
        }));
        assert!(apply_dataset_transformation_v2(&dataset, &divide, &options()).is_err());

        let overlap = DatasetTransformationSpecV2::Group {
            source_column: "item".to_owned(),
            target_column: "group".to_owned(),
            unmatched: GroupUnmatchedPolicyV2::Missing,
            target_label: None,
            rules: vec![
                DatasetGroupRuleV2::NumericRange {
                    output: NonMissingDatasetCellV2::Number(1.0),
                    minimum: Some(1.0),
                    maximum: Some(4.0),
                    include_minimum: true,
                    include_maximum: true,
                    label: None,
                },
                DatasetGroupRuleV2::NumericRange {
                    output: NonMissingDatasetCellV2::Number(2.0),
                    minimum: Some(3.0),
                    maximum: Some(5.0),
                    include_minimum: true,
                    include_maximum: true,
                    label: None,
                },
            ],
        };
        let preview = preview_dataset_transformation_v2(&dataset, &overlap);
        assert!(
            preview
                .issues
                .iter()
                .any(|issue| issue.code == "group.rule_overlap" && issue.row_index == Some(1))
        );
    }

    #[test]
    fn nonfinite_results_and_arrow_incompatible_outputs_are_not_coerced() {
        let huge =
            import_delimited_bytes(b"x,y\n1e308,1\n", "Huge", b',', &ImportOptions::default())
                .unwrap();
        let overflow = DatasetTransformationSpecV2::Arithmetic {
            left_column: "x".to_owned(),
            right: DatasetArithmeticRightV2::Constant { value: 1e308 },
            operator: ArithmeticOperatorV2::Multiply,
            target_column: "overflow".to_owned(),
            target_label: None,
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&huge, &overflow),
            "arithmetic.non_finite"
        ));

        let mixed_group = DatasetTransformationSpecV2::Group {
            source_column: "segment".to_owned(),
            target_column: "group".to_owned(),
            unmatched: GroupUnmatchedPolicyV2::Missing,
            target_label: None,
            rules: vec![
                DatasetGroupRuleV2::Values {
                    output: NonMissingDatasetCellV2::Number(1.0),
                    values: vec![NonMissingDatasetCellV2::Text("A".to_owned())],
                    label: None,
                },
                DatasetGroupRuleV2::Values {
                    output: NonMissingDatasetCellV2::Text("two".to_owned()),
                    values: vec![NonMissingDatasetCellV2::Text("B".to_owned())],
                    label: None,
                },
            ],
        };
        assert!(has_issue(
            &preview_dataset_transformation_v2(&fixture(), &mixed_group),
            "group.output_type_mixed"
        ));

        let keep_incompatible = DatasetTransformationSpecV2::Recode {
            source_column: "segment".to_owned(),
            target_column: "numeric_segment".to_owned(),
            mappings: vec![DatasetRecodeMappingV2 {
                source: NonMissingDatasetCellV2::Text("A".to_owned()),
                target: DatasetCellV2::Number(1.0),
            }],
            unmapped: RecodeUnmappedV2::Keep,
            target_type: ColumnType::Numeric,
            target_scale: ScaleType::Nominal,
            target_label: None,
            value_labels: BTreeMap::new(),
        };
        let preview = preview_dataset_transformation_v2(&fixture(), &keep_incompatible);
        assert!(preview.issues.iter().any(|issue| {
            issue.code == "recode.target_type_mismatch" && issue.row_index == Some(1)
        }));
    }

    #[test]
    fn hashes_and_operation_identity_are_deterministic_and_scientifically_sensitive() {
        let dataset = fixture();
        let first = apply_dataset_transformation_v2(&dataset, &reverse_spec(), &options()).unwrap();
        let repeat =
            apply_dataset_transformation_v2(&dataset, &reverse_spec(), &options()).unwrap();
        assert_eq!(first.lineage, repeat.lineage);
        assert_eq!(first.dataset.fingerprint, repeat.dataset.fingerprint);
        assert_eq!(first.lineage.spec_sha256.len(), 64);
        assert!(
            first
                .lineage
                .spec_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        );

        let changed_spec = DatasetTransformationSpecV2::ReverseScale {
            source_column: "item".to_owned(),
            target_column: "item_r".to_owned(),
            scale_min: 1.0,
            scale_max: 7.0,
            target_label: None,
        };
        let changed = apply_dataset_transformation_v2(&dataset, &changed_spec, &options()).unwrap();
        assert_ne!(changed.lineage.spec_sha256, first.lineage.spec_sha256);
        assert_ne!(changed.lineage.operation_id, first.lineage.operation_id);
        assert_ne!(changed.dataset.fingerprint, first.dataset.fingerprint);

        let json = serde_json::to_string(&reverse_spec()).unwrap();
        let reordered: DatasetTransformationSpecV2 = serde_json::from_str(
            r#"{"target_column":"item_r","scale_max":5,"kind":"reverse_scale","scale_min":1,"source_column":"item"}"#,
        )
        .unwrap();
        assert_eq!(
            canonical_dataset_transformation_json_v2(&reverse_spec()).unwrap(),
            canonical_dataset_transformation_json_v2(&reordered).unwrap()
        );
        assert!(!json.is_empty());
    }

    #[test]
    fn strict_wire_contract_rejects_unknown_fields_boolean_cells_and_bad_options() {
        let unknown_root = r#"{
            "kind":"reverse_scale","source_column":"item","target_column":"item_r",
            "scale_min":1,"scale_max":5,"unexpected":true
        }"#;
        assert!(serde_json::from_str::<DatasetTransformationSpecV2>(unknown_root).is_err());

        let unknown_mapping = r#"{
            "kind":"recode","source_column":"segment","target_column":"label",
            "mappings":[{"source":"A","target":"B","extra":1}],
            "unmapped":"error","target_type":"text","target_scale":"nominal"
        }"#;
        assert!(serde_json::from_str::<DatasetTransformationSpecV2>(unknown_mapping).is_err());
        assert!(serde_json::from_str::<DatasetCellV2>("true").is_err());
        assert!(serde_json::from_str::<NonMissingDatasetCellV2>("null").is_err());

        let mut bad_options = options();
        bad_options.output_dataset_id = "not-a-uuid".to_owned();
        bad_options.output_dataset_name = " ".to_owned();
        bad_options.created_at = "not-a-date".to_owned();
        let error =
            apply_dataset_transformation_v2(&fixture(), &reverse_spec(), &bad_options).unwrap_err();
        assert_eq!(
            error
                .issues
                .iter()
                .map(|issue| issue.code.as_str())
                .collect::<Vec<_>>(),
            vec![
                "output.id_invalid",
                "output.name_required",
                "output.created_at_invalid"
            ]
        );
    }

    #[test]
    fn preview_bounds_and_normalized_duplicate_columns_fail_closed() {
        let dataset = fixture();
        let invalid = preview_dataset_transformation_v2_with_limit(&dataset, &reverse_spec(), 101);
        assert!(has_issue(&invalid, "preview.limit_invalid"));
        assert!(invalid.rows.is_empty());
        assert_eq!(invalid.inspected_rows, 3);

        let mut duplicate = dataset.clone();
        duplicate.schema.columns[1].name = "ITEM".to_owned();
        let preview = preview_dataset_transformation_v2(&duplicate, &reverse_spec());
        assert!(has_issue(&preview, "dataset.column_duplicate"));
        assert!(has_issue(&preview, "dataset.column_binding_mismatch"));
    }
}
