import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  ClipboardCheck,
  Clock3,
  Database,
  FileText,
  Fingerprint,
  PauseCircle,
  Play,
  RotateCcw,
  Settings2,
  ShieldCheck,
  Square,
  TimerReset,
} from "lucide-react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { effectiveMethodStatus } from "../domain/methodStatus";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { RunMonitorStatus } from "../types";
import { PageHeader, Panel, StatusBadge, WorkspacePage } from "./Ui";

function plural(value: number, singular: string, pluralLabel = `${singular}s`) {
  return `${value} ${value === 1 ? singular : pluralLabel}`;
}

function shortHash(value: string | null | undefined) {
  return value ? value.slice(0, 12) : "Not available";
}

function monitorPercent(status: RunMonitorStatus, completed: number, total: number, ready: boolean) {
  if (status === "completed") return 100;
  if (status === "failed" || status === "cancelled") return total > 0 ? Math.max(0, Math.min(100, Math.round((completed / total) * 100))) : 0;
  if (total > 0) return Math.max(0, Math.min(99, Math.round((completed / total) * 100)));
  return ready ? 0 : 0;
}

function stepState(step: string, monitor: RunMonitorStatus, readinessCanRun: boolean) {
  if (!readinessCanRun) return step === "Validate data" ? "blocked" : "pending";
  if (monitor === "failed") return step === "Validate data" || step === "Run engine" ? "failed" : "pending";
  if (monitor === "cancelled") return step === "Run engine" ? "cancelled" : "done";
  if (monitor === "completed") return "done";
  if (monitor === "queued") return step === "Validate data" ? "active" : "pending";
  if (monitor === "validating") return step === "Validate data" ? "active" : "pending";
  if (monitor === "running" || monitor === "cancelling") {
    if (step === "Validate data" || step === "Prepare recipe") return "done";
    if (step === "Run engine") return "active";
    return "pending";
  }
  return readinessCanRun ? (step === "Validate data" ? "ready" : "pending") : "pending";
}

function StepIcon({ state }: { state: string }) {
  if (state === "done" || state === "ready") return <CheckCircle2 size={16} />;
  if (state === "blocked" || state === "failed") return <AlertTriangle size={16} />;
  if (state === "active") return <Clock3 size={16} />;
  if (state === "cancelled") return <PauseCircle size={16} />;
  return <span className="run-v227-step-dot" aria-hidden="true" />;
}

