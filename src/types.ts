import type { Edge, Node } from "@xyflow/react";
import type { NativeSampleProjectId } from "./domain/bundledSampleCatalog";

export type { NativeSampleProjectId } from "./domain/bundledSampleCatalog";

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

export type AnalysisMethodId = "pls_pm" | "bootstrap" | "permutation" | "pls_sample_size_power" | "plsc" | "wpls" | "cca" | "cta_pls" | "endogeneity" | "nonlinear_effects" | "moderated_mediation" | "predict" | "mga" | "ipma" | "cbsem" | "pca" | "gsca" | "regression" | "nca";
export type DiagramMode = "compact" | "sem" | "publication" | "smartpls_result";
export type DiagramOverlayMode = "model" | "loadings" | "paths_r2" | "significance" | "quality" | "cbsem_standardized" | "cbsem_residuals" | "modification_indices";
export type DiagramToolMode = "select" | "pan" | "construct" | "indicator" | "path" | "covariance" | "residual" | "caption" | "measurement" | "interaction" | "higher_order";
export type IndicatorSide = "left" | "right" | "top" | "bottom" | "free";
export type EdgeRouteStyle = "straight" | "curved" | "orthogonal" | "polyline";

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

/** Presentation-only routing for a generated indicator-to-construct connector. */
export interface MeasurementConnectorLayout {
  routing: EdgeRouteStyle;
  bendPoints?: DiagramPoint[];
}

export interface DiagramViewport {
  x: number;
  y: number;
  zoom: number;
}

export type StandardSemPresentationLayoutObject =
  | { kind: "caption"; id: string; text: string; x: number; y: number }
  | { kind: "note"; id: string; subject: string; text: string; x: number; y: number }
  | { kind: "shape"; id: string; shape: "rectangle" | "rounded_rectangle" | "ellipse" | "diamond"; x: number; y: number; width: number; height: number; label: string | null; style: Record<string, string> }
  | { kind: "image"; id: string; assetRef: string; altText: string; x: number; y: number; width: number; height: number; style: Record<string, string> }
  | { kind: "line"; id: string; x1: number; y1: number; x2: number; y2: number; label: string | null; startMarker: string | null; endMarker: string | null; style: Record<string, string> };

export interface StandardSemPresentationLayoutV1 {
  schemaVersion: 1;
  objects: StandardSemPresentationLayoutObject[];
}

export interface DiagramLayoutState {
  diagramVersion: "sem_designer_v1";
  constructLayouts: Record<string, ConstructLayout>;
  indicatorLayouts: Record<string, Record<string, IndicatorLayout>>;
  edgeLayouts: Record<string, EdgeLayout>;
  measurementConnectorLayouts: Record<string, Record<string, MeasurementConnectorLayout>>;
  diagramViewport?: DiagramViewport;
  diagramTheme: "academic_grayscale" | "smartpls_like" | "quickpls_color" | "journal_mono" | "high_contrast";
  showGrid: boolean;
  layoutLocked: boolean;
  /** Presentation-only SemModelV4 decorations; never scientific authority. */
  standardSemPresentation?: StandardSemPresentationLayoutV1;
  /** Optional visual positions for path-mounted moderation anchors, keyed by stable interaction term ID. */
  moderationAnchorFractions?: Record<string, number>;
  /** Optional presentation-only routing points for moderator connectors, keyed by visual connector ID. */
  moderationConnectorBendPoints?: Record<string, Array<{ x: number; y: number }>>;
}

export type ModelEditHigherOrderApproachV1 =
  | "repeated_indicators"
  | "extended_repeated_indicators"
  | "embedded_two_stage"
  | "disjoint_two_stage";

export type ModelEditHigherOrderMeasurementTypeV1 =
  | "reflective_reflective"
  | "reflective_formative"
  | "formative_reflective"
  | "formative_formative";

/** Authority-neutral HOC input. IDs remain outside the draft so edits cannot replace them. */
export interface ModelEditHigherOrderDraftV1 {
  name: string;
  shortName: string;
  components: string[];
  approach: ModelEditHigherOrderApproachV1;
  measurementType: ModelEditHigherOrderMeasurementTypeV1;
  initialPath?: {
    direction: "hoc_to_construct" | "construct_to_hoc";
    constructId: string;
    relationshipId: string;
    label?: string;
  };
}

export type ModelEditModeratingEffectTargetV1 =
  | { kind: "focal_relation"; relationId: string }
  | { kind: "parent_interaction"; interactionTermId: string };

export interface ModelEditModeratingEffectSpecV1 {
  label: string;
  operands: [predictor: string, moderator: string] | [predictor: string, firstModerator: string, secondModerator: string];
  target: ModelEditModeratingEffectTargetV1;
  outcomeId: string;
}

/**
 * One authority-aware model edit accepted by the native workbench gateway.
 * Scientific edits change the model authority; presentation edits only change
 * the stable diagram layout bound to that authority.
 */
export type ModelEditCommandV1 =
  | { kind: "add_construct"; constructId: string; label: string; columns?: string[]; position?: DiagramPoint }
  | { kind: "rename_construct"; constructId: string; label: string }
  | { kind: "invert_measurement_model"; constructId: string }
  | { kind: "assign_indicators"; constructId: string; columns: string[] }
  | { kind: "unassign_indicator"; constructId: string; column: string; replacementMarkerColumn?: string | null }
  | { kind: "add_path"; relationId: string; sourceId: string; targetId: string; label?: string }
  | { kind: "reverse_path"; relationId: string }
  | { kind: "remove_path"; relationId: string }
  | { kind: "create_higher_order"; termId: string; outputId: string; draft: ModelEditHigherOrderDraftV1 }
  | { kind: "edit_higher_order"; termId: string; outputId: string; draft: Omit<ModelEditHigherOrderDraftV1, "initialPath"> }
  | { kind: "remove_higher_order"; termId: string; outputId: string }
  | { kind: "create_moderating_effect"; effect: ModelEditModeratingEffectSpecV1 }
  | { kind: "edit_moderating_effect"; termId: string; outputId: string; effect: ModelEditModeratingEffectSpecV1 }
  | { kind: "remove_moderating_effect"; termId: string; outputId: string }
  | { kind: "move_construct"; constructId: string; position: DiagramPoint }
  | { kind: "set_construct_indicator_side"; constructId: string; side: Exclude<IndicatorSide, "free"> }
  | { kind: "set_indicator_side"; constructId: string; column: string; side: IndicatorSide }
  | { kind: "move_indicator"; constructId: string; column: string; position: DiagramPoint }
  | { kind: "reset_indicator_layout"; constructId: string; column?: string }
  | { kind: "set_path_routing"; relationId: string; routing: EdgeRouteStyle }
  | { kind: "set_path_bend_points"; relationId: string; points: DiagramPoint[] }
  | { kind: "reset_path_route"; relationId: string }
  | { kind: "nudge_path_label"; relationId: string; offset: DiagramPoint }
  | { kind: "reset_path_label"; relationId: string }
  | { kind: "set_measurement_connector_routing"; constructId: string; column?: string; routing: EdgeRouteStyle }
  | { kind: "set_measurement_connector_bend_points"; constructId: string; column: string; points: DiagramPoint[] }
  | { kind: "reset_measurement_connector_route"; constructId: string; column?: string }
  | { kind: "set_moderation_anchor_fraction"; interactionTermId: string; fraction: number }
  | { kind: "set_standard_sem_presentation"; presentation: StandardSemPresentationLayoutV1 }
  | { kind: "set_construct_pinned"; constructId: string; pinned: boolean }
  | { kind: "align_constructs"; constructIds: string[]; target: "left" | "centerX" | "right" | "top" | "centerY" | "bottom" }
  | { kind: "distribute_constructs"; constructIds: string[]; axis: "horizontal" | "vertical" }
  | { kind: "tidy_constructs"; constructIds: string[] }
  | { kind: "arrange_model"; direction: "horizontal" | "vertical" | "smartpls" };

