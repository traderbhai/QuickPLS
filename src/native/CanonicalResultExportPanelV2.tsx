import { Download, FileImage, FileSpreadsheet, FileText, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  canonicalResultExportChartsV2,
  dispatchCanonicalResultExportV2,
  type CanonicalResultExportChartV2,
  type CanonicalResultExportFormatV2,
  type CanonicalResultExportWritersV2,
  type PreparedCanonicalResultBinaryExportV2,
  type PreparedCanonicalResultTextExportV2,
  type PreparedCanonicalResultWorkbookExportV2,
} from "../domain/canonicalResultCrossFormatExportV2";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import { publishNativeCanonicalResultExportV2 } from "../services/canonicalResultExportPublicationV2Service";
import { isNativeDesktop } from "../services/projectService";

export interface CanonicalResultExportPanelV2Props {
  /** Immutable scientific/export authority. */
  document: CanonicalResultDocumentV2;
  /** Optional label-only projection; never used as the export authority. */
  presentationDocument?: CanonicalResultDocumentV2;
  /** Optional writer overrides keep the shared dispatcher independently testable. */
  writers?: CanonicalResultExportWritersV2;
  nativeDesktop?: boolean;
  /** Hides stable implementation identities from the ordinary researcher view. */
  researcherFacing?: boolean;
}

interface ExportFeedbackV2 {
  tone: "neutral" | "success" | "error";
  message: string;
}

function downloadBrowserArtifact(name: string, data: BlobPart, mediaType: string): string {
  if (typeof document === "undefined" || typeof URL === "undefined") {
    throw new Error("This runtime cannot publish browser downloads.");
  }
  const url = URL.createObjectURL(new Blob([data], { type: mediaType }));
  try {
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = name;
    anchor.rel = "noopener";
    anchor.click();
  } finally {
    window.setTimeout(() => URL.revokeObjectURL(url), 0);
  }
  return name;
}

async function defaultTextWriter(artifact: PreparedCanonicalResultTextExportV2, signal?: AbortSignal): Promise<string | null> {
  if (isNativeDesktop()) {
    return (await publishNativeCanonicalResultExportV2(artifact, signal))?.path ?? null;
  }
  return downloadBrowserArtifact(artifact.defaultFileName, artifact.contents, artifact.mediaType);
}

async function defaultWorkbookWriter(artifact: PreparedCanonicalResultWorkbookExportV2, signal?: AbortSignal): Promise<string | null> {
  if (!isNativeDesktop()) throw new Error("XLSX export requires the QuickPLS desktop runtime.");
  return (await publishNativeCanonicalResultExportV2(artifact, signal))?.path ?? null;
}

async function defaultBinaryWriter(artifact: PreparedCanonicalResultBinaryExportV2, signal?: AbortSignal): Promise<string | null> {
  if (isNativeDesktop()) {
    return (await publishNativeCanonicalResultExportV2(artifact, signal))?.path ?? null;
  }
  return downloadBrowserArtifact(artifact.defaultFileName, artifact.bytes, artifact.mediaType);
}

function readableFormat(format: CanonicalResultExportFormatV2): string {
  return format.toUpperCase();
}

function iconFor(format: CanonicalResultExportFormatV2) {
  if (format === "csv" || format === "xlsx") return <FileSpreadsheet size={16} aria-hidden="true" />;
  if (format === "svg" || format === "png") return <FileImage size={16} aria-hidden="true" />;
  return <FileText size={16} aria-hidden="true" />;
}

