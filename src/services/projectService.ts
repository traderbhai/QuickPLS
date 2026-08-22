import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { ResultTable } from "../domain/resultTables";
import type {
  DatasetTransformationPreviewV2,
  DatasetTransformationSpecV2,
} from "../domain/datasetTransformationsV2";
import {
  parseProjectDatasetVersionRecordV1,
  parseProjectDatasetVersionRecordsV1,
} from "../domain/projectDataLineageV1";
import {
  parseInternalRecipeV4CompletedResultV1,
  parseInternalRecipeV4PlsExecutionResultV1,
  type InternalLabsRecipeV4PlsExecutionRequestV1,
  type InternalRecipeV4PlsJobSnapshotV1,
} from "../domain/internalRecipeV4PlsExecution";
import {
  parseInternalRecipeV4CbsemCompletedResultV1,
  parseInternalRecipeV4CbsemExecutionResultV1,
  type InternalLabsRecipeV4CbsemExecutionRequestV1,
  type InternalRecipeV4CbsemJobSnapshotV1,
} from "../domain/internalRecipeV4CbsemExecution";
import {
  parseGeneralSemCbsemCompletedResultV1,
  parseGeneralSemEstimatorPreflightOutcomeV1,
  parseGeneralSemPlsCompletedResultV1,
  parseGeneralSemPlsJobSnapshotV1,
  parseGeneralSemProjectBootstrapOutcomeV1,
  selectGeneralSemExecutionAccessV1,
  type GeneralSemPlsJobRequestV1,
  type GeneralSemProjectBootstrapRequestV1,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import type { CapabilityCellReferenceV2 } from "../domain/canonicalResultDocumentV2";
import type { GeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import type { InternalProjectArchiveV6Wire } from "../domain/internalProjectArchiveV6Wire";
import type {
  InternalProjectSchema6ResultAppendOutcomeV1,
  InternalProjectSchema6ResultAppendRequestV1,
} from "../domain/internalProjectSchema6ResultAppend";
import {
  parseInternalProjectSchema6ResultReadOutcomeV1,
  type InternalProjectSchema6ResultReadRequestV1,
} from "../domain/internalProjectSchema6ResultRead";
import {
  parseNativeCapabilityRegistryV2,
  type NativeCapabilityRegistryV2Snapshot,
} from "../domain/nativeCapabilityRegistryV2";
import {
  INTERNAL_PROJECT_UPGRADE_V6_SURFACE,
  type ProjectUpgradeCancellationV1,
  type ProjectUpgradeExecutionV1,
  type ProjectUpgradeInspectionV1,
  type ProjectUpgradeOutcomeV1,
  type ProjectUpgradePlanInputV1,
  type ProjectUpgradePlanStateV1,
} from "../domain/internalProjectUpgradeV6";
import type { SemModelV4 } from "../domain/semModelV4";
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
  NativeSampleProjectId,
  RecodeColumnSpec,
} from "../types";

export interface ChecksumVerification {
  checksumFile: string | null;
  checked: number;
  verified: number;
  failures: string[];
  message: string;
}

export class NativeLegacyDatasetOperationBlockedError extends Error {
  readonly code = "schema6_general_sem.legacy_dataset_operation_blocked";

  constructor(operation: string, reason: "bound_archive" | "publication_pending") {
    super(reason === "bound_archive"
      ? `${operation} is unavailable because the active dataset is bound to an immutable schema-6 archive.`
      : `${operation} is unavailable while General SEM publication is validating and committing its archive.`);
    this.name = "NativeLegacyDatasetOperationBlockedError";
  }
}

async function boundSchema6SessionActive() {
  const { useInternalProjectArchiveV6Session } = await import("../internalProjectArchiveV6SessionStore");
  const session = useInternalProjectArchiveV6Session.getState().session;
  return Boolean(session?.standardActivation);
}

/** Central application boundary for every legacy dataset mutation service. */
export async function assertNativeLegacyDatasetMutationAllowed(operation: string) {
  const { useWorkspace } = await import("../store");
  if (useWorkspace.getState().generalSemPublicationPending) {
    throw new NativeLegacyDatasetOperationBlockedError(operation, "publication_pending");
  }
  if (await boundSchema6SessionActive()) {
    throw new NativeLegacyDatasetOperationBlockedError(operation, "bound_archive");
  }
}

async function assertNativeLegacyDatasetReadAllowed(operation: string) {
  if (await boundSchema6SessionActive()) {
    throw new NativeLegacyDatasetOperationBlockedError(operation, "bound_archive");
  }
}

export interface NativeTextExportRequest {
  defaultPath: string;
  filterName: string;
  extension: "csv" | "html" | "svg";
  contents: string;
}

export interface DiagnosticRedactionCounts {
  windowsPaths: number;
  emailAddresses: number;
  urlQueriesOrFragments: number;
  bearerTokens: number;
}

export interface DiagnosticSystemMetadata {
  schemaVersion: number;
  quickplsVersion: string;
  releaseChannel: string;
  sourceRevision: string;
  osFamily: string;
  architecture: string;
  desktopRuntime: string;
  locale: string;
  webview2Version: string;
  userDataIncluded: boolean;
  networkAccessed: boolean;
}

export interface DiagnosticEventRow {
  timestamp: string;
  sequence: number;
  severity: string;
  code: string;
}

export interface DiagnosticEntryDescriptor {
  name: string;
  sha256: string;
  bytes: number;
}

export interface DiagnosticManifestContents {
  schemaVersion: number;
  policyVersion: string;
  createdAt: string;
  quickplsVersion: string;
  entries: DiagnosticEntryDescriptor[];
  redactionCounts: DiagnosticRedactionCounts;
  redactionTotal: number;
  archiveLimits: {
    maximumEntries: number;
    maximumEntryBytes: number;
    maximumUncompressedBytes: number;
    maximumArchiveBytes: number;
    compression: "stored";
  };
  localOnly: boolean;
  networkAccessed: boolean;
}

export interface DiagnosticStagedContents {
  system: DiagnosticSystemMetadata;
  events: DiagnosticEventRow[];
  manifest: DiagnosticManifestContents;
}

export interface DiagnosticBundlePreview {
  previewId: string;
  createdAt: string;
  includedCategories: string[];
  excludedCategories: string[];
  redactionCounts: DiagnosticRedactionCounts;
  entryCount: number;
  eventCount: number;
  estimatedUncompressedBytes: number;
  localOnly: boolean;
  networkActivity: "none";
  stagedContents: DiagnosticStagedContents;
}

export interface DiagnosticBundleSaveResult {
  bytes: number;
  archiveSha256: string;
}

export const isNativeDesktop = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export type NativeNewProjectModeV1 = "standard" | "general_sem_v1";

export async function getNativeCapabilityRegistryV2(): Promise<NativeCapabilityRegistryV2Snapshot> {
  return parseNativeCapabilityRegistryV2(await invoke<unknown>("capability_registry_v2"));
}

const normalizeDataset = (dataset: Dataset): Dataset => ({
  ...dataset,
  rows: dataset.rows ?? [],
  columns: dataset.columns ?? [],
});

const normalizeProjectSnapshot = async (project: NativeProjectSnapshot): Promise<NativeProjectSnapshot> => {
  const datasets = (project.datasets ?? []).map(normalizeDataset);
  const datasetVersions = await parseProjectDatasetVersionRecordsV1(
    project.datasetVersions,
    datasets,
  );
  return {
    ...project,
    sourceArchiveVersion: project.sourceArchiveVersion ?? 0,
    migrationPending: project.migrationPending ?? false,
    compatibilityNotices: project.compatibilityNotices ?? [],
    futureUnsupported: project.futureUnsupported ?? { models: 0, recipes: 0, results: 0 },
    saveWarning: project.saveWarning ?? null,
    datasets,
    datasetVersions,
    models: project.models ?? [],
    recipes: project.recipes ?? [],
    results: project.results ?? [],
    activeModelId: project.activeModelId ?? null,
    modelPresentations: project.modelPresentations ?? {},
    savedReports: project.savedReports ?? [],
  };
};

export async function createNativeProject(
  name = "Untitled project",
  projectMode?: NativeNewProjectModeV1,
) {
  const commandArguments = projectMode === undefined ? { name } : { name, projectMode };
  return normalizeProjectSnapshot(
    await invoke<NativeProjectSnapshot>("new_project", commandArguments),
  );
}

/** Clears, but can never issue, the backend General SEM fresh-draft authority. */
export async function invalidateNativeGeneralSemFreshDraftAuthorityV1() {
  return invoke<void>("invalidate_general_sem_fresh_draft_authority_v1");
}

/**
 * Authorizes one source-preserving calculation-ready revision of the exact
 * active project. The subsequent schema-6 publication must still use a new
 * project identity and destination path.
 */
export async function authorizeNativeGeneralSemRevisionDraftV1() {
  return invoke<string>("authorize_general_sem_revision_draft_v1");
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

export async function openNativeDemoProject(sampleId: NativeSampleProjectId = "corporate_reputation") {
  const project = await invoke<NativeProjectSnapshot>("open_demo_project", { sampleId });
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
  await assertNativeLegacyDatasetMutationAllowed("Import data");
  const path = await open({ multiple: false, filters: [{ name: "Research data", extensions: ["csv", "tsv", "txt", "xls", "xlsx", "xlsb", "ods", "sav", "zsav"] }] });
  if (!path) return null;
  await assertNativeLegacyDatasetMutationAllowed("Import data");
  return normalizeDataset(await invoke<Dataset>("import_dataset", { path, dataKind, sampleSize, missingMarkers }));
}

/**
 * Imports one exact CSV path for the packaged named-evidence harness. The
 * backend remains the dataset authority: it parses the file, assigns the
 * resident UUID, computes the fingerprint, and appends import lineage to the
 * active native project. This seam deliberately does not accept dataset bytes
 * or an identity supplied by the renderer.
 */
export async function importNativeDatasetAtPathForValidation(path: string) {
  const exactPath = path.trim();
  if (!/^(?:[A-Za-z]:[\\/]|\\\\)[\s\S]+\.csv$/iu.test(exactPath)) {
    throw new Error("Named SEM validation data requires an absolute Windows CSV path.");
  }
  await assertNativeLegacyDatasetMutationAllowed("Import named SEM validation data");
  return normalizeDataset(await invoke<Dataset>("import_dataset", {
    path: exactPath,
    dataKind: "raw",
    sampleSize: undefined,
    missingMarkers: [],
  }));
}

export async function importNativeValidationFixture() {
  await assertNativeLegacyDatasetMutationAllowed("Import validation data");
  return normalizeDataset(await invoke<Dataset>("import_validation_fixture"));
}

export async function getNativeDatasetRows(datasetId: string, offset: number, limit: number) {
  await assertNativeLegacyDatasetReadAllowed("Legacy dataset row access");
  return invoke<DatasetRowsPage>("dataset_rows", { datasetId, offset, limit });
}

export async function profileNativeDatasetGroups(
  datasetId: string,
  columnName: string,
  analysisColumns: readonly string[],
) {
  await assertNativeLegacyDatasetReadAllowed("Legacy dataset group profiling");
  return invoke<DatasetGroupProfile>("profile_dataset_groups", {
    datasetId,
    columnName,
    analysisColumns: [...analysisColumns],
  });
}

export async function updateNativeColumnMetadata(datasetId: string, columnName: string, metadata: ColumnMetadata) {
  await assertNativeLegacyDatasetMutationAllowed("Update variable metadata");
  return normalizeDataset(await invoke<Dataset>("set_column_metadata", { datasetId, columnName, metadata }));
}

export async function activateNativeDataset(datasetId: string) {
  await assertNativeLegacyDatasetMutationAllowed("Activate a dataset version");
  return normalizeDataset(await invoke<Dataset>("activate_dataset", { datasetId }));
}

export async function recodeNativeDatasetColumn(datasetId: string, spec: RecodeColumnSpec) {
  await assertNativeLegacyDatasetMutationAllowed("Recode data");
  const mutation = await invoke<DatasetVersionMutation>("recode_dataset_column", { datasetId, spec });
  return {
    ...mutation,
    dataset: normalizeDataset(mutation.dataset),
    version: await parseProjectDatasetVersionRecordV1(mutation.version),
  };
}

export async function previewNativeDatasetTransformation(
  datasetId: string,
  spec: DatasetTransformationSpecV2,
) {
  await assertNativeLegacyDatasetMutationAllowed("Preview a dataset transformation");
  return invoke<DatasetTransformationPreviewV2>("preview_dataset_transformation", {
    datasetId,
    spec,
  });
}

export async function applyNativeDatasetTransformation(
  datasetId: string,
  spec: DatasetTransformationSpecV2,
  outputDatasetName: string,
) {
  await assertNativeLegacyDatasetMutationAllowed("Create a transformed dataset version");
  const mutation = await invoke<DatasetVersionMutation>("apply_dataset_transformation", {
    datasetId,
    spec,
    outputDatasetName,
  });
  return {
    ...mutation,
    dataset: normalizeDataset(mutation.dataset),
    version: await parseProjectDatasetVersionRecordV1(mutation.version),
  };
}

/**
 * Internal Experimental Labs bridge for the bounded recipe-v4 core PLS slice.
 * It returns an ephemeral result and is intentionally not used by Calculate.
 */
export async function runInternalLabsRecipeV4PlsExecution(
  request: InternalLabsRecipeV4PlsExecutionRequestV1,
) {
  const response = await invoke<unknown>(
    "run_internal_labs_recipe_v4_pls_execution",
    { request },
  );
  return parseInternalRecipeV4PlsExecutionResultV1(response);
}

/** Starts the cancellable Internal Labs lifecycle; Standard Calculate never calls this. */
export async function startInternalLabsRecipeV4PlsJob(
  request: InternalLabsRecipeV4PlsExecutionRequestV1,
) {
  return invoke<InternalRecipeV4PlsJobSnapshotV1>(
    "start_internal_labs_recipe_v4_pls_job",
    { request },
  );
}

export async function getInternalLabsRecipeV4PlsJob(jobId: string) {
  return invoke<InternalRecipeV4PlsJobSnapshotV1>(
    "internal_labs_recipe_v4_pls_job_status",
    { jobId },
  );
}

export async function cancelInternalLabsRecipeV4PlsJob(jobId: string) {
  return invoke<InternalRecipeV4PlsJobSnapshotV1>(
    "cancel_internal_labs_recipe_v4_pls_job",
    { jobId },
  );
}

export async function dismissInternalLabsRecipeV4PlsJob(jobId: string) {
  return invoke<void>("dismiss_internal_labs_recipe_v4_pls_job", { jobId });
}

export async function getInternalLabsRecipeV4PlsJobResult(jobId: string) {
  const response = await invoke<unknown>(
    "internal_labs_recipe_v4_pls_job_result",
    { jobId },
  );
  return parseInternalRecipeV4CompletedResultV1(response);
}

/**
 * Atomically creates a new schema-6 general_sem_v1 archive from exactly one
 * resident active-project dataset, one promoted SemModelV4, and one RecipeV4.
 */
export async function bootstrapInternalGeneralSemProjectArchiveV6(
  request: GeneralSemProjectBootstrapRequestV1,
) {
  const response = await invoke<unknown>(
    "bootstrap_internal_general_sem_project_archive_v6",
    { request },
  );
  return parseGeneralSemProjectBootstrapOutcomeV1(response);
}

/** Native Rust capability authority for an already populated marked project. */
export async function preflightInternalGeneralSemEstimatorsV1(input: {
  project: InternalProjectArchiveV6Wire;
  modelId: string;
  config: GeneralSemConfigV1;
  capabilityCell: CapabilityCellReferenceV2;
  experimentalLabsEnabled?: boolean;
}) {
  const access = selectGeneralSemExecutionAccessV1({
    capabilityCell: input.capabilityCell,
    experimentalLabsEnabled: input.experimentalLabsEnabled ?? true,
  });
  const response = await invoke<unknown>(
    "preflight_internal_general_sem_estimators_v1",
    {
        request: {
          ...access,
          capabilityCell: input.capabilityCell,
          project: input.project,
          modelId: input.modelId,
          config: input.config,
      },
    },
  );
  return parseGeneralSemEstimatorPreflightOutcomeV1(response);
}

export async function startInternalLabsGeneralSemPlsJobV1(request: GeneralSemPlsJobRequestV1) {
  return parseGeneralSemPlsJobSnapshotV1(await invoke<unknown>(
    "start_internal_labs_general_sem_pls_job_v1",
    { request },
  ));
}

export async function getInternalLabsGeneralSemPlsJobV1(jobId: string) {
  return parseGeneralSemPlsJobSnapshotV1(await invoke<unknown>(
    "status_internal_labs_general_sem_pls_job_v1",
    { jobId },
  ));
}

export async function cancelInternalLabsGeneralSemPlsJobV1(jobId: string) {
  return parseGeneralSemPlsJobSnapshotV1(await invoke<unknown>(
    "cancel_internal_labs_general_sem_pls_job_v1",
    { jobId },
  ));
}

export async function dismissInternalLabsGeneralSemPlsJobV1(jobId: string) {
  return invoke<null>("dismiss_internal_labs_general_sem_pls_job_v1", { jobId });
}

export async function getInternalLabsGeneralSemPlsJobResultV1(jobId: string) {
  return parseGeneralSemPlsCompletedResultV1(await invoke<unknown>(
    "result_internal_labs_general_sem_pls_job_v1",
    { jobId },
  ));
}

export async function startInternalLabsGeneralSemCbsemJobV1(request: GeneralSemPlsJobRequestV1) {
  return parseGeneralSemPlsJobSnapshotV1(await invoke<unknown>(
    "start_internal_labs_general_sem_cbsem_job_v1",
    { request },
  ));
}

export async function getInternalLabsGeneralSemCbsemJobV1(jobId: string) {
  return parseGeneralSemPlsJobSnapshotV1(await invoke<unknown>(
    "status_internal_labs_general_sem_cbsem_job_v1",
    { jobId },
  ));
}

export async function cancelInternalLabsGeneralSemCbsemJobV1(jobId: string) {
  return parseGeneralSemPlsJobSnapshotV1(await invoke<unknown>(
    "cancel_internal_labs_general_sem_cbsem_job_v1",
    { jobId },
  ));
}

export async function dismissInternalLabsGeneralSemCbsemJobV1(jobId: string) {
  return invoke<null>("dismiss_internal_labs_general_sem_cbsem_job_v1", { jobId });
}

export async function getInternalLabsGeneralSemCbsemJobResultV1(jobId: string) {
  return parseGeneralSemCbsemCompletedResultV1(await invoke<unknown>(
    "result_internal_labs_general_sem_cbsem_job_v1",
    { jobId },
  ));
}

/** Internal-only bounded CB-SEM Recipe-v4 bridge; Standard Calculate never calls this. */
export async function runInternalLabsRecipeV4CbsemExecution(
  request: InternalLabsRecipeV4CbsemExecutionRequestV1,
) {
  const response = await invoke<unknown>(
    "run_internal_labs_recipe_v4_cbsem_execution",
    { request },
  );
  return parseInternalRecipeV4CbsemExecutionResultV1(response);
}

/** Native SemModelV4 validation and scientific digest authority for Internal/Labs requests. */
export async function getInternalSemModelV4ScientificSha256(model: SemModelV4) {
  const response = await invoke<unknown>(
    "internal_sem_model_v4_scientific_sha256",
    { model },
  );
  if (typeof response !== "string" || !/^[a-f0-9]{64}$/.test(response)) {
    throw new Error("Native SemModelV4 scientific digest must be an exact lowercase SHA-256 value.");
  }
  return response;
}

export async function startInternalLabsRecipeV4CbsemJob(
  request: InternalLabsRecipeV4CbsemExecutionRequestV1,
) {
  return invoke<InternalRecipeV4CbsemJobSnapshotV1>(
    "start_internal_labs_recipe_v4_cbsem_job",
    { request },
  );
}

export async function getInternalLabsRecipeV4CbsemJob(jobId: string) {
  return invoke<InternalRecipeV4CbsemJobSnapshotV1>(
    "internal_labs_recipe_v4_cbsem_job_status",
    { jobId },
  );
}

export async function cancelInternalLabsRecipeV4CbsemJob(jobId: string) {
  return invoke<InternalRecipeV4CbsemJobSnapshotV1>(
    "cancel_internal_labs_recipe_v4_cbsem_job",
    { jobId },
  );
}

export async function dismissInternalLabsRecipeV4CbsemJob(jobId: string) {
  return invoke<void>("dismiss_internal_labs_recipe_v4_cbsem_job", { jobId });
}

export async function getInternalLabsRecipeV4CbsemJobResult(jobId: string) {
  const response = await invoke<unknown>(
    "internal_labs_recipe_v4_cbsem_job_result",
    { jobId },
  );
  return parseInternalRecipeV4CbsemCompletedResultV1(response);
}

/** Internal schema-6 result persistence boundary; never writes schema-5 projects. */
export async function appendInternalProjectSchema6CanonicalResultV2(
  request: InternalProjectSchema6ResultAppendRequestV1,
) {
  return invoke<InternalProjectSchema6ResultAppendOutcomeV1>(
    "append_internal_project_schema6_canonical_result_v2",
    { request },
  );
}

/** Reopens strict canonical documents from an exact digest-bound schema-6 project. */
export async function readInternalProjectSchema6CanonicalResultsV2(
  request: InternalProjectSchema6ResultReadRequestV1,
) {
  const response = await invoke<unknown>(
    "read_internal_project_schema6_canonical_results_v2",
    { request },
  );
  return parseInternalProjectSchema6ResultReadOutcomeV1(response, request);
}

/**
 * Consumes a successfully completed internal job and attaches its native-built
 * canonical document to an exact digest-bound schema-6 project. The archive
 * command remains atomic and returns a typed blocked outcome on any mismatch.
 */
export async function persistInternalLabsRecipeV4PlsJobResultToSchema6(
  jobId: string,
  archivePath: string,
  expectedSourceSha256: string,
) {
  const completed = await getInternalLabsRecipeV4PlsJobResult(jobId);
  const appendOutcome = await appendInternalProjectSchema6CanonicalResultV2({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    archivePath,
    expectedSourceSha256,
    canonicalDocument: completed.canonicalDocument,
  });
  return { completed, appendOutcome };
}

/**
 * Appends the exact native-built CB-SEM canonical document. TypeScript never
 * reconstructs parameters, fit indices, input moments, or provenance.
 */
export async function persistInternalLabsRecipeV4CbsemJobResultToSchema6(
  jobId: string,
  archivePath: string,
  expectedSourceSha256: string,
) {
  const completed = await getInternalLabsRecipeV4CbsemJobResult(jobId);
  const appendOutcome = await appendInternalProjectSchema6CanonicalResultV2({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    archivePath,
    expectedSourceSha256,
    canonicalDocument: completed.canonicalDocument,
  });
  return { completed, appendOutcome };
}

/**
 * Internal Experimental Labs boundary for inspecting schema-1-through-5
 * projects before a source-preserving schema-6 copy is planned.
 */
export async function inspectInternalProjectUpgradeV6(sourceArchivePath: string) {
  return invoke<ProjectUpgradeOutcomeV1<ProjectUpgradeInspectionV1>>(
    "inspect_internal_project_upgrade_v6",
    {
      request: {
        surface: INTERNAL_PROJECT_UPGRADE_V6_SURFACE,
        experimentalLabsEnabled: true,
        sourceArchivePath,
      },
    },
  );
}

/** Prepares an ephemeral plan. This call never creates or changes a project file. */
export async function planInternalProjectUpgradeV6(input: ProjectUpgradePlanInputV1) {
  return invoke<ProjectUpgradeOutcomeV1<ProjectUpgradePlanStateV1>>(
    "plan_internal_project_upgrade_v6",
    {
      request: {
        surface: INTERNAL_PROJECT_UPGRADE_V6_SURFACE,
        experimentalLabsEnabled: true,
        ...input,
        legacyDisplayCovariances: input.legacyDisplayCovariances ?? {},
        estimandConfirmations: input.estimandConfirmations ?? {},
      },
    },
  );
}

/** Writes only the new destination bound into the exact prepared plan. */
export async function executeInternalProjectUpgradeV6(
  planId: string,
  expectedPlanSha256: string,
) {
  return invoke<ProjectUpgradeOutcomeV1<ProjectUpgradeExecutionV1>>(
    "execute_internal_project_upgrade_v6",
    {
      request: {
        surface: INTERNAL_PROJECT_UPGRADE_V6_SURFACE,
        experimentalLabsEnabled: true,
        planId,
        expectedPlanSha256,
        confirmNewDestination: true,
      },
    },
  );
}

/** Discards an ephemeral plan. Cancellation never writes the destination. */
export async function cancelInternalProjectUpgradeV6(
  planId: string,
  expectedPlanSha256: string,
) {
  return invoke<ProjectUpgradeOutcomeV1<ProjectUpgradeCancellationV1>>(
    "cancel_internal_project_upgrade_v6",
    {
      request: {
        surface: INTERNAL_PROJECT_UPGRADE_V6_SURFACE,
        experimentalLabsEnabled: true,
        planId,
        expectedPlanSha256,
      },
    },
  );
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

export async function previewNativeDiagnosticBundle(replacesPreviewId: string | null = null) {
  return invoke<DiagnosticBundlePreview>("preview_diagnostic_bundle", { replacesPreviewId });
}

export async function cancelNativeDiagnosticBundlePreview(previewId: string) {
  return invoke<void>("cancel_diagnostic_bundle_preview", { previewId });
}

export async function saveNativeDiagnosticBundle(previewId: string) {
  const path = await save({
    defaultPath: "quickpls-diagnostic-bundle.zip",
    filters: [{ name: "QuickPLS diagnostic bundle", extensions: ["zip"] }],
  });
  if (!path) {
    await cancelNativeDiagnosticBundlePreview(previewId);
    return null;
  }
  return invoke<DiagnosticBundleSaveResult>("save_diagnostic_bundle", { path, previewId });
}
