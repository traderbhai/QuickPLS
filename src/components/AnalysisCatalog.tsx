import { CheckCircle2, Clock3, LockKeyhole, Play, SlidersHorizontal } from "lucide-react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { effectiveMethodStatus, isSelectableAnalysisMethod, methodStatusDescription, methodStatusLabel } from "../domain/methodStatus";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { AnalysisMethodId, MethodDefinition, MethodPresetId } from "../types";
import { ReadinessPanel } from "./ReadinessPanel";
import { ActionStrip, Card, PageHeader, StatusBadge, TabStrip } from "./Ui";

const runnableMethods = methods.filter(isSelectableAnalysisMethod);
const presets: Array<{ id: MethodPresetId; label: string; description: string }> = [
  { id: "standard_pls", label: "Standard PLS-SEM", description: "Core PLS path model with validated defaults." },
  { id: "pls_bootstrap", label: "PLS + Bootstrap", description: "Inference-ready PLS setup with bootstrap samples." },
  { id: "plspredict", label: "PLSpredict", description: "Prediction and segmentation workflow defaults." },
  { id: "micom_mga", label: "MICOM + MGA", description: "Two-group invariance and permutation MGA setup." },
  { id: "cbsem_cfa", label: "CB-SEM CFA", description: "Reflective raw-data CFA/SEM ML setup." },
  { id: "ols_regression", label: "OLS Regression", description: "Numeric OLS with HC3 robust standard errors." },
  { id: "nca", label: "NCA", description: "CE-FDH/CR-FDH necessity analysis." },
];

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
  const setup = useWorkspace((state) => state.methodSetupState);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  const setSetup = useWorkspace((state) => state.setMethodSetupState);
  const applyPreset = useWorkspace((state) => state.applyMethodPreset);
  const columns = useWorkspace((state) => state.dataset.columns);
  const dataset = useWorkspace((state) => state.dataset);
  const edges = useWorkspace((state) => state.edges);
  const nodes = useWorkspace((state) => state.nodes);
  const setView = useWorkspace((state) => state.setView);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const selectedMethod = methods.find((method) => method.id === settings.method) ?? methods[0] as MethodDefinition;
  const selectedStatus = effectiveMethodStatus(selectedMethod, settings);
  const basicFieldsReady = readiness.canRun && selectedStatus !== "unsupported";
  const methodCards = [
    { title: "Missing dataset", detail: dataset.columns.length ? `${dataset.columns.length} variables loaded` : "Import a dataset first", tone: dataset.columns.length ? "validated" : "warning" },
    { title: "Model", detail: nodes.every((node) => node.data.indicators.length > 0) ? `${nodes.length} constructs with indicators` : "Some constructs need indicators", tone: nodes.every((node) => node.data.indicators.length > 0) ? "validated" : "warning" },
    { title: "Unsupported shape", detail: selectedStatus === "unsupported" ? methodStatusDescription(selectedMethod, settings) : "No unsupported shape detected for the selected method settings", tone: selectedStatus === "unsupported" ? "warning" : "validated" },
    { title: "Experimental scope", detail: selectedStatus === "validated" ? "Validated for documented QuickPLS scope" : methodStatusDescription(selectedMethod, settings), tone: selectedStatus === "validated" ? "validated" : "warning" },
    { title: "Readiness", detail: readiness.summary, tone: readiness.canRun ? "validated" : "warning" },
    { title: "Run state", detail: readiness.summary, tone: readiness.canRun ? "validated" : "warning" },
  ] as const;

  const groupWorkflowActive = settings.method === "mga" || settings.method === "predict" || settings.method === "ipma";

  return <section className="workspace-page analysis-workspace-v14">
    <PageHeader title="Setup" description="Choose a method, validate model readiness, and keep advanced group or prediction settings available only when needed." actions={<StatusBadge status={selectedStatus === "validated" ? "validated" : selectedStatus === "experimental" ? "experimental" : "unsupported"}>{selectedStatus === "validated" ? "validated scope" : selectedStatus}</StatusBadge>} />
    <ReadinessPanel readiness={readiness} onNavigate={setView} />

    <ActionStrip>
      <TabStrip label="Method setup mode" value={setup.mode} onChange={(mode) => setSetup({ mode })} tabs={[{ id: "basic", label: "Basic" }, { id: "expert", label: "Expert" }]} />
      <button className="run-button" disabled={!basicFieldsReady} title={basicFieldsReady ? "Open run workspace" : readiness.blockers[0]?.detail ?? readiness.summary} onClick={() => setView("run")}><Play size={15} fill="currentColor" />Ready to run</button>
    </ActionStrip>

    <div className="setup-readiness-grid">
      {methodCards.map((card) => <Card key={card.title} title={card.title} description={card.detail} tone={card.tone} />)}
    </div>

    <div className="method-preset-grid">
      {presets.map((preset) => <button key={preset.id} className={setup.selectedPreset === preset.id ? "method-preset-card selected" : "method-preset-card"} onClick={() => applyPreset(preset.id)}>
        <strong>{preset.label}</strong><span>{preset.description}</span>
      </button>)}
    </div>

    <section className="group-setup-card" aria-label="Group and prediction workflow setup">
      <div>
        <strong>Group and prediction workflows</strong>
        <p>MICOM, permutation MGA, FIMIX-PLS, PLS-POS, and IPMA are configured here, then reviewed from the Groups tab in Results.</p>
      </div>
      <div className="group-setup-actions">
        <button className={setup.selectedPreset === "micom_mga" ? "secondary-button active" : "secondary-button"} onClick={() => applyPreset("micom_mga")}>MICOM + MGA setup</button>
        <button className={settings.method === "predict" ? "secondary-button active" : "secondary-button"} onClick={() => setSettings({ method: "predict", groupMethods: "pls_pos" })}>PLS-POS / FIMIX setup</button>
        <button className={settings.method === "ipma" ? "secondary-button active" : "secondary-button"} onClick={() => setSettings({ method: "ipma" })}>IPMA setup</button>
      </div>
      <small>{groupWorkflowActive ? "A group or prediction workflow is selected. Completed group outputs will appear in Results > Groups." : "Select a group workflow only when your research design needs invariance, group comparison, segmentation, or IPMA output."}</small>
    </section>

    <div className="analysis-settings guided-settings">
      <div><strong>Basic setup</strong><span className={readiness.canRun ? "status-text validated" : "status-text experimental"}>{readiness.canRun ? <CheckCircle2 size={14} /> : <Clock3 size={14} />}{readiness.canRun ? "ready" : "needs attention"}</span></div>
      <label>Run method<select value={settings.method} onChange={(event) => setSettings({ method: event.target.value as AnalysisMethodId })}>
        {runnableMethods.map((method) => <option key={method.id} value={method.id}>{method.name} | {method.id === "regression" ? "OLS/logistic/bounded PROCESS validated" : method.id === "mga" ? "MICOM/permutation MGA validated" : methodStatusLabel(method.status)}</option>)}
      </select></label>
      <label>Bootstrap<input type="checkbox" checked={settings.bootstrapSamples > 0} onChange={(event) => setSettings(event.target.checked ? { bootstrapSamples: 5000 } : { bootstrapSamples: 0, studentizedInnerSamples: 0 })} /></label>

      {settings.method === "wpls" && <SelectField label="Case weight column" value={settings.caseWeightColumn ?? ""} columns={columns} empty="Select column" onChange={(value) => setSettings({ caseWeightColumn: value || null })} />}
      {settings.method === "mga" && <SelectField label="Group column" value={settings.groupColumn ?? ""} columns={columns} empty="Select two-group column" onChange={(value) => setSettings({ groupColumn: value || null })} />}
      {settings.method === "ipma" && <SelectField label="IPMA target" value={settings.ipmaTargets ?? ""} columns={nodes.map((node) => node.id)} labels={new Map(nodes.map((node) => [node.id, node.data.label]))} empty="All endogenous constructs" onChange={(value) => setSettings({ ipmaTargets: value || null })} />}
      {settings.method === "regression" && <RegressionSettings columns={columns} />}
      {settings.method === "nca" && <NcaSettings columns={columns} />}
      {settings.method === "pca" && <PcaSettings columns={columns} />}
      {settings.method === "cbsem" && <CbsemSettings columns={columns} />}

      {setup.mode === "expert" && <details className="settings-section advanced-settings" open>
        <summary><SlidersHorizontal size={14} /> Expert resampling and reproducibility</summary>
        {settings.method === "mga" && <label>Group workflows<select value={settings.groupMethods ?? "micom,mga_permutation"} onChange={(event) => setSettings({ groupMethods: event.target.value })}><option value="micom,mga_permutation">MICOM + permutation MGA</option><option value="micom">MICOM only</option><option value="mga_permutation">Permutation MGA only</option></select></label>}
        {settings.method === "mga" && <NumberField label="Group permutation samples" value={settings.groupPermutationSamples ?? 999} min={1} max={10000} step={100} onChange={(value) => setSettings({ groupPermutationSamples: value })} />}
        {settings.method === "predict" && <PredictSettings />}
        <NumberField label="Bootstrap replicates" value={settings.bootstrapSamples} min={0} max={10000} step={100} onChange={(value) => setSettings(value === 0 ? { bootstrapSamples: 0, studentizedInnerSamples: 0 } : { bootstrapSamples: value })} />
        <NumberField label="Studentized inner replicates" value={settings.studentizedInnerSamples} min={0} max={999} step={2} onChange={(value) => setSettings({ studentizedInnerSamples: value })} />
        <NumberField label="Permutation samples" value={settings.permutationSamples} min={0} max={10000} step={100} onChange={(value) => setSettings({ permutationSamples: value === 0 ? 0 : Math.min(10000, Math.max(99, value)) })} />
        <NumberField label="Random seed" value={settings.seed} min={0} max={4294967295} step={1} onChange={(value) => setSettings({ seed: value })} />
        <NumberField label="Workers" value={settings.workers} min={1} max={64} step={1} onChange={(value) => setSettings({ workers: value })} />
        <label>Confidence level<input type="number" min={0.8} max={0.999} step={0.01} value={settings.confidenceLevel} onChange={(event) => setSettings({ confidenceLevel: Math.min(0.999, Math.max(0.8, Number(event.target.value) || 0.95)) })} /></label>
      </details>}
      {!readiness.canRun ? <p className="disabled-reason inline-disabled-reason">{readiness.blockers[0]?.detail ?? readiness.summary}</p> : null}
    </div>

    <section className="what-will-run-card" aria-label="What will run">
      <div>
        <strong>What will run</strong>
        <p>{selectedMethod.name} on {dataset.name} with {nodes.length} constructs, {edges.filter((edge) => edge.data?.role !== "covariance").length} structural paths, seed {settings.seed}, and {settings.workers} worker{settings.workers === 1 ? "" : "s"}.</p>
      </div>
      <dl>
        <div><dt>Bootstrap</dt><dd>{settings.bootstrapSamples > 0 ? `${settings.bootstrapSamples} replicates` : "off"}</dd></div>
        <div><dt>Permutation</dt><dd>{settings.permutationSamples > 0 ? `${settings.permutationSamples} samples` : "off"}</dd></div>
        <div><dt>Scope</dt><dd>{selectedStatus === "validated" ? "Validated documented scope" : methodStatusDescription(selectedMethod, settings)}</dd></div>
      </dl>
    </section>

    <div className="method-table"><div className="method-table-head"><span>Method</span><span>Family</span><span>Status</span></div>{methods.map((method) => {
      const selectable = isSelectableAnalysisMethod(method);
      return <button type="button" className={`method-row ${settings.method === method.id ? "selected" : ""}`} key={method.id} disabled={!selectable} title={methodStatusDescription(method, settings)} onClick={() => { if (selectable) setSettings({ method: method.id }); }}>
        <strong>{method.name}</strong><span>{method.family}</span><MethodStatusPill method={method} />
      </button>;
    })}</div>
  </section>;
}