function chartDomToken(value: string): string {
  return Array.from(new TextEncoder().encode(value), (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function displayChartValue(value: number | null | undefined): string {
  if (value == null) return "Not reported";
  if (Object.is(value, -0) || value === 0) return "0";
  const absolute = Math.abs(value);
  return absolute >= 1_000_000 || absolute < 0.0001
    ? value.toExponential(3)
    : value.toFixed(4).replace(/\.?0+$/u, "");
}

function CanonicalExportChartPreviewV2({
  entry,
  researcherFacing,
  sourceTableTitle,
}: {
  entry: CanonicalResultExportChartV2;
  researcherFacing: boolean;
  sourceTableTitle?: string;
}) {
  const { chart, origin } = entry;
  const domId = `nd-canonical-export-preview-${chartDomToken(chart.id)}`;
  const rows = chart.series.flatMap((series) => series.points.map((point, pointIndex) => ({
    seriesId: series.id,
    seriesLabel: series.label,
    pointIndex,
    point,
  })));
  const width = 640;
  const height = 260;
  const margin = { top: 16, right: 18, bottom: 42, left: 54 };
  const yValues = rows.flatMap(({ point }) => [point.y, point.lower, point.upper])
    .filter((value): value is number => typeof value === "number" && Number.isFinite(value));
  let yMinimum = Math.min(0, ...yValues);
  let yMaximum = Math.max(0, ...yValues);
  if (yMinimum === yMaximum) {
    const padding = Math.abs(yMinimum) > 0 ? Math.abs(yMinimum) * 0.1 : 1;
    yMinimum -= padding;
    yMaximum += padding;
  }
  const plotWidth = width - margin.left - margin.right;
  const plotHeight = height - margin.top - margin.bottom;
  const y = (value: number) => margin.top + ((yMaximum - value) / (yMaximum - yMinimum)) * plotHeight;
  const zeroY = y(0);
  const step = rows.length ? plotWidth / rows.length : plotWidth;
  const barWidth = Math.max(4, Math.min(36, step * 0.62));
  const summary = `${origin === "derived_from_canonical_table" ? "Export-derived" : researcherFacing ? "Saved" : "Persisted"} ${chart.kind} chart with ${chart.series.length} series and ${rows.length} exact ${rows.length === 1 ? "point" : "points"}.`;

  return <section aria-labelledby={`${domId}-region-title`}>
    <h4 id={`${domId}-region-title`} className="nd-sr-only">Selected canonical export chart preview</h4>
    <figure
      className="nd-canonical-chart"
      role="img"
      tabIndex={0}
      data-canonical-chart-id={chart.id}
      data-canonical-chart-origin={origin}
      aria-labelledby={`${domId}-title`}
      aria-describedby={`${domId}-description ${domId}-summary`}
    >
      <figcaption>
        <h4 id={`${domId}-title`}>{chart.title}</h4>
        <p id={`${domId}-description`}>{chart.description}</p>
      </figcaption>
      <p id={`${domId}-summary`} className="nd-canonical-chart__summary">{summary} Keyboard focus is available here; the exact values follow in a table.</p>
      <div className="nd-canonical-chart__plot-wrap">
        <svg className="nd-canonical-chart__plot" viewBox={`0 0 ${width} ${height}`} aria-hidden="true" focusable="false">
          <line className="nd-canonical-chart__axis" x1={margin.left} x2={margin.left} y1={margin.top} y2={height - margin.bottom} />
          <line className="nd-canonical-chart__axis" x1={margin.left} x2={width - margin.right} y1={zeroY} y2={zeroY} />
          {chart.kind === "bar" ? rows.map(({ point }, index) => {
            const x = margin.left + step * (index + 0.5);
            const pointY = y(point.y);
            const top = Math.min(pointY, zeroY);
            const rectHeight = Math.abs(zeroY - pointY);
            return <g className={`nd-canonical-chart__series--${index % 5}`} key={`${chart.id}:bar:${index}`}>
              <rect x={x - barWidth / 2} y={top} width={barWidth} height={Math.max(1, rectHeight)} fill="var(--nd-canonical-series-color)" />
            </g>;
          }) : chart.series.map((series, seriesIndex) => {
            const projected = series.points.map((point, pointIndex) => {
              const globalIndex = rows.findIndex((row) => row.seriesId === series.id && row.pointIndex === pointIndex);
              return { point, x: margin.left + step * (globalIndex + 0.5), y: y(point.y) };
            });
            const path = projected.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" ");
            return <g className={`nd-canonical-chart__series--${seriesIndex % 5}`} key={`${chart.id}:series:${series.id}`}>
              {projected.length > 1 ? <path className="nd-canonical-chart__line" d={path} /> : null}
              {projected.map((point, index) => <circle className="nd-canonical-chart__point" cx={point.x} cy={point.y} r={4} key={`${series.id}:${index}`} />)}
            </g>;
          })}
          {rows.map((_row, index) => {
            const x = margin.left + step * (index + 0.5);
            return <text className="nd-canonical-chart__tick" x={x} y={height - margin.bottom + 17} textAnchor="middle" key={`${chart.id}:tick:${index}`}>{index + 1}</text>;
          })}
          <text className="nd-canonical-chart__axis-label" x={(margin.left + width - margin.right) / 2} y={height - 7} textAnchor="middle">{chart.display.x_axis_label ?? "Point index"}</text>
          <text className="nd-canonical-chart__axis-label" x={12} y={height / 2} textAnchor="middle" transform={`rotate(-90 12 ${height / 2})`}>{chart.display.y_axis_label ?? "Estimate"}</text>
        </svg>
      </div>
      {origin === "derived_from_canonical_table" ? <p className="nd-canonical-chart__source">{researcherFacing
        ? "Derived visual only; the saved scientific result is unchanged."
        : "Derived visual only; the resident canonical result and its scientific identities are unchanged."}</p> : null}
    </figure>
    <div className="nd-cbsem-v4-table-wrap" data-canonical-chart-table-fallback={chart.id}>
      <table>
        <caption><strong>{chart.title} accessible data</strong><span>Exact table fallback in stable chart order.</span></caption>
        <thead><tr><th scope="col">Index</th><th scope="col">Series</th><th scope="col">Effect or point identity</th><th scope="col">X</th><th scope="col">Estimate</th><th scope="col">Lower</th><th scope="col">Upper</th><th scope="col">Source table</th></tr></thead>
        <tbody>{rows.map(({ seriesId, seriesLabel, pointIndex, point }, index) => <tr key={`${seriesId}:${pointIndex}`}>
          <th scope="row">{index + 1}</th>
          <td>{seriesLabel}</td>
          <td>{researcherFacing
            ? point.label ?? `Point ${index + 1}`
            : <code>{point.label ?? `${seriesId}:${pointIndex}`}</code>}</td>
          <td>{String(point.x)}</td>
          <td>{displayChartValue(point.y)}</td>
          <td>{displayChartValue(point.lower)}</td>
          <td>{displayChartValue(point.upper)}</td>
          <td>{researcherFacing
            ? sourceTableTitle ?? "No source table"
            : <code>{chart.source_table_id ?? "No source table"}</code>}</td>
        </tr>)}</tbody>
      </table>
    </div>
  </section>;
}

export function CanonicalResultExportPanelV2({
  document,
  presentationDocument,
  writers,
  nativeDesktop = isNativeDesktop(),
  researcherFacing = false,
}: CanonicalResultExportPanelV2Props) {
  const displayedDocument = researcherFacing
    && presentationDocument?.document_id === document.document_id
    ? presentationDocument
    : document;
  const tableIds = useMemo(() => displayedDocument.tables.map((table) => table.id), [displayedDocument]);
  const exportCharts = useMemo(() => canonicalResultExportChartsV2(displayedDocument), [displayedDocument]);
  const chartIds = useMemo(() => exportCharts.map((entry) => entry.chart.id), [exportCharts]);
  const [selectedTableIds, setSelectedTableIds] = useState<string[]>(tableIds);
  const [selectedChartId, setSelectedChartId] = useState<string>(chartIds[0] ?? "");
  const [busy, setBusy] = useState<CanonicalResultExportFormatV2 | null>(null);
  const [feedback, setFeedback] = useState<ExportFeedbackV2 | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const selectedChart = exportCharts.find((entry) => entry.chart.id === selectedChartId) ?? null;
  const selectedChartSourceTitle = selectedChart?.chart.source_table_id
    ? displayedDocument.tables.find((table) => table.id === selectedChart.chart.source_table_id)?.title
    : undefined;

  useEffect(() => {
    setSelectedTableIds(tableIds);
    setSelectedChartId(chartIds[0] ?? "");
    setFeedback(null);
    abortRef.current?.abort();
    abortRef.current = null;
    setBusy(null);
  }, [chartIds, displayedDocument.document_id, tableIds]);

  useEffect(() => () => abortRef.current?.abort(), []);

  const effectiveWriters = useMemo<CanonicalResultExportWritersV2>(() => ({
    text: writers?.text ?? defaultTextWriter,
    workbook: writers?.workbook ?? defaultWorkbookWriter,
    binary: writers?.binary ?? defaultBinaryWriter,
  }), [writers]);

  const runExport = async (format: CanonicalResultExportFormatV2) => {
    if (busy) return;
    const controller = new AbortController();
    abortRef.current = controller;
    setBusy(format);
    setFeedback({ tone: "neutral", message: `Preparing the verified ${readableFormat(format)} export.` });
    const reportCharts = format === "html" || format === "pdf" ? chartIds : undefined;
    const chartSelection = format === "svg" || format === "png"
      ? selectedChartId ? [selectedChartId] : []
      : reportCharts;
    try {
      const outcome = await dispatchCanonicalResultExportV2(document, {
        format,
        tableIds: format === "svg" || format === "png" ? [] : selectedTableIds,
        ...(chartSelection !== undefined ? { chartIds: chartSelection } : {}),
      }, effectiveWriters, controller.signal);
      if (outcome.status === "saved") {
        setFeedback({ tone: "success", message: `Saved ${outcome.path}. Semantic readback passed before publication.` });
      } else if (outcome.status === "cancelled") {
        setFeedback({ tone: "neutral", message: "Export cancelled. No native file was published; semantic readback completed before the publication boundary." });
      } else if (outcome.status === "unavailable") {
        setFeedback({ tone: "error", message: outcome.message });
      } else {
        setFeedback({ tone: "error", message: `Export failed: ${outcome.message}` });
      }
    } finally {
      if (abortRef.current === controller) abortRef.current = null;
      setBusy(null);
    }
  };

  const cancelExport = () => {
    abortRef.current?.abort();
    setFeedback({ tone: "neutral", message: "Cancelling export before publication." });
  };

  const toggleTable = (tableId: string, checked: boolean) => {
    setSelectedTableIds((current) => checked
      ? tableIds.filter((id) => id === tableId || current.includes(id))
      : current.filter((id) => id !== tableId));
  };

  const tableFormatsDisabled = busy !== null || selectedTableIds.length === 0;
  const chartFormatsDisabled = busy !== null || !selectedChartId;
  const xlsxUnavailable = !nativeDesktop && !writers?.workbook;
  const feedbackRole = feedback?.tone === "error" ? "alert" : "status";

  return <section className="nd-cbsem-v4-card nd-canonical-export-v2" aria-labelledby="nd-canonical-export-v2-heading" aria-busy={busy !== null}>
    <header><div><h3 id="nd-canonical-export-v2-heading"><Download size={17} aria-hidden="true" />{researcherFacing ? "Export verified result" : "Export verified canonical result"}</h3><p>{researcherFacing
      ? "Choose result tables once, then export each format from the same saved scientific result."
      : "Choose exact canonical tables once, then export every format through the same provenance-bound semantic dispatcher."}</p></div></header>
    <fieldset className="nd-canonical-export-v2__tables" disabled={busy !== null} aria-describedby="nd-canonical-export-v2-selection-summary">
      <legend>Tables to include</legend>
      <div className="nd-cbsem-v4-actions">
        <button type="button" disabled={busy !== null || selectedTableIds.length === tableIds.length} onClick={() => setSelectedTableIds(tableIds)}>Select all tables</button>
        <button type="button" disabled={busy !== null || selectedTableIds.length === 0} onClick={() => setSelectedTableIds([])}>Clear table selection</button>
      </div>
      {displayedDocument.tables.map((table) => {
        const inputId = `nd-canonical-export-table-${Array.from(new TextEncoder().encode(table.id), (byte) => byte.toString(16).padStart(2, "0")).join("")}`;
        return <label className="nd-checkbox-row" htmlFor={inputId} key={table.id}>
          <input id={inputId} type="checkbox" checked={selectedTableIds.includes(table.id)} onChange={(event) => toggleTable(table.id, event.target.checked)} />
          <span><strong>{table.title}</strong>{researcherFacing ? null : <small>Stable table ID: <code>{table.id}</code></small>}</span>
        </label>;
      })}
    </fieldset>
    {exportCharts.length ? <label htmlFor="nd-canonical-export-v2-chart">Chart for SVG or PNG
      <select id="nd-canonical-export-v2-chart" value={selectedChartId} disabled={busy !== null} onChange={(event) => setSelectedChartId(event.target.value)}>
        {exportCharts.map(({ chart, origin }) => <option value={chart.id} key={chart.id}>{chart.title} · {origin === "persisted"
          ? researcherFacing ? "saved" : "persisted"
          : researcherFacing ? "derived from result table" : "derived from canonical table"}{researcherFacing ? "" : ` · ${chart.id}`}</option>)}
      </select>
    </label> : <p role="note">This canonical result contains no persisted chart and no exact effect table from which QuickPLS can derive a truthful visual, so SVG and PNG chart export are unavailable.</p>}
    {selectedChart ? <CanonicalExportChartPreviewV2
      entry={selectedChart}
      researcherFacing={researcherFacing}
      sourceTableTitle={selectedChartSourceTitle}
    /> : null}
    <p id="nd-canonical-export-v2-selection-summary" role="status" aria-live="polite">{selectedTableIds.length} of {tableIds.length} {researcherFacing ? "result" : "canonical"} tables selected. {chartIds.length ? `${selectedChart?.origin === "derived_from_canonical_table" ? "Derived" : researcherFacing ? "Saved" : "Persisted"} chart${researcherFacing ? " selected" : ` ${selectedChartId || "not selected"}`}.` : "No exportable chart."}</p>
    <div className="nd-cbsem-v4-actions" aria-label="Canonical result export formats">
      {(["csv", "xlsx", "html", "pdf", "svg", "png"] as const).map((format) => {
        const chartFormat = format === "svg" || format === "png";
        const disabled = chartFormat ? chartFormatsDisabled : tableFormatsDisabled || (format === "xlsx" && xlsxUnavailable);
        const reason = format === "xlsx" && xlsxUnavailable
          ? "XLSX workbook publication requires the QuickPLS desktop runtime."
          : chartFormat && !selectedChartId
            ? "Select a canonical export chart first."
            : !chartFormat && selectedTableIds.length === 0
              ? "Select at least one canonical table first."
              : undefined;
        return <button type="button" key={format} disabled={disabled} title={reason} aria-describedby="nd-canonical-export-v2-selection-summary" onClick={() => void runExport(format)}>
          {iconFor(format)}{busy === format ? `Preparing ${readableFormat(format)}…` : `Export ${readableFormat(format)}`}
        </button>;
      })}
      {busy ? <button type="button" className="danger" onClick={cancelExport}><X size={15} aria-hidden="true" />Cancel export</button> : null}
    </div>
    {feedback ? <p className={`nd-export-feedback ${feedback.tone}`} role={feedbackRole} aria-live="polite" aria-atomic="true">{feedback.message}</p> : null}
    <details><summary>Export identity</summary><dl><div><dt>Document</dt><dd>{document.document_id}</dd></div><div><dt>Run</dt><dd>{document.provenance.run_id}</dd></div><div><dt>Method</dt><dd>{document.provenance.method_version}</dd></div><div><dt>Dataset fingerprint</dt><dd>{document.provenance.dataset_fingerprint}</dd></div>{researcherFacing ? <><div><dt>Selected table IDs</dt><dd>{selectedTableIds.join(", ") || "None"}</dd></div><div><dt>Selected chart ID</dt><dd>{selectedChartId || "None"}</dd></div></> : null}</dl></details>
  </section>;
}

export default CanonicalResultExportPanelV2;
