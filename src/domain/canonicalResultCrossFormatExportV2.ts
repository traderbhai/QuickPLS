import {
  buildCanonicalResultSemanticExportV2,
  canonicalResultSemanticExportJsonV2,
  type CanonicalResultSemanticExportV2,
} from "./canonicalResultSemanticExportV2";
import type {
  CanonicalChartPoint,
  CanonicalResultCell,
  CanonicalResultChart,
  CanonicalResultDocumentV2,
  CanonicalResultTable,
} from "./canonicalResultDocumentV2";
import type { ResultTable } from "./resultTables";
import { spreadsheetSafeCsvCell } from "./spreadsheetSafety";
import { sha256HexUtf8V1 } from "./sha256V1";

export const CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION = 2 as const;
export const CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT = "quickpls.canonical-result-cross-format-export" as const;
export const CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID = "quickpls-canonical-semantic-export-v2" as const;
export const CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2 = "quickpls_export_specific_indirect_effect_estimates_v2" as const;
export const CANONICAL_RESULT_DERIVED_AGGREGATE_EFFECT_CHART_ID_V2 = "quickpls_export_aggregate_effect_estimates_v2" as const;

export type CanonicalResultExportFormatV2 = "csv" | "xlsx" | "html" | "pdf" | "svg" | "png";

export interface CanonicalResultExportSelectionV2 {
  /** Exact canonical table IDs. Omit to select every table in document order. */
  tableIds?: readonly string[];
  /** Exact canonical chart IDs. Omit to select every chart for HTML/PDF and none for table-only formats. */
  chartIds?: readonly string[];
}

export interface CanonicalResultExportRequestV2 extends CanonicalResultExportSelectionV2 {
  format: CanonicalResultExportFormatV2;
}

interface CanonicalResultExportSemanticPayloadV2 {
  schema_version: typeof CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION;
  format: typeof CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT;
  source: CanonicalResultSemanticExportV2["source"] & { semantic_projection_sha256: string };
  title: string;
  provenance: CanonicalResultSemanticExportV2["provenance"];
  capability_cells?: CanonicalResultSemanticExportV2["capability_cells"];
  selection: {
    table_ids: string[];
    chart_ids: string[];
  };
  sections: CanonicalResultSemanticExportV2["sections"];
  tables: CanonicalResultSemanticExportV2["tables"];
  charts: CanonicalResultSemanticExportV2["charts"];
  notices: CanonicalResultSemanticExportV2["notices"];
  exclusions: CanonicalResultSemanticExportV2["exclusions"];
  footnotes: CanonicalResultSemanticExportV2["footnotes"];
  presentation: Pick<CanonicalResultSemanticExportV2["presentation"], "precision" | "missing_value_label">;
}

export interface CanonicalResultExportSemanticEnvelopeV2 extends CanonicalResultExportSemanticPayloadV2 {
  semantic_sha256: string;
}

interface PreparedCanonicalResultExportBaseV2 {
  format: CanonicalResultExportFormatV2;
  extension: CanonicalResultExportFormatV2;
  mediaType: string;
  defaultFileName: string;
  semantic: CanonicalResultExportSemanticEnvelopeV2;
}

export interface PreparedCanonicalResultTextExportV2 extends PreparedCanonicalResultExportBaseV2 {
  format: "csv" | "html" | "svg";
  contents: string;
}

export interface PreparedCanonicalResultWorkbookExportV2 extends PreparedCanonicalResultExportBaseV2 {
  format: "xlsx";
  workbookTables: ResultTable[];
}

export interface PreparedCanonicalResultBinaryExportV2 extends PreparedCanonicalResultExportBaseV2 {
  format: "pdf" | "png";
  bytes: Uint8Array;
}

export type PreparedCanonicalResultExportV2 =
  | PreparedCanonicalResultTextExportV2
  | PreparedCanonicalResultWorkbookExportV2
  | PreparedCanonicalResultBinaryExportV2;

export type CanonicalResultExportPreparationV2 =
  | { ok: true; artifact: PreparedCanonicalResultExportV2 }
  | {
      ok: false;
      code: "invalid_source_document" | "invalid_selection" | "unsupported_chart" | "unsupported_visible_text";
      errors: string[];
    };

export interface CanonicalResultExportWritersV2 {
  text?: (artifact: PreparedCanonicalResultTextExportV2, signal?: AbortSignal) => Promise<string | null>;
  workbook?: (artifact: PreparedCanonicalResultWorkbookExportV2, signal?: AbortSignal) => Promise<string | null>;
  binary?: (artifact: PreparedCanonicalResultBinaryExportV2, signal?: AbortSignal) => Promise<string | null>;
}

export type CanonicalResultExportDispatchV2 =
  | { status: "saved"; path: string; artifact: PreparedCanonicalResultExportV2 }
  | { status: "cancelled"; artifact?: PreparedCanonicalResultExportV2 }
  | { status: "unavailable"; format: CanonicalResultExportFormatV2; message: string }
  | { status: "failed"; format: CanonicalResultExportFormatV2; message: string; errors: string[] };

export interface CanonicalResultExportReadbackV2 {
  passed: boolean;
  exact_semantic_match: boolean;
  digest_match: boolean;
  rendered_surface_match: boolean;
  errors: string[];
  envelope?: CanonicalResultExportSemanticEnvelopeV2;
}

export interface CanonicalResultExportChartV2 {
  chart: CanonicalResultChart;
  origin: "persisted" | "derived_from_canonical_table";
}

const XLSX_MANIFEST_TABLE_ID = "quickpls_export_manifest_v2";
const XLSX_PROVENANCE_TABLE_ID = "quickpls_export_provenance_v2";
const PDF_METADATA_BEGIN = "%QPLS-CANONICAL-SEMANTIC-V2-BEGIN";
const PDF_METADATA_LINE = "%QPLS-V2:";
const PDF_METADATA_END = "%QPLS-CANONICAL-SEMANTIC-V2-END";
const PNG_SEMANTIC_KEYWORD = "quickpls.semantic.v2";
const PNG_WIDTH = 960;
const PNG_HEIGHT = 540;
const CHART_COLORS = [
  [25, 90, 132, 255],
  [190, 72, 42, 255],
  [45, 130, 91, 255],
  [120, 75, 150, 255],
  [171, 120, 30, 255],
] as const;

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return Object.fromEntries(Object.keys(record).sort().map((key) => [key, stableValue(record[key])]));
  }
  return value;
}

function stableJson(value: unknown): string {
  return JSON.stringify(stableValue(value));
}

const DERIVED_EFFECT_CHART_SOURCES_V2 = [
  {
    tableId: "general_sem_specific_indirect_effects",
    chartId: CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2,
    title: "Specific indirect effect estimates",
  },
  {
    tableId: "general_sem_aggregate_effects",
    chartId: CANONICAL_RESULT_DERIVED_AGGREGATE_EFFECT_CHART_ID_V2,
    title: "Aggregate effect estimates",
  },
] as const;

function derivedEffectEstimateChartV2(
  table: CanonicalResultTable,
  chartId: string,
  title: string,
): CanonicalResultChart | null {
  const effectIdIndex = table.columns.findIndex((column) => column.id === "effect_id" && column.data_type === "text");
  const estimateIndex = table.columns.findIndex((column) => column.id === "estimate" && column.data_type === "number");
  if (effectIdIndex < 0 || estimateIndex < 0 || table.rows.length === 0) return null;
  const points: CanonicalChartPoint[] = [];
  for (const [index, row] of table.rows.entries()) {
    const effectId = row.cells[effectIdIndex];
    const estimate = row.cells[estimateIndex];
    if (effectId?.kind !== "text" || estimate?.kind !== "number" || !Number.isFinite(estimate.value)) return null;
    points.push({ x: index + 1, y: estimate.value, label: effectId.value });
  }
  return {
    id: chartId,
    title,
    description: `Export-only visual derived exactly from canonical table ${table.id}. Effect index follows canonical row order; every point retains its effect_id and estimate without creating a new estimand.`,
    kind: "bar",
    series: [{ id: "estimate", label: "Estimate", points }],
    source_table_id: table.id,
    display: {
      show_legend: true,
      show_values: true,
      x_axis_label: "Effect index",
      y_axis_label: "Estimate",
    },
  };
}

function exportChartsForProjectionV2(
  projection: CanonicalResultSemanticExportV2,
): CanonicalResultExportChartV2[] {
  if (projection.charts.length > 0) {
    return projection.charts.map((chart) => ({ chart: structuredClone(chart), origin: "persisted" }));
  }
  if (projection.provenance.capability_cell.cell_id !== "qpls3.pls.mediation"
    || projection.provenance.capability_cell.capability_version !== "pls_mediation_v1") {
    return [];
  }
  return DERIVED_EFFECT_CHART_SOURCES_V2.flatMap((source) => {
    const table = projection.tables.find((candidate) => candidate.id === source.tableId);
    if (!table) return [];
    const chart = derivedEffectEstimateChartV2(table, source.chartId, source.title);
    return chart ? [{ chart, origin: "derived_from_canonical_table" as const }] : [];
  });
}

/**
 * Lists persisted charts or, only when none exist, deterministic export-only
 * visuals projected from exact General SEM effect table cells. The resident
 * CanonicalResultDocumentV2 is never mutated and no analytical value is added.
 */
export function canonicalResultExportChartsV2(
  document: CanonicalResultDocumentV2,
): CanonicalResultExportChartV2[] {
  const built = buildCanonicalResultSemanticExportV2(document);
  return built.ok ? exportChartsForProjectionV2(built.projection) : [];
}

function duplicateValues(values: readonly string[]): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) duplicates.add(value);
    seen.add(value);
  }
  return [...duplicates].sort();
}

function orderedSelection(
  requested: readonly string[] | undefined,
  available: readonly string[],
  defaultAll: boolean,
  label: string,
): { values: string[]; errors: string[] } {
  const raw = requested === undefined ? (defaultAll ? [...available] : []) : [...requested];
  const errors: string[] = [];
  const duplicates = duplicateValues(raw);
  if (duplicates.length) errors.push(`${label} contains duplicate IDs: ${duplicates.join(", ")}.`);
  const availableSet = new Set(available);
  const unknown = raw.filter((id) => !availableSet.has(id));
  if (unknown.length) errors.push(`${label} contains unknown IDs: ${[...new Set(unknown)].join(", ")}.`);
  const selected = new Set(raw);
  return { values: available.filter((id) => selected.has(id)), errors };
}

function relevantNotices(
  projection: CanonicalResultSemanticExportV2,
  tableIds: ReadonlySet<string>,
  sectionIds: ReadonlySet<string>,
) {
  return projection.notices.filter((notice) => (
    (notice.table_ids.length === 0 && notice.section_ids.length === 0)
    || notice.table_ids.some((id) => tableIds.has(id))
    || notice.section_ids.some((id) => sectionIds.has(id))
  ));
}

function buildSemanticEnvelope(
  projection: CanonicalResultSemanticExportV2,
  tableIds: string[],
  chartIds: string[],
  exportCharts: readonly CanonicalResultChart[],
): CanonicalResultExportSemanticEnvelopeV2 {
  const tableSet = new Set(tableIds);
  const chartSet = new Set(chartIds);
  const sections = projection.sections.filter((section) => (
    section.table_ids.some((id) => tableSet.has(id)) || section.chart_ids.some((id) => chartSet.has(id))
  )).map((section) => ({
    ...section,
    table_ids: section.table_ids.filter((id) => tableSet.has(id)),
    chart_ids: section.chart_ids.filter((id) => chartSet.has(id)),
  }));
  const sectionSet = new Set(sections.map((section) => section.id));
  const tables = projection.tables.filter((table) => tableSet.has(table.id));
  const footnoteSet = new Set(tables.flatMap((table) => table.footnote_ids));
  const semanticProjectionJson = canonicalResultSemanticExportJsonV2(projection);
  const payload: CanonicalResultExportSemanticPayloadV2 = {
    schema_version: CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION,
    format: CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT,
    source: {
      ...projection.source,
      semantic_projection_sha256: sha256HexUtf8V1(semanticProjectionJson),
    },
    title: projection.title,
    provenance: structuredClone(projection.provenance),
    ...(projection.capability_cells !== undefined
      ? { capability_cells: structuredClone(projection.capability_cells) }
      : {}),
    selection: { table_ids: tableIds, chart_ids: chartIds },
    sections: structuredClone(sections),
    tables: structuredClone(tables),
    charts: structuredClone(exportCharts.filter((chart) => chartSet.has(chart.id))),
    notices: structuredClone(relevantNotices(projection, tableSet, sectionSet)),
    exclusions: structuredClone(projection.exclusions),
    footnotes: structuredClone(projection.footnotes.filter((footnote) => footnoteSet.has(footnote.id))),
    presentation: {
      precision: projection.presentation.precision,
      missing_value_label: projection.presentation.missing_value_label,
    },
  };
  return { ...payload, semantic_sha256: sha256HexUtf8V1(stableJson(payload)) };
}

