//! Atomic native publication for verified CanonicalResultDocumentV2 exports.
//!
//! The frontend cross-format dispatcher owns semantic rendering and readback.
//! This module owns the final filesystem boundary: it binds the exact handed-
//! off payload to canonical provenance, stages bytes exclusively beside a new
//! destination, synchronizes them, and performs a no-replace atomic publish.

use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fmt, fs,
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const PUBLICATION_SCHEMA_VERSION_V2: u32 = 2;
const MAX_HANDOFF_BYTES_V2: usize = 64 * 1024 * 1024;
const MAX_PUBLISHED_BYTES_V2: usize = 128 * 1024 * 1024;
const MAX_WORKBOOK_TABLES_V2: usize = 256;
const MAX_EXCEL_COLUMNS_V2: usize = 16_384;
const MAX_EXCEL_ROWS_V2: usize = 1_048_571;
const MAX_EXCEL_CELL_UTF16_UNITS_V2: usize = 32_767;
const XLSX_MANIFEST_TABLE_ID_V2: &str = "quickpls_export_manifest_v2";
const XLSX_PROVENANCE_TABLE_ID_V2: &str = "quickpls_export_provenance_v2";
const TEMPORARY_FILE_PREFIX_V2: &str = ".quickpls-canonical-export-v2-";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CanonicalResultExportFormatV2 {
    Csv,
    Xlsx,
    Html,
    Pdf,
    Svg,
    Png,
}

impl CanonicalResultExportFormatV2 {
    fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Xlsx => "xlsx",
            Self::Html => "html",
            Self::Pdf => "pdf",
            Self::Svg => "svg",
            Self::Png => "png",
        }
    }

    fn is_workbook(self) -> bool {
        self == Self::Xlsx
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CanonicalResultExportIdentityV2 {
    document_id: String,
    run_id: String,
    project_id: String,
    model_id: String,
    model_digest: String,
    dataset_id: String,
    dataset_fingerprint: String,
    recipe_id: String,
    recipe_digest: String,
    capability_cell_id: String,
    method_version: String,
    engine_version: String,
    stable_table_ids: Vec<String>,
    stable_chart_ids: Vec<String>,
    semantic_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(crate) enum CanonicalResultExportPayloadV2 {
    #[serde(rename = "exact_bytes")]
    ExactBytes {
        #[serde(rename = "contentsBase64")]
        contents_base64: String,
        #[serde(rename = "byteLength")]
        byte_length: u64,
        sha256: String,
    },
    #[serde(rename = "xlsx_tables_json")]
    XlsxTablesJson {
        #[serde(rename = "tablesJson")]
        tables_json: String,
        #[serde(rename = "byteLength")]
        byte_length: u64,
        sha256: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CanonicalResultExportPublicationRequestV2 {
    schema_version: u32,
    format: CanonicalResultExportFormatV2,
    destination_path: String,
    identity: CanonicalResultExportIdentityV2,
    payload: CanonicalResultExportPayloadV2,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CanonicalResultExportPublicationReceiptV2 {
    schema_version: u32,
    format: CanonicalResultExportFormatV2,
    path: String,
    bytes: u64,
    sha256: String,
    payload_sha256: String,
    identity: CanonicalResultExportIdentityV2,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CanonicalWorkbookTableV2 {
    id: String,
    title: String,
    status: String,
    warning: Option<String>,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicationErrorV2 {
    code: &'static str,
    message: String,
}

impl PublicationErrorV2 {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for PublicationErrorV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)
    }
}

struct TemporaryPublicationGuardV2 {
    path: Option<PathBuf>,
}

impl TemporaryPublicationGuardV2 {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn path(&self) -> &Path {
        self.path
            .as_deref()
            .expect("armed temporary publication guard")
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TemporaryPublicationGuardV2 {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_lower_hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_nonempty(label: &str, value: &str) -> Result<(), PublicationErrorV2> {
    if value.is_empty() || value != value.trim() || value.chars().any(char::is_control) {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_IDENTITY_INVALID",
            format!("{label} must be nonempty and contain no surrounding whitespace or controls."),
        ));
    }
    Ok(())
}

fn validate_identity(
    format: CanonicalResultExportFormatV2,
    identity: &CanonicalResultExportIdentityV2,
) -> Result<(), PublicationErrorV2> {
    for (label, value) in [
        ("documentId", identity.document_id.as_str()),
        ("runId", identity.run_id.as_str()),
        ("projectId", identity.project_id.as_str()),
        ("modelId", identity.model_id.as_str()),
        ("datasetId", identity.dataset_id.as_str()),
        ("recipeId", identity.recipe_id.as_str()),
        ("capabilityCellId", identity.capability_cell_id.as_str()),
        ("methodVersion", identity.method_version.as_str()),
        ("engineVersion", identity.engine_version.as_str()),
    ] {
        require_nonempty(label, value)?;
    }
    for (label, digest) in [
        ("modelDigest", identity.model_digest.as_str()),
        ("datasetFingerprint", identity.dataset_fingerprint.as_str()),
        ("recipeDigest", identity.recipe_digest.as_str()),
        ("semanticSha256", identity.semantic_sha256.as_str()),
    ] {
        if !is_lower_hex_sha256(digest) {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_IDENTITY_INVALID",
                format!("{label} must be a lowercase SHA-256 digest."),
            ));
        }
    }
    let validate_ids = |label: &str, ids: &[String]| -> Result<(), PublicationErrorV2> {
        let mut unique = BTreeSet::new();
        for id in ids {
            require_nonempty(label, id)?;
            if !unique.insert(id) {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_IDENTITY_INVALID",
                    format!("{label} contains duplicate stable ID {id}."),
                ));
            }
        }
        Ok(())
    };
    validate_ids("stableTableIds", &identity.stable_table_ids)?;
    validate_ids("stableChartIds", &identity.stable_chart_ids)?;

    let selection_valid = match format {
        CanonicalResultExportFormatV2::Csv | CanonicalResultExportFormatV2::Xlsx => {
            !identity.stable_table_ids.is_empty() && identity.stable_chart_ids.is_empty()
        }
        CanonicalResultExportFormatV2::Svg | CanonicalResultExportFormatV2::Png => {
            identity.stable_table_ids.is_empty() && identity.stable_chart_ids.len() == 1
        }
        CanonicalResultExportFormatV2::Html | CanonicalResultExportFormatV2::Pdf => {
            !identity.stable_table_ids.is_empty() || !identity.stable_chart_ids.is_empty()
        }
    };
    if !selection_valid {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_SELECTION_INVALID",
            "Stable table/chart IDs do not match the selected export format.",
        ));
    }
    Ok(())
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_base64_v2(value: &str) -> Result<Vec<u8>, PublicationErrorV2> {
    let encoded = value.as_bytes();
    let maximum_encoded = MAX_HANDOFF_BYTES_V2.div_ceil(3) * 4;
    if encoded.is_empty() || encoded.len() > maximum_encoded || encoded.len() % 4 != 0 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_PAYLOAD_INVALID",
            "The exact-byte payload is empty, oversized, or not canonical base64.",
        ));
    }
    let mut decoded = Vec::with_capacity((encoded.len() / 4) * 3);
    for (chunk_index, chunk) in encoded.chunks_exact(4).enumerate() {
        let last = chunk_index + 1 == encoded.len() / 4;
        let first = base64_value(chunk[0]);
        let second = base64_value(chunk[1]);
        let third = base64_value(chunk[2]);
        let fourth = base64_value(chunk[3]);
        let (Some(first), Some(second)) = (first, second) else {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_PAYLOAD_INVALID",
                "The exact-byte payload is not canonical base64.",
            ));
        };
        decoded.push((first << 2) | (second >> 4));
        match (chunk[2], chunk[3], third, fourth) {
            (b'=', b'=', None, None) if last && second & 0x0f == 0 => {}
            (_, b'=', Some(third), None) if last && third & 0x03 == 0 => {
                decoded.push((second << 4) | (third >> 2));
            }
            (_, _, Some(third), Some(fourth)) => {
                decoded.push((second << 4) | (third >> 2));
                decoded.push((third << 6) | fourth);
            }
            _ => {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "The exact-byte payload has invalid or noncanonical padding.",
                ));
            }
        }
    }
    if decoded.len() > MAX_HANDOFF_BYTES_V2 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_PAYLOAD_INVALID",
            "The decoded exact-byte payload exceeds the native export limit.",
        ));
    }
    Ok(decoded)
}