export function RunWorkspace() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const runs = useWorkspace((state) => state.runs);
  const runMonitor = useWorkspace((state) => state.runMonitor);
  const setRunMonitor = useWorkspace((state) => state.setRunMonitor);
  const setView = useWorkspace((state) => state.setView);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const method = methods.find((candidate) => candidate.id === settings.method);
  const methodStatus = effectiveMethodStatus(method, settings);
  const blocker = readiness.blockers[0];
  const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance");
  const endogenousConstructs = new Set(structuralEdges.map((edge) => edge.target)).size;
  const indicatorCount = nodes.reduce((sum, node) => sum + node.data.indicators.length, 0);
  const multiIndicatorConstructs = nodes.filter((node) => node.data.indicators.length > 1).length;
  const bootstrapSamples = settings.bootstrapSamples ?? 0;
  const permutationSamples = settings.permutationSamples ?? 0;
  const activeStatuses: RunMonitorStatus[] = ["queued", "validating", "running", "cancelling"];
  const isActive = activeStatuses.includes(runMonitor.status);
  const progress = monitorPercent(runMonitor.status, runMonitor.completedUnits, runMonitor.totalUnits, readiness.canRun);
  const latestRun = runs[0] ?? null;
  const procedureSteps = [
    { label: "Validate data", detail: dataset.fingerprint ? "Dataset fingerprint is available." : "Import the dataset into the desktop project." },
    { label: "Prepare recipe", detail: `${plural(nodes.length, "construct")} and ${plural(structuralEdges.length, "structural path")} will be serialized.` },
    { label: "Run engine", detail: "Execute the selected QuickPLS estimator offline." },
    { label: "Commit result", detail: "Save the run with provenance, warnings, and immutable recipe." },
    { label: "Open results", detail: "Inspect tables, interpretation, diagram overlays, and report outputs." },
  ];
  const outputsProduced = [
    structuralEdges.length ? "Path coefficients and total effects" : "No structural paths to estimate",
    indicatorCount ? "Loadings or weights for assigned indicators" : "Measurement output unavailable until indicators are assigned",
    endogenousConstructs ? "R² and adjusted R² for endogenous constructs" : "R² unavailable until a construct has incoming paths",
    multiIndicatorConstructs ? "Reliability and validity diagnostics" : "Reliability diagnostics require multi-indicator constructs",
    bootstrapSamples > 0 ? "Bootstrap confidence intervals and p values" : "Bootstrap inference unavailable until enabled",
    permutationSamples > 0 ? "Permutation output" : "Permutation output unavailable until enabled",
  ];
  const unavailableOutputs = outputsProduced.filter((item) => item.toLowerCase().includes("unavailable") || item.startsWith("No "));
  const activeOutputs = outputsProduced.filter((item) => !unavailableOutputs.includes(item));
  const recipeFingerprint = `${nodes.length}C-${indicatorCount}I-${structuralEdges.length}P-${settings.seed}`;
  const logs = runMonitor.logs.length
    ? runMonitor.logs
    : [{
      id: "ready-fallback",
      timestamp: new Date().toISOString(),
      phase: readiness.canRun ? "Ready" : "Blocked",
      message: readiness.canRun ? "Ready to validate and launch the selected method." : blocker?.detail ?? readiness.summary,
      tone: readiness.canRun ? "success" : "warning",
    }];

  const triggerRunOrCancel = () => {
    if (isActive) {
      window.dispatchEvent(new CustomEvent("quickpls:cancel-analysis"));
      return;
    }
    if (readiness.canRun) {
      window.dispatchEvent(new CustomEvent("quickpls:run-analysis"));
      return;
    }
    setRunMonitor({
      status: "blocked",
      phase: "Blocked",
      message: blocker?.detail ?? readiness.summary,
      completedUnits: 0,
      totalUnits: 0,
      startedAt: null,
      completedAt: null,
      activeJobId: null,
      error: blocker?.label ?? readiness.summary,
    }, { phase: "Blocked", message: blocker?.detail ?? readiness.summary, tone: "warning" });
    if (blocker?.actionView) setView(blocker.actionView);
  };

  return <WorkspacePage
    data-v218-mockup-screen="run"
    data-v227-run-monitor="true"
    className="run-workspace run-v2-workspace run-v212-workspace run-v218-workspace run-v227-workspace"
  >
    <PageHeader
      kicker="Calculation"
      title="Run analysis"
      description="Launch the selected method, monitor the native job, review logs, and hand off the completed run to Results or Report."
      actions={<StatusBadge status={readiness.canRun ? "validated" : "warning"}>{readiness.canRun ? "Ready to calculate" : readiness.summary}</StatusBadge>}
    />

    <div className="run-v227-monitor-grid">
      <Panel title="Procedure" description="Execution checklist" className="run-v227-procedure-panel">
        <ol className="run-v227-steps">
          {procedureSteps.map((step, index) => {
            const state = stepState(step.label, runMonitor.status, readiness.canRun);
            return <li key={step.label} className={`run-v227-step ${state}`}>
              <StepIcon state={state} />
              <div>
                <strong>{index + 1}. {step.label}</strong>
                <p>{step.detail}</p>
              </div>
            </li>;
          })}
        </ol>
      </Panel>

      <Panel
        title={runMonitor.phase || "Calculation monitor"}
        description={runMonitor.message}
        className={`run-v227-progress-panel ${runMonitor.status}`}
        actions={<StatusBadge status={runMonitor.status === "completed" ? "validated" : runMonitor.status === "failed" || runMonitor.status === "blocked" ? "warning" : "experimental"}>{runMonitor.status}</StatusBadge>}
      >
        <div className="run-v227-progress-readout">
          <div>
            <strong>{progress}%</strong>
            <span>{runMonitor.totalUnits ? `${runMonitor.completedUnits}/${runMonitor.totalUnits} units` : readiness.canRun ? "Ready to start" : "Waiting for setup"}</span>
          </div>
          <div className="run-v227-progress-track" aria-label={`Calculation progress ${progress}%`}>
            <span style={{ width: `${progress}%` }} />
          </div>
        </div>
        {runMonitor.error ? <div className="run-v227-error"><AlertTriangle size={16} />{runMonitor.error}</div> : null}
        <div className="run-v227-log" aria-label="Calculation log">
          {logs.map((entry) => <article key={entry.id} className={entry.tone}>
            <time>{new Date(entry.timestamp).toLocaleTimeString()}</time>
            <strong>{entry.phase}</strong>
            <p>{entry.message}</p>
          </article>)}
        </div>
      </Panel>

      <Panel title="Run settings" description="Immutable summary" className="run-v227-settings-panel">
        <dl className="run-v227-setting-grid">
          <div><dt>Method</dt><dd>{method?.name ?? settings.method}</dd></div>
          <div><dt>Scope</dt><dd>{methodStatus === "validated" ? "Validated documented scope" : methodStatus === "experimental" ? "Experimental / watermarked" : "Unsupported"}</dd></div>
          <div><dt>Seed</dt><dd>{settings.seed}</dd></div>
          <div><dt>Workers</dt><dd>{plural(settings.workers, "worker")}</dd></div>
          <div><dt>Data fingerprint</dt><dd><Fingerprint size={13} />{shortHash(dataset.fingerprint)}</dd></div>
          <div><dt>Recipe fingerprint</dt><dd><ClipboardCheck size={13} />{recipeFingerprint}</dd></div>
          <div><dt>Bootstrap</dt><dd>{bootstrapSamples > 0 ? `${bootstrapSamples} samples` : "Off"}</dd></div>
          <div><dt>Permutation</dt><dd>{permutationSamples > 0 ? `${permutationSamples} samples` : "Off"}</dd></div>
        </dl>
        <div className="run-v227-output-list">
          <strong>Outputs produced</strong>
          <ul>{activeOutputs.map((item) => <li key={item}><CheckCircle2 size={13} />{item}</li>)}</ul>
        </div>
        {unavailableOutputs.length ? <div className="run-v227-output-list unavailable">
          <strong>Unavailable in this run</strong>
          <ul>{unavailableOutputs.map((item) => <li key={item}><AlertTriangle size={13} />{item}</li>)}</ul>
        </div> : null}
      </Panel>
    </div>

    <section className="run-v227-footer" aria-label="Run monitor actions">
      <div className="run-v227-footer-context">
        <Database size={17} />
        <div>
          <strong>{dataset.name}</strong>
          <span>{plural(dataset.rowCount ?? dataset.rows.length, "row")} | {plural(nodes.length, "construct")} | {plural(structuralEdges.length, "path")}</span>
        </div>
      </div>
      <div className="run-v227-footer-actions">
        <button
          type="button"
          className={isActive ? "secondary-button danger" : "qpls2-primary-action"}
          onClick={triggerRunOrCancel}
          disabled={runMonitor.status === "cancelling"}
          aria-label={isActive ? "Cancel active calculation" : readiness.canRun ? "Run selected method" : "Open the next required setup step"}
        >
          {isActive ? <Square size={15} fill="currentColor" /> : <Play size={16} fill="currentColor" />}
          {isActive ? runMonitor.status === "cancelling" ? "Cancelling..." : "Cancel run" : readiness.canRun ? "Run selected method" : blocker?.actionLabel ?? "Resolve blocker"}
        </button>
        <button className="secondary-button" onClick={() => setView("analyses")}><Settings2 size={15} />Setup</button>
        <button className="secondary-button" disabled={!runs.length} title={runs.length ? "Open completed results" : "Results unlock after a completed run"} onClick={() => setView("runs")}><ArrowRight size={15} />Open results</button>
        <button className="secondary-button" disabled={!runs.length} title={runs.length ? "Prepare report" : "Run a method before preparing report exports"} onClick={() => setView("reports")}><FileText size={15} />Prepare report</button>
        <button className="secondary-button" onClick={() => setRunMonitor({ status: readiness.canRun ? "idle" : "blocked", phase: readiness.canRun ? "Idle" : "Blocked", message: readiness.canRun ? "No calculation is currently running." : blocker?.detail ?? readiness.summary, completedUnits: 0, totalUnits: 0, error: null }, { phase: "Monitor reset", message: "Run monitor display was reset.", tone: "info" })}><RotateCcw size={15} />Reset monitor</button>
      </div>
      {!readiness.canRun && !isActive ? <p className="run-v227-disabled-reason">Run disabled: {blocker?.detail ?? readiness.summary}</p> : null}
      {!runs.length ? <p className="run-v227-disabled-reason">Results and report handoff unlock after a completed run.</p> : null}
      {latestRun ? <p className="run-v227-last-run">Last saved run: {latestRun.name} | {new Date(latestRun.createdAt).toLocaleString()}</p> : null}
    </section>
  </WorkspacePage>;
}
