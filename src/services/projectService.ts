import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { ResultTable } from "../domain/resultTables";
import type { AnalysisResultEnvelope, ColumnMetadata, Dataset, JobSnapshot, NativeProjectSnapshot } from "../types";

export const isNativeDesktop = () => "__TAURI_INTERNALS__" in window;

const normalizeDataset = (dataset: Dataset): Dataset => ({
  ...dataset,
  rows: dataset.rows ?? [],
  columns: dataset.columns ?? [],
});

export async function createNativeProject(name = "Untitled project") {
  return invoke<NativeProjectSnapshot>("new_project", { name });
}

export async function openNativeProject() {
  const path = await open({ multiple: false, filters: [{ name: "QuickPLS project", extensions: ["qpls"] }] });
  if (!path) return null;
  const project = await invoke<NativeProjectSnapshot>("open_project", { path });
  return { ...project, datasets: project.datasets.map(normalizeDataset) };
}

export async function openNativeDemoProject() {
  const project = await invoke<NativeProjectSnapshot>("open_demo_project");
  return { ...project, datasets: project.datasets.map(normalizeDataset) };
}

export async function saveNativeProject(currentPath: string | null, workspace: unknown) {
  const path = currentPath ?? await save({ defaultPath: "study.qpls", filters: [{ name: "QuickPLS project", extensions: ["qpls"] }] });
  if (!path) return null;
  return invoke<NativeProjectSnapshot>("save_active_project", { path, workspace });
}

export async function autosaveNativeProject(path: string, workspace: unknown) {
  return invoke<void>("autosave_active_project", { path, workspace });
}

export async function importNativeDataset(dataKind: "raw" | "covariance" | "correlation" = "raw", sampleSize?: number, missingMarkers?: string[]) {
  const path = await open({ multiple: false, filters: [{ name: "Research data", extensions: ["csv", "tsv", "txt", "xls", "xlsx", "xlsb", "ods", "sav", "zsav"] }] });
  if (!path) return null;
  return normalizeDataset(await invoke<Dataset>("import_dataset", { path, dataKind, sampleSize, missingMarkers }));
}

export async function importNativeValidationFixture() {
  return normalizeDataset(await invoke<Dataset>("import_validation_fixture"));
}

export async function updateNativeColumnMetadata(datasetId: string, columnName: string, metadata: ColumnMetadata) {
  return normalizeDataset(await invoke<Dataset>("set_column_metadata", { datasetId, columnName, metadata }));
}

export async function startNativePlsJob(recipe: unknown) {
  return invoke<JobSnapshot>("start_pls_job", { recipe });
}

export async function getNativePlsJob(jobId: string) {
  return invoke<JobSnapshot>("pls_job_status", { jobId });
}

export async function cancelNativePlsJob(jobId: string) {
  return invoke<JobSnapshot>("cancel_pls_job", { jobId });
}

export async function dismissNativePlsJob(jobId: string) {
  return invoke<void>("dismiss_pls_job", { jobId });
}

export async function getNativePlsJobResult(jobId: string) {
  return invoke<AnalysisResultEnvelope | null>("pls_job_result", { jobId });
}

export async function exportNativeXlsxTables(tables: ResultTable[]) {
  const path = await save({ defaultPath: "quickpls-result-tables.xlsx", filters: [{ name: "Excel workbook", extensions: ["xlsx"] }] });
  if (!path) return null;
  await invoke<void>("export_xlsx_tables", { path, tables });
  return path;
}
