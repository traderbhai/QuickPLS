import { AlertTriangle, CheckCircle2, ChevronDown, Copy, FlaskConical, Search } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { useWorkspace } from "../store";
import type { AnalysisRun, AssessmentResult, HtmtAssessment, PlsResult, ResultWorkspaceTab } from "../types";
import { findBcaParameter, findBootstrapParameter, findStudentizedParameter, formatParameterIdentity } from "../domain/inference";
import { analysisReadiness } from "../domain/analysisReadiness";
import { buildResultInterpretation, copyableInterpretationText, findingsByGroup, findingsForTab, rowSpecificInterpretation, type InterpretationFinding, type ResultInterpretation, type SemDiagramEdgeLike, type SemDiagramNodeLike } from "../domain/resultInterpretation";
import { isNativeDesktop } from "../services/projectService";
import { ReadinessPanel } from "./ReadinessPanel";
import { EmptyState, MethodConfidencePanel, MetricCard, PageHeader, Panel, ReportabilityChecklist, StatusBadge, WorkspacePage, type ReportabilityItem } from "./Ui";

const resultTabs: Array<{ id: ResultWorkspaceTab; label: string }> = [
  { id: "overview", label: "Overview" },
  { id: "measurement", label: "Measurement" },
  { id: "structural", label: "Structural" },
  { id: "validity", label: "Validity" },
  { id: "inference", label: "Inference" },
  { id: "prediction", label: "Prediction" },
  { id: "groups", label: "Groups" },
  { id: "diagnostics", label: "Diagnostics" },
  { id: "interpretation", label: "Interpretation" },
  { id: "comparison", label: "Comparison" },
];

function resultTabHint(tab: ResultWorkspaceTab) {
  switch (tab) {
    case "overview": return "run summary";
    case "measurement": return "loadings";
    case "structural": return "paths and R²";
    case "validity": return "reliability";
    case "inference": return "bootstrap";
    case "prediction": return "Q² predict";
    case "groups": return "MGA/IPMA";
    case "diagnostics": return "warnings";
    case "interpretation": return "checklist";
    case "comparison": return "two runs";
    default: return "";
  }
}

function resultTabLabel(tab: ResultWorkspaceTab) {
  return resultTabs.find((item) => item.id === tab)?.label ?? "Results";
}

export function RunHistory() {
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const resultState = useWorkspace((state) => state.resultWorkspaceState);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const [openResultsMenu, setOpenResultsMenu] = useState<null | "view" | "table" | "export" | "interpretation">(null);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const search = resultState.tableSearch.toLowerCase();
  const visibleRuns = runs.filter((run) => {
    const body = `${run.name} ${run.method} ${run.warnings.join(" ")} ${run.result?.paths.map((path) => `${path.source} ${path.target}`).join(" ") ?? ""}`.toLowerCase();
    return body.includes(search);
  });
  const selectedRun = visibleRuns.find((run) => run.id === resultState.selectedRunId) ?? visibleRuns[0];
  const significantWarningCount = selectedRun?.warnings.filter((warning) => !warning.toLowerCase().includes("validated")).length ?? 0;
  const bestR2 = selectedRun?.result ? Object.entries(selectedRun.result.r_squared).sort((a, b) => b[1] - a[1])[0] : null;
  const selectedInterpretation = selectedRun?.result ? buildResultInterpretation({ run: selectedRun, nodes, edges }) : null;
  const selectedComparisonRuns = resultState.comparisonRunIds
    .map((id) => runs.find((run) => run.id === id))
    .filter((run): run is AnalysisRun => Boolean(run?.result));
  const selectedEdge = edges.find((edge) => edge.id === selectedEdgeId);
  const activePath = selectedEdge ? { source: selectedEdge.source, target: selectedEdge.target } : null;
  const focusPath = (source: string, target: string) => {
    const edge = edges.find((candidate) => candidate.source === source && candidate.target === target);
    if (edge) {
      setSelectedEdge(edge.id);
      window.dispatchEvent(new CustomEvent("quickpls:focus-edge", { detail: { id: edge.id } }));
    } else {
      setSelectedNode(target);
      window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id: target } }));
    }
    setView("models");
  };
  const copyVisibleSummary = async () => {
    const text = visibleRuns.map((run) => `${run.name}\t${run.method}\t${run.status}\t${run.createdAt}`).join("\n");
    await navigator.clipboard?.writeText(text);
  };
  const exportCurrentTable = () => {
    const csv = selectedRun ? csvForCurrentResultTab(selectedRun, resultState.selectedTab) : "";
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `quickpls-${resultState.selectedTab}-results.csv`;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };
  const emptyPrimary = readiness.blockers[0]?.actionView ?? (readiness.canRun ? "run" : "analyses");
  const emptyPrimaryLabel = readiness.blockers[0]?.actionLabel ?? (readiness.canRun ? "Run method" : "Open setup");
  const previewTabs = ["Overview", "Measurement", "Structural", "Validity", "Inference", "Diagnostics"];

  useEffect(() => {
    const copyTable = () => {
      if (selectedRun?.result) {
        void navigator.clipboard?.writeText(csvForCurrentResultTab(selectedRun, resultState.selectedTab));
      }
    };
    const exportTable = () => {
      if (selectedRun?.result) exportCurrentTable();
    };
    const selectRun = () => {
      const selector = document.querySelector<HTMLSelectElement>('[aria-label="Selected completed run"]');
      selector?.focus();
    };
    const copyRunList = () => {
      void copyVisibleSummary();
    };
    const openInterpretation = () => setResultState({ selectedTab: "interpretation" });
    const openComparison = () => setResultState({ selectedTab: "comparison" });
    const prepareReport = () => setView("reports");
    const toggleConfidence = () => setUiPreferences({ methodScopeDrawerOpen: !uiPreferences.methodScopeDrawerOpen });
    window.addEventListener("quickpls:results-select-run", selectRun);
    window.addEventListener("quickpls:results-copy-run-list", copyRunList);
    window.addEventListener("quickpls:results-copy-current-table", copyTable);
    window.addEventListener("quickpls:results-export-current-table", exportTable);
    window.addEventListener("quickpls:results-open-interpretation", openInterpretation);
    window.addEventListener("quickpls:results-open-comparison", openComparison);
    window.addEventListener("quickpls:results-prepare-report", prepareReport);
    window.addEventListener("quickpls:results-method-confidence", toggleConfidence);
    return () => {
      window.removeEventListener("quickpls:results-select-run", selectRun);
      window.removeEventListener("quickpls:results-copy-run-list", copyRunList);
      window.removeEventListener("quickpls:results-copy-current-table", copyTable);
      window.removeEventListener("quickpls:results-export-current-table", exportTable);
      window.removeEventListener("quickpls:results-open-interpretation", openInterpretation);
      window.removeEventListener("quickpls:results-open-comparison", openComparison);
      window.removeEventListener("quickpls:results-prepare-report", prepareReport);
      window.removeEventListener("quickpls:results-method-confidence", toggleConfidence);
    };
  }, [selectedRun, visibleRuns, resultState.selectedTab, uiPreferences.methodScopeDrawerOpen]);

  if (runs.length === 0) return <WorkspacePage data-v218-mockup-screen="results-empty" className="results-v2-workspace results-v213-workspace results-v218-workspace">
    <PageHeader title="Results" description="Completed runs, immutable recipes, estimates, and provenance records." actions={<><StatusBadge status="warning">no completed run</StatusBadge><button className="secondary-button" type="button" onClick={() => setView("trust")}>Why trust this result?</button></>} />
    <Panel title="Result workbook" description="No completed run is selected yet." className="results-workbench-shell results-v2-command-center results-v2-empty-command results-v213-command-center">
      <nav className="results-section-nav results-v2-section-nav" aria-label="Result sections preview">
        <div className="results-v2-nav-header"><strong>Result workbook</strong><span>No selected run</span></div>
        {previewTabs.map((tab) => <span key={tab}>{tab}</span>)}
      </nav>
      <div className="results-tool-stack" aria-label="Result confidence controls">
        <button className="secondary-button" type="button" onClick={() => setView("trust")}>Why trust this result?</button>
        <button className="secondary-button" type="button" onClick={() => setView("analyses")}>Open setup</button>
      </div>
    </Panel>
    <ReadinessPanel readiness={readiness} compact onNavigate={setView} />
    <EmptyState title="No completed results" description={readiness.canRun ? "Run the selected method to create the first result." : readiness.blockers[0]?.detail ?? "Complete the analysis checklist before running."} actions={<><button className="run-button" onClick={() => setView(emptyPrimary)}>{emptyPrimaryLabel}</button><button className="secondary-button" onClick={() => setView("analyses")}>Open setup</button></>} />
  </WorkspacePage>;

  return <WorkspacePage data-v218-mockup-screen="results" data-v228-results-workbook="true" className={`results-v2-workspace results-v213-workspace results-v218-workspace results-v228-workspace${uiPreferences.showThresholdColors ? " threshold-colors-enabled" : " threshold-colors-hidden"}`}>
    <PageHeader title="Results" description="Review one saved run at a time with scoped interpretation, reportability checks, and export-ready tables." actions={<StatusBadge status="validated">{visibleRuns.length} visible</StatusBadge>} />
    <Panel title="Result workbook" description="Choose a section, refine tables, export, or open interpretation guidance." className="results-workbench-shell results-v2-command-center results-v213-command-center">
      <nav className="results-section-nav results-v2-section-nav" aria-label="Result sections">
        <div className="results-v2-nav-header"><strong>Result workbook</strong><span>{selectedRun ? selectedRun.name : "No selected run"}</span></div>
        {resultTabs.map((tab) => <button key={tab.id} type="button" className={resultState.selectedTab === tab.id ? "active" : undefined} aria-current={resultState.selectedTab === tab.id ? "page" : undefined} onClick={() => setResultState({ selectedTab: tab.id })}>
          <span>{tab.label}</span>
          <small>{resultTabHint(tab.id)}</small>
        </button>)}
      </nav>
      <div className="results-tool-stack" aria-label="Result table tools">
        <label className="result-search"><Search size={13} /><input aria-label="Search result tables" placeholder="Search runs, paths, warnings" value={resultState.tableSearch} onChange={(event) => setResultState({ tableSearch: event.target.value })} /></label>
        <ResultMenu label="View" name="view" open={openResultsMenu} onOpen={setOpenResultsMenu}>
          <label className="compact-select-label">Precision <select aria-label="Result precision" value={resultState.resultPrecision} onChange={(event) => setResultState({ resultPrecision: Number(event.target.value) })}>{[2, 3, 4, 5, 6].map((digits) => <option key={digits} value={digits}>{digits} decimals</option>)}</select></label>
          <button type="button" onClick={() => setResultState({ tableDensity: resultState.tableDensity === "compact" ? "comfortable" : "compact" })}>{resultState.tableDensity === "compact" ? "Comfortable density" : "Compact density"}</button>
          <button type="button" onClick={() => setUiPreferences({ showThresholdColors: !uiPreferences.showThresholdColors })}>{uiPreferences.showThresholdColors ? "Hide threshold colors" : "Show threshold colors"}</button>
        </ResultMenu>
        <ResultMenu label="Table" name="table" open={openResultsMenu} onOpen={setOpenResultsMenu}>
          <button type="button" onClick={() => void copyVisibleSummary()}><Copy size={14} />Copy run list</button>
          <button type="button" disabled={!selectedRun?.result} title={selectedRun?.result ? "Export the currently visible table" : "Run a method before exporting a table"} onClick={exportCurrentTable}>Export current table</button>
          <button type="button" onClick={() => setResultState({ showInterpretationColumns: !resultState.showInterpretationColumns })}>{resultState.showInterpretationColumns ? "Hide guidance columns" : "Show guidance columns"}</button>
        </ResultMenu>
        <ResultMenu label="Export" name="export" open={openResultsMenu} onOpen={setOpenResultsMenu}>
          <button type="button" disabled={!selectedRun?.result} title={selectedRun?.result ? "Prepare report from the selected run" : "Run a method before preparing a report"} onClick={() => setView("reports")}>Prepare report</button>
          <button type="button" disabled={!selectedRun?.result} onClick={exportCurrentTable}>Download current CSV</button>
        </ResultMenu>
        <ResultMenu label="Interpretation" name="interpretation" open={openResultsMenu} onOpen={setOpenResultsMenu}>
          <button type="button" onClick={() => setResultState({ includeExperimental: !resultState.includeExperimental })}>{resultState.includeExperimental ? "Hide experimental outputs" : "Validated scope only"}</button>
          <button type="button" onClick={() => setResultState({ selectedTab: "interpretation" })}>Open interpretation checklist</button>
          <button type="button" disabled={!selectedRun?.result} onClick={() => selectedRun?.result && navigator.clipboard?.writeText(copyableInterpretationText(buildResultInterpretation({ run: selectedRun, nodes, edges }).findings))}>Copy interpretation</button>
        </ResultMenu>
      </div>
    </Panel>
    {selectedRun ? <Panel title="Selected run context" description="Method, sample, seed, fingerprint, warnings, and report handoff." className="results-run-context-sticky results-v2-run-hero results-v213-run-context results-v228-run-header">
      <div className="results-run-context-main">
        <label><span>Selected run</span>
          <select aria-label="Selected completed run" value={selectedRun.id} onChange={(event) => setResultState({ selectedRunId: event.target.value })}>
            {visibleRuns.map((run) => <option key={run.id} value={run.id}>{run.name}</option>)}
          </select>
        </label>
        <div><span>Active results view</span><strong>{resultTabLabel(resultState.selectedTab)}</strong><small>{resultTabHint(resultState.selectedTab)}</small></div>
      </div>
      <div className="results-run-context-meta">
        <span>{selectedRun.method}</span>
        <span>{selectedRun.result?.used_observations ?? "N/A"} observations</span>
        <span>seed {selectedRun.seed}</span>
        <span>fingerprint {selectedRun.fingerprint.slice(0, 10)}</span>
        <StatusBadge status="validated">Validated scope</StatusBadge>
        <span className={significantWarningCount ? "run-warning-chip warning" : "run-warning-chip"}>{significantWarningCount ? `${significantWarningCount} warning(s)` : "No extra warnings"}</span>
        <button className="secondary-button" type="button" onClick={() => setView("reports")}>Prepare report</button>
        <details className="run-confidence-details">
          <summary>Why trust this result?</summary>
          <MethodConfidencePanel run={selectedRun} />
        </details>
      </div>
    </Panel> : null}
    <div className="results-v228-workbook-body">
      <main className="results-v228-table-area" aria-label="Selected result workbook tables">
        {selectedRun && selectedInterpretation ? <ResultsV2LensPanel run={selectedRun} tab={resultState.selectedTab} interpretation={selectedInterpretation} bestR2={bestR2} warningCount={significantWarningCount} /> : null}
        {selectedRun ? <div className="result-headline-grid results-v2-summary-row results-v228-summary-row">
          <MetricCard label="Selected run" value={selectedRun.name} detail={selectedRun.method} />
          <MetricCard label="Strongest R²" value={bestR2 ? bestR2[1].toFixed(resultState.resultPrecision) : "N/A"} detail={bestR2?.[0] ?? "No endogenous construct"} />
          <MetricCard label="Paths" value={selectedRun.result?.paths.length ?? 0} detail="Click a row to focus the diagram" />
          <MetricCard label="Warnings" value={significantWarningCount} detail={significantWarningCount ? "Review provenance before export" : "No extra warnings"} tone={significantWarningCount ? "warning" : "success"} />
        </div> : null}
        {resultState.selectedTab === "groups" ? <Panel title="Groups and segmentation results" description="MICOM, permutation MGA, FIMIX-PLS, PLS-POS, and IPMA outputs appear here when the selected run contains those payloads." className="results-groups-bridge results-v213-groups-bridge">
          <div>
            <strong>Groups and segmentation results</strong>
            <p>MICOM, permutation MGA, FIMIX-PLS, PLS-POS, and IPMA outputs appear here when the selected run contains those payloads.</p>
          </div>
          <button className="secondary-button" onClick={() => setView("analyses")}>Configure group workflow in Setup</button>
        </Panel> : null}
        <div className={`run-list results-v2-selected-run-list result-tab-${resultState.selectedTab} table-density-${resultState.tableDensity}`}>{selectedRun ? [selectedRun].map((run) => <article key={run.id} className="run-row researcher-result-card results-v2-selected-run-card">
          <div className="run-icon"><FlaskConical size={18} /></div>
          <div className="run-content"><strong>{run.name}</strong><p>{new Date(run.createdAt).toLocaleString()} | seed {run.seed} | fingerprint {run.fingerprint}</p><span><AlertTriangle size={13} />{scopeCopy(run.warnings[0])}</span>
            {run.result ? <RunResultSections run={run} tab={resultState.selectedTab} focusPath={focusPath} activePath={activePath} comparisonRuns={selectedComparisonRuns} allRuns={runs} nodes={nodes} edges={edges} /> : <SectionEmpty title="No result payload" detail="This saved run does not contain a completed result payload." />}
          </div>
          <div className="run-status"><StatusBadge status="validated">Scope checked</StatusBadge></div>
        </article>) : null}</div>
      </main>
      {selectedRun && selectedInterpretation ? <ResultsV228DetailPane run={selectedRun} tab={resultState.selectedTab} interpretation={selectedInterpretation} bestR2={bestR2} warningCount={significantWarningCount} onOpenTab={(tab) => setResultState({ selectedTab: tab })} onPrepareReport={() => setView("reports")} /> : null}
    </div>
    {selectedRun ? <ResultsV228ProvenanceFooter run={selectedRun} warningCount={significantWarningCount} /> : null}
    {visibleRuns.length === 0 ? <EmptyState title="No matching runs" description="Clear the search field or include a broader result section." /> : null}
  </WorkspacePage>;
}

