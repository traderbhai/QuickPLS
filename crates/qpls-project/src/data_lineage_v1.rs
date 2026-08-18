use chrono::DateTime;
use qpls_core::{
    ApplyDatasetTransformationOptionsV2, DATASET_TRANSFORMATION_ENGINE_V2,
    DATASET_TRANSFORMATION_SCHEMA_V2, DatasetTransformationLineageV2,
    apply_dataset_transformation_v2, canonical_dataset_transformation_json_v2, sha256_hex,
};
use qpls_data::{DataKind, Dataset, DatasetDescriptor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

pub const PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1: &str = "data_lineage";
pub const PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectDatasetVersionOperationV1 {
    Import,
    Metadata,
    Recode,
    Transform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectDatasetVersionRecordV1 {
    pub dataset_id: String,
    pub parent_dataset_id: Option<String>,
    pub operation: ProjectDatasetVersionOperationV1,
    pub created_at: Option<String>,
    pub summary: String,
    pub source_column: Option<String>,
    pub target_column: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transformation: Option<DatasetTransformationLineageV2>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectDataLineageV1 {
    pub schema_version: u32,
    pub records: Vec<ProjectDatasetVersionRecordV1>,
}

impl Default for ProjectDataLineageV1 {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1,
            records: Vec::new(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ProjectDataLineageV1Error {
    #[error("reserved data_lineage is malformed: {0}")]
    Malformed(String),
    #[error("reserved data_lineage schema {0} is unsupported")]
    UnsupportedSchema(u32),
    #[error("reserved data_lineage must contain at least one record when present")]
    Empty,
    #[error("resident dataset ID {0} is duplicated")]
    DuplicateDatasetId(Uuid),
    #[error("dataset {dataset_id} has duplicate column name {column}")]
    DuplicateDatasetColumn { dataset_id: Uuid, column: String },
    #[error("dataset {dataset_id} has an invalid resident shape: {reason}")]
    InvalidResidentDatasetShape { dataset_id: Uuid, reason: String },
    #[error("data_lineage {field} must be a canonical UUID, observed {value}")]
    NonCanonicalDatasetId { field: &'static str, value: String },
    #[error("data_lineage contains more than one record for dataset {0}")]
    DuplicateRecordDatasetId(Uuid),
    #[error("data_lineage record for dataset {dataset_id} is invalid: {reason}")]
    InvalidRecord { dataset_id: String, reason: String },
    #[error("data_lineage record refers to unknown {role} dataset {dataset_id}")]
    UnknownDataset {
        role: &'static str,
        dataset_id: Uuid,
    },
    #[error("data_lineage graph contains a cycle through dataset {0}")]
    Cycle(Uuid),
    #[error("data_lineage transform operation ID {0} is duplicated")]
    DuplicateTransformOperationId(String),
    #[error("data_lineage transform for dataset {dataset_id} cannot be canonicalized: {reason}")]
    TransformCanonicalization { dataset_id: Uuid, reason: String },
    #[error("data_lineage transform for dataset {dataset_id} cannot be replayed: {reason}")]
    TransformReplay { dataset_id: Uuid, reason: String },
    #[error(
        "data_lineage replay for dataset {dataset_id} differs from the archived output: {field}"
    )]
    TransformReplayMismatch {
        dataset_id: Uuid,
        field: &'static str,
    },
}

pub fn read_project_data_lineage_v1(
    layouts: &BTreeMap<String, Value>,
) -> Result<Option<ProjectDataLineageV1>, ProjectDataLineageV1Error> {
    let lineage: Option<ProjectDataLineageV1> = layouts
        .get(PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1)
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| ProjectDataLineageV1Error::Malformed(error.to_string()))?;
    if let Some(lineage) = lineage.as_ref() {
        if lineage.schema_version != PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1 {
            return Err(ProjectDataLineageV1Error::UnsupportedSchema(
                lineage.schema_version,
            ));
        }
        if lineage.records.is_empty() {
            return Err(ProjectDataLineageV1Error::Empty);
        }
    }
    Ok(lineage)
}

pub fn write_project_data_lineage_v1(
    layouts: &mut BTreeMap<String, Value>,
    lineage: &ProjectDataLineageV1,
) -> Result<(), ProjectDataLineageV1Error> {
    if lineage.schema_version != PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1 {
        return Err(ProjectDataLineageV1Error::UnsupportedSchema(
            lineage.schema_version,
        ));
    }
    if lineage.records.is_empty() {
        return Err(ProjectDataLineageV1Error::Empty);
    }
    let value = serde_json::to_value(lineage)
        .map_err(|error| ProjectDataLineageV1Error::Malformed(error.to_string()))?;
    layouts.insert(PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1.to_owned(), value);
    Ok(())
}

pub fn validate_project_data_lineage_descriptors_v1(
    datasets: &[DatasetDescriptor],
    layouts: &BTreeMap<String, Value>,
) -> Result<Option<ProjectDataLineageV1>, ProjectDataLineageV1Error> {
    let lineage = read_project_data_lineage_v1(layouts)?;
    validate_data_lineage_descriptors_v1(datasets, lineage.as_ref())?;
    Ok(lineage)
}

pub fn validate_project_data_lineage_resident_v1(
    datasets: &[Dataset],
    layouts: &BTreeMap<String, Value>,
) -> Result<Option<ProjectDataLineageV1>, ProjectDataLineageV1Error> {
    let lineage = read_project_data_lineage_v1(layouts)?;
    validate_data_lineage_resident_v1(datasets, lineage.as_ref())?;
    Ok(lineage)
}

pub fn validate_data_lineage_descriptors_v1(
    datasets: &[DatasetDescriptor],
    lineage: Option<&ProjectDataLineageV1>,
) -> Result<(), ProjectDataLineageV1Error> {
    let datasets_by_id = descriptor_index(datasets)?;
    let Some(lineage) = lineage else {
        return Ok(());
    };
    if lineage.schema_version != PROJECT_DATA_LINEAGE_SCHEMA_VERSION_V1 {
        return Err(ProjectDataLineageV1Error::UnsupportedSchema(
            lineage.schema_version,
        ));
    }
    if lineage.records.is_empty() {
        return Err(ProjectDataLineageV1Error::Empty);
    }

    let mut records_by_id = BTreeMap::<Uuid, &ProjectDatasetVersionRecordV1>::new();
    let mut parent_by_id = BTreeMap::<Uuid, Option<Uuid>>::new();
    let mut transform_operation_ids = BTreeSet::new();

    for record in &lineage.records {
        let dataset_id = canonical_dataset_id("datasetId", &record.dataset_id)?;
        if records_by_id.insert(dataset_id, record).is_some() {
            return Err(ProjectDataLineageV1Error::DuplicateRecordDatasetId(
                dataset_id,
            ));
        }
        let dataset = datasets_by_id.get(&dataset_id).copied().ok_or(
            ProjectDataLineageV1Error::UnknownDataset {
                role: "output",
                dataset_id,
            },
        )?;
        let parent_id = record
            .parent_dataset_id
            .as_deref()
            .map(|value| canonical_dataset_id("parentDatasetId", value))
            .transpose()?;
        if parent_id == Some(dataset_id) {
            return invalid_record(record, "parentDatasetId cannot equal datasetId");
        }
        let parent = parent_id
            .map(|id| {
                datasets_by_id
                    .get(&id)
                    .copied()
                    .ok_or(ProjectDataLineageV1Error::UnknownDataset {
                        role: "parent",
                        dataset_id: id,
                    })
            })
            .transpose()?;
        parent_by_id.insert(dataset_id, parent_id);
        validate_record_descriptor_shape(
            dataset_id,
            dataset,
            parent_id,
            parent,
            record,
            &mut transform_operation_ids,
        )?;
    }

    for start in records_by_id.keys().copied() {
        let mut visited = BTreeSet::new();
        let mut current = start;
        loop {
            if !visited.insert(current) {
                return Err(ProjectDataLineageV1Error::Cycle(current));
            }
            let Some(Some(parent)) = parent_by_id.get(&current) else {
                break;
            };
            if !records_by_id.contains_key(parent) {
                break;
            }
            current = *parent;
        }
    }
    Ok(())
}

pub fn validate_data_lineage_resident_v1(
    datasets: &[Dataset],
    lineage: Option<&ProjectDataLineageV1>,
) -> Result<(), ProjectDataLineageV1Error> {
    let mut residents_by_id = BTreeMap::<Uuid, &Dataset>::new();
    let mut descriptors = Vec::with_capacity(datasets.len());
    for dataset in datasets {
        if residents_by_id.insert(dataset.id, dataset).is_some() {
            return Err(ProjectDataLineageV1Error::DuplicateDatasetId(dataset.id));
        }
        validate_resident_dataset_shape(dataset)?;
        descriptors.push(DatasetDescriptor::from(dataset));
    }
    validate_data_lineage_descriptors_v1(&descriptors, lineage)?;
    let Some(lineage) = lineage else {
        return Ok(());
    };

    for record in &lineage.records {
        let Some(transformation) = record.transformation.as_ref() else {
            continue;
        };
        if record.operation != ProjectDatasetVersionOperationV1::Transform {
            continue;
        }
        let dataset_id = canonical_dataset_id("datasetId", &record.dataset_id)?;
        let source_id = canonical_dataset_id(
            "transformation.source_dataset_id",
            &transformation.source_dataset_id,
        )?;
        let source = residents_by_id.get(&source_id).copied().ok_or(
            ProjectDataLineageV1Error::UnknownDataset {
                role: "transform source",
                dataset_id: source_id,
            },
        )?;
        let archived = residents_by_id.get(&dataset_id).copied().ok_or(
            ProjectDataLineageV1Error::UnknownDataset {
                role: "transform output",
                dataset_id,
            },
        )?;
        let replay = apply_dataset_transformation_v2(
            source,
            &transformation.spec,
            &ApplyDatasetTransformationOptionsV2 {
                output_dataset_id: transformation.output_dataset_id.clone(),
                output_dataset_name: archived.name.clone(),
                created_at: transformation.created_at.clone(),
            },
        )
        .map_err(|error| ProjectDataLineageV1Error::TransformReplay {
            dataset_id,
            reason: error.to_string(),
        })?;
        if replay.lineage != *transformation {
            return Err(ProjectDataLineageV1Error::TransformReplayMismatch {
                dataset_id,
                field: "transformation receipt",
            });
        }
        if replay.dataset.id != archived.id {
            return Err(ProjectDataLineageV1Error::TransformReplayMismatch {
                dataset_id,
                field: "dataset ID",
            });
        }
        if replay.dataset.name != archived.name {
            return Err(ProjectDataLineageV1Error::TransformReplayMismatch {
                dataset_id,
                field: "dataset name",
            });
        }
        if replay.dataset.schema != archived.schema {
            return Err(ProjectDataLineageV1Error::TransformReplayMismatch {
                dataset_id,
                field: "dataset schema",
            });
        }
        if replay.dataset.fingerprint != archived.fingerprint {
            return Err(ProjectDataLineageV1Error::TransformReplayMismatch {
                dataset_id,
                field: "dataset fingerprint",
            });
        }
    }
    Ok(())
}

fn descriptor_index<'a>(
    datasets: &'a [DatasetDescriptor],
) -> Result<BTreeMap<Uuid, &'a DatasetDescriptor>, ProjectDataLineageV1Error> {
    let mut datasets_by_id = BTreeMap::new();
    for dataset in datasets {
        if datasets_by_id.insert(dataset.id, dataset).is_some() {
            return Err(ProjectDataLineageV1Error::DuplicateDatasetId(dataset.id));
        }
        let mut columns = BTreeSet::new();
        for column in &dataset.schema.columns {
            if !columns.insert(column.name.as_str()) {
                return Err(ProjectDataLineageV1Error::DuplicateDatasetColumn {
                    dataset_id: dataset.id,
                    column: column.name.clone(),
                });
            }
        }
    }
    Ok(datasets_by_id)
}

fn validate_resident_dataset_shape(dataset: &Dataset) -> Result<(), ProjectDataLineageV1Error> {
    if dataset.batch.num_rows() != dataset.schema.case_count {
        return Err(ProjectDataLineageV1Error::InvalidResidentDatasetShape {
            dataset_id: dataset.id,
            reason: "Arrow row count differs from schema case_count".into(),
        });
    }
    if dataset.batch.num_columns() != dataset.schema.columns.len() {
        return Err(ProjectDataLineageV1Error::InvalidResidentDatasetShape {
            dataset_id: dataset.id,
            reason: "Arrow column count differs from schema columns".into(),
        });
    }
    for (field, column) in dataset
        .batch
        .schema()
        .fields()
        .iter()
        .zip(&dataset.schema.columns)
    {
        if field.name() != &column.name {
            return Err(ProjectDataLineageV1Error::InvalidResidentDatasetShape {
                dataset_id: dataset.id,
                reason: format!(
                    "Arrow column {} differs from metadata column {}",
                    field.name(),
                    column.name
                ),
            });
        }
    }
    Ok(())
}

fn canonical_dataset_id(
    field: &'static str,
    value: &str,
) -> Result<Uuid, ProjectDataLineageV1Error> {
    let parsed =
        Uuid::parse_str(value).map_err(|_| ProjectDataLineageV1Error::NonCanonicalDatasetId {
            field,
            value: value.to_owned(),
        })?;
    if parsed.to_string() != value {
        return Err(ProjectDataLineageV1Error::NonCanonicalDatasetId {
            field,
            value: value.to_owned(),
        });
    }
    Ok(parsed)
}

fn invalid_record<T>(
    record: &ProjectDatasetVersionRecordV1,
    reason: impl Into<String>,
) -> Result<T, ProjectDataLineageV1Error> {
    Err(ProjectDataLineageV1Error::InvalidRecord {
        dataset_id: record.dataset_id.clone(),
        reason: reason.into(),
    })
}

fn require_parent<'a>(
    record: &ProjectDatasetVersionRecordV1,
    parent_id: Option<Uuid>,
    parent: Option<&'a DatasetDescriptor>,
) -> Result<(Uuid, &'a DatasetDescriptor), ProjectDataLineageV1Error> {
    match (parent_id, parent) {
        (Some(parent_id), Some(parent)) => Ok((parent_id, parent)),
        _ => invalid_record(record, "operation requires parentDatasetId"),
    }
}

