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
import { NATIVE_HIGHER_ORDER_SCOPE_LABEL, nativeHigherOrderScopeProblems } from "./nativeHigherOrder";

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
    const logistic = settings.regressionType === "logistic";
    const assessment = logistic
      ? nativeLogisticReadiness(dataset, settings)
      : nativeOlsReadiness(dataset, settings);
    return readinessFromItems([
      runtimeItem(nativeDesktop),
      dataItem(dataset),
      {
        id: "calculation",
        label: logistic ? "Binary logistic regression" : "Ordinary least squares regression",
        detail: assessment.detail,
        status: assessment.canRun ? "ready" : "blocked",
      },
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
    ["path.self", "path.duplicate", "path.cycle", "path.unknown_construct", "interaction.invalid", "interaction.multiple"].includes(issue.code)
    || issue.code.startsWith("higher_order."),
  );
  if (structural.length > 0) {
    const descriptions = new Set(structural.map((issue) => {
      if (issue.code === "path.cycle") return "a directed cycle";
      if (issue.code === "path.self") return "a self-referencing path";
      if (issue.code === "path.duplicate") return "a duplicate path";
      if (issue.code === "interaction.multiple") return "more than one interaction in the validated scope";
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
  if (settings.method === "mga") {
    const groupColumn = settings.groupColumn?.trim() ?? "";
    const groupA = settings.groupAValue?.trim() ?? "";
    const groupB = settings.groupBValue?.trim() ?? "";
    const assignedIndicators = new Set(nodes.flatMap((node) => node.data.indicators));
    const groupMethods = (settings.groupMethods ?? "")
      .split(",")
      .map((method) => method.trim())
      .filter(Boolean);
    const hasStructuralPath = edges.some((edge) => {
      const role = (edge.data as { role?: string } | undefined)?.role;
      return role !== "control" && role !== "covariance" && !edge.id.startsWith("measurement::");
    });
    const specialConstructs = nodes.filter((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order");
    const permutations = settings.groupPermutationSamples ?? 5_000;
    const groupMethodSet = new Set(groupMethods);
    const problems = [
      !groupColumn ? "Two-group MGA requires a grouping variable" : null,
      groupColumn && !dataset.columns.includes(groupColumn) ? "The grouping variable is absent from the active dataset" : null,
      groupColumn && assignedIndicators.has(groupColumn) ? "The grouping variable cannot also be a model indicator" : null,
      !groupA || !groupB ? "Choose explicit Group A and Group B values" : null,
      groupA && groupB && groupA === groupB ? "Group A and Group B must be different values" : null,
      !hasStructuralPath ? "Two-group MGA requires at least one structural path" : null,
      specialConstructs.length > 0 ? "Two-group MGA does not support interaction or higher-order constructs in the validated native scope" : null,
      settings.caseWeightColumn?.trim() ? "Two-group MGA does not support a case-weight column" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "Two-group MGA uses its dedicated group-label permutation plan and cannot be combined with other resampling settings"
        : null,
      (settings.weightingScheme ?? "path") !== "path" ? "MICOM and two-group MGA require path weighting" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "MICOM and two-group MGA require standardized preprocessing" : null,
      groupMethods.length !== 2 || groupMethodSet.size !== 2 || !groupMethodSet.has("micom") || !groupMethodSet.has("mga_permutation")
        ? "The native group workflow requires MICOM and two-group permutation MGA together"
        : null,
      settings.micomConfiguralConfirmed !== true
        ? "Confirm MICOM Step 1: identical indicators, coding, data treatment, algorithm settings, and substantive meaning across both groups"
        : null,
      !Number.isInteger(permutations) || permutations < 5_000 || permutations > 10_000
        ? "MICOM and two-group MGA require 5,000 to 10,000 group-label permutations"
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
          detail: `MICOM and two-group permutation MGA are selected for ${groupA} (Group A) minus ${groupB} (Group B), using ${permutations} deterministic permutations. Step 1 is researcher-confirmed; Steps 2 and 3, paths, loadings, and weights are calculated together.`,
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
        ? "PLSpredict / CVPAT does not support interaction or higher-order constructs in the native validated scope"
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
        ? "Importance-Performance Map Analysis does not support interaction or higher-order constructs in the validated native scope"
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
      controlPaths.length ? "CCA composite residual diagnostics do not support control paths in the validated native scope" : null,
      specialConstructs.length
        ? "CCA composite residual diagnostics do not support interaction or higher-order constructs in the validated native scope"
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
      controlPaths.length ? "The bounded CB-SEM / CFA scope does not support control-path annotations" : null,
      covariancePaths.length ? "Explicit covariance edges are not stored in the bounded CB-SEM contract; exogenous latent covariances are estimated automatically" : null,
      specialConstructs.length ? "The bounded CB-SEM / CFA scope does not support interaction or higher-order constructs" : null,
      observations < 10 ? "CB-SEM / CFA requires at least 10 observations before listwise filtering" : null,
      completeCases !== null && completeCases < 10 ? `CB-SEM / CFA requires at least 10 complete cases across all assigned indicators; ${completeCases} remain` : null,
      (settings.weightingScheme ?? "path") !== "path" ? "CB-SEM / CFA uses fixed path-weighted PLS initialization" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "CB-SEM / CFA uses listwise-standardized raw-data indicators" : null,
      settings.workers !== 1 ? "The bounded CB-SEM / CFA optimizer uses one deterministic worker" : null,
      settings.caseWeightColumn?.trim() ? "The bounded CB-SEM / CFA scope does not support case weights" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0 || (settings.cbsemBootstrapSamples ?? 0) > 0
        ? "Run CB-SEM / CFA without bootstrap or permutation inference in the bounded native scope"
        : null,
      settings.cbsemMeanStructure ? "Selectable mean structures remain outside the bounded native CB-SEM / CFA scope" : null,
      settings.cbsemGroupColumn?.trim() || settings.cbsemInvarianceSteps?.trim()
        ? "CB-SEM multigroup and measurement-invariance analysis remain outside the bounded native scope"
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
          detail: completeCases === null
            ? `Single-group reflective ${modelType === "cfa" ? "CFA" : "recursive CB-SEM"} with maximum likelihood, marker identification, and listwise-standardized raw-data indicators is selected. At least 10 complete cases are verified when the calculation starts.`
            : `Single-group reflective ${modelType === "cfa" ? "CFA" : "recursive CB-SEM"} with maximum likelihood, marker identification, and ${completeCases} complete cases is selected.`,
          status: completeCases === null ? "warning" : "ready",
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
      controlPaths.length ? "The bounded GSCA scope does not support control paths" : null,
      covariancePaths.length ? "The bounded GSCA scope does not support covariance paths" : null,
      specialConstructs.length ? "The bounded GSCA scope does not support interaction or higher-order constructs" : null,
      (settings.weightingScheme ?? "path") !== "path" ? "GSCA uses its own ALS estimator; the recipe weighting field is a fixed path sentinel" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "GSCA requires listwise-standardized numeric indicators" : null,
      settings.maxIterations !== 3_000 ? "GSCA uses a fixed maximum of 3,000 ALS iterations" : null,
      settings.tolerance !== 1e-7 ? "GSCA uses a fixed 1e-7 convergence criterion" : null,
      settings.workers !== 1 ? "The bounded GSCA solver uses one deterministic worker" : null,
      settings.caseWeightColumn?.trim() ? "The bounded GSCA scope does not support case weights" : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "GSCA inference is not included in the bounded native scope"
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
        ? "Consistent PLS does not support interaction or higher-order constructs in the native validated scope"
        : null,
      settings.bootstrapSamples > 0 || settings.studentizedInnerSamples > 0 || settings.permutationSamples > 0
        ? "Run Consistent PLS separately from bootstrap and permutation inference"
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
          detail: "Consistent PLS correction is selected for reflective constructs with at least two indicators each.",
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
          detail: `${NATIVE_HIGHER_ORDER_SCOPE_LABEL} estimation is selected. Lower-order scores are generated in stage 1 and become the HOC measurement block in stage 2.`,
          status: "ready",
        };
  }
  const moderationInteractions = nodes.filter((node) => node.data.semantic === "interaction");
  if (moderationInteractions.length > 0) {
    const higherOrderConstructs = nodes.filter((node) => node.data.semantic === "higher_order");
    const controlPaths = edges.filter((edge) => (edge.data as { role?: string } | undefined)?.role === "control");
    const problems = [
      moderationInteractions.length !== 1 ? "The validated moderation scope supports exactly one two-way interaction" : null,
      (settings.weightingScheme ?? "path") !== "path" ? "Two-stage moderation requires path weighting" : null,
      (settings.preprocessing ?? "standardized") !== "standardized" ? "Two-stage moderation requires standardized preprocessing" : null,
      settings.caseWeightColumn?.trim() ? "Two-stage moderation does not support case weights" : null,
      controlPaths.length ? "Two-stage moderation does not support control paths in the validated desktop scope" : null,
      higherOrderConstructs.length ? "Two-stage moderation does not support higher-order constructs in the validated desktop scope" : null,
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
        ? `PLS-SEM estimation${moderationSuffix} with single-model Freedman–Lane structural path randomization is selected.`
        : `PLS-SEM Algorithm estimation${moderationSuffix} is selected.`,
    status: "ready",
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
      ? "Weighted PLS does not support interaction or higher-order constructs in the native validated scope"
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
      : `Weighted PLS is selected with ${weightColumn}. Visible values are compatible; the native engine will validate the complete case-weight column before estimation.`,
    status: completePreview ? "ready" : "warning",
  };
}