function ResultMenu({ label, name, open, onOpen, children }: { label: string; name: "view" | "table" | "export" | "interpretation"; open: null | "view" | "table" | "export" | "interpretation"; onOpen: (name: null | "view" | "table" | "export" | "interpretation") => void; children: ReactNode }) {
  const expanded = open === name;
  return <div className="results-menu">
    <button type="button" className="secondary-button results-menu-trigger" aria-expanded={expanded} onClick={() => onOpen(expanded ? null : name)}>{label}<ChevronDown size={14} /></button>
    {expanded ? <div className="results-menu-panel" role="menu" aria-label={`${label} result controls`}>{children}</div> : null}
  </div>;
}

function ResultsV2LensPanel({ run, tab, interpretation, bestR2, warningCount }: { run: AnalysisRun; tab: ResultWorkspaceTab; interpretation: ResultInterpretation; bestR2: [string, number] | null | undefined; warningCount: number }) {
  const result = run.result;
  const tabFindings = findingsForTab(interpretation, tab);
  const must = tabFindings.filter((finding) => finding.severity === "issue").length;
  const review = tabFindings.filter((finding) => finding.severity === "caution" || finding.severity === "unavailable").length;
  const summary = resultsTabSummary(tab, run, interpretation, bestR2, warningCount);
  return <section className="results-v2-lens-panel" aria-label={`${resultTabLabel(tab)} result lens`}>
    <div className="results-v2-lens-copy">
      <span>{resultTabLabel(tab)}</span>
      <strong>{summary.question}</strong>
      <p>{summary.detail}</p>
    </div>
    <div className="results-v2-lens-metrics" aria-label="Current tab evidence summary">
      <article><span>Evidence</span><strong>{summary.evidence}</strong><small>{summary.evidenceDetail}</small></article>
      <article className={must ? "issue" : review ? "warning" : "validated"}><span>Findings</span><strong>{must ? `${must} must address` : review ? `${review} review` : "Clear"}</strong><small>{tabFindings.length} value-specific item(s)</small></article>
      <article><span>Report path</span><strong>{summary.reportAction}</strong><small>{result ? "Uses selected run values" : "Run required"}</small></article>
    </div>
  </section>;
}

function ResultsV228DetailPane({ run, tab, interpretation, bestR2, warningCount, onOpenTab, onPrepareReport }: { run: AnalysisRun; tab: ResultWorkspaceTab; interpretation: ResultInterpretation; bestR2: [string, number] | null | undefined; warningCount: number; onOpenTab: (tab: ResultWorkspaceTab) => void; onPrepareReport: () => void }) {
  const grouped = findingsByGroup(interpretation.findings);
  const tabFindings = findingsForTab(interpretation, tab);
  const topFindings = [
    ...grouped.must.slice(0, 2),
    ...grouped.recommended.slice(0, 2),
    ...grouped.optional.slice(0, 1),
  ].slice(0, 5);
  return <aside className="results-v228-detail-pane" aria-label="Result interpretation and method confidence">
    <section className="results-v228-pane-section">
      <div className="results-v228-pane-title">
        <span>Method confidence</span>
        <StatusBadge status="validated">Scope checked</StatusBadge>
      </div>
      <dl className="results-v228-confidence-grid">
        <div><dt>Method</dt><dd>{run.method}</dd></div>
        <div><dt>Observations</dt><dd>{run.result?.used_observations ?? "N/A"}</dd></div>
        <div><dt>Seed</dt><dd>{run.seed}</dd></div>
        <div><dt>Fingerprint</dt><dd>{run.fingerprint.slice(0, 10)}</dd></div>
      </dl>
      <details>
        <summary>Open full evidence</summary>
        <MethodConfidencePanel run={run} />
      </details>
    </section>
    <section className="results-v228-pane-section">
      <div className="results-v228-pane-title">
        <span>{resultTabLabel(tab)} detail</span>
        <small>{tabFindings.length} finding(s)</small>
      </div>
      <p>{resultsTabSummary(tab, run, interpretation, bestR2, warningCount).detail}</p>
      <div className="results-v228-tab-actions">
        <button type="button" className="secondary-button" onClick={() => onOpenTab("interpretation")}>Open checklist</button>
        <button type="button" className="secondary-button" onClick={onPrepareReport}>Prepare report</button>
      </div>
    </section>
    <section className="results-v228-pane-section">
      <div className="results-v228-pane-title">
        <span>Findings lanes</span>
        <small>Must address / Review / Info</small>
      </div>
      <ResultsV228FindingLane title="Must address" findings={grouped.must} tone="issue" onOpenTab={onOpenTab} />
      <ResultsV228FindingLane title="Review" findings={grouped.recommended} tone="warning" onOpenTab={onOpenTab} />
      <ResultsV228FindingLane title="Info" findings={grouped.optional.length ? grouped.optional : topFindings.filter((finding) => finding.severity === "good" || finding.severity === "info")} tone="info" onOpenTab={onOpenTab} />
    </section>
  </aside>;
}

function ResultsV228FindingLane({ title, findings, tone, onOpenTab }: { title: string; findings: InterpretationFinding[]; tone: "issue" | "warning" | "info"; onOpenTab: (tab: ResultWorkspaceTab) => void }) {
  const visible = findings.slice(0, 3);
  return <div className={`results-v228-finding-lane ${tone}`}>
    <div className="results-v228-lane-heading"><strong>{title}</strong><span>{findings.length}</span></div>
    {visible.length ? visible.map((finding) => <button key={finding.id} type="button" className="results-v228-finding-chip" onClick={() => onOpenTab(finding.tab)}>
      <span>{finding.metric}</span>
      <strong>{finding.value}</strong>
      <small>{finding.recommendedAction}</small>
    </button>) : <p>No current items.</p>}
    {findings.length > visible.length ? <small>{findings.length - visible.length} more in Interpretation.</small> : null}
  </div>;
}

function ResultsV228ProvenanceFooter({ run, warningCount }: { run: AnalysisRun; warningCount: number }) {
  return <footer className="results-v228-provenance-footer" aria-label="Selected run provenance">
    <span>Run {run.id}</span>
    <span>{new Date(run.createdAt).toLocaleString()}</span>
    <span>seed {run.seed}</span>
    <span>fingerprint {run.fingerprint}</span>
    <span>{warningCount ? `${warningCount} warning(s)` : "no extra warnings"}</span>
  </footer>;
}

function resultsTabSummary(tab: ResultWorkspaceTab, run: AnalysisRun, interpretation: ResultInterpretation, bestR2: [string, number] | null | undefined, warningCount: number) {
  const result = run.result;
  const assessment = run.assessment;
  const base = {
    question: "What should I report from this run?",
    detail: "Use the tab-specific tables and findings below before moving to the publication report.",
    evidence: result ? `${result.used_observations} observations` : "No result",
    evidenceDetail: `Seed ${run.seed}`,
    reportAction: "Copy wording",
  };
  if (!result) return base;
  switch (tab) {
    case "overview":
      return { question: "Is this run ready to interpret?", detail: "Start here for the highest-priority findings, reportability checklist, path estimates, effects, and provenance context.", evidence: `${result.paths.length} paths`, evidenceDetail: bestR2 ? `Strongest R²: ${bestR2[0]} ${bestR2[1].toFixed(4)}` : "No endogenous R²", reportAction: warningCount ? "Review warnings" : "Prepare report" };
    case "measurement":
      return { question: "Are the indicators representing constructs clearly?", detail: "Review loadings, weights, formative VIF, cross-loadings, and indicator-level guidance before emphasizing structural results.", evidence: `${result.outer_estimates.length} indicators`, evidenceDetail: `${new Set(result.outer_estimates.map((row) => row.construct)).size} constructs`, reportAction: "Report measurement" };
    case "structural":
      return { question: "Which relationships and explanatory power matter?", detail: "Review path coefficients, total effects, R², f², VIF, and mediation or moderation rows where available.", evidence: `${result.paths.length} direct paths`, evidenceDetail: `${Object.keys(result.r_squared).length} endogenous construct(s)`, reportAction: "Report structure" };
    case "validity":
      return { question: "Can constructs be interpreted as reliable and distinct?", detail: "Use reliability, AVE, Fornell-Larcker, HTMT, and cross-loading checks as methodological guidance rather than automatic pass/fail law.", evidence: `${assessment?.construct_quality.length ?? 0} constructs`, evidenceDetail: assessment?.htmt_plus ? "HTMT+ available" : "HTMT unavailable", reportAction: "Report validity" };
    case "inference":
      return { question: "Can I make p-value or confidence-interval claims?", detail: "Inference is available only when bootstrap or permutation was run. Estimate-only runs should not be reported as significance evidence.", evidence: run.bootstrap ? `${run.bootstrap.usable_replicates} bootstrap` : run.permutation ? `${run.permutation.plan.permutations} permutations` : "Not run", evidenceDetail: run.bootstrap || run.permutation ? "Resampling payload available" : "Enable in Setup and rerun", reportAction: run.bootstrap || run.permutation ? "Report inference" : "Rerun with inference" };
    case "prediction":
      return { question: "Did the model demonstrate predictive usefulness?", detail: "Prediction output is separate from explanatory fit; compare holdout or blindfolding evidence against the intended research objective.", evidence: result.predict ? "PLSpredict available" : assessment?.blindfolding ? "Q² available" : "Not run", evidenceDetail: result.predict?.targets.length ? `${result.predict.targets.length} target(s)` : "Configure prediction first", reportAction: result.predict || assessment?.blindfolding ? "Report prediction" : "Configure prediction" };
    case "groups":
      return { question: "Are group or segmentation results present and defensible?", detail: "Group workflows require method-specific prerequisites such as MICOM, group sizes, or segment recovery diagnostics.", evidence: result.mga || result.micom || result.mga_permutation || result.fimix || result.segmentation || result.ipma ? "Payload present" : "No payload", evidenceDetail: "Use Setup for group workflows", reportAction: "Report groups" };
    case "diagnostics":
      return { question: "What warnings, provenance, and method details must remain visible?", detail: "Use diagnostics to confirm recipe provenance, warnings, convergence, and method-specific payload notes before export.", evidence: `${warningCount} warning(s)`, evidenceDetail: `Fingerprint ${run.fingerprint.slice(0, 10)}`, reportAction: "Review provenance" };
    case "interpretation":
      return { question: "What should be addressed before reporting?", detail: "This checklist prioritizes exact value-driven issues, recommended checks, optional checks, and reusable report wording.", evidence: `${interpretation.findings.length} findings`, evidenceDetail: `${interpretation.reportParagraphs.length} report paragraph(s)`, reportAction: "Copy guidance" };
    case "comparison":
      return { question: "How do two compatible runs differ?", detail: "Compare path, R², and measurement deltas only for compatible completed runs; cross-family comparison remains out of scope here.", evidence: "Two-run scope", evidenceDetail: "Select comparable runs", reportAction: "Export deltas" };
    default:
      return base;
  }
}

function RunResultSections({ run, tab, focusPath, activePath, comparisonRuns, allRuns, nodes, edges }: { run: AnalysisRun; tab: ResultWorkspaceTab; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null; comparisonRuns: AnalysisRun[]; allRuns: AnalysisRun[]; nodes: SemDiagramNodeLike[]; edges: SemDiagramEdgeLike[] }) {
  const result = run.result;
  if (!result) return null;
  const interpretation = buildResultInterpretation({ run, nodes, edges });
  if (tab === "overview") return <SummaryResults run={run} focusPath={focusPath} activePath={activePath} interpretation={interpretation} />;
  if (tab === "measurement") return <MeasurementResults result={result} assessment={run.assessment} focusPath={focusPath} activePath={activePath} interpretation={interpretation} />;
  if (tab === "structural") return <StructuralResults run={run} focusPath={focusPath} activePath={activePath} interpretation={interpretation} />;
  if (tab === "validity") return <QualityResults assessment={run.assessment} interpretation={interpretation} />;
  if (tab === "inference") return <InferenceResults run={run} interpretation={interpretation} />;
  if (tab === "prediction") return <PredictionResults result={result} assessment={run.assessment} interpretation={interpretation} />;
  if (tab === "groups") return <GroupResults result={result} interpretation={interpretation} />;
  if (tab === "diagnostics") return <DiagnosticsResults run={run} interpretation={interpretation} />;
  if (tab === "interpretation") return <InterpretationResults run={run} interpretation={interpretation} />;
  return <ComparisonResults selectedRuns={comparisonRuns} allRuns={allRuns} />;
}

function SummaryResults({ run, focusPath, activePath, interpretation }: { run: AnalysisRun; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null; interpretation: ResultInterpretation }) {
  const result = run.result!;
  const warningCount = [...run.warnings, ...result.warnings].filter((warning) => !warning.toLowerCase().includes("validated")).length;
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const checklist = reportabilityItems(run, interpretation);
  return <div className="result-sections result-summary" tabIndex={0} role="region" aria-label={`${run.name} result summary`}>
    <FindingCards findings={findingsForTab(interpretation, "overview")} title="Run-specific findings" onFocusPath={focusPath} />
    <ReportabilityChecklist items={checklist} onSelect={(item) => setResultState({ selectedTab: reportabilityTargetTab(item.id) })} />
    <ReportabilityAssistantPanel run={run} interpretation={interpretation} items={checklist} onOpenTab={(tab) => setResultState({ selectedTab: tab })} />
    <div className="result-kpi-row">
      <MetricTile label="Iterations" value={String(result.iterations)} detail={result.converged ? "converged" : "not converged"} tone={result.converged ? "ok" : "warn"} />
      <MetricTile label="Observations" value={String(result.used_observations)} detail={result.omitted_observations ? `${result.omitted_observations} omitted` : "complete cases used"} />
      {Object.entries(result.r_squared).map(([construct, value]) => <MetricTile key={construct} label={`R² ${construct}`} value={value.toFixed(4)} detail={interpretR2(value)} tone={value >= 0.75 ? "ok" : value >= 0.25 ? "neutral" : "warn"} />)}
      <MetricTile label="Warnings" value={String(warningCount)} detail={warningCount ? "review diagnostics" : "none beyond scope status"} tone={warningCount ? "warn" : "ok"} />
    </div>
    <SectionTable title="Path coefficients" note="Click a path row to focus the related edge in the SEM diagram." columns={["Path", "Coefficient", "Direction"]} rows={result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toFixed(6), coefficientDirection(path.coefficient)])} activeRowIndexes={activeIndexes(result.paths, activePath)} onRowClick={(_, index) => focusPath(result.paths[index].source, result.paths[index].target)} guidance={interpretationRegistry.paths} />
    <EffectsTable result={result} activePath={activePath} />
    {result.mediation?.estimates.length ? <MediationTable run={run} /> : null}
    {result.moderation?.estimates.length ? <ModerationTable run={run} /> : null}
  </div>;
}

