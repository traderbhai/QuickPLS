use arrow::{
    array::{Array, ArrayRef, BooleanArray, Float64Array, Int64Array, StringArray, StringBuilder},
    datatypes::{DataType, Field, Schema},
    ipc::{reader::StreamReader, writer::StreamWriter},
    record_batch::RecordBatch,
};
use calamine::{Reader, open_workbook_auto};
use faer::{Mat, Side};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path, sync::Arc};
use thiserror::Error;
use uuid::Uuid;

pub const DATA_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScaleType {
    Continuous,
    Ordinal,
    Nominal,
    Binary,
    Identifier,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Numeric,
    Text,
    Boolean,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataKind {
    Raw,
    Covariance,
    Correlation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ColumnMetadata {
    pub name: String,
    pub label: Option<String>,
    pub column_type: ColumnType,
    pub scale_type: ScaleType,
    pub missing_markers: Vec<String>,
    pub theoretical_min: Option<f64>,
    pub theoretical_max: Option<f64>,
    pub value_labels: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetSchema {
    pub version: u32,
    pub kind: DataKind,
    pub columns: Vec<ColumnMetadata>,
    pub case_count: usize,
    #[serde(default)]
    pub sample_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataFingerprint(pub String);

#[derive(Debug, Clone)]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    pub schema: DatasetSchema,
    pub batch: RecordBatch,
    pub fingerprint: DataFingerprint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetDescriptor {
    pub id: Uuid,
    pub name: String,
    pub schema: DatasetSchema,
    pub fingerprint: DataFingerprint,
}

impl From<&Dataset> for DatasetDescriptor {
    fn from(value: &Dataset) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            schema: value.schema.clone(),
            fingerprint: value.fingerprint.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOptions {
    pub delimiter: Option<u8>,
    pub sheet_name: Option<String>,
    pub missing_markers: Vec<String>,
    pub data_kind: DataKind,
    pub sample_size: Option<usize>,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            delimiter: None,
            sheet_name: None,
            missing_markers: vec!["".into(), "NA".into(), "N/A".into(), ".".into()],
            data_kind: DataKind::Raw,
            sample_size: None,
        }
    }
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("unsupported data format: {0}")]
    UnsupportedFormat(String),
    #[error("dataset has no rows or columns")]
    Empty,
    #[error("duplicate or empty column name: {0}")]
    InvalidColumnName(String),
    #[error("spreadsheet does not contain a readable worksheet")]
    MissingWorksheet,
    #[error("data import failed: {0}")]
    Import(String),
    #[error("invalid covariance/correlation matrix: {0}")]
    InvalidMatrix(String),
    #[error("Arrow data failed: {0}")]
    Arrow(String),
    #[error("I/O failed: {0}")]
    Io(#[from] std::io::Error),
}

pub fn import_path(path: &Path, options: &ImportOptions) -> Result<Dataset, DataError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "csv" | "txt" | "tsv" => import_delimited(path, options),
        "xls" | "xlsx" | "xlsb" | "ods" => import_spreadsheet(path, options),
        "sav" | "zsav" => import_sav(path, options),
        _ => Err(DataError::UnsupportedFormat(extension)),
    }
}

pub fn import_delimited(path: &Path, options: &ImportOptions) -> Result<Dataset, DataError> {
    let bytes = fs::read(path)?;
    let delimiter = options.delimiter.unwrap_or_else(|| {
        if path
            .extension()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.eq_ignore_ascii_case("tsv"))
        {
            b'\t'
        } else {
            b','
        }
    });
    import_delimited_bytes(
        &bytes,
        path.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("dataset"),
        delimiter,
        options,
    )
}

pub fn import_delimited_bytes(
    bytes: &[u8],
    name: &str,
    delimiter: u8,
    options: &ImportOptions,
) -> Result<Dataset, DataError> {
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .flexible(false)
        .from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|e| DataError::Import(e.to_string()))?
        .iter()
        .map(str::trim)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let rows = reader
        .records()
        .map(|row| row.map(|record| record.iter().map(str::to_owned).collect::<Vec<_>>()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DataError::Import(e.to_string()))?;
    let (headers, rows) = normalize_matrix_labels(headers, rows, options.data_kind)?;
    validate_headers(&headers)?;
    dataset_from_cells(name, headers, rows, options)
}

