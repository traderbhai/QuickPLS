//! Registry-driven bootstrap for a runnable schema-6 General SEM project.
//!
//! A backend-authorized fresh General SEM draft contributes exactly one
//! already-resident dataset. The caller supplies a newly authored SemModelV4
//! and bound RecipeV4; qpls-project validates the complete authority and
//! publishes a new `general_sem_v1` archive atomically. No source model, recipe,
//! result, layout, or project identity is migrated.

use crate::{
    DesktopProject,
    general_sem_registry_access_v1::{
        GENERAL_SEM_INTERNAL_LABS_SURFACE as INTERNAL_LABS_SURFACE,
        GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1,
        GENERAL_SEM_STANDARD_SURFACE as STANDARD_SURFACE, GeneralSemRegistryAccessErrorV1,
        authorize_general_sem_registry_access_v1, decision_declares_general_sem_execution_cell_v1,
        general_sem_recipe_execution_surface_v1, selected_general_sem_execution_cell_v1,
    },
};
use chrono::{DateTime, Utc};
use qpls_core::{
    AnalysisRecipeV4, CapabilityCellReferenceV2, SemModelV4, preflight_general_sem_pls_v1,
};
use qpls_project::{
    GeneralSemPopulatedProjectArchiveCreationReceiptV1, GeneralSemProjectArchiveCreationErrorV1,
    Project, ProjectArchiveV6SaveCopyError, ProjectDatasetVersionOperationV1,
    create_populated_general_sem_project_archive_v6, read_project_data_lineage_v1,
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard, TryLockError},
};
use tauri::State;
use uuid::Uuid;

const GENERAL_SEM_BOOTSTRAP_RESULT_SCHEMA_VERSION: u32 = 1;
const DIAGNOSTIC_CODE_PREFIX: &str = "schema6_general_sem_bootstrap";
const GENERAL_SEM_FRESH_DRAFT_UNMARKED_SAVE_BLOCKER: &str = "This fresh General SEM draft cannot be saved or autosaved as an unmarked project. Use General SEM > Save and activate project.";
const GENERAL_SEM_FRESH_DRAFT_AUTHORITY_MISMATCH_BLOCKER: &str = "The fresh General SEM draft authority does not match the active native project. Save and autosave are blocked until the project authority is safely replaced.";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GeneralSemNewProjectModeV1 {
    Standard,
    GeneralSemV1,
}

#[derive(Debug, Clone)]
struct GeneralSemFreshDraftAuthorizationV1 {
    project_id: Uuid,
    issuance_id: Uuid,
    claim_id: Option<Uuid>,
}

#[derive(Debug, Default)]
struct GeneralSemFreshDraftAuthorityStateV1 {
    authorization: Option<GeneralSemFreshDraftAuthorizationV1>,
}

/// Backend-only authority proving that the active in-memory project was
/// explicitly created as a fresh General SEM draft. The token never crosses
/// the Tauri boundary and cannot be manufactured from a canvas payload.
#[derive(Clone, Default)]
pub(crate) struct DesktopGeneralSemFreshDraftAuthorityV1(
    Arc<Mutex<GeneralSemFreshDraftAuthorityStateV1>>,
);

impl DesktopGeneralSemFreshDraftAuthorityV1 {
    fn lock_recovering(&self) -> MutexGuard<'_, GeneralSemFreshDraftAuthorityStateV1> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn replace_active_project(
        &self,
        active_project: &Arc<Mutex<Project>>,
        replacement: Project,
        authorize_general_sem_draft: bool,
    ) -> Result<(), String> {
        let replacement_project_id = replacement.manifest.project_id;
        let mut authority = self.lock_recovering();
        let mut active = active_project
            .lock()
            .map_err(|_| "project state is unavailable".to_owned())?;
        *active = replacement;
        authority.authorization =
            authorize_general_sem_draft.then(|| GeneralSemFreshDraftAuthorizationV1 {
                project_id: replacement_project_id,
                issuance_id: Uuid::new_v4(),
                claim_id: None,
            });
        Ok(())
    }

    /// The only production issuance boundary: `new_project` passes its parsed
    /// mode here, and an omitted or Standard mode clears any prior token.
    pub(crate) fn replace_from_new_project(
        &self,
        active_project: &Arc<Mutex<Project>>,
        replacement: Project,
        project_mode: Option<GeneralSemNewProjectModeV1>,
    ) -> Result<(), String> {
        self.replace_active_project(
            active_project,
            replacement,
            matches!(project_mode, Some(GeneralSemNewProjectModeV1::GeneralSemV1)),
        )
    }

    pub(crate) fn replace_and_clear(
        &self,
        active_project: &Arc<Mutex<Project>>,
        replacement: Project,
    ) -> Result<(), String> {
        self.replace_active_project(active_project, replacement, false)
    }

    pub(crate) fn clear(&self) {
        self.lock_recovering().authorization = None;
    }

    /// Authorizes one source-preserving calculation-ready revision of the
    /// exact active project. The source project is never rewritten: the
    /// existing bootstrap command still requires a new project UUID and a new
    /// destination path before publishing the marked schema-6 authority.
    fn authorize_existing_project_revision(
        &self,
        active_project: &Arc<Mutex<Project>>,
    ) -> Result<Uuid, String> {
        let mut authority = self.lock_recovering();
        let project = active_project
            .lock()
            .map_err(|_| "project state is unavailable".to_owned())?;
        let project_id = project.manifest.project_id;
        if project_id.is_nil() {
            return Err("the active project has no stable project identity".to_owned());
        }
        authority.authorization = Some(GeneralSemFreshDraftAuthorizationV1 {
            project_id,
            issuance_id: Uuid::new_v4(),
            claim_id: None,
        });
        Ok(project_id)
    }

    /// Locks the active project for historical desktop persistence only after
    /// proving that it is not the exact fresh General SEM draft. The authority
    /// lock is always acquired before the project lock; it is released once the
    /// project guard makes replacement impossible for the remainder of the
    /// save operation.
    pub(crate) fn lock_project_for_unmarked_save<'a>(
        &self,
        active_project: &'a Arc<Mutex<Project>>,
    ) -> Result<MutexGuard<'a, Project>, String> {
        let authority = self.lock_recovering();
        let project = active_project
            .lock()
            .map_err(|_| "project state is unavailable".to_owned())?;
        if let Some(authorization) = authority.authorization.as_ref() {
            if authorization.project_id == project.manifest.project_id {
                return Err(GENERAL_SEM_FRESH_DRAFT_UNMARKED_SAVE_BLOCKER.to_owned());
            }
            return Err(GENERAL_SEM_FRESH_DRAFT_AUTHORITY_MISMATCH_BLOCKER.to_owned());
        }
        drop(authority);
        Ok(project)
    }

    #[cfg(test)]
    fn issue_for_test(&self, project_id: Uuid) {
        self.lock_recovering().authorization = Some(GeneralSemFreshDraftAuthorizationV1 {
            project_id,
            issuance_id: Uuid::new_v4(),
            claim_id: None,
        });
    }
}