function base64EncodeBytes(bytes: Uint8Array): string {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let output = "";
  for (let index = 0; index < bytes.length; index += 3) {
    const first = bytes[index] ?? 0;
    const second = bytes[index + 1] ?? 0;
    const third = bytes[index + 2] ?? 0;
    const combined = (first << 16) | (second << 8) | third;
    output += alphabet[(combined >>> 18) & 63];
    output += alphabet[(combined >>> 12) & 63];
    output += index + 1 < bytes.length ? alphabet[(combined >>> 6) & 63] : "=";
    output += index + 2 < bytes.length ? alphabet[combined & 63] : "=";
  }
  return output;
}

function base64DecodeBytes(value: string): Uint8Array | null {
  const compact = value.replace(/\s+/gu, "");
  if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(compact)) return null;
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const bytes: number[] = [];
  for (let index = 0; index < compact.length; index += 4) {
    const chars = compact.slice(index, index + 4);
    const a = alphabet.indexOf(chars[0] ?? "");
    const b = alphabet.indexOf(chars[1] ?? "");
    const c = chars[2] === "=" ? 0 : alphabet.indexOf(chars[2] ?? "");
    const d = chars[3] === "=" ? 0 : alphabet.indexOf(chars[3] ?? "");
    if (a < 0 || b < 0 || c < 0 || d < 0) return null;
    const combined = (a << 18) | (b << 12) | (c << 6) | d;
    bytes.push((combined >>> 16) & 0xff);
    if (chars[2] !== "=") bytes.push((combined >>> 8) & 0xff);
    if (chars[3] !== "=") bytes.push(combined & 0xff);
  }
  return Uint8Array.from(bytes);
}

function encodeEnvelope(envelope: CanonicalResultExportSemanticEnvelopeV2): string {
  return base64EncodeBytes(new TextEncoder().encode(stableJson(envelope)));
}

function decodeEnvelope(value: string): CanonicalResultExportSemanticEnvelopeV2 | null {
  const bytes = base64DecodeBytes(value);
  if (!bytes) return null;
  try {
    const parsed = JSON.parse(new TextDecoder().decode(bytes)) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed)
      ? parsed as CanonicalResultExportSemanticEnvelopeV2
      : null;
  } catch {
    return null;
  }
}

function semanticCellText(cell: CanonicalResultCell): string {
  if (cell.kind === "number") return Object.is(cell.value, -0) ? "0" : String(cell.value);
  if (cell.kind === "boolean") return cell.value ? "true" : "false";
  if (cell.kind === "text") return cell.value;
  return `missing:${cell.reason}`;
}

function displayCellText(cell: CanonicalResultCell, precision: number, missingLabel: string): string {
  if (cell.kind === "number") return cell.display ?? cell.value.toFixed(precision);
  if (cell.kind === "boolean") return cell.value ? "Yes" : "No";
  if (cell.kind === "text") return cell.value;
  return cell.display ?? missingLabel;
}

function csvRow(values: readonly string[]): string {
  return values.map(spreadsheetSafeCsvCell).join(",");
}

function canonicalCsv(envelope: CanonicalResultExportSemanticEnvelopeV2): string {
  const rows: string[][] = [
    [CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT, String(CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION)],
    [CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID, encodeEnvelope(envelope)],
    ["document_id", envelope.source.document_id],
    ["semantic_sha256", envelope.semantic_sha256],
    ["run_id", envelope.provenance.run_id],
    ["method_version", envelope.provenance.method_version],
    ["dataset_fingerprint", envelope.provenance.dataset_fingerprint],
    [],
  ];
  for (const table of envelope.tables) {
    rows.push(
      ["table_id", table.id],
      ["table_title", table.title],
      ["column_ids", "row_id", ...table.columns.map((column) => column.id)],
      ["column_labels", "Row ID", ...table.columns.map((column) => column.label)],
      ["column_types", "text", ...table.columns.map((column) => column.data_type)],
    );
    for (const row of table.rows) rows.push(["row", row.id, ...row.cells.map(semanticCellText)]);
    rows.push([]);
  }
  return rows.map(csvRow).join("\r\n");
}

