import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  MarkerType,
  reconnectEdge,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  type XYPosition,
} from "@xyflow/react";
import { create } from "zustand";
import { initialEdges, initialNodes, sampleDataset } from "./data/sample";
import { defaultDiagramLayout, layoutSmartplsModel } from "./domain/diagramGraph";
import { layoutModel } from "./domain/modelLayout";
import {
  parseStandardSemModelV4AuthorityRecordV1,
  reduceStandardSemModelV4AuthorityV1,
  type StandardSemModelV4AuthorityRecordV1,
  type StandardSemModelV4EditorIntentV1,
} from "./domain/standardSemModelV4Authority";
import {
  parseStandardSemModelV4DiagramLayoutV1,
  projectStandardSemModelV4DiagramV1,
  type StandardSemModelV4DiagramLayoutV1,
} from "./domain/standardSemModelV4DiagramProjection";
import {
  convertNativeCovarianceToPresentationV4,
  convertNativeCovarianceToScientificV4,
  nativeCovariancePairExistsV4,
  newNativeScientificCovarianceEdgeV4,
  withNativeConstructEstimandV4,
} from "./domain/semModelV4Authoring";
import { buildNativeRecipeModel } from "./native/nativeAnalysisRecipe";
import { currentNativeModelPresentation, nativeModelSnapshotFromCanonical } from "./native/nativeCanonicalProject";
import { nativeHigherOrderCreationBlocker, nativeHigherOrderDraftProblems, type NativeHigherOrderDraft } from "./native/nativeHigherOrder";
import { compareAndSwapStandardSemModelV4Authority } from "./services/standardSemModelV4AuthorityService";
import type { StandardSemModelV4AuthorityCasDiagnosticV1, StandardSemModelV4AuthorityCasOutcomeV1 } from "./domain/standardSemModelV4AuthorityCas";
import type { AnalysisMethodId, AnalysisRun, AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset, DatasetVersionMutation, DatasetVersionRecord, DesktopCommandStatus, DesktopDialogId, DesktopMenuId, DiagramLayoutState, DiagramMode, DiagramOverlaySettings, DiagramToolMode, ExplorerTab, GeneralSemProjectDraftModeV1, IndicatorSide, LargeModelViewState, MethodPresetId, MethodSetupState, NativeCanonicalModelSpec, NativeExplorerSelection, NativeModelPresentation, NativeProcessGraphRelationshipConfig, NativeSavedReport, OnboardingState, PublicationDiagramSettings, ResultWorkspaceState, RunMonitorLogEntry, RunMonitorState, SemModelV4AuthoringEndpoint, SemModelV4ConstructAuthoring, ToastNotification, UiPreferences, WorkflowCommandContext, WorkflowDestinationContext, WorkspaceView } from "./types";

export type GeneralSemTransientWorkBlockerV1 = "job_active" | "temporary_result_pending";

type AlignTarget = "left" | "centerX" | "right" | "top" | "centerY" | "bottom";
type DistributeAxis = "horizontal" | "vertical";
type PathRouting = "smoothstep" | "default" | "straight";

export type AddTwoStageInteractionBlockReason =
  | "constructs_not_distinct"
  | "duplicate_interaction"
  | "construct_missing"
  | "unsupported_construct"
  | "focal_path_missing"
  | "control_paths_unsupported";

export type AddTwoStageInteractionResult =
  | { status: "created"; interactionId: string }
  | { status: "blocked"; reason: AddTwoStageInteractionBlockReason };

export type AddHigherOrderConstructBlockReason =
  | "scope_unavailable"
  | "invalid_draft";

export type AddHigherOrderConstructResult =
  | { status: "created"; constructId: string }
  | { status: "blocked"; reason: AddHigherOrderConstructBlockReason; detail: string };

interface LegacyHistorySnapshot {
  kind: "legacy_graph";
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  diagramLayout: DiagramLayoutState;
}

interface StandardSemModelV4HistorySnapshot {
  kind: "standard_sem_model_v4";
  modelId: string;
  authority: StandardSemModelV4AuthorityRecordV1;
  persistence: Pick<StandardSemModelV4PersistenceV1, "readiness" | "scientificSha256">;
  diagramLayout: DiagramLayoutState;
}

type HistorySnapshot = LegacyHistorySnapshot | StandardSemModelV4HistorySnapshot;

export type StandardSemModelV4BlockedOperation =
  | "schema5_save"
  | "schema5_autosave"
  | "calculation"
  | "legacy_graph_serialization";

export type StandardSemModelV4AuthorityCommitResult =
  | { status: "committed"; authority: StandardSemModelV4AuthorityRecordV1 }
  | { status: "blocked"; diagnostic: StandardSemModelV4AuthorityCasDiagnosticV1 }
  | { status: "stale" }
  | { status: "rejected"; error: unknown };

export interface StandardSemModelV4PersistenceV1 {
  readiness: "ready" | "authoring_only";
  scientificSha256: string | null;
  anchorModelDocumentSha256: string;
  anchorLayout: StandardSemModelV4DiagramLayoutV1;
}

export interface StandardSemModelV4ActivationV1 {
  authority: StandardSemModelV4AuthorityRecordV1;
  layout?: StandardSemModelV4DiagramLayoutV1;
  readiness: "ready" | "authoring_only";
  scientificSha256: string | null;
}

export interface StandardSemModelV4RevisionAppendCasV1 {
  sourceModelId: string;
  expectedSourceModelDocumentSha256: string;
  expectedSourceEpoch: number;
}

export interface StandardSemModelV4SaveAuthorityV1 {
  authority: StandardSemModelV4AuthorityRecordV1;
  layout: StandardSemModelV4DiagramLayoutV1;
  readiness: "ready" | "authoring_only";
  scientificSha256: string | null;
  dirty: boolean;
}

export interface StandardSemModelV4DatasetDescriptorV1 {
  id: string;
  name: string;
  columns: string[];
  columnMetadata: ColumnMetadata[];
  rowCount: number;
  fingerprint: string;
  kind: "raw" | "covariance" | "correlation";
  sampleSize: number | null;
}

export interface WorkspaceState {
  view: WorkspaceView;
  workflowDestinationContext: WorkflowDestinationContext | null;
  workflowCommandContext: WorkflowCommandContext | null;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  selectedNodeId: string | null;
  selectedEdgeId: string | null;
  selectedResultRunId: string | null;
  explorerTab: ExplorerTab;
  explorerCollapsed: boolean;
  inspectorCollapsed: boolean;
  explorerWidth: number;
  uiPreferences: UiPreferences;
  resultWorkspaceState: ResultWorkspaceState;
  methodSetupState: MethodSetupState;
  onboardingState: OnboardingState;
  largeModelViewState: LargeModelViewState;
  commandPaletteOpen: boolean;
  shortcutOverlayOpen: boolean;
  activeDesktopMenu: DesktopMenuId | null;
  activeDesktopDialog: DesktopDialogId;
  desktopDialogPayload: Record<string, unknown> | null;
  desktopCommandStatus: DesktopCommandStatus | null;
  runMonitor: RunMonitorState;
  toasts: ToastNotification[];
  diagramMode: DiagramMode;
  diagramTool: DiagramToolMode;
  diagramOverlaySettings: DiagramOverlaySettings;
  publicationDiagramSettings: PublicationDiagramSettings;
  diagramLayout: DiagramLayoutState;
  dataset: Dataset;
  datasetCatalog: Dataset[];
  datasetVersions: DatasetVersionRecord[];
  projectModels: NativeCanonicalModelSpec[];
  activeModelId: string | null;
  modelPresentations: Record<string, NativeModelPresentation>;
  standardSemModelV4Authorities: Record<string, StandardSemModelV4AuthorityRecordV1>;
  standardSemModelV4ScientificEditLocks: Record<string, true>;
  standardSemModelV4Layouts: Record<string, StandardSemModelV4DiagramLayoutV1>;
  standardSemModelV4Epochs: Record<string, number>;
  standardSemModelV4Persistence: Record<string, StandardSemModelV4PersistenceV1>;
  standardSemModelV4DatasetDescriptors: Record<string, StandardSemModelV4DatasetDescriptorV1>;
  datasetDescriptorOnly: boolean;
  savedReports: NativeSavedReport[];
  explorerSelection: NativeExplorerSelection;
  runs: AnalysisRun[];
  analysisSettings: AnalysisUiSettings;
  projectName: string;
  projectId: string | null;
  projectPath: string | null;
  projectWritable: boolean;
  generalSemProjectDraftMode: GeneralSemProjectDraftModeV1 | null;
  generalSemPublicationPending: boolean;
  generalSemTransientWorkBlocker: GeneralSemTransientWorkBlockerV1 | null;
  past: HistorySnapshot[];
  future: HistorySnapshot[];
  setView: (view: WorkspaceView, context?: Omit<WorkflowDestinationContext, "timestamp">) => void;
  setWorkflowCommandContext: (context: Omit<WorkflowCommandContext, "timestamp"> | null) => void;
  clearWorkflowFeedback: () => void;
  setSelectedNode: (id: string | null) => void;
  setSelectedEdge: (id: string | null) => void;
  setSelectedResultRun: (id: string | null) => void;
  setExplorerTab: (tab: ExplorerTab) => void;
  setExplorerCollapsed: (collapsed: boolean) => void;
  setInspectorCollapsed: (collapsed: boolean) => void;
  setExplorerWidth: (width: number) => void;
  setUiPreferences: (patch: Partial<UiPreferences>) => void;
  setResultWorkspaceState: (patch: Partial<ResultWorkspaceState>) => void;
  setMethodSetupState: (patch: Partial<MethodSetupState>) => void;
  applyMethodPreset: (preset: MethodPresetId) => void;
  setOnboardingState: (patch: Partial<OnboardingState>) => void;
  setLargeModelViewState: (patch: Partial<LargeModelViewState>) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setShortcutOverlayOpen: (open: boolean) => void;
  setActiveDesktopMenu: (menu: DesktopMenuId | null) => void;
  setActiveDesktopDialog: (dialog: DesktopDialogId, payload?: Record<string, unknown> | null) => void;
  setDesktopCommandStatus: (status: Omit<DesktopCommandStatus, "timestamp"> | null) => void;
  setRunMonitor: (patch: Partial<RunMonitorState>, log?: Omit<RunMonitorLogEntry, "id" | "timestamp"> | null) => void;
  appendRunLog: (log: Omit<RunMonitorLogEntry, "id" | "timestamp">) => void;
  resetRunMonitor: () => void;
  pushToast: (toast: Omit<ToastNotification, "id">) => string;
  dismissToast: (id: string) => void;
  setDiagramMode: (mode: DiagramMode) => void;
  setDiagramTool: (tool: DiagramToolMode) => void;
  setDiagramOverlaySettings: (patch: Partial<DiagramOverlaySettings>) => void;
  setPublicationDiagramSettings: (patch: Partial<PublicationDiagramSettings>) => void;
  setDiagramViewport: (viewport: DiagramLayoutState["diagramViewport"]) => void;
  setDiagramTheme: (theme: DiagramLayoutState["diagramTheme"]) => void;
  setDiagramGridVisible: (showGrid: boolean) => void;
  setDiagramLayoutLocked: (layoutLocked: boolean) => void;
  checkpoint: () => void;
  undo: () => void;
  redo: () => void;
  onNodesChange: (changes: Array<NodeChange<Node<ConstructData>>>) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  reconnectPath: (edge: Edge, connection: Connection) => void;
  addPath: (source: string, target: string) => void;
  addCovariance: (source: string, target: string) => void;
  addTwoStageInteraction: (predictor: string, moderator: string, outcome: string) => AddTwoStageInteractionResult;
  addHigherOrderConstruct: (draft: NativeHigherOrderDraft) => AddHigherOrderConstructResult;
  updateConstruct: (id: string, patch: Partial<ConstructData>) => void;
  setConstructEstimandV4: (id: string, specification: SemModelV4ConstructAuthoring) => void;
  updateEdge: (id: string, patch: Partial<Edge>) => void;
  convertCovarianceToScientificV4: (id: string, endpoints?: { left: SemModelV4AuthoringEndpoint | null; right: SemModelV4AuthoringEndpoint | null }) => void;
  convertCovarianceToPresentationV4: (id: string) => void;
  setEdgeLabelOffset: (id: string, offset: { x: number; y: number }) => void;
  nudgeEdgeLabel: (id: string, delta: { x: number; y: number }) => void;
  resetEdgeLabel: (id: string) => void;
  resetAllEdgeLabels: () => void;
  addConstruct: (position?: XYPosition, indicators?: string[]) => void;
  addConstructsFromIndicators: (indicators: string[]) => void;
  addConstructsFromIndicatorGroups: (indicators: string[]) => void;
  duplicateSelected: () => void;
  removeSelection: () => void;
  reverseSelectedPath: () => void;
  setSelectedPathRouting: (routing: PathRouting) => void;
  setPathRouting: (id: string, routing: PathRouting) => void;
  alignSelectedConstructs: (target: AlignTarget) => void;
  distributeSelectedConstructs: (axis: DistributeAxis) => void;
  autoLayout: (direction?: "horizontal" | "vertical" | "smartpls") => void;
  moveIndicator: (constructId: string, indicator: string, position: XYPosition) => void;
  setIndicatorSide: (constructId: string, indicator: string, side: IndicatorSide) => void;
  setConstructIndicatorSide: (constructId: string, side: Exclude<IndicatorSide, "free">) => void;
  toggleConstructPinned: (constructId: string) => void;
  resetIndicatorLayout: (constructId: string, indicator?: string) => void;
  assignIndicator: (constructId: string, indicator: string) => void;
  assignIndicators: (constructId: string, indicators: string[]) => void;
  unassignIndicator: (constructId: string, indicator: string) => void;
  setDataset: (dataset: Dataset) => void;
  setDatasetCatalog: (datasets: Dataset[], versions: DatasetVersionRecord[]) => void;
  commitDatasetVersion: (mutation: DatasetVersionMutation) => void;
  setExplorerSelection: (selection: NativeExplorerSelection) => void;
  setProjectExplorer: (project: {
    projectModels: NativeCanonicalModelSpec[];
    activeModelId: string | null;
    modelPresentations: Record<string, NativeModelPresentation>;
    savedReports: NativeSavedReport[];
    explorerSelection?: NativeExplorerSelection;
  }) => void;
  installStandardSemModelV4Authority: (
    authority: StandardSemModelV4AuthorityRecordV1,
    layout?: StandardSemModelV4DiagramLayoutV1,
  ) => boolean;
  activateStandardSemModelV4Authorities: (
    installations: StandardSemModelV4ActivationV1[],
    activeModelId: string,
    projectName: string,
    datasetDescriptors?: StandardSemModelV4DatasetDescriptorV1[],
    scientificEditLockedModelIds?: readonly string[],
  ) => boolean;
  appendStandardSemModelV4Revision: (
    cas: StandardSemModelV4RevisionAppendCasV1,
    installation: StandardSemModelV4ActivationV1,
  ) => boolean;
  captureStandardSemModelV4SaveAuthorities: (
    modelIds: readonly string[],
  ) => Record<string, StandardSemModelV4SaveAuthorityV1> | null;
  reanchorStandardSemModelV4Authorities: (
    captured: Readonly<Record<string, StandardSemModelV4SaveAuthorityV1>>,
  ) => boolean;
  clearStandardSemModelV4Workspace: (modelIds: readonly string[]) => boolean;
  commitStandardSemModelV4Intent: (intent: StandardSemModelV4EditorIntentV1) => Promise<StandardSemModelV4AuthorityCommitResult>;
  standardSemModelV4OperationBlocker: (operation: StandardSemModelV4BlockedOperation) => string | null;
  switchProjectModel: (modelId: string) => boolean;
  addRun: (run: AnalysisRun) => void;
  setAnalysisSettings: (patch: Partial<AnalysisUiSettings>) => void;
  setProjectMeta: (name: string, path: string | null, projectId?: string | null) => void;
  setProjectWritable: (writable: boolean) => void;
  beginGeneralSemProjectDraftMode: (sourceProjectId: string) => boolean;
  clearGeneralSemProjectDraftMode: () => void;
  setGeneralSemPublicationPending: (pending: boolean) => void;
  setGeneralSemTransientWorkBlocker: (blocker: GeneralSemTransientWorkBlockerV1 | null) => void;
  closeProject: () => void;
  resetProject: () => void;
  loadProject: (project: { nodes: Array<Node<ConstructData>>; edges: Edge[]; dataset: Dataset; datasets?: Dataset[]; datasetVersions?: DatasetVersionRecord[]; projectModels?: NativeCanonicalModelSpec[]; activeModelId?: string | null; modelPresentations?: Record<string, NativeModelPresentation>; savedReports?: NativeSavedReport[]; explorerSelection?: NativeExplorerSelection; runs?: AnalysisRun[]; analysisSettings?: AnalysisUiSettings; diagramMode?: DiagramMode; diagramOverlaySettings?: Partial<DiagramOverlaySettings>; publicationDiagramSettings?: Partial<PublicationDiagramSettings>; diagramLayout?: Partial<DiagramLayoutState>; preserveGeneralSemProjectDraftMode?: GeneralSemProjectDraftModeV1 }) => void;
}

