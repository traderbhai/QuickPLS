import type { Edge, Node } from "@xyflow/react";

export type WorkspaceView = "welcome" | "data" | "models" | "analyses" | "run" | "runs" | "groups" | "reports" | "trust" | "settings";
export type ExplorerTab = "constructs" | "variables" | "structure" | "issues";
export type UiDensity = "comfortable" | "compact";
export type DesktopMenuId = "file" | "edit" | "data" | "model" | "calculate" | "results" | "report" | "view" | "tools" | "window" | "help";
export type DesktopDialogId = "new_project" | "open_project" | "import_data" | "export_options" | "calculation_setup" | "method_scope" | "settings" | "help_shortcuts" | null;
export type DesktopCommandTone = "info" | "success" | "warning" | "error";
export type ResultWorkspaceTab = "overview" | "measurement" | "structural" | "validity" | "inference" | "prediction" | "groups" | "diagnostics" | "interpretation" | "comparison";
export type MethodSetupMode = "basic" | "expert";
export type MethodPresetId = "standard_pls" | "pls_bootstrap" | "plspredict" | "micom_mga" | "cbsem_cfa" | "ols_regression" | "nca";
export type MeasurementMode = "reflective" | "formative";
export type MethodStatus = "experimental" | "validated" | "unsupported";

export interface WorkflowDestinationContext {
  from: WorkspaceView;
  to: WorkspaceView;
  actionLabel: string;
  coachId: string;
  timestamp: number;
}

export interface WorkflowCommandContext {
  from: WorkspaceView;
  event: string;
  actionLabel: string;
  coachId: string;
  timestamp: number;
}

export interface DesktopCommandStatus {
  id: string;
  label: string;
  detail: string;
  tone: DesktopCommandTone;
  timestamp: number;
}

export type RunMonitorStatus = "idle" | "blocked" | "queued" | "validating" | "running" | "cancelling" | "completed" | "failed" | "cancelled";

export interface RunMonitorLogEntry {
  id: string;
  timestamp: string;
  phase: string;
  message: string;
  tone: DesktopCommandTone;
}

export interface RunMonitorState {
  status: RunMonitorStatus;
  phase: string;
  message: string;
  completedUnits: number;
  totalUnits: number;
  startedAt: string | null;
  completedAt: string | null;
  activeJobId: string | null;
  lastRunId: string | null;
  error: string | null;
  logs: RunMonitorLogEntry[];
}

export type AnalysisMethodId = "pls_pm" | "bootstrap" | "permutation" | "plsc" | "wpls" | "cca" | "cta_pls" | "endogeneity" | "nonlinear_effects" | "moderated_mediation" | "predict" | "mga" | "ipma" | "cbsem" | "pca" | "gsca" | "regression" | "nca";
export type DiagramMode = "compact" | "sem" | "publication" | "smartpls_result";
export type DiagramOverlayMode = "model" | "loadings" | "paths_r2" | "significance" | "quality" | "cbsem_standardized" | "cbsem_residuals" | "modification_indices";
export type DiagramToolMode = "select" | "pan" | "construct" | "indicator" | "path" | "covariance" | "residual" | "caption" | "measurement" | "interaction" | "higher_order";
export type IndicatorSide = "left" | "right" | "top" | "bottom" | "free";
export type EdgeRouteStyle = "straight" | "curved" | "orthogonal";

export interface DiagramPoint {
  x: number;
  y: number;
}

export interface ConstructLayout {
  x: number;
  y: number;
  width?: number;
  height?: number;
  pinned?: boolean;
}

export interface IndicatorLayout {
  side: IndicatorSide;
  x?: number;
  y?: number;
  order: number;
  pinned?: boolean;
}

export interface EdgeLayout {
  routing: EdgeRouteStyle;
  bendPoints?: DiagramPoint[];
  labelOffset?: DiagramPoint;
  pinned?: boolean;
}

export interface DiagramViewport {
  x: number;
  y: number;
  zoom: number;
}

export interface DiagramLayoutState {
  diagramVersion: "sem_designer_v1";
  constructLayouts: Record<string, ConstructLayout>;
  indicatorLayouts: Record<string, Record<string, IndicatorLayout>>;
  edgeLayouts: Record<string, EdgeLayout>;
  diagramViewport?: DiagramViewport;
  diagramTheme: "academic_grayscale" | "smartpls_like" | "quickpls_color" | "journal_mono" | "high_contrast";
  showGrid: boolean;
  layoutLocked: boolean;
}

export interface UiPreferences {
  density: UiDensity;
  tableDensity: UiDensity;
  defaultPrecision: number;
  showAdvancedHelp: boolean;
  recentPanels: WorkspaceView[];
  methodScopeDrawerOpen: boolean;
  showThresholdColors: boolean;
  focusDiagramMode: boolean;
  selectedExportPreset: "journal_figure" | "journal_tables" | "thesis_appendix" | "reviewer_pack" | "full_reproducibility_report";
}

export interface ResultWorkspaceState {
  selectedRunId: string | null;
  selectedTab: ResultWorkspaceTab;
  tableSearch: string;
  tableDensity: UiDensity;
  includeExperimental: boolean;
  selectedDetailRow: string | null;
  resultPrecision: number;
  tableSort: string | null;
  activeInterpretationPanel: string | null;
  comparisonRunIds: string[];
  showInterpretationColumns: boolean;
}

export interface MethodSetupState {
  mode: MethodSetupMode;
  selectedPreset: MethodPresetId;
  expandedSections: string[];
}

export type NativeSampleProjectId = "corporate_reputation" | "simple_pls" | "mediation";

export interface OnboardingState {
  dismissed: boolean;
  selectedDemo: NativeSampleProjectId;
  recentProjectCards: string[];
}

export interface LargeModelViewState {
  indicatorsCollapsed: boolean;
  isolatedConstructId: string | null;
  neighborhoodMode: "off" | "selected" | "upstream_downstream";
}

export interface ToastNotification {
  id: string;
  tone: "success" | "warning" | "info" | "error";
  title: string;
  detail?: string;
}

