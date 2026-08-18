import type { Edge, Node } from "@xyflow/react";
import { methods } from "../data/sample";
import { methodApplicabilityFor } from "./methodApplicability";
import { effectiveMethodStatus } from "./methodStatus";
import type { AnalysisUiSettings, ConstructData, Dataset, WorkspaceView } from "../types";
import { validateModel, type ModelIssue } from "./modelValidation";

export interface ReadinessItem {
  id: string;
  label: string;
  detail: string;
  status: "ready" | "warning" | "blocked";
  actionLabel?: string;
  actionView?: WorkspaceView;
}

export interface AnalysisReadiness {
  canRun: boolean;
  summary: string;
  blockers: ReadinessItem[];
  warnings: ReadinessItem[];
  items: ReadinessItem[];
}

export interface AnalysisReadinessInput {
  dataset: Dataset;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  settings: AnalysisUiSettings;
  nativeDesktop: boolean;
}

export function analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop }: AnalysisReadinessInput): AnalysisReadiness {
  const issues = validateModel(nodes, edges);
  const method = methods.find((candidate) => candidate.id === settings.method);
  const applicability = methodApplicabilityFor(settings.method, { dataset, nodes, edges, settings, nativeDesktop });
  const items: ReadinessItem[] = [
    dataItem(dataset, nativeDesktop),
    constructItem(nodes),
    indicatorItem(nodes, issues),
    modelIssueItem(issues),
    sampleSizeItem(dataset, nodes),
    methodItem(method?.name ?? settings.method, effectiveMethodStatus(method, settings), applicability),
  ];
  const blockers = items.filter((item) => item.status === "blocked");
  const warnings = items.filter((item) => item.status === "warning");
  return {
    canRun: blockers.length === 0,
    summary: blockers.length
      ? `${blockers.length} blocker${blockers.length === 1 ? "" : "s"} before analysis`
      : warnings.length
        ? `Ready with ${warnings.length} warning${warnings.length === 1 ? "" : "s"}`
        : "Ready to run",
    blockers,
    warnings,
    items,
  };
}

function dataItem(dataset: Dataset, nativeDesktop: boolean): ReadinessItem {
  if (!nativeDesktop) {
    return {
      id: "runtime",
      label: "Runtime",
      detail: "Analysis runs require the offline QuickPLS desktop runtime; the web preview can inspect and design models.",
      status: "blocked",
      actionLabel: "Open model",
      actionView: "models",
    };
  }
  if (!dataset.fingerprint) {
    const hasVisibleData = dataset.rows.length > 0 && dataset.columns.length > 0;
    return {
      id: "data",
      label: "Data",
      detail: hasVisibleData
        ? "This dataset is available for design or preview, but it has not been imported into the desktop project with a reproducible fingerprint. Load the bundled example or import your data before running."
        : "Import a dataset into the project so QuickPLS can fingerprint the analysis input.",
      status: "blocked",
      actionLabel: "Open data",
      actionView: "data",
    };
  }
  return {
    id: "data",
    label: "Data",
    detail: `${dataset.name} is loaded with ${dataset.rows.length} rows and ${dataset.columns.length} variables.`,
    status: "ready",
  };
}

function constructItem(nodes: Array<Node<ConstructData>>): ReadinessItem {
  if (nodes.length === 0) {
    return { id: "constructs", label: "Constructs", detail: "Create at least one construct before running an analysis.", status: "blocked" };
  }
  return { id: "constructs", label: "Constructs", detail: `${nodes.length} constructs are present.`, status: "ready" };
}

function indicatorItem(nodes: Array<Node<ConstructData>>, issues: ModelIssue[]): ReadinessItem {
  const emptyConstructs = issues.filter((issue) => issue.code === "construct.no_indicators").length;
  const duplicateIndicators = issues.filter((issue) => issue.code === "indicator.duplicate").length;
  if (emptyConstructs || duplicateIndicators) {
    return {
      id: "indicators",
      label: "Indicators",
      detail: [
        emptyConstructs ? `${emptyConstructs} construct${emptyConstructs === 1 ? "" : "s"} need indicators` : null,
        duplicateIndicators ? `${duplicateIndicators} duplicate indicator assignment${duplicateIndicators === 1 ? "" : "s"}` : null,
      ].filter(Boolean).join("; "),
      status: "blocked",
    };
  }
  const indicatorCount = nodes.reduce((sum, node) => sum + node.data.indicators.length, 0);
  return { id: "indicators", label: "Indicators", detail: `${indicatorCount} indicators are assigned.`, status: "ready" };
}

function modelIssueItem(issues: ModelIssue[]): ReadinessItem {
  const structuralIssues = issues.filter((issue) => issue.code !== "construct.no_indicators" && issue.code !== "indicator.duplicate");
  if (structuralIssues.length) {
    return {
      id: "model",
      label: "Model structure",
      detail: `${structuralIssues.length} structural issue${structuralIssues.length === 1 ? "" : "s"} need correction.`,
      status: "blocked",
    };
  }
  return { id: "model", label: "Model structure", detail: "No duplicate paths, self paths, missing constructs, or cycles detected.", status: "ready" };
}

function sampleSizeItem(dataset: Dataset, nodes: Array<Node<ConstructData>>): ReadinessItem {
  if (dataset.rows.length < 10) {
    return {
      id: "sample-size",
      label: "Sample size",
      detail: `${dataset.rows.length} rows are available. This is useful for demos but too small for publication analysis.`,
      status: "warning",
    };
  }
  const indicatorCount = nodes.reduce((sum, node) => sum + node.data.indicators.length, 0);
  return {
    id: "sample-size",
    label: "Sample size",
    detail: `${dataset.rows.length} rows for ${indicatorCount} indicators.`,
    status: "ready",
  };
}

function methodItem(methodName: string, methodStatus: string, applicability: ReturnType<typeof methodApplicabilityFor>): ReadinessItem {
  const failedCheck = applicability.checks.find((check) => check.status === "failed");
  if (failedCheck) {
    return {
      id: "method",
      label: failedCheck.label,
      detail: failedCheck.detail,
      status: "blocked",
      actionLabel: failedCheck.actionLabel ?? "Open setup",
      actionView: failedCheck.actionView ?? "analyses",
    };
  }
  if (methodStatus === "unsupported") {
    return { id: "method", label: "Method", detail: `${methodName} is not runnable in this build.`, status: "blocked" };
  }
  return {
    id: "method",
    label: "Method",
    detail: applicability.reason || `${methodName} is ${methodStatus === "validated" ? "supported for the current model and data requirements" : "Experimental and marked in exports"}.`,
    status: applicability.status === "experimental" || methodStatus !== "validated" ? "warning" : "ready",
  };
}
