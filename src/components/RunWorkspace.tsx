import { AlertTriangle, CheckCircle2, Play } from "lucide-react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import { ReadinessPanel } from "./ReadinessPanel";

export function RunWorkspace() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const method = methods.find((candidate) => candidate.id === settings.method);

  return <section className="workspace-page run-workspace">
    <div className="page-heading"><div><h1>Run analysis</h1><p>Review readiness, launch the selected method, then inspect the saved result.</p></div></div>
    <ReadinessPanel readiness={readiness} onNavigate={setView} />
    <div className="run-launch-card">
      <div>
        <strong>{method?.name ?? settings.method}</strong>
        <span>{method?.status === "validated" ? "Validated for the documented v1.0 scope" : "Runs with explicit method-status warnings where available"}</span>
      </div>
      <div className="run-action-stack">
        <button className="run-button large" disabled={!readiness.canRun} title={readiness.canRun ? `Run ${method?.name ?? settings.method}` : readiness.blockers[0]?.detail ?? readiness.summary} onClick={() => window.dispatchEvent(new CustomEvent("quickpls:run-analysis"))}>
          <Play size={17} fill="currentColor" />
          <span>Run selected method</span>
        </button>
        {!readiness.canRun ? <p className="disabled-reason">{readiness.blockers[0]?.detail ?? readiness.summary}</p> : null}
      </div>
    </div>
    <div className="run-guidance-grid">
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
        <div><strong>Next step</strong><p>{runs.length ? "Open Results to compare or select diagram overlays." : "Your first completed run will appear in Results."}</p><button className="secondary-button" onClick={() => setView("runs")}>Open results</button></div>
      </article>
    </div>
  </section>;
}
