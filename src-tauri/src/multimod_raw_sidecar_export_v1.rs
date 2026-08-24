//! Strict, no-replace export of selected MultiMod Arrow evidence.
//!
//! This command never accepts arbitrary archive entries. It reopens the exact
//! expected schema-6 archive through the strict project loader, resolves one
//! descriptor from one completed result, and publishes only posterior,
//! membership, assignment, replicate-ledger, or target-vector evidence.

use crate::multimod_candidate_authority_v1::verify_multimod_candidate_receipt_against_embedded_v1;
use qpls_core::MultimodQualificationStateV1;
use qpls_project::{
    MultiModSidecarPayloadV1, load_project_archive_v6_from_file,
    read_multimod_sidecar_payload_from_file_v1, validate_multimod_sidecar_payload_v1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const RAW_SIDECAR_EXPORT_SCHEMA_VERSION_V1: u32 = 1;
const INTERNAL_LABS_SURFACE_V1: &str = "internal_labs_multimod_v1";
const TEMPORARY_PREFIX_V1: &str = ".quickpls-multimod-arrow-export-v1-";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModRawSidecarExportRequestV1 {
    schema_version: u32,
    surface: String,
    experimental_labs_enabled: bool,
    archive_path: String,
    expected_archive_sha256: String,
    project_id: String,
    result_id: String,
    entry_name: String,
    expected_identity_sha256: String,
    expected_payload_sha256: String,
    destination_path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModRawSidecarExportReceiptV1 {
    schema_version: u32,
    archive_sha256: String,
    project_id: String,
    result_id: String,
    entry_name: String,
    identity_sha256: String,
    path: String,
    bytes: u64,
    sha256: String,
    strict_reopen_validated: bool,
    no_replace_publication: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MultiModRawSidecarExportErrorV1 {
    code: &'static str,
    message: String,
    corrective_action: String,
}

impl MultiModRawSidecarExportErrorV1 {
    fn new(
        code: &'static str,
        message: impl Into<String>,
        corrective_action: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            corrective_action: corrective_action.into(),
        }
    }
}

struct TemporaryGuardV1(Option<PathBuf>);

impl TemporaryGuardV1 {
    fn path(&self) -> &Path {
        self.0.as_deref().expect("armed MultiMod export guard")
    }

    fn disarm(&mut self) {
        self.0 = None;
    }
}

impl Drop for TemporaryGuardV1 {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = fs::remove_file(path);
        }
    }
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn hash_open_file(file: &mut File) -> Result<String, MultiModRawSidecarExportErrorV1> {
    file.seek(SeekFrom::Start(0)).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.archive_unavailable",
            "The selected project archive could not be positioned for verification.",
            "Reopen the project and retry from the completed result.",
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| {
            MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.archive_unavailable",
                "The selected project archive could not be read completely.",
                "Reopen the project and retry from the completed result.",
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    file.seek(SeekFrom::Start(0)).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.archive_unavailable",
            "The selected project archive could not be reset for strict reopen.",
            "Reopen the project and retry from the completed result.",
        )
    })?;
    Ok(format!("{:x}", digest.finalize()))
}

fn exportable_entry(result_id: &str, entry_name: &str) -> bool {
    let prefix = format!("results/{result_id}/");
    let Some(leaf) = entry_name.strip_prefix(&prefix) else {
        return false;
    };
    if leaf.is_empty() || leaf.contains('/') || leaf.contains('\\') || leaf.contains("..") {
        return false;
    }
    [
        "-posteriors.arrow",
        "-memberships.arrow",
        "-assignments.arrow",
        "-hard-assignments.arrow",
        "-ledger.arrow",
        "-target-vectors.arrow",
        "-draw-rows.arrow",
        "-records.arrow",
        "-counts.arrow",
        "-usable-indices.arrow",
    ]
    .iter()
    .any(|suffix| leaf.ends_with(suffix))
}

fn exportable_evidence_role(evidence_role: &str) -> bool {
    let Some((_, table_role)) = evidence_role.split_once(':') else {
        return false;
    };
    [
        "posteriors",
        "memberships",
        "assignments",
        "hard-assignments",
        "ledger",
        "target-vectors",
        "draw-rows",
        "records",
        "counts",
        "usable-indices",
    ]
    .iter()
    .any(|suffix| table_role.ends_with(suffix))
}