fn import_spreadsheet(path: &Path, options: &ImportOptions) -> Result<Dataset, DataError> {
    let mut workbook = open_workbook_auto(path).map_err(|e| DataError::Import(e.to_string()))?;
    let selected = options
        .sheet_name
        .clone()
        .or_else(|| workbook.sheet_names().first().cloned())
        .ok_or(DataError::MissingWorksheet)?;
    let range = workbook
        .worksheet_range(&selected)
        .map_err(|e| DataError::Import(e.to_string()))?;
    let mut rows = range.rows();
    let headers = rows
        .next()
        .ok_or(DataError::Empty)?
        .iter()
        .map(ToString::to_string)
        .map(|v| v.trim().to_owned())
        .collect::<Vec<_>>();
    let body = rows
        .map(|row| row.iter().map(ToString::to_string).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let (headers, body) = normalize_matrix_labels(headers, body, options.data_kind)?;
    validate_headers(&headers)?;
    dataset_from_cells(
        path.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("dataset"),
        headers,
        body,
        options,
    )
}

fn import_sav(path: &Path, options: &ImportOptions) -> Result<Dataset, DataError> {
    let (batch, metadata) = ambers::read_sav(path).map_err(|e| DataError::Import(e.to_string()))?;
    let columns = batch
        .schema()
        .fields()
        .iter()
        .map(|field| {
            let column_type = arrow_column_type(field.data_type());
            let scale_type = metadata
                .measure(field.name())
                .map(|m| format!("{m:?}"))
                .map(|m| match m.as_str() {
                    "Scale" => ScaleType::Continuous,
                    "Ordinal" => ScaleType::Ordinal,
                    "Nominal" => ScaleType::Nominal,
                    _ => default_scale(column_type),
                })
                .unwrap_or_else(|| default_scale(column_type));
            ColumnMetadata {
                name: field.name().clone(),
                label: metadata.label(field.name()).map(str::to_owned),
                column_type,
                scale_type,
                missing_markers: options.missing_markers.clone(),
                theoretical_min: None,
                theoretical_max: None,
                value_labels: BTreeMap::new(),
            }
        })
        .collect();
    finish_dataset(
        path.file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("dataset"),
        batch,
        DatasetSchema {
            version: DATA_SCHEMA_VERSION,
            kind: options.data_kind,
            columns,
            case_count: metadata.number_rows.unwrap_or(0).max(0) as usize,
            sample_size: options.sample_size,
        },
    )
}

fn dataset_from_cells(
    name: &str,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    options: &ImportOptions,
) -> Result<Dataset, DataError> {
    if headers.is_empty() || rows.is_empty() {
        return Err(DataError::Empty);
    }
    if rows.iter().any(|row| row.len() != headers.len()) {
        return Err(DataError::Import(
            "row length does not match header length".into(),
        ));
    }
    let missing = |value: &str| {
        options
            .missing_markers
            .iter()
            .any(|marker| marker == value.trim())
    };
    let mut fields = Vec::with_capacity(headers.len());
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(headers.len());
    let mut columns = Vec::with_capacity(headers.len());
    for (index, header) in headers.iter().enumerate() {
        let numeric = rows
            .iter()
            .all(|row| missing(&row[index]) || row[index].trim().parse::<f64>().is_ok());
        if numeric {
            let values = rows
                .iter()
                .map(|row| {
                    if missing(&row[index]) {
                        None
                    } else {
                        row[index].trim().parse().ok()
                    }
                })
                .collect::<Vec<Option<f64>>>();
            fields.push(Field::new(header, DataType::Float64, true));
            arrays.push(Arc::new(Float64Array::from(values)));
            columns.push(column_meta(
                header,
                ColumnType::Numeric,
                ScaleType::Continuous,
                options,
            ));
        } else {
            let mut builder = StringBuilder::new();
            for row in &rows {
                if missing(&row[index]) {
                    builder.append_null();
                } else {
                    builder.append_value(row[index].trim());
                }
            }
            fields.push(Field::new(header, DataType::Utf8, true));
            arrays.push(Arc::new(builder.finish()));
            columns.push(column_meta(
                header,
                ColumnType::Text,
                ScaleType::Nominal,
                options,
            ));
        }
    }
    let batch = RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|e| DataError::Arrow(e.to_string()))?;
    finish_dataset(
        name,
        batch,
        DatasetSchema {
            version: DATA_SCHEMA_VERSION,
            kind: options.data_kind,
            columns,
            case_count: rows.len(),
            sample_size: options.sample_size,
        },
    )
}

fn finish_dataset(
    name: &str,
    batch: RecordBatch,
    mut schema: DatasetSchema,
) -> Result<Dataset, DataError> {
    schema.case_count = batch.num_rows();
    validate_matrix(&batch, schema.kind, schema.sample_size)?;
    let fingerprint = fingerprint_v2(&batch, &schema)?;
    Ok(Dataset {
        id: Uuid::new_v4(),
        name: name.into(),
        schema,
        batch,
        fingerprint,
    })
}

