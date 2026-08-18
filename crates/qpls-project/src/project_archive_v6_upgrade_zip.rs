//! Internal-Labs, new-destination-only schema-6 ZIP upgrade-copy writer.
//!
//! This module does not participate in the live schema-5 save, autosave,
//! backup, recovery, or new-project paths. It writes only a validated upgrade
//! plan to a new sibling-published archive and preserves the source archive.

use super::{
    Project, ProjectArchiveDocumentV6, ProjectArchiveUpgradePlanV6, ProjectArchiveUpgradeReceiptV6,
    ProjectArchiveUpgradeRequestV6, ProjectArchiveV6Error, ProjectArchiveWriteReceiptV6,
    ProjectError, ProjectManifest, ProjectModelPayloadV6, ProjectModelRecordV6,
    archive_integrity::{
        DEFAULT_ARCHIVE_LIMITS, MANIFEST_ENTRY_NAME, MAX_MANIFEST_UNCOMPRESSED_BYTES,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES, PROJECT_ENTRY_NAME,
    },
    load_project, load_project_archive_v6, plan_project_upgrade_to_v6,
    serialize_project_document_v6,
};
use qpls_core::{
    LegacyBasicModelInterpretationV4, LegacyDisplayCovarianceV4, SemAnnotationV4,
    convert_legacy_basic_model_v4,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter, result::ZipError, write::SimpleFileOptions};

#[derive(Debug, thiserror::Error)]
pub enum ProjectArchiveUpgradeZipV6Error {
    #[error(transparent)]
    Contract(#[from] ProjectArchiveV6Error),
    #[error(transparent)]
    ProjectArchive(#[from] ProjectError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] ZipError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("schema-6 ZIP upgrade-copy was cancelled; no destination was retained")]
    Cancelled,
    #[error("source archive does not match the schema-6 upgrade plan: {0}")]
    SourcePlanMismatch(String),
    #[error("schema-6 ZIP exceeds archive limits: {0}")]
    ArchiveLimit(String),
    #[error("{phase} schema-6 ZIP failed exact validation: {reason}")]
    ArchiveValidation { phase: &'static str, reason: String },
    #[error("schema-6 ZIP cleanup failed for {path} after {original_error}: {cleanup_error}")]
    CleanupFailed {
        path: PathBuf,
        original_error: String,
        cleanup_error: String,
    },
    #[error(
        "schema-6 ZIP destination ownership changed at {path} after {original_error}; the replacement was preserved at {retained_path}: {detail}"
    )]
    DestinationOwnershipLost {
        path: PathBuf,
        retained_path: PathBuf,
        original_error: String,
        detail: String,
    },
    #[error(
        "schema-6 ZIP temporary-file ownership changed at {path} after {original_error}; the replacement was preserved at {retained_path}: {detail}"
    )]
    TemporaryOwnershipLost {
        path: PathBuf,
        retained_path: PathBuf,
        original_error: String,
        detail: String,
    },
}

/// Writes a schema-6 ZIP upgrade copy to a destination that must not exist.
///
/// The historical source is opened read-only, digest-bound to the plan, and
/// reloaded through the live schema-1-through-5 reader. Validated Arrow entry
/// bytes are streamed into the new archive without reserialization.
pub fn execute_project_upgrade_zip_copy_v6(
    source: &Path,
    destination: &Path,
    plan: &ProjectArchiveUpgradePlanV6,
) -> Result<ProjectArchiveUpgradeReceiptV6, ProjectArchiveUpgradeZipV6Error> {
    execute_project_upgrade_zip_copy_v6_with_control(source, destination, plan, || false, || true)
}

/// Cancellation-aware form used by Internal-Labs orchestration and failure
/// tests. Cancellation is checked while hashing/copying and on both sides of
/// publication. Writer-owned files are removed on failure; an inability to
/// remove one is returned explicitly as `CleanupFailed` with the retained path.
pub fn execute_project_upgrade_zip_copy_v6_with_cancel<Cancelled>(
    source: &Path,
    destination: &Path,
    plan: &ProjectArchiveUpgradePlanV6,
    cancelled: Cancelled,
) -> Result<ProjectArchiveUpgradeReceiptV6, ProjectArchiveUpgradeZipV6Error>
where
    Cancelled: FnMut() -> bool,
{
    execute_project_upgrade_zip_copy_v6_with_control(source, destination, plan, cancelled, || true)
}