function reportabilityItems(run: AnalysisRun, interpretation: ResultInterpretation): ReportabilityItem[] {
  const result = run.result;
  const assessment = run.assessment;
  if (!result) {
    return [{ id: "result", label: "Completed result", status: "unavailable", evidence: "No result payload is available.", action: "Rerun the selected method." }];
  }

  const lowLoadings = result.outer_estimates.filter((row) => Math.abs(row.loading) < 0.4);
  const reviewLoadings = result.outer_estimates.filter((row) => Math.abs(row.loading) >= 0.4 && Math.abs(row.loading) < 0.708);
  const reliabilityIssues = assessment?.construct_quality.filter((row) => (row.cronbach_alpha != null && row.cronbach_alpha < 0.7) || (row.rho_c != null && row.rho_c < 0.7)) ?? [];
  const aveIssues = assessment?.construct_quality.filter((row) => row.ave != null && row.ave < 0.5) ?? [];
  const htmtIssueCount = assessment?.htmt_plus?.cells.flat().filter((cell) => (cell.value ?? 0) >= 0.9).length ?? 0;
  const htmtReviewCount = assessment?.htmt_plus?.cells.flat().filter((cell) => (cell.value ?? 0) >= 0.85 && (cell.value ?? 0) < 0.9).length ?? 0;
  const highVif = assessment?.structural_vif.filter((row) => (row.vif ?? 0) >= 5).length ?? 0;
  const reviewVif = assessment?.structural_vif.filter((row) => (row.vif ?? 0) >= 3.3 && (row.vif ?? 0) < 5).length ?? 0;
  const hasInference = Boolean(run.bootstrap || run.permutation);
  const warnings = [...run.warnings, ...result.warnings].filter((warning) => !warning.toLowerCase().includes("validated"));
  const shapeRecommendations = interpretation.diagramAdvice;

  return [
    {
      id: "indicator_reliability",
      label: "Indicator reliability",
      status: lowLoadings.length ? "issue" : reviewLoadings.length ? "review" : result.outer_estimates.length ? "ready" : "unavailable",
      evidence: lowLoadings.length ? `${lowLoadings.length} loading(s) below .40.` : reviewLoadings.length ? `${reviewLoadings.length} loading(s) between .40 and .708.` : `${result.outer_estimates.length} indicator estimate(s) available.`,
      action: lowLoadings.length || reviewLoadings.length ? "Inspect item wording and theoretical justification." : undefined,
    },
    {
      id: "internal_consistency",
      label: "Internal consistency",
      status: !assessment ? "unavailable" : reliabilityIssues.length ? "review" : "ready",
      evidence: !assessment ? "Assessment payload is unavailable." : reliabilityIssues.length ? `${reliabilityIssues.length} construct(s) below common reliability guidance.` : "Alpha/rho_C checks are available.",
      action: reliabilityIssues.length ? "Review reliability before reporting acceptance." : undefined,
    },
    {
      id: "convergent_validity",
      label: "Convergent validity",
      status: !assessment ? "unavailable" : aveIssues.length ? "issue" : "ready",
      evidence: !assessment ? "Assessment payload is unavailable." : aveIssues.length ? `${aveIssues.length} construct(s) with AVE below .50.` : "AVE values meet common guidance.",
      action: aveIssues.length ? "Review indicators and construct definition." : undefined,
    },
    {
      id: "discriminant_validity",
      label: "Discriminant validity",
      status: !assessment ? "unavailable" : htmtIssueCount ? "issue" : htmtReviewCount ? "review" : "ready",
      evidence: !assessment ? "HTMT/Fornell-Larcker not available." : htmtIssueCount ? `${htmtIssueCount} HTMT+ cell(s) at or above .90.` : htmtReviewCount ? `${htmtReviewCount} HTMT+ cell(s) between .85 and .90.` : "Discriminant validity tables are available.",
      action: htmtIssueCount || htmtReviewCount ? "Inspect construct overlap, theory, and item wording." : undefined,
    },
    {
      id: "collinearity",
      label: "Collinearity",
      status: !assessment ? "unavailable" : highVif ? "issue" : reviewVif ? "review" : "ready",
      evidence: !assessment ? "VIF values are unavailable." : highVif ? `${highVif} VIF value(s) at or above 5.` : reviewVif ? `${reviewVif} VIF value(s) at or above 3.3.` : "Structural VIF checks are within common guidance.",
      action: highVif || reviewVif ? "Review overlapping predictors before interpreting paths." : undefined,
    },
    {
      id: "structural_paths",
      label: "Structural paths",
      status: result.paths.length ? (hasInference ? "ready" : "review") : "not applicable",
      evidence: result.paths.length ? `${result.paths.length} path coefficient(s); ${hasInference ? "inference available" : "estimate-only without resampling"}.` : "No structural paths in this model.",
      action: hasInference ? undefined : "Enable bootstrap or permutation before significance claims.",
    },
    {
      id: "r_squared",
      label: "R² / adjusted R²",
      status: Object.keys(result.r_squared).length ? "ready" : "not applicable",
      evidence: Object.keys(result.r_squared).length ? `${Object.keys(result.r_squared).length} endogenous construct(s) with R².` : "No endogenous construct with R².",
    },
    {
      id: "f_squared",
      label: "f² effect sizes",
      status: !assessment ? "unavailable" : assessment.f_squared.length ? "ready" : "not applicable",
      evidence: !assessment ? "f² table is unavailable." : assessment.f_squared.length ? `${assessment.f_squared.length} effect-size row(s) available.` : "No f² rows for this run.",
    },
    {
      id: "prediction",
      label: "Q² / prediction",
      status: result.predict || assessment?.blindfolding ? "ready" : "unavailable",
      evidence: result.predict ? "PLSpredict output is available." : assessment?.blindfolding ? "Blindfolding Q² output is available." : "Prediction outputs were not run.",
      action: result.predict || assessment?.blindfolding ? undefined : "Run PLSpredict or blindfolding when prediction is a research objective.",
    },
    {
      id: "conditional_effects",
      label: "Mediation / moderation / groups",
      status: result.mediation?.estimates.length || result.moderation?.estimates.length || result.mga || result.micom ? "ready" : shapeRecommendations.length ? "review" : "not applicable",
      evidence: result.mediation?.estimates.length ? `${result.mediation.estimates.length} mediation effect row(s).` : result.moderation?.estimates.length ? `${result.moderation.estimates.length} moderation row(s).` : result.mga || result.micom ? "Group-analysis payload available." : shapeRecommendations.length ? `${shapeRecommendations.length} diagram-based recommendation(s).` : "No conditional or group payload.",
      action: shapeRecommendations.length ? "Use the advisor recommendations if they match your research design." : undefined,
    },
    {
      id: "inference",
      label: "Inference availability",
      status: hasInference ? "ready" : "unavailable",
      evidence: hasInference ? `${run.bootstrap ? "Bootstrap" : ""}${run.bootstrap && run.permutation ? " and " : ""}${run.permutation ? "permutation" : ""} output available.` : "No bootstrap or permutation output.",
      action: hasInference ? undefined : "Do not report p values or confidence intervals from this run.",
    },
    {
      id: "warnings",
      label: "Warnings and provenance",
      status: warnings.length ? "review" : "ready",
      evidence: warnings.length ? `${warnings.length} warning(s) beyond scope status.` : `Seed ${run.seed}, fingerprint ${run.fingerprint}.`,
      action: warnings.length ? "Review Diagnostics before reporting." : undefined,
    },
  ];
}

function reportabilityTargetTab(id: string): ResultWorkspaceTab {
  if (id === "indicator_reliability" || id === "internal_consistency") return "measurement";
  if (id === "convergent_validity" || id === "discriminant_validity") return "validity";
  if (id === "collinearity" || id === "structural_paths" || id === "r_squared" || id === "f_squared") return "structural";
  if (id === "prediction") return "prediction";
  if (id === "conditional_effects") return "groups";
  if (id === "inference") return "inference";
  if (id === "warnings") return "diagnostics";
  return "overview";
}

function ReportabilityAssistantPanel({ run, interpretation, items, onOpenTab }: { run: AnalysisRun; interpretation: ResultInterpretation; items: ReportabilityItem[]; onOpenTab: (tab: ResultWorkspaceTab) => void }) {
  const lanes = [
    { id: "issue", label: "Must address", statuses: ["issue"] },
    { id: "review", label: "Review before reporting", statuses: ["review"] },
    { id: "ready", label: "Ready evidence", statuses: ["ready"] },
    { id: "unavailable", label: "Unavailable / not applicable", statuses: ["unavailable", "not applicable"] },
  ];
  const reportSnippets = interpretation.reportParagraphs.slice(0, 5);
  const copySnippets = async () => {
    const text = reportSnippets.map((row) => `${row.section}: ${row.text}`).join("\n\n");
    await navigator.clipboard?.writeText(text);
  };
  return <section className="reportability-assistant" data-v230-reportability-assistant="true" aria-label="Interpretation and reportability assistant">
    <header>
      <div>
        <span>Reportability assistant</span>
        <strong>What should be reported, reviewed, or withheld for this run?</strong>
        <p>These checks use the selected run values, diagram shape, and availability of inference. Threshold colors are guidance, not universal pass/fail rules.</p>
      </div>
      <button type="button" className="secondary-button" onClick={copySnippets} disabled={!reportSnippets.length}>Copy report snippets</button>
    </header>
    <div className="reportability-assistant-grid">
      {lanes.map((lane) => {
        const laneItems = items.filter((item) => lane.statuses.includes(item.status));
        return <article key={lane.id} className={`reportability-lane ${lane.id}`}>
          <div className="reportability-lane-heading">
            <strong>{lane.label}</strong>
            <span>{laneItems.length}</span>
          </div>
          {laneItems.length ? laneItems.map((item) => <ReportabilityAssistantItem key={item.id} item={item} run={run} interpretation={interpretation} onOpenTab={onOpenTab} />) : <p className="muted-copy">No items in this lane for the selected run.</p>}
        </article>;
      })}
    </div>
    <div className="reportability-report-snippets" data-v230-report-snippets="true">
      <strong>Report wording from this run</strong>
      {reportSnippets.length ? reportSnippets.map((row) => <blockquote key={row.section}><b>{row.section}</b><span>{row.text}</span></blockquote>) : <p>No report wording is available for this run.</p>}
    </div>
  </section>;
}

function ReportabilityAssistantItem({ item, run, interpretation, onOpenTab }: { item: ReportabilityItem; run: AnalysisRun; interpretation: ResultInterpretation; onOpenTab: (tab: ResultWorkspaceTab) => void }) {
  const tab = reportabilityTargetTab(item.id);
  const finding = matchingFindingForReportability(item.id, interpretation);
  const reportSentence = finding?.reportSentence ?? reportSentenceForReportability(item, run);
  return <section className={`reportability-assistant-item ${item.status}`} data-v230-reportability-item={item.id}>
    <div className="reportability-item-title">
      <span>{item.status}</span>
      <strong>{item.label}</strong>
    </div>
    <dl>
      <div><dt>What the value says</dt><dd>{item.evidence}</dd></div>
      <div><dt>Why it matters</dt><dd>{finding?.interpretation ?? reportabilityWhyItMatters(item.id)}</dd></div>
      <div><dt>What to inspect next</dt><dd>{item.action ?? finding?.recommendedAction ?? "Keep the evidence with the selected run provenance and interpret it in context."}</dd></div>
      <div><dt>Report wording</dt><dd>{reportSentence}</dd></div>
    </dl>
    <button type="button" onClick={() => onOpenTab(tab)}>Open {resultTabLabel(tab)}</button>
  </section>;
}

function matchingFindingForReportability(id: string, interpretation: ResultInterpretation) {
  const tab = reportabilityTargetTab(id);
  const candidates = interpretation.findings.filter((finding) => finding.tab === tab);
  if (id === "indicator_reliability") return candidates.find((finding) => /loading/i.test(finding.metric));
  if (id === "internal_consistency") return candidates.find((finding) => /alpha|rho|reliability/i.test(finding.metric));
  if (id === "convergent_validity") return candidates.find((finding) => /AVE/i.test(finding.metric));
  if (id === "discriminant_validity") return candidates.find((finding) => /HTMT|Fornell|cross/i.test(finding.metric));
  if (id === "collinearity") return candidates.find((finding) => /VIF/i.test(finding.metric));
  if (id === "structural_paths") return candidates.find((finding) => /Path coefficient/i.test(finding.metric));
  if (id === "r_squared") return candidates.find((finding) => /R²|R2/i.test(finding.metric));
  if (id === "f_squared") return candidates.find((finding) => /f²|f2/i.test(finding.metric));
  if (id === "prediction") return candidates.find((finding) => /Q²|prediction|PLSpredict/i.test(finding.metric));
  if (id === "conditional_effects") return candidates.find((finding) => /mediation|moderation|group|shape/i.test(finding.metric));
  if (id === "inference") return candidates.find((finding) => /bootstrap|permutation|inference/i.test(finding.metric));
  if (id === "warnings") return candidates.find((finding) => /warning|provenance|scope/i.test(finding.metric));
  return candidates[0];
}

function reportabilityWhyItMatters(id: string) {
  switch (id) {
    case "indicator_reliability": return "Indicator quality affects construct scores and every downstream structural estimate.";
    case "internal_consistency": return "Reliability evidence supports whether indicators are coherently measuring the construct.";
    case "convergent_validity": return "AVE summarizes whether the construct explains enough indicator variance for common reflective-model reporting.";
    case "discriminant_validity": return "High discriminant-validity values can mean constructs are not empirically distinct.";
    case "collinearity": return "Collinearity can destabilize path estimates and distort interpretation of predictor importance.";
    case "structural_paths": return "Path coefficients describe modeled relationships, but inference is needed before reporting support.";
    case "r_squared": return "R² describes explained variance for endogenous constructs and anchors structural model interpretation.";
    case "f_squared": return "f² helps judge each predictor's contribution beyond the coefficient alone.";
    case "prediction": return "Prediction evidence matters when the study claims out-of-sample or predictive usefulness.";
    case "conditional_effects": return "Mediation, moderation, and group results should only be interpreted when the diagram and settings support them.";
    case "inference": return "p values and confidence intervals require resampling or permutation output.";
    case "warnings": return "Warnings and provenance define the boundary for defensible reporting.";
    default: return "This item affects how confidently the selected run can be reported.";
  }
}

function reportSentenceForReportability(item: ReportabilityItem, run: AnalysisRun) {
  if (item.status === "ready") return `${item.label} was available for ${run.name}; report the relevant values with the run scope and provenance.`;
  if (item.status === "review" || item.status === "issue") return `${item.label} requires review because ${item.evidence}`;
  if (item.status === "unavailable") return `${item.label} was unavailable for this run; do not report it as completed.`;
  return `${item.label} was not applicable to the selected run.`;
}

function MeasurementResults({ result, assessment, focusPath, activePath, interpretation }: { result: PlsResult; assessment?: AssessmentResult; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null; interpretation: ResultInterpretation }) {
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Measurement model results">
    <FindingCards findings={findingsForTab(interpretation, "measurement")} title="Measurement findings" onFocusPath={focusPath} />
    <SectionTable title="Outer loadings and weights" note="Reflective constructs are usually interpreted through loadings; formative constructs require weights and collinearity diagnostics." columns={["Construct", "Indicator", "Loading", "Weight", "Loading status"]} rows={result.outer_estimates.map((row) => [row.construct, row.indicator, row.loading.toFixed(6), row.weight.toFixed(6), loadingStatus(row.loading)])} guidance={interpretationRegistry.loadings} />
    {assessment?.formative_indicator_vif.length ? <SectionTable title="Outer VIF for formative indicators" note="Use VIF to screen formative indicator collinearity." columns={["Construct", "Indicator", "VIF", "Status"]} rows={assessment.formative_indicator_vif.map((row) => [row.construct, row.indicator, formatOptional(row.vif, 4), vifStatus(row.vif)])} /> : null}
    {assessment?.cross_loadings.length ? <SectionTable title="Cross-loadings" note="Each indicator should usually load highest on its assigned construct." columns={["Indicator", "Assigned construct", "Compared construct", "Loading"]} rows={assessment.cross_loadings.map((row) => [row.indicator, row.assigned_construct, row.construct, row.loading.toFixed(6)])} /> : null}
    <SectionTable title="Structural paths for diagram focus" note="This helper keeps measurement review linked to the model canvas." columns={["Path", "Coefficient"]} rows={result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toFixed(6)])} activeRowIndexes={activeIndexes(result.paths, activePath)} onRowClick={(_, index) => focusPath(result.paths[index].source, result.paths[index].target)} />
  </div>;
}

