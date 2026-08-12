import type { Edge, Node } from "@xyflow/react";
import { analysisReadiness } from "./analysisReadiness";
import type { AnalysisUiSettings, ConstructData, Dataset, WorkspaceView } from "../types";

export type WorkflowStepState = "complete" | "current" | "next" | "blocked" | "ready" | "pending";

export interface WorkflowStepStatus {
  view: WorkspaceView;
  label: string;
  state: WorkflowStepState;
  detail: string;
  actionLabel: string;
}

export interface WorkflowProgressInput {
  view: WorkspaceView;
  dataset: Dataset;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  runs: Array<{ status: string; result?: unknown }>;
  settings: AnalysisUiSettings;
  nativeDesktop: boolean;
}

const workflowSteps: Array<{ view: WorkspaceView; label: string }> = [
  { view: "data", label: "Data" },
  { view: "models", label: "Model" },
  { view: "analyses", label: "Setup" },
  { view: "run", label: "Run" },
  { view: "runs", label: "Results" },
  { view: "reports", label: "Report" },
];

export function workflowProgress(input: WorkflowProgressInput): WorkflowStepStatus[] {
  const { view, dataset, nodes, edges, runs, settings, nativeDesktop } = input;
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop });
  const rowCount = dataset.rowCount ?? dataset.rows.length;
  const hasData = dataset.columns.length > 0 && rowCount > 0;
  const hasFingerprint = Boolean(dataset.fingerprint);
  const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance");
  const hasModel = nodes.length > 0 && nodes.every((node) => node.data.indicators.length > 0) && structuralEdges.length > 0;
  const completedRuns = runs.filter((run) => run.status === "completed" && run.result);
  const hasCompletedRun = completedRuns.length > 0;

  const baseStatuses: Record<string, Omit<WorkflowStepStatus, "view" | "label">> = {
    data: {
      state: hasData ? "complete" : "next",
      detail: hasData
        ? `${dataset.name} is loaded with ${rowCount} rows and ${dataset.columns.length} variables${hasFingerprint ? "." : "; import into the desktop project before running."}`
        : "Import raw data, a matrix input, or load a sample dataset.",
      actionLabel: hasData ? "Inspect data" : "Import data",
    },
    models: {
      state: !hasData ? "blocked" : hasModel ? "complete" : "next",
      detail: !hasData
        ? "Import data first so constructs can be assigned to variables."
        : hasModel
          ? `${nodes.length} constructs and ${structuralEdges.length} structural paths are available.`
          : "Create constructs, assign indicators, and draw structural paths.",
      actionLabel: hasModel ? "Review diagram" : "Build model",
    },
    analyses: {
      state: !hasData || !hasModel ? "blocked" : readiness.canRun ? "complete" : "next",
      detail: !hasData || !hasModel
        ? "Complete Data and Model before calculation setup."
        : readiness.canRun
          ? "Method setup is ready for the selected documented scope."
          : readiness.blockers[0]?.detail ?? readiness.summary,
      actionLabel: readiness.canRun ? "Review setup" : "Resolve setup",
    },
    run: {
      state: !readiness.canRun ? "blocked" : hasCompletedRun ? "complete" : "next",
      detail: !readiness.canRun
        ? readiness.blockers[0]?.detail ?? readiness.summary
        : hasCompletedRun
          ? `${completedRuns.length} completed run${completedRuns.length === 1 ? "" : "s"} saved.`
          : "Launch the selected method and save the immutable run.",
      actionLabel: hasCompletedRun ? "Run again" : "Run method",
    },
    runs: {
      state: hasCompletedRun ? "complete" : "blocked",
      detail: hasCompletedRun
        ? "Review tables, findings, diagnostics, and interpretation for completed runs."
        : "Run a method before results are available.",
      actionLabel: hasCompletedRun ? "Review results" : "No results yet",
    },
    reports: {
      state: hasCompletedRun ? "ready" : "blocked",
      detail: hasCompletedRun
        ? "Prepare publication diagrams, tables, and reproducibility exports."
        : "Run a method before report exports are available.",
      actionLabel: hasCompletedRun ? "Prepare report" : "Needs run",
    },
  };

  return workflowSteps.map((step) => {
    const base = baseStatuses[step.view] ?? { state: "pending", detail: "Workspace is available.", actionLabel: "Open" };
    return {
      ...step,
      ...base,
      state: step.view === view ? "current" : base.state,
    };
  });
}

export function workflowStepStatusSummary(steps: WorkflowStepStatus[]): string {
  const blocked = steps.filter((step) => step.state === "blocked").length;
  const complete = steps.filter((step) => step.state === "complete").length;
  const next = steps.find((step) => step.state === "next" || step.state === "current" || step.state === "ready");
  if (blocked > 0) return `${blocked} workflow step${blocked === 1 ? "" : "s"} need attention; next: ${next?.label ?? "Data"}.`;
  return `${complete} workflow step${complete === 1 ? "" : "s"} complete; next: ${next?.label ?? "Report"}.`;
}