fn validate_destination(value: &str) -> Result<PathBuf, MultiModRawSidecarExportErrorV1> {
    if value.is_empty() || value != value.trim() {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_invalid",
            "The Save dialog must provide one absolute .arrow path without surrounding whitespace.",
            "Choose a new local .arrow destination.",
        ));
    }
    let requested = Path::new(value);
    if !requested.is_absolute()
        || requested
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("arrow"))
    {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_invalid",
            "Raw evidence export requires an absolute .arrow destination selected by the Save dialog.",
            "Choose a new local .arrow destination.",
        ));
    }
    #[cfg(windows)]
    if value.starts_with("\\\\") {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_unsupported",
            "Raw evidence export requires a local destination directory.",
            "Choose a directory on a local drive.",
        ));
    }
    let parent = requested.parent().ok_or_else(|| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_invalid",
            "The selected destination has no parent directory.",
            "Choose a new local .arrow destination.",
        )
    })?;
    let metadata = fs::symlink_metadata(parent).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_unavailable",
            "The selected destination directory is unavailable.",
            "Choose an existing writable local directory.",
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_unsupported",
            "The selected destination parent must be a regular local directory, not a link.",
            "Choose an existing writable local directory.",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.destination_unsupported",
                "The selected destination parent cannot be a Windows reparse point.",
                "Choose a regular local directory.",
            ));
        }
    }
    let parent = fs::canonicalize(parent).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_unavailable",
            "The selected destination directory could not be resolved.",
            "Choose an existing writable local directory.",
        )
    })?;
    let destination = parent.join(requested.file_name().expect("validated destination file"));
    match fs::symlink_metadata(&destination) {
        Ok(_) => Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_exists",
            "The selected file already exists and was not replaced.",
            "Choose a new destination name.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(destination),
        Err(_) => Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_unavailable",
            "The selected destination could not be inspected safely.",
            "Choose a new local .arrow destination.",
        )),
    }
}

fn create_temporary(
    destination: &Path,
) -> Result<(File, TemporaryGuardV1), MultiModRawSidecarExportErrorV1> {
    let parent = destination.parent().expect("validated destination parent");
    for _ in 0..16 {
        let path = parent.join(format!("{TEMPORARY_PREFIX_V1}{}.tmp", Uuid::new_v4()));
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
        match options.open(&path) {
            Ok(file) => return Ok((file, TemporaryGuardV1(Some(path)))),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => break,
        }
    }
    Err(MultiModRawSidecarExportErrorV1::new(
        "multimod.raw_export.temporary_create_failed",
        "QuickPLS could not create an exclusive temporary file beside the destination.",
        "Choose another writable local directory.",
    ))
}

fn validate_published_destination_v1(
    destination: &Path,
    descriptor: &qpls_core::MultimodResultSidecarDescriptorV1,
    result_id: &str,
) -> Result<u64, MultiModRawSidecarExportErrorV1> {
    let metadata = fs::symlink_metadata(destination).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_readback_failed",
            "The published Arrow destination could not be inspected.",
            "Preserve the file and source archive for diagnosis; do not rely on this export.",
        )
    })?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_readback_failed",
            "The published Arrow destination is not a regular file.",
            "Preserve the source archive and choose a regular local destination.",
        ));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.destination_readback_failed",
                "The published Arrow destination resolved to a Windows reparse point.",
                "Preserve the source archive and choose a regular local destination.",
            ));
        }
    }
    if metadata.len() != descriptor.uncompressed_bytes {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_readback_failed",
            "The published Arrow destination length differs from its strict descriptor.",
            "Preserve the file and source archive for diagnosis; do not rely on this export.",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options
            .share_mode(0)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let mut published = options.open(destination).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_readback_failed",
            "The published Arrow destination could not be opened exclusively for readback.",
            "Preserve the file and source archive for diagnosis; do not rely on this export.",
        )
    })?;
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    published.read_to_end(&mut bytes).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_readback_failed",
            "The published Arrow destination could not be read completely.",
            "Preserve the file and source archive for diagnosis; do not rely on this export.",
        )
    })?;
    let payload = MultiModSidecarPayloadV1 {
        descriptor: descriptor.clone(),
        bytes,
    };
    validate_multimod_sidecar_payload_v1(result_id, &payload).map_err(|error| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_readback_failed",
            format!("The published Arrow destination failed strict schema, shape, digest, or identity readback: {error}"),
            "Preserve the file and source archive for diagnosis; do not rely on this export.",
        )
    })?;
    Ok(metadata.len())
}

