import { FileSpreadsheet, FileText, Image } from "lucide-react";
import { useMemo, useState } from "react";
import { publicationDiagramSvg } from "../domain/publicationDiagram";
import { runExportTables, tablesToCsv, tablesToHtml } from "../domain/resultTables";
import { compareRuns } from "../domain/runComparison";
import { exportNativeXlsxTables, isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";

export function ReportsWorkspace() {
  const runs = useWorkspace((state) => state.runs);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const publicationDiagramSettings = useWorkspace((state) => state.publicationDiagramSettings);
  const setPublicationDiagramSettings = useWorkspace((state) => state.setPublicationDiagramSettings);
  const [selectedRunId, setSelectedRunId] = useState(runs.at(0)?.id ?? "");
  const [comparisonRunId, setComparisonRunId] = useState(runs.at(1)?.id ?? runs.at(0)?.id ?? "");
  const selectedRun = useMemo(() => runs.find((run) => run.id === selectedRunId) ?? runs.at(0), [runs, selectedRunId]);
  const comparisonRun = useMemo(() => runs.find((run) => run.id === comparisonRunId) ?? runs.find((run) => run.id !== selectedRun?.id), [runs, comparisonRunId, selectedRun?.id]);
  const tables = useMemo(() => selectedRun ? runExportTables(selectedRun) : [], [selectedRun]);
  const comparisonRows = useMemo(() => compareRuns(selectedRun, comparisonRun), [selectedRun, comparisonRun]);
  const diagramSvg = useMemo(() => publicationDiagramSvg(nodes, edges, selectedRun, publicationDiagramSettings, diagramLayout), [diagramLayout, edges, nodes, publicationDiagramSettings, selectedRun]);
  const exportDisabledReason = !tables.length
    ? "Run an available method before exporting result tables."
    : !isNativeDesktop()
      ? "CSV, HTML, SVG, and print/PDF are available here. XLSX export requires the desktop runtime."
      : null;

  const download = (name: string, contents: string, type: string) => {
    const url = URL.createObjectURL(new Blob([contents], { type }));
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = name;
    anchor.click();
    setTimeout(() => URL.revokeObjectURL(url), 0);
  };
  const printPdfReport = () => {
    const printable = window.open("", "_blank", "noopener,noreferrer");
    if (!printable) return;
    printable.document.write(tablesToHtml(tables));
    printable.document.close();
    printable.focus();
    printable.print();
  };
  const exportXlsx = async () => {
    if (!isNativeDesktop()) return;
    await exportNativeXlsxTables(tables);
  };

  return <section className="workspace-page publication-workspace">
    <div className="page-heading"><div><h1>Publication report</h1><p>Preview the exact diagram style, export tables, and preserve validation status, warnings, and run provenance.</p></div></div>
    <div className="analysis-settings">
      <div><strong>Publication setup</strong><span className={selectedRun ? "status-text validated" : "status-text experimental"}>{selectedRun ? "run selected" : "model-only preview"}</span></div>
      <label>Saved run<select value={selectedRun?.id ?? ""} onChange={(event) => setSelectedRunId(event.target.value)} disabled={!runs.length}>
        {runs.length ? runs.map((run) => <option key={run.id} value={run.id}>{run.name}</option>) : <option>No saved runs</option>}
      </select></label>
      <label>Compare with<select value={comparisonRun?.id ?? ""} onChange={(event) => setComparisonRunId(event.target.value)} disabled={runs.length < 2}>
        {runs.length > 1 ? runs.filter((run) => run.id !== selectedRun?.id).map((run) => <option key={run.id} value={run.id}>{run.name}</option>) : <option>Need two runs</option>}
      </select></label>
      <label>Diagram mode<select value={publicationDiagramSettings.mode} onChange={(event) => setPublicationDiagramSettings({ mode: event.target.value as typeof publicationDiagramSettings.mode })}>
        <option value="smartpls_result">SmartPLS-like</option>
        <option value="publication">QuickPLS publication</option>
        <option value="sem">SEM diagram</option>
        <option value="compact">Compact</option>
      </select></label>
      <label>Diagram precision<select value={publicationDiagramSettings.precision} onChange={(event) => setPublicationDiagramSettings({ precision: Number(event.target.value) })}>
        {[2, 3, 4, 5, 6].map((value) => <option key={value} value={value}>{value} decimals</option>)}
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
      <label>Loadings<input type="checkbox" checked={publicationDiagramSettings.showLoadings} onChange={(event) => setPublicationDiagramSettings({ showLoadings: event.target.checked })} /></label>
      <label>Path coefficients<input type="checkbox" checked={publicationDiagramSettings.showPathCoefficients} onChange={(event) => setPublicationDiagramSettings({ showPathCoefficients: event.target.checked })} /></label>
      <label>R<sup>2</sup><input type="checkbox" checked={publicationDiagramSettings.showRSquared} onChange={(event) => setPublicationDiagramSettings({ showRSquared: event.target.checked })} /></label>
    </div>
    {exportDisabledReason ? <p className="disabled-reason export-disabled-reason top-export-reason">{exportDisabledReason}</p> : null}
    <div className="publication-preview-shell">
      <div className="publication-preview-heading"><div><strong>Publication diagram preview</strong><span>{selectedRun ? "WYSIWYG SVG export with selected run overlays" : "Model-only SVG preview until a result is selected"}</span></div><button className="secondary-button" onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml")}><Image size={16} /> Export SVG</button></div>
      <div className="diagram-preview" aria-label="Publication diagram preview" dangerouslySetInnerHTML={{ __html: diagramSvg }} />
    </div>
    <div className="export-list">
      <button disabled={!tables.length} onClick={() => download("quickpls-result-tables.csv", tablesToCsv(tables), "text/csv")}><FileSpreadsheet /><span><strong>CSV tables</strong><small>Provenance and method tables</small></span></button>
      <button disabled={!tables.length} onClick={() => download("quickpls-result-report.html", tablesToHtml(tables), "text/html")}><FileText /><span><strong>HTML report</strong><small>Watermarked table report</small></span></button>
      <button disabled={!tables.length || !isNativeDesktop()} title={isNativeDesktop() ? "Export XLSX workbook" : "XLSX export is available in the desktop app and CLI"} onClick={() => { void exportXlsx().catch((error) => window.alert(error)); }}><FileSpreadsheet /><span><strong>XLSX workbook</strong><small>Desktop and CLI</small></span></button>
      <button disabled={!tables.length} onClick={printPdfReport}><FileText /><span><strong>Print / PDF</strong><small>Browser PDF path</small></span></button>
      <button onClick={() => download("quickpls-publication-diagram.svg", diagramSvg, "image/svg+xml")}><Image /><span><strong>Model diagram</strong><small>WYSIWYG SVG</small></span></button>
    </div>
    {exportDisabledReason ? <p className="disabled-reason export-disabled-reason">{exportDisabledReason}</p> : null}
    {comparisonRows.length > 0 && <div className="report-preview">
      <article>
        <div><strong>Run comparison</strong><span className="status-text experimental">review</span></div>
        <p>Baseline: {selectedRun?.name}. Comparison: {comparisonRun?.name}. Positive delta means the comparison run is larger.</p>
        <div className="bootstrap-table-scroll" tabIndex={0} role="region" aria-label="Run comparison table"><table><thead><tr><th>Metric</th><th>Item</th><th>Baseline</th><th>Comparison</th><th>Delta</th></tr></thead><tbody>
          {comparisonRows.map((row) => <tr key={`${row.metric}-${row.item}`}><td>{row.metric}</td><td>{row.item}</td><td>{row.baseline}</td><td>{row.comparison}</td><td>{row.delta}</td></tr>)}
        </tbody></table></div>
      </article>
    </div>}
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
