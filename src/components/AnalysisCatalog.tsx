import { CheckCircle2, Clock3, LockKeyhole } from "lucide-react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { effectiveMethodStatus, effectiveMethodStatusLabel, isSelectableAnalysisMethod, methodStatusDescription, methodStatusLabel } from "../domain/methodStatus";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { AnalysisMethodId, MethodDefinition } from "../types";
import { ReadinessPanel } from "./ReadinessPanel";

const runnableMethods = methods.filter(isSelectableAnalysisMethod);

function MethodStatusPill({ method }: { method: MethodDefinition }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const selectable = isSelectableAnalysisMethod(method);
  const effectiveStatus = selectable ? effectiveMethodStatus(method, settings) : "unsupported";
  return <span className={`status-text ${effectiveStatus}`} title={methodStatusDescription(method, settings)}>
    {effectiveStatus === "validated" ? <CheckCircle2 size={15} /> : effectiveStatus === "experimental" ? <Clock3 size={15} /> : <LockKeyhole size={15} />}
    {selectable ? methodStatusLabel(effectiveStatus) : "Configured elsewhere"}
  </span>;
}

export function AnalysisCatalog() {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  const columns = useWorkspace((state) => state.dataset.columns);
  const dataset = useWorkspace((state) => state.dataset);
  const edges = useWorkspace((state) => state.edges);
  const nodes = useWorkspace((state) => state.nodes);
  const setView = useWorkspace((state) => state.setView);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  return <section className="workspace-page">
    <div className="page-heading"><div><h1>Validate and run</h1><p>Check readiness, select a validated or watermarked method, then run the offline engine.</p></div></div>
    <ReadinessPanel readiness={readiness} onNavigate={setView} />
    <div className="analysis-settings">
      <div><strong>Analysis setup</strong><span className={readiness.canRun ? "status-text validated" : "status-text experimental"}>{readiness.canRun ? <CheckCircle2 size={14} /> : <Clock3 size={14} />}{readiness.canRun ? "ready" : "needs attention"}</span></div>
      <label>Run method<select value={settings.method} onChange={(event) => setSettings({ method: event.target.value as AnalysisMethodId })}>
        {runnableMethods.map((method) => <option key={method.id} value={method.id}>{method.name} | {method.id === "regression" ? "OLS/logistic/bounded PROCESS validated" : method.id === "mga" ? "MICOM/permutation MGA validated" : methodStatusLabel(method.status)}</option>)}
      </select></label>
      <p className="settings-guidance">Recommended defaults are applied automatically. Open advanced settings only when you need resampling, worker, or reproducibility controls.</p>
      {settings.method === "wpls" && <label>Case weight column<select value={settings.caseWeightColumn ?? ""} onChange={(event) => setSettings({ caseWeightColumn: event.target.value || null })}>
        <option value="">Select column</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "mga" && <label>Group column<select value={settings.groupColumn ?? ""} onChange={(event) => setSettings({ groupColumn: event.target.value || null })}>
        <option value="">Select two-group column</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "mga" && <label>Group workflows<select value={settings.groupMethods ?? "micom,mga_permutation"} onChange={(event) => setSettings({ groupMethods: event.target.value })}>
        <option value="micom,mga_permutation">MICOM + permutation MGA</option>
        <option value="micom">MICOM only</option>
        <option value="mga_permutation">Permutation MGA only</option>
      </select></label>}
      {settings.method === "mga" && <label>Group permutation samples<input type="number" min={1} max={10000} step={100} value={settings.groupPermutationSamples ?? 999} onChange={(event) => setSettings({ groupPermutationSamples: Math.min(10000, Math.max(1, Math.trunc(Number(event.target.value) || 999))) })} /></label>}
      {settings.method === "predict" && <label>Segmentation workflow<select value={settings.groupMethods?.includes("fimix") ? "fimix" : "pls_pos"} onChange={(event) => setSettings({ groupMethods: event.target.value })}>
        <option value="pls_pos">PLS-POS</option>
        <option value="fimix">FIMIX-PLS</option>
      </select></label>}
      {settings.method === "predict" && <label>Segment count<input type="number" min={2} max={5} step={1} value={settings.segmentCount ?? 2} onChange={(event) => setSettings({ segmentCount: Math.min(5, Math.max(2, Math.trunc(Number(event.target.value) || 2))) })} /></label>}
      {settings.method === "predict" && <label>Segment starts<input type="number" min={1} max={50} step={1} value={settings.segmentStarts ?? 10} onChange={(event) => setSettings({ segmentStarts: Math.min(50, Math.max(1, Math.trunc(Number(event.target.value) || 10))) })} /></label>}
      {settings.method === "predict" && <label>Minimum segment share<input type="number" min={0.05} max={0.4} step={0.01} value={settings.minimumSegmentShare ?? 0.10} onChange={(event) => setSettings({ minimumSegmentShare: Math.min(0.4, Math.max(0.05, Number(event.target.value) || 0.10)) })} /></label>}
      {settings.method === "ipma" && <label>IPMA target<select value={settings.ipmaTargets ?? ""} onChange={(event) => setSettings({ ipmaTargets: event.target.value || null })}>
        <option value="">All endogenous constructs</option>
        {nodes.map((node) => <option key={node.id} value={node.id}>{node.data.label}</option>)}
      </select></label>}
      {settings.method === "cbsem" && <label>CB-SEM model<select value={settings.cbsemModelType ?? "sem"} onChange={(event) => setSettings({ cbsemModelType: event.target.value as "cfa" | "sem" })}>
        <option value="sem">SEM with structural paths</option>
        <option value="cfa">CFA measurement model</option>
      </select></label>}
      {settings.method === "cbsem" && <label>Standardized solution<select value={settings.cbsemStandardization ?? "std_all"} onChange={(event) => setSettings({ cbsemStandardization: event.target.value as "std_lv" | "std_all" })}>
        <option value="std_all">std_all</option>
        <option value="std_lv">std_lv</option>
      </select></label>}
      {settings.method === "cbsem" && <label>Mean structure<select value={settings.cbsemMeanStructure ? "true" : "false"} onChange={(event) => setSettings({ cbsemMeanStructure: event.target.value === "true" })}>
        <option value="false">Disabled</option>
        <option value="true">Enabled</option>
      </select></label>}
      {settings.method === "cbsem" && <label>CB-SEM group column<select value={settings.cbsemGroupColumn ?? ""} onChange={(event) => setSettings({ cbsemGroupColumn: event.target.value || null })}>
        <option value="">No multigroup analysis</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "cbsem" && <label>Invariance steps<select value={settings.cbsemInvarianceSteps ?? "configural,metric,scalar"} onChange={(event) => setSettings({ cbsemInvarianceSteps: event.target.value })}>
        <option value="configural,metric,scalar">Configural + metric + scalar</option>
        <option value="configural,metric">Configural + metric</option>
        <option value="configural">Configural only</option>
      </select></label>}
      {settings.method === "cbsem" && <label>CB-SEM bootstrap samples<input type="number" min={0} max={10000} step={100} value={settings.cbsemBootstrapSamples ?? 0} onChange={(event) => setSettings({ cbsemBootstrapSamples: Math.min(10000, Math.max(0, Math.trunc(Number(event.target.value) || 0))) })} /></label>}
      {settings.method === "pca" && <label>PCA variables<input value={settings.pcaVariables ?? columns.join(",")} onChange={(event) => setSettings({ pcaVariables: event.target.value })} /></label>}
      {settings.method === "pca" && <label>Component rule<select value={settings.pcaComponentRule ?? "kaiser"} onChange={(event) => setSettings({ pcaComponentRule: event.target.value as "kaiser" | "fixed" | "variance_threshold" })}>
        <option value="kaiser">Kaiser</option>
        <option value="fixed">Fixed count</option>
        <option value="variance_threshold">Variance threshold</option>
      </select></label>}
      {settings.method === "pca" && <label>Fixed components<input type="number" min={1} max={50} step={1} value={settings.pcaComponents ?? 2} onChange={(event) => setSettings({ pcaComponents: Math.min(50, Math.max(1, Math.trunc(Number(event.target.value) || 2))) })} /></label>}
      {settings.method === "regression" && <label>Regression type<select value={settings.regressionType ?? "ols"} onChange={(event) => setSettings({ regressionType: event.target.value as "ols" | "logistic" | "process" })}>
        <option value="ols">OLS</option>
        <option value="logistic">Logistic</option>
        <option value="process">PROCESS-style</option>
      </select><span className={`setting-status ${effectiveMethodStatus(methods.find((method) => method.id === "regression"), settings)}`}>{effectiveMethodStatusLabel(methods.find((method) => method.id === "regression"), settings)} scope</span></label>}
      {settings.method === "regression" && <label>Outcome<select value={settings.regressionOutcome ?? ""} onChange={(event) => setSettings({ regressionOutcome: event.target.value || null })}>
        <option value="">Select outcome</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "regression" && <label>Predictors<input value={settings.regressionPredictors ?? ""} onChange={(event) => setSettings({ regressionPredictors: event.target.value })} placeholder="COMP1,LIKE1" /></label>}
      {settings.method === "regression" && <label>Controls<input value={settings.regressionControls ?? ""} onChange={(event) => setSettings({ regressionControls: event.target.value })} placeholder="Optional comma-separated controls" /></label>}
      {settings.method === "regression" && <label>Robust SE<select value={settings.robustSe ?? "hc3"} onChange={(event) => setSettings({ robustSe: event.target.value as "none" | "hc0" | "hc3" | "hc4" })}>
        <option value="hc3">HC3</option>
        <option value="hc0">HC0</option>
        <option value="hc4">HC4</option>
        <option value="none">Classical</option>
      </select></label>}
      {settings.method === "regression" && settings.regressionType === "process" && <label>PROCESS model<select value={settings.processModel ?? "mediation"} onChange={(event) => setSettings({ processModel: event.target.value as "mediation" | "moderation" | "moderated_mediation" })}>
        <option value="mediation">Mediation</option>
        <option value="moderation">Moderation</option>
        <option value="moderated_mediation">Moderated mediation</option>
      </select></label>}
      {settings.method === "regression" && settings.regressionType === "process" && <label>PROCESS X<select value={settings.processX ?? ""} onChange={(event) => setSettings({ processX: event.target.value || null })}>
        <option value="">Select X</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "regression" && settings.regressionType === "process" && <label>PROCESS mediator<select value={settings.processM ?? ""} onChange={(event) => setSettings({ processM: event.target.value || null })}>
        <option value="">Select mediator</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "regression" && settings.regressionType === "process" && <label>PROCESS moderator<select value={settings.processW ?? ""} onChange={(event) => setSettings({ processW: event.target.value || null })}>
        <option value="">Select moderator</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "nca" && <label>NCA X<select value={settings.ncaX ?? ""} onChange={(event) => setSettings({ ncaX: event.target.value || null })}>
        <option value="">Select X</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "nca" && <label>NCA Y<select value={settings.ncaY ?? ""} onChange={(event) => setSettings({ ncaY: event.target.value || null })}>
        <option value="">Select Y</option>
        {columns.map((column) => <option key={column} value={column}>{column}</option>)}
      </select></label>}
      {settings.method === "nca" && <label>Ceiling<select value={settings.ncaCeiling ?? "both"} onChange={(event) => setSettings({ ncaCeiling: event.target.value as "ce_fdh" | "cr_fdh" | "both" })}>
        <option value="both">CE-FDH + CR-FDH</option>
        <option value="ce_fdh">CE-FDH</option>
        <option value="cr_fdh">CR-FDH</option>
      </select></label>}
      {settings.method === "nca" && <label>NCA permutations<input type="number" min={1} max={10000} step={100} value={settings.ncaPermutationSamples ?? 999} onChange={(event) => setSettings({ ncaPermutationSamples: Math.min(10000, Math.max(1, Math.trunc(Number(event.target.value) || 999))) })} /></label>}
      <details className="settings-section advanced-settings">
        <summary>Advanced resampling and reproducibility</summary>
        <label>Bootstrap replicates<input type="number" min={0} max={10000} step={100} value={settings.bootstrapSamples} onChange={(event) => { const value = Math.min(10000, Math.max(0, Math.trunc(Number(event.target.value) || 0))); setSettings(value === 0 ? { bootstrapSamples: 0, studentizedInnerSamples: 0 } : { bootstrapSamples: value }); }} /></label>
        <label>Studentized inner replicates<input type="number" min={0} max={999} step={2} value={settings.studentizedInnerSamples} onChange={(event) => { const value = Math.trunc(Number(event.target.value) || 0); setSettings({ studentizedInnerSamples: value }); }} /></label>
        <label>Permutation samples<input type="number" min={0} max={10000} step={100} value={settings.permutationSamples} onChange={(event) => { const value = Math.trunc(Number(event.target.value) || 0); setSettings({ permutationSamples: value === 0 ? 0 : Math.min(10000, Math.max(99, value)) }); }} /></label>
        <label>Random seed<input type="number" min={0} max={4294967295} step={1} value={settings.seed} onChange={(event) => setSettings({ seed: Math.min(4294967295, Math.max(0, Math.trunc(Number(event.target.value) || 0))) })} /></label>
        <label>Workers<input type="number" min={1} max={64} step={1} value={settings.workers} onChange={(event) => setSettings({ workers: Math.min(64, Math.max(1, Math.trunc(Number(event.target.value) || 1))) })} /></label>
        <label>Confidence level<input type="number" min={0.8} max={0.999} step={0.01} value={settings.confidenceLevel} onChange={(event) => setSettings({ confidenceLevel: Math.min(0.999, Math.max(0.8, Number(event.target.value) || 0.95)) })} /></label>
      </details>
    </div>
    <div className="method-table"><div className="method-table-head"><span>Method</span><span>Family</span><span>Status</span></div>{methods.map((method) => {
      const selectable = isSelectableAnalysisMethod(method);
      return <button type="button" className={`method-row ${settings.method === method.id ? "selected" : ""}`} key={method.id} disabled={!selectable} title={methodStatusDescription(method, settings)} onClick={() => { if (selectable) setSettings({ method: method.id }); }}>
        <strong>{method.name}</strong><span>{method.family}</span><MethodStatusPill method={method} />
      </button>;
    })}</div>
  </section>;
}
