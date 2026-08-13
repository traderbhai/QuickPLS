import type { Edge, Node } from "@xyflow/react";
import type {
  AnalysisMethodId,
  AnalysisUiSettings,
  ConstructData,
  MeasurementMode,
  NativeAnalysisMethodConfig,
  PathEdgeData,
} from "../types";
import {
  NATIVE_PREDICTION_METHOD_LABEL,
  nativeCalculationModeForSettings,
  nativeCalculationSettingsForMode,
  type NativeCalculationMode,
} from "./nativeCalculationMode";
import { nativeIpmaTargetOptions } from "./nativeIpma";
import { NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS } from "./nativeRegressionBootstrapWitness";
import {
  NATIVE_PROCESS_MAX_EQUATION_TERMS,
  nativeProcessGraphAssessment,
} from "./nativeProcess";

/**
 * Wire-level recipe contract consumed by the Tauri `start_pls_job` command.
 * Keep field names aligned with `qpls_core::AnalysisRecipe` and its nested
 * serde contracts. This module deliberately has no UI, store, clock, or UUID
 * dependencies so callers can construct and test recipes deterministically.
 */

export type NativeEngineAnalysisMethod = Exclude<AnalysisMethodId, "bootstrap" | "permutation">;
export type NativeAdvancedAnalysisMethod = Exclude<NativeEngineAnalysisMethod, "pls_pm">;
export type NativeAnalysisRecipeKind =
  | "pls_algorithm"
  | "pls_bootstrap"
  | "pls_permutation"
  | NativeAdvancedAnalysisMethod;

export type NativeRecipeScopeStatus = "validated" | "experimental";

export interface NativeAnalysisRecipeDescriptor {
  kind: NativeAnalysisRecipeKind;
  engineMethod: NativeEngineAnalysisMethod;
  family: string;
  label: string;
  scopeStatus: NativeRecipeScopeStatus;
  scopeMetadata: string;
}

export const NATIVE_ANALYSIS_RECIPE_DESCRIPTORS = [
  { kind: "pls_algorithm", engineMethod: "pls_pm", family: "PLS-SEM", label: "PLS-SEM Algorithm", scopeStatus: "validated", scopeMetadata: "validated_v1_0_supported_pls_scope" },
  { kind: "pls_bootstrap", engineMethod: "pls_pm", family: "PLS-SEM", label: "PLS-SEM Bootstrapping", scopeStatus: "validated", scopeMetadata: "validated_v1_0_supported_pls_scope" },
  { kind: "pls_permutation", engineMethod: "pls_pm", family: "Inference", label: "Structural Path Randomization", scopeStatus: "experimental", scopeMetadata: "candidate_freedman_lane_path_randomization_scope" },
  { kind: "plsc", engineMethod: "plsc", family: "PLS-SEM", label: "Consistent PLS", scopeStatus: "validated", scopeMetadata: "validated_v1_2_1_plsc_bounded_scope" },
  { kind: "wpls", engineMethod: "wpls", family: "PLS-SEM", label: "Weighted PLS", scopeStatus: "validated", scopeMetadata: "validated_v1_2_1_wpls_bounded_scope" },
  { kind: "cca", engineMethod: "cca", family: "Assessment", label: "CCA composite residual diagnostics", scopeStatus: "validated", scopeMetadata: "validated_v1_2_3_cca_bounded_scope" },
  { kind: "cta_pls", engineMethod: "cta_pls", family: "PLS-SEM", label: "Confirmatory Tetrad Analysis", scopeStatus: "validated", scopeMetadata: "validated_v1_2_3_cta_pls_bounded_scope" },
  { kind: "endogeneity", engineMethod: "endogeneity", family: "PLS-SEM", label: "Gaussian-Copula Endogeneity", scopeStatus: "validated", scopeMetadata: "validated_v1_2_3_endogeneity_bounded_scope" },
  { kind: "nonlinear_effects", engineMethod: "nonlinear_effects", family: "PLS-SEM", label: "Nonlinear Effects", scopeStatus: "validated", scopeMetadata: "validated_v1_2_3_nonlinear_effects_bounded_scope" },
  { kind: "moderated_mediation", engineMethod: "moderated_mediation", family: "PLS-SEM", label: "Moderated Mediation", scopeStatus: "validated", scopeMetadata: "validated_v1_2_3_moderated_mediation_bounded_scope" },
  { kind: "predict", engineMethod: "predict", family: "Prediction", label: NATIVE_PREDICTION_METHOD_LABEL, scopeStatus: "validated", scopeMetadata: "validated_plspredict_indicator_v2_and_cvpat_indicator_benchmarks_v2_bounded_scope" },
  { kind: "mga", engineMethod: "mga", family: "Groups", label: "MICOM and Two-Group Permutation MGA", scopeStatus: "validated", scopeMetadata: "validated_micom_v2_and_permutation_mga_v2_bounded_scope" },
  { kind: "ipma", engineMethod: "ipma", family: "Assessment", label: "Importance-Performance Map Analysis", scopeStatus: "validated", scopeMetadata: "validated_v1_2_1_ipma_bounded_scope" },
  { kind: "cbsem", engineMethod: "cbsem", family: "CB-SEM", label: "CB-SEM / CFA", scopeStatus: "validated", scopeMetadata: "validated_v1_2_4_cbsem_single_group_bounded_scope" },
  { kind: "pca", engineMethod: "pca", family: "Components", label: "Principal Component Analysis", scopeStatus: "validated", scopeMetadata: "validated_pca_v1_bounded_scope" },
  { kind: "gsca", engineMethod: "gsca", family: "Component Models", label: "GSCA", scopeStatus: "validated", scopeMetadata: "validated_gsca_als_v2_bounded_scope" },
  { kind: "regression", engineMethod: "regression", family: "Regression", label: "Ordinary Least Squares Regression", scopeStatus: "validated", scopeMetadata: "validated_regression_ols_v1_bounded_scope" },
  { kind: "nca", engineMethod: "nca", family: "Necessary Conditions", label: "Necessary Condition Analysis", scopeStatus: "validated", scopeMetadata: "validated_nca_v2_bounded_scope" },
] as const satisfies readonly NativeAnalysisRecipeDescriptor[];

