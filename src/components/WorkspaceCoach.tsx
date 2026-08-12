import { ArrowRight, CheckCircle2, Info, TriangleAlert } from "lucide-react";
import { workflowCoachMessage, type WorkflowCoachAction } from "../domain/workflowCoach";
import { analysisReadiness } from "../domain/analysisReadiness";
import { dispatchWorkspaceCommand } from "../domain/workspaceCommands";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { WorkspaceView } from "../types";

function commandLabel(event: string) {
  const labels: Record<string, string> = {
    "quickpls:run-analysis": "run analysis",
    "quickpls:save-project": "save project",
    "quickpls:open-project": "open project",
    "quickpls:open-demo-project": "open demo project",
    "quickpls:import-data": "import data",
  };
  return labels[event] ?? event;
}

function runAction(
  action: WorkflowCoachAction,
  messageId: string,
  currentView: WorkspaceView,
  setView: (view: WorkspaceView, context?: { from: WorkspaceView; to: WorkspaceView; actionLabel: string; coachId: string }) => void,
  setWorkflowCommandContext: (context: { from: WorkspaceView; event: string; actionLabel: string; coachId: string } | null) => void,
) {
  if (action.disabled) return;
  if (action.view) setView(action.view, { from: currentView, to: action.view, actionLabel: action.label, coachId: messageId });
  if (action.event) {
    setWorkflowCommandContext({ from: currentView, event: action.event, actionLabel: action.label, coachId: messageId });
    window.setTimeout(() => dispatchWorkspaceCommand(action.event!), 0);
  }
}

function actionIdentity(action: WorkflowCoachAction) {
  return `${action.label}|${action.view ?? ""}|${action.event ?? ""}`;
}

function workspaceLabel(view: WorkspaceView) {
  const labels: Record<WorkspaceView, string> = {
    welcome: "Home",
    data: "Data",
    models: "Model",
    analyses: "Setup",
    run: "Run",
    runs: "Results",
    groups: "Results groups",
    reports: "Report",
    trust: "Trust Center",
    settings: "Settings",
  };
  return labels[view];
}

export function WorkspaceCoach() {
  const view = useWorkspace((state) => state.view);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const settings = useWorkspace((state) => state.analysisSettings);
  const setView = useWorkspace((state) => state.setView);
  const setWorkflowCommandContext = useWorkspace((state) => state.setWorkflowCommandContext);
  const clearWorkflowFeedback = useWorkspace((state) => state.clearWorkflowFeedback);
  const destinationContext = useWorkspace((state) => state.workflowDestinationContext);
  const commandContext = useWorkspace((state) => state.workflowCommandContext);
  const message = workflowCoachMessage({ view, dataset, nodes, edges, runs, settings, nativeDesktop: isNativeDesktop() });
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const Icon = message.status === "ready" ? CheckCircle2 : message.status === "blocked" ? TriangleAlert : Info;
  const secondary = message.secondary && actionIdentity(message.secondary) !== actionIdentity(message.primary) ? message.secondary : undefined;
  const primaryReasonId = `${message.id}-primary-reason`;
  const secondaryReasonId = `${message.id}-secondary-reason`;

  return <aside className={`workspace-coach ${message.status}`} aria-label="Workflow coach" data-coach-id={message.id}>
    <div className="workspace-coach-main">
      <Icon size={18} />
      <div>
        <strong>{message.title}</strong>
        <span>{message.detail}</span>
      </div>
    </div>
    <div className="workspace-coach-actions">
      <span className="workspace-coach-action-block">
        <button
          type="button"
          className="qpls2-primary-action compact"
          disabled={message.primary.disabled}
          title={message.primary.disabled ? message.primary.reason : message.primary.label}
          aria-describedby={message.primary.disabled && message.primary.reason ? primaryReasonId : undefined}
          data-action-label={message.primary.label}
          data-action-disabled={message.primary.disabled ? "true" : "false"}
          data-action-view={message.primary.view ?? ""}
          data-action-event={message.primary.event ?? ""}
          onClick={() => runAction(message.primary, message.id, view, setView, setWorkflowCommandContext)}
        >
          {message.primary.label}<ArrowRight size={14} />
        </button>
        {message.primary.disabled && message.primary.reason ? <small id={primaryReasonId} className="workspace-coach-action-reason">{message.primary.reason}</small> : null}
      </span>
      {secondary ? <span className="workspace-coach-action-block">
        <button
          type="button"
          className="qpls2-secondary-action compact"
          disabled={secondary.disabled}
          title={secondary.disabled ? secondary.reason : secondary.label}
          aria-describedby={secondary.disabled && secondary.reason ? secondaryReasonId : undefined}
          data-action-label={secondary.label}
          data-action-disabled={secondary.disabled ? "true" : "false"}
          data-action-view={secondary.view ?? ""}
          data-action-event={secondary.event ?? ""}
          onClick={() => runAction(secondary, message.id, view, setView, setWorkflowCommandContext)}
        >
          {secondary.label}
        </button>
        {secondary.disabled && secondary.reason ? <small id={secondaryReasonId} className="workspace-coach-action-reason">{secondary.reason}</small> : null}
      </span> : null}
    </div>
    {destinationContext?.to === view ? <div className="workspace-coach-feedback-line">
      <p
        className="workspace-coach-destination"
        data-destination-from={destinationContext.from}
        data-destination-to={destinationContext.to}
        data-destination-action={destinationContext.actionLabel}
        data-destination-coach={destinationContext.coachId}
      >
        Opened {workspaceLabel(destinationContext.to)} from {destinationContext.actionLabel}.
      </p>
      <button type="button" className="workspace-coach-feedback-dismiss" aria-label="Dismiss workflow feedback" onClick={clearWorkflowFeedback}>Dismiss</button>
    </div> : null}
    {commandContext && (commandContext.from === view || destinationContext?.coachId === commandContext.coachId) ? <div className="workspace-coach-feedback-line">
      <p
        className="workspace-coach-command"
        data-command-from={commandContext.from}
        data-command-event={commandContext.event}
        data-command-action={commandContext.actionLabel}
        data-command-coach={commandContext.coachId}
      >
        Requested {commandLabel(commandContext.event)} from {commandContext.actionLabel}.
      </p>
      <button type="button" className="workspace-coach-feedback-dismiss" aria-label="Dismiss workflow feedback" onClick={clearWorkflowFeedback}>Dismiss</button>
    </div> : null}
    {!readiness.canRun && (view === "analyses" || view === "run") ? <p className="workspace-coach-reason">Current blocker: {readiness.blockers[0]?.detail ?? readiness.summary}</p> : null}
  </aside>;
}