pub fn update_column_metadata(
    dataset: &mut Dataset,
    column_name: &str,
    metadata: ColumnMetadata,
) -> Result<(), DataError> {
    if metadata.name != column_name {
        return Err(DataError::Import(
            "column names cannot be changed by a metadata update".into(),
        ));
    }
    let column = dataset
        .schema
        .columns
        .iter_mut()
        .find(|column| column.name == column_name)
        .ok_or_else(|| DataError::Import(format!("unknown column {column_name}")))?;
    if metadata
        .theoretical_min
        .zip(metadata.theoretical_max)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err(DataError::Import(
            "theoretical minimum cannot exceed maximum".into(),
        ));
    }
    *column = metadata;
    dataset.fingerprint = fingerprint_v2(&dataset.batch, &dataset.schema)?;
    Ok(())
}

fn validate_matrix(
    batch: &RecordBatch,
    kind: DataKind,
    sample_size: Option<usize>,
) -> Result<(), DataError> {
    if kind == DataKind::Raw {
        return Ok(());
    }
    if sample_size.is_none_or(|sample_size| sample_size < 2) {
        return Err(DataError::InvalidMatrix(
            "covariance/correlation input requires a sample size of at least 2".into(),
        ));
    }
    if batch.num_rows() != batch.num_columns() {
        return Err(DataError::InvalidMatrix(format!(
            "expected a square matrix but found {} rows and {} columns",
            batch.num_rows(),
            batch.num_columns()
        )));
    }
    let columns = batch
        .columns()
        .iter()
        .map(|array| {
            let values = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or_else(|| {
                    DataError::InvalidMatrix("every matrix cell must be numeric".into())
                })?;
            if values.null_count() > 0 {
                return Err(DataError::InvalidMatrix(
                    "matrix cells cannot be missing".into(),
                ));
            }
            Ok(values)
        })
        .collect::<Result<Vec<_>, DataError>>()?;
    for row in 0..batch.num_rows() {
        for column in 0..batch.num_columns() {
            let value = columns[column].value(row);
            if !value.is_finite() {
                return Err(DataError::InvalidMatrix(
                    "matrix cells must be finite".into(),
                ));
            }
            if (value - columns[row].value(column)).abs() > 1e-10 {
                return Err(DataError::InvalidMatrix(format!(
                    "matrix is not symmetric at row {}, column {}",
                    row + 1,
                    column + 1
                )));
            }
            if kind == DataKind::Correlation && !(-1.0..=1.0).contains(&value) {
                return Err(DataError::InvalidMatrix(format!(
                    "correlation {value} is outside [-1, 1]"
                )));
            }
        }
        let diagonal = columns[row].value(row);
        if kind == DataKind::Correlation && (diagonal - 1.0).abs() > 1e-10 {
            return Err(DataError::InvalidMatrix(format!(
                "correlation diagonal {} is not 1",
                row + 1
            )));
        }
        if kind == DataKind::Covariance && diagonal < 0.0 {
            return Err(DataError::InvalidMatrix(format!(
                "covariance diagonal {} is negative",
                row + 1
            )));
        }
    }
    let matrix = Mat::from_fn(batch.num_rows(), batch.num_columns(), |row, column| {
        columns[column].value(row)
    });
    let eigenvalues = matrix
        .self_adjoint_eigenvalues(Side::Lower)
        .map_err(|error| {
            DataError::InvalidMatrix(format!("eigendecomposition failed: {error:?}"))
        })?;
    let scale = eigenvalues
        .iter()
        .map(|value| value.abs())
        .fold(0.0, f64::max)
        .max(1.0);
    let tolerance = scale * batch.num_rows() as f64 * f64::EPSILON * 100.0;
    if eigenvalues.iter().any(|value| *value < -tolerance) {
        return Err(DataError::InvalidMatrix(format!(
            "matrix is not positive semidefinite (minimum eigenvalue {})",
            eigenvalues[0]
        )));
    }
    Ok(())
}

pub fn write_arrow(batch: &RecordBatch) -> Result<Vec<u8>, DataError> {
    let mut output = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut output, &batch.schema())
            .map_err(|e| DataError::Arrow(e.to_string()))?;
        writer
            .write(batch)
            .map_err(|e| DataError::Arrow(e.to_string()))?;
        writer
            .finish()
            .map_err(|e| DataError::Arrow(e.to_string()))?;
    }
    Ok(output)
}

pub fn read_arrow(bytes: &[u8]) -> Result<RecordBatch, DataError> {
    let mut reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|e| DataError::Arrow(e.to_string()))?;
    reader
        .next()
        .ok_or(DataError::Empty)?
        .map_err(|e| DataError::Arrow(e.to_string()))
}

