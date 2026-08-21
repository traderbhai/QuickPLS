import type { Edge, Node } from "@xyflow/react";
import { validateModel, type ModelIssue } from "../domain/modelValidation";
import type { AnalysisUiSettings, ConstructData, Dataset } from "../types";
import { nativeIpmaTargetOptions } from "./nativeIpma";
import {
  NATIVE_PREDICTION_CONFIDENCE_LEVEL,
  NATIVE_PREDICTION_FOLDS,
  NATIVE_PREDICTION_MIN_COMPLETE_CASES,
  NATIVE_PREDICTION_REPEATS,
} from "./nativeCalculationMode";
import { nativeNcaReadiness } from "./nativeNca";
import { nativePcaReadiness } from "./nativePca";
import { nativeOlsReadiness } from "./nativeOls";
import { nativeLogisticReadiness } from "./nativeLogistic";
import { nativeProcessReadiness } from "./nativeProcess";
import { NATIVE_HIGHER_ORDER_SCOPE_LABEL, nativeHigherOrderScopeProblems } from "./nativeHigherOrder";
import { NATIVE_ANALYSIS_RECIPE_BOUNDS, NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS } from "./nativeAnalysisRecipe";
import { nativeCtaPlsSetupAssessment } from "./nativeCtaPls";
import { buildNativePlsSampleSizePowerRecipe } from "./nativePlsSampleSizePower";

export type NativePlsReadinessStatus = "ready" | "warning" | "blocked";

export interface NativePlsReadinessItem {
  id: string;
  label: string;
  detail: string;
  status: NativePlsReadinessStatus;
}

export interface NativePlsReadiness {
  canRun: boolean;
  summary: string;
  blockers: NativePlsReadinessItem[];
  warnings: NativePlsReadinessItem[];
  items: NativePlsReadinessItem[];
}

export interface NativePlsReadinessInput {
  dataset: Dataset;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  settings: AnalysisUiSettings;
  nativeDesktop: boolean;
}

/**
 * Readiness for the native model calculation paths currently exposed by the
 * compact desktop workbench.
 *
 * This intentionally avoids the broad method catalog and workflow guidance.
 * The native shell needs a small set of factual, engine-facing prerequisites.
 */