/// Cancellation/commit-arbitrated form for the Internal-Labs native service.
/// `commit` is called exactly once after pre-commit validation while the
/// destination cleanup guard is still armed. Returning `false` cancels and
/// removes the writer-owned destination. Returning `true` is followed by a
/// final ownership, source-binding, and strict persisted-archive validation
/// before the guard is disarmed.
pub fn execute_project_upgrade_zip_copy_v6_with_control<Cancelled, Commit>(
    source: &Path,
    destination: &Path,
    plan: &ProjectArchiveUpgradePlanV6,
    mut cancelled: Cancelled,
    commit: Commit,
) -> Result<ProjectArchiveUpgradeReceiptV6, ProjectArchiveUpgradeZipV6Error>
where
    Cancelled: FnMut() -> bool,
    Commit: FnOnce() -> bool,
{
    plan.ensure_valid()?;
    let lineage = plan
        .document
        .upgrade_lineage()
        .ok_or(ProjectArchiveV6Error::UpgradeOriginRequired)?;
    ensure_lineage_path_binding("source_archive_path", source, &lineage.source_archive_path)?;
    ensure_lineage_path_binding(
        "destination_archive_path",
        destination,
        &lineage.destination_archive_path,
    )?;
    ensure_destination_absent(destination)?;
    ensure_not_cancelled(&mut cancelled)?;

    let expected_source_sha256 = &lineage.source_archive_sha256;
    let observed_source_sha256 = sha256_file_with_cancel(source, &mut cancelled)?;
    if observed_source_sha256 != *expected_source_sha256 {
        return Err(ProjectArchiveV6Error::SourceDigestMismatch {
            expected: expected_source_sha256.clone(),
            observed: observed_source_sha256,
        }
        .into());
    }

    let source_project = load_project(source)?;
    validate_source_against_plan(&source_project, &plan.document)?;
    if sha256_file_with_cancel(source, &mut cancelled)? != *expected_source_sha256 {
        return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
    }
    ensure_destination_absent(destination)?;
    ensure_not_cancelled(&mut cancelled)?;

    let project_bytes = serialize_project_document_v6(&plan.document)?;
    ensure_entry_size(
        PROJECT_ENTRY_NAME,
        project_bytes.len() as u64,
        MAX_PROJECT_DOCUMENT_UNCOMPRESSED_BYTES,
    )?;
    let document_sha256 = sha256_bytes(&project_bytes);
    let entry_count = plan.document.datasets.len().saturating_add(2);
    if entry_count > DEFAULT_ARCHIVE_LIMITS.max_entries {
        return Err(ProjectArchiveUpgradeZipV6Error::ArchiveLimit(format!(
            "{entry_count} entries exceed the {}-entry limit",
            DEFAULT_ARCHIVE_LIMITS.max_entries
        )));
    }
    ensure_file_identity_supported()?;

    let temporary = temporary_upgrade_zip_path(destination)?;
    let temporary_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let temporary_identity = match file_identity_from_file(&temporary_file) {
        Ok(identity) => identity,
        Err(error) => {
            return Err(ProjectArchiveUpgradeZipV6Error::CleanupFailed {
                path: temporary,
                original_error: "temporary-file identity acquisition failed".to_owned(),
                cleanup_error: format!(
                    "ownership could not be established, so the created file was retained rather than risking deletion of a replacement: {error}"
                ),
            });
        }
    };
    let mut temporary_guard =
        OwnedFileGuard::new(temporary.clone(), temporary_identity, "upgrade-v6");
    let mut destination_guard = None;
    let initial_token = match temporary_file.try_clone() {
        Ok(token) => token,
        Err(error) => {
            return Err(cleanup_upgrade_failure(
                error.into(),
                &mut temporary_guard,
                None,
                destination,
            ));
        }
    };
    let mut ownership_tokens = vec![initial_token];
    let operation = (|| {
        let mut source_archive = ZipArchive::new(File::open(source)?)?;
        let mut output = ZipWriter::new(temporary_file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut checksums = BTreeMap::new();
        let mut total_uncompressed = 0_u64;

        output.start_file(PROJECT_ENTRY_NAME, options)?;
        output.write_all(&project_bytes)?;
        checksums.insert(PROJECT_ENTRY_NAME.to_owned(), document_sha256.clone());
        add_total_uncompressed(&mut total_uncompressed, project_bytes.len() as u64)?;
        ensure_not_cancelled(&mut cancelled)?;

        for descriptor in &plan.document.datasets {
            let entry_name = format!("data/{}.arrow", descriptor.id);
            if entry_name.len() > DEFAULT_ARCHIVE_LIMITS.max_entry_name_bytes {
                return Err(ProjectArchiveUpgradeZipV6Error::ArchiveLimit(format!(
                    "entry name {entry_name} exceeds the {}-byte limit",
                    DEFAULT_ARCHIVE_LIMITS.max_entry_name_bytes
                )));
            }
            let mut source_entry = source_archive.by_name(&entry_name)?;
            let declared_size = source_entry.size();
            ensure_entry_size(
                &entry_name,
                declared_size,
                DEFAULT_ARCHIVE_LIMITS.max_entry_uncompressed_bytes,
            )?;
            output.start_file(&entry_name, options)?;
            let (checksum, copied) = stream_copy_with_sha256(
                &mut source_entry,
                &mut output,
                declared_size,
                &mut cancelled,
            )?;
            checksums.insert(entry_name, checksum);
            add_total_uncompressed(&mut total_uncompressed, copied)?;
        }
        drop(source_archive);

        let manifest = ProjectManifest {
            schema_version: super::PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
            project_id: plan.document.project_id,
            name: plan.document.name.clone(),
            created_at: plan.document.created_at,
            modified_at: plan.document.modified_at,
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
        let finished_file = output.finish()?;
        finished_file.sync_all()?;
        if file_identity_from_file(&finished_file)? != temporary_identity {
            return Err(ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
                phase: "temporary",
                reason: "ZIP writer handle no longer identifies the created temporary file"
                    .to_owned(),
            });
        }
        ownership_tokens.push(finished_file);
        ensure_not_cancelled(&mut cancelled)?;

        validate_written_archive(
            "temporary",
            &temporary,
            &manifest,
            &plan.document,
            &project_bytes,
        )?;
        ensure_not_cancelled(&mut cancelled)?;
        if sha256_file_with_cancel(source, &mut cancelled)? != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
        }
        ensure_not_cancelled(&mut cancelled)?;

        let published_token = owned_upgrade_file(&temporary_guard)?;
        publish_new_no_clobber(&temporary, destination)?;
        destination_guard = Some(OwnedFileGuard::new(
            destination.to_path_buf(),
            temporary_identity,
            "upgrade-v6",
        ));
        ownership_tokens.push(published_token);
        let destination_token = owned_upgrade_file(
            destination_guard
                .as_ref()
                .expect("published destination must have an ownership guard"),
        )?;
        ownership_tokens.push(destination_token);
        sync_parent_directory(destination)?;
        ensure_not_cancelled(&mut cancelled)?;

        if sha256_file_with_cancel(source, &mut cancelled)? != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
        }
        validate_written_archive(
            "persisted",
            destination,
            &manifest,
            &plan.document,
            &project_bytes,
        )?;
        if sha256_file_with_cancel(source, &mut cancelled)? != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
        }
        ensure_not_cancelled(&mut cancelled)?;
        remove_temporary_before_commit(&mut temporary_guard)?;
        sync_parent_directory(destination)?;
        if sha256_file_with_cancel(source, &mut cancelled)? != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
        }
        ensure_not_cancelled(&mut cancelled)?;
        let commit_token = owned_upgrade_file(
            destination_guard
                .as_ref()
                .expect("published destination must have an ownership guard"),
        )?;
        ownership_tokens.push(commit_token);
        if !commit() {
            return Err(ProjectArchiveUpgradeZipV6Error::Cancelled);
        }

        let post_commit_token = owned_upgrade_file(
            destination_guard
                .as_ref()
                .expect("published destination must have an ownership guard"),
        )?;
        ownership_tokens.push(post_commit_token);
        let mut post_commit_cancelled = || false;
        if sha256_file_with_cancel(source, &mut post_commit_cancelled)? != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
        }
        validate_written_archive(
            "post-commit",
            destination,
            &manifest,
            &plan.document,
            &project_bytes,
        )?;
        if sha256_file_with_cancel(source, &mut post_commit_cancelled)? != *expected_source_sha256 {
            return Err(ProjectArchiveV6Error::SourceChangedDuringUpgrade.into());
        }
        let final_destination_token = owned_upgrade_file(
            destination_guard
                .as_ref()
                .expect("published destination must have an ownership guard"),
        )?;
        let destination_metadata = final_destination_token.metadata()?;
        ownership_tokens.push(final_destination_token);

        let receipt = ProjectArchiveUpgradeReceiptV6 {
            write: ProjectArchiveWriteReceiptV6 {
                schema_version: super::PROJECT_ARCHIVE_SCHEMA_V6_VERSION,
                project_id: plan.document.project_id,
                destination_archive_path: destination.to_string_lossy().into_owned(),
                document_sha256,
                byte_length: destination_metadata.len(),
                post_write_validated: true,
            },
            source_archive_path: lineage.source_archive_path.clone(),
            source_archive_sha256: expected_source_sha256.clone(),
            source_verified_unchanged: true,
            historical_results_immutable: lineage.historical_results_immutable,
        };
        Ok(receipt)
    })();

    match operation {
        Ok(receipt) => {
            destination_guard
                .as_mut()
                .expect("successful publication must arm the destination guard")
                .disarm();
            Ok(receipt)
        }
        Err(error) => Err(cleanup_upgrade_failure(
            error,
            &mut temporary_guard,
            destination_guard.as_mut(),
            destination,
        )),
    }
}