fn validate_exact_format_bytes(
    format: CanonicalResultExportFormatV2,
    bytes: &[u8],
) -> Result<(), PublicationErrorV2> {
    match format {
        CanonicalResultExportFormatV2::Csv => {
            std::str::from_utf8(bytes).map_err(|_| {
                PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "CSV publication requires exact UTF-8 text bytes.",
                )
            })?;
        }
        CanonicalResultExportFormatV2::Html => {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "HTML publication requires exact UTF-8 text bytes.",
                )
            })?;
            if !text
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("<!doctype html")
            {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "HTML publication requires a self-contained HTML document.",
                ));
            }
        }
        CanonicalResultExportFormatV2::Svg => {
            let text = std::str::from_utf8(bytes).map_err(|_| {
                PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "SVG publication requires exact UTF-8 text bytes.",
                )
            })?;
            if !text.contains("<svg") || !text.contains("</svg>") {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "SVG publication requires a complete SVG document.",
                ));
            }
        }
        CanonicalResultExportFormatV2::Pdf => {
            if !bytes.starts_with(b"%PDF-") || !bytes.ends_with(b"%%EOF\n") {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "PDF publication requires a complete PDF byte stream.",
                ));
            }
        }
        CanonicalResultExportFormatV2::Png => {
            const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
            const IEND: &[u8] = b"\x00\x00\x00\x00IEND\xaeB\x60\x82";
            if !bytes.starts_with(SIGNATURE) || !bytes.ends_with(IEND) {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "PNG publication requires a complete PNG byte stream.",
                ));
            }
        }
        CanonicalResultExportFormatV2::Xlsx => {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_PAYLOAD_KIND_MISMATCH",
                "XLSX publication requires the string-only workbook-table payload.",
            ));
        }
    }
    Ok(())
}

