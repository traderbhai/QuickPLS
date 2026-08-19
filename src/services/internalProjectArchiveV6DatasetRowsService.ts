import { invoke } from "@tauri-apps/api/core";
import {
  parseInternalProjectArchiveV6DatasetRowsOutcomeV1,
  type InternalProjectArchiveV6DatasetRowsRequestV1,
} from "../domain/internalProjectArchiveV6DatasetRows";

const READ_SCHEMA6_DATASET_ROWS_COMMAND = "read_internal_project_archive_v6_dataset_rows";

/** Reads one bounded page from the exact strictly validated General SEM archive. */
export async function readInternalProjectArchiveV6DatasetRows(
  request: InternalProjectArchiveV6DatasetRowsRequestV1,
) {
  const response = await invoke<unknown>(READ_SCHEMA6_DATASET_ROWS_COMMAND, { request });
  return parseInternalProjectArchiveV6DatasetRowsOutcomeV1(response, request);
}