export const NATIVE_ANALYSIS_RECIPE_KINDS = NATIVE_ANALYSIS_RECIPE_DESCRIPTORS.map((item) => item.kind) as readonly NativeAnalysisRecipeKind[];

const descriptorByKind = new Map<NativeAnalysisRecipeKind, NativeAnalysisRecipeDescriptor>(
  NATIVE_ANALYSIS_RECIPE_DESCRIPTORS.map((item) => [item.kind, item]),
);

// Desktop-policy bounds. Some Rust scalar contracts are intentionally wider;
// these values mirror the existing native setup controls and store normalizer.
export const NATIVE_ANALYSIS_RECIPE_BOUNDS = {
  tolerance: { minimum: 1e-12, maximum: 0.01 },
  maxIterations: { minimum: 100, maximum: 100_000 },
  bootstrapSamples: { minimum: 100, maximum: 10_000, default: 10_000 },
  regressionBootstrapSamples: { minimum: 99, maximum: 10_000, default: 10_000 },
  studentizedInnerSamples: { minimum: 99, maximum: 999 },
  permutationSamples: { minimum: 99, maximum: 10_000, default: 999 },
  workers: { minimum: 1, maximum: 64 },
  seed: { minimum: 0, maximum: 4_294_967_295 },
  confidenceLevel: { minimum: 0.8, maximum: 0.999 },
  groupPermutationSamples: { minimum: 5_000, maximum: 10_000, default: 5_000 },
  segmentCount: { minimum: 2, maximum: 5 },
  fimixClassCount: { minimum: 2, maximum: 3 },
  segmentStarts: { minimum: 1, maximum: 50 },
  minimumSegmentShare: { minimum: 0.05, maximum: 0.4 },
  cbsemBootstrapSamples: { minimum: 0, maximum: 10_000 },
  pcaComponents: { minimum: 1, maximum: 50 },
  ncaPermutationSamples: { minimum: 1, maximum: 10_000 },
} as const;

export interface NativeRecipeConstruct {
  id: string;
  name: string;
  short_name: string;
  mode: MeasurementMode;
  indicators: string[];
}

export interface NativeRecipeStructuralPath {
  source: string;
  target: string;
}

export interface NativeRecipeControlPath extends NativeRecipeStructuralPath {
  label: string | null;
}

export interface NativeRecipeHigherOrderConstruct {
  id: string;
  components: string[];
  method: "repeated_indicators" | "two_stage" | "hybrid";
  stage_one_recipe: string | null;
}

export interface NativeRecipeInteraction {
  id: string;
  predictor: string;
  moderator: string;
  product_construct: string;
  outcome: string;
  method: "two_stage_product_score";
}

export interface NativeRecipeModel {
  id: string;
  name: string;
  constructs: NativeRecipeConstruct[];
  paths: NativeRecipeStructuralPath[];
  controls: NativeRecipeControlPath[];
  higher_order_constructs: NativeRecipeHigherOrderConstruct[];
  interactions: NativeRecipeInteraction[];
}

export interface NativeRecipeSettings {
  method: NativeEngineAnalysisMethod;
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

export interface NativeAnalysisRecipe {
  schema_version: 3;
  id: string;
  created_at: string;
  dataset_fingerprint: string;
  model: NativeRecipeModel;
  settings: NativeRecipeSettings;
  method_config: NativeAnalysisMethodConfig;
  metadata: Record<string, string>;
}

export interface NativeAnalysisRecipeBuildInput {
  kind?: NativeAnalysisRecipeKind;
  recipeId: string;
  modelId: string;
  createdAt: string;
  datasetFingerprint: string;
  projectName: string;
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  settings: Readonly<AnalysisUiSettings>;
}

export class NativeAnalysisRecipeBuildError extends Error {
  readonly field: string;

