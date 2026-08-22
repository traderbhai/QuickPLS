import {
  Calculator,
  Check,
  ChevronDown,
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
import {
  ModelCanvas,
  type ModelCanvasContextMenuRequest,
  type ModelCanvasContextMenuTarget,
} from "../components/ModelCanvas";
import { NativeDataImportDialog } from "./NativeDataImportDialog";
import { NativeDatasetTransformDialog } from "./NativeDatasetTransformDialog";
import { NativeRecodeDialog } from "./NativeRecodeDialog";
import { NativeDataSurface } from "./NativeDataSurface";
import { NativeModelInspector } from "./NativeModelInspector";
import { NativeRecipeV4CbsemWorkspace } from "./NativeRecipeV4CbsemWorkspace";
import { NativeRecipeV4GeneralSemWorkspace } from "./NativeRecipeV4GeneralSemWorkspace";
import { NativeSemParameterTable, observedSemanticsForParameterTable } from "./NativeSemParameterTable";
import NativeWorkspaceExplorer, {
  NativeWorkspaceExplorerDialog,
  type NativeExplorerDialog,
} from "./NativeWorkspaceExplorer";
import { nextNativeWorkspaceModelName } from "./nativeWorkspaceTree";
import {
  canAddNativeModeration,
  nativeModeratingEffect,
} from "./nativeModeration";
import type {
  NativeModerationDialogCommitResult,
  NativeModerationDialogRequest,
  NativeModerationDialogSubmissionV1,
} from "./NativeModerationDialog";
import {
  canCreateNativeHigherOrder,
  nativeHigherOrderDraftApproach,
  nativeHigherOrderDraftMeasurementType,
} from "./nativeHigherOrder";
import type {
  NativeHigherOrderDialogCommitResult,
  NativeHigherOrderDialogRequest,
  NativeHigherOrderDialogSubmission,
} from "./NativeHigherOrderDialog";
import {
  nativeDataContextSelection,
  type NativeDataContextMenuRequest,
  type NativeDataContextTarget,
} from "./nativeDataContext";
import { NativeDesktopController } from "./NativeDesktopController";
import {
  nativeIndicatorDragLabelV1,
  nativeIndicatorDragSelectionV1,
  nextNativeIndicatorSelectionV1,
  type NativeIndicatorSelectionV1,
} from "./nativeIndicatorSelectionV1";
import { planNativeIndicatorGroupActionV1 } from "./nativeIndicatorGroupActionV1";
import {
  NATIVE_CANVAS_ARRANGE_MENU_V1,
  NATIVE_CANVAS_FIT_MENU_V1,
} from "./nativeCanvasCommandMenusV1";
import { nativeModelNavigatorRelationshipsV1 } from "./nativeModelNavigatorV1";
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
  type NativeModerationMutationAuthorityV1,
  type NativeSurface,
} from "./nativeCommands";
import {
  contextMenuCoordinates,
  isContextMenuKeyboardGesture,
  nextEnabledItemIndex,
  nextMenuIndex,
  type MenuItemNavigationKey,
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
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type {
  UnifiedSemCalculationActionV1,
  UnifiedSemCalculationContextV1,
  UnifiedSemCalculationPlanV1,
} from "../domain/unifiedSemCalculationV1";
import {
  loadNativeRecentProjects,
  rememberNativeRecentProject,
  storeNativeRecentProjects,
  type NativeRecentProject,
} from "./nativeRecentProjects";
import { nativePlsReadiness, type NativePlsReadiness } from "./nativePlsReadiness";
import {
  applyNativeDatasetTransformation,
  authorizeNativeGeneralSemRevisionDraftV1,
  getNativeCapabilityRegistryV2,
  invalidateNativeGeneralSemFreshDraftAuthorityV1,
  isNativeDesktop,
  previewNativeDatasetTransformation,
  recodeNativeDatasetColumn,
} from "../services/projectService";
import type { DatasetTransformationSpecV2 } from "../domain/datasetTransformationsV2";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
} from "../domain/generalSemCapabilityPreflightV1";
import { supportsGeneralSemV1 } from "../domain/internalProjectArchiveV6Wire";
import {
  bindGeneralSemPlsModelToDatasetV1,
  defaultGeneralSemPlsEngineOptionsV1,
  generalSemConfigFromEngineV1,
  generalSemWorkspaceProductAccessV1,
  rehydrateGeneralSemExecutionAuthorityV1,
} from "../domain/internalRecipeV4GeneralSemWorkspace";
import { adaptAuthoredNativeWorkbenchToSemModelV4 } from "../domain/nativeWorkbenchSemModelV4Adapter";
import { bindInternalRecipeV4CbsemDatasetV1 } from "../domain/internalRecipeV4CbsemWorkspace";
import { methodCapabilityAvailabilityV2 } from "../domain/methodCapabilityRegistryV2";
import {
  MODERATION_CANVAS_REQUEST_EVENT,
  type ModerationCanvasRequestV1,
  type ModeratingEffectTargetV1,
} from "../domain/moderationDiagramProjectionV1";
import {
  GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
  standardSemGeneralSemModerationV3IdentityV1,
  type StandardSemModelV4EditorIntentV1,
} from "../domain/standardSemModelV4Authority";
import type { SemModelV4 } from "../domain/semModelV4";
import { useInternalProjectArchiveV6Session } from "../internalProjectArchiveV6SessionStore";
import { useWorkspace } from "../store";
import type {
  AnalysisRun,
  AnalysisUiSettings,
  InteractionData,
  ModelEditCommandV1,
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
type NativeDialog = "new-project" | "import-data" | "recode-data" | "derive-variable" | "group-setup" | "higher-order" | "moderation" | "calculation" | "advanced-calculation" | "advanced-parameters" | "export" | "trust" | "settings" | "run-details" | "shortcuts" | "about" | null;
export function completedRunNavigationTarget(
  status: RunMonitorStatus,
  lastRunId: string | null,
  lastNavigatedCompletedRunId: string | null,
): string | null {
  return status === "completed" && lastRunId && lastRunId !== lastNavigatedCompletedRunId
    ? lastRunId
    : null;
}

interface StrictDesktopModerationIntentCommonV1 {
  readonly label: string;
  readonly predictor: string;
  readonly moderator: string;
  readonly focalRelation: string;
  readonly outcome: string;
}

export type StrictDesktopModerationIntentInputV1 = StrictDesktopModerationIntentCommonV1 & (
  | {
    readonly projectMode: "standard";
    readonly legacyTermId: string;
    readonly legacyOutputId: string;
  }
  | { readonly projectMode: "general_sem_v1" }
);

export function buildStrictDesktopModerationIntentV1(
  input: StrictDesktopModerationIntentInputV1,
): { intent: StandardSemModelV4EditorIntentV1; interactionId: string } {
  if (input.projectMode === "general_sem_v1") {
    const identity = standardSemGeneralSemModerationV3IdentityV1(
      { kind: "focal_relation", relationId: input.focalRelation },
      [input.predictor, input.moderator],
    );
    return {
      intent: {
        kind: "add_moderating_effect_v3",
        intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
        sem_generation: "general_sem_v1",
        label: input.label,
        operands: [input.predictor, input.moderator],
        target: { kind: "focal_relation", relationId: input.focalRelation },
        outcome: input.outcome,
        method: "two_stage",
        hierarchy_policy: "strong",
      },
      interactionId: identity.outputId,
    };
  }
  return {
    intent: {
      kind: "add_interaction",
      term_id: input.legacyTermId,
      output_id: input.legacyOutputId,
      label: input.label,
      predictor: input.predictor,
      moderator: input.moderator,
      focal_relation: input.focalRelation,
      outcome: input.outcome,
      method: "two_stage",
    },
    interactionId: input.legacyOutputId,
  };
}

export interface NativeGeneralSemRevisionCommandStateV1 {
  readonly standardActivationPending: boolean;
  readonly revisionForkPending: boolean;
  readonly saveCopyPending: boolean;
  readonly sessionDirty: boolean;
  readonly publicationPending: boolean;
  readonly transientWorkBlocker: "job_active" | "temporary_result_pending" | null;
  readonly calculationStatus: RunMonitorStatus;
}

export function nativeGeneralSemRevisionCommandDisabledReasonV1(
  state: NativeGeneralSemRevisionCommandStateV1,
): string | null {
  if (state.revisionForkPending) {
    return "Wait for the current calculation-ready Save As Revision transaction to finish.";
  }
  if (state.standardActivationPending || state.saveCopyPending) {
    return "Wait for the current schema-6 authority operation to finish.";
  }
  if (state.publicationPending) {
    return "Wait for calculation-ready project publication to finish.";
  }
  if (state.transientWorkBlocker === "job_active") {
    return "Finish or cancel the active advanced calculation before creating a revision.";
  }
  if (state.transientWorkBlocker === "temporary_result_pending") {
    return "Save and strictly reopen the completed result, or dismiss it, before creating a revision.";
  }
  if (isNativeCalculationActive(state.calculationStatus)) {
    return "Finish or cancel the active calculation before creating a revision.";
  }
  if (state.sessionDirty) {
    return "Restore or reopen the exact clean calculation authority before creating a revision.";
  }
  return null;
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
      loadNamedSemEvidenceFixture: (fixture: import("../data/v255NamedSemEvidenceFixtures").V255NamedSemFixture) => unknown | Promise<unknown>;
      namedSemEvidenceSnapshot: () => unknown;
      exerciseNamedAdvancedParameterRevision: () => Promise<unknown>;
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
  disabledReason?: string;
  pressed?: boolean;
  primary?: boolean;
  action: () => void;
  menuItems?: readonly MenuItem[];
}

interface MenuItem {
  id: string;
  label: string;
  shortcut?: string;
  disabled?: boolean;
  disabledReason?: string;
  separator?: boolean;
  action: () => void;
}

interface NativeContextMenuState {
  x: number;
  y: number;
  returnFocus: HTMLElement | null;
  selection: NativeCommandContext["selection"];
  target?: NativeDataContextTarget;
  modelTarget?: ModelCanvasContextMenuTarget;
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

const menuGroups: ReadonlyArray<readonly [string, NativeMenuId]> = [["File", "file"], ["Edit", "edit"], ["View", "view"], ["Model", "model"], ["Calculate", "calculate"], ["Tools", "tools"], ["Help", "help"]];
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
  const projectId = useWorkspace((state) => state.projectId);
  const projectPath = useWorkspace((state) => state.projectPath);
  const dataset = useWorkspace((state) => state.dataset);
  const datasetDescriptorOnly = useWorkspace((state) => state.datasetDescriptorOnly);
  const generalSemPublicationPending = useWorkspace((state) => state.generalSemPublicationPending);
  const generalSemTransientWorkBlocker = useWorkspace((state) => state.generalSemTransientWorkBlocker);
  const projectWritable = useWorkspace((state) => state.projectWritable);
  const projectModels = useWorkspace((state) => state.projectModels);
  const strictAuthorities = useWorkspace((state) => state.standardSemModelV4Authorities);
  const strictScientificEditLocks = useWorkspace((state) => state.standardSemModelV4ScientificEditLocks);
  const activeModelId = useWorkspace((state) => state.activeModelId);
  const schema6Session = useInternalProjectArchiveV6Session((state) => state.session);
  const generalSemStandardActivationPending = useInternalProjectArchiveV6Session((state) => state.standardActivationPending);
  const generalSemRevisionPending = useInternalProjectArchiveV6Session((state) => state.revisionForkPending);
  const generalSemSaveCopyPending = useInternalProjectArchiveV6Session((state) => state.saveCopyPending);
  const generalSemSessionDirty = useInternalProjectArchiveV6Session((state) => state.dirty);
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
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const setAnalysisSettings = useWorkspace((state) => state.setAnalysisSettings);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const toasts = useWorkspace((state) => state.toasts);
  const dismissToast = useWorkspace((state) => state.dismissToast);
  const resetRunMonitor = useWorkspace((state) => state.resetRunMonitor);
  const addRun = useWorkspace((state) => state.addRun);
  const commitDatasetVersion = useWorkspace((state) => state.commitDatasetVersion);
  const pushToast = useWorkspace((state) => state.pushToast);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const strictGeneralSemAuthority = Boolean(
    strictAuthority
    && activeModelId
    && schema6Session?.standardActivation?.modelIds.includes(activeModelId)
    && supportsGeneralSemV1(schema6Session.project),
  );
  const generalSemProjectDraftMode = useWorkspace((state) => state.generalSemProjectDraftMode);
  const beginGeneralSemProjectRevisionDraftMode = useWorkspace((state) => state.beginGeneralSemProjectRevisionDraftMode);
  const strictGeneralSemRevisionRequired = Boolean(
    strictGeneralSemAuthority
    && activeModelId
    && (strictScientificEditLocks[activeModelId] || !projectWritable),
  );
  const generalSemRevisionDisabledReason = strictGeneralSemRevisionRequired
    ? nativeGeneralSemRevisionCommandDisabledReasonV1({
      standardActivationPending: generalSemStandardActivationPending,
      revisionForkPending: generalSemRevisionPending,
      saveCopyPending: generalSemSaveCopyPending,
      sessionDirty: generalSemSessionDirty,
      publicationPending: generalSemPublicationPending,
      transientWorkBlocker: generalSemTransientWorkBlocker,
      calculationStatus: runMonitor.status,
    })
    : null;
  const commitStandardIntent = useWorkspace((state) => state.commitStandardSemModelV4Intent);
  const executeModelEditCommand = useWorkspace((state) => state.executeModelEditCommand);
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
  const commitGatewayDesktopCommand = async (command: ModelEditCommandV1, label: string) => {
    try {
      const result = await executeModelEditCommand(command);
      pushToast(result.status === "applied"
        ? { tone: "success", title: `${label} updated`, detail: "Applied as one undoable model transaction." }
        : { tone: "warning", title: `${label} unavailable`, detail: `${result.message} ${result.correctiveAction}` });
      return result;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      pushToast({ tone: "error", title: `${label} rejected`, detail: message });
      return { status: "rejected" as const, message };
    }
  };
  const dialogFailureForGatewayResult = (
    result: { status: "blocked"; code: string; message: string; correctiveAction: string }
      | { status: "rejected"; message: string },
  ): { status: "blocked" | "stale" | "rejected"; detail: string } => {
    if (result.status === "rejected") return { status: "rejected", detail: result.message };
    return {
      status: result.code === "model_edit.authority_stale"
        ? "stale"
        : result.code === "model_edit.authority_rejected"
          ? "rejected"
          : "blocked",
      detail: `${result.message} ${result.correctiveAction}`,
    };
  };
  const [surface, setSurface] = useState<NativeSurface>("launcher");
  const [dialog, setDialog] = useState<NativeDialog>(null);
  const [higherOrderDialogRequest, setHigherOrderDialogRequest] = useState<NativeHigherOrderDialogRequest>({ kind: "create" });
  const [moderationDialogRequest, setModerationDialogRequest] = useState<NativeModerationDialogRequest>({ kind: "create" });
  const [higherOrderCommitPending, setHigherOrderCommitPending] = useState(false);
  const [moderationCommitPending, setModerationCommitPending] = useState(false);
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
    if (currentDialogRef.current === "advanced-calculation"
      && useWorkspace.getState().generalSemTransientWorkBlocker) return;
    if ((currentDialogRef.current === "advanced-calculation" || currentDialogRef.current === "advanced-parameters")
      && useWorkspace.getState().generalSemRevisionDraftSource) {
      useWorkspace.getState().clearGeneralSemProjectDraftMode();
      void invalidateNativeGeneralSemFreshDraftAuthorityV1();
      useWorkspace.getState().pushToast({
        tone: "info",
        title: "Revision cancelled",
        detail: "The original project and scientific model were restored unchanged.",
      });
    }
    const closingCalculation = currentDialogRef.current === "calculation";
    currentDialogRef.current = null;
    setDialog(null);
    if (closingCalculation && !isNativeCalculationActive(useWorkspace.getState().runMonitor.status)) resetRunMonitor();
    restoreDialogFocus();
  }, [resetRunMonitor, restoreDialogFocus]);
  const openHigherOrderDialog = useCallback((request: NativeHigherOrderDialogRequest) => {
    setHigherOrderCommitPending(false);
    setHigherOrderDialogRequest(request);
    openDialog("higher-order");
  }, [openDialog]);
  const openModerationDialog = useCallback((request: NativeModerationDialogRequest) => {
    setModerationCommitPending(false);
    setModerationDialogRequest(request);
    openDialog("moderation");
  }, [openDialog]);
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
  const [advancedCalculationKind, setAdvancedCalculationKind] = useState<"pls_algorithm" | "pls_bootstrap" | "cbsem">("pls_algorithm");
  const [advancedCalculationPlan, setAdvancedCalculationPlan] = useState<UnifiedSemCalculationPlanV1 | null>(null);
  const [pendingExactCbsemPlan, setPendingExactCbsemPlan] = useState<UnifiedSemCalculationPlanV1 | null>(null);
  const [generalSemCanonicalResult, setGeneralSemCanonicalResult] = useState<CanonicalResultDocumentV2 | null>(null);
  const [generalSemResultSelected, setGeneralSemResultSelected] = useState(false);
  const lastNavigatedCompletedRunId = useRef<string | null>(null);
  const [recentProjects, setRecentProjects] = useState<NativeRecentProject[]>(() => loadNativeRecentProjects(window.localStorage));
  const smokeSeeded = useRef(false);

  useEffect(() => {
    setGeneralSemCanonicalResult(null);
    setGeneralSemResultSelected(false);
  }, [projectPath]);

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
  const rehydratedGeneralSemExecution = useMemo(() => {
    if (!strictGeneralSemAuthority || !schema6Session) return null;
    try { return rehydrateGeneralSemExecutionAuthorityV1(schema6Session.snapshot); } catch { return null; }
  }, [schema6Session, strictGeneralSemAuthority]);
  const draftGeneralSemModel = useMemo(() => {
    if (!activeModelId || !projectId) return null;
    const bindForUnifiedCalculation = (model: SemModelV4) => {
      if (dataset.kind === "covariance" || dataset.kind === "correlation") {
        const unitScales = Object.fromEntries(model.variables
          .filter((variable) => variable.kind === "observed")
          .map((variable) => [variable.id, 1]));
        return bindInternalRecipeV4CbsemDatasetV1(model, dataset, {
          covarianceDenominator: "sample_n_minus_one",
          missingDataPolicy: "listwise_deletion",
          correlationStandardDeviations: unitScales,
        });
      }
      return bindGeneralSemPlsModelToDatasetV1(model, dataset);
    };
    if (strictAuthority) {
      try { return bindForUnifiedCalculation(strictAuthority.model); } catch { return null; }
    }
    const indicatorColumns = [...new Set(nodes.flatMap((node) => node.data.indicators))].sort();
    const adapted = adaptAuthoredNativeWorkbenchToSemModelV4({
      model_id: activeModelId,
      model_name: activeEditableModelName,
      nodes,
      edges,
      diagram_layout: diagramLayout,
      data_binding: {
        kind: "raw",
        dataset_id: dataset.id,
        missing_data: "listwise_deletion",
        weight: null,
        cluster_variable: null,
        strata_variable: null,
      },
      group: { kind: "single_group" },
      observed_semantics: observedSemanticsForParameterTable(dataset, indicatorColumns),
    });
    if (!adapted.ok) return null;
    try { return bindForUnifiedCalculation(adapted.model); } catch { return null; }
  }, [activeEditableModelName, activeModelId, dataset, diagramLayout, edges, generalSemProjectDraftMode, nodes, projectId, strictAuthority]);
  const unifiedSemCalculation = useMemo<UnifiedSemCalculationContextV1 | null>(() => {
    const model = generalSemProjectDraftMode
      ? draftGeneralSemModel
      : strictGeneralSemAuthority
        ? strictAuthority?.model ?? null
        : draftGeneralSemModel;
    if (!model || !activeModelId) return null;
    let config = generalSemProjectDraftMode ? null : rehydratedGeneralSemExecution?.config ?? null;
    if (!config) {
      try {
        config = generalSemConfigFromEngineV1({
          ...defaultGeneralSemPlsEngineOptionsV1(),
          tolerance: analysisSettings.tolerance ?? 1e-7,
          maxIterations: analysisSettings.maxIterations ?? 1_000,
          seed: analysisSettings.seed,
          workers: analysisSettings.workers,
          confidenceLevel: analysisSettings.confidenceLevel,
          bootstrapSamples: Math.max(analysisSettings.bootstrapSamples ?? 500, 2),
        });
      } catch { return null; }
    }
    return {
      authorityKey: `${projectId ?? "draft"}:${activeModelId}:${strictAuthority?.model_document_sha256 ?? "canvas"}`,
      model,
      config,
      canonicalDocument: generalSemCanonicalResult,
    };
  }, [activeModelId, analysisSettings, draftGeneralSemModel, generalSemCanonicalResult, generalSemProjectDraftMode, projectId, rehydratedGeneralSemExecution?.config, strictAuthority, strictGeneralSemAuthority]);
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
    const modelId = surface === "results"
      ? generalSemResultSelected
        ? generalSemCanonicalResult?.provenance.model_id
        : selectedRun?.modelId
      : activeModelId;
    return Boolean(modelId && (
      projectModels.some((model) => model.id === modelId)
      || strictAuthorities[modelId]
    ));
  }, [activeModelId, generalSemCanonicalResult?.provenance.model_id, generalSemResultSelected, projectModels, selectedRun?.modelId, strictAuthorities, surface]);
  const selectedRunId = selectedRun?.id ?? "";
  const resultNavigation = useMemo(() => navigationWithPrecision(buildNativeResultNavigation(selectedRun), uiPreferences.defaultPrecision), [selectedRun, uiPreferences.defaultPrecision]);
  const resultTables = resultNavigation.tables;
  const selectedResultItem = resultNavigation.groups.flatMap((group) => group.items).find((item) => item.id === selectedTableId) ?? resultNavigation.groups[0]?.items[0];
  const selectedTable = selectedResultItem ? resultTableForItem(resultNavigation, selectedResultItem.id) : undefined;
  const modelReadiness = useMemo(
    () => nativePlsReadiness({ dataset, nodes, edges, settings: nativeAnalysisSettingsForWorkbenchKind(analysisSettings, "pls_algorithm"), nativeDesktop: isNativeDesktop() }),
    [analysisSettings, dataset, edges, nodes],
  );
  const canvasFeatureInventory = useMemo(
    () => nativeCanvasFeatureInventory(nodes, edges),
    [edges, nodes],
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
        : surface === "results" && (selectedRun || (generalSemResultSelected && generalSemCanonicalResult))
          ? { kind: "result" as const, count: 1 }
          : { kind: "none" as const, count: 0 };
    const moderationMutationAuthority: NativeModerationMutationAuthorityV1 = strictGeneralSemRevisionRequired
      ? {
        kind: "general_sem_revision",
        available: generalSemRevisionDisabledReason === null,
        ...(generalSemRevisionDisabledReason ? { disabledReason: generalSemRevisionDisabledReason } : {}),
      }
      : projectWritable
        ? { kind: "direct" }
        : {
          kind: "blocked",
          disabledReason: "This project does not permit direct model mutations.",
        };
    return {
      surface,
      projectOpen: projectName !== "No project open",
      projectWritable,
      hasDataset: dataset.columns.length > 0,
      hasCompletedRun: completedRuns.length > 0 || Boolean(generalSemCanonicalResult),
      selectedResultSaved,
      canOpenContextModel,
      canCalculate: strictGeneralSemRevisionRequired || (modelReadiness.canRun && !strictAuthority),
      generalSemCalculationAvailable: strictGeneralSemRevisionRequired,
      canUndo: past.length > 0,
      canRedo: future.length > 0,
      canRecode: Boolean(selectedColumn) && projectWritable && (dataset.kind ?? "raw") === "raw",
      canConfigureGroups: Boolean(selectedColumn)
        && (dataset.kind ?? "raw") === "raw"
        && !nodes.some((node) => node.data.indicators.includes(selectedColumn)),
      selectedVariableIsGrouping: Boolean(selectedColumn && selectedColumn === analysisSettings.groupColumn?.trim()),
      canAddModeration: Boolean(selectedEdgeId)
        && canAddNativeModeration(nodes, edges, selectedEdgeId),
      moderationMutationAuthority,
      canAddHigherOrder: canCreateNativeHigherOrder(nodes, edges),
      hasActiveModel: Boolean(activeModelId),
      calculationReady: strictGeneralSemAuthority || Boolean(generalSemProjectDraftMode),
      canConfigureConditionalProcess: Boolean(activeModelId) && canvasFeatureInventory.interactions > 0,
      canOpenAdvancedParameters: Boolean(activeModelId),
      selectedConstructPinned: Boolean(selectedNodeId && diagramLayout.constructLayouts[selectedNodeId]?.pinned),
      selectedHigherOrder: selectionCount === 1
        && Boolean(selectedNodeId && nodes.find((node) => node.id === selectedNodeId)?.data.semantic === "higher_order"),
      propertiesOpen,
      selection,
      calculationStatus: runMonitor.status,
    };
  }, [activeModelId, analysisSettings.groupColumn, canOpenContextModel, canvasFeatureInventory.interactions, completedRuns.length, dataset.columns.length, dataset.kind, diagramLayout.constructLayouts, explorerSelection, future.length, generalSemCanonicalResult, generalSemProjectDraftMode, generalSemRevisionDisabledReason, modelReadiness.canRun, nodes, past.length, projectName, projectWritable, propertiesOpen, runMonitor.status, selectedColumn, selectedEdgeId, selectedNodeId, selectedResultSaved, selectedRun, strictAuthority, strictGeneralSemAuthority, strictGeneralSemRevisionRequired, surface]);

  const dataMutationsLocked = datasetDescriptorOnly
    || generalSemPublicationPending
    || Boolean(generalSemTransientWorkBlocker)
    || isNativeCalculationActive(runMonitor.status);

  const navigate = useCallback((next: NativeSurface) => {
    if (generalSemTransientWorkBlocker && next !== surface) {
      const temporaryResult = generalSemTransientWorkBlocker === "temporary_result_pending";
      pushToast({
        tone: "warning",
        title: temporaryResult ? "Advanced result not yet secured" : "Advanced calculation in progress",
        detail: temporaryResult
          ? "Save and strictly reopen the result, or dismiss it explicitly, before leaving this calculation."
          : "Finish or cancel the advanced calculation before leaving its progress view.",
      });
      return;
    }
    setSurface(next);
    setOpenMenu(null);
    setContextMenu(null);
  }, [generalSemTransientWorkBlocker, pushToast, surface]);

  useEffect(() => {
    const onNavigate = (event: Event) => {
      const next = (event as CustomEvent<{ surface?: NativeSurface }>).detail?.surface;
      if (next && ["launcher", "data", "model", "results"].includes(next)) navigate(next);
    };
    window.addEventListener("quickpls:navigate-surface", onNavigate);
    return () => window.removeEventListener("quickpls:navigate-surface", onNavigate);

  }, [navigate]);
  useEffect(() => {
    const onCanonicalResult = (event: Event) => {
      const detail = (event as CustomEvent<{
        document?: CanonicalResultDocumentV2;
        navigate?: boolean;
      }>).detail;
      if (!detail?.document) return;
      setGeneralSemCanonicalResult(detail.document);
      setGeneralSemResultSelected(true);
      if (detail.navigate) {
        setPendingExactCbsemPlan(null);
        currentDialogRef.current = null;
        setDialog(null);
        // This event is emitted only after schema-6 append and strict fresh
        // readback. Clear the temporary-result lock synchronously so the
        // navigation cannot be rejected by the prior render's stale blocker.
        useWorkspace.getState().setGeneralSemTransientWorkBlocker(null);
        setSurface("results");
        setOpenMenu(null);
        setContextMenu(null);
      }
    };
    window.addEventListener("quickpls:general-sem-canonical-result", onCanonicalResult);
    return () => window.removeEventListener("quickpls:general-sem-canonical-result", onCanonicalResult);
  }, [navigate]);
  const openCalculation = () => {
    if (!["queued", "validating", "running", "cancelling"].includes(runMonitor.status)) {
      resetRunMonitor();
    }
    const preferredKind = surface === "data" ? "nca" : nativeWorkbenchAnalysisKindForSettings(analysisSettings);
    setAdvancedCalculationPlan(null);
    setPendingExactCbsemPlan(null);
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
      loadNamedSemEvidenceFixture: async (
        fixture: import("../data/v255NamedSemEvidenceFixtures").V255NamedSemFixture,
      ) => {
        const { v255NamedSemEvidenceFixture } = await import("../data/v255NamedSemEvidenceFixtures");
        const project = v255NamedSemEvidenceFixture(fixture);
        const current = useWorkspace.getState();
        const retainedProjectId = current.projectId;
        const retainedDraft = current.generalSemProjectDraftMode;
        loadProject({
          nodes: project.nodes,
          edges: project.edges,
          dataset: project.dataset,
          projectModels: project.projectModels,
          activeModelId: project.activeModelId,
          runs: [],
          diagramMode: "sem",
          ...(retainedDraft ? { preserveGeneralSemProjectDraftMode: retainedDraft } : {}),
        });
        setProjectMeta(`QuickPLS 2.55 named evidence: ${fixture}`, null, retainedProjectId);
        navigate("model");
        return {
          fixture,
          modelId: project.modelId,
          constructs: project.nodes.filter((node) => !node.data.semantic).length,
          derivedTerms: project.nodes.filter((node) => Boolean(node.data.semantic)).length,
          paths: project.edges.filter((edge) => !edge.data?.technicalGenerated).length,
        };
      },
      namedSemEvidenceSnapshot: () => {
        const state = useWorkspace.getState();
        const authority = state.activeModelId ? state.standardSemModelV4Authorities[state.activeModelId] ?? null : null;
        const derived = authority?.model.derived_terms ?? [];
        const canonical = generalSemCanonicalResult;
        const selectedLegacy = state.runs.find((run) => run.id === state.selectedResultRunId) ?? null;
        const cells = canonical?.capability_cells ?? (canonical ? [canonical.provenance.capability_cell] : []);
        const executionCell = canonical?.general_sem_results?.cbsem_bootstrap_receipt?.capability_cell
          ?? canonical?.general_sem_results?.inference_receipt?.capability_cell
          ?? canonical?.general_sem_results?.higher_order_inference_receipt?.capability_cell
          ?? canonical?.provenance.capability_cell
          ?? null;
        return {
          model: {
            model_id: authority?.model.id ?? state.activeModelId,
            model_document_sha256: authority?.model_document_sha256 ?? null,
            ordinary_construct_count: authority
              ? authority.model.variables.filter((variable) => variable.kind === "composite" || variable.kind === "common_factor").length
              : state.nodes.filter((node) => !node.data.semantic).length,
            common_factor_count: authority?.model.variables.filter((variable) => variable.kind === "common_factor").length ?? 0,
            structural_relation_count: authority?.model.relations.filter((relation) => relation.kind === "structural").length
              ?? state.edges.filter((edge) => edge.data?.role !== "covariance" && !edge.data?.technicalGenerated).length,
            interaction_orders: derived
              .flatMap((term) => term.kind === "interaction_v2" ? [term.operands.length] : term.kind === "interaction" ? [2] : [])
              .sort((left, right) => left - right),
            higher_order_measurement_types: derived
              .flatMap((term) => term.kind === "higher_order" ? [term.measurement_type] : [])
              .sort(),
            advanced_equality_labels: authority?.model.parameters
              .flatMap((parameter) => parameter.kind === "free" && parameter.equality_label ? [parameter.equality_label] : [])
              .sort() ?? [],
            advanced_equalities: authority?.model.parameters
              .flatMap((parameter) => parameter.kind === "free" && parameter.equality_label
                ? [{ parameter_id: parameter.id, equality_label: parameter.equality_label }]
                : [])
              .sort((left, right) => left.parameter_id.localeCompare(right.parameter_id)) ?? [],
          },
          canonical_result: canonical ? {
            document_id: canonical.document_id,
            run_id: canonical.provenance.run_id,
            model_id: canonical.provenance.model_id,
            method_version: canonical.provenance.method_version,
            primary_cell_id: canonical.provenance.capability_cell.cell_id,
            execution_cell_id: executionCell?.cell_id ?? null,
            capability_cell_ids: cells.map((cell) => cell.cell_id).sort(),
            tables: canonical.tables.map((table) => ({
              id: table.id,
              rows: table.rows.length,
              columns: table.columns.length,
            })).sort((left, right) => left.id.localeCompare(right.id)),
            specific_indirect_count: canonical.general_sem_results?.specific_indirect_effects?.length ?? 0,
            interaction_effect_count: canonical.general_sem_results?.interaction_effects?.length ?? 0,
            conditional_slope_count: canonical.general_sem_results?.conditional_effects?.length ?? 0,
            conditional_probe_contracts: canonical.general_sem_results?.conditional_effect_probes?.map((probe) => ({
              moderator_id: probe.moderator_id,
              kind: probe.values.kind,
              values: probe.values.kind === "explicit"
                ? [...probe.values.values]
                : ["-1 SD", "Mean", "+1 SD"],
            })).sort((left, right) => left.moderator_id.localeCompare(right.moderator_id)) ?? [],
            three_way_effect_count: canonical.general_sem_results?.three_way_interaction_effects?.length ?? 0,
            three_way_conditional_effect_count: canonical.general_sem_results?.three_way_conditional_interaction_effects?.length ?? 0,
            three_way_simple_slope_count: canonical.general_sem_results?.three_way_simple_slopes?.length ?? 0,
            conditional_indirect_count: canonical.general_sem_results?.conditional_indirect_effects?.length ?? 0,
            moderated_mediation_index_count: canonical.general_sem_results?.moderated_mediation_indices?.length ?? 0,
            higher_order_stage_count: canonical.general_sem_results?.higher_order_stages?.length ?? 0,
          } : null,
          legacy_result: selectedLegacy ? {
            result_id: selectedLegacy.id,
            method: selectedLegacy.method,
            method_version: selectedLegacy.result?.method_version ?? null,
            status: selectedLegacy.status,
            mediation_specific_count: selectedLegacy.result?.mediation?.estimates.length ?? 0,
            moderation_effect_count: selectedLegacy.result?.moderation?.estimates.length ?? 0,
            regression_type: selectedLegacy.result?.regression?.regression_type ?? null,
            process_model: selectedLegacy.result?.regression?.process?.model ?? null,
          } : null,
        };
      },
      exerciseNamedAdvancedParameterRevision: async () => {
        const before = useWorkspace.getState();
        const modelId = before.activeModelId;
        const authority = modelId ? before.standardSemModelV4Authorities[modelId] ?? null : null;
        if (!modelId || !authority) throw new Error("A strict Standard SemModelV4 authority is required.");
        const parameter = authority.model.parameters.find((candidate) => candidate.kind === "free");
        if (!parameter || parameter.kind !== "free") throw new Error("No free parameter is available for the Advanced Parameter Table revision.");
        const result = await before.commitStandardSemModelV4Intent({
          kind: "set_parameter_specification",
          parameter_id: parameter.id,
          specification: {
            kind: "free",
            start: parameter.start ?? 0.125,
            lower: parameter.lower ?? null,
            upper: parameter.upper ?? null,
            equality_label: "V255Evidence",
          },
          label: parameter.label,
        });
        if (result.status !== "committed") {
          const detail = result.status === "blocked"
            ? result.diagnostic.message
            : result.status === "stale"
              ? "the active model authority changed before the revision committed"
              : result.error instanceof Error
                ? result.error.message
                : String(result.error);
          throw new Error(`Advanced Parameter Table revision was not committed: ${detail}`);
        }
        const after = useWorkspace.getState().standardSemModelV4Authorities[modelId];
        const revised = after?.model.parameters.find((candidate) => candidate.id === parameter.id);
        if (!after || revised?.kind !== "free" || revised.equality_label !== "V255Evidence") {
          throw new Error("The Advanced Parameter Table revision did not persist in the active authority.");
        }
        return {
          model_id: modelId,
          parameter_id: parameter.id,
          before_model_document_sha256: authority.model_document_sha256,
          after_model_document_sha256: after.model_document_sha256,
          equality_label: revised.equality_label,
          stable_parameter_id: revised.id === parameter.id,
          changed_authority: after.model_document_sha256 !== authority.model_document_sha256,
        };
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
  }, [addRun, completedRuns.length, generalSemCanonicalResult, loadProject, navigate, setProjectMeta]);

  const startCalculation = (dataProfile?: NativeLogisticProfile | NativeProcessProfile) => {
    if (!calculationReadiness.canRun || ["queued", "validating", "running", "cancelling"].includes(runMonitor.status)) return;
    setAnalysisSettings(calculationSettings);
    commandEvent("run-analysis", createNativeCalculationRequest(calculationKind, calculationSettings, dataProfile));
  };

  const prepareCalculationReadyRevision = async (
    preferredKind: "pls_algorithm" | "pls_bootstrap" | "cbsem" = "pls_algorithm",
    plan: UnifiedSemCalculationPlanV1 | null = null,
    destination: "advanced-calculation" | "advanced-parameters" = "advanced-calculation",
  ) => {
    if (!isNativeDesktop() || !projectId || !activeModelId) {
      pushToast({
        tone: "warning",
        title: "Revision unavailable",
        detail: "Open a native QuickPLS project with a dataset and model before preparing advanced methods.",
      });
      return;
    }
    try {
      const authorizedProjectId = await authorizeNativeGeneralSemRevisionDraftV1();
      if (authorizedProjectId !== projectId || !beginGeneralSemProjectRevisionDraftMode(projectId)) {
        await invalidateNativeGeneralSemFreshDraftAuthorityV1();
        throw new Error("The active project changed before its calculation-ready revision could be prepared.");
      }
      setAdvancedCalculationKind(preferredKind);
      setAdvancedCalculationPlan(plan);
      pushToast({
        tone: "info",
        title: "Calculation-ready revision prepared",
        detail: "Review the detected model and choose Save and activate. The current project remains unchanged.",
      });
      openDialog(destination);
    } catch (error) {
      await invalidateNativeGeneralSemFreshDraftAuthorityV1().catch(() => undefined);
      pushToast({
        tone: "error",
        title: "Revision could not be prepared",
        detail: error instanceof Error ? error.message : String(error),
      });
    }
  };

  const handleUnifiedSemCalculationAction = (action: UnifiedSemCalculationActionV1) => {
    if (action.kind === "edit_higher_order") {
      openHigherOrderDialog({ kind: "edit", constructId: action.higherOrderConstructId });
      return;
    }
    if (action.kind === "open_advanced_parameter_table") {
      setAdvancedCalculationKind(action.plan.method);
      const exactCompatibility = action.plan.route === "exact_cbsem_compatibility";
      setPendingExactCbsemPlan(exactCompatibility ? action.plan : null);
      setAdvancedCalculationPlan(exactCompatibility ? null : action.plan);
      if (!generalSemProjectDraftMode) {
        void prepareCalculationReadyRevision(
          action.plan.method,
          exactCompatibility ? null : action.plan,
          "advanced-parameters",
        );
      } else openDialog("advanced-parameters");
      return;
    }
    if (action.kind === "configure_moderated_mediation") {
      if (!generalSemProjectDraftMode) {
        void prepareCalculationReadyRevision("pls_bootstrap");
      } else {
        setAdvancedCalculationKind("pls_bootstrap");
        setAdvancedCalculationPlan(null);
        openDialog("advanced-calculation");
      }
      return;
    }
    if (action.plan.route === "exact_cbsem_compatibility") {
      setAdvancedCalculationKind("cbsem");
      setAdvancedCalculationPlan(action.plan);
      openDialog("advanced-calculation");
      return;
    }
    const requestedEstimator = action.plan.method === "cbsem"
      ? GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1
      : GENERAL_SEM_PLS_ESTIMATOR_ID_V1;
    const residentConfigChanged = Boolean(
      action.plan.requestedConfig
      && rehydratedGeneralSemExecution
      && JSON.stringify(action.plan.requestedConfig) !== JSON.stringify(rehydratedGeneralSemExecution.config),
    );
    if (strictGeneralSemAuthority
      && (rehydratedGeneralSemExecution?.estimatorId !== requestedEstimator || residentConfigChanged)) {
      void prepareCalculationReadyRevision(action.plan.method, action.plan);
      return;
    }
    if (!strictGeneralSemAuthority && !generalSemProjectDraftMode) {
      void prepareCalculationReadyRevision(action.plan.method, action.plan);
      return;
    }
    setAdvancedCalculationKind(action.plan.method);
    setAdvancedCalculationPlan(action.plan);
    openDialog("advanced-calculation");
  };

  const createProject = () => {
    const name = newProjectName.trim() || "Untitled project";
    const projectMode = isNativeDesktop()
      && generalSemWorkspaceProductAccessV1(uiPreferences.experimentalLabsEnabled)
      ? "general_sem_v1"
      : "standard";
    commandEvent("new-project", { name, projectMode });
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

  const rejectLockedDataMutation = (operation: string) => {
    if (projectWritable && !dataMutationsLocked) return false;
    const detail = generalSemPublicationPending
      ? `${operation} is locked until General SEM archive publication finishes.`
      : datasetDescriptorOnly
        ? `${operation} is unavailable for an immutable, archive-bound General SEM dataset.`
        : `${operation} requires a writable project and no active calculation.`;
    pushToast({ tone: "warning", title: "Dataset change blocked", detail });
    return true;
  };

  const currentGeneralSemRevisionDisabledReason = () => {
    const authorityState = useInternalProjectArchiveV6Session.getState();
    const workspaceState = useWorkspace.getState();
    return nativeGeneralSemRevisionCommandDisabledReasonV1({
      standardActivationPending: authorityState.standardActivationPending,
      revisionForkPending: authorityState.revisionForkPending,
      saveCopyPending: authorityState.saveCopyPending,
      sessionDirty: authorityState.dirty,
      publicationPending: workspaceState.generalSemPublicationPending,
      transientWorkBlocker: workspaceState.generalSemTransientWorkBlocker,
      calculationStatus: workspaceState.runMonitor.status,
    });
  };

  const launchHigherOrderRevision = async (
    intent: Extract<StandardSemModelV4EditorIntentV1, { kind: "add_higher_order" | "replace_higher_order" }>,
    operation: "created" | "updated",
  ): Promise<NativeHigherOrderDialogCommitResult> => {
    const disabledReason = currentGeneralSemRevisionDisabledReason();
    if (disabledReason) return { status: "blocked", detail: disabledReason };
    pushToast({
      tone: "info",
      title: "Save calculation-ready revision",
      detail: `Choose a new .qpls filename. QuickPLS will preserve the current archive and ${operation === "created" ? "add" : "update"} the HOC in one revision.`,
    });
    try {
      const result = await useInternalProjectArchiveV6Session.getState()
        .reviseGeneralSemExecutionAuthority({ intent });
      const state = useInternalProjectArchiveV6Session.getState();
      const detail = result === "cancelled"
        ? state.revisionForkStatusMessage
        : state.revisionForkFailure
          ? `${state.revisionForkFailure.message} ${state.revisionForkFailure.correctiveAction}`
          : state.revisionForkStatusMessage;
      if (result === "saved") {
        pushToast({
          tone: "success",
          title: `Higher-order construct ${operation}`,
          detail: state.revisionForkStatusMessage,
        });
        return { status: "applied", constructId: intent.output_id };
      }
      if (result === "cancelled") {
        pushToast({
          tone: "info",
          title: "Higher-order revision cancelled",
          detail: state.revisionForkStatusMessage,
        });
        return { status: "cancelled", detail };
      }
      if (result === "stale") {
        pushToast({
          tone: "warning",
          title: "Higher-order revision stale",
          detail,
        });
        return { status: "stale", detail };
      }
      pushToast({
        tone: "error",
        title: "Higher-order revision blocked",
        detail,
      });
      return { status: "blocked", detail };
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      pushToast({ tone: "error", title: "Higher-order revision rejected", detail });
      return { status: "rejected", detail };
    }
  };

  const commitHigherOrderDialogSubmission = async (
    submission: NativeHigherOrderDialogSubmission,
  ): Promise<NativeHigherOrderDialogCommitResult> => {
    const draft = submission.draft;
    const approach = nativeHigherOrderDraftApproach(draft);
    const measurementType = nativeHigherOrderDraftMeasurementType(draft);
    const gatewayDraft = {
      name: draft.name.trim(),
      shortName: draft.shortName.trim(),
      components: [...draft.components],
      approach,
      measurementType,
    };
    if (submission.kind === "edit") {
      if (strictGeneralSemRevisionRequired) {
        const intent: Extract<
          StandardSemModelV4EditorIntentV1,
          { kind: "replace_higher_order" }
        > = {
          kind: "replace_higher_order",
          term_id: submission.termId,
          output_id: submission.outputId,
          label: draft.name.trim(),
          components: [...draft.components],
          approach,
          measurement_type: measurementType,
        };
        return launchHigherOrderRevision(intent, "updated");
      }
      const result = await commitGatewayDesktopCommand({
        kind: "edit_higher_order",
        termId: submission.termId,
        outputId: submission.outputId,
        draft: gatewayDraft,
      }, "Higher-order construct");
      return result.status === "applied"
        ? { status: "applied", constructId: submission.outputId }
        : dialogFailureForGatewayResult(result);
    }

    const termId = nextStrictIntentId("higher-order-term");
    const outputId = strictAuthority ? nextStrictIntentId("higher-order-output") : termId;
    const initialRelationshipId = draft.initialPath ? nextStrictIntentId("higher-order-path") : undefined;
    const gatewayCreateDraft = {
      ...gatewayDraft,
      ...(draft.initialPath && initialRelationshipId ? {
        initialPath: {
          ...draft.initialPath,
          relationshipId: initialRelationshipId,
          label: draft.initialPath.direction === "hoc_to_construct"
            ? `${draft.name.trim()} effect`
            : `${draft.name.trim()} antecedent`,
        },
      } : {}),
    };
    if (strictGeneralSemRevisionRequired) {
      const intent: Extract<
        StandardSemModelV4EditorIntentV1,
        { kind: "add_higher_order" }
      > = {
        kind: "add_higher_order",
        term_id: termId,
        output_id: outputId,
        label: gatewayCreateDraft.name,
        components: gatewayCreateDraft.components,
        approach: gatewayCreateDraft.approach,
        measurement_type: gatewayCreateDraft.measurementType,
        initial_path: gatewayCreateDraft.initialPath
          ? {
              relation_id: gatewayCreateDraft.initialPath.relationshipId,
              source: gatewayCreateDraft.initialPath.direction === "hoc_to_construct"
                ? outputId
                : gatewayCreateDraft.initialPath.constructId,
              target: gatewayCreateDraft.initialPath.direction === "hoc_to_construct"
                ? gatewayCreateDraft.initialPath.constructId
                : outputId,
              label: gatewayCreateDraft.initialPath.label ?? "Path",
            }
          : undefined,
      };
      return launchHigherOrderRevision(intent, "created");
    }
    const result = await commitGatewayDesktopCommand({
      kind: "create_higher_order",
      termId,
      outputId,
      draft: gatewayCreateDraft,
    }, "Higher-order construct");
    return result.status === "applied"
      ? { status: "applied", constructId: outputId }
      : dialogFailureForGatewayResult(result);
  };

  const removeHigherOrderConstruct = (termId: string, outputId: string) => {
    if (strictGeneralSemRevisionRequired) {
      const disabledReason = currentGeneralSemRevisionDisabledReason();
      if (disabledReason) {
        pushToast({ tone: "warning", title: "Higher-order removal unavailable", detail: disabledReason });
        return;
      }
      pushToast({
        tone: "info",
        title: "Save calculation-ready revision",
        detail: "Choose a new .qpls filename. QuickPLS will preserve the current archive and remove the HOC in one revision.",
      });
      void useInternalProjectArchiveV6Session.getState()
        .reviseGeneralSemExecutionAuthority({ intent: { kind: "remove_higher_order", term_id: termId, output_id: outputId } })
        .then((result) => {
          const state = useInternalProjectArchiveV6Session.getState();
          pushToast(result === "saved"
            ? { tone: "success", title: "Higher-order construct removed", detail: state.revisionForkStatusMessage }
            : result === "cancelled"
              ? { tone: "info", title: "Higher-order revision cancelled", detail: state.revisionForkStatusMessage }
              : {
                  tone: "error",
                  title: "Higher-order removal blocked",
                  detail: state.revisionForkFailure
                    ? `${state.revisionForkFailure.message} ${state.revisionForkFailure.correctiveAction}`
                    : state.revisionForkStatusMessage,
                });
        });
      return;
    }
    commitGatewayDesktopCommand({ kind: "remove_higher_order", termId, outputId }, "Higher-order construct");
  };

  type ModerationAuthorityIntentV3 = Extract<
    StandardSemModelV4EditorIntentV1,
    { kind: "add_moderating_effect_v3" | "replace_moderating_effect" | "remove_moderating_effect" }
  >;

  const moderationIntentIdentity = (
    target: ModeratingEffectTargetV1,
    operands: readonly [string, string] | readonly [string, string, string],
  ) => standardSemGeneralSemModerationV3IdentityV1(target, operands);

  const launchModerationRevision = async (
    intent: ModerationAuthorityIntentV3,
    operation: "created" | "updated" | "removed",
  ): Promise<NativeModerationDialogCommitResult> => {
    const disabledReason = currentGeneralSemRevisionDisabledReason();
    if (disabledReason) return { status: "blocked", reason: disabledReason };
    pushToast({
      tone: "info",
      title: "Save calculation-ready revision",
      detail: `Choose a new .qpls filename. QuickPLS will preserve the current archive and ${operation === "created" ? "add" : operation === "updated" ? "update" : "remove"} the moderating effect in one revision.`,
    });
    try {
      const result = await useInternalProjectArchiveV6Session.getState()
        .reviseGeneralSemExecutionAuthority({ intent });
      const state = useInternalProjectArchiveV6Session.getState();
      const detail = result === "cancelled"
        ? state.revisionForkStatusMessage
        : state.revisionForkFailure
          ? `${state.revisionForkFailure.message} ${state.revisionForkFailure.correctiveAction}`
          : state.revisionForkStatusMessage;
      if (result === "saved") {
        pushToast({
          tone: "success",
          title: `Moderating effect ${operation}`,
          detail: state.revisionForkStatusMessage,
        });
        if (intent.kind === "add_moderating_effect_v3") {
          const identity = moderationIntentIdentity(intent.target, intent.operands);
          return { status: "created", interactionId: identity.outputId };
        }
        return { status: "updated", interactionTermId: intent.term_id };
      }
      if (result === "cancelled") {
        pushToast({
          tone: "info",
          title: "Moderation revision cancelled",
          detail: state.revisionForkStatusMessage,
        });
        return { status: "cancelled", reason: detail };
      }
      if (result === "stale") {
        pushToast({
          tone: "warning",
          title: "Moderation revision stale",
          detail,
        });
        return { status: "stale", reason: detail };
      }
      pushToast({
        tone: "error",
        title: "Moderation revision blocked",
        detail,
      });
      return { status: "blocked", reason: detail };
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      pushToast({ tone: "error", title: "Moderation revision rejected", detail });
      return { status: "rejected", reason: detail };
    }
  };

  const commitModerationDialogSubmission = async (
    submission: NativeModerationDialogSubmissionV1,
  ): Promise<NativeModerationDialogCommitResult> => {
    const parent = submission.target.kind === "parent_interaction"
      ? nativeModeratingEffect(nodes, edges, submission.target.interactionTermId)
      : undefined;
    if (submission.order === 3 && (!parent || parent.order !== 2 || !parent.moderatorIds[0])) {
      return { status: "blocked", reason: "The selected parent two-way interaction is no longer available." };
    }
    const operands = submission.order === 3
      ? [parent!.predictor, parent!.moderatorIds[0]!, submission.moderatorId] as const
      : [submission.predictorId, submission.moderatorId] as const;
    if (!strictAuthority && (submission.mode !== "create" || submission.order !== 2)) {
      return {
        status: "blocked",
        reason: "Create a calculation-ready revision before editing or adding three-way moderation to this older project.",
      };
    }
    const labelFor = (id: string) => nodes.find((node) => node.id === id)?.data.label.trim() || id;
    const common = {
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1" as const,
      label: operands.map(labelFor).join(" × "),
      operands,
      target: submission.target,
      outcome: submission.outcomeId,
      method: "two_stage" as const,
      hierarchy_policy: "strong" as const,
    };
    const editingResident = submission.mode === "edit" && submission.interactionTermId
      ? nativeModeratingEffect(nodes, edges, submission.interactionTermId)
      : undefined;
    if (submission.mode === "edit" && !editingResident) {
      return { status: "blocked", reason: "The selected moderating effect is no longer available." };
    }
    const intent: ModerationAuthorityIntentV3 = submission.mode === "edit"
      ? {
        kind: "replace_moderating_effect",
        term_id: editingResident!.interactionTermId,
        output_id: editingResident!.interactionNodeId,
        ...common,
      }
      : { kind: "add_moderating_effect_v3", ...common };
    if (strictGeneralSemRevisionRequired) {
      return launchModerationRevision(intent, submission.mode === "edit" ? "updated" : "created");
    }
    const effect = {
      label: common.label,
      operands: [...common.operands] as [string, string] | [string, string, string],
      target: common.target,
      outcomeId: common.outcome,
    };
    if (intent.kind === "replace_moderating_effect") {
      const result = await commitGatewayDesktopCommand({
        kind: "edit_moderating_effect",
        termId: intent.term_id,
        outputId: intent.output_id,
        effect,
      }, "Moderating effect");
      if (result.status === "applied") return { status: "updated", interactionTermId: intent.term_id };
      const failure = dialogFailureForGatewayResult(result);
      return { status: failure.status, reason: failure.detail };
    }
    const result = await commitGatewayDesktopCommand({ kind: "create_moderating_effect", effect }, "Moderating effect");
    if (result.status === "applied") {
      return { status: "created", interactionId: moderationIntentIdentity(intent.target, intent.operands).outputId };
    }
    const failure = dialogFailureForGatewayResult(result);
    return { status: failure.status, reason: failure.detail };
  };

  const removeModeratingEffect = (requestedTermId: string) => {
    const resident = nativeModeratingEffect(nodes, edges, requestedTermId);
    if (!resident) {
      pushToast({ tone: "warning", title: "Moderating effect unavailable", detail: "Refresh the Canvas and select the effect again." });
      return;
    }
    const intent: ModerationAuthorityIntentV3 = {
      kind: "remove_moderating_effect",
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1",
      term_id: resident.interactionTermId,
      output_id: resident.interactionNodeId,
    };
    if (strictGeneralSemRevisionRequired) {
      launchModerationRevision(intent, "removed");
      return;
    }
    commitGatewayDesktopCommand({
      kind: "remove_moderating_effect",
      termId: resident.interactionTermId,
      outputId: resident.interactionNodeId,
    }, "Moderating effect");
  };

  const dispatchNativeAction = (action: NativeCommandAction, target?: NativeDataContextTarget) => {
    switch (action.id) {
      case "project.new": openDialog("new-project"); return;
      case "project.open": commandEvent("open-project"); return;
      case "project.open-demo": commandEvent("open-demo-project"); return;
      case "project.import-data":
        if (!rejectLockedDataMutation("Import data")) openDialog("import-data");
        return;
      case "data.recode": {
        if (rejectLockedDataMutation("Recode data")) return;
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
          const resultModelId = generalSemResultSelected
            ? generalSemCanonicalResult?.provenance.model_id
            : selectedRun?.modelId;
          if (resultModelId && (
            projectModels.some((model) => model.id === resultModelId)
            || strictAuthorities[resultModelId]
          )) {
            commandEvent("open-explorer-model", { modelId: resultModelId });
          }
          return;
        }
        if (action.surface === "results" && generalSemCanonicalResult) {
          setGeneralSemResultSelected(true);
        }
        navigate(action.surface);
        return;
      }
      case "model.undo": commandEvent("model-undo"); return;
      case "model.redo": commandEvent("model-redo"); return;
      case "model.set-tool": commandEvent("model-tool", { tool: action.tool }); return;
      case "model.add-construct": commandEvent("model-add-construct"); return;
      case "model.add-higher-order":
        openHigherOrderDialog({
          kind: "create",
          selectedComponentIds: nodes.filter((node) => node.selected || node.id === selectedNodeId).map((node) => node.id),
          requireInitialPath: strictGeneralSemRevisionRequired,
        });
        return;
      case "model.add-moderating-effect":
        if (strictGeneralSemRevisionRequired) {
          const disabledReason = currentGeneralSemRevisionDisabledReason();
          if (!disabledReason) {
            openModerationDialog({
              kind: "create",
              ...(selectedEdgeId ? { target: { kind: "focal_relation", relationId: selectedEdgeId } } : {}),
            });
            return;
          }
          pushToast({
            tone: "warning",
            title: "Calculation-ready revision unavailable",
            detail: disabledReason,
          });
          return;
        }
        openModerationDialog({
          kind: "create",
          ...(selectedEdgeId ? { target: { kind: "focal_relation", relationId: selectedEdgeId } } : {}),
        });
        return;
      case "model.prepare-calculation-ready":
        void prepareCalculationReadyRevision();
        return;
      case "model.open-conditional-process":
        setCalculationKind("pls_bootstrap");
        setAdvancedCalculationPlan(null);
        setCalculationDraft(nativeAnalysisSettingsForWorkbenchKind(analysisSettings, "pls_bootstrap"));
        openDialog("calculation");
        return;
      case "model.open-advanced-parameters":
        setPendingExactCbsemPlan(null);
        if (strictGeneralSemRevisionRequired) void prepareCalculationReadyRevision("pls_algorithm", null, "advanced-parameters");
        else if (strictGeneralSemAuthority || generalSemProjectDraftMode) openDialog("advanced-parameters");
        else void prepareCalculationReadyRevision("pls_algorithm", null, "advanced-parameters");
        return;
      case "model.edit-selection": {
        const selectedHigherOrder = selectedNodeId
          ? nodes.find((node) => node.id === selectedNodeId && node.data.semantic === "higher_order")
          : null;
        if (selectedHigherOrder) {
          openHigherOrderDialog({ kind: "edit", constructId: selectedHigherOrder.id });
          return;
        }
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
      case "model.fit": commandEvent("model-fit", { scope: action.scope }); return;
      case "model.toggle-pin": commandEvent("model-toggle-pin"); return;
      case "model.focus-selection": commandEvent("model-focus-selection"); return;
      case "calculation.open": openCalculation(); return;
      case "calculation.cancel": commandEvent("cancel-analysis"); return;
      case "results.export":
        if (generalSemResultSelected && generalSemCanonicalResult) {
          document.getElementById("nd-canonical-export-v2-heading")?.scrollIntoView({ behavior: "smooth", block: "start" });
          document.getElementById("nd-canonical-export-v2-chart")?.focus();
          return;
        }
        openDialog("export"); return;
      case "results.open-run-details": openDialog("run-details"); return;
      case "view.toggle-properties": setPropertiesOpen((value) => !value); return;
      case "utility.open": openDialog(({ "method-scope": "trust", preferences: "settings", shortcuts: "shortcuts", about: "about" } as const)[action.utility]); return;
    }
  };
  const arrangeMenuItems: readonly MenuItem[] = NATIVE_CANVAS_ARRANGE_MENU_V1.map((item) => ({
    id: item.id,
    label: item.label,
    separator: item.separatorBefore,
    action: () => dispatchNativeAction(item.action),
  }));
  const fitMenuItems: readonly MenuItem[] = NATIVE_CANVAS_FIT_MENU_V1.map((item) => ({
    id: item.id,
    label: item.label,
    separator: item.separatorBefore,
    action: () => dispatchNativeAction(item.action),
  }));
  const commands: DesktopCommand[] = nativeCommandsFor({ kind: "toolbar", surface }, commandContext).map((command) => ({
    id: command.id,
    label: command.label,
    icon: nativeCommandIcons[command.id],
    disabled: !command.enabled,
    disabledReason: command.disabledReason,
    pressed: command.action.id === "model.set-tool" ? diagramTool === command.action.tool : undefined,
    primary: command.toolbar?.some((placement) => placement.surface === surface && placement.primary),
    action: () => dispatchNativeAction(command.action),
    ...(command.id === "arrange-model" ? { menuItems: arrangeMenuItems } : {}),
    ...(command.id === "fit-model" ? { menuItems: fitMenuItems } : {}),
  }));

  const menus: Record<string, MenuItem[]> = {};
  for (const [label, menu] of menuGroups) {
    const items = nativeCommandsFor({ kind: "menu", menu }, commandContext).map((command) => ({
      id: command.id,
      label: command.label,
      shortcut: formatNativeShortcut(command.shortcut),
      disabled: !command.enabled,
      disabledReason: command.disabledReason,
      separator: command.menu?.separatorBefore,
      action: () => dispatchNativeAction(command.action),
    }));
    if (items.length) menus[label] = items;
  }

  const contextModerationTarget = contextMenu?.modelTarget?.kind === "moderation"
    ? contextMenu.modelTarget
    : null;
  const contextModerationEffect = contextModerationTarget
    ? nativeModeratingEffect(nodes, edges, contextModerationTarget.interactionTermId)
    : undefined;
  const canExtendContextModeration = contextModerationEffect?.order === 2
    && !nodes.some((node) => node.data.interaction?.kind === "interaction_v2"
      && node.data.interaction.operands.length === 3);
  const contextConstructTarget = contextMenu?.modelTarget?.kind === "construct"
    ? contextMenu.modelTarget
    : null;
  const contextConstruct = contextConstructTarget
    ? nodes.find((node) => node.id === contextConstructTarget.id)
    : undefined;
  const contextConstructIsHigherOrder = Boolean(
    contextConstruct?.data.semantic === "higher_order" && contextConstruct.data.higherOrder,
  );
  const contextHigherOrderAuthorityCommand = contextConstructIsHigherOrder
    ? nativeCommandsFor({ kind: "menu", menu: "model" }, {
        ...commandContext,
        selection: contextMenu?.selection ?? commandContext.selection,
        selectedHigherOrder: true,
      }).find((command) => command.id === "edit-higher-order")
    : undefined;
  const executeContextModelEdit = (command: ModelEditCommandV1) => {
    void executeModelEditCommand(command).then((result) => pushToast(result.status === "applied"
      ? { tone: "success", title: "Model updated", detail: "Applied as one undoable transaction." }
      : { tone: "warning", title: "Model edit unavailable", detail: `${result.message} ${result.correctiveAction}` }));
  };
  const contextMenuItems: MenuItem[] = contextModerationTarget
    ? [...(canExtendContextModeration ? [{
      id: "extend-moderating-effect",
      label: "Add Second Moderator…",
      action: () => openModerationDialog({
        kind: "create",
        target: { kind: "parent_interaction", interactionTermId: contextModerationTarget.interactionTermId },
      }),
    } satisfies MenuItem] : []), {
      id: "edit-moderating-effect",
      label: "Edit Moderating Effect…",
      action: () => openModerationDialog({ kind: "edit", interactionTermId: contextModerationTarget.interactionTermId }),
    }, {
      id: "remove-moderating-effect",
      label: "Remove Moderating Effect",
      separator: true,
      action: () => removeModeratingEffect(contextModerationTarget.interactionTermId),
    }]
    : [
      ...nativeContextMenuCommands(
        contextMenu?.canAddModeration === undefined ? commandContext : { ...commandContext, canAddModeration: contextMenu.canAddModeration },
        contextMenu?.selection,
      ).filter((command) => !contextConstructIsHigherOrder
        || (command.id !== "edit-selection" && command.id !== "delete-selection"))
        .map((command, index) => ({
          id: command.id,
          label: command.label,
          shortcut: formatNativeShortcut(command.shortcut),
          disabled: !command.enabled,
          disabledReason: command.disabledReason,
          separator: index > 0 && Boolean(command.contextMenu?.find((placement) => placement.surface === surface)?.separatorBefore),
          action: () => dispatchNativeAction(command.action, contextMenu?.target),
        })),
      ...(contextConstructTarget && contextConstructIsHigherOrder ? [{
        id: "edit-higher-order-construct",
        label: "Edit Higher-Order Construct…",
        separator: true,
        disabled: contextHigherOrderAuthorityCommand ? !contextHigherOrderAuthorityCommand.enabled : true,
        disabledReason: contextHigherOrderAuthorityCommand?.disabledReason,
        action: () => openHigherOrderDialog({ kind: "edit", constructId: contextConstruct!.id }),
      }, {
        id: "remove-higher-order-construct",
        label: "Remove Higher-Order Construct",
        disabled: contextHigherOrderAuthorityCommand ? !contextHigherOrderAuthorityCommand.enabled : true,
        disabledReason: contextHigherOrderAuthorityCommand?.disabledReason,
        action: () => removeHigherOrderConstruct(contextConstruct!.data.higherOrder!.id, contextConstruct!.id),
      }] satisfies MenuItem[] : contextConstructTarget ? [{
        id: "indicator-position",
        label: "Indicator Position…",
        separator: true,
        action: () => {
          setPropertiesOpen(true);
          window.setTimeout(() => window.dispatchEvent(new CustomEvent("quickpls:model-inspector-show-appearance")), 0);
        },
      }, {
        id: "reset-indicator-position",
        label: "Reset Indicator Positions",
        action: () => executeContextModelEdit({ kind: "reset_indicator_layout", constructId: contextConstructTarget.id }),
      }, {
        id: "invert-measurement-model",
        label: "Invert Measurement Model",
        separator: true,
        action: () => executeContextModelEdit({ kind: "invert_measurement_model", constructId: contextConstructTarget.id }),
      }] satisfies MenuItem[] : []),
    ];

  const showWorkspaceContextMenu = (
    requestedX: number,
    requestedY: number,
    returnFocus: HTMLElement | null,
    selection: NativeCommandContext["selection"] = commandContext.selection,
    target?: NativeDataContextTarget,
    canAddModerationOverride?: boolean,
    modelTarget?: ModelCanvasContextMenuTarget,
  ) => {
    const availableCommands = modelTarget?.kind === "moderation"
      ? [{ id: "edit" }, { id: "remove" }]
      : nativeContextMenuCommands(
        canAddModerationOverride === undefined
          ? commandContext
          : { ...commandContext, canAddModeration: canAddModerationOverride },
        selection,
      );
    const extraModelCommands = modelTarget?.kind === "construct" ? 3 : 0;
    if (!availableCommands.length && !extraModelCommands) return false;
    const position = contextMenuCoordinates(
      requestedX,
      requestedY,
      window.innerWidth,
      window.innerHeight,
      244,
      Math.min(320, 10 + (availableCommands.length + extraModelCommands) * 29),
    );
    setOpenMenu(null);
    setContextMenu({ ...position, returnFocus, selection: { ...selection }, target, modelTarget, canAddModeration: canAddModerationOverride });
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
    if (request.target.kind === "moderation") {
      showWorkspaceContextMenu(
        request.clientX,
        request.clientY,
        request.returnFocus,
        { kind: "path", count: 1 },
        undefined,
        false,
        request.target,
      );
      return;
    }
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
      request.target,
    );
  };

  useEffect(() => {
    const onModerationRequest = (event: Event) => {
      const request = (event as CustomEvent<ModerationCanvasRequestV1>).detail;
      if (!request) return;
      if (request.action === "remove") {
        removeModeratingEffect(request.interactionTermId);
        return;
      }
      if (request.action === "edit") {
        openModerationDialog({ kind: "edit", interactionTermId: request.interactionTermId });
        return;
      }
      if (request.action !== "create") return;
      openModerationDialog({
        kind: "create",
        target: request.target,
        ...(request.moderatorId ? { moderatorId: request.moderatorId } : {}),
      });
    };
    window.addEventListener(MODERATION_CANVAS_REQUEST_EVENT, onModerationRequest);
    return () => window.removeEventListener(MODERATION_CANVAS_REQUEST_EVENT, onModerationRequest);
  });

  useEffect(() => {
    const onHigherOrderEditRequest = (event: Event) => {
      const constructId = (event as CustomEvent<{ constructId?: string }>).detail?.constructId;
      if (constructId) openHigherOrderDialog({ kind: "edit", constructId });
    };
    window.addEventListener("quickpls:edit-higher-order", onHigherOrderEditRequest);
    return () => window.removeEventListener("quickpls:edit-higher-order", onHigherOrderEditRequest);
  }, [openHigherOrderDialog]);

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
        onDerive={() => { if (!rejectLockedDataMutation("Derive a variable")) openDialog("derive-variable"); }}
        onContextMenuRequest={onDataContextMenuRequest}
      /> : null}
      {surface === "model" ? <ModelSurface
        modelName={activeEditableModelName}
        propertiesOpen={propertiesOpen}
        readiness={modelReadiness}
        generalSemRevisionRequired={strictGeneralSemRevisionRequired}
        onContextMenuRequest={onModelCanvasContextMenuRequest}
        onEditHigherOrder={(strictGeneralSemRevisionRequired
          ? generalSemRevisionDisabledReason === null
          : projectWritable && !isNativeCalculationActive(runMonitor.status))
          ? ({ nodeId }) => openHigherOrderDialog({ kind: "edit", constructId: nodeId })
          : undefined}
        onRemoveHigherOrder={(strictGeneralSemRevisionRequired
          ? generalSemRevisionDisabledReason === null
          : projectWritable && !isNativeCalculationActive(runMonitor.status))
          ? ({ nodeId, termId }) => removeHigherOrderConstruct(termId, nodeId)
          : undefined}
        onRemoveModeratingEffect={(strictGeneralSemRevisionRequired
          ? generalSemRevisionDisabledReason === null
          : projectWritable && !isNativeCalculationActive(runMonitor.status))
          ? ({ termId }) => removeModeratingEffect(termId)
          : undefined}
      /> : null}
      {surface === "results" ? <Suspense fallback={<ResultsSurfaceLoading propertiesOpen={propertiesOpen} />}><NativeResultsSurface
        runs={completedRuns}
        selectedRun={selectedRun}
        selectedRunId={selectedRunId}
        setSelectedRunId={(id) => {
          setGeneralSemResultSelected(false);
          setSelectedResultRun(id);
        }}
        canonicalDocument={generalSemCanonicalResult ?? undefined}
        canonicalSelected={generalSemResultSelected && Boolean(generalSemCanonicalResult)}
        selectCanonicalDocument={() => setGeneralSemResultSelected(true)}
        navigation={resultNavigation}
        selectedItem={selectedResultItem}
        selectedTable={selectedTable}
        setSelectedTableId={setSelectedTableId}
        propertiesOpen={propertiesOpen}
        openMethodDetails={() => openDialog("trust")}
        onCalculate={() => dispatchNativeAction({ id: "calculation.open" })}
      /></Suspense> : null}
    </div>
    {contextMenu ? <ContextCommandMenu items={contextMenuItems} state={contextMenu} close={closeContextMenu} /> : null}
    <NativeToastStack toasts={toasts} dismiss={dismissToast} />
    <StatusBar
      surface={surface}
      projectName={projectName}
      datasetName={dataset.name}
      cases={dataset.rowCount ?? dataset.rows.length}
      constructs={nodes.filter((node) => node.data.semantic !== "interaction").length}
      features={surface === "model" ? nativeCanvasFeatureInventory(nodes, edges) : null}
      runMonitor={runMonitor}
    />
    {dialog ? <DialogHost dialog={dialog} close={closeDialog} title={dialogTitle(dialog, higherOrderDialogRequest.kind, moderationDialogRequest.kind)} dismissible={dialog === "recode-data"
      ? !recodeBusy
      : dialog === "derive-variable"
        ? !deriveBusy
        : dialog === "higher-order"
          ? !higherOrderCommitPending
          : dialog === "moderation"
            ? !moderationCommitPending
            : dialog === "advanced-calculation"
              ? !generalSemTransientWorkBlocker
              : dialog !== "calculation" || !["queued", "validating", "running", "cancelling"].includes(runMonitor.status)}>
      {dialog === "new-project" ? <NewProjectDialog
        value={newProjectName}
        setValue={setNewProjectName}
        close={closeDialog}
        create={createProject}
        experimentalLabsEnabled={uiPreferences.experimentalLabsEnabled}
        nativeDesktop={isNativeDesktop()}
      /> : null}
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
        request={higherOrderDialogRequest}
        commit={commitHigherOrderDialogSubmission}
        onPendingChange={setHigherOrderCommitPending}
        close={closeDialog}
      /></Suspense> : null}
      {dialog === "moderation" ? <Suspense fallback={<UtilityDialogLoading label="Opening moderating effect setup" />}><NativeModerationDialog
        nodes={nodes}
        edges={edges}
        request={moderationDialogRequest}
        commit={commitModerationDialogSubmission}
        onPendingChange={setModerationCommitPending}
        close={closeDialog}
      /></Suspense> : null}
      {dialog === "advanced-parameters" ? <NativeSemParameterTable
        modelName={activeEditableModelName}
        presentation="dialog"
        onContinueToCalculation={() => {
          openDialog("advanced-calculation");
        }}
        onShowCanvas={() => {
          closeDialog();
          navigate("model");
          window.setTimeout(() => document.getElementById("nd-model-canvas-panel")?.focus(), 0);
        }}
      /> : null}
      {dialog === "advanced-calculation" ? advancedCalculationPlan?.route === "exact_cbsem_compatibility"
        ? <NativeRecipeV4CbsemWorkspace
            key={`${activeModelId ?? "model"}:exact-cbsem-compatibility`}
            modelName={activeEditableModelName}
            experimentalLabsEnabled={uiPreferences.experimentalLabsEnabled}
            presentation="calculation"
            calculationPlan={advancedCalculationPlan}
          />
        : <NativeRecipeV4GeneralSemWorkspace
            key={`${activeModelId ?? "model"}:${advancedCalculationKind}:${advancedCalculationPlan?.inference ?? "default"}`}
            modelName={activeEditableModelName}
            experimentalLabsEnabled={uiPreferences.experimentalLabsEnabled}
            projectActivationConnected
            presentation="calculation"
            initialCalculationKind={advancedCalculationKind}
            calculationPlan={advancedCalculationPlan}
            activationOnly={Boolean(pendingExactCbsemPlan)}
            onAuthorityActivated={pendingExactCbsemPlan ? () => {
              setAdvancedCalculationPlan(pendingExactCbsemPlan);
              setPendingExactCbsemPlan(null);
            } : undefined}
          /> : null}
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
        unifiedSem={unifiedSemCalculation}
        onUnifiedSemAction={handleUnifiedSemCalculationAction}
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
          {items.map((item) => <button key={item.id} role="menuitem" tabIndex={-1} type="button" className={item.separator ? "separator" : ""} disabled={item.disabled} title={item.disabled ? item.disabledReason : undefined} aria-label={item.disabledReason ? `${item.label}. Unavailable: ${item.disabledReason}` : item.label} onClick={() => { setOpenMenu(null); triggerRefs.current[index]?.focus(); item.action(); }}><span>{item.label}</span>{item.shortcut ? <kbd>{item.shortcut}</kbd> : null}</button>)}
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
    {items.map((item) => <button key={item.id} role="menuitem" tabIndex={-1} type="button" className={item.separator ? "separator" : ""} disabled={item.disabled} title={item.disabled ? item.disabledReason : undefined} aria-label={item.disabledReason ? `${item.label}. Unavailable: ${item.disabledReason}` : item.label} onClick={() => { focusReturnTarget(); close(); item.action(); }}><span>{item.label}</span>{item.shortcut ? <kbd>{item.shortcut}</kbd> : null}</button>)}
  </div>;
}