fn required_column<'a>(
    record: &'a ProjectDatasetVersionRecordV1,
    value: &'a Option<String>,
    field: &'static str,
) -> Result<&'a str, ProjectDataLineageV1Error> {
    value
        .as_deref()
        .filter(|column| !column.is_empty())
        .ok_or_else(|| ProjectDataLineageV1Error::InvalidRecord {
            dataset_id: record.dataset_id.clone(),
            reason: format!("{field} is required and cannot be empty"),
        })
}

fn ensure_column(
    record: &ProjectDatasetVersionRecordV1,
    dataset: &DatasetDescriptor,
    column: &str,
    role: &'static str,
) -> Result<(), ProjectDataLineageV1Error> {
    if dataset
        .schema
        .columns
        .iter()
        .any(|candidate| candidate.name == column)
    {
        Ok(())
    } else {
        invalid_record(record, format!("{role} column {column} does not exist"))
    }
}

fn same_base_shape(left: &DatasetDescriptor, right: &DatasetDescriptor) -> bool {
    left.schema.version == right.schema.version
        && left.schema.kind == right.schema.kind
        && left.schema.case_count == right.schema.case_count
        && left.schema.sample_size == right.schema.sample_size
}

fn same_column_names(left: &DatasetDescriptor, right: &DatasetDescriptor) -> bool {
    left.schema
        .columns
        .iter()
        .map(|column| column.name.as_str())
        .eq(right
            .schema
            .columns
            .iter()
            .map(|column| column.name.as_str()))
}

