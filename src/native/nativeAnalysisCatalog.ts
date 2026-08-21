import type { AnalysisUiSettings } from "../types";
import {
  NATIVE_ANALYSIS_RECIPE_BOUNDS,
  nativeAnalysisRecipeDescriptor,
  nativeAnalysisRecipeKindForSettings,
} from "./nativeAnalysisRecipe";
import {
  NATIVE_PREDICTION_SCOPE_DESCRIPTION,
  nativeCalculationSettingsForMode,
} from "./nativeCalculationMode";
import { nativeProcessGraphAssessment, parseNativeProcessGraph } from "./nativeProcess";

export type NativeWorkbenchAnalysisKind =
  | "pls_algorithm"
  | "plsc"
  | "plsc_bootstrap"
  | "wpls"
  | "gsca"
  | "cca"
  | "cta_pls"
  | "ipma"
  | "cbsem"
  | "pls_bootstrap"
  | "pls_permutation"
  | "pls_posthoc_technical_minimum_sample_size"
  | "pls_sample_size_power"
  | "mga"
  | "predict"
  | "nca"
  | "pca"
  | "regression";

export type NativeAnalysisCategoryId = "estimation" | "component_models" | "assessment" | "covariance" | "inference" | "groups" | "prediction" | "standalone";

/**
 * The established Calculate catalogue that shipped before capability-evidence
 * filtering was introduced. These workflows have native implementations and
 * remain selectable in Experimental Labs; Registry V2 controls qualification
 * claims, not whether an implemented bounded workflow disappears from the UI.
 */
export const NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1 = [
  "pls_algorithm",
  "plsc",
  "wpls",
  "gsca",
  "cca",
  "ipma",
  "cbsem",
  "pls_bootstrap",
  "pls_permutation",
  "mga",
  "predict",
  "nca",
  "pca",
  "regression",
] as const satisfies readonly NativeWorkbenchAnalysisKind[];

const ESTABLISHED_WORKING_ANALYSIS_KINDS = new Set<NativeWorkbenchAnalysisKind>(
  NATIVE_ESTABLISHED_WORKING_ANALYSIS_KINDS_V1,
);

export function isNativeEstablishedWorkingAnalysisKindV1(
  kind: NativeWorkbenchAnalysisKind,
): boolean {
  return ESTABLISHED_WORKING_ANALYSIS_KINDS.has(kind);
}

export type NativeAnalysisCapabilityId =
  | "qpls3.pls.algorithm"
  | "qpls3.pls.consistent"
  | "qpls3.pls.weighted"
  | "qpls3.gsca.als"
  | "qpls3.assessment.cca_residuals"
  | "qpls3.assessment.cta_pls"
  | "qpls3.assessment.ipma"
  | "qpls3.cbsem.ml"
  | "qpls3.cbsem.bootstrap"
  | "qpls3.inference.bootstrap"
  | "qpls3.inference.consistent_bootstrap"
  | "qpls3.inference.structural_path_randomization"
  | "qpls3.pls.posthoc_technical_minimum_sample_size"
  | "qpls3.pls.sample_size_power"
  | "qpls3.groups.micom_permutation_mga"
  | "qpls3.prediction.plspredict_cvpat"
  | "qpls3.standalone.nca"
  | "qpls3.standalone.pca"
  | "qpls3.standalone.ols"
  | "qpls3.standalone.logistic"
  | "qpls3.standalone.regression_bootstrap"
  | "qpls3.standalone.process";

export interface NativeAnalysisCatalogItem {
  kind: NativeWorkbenchAnalysisKind;
  categoryId: NativeAnalysisCategoryId;
  categoryLabel: string;
  label: string;
  description: string;
  keywords: readonly string[];
  /** Stable machine-audit linkage; not rendered as user-facing availability. */
  capabilityIds: readonly NativeAnalysisCapabilityId[];
}

