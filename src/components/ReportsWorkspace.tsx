import { Download, ExternalLink, FileSpreadsheet, FileText, Image, Printer } from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { methods } from "../data/sample";
import { publicationDiagramSvg } from "../domain/publicationDiagram";
import { buildResultInterpretation } from "../domain/resultInterpretation";
import { runExportTables, tablesToCsv, tablesToHtml, type ResultTable } from "../domain/resultTables";
import { compareRuns } from "../domain/runComparison";
import { nativeLegacyProcessResultProjection, nativeProcessResultProjection } from "../native/nativeProcessResults";
import { nativeStructuralPathRandomizationProjection } from "../native/nativeStructuralPathRandomization";
import { exportNativeXlsxTables, isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import { Card, MethodConfidencePanel, MethodScopeDrawer, MetricCard, PageHeader, Panel, StatusBadge, WorkspacePage } from "./Ui";

type ReportPreset = "thesis" | "journal_figure" | "journal_tables" | "presentation" | "reviewer_pack" | "full_report";
type ReportExportTable = ReturnType<typeof runExportTables>[number];
type ReportWizardStep = "content" | "preview" | "settings" | "export";

export const PROCESS_GENERIC_DIAGRAM_UNAVAILABLE =
  "Generic model SVG is not applicable to PROCESS archives; this run has no archived method-specific diagram.";

export function reportableRuns(runs: readonly import("../types").AnalysisRun[]) {
  return runs.filter((run) => run.status === "completed" && Boolean(run.result));
}

export function reportScopeStatus(tables: readonly ResultTable[]): "validated" | "experimental" {
  return tables.length > 0 && tables.every((table) => table.status === "validated")
    ? "validated"
    : "experimental";
}

export function reportMethodId(
  run: import("../types").AnalysisRun | undefined,
  setupMethod: string,
): string {
  if (nativeStructuralPathRandomizationProjection(run)) return "permutation";
  return run?.provenance?.method ?? run?.method ?? setupMethod;
}

export function reportDiagramSvgForRun(
  run: import("../types").AnalysisRun | undefined,
  nodes: Parameters<typeof publicationDiagramSvg>[0],
  edges: Parameters<typeof publicationDiagramSvg>[1],
  settings: Parameters<typeof publicationDiagramSvg>[3],
  layout: Parameters<typeof publicationDiagramSvg>[4],
): string {
  if (run && (nativeProcessResultProjection(run) || nativeLegacyProcessResultProjection(run))) return "";
  if (!run) return publicationDiagramSvg(nodes, edges, run, settings, layout);
  const model = reportModelForRun(run, nodes, edges, layout);
  return publicationDiagramSvg(
    model.nodes,
    model.edges,
    run,
    nativeStructuralPathRandomizationProjection(run)
      ? { ...settings, showValidationWatermark: true }
      : settings,
    model.diagramLayout,
  );
}

export function reportModelForRun(
  run: import("../types").AnalysisRun | undefined,
  nodes: Parameters<typeof publicationDiagramSvg>[0],
  edges: Parameters<typeof publicationDiagramSvg>[1],
  layout: Parameters<typeof publicationDiagramSvg>[4],
) {
  return {
    nodes: run?.modelSnapshot?.nodes ?? nodes,
    edges: run?.modelSnapshot?.edges ?? edges,
    diagramLayout: run?.modelSnapshot?.diagramLayout ?? layout,
  };
}

export function ReportsWorkspace() {
  const runs = useWorkspace((state) => state.runs);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const analysisMethod = useWorkspace((state) => state.analysisSettings.method);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const setPublicationDiagramSettings = useWorkspace((state) => state.setPublicationDiagramSettings);
  const setView = useWorkspace((state) => state.setView);
  const setResultWorkspaceState = useWorkspace((state) => state.setResultWorkspaceState);
  const initialReportPreset = uiPreferences.selectedExportPreset === "thesis_appendix" ? "thesis" : uiPreferences.selectedExportPreset === "full_reproducibility_report" ? "full_report" : uiPreferences.selectedExportPreset;
  const completedRuns = useMemo(() => reportableRuns(runs), [runs]);
  const [selectedRunId, setSelectedRunId] = useState(completedRuns.at(0)?.id ?? "");
  const [selectedPreset, setSelectedPreset] = useState<ReportPreset>(initialReportPreset);
  const [includeInterpretationNotes, setIncludeInterpretationNotes] = useState(initialReportPreset === "reviewer_pack" || initialReportPreset === "full_report");
  const [lastExportMessage, setLastExportMessage] = useState<string | null>(null);
  const [activeWizardStep, setActiveWizardStep] = useState<ReportWizardStep>("content");
  const selectedRun = useMemo(() => completedRuns.find((run) => run.id === selectedRunId) ?? completedRuns.at(0), [completedRuns, selectedRunId]);
  const comparisonRun = useMemo(() => completedRuns.find((run) => run.id !== selectedRun?.id), [completedRuns, selectedRun?.id]);
  const tables = useMemo(() => selectedRun ? runExportTables(selectedRun) : [], [selectedRun]);
  const selectedScopeStatus = reportScopeStatus(tables);
  const processDiagramUnavailable = Boolean(selectedRun
    && (nativeProcessResultProjection(selectedRun) || nativeLegacyProcessResultProjection(selectedRun)));
  const archivedMethodId = reportMethodId(selectedRun, analysisMethod);
  const selectedMethod = useMemo(() => {
    if (!selectedRun) return undefined;
    const method = methods.find((candidate) => candidate.id === archivedMethodId);
    return method ? { ...method, status: selectedScopeStatus } : undefined;
  }, [archivedMethodId, selectedRun, selectedScopeStatus]);
  const reportModel = useMemo(
    () => reportModelForRun(selectedRun, nodes, edges, diagramLayout),
    [diagramLayout, edges, nodes, selectedRun],
  );
  const interpretation = useMemo(
    () => selectedRun ? buildResultInterpretation({ run: selectedRun, nodes: reportModel.nodes, edges: reportModel.edges }) : null,
    [reportModel.edges, reportModel.nodes, selectedRun],
  );
  const comparisonRows = useMemo(() => compareRuns(selectedRun, comparisonRun), [selectedRun, comparisonRun]);
  const diagramSvg = useMemo(
    () => reportDiagramSvgForRun(selectedRun, nodes, edges, publicationDiagramSettings, diagramLayout),
    [diagramLayout, edges, nodes, publicationDiagramSettings, selectedRun],
  );
  const tableExportDisabledReason = !tables.length ? "Run an available method before exporting result tables." : null;
  const xlsxDisabledReason = !tables.length ? "Run an available method before exporting XLSX." : !isNativeDesktop() ? "XLSX export requires the native desktop runtime." : null;
  const svgDisabledReason = processDiagramUnavailable
    ? PROCESS_GENERIC_DIAGRAM_UNAVAILABLE
    : !diagramSvg
      ? "Create a model diagram before exporting SVG."
      : null;
  const pdfDisabledReason = !tables.length ? "Run an available method before printing a report." : null;
  const outputTotal = processDiagramUnavailable ? 4 : 5;
  const readyOutputs = [
    !tableExportDisabledReason,
    !tableExportDisabledReason,
    !xlsxDisabledReason,
    !pdfDisabledReason,
    ...(processDiagramUnavailable ? [] : [!svgDisabledReason]),
  ].filter(Boolean).length;
  const previewRisk = processDiagramUnavailable
    ? PROCESS_GENERIC_DIAGRAM_UNAVAILABLE
    : publicationDiagramSettings.layoutSource === "current_canvas"
    ? "Preview uses current canvas layout. If labels overlap, switch to Tidy publication before export."
    : "Tidy publication layout is selected for cleaner figure export.";

  const download = (name: string, contents: string, type: string, label = "File") => {
    const url = URL.createObjectURL(new Blob([contents], { type }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = name;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
    setLastExportMessage(`${label} export started: ${name}`);
  };
  const printPdfReport = () => {
    const printable = window.open("", "_blank", "noopener,noreferrer");
    if (!printable) {
      setLastExportMessage("Print/PDF window was blocked by the browser runtime.");
      return;
    }
    printable.document.write(reportHtml(tablesToHtml(tables), includeInterpretationNotes ? interpretation : null));
    printable.document.close();
    printable.focus();
    printable.print();
    setLastExportMessage("Print/PDF dialog opened. Use the browser print dialog to save PDF.");
  };
  const exportXlsx = async () => {
    if (!isNativeDesktop()) return;
    await exportNativeXlsxTables(tables);
    setLastExportMessage("XLSX workbook export completed through the desktop runtime.");
  };

  const applyExportPreset = (preset: ReportPreset) => {
    setSelectedPreset(preset);
    const persistedPreset = preset === "thesis"
      ? "thesis_appendix"
      : preset === "full_report"
        ? "full_reproducibility_report"
        : preset === "presentation"
          ? "journal_figure"
          : preset;
    setUiPreferences({ selectedExportPreset: persistedPreset });
    if (preset === "journal_figure") setPublicationDiagramSettings({ mode: "smartpls_result", palette: "grayscale", precision: 3, layoutSource: "tidy_publication", showValidationWatermark: false });
    else if (preset === "presentation") setPublicationDiagramSettings({ mode: "smartpls_result", palette: "quickpls_color", precision: 2, layoutSource: "current_canvas", showValidationWatermark: true });
    else if (preset === "thesis") setPublicationDiagramSettings({ mode: "publication", palette: "grayscale", precision: 4, layoutSource: "current_canvas", showValidationWatermark: true, showRunProvenance: true });
    else if (preset === "reviewer_pack") {
      setPublicationDiagramSettings({ mode: "publication", palette: "high_contrast", precision: 4, layoutSource: "current_canvas", showValidationWatermark: true, showRunProvenance: true });
      setIncludeInterpretationNotes(true);
    }
    else if (preset === "full_report") {
      setPublicationDiagramSettings({ mode: "publication", palette: "high_contrast", precision: 4, layoutSource: "current_canvas", showValidationWatermark: true, showRunProvenance: true });
      setIncludeInterpretationNotes(true);
    }
    else setPublicationDiagramSettings({ precision: 4, showLoadings: true, showPathCoefficients: true, showRSquared: true });
    setLastExportMessage(null);
  };
  const openResultsComparison = () => {
    const ids = [selectedRun?.id, comparisonRun?.id].filter(Boolean) as string[];
    setResultWorkspaceState({ selectedTab: "comparison", comparisonRunIds: ids });
    setView("runs");
  };

  useEffect(() => {
    const selectRun = () => {
      setActiveWizardStep("settings");
      setTimeout(() => document.querySelector<HTMLSelectElement>(".report-settings-section select")?.focus(), 0);
    };
    const preview = () => setActiveWizardStep("preview");
    const exportSvg = () => {
      if (!svgDisabledReason) download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml", "SVG diagram");
    };
    const exportTables = () => {
      if (!tableExportDisabledReason) {
        setActiveWizardStep("export");
        download("quickpls-result-tables.csv", tablesToCsv(tables), "text/csv", "CSV tables");
      }
    };
    const exportWorkbook = () => {
      if (!xlsxDisabledReason) void exportXlsx().catch((error) => window.alert(error));
    };
    const printReport = () => {
      if (!pdfDisabledReason) printPdfReport();
    };
    window.addEventListener("quickpls:report-select-run", selectRun);
    window.addEventListener("quickpls:report-preview", preview);
    window.addEventListener("quickpls:report-export-svg", exportSvg);
    window.addEventListener("quickpls:report-export-tables", exportTables);
    window.addEventListener("quickpls:report-export-workbook", exportWorkbook);
    window.addEventListener("quickpls:report-print", printReport);
    return () => {
      window.removeEventListener("quickpls:report-select-run", selectRun);
      window.removeEventListener("quickpls:report-preview", preview);
      window.removeEventListener("quickpls:report-export-svg", exportSvg);
      window.removeEventListener("quickpls:report-export-tables", exportTables);
      window.removeEventListener("quickpls:report-export-workbook", exportWorkbook);
      window.removeEventListener("quickpls:report-print", printReport);
    };
  }, [diagramSvg, svgDisabledReason, tableExportDisabledReason, tables, xlsxDisabledReason, pdfDisabledReason, includeInterpretationNotes]);

  return <WorkspacePage className="publication-workspace report-v2-workspace report-v213-workspace report-v2100-workspace report-v219-workspace report-v2310-workspace" data-report-export-flow="v2.31" data-v219-mockup-screen="report" data-v2310-report-wizard="true">
    <PageHeader title="Publication report" description="Assemble figure, tables, provenance, and optional interpretation notes from one completed result-backed run." actions={<StatusBadge status={selectedRun ? selectedScopeStatus : "warning"}>{selectedRun ? selectedScopeStatus === "validated" ? "Validated run" : "Candidate run" : "model-only"}</StatusBadge>} />
    <MethodScopeDrawer method={selectedMethod} open={uiPreferences.methodScopeDrawerOpen} onToggle={() => setUiPreferences({ methodScopeDrawerOpen: !uiPreferences.methodScopeDrawerOpen })} />
    <ReportWizardNav active={activeWizardStep} setActive={setActiveWizardStep} selectedRun={Boolean(selectedRun)} hasPreview={Boolean(diagramSvg || tables.length)} hasExports={Boolean(tables.length || diagramSvg)} />
    <Panel title="Report package" description="Selected preset, run, tables, figure, and export readiness." className="report-v2-hero report-v213-hero">
      <div className="report-v2-hero-copy">
        <span>Report package</span>
        <strong>{selectedPresetLabel(selectedPreset)}</strong>
        <p>{selectedRun ? processDiagramUnavailable
          ? `${selectedRun.name} is selected with ${tables.length} result table(s) and ${readyOutputs} ready export output(s); a generic live-canvas SVG is not applicable to this PROCESS run.`
          : `${selectedRun.name} is selected with ${tables.length} result table(s), ${readyOutputs} ready export output(s), and ${publicationDiagramSettings.precision}-decimal figure labels.`
          : "Select or run an analysis to unlock result tables and full report exports."}</p>
      </div>
      <div className="report-v2-hero-metrics" aria-label="Report export readiness">
        <MetricCard label="Run" value={selectedRun ? "Selected" : "Missing"} detail={selectedRun?.method ?? "Run a method first"} tone={selectedRun ? "success" : "warning"} />
        <MetricCard label="Tables" value={tables.length} detail={tables.length ? "export table(s)" : "not available"} tone={tables.length ? "success" : "warning"} />
        <MetricCard label="Figure" value={processDiagramUnavailable ? "Not applicable" : svgDisabledReason ? "Model only" : "SVG ready"} detail={processDiagramUnavailable ? "No archived PROCESS-specific SVG" : publicationDiagramSettings.layoutSource.replace("_", " ")} tone={svgDisabledReason ? "warning" : "success"} />
        <MetricCard label="Outputs" value={`${readyOutputs}/${outputTotal}`} detail={processDiagramUnavailable ? "CSV, HTML, XLSX, PDF path" : "CSV, HTML, XLSX, PDF path, SVG"} />
      </div>
    </Panel>
    {activeWizardStep === "content" ? <section className="report-wizard-pane" data-v2310-wizard-step="content" aria-label="Select report content">
    <Panel title="Choose export preset" description="Presets only change presentation defaults; result values remain unchanged." actions={<StatusBadge status={includeInterpretationNotes ? "info" : "validated"}>{includeInterpretationNotes ? "interpretation included" : "numeric by default"}</StatusBadge>} className="report-v2-command-center report-v213-command-center">
      <div className="report-v2-command-heading">
        <div><strong>Choose export preset</strong><span>Presets only change presentation defaults; result values remain unchanged.</span></div>
        <StatusBadge status={includeInterpretationNotes ? "info" : "validated"}>{includeInterpretationNotes ? "interpretation included" : "numeric by default"}</StatusBadge>
      </div>
      <div className="report-preset-panel" aria-label="Report export presets">
        {[
          ["thesis", "Thesis appendix", "Detailed tables, provenance, and validation footer."],
          ["journal_figure", "Journal figure", "Clean grayscale SVG diagram for manuscript figures."],
          ["journal_tables", "Journal tables", "Numerical tables with publication-safe precision."],
          ["presentation", "Presentation", "Color figure and compact table settings."],
          ["reviewer_pack", "Reviewer pack", "Scope, fingerprints, known limits, validation index, tables, and diagram."],
          ["full_report", "Full reproducibility report", "Tables, provenance, and interpretation notes."],
        ].map(([id, label, detail]) => <button key={id} className={selectedPreset === id ? "report-preset-card active" : "report-preset-card"} onClick={() => applyExportPreset(id as ReportPreset)}>
          <strong>{label}</strong><span>{detail}</span>
        </button>)}
      </div>
      <ol className="export-stepper report-stepper" aria-label="Publication export steps">
        <li className={selectedRun ? "complete" : "active"}><b>1</b><span>Select run</span></li>
        <li className="complete"><b>2</b><span>Choose preset</span></li>
        <li className={(processDiagramUnavailable || diagramSvg) && tables.length ? "complete" : "active"}><b>3</b><span>{processDiagramUnavailable ? "Review tables" : "Review figure and tables"}</span></li>
        <li className={tables.length || diagramSvg ? "complete" : ""}><b>4</b><span>Export package</span></li>
      </ol>
    </Panel>
    {selectedRun ? <Panel title="Method confidence" description="Selected run provenance and documented scope status." className="report-v2-confidence report-v213-confidence"><MethodConfidencePanel run={selectedRun} /></Panel> : null}
    <div className="report-status-row" aria-label="Publication readiness summary">
      <Card title="Figure" description={processDiagramUnavailable ? PROCESS_GENERIC_DIAGRAM_UNAVAILABLE : "SVG is the audited publication diagram format."} tone={processDiagramUnavailable ? "warning" : "validated"} />
      <Card title="Tables" description={tables.length ? `${tables.length} export table(s) available for this run.` : "Run a method before exporting result tables."} tone={tables.length ? "validated" : "warning"} />
      <Card title="Print / PDF" description="Use the guided browser print path; native PDF remains post-v1.5 unless separately audited." />
      <Card title="Comparison" description={comparisonRows.length ? "Run comparison stays in Results; Report links to it." : "Add a second compatible run for comparison."} tone={comparisonRows.length ? "validated" : undefined} />
    </div>
    <div className="report-wizard-footer"><button className="primary-button" type="button" onClick={() => setActiveWizardStep("preview")}>Next: Preview figure and tables</button></div>
    </section> : null}
    {activeWizardStep === "settings" ? <section className="report-wizard-pane" data-v2310-wizard-step="settings" aria-label="Document settings">
    <Panel title="Publication setup" description="Figure, table, notes, and reviewer-pack settings." className="report-settings-shell report-v213-settings">
      <div className="report-settings-section">
        <h3>Figure settings</h3>
        <label>Saved run<select value={selectedRun?.id ?? ""} onChange={(event) => setSelectedRunId(event.target.value)} disabled={!completedRuns.length}>
          {completedRuns.length ? completedRuns.map((run) => <option key={run.id} value={run.id}>{run.name}</option>) : <option>No completed result-backed runs</option>}
        </select></label>
        {processDiagramUnavailable ? <p className="disabled-reason inline-disabled-reason">Figure controls hidden: {PROCESS_GENERIC_DIAGRAM_UNAVAILABLE}</p> : <>
          <label>Diagram style<select value={publicationDiagramSettings.mode} onChange={(event) => setPublicationDiagramSettings({ mode: event.target.value as typeof publicationDiagramSettings.mode })}>
          <option value="smartpls_result">SmartPLS-like</option>
          <option value="publication">QuickPLS publication</option>
          <option value="sem">SEM diagram</option>
          <option value="compact">Compact</option>
        </select></label>
        <label>Diagram palette<select value={publicationDiagramSettings.palette} onChange={(event) => setPublicationDiagramSettings({ palette: event.target.value as typeof publicationDiagramSettings.palette })}>
          <option value="grayscale">Grayscale</option>
          <option value="high_contrast">High contrast</option>
          <option value="quickpls_color">QuickPLS color</option>
        </select></label>
        <label>Diagram layout<select value={publicationDiagramSettings.layoutSource} onChange={(event) => setPublicationDiagramSettings({ layoutSource: event.target.value as typeof publicationDiagramSettings.layoutSource })}>
          <option value="current_canvas">Current canvas</option>
          <option value="tidy_publication">Tidy publication</option>
        </select></label>
        </>}
      </div>
      <div className="report-settings-section">
        <h3>Table settings</h3>
        <label>Precision<select value={publicationDiagramSettings.precision} onChange={(event) => setPublicationDiagramSettings({ precision: Number(event.target.value) })}>
          {[2, 3, 4, 5, 6].map((value) => <option key={value} value={value}>{value} decimals</option>)}
        </select></label>
        {!processDiagramUnavailable ? <>
          <label className="checkbox-row compact-check">Loadings<input type="checkbox" checked={publicationDiagramSettings.showLoadings} onChange={(event) => setPublicationDiagramSettings({ showLoadings: event.target.checked })} /></label>
          <label className="checkbox-row compact-check">Path coefficients<input type="checkbox" checked={publicationDiagramSettings.showPathCoefficients} onChange={(event) => setPublicationDiagramSettings({ showPathCoefficients: event.target.checked })} /></label>
          <label className="checkbox-row compact-check">R<sup>2</sup><input type="checkbox" checked={publicationDiagramSettings.showRSquared} onChange={(event) => setPublicationDiagramSettings({ showRSquared: event.target.checked })} /></label>
        </> : null}
      </div>
      <div className="report-settings-section">
        <h3>Notes and interpretation</h3>
        <p>{tables.length ? "CSV, HTML, desktop XLSX, and print report actions are available below." : "Run a method before table export becomes available."}</p>
        {tableExportDisabledReason ? <p className="disabled-reason inline-disabled-reason">Table exports disabled: {tableExportDisabledReason}</p> : null}
      </div>
      <div className="report-settings-section">
        <h3>Provenance and reviewer pack</h3>
        <label className="checkbox-row compact-check">Include interpretation notes<input type="checkbox" checked={includeInterpretationNotes} onChange={(event) => setIncludeInterpretationNotes(event.target.checked)} /></label>
        <p>{selectedPreset === "reviewer_pack" || selectedPreset === "full_report" ? "This preset includes interpretation notes by default for reviewer context." : "Interpretation notes are deterministic QuickPLS guidance and are included only when explicitly selected."}</p>
      </div>
    </Panel>
    <Panel title="Export review" description={`${selectedPresetLabel(selectedPreset)} preset with ${publicationDiagramSettings.palette.replaceAll("_", " ")} palette and ${publicationDiagramSettings.precision} decimal precision.`} className="report-export-review report-v213-export-review">
      <div><strong>Export review</strong><span>{selectedPresetLabel(selectedPreset)} preset with {publicationDiagramSettings.palette.replaceAll("_", " ")} palette and {publicationDiagramSettings.precision} decimal precision.</span></div>
      <ul>
        <li>{selectedRun ? `Run: ${selectedRun.name}` : "No run selected; diagram export is model-only."}</li>
        {processDiagramUnavailable ? <li>{PROCESS_GENERIC_DIAGRAM_UNAVAILABLE}</li> : null}
        <li>{tables.length ? `${tables.length} table(s) will be available for CSV/HTML/XLSX export.` : "No result tables are available yet."}</li>
        <li>{includeInterpretationNotes ? "Interpretation notes will be included in HTML/reviewer-style outputs." : "Numeric exports stay clean; interpretation notes are not included."}</li>
      </ul>
      {lastExportMessage ? <p className="export-status-feedback" role="status">{lastExportMessage}</p> : null}
    </Panel>
    {selectedPreset === "reviewer_pack" ? <section className="reviewer-pack-preview" aria-label="Reviewer pack preview">
      <header><strong>Reviewer pack contents</strong><span>Designed for transparent method review, not for unsupported equivalence claims.</span></header>
      <ul>
        <li>Method scope statement, validation status, and known limitations.</li>
        <li>Data fingerprint, recipe fingerprint, seed, worker count, and run provenance.</li>
        <li>{processDiagramUnavailable ? "Warnings, known differences, validation artifact index references, and result tables; no generic live-canvas diagram." : "Warnings, known differences, validation artifact index references, result tables, and publication diagram."}</li>
        <li>Interpretation notes only because Reviewer Pack explicitly opts in.</li>
      </ul>
    </section> : null}
    <div className="report-wizard-footer"><button className="secondary-button" type="button" onClick={() => setActiveWizardStep("preview")}>Back: Preview</button><button className="primary-button" type="button" onClick={() => setActiveWizardStep("export")}>Next: Export package</button></div>
    </section> : null}
    {activeWizardStep === "preview" ? <section className="report-wizard-pane" data-v2310-wizard-step="preview" aria-label="Report preview">
    {processDiagramUnavailable ? <div className="method-note wide" data-process-report-diagram="not-applicable"><strong>PROCESS diagram not available</strong><p>{PROCESS_GENERIC_DIAGRAM_UNAVAILABLE}</p></div> : <Panel title="Publication diagram preview" description={selectedRun ? "WYSIWYG SVG export with selected run overlays" : "Model-only SVG preview until a result is selected"} actions={<button className="secondary-button" disabled={Boolean(svgDisabledReason)} title={svgDisabledReason ?? "Export the visible publication SVG"} onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml", "SVG diagram")}><Image size={16} /> Export SVG</button>} className="publication-preview-shell report-v2-preview-shell report-v213-preview">
      <div className="publication-preview-heading">
        <div><strong>Publication diagram preview</strong><span>{selectedRun ? "WYSIWYG SVG export with selected run overlays" : "Model-only SVG preview until a result is selected"}</span></div>
        <div className="publication-preview-actions">
          <span className={publicationDiagramSettings.layoutSource === "tidy_publication" ? "status-text validated" : "status-text warning"}>{publicationDiagramSettings.layoutSource === "tidy_publication" ? "tidy layout" : "current canvas"}</span>
          <button className="secondary-button" disabled={Boolean(svgDisabledReason)} title={svgDisabledReason ?? "Export the visible publication SVG"} onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml", "SVG diagram")}><Image size={16} /> Export SVG</button>
        </div>
      </div>
      <p className={publicationDiagramSettings.layoutSource === "tidy_publication" ? "preview-guidance validated" : "preview-guidance warning"}>{previewRisk}</p>
      <div className="diagram-preview publication-preview-frame" aria-label="Publication diagram preview" dangerouslySetInnerHTML={{ __html: diagramSvg }} />
    </Panel>}
    {!tables.length ? <div className="method-note wide"><strong>Table preview unavailable</strong><p>Run an available method before exporting result tables. The diagram preview can still be reviewed as a model figure.</p></div> : <div className="report-preview report-v2100-table-preview-list">
      {tables.map((table) => <ReportTablePreview key={table.id} table={table} onStatus={setLastExportMessage} />)}
    </div>}
    <div className="report-wizard-footer"><button className="secondary-button" type="button" onClick={() => setActiveWizardStep("content")}>Back: Content</button><button className="primary-button" type="button" onClick={() => setActiveWizardStep("settings")}>Next: Document settings</button></div>
    </section> : null}
    {activeWizardStep === "export" ? <section className="report-wizard-pane" data-v2310-wizard-step="export" aria-label="Export report package">
    <Panel title="Export outputs" description="Each action shows its enabled state and exact disabled reason." className="report-export-actions report-v2-export-actions report-v213-export-actions">
      <ExportAction icon={<FileSpreadsheet />} title="CSV tables" detail={tables.length ? "Provenance and method tables" : "Run a method before CSV export"} disabledReason={tableExportDisabledReason} onClick={() => download("quickpls-result-tables.csv", tablesToCsv(tables), "text/csv", "CSV tables")} />
      <ExportAction icon={<FileText />} title="HTML report" detail={tables.length ? (includeInterpretationNotes ? "Tables plus interpretation notes" : "Watermarked table report") : "Run a method before HTML export"} disabledReason={tableExportDisabledReason} onClick={() => download("quickpls-result-report.html", reportHtml(tablesToHtml(tables), includeInterpretationNotes ? interpretation : null), "text/html", "HTML report")} />
      <ExportAction icon={<FileSpreadsheet />} title="XLSX workbook" detail={isNativeDesktop() ? "Desktop workbook export" : "Desktop runtime required"} disabledReason={xlsxDisabledReason} onClick={() => { void exportXlsx().catch((error) => window.alert(error)); }} />
      <ExportAction icon={<Printer />} title="Print / PDF" detail="Open browser print dialog for PDF output" disabledReason={pdfDisabledReason} onClick={printPdfReport} />
      {!processDiagramUnavailable ? <ExportAction icon={<Image />} title="Model diagram SVG" detail="WYSIWYG publication figure" disabledReason={svgDisabledReason} onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml", "SVG diagram")} /> : null}
    </Panel>
    <Panel title="Run comparison" description={comparisonRows.length ? `${comparisonRows.length} comparable metric rows are available in Results.` : "Comparison needs two compatible completed runs."} className="report-comparison-link report-v213-comparison">
      <div>
        <strong>Run comparison</strong>
        <span>{comparisonRows.length ? `${comparisonRows.length} comparable metric rows are available in Results.` : "Comparison needs two compatible completed runs."}</span>
      </div>
      <button className="secondary-button" disabled={!comparisonRows.length} title={comparisonRows.length ? "Open Results comparison workspace" : "Create a second compatible run before comparing"} onClick={openResultsComparison}><ExternalLink size={15} /> Open Results Comparison</button>
    </Panel>
    {!tables.length ? <div className="method-note wide"><strong>Export gate</strong><p>Run an available method before exporting result tables. Stable publication exports remain gated until the relevant method family is validated.</p></div> : null}
    <div className="report-wizard-footer"><button className="secondary-button" type="button" onClick={() => setActiveWizardStep("settings")}>Back: Settings</button></div>
    </section> : null}
  </WorkspacePage>;
}

function ReportWizardNav({ active, setActive, selectedRun, hasPreview, hasExports }: { active: ReportWizardStep; setActive: (step: ReportWizardStep) => void; selectedRun: boolean; hasPreview: boolean; hasExports: boolean }) {
  const steps: Array<{ id: ReportWizardStep; label: string; detail: string; ready: boolean }> = [
    { id: "content", label: "Select content", detail: selectedRun ? "Run and preset selected" : "Choose a run or model-only package", ready: selectedRun },
    { id: "preview", label: "Preview", detail: hasPreview ? "Figure and tables available" : "Review model-only preview", ready: hasPreview },
    { id: "settings", label: "Document settings", detail: "Figure, tables, notes, provenance", ready: true },
    { id: "export", label: "Export", detail: hasExports ? "Choose output files" : "Outputs need a run/model", ready: hasExports },
  ];
  return <nav className="report-wizard-nav" aria-label="Report export wizard steps">
    {steps.map((step, index) => <button key={step.id} type="button" className={active === step.id ? "active" : step.ready ? "complete" : ""} onClick={() => setActive(step.id)} data-v2310-step-button={step.id}>
      <b>{index + 1}</b>
      <span><strong>{step.label}</strong><small>{step.detail}</small></span>
    </button>)}
  </nav>;
}

function ExportAction({ icon, title, detail, disabledReason, onClick }: { icon: ReactNode; title: string; detail: string; disabledReason: string | null; onClick: () => void }) {
  return <button className="report-export-action" disabled={Boolean(disabledReason)} title={disabledReason ?? title} onClick={onClick}>
    {icon}
    <span><strong>{title}</strong><small>{detail}</small>{disabledReason ? <em>{disabledReason}</em> : null}</span>
    <Download size={14} className="export-action-cue" />
  </button>;
}

function ReportTablePreview({ table, onStatus }: { table: ReportExportTable; onStatus: (message: string) => void }) {
  const exportFileStem = table.title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || table.id;
  const tableBody = [table.columns, ...table.rows];
  const copyTable = async () => {
    await navigator.clipboard?.writeText(tableBody.map((row) => row.join("\t")).join("\n"));
    onStatus(`Copied preview table: ${table.title}`);
  };
  const exportReportTable = () => {
    const csv = tableBody.map((row) => row.map(csvCell).join(",")).join("\n");
    const url = URL.createObjectURL(new Blob([csv], { type: "text/csv" }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `quickpls-${exportFileStem}.csv`;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
    onStatus(`Table preview export started: quickpls-${exportFileStem}.csv`);
  };
  return <article className="report-table-preview research-table-shell v2100-report-table-preview" data-report-table-preview="v2.10">
    <header className="v2100-report-table-header">
      <div>
        <strong>{table.title}</strong>
        <span>{table.rows.length} row(s), {table.columns.length} column(s). First column stays pinned for wide review.</span>
      </div>
      <div className="v2100-report-table-actions">
        <span className={`status-text ${table.status}`}>{table.status}</span>
        <button type="button" className="secondary-button" onClick={() => void copyTable()}>Copy table</button>
        <button type="button" className="secondary-button" onClick={exportReportTable}>Export table</button>
      </div>
    </header>
    {table.warning ? <p className="v2100-report-table-warning">{table.warning}</p> : null}
    <div className="v2100-report-table-meta">
      <span>Preview matches CSV/HTML/XLSX table values for this selected run.</span>
      <span>{table.columns.length > 6 ? "Wide output: use horizontal scroll or export this table." : "Compact output: all core columns are visible."}</span>
    </div>
    <div className="bootstrap-table-scroll report-v2100-table-scroll" tabIndex={0} role="region" aria-label={`${table.title} table`}>
      <table>
        <caption>{table.title} export preview</caption>
        <thead><tr>{table.columns.map((column, index) => <th key={column} className={index === 0 ? "sticky-col" : undefined}>{column}</th>)}</tr></thead>
        <tbody>
          {table.rows.map((row, rowIndex) => <tr key={rowIndex}>{row.map((cell, columnIndex) => <td key={`${rowIndex}-${columnIndex}`} className={columnIndex === 0 ? "sticky-col" : undefined}>{cell}</td>)}</tr>)}
        </tbody>
      </table>
    </div>
  </article>;
}

function selectedPresetLabel(preset: ReportPreset) {
  if (preset === "thesis") return "Thesis appendix";
  if (preset === "journal_figure") return "Journal figure";
  if (preset === "journal_tables") return "Journal tables";
  if (preset === "presentation") return "Presentation";
  if (preset === "reviewer_pack") return "Reviewer pack";
  return "Full reproducibility report";
}

function reportHtml(baseHtml: string, interpretation: ReturnType<typeof buildResultInterpretation> | null) {
  if (!interpretation) return baseHtml;
  const notes = interpretation.reportParagraphs.map((paragraph) => `<section><h2>${escapeHtml(paragraph.section)}</h2><p>${escapeHtml(paragraph.text)}</p></section>`).join("");
  return baseHtml.replace("</body>", `<section><h1>Interpretation notes</h1>${notes}<p><small>Interpretation notes are deterministic QuickPLS guidance based on available result values and documented scope; they are not AI-generated and do not establish causality.</small></p></section></body>`);
}

function escapeHtml(value: string) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll('"', "&quot;");
}

function csvCell(value: string) {
  return /[",\n]/.test(value) ? `"${value.replaceAll('"', '""')}"` : value;
}
