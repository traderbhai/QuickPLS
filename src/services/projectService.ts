import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { ResultTable } from "../domain/resultTables";
import type {
  AnalysisResultEnvelope,
  ColumnMetadata,
  Dataset,
  DatasetGroupProfile,
  DatasetRowsPage,
  DatasetVersionMutation,
  JobSnapshot,
  NativeCanonicalModelSpec,
  NativeModelPresentation,
  NativeProjectExplorerMutationRequest,
  NativeProjectSnapshot,
  RecodeColumnSpec,
} from "../types";

export interface ChecksumVerification {
  checksumFile: string | null;
  checked: number;
  verified: number;
  failures: string[];
  message: string;
}

export interface NativeTextExportRequest {
  defaultPath: string;
  filterName: string;
  extension: "csv" | "html" | "svg";
  contents: string;
}

export const isNativeDesktop = () => "__TAURI_INTERNALS__" in window;

const normalizeDataset = (dataset: Dataset): Dataset => ({
  ...dataset,
  rows: dataset.rows ?? [],
  columns: dataset.columns ?? [],
});

const normalizeProjectSnapshot = (project: NativeProjectSnapshot): NativeProjectSnapshot => ({
  ...project,
  sourceArchiveVersion: project.sourceArchiveVersion ?? 0,
  migrationPending: project.migrationPending ?? false,
  compatibilityNotices: project.compatibilityNotices ?? [],
  futureUnsupported: project.futureUnsupported ?? { models: 0, recipes: 0, results: 0 },
  saveWarning: project.saveWarning ?? null,
  datasets: (project.datasets ?? []).map(normalizeDataset),
  datasetVersions: project.datasetVersions ?? [],
  models: project.models ?? [],
  recipes: project.recipes ?? [],
  results: project.results ?? [],
  activeModelId: project.activeModelId ?? null,
  modelPresentations: project.modelPresentations ?? {},
  savedReports: project.savedReports ?? [],
});

export async function createNativeProject(name = "Untitled project") {
  return normalizeProjectSnapshot(await invoke<NativeProjectSnapshot>("new_project", { name }));
}

export async function openNativeProjectAt(path: string) {
  const project = await invoke<NativeProjectSnapshot>("open_project", { path });
  return normalizeProjectSnapshot(project);
}

export async function openNativeProject() {
  const path = await open({ multiple: false, filters: [{ name: "QuickPLS project", extensions: ["qpls"] }] });
  if (!path) return null;
  return openNativeProjectAt(path);
}

export async function openNativeDemoProject() {
  const project = await invoke<NativeProjectSnapshot>("open_demo_project");
  return normalizeProjectSnapshot(project);
}

export async function saveNativeProject(
  currentPath: string | null,
  workspace: unknown,
  model: NativeCanonicalModelSpec | null = null,
  modelPresentation: NativeModelPresentation | null = null,
) {
  const path = currentPath ?? await save({ defaultPath: "study.qpls", filters: [{ name: "QuickPLS project", extensions: ["qpls"] }] });
  if (!path) return null;
  return normalizeProjectSnapshot(await invoke<NativeProjectSnapshot>("save_active_project", {
    path,
    workspace,
    model,
    modelPresentation,
  }));
}

export async function autosaveNativeProject(
  path: string,
  workspace: unknown,
  model: NativeCanonicalModelSpec | null = null,
  modelPresentation: NativeModelPresentation | null = null,
) {
  return invoke<void>("autosave_active_project", { path, workspace, model, modelPresentation });
}

export async function mutateNativeProjectExplorer(request: NativeProjectExplorerMutationRequest) {
  return normalizeProjectSnapshot(await invoke<NativeProjectSnapshot>("mutate_project_explorer", { request }));
}

export async function importNativeDataset(dataKind: "raw" | "covariance" | "correlation" = "raw", sampleSize?: number, missingMarkers?: string[]) {
  const path = await open({ multiple: false, filters: [{ name: "Research data", extensions: ["csv", "tsv", "txt", "xls", "xlsx", "xlsb", "ods", "sav", "zsav"] }] });
  if (!path) return null;
  return normalizeDataset(await invoke<Dataset>("import_dataset", { path, dataKind, sampleSize, missingMarkers }));
}