function workbookTables(envelope: CanonicalResultExportSemanticEnvelopeV2): ResultTable[] {
  const payload = encodeEnvelope(envelope);
  const payloadChunks = payload.match(/.{1,30000}/gu) ?? [""];
  const manifest: ResultTable = {
    id: XLSX_MANIFEST_TABLE_ID,
    title: XLSX_MANIFEST_TABLE_ID,
    status: "validated",
    warning: "Machine-readable cross-format manifest. Keep this worksheet with the selected canonical result worksheets.",
    columns: ["Field ID", "Value"],
    rows: [
      ["export_schema_version", String(envelope.schema_version)],
      ["export_format", envelope.format],
      ["document_id", envelope.source.document_id],
      ["semantic_projection_sha256", envelope.source.semantic_projection_sha256],
      ["semantic_sha256", envelope.semantic_sha256],
      ["selected_table_ids", envelope.selection.table_ids.join("\u001f")],
      ["selected_chart_ids", envelope.selection.chart_ids.join("\u001f")],
      ["semantic_payload_chunk_count", String(payloadChunks.length)],
      ...payloadChunks.map((chunk, index) => [
        `${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID}.${String(index).padStart(6, "0")}`,
        chunk,
      ]),
    ],
  };
  const selected = envelope.tables.map<ResultTable>((table) => ({
    id: table.id,
    title: table.id,
    status: "validated",
    warning: `Canonical table title: ${table.title}. Source document: ${envelope.source.document_id}.`,
    columns: ["Row ID", ...table.columns.map((column) => `${column.label} [${column.id}]`)],
    rows: table.rows.map((row) => [row.id, ...row.cells.map(semanticCellText)]),
  }));
  const provenance: ResultTable = {
    id: XLSX_PROVENANCE_TABLE_ID,
    title: XLSX_PROVENANCE_TABLE_ID,
    status: "validated",
    warning: "CanonicalResultDocumentV2 provenance for the selected immutable result.",
    columns: ["Field ID", "Value"],
    rows: [
      ["document_id", envelope.source.document_id],
      ["run_id", envelope.provenance.run_id],
      ["project_id", envelope.provenance.project_id],
      ["model_id", envelope.provenance.model_id],
      ["model_digest", envelope.provenance.model_digest],
      ["dataset_id", envelope.provenance.dataset_id],
      ["dataset_fingerprint", envelope.provenance.dataset_fingerprint],
      ["recipe_id", envelope.provenance.recipe_id],
      ["recipe_digest", envelope.provenance.recipe_digest],
      ["capability_id", envelope.provenance.capability_cell.capability_id],
      ["capability_cell_id", envelope.provenance.capability_cell.cell_id],
      ["capability_version", envelope.provenance.capability_cell.capability_version],
      ["method_version", envelope.provenance.method_version],
      ["engine_version", envelope.provenance.engine_version],
      ["seed", envelope.provenance.seed === null ? "null" : String(envelope.provenance.seed)],
      ["workers", String(envelope.provenance.workers)],
      ["started_at", envelope.provenance.started_at],
      ["completed_at", envelope.provenance.completed_at],
    ],
  };
  return [manifest, ...selected, provenance];
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function formatChartNumber(value: number): string {
  if (Object.is(value, -0) || value === 0) return "0";
  const absolute = Math.abs(value);
  if (absolute >= 1_000_000 || absolute < 0.0001) return value.toExponential(3);
  return value.toFixed(4).replace(/\.?0+$/u, "");
}

interface ChartProjectionV2 {
  x: (value: number | string) => number;
  y: (value: number) => number;
  xTicks: Array<{ value: number | string; coordinate: number; label: string }>;
  yTicks: Array<{ value: number; coordinate: number; label: string }>;
  plot: { left: number; right: number; top: number; bottom: number };
}

function paddedExtent(minimum: number, maximum: number, includeZero = false): readonly [number, number] {
  let min = includeZero ? Math.min(minimum, 0) : minimum;
  let max = includeZero ? Math.max(maximum, 0) : maximum;
  if (min === max) {
    const padding = Math.abs(min) > 0 ? Math.abs(min) * 0.1 : 1;
    return [min - padding, max + padding];
  }
  const padding = (max - min) * 0.08;
  min -= padding;
  max += padding;
  return [min, max];
}

function chartProjection(chart: CanonicalResultChart, width: number, height: number): ChartProjectionV2 | null {
  const points = chart.series.flatMap((series) => series.points);
  if (!points.length) return null;
  const plot = { left: 74, right: width - 26, top: 58, bottom: height - 74 };
  const yValues = points.flatMap((point) => [point.y, point.lower, point.upper])
    .filter((value): value is number => typeof value === "number" && Number.isFinite(value));
  if (!yValues.length) return null;
  const [yMin, yMax] = paddedExtent(Math.min(...yValues), Math.max(...yValues), chart.kind === "bar");
  const y = (value: number) => plot.bottom - ((value - yMin) / (yMax - yMin)) * (plot.bottom - plot.top);
  const yTicks = Array.from({ length: 5 }, (_, index) => {
    const value = yMin + ((yMax - yMin) * index) / 4;
    return { value, coordinate: y(value), label: formatChartNumber(value) };
  });
  const numeric = points.every((point) => typeof point.x === "number" && Number.isFinite(point.x));
  if (numeric) {
    const values = points.map((point) => point.x as number);
    const [xMin, xMax] = paddedExtent(Math.min(...values), Math.max(...values));
    const x = (value: number | string) => plot.left + ((Number(value) - xMin) / (xMax - xMin)) * (plot.right - plot.left);
    const unique = [...new Set(values)].sort((left, right) => left - right);
    const tickValues = unique.length <= 7
      ? unique
      : Array.from({ length: 5 }, (_, index) => xMin + ((xMax - xMin) * index) / 4);
    return { x, y, xTicks: tickValues.map((value) => ({ value, coordinate: x(value), label: formatChartNumber(value) })), yTicks, plot };
  }
  const categories: string[] = [];
  for (const point of points) {
    const value = String(point.x);
    if (!categories.includes(value)) categories.push(value);
  }
  const x = (value: number | string) => {
    const index = Math.max(0, categories.indexOf(String(value)));
    if (categories.length === 1) return (plot.left + plot.right) / 2;
    return plot.left + (index / (categories.length - 1)) * (plot.right - plot.left);
  };
  return { x, y, xTicks: categories.map((value) => ({ value, coordinate: x(value), label: value })), yTicks, plot };
}

function pointAccessibleText(seriesLabel: string, point: CanonicalChartPoint): string {
  const identity = point.label ? `, effect ${point.label}` : "";
  const interval = point.lower != null || point.upper != null
    ? `, interval ${point.lower == null ? "not reported" : formatChartNumber(point.lower)} to ${point.upper == null ? "not reported" : formatChartNumber(point.upper)}`
    : "";
  return `${seriesLabel}: x ${String(point.x)}, y ${formatChartNumber(point.y)}${identity}${interval}`;
}

function canonicalChartSvg(
  chart: CanonicalResultChart,
  envelope: CanonicalResultExportSemanticEnvelopeV2,
  embedded = false,
): string {
  const width = 960;
  const height = 540;
  const projection = chartProjection(chart, width, height);
  const metadata = encodeEnvelope(envelope);
  const prefix = embedded ? "" : '<?xml version="1.0" encoding="UTF-8"?>\n';
  const seriesMarkup = projection ? chart.series.map((series, seriesIndex) => {
    const color = `rgb(${CHART_COLORS[seriesIndex % CHART_COLORS.length]!.slice(0, 3).join(",")})`;
    const points = series.points.map((point) => ({ point, x: projection.x(point.x), y: projection.y(point.y) }));
    const path = points.map(({ x, y }, index) => `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`).join(" ");
    const bars = chart.kind === "bar" ? points.map(({ point, x, y }, index) => {
      const zero = projection.y(0);
      const barWidth = Math.max(8, Math.min(42, (projection.plot.right - projection.plot.left) / Math.max(4, points.length * chart.series.length + 1)));
      const offset = (seriesIndex - (chart.series.length - 1) / 2) * barWidth;
      return `<rect data-canonical-point-index="${index}" x="${(x + offset - barWidth * 0.42).toFixed(2)}" y="${Math.min(y, zero).toFixed(2)}" width="${(barWidth * 0.84).toFixed(2)}" height="${Math.abs(zero - y).toFixed(2)}" fill="${color}"><title>${escapeHtml(pointAccessibleText(series.label, point))}</title></rect>`;
    }).join("") : "";
    const intervals = points.map(({ point, x, y }) => {
      if (point.lower == null && point.upper == null) return "";
      const lower = point.lower == null ? y : projection.y(point.lower);
      const upper = point.upper == null ? y : projection.y(point.upper);
      return `<line x1="${x.toFixed(2)}" x2="${x.toFixed(2)}" y1="${lower.toFixed(2)}" y2="${upper.toFixed(2)}" stroke="${color}" stroke-width="2"/>`;
    }).join("");
    const marks = chart.kind === "bar" ? "" : points.map(({ point, x, y }, index) => (
      `<circle data-canonical-point-index="${index}" cx="${x.toFixed(2)}" cy="${y.toFixed(2)}" r="4" fill="${color}"><title>${escapeHtml(pointAccessibleText(series.label, point))}</title></circle>`
    )).join("");
    const line = chart.kind === "line" || chart.kind === "interval"
      ? `<path d="${path}" fill="none" stroke="${color}" stroke-width="2.5"/>`
      : "";
    return `<g data-canonical-series-id="${escapeHtml(series.id)}">${line}${bars}${intervals}${marks}</g>`;
  }).join("") : "";
  const axes = projection ? `<g aria-hidden="true">
${projection.yTicks.map((tick) => `<line x1="${projection.plot.left}" x2="${projection.plot.right}" y1="${tick.coordinate.toFixed(2)}" y2="${tick.coordinate.toFixed(2)}" stroke="#d7dee2"/><text x="${projection.plot.left - 10}" y="${(tick.coordinate + 4).toFixed(2)}" text-anchor="end" font-size="12" fill="#44525a">${escapeHtml(tick.label)}</text>`).join("")}
<line x1="${projection.plot.left}" x2="${projection.plot.left}" y1="${projection.plot.top}" y2="${projection.plot.bottom}" stroke="#26343c"/>
<line x1="${projection.plot.left}" x2="${projection.plot.right}" y1="${projection.plot.bottom}" y2="${projection.plot.bottom}" stroke="#26343c"/>
${projection.xTicks.map((tick) => `<text x="${tick.coordinate.toFixed(2)}" y="${projection.plot.bottom + 24}" text-anchor="middle" font-size="12" fill="#44525a">${escapeHtml(tick.label)}</text>`).join("")}
</g>` : "";
  const legend = chart.series.map((series, index) => {
    const color = `rgb(${CHART_COLORS[index % CHART_COLORS.length]!.slice(0, 3).join(",")})`;
    const x = 28 + (index % 3) * 300;
    const y = 50 + Math.floor(index / 3) * 16;
    return `<g data-canonical-legend-series-id="${escapeHtml(series.id)}"><rect x="${x}" y="${y - 9}" width="12" height="8" fill="${color}"/><text x="${x + 18}" y="${y}" font-family="Arial, sans-serif" font-size="12" fill="#2d3940">${escapeHtml(series.label)}</text></g>`;
  }).join("");
  const axisLabels = projection ? `<g aria-hidden="true">${chart.display.x_axis_label ? `<text x="${width / 2}" y="${height - 43}" text-anchor="middle" font-family="Arial, sans-serif" font-size="12" fill="#26343c">${escapeHtml(chart.display.x_axis_label)}</text>` : ""}${chart.display.y_axis_label ? `<text x="${projection.plot.left}" y="${height - 43}" font-family="Arial, sans-serif" font-size="12" fill="#26343c">Y: ${escapeHtml(chart.display.y_axis_label)}</text>` : ""}</g>` : "";
  const empty = projection ? "" : `<text x="${width / 2}" y="${height / 2}" text-anchor="middle" fill="#44525a">No persisted chart points.</text>`;
  return `${prefix}<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="chart-title chart-description" data-canonical-document-id="${escapeHtml(envelope.source.document_id)}" data-canonical-chart-id="${escapeHtml(chart.id)}">
<title id="chart-title">${escapeHtml(chart.title)}</title>
<desc id="chart-description">${escapeHtml(chart.description)} Source run ${escapeHtml(envelope.provenance.run_id)}; method ${escapeHtml(envelope.provenance.method_version)}.</desc>
<metadata id="${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID}">${metadata}</metadata>
<rect width="100%" height="100%" fill="#ffffff"/>
<text x="28" y="34" font-family="Arial, sans-serif" font-size="21" font-weight="700" fill="#172126">${escapeHtml(chart.title)}</text>
${legend}${axes}${seriesMarkup}${axisLabels}${empty}
<text x="28" y="${height - 18}" font-family="Arial, sans-serif" font-size="11" fill="#596870">Document ${escapeHtml(envelope.source.document_id)} · chart ${escapeHtml(chart.id)} · dataset ${escapeHtml(envelope.provenance.dataset_fingerprint)}</text>
</svg>`;
}

function canonicalHtml(envelope: CanonicalResultExportSemanticEnvelopeV2): string {
  const metadata = encodeEnvelope(envelope);
  const tables = envelope.tables.map((table) => {
    const header = table.columns.map((column) => `<th scope="col" data-canonical-column-id="${escapeHtml(column.id)}" title="${escapeHtml(column.description)}">${escapeHtml(column.label)}</th>`).join("");
    const rows = table.rows.map((row) => `<tr data-canonical-row-id="${escapeHtml(row.id)}"><th scope="row">${escapeHtml(row.id)}</th>${row.cells.map((cell, index) => {
      const precision = Math.max(0, Math.min(12, table.columns[index]?.default_precision ?? envelope.presentation.precision));
      return `<td data-canonical-cell-kind="${cell.kind}">${escapeHtml(displayCellText(cell, precision, envelope.presentation.missing_value_label))}</td>`;
    }).join("")}</tr>`).join("\n");
    return `<section aria-labelledby="table-${escapeHtml(table.id)}-heading" data-canonical-table-id="${escapeHtml(table.id)}"><h2 id="table-${escapeHtml(table.id)}-heading">${escapeHtml(table.title)}</h2>${table.description ? `<p>${escapeHtml(table.description)}</p>` : ""}<table><caption>Canonical table ID: <code>${escapeHtml(table.id)}</code></caption><thead><tr><th scope="col">Row ID</th>${header}</tr></thead><tbody>${rows}</tbody></table></section>`;
  }).join("\n");
  const charts = envelope.charts.map((chart) => `<figure data-canonical-chart-id="${escapeHtml(chart.id)}">${canonicalChartSvg(chart, envelope, true)}</figure>`).join("\n");
  const notices = envelope.notices.length
    ? `<section aria-labelledby="notices-heading"><h2 id="notices-heading">Notices</h2><ul>${envelope.notices.map((notice) => `<li data-canonical-notice-id="${escapeHtml(notice.id)}"><strong>${escapeHtml(notice.severity)}</strong> ${escapeHtml(notice.message)}</li>`).join("")}</ul></section>`
    : "";
  return `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; img-src data:"><title>${escapeHtml(envelope.title)} · QuickPLS</title><style>
:root{font-family:Arial,sans-serif;color:#172126;background:#fff}body{max-width:1180px;margin:0 auto;padding:28px}header,section,figure{margin:0 0 28px}dl{display:grid;grid-template-columns:max-content 1fr;gap:6px 14px}dt{font-weight:700}dd{margin:0;overflow-wrap:anywhere}table{border-collapse:collapse;width:100%;font-size:13px}caption{text-align:left;margin:0 0 8px}th,td{border:1px solid #cbd4d9;padding:7px 9px;text-align:left;vertical-align:top}thead th{background:#eef3f4}tbody th{font-family:monospace;font-weight:400;background:#f8fafb}code{overflow-wrap:anywhere}svg{max-width:100%;height:auto;border:1px solid #d7dee2}@media print{body{max-width:none;padding:0}section,figure{break-inside:avoid}a{color:inherit}}
</style></head><body data-canonical-document-id="${escapeHtml(envelope.source.document_id)}"><script type="application/vnd.quickpls.canonical-semantic+base64" id="${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID}">${metadata}</script>
<header><h1>${escapeHtml(envelope.title)}</h1><p>Self-contained CanonicalResultDocumentV2 export.</p><dl><dt>Document ID</dt><dd><code>${escapeHtml(envelope.source.document_id)}</code></dd><dt>Run ID</dt><dd><code>${escapeHtml(envelope.provenance.run_id)}</code></dd><dt>Method</dt><dd><code>${escapeHtml(envelope.provenance.method_version)}</code></dd><dt>Dataset fingerprint</dt><dd><code>${escapeHtml(envelope.provenance.dataset_fingerprint)}</code></dd><dt>Semantic SHA-256</dt><dd><code>${envelope.semantic_sha256}</code></dd></dl></header>
${notices}${tables}${charts}</body></html>`;
}

function wrapText(value: string, width = 96): string[] {
  if (!value) return [""];
  return Array.from({ length: Math.ceil(value.length / width) }, (_, index) => (
    value.slice(index * width, (index + 1) * width)
  ));
}

function pdfEscape(value: string): string {
  return value.replaceAll("\\", "\\\\").replaceAll("(", "\\(").replaceAll(")", "\\)");
}

function pdfRawReportLines(envelope: CanonicalResultExportSemanticEnvelopeV2): string[] {
  const lines = [
    envelope.title,
    `Document ID: ${envelope.source.document_id}`,
    `Run ID: ${envelope.provenance.run_id}`,
    `Project ID: ${envelope.provenance.project_id}`,
    `Model: ${envelope.provenance.model_id} (${envelope.provenance.model_digest})`,
    `Dataset: ${envelope.provenance.dataset_id} (${envelope.provenance.dataset_fingerprint})`,
    `Recipe: ${envelope.provenance.recipe_id} (${envelope.provenance.recipe_digest})`,
    `Method: ${envelope.provenance.method_version}`,
    `Capability cell: ${envelope.provenance.capability_cell.cell_id}`,
    `Semantic SHA-256: ${envelope.semantic_sha256}`,
    "",
  ];
  for (const table of envelope.tables) {
    lines.push(`[table:${table.id}] ${table.title}`);
    lines.push(`row_id | ${table.columns.map((column) => `${column.label} [${column.id}]`).join(" | ")}`);
    for (const row of table.rows) lines.push(`${row.id} | ${row.cells.map(semanticCellText).join(" | ")}`);
    lines.push("");
  }
  for (const chart of envelope.charts) {
    lines.push(`[chart:${chart.id}] ${chart.title}`);
    for (const series of chart.series) {
      lines.push(`series:${series.id} ${series.label}`);
      for (const point of series.points) lines.push(pointAccessibleText(series.label, point));
    }
    lines.push("");
  }
  return lines;
}

function visibleGlyph(value: string): string | null {
  return Array.from(value).find((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint < 0x20 || codePoint > 0x7e;
  }) ?? null;
}

function glyphDescription(glyph: string): string {
  const codePoint = glyph.codePointAt(0) ?? 0;
  return `${JSON.stringify(glyph)} (U+${codePoint.toString(16).toUpperCase().padStart(4, "0")})`;
}

function pdfVisibleTextError(envelope: CanonicalResultExportSemanticEnvelopeV2): string | null {
  for (const line of pdfRawReportLines(envelope)) {
    const glyph = visibleGlyph(line);
    if (glyph) {
      return `PDF export cannot render canonical glyph ${glyphDescription(glyph)} with its bundled offline Courier font. Use HTML, XLSX, CSV, or SVG for this text; no file was written.`;
    }
  }
  return null;
}

function pdfReportLines(envelope: CanonicalResultExportSemanticEnvelopeV2): string[] {
  return pdfRawReportLines(envelope).flatMap((line) => wrapText(line));
}

function canonicalPdf(envelope: CanonicalResultExportSemanticEnvelopeV2): Uint8Array {
  const metadata = encodeEnvelope(envelope);
  const metadataLines = metadata.match(/.{1,76}/gu) ?? [];
  const reportLines = pdfReportLines(envelope);
  const linesPerPage = 54;
  const pages = Array.from({ length: Math.max(1, Math.ceil(reportLines.length / linesPerPage)) }, (_, index) => (
    reportLines.slice(index * linesPerPage, (index + 1) * linesPerPage)
  ));
  const objects: string[] = [];
  const pageObjectNumbers = pages.map((_, index) => 4 + index * 2);
  objects.push("<< /Type /Catalog /Pages 2 0 R >>");
  objects.push(`<< /Type /Pages /Count ${pages.length} /Kids [${pageObjectNumbers.map((number) => `${number} 0 R`).join(" ")}] >>`);
  objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>");
  pages.forEach((page, pageIndex) => {
    const pageNumber = 4 + pageIndex * 2;
    const contentNumber = pageNumber + 1;
    objects.push(`<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents ${contentNumber} 0 R >>`);
    const content = `BT\n/F1 8 Tf\n38 808 Td\n12 TL\n${page.map((line) => `(${pdfEscape(line)}) Tj\nT*`).join("\n")}\nET`;
    objects.push(`<< /Length ${new TextEncoder().encode(content).length} >>\nstream\n${content}\nendstream`);
  });
  let pdf = `%PDF-1.4\n%QuickPLS CanonicalResultDocumentV2\n${PDF_METADATA_BEGIN}\n${metadataLines.map((line) => `${PDF_METADATA_LINE}${line}`).join("\n")}\n${PDF_METADATA_END}\n`;
  const offsets = [0];
  objects.forEach((object, index) => {
    offsets.push(new TextEncoder().encode(pdf).length);
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`;
  });
  const xrefOffset = new TextEncoder().encode(pdf).length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets.slice(1)) pdf += `${String(offset).padStart(10, "0")} 00000 n \n`;
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  return new TextEncoder().encode(pdf);
}

function setPixel(pixels: Uint8Array, width: number, height: number, x: number, y: number, color: readonly number[]) {
  const roundedX = Math.round(x);
  const roundedY = Math.round(y);
  if (roundedX < 0 || roundedY < 0 || roundedX >= width || roundedY >= height) return;
  const offset = (roundedY * width + roundedX) * 4;
  pixels[offset] = color[0] ?? 0;
  pixels[offset + 1] = color[1] ?? 0;
  pixels[offset + 2] = color[2] ?? 0;
  pixels[offset + 3] = color[3] ?? 255;
}

function drawLine(pixels: Uint8Array, width: number, height: number, x0: number, y0: number, x1: number, y1: number, color: readonly number[], thickness = 1) {
  let left = Math.round(x0);
  let top = Math.round(y0);
  const right = Math.round(x1);
  const bottom = Math.round(y1);
  const dx = Math.abs(right - left);
  const sx = left < right ? 1 : -1;
  const dy = -Math.abs(bottom - top);
  const sy = top < bottom ? 1 : -1;
  let error = dx + dy;
  while (true) {
    for (let offsetX = -Math.floor(thickness / 2); offsetX <= Math.floor(thickness / 2); offsetX += 1) {
      for (let offsetY = -Math.floor(thickness / 2); offsetY <= Math.floor(thickness / 2); offsetY += 1) {
        setPixel(pixels, width, height, left + offsetX, top + offsetY, color);
      }
    }
    if (left === right && top === bottom) break;
    const twice = 2 * error;
    if (twice >= dy) { error += dy; left += sx; }
    if (twice <= dx) { error += dx; top += sy; }
  }
}

function fillRectangle(pixels: Uint8Array, width: number, height: number, x: number, y: number, rectangleWidth: number, rectangleHeight: number, color: readonly number[]) {
  const left = Math.max(0, Math.floor(x));
  const right = Math.min(width, Math.ceil(x + rectangleWidth));
  const top = Math.max(0, Math.floor(y));
  const bottom = Math.min(height, Math.ceil(y + rectangleHeight));
  for (let row = top; row < bottom; row += 1) {
    for (let column = left; column < right; column += 1) setPixel(pixels, width, height, column, row, color);
  }
}

function drawCircle(pixels: Uint8Array, width: number, height: number, centerX: number, centerY: number, radius: number, color: readonly number[]) {
  for (let y = -radius; y <= radius; y += 1) {
    for (let x = -radius; x <= radius; x += 1) {
      if (x * x + y * y <= radius * radius) setPixel(pixels, width, height, centerX + x, centerY + y, color);
    }
  }
}

const BITMAP_FONT_5X7: Readonly<Record<string, readonly number[]>> = {
  " ": [0, 0, 0, 0, 0, 0, 0],
  A: [14, 17, 17, 31, 17, 17, 17], B: [30, 17, 17, 30, 17, 17, 30], C: [14, 17, 16, 16, 16, 17, 14],
  D: [30, 17, 17, 17, 17, 17, 30], E: [31, 16, 16, 30, 16, 16, 31], F: [31, 16, 16, 30, 16, 16, 16],
  G: [14, 17, 16, 23, 17, 17, 15], H: [17, 17, 17, 31, 17, 17, 17], I: [31, 4, 4, 4, 4, 4, 31],
  J: [7, 2, 2, 2, 2, 18, 12], K: [17, 18, 20, 24, 20, 18, 17], L: [16, 16, 16, 16, 16, 16, 31],
  M: [17, 27, 21, 21, 17, 17, 17], N: [17, 25, 21, 19, 17, 17, 17], O: [14, 17, 17, 17, 17, 17, 14],
  P: [30, 17, 17, 30, 16, 16, 16], Q: [14, 17, 17, 17, 21, 18, 13], R: [30, 17, 17, 30, 20, 18, 17],
  S: [15, 16, 16, 14, 1, 1, 30], T: [31, 4, 4, 4, 4, 4, 4], U: [17, 17, 17, 17, 17, 17, 14],
  V: [17, 17, 17, 17, 17, 10, 4], W: [17, 17, 17, 21, 21, 21, 10], X: [17, 17, 10, 4, 10, 17, 17],
  Y: [17, 17, 10, 4, 4, 4, 4], Z: [31, 1, 2, 4, 8, 16, 31],
  a: [0, 0, 14, 1, 15, 17, 15], b: [16, 16, 30, 17, 17, 17, 30], c: [0, 0, 14, 17, 16, 17, 14],
  d: [1, 1, 15, 17, 17, 17, 15], e: [0, 0, 14, 17, 31, 16, 14], f: [6, 9, 8, 28, 8, 8, 8],
  g: [0, 0, 15, 17, 15, 1, 14], h: [16, 16, 30, 17, 17, 17, 17], i: [4, 0, 12, 4, 4, 4, 14],
  j: [2, 0, 6, 2, 2, 18, 12], k: [16, 16, 18, 20, 24, 20, 18], l: [12, 4, 4, 4, 4, 4, 14],
  m: [0, 0, 26, 21, 21, 17, 17], n: [0, 0, 30, 17, 17, 17, 17], o: [0, 0, 14, 17, 17, 17, 14],
  p: [0, 0, 30, 17, 30, 16, 16], q: [0, 0, 15, 17, 15, 1, 1], r: [0, 0, 22, 25, 16, 16, 16],
  s: [0, 0, 15, 16, 14, 1, 30], t: [8, 8, 28, 8, 8, 9, 6], u: [0, 0, 17, 17, 17, 19, 13],
  v: [0, 0, 17, 17, 17, 10, 4], w: [0, 0, 17, 17, 21, 21, 10], x: [0, 0, 17, 10, 4, 10, 17],
  y: [0, 0, 17, 17, 15, 1, 14], z: [0, 0, 31, 2, 4, 8, 31],
  "0": [14, 17, 19, 21, 25, 17, 14], "1": [4, 12, 4, 4, 4, 4, 14], "2": [14, 17, 1, 2, 4, 8, 31],
  "3": [30, 1, 1, 14, 1, 1, 30], "4": [2, 6, 10, 18, 31, 2, 2], "5": [31, 16, 16, 30, 1, 1, 30],
  "6": [14, 16, 16, 30, 17, 17, 14], "7": [31, 1, 2, 4, 8, 8, 8], "8": [14, 17, 17, 14, 17, 17, 14],
  "9": [14, 17, 17, 15, 1, 1, 14],
  "-": [0, 0, 0, 31, 0, 0, 0], "+": [0, 4, 4, 31, 4, 4, 0], "=": [0, 31, 0, 31, 0, 0, 0],
  ".": [0, 0, 0, 0, 0, 12, 12], ",": [0, 0, 0, 0, 0, 12, 8], ":": [0, 12, 12, 0, 12, 12, 0],
  "/": [1, 2, 2, 4, 8, 8, 16], "_": [0, 0, 0, 0, 0, 0, 31], "(": [2, 4, 8, 8, 8, 4, 2],
  ")": [8, 4, 2, 2, 2, 4, 8], "[": [14, 8, 8, 8, 8, 8, 14], "]": [14, 2, 2, 2, 2, 2, 14],
  "?": [14, 17, 1, 2, 4, 0, 4], "'": [4, 4, 8, 0, 0, 0, 0], "*": [0, 21, 14, 31, 14, 21, 0],
};

function bitmapTextWidth(value: string, scale: number): number {
  return Math.max(0, value.length * 6 * scale - scale);
}

function drawBitmapText(
  pixels: Uint8Array,
  width: number,
  height: number,
  value: string,
  x: number,
  y: number,
  scale: number,
  color: readonly number[],
  maxWidth: number,
) {
  if (bitmapTextWidth(value, scale) > maxWidth) throw new Error("PNG bitmap text exceeded its preflight width.");
  let cursor = Math.round(x);
  for (const character of value) {
    const rows = BITMAP_FONT_5X7[character];
    if (!rows) throw new Error(`PNG bitmap text preflight missed ${glyphDescription(character)}.`);
    rows.forEach((bits, row) => {
      for (let column = 0; column < 5; column += 1) {
        if ((bits & (1 << (4 - column))) !== 0) {
          fillRectangle(pixels, width, height, cursor + column * scale, y + row * scale, scale, scale, color);
        }
      }
    });
    cursor += 6 * scale;
  }
}

interface PngVisibleTextSurfaceV2 {
  label: string;
  value: string;
  scale: number;
  maxWidth: number;
}

function pngVisibleTextSurfaces(
  chart: CanonicalResultChart,
  envelope: CanonicalResultExportSemanticEnvelopeV2,
): PngVisibleTextSurfaceV2[] {
  const projection = chartProjection(chart, PNG_WIDTH, PNG_HEIGHT);
  const surfaces: PngVisibleTextSurfaceV2[] = [
    { label: "chart title", value: chart.title, scale: 2, maxWidth: PNG_WIDTH - 48 },
    ...chart.series.map((series) => ({ label: `series label ${series.id}`, value: series.label, scale: 1, maxWidth: 260 })),
    { label: "export footer", value: `DOC ${envelope.source.document_id} CHART ${chart.id}`, scale: 1, maxWidth: PNG_WIDTH - 48 },
  ];
  if (projection) {
    surfaces.push(...projection.yTicks.map((tick) => ({ label: "y-axis tick", value: tick.label, scale: 1, maxWidth: projection.plot.left - 14 })));
    surfaces.push(...projection.xTicks.map((tick) => ({ label: "x-axis tick", value: tick.label, scale: 1, maxWidth: 84 })));
    if (chart.display.x_axis_label) surfaces.push({ label: "x-axis label", value: chart.display.x_axis_label, scale: 1, maxWidth: PNG_WIDTH - 100 });
    if (chart.display.y_axis_label) surfaces.push({ label: "y-axis label", value: `Y: ${chart.display.y_axis_label}`, scale: 1, maxWidth: 280 });
  }
  return surfaces;
}

function pngVisibleTextError(
  chart: CanonicalResultChart,
  envelope: CanonicalResultExportSemanticEnvelopeV2,
): string | null {
  for (const surface of pngVisibleTextSurfaces(chart, envelope)) {
    const unsupported = Array.from(surface.value).find((character) => !BITMAP_FONT_5X7[character]);
    if (unsupported) {
      return `PNG export cannot render canonical glyph ${glyphDescription(unsupported)} in ${surface.label} with its bundled offline bitmap font. Use SVG for this label; no file was written.`;
    }
    if (bitmapTextWidth(surface.value, surface.scale) > surface.maxWidth) {
      return `PNG export cannot fit the exact ${surface.label} without truncation. Shorten that visible label or use SVG; no file was written.`;
    }
  }
  return null;
}

function crc32(bytes: Uint8Array): number {
  let crc = 0xffff_ffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) crc = (crc >>> 1) ^ (0xedb8_8320 & -(crc & 1));
  }
  return (crc ^ 0xffff_ffff) >>> 0;
}

function adler32(bytes: Uint8Array): number {
  let a = 1;
  let b = 0;
  for (const byte of bytes) {
    a = (a + byte) % 65_521;
    b = (b + a) % 65_521;
  }
  return ((b << 16) | a) >>> 0;
}

function uint32Bytes(value: number): Uint8Array {
  return Uint8Array.from([(value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff]);
}

function concatenateBytes(parts: readonly Uint8Array[]): Uint8Array {
  const result = new Uint8Array(parts.reduce((total, part) => total + part.length, 0));
  let offset = 0;
  for (const part of parts) { result.set(part, offset); offset += part.length; }
  return result;
}

function pngChunk(type: string, data: Uint8Array): Uint8Array {
  const typeBytes = new TextEncoder().encode(type);
  return concatenateBytes([uint32Bytes(data.length), typeBytes, data, uint32Bytes(crc32(concatenateBytes([typeBytes, data])))]);
}

function deflateStored(bytes: Uint8Array): Uint8Array {
  const blocks: Uint8Array[] = [Uint8Array.from([0x78, 0x01])];
  for (let offset = 0; offset < bytes.length; offset += 65_535) {
    const length = Math.min(65_535, bytes.length - offset);
    const final = offset + length >= bytes.length ? 1 : 0;
    blocks.push(Uint8Array.from([final, length & 0xff, (length >>> 8) & 0xff, (~length) & 0xff, ((~length) >>> 8) & 0xff]));
    blocks.push(bytes.slice(offset, offset + length));
  }
  blocks.push(uint32Bytes(adler32(bytes)));
  return concatenateBytes(blocks);
}

function pngTextChunk(keyword: string, value: string): Uint8Array {
  return pngChunk("tEXt", concatenateBytes([new TextEncoder().encode(keyword), Uint8Array.from([0]), new TextEncoder().encode(value)]));
}

function canonicalPng(chart: CanonicalResultChart, envelope: CanonicalResultExportSemanticEnvelopeV2): Uint8Array {
  const pixels = new Uint8Array(PNG_WIDTH * PNG_HEIGHT * 4);
  pixels.fill(255);
  const projection = chartProjection(chart, PNG_WIDTH, PNG_HEIGHT);
  drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, chart.title, 24, 10, 2, [23, 33, 38, 255], PNG_WIDTH - 48);
  if (projection) {
    for (const tick of projection.yTicks) {
      drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, projection.plot.left, tick.coordinate, projection.plot.right, tick.coordinate, [218, 225, 229, 255]);
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, tick.label, 6, tick.coordinate - 3, 1, [68, 82, 90, 255], projection.plot.left - 14);
    }
    drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, projection.plot.left, projection.plot.top, projection.plot.left, projection.plot.bottom, [38, 52, 60, 255], 2);
    drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, projection.plot.left, projection.plot.bottom, projection.plot.right, projection.plot.bottom, [38, 52, 60, 255], 2);
    projection.xTicks.forEach((tick) => {
      const label = tick.label;
      const labelWidth = bitmapTextWidth(label, 1);
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, label, tick.coordinate - labelWidth / 2, projection.plot.bottom + 12, 1, [68, 82, 90, 255], Math.max(42, labelWidth));
    });
    chart.series.forEach((series, seriesIndex) => {
      const color = CHART_COLORS[seriesIndex % CHART_COLORS.length]!;
      const legendX = 24 + (seriesIndex % 3) * 300;
      const legendY = 34 + Math.floor(seriesIndex / 3) * 14;
      fillRectangle(pixels, PNG_WIDTH, PNG_HEIGHT, legendX, legendY, 12, 7, color);
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, series.label, legendX + 18, legendY, 1, [45, 57, 64, 255], 260);
      const points = series.points.map((point) => ({ point, x: projection.x(point.x), y: projection.y(point.y) }));
      if (chart.kind === "line" || chart.kind === "interval") {
        for (let index = 1; index < points.length; index += 1) {
          const previous = points[index - 1]!;
          const current = points[index]!;
          drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, previous.x, previous.y, current.x, current.y, color, 3);
        }
      }
      points.forEach(({ point, x, y }) => {
        if (point.lower != null || point.upper != null) drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, x, projection.y(point.lower ?? point.y), x, projection.y(point.upper ?? point.y), color, 2);
        if (chart.kind === "bar") {
          const zero = projection.y(0);
          const barWidth = Math.max(8, Math.min(36, (projection.plot.right - projection.plot.left) / Math.max(4, points.length * chart.series.length + 1)));
          const offset = (seriesIndex - (chart.series.length - 1) / 2) * barWidth;
          fillRectangle(pixels, PNG_WIDTH, PNG_HEIGHT, x + offset - barWidth * 0.4, Math.min(y, zero), barWidth * 0.8, Math.abs(y - zero), color);
        } else {
          drawCircle(pixels, PNG_WIDTH, PNG_HEIGHT, x, y, chart.kind === "heatmap" ? 8 : 5, color);
        }
      });
    });
    if (chart.display.x_axis_label) {
      const label = chart.display.x_axis_label;
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, label, (PNG_WIDTH - bitmapTextWidth(label, 1)) / 2, PNG_HEIGHT - 43, 1, [38, 52, 60, 255], PNG_WIDTH - 100);
    }
    if (chart.display.y_axis_label) drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, `Y: ${chart.display.y_axis_label}`, projection.plot.left, PNG_HEIGHT - 43, 1, [38, 52, 60, 255], 280);
  }
  drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, `DOC ${envelope.source.document_id} CHART ${chart.id}`, 24, PNG_HEIGHT - 18, 1, [89, 104, 112, 255], PNG_WIDTH - 48);
  const scanlines = new Uint8Array((PNG_WIDTH * 4 + 1) * PNG_HEIGHT);
  for (let row = 0; row < PNG_HEIGHT; row += 1) {
    const target = row * (PNG_WIDTH * 4 + 1);
    scanlines[target] = 0;
    scanlines.set(pixels.subarray(row * PNG_WIDTH * 4, (row + 1) * PNG_WIDTH * 4), target + 1);
  }
  const header = new Uint8Array(13);
  header.set(uint32Bytes(PNG_WIDTH), 0);
  header.set(uint32Bytes(PNG_HEIGHT), 4);
  header.set([8, 6, 0, 0, 0], 8);
  return concatenateBytes([
    Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk("IHDR", header),
    pngTextChunk(PNG_SEMANTIC_KEYWORD, encodeEnvelope(envelope)),
    pngTextChunk("Document ID", envelope.source.document_id),
    pngTextChunk("Chart ID", chart.id),
    pngTextChunk("Run ID", envelope.provenance.run_id),
    pngTextChunk("Method version", envelope.provenance.method_version),
    pngTextChunk("Dataset fingerprint", envelope.provenance.dataset_fingerprint),
    pngChunk("IDAT", deflateStored(scanlines)),
    pngChunk("IEND", new Uint8Array()),
  ]);
}

function safeFileToken(value: string): string {
  const token = value.toLowerCase().replace(/[^a-z0-9._-]+/gu, "-").replace(/^-+|-+$/gu, "");
  return token.slice(0, 72) || "result";
}

function fileNameFor(format: CanonicalResultExportFormatV2, envelope: CanonicalResultExportSemanticEnvelopeV2): string {
  const chartSuffix = (format === "svg" || format === "png") && envelope.selection.chart_ids[0]
    ? `-${safeFileToken(envelope.selection.chart_ids[0])}`
    : "";
  return `quickpls-${safeFileToken(envelope.source.document_id)}${chartSuffix}.${format}`;
}

function mediaTypeFor(format: CanonicalResultExportFormatV2): string {
  switch (format) {
    case "csv": return "text/csv;charset=utf-8";
    case "xlsx": return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
    case "html": return "text/html;charset=utf-8";
    case "pdf": return "application/pdf";
    case "svg": return "image/svg+xml;charset=utf-8";
    case "png": return "image/png";
  }
}

export function prepareCanonicalResultExportV2(
  document: CanonicalResultDocumentV2,
  request: CanonicalResultExportRequestV2,
): CanonicalResultExportPreparationV2 {
  const built = buildCanonicalResultSemanticExportV2(document);
  if (!built.ok) return { ok: false, code: "invalid_source_document", errors: built.errors };
  const projection = built.projection;
  const exportCharts = exportChartsForProjectionV2(projection).map((entry) => entry.chart);
  const availableTables = projection.tables.map((table) => table.id);
  const availableCharts = exportCharts.map((chart) => chart.id);
  const tableDefaultAll = request.format !== "svg" && request.format !== "png";
  const tableSelection = orderedSelection(request.tableIds, availableTables, tableDefaultAll, "tableIds");
  const chartDefaultAll = request.format === "html" || request.format === "pdf";
  const chartSelection = orderedSelection(request.chartIds, availableCharts, chartDefaultAll, "chartIds");
  const errors = [...tableSelection.errors, ...chartSelection.errors];
  if ((request.format === "csv" || request.format === "xlsx") && tableSelection.values.length === 0) {
    errors.push(`${request.format.toUpperCase()} export requires at least one selected canonical table.`);
  }
  if ((request.format === "svg" || request.format === "png") && chartSelection.values.length !== 1) {
    errors.push(`${request.format.toUpperCase()} export requires exactly one selected canonical chart.`);
  }
  if ((request.format === "html" || request.format === "pdf") && tableSelection.values.length === 0 && chartSelection.values.length === 0) {
    errors.push(`${request.format.toUpperCase()} export requires at least one selected table or chart.`);
  }
  if (errors.length) return { ok: false, code: "invalid_selection", errors };
  const envelope = buildSemanticEnvelope(projection, tableSelection.values, chartSelection.values, exportCharts);
  const base = {
    extension: request.format,
    mediaType: mediaTypeFor(request.format),
    defaultFileName: fileNameFor(request.format, envelope),
    semantic: envelope,
  } as const;
  switch (request.format) {
    case "csv": return { ok: true, artifact: { ...base, format: "csv", contents: canonicalCsv(envelope) } };
    case "xlsx": return { ok: true, artifact: { ...base, format: "xlsx", workbookTables: workbookTables(envelope) } };
    case "html": return { ok: true, artifact: { ...base, format: "html", contents: canonicalHtml(envelope) } };
    case "pdf": {
      const visibleTextError = pdfVisibleTextError(envelope);
      if (visibleTextError) return { ok: false, code: "unsupported_visible_text", errors: [visibleTextError] };
      return { ok: true, artifact: { ...base, format: "pdf", bytes: canonicalPdf(envelope) } };
    }
    case "svg": {
      const chart = envelope.charts[0];
      if (!chart) return { ok: false, code: "unsupported_chart", errors: ["The selected canonical chart is unavailable."] };
      return { ok: true, artifact: { ...base, format: "svg", contents: canonicalChartSvg(chart, envelope) } };
    }
    case "png": {
      const chart = envelope.charts[0];
      if (!chart) return { ok: false, code: "unsupported_chart", errors: ["The selected canonical chart is unavailable."] };
      const visibleTextError = pngVisibleTextError(chart, envelope);
      if (visibleTextError) return { ok: false, code: "unsupported_visible_text", errors: [visibleTextError] };
      return { ok: true, artifact: { ...base, format: "png", bytes: canonicalPng(chart, envelope) } };
    }
  }
}

function writerFor(
  artifact: PreparedCanonicalResultExportV2,
  writers: CanonicalResultExportWritersV2,
): ((artifact: never, signal?: AbortSignal) => Promise<string | null>) | undefined {
  if (artifact.format === "xlsx") return writers.workbook as ((artifact: never, signal?: AbortSignal) => Promise<string | null>) | undefined;
  if (artifact.format === "pdf" || artifact.format === "png") return writers.binary as ((artifact: never, signal?: AbortSignal) => Promise<string | null>) | undefined;
  return writers.text as ((artifact: never, signal?: AbortSignal) => Promise<string | null>) | undefined;
}

const ASYNC_EXPORT_PREPARATION_CHUNK_UNITS_V2 = 1_000_000;
type ExportPreparationCheckpointV2 = (units?: number, force?: boolean) => Promise<boolean>;
type AsyncCanonicalResultExportPreparationV2 = CanonicalResultExportPreparationV2 | { ok: false; code: "cancelled"; errors: [] };

function yieldExportPreparationTaskV2(): Promise<void> {
  // A timer task yields reliably in both WebView2 and the Node test runtime.
  // Node's MessagePort may stay referenced until its close notification is
  // drained, adding seconds of artificial latency to a handful of checkpoints.
  return new Promise<void>((resolve) => globalThis.setTimeout(resolve, 0));
}

function exportPreparationCheckpointV2(signal?: AbortSignal): ExportPreparationCheckpointV2 {
  let pendingUnits = 0;
  return async (units = 1, force = false): Promise<boolean> => {
    if (signal?.aborted) return false;
    pendingUnits += units;
    if (!force && pendingUnits < ASYNC_EXPORT_PREPARATION_CHUNK_UNITS_V2) return true;
    pendingUnits = 0;
    await yieldExportPreparationTaskV2();
    return !signal?.aborted;
  };
}

async function cloneCanonicalTableForExportV2(
  table: CanonicalResultTable,
  checkpoint: ExportPreparationCheckpointV2,
): Promise<CanonicalResultTable | null> {
  const { rows: sourceRows, ...tableWithoutRows } = table;
  const rows: CanonicalResultTable["rows"] = [];
  for (const [rowIndex, row] of sourceRows.entries()) {
    rows.push(structuredClone(row));
    if (!await checkpoint(row.cells.length + 1, rowIndex % 256 === 255)) return null;
  }
  return { ...structuredClone(tableWithoutRows), rows };
}

async function cloneCanonicalChartForExportV2(
  chart: CanonicalResultChart,
  checkpoint: ExportPreparationCheckpointV2,
): Promise<CanonicalResultChart | null> {
  const { series: sourceSeries, ...chartWithoutSeries } = chart;
  const series: CanonicalResultChart["series"] = [];
  for (const source of sourceSeries) {
    const { points: sourcePoints, ...seriesWithoutPoints } = source;
    const points: CanonicalChartPoint[] = [];
    for (const [pointIndex, point] of sourcePoints.entries()) {
      points.push(structuredClone(point));
      if (!await checkpoint(1, pointIndex % 256 === 255)) return null;
    }
    series.push({ ...structuredClone(seriesWithoutPoints), points });
  }
  return { ...structuredClone(chartWithoutSeries), series };
}

async function buildSemanticEnvelopeAsyncV2(
  projection: CanonicalResultSemanticExportV2,
  tableIds: string[],
  chartIds: string[],
  exportCharts: readonly CanonicalResultChart[],
  checkpoint: ExportPreparationCheckpointV2,
): Promise<CanonicalResultExportSemanticEnvelopeV2 | null> {
  const tableSet = new Set(tableIds);
  const chartSet = new Set(chartIds);
  const sections = projection.sections.filter((section) => (
    section.table_ids.some((id) => tableSet.has(id)) || section.chart_ids.some((id) => chartSet.has(id))
  )).map((section) => ({
    ...section,
    table_ids: section.table_ids.filter((id) => tableSet.has(id)),
    chart_ids: section.chart_ids.filter((id) => chartSet.has(id)),
  }));
  const tables: CanonicalResultTable[] = [];
  for (const table of projection.tables) {
    if (!tableSet.has(table.id)) continue;
    const cloned = await cloneCanonicalTableForExportV2(table, checkpoint);
    if (!cloned) return null;
    tables.push(cloned);
  }
  const charts: CanonicalResultChart[] = [];
  for (const chart of exportCharts) {
    if (!chartSet.has(chart.id)) continue;
    const cloned = await cloneCanonicalChartForExportV2(chart, checkpoint);
    if (!cloned) return null;
    charts.push(cloned);
  }
  const sectionSet = new Set(sections.map((section) => section.id));
  const footnoteSet = new Set(tables.flatMap((table) => table.footnote_ids));
  if (!await checkpoint(1, true)) return null;
  const semanticProjectionJson = canonicalResultSemanticExportJsonV2(projection);
  const payload: CanonicalResultExportSemanticPayloadV2 = {
    schema_version: CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION,
    format: CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT,
    source: {
      ...projection.source,
      semantic_projection_sha256: sha256HexUtf8V1(semanticProjectionJson),
    },
    title: projection.title,
    provenance: structuredClone(projection.provenance),
    ...(projection.capability_cells !== undefined
      ? { capability_cells: structuredClone(projection.capability_cells) }
      : {}),
    selection: { table_ids: tableIds, chart_ids: chartIds },
    sections: structuredClone(sections),
    tables,
    charts,
    notices: structuredClone(relevantNotices(projection, tableSet, sectionSet)),
    exclusions: structuredClone(projection.exclusions),
    footnotes: structuredClone(projection.footnotes.filter((footnote) => footnoteSet.has(footnote.id))),
    presentation: {
      precision: projection.presentation.precision,
      missing_value_label: projection.presentation.missing_value_label,
    },
  };
  if (!await checkpoint(1, true)) return null;
  return { ...payload, semantic_sha256: sha256HexUtf8V1(stableJson(payload)) };
}

async function canonicalCsvAsyncV2(
  envelope: CanonicalResultExportSemanticEnvelopeV2,
  checkpoint: ExportPreparationCheckpointV2,
): Promise<string | null> {
  const rows = [
    csvRow([CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT, String(CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION)]),
    csvRow([CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID, encodeEnvelope(envelope)]),
    csvRow(["document_id", envelope.source.document_id]),
    csvRow(["semantic_sha256", envelope.semantic_sha256]),
    csvRow(["run_id", envelope.provenance.run_id]),
    csvRow(["method_version", envelope.provenance.method_version]),
    csvRow(["dataset_fingerprint", envelope.provenance.dataset_fingerprint]),
    "",
  ];
  for (const table of envelope.tables) {
    rows.push(
      csvRow(["table_id", table.id]),
      csvRow(["table_title", table.title]),
      csvRow(["column_ids", "row_id", ...table.columns.map((column) => column.id)]),
      csvRow(["column_labels", "Row ID", ...table.columns.map((column) => column.label)]),
      csvRow(["column_types", "text", ...table.columns.map((column) => column.data_type)]),
    );
    for (const [rowIndex, row] of table.rows.entries()) {
      rows.push(csvRow(["row", row.id, ...row.cells.map(semanticCellText)]));
      if (!await checkpoint(row.cells.length + 1, rowIndex % 256 === 255)) return null;
    }
    rows.push("");
  }
  if (!await checkpoint(1, true)) return null;
  return rows.join("\r\n");
}

async function workbookTablesAsyncV2(
  envelope: CanonicalResultExportSemanticEnvelopeV2,
  checkpoint: ExportPreparationCheckpointV2,
): Promise<ResultTable[] | null> {
  const payload = encodeEnvelope(envelope);
  const payloadChunks = payload.match(/.{1,30000}/gu) ?? [""];
  const manifest: ResultTable = {
    id: XLSX_MANIFEST_TABLE_ID,
    title: XLSX_MANIFEST_TABLE_ID,
    status: "validated",
    warning: "Machine-readable cross-format manifest. Keep this worksheet with the selected canonical result worksheets.",
    columns: ["Field ID", "Value"],
    rows: [
      ["export_schema_version", String(envelope.schema_version)],
      ["export_format", envelope.format],
      ["document_id", envelope.source.document_id],
      ["semantic_projection_sha256", envelope.source.semantic_projection_sha256],
      ["semantic_sha256", envelope.semantic_sha256],
      ["selected_table_ids", envelope.selection.table_ids.join("\u001f")],
      ["selected_chart_ids", envelope.selection.chart_ids.join("\u001f")],
      ["semantic_payload_chunk_count", String(payloadChunks.length)],
      ...payloadChunks.map((chunk, index) => [`${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID}.${String(index).padStart(6, "0")}`, chunk]),
    ],
  };
  const selected: ResultTable[] = [];
  for (const table of envelope.tables) {
    const rows: string[][] = [];
    for (const [rowIndex, row] of table.rows.entries()) {
      rows.push([row.id, ...row.cells.map(semanticCellText)]);
      if (!await checkpoint(row.cells.length + 1, rowIndex % 256 === 255)) return null;
    }
    selected.push({
      id: table.id,
      title: table.id,
      status: "validated",
      warning: `Canonical table title: ${table.title}. Source document: ${envelope.source.document_id}.`,
      columns: ["Row ID", ...table.columns.map((column) => `${column.label} [${column.id}]`)],
      rows,
    });
  }
  const provenance: ResultTable = {
    id: XLSX_PROVENANCE_TABLE_ID,
    title: XLSX_PROVENANCE_TABLE_ID,
    status: "validated",
    warning: "CanonicalResultDocumentV2 provenance for the selected immutable result.",
    columns: ["Field ID", "Value"],
    rows: [
      ["document_id", envelope.source.document_id],
      ["run_id", envelope.provenance.run_id],
      ["project_id", envelope.provenance.project_id],
      ["model_id", envelope.provenance.model_id],
      ["model_digest", envelope.provenance.model_digest],
      ["dataset_id", envelope.provenance.dataset_id],
      ["dataset_fingerprint", envelope.provenance.dataset_fingerprint],
      ["recipe_id", envelope.provenance.recipe_id],
      ["recipe_digest", envelope.provenance.recipe_digest],
      ["capability_id", envelope.provenance.capability_cell.capability_id],
      ["capability_cell_id", envelope.provenance.capability_cell.cell_id],
      ["capability_version", envelope.provenance.capability_cell.capability_version],
      ["method_version", envelope.provenance.method_version],
      ["engine_version", envelope.provenance.engine_version],
      ["seed", envelope.provenance.seed === null ? "null" : String(envelope.provenance.seed)],
      ["workers", String(envelope.provenance.workers)],
      ["started_at", envelope.provenance.started_at],
      ["completed_at", envelope.provenance.completed_at],
    ],
  };
  if (!await checkpoint(payloadChunks.length + 1, true)) return null;
  return [manifest, ...selected, provenance];
}

async function canonicalPdfAsyncV2(
  envelope: CanonicalResultExportSemanticEnvelopeV2,
  checkpoint: ExportPreparationCheckpointV2,
): Promise<{ bytes?: Uint8Array; visibleTextError?: string } | null> {
  const rawLines = [
    envelope.title,
    `Document ID: ${envelope.source.document_id}`,
    `Run ID: ${envelope.provenance.run_id}`,
    `Project ID: ${envelope.provenance.project_id}`,
    `Model: ${envelope.provenance.model_id} (${envelope.provenance.model_digest})`,
    `Dataset: ${envelope.provenance.dataset_id} (${envelope.provenance.dataset_fingerprint})`,
    `Recipe: ${envelope.provenance.recipe_id} (${envelope.provenance.recipe_digest})`,
    `Method: ${envelope.provenance.method_version}`,
    `Capability cell: ${envelope.provenance.capability_cell.cell_id}`,
    `Semantic SHA-256: ${envelope.semantic_sha256}`,
    "",
  ];
  for (const table of envelope.tables) {
    rawLines.push(`[table:${table.id}] ${table.title}`);
    rawLines.push(`row_id | ${table.columns.map((column) => `${column.label} [${column.id}]`).join(" | ")}`);
    for (const [rowIndex, row] of table.rows.entries()) {
      rawLines.push(`${row.id} | ${row.cells.map(semanticCellText).join(" | ")}`);
      if (!await checkpoint(row.cells.length + 1, rowIndex % 256 === 255)) return null;
    }
    rawLines.push("");
  }
  for (const chart of envelope.charts) {
    rawLines.push(`[chart:${chart.id}] ${chart.title}`);
    for (const series of chart.series) {
      rawLines.push(`series:${series.id} ${series.label}`);
      for (const [pointIndex, point] of series.points.entries()) {
        rawLines.push(pointAccessibleText(series.label, point));
        if (!await checkpoint(1, pointIndex % 256 === 255)) return null;
      }
    }
    rawLines.push("");
  }
  for (const line of rawLines) {
    const glyph = visibleGlyph(line);
    if (glyph) return { visibleTextError: `PDF export cannot render canonical glyph ${glyphDescription(glyph)} with its bundled offline Courier font. Use HTML, XLSX, CSV, or SVG for this text; no file was written.` };
  }
  const metadata = encodeEnvelope(envelope);
  const metadataLines = metadata.match(/.{1,76}/gu) ?? [];
  const reportLines = rawLines.flatMap((line) => wrapText(line));
  const linesPerPage = 54;
  const pages = Array.from({ length: Math.max(1, Math.ceil(reportLines.length / linesPerPage)) }, (_, index) => reportLines.slice(index * linesPerPage, (index + 1) * linesPerPage));
  const objects: string[] = [];
  const pageObjectNumbers = pages.map((_, index) => 4 + index * 2);
  objects.push("<< /Type /Catalog /Pages 2 0 R >>");
  objects.push(`<< /Type /Pages /Count ${pages.length} /Kids [${pageObjectNumbers.map((number) => `${number} 0 R`).join(" ")}] >>`);
  objects.push("<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>");
  for (const [pageIndex, page] of pages.entries()) {
    const pageNumber = 4 + pageIndex * 2;
    const contentNumber = pageNumber + 1;
    objects.push(`<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 3 0 R >> >> /Contents ${contentNumber} 0 R >>`);
    const content = `BT\n/F1 8 Tf\n38 808 Td\n12 TL\n${page.map((line) => `(${pdfEscape(line)}) Tj\nT*`).join("\n")}\nET`;
    objects.push(`<< /Length ${new TextEncoder().encode(content).length} >>\nstream\n${content}\nendstream`);
    if (!await checkpoint(page.length)) return null;
  }
  let pdf = `%PDF-1.4\n%QuickPLS CanonicalResultDocumentV2\n${PDF_METADATA_BEGIN}\n${metadataLines.map((line) => `${PDF_METADATA_LINE}${line}`).join("\n")}\n${PDF_METADATA_END}\n`;
  const offsets = [0];
  for (const [index, object] of objects.entries()) {
    offsets.push(new TextEncoder().encode(pdf).length);
    pdf += `${index + 1} 0 obj\n${object}\nendobj\n`;
    if (!await checkpoint(object.length)) return null;
  }
  const xrefOffset = new TextEncoder().encode(pdf).length;
  pdf += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n`;
  for (const offset of offsets.slice(1)) pdf += `${String(offset).padStart(10, "0")} 00000 n \n`;
  pdf += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  if (!await checkpoint(pdf.length, true)) return null;
  return { bytes: new TextEncoder().encode(pdf) };
}

async function canonicalPngAsyncV2(
  chart: CanonicalResultChart,
  envelope: CanonicalResultExportSemanticEnvelopeV2,
  checkpoint: ExportPreparationCheckpointV2,
): Promise<Uint8Array | null> {
  const pixels = new Uint8Array(PNG_WIDTH * PNG_HEIGHT * 4);
  pixels.fill(255);
  const projection = chartProjection(chart, PNG_WIDTH, PNG_HEIGHT);
  drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, chart.title, 24, 10, 2, [23, 33, 38, 255], PNG_WIDTH - 48);
  if (projection) {
    for (const tick of projection.yTicks) {
      drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, projection.plot.left, tick.coordinate, projection.plot.right, tick.coordinate, [218, 225, 229, 255]);
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, tick.label, 6, tick.coordinate - 3, 1, [68, 82, 90, 255], projection.plot.left - 14);
    }
    drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, projection.plot.left, projection.plot.top, projection.plot.left, projection.plot.bottom, [38, 52, 60, 255], 2);
    drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, projection.plot.left, projection.plot.bottom, projection.plot.right, projection.plot.bottom, [38, 52, 60, 255], 2);
    for (const tick of projection.xTicks) {
      const labelWidth = bitmapTextWidth(tick.label, 1);
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, tick.label, tick.coordinate - labelWidth / 2, projection.plot.bottom + 12, 1, [68, 82, 90, 255], Math.max(42, labelWidth));
    }
    for (const [seriesIndex, series] of chart.series.entries()) {
      const color = CHART_COLORS[seriesIndex % CHART_COLORS.length]!;
      const legendX = 24 + (seriesIndex % 3) * 300;
      const legendY = 34 + Math.floor(seriesIndex / 3) * 14;
      fillRectangle(pixels, PNG_WIDTH, PNG_HEIGHT, legendX, legendY, 12, 7, color);
      drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, series.label, legendX + 18, legendY, 1, [45, 57, 64, 255], 260);
      const points = series.points.map((point) => ({ point, x: projection.x(point.x), y: projection.y(point.y) }));
      if (chart.kind === "line" || chart.kind === "interval") {
        for (let index = 1; index < points.length; index += 1) {
          const previous = points[index - 1]!;
          const current = points[index]!;
          drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, previous.x, previous.y, current.x, current.y, color, 3);
          if (!await checkpoint(1, index % 256 === 0)) return null;
        }
      }
      for (const [pointIndex, { point, x, y }] of points.entries()) {
        if (point.lower != null || point.upper != null) drawLine(pixels, PNG_WIDTH, PNG_HEIGHT, x, projection.y(point.lower ?? point.y), x, projection.y(point.upper ?? point.y), color, 2);
        if (chart.kind === "bar") {
          const zero = projection.y(0);
          const barWidth = Math.max(8, Math.min(36, (projection.plot.right - projection.plot.left) / Math.max(4, points.length * chart.series.length + 1)));
          const seriesOffset = (seriesIndex - (chart.series.length - 1) / 2) * barWidth;
          fillRectangle(pixels, PNG_WIDTH, PNG_HEIGHT, x + seriesOffset - barWidth * 0.4, Math.min(y, zero), barWidth * 0.8, Math.abs(y - zero), color);
        } else {
          drawCircle(pixels, PNG_WIDTH, PNG_HEIGHT, x, y, chart.kind === "heatmap" ? 8 : 5, color);
        }
        if (!await checkpoint(1, pointIndex % 256 === 255)) return null;
      }
    }
    if (chart.display.x_axis_label) drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, chart.display.x_axis_label, (PNG_WIDTH - bitmapTextWidth(chart.display.x_axis_label, 1)) / 2, PNG_HEIGHT - 43, 1, [38, 52, 60, 255], PNG_WIDTH - 100);
    if (chart.display.y_axis_label) drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, `Y: ${chart.display.y_axis_label}`, projection.plot.left, PNG_HEIGHT - 43, 1, [38, 52, 60, 255], 280);
  }
  if (!await checkpoint(1, true)) return null;
  drawBitmapText(pixels, PNG_WIDTH, PNG_HEIGHT, `DOC ${envelope.source.document_id} CHART ${chart.id}`, 24, PNG_HEIGHT - 18, 1, [89, 104, 112, 255], PNG_WIDTH - 48);
  const scanlines = new Uint8Array((PNG_WIDTH * 4 + 1) * PNG_HEIGHT);
  for (let row = 0; row < PNG_HEIGHT; row += 1) {
    const target = row * (PNG_WIDTH * 4 + 1);
    scanlines[target] = 0;
    scanlines.set(pixels.subarray(row * PNG_WIDTH * 4, (row + 1) * PNG_WIDTH * 4), target + 1);
  }
  if (!await checkpoint(scanlines.length, true)) return null;
  const header = new Uint8Array(13);
  header.set(uint32Bytes(PNG_WIDTH), 0);
  header.set(uint32Bytes(PNG_HEIGHT), 4);
  header.set([8, 6, 0, 0, 0], 8);
  const compressed = deflateStored(scanlines);
  if (!await checkpoint(1, true)) return null;
  return concatenateBytes([
    Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk("IHDR", header),
    pngTextChunk(PNG_SEMANTIC_KEYWORD, encodeEnvelope(envelope)),
    pngTextChunk("Document ID", envelope.source.document_id),
    pngTextChunk("Chart ID", chart.id),
    pngTextChunk("Run ID", envelope.provenance.run_id),
    pngTextChunk("Method version", envelope.provenance.method_version),
    pngTextChunk("Dataset fingerprint", envelope.provenance.dataset_fingerprint),
    pngChunk("IDAT", compressed),
    pngChunk("IEND", new Uint8Array()),
  ]);
}