fn cleanup_upgrade_failure(
    error: ProjectArchiveUpgradeZipV6Error,
    temporary_guard: &mut OwnedFileGuard,
    destination_guard: Option<&mut OwnedFileGuard>,
    destination: &Path,
) -> ProjectArchiveUpgradeZipV6Error {
    let original_error = error.to_string();
    let mut failures = Vec::new();
    let mut ownership_lost = None;
    if let Some(destination_guard) = destination_guard {
        match destination_guard.remove_now() {
            Ok(()) => {
                if let Err(cleanup_error) = sync_parent_directory(destination) {
                    failures.push((destination.to_path_buf(), cleanup_error.to_string()));
                }
            }
            Err(OwnedFileCleanupError::Io { path, error }) => {
                failures.push((path, error.to_string()));
            }
            Err(OwnedFileCleanupError::OwnershipLost {
                path,
                retained_path,
                detail,
            }) => {
                ownership_lost = Some((path, retained_path, detail));
            }
        }
    }
    let mut temporary_ownership_lost = None;
    match temporary_guard.remove_now() {
        Ok(()) => {}
        Err(OwnedFileCleanupError::Io { path, error }) => {
            failures.push((path, error.to_string()));
        }
        Err(OwnedFileCleanupError::OwnershipLost {
            path,
            retained_path,
            detail,
        }) => {
            temporary_ownership_lost = Some((path, retained_path, detail));
        }
    }
    if let Some((path, retained_path, mut detail)) = ownership_lost {
        if let Some((temporary_path, temporary_retained_path, temporary_detail)) =
            temporary_ownership_lost.take()
        {
            detail.push_str(&format!(
                "; temporary ownership also changed at {} and was preserved at {}: {temporary_detail}",
                temporary_path.display(),
                temporary_retained_path.display()
            ));
        }
        if !failures.is_empty() {
            detail.push_str("; additional cleanup failures: ");
            detail.push_str(
                &failures
                    .iter()
                    .map(|(path, error)| format!("{}: {error}", path.display()))
                    .collect::<Vec<_>>()
                    .join("; "),
            );
        }
        ProjectArchiveUpgradeZipV6Error::DestinationOwnershipLost {
            path,
            retained_path,
            original_error,
            detail,
        }
    } else if let Some((path, retained_path, mut detail)) = temporary_ownership_lost.take() {
        if !failures.is_empty() {
            detail.push_str("; additional cleanup failures: ");
            detail.push_str(
                &failures
                    .iter()
                    .map(|(path, error)| format!("{}: {error}", path.display()))
                    .collect::<Vec<_>>()
                    .join("; "),
            );
        }
        ProjectArchiveUpgradeZipV6Error::TemporaryOwnershipLost {
            path,
            retained_path,
            original_error,
            detail,
        }
    } else if failures.is_empty() {
        error
    } else {
        let path = failures[0].0.clone();
        let cleanup_error = failures
            .into_iter()
            .map(|(path, error)| format!("{}: {error}", path.display()))
            .collect::<Vec<_>>()
            .join("; ");
        ProjectArchiveUpgradeZipV6Error::CleanupFailed {
            path,
            original_error,
            cleanup_error,
        }
    }
}

fn owned_upgrade_file(guard: &OwnedFileGuard) -> Result<File, ProjectArchiveUpgradeZipV6Error> {
    guard
        .owned_file()
        .map_err(|error| ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
            phase: "ownership",
            reason: error.to_string(),
        })
}

fn remove_temporary_before_commit(
    temporary_guard: &mut OwnedFileGuard,
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    match temporary_guard.remove_now() {
        Ok(()) => Ok(()),
        Err(OwnedFileCleanupError::Io { path, error }) => {
            Err(ProjectArchiveUpgradeZipV6Error::CleanupFailed {
                path,
                original_error: "pre-commit temporary-file cleanup".to_owned(),
                cleanup_error: error.to_string(),
            })
        }
        Err(OwnedFileCleanupError::OwnershipLost {
            path,
            retained_path,
            detail,
        }) => Err(ProjectArchiveUpgradeZipV6Error::TemporaryOwnershipLost {
            path,
            retained_path,
            original_error: "pre-commit temporary-file cleanup".to_owned(),
            detail,
        }),
    }
}

