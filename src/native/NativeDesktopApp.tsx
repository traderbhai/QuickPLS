import {
  Calculator,
  Check,
  ChevronRight,
  Circle,
  Database,
  FileDown,
  FilePlus2,
  FileText,
  FolderOpen,
  GitBranch,
  Hand,
  LayoutGrid,
  Maximize2,
  MousePointer2,
  PanelRightClose,
  PanelRightOpen,
  Replace,
  Save,
  Search,
  Square,
  TableProperties,
  UsersRound,
  X,
} from "lucide-react";
import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useLayoutEffect,
  useRef,
  useState,
  type DragEvent,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";
import { ModelCanvas, type ModelCanvasContextMenuRequest } from "../components/ModelCanvas";
import { NativeDataImportDialog } from "./NativeDataImportDialog";
import { NativeDatasetTransformDialog } from "./NativeDatasetTransformDialog";
import { NativeRecodeDialog } from "./NativeRecodeDialog";
import { NativeDataSurface } from "./NativeDataSurface";
import { NativeModelInspector } from "./NativeModelInspector";
import { NativeRecipeV4CbsemWorkspace } from "./NativeRecipeV4CbsemWorkspace";
import { NativeSemParameterTable } from "./NativeSemParameterTable";
import NativeWorkspaceExplorer, {
  NativeWorkspaceExplorerDialog,
  type NativeExplorerDialog,
} from "./NativeWorkspaceExplorer";
import { nextNativeWorkspaceModelName } from "./nativeWorkspaceTree";
import { canAddNativeModeration, nativeModerationCreationError } from "./nativeModeration";
import { canCreateNativeHigherOrder } from "./nativeHigherOrder";
import {
  nativeDataContextSelection,
  type NativeDataContextMenuRequest,
  type NativeDataContextTarget,
} from "./nativeDataContext";
import { NativeDesktopController } from "./NativeDesktopController";
import {
  NATIVE_ANALYSIS_CATALOG,
  isNativeEstablishedWorkingAnalysisKindV1,
  nativeCapabilitySettingsForWorkbenchKindV2,
  nativeAnalysisSettingsForWorkbenchKind,
  nativeWorkbenchAnalysisKindForSettings,
  type NativeWorkbenchAnalysisKind,
} from "./nativeAnalysisCatalog";
import { createNativeCalculationRequest } from "./nativeCalculationRequest";
import type { NativeLogisticProfile } from "./nativeLogistic";
import type { NativeProcessProfile } from "./nativeProcess";
import { nativeCalculationPhaseLabel } from "./nativeCalculationLifecycle";
import { nativeRunSettingApplicability } from "./nativeExportTables";
import type { NativeDataImportRequest } from "./nativeDataImport";
import {
  formatNativeShortcut,
  isNativeCalculationActive,
  nativeCommandForShortcut,
  nativeContextMenuCommands,
  nativeCommandsFor,
  type NativeCommandAction,
  type NativeCommandContext,
  type NativeMenuId,
  type NativeSurface,
} from "./nativeCommands";
import {
  contextMenuCoordinates,
  isContextMenuKeyboardGesture,
  nextEnabledItemIndex,
  nextMenuIndex,
} from "./nativeMenuNavigation";
import {
  buildNativeResultNavigation,
  completedResultRuns,
  nativeCbsemResultProjection,
  nativeGscaResultProjection,
  nativeNcaCeilingLabel,
  nativeNcaResultProjection,
  nativePcaComponentRuleLabel,
  nativePcaResultProjection,
  nativeOlsResultProjection,
  resolveSelectedCompletedRun,
  resultTableForItem,
  type NativeResultNavigation,
} from "./nativeResults";
import {
  loadNativeRecentProjects,
  rememberNativeRecentProject,
  storeNativeRecentProjects,
  type NativeRecentProject,
} from "./nativeRecentProjects";
import { nativePlsReadiness, type NativePlsReadiness } from "./nativePlsReadiness";
import {
  applyNativeDatasetTransformation,
  getNativeCapabilityRegistryV2,
  isNativeDesktop,
  previewNativeDatasetTransformation,
  recodeNativeDatasetColumn,
} from "../services/projectService";
import type { DatasetTransformationSpecV2 } from "../domain/datasetTransformationsV2";
import { methodCapabilityAvailabilityV2 } from "../domain/methodCapabilityRegistryV2";
import type { StandardSemModelV4AuthorityRecordV1, StandardSemModelV4EditorIntentV1 } from "../domain/standardSemModelV4Authority";
import { compareUtf8StringsV1, type SemVariableV4 } from "../domain/semModelV4";
import { useWorkspace, type StandardSemModelV4AuthorityCommitResult } from "../store";
import type {
  AnalysisRun,
  AnalysisUiSettings,
  Dataset,
  NativeCanonicalModelSpec,
  NativeExplorerSelection,
  NativeProjectExplorerMutation,
  NativeProjectExplorerMutationEventDetail,
  NativeSampleProjectId,
  RecodeColumnSpec,
  RunMonitorStatus,
} from "../types";
import "./nativeDesktop.css";
import "./nativeCanvas.css";

export type { NativeSurface } from "./nativeCommands";
type NativeDialog = "new-project" | "import-data" | "recode-data" | "derive-variable" | "group-setup" | "higher-order" | "moderation" | "calculation" | "export" | "trust" | "settings" | "run-details" | "shortcuts" | "about" | null;

export function completedRunNavigationTarget(
  status: RunMonitorStatus,
  lastRunId: string | null,
  lastNavigatedCompletedRunId: string | null,
): string | null {
  return status === "completed" && lastRunId && lastRunId !== lastNavigatedCompletedRunId
    ? lastRunId
    : null;
}
declare global {
  interface Window {
    __QUICKPLS_SMOKE__?: {
      loadEmptyProject: () => void;
      loadNcaFixture: () => { variables: number; models: number };
      loadPcaFixture: () => { variables: number; models: number };
      loadOlsFixture: () => { variables: number; models: number };
      loadProcessV2Fixture: () => { variables: number; models: number };
      loadHocFixture: () => { variables: number; models: number };
      loadDiagramFixture: (fixture: string) => unknown | Promise<unknown>;
      modelCounts: () => { constructs: number; indicators: number };
      modelPreflight: () => { canRun: boolean; ready: number; blockers: number; warnings: number };
      setView: (nextView: string) => void;
    };
  }
}

interface DesktopCommand {
  id: string;
  label: string;
  icon?: typeof Save;
  shortcut?: string;
  disabled?: boolean;
  pressed?: boolean;
  primary?: boolean;
  action: () => void;
}

interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
  action: () => void;
}

interface NativeContextMenuState {
  x: number;
  y: number;
  returnFocus: HTMLElement | null;
  selection: NativeCommandContext["selection"];
  target?: NativeDataContextTarget;
  canAddModeration?: boolean;
}

const nativeCommandIcons: Record<string, typeof Save> = {
  "new-project": FilePlus2,
  "open-project": FolderOpen,
  "import-data": FolderOpen,
  "recode-variable": Replace,
  "configure-groups": UsersRound,
  "save-project": Save,
  "new-model": FilePlus2,
  "save-report": Save,
  "go-model": LayoutGrid,
  "go-result-data": Database,
  "select-tool": MousePointer2,
  "pan-tool": Hand,
  "add-construct": Circle,
  "path-tool": GitBranch,
  "add-higher-order": GitBranch,
  "arrange-model": LayoutGrid,
  "fit-model": Maximize2,
  "open-calculation": Calculator,
  "export-results": FileDown,
  "run-details": FileText,
};

const menuGroups: ReadonlyArray<readonly [string, NativeMenuId]> = [["File", "file"], ["Edit", "edit"], ["View", "view"], ["Calculate", "calculate"], ["Tools", "tools"], ["Help", "help"]];
const COMPACT_PANE_MEDIA_QUERY = "(max-width: 1100px)";
const NativeResultsSurface = lazy(() => import("./NativeResultsSurface"));
const NativeCalculationDialog = lazy(() => import("./NativeCalculationDialog"));
const NativeGroupSetupDialog = lazy(() => import("./NativeGroupSetupDialog"));
const NativeHigherOrderDialog = lazy(() => import("./NativeHigherOrderDialog"));
const NativeModerationDialog = lazy(() => import("./NativeModerationDialog"));
const NativeExportDialog = lazy(() => import("./NativeExportDialog"));
const NativeUtilityDialog = lazy(() => import("./NativeUtilityDialog"));

export const NATIVE_BUNDLED_SAMPLE_PROJECTS = [
  {
    id: "corporate_reputation",
    label: "Corporate reputation",
    detail: "Four constructs and a completed bootstrap-backed PLS-SEM run.",
  },
  {
    id: "simple_pls",
    label: "Simple reflective PLS-SEM",
    detail: "Two reflective constructs and a completed PLS algorithm run.",
  },
  {
    id: "mediation",
    label: "Mediation",
    detail: "Three constructs with completed direct, indirect, and total effects.",
  },
] as const satisfies ReadonlyArray<{
  id: NativeSampleProjectId;
  label: string;
  detail: string;
}>;