fn validate_declared_payload(
    bytes: &[u8],
    byte_length: u64,
    expected_sha256: &str,
) -> Result<(), PublicationErrorV2> {
    if !is_lower_hex_sha256(expected_sha256) {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_PAYLOAD_INVALID",
            "The payload SHA-256 must be a lowercase digest.",
        ));
    }
    if bytes.len() as u64 != byte_length {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_PAYLOAD_LENGTH_MISMATCH",
            "The payload byte length differs from its declared length.",
        ));
    }
    if sha256_hex(bytes) != expected_sha256 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_PAYLOAD_SHA256_MISMATCH",
            "The payload bytes differ from their declared SHA-256 digest.",
        ));
    }
    Ok(())
}

fn workbook_row_value<'a>(table: &'a CanonicalWorkbookTableV2, key: &str) -> Option<&'a str> {
    table.rows.iter().find_map(|row| {
        (row.first().map(String::as_str) == Some(key))
            .then(|| row.get(1).map(String::as_str))
            .flatten()
    })
}

fn validate_workbook_tables(
    tables: &[CanonicalWorkbookTableV2],
    identity: &CanonicalResultExportIdentityV2,
) -> Result<(), PublicationErrorV2> {
    if tables.len() < 3 || tables.len() > MAX_WORKBOOK_TABLES_V2 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_XLSX_INVALID",
            "The canonical workbook has an invalid table count.",
        ));
    }
    let mut expected_ids = Vec::with_capacity(identity.stable_table_ids.len() + 2);
    expected_ids.push(XLSX_MANIFEST_TABLE_ID_V2.to_string());
    expected_ids.extend(identity.stable_table_ids.iter().cloned());
    expected_ids.push(XLSX_PROVENANCE_TABLE_ID_V2.to_string());
    let actual_ids = tables
        .iter()
        .map(|table| table.id.clone())
        .collect::<Vec<_>>();
    if actual_ids != expected_ids {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_XLSX_IDENTITY_MISMATCH",
            "Workbook table order or stable IDs differ from the verified selection.",
        ));
    }
    for table in tables {
        require_nonempty("workbook table ID", &table.id)?;
        require_nonempty("workbook table title", &table.title)?;
        if !matches!(table.status.as_str(), "validated" | "experimental") {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_INVALID",
                "Workbook status must be validated or experimental.",
            ));
        }
        if table.columns.is_empty()
            || table.columns.len() > MAX_EXCEL_COLUMNS_V2
            || table.rows.len() > MAX_EXCEL_ROWS_V2
            || table
                .rows
                .iter()
                .any(|row| row.len() != table.columns.len())
        {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_INVALID",
                "Workbook rows must exactly match a bounded nonempty column inventory.",
            ));
        }
        let strings = std::iter::once(&table.id)
            .chain(std::iter::once(&table.title))
            .chain(table.warning.iter())
            .chain(table.columns.iter())
            .chain(table.rows.iter().flatten());
        if strings
            .into_iter()
            .any(|value| value.encode_utf16().count() > MAX_EXCEL_CELL_UTF16_UNITS_V2)
        {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_INVALID",
                "A workbook string exceeds Excel's cell text limit.",
            ));
        }
    }

    let manifest = &tables[0];
    let provenance = tables
        .last()
        .expect("validated canonical workbook table inventory");
    let selected_table_ids = identity.stable_table_ids.join("\u{001f}");
    let selected_chart_ids = identity.stable_chart_ids.join("\u{001f}");
    for (table, key, expected) in [
        (manifest, "document_id", identity.document_id.as_str()),
        (
            manifest,
            "semantic_sha256",
            identity.semantic_sha256.as_str(),
        ),
        (manifest, "selected_table_ids", selected_table_ids.as_str()),
        (manifest, "selected_chart_ids", selected_chart_ids.as_str()),
        (provenance, "document_id", identity.document_id.as_str()),
        (provenance, "run_id", identity.run_id.as_str()),
        (provenance, "project_id", identity.project_id.as_str()),
        (provenance, "model_id", identity.model_id.as_str()),
        (provenance, "model_digest", identity.model_digest.as_str()),
        (provenance, "dataset_id", identity.dataset_id.as_str()),
        (
            provenance,
            "dataset_fingerprint",
            identity.dataset_fingerprint.as_str(),
        ),
        (provenance, "recipe_id", identity.recipe_id.as_str()),
        (provenance, "recipe_digest", identity.recipe_digest.as_str()),
        (
            provenance,
            "capability_cell_id",
            identity.capability_cell_id.as_str(),
        ),
        (
            provenance,
            "method_version",
            identity.method_version.as_str(),
        ),
        (
            provenance,
            "engine_version",
            identity.engine_version.as_str(),
        ),
    ] {
        if workbook_row_value(table, key) != Some(expected) {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_IDENTITY_MISMATCH",
                format!("Workbook field {key} differs from the verified export identity."),
            ));
        }
    }
    Ok(())
}

