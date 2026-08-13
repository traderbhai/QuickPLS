import { ArrowRightCircle, CheckCircle2, Circle, Clock3, Lock } from "lucide-react";
import { workflowProgress, workflowStepStatusSummary, type WorkflowStepState } from "../domain/workflowProgress";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { WorkspaceView } from "../types";

export function WorkflowStrip() {
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.setView);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const settings = useWorkspace((state) => state.analysisSettings);
  const steps = workflowProgress({ view, dataset, nodes, edges, runs, settings, nativeDesktop: isNativeDesktop() });
  const summary = workflowStepStatusSummary(steps);
  const openStep = (targetView: WorkspaceView, label: string) => {
    if (targetView === view) {
      setView(targetView);
      return;
    }
    setView(targetView, {
      from: view,
      to: targetView,
      actionLabel: `Workflow: ${label}`,
      coachId: "workflow-strip",
    });
  };
  return <nav
    className="workflow-strip"
    aria-label="Primary research workflow progress"
    aria-description={`${workflowStepStatusSummary(steps)} Support destinations are available from the left navigation rail.`}
    data-workflow-scope="primary-research-workflow"
    data-workflow-count={steps.length}
  >
    <span className="workflow-strip-label" aria-hidden="true">Workflow</span>
    {steps.map((step) => {
      return <button
        key={step.view}
        type="button"
        className={`workflow-step ${step.state}`}
        data-workflow-state={step.state}
        data-workflow-view={step.view}
        data-workflow-label={step.label}
        data-workflow-action={step.actionLabel}
        data-workflow-detail={step.detail}
        title={step.detail}
        aria-current={step.view === view ? "step" : undefined}
        aria-label={`${step.label}: ${step.state}. ${step.detail}. ${summary}`}
        onClick={() => openStep(step.view, step.label)}
      >
        <WorkflowStepIcon state={step.state} />
        <span>
          <strong>{step.label}</strong>
          <small>{step.actionLabel}</small>
        </span>
      </button>;
    })}
  </nav>;
}

function WorkflowStepIcon({ state }: { state: WorkflowStepState }) {
  if (state === "complete") return <CheckCircle2 size={14} />;
  if (state === "current") return <ArrowRightCircle size={14} />;
  if (state === "blocked") return <Lock size={14} />;
  if (state === "ready") return <Clock3 size={14} />;
  return <Circle size={14} />;
}
