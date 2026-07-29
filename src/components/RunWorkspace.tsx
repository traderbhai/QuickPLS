import { AlertTriangle, CheckCircle2, ClipboardCheck, Play } from "lucide-react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { effectiveMethodStatus } from "../domain/methodStatus";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";

export function RunWorkspace() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const method = methods.find((candidate) => candidate.id === settings.method);
  const methodStatus = effectiveMethodStatus(method, settings);

  return <section className="workspace-page run-workspace">
    <div className="page-heading"><div><h1>Run analysis</h1><p>Launch the configured method, monitor execution, then inspect the saved result.</p></div></div>
    <section className="run-monitor-summary" aria-label="Run readiness summary">
      <div>
        <strong>{readiness.canRun ? "Ready to run" : readiness.summary}</strong>
        <span>{readiness.canRun ? `${method?.name ?? settings.method} is configured for the current data and model.` : readiness.blockers[0]?.detail ?? readiness.summary}</span>
      </div>
      <div className="run-monitor-checks">
        {readiness.items.map((item) => <span key={item.id} className={`status-text ${item.status === "ready" ? "validated" : "experimental"}`} title={item.detail}>{item.status === "ready" ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}{item.label}</span>)}
      </div>
    </section>
    <div className="run-launch-card">
      <div>
        <strong>{method?.name ?? settings.method}</strong>
        <span>{methodStatus === "validated" ? "Validated for the documented supported scope" : "Runs with explicit method-status warnings where available"}</span>
        <small>Seed {settings.seed} | {settings.workers} worker{settings.workers === 1 ? "" : "s"} | bootstrap {settings.bootstrapSamples > 0 ? settings.bootstrapSamples : "off"} | permutation {settings.permutationSamples > 0 ? settings.permutationSamples : "off"}</small>
      </div>
      <div className="run-action-stack">
        <button className="run-button large" disabled={!readiness.canRun} title={readiness.canRun ? `Run ${method?.name ?? settings.method}` : readiness.blockers[0]?.detail ?? readiness.summary} onClick={() => window.dispatchEvent(new CustomEvent("quickpls:run-analysis"))}>
          <Play size={17} fill="currentColor" />
          <span>Run selected method</span>
        </button>
        {!readiness.canRun ? <p className="disabled-reason">Run disabled: {readiness.blockers[0]?.detail ?? readiness.summary}</p> : null}
      </div>
    </div>
    <div className="run-guidance-grid">
      <article>
        <ClipboardCheck size={18} />
        <div><strong>Need to change settings?</strong><p>Method, bootstrap, group, prediction, and reproducibility settings are controlled in Setup.</p><button className="secondary-button" onClick={() => setView("analyses")}>Open setup</button></div>
      </article>
      <article>
        <CheckCircle2 size={18} />
        <div><strong>After completion</strong><p>QuickPLS saves the run with recipe, seed, data fingerprint, warnings, and estimates.</p></div>
      </article>
      <article>
        <AlertTriangle size={18} />
        <div><strong>Before publication</strong><p>Review warnings, scope status, and export watermarking in Results and Report.</p></div>
      </article>
      <article>
        <Play size={18} />
        <div><strong>Next step</strong><p>{runs.length ? "Open Results to compare or select diagram overlays." : "Run a method before opening completed results."}</p><button className="secondary-button" disabled={!runs.length} title={runs.length ? "Open completed results" : "No completed run exists yet"} onClick={() => setView("runs")}>Open results</button>{!runs.length ? <small className="inline-disabled-reason">Results unlock after the first completed run.</small> : null}</div>
      </article>
    </div>
  </section>;
}
