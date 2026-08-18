import { analysisReadiness } from "./analysisReadiness";
import type { AnalysisRun, AnalysisUiSettings, ConstructData, Dataset, WorkspaceView } from "../types";
import type { Edge, Node } from "@xyflow/react";
import type { WorkspaceCommandEvent } from "./workspaceCommands";

export interface WorkflowCoachAction {
  label: string;
  view?: WorkspaceView;
  event?: WorkspaceCommandEvent;
  disabled?: boolean;
  reason?: string;
}

export interface WorkflowCoachMessage {
  id: string;
  title: string;
  detail: string;
  status: "ready" | "review" | "blocked" | "info";
  primary: WorkflowCoachAction;
  secondary?: WorkflowCoachAction;
}

function coachLabel(label: string) {
  const normalized: Record<string, string> = {
    "Import data": "Import Data",
    "Open data": "Open Data",
    "Open model": "Open Model",
    "Open setup": "Open Setup",
    "Open run": "Open Run",
    "Open results": "Open Results",
    "Prepare report": "Prepare Report",
    "Resolve blocker": "Resolve Blocker",
    "Run method": "Run Method",
    "Run now": "Run Now",
  };
  return normalized[label] ?? label;
}

export function workflowCoachMessage(input: {
  view: WorkspaceView;
  dataset: Dataset;
  nodes: Array<Node<ConstructData>>;
  edges: Edge[];
  runs: AnalysisRun[];
  settings: AnalysisUiSettings;
  nativeDesktop: boolean;
}): WorkflowCoachMessage {
  const { view, dataset, nodes, edges, runs, settings, nativeDesktop } = input;
  const hasData = dataset.columns.length > 0 && (dataset.rowCount ?? dataset.rows.length) > 0;
  const hasModel = nodes.length > 0 && nodes.every((node) => node.data.indicators.length > 0) && edges.some((edge) => edge.data?.role !== "covariance");
  const completedRuns = runs.filter((run) => run.status === "completed" && run.result);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop });
  const blocker = readiness.blockers[0];

  if (view === "welcome") {
    if (!hasData) {
      return {
        id: "home-import-data",
        title: "Start with a reproducible dataset",
        detail: "Import a raw dataset or open the demo project before building a reportable SEM workflow.",
        status: "review",
        primary: { label: "Open Data", view: "data" },
        secondary: { label: "Open project", event: "quickpls:open-project" },
      };
    }
    if (!hasModel) {
      return {
        id: "home-build-model",
        title: "Dataset is ready; build the SEM diagram next",
        detail: `${dataset.name} is loaded. Create constructs, assign indicators, and draw paths before setup.`,
        status: "ready",
        primary: { label: "Open Model", view: "models" },
        secondary: { label: "Open Data", view: "data" },
      };
    }
    return {
      id: "home-ready-setup",
      title: "Workspace has data and a SEM diagram",
      detail: "Review method setup, run the calculation when readiness is clear, or prepare the report from an existing run.",
      status: completedRuns.length ? "ready" : "info",
      primary: { label: readiness.canRun ? "Run now" : "Open Setup", view: readiness.canRun ? "run" : "analyses", event: readiness.canRun ? "quickpls:run-analysis" : undefined },
      secondary: completedRuns.length ? { label: "Prepare Report", view: "reports" } : { label: "Open Results", view: "runs" },
    };
  }

  if (view === "data") {
    return {
      id: hasData ? "data-continue-model" : "data-import",
      title: hasData ? "Data is available for model building" : "Import data before modeling",
      detail: hasData
        ? `${dataset.name} has ${dataset.rowCount ?? dataset.rows.length} rows and ${dataset.columns.length} variables. Continue to the SEM designer or refine metadata first.`
        : "Use Import Data or Load Sample Dataset, then inspect quality and metadata before creating constructs.",
      status: hasData ? "ready" : "review",
      primary: hasData ? { label: "Open Model", view: "models" } : { label: "Import Data", event: "quickpls:import-data" },
      secondary: { label: "Open Setup", view: "analyses", disabled: !hasData, reason: "Setup needs a dataset first." },
    };
  }

  if (view === "models") {
    if (!hasData) {
      return {
        id: "model-needs-data",
        title: "Import data before building the SEM diagram",
        detail: "The designer can preview sample structures, but reportable model setup needs a reproducible dataset first.",
        status: "blocked",
        primary: { label: "Open Data", view: "data" },
        secondary: { label: "Import Data", event: "quickpls:import-data" },
      };
    }
    if (!hasModel) {
      return {
        id: "model-build-diagram",
        title: "Build a complete SEM diagram",
        detail: "Create constructs, assign indicators, and draw at least one structural path before method setup.",
        status: "review",
        primary: { label: "Open Setup", view: "analyses" },
        secondary: { label: "Open Data", view: "data" },
      };
    }
    return {
      id: readiness.canRun ? "model-ready-run" : "model-ready-setup",
      title: readiness.canRun ? "Diagram is ready for calculation" : "Diagram is ready; complete method setup",
      detail: readiness.canRun
        ? "The current diagram, data, and selected method are ready. You can run now or review setup before calculation."
        : blocker?.detail ?? "The SEM diagram is structurally ready. Review method-specific settings before running.",
      status: readiness.canRun ? "ready" : "info",
      primary: { label: readiness.canRun ? "Run now" : "Open Setup", view: readiness.canRun ? "run" : "analyses", event: readiness.canRun ? "quickpls:run-analysis" : undefined },
      secondary: { label: "Open Data", view: "data" },
    };
  }

  if (view === "analyses") {
    return {
      id: readiness.canRun ? "setup-ready-run" : "setup-needs-work",
      title: readiness.canRun ? "Setup is ready for calculation" : "Setup is not ready yet",
      detail: readiness.canRun
        ? "The selected method has the required data, model, and settings. Run now or review Method Details before calculation."
        : blocker?.detail ?? readiness.summary,
      status: readiness.canRun ? "ready" : "blocked",
      primary: { label: readiness.canRun ? "Run Now" : coachLabel(blocker?.actionLabel ?? "Resolve Blocker"), view: readiness.canRun ? "run" : blocker?.actionView, event: readiness.canRun ? "quickpls:run-analysis" : undefined },
      secondary: { label: "Review Model", view: "models" },
    };
  }

  if (view === "run") {
    return {
      id: readiness.canRun ? "run-launch" : "run-blocked",
      title: readiness.canRun ? "Calculation package is ready" : "Run is blocked",
      detail: readiness.canRun
        ? "Launch the selected method from this page or the top command bar. Completed runs are saved with provenance."
        : blocker?.detail ?? readiness.summary,
      status: readiness.canRun ? "ready" : "blocked",
      primary: { label: "Run selected method", event: "quickpls:run-analysis", disabled: !readiness.canRun, reason: blocker?.detail },
      secondary: { label: "Open Setup", view: "analyses" },
    };
  }

  if (view === "runs") {
    return {
      id: completedRuns.length ? "results-review" : "results-empty",
      title: completedRuns.length ? "Review completed results" : "No completed run yet",
      detail: completedRuns.length
        ? "Use Results to inspect findings, tables, warnings, analysis details, and row-level interpretation."
        : readiness.canRun ? "Run the selected method to unlock result tables and interpretation." : blocker?.detail ?? readiness.summary,
      status: completedRuns.length ? "ready" : readiness.canRun ? "review" : "blocked",
      primary: completedRuns.length ? { label: "Prepare Report", view: "reports" } : readiness.canRun ? { label: "Run Method", event: "quickpls:run-analysis" } : { label: coachLabel(blocker?.actionLabel ?? "Open Setup"), view: blocker?.actionView ?? "analyses" },
      secondary: { label: "Open Setup", view: "analyses" },
    };
  }

  if (view === "reports") {
    return {
      id: completedRuns.length ? "report-ready" : "report-needs-run",
      title: completedRuns.length ? "Report outputs are ready to review" : "Run a method before exporting",
      detail: completedRuns.length
        ? "Choose a preset, preview the publication diagram, and export SVG, CSV, HTML, or XLSX with provenance."
        : "Reports need a completed run so exported tables and figures can reference exact results.",
      status: completedRuns.length ? "ready" : "blocked",
      primary: completedRuns.length ? { label: "Review Report", view: "reports" } : { label: "Open Run", view: "run" },
      secondary: { label: "Open Results", view: "runs", disabled: !completedRuns.length, reason: "Results need a completed run." },
    };
  }

  if (view === "trust") {
    return {
      id: "trust-scope",
      title: "Review Method Details before reporting",
      detail: "Methods & References lists requirements, assumptions, known limitations, and offline behavior.",
      status: "info",
      primary: { label: "Open Setup", view: "analyses" },
      secondary: { label: "Open Results", view: "runs" },
    };
  }

  if (view === "settings") {
    return {
      id: "settings-preferences",
      title: "Tune the desktop workspace without changing results",
      detail: "Settings are UI preferences only; they do not change formulas, result schemas, or numerical fingerprints.",
      status: "info",
      primary: { label: "Open Data", view: "data" },
      secondary: { label: "Open Trust Center", view: "trust" },
    };
  }

  return {
    id: "workflow-continue",
    title: "Continue the research workflow",
    detail: "Move from data to model, setup, run, results, and report with requirement-aware checks.",
    status: "info",
    primary: { label: "Open Setup", view: "analyses" },
  };
}