const supportedAnalysisMethods = new Set<AnalysisMethodId>(["pls_pm", "bootstrap", "permutation", "pls_sample_size_power", "plsc", "wpls", "cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation", "predict", "mga", "ipma", "cbsem", "pca", "gsca", "regression", "nca"]);

const defaultAnalysisSettings: AnalysisUiSettings = { method: "pls_pm", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, seed: 20260718, workers: 1, confidenceLevel: 0.95, caseWeightColumn: null, groupColumn: null, groupAValue: null, groupBValue: null, ipmaTargets: null, groupMethods: "micom", groupPermutationSamples: 5_000, micomConfiguralConfirmed: false, segmentCount: 2, segmentStarts: 10, minimumSegmentShare: 0.10, cbsemModelType: "sem", cbsemMeanStructure: false, cbsemStandardization: "std_all", cbsemGroupColumn: null, cbsemInvarianceSteps: "configural,metric,scalar", cbsemBootstrapSamples: 0, pcaVariables: null, pcaComponentRule: "kaiser", pcaComponents: 2, pcaVarianceThreshold: 0.80, regressionType: "ols", regressionOutcome: null, regressionPredictors: null, regressionControls: null, regressionBootstrap: false, robustSe: "hc3", processModel: "mediation", processX: null, processM: null, processW: null, processGraph: null, ncaX: null, ncaY: null, ncaCeiling: "both", ncaPermutationSamples: 999, plsPowerScenarioIdentity: "prospective_two_construct_path", plsPowerPredictorConstruct: null, plsPowerOutcomeConstruct: null, plsPowerPredictorLoadings: null, plsPowerOutcomeLoadings: null, plsPowerPopulationPath: 0.30, plsPowerSampleSizeGrid: "50,100,150", plsPowerAlpha: 0.05, plsPowerTargetPower: 0.80, plsPowerMonteCarloReplicates: 250, plsPowerBootstrapReplicates: 199 };
const defaultDiagramOverlaySettings: DiagramOverlaySettings = { selectedRunId: null, mode: "model", precision: 3, showLoadings: true, showPathCoefficients: true, showPValues: false, showTValues: false, showRSquared: true, showWarnings: true, showWatermark: true };
const defaultPublicationDiagramSettings: PublicationDiagramSettings = { mode: "smartpls_result", precision: 3, overlayMode: "paths_r2", aspectRatio: "wide", palette: "grayscale", layoutSource: "current_canvas", showLoadings: true, showPathCoefficients: true, showRSquared: true, showValidationWatermark: true, showUnsupportedWarning: true, showRunProvenance: true };
const defaultUiPreferences: UiPreferences = {
  density: "compact",
  tableDensity: "compact",
  defaultPrecision: 4,
  showAdvancedHelp: true,
  experimentalLabsEnabled: false,
  recentPanels: ["models", "runs", "reports"],
  methodScopeDrawerOpen: false,
  showThresholdColors: true,
  focusDiagramMode: false,
  selectedExportPreset: "journal_figure",
};
const uiPreferencesStorageKey = "quickpls:native-ui-preferences:v1";

function normalizedUiPreferences(candidate: Partial<UiPreferences> = {}): UiPreferences {
  const precision = Number(candidate.defaultPrecision ?? defaultUiPreferences.defaultPrecision);
  return {
    ...defaultUiPreferences,
    ...candidate,
    density: candidate.density === "comfortable" ? "comfortable" : "compact",
    tableDensity: candidate.tableDensity === "comfortable" ? "comfortable" : "compact",
    defaultPrecision: Math.min(6, Math.max(2, Number.isFinite(precision) ? Math.trunc(precision) : defaultUiPreferences.defaultPrecision)),
    experimentalLabsEnabled: candidate.experimentalLabsEnabled === true,
  };
}

function loadUiPreferences(): UiPreferences {
  if (typeof window === "undefined") return defaultUiPreferences;
  try {
    const stored = window.localStorage.getItem(uiPreferencesStorageKey);
    return stored ? normalizedUiPreferences(JSON.parse(stored) as Partial<UiPreferences>) : defaultUiPreferences;
  } catch {
    return defaultUiPreferences;
  }
}

function persistUiPreferences(preferences: UiPreferences) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(uiPreferencesStorageKey, JSON.stringify(preferences));
  } catch {
    // Preferences remain available for the current session when storage is unavailable.
  }
}
const defaultResultWorkspaceState: ResultWorkspaceState = {
  selectedRunId: null,
  selectedTab: "overview",
  tableSearch: "",
  tableDensity: "compact",
  includeExperimental: false,
  selectedDetailRow: null,
  resultPrecision: 4,
  tableSort: null,
  activeInterpretationPanel: null,
  comparisonRunIds: [],
  showInterpretationColumns: true,
};
const defaultMethodSetupState: MethodSetupState = { mode: "basic", selectedPreset: "standard_pls", expandedSections: ["basic"] };
const defaultOnboardingState: OnboardingState = { dismissed: false, selectedDemo: "corporate_reputation", recentProjectCards: [] };
const defaultLargeModelViewState: LargeModelViewState = { indicatorsCollapsed: false, isolatedConstructId: null, neighborhoodMode: "off" };
const defaultRunMonitor: RunMonitorState = {
  status: "idle",
  phase: "Idle",
  message: "No calculation is currently running.",
  completedUnits: 0,
  totalUnits: 0,
  startedAt: null,
  completedAt: null,
  activeJobId: null,
  lastRunId: null,
  error: null,
  logs: [],
};

const emptyDataset: Dataset = {
  id: "empty",
  name: "No dataset loaded",
  columns: [],
  rows: [],
  missing: 0,
  rowCount: 0,
  kind: "raw",
  columnMetadata: [],
};

const normalizeDiagramOverlaySettings = (settings?: Partial<DiagramOverlaySettings>): DiagramOverlaySettings => ({
  ...defaultDiagramOverlaySettings,
  ...settings,
  precision: Math.min(6, Math.max(0, Math.trunc(settings?.precision ?? defaultDiagramOverlaySettings.precision))),
  selectedRunId: typeof settings?.selectedRunId === "string" ? settings.selectedRunId : null,
});

const normalizePublicationDiagramSettings = (settings?: Partial<PublicationDiagramSettings>): PublicationDiagramSettings => ({
  ...defaultPublicationDiagramSettings,
  ...settings,
  palette: settings?.palette === "monochrome" ? "grayscale" : settings?.palette ?? defaultPublicationDiagramSettings.palette,
  layoutSource: settings?.layoutSource === "tidy_publication" ? "tidy_publication" : "current_canvas",
  precision: Math.min(6, Math.max(0, Math.trunc(settings?.precision ?? defaultPublicationDiagramSettings.precision))),
});

function clonedProcessGraph(
  value: AnalysisUiSettings["processGraph"],
): NativeProcessGraphRelationshipConfig | null {
  if (!value || value.model !== "graph"
    || typeof value.focal_predictor !== "string"
    || !Array.isArray(value.paths)
    || !Array.isArray(value.moderators)
    || !Array.isArray(value.moderations)
    || value.continuous_product_centering !== "equation_complete_case_mean_v1") return null;
  return {
    model: "graph",
    focal_predictor: value.focal_predictor.trim(),
    paths: value.paths.map((path) => ({ from: path.from.trim(), to: path.to.trim() })),
    moderators: value.moderators.map((moderator) => ({
      variable: moderator.variable.trim(),
      scale: moderator.scale,
    })),
    moderations: value.moderations.map((moderation) => ({
      from: moderation.from.trim(),
      to: moderation.to.trim(),
      moderator: moderation.moderator.trim(),
      ...(moderation.conditioning_moderator?.trim()
        ? { conditioning_moderator: moderation.conditioning_moderator.trim() }
        : {}),
    })),
    continuous_product_centering: "equation_complete_case_mean_v1",
  };
}

const normalizeAnalysisSettings = (settings: Partial<AnalysisUiSettings>): AnalysisUiSettings => {
  const weightingScheme = settings.weightingScheme === "factor" || settings.weightingScheme === "pca" ? settings.weightingScheme : "path";
  const tolerance = Number.isFinite(settings.tolerance) ? settings.tolerance! : 1e-7;
  const maxIterations = Number.isFinite(settings.maxIterations) ? Math.trunc(settings.maxIterations!) : 3000;
  const preprocessing = settings.preprocessing === "mean_centered" || settings.preprocessing === "unstandardized" ? settings.preprocessing : "standardized";
  const bootstrapSamples = Number.isFinite(settings.bootstrapSamples) ? Math.trunc(settings.bootstrapSamples!) : defaultAnalysisSettings.bootstrapSamples;
  const studentizedInnerSamples = Number.isFinite(settings.studentizedInnerSamples) ? Math.trunc(settings.studentizedInnerSamples!) : defaultAnalysisSettings.studentizedInnerSamples;
  const permutationSamples = Number.isFinite(settings.permutationSamples) ? Math.trunc(settings.permutationSamples!) : defaultAnalysisSettings.permutationSamples;
  const seed = Number.isFinite(settings.seed) ? Math.trunc(settings.seed!) : defaultAnalysisSettings.seed;
  const workers = Number.isFinite(settings.workers) ? Math.trunc(settings.workers!) : defaultAnalysisSettings.workers;
  const confidenceLevel = Number.isFinite(settings.confidenceLevel) ? settings.confidenceLevel! : defaultAnalysisSettings.confidenceLevel;
  const caseWeightColumn = typeof settings.caseWeightColumn === "string" && settings.caseWeightColumn.trim() ? settings.caseWeightColumn.trim() : null;
  const groupColumn = typeof settings.groupColumn === "string" && settings.groupColumn.trim() ? settings.groupColumn.trim() : null;
  const groupAValue = typeof settings.groupAValue === "string" && settings.groupAValue.trim() ? settings.groupAValue.trim() : null;
  const groupBValue = typeof settings.groupBValue === "string" && settings.groupBValue.trim() ? settings.groupBValue.trim() : null;
  const ipmaTargets = typeof settings.ipmaTargets === "string" && settings.ipmaTargets.trim() ? settings.ipmaTargets.trim() : null;
  const groupMethods = typeof settings.groupMethods === "string"
    ? [...new Set(settings.groupMethods.split(",").map((token) => token.trim()).filter((token) => ["micom", "mga_permutation", "pls_pos", "fimix"].includes(token)))].join(",") || null
    : defaultAnalysisSettings.groupMethods;
  const groupPermutationSamples = Number.isFinite(settings.groupPermutationSamples) ? Math.trunc(settings.groupPermutationSamples!) : defaultAnalysisSettings.groupPermutationSamples!;
  const segmentCount = Number.isFinite(settings.segmentCount) ? Math.trunc(settings.segmentCount!) : defaultAnalysisSettings.segmentCount!;
  const segmentStarts = Number.isFinite(settings.segmentStarts) ? Math.trunc(settings.segmentStarts!) : defaultAnalysisSettings.segmentStarts!;
  const minimumSegmentShare = Number.isFinite(settings.minimumSegmentShare) ? settings.minimumSegmentShare! : defaultAnalysisSettings.minimumSegmentShare!;
  const cbsemModelType = settings.cbsemModelType === "cfa" ? "cfa" : defaultAnalysisSettings.cbsemModelType!;
  const cbsemMeanStructure = Boolean(settings.cbsemMeanStructure);
  const cbsemStandardization = settings.cbsemStandardization === "std_lv" ? "std_lv" : defaultAnalysisSettings.cbsemStandardization!;
  const cbsemGroupColumn = typeof settings.cbsemGroupColumn === "string" && settings.cbsemGroupColumn.trim() ? settings.cbsemGroupColumn.trim() : null;
  const cbsemInvarianceSteps = typeof settings.cbsemInvarianceSteps === "string" && settings.cbsemInvarianceSteps.trim() ? settings.cbsemInvarianceSteps.trim() : defaultAnalysisSettings.cbsemInvarianceSteps;
  const cbsemBootstrapSamples = Number.isFinite(settings.cbsemBootstrapSamples) ? Math.trunc(settings.cbsemBootstrapSamples!) : defaultAnalysisSettings.cbsemBootstrapSamples!;
  const pcaVariables = typeof settings.pcaVariables === "string" && settings.pcaVariables.trim() ? settings.pcaVariables.trim() : null;
  const pcaComponentRule = settings.pcaComponentRule === "fixed" || settings.pcaComponentRule === "variance_threshold" ? settings.pcaComponentRule : defaultAnalysisSettings.pcaComponentRule!;
  const pcaComponents = Number.isFinite(settings.pcaComponents) ? Math.trunc(settings.pcaComponents!) : defaultAnalysisSettings.pcaComponents!;
  const pcaVarianceThreshold = Number.isFinite(settings.pcaVarianceThreshold) ? settings.pcaVarianceThreshold! : defaultAnalysisSettings.pcaVarianceThreshold!;
  const regressionType = settings.regressionType === "logistic" || settings.regressionType === "process" ? settings.regressionType : defaultAnalysisSettings.regressionType!;
  const regressionOutcome = typeof settings.regressionOutcome === "string" && settings.regressionOutcome.trim() ? settings.regressionOutcome.trim() : null;
  const regressionPredictors = typeof settings.regressionPredictors === "string" && settings.regressionPredictors.trim() ? settings.regressionPredictors.trim() : null;
  const regressionControls = typeof settings.regressionControls === "string" && settings.regressionControls.trim() ? settings.regressionControls.trim() : null;
  const robustSe = settings.robustSe === "none" || settings.robustSe === "hc0" || settings.robustSe === "hc4" ? settings.robustSe : defaultAnalysisSettings.robustSe!;
  const processModel = settings.processModel === "moderation" || settings.processModel === "moderated_mediation" ? settings.processModel : defaultAnalysisSettings.processModel!;
  const processX = typeof settings.processX === "string" && settings.processX.trim() ? settings.processX.trim() : null;
  const processM = typeof settings.processM === "string" && settings.processM.trim() ? settings.processM.trim() : null;
  const processW = typeof settings.processW === "string" && settings.processW.trim() ? settings.processW.trim() : null;
  const processGraph = clonedProcessGraph(settings.processGraph);
  const ncaX = typeof settings.ncaX === "string" && settings.ncaX.trim() ? settings.ncaX.trim() : null;
  const ncaY = typeof settings.ncaY === "string" && settings.ncaY.trim() ? settings.ncaY.trim() : null;
  const ncaCeiling = settings.ncaCeiling === "ce_fdh" || settings.ncaCeiling === "cr_fdh" ? settings.ncaCeiling : defaultAnalysisSettings.ncaCeiling!;
  const ncaPermutationSamples = Number.isFinite(settings.ncaPermutationSamples) ? Math.trunc(settings.ncaPermutationSamples!) : defaultAnalysisSettings.ncaPermutationSamples!;
  const plsPowerScenarioIdentity = typeof settings.plsPowerScenarioIdentity === "string" && settings.plsPowerScenarioIdentity.trim() ? settings.plsPowerScenarioIdentity.trim() : defaultAnalysisSettings.plsPowerScenarioIdentity!;
  const plsPowerPredictorConstruct = typeof settings.plsPowerPredictorConstruct === "string" && settings.plsPowerPredictorConstruct.trim() ? settings.plsPowerPredictorConstruct.trim() : null;
  const plsPowerOutcomeConstruct = typeof settings.plsPowerOutcomeConstruct === "string" && settings.plsPowerOutcomeConstruct.trim() ? settings.plsPowerOutcomeConstruct.trim() : null;
  const plsPowerPredictorLoadings = typeof settings.plsPowerPredictorLoadings === "string" && settings.plsPowerPredictorLoadings.trim() ? settings.plsPowerPredictorLoadings.trim() : null;
  const plsPowerOutcomeLoadings = typeof settings.plsPowerOutcomeLoadings === "string" && settings.plsPowerOutcomeLoadings.trim() ? settings.plsPowerOutcomeLoadings.trim() : null;
  const plsPowerPopulationPath = Number.isFinite(settings.plsPowerPopulationPath) ? settings.plsPowerPopulationPath! : defaultAnalysisSettings.plsPowerPopulationPath!;
  const plsPowerSampleSizeGrid = typeof settings.plsPowerSampleSizeGrid === "string" && settings.plsPowerSampleSizeGrid.trim() ? settings.plsPowerSampleSizeGrid.trim() : defaultAnalysisSettings.plsPowerSampleSizeGrid!;
  const plsPowerAlpha = Number.isFinite(settings.plsPowerAlpha) ? settings.plsPowerAlpha! : defaultAnalysisSettings.plsPowerAlpha!;
  const plsPowerTargetPower = Number.isFinite(settings.plsPowerTargetPower) ? settings.plsPowerTargetPower! : defaultAnalysisSettings.plsPowerTargetPower!;
  const plsPowerMonteCarloReplicates = Number.isFinite(settings.plsPowerMonteCarloReplicates) ? Math.trunc(settings.plsPowerMonteCarloReplicates!) : defaultAnalysisSettings.plsPowerMonteCarloReplicates!;
  const powerBootstrapReplicates = Number.isFinite(settings.plsPowerBootstrapReplicates) ? Math.trunc(settings.plsPowerBootstrapReplicates!) : defaultAnalysisSettings.plsPowerBootstrapReplicates!;
  const clampedPowerBootstrapReplicates = Math.min(1_999, Math.max(99, powerBootstrapReplicates));
  const plsPowerBootstrapReplicates = clampedPowerBootstrapReplicates % 2 === 0 ? Math.max(99, clampedPowerBootstrapReplicates - 1) : clampedPowerBootstrapReplicates;
  const method = typeof settings.method === "string" && supportedAnalysisMethods.has(settings.method as AnalysisMethodId) ? settings.method as AnalysisMethodId : defaultAnalysisSettings.method;
  const clampedStudentized = Math.min(999, Math.max(99, studentizedInnerSamples));
  const normalizedStudentized = studentizedInnerSamples === 0 ? 0 : clampedStudentized % 2 === 0 ? Math.min(999, clampedStudentized + 1) : clampedStudentized;
  const normalizedBootstrap = normalizedStudentized > 0 ? Math.max(999, bootstrapSamples) : bootstrapSamples;
  return {
    method,
    weightingScheme,
    tolerance: Math.min(0.01, Math.max(1e-12, tolerance)),
    maxIterations: Math.min(100000, Math.max(100, maxIterations)),
    preprocessing,
    bootstrapSamples: Math.min(10000, Math.max(0, normalizedBootstrap)),
    studentizedInnerSamples: normalizedStudentized,
    permutationSamples: permutationSamples === 0 ? 0 : Math.min(10000, Math.max(99, permutationSamples)),
    seed: Math.min(4294967295, Math.max(0, seed)),
    workers: Math.min(64, Math.max(1, workers)),
    confidenceLevel: Math.min(0.999, Math.max(0.8, confidenceLevel)),
    caseWeightColumn,
    groupColumn,
    groupAValue,
    groupBValue,
    ipmaTargets,
    groupMethods,
    groupPermutationSamples: Math.min(10000, Math.max(5000, groupPermutationSamples)),
    micomConfiguralConfirmed: settings.micomConfiguralConfirmed === true,
    segmentCount: Math.min(5, Math.max(2, segmentCount)),
    segmentStarts: Math.min(50, Math.max(1, segmentStarts)),
    minimumSegmentShare: Math.min(0.4, Math.max(0.05, minimumSegmentShare)),
    cbsemModelType,
    cbsemMeanStructure,
    cbsemStandardization,
    cbsemGroupColumn,
    cbsemInvarianceSteps,
    cbsemBootstrapSamples: Math.min(10000, Math.max(0, cbsemBootstrapSamples)),
    pcaVariables,
    pcaComponentRule,
    pcaComponents: Math.min(50, Math.max(1, pcaComponents)),
    pcaVarianceThreshold: Math.min(0.999, Math.max(0.01, pcaVarianceThreshold)),
    regressionType,
    regressionOutcome,
    regressionPredictors,
    regressionControls,
    regressionBootstrap: settings.regressionBootstrap === true,
    robustSe,
    processModel,
    processX,
    processM,
    processW,
    processGraph,
    ncaX,
    ncaY,
    ncaCeiling,
    ncaPermutationSamples: Math.min(10000, Math.max(1, ncaPermutationSamples)),
    plsPowerScenarioIdentity,
    plsPowerPredictorConstruct,
    plsPowerOutcomeConstruct,
    plsPowerPredictorLoadings,
    plsPowerOutcomeLoadings,
    plsPowerPopulationPath: Math.min(0.80, Math.max(-0.80, plsPowerPopulationPath)),
    plsPowerSampleSizeGrid,
    plsPowerAlpha: Math.min(0.10, Math.max(0.001, plsPowerAlpha)),
    plsPowerTargetPower: Math.min(0.99, Math.max(0.50, plsPowerTargetPower)),
    plsPowerMonteCarloReplicates: Math.min(10_000, Math.max(100, plsPowerMonteCarloReplicates)),
    plsPowerBootstrapReplicates,
  };
};

