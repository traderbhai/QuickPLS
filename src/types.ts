export type WorkspaceView = "welcome" | "data" | "models" | "analyses" | "run" | "runs" | "groups" | "reports";
export type ExplorerTab = "constructs" | "variables" | "structure" | "issues";
export type UiDensity = "comfortable" | "compact";
export type ResultWorkspaceTab = "overview" | "measurement" | "structural" | "validity" | "inference" | "prediction" | "groups" | "diagnostics" | "interpretation" | "comparison";
export type MethodSetupMode = "basic" | "expert";
export type MethodPresetId = "standard_pls" | "pls_bootstrap" | "plspredict" | "micom_mga" | "cbsem_cfa" | "ols_regression" | "nca";
export type MeasurementMode = "reflective" | "formative";
export type MethodStatus = "experimental" | "validated" | "unsupported";
export type AnalysisMethodId = "pls_pm" | "bootstrap" | "plsc" | "wpls" | "cca" | "cta_pls" | "endogeneity" | "nonlinear_effects" | "moderated_mediation" | "predict" | "mga" | "ipma" | "cbsem" | "pca" | "gsca" | "regression" | "nca";
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
  selectedDemo: "corporate_reputation" | "simple_pls" | "cbsem_cfa";
  recentProjectCards: string[];
}

export interface LargeModelViewState {
  indicatorsCollapsed: boolean;
  isolatedConstructId: string | null;
  neighborhoodMode: "off" | "selected" | "upstream_downstream";
}

export interface ToastNotification {
  id: string;
  tone: "success" | "warning" | "info";
  title: string;
  detail?: string;
}

export interface AnalysisUiSettings {
  method: AnalysisMethodId;
  bootstrapSamples: number;
  studentizedInnerSamples: number;
  permutationSamples: number;
  seed: number;
  workers: number;
  confidenceLevel: number;
  caseWeightColumn?: string | null;
  groupColumn?: string | null;
  ipmaTargets?: string | null;
  groupMethods?: string | null;
  groupPermutationSamples?: number;
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
  regressionType?: "ols" | "logistic" | "process";
  regressionOutcome?: string | null;
  regressionPredictors?: string | null;
  regressionControls?: string | null;
  robustSe?: "none" | "hc0" | "hc3" | "hc4";
  processModel?: "mediation" | "moderation" | "moderated_mediation";
  processX?: string | null;
  processM?: string | null;
  processW?: string | null;
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
  fingerprint?: string;
  kind?: "raw" | "covariance" | "correlation";
  sampleSize?: number | null;
  columnMetadata?: ColumnMetadata[];
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

export interface NativeProjectSnapshot {
  name: string;
  path: string | null;
  readOnly: boolean;
  recovered: boolean;
  recoverySource?: "autosave" | "backup" | null;
  datasets: Dataset[];
  workspace?: { nodes: unknown[]; edges: unknown[]; runs?: AnalysisRun[]; analysisSettings?: AnalysisUiSettings; diagramMode?: DiagramMode; diagramOverlaySettings?: Partial<DiagramOverlaySettings>; publicationDiagramSettings?: Partial<PublicationDiagramSettings>; diagramLayout?: Partial<DiagramLayoutState>; activeDatasetId?: string } | null;
}

export interface AnalysisRun {
  id: string;
  name: string;
  method: string;
  createdAt: string;
  seed: number;
  status: "completed" | "failed";
  warnings: string[];
  fingerprint: string;
  result?: PlsResult;
  assessment?: AssessmentResult;
  bootstrap?: PlsBootstrapRun;
  permutation?: PlsPermutationRun;
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
  coefficients: Array<{ term: string; estimate: number; standard_error: number; statistic: number; p_value_two_sided: number; confidence_interval_lower: number; confidence_interval_upper: number; odds_ratio?: number | null }>;
  fit: { r_squared?: number | null; adjusted_r_squared?: number | null; f_statistic?: number | null; log_likelihood?: number | null; pseudo_r_squared?: number | null; aic: number; bic: number; rmse?: number | null };
  predictions: Array<{ observation: number; fitted: number; residual?: number | null; probability?: number | null }>;
  process?: { method_version: string; model: string; effects: Array<{ effect: string; estimate: number; lower_percentile?: number | null; upper_percentile?: number | null }>; simple_slopes: Array<{ moderator_value: number; slope: number }>; warnings: string[] } | null;
  warnings: string[];
}

export interface NcaAnalysis {
  method_version: string;
  ceiling: string;
  permutation_samples: number;
  usable_permutations: number;
  x: string;
  y: string;
  observations: number;
  ceilings: Array<{ ceiling: string; effect_size: number; permutation_p_value?: number | null }>;
  bottlenecks: Array<{ outcome_percent: number; required_x_percent: number }>;
  warnings: string[];
}

export interface GscaAnalysis {
  method_version: string;
  iterations: number;
  fit: number;
  adjusted_fit: number;
  gfi: number;
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
  repeated_kfold?: {
    method_version: string;
    folds: number;
    repeats: number;
    assignment: string;
    total_test_observations: number;
    targets: PlsPredictTarget[];
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
  warnings: string[];
}

export interface MicomAnalysis {
  method_version: string;
  group_column: string;
  permutation_samples: number;
  usable_permutations: number;
  groups: Array<{ group: string; observations: number }>;
  constructs: Array<{
    construct: string;
    configural_invariance: boolean;
    compositional_correlation: number;
    compositional_p_value: number | null;
    mean_difference: number;
    mean_p_value: number | null;
    variance_difference: number;
    variance_p_value: number | null;
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
  comparisons: Array<{
    source: string;
    target: string;
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

export interface AnalysisResultEnvelope {
  schema_version: number;
  id: string;
  status: "completed" | "failed";
  provenance: {
    recipe_id: string;
    dataset_fingerprint: string;
    method: string;
    method_version: string;
    engine_version: string;
    seed: number;
    started_at: string;
    completed_at: string;
  };
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
