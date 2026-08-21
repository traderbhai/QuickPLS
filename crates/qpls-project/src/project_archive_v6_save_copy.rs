//! Windows-first, schema-6-to-schema-6 new-destination save-copy.
//!
//! The live schema-5 writer and the historical upgrade writer are deliberately
//! outside this module. This seam persists only a detached schema-6 document
//! whose non-model authority still matches a strictly opened schema-6 source.
//! On Windows the destination parent is pinned by handle. A temporary child is
//! created relative to that handle with `FILE_CREATE`, no sharing, and native
//! delete-on-close. All I/O and strict validation use that identity. The final
//! commit atomically publishes a no-replace hard link relative to the same
//! pinned parent; closing removes only the temporary link. This module never
//! deletes, renames, replaces, or cleans up by pathname.

use super::{
    ProjectArchiveDocumentV6, ProjectArchiveV6Error, ProjectError, ProjectManifest,
    archive_integrity::{
        DEFAULT_ARCHIVE_LIMITS, MANIFEST_ENTRY_NAME, MAX_MANIFEST_UNCOMPRESSED_BYTES,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES, PROJECT_ENTRY_NAME,
    },
    load_project_archive_v6_from_file, serialize_project_document_v6,
};
use qpls_data::{Dataset, DatasetDescriptor, write_arrow};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use zip::{ZipArchive, ZipWriter, result::ZipError, write::SimpleFileOptions};

const STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY: &str =
    "standard_sem_model_v4_diagram_layouts_v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardSemModelV4DiagramLayoutsLaneV1 {
    schema_version: u32,
    models: BTreeMap<String, StandardSemModelV4DiagramLayoutV1>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardSemModelV4DiagramLayoutV1 {
    schema_version: u32,
    model_id: String,
    diagram_layout: StandardSemModelV4DiagramLayoutStateV1,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StandardSemModelV4DiagramLayoutStateV1 {
    diagram_version: String,
    construct_layouts: BTreeMap<String, StandardSemConstructLayoutV1>,
    indicator_layouts: BTreeMap<String, BTreeMap<String, StandardSemIndicatorLayoutV1>>,
    edge_layouts: BTreeMap<String, StandardSemEdgeLayoutV1>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    diagram_viewport: Option<StandardSemDiagramViewportV1>,
    diagram_theme: String,
    show_grid: bool,
    layout_locked: bool,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    standard_sem_presentation: Option<StandardSemPresentationLayoutV1>,
    #[serde(default)]
    moderation_anchor_fractions: BTreeMap<String, f64>,
    #[serde(default)]
    moderation_connector_bend_points: BTreeMap<String, Vec<StandardSemDiagramPointV1>>,
}

fn deserialize_optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StandardSemConstructLayoutV1 {
    x: f64,
    y: f64,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    width: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    height: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StandardSemIndicatorSideV1 {
    Left,
    Right,
    Top,
    Bottom,
    Free,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StandardSemIndicatorLayoutV1 {
    side: StandardSemIndicatorSideV1,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    x: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    y: Option<f64>,
    order: f64,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StandardSemEdgeRoutingV1 {
    Straight,
    Curved,
    Orthogonal,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardSemDiagramPointV1 {
    x: f64,
    y: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StandardSemEdgeLayoutV1 {
    routing: StandardSemEdgeRoutingV1,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    bend_points: Option<Vec<StandardSemDiagramPointV1>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    label_offset: Option<StandardSemDiagramPointV1>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pinned: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StandardSemDiagramViewportV1 {
    x: f64,
    y: f64,
    zoom: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StandardSemPresentationLayoutV1 {
    schema_version: u32,
    objects: Vec<StandardSemPresentationLayoutObjectV1>,
}

#[derive(Debug)]
struct StandardSemNullableTextV1(Option<String>);

impl<'de> Deserialize<'de> for StandardSemNullableTextV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer).map(Self)
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StandardSemPresentationLayoutObjectV1 {
    Caption {
        id: String,
        text: String,
        x: f64,
        y: f64,
    },
    Note {
        id: String,
        subject: String,
        text: String,
        x: f64,
        y: f64,
    },
    Shape {
        id: String,
        shape: StandardSemPresentationShapeV1,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        label: StandardSemNullableTextV1,
        style: BTreeMap<String, String>,
    },
    Image {
        id: String,
        #[serde(rename = "assetRef")]
        asset_ref: String,
        #[serde(rename = "altText")]
        alt_text: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        style: BTreeMap<String, String>,
    },
    Line {
        id: String,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        label: StandardSemNullableTextV1,
        #[serde(rename = "startMarker")]
        start_marker: StandardSemNullableTextV1,
        #[serde(rename = "endMarker")]
        end_marker: StandardSemNullableTextV1,
        style: BTreeMap<String, String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StandardSemPresentationShapeV1 {
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Diamond,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectArchiveV6SaveCopyReceipt {
    pub schema_version: u32,
    pub source_archive_path: String,
    pub source_archive_sha256: String,
    pub source_verified_unchanged: bool,
    pub destination_archive_path: String,
    pub destination_archive_sha256: String,
    pub destination_archive_bytes: u64,
    pub strict_reopen_validated: bool,
    pub model_count: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectArchiveV6SaveCopyError {
    #[error("schema-6 save-copy is supported only by the Windows desktop writer")]
    UnsupportedPlatform,
    #[error("schema-6 save-copy requires absolute source and destination paths")]
    AbsolutePathsRequired,
    #[error("schema-6 save-copy source and destination paths must differ")]
    SourceAndDestinationMustDiffer,
    #[error("schema-6 save-copy destination filename is invalid: {0}")]
    InvalidDestinationName(String),
    #[error("schema-6 save-copy destination must use the .qpls extension")]
    DestinationExtension,
    #[error("schema-6 save-copy source must be a regular non-reparse file")]
    SourceMustBeRegularNonReparseFile,
    #[error("schema-6 save-copy destination parent must be a local, non-reparse directory")]
    DestinationParentMustBeLocalNonReparseDirectory,
    #[error("schema-6 save-copy does not support remote destination directories")]
    RemoteDestinationUnsupported,
    #[error("schema-6 save-copy does not support destination filesystem {0}")]
    UnsupportedDestinationFilesystem(String),
    #[error("schema-6 save-copy destination already exists: {0}")]
    DestinationExists(PathBuf),
    #[error("schema-6 save-copy source digest is stale (expected {expected}, observed {observed})")]
    SourceDigestMismatch { expected: String, observed: String },
    #[error("schema-6 save-copy source changed while the copy was prepared")]
    SourceChangedDuringSave,
    #[error("schema-6 save-copy may change only the detached models lane")]
    NonModelAuthorityChanged,
    #[error("schema-6 save-copy could not publish its validated file identity: {0}")]
    PublicationFailed(String),
    #[error("schema-6 save-copy was cancelled before commit")]
    CancelledBeforeCommit,
    #[error("schema-6 save-copy exceeded archive limits: {0}")]
    ArchiveLimit(String),
    #[error("schema-6 save-copy strict reopen differed from the requested document")]
    StrictReopenMismatch,
    #[error("new schema-6 archive publication requires a document with no resident datasets")]
    NewDocumentRequiresEmptyDatasets,
    #[error("schema-6 save-copy destination handle identity changed unexpectedly")]
    DestinationIdentityChanged,
    #[error(transparent)]
    Contract(#[from] ProjectArchiveV6Error),
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] ZipError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Persists one detached schema-6 model-authority document to a destination
/// that must not exist. The source archive is never written.
pub fn save_project_archive_v6_model_copy(
    source: &Path,
    expected_source_sha256: &str,
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
) -> Result<ProjectArchiveV6SaveCopyReceipt, ProjectArchiveV6SaveCopyError> {
    save_project_archive_v6_model_copy_with_control(
        source,
        expected_source_sha256,
        destination,
        document,
        || false,
    )
}

/// Cancellation-aware pre-commit form. Cancellation is never observed after
/// the validated identity's final no-replace link is published.
pub fn save_project_archive_v6_model_copy_with_control<Cancelled>(
    source: &Path,
    expected_source_sha256: &str,
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    mut cancelled: Cancelled,
) -> Result<ProjectArchiveV6SaveCopyReceipt, ProjectArchiveV6SaveCopyError>
where
    Cancelled: FnMut() -> bool,
{
    #[cfg(windows)]
    {
        save_copy_windows_with_hooks(
            source,
            expected_source_sha256,
            destination,
            document,
            &mut cancelled,
            |_| Ok(()),
            |_| Ok(()),
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (
            source,
            expected_source_sha256,
            destination,
            document,
            &mut cancelled,
        );
        Err(ProjectArchiveV6SaveCopyError::UnsupportedPlatform)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectArchiveV6NewDocumentPublication {
    pub destination_archive_sha256: String,
    pub destination_archive_bytes: u64,
    pub strict_reopen_validated: bool,
}

/// Publishes a validated, dataset-free schema-6 document to a destination that
/// must not exist. This crate-private seam is shared by the public new-project
/// API and the proven save-copy writer's pinned-parent implementation.
pub(crate) fn publish_new_project_archive_v6_document(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
) -> Result<ProjectArchiveV6NewDocumentPublication, ProjectArchiveV6SaveCopyError> {
    #[cfg(windows)]
    {
        publish_new_project_archive_v6_document_windows_with_hooks(
            destination,
            document,
            |_| Ok(()),
            |_| Ok(()),
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (destination, document);
        Err(ProjectArchiveV6SaveCopyError::UnsupportedPlatform)
    }
}

/// Publishes a newly authored schema-6 document together with its exact
/// resident datasets. This is intentionally separate from the dataset-free
/// new-document seam and the schema-6 model-only save-copy seam: neither
/// existing contract is widened for General SEM project bootstrap.
pub(crate) fn publish_new_project_archive_v6_document_with_resident_datasets(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    datasets: &[Dataset],
) -> Result<ProjectArchiveV6NewDocumentPublication, ProjectArchiveV6SaveCopyError> {
    publish_new_project_archive_v6_document_with_resident_datasets_before_publish(
        destination,
        document,
        datasets,
        || Ok(()),
    )
}

/// Resident-dataset publication with one fail-closed callback after strict
/// reopen validation and immediately before the final no-replace link. This is
/// crate-private so authority-revision writers can recheck a pinned source
/// handle without widening the existing public save-copy contract.
pub(crate) fn publish_new_project_archive_v6_document_with_resident_datasets_before_publish<
    BeforePublish,
>(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    datasets: &[Dataset],
    before_publish: BeforePublish,
) -> Result<ProjectArchiveV6NewDocumentPublication, ProjectArchiveV6SaveCopyError>
where
    BeforePublish: FnOnce() -> Result<(), ProjectArchiveV6SaveCopyError>,
{
    #[cfg(windows)]
    {
        publish_new_project_archive_v6_document_with_resident_datasets_windows(
            destination,
            document,
            datasets,
            before_publish,
        )
    }
    #[cfg(not(windows))]
    {
        let _ = (destination, document, datasets, before_publish);
        Err(ProjectArchiveV6SaveCopyError::UnsupportedPlatform)
    }
}

#[cfg(all(test, windows))]
pub(crate) fn publish_new_project_archive_v6_document_with_hooks<BeforeWrite, BeforeStrictReopen>(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    before_write: BeforeWrite,
    before_strict_reopen: BeforeStrictReopen,
) -> Result<ProjectArchiveV6NewDocumentPublication, ProjectArchiveV6SaveCopyError>
where
    BeforeWrite: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
    BeforeStrictReopen: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
{
    publish_new_project_archive_v6_document_windows_with_hooks(
        destination,
        document,
        before_write,
        before_strict_reopen,
    )
}

fn ensure_not_cancelled<Cancelled>(
    cancelled: &mut Cancelled,
) -> Result<(), ProjectArchiveV6SaveCopyError>
where
    Cancelled: FnMut() -> bool,
{
    if cancelled() {
        Err(ProjectArchiveV6SaveCopyError::CancelledBeforeCommit)
    } else {
        Ok(())
    }
}

fn paths_are_textually_distinct(source: &Path, destination: &Path) -> bool {
    let normalize = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    normalize(source) != normalize(destination)
}

fn validate_destination_name(destination: &Path) -> Result<(), ProjectArchiveV6SaveCopyError> {
    let Some(name) = destination.file_name().and_then(|name| name.to_str()) else {
        return Err(ProjectArchiveV6SaveCopyError::InvalidDestinationName(
            "a Unicode filename is required".to_owned(),
        ));
    };
    if !destination
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("qpls"))
    {
        return Err(ProjectArchiveV6SaveCopyError::DestinationExtension);
    }
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with([' ', '.'])
        || name.chars().any(|character| {
            character <= '\u{1f}'
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
    {
        return Err(ProjectArchiveV6SaveCopyError::InvalidDestinationName(
            name.to_owned(),
        ));
    }
    let device_stem = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches([' ', '.'])
        .to_ascii_uppercase();
    let reserved = matches!(device_stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (device_stem.len() == 4
            && (device_stem.starts_with("COM") || device_stem.starts_with("LPT"))
            && matches!(device_stem.as_bytes()[3], b'1'..=b'9'));
    if reserved {
        return Err(ProjectArchiveV6SaveCopyError::InvalidDestinationName(
            name.to_owned(),
        ));
    }
    Ok(())
}

fn validate_standard_diagram_layout_lane(
    candidate: &ProjectArchiveDocumentV6,
    value: &serde_json::Value,
) -> Result<(), ProjectArchiveV6SaveCopyError> {
    let lane: StandardSemModelV4DiagramLayoutsLaneV1 = serde_json::from_value(value.clone())
        .map_err(|_| ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged)?;
    if lane.schema_version != 1 {
        return Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged);
    }
    let expected_ids = candidate
        .models
        .iter()
        .filter(|record| {
            !matches!(
                &record.payload,
                super::ProjectModelPayloadV6::LegacyEstimandUnspecified { .. }
            )
        })
        .map(|record| record.model_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let observed_ids = lane
        .models
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    if observed_ids != expected_ids {
        return Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged);
    }
    for (model_id, layout) in lane.models {
        if layout.schema_version != 1
            || layout.model_id != model_id
            || model_id.trim() != model_id
            || model_id.is_empty()
            || layout.diagram_layout.diagram_version != "sem_designer_v1"
            || !matches!(
                layout.diagram_layout.diagram_theme.as_str(),
                "academic_grayscale"
                    | "smartpls_like"
                    | "quickpls_color"
                    | "journal_mono"
                    | "high_contrast"
            )
            || !standard_diagram_layout_state_is_valid(&layout.diagram_layout)
        {
            return Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged);
        }
    }
    Ok(())
}

fn standard_diagram_layout_state_is_valid(layout: &StandardSemModelV4DiagramLayoutStateV1) -> bool {
    let construct_layouts_valid = layout.construct_layouts.iter().all(|(id, item)| {
        exact_nonempty_id(id)
            && item.x.is_finite()
            && item.y.is_finite()
            && item.width.is_none_or(f64::is_finite)
            && item.height.is_none_or(f64::is_finite)
            && item.pinned.is_none_or(|_| true)
    });
    let indicator_layouts_valid = layout
        .indicator_layouts
        .iter()
        .all(|(construct_id, items)| {
            exact_nonempty_id(construct_id)
                && items.values().all(|item| {
                    let _ = &item.side;
                    item.x.is_none_or(f64::is_finite)
                        && item.y.is_none_or(f64::is_finite)
                        && item.order.is_finite()
                        && item.order >= 0.0
                        && item.order.fract() == 0.0
                        && item.pinned.is_none_or(|_| true)
                })
        });
    let edge_layouts_valid = layout.edge_layouts.values().all(|item| {
        let _ = &item.routing;
        item.bend_points
            .as_ref()
            .is_none_or(|points| points.iter().all(standard_diagram_point_is_valid))
            && item
                .label_offset
                .as_ref()
                .is_none_or(standard_diagram_point_is_valid)
            && item.pinned.is_none_or(|_| true)
    });
    construct_layouts_valid
        && indicator_layouts_valid
        && edge_layouts_valid
        && layout.diagram_viewport.as_ref().is_none_or(|viewport| {
            viewport.x.is_finite() && viewport.y.is_finite() && viewport.zoom.is_finite()
        })
        && layout
            .standard_sem_presentation
            .as_ref()
            .is_none_or(standard_sem_presentation_is_valid)
        && layout
            .moderation_anchor_fractions
            .iter()
            .all(|(term_id, fraction)| {
                exact_nonempty_id(term_id) && fraction.is_finite() && (0.2..=0.8).contains(fraction)
            })
        && layout
            .moderation_connector_bend_points
            .iter()
            .all(|(connector_id, points)| {
                exact_nonempty_id(connector_id)
                    && points.len() <= 8
                    && points.iter().all(standard_diagram_point_is_valid)
            })
        && matches!(layout.show_grid, true | false)
        && matches!(layout.layout_locked, true | false)
}

fn standard_diagram_point_is_valid(point: &StandardSemDiagramPointV1) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn standard_sem_presentation_is_valid(presentation: &StandardSemPresentationLayoutV1) -> bool {
    if presentation.schema_version != 1 {
        return false;
    }
    let mut ids = std::collections::BTreeSet::new();
    presentation.objects.iter().all(|object| {
        let id = match object {
            StandardSemPresentationLayoutObjectV1::Caption { id, .. }
            | StandardSemPresentationLayoutObjectV1::Note { id, .. }
            | StandardSemPresentationLayoutObjectV1::Shape { id, .. }
            | StandardSemPresentationLayoutObjectV1::Image { id, .. }
            | StandardSemPresentationLayoutObjectV1::Line { id, .. } => id,
        };
        exact_nonempty_id(id)
            && ids.insert(id.as_str())
            && standard_sem_presentation_object_is_valid(object)
    })
}

fn standard_sem_presentation_object_is_valid(
    object: &StandardSemPresentationLayoutObjectV1,
) -> bool {
    match object {
        StandardSemPresentationLayoutObjectV1::Caption { text, x, y, .. } => {
            let _ = text;
            x.is_finite() && y.is_finite()
        }
        StandardSemPresentationLayoutObjectV1::Note {
            subject,
            text,
            x,
            y,
            ..
        } => {
            let _ = (subject, text);
            x.is_finite() && y.is_finite()
        }
        StandardSemPresentationLayoutObjectV1::Shape {
            shape,
            x,
            y,
            width,
            height,
            label,
            style,
            ..
        } => {
            let _ = (shape, &label.0, style);
            x.is_finite()
                && y.is_finite()
                && width.is_finite()
                && *width > 0.0
                && height.is_finite()
                && *height > 0.0
        }
        StandardSemPresentationLayoutObjectV1::Image {
            asset_ref,
            alt_text,
            x,
            y,
            width,
            height,
            style,
            ..
        } => {
            let _ = style;
            !asset_ref.trim().is_empty()
                && !alt_text.trim().is_empty()
                && x.is_finite()
                && y.is_finite()
                && width.is_finite()
                && *width > 0.0
                && height.is_finite()
                && *height > 0.0
        }
        StandardSemPresentationLayoutObjectV1::Line {
            x1,
            y1,
            x2,
            y2,
            label,
            start_marker,
            end_marker,
            style,
            ..
        } => {
            let _ = (&label.0, &start_marker.0, &end_marker.0, style);
            x1.is_finite()
                && y1.is_finite()
                && x2.is_finite()
                && y2.is_finite()
                && (*x1 != *x2 || *y1 != *y2)
        }
    }
}

fn exact_nonempty_id(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn non_model_authority_is_unchanged(
    source: &ProjectArchiveDocumentV6,
    candidate: &ProjectArchiveDocumentV6,
) -> Result<bool, ProjectArchiveV6SaveCopyError> {
    let mut normalized = candidate.clone();
    normalized.models = source.models.clone();
    let source_layout = source
        .layouts
        .get(STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY);
    let candidate_layout = candidate
        .layouts
        .get(STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY);
    if source_layout != candidate_layout {
        let candidate_layout =
            candidate_layout.ok_or(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged)?;
        validate_standard_diagram_layout_lane(candidate, candidate_layout)?;
        match source_layout {
            Some(value) => {
                normalized.layouts.insert(
                    STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY.to_owned(),
                    value.clone(),
                );
            }
            None => {
                normalized
                    .layouts
                    .remove(STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY);
            }
        }
    }
    Ok(serde_json::to_vec(&normalized)? == serde_json::to_vec(source)?)
}

fn sha256_file_handle(file: &mut File) -> Result<(u64, String), std::io::Error> {
    file.seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes = bytes.checked_add(read as u64).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "file length overflow")
        })?;
        digest.update(&buffer[..read]);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok((bytes, format!("{:x}", digest.finalize())))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn ensure_entry_size(
    name: &str,
    size: u64,
    limit: u64,
) -> Result<(), ProjectArchiveV6SaveCopyError> {
    if size > limit {
        Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
            "entry {name} declares {size} bytes; limit is {limit}"
        )))
    } else {
        Ok(())
    }
}

fn add_total_uncompressed(total: &mut u64, size: u64) -> Result<(), ProjectArchiveV6SaveCopyError> {
    *total = total.checked_add(size).ok_or_else(|| {
        ProjectArchiveV6SaveCopyError::ArchiveLimit("total uncompressed size overflowed".to_owned())
    })?;
    if *total > DEFAULT_ARCHIVE_LIMITS.max_total_uncompressed_bytes {
        return Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
            "total uncompressed size exceeds {} bytes",
            DEFAULT_ARCHIVE_LIMITS.max_total_uncompressed_bytes
        )));
    }
    Ok(())
}

#[cfg(windows)]
struct WrittenProjectArchiveV6 {
    destination_archive_sha256: String,
    destination_archive_bytes: u64,
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn write_project_archive_v6_to_temporary<
    Cancelled,
    BeforeWrite,
    WriteDatasets,
    BeforeStrictReopen,
    AfterStrictReopen,
>(
    owned_temporary: &mut DeleteOnCloseTemporary,
    destination_identity: &FileIdentity,
    document: &ProjectArchiveDocumentV6,
    cancelled: &mut Cancelled,
    before_write: BeforeWrite,
    write_datasets: WriteDatasets,
    before_strict_reopen: BeforeStrictReopen,
    after_strict_reopen: AfterStrictReopen,
) -> Result<WrittenProjectArchiveV6, ProjectArchiveV6SaveCopyError>
where
    Cancelled: FnMut() -> bool,
    BeforeWrite: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
    WriteDatasets: FnOnce(
        &mut ZipWriter<File>,
        SimpleFileOptions,
        &mut BTreeMap<String, String>,
        &mut u64,
        &mut Cancelled,
    ) -> Result<(), ProjectArchiveV6SaveCopyError>,
    BeforeStrictReopen: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
    AfterStrictReopen: FnOnce() -> Result<(), ProjectArchiveV6SaveCopyError>,
{
    let project_bytes = serialize_project_document_v6(document)?;
    ensure_entry_size(
        PROJECT_ENTRY_NAME,
        project_bytes.len() as u64,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES,
    )?;
    let entry_count = document.datasets.len().saturating_add(2);
    if entry_count > DEFAULT_ARCHIVE_LIMITS.max_entries {
        return Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
            "{entry_count} entries exceed the {}-entry limit",
            DEFAULT_ARCHIVE_LIMITS.max_entries
        )));
    }

    before_write(&mut owned_temporary.file)?;
    owned_temporary.file.seek(SeekFrom::Start(0))?;
    let mut output = ZipWriter::new(owned_temporary.file.try_clone()?);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut checksums = BTreeMap::new();
    let mut total_uncompressed = 0_u64;

    output.start_file(PROJECT_ENTRY_NAME, options)?;
    output.write_all(&project_bytes)?;
    checksums.insert(PROJECT_ENTRY_NAME.to_owned(), sha256_bytes(&project_bytes));
    add_total_uncompressed(&mut total_uncompressed, project_bytes.len() as u64)?;
    ensure_not_cancelled(cancelled)?;

    write_datasets(
        &mut output,
        options,
        &mut checksums,
        &mut total_uncompressed,
        cancelled,
    )?;

    let manifest = ProjectManifest {
        schema_version: super::PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
        project_id: document.project_id,
        name: document.name.clone(),
        created_at: document.created_at,
        modified_at: document.modified_at,
        engine_version: qpls_core::ENGINE_VERSION.to_owned(),
        checksum_algorithm: "sha256".to_owned(),
        checksums,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)?;
    ensure_entry_size(
        MANIFEST_ENTRY_NAME,
        manifest_bytes.len() as u64,
        MAX_MANIFEST_UNCOMPRESSED_BYTES,
    )?;
    add_total_uncompressed(&mut total_uncompressed, manifest_bytes.len() as u64)?;
    output.start_file(MANIFEST_ENTRY_NAME, options)?;
    output.write_all(&manifest_bytes)?;
    let mut finished = output.finish()?;
    finished.sync_all()?;
    if file_identity_from_file(&finished)? != *destination_identity {
        return Err(ProjectArchiveV6SaveCopyError::DestinationIdentityChanged);
    }
    ensure_not_cancelled(cancelled)?;

    before_strict_reopen(&mut finished)?;
    let reopened = load_project_archive_v6_from_file(finished.try_clone()?)?;
    if serialize_project_document_v6(&reopened.document)? != project_bytes
        || serde_json::to_vec(&reopened.manifest)? != serde_json::to_vec(&manifest)?
    {
        return Err(ProjectArchiveV6SaveCopyError::StrictReopenMismatch);
    }
    ensure_not_cancelled(cancelled)?;
    after_strict_reopen()?;

    let (destination_archive_bytes, destination_archive_sha256) =
        sha256_file_handle(&mut finished)?;
    if file_identity_from_file(&finished)? != *destination_identity {
        return Err(ProjectArchiveV6SaveCopyError::DestinationIdentityChanged);
    }
    ensure_not_cancelled(cancelled)?;

    Ok(WrittenProjectArchiveV6 {
        destination_archive_sha256,
        destination_archive_bytes,
    })
}

#[cfg(windows)]
fn publish_new_project_archive_v6_document_windows_with_hooks<BeforeWrite, BeforeStrictReopen>(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    before_write: BeforeWrite,
    before_strict_reopen: BeforeStrictReopen,
) -> Result<ProjectArchiveV6NewDocumentPublication, ProjectArchiveV6SaveCopyError>
where
    BeforeWrite: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
    BeforeStrictReopen: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
{
    if !destination.is_absolute() {
        return Err(ProjectArchiveV6SaveCopyError::AbsolutePathsRequired);
    }
    validate_destination_name(destination)?;
    document.ensure_valid()?;
    if !document.datasets.is_empty() {
        return Err(ProjectArchiveV6SaveCopyError::NewDocumentRequiresEmptyDatasets);
    }

    let parent = open_and_validate_destination_parent(destination)?;
    let mut owned_temporary = create_relative_temporary(&parent, destination)?;
    let destination_identity = file_identity_from_file(&owned_temporary.file)?;
    let mut cancelled = || false;
    let written = write_project_archive_v6_to_temporary(
        &mut owned_temporary,
        &destination_identity,
        document,
        &mut cancelled,
        before_write,
        |_, _, _, _, _| Ok(()),
        before_strict_reopen,
        || Ok(()),
    )?;
    let publication = ProjectArchiveV6NewDocumentPublication {
        destination_archive_sha256: written.destination_archive_sha256,
        destination_archive_bytes: written.destination_archive_bytes,
        strict_reopen_validated: true,
    };

    // Final commit point: the no-replace link is the only publication. No
    // fallible validation occurs after it succeeds.
    owned_temporary.publish(&parent, destination)?;
    Ok(publication)
}

#[cfg(windows)]
fn publish_new_project_archive_v6_document_with_resident_datasets_windows<BeforePublish>(
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    datasets: &[Dataset],
    before_publish: BeforePublish,
) -> Result<ProjectArchiveV6NewDocumentPublication, ProjectArchiveV6SaveCopyError>
where
    BeforePublish: FnOnce() -> Result<(), ProjectArchiveV6SaveCopyError>,
{
    if !destination.is_absolute() {
        return Err(ProjectArchiveV6SaveCopyError::AbsolutePathsRequired);
    }
    validate_destination_name(destination)?;
    document.ensure_valid()?;
    if datasets.is_empty() || datasets.len() != document.datasets.len() {
        return Err(ProjectArchiveV6SaveCopyError::Project(
            ProjectError::Invalid(
                "new schema-6 resident dataset authorities differ from the project document".into(),
            ),
        ));
    }

    let mut encoded_datasets = Vec::with_capacity(datasets.len());
    for (descriptor, dataset) in document.datasets.iter().zip(datasets) {
        let resident_descriptor = DatasetDescriptor::from(dataset);
        if descriptor.id != resident_descriptor.id
            || descriptor.name != resident_descriptor.name
            || descriptor.schema != resident_descriptor.schema
            || descriptor.fingerprint != resident_descriptor.fingerprint
        {
            return Err(ProjectArchiveV6SaveCopyError::Project(
                ProjectError::Invalid(format!(
                    "resident dataset {} differs from its schema-6 descriptor authority",
                    descriptor.id
                )),
            ));
        }
        let entry_name = format!("data/{}.arrow", descriptor.id);
        if entry_name.len() > DEFAULT_ARCHIVE_LIMITS.max_entry_name_bytes {
            return Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
                "entry name {entry_name} exceeds the {}-byte limit",
                DEFAULT_ARCHIVE_LIMITS.max_entry_name_bytes
            )));
        }
        let arrow_bytes = write_arrow(&dataset.batch)
            .map_err(ProjectError::from)
            .map_err(ProjectArchiveV6SaveCopyError::Project)?;
        ensure_entry_size(
            &entry_name,
            arrow_bytes.len() as u64,
            DEFAULT_ARCHIVE_LIMITS.max_entry_uncompressed_bytes,
        )?;
        encoded_datasets.push((entry_name, arrow_bytes));
    }

    let parent = open_and_validate_destination_parent(destination)?;
    let mut owned_temporary = create_relative_temporary(&parent, destination)?;
    let destination_identity = file_identity_from_file(&owned_temporary.file)?;
    let mut cancelled = || false;
    let written = write_project_archive_v6_to_temporary(
        &mut owned_temporary,
        &destination_identity,
        document,
        &mut cancelled,
        |_| Ok(()),
        move |output, options, checksums, total_uncompressed, cancelled| {
            for (entry_name, arrow_bytes) in encoded_datasets {
                ensure_not_cancelled(cancelled)?;
                output.start_file(&entry_name, options)?;
                output.write_all(&arrow_bytes)?;
                add_total_uncompressed(total_uncompressed, arrow_bytes.len() as u64)?;
                checksums.insert(entry_name, sha256_bytes(&arrow_bytes));
            }
            Ok(())
        },
        |_| Ok(()),
        || Ok(()),
    )?;
    let publication = ProjectArchiveV6NewDocumentPublication {
        destination_archive_sha256: written.destination_archive_sha256,
        destination_archive_bytes: written.destination_archive_bytes,
        strict_reopen_validated: true,
    };

    // The callback is deliberately after strict reopen and before publication;
    // failure drops the delete-on-close temporary without exposing a final
    // destination link.
    before_publish()?;
    // No fallible validation follows the no-replace publication point.
    owned_temporary.publish(&parent, destination)?;
    Ok(publication)
}

#[cfg(windows)]
fn save_copy_windows_with_hooks<Cancelled, BeforeWrite, BeforeStrictReopen>(
    source: &Path,
    expected_source_sha256: &str,
    destination: &Path,
    document: &ProjectArchiveDocumentV6,
    cancelled: &mut Cancelled,
    before_write: BeforeWrite,
    before_strict_reopen: BeforeStrictReopen,
) -> Result<ProjectArchiveV6SaveCopyReceipt, ProjectArchiveV6SaveCopyError>
where
    Cancelled: FnMut() -> bool,
    BeforeWrite: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
    BeforeStrictReopen: FnOnce(&mut File) -> Result<(), ProjectArchiveV6SaveCopyError>,
{
    if !source.is_absolute() || !destination.is_absolute() {
        return Err(ProjectArchiveV6SaveCopyError::AbsolutePathsRequired);
    }
    if !paths_are_textually_distinct(source, destination) {
        return Err(ProjectArchiveV6SaveCopyError::SourceAndDestinationMustDiffer);
    }
    validate_destination_name(destination)?;
    document.ensure_valid()?;
    ensure_not_cancelled(cancelled)?;

    let mut source_file = open_exclusive_non_reparse_source(source)?;
    let (_, source_sha256) = sha256_file_handle(&mut source_file)?;
    if source_sha256 != expected_source_sha256 {
        return Err(ProjectArchiveV6SaveCopyError::SourceDigestMismatch {
            expected: expected_source_sha256.to_owned(),
            observed: source_sha256,
        });
    }
    let source_loaded = load_project_archive_v6_from_file(source_file.try_clone()?)?;
    if !non_model_authority_is_unchanged(&source_loaded.document, document)? {
        return Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged);
    }
    let (_, rechecked_source_sha256) = sha256_file_handle(&mut source_file)?;
    if rechecked_source_sha256 != expected_source_sha256 {
        return Err(ProjectArchiveV6SaveCopyError::SourceChangedDuringSave);
    }
    ensure_not_cancelled(cancelled)?;

    let parent = open_and_validate_destination_parent(destination)?;
    let mut owned_temporary = create_relative_temporary(&parent, destination)?;
    let destination_identity = file_identity_from_file(&owned_temporary.file)?;

    let operation: Result<ProjectArchiveV6SaveCopyReceipt, ProjectArchiveV6SaveCopyError> =
        (|| {
            source_file.seek(SeekFrom::Start(0))?;
            let source_zip = ZipArchive::new(source_file.try_clone()?)?;
            let written = write_project_archive_v6_to_temporary(
                &mut owned_temporary,
                &destination_identity,
                document,
                cancelled,
                before_write,
                move |output, options, checksums, total_uncompressed, cancelled| {
                    let mut source_zip = source_zip;
                    for descriptor in &document.datasets {
                        let entry_name = format!("data/{}.arrow", descriptor.id);
                        if entry_name.len() > DEFAULT_ARCHIVE_LIMITS.max_entry_name_bytes {
                            return Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
                                "entry name {entry_name} exceeds the {}-byte limit",
                                DEFAULT_ARCHIVE_LIMITS.max_entry_name_bytes
                            )));
                        }
                        let mut source_entry = source_zip.by_name(&entry_name)?;
                        let declared_size = source_entry.size();
                        ensure_entry_size(
                            &entry_name,
                            declared_size,
                            DEFAULT_ARCHIVE_LIMITS.max_entry_uncompressed_bytes,
                        )?;
                        output.start_file(&entry_name, options)?;
                        let mut digest = Sha256::new();
                        let mut copied = 0_u64;
                        let mut buffer = [0_u8; 64 * 1024];
                        loop {
                            ensure_not_cancelled(cancelled)?;
                            let read = source_entry.read(&mut buffer)?;
                            if read == 0 {
                                break;
                            }
                            copied = copied.checked_add(read as u64).ok_or_else(|| {
                                ProjectArchiveV6SaveCopyError::ArchiveLimit(
                                    "copied Arrow entry size overflowed".to_owned(),
                                )
                            })?;
                            if copied > declared_size {
                                return Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
                                    "entry {entry_name} expanded beyond its declared size"
                                )));
                            }
                            output.write_all(&buffer[..read])?;
                            digest.update(&buffer[..read]);
                        }
                        if copied != declared_size {
                            return Err(ProjectArchiveV6SaveCopyError::ArchiveLimit(format!(
                                "entry {entry_name} yielded {copied} bytes; expected {declared_size}"
                            )));
                        }
                        add_total_uncompressed(total_uncompressed, copied)?;
                        checksums.insert(entry_name, format!("{:x}", digest.finalize()));
                    }
                    Ok(())
                },
                before_strict_reopen,
                || {
                    let (_, final_source_sha256) = sha256_file_handle(&mut source_file)?;
                    if final_source_sha256 != expected_source_sha256 {
                        return Err(ProjectArchiveV6SaveCopyError::SourceChangedDuringSave);
                    }
                    Ok(())
                },
            )?;

            Ok(ProjectArchiveV6SaveCopyReceipt {
                schema_version: 1,
                source_archive_path: source.to_string_lossy().into_owned(),
                source_archive_sha256: expected_source_sha256.to_owned(),
                source_verified_unchanged: true,
                destination_archive_path: destination.to_string_lossy().into_owned(),
                destination_archive_sha256: written.destination_archive_sha256,
                destination_archive_bytes: written.destination_archive_bytes,
                strict_reopen_validated: true,
                model_count: document.models.len(),
            })
        })();

    let receipt = operation?;
    // Final commit point. No cancellation or fallible work occurs after the
    // final no-replace link has been published. Dropping the handle then
    // removes only its temporary delete-on-close link.
    owned_temporary.publish(&parent, destination)?;
    Ok(receipt)
}

