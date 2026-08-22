//! Exact native adoption bridge for an already-inspected schema-6 General SEM archive.
//!
//! Schema 6 is intentionally rejected by the legacy `open_project` command. This
//! bridge does not weaken that boundary: it strictly reopens the exact archive,
//! rechecks every caller-supplied identity, rejects legacy autosave recovery, and
//! installs only a read-only in-memory revision source. The strict schema-6
//! session remains authoritative for model, RecipeV4, and result presentation.

use crate::{
    DesktopProject,
    project_archive_v6_general_sem_bootstrap::{
        DesktopGeneralSemFreshDraftAuthorityV1, GeneralSemNewProjectModeV1,
    },
};
use qpls_core::{AnalysisRecipeModelBindingV4, AnalysisRecipeV4, sha256_serialized};
use qpls_project::{
    PROJECT_ARCHIVE_SCHEMA_V6_VERSION, Project, ProjectModelPayloadV6, autosave_path,
    load_project_archive_v6_from_file,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};
use tauri::State;
use uuid::Uuid;

const DIAGNOSTIC_PREFIX: &str = "schema6_native_adoption";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6NativeAdoptionRequestV1 {
    archive_path: String,
    expected_archive_sha256: String,
    expected_archive_bytes: u64,
    expected_project_id: String,
    expected_dataset_id: String,
    expected_dataset_fingerprint: String,
    expected_model_id: String,
    expected_model_scientific_sha256: String,
    expected_recipe_id: String,
    expected_recipe_document_sha256: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProjectArchiveV6NativeAdoptionReceiptV1 {
    schema_version: u32,
    archive_path: String,
    archive_sha256: String,
    archive_bytes: u64,
    project_id: String,
    dataset_id: String,
    dataset_fingerprint: String,
    model_id: String,
    model_scientific_sha256: String,
    recipe_id: String,
    recipe_document_sha256: String,
    read_only: bool,
    autosave_recovery_used: bool,
    source_rechecked_unchanged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectArchiveV6NativeAdoptionBindingV1 {
    request: ProjectArchiveV6NativeAdoptionRequestV1,
}

/// Backend-only binding between the native in-memory revision source and the
/// exact schema-6 archive identity that established it.
#[derive(Clone, Default)]
pub(crate) struct DesktopSchema6NativeAdoptionAuthorityV1(
    Arc<Mutex<Option<ProjectArchiveV6NativeAdoptionBindingV1>>>,
);

fn failure(code: &str, message: impl AsRef<str>) -> String {
    format!("{DIAGNOSTIC_PREFIX}.{code}: {}", message.as_ref())
}

fn lowercase_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_file_handle(file: &mut File) -> Result<(u64, String), String> {
    file.seek(SeekFrom::Start(0))
        .map_err(|error| failure("source_unavailable", error.to_string()))?;
    let mut digest = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| failure("source_unavailable", error.to_string()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
        bytes += read as u64;
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}

fn select_general_sem_authority_recipe_v1(
    recipes: &[AnalysisRecipeV4],
) -> Result<&AnalysisRecipeV4, String> {
    let candidates = recipes
        .iter()
        .filter(|recipe| recipe.general_sem_config.is_some())
        .collect::<Vec<_>>();
    let [recipe] = candidates.as_slice() else {
        return Err(failure(
            "recipe_authority_mismatch",
            "the bounded General SEM archive must contain exactly one GeneralSemConfig project-model RecipeV4 authority",
        ));
    };
    Ok(*recipe)
}

fn adopt_exact_schema6_general_sem_project(
    request: &ProjectArchiveV6NativeAdoptionRequestV1,
) -> Result<(Project, ProjectArchiveV6NativeAdoptionReceiptV1), String> {
    if request.archive_path.is_empty() || request.archive_path.trim() != request.archive_path {
        return Err(failure(
            "archive_path_invalid",
            "archivePath must be nonempty without surrounding whitespace",
        ));
    }
    let archive_path = Path::new(&request.archive_path);
    if !archive_path.is_absolute() {
        return Err(failure(
            "absolute_path_required",
            "archivePath must identify the exact absolute local schema-6 archive",
        ));
    }
    if !lowercase_sha256(&request.expected_archive_sha256)
        || !lowercase_sha256(&request.expected_model_scientific_sha256)
        || !lowercase_sha256(&request.expected_recipe_document_sha256)
    {
        return Err(failure(
            "sha256_invalid",
            "all expected SHA-256 identities must be lowercase hexadecimal digests",
        ));
    }
    if request.expected_archive_bytes == 0 {
        return Err(failure(
            "archive_bytes_invalid",
            "expectedArchiveBytes must be positive",
        ));
    }
    let expected_project_id = Uuid::parse_str(&request.expected_project_id)
        .ok()
        .filter(|project_id| !project_id.is_nil())
        .ok_or_else(|| {
            failure(
                "project_id_invalid",
                "expectedProjectId must be a non-nil UUID",
            )
        })?;
    let expected_dataset_id = Uuid::parse_str(&request.expected_dataset_id)
        .ok()
        .filter(|dataset_id| !dataset_id.is_nil())
        .ok_or_else(|| {
            failure(
                "dataset_id_invalid",
                "expectedDatasetId must be a non-nil UUID",
            )
        })?;
    let expected_recipe_id = Uuid::parse_str(&request.expected_recipe_id)
        .ok()
        .filter(|recipe_id| !recipe_id.is_nil())
        .ok_or_else(|| {
            failure(
                "recipe_id_invalid",
                "expectedRecipeId must be a non-nil UUID",
            )
        })?;
    if request.expected_dataset_fingerprint.is_empty() || request.expected_model_id.is_empty() {
        return Err(failure(
            "authority_identity_invalid",
            "dataset fingerprint and model identity must be nonempty",
        ));
    }

    match fs::symlink_metadata(archive_path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            return Err(failure(
                "regular_file_required",
                "the schema-6 source must be a regular local file and cannot be a symbolic link",
            ));
        }
        Err(error) => return Err(failure("source_unavailable", error.to_string())),
    }
    if autosave_path(archive_path).exists() {
        return Err(failure(
            "autosave_present",
            "a legacy autosave exists beside the schema-6 source; resolve it explicitly before adoption",
        ));
    }

    let mut source = File::open(archive_path)
        .map_err(|error| failure("source_unavailable", error.to_string()))?;
    let (before_bytes, before_sha256) = sha256_file_handle(&mut source)?;
    if before_bytes != request.expected_archive_bytes
        || before_sha256 != request.expected_archive_sha256
    {
        return Err(failure(
            "archive_changed",
            "the schema-6 source no longer matches its strict inspection receipt",
        ));
    }
    let loaded = load_project_archive_v6_from_file(
        source
            .try_clone()
            .map_err(|error| failure("source_unavailable", error.to_string()))?,
    )
    .map_err(|error| failure("invalid_archive", error.to_string()))?;
    let (same_handle_bytes, same_handle_sha256) = sha256_file_handle(&mut source)?;
    let mut rebound = File::open(archive_path)
        .map_err(|error| failure("source_unavailable", error.to_string()))?;
    let (rebound_bytes, rebound_sha256) = sha256_file_handle(&mut rebound)?;
    if same_handle_bytes != before_bytes
        || same_handle_sha256 != before_sha256
        || rebound_bytes != before_bytes
        || rebound_sha256 != before_sha256
    {
        return Err(failure(
            "source_changed_during_read",
            "the schema-6 source changed while its exact native adoption was being validated",
        ));
    }

    if loaded.manifest.schema_version != PROJECT_ARCHIVE_SCHEMA_V6_VERSION
        || loaded.manifest.project_id != expected_project_id
        || loaded.document.project_id != expected_project_id
        || !loaded.document.supports_general_sem_v1()
    {
        return Err(failure(
            "project_authority_mismatch",
            "the strictly reopened archive differs from the expected marked General SEM project",
        ));
    }
    let [dataset] = loaded.datasets.as_slice() else {
        return Err(failure(
            "dataset_authority_mismatch",
            "the bounded General SEM archive must contain exactly one resident dataset",
        ));
    };
    let [model_record] = loaded.document.models.as_slice() else {
        return Err(failure(
            "model_authority_mismatch",
            "the bounded General SEM archive must contain exactly one resident model authority",
        ));
    };
    let ProjectModelPayloadV6::SemModelV4 {
        scientific_sha256, ..
    } = &model_record.payload
    else {
        return Err(failure(
            "model_authority_mismatch",
            "the resident General SEM model is not a promoted SemModelV4 authority",
        ));
    };
    let recipe = select_general_sem_authority_recipe_v1(&loaded.document.recipes)?;
    let AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
        model_id: bound_model_id,
        scientific_sha256: bound_model_sha256,
    } = &recipe.model_binding
    else {
        return Err(failure(
            "recipe_authority_mismatch",
            "the resident RecipeV4 does not reference the project SemModelV4 authority",
        ));
    };
    let recipe_document_sha256 = sha256_serialized(recipe);
    if dataset.id != expected_dataset_id
        || dataset.fingerprint.0 != request.expected_dataset_fingerprint
        || model_record.model_id != request.expected_model_id
        || scientific_sha256 != &request.expected_model_scientific_sha256
        || bound_model_id != &model_record.model_id
        || bound_model_sha256 != scientific_sha256
        || recipe.id != expected_recipe_id
        || recipe.dataset_fingerprint != dataset.fingerprint.0
        || recipe.general_sem_config.is_none()
        || recipe_document_sha256 != request.expected_recipe_document_sha256
    {
        return Err(failure(
            "execution_authority_mismatch",
            "the resident dataset, model, or RecipeV4 differs from the strictly inspected execution authority",
        ));
    }

    let receipt = ProjectArchiveV6NativeAdoptionReceiptV1 {
        schema_version: 1,
        archive_path: request.archive_path.clone(),
        archive_sha256: before_sha256,
        archive_bytes: before_bytes,
        project_id: expected_project_id.to_string(),
        dataset_id: expected_dataset_id.to_string(),
        dataset_fingerprint: request.expected_dataset_fingerprint.clone(),
        model_id: request.expected_model_id.clone(),
        model_scientific_sha256: scientific_sha256.clone(),
        recipe_id: expected_recipe_id.to_string(),
        recipe_document_sha256,
        read_only: true,
        autosave_recovery_used: false,
        source_rechecked_unchanged: true,
    };

    // Preserve the exact resident Arrow dataset and its lineage layout for the
    // source-preserving revision bootstrap. Schema-6 models and RecipeV4 never
    // enter the legacy Project model/recipe vectors.
    let mut project = Project::new(loaded.document.name.clone());
    project.manifest = loaded.manifest;
    project.datasets = loaded.datasets;
    project.layouts = loaded.document.layouts;
    project.read_only = true;
    project.source_archive_version = PROJECT_ARCHIVE_SCHEMA_V6_VERSION;
    project.migration_pending = false;
    Ok((project, receipt))
}

fn exact_native_projection_matches(active: &Project, expected: &Project) -> bool {
    active.read_only
        && active.source_archive_version == PROJECT_ARCHIVE_SCHEMA_V6_VERSION
        && !active.migration_pending
        && active.manifest.schema_version == expected.manifest.schema_version
        && active.manifest.project_id == expected.manifest.project_id
        && active.manifest.name == expected.manifest.name
        && active.manifest.created_at == expected.manifest.created_at
        && active.manifest.modified_at == expected.manifest.modified_at
        && active.manifest.engine_version == expected.manifest.engine_version
        && active.manifest.checksum_algorithm == expected.manifest.checksum_algorithm
        && active.manifest.checksums == expected.manifest.checksums
        && active.layouts == expected.layouts
        && active.models.is_empty()
        && active.recipes.is_empty()
        && active.results.is_empty()
        && active.datasets.len() == expected.datasets.len()
        && active
            .datasets
            .iter()
            .zip(&expected.datasets)
            .all(|(active, expected)| {
                active.id == expected.id
                    && active.name == expected.name
                    && active.schema == expected.schema
                    && active.fingerprint == expected.fingerprint
                    && active.batch == expected.batch
            })
}

impl DesktopSchema6NativeAdoptionAuthorityV1 {
    fn lock_recovering(&self) -> MutexGuard<'_, Option<ProjectArchiveV6NativeAdoptionBindingV1>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn replace_from_new_project(
        &self,
        active_project: &Arc<Mutex<Project>>,
        fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
        replacement: Project,
        project_mode: Option<GeneralSemNewProjectModeV1>,
    ) -> Result<(), String> {
        let mut binding = self.lock_recovering();
        fresh_draft_authority.replace_from_new_project(
            active_project,
            replacement,
            project_mode,
        )?;
        *binding = None;
        Ok(())
    }

    pub(crate) fn replace_ordinary_project(
        &self,
        active_project: &Arc<Mutex<Project>>,
        fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
        replacement: Project,
    ) -> Result<(), String> {
        let mut binding = self.lock_recovering();
        fresh_draft_authority.replace_and_clear(active_project, replacement)?;
        *binding = None;
        Ok(())
    }

    fn replace_with_exact_schema6_binding(
        &self,
        active_project: &Arc<Mutex<Project>>,
        fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
        replacement: Project,
        request: ProjectArchiveV6NativeAdoptionRequestV1,
    ) -> Result<(), String> {
        let mut binding = self.lock_recovering();
        fresh_draft_authority.replace_and_clear(active_project, replacement)?;
        *binding = Some(ProjectArchiveV6NativeAdoptionBindingV1 { request });
        Ok(())
    }

    pub(crate) fn clear_and_invalidate(
        &self,
        fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
    ) {
        let mut binding = self.lock_recovering();
        fresh_draft_authority.clear();
        *binding = None;
    }

    pub(crate) fn authorize_exact_revision(
        &self,
        active_project: &Arc<Mutex<Project>>,
        fresh_draft_authority: &DesktopGeneralSemFreshDraftAuthorityV1,
    ) -> Result<Uuid, String> {
        let mut binding = self.lock_recovering();
        let Some(stored_binding) = binding.as_ref() else {
            let ordinary_writable_project = active_project
                .lock()
                .map(|active| {
                    !active.read_only
                        && active.source_archive_version != PROJECT_ARCHIVE_SCHEMA_V6_VERSION
                })
                .unwrap_or(false);
            if ordinary_writable_project {
                return fresh_draft_authority.authorize_existing_project_revision(active_project);
            }
            return Err(failure(
                "binding_required",
                "a read-only schema-6 native revision source requires its exact archive binding",
            ));
        };
        let request = stored_binding.request.clone();
        let (revalidated, _) = match adopt_exact_schema6_general_sem_project(&request) {
            Ok(value) => value,
            Err(error) => {
                fresh_draft_authority.clear();
                *binding = None;
                return Err(error);
            }
        };
        let active_matches = active_project
            .lock()
            .map(|active| exact_native_projection_matches(&active, &revalidated))
            .unwrap_or(false);
        if !active_matches {
            fresh_draft_authority.clear();
            *binding = None;
            return Err(failure(
                "active_project_mismatch",
                "the active native revision source differs from its exact schema-6 binding",
            ));
        }
        match fresh_draft_authority.authorize_existing_project_revision(active_project) {
            Ok(project_id) => Ok(project_id),
            Err(error) => {
                fresh_draft_authority.clear();
                *binding = None;
                Err(error)
            }
        }
    }

    #[cfg(test)]
    fn has_binding(&self) -> bool {
        self.lock_recovering().is_some()
    }
}

