import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  activateNativeDataset,
  autosaveNativeProject,
  cancelNativeAnalysisJob,
  cancelNativePlsJob,
  createNativeProject,
  dismissNativeAnalysisJob,
  dismissNativePlsJob,
  exportNativeTextFile,
  exportNativeXlsxTables,
  getNativeAnalysisJob,
  getNativeAnalysisJobResult,
  getNativeDatasetRows,
  getNativePlsJob,
  getNativePlsJobResult,
  mutateNativeProjectExplorer,
  profileNativeDatasetGroups,
  recodeNativeDatasetColumn,
  saveNativeProject,
  startNativeAnalysisJob,
  startNativePlsJob,
} from "./projectService";
import type { NativeCanonicalModelSpec, NativeModelPresentation, RecodeColumnSpec } from "../types";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), save: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), save: mocks.save }));

describe("native dataset row paging service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("requests an id-scoped offset and limit without loading the entire dataset", async () => {
    const response = {
      datasetId: "dataset-1",
      offset: 250,
      limit: 50,
      rowCount: 10_000,
      rows: [{ score: "7" }],
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(getNativeDatasetRows("dataset-1", 250, 50)).resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith("dataset_rows", {
      datasetId: "dataset-1",
      offset: 250,
      limit: 50,
    });
  });

  it("profiles groups against the full native dataset and exact model columns", async () => {
    const response = {
      datasetId: "dataset-1",
      columnName: "region",
      rowCount: 240,
      missingCount: 2,
      unsupportedCount: 0,
      truncated: false,
      groups: [
        { value: "north", label: "North", observations: 120, completeCases: 116 },
        { value: "south", label: "South", observations: 118, completeCases: 114 },
      ],
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(profileNativeDatasetGroups("dataset-1", "region", ["x1", "x2", "y1", "y2"]))
      .resolves.toEqual(response);
    expect(mocks.invoke).toHaveBeenCalledWith("profile_dataset_groups", {
      datasetId: "dataset-1",
      columnName: "region",
      analysisColumns: ["x1", "x2", "y1", "y2"],
    });
  });
});

describe("native generic analysis job service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("uses method-neutral desktop commands without changing payload shapes", async () => {
    const recipe = { method: "pls_sem", settings: { seed: 20260812 } };
    mocks.invoke.mockResolvedValue({ id: "job-1", state: "queued" });

    await startNativeAnalysisJob(recipe);
    await getNativeAnalysisJob("job-1");
    await cancelNativeAnalysisJob("job-1");
    await dismissNativeAnalysisJob("job-1");
    await getNativeAnalysisJobResult("job-1");

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_analysis_job", { recipe });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "analysis_job_status", { jobId: "job-1" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "cancel_analysis_job", { jobId: "job-1" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "dismiss_analysis_job", { jobId: "job-1" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(5, "analysis_job_result", { jobId: "job-1" });
  });

  it("keeps legacy PLS service names as delegates to the generic command contract", async () => {
    const recipe = { method: "ols_regression" };
    mocks.invoke.mockResolvedValue({ id: "job-2", state: "running" });

    await startNativePlsJob(recipe);
    await getNativePlsJob("job-2");
    await cancelNativePlsJob("job-2");
    await dismissNativePlsJob("job-2");
    await getNativePlsJobResult("job-2");

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "start_analysis_job", { recipe });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "analysis_job_status", { jobId: "job-2" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "cancel_analysis_job", { jobId: "job-2" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(4, "dismiss_analysis_job", { jobId: "job-2" });
    expect(mocks.invoke).toHaveBeenNthCalledWith(5, "analysis_job_result", { jobId: "job-2" });
  });
});

