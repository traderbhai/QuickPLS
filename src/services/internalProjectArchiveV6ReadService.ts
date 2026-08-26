import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  parseInternalProjectArchiveV6ReadOutcomeV1,
  parseInternalProjectArchiveV6ReadRequestV1,
} from "../domain/internalProjectArchiveV6Read";
import {
  resolveInternalProjectArchiveV6AccessV1,
  type InternalProjectArchiveV6AccessDependenciesV1,
} from "./internalProjectArchiveV6AccessService";

const INSPECT_SCHEMA6_ZIP_COMMAND = "inspect_internal_project_archive_v6_zip";

/**
 * Authority-selected inspection of an existing schema-6 ZIP archive.
 *
 * This service does not install the snapshot as the active project and exposes
 * no save, autosave, recovery, or upgrade operation.
 */
export async function inspectInternalProjectArchiveV6At(
  archivePath: string,
  dependencies: InternalProjectArchiveV6AccessDependenciesV1 = {},
) {
  const request = parseInternalProjectArchiveV6ReadRequestV1({
    ...(await resolveInternalProjectArchiveV6AccessV1(dependencies)),
    archivePath,
  });
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
export async function openInternalProjectArchiveV6(
  dependencies: InternalProjectArchiveV6AccessDependenciesV1 = {},
) {
  const archivePath = await open({
    multiple: false,
    filters: [{ name: "QuickPLS schema-6 ZIP project", extensions: ["qpls"] }],
  });
  if (!archivePath) return null;
  return inspectInternalProjectArchiveV6At(archivePath, dependencies);
}