async function prepareCanonicalResultExportAsyncV2(
  document: CanonicalResultDocumentV2,
  request: CanonicalResultExportRequestV2,
  signal?: AbortSignal,
): Promise<AsyncCanonicalResultExportPreparationV2> {
  if (signal?.aborted) return { ok: false, code: "cancelled", errors: [] };
  const checkpoint = exportPreparationCheckpointV2(signal);
  const built = buildCanonicalResultSemanticExportV2(document);
  if (!built.ok) return { ok: false, code: "invalid_source_document", errors: built.errors };
  const projection = built.projection;
  const exportCharts = exportChartsForProjectionV2(projection).map((entry) => entry.chart);
  const availableTables = projection.tables.map((table) => table.id);
  const availableCharts = exportCharts.map((chart) => chart.id);
  const tableSelection = orderedSelection(request.tableIds, availableTables, request.format !== "svg" && request.format !== "png", "tableIds");
  const chartSelection = orderedSelection(request.chartIds, availableCharts, request.format === "html" || request.format === "pdf", "chartIds");
  const errors = [...tableSelection.errors, ...chartSelection.errors];
  if ((request.format === "csv" || request.format === "xlsx") && tableSelection.values.length === 0) errors.push(`${request.format.toUpperCase()} export requires at least one selected canonical table.`);
  if ((request.format === "svg" || request.format === "png") && chartSelection.values.length !== 1) errors.push(`${request.format.toUpperCase()} export requires exactly one selected canonical chart.`);
  if ((request.format === "html" || request.format === "pdf") && tableSelection.values.length === 0 && chartSelection.values.length === 0) errors.push(`${request.format.toUpperCase()} export requires at least one selected table or chart.`);
  if (errors.length) return { ok: false, code: "invalid_selection", errors };
  const envelope = await buildSemanticEnvelopeAsyncV2(projection, tableSelection.values, chartSelection.values, exportCharts, checkpoint);
  if (!envelope) return { ok: false, code: "cancelled", errors: [] };
  const base = { extension: request.format, mediaType: mediaTypeFor(request.format), defaultFileName: fileNameFor(request.format, envelope), semantic: envelope } as const;
  switch (request.format) {
    case "csv": {
      const contents = await canonicalCsvAsyncV2(envelope, checkpoint);
      return contents === null ? { ok: false, code: "cancelled", errors: [] } : { ok: true, artifact: { ...base, format: "csv", contents } };
    }
    case "xlsx": {
      const workbook = await workbookTablesAsyncV2(envelope, checkpoint);
      return workbook === null ? { ok: false, code: "cancelled", errors: [] } : { ok: true, artifact: { ...base, format: "xlsx", workbookTables: workbook } };
    }
    case "html": return { ok: true, artifact: { ...base, format: "html", contents: canonicalHtml(envelope) } };
    case "pdf": {
      const result = await canonicalPdfAsyncV2(envelope, checkpoint);
      if (!result) return { ok: false, code: "cancelled", errors: [] };
      if (result.visibleTextError) return { ok: false, code: "unsupported_visible_text", errors: [result.visibleTextError] };
      return { ok: true, artifact: { ...base, format: "pdf", bytes: result.bytes! } };
    }
    case "svg": {
      const chart = envelope.charts[0];
      return chart ? { ok: true, artifact: { ...base, format: "svg", contents: canonicalChartSvg(chart, envelope) } } : { ok: false, code: "unsupported_chart", errors: ["The selected canonical chart is unavailable."] };
    }
    case "png": {
      const chart = envelope.charts[0];
      if (!chart) return { ok: false, code: "unsupported_chart", errors: ["The selected canonical chart is unavailable."] };
      const visibleTextError = pngVisibleTextError(chart, envelope);
      if (visibleTextError) return { ok: false, code: "unsupported_visible_text", errors: [visibleTextError] };
      const bytes = await canonicalPngAsyncV2(chart, envelope, checkpoint);
      return bytes === null ? { ok: false, code: "cancelled", errors: [] } : { ok: true, artifact: { ...base, format: "png", bytes } };
    }
  }
}

