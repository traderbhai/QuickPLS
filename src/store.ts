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
import type { AnalysisMethodId, AnalysisRun, AnalysisUiSettings, ConstructData, Dataset, DiagramLayoutState, DiagramMode, DiagramOverlaySettings, DiagramToolMode, ExplorerTab, IndicatorSide, LargeModelViewState, MethodPresetId, MethodSetupState, OnboardingState, PublicationDiagramSettings, ResultWorkspaceState, ToastNotification, UiPreferences, WorkspaceView } from "./types";

type AlignTarget = "left" | "centerX" | "right" | "top" | "centerY" | "bottom";
type DistributeAxis = "horizontal" | "vertical";
type PathRouting = "smoothstep" | "default" | "straight";

interface HistorySnapshot {
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  diagramLayout: DiagramLayoutState;
}

interface WorkspaceState {
  view: WorkspaceView;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  selectedNodeId: string | null;
  selectedEdgeId: string | null;
  selectedResultRunId: string | null;
  explorerTab: ExplorerTab;
  explorerCollapsed: boolean;
  explorerWidth: number;
  uiPreferences: UiPreferences;
  resultWorkspaceState: ResultWorkspaceState;
  methodSetupState: MethodSetupState;
  onboardingState: OnboardingState;
  largeModelViewState: LargeModelViewState;
  commandPaletteOpen: boolean;
  shortcutOverlayOpen: boolean;
  toasts: ToastNotification[];
  diagramMode: DiagramMode;
  diagramTool: DiagramToolMode;
  diagramOverlaySettings: DiagramOverlaySettings;
  publicationDiagramSettings: PublicationDiagramSettings;
  diagramLayout: DiagramLayoutState;
  dataset: Dataset;
  runs: AnalysisRun[];
  analysisSettings: AnalysisUiSettings;
  projectName: string;
  projectPath: string | null;
  past: HistorySnapshot[];
  future: HistorySnapshot[];
  setView: (view: WorkspaceView) => void;
  setSelectedNode: (id: string | null) => void;
  setSelectedEdge: (id: string | null) => void;
  setSelectedResultRun: (id: string | null) => void;
  setExplorerTab: (tab: ExplorerTab) => void;
  setExplorerCollapsed: (collapsed: boolean) => void;
  setExplorerWidth: (width: number) => void;
  setUiPreferences: (patch: Partial<UiPreferences>) => void;
  setResultWorkspaceState: (patch: Partial<ResultWorkspaceState>) => void;
  setMethodSetupState: (patch: Partial<MethodSetupState>) => void;
  applyMethodPreset: (preset: MethodPresetId) => void;
  setOnboardingState: (patch: Partial<OnboardingState>) => void;
  setLargeModelViewState: (patch: Partial<LargeModelViewState>) => void;
  setCommandPaletteOpen: (open: boolean) => void;
  setShortcutOverlayOpen: (open: boolean) => void;
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
  addTwoStageInteraction: (predictor: string, moderator: string, outcome: string) => void;
  updateConstruct: (id: string, patch: Partial<ConstructData>) => void;
  updateEdge: (id: string, patch: Partial<Edge>) => void;
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
  addRun: (run: AnalysisRun) => void;
  setAnalysisSettings: (patch: Partial<AnalysisUiSettings>) => void;
  setProjectMeta: (name: string, path: string | null) => void;
  resetProject: () => void;
  loadProject: (project: { nodes: Array<Node<ConstructData>>; edges: Edge[]; dataset: Dataset; runs?: AnalysisRun[]; analysisSettings?: AnalysisUiSettings; diagramMode?: DiagramMode; diagramOverlaySettings?: Partial<DiagramOverlaySettings>; publicationDiagramSettings?: Partial<PublicationDiagramSettings>; diagramLayout?: Partial<DiagramLayoutState> }) => void;
}

const supportedAnalysisMethods = new Set<AnalysisMethodId>(["pls_pm", "bootstrap", "plsc", "wpls", "cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation", "predict", "mga", "ipma", "cbsem", "pca", "gsca", "regression", "nca"]);