export async function importNativeValidationFixture() {
  return normalizeDataset(await invoke<Dataset>("import_validation_fixture"));
}

export async function getNativeDatasetRows(datasetId: string, offset: number, limit: number) {
  return invoke<DatasetRowsPage>("dataset_rows", { datasetId, offset, limit });
}

export async function profileNativeDatasetGroups(
  datasetId: string,
  columnName: string,
  analysisColumns: readonly string[],
) {
  return invoke<DatasetGroupProfile>("profile_dataset_groups", {
    datasetId,
    columnName,
    analysisColumns: [...analysisColumns],
  });
}

export async function updateNativeColumnMetadata(datasetId: string, columnName: string, metadata: ColumnMetadata) {
  return normalizeDataset(await invoke<Dataset>("set_column_metadata", { datasetId, columnName, metadata }));
}

export async function activateNativeDataset(datasetId: string) {
  return normalizeDataset(await invoke<Dataset>("activate_dataset", { datasetId }));
}

export async function recodeNativeDatasetColumn(datasetId: string, spec: RecodeColumnSpec) {
  const mutation = await invoke<DatasetVersionMutation>("recode_dataset_column", { datasetId, spec });
  return { ...mutation, dataset: normalizeDataset(mutation.dataset) };
}

export async function startNativeAnalysisJob(recipe: unknown) {
  return invoke<JobSnapshot>("start_analysis_job", { recipe });
}

export async function getNativeAnalysisJob(jobId: string) {
  return invoke<JobSnapshot>("analysis_job_status", { jobId });
}

export async function cancelNativeAnalysisJob(jobId: string) {
  return invoke<JobSnapshot>("cancel_analysis_job", { jobId });
}

export async function dismissNativeAnalysisJob(jobId: string) {
  return invoke<void>("dismiss_analysis_job", { jobId });
}

export async function getNativeAnalysisJobResult(jobId: string) {
  return invoke<AnalysisResultEnvelope | null>("analysis_job_result", { jobId });
}

/** @deprecated Use startNativeAnalysisJob. Retained for one major release. */
export async function startNativePlsJob(recipe: unknown) {
  return startNativeAnalysisJob(recipe);
}

/** @deprecated Use getNativeAnalysisJob. Retained for one major release. */
export async function getNativePlsJob(jobId: string) {
  return getNativeAnalysisJob(jobId);
}

/** @deprecated Use cancelNativeAnalysisJob. Retained for one major release. */
export async function cancelNativePlsJob(jobId: string) {
  return cancelNativeAnalysisJob(jobId);
}

/** @deprecated Use dismissNativeAnalysisJob. Retained for one major release. */
export async function dismissNativePlsJob(jobId: string) {
  return dismissNativeAnalysisJob(jobId);
}

/** @deprecated Use getNativeAnalysisJobResult. Retained for one major release. */
export async function getNativePlsJobResult(jobId: string) {
  return getNativeAnalysisJobResult(jobId);
}

export async function exportNativeXlsxTables(tables: ResultTable[]) {
  const path = await save({ defaultPath: "quickpls-result-tables.xlsx", filters: [{ name: "Excel workbook", extensions: ["xlsx"] }] });
  if (!path) return null;
  await invoke<void>("export_xlsx_tables", { path, tables });
  return path;
}

export async function exportNativeTextFile({
  defaultPath,
  filterName,
  extension,
  contents,
}: NativeTextExportRequest) {
  const path = await save({
    defaultPath,
    filters: [{ name: filterName, extensions: [extension] }],
  });
  if (!path) return null;
  await invoke<void>("export_text_file", { path, contents });
  return path;
}

export async function openNativeDefaultExportFolder() {
  return invoke<string>("open_default_export_folder");
}

export async function verifyNativeLatestReleaseChecksums() {
  return invoke<ChecksumVerification>("verify_latest_release_checksums");
}