fn validate_source_against_plan(
    source: &Project,
    document: &ProjectArchiveDocumentV6,
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    let lineage = document
        .upgrade_lineage()
        .ok_or(ProjectArchiveV6Error::UpgradeOriginRequired)?;
    if source.read_only || !(1..=5).contains(&source.source_archive_version) {
        return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
            "source is not a writable historical schema-1-through-5 archive".to_owned(),
        ));
    }
    if source.source_archive_version != lineage.source_archive_schema_version
        || source.manifest.project_id != lineage.source_project_id
        || source.manifest.project_id != document.project_id
        || source.manifest.name != document.name
        || source.manifest.created_at != document.created_at
    {
        return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
            "project identity, source schema, or creation timestamp differs".to_owned(),
        ));
    }
    let display_covariances = document
        .models
        .iter()
        .map(|record| {
            Ok((
                record.model_id.clone(),
                display_covariances_from_planned_model(record)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, ProjectArchiveUpgradeZipV6Error>>()?;
    let expected = plan_project_upgrade_to_v6(
        source,
        &ProjectArchiveUpgradeRequestV6 {
            source_archive_sha256: lineage.source_archive_sha256.clone(),
            source_archive_path: lineage.source_archive_path.clone(),
            destination_archive_path: lineage.destination_archive_path.clone(),
            upgraded_at: lineage.upgraded_at,
            legacy_display_covariances: display_covariances,
        },
    )
    .map_err(|error| {
        ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(format!(
            "authoritative upgrade planning failed: {error}"
        ))
    })?;

    // Bind every source-derived lane, not just dataset identity. Replacing the
    // expected models with the submitted models isolates the comparison to
    // project identity/timestamps, datasets, layouts, immutable history,
    // current recipes, canonical attachments, and exact upgrade origin.
    let mut expected_non_model_lanes = expected.document.clone();
    expected_non_model_lanes.models = document.models.clone();
    if serde_json::to_value(&expected_non_model_lanes)? != serde_json::to_value(document)? {
        return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
            "a source-derived project lane differs from the authoritative upgrade plan".to_owned(),
        ));
    }
    validate_planned_models(&expected.document.models, &document.models)?;
    Ok(())
}

fn display_covariances_from_planned_model(
    record: &ProjectModelRecordV6,
) -> Result<Vec<LegacyDisplayCovarianceV4>, ProjectArchiveUpgradeZipV6Error> {
    match &record.payload {
        ProjectModelPayloadV6::LegacyEstimandUnspecified {
            display_covariances,
            ..
        } => Ok(display_covariances.clone()),
        ProjectModelPayloadV6::SemModelV4 { model, .. }
        | ProjectModelPayloadV6::SemModelV4Draft { model, .. } => model
            .annotations
            .iter()
            .map(|annotation| match annotation {
                SemAnnotationV4::DisplayOnlyCovariance {
                    id,
                    left,
                    right,
                    label,
                } => Ok(LegacyDisplayCovarianceV4 {
                    id: id.clone(),
                    left_construct: legacy_construct_id(left, &record.model_id)?,
                    right_construct: legacy_construct_id(right, &record.model_id)?,
                    label: label.clone(),
                }),
                SemAnnotationV4::Caption { .. } | SemAnnotationV4::Note { .. } => {
                    Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(format!(
                        "model {} contains an annotation that legacy upgrade conversion cannot produce",
                        record.model_id
                    )))
                }
            })
            .collect(),
    }
}

fn legacy_construct_id(
    sem_endpoint: &str,
    model_id: &str,
) -> Result<String, ProjectArchiveUpgradeZipV6Error> {
    sem_endpoint
        .strip_prefix("construct:")
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(format!(
                "model {model_id} has a display covariance endpoint that legacy conversion cannot produce"
            ))
        })
}

fn validate_planned_models(
    expected: &[ProjectModelRecordV6],
    actual: &[ProjectModelRecordV6],
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    if expected.len() != actual.len() {
        return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
            "model count differs from the authoritative upgrade plan".to_owned(),
        ));
    }
    for (expected, actual) in expected.iter().zip(actual) {
        if expected.model_id != actual.model_id {
            return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
                "model order or identity differs from the authoritative upgrade plan".to_owned(),
            ));
        }
        if expected == actual || exact_draft_wrapper(expected, actual) {
            continue;
        }
        let ProjectModelPayloadV6::LegacyEstimandUnspecified {
            legacy_model,
            display_covariances,
            ..
        } = &expected.payload
        else {
            return Err(model_plan_mismatch(&actual.model_id));
        };
        let actual_model = match &actual.payload {
            ProjectModelPayloadV6::SemModelV4 { model, .. }
            | ProjectModelPayloadV6::SemModelV4Draft { model, .. } => model,
            ProjectModelPayloadV6::LegacyEstimandUnspecified { .. } => {
                return Err(model_plan_mismatch(&actual.model_id));
            }
        };
        let permitted = [
            LegacyBasicModelInterpretationV4::PlsComposite,
            LegacyBasicModelInterpretationV4::CbsemCommonFactor,
        ]
        .into_iter()
        .filter_map(|interpretation| {
            convert_legacy_basic_model_v4(legacy_model, interpretation, display_covariances).ok()
        })
        .any(|converted| converted == *actual_model);
        if !permitted {
            return Err(model_plan_mismatch(&actual.model_id));
        }
    }
    Ok(())
}

fn exact_draft_wrapper(expected: &ProjectModelRecordV6, actual: &ProjectModelRecordV6) -> bool {
    matches!(
        (&expected.payload, &actual.payload),
        (
            ProjectModelPayloadV6::SemModelV4 {
                model: expected_model,
                ..
            },
            ProjectModelPayloadV6::SemModelV4Draft {
                model: actual_model,
                ..
            }
        ) if expected_model == actual_model
    )
}

fn model_plan_mismatch(model_id: &str) -> ProjectArchiveUpgradeZipV6Error {
    ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(format!(
        "model {model_id} is not an exact legacy snapshot, permitted conversion, or exact draft wrapper"
    ))
}

fn stream_copy_with_sha256<Reader, Writer, Cancelled>(
    reader: &mut Reader,
    writer: &mut Writer,
    declared_size: u64,
    cancelled: &mut Cancelled,
) -> Result<(String, u64), ProjectArchiveUpgradeZipV6Error>
where
    Reader: Read,
    Writer: Write,
    Cancelled: FnMut() -> bool,
{
    let mut digest = Sha256::new();
    let mut copied = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        ensure_not_cancelled(cancelled)?;
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        copied = copied.checked_add(read as u64).ok_or_else(|| {
            ProjectArchiveUpgradeZipV6Error::ArchiveLimit(
                "copied Arrow entry size overflowed".to_owned(),
            )
        })?;
        if copied > declared_size {
            return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
                "Arrow entry expanded beyond its declared size".to_owned(),
            ));
        }
        writer.write_all(&buffer[..read])?;
        digest.update(&buffer[..read]);
    }
    if copied != declared_size {
        return Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(
            "Arrow entry length differs from its central-directory declaration".to_owned(),
        ));
    }
    Ok((format!("{:x}", digest.finalize()), copied))
}