export function nativePlsReadiness(input: NativePlsReadinessInput): NativePlsReadiness {
  const { dataset, nodes, edges, settings, nativeDesktop } = input;
  if (settings.method === "pls_sample_size_power") {
    return readinessFromItems([
      runtimeItem(nativeDesktop),
      {
        id: "provenance-anchor",
        label: "Project provenance",
        detail: dataset.fingerprint?.trim()
          ? "The active dataset fingerprint anchors the project recipe only; observed values and observed sample size are not used in the prospective simulation."
          : "Save or import an active project dataset so the prospective recipe has a reproducible project fingerprint. Its observed values will not be used.",
        status: dataset.fingerprint?.trim() ? "ready" : "blocked",
      },
      prospectivePowerItem(settings, nodes, edges),
    ]);
  }
  if (settings.method === "nca") {
    const assessment = nativeNcaReadiness(dataset, settings);
    return readinessFromItems([
      runtimeItem(nativeDesktop),
      dataItem(dataset),
      {
        id: "calculation",
        label: "Necessary condition analysis",
        detail: assessment.detail,
        status: assessment.canRun ? "ready" : "blocked",
      },
    ]);
  }
  if (settings.method === "pca") {
    const assessment = nativePcaReadiness(dataset, settings);
    return readinessFromItems([
      runtimeItem(nativeDesktop),
      dataItem(dataset),
      {
        id: "calculation",
        label: "Principal component analysis",
        detail: assessment.detail,
        status: assessment.canRun ? "ready" : "blocked",
      },
    ]);
  }
  if (settings.method === "regression") {
    if (settings.regressionType === "process") {
      const assessment = nativeProcessReadiness(dataset, settings);
      const bootstrap = settings.regressionBootstrap === true;
      const bootstrapProblems = !bootstrap ? [] : [
        !Number.isInteger(settings.bootstrapSamples)
          || settings.bootstrapSamples < NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum
          || settings.bootstrapSamples > NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum
          ? `Choose ${NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum} to ${NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum} whole-number bootstrap samples`
          : null,
        !Number.isInteger(settings.workers) || settings.workers < 1 || settings.workers > 64
          ? "Choose 1 to 64 PROCESS bootstrap workers"
          : null,
      ].filter((problem): problem is string => Boolean(problem));
      return readinessFromItems([
        runtimeItem(nativeDesktop),
        dataItem(dataset),
        {
          id: "calculation",
          label: "Graph-defined path analysis",
          detail: assessment.detail,
          status: assessment.canRun ? "ready" : "blocked",
        },
        ...(bootstrap ? [{
          id: "process-bootstrap",
          label: "PROCESS bootstrap",
          detail: bootstrapProblems.length
            ? `${bootstrapProblems.join("; ")}.`
            : `${settings.bootstrapSamples} seeded case resamples; percentile intervals primary and BCa conditional; fixed two-sided 95% normal-reference bootstrap-ratio tests; deterministic indexed streams.`,
          status: bootstrapProblems.length ? "blocked" as const : "ready" as const,
        }] : []),
      ]);
    }
    const logistic = settings.regressionType === "logistic";
    const assessment = logistic
      ? nativeLogisticReadiness(dataset, settings)
      : nativeOlsReadiness(dataset, settings);
    const bootstrap = settings.regressionBootstrap === true;
    const bootstrapProblems = !bootstrap ? [] : [
      !Number.isInteger(settings.bootstrapSamples)
        || settings.bootstrapSamples < NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum
        || settings.bootstrapSamples > NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum
        ? `Choose ${NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.minimum} to ${NATIVE_ANALYSIS_RECIPE_BOUNDS.regressionBootstrapSamples.maximum} whole-number bootstrap samples`
        : null,
      settings.confidenceLevel !== 0.95 ? "Regression bootstrap inference uses a fixed 95% confidence level" : null,
      settings.studentizedInnerSamples !== 0 ? "Regression bootstrap does not support studentized intervals" : null,
      settings.permutationSamples !== 0 ? "Regression bootstrap cannot be combined with permutation inference" : null,
      !Number.isInteger(settings.workers) || settings.workers < 1 || settings.workers > 64
        ? "Choose 1 to 64 bootstrap workers"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return readinessFromItems([
      runtimeItem(nativeDesktop),
      dataItem(dataset),
      {
        id: "calculation",
        label: logistic ? "Binary logistic regression" : "Ordinary least squares regression",
        detail: assessment.detail,
        status: assessment.canRun ? "ready" : "blocked",
      },
      ...(bootstrap ? [{
        id: "regression-bootstrap",
        label: "Regression bootstrap",
        detail: bootstrapProblems.length
          ? `${bootstrapProblems.join("; ")}.`
          : `${settings.bootstrapSamples} seeded case resamples; percentile intervals primary, BCa conditional, fixed two-sided 95% normal-reference bootstrap-ratio tests; deterministic worker-invariant indexed streams. Runtime scales with resamples.`,
        status: bootstrapProblems.length ? "blocked" as const : "ready" as const,
      }] : []),
    ]);
  }
  const issues = validateModel(nodes, edges);
  const items = [
    runtimeItem(nativeDesktop),
    dataItem(dataset),
    constructItem(nodes, issues),
    indicatorItem(dataset, nodes, issues),
    modelItem(nodes, issues),
    numericIndicatorItem(dataset, nodes),
    sampleSizeItem(dataset),
    calculationItem(settings, dataset, nodes, edges),
  ];
  return readinessFromItems(items);
}

function readinessFromItems(items: NativePlsReadinessItem[]): NativePlsReadiness {
  const blockers = items.filter((item) => item.status === "blocked");
  const warnings = items.filter((item) => item.status === "warning");

  return {
    canRun: blockers.length === 0,
    summary: blockers.length
      ? `${blockers.length} blocker${blockers.length === 1 ? "" : "s"} before calculation`
      : warnings.length
        ? `Ready with ${warnings.length} warning${warnings.length === 1 ? "" : "s"}`
        : "Ready to calculate",
    blockers,
    warnings,
    items,
  };
}

function runtimeItem(nativeDesktop: boolean): NativePlsReadinessItem {
  return nativeDesktop
    ? { id: "runtime", label: "Runtime", detail: "QuickPLS desktop runtime is available.", status: "ready" }
    : {
        id: "runtime",
        label: "Runtime",
        detail: "Calculations require the offline QuickPLS desktop runtime. This preview can inspect models but cannot run them.",
        status: "blocked",
      };
}

function dataItem(dataset: Dataset): NativePlsReadinessItem {
  if (dataset.kind && dataset.kind !== "raw") {
    return {
      id: "data",
      label: "Data",
      detail: "Native model calculations require case-level raw data.",
      status: "blocked",
    };
  }
  if (!dataset.fingerprint?.trim()) {
    return {
      id: "data",
      label: "Data",
      detail: "The raw dataset does not have the reproducible fingerprint required by the desktop calculation engine.",
      status: "blocked",
    };
  }
  if (dataset.columns.length === 0 || (dataset.rowCount ?? dataset.rows.length) === 0) {
    return {
      id: "data",
      label: "Data",
      detail: "The fingerprinted dataset must contain variables and observations.",
      status: "blocked",
    };
  }
  return {
    id: "data",
    label: "Data",
    detail: `${dataset.name} contains ${dataset.rowCount ?? dataset.rows.length} observations and ${dataset.columns.length} variables with a reproducible fingerprint.`,
    status: "ready",
  };
}

function constructItem(nodes: Array<Node<ConstructData>>, issues: ModelIssue[]): NativePlsReadinessItem {
  if (nodes.length === 0) {
    return { id: "constructs", label: "Constructs", detail: "The model does not contain any constructs.", status: "blocked" };
  }
  const unnamed = issues.filter((issue) => issue.code === "construct.empty_name").length;
  if (unnamed > 0) {
    return {
      id: "constructs",
      label: "Constructs",
      detail: `${unnamed} construct${unnamed === 1 ? " has" : "s have"} no name.`,
      status: "blocked",
    };
  }
  return { id: "constructs", label: "Constructs", detail: `${nodes.length} named construct${nodes.length === 1 ? " is" : "s are"} present.`, status: "ready" };
}

function indicatorItem(dataset: Dataset, nodes: Array<Node<ConstructData>>, issues: ModelIssue[]): NativePlsReadinessItem {
  const unassigned = issues.filter((issue) => issue.code === "construct.no_indicators").length;
  const assigned = nodes.flatMap((node) => node.data.indicators);
  if (assigned.length === 0) {
    return { id: "indicators", label: "Indicators", detail: "The model does not contain any assigned indicators.", status: "blocked" };
  }
  const counts = new Map<string, number>();
  for (const indicator of assigned) counts.set(indicator, (counts.get(indicator) ?? 0) + 1);
  const duplicates = [...counts].filter(([, count]) => count > 1).map(([indicator]) => indicator);
  const missing = [...new Set(assigned.filter((indicator) => !dataset.columns.includes(indicator)))];
  const problems = [
    unassigned ? `${unassigned} construct${unassigned === 1 ? " has" : "s have"} no assigned indicators` : null,
    duplicates.length ? `${duplicates.length} indicator${duplicates.length === 1 ? " is" : "s are"} assigned more than once` : null,
    missing.length ? `${missing.length} assigned indicator${missing.length === 1 ? " is" : "s are"} absent from the dataset` : null,
  ].filter((problem): problem is string => Boolean(problem));

  if (problems.length > 0) return { id: "indicators", label: "Indicators", detail: problems.join("; ") + ".", status: "blocked" };
  return {
    id: "indicators",
    label: "Indicators",
    detail: `${assigned.length} unique indicator${assigned.length === 1 ? " is" : "s are"} assigned to the model.`,
    status: "ready",
  };
}

function modelItem(nodes: Array<Node<ConstructData>>, issues: ModelIssue[]): NativePlsReadinessItem {
  if (nodes.length === 0) {
    return { id: "model", label: "Structural model", detail: "Structural validity cannot be checked without constructs.", status: "blocked" };
  }
  const structural = issues.filter((issue) =>
    ["path.self", "path.duplicate", "path.cycle", "path.unknown_construct", "interaction.invalid", "interaction.duplicate"].includes(issue.code)
    || issue.code.startsWith("higher_order."),
  );
  if (structural.length > 0) {
    const descriptions = new Set(structural.map((issue) => {
      if (issue.code === "path.cycle") return "a directed cycle";
      if (issue.code === "path.self") return "a self-referencing path";
      if (issue.code === "path.duplicate") return "a duplicate path";
      if (issue.code === "interaction.duplicate") return "a duplicate moderating effect";
      if (issue.code === "interaction.invalid") return "an incomplete moderating effect";
      if (issue.code === "higher_order.invalid") return "an incomplete higher-order construct declaration";
      if (issue.code === "higher_order.components") return "a higher-order construct with fewer than two components";
      if (issue.code === "higher_order.self_component") return "a higher-order construct that includes itself";
      if (issue.code === "higher_order.unknown_component") return "a higher-order construct with an unknown component";
      if (issue.code === "higher_order.duplicate_component") return "a higher-order construct with a duplicate component";
      if (issue.code === "higher_order.hybrid_component_indicators") return "a hybrid higher-order component with fewer than two indicators";
      return "a path connected to an unknown construct";
    }));
    return {
      id: "model",
      label: "Structural model",
      detail: `The structural model contains ${[...descriptions].join(", ")}.`,
      status: "blocked",
    };
  }
  return {
    id: "model",
    label: "Structural model",
    detail: "The directed structural model is acyclic and its paths reference known constructs.",
    status: "ready",
  };
}

function numericIndicatorItem(dataset: Dataset, nodes: Array<Node<ConstructData>>): NativePlsReadinessItem {
  const assigned = new Set(nodes.flatMap((node) => node.data.indicators));
  const metadata = dataset.columnMetadata ?? [];
  const nonNumeric = metadata.filter((column) => assigned.has(column.name) && column.column_type !== "numeric");
  if (nonNumeric.length > 0) {
    return {
      id: "numeric-indicators",
      label: "Indicator values",
      detail: `${nonNumeric.map((column) => column.name).join(", ")} ${nonNumeric.length === 1 ? "is not a numeric variable" : "are not numeric variables"}.`,
      status: "blocked",
    };
  }
  const declared = metadata.filter((column) => assigned.has(column.name)).length;
  return {
    id: "numeric-indicators",
    label: "Indicator values",
    detail: declared > 0
      ? `${declared} assigned indicator${declared === 1 ? " has" : "s have"} numeric metadata.`
      : "No incompatible indicator type metadata is present.",
    status: "ready",
  };
}

function sampleSizeItem(dataset: Dataset): NativePlsReadinessItem {
  const observations = dataset.rowCount ?? dataset.rows.length;
  if (observations > 0 && observations < 10) {
    return {
      id: "sample-size",
      label: "Sample size",
      detail: `${observations} observations are available. This very small sample may produce unstable estimates; adequacy depends on the model and intended inference.`,
      status: "warning",
    };
  }
  return {
    id: "sample-size",
    label: "Sample size",
    detail: observations > 0 ? `${observations} observations are available.` : "No observations are available for sample-size assessment.",
    status: "ready",
  };
}

function calculationItem(
  settings: AnalysisUiSettings,
  dataset: Dataset,
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
): NativePlsReadinessItem {
  if (settings.method === "pls_sample_size_power") {
    return prospectivePowerItem(settings, nodes, edges);
  }
  if (settings.method === "cta_pls") {
    const assessment = nativeCtaPlsSetupAssessment(dataset, nodes, settings, edges);
    return {
      id: "calculation",
      label: "CTA-PLS tetrad diagnostics",
      detail: assessment.canRun
        ? `${assessment.detail} Values are descriptive; no block classification or inferential decision is calculated.`
        : assessment.detail,
      status: assessment.canRun ? "ready" : "blocked",
    };
  }
  if (settings.method === "mga") {
    const groupColumn = settings.groupColumn?.trim() ?? "";
    const groupA = settings.groupAValue?.trim() ?? "";
    const groupB = settings.groupBValue?.trim() ?? "";
    const assignedIndicators = new Set(nodes.flatMap((node) => node.data.indicators));
    const groupMethods = (settings.groupMethods ?? "")
      .split(",")
      .map((method) => method.trim())
      .filter(Boolean);
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const permutations = settings.groupPermutationSamples ?? 5_000;
    const groupMethodSet = new Set(groupMethods);
    const problems = [
      !groupColumn ? "MICOM requires a grouping variable" : null,
      groupColumn && !dataset.columns.includes(groupColumn) ? "The grouping variable is absent from the active dataset" : null,
      groupColumn && assignedIndicators.has(groupColumn) ? "The grouping variable cannot also be a model indicator" : null,
      !groupA || !groupB ? "Choose explicit Group A and Group B values" : null,
      groupA && groupB && groupA === groupB ? "Group A and Group B must be different values" : null,
      specialConstructs.length > 0 ? "MICOM v3.1 does not support interaction or higher-order constructs" : null,
      settings.caseWeightColumn?.trim() ? "MICOM v3.1 does not support a case-weight column" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "MICOM uses its dedicated group-label permutation plan and cannot be combined with other resampling settings"
        : null,
      (settings.weightingScheme ?? "path") !== "path" ? "MICOM requires path weighting" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "MICOM requires standardized preprocessing" : null,
      groupMethods.length !== 2 || groupMethodSet.size !== 2 || !groupMethodSet.has("micom") || !groupMethodSet.has("mga_permutation")
        ? "The group workflow requires both MICOM and two-group permutation MGA"
        : null,
      settings.micomConfiguralConfirmed !== true
        ? "Confirm MICOM Step 1: identical indicators, coding, data treatment, algorithm settings, and substantive meaning across both groups"
        : null,
      !Number.isInteger(permutations) || permutations < 5_000 || permutations > 10_000
        ? "MICOM requires 5,000 to 10,000 group-label permutations"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${problems.join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: `MICOM and two-group permutation MGA are selected for ${groupA} (Group A) and ${groupB} (Group B), using exactly ${permutations} deterministic size-preserving permutations. MICOM Step 1 is researcher-confirmed; Steps 2 and 3 plus structural-path group differences are calculated.`,
          status: "ready",
        };
  }
  if (settings.method === "predict") {
    const observations = dataset.rowCount ?? dataset.rows.length;
    const structuralEdges = edges.filter((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role !== "control" && role !== "covariance" && !edge.id.startsWith("measurement::");
    });
    const hasStructuralPath = structuralEdges.length > 0;
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const endogenousConstructIds = new Set(structuralEdges.map((edge) => edge.target));
    const formativeEndogenous = nodes.filter((node) => endogenousConstructIds.has(node.id) && node.data.mode === "formative");
    const indicators = [...new Set(nodes.flatMap((node) => node.data.indicators))];
    const completeDatasetPreview = dataset.rows.length >= observations;
    const completeCases = completeDatasetPreview
      ? dataset.rows.filter((row) => indicators.every((indicator) => {
          const value = row[indicator];
          if (value === null || value === undefined || value === "") return false;
          return Number.isFinite(typeof value === "number" ? value : Number(value));
        })).length
      : null;
    const problems = [
      settings.bootstrapSamples > 0 || settings.permutationSamples > 0
        ? "Run prediction separately from bootstrap and permutation inference"
        : null,
      observations < NATIVE_PREDICTION_MIN_COMPLETE_CASES
        ? `PLSpredict / CVPAT requires at least ${NATIVE_PREDICTION_MIN_COMPLETE_CASES} observations before listwise filtering`
        : null,
      completeCases !== null && completeCases < NATIVE_PREDICTION_MIN_COMPLETE_CASES
        ? `PLSpredict / CVPAT requires at least ${NATIVE_PREDICTION_MIN_COMPLETE_CASES} complete cases across all model indicators; ${completeCases} remain after listwise filtering`
        : null,
      !hasStructuralPath ? "PLSpredict / CVPAT requires at least one structural path" : null,
      specialConstructs.length > 0
        ? "PLSpredict / CVPAT does not support interaction or higher-order constructs"
        : null,
      formativeEndogenous.length > 0
        ? `PLSpredict / CVPAT requires reflective endogenous constructs; ${formativeEndogenous.length} endogenous construct${formativeEndogenous.length === 1 ? " is" : "s are"} formative`
        : null,
      settings.caseWeightColumn?.trim()
        ? "PLSpredict / CVPAT does not support a case-weight column"
        : null,
      settings.confidenceLevel !== NATIVE_PREDICTION_CONFIDENCE_LEVEL
        ? "PLSpredict / CVPAT uses a fixed 95% confidence level for its one-sided benchmark tests"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${problems.join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: completeCases === null
            ? `PLSpredict / CVPAT uses fixed seeded ${NATIVE_PREDICTION_FOLDS}-fold × ${NATIVE_PREDICTION_REPEATS}-repeat indicator prediction with IA/LM benchmarks and one-sided 95% CVPAT tests. At least ${NATIVE_PREDICTION_MIN_COMPLETE_CASES} complete cases across all model indicators are required; QuickPLS verifies this after listwise filtering.`
            : `PLSpredict / CVPAT uses fixed seeded ${NATIVE_PREDICTION_FOLDS}-fold × ${NATIVE_PREDICTION_REPEATS}-repeat indicator prediction with IA/LM benchmarks and one-sided 95% CVPAT tests. ${completeCases} complete cases are available across all model indicators.`,
          status: completeCases === null ? "warning" : "ready",
        };
  }
  if (settings.method === "ipma") {
    const targetTokens = (settings.ipmaTargets ?? "")
      .split(",")
      .map((target) => target.trim())
      .filter(Boolean);
    const targetId = targetTokens.length === 1 ? targetTokens[0] : "";
    const targetOptions = nativeIpmaTargetOptions(nodes, edges);
    const target = targetOptions.find((option) => option.id === targetId);
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const problems = [
      targetTokens.length !== 1 ? "Choose exactly one endogenous target construct" : null,
      targetId && !nodes.some((node) => node.id === targetId) ? "The selected IPMA target is not part of the active model" : null,
      targetId && nodes.some((node) => node.id === targetId) && !target
        ? "The selected IPMA target requires at least one incoming structural path"
        : null,
      targetOptions.length === 0 ? "Importance-Performance Map Analysis requires at least one endogenous construct" : null,
      specialConstructs.length > 0
        ? "Importance-Performance Map Analysis does not support interaction or higher-order constructs"
        : null,
      settings.caseWeightColumn?.trim() ? "Importance-Performance Map Analysis does not support case weights" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "Run Importance-Performance Map Analysis separately from resampling inference"
        : null,
      (settings.weightingScheme ?? "path") !== "path" ? "Importance-Performance Map Analysis requires path weighting" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "Importance-Performance Map Analysis requires standardized preprocessing" : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${[...new Set(problems)].join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: `Importance-Performance Map Analysis is selected for ${target!.optionLabel}. Performance uses 0–100 observed-range scaling of standardized composite scores; no theoretical-range correction is applied.`,
          status: "ready",
        };
  }
  if (settings.method === "cca") {
    const nonReflective = nodes.filter((node) => node.data.mode !== "reflective");
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const controlPaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "control");
    const hasStructuralPath = edges.some((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role !== "control" && role !== "covariance" && !edge.id.startsWith("measurement::");
    });
    const problems = [
      nodes.length < 2 ? "CCA composite residual diagnostics require at least two constructs" : null,
      nonReflective.length
        ? `CCA composite residual diagnostics require reflective constructs; ${nonReflective.length} construct${nonReflective.length === 1 ? " is" : "s are"} formative`
        : null,
      !hasStructuralPath ? "CCA composite residual diagnostics require at least one structural path" : null,
      controlPaths.length ? "CCA composite residual diagnostics do not support control paths" : null,
      specialConstructs.length
        ? "CCA composite residual diagnostics do not support interaction or higher-order constructs"
        : null,
      (settings.weightingScheme ?? "path") === "pca" ? "CCA composite residual diagnostics require path or factor weighting" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "CCA composite residual diagnostics require standardized preprocessing" : null,
      settings.caseWeightColumn?.trim() ? "CCA composite residual diagnostics do not support case weights" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "CCA composite residual diagnostics do not calculate resampling inference"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${problems.join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: "CCA composite residual diagnostics are selected for a standardized reflective composite path model. Results are descriptive residuals only; no thresholds or inferential classification are calculated.",
          status: "ready",
        };
  }
  if (settings.method === "cbsem") {
    const observations = dataset.rowCount ?? dataset.rows.length;
    const modelType = settings.cbsemModelType ?? "sem";
    const cbsemBootstrapSamples = settings.cbsemBootstrapSamples ?? 0;
    const cbsemBootstrapEnabled = cbsemBootstrapSamples > 0;
    const cbsemBootstrapTestTail = settings.cbsemBootstrapTestTail ?? "two_sided";
    const cbsemBootstrapInterval = settings.cbsemBootstrapInterval ?? "percentile_type7";
    const analyticStudentized = cbsemBootstrapInterval === "analytic_studentized_type7";
    const bcaType7 = cbsemBootstrapInterval === "bca_type7";
    const boundedLabsInterval = analyticStudentized || bcaType7;
    const boundedLabsIntervalLabel = analyticStudentized ? "Analytic studentized" : "BCa Type 7";
    const cbsemBootstrapTestLabel = modelType === "cfa"
      ? ` (${cbsemBootstrapTestTail.replaceAll("_", " ")} zero-null test)`
      : "";
    const structuralEdges = edges.filter((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role !== "control" && role !== "covariance" && !edge.id.startsWith("measurement::");
    });
    const controlPaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "control");
    const covariancePaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "covariance");
    const nonReflective = nodes.filter((node) => node.data.mode !== "reflective");
    const underspecified = nodes.filter((node) => node.data.indicators.length < 2);
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const assignedIndicators = [...new Set(nodes.flatMap((node) => node.data.indicators))];
    const completeDatasetPreview = dataset.rows.length >= observations;
    const completeCases = completeDatasetPreview
      ? dataset.rows.filter((row) => assignedIndicators.every((indicator) => {
          const value = row[indicator];
          if (value === null || value === undefined || value === "") return false;
          return Number.isFinite(typeof value === "number" ? value : Number(value));
        })).length
      : null;
    const problems = [
      modelType !== "cfa" && modelType !== "sem" ? "Choose confirmatory factor analysis or structural equation modeling" : null,
      modelType === "cfa" && structuralEdges.length > 0 ? "Confirmatory factor analysis requires a measurement-only model with no structural paths" : null,
      modelType === "sem" && structuralEdges.length === 0 ? "Structural equation modeling requires at least one recursive latent path" : null,
      nodes.length === 0 ? "CB-SEM / CFA requires at least one latent factor" : null,
      nonReflective.length
        ? `CB-SEM / CFA requires reflective factors; ${nonReflective.length} construct${nonReflective.length === 1 ? " is" : "s are"} formative`
        : null,
      underspecified.length
        ? `CB-SEM / CFA requires at least two indicators per factor; ${underspecified.length} factor${underspecified.length === 1 ? " has" : "s have"} fewer`
        : null,
      controlPaths.length ? "This CB-SEM / CFA calculation does not support control-path annotations" : null,
      covariancePaths.length ? "This CB-SEM calculation does not yet store explicit covariance edges; exogenous latent covariances are estimated automatically" : null,
      specialConstructs.length ? "This CB-SEM / CFA calculation does not support interaction or higher-order constructs" : null,
      observations < 10 ? "CB-SEM / CFA requires at least 10 observations before listwise filtering" : null,
      completeCases !== null && completeCases < 10 ? `CB-SEM / CFA requires at least 10 complete cases across all assigned indicators; ${completeCases} remain` : null,
      (settings.weightingScheme ?? "path") !== "path" ? "CB-SEM / CFA uses fixed path-weighted PLS initialization" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "CB-SEM / CFA uses listwise-standardized raw-data indicators" : null,
      (!cbsemBootstrapEnabled && settings.workers !== 1)
        ? "Point-only CB-SEM / CFA uses one deterministic worker"
        : null,
      settings.caseWeightColumn?.trim() ? "This CB-SEM / CFA calculation does not support case weights" : null,
      cbsemBootstrapEnabled
        ? "Choose Case bootstrap in the CB-SEM Calculate settings; this point-only setup will not emit a historical schema-3 bootstrap recipe"
        : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "CB-SEM bootstrap v2 cannot be combined with generic PLS bootstrap, studentized, or permutation inference"
        : null,
      cbsemBootstrapEnabled && (cbsemBootstrapSamples < 500 || cbsemBootstrapSamples > 10_000)
        ? "CB-SEM bootstrap v2 requires 500 to 10,000 full-ML case-resampling replicates"
        : null,
      cbsemBootstrapEnabled && settings.confidenceLevel !== 0.95
        ? "CB-SEM bootstrap v2 uses a fixed two-sided 95% interval"
        : null,
      !["percentile_type7", "analytic_studentized_type7", "bca_type7"].includes(cbsemBootstrapInterval)
        ? "Choose the percentile Type 7, analytic studentized Type 7, or BCa Type 7 CB-SEM interval"
        : null,
      boundedLabsInterval && !cbsemBootstrapEnabled
        ? `${boundedLabsIntervalLabel} intervals require exact full-refit case bootstrapping`
        : null,
      boundedLabsInterval && modelType !== "cfa"
        ? `${boundedLabsIntervalLabel} intervals are available only for confirmatory factor analysis`
        : null,
      boundedLabsInterval && cbsemBootstrapTestTail !== "two_sided"
        ? `${boundedLabsIntervalLabel} intervals use the fixed two-sided exact CFA contract`
        : null,
      boundedLabsInterval && (
        !Number.isInteger(settings.workers)
        || settings.workers < NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.workers.minimum
        || settings.workers > NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.workers.maximum
      )
        ? `${boundedLabsIntervalLabel} exact CFA requires ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.workers.minimum} to ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.workers.maximum} workers`
        : null,
      boundedLabsInterval && assignedIndicators.length > NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.modeledVariables.maximum
        ? `${boundedLabsIntervalLabel} exact CFA supports at most ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.modeledVariables.maximum} modeled observed variables; ${assignedIndicators.length} are assigned`
        : null,
      boundedLabsInterval && completeCases !== null && completeCases > NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.completeCases.maximum
        ? `${boundedLabsIntervalLabel} exact CFA supports at most ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.completeCases.maximum} complete cases; ${completeCases} remain`
        : null,
      !["two_sided", "one_sided_greater", "one_sided_less"].includes(cbsemBootstrapTestTail)
        ? "CB-SEM bootstrap test tail must be two-sided, one-sided greater, or one-sided less"
        : null,
      cbsemBootstrapTestTail !== "two_sided" && !cbsemBootstrapEnabled
        ? "A one-sided CB-SEM zero-null test requires exact full-refit case bootstrapping"
        : null,
      cbsemBootstrapTestTail !== "two_sided" && modelType !== "cfa"
        ? "One-sided CB-SEM zero-null tests are available only for exact CFA case bootstrapping"
        : null,
      settings.cbsemMeanStructure ? "CB-SEM / CFA does not yet support selectable mean structures" : null,
      settings.cbsemGroupColumn?.trim() || settings.cbsemInvarianceSteps?.trim()
        ? "CB-SEM does not yet support multigroup or measurement-invariance analysis"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${[...new Set(problems)].join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: `${completeCases === null
            ? `Single-group reflective ${modelType === "cfa" ? "CFA" : "recursive CB-SEM"} with maximum likelihood${cbsemBootstrapEnabled ? ` and ${cbsemBootstrapSamples.toLocaleString()} full-refit case-bootstrap draws at fixed 95% confidence${cbsemBootstrapTestLabel}` : ""}, marker identification, and listwise-standardized raw-data indicators is selected. At least 10 complete cases are verified when the calculation starts.`
            : `Single-group reflective ${modelType === "cfa" ? "CFA" : "recursive CB-SEM"} with maximum likelihood${cbsemBootstrapEnabled ? ` and ${cbsemBootstrapSamples.toLocaleString()} full-refit case-bootstrap draws at fixed 95% confidence${cbsemBootstrapTestLabel}` : ""}, marker identification, and ${completeCases} complete cases is selected.`}${analyticStudentized
            ? ` Analytic studentized inference computes expected-information standard errors for the point fit and every successful outer refit. The compiled runner also enforces at most ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.freeParameterRows.maximum} free-parameter rows and ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.optimizerDimensions.maximum} optimizer dimensions. Archived evidence validates the stored identity, accounting, digests, and interval arithmetic; an unsigned local archive does not provide trusted authentication against a coordinated semantic rewrite.`
            : bcaType7
              ? ` BCa inference uses the successful outer-refit ledger and an exactly-once delete-one schedule. It is complete-only: one failed delete-one fit stores globally typed unavailable BCa inference. The compiled runner also enforces at most ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.freeParameterRows.maximum} free-parameter rows and ${NATIVE_CBSEM_ANALYTIC_STUDENTIZED_CAPS.optimizerDimensions.maximum} optimizer dimensions. Archived evidence validates the stored identity, accounting, digests, and interval arithmetic; an unsigned local archive does not provide trusted authentication against a coordinated semantic rewrite.`
              : ""}${cbsemBootstrapEnabled && cbsemBootstrapSamples < 1_000
            ? ` With ${cbsemBootstrapSamples.toLocaleString()} requested draws, the fixed 1,000-usable threshold is unreachable: percentile${analyticStudentized ? ", studentized," : ""} and BCa intervals${modelType === "cfa" ? " plus zero-null tests" : ""} are typed unavailable. Use 5,000 or 10,000 draws for inference.`
            : ""}`,
          status: completeCases === null || (cbsemBootstrapEnabled && cbsemBootstrapSamples < 1_000) ? "warning" : "ready",
        };
  }
  if (settings.method === "gsca") {
    const structuralEdges = edges.filter((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role !== "control" && role !== "covariance" && !edge.id.startsWith("measurement::");
    });
    const controlPaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "control");
    const covariancePaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "covariance");
    const connected = new Set(structuralEdges.flatMap((edge) => [edge.source, edge.target]));
    const isolated = nodes.filter((node) => !connected.has(node.id));
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const problems = [
      nodes.length < 2 ? "GSCA requires at least two component constructs" : null,
      structuralEdges.length === 0 ? "GSCA requires at least one recursive structural path" : null,
      isolated.length ? `Every GSCA construct must participate in the structural model; ${isolated.length} ${isolated.length === 1 ? "is" : "are"} isolated` : null,
      controlPaths.length ? "GSCA does not support control paths" : null,
      covariancePaths.length ? "GSCA does not support covariance paths" : null,
      specialConstructs.length ? "GSCA does not support interaction or higher-order constructs" : null,
      (settings.weightingScheme ?? "path") !== "path" ? "GSCA uses its own ALS estimator; the recipe weighting field is a fixed path sentinel" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "GSCA requires listwise-standardized numeric indicators" : null,
      settings.maxIterations !== 3_000 ? "GSCA uses a fixed maximum of 3,000 ALS iterations" : null,
      settings.tolerance !== 1e-7 ? "GSCA uses a fixed 1e-7 convergence criterion" : null,
      settings.workers !== 1 ? "GSCA uses one deterministic worker" : null,
      settings.caseWeightColumn?.trim() ? "GSCA does not support case weights" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "GSCA does not yet support resampling inference"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? { id: "calculation", label: "Calculation", detail: `${[...new Set(problems)].join("; ")}.`, status: "blocked" }
      : {
          id: "calculation",
          label: "Calculation",
          detail: "GSCA joint global least-squares ALS is selected for a recursive single-group component model with disjoint reflective or formative blocks. Results are point estimates and fit diagnostics only; inference is not calculated.",
          status: "ready",
        };
  }
  if (settings.method === "plsc") {
    const consistentBootstrapSelected = settings.bootstrapSamples !== 0;
    const nonReflective = nodes.filter((node) => node.data.mode !== "reflective");
    const underspecified = nodes.filter((node) => node.data.indicators.length < 2);
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const problems = [
      settings.weightingScheme === "pca" ? "Consistent PLS requires path or factor weighting" : null,
      nonReflective.length
        ? `Consistent PLS requires reflective measurement models; ${nonReflective.length} construct${nonReflective.length === 1 ? " is" : "s are"} formative`
        : null,
      underspecified.length
        ? `Consistent PLS requires at least two indicators per construct; ${underspecified.length} construct${underspecified.length === 1 ? " has" : "s have"} fewer`
        : null,
      specialConstructs.length
        ? "Consistent PLS does not support interaction or higher-order constructs"
        : null,
      consistentBootstrapSelected && (
        !Number.isInteger(settings.bootstrapSamples)
        || settings.bootstrapSamples < 1_000
        || settings.bootstrapSamples > 10_000
      )
        ? "PLSc consistent bootstrapping requires 1,000 to 10,000 whole-number case resamples"
        : null,
      settings.studentizedInnerSamples !== 0
        ? "PLSc consistent bootstrapping does not support studentized intervals"
        : null,
      settings.permutationSamples !== 0
        ? "PLSc consistent permutation is not available in the Standard calculation workflow"
        : null,
      consistentBootstrapSelected && (settings.bootstrapTestTail ?? "two_sided") !== "two_sided"
        ? "PLSc consistent bootstrapping uses fixed two-sided inference"
        : null,
      consistentBootstrapSelected && (
        !Number.isInteger(settings.workers)
        || settings.workers < NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.minimum
        || settings.workers > NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.maximum
      )
        ? `Choose ${NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.minimum} to ${NATIVE_ANALYSIS_RECIPE_BOUNDS.workers.maximum} PLSc bootstrap workers`
        : null,
      consistentBootstrapSelected && (
        !Number.isInteger(settings.seed)
        || settings.seed < NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.minimum
        || settings.seed > NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.maximum
      )
        ? `Choose a whole-number PLSc bootstrap seed from ${NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.minimum} through ${NATIVE_ANALYSIS_RECIPE_BOUNDS.seed.maximum}`
        : null,
      consistentBootstrapSelected && (
        !Number.isFinite(settings.confidenceLevel)
        || settings.confidenceLevel < NATIVE_ANALYSIS_RECIPE_BOUNDS.confidenceLevel.minimum
        || settings.confidenceLevel > NATIVE_ANALYSIS_RECIPE_BOUNDS.confidenceLevel.maximum
      )
        ? "Choose a PLSc bootstrap confidence level from 80% through 99.9%"
        : null,
    ].filter((problem): problem is string => Boolean(problem));
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${problems.join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: consistentBootstrapSelected
            ? `Full-refit consistent PLS bootstrapping is selected with ${settings.bootstrapSamples.toLocaleString()} indexed case resamples. Point PLSc is re-estimated for every primary and delete-one sample; two-sided normal-reference diagnostics, percentile intervals, and conditional BCa are reported.`
            : "Consistent PLS correction is selected for reflective constructs with at least two indicators each.",
          status: "ready",
        };
  }
  if (settings.method === "wpls") {
    return weightedPlsItem(settings, dataset, nodes);
  }
  if (settings.method !== "pls_pm" && settings.method !== "bootstrap") {
    return {
      id: "calculation",
      label: "Calculation",
      detail: "This method is not yet available in the native calculation workbench.",
      status: "blocked",
    };
  }
  if (nodes.some((node) => node.data.semantic === "higher_order")) {
    const problems = nativeHigherOrderScopeProblems(nodes, edges, settings);
    return problems.length
      ? {
          id: "calculation",
          label: "Calculation",
          detail: `${problems.join("; ")}.`,
          status: "blocked",
        }
      : {
          id: "calculation",
          label: "Calculation",
          detail: `${NATIVE_HIGHER_ORDER_SCOPE_LABEL} estimation is selected. The chosen approach runs its exact repeated-indicator and/or generated-score stages${settings.bootstrapSamples > 0 ? " for every indexed case-bootstrap replicate" : ""}.`,
          status: "ready",
        };
  }
  const moderationInteractions = nodes.filter((node) => node.data.semantic === "interaction");
  if (moderationInteractions.length > 0) {
    const higherOrderConstructs = nodes.filter((node) => node.data.semantic === "higher_order");
    const controlPaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "control");
    const problems = [
      moderationInteractions.length !== 1 ? "Two-stage moderation requires exactly one two-way interaction" : null,
      moderationInteractions.some((node) => node.data.interaction?.kind === "interaction_v2")
        ? "Choose PLS Algorithm or Bootstrapping in Calculate so QuickPLS can route the interaction model to its qualified engine."
        : null,
      (settings.weightingScheme ?? "path") !== "path" ? "Two-stage moderation requires path weighting" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "Two-stage moderation requires standardized preprocessing" : null,
      settings.caseWeightColumn?.trim() ? "Two-stage moderation does not support case weights" : null,
      controlPaths.length ? "Two-stage moderation does not support control paths" : null,
      higherOrderConstructs.length ? "Two-stage moderation does not support higher-order constructs" : null,
    ].filter((problem): problem is string => Boolean(problem));
    if (problems.length) {
      return {
        id: "calculation",
        label: "Calculation",
        detail: `${problems.join("; ")}.`,
        status: "blocked",
      };
    }
  }
  const bootstrapping = settings.method === "bootstrap" || settings.bootstrapSamples > 0;
  const permutation = settings.permutationSamples > 0;
  if (bootstrapping && permutation) {
    return {
      id: "calculation",
      label: "Calculation",
      detail: "Run bootstrapping and permutation inference as separate calculations.",
      status: "blocked",
    };
  }
  if (bootstrapping && (!Number.isInteger(settings.bootstrapSamples) || settings.bootstrapSamples < 100 || settings.bootstrapSamples > 10_000)) {
    return {
      id: "calculation",
      label: "Calculation",
      detail: "Bootstrapping requires 100 to 10,000 resamples.",
      status: "blocked",
    };
  }
  if (bootstrapping && settings.studentizedInnerSamples > 0 && (
    !Number.isInteger(settings.studentizedInnerSamples)
    || settings.studentizedInnerSamples < 99
    || settings.studentizedInnerSamples > 999
    || settings.studentizedInnerSamples % 2 === 0
    || settings.bootstrapSamples < 999
  )) {
    return {
      id: "calculation",
      label: "Calculation",
      detail: "Studentized intervals require an odd inner sample count from 99 to 999 and at least 999 primary bootstrap samples.",
      status: "blocked",
    };
  }
  if (permutation && (!Number.isInteger(settings.permutationSamples) || settings.permutationSamples < 99 || settings.permutationSamples > 10_000)) {
    return {
      id: "calculation",
      label: "Calculation",
      detail: "Permutation inference requires 99 to 10,000 permutations.",
      status: "blocked",
    };
  }
  const moderationSuffix = moderationInteractions.length ? " with two-stage moderation" : "";
  return {
    id: "calculation",
    label: "Calculation",
    detail: bootstrapping
      ? `PLS-SEM estimation${moderationSuffix} and bootstrapping is selected.`
      : permutation
        ? `PLS-SEM estimation${moderationSuffix} with single-model Freedman-Lane structural path randomization on fixed original construct scores is selected.`
        : `PLS-SEM Algorithm estimation${moderationSuffix} is selected.`,
    status: "ready",
  };
}