const defaultAnalysisSettings: AnalysisUiSettings = { method: "pls_pm", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0, seed: 20260718, workers: 1, confidenceLevel: 0.95, caseWeightColumn: null, groupColumn: null, ipmaTargets: null, groupMethods: "micom,mga_permutation", groupPermutationSamples: 999, segmentCount: 2, segmentStarts: 10, minimumSegmentShare: 0.10, cbsemModelType: "sem", cbsemMeanStructure: false, cbsemStandardization: "std_all", cbsemGroupColumn: null, cbsemInvarianceSteps: "configural,metric,scalar", cbsemBootstrapSamples: 0, pcaVariables: null, pcaComponentRule: "kaiser", pcaComponents: 2, regressionType: "ols", regressionOutcome: null, regressionPredictors: null, regressionControls: null, robustSe: "hc3", processModel: "mediation", processX: null, processM: null, processW: null, ncaX: null, ncaY: null, ncaCeiling: "both", ncaPermutationSamples: 999 };
const defaultDiagramOverlaySettings: DiagramOverlaySettings = { selectedRunId: null, mode: "model", precision: 3, showLoadings: true, showPathCoefficients: true, showPValues: false, showTValues: false, showRSquared: true, showWarnings: true, showWatermark: true };
const defaultPublicationDiagramSettings: PublicationDiagramSettings = { mode: "smartpls_result", precision: 3, overlayMode: "paths_r2", aspectRatio: "wide", palette: "grayscale", layoutSource: "current_canvas", showLoadings: true, showPathCoefficients: true, showRSquared: true, showValidationWatermark: true, showUnsupportedWarning: true, showRunProvenance: true };
const defaultUiPreferences: UiPreferences = { density: "compact", tableDensity: "compact", defaultPrecision: 4, showAdvancedHelp: true, recentPanels: ["models", "runs", "reports"] };
const defaultResultWorkspaceState: ResultWorkspaceState = { selectedRunId: null, selectedTab: "summary", tableSearch: "", tableDensity: "compact", includeExperimental: false };
const defaultMethodSetupState: MethodSetupState = { mode: "basic", selectedPreset: "standard_pls", expandedSections: ["basic"] };
const defaultOnboardingState: OnboardingState = { dismissed: false, selectedDemo: "corporate_reputation", recentProjectCards: [] };
const defaultLargeModelViewState: LargeModelViewState = { indicatorsCollapsed: false, isolatedConstructId: null, neighborhoodMode: "off" };

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