/**
 * Fully prepares and semantically verifies an artifact before invoking a file
 * writer. An already-aborted request and every preparation/readback failure
 * leave the writer untouched, which is the frontend no-partial-file boundary.
 */
export async function dispatchCanonicalResultExportV2(
  document: CanonicalResultDocumentV2,
  request: CanonicalResultExportRequestV2,
  writers: CanonicalResultExportWritersV2,
  signal?: AbortSignal,
): Promise<CanonicalResultExportDispatchV2> {
  if (signal?.aborted) return { status: "cancelled" };
  const prepared = await prepareCanonicalResultExportAsyncV2(document, request, signal);
  if (!prepared.ok && prepared.code === "cancelled") return { status: "cancelled" };
  if (!prepared.ok) return { status: "failed", format: request.format, message: prepared.errors[0] ?? "Export preparation failed.", errors: prepared.errors };
  if (signal?.aborted) return { status: "cancelled", artifact: prepared.artifact };
  const readback = verifyPreparedCanonicalResultExportV2(document, prepared.artifact);
  if (!readback.passed) return { status: "failed", format: request.format, message: readback.errors[0] ?? "Semantic readback failed.", errors: readback.errors };
  // Yield through a task boundary—not only a microtask—so React can paint the
  // busy state and the user can activate Cancel before any file dialog or
  // native publication begins. Once publication starts it remains an atomic,
  // no-replace operation.
  await new Promise<void>((resolve) => globalThis.setTimeout(resolve, 0));
  if (signal?.aborted) return { status: "cancelled", artifact: prepared.artifact };
  const writer = writerFor(prepared.artifact, writers);
  if (!writer) return { status: "unavailable", format: request.format, message: `No ${request.format.toUpperCase()} file writer is available in this runtime.` };
  try {
    const path = await writer(prepared.artifact as never, signal);
    return path ? { status: "saved", path, artifact: prepared.artifact } : { status: "cancelled", artifact: prepared.artifact };
  } catch (error) {
    const message = error instanceof Error && error.message ? error.message : "The export writer failed.";
    return { status: "failed", format: request.format, message, errors: [message] };
  }
}