export interface AnalysisUiSettings {
  method: AnalysisMethodId;
  weightingScheme?: "path" | "factor" | "pca";
  tolerance?: number;
  maxIterations?: number;
  preprocessing?: "standardized" | "mean_centered" | "unstandardized";
  bootstrapSamples: number;
  studentizedInnerSamples: number;
  permutationSamples: number;
  seed: number;
  workers: number;
  confidenceLevel: number;
  caseWeightColumn?: string | null;
  groupColumn?: string | null;
  groupAValue?: string | null;
  groupBValue?: string | null;
  ipmaTargets?: string | null;
  groupMethods?: string | null;
  groupPermutationSamples?: number;
  micomConfiguralConfirmed?: boolean;
  segmentCount?: number;
  segmentStarts?: number;
  minimumSegmentShare?: number;
  cbsemModelType?: "cfa" | "sem";
  cbsemMeanStructure?: boolean;
  cbsemStandardization?: "std_lv" | "std_all";
  cbsemGroupColumn?: string | null;
  cbsemInvarianceSteps?: string | null;
  cbsemBootstrapSamples?: number;
  pcaVariables?: string | null;
  pcaComponentRule?: "kaiser" | "fixed" | "variance_threshold";
  pcaComponents?: number;
  pcaVarianceThreshold?: number;
  regressionType?: "ols" | "logistic" | "process";
  regressionOutcome?: string | null;
  regressionPredictors?: string | null;
  regressionControls?: string | null;
  regressionBootstrap?: boolean;
  robustSe?: "none" | "hc0" | "hc3" | "hc4";
  processModel?: "mediation" | "moderation" | "moderated_mediation";
  processX?: string | null;
  processM?: string | null;
  processW?: string | null;
  processGraph?: NativeProcessGraphRelationshipConfig | null;
  ncaX?: string | null;
  ncaY?: string | null;
  ncaCeiling?: "ce_fdh" | "cr_fdh" | "both";
  ncaPermutationSamples?: number;
}

export interface ConstructData {
  [key: string]: unknown;
  label: string;
  shortName: string;
  mode: MeasurementMode;
  indicators: string[];
  semantic?: "interaction" | "higher_order";
  interaction?: InteractionData;
  higherOrder?: HigherOrderConstructData;
  score?: number;
  resultLoadings?: Record<string, number>;
  resultR2?: number;
}

export interface DiagramOverlaySettings {
  selectedRunId: string | null;
  mode: DiagramOverlayMode;
  precision: number;
  showLoadings: boolean;
  showPathCoefficients: boolean;
  showPValues: boolean;
  showTValues: boolean;
  showRSquared: boolean;
  showWarnings: boolean;
  showWatermark: boolean;
}

export interface PublicationDiagramSettings {
  mode: DiagramMode;
  precision: number;
  overlayMode: DiagramOverlayMode;
  aspectRatio: "wide" | "square" | "portrait";
  palette: "color" | "monochrome" | "grayscale" | "high_contrast" | "quickpls_color";
  layoutSource: "current_canvas" | "tidy_publication";
  showLoadings: boolean;
  showPathCoefficients: boolean;
  showRSquared: boolean;
  showValidationWatermark: boolean;
  showUnsupportedWarning: boolean;
  showRunProvenance: boolean;
}

export interface InteractionData {
  predictor: string;
  moderator: string;
  outcome: string;
  method: "two_stage_product_score";
}

export interface HigherOrderConstructData {
  id: string;
  components: string[];
  method: "repeated_indicators" | "two_stage" | "hybrid";
  stage_one_recipe?: string | null;
}

export interface ControlData {
  source: string;
  target: string;
  label?: string | null;
}

export interface PathEdgeData {
  role?: "control" | "covariance";
  controlLabel?: string | null;
}

export interface Dataset {
  id: string;
  name: string;
  columns: string[];
  rows: Array<Record<string, string | number | null>>;
  missing: number;
  rowCount?: number;
  missingByColumn?: Record<string, number>;
  fingerprint?: string;
  kind?: "raw" | "covariance" | "correlation";
  sampleSize?: number | null;
  columnMetadata?: ColumnMetadata[];
}

export interface DatasetRowsPage {
  datasetId: string;
  offset: number;
  limit: number;
  rowCount: number;
  rows: Dataset["rows"];
}

export interface DatasetGroupProfileValue {
  value: string;
  label: string | null;
  observations: number;
  completeCases: number;
}

export interface DatasetGroupProfile {
  datasetId: string;
  columnName: string;
  rowCount: number;
  missingCount: number;
  unsupportedCount: number;
  truncated: boolean;
  groups: DatasetGroupProfileValue[];
}

export type DatasetVersionOperation = "import" | "metadata" | "recode";

export interface DatasetVersionRecord {
  datasetId: string;
  parentDatasetId: string | null;
  operation: DatasetVersionOperation;
  createdAt: string | null;
  summary: string;
  sourceColumn: string | null;
  targetColumn: string | null;
}

export interface DatasetVersionMutation {
  dataset: Dataset;
  version: DatasetVersionRecord;
}

export type RecodeUnmappedPolicy = "keep_original" | "set_missing" | "error";

export interface RecodeValueMapping {
  source: string;
  target: string | null;
}

export interface RecodeColumnSpec {
  sourceColumn: string;
  targetColumn: string;
  targetLabel: string | null;
  targetType: ColumnMetadata["column_type"];
  targetScale: ColumnMetadata["scale_type"];
  mappings: RecodeValueMapping[];
  unmapped: RecodeUnmappedPolicy;
}

export interface ColumnMetadata {
  name: string;
  label: string | null;
  column_type: "numeric" | "text" | "boolean";
  scale_type: "continuous" | "ordinal" | "nominal" | "binary" | "identifier";
  missing_markers: string[];
  theoretical_min: number | null;
  theoretical_max: number | null;
  value_labels: Record<string, string>;
}

export interface NativeProjectCompatibilityNotice {
  resultId: string;
  code: string;
  message: string;
}

export interface NativeProjectFutureUnsupported {
  models: number;
  recipes: number;
  results: number;
}

export interface NativeProjectSnapshot {
  name: string;
  path: string | null;
  readOnly: boolean;
  sourceArchiveVersion: number;
  migrationPending: boolean;
  compatibilityNotices: NativeProjectCompatibilityNotice[];
  futureUnsupported: NativeProjectFutureUnsupported;
  saveWarning: string | null;
  recovered: boolean;
  recoverySource?: "autosave" | "backup" | null;
  datasets: Dataset[];
  datasetVersions: DatasetVersionRecord[];
  models?: NativeCanonicalModelSpec[];
  recipes?: NativeCanonicalAnalysisRecipe[];
  results?: AnalysisResultEnvelope[];
  activeModelId?: string | null;
  modelPresentations?: Record<string, NativeModelPresentation>;
  savedReports?: NativeSavedReport[];
  workspace?: {
    nodes: unknown[];
    edges: unknown[];
    runs?: Array<AnalysisRun | NativeWorkspaceRunPresentation>;
    analysisSettings?: AnalysisUiSettings;
    diagramMode?: DiagramMode;
    diagramOverlaySettings?: Partial<DiagramOverlaySettings>;
    publicationDiagramSettings?: Partial<PublicationDiagramSettings>;
    diagramLayout?: Partial<DiagramLayoutState>;
    activeDatasetId?: string;
    activeModelId?: string;
  } | null;
}

/** Presentation-only state for one canonical editable model. */
export interface NativeModelPresentation {
  nodes?: Array<Node<ConstructData>>;
  edges?: Edge[];
  diagramLayout?: Partial<DiagramLayoutState>;
}

export interface NativeSavedReport {
  resultId: string;
  name: string;
  savedAt: string;
}

export type NativeExplorerSelection =
  | { kind: "project" }
  | { kind: "data" }
  | { kind: "models" }
  | { kind: "model"; modelId: string }
  | { kind: "reports" }
  | { kind: "report"; resultId: string };