function SelectField({ label, value, columns, labels, empty, onChange }: { label: string; value: string; columns: string[]; labels?: Map<string, string>; empty: string; onChange: (value: string) => void }) {
  return <label>{label}<select value={value} onChange={(event) => onChange(event.target.value)}><option value="">{empty}</option>{columns.map((column) => <option key={column} value={column}>{labels?.get(column) ?? column}</option>)}</select></label>;
}

function NumberField({ label, value, min, max, step, onChange }: { label: string; value: number; min: number; max: number; step: number; onChange: (value: number) => void }) {
  return <label>{label}<input type="number" min={min} max={max} step={step} value={value} onChange={(event) => onChange(Math.min(max, Math.max(min, Math.trunc(Number(event.target.value) || min))))} /></label>;
}

function RegressionSettings({ columns }: { columns: string[] }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  return <>
    <label>Regression type<select value={settings.regressionType ?? "ols"} onChange={(event) => setSettings({ regressionType: event.target.value as "ols" | "logistic" | "process" })}><option value="ols">OLS</option><option value="logistic">Logistic</option><option value="process">PROCESS-style</option></select></label>
    <SelectField label="Outcome" value={settings.regressionOutcome ?? ""} columns={columns} empty="Select outcome" onChange={(value) => setSettings({ regressionOutcome: value || null })} />
    <label>Predictors<input value={settings.regressionPredictors ?? ""} onChange={(event) => setSettings({ regressionPredictors: event.target.value })} placeholder="COMP1, LIKE1" /></label>
    <label>Controls<input value={settings.regressionControls ?? ""} onChange={(event) => setSettings({ regressionControls: event.target.value })} placeholder="Optional controls" /></label>
    <label>Robust SE<select value={settings.robustSe ?? "hc3"} onChange={(event) => setSettings({ robustSe: event.target.value as "none" | "hc0" | "hc3" | "hc4" })}><option value="hc3">HC3</option><option value="hc0">HC0</option><option value="hc4">HC4</option><option value="none">Classical</option></select></label>
  </>;
}