function StructuralResults({ run, focusPath, activePath, interpretation }: { run: AnalysisRun; focusPath: (source: string, target: string) => void; activePath: { source: string; target: string } | null; interpretation: ResultInterpretation }) {
  const result = run.result!;
  const assessment = run.assessment;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Structural model results">
    <FindingCards findings={findingsForTab(interpretation, "structural")} title="Structural findings" onFocusPath={focusPath} />
    <SectionTable title="Path coefficients" note="Bootstrapped t values and p values appear in Inference after bootstrap is enabled." columns={["Path", "Coefficient", "Direction"]} rows={result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toFixed(6), coefficientDirection(path.coefficient)])} activeRowIndexes={activeIndexes(result.paths, activePath)} onRowClick={(_, index) => focusPath(result.paths[index].source, result.paths[index].target)} guidance={interpretationRegistry.paths} />
    <EffectsTable result={result} activePath={activePath} />
    {(result.control_estimates ?? []).length ? <SectionTable title="Control paths" columns={["Control path", "Coefficient"]} rows={result.control_estimates!.map((control) => [pathLabel(control.source, control.target), control.coefficient.toFixed(6)])} /> : null}
    {assessment?.structural_quality.length ? <SectionTable title="R² and adjusted R²" note="Use R² for explained variance and adjusted R² when comparing models with different predictor counts." columns={["Construct", "Predictors", "R²", "Adjusted R²", "Interpretation"]} rows={assessment.structural_quality.map((row) => [row.construct, String(row.predictor_count), row.r_squared.toFixed(4), formatOptional(row.adjusted_r_squared, 4), interpretR2(row.r_squared)])} guidance={interpretationRegistry.structuralQuality} /> : null}
    {assessment?.structural_vif.length ? <SectionTable title="Inner VIF" note="High VIF suggests predictor collinearity in the structural model." columns={["Target", "Predictor", "VIF", "Status"]} rows={assessment.structural_vif.map((row) => [row.target_construct, row.predictor_construct, formatOptional(row.vif, 4), vifStatus(row.vif)])} guidance={interpretationRegistry.structuralQuality} /> : null}
    {assessment?.f_squared.length ? <SectionTable title="Cohen f² effect sizes" note="f² describes how much an omitted predictor changes the target construct R²." columns={["Path", "R² included", "R² excluded", "f²", "Interpretation"]} rows={assessment.f_squared.map((row) => [pathLabel(row.source_construct, row.target_construct), row.included_r_squared.toFixed(4), formatOptional(row.excluded_r_squared, 4), formatOptional(row.f_squared, 4), interpretF2(row.f_squared)])} guidance={interpretationRegistry.structuralQuality} /> : null}
    {result.mediation?.estimates.length ? <MediationTable run={run} /> : null}
    {result.moderation?.estimates.length ? <ModerationTable run={run} /> : null}
  </div>;
}

function QualityResults({ assessment, interpretation }: { assessment?: AssessmentResult; interpretation: ResultInterpretation }) {
  if (!assessment) return <SectionEmpty title="No assessment payload" detail="Run a PLS-SEM method with assessment outputs to review reliability and validity." />;
   return <div className="result-sections quality-summary" tabIndex={0} role="region" aria-label="measurement quality tables">
    <FindingCards findings={findingsForTab(interpretation, "validity")} title="Validity findings" />
    <SectionTable title="Construct reliability and convergent validity" note="Common reporting columns for reflective PLS-SEM measurement model assessment." columns={["Construct", "Cronbach alpha", "rho_A", "rho_C", "AVE", "Quick check"]} rows={assessment.construct_quality.map((quality) => [
      quality.construct,
      formatOptional(quality.cronbach_alpha, 4),
      quality.rho_a == null ? formatDiagnosticCode(quality.rho_a_reason ?? "N/A") : quality.rho_a.toFixed(4),
      formatOptional(quality.rho_c, 4),
      formatOptional(quality.ave, 4),
      reliabilityStatus(quality.cronbach_alpha, quality.rho_c, quality.ave),
    ])} guidance={interpretationRegistry.reliability} />
    <MatrixTable title="Fornell-Larcker criterion" note="Diagonal values should be read against construct correlations according to the documented QuickPLS convention." constructs={assessment.fornell_larcker.constructs} values={assessment.fornell_larcker.values} guidance={interpretationRegistry.discriminant} />
    {assessment.htmt_plus && <HtmtTable label="HTMT+" artifact={assessment.htmt_plus} />}
    {assessment.htmt_original && <HtmtTable label="Original HTMT" artifact={assessment.htmt_original} />}
    {assessment.htmt && !assessment.htmt_plus && <MatrixTable title="HTMT+ (legacy)" constructs={assessment.htmt.constructs} values={assessment.htmt.values} guidance={interpretationRegistry.discriminant} />}
    {assessment.cross_loadings.length ? <SectionTable title="Cross-loadings check" note="Confirm each indicator is strongest on its assigned construct." columns={["Indicator", "Assigned construct", "Compared construct", "Loading"]} rows={assessment.cross_loadings.map((row) => [row.indicator, row.assigned_construct, row.construct, row.loading.toFixed(6)])} /> : null}
  </div>;
}

function InferenceResults({ run, interpretation }: { run: AnalysisRun; interpretation: ResultInterpretation }) {
  if (!run.bootstrap && !run.permutation) return <div className="result-sections" tabIndex={0} role="region" aria-label="Inference results">
    <FindingCards findings={findingsForTab(interpretation, "inference")} title="Inference findings" />
    <SectionEmpty title="Inference not run" detail="Enable bootstrap or permutation in Setup, rerun the model, then return here for t values, p values, and confidence intervals." />
  </div>;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Inference results">
    <FindingCards findings={findingsForTab(interpretation, "inference")} title="Inference findings" />
    {run.bootstrap ? <BootstrapSection run={run} /> : null}
    {run.permutation ? <PermutationSection run={run} /> : null}
  </div>;
}

function PredictionResults({ result, assessment, interpretation }: { result: PlsResult; assessment?: AssessmentResult; interpretation: ResultInterpretation }) {
  const hasPredict = Boolean(result.predict);
  const hasBlindfolding = Boolean(assessment?.blindfolding);
  if (!hasPredict && !hasBlindfolding) return <div className="result-sections" tabIndex={0} role="region" aria-label="Prediction results">
    <FindingCards findings={findingsForTab(interpretation, "prediction")} title="Prediction findings" />
    <SectionEmpty title="Prediction outputs not run" detail="Enable PLSpredict or blindfolding-related prediction settings, rerun the model, then review holdout metrics and Q² here." />
  </div>;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Prediction results">
    <FindingCards findings={findingsForTab(interpretation, "prediction")} title="Prediction findings" />
    {result.predict ? <><strong className="result-section-heading">PLSpredict holdout</strong><MethodWarnings warnings={result.predict.warnings} /><PlsPredictTable targets={result.predict.targets} />{result.predict.repeated_kfold ? <><strong className="result-section-heading">Repeated k-fold prediction</strong><MethodWarnings warnings={result.predict.repeated_kfold.warnings} /><PlsPredictTable targets={result.predict.repeated_kfold.targets} />{result.predict.repeated_kfold.cvpat?.length ? <CvpatTable comparisons={result.predict.repeated_kfold.cvpat} /> : null}</> : null}</> : null}
    {assessment?.blindfolding ? <SectionTable title="Blindfolding Q²" note={`Omission distance ${assessment.blindfolding.settings.omission_distance}.`} columns={["Construct", "Q²", "PRESS", "SSO"]} rows={assessment.blindfolding.constructs.map((row) => [row.construct, formatOptional(row.q_squared, 4), formatOptional(row.prediction_error_sum_squares, 6), formatOptional(row.observation_sum_squares, 6)])} /> : null}
  </div>;
}

function GroupResults({ result, interpretation }: { result: PlsResult; interpretation: ResultInterpretation }) {
  if (!result.mga && !result.micom && !result.mga_permutation && !result.fimix && !result.segmentation && !result.ipma) return <div className="result-sections" tabIndex={0} role="region" aria-label="Groups and segmentation results">
    <FindingCards findings={findingsForTab(interpretation, "groups")} title="Groups findings" />
    <SectionEmpty title="No group or segmentation payloads" detail="Configure MICOM/MGA, FIMIX-PLS, PLS-POS, or IPMA in Setup and rerun the model to populate this tab." />
  </div>;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Groups and segmentation results"><FindingCards findings={findingsForTab(interpretation, "groups")} title="Groups findings" /><MethodPayloadSections result={result} /></div>;
}

function DiagnosticsResults({ run, interpretation }: { run: AnalysisRun; interpretation: ResultInterpretation }) {
  const result = run.result!;
  const assessment = run.assessment;
  return <div className="result-sections" tabIndex={0} role="region" aria-label="Diagnostics results">
    <FindingCards findings={findingsForTab(interpretation, "diagnostics")} title="Method-specific findings" />
    <SectionTable title="Run provenance" columns={["Field", "Value"]} rows={[["Method", run.method], ["Created", new Date(run.createdAt).toLocaleString()], ["Seed", String(run.seed)], ["Fingerprint", run.fingerprint], ["Converged", result.converged ? "yes" : "no"], ["Iterations", String(result.iterations)], ["Used observations", String(result.used_observations)], ["Omitted observations", String(result.omitted_observations)]]} />
    <SectionTable title="Warnings and scope status" columns={["Message"]} rows={[...run.warnings, ...result.warnings, ...(assessment?.warnings ?? [])].map((warning) => [scopeCopy(warning)])} />
    {assessment?.model_fit ? <SectionTable title="Correlation-residual fit" note="PLS-SEM approximate fit diagnostics should be interpreted within the documented QuickPLS scope." columns={["Model", "SRMR", "d_ULS"]} rows={[["Saturated", assessment.model_fit.saturated.srmr.toFixed(4), assessment.model_fit.saturated.d_uls.toFixed(6)], ["Estimated", assessment.model_fit.estimated.srmr.toFixed(4), assessment.model_fit.estimated.d_uls.toFixed(6)]]} /> : null}
    {result.plsc || result.wpls || result.cca || result.cta_pls || result.endogeneity || result.nonlinear_effects || result.moderated_mediation || result.cbsem || result.gsca || result.regression || result.nca || result.pca ? <MethodPayloadSections result={result} /> : null}
  </div>;
}

function InterpretationResults({ run, interpretation }: { run: AnalysisRun; interpretation: ResultInterpretation }) {
  const result = run.result!;
  const grouped = findingsByGroup(interpretation.findings);
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const checklist = reportabilityItems(run, interpretation);
  return <div className="result-sections interpretation-workspace" tabIndex={0} role="region" aria-label="Interpretation and report wording">
    <ReportabilityAssistantPanel run={run} interpretation={interpretation} items={checklist} onOpenTab={(tab) => setResultState({ selectedTab: tab })} />
    <FindingChecklist title="Must address before reporting" findings={grouped.must} />
    <FindingChecklist title="Recommended checks" findings={grouped.recommended} />
    <FindingChecklist title="Optional advanced checks" findings={grouped.optional} />
    <SectionTable title="Report wording" note="Value-filled draft wording from this run. Adjust language for theory, sample, and journal requirements." columns={["Section", "Draft wording"]} rows={interpretation.reportParagraphs.map((row) => [row.section, row.text])} />
    <SectionTable title="Result availability map" columns={["Area", "Status"]} rows={[
      ["Measurement", result.outer_estimates.length ? "available" : "not available"],
      ["Validity", run.assessment ? "available" : "not available"],
      ["Inference", run.bootstrap || run.permutation ? "available" : "not run"],
      ["Prediction", result.predict || run.assessment?.blindfolding ? "available" : "not run"],
      ["Groups", result.mga || result.micom || result.mga_permutation || result.fimix || result.segmentation || result.ipma ? "available" : "not run"],
      ["Extended methods", result.cbsem || result.gsca || result.regression || result.nca || result.pca ? "available" : "not run"],
    ]} />
  </div>;
}

function ComparisonResults({ selectedRuns, allRuns }: { selectedRuns: AnalysisRun[]; allRuns: AnalysisRun[] }) {
  const resultState = useWorkspace((state) => state.resultWorkspaceState);
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const completedRuns = allRuns.filter((run) => run.result);
  const chosen = selectedRuns.length >= 2 ? selectedRuns.slice(0, 2) : completedRuns.slice(0, 2);
  const [a, b] = chosen;
  if (!a || !b || !a.result || !b.result) return <SectionEmpty title="Select comparison runs" detail="Choose two completed compatible PLS-family runs to compare path coefficients, R², diagnostics, and export-ready differences." />;
  const compatible = a.method === b.method;
  const fingerprintMatch = a.fingerprint === b.fingerprint;
  const selectedIds = new Set(chosen.map((run) => run.id));
  return <div className="result-sections comparison-workspace" tabIndex={0} role="region" aria-label="Bounded two-run comparison">
    <div className="comparison-selector">
      {completedRuns.map((run) => <label key={run.id}><input type="checkbox" checked={selectedIds.has(run.id)} onChange={() => {
        const next = selectedIds.has(run.id) ? resultState.comparisonRunIds.filter((id) => id !== run.id) : [...resultState.comparisonRunIds, run.id].slice(-2);
        setResultState({ comparisonRunIds: next });
      }} />{run.name}</label>)}
    </div>
    {!compatible ? <ResultGuidance title="Comparison blocked" items={["The selected runs use different method families. This milestone supports bounded comparison for compatible PLS-family runs first."]} /> : null}
    {!fingerprintMatch ? <ResultGuidance title="Model fingerprint differs" items={["Recipe or data fingerprint differs. Deltas are still shown for review, but do not treat them as same-model sensitivity evidence."]} /> : null}
    <SectionTable title="Run metadata comparison" columns={["Field", a.name, b.name]} rows={[["Method", a.method, b.method], ["Created", new Date(a.createdAt).toLocaleString(), new Date(b.createdAt).toLocaleString()], ["Seed", String(a.seed), String(b.seed)], ["Fingerprint", a.fingerprint, b.fingerprint], ["Warnings", String(a.warnings.length + (a.result?.warnings.length ?? 0)), String(b.warnings.length + (b.result?.warnings.length ?? 0))]]} />
    <SectionTable title="Path coefficient deltas" note="Delta is second selected run minus first selected run." columns={["Path", a.name, b.name, "Delta"]} rows={comparisonPathRows(a.result, b.result)} guidance={interpretationRegistry.paths} />
    <SectionTable title="R² deltas" columns={["Construct", a.name, b.name, "Delta"]} rows={comparisonR2Rows(a.result, b.result)} guidance={interpretationRegistry.structuralQuality} />
    {a.assessment && b.assessment ? <SectionTable title="Measurement metric deltas" columns={["Construct", "Metric", a.name, b.name, "Delta"]} rows={comparisonMeasurementRows(a.assessment, b.assessment)} guidance={interpretationRegistry.reliability} /> : <SectionEmpty title="No comparable measurement assessment" detail="Both runs need assessment payloads before reliability/validity deltas can be displayed." />}
  </div>;
}