#[cfg(windows)]
struct PinnedDestinationParent {
    file: File,
}

#[cfg(windows)]
struct DeleteOnCloseTemporary {
    file: File,
}

#[cfg(windows)]
impl DeleteOnCloseTemporary {
    fn publish(
        &mut self,
        parent: &PinnedDestinationParent,
        destination: &Path,
    ) -> Result<(), ProjectArchiveV6SaveCopyError> {
        publish_final_link(&self.file, parent, destination)
    }
}

#[cfg(windows)]
fn open_exclusive_non_reparse_source(source: &Path) -> Result<File, ProjectArchiveV6SaveCopyError> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;

    let file = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(source)?;
    let information = file_information(&file)?;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    };
    if information.dwFileAttributes & (FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT) != 0
    {
        return Err(ProjectArchiveV6SaveCopyError::SourceMustBeRegularNonReparseFile);
    }
    Ok(file)
}

#[cfg(windows)]
fn open_and_validate_destination_parent(
    destination: &Path,
) -> Result<PinnedDestinationParent, ProjectArchiveV6SaveCopyError> {
    use std::os::windows::{ffi::OsStrExt, io::FromRawHandle};
    use windows_sys::Win32::{
        Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CreateFileW, FILE_ADD_FILE, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_DELETE,
            FILE_SHARE_READ, FILE_SHARE_WRITE, GetFinalPathNameByHandleW,
            GetVolumeInformationByHandleW, OPEN_EXISTING, VOLUME_NAME_DOS,
        },
    };

    let parent_path = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or(ProjectArchiveV6SaveCopyError::DestinationParentMustBeLocalNonReparseDirectory)?;
    let wide = parent_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: the UTF-16 buffer is terminated and remains alive for the call.
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            GENERIC_READ | FILE_ADD_FILE,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            std::ptr::null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error().into());
    }
    // SAFETY: `raw` is a newly owned valid handle from CreateFileW.
    let file = unsafe { File::from_raw_handle(raw) };
    let information = file_information(&file)?;
    if information.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || information.dwFileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(ProjectArchiveV6SaveCopyError::DestinationParentMustBeLocalNonReparseDirectory);
    }

    use std::os::windows::io::AsRawHandle;
    let handle = file.as_raw_handle();
    let required =
        unsafe { GetFinalPathNameByHandleW(handle, std::ptr::null_mut(), 0, VOLUME_NAME_DOS) };
    if required == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let mut final_path = vec![0_u16; required as usize + 1];
    let written = unsafe {
        GetFinalPathNameByHandleW(
            handle,
            final_path.as_mut_ptr(),
            final_path.len() as u32,
            VOLUME_NAME_DOS,
        )
    };
    if written == 0 || written as usize >= final_path.len() {
        return Err(std::io::Error::last_os_error().into());
    }
    let final_path = String::from_utf16_lossy(&final_path[..written as usize]);
    if final_path
        .trim_start_matches("\\\\?\\")
        .to_ascii_uppercase()
        .starts_with("UNC\\")
    {
        return Err(ProjectArchiveV6SaveCopyError::RemoteDestinationUnsupported);
    }

    let mut filesystem_buffer = [0_u16; 32];
    let mut flags = 0_u32;
    let succeeded = unsafe {
        GetVolumeInformationByHandleW(
            handle,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut flags,
            filesystem_buffer.as_mut_ptr(),
            filesystem_buffer.len() as u32,
        )
    };
    if succeeded == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let filesystem_length = filesystem_buffer
        .iter()
        .position(|item| *item == 0)
        .unwrap_or(filesystem_buffer.len());
    let filesystem = String::from_utf16_lossy(&filesystem_buffer[..filesystem_length]);
    if !filesystem.eq_ignore_ascii_case("NTFS") {
        return Err(ProjectArchiveV6SaveCopyError::UnsupportedDestinationFilesystem(filesystem));
    }
    let _ = flags;
    Ok(PinnedDestinationParent { file })
}

