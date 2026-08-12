import { describe, expect, it } from "vitest";
import type { Dataset, DatasetVersionRecord } from "../types";
import { nativeDatasetOperationLabel, nativeDatasetVersionItems } from "./nativeDatasetVersions";

const dataset = (id: string): Dataset => ({ id, name: id, columns: [], rows: [], missing: 0 });
const record = (datasetId: string, parentDatasetId: string | null, operation: DatasetVersionRecord["operation"]): DatasetVersionRecord => ({
  datasetId,
  parentDatasetId,
  operation,
  createdAt: null,
  summary: datasetId,
  sourceColumn: null,
  targetColumn: null,
});

describe("native dataset version navigation", () => {
  it("orders authoritative records and derives bounded lineage depth", () => {
    const items = nativeDatasetVersionItems(
      [dataset("root"), dataset("metadata"), dataset("recode")],
      [record("root", null, "import"), record("metadata", "root", "metadata"), record("recode", "metadata", "recode")],
    );

    expect(items.map(({ dataset: item, versionNumber, depth }) => [item.id, versionNumber, depth])).toEqual([
      ["root", 1, 0],
      ["metadata", 2, 1],
      ["recode", 3, 2],
    ]);
  });

  it("keeps archive snapshots without lineage truthful instead of inventing metadata", () => {
    const items = nativeDatasetVersionItems([dataset("known"), dataset("legacy")], [record("known", null, "import")]);
    expect(items[1]).toMatchObject({ record: null, versionNumber: 2, depth: 0 });
  });

  it("ignores orphan records and labels only known operations", () => {
    expect(nativeDatasetVersionItems([dataset("root")], [record("missing", null, "recode")])).toEqual([
      expect.objectContaining({ dataset: expect.objectContaining({ id: "root" }), record: null }),
    ]);
    const operations: Array<DatasetVersionRecord["operation"] | null> = ["import", "metadata", "recode", null];
    expect(operations.map(nativeDatasetOperationLabel)).toEqual([
      "Imported",
      "Metadata",
      "Recoded",
      "Dataset",
    ]);
  });
});