export type ModelEditTransactionClassV1 = "scientific" | "presentation";
export type ModelEditAuthorityKindV1 = "legacy_graph" | "standard_sem_model_v4";

export interface ModelEditAffectedIdentitiesV1 {
  constructIds: string[];
  indicatorIds: string[];
  relationshipIds: string[];
}

export type ModelEditCommandResultV1 =
  | {
      status: "applied";
      command: ModelEditCommandV1["kind"];
      transaction: ModelEditTransactionClassV1;
      authority: ModelEditAuthorityKindV1;
      modelId: string | null;
      affected: ModelEditAffectedIdentitiesV1;
      undoable: true;
      stableIdsPreserved: true;
    }
  | {
      status: "blocked";
      command: ModelEditCommandV1["kind"];
      transaction: ModelEditTransactionClassV1;
      authority: ModelEditAuthorityKindV1;
      modelId: string | null;
      code: string;
      message: string;
      correctiveAction: string;
    };

export interface UiPreferences {
  density: UiDensity;
  tableDensity: UiDensity;
  defaultPrecision: number;
  showAdvancedHelp: boolean;
  experimentalLabsEnabled: boolean;
  /** Expert presentation preference; generated interaction terms remain hidden by default. */
  showGeneratedInteractionTerms: boolean;
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
  /** Omitted for the historical/default two-sided PLS bootstrap contract. */
  bootstrapTestTail?: PlsBootstrapTestTail;
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
  /** Exact CB-SEM bootstrap interval; omitted preserves the historical percentile wire. */
  cbsemBootstrapInterval?: "percentile_type7" | "analytic_studentized_type7" | "bca_type7";
  /** Omitted for the historical/default two-sided exact CB-SEM bootstrap test. */
  cbsemBootstrapTestTail?: CbsemBootstrapTestTail;
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
  plsPowerScenarioIdentity?: string | null;
  plsPowerPredictorConstruct?: string | null;
  plsPowerOutcomeConstruct?: string | null;
  plsPowerPredictorLoadings?: string | null;
  plsPowerOutcomeLoadings?: string | null;
  plsPowerPopulationPath?: number;
  plsPowerSampleSizeGrid?: string | null;
  plsPowerAlpha?: number;
  plsPowerTargetPower?: number;
  plsPowerMonteCarloReplicates?: number;
  plsPowerBootstrapReplicates?: number;
  /** Product-only selector for the versioned scoped Standard post-hoc add-on. */
  posthocTechnicalMinimumSampleSize?: boolean;
}