#[cfg(windows)]
fn create_relative_temporary(
    parent: &PinnedDestinationParent,
    destination: &Path,
) -> Result<DeleteOnCloseTemporary, ProjectArchiveV6SaveCopyError> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle};
    use windows_sys::Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FILE_CREATE, FILE_DELETE_ON_CLOSE, FILE_NON_DIRECTORY_FILE, FILE_OPEN_REPARSE_POINT,
            FILE_SYNCHRONOUS_IO_NONALERT, FILE_WRITE_THROUGH, NtCreateFile,
        },
    };
    use windows_sys::Win32::{
        Foundation::{
            HANDLE, OBJ_CASE_INSENSITIVE, RtlNtStatusToDosError, STATUS_OBJECT_NAME_COLLISION,
            UNICODE_STRING,
        },
        Storage::FileSystem::{
            DELETE, FILE_ATTRIBUTE_NORMAL, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        },
        System::IO::IO_STATUS_BLOCK,
    };

    let final_name = destination
        .file_name()
        .expect("destination name validated before parent pinning")
        .to_string_lossy();
    for _ in 0..16 {
        let temporary_name = format!(".{final_name}.schema6-save-{}.tmp", uuid::Uuid::new_v4());
        let mut wide_name = temporary_name.encode_utf16().collect::<Vec<_>>();
        let length_bytes = wide_name.len().checked_mul(2).ok_or_else(|| {
            ProjectArchiveV6SaveCopyError::InvalidDestinationName(temporary_name.clone())
        })?;
        let length = u16::try_from(length_bytes).map_err(|_| {
            ProjectArchiveV6SaveCopyError::InvalidDestinationName(temporary_name.clone())
        })?;
        let unicode = UNICODE_STRING {
            Length: length,
            MaximumLength: length,
            Buffer: wide_name.as_mut_ptr(),
        };
        let attributes = OBJECT_ATTRIBUTES {
            Length: std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: parent.file.as_raw_handle(),
            ObjectName: &unicode,
            Attributes: OBJ_CASE_INSENSITIVE,
            SecurityDescriptor: std::ptr::null(),
            SecurityQualityOfService: std::ptr::null(),
        };
        let mut status_block = IO_STATUS_BLOCK::default();
        let mut raw: HANDLE = std::ptr::null_mut();
        // SAFETY: all descriptors and buffers are valid for this synchronous
        // call; RootDirectory is the pinned destination parent.
        let status = unsafe {
            NtCreateFile(
                &mut raw,
                FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE,
                &attributes,
                &mut status_block,
                std::ptr::null(),
                FILE_ATTRIBUTE_NORMAL,
                0,
                FILE_CREATE,
                FILE_DELETE_ON_CLOSE
                    | FILE_NON_DIRECTORY_FILE
                    | FILE_SYNCHRONOUS_IO_NONALERT
                    | FILE_OPEN_REPARSE_POINT
                    | FILE_WRITE_THROUGH,
                std::ptr::null(),
                0,
            )
        };
        if status == STATUS_OBJECT_NAME_COLLISION {
            continue;
        }
        if status < 0 {
            let win32_error = unsafe { RtlNtStatusToDosError(status) };
            return Err(std::io::Error::from_raw_os_error(win32_error as i32).into());
        }
        // SAFETY: successful NtCreateFile returned one newly owned handle.
        let file = unsafe { File::from_raw_handle(raw) };
        return Ok(DeleteOnCloseTemporary { file });
    }
    Err(ProjectArchiveV6SaveCopyError::PublicationFailed(
        "could not allocate a unique handle-owned temporary child".to_owned(),
    ))
}