fn safe_sheet_name(title: &str, index: usize) -> String {
    let cleaned = title
        .chars()
        .map(|character| match character {
            ':' | '\\' | '/' | '?' | '*' | '[' | ']' => ' ',
            other => other,
        })
        .collect::<String>();
    let trimmed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let fallback = format!("Table {}", index + 1);
    let name = if trimmed.is_empty() {
        fallback
    } else {
        trimmed
    };
    name.chars()
        .take(31)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn unique_sheet_name(title: &str, index: usize, used: &mut BTreeSet<String>) -> String {
    let base = safe_sheet_name(title, index);
    let mut ordinal = 1usize;
    loop {
        let candidate = if ordinal == 1 {
            base.clone()
        } else {
            let suffix = format!(" ({ordinal})");
            let prefix_length = 31usize.saturating_sub(suffix.chars().count());
            format!(
                "{}{}",
                base.chars().take(prefix_length).collect::<String>(),
                suffix
            )
        };
        if used.insert(candidate.to_lowercase()) {
            return candidate;
        }
        ordinal += 1;
    }
}

fn build_workbook_bytes(
    tables: &[CanonicalWorkbookTableV2],
) -> Result<Vec<u8>, PublicationErrorV2> {
    let mut workbook = Workbook::new();
    let mut used_sheet_names = BTreeSet::new();
    for (index, table) in tables.iter().enumerate() {
        let worksheet = workbook.add_worksheet();
        let sheet_name = unique_sheet_name(&table.title, index, &mut used_sheet_names);
        worksheet.set_name(&sheet_name).map_err(|error| {
            PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                format!("Workbook sheet naming failed: {error}"),
            )
        })?;
        worksheet
            .write_string(0, 0, &table.title)
            .map_err(|error| {
                PublicationErrorV2::new(
                    "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                    format!("Workbook title write failed: {error}"),
                )
            })?;
        worksheet.write_string(1, 0, "Status").map_err(|error| {
            PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                format!("Workbook status label write failed: {error}"),
            )
        })?;
        worksheet
            .write_string(1, 1, &table.status)
            .map_err(|error| {
                PublicationErrorV2::new(
                    "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                    format!("Workbook status write failed: {error}"),
                )
            })?;
        worksheet.write_string(2, 0, "Warning").map_err(|error| {
            PublicationErrorV2::new(
                "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                format!("Workbook warning label write failed: {error}"),
            )
        })?;
        worksheet
            .write_string(2, 1, table.warning.as_deref().unwrap_or(""))
            .map_err(|error| {
                PublicationErrorV2::new(
                    "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                    format!("Workbook warning write failed: {error}"),
                )
            })?;
        for (column, header) in table.columns.iter().enumerate() {
            worksheet
                .write_string(4, column as u16, header)
                .map_err(|error| {
                    PublicationErrorV2::new(
                        "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                        format!("Workbook column write failed: {error}"),
                    )
                })?;
        }
        for (row_index, row) in table.rows.iter().enumerate() {
            for (column, value) in row.iter().enumerate() {
                // Deliberately use write_string for every value. Canonical
                // labels such as "=1+1" must never become formulas or numbers.
                worksheet
                    .write_string((row_index + 5) as u32, column as u16, value)
                    .map_err(|error| {
                        PublicationErrorV2::new(
                            "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
                            format!("Workbook cell write failed: {error}"),
                        )
                    })?;
            }
        }
        worksheet.autofit();
    }
    let bytes = workbook.save_to_buffer().map_err(|error| {
        PublicationErrorV2::new(
            "CANONICAL_EXPORT_XLSX_BUILD_FAILED",
            format!("Workbook finalization failed: {error}"),
        )
    })?;
    if bytes.len() > MAX_PUBLISHED_BYTES_V2 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_OUTPUT_TOO_LARGE",
            "The generated XLSX exceeds the native publication limit.",
        ));
    }
    Ok(bytes)
}

fn materialize_payload(
    format: CanonicalResultExportFormatV2,
    identity: &CanonicalResultExportIdentityV2,
    payload: &CanonicalResultExportPayloadV2,
) -> Result<(Vec<u8>, String), PublicationErrorV2> {
    match payload {
        CanonicalResultExportPayloadV2::ExactBytes {
            contents_base64,
            byte_length,
            sha256,
        } => {
            if format.is_workbook() {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_KIND_MISMATCH",
                    "XLSX publication requires the string-only workbook-table payload.",
                ));
            }
            let bytes = decode_base64_v2(contents_base64)?;
            validate_declared_payload(&bytes, *byte_length, sha256)?;
            validate_exact_format_bytes(format, &bytes)?;
            Ok((bytes, sha256.clone()))
        }
        CanonicalResultExportPayloadV2::XlsxTablesJson {
            tables_json,
            byte_length,
            sha256,
        } => {
            if !format.is_workbook() {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_KIND_MISMATCH",
                    "Only XLSX publication accepts workbook tables.",
                ));
            }
            let handoff = tables_json.as_bytes();
            if handoff.is_empty() || handoff.len() > MAX_HANDOFF_BYTES_V2 {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_PAYLOAD_INVALID",
                    "The workbook-table handoff is empty or exceeds the native export limit.",
                ));
            }
            validate_declared_payload(handoff, *byte_length, sha256)?;
            let tables = serde_json::from_str::<Vec<CanonicalWorkbookTableV2>>(tables_json)
                .map_err(|error| {
                    PublicationErrorV2::new(
                        "CANONICAL_EXPORT_XLSX_INVALID",
                        format!("The string-only workbook-table payload is invalid: {error}"),
                    )
                })?;
            validate_workbook_tables(&tables, identity)?;
            Ok((build_workbook_bytes(&tables)?, sha256.clone()))
        }
    }
}

