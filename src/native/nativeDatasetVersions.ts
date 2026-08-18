import type { Dataset, DatasetVersionOperation, DatasetVersionRecord } from "../types";

export interface NativeDatasetVersionItem {
  dataset: Dataset;
  record: DatasetVersionRecord | null;
  versionNumber: number;
  depth: number;
}

const MAX_LINEAGE_DEPTH = 32;

function lineageDepth(record: DatasetVersionRecord, recordsById: ReadonlyMap<string, DatasetVersionRecord>): number {
  let depth = 0;
  let parentId = record.parentDatasetId;
  const visited = new Set([record.datasetId]);
  while (parentId && depth < MAX_LINEAGE_DEPTH && !visited.has(parentId)) {
    visited.add(parentId);
    const parent = recordsById.get(parentId);
    if (!parent) break;
    depth += 1;
    parentId = parent.parentDatasetId;
  }
  return depth;
}

/**
 * Joins backend-owned dataset snapshots to backend-owned lineage records.
 * Missing records remain explicitly unknown; the frontend never fabricates
 * operations or timestamps for snapshots returned by older archives.
 */
export function nativeDatasetVersionItems(
  datasets: readonly Dataset[],
  records: readonly DatasetVersionRecord[],
): NativeDatasetVersionItem[] {
  const datasetsById = new Map(datasets.map((dataset) => [dataset.id, dataset]));
  const recordsById = new Map(records.map((record) => [record.datasetId, record]));
  const orderedIds = [
    ...records.map((record) => record.datasetId).filter((id) => datasetsById.has(id)),
    ...datasets.map((dataset) => dataset.id).filter((id) => !recordsById.has(id)),
  ];
  return [...new Set(orderedIds)].map((id, index) => {
    const record = recordsById.get(id) ?? null;
    return {
      dataset: datasetsById.get(id)!,
      record,
      versionNumber: index + 1,
      depth: record ? lineageDepth(record, recordsById) : 0,
    };
  });
}

export function nativeDatasetOperationLabel(operation: DatasetVersionOperation | null): string {
  if (operation === "import") return "Imported";
  if (operation === "metadata") return "Metadata";
  if (operation === "recode") return "Recoded";
  if (operation === "transform") return "Derived";
  return "Dataset";
}