#[tauri::command]
pub(crate) fn adopt_internal_project_archive_v6_native_revision_source_v1(
    request: ProjectArchiveV6NativeAdoptionRequestV1,
    project: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
    native_adoption_authority: State<'_, DesktopSchema6NativeAdoptionAuthorityV1>,
) -> Result<ProjectArchiveV6NativeAdoptionReceiptV1, String> {
    let (replacement, receipt) = adopt_exact_schema6_general_sem_project(&request)?;
    native_adoption_authority.replace_with_exact_schema6_binding(
        &project.0,
        fresh_draft_authority.inner(),
        replacement,
        request,
    )?;
    Ok(receipt)
}

/// Explicitly releases the native schema-6 revision source during a real
/// project close or failed activation rollback. Fresh-draft invalidation alone
/// intentionally preserves this binding because Standard activation invokes it.
#[tauri::command]
pub(crate) fn clear_internal_project_archive_v6_native_revision_source_v1(
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
    native_adoption_authority: State<'_, DesktopSchema6NativeAdoptionAuthorityV1>,
) {
    native_adoption_authority.clear_and_invalidate(fresh_draft_authority.inner());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_archive_v6_general_sem_bootstrap::tests::general_sem_native_fixture_v1;
    use chrono::{TimeZone, Utc};
    use qpls_project::{create_populated_general_sem_project_archive_v6, load_project_archive_v6};
    use std::{fs::OpenOptions, io::Write};
    use tempfile::tempdir;

    fn fixture_request(path: &Path) -> ProjectArchiveV6NativeAdoptionRequestV1 {
        let fixture = general_sem_native_fixture_v1();
        let project_id = Uuid::from_u128(0x6e61_7469_7665_5f61_646f_7074_0001);
        let receipt = create_populated_general_sem_project_archive_v6(
            path,
            project_id,
            "Adopted schema-6 General SEM",
            Utc.timestamp_opt(1_700_000_500, 0).unwrap(),
            &fixture.dataset,
            fixture.model,
            fixture.recipe,
        )
        .unwrap();
        let loaded = load_project_archive_v6(path).unwrap();
        let model = &loaded.document.models[0];
        let ProjectModelPayloadV6::SemModelV4 {
            scientific_sha256, ..
        } = &model.payload
        else {
            unreachable!()
        };
        let recipe = &loaded.document.recipes[0];
        ProjectArchiveV6NativeAdoptionRequestV1 {
            archive_path: path.to_string_lossy().into_owned(),
            expected_archive_sha256: receipt.destination_archive_sha256,
            expected_archive_bytes: fs::metadata(path).unwrap().len(),
            expected_project_id: project_id.to_string(),
            expected_dataset_id: fixture.dataset.id.to_string(),
            expected_dataset_fingerprint: fixture.dataset.fingerprint.0,
            expected_model_id: model.model_id.clone(),
            expected_model_scientific_sha256: scientific_sha256.clone(),
            expected_recipe_id: recipe.id.to_string(),
            expected_recipe_document_sha256: sha256_serialized(recipe),
        }
    }

    fn installed_binding(
        path: &Path,
    ) -> (
        DesktopSchema6NativeAdoptionAuthorityV1,
        DesktopGeneralSemFreshDraftAuthorityV1,
        Arc<Mutex<Project>>,
        ProjectArchiveV6NativeAdoptionRequestV1,
    ) {
        let request = fixture_request(path);
        let (project, _) = adopt_exact_schema6_general_sem_project(&request).unwrap();
        let adoption = DesktopSchema6NativeAdoptionAuthorityV1::default();
        let fresh = DesktopGeneralSemFreshDraftAuthorityV1::default();
        let active = Arc::new(Mutex::new(Project::new("Prior project")));
        adoption
            .replace_with_exact_schema6_binding(&active, &fresh, project, request.clone())
            .unwrap();
        (adoption, fresh, active, request)
    }

    #[test]
    fn authority_selector_tolerates_one_non_authority_exact_result_recipe_only() {
        let fixture = general_sem_native_fixture_v1();
        let authority = fixture.recipe;
        let mut exact_result_recipe = authority.clone();
        exact_result_recipe.id = Uuid::from_u128(0x6e61_7469_7665_5f61_646f_7074_0099);
        exact_result_recipe.general_sem_config = None;
        let recipes = [authority.clone(), exact_result_recipe];
        let selected = select_general_sem_authority_recipe_v1(&recipes).unwrap();
        assert_eq!(selected.id, authority.id);

        let mut second_authority = authority.clone();
        second_authority.id = Uuid::from_u128(0x6e61_7469_7665_5f61_646f_7074_0098);
        assert!(
            select_general_sem_authority_recipe_v1(&[authority, second_authority])
                .unwrap_err()
                .contains("schema6_native_adoption.recipe_authority_mismatch")
        );
    }

    #[test]
    fn exact_schema6_general_sem_source_is_adopted_read_only_without_legacy_projection() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("exact-source.qpls");
        let request = fixture_request(&path);

        let (project, receipt) = adopt_exact_schema6_general_sem_project(&request).unwrap();

        assert_eq!(receipt.project_id, request.expected_project_id);
        assert_eq!(receipt.archive_sha256, request.expected_archive_sha256);
        assert!(receipt.read_only);
        assert!(!receipt.autosave_recovery_used);
        assert!(receipt.source_rechecked_unchanged);
        assert_eq!(
            project.manifest.project_id.to_string(),
            request.expected_project_id
        );
        assert_eq!(
            project.source_archive_version,
            PROJECT_ARCHIVE_SCHEMA_V6_VERSION
        );
        assert!(project.read_only);
        assert_eq!(project.datasets.len(), 1);
        assert!(project.models.is_empty());
        assert!(project.recipes.is_empty());
        assert!(project.results.is_empty());
    }

    #[test]
    fn digest_or_execution_authority_mismatch_fails_before_project_replacement() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("mismatch-source.qpls");
        let exact_request = fixture_request(&path);
        let mut request = exact_request.clone();
        request.expected_archive_sha256 = "0".repeat(64);
        assert!(
            adopt_exact_schema6_general_sem_project(&request)
                .unwrap_err()
                .contains("schema6_native_adoption.archive_changed")
        );

        let mut request = exact_request;
        request.expected_model_scientific_sha256 = "0".repeat(64);
        assert!(
            adopt_exact_schema6_general_sem_project(&request)
                .unwrap_err()
                .contains("schema6_native_adoption.execution_authority_mismatch")
        );
    }

    #[test]
    fn adjacent_legacy_autosave_is_never_consulted_or_silently_ignored() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("autosave-source.qpls");
        let request = fixture_request(&path);
        fs::write(autosave_path(&path), b"untrusted recovery bytes").unwrap();

        assert!(
            adopt_exact_schema6_general_sem_project(&request)
                .unwrap_err()
                .contains("schema6_native_adoption.autosave_present")
        );
    }

    #[test]
    fn tamper_between_adoption_and_revision_authorization_revokes_the_binding() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("tampered-after-adoption.qpls");
        let (adoption, fresh, active, _) = installed_binding(&path);
        assert!(adoption.has_binding());
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"tamper")
            .unwrap();

        let error = adoption
            .authorize_exact_revision(&active, &fresh)
            .unwrap_err();

        assert!(error.contains("schema6_native_adoption.archive_changed"));
        assert!(!adoption.has_binding());
    }

    #[test]
    fn standard_fresh_draft_revocation_preserves_exact_adoption_for_revision() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("standard-activation.qpls");
        let (adoption, fresh, active, request) = installed_binding(&path);

        fresh.clear();

        assert!(adoption.has_binding());
        assert_eq!(
            adoption.authorize_exact_revision(&active, &fresh).unwrap(),
            Uuid::parse_str(&request.expected_project_id).unwrap()
        );
    }

    #[test]
    fn every_ordinary_new_or_invalidated_project_transition_clears_stale_binding() {
        let directory = tempdir().unwrap();
        let first_path = directory.path().join("ordinary-replacement.qpls");
        let (adoption, fresh, active, _) = installed_binding(&first_path);
        let legacy_replacement = Project::new("Legacy replacement");
        let legacy_project_id = legacy_replacement.manifest.project_id;
        adoption
            .replace_ordinary_project(&active, &fresh, legacy_replacement)
            .unwrap();
        assert!(!adoption.has_binding());
        assert_eq!(
            adoption.authorize_exact_revision(&active, &fresh).unwrap(),
            legacy_project_id
        );

        let second_path = directory.path().join("new-replacement.qpls");
        let (adoption, fresh, active, _) = installed_binding(&second_path);
        adoption
            .replace_from_new_project(
                &active,
                &fresh,
                Project::new("Fresh replacement"),
                Some(GeneralSemNewProjectModeV1::GeneralSemV1),
            )
            .unwrap();
        assert!(!adoption.has_binding());

        let third_path = directory.path().join("invalidated-replacement.qpls");
        let (adoption, fresh, active, _) = installed_binding(&third_path);
        adoption.clear_and_invalidate(&fresh);
        assert!(!adoption.has_binding());
        assert!(
            adoption
                .authorize_exact_revision(&active, &fresh)
                .unwrap_err()
                .contains("schema6_native_adoption.binding_required")
        );
    }
}