pub fn dataset_from_descriptor(
    descriptor: DatasetDescriptor,
    arrow_bytes: &[u8],
) -> Result<Dataset, DataError> {
    let batch = read_arrow(arrow_bytes)?;
    let valid_fingerprint = if descriptor.fingerprint.0.starts_with("v2:") {
        fingerprint_v2(&batch, &descriptor.schema)?.0 == descriptor.fingerprint.0
    } else {
        sha256(arrow_bytes) == descriptor.fingerprint.0
    };
    if !valid_fingerprint {
        return Err(DataError::Arrow("dataset fingerprint mismatch".into()));
    }
    Ok(Dataset {
        id: descriptor.id,
        name: descriptor.name,
        schema: descriptor.schema,
        batch,
        fingerprint: descriptor.fingerprint,
    })
}

pub fn preview(dataset: &Dataset, limit: usize) -> Vec<BTreeMap<String, Option<String>>> {
    (0..dataset.batch.num_rows().min(limit))
        .map(|row| {
            dataset
                .batch
                .columns()
                .iter()
                .enumerate()
                .map(|(column, array)| {
                    (
                        dataset.schema.columns[column].name.clone(),
                        value_to_string(array, row),
                    )
                })
                .collect()
        })
        .collect()
}

fn value_to_string(array: &ArrayRef, row: usize) -> Option<String> {
    if array.is_null(row) {
        return None;
    }
    if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
        return Some(values.value(row).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<Int64Array>() {
        return Some(values.value(row).to_string());
    }
    if let Some(values) = array.as_any().downcast_ref::<BooleanArray>() {
        return Some(values.value(row).to_string());
    }
    array
        .as_any()
        .downcast_ref::<StringArray>()
        .map(|values| values.value(row).to_owned())
}

fn validate_headers(headers: &[String]) -> Result<(), DataError> {
    let mut seen = std::collections::HashSet::new();
    for header in headers {
        if header.is_empty() || !seen.insert(header) {
            return Err(DataError::InvalidColumnName(header.clone()));
        }
    }
    Ok(())
}
fn normalize_matrix_labels(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    kind: DataKind,
) -> Result<(Vec<String>, Vec<Vec<String>>), DataError> {
    if kind == DataKind::Raw || headers.len() != rows.len() + 1 {
        return Ok((headers, rows));
    }
    let column_names = headers[1..].to_vec();
    let mut values = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        if row.len() != headers.len() {
            return Err(DataError::InvalidMatrix(
                "matrix row length does not match its header".into(),
            ));
        }
        if row[0].trim() != column_names[index].trim() {
            return Err(DataError::InvalidMatrix(format!(
                "matrix row label {} does not match column label {}",
                row[0], column_names[index]
            )));
        }
        values.push(row[1..].to_vec());
    }
    Ok((column_names, values))
}