/// One-way invalidation seam for strict schema-6 project activation paths that
/// do not replace `DesktopProject`. It deliberately cannot issue authority.
#[tauri::command]
pub(crate) fn invalidate_general_sem_fresh_draft_authority_v1(
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
) {
    fresh_draft_authority.clear();
}

#[tauri::command]
pub(crate) fn authorize_general_sem_revision_draft_v1(
    project: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
) -> Result<String, String> {
    fresh_draft_authority
        .authorize_existing_project_revision(&project.0)
        .map(|project_id| project_id.to_string())
}

struct GeneralSemFreshDraftClaimV1<'a> {
    authority: MutexGuard<'a, GeneralSemFreshDraftAuthorityStateV1>,
    issuance_id: Uuid,
    claim_id: Uuid,
    finalized: bool,
}

impl GeneralSemFreshDraftClaimV1<'_> {
    fn restore(mut self) {
        if let Some(authorization) = self.authority.authorization.as_mut() {
            if authorization.issuance_id == self.issuance_id
                && authorization.claim_id == Some(self.claim_id)
            {
                authorization.claim_id = None;
            }
        }
        self.finalized = true;
    }

    fn consume(mut self) {
        if self
            .authority
            .authorization
            .as_ref()
            .is_some_and(|authorization| {
                authorization.issuance_id == self.issuance_id
                    && authorization.claim_id == Some(self.claim_id)
            })
        {
            self.authority.authorization = None;
        }
        self.finalized = true;
    }
}