export type NativeProjectExplorerMutation =
  | { kind: "create_model"; name: string }
  | { kind: "activate_model"; modelId: string }
  | { kind: "rename_model"; modelId: string; name: string }
  | { kind: "delete_model"; modelId: string }
  | { kind: "save_report"; resultId: string; name: string }
  | { kind: "rename_report"; resultId: string; name: string }
  | { kind: "remove_report"; resultId: string };

export interface NativeProjectExplorerMutationRequest {
  mutation: NativeProjectExplorerMutation;
  currentModel?: NativeCanonicalModelSpec | null;
  currentPresentation?: NativeModelPresentation | null;
  path?: string | null;
}

/** In-process request/response detail used by Explorer dialogs. */
export interface NativeProjectExplorerMutationEventDetail {
  mutation: NativeProjectExplorerMutation;
  resolve: () => void;
  reject: (reason: unknown) => void;
}

export interface NativeCanonicalConstruct {
  id: string;
  name: string;
  short_name: string;
  mode: MeasurementMode;
  indicators: string[];
}

export interface NativeCanonicalStructuralPath {
  source: string;
  target: string;
}

export interface NativeCanonicalControlPath extends NativeCanonicalStructuralPath {
  label: string | null;
}

export interface NativeCanonicalHigherOrderConstruct {
  id: string;
  components: string[];
  method: "repeated_indicators" | "two_stage" | "hybrid";
  stage_one_recipe: string | null;
}

export interface NativeCanonicalInteraction {
  id: string;
  predictor: string;
  moderator: string;
  product_construct: string;
  outcome: string;
  method: "two_stage_product_score";
}

export interface NativeCanonicalModelSpec {
  id: string;
  name: string;
  constructs: NativeCanonicalConstruct[];
  paths: NativeCanonicalStructuralPath[];
  controls: NativeCanonicalControlPath[];
  higher_order_constructs: NativeCanonicalHigherOrderConstruct[];
  interactions: NativeCanonicalInteraction[];
}

export interface NativeCanonicalAnalysisRecipe {
  schema_version: number;
  id: string;
  created_at: string;
  dataset_fingerprint: string;
  model: NativeCanonicalModelSpec;
  settings: AnalysisEngineSettingsSnapshot;
  /** Required for schema v3; absent on readable legacy v1/v2 recipes. */
  method_config?: NativeAnalysisMethodConfig;
  metadata: Record<string, string>;
}

export type NativeAnalysisMethodConfig =
  | { kind: "pls_algorithm" }
  | { kind: "pls_bootstrap" }
  | { kind: "pls_permutation" }
  | { kind: "plsc" }
  | { kind: "wpls" }
  | { kind: "cca" }
  | { kind: "cta_pls" }
  | { kind: "endogeneity" }
  | { kind: "nonlinear_effects" }
  | { kind: "moderated_mediation" }
  | {
      kind: "predict";
      pls_pos?: NativePredictionSegmentationConfig;
      fimix?: NativePredictionSegmentationConfig;
    }
  | {
      kind: "mga";
      group_column: string;
      group_a: string;
      group_b: string;
      methods: Array<"micom" | "mga_permutation">;
      permutation_samples: number;
      configural_invariance_confirmed: boolean;
    }
  | { kind: "ipma"; targets: string[] }
  | {
      kind: "cbsem";
      model_type: "cfa" | "sem";
      estimator: "ml" | "robust_ml" | "wlsmv";
      input: "raw" | "covariance" | "correlation";
      mean_structure: boolean;
      bootstrap_samples: number;
      group_column?: string;
      invariance_steps?: Array<"configural" | "metric" | "scalar">;
    }
  | { kind: "pca"; variables: string[]; retention: NativePcaRetentionConfig }
  | { kind: "gsca" }
  | {
      kind: "regression";
      outcome: string;
      predictors: string[];
      controls?: string[];
      model: NativeRegressionModelConfig;
      bootstrap?: NativeRegressionBootstrapConfig;
    }
  | {
      kind: "nca";
      condition: string;
      outcome: string;
      ceiling: "ce_fdh" | "cr_fdh" | "both";
      permutation_samples: number;
    }
  | { kind: "legacy" };

export interface NativePredictionSegmentationConfig {
  segments: number;
  starts: number;
  minimum_segment_share: number;
}

export type NativePcaRetentionConfig =
  | { rule: "kaiser" }
  | { rule: "fixed"; components: number }
  | { rule: "variance_threshold"; threshold: number };

export type NativeRegressionModelConfig =
  | { type: "ols"; robust_se: "hc3" }
  | { type: "logistic" }
  | { type: "process"; relationship: NativeProcessRelationshipConfig };

export interface NativeRegressionBootstrapConfig {
  algorithm: "case_resampling";
  intervals: ["percentile", "bca"];
}

export type NativeProcessRelationshipConfig =
  | { model: "mediation"; x: string; mediator: string }
  | { model: "moderation"; x: string; moderator: string }
  | { model: "moderated_mediation"; x: string; mediator: string; moderator: string }
  | NativeProcessGraphRelationshipConfig;

export interface NativeProcessGraphRelationshipConfig {
  model: "graph";
  focal_predictor: string;
  paths: NativeProcessPathConfig[];
  moderators: NativeProcessModeratorConfig[];
  moderations: NativeProcessModerationConfig[];
  continuous_product_centering: "equation_complete_case_mean_v1";
}

export interface NativeProcessPathConfig {
  from: string;
  to: string;
}

export interface NativeProcessModeratorConfig {
  variable: string;
  scale: "continuous" | "binary_0_1";
}

export interface NativeProcessModerationConfig {
  from: string;
  to: string;
  moderator: string;
  conditioning_moderator?: string;
}

export interface AnalysisRun {
  id: string;
  modelId?: string | null;
  name: string;
  method: string;
  createdAt: string;
  seed: number;
  status: "completed" | "failed";
  warnings: string[];
  logs?: RunMonitorLogEntry[];
  fingerprint: string;
  modelSnapshot?: AnalysisModelSnapshot;
  result?: PlsResult;
  assessment?: AssessmentResult;
  bootstrap?: PlsBootstrapRun;
  permutation?: PlsPermutationRun;
  provenance?: AnalysisResultProvenance;
}

export type NativeWorkspaceRunPresentation = Omit<
  AnalysisRun,
  "result" | "assessment" | "bootstrap" | "permutation" | "provenance"
>;

export interface AnalysisModelSnapshot {
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  diagramLayout?: DiagramLayoutState;
}