function envelopeFromCsv(contents: string): CanonicalResultExportSemanticEnvelopeV2 | null {
  const prefix = `${spreadsheetSafeCsvCell(CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID)},`;
  const line = contents.split(/\r?\n/gu).find((candidate) => candidate.startsWith(prefix));
  if (!line) return null;
  const raw = line.slice(prefix.length);
  const value = raw.startsWith('"') && raw.endsWith('"') ? raw.slice(1, -1).replaceAll('""', '"') : raw;
  return decodeEnvelope(value);
}

function envelopeFromHtmlOrSvg(contents: string): CanonicalResultExportSemanticEnvelopeV2 | null {
  const escapedId = CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const match = contents.match(new RegExp(`<(?:script|metadata)[^>]*id=["']${escapedId}["'][^>]*>([^<]+)</(?:script|metadata)>`, "u"));
  return match?.[1] ? decodeEnvelope(match[1]) : null;
}

function envelopeFromPdf(bytes: Uint8Array): CanonicalResultExportSemanticEnvelopeV2 | null {
  const text = new TextDecoder().decode(bytes);
  const begin = text.indexOf(PDF_METADATA_BEGIN);
  const end = text.indexOf(PDF_METADATA_END, begin + PDF_METADATA_BEGIN.length);
  if (begin < 0 || end < 0) return null;
  const encoded = text.slice(begin + PDF_METADATA_BEGIN.length, end)
    .split(/\r?\n/gu)
    .filter((line) => line.startsWith(PDF_METADATA_LINE))
    .map((line) => line.slice(PDF_METADATA_LINE.length))
    .join("");
  return decodeEnvelope(encoded);
}