fn validate_written_archive(
    phase: &'static str,
    path: &Path,
    expected_manifest: &ProjectManifest,
    expected_document: &ProjectArchiveDocumentV6,
    expected_project_bytes: &[u8],
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    let loaded = load_project_archive_v6(path).map_err(|error| {
        ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
            phase,
            reason: error.to_string(),
        }
    })?;
    let manifest = loaded.manifest;
    if manifest.schema_version != expected_manifest.schema_version
        || manifest.project_id != expected_manifest.project_id
        || manifest.name != expected_manifest.name
        || manifest.created_at != expected_manifest.created_at
        || manifest.modified_at != expected_manifest.modified_at
        || manifest.engine_version != expected_manifest.engine_version
        || manifest.checksum_algorithm != expected_manifest.checksum_algorithm
        || manifest.checksums != expected_manifest.checksums
    {
        return Err(ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
            phase,
            reason: "manifest differs from the write plan".to_owned(),
        });
    }
    let reopened_bytes = serialize_project_document_v6(&loaded.document).map_err(|error| {
        ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
            phase,
            reason: error.to_string(),
        }
    })?;
    if reopened_bytes != expected_project_bytes
        || loaded.document.project_id != expected_document.project_id
    {
        return Err(ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
            phase,
            reason: "project document differs from the write plan".to_owned(),
        });
    }
    Ok(())
}

fn ensure_lineage_path_binding(
    field: &'static str,
    observed: &Path,
    expected: &str,
) -> Result<(), ProjectArchiveV6Error> {
    let observed = observed
        .to_str()
        .ok_or(ProjectArchiveV6Error::NonUnicodeUpgradePath { field })?;
    if observed != expected {
        return Err(ProjectArchiveV6Error::UpgradePathBinding {
            field,
            expected: expected.to_owned(),
            observed: observed.to_owned(),
        });
    }
    Ok(())
}

fn ensure_destination_absent(destination: &Path) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    match fs::symlink_metadata(destination) {
        Ok(_) => Err(ProjectArchiveV6Error::DestinationExists(destination.to_path_buf()).into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn ensure_not_cancelled<Cancelled>(
    cancelled: &mut Cancelled,
) -> Result<(), ProjectArchiveUpgradeZipV6Error>
where
    Cancelled: FnMut() -> bool,
{
    if cancelled() {
        Err(ProjectArchiveUpgradeZipV6Error::Cancelled)
    } else {
        Ok(())
    }
}

fn ensure_entry_size(
    name: &str,
    size: u64,
    limit: u64,
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    if size > limit {
        Err(ProjectArchiveUpgradeZipV6Error::ArchiveLimit(format!(
            "entry {name} declares {size} bytes; limit is {limit}"
        )))
    } else {
        Ok(())
    }
}

fn add_total_uncompressed(
    total: &mut u64,
    amount: u64,
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    *total = total.checked_add(amount).ok_or_else(|| {
        ProjectArchiveUpgradeZipV6Error::ArchiveLimit(
            "total uncompressed size overflowed".to_owned(),
        )
    })?;
    if *total > DEFAULT_ARCHIVE_LIMITS.max_total_uncompressed_bytes {
        return Err(ProjectArchiveUpgradeZipV6Error::ArchiveLimit(format!(
            "total uncompressed size exceeds {} bytes",
            DEFAULT_ARCHIVE_LIMITS.max_total_uncompressed_bytes
        )));
    }
    Ok(())
}

fn sha256_file_with_cancel<Cancelled>(
    path: &Path,
    cancelled: &mut Cancelled,
) -> Result<String, ProjectArchiveUpgradeZipV6Error>
where
    Cancelled: FnMut() -> bool,
{
    let mut file = File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        ensure_not_cancelled(cancelled)?;
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn temporary_upgrade_zip_path(
    destination: &Path,
) -> Result<PathBuf, ProjectArchiveUpgradeZipV6Error> {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ProjectArchiveV6Error::DestinationFileName)?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    Ok(parent.join(format!(
        ".{file_name}.upgrade-v6-zip-{}.tmp",
        Uuid::new_v4()
    )))
}

fn publish_new_no_clobber(
    temporary: &Path,
    destination: &Path,
) -> Result<(), ProjectArchiveUpgradeZipV6Error> {
    fs::hard_link(temporary, destination).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists
            || fs::symlink_metadata(destination).is_ok()
        {
            ProjectArchiveUpgradeZipV6Error::Contract(ProjectArchiveV6Error::DestinationExists(
                destination.to_path_buf(),
            ))
        } else {
            ProjectArchiveUpgradeZipV6Error::Io(error)
        }
    })
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    volume_serial_number: u32,
    file_index: u64,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(not(any(windows, unix)))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileIdentity;

#[derive(Debug)]
enum OwnedFileAccessError {
    NotArmed,
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    OwnershipLost {
        path: PathBuf,
    },
}

impl std::fmt::Display for OwnedFileAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotArmed => formatter.write_str("writer-owned path guard is not armed"),
            Self::Io { path, error } => {
                write!(formatter, "could not inspect {}: {error}", path.display())
            }
            Self::OwnershipLost { path } => write!(
                formatter,
                "{} no longer identifies the writer-owned file",
                path.display()
            ),
        }
    }
}

#[derive(Debug)]
enum OwnedFileCleanupError {
    Io {
        path: PathBuf,
        error: std::io::Error,
    },
    OwnershipLost {
        path: PathBuf,
        retained_path: PathBuf,
        detail: String,
    },
}

struct OwnedFileGuard {
    path: Option<PathBuf>,
    identity: FileIdentity,
    cleanup_label: &'static str,
}

impl OwnedFileGuard {
    fn new(path: PathBuf, identity: FileIdentity, cleanup_label: &'static str) -> Self {
        Self {
            path: Some(path),
            identity,
            cleanup_label,
        }
    }

    fn disarm(&mut self) {
        self.path = None;
    }

    fn owned_file(&self) -> Result<File, OwnedFileAccessError> {
        let path = self.path.as_ref().ok_or(OwnedFileAccessError::NotArmed)?;
        let file = File::open(path).map_err(|error| OwnedFileAccessError::Io {
            path: path.clone(),
            error,
        })?;
        let observed =
            file_identity_from_file(&file).map_err(|error| OwnedFileAccessError::Io {
                path: path.clone(),
                error,
            })?;
        if observed != self.identity {
            return Err(OwnedFileAccessError::OwnershipLost { path: path.clone() });
        }
        Ok(file)
    }