export interface PlsResult {
  method_version: string;
  converged: boolean;
  iterations: number;
  used_observations: number;
  omitted_observations: number;
  outer_estimates: Array<{ construct: string; indicator: string; weight: number; loading: number }>;
  paths: Array<{ source: string; target: string; coefficient: number }>;
  control_estimates?: Array<{ source: string; target: string; label?: string | null; coefficient: number }>;
  effects: Array<{ source: string; target: string; direct: number; indirect: number; total: number }>;
  mediation?: MediationAnalysis;
  moderation?: ModerationAnalysis;
  plsc?: PlscAnalysis | null;
  endogeneity?: GaussianCopulaEndogeneityAnalysis | null;
  nonlinear_effects?: NonlinearEffectsAnalysis | null;
  moderated_mediation?: ModeratedMediationAnalysis | null;
  cta_pls?: CtaPlsAnalysis | null;
  wpls?: WplsAnalysis | null;
  cca?: CcaAnalysis | null;
  predict?: PlsPredictAnalysis | null;
  segmentation?: PlsSegmentationAnalysis | null;
  mga?: PlsMgaAnalysis | null;
  micom?: MicomAnalysis | null;
  mga_permutation?: PlsMgaPermutationAnalysis | null;
  fimix?: FimixPlsAnalysis | null;
  ipma?: IpmaAnalysis | null;
  cbsem?: CbsemAnalysis | null;
  pca?: PcaAnalysis | null;
  regression?: RegressionAnalysis | null;
  nca?: NcaAnalysis | null;
  gsca?: GscaAnalysis | null;
  r_squared: Record<string, number>;
  warnings: string[];
}

export interface PcaAnalysis {
  method_version: string;
  component_rule: string;
  retained_components: number;
  observations: number;
  variables: string[];
  components: Array<{ component: string; eigenvalue: number; explained_variance: number; cumulative_variance: number }>;
  loadings: Array<{ variable: string; component: string; loading: number; weight: number }>;
  scores: Array<{ observation: number; component: string; score: number }>;
  warnings: string[];
}

export interface RegressionAnalysis {
  method_version: string;
  regression_type: string;
  outcome: string;
  predictors: string[];
  controls: string[];
  observations: number;
  coefficients: Array<{ term: string; estimate: number; standard_error: number; statistic: number; p_value_two_sided: number; confidence_interval_lower: number; confidence_interval_upper: number; odds_ratio?: number | null; odds_ratio_confidence_interval_lower?: number | null; odds_ratio_confidence_interval_upper?: number | null }>;
  /** PROCESS v2 stores equation-specific fit inside process.graph_v2 and leaves this generic shell field null. */
  fit: { r_squared?: number | null; adjusted_r_squared?: number | null; f_statistic?: number | null; log_likelihood?: number | null; pseudo_r_squared?: number | null; aic: number; bic: number; rmse?: number | null; null_log_likelihood?: number | null; deviance?: number | null; null_deviance?: number | null; likelihood_ratio_chi_square?: number | null; likelihood_ratio_degrees_of_freedom?: number | null; likelihood_ratio_p_value?: number | null; pseudo_r_squared_method?: string | null } | null;
  predictions: Array<{ observation: number; fitted: number; residual?: number | null; probability?: number | null }>;
  logistic?: LogisticRegressionDiagnostics | null;
  bootstrap?: RegressionBootstrapAnalysis | null;
  process?: ProcessAnalysis | null;
  warnings: string[];
}

export interface ProcessAnalysis {
  method_version: string;
  model: string;
  /** Historical regression_process_v1 rows; current graph output is graph_v2. */
  effects: Array<{
    effect: string;
    estimate: number;
    lower_percentile?: number | null;
    upper_percentile?: number | null;
  }>;
  /** Historical regression_process_v1 rows; current graph output is graph_v2. */
  simple_slopes: Array<{ moderator_value: number; slope: number }>;
  warnings: string[];
  graph_v2?: ProcessGraphAnalysis | null;
}

export type ProcessVariableRole = "focal_predictor" | "mediator" | "moderator" | "outcome" | "control";
export type ProcessVariableScale = "continuous" | "binary_0_1";

export interface ProcessVariableProfile {
  variable: string;
  role: ProcessVariableRole;
  scale: ProcessVariableScale;
  raw_mean: number;
  raw_sample_sd: number;
  raw_min: number;
  raw_max: number;
  levels: number[];
}

export interface ProcessGraphPath {
  path_id: string;
  from: string;
  to: string;
}

export interface ProcessGraphModeration {
  moderation_id: string;
  from: string;
  to: string;
  moderator: string;
  conditioning_moderator?: string;
}

export type ProcessEquationCoefficientKind = "intercept" | "path" | "moderator_main" | "interaction" | "control";

export interface ProcessEquationCoefficient {
  term_id: string;
  kind: ProcessEquationCoefficientKind;
  variables: string[];
  estimate: number;
  standard_error: number;
  statistic: number;
  p_value_two_sided: number;
  confidence_interval_lower: number;
  confidence_interval_upper: number;
}

export interface ProcessEquationFit {
  observations: number;
  parameter_count: number;
  residual_sum_squares: number;
  total_sum_squares: number;
  r_squared: number;
  adjusted_r_squared: number;
  f_statistic: number;
  aic: number;
  bic: number;
  rmse: number;
}

export interface ProcessEquationAnalysis {
  equation_id: string;
  outcome: string;
  term_ids: string[];
  coefficients: ProcessEquationCoefficient[];
  coefficient_covariance: number[][];
  residual_degrees_of_freedom: number;
  fit: ProcessEquationFit;
}

export type ProcessReferenceEffectKind = "direct" | "indirect" | "total_indirect" | "total";

export interface ProcessReferenceEffect {
  effect_id: string;
  kind: ProcessReferenceEffectKind;
  path: string[];
  estimate: number;
}

export interface ProcessModeratorValue {
  variable: string;
  raw_value: number;
  coded_value: number;
}

export interface ProcessConditionalIndirectEffect {
  effect_id: string;
  path_id: string;
  moderator_values: ProcessModeratorValue[];
  estimate: number;
}

export interface ProcessModeratedMediationIndex {
  effect_id: string;
  path_id: string;
  moderated_edge: string;
  moderator: string;
  estimate: number;
}

export interface ProcessSimpleSlope {
  effect_id: string;
  moderation_id: string;
  moderator_values: ProcessModeratorValue[];
  estimate: number;
  standard_error: number;
  statistic: number;
  p_value_two_sided: number;
  confidence_interval_lower: number;
  confidence_interval_upper: number;
}

export interface ProcessPlotPoint {
  predictor_raw: number;
  predicted_raw: number;
  confidence_interval_lower: number;
  confidence_interval_upper: number;
}

export interface ProcessPlotSeries {
  series_id: string;
  moderator_values: ProcessModeratorValue[];
  points: ProcessPlotPoint[];
}

export interface ProcessConditionalPlot {
  plot_id: string;
  moderation_id: string;
  series: ProcessPlotSeries[];
}

export interface ProcessJohnsonNeymanRegion {
  lower: number;
  upper: number;
  status: "significant_negative" | "not_significant" | "significant_positive";
}

export interface ProcessJohnsonNeymanCurvePoint {
  moderator_raw: number;
  effect: number;
  standard_error: number;
  confidence_interval_lower: number;
  confidence_interval_upper: number;
}

interface ProcessJohnsonNeymanIdentity {
  moderation_id: string;
  solved_moderator: string;
  conditioning_values: ProcessModeratorValue[];
}

