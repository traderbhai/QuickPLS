import { AlertTriangle, CheckCircle2, Clock3, LockKeyhole, Play, SlidersHorizontal } from "lucide-react";
import { methods } from "../data/sample";
import { analysisReadiness } from "../domain/analysisReadiness";
import { analysisCatalogCapabilityEntriesV2 } from "../domain/analysisCatalogCapabilityV2";
import { evaluateMethodApplicability, methodCategoryLabels, type ApplicabilityStatus, type MethodApplicability } from "../domain/methodApplicability";
import { methodStatusDescription, methodStatusLabel } from "../domain/methodStatus";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { AnalysisMethodId, MethodDefinition, MethodPresetId } from "../types";
import { ReadinessPanel } from "./ReadinessPanel";
import { Card, MethodScopeDrawer, PageHeader, Panel, StatusBadge, TabStrip, WorkspacePage } from "./Ui";

const presets: Array<{ id: MethodPresetId; label: string; description: string }> = [
  { id: "standard_pls", label: "Standard PLS-SEM", description: "Core PLS path model with documented defaults." },
  { id: "pls_bootstrap", label: "PLS + Bootstrap", description: "Inference-ready PLS setup with bootstrap samples." },
  { id: "plspredict", label: "PLSpredict", description: "Prediction defaults; optional segmentation diagnostics remain Experimental previews." },
  { id: "cbsem_cfa", label: "CB-SEM CFA", description: "Reflective raw-data CFA/SEM ML setup." },
  { id: "ols_regression", label: "OLS Regression", description: "Numeric OLS with HC3 robust standard errors." },
  { id: "nca", label: "NCA", description: "CE-FDH/CR-FDH necessity analysis." },
];

function MethodStatusPill({ method, experimental }: { method: MethodDefinition; experimental: boolean }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const effectiveStatus = experimental ? "experimental" : "validated";
  return <span className={`status-text ${effectiveStatus}`} title={methodStatusDescription(method, settings)}>
    {experimental ? <Clock3 size={15} /> : <CheckCircle2 size={15} />}
    {methodStatusLabel(effectiveStatus)}
  </span>;
}