    fn remove_now(&mut self) -> Result<(), OwnedFileCleanupError> {
        let Some(path) = self.path.take() else {
            return Ok(());
        };
        let quarantine_directory = match create_cleanup_quarantine(&path, self.cleanup_label) {
            Ok(directory) => directory,
            Err(error) => {
                self.path = Some(path.clone());
                return Err(OwnedFileCleanupError::Io { path, error });
            }
        };
        let quarantine_path = quarantine_directory.join("published.qpls");
        match fs::rename(&path, &quarantine_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return match fs::remove_dir(&quarantine_directory) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(error) => Err(OwnedFileCleanupError::Io {
                        path: quarantine_directory,
                        error,
                    }),
                };
            }
            Err(error) => {
                self.path = Some(path.clone());
                return match fs::remove_dir(&quarantine_directory) {
                    Ok(()) => Err(OwnedFileCleanupError::Io { path, error }),
                    Err(quarantine_error) => Err(OwnedFileCleanupError::Io {
                        path: quarantine_directory,
                        error: std::io::Error::new(
                            error.kind(),
                            format!(
                                "guarded-path rename failed: {error}; cleanup quarantine removal also failed: {quarantine_error}"
                            ),
                        ),
                    }),
                };
            }
        }

        let observed_identity =
            file_identity(&quarantine_path).map_err(|error| OwnedFileCleanupError::Io {
                path: quarantine_path.clone(),
                error,
            })?;
        if observed_identity == self.identity {
            fs::remove_file(&quarantine_path).map_err(|error| OwnedFileCleanupError::Io {
                path: quarantine_path.clone(),
                error,
            })?;
            fs::remove_dir(&quarantine_directory).map_err(|error| OwnedFileCleanupError::Io {
                path: quarantine_directory,
                error,
            })?;
            return Ok(());
        }

        let mut retained_path = quarantine_path.clone();
        let mut detail =
            "the guarded pathname referred to a different file at cleanup time".to_owned();
        match fs::hard_link(&quarantine_path, &path) {
            Ok(()) => {
                retained_path = path.clone();
                if let Err(error) = fs::remove_file(&quarantine_path) {
                    retained_path = quarantine_path.clone();
                    detail.push_str(&format!(
                        "; replacement restored but its quarantine link could not be removed: {error}"
                    ));
                } else if let Err(error) = fs::remove_dir(&quarantine_directory) {
                    detail.push_str(&format!(
                        "; empty cleanup quarantine could not be removed: {error}"
                    ));
                }
            }
            Err(error) => detail.push_str(&format!(
                "; replacement remains quarantined because no-clobber restoration failed: {error}"
            )),
        }
        Err(OwnedFileCleanupError::OwnershipLost {
            path,
            retained_path,
            detail,
        })
    }
}

impl Drop for OwnedFileGuard {
    fn drop(&mut self) {
        let _ = self.remove_now();
    }
}

#[cfg(any(windows, unix))]
fn ensure_file_identity_supported() -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(not(any(windows, unix)))]
fn ensure_file_identity_supported() -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "schema-6 file-ownership checks are unsupported on this platform",
    ))
}

#[cfg(windows)]
fn file_identity(path: &Path) -> Result<FileIdentity, std::io::Error> {
    file_identity_from_file(&File::open(path)?)
}

#[cfg(windows)]
fn file_identity_from_file(file: &File) -> Result<FileIdentity, std::io::Error> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: `file` owns a valid handle and the output pointer is valid.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut information) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(FileIdentity {
        volume_serial_number: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
    })
}

#[cfg(unix)]
fn file_identity(path: &Path) -> Result<FileIdentity, std::io::Error> {
    file_identity_from_file(&File::open(path)?)
}

#[cfg(unix)]
fn file_identity_from_file(file: &File) -> Result<FileIdentity, std::io::Error> {
    use std::os::unix::fs::MetadataExt;

    let metadata = file.metadata()?;
    Ok(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(any(windows, unix)))]
fn file_identity(_path: &Path) -> Result<FileIdentity, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "schema-6 file-ownership checks are unsupported on this platform",
    ))
}

#[cfg(not(any(windows, unix)))]
fn file_identity_from_file(_file: &File) -> Result<FileIdentity, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "schema-6 file-ownership checks are unsupported on this platform",
    ))
}

fn sync_parent_directory(path: &Path) -> Result<(), std::io::Error> {
    #[cfg(not(windows))]
    {
        File::open(parent_directory(path))?.sync_all()?;
    }
    #[cfg(windows)]
    {
        let _ = path;
    }
    Ok(())
}