export type ProcessJohnsonNeymanAnalysis =
  | ProcessJohnsonNeymanIdentity & {
      status: "available";
      raw_min: number;
      raw_max: number;
      roots: number[];
      regions: ProcessJohnsonNeymanRegion[];
      curve_points: ProcessJohnsonNeymanCurvePoint[];
    }
  | ProcessJohnsonNeymanIdentity & {
      status: "unavailable";
      reason_code:
        | "binary_solved_moderator"
        | "invalid_hc3_covariance";
      message: string;
    };

export type ProcessBootstrapTest =
  | { status: "available"; statistic: number; p_value_two_sided: number }
  | { status: "unavailable"; reason_code: "zero_bootstrap_standard_error"; message: string };

export type ProcessBootstrapBcaInterval =
  | { status: "available"; bias_correction: number; acceleration: number; lower: number; upper: number }
  | {
      status: "unavailable";
      reason_code: "incomplete_jackknife" | "zero_jackknife_variance" | "nonfinite_adjusted_probability";
      message: string;
    };

export interface ProcessBootstrapEstimand {
  effect_id: string;
  original: number;
  bootstrap_mean: number;
  bias: number;
  standard_error: number;
  test: ProcessBootstrapTest;
  percentile_lower: number;
  percentile_upper: number;
  bca: ProcessBootstrapBcaInterval;
  usable_replicates: number;
}

export interface ProcessBootstrapFailure {
  replicate_index: number;
  reason_code:
    | "rank_deficient_equation"
    | "nonfinite_estimate"
    | "invalid_binary_profile"
    | "high_leverage_hc3_instability"
    | "invalid_hc3_covariance"
    | "degenerate_simple_slope_variance";
  message: string;
}

export interface ProcessBootstrapValidationWitness {
  method_version: "regression_process_bootstrap_validation_witness_v1";
  estimand_ids: string[];
  successful_bootstrap: Array<{ replicate_index: number; estimates: number[] }>;
  successful_jackknife: Array<{ omitted_case: number; estimates: number[] }>;
  failed_jackknife: Array<{ omitted_case: number; reason_code: string; message: string }>;
}

export interface ProcessBootstrapAnalysis {
  method_version: "regression_process_bootstrap_v1";
  algorithm: "indexed_case_resampling_v1";
  interval_policy: "percentile_primary_bca_conditional_v1";
  test_reference: "standard_normal_bootstrap_ratio_v1";
  requested_replicates: number;
  usable_replicates: number;
  minimum_usable_fraction: 0.9;
  jackknife_cases: number;
  usable_jackknife_cases: number;
  seed: number;
  workers: number;
  stream_token: "process_indexed_case_stream_v1";
  failed_replicates: ProcessBootstrapFailure[];
  estimands: ProcessBootstrapEstimand[];
  validation_witness: ProcessBootstrapValidationWitness;
  warnings: string[];
}

export interface ProcessGraphAnalysis {
  policies: {
    centering: "equation_complete_case_mean_v1";
    covariance: "hc3_v1";
    inference_reference: "student_t_residual_df_v1";
    confidence_level: 0.95;
  };
  complete_cases: number;
  omitted_cases: number;
  variable_profiles: ProcessVariableProfile[];
  paths: ProcessGraphPath[];
  moderations: ProcessGraphModeration[];
  equations: ProcessEquationAnalysis[];
  reference_effects: ProcessReferenceEffect[];
  conditional_indirect_effects: ProcessConditionalIndirectEffect[];
  moderated_mediation_indices: ProcessModeratedMediationIndex[];
  simple_slopes: ProcessSimpleSlope[];
  plots: ProcessConditionalPlot[];
  johnson_neyman: ProcessJohnsonNeymanAnalysis[];
  bootstrap?: ProcessBootstrapAnalysis | null;
}

export type RegressionBootstrapBcaInterval =
  | { status: "available"; bias_correction: number; acceleration: number; lower: number; upper: number }
  | { status: "unavailable"; reason_code: "insufficient_jackknife_estimates" | "incomplete_jackknife" | "degenerate_jackknife_acceleration"; message: string };

export interface RegressionBootstrapOddsRatioInterval {
  original: number;
  percentile_lower: number;
  percentile_upper: number;
  bca: RegressionBootstrapBcaInterval;
}

export interface RegressionBootstrapCoefficient {
  term: string;
  original: number;
  bootstrap_mean: number;
  bias: number;
  standard_error: number;
  replicate_max_abs: number;
  test_tolerance: number;
  test:
    | { status: "available"; statistic: number; p_value_two_sided: number }
    | { status: "unavailable"; reason_code: "degenerate_bootstrap_standard_error"; message: string };
  percentile_lower: number;
  percentile_upper: number;
  usable_replicates: number;
  bca: RegressionBootstrapBcaInterval;
  odds_ratio?: RegressionBootstrapOddsRatioInterval | null;
}

export interface RegressionBootstrapAnalysis {
  method_version: "regression_bootstrap_v1";
  algorithm: "indexed_case_resampling_v1";
  alternative: "two_sided";
  interval_policy: "percentile_primary_bca_conditional_v1";
  test_reference: "standard_normal_bootstrap_ratio_v1";
  test_tolerance_policy: "64eps_max_1_original_replicates_v1";
  workers: number;
  stream_token: "quickpls_indexed_resampling_v1";
  confidence_level: 0.95;
  requested_replicates: number;
  usable_replicates: number;
  minimum_usable_fraction: 0.9;
  seed: number;
  failed_replicates: Array<{ replicate_index: number; reason_code: string; message: string }>;
  jackknife_cases: number;
  usable_jackknife_cases: number;
  validation_witness: RegressionBootstrapValidationWitness;
  coefficients: RegressionBootstrapCoefficient[];
  warnings: string[];
}

export interface RegressionBootstrapValidationWitness {
  method_version: "regression_bootstrap_validation_witness_v1";
  terms: string[];
  successful_bootstrap: Array<{ replicate_index: number; coefficients: number[] }>;
  successful_jackknife: Array<{ omitted_case: number; coefficients: number[] }>;
  failed_jackknife: Array<{ omitted_case: number; reason_code: string; message: string }>;
}

export interface LogisticRegressionDiagnostics {
  outcome_profile: {
    outcome: string;
    coding: string;
    complete_cases: number;
    omitted_cases: number;
    zero_count: number;
    one_count: number;
    invalid_count: number;
    prevalence: number | null;
    readiness: "ready" | "non_binary_values" | "single_observed_class";
  };
  convergence: {
    algorithm: string;
    converged: boolean;
    iterations: number;
    max_iterations: number;
    tolerance: number;
    final_max_abs_step: number;
    separation_probability_tolerance: number;
  };
  classification: {
    threshold: number;
    true_positive: number;
    true_negative: number;
    false_positive: number;
    false_negative: number;
    accuracy: number;
    sensitivity: number;
    specificity: number;
  };
}

