import { invoke } from "@tauri-apps/api/core";
import {
  parseInternalProjectArchiveV6DatasetRowsRequestV1,
  parseInternalProjectArchiveV6DatasetRowsOutcomeV1,
  type InternalProjectArchiveV6DatasetRowsRequestV1,
} from "../domain/internalProjectArchiveV6DatasetRows";
import {
  resolveInternalProjectArchiveV6AccessV1,
  type InternalProjectArchiveV6AccessDependenciesV1,
} from "./internalProjectArchiveV6AccessService";

const READ_SCHEMA6_DATASET_ROWS_COMMAND = "read_internal_project_archive_v6_dataset_rows";

/** Reads one bounded page from the exact strictly validated General SEM archive. */
export async function readInternalProjectArchiveV6DatasetRows(
  request: InternalProjectArchiveV6DatasetRowsRequestV1,
  dependencies: InternalProjectArchiveV6AccessDependenciesV1 = {},
) {
  const requested = parseInternalProjectArchiveV6DatasetRowsRequestV1(request);
  const authorizedRequest = parseInternalProjectArchiveV6DatasetRowsRequestV1({
    ...requested,
    ...(await resolveInternalProjectArchiveV6AccessV1(dependencies)),
  });
  const response = await invoke<unknown>(READ_SCHEMA6_DATASET_ROWS_COMMAND, {
    request: authorizedRequest,
  });
  return parseInternalProjectArchiveV6DatasetRowsOutcomeV1(
    response,
    authorizedRequest,
  );
}