function readUint32(bytes: Uint8Array, offset: number): number {
  return (((bytes[offset] ?? 0) << 24) | ((bytes[offset + 1] ?? 0) << 16) | ((bytes[offset + 2] ?? 0) << 8) | (bytes[offset + 3] ?? 0)) >>> 0;
}

function envelopeFromPng(bytes: Uint8Array): CanonicalResultExportSemanticEnvelopeV2 | null {
  const signature = [137, 80, 78, 71, 13, 10, 26, 10];
  if (signature.some((value, index) => bytes[index] !== value)) return null;
  let offset = 8;
  while (offset + 12 <= bytes.length) {
    const length = readUint32(bytes, offset);
    const type = new TextDecoder().decode(bytes.slice(offset + 4, offset + 8));
    const dataStart = offset + 8;
    const dataEnd = dataStart + length;
    if (dataEnd + 4 > bytes.length) return null;
    const recordedCrc = readUint32(bytes, dataEnd);
    if (crc32(bytes.slice(offset + 4, dataEnd)) !== recordedCrc) return null;
    if (type === "tEXt") {
      const data = bytes.slice(dataStart, dataEnd);
      const separator = data.indexOf(0);
      if (separator >= 0) {
        const keyword = new TextDecoder().decode(data.slice(0, separator));
        if (keyword === PNG_SEMANTIC_KEYWORD) return decodeEnvelope(new TextDecoder().decode(data.slice(separator + 1)));
      }
    }
    offset = dataEnd + 4;
  }
  return null;
}

