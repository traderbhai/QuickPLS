import type { Edge, Node } from "@xyflow/react";
import { methods } from "../data/sample";
import type { AnalysisMethodId, AnalysisUiSettings, ColumnMetadata, ConstructData, Dataset, MethodDefinition, WorkspaceView } from "../types";
import { validateModel } from "./modelValidation";

export type ApplicabilityStatus = "recommended" | "available" | "needs_setup" | "not_applicable" | "unsupported" | "experimental";
export type MethodCategory = "core_model_estimation" | "inference_add_on" | "assessment_diagnostics" | "prediction_segmentation" | "standalone_analysis" | "workflow_analysis";
export type RequirementStatus = "passed" | "warning" | "failed";

export interface RequirementCheck {
  id: string;
  label: string;
  status: RequirementStatus;
  detail: string;
  actionLabel?: string;
  actionView?: WorkspaceView;
}

export interface MethodApplicability {
  method: MethodDefinition;
  category: MethodCategory;
  status: ApplicabilityStatus;
  scopeStatus: "validated" | "experimental" | "unsupported";
  reason: string;
  nextActionLabel: string;
  checks: RequirementCheck[];
  expectedOutputs: string[];
}

export interface MethodApplicabilityInput {
  dataset: Dataset;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  settings: AnalysisUiSettings;
  nativeDesktop: boolean;
}

export const methodCategoryLabels: Record<MethodCategory, string> = {
  core_model_estimation: "Core model estimation",
  inference_add_on: "Inference add-on",
  assessment_diagnostics: "Assessment and diagnostics",
  prediction_segmentation: "Prediction and segmentation",
  standalone_analysis: "Standalone analysis",
  workflow_analysis: "Workflow analysis",
};

const categories: Record<AnalysisMethodId, MethodCategory> = {
  pls_pm: "core_model_estimation",
  plsc: "core_model_estimation",
  wpls: "core_model_estimation",
  cbsem: "core_model_estimation",
  gsca: "core_model_estimation",
  bootstrap: "inference_add_on",
  permutation: "inference_add_on",
  cca: "assessment_diagnostics",
  cta_pls: "assessment_diagnostics",
  endogeneity: "assessment_diagnostics",
  nonlinear_effects: "assessment_diagnostics",
  predict: "prediction_segmentation",
  mga: "prediction_segmentation",
  ipma: "prediction_segmentation",
  pca: "standalone_analysis",
  regression: "standalone_analysis",
  nca: "standalone_analysis",
  moderated_mediation: "workflow_analysis",
};

const outputMap: Record<AnalysisMethodId, string[]> = {
  pls_pm: ["paths", "loadings / weights", "R²", "effects", "quality diagnostics"],
  bootstrap: ["bootstrap standard errors", "p values", "confidence intervals"],
  permutation: ["structural path coefficients", "exceedance counts", "raw two-sided plus-one p values"],
  plsc: ["consistent PLS corrected paths", "corrected loadings", "corrected R²"],
  wpls: ["weighted paths", "weighted loadings", "weighted R²"],
  cca: ["composite residual diagnostics", "reproduced correlations"],
  cta_pls: ["tetrads", "indicator-block diagnostics"],
  endogeneity: ["Gaussian-copula diagnostic coefficients", "diagnostic warnings"],
  nonlinear_effects: ["quadratic effect diagnostics", "delta R²"],
  moderated_mediation: ["conditional indirect effects", "index of moderated mediation"],
  predict: ["PLSpredict", "CVPAT", "segmentation diagnostics where configured"],
  mga: ["MICOM", "permutation MGA", "group path differences"],
  ipma: ["importance-performance tables", "target construct diagnostics"],
  cbsem: ["ML CFA/SEM estimates", "fit indices", "standardized solution"],
  pca: ["eigenvalues", "loadings", "component scores"],
  gsca: ["component weights", "paths", "FIT / AFIT / GFI diagnostics"],
  regression: ["coefficients", "robust standard errors", "predictions"],
  nca: ["CE-FDH / CR-FDH ceilings", "effect sizes", "bottleneck table"],
};

