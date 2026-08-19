import {
  buildInternalProjectArchiveV6DatasetRowsRequestV1,
  type InternalProjectArchiveV6DatasetRowsOutcomeV1,
  type InternalProjectArchiveV6DatasetRowsRequestV1,
} from "../domain/internalProjectArchiveV6DatasetRows";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import type { InternalProjectArchiveV6ReadOnlySession } from "../internalProjectArchiveV6SessionStore";
import type { Dataset, DatasetRowsPage } from "../types";

export type StrictGeneralSemDatasetPageReaderV1 = (
  request: InternalProjectArchiveV6DatasetRowsRequestV1,
) => Promise<InternalProjectArchiveV6DatasetRowsOutcomeV1>;

export type LegacyDatasetPageReaderV1 = (
  datasetId: string,
  offset: number,
  limit: number,
) => Promise<DatasetRowsPage>;

export interface NativeDatasetRowPagingContextV1 {
  dataset: Dataset;
  datasetDescriptorOnly: boolean;
  session: InternalProjectArchiveV6ReadOnlySession | null;
  offset: number;
  limit: number;
}

/**
 * Routes descriptor-only General SEM pages through the strict archive reader.
 * No fallback to the legacy resident project is permitted once schema-6 is bound.
 */
export async function readNativeDatasetPageV1(
  context: NativeDatasetRowPagingContextV1,
  strictReader: StrictGeneralSemDatasetPageReaderV1,
  legacyReader: LegacyDatasetPageReaderV1,
): Promise<DatasetRowsPage> {
  if (!context.datasetDescriptorOnly) {
    return legacyReader(context.dataset.id, context.offset, context.limit);
  }

  const session = context.session;
  if (!session?.standardActivation || !supportsGeneralSemV1(session.project)) {
    throw new Error("The descriptor-only dataset is not bound to a strict General SEM archive session.");
  }
  const request = buildInternalProjectArchiveV6DatasetRowsRequestV1(
    session.snapshot,
    context.dataset.id,
    context.offset,
    context.limit,
  );
  if (request.datasetFingerprint !== context.dataset.fingerprint
    || request.projectId !== session.project.project_id
    || session.standardActivation.sourceArchiveSha256 !== request.expectedArchiveSha256) {
    throw new Error("The visible dataset or active session no longer matches its strict archive receipt.");
  }
  const outcome = await strictReader(request);
  if (outcome.status === "blocked") {
    throw new Error(`${outcome.diagnostic.message} ${outcome.diagnostic.correctiveAction}`);
  }
  if (outcome.value.columns.length !== context.dataset.columns.length
    || outcome.value.columns.some((column, index) => column !== context.dataset.columns[index])) {
    throw new Error("The strict archive page columns differ from the visible dataset descriptor.");
  }
  return {
    datasetId: outcome.value.datasetId,
    offset: outcome.value.offset,
    limit: outcome.value.limit,
    rowCount: outcome.value.rowCount,
    rows: outcome.value.rows,
  };
}
