import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_READ_SURFACE,
  parseInternalProjectArchiveV6ReadOutcomeV1,
  type InternalProjectArchiveV6ReadRequestV1,
} from "../domain/internalProjectArchiveV6Read";

const INSPECT_SCHEMA6_ZIP_COMMAND = "inspect_internal_project_archive_v6_zip";

/**
 * Internal/Labs-only inspection of an existing schema-6 ZIP archive.
 *
 * This service does not install the snapshot as the active project and exposes
 * no save, autosave, recovery, or upgrade operation.
 */
export async function inspectInternalProjectArchiveV6At(archivePath: string) {
  const request: InternalProjectArchiveV6ReadRequestV1 = {
    surface: INTERNAL_PROJECT_ARCHIVE_V6_READ_SURFACE,
    experimentalLabsEnabled: true,
    archivePath,
  };
  const response = await invoke<unknown>(INSPECT_SCHEMA6_ZIP_COMMAND, { request });
  return parseInternalProjectArchiveV6ReadOutcomeV1(response);
}

/** Selects one project path so the controller can choose its versioned loader. */
export async function selectQuickPlsProjectArchivePath() {
  const archivePath = await open({
    multiple: false,
    filters: [{ name: "QuickPLS project", extensions: ["qpls"] }],
  });
  return typeof archivePath === "string" ? archivePath : null;
}

/** Selects a local `.qpls` file and returns an ephemeral read-only snapshot. */
export async function openInternalProjectArchiveV6() {
  const archivePath = await open({
    multiple: false,
    filters: [{ name: "QuickPLS schema-6 ZIP project", extensions: ["qpls"] }],
  });
  if (!archivePath) return null;
  return inspectInternalProjectArchiveV6At(archivePath);
}
