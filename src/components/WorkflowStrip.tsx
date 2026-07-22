import { CheckCircle2, Circle } from "lucide-react";
import { analysisReadiness } from "../domain/analysisReadiness";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { WorkspaceView } from "../types";

const steps: Array<{ view: WorkspaceView; label: string }> = [
  { view: "data", label: "Data" },
  { view: "models", label: "Model" },
  { view: "analyses", label: "Setup" },
  { view: "run", label: "Run" },
  { view: "runs", label: "Results" },
  { view: "reports", label: "Report" },
];

export function WorkflowStrip() {
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.setView);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const settings = useWorkspace((state) => state.analysisSettings);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const completedRuns = runs.filter((run) => run.status === "completed" && run.result);
  const complete = {
    welcome: true,
    data: dataset.columns.length > 0 && (dataset.rowCount ?? dataset.rows.length) > 0,
    models: nodes.length > 0 && nodes.every((node) => node.data.indicators.length > 0) && edges.length > 0,
    analyses: readiness.items.filter((item) => item.id !== "runtime").every((item) => item.status !== "blocked"),
    run: completedRuns.length > 0,
    runs: completedRuns.length > 0,
    reports: completedRuns.length > 0,
    groups: runs.some((run) => Boolean(run.result?.segmentation || run.result?.mga || run.result?.micom || run.result?.mga_permutation || run.result?.fimix || run.result?.ipma)),
  } satisfies Record<WorkspaceView, boolean>;
  return <nav className="workflow-strip" aria-label="Research workflow">
    {steps.map((step) => {
      const active = step.view === view;
      const completed = complete[step.view];
      return <button
        key={step.view}
        type="button"
        className={`workflow-step${active ? " active" : ""}${completed ? " completed" : ""}`}
        aria-current={active ? "step" : undefined}
        aria-label={`${step.label}${completed ? ", complete" : ""}`}
        onClick={() => setView(step.view)}
      >
        {completed ? <CheckCircle2 size={14} /> : <Circle size={14} />}
        <span>{step.label}</span>
      </button>;
    })}
  </nav>;
}