fn validate_destination(
    format: CanonicalResultExportFormatV2,
    destination_path: &str,
) -> Result<PathBuf, PublicationErrorV2> {
    if destination_path.is_empty() || destination_path != destination_path.trim() {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_INVALID",
            "The Save dialog must provide a nonempty path without surrounding whitespace.",
        ));
    }
    let requested = Path::new(destination_path);
    if !requested.is_absolute() {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_INVALID",
            "Canonical exports require an absolute path selected by the Save dialog.",
        ));
    }
    #[cfg(windows)]
    if destination_path.starts_with("\\\\") {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_UNSUPPORTED",
            "Canonical exports require a local destination directory.",
        ));
    }
    let extension = requested
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if extension.as_deref() != Some(format.extension()) {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_EXTENSION_MISMATCH",
            format!(
                "The selected destination must end in .{} for this export.",
                format.extension()
            ),
        ));
    }
    let file_name = requested.file_name().ok_or_else(|| {
        PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_INVALID",
            "The selected destination has no file name.",
        )
    })?;
    let requested_parent = requested.parent().ok_or_else(|| {
        PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_INVALID",
            "The selected destination has no parent directory.",
        )
    })?;
    let parent_metadata = fs::symlink_metadata(requested_parent).map_err(|_| {
        PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_UNAVAILABLE",
            "The selected export directory is unavailable.",
        )
    })?;
    if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_UNSUPPORTED",
            "The selected export parent must be a regular local directory, not a link.",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if parent_metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_DESTINATION_UNSUPPORTED",
                "The selected export parent cannot be a Windows reparse point.",
            ));
        }
    }
    let canonical_parent = fs::canonicalize(requested_parent).map_err(|_| {
        PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_UNAVAILABLE",
            "The selected export directory could not be resolved.",
        )
    })?;
    let destination = canonical_parent.join(file_name);
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_DESTINATION_EXISTS",
                "The selected file already exists and was not replaced.",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(PublicationErrorV2::new(
                "CANONICAL_EXPORT_DESTINATION_UNAVAILABLE",
                "The selected destination could not be inspected safely.",
            ));
        }
    }
    Ok(destination)
}

fn create_exclusive_temporary(
    destination: &Path,
) -> Result<(File, TemporaryPublicationGuardV2), PublicationErrorV2> {
    let parent = destination.parent().expect("validated destination parent");
    for _ in 0..16 {
        let temporary = parent.join(format!("{TEMPORARY_FILE_PREFIX_V2}{}.tmp", Uuid::new_v4()));
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
            options
                .share_mode(0)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        }
        match options.open(&temporary) {
            Ok(file) => return Ok((file, TemporaryPublicationGuardV2::new(temporary))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                return Err(PublicationErrorV2::new(
                    "CANONICAL_EXPORT_TEMP_CREATE_FAILED",
                    "QuickPLS could not create an exclusive temporary export beside the destination.",
                ));
            }
        }
    }
    Err(PublicationErrorV2::new(
        "CANONICAL_EXPORT_TEMP_CREATE_FAILED",
        "QuickPLS could not reserve a unique temporary export name.",
    ))
}

#[cfg(windows)]
fn publish_no_replace(temporary: &Path, destination: &Path) -> Result<(), PublicationErrorV2> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};

    let temporary_wide = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe {
        MoveFileExW(
            temporary_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_ATOMIC_PUBLISH_FAILED",
            "The synchronized export could not be atomically published; an existing file was never replaced.",
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn publish_no_replace(temporary: &Path, destination: &Path) -> Result<(), PublicationErrorV2> {
    fs::hard_link(temporary, destination).map_err(|_| {
        PublicationErrorV2::new(
            "CANONICAL_EXPORT_ATOMIC_PUBLISH_FAILED",
            "The synchronized export could not be atomically published without replacing a file.",
        )
    })?;
    if fs::remove_file(temporary).is_err() {
        let _ = fs::remove_file(destination);
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_ATOMIC_PUBLISH_FAILED",
            "The temporary export could not be removed after no-replace publication.",
        ));
    }
    Ok(())
}