function CommandBar({ commands, surface, projectName, projectPath, modelName, propertiesOpen, setPropertiesOpen }: { commands: DesktopCommand[]; surface: NativeSurface; projectName: string; projectPath: string | null; modelName: string; propertiesOpen: boolean; setPropertiesOpen: (value: boolean) => void }) {
  return <div className="nd-commandbar" role="toolbar" aria-label={`${surface} commands`}>
    <div className="nd-document-context"><strong>{surface === "launcher" ? "Project" : surface === "data" ? "Data" : surface === "model" ? modelName : "Results"}</strong>{surface !== "launcher" ? <span>{projectPath ?? projectName}</span> : null}</div>
    <div className="nd-command-list">
      {commands.map((command) => {
        const Icon = command.icon;
        if (command.menuItems?.length) return <CommandBarSplitButton key={command.id} command={command} />;
        return <button type="button" key={command.id} className={command.primary ? "primary" : ""} disabled={command.disabled} aria-pressed={command.pressed} aria-label={command.disabledReason ? `${command.label}. Unavailable: ${command.disabledReason}` : command.label} title={command.disabled ? command.disabledReason ?? command.label : command.label} onClick={command.action}>{Icon ? <Icon size={15} aria-hidden="true" /> : null}<span>{command.label}</span></button>;
      })}
    </div>
    {surface !== "launcher" ? <button className="nd-pane-toggle" type="button" aria-pressed={propertiesOpen} title={propertiesOpen ? "Hide Properties" : "Show Properties"} onClick={() => setPropertiesOpen(!propertiesOpen)}>{propertiesOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />} Properties</button> : null}
  </div>;
}