export function AnalysisCatalog() {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setup = useWorkspace((state) => state.methodSetupState);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setSettings = useWorkspace((state) => state.setAnalysisSettings);
  const setSetup = useWorkspace((state) => state.setMethodSetupState);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);
  const applyPreset = useWorkspace((state) => state.applyMethodPreset);
  const columns = useWorkspace((state) => state.dataset.columns);
  const dataset = useWorkspace((state) => state.dataset);
  const edges = useWorkspace((state) => state.edges);
  const nodes = useWorkspace((state) => state.nodes);
  const setView = useWorkspace((state) => state.setView);
  const catalogEntries = analysisCatalogCapabilityEntriesV2(methods, settings, {
    experimentalLabsEnabled: uiPreferences.experimentalLabsEnabled,
  });
  const visibleCatalogEntries = catalogEntries.filter((entry) => entry.availability.selectable);
  const visibleMethodIds = new Set(visibleCatalogEntries.map((entry) => entry.method.id));
  const selectedRegistryEntry = visibleCatalogEntries.find((entry) => entry.method.id === settings.method);

  if (visibleCatalogEntries.length === 0) {
    return <WorkspacePage className="setup-v2-workspace" data-capability-registry-empty="true">
      <PageHeader title="Calculation Setup" description="No analysis currently meets all Standard requirements." />
      <Panel title="No Standard analyses are available yet" description="Requirements">
        <p>QuickPLS will show an analysis here only after its complete workflow and scientific evidence are ready.</p>
        <button type="button" className="secondary-button" onClick={() => setView("settings")}>Open Settings</button>
      </Panel>
    </WorkspacePage>;
  }

  if (!selectedRegistryEntry) {
    return <WorkspacePage className="setup-v2-workspace" data-capability-selection-required="true">
      <PageHeader title="Calculation Setup" description="The previous analysis selection is not available in the current product mode." />
      <Panel title="Choose an available analysis" description="Experimental Labs">
        <div className="method-guidance-grid">
          {visibleCatalogEntries.map((entry) => <button
            key={entry.method.id}
            type="button"
            className="method-guidance-card experimental"
            onClick={() => setSettings({ method: entry.method.id as AnalysisMethodId })}
          >
            <strong>{entry.method.name}</strong>
            <span>{entry.method.family}</span>
            <small>Experimental</small>
          </button>)}
        </div>
      </Panel>
    </WorkspacePage>;
  }
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const applicability = evaluateMethodApplicability({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() })
    .filter((entry) => visibleMethodIds.has(entry.method.id));
  const selectedMethod = methods.find((method) => method.id === settings.method) ?? methods[0] as MethodDefinition;
  const selectedApplicability = applicability.find((item) => item.method.id === settings.method);
  const selectedStatus = selectedRegistryEntry.availability.tier === "standard" ? "validated" : "experimental";
  const basicFieldsReady = readiness.canRun;
  const methodCards = [
    { title: "Dataset", detail: dataset.columns.length ? `${dataset.columns.length} variables loaded` : "Import a dataset first", tone: dataset.columns.length ? "validated" : "warning" },
    { title: "Model", detail: nodes.every((node) => node.data.indicators.length > 0) ? `${nodes.length} constructs with indicators` : "Some constructs need indicators", tone: nodes.every((node) => node.data.indicators.length > 0) ? "validated" : "warning" },
    { title: "Model compatibility", detail: selectedApplicability?.reason ?? "Review the model and data requirements below.", tone: readiness.canRun ? "validated" : "warning" },
    { title: "Availability", detail: selectedStatus === "validated" ? "Supported setup" : "Experimental; review Method Details before use", tone: selectedStatus === "validated" ? "validated" : "warning" },
  ] as const;

  const groupWorkflowActive = settings.method === "mga" || settings.method === "predict" || settings.method === "ipma";
  const primaryApplicability = applicability.filter((item) => item.category !== "inference_add_on");
  const recommended = primaryApplicability.filter((item) => item.status === "recommended");
  const available = primaryApplicability.filter((item) => item.status === "available");
  const availableAfterSetup = primaryApplicability.filter((item) => item.status === "needs_setup").slice(0, 8);
  const advancedDiagnostics = primaryApplicability.filter((item) => item.category === "assessment_diagnostics" || item.category === "prediction_segmentation");
  const standalone = applicability.filter((item) => item.category === "standalone_analysis" || item.category === "workflow_analysis");
  const inferenceAddOns = applicability.filter((item) => item.category === "inference_add_on");
  const notApplicable = primaryApplicability.filter((item) => ["not_applicable", "unsupported", "experimental"].includes(item.status));
  const applicabilityCounts = {
    recommended: recommended.length,
    available: primaryApplicability.filter((item) => item.status === "available").length,
    needsSetup: primaryApplicability.filter((item) => item.status === "needs_setup").length,
    blocked: primaryApplicability.filter((item) => ["not_applicable", "unsupported", "experimental"].includes(item.status)).length,
  };
  const selectedChecks = selectedApplicability?.checks ?? [];
  const selectedExpectedOutputs = selectedApplicability?.expectedOutputs ?? ["paths", "loadings / weights", "R²", "reliability / validity"];
  const selectedFailure = selectedChecks.find((check) => check.status === "failed");
  const selectedWarningCount = selectedChecks.filter((check) => check.status === "warning").length;
  const selectedBlockingChecks = selectedChecks.filter((check) => check.status === "failed");
  const selectedFirstFailed = selectedBlockingChecks[0];
  const selectedDecisionLabel = selectedApplicability ? applicabilityStatusLabel(selectedApplicability.status) : methodStatusLabel(selectedStatus);
  const selectedDecisionDetail = selectedApplicability?.reason ?? methodStatusDescription(selectedMethod, settings);
  const selectedDecisionAction = selectedFirstFailed?.actionLabel ?? selectedApplicability?.nextActionLabel ?? (basicFieldsReady ? "Open Run" : "Review Setup");
  const selectedDecisionView = selectedFirstFailed?.actionView ?? (basicFieldsReady ? "run" : "analyses");

  return <WorkspacePage className="setup-v2-workspace setup-v212-workspace setup-v2110-workspace setup-v215-workspace setup-v217-workspace setup-v226-workspace" data-method-applicability-polish="v2.11.0" data-workflow-method-guidance-triage="v2.15.0" data-v217-mockup-screen="setup" data-v226-method-setup-center="true">
    <PageHeader title="Calculation Setup" description="Choose the analysis that fits this dataset, SEM model, and the listed requirements." actions={<StatusBadge status={selectedStatus === "validated" ? "validated" : selectedStatus === "experimental" ? "experimental" : "unsupported"}>{methodStatusLabel(selectedStatus)}</StatusBadge>} />

    <section className="setup-v2-hero setup-v217-method-summary qpls2-panel" aria-label="Selected calculation command">
      <div>
        <span className="qpls2-eyebrow">Selected calculation command</span>
        <h2>{selectedMethod.name}</h2>
        <p>{selectedApplicability?.reason ?? methodStatusDescription(selectedMethod, settings)}</p>
        <div className="setup-v2-hero-meta">
          <span>{methodCategoryLabels[selectedApplicability?.category ?? "core_model_estimation"]}</span>
          <span>{selectedExpectedOutputs.slice(0, 4).join(", ")}</span>
        </div>
        <button className="secondary-button" onClick={() => setUiPreferences({ methodScopeDrawerOpen: true })}>Method details</button>
      </div>
      <div className="setup-v2-hero-actions">
        <TabStrip label="Method setup mode" value={setup.mode} onChange={(mode) => setSetup({ mode })} tabs={[{ id: "basic", label: "Basic" }, { id: "expert", label: "Expert" }]} />
        <button className="qpls2-primary-action setup-v212-run-button" disabled={!basicFieldsReady} title={basicFieldsReady ? `Run ${selectedMethod.name}` : selectedFailure?.detail ?? readiness.blockers[0]?.detail ?? readiness.summary} onClick={() => window.dispatchEvent(new CustomEvent("quickpls:run-analysis"))}><Play size={17} fill="currentColor" />Run now</button>
        {!basicFieldsReady ? <span className="disabled-reason inline-disabled-reason">Run disabled: {selectedFailure?.detail ?? readiness.blockers[0]?.detail ?? readiness.summary}</span> : <span className="status-text validated"><CheckCircle2 size={14} />Ready for offline run</span>}
      </div>
    </section>

    <section className="setup-v226-category-tabs qpls2-panel" aria-label="Method categories">
      <button type="button" className="active"><strong>Recommended</strong><span>{recommended.length}</span></button>
      <button type="button"><strong>Available</strong><span>{available.length}</span></button>
      <button type="button"><strong>Needs setup</strong><span>{applicabilityCounts.needsSetup}</span></button>
      <button type="button"><strong>Diagnostics</strong><span>{advancedDiagnostics.length}</span></button>
      <button type="button"><strong>Standalone</strong><span>{standalone.length}</span></button>
      <button type="button"><strong>Not applicable</strong><span>{notApplicable.length}</span></button>
    </section>

    <section className="setup-v2110-applicability-summary qpls2-panel" aria-label="Method availability for this project">
      <div>
        <span className="qpls2-eyebrow">Method availability</span>
        <strong>QuickPLS is filtering methods by the current data, SEM diagram, settings, and method requirements.</strong>
        <p>Use recommended methods first. Methods that need setup stay visible with exact missing fields; incompatible shapes stay under Show all with reasons instead of disappearing.</p>
      </div>
      <dl>
        <div><dt>Recommended</dt><dd>{applicabilityCounts.recommended}</dd></div>
        <div><dt>Available</dt><dd>{applicabilityCounts.available}</dd></div>
        <div><dt>Needs setup</dt><dd>{applicabilityCounts.needsSetup}</dd></div>
        <div><dt>Not available</dt><dd>{applicabilityCounts.blocked}</dd></div>
      </dl>
    </section>

    <section
      className={`setup-v215-decision-panel qpls2-panel ${selectedApplicability?.status ?? selectedStatus}`}
      aria-label="Selected method decision and next action"
      data-selected-method-decision={selectedDecisionLabel}
      data-selected-method-next-action={selectedDecisionAction}
      data-selected-method-first-failed-check={selectedFirstFailed?.id ?? ""}
    >
      <div>
        <span className="qpls2-eyebrow">Recommended next move</span>
        <strong>{selectedDecisionAction}</strong>
        <p><b>{selectedDecisionLabel}:</b> {selectedDecisionDetail}</p>
        {selectedFirstFailed ? <small>First blocker: {selectedFirstFailed.label} - {selectedFirstFailed.detail}</small> : <small>Expected outputs: {selectedExpectedOutputs.slice(0, 5).join(", ")}</small>}
      </div>
      <div className="setup-v215-decision-actions">
        <button className="qpls2-secondary-action" onClick={() => setView(selectedDecisionView)}>{selectedDecisionAction}</button>
        <details>
          <summary>If you expected another method</summary>
          <p>Open Show all methods below. QuickPLS keeps unavailable methods visible with reasons instead of showing them as runnable choices.</p>
        </details>
      </div>
    </section>

    <MethodScopeDrawer method={selectedMethod} open={uiPreferences.methodScopeDrawerOpen} onToggle={() => setUiPreferences({ methodScopeDrawerOpen: !uiPreferences.methodScopeDrawerOpen })} />
    <Panel
      title="Data, model, and method requirements"
      description="Readiness"
      className="setup-v2-readiness"
      actions={<StatusBadge status={readiness.canRun ? "validated" : "experimental"}>{readiness.summary}</StatusBadge>}
    >
      <ReadinessPanel readiness={readiness} onNavigate={setView} />
      <div className="setup-v2-status-strip">
        {methodCards.map((card) => <Card key={card.title} title={card.title} description={card.detail} tone={card.tone} />)}
      </div>
    </Panel>

    <div className="setup-v2-main setup-v217-main">
      <Panel
        title="Recommended methods appear first"
        description="Method guidance"
        className="setup-v2-method-browser"
        actions={<small>{recommended.length} recommended, {availableAfterSetup.length} need setup</small>}
      >
        <div className="setup-v2-browser-body">
          <MethodSection title="Recommended for this project" description="Ready and sensible for the current dataset and SEM model." items={recommended} selectedId={settings.method} onSelect={(method) => setSettings({ method })} empty="No method is fully recommended yet. Complete the failed readiness item above." />
          <MethodSection title="Available now" description="Runnable or valid for this project, but not necessarily the primary recommendation." items={available.slice(0, 8)} selectedId={settings.method} onSelect={(method) => setSettings({ method })} empty="No secondary methods are ready right now." />
          <MethodSection title="Available with setup" description="Possible after one or more required fields are completed." items={availableAfterSetup} selectedId={settings.method} onSelect={(method) => setSettings({ method })} empty="No setup-dependent methods are relevant right now." />
          <div className="setup-v2-collapsed-sections">
            <MethodSection title="Advanced diagnostics" description="Prediction, groups, invariance, endogeneity, nonlinear effects, and composite diagnostics." items={advancedDiagnostics} selectedId={settings.method} onSelect={(method) => setSettings({ method })} collapsed />
            <MethodSection title="Standalone analyses" description="Analyses that use selected variables and do not always require the SEM diagram." items={standalone} selectedId={settings.method} onSelect={(method) => setSettings({ method })} collapsed />
            <MethodSection title="Not applicable or unavailable" description="Methods stay visible with exact reasons instead of being offered as runnable choices." items={notApplicable} selectedId={settings.method} onSelect={(method) => setSettings({ method })} collapsed />
          </div>
        </div>
      </Panel>

      <Panel
        title="Requirements and fields"
        description="Selected method"
        className="setup-v2-sidecar"
        actions={<StatusBadge status={selectedStatus === "validated" ? "validated" : selectedStatus === "experimental" ? "experimental" : "unsupported"}>{methodStatusLabel(selectedStatus)}</StatusBadge>}
      >
        <div className="setup-v2-sidecar-body">
          <div className={`setup-v2-selected-card ${selectedApplicability?.status ?? "available"}`}>
            <div>
              <span>{methodCategoryLabels[selectedApplicability?.category ?? "core_model_estimation"]}</span>
              <strong>{selectedMethod.name}</strong>
              <p>{selectedApplicability?.reason ?? methodStatusDescription(selectedMethod, settings)}</p>
            </div>
            <b>{applicabilityStatusLabel(selectedApplicability?.status ?? "available")}</b>
          </div>

          <section className="setup-v2-requirements" aria-label="Selected method requirement checks">
            <header><strong>Requirement checks</strong><small>{selectedChecks.filter((check) => check.status === "passed").length} passed, {selectedWarningCount} warnings</small></header>
            {selectedBlockingChecks.length ? <p className="setup-v2110-why-not">Why not available yet: {selectedBlockingChecks[0].detail}</p> : <p className="setup-v2110-why-not ready">All required checks for the selected method are satisfied.</p>}
            <div>
              {selectedChecks.map((check) => <button key={check.id} type="button" className={`setup-v2-requirement ${check.status}`} onClick={() => check.actionView ? setView(check.actionView) : undefined}>
                {check.status === "passed" ? <CheckCircle2 size={15} /> : check.status === "warning" ? <AlertTriangle size={15} /> : <LockKeyhole size={15} />}
                <span><strong>{check.label}</strong><small>{check.detail}</small></span>
                {check.actionLabel ? <b>{check.actionLabel}</b> : null}
              </button>)}
            </div>
          </section>

          <section className="setup-v226-addons" aria-label="Inference add-ons">
            <header><strong>Inference add-ons</strong><small>Configured with the primary estimator</small></header>
            {inferenceAddOns.map((item) => <div key={item.method.id} className={`setup-v226-addon ${item.status}`}>
              <div>
                <strong>{item.method.name}</strong>
                <span>{item.reason}</span>
              </div>
              <label className="checkbox-row">
                Enable
                <input
                  type="checkbox"
                  checked={settings.bootstrapSamples > 0}
                  onChange={(event) => setSettings(event.target.checked ? { bootstrapSamples: 5000 } : { bootstrapSamples: 0, studentizedInnerSamples: 0 })}
                />
              </label>
            </div>)}
            <div className="setup-v226-addon muted">
              <div>
                <strong>Freedman-Lane structural path randomization</strong>
                <span>Expert single-model inference with fixed original PLS construct scores and unadjusted pathwise two-sided plus-one p values; not a group comparison.</span>
              </div>
              <span className="applicability-pill needs_setup">Expert</span>
            </div>
          </section>

          <div className="analysis-settings guided-settings setup-v2-settings">
            <div><strong>Basic setup</strong><span className={readiness.canRun ? "status-text validated" : "status-text experimental"}>{readiness.canRun ? <CheckCircle2 size={14} /> : <Clock3 size={14} />}{readiness.canRun ? "ready" : "needs attention"}</span></div>
            <label>Run method<select value={settings.method} onChange={(event) => setSettings({ method: event.target.value as AnalysisMethodId })}>
              {primaryApplicability.filter((item) => ["recommended", "available", "needs_setup"].includes(item.status)).map((item) => <option key={item.method.id} value={item.method.id}>{item.method.name} | {applicabilityStatusLabel(item.status)}</option>)}
            </select></label>
            <label className="checkbox-row">Bootstrap<input type="checkbox" checked={settings.bootstrapSamples > 0} onChange={(event) => setSettings(event.target.checked ? { bootstrapSamples: 5000 } : { bootstrapSamples: 0, studentizedInnerSamples: 0 })} /></label>

            {settings.method === "wpls" && <SelectField label="Case weight column" value={settings.caseWeightColumn ?? ""} columns={columns} empty="Select column" onChange={(value) => setSettings({ caseWeightColumn: value || null })} />}
            {settings.method === "mga" && <SelectField label="Group column" value={settings.groupColumn ?? ""} columns={columns} empty="Select two-group column" onChange={(value) => setSettings({ groupColumn: value || null })} />}
            {settings.method === "ipma" && <SelectField label="IPMA target" value={settings.ipmaTargets ?? ""} columns={nodes.map((node) => node.id)} labels={new Map(nodes.map((node) => [node.id, node.data.label]))} empty="All endogenous constructs" onChange={(value) => setSettings({ ipmaTargets: value || null })} />}
            {settings.method === "regression" && <RegressionSettings columns={columns} />}
            {settings.method === "nca" && <NcaSettings columns={columns} />}
            {settings.method === "pca" && <PcaSettings columns={columns} />}
            {settings.method === "cbsem" && <CbsemSettings columns={columns} />}

            {setup.mode === "expert" && <details className="settings-section advanced-settings" open>
              <summary><SlidersHorizontal size={14} /> Expert resampling and reproducibility</summary>
              {settings.method === "mga" && <label>Group workflow<select value={settings.groupMethods === "micom" ? "micom" : ""} onChange={() => setSettings({ groupMethods: "micom" })}><option value="">Choose the current workflow</option><option value="micom">MICOM v3.1</option></select></label>}
              {settings.method === "mga" && <NumberField label="Group permutation samples" value={settings.groupPermutationSamples ?? 5_000} min={5_000} max={10_000} step={100} onChange={(value) => setSettings({ groupPermutationSamples: value })} />}
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
        </div>
      </Panel>
    </div>

    <Panel title="Start from a common analysis setup" description="Research presets" className="setup-v2-presets">
      <div className="method-preset-grid">
        {presets.map((preset) => <button key={preset.id} className={setup.selectedPreset === preset.id ? "method-preset-card selected" : "method-preset-card"} onClick={() => applyPreset(preset.id)}>
          <strong>{preset.label}</strong><span>{preset.description}</span>
        </button>)}
      </div>
    </Panel>

    {setup.mode === "expert" ? <section className="group-setup-card setup-v2-expert-workflows" aria-label="Group and prediction workflow setup">
      <div>
        <strong>Group and prediction workflows</strong>
        <p>IPMA and Experimental PLS-POS-style or FIMIX-style diagnostics are configured here, then reviewed from the appropriate Results tabs.</p>
      </div>
      <div className="group-setup-actions">
        <button className={settings.method === "predict" ? "secondary-button active" : "secondary-button"} onClick={() => setSettings({ method: "predict", groupMethods: "pls_pos" })}>Segmentation preview setup</button>
        <button className={settings.method === "ipma" ? "secondary-button active" : "secondary-button"} onClick={() => setSettings({ method: "ipma" })}>IPMA setup</button>
      </div>
      <small>{groupWorkflowActive ? "A group or prediction workflow is selected. Completed group outputs will appear in Results > Groups." : "Select a group workflow only when your research design needs invariance, group comparison, segmentation, or IPMA output."}</small>
    </section> : null}

    <section className="setup-launch-panel setup-v2-launch" aria-label="Setup launch summary">
      <div>
        <strong>Ready-to-run summary</strong>
        <p>{selectedMethod.name} on {dataset.name} with {nodes.length} constructs, {edges.filter((edge) => edge.data?.role !== "covariance").length} structural paths, seed {settings.seed}, and {settings.workers} worker{settings.workers === 1 ? "" : "s"}.</p>
      </div>
      <dl>
        <div><dt>Bootstrap</dt><dd>{settings.bootstrapSamples > 0 ? `${settings.bootstrapSamples} replicates` : "off"}</dd></div>
        <div><dt>Permutation</dt><dd>{settings.permutationSamples > 0 ? `${settings.permutationSamples} samples` : "off"}</dd></div>
        <div><dt>Requirements</dt><dd>{selectedStatus === "validated" ? "Supported setup" : methodStatusDescription(selectedMethod, settings)}</dd></div>
      </dl>
      <div className="setup-launch-actions">
        <button className="qpls2-primary-action setup-v212-run-button" disabled={!basicFieldsReady} title={basicFieldsReady ? `Run ${selectedMethod.name}` : readiness.blockers[0]?.detail ?? readiness.summary} onClick={() => window.dispatchEvent(new CustomEvent("quickpls:run-analysis"))}><Play size={17} fill="currentColor" />Run selected method</button>
        {!basicFieldsReady ? <span className="disabled-reason inline-disabled-reason"><AlertTriangle size={14} />{readiness.blockers[0]?.detail ?? readiness.summary}</span> : <button className="secondary-button" onClick={() => setView("run")}>Open run monitor</button>}
      </div>
    </section>

    <Panel title="Calculation preview" description="Outputs expected from the selected settings before the offline engine starts." className="calculation-preview-panel setup-v2-preview">
      <div className="calculation-preview-grid">
        <Card title="Algorithm" description={`${selectedMethod.name}; ${methodStatusDescription(selectedMethod, settings)}`} tone={selectedStatus === "validated" ? "validated" : "warning"} />
        <Card title="Produced outputs" description={selectedExpectedOutputs.concat(settings.bootstrapSamples > 0 ? ["bootstrap inference"] : []).join(", ")} tone="validated" />
        <Card title="Unavailable outputs" description={settings.bootstrapSamples > 0 ? "Permutation and other variants appear only when configured." : "p values and confidence intervals require bootstrap or permutation settings."} tone={settings.bootstrapSamples > 0 ? "validated" : "warning"} />
        <Card title="After run" description="Completed runs open in Results with reportability checklist, interpretation notes, export tables, and publication diagram overlays." />
      </div>
    </Panel>

    <details className="show-all-methods setup-v2-all-methods">
      <summary>Show all methods, including choices that are not available for this project</summary>
      <div className="method-table"><div className="method-table-head"><span>Method</span><span>Family</span><span>Status</span></div>{visibleCatalogEntries.map(({ method, availability }) => {
      return <button type="button" className={`method-row ${settings.method === method.id ? "selected" : ""}`} key={method.id} title={methodStatusDescription(method, settings)} onClick={() => setSettings({ method: method.id as AnalysisMethodId })}>
        <strong>{method.name}</strong><span>{method.family}</span><MethodStatusPill method={method} experimental={availability.tier === "experimental"} />
      </button>;
    })}</div>
    </details>
  </WorkspacePage>;
}