interface CatalogItemDraft extends Omit<NativeAnalysisCatalogItem, "label"> {
  kind: NativeWorkbenchAnalysisKind;
}

const CATALOG_DRAFTS: readonly CatalogItemDraft[] = [
  {
    kind: "pls_algorithm",
    categoryId: "estimation",
    categoryLabel: "PLS-SEM estimation",
    description: "Estimate composite scores, paths, loadings, weights, and model quality.",
    keywords: ["pls", "algorithm", "composite", "path modeling"],
    capabilityIds: ["qpls3.pls.algorithm"],
  },
  {
    kind: "plsc",
    categoryId: "estimation",
    categoryLabel: "PLS-SEM estimation",
    description: "Apply consistent PLS correction to reflective measurement models.",
    keywords: ["plsc", "consistent pls", "reflective", "correction"],
    capabilityIds: ["qpls3.pls.consistent"],
  },
  {
    kind: "wpls",
    categoryId: "estimation",
    categoryLabel: "PLS-SEM estimation",
    description: "Estimate a reflective PLS model using positive case weights.",
    keywords: ["wpls", "weighted pls", "case weights", "survey weights"],
    capabilityIds: ["qpls3.pls.weighted"],
  },
  {
    kind: "gsca",
    categoryId: "component_models",
    categoryLabel: "Component models",
    description: "Estimate a generalized structured component model with reflective or formative blocks and recursive structural paths.",
    keywords: ["gsca", "generalized structured component analysis", "component model", "alternating least squares", "als"],
    capabilityIds: ["qpls3.gsca.als"],
  },
  {
    kind: "cca",
    categoryId: "assessment",
    categoryLabel: "Assessment",
    description: "Inspect descriptive residuals between observed and model-reproduced composite correlations.",
    keywords: ["cca", "composite residual", "residual diagnostics", "confirmatory composite analysis", "assessment"],
    capabilityIds: ["qpls3.assessment.cca_residuals"],
  },
  {
    kind: "cta_pls",
    categoryId: "assessment",
    categoryLabel: "Assessment",
    description: "Inspect every descriptive sample-covariance tetrad for eligible four-or-more-indicator PLS blocks without inferential classification.",
    keywords: ["cta-pls", "cta pls", "confirmatory tetrad", "tetrad", "measurement model", "assessment"],
    capabilityIds: ["qpls3.assessment.cta_pls"],
  },
  {
    kind: "ipma",
    categoryId: "assessment",
    categoryLabel: "Assessment",
    description: "Map each structural predecessor's total importance against observed-range construct performance for one endogenous target.",
    keywords: ["ipma", "importance performance", "priority map", "target", "assessment"],
    capabilityIds: ["qpls3.assessment.ipma"],
  },
  {
    kind: "cbsem",
    categoryId: "covariance",
    categoryLabel: "Covariance-based SEM",
    description: "Estimate single-group reflective CFA or recursive latent SEM with maximum likelihood and optional exact case bootstrap.",
    keywords: ["cbsem", "cb-sem", "cfa", "confirmatory factor analysis", "maximum likelihood", "covariance", "model fit", "case bootstrap", "percentile interval"],
    capabilityIds: ["qpls3.cbsem.ml", "qpls3.cbsem.bootstrap"],
  },
  {
    kind: "pls_bootstrap",
    categoryId: "inference",
    categoryLabel: "Inference",
    description: "Estimate confidence intervals and significance with deterministic resampling.",
    keywords: ["bootstrap", "confidence interval", "significance", "inference"],
    capabilityIds: ["qpls3.inference.bootstrap"],
  },
  {
    kind: "plsc_bootstrap",
    categoryId: "inference",
    categoryLabel: "Inference",
    description: "Fully re-estimate consistent PLS for each indexed case resample and report percentile plus conditional BCa inference.",
    keywords: ["plsc bootstrap", "consistent bootstrap", "full re-estimation", "bca", "confidence interval", "inference"],
    capabilityIds: ["qpls3.inference.consistent_bootstrap"],
  },
  {
    kind: "pls_permutation",
    categoryId: "inference",
    categoryLabel: "Inference",
    description: "Run single-model Freedman-Lane randomization for structural paths using fixed original PLS construct scores and unadjusted pathwise p values within the documented scope.",
    keywords: ["freedman lane", "permutation", "randomization", "path significance", "inference"],
    capabilityIds: ["qpls3.inference.structural_path_randomization"],
  },
  {
    kind: "pls_posthoc_technical_minimum_sample_size",
    categoryId: "inference",
    categoryLabel: "Inference",
    description: "Retrospective inverse-square-root technical minimum sample-size result, using the weakest statistically significant structural path from linked PLS bootstrap inference.",
    keywords: ["post hoc", "technical minimum sample size", "inverse square root", "bootstrap", "power", "retrospective"],
    capabilityIds: ["qpls3.pls.posthoc_technical_minimum_sample_size"],
  },
  {
    kind: "pls_sample_size_power",
    categoryId: "inference",
    categoryLabel: "Inference",
    description: "Prospective Monte Carlo power for exactly one two-construct reflective Gaussian path, evaluated only on an explicit sample-size grid.",
    keywords: ["power", "sample size", "monte carlo", "prospective", "simulation", "wilson", "gaussian", "reflective", "inference"],
    capabilityIds: ["qpls3.pls.sample_size_power"],
  },
  {
    kind: "mga",
    categoryId: "groups",
    categoryLabel: "Groups",
    description: "Assess the three MICOM measurement-invariance steps using an explicit researcher-confirmed configural review, then compare Group A minus Group B paths, loadings, and weights with one deterministic size-preserving permutation plan.",
    keywords: ["micom", "measurement invariance", "configural invariance", "compositional invariance", "group a", "group b", "permutation", "inference"],
    capabilityIds: ["qpls3.groups.micom_permutation_mga"],
  },
  {
    kind: "predict",
    categoryId: "prediction",
    categoryLabel: "Prediction",
    description: NATIVE_PREDICTION_SCOPE_DESCRIPTION,
    keywords: ["plspredict", "cvpat", "prediction", "indicator", "indicator average", "linear model", "holdout", "cross validation"],
    capabilityIds: ["qpls3.prediction.plspredict_cvpat"],
  },
  {
    kind: "nca",
    categoryId: "standalone",
    categoryLabel: "Standalone analysis",
    description: "Analyze whether one numeric observed condition is necessary for one numeric observed outcome using ceiling lines and bottlenecks.",
    keywords: ["nca", "necessary condition", "ce-fdh", "cr-fdh", "ceiling", "bottleneck", "observed variable"],
    capabilityIds: ["qpls3.standalone.nca"],
  },
  {
    kind: "pca",
    categoryId: "standalone",
    categoryLabel: "Standalone analysis",
    description: "Reduce selected numeric variables to orthogonal principal components using a correlation-matrix eigensystem.",
    keywords: ["pca", "principal component", "dimension reduction", "eigenvalue", "kaiser", "variance", "observed variable"],
    capabilityIds: ["qpls3.standalone.pca"],
  },
  {
    kind: "regression",
    categoryId: "standalone",
    categoryLabel: "Standalone analysis",
    description: "Fit raw numeric OLS, strict 0/1 binary logistic regression, or graph-defined path analysis with mediation and moderation as a model-free observed-variable analysis.",
    keywords: ["ols", "ordinary least squares", "linear regression", "logistic", "binary", "odds ratio", "wald", "probability", "hc3", "case bootstrap", "percentile", "bca", "inference", "path analysis", "process", "mediation", "moderation", "johnson neyman", "observed variable"],
    capabilityIds: ["qpls3.standalone.ols", "qpls3.standalone.logistic", "qpls3.standalone.regression_bootstrap", "qpls3.standalone.process"],
  },
] as const;