fn create_cleanup_quarantine(
    destination: &Path,
    cleanup_label: &str,
) -> Result<PathBuf, std::io::Error> {
    let parent = parent_directory(destination);
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project.qpls");
    for _ in 0..8 {
        let candidate = parent.join(format!(
            ".{file_name}.{cleanup_label}-cleanup-{}",
            Uuid::new_v4()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not allocate a unique schema-6 cleanup quarantine",
    ))
}

fn parent_directory(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ProjectArchiveUpgradeRequestV6, ProjectModelPayloadV6, load_project_archive_v6,
        plan_project_upgrade_to_v6, save_project,
    };
    use chrono::{TimeZone, Utc};
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisPayload, AnalysisRecipe,
        AnalysisResult, AnalysisSettings, Construct, MeasurementMode, MethodConfig, ModelSpec,
        RESULT_SCHEMA_VERSION, RunProvenance, RunStatus, StructuralPath,
    };
    use qpls_data::{DataFingerprint, ImportOptions, import_delimited_bytes, write_arrow};
    use serde_json::json;
    use std::{cell::Cell, collections::BTreeSet, io::Read};
    use tempfile::TempDir;

    fn legacy_model() -> ModelSpec {
        ModelSpec {
            id: Uuid::from_u128(10),
            name: "Legacy model".into(),
            constructs: vec![
                Construct {
                    id: "x".into(),
                    name: "X".into(),
                    short_name: "X".into(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec!["x1".into(), "x2".into()],
                },
                Construct {
                    id: "y".into(),
                    name: "Y".into(),
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
        }
    }

    fn historical_recipe() -> AnalysisRecipe {
        AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(20),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: "historical-dataset".into(),
            model: legacy_model(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        }
    }

    fn historical_result(recipe: &AnalysisRecipe) -> AnalysisResult {
        AnalysisResult {
            schema_version: RESULT_SCHEMA_VERSION,
            id: Uuid::from_u128(30),
            status: RunStatus::Completed,
            provenance: RunProvenance {
                recipe_id: recipe.id,
                dataset_fingerprint: recipe.dataset_fingerprint.clone(),
                method: recipe.settings.method,
                method_version: "historical".into(),
                engine_version: "historical".into(),
                seed: 7,
                settings: recipe.settings.clone(),
                started_at: recipe.created_at,
                completed_at: Utc.timestamp_opt(1_700_000_001, 0).unwrap(),
            },
            diagnostics: Vec::new(),
            payload: AnalysisPayload::Legacy {
                value: json!({"coefficient": 0.9954396945354063}),
            },
        }
    }

    fn upgrade_fixture() -> (
        TempDir,
        PathBuf,
        PathBuf,
        ProjectArchiveUpgradePlanV6,
        Vec<u8>,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.qpls");
        let destination = directory.path().join("upgraded-v6.qpls");
        let v2_dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n1,2,3,4\n5,6,7,8\n",
            "v2.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let mut legacy_dataset = import_delimited_bytes(
            b"x1,x2,y1,y2\n9,10,11,12\n13,14,15,16\n",
            "legacy.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        legacy_dataset.fingerprint =
            DataFingerprint(sha256_bytes(&write_arrow(&legacy_dataset.batch).unwrap()));

        let recipe = historical_recipe();
        let mut project = Project::new("Schema-6 ZIP upgrade fixture");
        project.datasets = vec![v2_dataset, legacy_dataset];
        project.models.push(legacy_model());
        project.recipes.push(recipe.clone());
        project.results.push(historical_result(&recipe));
        project.layouts.insert(
            "workspace".into(),
            json!({"zoom": 1.25, "selection": ["x"], "draft": true}),
        );
        save_project(&source, &project).unwrap();
        let source_bytes = fs::read(&source).unwrap();
        let source_project = load_project(&source).unwrap();
        let mut plan = plan_project_upgrade_to_v6(
            &source_project,
            &ProjectArchiveUpgradeRequestV6 {
                source_archive_sha256: sha256_bytes(&source_bytes),
                source_archive_path: source.to_str().unwrap().into(),
                destination_archive_path: destination.to_str().unwrap().into(),
                upgraded_at: Utc.timestamp_opt(1_800_000_000, 0).unwrap(),
                legacy_display_covariances: BTreeMap::new(),
            },
        )
        .unwrap();
        let ready_model = match &plan.document.models[0].payload {
            ProjectModelPayloadV6::SemModelV4 { model, .. } => model.clone(),
            payload => panic!("fixture expected a ready model, found {payload:?}"),
        };
        plan.document.models[0].payload = ProjectModelPayloadV6::SemModelV4Draft {
            model_document_sha256: ready_model.model_document_sha256().unwrap(),
            model: ready_model,
        };
        plan.ensure_valid().unwrap();
        (directory, source, destination, plan, source_bytes)
    }

    fn zip_entry(path: &Path, name: &str) -> Vec<u8> {
        let mut archive = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut entry = archive.by_name(name).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        bytes
    }

    fn temporary_upgrade_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".upgrade-v6-zip-") && name.ends_with(".tmp"))
            })
            .collect()
    }

    #[test]
    fn writes_strict_upgrade_zip_and_preserves_source_arrow_bytes_and_document() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let expected_document = serialize_project_document_v6(&plan.document).unwrap();
        let expected_arrow = plan
            .document
            .datasets
            .iter()
            .map(|descriptor| {
                let name = format!("data/{}.arrow", descriptor.id);
                (name.clone(), zip_entry(&source, &name))
            })
            .collect::<BTreeMap<_, _>>();

        let receipt = execute_project_upgrade_zip_copy_v6(&source, &destination, &plan).unwrap();
        let loaded = load_project_archive_v6(&destination).unwrap();

        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert_eq!(receipt.source_archive_sha256, sha256_bytes(&source_bytes));
        assert!(receipt.source_verified_unchanged);
        assert!(receipt.historical_results_immutable);
        assert!(receipt.write.post_write_validated);
        assert_eq!(
            receipt.write.byte_length,
            fs::metadata(&destination).unwrap().len()
        );
        assert_eq!(
            serialize_project_document_v6(&loaded.document).unwrap(),
            expected_document
        );
        assert!(matches!(
            &loaded.document.models[0].payload,
            ProjectModelPayloadV6::SemModelV4Draft { .. }
        ));
        assert_eq!(loaded.document.historical_recipes.len(), 1);
        assert_eq!(loaded.document.historical_results.len(), 1);
        assert_eq!(loaded.document.layouts, plan.document.layouts);
        assert!(loaded.document.upgrade_lineage().is_some());
        for (name, expected) in expected_arrow {
            let actual = zip_entry(&destination, &name);
            assert_eq!(actual, expected);
            assert_eq!(loaded.manifest.checksums[&name], sha256_bytes(&actual));
        }
        let entry_names = {
            let mut archive = ZipArchive::new(File::open(&destination).unwrap()).unwrap();
            (0..archive.len())
                .map(|index| archive.by_index(index).unwrap().name().to_owned())
                .collect::<BTreeSet<_>>()
        };
        let expected_entries = plan
            .document
            .datasets
            .iter()
            .map(|descriptor| format!("data/{}.arrow", descriptor.id))
            .chain([
                PROJECT_ENTRY_NAME.to_owned(),
                MANIFEST_ENTRY_NAME.to_owned(),
            ])
            .collect::<BTreeSet<_>>();
        assert_eq!(entry_names, expected_entries);
        assert!(temporary_upgrade_files(directory.path()).is_empty());
    }

    #[test]
    fn rejects_existing_or_racing_destination_without_clobbering_it() {
        let (directory, source, destination, plan, _) = upgrade_fixture();
        fs::write(&destination, b"existing destination").unwrap();
        assert!(matches!(
            execute_project_upgrade_zip_copy_v6(&source, &destination, &plan),
            Err(ProjectArchiveUpgradeZipV6Error::Contract(
                ProjectArchiveV6Error::DestinationExists(_)
            ))
        ));
        assert_eq!(fs::read(&destination).unwrap(), b"existing destination");
        fs::remove_file(&destination).unwrap();

        let raced = Cell::new(false);
        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                if !raced.get() && !temporary_upgrade_files(directory.path()).is_empty() {
                    fs::write(&destination, b"racing destination").unwrap();
                    raced.set(true);
                }
                false
            })
            .unwrap_err();
        assert!(matches!(
            error,
            ProjectArchiveUpgradeZipV6Error::Contract(ProjectArchiveV6Error::DestinationExists(_))
        ));
        assert!(raced.get());
        assert_eq!(fs::read(&destination).unwrap(), b"racing destination");
        assert!(temporary_upgrade_files(directory.path()).is_empty());
    }

    #[test]
    fn cancellation_before_and_after_publish_removes_only_writer_owned_files() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                !temporary_upgrade_files(directory.path()).is_empty()
            })
            .unwrap_err();
        assert!(matches!(error, ProjectArchiveUpgradeZipV6Error::Cancelled));
        assert!(!destination.exists());
        assert!(temporary_upgrade_files(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);

        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                destination.exists()
            })
            .unwrap_err();
        assert!(matches!(error, ProjectArchiveUpgradeZipV6Error::Cancelled));
        assert!(!destination.exists());
        assert!(temporary_upgrade_files(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn denied_commit_arbitration_removes_the_validated_destination() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let commit_called = Cell::new(false);

        let error = execute_project_upgrade_zip_copy_v6_with_control(
            &source,
            &destination,
            &plan,
            || false,
            || {
                commit_called.set(true);
                false
            },
        )
        .unwrap_err();

        assert!(matches!(error, ProjectArchiveUpgradeZipV6Error::Cancelled));
        assert!(commit_called.get());
        assert!(!destination.exists());
        assert!(temporary_upgrade_files(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn commit_callback_cannot_replace_destination_and_return_success() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let commit_called = Cell::new(false);

        let error = execute_project_upgrade_zip_copy_v6_with_control(
            &source,
            &destination,
            &plan,
            || false,
            || {
                commit_called.set(true);
                fs::remove_file(&destination).unwrap();
                fs::write(&destination, b"external commit replacement").unwrap();
                true
            },
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ProjectArchiveUpgradeZipV6Error::DestinationOwnershipLost { .. }
        ));
        assert!(commit_called.get());
        assert_eq!(
            fs::read(&destination).unwrap(),
            b"external commit replacement"
        );
        assert!(temporary_upgrade_files(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn cleanup_preserves_a_replacement_that_takes_over_the_destination_name() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let replaced = Cell::new(false);

        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                if destination.exists() && !replaced.get() {
                    fs::remove_file(&destination).unwrap();
                    fs::write(&destination, b"external replacement").unwrap();
                    replaced.set(true);
                    true
                } else {
                    false
                }
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectArchiveUpgradeZipV6Error::DestinationOwnershipLost { .. }
        ));
        assert!(replaced.get());
        assert_eq!(fs::read(&destination).unwrap(), b"external replacement");
        assert!(temporary_upgrade_files(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn cleanup_preserves_a_replacement_that_takes_over_the_temporary_name() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let replaced = Cell::new(false);

        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                if !replaced.get() {
                    if let Some(path) = temporary_upgrade_files(directory.path())
                        .into_iter()
                        .find(|path| load_project_archive_v6(path).is_ok())
                    {
                        fs::remove_file(&path).unwrap();
                        fs::write(&path, b"external temporary replacement").unwrap();
                        replaced.set(true);
                    }
                }
                false
            })
            .unwrap_err();

        assert!(matches!(
            error,
            ProjectArchiveUpgradeZipV6Error::TemporaryOwnershipLost { .. }
        ));
        assert!(replaced.get());
        let retained = temporary_upgrade_files(directory.path());
        assert_eq!(retained.len(), 1);
        assert_eq!(
            fs::read(&retained[0]).unwrap(),
            b"external temporary replacement"
        );
        assert!(!destination.exists());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn source_recheck_and_post_publish_validation_remove_failed_copy() {
        let (directory, source, destination, plan, _) = upgrade_fixture();
        let changed = Cell::new(false);
        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                if !changed.get()
                    && temporary_upgrade_files(directory.path())
                        .iter()
                        .any(|path| load_project_archive_v6(path).is_ok())
                {
                    fs::write(&source, b"external source change").unwrap();
                    changed.set(true);
                }
                false
            })
            .unwrap_err();
        assert!(matches!(
            error,
            ProjectArchiveUpgradeZipV6Error::Contract(
                ProjectArchiveV6Error::SourceChangedDuringUpgrade
            )
        ));
        assert!(changed.get());
        assert!(!destination.exists());
        assert!(temporary_upgrade_files(directory.path()).is_empty());

        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();
        let tampered = Cell::new(false);
        let error =
            execute_project_upgrade_zip_copy_v6_with_cancel(&source, &destination, &plan, || {
                if destination.exists() && !tampered.get() {
                    fs::write(&destination, b"post-publish tamper").unwrap();
                    tampered.set(true);
                }
                false
            })
            .unwrap_err();
        assert!(matches!(
            error,
            ProjectArchiveUpgradeZipV6Error::ArchiveValidation {
                phase: "persisted",
                ..
            }
        ));
        assert!(tampered.get());
        assert!(!destination.exists());
        assert!(temporary_upgrade_files(directory.path()).is_empty());
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
    }

    #[test]
    fn rejects_plan_dataset_drift_without_writing() {
        let (directory, source, destination, mut plan, source_bytes) = upgrade_fixture();
        plan.document.datasets[0].name.push_str(" changed");
        plan.ensure_valid().unwrap();

        assert!(matches!(
            execute_project_upgrade_zip_copy_v6(&source, &destination, &plan),
            Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(_))
        ));
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert!(!destination.exists());
        assert!(temporary_upgrade_files(directory.path()).is_empty());
    }

    #[test]
    fn rejects_valid_plan_drift_in_preserved_lanes_and_model_conversion() {
        let (directory, source, destination, plan, source_bytes) = upgrade_fixture();

        let mut layout_drift = plan.clone();
        layout_drift.document.layouts.insert(
            "workspace".into(),
            json!({"zoom": 9.0, "selection": [], "draft": false}),
        );
        layout_drift.ensure_valid().unwrap();

        let mut history_drift = plan.clone();
        history_drift.document.historical_recipes.clear();
        history_drift.document.historical_results.clear();
        history_drift.ensure_valid().unwrap();

        let mut model_drift = plan.clone();
        let ProjectModelPayloadV6::SemModelV4Draft {
            model,
            model_document_sha256,
        } = &mut model_drift.document.models[0].payload
        else {
            panic!("fixture must carry the exact converted model as a draft");
        };
        model.name.push_str(" substituted");
        *model_document_sha256 = model.model_document_sha256().unwrap();
        model_drift.ensure_valid().unwrap();

        for drifted in [&layout_drift, &history_drift, &model_drift] {
            assert!(matches!(
                execute_project_upgrade_zip_copy_v6(&source, &destination, drifted),
                Err(ProjectArchiveUpgradeZipV6Error::SourcePlanMismatch(_))
            ));
            assert!(!destination.exists());
        }
        assert_eq!(fs::read(&source).unwrap(), source_bytes);
        assert!(temporary_upgrade_files(directory.path()).is_empty());
    }
}