const normalizeAnalysisSettings = (settings: Partial<AnalysisUiSettings>): AnalysisUiSettings => {
  const bootstrapSamples = Number.isFinite(settings.bootstrapSamples) ? Math.trunc(settings.bootstrapSamples!) : defaultAnalysisSettings.bootstrapSamples;
  const studentizedInnerSamples = Number.isFinite(settings.studentizedInnerSamples) ? Math.trunc(settings.studentizedInnerSamples!) : defaultAnalysisSettings.studentizedInnerSamples;
  const permutationSamples = Number.isFinite(settings.permutationSamples) ? Math.trunc(settings.permutationSamples!) : defaultAnalysisSettings.permutationSamples;
  const seed = Number.isFinite(settings.seed) ? Math.trunc(settings.seed!) : defaultAnalysisSettings.seed;
  const workers = Number.isFinite(settings.workers) ? Math.trunc(settings.workers!) : defaultAnalysisSettings.workers;
  const confidenceLevel = Number.isFinite(settings.confidenceLevel) ? settings.confidenceLevel! : defaultAnalysisSettings.confidenceLevel;
  const caseWeightColumn = typeof settings.caseWeightColumn === "string" && settings.caseWeightColumn.trim() ? settings.caseWeightColumn.trim() : null;
  const groupColumn = typeof settings.groupColumn === "string" && settings.groupColumn.trim() ? settings.groupColumn.trim() : null;
  const ipmaTargets = typeof settings.ipmaTargets === "string" && settings.ipmaTargets.trim() ? settings.ipmaTargets.trim() : null;
  const groupMethods = typeof settings.groupMethods === "string" && settings.groupMethods.trim() ? settings.groupMethods.trim() : defaultAnalysisSettings.groupMethods;
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
  const regressionType = settings.regressionType === "logistic" || settings.regressionType === "process" ? settings.regressionType : defaultAnalysisSettings.regressionType!;
  const regressionOutcome = typeof settings.regressionOutcome === "string" && settings.regressionOutcome.trim() ? settings.regressionOutcome.trim() : null;
  const regressionPredictors = typeof settings.regressionPredictors === "string" && settings.regressionPredictors.trim() ? settings.regressionPredictors.trim() : null;
  const regressionControls = typeof settings.regressionControls === "string" && settings.regressionControls.trim() ? settings.regressionControls.trim() : null;
  const robustSe = settings.robustSe === "none" || settings.robustSe === "hc0" || settings.robustSe === "hc4" ? settings.robustSe : defaultAnalysisSettings.robustSe!;
  const processModel = settings.processModel === "moderation" || settings.processModel === "moderated_mediation" ? settings.processModel : defaultAnalysisSettings.processModel!;
  const processX = typeof settings.processX === "string" && settings.processX.trim() ? settings.processX.trim() : null;
  const processM = typeof settings.processM === "string" && settings.processM.trim() ? settings.processM.trim() : null;
  const processW = typeof settings.processW === "string" && settings.processW.trim() ? settings.processW.trim() : null;
  const ncaX = typeof settings.ncaX === "string" && settings.ncaX.trim() ? settings.ncaX.trim() : null;
  const ncaY = typeof settings.ncaY === "string" && settings.ncaY.trim() ? settings.ncaY.trim() : null;
  const ncaCeiling = settings.ncaCeiling === "ce_fdh" || settings.ncaCeiling === "cr_fdh" ? settings.ncaCeiling : defaultAnalysisSettings.ncaCeiling!;
  const ncaPermutationSamples = Number.isFinite(settings.ncaPermutationSamples) ? Math.trunc(settings.ncaPermutationSamples!) : defaultAnalysisSettings.ncaPermutationSamples!;
  const method = typeof settings.method === "string" && supportedAnalysisMethods.has(settings.method as AnalysisMethodId) ? settings.method as AnalysisMethodId : defaultAnalysisSettings.method;
  const clampedStudentized = Math.min(999, Math.max(99, studentizedInnerSamples));
  const normalizedStudentized = studentizedInnerSamples === 0 ? 0 : clampedStudentized % 2 === 0 ? Math.min(999, clampedStudentized + 1) : clampedStudentized;
  const normalizedBootstrap = normalizedStudentized > 0 ? Math.max(999, bootstrapSamples) : bootstrapSamples;
  return {
    method,
    bootstrapSamples: Math.min(10000, Math.max(0, normalizedBootstrap)),
    studentizedInnerSamples: normalizedStudentized,
    permutationSamples: permutationSamples === 0 ? 0 : Math.min(10000, Math.max(99, permutationSamples)),
    seed: Math.min(4294967295, Math.max(0, seed)),
    workers: Math.min(64, Math.max(1, workers)),
    confidenceLevel: Math.min(0.999, Math.max(0.8, confidenceLevel)),
    caseWeightColumn,
    groupColumn,
    ipmaTargets,
    groupMethods,
    groupPermutationSamples: Math.min(10000, Math.max(1, groupPermutationSamples)),
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
    regressionType,
    regressionOutcome,
    regressionPredictors,
    regressionControls,
    robustSe,
    processModel,
    processX,
    processM,
    processW,
    ncaX,
    ncaY,
    ncaCeiling,
    ncaPermutationSamples: Math.min(10000, Math.max(1, ncaPermutationSamples)),
  };
};

const historyPatch = (state: WorkspaceState) => ({
  past: [...state.past.slice(-49), { nodes: state.nodes, edges: state.edges, diagramLayout: state.diagramLayout }],
  future: [],
});

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

const validUniqueIndicators = (indicators: string[], dataset: Dataset) =>
  [...new Set(indicators)].filter((indicator): indicator is string => typeof indicator === "string" && dataset.columns.includes(indicator));