export interface NcaAnalysis {
  method_version: string;
  ceiling: string;
  permutation_samples: number;
  usable_permutations: number;
  x: string;
  y: string;
  observations: number;
  scope?: { minimum_x: number; maximum_x: number; minimum_y: number; maximum_y: number };
  ce_fdh_peers?: Array<{ x: number; y: number }>;
  ceilings: Array<{ ceiling: string; effect_size: number; permutation_p_value?: number | null; slope?: number | null; intercept?: number | null }>;
  bottlenecks: Array<{ ceiling?: string; outcome_percent: number; required_x_percent?: number | null; status?: "required" | "not_necessary" | "not_attainable" | string }>;
  warnings: string[];
}

export interface GscaAnalysis {
  method_version: string;
  algorithm?: string;
  converged?: boolean;
  iterations: number;
  stop_criterion?: number;
  final_change?: number;
  objective?: number;
  fit: number;
  measurement_fit?: number;
  structural_fit?: number;
  adjusted_fit: number;
  gfi: number;
  srmr?: number;
  covariance_discrepancy?: number;
  covariance_sample_total?: number;
  standardized_residual_sum?: number;
  observations?: number;
  free_parameters?: number;
  weights: Array<{ construct: string; indicator: string; weight: number; loading: number }>;
  loadings: Array<{ construct: string; indicator: string; weight: number; loading: number }>;
  paths: Array<{ source: string; target: string; coefficient: number }>;
  r_squared: Record<string, number>;
  bootstrap_intervals: Array<{ parameter: string; original: number; lower_percentile: number; upper_percentile: number }>;
  warnings: string[];
}

export interface CbsemAnalysis {
  method_version: string;
  model_type: string;
  estimator: string;
  input: string;
  mean_structure: boolean;
  converged: boolean;
  iterations: number;
  objective: number;
  gradient_norm: number;
  sample_size: number;
  parameters: CbsemParameter[];
  standardized: CbsemStandardizedParameter[];
  implied_covariance: CbsemMatrixCell[];
  residual_covariance: CbsemMatrixCell[];
  residual_correlation: CbsemMatrixCell[];
  fit: CbsemFitIndices;
  modification_indices: CbsemModificationIndex[];
  bootstrap?: CbsemBootstrapAnalysis | null;
  multigroup?: CbsemMultigroupAnalysis | null;
  diagnostics: string[];
  warnings: string[];
}

export interface CbsemParameter {
  name: string;
  kind: string;
  lhs: string;
  rhs: string;
  estimate: number;
  standard_error?: number | null;
  z_statistic?: number | null;
  p_value_two_sided?: number | null;
  fixed: boolean;
  warning?: string | null;
}

export interface CbsemStandardizedParameter {
  name: string;
  kind: string;
  lhs: string;
  rhs: string;
  std_lv: number;
  std_all: number;
}

export interface CbsemMatrixCell {
  row: string;
  column: string;
  value: number;
}

export interface CbsemFitIndices {
  method_version: string;
  chi_square: number;
  degrees_of_freedom: number;
  p_value?: number | null;
  cfi?: number | null;
  tli?: number | null;
  rmsea?: number | null;
  rmsea_ci_lower?: number | null;
  rmsea_ci_upper?: number | null;
  srmr: number;
  aic: number;
  bic: number;
  baseline_chi_square: number;
  baseline_degrees_of_freedom: number;
}

export interface CbsemModificationIndex {
  method_version: string;
  kind: string;
  lhs: string;
  rhs: string;
  modification_index: number;
  expected_parameter_change?: number | null;
}

export interface CbsemBootstrapAnalysis {
  method_version: string;
  samples: number;
  usable_samples: number;
  intervals: Array<{ parameter: string; original: number; lower_percentile: number; upper_percentile: number }>;
  warnings: string[];
}

export interface CbsemMultigroupAnalysis {
  method_version: string;
  group_column: string;
  groups: Array<{ group: string; observations: number; chi_square: number; degrees_of_freedom: number; cfi?: number | null; rmsea?: number | null }>;
  invariance: Array<{ step: string; chi_square: number; degrees_of_freedom: number; delta_chi_square?: number | null; delta_degrees_of_freedom?: number | null; delta_cfi?: number | null; delta_rmsea?: number | null; warning?: string | null }>;
  warnings: string[];
}

export interface PlscAnalysis {
  method_version: string;
  reliability_method_version: string;
  tolerance: number;
  reliabilities: Array<{ construct: string; rho_a: number }>;
  construct_correlations: Array<{ left: string; right: string; original: number; corrected: number }>;
  corrected_paths: Array<{ source: string; target: string; coefficient: number }>;
  corrected_outer_loadings: Array<{ construct: string; indicator: string; weight: number; loading: number }>;
  corrected_r_squared: Record<string, number>;
  warnings: string[];
}

export interface GaussianCopulaEndogeneityAnalysis {
  method_version: string;
  transform: string;
  estimates: Array<{
    source: string;
    target: string;
    path_coefficient: number;
    copula_coefficient: number;
    standard_error: number;
    t_statistic: number;
    p_value_two_sided: number;
    predictor_skewness: number;
    applicable: boolean;
    warning: string | null;
  }>;
  warnings: string[];
}

export interface NonlinearEffectsAnalysis {
  method_version: string;
  term: string;
  estimates: Array<{
    source: string;
    target: string;
    linear_coefficient: number;
    quadratic_coefficient: number;
    standard_error: number;
    t_statistic: number;
    p_value_two_sided: number;
    linear_r_squared: number;
    augmented_r_squared: number;
    delta_r_squared: number;
    warning: string | null;
  }>;
  warnings: string[];
}

export interface ModeratedMediationAnalysis {
  method_version: string;
  moderator_score_levels: number[];
  estimates: Array<{
    interaction: string;
    predictor: string;
    moderator: string;
    mediator: string;
    target: string;
    moderated_stage: string;
    index_of_moderated_mediation: number;
    conditional_indirect_effects: Array<{
      moderator_score: number;
      first_stage_effect: number;
      second_stage_effect: number;
      indirect_effect: number;
    }>;
    warning: string | null;
  }>;
  warnings: string[];
}

export interface CtaPlsAnalysis {
  method_version: string;
  covariance: string;
  estimates: Array<{
    construct: string;
    indicator_a: string;
    indicator_b: string;
    indicator_c: string;
    indicator_d: string;
    pairing: string;
    tetrad: number;
    absolute_tetrad: number;
  }>;
  max_absolute_tetrad_by_construct: Record<string, number>;
  warnings: string[];
}

export interface WplsAnalysis {
  method_version: string;
  case_weight_column: string;
  weight_sum: number;
  effective_sample_size: number;
  covariance: string;
  warnings: string[];
}

export interface CcaAnalysis {
  method_version: string;
  model: string;
  correlations: Array<{
    left: string;
    right: string;
    observed: number;
    reproduced: number;
    residual: number;
    absolute_residual: number;
  }>;
  max_absolute_residual: number;
  warnings: string[];
}