fn publish_with_hooks<Cancelled, BeforePublish>(
    request: CanonicalResultExportPublicationRequestV2,
    cancelled_before_write: Cancelled,
    before_publish: BeforePublish,
) -> Result<CanonicalResultExportPublicationReceiptV2, PublicationErrorV2>
where
    Cancelled: FnOnce() -> bool,
    BeforePublish: FnOnce(&Path) -> Result<(), PublicationErrorV2>,
{
    if request.schema_version != PUBLICATION_SCHEMA_VERSION_V2 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_SCHEMA_UNSUPPORTED",
            "Only canonical export publication schema version 2 is supported.",
        ));
    }
    validate_identity(request.format, &request.identity)?;
    let (published_bytes, payload_sha256) =
        materialize_payload(request.format, &request.identity, &request.payload)?;
    if published_bytes.is_empty() || published_bytes.len() > MAX_PUBLISHED_BYTES_V2 {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_OUTPUT_TOO_LARGE",
            "The final export is empty or exceeds the native publication limit.",
        ));
    }
    if cancelled_before_write() {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_CANCELLED_BEFORE_WRITE",
            "Export publication was cancelled before any filesystem write.",
        ));
    }
    let destination = validate_destination(request.format, &request.destination_path)?;
    let published_sha256 = sha256_hex(&published_bytes);
    let receipt = CanonicalResultExportPublicationReceiptV2 {
        schema_version: PUBLICATION_SCHEMA_VERSION_V2,
        format: request.format,
        path: request.destination_path.clone(),
        bytes: published_bytes.len() as u64,
        sha256: published_sha256,
        payload_sha256,
        identity: request.identity,
    };

    let (mut temporary_file, mut temporary_guard) = create_exclusive_temporary(&destination)?;
    let write_result = temporary_file
        .write_all(&published_bytes)
        .and_then(|_| temporary_file.flush())
        .and_then(|_| temporary_file.sync_all());
    drop(temporary_file);
    if write_result.is_err() {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_WRITE_FAILED",
            "QuickPLS could not fully write and synchronize the temporary export.",
        ));
    }
    before_publish(temporary_guard.path())?;
    if destination.exists() {
        return Err(PublicationErrorV2::new(
            "CANONICAL_EXPORT_DESTINATION_EXISTS",
            "The selected destination appeared during export and was not replaced.",
        ));
    }
    publish_no_replace(temporary_guard.path(), &destination)?;
    temporary_guard.disarm();
    #[cfg(unix)]
    if let Some(parent) = destination.parent() {
        let _ = File::open(parent).and_then(|directory| directory.sync_all());
    }
    Ok(receipt)
}

fn publish(
    request: CanonicalResultExportPublicationRequestV2,
) -> Result<CanonicalResultExportPublicationReceiptV2, PublicationErrorV2> {
    publish_with_hooks(request, || false, |_| Ok(()))
}