  constructor(field: string, message: string) {
    super(message);
    this.name = "NativeAnalysisRecipeBuildError";
    this.field = field;
  }
}

export function nativeAnalysisRecipeDescriptor(kind: NativeAnalysisRecipeKind): NativeAnalysisRecipeDescriptor {
  const descriptor = descriptorByKind.get(kind);
  if (!descriptor) throw new NativeAnalysisRecipeBuildError("kind", `Unsupported native analysis recipe kind: ${String(kind)}`);
  return descriptor;
}

export function nativeAnalysisRecipeKindForSettings(settings: Readonly<AnalysisUiSettings>): NativeAnalysisRecipeKind {
  if (settings.method === "permutation") return "pls_permutation";
  if (settings.method !== "pls_pm" && settings.method !== "bootstrap") return settings.method;
  return nativeAnalysisRecipeKindForCalculationMode(nativeCalculationModeForSettings(settings));
}

export function nativeAnalysisRecipeKindForCalculationMode(
  mode: NativeCalculationMode,
): "pls_algorithm" | "pls_bootstrap" | "pls_permutation" | "predict" {
  if (mode === "predict") return "predict";
  if (mode === "bootstrap") return "pls_bootstrap";
  if (mode === "permutation") return "pls_permutation";
  return "pls_algorithm";
}

export function buildNativeAnalysisRecipe(input: NativeAnalysisRecipeBuildInput): NativeAnalysisRecipe {
  const kind = input.kind ?? nativeAnalysisRecipeKindForSettings(input.settings);
  const descriptor = nativeAnalysisRecipeDescriptor(kind);
  validateIdentity(input);

  const settings = buildSettings(kind, descriptor.engineMethod, input.settings);
  const methodConfig = buildMethodConfig(kind, input.settings);
  const metadata = buildMetadata(descriptor.scopeMetadata, methodConfig);
  const model = buildNativeRecipeModel(input.modelId, input.projectName, input.nodes, input.edges);
  if (kind === "cca") validateCcaModel(model);
  if (kind === "cta_pls") validateCtaPlsModel(model);
  if (kind === "ipma" && methodConfig.kind === "ipma") validateIpmaModel(model, methodConfig.targets[0], input.nodes, input.edges);
  if (kind === "cbsem" && methodConfig.kind === "cbsem") validateCbsemModel(model, methodConfig.model_type);
  if (kind === "gsca") validateGscaModel(model, input.edges);
  if (
    methodConfig.kind === "mga"
    && model.constructs.some((construct) => construct.indicators.includes(methodConfig.group_column))
  ) {
    fail("groupColumn", "The two-group MGA grouping variable cannot also be assigned as a model indicator.");
  }

  return {
    schema_version: 3,
    id: input.recipeId,
    created_at: input.createdAt,
    dataset_fingerprint: input.datasetFingerprint.trim(),
    model,
    settings,
    method_config: methodConfig,
    metadata,
  };
}

function buildSettings(
  kind: NativeAnalysisRecipeKind,
  method: NativeEngineAnalysisMethod,
  source: Readonly<AnalysisUiSettings>,
): NativeRecipeSettings {
  const weightingScheme = kind === "nca" || kind === "pca" || kind === "regression" || kind === "cbsem" || kind === "gsca" ? "path" : (source.weightingScheme ?? "path");
  const tolerance = kind === "gsca" ? 1e-7 : (source.tolerance ?? 1e-7);
  const maxIterations = kind === "gsca" ? 3_000 : (source.maxIterations ?? 3_000);
  const regressionBootstrap = kind === "regression" && source.regressionBootstrap === true;
  const workers = kind === "regression" && !regressionBootstrap ? 1 : source.workers;
  const preprocessing = kind === "nca" || kind === "regression"
    ? "unstandardized"
    : kind === "pca" || kind === "cbsem" || kind === "gsca"
      ? "standardized"
      : (source.preprocessing ?? "standardized");

  assertEnum("weightingScheme", weightingScheme, ["path", "factor", "pca"] as const);
  assertNumberInRange("tolerance", tolerance, NATIVE_ANALYSIS_RECIPE_BOUNDS.tolerance.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.tolerance.maximum);
  assertIntegerInRange("maxIterations", maxIterations, NATIVE_ANALYSIS_RECIPE_BOUNDS.maxIterations.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.maxIterations.maximum);
  assertIntegerInRange("seed", source.seed, NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.maximum);
  assertIntegerInRange("workers", workers, NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.maximum);
  assertNumberInRange("confidenceLevel", source.confidenceLevel, NATIVE_ANALYSIS_RECIPE_BOUNDS.confidenceLevel.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.confidenceLevel.maximum);
  assertEnum("preprocessing", preprocessing, ["standardized", "mean_centered", "unstandardized"] as const);

  if (["plsc", "wpls", "cca", "cta_pls", "endogeneity", "nonlinear_effects", "moderated_mediation"].includes(kind) && weightingScheme === "pca") {
    fail("weightingScheme", `${nativeAnalysisRecipeDescriptor(kind).label} requires path or factor weighting in its validated scope.`);
  }
  if (kind === "wpls" && preprocessing !== "standardized") {
    fail("preprocessing", "Weighted PLS requires standardized preprocessing in its validated scope.");
  }
  if (kind === "cca" && preprocessing !== "standardized") {
    fail("preprocessing", "CCA composite residual diagnostics require standardized preprocessing in the validated scope.");
  }
  if (kind === "ipma" && weightingScheme !== "path") {
    fail("weightingScheme", "Importance-Performance Map Analysis requires path weighting in the validated native scope.");
  }
  if (kind === "ipma" && preprocessing !== "standardized") {
    fail("preprocessing", "Importance-Performance Map Analysis requires standardized preprocessing in the validated native scope.");
  }

  let bootstrapSamples = 0;
  let studentizedInnerSamples = 0;
  let permutationSamples = 0;
  if (isPrimaryKind(kind)) {
    const normalized = nativeCalculationSettingsForMode(source, calculationModeForKind(kind));
    bootstrapSamples = normalized.bootstrapSamples;
    studentizedInnerSamples = normalized.studentizedInnerSamples;
    permutationSamples = normalized.permutationSamples;
    if (kind === "pls_bootstrap") validateBootstrapPlan(bootstrapSamples, studentizedInnerSamples);
  } else if (regressionBootstrap) {
    bootstrapSamples = source.bootstrapSamples;
    assertIntegerInRange(
      "bootstrapSamples",
      bootstrapSamples,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum,
    );
    if (source.studentizedInnerSamples !== 0 || source.permutationSamples !== 0) {
      fail("bootstrapSamples", "Regression case-resampling cannot be combined with studentized or permutation settings.");
    }
    if (source.confidenceLevel !== 0.95) {
      fail("confidenceLevel", "The qualified regression bootstrap slice uses fixed two-sided 95% inference.");
    }
  }

  const caseWeightColumn = kind === "wpls"
    ? requiredText("caseWeightColumn", source.caseWeightColumn)
    : null;

  return {
    method,
    weighting_scheme: weightingScheme,
    tolerance,
    max_iterations: maxIterations,
    bootstrap_samples: bootstrapSamples,
    studentized_inner_samples: studentizedInnerSamples,
    permutation_samples: permutationSamples,
    seed: source.seed,
    workers: kind === "cbsem" || kind === "gsca" ? 1 : workers,
    confidence_level: source.confidenceLevel,
    preprocessing,
    missing_data: "listwise_deletion",
    case_weight_column: caseWeightColumn,
  };
}

function validateGscaModel(model: NativeRecipeModel, edges: readonly Edge[]) {
  if (model.constructs.length < 2) {
    fail("model", "GSCA requires at least two component constructs.");
  }
  if (!model.paths.length) {
    fail("model", "GSCA requires at least one recursive structural path.");
  }
  if (model.constructs.some((construct) => construct.indicators.length === 0)) {
    fail("model", "Every GSCA construct requires at least one observed indicator.");
  }
  if (model.controls.length || model.interactions.length || model.higher_order_constructs.length) {
    fail("model", "The bounded GSCA scope does not support controls, interactions, or higher-order constructs.");
  }
  if (edges.some((edge) => (edge.data as PathEdgeData | undefined)?.role === "covariance")) {
    fail("model", "The bounded GSCA scope does not support covariance paths.");
  }
  const connected = new Set(model.paths.flatMap((path) => [path.source, path.target]));
  if (model.constructs.some((construct) => !connected.has(construct.id))) {
    fail("model", "Every GSCA construct must participate in the recursive structural model.");
  }
  const indegree = new Map(model.constructs.map((construct) => [construct.id, 0]));
  const outgoing = new Map(model.constructs.map((construct) => [construct.id, [] as string[]]));
  for (const path of model.paths) {
    indegree.set(path.target, (indegree.get(path.target) ?? 0) + 1);
    outgoing.get(path.source)?.push(path.target);
  }
  const pending = [...indegree].filter(([, degree]) => degree === 0).map(([id]) => id);
  let visited = 0;
  while (pending.length) {
    const source = pending.shift()!;
    visited += 1;
    for (const target of outgoing.get(source) ?? []) {
      const degree = (indegree.get(target) ?? 0) - 1;
      indegree.set(target, degree);
      if (degree === 0) pending.push(target);
    }
  }
  if (visited !== model.constructs.length) {
    fail("model", "GSCA requires a recursive acyclic structural model.");
  }
}

function buildMetadata(
  scopeMetadata: string,
  methodConfig: NativeAnalysisMethodConfig,
): Record<string, string> {
  const status = methodConfig.kind === "predict" && methodConfig.fimix
    ? "preview_fimix_pls_v1_bounded_score_space_diagnostic"
    : methodConfig.kind === "predict" && methodConfig.pls_pos
      ? "preview_pls_pos_v1_bounded_score_space_diagnostic"
      : methodConfig.kind !== "regression"
    ? scopeMetadata
    : methodConfig.model.type === "process"
      ? methodConfig.bootstrap
        ? "candidate_regression_process_v2_plus_bootstrap_v1_bounded_scope"
        : "candidate_regression_process_v2_bounded_scope"
    : methodConfig.bootstrap
      ? "validated_regression_bootstrap_v1_bounded_scope"
      : methodConfig.model.type === "logistic"
        ? "validated_regression_logistic_v2_bounded_scope"
        : scopeMetadata;
  return { status };
}

function buildMethodConfig(
  kind: NativeAnalysisRecipeKind,
  settings: Readonly<AnalysisUiSettings>,
): NativeAnalysisMethodConfig {
  switch (kind) {
    case "pls_algorithm":
    case "pls_bootstrap":
    case "pls_permutation":
    case "plsc":
    case "wpls":
    case "cca":
    case "cta_pls":
    case "endogeneity":
    case "nonlinear_effects":
    case "moderated_mediation":
    case "gsca":
      return { kind };
    case "predict": {
      const values = predictionMetadata(settings);
      const segmentation = values.segment_starts === undefined
        ? undefined
        : {
            segments: Number(values.segment_count ?? values.fimix_classes),
            starts: Number(values.segment_starts),
            minimum_segment_share: Number(values.minimum_segment_share),
          };
      return {
        kind,
        ...(values.segment_count && segmentation ? { pls_pos: segmentation } : {}),
        ...(values.fimix_classes && segmentation ? { fimix: segmentation } : {}),
      };
    }
    case "mga": {
      const values = mgaMetadata(settings);
      return {
        kind,
        group_column: values.mga_group_column,
        group_a: values.mga_group_a,
        group_b: values.mga_group_b,
        methods: ["micom", "mga_permutation"],
        permutation_samples: Number(values.group_permutation_samples),
        configural_invariance_confirmed: values.micom_configural_confirmed === "true",
      };
    }
    case "ipma":
      return { kind, targets: [requiredSingleTarget(settings.ipmaTargets)] };
    case "cbsem": {
      const values = cbsemMetadata(settings);
      return {
        kind,
        model_type: values.cbsem_model_type as "cfa" | "sem",
        estimator: "ml",
        input: "raw",
        mean_structure: false,
        bootstrap_samples: 0,
      };
    }
    case "pca": {
      const values = pcaMetadata(settings);
      const rule = values.pca_component_rule as "kaiser" | "fixed" | "variance_threshold";
      const retention = rule === "fixed"
        ? { rule, components: Number(values.pca_components) } as const
        : rule === "variance_threshold"
          ? { rule, threshold: Number(values.pca_variance_threshold) } as const
          : { rule } as const;
      return { kind, variables: values.pca_variables.split(","), retention };
    }
    case "regression": {
      if (settings.regressionType === "process") {
        const assessment = nativeProcessGraphAssessment(settings);
        if (!assessment.canRun || !assessment.graph) {
          fail("processGraph", assessment.blockers[0] ?? "Define a supported graph-based PROCESS relationship.");
        }
        if (assessment.equationTermCounts.some((equation) => equation.terms > NATIVE_PROCESS_MAX_EQUATION_TERMS)) {
          fail("processGraph", `Each PROCESS equation supports at most ${NATIVE_PROCESS_MAX_EQUATION_TERMS} non-intercept terms.`);
        }
        return {
          kind,
          outcome: assessment.outcome,
          predictors: assessment.predictors,
          ...(assessment.controls.length ? { controls: assessment.controls } : {}),
          model: { type: "process", relationship: assessment.graph },
          ...(settings.regressionBootstrap === true ? {
            bootstrap: {
              algorithm: "case_resampling",
              intervals: ["percentile", "bca"],
            } as const,
          } : {}),
        };
      }
      const values = regressionMetadata(settings);
      const regressionType = values.regression_type as "ols" | "logistic";
      const model = regressionType === "ols"
        ? { type: "ols", robust_se: "hc3" } as const
        : { type: "logistic" } as const;
      return {
        kind,
        outcome: values.regression_outcome,
        predictors: values.regression_predictors.split(","),
        ...(values.regression_controls ? { controls: values.regression_controls.split(",") } : {}),
        model,
        ...(settings.regressionBootstrap === true ? {
          bootstrap: {
            algorithm: "case_resampling",
            intervals: ["percentile", "bca"],
          } as const,
        } : {}),
      };
    }
    case "nca": {
      const values = ncaMetadata(settings);
      return {
        kind,
        condition: values.nca_x,
        outcome: values.nca_y,
        ceiling: values.nca_ceiling as "ce_fdh" | "cr_fdh" | "both",
        permutation_samples: Number(values.nca_permutation_samples),
      };
    }
  }
}

function predictionMetadata(settings: Readonly<AnalysisUiSettings>): Record<string, string> {
  const requested = methodTokens(settings.groupMethods, ["pls_pos", "fimix"]);
  if (!requested.length) return {};

  const segmentCount = settings.segmentCount ?? 2;
  const segmentStarts = settings.segmentStarts ?? 10;
  const minimumShare = settings.minimumSegmentShare ?? 0.1;
  assertIntegerInRange("segmentCount", segmentCount, NATIVE_ANALYSIS_RECIPE_BOUNDS.segmentCount.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.segmentCount.maximum);
  assertIntegerInRange("segmentStarts", segmentStarts, NATIVE_ANALYSIS_RECIPE_BOUNDS.segmentStarts.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.segmentStarts.maximum);
  assertNumberInRange("minimumSegmentShare", minimumShare, NATIVE_ANALYSIS_RECIPE_BOUNDS.minimumSegmentShare.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.minimumSegmentShare.maximum);
  if (requested.includes("fimix")) {
    assertIntegerInRange("segmentCount", segmentCount, NATIVE_ANALYSIS_RECIPE_BOUNDS.fimixClassCount.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.fimixClassCount.maximum);
  }

  return {
    // Rust activates PLS-POS through segment_count. Only the `fimix` token is
    // consumed from group_methods, so never serialize the dormant `pls_pos`
    // token from the legacy TopBar mapper.
    ...(requested.includes("fimix") ? { group_methods: "fimix" } : {}),
    ...(requested.includes("pls_pos") ? { segment_count: String(segmentCount) } : {}),
    ...(requested.includes("fimix") ? { fimix_classes: String(segmentCount) } : {}),
    segment_starts: String(segmentStarts),
    minimum_segment_share: String(minimumShare),
  };
}

function mgaMetadata(settings: Readonly<AnalysisUiSettings>): Record<string, string> {
  const groupColumn = requiredText("groupColumn", settings.groupColumn);
  const groupA = requiredText("groupAValue", settings.groupAValue);
  const groupB = requiredText("groupBValue", settings.groupBValue);
  if (groupA === groupB) fail("groupBValue", "Group A and Group B must use different observed values.");
  const samples = settings.groupPermutationSamples ?? NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.default;
  assertIntegerInRange("groupPermutationSamples", samples, NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.maximum);
  const methods = methodTokens(settings.groupMethods, ["micom", "mga_permutation"]);
  if (methods.length !== 2 || !methods.includes("micom") || !methods.includes("mga_permutation")) {
    fail("groupMethods", "The validated native group workflow requires MICOM and two-group permutation MGA together.");
  }
  if (settings.micomConfiguralConfirmed !== true) {
    fail("micomConfiguralConfirmed", "Confirm MICOM configural invariance before starting the group analysis.");
  }
  return {
    mga_group_column: groupColumn,
    mga_group_a: groupA,
    mga_group_b: groupB,
    group_methods: "micom,mga_permutation",
    group_permutation_samples: String(samples),
    micom_configural_confirmed: "true",
  };
}

function cbsemMetadata(settings: Readonly<AnalysisUiSettings>): Record<string, string> {
  const modelType = settings.cbsemModelType ?? "sem";
  assertEnum("cbsemModelType", modelType, ["cfa", "sem"] as const);

  // Rust currently ignores this UI field. Rejecting the non-default variant is
  // safer than serializing a setting that would not change the calculation.
  if (settings.cbsemStandardization && settings.cbsemStandardization !== "std_all") {
    fail("cbsemStandardization", "The validated native CB-SEM scope does not implement selectable standardization.");
  }
  if (settings.cbsemMeanStructure) {
    fail("cbsemMeanStructure", "The validated native CB-SEM scope does not estimate a selectable mean structure.");
  }
  if (optionalText(settings.cbsemGroupColumn)) {
    fail("cbsemGroupColumn", "CB-SEM multigroup/invariance remains outside the validated native single-group scope.");
  }
  const bootstrapSamples = settings.cbsemBootstrapSamples ?? 0;
  assertIntegerInRange("cbsemBootstrapSamples", bootstrapSamples, NATIVE_ANALYSIS_RECIPE_BOUNDS.cbsemBootstrapSamples.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.cbsemBootstrapSamples.maximum);
  if (bootstrapSamples > 0) {
    fail("cbsemBootstrapSamples", "CB-SEM bootstrap remains outside the validated native single-group scope.");
  }

  return {
    cbsem_model_type: modelType,
    cbsem_estimator: "ml",
    cbsem_input: "raw",
    cbsem_mean_structure: "false",
  };
}

function validateCbsemModel(model: NativeRecipeModel, modelType: string | undefined) {
  if (modelType !== "cfa" && modelType !== "sem") {
    fail("cbsemModelType", "Choose confirmatory factor analysis or structural equation modeling.");
  }
  if (!model.constructs.length) {
    fail("model", "CB-SEM / CFA requires at least one reflective latent construct.");
  }
  const nonReflective = model.constructs.filter((construct) => construct.mode !== "reflective");
  if (nonReflective.length) {
    fail("model", "The bounded native CB-SEM / CFA scope supports reflective constructs only.");
  }
  const underspecified = model.constructs.filter((construct) => construct.indicators.length < 2);
  if (underspecified.length) {
    fail("model", "Each CB-SEM / CFA latent factor requires at least two observed indicators.");
  }
  if (model.controls.length || model.interactions.length || model.higher_order_constructs.length) {
    fail("model", "The bounded native CB-SEM / CFA scope does not support controls, interactions, or higher-order constructs.");
  }
  if (modelType === "cfa" && model.paths.length) {
    fail("cbsemModelType", "Confirmatory factor analysis does not accept structural paths.");
  }
  if (modelType === "sem" && !model.paths.length) {
    fail("cbsemModelType", "Structural equation modeling requires at least one recursive latent path.");
  }
}

function pcaMetadata(settings: Readonly<AnalysisUiSettings>): Record<string, string> {
  const componentRule = settings.pcaComponentRule ?? "kaiser";
  assertEnum("pcaComponentRule", componentRule, ["kaiser", "fixed", "variance_threshold"] as const);
  const rawSelectedVariables = typeof settings.pcaVariables === "string"
    ? settings.pcaVariables.split(",").map((variable) => variable.trim()).filter(Boolean)
    : [];
  if (new Set(rawSelectedVariables).size !== rawSelectedVariables.length) {
    fail("pcaVariables", "Standalone PCA requires distinct selected variables.");
  }
  const variables = requiredCsv("pcaVariables", settings.pcaVariables);
  if (variables.split(",").length < 2) {
    fail("pcaVariables", "Standalone PCA requires at least two selected variables.");
  }
  const selectedVariables = variables.split(",");
  if (selectedVariables.length > 50) {
    fail("pcaVariables", "Standalone PCA supports at most 50 selected variables in the validated native scope.");
  }
  const metadata: Record<string, string> = {
    pca_variables: variables,
    pca_component_rule: componentRule,
  };
  if (componentRule === "fixed") {
    const components = settings.pcaComponents ?? 2;
    assertIntegerInRange("pcaComponents", components, NATIVE_ANALYSIS_RECIPE_BOUNDS.pcaComponents.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.pcaComponents.maximum);
    if (components > selectedVariables.length) {
      fail("pcaComponents", "Fixed PCA components cannot exceed the number of selected variables.");
    }
    metadata.pca_components = String(components);
  }
  if (componentRule === "variance_threshold") {
    const threshold = settings.pcaVarianceThreshold ?? 0.80;
    assertNumberInRange("pcaVarianceThreshold", threshold, 0.01, 0.999);
    metadata.pca_variance_threshold = String(threshold);
  }
  return metadata;
}

function regressionMetadata(settings: Readonly<AnalysisUiSettings>): Record<string, string> {
  const regressionType = settings.regressionType ?? "ols";
  assertEnum("regressionType", regressionType, ["ols", "logistic", "process"] as const);
  if (regressionType === "process") {
    fail("processGraph", "Graph-defined PROCESS configuration must be serialized through its typed relationship contract.");
  }
  const outcome = requiredText("regressionOutcome", settings.regressionOutcome);
  const predictors = requiredCsv("regressionPredictors", settings.regressionPredictors);
  const controls = optionalCsv(settings.regressionControls);
  const metadata: Record<string, string> = {
    regression_type: regressionType,
    regression_outcome: outcome,
    regression_predictors: predictors,
    ...(controls ? { regression_controls: controls } : {}),
    ...(regressionType === "ols" ? { robust_se: "hc3" } : {}),
  };

  if (regressionType === "ols" && settings.robustSe && settings.robustSe !== "hc3") {
    fail("robustSe", "The validated native OLS scope computes HC3 standard errors; the selected alternative is not implemented.");
  }
  if (settings.regressionBootstrap === true) {
    const selectedTermCount = predictors.split(",").length + (controls?.split(",").length ?? 0);
    if (selectedTermCount > NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS) {
      fail(
        "regressionPredictors",
        `Regression bootstrap supports at most ${NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS} predictors and controls (${NATIVE_REGRESSION_BOOTSTRAP_MAX_SELECTED_TERMS + 1} coefficient terms including the intercept).`,
      );
    }
  }
  return metadata;
}

function ncaMetadata(settings: Readonly<AnalysisUiSettings>): Record<string, string> {
  const ceiling = settings.ncaCeiling ?? "both";
  assertEnum("ncaCeiling", ceiling, ["ce_fdh", "cr_fdh", "both"] as const);
  const permutations = settings.ncaPermutationSamples ?? 999;
  assertIntegerInRange("ncaPermutationSamples", permutations, NATIVE_ANALYSIS_RECIPE_BOUNDS.ncaPermutationSamples.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.ncaPermutationSamples.maximum);
  const x = requiredText("ncaX", settings.ncaX);
  const y = requiredText("ncaY", settings.ncaY);
  if (x === y) fail("ncaY", "Necessary Condition Analysis requires different X and Y variables.");
  return {
    nca_x: x,
    nca_y: y,
    nca_ceiling: ceiling,
    nca_permutation_samples: String(permutations),
  };
}

export function buildNativeRecipeModel(
  modelId: string,
  projectName: string,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
): NativeRecipeModel {
  const isMeasurementEdge = (edge: Edge) => edge.id.startsWith("measurement::");
  const role = (edge: Edge) => (edge.data as PathEdgeData | undefined)?.role;
  const structuralEdges = edges.filter((edge) => !isMeasurementEdge(edge) && role(edge) !== "covariance");
  const controlEdges = edges.filter((edge) => !isMeasurementEdge(edge) && role(edge) === "control");
  const structuralPaths = [...new Map(structuralEdges.map((edge) => [
    `${edge.source}\u0000${edge.target}`,
    { source: edge.source, target: edge.target },
  ])).values()];

  return {
    id: modelId,
    name: projectName.trim(),
    constructs: nodes.map((node) => ({
      id: node.id,
      name: node.data.label,
      short_name: node.data.shortName,
      mode: node.data.mode,
      indicators: [...node.data.indicators],
    })),
    paths: structuralPaths,
    controls: controlEdges.map((edge) => ({
      source: edge.source,
      target: edge.target,
      label: optionalText((edge.data as PathEdgeData | undefined)?.controlLabel),
    })),
    higher_order_constructs: nodes
      .filter((node) => node.data.semantic === "higher_order" && node.data.higherOrder)
      .map((node) => ({
        id: node.id,
        components: [...node.data.higherOrder!.components],
        method: node.data.higherOrder!.method,
        stage_one_recipe: optionalText(node.data.higherOrder!.stage_one_recipe),
      })),
    interactions: nodes
      .filter((node) => node.data.semantic === "interaction" && node.data.interaction)
      .map((node) => ({
        id: node.id,
        predictor: node.data.interaction!.predictor,
        moderator: node.data.interaction!.moderator,
        product_construct: node.id,
        outcome: node.data.interaction!.outcome,
        method: node.data.interaction!.method,
      })),
  };
}

function validateIdentity(input: NativeAnalysisRecipeBuildInput) {
  assertUuid("recipeId", input.recipeId);
  assertUuid("modelId", input.modelId);
  const rfc3339 = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/;
  if (!rfc3339.test(input.createdAt) || !Number.isFinite(Date.parse(input.createdAt))) {
    fail("createdAt", "createdAt must be a valid RFC 3339 timestamp.");
  }
  requiredText("datasetFingerprint", input.datasetFingerprint);
  requiredText("projectName", input.projectName);
}

function validateCcaModel(model: NativeRecipeModel) {
  if (model.constructs.length < 2) {
    fail("model", "CCA composite residual diagnostics require at least two constructs.");
  }
  if (model.constructs.some((construct) => construct.mode !== "reflective")) {
    fail("model", "CCA composite residual diagnostics require reflective constructs.");
  }
  if (model.constructs.some((construct) => construct.indicators.length === 0)) {
    fail("model", "Every CCA construct requires at least one observed indicator.");
  }
  if (model.paths.length === 0) {
    fail("model", "CCA composite residual diagnostics require at least one structural path.");
  }
  if (model.controls.length > 0) {
    fail("model", "CCA composite residual diagnostics do not support control paths in the validated scope.");
  }
  if (model.interactions.length > 0 || model.higher_order_constructs.length > 0) {
    fail("model", "CCA composite residual diagnostics do not support interaction or higher-order constructs in the validated scope.");
  }
}

function validateCtaPlsModel(model: NativeRecipeModel) {
  if (!model.constructs.some((construct) => construct.indicators.length >= 4)) {
    fail("model", "CTA-PLS requires at least one ordinary construct with four or more indicators.");
  }
  if (model.controls.length > 0) {
    fail("model", "CTA-PLS does not support control paths in the bounded native descriptive scope.");
  }
  if (model.interactions.length > 0 || model.higher_order_constructs.length > 0) {
    fail("model", "CTA-PLS does not support interaction or higher-order constructs in the bounded native descriptive scope.");
  }
}

function validateIpmaModel(
  model: NativeRecipeModel,
  targetId: string | undefined,
  nodes: readonly Node<ConstructData>[],
  edges: readonly Edge[],
) {
  if (!targetId || !model.constructs.some((construct) => construct.id === targetId)) {
    fail("ipmaTargets", "Choose one endogenous construct from the active model.");
  }
  if (!nativeIpmaTargetOptions(nodes, edges).some((target) => target.id === targetId)) {
    fail("ipmaTargets", "Importance-Performance Map Analysis requires an endogenous target with at least one incoming structural path.");
  }
  if (model.interactions.length > 0 || model.higher_order_constructs.length > 0) {
    fail("model", "Importance-Performance Map Analysis does not support interaction or higher-order constructs in the validated native scope.");
  }
}

function validateBootstrapPlan(bootstrapSamples: number, studentizedInnerSamples: number) {
  assertIntegerInRange("bootstrapSamples", bootstrapSamples, NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.bootstrapSamples.maximum);
  if (studentizedInnerSamples === 0) return;
  assertIntegerInRange("studentizedInnerSamples", studentizedInnerSamples, NATIVE_ANALYSIS_RECIPE_BOUNDS.studentizedInnerSamples.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.studentizedInnerSamples.maximum);
  if (studentizedInnerSamples % 2 === 0) fail("studentizedInnerSamples", "Studentized bootstrap inner samples must be odd.");
  if (bootstrapSamples < 999) fail("bootstrapSamples", "Studentized bootstrap requires at least 999 primary bootstrap samples.");
}

function isPrimaryKind(kind: NativeAnalysisRecipeKind): kind is "pls_algorithm" | "pls_bootstrap" | "pls_permutation" {
  return kind === "pls_algorithm" || kind === "pls_bootstrap" || kind === "pls_permutation";
}

function calculationModeForKind(kind: "pls_algorithm" | "pls_bootstrap" | "pls_permutation"): NativeCalculationMode {
  if (kind === "pls_bootstrap") return "bootstrap";
  if (kind === "pls_permutation") return "permutation";
  return "pls";
}

function methodTokens(value: string | null | undefined, allowed: readonly string[]): string[] {
  const selected = optionalCsv(value)?.split(",") ?? [];
  const known = new Set(["pls_pos", "fimix", "micom", "mga_permutation"]);
  const unknown = selected.filter((item) => !known.has(item));
  if (unknown.length) fail("groupMethods", `Unknown group workflow token(s): ${unknown.join(", ")}.`);
  return [...new Set(selected.filter((item) => allowed.includes(item)))];
}

function optionalCsv(value: string | null | undefined): string | null {
  if (typeof value !== "string") return null;
  const items = value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
  return items.length ? [...new Set(items)].join(",") : null;
}

function requiredCsv(field: string, value: string | null | undefined): string {
  const normalized = optionalCsv(value);
  if (!normalized) fail(field, `${field} is required.`);
  return normalized;
}

function requiredSingleTarget(value: string | null | undefined): string {
  const normalized = requiredCsv("ipmaTargets", value);
  const targets = normalized.split(",");
  if (targets.length !== 1) fail("ipmaTargets", "Choose exactly one endogenous target construct.");
  return targets[0];
}

function optionalText(value: string | null | undefined): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function requiredText(field: string, value: string | null | undefined): string {
  const normalized = optionalText(value);
  if (!normalized) fail(field, `${field} is required.`);
  return normalized;
}

function assertUuid(field: string, value: string) {
  if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)) {
    fail(field, `${field} must be a valid UUID.`);
  }
}

function assertIntegerInRange(field: string, value: number, minimum: number, maximum: number) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    fail(field, `${field} must be an integer between ${minimum} and ${maximum}.`);
  }
}

function assertNumberInRange(field: string, value: number, minimum: number, maximum: number) {
  if (!Number.isFinite(value) || value < minimum || value > maximum) {
    fail(field, `${field} must be between ${minimum} and ${maximum}.`);
  }
}

function assertEnum<const T extends string>(field: string, value: string, allowed: readonly T[]): asserts value is T {
  if (!allowed.includes(value as T)) fail(field, `${field} must be one of: ${allowed.join(", ")}.`);
}

function fail(field: string, message: string): never {
  throw new NativeAnalysisRecipeBuildError(field, message);
}