function CommandBarSplitButton({ command }: { command: DesktopCommand }) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const Icon = command.icon;
  useEffect(() => {
    if (!open) return;
    const dismiss = (event: PointerEvent) => {
      if (rootRef.current?.contains(event.target as globalThis.Node)) return;
      setOpen(false);
    };
    window.addEventListener("pointerdown", dismiss);
    return () => window.removeEventListener("pointerdown", dismiss);
  }, [open]);
  const onMenuKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
      toggleRef.current?.focus();
      return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const buttons = Array.from(event.currentTarget.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'));
    if (!buttons.length) return;
    const current = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next = nextEnabledItemIndex(buttons.map((button) => button.disabled), current, event.key as MenuItemNavigationKey);
    buttons[next]?.focus();
  };
  return <div className="nd-command-split" ref={rootRef}>
    <button
      type="button"
      className={command.primary ? "primary" : ""}
      disabled={command.disabled}
      aria-pressed={command.pressed}
      aria-label={command.disabledReason ? `${command.label}. Unavailable: ${command.disabledReason}` : command.label}
      title={command.disabled ? command.disabledReason ?? command.label : command.label}
      onClick={command.action}
    >{Icon ? <Icon size={15} aria-hidden="true" /> : null}<span>{command.label}</span></button>
    <button
      ref={toggleRef}
      type="button"
      className="nd-command-split-toggle"
      disabled={command.disabled}
      aria-label={`${command.label} options`}
      aria-haspopup="menu"
      aria-expanded={open}
      onClick={() => setOpen((value) => !value)}
    ><ChevronDown size={13} aria-hidden="true" /></button>
    {open ? <div className="nd-command-split-menu" role="menu" aria-label={`${command.label} options`} onKeyDown={onMenuKeyDown}>
      {command.menuItems?.map((item, index) => <button
        key={item.id}
        type="button"
        role="menuitem"
        tabIndex={index === 0 ? 0 : -1}
        className={item.separator ? "separator" : ""}
        disabled={item.disabled}
        title={item.disabled ? item.disabledReason : undefined}
        onClick={() => { setOpen(false); item.action(); }}
      >{item.label}</button>)}
    </div> : null}
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