fn parent_plus_target_shape(
    parent: &DatasetDescriptor,
    output: &DatasetDescriptor,
    target: &str,
) -> bool {
    same_base_shape(parent, output)
        && output.schema.columns.len() == parent.schema.columns.len() + 1
        && parent
            .schema
            .columns
            .iter()
            .zip(&output.schema.columns)
            .all(|(left, right)| left.name == right.name)
        && output
            .schema
            .columns
            .last()
            .is_some_and(|column| column.name == target)
}

fn parent_plus_targets_shape(
    parent: &DatasetDescriptor,
    output: &DatasetDescriptor,
    targets: &[String],
) -> bool {
    same_base_shape(parent, output)
        && !targets.is_empty()
        && output.schema.columns.len() == parent.schema.columns.len() + targets.len()
        && parent
            .schema
            .columns
            .iter()
            .zip(&output.schema.columns)
            .all(|(left, right)| left.name == right.name)
        && output.schema.columns[parent.schema.columns.len()..]
            .iter()
            .map(|column| column.name.as_str())
            .eq(targets.iter().map(String::as_str))
}

fn validate_record_descriptor_shape(
    dataset_id: Uuid,
    dataset: &DatasetDescriptor,
    parent_id: Option<Uuid>,
    parent: Option<&DatasetDescriptor>,
    record: &ProjectDatasetVersionRecordV1,
    transform_operation_ids: &mut BTreeSet<String>,
) -> Result<(), ProjectDataLineageV1Error> {
    if record.summary.trim().is_empty() {
        return invalid_record(record, "summary cannot be empty");
    }
    if let Some(created_at) = record.created_at.as_deref()
        && DateTime::parse_from_rfc3339(created_at).is_err()
    {
        return invalid_record(record, "createdAt must be an RFC 3339 timestamp");
    }

    match record.operation {
        ProjectDatasetVersionOperationV1::Import => {
            if parent_id.is_some()
                || record.source_column.is_some()
                || record.target_column.is_some()
                || record.transformation.is_some()
            {
                return invalid_record(
                    record,
                    "import must be a root without columns or a transformation receipt",
                );
            }
        }
        ProjectDatasetVersionOperationV1::Metadata => {
            let (_, parent) = require_parent(record, parent_id, parent)?;
            let source = required_column(record, &record.source_column, "sourceColumn")?;
            if record.target_column.is_some() || record.transformation.is_some() {
                return invalid_record(
                    record,
                    "metadata cannot declare targetColumn or a transformation receipt",
                );
            }
            ensure_column(record, parent, source, "source")?;
            ensure_column(record, dataset, source, "output")?;
            if !same_base_shape(parent, dataset) || !same_column_names(parent, dataset) {
                return invalid_record(
                    record,
                    "metadata output must retain the parent dataset row and column shape",
                );
            }
        }
        ProjectDatasetVersionOperationV1::Recode => {
            let (_, parent) = require_parent(record, parent_id, parent)?;
            let source = required_column(record, &record.source_column, "sourceColumn")?;
            let target = required_column(record, &record.target_column, "targetColumn")?;
            if record.transformation.is_some() {
                return invalid_record(
                    record,
                    "legacy recode is referential-only and cannot carry a transform receipt",
                );
            }
            if parent.schema.kind != DataKind::Raw || dataset.schema.kind != DataKind::Raw {
                return invalid_record(record, "recode requires raw parent and output datasets");
            }
            ensure_column(record, parent, source, "source")?;
            if !parent_plus_target_shape(parent, dataset, target) {
                return invalid_record(
                    record,
                    "recode output must append exactly the declared target to the parent shape",
                );
            }
        }
        ProjectDatasetVersionOperationV1::Transform => {
            let (parent_id, parent) = require_parent(record, parent_id, parent)?;
            let target = required_column(record, &record.target_column, "targetColumn")?;
            let transformation = record.transformation.as_ref().ok_or_else(|| {
                ProjectDataLineageV1Error::InvalidRecord {
                    dataset_id: record.dataset_id.clone(),
                    reason: "transform requires an exact transformation receipt".into(),
                }
            })?;
            if parent.schema.kind != DataKind::Raw || dataset.schema.kind != DataKind::Raw {
                return invalid_record(record, "transform requires raw parent and output datasets");
            }
            let expected_inputs = transformation.spec.input_columns();
            let expected_outputs = transformation.spec.target_columns();
            if record.source_column.as_deref() != expected_inputs.first().map(String::as_str) {
                return invalid_record(
                    record,
                    "transform sourceColumn must equal the first declared input or be absent for a zero-input transform",
                );
            }
            if let Some(source) = record.source_column.as_deref() {
                ensure_column(record, parent, source, "source")?;
            }
            if !parent_plus_targets_shape(parent, dataset, &expected_outputs) {
                return invalid_record(
                    record,
                    "transform output must append exactly the declared targets to the parent shape",
                );
            }
            validate_transform_descriptor_contract(
                dataset_id,
                dataset,
                parent_id,
                parent,
                record.source_column.as_deref(),
                target,
                record,
                transformation,
            )?;
            if !transform_operation_ids.insert(transformation.operation_id.clone()) {
                return Err(ProjectDataLineageV1Error::DuplicateTransformOperationId(
                    transformation.operation_id.clone(),
                ));
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_transform_descriptor_contract(
    dataset_id: Uuid,
    dataset: &DatasetDescriptor,
    parent_id: Uuid,
    parent: &DatasetDescriptor,
    source: Option<&str>,
    target: &str,
    record: &ProjectDatasetVersionRecordV1,
    transformation: &DatasetTransformationLineageV2,
) -> Result<(), ProjectDataLineageV1Error> {
    if transformation.schema_version != DATASET_TRANSFORMATION_SCHEMA_V2
        || transformation.engine != DATASET_TRANSFORMATION_ENGINE_V2
    {
        return invalid_record(record, "transform receipt schema or engine is unsupported");
    }
    let receipt_source = canonical_dataset_id(
        "transformation.source_dataset_id",
        &transformation.source_dataset_id,
    )?;
    let receipt_output = canonical_dataset_id(
        "transformation.output_dataset_id",
        &transformation.output_dataset_id,
    )?;
    if receipt_source != parent_id || receipt_output != dataset_id {
        return invalid_record(
            record,
            "transform receipt dataset identities differ from the lineage edge",
        );
    }
    if transformation.source_dataset_fingerprint != parent.fingerprint.0
        || transformation.output_dataset_fingerprint != dataset.fingerprint.0
    {
        return invalid_record(
            record,
            "transform receipt fingerprints differ from the resident descriptors",
        );
    }
    let expected_inputs = transformation.spec.input_columns();
    let expected_outputs = transformation.spec.target_columns();
    if transformation.input_columns != expected_inputs
        || transformation.output_columns != expected_outputs
        || expected_outputs.first().map(String::as_str) != Some(target)
        || expected_inputs.first().map(String::as_str) != source
    {
        return invalid_record(
            record,
            "transform receipt columns differ from the canonical specification",
        );
    }
    for input in &expected_inputs {
        ensure_column(record, parent, input, "transform input")?;
    }
    let maximum_output_cells = dataset
        .schema
        .case_count
        .checked_mul(expected_outputs.len())
        .unwrap_or(usize::MAX);
    if transformation.source_row_count != parent.schema.case_count
        || transformation.output_missing_count > maximum_output_cells
    {
        return invalid_record(record, "transform receipt row accounting is invalid");
    }
    if record.created_at.as_deref() != Some(transformation.created_at.as_str())
        || DateTime::parse_from_rfc3339(&transformation.created_at).is_err()
    {
        return invalid_record(
            record,
            "transform createdAt must exactly equal the receipt timestamp",
        );
    }
    let canonical_spec =
        canonical_dataset_transformation_json_v2(&transformation.spec).map_err(|error| {
            ProjectDataLineageV1Error::TransformCanonicalization {
                dataset_id,
                reason: error.to_string(),
            }
        })?;
    let expected_spec_sha256 = sha256_hex(canonical_spec.as_bytes());
    if transformation.spec_sha256 != expected_spec_sha256 {
        return invalid_record(
            record,
            "transform spec_sha256 does not match the canonical spec",
        );
    }
    let operation_digest = sha256_hex(
        format!(
            "{}\0{}\0{}",
            transformation.source_dataset_fingerprint,
            transformation.spec_sha256,
            transformation.output_dataset_id
        )
        .as_bytes(),
    );
    let expected_operation_id = format!("dataset_transform:{}", &operation_digest[..24]);
    if transformation.operation_id != expected_operation_id {
        return invalid_record(
            record,
            "transform operation_id does not match the exact source/spec/output identity",
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Project, ProjectError, load_project, save_project};
    use qpls_core::{DatasetTransformationSpecV2, apply_dataset_transformation_v2};
    use qpls_data::{ImportOptions, import_delimited_bytes};
    use sha2::{Digest, Sha256};
    use std::{
        fs::File,
        io::{Read, Write},
    };
    use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

    fn source_dataset() -> Dataset {
        import_delimited_bytes(
            b"x,y\n1,4\n2,5\n3,6\n",
            "source.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap()
    }

    fn reconstructable_fixture() -> (Vec<Dataset>, ProjectDataLineageV1) {
        let source = source_dataset();
        let spec = DatasetTransformationSpecV2::ReverseScale {
            source_column: "x".into(),
            target_column: "x_reversed".into(),
            scale_min: 1.0,
            scale_max: 3.0,
            target_label: None,
        };
        let mutation = apply_dataset_transformation_v2(
            &source,
            &spec,
            &ApplyDatasetTransformationOptionsV2 {
                output_dataset_id: Uuid::new_v4().to_string(),
                output_dataset_name: "derived".into(),
                created_at: "2026-08-15T00:00:00.000Z".into(),
            },
        )
        .unwrap();
        let lineage = ProjectDataLineageV1 {
            schema_version: 1,
            records: vec![
                ProjectDatasetVersionRecordV1 {
                    dataset_id: source.id.to_string(),
                    parent_dataset_id: None,
                    operation: ProjectDatasetVersionOperationV1::Import,
                    created_at: None,
                    summary: "Imported source".into(),
                    source_column: None,
                    target_column: None,
                    transformation: None,
                },
                ProjectDatasetVersionRecordV1 {
                    dataset_id: mutation.dataset.id.to_string(),
                    parent_dataset_id: Some(source.id.to_string()),
                    operation: ProjectDatasetVersionOperationV1::Transform,
                    created_at: Some(mutation.lineage.created_at.clone()),
                    summary: "Derived x_reversed".into(),
                    source_column: Some("x".into()),
                    target_column: Some("x_reversed".into()),
                    transformation: Some(mutation.lineage),
                },
            ],
        };
        (vec![source, mutation.dataset], lineage)
    }

    #[test]
    fn absent_lineage_is_backward_readable_without_synthesis() {
        let datasets = vec![source_dataset()];
        assert!(
            validate_project_data_lineage_resident_v1(&datasets, &BTreeMap::new())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn reconstructable_transform_replays_exactly() {
        let (datasets, lineage) = reconstructable_fixture();
        validate_data_lineage_resident_v1(&datasets, Some(&lineage)).unwrap();
    }

    #[test]
    fn zero_input_and_multi_output_transform_descriptors_are_exact_and_replayable() {
        let source = source_dataset();
        let add = apply_dataset_transformation_v2(
            &source,
            &DatasetTransformationSpecV2::AddColumn {
                target_column: "cohort".into(),
                value: qpls_core::DatasetCellV2::Missing,
                target_type: qpls_data::ColumnType::Numeric,
                target_scale: qpls_data::ScaleType::Continuous,
                target_label: None,
                value_labels: BTreeMap::new(),
            },
            &ApplyDatasetTransformationOptionsV2 {
                output_dataset_id: Uuid::new_v4().to_string(),
                output_dataset_name: "added".into(),
                created_at: "2026-08-15T00:00:00.000Z".into(),
            },
        )
        .unwrap();
        let mut add_lineage = ProjectDataLineageV1 {
            schema_version: 1,
            records: vec![ProjectDatasetVersionRecordV1 {
                dataset_id: add.dataset.id.to_string(),
                parent_dataset_id: Some(source.id.to_string()),
                operation: ProjectDatasetVersionOperationV1::Transform,
                created_at: Some(add.lineage.created_at.clone()),
                summary: "Added cohort".into(),
                source_column: None,
                target_column: Some("cohort".into()),
                transformation: Some(add.lineage.clone()),
            }],
        };
        let add_dataset = add.dataset.clone();
        validate_data_lineage_resident_v1(
            &[source.clone(), add_dataset.clone()],
            Some(&add_lineage),
        )
        .unwrap();
        add_lineage.records[0].source_column = Some("x".into());
        assert!(
            validate_data_lineage_descriptors_v1(
                &[
                    DatasetDescriptor::from(&source),
                    DatasetDescriptor::from(&add_dataset),
                ],
                Some(&add_lineage),
            )
            .is_err()
        );

        let missing = apply_dataset_transformation_v2(
            &source,
            &DatasetTransformationSpecV2::MissingMarkers {
                columns: vec![
                    qpls_core::DatasetMissingMarkerColumnV2 {
                        source_column: "x".into(),
                        target_column: "x_clean".into(),
                        markers: vec![qpls_core::NonMissingDatasetCellV2::Number(2.0)],
                        target_type: qpls_data::ColumnType::Numeric,
                        target_scale: qpls_data::ScaleType::Continuous,
                        target_label: None,
                        value_labels: BTreeMap::new(),
                    },
                    qpls_core::DatasetMissingMarkerColumnV2 {
                        source_column: "y".into(),
                        target_column: "y_clean".into(),
                        markers: vec![qpls_core::NonMissingDatasetCellV2::Number(5.0)],
                        target_type: qpls_data::ColumnType::Numeric,
                        target_scale: qpls_data::ScaleType::Continuous,
                        target_label: None,
                        value_labels: BTreeMap::new(),
                    },
                ],
            },
            &ApplyDatasetTransformationOptionsV2 {
                output_dataset_id: Uuid::new_v4().to_string(),
                output_dataset_name: "cleaned".into(),
                created_at: "2026-08-15T00:00:00.000Z".into(),
            },
        )
        .unwrap();
        let missing_lineage = ProjectDataLineageV1 {
            schema_version: 1,
            records: vec![ProjectDatasetVersionRecordV1 {
                dataset_id: missing.dataset.id.to_string(),
                parent_dataset_id: Some(source.id.to_string()),
                operation: ProjectDatasetVersionOperationV1::Transform,
                created_at: Some(missing.lineage.created_at.clone()),
                summary: "Cleaned x and y".into(),
                source_column: Some("x".into()),
                target_column: Some("x_clean".into()),
                transformation: Some(missing.lineage),
            }],
        };
        validate_data_lineage_resident_v1(&[source, missing.dataset], Some(&missing_lineage))
            .unwrap();
    }

    #[test]
    fn malformed_reserved_lineage_never_becomes_empty() {
        let layouts = BTreeMap::from([(
            PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1.into(),
            serde_json::json!({"schemaVersion": 1, "records": [], "unexpected": true}),
        )]);
        assert!(matches!(
            read_project_data_lineage_v1(&layouts),
            Err(ProjectDataLineageV1Error::Malformed(_))
        ));
    }

    #[test]
    fn canonical_dataset_ids_and_acyclic_edges_are_required() {
        let source = source_dataset();
        let mut second = source.clone();
        second.id = Uuid::new_v4();
        let mut lineage = ProjectDataLineageV1 {
            schema_version: 1,
            records: vec![
                ProjectDatasetVersionRecordV1 {
                    dataset_id: source.id.to_string(),
                    parent_dataset_id: Some(second.id.to_string()),
                    operation: ProjectDatasetVersionOperationV1::Metadata,
                    created_at: None,
                    summary: "metadata".into(),
                    source_column: Some("x".into()),
                    target_column: None,
                    transformation: None,
                },
                ProjectDatasetVersionRecordV1 {
                    dataset_id: second.id.to_string(),
                    parent_dataset_id: Some(source.id.to_string()),
                    operation: ProjectDatasetVersionOperationV1::Metadata,
                    created_at: None,
                    summary: "metadata".into(),
                    source_column: Some("x".into()),
                    target_column: None,
                    transformation: None,
                },
            ],
        };
        let descriptors = vec![
            DatasetDescriptor::from(&source),
            DatasetDescriptor::from(&second),
        ];
        assert!(matches!(
            validate_data_lineage_descriptors_v1(&descriptors, Some(&lineage)),
            Err(ProjectDataLineageV1Error::Cycle(_))
        ));

        lineage.records[0].dataset_id = source.id.as_braced().to_string();
        assert!(matches!(
            validate_data_lineage_descriptors_v1(&descriptors, Some(&lineage)),
            Err(ProjectDataLineageV1Error::NonCanonicalDatasetId { .. })
        ));
    }

    #[test]
    fn coordinated_transform_receipt_tamper_is_rejected_by_replay() {
        let (datasets, mut lineage) = reconstructable_fixture();
        let transformation = lineage.records[1].transformation.as_mut().unwrap();
        transformation.output_missing_count += 1;
        assert!(validate_data_lineage_resident_v1(&datasets, Some(&lineage)).is_err());
    }

    #[test]
    fn v5_save_and_load_replay_lineage_and_preserve_unreserved_layouts() {
        let (datasets, lineage) = reconstructable_fixture();
        let mut project = Project::new("Lineage round trip");
        project.datasets = datasets;
        project.layouts.insert(
            "third_party_presentation".into(),
            serde_json::json!({"keep": [3, 2, 1]}),
        );
        write_project_data_lineage_v1(&mut project.layouts, &lineage).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lineage.qpls");

        save_project(&path, &project).unwrap();
        let reopened = load_project(&path).unwrap();

        assert_eq!(
            read_project_data_lineage_v1(&reopened.layouts).unwrap(),
            Some(lineage)
        );
        assert_eq!(
            reopened.layouts["third_party_presentation"],
            serde_json::json!({"keep": [3, 2, 1]})
        );
    }

    #[test]
    fn v5_save_rejects_malformed_or_tampered_reserved_lineage_before_writing() {
        let mut malformed = Project::new("Malformed lineage");
        malformed.datasets.push(source_dataset());
        malformed.layouts.insert(
            PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1.into(),
            serde_json::json!({"schemaVersion": 1, "records": [], "unknown": true}),
        );
        let directory = tempfile::tempdir().unwrap();
        let malformed_path = directory.path().join("malformed.qpls");
        assert!(matches!(
            save_project(&malformed_path, &malformed),
            Err(ProjectError::Invalid(_))
        ));
        assert!(!malformed_path.exists());

        let (datasets, mut lineage) = reconstructable_fixture();
        lineage.records[1]
            .transformation
            .as_mut()
            .unwrap()
            .output_missing_count += 1;
        let mut tampered = Project::new("Tampered lineage");
        tampered.datasets = datasets;
        write_project_data_lineage_v1(&mut tampered.layouts, &lineage).unwrap();
        let tampered_path = directory.path().join("tampered.qpls");
        assert!(matches!(
            save_project(&tampered_path, &tampered),
            Err(ProjectError::Invalid(_))
        ));
        assert!(!tampered_path.exists());
    }

    #[test]
    fn v5_load_rejects_checksum_recomputed_malformed_reserved_lineage() {
        let (datasets, lineage) = reconstructable_fixture();
        let mut project = Project::new("Load tamper");
        project.datasets = datasets;
        write_project_data_lineage_v1(&mut project.layouts, &lineage).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.qpls");
        let tampered_path = directory.path().join("tampered.qpls");
        save_project(&source_path, &project).unwrap();

        let mut source = ZipArchive::new(File::open(&source_path).unwrap()).unwrap();
        let mut entries = Vec::new();
        for index in 0..source.len() {
            let mut entry = source.by_index(index).unwrap();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).unwrap();
            entries.push((entry.name().to_owned(), bytes));
        }
        let project_index = entries
            .iter()
            .position(|(name, _)| name == "project.json")
            .unwrap();
        let mut project_json: Value = serde_json::from_slice(&entries[project_index].1).unwrap();
        project_json["layouts"][PROJECT_DATA_LINEAGE_LAYOUT_KEY_V1]["records"] =
            serde_json::json!([]);
        entries[project_index].1 = serde_json::to_vec_pretty(&project_json).unwrap();
        let project_sha256 = format!("{:x}", Sha256::digest(&entries[project_index].1));
        let manifest_index = entries
            .iter()
            .position(|(name, _)| name == "manifest.json")
            .unwrap();
        let mut manifest: Value = serde_json::from_slice(&entries[manifest_index].1).unwrap();
        manifest["checksums"]["project.json"] = Value::String(project_sha256);
        entries[manifest_index].1 = serde_json::to_vec_pretty(&manifest).unwrap();

        let mut output = ZipWriter::new(File::create(&tampered_path).unwrap());
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in entries {
            output.start_file(name, options).unwrap();
            output.write_all(&bytes).unwrap();
        }
        output.finish().unwrap();

        assert!(matches!(
            load_project(&tampered_path),
            Err(ProjectError::Invalid(_))
        ));
    }
}