let standardSemModelV4Epoch = 0;
const standardSemModelV4Queues = new Map<string, Promise<void>>();

const nextStandardSemModelV4Epoch = () => {
  standardSemModelV4Epoch += 1;
  return standardSemModelV4Epoch;
};

const activeStandardSemModelV4Authority = (state: WorkspaceState) => state.activeModelId
  ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
  : null;

const standardSemModelV4Layout = (
  modelId: string,
  diagramLayout: DiagramLayoutState,
) => parseStandardSemModelV4DiagramLayoutV1({
  schema_version: 1,
  model_id: modelId,
  diagram_layout: diagramLayout,
});

const sameStandardSemModelV4Layout = (
  left: StandardSemModelV4DiagramLayoutV1,
  right: StandardSemModelV4DiagramLayoutV1,
) => JSON.stringify(left) === JSON.stringify(right);

const currentStandardSemModelV4Layout = (
  state: WorkspaceState,
  modelId: string,
) => state.activeModelId === modelId
  ? standardSemModelV4Layout(modelId, state.diagramLayout)
  : state.standardSemModelV4Layouts[modelId];

const validStandardSemModelV4Readiness = (
  readiness: "ready" | "authoring_only",
  scientificSha256: string | null,
) => readiness === "ready"
  ? typeof scientificSha256 === "string" && /^[0-9a-f]{64}$/.test(scientificSha256)
  : scientificSha256 === null;

const datasetFromStandardSemModelV4Descriptor = (
  descriptor: StandardSemModelV4DatasetDescriptorV1,
): Dataset => ({
  id: descriptor.id,
  name: descriptor.name,
  columns: [...descriptor.columns],
  columnMetadata: descriptor.columnMetadata.map((column) => ({ ...column })),
  rows: [],
  // Descriptor-only schema-6 activation has no resident row or missing-cell values.
  missing: Number.NaN,
  rowCount: descriptor.rowCount,
  fingerprint: descriptor.fingerprint,
  kind: descriptor.kind,
  sampleSize: descriptor.sampleSize,
});

const historySnapshot = (state: WorkspaceState): HistorySnapshot => {
  const authority = activeStandardSemModelV4Authority(state);
  return authority && state.activeModelId
    ? {
        kind: "standard_sem_model_v4",
        modelId: state.activeModelId,
        authority,
        persistence: {
          readiness: state.standardSemModelV4Persistence[state.activeModelId]?.readiness ?? "authoring_only",
          scientificSha256: state.standardSemModelV4Persistence[state.activeModelId]?.scientificSha256 ?? null,
        },
        diagramLayout: state.diagramLayout,
      }
    : {
        kind: "legacy_graph",
        nodes: state.nodes,
        edges: state.edges,
        diagramLayout: state.diagramLayout,
      };
};

const historyPatch = (state: WorkspaceState) => ({
  past: [...state.past.slice(-49), historySnapshot(state)],
  future: [],
});

const queueStandardSemModelV4Commit = <T>(
  modelId: string,
  epoch: number,
  operation: () => Promise<T>,
): Promise<T> => {
  const queueKey = `${modelId}\0${epoch}`;
  const previous = standardSemModelV4Queues.get(queueKey) ?? Promise.resolve();
  const current = previous.then(operation);
  const barrier = current.then(() => undefined, () => undefined);
  standardSemModelV4Queues.set(queueKey, barrier);
  void barrier.then(() => {
    if (standardSemModelV4Queues.get(queueKey) === barrier) standardSemModelV4Queues.delete(queueKey);
  });
  return current;
};

const standardSemModelV4OperationMessage = (
  modelId: string,
  operation: StandardSemModelV4BlockedOperation,
) => {
  const labels: Record<StandardSemModelV4BlockedOperation, string> = {
    schema5_save: "Schema-5 save",
    schema5_autosave: "Schema-5 autosave",
    calculation: "calculation",
    legacy_graph_serialization: "legacy graph serialization",
  };
  return `${labels[operation]} is blocked for strict Standard SemModelV4 model '${modelId}'. Use the schema-6 authority workflow.`;
};

const interactionNodes = (nodes: Array<Node<ConstructData>>) => nodes.filter((node) =>
  node.data.semantic === "interaction" && node.data.interaction,
);

const diagramInteractionOperands = (interaction: NonNullable<ConstructData["interaction"]>): readonly string[] =>
  interaction.kind === "interaction_v2"
    ? interaction.operands
    : [interaction.predictor, interaction.moderator];

type RequiredDiagramInteractionPath = { source: string; target: string; relationId?: string };

const requiredDiagramInteractionPaths = (node: Node<ConstructData>): RequiredDiagramInteractionPath[] => {
  const interaction = node.data.interaction!;
  const operands = diagramInteractionOperands(interaction);
  const required: RequiredDiagramInteractionPath[] = [
    {
      source: operands[0]!,
      target: interaction.outcome,
      ...(interaction.focalRelationId ? { relationId: interaction.focalRelationId } : {}),
    },
    { source: node.id, target: interaction.outcome },
  ];
  if (interaction.kind !== "interaction_v2" || interaction.hierarchyPolicy !== "none") {
    required.push(...operands.slice(1).map((operand) => ({ source: operand, target: interaction.outcome })));
  }
  return required;
};

const matchesRequiredDiagramInteractionPath = (
  edge: Pick<Edge, "id" | "source" | "target">,
  required: RequiredDiagramInteractionPath,
) => edge.source === required.source
  && edge.target === required.target
  && (!required.relationId || edge.id === required.relationId);

const requiredLowerOrderInteractionNodeIds = (
  node: Node<ConstructData>,
  nodes: Array<Node<ConstructData>>,
): string[] => {
  const interaction = node.data.interaction!;
  if (interaction.kind !== "interaction_v2" || interaction.hierarchyPolicy !== "strong" || interaction.operands.length <= 2) return [];
  return interaction.operands.flatMap((_, omitted) => {
    const required = new Set(interaction.operands.filter((__, index) => index !== omitted));
    const lowerOrder = interactionNodes(nodes).find((candidate) => {
      if (candidate.id === node.id || candidate.data.interaction!.outcome !== interaction.outcome) return false;
      const candidateInteraction = candidate.data.interaction!;
      const candidateOperands = diagramInteractionOperands(candidateInteraction);
      if (candidateOperands.length !== required.size
        || candidateOperands.some((operand) => !required.has(operand))) return false;
      return candidateOperands.length <= 2
        || candidateInteraction.kind === "interaction_v2" && candidateInteraction.hierarchyPolicy === "strong";
    });
    return lowerOrder ? [lowerOrder.id] : [];
  });
};

const uniqueStableGraphId = (base: string, occupiedIds: ReadonlySet<string>) => {
  const normalized = base.replace(/[^a-zA-Z0-9_-]/g, "-") || "interaction";
  if (!occupiedIds.has(normalized)) return normalized;
  let suffix = 2;
  while (occupiedIds.has(`${normalized}-${suffix}`)) suffix += 1;
  return `${normalized}-${suffix}`;
};

const touchesGeneratedInteraction = (nodes: Array<Node<ConstructData>>, source: string, target: string) => {
  const generatedIds = new Set(interactionNodes(nodes).map((node) => node.id));
  return generatedIds.has(source) || generatedIds.has(target);
};

const requiredInteractionEdge = (nodes: Array<Node<ConstructData>>, edge: Pick<Edge, "id" | "source" | "target">) => interactionNodes(nodes).some((node) => {
  return requiredDiagramInteractionPaths(node)
    .some((required) => matchesRequiredDiagramInteractionPath(edge, required));
});

const cascadingInteractionNodeIds = (
  nodes: Array<Node<ConstructData>>,
  removedNodeIds: ReadonlySet<string>,
  removedEdges: readonly Pick<Edge, "id" | "source" | "target">[],
) => {
  const interactions = interactionNodes(nodes);
  const cascadingIds = new Set(interactions
    .filter((node) => {
    const interaction = node.data.interaction!;
    const operands = diagramInteractionOperands(interaction);
    return removedNodeIds.has(node.id)
      || operands.some((operand) => removedNodeIds.has(operand))
      || removedNodeIds.has(interaction.outcome)
      || removedEdges.some((edge) => requiredDiagramInteractionPaths(node)
        .some((required) => matchesRequiredDiagramInteractionPath(edge, required)));
    })
    .map((node) => node.id));
  let changed = true;
  while (changed) {
    changed = false;
    for (const node of interactions) {
      if (cascadingIds.has(node.id)) continue;
      if (requiredLowerOrderInteractionNodeIds(node, nodes).some((id) => cascadingIds.has(id))) {
        cascadingIds.add(node.id);
        changed = true;
      }
    }
  }
  return cascadingIds;
};

const cascadingHigherOrderNodeIds = (
  nodes: Array<Node<ConstructData>>,
  removedNodeIds: ReadonlySet<string>,
) => new Set(nodes
  .filter((node) => node.data.semantic === "higher_order" && node.data.higherOrder)
  .filter((node) => removedNodeIds.has(node.id)
    || node.data.higherOrder!.components.some((component) => removedNodeIds.has(component)))
  .map((node) => node.id));

const syncedDiagramLayout = (nodes: Array<Node<ConstructData>>, edges: Edge[], existing?: Partial<DiagramLayoutState>) =>
  defaultDiagramLayout(nodes, edges, existing);

const constructSize = { width: 170, height: 118 };

const routeStyleForType = (routing: PathRouting): DiagramLayoutState["edgeLayouts"][string]["routing"] =>
  routing === "smoothstep" ? "orthogonal" : routing === "default" ? "curved" : "straight";

const setPathRoutingState = (state: WorkspaceState, id: string, routing: PathRouting) => {
  if (!state.edges.some((edge) => edge.id === id)) return state;
  return {
    ...historyPatch(state),
    edges: state.edges.map((edge) => edge.id === id ? { ...edge, type: routing } : edge),
    diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
      ...state.diagramLayout,
      edgeLayouts: {
        ...state.diagramLayout.edgeLayouts,
        [id]: {
          ...(state.diagramLayout.edgeLayouts[id] ?? { routing: routeStyleForType(routing) }),
          routing: routeStyleForType(routing),
          pinned: routing !== "straight",
        },
      },
    }),
  };
};

const selectedConstructIds = (state: WorkspaceState) => new Set([
  ...state.nodes.filter((node) => node.selected).map((node) => node.id),
  ...(state.selectedNodeId ? [state.selectedNodeId] : []),
]);

const selectedConstructs = (state: WorkspaceState) => {
  const ids = selectedConstructIds(state);
  return state.nodes.filter((node) => ids.has(node.id));
};