function ModelSurface({ modelName, propertiesOpen, readiness, generalSemRevisionRequired, onContextMenuRequest, onEditHigherOrder, onRemoveHigherOrder, onRemoveModeratingEffect }: {
  modelName: string;
  propertiesOpen: boolean;
  readiness: NativePlsReadiness;
  generalSemRevisionRequired: boolean;
  onContextMenuRequest: (request: ModelCanvasContextMenuRequest) => void;
  onEditHigherOrder?: (request: { nodeId: string; termId: string }) => void;
  onRemoveHigherOrder?: (request: { nodeId: string; termId: string }) => void;
  onRemoveModeratingEffect?: (request: { nodeId: string; termId: string }) => void;
}) {
  const dataset = useWorkspace((state) => state.dataset);
  const groupingVariable = useWorkspace((state) => state.analysisSettings.groupColumn?.trim() ?? "");
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const showGeneratedInteractionTerms = useWorkspace((state) => state.uiPreferences.showGeneratedInteractionTerms);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const executeModelEditCommand = useWorkspace((state) => state.executeModelEditCommand);
  const [authorityStatus, setAuthorityStatus] = useState<string | null>(null);
  const indicatorIntentCounter = useRef(0);
  const selected = nodes.find((node) => node.id === selectedNodeId);
  const selectedAssignableConstruct = selected?.data.semantic === "interaction" || selected?.data.semantic === "higher_order" ? undefined : selected;
  const [query, setQuery] = useState("");
  const [navigatorSection, setNavigatorSection] = useState<"indicators" | "constructs" | "relationships">("indicators");
  const [relationshipQuery, setRelationshipQuery] = useState("");
  const [indicatorSelection, setIndicatorSelection] = useState<NativeIndicatorSelectionV1>({ selected: [], anchor: null });
  const [indicatorContextMenu, setIndicatorContextMenu] = useState<NativeContextMenuState | null>(null);
  const visibleColumns = dataset.columns.filter((column) => column.toLowerCase().includes(query.trim().toLowerCase()));
  const relationshipRows = useMemo(
    () => nativeModelNavigatorRelationshipsV1(nodes, edges).filter((row) => `${row.label} ${row.detail}`.toLowerCase().includes(relationshipQuery.trim().toLowerCase())),
    [edges, nodes, relationshipQuery],
  );
  useEffect(() => {
    const available = new Set(dataset.columns.filter((column) => column !== groupingVariable));
    setIndicatorSelection((current) => ({
      selected: current.selected.filter((value) => available.has(value)),
      anchor: current.anchor && available.has(current.anchor) ? current.anchor : null,
    }));
  }, [dataset.columns, groupingVariable]);
  const selectedIndicators = indicatorSelection.selected.filter((column) => column !== groupingVariable);
  const createConstructFromIndicators = (indicators: readonly string[]) => {
    const first = indicators[0] ?? "Construct";
    const plan = planNativeIndicatorGroupActionV1(
      dataset.columns,
      indicators,
      {
        kind: "create_construct",
        constructId: `model:construct:${Date.now()}:${++indicatorIntentCounter.current}`,
        label: indicators.length === 1 ? first : `Construct ${nodes.length + 1}`,
      },
      groupingVariable || null,
    );
    if (plan.status === "blocked") {
      setAuthorityStatus(plan.message);
      return;
    }
    setAuthorityStatus("Creating construct…");
    void executeModelEditCommand(plan.command).then((result) => setAuthorityStatus(result.status === "applied"
      ? "Applied as one undoable model transaction."
      : `${result.message} ${result.correctiveAction}`));
  };
  const assignSelectedIndicators = (indicators: readonly string[]) => {
    if (!selectedAssignableConstruct) return;
    const plan = planNativeIndicatorGroupActionV1(
      dataset.columns,
      indicators,
      { kind: "assign_indicators", constructId: selectedAssignableConstruct.id },
      groupingVariable || null,
    );
    if (plan.status === "blocked") {
      setAuthorityStatus(plan.message);
      return;
    }
    setAuthorityStatus("Applying indicator assignment…");
    void executeModelEditCommand(plan.command).then((result) => setAuthorityStatus(result.status === "applied"
      ? "Applied as one undoable model transaction."
      : `${result.message} ${result.correctiveAction}`));
  };
  const selectIndicator = (event: ReactMouseEvent<HTMLButtonElement>, variable: string) => {
    if (variable === groupingVariable) return;
    setIndicatorSelection((current) => nextNativeIndicatorSelectionV1({
      visible: visibleColumns.filter((column) => column !== groupingVariable),
      current,
      indicator: variable,
      toggle: event.ctrlKey || event.metaKey,
      range: event.shiftKey,
    }));
  };
  const dragVariable = (event: DragEvent<HTMLButtonElement>, variable: string) => {
    if (variable === groupingVariable) {
      event.preventDefault();
      return;
    }
    const dragged = nativeIndicatorDragSelectionV1(selectedIndicators, variable);
    if (!selectedIndicators.includes(variable)) setIndicatorSelection({ selected: dragged, anchor: variable });
    event.dataTransfer.setData("application/qpls-indicator", variable);
    event.dataTransfer.setData("application/qpls-indicators", JSON.stringify(dragged));
    event.dataTransfer.setData("text/plain", nativeIndicatorDragLabelV1(dragged));
    const ghost = document.createElement("div");
    ghost.className = "nd-indicator-drag-ghost";
    ghost.textContent = nativeIndicatorDragLabelV1(dragged);
    document.body.appendChild(ghost);
    event.dataTransfer.setDragImage(ghost, 12, 12);
    window.setTimeout(() => ghost.remove(), 0);
    window.dispatchEvent(new CustomEvent("quickpls:variables-dragging", { detail: { count: dragged.length } }));
  };
  const openIndicatorContextMenu = (event: ReactMouseEvent<HTMLButtonElement>, variable: string) => {
    event.preventDefault();
    let nextSelection = indicatorSelection;
    if (!indicatorSelection.selected.includes(variable)) {
      nextSelection = { selected: [variable], anchor: variable };
      setIndicatorSelection(nextSelection);
    }
    const position = contextMenuCoordinates(event.clientX, event.clientY, window.innerWidth, window.innerHeight, 244, 76);
    setIndicatorContextMenu({
      ...position,
      returnFocus: event.currentTarget,
      selection: { kind: "multiple", count: nextSelection.selected.length },
    });
  };
  const indicatorContextMenuItems: MenuItem[] = [{
    id: "create-construct-from-indicators",
    label: `Create Construct from ${selectedIndicators.length || 1} Indicator${selectedIndicators.length === 1 ? "" : "s"}`,
    disabled: selectedIndicators.length === 0,
    action: () => createConstructFromIndicators(selectedIndicators),
  }, {
    id: "assign-indicators-to-construct",
    label: selectedAssignableConstruct
      ? `Assign to ${selectedAssignableConstruct.data.label}`
      : "Assign to Selected Construct",
    disabled: !selectedAssignableConstruct || selectedIndicators.length === 0,
    disabledReason: selectedAssignableConstruct ? undefined : "Select a measured construct first.",
    action: () => assignSelectedIndicators(selectedIndicators),
  }];
  return <div className={`nd-three-pane nd-model-workspace${propertiesOpen ? "" : " no-properties"}`}>
    <aside className="nd-navigator" aria-label="Model navigator">
      <PaneTitle icon={<Database size={14} />} title="Model objects" />
      <div className="nd-model-navigator-tabs" role="tablist" aria-label="Model object type">
        {(["indicators", "constructs", "relationships"] as const).map((section) => <button
          key={section}
          type="button"
          role="tab"
          aria-selected={navigatorSection === section}
          tabIndex={navigatorSection === section ? 0 : -1}
          onClick={() => setNavigatorSection(section)}
          onKeyDown={(event) => {
            if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
            event.preventDefault();
            const sections = ["indicators", "constructs", "relationships"] as const;
            const current = sections.indexOf(navigatorSection);
            const next = event.key === "Home" ? 0 : event.key === "End" ? sections.length - 1 : (current + (event.key === "ArrowRight" ? 1 : -1) + sections.length) % sections.length;
            setNavigatorSection(sections[next]!);
            window.setTimeout(() => document.querySelectorAll<HTMLButtonElement>(".nd-model-navigator-tabs [role='tab']")[next]?.focus(), 0);
          }}
        >{section === "indicators" ? "Indicators" : section === "constructs" ? "Constructs" : "Relationships"}</button>)}
      </div>
      {navigatorSection === "indicators" ? <>
        <label className="nd-search"><Search size={13} /><input aria-label="Search indicators" placeholder="Search indicators" value={query} onChange={(event) => setQuery(event.target.value)} /></label>
        <p id="nd-indicator-drag-help" className="nd-variable-instruction">Select normally; Ctrl/Shift extends the selection. Drag the selection to the Canvas or a construct.</p>
        {selectedIndicators.length ? <div className="nd-indicator-selection-actions" role="group" aria-label={`${selectedIndicators.length} selected indicators`}>
          <span>{selectedIndicators.length} selected</span>
          <button type="button" onClick={() => createConstructFromIndicators(selectedIndicators)}>Create construct</button>
          <button type="button" disabled={!selectedAssignableConstruct} title={selectedAssignableConstruct ? undefined : "Select a measured construct first."} onClick={() => assignSelectedIndicators(selectedIndicators)}>Assign{selectedAssignableConstruct ? ` to ${selectedAssignableConstruct.data.label}` : ""}</button>
        </div> : null}
        <div className="nd-variable-list nd-indicator-selection-list" role="listbox" aria-label="Dataset indicators" aria-multiselectable="true" aria-describedby="nd-indicator-drag-help">
          {visibleColumns.map((column) => {
            const owner = nodes.find((node) => node.data.indicators.includes(column));
            const isGroupingVariable = column === groupingVariable;
            const selectedIndicator = selectedIndicators.includes(column);
            const action = isGroupingVariable ? "Grouping variable; unavailable as an indicator" : owner ? `Assigned to ${owner.data.label}` : "Available indicator";
            return <button
              key={column}
              type="button"
              role="option"
              aria-selected={selectedIndicator}
              aria-keyshortcuts="Control+Enter Alt+Enter"
              draggable={!isGroupingVariable}
              disabled={isGroupingVariable}
              onDragStart={(event) => dragVariable(event, column)}
              onDragEnd={() => window.dispatchEvent(new CustomEvent("quickpls:variables-dragging", { detail: { count: 0 } }))}
              onClick={(event) => selectIndicator(event, column)}
              onContextMenu={(event) => openIndicatorContextMenu(event, column)}
              onKeyDown={(event) => {
                if (event.key !== "Enter" || (!event.ctrlKey && !event.altKey)) return;
                event.preventDefault();
                const selectedForAction = selectedIndicators.includes(column) ? selectedIndicators : [column];
                if (event.ctrlKey) createConstructFromIndicators(selectedForAction);
                else assignSelectedIndicators(selectedForAction);
              }}
              className={`nd-variable-item${selectedIndicator ? " active" : ""}${owner ? " assigned" : ""}${isGroupingVariable ? " grouping" : ""}`}
              title={isGroupingVariable ? action : `${action}; drag the current selection to place or assign`}
              aria-label={`${column}. ${action}`}
            ><Square size={9} fill="currentColor" />{column}{isGroupingVariable ? <small>Group</small> : owner ? <Check size={12} /> : null}</button>;
          })}
        </div>
        {indicatorContextMenu ? <ContextCommandMenu items={indicatorContextMenuItems} state={indicatorContextMenu} close={() => setIndicatorContextMenu(null)} /> : null}
        {authorityStatus ? <p className="nd-authority-feedback" role="status" aria-live="polite">{authorityStatus}</p> : null}
      </> : null}
      {navigatorSection === "constructs" ? <div className="nd-variable-list nd-model-object-list">
        {nodes.filter((node) => showGeneratedInteractionTerms || node.data.semantic !== "interaction").map((node) => <button key={node.id} className={selectedNodeId === node.id ? "active" : ""} onClick={() => { setSelectedNode(node.id); window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id: node.id } })); }}>{node.data.semantic === "interaction" ? <GitBranch size={11} /> : <Circle size={11} />}{node.data.label}<small>{node.data.semantic === "interaction" ? "INT" : node.data.semantic === "higher_order" ? "HOC" : node.data.indicators.length}</small></button>)}
      </div> : null}
      {navigatorSection === "relationships" ? <>
        <label className="nd-search"><Search size={13} /><input aria-label="Search relationships" placeholder="Search relationships" value={relationshipQuery} onChange={(event) => setRelationshipQuery(event.target.value)} /></label>
        <div className="nd-variable-list nd-relationship-list">
          {relationshipRows.map((row) => <button key={row.id} type="button" onClick={() => {
            if (row.kind === "relationship") {
              setSelectedEdge(row.relationId);
              window.dispatchEvent(new CustomEvent("quickpls:focus-edge", { detail: { id: row.relationId } }));
            } else if (row.kind === "moderation") {
              window.dispatchEvent(new CustomEvent("quickpls:focus-moderation", { detail: { interactionTermId: row.interactionTermId } }));
            } else {
              setSelectedNode(row.constructId);
              window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id: row.constructId } }));
            }
          }}><GitBranch size={11} aria-hidden="true" /><span>{row.label}</span><small>{row.detail}</small></button>)}
          {!relationshipRows.length ? <p className="nd-nav-empty">No matching relationships.</p> : null}
        </div>
      </> : null}
    </aside>
    <section className="nd-document nd-model-document">
      <div className="nd-model-document-tabs nd-model-document-toolbar" role="toolbar" aria-label={`${modelName} model actions`}>
        <span className="nd-model-document-title" title={modelName}><GitBranch size={14} aria-hidden="true" />{modelName}</span>
        <span className="nd-model-view-label"><GitBranch size={13} aria-hidden="true" />Canvas</span>
        <span className="nd-model-toolbar-spacer" />
      </div>
      {generalSemRevisionRequired ? <p className="nd-inline-warning" role="note" data-testid="general-sem-scientific-revision-required">
        <strong>Safe revision required.</strong> Advanced scientific edits create a new calculation-ready revision; the current project remains unchanged.
      </p> : null}
      <div id="nd-model-canvas-panel" className="nd-canvas-host" role="region" aria-label={`${modelName} model canvas`} tabIndex={-1}><ModelCanvas onContextMenuRequest={onContextMenuRequest} showGeneratedInteractionTerms={showGeneratedInteractionTerms} /></div>
    </section>
    {propertiesOpen ? <NativeModelInspector
      readiness={readiness}
      onEditHigherOrder={onEditHigherOrder}
      onRemoveHigherOrder={onRemoveHigherOrder}
      onRemoveModeratingEffect={onRemoveModeratingEffect}
    /> : null}
  </div>;
}