fn fingerprint_v2(
    batch: &RecordBatch,
    schema: &DatasetSchema,
) -> Result<DataFingerprint, DataError> {
    let mut digest = Sha256::new();
    digest.update(b"quickpls-dataset-fingerprint-v2\0");
    digest.update(serde_json::to_vec(schema).map_err(|error| DataError::Arrow(error.to_string()))?);
    digest.update(write_arrow(batch)?);
    Ok(DataFingerprint(format!("v2:{:x}", digest.finalize())))
}
fn column_meta(
    name: &str,
    column_type: ColumnType,
    scale_type: ScaleType,
    options: &ImportOptions,
) -> ColumnMetadata {
    ColumnMetadata {
        name: name.into(),
        label: None,
        column_type,
        scale_type,
        missing_markers: options.missing_markers.clone(),
        theoretical_min: None,
        theoretical_max: None,
        value_labels: BTreeMap::new(),
    }
}
fn arrow_column_type(data_type: &DataType) -> ColumnType {
    match data_type {
        DataType::Boolean => ColumnType::Boolean,
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View => ColumnType::Text,
        _ => ColumnType::Numeric,
    }
}
fn default_scale(column_type: ColumnType) -> ScaleType {
    match column_type {
        ColumnType::Numeric => ScaleType::Continuous,
        ColumnType::Text => ScaleType::Nominal,
        ColumnType::Boolean => ScaleType::Binary,
    }
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_import_infers_types_missing_values_and_round_trips_arrow() {
        let data = import_delimited_bytes(
            b"x,y,group\n1,2,A\n3,NA,B\n",
            "fixture.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        assert_eq!(data.schema.case_count, 2);
        assert_eq!(data.schema.columns[0].column_type, ColumnType::Numeric);
        assert_eq!(data.schema.columns[2].column_type, ColumnType::Text);
        let bytes = write_arrow(&data.batch).unwrap();
        let restored = read_arrow(&bytes).unwrap();
        assert_eq!(restored, data.batch);
    }
    #[test]
    fn duplicate_headers_are_rejected() {
        assert!(matches!(
            import_delimited_bytes(b"x,x\n1,2\n", "bad.csv", b',', &ImportOptions::default()),
            Err(DataError::InvalidColumnName(_))
        ));
    }
    #[test]
    fn correlation_matrix_requires_square_symmetric_unit_diagonal_data() {
        let options = ImportOptions {
            data_kind: DataKind::Correlation,
            sample_size: Some(200),
            ..ImportOptions::default()
        };
        let matrix =
            import_delimited_bytes(b"x,y\n1,.4\n.4,1\n", "cor.csv", b',', &options).unwrap();
        assert_eq!(matrix.schema.sample_size, Some(200));
        assert!(matches!(
            import_delimited_bytes(b"x,y\n1,.4\n.3,1\n", "bad.csv", b',', &options),
            Err(DataError::InvalidMatrix(_))
        ));
        assert!(matches!(
            import_delimited_bytes(
                b"x,y,z\n1,.9,.9\n.9,1,-.9\n.9,-.9,1\n",
                "not-psd.csv",
                b',',
                &options
            ),
            Err(DataError::InvalidMatrix(_))
        ));
        let labelled =
            import_delimited_bytes(b",x,y\nx,1,.4\ny,.4,1\n", "labelled.csv", b',', &options)
                .unwrap();
        assert_eq!(
            labelled
                .schema
                .columns
                .iter()
                .map(|column| column.name.as_str())
                .collect::<Vec<_>>(),
            vec!["x", "y"]
        );
        let no_sample_size = ImportOptions {
            data_kind: DataKind::Correlation,
            ..ImportOptions::default()
        };
        assert!(matches!(
            import_delimited_bytes(b"x,y\n1,.4\n.4,1\n", "missing-n.csv", b',', &no_sample_size),
            Err(DataError::InvalidMatrix(_))
        ));
    }
    #[test]
    fn column_metadata_updates_are_validated() {
        let mut data =
            import_delimited_bytes(b"x\n1\n2\n", "data.csv", b',', &ImportOptions::default())
                .unwrap();
        let mut metadata = data.schema.columns[0].clone();
        metadata.scale_type = ScaleType::Ordinal;
        metadata.theoretical_min = Some(1.0);
        metadata.theoretical_max = Some(7.0);
        let previous_fingerprint = data.fingerprint.clone();
        update_column_metadata(&mut data, "x", metadata).unwrap();
        assert_eq!(data.schema.columns[0].scale_type, ScaleType::Ordinal);
        assert_ne!(data.fingerprint, previous_fingerprint);
        assert!(data.fingerprint.0.starts_with("v2:"));
    }
    #[test]
    fn xlsx_fixture_imports_numeric_and_text_columns() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.xlsx");
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.write_string(0, 0, "score").unwrap();
        sheet.write_string(0, 1, "group").unwrap();
        sheet.write_number(1, 0, 5.0).unwrap();
        sheet.write_string(1, 1, "A").unwrap();
        sheet.write_number(2, 0, 7.0).unwrap();
        sheet.write_string(2, 1, "B").unwrap();
        workbook.save(&path).unwrap();
        let dataset = import_path(&path, &ImportOptions::default()).unwrap();
        assert_eq!(dataset.schema.case_count, 2);
        assert_eq!(dataset.schema.columns[0].column_type, ColumnType::Numeric);
        assert_eq!(dataset.schema.columns[1].column_type, ColumnType::Text);
    }
    #[test]
    fn sav_fixture_preserves_rows_columns_and_measure_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fixture.sav");
        let source = import_delimited_bytes(
            b"score,group\n5,A\n7,B\n",
            "source.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let metadata = ambers::SpssMetadata::from_arrow_schema(source.batch.schema().as_ref());
        ambers::write_sav(
            &path,
            &source.batch,
            &metadata,
            ambers::Compression::None,
            None,
        )
        .unwrap();
        let imported = import_path(&path, &ImportOptions::default()).unwrap();
        assert_eq!(imported.schema.case_count, 2);
        assert_eq!(imported.schema.columns.len(), 2);
        assert_eq!(imported.schema.columns[0].scale_type, ScaleType::Continuous);
        assert_eq!(imported.schema.columns[1].scale_type, ScaleType::Nominal);
    }
}