export interface PlsPredictAnalysis {
  method_version: string;
  split: string;
  training_observations: number;
  test_observations: number;
  benchmark: string;
  targets: PlsPredictTarget[];
  indicator_targets?: PlsPredictIndicatorTarget[];
  repeated_kfold?: {
    method_version: string;
    folds: number;
    repeats: number;
    assignment: string;
    assignment_digest?: string;
    seed?: number;
    total_test_observations: number;
    targets: PlsPredictTarget[];
    indicator_targets?: PlsPredictIndicatorTarget[];
    cvpat_benchmark_assessments?: CvpatBenchmarkAssessment[];
    paired_loss_diagnostics?: CvpatComparison[];
    /** Archive-compatible v1 field. Current v2 results use cvpat_benchmark_assessments. */
    cvpat?: CvpatComparison[];
    warnings: string[];
  } | null;
  warnings: string[];
}

export interface PlsPredictTarget {
  construct: string;
  predictor_count: number;
  rmse_pls: number;
  mae_pls: number;
  rmse_benchmark: number;
  mae_benchmark: number;
  q_squared_predict: number | null;
  rmse_lm?: number | null;
  mae_lm?: number | null;
  q_squared_predict_lm?: number | null;
}

export interface PlsPredictErrorMetrics {
  observations: number;
  squared_error_sum: number;
  absolute_error_sum: number;
  rmse: number;
  mae: number;
  absolute_percentage_error_sum: number | null;
  mape_observations: number;
  mape_percent: number | null;
}

export interface PlsPredictLinearModelBenchmark {
  status: "available" | "unavailable";
  metrics?: PlsPredictErrorMetrics | null;
  reason?: string | null;
}

export interface PlsPredictIndicatorTarget {
  construct: string;
  indicator: string;
  predictor_scope: string;
  predictor_count: number;
  pls: PlsPredictErrorMetrics;
  indicator_average: PlsPredictErrorMetrics;
  linear_model: PlsPredictLinearModelBenchmark;
  q_squared_predict: number | null;
}

export interface CvpatComparison {
  target: string;
  comparison: string;
  loss: string;
  mean_loss_difference: number;
  standard_error: number | null;
  t_statistic: number | null;
  p_value_two_sided: number | null;
  observations: number;
  preferred_model: string;
  warning: string | null;
}

export interface CvpatBenchmarkAssessment {
  method_version: string;
  comparison_kind: "benchmark_assessment";
  target_scope: "all_endogenous_indicators" | string;
  benchmark: "indicator_average" | "linear_model" | string;
  loss: "mean_squared_error_across_indicators_per_observation" | string;
  alternative: "pls_loss_less_than_benchmark" | string;
  confidence_level: number;
  mean_loss_pls: number | null;
  mean_loss_benchmark: number | null;
  mean_loss_difference: number | null;
  standard_error: number | null;
  t_statistic: number | null;
  p_value_one_sided: number | null;
  confidence_interval_lower: number | null;
  confidence_interval_upper: number | null;
  observations: number;
  indicator_count: number;
  status: "available" | "inferential_test_unavailable" | "benchmark_unavailable";
  preferred_model: "pls_sem" | null;
  reason: string | null;
}

export interface PlsSegmentationAnalysis {
  method_version: string;
  algorithm: string;
  requested_segments: number;
  selected_segments: number;
  assignment: string;
  observations: number;
  objective: number;
  pooled_objective: number;
  objective_improvement: number;
  min_segment_share: number;
  segment_size_imbalance: number;
  max_path_separation: number;
  segments: Array<{
    segment: string;
    observations: number;
    share: number;
    paths: Array<{ source: string; target: string; coefficient: number }>;
    r_squared: Record<string, number>;
  }>;
  memberships?: Array<{ observation: number; segment: string }>;
  objective_history?: Array<{ start: number; iteration: number; objective: number }>;
  warnings: string[];
}

export interface PlsMgaAnalysis {
  method_version: string;
  group_column: string;
  groups: Array<{
    group: string;
    observations: number;
    paths: Array<{ source: string; target: string; coefficient: number }>;
    r_squared: Record<string, number>;
    outer_estimates?: Array<{ construct: string; indicator: string; weight: number; loading: number }>;
    transforms?: Array<{ indicator: string; mean: number; scale: number }>;
  }>;
  comparisons: Array<{
    source: string;
    target: string;
    group_a: string;
    group_b: string;
    coefficient_a: number;
    coefficient_b: number;
    difference: number;
    standard_error: number | null;
    t_statistic: number | null;
    p_value_two_sided: number | null;
    warning: string | null;
  }>;
  measurement_comparisons?: Array<{
    parameter: "outer_loading" | "outer_weight" | string;
    construct: string;
    indicator: string;
    group_a: string;
    group_b: string;
    estimate_a: number;
    estimate_b: number;
    difference: number;
  }>;
  warnings: string[];
}

export interface MicomAnalysis {
  method_version: string;
  group_column: string;
  permutation_samples: number;
  usable_permutations: number;
  attempted_permutations?: number | null;
  failed_permutations?: number | null;
  confidence_level?: number | null;
  groups: Array<{ group: string; observations: number }>;
  constructs: Array<{
    construct: string;
    configural_invariance: boolean;
    compositional_correlation: number;
    compositional_p_value: number | null;
    compositional_correlation_lower?: number | null;
    mean_a?: number | null;
    mean_b?: number | null;
    mean_difference: number;
    mean_p_value: number | null;
    mean_difference_lower?: number | null;
    mean_difference_upper?: number | null;
    variance_a?: number | null;
    variance_b?: number | null;
    variance_difference: number;
    variance_p_value: number | null;
    variance_difference_lower?: number | null;
    variance_difference_upper?: number | null;
    equal_means?: boolean | null;
    equal_variances?: boolean | null;
    partial_invariance: boolean;
    full_invariance: boolean;
  }>;
  warnings: string[];
}

export interface PlsMgaPermutationAnalysis {
  method_version: string;
  group_column: string;
  permutation_samples: number;
  usable_permutations: number;
  attempted_permutations?: number | null;
  failed_permutations?: number | null;
  comparisons: Array<{
    source: string;
    target: string;
    original_difference: number;
    empirical_p_value_two_sided: number | null;
    percentile_rank: number | null;
  }>;
  measurement_comparisons?: Array<{
    parameter: "outer_loading" | "outer_weight" | string;
    construct: string;
    indicator: string;
    original_difference: number;
    empirical_p_value_two_sided: number | null;
    percentile_rank: number | null;
  }>;
  warnings: string[];
}

export interface FimixPlsAnalysis {
  method_version: string;
  classes: number;
  starts: number;
  iterations: number;
  log_likelihood: number;
  aic: number;
  bic: number;
  caic: number;
  entropy: number;
  classes_summary: Array<{
    class: string;
    observations: number;
    share: number;
    paths: Array<{ source: string; target: string; coefficient: number }>;
    r_squared: Record<string, number>;
  }>;
  memberships: Array<{ observation: number; class: string; probability: number }>;
  warnings: string[];
}