function envelopeFromWorkbook(tables: ResultTable[]): CanonicalResultExportSemanticEnvelopeV2 | null {
  const manifest = tables.find((table) => table.id === XLSX_MANIFEST_TABLE_ID && table.title === XLSX_MANIFEST_TABLE_ID);
  const chunks = manifest?.rows
    .filter(([field]) => field.startsWith(`${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID}.`))
    .sort(([left], [right]) => left.localeCompare(right))
    .map((row) => row[1] ?? "") ?? [];
  return chunks.length ? decodeEnvelope(chunks.join("")) : null;
}

export function readPreparedCanonicalResultExportSemanticV2(
  artifact: PreparedCanonicalResultExportV2,
): CanonicalResultExportSemanticEnvelopeV2 | null {
  switch (artifact.format) {
    case "csv": return envelopeFromCsv(artifact.contents);
    case "html":
    case "svg": return envelopeFromHtmlOrSvg(artifact.contents);
    case "xlsx": return envelopeFromWorkbook(artifact.workbookTables);
    case "pdf": return envelopeFromPdf(artifact.bytes);
    case "png": return envelopeFromPng(artifact.bytes);
  }
}

function envelopeDigestMatches(envelope: CanonicalResultExportSemanticEnvelopeV2): boolean {
  if (envelope.schema_version !== CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_SCHEMA_VERSION) return false;
  if (envelope.format !== CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_FORMAT) return false;
  if (!/^[a-f0-9]{64}$/u.test(envelope.semantic_sha256)) return false;
  const { semantic_sha256: _recorded, ...payload } = envelope;
  return sha256HexUtf8V1(stableJson(payload)) === envelope.semantic_sha256;
}

export function verifyPreparedCanonicalResultExportV2(
  document: CanonicalResultDocumentV2,
  artifact: PreparedCanonicalResultExportV2,
): CanonicalResultExportReadbackV2 {
  const readback = readPreparedCanonicalResultExportSemanticV2(artifact);
  if (!readback) return { passed: false, exact_semantic_match: false, digest_match: false, rendered_surface_match: false, errors: ["The artifact does not contain a readable QuickPLS semantic envelope."] };
  const digestMatch = envelopeDigestMatches(readback);
  const rebuilt = buildCanonicalResultSemanticExportV2(document);
  if (!rebuilt.ok) return { passed: false, exact_semantic_match: false, digest_match: digestMatch, rendered_surface_match: false, errors: rebuilt.errors, envelope: readback };
  const exportCharts = exportChartsForProjectionV2(rebuilt.projection).map((entry) => entry.chart);
  const expected = buildSemanticEnvelope(rebuilt.projection, artifact.semantic.selection.table_ids, artifact.semantic.selection.chart_ids, exportCharts);
  const exact = stableJson(readback) === stableJson(expected) && stableJson(readback) === stableJson(artifact.semantic);
  let renderedSurfaceMatch = false;
  switch (artifact.format) {
    case "csv": renderedSurfaceMatch = artifact.contents === canonicalCsv(expected); break;
    case "xlsx": renderedSurfaceMatch = stableJson(artifact.workbookTables) === stableJson(workbookTables(expected)); break;
    case "html": renderedSurfaceMatch = artifact.contents === canonicalHtml(expected); break;
    case "pdf": renderedSurfaceMatch = bytesEqual(artifact.bytes, canonicalPdf(expected)); break;
    case "svg": renderedSurfaceMatch = Boolean(expected.charts[0]) && artifact.contents === canonicalChartSvg(expected.charts[0]!, expected); break;
    case "png": renderedSurfaceMatch = Boolean(expected.charts[0]) && bytesEqual(artifact.bytes, canonicalPng(expected.charts[0]!, expected)); break;
  }
  const errors: string[] = [];
  if (!digestMatch) errors.push("The embedded semantic SHA-256 does not match the artifact payload.");
  if (!exact) errors.push("The exported semantic envelope differs from the selected canonical source projection.");
  if (!renderedSurfaceMatch) errors.push("The rendered export surface differs from its canonical semantic envelope.");
  return {
    passed: digestMatch && exact && renderedSurfaceMatch,
    exact_semantic_match: exact,
    digest_match: digestMatch,
    rendered_surface_match: renderedSurfaceMatch,
    errors,
    envelope: readback,
  };
}

function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) return false;
  return left.every((value, index) => value === right[index]);
}