fn publish_final_link(
    file: &File,
    parent: &PinnedDestinationParent,
    destination: &Path,
) -> Result<(), ProjectArchiveV6SaveCopyError> {
    use std::os::windows::{ffi::OsStrExt, io::AsRawHandle};
    use windows_sys::Wdk::Storage::FileSystem::{
        FILE_LINK_INFORMATION, FileLinkInformation, NtSetInformationFile,
    };
    use windows_sys::Win32::{
        Foundation::{RtlNtStatusToDosError, STATUS_OBJECT_NAME_COLLISION},
        System::IO::IO_STATUS_BLOCK,
    };

    let name = destination
        .file_name()
        .expect("destination name validated before publication");
    let wide_name = name.encode_wide().collect::<Vec<_>>();
    let file_name_bytes = wide_name.len().checked_mul(2).ok_or_else(|| {
        ProjectArchiveV6SaveCopyError::InvalidDestinationName(name.to_string_lossy().into_owned())
    })?;
    let header_bytes = std::mem::offset_of!(FILE_LINK_INFORMATION, FileName);
    let information_bytes = header_bytes.checked_add(file_name_bytes).ok_or_else(|| {
        ProjectArchiveV6SaveCopyError::InvalidDestinationName(name.to_string_lossy().into_owned())
    })?;
    let word_bytes = std::mem::size_of::<usize>();
    let words = information_bytes.div_ceil(word_bytes);
    let mut storage = vec![0_usize; words];
    let information = storage.as_mut_ptr().cast::<FILE_LINK_INFORMATION>();
    // SAFETY: `storage` is pointer-aligned and large enough for the fixed
    // header plus the exact UTF-16 filename payload.
    unsafe {
        (*information).Anonymous.ReplaceIfExists = false;
        (*information).RootDirectory = parent.file.as_raw_handle();
        (*information).FileNameLength = u32::try_from(file_name_bytes).map_err(|_| {
            ProjectArchiveV6SaveCopyError::InvalidDestinationName(
                name.to_string_lossy().into_owned(),
            )
        })?;
        std::ptr::copy_nonoverlapping(
            wide_name.as_ptr(),
            (*information).FileName.as_mut_ptr(),
            wide_name.len(),
        );
    }
    let mut status_block = IO_STATUS_BLOCK::default();
    // SAFETY: the buffer layout matches FILE_LINK_INFORMATION and both file
    // and RootDirectory handles remain valid for the synchronous call.
    let status = unsafe {
        NtSetInformationFile(
            file.as_raw_handle(),
            &mut status_block,
            information.cast(),
            u32::try_from(information_bytes).map_err(|_| {
                ProjectArchiveV6SaveCopyError::InvalidDestinationName(
                    name.to_string_lossy().into_owned(),
                )
            })?,
            FileLinkInformation,
        )
    };
    if status == STATUS_OBJECT_NAME_COLLISION {
        return Err(ProjectArchiveV6SaveCopyError::DestinationExists(
            destination.to_path_buf(),
        ));
    }
    if status < 0 {
        let win32_error = unsafe { RtlNtStatusToDosError(status) };
        return Err(ProjectArchiveV6SaveCopyError::PublicationFailed(
            std::io::Error::from_raw_os_error(win32_error as i32).to_string(),
        ));
    }
    // No operations, cancellation checks, or validation occur after this
    // successful no-replace publication. Caller returns immediately.
    Ok(())
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume_serial_number: u32,
    file_index: u64,
}