function NcaSettings({ columns }: { columns: string[] }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  return <><SelectField label="NCA X" value={settings.ncaX ?? ""} columns={columns} empty="Select X" onChange={(value) => setSettings({ ncaX: value || null })} /><SelectField label="NCA Y" value={settings.ncaY ?? ""} columns={columns} empty="Select Y" onChange={(value) => setSettings({ ncaY: value || null })} /></>;
}

function PcaSettings({ columns }: { columns: string[] }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  return <><label>PCA variables<input value={settings.pcaVariables ?? columns.join(",")} onChange={(event) => setSettings({ pcaVariables: event.target.value })} /></label><label>Component rule<select value={settings.pcaComponentRule ?? "kaiser"} onChange={(event) => setSettings({ pcaComponentRule: event.target.value as "kaiser" | "fixed" | "variance_threshold" })}><option value="kaiser">Kaiser</option><option value="fixed">Fixed count</option><option value="variance_threshold">Variance threshold</option></select></label></>;
}

function CbsemSettings({ columns }: { columns: string[] }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  return <><label>CB-SEM model<select value={settings.cbsemModelType ?? "sem"} onChange={(event) => setSettings({ cbsemModelType: event.target.value as "cfa" | "sem" })}><option value="sem">SEM with structural paths</option><option value="cfa">CFA measurement model</option></select></label><SelectField label="CB-SEM group column" value={settings.cbsemGroupColumn ?? ""} columns={columns} empty="No multigroup analysis" onChange={(value) => setSettings({ cbsemGroupColumn: value || null })} /></>;
}

function PredictSettings() {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  return <><label>Segmentation workflow<select value={settings.groupMethods?.includes("fimix") ? "fimix" : "pls_pos"} onChange={(event) => setSettings({ groupMethods: event.target.value })}><option value="pls_pos">PLS-POS</option><option value="fimix">FIMIX-PLS</option></select></label><NumberField label="Segment count" value={settings.segmentCount ?? 2} min={2} max={5} step={1} onChange={(value) => setSettings({ segmentCount: value })} /><NumberField label="Segment starts" value={settings.segmentStarts ?? 10} min={1} max={50} step={1} onChange={(value) => setSettings({ segmentStarts: value })} /></>;
}