export function evaluateMethodApplicability(input: MethodApplicabilityInput): MethodApplicability[] {
  return methods
    .filter((method): method is MethodDefinition & { id: AnalysisMethodId } => method.id in categories)
    .map((method) => evaluateOne(method, input));
}

export function methodApplicabilityFor(methodId: AnalysisMethodId, input: MethodApplicabilityInput): MethodApplicability {
  const method = methods.find((candidate) => candidate.id === methodId && candidate.id in categories) as (MethodDefinition & { id: AnalysisMethodId }) | undefined;
  if (method) return evaluateOne(method, input);
  return {
    method: { id: methodId, name: methodId, family: "Unsupported", status: "unsupported" },
    category: "core_model_estimation",
    status: "unsupported",
    scopeStatus: "unsupported",
    reason: "This method is not available in the current QuickPLS method catalog.",
    nextActionLabel: "Choose another method",
    checks: [fail("method-catalog", "Method catalog", "This method is not available in the current QuickPLS method catalog.", "Open Setup", "analyses")],
    expectedOutputs: [],
  };
}

export function topBarMethods(applicabilities: MethodApplicability[], selectedMethod: AnalysisMethodId): MethodApplicability[] {
  const preferred = applicabilities.filter((item) => item.category !== "inference_add_on" && ["recommended", "available"].includes(item.status));
  const selected = applicabilities.find((item) => item.method.id === selectedMethod);
  if (selected && !preferred.some((item) => item.method.id === selected.method.id)) return [...preferred, selected];
  return preferred;
}

export function dataGuidance(input: MethodApplicabilityInput): Array<{ title: string; detail: string; tone: "validated" | "warning" | "neutral"; actionLabel: string; actionView: WorkspaceView }> {
  const { dataset } = input;
  const applicability = evaluateMethodApplicability(input);
  if (!dataset.columns.length) {
    return [{ title: "Import a dataset first", detail: "QuickPLS can recommend SEM, regression, PCA, or NCA only after it sees your variables and metadata.", tone: "warning", actionLabel: "Import data", actionView: "data" }];
  }
  if (dataset.kind === "covariance" || dataset.kind === "correlation") {
    return [{ title: "Matrix input detected", detail: "Use matrix-compatible single-group estimation only; bootstrap, prediction, regression, NCA, and case-level workflows need raw data.", tone: "warning", actionLabel: "Review Setup", actionView: "analyses" }];
  }
  const recommended = applicability.filter((item) => item.status === "recommended").slice(0, 3);
  return recommended.length
    ? recommended.map((item) => ({ title: item.method.name, detail: item.reason, tone: "validated", actionLabel: item.nextActionLabel, actionView: actionViewFor(item) }))
    : [{ title: "Dataset loaded", detail: "Use Setup to choose variables or complete model fields before QuickPLS recommends specific analyses.", tone: "neutral", actionLabel: "Open Setup", actionView: "analyses" }];
}

export function modelGuidance(input: MethodApplicabilityInput): Array<{ title: string; detail: string; tone: "validated" | "warning" | "neutral"; actionLabel: string; actionView: WorkspaceView }> {
  const shape = modelShape(input.nodes, input.edges);
  const cards: Array<{ title: string; detail: string; tone: "validated" | "warning" | "neutral"; actionLabel: string; actionView: WorkspaceView }> = [];
  if (!input.nodes.length) return [{ title: "No constructs yet", detail: "Create constructs or use Data prefix grouping before selecting SEM methods.", tone: "warning", actionLabel: "Add construct", actionView: "models" }];
  if (shape.hasFormative) cards.push({ title: "Formative construct detected", detail: "PLS/WPLS/GSCA are candidates; CB-SEM and PLSc are blocked for this scope.", tone: "warning", actionLabel: "Review Setup", actionView: "analyses" });
  if (shape.isMediated) cards.push({ title: "Mediation-shaped model", detail: "Review indirect effects and enable bootstrap before reporting mediation inference.", tone: "validated", actionLabel: "Setup bootstrap", actionView: "analyses" });
  if (shape.endogenousConstructs.length) cards.push({ title: "Endogenous constructs present", detail: "PLS, PLSc, IPMA, and PLSpredict can be considered depending on your research objective.", tone: "validated", actionLabel: "Review methods", actionView: "analyses" });
  if (!cards.length) cards.push({ title: "Model can be estimated", detail: "Add structural paths or choose standalone analyses if your research question does not use an SEM diagram.", tone: "neutral", actionLabel: "Open Setup", actionView: "analyses" });
  return cards.slice(0, 3);
}