export const NATIVE_ANALYSIS_CATALOG: readonly NativeAnalysisCatalogItem[] = CATALOG_DRAFTS.map((item) => ({
  ...item,
  label: item.kind === "regression"
    ? "Regression"
    : item.kind === "mga"
      ? "MICOM and Two-Group Permutation MGA"
      : nativeAnalysisRecipeDescriptor(item.kind).label,
}));

const workbenchKinds = new Set<string>(NATIVE_ANALYSIS_CATALOG.map((item) => item.kind));

const DEFAULT_NATIVE_SCALARS = {
  tolerance: 1e-7,
  maxIterations: 3_000,
  seed: 20_260_718,
  workers: 1,
  confidenceLevel: 0.95,
} as const;

function boundedNumber(
  value: number | undefined,
  fallback: number,
  minimum: number,
  maximum: number,
): number {
  const candidate = Number.isFinite(value) ? value! : fallback;
  return Math.min(maximum, Math.max(minimum, candidate));
}

function boundedInteger(
  value: number | undefined,
  fallback: number,
  minimum: number,
  maximum: number,
): number {
  return Math.trunc(boundedNumber(value, fallback, minimum, maximum));
}

function normalizedNativeRecipeScalars(
  settings: Readonly<AnalysisUiSettings>,
): AnalysisUiSettings {
  return {
    ...settings,
    tolerance: boundedNumber(
      settings.tolerance,
      DEFAULT_NATIVE_SCALARS.tolerance,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.tolerance.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.tolerance.maximum,
    ),
    maxIterations: boundedInteger(
      settings.maxIterations,
      DEFAULT_NATIVE_SCALARS.maxIterations,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.maxIterations.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.maxIterations.maximum,
    ),
    seed: boundedInteger(
      settings.seed,
      DEFAULT_NATIVE_SCALARS.seed,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.maximum,
    ),
    workers: boundedInteger(
      settings.workers,
      DEFAULT_NATIVE_SCALARS.workers,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.maximum,
    ),
    confidenceLevel: boundedNumber(
      settings.confidenceLevel,
      DEFAULT_NATIVE_SCALARS.confidenceLevel,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.confidenceLevel.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.confidenceLevel.maximum,
    ),
  };
}