describe("native canonical project services", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("normalizes additive canonical collections from legacy desktop snapshots", async () => {
    mocks.invoke.mockResolvedValue({
      name: "Legacy",
      path: null,
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      workspace: null,
    });

    await expect(createNativeProject("Legacy")).resolves.toMatchObject({
      sourceArchiveVersion: 0,
      migrationPending: false,
      compatibilityNotices: [],
      futureUnsupported: { models: 0, recipes: 0, results: 0 },
      saveWarning: null,
      models: [],
      recipes: [],
      results: [],
      activeModelId: null,
      modelPresentations: {},
      savedReports: [],
    });
  });

  it("sends the typed active model with both explicit saves and recovery saves", async () => {
    const model: NativeCanonicalModelSpec = {
      id: "model-1",
      name: "Model",
      constructs: [],
      paths: [],
      controls: [],
      higher_order_constructs: [],
      interactions: [],
    };
    const workspace = { nodes: [], edges: [], activeModelId: model.id };
    const modelPresentation: NativeModelPresentation = { nodes: [], edges: [] };
    mocks.invoke.mockResolvedValueOnce({
      name: "Study",
      path: "D:/projects/study.qpls",
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      models: [model],
      recipes: [],
      results: [],
      activeModelId: model.id,
      workspace,
    }).mockResolvedValueOnce(undefined);

    await saveNativeProject("D:/projects/study.qpls", workspace, model, modelPresentation);
    await autosaveNativeProject("D:/projects/study.qpls", workspace, model, modelPresentation);

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "save_active_project", {
      path: "D:/projects/study.qpls",
      workspace,
      model,
      modelPresentation,
    });
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "autosave_active_project", {
      path: "D:/projects/study.qpls",
      workspace,
      model,
      modelPresentation,
    });
  });

  it("sends one typed explorer mutation request and normalizes its snapshot", async () => {
    const request = {
      mutation: { kind: "rename_model" as const, modelId: "model-1", name: "Revised model" },
      currentModel: null,
      currentPresentation: null,
      path: "D:/projects/study.qpls",
    };
    mocks.invoke.mockResolvedValue({
      name: "Study",
      path: request.path,
      readOnly: false,
      recovered: false,
      datasets: [],
      datasetVersions: [],
      models: [],
      recipes: [],
      results: [],
      activeModelId: null,
      workspace: null,
    });

    await expect(mutateNativeProjectExplorer(request)).resolves.toMatchObject({
      modelPresentations: {},
      savedReports: [],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("mutate_project_explorer", { request });
  });
});

describe("native dataset version services", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("activates a backend-owned version by dataset id and normalizes its row stub", async () => {
    mocks.invoke.mockResolvedValue({ id: "version-2", name: "Version 2", columns: ["score"], missing: 0 });

    await expect(activateNativeDataset("version-2")).resolves.toMatchObject({ id: "version-2", rows: [] });
    expect(mocks.invoke).toHaveBeenCalledWith("activate_dataset", { datasetId: "version-2" });
  });

  it("sends the exact camelCase recode payload and retains the authoritative version record", async () => {
    const spec: RecodeColumnSpec = {
      sourceColumn: "segment",
      targetColumn: "segment_binary",
      targetLabel: "Segment binary",
      targetType: "numeric",
      targetScale: "binary",
      mappings: [{ source: "A", target: "1" }, { source: "B", target: null }],
      unmapped: "error",
    };
    const response = {
      dataset: { id: "version-2", name: "Recode", columns: ["segment", "segment_binary"], missing: 1 },
      version: {
        datasetId: "version-2",
        parentDatasetId: "version-1",
        operation: "recode",
        createdAt: "2026-08-10T12:00:00Z",
        summary: "Recoded segment into segment_binary",
        sourceColumn: "segment",
        targetColumn: "segment_binary",
      },
    };
    mocks.invoke.mockResolvedValue(response);

    await expect(recodeNativeDatasetColumn("version-1", spec)).resolves.toEqual({
      ...response,
      dataset: { ...response.dataset, rows: [] },
    });
    expect(mocks.invoke).toHaveBeenCalledWith("recode_dataset_column", { datasetId: "version-1", spec });
  });
});

describe("native result export service", () => {
  const request = {
    defaultPath: "quickpls-results.csv",
    filterName: "CSV tables",
    extension: "csv" as const,
    contents: "metric,value\npath_coefficient,0.42\n",
  };

  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
  });

  it("uses the native Save dialog before writing a text export", async () => {
    const path = "D:/exports/quickpls-results.csv";
    mocks.save.mockResolvedValue(path);
    mocks.invoke.mockResolvedValue(undefined);

    await expect(exportNativeTextFile(request)).resolves.toBe(path);
    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: request.defaultPath,
      filters: [{ name: request.filterName, extensions: [request.extension] }],
    });
    expect(mocks.invoke).toHaveBeenCalledWith("export_text_file", {
      path,
      contents: request.contents,
    });
  });

  it("treats cancellation as a neutral result without invoking a writer", async () => {
    mocks.save.mockResolvedValue(null);

    await expect(exportNativeTextFile(request)).resolves.toBeNull();
    await expect(exportNativeXlsxTables([])).resolves.toBeNull();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("propagates native write failures for the dialog to report", async () => {
    mocks.save.mockResolvedValue("D:/exports/quickpls-results.csv");
    mocks.invoke.mockRejectedValue(new Error("disk is full"));

    await expect(exportNativeTextFile(request)).rejects.toThrow("disk is full");
  });
});