function applicabilityStatusLabel(status: ApplicabilityStatus) {
  if (status === "recommended") return "Recommended";
  if (status === "available") return "Available";
  if (status === "needs_setup") return "Needs setup";
  if (status === "not_applicable") return "Not applicable now";
  if (status === "experimental") return "Experimental";
  return "Not available";
}

function MethodSection({ title, description, items, selectedId, onSelect, empty, collapsed = false }: { title: string; description: string; items: MethodApplicability[]; selectedId: AnalysisMethodId; onSelect: (method: AnalysisMethodId) => void; empty?: string; collapsed?: boolean }) {
  const body = <div className="method-guidance-grid">
    {items.length ? items.map((item) => {
      const failedCheck = item.checks.find((check) => check.status === "failed");
      return <button
        key={item.method.id}
        type="button"
        className={`method-guidance-card ${item.status} ${selectedId === item.method.id ? "selected" : ""}`}
        data-method-id={item.method.id}
        data-method-status={item.status}
        data-method-category={item.category}
        data-method-next-action={item.nextActionLabel}
        data-method-failed-check={failedCheck?.id ?? ""}
        aria-label={`${item.method.name}: ${applicabilityStatusLabel(item.status)}. ${item.reason}`}
        onClick={() => onSelect(item.method.id as AnalysisMethodId)}
      >
        <div className="method-guidance-card-head">
          <strong>{item.method.name}</strong>
          <span className={`applicability-pill ${item.status}`}>{applicabilityStatusLabel(item.status)}</span>
        </div>
        <span>{methodCategoryLabels[item.category]}</span>
        <p>{item.reason}</p>
        {failedCheck ? <small className="method-guidance-needed">Needs: {failedCheck.label} - {failedCheck.actionLabel ?? "Complete setup"}</small> : null}
        <small>Expected: {item.expectedOutputs.slice(0, 4).join(", ")}</small>
        <b>{item.nextActionLabel}</b>
      </button>;
    }) : <p className="method-guidance-empty">{empty ?? "No methods in this section."}</p>}
  </div>;
  if (collapsed) {
    return <details className="method-guidance-section">
      <summary><span><strong>{title}</strong><small>{description}</small></span></summary>
      {body}
    </details>;
  }
  return <section className="method-guidance-section">
    <header><strong>{title}</strong><small>{description}</small></header>
    {body}
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
  return <><label>Experimental segmentation diagnostic<select value={settings.groupMethods?.includes("fimix") ? "fimix" : "pls_pos"} onChange={(event) => setSettings({ groupMethods: event.target.value })}><option value="pls_pos">PLS-POS-style Experimental diagnostic</option><option value="fimix">FIMIX-style Experimental diagnostic (not full EM)</option></select></label><NumberField label="Segment count" value={settings.segmentCount ?? 2} min={2} max={5} step={1} onChange={(value) => setSettings({ segmentCount: value })} /><NumberField label="Segment starts" value={settings.segmentStarts ?? 10} min={1} max={50} step={1} onChange={(value) => setSettings({ segmentStarts: value })} /></>;
}