const nextConstructName = (nodes: Array<Node<ConstructData>>) => {
  let number = nodes.length + 1;
  const names = new Set(nodes.map((node) => node.data.shortName));
  while (names.has(`C${number}`)) number += 1;
  return { label: `Construct ${number}`, shortName: `C${number}` };
};

const nextConstructPosition = (nodes: Array<Node<ConstructData>>): XYPosition => {
  for (let row = 0; row < 12; row += 1) {
    for (let column = 0; column < 8; column += 1) {
      const candidate = { x: 80 + column * 220, y: 85 + row * 170 };
      const occupied = nodes.some((node) => Math.abs(node.position.x - candidate.x) < 190 && Math.abs(node.position.y - candidate.y) < 140);
      if (!occupied) return candidate;
    }
  }
  return { x: 80, y: 85 + nodes.length * 170 };
};

const snapPosition = (position: XYPosition): XYPosition => ({
  x: Math.round(position.x / 10) * 10,
  y: Math.round(position.y / 10) * 10,
});

const constructPositionIsOpen = (candidate: XYPosition, nodes: Array<Node<ConstructData>>) =>
  nodes.every((node) => Math.abs(node.position.x - candidate.x) >= 190 || Math.abs(node.position.y - candidate.y) >= 140);

const nearestOpenConstructPosition = (requested: XYPosition, nodes: Array<Node<ConstructData>>): XYPosition => {
  const origin = snapPosition(requested);
  if (constructPositionIsOpen(origin, nodes)) return origin;
  const offsets = [
    { x: 220, y: 0 },
    { x: 0, y: 170 },
    { x: 220, y: 170 },
    { x: -220, y: 0 },
    { x: 0, y: -170 },
    { x: -220, y: 170 },
    { x: 220, y: -170 },
    { x: -220, y: -170 },
  ];
  for (let ring = 1; ring <= 6; ring += 1) {
    for (const offset of offsets) {
      const candidate = snapPosition({ x: origin.x + offset.x * ring, y: origin.y + offset.y * ring });
      if (constructPositionIsOpen(candidate, nodes)) return candidate;
    }
  }
  return nextConstructPosition(nodes);
};

const constructIdFromIndicator = (indicator: string, nodes: Array<Node<ConstructData>>) => {
  const base = `construct-${indicator.toLowerCase().replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "") || "indicator"}`;
  const ids = new Set(nodes.map((node) => node.id));
  let id = base;
  let suffix = 2;
  while (ids.has(id)) {
    id = `${base}-${suffix}`;
    suffix += 1;
  }
  return id;
};

const indicatorGroupKey = (indicator: string) => {
  const clean = indicator.trim();
  const prefix = clean.match(/^[A-Za-z]+/)?.[0] ?? clean;
  return prefix.replace(/[^a-zA-Z0-9]/g, "").toUpperCase().slice(0, 8) || "GROUP";
};

const validUniqueIndicators = (
  indicators: string[],
  dataset: Dataset,
  reservedGroupColumn?: string | null,
) => {
  const groupColumn = reservedGroupColumn?.trim() ?? "";
  return [...new Set(indicators)].filter((indicator): indicator is string =>
    typeof indicator === "string"
    && dataset.columns.includes(indicator)
    && indicator !== groupColumn,
  );
};

const upsertDatasetCatalog = (catalog: Dataset[], dataset: Dataset) => {
  const index = catalog.findIndex((candidate) => candidate.id === dataset.id);
  if (index < 0) return [...catalog, dataset];
  return catalog.map((candidate, candidateIndex) => candidateIndex === index ? dataset : candidate);
};

const sampleProjectModelId = "quickpls-corporate-reputation-model-v1";
const sampleProjectModel = buildNativeRecipeModel(
  sampleProjectModelId,
  "Corporate Reputation Model",
  initialNodes,
  initialEdges,
);
const sampleModelPresentation = currentNativeModelPresentation(
  initialNodes,
  initialEdges,
  syncedDiagramLayout(initialNodes, initialEdges),
);