function BootstrapSection({ run }: { run: AnalysisRun }) {
  const bootstrap = run.bootstrap!;
  const estimateRows = bootstrap.percentile.parameters.map((parameter) => [
    formatParameterIdentity(parameter.parameter),
    parameter.original.toFixed(6),
    parameter.bootstrap_mean.toFixed(6),
    parameter.bias.toFixed(6),
    parameter.standard_error.toFixed(6),
    parameter.t_statistic?.toFixed(4) ?? "N/A",
    formatPValue(parameter.p_value_two_sided),
  ]);
  const percentileRows = bootstrap.percentile.parameters.map((parameter) => [
    formatParameterIdentity(parameter.parameter),
    parameter.lower.toFixed(6),
    parameter.upper.toFixed(6),
    ciZeroStatus(parameter.lower, parameter.upper),
  ]);
  const bcaRows = bootstrap.percentile.parameters.map((parameter) => {
    const bca = bootstrap.bca?.parameters.find((value) => value.parameter === parameter.parameter);
    return [
      formatParameterIdentity(parameter.parameter),
      bca?.lower?.toFixed(6) ?? "N/A",
      bca?.upper?.toFixed(6) ?? "N/A",
      bca?.unavailable_reason ? formatDiagnosticCode(bca.unavailable_reason) : ciZeroStatus(bca?.lower, bca?.upper),
    ];
  });
  const studentizedRows = bootstrap.percentile.parameters.map((parameter) => {
    const studentized = bootstrap.studentized?.parameters.find((value) => value.parameter === parameter.parameter);
    return [
      formatParameterIdentity(parameter.parameter),
      studentized?.lower?.toFixed(6) ?? "N/A",
      studentized?.upper?.toFixed(6) ?? "N/A",
      studentized?.unavailable_reason ? formatDiagnosticCode(studentized.unavailable_reason) : ciZeroStatus(studentized?.lower, studentized?.upper),
    ];
  });
  return <div className="bootstrap-summary" aria-label="bootstrap parameter table">
    <div className="bootstrap-meta"><strong>Bootstrap replicates</strong><span>{bootstrap.usable_replicates} usable</span><span>{bootstrap.failed_replicates.length} failed</span><span>{Math.round(bootstrap.percentile.confidence_level * 100)}% percentile CI</span>{bootstrap.bca && <span>{bootstrap.bca.jackknife_case_count} jackknife cases | BCa CI</span>}{bootstrap.studentized && <span>{bootstrap.studentized.inner_replicates} inner replicates | {bootstrap.studentized.failure ? "bootstrap-t failed" : "bootstrap-t CI"}</span>}</div>
    {bootstrap.studentized?.failure && <div className="inference-failure" role="alert"><strong>Bootstrap-t unavailable</strong><span>{bootstrap.studentized.failure.message}</span></div>}
    <div className="bootstrap-section-grid">
      <SectionTable title="Bootstrap estimates" note="Point estimate, bootstrap mean, bias, standard error, t statistic, and p value stay together." columns={["Parameter", "Original", "Mean", "Bias", "SE", "t", "p"]} rows={estimateRows} guidance={interpretationRegistry.inference} />
      <SectionTable title="Percentile confidence intervals" note="Status describes whether the interval excludes zero; use the full interval in reporting." columns={["Parameter", "Lower", "Upper", "Zero status"]} rows={percentileRows} guidance={interpretationRegistry.inference} />
      <SectionTable title="BCa confidence intervals" note={bootstrap.bca ? "Bias-corrected and accelerated intervals are shown where available." : "BCa intervals are not available for this run."} columns={["Parameter", "Lower", "Upper", "Zero status"]} rows={bcaRows} guidance={interpretationRegistry.inference} />
      <SectionTable title="Bootstrap-t confidence intervals" note={bootstrap.studentized ? "Studentized/bootstrap-t intervals are shown where available." : "Bootstrap-t intervals are not available for this run."} columns={["Parameter", "Lower", "Upper", "Zero status"]} rows={studentizedRows} guidance={interpretationRegistry.inference} />
    </div>
  </div>;
}

function PermutationSection({ run }: { run: AnalysisRun }) {
  const permutation = run.permutation!;
  return <div className="bootstrap-summary">
    <div className="bootstrap-meta"><strong>Freedman-Lane permutation</strong><span>{permutation.plan.permutations} samples</span><span>two-sided finite-sample corrected p-values</span></div>
    <SectionTable title="permutation parameter table" columns={["Path", "Original coefficient", "Exceedances", "p"]} rows={permutation.parameters.map((parameter) => [formatParameterIdentity(parameter.parameter), parameter.original.toFixed(6), `${parameter.exceedances} / ${parameter.permutations}`, formatPValue(parameter.p_value_two_sided)])} />
  </div>;
}

function MediationTable({ run }: { run: AnalysisRun }) {
  const estimates = run.result?.mediation?.estimates ?? [];
  const rows = estimates.map((effect) => {
    const parameter = findBootstrapParameter(run.bootstrap, "indirect_effect", [effect.source, effect.target]);
    const bca = parameter ? findBcaParameter(run.bootstrap, parameter.parameter) : undefined;
    const studentized = parameter ? findStudentizedParameter(run.bootstrap, parameter.parameter) : undefined;
    return {
      effect: pathLabel(effect.source, effect.target),
      direct: effect.direct.toFixed(6),
      indirect: effect.indirect.toFixed(6),
      total: effect.total.toFixed(6),
      indirectP: formatPValue(parameter?.p_value_two_sided),
      percentileCi: formatInterval(parameter?.lower, parameter?.upper),
      bcaCi: formatInterval(bca?.lower, bca?.upper),
      studentizedCi: formatInterval(studentized?.lower, studentized?.upper),
      vaf: effect.variance_accounted_for?.toFixed(4) ?? "N/A",
      classification: formatMediationClass(effect.classification),
    };
  });
  return <div className="mediation-table-stack" aria-label="Mediation effect sections">
    <SectionTable title="Mediation effects summary" note="Direct, indirect, and total effects are separated for readable path interpretation." columns={["Effect", "Direct", "Indirect", "Total"]} rows={rows.map((row) => [row.effect, row.direct, row.indirect, row.total])} />
    <SectionTable title="Mediation inference" note={run.bootstrap ? "Bootstrap inference is shown where the matching indirect-effect parameter exists." : "Bootstrap was not run; p values and confidence intervals are unavailable."} columns={["Effect", "Indirect p", "Percentile CI", "BCa CI", "Bootstrap-t CI"]} rows={rows.map((row) => [row.effect, row.indirectP, row.percentileCi, row.bcaCi, row.studentizedCi])} />
    <SectionTable title="Mediation classification" note="VAF and class are descriptive guides; use inference before claiming mediation support." columns={["Effect", "VAF", "Class"]} rows={rows.map((row) => [row.effect, row.vaf, row.classification])} />
  </div>;
}

function EffectsTable({ result, activePath }: { result: PlsResult; activePath: { source: string; target: string } | null }) {
  return <SectionTable
    title="Total effects"
    note="Direct, indirect, and total effects are separated so mediation and serial path interpretation stays explicit."
    columns={["Effect", "Direct", "Indirect", "Total", "Effect type"]}
    rows={result.effects.map((effect) => [
      pathLabel(effect.source, effect.target),
      effect.direct.toFixed(6),
      effect.indirect.toFixed(6),
      effect.total.toFixed(6),
      effect.indirect === 0 ? "direct only" : effect.direct === 0 ? "indirect only" : "direct and indirect",
    ])}
    activeRowIndexes={activeIndexes(result.effects, activePath)}
  />;
}

function ModerationTable({ run }: { run: AnalysisRun }) {
  const estimates = run.result?.moderation?.estimates ?? [];
  return <SectionTable title="Moderation effects" note={run.bootstrap ? "Bootstrap inference is shown where the product-path parameter exists." : "Bootstrap was not run; p values and confidence intervals are unavailable."} columns={["Interaction", "Main effect", "Interaction", "Interaction p", "Percentile CI", "BCa CI", "Bootstrap-t CI", "Simple slopes"]} rows={estimates.map((effect) => {
    const parameter = findBootstrapParameter(run.bootstrap, "path", [effect.product_construct, effect.outcome]);
    const bca = parameter ? findBcaParameter(run.bootstrap, parameter.parameter) : undefined;
    const studentized = parameter ? findStudentizedParameter(run.bootstrap, parameter.parameter) : undefined;
    return [`${effect.predictor} x ${effect.moderator} -> ${effect.outcome}`, effect.predictor_main_effect?.toFixed(6) ?? "N/A", effect.interaction_effect.toFixed(6), formatPValue(parameter?.p_value_two_sided), formatInterval(parameter?.lower, parameter?.upper), formatInterval(bca?.lower, bca?.upper), formatInterval(studentized?.lower, studentized?.upper), effect.simple_slopes.length ? effect.simple_slopes.map((slope) => `${formatModeratorLevel(slope.moderator_score)}: ${slope.effect.toFixed(6)}`).join(" | ") : "N/A"];
  })} />;
}

type InterpretationTone = "good" | "caution" | "issue" | "informational" | "not_applicable";

interface InterpretationDescriptor {
  metricId: string;
  label: string;
  scopeStatus: "Validated for documented QuickPLS scope" | "Experimental / watermarked" | "Unsupported" | "Not available for this run";
  tone: InterpretationTone;
  interpretation: string;
  thresholds: string;
  why: string;
  report: string;
}

const interpretationRegistry: Record<string, InterpretationDescriptor> = {
  paths: {
    metricId: "pls.paths",
    label: "Path coefficients",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "Use sign, magnitude, confidence intervals, and theory together. A path coefficient alone does not establish substantive importance.",
    thresholds: "No universal cutoff is applied. Bootstrap or permutation evidence is needed before inferential claims.",
    why: "Paths express structural relationships among latent scores in the selected model and should be interpreted within the saved recipe and sample.",
    report: "Report coefficient, inference method, confidence interval or p value when available, sample size, and model scope status.",
  },
  loadings: {
    metricId: "pls.measurement.loadings",
    label: "Outer loadings and weights",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "Reflective indicators are reviewed mainly through loadings; formative indicators require weights plus collinearity and content-validity review.",
    thresholds: "A loading near 0.708 is a common reliability guide; values between 0.40 and 0.708 require theory and reliability context.",
    why: "Measurement quality determines whether construct scores are interpretable before structural paths are emphasized.",
    report: "Report retained indicators, loading/weight range, and any indicators kept despite review flags.",
  },
  reliability: {
    metricId: "pls.validity.reliability",
    label: "Reliability and convergent validity",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "Review alpha, rho_A, rho_C, and AVE together instead of treating any one metric as decisive.",
    thresholds: "Common guides are reliability >= 0.70 and AVE >= 0.50, with stricter interpretation depending on research context.",
    why: "Reliability and convergent validity show whether indicators consistently represent the intended construct.",
    report: "Report alpha, rho_A, composite reliability, AVE, and any two-indicator or estimation warnings.",
  },
  discriminant: {
    metricId: "pls.validity.discriminant",
    label: "Discriminant validity",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "caution",
    interpretation: "High HTMT or Fornell-Larcker conflicts require construct-definition review before claiming distinct constructs.",
    thresholds: "HTMT guides often use 0.85 or 0.90 depending on construct similarity; values above 1 require direct review.",
    why: "Discriminant validity checks whether constructs are empirically separable enough for the structural model.",
    report: "Report the HTMT convention, threshold used, and any construct pairs exceeding the chosen guide.",
  },
  structuralQuality: {
    metricId: "pls.structural.quality",
    label: "Structural quality",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "R², f², VIF, and Q² answer different questions and should be read as a diagnostic set.",
    thresholds: "R² guides are context-dependent; f² guide values are 0.02, 0.15, and 0.35; VIF above 3.3 or 5 should be reviewed.",
    why: "This section separates explanatory power, predictor contribution, collinearity, and predictive relevance.",
    report: "Report R²/adjusted R² for endogenous constructs, VIF range, f² effects, and Q² where available.",
  },
  inference: {
    metricId: "pls.inference",
    label: "Inference",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "Use confidence intervals and p values only for runs where the matching inference procedure was executed.",
    thresholds: "Common alpha levels such as 0.05 are reporting conventions, not automatic evidence of practical importance.",
    why: "Inference is resampling-dependent; missing bootstrap or permutation output means the run supports estimation but not inferential claims.",
    report: "Report resampling type, samples, seed, interval type, p values, failures, and unavailable intervals.",
  },
  prediction: {
    metricId: "pls.prediction",
    label: "Prediction",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "Prediction output should be read against benchmark performance and leakage-safe validation settings.",
    thresholds: "Q² predict above zero and lower PLS error than benchmark are directional guides, not universal success criteria.",
    why: "Predictive relevance is different from explanatory structural fit.",
    report: "Report split or repeated k-fold settings, target metrics, benchmark comparison, and CVPAT results where available.",
  },
  groups: {
    metricId: "pls.groups",
    label: "Groups and segmentation",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "caution",
    interpretation: "Group and segmentation outputs require design justification and method-specific prerequisites such as MICOM where applicable.",
    thresholds: "Permutation p values, invariance steps, segment sizes, and information criteria are method-specific guides.",
    why: "Group differences are easy to overclaim if measurement invariance, sample size, or segment recovery is weak.",
    report: "Report group column, group sizes, permutation count, invariance status, segment count, and method warnings.",
  },
  extended: {
    metricId: "extended.methods",
    label: "Extended method payloads",
    scopeStatus: "Validated for documented QuickPLS scope",
    tone: "informational",
    interpretation: "Extended outputs are shown only within their documented QuickPLS scope; unsupported variants stay warning-marked.",
    thresholds: "Use the method-specific documentation rather than transferring PLS-SEM thresholds across method families.",
    why: "Regression, NCA, PCA, GSCA, and CB-SEM answer different methodological questions.",
    report: "Report the method version, supported scope, estimator/settings, key estimates, and warnings.",
  },
};

function InterpretationPanel({ descriptor }: { descriptor: InterpretationDescriptor }) {
  return <details className={`interpretation-panel ${descriptor.tone}`} open>
    <summary><CheckCircle2 size={14} />{descriptor.label}<StatusBadge status={descriptor.scopeStatus.startsWith("Validated") ? "validated" : descriptor.scopeStatus.startsWith("Experimental") ? "warning" : "info"}>{descriptor.scopeStatus}</StatusBadge></summary>
    <div className="interpretation-grid">
      <article><strong>Interpretation</strong><p>{descriptor.interpretation}</p></article>
      <article><strong>Threshold guidance</strong><p>{descriptor.thresholds}</p></article>
      <article><strong>Why this matters</strong><p>{descriptor.why}</p></article>
      <article><strong>What to report</strong><p>{descriptor.report}</p></article>
    </div>
  </details>;
}