impl Drop for GeneralSemFreshDraftClaimV1<'_> {
    fn drop(&mut self) {
        if self.finalized {
            return;
        }
        if let Some(authorization) = self.authority.authorization.as_mut() {
            if authorization.issuance_id == self.issuance_id
                && authorization.claim_id == Some(self.claim_id)
            {
                authorization.claim_id = None;
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemProjectArchiveBootstrapRequestV1 {
    surface: String,
    experimental_labs_enabled: bool,
    capability_cell: CapabilityCellReferenceV2,
    destination_path: String,
    project_id: String,
    name: String,
    created_at: String,
    source_project_id: String,
    source_dataset_id: String,
    source_dataset_fingerprint: String,
    model: SemModelV4,
    recipe: AnalysisRecipeV4,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemProjectArchiveBootstrapResultV1 {
    schema_version: u32,
    receipt: GeneralSemPopulatedProjectArchiveCreationReceiptV1,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct GeneralSemProjectArchiveBootstrapDiagnosticV1 {
    code: String,
    message: String,
    corrective_action: String,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum GeneralSemProjectArchiveBootstrapOutcomeV1 {
    Ok {
        value: GeneralSemProjectArchiveBootstrapResultV1,
    },
    Blocked {
        diagnostic: GeneralSemProjectArchiveBootstrapDiagnosticV1,
    },
}

fn blocked(
    code_suffix: &str,
    message: impl Into<String>,
    corrective_action: impl Into<String>,
) -> GeneralSemProjectArchiveBootstrapOutcomeV1 {
    GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked {
        diagnostic: GeneralSemProjectArchiveBootstrapDiagnosticV1 {
            code: format!("{DIAGNOSTIC_CODE_PREFIX}.{code_suffix}"),
            message: message.into(),
            corrective_action: corrective_action.into(),
        },
    }
}

fn publication_error(
    error: GeneralSemProjectArchiveCreationErrorV1,
) -> GeneralSemProjectArchiveBootstrapOutcomeV1 {
    let GeneralSemProjectArchiveCreationErrorV1::Publication(error) = error;
    let (code_suffix, message, corrective_action) = match error {
        ProjectArchiveV6SaveCopyError::DestinationExists(_) => (
            "destination_exists",
            "The destination archive already exists and was not changed.",
            "Choose a new .qpls destination filename; existing files are never overwritten.",
        ),
        ProjectArchiveV6SaveCopyError::AbsolutePathsRequired
        | ProjectArchiveV6SaveCopyError::SourceAndDestinationMustDiffer
        | ProjectArchiveV6SaveCopyError::InvalidDestinationName(_)
        | ProjectArchiveV6SaveCopyError::DestinationExtension => (
            "destination_invalid",
            "The destination is not a valid absolute .qpls project path.",
            "Choose a new absolute .qpls destination path without surrounding whitespace.",
        ),
        ProjectArchiveV6SaveCopyError::DestinationParentMustBeLocalNonReparseDirectory
        | ProjectArchiveV6SaveCopyError::RemoteDestinationUnsupported
        | ProjectArchiveV6SaveCopyError::UnsupportedDestinationFilesystem(_) => (
            "destination_not_supported",
            "The destination folder does not support safe schema-6 publication.",
            "Choose a local NTFS folder whose immediate parent is not a reparse point.",
        ),
        ProjectArchiveV6SaveCopyError::UnsupportedPlatform => (
            "windows_desktop_required",
            "Schema-6 General SEM project bootstrap requires the Windows desktop writer.",
            "Use the installed QuickPLS Windows desktop application.",
        ),
        ProjectArchiveV6SaveCopyError::ArchiveLimit(_) => (
            "archive_limit",
            "The project archive exceeded a schema-6 safety limit.",
            "Use a supported resident dataset and retry with a new destination filename.",
        ),
        ProjectArchiveV6SaveCopyError::Contract(_)
        | ProjectArchiveV6SaveCopyError::Project(_)
        | ProjectArchiveV6SaveCopyError::NewDocumentRequiresEmptyDatasets
        | ProjectArchiveV6SaveCopyError::NonModelAuthorityChanged => (
            "authority_invalid",
            "The resident dataset, promoted model, and RecipeV4 do not form one valid General SEM authority.",
            "Refresh the dataset binding, validate the canvas model and recipe, then create a new General SEM project.",
        ),
        ProjectArchiveV6SaveCopyError::StrictReopenMismatch
        | ProjectArchiveV6SaveCopyError::DestinationIdentityChanged
        | ProjectArchiveV6SaveCopyError::PublicationFailed(_) => (
            "commit_validation_failed",
            "The project archive could not pass safe publication validation.",
            "Keep the General SEM draft unchanged and retry with a new local destination filename.",
        ),
        ProjectArchiveV6SaveCopyError::Io(_) => (
            "io_failed",
            "A local filesystem operation prevented the project archive from being created.",
            "Confirm the destination folder is available and writable, then retry with a new filename.",
        ),
        ProjectArchiveV6SaveCopyError::SourceMustBeRegularNonReparseFile
        | ProjectArchiveV6SaveCopyError::SourceDigestMismatch { .. }
        | ProjectArchiveV6SaveCopyError::SourceChangedDuringSave
        | ProjectArchiveV6SaveCopyError::CancelledBeforeCommit
        | ProjectArchiveV6SaveCopyError::Zip(_)
        | ProjectArchiveV6SaveCopyError::Json(_) => (
            "write_failed",
            "The General SEM project archive could not be created.",
            "Keep the General SEM draft unchanged and retry with a new local destination filename.",
        ),
    };
    blocked(code_suffix, message, corrective_action)
}

fn registry_access_block(
    request: &GeneralSemProjectArchiveBootstrapRequestV1,
) -> Option<GeneralSemProjectArchiveBootstrapOutcomeV1> {
    if let Err(error) = authorize_general_sem_registry_access_v1(
        &request.surface,
        request.experimental_labs_enabled,
        &request.capability_cell,
    ) {
        return Some(match error {
            GeneralSemRegistryAccessErrorV1::RegistryInvalid(detail) => blocked(
                "capability_registry_invalid",
                format!("Capability Registry V2 is invalid: {detail}"),
                "Keep the draft unchanged and repair the embedded registry before publication.",
            ),
            GeneralSemRegistryAccessErrorV1::CapabilityUnavailable => blocked(
                "capability_unavailable",
                "The exact requested General SEM option cell is not executable in Standard or Labs.",
                "Refresh the exact capability selection before publishing this project.",
            ),
            GeneralSemRegistryAccessErrorV1::StandardSurfaceRequired => blocked(
                "standard_surface_required",
                "The exact requested General SEM option cell is qualified for the Standard surface.",
                "Refresh capability access and publish through the Standard surface.",
            ),
            GeneralSemRegistryAccessErrorV1::InternalLabsRequired => blocked(
                "internal_labs_required",
                "The exact requested General SEM option cell is available only through Experimental Labs.",
                "Enable Experimental Labs, or choose a Standard-qualified General SEM cell.",
            ),
        });
    }
    let expected_recipe_surface = general_sem_recipe_execution_surface_v1(&request.surface)
        .expect("Registry authorization accepted one of the two General SEM surfaces");
    if request
        .recipe
        .metadata
        .get("execution_surface")
        .map(String::as_str)
        != Some(expected_recipe_surface)
        || request
            .recipe
            .metadata
            .get("general_sem_generation")
            .map(String::as_str)
            != Some("general_sem_v1")
    {
        return Some(blocked(
            "recipe_execution_surface_mismatch",
            "The RecipeV4 execution-surface metadata disagrees with the exact Registry-authorized request.",
            "Rebuild the recipe from the unchanged model, configuration, and exact selected cell.",
        ));
    }
    let Some(config) = request.recipe.general_sem_config.as_ref() else {
        return Some(blocked(
            "capability_cell_mismatch",
            "The bound RecipeV4 does not contain a General SEM configuration.",
            "Rebuild the recipe from the current marked General SEM draft.",
        ));
    };
    let decision = match preflight_general_sem_pls_v1(&request.model, config) {
        Ok(decision) => decision,
        Err(error) => {
            return Some(blocked(
                "capability_decision_invalid",
                format!("The exact PLS capability decision is invalid: {error}"),
                "Keep the draft unchanged and report this capability-contract error.",
            ));
        }
    };
    // Point provenance and supplemental bootstrap ownership are independent of
    // the capability decision's canonical cell ordering.
    let selected = selected_general_sem_execution_cell_v1(&request.model, config);
    if !decision_declares_general_sem_execution_cell_v1(&decision, &selected)
        || selected != request.capability_cell
    {
        return Some(blocked(
            "capability_cell_mismatch",
            "The requested option cell differs from the exact native model and RecipeV4 decision.",
            "Refresh the unchanged draft and rerun exact capability preflight.",
        ));
    }
    None
}

fn claim_fresh_general_sem_draft<'a>(
    request: &GeneralSemProjectArchiveBootstrapRequestV1,
    active_project: &Arc<Mutex<Project>>,
    authority: &'a DesktopGeneralSemFreshDraftAuthorityV1,
) -> Result<GeneralSemFreshDraftClaimV1<'a>, GeneralSemProjectArchiveBootstrapOutcomeV1> {
    let mut authority = match authority.0.try_lock() {
        Ok(authority) => authority,
        Err(TryLockError::WouldBlock) => {
            return Err(blocked(
                "fresh_draft_claim_in_progress",
                "A General SEM project bootstrap is already using the fresh-draft authorization.",
                "Wait for the current create operation to finish before retrying.",
            ));
        }
        Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner(),
    };
    let Some(authorization) = authority.authorization.as_ref() else {
        return Err(blocked(
            "fresh_draft_authority_required",
            "The active project was not created as a fresh General SEM draft.",
            "Use New General SEM Project before importing data or authoring the model.",
        ));
    };
    if authorization.claim_id.is_some() {
        return Err(blocked(
            "fresh_draft_claim_in_progress",
            "A General SEM project bootstrap is already using the fresh-draft authorization.",
            "Wait for the current create operation to finish before retrying.",
        ));
    }

    let requested_source_project_id = Uuid::parse_str(&request.source_project_id)
        .ok()
        .filter(|project_id| !project_id.is_nil());
    let active_project_id = active_project
        .lock()
        .map_err(|_| {
            blocked(
                "source_project_unavailable",
                "The active project authority is temporarily unavailable.",
                "Retry after the active project finishes its current operation.",
            )
        })?
        .manifest
        .project_id;
    if requested_source_project_id != Some(authorization.project_id)
        || active_project_id != authorization.project_id
    {
        return Err(blocked(
            "fresh_draft_project_mismatch",
            "The fresh General SEM draft authorization does not match the exact active project identity.",
            "Return to the newly created General SEM draft and rebuild the request from its current project snapshot.",
        ));
    }

    let issuance_id = authorization.issuance_id;
    let claim_id = Uuid::new_v4();
    authority
        .authorization
        .as_mut()
        .expect("authorization was checked above")
        .claim_id = Some(claim_id);
    Ok(GeneralSemFreshDraftClaimV1 {
        authority,
        issuance_id,
        claim_id,
        finalized: false,
    })
}

fn bootstrap_with_fresh_draft_authority(
    request: GeneralSemProjectArchiveBootstrapRequestV1,
    active_project: Arc<Mutex<Project>>,
    authority: &DesktopGeneralSemFreshDraftAuthorityV1,
) -> GeneralSemProjectArchiveBootstrapOutcomeV1 {
    let claim = match claim_fresh_general_sem_draft(&request, &active_project, authority) {
        Ok(claim) => claim,
        Err(outcome) => return outcome,
    };
    let outcome = bootstrap_general_sem_project_archive(request, active_project);
    if matches!(
        outcome,
        GeneralSemProjectArchiveBootstrapOutcomeV1::Ok { .. }
    ) {
        claim.consume();
    } else {
        claim.restore();
    }
    outcome
}

fn bootstrap_general_sem_project_archive(
    request: GeneralSemProjectArchiveBootstrapRequestV1,
    active_project: Arc<Mutex<Project>>,
) -> GeneralSemProjectArchiveBootstrapOutcomeV1 {
    // The command wrapper evaluates Registry access before cloning or locking
    // project state and before any destination-path inspection.
    if request.destination_path.is_empty()
        || request.destination_path != request.destination_path.trim()
    {
        return blocked(
            "destination_invalid",
            "The destination path must be nonempty and cannot contain surrounding whitespace.",
            "Choose a new absolute .qpls destination path without surrounding whitespace.",
        );
    }
    if request.name.trim().is_empty() || request.name != request.name.trim() {
        return blocked(
            "name_invalid",
            "The project name must be nonempty and cannot contain surrounding whitespace.",
            "Enter a project name without surrounding whitespace.",
        );
    }

    let parse_uuid = |value: &str| Uuid::parse_str(value).ok().filter(|value| !value.is_nil());
    let Some(project_id) = parse_uuid(&request.project_id) else {
        return blocked(
            "project_id_invalid",
            "The new projectId must be a non-nil UUID.",
            "Generate a new UUID for the General SEM project and retry.",
        );
    };
    let Some(source_project_id) = parse_uuid(&request.source_project_id) else {
        return blocked(
            "source_project_id_invalid",
            "The sourceProjectId must be a non-nil UUID.",
            "Refresh the active project snapshot and retry.",
        );
    };
    if project_id == source_project_id {
        return blocked(
            "new_project_identity_required",
            "A General SEM schema-6 project must use a new project identity.",
            "Generate a new projectId; the active project is used only as the dataset source.",
        );
    }
    let Some(source_dataset_id) = parse_uuid(&request.source_dataset_id) else {
        return blocked(
            "source_dataset_id_invalid",
            "The sourceDatasetId must be a non-nil UUID.",
            "Select a resident dataset from the active project and retry.",
        );
    };
    if request.source_dataset_fingerprint.trim().is_empty()
        || request.source_dataset_fingerprint != request.source_dataset_fingerprint.trim()
    {
        return blocked(
            "source_dataset_fingerprint_invalid",
            "The source dataset fingerprint must be nonempty and exact.",
            "Refresh the active dataset binding and retry.",
        );
    }
    let created_at = match DateTime::parse_from_rfc3339(&request.created_at) {
        Ok(created_at) => created_at.with_timezone(&Utc),
        Err(_) => {
            return blocked(
                "created_at_invalid",
                "The createdAt value must be an RFC3339 timestamp.",
                "Provide createdAt in RFC3339 form, for example 2026-08-18T09:30:00Z.",
            );
        }
    };

    let dataset = {
        let project = match active_project.lock() {
            Ok(project) => project,
            Err(_) => {
                return blocked(
                    "source_project_unavailable",
                    "The active project authority is temporarily unavailable.",
                    "Retry after the active project finishes its current operation.",
                );
            }
        };
        if project.manifest.project_id != source_project_id {
            return blocked(
                "source_project_changed",
                "The active project identity changed before General SEM bootstrap began.",
                "Refresh the project snapshot and create the General SEM project again.",
            );
        }
        let Some(dataset) = project
            .datasets
            .iter()
            .find(|dataset| dataset.id == source_dataset_id)
        else {
            return blocked(
                "source_dataset_not_resident",
                "The selected dataset is not resident in the active project.",
                "Select a current resident dataset and retry.",
            );
        };
        if dataset.fingerprint.0 != request.source_dataset_fingerprint {
            return blocked(
                "source_dataset_changed",
                "The selected dataset fingerprint changed before General SEM bootstrap began.",
                "Refresh the dataset binding and rebuild the RecipeV4 request.",
            );
        }
        let lineage = match read_project_data_lineage_v1(&project.layouts) {
            Ok(lineage) => lineage,
            Err(_) => {
                return blocked(
                    "source_dataset_lineage_invalid",
                    "The active dataset lineage authority is invalid.",
                    "Repair or reimport the dataset before creating a General SEM project.",
                );
            }
        };
        if lineage.as_ref().is_some_and(|lineage| {
            lineage.records.iter().any(|record| {
                record.dataset_id == source_dataset_id.to_string()
                    && (record.operation != ProjectDatasetVersionOperationV1::Import
                        || record.parent_dataset_id.is_some()
                        || record.transformation.is_some())
            })
        }) {
            return blocked(
                "source_dataset_transformation_lineage_unsupported",
                "This exact General SEM execution cell does not accept a derived or transformed dataset authority.",
                "Import the original raw dataset as a new resident dataset and rebuild the General SEM project.",
            );
        }
        dataset.clone()
    };

    match create_populated_general_sem_project_archive_v6(
        Path::new(&request.destination_path),
        project_id,
        request.name,
        created_at,
        &dataset,
        request.model,
        request.recipe,
    ) {
        Ok(receipt) => GeneralSemProjectArchiveBootstrapOutcomeV1::Ok {
            value: GeneralSemProjectArchiveBootstrapResultV1 {
                schema_version: GENERAL_SEM_BOOTSTRAP_RESULT_SCHEMA_VERSION,
                receipt,
            },
        },
        Err(error) => publication_error(error),
    }
}

#[tauri::command]
pub(crate) async fn bootstrap_internal_general_sem_project_archive_v6(
    request: GeneralSemProjectArchiveBootstrapRequestV1,
    project: State<'_, DesktopProject>,
    fresh_draft_authority: State<'_, DesktopGeneralSemFreshDraftAuthorityV1>,
) -> Result<GeneralSemProjectArchiveBootstrapOutcomeV1, String> {
    // Denied callers cannot lock project state or trigger destination lookup.
    if let Some(blocked) = registry_access_block(&request) {
        return Ok(blocked);
    }
    let project = project.0.clone();
    let fresh_draft_authority = fresh_draft_authority.inner().clone();
    Ok(
        match tauri::async_runtime::spawn_blocking(move || {
            bootstrap_with_fresh_draft_authority(request, project, &fresh_draft_authority)
        })
        .await
        {
            Ok(outcome) => outcome,
            Err(_) => blocked(
                "worker_failed",
                "The General SEM project bootstrap worker stopped before returning an outcome.",
                "Keep the General SEM draft unchanged and retry with a new destination filename.",
            ),
        },
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::TimeZone;
    use qpls_core::{
        ANALYSIS_RECIPE_SCHEMA_VERSION, AnalysisMethod, AnalysisRecipe,
        AnalysisRecipeModelBindingV4, AnalysisSettings, Construct, GeneralSemBootstrapIntervalV1,
        GeneralSemConfigV1, GeneralSemInferenceTailV1, GeneralSemInferenceV1,
        LegacyBasicModelInterpretationV4, MeasurementMode, MethodConfig, ModelSpec,
        SemDataBindingV4, StructuralPath, confirm_legacy_recipe_estimand_v4,
        migrate_analysis_recipe_to_v4_pending,
    };
    use qpls_data::{Dataset, ImportOptions, import_delimited_bytes};
    use std::{collections::BTreeMap, fs};

    pub(crate) struct GeneralSemNativeFixtureV1 {
        pub(crate) project: Project,
        pub(crate) dataset: Dataset,
        pub(crate) model: SemModelV4,
        pub(crate) recipe: AnalysisRecipeV4,
    }

    pub(crate) fn general_sem_native_fixture_v1() -> GeneralSemNativeFixtureV1 {
        let source_model = ModelSpec {
            id: Uuid::from_u128(0x6e61_7469_7665_5f67_656e_7365_6d01),
            name: "Native General SEM fixture".into(),
            constructs: ["x", "m1", "m2", "y"]
                .into_iter()
                .map(|id| Construct {
                    id: id.into(),
                    name: id.to_uppercase(),
                    short_name: id.to_uppercase(),
                    mode: MeasurementMode::Reflective,
                    indicators: vec![format!("{id}1"), format!("{id}2")],
                })
                .collect(),
            paths: [
                ("x", "m1"),
                ("x", "m2"),
                ("x", "y"),
                ("m1", "m2"),
                ("m1", "y"),
                ("m2", "y"),
            ]
            .into_iter()
            .map(|(source, target)| StructuralPath {
                source: source.into(),
                target: target.into(),
            })
            .collect(),
            controls: Vec::new(),
            higher_order_constructs: Vec::new(),
            interactions: Vec::new(),
        };
        let dataset = import_delimited_bytes(
            b"x1,x2,m11,m12,m21,m22,y1,y2\n1,2,2,1,1,3,2,1\n2,1,3,2,2,2,3,2\n3,4,4,3,4,3,5,4\n4,3,5,5,3,5,6,5\n5,6,7,6,6,7,8,7\n6,5,6,7,7,6,9,8\n7,8,9,7,8,9,11,9\n8,7,8,9,9,8,10,11\n9,10,11,10,10,12,13,12\n10,9,12,11,12,10,14,13\n11,12,13,12,13,14,16,15\n12,11,14,13,14,13,17,16\n",
            "native-general-sem.csv",
            b',',
            &ImportOptions::default(),
        )
        .unwrap();
        let source_recipe = AnalysisRecipe {
            schema_version: ANALYSIS_RECIPE_SCHEMA_VERSION,
            id: Uuid::from_u128(0x6e61_7469_7665_5f67_656e_7365_6d02),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            dataset_fingerprint: dataset.fingerprint.0.clone(),
            model: source_model.clone(),
            settings: AnalysisSettings {
                method: AnalysisMethod::PlsPm,
                workers: 1,
                ..AnalysisSettings::default()
            },
            method_config: Some(MethodConfig::PlsAlgorithm),
            metadata: BTreeMap::new(),
        };
        let pending = migrate_analysis_recipe_to_v4_pending(&source_recipe).unwrap();
        let (mut recipe, mut model) = confirm_legacy_recipe_estimand_v4(
            &pending,
            &source_model,
            &[],
            LegacyBasicModelInterpretationV4::PlsComposite,
        )
        .unwrap();
        let SemDataBindingV4::Raw { dataset_id, .. } = &mut model.data_binding else {
            unreachable!()
        };
        *dataset_id = dataset.id.to_string();
        recipe.model_binding = AnalysisRecipeModelBindingV4::ProjectSemModelV4Reference {
            model_id: model.id.clone(),
            scientific_sha256: model.scientific_sha256().unwrap(),
        };
        recipe.general_sem_config = Some(GeneralSemConfigV1::default());
        recipe.metadata.insert(
            "execution_surface".into(),
            GENERAL_SEM_PLS_STANDARD_RECIPE_EXECUTION_SURFACE_V1.into(),
        );
        recipe
            .metadata
            .insert("general_sem_generation".into(), "general_sem_v1".into());

        let mut project = Project::new("Native General SEM source");
        project.datasets.push(dataset.clone());
        GeneralSemNativeFixtureV1 {
            project,
            dataset,
            model,
            recipe,
        }
    }

    fn request(
        destination: &Path,
        fixture: &GeneralSemNativeFixtureV1,
    ) -> GeneralSemProjectArchiveBootstrapRequestV1 {
        GeneralSemProjectArchiveBootstrapRequestV1 {
            surface: STANDARD_SURFACE.into(),
            experimental_labs_enabled: false,
            capability_cell: qpls_core::pls_general_recursive_effects_capability_cell_v1(),
            destination_path: destination.to_string_lossy().into_owned(),
            project_id: "60000002-0000-4000-8000-000000000001".into(),
            name: "Populated General SEM project".into(),
            created_at: "2026-08-18T09:30:00Z".into(),
            source_project_id: fixture.project.manifest.project_id.to_string(),
            source_dataset_id: fixture.dataset.id.to_string(),
            source_dataset_fingerprint: fixture.dataset.fingerprint.0.clone(),
            model: fixture.model.clone(),
            recipe: fixture.recipe.clone(),
        }
    }

    #[test]
    fn bootstrap_request_wire_is_strict_camel_case_and_denies_unknown_fields() {
        let fixture = general_sem_native_fixture_v1();
        let valid = serde_json::json!({
            "surface": "standard",
            "experimentalLabsEnabled": false,
            "capabilityCell": qpls_core::pls_general_recursive_effects_capability_cell_v1(),
            "destinationPath": r"D:\projects\general-sem.qpls",
            "projectId": "60000002-0000-4000-8000-000000000001",
            "name": "General SEM",
            "createdAt": "2026-08-18T09:30:00Z",
            "sourceProjectId": fixture.project.manifest.project_id,
            "sourceDatasetId": fixture.dataset.id,
            "sourceDatasetFingerprint": fixture.dataset.fingerprint.0,
            "model": fixture.model,
            "recipe": fixture.recipe,
        });
        serde_json::from_value::<GeneralSemProjectArchiveBootstrapRequestV1>(valid.clone())
            .unwrap();

        let mut unknown = valid.clone();
        unknown
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), serde_json::Value::Bool(true));
        assert!(
            serde_json::from_value::<GeneralSemProjectArchiveBootstrapRequestV1>(unknown).is_err()
        );

        let mut snake = valid;
        let body = snake.as_object_mut().unwrap();
        let enabled = body.remove("experimentalLabsEnabled").unwrap();
        body.insert("experimental_labs_enabled".into(), enabled);
        assert!(
            serde_json::from_value::<GeneralSemProjectArchiveBootstrapRequestV1>(snake).is_err()
        );

        assert_eq!(
            serde_json::from_value::<GeneralSemNewProjectModeV1>(serde_json::json!(
                "general_sem_v1"
            ))
            .unwrap(),
            GeneralSemNewProjectModeV1::GeneralSemV1
        );
        assert!(
            serde_json::from_value::<GeneralSemNewProjectModeV1>(serde_json::json!("generalSemV1"))
                .is_err()
        );
    }

    #[test]
    fn registry_denial_precedes_destination_or_project_action() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = general_sem_native_fixture_v1();
        for (index, (surface, enabled)) in [
            (INTERNAL_LABS_SURFACE, true),
            (INTERNAL_LABS_SURFACE, false),
        ]
        .into_iter()
        .enumerate()
        {
            let destination = directory.path().join(format!("blocked-{index}.qpls"));
            let mut denied = request(&destination, &fixture);
            denied.surface = surface.into();
            denied.experimental_labs_enabled = enabled;
            denied.destination_path = format!(" {} ", destination.to_string_lossy());
            let outcome = registry_access_block(&denied).expect("registry gate must deny first");
            assert!(matches!(
                &outcome,
                GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                    if diagnostic.code
                        == "schema6_general_sem_bootstrap.standard_surface_required"
            ));
            let wire = serde_json::to_value(&outcome).unwrap();
            assert_eq!(wire["status"], "blocked");
            assert!(wire["diagnostic"].get("correctiveAction").is_some());
            assert!(wire["diagnostic"].get("corrective_action").is_none());
            assert!(!destination.exists());
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn bootstrap_requires_supplemental_cell_for_bootstrap_inference_before_path_access() {
        let directory = tempfile::tempdir().unwrap();
        let mut fixture = general_sem_native_fixture_v1();
        fixture
            .recipe
            .general_sem_config
            .as_mut()
            .unwrap()
            .inference = GeneralSemInferenceV1::CaseBootstrap {
            resamples: 500,
            seed: 7,
            confidence_level: 0.95,
            interval: GeneralSemBootstrapIntervalV1::Percentile,
            tail: GeneralSemInferenceTailV1::TwoSided,
        };
        let mut wrong = request(&directory.path().join("must-not-open.qpls"), &fixture);
        wrong.capability_cell = qpls_core::pls_general_recursive_effects_capability_cell_v1();
        wrong.destination_path = " relative-and-unreachable ".into();
        let outcome = registry_access_block(&wrong).expect("point cell must not own bootstrap");
        assert!(matches!(
            outcome,
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code
                    == "schema6_general_sem_bootstrap.capability_cell_mismatch"
        ));
        let mut exact = wrong;
        exact.capability_cell = qpls_core::pls_general_bootstrap_capability_cell_v1();
        assert!(
            registry_access_block(&exact).is_none(),
            "the supplemental mediation bootstrap cell owns the operation regardless of canonical decision order"
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn standard_or_legacy_active_project_has_no_fresh_draft_authority() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();

        let outcome = bootstrap_with_fresh_draft_authority(
            request(&directory.path().join("blocked.qpls"), &fixture),
            active_project.clone(),
            &authority,
        );
        assert!(matches!(
            outcome,
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code
                    == "schema6_general_sem_bootstrap.fresh_draft_authority_required"
        ));

        authority.issue_for_test(fixture.project.manifest.project_id);
        authority
            .replace_and_clear(&active_project, fixture.project.clone())
            .unwrap();
        let outcome = bootstrap_with_fresh_draft_authority(
            request(&directory.path().join("cleared.qpls"), &fixture),
            active_project,
            &authority,
        );
        assert!(matches!(
            outcome,
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code
                    == "schema6_general_sem_bootstrap.fresh_draft_authority_required"
        ));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn only_general_sem_new_project_mode_issues_a_project_bound_authorization() {
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        let active_project = Arc::new(Mutex::new(Project::new("Initial")));

        let ordinary = Project::new("Ordinary");
        authority
            .replace_from_new_project(&active_project, ordinary, None)
            .unwrap();
        assert!(authority.lock_recovering().authorization.is_none());

        let general_sem = Project::new("General SEM");
        let general_sem_id = general_sem.manifest.project_id;
        authority
            .replace_from_new_project(
                &active_project,
                general_sem,
                Some(GeneralSemNewProjectModeV1::GeneralSemV1),
            )
            .unwrap();
        assert_eq!(
            authority
                .lock_recovering()
                .authorization
                .as_ref()
                .map(|authorization| authorization.project_id),
            Some(general_sem_id)
        );

        authority
            .replace_from_new_project(
                &active_project,
                Project::new("Standard"),
                Some(GeneralSemNewProjectModeV1::Standard),
            )
            .unwrap();
        assert!(authority.lock_recovering().authorization.is_none());
    }

    #[test]
    fn unmarked_save_and_autosave_are_denied_without_clearing_the_fresh_draft() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        authority.issue_for_test(fixture.project.manifest.project_id);

        let save_error = crate::save_active_project_with_fresh_draft_authority(
            directory
                .path()
                .join("denied-save.qpls")
                .to_string_lossy()
                .into_owned(),
            serde_json::Value::Null,
            None,
            None,
            &active_project,
            &authority,
        )
        .unwrap_err();
        let autosave_error = crate::autosave_active_project_with_fresh_draft_authority(
            directory
                .path()
                .join("denied-autosave.qpls")
                .to_string_lossy()
                .into_owned(),
            serde_json::Value::Null,
            None,
            None,
            &active_project,
            &authority,
        )
        .unwrap_err();

        assert_eq!(save_error, GENERAL_SEM_FRESH_DRAFT_UNMARKED_SAVE_BLOCKER);
        assert_eq!(
            autosave_error,
            GENERAL_SEM_FRESH_DRAFT_UNMARKED_SAVE_BLOCKER
        );
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        let authorization = authority.lock_recovering().authorization.clone().unwrap();
        assert_eq!(
            authorization.project_id,
            fixture.project.manifest.project_id
        );
        assert!(authorization.claim_id.is_none());

        let claim = claim_fresh_general_sem_draft(
            &request(&directory.path().join("still-claimable.qpls"), &fixture),
            &active_project,
            &authority,
        )
        .expect("denied unmarked persistence must leave the draft claimable");
        claim.restore();
    }

    #[test]
    fn mismatched_fresh_draft_authority_blocks_unmarked_save_without_clearing_it() {
        let directory = tempfile::tempdir().unwrap();
        let active_project = Arc::new(Mutex::new(Project::new("Active project")));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        let authorized_project_id = Uuid::from_u128(0x6000_00aa);
        authority.issue_for_test(authorized_project_id);

        let error = crate::save_active_project_with_fresh_draft_authority(
            directory
                .path()
                .join("mismatched-authority.qpls")
                .to_string_lossy()
                .into_owned(),
            serde_json::Value::Null,
            None,
            None,
            &active_project,
            &authority,
        )
        .unwrap_err();

        assert_eq!(error, GENERAL_SEM_FRESH_DRAFT_AUTHORITY_MISMATCH_BLOCKER);
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
        let authorization = authority.lock_recovering().authorization.clone().unwrap();
        assert_eq!(authorization.project_id, authorized_project_id);
        assert!(authorization.claim_id.is_none());
    }

    #[test]
    fn fresh_draft_authority_is_bound_to_the_exact_active_project_identity() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        authority.issue_for_test(fixture.project.manifest.project_id);

        let mut mismatched = request(&directory.path().join("mismatch.qpls"), &fixture);
        mismatched.source_project_id = Uuid::from_u128(0x107).to_string();
        assert!(matches!(
            bootstrap_with_fresh_draft_authority(mismatched, active_project, &authority),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code
                    == "schema6_general_sem_bootstrap.fresh_draft_project_mismatch"
        ));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn failed_validation_restores_the_fresh_draft_authorization_for_retry() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        authority.issue_for_test(fixture.project.manifest.project_id);

        let mut invalid = request(&directory.path().join("invalid.qpls"), &fixture);
        invalid.destination_path = format!(" {} ", invalid.destination_path);
        assert!(matches!(
            bootstrap_with_fresh_draft_authority(invalid, active_project.clone(), &authority),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_bootstrap.destination_invalid"
        ));

        let mut retry = request(&directory.path().join("retry.qpls"), &fixture);
        retry.source_dataset_fingerprint = "stale-fingerprint".into();
        assert!(matches!(
            bootstrap_with_fresh_draft_authority(retry, active_project, &authority),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_bootstrap.source_dataset_changed"
        ));
    }

    #[test]
    fn an_in_flight_claim_cannot_be_claimed_again() {
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        authority.issue_for_test(fixture.project.manifest.project_id);
        let _held = authority.0.lock().unwrap();

        assert!(matches!(
            claim_fresh_general_sem_draft(
                &request(Path::new(r"D:\projects\blocked.qpls"), &fixture),
                &active_project,
                &authority,
            ),
            Err(GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic })
                if diagnostic.code
                    == "schema6_general_sem_bootstrap.fresh_draft_claim_in_progress"
        ));
    }

    #[test]
    fn source_project_dataset_and_fingerprint_mismatches_fail_before_publication() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));

        let project_destination = directory.path().join("project-mismatch.qpls");
        let mut project_mismatch = request(&project_destination, &fixture);
        project_mismatch.source_project_id = Uuid::from_u128(0x101).to_string();
        assert!(matches!(
            bootstrap_general_sem_project_archive(project_mismatch, active_project.clone()),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_bootstrap.source_project_changed"
        ));

        let dataset_destination = directory.path().join("dataset-mismatch.qpls");
        let mut dataset_mismatch = request(&dataset_destination, &fixture);
        dataset_mismatch.source_dataset_id = Uuid::from_u128(0x102).to_string();
        assert!(matches!(
            bootstrap_general_sem_project_archive(dataset_mismatch, active_project.clone()),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_bootstrap.source_dataset_not_resident"
        ));

        let fingerprint_destination = directory.path().join("fingerprint-mismatch.qpls");
        let mut fingerprint_mismatch = request(&fingerprint_destination, &fixture);
        fingerprint_mismatch.source_dataset_fingerprint = "stale-fingerprint".into();
        assert!(matches!(
            bootstrap_general_sem_project_archive(fingerprint_mismatch, active_project),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_bootstrap.source_dataset_changed"
        ));

        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn valid_bootstrap_publishes_exact_populated_authority_and_receipt() {
        use qpls_project::{ProjectModelPayloadV6, load_project_archive_v6};

        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("populated-general-sem.qpls");
        let fixture = general_sem_native_fixture_v1();
        let expected_dataset_id = fixture.dataset.id;
        let expected_dataset_fingerprint = fixture.dataset.fingerprint.0.clone();
        let expected_model_id = fixture.model.id.clone();
        let expected_model_sha256 = fixture.model.scientific_sha256().unwrap();
        let expected_recipe_id = fixture.recipe.id;
        let expected_recipe_sha256 = qpls_core::sha256_serialized(&fixture.recipe);
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        authority.issue_for_test(fixture.project.manifest.project_id);

        assert_eq!(
            crate::save_active_project_with_fresh_draft_authority(
                directory
                    .path()
                    .join("denied-save.qpls")
                    .to_string_lossy()
                    .into_owned(),
                serde_json::Value::Null,
                None,
                None,
                &active_project,
                &authority,
            )
            .unwrap_err(),
            GENERAL_SEM_FRESH_DRAFT_UNMARKED_SAVE_BLOCKER
        );
        assert_eq!(
            crate::autosave_active_project_with_fresh_draft_authority(
                directory
                    .path()
                    .join("denied-autosave.qpls")
                    .to_string_lossy()
                    .into_owned(),
                serde_json::Value::Null,
                None,
                None,
                &active_project,
                &authority,
            )
            .unwrap_err(),
            GENERAL_SEM_FRESH_DRAFT_UNMARKED_SAVE_BLOCKER
        );
        assert!(!directory.path().join("denied-save.qpls").exists());
        assert!(!directory.path().join("denied-autosave.qpls").exists());

        let GeneralSemProjectArchiveBootstrapOutcomeV1::Ok { value } =
            bootstrap_with_fresh_draft_authority(
                request(&destination, &fixture),
                active_project.clone(),
                &authority,
            )
        else {
            panic!("valid populated General SEM bootstrap was blocked")
        };

        assert_eq!(
            value.schema_version,
            GENERAL_SEM_BOOTSTRAP_RESULT_SCHEMA_VERSION
        );
        assert!(value.receipt.strict_reopen_validated);
        assert_eq!(value.receipt.resident_dataset_id, expected_dataset_id);
        assert_eq!(
            value.receipt.resident_dataset_fingerprint,
            expected_dataset_fingerprint
        );
        assert_eq!(value.receipt.resident_model_id, expected_model_id);
        assert_eq!(
            value.receipt.resident_model_scientific_sha256,
            expected_model_sha256
        );
        assert_eq!(value.receipt.resident_recipe_id, expected_recipe_id);
        assert_eq!(
            value.receipt.resident_recipe_document_sha256,
            expected_recipe_sha256
        );
        assert_eq!(
            value.receipt.destination_archive_bytes,
            fs::metadata(&destination).unwrap().len()
        );
        assert_eq!(value.receipt.destination_archive_sha256.len(), 64);
        assert!(
            value
                .receipt
                .destination_archive_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );

        let reopened = load_project_archive_v6(&destination).unwrap();
        assert!(reopened.document.supports_general_sem_v1());
        assert_eq!(reopened.document.datasets.len(), 1);
        assert_eq!(reopened.datasets.len(), 1);
        assert_eq!(reopened.document.models.len(), 1);
        assert_eq!(reopened.document.recipes.len(), 1);
        assert!(reopened.document.historical_recipes.is_empty());
        assert!(reopened.document.historical_results.is_empty());
        assert_eq!(reopened.datasets[0].id, expected_dataset_id);
        assert!(matches!(
            &reopened.document.models[0].payload,
            ProjectModelPayloadV6::SemModelV4 { .. }
        ));

        let second_destination = directory.path().join("second-general-sem.qpls");
        assert!(matches!(
            bootstrap_with_fresh_draft_authority(
                request(&second_destination, &fixture),
                active_project,
                &authority,
            ),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code
                    == "schema6_general_sem_bootstrap.fresh_draft_authority_required"
        ));
        assert!(!second_destination.exists());
    }

    #[cfg(windows)]
    #[test]
    fn publication_failure_restores_authorization_and_a_new_destination_can_succeed() {
        let directory = tempfile::tempdir().unwrap();
        let existing_destination = directory.path().join("already-exists.qpls");
        fs::write(&existing_destination, b"existing").unwrap();
        let fixture = general_sem_native_fixture_v1();
        let active_project = Arc::new(Mutex::new(fixture.project.clone()));
        let authority = DesktopGeneralSemFreshDraftAuthorityV1::default();
        authority.issue_for_test(fixture.project.manifest.project_id);

        assert!(matches!(
            bootstrap_with_fresh_draft_authority(
                request(&existing_destination, &fixture),
                active_project.clone(),
                &authority,
            ),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Blocked { diagnostic }
                if diagnostic.code == "schema6_general_sem_bootstrap.destination_exists"
        ));
        assert_eq!(fs::read(&existing_destination).unwrap(), b"existing");

        let retry_destination = directory.path().join("retry-general-sem.qpls");
        assert!(matches!(
            bootstrap_with_fresh_draft_authority(
                request(&retry_destination, &fixture),
                active_project,
                &authority,
            ),
            GeneralSemProjectArchiveBootstrapOutcomeV1::Ok { .. }
        ));
        assert!(retry_destination.is_file());
    }
}