export function isNativeWorkbenchAnalysisKind(kind: string): kind is NativeWorkbenchAnalysisKind {
  return workbenchKinds.has(kind);
}

export function nativeWorkbenchAnalysisKindForSettings(
  settings: Readonly<AnalysisUiSettings>,
): NativeWorkbenchAnalysisKind {
  const kind = nativeAnalysisRecipeKindForSettings(settings);
  return isNativeWorkbenchAnalysisKind(kind) ? kind : "pls_algorithm";
}

export function nativeAnalysisCatalogItem(kind: NativeWorkbenchAnalysisKind): NativeAnalysisCatalogItem {
  const item = NATIVE_ANALYSIS_CATALOG.find((candidate) => candidate.kind === kind);
  if (!item) throw new Error(`Unknown native workbench analysis: ${kind}`);
  return item;
}

export function filterNativeAnalysisCatalog(query: string): NativeAnalysisCatalogItem[] {
  const terms = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  if (!terms.length) return [...NATIVE_ANALYSIS_CATALOG];
  return NATIVE_ANALYSIS_CATALOG.filter((item) => {
    const haystack = [item.label, item.description, item.categoryLabel, ...item.keywords]
      .join(" ")
      .toLocaleLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
}

export function nativeAnalysisSettingsForWorkbenchKind(
  settings: Readonly<AnalysisUiSettings>,
  kind: NativeWorkbenchAnalysisKind,
): AnalysisUiSettings {
  const normalized = normalizedNativeRecipeScalars(settings);
  if (kind === "pls_algorithm" || kind === "pls_bootstrap" || kind === "pls_permutation" || kind === "pls_posthoc_technical_minimum_sample_size" || kind === "predict") {
    const mode = kind === "pls_bootstrap"
      ? "bootstrap"
      : kind === "pls_posthoc_technical_minimum_sample_size"
        ? "bootstrap"
      : kind === "pls_permutation"
        ? "permutation"
        : kind === "predict"
          ? "predict"
          : "pls";
    const selected = {
      ...nativeCalculationSettingsForMode(normalized, mode),
      caseWeightColumn: null,
    };
    return kind === "pls_posthoc_technical_minimum_sample_size"
      ? { ...selected, studentizedInnerSamples: 0, permutationSamples: 0 }
      : selected;
  }

  if (kind === "mga") {
    return {
      ...normalized,
      method: "mga",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      groupMethods: "micom,mga_permutation",
      groupPermutationSamples: boundedInteger(
        normalized.groupPermutationSamples,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.default,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.minimum,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.groupPermutationSamples.maximum,
      ),
      micomConfiguralConfirmed: normalized.micomConfiguralConfirmed === true,
    };
  }

  if (kind === "pls_sample_size_power") {
    return {
      ...normalized,
      method: "pls_sample_size_power",
      weightingScheme: "path",
      preprocessing: "standardized",
      tolerance: boundedNumber(normalized.tolerance, 1e-7, 1e-10, 1e-3),
      maxIterations: boundedInteger(normalized.maxIterations, 3_000, 100, 10_000),
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      caseWeightColumn: null,
      confidenceLevel: boundedNumber(normalized.confidenceLevel, 0.95, 0.80, 0.999),
      plsPowerScenarioIdentity: normalized.plsPowerScenarioIdentity?.trim() || "prospective_two_construct_path",
      plsPowerPredictorConstruct: normalized.plsPowerPredictorConstruct?.trim() || null,
      plsPowerOutcomeConstruct: normalized.plsPowerOutcomeConstruct?.trim() || null,
      plsPowerPredictorLoadings: normalized.plsPowerPredictorLoadings?.trim() || null,
      plsPowerOutcomeLoadings: normalized.plsPowerOutcomeLoadings?.trim() || null,
      plsPowerPopulationPath: boundedNumber(normalized.plsPowerPopulationPath, 0.30, -0.80, 0.80),
      plsPowerSampleSizeGrid: normalized.plsPowerSampleSizeGrid?.trim() || "50,100,150",
      plsPowerAlpha: boundedNumber(normalized.plsPowerAlpha, 0.05, 0.001, 0.10),
      plsPowerTargetPower: boundedNumber(normalized.plsPowerTargetPower, 0.80, 0.50, 0.99),
      // 3 grid points * 250 data sets * (1 + 199 fits) = 150,000 PLS fits.
      plsPowerMonteCarloReplicates: boundedInteger(normalized.plsPowerMonteCarloReplicates, 250, 100, 10_000),
      plsPowerBootstrapReplicates: (() => {
        const value = boundedInteger(normalized.plsPowerBootstrapReplicates, 199, 99, 1_999);
        return value % 2 === 0 ? Math.max(99, value - 1) : value;
      })(),
    };
  }

  if (kind === "ipma") {
    const targets = (normalized.ipmaTargets ?? "")
      .split(",")
      .map((target) => target.trim())
      .filter(Boolean);
    return {
      ...normalized,
      method: "ipma",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      ipmaTargets: targets.length === 1 ? targets[0] : null,
    };
  }

  if (kind === "cbsem") {
    const cbsemBootstrapSamples = boundedInteger(
      normalized.cbsemBootstrapSamples,
      0,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.cbsemBootstrapSamples.minimum,
      NATIVE_ANALYSIS_RECIPE_BOUNDS.cbsemBootstrapSamples.maximum,
    );
    return {
      ...normalized,
      method: "cbsem",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: cbsemBootstrapSamples > 0 ? normalized.workers : 1,
      caseWeightColumn: null,
      cbsemModelType: normalized.cbsemModelType === "cfa" ? "cfa" : "sem",
      cbsemMeanStructure: false,
      cbsemStandardization: "std_all",
      cbsemGroupColumn: null,
      cbsemInvarianceSteps: null,
      cbsemBootstrapSamples,
      ...(cbsemBootstrapSamples > 0 ? { confidenceLevel: 0.95 } : {}),
    };
  }

  if (kind === "plsc_bootstrap") {
    return {
      ...normalized,
      method: "plsc",
      weightingScheme: normalized.weightingScheme === "pca" ? "path" : (normalized.weightingScheme ?? "path"),
      preprocessing: normalized.preprocessing ?? "standardized",
      bootstrapSamples: boundedInteger(
        normalized.bootstrapSamples > 0 ? normalized.bootstrapSamples : undefined,
        10_000,
        1_000,
        10_000,
      ),
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: normalized.workers,
      caseWeightColumn: null,
    };
  }

  if (kind === "gsca") {
    return {
      ...normalized,
      method: "gsca",
      weightingScheme: "path",
      preprocessing: "standardized",
      tolerance: 1e-7,
      maxIterations: 3_000,
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
    };
  }

  if (kind === "nca") {
    const ceiling = normalized.ncaCeiling === "ce_fdh" || normalized.ncaCeiling === "cr_fdh"
      ? normalized.ncaCeiling
      : "both";
    return {
      ...normalized,
      method: "nca",
      weightingScheme: "path",
      preprocessing: "unstandardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      ncaX: normalized.ncaX?.trim() || null,
      ncaY: normalized.ncaY?.trim() || null,
      ncaCeiling: ceiling,
      ncaPermutationSamples: boundedInteger(
        normalized.ncaPermutationSamples,
        999,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.ncaPermutationSamples.minimum,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.ncaPermutationSamples.maximum,
      ),
    };
  }

  if (kind === "pca") {
    const variables = (normalized.pcaVariables ?? "")
      .split(",")
      .map((variable) => variable.trim())
      .filter(Boolean);
    const componentRule = normalized.pcaComponentRule === "fixed"
      || normalized.pcaComponentRule === "variance_threshold"
      ? normalized.pcaComponentRule
      : "kaiser";
    return {
      ...normalized,
      method: "pca",
      weightingScheme: "path",
      preprocessing: "standardized",
      bootstrapSamples: 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: 1,
      caseWeightColumn: null,
      pcaVariables: variables.join(",") || null,
      pcaComponentRule: componentRule,
      pcaComponents: boundedInteger(
        normalized.pcaComponents,
        2,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.pcaComponents.minimum,
        NATIVE_ANALYSIS_RECIPE_BOUNDS.pcaComponents.maximum,
      ),
      pcaVarianceThreshold: boundedNumber(normalized.pcaVarianceThreshold, 0.80, 0.01, 0.999),
    };
  }

  if (kind === "regression") {
    const normalizeCsv = (value: string | null | undefined) => (value ?? "")
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean)
      .join(",");
    const regressionType = normalized.regressionType === "logistic"
      ? "logistic"
      : normalized.regressionType === "process"
        ? "process"
        : "ols";
    const processGraph = regressionType === "process" ? parseNativeProcessGraph(normalized.processGraph) : null;
    const processAssessment = processGraph
      ? nativeProcessGraphAssessment({
          ...normalized,
          regressionType: "process",
          processGraph,
          regressionPredictors: null,
        })
      : null;
    const bootstrapEnabled = normalized.regressionBootstrap === true;
    return {
      ...normalized,
      method: "regression",
      weightingScheme: "path",
      preprocessing: "unstandardized",
      bootstrapSamples: bootstrapEnabled
        ? boundedInteger(
            normalized.bootstrapSamples > 0 ? normalized.bootstrapSamples : undefined,
            NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.default,
            NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum,
            NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum,
          )
        : 0,
      studentizedInnerSamples: 0,
      permutationSamples: 0,
      workers: bootstrapEnabled
        ? boundedInteger(normalized.workers, 1, NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.minimum, NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.maximum)
        : 1,
      confidenceLevel: 0.95,
      caseWeightColumn: null,
      regressionType,
      regressionOutcome: normalized.regressionOutcome?.trim() || null,
      regressionPredictors: regressionType === "process"
        ? processAssessment?.predictors.join(",") || null
        : normalizeCsv(normalized.regressionPredictors) || null,
      regressionControls: normalizeCsv(normalized.regressionControls) || null,
      regressionBootstrap: bootstrapEnabled,
      robustSe: regressionType === "logistic" ? "none" : "hc3",
      processX: null,
      processM: null,
      processW: null,
      processGraph,
    };
  }

  return {
    ...normalized,
    method: kind,
    weightingScheme: normalized.weightingScheme === "pca" ? "path" : (normalized.weightingScheme ?? "path"),
    preprocessing: kind === "wpls" || kind === "cca" ? "standardized" : (normalized.preprocessing ?? "standardized"),
    bootstrapSamples: 0,
    studentizedInnerSamples: 0,
    permutationSamples: 0,
    workers: 1,
    caseWeightColumn: kind === "wpls" ? normalized.caseWeightColumn : null,
  };
}

/**
 * Product-capability projection for a concrete native catalogue entry.
 *
 * Bootstrap and path randomization share the legacy PLS engine recipe field,
 * but they are distinct option cells in Capability Registry V2. Customer
 * surfaces must use this projection before asking the registry for visibility.
 */
export function nativeCapabilitySettingsForWorkbenchKindV2(
  settings: Readonly<AnalysisUiSettings>,
  kind: NativeWorkbenchAnalysisKind,
): AnalysisUiSettings {
  const normalized = nativeAnalysisSettingsForWorkbenchKind(settings, kind);
  if (kind === "pls_bootstrap") return { ...normalized, method: "bootstrap" };
  if (kind === "pls_permutation") return { ...normalized, method: "permutation" };
  if (kind === "pls_posthoc_technical_minimum_sample_size") {
    return { ...normalized, method: "pls_pm", posthocTechnicalMinimumSampleSize: true };
  }
  return normalized;
}

export function nativeAnalysisStartLabel(
  kind: NativeWorkbenchAnalysisKind,
  retry: boolean,
  regressionType?: AnalysisUiSettings["regressionType"],
  regressionBootstrap = false,
): string {
  const verb = retry ? "Retry" : "Start";
  if (kind === "pls_bootstrap") return `${verb} bootstrapping`;
  if (kind === "plsc_bootstrap") return `${verb} consistent bootstrapping`;
  if (kind === "pls_permutation") return `${verb} path randomization`;
  if (kind === "pls_posthoc_technical_minimum_sample_size") return `${verb} post-hoc calculation`;
  if (kind === "pls_sample_size_power") return `${verb} prospective power analysis`;
  if (kind === "mga") return `${verb} group analysis`;
  if (kind === "predict") return `${verb} prediction`;
  if (kind === "plsc") return `${verb} consistent PLS`;
  if (kind === "wpls") return `${verb} weighted PLS`;
  if (kind === "cca") return `${verb} composite diagnostics`;
  if (kind === "cta_pls") return `${verb} tetrad diagnostics`;
  if (kind === "ipma") return `${verb} importance-performance analysis`;
  if (kind === "cbsem") return `${verb} CB-SEM / CFA`;
  if (kind === "gsca") return `${verb} GSCA`;
  if (kind === "nca") return `${verb} necessary condition analysis`;
  if (kind === "pca") return `${verb} principal component analysis`;
  if (kind === "regression") return `${verb} ${regressionType === "logistic"
    ? "binary logistic regression"
    : regressionType === "process"
      ? "graph-defined path analysis"
      : "OLS regression"}${regressionBootstrap ? " with bootstrap" : ""}`;
  return `${verb} calculation`;
}
