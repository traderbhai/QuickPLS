import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_SAVE_COPY_SURFACE,
  parseInternalProjectArchiveV6SaveCopyOutcomeV1,
  parseInternalProjectArchiveV6SaveCopyRequestV1,
} from "../domain/internalProjectArchiveV6SaveCopy";
import type { InternalProjectArchiveV6Wire } from "../domain/internalProjectArchiveV6Wire";

const SAVE_SCHEMA6_COPY_COMMAND = "save_internal_project_archive_v6_copy";

function suggestedCopyPath(sourceArchivePath: string): string {
  return /\.qpls$/i.test(sourceArchivePath)
    ? sourceArchivePath.replace(/\.qpls$/i, "-model-copy.qpls")
    : `${sourceArchivePath}-model-copy.qpls`;
}

export async function saveInternalProjectArchiveV6CopyAt(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  project: InternalProjectArchiveV6Wire,
  destinationArchivePath: string,
) {
  const request = parseInternalProjectArchiveV6SaveCopyRequestV1({
    surface: INTERNAL_PROJECT_ARCHIVE_V6_SAVE_COPY_SURFACE,
    experimentalLabsEnabled: true,
    sourceArchivePath: snapshot.archivePath,
    expectedSourceArchiveSha256: snapshot.archiveSha256,
    destinationArchivePath,
    project,
  });
  const response = await invoke<unknown>(SAVE_SCHEMA6_COPY_COMMAND, { request });
  return parseInternalProjectArchiveV6SaveCopyOutcomeV1(response, request);
}

/** Chooses a new `.qpls` destination. Cancellation performs no native call. */
export async function saveInternalProjectArchiveV6Copy(
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  project: InternalProjectArchiveV6Wire,
) {
  const destinationArchivePath = await save({
    defaultPath: suggestedCopyPath(snapshot.archivePath),
    filters: [{ name: "QuickPLS schema-6 ZIP project", extensions: ["qpls"] }],
  });
  if (!destinationArchivePath) return null;
  return saveInternalProjectArchiveV6CopyAt(snapshot, project, destinationArchivePath);
}