export const useWorkspace = create<WorkspaceState>()((set, get) => ({
  view: "welcome",
  workflowDestinationContext: null,
  workflowCommandContext: null,
  nodes: initialNodes,
  edges: initialEdges,
  selectedNodeId: "satisfaction",
  selectedEdgeId: null,
  selectedResultRunId: null,
  explorerTab: "constructs",
  explorerCollapsed: false,
  inspectorCollapsed: false,
  explorerWidth: 330,
  uiPreferences: loadUiPreferences(),
  resultWorkspaceState: defaultResultWorkspaceState,
  methodSetupState: defaultMethodSetupState,
  onboardingState: defaultOnboardingState,
  largeModelViewState: defaultLargeModelViewState,
  commandPaletteOpen: false,
  shortcutOverlayOpen: false,
  activeDesktopMenu: null,
  activeDesktopDialog: null,
  desktopDialogPayload: null,
  desktopCommandStatus: null,
  runMonitor: defaultRunMonitor,
  toasts: [],
  diagramMode: "sem",
  diagramTool: "select",
  diagramOverlaySettings: defaultDiagramOverlaySettings,
  publicationDiagramSettings: defaultPublicationDiagramSettings,
  diagramLayout: syncedDiagramLayout(initialNodes, initialEdges),
  dataset: sampleDataset,
  datasetCatalog: [sampleDataset],
  datasetVersions: [],
  projectModels: [sampleProjectModel],
  activeModelId: sampleProjectModelId,
  modelPresentations: { [sampleProjectModelId]: sampleModelPresentation },
  standardSemModelV4Authorities: {},
  standardSemModelV4ScientificEditLocks: {},
  standardSemModelV4Layouts: {},
  standardSemModelV4Epochs: {},
  standardSemModelV4Persistence: {},
  standardSemModelV4DatasetDescriptors: {},
  datasetDescriptorOnly: false,
  savedReports: [],
  explorerSelection: { kind: "model", modelId: sampleProjectModelId },
  runs: [],
  analysisSettings: defaultAnalysisSettings,
  projectName: "Corporate Reputation Study",
  projectId: null,
  projectPath: null,
  projectWritable: true,
  generalSemProjectDraftMode: null,
  generalSemPublicationPending: false,
  generalSemTransientWorkBlocker: null,
  past: [],
  future: [],
  setView: (view, context) => set((state) => {
    const nextView = view === "groups" ? "runs" : view;
    const destinationContext = context
      ? { ...context, to: nextView, timestamp: Date.now() }
      : state.view === nextView ? state.workflowDestinationContext : null;
    const workflowCommandContext = state.view === nextView ? state.workflowCommandContext : null;
    return view === "groups"
      ? { view: nextView, workflowDestinationContext: destinationContext, workflowCommandContext, resultWorkspaceState: { ...state.resultWorkspaceState, selectedTab: "groups" } }
      : { view: nextView, workflowDestinationContext: destinationContext, workflowCommandContext };
  }),
  setWorkflowCommandContext: (context) => set({ workflowCommandContext: context ? { ...context, timestamp: Date.now() } : null }),
  clearWorkflowFeedback: () => set({ workflowDestinationContext: null, workflowCommandContext: null }),
  setSelectedNode: (selectedNodeId) => set({ selectedNodeId, selectedEdgeId: null }),
  setSelectedEdge: (selectedEdgeId) => set({ selectedEdgeId, selectedNodeId: null }),
  setSelectedResultRun: (selectedResultRunId) => set((state) => ({ selectedResultRunId, diagramOverlaySettings: { ...state.diagramOverlaySettings, selectedRunId: selectedResultRunId } })),
  setExplorerTab: (explorerTab) => set({ explorerTab }),
  setExplorerCollapsed: (explorerCollapsed) => set({ explorerCollapsed }),
  setInspectorCollapsed: (inspectorCollapsed) => set({ inspectorCollapsed }),
  setExplorerWidth: (explorerWidth) => set({ explorerWidth: Math.min(430, Math.max(250, Math.trunc(explorerWidth))) }),
  setUiPreferences: (patch) => set((state) => {
    if (state.generalSemTransientWorkBlocker && patch.experimentalLabsEnabled === false) return {};
    const uiPreferences = normalizedUiPreferences({ ...state.uiPreferences, ...patch });
    persistUiPreferences(uiPreferences);
    return { uiPreferences };
  }),
  setResultWorkspaceState: (patch) => set((state) => ({ resultWorkspaceState: { ...state.resultWorkspaceState, ...patch } })),
  setMethodSetupState: (patch) => set((state) => ({ methodSetupState: { ...state.methodSetupState, ...patch, expandedSections: patch.expandedSections ?? state.methodSetupState.expandedSections } })),
  applyMethodPreset: (preset) => set((state) => {
    const presets: Record<MethodPresetId, Partial<AnalysisUiSettings>> = {
      standard_pls: { method: "pls_pm", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0 },
      pls_bootstrap: { method: "bootstrap", bootstrapSamples: 5000, studentizedInnerSamples: 0, permutationSamples: 0 },
      plspredict: { method: "predict", groupMethods: "pls_pos", segmentCount: 2, segmentStarts: 10 },
      micom_mga: { method: "mga", groupMethods: "micom", groupPermutationSamples: 5_000, micomConfiguralConfirmed: false },
      cbsem_cfa: { method: "cbsem", cbsemModelType: "cfa", cbsemStandardization: "std_all", cbsemMeanStructure: false },
      ols_regression: { method: "regression", regressionType: "ols", robustSe: "hc3" },
      nca: { method: "nca", ncaCeiling: "both", ncaPermutationSamples: 999 },
    };
    return {
      analysisSettings: normalizeAnalysisSettings({ ...state.analysisSettings, ...presets[preset] }),
      methodSetupState: { ...state.methodSetupState, selectedPreset: preset, mode: preset === "standard_pls" ? "basic" : state.methodSetupState.mode },
    };
  }),
  setOnboardingState: (patch) => set((state) => ({ onboardingState: { ...state.onboardingState, ...patch } })),
  setLargeModelViewState: (patch) => set((state) => ({ largeModelViewState: { ...state.largeModelViewState, ...patch } })),
  setCommandPaletteOpen: (commandPaletteOpen) => set({ commandPaletteOpen }),
  setShortcutOverlayOpen: (shortcutOverlayOpen) => set({ shortcutOverlayOpen }),
  setActiveDesktopMenu: (activeDesktopMenu) => set({ activeDesktopMenu }),
  setActiveDesktopDialog: (activeDesktopDialog, desktopDialogPayload = null) => set({ activeDesktopDialog, desktopDialogPayload, activeDesktopMenu: null }),
  setDesktopCommandStatus: (desktopCommandStatus) => set({ desktopCommandStatus: desktopCommandStatus ? { ...desktopCommandStatus, timestamp: Date.now() } : null }),
  setRunMonitor: (patch, log = null) => set((state) => {
    const entry = log ? { id: crypto.randomUUID(), timestamp: new Date().toISOString(), ...log } : null;
    return {
      runMonitor: {
        ...state.runMonitor,
        ...patch,
        logs: entry ? [entry, ...state.runMonitor.logs].slice(0, 80) : state.runMonitor.logs,
      },
    };
  }),
  appendRunLog: (log) => set((state) => ({
    runMonitor: {
      ...state.runMonitor,
      logs: [{ id: crypto.randomUUID(), timestamp: new Date().toISOString(), ...log }, ...state.runMonitor.logs].slice(0, 80),
    },
  })),
  resetRunMonitor: () => set({ runMonitor: defaultRunMonitor }),
  pushToast: (toast) => {
    const id = crypto.randomUUID();
    set((state) => ({ toasts: [{ id, ...toast }, ...state.toasts].slice(0, 2) }));
    return id;
  },
  dismissToast: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
  setDiagramMode: (diagramMode) => set((state) => ({
    diagramMode,
    diagramTool: diagramMode === "smartpls_result" ? "select" : state.diagramTool,
    diagramOverlaySettings: diagramMode === "smartpls_result"
      ? { ...state.diagramOverlaySettings, mode: state.selectedResultRunId ? "paths_r2" : "model" }
      : state.diagramOverlaySettings,
  })),
  setDiagramTool: (diagramTool) => set({ diagramTool }),
  setDiagramOverlaySettings: (patch) => set((state) => {
    const diagramOverlaySettings = normalizeDiagramOverlaySettings({ ...state.diagramOverlaySettings, ...patch });
    return { diagramOverlaySettings, selectedResultRunId: diagramOverlaySettings.selectedRunId };
  }),
  setPublicationDiagramSettings: (patch) => set((state) => ({ publicationDiagramSettings: normalizePublicationDiagramSettings({ ...state.publicationDiagramSettings, ...patch }) })),
  setDiagramViewport: (diagramViewport) => set((state) => ({ diagramLayout: { ...state.diagramLayout, diagramViewport } })),
  setDiagramTheme: (diagramTheme) => set((state) => ({ diagramLayout: { ...state.diagramLayout, diagramTheme } })),
  setDiagramGridVisible: (showGrid) => set((state) => ({ diagramLayout: { ...state.diagramLayout, showGrid } })),
  setDiagramLayoutLocked: (layoutLocked) => set((state) => ({ diagramLayout: { ...state.diagramLayout, layoutLocked } })),
  checkpoint: () => set((state) => historyPatch(state)),
  undo: () => set((state) => {
    const previous = state.past.at(-1);
    if (!previous) return state;
    const current = historySnapshot(state);
    if (previous.kind === "standard_sem_model_v4") {
      if (state.activeModelId !== previous.modelId) return state;
      const layout = standardSemModelV4Layout(previous.modelId, previous.diagramLayout);
      const projected = projectStandardSemModelV4DiagramV1(previous.authority, layout);
      return {
        nodes: projected.nodes,
        edges: projected.edges,
        diagramLayout: projected.diagramLayout,
        standardSemModelV4Authorities: {
          ...state.standardSemModelV4Authorities,
          [previous.modelId]: previous.authority,
        },
        standardSemModelV4Layouts: {
          ...state.standardSemModelV4Layouts,
          [previous.modelId]: layout,
        },
        standardSemModelV4Epochs: {
          ...state.standardSemModelV4Epochs,
          [previous.modelId]: nextStandardSemModelV4Epoch(),
        },
        standardSemModelV4Persistence: {
          ...state.standardSemModelV4Persistence,
          [previous.modelId]: {
            ...(state.standardSemModelV4Persistence[previous.modelId] ?? {
              anchorModelDocumentSha256: previous.authority.model_document_sha256,
              anchorLayout: layout,
            }),
            ...previous.persistence,
          },
        },
        past: state.past.slice(0, -1),
        future: [current, ...state.future].slice(0, 50),
        selectedNodeId: null,
        selectedEdgeId: null,
        selectedResultRunId: null,
      };
    }
    return {
      nodes: previous.nodes,
      edges: previous.edges,
      diagramLayout: previous.diagramLayout,
      past: state.past.slice(0, -1),
      future: [current, ...state.future].slice(0, 50),
      selectedNodeId: null,
      selectedEdgeId: null,
      selectedResultRunId: null,
    };
  }),
  redo: () => set((state) => {
    const next = state.future[0];
    if (!next) return state;
    const current = historySnapshot(state);
    if (next.kind === "standard_sem_model_v4") {
      if (state.activeModelId !== next.modelId) return state;
      const layout = standardSemModelV4Layout(next.modelId, next.diagramLayout);
      const projected = projectStandardSemModelV4DiagramV1(next.authority, layout);
      return {
        nodes: projected.nodes,
        edges: projected.edges,
        diagramLayout: projected.diagramLayout,
        standardSemModelV4Authorities: {
          ...state.standardSemModelV4Authorities,
          [next.modelId]: next.authority,
        },
        standardSemModelV4Layouts: {
          ...state.standardSemModelV4Layouts,
          [next.modelId]: layout,
        },
        standardSemModelV4Epochs: {
          ...state.standardSemModelV4Epochs,
          [next.modelId]: nextStandardSemModelV4Epoch(),
        },
        standardSemModelV4Persistence: {
          ...state.standardSemModelV4Persistence,
          [next.modelId]: {
            ...(state.standardSemModelV4Persistence[next.modelId] ?? {
              anchorModelDocumentSha256: next.authority.model_document_sha256,
              anchorLayout: layout,
            }),
            ...next.persistence,
          },
        },
        past: [...state.past, current].slice(-50),
        future: state.future.slice(1),
        selectedNodeId: null,
        selectedEdgeId: null,
        selectedResultRunId: null,
      };
    }
    return {
      nodes: next.nodes,
      edges: next.edges,
      diagramLayout: next.diagramLayout,
      past: [...state.past, current].slice(-50),
      future: state.future.slice(1),
      selectedNodeId: null,
      selectedEdgeId: null,
      selectedResultRunId: null,
    };
  }),
  onNodesChange: (changes) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) {
      const presentationChanges = changes.filter((change) =>
        change.type === "position" || change.type === "select" || change.type === "dimensions",
      );
      if (presentationChanges.length === 0) return state;
      const nodes = applyNodeChanges(presentationChanges, state.nodes);
      return {
        nodes,
        diagramLayout: syncedDiagramLayout(nodes, state.edges, {
          ...state.diagramLayout,
          constructLayouts: {
            ...state.diagramLayout.constructLayouts,
            ...Object.fromEntries(nodes.map((node) => [node.id, {
              ...(state.diagramLayout.constructLayouts[node.id] ?? {}),
              x: node.position.x,
              y: node.position.y,
            }])),
          },
        }),
      };
    }
    const removedNodeIds = new Set(changes.filter((change) => change.type === "remove").map((change) => change.id));
    const cascadeIds = new Set([
      ...cascadingInteractionNodeIds(state.nodes, removedNodeIds, []),
      ...cascadingHigherOrderNodeIds(state.nodes, removedNodeIds),
    ]);
    const allRemovedNodeIds = new Set([...removedNodeIds, ...cascadeIds]);
    const nodes = applyNodeChanges(changes, state.nodes).filter((node) => !cascadeIds.has(node.id));
    const edges = state.edges.filter((edge) => !allRemovedNodeIds.has(edge.source) && !allRemovedNodeIds.has(edge.target));
    const layout = syncedDiagramLayout(nodes, edges, {
      ...state.diagramLayout,
      constructLayouts: {
        ...state.diagramLayout.constructLayouts,
        ...Object.fromEntries(nodes.map((node) => [node.id, { ...(state.diagramLayout.constructLayouts[node.id] ?? {}), x: node.position.x, y: node.position.y }])),
      },
    });
    return {
      ...(changes.some((change) => change.type === "remove") ? historyPatch(state) : {}),
      nodes,
      edges,
      diagramLayout: layout,
    };
  }),
  onEdgesChange: (changes) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) {
      const presentationChanges = changes.filter((change) => change.type === "select");
      return presentationChanges.length === 0
        ? state
        : { edges: applyEdgeChanges(presentationChanges, state.edges) };
    }
    const removedIds = new Set(changes.filter((change) => change.type === "remove").map((change) => change.id));
    const removedEdges = state.edges.filter((edge) => removedIds.has(edge.id));
    const cascadeIds = cascadingInteractionNodeIds(state.nodes, new Set(), removedEdges);
    const nodes = state.nodes.filter((node) => !cascadeIds.has(node.id));
    const edges = applyEdgeChanges(changes, state.edges).filter((edge) => !cascadeIds.has(edge.source) && !cascadeIds.has(edge.target));
    return {
      ...(removedIds.size ? historyPatch(state) : {}),
      nodes,
      edges,
      diagramLayout: syncedDiagramLayout(nodes, edges, state.diagramLayout),
    };
  }),
  onConnect: (connection) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    if (!connection.source || !connection.target || connection.source === connection.target) return state;
    if (touchesGeneratedInteraction(state.nodes, connection.source, connection.target)) return state;
    if (state.edges.some((edge) => edge.source === connection.source && edge.target === connection.target)) return state;
    const id = `path-${connection.source}-${connection.target}-${Date.now()}`;
    return {
      ...historyPatch(state),
      selectedNodeId: null,
      selectedEdgeId: id,
      edges: addEdge({
        ...connection,
        id,
        type: "straight",
        label: "Path",
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
      }, state.edges),
    };
  }),
  reconnectPath: (edge, connection) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    if (!connection.source || !connection.target || connection.source === connection.target) return state;
    if (requiredInteractionEdge(state.nodes, edge) || touchesGeneratedInteraction(state.nodes, connection.source, connection.target)) return state;
    if (state.edges.some((candidate) => candidate.id !== edge.id && candidate.source === connection.source && candidate.target === connection.target)) return state;
    return {
      ...historyPatch(state),
      edges: reconnectEdge(edge, connection, state.edges, { shouldReplaceId: false }),
      selectedNodeId: null,
      selectedEdgeId: edge.id,
    };
  }),
  addPath: (source, target) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    if (source === target || touchesGeneratedInteraction(state.nodes, source, target) || state.edges.some((edge) => edge.source === source && edge.target === target)) return state;
    const id = `path-${source}-${target}-${Date.now()}`;
    return {
      ...historyPatch(state),
      selectedNodeId: null,
      selectedEdgeId: id,
      edges: addEdge({
        id,
        source,
        target,
        type: "straight",
        label: "Path",
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
      }, state.edges),
    };
  }),
  addCovariance: (source, target) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    if (source === target) return state;
    if (touchesGeneratedInteraction(state.nodes, source, target)) return state;
    const [left, right] = [source, target].sort();
    if (nativeCovariancePairExistsV4(state.edges, left, right)) return state;
    const id = `covariance-${left}-${right}-${Date.now()}`;
    return {
      ...historyPatch(state),
      selectedNodeId: null,
      selectedEdgeId: id,
      edges: [...state.edges, newNativeScientificCovarianceEdgeV4(id, left, right)],
    };
  }),
  addTwoStageInteraction: (predictor, moderator, outcome) => {
    let result: AddTwoStageInteractionResult = { status: "blocked", reason: "construct_missing" };
    set((state) => {
    if (activeStandardSemModelV4Authority(state)) {
      result = { status: "blocked", reason: "unsupported_construct" };
      return state;
    }
    if (new Set([predictor, moderator, outcome]).size !== 3) {
      result = { status: "blocked", reason: "constructs_not_distinct" };
      return state;
    }
    const predictorNode = state.nodes.find((node) => node.id === predictor);
    const moderatorNode = state.nodes.find((node) => node.id === moderator);
    const outcomeNode = state.nodes.find((node) => node.id === outcome);
    if (!predictorNode || !moderatorNode || !outcomeNode) {
      result = { status: "blocked", reason: "construct_missing" };
      return state;
    }
    if ([predictorNode, moderatorNode, outcomeNode].some((node) => node.data.semantic || node.data.indicators.length === 0)) {
      result = { status: "blocked", reason: "unsupported_construct" };
      return state;
    }
    if (state.edges.some((edge) => (edge.data as { role?: string } | undefined)?.role === "control")) {
      result = { status: "blocked", reason: "control_paths_unsupported" };
      return state;
    }
    const focalEdge = state.edges.find((edge) =>
      !edge.id.startsWith("measurement::")
      && edge.source === predictor
      && edge.target === outcome
      && edge.data?.role !== "control"
      && edge.data?.role !== "covariance",
    );
    if (!focalEdge) {
      result = { status: "blocked", reason: "focal_path_missing" };
      return state;
    }
    if (interactionNodes(state.nodes).some((node) => {
      const interaction = node.data.interaction!;
      const operands = diagramInteractionOperands(interaction);
      return operands.length === 2
        && operands[0] === predictor
        && operands[1] === moderator
        && interaction.outcome === outcome;
    })) {
      result = { status: "blocked", reason: "duplicate_interaction" };
      return state;
    }
    const occupiedIds = new Set([
      ...state.nodes.map((node) => node.id),
      ...state.edges.map((edge) => edge.id),
    ]);
    const id = uniqueStableGraphId(`interaction-${predictor}-${moderator}-${outcome}`, occupiedIds);
    occupiedIds.add(id);
    const shortName = `${predictorNode.data.shortName}x${moderatorNode.data.shortName}`.replace(/[^a-zA-Z0-9]/g, "").slice(0, 8) || "INT";
    const hasModeratorMainEffect = state.edges.some((edge) =>
      !edge.id.startsWith("measurement::")
      && edge.source === moderator
      && edge.target === outcome
      && edge.data?.role !== "covariance",
    );
    const moderatorEdgeId = uniqueStableGraphId(`path-${moderator}-${outcome}`, occupiedIds);
    const withModeratorMainEffect = hasModeratorMainEffect
      ? state.edges
      : addEdge({
        id: moderatorEdgeId,
        source: moderator,
        target: outcome,
        type: "straight",
        label: "Path",
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
      }, state.edges);
    for (const edge of withModeratorMainEffect) occupiedIds.add(edge.id);
    const interactionEdgeId = uniqueStableGraphId(`path-${id}-${outcome}`, occupiedIds);
    const interaction: NonNullable<ConstructData["interaction"]> = state.generalSemProjectDraftMode?.semGeneration === "general_sem_v1"
      ? {
          kind: "interaction_v2",
          termId: `interaction-term:${id}`,
          operands: [predictor, moderator],
          outcome,
          focalRelationId: focalEdge.id,
          canonicalMethod: "two_stage",
          hierarchyPolicy: "strong",
          productIndicator: null,
        }
      : { predictor, moderator, outcome, method: "two_stage_product_score" };
    result = { status: "created", interactionId: id };
    return {
      ...historyPatch(state),
      selectedNodeId: id,
      selectedEdgeId: null,
      nodes: [...state.nodes, {
        id,
        type: "construct",
        position: {
          x: Math.max(predictorNode.position.x, moderatorNode.position.x) + 220,
          y: (predictorNode.position.y + moderatorNode.position.y + outcomeNode.position.y) / 3,
        },
        data: {
          label: `${predictorNode.data.shortName} x ${moderatorNode.data.shortName}`,
          shortName,
          mode: "formative",
          indicators: [],
          semantic: "interaction",
          interaction,
        },
      }],
      edges: withModeratorMainEffect.some((edge) => edge.source === id && edge.target === outcome)
        ? withModeratorMainEffect
        : addEdge({
          id: interactionEdgeId,
          source: id,
          target: outcome,
          type: "straight",
          label: "Interaction",
          markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        }, withModeratorMainEffect),
    };
    });
    return result;
  },
  addHigherOrderConstruct: (draft) => {
    let result: AddHigherOrderConstructResult = { status: "blocked", reason: "invalid_draft", detail: "The higher-order construct could not be created." };
    set((state) => {
      if (activeStandardSemModelV4Authority(state)) {
        result = {
          status: "blocked",
          reason: "scope_unavailable",
          detail: "Strict Standard models must be edited through a versioned SemModelV4 authority intent.",
        };
        return state;
      }
      const scopeBlocker = nativeHigherOrderCreationBlocker(state.nodes, state.edges);
      if (scopeBlocker) {
        result = { status: "blocked", reason: "scope_unavailable", detail: scopeBlocker };
        return state;
      }
      const problems = nativeHigherOrderDraftProblems(draft, state.nodes, state.edges);
      if (problems.length) {
        result = { status: "blocked", reason: "invalid_draft", detail: problems[0] };
        return state;
      }
      const base = `hoc-${draft.shortName.trim().normalize("NFKC").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "construct"}`;
      let id = base;
      let suffix = 2;
      while (state.nodes.some((node) => node.id === id)) id = `${base}-${suffix++}`;
      result = { status: "created", constructId: id };
      return {
        ...historyPatch(state),
        selectedNodeId: id,
        selectedEdgeId: null,
        nodes: [
          ...state.nodes.map((node) => ({ ...node, selected: false })),
          {
            id,
            type: "construct",
            position: nextConstructPosition(state.nodes),
            selected: true,
            data: {
              label: draft.name.trim(),
              shortName: draft.shortName.trim(),
              mode: "reflective",
              indicators: [],
              semantic: "higher_order",
              higherOrder: {
                id,
                components: [...draft.components],
                method: "two_stage",
                stage_one_recipe: null,
              },
            },
          },
        ],
      };
    });
    return result;
  },
  updateConstruct: (id, patch) => set((state) => activeStandardSemModelV4Authority(state) ? state : ({
    ...historyPatch(state),
    nodes: state.nodes.map((node) => {
      if (node.id !== id) return node;
      const data = { ...node.data, ...patch };
      return node.data.semantic === "interaction"
        ? { ...node, data: { ...data, mode: "formative", indicators: [], semantic: "interaction", interaction: node.data.interaction } }
        : node.data.semantic === "higher_order"
          ? { ...node, data: { ...data, mode: "reflective", indicators: [], semantic: "higher_order", higherOrder: node.data.higherOrder } }
        : { ...node, data };
    }),
  })),
  setConstructEstimandV4: (id, specification) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const node = state.nodes.find((candidate) => candidate.id === id);
    if (!node || node.data.semantic === "interaction" || node.data.semantic === "higher_order") return state;
    return {
      ...historyPatch(state),
      nodes: state.nodes.map((candidate) => candidate.id === id ? withNativeConstructEstimandV4(candidate, specification) : candidate),
    };
  }),
  updateEdge: (id, patch) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const selected = state.edges.find((edge) => edge.id === id);
    if (!selected) return state;
    const candidateData = patch.data === undefined ? selected.data : patch.data;
    const candidateRole = candidateData && typeof candidateData === "object" && !Array.isArray(candidateData)
      ? (candidateData as { role?: unknown }).role
      : undefined;
    const candidateSource = patch.source ?? selected.source;
    const candidateTarget = patch.target ?? selected.target;
    if (candidateRole === "covariance" && nativeCovariancePairExistsV4(state.edges, candidateSource, candidateTarget, id)) return state;
    return {
      ...historyPatch(state),
      edges: state.edges.map((edge) => {
        if (edge.id !== id) return edge;
        if (!requiredInteractionEdge(state.nodes, edge)) return { ...edge, ...patch };
        const currentRole = edge.data?.role;
        const data = { ...(edge.data ?? {}), ...(patch.data ?? {}) };
        if (currentRole) data.role = currentRole;
        else delete data.role;
        return { ...edge, ...patch, source: edge.source, target: edge.target, data };
      }),
    };
  }),
  convertCovarianceToScientificV4: (id, endpoints = { left: null, right: null }) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const edge = state.edges.find((candidate) => candidate.id === id);
    if (!edge || edge.data?.role !== "covariance") return state;
    return {
      ...historyPatch(state),
      edges: state.edges.map((candidate) => candidate.id === id
        ? convertNativeCovarianceToScientificV4(candidate, endpoints)
        : candidate),
    };
  }),
  convertCovarianceToPresentationV4: (id) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const edge = state.edges.find((candidate) => candidate.id === id);
    if (!edge || edge.data?.role !== "covariance") return state;
    return {
      ...historyPatch(state),
      edges: state.edges.map((candidate) => candidate.id === id
        ? convertNativeCovarianceToPresentationV4(candidate)
        : candidate),
    };
  }),
  setEdgeLabelOffset: (id, offset) => set((state) => ({
    diagramLayout: {
      ...state.diagramLayout,
      edgeLayouts: {
        ...state.diagramLayout.edgeLayouts,
        [id]: {
          ...(state.diagramLayout.edgeLayouts[id] ?? { routing: "straight" }),
          labelOffset: offset,
          pinned: true,
        },
      },
    },
  })),
  nudgeEdgeLabel: (id, delta) => set((state) => {
    const current = state.diagramLayout.edgeLayouts[id]?.labelOffset ?? { x: 0, y: 0 };
    return {
      ...historyPatch(state),
      diagramLayout: {
        ...state.diagramLayout,
        edgeLayouts: {
          ...state.diagramLayout.edgeLayouts,
          [id]: {
            ...(state.diagramLayout.edgeLayouts[id] ?? { routing: "straight" }),
            labelOffset: { x: current.x + delta.x, y: current.y + delta.y },
            pinned: true,
          },
        },
      },
    };
  }),
  resetEdgeLabel: (id) => set((state) => ({
    ...historyPatch(state),
    diagramLayout: {
      ...state.diagramLayout,
      edgeLayouts: {
        ...state.diagramLayout.edgeLayouts,
        [id]: { ...(state.diagramLayout.edgeLayouts[id] ?? { routing: "straight" }), labelOffset: undefined, pinned: false },
      },
    },
  })),
  resetAllEdgeLabels: () => set((state) => ({
    ...historyPatch(state),
    diagramLayout: {
      ...state.diagramLayout,
      edgeLayouts: Object.fromEntries(Object.entries(state.diagramLayout.edgeLayouts).map(([id, layout]) => [
        id,
        { ...layout, labelOffset: undefined, pinned: false },
      ])),
    },
  })),
  addConstruct: (position, indicators = []) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const id = `construct-${Date.now()}`;
    const name = nextConstructName(state.nodes);
    const fallback = nextConstructPosition(state.nodes);
    const validIndicators = validUniqueIndicators(indicators, state.dataset, state.analysisSettings.groupColumn);
    const nextPosition = position ? nearestOpenConstructPosition(position, state.nodes) : fallback;
    return {
      ...historyPatch(state),
      selectedNodeId: id,
      selectedEdgeId: null,
      nodes: [...state.nodes.map((node) => ({
        ...node,
        data: { ...node.data, indicators: node.data.indicators.filter((indicator) => !validIndicators.includes(indicator)) },
      })), {
        id,
        type: "construct",
        position: nextPosition,
        data: { ...name, mode: "reflective", indicators: validIndicators },
      }],
    };
  }),
  addConstructsFromIndicators: (indicators) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const validIndicators = validUniqueIndicators(indicators, state.dataset, state.analysisSettings.groupColumn);
    if (validIndicators.length === 0) return state;
    let nextNodes = state.nodes.map((node) => ({
      ...node,
      data: { ...node.data, indicators: node.data.indicators.filter((indicator) => !validIndicators.includes(indicator)) },
    }));
    const createdIds: string[] = [];
    for (const indicator of validIndicators) {
      const id = constructIdFromIndicator(indicator, nextNodes);
      const shortName = indicator.replace(/[^a-zA-Z0-9]/g, "").slice(0, 8) || "ITEM";
      createdIds.push(id);
      nextNodes = [...nextNodes, {
        id,
        type: "construct",
        position: nextConstructPosition(nextNodes),
        selected: true,
        data: {
          label: indicator,
          shortName,
          mode: "reflective",
          indicators: [indicator],
        },
      }];
    }
    return {
      ...historyPatch(state),
      selectedNodeId: createdIds.at(-1) ?? null,
      selectedEdgeId: null,
      nodes: nextNodes.map((node) => createdIds.includes(node.id) ? node : { ...node, selected: false }),
    };
  }),
  addConstructsFromIndicatorGroups: (indicators) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const validIndicators = validUniqueIndicators(indicators, state.dataset, state.analysisSettings.groupColumn);
    if (validIndicators.length === 0) return state;
    const groups = new Map<string, string[]>();
    for (const indicator of validIndicators) {
      const key = indicatorGroupKey(indicator);
      groups.set(key, [...(groups.get(key) ?? []), indicator]);
    }
    let nextNodes = state.nodes.map((node) => ({
      ...node,
      data: { ...node.data, indicators: node.data.indicators.filter((indicator) => !validIndicators.includes(indicator)) },
    }));
    const createdIds: string[] = [];
    for (const [key, groupIndicators] of groups) {
      const id = constructIdFromIndicator(key, nextNodes);
      createdIds.push(id);
      nextNodes = [...nextNodes, {
        id,
        type: "construct",
        position: nextConstructPosition(nextNodes),
        selected: true,
        data: {
          label: key,
          shortName: key,
          mode: "reflective",
          indicators: groupIndicators,
        },
      }];
    }
    return {
      ...historyPatch(state),
      selectedNodeId: createdIds.at(-1) ?? null,
      selectedEdgeId: null,
      nodes: nextNodes.map((node) => createdIds.includes(node.id) ? node : { ...node, selected: false }),
    };
  }),
  duplicateSelected: () => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const source = state.nodes.find((node) => node.id === state.selectedNodeId);
    if (!source) return state;
    const id = `construct-${Date.now()}`;
    return {
      ...historyPatch(state),
      selectedNodeId: id,
      nodes: [...state.nodes, {
        ...source,
        id,
        selected: false,
        position: { x: source.position.x + 35, y: source.position.y + 35 },
        data: { ...source.data, label: `${source.data.label} copy`, shortName: `${source.data.shortName}2`.slice(0, 8), indicators: [], semantic: undefined, interaction: undefined, higherOrder: undefined },
      }],
    };
  }),
  removeSelection: () => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const selectedNodeIds = new Set([
      ...state.nodes.filter((node) => node.selected).map((node) => node.id),
      ...(state.selectedNodeId ? [state.selectedNodeId] : []),
    ]);
    const edgeIds = new Set([
      ...state.edges.filter((edge) => edge.selected).map((edge) => edge.id),
      ...(state.selectedEdgeId ? [state.selectedEdgeId] : []),
    ]);
    if (selectedNodeIds.size === 0 && edgeIds.size === 0) return state;
    const removedEdges = state.edges.filter((edge) => edgeIds.has(edge.id));
    const cascadeIds = new Set([
      ...cascadingInteractionNodeIds(state.nodes, selectedNodeIds, removedEdges),
      ...cascadingHigherOrderNodeIds(state.nodes, selectedNodeIds),
    ]);
    const nodeIds = new Set([...selectedNodeIds, ...cascadeIds]);
    return {
      ...historyPatch(state),
      nodes: state.nodes.filter((node) => !nodeIds.has(node.id)),
      edges: state.edges.filter((edge) => !edgeIds.has(edge.id) && !nodeIds.has(edge.source) && !nodeIds.has(edge.target)),
      selectedNodeId: null,
      selectedEdgeId: null,
    };
  }),
  reverseSelectedPath: () => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const selected = state.edges.find((edge) => edge.id === state.selectedEdgeId);
    if (!selected || requiredInteractionEdge(state.nodes, selected) || state.edges.some((edge) => edge.id !== selected.id && edge.source === selected.target && edge.target === selected.source)) return state;
    return {
      ...historyPatch(state),
      edges: state.edges.map((edge) => edge.id === selected.id ? {
        ...edge,
        source: selected.target,
        target: selected.source,
        sourceHandle: null,
        targetHandle: null,
      } : edge),
    };
  }),
  setSelectedPathRouting: (routing) => set((state) => {
    const selected = state.edges.find((edge) => edge.id === state.selectedEdgeId);
    if (!selected) return state;
    return setPathRoutingState(state, selected.id, routing);
  }),
  setPathRouting: (id, routing) => set((state) => setPathRoutingState(state, id, routing)),
  alignSelectedConstructs: (target) => set((state) => {
    const selected = selectedConstructs(state);
    if (selected.length < 2) return state;
    const xValues = selected.map((node) => node.position.x);
    const yValues = selected.map((node) => node.position.y);
    const centerXValues = selected.map((node) => node.position.x + constructSize.width / 2);
    const centerYValues = selected.map((node) => node.position.y + constructSize.height / 2);
    const rightValues = selected.map((node) => node.position.x + constructSize.width);
    const bottomValues = selected.map((node) => node.position.y + constructSize.height);
    const targetValue = target === "left" ? Math.min(...xValues)
      : target === "right" ? Math.max(...rightValues)
        : target === "centerX" ? centerXValues.reduce((sum, value) => sum + value, 0) / centerXValues.length
          : target === "top" ? Math.min(...yValues)
            : target === "bottom" ? Math.max(...bottomValues)
              : centerYValues.reduce((sum, value) => sum + value, 0) / centerYValues.length;
    const selectedIds = new Set(selected.map((node) => node.id));
    return {
      ...historyPatch(state),
      nodes: state.nodes.map((node) => {
        if (!selectedIds.has(node.id)) return node;
        const position = { ...node.position };
        if (target === "left") position.x = targetValue;
        else if (target === "right") position.x = targetValue - constructSize.width;
        else if (target === "centerX") position.x = targetValue - constructSize.width / 2;
        else if (target === "top") position.y = targetValue;
        else if (target === "bottom") position.y = targetValue - constructSize.height;
        else position.y = targetValue - constructSize.height / 2;
        return { ...node, position };
      }),
    };
  }),
  distributeSelectedConstructs: (axis) => set((state) => {
    const selected = selectedConstructs(state);
    if (selected.length < 3) return state;
    const sorted = [...selected].sort((left, right) => axis === "horizontal" ? left.position.x - right.position.x : left.position.y - right.position.y);
    const centers = sorted.map((node) => axis === "horizontal" ? node.position.x + constructSize.width / 2 : node.position.y + constructSize.height / 2);
    const first = centers[0];
    const last = centers.at(-1)!;
    const spacing = (last - first) / (sorted.length - 1);
    const targetCenters = new Map(sorted.map((node, index) => [node.id, first + spacing * index]));
    return {
      ...historyPatch(state),
      nodes: state.nodes.map((node) => {
        const center = targetCenters.get(node.id);
        if (center === undefined) return node;
        return {
          ...node,
          position: axis === "horizontal"
            ? { ...node.position, x: center - constructSize.width / 2 }
            : { ...node.position, y: center - constructSize.height / 2 },
        };
      }),
    };
  }),
  autoLayout: (direction = "horizontal") => set((state) => {
    const nodes = direction === "smartpls" ? layoutSmartplsModel(state.nodes, state.edges) : layoutModel(state.nodes, state.edges, direction);
    const diagramLayout = syncedDiagramLayout(nodes, state.edges, state.diagramLayout);
    for (const node of nodes) {
      diagramLayout.constructLayouts[node.id] = {
        ...(diagramLayout.constructLayouts[node.id] ?? {}),
        x: node.position.x,
        y: node.position.y,
        pinned: false,
      };
    }
    return { ...historyPatch(state), nodes, diagramLayout };
  }),
  moveIndicator: (constructId, indicator, position) => set((state) => {
    const construct = state.nodes.find((node) => node.id === constructId);
    if (!construct?.data.indicators.includes(indicator)) return state;
    const constructIndicators = state.diagramLayout.indicatorLayouts[constructId] ?? {};
    const current = constructIndicators[indicator] ?? { side: "free" as const, order: construct.data.indicators.indexOf(indicator) };
    return {
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
        ...state.diagramLayout,
        indicatorLayouts: {
          ...state.diagramLayout.indicatorLayouts,
          [constructId]: {
            ...constructIndicators,
            [indicator]: { ...current, side: "free", x: position.x, y: position.y, pinned: true },
          },
        },
      }),
    };
  }),
  setIndicatorSide: (constructId, indicator, side) => set((state) => {
    const construct = state.nodes.find((node) => node.id === constructId);
    if (!construct?.data.indicators.includes(indicator)) return state;
    const constructIndicators = state.diagramLayout.indicatorLayouts[constructId] ?? {};
    const current = constructIndicators[indicator] ?? { side, order: construct.data.indicators.indexOf(indicator) };
    return {
      ...historyPatch(state),
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
        ...state.diagramLayout,
        indicatorLayouts: {
          ...state.diagramLayout.indicatorLayouts,
          [constructId]: {
            ...constructIndicators,
            [indicator]: { ...current, side, x: undefined, y: undefined, pinned: true },
          },
        },
      }),
    };
  }),
  setConstructIndicatorSide: (constructId, side) => set((state) => {
    const construct = state.nodes.find((node) => node.id === constructId);
    if (!construct || construct.data.indicators.length === 0) return state;
    const constructIndicators = state.diagramLayout.indicatorLayouts[constructId] ?? {};
    const nextIndicators = Object.fromEntries(construct.data.indicators.map((indicator, index) => {
      const current = constructIndicators[indicator] ?? { order: index };
      return [indicator, { ...current, side, x: undefined, y: undefined, order: current.order ?? index, pinned: true }];
    }));
    return {
      ...historyPatch(state),
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
        ...state.diagramLayout,
        indicatorLayouts: {
          ...state.diagramLayout.indicatorLayouts,
          [constructId]: nextIndicators,
        },
      }),
    };
  }),
  toggleConstructPinned: (constructId) => set((state) => {
    const construct = state.nodes.find((node) => node.id === constructId);
    if (!construct) return state;
    const current = state.diagramLayout.constructLayouts[constructId] ?? { x: construct.position.x, y: construct.position.y };
    return {
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
        ...state.diagramLayout,
        constructLayouts: {
          ...state.diagramLayout.constructLayouts,
          [constructId]: { ...current, x: construct.position.x, y: construct.position.y, pinned: !current.pinned },
        },
      }),
    };
  }),
  resetIndicatorLayout: (constructId, indicator) => set((state) => {
    const constructIndicators = { ...(state.diagramLayout.indicatorLayouts[constructId] ?? {}) };
    if (indicator) delete constructIndicators[indicator];
    else Object.keys(constructIndicators).forEach((key) => delete constructIndicators[key]);
    return {
      ...historyPatch(state),
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
        ...state.diagramLayout,
        indicatorLayouts: { ...state.diagramLayout.indicatorLayouts, [constructId]: constructIndicators },
      }),
    };
  }),
  assignIndicator: (constructId, indicator) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const target = state.nodes.find((node) => node.id === constructId);
    if (
      !target
      || target.data.semantic === "interaction"
      || target.data.semantic === "higher_order"
      || target.data.indicators.includes(indicator)
      || indicator === state.analysisSettings.groupColumn?.trim()
    ) return state;
    const indicatorLayout = Object.fromEntries(Object.entries(state.diagramLayout.indicatorLayouts).map(([nodeId, indicators]) => {
      const next = { ...indicators };
      if (nodeId !== constructId) delete next[indicator];
      return [nodeId, next];
    }));
    return {
      ...historyPatch(state),
      nodes: state.nodes.map((node) => ({
        ...node,
        data: {
          ...node.data,
          indicators: node.id === constructId
            ? [...node.data.indicators, indicator]
            : node.data.indicators.filter((item) => item !== indicator),
        },
      })),
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, { ...state.diagramLayout, indicatorLayouts: indicatorLayout }),
    };
  }),
  assignIndicators: (constructId, indicators) => set((state) => {
    if (activeStandardSemModelV4Authority(state)) return state;
    const target = state.nodes.find((node) => node.id === constructId);
    const unique = validUniqueIndicators(indicators, state.dataset, state.analysisSettings.groupColumn);
    if (!target || target.data.semantic === "interaction" || target.data.semantic === "higher_order" || unique.length === 0) return state;
    const indicatorLayout = Object.fromEntries(Object.entries(state.diagramLayout.indicatorLayouts).map(([nodeId, current]) => {
      const next = { ...current };
      if (nodeId !== constructId) unique.forEach((indicator) => delete next[indicator]);
      return [nodeId, next];
    }));
    return {
      ...historyPatch(state),
      nodes: state.nodes.map((node) => ({
        ...node,
        data: {
          ...node.data,
          indicators: node.id === constructId
            ? [...node.data.indicators.filter((item) => !unique.includes(item)), ...unique]
            : node.data.indicators.filter((item) => !unique.includes(item)),
        },
      })),
      diagramLayout: syncedDiagramLayout(state.nodes, state.edges, { ...state.diagramLayout, indicatorLayouts: indicatorLayout }),
    };
  }),
  unassignIndicator: (constructId, indicator) => set((state) => activeStandardSemModelV4Authority(state) ? state : ({
    ...historyPatch(state),
    nodes: state.nodes.map((node) => node.id === constructId ? {
      ...node,
      data: { ...node.data, indicators: node.data.indicators.filter((item) => item !== indicator) },
    } : node),
    diagramLayout: syncedDiagramLayout(state.nodes, state.edges, {
      ...state.diagramLayout,
      indicatorLayouts: {
        ...state.diagramLayout.indicatorLayouts,
        [constructId]: Object.fromEntries(Object.entries(state.diagramLayout.indicatorLayouts[constructId] ?? {}).filter(([key]) => key !== indicator)),
      },
    }),
  })),
  setDataset: (dataset) => set((state) => activeStandardSemModelV4Authority(state) || state.generalSemPublicationPending ? state : ({
    dataset,
    datasetCatalog: upsertDatasetCatalog(state.datasetCatalog, dataset),
    datasetDescriptorOnly: false,
    view: "data",
    workflowDestinationContext: null,
    workflowCommandContext: null,
  })),
  setDatasetCatalog: (datasetCatalog, datasetVersions) => set((state) => activeStandardSemModelV4Authority(state) || state.generalSemPublicationPending
    ? state
    : { datasetCatalog, datasetVersions, datasetDescriptorOnly: false }),
  commitDatasetVersion: ({ dataset, version }) => set((state) => activeStandardSemModelV4Authority(state) || state.generalSemPublicationPending ? state : ({
    dataset,
    datasetCatalog: upsertDatasetCatalog(state.datasetCatalog, dataset),
    datasetDescriptorOnly: false,
    datasetVersions: [
      ...state.datasetVersions.filter((candidate) => candidate.datasetId !== version.datasetId),
      version,
    ],
    view: "data",
    workflowDestinationContext: null,
    workflowCommandContext: null,
  })),
  setExplorerSelection: (explorerSelection) => set({ explorerSelection }),
  setProjectExplorer: ({ projectModels, activeModelId, modelPresentations, savedReports, explorerSelection }) => set((state) => {
    const projectModelIds = new Set(projectModels.map((model) => model.id));
    const nextActiveModelId = activeModelId && projectModelIds.has(activeModelId) ? activeModelId : null;
    const standardSemModelV4Authorities = Object.fromEntries(
      Object.entries(state.standardSemModelV4Authorities).filter(([modelId]) => projectModelIds.has(modelId)),
    );
    const standardSemModelV4ScientificEditLocks = Object.fromEntries(
      Object.entries(state.standardSemModelV4ScientificEditLocks).filter(([modelId]) => projectModelIds.has(modelId)),
    );
    const strictModelIds = new Set(Object.keys(standardSemModelV4Authorities));
    const legacyModelPresentations = Object.fromEntries(
      Object.entries(modelPresentations).filter(([modelId]) => !strictModelIds.has(modelId)),
    );
    const activeChanged = state.activeModelId !== nextActiveModelId;
    const outgoingAuthority = activeStandardSemModelV4Authority(state);
    const targetAuthority = nextActiveModelId ? standardSemModelV4Authorities[nextActiveModelId] : undefined;
    const retainedLayouts = Object.fromEntries(
      Object.entries(state.standardSemModelV4Layouts).filter(([modelId]) => projectModelIds.has(modelId)),
    );
    const standardSemModelV4Layouts = activeChanged
      && state.activeModelId
      && outgoingAuthority
      && projectModelIds.has(state.activeModelId)
      ? {
          ...retainedLayouts,
          [state.activeModelId]: standardSemModelV4Layout(state.activeModelId, state.diagramLayout),
        }
      : retainedLayouts;
    const standardSemModelV4Epochs = Object.fromEntries(
      Object.entries(state.standardSemModelV4Epochs).filter(([modelId]) => projectModelIds.has(modelId)),
    );
    const standardSemModelV4Persistence = Object.fromEntries(
      Object.entries(state.standardSemModelV4Persistence).filter(([modelId]) => projectModelIds.has(modelId)),
    );
    if (activeChanged) {
      if (state.activeModelId && standardSemModelV4Authorities[state.activeModelId]) {
        standardSemModelV4Epochs[state.activeModelId] = nextStandardSemModelV4Epoch();
      }
      if (nextActiveModelId && standardSemModelV4Authorities[nextActiveModelId]) {
        standardSemModelV4Epochs[nextActiveModelId] = nextStandardSemModelV4Epoch();
      }
    }
    const targetModel = nextActiveModelId ? projectModels.find((model) => model.id === nextActiveModelId) : undefined;
    const target = activeChanged && (outgoingAuthority || targetAuthority)
      ? targetAuthority
        ? projectStandardSemModelV4DiagramV1(targetAuthority, standardSemModelV4Layouts[nextActiveModelId!])
        : targetModel
          ? nativeModelSnapshotFromCanonical(targetModel, legacyModelPresentations[nextActiveModelId!])
          : { nodes: [], edges: [], diagramLayout: syncedDiagramLayout([], []) }
      : null;
    return {
      projectModels,
      activeModelId: nextActiveModelId,
      modelPresentations: legacyModelPresentations,
      savedReports,
      standardSemModelV4Authorities,
      standardSemModelV4ScientificEditLocks,
      standardSemModelV4Layouts,
      standardSemModelV4Epochs,
      standardSemModelV4Persistence,
      explorerSelection: explorerSelection ?? state.explorerSelection,
      ...(target
        ? {
            nodes: target.nodes,
            edges: target.edges,
            diagramLayout: syncedDiagramLayout(target.nodes, target.edges, target.diagramLayout),
            selectedNodeId: target.nodes[0]?.id ?? null,
            selectedEdgeId: null,
            past: [],
            future: [],
          }
        : {}),
    };
  }),
  installStandardSemModelV4Authority: (authority, layout) => {
    const parsedAuthority = parseStandardSemModelV4AuthorityRecordV1(authority);
    const modelId = parsedAuthority.model.id;
    const parsedLayout = layout ? parseStandardSemModelV4DiagramLayoutV1(layout) : undefined;
    if (parsedLayout && parsedLayout.model_id !== modelId) return false;
    let installed = false;
    set((state) => {
      if (!state.projectModels.some((model) => model.id === modelId) && state.activeModelId !== modelId) return state;
      const projection = projectStandardSemModelV4DiagramV1(parsedAuthority, parsedLayout);
      const storedLayout = standardSemModelV4Layout(modelId, projection.diagramLayout);
      const { [modelId]: _legacyPresentation, ...modelPresentations } = state.modelPresentations;
      installed = true;
      return {
        standardSemModelV4Authorities: {
          ...state.standardSemModelV4Authorities,
          [modelId]: parsedAuthority,
        },
        standardSemModelV4Layouts: {
          ...state.standardSemModelV4Layouts,
          [modelId]: storedLayout,
        },
        standardSemModelV4Epochs: {
          ...state.standardSemModelV4Epochs,
          [modelId]: nextStandardSemModelV4Epoch(),
        },
        standardSemModelV4Persistence: {
          ...state.standardSemModelV4Persistence,
          [modelId]: {
            readiness: "authoring_only",
            scientificSha256: null,
            anchorModelDocumentSha256: parsedAuthority.model_document_sha256,
            anchorLayout: storedLayout,
          },
        },
        modelPresentations,
        ...(state.activeModelId === modelId
          ? {
              nodes: projection.nodes,
              edges: projection.edges,
              diagramLayout: projection.diagramLayout,
              selectedNodeId: projection.nodes[0]?.id ?? null,
              selectedEdgeId: null,
              past: [],
              future: [],
            }
          : {}),
      };
    });
    return installed;
  },
  activateStandardSemModelV4Authorities: (installations, activeModelId, projectName, datasetDescriptors, scientificEditLockedModelIds = []) => {
    if (!installations.length || !datasetDescriptors?.length) return false;
    try {
      const authorities: Record<string, StandardSemModelV4AuthorityRecordV1> = {};
      const layouts: Record<string, StandardSemModelV4DiagramLayoutV1> = {};
      const persistence: Record<string, StandardSemModelV4PersistenceV1> = {};
      const projections = new Map<string, ReturnType<typeof projectStandardSemModelV4DiagramV1>>();
      const descriptors = Object.fromEntries(datasetDescriptors.map((descriptor) => [descriptor.id, descriptor]));
      if (Object.keys(descriptors).length !== datasetDescriptors.length) return false;
      for (const installation of installations) {
        const authority = parseStandardSemModelV4AuthorityRecordV1(installation.authority);
        const modelId = authority.model.id;
        if (
          authorities[modelId]
          || !validStandardSemModelV4Readiness(installation.readiness, installation.scientificSha256)
          || !descriptors[authority.model.data_binding.dataset_id]
        ) return false;
        const suppliedLayout = installation.layout
          ? parseStandardSemModelV4DiagramLayoutV1(installation.layout)
          : undefined;
        if (suppliedLayout && suppliedLayout.model_id !== modelId) return false;
        const projection = projectStandardSemModelV4DiagramV1(authority, suppliedLayout);
        const layout = standardSemModelV4Layout(modelId, projection.diagramLayout);
        authorities[modelId] = authority;
        layouts[modelId] = layout;
        persistence[modelId] = {
          readiness: installation.readiness,
          scientificSha256: installation.scientificSha256,
          anchorModelDocumentSha256: authority.model_document_sha256,
          anchorLayout: layout,
        };
        projections.set(modelId, projection);
      }
      const active = projections.get(activeModelId);
      if (!active) return false;
      const activeDescriptor = descriptors[authorities[activeModelId].model.data_binding.dataset_id];
      if (!activeDescriptor) return false;
      const descriptorDatasets = datasetDescriptors.map(datasetFromStandardSemModelV4Descriptor);
      const epochs = Object.fromEntries(
        Object.keys(authorities).map((modelId) => [modelId, nextStandardSemModelV4Epoch()]),
      );
      const locked = new Set(scientificEditLockedModelIds);
      if (locked.size !== scientificEditLockedModelIds.length
        || [...locked].some((modelId) => !authorities[modelId])) return false;
      set({
        projectModels: [],
        activeModelId,
        modelPresentations: {},
        standardSemModelV4Authorities: authorities,
        standardSemModelV4ScientificEditLocks: Object.fromEntries([...locked].map((modelId) => [modelId, true])),
        standardSemModelV4Layouts: layouts,
        standardSemModelV4Epochs: epochs,
        standardSemModelV4Persistence: persistence,
        standardSemModelV4DatasetDescriptors: descriptors,
        datasetDescriptorOnly: true,
        generalSemProjectDraftMode: null,
        dataset: datasetFromStandardSemModelV4Descriptor(activeDescriptor),
        datasetCatalog: descriptorDatasets,
        datasetVersions: [],
        savedReports: [],
        runs: [],
        projectName,
        projectPath: null,
        projectWritable: false,
        explorerSelection: { kind: "model", modelId: activeModelId },
        nodes: active.nodes,
        edges: active.edges,
        diagramLayout: active.diagramLayout,
        selectedNodeId: active.nodes[0]?.id ?? null,
        selectedEdgeId: null,
        selectedResultRunId: null,
        diagramTool: "select",
        view: "models",
        past: [],
        future: [],
      });
      return true;
    } catch {
      return false;
    }
  },
  appendStandardSemModelV4Revision: (cas, installation) => {
    let appended = false;
    try {
      const authority = parseStandardSemModelV4AuthorityRecordV1(installation.authority);
      const revisionModelId = authority.model.id;
      set((state) => {
        const source = state.standardSemModelV4Authorities[cas.sourceModelId];
        const sourcePersistence = state.standardSemModelV4Persistence[cas.sourceModelId];
        const sourceLayout = currentStandardSemModelV4Layout(state, cas.sourceModelId);
        if (
          state.activeModelId !== cas.sourceModelId
          || !source
          || !sourcePersistence
          || !sourceLayout
          || source.model_document_sha256 !== cas.expectedSourceModelDocumentSha256
          || state.standardSemModelV4Epochs[cas.sourceModelId] !== cas.expectedSourceEpoch
          || sourcePersistence.anchorModelDocumentSha256 !== source.model_document_sha256
          || !sameStandardSemModelV4Layout(sourceLayout, sourcePersistence.anchorLayout)
          || [...standardSemModelV4Queues.keys()].some((key) => key.startsWith(`${cas.sourceModelId}\0`))
          || revisionModelId === cas.sourceModelId
          || state.standardSemModelV4Authorities[revisionModelId]
          || state.projectModels.some((model) => model.id === revisionModelId)
          || !validStandardSemModelV4Readiness(installation.readiness, installation.scientificSha256)
          || !state.standardSemModelV4DatasetDescriptors[authority.model.data_binding.dataset_id]
        ) return state;
        const suppliedLayout = installation.layout
          ? parseStandardSemModelV4DiagramLayoutV1(installation.layout)
          : undefined;
        if (suppliedLayout && suppliedLayout.model_id !== revisionModelId) return state;
        const projection = projectStandardSemModelV4DiagramV1(authority, suppliedLayout);
        const revisionLayout = standardSemModelV4Layout(revisionModelId, projection.diagramLayout);
        const targetDescriptor = state.standardSemModelV4DatasetDescriptors[authority.model.data_binding.dataset_id];
        appended = true;
        return {
          standardSemModelV4Authorities: {
            ...state.standardSemModelV4Authorities,
            [revisionModelId]: authority,
          },
          standardSemModelV4Layouts: {
            ...state.standardSemModelV4Layouts,
            [cas.sourceModelId]: sourceLayout,
            [revisionModelId]: revisionLayout,
          },
          standardSemModelV4Epochs: {
            ...state.standardSemModelV4Epochs,
            [revisionModelId]: nextStandardSemModelV4Epoch(),
          },
          standardSemModelV4Persistence: {
            ...state.standardSemModelV4Persistence,
            [revisionModelId]: {
              readiness: installation.readiness,
              scientificSha256: installation.scientificSha256,
              anchorModelDocumentSha256: authority.model_document_sha256,
              anchorLayout: revisionLayout,
            },
          },
          activeModelId: revisionModelId,
          explorerSelection: { kind: "model", modelId: revisionModelId },
          nodes: projection.nodes,
          edges: projection.edges,
          diagramLayout: projection.diagramLayout,
          dataset: datasetFromStandardSemModelV4Descriptor(targetDescriptor),
          selectedNodeId: projection.nodes[0]?.id ?? null,
          selectedEdgeId: null,
          diagramTool: "select",
          view: "models",
          past: [],
          future: [],
        };
      });
    } catch {
      return false;
    }
    return appended;
  },
  captureStandardSemModelV4SaveAuthorities: (modelIds) => {
    const state = get();
    if (!modelIds.length || new Set(modelIds).size !== modelIds.length) return null;
    const captured: Record<string, StandardSemModelV4SaveAuthorityV1> = {};
    for (const modelId of modelIds) {
      const authority = state.standardSemModelV4Authorities[modelId];
      const persistence = state.standardSemModelV4Persistence[modelId];
      const layout = currentStandardSemModelV4Layout(state, modelId);
      if (!authority || !persistence || !layout) return null;
      captured[modelId] = {
        authority,
        layout,
        readiness: persistence.readiness,
        scientificSha256: persistence.scientificSha256,
        dirty: authority.model_document_sha256 !== persistence.anchorModelDocumentSha256
          || !sameStandardSemModelV4Layout(layout, persistence.anchorLayout),
      };
    }
    return captured;
  },
  reanchorStandardSemModelV4Authorities: (captured) => {
    let reanchored = false;
    set((state) => {
      const modelIds = Object.keys(captured);
      if (
        !modelIds.length
        || modelIds.length !== Object.keys(state.standardSemModelV4Authorities).length
      ) return state;
      for (const modelId of modelIds) {
        const expected = captured[modelId];
        const authority = state.standardSemModelV4Authorities[modelId];
        const persistence = state.standardSemModelV4Persistence[modelId];
        const layout = currentStandardSemModelV4Layout(state, modelId);
        if (
          !authority
          || !persistence
          || !layout
          || authority.model_document_sha256 !== expected.authority.model_document_sha256
          || persistence.readiness !== expected.readiness
          || persistence.scientificSha256 !== expected.scientificSha256
          || !sameStandardSemModelV4Layout(layout, expected.layout)
        ) return state;
      }
      reanchored = true;
      return {
        standardSemModelV4Layouts: {
          ...state.standardSemModelV4Layouts,
          ...Object.fromEntries(modelIds.map((modelId) => [modelId, captured[modelId].layout])),
        },
        standardSemModelV4Persistence: {
          ...state.standardSemModelV4Persistence,
          ...Object.fromEntries(modelIds.map((modelId) => [modelId, {
            ...state.standardSemModelV4Persistence[modelId],
            anchorModelDocumentSha256: captured[modelId].authority.model_document_sha256,
            anchorLayout: captured[modelId].layout,
          }])),
        },
      };
    });
    return reanchored;
  },
  clearStandardSemModelV4Workspace: (modelIds) => {
    const state = get();
    const expected = new Set(modelIds);
    const installed = Object.keys(state.standardSemModelV4Authorities);
    if (
      !modelIds.length
      || expected.size !== modelIds.length
      || installed.length !== expected.size
      || installed.some((modelId) => !expected.has(modelId))
      || [...standardSemModelV4Queues.keys()].some((key) => modelIds.some((modelId) => key.startsWith(`${modelId}\0`)))
    ) return false;
    const captured = state.captureStandardSemModelV4SaveAuthorities(modelIds);
    if (!captured || Object.values(captured).some((authority) => authority.dirty)) return false;
    state.closeProject();
    return Object.keys(get().standardSemModelV4Authorities).length === 0;
  },
  commitStandardSemModelV4Intent: async (intent) => {
    const invoked = get();
    const modelId = invoked.activeModelId;
    const authority = activeStandardSemModelV4Authority(invoked);
    const epoch = modelId ? invoked.standardSemModelV4Epochs[modelId] : undefined;
    if (!modelId || !authority || epoch === undefined) return { status: "stale" };
    if (invoked.standardSemModelV4ScientificEditLocks[modelId]) {
      return {
        status: "blocked",
        diagnostic: {
          code: "schema6_standard_authority.scientific_revision_fork_required",
          message: `Model '${modelId}' is frozen by a RecipeV4 or canonical result authority.`,
          correctiveAction: "Use Edit active model as new revision, then make scientific edits on the new model identity.",
          authoringIssues: [],
          readinessIssues: [],
        },
      };
    }

    return queueStandardSemModelV4Commit(modelId, epoch, async () => {
      const sourceState = get();
      const source = sourceState.standardSemModelV4Authorities[modelId];
      if (
        sourceState.activeModelId !== modelId
        || sourceState.standardSemModelV4Epochs[modelId] !== epoch
        || !source
      ) return { status: "stale" };
      const sourceStillCurrent = () => {
        const current = get();
        return current.activeModelId === modelId
          && current.standardSemModelV4Epochs[modelId] === epoch
          && current.standardSemModelV4Authorities[modelId]?.model_document_sha256 === source.model_document_sha256;
      };

      let candidate;
      try {
        candidate = reduceStandardSemModelV4AuthorityV1(source, intent);
      } catch (error) {
        return { status: "rejected", error };
      }

      let outcome: StandardSemModelV4AuthorityCasOutcomeV1;
      try {
        outcome = await compareAndSwapStandardSemModelV4Authority(
          source.model,
          source.model_document_sha256,
          candidate.model,
        );
      } catch (error) {
        return sourceStillCurrent() ? { status: "rejected", error } : { status: "stale" };
      }
      if (!sourceStillCurrent()) return { status: "stale" };
      if (outcome.status === "blocked") return { status: "blocked", diagnostic: outcome.diagnostic };
      if (
        outcome.value.sourceModelDocumentSha256 !== source.model_document_sha256
        || outcome.value.canonicalCandidate.id !== modelId
      ) {
        return {
          status: "rejected",
          error: new Error("The Standard SemModelV4 CAS receipt does not match the active source authority."),
        };
      }

      let committedAuthority: StandardSemModelV4AuthorityRecordV1;
      try {
        committedAuthority = parseStandardSemModelV4AuthorityRecordV1({
          schema_version: 1,
          model_document_sha256: outcome.value.candidateModelDocumentSha256,
          model: outcome.value.canonicalCandidate,
        });
      } catch (error) {
        return { status: "rejected", error };
      }

      const beforeCommit = get();
      if (!sourceStillCurrent()) return { status: "stale" };
      const layout = standardSemModelV4Layout(modelId, beforeCommit.diagramLayout);
      const projection = projectStandardSemModelV4DiagramV1(committedAuthority, layout);
      let committed = false;
      set((state) => {
        if (
          state.activeModelId !== modelId
          || state.standardSemModelV4Epochs[modelId] !== epoch
          || state.standardSemModelV4Authorities[modelId]?.model_document_sha256 !== source.model_document_sha256
        ) return state;
        committed = true;
        return {
          ...historyPatch(state),
          standardSemModelV4Authorities: {
            ...state.standardSemModelV4Authorities,
            [modelId]: committedAuthority,
          },
          standardSemModelV4Layouts: {
            ...state.standardSemModelV4Layouts,
            [modelId]: standardSemModelV4Layout(modelId, projection.diagramLayout),
          },
          standardSemModelV4Persistence: {
            ...state.standardSemModelV4Persistence,
            [modelId]: {
              ...(state.standardSemModelV4Persistence[modelId] ?? {
                anchorModelDocumentSha256: source.model_document_sha256,
                anchorLayout: layout,
              }),
              readiness: outcome.value.readiness,
              scientificSha256: outcome.value.candidateScientificSha256,
            },
          },
          nodes: projection.nodes,
          edges: projection.edges,
          diagramLayout: projection.diagramLayout,
          selectedNodeId: null,
          selectedEdgeId: null,
        };
      });
      return committed
        ? { status: "committed", authority: committedAuthority }
        : { status: "stale" };
    });
  },
  standardSemModelV4OperationBlocker: (operation) => {
    const state = get();
    return state.activeModelId && activeStandardSemModelV4Authority(state)
      ? standardSemModelV4OperationMessage(state.activeModelId, operation)
      : null;
  },
  switchProjectModel: (modelId) => {
    let switched = false;
    set((state) => {
      const requestedModel = state.projectModels.find((model) => model.id === modelId);
      const requestedAuthority = state.standardSemModelV4Authorities[modelId];
      if (!requestedModel && !requestedAuthority) return state;
      switched = true;
      if (state.activeModelId === modelId) return state;

      const outgoingModelId = state.activeModelId;
      const outgoingAuthority = activeStandardSemModelV4Authority(state);
      const currentModelName = state.projectModels.find((model) => model.id === state.activeModelId)?.name
        ?? state.projectName;
      const projectModels = outgoingModelId && !outgoingAuthority
        ? state.projectModels.map((model) => model.id === outgoingModelId
          ? buildNativeRecipeModel(model.id, currentModelName, state.nodes, state.edges)
          : model)
        : state.projectModels;
      const modelPresentations = outgoingModelId && !outgoingAuthority
        ? {
            ...state.modelPresentations,
            [outgoingModelId]: currentNativeModelPresentation(state.nodes, state.edges, state.diagramLayout),
          }
        : state.modelPresentations;
      const standardSemModelV4Layouts = outgoingModelId && outgoingAuthority
        ? {
            ...state.standardSemModelV4Layouts,
            [outgoingModelId]: standardSemModelV4Layout(outgoingModelId, state.diagramLayout),
          }
        : state.standardSemModelV4Layouts;
      const standardSemModelV4Epochs = { ...state.standardSemModelV4Epochs };
      if (outgoingModelId && outgoingAuthority) standardSemModelV4Epochs[outgoingModelId] = nextStandardSemModelV4Epoch();

      const targetAuthority = requestedAuthority;
      const targetDatasetDescriptor = targetAuthority
        ? state.standardSemModelV4DatasetDescriptors[targetAuthority.model.data_binding.dataset_id]
        : undefined;
      if (targetAuthority && state.datasetDescriptorOnly && !targetDatasetDescriptor) return state;
      const target = targetAuthority
        ? projectStandardSemModelV4DiagramV1(targetAuthority, standardSemModelV4Layouts[modelId])
        : nativeModelSnapshotFromCanonical(
            requestedModel!,
            modelPresentations[modelId],
          );
      if (targetAuthority) standardSemModelV4Epochs[modelId] = nextStandardSemModelV4Epoch();
      return {
        projectModels,
        activeModelId: modelId,
        modelPresentations,
        standardSemModelV4Layouts,
        standardSemModelV4Epochs,
        explorerSelection: { kind: "model", modelId },
        nodes: target.nodes,
        edges: target.edges,
        diagramLayout: syncedDiagramLayout(target.nodes, target.edges, target.diagramLayout),
        selectedNodeId: target.nodes[0]?.id ?? null,
        selectedEdgeId: null,
        diagramTool: "select",
        past: [],
        future: [],
        ...(targetDatasetDescriptor
          ? { dataset: datasetFromStandardSemModelV4Descriptor(targetDatasetDescriptor), datasetDescriptorOnly: true }
          : {}),
      };
    });
    return switched;
  },
  addRun: (run) => set((state) => activeStandardSemModelV4Authority(state) ? state : ({
    runs: [run, ...state.runs],
    selectedResultRunId: run.result ? run.id : state.selectedResultRunId,
    diagramOverlaySettings: run.result ? { ...state.diagramOverlaySettings, selectedRunId: run.id, mode: state.diagramOverlaySettings.mode === "model" ? "paths_r2" : state.diagramOverlaySettings.mode } : state.diagramOverlaySettings,
    view: "runs",
  })),
  setAnalysisSettings: (patch) => set((state) => ({ analysisSettings: normalizeAnalysisSettings({ ...state.analysisSettings, ...patch }) })),
  setProjectMeta: (projectName, projectPath, projectId = null) => set((state) => ({
    projectName,
    projectPath,
    projectId,
    generalSemProjectDraftMode: state.generalSemProjectDraftMode
      && projectPath === null
      && projectId === state.generalSemProjectDraftMode.sourceProjectId
      ? state.generalSemProjectDraftMode
      : null,
  })),
  setProjectWritable: (projectWritable) => set({ projectWritable }),
  beginGeneralSemProjectDraftMode: (sourceProjectId) => {
    let activated = false;
    set((state) => {
      const noResidentData = state.datasetCatalog.length === 0
        || (state.datasetCatalog.length === 1
          && state.datasetCatalog[0].columns.length === 0
          && (state.datasetCatalog[0].rowCount ?? state.datasetCatalog[0].rows.length) === 0);
      const fresh = sourceProjectId.length > 0
        && state.projectId === sourceProjectId
        && state.projectPath === null
        && noResidentData
        && state.projectModels.length === 0
        && state.activeModelId === null
        && state.nodes.length === 0
        && state.edges.length === 0
        && state.runs.length === 0
        && Object.keys(state.standardSemModelV4Authorities).length === 0;
      if (!fresh) return state;
      activated = true;
      return {
        generalSemProjectDraftMode: {
          schemaVersion: 1,
          semGeneration: "general_sem_v1",
          sourceProjectId,
        },
      };
    });
    return activated;
  },
  clearGeneralSemProjectDraftMode: () => set({ generalSemProjectDraftMode: null }),
  setGeneralSemPublicationPending: (generalSemPublicationPending) => set({ generalSemPublicationPending }),
  setGeneralSemTransientWorkBlocker: (generalSemTransientWorkBlocker) => set({ generalSemTransientWorkBlocker }),
  closeProject: () => set({
    nodes: [],
    edges: [],
    selectedNodeId: null,
    selectedEdgeId: null,
    selectedResultRunId: null,
    explorerTab: "constructs",
    explorerCollapsed: false,
    inspectorCollapsed: false,
    resultWorkspaceState: defaultResultWorkspaceState,
    methodSetupState: defaultMethodSetupState,
    largeModelViewState: defaultLargeModelViewState,
    runMonitor: defaultRunMonitor,
    diagramMode: "sem",
    diagramTool: "select",
    diagramOverlaySettings: defaultDiagramOverlaySettings,
    publicationDiagramSettings: defaultPublicationDiagramSettings,
    diagramLayout: syncedDiagramLayout([], []),
    dataset: emptyDataset,
    datasetCatalog: [],
    datasetVersions: [],
    projectModels: [],
    activeModelId: null,
    modelPresentations: {},
    standardSemModelV4Authorities: {},
    standardSemModelV4ScientificEditLocks: {},
    standardSemModelV4Layouts: {},
    standardSemModelV4Epochs: {},
    standardSemModelV4Persistence: {},
    standardSemModelV4DatasetDescriptors: {},
    datasetDescriptorOnly: false,
    savedReports: [],
    explorerSelection: { kind: "data" },
    runs: [],
    analysisSettings: defaultAnalysisSettings,
    view: "welcome",
    workflowDestinationContext: null,
    workflowCommandContext: null,
    projectName: "No project open",
    projectId: null,
    projectPath: null,
    projectWritable: true,
    generalSemProjectDraftMode: null,
    generalSemPublicationPending: false,
    generalSemTransientWorkBlocker: null,
    past: [],
    future: [],
  }),
  resetProject: () => set({
    nodes: initialNodes,
    edges: initialEdges,
    selectedNodeId: "satisfaction",
    selectedEdgeId: null,
    selectedResultRunId: null,
    explorerTab: "constructs",
    explorerCollapsed: false,
    inspectorCollapsed: false,
    resultWorkspaceState: defaultResultWorkspaceState,
    methodSetupState: defaultMethodSetupState,
    largeModelViewState: defaultLargeModelViewState,
    runMonitor: defaultRunMonitor,
    diagramMode: "sem",
    diagramTool: "select",
    diagramOverlaySettings: defaultDiagramOverlaySettings,
    publicationDiagramSettings: defaultPublicationDiagramSettings,
    diagramLayout: syncedDiagramLayout(initialNodes, initialEdges),
    dataset: sampleDataset,
    datasetCatalog: [sampleDataset],
    datasetVersions: [],
    projectModels: [sampleProjectModel],
    activeModelId: sampleProjectModelId,
    modelPresentations: { [sampleProjectModelId]: sampleModelPresentation },
    standardSemModelV4Authorities: {},
    standardSemModelV4ScientificEditLocks: {},
    standardSemModelV4Layouts: {},
    standardSemModelV4Epochs: {},
    standardSemModelV4Persistence: {},
    standardSemModelV4DatasetDescriptors: {},
    datasetDescriptorOnly: false,
    savedReports: [],
    explorerSelection: { kind: "model", modelId: sampleProjectModelId },
    runs: [],
    analysisSettings: defaultAnalysisSettings,
    view: "models",
    workflowDestinationContext: null,
    workflowCommandContext: null,
    projectName: "Untitled project",
    projectId: null,
    projectPath: null,
    projectWritable: true,
    generalSemProjectDraftMode: null,
    generalSemPublicationPending: false,
    generalSemTransientWorkBlocker: null,
    past: [],
    future: [],
  }),
  loadProject: (project) => set((state) => ({
    nodes: project.nodes,
    edges: project.edges,
    dataset: project.dataset,
    datasetCatalog: project.datasets?.length ? project.datasets : [project.dataset],
    datasetVersions: project.datasetVersions ?? [],
    projectModels: project.projectModels ?? [],
    activeModelId: project.activeModelId ?? null,
    modelPresentations: project.modelPresentations ?? {},
    standardSemModelV4Authorities: {},
    standardSemModelV4ScientificEditLocks: {},
    standardSemModelV4Layouts: {},
    standardSemModelV4Epochs: {},
    standardSemModelV4Persistence: {},
    standardSemModelV4DatasetDescriptors: {},
    datasetDescriptorOnly: false,
    projectId: null,
    generalSemTransientWorkBlocker: null,
    generalSemProjectDraftMode: project.preserveGeneralSemProjectDraftMode
      && state.generalSemProjectDraftMode
      && state.projectPath === null
      && state.projectId === project.preserveGeneralSemProjectDraftMode.sourceProjectId
      && state.generalSemProjectDraftMode.sourceProjectId === project.preserveGeneralSemProjectDraftMode.sourceProjectId
      ? state.generalSemProjectDraftMode
      : null,
    savedReports: project.savedReports ?? [],
    explorerSelection: project.explorerSelection
      ?? (project.activeModelId ? { kind: "model", modelId: project.activeModelId } : { kind: "data" }),
    runs: project.runs ?? [],
    analysisSettings: normalizeAnalysisSettings(project.analysisSettings ?? {}),
    diagramMode: project.diagramMode ?? "sem",
    diagramTool: "select",
    diagramOverlaySettings: normalizeDiagramOverlaySettings({ ...project.diagramOverlaySettings, selectedRunId: null }),
    publicationDiagramSettings: normalizePublicationDiagramSettings(project.publicationDiagramSettings),
    diagramLayout: syncedDiagramLayout(project.nodes, project.edges, project.diagramLayout),
    selectedNodeId: project.nodes[0]?.id ?? null,
    selectedEdgeId: null,
    selectedResultRunId: null,
    explorerTab: "constructs",
    explorerCollapsed: false,
    inspectorCollapsed: false,
    resultWorkspaceState: defaultResultWorkspaceState,
    largeModelViewState: defaultLargeModelViewState,
    runMonitor: defaultRunMonitor,
    view: "models",
    workflowDestinationContext: null,
    workflowCommandContext: null,
    past: [],
    future: [],
  })),
}));