function evaluateOne(method: MethodDefinition & { id: AnalysisMethodId }, input: MethodApplicabilityInput): MethodApplicability {
  const shape = modelShape(input.nodes, input.edges);
  const base = baseChecks(input);
  const raw = rawDataCheck(input.dataset);
  const sem = semChecks(input, shape);
  const numericIndicators = numericIndicatorsCheck(input.dataset, input.nodes);
  const reflective = reflectiveOnlyCheck(input.nodes);
  const selectedMeta = (column?: string | null) => column ? metadataFor(input.dataset, column) : undefined;
  let checks: RequirementCheck[] = [];
  let status: ApplicabilityStatus = "available";
  let reason = `${method.name} is available for the current project.`;
  let nextActionLabel = "Select method";

  switch (method.id) {
    case "pls_pm":
      checks = [base.runtime, raw, ...sem, numericIndicators];
      status = failed(checks) ? "needs_setup" : "recommended";
      reason = failed(checks) ? firstFailure(checks).detail : "Recommended for a raw numeric SEM model with assigned indicators and structural paths.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Complete setup" : "Run PLS-SEM";
      break;
    case "bootstrap":
      checks = [base.runtime, raw, ...sem, numericIndicators];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Use as an inference add-on after the base estimator is ready.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Complete setup" : "Enable bootstrap";
      break;
    case "permutation":
      checks = [base.runtime, raw, ...sem, numericIndicators, structuralPathCheck(shape)];
      status = failed(checks) ? "needs_setup" : "experimental";
      reason = failed(checks)
        ? firstFailure(checks).detail
        : "Candidate fixed-score Freedman-Lane inference is available for the current structural paths; p values are raw and unadjusted for multiplicity.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Complete setup" : "Setup path randomization";
      break;
    case "plsc":
      checks = [base.runtime, raw, ...sem, numericIndicators, reflective];
      status = failed(checks) ? (reflective.status === "failed" ? "not_applicable" : "needs_setup") : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available for reflective-only PLS models in the documented PLSc scope.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review model" : "Select PLSc";
      break;
    case "wpls":
      checks = [base.runtime, raw, ...sem, numericIndicators, weightColumnCheck(input, selectedMeta(input.settings.caseWeightColumn))];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available because a positive numeric case-weight column is selected.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Choose weight column" : "Select WPLS";
      break;
    case "cta_pls":
      checks = [base.runtime, raw, ...sem, reflective, tetradBlockCheck(input.nodes)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available for reflective indicator blocks with enough indicators for tetrad diagnostics.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review indicators" : "Select CTA-PLS";
      break;
    case "cca":
    case "endogeneity":
    case "nonlinear_effects":
      checks = [base.runtime, raw, ...sem, numericIndicators, structuralPathCheck(shape)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : method.id === "endogeneity" ? "Diagnostic only; available for supported numeric structural predictors and does not prove causality." : "Available as a diagnostic for the current PLS-compatible structural model.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review model" : "Select diagnostic";
      break;
    case "predict":
      checks = [base.runtime, raw, ...sem, numericIndicators, endogenousCheck(shape)];
      status = failed(checks) ? "needs_setup" : "recommended";
      reason = failed(checks) ? firstFailure(checks).detail : "Recommended when prediction or segmentation is part of the research objective.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review Setup" : "Setup prediction";
      break;
    case "ipma":
      checks = [base.runtime, raw, ...sem, numericIndicators, endogenousCheck(shape)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available for endogenous target constructs using PLS total effects as importance.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Choose target" : "Setup IPMA";
      break;
    case "mga":
      checks = [base.runtime, raw, ...sem, numericIndicators, groupColumnCheck(input, selectedMeta(input.settings.groupColumn))];
      status = failed(checks) ? "needs_setup" : "recommended";
      reason = failed(checks) ? firstFailure(checks).detail : "Recommended because a two-group column is selected for MICOM/MGA workflows.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Select group column" : "Setup MICOM/MGA";
      break;
    case "cbsem":
      checks = [base.runtime, raw, ...sem, numericIndicators, reflective, unsupportedSemShapeCheck(shape)];
      status = failed(checks) ? (reflective.status === "failed" || unsupportedSemShapeCheck(shape).status === "failed" ? "not_applicable" : "needs_setup") : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available for reflective raw-data single-group CFA/SEM ML scope.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review model" : "Select CB-SEM/CFA";
      break;
    case "gsca":
      checks = [base.runtime, raw, ...sem, numericIndicators, unsupportedSegmentationShapeCheck(shape)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available for bounded reflective/formative component-model shapes.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review model" : "Select GSCA";
      break;
    case "pca":
      checks = [base.runtime, raw, pcaVariableCheck(input)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available as a standalone numeric-variable analysis; no SEM diagram is required.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Choose variables" : "Select PCA";
      break;
    case "regression":
      checks = [base.runtime, raw, regressionCheck(input)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : `${input.settings.regressionType === "logistic" ? "Logistic" : input.settings.regressionType === "process" ? "Bounded PROCESS-style" : "OLS"} regression setup is complete for selected variables.`;
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Choose regression variables" : "Select regression";
      break;
    case "nca":
      checks = [base.runtime, raw, numericPairCheck(input, input.settings.ncaX, input.settings.ncaY)];
      status = failed(checks) ? "needs_setup" : "available";
      reason = failed(checks) ? firstFailure(checks).detail : "Available for selected numeric X and Y with complete cases.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Choose X/Y" : "Select NCA";
      break;
    case "moderated_mediation":
      checks = [base.runtime, raw, ...sem, numericIndicators, moderatedMediationShapeCheck(shape)];
      status = failed(checks) ? "needs_setup" : "experimental";
      reason = failed(checks) ? firstFailure(checks).detail : "Bounded moderated mediation remains excluded from general validated scope unless explicitly configured.";
      nextActionLabel = failed(checks) ? firstFailure(checks).actionLabel ?? "Review model" : "Review experimental scope";
      break;
  }

  const scopeStatus = status === "not_applicable" ? "unsupported" : status === "experimental" || method.status === "experimental" ? "experimental" : "validated";
  return { method, category: categories[method.id], status, scopeStatus, reason, nextActionLabel, checks, expectedOutputs: outputMap[method.id] };
}

function baseChecks(input: MethodApplicabilityInput) {
  return {
    runtime: input.nativeDesktop
      ? pass("runtime", "Desktop runtime", "QuickPLS desktop runtime is available.")
      : fail("runtime", "Desktop runtime", "Open the native desktop app to run analyses; browser preview can inspect and design only.", "Open Model", "models"),
  };
}

function rawDataCheck(dataset: Dataset): RequirementCheck {
  if (!dataset.columns.length) return fail("raw-data", "Raw dataset", "Import raw data before selecting case-level analyses.", "Import raw dataset", "data");
  if (dataset.kind === "covariance" || dataset.kind === "correlation") return fail("raw-data", "Raw dataset", "This method needs raw case-level data; covariance/correlation matrices support only matrix-compatible estimation.", "Import raw dataset", "data");
  if (!dataset.fingerprint) return fail("raw-data", "Raw dataset", "Import the dataset into the desktop project so QuickPLS can store a reproducible data fingerprint.", "Import raw dataset", "data");
  return pass("raw-data", "Raw dataset", `${dataset.name} is available as raw fingerprinted data.`);
}

function semChecks(input: MethodApplicabilityInput, shape: ReturnType<typeof modelShape>): RequirementCheck[] {
  const issues = validateModel(input.nodes, input.edges);
  return [
    input.nodes.length ? pass("constructs", "Constructs", `${input.nodes.length} constructs are present.`) : fail("constructs", "Constructs", "Create at least one construct before SEM methods can run.", "Open Model", "models"),
    shape.indicatorCount ? pass("indicators", "Indicators", `${shape.indicatorCount} indicators are assigned.`) : fail("indicators", "Indicators", "Assign observed indicators to constructs before SEM methods can run.", "Assign indicators", "models"),
    issues.length ? fail("model-structure", "Model structure", "Resolve duplicate paths, self paths, missing constructs, cycles, or duplicate indicators.", "Validate diagram", "models") : pass("model-structure", "Model structure", "No blocking model-structure issues detected."),
  ];
}

function numericIndicatorsCheck(dataset: Dataset, nodes: Array<Node<ConstructData>>): RequirementCheck {
  const metadata = metadataMap(dataset);
  const nonNumeric = nodes.flatMap((node) => node.data.indicators).filter((indicator) => !isNumericColumn(dataset, indicator, metadata.get(indicator)));
  return nonNumeric.length
    ? fail("numeric-indicators", "Numeric indicators", `These indicators are not numeric in metadata: ${nonNumeric.slice(0, 4).join(", ")}.`, "Review Data metadata", "data")
    : pass("numeric-indicators", "Numeric indicators", "All assigned indicators are numeric-compatible.");
}

function reflectiveOnlyCheck(nodes: Array<Node<ConstructData>>): RequirementCheck {
  const formative = nodes.filter((node) => node.data.mode === "formative").map((node) => node.data.label);
  return formative.length ? fail("reflective-only", "Reflective constructs only", `This documented scope requires reflective constructs; formative construct(s): ${formative.join(", ")}.`, "Use reflective constructs only", "models") : pass("reflective-only", "Reflective constructs only", "All constructs are reflective.");
}

function structuralPathCheck(shape: ReturnType<typeof modelShape>) {
  return shape.structuralEdges.length ? pass("structural-paths", "Structural paths", `${shape.structuralEdges.length} structural paths are present.`) : fail("structural-paths", "Structural paths", "Add at least one structural path for this diagnostic.", "Open Model", "models");
}

function endogenousCheck(shape: ReturnType<typeof modelShape>) {
  return shape.endogenousConstructs.length ? pass("endogenous", "Endogenous target", `${shape.endogenousConstructs.length} endogenous construct(s) are available.`) : fail("endogenous", "Endogenous target", "Add an endogenous construct with an incoming structural path.", "Open Model", "models");
}

function tetradBlockCheck(nodes: Array<Node<ConstructData>>) {
  const eligible = nodes.filter((node) => node.data.mode === "reflective" && node.data.indicators.length >= 4);
  return eligible.length ? pass("tetrad-blocks", "CTA-PLS indicator blocks", `${eligible.length} reflective block(s) have at least four indicators.`) : fail("tetrad-blocks", "CTA-PLS indicator blocks", "Add at least four indicators to a reflective construct for CTA-PLS tetrad diagnostics.", "Add at least four indicators", "models");
}

function weightColumnCheck(input: MethodApplicabilityInput, meta?: ColumnMetadata): RequirementCheck {
  const column = input.settings.caseWeightColumn;
  if (!column) return fail("case-weight", "Case weight column", "Choose a positive numeric weight column before running WPLS.", "Choose positive numeric weight column", "analyses");
  if (!isNumericColumn(input.dataset, column, meta)) return fail("case-weight", "Case weight column", `${column} is not numeric in the current metadata.`, "Review Data metadata", "data");
  const invalid = input.dataset.rows.some((row) => Number(row[column]) <= 0 || !Number.isFinite(Number(row[column])));
  return invalid ? fail("case-weight", "Case weight column", `${column} contains nonpositive or invalid weights.`, "Choose positive numeric weight column", "analyses") : pass("case-weight", "Case weight column", `${column} is a positive numeric weight column.`);
}

function groupColumnCheck(input: MethodApplicabilityInput, meta?: ColumnMetadata): RequirementCheck {
  const column = input.settings.groupColumn;
  if (!column) return fail("group-column", "Group column", "Select an observed group column for MICOM/MGA.", "Select group column", "analyses");
  if (!input.dataset.columns.includes(column)) return fail("group-column", "Group column", `${column} is not in the current dataset.`, "Select group column", "analyses");
  const groups = distinctValues(input.dataset, column);
  if (groups.length !== 2) return fail("group-column", "Group column", `Documented MICOM/MGA scope expects exactly two groups; ${column} has ${groups.length}.`, "Select group column", "analyses");
  if (meta?.scale_type === "continuous") return warn("group-column", "Group column", `${column} has two observed groups but is marked continuous; confirm it is a categorical grouping variable.`);
  return pass("group-column", "Group column", `${column} has two observed groups.`);
}

function pcaVariableCheck(input: MethodApplicabilityInput): RequirementCheck {
  const selected = splitList(input.settings.pcaVariables);
  if (!selected.length) return fail("pca-variables", "PCA variables", "Select at least two numeric variables for standalone PCA.", "Select PCA variables", "analyses");
  const invalid = selected.filter((column) => !isNumericColumn(input.dataset, column, metadataFor(input.dataset, column)));
  if (invalid.length) return fail("pca-variables", "PCA variables", `PCA variables must be numeric in this scope: ${invalid.join(", ")}.`, "Review Data metadata", "data");
  return selected.length >= 2 ? pass("pca-variables", "PCA variables", `${selected.length} numeric variable(s) selected.`) : fail("pca-variables", "PCA variables", "Select at least two numeric variables for standalone PCA.", "Select PCA variables", "analyses");
}

function regressionCheck(input: MethodApplicabilityInput): RequirementCheck {
  const outcome = input.settings.regressionOutcome;
  const predictors = splitList(input.settings.regressionPredictors);
  if (!outcome) return fail("regression-outcome", "Regression outcome", "Choose an outcome variable for regression.", "Choose outcome", "analyses");
  if (!predictors.length) return fail("regression-predictors", "Regression predictors", "Choose at least one predictor variable for regression.", "Choose predictors", "analyses");
  const columns = [outcome, ...predictors, ...splitList(input.settings.regressionControls)];
  const nonNumeric = columns.filter((column) => !isNumericColumn(input.dataset, column, metadataFor(input.dataset, column)));
  if (nonNumeric.length) return fail("regression-numeric", "Numeric regression variables", `Regression variables must be numeric in this scope: ${nonNumeric.join(", ")}.`, "Review Data metadata", "data");
  if (input.settings.regressionType === "logistic" && !isBinaryColumn(input.dataset, outcome, metadataFor(input.dataset, outcome))) {
    return fail("logistic-binary", "Binary outcome", "Logistic regression requires a binary 0/1 or two-level outcome.", "Choose binary outcome", "analyses");
  }
  if (input.settings.regressionType === "process" && !input.settings.processGraph) {
    return fail("process-scope", "PROCESS scope", "New PROCESS work requires an explicit graph-defined v2 relationship; historical generated v1 templates are archive-only.", "Author PROCESS graph", "analyses");
  }
  return pass("regression-variables", "Regression variables", "Required regression variables are selected and numeric-compatible.");
}

function numericPairCheck(input: MethodApplicabilityInput, x?: string | null, y?: string | null): RequirementCheck {
  if (!x || !y) return fail("nca-pair", "NCA X/Y", "Select numeric X and numeric Y variables for NCA.", "Select NCA X/Y", "analyses");
  const invalid = [x, y].filter((column) => !isNumericColumn(input.dataset, column, metadataFor(input.dataset, column)));
  return invalid.length ? fail("nca-pair", "NCA X/Y", `NCA requires numeric variables; review ${invalid.join(", ")}.`, "Review Data metadata", "data") : pass("nca-pair", "NCA X/Y", `${x} and ${y} are numeric-compatible.`);
}

function unsupportedSemShapeCheck(shape: ReturnType<typeof modelShape>): RequirementCheck {
  if (shape.hasFormative) return fail("unsupported-cbsem-shape", "CB-SEM supported shape", "CB-SEM/CFA v1.8 scope supports reflective constructs only; formative constructs are blocked.", "Review Model", "models");
  if (shape.hasGeneratedConstructs) return fail("unsupported-cbsem-shape", "CB-SEM supported shape", "CB-SEM/CFA v1.8 scope blocks generated interactions and higher-order constructs.", "Review Model", "models");
  return pass("unsupported-cbsem-shape", "CB-SEM supported shape", "No formative, generated interaction, or higher-order constructs detected.");
}

function unsupportedSegmentationShapeCheck(shape: ReturnType<typeof modelShape>): RequirementCheck {
  return shape.hasGeneratedConstructs ? fail("unsupported-generated-shape", "Supported bounded shape", "This bounded segmentation/component scope blocks generated interactions and higher-order constructs.", "Review Model", "models") : pass("unsupported-generated-shape", "Supported bounded shape", "No generated construct shape blocks this method.");
}

function moderatedMediationShapeCheck(shape: ReturnType<typeof modelShape>): RequirementCheck {
  return shape.hasGeneratedConstructs && shape.isMediated ? pass("moderated-mediation-shape", "Moderated mediation shape", "Interaction and mediated paths are present.") : fail("moderated-mediation-shape", "Moderated mediation shape", "Create a supported interaction plus mediated structural path before moderated mediation.", "Review Model", "models");
}

function modelShape(nodes: Array<Node<ConstructData>>, edges: Edge[]) {
  const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance" && !edge.id.startsWith("measurement::"));
  const incoming = new Map<string, number>();
  const outgoing = new Map<string, number>();
  structuralEdges.forEach((edge) => {
    incoming.set(edge.target, (incoming.get(edge.target) ?? 0) + 1);
    outgoing.set(edge.source, (outgoing.get(edge.source) ?? 0) + 1);
  });
  const endogenousConstructs = nodes.filter((node) => (incoming.get(node.id) ?? 0) > 0).map((node) => node.id);
  const mediators = nodes.filter((node) => (incoming.get(node.id) ?? 0) > 0 && (outgoing.get(node.id) ?? 0) > 0).map((node) => node.id);
  return {
    structuralEdges,
    indicatorCount: nodes.reduce((sum, node) => sum + node.data.indicators.length, 0),
    endogenousConstructs,
    mediators,
    isMediated: mediators.length > 0,
    hasFormative: nodes.some((node) => node.data.mode === "formative"),
    hasGeneratedConstructs: nodes.some((node) => node.data.semantic === "interaction" || node.data.semantic === "higher_order"),
  };
}

function pass(id: string, label: string, detail: string): RequirementCheck {
  return { id, label, detail, status: "passed" };
}
function warn(id: string, label: string, detail: string): RequirementCheck {
  return { id, label, detail, status: "warning" };
}
function fail(id: string, label: string, detail: string, actionLabel?: string, actionView?: WorkspaceView): RequirementCheck {
  return { id, label, detail, status: "failed", actionLabel, actionView };
}
function failed(checks: RequirementCheck[]) {
  return checks.some((check) => check.status === "failed");
}
function firstFailure(checks: RequirementCheck[]) {
  return checks.find((check) => check.status === "failed") ?? checks[0];
}
function metadataMap(dataset: Dataset) {
  return new Map((dataset.columnMetadata ?? []).map((column) => [column.name, column]));
}
function metadataFor(dataset: Dataset, column: string) {
  return metadataMap(dataset).get(column);
}
function isNumericColumn(dataset: Dataset, column: string, metadata?: ColumnMetadata) {
  if (!dataset.columns.includes(column)) return false;
  if (metadata?.scale_type === "identifier" || metadata?.scale_type === "nominal") return false;
  if (metadata?.column_type === "numeric" || metadata?.column_type === "boolean" || metadata?.scale_type === "binary" || metadata?.scale_type === "ordinal") return true;
  return dataset.rows.some((row) => Number.isFinite(Number(row[column])));
}
function isBinaryColumn(dataset: Dataset, column: string, metadata?: ColumnMetadata) {
  if (metadata?.scale_type === "binary" || metadata?.column_type === "boolean") return true;
  return distinctValues(dataset, column).length === 2;
}
function distinctValues(dataset: Dataset, column: string) {
  return [...new Set(dataset.rows.map((row) => row[column]).filter((value) => value !== null && value !== "" && value !== undefined).map(String))];
}
function splitList(value?: string | null) {
  return (value ?? "").split(",").map((item) => item.trim()).filter(Boolean);
}
function actionViewFor(item: MethodApplicability): WorkspaceView {
  return item.checks.find((check) => check.status === "failed" && check.actionView)?.actionView ?? "analyses";
}
