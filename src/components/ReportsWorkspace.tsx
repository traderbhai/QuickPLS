import { Download, ExternalLink, FileSpreadsheet, FileText, Image, Printer } from "lucide-react";
import { useMemo, useState, type ReactNode } from "react";
import { methods } from "../data/sample";
import { publicationDiagramSvg } from "../domain/publicationDiagram";
import { buildResultInterpretation } from "../domain/resultInterpretation";
import { runExportTables, tablesToCsv, tablesToHtml } from "../domain/resultTables";
import { compareRuns } from "../domain/runComparison";
import { exportNativeXlsxTables, isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import { Card, MethodConfidencePanel, MethodScopeDrawer, PageHeader, StatusBadge } from "./Ui";

type ReportPreset = "thesis" | "journal_figure" | "journal_tables" | "presentation" | "reviewer_pack" | "full_report";

export function ReportsWorkspace() {
  const runs = useWorkspace((state) => state.runs);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const analysisMethod = useWorkspace((state) => state.analysisSettings.method);
  const selectedMethod = methods.find((method) => method.id === analysisMethod);
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const setPublicationDiagramSettings = useWorkspace((state) => state.setPublicationDiagramSettings);
  const setView = useWorkspace((state) => state.setView);
  const setResultWorkspaceState = useWorkspace((state) => state.setResultWorkspaceState);
  const initialReportPreset = uiPreferences.selectedExportPreset === "thesis_appendix" ? "thesis" : uiPreferences.selectedExportPreset === "full_reproducibility_report" ? "full_report" : uiPreferences.selectedExportPreset;
  const [selectedRunId, setSelectedRunId] = useState(runs.at(0)?.id ?? "");
  const [selectedPreset, setSelectedPreset] = useState<ReportPreset>(initialReportPreset);
  const [includeInterpretationNotes, setIncludeInterpretationNotes] = useState(initialReportPreset === "reviewer_pack" || initialReportPreset === "full_report");
  const [lastExportMessage, setLastExportMessage] = useState<string | null>(null);
  const selectedRun = useMemo(() => runs.find((run) => run.id === selectedRunId) ?? runs.at(0), [runs, selectedRunId]);
  const comparisonRun = useMemo(() => runs.find((run) => run.id !== selectedRun?.id), [runs, selectedRun?.id]);
  const tables = useMemo(() => selectedRun ? runExportTables(selectedRun) : [], [selectedRun]);
  const interpretation = useMemo(() => selectedRun ? buildResultInterpretation({ run: selectedRun, nodes, edges }) : null, [edges, nodes, selectedRun]);
  const comparisonRows = useMemo(() => compareRuns(selectedRun, comparisonRun), [selectedRun, comparisonRun]);
  const diagramSvg = useMemo(() => publicationDiagramSvg(nodes, edges, selectedRun, publicationDiagramSettings, diagramLayout), [diagramLayout, edges, nodes, publicationDiagramSettings, selectedRun]);
  const tableExportDisabledReason = !tables.length ? "Run an available method before exporting result tables." : null;
  const xlsxDisabledReason = !tables.length ? "Run an available method before exporting XLSX." : !isNativeDesktop() ? "XLSX export requires the native desktop runtime." : null;
  const svgDisabledReason = !diagramSvg ? "Create a model diagram before exporting SVG." : null;
  const pdfDisabledReason = !tables.length ? "Run an available method before printing a report." : null;
  const previewRisk = publicationDiagramSettings.layoutSource === "current_canvas"
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

  return <section className="workspace-page publication-workspace">
    <PageHeader title="Publication report" description="Preview the exact diagram style, export tables, and preserve validation status, warnings, and run provenance." actions={<StatusBadge status={selectedRun ? "validated" : "warning"}>{selectedRun ? "run selected" : "model-only"}</StatusBadge>} />
    <MethodScopeDrawer method={selectedMethod} open={uiPreferences.methodScopeDrawerOpen} onToggle={() => setUiPreferences({ methodScopeDrawerOpen: !uiPreferences.methodScopeDrawerOpen })} />
    <section className="report-preset-panel" aria-label="Report export presets">
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
    </section>
    {selectedRun ? <MethodConfidencePanel run={selectedRun} /> : null}
    <div className="report-status-row" aria-label="Publication readiness summary">
      <Card title="Figure" description="SVG is the audited publication diagram format." tone="validated" />
      <Card title="Tables" description={tables.length ? `${tables.length} export table(s) available for this run.` : "Run a method before exporting result tables."} tone={tables.length ? "validated" : "warning"} />
      <Card title="Print / PDF" description="Use the guided browser print path; native PDF remains post-v1.5 unless separately audited." />
      <Card title="Comparison" description={comparisonRows.length ? "Run comparison stays in Results; Report links to it." : "Add a second compatible run for comparison."} tone={comparisonRows.length ? "validated" : undefined} />
    </div>
    <ol className="export-stepper report-stepper" aria-label="Publication export steps">
      <li className={selectedRun ? "complete" : "active"}><b>1</b><span>Select run</span></li>
      <li className="complete"><b>2</b><span>Choose preset</span></li>
      <li className={diagramSvg && tables.length ? "complete" : "active"}><b>3</b><span>Review figure and table preview</span></li>
      <li className={tables.length || diagramSvg ? "complete" : ""}><b>4</b><span>Export</span></li>
    </ol>
    <section className="report-settings-shell" aria-label="Publication setup">
      <div className="report-settings-section">
        <h3>Figure settings</h3>
        <label>Saved run<select value={selectedRun?.id ?? ""} onChange={(event) => setSelectedRunId(event.target.value)} disabled={!runs.length}>
          {runs.length ? runs.map((run) => <option key={run.id} value={run.id}>{run.name}</option>) : <option>No saved runs</option>}
        </select></label>
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
      </div>
      <div className="report-settings-section">
        <h3>Table settings</h3>
        <label>Precision<select value={publicationDiagramSettings.precision} onChange={(event) => setPublicationDiagramSettings({ precision: Number(event.target.value) })}>
          {[2, 3, 4, 5, 6].map((value) => <option key={value} value={value}>{value} decimals</option>)}
        </select></label>
        <label className="checkbox-row compact-check">Loadings<input type="checkbox" checked={publicationDiagramSettings.showLoadings} onChange={(event) => setPublicationDiagramSettings({ showLoadings: event.target.checked })} /></label>
        <label className="checkbox-row compact-check">Path coefficients<input type="checkbox" checked={publicationDiagramSettings.showPathCoefficients} onChange={(event) => setPublicationDiagramSettings({ showPathCoefficients: event.target.checked })} /></label>
        <label className="checkbox-row compact-check">R<sup>2</sup><input type="checkbox" checked={publicationDiagramSettings.showRSquared} onChange={(event) => setPublicationDiagramSettings({ showRSquared: event.target.checked })} /></label>
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
    </section>
    <section className="report-export-review" aria-label="Review selected report outputs before export">
      <div><strong>Export review</strong><span>{selectedPresetLabel(selectedPreset)} preset with {publicationDiagramSettings.palette.replaceAll("_", " ")} palette and {publicationDiagramSettings.precision} decimal precision.</span></div>
      <ul>
        <li>{selectedRun ? `Run: ${selectedRun.name}` : "No run selected; diagram export is model-only."}</li>
        <li>{tables.length ? `${tables.length} table(s) will be available for CSV/HTML/XLSX export.` : "No result tables are available yet."}</li>
        <li>{includeInterpretationNotes ? "Interpretation notes will be included in HTML/reviewer-style outputs." : "Numeric exports stay clean; interpretation notes are not included."}</li>
      </ul>
      {lastExportMessage ? <p className="export-status-feedback" role="status">{lastExportMessage}</p> : null}
    </section>
    {selectedPreset === "reviewer_pack" ? <section className="reviewer-pack-preview" aria-label="Reviewer pack preview">
      <header><strong>Reviewer pack contents</strong><span>Designed for transparent method review, not for unsupported equivalence claims.</span></header>
      <ul>
        <li>Method scope statement, validation status, and known limitations.</li>
        <li>Data fingerprint, recipe fingerprint, seed, worker count, and run provenance.</li>
        <li>Warnings, known differences, validation artifact index references, result tables, and publication diagram.</li>
        <li>Interpretation notes only because Reviewer Pack explicitly opts in.</li>
      </ul>
    </section> : null}
    <div className="publication-preview-shell">
      <div className="publication-preview-heading">
        <div><strong>Publication diagram preview</strong><span>{selectedRun ? "WYSIWYG SVG export with selected run overlays" : "Model-only SVG preview until a result is selected"}</span></div>
        <div className="publication-preview-actions">
          <span className={publicationDiagramSettings.layoutSource === "tidy_publication" ? "status-text validated" : "status-text warning"}>{publicationDiagramSettings.layoutSource === "tidy_publication" ? "tidy layout" : "current canvas"}</span>
          <button className="secondary-button" disabled={Boolean(svgDisabledReason)} title={svgDisabledReason ?? "Export the visible publication SVG"} onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml", "SVG diagram")}><Image size={16} /> Export SVG</button>
        </div>
      </div>
      <p className={publicationDiagramSettings.layoutSource === "tidy_publication" ? "preview-guidance validated" : "preview-guidance warning"}>{previewRisk}</p>
      <div className="diagram-preview publication-preview-frame" aria-label="Publication diagram preview" dangerouslySetInnerHTML={{ __html: diagramSvg }} />
    </div>
    <section className="report-export-actions" aria-label="Export outputs">
      <ExportAction icon={<FileSpreadsheet />} title="CSV tables" detail={tables.length ? "Provenance and method tables" : "Run a method before CSV export"} disabledReason={tableExportDisabledReason} onClick={() => download("quickpls-result-tables.csv", tablesToCsv(tables), "text/csv", "CSV tables")} />
      <ExportAction icon={<FileText />} title="HTML report" detail={tables.length ? (includeInterpretationNotes ? "Tables plus interpretation notes" : "Watermarked table report") : "Run a method before HTML export"} disabledReason={tableExportDisabledReason} onClick={() => download("quickpls-result-report.html", reportHtml(tablesToHtml(tables), includeInterpretationNotes ? interpretation : null), "text/html", "HTML report")} />
      <ExportAction icon={<FileSpreadsheet />} title="XLSX workbook" detail={isNativeDesktop() ? "Desktop workbook export" : "Desktop runtime required"} disabledReason={xlsxDisabledReason} onClick={() => { void exportXlsx().catch((error) => window.alert(error)); }} />
      <ExportAction icon={<Printer />} title="Print / PDF" detail="Open browser print dialog for PDF output" disabledReason={pdfDisabledReason} onClick={printPdfReport} />
      <ExportAction icon={<Image />} title="Model diagram SVG" detail="WYSIWYG publication figure" disabledReason={svgDisabledReason} onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml", "SVG diagram")} />
    </section>
    <section className="report-comparison-link">
      <div>
        <strong>Run comparison</strong>
        <span>{comparisonRows.length ? `${comparisonRows.length} comparable metric rows are available in Results.` : "Comparison needs two compatible completed runs."}</span>
      </div>
      <button className="secondary-button" disabled={!comparisonRows.length} title={comparisonRows.length ? "Open Results comparison workspace" : "Create a second compatible run before comparing"} onClick={openResultsComparison}><ExternalLink size={15} /> Open Results Comparison</button>
    </section>
    {!tables.length ? <div className="method-note wide"><strong>Export gate</strong><p>Run an available method before exporting result tables. Stable publication exports remain gated until the relevant method family is validated.</p></div> : <div className="report-preview">
      {tables.map((table) => <article key={table.id}>
        <div><strong>{table.title}</strong><span className={`status-text ${table.status}`}>{table.status}</span></div>
        {table.warning && <p>{table.warning}</p>}
        <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label={`${table.title} table`}><table><caption>{table.title}</caption><thead><tr>{table.columns.map((column) => <th key={column}>{column}</th>)}</tr></thead><tbody>
          {table.rows.map((row, rowIndex) => <tr key={rowIndex}>{row.map((cell, columnIndex) => <td key={`${rowIndex}-${columnIndex}`}>{cell}</td>)}</tr>)}
        </tbody></table></div>
      </article>)}
    </div>}
  </section>;
}

function ExportAction({ icon, title, detail, disabledReason, onClick }: { icon: ReactNode; title: string; detail: string; disabledReason: string | null; onClick: () => void }) {
  return <button className="report-export-action" disabled={Boolean(disabledReason)} title={disabledReason ?? title} onClick={onClick}>
    {icon}
    <span><strong>{title}</strong><small>{detail}</small>{disabledReason ? <em>{disabledReason}</em> : null}</span>
    <Download size={14} className="export-action-cue" />
  </button>;
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