export function nativeCanvasFeatureInventory(
  nodes: readonly { id: string; data: { semantic?: string; interaction?: InteractionData } }[],
  edges: readonly {
    id?: string;
    source: string;
    target: string;
    data?: { role?: string; visualOnly?: boolean; technicalGenerated?: boolean; standardSemV4Authority?: { presentationOnly?: boolean } };
  }[],
): { indirectPaths: number; interactions: number; higherOrderConstructs: number } {
  const scientificNodeIds = new Set(nodes
    .filter((node) => !node.data.semantic)
    .map((node) => node.id));
  const outgoing = new Map<string, Array<{ target: string; relationId: string }>>();
  for (const [edgeIndex, edge] of edges.entries()) {
    if (edge.id?.startsWith("measurement::")
      || edge.data?.role === "control"
      || edge.data?.role === "covariance"
      || edge.data?.visualOnly
      || edge.data?.technicalGenerated
      || edge.data?.standardSemV4Authority?.presentationOnly
      || !scientificNodeIds.has(edge.source)
      || !scientificNodeIds.has(edge.target)
      || edge.source === edge.target) continue;
    outgoing.set(edge.source, [
      ...(outgoing.get(edge.source) ?? []),
      { target: edge.target, relationId: edge.id ?? `relation:${edgeIndex}` },
    ]);
  }
  const paths = new Set<string>();
  const indirectPathLimit = 10_000;
  const visit = (
    nodeId: string,
    visitedNodeIds: ReadonlySet<string>,
    relationIds: readonly string[],
  ): void => {
    if (paths.size >= indirectPathLimit) return;
    for (const relation of outgoing.get(nodeId) ?? []) {
      if (visitedNodeIds.has(relation.target)) continue;
      const nextRelationIds = [...relationIds, relation.relationId];
      if (nextRelationIds.length >= 2) paths.add(nextRelationIds.join("\u0000"));
      if (paths.size >= indirectPathLimit) return;
      visit(
        relation.target,
        new Set([...visitedNodeIds, relation.target]),
        nextRelationIds,
      );
      if (paths.size >= indirectPathLimit) return;
    }
  };
  for (const source of outgoing.keys()) {
    visit(source, new Set([source]), []);
    if (paths.size >= indirectPathLimit) break;
  }
  const interactionTerms = nodes.flatMap((node) => {
    if (node.data.semantic !== "interaction" || !node.data.interaction) return [];
    const interaction = node.data.interaction;
    const operands = interaction.kind === "interaction_v2"
      ? [...interaction.operands]
      : [interaction.predictor, interaction.moderator];
    return [{ operands, outcome: interaction.outcome }];
  });
  const researcherFacingModerationCount = interactionTerms.filter((candidate, index) => !interactionTerms.some((parent, parentIndex) => (
    parentIndex !== index
    && parent.outcome === candidate.outcome
    && parent.operands.length > candidate.operands.length
    && candidate.operands.every((operand) => parent.operands.includes(operand))
  ))).length;
  return {
    indirectPaths: paths.size,
    interactions: researcherFacingModerationCount,
    higherOrderConstructs: nodes.filter((node) => node.data.semantic === "higher_order").length,
  };
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

export function NewProjectDialog({ value, setValue, close, create, experimentalLabsEnabled, nativeDesktop }: {
  value: string;
  setValue: (value: string) => void;
  close: () => void;
  create: () => void;
  experimentalLabsEnabled: boolean;
  nativeDesktop: boolean;
}) {
  const generalSemAccess = generalSemWorkspaceProductAccessV1(experimentalLabsEnabled);
  const generalSemAvailable = Boolean(generalSemAccess) && nativeDesktop;
  return <form className="nd-dialog-form" onSubmit={(event) => { event.preventDefault(); create(); }}>
    <label>Project name<input autoFocus value={value} onChange={(event) => setValue(event.target.value)} /></label>
    <p className="nd-dialog-note" role="note">{generalSemAvailable
      ? "QuickPLS will prepare one calculation-ready project for the Canvas, all compatible PLS-SEM and CB-SEM methods, and verified Results."
      : "This preview creates a standard project. The installed Windows app adds the calculation-ready scientific project authority."}</p>
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
  const shortcuts = [["Ctrl+N", "New project"], ["Ctrl+O", "Open project"], ["Ctrl+S", "Save"], ["Ctrl+Shift+S", "Save as"], ["Ctrl+R", "Calculate"], ["C", "Create construct"], ["V", "Select tool"], ["P", "Path tool"], ["M", "Add or extend a moderating effect"], ["F", "Fit model"], ["Enter", "Edit selected construct, path, or moderating effect"], ["Delete", "Delete selection"], ["Tab", "Navigate model and inspector controls"], ["Arrow keys", "Move between inspector sections"]];
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
  const availabilityView = experimentalLabsEnabled ? "Standard + Experimental Labs" : "Standard";
  return <div className="nd-about"><div className="nd-about-mark">Q</div><div><h3>QuickPLS</h3><p>Offline structural equation modeling for Windows.</p><dl className="nd-property-list"><div><dt>Version</dt><dd>2.54.0</dd></div><div><dt>Availability view</dt><dd>{availabilityView}</dd></div><div><dt>Available calculation methods</dt><dd>{visibleMethods.length ? visibleMethods.join(", ") : "No methods are available in the current view."}</dd></div><div><dt>Model workflow</dt><dd>Authority-aware Canvas editing and Registry-authorized PLS-SEM and CB-SEM use one Canvas, Calculate, Results, export, and reopen workflow.</dd></div><div><dt>Conditional result groups</dt><dd>Researcher-facing mediation, two-way and three-way moderation, higher-order, moderated-mediation, and CB-SEM output appears only when owned by the completed result.</dd></div><div><dt>Runtime</dt><dd>{isNativeDesktop() ? "Native desktop" : "Browser preview"}</dd></div><div><dt>Implementation</dt><dd>Independent QuickPLS engine</dd></div><div><dt>Third-party notices</dt><dd>Included with the installed application</dd></div></dl></div></div>;
}

function StatusBar({
  surface,
  projectName,
  datasetName,
  cases,
  constructs,
  features,
  runMonitor,
}: {
  surface: NativeSurface;
  projectName: string;
  datasetName: string;
  cases: number;
  constructs: number;
  features: ReturnType<typeof nativeCanvasFeatureInventory> | null;
  runMonitor: ReturnType<typeof useWorkspace.getState>["runMonitor"];
}) {
  const stateLabel = ["queued", "validating", "running", "cancelling", "blocked", "failed", "cancelled"].includes(runMonitor.status)
    ? nativeCalculationPhaseLabel(runMonitor.phase, runMonitor.status)
    : "Ready";
  const projectOpen = projectName !== "No project open";
  return <footer className="nd-statusbar" aria-live="polite">
    <span className={`nd-status-dot ${runMonitor.status}`} />
    <strong>{stateLabel}</strong>
    {surface !== "launcher" || projectOpen ? <><span>{projectName}</span><span>{datasetName}</span><span>{cases} cases</span><span>{constructs} constructs</span></> : null}
    {features && (features.indirectPaths || features.interactions || features.higherOrderConstructs)
      ? <span className="nd-status-features" aria-label="Detected model features">
          {[
            features.indirectPaths ? `${features.indirectPaths} indirect path${features.indirectPaths === 1 ? "" : "s"}` : "",
            features.interactions ? `${features.interactions} moderation effect${features.interactions === 1 ? "" : "s"}` : "",
            features.higherOrderConstructs ? `${features.higherOrderConstructs} HOC` : "",
          ].filter(Boolean).join(" · ")}
        </span>
      : null}
    <span className="spacer" />
    <span>{surface === "launcher" && projectOpen ? "Project" : surface[0].toUpperCase() + surface.slice(1)}</span>
    <span>Offline</span>
  </footer>;
}

function dialogTitle(
  dialog: Exclude<NativeDialog, null>,
  higherOrderMode: NativeHigherOrderDialogRequest["kind"] = "create",
  moderationMode: NativeModerationDialogRequest["kind"] = "create",
) {
  if (dialog === "new-project") return "New Project";
  if (dialog === "import-data") return "Import Data";
  if (dialog === "recode-data") return "Recode Variable";
  if (dialog === "derive-variable") return "Derive Variable";
  if (dialog === "group-setup") return "Configure Groups";
  if (dialog === "moderation") return moderationMode === "edit" ? "Edit Moderating Effect" : "Create Moderating Effect";
  if (dialog === "higher-order") return higherOrderMode === "edit"
    ? "Edit Higher-Order Construct"
    : "Create Higher-Order Construct";
  if (dialog === "calculation") return "Calculate";
  if (dialog === "advanced-calculation") return "Calculate Advanced Model";
  if (dialog === "advanced-parameters") return "Advanced Parameter Table";
  if (dialog === "export") return "Export Results";
  if (dialog === "trust") return "Method Details";
  if (dialog === "settings") return "Preferences";
  if (dialog === "shortcuts") return "Keyboard Shortcuts";
  if (dialog === "about") return "About QuickPLS";
  return "Run Details";
}