function prospectivePowerItem(
  settings: AnalysisUiSettings,
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
): NativePlsReadinessItem {
  const predictor = settings.plsPowerPredictorConstruct?.trim() ?? "";
  const outcome = settings.plsPowerOutcomeConstruct?.trim() ?? "";
  const eligible = nodes.filter((node) => (
    !node.data.semantic
    && node.data.mode === "reflective"
    && node.data.indicators.length >= 3
    && node.data.indicators.length <= 10
  ));
  const selectedPredictor = eligible.find((node) => node.id === predictor);
  const selectedOutcome = eligible.find((node) => node.id === outcome);
  const ordinaryPaths = edges.filter((edge) => {
    const role = (edge.data as { role?: string } | undefined)?.role;
    return role !== "control" && role !== "covariance";
  });
  const targetPath = ordinaryPaths.filter((edge) => edge.source === predictor && edge.target === outcome);
  const problems = [
    nodes.length !== 2 || ordinaryPaths.length !== 1
      ? "Power v2 requires exactly two constructs and one directed path"
      : null,
    edges.some((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role === "control" || role === "covariance";
    }) ? "Power v2 excludes control and covariance paths" : null,
    eligible.length !== 2
      ? "Both constructs must be ordinary reflective blocks with 3 to 10 indicators"
      : null,
    !predictor ? "Choose the predictor construct" : null,
    !outcome ? "Choose the outcome construct" : null,
    predictor && outcome && predictor === outcome ? "Predictor and outcome constructs must differ" : null,
    predictor && !selectedPredictor ? "The selected predictor is not an eligible reflective construct" : null,
    outcome && !selectedOutcome ? "The selected outcome is not an eligible reflective construct" : null,
    predictor && outcome && targetPath.length !== 1
      ? "The selected predictor and outcome must match the model's single directed path"
      : null,
  ].filter((problem): problem is string => Boolean(problem));
  let workload: ReturnType<typeof buildNativePlsSampleSizePowerRecipe>["workload"] | null = null;
  if (!problems.length) {
    try {
      workload = buildNativePlsSampleSizePowerRecipe({
        scenarioIdentity: settings.plsPowerScenarioIdentity ?? "",
        predictorConstruct: predictor,
        outcomeConstruct: outcome,
        predictorIndicatorLoadings: settings.plsPowerPredictorLoadings ?? "",
        outcomeIndicatorLoadings: settings.plsPowerOutcomeLoadings ?? "",
        populationPath: String(settings.plsPowerPopulationPath ?? ""),
        exogenousDistribution: "standard_normal",
        structuralDisturbanceDistribution: "standard_normal",
        indicatorErrorDistribution: "standard_normal",
        missingData: "none",
        weightingScheme: settings.weightingScheme === "path" ? "path" : "",
        preprocessing: settings.preprocessing === "standardized" ? "standardized" : "",
        tolerance: String(settings.tolerance ?? ""),
        maxIterations: String(settings.maxIterations ?? ""),
        inference: "case_bootstrap_null_centered_two_sided_plus_one",
        sampleSizeGrid: settings.plsPowerSampleSizeGrid ?? "",
        alpha: String(settings.plsPowerAlpha ?? ""),
        targetPower: String(settings.plsPowerTargetPower ?? ""),
        confidenceLevel: String(settings.confidenceLevel),
        monteCarloReplicates: String(settings.plsPowerMonteCarloReplicates ?? ""),
        bootstrapReplicates: String(settings.plsPowerBootstrapReplicates ?? ""),
        masterSeed: String(settings.seed),
        workers: String(settings.workers),
      }).workload;
    } catch (error) {
      problems.push(error instanceof Error ? error.message : String(error));
    }
  }
  return {
    id: "calculation",
    label: "Prospective PLS-SEM power",
    detail: problems.length
      ? `${problems.join("; ")}.`
      : `Explicit two-construct reflective Gaussian DGP; ${workload!.plannedDatasets.toLocaleString("en-US")} independent simulated datasets, ${workload!.estimatedPlsFits.toLocaleString("en-US")} PLS fits and ${workload!.estimatedPlsCaseFits.toLocaleString("en-US")} fitted rows. Failed replicates count as non-rejections; the first evaluated n whose Wilson lower bound reaches target is selected without interpolation or extrapolation.`,
    status: problems.length ? "blocked" : "ready",
  };
}