export const useWorkspace = create<WorkspaceState>()((set) => ({
  view: "welcome",
  nodes: initialNodes,
  edges: initialEdges,
  selectedNodeId: "satisfaction",
  selectedEdgeId: null,
  selectedResultRunId: null,
  explorerTab: "constructs",
  explorerCollapsed: false,
  explorerWidth: 330,
  uiPreferences: defaultUiPreferences,
  resultWorkspaceState: defaultResultWorkspaceState,
  methodSetupState: defaultMethodSetupState,
  onboardingState: defaultOnboardingState,
  largeModelViewState: defaultLargeModelViewState,
  commandPaletteOpen: false,
  shortcutOverlayOpen: false,
  toasts: [],
  diagramMode: "sem",
  diagramTool: "select",
  diagramOverlaySettings: defaultDiagramOverlaySettings,
  publicationDiagramSettings: defaultPublicationDiagramSettings,
  diagramLayout: syncedDiagramLayout(initialNodes, initialEdges),
  dataset: sampleDataset,
  runs: [],
  analysisSettings: defaultAnalysisSettings,
  projectName: "Corporate Reputation Study",
  projectPath: null,
  past: [],
  future: [],
  setView: (view) => set({ view }),
  setSelectedNode: (selectedNodeId) => set({ selectedNodeId, selectedEdgeId: null }),
  setSelectedEdge: (selectedEdgeId) => set({ selectedEdgeId, selectedNodeId: null }),
  setSelectedResultRun: (selectedResultRunId) => set((state) => ({ selectedResultRunId, diagramOverlaySettings: { ...state.diagramOverlaySettings, selectedRunId: selectedResultRunId } })),
  setExplorerTab: (explorerTab) => set({ explorerTab }),
  setExplorerCollapsed: (explorerCollapsed) => set({ explorerCollapsed }),
  setExplorerWidth: (explorerWidth) => set({ explorerWidth: Math.min(430, Math.max(250, Math.trunc(explorerWidth))) }),
  setUiPreferences: (patch) => set((state) => ({ uiPreferences: { ...state.uiPreferences, ...patch, defaultPrecision: Math.min(6, Math.max(2, Math.trunc(patch.defaultPrecision ?? state.uiPreferences.defaultPrecision))) } })),
  setResultWorkspaceState: (patch) => set((state) => ({ resultWorkspaceState: { ...state.resultWorkspaceState, ...patch } })),
  setMethodSetupState: (patch) => set((state) => ({ methodSetupState: { ...state.methodSetupState, ...patch, expandedSections: patch.expandedSections ?? state.methodSetupState.expandedSections } })),
  applyMethodPreset: (preset) => set((state) => {
    const presets: Record<MethodPresetId, Partial<AnalysisUiSettings>> = {
      standard_pls: { method: "pls_pm", bootstrapSamples: 0, studentizedInnerSamples: 0, permutationSamples: 0 },
      pls_bootstrap: { method: "bootstrap", bootstrapSamples: 5000, studentizedInnerSamples: 0, permutationSamples: 0 },
      plspredict: { method: "predict", groupMethods: "pls_pos", segmentCount: 2, segmentStarts: 10 },
      micom_mga: { method: "mga", groupMethods: "micom,mga_permutation", groupPermutationSamples: 999 },
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
  pushToast: (toast) => {
    const id = crypto.randomUUID();
    set((state) => ({ toasts: [{ id, ...toast }, ...state.toasts].slice(0, 4) }));
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
    return {
      nodes: previous.nodes,
      edges: previous.edges,
      diagramLayout: previous.diagramLayout,
      past: state.past.slice(0, -1),
      future: [{ nodes: state.nodes, edges: state.edges, diagramLayout: state.diagramLayout }, ...state.future].slice(0, 50),
      selectedNodeId: null,
      selectedEdgeId: null,
      selectedResultRunId: null,
    };
  }),
  redo: () => set((state) => {
    const next = state.future[0];
    if (!next) return state;
    return {
      nodes: next.nodes,
      edges: next.edges,
      diagramLayout: next.diagramLayout,
      past: [...state.past, { nodes: state.nodes, edges: state.edges, diagramLayout: state.diagramLayout }].slice(-50),
      future: state.future.slice(1),
      selectedNodeId: null,
      selectedEdgeId: null,
      selectedResultRunId: null,
    };
  }),
  onNodesChange: (changes) => set((state) => {
    const nodes = applyNodeChanges(changes, state.nodes);
    const layout = syncedDiagramLayout(nodes, state.edges, {
      ...state.diagramLayout,
      constructLayouts: {
        ...state.diagramLayout.constructLayouts,
        ...Object.fromEntries(nodes.map((node) => [node.id, { ...(state.diagramLayout.constructLayouts[node.id] ?? {}), x: node.position.x, y: node.position.y }])),
      },
    });
    return {
      ...(changes.some((change) => change.type === "remove") ? historyPatch(state) : {}),
      nodes,
      diagramLayout: layout,
    };
  }),
  onEdgesChange: (changes) => set((state) => ({
    ...(changes.some((change) => change.type === "remove") ? historyPatch(state) : {}),
    edges: applyEdgeChanges(changes, state.edges),
    diagramLayout: syncedDiagramLayout(state.nodes, applyEdgeChanges(changes, state.edges), state.diagramLayout),
  })),
  onConnect: (connection) => set((state) => {
    if (!connection.source || !connection.target || connection.source === connection.target) return state;
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
    if (!connection.source || !connection.target || connection.source === connection.target) return state;
    if (state.edges.some((candidate) => candidate.id !== edge.id && candidate.source === connection.source && candidate.target === connection.target)) return state;
    return {
      ...historyPatch(state),
      edges: reconnectEdge(edge, connection, state.edges, { shouldReplaceId: false }),
      selectedNodeId: null,
      selectedEdgeId: edge.id,
    };
  }),
  addPath: (source, target) => set((state) => {
    if (source === target || state.edges.some((edge) => edge.source === source && edge.target === target)) return state;
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
    if (source === target) return state;
    const [left, right] = [source, target].sort();
    if (state.edges.some((edge) => edge.data?.role === "covariance" && [edge.source, edge.target].sort().join("\u0000") === `${left}\u0000${right}`)) return state;
    const id = `covariance-${left}-${right}-${Date.now()}`;
    return {
      ...historyPatch(state),
      selectedNodeId: null,
      selectedEdgeId: id,
      edges: [...state.edges, { id, source: left, target: right, type: "default", label: "Covariance", data: { role: "covariance" } }],
    };
  }),
  addTwoStageInteraction: (predictor, moderator, outcome) => set((state) => {
    if (new Set([predictor, moderator, outcome]).size !== 3) return state;
    const predictorNode = state.nodes.find((node) => node.id === predictor);
    const moderatorNode = state.nodes.find((node) => node.id === moderator);
    const outcomeNode = state.nodes.find((node) => node.id === outcome);
    if (!predictorNode || !moderatorNode || !outcomeNode) return state;
    if (state.nodes.some((node) => node.data.interaction?.predictor === predictor && node.data.interaction?.moderator === moderator && node.data.interaction?.outcome === outcome)) return state;
    const baseId = `interaction-${predictor}-${moderator}-${outcome}`.replace(/[^a-zA-Z0-9_-]/g, "-");
    const id = state.nodes.some((node) => node.id === baseId) ? `${baseId}-${Date.now()}` : baseId;
    const edgeId = `path-${id}-${outcome}`;
    const shortName = `${predictorNode.data.shortName}x${moderatorNode.data.shortName}`.replace(/[^a-zA-Z0-9]/g, "").slice(0, 8) || "INT";
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
          interaction: { predictor, moderator, outcome, method: "two_stage_product_score" },
        },
      }],
      edges: state.edges.some((edge) => edge.source === id && edge.target === outcome)
        ? state.edges
        : addEdge({
          id: edgeId,
          source: id,
          target: outcome,
          type: "straight",
          label: "Interaction",
          markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
        }, state.edges),
    };
  }),
  updateConstruct: (id, patch) => set((state) => ({
    ...historyPatch(state),
    nodes: state.nodes.map((node) => node.id === id ? { ...node, data: { ...node.data, ...patch } } : node),
  })),
  updateEdge: (id, patch) => set((state) => ({
    ...historyPatch(state),
    edges: state.edges.map((edge) => edge.id === id ? { ...edge, ...patch } : edge),
  })),
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
    const id = `construct-${Date.now()}`;
    const name = nextConstructName(state.nodes);
    const fallback = nextConstructPosition(state.nodes);
    const validIndicators = validUniqueIndicators(indicators, state.dataset);
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
    const validIndicators = validUniqueIndicators(indicators, state.dataset);
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
    const validIndicators = validUniqueIndicators(indicators, state.dataset);
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
    const nodeIds = new Set([
      ...state.nodes.filter((node) => node.selected).map((node) => node.id),
      ...(state.selectedNodeId ? [state.selectedNodeId] : []),
    ]);
    const edgeIds = new Set([
      ...state.edges.filter((edge) => edge.selected).map((edge) => edge.id),
      ...(state.selectedEdgeId ? [state.selectedEdgeId] : []),
    ]);
    if (nodeIds.size === 0 && edgeIds.size === 0) return state;
    return {
      ...historyPatch(state),
      nodes: state.nodes.filter((node) => !nodeIds.has(node.id)),
      edges: state.edges.filter((edge) => !edgeIds.has(edge.id) && !nodeIds.has(edge.source) && !nodeIds.has(edge.target)),
      selectedNodeId: null,
      selectedEdgeId: null,
    };
  }),
  reverseSelectedPath: () => set((state) => {
    const selected = state.edges.find((edge) => edge.id === state.selectedEdgeId);
    if (!selected || state.edges.some((edge) => edge.id !== selected.id && edge.source === selected.target && edge.target === selected.source)) return state;
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
    const target = state.nodes.find((node) => node.id === constructId);
    if (!target || target.data.indicators.includes(indicator)) return state;
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
    const target = state.nodes.find((node) => node.id === constructId);
    const unique = [...new Set(indicators)].filter((indicator) => state.dataset.columns.includes(indicator));
    if (!target || unique.length === 0) return state;
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
  unassignIndicator: (constructId, indicator) => set((state) => ({
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
  setDataset: (dataset) => set({ dataset, view: "data" }),
  addRun: (run) => set((state) => ({
    runs: [run, ...state.runs],
    selectedResultRunId: run.result ? run.id : state.selectedResultRunId,
    diagramOverlaySettings: run.result ? { ...state.diagramOverlaySettings, selectedRunId: run.id, mode: state.diagramOverlaySettings.mode === "model" ? "paths_r2" : state.diagramOverlaySettings.mode } : state.diagramOverlaySettings,
    view: "runs",
  })),
  setAnalysisSettings: (patch) => set((state) => ({ analysisSettings: normalizeAnalysisSettings({ ...state.analysisSettings, ...patch }) })),
  setProjectMeta: (projectName, projectPath) => set({ projectName, projectPath }),
  resetProject: () => set({
    nodes: initialNodes,
    edges: initialEdges,
    selectedNodeId: "satisfaction",
    selectedEdgeId: null,
    selectedResultRunId: null,
    explorerTab: "constructs",
    explorerCollapsed: false,
    resultWorkspaceState: defaultResultWorkspaceState,
    methodSetupState: defaultMethodSetupState,
    largeModelViewState: defaultLargeModelViewState,
    diagramMode: "sem",
    diagramTool: "select",
    diagramOverlaySettings: defaultDiagramOverlaySettings,
    publicationDiagramSettings: defaultPublicationDiagramSettings,
    diagramLayout: syncedDiagramLayout(initialNodes, initialEdges),
    dataset: sampleDataset,
    runs: [],
    analysisSettings: defaultAnalysisSettings,
    view: "models",
    projectName: "Untitled project",
    projectPath: null,
    past: [],
    future: [],
  }),
  loadProject: (project) => set({
    nodes: project.nodes,
    edges: project.edges,
    dataset: project.dataset,
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
    resultWorkspaceState: defaultResultWorkspaceState,
    largeModelViewState: defaultLargeModelViewState,
    view: "models",
    past: [],
    future: [],
  }),
}));