export interface ConstructData {
  [key: string]: unknown;
  label: string;
  shortName: string;
  mode: MeasurementMode;
  indicators: string[];
  semantic?: "interaction" | "higher_order" | "polynomial" | "observed";
  interaction?: InteractionData;
  higherOrder?: HigherOrderConstructData;
  polynomial?: PolynomialConstructData;
  /** Read-only projection metadata from the sole canonical Standard SemModelV4 authority. */
  standardSemV4Authority?: StandardSemV4NodeAuthorityData;
  /** Persisted factor-versus-composite authoring intent; absent means legacy unspecified. */
  semModelV4?: SemModelV4ConstructAuthoringState;
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

interface InteractionDataBase {
  termId?: string;
  outcome: string;
  focalRelationId?: string;
  canonicalMethod?: "two_stage" | "product_indicator" | "orthogonalizing";
  productIndicator?: {
    centering: "none" | "mean_center" | "double_mean_center";
    standardization: "none" | "sample_standard_deviation";
    pairing: "all_pairs";
  } | null;
}

/** Historical two-way canvas interaction. An omitted kind preserves legacy serialized bytes. */
export interface LegacyInteractionData extends InteractionDataBase {
  kind?: "interaction";
  predictor: string;
  moderator: string;
  /** Legacy canvas and native-recipe compatibility token. */
  method: "two_stage_product_score";
}

/** Lossless readback metadata for ordered SemModelV4 interaction_v2 terms. */
export interface InteractionV2Data extends InteractionDataBase {
  kind: "interaction_v2";
  termId: string;
  /** operands[0] is the focal predictor; remaining operands are moderators in authored order. */
  operands: [string, string, ...string[]];
  focalRelationId: string;
  canonicalMethod: "two_stage" | "product_indicator" | "orthogonalizing";
  hierarchyPolicy: "strong" | "weak" | "none";
}

export type InteractionData = LegacyInteractionData | InteractionV2Data;

export interface HigherOrderConstructData {
  id: string;
  components: string[];
  method: "repeated_indicators" | "two_stage" | "hybrid";
  canonicalApproach?: "repeated_indicators" | "extended_repeated_indicators" | "embedded_two_stage" | "disjoint_two_stage" | "hybrid";
  measurementType?: "reflective_reflective" | "reflective_formative" | "formative_reflective" | "formative_formative";
  stage_one_recipe?: string | null;
}

export interface PolynomialConstructData {
  termId: string;
  source: string;
  degree: number;
}

export interface StandardSemV4MeasurementBindingData {
  relationId: string;
  parameterId: string;
  observedId: string;
  sourceColumn: string;
  relationKind: "measurement_effect" | "measurement_causal";
}

export interface StandardSemV4NodeAuthorityData {
  variableId: string;
  variableKind: "observed" | "common_factor" | "composite" | "derived";
  readOnly: true;
  observedRole?: "indicator" | "structural" | "both" | "control";
  measurementBindings: StandardSemV4MeasurementBindingData[];
}

export interface ControlData {
  source: string;
  target: string;
  label?: string | null;
}

export type SemModelV4AuthoringEndpoint =
  | { kind: "variable"; id: string }
  | { kind: "residual_of"; id: string }
  | { kind: "disturbance_of"; id: string };

export type SemModelV4ConstructAuthoring =
  | { kind: "composite" }
  | { kind: "common_factor"; marker_indicator: string | null }
  | { kind: "legacy_estimand_unspecified" };

export type SemModelV4FactorIdentificationAuthoring =
  | { kind: "marker_loading"; indicator: string }
  | { kind: "fixed_variance" }
  | { kind: "effects_coding" };

export type SemModelV4ParameterAuthoringTarget =
  | { kind: "loading"; construct: string; indicator: string }
  | { kind: "weight"; indicator: string; composite: string }
  | { kind: "regression"; source: string; target: string }
  | { kind: "variance"; endpoint: SemModelV4AuthoringEndpoint }
  | { kind: "covariance"; left: SemModelV4AuthoringEndpoint; right: SemModelV4AuthoringEndpoint }
  | { kind: "intercept"; variable: string }
  | { kind: "mean"; variable: string }
  | { kind: "threshold"; variable: string; index: number };

export type SemModelV4ParameterAuthoringSpecification =
  | {
    kind: "free";
    start: number | null;
    lower: number | null;
    upper: number | null;
    equality_label: string | null;
  }
  | { kind: "fixed"; value: number };

export interface SemModelV4ParameterAuthoringEntry {
  parameter_id: string;
  target: SemModelV4ParameterAuthoringTarget;
  specification: SemModelV4ParameterAuthoringSpecification;
}

export interface SemModelV4ConstructAuthoringState {
  version: 1;
  construct: SemModelV4ConstructAuthoring;
  identification?: SemModelV4FactorIdentificationAuthoring;
  parameters?: SemModelV4ParameterAuthoringEntry[];
}

export interface SemModelV4CovarianceAuthoringState {
  version: 1;
  covariance: SemModelV4CovarianceAuthoring;
  parameters?: SemModelV4ParameterAuthoringEntry[];
}

export interface SemModelV4RelationshipParameterAuthoringState {
  version: 1;
  parameters: SemModelV4ParameterAuthoringEntry[];
}

export type SemModelV4CovarianceAuthoring =
  | {
    kind: "scientific";
    origin: "new_authoring" | "explicit_conversion";
    left: SemModelV4AuthoringEndpoint | null;
    right: SemModelV4AuthoringEndpoint | null;
  }
  | {
    kind: "presentation_only";
    origin: "explicit_conversion" | "legacy_migration";
  }
  | {
    kind: "legacy_unspecified";
    origin: "legacy_archive" | "role_conversion";
  };

export interface PathEdgeData {
  role?: "control" | "covariance";
  controlLabel?: string | null;
  /** Presentation hint for strong-hierarchy relations generated by moderation authoring. */
  technicalGenerated?: boolean;
  /** Persisted editor intent for the dormant SemModelV4 execution path. */
  semModelV4?: SemModelV4CovarianceAuthoringState;
  /** Persisted parameter edits for non-covariance scientific relationships. */
  semModelV4ParameterAuthoring?: SemModelV4RelationshipParameterAuthoringState;
  /** Read-only source identity retained by the Standard SemModelV4 projection. */
  standardSemV4Authority?: StandardSemV4EdgeAuthorityData;
}

export interface StandardSemV4EdgeAuthorityData {
  authorityObjectId: string;
  relationKind: "structural" | "covariance" | "display_only_covariance";
  parameterId: string | null;
  leftEndpoint?: SemModelV4AuthoringEndpoint;
  rightEndpoint?: SemModelV4AuthoringEndpoint;
  presentationOnly: boolean;
  readOnly: true;
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

export type DatasetVersionOperation = "import" | "metadata" | "recode" | "transform";

export interface DatasetTransformationLineageRecordV2 {
  schema_version: 2;
  engine: "qpls.dataset_transform.v2";
  operation_id: string;
  source_dataset_id: string;
  source_dataset_fingerprint: string;
  output_dataset_id: string;
  output_dataset_fingerprint: string;
  created_at: string;
  spec_sha256: string;
  spec: import("./domain/datasetTransformationsV2").DatasetTransformationSpecV2;
  input_columns: string[];
  output_columns: string[];
  source_row_count: number;
  output_missing_count: number;
}

export interface DatasetVersionRecord {
  datasetId: string;
  parentDatasetId: string | null;
  operation: DatasetVersionOperation;
  createdAt: string | null;
  summary: string;
  sourceColumn: string | null;
  targetColumn: string | null;
  transformation?: DatasetTransformationLineageRecordV2;
}

export interface ProjectDataLineageV1 {
  schemaVersion: 1;
  records: DatasetVersionRecord[];
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

/**
 * Transient authority for a freshly created, unsaved General SEM project.
 * It is never serialized into the historical desktop project format.
 */
export interface GeneralSemProjectDraftModeV1 {
  schemaVersion: 1;
  semGeneration: "general_sem_v1";
  sourceProjectId: string;
}

export interface NativeProjectSnapshot {
  projectId: string;
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

/** Presentation plus dormant, versioned SemModelV4 authoring intent for one editable model. */
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

export type PlscPermutationTestTail = "two_sided" | "group_a_greater" | "group_a_less";

export type NativeAnalysisMethodConfig =
  | { kind: "pls_algorithm" }
  | {
      kind: "pls_algorithm_configured_v2";
      initialization_contract_version: "pls_initial_outer_weights_v2";
      initial_outer_weights:
        | { kind: "standard" }
        | {
            kind: "individual";
            weights: Array<{ construct_id: string; indicator_id: string; value: number }>;
          };
    }
  | { kind: "pls_bootstrap" }
  | { kind: "pls_permutation" }
  | ({
      kind: "pls_posthoc_technical_minimum_sample_size";
      capability_cell: {
        registry_schema_version: 2;
        capability_id: "smartpls.pls_power_analysis";
        cell_id: "qpls3.pls.posthoc_technical_minimum_sample_size";
        capability_version: "pls_posthoc_technical_minimum_sample_size_v2";
      };
      method_version: "inverse_square_root_posthoc_v2";
    } & (
      | { base_analysis: "pls_algorithm"; inference: "point_estimate_only" }
      | { base_analysis: "pls_bootstrap"; inference: "case_bootstrap_normal_reference_two_sided" }
    ))
  | {
      kind: "pls_sample_size_power";
      scenario_identity: string;
      predictor_construct: string;
      outcome_construct: string;
      predictor_indicator_loadings: number[];
      outcome_indicator_loadings: number[];
      population_path: number;
      exogenous_distribution: "standard_normal";
      structural_disturbance_distribution: "standard_normal";
      indicator_error_distribution: "standard_normal";
      missing_data: "none";
      inference:
        | "case_bootstrap_normal_reference_two_sided"
        | "case_bootstrap_null_centered_two_sided_plus_one";
      sample_size_grid: number[];
      alpha: number;
      target_power: number;
      interval_confidence_level: number;
      monte_carlo_replicates: number;
      bootstrap_replicates: number;
    }
  | { kind: "plsc" }
  | {
      kind: "plsc_permutation";
      group_column: string;
      group_a: string;
      group_b: string;
      /** Omitted for the historical/default two-sided selection. */
      test_tail?: PlscPermutationTestTail;
    }
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
  | {
      kind: "micom";
      group_column: string;
      group_a: string;
      group_b: string;
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
      bootstrap_v2?: NativeCbsemBootstrapConfigV2;
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

export interface NativeCbsemBootstrapConfigV2 {
  algorithm: "case_resampling_full_ml";
  interval: "percentile_type7" | "analytic_studentized_type7" | "bca_type7";
  /** Omitted for the historical/default two-sided exact CB-SEM bootstrap test. */
  test_tail?: Exclude<CbsemBootstrapTestTail, "two_sided">;
}

export type CbsemBootstrapTestTail = "two_sided" | "one_sided_greater" | "one_sided_less";

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
  plsSampleSizePower?: PlsSampleSizePowerResultV1 | PlsSampleSizePowerResultV2;
  plsSampleSizePowerRecipe?: PlsSampleSizePowerRecipeV1 | PlsSampleSizePowerRecipeV2;
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
  score_execution?: PlsResolvedScoreExecutionV2;
  fixed_score_scale_receipt?: PlsFixedScoreScaleReceiptV1;
  point_estimate_attribution?: PlsPointEstimateAttributionV1;
  algorithm_convergence_receipt?: PlsAlgorithmConvergenceReceiptV1;
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
  posthoc_minimum_sample_size?: PlsPosthocMinimumSampleSize | null;
  r_squared: Record<string, number>;
  warnings: string[];
}

export type PlsPointEstimateScaleV1 =
  | "sample_mean"
  | "no_centering"
  | "sample_standard_deviation"
  | "unit_scale"
  | "preprocessed_indicator_to_unit_variance_construct_score"
  | "indicator_construct_score_correlation"
  | "zero_mean_unit_variance_construct_score"
  | "standardized_construct_score_regression"
  | "standardized_structural_path_decomposition";

export interface PlsPointEstimateAttributionV1 {
  contract_version: "pls_point_estimate_attribution_v1";
  preprocessing: "standardized" | "mean_centered" | "unstandardized";
  indicator_centering: "sample_mean" | "no_centering";
  indicator_scaling: "sample_standard_deviation" | "unit_scale";
  outer_weights: "preprocessed_indicator_to_unit_variance_construct_score";
  outer_loadings: "indicator_construct_score_correlation";
  construct_scores: "zero_mean_unit_variance_construct_score";
  structural_paths: "standardized_construct_score_regression";
  effects: "standardized_structural_path_decomposition";
}

export interface PlsFixedScoreBlockScaleReceiptV1 {
  construct_id: string;
  indicator_ids: string[];
  pre_standardization_center: number;
  pre_standardization_scale: number;
  effective_unit_score_weights: PlsResolvedScoreWeightV2[];
}

export interface PlsFixedScoreScaleReceiptV1 {
  contract_version: "pls_fixed_score_scale_receipt_v1";
  blocks: PlsFixedScoreBlockScaleReceiptV1[];
}

export interface PlsAlgorithmBlockReceiptV1 {
  construct_id: string;
  indicator_order: string[];
  update_rule: "mode_a_covariance" | "mode_b_ols" | "fixed_no_update";
  initialization:
    | "standard_unit_weights"
    | "individual_requested_weights"
    | "fixed_unit_weights"
    | "fixed_custom_weights";
}

export interface PlsAlgorithmConvergenceReceiptV1 {
  contract_version: "pls_algorithm_convergence_receipt_v1";
  weighting_scheme: "path" | "factor";
  maximum_iterations: number;
  stop_criterion: number;
  comparison: "less_than_or_equal";
  performed_iterations: number;
  estimated_block_updates: number;
  termination_reason: "converged_tolerance" | "all_blocks_fixed";
  final_max_outer_weight_change?: number | null;
  blocks: PlsAlgorithmBlockReceiptV1[];
}

export interface PlsResolvedScoreWeightV2 {
  indicator_id: string;
  value: number;
}

export type PlsResolvedInitialOuterWeightsV2 =
  | { kind: "standard"; weights: PlsResolvedScoreWeightV2[] }
  | { kind: "individual"; weights: PlsResolvedScoreWeightV2[] };

export type PlsResolvedScoreBlockKindV2 =
  | {
      kind: "estimated";
      mode: "mode_a" | "mode_b";
      requested_initialization: PlsResolvedInitialOuterWeightsV2;
      resolved_initial_weights: PlsResolvedScoreWeightV2[];
    }
  | {
      kind: "fixed_unit";
      normalization: "none" | "sum_to_one" | "unit_variance";
      requested_weights: PlsResolvedScoreWeightV2[];
      resolved_effective_weights: PlsResolvedScoreWeightV2[];
    }
  | {
      kind: "fixed_custom";
      normalization: "none" | "sum_to_one" | "unit_variance";
      requested_weights: PlsResolvedScoreWeightV2[];
      resolved_effective_weights: PlsResolvedScoreWeightV2[];
    };

export interface PlsResolvedScoreExecutionV2 {
  contract_version: "pls_score_execution_v2";
  blocks: Array<{
    construct_id: string;
    indicator_ids: string[];
    scoring: PlsResolvedScoreBlockKindV2;
  }>;
  iteration_accounting: {
    maximum_iterations: number;
    stop_criterion: number;
    estimated_block_count: number;
    fixed_block_count: number;
    performed_iterations: number;
    estimated_block_updates: number;
  };
}

export interface PlsPosthocMinimumSampleSize {
  method_version: "inverse_square_root_posthoc_v1" | "inverse_square_root_posthoc_v2";
  alpha: number;
  power: number;
  test: "directional";
  inverse_square_root_constant: number;
  selection_rule?: "smallest_absolute_statistically_significant_structural_path" | "";
  significance_source?: "pls_bootstrap_normal_reference_two_sided" | null;
  significance_alpha?: number | null;
  eligible_path_count?: number;
  significant_path_count?: number | null;
  driver_source: string | null;
  driver_target: string | null;
  driver_p_value_two_sided?: number | null;
  minimum_absolute_path_coefficient: number | null;
  technically_required_sample_size: number | null;
  analytical_sample_size: number;
  meets_technical_requirement: boolean | null;
  status:
    | "available"
    | "not_applicable_no_structural_path"
    | "inference_unavailable"
    | "inference_incomplete"
    | "no_statistically_significant_path"
    | "undefined_zero_path"
    | "exceeds_supported_integer_range";
  caution: string;
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
  score_lm?: CbsemCfaScoreLmBundleV1 | null;
  bootstrap?: CbsemBootstrapAnalysis | null;
  bootstrap_v2?: CbsemBootstrapAnalysisV2 | null;
  exact_case_bootstrap?: CbsemExactCaseBootstrapResultV1 | null;
  exact_case_bootstrap_studentized?: CbsemExactCaseBootstrapWithStudentizedResultV1 | null;
  exact_case_bootstrap_bca?: CbsemExactCaseBootstrapWithBcaResultV1 | null;
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

export interface CbsemRmseaIntervalAttributionV1 {
  method_version: "rmsea_noncentral_chi_square_inversion_90_n_minus_one_v1";
  confidence_level: 0.9;
}

export interface CbsemFitIndices {
  method_version: string;
  chi_square: number;
  degrees_of_freedom: number;
  p_value?: number | null;
  cfi?: number | null;
  tli?: number | null;
  rmsea?: number | null;
  rmsea_interval_attribution?: CbsemRmseaIntervalAttributionV1 | null;
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

export type CbsemCfaScoreLmUnavailableReasonV1 =
  | "nuisance_information_unavailable"
  | "efficient_information_non_positive"
  | "non_finite_computation";

export type CbsemCfaScoreLmOutcomeV1 =
  | {
      status: "available";
      score: number;
      efficient_score: number;
      candidate_information: number;
      efficient_information: number;
      modification_index: number;
      expected_parameter_change: number;
      p_value: number;
    }
  | {
      status: "unavailable";
      reason: CbsemCfaScoreLmUnavailableReasonV1;
    };

export interface CbsemCfaScoreLmRowV1 {
  parameter_id: string;
  kind: "residual_covariance";
  lhs: string;
  rhs: string;
  outcome: CbsemCfaScoreLmOutcomeV1;
}

export interface CbsemCfaScoreLmBundleV1 {
  method_version: "cbsem_cfa_score_lm_v1";
  scope: "covariance_only_declared_zero_residual_covariances";
  rows: CbsemCfaScoreLmRowV1[];
}

export interface CbsemBootstrapAnalysis {
  method_version: string;
  samples: number;
  usable_samples: number;
  intervals: Array<{ parameter: string; original: number; lower_percentile: number; upper_percentile: number }>;
  warnings: string[];
}

export interface CbsemBootstrapAnalysisV2 {
  method_version: "cbsem_bootstrap_v2";
  algorithm: "indexed_raw_case_refit_ml_v2";
  interval_method: "percentile_type7_v1";
  retry_policy: "no_retry_fixed_preplanned_primary_draws_v1";
  confidence_level: number;
  requested_replicates: number;
  attempted_fits: number;
  usable_replicates: number;
  failed_replicates: number;
  minimum_usable_fraction: number;
  minimum_usable_replicates: number;
  max_attempts_per_replicate: number;
  complete_case_sample_size: number;
  seed: number;
  stream_token: "quickpls_cbsem_ml_case_bootstrap_v2";
  inference:
    | { status: "available" }
    | { status: "unavailable"; reason_code: "insufficient_usable_replicates"; message: string };
  intervals: CbsemBootstrapParameterIntervalV2[];
  failures: CbsemBootstrapFailedReplicateV2[];
  validation_witness: CbsemBootstrapValidationWitnessV2;
  warnings: string[];
}

export interface CbsemBootstrapParameterIntervalV2 {
  parameter: string;
  original: number;
  bootstrap_mean: number;
  bias: number;
  standard_error: number;
  percentile_lower: number;
  percentile_upper: number;
  usable_replicates: number;
}

export interface CbsemBootstrapFailedReplicateV2 {
  replicate_index: number;
  sample_indices_sha256: string;
  reason_code: string;
  message: string;
}

export interface CbsemBootstrapValidationWitnessV2 {
  method_version: "cbsem_bootstrap_validation_witness_v2";
  dataset_fingerprint: string;
  recipe_sha256: string;
  base_result_sha256: string;
  parameter_names: string[];
  successful_replicates: Array<{
    replicate_index: number;
    sample_indices_sha256: string;
    iterations: number;
    objective: number;
    parameter_estimates: number[];
  }>;
}

export type CbsemExactCaseBootstrapFailureKindV1 =
  | "moment_matrix_not_positive_definite"
  | "non_convergence"
  | "inadmissible_solution"
  | "numerical_failure";

export interface CbsemExactCaseBootstrapParameterIntervalV1 {
  parameter_id: string;
  original: number;
  bootstrap_mean: number;
  bias: number;
  standard_error: number;
  percentile_lower: number;
  percentile_upper: number;
  usable_replicates: number;
}

export interface CbsemExactCaseBootstrapWitnessV1 {
  replicate_index: number;
  sampling_positions_sha256: string;
  sample_indices_sha256: string;
  parameter_estimates: number[];
  iterations: number;
  objective: number;
  gradient_norm: number;
}

export interface CbsemExactCaseBootstrapFailureV1 {
  replicate_index: number;
  sampling_positions_sha256: string;
  sample_indices_sha256: string;
  kind: CbsemExactCaseBootstrapFailureKindV1;
  message: string;
}

export type CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1 =
  | "insufficient_usable_replicates"
  | "nonregular_variance_boundary"
  | "zero_null_outside_open_domain"
  | "unsupported_parameter_family";

export type CbsemExactCaseBootstrapHypothesisTestOutcomeV1 =
  | {
      status: "available";
      point_estimate: number;
      two_sided_exceedances: number;
      greater_or_equal_exceedances: number;
      less_or_equal_exceedances: number;
      p_value_two_sided: number;
      p_value_greater: number;
      p_value_less: number;
      selected_exceedances: number;
      selected_p_value: number;
      reject_null: boolean;
    }
  | {
      status: "unavailable";
      reason: CbsemExactCaseBootstrapHypothesisTestUnavailableReasonV1;
    };

export interface CbsemExactCaseBootstrapHypothesisTestParameterV1 {
  parameter_id: string;
  outcome: CbsemExactCaseBootstrapHypothesisTestOutcomeV1;
}

export interface CbsemExactCaseBootstrapHypothesisTestsV1 {
  method_version: "cbsem_exact_case_bootstrap_null_centered_test_tail_v1";
  null_hypothesis: "compiled_free_parameter_equals_zero_v1";
  statistic: "unstudentized_null_centered_parameter_estimate_v1";
  tie_policy: "inclusive_ieee_comparison_v1";
  probability_method: "plus_one_over_usable_plus_one_v1";
  decision_rule: "selected_p_value_less_than_or_equal_alpha_v1";
  selected_test_tail: CbsemBootstrapTestTail;
  null_value: 0;
  significance_level: 0.05;
  usable_replicates: number;
  inference:
    | { status: "available" }
    | { status: "unavailable"; reason_code: "insufficient_usable_refits"; message: string };
  parameters: CbsemExactCaseBootstrapHypothesisTestParameterV1[];
}

export interface CbsemExactCaseBootstrapResultV1 {
  method_version: "cbsem_exact_case_bootstrap_v1";
  estimator_method_version: "cbsem_ml_exact_parameter_table_v3";
  source_dataset_id: string;
  source_dataset_fingerprint: string;
  outer_recipe_analytical_identity_sha256: string;
  base_point_result_sha256: string;
  compiler_analytical_identity_sha256: string;
  plan_sha256: string;
  model_scientific_sha256: string;
  complete_case_sample_size: number;
  complete_case_universe_digest_method: "sha256_source_fingerprint_and_ordered_complete_case_u64_indices_v1";
  complete_case_universe_sha256: string;
  covariance_denominator: "maximum_likelihood_n";
  sample_indices_digest_method: "sha256_source_fingerprint_and_ordered_u64_indices_v1";
  sampling_positions_digest_method: "sha256_stream_seed_replicate_complete_case_n_and_ordered_sampling_positions_v1";
  interval_method: "percentile_type7_v1";
  confidence_level: 0.95;
  requested_replicates: number;
  attempted_refits: number;
  usable_replicates: number;
  failed_replicates: number;
  minimum_usable_fraction: 0.9;
  minimum_usable_replicates: number;
  seed: number;
  stream_token: "quickpls_cbsem_exact_cfa_ml_case_bootstrap_v1";
  retry_policy: "no_retry_fixed_preplanned_primary_draws_v1";
  max_attempts_per_replicate: 1;
  parameter_ids: string[];
  inference:
    | { status: "available" }
    | { status: "unavailable"; reason_code: string; message: string };
  intervals: CbsemExactCaseBootstrapParameterIntervalV1[];
  successful_refits: CbsemExactCaseBootstrapWitnessV1[];
  failed_refits: CbsemExactCaseBootstrapFailureV1[];
  hypothesis_tests?: CbsemExactCaseBootstrapHypothesisTestsV1 | null;
}

export type CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1 =
  | "singular_information"
  | "information_not_positive_definite"
  | "invalid_information_variance_or_standard_error"
  | "derivative_unavailable"
  | "numerical_information_failure";

export interface CbsemExactCaseBootstrapParameterStandardErrorV1 {
  parameter_id: string;
  standard_error: number;
}

export interface CbsemExactCaseBootstrapRefitStandardErrorsV1 {
  method_version: "cbsem_exact_case_bootstrap_refit_standard_errors_v1";
  outcome:
    | {
        status: "available";
        information_method: "cbsem_ml_expected_information_delta_method_v1";
        parameters: CbsemExactCaseBootstrapParameterStandardErrorV1[];
      }
    | {
        status: "unavailable";
        reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1;
      };
}

export type CbsemExactCaseBootstrapStudentizedUnavailableReasonV1 =
  | "point_standard_errors_unavailable"
  | "insufficient_studentized_usable_replicates";

export type CbsemExactCaseBootstrapStudentizedInferenceV1 =
  | { status: "available" }
  | {
      status: "unavailable";
      reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1;
      message: string;
    };

export interface CbsemExactCaseBootstrapStudentizedParameterIntervalV1 {
  parameter_id: string;
  outcome:
    | {
        status: "available";
        point_estimate: number;
        point_standard_error: number;
        lower_pivot_quantile: number;
        upper_pivot_quantile: number;
        interval_lower: number;
        interval_upper: number;
        usable_replicates: number;
      }
    | {
        status: "unavailable";
        reason: CbsemExactCaseBootstrapStudentizedUnavailableReasonV1;
      };
}

export interface CbsemExactCaseBootstrapStudentizedRefitStandardErrorsV1 {
  replicate_index: number;
  outcome:
    | {
        status: "available";
        information_method: "cbsem_ml_expected_information_delta_method_v1";
        standard_errors: number[];
      }
    | {
        status: "unavailable";
        reason: CbsemExactCaseBootstrapRefitStandardErrorUnavailableReasonV1;
      };
}

export interface CbsemExactCaseBootstrapStudentizedSidecarV1 {
  method_version: "cbsem_exact_case_bootstrap_analytic_studentized_interval_v1";
  standard_error_method_version: "cbsem_exact_case_bootstrap_refit_standard_errors_v1";
  expected_information_method: "cbsem_ml_expected_information_delta_method_v1";
  pivot_method: "outer_estimate_minus_point_estimate_over_outer_analytic_standard_error_v1";
  quantile_method: "percentile_type7_v1";
  interval_method: "reversed_type7_studentized_pivot_v1";
  archive_validation_scope: "ledger_and_arithmetic_only_no_raw_refit_or_expected_information_replay_v1";
  confidence_level: 0.95;
  minimum_usable_fraction: 0.9;
  minimum_usable_replicates: number;
  studentized_usable_replicates: number;
  parameter_ids: string[];
  point_standard_errors: CbsemExactCaseBootstrapRefitStandardErrorsV1;
  inference: CbsemExactCaseBootstrapStudentizedInferenceV1;
  intervals: CbsemExactCaseBootstrapStudentizedParameterIntervalV1[];
  refit_standard_errors: CbsemExactCaseBootstrapStudentizedRefitStandardErrorsV1[];
}

export interface CbsemExactCaseBootstrapWithStudentizedResultV1 {
  base: CbsemExactCaseBootstrapResultV1;
  studentized: CbsemExactCaseBootstrapStudentizedSidecarV1;
}

export type CbsemExactCaseBootstrapBcaUnavailableReasonV1 =
  | "base_inference_unavailable"
  | "incomplete_delete_one_ledger"
  | "bias_correction_probability_at_boundary"
  | "degenerate_jackknife_acceleration"
  | "nonfinite_jackknife_arithmetic"
  | "singular_acceleration_adjustment"
  | "invalid_adjusted_probability"
  | "adjusted_probability_order_invalid"
  | "nonfinite_or_reversed_interval";

export type CbsemExactCaseBootstrapBcaInferenceV1 =
  | { status: "available" }
  | {
      status: "unavailable";
      reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1;
      message: string;
    };

export interface CbsemExactCaseBootstrapBcaParameterIntervalV1 {
  parameter_id: string;
  outcome:
    | {
        status: "available";
        point_estimate: number;
        bias_correction: number;
        acceleration: number;
        adjusted_lower_probability: number;
        adjusted_upper_probability: number;
        interval_lower: number;
        interval_upper: number;
        usable_replicates: number;
      }
    | {
        status: "unavailable";
        reason: CbsemExactCaseBootstrapBcaUnavailableReasonV1;
      };
}

export interface CbsemExactCaseBootstrapDeleteOneWitnessV1 {
  omitted_complete_case_position: number;
  omitted_source_row_index: number;
  retained_sampling_positions_sha256: string;
  retained_sample_indices_sha256: string;
  parameter_estimates: number[];
  iterations: number;
  objective: number;
  gradient_norm: number;
}

export interface CbsemExactCaseBootstrapDeleteOneFailureV1 {
  omitted_complete_case_position: number;
  omitted_source_row_index: number;
  retained_sampling_positions_sha256: string;
  retained_sample_indices_sha256: string;
  kind: CbsemExactCaseBootstrapFailureKindV1;
  message: string;
}

export interface CbsemExactCaseBootstrapBcaSidecarV1 {
  method_version: "cbsem_exact_case_bootstrap_bca_interval_v1";
  base_bootstrap_method_version: "cbsem_exact_case_bootstrap_v1";
  outer_recipe_analytical_identity_sha256: string;
  base_point_result_sha256: string;
  compiler_analytical_identity_sha256: string;
  plan_sha256: string;
  model_scientific_sha256: string;
  delete_one_refit_method_version: "cbsem_exact_case_bootstrap_delete_one_refit_v1";
  bias_correction_method: "midrank_less_plus_half_ties_no_clamp_v1";
  acceleration_method: "complete_delete_one_jackknife_neumaier_mean_squares_cubes_acceleration_v2";
  adjusted_probability_method: "efron_bca_statrs_inverse_normal_libm_erfc_cdf_adjustment_v2";
  quantile_method: "percentile_type7_v1";
  retry_policy: "no_retry_exactly_one_fit_per_omitted_case_v1";
  confidence_level: 0.95;
  bootstrap_usable_replicates: number;
  minimum_bootstrap_usable_replicates: number;
  delete_one_case_count: number;
  parameter_ids: string[];
  inference: CbsemExactCaseBootstrapBcaInferenceV1;
  intervals: CbsemExactCaseBootstrapBcaParameterIntervalV1[];
  successful_delete_one_refits: CbsemExactCaseBootstrapDeleteOneWitnessV1[];
  failed_delete_one_refits: CbsemExactCaseBootstrapDeleteOneFailureV1[];
}

export interface CbsemExactCaseBootstrapWithBcaResultV1 {
  base: CbsemExactCaseBootstrapResultV1;
  bca: CbsemExactCaseBootstrapBcaSidecarV1;
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

export interface MicomPermutationLedgerEntry {
  replicate: number;
  partition_sha256: string;
  group_a_rows: number;
  group_b_rows: number;
  step2_status: "usable" | "failed";
  step2_failure_code?: string | null;
  step3_status: "usable" | "failed";
  step3_failure_code?: string | null;
}

export interface MicomAnalysis {
  method_version: string;
  group_column: string;
  permutation_samples: number;
  usable_permutations: number;
  attempted_permutations?: number | null;
  failed_permutations?: number | null;
  confidence_level?: number | null;
  retry_policy?: string | null;
  step1_status?: string | null;
  step1_computed?: boolean | null;
  step2_usable_permutations?: number | null;
  step2_failed_permutations?: number | null;
  step3_usable_permutations?: number | null;
  step3_failed_permutations?: number | null;
  permutation_plan_sha256?: string | null;
  permutation_ledger?: MicomPermutationLedgerEntry[];
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
  retry_policy?: string | null;
  permutation_plan_sha256?: string | null;
  permutation_ledger?: MicomPermutationLedgerEntry[];
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

export type PlsFitCriterionValue =
  | { status: "available"; value: number }
  | { status: "unavailable"; reason_code: string };

export interface PlsFitMeasures {
  srmr: number;
  d_uls: number;
  d_g?: PlsFitCriterionValue;
  chi_square?: PlsFitCriterionValue;
  degrees_of_freedom?: PlsFitCriterionValue;
  nfi?: PlsFitCriterionValue;
}

export interface PlsModelFit {
  method_version?: string;
  analytical_sample_size?: number;
  indicator_order?: string[];
  matrix_convention?: string;
  geodesic_logarithm?: string;
  observed_correlation?: number[][];
  saturated_implied_correlation?: number[][];
  estimated_implied_correlation?: number[][];
  null_model_chi_square?: PlsFitCriterionValue;
  saturated: PlsFitMeasures;
  estimated: PlsFitMeasures;
  exact_fit_inference?: {
    procedure: string;
    status: "unavailable";
    reason_code: string;
  };
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
  model_fit?: PlsModelFit;
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

export type HtmtBootstrapInferenceStatus = "available" | "not_applicable" | "unavailable";

export interface HtmtBootstrapInferenceCell {
  status: HtmtBootstrapInferenceStatus;
  reason: string | null;
  original: number | null;
  bootstrap_mean: number | null;
  bias: number | null;
  standard_error: number | null;
  bias_correction: number | null;
  lower: number | null;
  upper: number | null;
  usable_replicates: number;
  failed_replicates: number;
  below_original: number;
  tied_original: number;
  replicate_min: number | null;
  replicate_max: number | null;
  upper_bound_below_critical_value: boolean | null;
  usable_replicate_indices_sha256: string | null;
  pair_unavailable_replicates: Array<{
    replicate_index: number;
    reason_code: string;
  }>;
}

export interface HtmtBootstrapInference {
  method_version: string;
  point_method_version: string;
  constructs: string[];
  correlation_type: "pearson";
  absolute_correlations: boolean;
  interval_method: "bias_corrected_percentile_type7_v1";
  test_type: "one_tailed_upper";
  significance_level: 0.05;
  equivalent_two_sided_confidence_level: 0.90;
  critical_value: 0.90;
  decision_rule: "bias_corrected_upper_bound_strictly_below_critical_value_v1";
  replicate_index_digest_method: "sha256_u32_le_v1";
  requested_replicates: number;
  minimum_usable_replicates: number;
  retry_policy: "no_retry_fixed_preplanned_primary_draws_v1";
  cells: HtmtBootstrapInferenceCell[][];
}

export interface HtmtBootstrapInferenceBundle {
  method_version: "htmt_bias_corrected_bootstrap_inference_v1";
  htmt_plus: HtmtBootstrapInference;
  htmt_original: HtmtBootstrapInference;
}

export type PlsModelFitExactStatus = "available" | "partial" | "unavailable";
export type PlsModelFitExactCriterion = "srmr" | "d_uls" | "d_g";

export interface PlsModelFitExactCriterionInference {
  criterion: PlsModelFitExactCriterion;
  status: PlsModelFitExactStatus;
  original: number;
  requested_replicates: number;
  minimum_usable_replicates: number;
  usable_replicates: number;
  failed_replicates: number;
  usable_replicate_indices_sha256: string;
  replicate_min: number | null;
  replicate_max: number | null;
  upper_95: number | null;
  upper_99: number | null;
  not_rejected_95: boolean | null;
  not_rejected_99: boolean | null;
  exceed_or_equal_count: number;
  empirical_upper_tail_probability: number | null;
  unavailable_reason_code: string | null;
}

export interface PlsModelFitExactReplicateLedgerEntry {
  replicate_index: number;
  sample_indices_sha256: string;
  status: "success" | "partial" | "failed";
  srmr: number | null;
  d_uls: number | null;
  d_g: number | null;
  criterion_failures: Array<{
    criterion: PlsModelFitExactCriterion;
    reason_code: string;
  }>;
  failure_reason_code: string | null;
  failure_message: string | null;
}

export interface PlsModelFitExactVariantInference {
  variant: "saturated" | "estimated";
  status: PlsModelFitExactStatus;
  operation: string;
  target_correlation_sha256: string;
  transformed_correlation: number[][];
  transformed_correlation_sha256: string;
  transformation_max_abs_error: number;
  requested_replicates: number;
  ledger: PlsModelFitExactReplicateLedgerEntry[];
  criteria: PlsModelFitExactCriterionInference[];
}

export interface PlsModelFitExactInference {
  method_version: "pls_model_fit_exact_v1";
  point_fit_method_version: "pls_model_fit_v2";
  estimator_method_version: string;
  resampling_method_version: string;
  procedure: "adapted_bollen_stine_saturated_and_estimated_v1";
  transformation: string;
  matrix_power: string;
  quantile_method: "hyndman_fan_type7_v1";
  decision_rule: string;
  retry_policy: "no_retry_no_replacement_fixed_indexed_draws_v1";
  sample_digest_method: "sha256_u64_le_v1";
  usable_index_digest_method: "sha256_u32_le_v1";
  matrix_digest_method: "sha256_f64_bits_row_major_v1";
  status: PlsModelFitExactStatus;
  analytical_sample_size: number;
  indicator_order: string[];
  master_seed: number;
  requested_replicates: number;
  minimum_usable_fraction: 0.90;
  observed_correlation_sha256: string;
  saturated: PlsModelFitExactVariantInference;
  estimated: PlsModelFitExactVariantInference;
}

export interface PlsBootstrapRun {
  method_version: string;
  /** Present only for the separately attributed full-reestimation PLSc workflow. */
  estimator_method_version?: string;
  /** Indexed-resampling kernel version used underneath the PLSc workflow. */
  resampling_method_version?: string;
  plan: { replicates: number; master_seed: number; operation: string };
  minimum_usable_fraction?: number;
  retry_policy?: string;
  original_parameter_values_sha256?: string;
  usable_replicates: number;
  failed_replicates: Array<{ replicate_index: number; message: string; reason_code?: string; sample_indices_sha256?: string }>;
  replicate_ledger?: Array<{
    replicate_index: number;
    sample_indices_sha256: string;
    status: "success" | "failed";
    parameter_values_sha256: string | null;
    reason_code: string | null;
    message: string | null;
  }>;
  /** Replayable full-PLSc primary refits; required by current plsc_bootstrap_v1 output. */
  successful_replicates?: Array<{
    replicate_index: number;
    iterations: number;
    used_observations: number;
    omitted_observations: number;
    parameters: Record<string, number>;
  }>;
  percentile: {
    confidence_level: number;
    parameters: Array<{ parameter: string; original: number; bootstrap_mean: number; bias: number; standard_error: number; lower: number; upper: number; usable_replicates: number; t_statistic?: number | null; p_value_two_sided?: number | null }>;
  };
  bca?: {
    confidence_level: number;
    jackknife_case_count: number;
    parameters: Array<{ parameter: string; bias_correction: number | null; acceleration: number | null; lower: number | null; upper: number | null; unavailable_reason: string | null }>;
  } | null;
  /** Replayable full-PLSc delete-one refits; required by current plsc_bootstrap_v1 output. */
  successful_jackknife_cases?: Array<{
    omitted_case: number;
    iterations: number;
    used_observations: number;
    omitted_observations: number;
    parameters: Record<string, number>;
  }>;
  studentized?: {
    method_version: string;
    confidence_level: number;
    inner_replicates: number;
    minimum_usable_fraction: number;
    stream_domain: string;
    failure?: { reason_code: string; first_primary_replicate: number; failed_primary_replicates: number; message: string } | null;
    parameters: Array<{ parameter: string; original: number; outer_standard_error: number; outer_scale: number; usable_primary_replicates: number; lower_pivot: number | null; upper_pivot: number | null; lower: number | null; upper: number | null; unavailable_reason: string | null }>;
  } | null;
  failed_jackknife_cases?: Array<{ omitted_case: number; reason_code: string; message: string }>;
  warnings?: string[];
  htmt_inference?: HtmtBootstrapInferenceBundle | null;
  model_fit_exact_inference?: PlsModelFitExactInference | null;
  /** Present only when the user explicitly selects a one-sided PLS bootstrap test. */
  test_tail_inference?: PlsBootstrapTestTailInference;
}

export type PlsBootstrapTestTail = "two_sided" | "one_sided_greater" | "one_sided_less";

export interface PlsBootstrapTestTailInference {
  method_version: "pls_bootstrap_null_centered_test_tail_v1";
  selected_test_tail: Exclude<PlsBootstrapTestTail, "two_sided">;
  parameters: PlsBootstrapTestTailParameterInference[];
}

export interface PlsBootstrapTestTailParameterInference {
  parameter: string;
  usable_replicates: number;
  two_sided_exceedances: number;
  greater_or_equal_exceedances: number;
  less_or_equal_exceedances: number;
  p_value_two_sided: number;
  p_value_greater: number;
  p_value_less: number;
}

export interface PlsPermutationRun {
  method_version: string;
  estimator_method_version?: string;
  scheduler_method_version?: string;
  plan: { permutations: number; master_seed: number; operation: string };
  test_method?: string;
  significance_level?: number;
  minimum_usable_fraction?: number;
  retry_policy?: string;
  group_column?: string;
  group_a?: { group: string; observations: number; parameter_values_sha256: string };
  group_b?: { group: string; observations: number; parameter_values_sha256: string };
  pooled_parameter_values_sha256?: string;
  usable_permutations?: number;
  failed_permutations?: Array<{
    permutation_index: number;
    label_assignment_sha256: string;
    reason_code: string;
    message: string;
  }>;
  permutation_ledger?: Array<{
    permutation_index: number;
    label_assignment_sha256: string;
    status: "success" | "failed";
    parameter_values_sha256: string | null;
    reason_code: string | null;
    message: string | null;
  }>;
  parameters: Array<{
    parameter: string;
    family?: "path" | "outer_loading" | "rho_a" | "construct_correlation" | "r_squared";
    estimate_a?: number;
    estimate_b?: number;
    original: number;
    exceedances: number;
    p_value_two_sided: number;
    permutations: number;
  }>;
  directional_inference?: {
    method_version: string;
    test_method: string;
    parameters: Array<{
      parameter: string;
      greater_or_equal: number;
      less_or_equal: number;
      p_value_greater: number;
      p_value_less: number;
      permutations: number;
    }>;
  } | null;
  /** Present only for an explicitly selected one-sided PLSc permutation tail. */
  selected_tail_inference?: PlscPermutationSelectedTailInference | null;
  warnings?: string[];
}

export interface PlscPermutationSelectedTailInference {
  method_version: "plsc_permutation_selected_tail_v1";
  orientation: "group_a_minus_group_b";
  selected_test_tail: Exclude<PlscPermutationTestTail, "two_sided">;
  parameters: PlscPermutationSelectedTailParameterInference[];
}

export interface PlscPermutationSelectedTailParameterInference {
  parameter: string;
  selected_exceedances: number;
  selected_p_value: number;
  permutations: number;
}

export interface AnalysisEngineSettingsSnapshot {
  method: AnalysisMethodId;
  weighting_scheme: "path" | "factor" | "pca";
  tolerance: number;
  max_iterations: number;
  bootstrap_samples: number;
  /** Rust omits the default two-sided value to preserve historical bytes. */
  bootstrap_test_tail?: PlsBootstrapTestTail;
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
    | { kind: "pls_sample_size_power_v1"; analysis: PlsSampleSizePowerResultV1 }
    | { kind: "pls_sample_size_power_v2"; analysis: PlsSampleSizePowerResultV2 }
    | { kind: "legacy"; value: unknown };
}

export interface PlsSampleSizePowerOutcomeV1 {
  sample_size: number;
  replicate_index: number;
  stream_identity: string;
  attempted: boolean;
  successful: boolean;
  converged: boolean;
  target_estimate: number | null;
  p_value_two_sided: number | null;
  bootstrap_requested_replicates?: number | null;
  bootstrap_usable_replicates?: number | null;
  bootstrap_failed_replicates?: number | null;
  bootstrap_two_sided_exceedances?: number | null;
  rejected: boolean;
  failure_code: string | null;
  failure_message: string | null;
}

export interface PlsSampleSizePowerRecipeV1 {
  schema_version: 1;
  capability_id: "qpls3.pls.sample_size_power";
  method_version: "pls_sample_size_power_v1";
  scenario_identity: string;
  design: {
    predictor_construct: string;
    outcome_construct: string;
    predictor_indicator_loadings: number[];
    outcome_indicator_loadings: number[];
    population_path: number;
    exogenous_distribution: "standard_normal";
    structural_disturbance_distribution: "standard_normal";
    indicator_error_distribution: "standard_normal";
    missing_data: "none";
  };
  estimator: {
    weighting_scheme: "path";
    preprocessing: "standardized";
    tolerance: number;
    max_iterations: number;
  };
  inference: "case_bootstrap_normal_reference_two_sided";
  sample_size_grid: number[];
  alpha: number;
  target_power: number;
  confidence_level: number;
  monte_carlo_replicates: number;
  bootstrap_replicates: number;
  master_seed: number;
  workers: number;
}

export interface PlsSampleSizePowerRowV1 {
  sample_size: number;
  requested_replicates: number;
  attempted_replicates: number;
  successful_replicates: number;
  failed_replicates: number;
  rejections: number;
  achieved_power: number;
  confidence_lower: number;
  confidence_upper: number;
  qualifies: boolean;
}

export interface PlsSampleSizePowerResultV1 {
  schema_version: 1;
  capability_id: "qpls3.pls.sample_size_power";
  method_version: "pls_sample_size_power_v1";
  recipe_digest: string;
  stream_domain: string;
  failure_policy: "failed_replicates_count_as_non_rejections_v1";
  interval_method: "wilson_score_two_sided_v1";
  inference_method: "pls_pm_case_bootstrap_normal_reference_two_sided_v1";
  pls_method_version: string;
  resampling_method_version: string;
  workload: {
    grid_points: number;
    planned_datasets: number;
    estimated_pls_fits: number;
    estimated_pls_case_fits: number;
  };
  rows: PlsSampleSizePowerRowV1[];
  outcomes: PlsSampleSizePowerOutcomeV1[];
  outcome_digest: string;
  decision: { status: "reached"; sample_size: number } | { status: "not_reached" };
  monotonicity_violations: number;
  warnings: string[];
  exclusions: string[];
}

export interface PlsSampleSizePowerRecipeV2
  extends Omit<PlsSampleSizePowerRecipeV1, "schema_version" | "method_version" | "inference"> {
  schema_version: 2;
  method_version: "pls_sample_size_power_v2";
  inference: "case_bootstrap_null_centered_two_sided_plus_one";
}

export interface PlsSampleSizePowerResultV2
  extends Omit<
    PlsSampleSizePowerResultV1,
    "schema_version" | "method_version" | "inference_method"
  > {
  schema_version: 2;
  method_version: "pls_sample_size_power_v2";
  inference_method: "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2";
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