#[tauri::command]
pub(crate) fn publish_canonical_result_export_v2(
    request: CanonicalResultExportPublicationRequestV2,
) -> Result<CanonicalResultExportPublicationReceiptV2, String> {
    publish(request).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{Reader, open_workbook_auto};
    use serde_json::json;

    fn base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let first = chunk[0];
            let second = *chunk.get(1).unwrap_or(&0);
            let third = *chunk.get(2).unwrap_or(&0);
            let combined = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;
            output.push(ALPHABET[((combined >> 18) & 63) as usize] as char);
            output.push(ALPHABET[((combined >> 12) & 63) as usize] as char);
            output.push(
                chunk
                    .get(1)
                    .map(|_| ALPHABET[((combined >> 6) & 63) as usize] as char)
                    .unwrap_or('='),
            );
            output.push(
                chunk
                    .get(2)
                    .map(|_| ALPHABET[(combined & 63) as usize] as char)
                    .unwrap_or('='),
            );
        }
        output
    }

    fn identity(tables: Vec<String>, charts: Vec<String>) -> CanonicalResultExportIdentityV2 {
        CanonicalResultExportIdentityV2 {
            document_id: "result.general-sem:export-v2".into(),
            run_id: "run:export-v2".into(),
            project_id: "project:export-v2".into(),
            model_id: "model:export-v2".into(),
            model_digest: "a".repeat(64),
            dataset_id: "dataset:export-v2".into(),
            dataset_fingerprint: "b".repeat(64),
            recipe_id: "recipe:export-v2".into(),
            recipe_digest: "c".repeat(64),
            capability_cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point".into(),
            method_version: "qpls.general-sem-pls.multiple-two-way.point.v1".into(),
            engine_version: "compiled_recipe_v4_pls_plan_v2_execution_v3".into(),
            stable_table_ids: tables,
            stable_chart_ids: charts,
            semantic_sha256: "d".repeat(64),
        }
    }

    fn exact_request(
        format: CanonicalResultExportFormatV2,
        path: PathBuf,
        bytes: &[u8],
    ) -> CanonicalResultExportPublicationRequestV2 {
        let (tables, charts) = match format {
            CanonicalResultExportFormatV2::Svg | CanonicalResultExportFormatV2::Png => {
                (vec![], vec!["interaction_plot".into()])
            }
            _ => (vec!["effects".into()], vec![]),
        };
        CanonicalResultExportPublicationRequestV2 {
            schema_version: 2,
            format,
            destination_path: path.to_string_lossy().into_owned(),
            identity: identity(tables, charts),
            payload: CanonicalResultExportPayloadV2::ExactBytes {
                contents_base64: base64(bytes),
                byte_length: bytes.len() as u64,
                sha256: sha256_hex(bytes),
            },
        }
    }

    fn workbook_tables(
        identity: &CanonicalResultExportIdentityV2,
    ) -> Vec<CanonicalWorkbookTableV2> {
        vec![
            CanonicalWorkbookTableV2 {
                id: XLSX_MANIFEST_TABLE_ID_V2.into(),
                title: XLSX_MANIFEST_TABLE_ID_V2.into(),
                status: "validated".into(),
                warning: Some("Machine-readable manifest".into()),
                columns: vec!["Field ID".into(), "Value".into()],
                rows: vec![
                    vec!["document_id".into(), identity.document_id.clone()],
                    vec!["semantic_sha256".into(), identity.semantic_sha256.clone()],
                    vec![
                        "selected_table_ids".into(),
                        identity.stable_table_ids.join("\u{001f}"),
                    ],
                    vec![
                        "selected_chart_ids".into(),
                        identity.stable_chart_ids.join("\u{001f}"),
                    ],
                ],
            },
            CanonicalWorkbookTableV2 {
                id: "effects".into(),
                title: "effects".into(),
                status: "validated".into(),
                warning: None,
                columns: vec!["Label".into(), "Value".into()],
                rows: vec![vec!["β interaction".into(), "=1+1".into()]],
            },
            CanonicalWorkbookTableV2 {
                id: XLSX_PROVENANCE_TABLE_ID_V2.into(),
                title: XLSX_PROVENANCE_TABLE_ID_V2.into(),
                status: "validated".into(),
                warning: None,
                columns: vec!["Field ID".into(), "Value".into()],
                rows: vec![
                    vec!["document_id".into(), identity.document_id.clone()],
                    vec!["run_id".into(), identity.run_id.clone()],
                    vec!["project_id".into(), identity.project_id.clone()],
                    vec!["model_id".into(), identity.model_id.clone()],
                    vec!["model_digest".into(), identity.model_digest.clone()],
                    vec!["dataset_id".into(), identity.dataset_id.clone()],
                    vec![
                        "dataset_fingerprint".into(),
                        identity.dataset_fingerprint.clone(),
                    ],
                    vec!["recipe_id".into(), identity.recipe_id.clone()],
                    vec!["recipe_digest".into(), identity.recipe_digest.clone()],
                    vec![
                        "capability_cell_id".into(),
                        identity.capability_cell_id.clone(),
                    ],
                    vec!["method_version".into(), identity.method_version.clone()],
                    vec!["engine_version".into(), identity.engine_version.clone()],
                ],
            },
        ]
    }

    fn workbook_request(path: PathBuf) -> CanonicalResultExportPublicationRequestV2 {
        let identity = identity(vec!["effects".into()], vec![]);
        let tables_json = serde_json::to_string(&workbook_tables(&identity)).unwrap();
        let byte_length = tables_json.len() as u64;
        let payload_sha256 = sha256_hex(tables_json.as_bytes());
        CanonicalResultExportPublicationRequestV2 {
            schema_version: 2,
            format: CanonicalResultExportFormatV2::Xlsx,
            destination_path: path.to_string_lossy().into_owned(),
            identity,
            payload: CanonicalResultExportPayloadV2::XlsxTablesJson {
                tables_json,
                byte_length,
                sha256: payload_sha256,
            },
        }
    }

    fn temporary_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(TEMPORARY_FILE_PREFIX_V2))
            })
            .collect()
    }

    #[test]
    fn publishes_all_exact_byte_formats_atomically_with_bound_receipts() {
        let directory = tempfile::tempdir().unwrap();
        let fixtures = [
            (
                CanonicalResultExportFormatV2::Csv,
                b"id,value\r\neffect,0.2\r\n".as_slice(),
            ),
            (
                CanonicalResultExportFormatV2::Html,
                b"<!doctype html><html><body>QuickPLS</body></html>".as_slice(),
            ),
            (
                CanonicalResultExportFormatV2::Pdf,
                b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\n%%EOF\n".as_slice(),
            ),
            (
                CanonicalResultExportFormatV2::Svg,
                b"<svg xmlns=\"http://www.w3.org/2000/svg\"><text>QuickPLS</text></svg>".as_slice(),
            ),
            (
                CanonicalResultExportFormatV2::Png,
                b"\x89PNG\r\n\x1a\n\x00\x00\x00\x00IEND\xaeB\x60\x82".as_slice(),
            ),
        ];
        for (format, bytes) in fixtures {
            let path = directory
                .path()
                .join(format!("result.{}", format.extension()));
            let receipt = publish(exact_request(format, path.clone(), bytes)).unwrap();
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(receipt.bytes, bytes.len() as u64);
            assert_eq!(receipt.sha256, sha256_hex(bytes));
            assert_eq!(receipt.payload_sha256, receipt.sha256);
            assert_eq!(receipt.identity.semantic_sha256, "d".repeat(64));
        }
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn publishes_string_only_xlsx_and_binds_manifest_provenance_and_stable_ids() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("result.xlsx");
        let request = workbook_request(path.clone());
        let expected_payload_sha256 = match &request.payload {
            CanonicalResultExportPayloadV2::XlsxTablesJson { sha256, .. } => sha256.clone(),
            _ => unreachable!(),
        };
        let receipt = publish(request).unwrap();

        assert_eq!(receipt.payload_sha256, expected_payload_sha256);
        assert_eq!(receipt.sha256, sha256_hex(&fs::read(&path).unwrap()));
        assert_eq!(receipt.identity.stable_table_ids, vec!["effects"]);
        let mut workbook = open_workbook_auto(&path).unwrap();
        let range = workbook.worksheet_range("effects").unwrap();
        assert_eq!(range.get((5, 0)).unwrap().to_string(), "β interaction");
        assert_eq!(range.get((5, 1)).unwrap().to_string(), "=1+1");
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn rejects_unknown_format_path_tamper_and_existing_destination_without_filesystem_drift() {
        let unknown = json!({
            "schemaVersion": 2,
            "format": "docx",
            "destinationPath": "D:\\exports\\result.docx",
            "identity": {},
            "payload": { "kind": "exact_bytes", "contentsBase64": "QQ==", "byteLength": 1, "sha256": "a".repeat(64) }
        });
        assert!(
            serde_json::from_value::<CanonicalResultExportPublicationRequestV2>(unknown).is_err()
        );

        let directory = tempfile::tempdir().unwrap();
        let wrong_extension = directory.path().join("result.pdf");
        let error = publish(exact_request(
            CanonicalResultExportFormatV2::Csv,
            wrong_extension.clone(),
            b"id,value\r\n",
        ))
        .unwrap_err();
        assert_eq!(
            error.code,
            "CANONICAL_EXPORT_DESTINATION_EXTENSION_MISMATCH"
        );
        assert!(!wrong_extension.exists());

        let existing = directory.path().join("result.csv");
        fs::write(&existing, b"owned-by-user").unwrap();
        let error = publish(exact_request(
            CanonicalResultExportFormatV2::Csv,
            existing.clone(),
            b"id,value\r\n",
        ))
        .unwrap_err();
        assert_eq!(error.code, "CANONICAL_EXPORT_DESTINATION_EXISTS");
        assert_eq!(fs::read(&existing).unwrap(), b"owned-by-user");
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn rejects_byte_length_digest_and_workbook_identity_tamper_before_writing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tampered.csv");
        let mut request = exact_request(
            CanonicalResultExportFormatV2::Csv,
            path.clone(),
            b"id,value\r\n",
        );
        let CanonicalResultExportPayloadV2::ExactBytes { byte_length, .. } = &mut request.payload
        else {
            unreachable!()
        };
        *byte_length += 1;
        let error = publish(request).unwrap_err();
        assert_eq!(error.code, "CANONICAL_EXPORT_PAYLOAD_LENGTH_MISMATCH");
        assert!(!path.exists());

        let mut request = exact_request(
            CanonicalResultExportFormatV2::Csv,
            path.clone(),
            b"id,value\r\n",
        );
        let CanonicalResultExportPayloadV2::ExactBytes { sha256, .. } = &mut request.payload else {
            unreachable!()
        };
        *sha256 = "f".repeat(64);
        let error = publish(request).unwrap_err();
        assert_eq!(error.code, "CANONICAL_EXPORT_PAYLOAD_SHA256_MISMATCH");

        let xlsx_path = directory.path().join("tampered.xlsx");
        let mut request = workbook_request(xlsx_path.clone());
        request.identity.stable_table_ids = vec!["different".into()];
        let error = publish(request).unwrap_err();
        assert_eq!(error.code, "CANONICAL_EXPORT_XLSX_IDENTITY_MISMATCH");
        assert!(!xlsx_path.exists());
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn writer_failure_after_sync_cleans_temporary_and_never_publishes_final() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("writer-failure.csv");
        let request = exact_request(
            CanonicalResultExportFormatV2::Csv,
            path.clone(),
            b"id,value\r\n",
        );
        let error = publish_with_hooks(
            request,
            || false,
            |_| {
                Err(PublicationErrorV2::new(
                    "TEST_WRITER_FAILURE",
                    "injected writer failure",
                ))
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "TEST_WRITER_FAILURE");
        assert!(!path.exists());
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn cancellation_before_write_leaves_no_final_or_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cancelled.csv");
        let request = exact_request(
            CanonicalResultExportFormatV2::Csv,
            path.clone(),
            b"id,value\r\n",
        );
        let error = publish_with_hooks(request, || true, |_| Ok(())).unwrap_err();
        assert_eq!(error.code, "CANONICAL_EXPORT_CANCELLED_BEFORE_WRITE");
        assert!(!path.exists());
        assert!(temporary_files(directory.path()).is_empty());
    }
}