export interface IpmaAnalysis {
  method_version: string;
  performance_scale: string;
  targets: string[];
  constructs: Array<{
    target: string;
    construct: string;
    importance: number;
    performance: number;
    score_mean: number;
  }>;
  indicators: Array<{
    target: string;
    construct: string;
    indicator: string;
    construct_importance: number;
    loading: number;
    performance: number;
    score_mean: number;
  }>;
  warnings: string[];
}

export interface MediationAnalysis {
  method_version: string;
  tolerance: number;
  estimates: Array<{
    source: string;
    target: string;
    direct: number;
    indirect: number;
    total: number;
    variance_accounted_for: number | null;
    classification: "no_effect" | "direct_only" | "indirect_only" | "complementary_partial" | "competitive_partial";
    warning: string | null;
  }>;
  warnings: string[];
}

export interface ModerationAnalysis {
  method_version: string;
  moderator_score_levels: number[];
  estimates: Array<{
    interaction: string;
    predictor: string;
    moderator: string;
    product_construct: string;
    outcome: string;
    predictor_main_effect: number | null;
    moderator_main_effect: number | null;
    interaction_effect: number;
    simple_slopes: Array<{ moderator_score: number; effect: number }>;
    warning: string | null;
  }>;
  warnings: string[];
}

export interface AssessmentResult {
  method_version: string;
  rho_a_method_version?: string | null;
  construct_quality: Array<{
    construct: string;
    cronbach_alpha: number | null;
    rho_c: number | null;
    ave: number | null;
    rho_a?: number | null;
    rho_a_status?: "available" | "not_applicable" | "unavailable" | null;
    rho_a_reason?: string | null;
    rho_a_warning_codes?: string[];
    rho_a_indicator_count?: number | null;
    score_variance_before_normalization?: number | null;
    normalized_weight_norm_squared?: number | null;
    off_diagonal_numerator?: number | null;
    off_diagonal_denominator?: number | null;
  }>;
  cross_loadings: Array<{
    indicator: string;
    assigned_construct: string;
    construct: string;
    loading: number;
  }>;
  fornell_larcker: { constructs: string[]; values: Array<Array<number | null>> };
  htmt?: { constructs: string[]; values: Array<Array<number | null>> };
  htmt_plus_method_version?: string | null;
  htmt_plus?: HtmtAssessment | null;
  htmt_original_method_version?: string | null;
  htmt_original?: HtmtAssessment | null;
  r_squared: Record<string, number>;
  structural_quality: Array<{ construct: string; predictor_count: number; r_squared: number; adjusted_r_squared: number | null }>;
  structural_vif: Array<{ target_construct: string; predictor_construct: string; vif: number | null }>;
  formative_indicator_vif: Array<{ construct: string; indicator: string; vif: number | null }>;
  f_squared: Array<{ source_construct: string; target_construct: string; included_r_squared: number; excluded_r_squared: number | null; f_squared: number | null }>;
  model_fit?: { saturated: { srmr: number; d_uls: number }; estimated: { srmr: number; d_uls: number } };
  blindfolding?: {
    settings: { omission_distance: number; selection: string; missing_value_treatment: string };
    constructs: Array<{ construct: string; q_squared: number | null; prediction_error_sum_squares: number | null; observation_sum_squares: number | null }>;
  };
  warnings: string[];
}

export interface HtmtAssessment {
  constructs: string[];
  correlation_type: "pearson";
  absolute_correlations: boolean;
  cells: Array<Array<{
    value: number | null;
    status: "available" | "not_applicable" | "unavailable";
    reason: string | null;
  }>>;
}

export interface PlsBootstrapRun {
  method_version: string;
  plan: { replicates: number; master_seed: number; operation: string };
  usable_replicates: number;
  failed_replicates: Array<{ replicate_index: number; message: string }>;
  percentile: {
    confidence_level: number;
    parameters: Array<{ parameter: string; original: number; bootstrap_mean: number; bias: number; standard_error: number; lower: number; upper: number; usable_replicates: number; t_statistic?: number | null; p_value_two_sided?: number | null }>;
  };
  bca?: {
    confidence_level: number;
    jackknife_case_count: number;
    parameters: Array<{ parameter: string; bias_correction: number | null; acceleration: number | null; lower: number | null; upper: number | null; unavailable_reason: string | null }>;
  } | null;
  studentized?: {
    method_version: string;
    confidence_level: number;
    inner_replicates: number;
    minimum_usable_fraction: number;
    stream_domain: string;
    failure?: { reason_code: string; first_primary_replicate: number; failed_primary_replicates: number; message: string } | null;
    parameters: Array<{ parameter: string; original: number; outer_standard_error: number; outer_scale: number; usable_primary_replicates: number; lower_pivot: number | null; upper_pivot: number | null; lower: number | null; upper: number | null; unavailable_reason: string | null }>;
  } | null;
}

export interface PlsPermutationRun {
  method_version: string;
  plan: { permutations: number; master_seed: number; operation: string };
  parameters: Array<{ parameter: string; original: number; exceedances: number; p_value_two_sided: number; permutations: number }>;
}

export interface AnalysisEngineSettingsSnapshot {
  method: AnalysisMethodId;
  weighting_scheme: "path" | "factor" | "pca";
  tolerance: number;
  max_iterations: number;
  bootstrap_samples: number;
  studentized_inner_samples: number;
  permutation_samples: number;
  seed: number;
  workers: number;
  confidence_level: number;
  preprocessing: "standardized" | "mean_centered" | "unstandardized";
  missing_data: "listwise_deletion";
  case_weight_column: string | null;
}

export interface AnalysisResultProvenance {
  recipe_id: string;
  dataset_fingerprint: string;
  method: AnalysisMethodId;
  method_version: string;
  engine_version: string;
  seed: number;
  settings: AnalysisEngineSettingsSnapshot;
  started_at: string;
  completed_at: string;
}

export interface AnalysisResultEnvelope {
  schema_version: number;
  id: string;
  status: "completed" | "failed";
  provenance: AnalysisResultProvenance;
  diagnostics: Array<{ code: string; level: "information" | "warning" | "error"; message: string }>;
  payload:
    | { kind: "pls_pm_v1"; estimation: PlsResult; assessment: AssessmentResult }
    | { kind: "pls_pm_v2"; estimation: PlsResult; assessment: AssessmentResult; bootstrap: PlsBootstrapRun }
    | { kind: "pls_pm_v3"; estimation: PlsResult; assessment: AssessmentResult; bootstrap?: PlsBootstrapRun | null; permutation?: PlsPermutationRun | null }
    | { kind: "legacy"; value: unknown };
}

export interface JobSnapshot {
  id: string;
  state: "queued" | "running" | "cancelling" | "committing" | "completed" | "failed" | "cancelled";
  phase: string;
  completed_units: number;
  total_units: number;
  message: string | null;
}

export interface MethodDefinition {
  id: string;
  family: string;
  name: string;
  status: MethodStatus;
}