function commandEvent(name: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:${name}`, { detail }));
}

export function openNativeSampleProject(sampleId: NativeSampleProjectId) {
  commandEvent("open-demo-project", { sampleId });
}

function requestProjectExplorerMutation(mutation: NativeProjectExplorerMutation): Promise<void> {
  return new Promise((resolve, reject) => {
    const detail: NativeProjectExplorerMutationEventDetail = { mutation, resolve, reject };
    window.dispatchEvent(new CustomEvent("quickpls:mutate-project-explorer", { detail }));
  });
}

function explorerCommandSelection(selection: NativeExplorerSelection): NativeCommandContext["selection"] {
  switch (selection.kind) {
    case "project": return { kind: "none", count: 0 };
    case "data": return { kind: "project-data", count: 1 };
    case "models": return { kind: "project-models", count: 1 };
    case "model": return { kind: "project-model", count: 1 };
    case "reports": return { kind: "project-reports", count: 1 };
    case "report": return { kind: "project-report", count: 1 };
  }
}

function navigationWithPrecision(navigation: NativeResultNavigation, digits: number): NativeResultNavigation {
  const precision = Math.min(6, Math.max(2, Math.trunc(digits)));
  if (precision === 6) return navigation;
  return {
    ...navigation,
    tables: navigation.tables.map((table) => ({
      ...table,
      rows: table.rows.map((row) => row.map((cell) => /^-?\d+\.\d{6}$/.test(cell) ? Number(cell).toFixed(precision) : cell)),
    })),
  };
}


export function NativeDesktopApp() {
  const projectName = useWorkspace((state) => state.projectName);
  const projectPath = useWorkspace((state) => state.projectPath);
  const dataset = useWorkspace((state) => state.dataset);
  const datasetDescriptorOnly = useWorkspace((state) => state.datasetDescriptorOnly);
  const projectWritable = useWorkspace((state) => state.projectWritable);
  const projectModels = useWorkspace((state) => state.projectModels);
  const strictAuthorities = useWorkspace((state) => state.standardSemModelV4Authorities);
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const savedReports = useWorkspace((state) => state.savedReports);
  const explorerSelection = useWorkspace((state) => state.explorerSelection);
  const setExplorerSelection = useWorkspace((state) => state.setExplorerSelection);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const diagramTool = useWorkspace((state) => state.diagramTool);
  const past = useWorkspace((state) => state.past);
  const future = useWorkspace((state) => state.future);
  const runMonitor = useWorkspace((state) => state.runMonitor);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const setAnalysisSettings = useWorkspace((state) => state.setAnalysisSettings);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const toasts = useWorkspace((state) => state.toasts);
  const dismissToast = useWorkspace((state) => state.dismissToast);
  const resetRunMonitor = useWorkspace((state) => state.resetRunMonitor);
  const addRun = useWorkspace((state) => state.addRun);
  const commitDatasetVersion = useWorkspace((state) => state.commitDatasetVersion);
  const addTwoStageInteraction = useWorkspace((state) => state.addTwoStageInteraction);
  const addHigherOrderConstruct = useWorkspace((state) => state.addHigherOrderConstruct);
  const pushToast = useWorkspace((state) => state.pushToast);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const commitStandardIntent = useWorkspace((state) => state.commitStandardSemModelV4Intent);
  const loadProject = useWorkspace((state) => state.loadProject);
  const setProjectMeta = useWorkspace((state) => state.setProjectMeta);
  const setSelectedResultRun = useWorkspace((state) => state.setSelectedResultRun);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const strictIntentCounter = useRef(0);
  const nextStrictIntentId = (kind: string) => `standard:editor:${kind}:${Date.now()}:${++strictIntentCounter.current}`;
  const commitStrictDesktopIntent = (intent: StandardSemModelV4EditorIntentV1, label: string) => {
    void commitStandardIntent(intent).then((result) => {
      if (result.status === "committed") pushToast({ tone: "success", title: `${label} committed`, detail: "The strict Standard model authority accepted the edit." });
      else if (result.status === "blocked") pushToast({ tone: "error", title: `${label} blocked`, detail: `${result.diagnostic.message} ${result.diagnostic.correctiveAction}` });
      else if (result.status === "stale") pushToast({ tone: "warning", title: `${label} stale`, detail: "The active authority changed; review the current model and retry." });
      else pushToast({ tone: "error", title: `${label} rejected`, detail: result.error instanceof Error ? result.error.message : String(result.error) });
    });
  };
  const [surface, setSurface] = useState<NativeSurface>("launcher");
  const [dialog, setDialog] = useState<NativeDialog>(null);
  const [explorerDialog, setExplorerDialog] = useState<NativeExplorerDialog | null>(null);
  const explorerDialogReturnFocusRef = useRef<HTMLElement | null>(null);
  const currentDialogRef = useRef<NativeDialog>(null);
  const dialogScopeRef = useRef(0);
  const [dialogScope, setDialogScope] = useState(0);
  const recodeBusyRef = useRef(false);
  const [recodeBusy, setRecodeBusy] = useState(false);
  const deriveBusyRef = useRef(false);
  const [deriveBusy, setDeriveBusy] = useState(false);
  const [recodeSourceColumn, setRecodeSourceColumn] = useState("");
  const [groupSetupColumn, setGroupSetupColumn] = useState("");
  const dialogReturnFocusRef = useRef<HTMLElement | null>(null);
  const restoreDialogFocus = useCallback(() => {
    window.setTimeout(() => {
      const previous = dialogReturnFocusRef.current;
      const fallback = document.getElementById("nd-main");
      (previous?.isConnected ? previous : fallback)?.focus();
      dialogReturnFocusRef.current = null;
    }, 0);
  }, []);
  const openExplorerDialog = useCallback((next: NativeExplorerDialog) => {
    explorerDialogReturnFocusRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : document.getElementById("nd-main");
    setExplorerDialog(next);
  }, []);
  const closeExplorerDialog = useCallback(() => {
    setExplorerDialog(null);
    window.setTimeout(() => {
      const previous = explorerDialogReturnFocusRef.current;
      const fallback = document.getElementById("nd-main");
      (previous?.isConnected ? previous : fallback)?.focus();
      explorerDialogReturnFocusRef.current = null;
    }, 0);
  }, []);
  const openDialog = useCallback((next: Exclude<NativeDialog, null>) => {
    if ((currentDialogRef.current === "recode-data" && recodeBusyRef.current)
      || (currentDialogRef.current === "derive-variable" && deriveBusyRef.current)) return;
    const active = document.activeElement;
    dialogReturnFocusRef.current = active instanceof HTMLElement ? active : document.getElementById("nd-main");
    const nextScope = dialogScopeRef.current + 1;
    dialogScopeRef.current = nextScope;
    currentDialogRef.current = next;
    recodeBusyRef.current = false;
    deriveBusyRef.current = false;
    setRecodeBusy(false);
    setDeriveBusy(false);
    setDialogScope(nextScope);
    setDialog(next);
  }, []);
  const closeDialog = useCallback(() => {
    if ((currentDialogRef.current === "recode-data" && recodeBusyRef.current)
      || (currentDialogRef.current === "derive-variable" && deriveBusyRef.current)) return;
    const closingCalculation = currentDialogRef.current === "calculation";
    currentDialogRef.current = null;
    setDialog(null);
    if (closingCalculation && !isNativeCalculationActive(useWorkspace.getState().runMonitor.status)) resetRunMonitor();
    restoreDialogFocus();
  }, [resetRunMonitor, restoreDialogFocus]);
  const setScopedRecodeBusy = useCallback((scope: number, busy: boolean) => {
    if (scope !== dialogScopeRef.current || currentDialogRef.current !== "recode-data") return;
    recodeBusyRef.current = busy;
    setRecodeBusy(busy);
  }, []);
  const completeRecodeDialog = useCallback((scope: number) => {
    if (scope !== dialogScopeRef.current || currentDialogRef.current !== "recode-data") return;
    recodeBusyRef.current = false;
    setRecodeBusy(false);
    currentDialogRef.current = null;
    setDialog(null);
    restoreDialogFocus();
  }, [restoreDialogFocus]);
  const setScopedDeriveBusy = useCallback((scope: number, busy: boolean) => {
    if (scope !== dialogScopeRef.current || currentDialogRef.current !== "derive-variable") return;
    deriveBusyRef.current = busy;
    setDeriveBusy(busy);
  }, []);
  const completeDeriveDialog = useCallback((scope: number) => {
    if (scope !== dialogScopeRef.current || currentDialogRef.current !== "derive-variable") return;
    deriveBusyRef.current = false;
    setDeriveBusy(false);
    currentDialogRef.current = null;
    setDialog(null);
    restoreDialogFocus();
  }, [restoreDialogFocus]);
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<NativeContextMenuState | null>(null);
  const closeContextMenu = useCallback(() => setContextMenu(null), []);
  const [propertiesOpen, setPropertiesOpen] = useState(() =>
    typeof window === "undefined" || !window.matchMedia(COMPACT_PANE_MEDIA_QUERY).matches,
  );
  const [selectedColumn, setSelectedColumn] = useState(dataset.columns[0] ?? "");
  const [selectedTableId, setSelectedTableId] = useState("model_estimates");
  const [calculationKind, setCalculationKind] = useState<NativeWorkbenchAnalysisKind>(() => nativeWorkbenchAnalysisKindForSettings(analysisSettings));
  const [calculationDraft, setCalculationDraft] = useState<AnalysisUiSettings>(() => ({ ...analysisSettings }));
  const [nativeRegistryVerification, setNativeRegistryVerification] = useState<"browser" | "pending" | "verified" | "failed">(
    () => isNativeDesktop() ? "pending" : "browser",
  );
  const [newProjectName, setNewProjectName] = useState("Untitled project");
  const lastNavigatedCompletedRunId = useRef<string | null>(null);
  const [recentProjects, setRecentProjects] = useState<NativeRecentProject[]>(() => loadNativeRecentProjects(window.localStorage));
  const smokeSeeded = useRef(false);

  useEffect(() => {
    if (!isNativeDesktop()) return;
    let active = true;
    void getNativeCapabilityRegistryV2()
      .then(() => { if (active) setNativeRegistryVerification("verified"); })
      .catch(() => { if (active) setNativeRegistryVerification("failed"); });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    const compactLayout = window.matchMedia(COMPACT_PANE_MEDIA_QUERY);
    const collapseProperties = (event: MediaQueryListEvent) => {
      if (event.matches) setPropertiesOpen(false);
    };

    compactLayout.addEventListener("change", collapseProperties);
    return () => compactLayout.removeEventListener("change", collapseProperties);
  }, []);

  const completedRuns = useMemo(() => completedResultRuns(runs), [runs]);
  const selectedRun = useMemo(
    () => resolveSelectedCompletedRun(completedRuns, selectedResultRunId),
    [completedRuns, selectedResultRunId],
  );
  const selectedResultSaved = useMemo(
    () => Boolean(selectedRun && savedReports.some((report) => report.resultId === selectedRun.id)),
    [savedReports, selectedRun],
  );
  const activeEditableModelName = useMemo(
    () => strictAuthorities[activeModelId ?? ""]?.model.name
      ?? projectModels.find((model) => model.id === activeModelId)?.name
      ?? "Model",
    [activeModelId, projectModels, strictAuthorities],
  );
  const explorerModels = useMemo(() => [
    ...projectModels,
    ...Object.values(strictAuthorities)
      .filter((authority) => !projectModels.some((model) => model.id === authority.model.id))
      .map((authority) => ({
        id: authority.model.id,
        name: authority.model.name,
      } as NativeCanonicalModelSpec)),
  ], [projectModels, strictAuthorities]);
  const canOpenContextModel = useMemo(() => {
    const modelId = surface === "results" ? selectedRun?.modelId : activeModelId;
    return Boolean(modelId && (
      projectModels.some((model) => model.id === modelId)
      || strictAuthorities[modelId]
    ));
  }, [activeModelId, projectModels, selectedRun?.modelId, strictAuthorities, surface]);
  const selectedRunId = selectedRun?.id ?? "";
  const resultNavigation = useMemo(() => navigationWithPrecision(buildNativeResultNavigation(selectedRun), uiPreferences.defaultPrecision), [selectedRun, uiPreferences.defaultPrecision]);
  const resultTables = resultNavigation.tables;
  const selectedResultItem = resultNavigation.groups.flatMap((group) => group.items).find((item) => item.id === selectedTableId) ?? resultNavigation.groups[0]?.items[0];
  const selectedTable = selectedResultItem ? resultTableForItem(resultNavigation, selectedResultItem.id) : undefined;
  const modelReadiness = useMemo(
    () => nativePlsReadiness({ dataset, nodes, edges, settings: nativeAnalysisSettingsForWorkbenchKind(analysisSettings, "pls_algorithm"), nativeDesktop: isNativeDesktop() }),
    [analysisSettings, dataset, edges, nodes],
  );
  const calculationSettings = useMemo(
    () => nativeAnalysisSettingsForWorkbenchKind(calculationDraft, calculationKind),
    [calculationDraft, calculationKind],
  );
  const calculationAnalysisColumns = useMemo(
    () => [...new Set(nodes.flatMap((node) => node.data.indicators))].sort(),
    [nodes],
  );
  const calculationReadiness = useMemo(
    () => nativePlsReadiness({ dataset, nodes, edges, settings: calculationSettings, nativeDesktop: isNativeDesktop() }),
    [calculationSettings, dataset, edges, nodes],
  );
  const commandContext = useMemo<NativeCommandContext>(() => {
    const selectedConstructs = new Set([
      ...nodes.filter((node) => node.selected).map((node) => node.id),
      ...(selectedNodeId ? [selectedNodeId] : []),
    ]);
    const selectionCount = selectedConstructs.size + (selectedEdgeId ? 1 : 0);
    const selection = surface === "launcher" && projectName !== "No project open"
      ? explorerCommandSelection(explorerSelection)
      : surface === "model"
      ? { kind: selectionCount > 1 ? "multiple" as const : selectedEdgeId ? "path" as const : selectionCount ? "construct" as const : "none" as const, count: selectionCount }
      : surface === "data"
        ? { kind: selectedColumn ? "variable" as const : dataset.columns.length ? "dataset" as const : "none" as const, count: selectedColumn || dataset.columns.length ? 1 : 0 }
        : surface === "results" && selectedRun
          ? { kind: "result" as const, count: 1 }
          : { kind: "none" as const, count: 0 };
    return {
      surface,
      projectOpen: projectName !== "No project open",
      projectWritable,
      hasDataset: dataset.columns.length > 0,
      hasCompletedRun: completedRuns.length > 0,
      selectedResultSaved,
      canOpenContextModel,
      canCalculate: modelReadiness.canRun && !strictAuthority,
      canUndo: past.length > 0,
      canRedo: future.length > 0,
      canRecode: Boolean(selectedColumn) && projectWritable && (dataset.kind ?? "raw") === "raw",
      canConfigureGroups: Boolean(selectedColumn)
        && (dataset.kind ?? "raw") === "raw"
        && !nodes.some((node) => node.data.indicators.includes(selectedColumn)),
      selectedVariableIsGrouping: Boolean(selectedColumn && selectedColumn === analysisSettings.groupColumn?.trim()),
      canAddModeration: Boolean(selectedEdgeId) && canAddNativeModeration(nodes, edges, selectedEdgeId),
      canAddHigherOrder: canCreateNativeHigherOrder(nodes, edges),
      propertiesOpen,
      selection,
      calculationStatus: runMonitor.status,
    };
  }, [analysisSettings.groupColumn, canOpenContextModel, completedRuns.length, dataset.columns.length, dataset.kind, explorerSelection, future.length, modelReadiness.canRun, nodes, past.length, projectName, projectWritable, propertiesOpen, runMonitor.status, selectedColumn, selectedEdgeId, selectedNodeId, selectedResultSaved, selectedRun, strictAuthority, surface]);

  const dataMutationsLocked = datasetDescriptorOnly || isNativeCalculationActive(runMonitor.status);

  const navigate = useCallback((next: NativeSurface) => {
    setSurface(next);
    setOpenMenu(null);
    setContextMenu(null);
  }, []);

  useEffect(() => {
    const onNavigate = (event: Event) => {
      const next = (event as CustomEvent<{ surface?: NativeSurface }>).detail?.surface;
      if (next && ["launcher", "data", "model", "results"].includes(next)) navigate(next);
    };
    window.addEventListener("quickpls:navigate-surface", onNavigate);
    return () => window.removeEventListener("quickpls:navigate-surface", onNavigate);

  }, [navigate]);
  const openCalculation = () => {
    if (!["queued", "validating", "running", "cancelling"].includes(runMonitor.status)) {
      resetRunMonitor();
    }
    const preferredKind = surface === "data" ? "nca" : nativeWorkbenchAnalysisKindForSettings(analysisSettings);
    setCalculationDraft(nativeAnalysisSettingsForWorkbenchKind(analysisSettings, preferredKind));
    setCalculationKind(preferredKind);
    openDialog("calculation");
  };
  useEffect(() => {
    if (!projectPath || projectName === "No project open") return;
    setRecentProjects((current) => {
      const next = rememberNativeRecentProject(current, { name: projectName, path: projectPath, openedAt: new Date().toISOString() });
      storeNativeRecentProjects(window.localStorage, next);
      return next;
    });
  }, [projectName, projectPath]);


  useEffect(() => {
    if (!selectedColumn || !dataset.columns.includes(selectedColumn)) setSelectedColumn(dataset.columns[0] ?? "");
  }, [dataset.columns, selectedColumn]);

  useEffect(() => {
    const resolvedRunId = selectedRun?.id ?? null;
    if (selectedResultRunId !== resolvedRunId) setSelectedResultRun(resolvedRunId);
  }, [selectedResultRunId, selectedRun?.id, setSelectedResultRun]);

  useEffect(() => {
    setSelectedTableId(resultNavigation.defaultItemId ?? "");
  }, [resultNavigation.defaultItemId, resultNavigation.runId]);

  useEffect(() => {
    const completedRunId = completedRunNavigationTarget(
      runMonitor.status,
      runMonitor.lastRunId,
      lastNavigatedCompletedRunId.current,
    );
    if (completedRunId) {
      lastNavigatedCompletedRunId.current = completedRunId;
      currentDialogRef.current = null;
      setDialog(null);
      navigate("results");
    }
  }, [navigate, runMonitor.lastRunId, runMonitor.status]);

  useEffect(() => {
    if (dialog || explorerDialog) {
      setOpenMenu(null);
      setContextMenu(null);
    }
  }, [dialog, explorerDialog]);

  useEffect(() => {
    if (!openMenu) return;
    const closeFromPointer = (event: PointerEvent) => {
      if (!(event.target instanceof Element) || !event.target.closest(".nd-menu")) setOpenMenu(null);
    };
    document.addEventListener("pointerdown", closeFromPointer);
    return () => document.removeEventListener("pointerdown", closeFromPointer);
  }, [openMenu]);

  useEffect(() => {
    if (!new URLSearchParams(window.location.search).has("quickpls_smoke")) return;
    if (new URLSearchParams(window.location.search).get("quickpls_smoke") === "completed" && completedRuns.length === 0 && !smokeSeeded.current) {
      smokeSeeded.current = true;
      void import("../data/smokeRun").then(({ completedSamplePlsRun }) => addRun(completedSamplePlsRun()));
    }
    const smoke = {
      loadEmptyProject: () => {
        loadProject({ nodes: [], edges: [], dataset: { id: "empty", name: "No dataset", columns: [], rows: [], missing: 0, rowCount: 0 }, runs: [], diagramMode: "sem" });
        setProjectMeta("Untitled project", null);
        navigate("launcher");
      },
      loadNcaFixture: () => {
        const rows = [
          { condition: 1, outcome: 1.0 },
          { condition: 2, outcome: 1.4 },
          { condition: 3, outcome: 2.2 },
          { condition: 4, outcome: 3.1 },
          { condition: 5, outcome: 4.6 },
          { condition: 6, outcome: 6.4 },
          { condition: 7, outcome: 7.1 },
          { condition: 8, outcome: 8.5 },
        ];
        loadProject({
          nodes: [],
          edges: [],
          dataset: {
            id: "native-nca-smoke",
            name: "NCA observed-variable fixture",
            columns: ["condition", "outcome"],
            rows,
            rowCount: rows.length,
            missing: 0,
            fingerprint: "sha256:native-nca-smoke-v1",
            kind: "raw",
            columnMetadata: ["condition", "outcome"].map((name) => ({
              name,
              label: null,
              column_type: "numeric" as const,
              scale_type: "continuous" as const,
              missing_markers: [],
              theoretical_min: null,
              theoretical_max: null,
              value_labels: {},
            })),
          },
          projectModels: [],
          activeModelId: null,
          runs: [],
          diagramMode: "sem",
        });
        setProjectMeta("NCA standalone acceptance fixture", null);
        navigate("data");
        return { variables: 2, models: 0 };
      },
      loadPcaFixture: () => {
        const rows = Array.from({ length: 24 }, (_, index) => ({
          service: index + 1,
          quality: (index + 1) * 1.2 + (index % 3) * 0.2,
          value: 30 - index + (index % 4) * 0.15,
          trust: (index % 7) * 1.5 + index * 0.3,
          segment: index % 2 ? "B" : "A",
        }));
        const columns = ["service", "quality", "value", "trust", "segment"];
        loadProject({
          nodes: [],
          edges: [],
          dataset: {
            id: "native-pca-smoke",
            name: "PCA numeric-variable fixture",
            columns,
            rows,
            rowCount: rows.length,
            missing: 0,
            fingerprint: "sha256:native-pca-smoke-v1",
            kind: "raw",
            columnMetadata: columns.map((name) => ({
              name,
              label: null,
              column_type: name === "segment" ? "text" as const : "numeric" as const,
              scale_type: name === "segment" ? "nominal" as const : "continuous" as const,
              missing_markers: [],
              theoretical_min: null,
              theoretical_max: null,
              value_labels: {},
            })),
          },
          projectModels: [],
          activeModelId: null,
          runs: [],
          diagramMode: "sem",
        });
        setProjectMeta("PCA standalone acceptance fixture", null);
        navigate("data");
        return { variables: 5, models: 0 };
      },
      loadOlsFixture: () => {
        const rows = Array.from({ length: 36 }, (_, index) => {
          const x = index + 1;
          const m = (index % 6) - 2.5;
          const control = (index % 4) - 1.5;
          return {
            outcome: 1.75 + 1.4 * x - 0.65 * m + 0.3 * control + ((index % 5) - 2) * 0.08,
            predictor: x,
            moderator: m,
            control,
            group: index % 2 ? "B" : "A",
          };
        });
        const columns = ["outcome", "predictor", "moderator", "control", "group"];
        loadProject({
          nodes: [],
          edges: [],
          dataset: {
            id: "native-ols-smoke",
            name: "OLS numeric-variable fixture",
            columns,
            rows,
            rowCount: rows.length,
            missing: 0,
            fingerprint: "sha256:native-ols-smoke-v1",
            kind: "raw",
            columnMetadata: columns.map((name) => ({
              name,
              label: null,
              column_type: name === "group" ? "text" as const : "numeric" as const,
              scale_type: name === "group" ? "nominal" as const : "continuous" as const,
              missing_markers: [],
              theoretical_min: null,
              theoretical_max: null,
              value_labels: {},
            })),
          },
          projectModels: [],
          activeModelId: null,
          runs: [],
          diagramMode: "sem",
        });
        setProjectMeta("OLS standalone acceptance fixture", null);
        navigate("data");
        return { variables: 5, models: 0 };
      },
      loadProcessV2Fixture: () => {
        const columns = ["X", "M1", "M2", "M3", "M4", "W", "B", "C", "Y"];
        const rows = Array.from({ length: 64 }, (_, index) => {
          const x = (index - 31.5) / 8;
          const w = ((index * 5) % 17 - 8) / 4;
          const b = index % 2;
          const c = ((index * 3) % 13 - 6) / 5;
          const m1 = 0.55 * x + 0.18 * c + ((index % 5) - 2) * 0.04;
          const m2 = 0.62 * m1 + 0.12 * c + ((index % 7) - 3) * 0.035;
          const m3 = 0.45 * x + 0.28 * w + 0.32 * x * w + 0.15 * c + ((index % 4) - 1.5) * 0.05;
          const m4 = 0.50 * x + 0.12 * c + ((index % 6) - 2.5) * 0.045;
          const y = 0.25 * x + 0.42 * m2 + 0.36 * m3 + 0.33 * m4 + 0.16 * w + 0.11 * b
            + 0.21 * x * w + 0.13 * x * b + 0.10 * w * b + 0.18 * x * w * b
            + 0.24 * m4 * b + 0.14 * c + ((index % 9) - 4) * 0.03;
          return {
            X: x,
            M1: m1,
            M2: index === 7 ? null : m2,
            M3: m3,
            M4: m4,
            W: index === 29 ? null : w,
            B: b,
            C: c,
            Y: y,
          };
        });
        const missingByColumn = Object.fromEntries(columns.map((column) => [
          column,
          rows.filter((row) => row[column as keyof typeof row] == null).length,
        ]));
        loadProject({
          nodes: [],
          edges: [],
          dataset: {
            id: "native-process-v2-smoke",
            name: "PROCESS v2 graph-defined fixture",
            columns,
            rows,
            rowCount: rows.length,
            missing: Object.values(missingByColumn).reduce((sum, count) => sum + count, 0),
            missingByColumn,
            fingerprint: "sha256:native-process-v2-smoke-v1",
            kind: "raw",
            columnMetadata: columns.map((name) => ({
              name,
              label: null,
              column_type: "numeric" as const,
              scale_type: name === "B" ? "binary" as const : "continuous" as const,
              missing_markers: [],
              theoretical_min: null,
              theoretical_max: null,
              value_labels: (name === "B" ? { "0": "Class 0", "1": "Class 1" } : {}) as Record<string, string>,
            })),
          },
          projectModels: [],
          activeModelId: null,
          runs: [],
          diagramMode: "sem",
        });
        setProjectMeta("PROCESS v2 standalone acceptance fixture", null);
        navigate("data");
        return { variables: 9, models: 0 };
      },
      loadHocFixture: () => {
        const rows = Array.from({ length: 36 }, (_, index) => {
          const capability = (index % 9) - 4;
          const resources = Math.floor(index / 6) - 2.5;
          return {
            capability,
            resources,
            performance: 0.55 * capability + 0.45 * resources + ((index % 4) - 1.5) * 0.04,
          };
        });
        const columns = ["capability", "resources", "performance"];
        loadProject({
          nodes: [],
          edges: [],
          dataset: {
            id: "native-hoc-smoke",
            name: "Disjoint two-stage HOC fixture",
            columns,
            rows,
            rowCount: rows.length,
            missing: 0,
            fingerprint: "sha256:native-hoc-smoke-v1",
            kind: "raw",
            columnMetadata: columns.map((name) => ({
              name,
              label: null,
              column_type: "numeric" as const,
              scale_type: "continuous" as const,
              missing_markers: [],
              theoretical_min: null,
              theoretical_max: null,
              value_labels: {},
            })),
          },
          projectModels: [],
          activeModelId: null,
          runs: [],
          diagramMode: "sem",
        });
        setProjectMeta("Higher-order construct acceptance fixture", null);
        navigate("data");
        return { variables: 3, models: 0 };
      },
      loadDiagramFixture: async (fixture: string) => {
        if (fixture !== "large" && fixture !== "large-stress") return undefined;
        const { largeModelSmokeProject } = await import("../data/largeModelSmoke");
        const profile = fixture === "large-stress" ? "stress" : "applied";
        const project = largeModelSmokeProject(profile);
        loadProject({ ...project, runs: [], diagramMode: "sem" });
        setProjectMeta(profile === "stress" ? "Stress model acceptance fixture" : "Large model acceptance fixture", null);
        navigate("model");
        return { constructs: project.nodes.length, indicators: project.dataset.columns.length };
      },
      modelCounts: () => {
        const state = useWorkspace.getState();
        return {
          constructs: state.nodes.length,
          indicators: state.nodes.reduce((total, node) => total + node.data.indicators.length, 0),
        };
      },
      modelPreflight: () => {
        const state = useWorkspace.getState();
        const readiness = nativePlsReadiness({
          dataset: state.dataset,
          nodes: state.nodes,
          edges: state.edges,
          settings: nativeAnalysisSettingsForWorkbenchKind(state.analysisSettings, "pls_algorithm"),
          nativeDesktop: isNativeDesktop(),
        });
        return {
          canRun: readiness.canRun,
          ready: readiness.items.filter((item) => item.status === "ready").length,
          blockers: readiness.blockers.length,
          warnings: readiness.warnings.length,
        };
      },
      setView: (view: string) => navigate(view === "welcome" || view === "home" ? "launcher" : view === "models" || view === "model" ? "model" : view === "runs" || view === "results" ? "results" : "data"),
    };
    window.__QUICKPLS_SMOKE__ = smoke;
    return () => { delete window.__QUICKPLS_SMOKE__; };
  }, [addRun, completedRuns.length, loadProject, navigate, setProjectMeta]);

  const startCalculation = (dataProfile?: NativeLogisticProfile | NativeProcessProfile) => {
    if (!calculationReadiness.canRun || ["queued", "validating", "running", "cancelling"].includes(runMonitor.status)) return;
    setAnalysisSettings(calculationSettings);
    commandEvent("run-analysis", createNativeCalculationRequest(calculationKind, calculationSettings, dataProfile));
  };

  const createProject = () => {
    const name = newProjectName.trim() || "Untitled project";
    commandEvent("new-project", { name });
    closeDialog();
  };

  const beginDataImport = (request: NativeDataImportRequest) => {
    commandEvent("import-data", request);
    closeDialog();
  };

  const createRecode = async (spec: RecodeColumnSpec) => {
    if (!isNativeDesktop()) throw new Error("Recode is available only in the installed Windows app.");
    const mutation = await recodeNativeDatasetColumn(dataset.id, spec);
    commitDatasetVersion(mutation);
    setSelectedColumn(mutation.version.targetColumn ?? spec.targetColumn);
    pushToast({ tone: "success", title: "Recoded variable created", detail: mutation.version.summary });
  };

  const previewDerivedVariable = async (spec: DatasetTransformationSpecV2) => {
    if (!isNativeDesktop()) throw new Error("Derived variables are available only in the installed Windows app.");
    return previewNativeDatasetTransformation(dataset.id, spec);
  };

  const createDerivedVariable = async (spec: DatasetTransformationSpecV2, outputDatasetName: string) => {
    if (!isNativeDesktop()) throw new Error("Derived variables are available only in the installed Windows app.");
    const mutation = await applyNativeDatasetTransformation(dataset.id, spec, outputDatasetName);
    commitDatasetVersion(mutation);
    const firstTarget = spec.kind === "missing_markers" ? spec.columns[0]?.target_column : spec.target_column;
    setSelectedColumn(mutation.version.targetColumn ?? firstTarget ?? null);
    pushToast({ tone: "success", title: "Derived dataset created", detail: mutation.version.summary });
    return mutation;
  };

  const dispatchNativeAction = (action: NativeCommandAction, target?: NativeDataContextTarget) => {
    switch (action.id) {
      case "project.new": openDialog("new-project"); return;
      case "project.open": commandEvent("open-project"); return;
      case "project.open-demo": commandEvent("open-demo-project"); return;
      case "project.import-data": openDialog("import-data"); return;
      case "data.recode": {
        const column = target?.kind === "variable" ? target.column : selectedColumn;
        if (!column) return;
        setSelectedColumn(column);
        setRecodeSourceColumn(column);
        openDialog("recode-data");
        return;
      }
      case "data.configure-groups": {
        const column = target?.kind === "variable"
          ? target.column
          : selectedColumn || analysisSettings.groupColumn?.trim() || "";
        if (!column || nodes.some((node) => node.data.indicators.includes(column))) return;
        setSelectedColumn(column);
        setGroupSetupColumn(column);
        openDialog("group-setup");
        return;
      }
      case "project.save": commandEvent(action.saveAs ? "save-project-as" : "save-project"); return;
      case "explorer.open-selection": {
        if (explorerSelection.kind === "data") commandEvent("open-explorer-data");
        else if (explorerSelection.kind === "model") commandEvent("open-explorer-model", { modelId: explorerSelection.modelId });
        else if (explorerSelection.kind === "report") commandEvent("open-explorer-report", { resultId: explorerSelection.resultId });
        return;
      }
      case "explorer.new-model":
        openExplorerDialog({ kind: "new-model", initialValue: nextNativeWorkspaceModelName(projectModels) });
        return;
      case "explorer.rename-selection": {
        if (explorerSelection.kind === "model") {
          const model = projectModels.find((candidate) => candidate.id === explorerSelection.modelId);
          if (model) openExplorerDialog({ kind: "rename-model", modelId: model.id, initialValue: model.name });
        } else if (explorerSelection.kind === "report") {
          const report = savedReports.find((candidate) => candidate.resultId === explorerSelection.resultId);
          if (report) openExplorerDialog({ kind: "rename-report", resultId: report.resultId, initialValue: report.name });
        }
        return;
      }
      case "explorer.delete-selection": {
        if (explorerSelection.kind === "model") {
          const model = projectModels.find((candidate) => candidate.id === explorerSelection.modelId);
          if (model) openExplorerDialog({ kind: "delete-model", modelId: model.id, name: model.name });
        } else if (explorerSelection.kind === "report") {
          const report = savedReports.find((candidate) => candidate.resultId === explorerSelection.resultId);
          if (report) openExplorerDialog({ kind: "remove-report", resultId: report.resultId, name: report.name });
        }
        return;
      }
      case "explorer.save-report":
        if (selectedRun && !selectedResultSaved) {
          openExplorerDialog({ kind: "save-report", resultId: selectedRun.id, initialValue: selectedRun.name || "Results report" });
        }
        return;
      case "surface.navigate": {
        if (action.surface === "model" && surface === "results") {
          const resultModelId = selectedRun?.modelId;
          if (resultModelId && projectModels.some((model) => model.id === resultModelId)) {
            commandEvent("open-explorer-model", { modelId: resultModelId });
          }
          return;
        }
        navigate(action.surface);
        return;
      }
      case "model.undo": commandEvent("model-undo"); return;
      case "model.redo": commandEvent("model-redo"); return;
      case "model.set-tool": commandEvent("model-tool", { tool: action.tool }); return;
      case "model.add-construct": commandEvent("model-add-construct"); return;
      case "model.add-higher-order": openDialog("higher-order"); return;
      case "model.add-moderating-effect": openDialog("moderation"); return;
      case "model.edit-selection": {
        window.dispatchEvent(new CustomEvent("quickpls:model-inspector-show-editor"));
        const focusModelSelectionEditor = () => {
          const workspace = useWorkspace.getState();
          const editorId = workspace.selectedNodeId
            ? "nd-model-construct-name"
            : workspace.selectedEdgeId
              ? "nd-model-path-label"
              : null;
          const editor = editorId ? document.getElementById(editorId) : null;
          if (!(editor instanceof HTMLInputElement)) return false;
          editor.focus({ preventScroll: true });
          editor.select();
          return true;
        };
        setPropertiesOpen(true);
        // An already-visible Properties pane can be focused during the same
        // keyboard event. Defer only when opening the pane must first mount it.
        if (!focusModelSelectionEditor()) window.setTimeout(focusModelSelectionEditor, 0);
        return;
      }
      case "model.delete-selection": commandEvent("model-delete-selection"); return;
      case "model.arrange": commandEvent("model-arrange", { direction: action.strategy }); return;
      case "model.fit": commandEvent("model-fit"); return;
      case "calculation.open": openCalculation(); return;
      case "calculation.cancel": commandEvent("cancel-analysis"); return;
      case "results.export": openDialog("export"); return;
      case "results.open-run-details": openDialog("run-details"); return;
      case "view.toggle-properties": setPropertiesOpen((value) => !value); return;
      case "utility.open": openDialog(({ "method-scope": "trust", preferences: "settings", shortcuts: "shortcuts", about: "about" } as const)[action.utility]); return;
    }
  };
  const commands: DesktopCommand[] = nativeCommandsFor({ kind: "toolbar", surface }, commandContext).map((command) => ({
    id: command.id,
    label: command.label,
    icon: nativeCommandIcons[command.id],
    disabled: !command.enabled,
    pressed: command.action.id === "model.set-tool" ? diagramTool === command.action.tool : undefined,
    primary: command.toolbar?.some((placement) => placement.surface === surface && placement.primary),
    action: () => dispatchNativeAction(command.action),
  }));

  const menus: Record<string, MenuItem[]> = {};
  for (const [label, menu] of menuGroups) {
    const items = nativeCommandsFor({ kind: "menu", menu }, commandContext).map((command) => ({
      id: command.id,
      label: command.label,
      shortcut: formatNativeShortcut(command.shortcut),
      disabled: !command.enabled,
      separator: command.menu?.separatorBefore,
      action: () => dispatchNativeAction(command.action),
    }));
    if (items.length) menus[label] = items;
  }

  const contextMenuItems: MenuItem[] = nativeContextMenuCommands(
    contextMenu?.canAddModeration === undefined ? commandContext : { ...commandContext, canAddModeration: contextMenu.canAddModeration },
    contextMenu?.selection,
  ).map((command, index) => ({
      id: command.id,
      label: command.label,
      shortcut: formatNativeShortcut(command.shortcut),
      disabled: !command.enabled,
      separator: index > 0 && Boolean(command.contextMenu?.find((placement) => placement.surface === surface)?.separatorBefore),
      action: () => dispatchNativeAction(command.action, contextMenu?.target),
    }));

  const showWorkspaceContextMenu = (
    requestedX: number,
    requestedY: number,
    returnFocus: HTMLElement | null,
    selection: NativeCommandContext["selection"] = commandContext.selection,
    target?: NativeDataContextTarget,
    canAddModerationOverride?: boolean,
  ) => {
    const contextualCommandContext = canAddModerationOverride === undefined
      ? commandContext
      : { ...commandContext, canAddModeration: canAddModerationOverride };
    const availableCommands = nativeContextMenuCommands(contextualCommandContext, selection);
    if (!availableCommands.length) return false;
    const position = contextMenuCoordinates(
      requestedX,
      requestedY,
      window.innerWidth,
      window.innerHeight,
      244,
      Math.min(320, 10 + availableCommands.length * 29),
    );
    setOpenMenu(null);
    setContextMenu({ ...position, returnFocus, selection: { ...selection }, target, canAddModeration: canAddModerationOverride });
    return true;
  };

  const onDataContextMenuRequest = (request: NativeDataContextMenuRequest) => {
    if (request.target.kind === "variable") setSelectedColumn(request.target.column);
    return showWorkspaceContextMenu(
      request.clientX,
      request.clientY,
      request.returnFocus,
      nativeDataContextSelection(request.target),
      request.target,
    );
  };

  const onModelCanvasContextMenuRequest = (request: ModelCanvasContextMenuRequest) => {
    const selection: NativeCommandContext["selection"] = request.target.kind === "canvas"
      ? { kind: "none", count: 0 }
      : request.target.kind === "path"
        ? { kind: "path", count: 1 }
        : { kind: "construct", count: 1 };
    showWorkspaceContextMenu(
      request.clientX,
      request.clientY,
      request.returnFocus,
      selection,
      undefined,
      request.target.kind === "path" ? canAddNativeModeration(nodes, edges, request.target.id) : false,
    );
  };

  const onWorkspaceContextMenu = (event: ReactMouseEvent<HTMLDivElement>) => {
    const target = event.target instanceof Element ? event.target : event.currentTarget;
    if (target.closest("input, textarea, select, [contenteditable='true']")) return;
    const returnFocus = target.closest<HTMLElement>("button, [href], input, select, textarea, [tabindex]")
      ?? (target instanceof HTMLElement ? target : event.currentTarget);
    if (showWorkspaceContextMenu(event.clientX, event.clientY, returnFocus)) event.preventDefault();
  };

  const onWorkspaceKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.defaultPrevented || !isContextMenuKeyboardGesture(event.key, event.shiftKey)) return;
    const target = event.target instanceof Element ? event.target : event.currentTarget;
    if (target.closest("input, textarea, select, [contenteditable='true']")) return;
    const returnFocus = target.closest<HTMLElement>("button, [href], input, select, textarea, [tabindex]")
      ?? (target instanceof HTMLElement ? target : event.currentTarget);
    const bounds = returnFocus.getBoundingClientRect();
    const opened = showWorkspaceContextMenu(
      bounds.left + Math.min(24, Math.max(8, bounds.width / 2)),
      bounds.bottom || bounds.top + 24,
      returnFocus,
    );
    if (!opened) return;
    event.preventDefault();
    event.stopPropagation();
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      if (event.defaultPrevented || openMenu || contextMenu) return;
      const editable = target instanceof HTMLElement && Boolean(target.closest("input, textarea, select, [contenteditable='true']"));
      if (explorerDialog || (dialog && !(dialog === "calculation" && event.key.toLowerCase() === "escape"))) return;
      const command = nativeCommandForShortcut({ key: event.key, ctrlKey: event.ctrlKey, shiftKey: event.shiftKey, altKey: event.altKey, metaKey: event.metaKey, editable }, commandContext);
      if (!command) return;
      event.preventDefault();
      dispatchNativeAction(command.action);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [commandContext, contextMenu, dialog, explorerDialog, openMenu]);

  return <div className="nd-app" data-native-desktop-shell="true" data-surface={surface} data-density={uiPreferences.density} data-table-density={uiPreferences.tableDensity}>
    <NativeDesktopController />
    <a className="nd-skip-link" href="#nd-main">Skip to workspace</a>
    <MenuBar menus={menus} openMenu={openMenu} setOpenMenu={setOpenMenu} projectName={projectName} />
    <CommandBar commands={commands} surface={surface} projectName={projectName} projectPath={projectPath} modelName={activeEditableModelName} propertiesOpen={propertiesOpen} setPropertiesOpen={setPropertiesOpen} />
    <div id="nd-main" className="nd-workspace" role="main" tabIndex={-1} onContextMenu={onWorkspaceContextMenu} onKeyDown={onWorkspaceKeyDown}>
      {surface === "launcher" ? projectName !== "No project open" ? <NativeWorkspaceExplorer
        projectName={projectName}
        projectPath={projectPath}
        projectWritable={projectWritable}
        datasetName={dataset.name}
        datasetRows={dataset.rowCount ?? dataset.rows.length}
        datasetColumns={dataset.columns.length}
        models={explorerModels}
        activeModelId={activeModelId}
        reports={savedReports}
        selection={explorerSelection}
        currentResultId={selectedRun?.id ?? null}
        currentResultName={selectedRun?.name}
        currentResultSaved={selectedResultSaved}
        calculationStatus={runMonitor.status}
        onSelectionChange={setExplorerSelection}
        onOpenData={() => commandEvent("open-explorer-data")}
        onOpenModel={(modelId) => commandEvent("open-explorer-model", { modelId })}
        onOpenReport={(resultId) => commandEvent("open-explorer-report", { resultId })}
        onCreateModel={(name) => requestProjectExplorerMutation({ kind: "create_model", name })}
        onRenameModel={(modelId, name) => requestProjectExplorerMutation({ kind: "rename_model", modelId, name })}
        onDeleteModel={(modelId) => requestProjectExplorerMutation({ kind: "delete_model", modelId })}
        onSaveReport={(resultId, name) => requestProjectExplorerMutation({ kind: "save_report", resultId, name })}
        onRenameReport={(resultId, name) => requestProjectExplorerMutation({ kind: "rename_report", resultId, name })}
        onRemoveReport={(resultId) => requestProjectExplorerMutation({ kind: "remove_report", resultId })}
      /> : <Launcher projectName={projectName} projectPath={projectPath} datasetName={dataset.name} runs={completedRuns} recentProjects={recentProjects} onNavigate={navigate} onOpenRecent={(path) => commandEvent("open-project-path", { path })} onOpenSample={openNativeSampleProject} /> : null}
      {surface === "data" ? <NativeDataSurface
        selectedColumn={selectedColumn}
        setSelectedColumn={setSelectedColumn}
        groupColumn={analysisSettings.groupColumn ?? null}
        propertiesOpen={propertiesOpen}
        hasEditableModel={projectModels.length > 0}
        projectWritable={projectWritable}
        mutationsLocked={dataMutationsLocked}
        onNewModel={() => dispatchNativeAction({ id: "explorer.new-model" })}
        onAnalyze={() => dispatchNativeAction({ id: "calculation.open" })}
        onDerive={() => openDialog("derive-variable")}
        onContextMenuRequest={onDataContextMenuRequest}
      /> : null}
      {surface === "model" ? <ModelSurface modelName={activeEditableModelName} propertiesOpen={propertiesOpen} readiness={modelReadiness} onContextMenuRequest={onModelCanvasContextMenuRequest} /> : null}
      {surface === "results" ? <Suspense fallback={<ResultsSurfaceLoading propertiesOpen={propertiesOpen} />}><NativeResultsSurface runs={completedRuns} selectedRun={selectedRun} selectedRunId={selectedRunId} setSelectedRunId={setSelectedResultRun} navigation={resultNavigation} selectedItem={selectedResultItem} selectedTable={selectedTable} setSelectedTableId={setSelectedTableId} propertiesOpen={propertiesOpen} openMethodDetails={() => openDialog("trust")} /></Suspense> : null}
    </div>
    {contextMenu ? <ContextCommandMenu items={contextMenuItems} state={contextMenu} close={closeContextMenu} /> : null}
    <NativeToastStack toasts={toasts} dismiss={dismissToast} />
    <StatusBar surface={surface} projectName={projectName} datasetName={dataset.name} cases={dataset.rowCount ?? dataset.rows.length} constructs={nodes.length} runMonitor={runMonitor} />
    {dialog ? <DialogHost dialog={dialog} close={closeDialog} title={dialogTitle(dialog)} dismissible={dialog === "recode-data" ? !recodeBusy : dialog === "derive-variable" ? !deriveBusy : dialog !== "calculation" || !["queued", "validating", "running", "cancelling"].includes(runMonitor.status)}>
      {dialog === "new-project" ? <NewProjectDialog value={newProjectName} setValue={setNewProjectName} close={closeDialog} create={createProject} /> : null}
      {dialog === "import-data" ? <NativeDataImportDialog close={closeDialog} importData={beginDataImport} /> : null}
      {dialog === "recode-data" ? <NativeRecodeDialog
        key={dialogScope}
        dataset={dataset}
        sourceColumn={recodeSourceColumn}
        nativeDesktop={isNativeDesktop()}
        projectWritable={projectWritable}
        dialogScope={dialogScope}
        close={closeDialog}
        complete={completeRecodeDialog}
        onBusyChange={setScopedRecodeBusy}
        recode={createRecode}
      /> : null}
      {dialog === "derive-variable" ? <NativeDatasetTransformDialog
        key={dialogScope}
        dataset={dataset}
        selectedColumn={selectedColumn}
        nativeDesktop={isNativeDesktop()}
        projectWritable={projectWritable}
        mutationsLocked={dataMutationsLocked}
        datasetResident={isNativeDesktop() || (dataset.rowCount ?? dataset.rows.length) === dataset.rows.length}
        dialogScope={dialogScope}
        close={closeDialog}
        complete={completeDeriveDialog}
        onBusyChange={setScopedDeriveBusy}
        previewTransformation={previewDerivedVariable}
        applyTransformation={createDerivedVariable}
      /> : null}
      {dialog === "group-setup" ? <Suspense fallback={<UtilityDialogLoading label="Loading complete dataset groups" />}><NativeGroupSetupDialog
        key={dialogScope}
        dataset={dataset}
        analysisColumns={calculationAnalysisColumns}
        initialColumn={groupSetupColumn}
        settings={analysisSettings}
        nativeDesktop={isNativeDesktop()}
        projectWritable={projectWritable}
        apply={(patch) => {
          if (strictAuthority) {
            const grouping = patch.groupColumn
              ? strictAuthority.model.variables.find((variable) => variable.kind === "observed" && variable.source_column === patch.groupColumn)
              : null;
            if (patch.groupColumn && (!grouping || grouping.kind !== "observed")) {
              pushToast({ tone: "error", title: "Groups rejected", detail: "The selected grouping variable is not present in the strict model authority." });
              return;
            }
            commitStrictDesktopIntent({
              kind: "set_group",
              group: grouping?.kind === "observed"
                ? {
                  kind: "observed_groups",
                  grouping_variable: grouping.id,
                  levels: [patch.groupAValue, patch.groupBValue].flatMap((value, index) => value ? [{ id: `group-${index + 1}`, value, label: value }] : []),
                }
                : { kind: "single_group" },
            }, patch.groupColumn ? "Groups" : "Group clearing");
          } else {
            setAnalysisSettings(patch);
            pushToast({
              tone: "success",
              title: patch.groupColumn ? "Groups configured" : "Groups cleared",
              detail: patch.groupColumn ? `${patch.groupColumn}: ${patch.groupAValue} vs ${patch.groupBValue}` : "No grouping variable is configured.",
            });
          }
        }}
        close={closeDialog}
      /></Suspense> : null}
      {dialog === "higher-order" ? <Suspense fallback={<UtilityDialogLoading label="Opening higher-order construct setup" />}><NativeHigherOrderDialog
        nodes={nodes}
        edges={edges}
        selectedComponentIds={nodes.filter((node) => node.selected || node.id === selectedNodeId).map((node) => node.id)}
        create={(draft) => {
          if (strictAuthority) {
            const termId = nextStrictIntentId("higher-order-term");
            const outputId = nextStrictIntentId("higher-order-output");
            commitStrictDesktopIntent({
              kind: "add_higher_order",
              term_id: termId,
              output_id: outputId,
              label: draft.name,
              components: draft.components,
              approach: "disjoint_two_stage",
              measurement_type: "reflective_reflective",
            }, "Higher-order construct");
            return { status: "created", constructId: outputId };
          }
          const result = addHigherOrderConstruct(draft);
          if (result.status === "created") {
            pushToast({ tone: "success", title: "Higher-order construct created", detail: "Use the Path tool to connect it to a measured outcome, then run PLS-SEM Algorithm." });
          } else {
            pushToast({ tone: "error", title: "Higher-order construct not created", detail: result.detail });
          }
          return result;
        }}
        close={closeDialog}
      /></Suspense> : null}
      {dialog === "moderation" ? <Suspense fallback={<UtilityDialogLoading label="Opening moderating effect setup" />}><NativeModerationDialog
        nodes={nodes}
        edges={edges}
        selectedEdgeId={selectedEdgeId}
        create={(predictor, moderator, outcome) => {
          if (strictAuthority && selectedEdgeId) {
            const termId = nextStrictIntentId("interaction-term");
            const outputId = nextStrictIntentId("interaction-output");
            commitStrictDesktopIntent({
              kind: "add_interaction",
              term_id: termId,
              output_id: outputId,
              label: `${predictor} × ${moderator}`,
              predictor,
              moderator,
              focal_relation: selectedEdgeId,
              outcome,
              method: "two_stage",
            }, "Moderating effect");
            return { status: "created", interactionId: outputId };
          }
          const result = addTwoStageInteraction(predictor, moderator, outcome);
          if (result.status === "created") {
            pushToast({ tone: "success", title: "Moderating effect created", detail: "Two-stage interaction and required main-effect paths were added to the model." });
          } else {
            pushToast({ tone: "error", title: "Moderating effect not created", detail: nativeModerationCreationError(result.reason) });
          }
          return result;
        }}
        close={closeDialog}
      /></Suspense> : null}
      {dialog === "calculation" ? <Suspense fallback={<UtilityDialogLoading label="Opening calculation setup" />}><NativeCalculationDialog
        kind={calculationKind}
        setKind={setCalculationKind}
        settings={calculationSettings}
        setSettings={(patch) => setCalculationDraft((current) => ({ ...current, ...patch }))}
        readiness={calculationReadiness}
        runMonitor={runMonitor}
        dataset={dataset}
        analysisColumns={calculationAnalysisColumns}
        nodes={nodes}
        edges={edges}
        experimentalLabsEnabled={uiPreferences.experimentalLabsEnabled}
        openMethodDetails={() => openDialog("trust")}
        registryUnavailableReason={nativeRegistryVerification === "pending"
          ? "Checking the installed calculation catalogue."
          : nativeRegistryVerification === "failed"
            ? "QuickPLS could not verify the installed calculation catalogue. Restart the app or reinstall this build before calculating."
            : null}
        start={startCalculation}
        cancel={() => commandEvent("cancel-analysis")}
        close={closeDialog}
      /></Suspense> : null}
      {dialog === "export" && selectedRun ? <Suspense fallback={<UtilityDialogLoading label="Preparing export options" />}><NativeExportDialog run={selectedRun} tables={resultTables} close={closeDialog} /></Suspense> : null}
      {dialog === "run-details" && selectedRun ? <RunDetailsDialog run={selectedRun} /> : null}
      {dialog === "trust" || dialog === "settings" ? <Suspense fallback={<UtilityDialogLoading label={dialog === "trust" ? "Opening Method Details" : "Opening preferences"} />}><NativeUtilityDialog kind={dialog} close={closeDialog} run={dialog === "trust" && surface === "results" ? selectedRun : null} /></Suspense> : null}
      {dialog === "shortcuts" ? <ShortcutsDialog /> : null}
      {dialog === "about" ? <AboutDialog settings={analysisSettings} experimentalLabsEnabled={uiPreferences.experimentalLabsEnabled} /> : null}
    </DialogHost> : null}
    {explorerDialog ? <NativeWorkspaceExplorerDialog
      dialog={explorerDialog}
      close={closeExplorerDialog}
      onCreateModel={(name) => requestProjectExplorerMutation({ kind: "create_model", name })}
      onRenameModel={(modelId, name) => requestProjectExplorerMutation({ kind: "rename_model", modelId, name })}
      onDeleteModel={(modelId) => requestProjectExplorerMutation({ kind: "delete_model", modelId })}
      onSaveReport={(resultId, name) => requestProjectExplorerMutation({ kind: "save_report", resultId, name })}
      onRenameReport={(resultId, name) => requestProjectExplorerMutation({ kind: "rename_report", resultId, name })}
      onRemoveReport={(resultId) => requestProjectExplorerMutation({ kind: "remove_report", resultId })}
    /> : null}
  </div>;
}

function MenuBar({ menus, openMenu, setOpenMenu, projectName }: { menus: Record<string, MenuItem[]>; openMenu: string | null; setOpenMenu: (menu: string | null) => void; projectName: string }) {
  const names = Object.keys(menus);
  const triggerRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const [rovingIndex, setRovingIndex] = useState(0);

  useEffect(() => {
    if (rovingIndex >= names.length) setRovingIndex(Math.max(0, names.length - 1));
  }, [names.length, rovingIndex]);

  const popupId = (index: number) => `nd-menu-popup-${index}`;
  const triggerId = (index: number) => `nd-menu-trigger-${index}`;
  const focusPopupItem = (index: number, last: boolean) => {
    window.setTimeout(() => {
      const popup = document.getElementById(popupId(index));
      const enabled = Array.from(popup?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? []);
      (last ? enabled[enabled.length - 1] : enabled[0])?.focus();
    }, 0);
  };
  const restoreTrigger = (index: number) => {
    setOpenMenu(null);
    window.setTimeout(() => triggerRefs.current[index]?.focus(), 0);
  };
  const moveTopLevel = (current: number, direction: -1 | 1) => {
    const next = nextMenuIndex(current, names.length, direction);
    if (next < 0) return;
    setRovingIndex(next);
    if (openMenu) {
      setOpenMenu(names[next]);
      focusPopupItem(next, false);
    } else {
      triggerRefs.current[next]?.focus();
    }
  };
  const onTriggerKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>, index: number) => {
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      moveTopLevel(index, event.key === "ArrowLeft" ? -1 : 1);
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setOpenMenu(names[index]);
      focusPopupItem(index, event.key === "ArrowUp");
      return;
    }
    if (event.key === "Escape" && openMenu) {
      event.preventDefault();
      restoreTrigger(index);
    }
  };
  const onPopupKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>, index: number) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Home" || event.key === "End") {
      event.preventDefault();
      const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'));
      const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = nextEnabledItemIndex(buttons.map((button) => button.disabled), current, event.key);
      buttons[next]?.focus();
      return;
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      moveTopLevel(index, event.key === "ArrowLeft" ? -1 : 1);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      restoreTrigger(index);
      return;
    }
    if (event.key === "Tab") setOpenMenu(null);
  };

  return <div className="nd-menubar-shell">
    <div className="nd-brand" aria-label="QuickPLS"><span>Q</span><strong>QuickPLS</strong></div>
    <nav className="nd-menubar" role="menubar" aria-label="Application menu">
      {Object.entries(menus).map(([name, items], index) => <div className="nd-menu" role="none" key={name} onMouseEnter={() => {
        if (!openMenu || openMenu === name) return;
        setRovingIndex(index);
        setOpenMenu(name);
        focusPopupItem(index, false);
      }}>
        <button
          ref={(node) => { triggerRefs.current[index] = node; }}
          id={triggerId(index)}
          type="button"
          role="menuitem"
          tabIndex={rovingIndex === index ? 0 : -1}
          aria-haspopup="menu"
          aria-controls={popupId(index)}
          aria-expanded={openMenu === name}
          onFocus={() => setRovingIndex(index)}
          onKeyDown={(event) => onTriggerKeyDown(event, index)}
          onClick={() => {
            const opening = openMenu !== name;
            setRovingIndex(index);
            setOpenMenu(opening ? name : null);
            if (opening) focusPopupItem(index, false);
          }}
        >{name}</button>
        {openMenu === name ? <div id={popupId(index)} className="nd-menu-popup" role="menu" aria-labelledby={triggerId(index)} onKeyDown={(event) => onPopupKeyDown(event, index)}>
          {items.map((item) => <button key={item.id} role="menuitem" tabIndex={-1} type="button" className={item.separator ? "separator" : ""} disabled={item.disabled} onClick={() => { setOpenMenu(null); triggerRefs.current[index]?.focus(); item.action(); }}><span>{item.label}</span>{item.shortcut ? <kbd>{item.shortcut}</kbd> : null}</button>)}
        </div> : null}
      </div>)}
    </nav>
    {projectName !== "No project open" ? <span className="nd-window-project">{projectName}</span> : null}
  </div>;
}

function ContextCommandMenu({ items, state, close }: { items: MenuItem[]; state: NativeContextMenuState; close: () => void }) {
  const menuRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')?.focus({ preventScroll: true });
  }, []);
  useEffect(() => {
    const closeFromPointer = (event: PointerEvent) => {
      if (!(event.target instanceof Node) || !menuRef.current?.contains(event.target)) close();
    };
    document.addEventListener("pointerdown", closeFromPointer);
    return () => document.removeEventListener("pointerdown", closeFromPointer);
  }, [close]);

  const focusReturnTarget = () => {
    if (!state.returnFocus?.isConnected) return;
    state.returnFocus.focus({ preventScroll: true });
  };
  const restoreFocus = () => {
    // Focus the exact invoking element before unmounting the menu so callers
    // observing the hidden state cannot race a deferred restoration timer.
    focusReturnTarget();
    close();
  };
  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Home" || event.key === "End") {
      event.preventDefault();
      const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'));
      const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = nextEnabledItemIndex(buttons.map((button) => button.disabled), current, event.key);
      buttons[next]?.focus();
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      restoreFocus();
      return;
    }
    if (event.key === "Tab") close();
  };

  return <div
    ref={menuRef}
    className="nd-context-menu"
    role="menu"
    aria-label="Workspace commands"
    style={{ left: state.x, top: state.y }}
    onKeyDown={onKeyDown}
  >
    {items.map((item) => <button key={item.id} role="menuitem" tabIndex={-1} type="button" className={item.separator ? "separator" : ""} disabled={item.disabled} onClick={() => { focusReturnTarget(); close(); item.action(); }}><span>{item.label}</span>{item.shortcut ? <kbd>{item.shortcut}</kbd> : null}</button>)}
  </div>;
}

function CommandBar({ commands, surface, projectName, projectPath, modelName, propertiesOpen, setPropertiesOpen }: { commands: DesktopCommand[]; surface: NativeSurface; projectName: string; projectPath: string | null; modelName: string; propertiesOpen: boolean; setPropertiesOpen: (value: boolean) => void }) {
  return <div className="nd-commandbar" role="toolbar" aria-label={`${surface} commands`}>
    <div className="nd-document-context"><strong>{surface === "launcher" ? "Project" : surface === "data" ? "Data" : surface === "model" ? modelName : "Results"}</strong>{surface !== "launcher" ? <span>{projectPath ?? projectName}</span> : null}</div>
    <div className="nd-command-list">
      {commands.map((command) => {
        const Icon = command.icon;
        return <button type="button" key={command.id} className={command.primary ? "primary" : ""} disabled={command.disabled} aria-pressed={command.pressed} title={command.label} onClick={command.action}>{Icon ? <Icon size={15} aria-hidden="true" /> : null}<span>{command.label}</span></button>;
      })}
    </div>
    {surface !== "launcher" ? <button className="nd-pane-toggle" type="button" aria-pressed={propertiesOpen} title={propertiesOpen ? "Hide Properties" : "Show Properties"} onClick={() => setPropertiesOpen(!propertiesOpen)}>{propertiesOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />} Properties</button> : null}
  </div>;
}

export function Launcher({ projectName, projectPath, datasetName, runs, recentProjects, onNavigate, onOpenRecent, onOpenSample }: { projectName: string; projectPath: string | null; datasetName: string; runs: AnalysisRun[]; recentProjects: NativeRecentProject[]; onNavigate: (surface: NativeSurface) => void; onOpenRecent: (path: string) => void; onOpenSample: (sampleId: NativeSampleProjectId) => void }) {
  const hasUnsavedProject = projectName !== "No project open" && !projectPath;
  const hasRows = hasUnsavedProject || recentProjects.length > 0;
  return <div className="nd-launcher" aria-label="Project launcher">
    <section className="nd-launch-actions">
      <h1>QuickPLS</h1>
      <p>Structural equation modeling for Windows.</p>
      <section className="nd-sample-projects" aria-labelledby="nd-sample-projects-heading">
        <header>
          <h2 id="nd-sample-projects-heading">Bundled samples</h2>
          <p>Open a complete project with data, model, and a completed result.</p>
        </header>
        <div className="nd-sample-project-list">
          {NATIVE_BUNDLED_SAMPLE_PROJECTS.map((sample) => <button
            key={sample.id}
            type="button"
            data-sample-id={sample.id}
            onClick={() => onOpenSample(sample.id)}
          >
            <span><strong>{sample.label}</strong><small>{sample.detail}</small></span>
            <ChevronRight size={15} aria-hidden="true" />
          </button>)}
        </div>
      </section>
    </section>
    <section className="nd-recent-projects" aria-labelledby="recent-heading">
      <header><h2 id="recent-heading">Recent projects</h2></header>
      {hasUnsavedProject ? <button className="nd-project-row" onClick={() => onNavigate("model")}>
        <span className="nd-file-icon"><FileText size={20} /></span>
        <span><strong>{projectName}</strong><small>Unsaved project</small></span>
        <span><small>Current project</small><small>{runs.length} completed run{runs.length === 1 ? "" : "s"}</small></span>
        <ChevronRight size={16} />
      </button> : null}
      {recentProjects.map((entry) => {
        const isCurrent = entry.path.toLocaleLowerCase() === projectPath?.toLocaleLowerCase();
        const openedAt = new Date(entry.openedAt);
        const openedLabel = Number.isNaN(openedAt.valueOf()) ? "Saved project" : openedAt.toLocaleDateString();
        return <button className="nd-project-row" key={entry.path.toLocaleLowerCase()} onClick={() => isCurrent ? onNavigate("model") : onOpenRecent(entry.path)}>
          <span className="nd-file-icon"><FileText size={20} /></span>
          <span><strong>{entry.name}</strong><small>{entry.path}</small></span>
          <span><small>{isCurrent ? datasetName : "Saved project"}</small><small>{isCurrent ? `${runs.length} completed run${runs.length === 1 ? "" : "s"}` : openedLabel}</small></span>
          <ChevronRight size={16} />
        </button>;
      })}
      {!hasRows ? <div className="nd-empty"><FolderOpen size={28} /><strong>No recent projects</strong><span>Create a project or open an existing .qpls file.</span></div> : null}
    </section>
  </div>;
}

function ModelSurface({ modelName, propertiesOpen, readiness, onContextMenuRequest }: { modelName: string; propertiesOpen: boolean; readiness: NativePlsReadiness; onContextMenuRequest: (request: ModelCanvasContextMenuRequest) => void }) {
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const experimentalSemAuthoringEnabled = useWorkspace((state) => state.uiPreferences.experimentalLabsEnabled);
  const dataset = useWorkspace((state) => state.dataset);
  const groupingVariable = useWorkspace((state) => state.analysisSettings.groupColumn?.trim() ?? "");
  const nodes = useWorkspace((state) => state.nodes);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const commitStandardIntent = useWorkspace((state) => state.commitStandardSemModelV4Intent);
  const [authorityStatus, setAuthorityStatus] = useState<string | null>(null);
  const selected = nodes.find((node) => node.id === selectedNodeId);
  const selectedAssignableConstruct = selected?.data.semantic === "interaction" || selected?.data.semantic === "higher_order" ? undefined : selected;

  type ModelDocumentView = "canvas" | "parameters" | "cbsem_labs";
  const [documentView, setDocumentView] = useState<ModelDocumentView>("canvas");
  useEffect(() => setDocumentView("canvas"), [activeModelId, experimentalSemAuthoringEnabled]);
  const selectDocumentView = (view: ModelDocumentView, moveFocus = false) => {
    if (view === "parameters" && !experimentalSemAuthoringEnabled) return;
    setDocumentView(view);
    if (moveFocus) window.setTimeout(() => document.getElementById({ canvas: "nd-model-canvas-tab", parameters: "nd-model-parameter-tab", cbsem_labs: "nd-model-cbsem-labs-tab" }[view])?.focus(), 0);
  };
  const onDocumentTabKeyDown = (event: ReactKeyboardEvent<HTMLButtonElement>) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const views: ModelDocumentView[] = experimentalSemAuthoringEnabled ? ["canvas", "parameters", "cbsem_labs"] : ["canvas", "cbsem_labs"];
    const currentIndex = Math.max(0, views.indexOf(documentView));
    const nextIndex = event.key === "Home"
      ? 0
      : event.key === "End"
        ? views.length - 1
        : event.key === "ArrowLeft"
          ? (currentIndex - 1 + views.length) % views.length
          : (currentIndex + 1) % views.length;
    selectDocumentView(views[nextIndex], true);
  };
  const [query, setQuery] = useState("");
  const visibleColumns = dataset.columns.filter((column) => column.toLowerCase().includes(query.trim().toLowerCase()));
  const dragVariable = (event: DragEvent<HTMLButtonElement>, variable: string) => {
    if (variable === groupingVariable) {
      event.preventDefault();
      return;
    }
    event.dataTransfer.setData("application/qpls-indicator", variable);
    event.dataTransfer.setData("application/qpls-indicators", JSON.stringify([variable]));
  };
  const activateIndicator = (variable: string) => {
    if (variable === groupingVariable) return;
    const owner = nodes.find((node) => node.data.indicators.includes(variable));
    if (owner) {
      setSelectedNode(owner.id);
      window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id: owner.id } }));
    } else if (selectedAssignableConstruct) {
      if (strictAuthority) {
        setAuthorityStatus("Committing strict indicator assignment…");
        void commitStandardIntent({ kind: "assign_indicators", construct_id: selectedAssignableConstruct.id, indicators: [observedForStrictDesktop(strictAuthority, dataset, variable)] })
          .then((result) => setAuthorityStatus(desktopAuthorityResultMessage(result)));
      } else assignIndicator(selectedAssignableConstruct.id, variable);
    } else {
      if (strictAuthority) {
        setAuthorityStatus("Committing strict construct creation…");
        void commitStandardIntent({
          kind: "add_construct",
          variable_id: `standard:editor:construct:${Date.now()}`,
          label: variable,
          representation: { kind: "composite", weighting: { kind: "mode_a" } },
          indicators: [observedForStrictDesktop(strictAuthority, dataset, variable)],
        }).then((result) => setAuthorityStatus(desktopAuthorityResultMessage(result)));
      } else addConstruct(undefined, [variable]);
    }
  };
  return <div className={`nd-three-pane nd-model-workspace${propertiesOpen ? "" : " no-properties"}`}>
    <aside className="nd-navigator" aria-label="Model navigator">
      <PaneTitle icon={<Database size={14} />} title="Indicators" />
      <label className="nd-search"><Search size={13} /><input aria-label="Search indicators" placeholder="Search" value={query} onChange={(event) => setQuery(event.target.value)} /></label>
      <p id="nd-indicator-drag-help" className="nd-variable-instruction">Drag an indicator to the canvas or onto a construct.</p>
      <div className="nd-variable-list" aria-describedby="nd-indicator-drag-help">
        {visibleColumns.map((column) => {
          const owner = nodes.find((node) => node.data.indicators.includes(column));
          const isGroupingVariable = column === groupingVariable;
          const action = isGroupingVariable
            ? "Grouping variable; unavailable as an indicator"
            : owner
              ? `Select owner ${owner.data.label}`
              : selectedAssignableConstruct
                ? `Assign to ${selectedAssignableConstruct.data.label}`
                : "Create construct with this indicator";
          return <button
            key={column}
            type="button"
            draggable={!isGroupingVariable}
            disabled={isGroupingVariable}
            onDragStart={(event) => dragVariable(event, column)}
            onClick={() => activateIndicator(column)}
            className={`nd-variable-item${owner ? " assigned" : ""}${isGroupingVariable ? " grouping" : ""}`}
            title={isGroupingVariable ? action : `${action}; drag to place elsewhere`}
            aria-label={isGroupingVariable ? `${column}. ${action}` : `${column}. ${action}; or drag to the model canvas or a construct`}
          ><Square size={9} fill="currentColor" />{column}{isGroupingVariable ? <small>Group</small> : owner ? <Check size={12} /> : null}</button>;
        })}
      </div>
      {authorityStatus ? <p className="nd-authority-feedback" role="status" aria-live="polite">{authorityStatus}</p> : null}
      <PaneTitle icon={<Circle size={13} />} title="Constructs" />
      <div className="nd-variable-list">
        {nodes.map((node) => <button key={node.id} className={selectedNodeId === node.id ? "active" : ""} onClick={() => setSelectedNode(node.id)}>{node.data.semantic === "interaction" ? <GitBranch size={11} /> : <Circle size={11} />}{node.data.label}<small>{node.data.semantic === "interaction" ? "INT" : node.data.semantic === "higher_order" ? "HOC" : node.data.indicators.length}</small></button>)}
      </div>
    </aside>
    <section className="nd-document nd-model-document">
      <div className="nd-model-document-tabs" role="tablist" aria-label={`${modelName} views`}>
        <span className="nd-model-document-title" title={modelName}><GitBranch size={14} aria-hidden="true" />{modelName}</span>
        <button
          id="nd-model-canvas-tab"
          type="button"
          role="tab"
          aria-selected={documentView === "canvas"}
          aria-controls="nd-model-canvas-panel"
          tabIndex={documentView === "canvas" ? 0 : -1}
          onClick={() => selectDocumentView("canvas")}
          onKeyDown={onDocumentTabKeyDown}
        ><GitBranch size={13} aria-hidden="true" />Canvas</button>
        {experimentalSemAuthoringEnabled ? <button
          id="nd-model-parameter-tab"
          type="button"
          role="tab"
          aria-selected={documentView === "parameters"}
          aria-controls="nd-model-parameter-panel"
          tabIndex={documentView === "parameters" ? 0 : -1}
          onClick={() => selectDocumentView("parameters")}
         onKeyDown={onDocumentTabKeyDown}
        ><TableProperties size={13} aria-hidden="true" />Parameter Table <span className="nd-experimental-chip">Experimental</span></button> : null}
        <button
          id="nd-model-cbsem-labs-tab"
          type="button"
          role="tab"
          aria-selected={documentView === "cbsem_labs"}
          aria-controls="nd-model-cbsem-labs-panel"
          tabIndex={documentView === "cbsem_labs" ? 0 : -1}
          onClick={() => selectDocumentView("cbsem_labs")}
          onKeyDown={onDocumentTabKeyDown}
        ><Calculator size={13} aria-hidden="true" />Exact CB-SEM</button>
      </div>
      {documentView === "canvas" ? <div id="nd-model-canvas-panel" className="nd-canvas-host" role="tabpanel" aria-labelledby="nd-model-canvas-tab"><ModelCanvas onContextMenuRequest={onContextMenuRequest} /></div> : null}
      {experimentalSemAuthoringEnabled && documentView === "parameters" ? <NativeSemParameterTable modelName={modelName} onShowCanvas={() => selectDocumentView("canvas")} /> : null}
      {documentView === "cbsem_labs" ? <NativeRecipeV4CbsemWorkspace modelName={modelName} experimentalLabsEnabled={false} /> : null}
    </section>
    {propertiesOpen ? <NativeModelInspector readiness={readiness} /> : null}
  </div>;
}

function observedForStrictDesktop(
  authority: StandardSemModelV4AuthorityRecordV1,
  dataset: Dataset,
  column: string,
): Extract<SemVariableV4, { kind: "observed" }> {
  const existing = authority.model.variables.find((variable): variable is Extract<SemVariableV4, { kind: "observed" }> =>
    variable.kind === "observed" && variable.source_column === column);
  if (existing) return structuredClone(existing);
  const metadata = dataset.columnMetadata?.find((item) => item.name === column);
  return {
    kind: "observed",
    id: `observed:${column}`,
    label: metadata?.label?.trim() || column,
    source_column: column,
    scale: metadata?.scale_type ?? "continuous",
    role: "indicator",
    categories: Object.keys(metadata?.value_labels ?? {}).sort(compareUtf8StringsV1),
    value_labels: { ...(metadata?.value_labels ?? {}) },
    missing_markers: [...new Set((metadata?.missing_markers ?? []).map((value) => value.trim()).filter(Boolean))].sort(compareUtf8StringsV1),
    transformation_lineage: [],
  };
}

function desktopAuthorityResultMessage(result: StandardSemModelV4AuthorityCommitResult): string {
  if (result.status === "committed") return "Committed to the strict Standard model authority.";
  if (result.status === "blocked") return `Blocked: ${result.diagnostic.message} ${result.diagnostic.correctiveAction}`;
  if (result.status === "stale") return "Stale edit ignored because the active authority changed.";
  return `Rejected: ${result.error instanceof Error ? result.error.message : String(result.error)}`;
}

function ResultsSurfaceLoading({ propertiesOpen }: { propertiesOpen: boolean }) {
  return <div className={`nd-three-pane nd-results-workspace${propertiesOpen ? "" : " no-properties"}`} aria-busy="true">
    <aside className="nd-navigator" aria-hidden="true" />
    <section className="nd-document nd-results-document">
      <div className="nd-empty" role="status"><strong>Opening results...</strong></div>
    </section>
    {propertiesOpen ? <aside className="nd-properties" aria-hidden="true" /> : null}
  </div>;
}
function PaneTitle({ title, icon }: { title: string; icon?: ReactNode }) {
  return <header className="nd-pane-title">{icon}<strong>{title}</strong></header>;
}

function DialogHost({ dialog, close, title, children, dismissible = true }: { dialog: NativeDialog; close: () => void; title: string; children: ReactNode; dismissible?: boolean }) {
  const dialogRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const root = dialogRef.current;
    const focusable = () => Array.from(root?.querySelectorAll<HTMLElement>('button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [href], [tabindex]:not([tabindex="-1"])') ?? []).filter((element) => !element.hidden && element.getAttribute("aria-hidden") !== "true");
    const initial = root?.querySelector<HTMLElement>("[autofocus]") ?? focusable()[0] ?? root;
    window.setTimeout(() => initial?.focus(), 0);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        if (dismissible) {
          event.preventDefault();
          close();
        }
        return;
      }
      if (event.key !== "Tab") return;
      const candidates = focusable();
      if (!candidates.length) {
        event.preventDefault();
        root?.focus();
        return;
      }
      const first = candidates[0];
      const last = candidates[candidates.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => { window.removeEventListener("keydown", onKeyDown); previous?.focus(); };
  }, [close, dismissible]);
  return <div className="nd-dialog-backdrop" role="presentation" onMouseDown={(event) => { if (dismissible && event.target === event.currentTarget) close(); }}>
    <div className={`nd-dialog nd-dialog-${dialog}`} role="dialog" aria-modal="true" aria-labelledby="nd-dialog-title" ref={dialogRef} tabIndex={-1}>
      <header><h2 id="nd-dialog-title">{title}</h2>{dismissible ? <button aria-label="Close dialog" onClick={close}><X size={15} /></button> : null}</header>
      {children}
    </div>
  </div>;
}

function NewProjectDialog({ value, setValue, close, create }: { value: string; setValue: (value: string) => void; close: () => void; create: () => void }) {
  return <form className="nd-dialog-form" onSubmit={(event) => { event.preventDefault(); create(); }}>
    <label>Project name<input autoFocus value={value} onChange={(event) => setValue(event.target.value)} /></label>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">Create</button></footer>
  </form>;
}

function UtilityDialogLoading({ label }: { label: string }) {
  return <div className="nd-utility-dialog" role="status" aria-live="polite" aria-busy="true"><p>{label}...</p></div>;
}

function RunDetailsDialog({ run }: { run: AnalysisRun }) {
  const settingApplicability = nativeRunSettingApplicability(run);
  const nca = nativeNcaResultProjection(run);
  const pca = nativePcaResultProjection(run);
  const ols = nativeOlsResultProjection(run);
  const cbsem = nativeCbsemResultProjection(run);
  const gsca = nativeGscaResultProjection(run);
  const warnings = [...new Set([
    ...run.warnings,
    ...(nca?.warnings ?? []),
    ...(pca?.warnings ?? []),
    ...(ols?.warnings ?? []),
    ...(cbsem?.analysis.warnings ?? []),
    ...(gsca?.analysis.warnings ?? []),
  ])];
  return <div className="nd-run-details">
    <dl className="nd-property-list">
      <div><dt>Run</dt><dd>{run.name}</dd></div>
      <div><dt>Method</dt><dd>{run.method}</dd></div>
      <div><dt>Created</dt><dd>{new Date(run.createdAt).toLocaleString()}</dd></div>
      {settingApplicability.usesSeed ? <div><dt>Recorded seed</dt><dd>{run.provenance?.seed ?? run.seed}</dd></div> : null}
      <div><dt>Dataset fingerprint</dt><dd>{run.provenance?.dataset_fingerprint ?? run.fingerprint}</dd></div>
      {run.provenance ? <>
        <div><dt>Recipe</dt><dd>{run.provenance.recipe_id}</dd></div>
        <div><dt>Engine</dt><dd>{run.provenance.engine_version}</dd></div>
        <div><dt>Method version</dt><dd>{run.provenance.method_version}</dd></div>
        {nca ? <>
          <div><dt>Condition (X)</dt><dd>{nca.x}</dd></div>
          <div><dt>Outcome (Y)</dt><dd>{nca.y}</dd></div>
          <div><dt>Observations</dt><dd>{nca.observations}</dd></div>
          <div><dt>Ceiling lines</dt><dd>{nativeNcaCeilingLabel(nca.ceiling)}</dd></div>
          <div><dt>Requested permutations</dt><dd>{nca.permutationSamples}</dd></div>
          <div><dt>Usable permutations</dt><dd>{nca.usablePermutations}</dd></div>
          <div><dt>Missing data</dt><dd>Listwise deletion</dd></div>
        </> : pca ? <>
          <div><dt>Variables</dt><dd>{pca.variables.length}</dd></div>
          <div><dt>Retention rule</dt><dd>{nativePcaComponentRuleLabel(pca.componentRule)}</dd></div>
          <div><dt>Retained components</dt><dd>{pca.retainedComponents}</dd></div>
          <div><dt>Observations</dt><dd>{pca.observations}</dd></div>
          <div><dt>Input matrix</dt><dd>Correlation matrix of standardized variables</dd></div>
          <div><dt>Missing data</dt><dd>Listwise deletion</dd></div>
        </> : ols ? <>
          <div><dt>Outcome</dt><dd>{ols.outcome}</dd></div>
          <div><dt>Predictors</dt><dd>{ols.predictors.join(", ")}</dd></div>
          <div><dt>Controls</dt><dd>{ols.controls.length ? ols.controls.join(", ") : "None"}</dd></div>
          <div><dt>Observations</dt><dd>{ols.observations}</dd></div>
          <div><dt>Estimator</dt><dd>OLS with intercept</dd></div>
          <div><dt>Standard errors</dt><dd>HC3 robust</dd></div>
          <div><dt>Confidence intervals</dt><dd>Two-sided 95%</dd></div>
          <div><dt>Missing data</dt><dd>Listwise deletion</dd></div>
        </> : cbsem ? <>
          <div><dt>Model type</dt><dd>{cbsem.modelType === "cfa" ? "Confirmatory factor analysis" : "Recursive structural equation model"}</dd></div>
          <div><dt>Estimator</dt><dd>Maximum likelihood</dd></div>
          <div><dt>Complete cases</dt><dd>{cbsem.analysis.sample_size}</dd></div>
          <div><dt>Converged</dt><dd>{cbsem.analysis.converged ? "Yes" : "No"}</dd></div>
          <div><dt>Optimizer iterations</dt><dd>{cbsem.analysis.iterations}</dd></div>
          <div><dt>Input</dt><dd>Raw case-level data</dd></div>
          <div><dt>Missing data</dt><dd>Listwise deletion</dd></div>
          <div><dt>Identification</dt><dd>First loading fixed to 1 per latent factor</dd></div>
        </> : gsca ? <>
          <div><dt>Estimator</dt><dd>Joint global least-squares ALS</dd></div>
          <div><dt>Algorithm version</dt><dd>{gsca.algorithmVersion}</dd></div>
          <div><dt>Complete cases</dt><dd>{gsca.usedObservations}</dd></div>
          <div><dt>Omitted cases</dt><dd>{gsca.omittedObservations}</dd></div>
          <div><dt>Converged</dt><dd>{gsca.analysis.converged ? "Yes" : "No"}</dd></div>
          <div><dt>ALS iterations</dt><dd>{gsca.analysis.iterations}</dd></div>
          <div><dt>Global FIT</dt><dd>{gsca.analysis.fit.toFixed(6)}</dd></div>
          <div><dt>Adjusted FIT</dt><dd>{gsca.analysis.adjusted_fit.toFixed(6)}</dd></div>
          <div><dt>GFI</dt><dd>{gsca.analysis.gfi.toFixed(6)}</dd></div>
          <div><dt>SRMR</dt><dd>{gsca.analysis.srmr.toFixed(6)}</dd></div>
          <div><dt>Input</dt><dd>Listwise-standardized raw case data</dd></div>
          <div><dt>Inference</dt><dd>Point estimates only</dd></div>
        </> : <>
          <div><dt>Weighting</dt><dd>{run.provenance.settings.weighting_scheme}</dd></div>
          <div><dt>Preprocessing</dt><dd>{run.provenance.settings.preprocessing}</dd></div>
          {run.provenance.settings.case_weight_column ? <div><dt>Case-weight variable</dt><dd>{run.provenance.settings.case_weight_column}</dd></div> : null}
        </>}
      </> : run.result?.method_version ? <div><dt>Method version</dt><dd>{run.result.method_version}</dd></div> : null}
    </dl>
    {warnings.length ? <details><summary>Warnings ({warnings.length})</summary><ul>{warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></details> : null}
    {run.logs?.length ? <details open><summary>Calculation log ({run.logs.length})</summary><ol>{run.logs.map((entry) => <li key={entry.id}><time>{new Date(entry.timestamp).toLocaleTimeString()}</time><span><strong>{entry.phase}</strong> {entry.message}</span></li>)}</ol></details> : <p>No calculation log was stored for this older run.</p>}
  </div>;
}


function NativeToastStack({ toasts, dismiss }: { toasts: ReturnType<typeof useWorkspace.getState>["toasts"]; dismiss: (id: string) => void }) {
  if (!toasts.length) return null;
  return <aside className="nd-toast-stack" aria-live="polite" aria-label="Application notifications">
    {toasts.map((toast) => <NativeToastItem key={toast.id} toast={toast} dismiss={dismiss} />)}
  </aside>;
}

function NativeToastItem({ toast, dismiss }: { toast: ReturnType<typeof useWorkspace.getState>["toasts"][number]; dismiss: (id: string) => void }) {
  useEffect(() => {
    const timer = window.setTimeout(() => dismiss(toast.id), 5200);
    return () => window.clearTimeout(timer);
  }, [dismiss, toast.id]);
  return <article className={`nd-toast ${toast.tone}`}>
      <span className="nd-toast-icon" aria-hidden="true">{toast.tone === "success" ? <Check size={12} /> : toast.tone === "error" ? "!" : "i"}</span>
      <span><strong>{toast.title}</strong>{toast.detail ? <small>{toast.detail}</small> : null}</span>
      <button type="button" aria-label={`Dismiss ${toast.title}`} onClick={() => dismiss(toast.id)}><X size={13} /></button>
    </article>;
}
function ShortcutsDialog() {
  const shortcuts = [["Ctrl+N", "New project"], ["Ctrl+O", "Open project"], ["Ctrl+S", "Save"], ["Ctrl+Shift+S", "Save as"], ["Ctrl+R", "Calculate"], ["C", "Create construct"], ["V", "Select tool"], ["P", "Path tool"], ["F", "Fit model"], ["Enter", "Edit selected construct or path"], ["Delete", "Delete selection"], ["Tab", "Navigate model and inspector controls"], ["Arrow keys", "Move between inspector sections"]];
  return <div className="nd-shortcuts" role="list">{shortcuts.map(([keys, label]) => <div role="listitem" key={keys}><kbd>{keys}</kbd><span>{label}</span></div>)}</div>;
}
export function aboutVisibleAnalysisLabelsV2(settings: AnalysisUiSettings, experimentalLabsEnabled: boolean): readonly string[] {
  return NATIVE_ANALYSIS_CATALOG.filter((item) => {
    const availability = methodCapabilityAvailabilityV2(
      nativeCapabilitySettingsForWorkbenchKindV2(settings, item.kind),
      { experimentalLabsEnabled },
    );
    return availability.selectable
      || (experimentalLabsEnabled && isNativeEstablishedWorkingAnalysisKindV1(item.kind));
  }).map((item) => item.label);
}

function AboutDialog({ settings, experimentalLabsEnabled }: { settings: AnalysisUiSettings; experimentalLabsEnabled: boolean }) {
  const visibleMethods = aboutVisibleAnalysisLabelsV2(settings, experimentalLabsEnabled);
  const mode = experimentalLabsEnabled ? "Experimental Labs" : "Standard";
  return <div className="nd-about"><div className="nd-about-mark">Q</div><div><h3>QuickPLS</h3><p>Offline structural equation modeling for Windows.</p><dl className="nd-property-list"><div><dt>Calculate mode</dt><dd>{mode}</dd></div><div><dt>Available calculation methods</dt><dd>{visibleMethods.length ? visibleMethods.join(", ") : "No methods are available in the current mode."}</dd></div><div><dt>Conditional result groups</dt><dd>Mediation and two-stage Moderation appear only when the completed model contains those effects.</dd></div><div><dt>Runtime</dt><dd>{isNativeDesktop() ? "Native desktop" : "Browser preview"}</dd></div><div><dt>Implementation</dt><dd>Independent QuickPLS engine</dd></div><div><dt>Third-party notices</dt><dd>Included with the installed application</dd></div></dl></div></div>;
}

function StatusBar({ surface, projectName, datasetName, cases, constructs, runMonitor }: { surface: NativeSurface; projectName: string; datasetName: string; cases: number; constructs: number; runMonitor: ReturnType<typeof useWorkspace.getState>["runMonitor"] }) {
  const stateLabel = ["queued", "validating", "running", "cancelling", "blocked", "failed", "cancelled"].includes(runMonitor.status)
    ? nativeCalculationPhaseLabel(runMonitor.phase, runMonitor.status)
    : "Ready";
  const projectOpen = projectName !== "No project open";
  return <footer className="nd-statusbar" aria-live="polite">
    <span className={`nd-status-dot ${runMonitor.status}`} />
    <strong>{stateLabel}</strong>
    {surface !== "launcher" || projectOpen ? <><span>{projectName}</span><span>{datasetName}</span><span>{cases} cases</span><span>{constructs} constructs</span></> : null}
    <span className="spacer" />
    <span>{surface === "launcher" && projectOpen ? "Project" : surface[0].toUpperCase() + surface.slice(1)}</span>
    <span>Offline</span>
  </footer>;
}

function dialogTitle(dialog: Exclude<NativeDialog, null>) {
  if (dialog === "new-project") return "New Project";
  if (dialog === "import-data") return "Import Data";
  if (dialog === "recode-data") return "Recode Variable";
  if (dialog === "derive-variable") return "Derive Variable";
  if (dialog === "group-setup") return "Configure Groups";
  if (dialog === "moderation") return "Create Moderating Effect";
  if (dialog === "higher-order") return "Create Higher-Order Construct";
  if (dialog === "calculation") return "Calculate";
  if (dialog === "export") return "Export Results";
  if (dialog === "trust") return "Method Details";
  if (dialog === "settings") return "Preferences";
  if (dialog === "shortcuts") return "Keyboard Shortcuts";
  if (dialog === "about") return "About QuickPLS";
  return "Run Details";
}
