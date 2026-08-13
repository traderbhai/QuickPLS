import { beforeEach, describe, expect, it } from "vitest";
import { useWorkspace } from "../store";
import type { Dataset, DatasetVersionRecord } from "../types";

const makeDataset = (id: string, columns = ["group"]): Dataset => ({
  id,
  name: id,
  columns,
  rows: [],
  rowCount: 20,
  missing: 0,
  kind: "raw",
});

const importVersion: DatasetVersionRecord = {
  datasetId: "dataset-v1",
  parentDatasetId: null,
  operation: "import",
  createdAt: null,
  summary: "Imported data.csv",
  sourceColumn: null,
  targetColumn: null,
};

describe("native dataset catalog state", () => {
  beforeEach(() => useWorkspace.getState().resetProject());

  it("loads a complete backend catalog separately from the active dataset", () => {
    const first = makeDataset("dataset-v1");
    const second = makeDataset("dataset-v2", ["group", "group_binary"]);
    useWorkspace.getState().loadProject({ nodes: [], edges: [], dataset: second, datasets: [first, second], datasetVersions: [importVersion] });

    const state = useWorkspace.getState();
    expect(state.dataset).toBe(second);
    expect(state.datasetCatalog).toEqual([first, second]);
    expect(state.datasetVersions).toEqual([importVersion]);
  });

  it("commits an authoritative recode mutation without changing the source snapshot rows", () => {
    const source = makeDataset("dataset-v1");
    const derived = makeDataset("dataset-v2", ["group", "group_binary"]);
    const sourceRows = source.rows;
    useWorkspace.getState().setDatasetCatalog([source], [importVersion]);
    useWorkspace.getState().setDataset(source);
    useWorkspace.getState().commitDatasetVersion({
      dataset: derived,
      version: {
        datasetId: "dataset-v2",
        parentDatasetId: "dataset-v1",
        operation: "recode",
        createdAt: null,
        summary: "Recoded group into group_binary",
        sourceColumn: "group",
        targetColumn: "group_binary",
      },
    });

    const state = useWorkspace.getState();
    expect(state.dataset).toBe(derived);
    expect(state.datasetCatalog).toEqual([source, derived]);
    expect(state.datasetVersions.map((version) => version.datasetId)).toEqual(["dataset-v1", "dataset-v2"]);
    expect(source.rows).toBe(sourceRows);
    expect(source.rows).toEqual([]);
  });

  it("upserts an activated backend snapshot without discarding lineage", () => {
    const source = makeDataset("dataset-v1");
    useWorkspace.getState().setDatasetCatalog([source], [importVersion]);
    useWorkspace.getState().setDataset({ ...source, name: "Activated source" });

    const state = useWorkspace.getState();
    expect(state.datasetCatalog).toEqual([{ ...source, name: "Activated source" }]);
    expect(state.datasetVersions).toEqual([importVersion]);
  });
});