function weightedPlsItem(
  settings: AnalysisUiSettings,
  dataset: Dataset,
  nodes: Array<Node<ConstructData>>,
): NativePlsReadinessItem {
  const weightColumn = settings.caseWeightColumn?.trim() ?? "";
  const metadata = dataset.columnMetadata?.find((column) => column.name === weightColumn);
  const nonReflective = nodes.filter((node) => node.data.mode !== "reflective");
  const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
  const invalidPreviewValues = weightColumn && dataset.columns.includes(weightColumn)
    ? dataset.rows.filter((row) => {
        const value = row[weightColumn];
        if (value === null || value === undefined || value === "") return false;
        const numeric = typeof value === "number" ? value : Number(value);
        return !Number.isFinite(numeric) || numeric <= 0;
      }).length
    : 0;
  const problems = [
    settings.weightingScheme === "pca" ? "Weighted PLS requires path or factor weighting" : null,
    settings.preprocessing !== "standardized" ? "Weighted PLS requires standardized preprocessing" : null,
    nonReflective.length
      ? `Weighted PLS requires reflective measurement models; ${nonReflective.length} construct${nonReflective.length === 1 ? " is" : "s are"} formative`
      : null,
    specialConstructs.length
      ? "Weighted PLS does not support interaction or higher-order constructs"
      : null,
    settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
      ? "Run Weighted PLS separately from bootstrap and permutation inference"
      : null,
    !weightColumn ? "Choose a positive numeric case-weight variable" : null,
    weightColumn && !dataset.columns.includes(weightColumn) ? `The selected case-weight variable ${weightColumn} is not in the active dataset` : null,
    metadata && metadata.column_type !== "numeric" ? `The selected case-weight variable ${weightColumn} is not numeric` : null,
    invalidPreviewValues
      ? `${invalidPreviewValues} visible case-weight value${invalidPreviewValues === 1 ? " is" : "s are"} nonpositive or nonnumeric`
      : null,
  ].filter((problem): problem is string => Boolean(problem));

  if (problems.length) {
    return {
      id: "calculation",
      label: "Calculation",
      detail: `${problems.join("; ")}.`,
      status: "blocked",
    };
  }

  const completePreview = dataset.rows.length >= (dataset.rowCount ?? dataset.rows.length);
  return {
    id: "calculation",
    label: "Calculation",
    detail: completePreview
      ? `Weighted PLS is selected with positive numeric case weights from ${weightColumn}.`
      : `Weighted PLS is selected with ${weightColumn}. Visible values are compatible; the calculation checks the complete case-weight column before estimation.`,
    status: completePreview ? "ready" : "warning",
  };
}