#[cfg(windows)]
fn publish_no_replace(
    temporary: &Path,
    destination: &Path,
) -> Result<(), MultiModRawSidecarExportErrorV1> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MOVEFILE_WRITE_THROUGH, MoveFileExW};
    let source = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let target = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let moved = unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_WRITE_THROUGH) };
    if moved == 0 {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.atomic_publish_failed",
            "The synchronized Arrow evidence could not be atomically published; an existing file was never replaced.",
            "Choose a new destination and retry.",
        ));
    }
    Ok(())
}

fn publish_strictly_validated_payload_v1(
    result_id: &str,
    payload: &MultiModSidecarPayloadV1,
    destination_path: &str,
) -> Result<(PathBuf, u64), MultiModRawSidecarExportErrorV1> {
    validate_multimod_sidecar_payload_v1(result_id, payload).map_err(|error| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.payload_mismatch",
            format!("The selected Arrow payload failed strict schema, shape, digest, or identity readback: {error}"),
            "Do not export this payload; preserve the archive for diagnosis.",
        )
    })?;
    let bytes = &payload.bytes;
    let descriptor = &payload.descriptor;
    if bytes.len() as u64 != descriptor.uncompressed_bytes
        || sha256_bytes(bytes) != descriptor.sha256
    {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.payload_mismatch",
            "The strict payload bytes differ from their descriptor.",
            "Do not export this payload; preserve the archive for diagnosis.",
        ));
    }
    let destination = validate_destination(destination_path)?;
    let (mut file, mut guard) = create_temporary(&destination)?;
    file.write_all(bytes)
        .and_then(|_| file.flush())
        .and_then(|_| file.sync_all())
        .map_err(|_| {
            MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.write_failed",
                "QuickPLS could not fully write and synchronize the Arrow evidence.",
                "Choose another writable local directory.",
            )
        })?;
    drop(file);
    if destination.exists() {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.destination_exists",
            "The selected destination appeared during export and was not replaced.",
            "Choose a new destination name.",
        ));
    }
    publish_no_replace(guard.path(), &destination)?;
    guard.disarm();
    let published_bytes = validate_published_destination_v1(&destination, descriptor, result_id)?;
    Ok((destination, published_bytes))
}

#[cfg(not(windows))]
fn publish_no_replace(
    temporary: &Path,
    destination: &Path,
) -> Result<(), MultiModRawSidecarExportErrorV1> {
    fs::hard_link(temporary, destination).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.atomic_publish_failed",
            "The synchronized Arrow evidence could not be atomically published without replacing a file.",
            "Choose a new destination and retry.",
        )
    })?;
    fs::remove_file(temporary).map_err(|_| {
        let _ = fs::remove_file(destination);
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.atomic_publish_failed",
            "The temporary Arrow evidence could not be removed after no-replace publication.",
            "Choose a new destination and retry.",
        )
    })
}