#[cfg(windows)]
fn file_information(
    file: &File,
) -> Result<windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION, std::io::Error> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(information)
    }
}

#[cfg(windows)]
fn file_identity_from_file(file: &File) -> Result<FileIdentity, std::io::Error> {
    let information = file_information(file)?;
    Ok(FileIdentity {
        volume_serial_number: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Project, ProjectArchiveUpgradeRequestV6, insert_sem_model_v4_draft_v6,
        load_project_archive_v6, plan_project_upgrade_to_v6,
    };
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        Construct, LegacyBasicModelInterpretationV4, MeasurementMode, ModelSpec, StructuralPath,
        convert_legacy_basic_model_v4,
    };
    use qpls_data::{ImportOptions, import_delimited_bytes, write_arrow};
    use std::{cell::Cell, collections::BTreeMap, fs};
    use uuid::Uuid;

    fn draft_model() -> qpls_core::SemModelV4 {
        convert_legacy_basic_model_v4(
            &ModelSpec {
                id: Uuid::from_u128(401),
                name: "Persisted detached draft".into(),
                constructs: vec![
                    Construct {
                        id: "x".into(),
                        name: "Predictor".into(),
                        short_name: "X".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["x1".into(), "x2".into()],
                    },
                    Construct {
                        id: "y".into(),
                        name: "Outcome".into(),
                        short_name: "Y".into(),
                        mode: MeasurementMode::Reflective,
                        indicators: vec!["y1".into(), "y2".into()],
                    },
                ],
                paths: vec![StructuralPath {
                    source: "x".into(),
                    target: "y".into(),
                }],
                controls: Vec::new(),
                higher_order_constructs: Vec::new(),
                interactions: Vec::new(),
            },
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
            &[],
        )
        .unwrap()
    }

    fn write_schema6_fixture(source: &Path) -> (ProjectArchiveDocumentV6, Vec<u8>, String) {
        let dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,3,4\n5,6,7,8\n",
            "save-copy.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let arrow = write_arrow(&dataset.batch).unwrap();
        let mut project = Project::new("Schema-6 model save-copy fixture");
        project.datasets.push(dataset);
        let document = plan_project_upgrade_to_v6(
            &project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: "a".repeat(64),
                source_archive_path: source
                    .with_file_name("historical-v5.qpls")
                    .to_string_lossy()
                    .into_owned(),
                destination_archive_path: source.to_string_lossy().into_owned(),
                upgraded_at: Utc.with_ymd_and_hms(2026, 8, 15, 12, 0, 0).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap()
        .document;
        let project_bytes = serialize_project_document_v6(&document).unwrap();
        let arrow_name = format!("data/{}.arrow", document.datasets[0].id);
        let manifest = ProjectManifest {
            schema_version: super::super::PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: document.project_id,
            name: document.name.clone(),
            created_at: document.created_at,
            modified_at: document.modified_at,
            engine_version: qpls_core::ENGINE_VERSION.into(),
            checksum_algorithm: "sha256".into(),
            checksums: BTreeMap::from([
                (PROJECT_ENTRY_NAME.to_owned(), sha256_bytes(&project_bytes)),
                (arrow_name.clone(), sha256_bytes(&arrow)),
            ]),
        };
        let mut writer = ZipWriter::new(File::create(source).unwrap());
        let options = SimpleFileOptions::default();
        writer.start_file(PROJECT_ENTRY_NAME, options).unwrap();
        writer.write_all(&project_bytes).unwrap();
        writer.start_file(&arrow_name, options).unwrap();
        writer.write_all(&arrow).unwrap();
        writer.start_file(MANIFEST_ENTRY_NAME, options).unwrap();
        writer
            .write_all(&serde_json::to_vec_pretty(&manifest).unwrap())
            .unwrap();
        writer.finish().unwrap();
        let source_bytes = fs::read(source).unwrap();
        (document, arrow, sha256_bytes(&source_bytes))
    }

    fn zip_entry(path: &Path, name: &str) -> Vec<u8> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entry = archive.by_name(name).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        bytes
    }

    fn temporary_links(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".schema6-save-") && name.ends_with(".tmp"))
            })
            .collect()
    }

    #[test]
    fn destination_name_rejects_ads_reserved_and_non_project_names() {
        assert!(matches!(
            validate_destination_name(Path::new(r"D:\study:stream.qpls")),
            Err(ProjectArchiveV6SaveCopyError::InvalidDestinationName(_))
        ));
        assert!(matches!(
            validate_destination_name(Path::new(r"D:\CON.qpls")),
            Err(ProjectArchiveV6SaveCopyError::InvalidDestinationName(_))
        ));
        assert!(matches!(
            validate_destination_name(Path::new(r"D:\study.zip")),
            Err(ProjectArchiveV6SaveCopyError::DestinationExtension)
        ));
        validate_destination_name(Path::new(r"D:\study-copy.qpls")).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn saves_detached_model_authority_to_new_strict_copy_and_preserves_source_and_arrow() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let destination = directory.path().join("saved-model-copy.qpls");
        let (source_document, arrow, source_sha256) = write_schema6_fixture(&source);
        let source_bytes = fs::read(&source).unwrap();
        let mut candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();
        let model_id = candidate.models[0].model_id.clone();
        candidate.layouts.insert(
            STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY.to_owned(),
            serde_json::json!({
                "schema_version": 1,
                "models": {
                    model_id.clone(): {
                        "schema_version": 1,
                        "model_id": model_id,
                        "diagram_layout": {
                            "diagramVersion": "sem_designer_v1",
                            "constructLayouts": {},
                            "indicatorLayouts": {},
                            "edgeLayouts": {},
                            "diagramTheme": "smartpls_like",
                            "showGrid": true,
                            "layoutLocked": false,
                            "standardSemPresentation": {
                                "schemaVersion": 1,
                                "objects": [{
                                    "kind": "caption",
                                    "id": "caption:1",
                                    "text": "Saved presentation",
                                    "x": 40,
                                    "y": 50
                                }]
                            }
                        }
                    }
                }
            }),
        );

        let receipt =
            save_project_archive_v6_model_copy(&source, &source_sha256, &destination, &candidate)
                .unwrap();

        assert_eq!(receipt.schema_version, 1);
        assert_eq!(
            receipt.source_archive_path,
            source.to_string_lossy().into_owned()
        );
        assert_eq!(receipt.source_archive_sha256, source_sha256);
        assert_eq!(
            receipt.destination_archive_path,
            destination.to_string_lossy().into_owned()
        );
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert!(receipt.source_verified_unchanged);
        assert!(receipt.strict_reopen_validated);
        assert_eq!(receipt.model_count, 1);
        let reopened = super::super::load_project_archive_v6(&destination).unwrap();
        assert_eq!(
            serialize_project_document_v6(&reopened.document).unwrap(),
            serialize_project_document_v6(&candidate).unwrap()
        );
        let arrow_name = format!("data/{}.arrow", candidate.datasets[0].id);
        assert_eq!(zip_entry(&destination, &arrow_name), arrow);
        assert_eq!(
            receipt.destination_archive_sha256,
            sha256_bytes(&fs::read(&destination).unwrap())
        );
        assert_eq!(
            receipt.destination_archive_bytes,
            fs::metadata(&destination).unwrap().len()
        );
        assert!(temporary_links(directory.path()).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn resident_precommit_callback_runs_after_strict_reopen_and_failure_publishes_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("resident-source-v6.qpls");
        let destination = directory.path().join("resident-revision-v6.qpls");
        let (source_document, _, source_sha256) = write_schema6_fixture(&source);
        let source_bytes = fs::read(&source).unwrap();
        let loaded = load_project_archive_v6(&source).unwrap();
        let callback_seen = Cell::new(false);

        let result = publish_new_project_archive_v6_document_with_resident_datasets_before_publish(
            &destination,
            &source_document,
            &loaded.datasets,
            || {
                callback_seen.set(true);
                // The validated temporary exists, but its final no-replace link
                // is not visible until this callback succeeds.
                assert!(!destination.exists());
                assert_eq!(temporary_links(directory.path()).len(), 1);
                Err(ProjectArchiveV6SaveCopyError::PublicationFailed(
                    "injected post-reopen precommit failure".into(),
                ))
            },
        );

        assert!(callback_seen.get());
        assert!(matches!(
            result,
            Err(ProjectArchiveV6SaveCopyError::PublicationFailed(message))
                if message == "injected post-reopen precommit failure"
        ));
        assert!(!destination.exists());
        assert!(temporary_links(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert_eq!(sha256_bytes(&source_bytes), source_sha256);
    }

    #[cfg(windows)]
    #[test]
    fn opaque_layout_change_remains_rejected() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let destination = directory.path().join("opaque-layout-change.qpls");
        let (source_document, _, source_sha256) = write_schema6_fixture(&source);
        let mut candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();
        candidate.layouts.insert(
            "customer_owned_opaque_layout".to_owned(),
            serde_json::json!({ "changed": true }),
        );

        assert!(matches!(
            save_project_archive_v6_model_copy(&source, &source_sha256, &destination, &candidate,),
            Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged)
        ));
        assert!(!destination.exists());
        assert!(temporary_links(directory.path()).is_empty());
    }

    #[test]
    fn reserved_layout_lane_rejects_mismatched_model_ids() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let (source_document, _, _) = write_schema6_fixture(&source);
        let mut candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();
        candidate.layouts.insert(
            STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY.to_owned(),
            serde_json::json!({ "schema_version": 1, "models": {} }),
        );

        assert!(matches!(
            non_model_authority_is_unchanged(&source_document, &candidate),
            Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged)
        ));
    }

    #[test]
    fn reserved_layout_lane_rejects_malformed_nested_layout() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let (source_document, _, _) = write_schema6_fixture(&source);
        let mut candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();
        let model_id = candidate.models[0].model_id.clone();
        candidate.layouts.insert(
            STANDARD_SEM_MODEL_V4_DIAGRAM_LAYOUTS_V1_KEY.to_owned(),
            serde_json::json!({
                "schema_version": 1,
                "models": {
                    model_id.clone(): {
                        "schema_version": 1,
                        "model_id": model_id,
                        "diagram_layout": {
                            "diagramVersion": "sem_designer_v1",
                            "constructLayouts": {
                                "construct:1": { "x": 10, "y": 20, "invented": true }
                            },
                            "indicatorLayouts": {},
                            "edgeLayouts": {},
                            "diagramTheme": "smartpls_like",
                            "showGrid": true,
                            "layoutLocked": false
                        }
                    }
                }
            }),
        );

        assert!(matches!(
            non_model_authority_is_unchanged(&source_document, &candidate),
            Err(ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn existing_destination_is_never_opened_or_clobbered() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let destination = directory.path().join("existing.qpls");
        let (source_document, _, source_sha256) = write_schema6_fixture(&source);
        let candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();
        fs::write(&destination, b"customer-owned destination").unwrap();

        assert!(matches!(
            save_project_archive_v6_model_copy(
                &source,
                &source_sha256,
                &destination,
                &candidate,
            ),
            Err(ProjectArchiveV6SaveCopyError::DestinationExists(path)) if path == destination
        ));
        assert_eq!(
            fs::read(&destination).unwrap(),
            b"customer-owned destination"
        );
        assert!(temporary_links(directory.path()).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn cancellation_after_handle_creation_deletes_only_through_armed_file_identity() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let destination = directory.path().join("cancelled.qpls");
        let (source_document, _, source_sha256) = write_schema6_fixture(&source);
        let candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();
        let checks = Cell::new(0_u32);

        let result = save_project_archive_v6_model_copy_with_control(
            &source,
            &source_sha256,
            &destination,
            &candidate,
            || {
                let next = checks.get() + 1;
                checks.set(next);
                next >= 3
            },
        );

        assert!(matches!(
            result,
            Err(ProjectArchiveV6SaveCopyError::CancelledBeforeCommit)
        ));
        assert!(checks.get() >= 3);
        assert!(!destination.exists());
        assert!(temporary_links(directory.path()).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn write_failure_removes_temporary_identity_and_never_publishes_final_link() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let destination = directory.path().join("write-failed.qpls");
        let (source_document, _, source_sha256) = write_schema6_fixture(&source);
        let source_bytes = fs::read(&source).unwrap();
        let candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();

        let result = save_copy_windows_with_hooks(
            &source,
            &source_sha256,
            &destination,
            &candidate,
            &mut || false,
            |_| {
                Err(ProjectArchiveV6SaveCopyError::Io(std::io::Error::other(
                    "injected write failure",
                )))
            },
            |_| Ok(()),
        );

        assert!(matches!(result, Err(ProjectArchiveV6SaveCopyError::Io(_))));
        assert!(!destination.exists());
        assert!(temporary_links(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[cfg(windows)]
    #[test]
    fn strict_reopen_failure_removes_temporary_identity_and_never_publishes_final_link() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source-v6.qpls");
        let destination = directory.path().join("strict-reopen-failed.qpls");
        let (source_document, _, source_sha256) = write_schema6_fixture(&source);
        let source_bytes = fs::read(&source).unwrap();
        let candidate = insert_sem_model_v4_draft_v6(&source_document, draft_model()).unwrap();

        let result = save_copy_windows_with_hooks(
            &source,
            &source_sha256,
            &destination,
            &candidate,
            &mut || false,
            |_| Ok(()),
            |file| {
                file.set_len(8)?;
                file.seek(SeekFrom::Start(0))?;
                file.write_all(b"not-a-zip")?;
                file.sync_all()?;
                Ok(())
            },
        );

        assert!(matches!(
            result,
            Err(ProjectArchiveV6SaveCopyError::Project(_))
        ));
        assert!(!destination.exists());
        assert!(temporary_links(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }
}
