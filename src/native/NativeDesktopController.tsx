import Papa from "papaparse";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useRef, useState } from "react";
import { defaultDiagramLayout } from "../domain/diagramGraph";
import {
  autosaveNativeProject,
  cancelNativePlsJob,
  createNativeProject,
  dismissNativePlsJob,
  getNativePlsJob,
  getNativePlsJobResult,
  importNativeDataset,
  invalidateNativeGeneralSemFreshDraftAuthorityV1,
  isNativeDesktop,
  mutateNativeProjectExplorer,
  openNativeDemoProject,
  openNativeProjectAt,
  readInternalProjectSchema6CanonicalResultsV2,
  saveNativeProject,
  startNativePlsJob,
} from "../services/projectService";
import {
  inspectInternalProjectArchiveV6At,
  selectQuickPlsProjectArchivePath,
} from "../services/internalProjectArchiveV6ReadService";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import {
  generalSemWorkspaceProductAccessV1,
  rehydrateGeneralSemExecutionAuthorityV1,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import { selectLatestGeneralSemReopenedEntryV1 } from "./NativeRecipeV4GeneralSemWorkspace";
import {
  hasUnsavedNativeProjectChanges,
  nativeLegacyProjectOperationBlocker,
  nativeProjectSignature,
  nativeSavedProjectSignature,
  nativeSchema6BoundWorkspaceReplacementBlocker,
  resolveNativeWorkspaceReplacementChoiceV1,
} from "./nativeProjectLifecycle";
import {
  buildNativeAnalysisRecipe,
  buildNativeRecipeModel,
  nativeAnalysisRecipeDescriptor,
} from "./nativeAnalysisRecipe";
import {
  compactNativeWorkspaceRuns,
  currentNativeModelPresentation,
  reconcileNativeCanonicalProject,
} from "./nativeCanonicalProject";
import { parseNativeCalculationRequest, type NativeCalculationRequest } from "./nativeCalculationRequest";
import { isStandaloneNativeAnalysis } from "./nativeStandaloneAnalysis";
import {
  DEFAULT_NATIVE_DATA_IMPORT_REQUEST,
  normalizeNativeDataImportRequest,
  type NativeDataImportRequest,
} from "./nativeDataImport";
import {
  isCalculationActive,
  transitionCalculationMonitor,
  type CalculationMonitorPatch,
} from "./nativeCalculationLifecycle";
import { nativePlsReadiness } from "./nativePlsReadiness";
import { nativeLogisticReadiness } from "./nativeLogistic";
import { nativeProcessReadiness } from "./nativeProcess";
import { nativePlsSampleSizePowerRecipeFromCanonical } from "./nativePlsSampleSizePower";
import { createAnalysisModelSnapshot } from "./nativeRunModelSnapshot";
import {
  isStructuralPathRandomizationIdentityPresent,
  nativeStructuralPathRandomizationProjection,
  nativeStructuralPathRandomizationRecipeMatches,
} from "./nativeStructuralPathRandomization";
import { useWorkspace } from "../store";
import { useInternalProjectArchiveV6Session } from "../internalProjectArchiveV6SessionStore";
import type {
  AnalysisRun,
  AnalysisUiSettings,
  Dataset,
  DiagramLayoutState,
  DiagramMode,
  DiagramOverlaySettings,
  JobSnapshot,
  NativeExplorerSelection,
  NativeProjectExplorerMutation,
  NativeProjectExplorerMutationEventDetail,
  NativeProjectSnapshot,
  PublicationDiagramSettings,
  RunMonitorLogEntry,
  NativeSampleProjectId,
} from "../types";

export type NativeControllerEvent =
  | "quickpls:new-project"
  | "quickpls:open-project"
  | "quickpls:open-project-path"
  | "quickpls:open-demo-project"
  | "quickpls:save-project"
  | "quickpls:save-project-as"
  | "quickpls:import-data"
  | "quickpls:run-analysis"
  | "quickpls:cancel-analysis"
  | "quickpls:open-explorer-data"
  | "quickpls:open-explorer-model"
  | "quickpls:open-explorer-report"
  | "quickpls:mutate-project-explorer"
  | "quickpls:set-explorer-selection";

type NativeNewProjectMode = "standard" | "general_sem_v1";

interface ProjectWorkspaceSnapshot {
  nodes: ReturnType<typeof useWorkspace.getState>["nodes"];
  edges: ReturnType<typeof useWorkspace.getState>["edges"];
  runs?: ReturnType<typeof useWorkspace.getState>["runs"] | ReturnType<typeof compactNativeWorkspaceRuns>;
  analysisSettings?: AnalysisUiSettings;
  diagramMode?: DiagramMode;
  diagramOverlaySettings?: Partial<DiagramOverlaySettings>;
  publicationDiagramSettings?: Partial<PublicationDiagramSettings>;
  diagramLayout?: Partial<DiagramLayoutState>;
  activeDatasetId?: string;
  activeModelId?: string;
}

const OPEN_PROJECT_PLACEHOLDER = "No project open";
const AUTOSAVE_DEBOUNCE_MS = 2_000;

function currentWorkspaceSnapshot(
  activeModelId: string | null = null,
  compactCanonicalRuns = false,
): ProjectWorkspaceSnapshot {
  const state = useWorkspace.getState();
  return {
    nodes: state.nodes,
    edges: state.edges,
    runs: compactCanonicalRuns ? compactNativeWorkspaceRuns(state.runs) : state.runs,
    analysisSettings: state.analysisSettings,
    diagramMode: state.diagramMode,
    diagramOverlaySettings: state.diagramOverlaySettings,
    publicationDiagramSettings: state.publicationDiagramSettings,
    diagramLayout: state.diagramLayout,
    activeDatasetId: state.dataset.id,
    ...(activeModelId ? { activeModelId } : {}),
  };
}

function currentProjectSignatureInput(state = useWorkspace.getState()) {
  return {
    dataset: state.dataset,
    datasetCatalog: state.datasetCatalog,
    datasetVersions: state.datasetVersions,
    projectModels: state.projectModels,
    activeModelId: state.activeModelId,
    modelPresentations: state.modelPresentations,
    savedReports: state.savedReports,
    nodes: state.nodes,
    edges: state.edges,
    runs: state.runs,
    analysisSettings: state.analysisSettings,
    diagramMode: state.diagramMode,
    diagramOverlaySettings: state.diagramOverlaySettings,
    publicationDiagramSettings: state.publicationDiagramSettings,
    diagramLayout: state.diagramLayout,
  };
}
function currentProjectSignature() {
  return nativeProjectSignature(currentProjectSignatureInput());
}

function currentModelName(
  state: ReturnType<typeof useWorkspace.getState>,
  modelId: string,
) {
  return state.projectModels.find((model) => model.id === modelId)?.name ?? state.projectName;
}

function currentModelArtifacts(
  state: ReturnType<typeof useWorkspace.getState>,
  modelId: string,
) {
  return {
    model: buildNativeRecipeModel(
      modelId,
      currentModelName(state, modelId),
      state.nodes,
      state.edges,
    ),
    presentation: currentNativeModelPresentation(state.nodes, state.edges, state.diagramLayout),
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function download(name: string, contents: string, type: string) {
  const url = URL.createObjectURL(new Blob([contents], { type }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = name;
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

export function NativeDesktopController() {
  const projectInputRef = useRef<HTMLInputElement>(null);
  const dataInputRef = useRef<HTMLInputElement>(null);
  const pendingWebImportRef = useRef<NativeDataImportRequest>({
    ...DEFAULT_NATIVE_DATA_IMPORT_REQUEST,
    missingMarkers: [...DEFAULT_NATIVE_DATA_IMPORT_REQUEST.missingMarkers],
  });
  const initialized = useRef(false);
  const closeBypassRef = useRef(false);
  const closePromptOpenRef = useRef(false);
  const replacementPromptOpenRef = useRef(false);
  const autosaveFailureSignatureRef = useRef<string | null>(null);
  const dirtyRef = useRef(false);
  const authorityOperationPendingRef = useRef(false);
  const schema6SourceBindingRef = useRef({ bound: false, dirty: false });
  const calculationActiveRef = useRef(false);
  const generalSemPublicationPendingRef = useRef(false);
  const generalSemTransientWorkBlockerRef = useRef<"job_active" | "temporary_result_pending" | null>(null);
  const calculationCancellationRequestedRef = useRef(false);
  const projectSignatureRef = useRef("");
  const projectPathRef = useRef<string | null>(null);
  const projectNameRef = useRef(OPEN_PROJECT_PLACEHOLDER);
  const projectWritableRef = useRef(true);
  const saveProjectRef = useRef<(saveAs: boolean) => Promise<boolean>>(async () => false);
  const [activeJob, setActiveJob] = useState<JobSnapshot | null>(null);
  const [cleanSignature, setCleanSignature] = useState<string | null>(null);
  const [projectWritable, setProjectWritable] = useState(true);

  const dataset = useWorkspace((state) => state.dataset);
  const strictAuthorityActive = useWorkspace((state) => Object.keys(state.standardSemModelV4Authorities).length > 0);
  const strictAuthorityDirty = useWorkspace((state) => {
    const modelIds = Object.keys(state.standardSemModelV4Authorities);
    if (!modelIds.length) return false;
    const captured = state.captureStandardSemModelV4SaveAuthorities(modelIds);
    return !captured || Object.values(captured).some((authority) => authority.dirty);
  });
  const standardActivationPending = useInternalProjectArchiveV6Session((state) => state.standardActivationPending);
  const saveCopyPending = useInternalProjectArchiveV6Session((state) => state.saveCopyPending);
  const schema6SourceBound = useInternalProjectArchiveV6Session((state) => Boolean(state.session?.standardActivation));
  const schema6SourceDirty = useInternalProjectArchiveV6Session((state) => state.dirty);
  const datasetCatalog = useWorkspace((state) => state.datasetCatalog);
  const datasetVersions = useWorkspace((state) => state.datasetVersions);
  const projectModels = useWorkspace((state) => state.projectModels);
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const modelPresentations = useWorkspace((state) => state.modelPresentations);
  const savedReports = useWorkspace((state) => state.savedReports);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const projectName = useWorkspace((state) => state.projectName);
  const projectPath = useWorkspace((state) => state.projectPath);
  const generalSemProjectDraftMode = useWorkspace((state) => state.generalSemProjectDraftMode);
  const generalSemPublicationPending = useWorkspace((state) => state.generalSemPublicationPending);
  const generalSemTransientWorkBlocker = useWorkspace((state) => state.generalSemTransientWorkBlocker);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const runMonitorStatus = useWorkspace((state) => state.runMonitor.status);
  const loadProject = useWorkspace((state) => state.loadProject);
  const closeProject = useWorkspace((state) => state.closeProject);
  const setProjectMeta = useWorkspace((state) => state.setProjectMeta);
  const setDataset = useWorkspace((state) => state.setDataset);
  const setDatasetCatalog = useWorkspace((state) => state.setDatasetCatalog);
  const setProjectExplorer = useWorkspace((state) => state.setProjectExplorer);
  const setExplorerSelection = useWorkspace((state) => state.setExplorerSelection);
  const switchProjectModel = useWorkspace((state) => state.switchProjectModel);
  const setSelectedResultRun = useWorkspace((state) => state.setSelectedResultRun);
  const setProjectWritableState = useWorkspace((state) => state.setProjectWritable);
  const beginGeneralSemProjectDraftMode = useWorkspace((state) => state.beginGeneralSemProjectDraftMode);
  const addRun = useWorkspace((state) => state.addRun);
  const setRunMonitor = useWorkspace((state) => state.setRunMonitor);
  const pushToast = useWorkspace((state) => state.pushToast);
  const transitionRunMonitor = (
    patch: CalculationMonitorPatch,
    log: Omit<RunMonitorLogEntry, "id" | "timestamp"> | null = null,
  ) => {
    transitionCalculationMonitor(useWorkspace.getState().runMonitor, patch);
    setRunMonitor(patch, log);
  };


  const projectSignature = useMemo(() => nativeProjectSignature({
    dataset,
    datasetCatalog,
    datasetVersions,
    projectModels,
    activeModelId,
    modelPresentations,
    savedReports,
    nodes,
    edges,
    runs,
    analysisSettings,
    diagramMode,
    diagramOverlaySettings,
    publicationDiagramSettings,
    diagramLayout,
  }), [
    analysisSettings,
    dataset,
    datasetCatalog,
    datasetVersions,
    projectModels,
    activeModelId,
    modelPresentations,
    savedReports,
    diagramLayout,
    diagramMode,
    diagramOverlaySettings,
    edges,
    nodes,
    publicationDiagramSettings,
    runs,
  ]);
  const projectOpen = projectName !== OPEN_PROJECT_PLACEHOLDER;
  const authorityOperationPending = standardActivationPending || saveCopyPending;
  const isDirty = hasUnsavedNativeProjectChanges(projectOpen, cleanSignature, projectSignature, {
    active: strictAuthorityActive,
    dirty: strictAuthorityDirty,
    operationPending: authorityOperationPending,
  });
  const calculationActive = isCalculationActive(runMonitorStatus);

  dirtyRef.current = isDirty;
  authorityOperationPendingRef.current = authorityOperationPending;
  schema6SourceBindingRef.current = { bound: schema6SourceBound, dirty: schema6SourceDirty };
  calculationActiveRef.current = calculationActive;
  generalSemPublicationPendingRef.current = generalSemPublicationPending;
  generalSemTransientWorkBlockerRef.current = generalSemTransientWorkBlocker;
  projectSignatureRef.current = projectSignature;
  projectPathRef.current = projectPath;
  projectNameRef.current = projectName;
  projectWritableRef.current = projectWritable;

  const markWorkspaceCleanAt = (signature: string) => {
    const currentSignature = currentProjectSignature();
    dirtyRef.current = currentSignature !== signature;
    projectSignatureRef.current = currentSignature;
    setCleanSignature(signature);
  };

  const markCurrentWorkspaceClean = () => {
    markWorkspaceCleanAt(currentProjectSignature());
  };

  const markRecoveredWorkspaceDirty = () => {
    const signature = currentProjectSignature();
    dirtyRef.current = true;
    projectSignatureRef.current = signature;
    setCleanSignature("recovered:" + signature);
  };

  const updateProjectWritable = (writable: boolean) => {
    projectWritableRef.current = writable;
    setProjectWritable(writable);
    setProjectWritableState(writable);
  };

  const confirmWorkspaceReplacement = async (action: string): Promise<boolean> => {
    if (generalSemPublicationPendingRef.current) {
      pushToast({ tone: "warning", title: "Project publication in progress", detail: `Wait for the calculation-ready project file to finish publishing and validating before ${action}.` });
      return false;
    }
    if (generalSemTransientWorkBlockerRef.current) {
      const temporaryResult = generalSemTransientWorkBlockerRef.current === "temporary_result_pending";
      pushToast({
        tone: "warning",
        title: temporaryResult ? "Advanced result not yet secured" : "Advanced calculation in progress",
        detail: temporaryResult
          ? `Save and strictly reopen the result, or dismiss it explicitly, before ${action}.`
          : `Finish or cancel the General SEM calculation before ${action}.`,
      });
      return false;
    }
    if (calculationActiveRef.current) {
      pushToast({ tone: "warning", title: "Calculation in progress", detail: `Finish or cancel the calculation before ${action}.` });
      return false;
    }
    if (authorityOperationPendingRef.current) {
      pushToast({ tone: "warning", title: "Schema-6 operation in progress", detail: `Wait for Standard activation or validated save-copy to finish before ${action}.` });
      return false;
    }
    const schema6BindingBlocker = nativeSchema6BoundWorkspaceReplacementBlocker(
      schema6SourceBindingRef.current.bound,
      schema6SourceBindingRef.current.dirty,
    );
    if (schema6BindingBlocker) {
      pushToast({
        tone: "warning",
        title: "Schema-6 Standard project still bound",
        detail: `${schema6BindingBlocker} before ${action}.`,
      });
      return false;
    }
    if (!dirtyRef.current) return true;
    if (!isNativeDesktop()) return window.confirm(`Discard unsaved changes to ${projectNameRef.current} before ${action}?`);
    if (replacementPromptOpenRef.current) return false;
    replacementPromptOpenRef.current = true;
    try {
      const choice = await message(
        `Save changes to ${projectNameRef.current} before ${action}?`,
        {
          title: "QuickPLS",
          kind: "warning",
          buttons: { yes: "Save", no: "Don't Save", cancel: "Cancel" },
        },
      );
      return resolveNativeWorkspaceReplacementChoiceV1(
        choice,
        () => saveProjectRef.current(!projectWritableRef.current),
        () => dirtyRef.current,
      );
    } catch (error) {
      pushToast({ tone: "error", title: "Project switch failed", detail: errorMessage(error) });
      return false;
    } finally {
      replacementPromptOpenRef.current = false;
    }
  };


  const loadNativeSnapshot = (
    project: NativeProjectSnapshot | null,
    successTitle = "Project opened",
    options: { announce?: boolean; navigate?: boolean; establishBaseline?: boolean } = {},
  ) => {
    if (!project) return false;
    const workspaceBefore = useWorkspace.getState();
    const preservedGeneralSemDraft = workspaceBefore.generalSemProjectDraftMode
      && workspaceBefore.projectId === project.projectId
      && workspaceBefore.projectPath === null
      && project.path === null
      ? workspaceBefore.generalSemProjectDraftMode
      : undefined;
    const workspace = project.workspace as ProjectWorkspaceSnapshot | null | undefined;
    const canonical = reconcileNativeCanonicalProject(project);
    const activeDataset = project.datasets.find((candidate) => candidate.id === workspace?.activeDatasetId)
      ?? project.datasets[0]
      ?? { id: "empty", name: "No dataset", columns: [], rows: [], missing: 0, rowCount: 0 };
    loadProject({
      nodes: canonical.nodes,
      edges: canonical.edges,
      dataset: activeDataset,
      datasets: project.datasets,
      datasetVersions: project.datasetVersions,
      projectModels: canonical.projectModels,
      activeModelId: canonical.activeModelId,
      modelPresentations: canonical.modelPresentations,
      savedReports: canonical.savedReports,
      explorerSelection: canonical.explorerSelection,
      runs: canonical.runs,
      analysisSettings: workspace?.analysisSettings,
      diagramMode: workspace?.diagramMode,
      diagramOverlaySettings: workspace?.diagramOverlaySettings,
      publicationDiagramSettings: workspace?.publicationDiagramSettings,
      diagramLayout: canonical.diagramLayout ?? workspace?.diagramLayout,
      preserveGeneralSemProjectDraftMode: preservedGeneralSemDraft,
    });
    setProjectMeta(project.name, project.path, project.projectId);
    updateProjectWritable(!project.readOnly);
    if (options.establishBaseline !== false) {
      if (project.recovered) {
        markRecoveredWorkspaceDirty();
      } else {
        markCurrentWorkspaceClean();
      }
    }
    if (options.announce !== false) {
      pushToast({
        tone: project.recovered || project.readOnly || project.migrationPending ? "warning" : "success",
        title: project.recovered
          ? "Project recovered"
          : project.readOnly
            ? "Project opened read-only"
            : project.migrationPending
              ? "Project upgrade pending"
              : successTitle,
        detail: project.readOnly
          ? `${project.name} uses archive schema ${project.sourceArchiveVersion}; compatible content can be viewed and exported, but this app will not modify or resave it.${Object.values(project.futureUnsupported).some((count) => count > 0) ? ` Unsupported items hidden: ${project.futureUnsupported.models} models, ${project.futureUnsupported.recipes} recipes, ${project.futureUnsupported.results} results.` : ""}`
          : project.migrationPending
            ? `${project.name} was opened from archive schema ${project.sourceArchiveVersion}. The original file will be retained as a backup on the first explicit save.`
            : project.name,
      });
      if (project.compatibilityNotices.length) {
        pushToast({
          tone: "warning",
          title: "Historical result compatibility",
          detail: project.compatibilityNotices.length === 1
            ? project.compatibilityNotices[0].message
            : `${project.compatibilityNotices.length} historical results remain readable under their original method versions.`,
        });
      }
    }
    if (options.navigate !== false) {
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: canonical.nodes.length ? "model" : "data" } }));
    }
    return true;
  };

  const createProject = async (name: string, mode: NativeNewProjectMode = "standard") => {
    if (!await confirmWorkspaceReplacement("creating a new project")) return;
    if (isNativeDesktop()) {
      const created = await createNativeProject(name, mode === "general_sem_v1" ? "general_sem_v1" : undefined);
      if (!created) return;
      if (!loadNativeSnapshot(created, mode === "general_sem_v1" ? "Calculation-ready project started" : "Project created")) return;
      if (mode === "general_sem_v1") {
        if (!beginGeneralSemProjectDraftMode(created.projectId)) {
          throw new Error("The fresh native project did not satisfy the empty General SEM draft authority contract.");
        }
        pushToast({
          tone: "info",
          title: "Calculation-ready project active",
          detail: "Import raw data, author the Canvas, then click Calculate to review, save, and activate the scientific model before its first advanced calculation.",
        });
      }
    } else {
      if (mode === "general_sem_v1") throw new Error("Calculation-ready project creation requires the installed QuickPLS desktop app.");
      closeProject();
      setProjectMeta(name, null);
      updateProjectWritable(true);
      markCurrentWorkspaceClean();
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "data" } }));
      pushToast({ tone: "success", title: "Project created", detail: name });
    }
  };

  const openProject = async (path?: string) => {
    if (!await confirmWorkspaceReplacement(path ? "opening another project" : "opening a project")) return;
    if (!isNativeDesktop()) {
      projectInputRef.current?.click();
      return;
    }
    const selectedPath = path ?? await selectQuickPlsProjectArchivePath();
    if (!selectedPath) return;
    const inspected = await inspectInternalProjectArchiveV6At(selectedPath);
    if (inspected.status === "ok" && supportsGeneralSemV1(inspected.value.project)) {
      await invalidateNativeGeneralSemFreshDraftAuthorityV1();
      // Activation revokes any backend-only fresh-draft token before resolving
      // and installing Standard authority, so a previous draft cannot survive.
      const sessionStore = useInternalProjectArchiveV6Session.getState();
      let openedHere = false;
      try {
        const opened = await sessionStore.open(async () => ({ status: "ok", value: inspected.value }));
        if (opened !== "activated") throw new Error(`The General SEM archive could not enter the strict schema-6 session (${opened}).`);
        openedHere = true;
        const activated = await useInternalProjectArchiveV6Session.getState().activateStandardAuthorities();
        if (activated !== "activated") throw new Error(`The General SEM archive could not activate its Standard canvas authority (${activated}).`);
        const execution = rehydrateGeneralSemExecutionAuthorityV1(inspected.value);
        const active = useWorkspace.getState();
        if (active.activeModelId !== execution.receipt.residentModelId
          || active.standardSemModelV4Persistence[execution.receipt.residentModelId]?.scientificSha256
            !== execution.receipt.residentModelScientificSha256) {
          throw new Error("The reopened General SEM model differs from its native execution authority.");
        }
        active.setProjectMeta(inspected.value.project.name, inspected.value.archivePath, inspected.value.project.project_id);
        active.clearGeneralSemProjectDraftMode();
        updateProjectWritable(false);
        markCurrentWorkspaceClean();
        try {
          const resultReadback = await readInternalProjectSchema6CanonicalResultsV2({
            ...execution.readAccess,
            capabilityCell: execution.capabilityCell,
            archivePath: inspected.value.archivePath,
            expectedSourceSha256: inspected.value.archiveSha256,
          });
          if (resultReadback.status === "ok"
            && resultReadback.value.projectId === execution.receipt.projectId
            && resultReadback.value.archivePath === inspected.value.archivePath
            && resultReadback.value.sourceDocumentSha256 === inspected.value.archiveSha256) {
            const latest = selectLatestGeneralSemReopenedEntryV1(
              resultReadback.value.documents,
              execution.receipt,
            );
            if (latest) {
              window.dispatchEvent(new CustomEvent("quickpls:general-sem-canonical-result", {
                detail: { document: latest.canonicalDocument, navigate: false },
              }));
            }
          }
        } catch {
          // Result restoration is fail-closed: the verified model remains open,
          // but no unverified or stale canonical document is exposed in Results.
        }
        pushToast({
          tone: "success",
          title: "Calculation-ready project opened",
          detail: `${inspected.value.project.name} was strictly validated and restored from its resident RecipeV4 authority.`,
        });
        window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "model" } }));
        return;
      } catch (error) {
        if (openedHere) {
          const currentSession = useInternalProjectArchiveV6Session.getState();
          if (currentSession.session?.standardActivation) currentSession.closeStandardProject();
          else currentSession.deactivate();
        }
        throw error;
      }
    }
    loadNativeSnapshot(await openNativeProjectAt(selectedPath));
  };

  const openDemoProject = async (sampleId: NativeSampleProjectId = "corporate_reputation") => {
    if (!isNativeDesktop()) return;
    if (!await confirmWorkspaceReplacement("opening the sample project")) return;
    loadNativeSnapshot(await openNativeDemoProject(sampleId));
  };

  const saveProject = async (saveAs: boolean): Promise<boolean> => {
    const currentState = useWorkspace.getState();
    if (currentState.generalSemProjectDraftMode) {
      pushToast({
        tone: "warning",
        title: "Finish the calculation-ready revision",
        detail: "This advanced-method draft cannot be written as an ordinary project. Return to Calculate and choose Save and activate project.",
      });
      return false;
    }
    const lifecycleBlocker = nativeLegacyProjectOperationBlocker(currentState, "schema5_save");
    if (lifecycleBlocker) {
      pushToast({ tone: "warning", title: "Schema-6 save required", detail: lifecycleBlocker });
      return false;
    }
    const nativeDesktop = isNativeDesktop();
    const shouldPersistModel = currentState.nodes.length > 0 || currentState.activeModelId !== null;
    const modelId = shouldPersistModel
      ? currentState.activeModelId ?? crypto.randomUUID()
      : null;
    const artifacts = modelId ? currentModelArtifacts(currentState, modelId) : null;
    const model = artifacts?.model ?? null;
    const modelPresentation = artifacts?.presentation ?? null;
    const workspace = currentWorkspaceSnapshot(modelId, nativeDesktop);
    const savedInput = currentProjectSignatureInput(currentState);
    if (!nativeDesktop) {
      const projectModels = model
        ? [...currentState.projectModels.filter((candidate) => candidate.id !== model.id), model]
        : currentState.projectModels;
      const modelPresentations = model && modelPresentation
        ? { ...currentState.modelPresentations, [model.id]: modelPresentation }
        : currentState.modelPresentations;
      setProjectExplorer({
        projectModels,
        activeModelId: modelId,
        modelPresentations,
        savedReports: currentState.savedReports,
        explorerSelection: modelId ? { kind: "model", modelId } : currentState.explorerSelection,
      });
      download(
        "quickpls-project.qpls.json",
        JSON.stringify({ schemaVersion: 1, dataset: currentState.dataset, models: projectModels, modelPresentations, savedReports: currentState.savedReports, ...workspace }, null, 2),
        "application/json",
      );
      markWorkspaceCleanAt(nativeProjectSignature({
        ...savedInput,
        projectModels,
        activeModelId: modelId,
        modelPresentations,
      }));
      return true;
    }
    const destination = saveAs || !projectWritableRef.current ? null : projectPathRef.current;
    const saved = await saveNativeProject(destination, workspace, model, modelPresentation);
    if (!saved) return false;
    const canonical = reconcileNativeCanonicalProject(saved);
    const authoritativeSavedSignature = nativeSavedProjectSignature(
      savedInput,
      saved.datasets,
      saved.datasetVersions,
      workspace.activeDatasetId ?? savedInput.dataset.id,
      {
        projectModels: canonical.projectModels,
        activeModelId: canonical.activeModelId,
        modelPresentations: canonical.modelPresentations,
        savedReports: canonical.savedReports,
      },
    );
    setProjectMeta(saved.name, saved.path, saved.projectId);
    setDatasetCatalog(saved.datasets, saved.datasetVersions);
    setProjectExplorer({
      projectModels: canonical.projectModels,
      activeModelId: canonical.activeModelId,
      modelPresentations: canonical.modelPresentations,
      savedReports: canonical.savedReports,
      explorerSelection: canonical.explorerSelection,
    });
    updateProjectWritable(!saved.readOnly);
    markWorkspaceCleanAt(authoritativeSavedSignature);
    pushToast({ tone: "success", title: "Project saved", detail: saved.path ?? saved.name });
    if (saved.saveWarning) {
      pushToast({ tone: "warning", title: "Saved with cleanup warning", detail: saved.saveWarning });
    }
    return true;
  };

  saveProjectRef.current = saveProject;

  const importData = async (request: NativeDataImportRequest) => {
    if (!isNativeDesktop()) {
      pendingWebImportRef.current = { ...request, missingMarkers: [...request.missingMarkers] };
      dataInputRef.current?.click();
      return;
    }
    const imported = await importNativeDataset(request.dataKind, request.sampleSize, request.missingMarkers);
    if (imported) {
      setDataset(imported);
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "data" } }));
      pushToast({
        tone: "success",
        title: "Dataset imported",
        detail: imported.name + ": " + (imported.rowCount ?? imported.rows.length) + " rows, " + imported.columns.length + " variables",
      });
    }
  };

  const applyBrowserExplorerMutation = (mutation: NativeProjectExplorerMutation) => {
    const state = useWorkspace.getState();
    switch (mutation.kind) {
      case "create_model": {
        const modelId = crypto.randomUUID();
        const model = buildNativeRecipeModel(modelId, mutation.name, [], []);
        setProjectExplorer({
          projectModels: [...state.projectModels, model],
          activeModelId: state.activeModelId,
          modelPresentations: {
            ...state.modelPresentations,
            [modelId]: currentNativeModelPresentation([], [], defaultDiagramLayout([], [])),
          },
          savedReports: state.savedReports,
          explorerSelection: { kind: "model", modelId },
        });
        switchProjectModel(modelId);
        break;
      }
      case "activate_model":
        if (!switchProjectModel(mutation.modelId)) throw new Error("The selected model is no longer available.");
        break;
      case "rename_model":
        setProjectExplorer({
          projectModels: state.projectModels.map((model) => model.id === mutation.modelId ? { ...model, name: mutation.name } : model),
          activeModelId: state.activeModelId,
          modelPresentations: state.modelPresentations,
          savedReports: state.savedReports,
          explorerSelection: { kind: "model", modelId: mutation.modelId },
        });
        break;
      case "delete_model": {
        const projectModels = state.projectModels.filter((model) => model.id !== mutation.modelId);
        const modelPresentations = Object.fromEntries(Object.entries(state.modelPresentations)
          .filter(([modelId]) => modelId !== mutation.modelId));
        const deletingActiveModel = state.activeModelId === mutation.modelId;
        const nextModelId = deletingActiveModel ? projectModels[0]?.id ?? null : state.activeModelId;
        setProjectExplorer({
          projectModels,
          activeModelId: deletingActiveModel ? null : nextModelId,
          modelPresentations,
          savedReports: state.savedReports,
          explorerSelection: nextModelId ? { kind: "model", modelId: nextModelId } : { kind: "models" },
        });
        if (deletingActiveModel && nextModelId) {
          switchProjectModel(nextModelId);
        } else if (deletingActiveModel) {
          useWorkspace.setState({
            nodes: [],
            edges: [],
            diagramLayout: defaultDiagramLayout([], []),
            selectedNodeId: null,
            selectedEdgeId: null,
            past: [],
            future: [],
          });
        }
        break;
      }
      case "save_report":
        setProjectExplorer({
          projectModels: state.projectModels,
          activeModelId: state.activeModelId,
          modelPresentations: state.modelPresentations,
          savedReports: [
            ...state.savedReports.filter((report) => report.resultId !== mutation.resultId),
            { resultId: mutation.resultId, name: mutation.name, savedAt: new Date().toISOString() },
          ],
          explorerSelection: { kind: "report", resultId: mutation.resultId },
        });
        break;
      case "rename_report":
        setProjectExplorer({
          projectModels: state.projectModels,
          activeModelId: state.activeModelId,
          modelPresentations: state.modelPresentations,
          savedReports: state.savedReports.map((report) => report.resultId === mutation.resultId ? { ...report, name: mutation.name } : report),
          explorerSelection: { kind: "report", resultId: mutation.resultId },
        });
        break;
      case "remove_report":
        setProjectExplorer({
          projectModels: state.projectModels,
          activeModelId: state.activeModelId,
          modelPresentations: state.modelPresentations,
          savedReports: state.savedReports.filter((report) => report.resultId !== mutation.resultId),
          explorerSelection: { kind: "reports" },
        });
        break;
    }
  };

  const mutateProjectExplorer = async (mutation: NativeProjectExplorerMutation) => {
    if (projectNameRef.current === OPEN_PROJECT_PLACEHOLDER) {
      pushToast({ tone: "warning", title: "Open a project", detail: "Create or open a project before changing its models or saved reports." });
      return false;
    }
    if (calculationActiveRef.current) {
      pushToast({ tone: "warning", title: "Calculation in progress", detail: "Finish or cancel the calculation before changing the project structure." });
      return false;
    }
    if (!projectWritableRef.current) {
      pushToast({ tone: "warning", title: "Read-only project", detail: "Save a writable copy before changing models or saved reports." });
      return false;
    }

    const serializationBlocker = useWorkspace.getState()
      .standardSemModelV4OperationBlocker("legacy_graph_serialization");
    if (serializationBlocker) {
      pushToast({ tone: "warning", title: "Schema-6 workflow required", detail: serializationBlocker });
      return false;
    }

    if (!isNativeDesktop()) {
      applyBrowserExplorerMutation(mutation);
    } else {
      const state = useWorkspace.getState();
      const editableActiveModelId = state.activeModelId
        && state.projectModels.some((model) => model.id === state.activeModelId)
        ? state.activeModelId
        : null;
      const artifacts = editableActiveModelId
        ? currentModelArtifacts(state, editableActiveModelId)
        : null;
      const updated = await mutateNativeProjectExplorer({
        mutation,
        currentModel: artifacts?.model ?? null,
        currentPresentation: artifacts?.presentation ?? null,
        path: state.projectPath,
      });
      if (!loadNativeSnapshot(updated, "Project updated", { announce: false, navigate: false, establishBaseline: false })) return false;
    }

    const nextState = useWorkspace.getState();
    const nextSignature = currentProjectSignature();
    dirtyRef.current = hasUnsavedNativeProjectChanges(true, cleanSignature, nextSignature, {
      active: strictAuthorityActive,
      dirty: strictAuthorityDirty,
      operationPending: authorityOperationPendingRef.current,
    });
    projectSignatureRef.current = nextSignature;
    const affectedModelId = "modelId" in mutation ? mutation.modelId : nextState.activeModelId;
    const affectedResultId = "resultId" in mutation ? mutation.resultId : null;
    if (mutation.kind === "create_model" || mutation.kind === "activate_model") {
      const modelId = nextState.activeModelId;
      if (modelId) setExplorerSelection({ kind: "model", modelId });
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "model" } }));
    } else if (mutation.kind === "delete_model") {
      if (nextState.activeModelId) setExplorerSelection({ kind: "model", modelId: nextState.activeModelId });
      else setExplorerSelection({ kind: "models" });
    } else if (mutation.kind === "rename_model" && affectedModelId) {
      setExplorerSelection({ kind: "model", modelId: affectedModelId });
    } else if ((mutation.kind === "save_report" || mutation.kind === "rename_report") && affectedResultId) {
      setExplorerSelection({ kind: "report", resultId: affectedResultId });
      if (nextState.runs.some((run) => run.id === affectedResultId && run.result)) setSelectedResultRun(affectedResultId);
    } else if (mutation.kind === "remove_report") {
      setExplorerSelection({ kind: "reports" });
    }
    pushToast({
      tone: "success",
      title: mutation.kind.includes("report") ? "Saved reports updated" : "Models updated",
    });
    return true;
  };

  const runAnalysis = async (request: NativeCalculationRequest) => {
    calculationCancellationRequestedRef.current = false;
    const lifecycleBlocker = nativeLegacyProjectOperationBlocker(useWorkspace.getState(), "calculation");
    if (lifecycleBlocker) {
      transitionRunMonitor({
        status: "blocked",
        phase: "Blocked",
        message: lifecycleBlocker,
        completedUnits: 0,
        totalUnits: 0,
        startedAt: null,
        completedAt: null,
        activeJobId: null,
        error: lifecycleBlocker,
      }, { phase: "Blocked", message: lifecycleBlocker, tone: "warning" });
      return;
    }
    const submittedSettings = request.settings;
    const readiness = nativePlsReadiness({ dataset, nodes, edges, settings: submittedSettings, nativeDesktop: isNativeDesktop() });
    const logisticDispatch = request.kind === "regression" && submittedSettings.regressionType === "logistic";
    const logisticAssessment = logisticDispatch
      ? nativeLogisticReadiness(dataset, submittedSettings, request.logisticProfile ?? null)
      : null;
    const logisticDispatchError = !logisticDispatch
      ? null
      : !request.logisticProfile
        ? "Return to Calculate and profile every dataset row before starting binary logistic regression."
        : !logisticAssessment?.canRun || logisticAssessment.profileRequired
          ? logisticAssessment?.blockers[0] ?? logisticAssessment?.detail ?? "The verified binary logistic profile is no longer current."
          : null;
    const processDispatch = request.kind === "regression" && submittedSettings.regressionType === "process";
    const processAssessment = processDispatch
      ? nativeProcessReadiness(dataset, submittedSettings, request.processProfile ?? null)
      : null;
    const processDispatchError = !processDispatch
      ? null
      : !request.processProfile
        ? "Return to Calculate and profile every dataset row before starting graph-defined path analysis."
        : !processAssessment?.canRun || processAssessment.profileRequired
          ? processAssessment?.blockers[0] ?? processAssessment?.detail ?? "The verified PROCESS data profile is no longer current."
          : null;
    if (!readiness.canRun || logisticDispatchError || processDispatchError) {
      const message = logisticDispatchError ?? processDispatchError ?? readiness.blockers[0]?.detail ?? readiness.summary;
      transitionRunMonitor({
        status: "blocked",
        phase: "Blocked",
        message,
        completedUnits: 0,
        totalUnits: 0,
        startedAt: null,
        completedAt: null,
        activeJobId: null,
        error: message,
      }, { phase: "Blocked", message, tone: "warning" });
      return;
    }

    const standalone = isStandaloneNativeAnalysis(request.kind);
    const modelSnapshot = standalone ? undefined : createAnalysisModelSnapshot(nodes, edges, diagramLayout);
    const regressionMethodName = request.kind === "regression"
      ? submittedSettings.regressionType === "logistic"
        ? "Binary Logistic Regression"
        : submittedSettings.regressionType === "process"
          ? "Graph-Defined Path Analysis"
          : nativeAnalysisRecipeDescriptor(request.kind).label
      : nativeAnalysisRecipeDescriptor(request.kind).label;
    const methodName = request.kind === "regression" && submittedSettings.regressionBootstrap === true
      ? `${regressionMethodName} with Bootstrap`
      : regressionMethodName;
    const startedAt = new Date().toISOString();
    transitionRunMonitor({
      status: "queued",
      phase: "Queued",
      message: methodName + " is waiting for preflight checks.",
      completedUnits: 0,
      totalUnits: 5,
      startedAt,
      completedAt: null,
      activeJobId: null,
      lastRunId: null,
      error: null,
    }, { phase: "Queued", message: methodName + " requested.", tone: "info" });

    if (!dataset.fingerprint) throw new Error("Import a fingerprinted dataset before calculating.");

    transitionRunMonitor({
      status: "validating",
      phase: "Checking",
      message: "Checking the dataset, model, method requirements, and calculation settings.",
      completedUnits: 1,
      totalUnits: 5,
    }, { phase: "Checking", message: "Frontend readiness checks passed.", tone: "success" });

    const recipeId = crypto.randomUUID();
    const currentState = useWorkspace.getState();
    const transientModelId = crypto.randomUUID();
    const modelId = standalone ? transientModelId : (currentState.activeModelId ?? transientModelId);
    const modelName = standalone ? `${methodName} (standalone)` : currentModelName(currentState, modelId);
    if (!standalone) {
      const currentModel = buildNativeRecipeModel(modelId, modelName, nodes, edges);
      setProjectExplorer({
        projectModels: [...currentState.projectModels.filter((model) => model.id !== modelId), currentModel],
        activeModelId: modelId,
        modelPresentations: {
          ...currentState.modelPresentations,
          [modelId]: currentNativeModelPresentation(nodes, edges, diagramLayout),
        },
        savedReports: currentState.savedReports,
        explorerSelection: { kind: "model", modelId },
      });
    }
    const recipe = buildNativeAnalysisRecipe({
      kind: request.kind,
      recipeId,
      modelId,
      createdAt: startedAt,
      datasetFingerprint: dataset.fingerprint,
      projectName: modelName,
      nodes: standalone ? [] : nodes,
      edges: standalone ? [] : edges,
      settings: submittedSettings,
    });

    let job = await startNativePlsJob(recipe);
    if (calculationCancellationRequestedRef.current) {
      job = await cancelNativePlsJob(job.id);
    }
    setActiveJob(job);
    transitionRunMonitor({
      status: job.state === "cancelling"
        ? "cancelling"
        : job.state === "queued"
          ? "queued"
          : "running",
      phase: job.phase || "Engine",
      message: job.message ?? "Native engine accepted the calculation job.",
      completedUnits: job.completed_units,
      totalUnits: job.total_units,
      activeJobId: job.id,
    }, { phase: job.phase || "Engine", message: "Native calculation job started.", tone: "info" });

    while (!["completed", "failed", "cancelled"].includes(job.state)) {
      await new Promise((resolve) => window.setTimeout(resolve, 150));
      job = await getNativePlsJob(job.id);
      setActiveJob(job);
      const currentMonitorStatus = useWorkspace.getState().runMonitor.status;
      transitionRunMonitor({
        status: currentMonitorStatus === "cancelling"
          ? "cancelling"
          : job.state === "queued"
            ? "queued"
            : "running",
        phase: job.phase || "Engine",
        message: job.message ?? "QuickPLS is calculating.",
        completedUnits: job.completed_units,
        totalUnits: job.total_units,
        activeJobId: job.id,
      });
    }

    if (job.state === "cancelled") {
      await dismissNativePlsJob(job.id);
      setActiveJob(null);
      calculationCancellationRequestedRef.current = false;
      transitionRunMonitor({
        status: "cancelled",
        phase: "Cancelled",
        message: job.message ?? "Calculation cancelled.",
        completedUnits: job.completed_units,
        totalUnits: job.total_units,
        completedAt: new Date().toISOString(),
        activeJobId: null,
        error: null,
      }, { phase: "Cancelled", message: "Calculation cancelled.", tone: "warning" });
      return;
    }

    if (job.state === "failed") throw new Error(job.message ?? "Analysis failed.");

    const envelope = await getNativePlsJobResult(job.id);
    setActiveJob(null);
    calculationCancellationRequestedRef.current = false;
    transitionRunMonitor({
      status: useWorkspace.getState().runMonitor.status,
      activeJobId: null,
    });
    if (!envelope || envelope.payload.kind === "legacy") throw new Error("The completed job did not return a compatible result.");
    if (
      envelope.payload.kind === "pls_sample_size_power_v1"
      || envelope.payload.kind === "pls_sample_size_power_v2"
    ) {
      if (recipe.method_config.kind !== "pls_sample_size_power") {
        throw new Error("The completed power result does not match its typed prospective recipe.");
      }
      const method = "PLS-SEM Sample Size and Power Analysis";
      const completedRun: AnalysisRun = {
        id: envelope.id,
        modelId,
        name: `${method} run`,
        method,
        createdAt: envelope.provenance.completed_at,
        seed: envelope.provenance.seed,
        status: "completed",
        warnings: envelope.diagnostics.filter((item) => item.level === "warning").map((item) => item.message),
        logs: [
          ...useWorkspace.getState().runMonitor.logs,
          {
            id: `run-${envelope.id}-completed`,
            timestamp: envelope.provenance.completed_at,
            phase: "Completed",
            message: `${method} completed successfully.`,
            tone: "success",
          },
        ],
        fingerprint: envelope.provenance.dataset_fingerprint.slice(0, 12),
        ...(modelSnapshot ? { modelSnapshot } : {}),
        plsSampleSizePower: envelope.payload.analysis,
        plsSampleSizePowerRecipe: nativePlsSampleSizePowerRecipeFromCanonical(recipe.method_config, recipe.settings),
        provenance: envelope.provenance,
      };
      addRun(completedRun);
      transitionRunMonitor({
        status: "completed",
        phase: "Completed",
        message: `${method} completed successfully.`,
        completedUnits: job.total_units,
        totalUnits: job.total_units,
        completedAt: envelope.provenance.completed_at,
        activeJobId: null,
        lastRunId: envelope.id,
        error: null,
      }, { phase: "Completed", message: "Completed power run saved.", tone: "success" });
      pushToast({ tone: "success", title: "Calculation completed", detail: method });
      return;
    }
    const { estimation: result, assessment } = envelope.payload;
    const bootstrap = envelope.payload.kind === "pls_pm_v2"
      ? envelope.payload.bootstrap
      : envelope.payload.kind === "pls_pm_v3"
        ? envelope.payload.bootstrap ?? undefined
        : undefined;
    const permutation = envelope.payload.kind === "pls_pm_v3" ? envelope.payload.permutation ?? undefined : undefined;

    const completedRun: AnalysisRun = {
      id: envelope.id,
      modelId: standalone ? null : modelId,
      name: methodName + " run",
      method: methodName,
      createdAt: envelope.provenance.completed_at,
      seed: envelope.provenance.seed,
      status: "completed",
      warnings: envelope.diagnostics.filter((item) => item.level === "warning").map((item) => item.message),
      logs: [
        ...useWorkspace.getState().runMonitor.logs,
        {
          id: `run-${envelope.id}-completed`,
          timestamp: envelope.provenance.completed_at,
          phase: "Completed",
          message: methodName + " completed successfully.",
          tone: "success",
        },
      ],
      fingerprint: envelope.provenance.dataset_fingerprint.slice(0, 12),
      ...(!standalone && modelSnapshot ? { modelSnapshot } : {}),
      result,
      assessment,
      bootstrap,
      permutation,
      provenance: envelope.provenance,
    };
    if (isStructuralPathRandomizationIdentityPresent(recipe, envelope)) {
      const projection = nativeStructuralPathRandomizationProjection(completedRun);
      if (!projection || !nativeStructuralPathRandomizationRecipeMatches(recipe, envelope, projection)) {
        throw new Error("The completed structural path randomization result failed its current scientific contract.");
      }
    }
    addRun(completedRun);
    transitionRunMonitor({
      status: "completed",
      phase: "Completed",
      message: methodName + " completed successfully.",
      completedUnits: job.total_units,
      totalUnits: job.total_units,
      completedAt: envelope.provenance.completed_at,
      activeJobId: null,
      lastRunId: envelope.id,
      error: null,
    }, { phase: "Completed", message: "Completed run saved.", tone: "success" });
    pushToast({ tone: "success", title: "Calculation completed", detail: methodName });
  };

  const executeRun = async (request: NativeCalculationRequest) => {
    try {
      await runAnalysis(request);
    } catch (error) {
      let message = errorMessage(error);
      const jobId = useWorkspace.getState().runMonitor.activeJobId;
      if (jobId) {
        try {
          await dismissNativePlsJob(jobId);
        } catch (cleanupError) {
          message += ` Job cleanup also failed: ${errorMessage(cleanupError)}`;
        }
      }
      setActiveJob(null);
      transitionRunMonitor({
        status: "failed",
        phase: "Failed",
        message,
        completedAt: new Date().toISOString(),
        activeJobId: null,
        error: message,
      }, { phase: "Failed", message, tone: "error" });
      pushToast({ tone: "error", title: "Calculation failed", detail: message });
    }
  };

  const cancelAnalysis = async () => {
    const monitor = useWorkspace.getState().runMonitor;
    if (!isCalculationActive(monitor.status)) return;
    calculationCancellationRequestedRef.current = true;
    const jobId = activeJob?.id ?? monitor.activeJobId;
    transitionRunMonitor({
      status: "cancelling",
      phase: "Cancelling",
      message: "Waiting for the engine to stop safely.",
      activeJobId: jobId,
    }, { phase: "Cancelling", message: "Cancellation requested.", tone: "warning" });
    if (!jobId) return;
    try {
      setActiveJob(await cancelNativePlsJob(jobId));
    } catch (error) {
      const message = errorMessage(error);
      if (useWorkspace.getState().runMonitor.status === "cancelling") {
        transitionRunMonitor({
          status: "running",
          phase: "Running",
          message: "Cancellation could not be requested. The calculation is still being monitored.",
          error: message,
        }, { phase: "Cancellation", message, tone: "warning" });
      }
      throw error;
    }
  };

  useEffect(() => {
    if (initialized.current) return;
    initialized.current = true;
    if (!new URLSearchParams(window.location.search).has("quickpls_smoke")) {
      closeProject();
    }
    updateProjectWritable(true);
    markCurrentWorkspaceClean();
  }, [closeProject]);

  useEffect(() => {
    document.title = projectOpen
      ? `QuickPLS - ${projectName}${isDirty ? " *" : ""}`
      : "QuickPLS";
  }, [isDirty, projectName, projectOpen]);

  useEffect(() => {
    if (isNativeDesktop()) return;
    const onBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!dirtyRef.current && !authorityOperationPendingRef.current && !generalSemPublicationPendingRef.current && !generalSemTransientWorkBlockerRef.current) return;
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => window.removeEventListener("beforeunload", onBeforeUnload);
  }, []);

  useEffect(() => {
    if (
      !isNativeDesktop()
      || !isDirty
      || !projectPath
      || !projectWritable
      || calculationActive
    ) return;

    const scheduledSignature = projectSignature;
    const timeout = window.setTimeout(() => {
      if (
        !dirtyRef.current
        || calculationActiveRef.current
        || projectSignatureRef.current !== scheduledSignature
        || projectPathRef.current !== projectPath
      ) return;
      const state = useWorkspace.getState();
      if (state.generalSemProjectDraftMode) return;
      const lifecycleBlocker = nativeLegacyProjectOperationBlocker(state, "schema5_autosave");
      if (lifecycleBlocker) {
        if (autosaveFailureSignatureRef.current === scheduledSignature) return;
        autosaveFailureSignatureRef.current = scheduledSignature;
        pushToast({ tone: "warning", title: "Recovery save blocked", detail: lifecycleBlocker });
        return;
      }
      const shouldPersistModel = state.nodes.length > 0 || state.activeModelId !== null;
      const modelId = shouldPersistModel
        ? state.activeModelId ?? crypto.randomUUID()
        : null;
      const artifacts = modelId ? currentModelArtifacts(state, modelId) : null;
      const model = artifacts?.model ?? null;
      const modelPresentation = artifacts?.presentation ?? null;
      if (model && modelPresentation && state.activeModelId === null) {
        setProjectExplorer({
          projectModels: [...state.projectModels.filter((candidate) => candidate.id !== model.id), model],
          activeModelId: model.id,
          modelPresentations: { ...state.modelPresentations, [model.id]: modelPresentation },
          savedReports: state.savedReports,
          explorerSelection: { kind: "model", modelId: model.id },
        });
      }
      void autosaveNativeProject(projectPath, currentWorkspaceSnapshot(modelId, true), model, modelPresentation)
        .then(() => {
          autosaveFailureSignatureRef.current = null;
        })
        .catch((error) => {
          if (autosaveFailureSignatureRef.current === scheduledSignature) return;
          autosaveFailureSignatureRef.current = scheduledSignature;
          pushToast({ tone: "error", title: "Recovery save failed", detail: errorMessage(error) });
        });
    }, AUTOSAVE_DEBOUNCE_MS);

    return () => window.clearTimeout(timeout);
  }, [calculationActive, generalSemProjectDraftMode, isDirty, projectPath, projectSignature, projectWritable, pushToast]);

  useEffect(() => {
    if (!isNativeDesktop()) return;
    const appWindow = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void appWindow.onCloseRequested(async (event) => {
      if (closeBypassRef.current) return;
      if (generalSemPublicationPendingRef.current) {
        event.preventDefault();
        pushToast({ tone: "warning", title: "Project publication in progress", detail: "Wait for the calculation-ready project file to finish publishing and validating before closing." });
        return;
      }
      if (generalSemTransientWorkBlockerRef.current) {
        event.preventDefault();
        const temporaryResult = generalSemTransientWorkBlockerRef.current === "temporary_result_pending";
        pushToast({
          tone: "warning",
          title: temporaryResult ? "Advanced result not yet secured" : "Advanced calculation in progress",
          detail: temporaryResult
            ? "Save and strictly reopen the completed result, or dismiss it explicitly, before closing QuickPLS."
            : "Finish or cancel the advanced calculation before closing QuickPLS.",
        });
        return;
      }
      if (authorityOperationPendingRef.current) {
        event.preventDefault();
        pushToast({ tone: "warning", title: "Schema-6 operation in progress", detail: "Wait for Standard activation or validated save-copy to finish before closing." });
        return;
      }
      if (!dirtyRef.current) return;
      event.preventDefault();
      if (closePromptOpenRef.current) return;
      closePromptOpenRef.current = true;
      try {
        const choice = await message(
          `Save changes to ${projectNameRef.current} before closing?`,
          {
            title: "QuickPLS",
            kind: "warning",
            buttons: { yes: "Save", no: "Don't Save", cancel: "Cancel" },
          },
        );
        if (choice === "Cancel") return;
        if (choice === "Yes" || choice === "Save") {
          const saved = await saveProjectRef.current(!projectWritableRef.current);
          if (!saved || dirtyRef.current) return;
        } else if (choice !== "No" && choice !== "Don't Save") {
          return;
        }
        closeBypassRef.current = true;
        await appWindow.destroy();
      } catch (error) {
        pushToast({ tone: "error", title: "Close failed", detail: errorMessage(error) });
      } finally {
        closePromptOpenRef.current = false;
      }
    }).then((stopListening) => {
      if (disposed) {
        stopListening();
      } else {
        unlisten = stopListening;
      }
    }).catch((error) => {
      pushToast({ tone: "error", title: "Close protection unavailable", detail: errorMessage(error) });
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [pushToast]);

  useEffect(() => {
    const onNewProject = (event: Event) => {
      const detail = (event as CustomEvent<{ name?: string; projectMode?: NativeNewProjectMode }>).detail;
      const name = detail?.name?.trim() || "Untitled project";
      const projectMode = detail?.projectMode === "general_sem_v1"
        && Boolean(generalSemWorkspaceProductAccessV1(
          useWorkspace.getState().uiPreferences.experimentalLabsEnabled,
        ))
        ? "general_sem_v1"
        : "standard";
      void createProject(name, projectMode).catch((error) => {
        const message = errorMessage(error);
        pushToast({ tone: "error", title: "Project creation failed", detail: message });
      });
    };
    const onOpenProject = () => { void openProject().catch((error) => pushToast({ tone: "error", title: "Open failed", detail: errorMessage(error) })); };
    const onOpenProjectPath = (event: Event) => {
      const path = (event as CustomEvent<{ path?: string }>).detail?.path?.trim();
      if (path) void openProject(path).catch((error) => pushToast({ tone: "error", title: "Open failed", detail: errorMessage(error) }));
    };
    const onOpenDemo = (event: Event) => {
      const sampleId = (event as CustomEvent<{ sampleId?: NativeSampleProjectId }>).detail?.sampleId;
      void openDemoProject(sampleId).catch((error) => pushToast({ tone: "error", title: "Demo failed", detail: errorMessage(error) }));
    };
    const onSave = () => { void saveProject(false).catch((error) => pushToast({ tone: "error", title: "Save failed", detail: errorMessage(error) })); };
    const onSaveAs = () => { void saveProject(true).catch((error) => pushToast({ tone: "error", title: "Save failed", detail: errorMessage(error) })); };
    const onImport = (event: Event) => {
      const request = normalizeNativeDataImportRequest((event as CustomEvent<unknown>).detail);
      void importData(request).catch((error) => pushToast({ tone: "error", title: "Import failed", detail: errorMessage(error) }));
    };
    const onRun = (event: Event) => {
      const status = useWorkspace.getState().runMonitor.status;
      const request = parseNativeCalculationRequest((event as CustomEvent<unknown>).detail);
      if (!request) {
        pushToast({ tone: "warning", title: "Calculation setup required", detail: "Open Calculate and review a supported method before starting." });
        return;
      }
      if (!activeJob && !isCalculationActive(status)) void executeRun(request);
    };
    const onCancel = () => { void cancelAnalysis().catch((error) => pushToast({ tone: "error", title: "Cancellation failed", detail: errorMessage(error) })); };
    const onMutateProjectExplorer = (event: Event) => {
      const detail = (event as CustomEvent<NativeProjectExplorerMutationEventDetail>).detail;
      if (!detail?.mutation || typeof detail.resolve !== "function" || typeof detail.reject !== "function") return;
      void mutateProjectExplorer(detail.mutation)
        .then((updated) => {
          if (updated) detail.resolve();
          else detail.reject(new Error("The project item could not be updated."));
        })
        .catch(detail.reject);
    };
    const onExplorerSelection = (event: Event) => {
      const selection = (event as CustomEvent<NativeExplorerSelection>).detail;
      if (selection?.kind) setExplorerSelection(selection);
    };
    const onOpenExplorerData = () => {
      setExplorerSelection({ kind: "data" });
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "data" } }));
    };
    const onOpenExplorerModel = (event: Event) => {
      const modelId = (event as CustomEvent<{ modelId?: string }>).detail?.modelId;
      if (!modelId) return;
      if (useWorkspace.getState().activeModelId === modelId) {
        setExplorerSelection({ kind: "model", modelId });
        window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "model" } }));
        return;
      }
      void mutateProjectExplorer({ kind: "activate_model", modelId }).catch((error) => pushToast({ tone: "error", title: "Model switch failed", detail: errorMessage(error) }));
    };
    const onOpenExplorerReport = (event: Event) => {
      const resultId = (event as CustomEvent<{ resultId?: string }>).detail?.resultId;
      if (!resultId || !useWorkspace.getState().runs.some((run) => run.id === resultId && run.result)) {
        pushToast({ tone: "warning", title: "Result unavailable", detail: "The saved report no longer has a completed result to open." });
        return;
      }
      setExplorerSelection({ kind: "report", resultId });
      setSelectedResultRun(resultId);
      window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "results" } }));
    };

    const listeners: Array<[NativeControllerEvent, EventListener]> = [
      ["quickpls:new-project", onNewProject as EventListener],
      ["quickpls:open-project", onOpenProject],
      ["quickpls:open-project-path", onOpenProjectPath as EventListener],
      ["quickpls:open-demo-project", onOpenDemo],
      ["quickpls:save-project", onSave],
      ["quickpls:save-project-as", onSaveAs],
      ["quickpls:import-data", onImport as EventListener],
      ["quickpls:run-analysis", onRun as EventListener],
      ["quickpls:cancel-analysis", onCancel],
      ["quickpls:mutate-project-explorer", onMutateProjectExplorer as EventListener],
      ["quickpls:set-explorer-selection", onExplorerSelection as EventListener],
      ["quickpls:open-explorer-data", onOpenExplorerData],
      ["quickpls:open-explorer-model", onOpenExplorerModel as EventListener],
      ["quickpls:open-explorer-report", onOpenExplorerReport as EventListener],
    ];
    listeners.forEach(([name, listener]) => window.addEventListener(name, listener));
    return () => listeners.forEach(([name, listener]) => window.removeEventListener(name, listener));
  });

  const importWebProject = async (file?: File) => {
    if (!file) return;
    const project = JSON.parse(await file.text()) as {
      schemaVersion: number;
      nodes: ProjectWorkspaceSnapshot["nodes"];
      edges: ProjectWorkspaceSnapshot["edges"];
      dataset: Dataset;
      runs?: ProjectWorkspaceSnapshot["runs"];
      analysisSettings?: AnalysisUiSettings;
      models?: NonNullable<NativeProjectSnapshot["models"]>;
      activeModelId?: string | null;
      modelPresentations?: NonNullable<NativeProjectSnapshot["modelPresentations"]>;
      savedReports?: NonNullable<NativeProjectSnapshot["savedReports"]>;
    };
    if (project.schemaVersion !== 1 || !Array.isArray(project.nodes) || !Array.isArray(project.edges)) throw new Error("Unsupported QuickPLS project.");
    const importedProjectName = file.name.replace(/\.(qpls\.)?json$/i, "");
    const modelId = project.activeModelId
      ?? project.models?.[0]?.id
      ?? (project.nodes.length ? crypto.randomUUID() : null);
    const projectModels = project.models?.length
      ? project.models
      : modelId
        ? [buildNativeRecipeModel(modelId, importedProjectName, project.nodes, project.edges)]
        : [];
    const modelPresentations = modelId
      ? {
          ...(project.modelPresentations ?? {}),
          [modelId]: project.modelPresentations?.[modelId]
            ?? currentNativeModelPresentation(project.nodes, project.edges, defaultDiagramLayout(project.nodes, project.edges)),
        }
      : {};
    loadProject({
      ...project,
      projectModels,
      activeModelId: modelId,
      modelPresentations,
      savedReports: project.savedReports ?? [],
      explorerSelection: modelId ? { kind: "model", modelId } : { kind: "data" },
    });
    setProjectMeta(importedProjectName, null);
    updateProjectWritable(true);
    markCurrentWorkspaceClean();
    window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: project.nodes.length ? "model" : "data" } }));
  };

  const importWebData = (file?: File) => {
    if (!file) return;
    const request = pendingWebImportRef.current;
    const missingMarkers = new Set(request.missingMarkers);
    Papa.parse<Record<string, string | number | null>>(file, {
      header: true,
      dynamicTyping: true,
      skipEmptyLines: true,
      complete: ({ data, meta }) => {
        const rows = data.map((row) => Object.fromEntries(Object.entries(row).map(([column, value]) => [
          column,
          value == null || missingMarkers.has(String(value).trim()) ? null : value,
        ])) as Dataset["rows"][number]);
        const columns = meta.fields ?? [];
        const missingByColumn = Object.fromEntries(columns.map((column) => [
          column,
          rows.reduce((count, row) => count + (row[column] == null ? 1 : 0), 0),
        ]));
        const missing = Object.values(missingByColumn).reduce((sum, count) => sum + count, 0);
        setDataset({
          id: crypto.randomUUID(),
          name: file.name,
          columns,
          rows,
          rowCount: rows.length,
          missing,
          missingByColumn,
          kind: request.dataKind,
          sampleSize: request.dataKind === "raw" ? null : request.sampleSize,
        });
        window.dispatchEvent(new CustomEvent("quickpls:navigate-surface", { detail: { surface: "data" } }));
        pushToast({ tone: "success", title: "Dataset imported", detail: `${file.name}: ${rows.length} rows, ${columns.length} variables` });
      },
      error: (error) => pushToast({ tone: "error", title: "Import failed", detail: error.message }),
    });
  };

  return <>
    <input ref={projectInputRef} hidden type="file" accept=".json,.qpls" onChange={(event) => { void importWebProject(event.target.files?.[0]); event.currentTarget.value = ""; }} />
    <input ref={dataInputRef} hidden type="file" accept=".csv,.tsv,.txt" onChange={(event) => { importWebData(event.target.files?.[0]); event.currentTarget.value = ""; }} />
  </>;
}