#[tauri::command]
pub(crate) fn publish_internal_labs_multimod_raw_sidecar_v1(
    request: MultiModRawSidecarExportRequestV1,
) -> Result<MultiModRawSidecarExportReceiptV1, MultiModRawSidecarExportErrorV1> {
    if request.schema_version != RAW_SIDECAR_EXPORT_SCHEMA_VERSION_V1
        || request.surface != INTERNAL_LABS_SURFACE_V1
        || !request.experimental_labs_enabled
        || !lowercase_sha256(&request.expected_archive_sha256)
        || !lowercase_sha256(&request.expected_identity_sha256)
        || !lowercase_sha256(&request.expected_payload_sha256)
    {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.request_invalid",
            "The raw-sidecar request is not the exact Labs V1 contract.",
            "Reopen the completed result and select its listed evidence entry.",
        ));
    }
    let expected_project = Uuid::parse_str(&request.project_id).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.request_invalid",
            "The project identity is invalid.",
            "Reopen the completed result and retry.",
        )
    })?;
    if !exportable_entry(&request.result_id, &request.entry_name) {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.entry_not_exportable",
            "Only listed posterior, membership, assignment, replicate-ledger, and target-vector Arrow evidence may be exported.",
            "Choose an eligible evidence row from the completed result.",
        ));
    }
    let mut source = File::open(&request.archive_path).map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.archive_unavailable",
            "The source archive is unavailable.",
            "Reopen the project and retry from the completed result.",
        )
    })?;
    let archive_sha256 = hash_open_file(&mut source)?;
    if archive_sha256 != request.expected_archive_sha256 {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.archive_changed",
            "The project archive changed after this completed result was opened.",
            "Reopen the project and select the evidence entry again.",
        ));
    }
    let selected_source = source.try_clone().map_err(|_| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.archive_unavailable",
            "The pinned source archive could not be cloned for selected-entry readback.",
            "Reopen the project and retry from the completed result.",
        )
    })?;
    let loaded = load_project_archive_v6_from_file(source).map_err(|error| {
        MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.strict_reopen_failed",
            format!("Strict Archive V6 reopen rejected the raw evidence source: {error}"),
            "Do not export this payload; preserve the archive for diagnosis.",
        )
    })?;
    if loaded.document.project_id != expected_project {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.project_mismatch",
            "The reopened project identity differs from the completed result.",
            "Reopen the intended project and retry.",
        ));
    }
    let attachment = loaded
        .document
        .multimod_results
        .iter()
        .find(|result| result.result_id == request.result_id)
        .ok_or_else(|| {
            MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.result_absent",
                "The requested completed MultiMod result is absent after strict reopen.",
                "Reopen the project and select a current completed result.",
            )
        })?;
    let provenance = attachment.result.provenance();
    if provenance.qualification == MultimodQualificationStateV1::ReleaseQualifiedCandidate {
        let receipt = provenance
            .candidate_qualification_receipt
            .as_ref()
            .ok_or_else(|| {
                MultiModRawSidecarExportErrorV1::new(
                    "multimod.raw_export.candidate_receipt_missing",
                    "The candidate result has no build-embedded authority receipt.",
                    "Do not export this result from an unbound executable.",
                )
            })?;
        verify_multimod_candidate_receipt_against_embedded_v1(receipt).map_err(|error| {
            MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.candidate_authority_mismatch",
                error,
                "Open the result only in the exact candidate executable that produced it.",
            )
        })?;
    }
    let descriptor = attachment
        .sidecars
        .iter()
        .find(|descriptor| descriptor.entry_name == request.entry_name)
        .ok_or_else(|| {
            MultiModRawSidecarExportErrorV1::new(
                "multimod.raw_export.descriptor_absent",
                "The requested Arrow descriptor is absent after strict reopen.",
                "Choose an evidence entry listed by the current result.",
            )
        })?;
    if !exportable_evidence_role(&descriptor.evidence_role) {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.evidence_role_not_exportable",
            "The selected Arrow descriptor is not a posterior, membership, or replicate/target-ledger evidence contract.",
            "Choose an eligible evidence row from the completed result.",
        ));
    }
    if attachment.identity_sha256 != request.expected_identity_sha256
        || descriptor.identity_sha256 != request.expected_identity_sha256
        || descriptor.sha256 != request.expected_payload_sha256
    {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.identity_mismatch",
            "The reopened result, descriptor, or payload digest differs from the selected evidence receipt.",
            "Do not export this payload; reopen the result and retry.",
        ));
    }
    if !loaded.multimod_sidecars.contains(&request.entry_name) {
        return Err(MultiModRawSidecarExportErrorV1::new(
            "multimod.raw_export.payload_absent",
            "The requested Arrow payload is absent after strict reopen.",
            "Do not export this payload; preserve the archive for diagnosis.",
        ));
    }
    let selected_payload =
        read_multimod_sidecar_payload_from_file_v1(selected_source, &request.result_id, descriptor)
            .map_err(|error| {
                MultiModRawSidecarExportErrorV1::new(
                    "multimod.raw_export.payload_mismatch",
                    format!("The selected Arrow payload failed exact lazy readback: {error}"),
                    "Do not export this payload; preserve the archive for diagnosis.",
                )
            })?;
    let (destination, published_bytes) = publish_strictly_validated_payload_v1(
        &request.result_id,
        &selected_payload,
        &request.destination_path,
    )?;
    Ok(MultiModRawSidecarExportReceiptV1 {
        schema_version: RAW_SIDECAR_EXPORT_SCHEMA_VERSION_V1,
        archive_sha256,
        project_id: request.project_id,
        result_id: request.result_id,
        entry_name: request.entry_name,
        identity_sha256: request.expected_identity_sha256,
        path: destination.to_string_lossy().into_owned(),
        bytes: published_bytes,
        sha256: request.expected_payload_sha256,
        strict_reopen_validated: true,
        no_replace_publication: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{exportable_entry, publish_strictly_validated_payload_v1};
    use qpls_project::{
        encode_multimod_arrow_sidecar_v1, multimod_membership_with_row_tokens_batch_v1,
        multimod_target_ledger_batch_v1,
    };
    use std::fs;

    #[test]
    fn allows_only_bounded_scientific_evidence_leaf_names() {
        assert!(exportable_entry(
            "result-1",
            "results/result-1/evidence-0001-fimix-posteriors.arrow"
        ));
        assert!(exportable_entry(
            "result-1",
            "results/result-1/evidence-0002-conditional-inner-target-vectors.arrow"
        ));
        assert!(!exportable_entry(
            "result-1",
            "results/result-1/evidence-0000-metadata.arrow"
        ));
        assert!(!exportable_entry(
            "result-1",
            "results/result-1/../other/posteriors.arrow"
        ));
        assert!(!exportable_entry(
            "result-1",
            "results/other/evidence-0001-posteriors.arrow"
        ));
    }

    #[test]
    fn optional_posterior_and_replicate_exports_use_strict_no_replace_publication_and_readback() {
        let result_id = "result-sidecar-export-qualification";
        let identity = "a".repeat(64);
        let posterior = encode_multimod_arrow_sidecar_v1(
            result_id,
            "fimix-candidate-posteriors.arrow",
            &identity,
            "fimix-candidate:posteriors",
            &multimod_membership_with_row_tokens_batch_v1(
                vec![0, 0, 1, 1],
                vec![
                    "source-row:0".into(),
                    "source-row:0".into(),
                    "source-row:1".into(),
                    "source-row:1".into(),
                ],
                vec![0, 1, 0, 1],
                vec![0.8, 0.2, 0.1, 0.9],
            )
            .unwrap(),
        )
        .unwrap();
        let replicates = encode_multimod_arrow_sidecar_v1(
            result_id,
            "conditional-target-vectors.arrow",
            &identity,
            "conditional-inference:target-vectors",
            &multimod_target_ledger_batch_v1(
                vec![0, 1],
                vec!["target:path@z=-1".into(), "target:path@z=-1".into()],
                vec![0.24, 0.26],
                vec![true, true],
                vec![String::new(), String::new()],
            )
            .unwrap(),
        )
        .unwrap();
        let directory = tempfile::tempdir().unwrap();
        for (name, payload) in [
            ("posteriors.arrow", posterior),
            ("replicates.arrow", replicates),
        ] {
            let path = directory.path().join(name);
            let (published, byte_count) =
                publish_strictly_validated_payload_v1(result_id, &payload, &path.to_string_lossy())
                    .unwrap();
            assert_eq!(published, path);
            assert_eq!(byte_count, payload.bytes.len() as u64);
            assert_eq!(fs::read(&path).unwrap(), payload.bytes);

            let error =
                publish_strictly_validated_payload_v1(result_id, &payload, &path.to_string_lossy())
                    .unwrap_err();
            assert_eq!(error.code, "multimod.raw_export.destination_exists");
        }
    }

    #[test]
    fn optional_raw_export_rejects_tampered_payload_before_destination_creation() {
        let result_id = "result-sidecar-export-tamper";
        let identity = "b".repeat(64);
        let mut payload = encode_multimod_arrow_sidecar_v1(
            result_id,
            "conditional-target-vectors.arrow",
            &identity,
            "conditional-inference:target-vectors",
            &multimod_target_ledger_batch_v1(
                vec![0],
                vec!["target:path@z=0".into()],
                vec![0.25],
                vec![true],
                vec![String::new()],
            )
            .unwrap(),
        )
        .unwrap();
        payload.bytes[0] ^= 0xff;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("tampered.arrow");
        let error =
            publish_strictly_validated_payload_v1(result_id, &payload, &path.to_string_lossy())
                .unwrap_err();
        assert_eq!(error.code, "multimod.raw_export.payload_mismatch");
        assert!(!path.exists());
    }
}