function displayFindings(findings: InterpretationFinding[], limit = 6) {
  const seen = new Set<string>();
  const priority: Record<InterpretationFinding["severity"], number> = { issue: 0, caution: 1, unavailable: 2, info: 3, good: 4 };
  return findings
    .filter((finding) => {
      const pair = finding.metric === "HTMT+" && finding.interpretation.includes(" between ")
        ? finding.interpretation.replace(/^HTMT\+ between /, "").replace(/ is .*/, "").split(" and ").sort().join("|")
        : "";
      const subject = pair || finding.construct || finding.indicator || (finding.path ? `${finding.path.source}->${finding.path.target}` : "");
      const key = `${finding.metric}|${subject}|${finding.value}|${finding.recommendedAction}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    })
    .sort((left, right) => priority[left.severity] - priority[right.severity])
    .slice(0, limit);
}

function FindingCards({ findings, title, onFocusPath }: { findings: InterpretationFinding[]; title: string; onFocusPath?: (source: string, target: string) => void }) {
  if (!findings.length) return <section className="finding-panel empty"><div><strong>{title}</strong><span>No value-specific findings are available for this tab.</span></div></section>;
  const copyFindings = async () => navigator.clipboard?.writeText(copyableInterpretationText(findings));
  const visibleFindings = displayFindings(findings);
  const issueCount = findings.filter((finding) => finding.severity === "issue").length;
  const cautionCount = findings.filter((finding) => finding.severity === "caution" || finding.severity === "unavailable").length;
  const remainingCount = Math.max(0, findings.length - visibleFindings.length);
  const lanes = [
    { label: "Must address", findings: visibleFindings.filter((finding) => finding.severity === "issue") },
    { label: "Review", findings: visibleFindings.filter((finding) => finding.severity === "caution" || finding.severity === "unavailable") },
    { label: "Info", findings: visibleFindings.filter((finding) => finding.severity === "info" || finding.severity === "good") },
  ].filter((lane) => lane.findings.length);
  return <section className="finding-panel" aria-label={title}>
    <div className="finding-panel-header"><strong>{title}</strong><button type="button" onClick={() => void copyFindings()}>Copy interpretation</button></div>
    <div className="finding-triage-row" aria-label="Finding priority summary">
      <span className={issueCount ? "issue" : "good"}>{issueCount} must address</span>
      <span className={cautionCount ? "caution" : "good"}>{cautionCount} review</span>
      <span>{findings.length} total findings</span>
    </div>
    <div className="finding-lane-grid">
      {lanes.map((lane) => <div key={lane.label} className="finding-lane">
        <h4>{lane.label}</h4>
        <div className="finding-card-grid">
          {lane.findings.map((finding) => <article key={finding.id} className={`finding-card ${finding.severity}`}>
            <div><span>{severityText(finding.severity)}</span><b>{finding.metric}</b></div>
            <strong>{finding.value}</strong>
            <dl className="finding-card-detail">
              <div><dt>What the value says</dt><dd>{finding.interpretation}</dd></div>
              <div><dt>Why it matters</dt><dd>{finding.thresholdGuide}</dd></div>
              <div><dt>What to inspect next</dt><dd>{finding.recommendedAction}</dd></div>
              <div><dt>Report wording</dt><dd>{finding.reportSentence}</dd></div>
            </dl>
            {finding.path && onFocusPath ? <button type="button" onClick={() => onFocusPath(finding.path!.source, finding.path!.target)}>Focus diagram path</button> : null}
          </article>)}
        </div>
      </div>)}
    </div>
    {remainingCount ? <p className="finding-more-note">{remainingCount} more finding(s) are available in the Interpretation tab or copied text.</p> : null}
  </section>;
}

function FindingChecklist({ title, findings }: { title: string; findings: InterpretationFinding[] }) {
  const copyFindings = async () => navigator.clipboard?.writeText(copyableInterpretationText(findings));
  if (!findings.length) return <section className="finding-checklist"><div className="result-section-title"><strong>{title}</strong></div><p>No items in this group.</p></section>;
  return <section className="finding-checklist">
    <div className="result-section-title"><strong>{title}</strong><div className="result-section-actions"><button type="button" onClick={() => void copyFindings()}>Copy checklist</button></div></div>
    <ol>
      {findings.map((finding) => <li key={finding.id} className={finding.severity}>
        <strong>{finding.metric}: {finding.value}</strong>
        <p>{finding.interpretation}</p>
        <span>{finding.recommendedAction}</span>
      </li>)}
    </ol>
  </section>;
}

function SectionTable({ title, note, columns, rows, onRowClick, activeRowIndexes, guidance }: { title: string; note?: string; columns: string[]; rows: string[][]; onRowClick?: (row: string[], index: number) => void; activeRowIndexes?: number[]; guidance?: InterpretationDescriptor }) {
  const resultState = useWorkspace((state) => state.resultWorkspaceState);
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const [localSearch, setLocalSearch] = useState("");
  const [selectedRows, setSelectedRows] = useState<Set<number>>(new Set());
  if (!rows.length) return null;
  const activeRows = new Set(activeRowIndexes ?? []);
  const globalQuery = resultState.tableSearch.trim().toLowerCase();
  const localQuery = localSearch.trim().toLowerCase();
  const searchableRows = rows
    .map((row, index) => ({ row, index }))
    .filter(({ row }) => {
      const haystack = [title, ...row].join(" ").toLowerCase();
      return (!globalQuery || haystack.includes(globalQuery)) && (!localQuery || haystack.includes(localQuery));
    });
  const sortedRows = [...searchableRows];
  const [sortTitle, sortIndexText, sortDirection] = resultState.tableSort?.split("|") ?? [];
  const sortIndex = Number(sortIndexText);
  if (sortTitle === title && Number.isInteger(sortIndex)) {
    sortedRows.sort((left, right) => {
      const leftValue = left.row[sortIndex] ?? "";
      const rightValue = right.row[sortIndex] ?? "";
      const leftNumber = Number(leftValue);
      const rightNumber = Number(rightValue);
      const comparison = Number.isFinite(leftNumber) && Number.isFinite(rightNumber)
        ? leftNumber - rightNumber
        : leftValue.localeCompare(rightValue, undefined, { numeric: true, sensitivity: "base" });
      return sortDirection === "desc" ? -comparison : comparison;
    });
  }
  const selectedDetail = resultState.selectedDetailRow?.startsWith(`${title}:`) ? resultState.selectedDetailRow : null;
  const interpretationColumnIndexes = new Set(columns
    .map((column, index) => (/interpretation|status|quick check|direction|effect type|class|scope/i.test(column) ? index : -1))
    .filter((index) => index >= 0));
  const visibleColumnEntries = columns
    .map((column, index) => ({ column, index }))
    .filter(({ index }) => resultState.showInterpretationColumns || !interpretationColumnIndexes.has(index));
  const displayRows = sortedRows.map(({ row }) => visibleColumnEntries.map(({ index }) => formatDisplayCell(row[index] ?? "", resultState.resultPrecision)));
  const isWideTable = visibleColumnEntries.length > 6;
  const activeSortColumn = sortTitle === title && Number.isInteger(sortIndex) ? columns[sortIndex] : null;
  const tableSlug = title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "table";
  const tableBody = [visibleColumnEntries.map(({ column }) => column), ...displayRows];
  const selectedDisplayRows = sortedRows
    .map(({ row, index }) => ({ index, row: visibleColumnEntries.map(({ index: columnIndex }) => formatDisplayCell(row[columnIndex] ?? "", resultState.resultPrecision)) }))
    .filter(({ index }) => selectedRows.has(index));
  const copyTable = async () => {
    const body = tableBody.map((row) => row.join("\t")).join("\n");
    await navigator.clipboard?.writeText(body);
  };
  const copySelectedRows = async () => {
    const selectedBody = [visibleColumnEntries.map(({ column }) => column), ...selectedDisplayRows.map(({ row }) => row)];
    await navigator.clipboard?.writeText(selectedBody.map((row) => row.join("\t")).join("\n"));
  };
  const exportCurrentTable = () => {
    const csv = tableBody
      .map((row) => row.map((cell) => {
        const value = String(cell ?? "");
        return /[",\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
      }).join(","))
      .join("\n");
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `quickpls-${tableSlug}.csv`;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };
  const activePanel = resultState.activeInterpretationPanel === title;
  const toggleSort = (columnIndex: number) => {
    const sortKey = `${title}|${columnIndex}|`;
    const nextDirection = resultState.tableSort?.startsWith(sortKey) && resultState.tableSort.endsWith("|asc") ? "desc" : "asc";
    setResultState({ tableSort: `${title}|${columnIndex}|${nextDirection}` });
  };
  const toggleSelectedRow = (rowIndex: number) => {
    setSelectedRows((current) => {
      const next = new Set(current);
      if (next.has(rowIndex)) next.delete(rowIndex);
      else next.add(rowIndex);
      return next;
    });
  };
  const toggleAllVisibleRows = () => {
    setSelectedRows((current) => {
      const next = new Set(current);
      const allVisibleSelected = sortedRows.every(({ index }) => next.has(index));
      sortedRows.forEach(({ index }) => {
        if (allVisibleSelected) next.delete(index);
        else next.add(index);
      });
      return next;
    });
  };
  return <section className={`result-table-section research-table-shell results-v2-table-shell v2100-research-table v2290-research-table ${resultState.tableDensity}${isWideTable ? " wide" : ""}`} data-results-research-table-pass="v2.10" data-v229-research-table="true" data-result-table-title={title}>
    <div className="result-section-title results-v2-table-header">
      <div className="results-v2-table-title"><strong>{title}</strong>{note ? <span>{note}</span> : null}</div>
      <div className="result-section-meta results-v2-table-meta"><span>{displayRows.length} of {rows.length} rows</span><span>{visibleColumnEntries.length} of {columns.length} columns</span><span>{selectedDisplayRows.length} selected</span>{isWideTable ? <span>Wide table: first column stays pinned while scrolling</span> : null}</div>
      <div className="result-section-actions"><button type="button" onClick={() => void copyTable()}>Copy table</button><button type="button" disabled={!selectedDisplayRows.length} title={selectedDisplayRows.length ? "Copy selected rows" : "Select one or more rows first"} onClick={() => void copySelectedRows()}>Copy selected</button><button type="button" onClick={exportCurrentTable}>Export table</button>{guidance ? <button type="button" onClick={() => setResultState({ activeInterpretationPanel: activePanel ? null : title })}>{activePanel ? "Hide guidance" : "Interpretation"}</button> : null}</div>
    </div>
    <div className="research-table-toolbar" aria-label={`${title} table tools`}>
      <label><span>Search this table</span><input value={localSearch} onChange={(event) => setLocalSearch(event.target.value)} placeholder="Filter rows..." aria-label={`Search ${title}`} /></label>
      <label><span>Precision</span><select value={resultState.resultPrecision} onChange={(event) => setResultState({ resultPrecision: Number(event.target.value) })} aria-label={`Precision for ${title}`}>{[2, 3, 4, 5, 6].map((precision) => <option key={precision} value={precision}>{precision} decimals</option>)}</select></label>
      <button type="button" onClick={() => setResultState({ tableDensity: resultState.tableDensity === "compact" ? "comfortable" : "compact" })}>{resultState.tableDensity === "compact" ? "Comfortable rows" : "Compact rows"}</button>
      <button type="button" onClick={() => setResultState({ showInterpretationColumns: !resultState.showInterpretationColumns })}>{resultState.showInterpretationColumns ? "Hide interpretation columns" : "Show interpretation columns"}</button>
    </div>
    {guidance && activePanel ? <InterpretationPanel descriptor={guidance} /> : null}
    <div className="result-table-affordance v2100-table-affordance">
      <span>{globalQuery || localQuery ? `Filtered by ${[resultState.tableSearch.trim(), localSearch.trim()].filter(Boolean).map((item) => `"${item}"`).join(" and ")}.` : "Use workbook search or table search to filter rows."}</span>
      <span>{activeSortColumn ? `Sorted by ${activeSortColumn} ${sortDirection === "desc" ? "descending" : "ascending"}.` : "Click any column header to sort."}</span>
      {isWideTable ? <span>Scroll horizontally; first column stays pinned.</span> : null}
    </div>
    <div className="bootstrap-table-scroll result-table-scroll" tabIndex={0} role="region" aria-label={`${title} table`}><table><caption>{title}. {displayRows.length} row(s), {visibleColumnEntries.length} visible column(s).</caption><thead><tr><th className="research-table-select-cell"><input type="checkbox" aria-label={`Select all visible ${title} rows`} checked={sortedRows.length > 0 && sortedRows.every(({ index }) => selectedRows.has(index))} onChange={toggleAllVisibleRows} /></th>{visibleColumnEntries.map(({ column, index }) => <th key={`${title}-${column}-${index}`}><button type="button" className="table-sort-button" onClick={() => toggleSort(index)}>{column}</button></th>)}</tr></thead><tbody>
      {displayRows.map((row, displayIndex) => {
        const original = sortedRows[displayIndex];
        return <tr key={`${title}-${original.index}`} className={`${onRowClick ? "result-path-row" : ""}${activeRows.has(original.index) ? " active-result-row" : ""}`.trim() || undefined} aria-current={activeRows.has(original.index) ? "true" : undefined} onClick={() => {
        setResultState({ selectedDetailRow: `${title}:${row.join("|")}` });
        onRowClick?.(original.row, original.index);
      }}><td className="research-table-select-cell" onClick={(event) => event.stopPropagation()}><input type="checkbox" aria-label={`Select ${title} row ${displayIndex + 1}`} checked={selectedRows.has(original.index)} onChange={() => toggleSelectedRow(original.index)} /></td>{row.map((cell, cellIndex) => <td key={`${title}-${original.index}-${cellIndex}`}>{cell}</td>)}</tr>;
      })}
    </tbody></table></div>
    {selectedDetail ? <div className="result-row-detail"><strong>Selected row interpretation</strong><span>{rowSpecificInterpretation(title, visibleColumnEntries.map(({ column }) => column), selectedDetail.replace(`${title}:`, "").split("|"))}</span></div> : null}
  </section>;
}

function MatrixTable({ title, note, constructs, values, guidance }: { title: string; note?: string; constructs: string[]; values: Array<Array<number | null>>; guidance?: InterpretationDescriptor }) {
  const resultState = useWorkspace((state) => state.resultWorkspaceState);
  const setResultState = useWorkspace((state) => state.setResultWorkspaceState);
  const activePanel = resultState.activeInterpretationPanel === title;
  return <section className="result-table-section research-table-shell results-v2-table-shell">
    <div className="result-section-title results-v2-table-header">
      <div className="results-v2-table-title"><strong>{title}</strong>{note ? <span>{note}</span> : null}</div>
      <div className="result-section-meta results-v2-table-meta"><span>{constructs.length} constructs</span><span>{constructs.length + 1} columns</span></div>
      <div className="result-section-actions">{guidance ? <button type="button" onClick={() => setResultState({ activeInterpretationPanel: activePanel ? null : title })}>{activePanel ? "Hide guidance" : "Interpretation"}</button> : null}</div>
    </div>
    {guidance && activePanel ? <InterpretationPanel descriptor={guidance} /> : null}
    <div className="bootstrap-table-scroll result-table-scroll" tabIndex={0} role="region" aria-label={`${title} matrix`}><table><thead><tr><th>Construct</th>{constructs.map((construct) => <th key={construct}>{construct}</th>)}</tr></thead><tbody>
      {values.map((row, rowIndex) => <tr key={constructs[rowIndex]}><td>{constructs[rowIndex]}</td>{row.map((value, columnIndex) => <td key={constructs[columnIndex]}>{value?.toFixed(4) ?? "N/A"}</td>)}</tr>)}
    </tbody></table></div>
  </section>;
}

function MetricTile({ label, value, detail, tone = "neutral" }: { label: string; value: string; detail: string; tone?: "ok" | "warn" | "neutral" }) {
  return <article className={`result-metric-tile ${tone}`}><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>;
}

function ResultGuidance({ title, items }: { title: string; items: string[] }) {
  return <section className="result-guidance"><strong>{title}</strong><ul>{items.map((item) => <li key={item}>{item}</li>)}</ul></section>;
}

function SectionEmpty({ title, detail }: { title: string; detail: string }) {
  return <div className="result-section-empty"><strong>{title}</strong><p>{detail}</p></div>;
}

function MethodPayloadSections({ result }: { result: PlsResult }) {
  return <div className="method-results">
    {result.plsc && <><strong>PLSc correction</strong><MethodWarnings warnings={result.plsc.warnings} /><table><thead><tr><th>Construct</th><th>rho_A</th></tr></thead><tbody>
      {result.plsc.reliabilities.map((row) => <tr key={row.construct}><td>{row.construct}</td><td>{row.rho_a.toFixed(6)}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Correlation</th><th>Original</th><th>Corrected</th></tr></thead><tbody>
      {result.plsc.construct_correlations.map((row) => <tr key={`${row.left}-${row.right}`}><td>{row.left} - {row.right}</td><td>{row.original.toFixed(6)}</td><td>{row.corrected.toFixed(6)}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Corrected path</th><th>Coefficient</th></tr></thead><tbody>
      {result.plsc.corrected_paths.map((path) => <tr key={`${path.source}-${path.target}`}><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>)}
    </tbody></table></>}

    {result.wpls && <><strong>WPLS case weights</strong><MethodWarnings warnings={result.wpls.warnings} /><table><thead><tr><th>Weight column</th><th>Weight sum</th><th>Effective sample size</th><th>Covariance</th></tr></thead><tbody>
      <tr><td>{result.wpls.case_weight_column}</td><td>{result.wpls.weight_sum.toFixed(6)}</td><td>{result.wpls.effective_sample_size.toFixed(4)}</td><td>{formatDiagnosticCode(result.wpls.covariance)}</td></tr>
    </tbody></table></>}

    {result.cca && <><strong>CCA composite residuals</strong><MethodWarnings warnings={result.cca.warnings} /><div className="method-metric"><span>Max absolute residual</span><b>{result.cca.max_absolute_residual.toFixed(6)}</b></div><table><thead><tr><th>Construct pair</th><th>Observed</th><th>Reproduced</th><th>Residual</th><th>|Residual|</th></tr></thead><tbody>
      {result.cca.correlations.map((row) => <tr key={`${row.left}-${row.right}`}><td>{row.left} - {row.right}</td><td>{row.observed.toFixed(6)}</td><td>{row.reproduced.toFixed(6)}</td><td>{row.residual.toFixed(6)}</td><td>{row.absolute_residual.toFixed(6)}</td></tr>)}
    </tbody></table></>}

    {result.pca && <><strong>Standalone PCA</strong><MethodWarnings warnings={result.pca.warnings} /><div className="method-metric"><span>Retained components</span><b>{result.pca.retained_components} by {formatDiagnosticCode(result.pca.component_rule)} | {result.pca.observations} observations</b></div><table><thead><tr><th>Component</th><th>Eigenvalue</th><th>Explained variance</th><th>Cumulative</th></tr></thead><tbody>
      {result.pca.components.map((row) => <tr key={row.component}><td>{row.component}</td><td>{row.eigenvalue.toFixed(6)}</td><td>{row.explained_variance.toFixed(4)}</td><td>{row.cumulative_variance.toFixed(4)}</td></tr>)}
    </tbody></table><div className="bootstrap-table-scroll"><table><thead><tr><th>Variable</th><th>Component</th><th>Loading</th><th>Weight</th></tr></thead><tbody>
      {result.pca.loadings.slice(0, 100).map((row) => <tr key={`${row.variable}-${row.component}`}><td>{row.variable}</td><td>{row.component}</td><td>{row.loading.toFixed(6)}</td><td>{row.weight.toFixed(6)}</td></tr>)}
    </tbody></table></div></>}

    {result.regression && <><strong>{result.regression.regression_type === "process" ? "PROCESS-style workflow" : result.regression.regression_type === "logistic" ? "Logistic regression" : "OLS regression"}</strong><MethodWarnings warnings={result.regression.warnings} /><div className="method-metric"><span>Model fit</span><b>{result.regression.outcome} | n {result.regression.observations} | R2 {formatOptional(result.regression.fit.r_squared ?? result.regression.fit.pseudo_r_squared, 4)} | AIC {result.regression.fit.aic.toFixed(4)}</b></div><table><thead><tr><th>Term</th><th>Estimate</th><th>SE</th><th>t/z</th><th>p</th><th>CI</th><th>Odds ratio</th></tr></thead><tbody>
      {result.regression.coefficients.map((row) => <tr key={row.term}><td>{row.term}</td><td>{row.estimate.toFixed(6)}</td><td>{row.standard_error.toFixed(6)}</td><td>{row.statistic.toFixed(4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{formatInterval(row.confidence_interval_lower, row.confidence_interval_upper)}</td><td>{formatOptional(row.odds_ratio, 6)}</td></tr>)}
    </tbody></table>
    {result.regression.process && <><strong>PROCESS effects</strong><MethodWarnings warnings={result.regression.process.warnings} /><table><thead><tr><th>Effect</th><th>Estimate</th><th>Bootstrap CI</th></tr></thead><tbody>
      {result.regression.process.effects.map((row) => <tr key={row.effect}><td>{formatDiagnosticCode(row.effect)}</td><td>{row.estimate.toFixed(6)}</td><td>{formatInterval(row.lower_percentile, row.upper_percentile)}</td></tr>)}
    </tbody></table>{result.regression.process.simple_slopes.length > 0 && <table><thead><tr><th>Moderator value</th><th>Simple slope</th></tr></thead><tbody>
      {result.regression.process.simple_slopes.map((row) => <tr key={row.moderator_value}><td>{row.moderator_value.toFixed(4)}</td><td>{row.slope.toFixed(6)}</td></tr>)}
    </tbody></table>}</>}
    </>}

    {result.nca && <><strong>NCA ceilings</strong><MethodWarnings warnings={result.nca.warnings} /><div className="method-metric"><span>Variables</span><b>{result.nca.x} &gt; {result.nca.y} | {result.nca.observations} observations | {result.nca.usable_permutations}/{result.nca.permutation_samples} permutations</b></div><table><thead><tr><th>Ceiling</th><th>Effect size</th><th>Permutation p</th></tr></thead><tbody>
      {result.nca.ceilings.map((row) => <tr key={row.ceiling}><td>{formatDiagnosticCode(row.ceiling)}</td><td>{row.effect_size.toFixed(6)}</td><td>{formatPValue(row.permutation_p_value)}</td></tr>)}
    </tbody></table><table><thead><tr><th>Ceiling</th><th>Outcome target %</th><th>Required X %</th><th>Interpretation</th></tr></thead><tbody>
      {result.nca.bottlenecks.map((row) => <tr key={`${row.ceiling ?? result.nca?.ceiling}-${row.outcome_percent}`}><td>{formatDiagnosticCode(row.ceiling ?? result.nca?.ceiling ?? "")}</td><td>{row.outcome_percent.toFixed(0)}</td><td>{row.required_x_percent == null ? "" : row.required_x_percent.toFixed(4)}</td><td>{row.status === "not_necessary" ? "Not necessary at this outcome level" : row.status === "not_attainable" ? "Outcome level is above this ceiling line" : "Required condition level"}</td></tr>)}
    </tbody></table></>}

    {result.gsca && <><strong>GSCA component model</strong><MethodWarnings warnings={result.gsca.warnings} /><div className="method-metric"><span>Fit</span><b>FIT {result.gsca.fit.toFixed(4)} | AFIT {result.gsca.adjusted_fit.toFixed(4)} | GFI {result.gsca.gfi.toFixed(4)}</b></div><table><thead><tr><th>Path</th><th>Coefficient</th></tr></thead><tbody>
      {result.gsca.paths.map((path) => <tr key={`${path.source}-${path.target}`}><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>)}
    </tbody></table><div className="bootstrap-table-scroll"><table><thead><tr><th>Construct</th><th>Indicator</th><th>Weight</th><th>Loading</th></tr></thead><tbody>
      {result.gsca.weights.slice(0, 100).map((row) => <tr key={`${row.construct}-${row.indicator}`}><td>{row.construct}</td><td>{row.indicator}</td><td>{row.weight.toFixed(6)}</td><td>{row.loading.toFixed(6)}</td></tr>)}
    </tbody></table></div></>}

    {result.predict && <><strong>PLSpredict holdout</strong><MethodWarnings warnings={result.predict.warnings} /><div className="method-metric"><span>Split</span><b>{result.predict.training_observations} train / {result.predict.test_observations} test</b></div><PlsPredictTable targets={result.predict.targets} />
    {result.predict.repeated_kfold && <><strong>Repeated k-fold prediction</strong><MethodWarnings warnings={result.predict.repeated_kfold.warnings} /><div className="method-metric"><span>Plan</span><b>{result.predict.repeated_kfold.repeats} x {result.predict.repeated_kfold.folds} folds / {result.predict.repeated_kfold.total_test_observations} tests</b></div><PlsPredictTable targets={result.predict.repeated_kfold.targets} />{result.predict.repeated_kfold.cvpat?.length ? <CvpatTable comparisons={result.predict.repeated_kfold.cvpat} /> : null}</>}</>}

    {result.mga && <><strong>Bounded two-group MGA</strong><MethodWarnings warnings={result.mga.warnings} /><div className="method-metric"><span>Group column</span><b>{result.mga.group_column}</b></div><table><thead><tr><th>Path</th><th>Group A</th><th>Coef A</th><th>Group B</th><th>Coef B</th><th>Difference</th><th>SE</th><th>t</th><th>p</th></tr></thead><tbody>
      {result.mga.comparisons.map((row) => <tr key={`${row.source}-${row.target}-${row.group_a}-${row.group_b}`} title={row.warning ?? undefined}><td>{row.source} -&gt; {row.target}</td><td>{row.group_a}</td><td>{row.coefficient_a.toFixed(6)}</td><td>{row.group_b}</td><td>{row.coefficient_b.toFixed(6)}</td><td>{row.difference.toFixed(6)}</td><td>{row.standard_error?.toFixed(6) ?? "N/A"}</td><td>{row.t_statistic?.toFixed(4) ?? "N/A"}</td><td>{formatPValue(row.p_value_two_sided)}</td></tr>)}
    </tbody></table></>}

    {result.micom && <><strong>MICOM</strong><MethodWarnings warnings={result.micom.warnings} /><table><thead><tr><th>Construct</th><th>Composition p</th><th>Mean p</th><th>Variance p</th><th>Partial</th><th>Full</th></tr></thead><tbody>
      {result.micom.constructs.map((row) => <tr key={row.construct}><td>{row.construct}</td><td>{formatPValue(row.compositional_p_value)}</td><td>{formatPValue(row.mean_p_value)}</td><td>{formatPValue(row.variance_p_value)}</td><td>{row.partial_invariance ? "yes" : "no"}</td><td>{row.full_invariance ? "yes" : "no"}</td></tr>)}
    </tbody></table></>}

    {result.mga_permutation && <><strong>Permutation MGA</strong><MethodWarnings warnings={result.mga_permutation.warnings} /><table><thead><tr><th>Path</th><th>Difference</th><th>Empirical p</th><th>Percentile</th></tr></thead><tbody>
      {result.mga_permutation.comparisons.map((row) => <tr key={`${row.source}-${row.target}`}><td>{row.source} -&gt; {row.target}</td><td>{row.original_difference.toFixed(6)}</td><td>{formatPValue(row.empirical_p_value_two_sided)}</td><td>{row.percentile_rank?.toFixed(4) ?? "N/A"}</td></tr>)}
    </tbody></table></>}

    {result.fimix && <><strong>FIMIX-PLS</strong><MethodWarnings warnings={result.fimix.warnings} /><div className="method-metric"><span>Classes</span><b>{result.fimix.classes}; BIC {result.fimix.bic.toFixed(4)}</b></div><table><thead><tr><th>Class</th><th>Observations</th><th>Share</th><th>Path</th><th>Coefficient</th></tr></thead><tbody>
      {result.fimix.classes_summary.flatMap((item) => item.paths.map((path) => <tr key={`${item.class}-${path.source}-${path.target}`}><td>{item.class}</td><td>{item.observations}</td><td>{item.share.toFixed(4)}</td><td>{path.source} -&gt; {path.target}</td><td>{path.coefficient.toFixed(6)}</td></tr>))}
    </tbody></table></>}

    {result.ipma && <><strong>IPMA / cIPMA</strong><MethodWarnings warnings={result.ipma.warnings} /><table><thead><tr><th>Target</th><th>Construct</th><th>Importance</th><th>Performance</th></tr></thead><tbody>
      {result.ipma.constructs.map((row) => <tr key={`${row.target}-${row.construct}`}><td>{row.target}</td><td>{row.construct}</td><td>{row.importance.toFixed(6)}</td><td>{row.performance.toFixed(4)}</td></tr>)}
    </tbody></table></>}

    {result.cbsem && <><strong>CB-SEM / CFA ML</strong><MethodWarnings warnings={[...result.cbsem.warnings, ...result.cbsem.diagnostics]} /><div className="method-metric"><span>Fit</span><b>chi-square {result.cbsem.fit.chi_square.toFixed(4)} | df {result.cbsem.fit.degrees_of_freedom} | CFI {formatOptional(result.cbsem.fit.cfi, 4)} | RMSEA {formatOptional(result.cbsem.fit.rmsea, 4)} | SRMR {result.cbsem.fit.srmr.toFixed(4)}</b></div>
    <table><thead><tr><th>Parameter</th><th>Kind</th><th>Estimate</th><th>SE</th><th>z</th><th>p</th><th>Fixed</th></tr></thead><tbody>
      {result.cbsem.parameters.slice(0, 80).map((row) => <tr key={row.name} title={row.warning ?? undefined}><td>{row.lhs} - {row.rhs}</td><td>{formatDiagnosticCode(row.kind)}</td><td>{row.estimate.toFixed(6)}</td><td>{formatOptional(row.standard_error, 6)}</td><td>{formatOptional(row.z_statistic, 4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{row.fixed ? "yes" : "no"}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Parameter</th><th>std_lv</th><th>std_all</th></tr></thead><tbody>
      {result.cbsem.standardized.slice(0, 80).map((row) => <tr key={row.name}><td>{row.lhs} - {row.rhs}</td><td>{row.std_lv.toFixed(6)}</td><td>{row.std_all.toFixed(6)}</td></tr>)}
    </tbody></table>
    <table><thead><tr><th>Modification</th><th>MI</th><th>EPC</th></tr></thead><tbody>
      {result.cbsem.modification_indices.slice(0, 40).map((row) => <tr key={`${row.kind}-${row.lhs}-${row.rhs}`}><td>{formatDiagnosticCode(row.kind)} {row.lhs} - {row.rhs}</td><td>{row.modification_index.toFixed(4)}</td><td>{formatOptional(row.expected_parameter_change, 6)}</td></tr>)}
    </tbody></table>
    {result.cbsem.bootstrap && <><strong>CB-SEM bootstrap intervals</strong><MethodWarnings warnings={result.cbsem.bootstrap.warnings} /><table><thead><tr><th>Parameter</th><th>Original</th><th>Lower</th><th>Upper</th></tr></thead><tbody>
      {result.cbsem.bootstrap.intervals.map((row) => <tr key={row.parameter}><td>{row.parameter}</td><td>{row.original.toFixed(6)}</td><td>{row.lower_percentile.toFixed(6)}</td><td>{row.upper_percentile.toFixed(6)}</td></tr>)}
    </tbody></table></>}
    {result.cbsem.multigroup && <><strong>CB-SEM multigroup invariance</strong><MethodWarnings warnings={result.cbsem.multigroup.warnings} /><table><thead><tr><th>Group</th><th>Observations</th><th>chi-square</th><th>df</th><th>CFI</th><th>RMSEA</th></tr></thead><tbody>
      {result.cbsem.multigroup.groups.map((row) => <tr key={row.group}><td>{row.group}</td><td>{row.observations}</td><td>{row.chi_square.toFixed(4)}</td><td>{row.degrees_of_freedom}</td><td>{formatOptional(row.cfi, 4)}</td><td>{formatOptional(row.rmsea, 4)}</td></tr>)}
    </tbody></table><table><thead><tr><th>Step</th><th>chi-square</th><th>df</th><th>Delta chi-square</th><th>Delta df</th><th>Delta CFI</th><th>Delta RMSEA</th></tr></thead><tbody>
      {result.cbsem.multigroup.invariance.map((row) => <tr key={row.step} title={row.warning ?? undefined}><td>{formatDiagnosticCode(row.step)}</td><td>{row.chi_square.toFixed(4)}</td><td>{row.degrees_of_freedom}</td><td>{formatOptional(row.delta_chi_square, 4)}</td><td>{row.delta_degrees_of_freedom ?? "N/A"}</td><td>{formatOptional(row.delta_cfi, 4)}</td><td>{formatOptional(row.delta_rmsea, 4)}</td></tr>)}
    </tbody></table></>}</>}

    {result.cta_pls && <><strong>CTA-PLS tetrads</strong><MethodWarnings warnings={result.cta_pls.warnings} /><table><thead><tr><th>Construct</th><th>Max |tetrad|</th></tr></thead><tbody>
      {Object.entries(result.cta_pls.max_absolute_tetrad_by_construct).map(([construct, value]) => <tr key={construct}><td>{construct}</td><td>{value.toFixed(6)}</td></tr>)}
    </tbody></table>
    <div className="bootstrap-table-scroll"><table><thead><tr><th>Construct</th><th>Indicators</th><th>Pairing</th><th>Tetrad</th><th>|Tetrad|</th></tr></thead><tbody>
      {result.cta_pls.estimates.map((row) => <tr key={`${row.construct}-${row.indicator_a}-${row.indicator_b}-${row.indicator_c}-${row.indicator_d}-${row.pairing}`}><td>{row.construct}</td><td>{row.indicator_a}, {row.indicator_b}, {row.indicator_c}, {row.indicator_d}</td><td>{formatDiagnosticCode(row.pairing)}</td><td>{row.tetrad.toFixed(6)}</td><td>{row.absolute_tetrad.toFixed(6)}</td></tr>)}
    </tbody></table></div></>}

    {result.endogeneity && <><strong>Gaussian-copula endogeneity</strong><MethodWarnings warnings={result.endogeneity.warnings} /><table><thead><tr><th>Path</th><th>Path coefficient</th><th>Copula coefficient</th><th>t</th><th>p</th><th>Skewness</th><th>Applicability</th></tr></thead><tbody>
      {result.endogeneity.estimates.map((row) => <tr key={`${row.source}-${row.target}`} title={row.warning ?? undefined}><td>{row.source} -&gt; {row.target}</td><td>{row.path_coefficient.toFixed(6)}</td><td>{row.copula_coefficient.toFixed(6)}</td><td>{row.t_statistic.toFixed(4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{row.predictor_skewness.toFixed(4)}</td><td>{row.applicable ? "screenable" : "weak"}</td></tr>)}
    </tbody></table></>}

    {result.nonlinear_effects && <><strong>Nonlinear effects</strong><MethodWarnings warnings={result.nonlinear_effects.warnings} /><table><thead><tr><th>Path</th><th>Linear</th><th>Quadratic</th><th>t</th><th>p</th><th>Linear R2</th><th>Augmented R2</th><th>Delta R2</th></tr></thead><tbody>
      {result.nonlinear_effects.estimates.map((row) => <tr key={`${row.source}-${row.target}`} title={row.warning ?? undefined}><td>{row.source} -&gt; {row.target}</td><td>{row.linear_coefficient.toFixed(6)}</td><td>{row.quadratic_coefficient.toFixed(6)}</td><td>{row.t_statistic.toFixed(4)}</td><td>{formatPValue(row.p_value_two_sided)}</td><td>{row.linear_r_squared.toFixed(4)}</td><td>{row.augmented_r_squared.toFixed(4)}</td><td>{row.delta_r_squared.toFixed(4)}</td></tr>)}
    </tbody></table></>}

    {result.moderated_mediation && <><strong>Moderated mediation</strong><MethodWarnings warnings={result.moderated_mediation.warnings} /><table><thead><tr><th>Effect</th><th>Stage</th><th>Index</th><th>Conditional indirect effects</th></tr></thead><tbody>
      {result.moderated_mediation.estimates.map((row) => <tr key={`${row.interaction}-${row.predictor}-${row.target}`} title={row.warning ?? undefined}><td>{row.predictor} via {row.mediator} to {row.target}</td><td>{formatDiagnosticCode(row.moderated_stage)}</td><td>{row.index_of_moderated_mediation.toFixed(6)}</td><td>{row.conditional_indirect_effects.map((effect) => `${formatModeratorLevel(effect.moderator_score)}: ${effect.indirect_effect.toFixed(6)}`).join(" | ")}</td></tr>)}
    </tbody></table></>}
  </div>;
}

function PlsPredictTable({ targets }: { targets: NonNullable<PlsResult["predict"]>["targets"] }) {
  return <SectionTable
    title="PLSpredict target metrics"
    note="Holdout prediction metrics are grouped in the shared research table shell."
    columns={["Construct", "Predictors", "RMSE PLS", "MAE PLS", "RMSE benchmark", "MAE benchmark", "Q² predict", "RMSE LM", "Q² LM"]}
    rows={targets.map((row) => [row.construct, String(row.predictor_count), row.rmse_pls.toFixed(6), row.mae_pls.toFixed(6), row.rmse_benchmark.toFixed(6), row.mae_benchmark.toFixed(6), row.q_squared_predict?.toFixed(6) ?? "N/A", row.rmse_lm?.toFixed(6) ?? "N/A", row.q_squared_predict_lm?.toFixed(6) ?? "N/A"])}
    guidance={interpretationRegistry.prediction}
  />;
}

function CvpatTable({ comparisons }: { comparisons: NonNullable<NonNullable<PlsResult["predict"]>["repeated_kfold"]>["cvpat"] }) {
  return <SectionTable
    title="CVPAT paired loss comparisons"
    columns={["Target", "Comparison", "Mean loss diff", "SE", "t", "p", "Preferred"]}
    rows={(comparisons ?? []).map((row) => [row.target, formatDiagnosticCode(row.comparison), row.mean_loss_difference.toFixed(6), row.standard_error?.toFixed(6) ?? "N/A", row.t_statistic?.toFixed(4) ?? "N/A", formatPValue(row.p_value_two_sided), formatDiagnosticCode(row.preferred_model)])}
    guidance={interpretationRegistry.prediction}
  />;
}

function MethodWarnings({ warnings }: { warnings: string[] }) {
  if (!warnings.length) return null;
  return <ul className="method-warnings">{warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul>;
}

function HtmtTable({ label, artifact }: { label: string; artifact: HtmtAssessment }) {
  const pairRows = artifact.cells.flatMap((row, rowIndex) => row.map((cell, columnIndex) => ({
    rowIndex,
    columnIndex,
    left: artifact.constructs[rowIndex],
    right: artifact.constructs[columnIndex],
    value: cell.value,
    reason: cell.reason,
    status: cell.status,
  }))).filter((row) => row.rowIndex < row.columnIndex);
  const rows = pairRows.map((row) => {
    const status = htmtPairStatus(row.value);
    return [
      `${row.left} - ${row.right}`,
      row.value == null ? formatDiagnosticCode(row.reason ?? row.status) : row.value.toFixed(4),
      status,
      status === "issue" ? "Inspect wording, construct overlap, and theory before claiming discriminant validity." : status === "review" ? "Review against the stricter .85 guide and the conceptual distance between constructs." : "No threshold-relevant issue under common .85/.90 HTMT guides.",
    ];
  });
  return <div className="htmt-table-stack">
    <SectionTable title={`${label} construct pairs`} note="One row per construct pair is shown by default to avoid mirrored duplicate HTMT values." columns={["Construct pair", label, "Status", "Guidance"]} rows={rows} guidance={interpretationRegistry.discriminant} />
    <details className="result-matrix-details">
      <summary>Show full {label} matrix</summary>
      <div className="bootstrap-table-scroll result-table-scroll" tabIndex={0} role="region" aria-label={`${label} full matrix`}>
        <table><thead><tr><th>Construct</th>{artifact.constructs.map((construct) => <th key={construct}>{construct}</th>)}</tr></thead><tbody>
          {artifact.cells.map((row, rowIndex) => <tr key={artifact.constructs[rowIndex]}><th>{artifact.constructs[rowIndex]}</th>{row.map((cell, columnIndex) => <td key={artifact.constructs[columnIndex]} className={htmtPairStatus(cell.value)} title={cell.reason ? formatDiagnosticCode(cell.reason) : undefined}>{cell.value?.toFixed(4) ?? formatDiagnosticCode(cell.reason ?? cell.status)}</td>)}</tr>)}
        </tbody></table>
      </div>
    </details>
  </div>;
}

function interpretationNextSteps(run: AnalysisRun) {
  const result = run.result!;
  const steps: Array<{ reason: string; target: string }> = [];
  const weakLoadings = result.outer_estimates.filter((row) => Math.abs(row.loading) < 0.708);
  if (weakLoadings.length) steps.push({ reason: `${weakLoadings.length} indicator loading(s) are below the common 0.708 guide.`, target: "Measurement" });
  const highHtmt = run.assessment?.htmt_plus?.cells.flat().filter((cell) => (cell.value ?? 0) >= 0.9).length ?? 0;
  if (highHtmt) steps.push({ reason: `${highHtmt} HTMT+ cell(s) are at or above 0.90; review discriminant validity.`, target: "Validity" });
  const highVif = run.assessment?.structural_vif.filter((row) => (row.vif ?? 0) >= 3.3).length ?? 0;
  if (highVif) steps.push({ reason: `${highVif} structural VIF value(s) need collinearity review.`, target: "Structural" });
  const weakR2 = Object.entries(result.r_squared).filter(([, value]) => value < 0.25).map(([construct]) => construct);
  if (weakR2.length) steps.push({ reason: `Weak R² for ${weakR2.join(", ")}; review theory, predictors, and prediction diagnostics.`, target: "Structural / Prediction" });
  if (!run.bootstrap && !run.permutation) steps.push({ reason: "Inference was not run, so p values and confidence intervals are unavailable.", target: "Inference / Setup" });
  if (!steps.length) steps.push({ reason: "No immediate interpretation blockers were detected from common guidance checks.", target: "Report" });
  return steps;
}

function reportWording(run: AnalysisRun) {
  const result = run.result!;
  const bestR2 = Object.entries(result.r_squared).sort((a, b) => b[1] - a[1])[0];
  const loadingRange = result.outer_estimates.length ? rangeText(result.outer_estimates.map((row) => Math.abs(row.loading))) : "not available";
  const pathRange = result.paths.length ? rangeText(result.paths.map((row) => row.coefficient)) : "not available";
  return [
    { section: "Model and provenance", text: `${run.name} was estimated with ${run.method} using seed ${run.seed}, ${result.used_observations} observations, and fingerprint ${run.fingerprint}.` },
    { section: "Measurement model", text: `Outer loading magnitudes ranged from ${loadingRange}. Reliability and validity were reviewed using the documented QuickPLS assessment outputs.` },
    { section: "Structural model", text: `Path coefficients ranged from ${pathRange}${bestR2 ? `, and the strongest R² was ${bestR2[1].toFixed(4)} for ${bestR2[0]}` : ""}.` },
    { section: "Inference caveat", text: run.bootstrap || run.permutation ? "Inference should be reported with the selected resampling procedure, confidence level, seed, and any failed or unavailable intervals." : "This run does not include bootstrap or permutation inference; avoid p-value or confidence-interval claims from this run." },
    { section: "Scope status", text: scopeCopy(run.warnings[0]) },
  ];
}

function comparisonPathRows(a: PlsResult, b: PlsResult) {
  const bByPath = new Map(b.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient]));
  return a.paths.map((path) => {
    const key = pathLabel(path.source, path.target);
    const bValue = bByPath.get(key);
    return [key, path.coefficient.toFixed(6), bValue == null ? "N/A" : bValue.toFixed(6), bValue == null ? "not comparable" : (bValue - path.coefficient).toFixed(6)];
  });
}

function comparisonR2Rows(a: PlsResult, b: PlsResult) {
  const constructs = Array.from(new Set([...Object.keys(a.r_squared), ...Object.keys(b.r_squared)])).sort();
  return constructs.map((construct) => {
    const left = a.r_squared[construct];
    const right = b.r_squared[construct];
    return [construct, left == null ? "N/A" : left.toFixed(4), right == null ? "N/A" : right.toFixed(4), left == null || right == null ? "not comparable" : (right - left).toFixed(4)];
  });
}

function comparisonMeasurementRows(a: AssessmentResult, b: AssessmentResult) {
  const bByConstruct = new Map(b.construct_quality.map((row) => [row.construct, row]));
  return a.construct_quality.flatMap((row) => {
    const right = bByConstruct.get(row.construct);
    return [
      metricDelta(row.construct, "Cronbach alpha", row.cronbach_alpha, right?.cronbach_alpha),
      metricDelta(row.construct, "rho_A", row.rho_a, right?.rho_a),
      metricDelta(row.construct, "rho_C", row.rho_c, right?.rho_c),
      metricDelta(row.construct, "AVE", row.ave, right?.ave),
    ];
  });
}

function metricDelta(construct: string, metric: string, left: number | null | undefined, right: number | null | undefined) {
  return [construct, metric, formatOptional(left, 4), formatOptional(right, 4), left == null || right == null ? "not comparable" : (right - left).toFixed(4)];
}

function rangeText(values: number[]) {
  if (!values.length) return "not available";
  return `${Math.min(...values).toFixed(4)} to ${Math.max(...values).toFixed(4)}`;
}

function formatDiagnosticCode(code: string) {
  return code.replace(/^(rho_a|htmt)\./, "").replaceAll("_", " ");
}

function formatMediationClass(code: string) {
  return code.replaceAll("_", " ");
}

function formatPValue(value: number | null | undefined) {
  if (value == null) return "N/A";
  return value < 0.0001 ? "<0.0001" : value.toFixed(4);
}

function formatInterval(lower: number | null | undefined, upper: number | null | undefined) {
  if (lower == null || upper == null) return "N/A";
  return `${lower.toFixed(6)} to ${upper.toFixed(6)}`;
}

function ciZeroStatus(lower: number | null | undefined, upper: number | null | undefined) {
  if (lower == null || upper == null || !Number.isFinite(lower) || !Number.isFinite(upper)) return "unavailable";
  if (lower <= 0 && upper >= 0) return "CI includes zero";
  return "CI excludes zero";
}

function htmtPairStatus(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "unavailable";
  if (value >= 0.9) return "issue";
  if (value >= 0.85) return "review";
  return "ok";
}

function formatOptional(value: number | null | undefined, digits: number) {
  return value == null || !Number.isFinite(value) ? "N/A" : value.toFixed(digits);
}

function formatDisplayCell(value: string, digits: number) {
  if (!/^-?\d+\.\d{3,}$/.test(value)) return value;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed.toFixed(digits) : value;
}

function formatModeratorLevel(value: number) {
  if (value === -1) return "-1 SD";
  if (value === 0) return "Mean";
  if (value === 1) return "+1 SD";
  return value.toFixed(2);
}

function pathLabel(source: string, target: string) {
  return `${source} -> ${target}`;
}

function activeIndexes<T extends { source: string; target: string }>(rows: T[], activePath: { source: string; target: string } | null) {
  if (!activePath) return [];
  return rows.map((row, index) => row.source === activePath.source && row.target === activePath.target ? index : -1).filter((index) => index >= 0);
}

function coefficientDirection(value: number) {
  if (Math.abs(value) < 0.000001) return "near zero";
  return value > 0 ? "positive" : "negative";
}

function loadingStatus(value: number) {
  const absolute = Math.abs(value);
  if (absolute >= 0.708) return "strong";
  if (absolute >= 0.4) return "review";
  return "weak";
}

function vifStatus(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "N/A";
  if (value >= 5) return "high";
  if (value >= 3.3) return "review";
  return "acceptable";
}

function interpretR2(value: number) {
  if (value >= 0.75) return "substantial";
  if (value >= 0.5) return "moderate";
  if (value >= 0.25) return "weak to moderate";
  return "weak";
}

function interpretF2(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "N/A";
  if (value >= 0.35) return "large";
  if (value >= 0.15) return "medium";
  if (value >= 0.02) return "small";
  return "very small";
}

function reliabilityStatus(alpha: number | null, rhoC: number | null, ave: number | null) {
  const issues: string[] = [];
  if (alpha != null && alpha < 0.7) issues.push("alpha");
  if (rhoC != null && rhoC < 0.7) issues.push("rho_C");
  if (ave != null && ave < 0.5) issues.push("AVE");
  return issues.length ? `review ${issues.join(", ")}` : "passes common cutoffs";
}

function severityText(severity: InterpretationFinding["severity"]) {
  if (severity === "good") return "Good";
  if (severity === "issue") return "Issue";
  if (severity === "caution") return "Caution";
  if (severity === "unavailable") return "Unavailable";
  return "Info";
}

function scopeCopy(warning: string | undefined) {
  if (!warning) return "Validated for documented QuickPLS scope.";
  return warning.replace(/QuickPLS v\d+\.\d+\.\d+ supported scope/g, "documented QuickPLS supported scope");
}

function csvForCurrentResultTab(run: AnalysisRun, tab: ResultWorkspaceTab) {
  const result = run.result;
  const assessment = run.assessment;
  const rows: string[][] = [];
  if (!result) return "message\nNo result payload";
  if (tab === "measurement") {
    rows.push(["construct", "indicator", "loading", "weight"], ...result.outer_estimates.map((row) => [row.construct, row.indicator, row.loading.toString(), row.weight.toString()]));
  } else if (tab === "validity") {
    rows.push(["construct", "cronbach_alpha", "rho_a", "rho_c", "ave"], ...(assessment?.construct_quality ?? []).map((row) => [row.construct, String(row.cronbach_alpha ?? ""), String(row.rho_a ?? ""), String(row.rho_c ?? ""), String(row.ave ?? "")]));
  } else if (tab === "structural" || tab === "overview") {
    rows.push(["path", "coefficient"], ...result.paths.map((path) => [pathLabel(path.source, path.target), path.coefficient.toString()]));
  } else if (tab === "inference" && run.bootstrap) {
    rows.push(["parameter", "original", "mean", "bias", "se", "p"], ...run.bootstrap.percentile.parameters.map((parameter) => [formatParameterIdentity(parameter.parameter), String(parameter.original), String(parameter.bootstrap_mean), String(parameter.bias), String(parameter.standard_error), String(parameter.p_value_two_sided ?? "")]));
  } else if (tab === "prediction" && assessment?.blindfolding) {
    rows.push(["construct", "q2", "press", "sso"], ...assessment.blindfolding.constructs.map((row) => [row.construct, String(row.q_squared ?? ""), String(row.prediction_error_sum_squares ?? ""), String(row.observation_sum_squares ?? "")]));
  } else if (tab === "diagnostics") {
    rows.push(["field", "value"], ["method", run.method], ["seed", String(run.seed)], ["fingerprint", run.fingerprint], ["iterations", String(result.iterations)], ["observations", String(result.used_observations)]);
  } else if (tab === "interpretation") {
    rows.push(["section", "draft_wording"], ...reportWording(run).map((row) => [row.section, row.text]));
  } else if (tab === "comparison") {
    rows.push(["message"], ["Use the Comparison tab table-level copy controls for selected two-run comparison output."]);
  } else {
    rows.push(["message"], [`No exportable ${tab} table is available for this run.`]);
  }
  return rows.map((row) => row.map(csvCell).join(",")).join("\n");
}

function csvCell(value: string) {
  return /[",\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}

