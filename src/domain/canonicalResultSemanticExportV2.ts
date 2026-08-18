import {
  CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
  canonicalAnalyticalResultJson,
  canonicalResultDocumentJson,
  type CanonicalChartDisplayOptions,
  type CanonicalResultCell,
  type CanonicalResultDocumentV2,
  type CapabilityCellReferenceV2,
  validateCanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";

/**
 * Format-neutral export contract. CSV, workbook, HTML, and other writers can
 * consume this projection without reaching back into method-specific result
 * payloads.
 */
export const CANONICAL_RESULT_SEMANTIC_EXPORT_V2_SCHEMA_VERSION = 2 as const;
export const CANONICAL_RESULT_SEMANTIC_EXPORT_V2_FORMAT = "quickpls.canonical-result-semantic-export" as const;

export interface CanonicalSemanticExportOrderingV2 {
  sections: Array<{
    section_id: string;
    table_ids: string[];
    chart_ids: string[];
  }>;
  tables: Array<{
    table_id: string;
    column_ids: string[];
    row_ids: string[];
  }>;
  charts: Array<{
    chart_id: string;
    series: Array<{
      series_id: string;
      point_count: number;
    }>;
  }>;
  notice_ids: string[];
  exclusion_ids: string[];
  footnote_ids: string[];
}

export interface CanonicalResultSemanticExportV2 {
  export_schema_version: typeof CANONICAL_RESULT_SEMANTIC_EXPORT_V2_SCHEMA_VERSION;
  format: typeof CANONICAL_RESULT_SEMANTIC_EXPORT_V2_FORMAT;
  source: {
    document_schema_version: typeof CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
    document_id: string;
  };
  title: string;
  provenance: CanonicalResultDocumentV2["provenance"];
  capability_cells?: CapabilityCellReferenceV2[];
  ordering: CanonicalSemanticExportOrderingV2;
  sections: CanonicalResultDocumentV2["sections"];
  tables: CanonicalResultDocumentV2["tables"];
  charts: CanonicalResultDocumentV2["charts"];
  notices: CanonicalResultDocumentV2["notices"];
  exclusions: CanonicalResultDocumentV2["exclusions"];
  footnotes: CanonicalResultDocumentV2["footnotes"];
  presentation: CanonicalResultDocumentV2["presentation"];
}

export type CanonicalResultSemanticExportBuildV2 =
  | { ok: true; projection: CanonicalResultSemanticExportV2 }
  | { ok: false; code: "invalid_source_document"; errors: string[] };

export type CanonicalResultSemanticExportParseV2 =
  | {
      ok: true;
      projection: CanonicalResultSemanticExportV2;
      document: CanonicalResultDocumentV2;
    }
  | {
      ok: false;
      code: "invalid_json" | "invalid_export";
      errors: string[];
    };

export interface CanonicalResultSemanticExportReadbackV2 {
  passed: boolean;
  exact_document_match: boolean;
  analytical_match: boolean;
  errors: string[];
  projection?: CanonicalResultSemanticExportV2;
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return Object.fromEntries(Object.keys(record).sort().map((key) => [key, stableValue(record[key])]));
  }
  return value;
}

function copyCapability(reference: CapabilityCellReferenceV2): CapabilityCellReferenceV2 {
  return {
    registry_schema_version: reference.registry_schema_version,
    capability_id: reference.capability_id,
    cell_id: reference.cell_id,
    capability_version: reference.capability_version,
  };
}

function copyDisplay(display: CanonicalChartDisplayOptions): CanonicalChartDisplayOptions {
  return {
    ...(display.palette !== undefined ? { palette: display.palette } : {}),
    ...(display.show_legend !== undefined ? { show_legend: display.show_legend } : {}),
    ...(display.show_values !== undefined ? { show_values: display.show_values } : {}),
    ...(display.x_axis_label !== undefined ? { x_axis_label: display.x_axis_label } : {}),
    ...(display.y_axis_label !== undefined ? { y_axis_label: display.y_axis_label } : {}),
  };
}

function copyCell(cell: CanonicalResultCell): CanonicalResultCell {
  switch (cell.kind) {
    case "number":
      return {
        kind: "number",
        value: cell.value,
        ...(cell.display !== undefined ? { display: cell.display } : {}),
      };
    case "text":
      return { kind: "text", value: cell.value };
    case "boolean":
      return { kind: "boolean", value: cell.value };
    case "missing":
      return {
        kind: "missing",
        reason: cell.reason,
        ...(cell.display !== undefined ? { display: cell.display } : {}),
      };
  }
}

function copySections(document: CanonicalResultDocumentV2): CanonicalResultDocumentV2["sections"] {
  return document.sections.map((section) => ({
    id: section.id,
    title: section.title,
    ...(section.description !== undefined ? { description: section.description } : {}),
    table_ids: [...section.table_ids],
    chart_ids: [...section.chart_ids],
    ...(section.capability_cells !== undefined
      ? { capability_cells: section.capability_cells.map(copyCapability) }
      : {}),
  }));
}

function copyTables(document: CanonicalResultDocumentV2): CanonicalResultDocumentV2["tables"] {
  return document.tables.map((table) => ({
    id: table.id,
    title: table.title,
    ...(table.description !== undefined ? { description: table.description } : {}),
    columns: table.columns.map((column) => ({
      id: column.id,
      label: column.label,
      data_type: column.data_type,
      description: column.description,
      ...(column.role !== undefined ? { role: column.role } : {}),
      ...(column.unit !== undefined ? { unit: column.unit } : {}),
      ...(column.default_precision !== undefined ? { default_precision: column.default_precision } : {}),
    })),
    rows: table.rows.map((row) => ({ id: row.id, cells: row.cells.map(copyCell) })),
    footnote_ids: [...table.footnote_ids],
    ...(table.capability_cells !== undefined
      ? { capability_cells: table.capability_cells.map(copyCapability) }
      : {}),
  }));
}

function copyCharts(document: CanonicalResultDocumentV2): CanonicalResultDocumentV2["charts"] {
  return document.charts.map((chart) => ({
    id: chart.id,
    title: chart.title,
    description: chart.description,
    kind: chart.kind,
    series: chart.series.map((series) => ({
      id: series.id,
      label: series.label,
      ...(series.group !== undefined ? { group: series.group } : {}),
      points: series.points.map((point) => ({
        x: point.x,
        y: point.y,
        ...(point.lower !== undefined ? { lower: point.lower } : {}),
        ...(point.upper !== undefined ? { upper: point.upper } : {}),
        ...(point.label !== undefined ? { label: point.label } : {}),
      })),
    })),
    ...(chart.source_table_id !== undefined ? { source_table_id: chart.source_table_id } : {}),
    display: copyDisplay(chart.display),
  }));
}

function copyDocumentCollections(document: CanonicalResultDocumentV2) {
  return {
    sections: copySections(document),
    tables: copyTables(document),
    charts: copyCharts(document),
    notices: document.notices.map((notice) => ({
      id: notice.id,
      code: notice.code,
      severity: notice.severity,
      message: notice.message,
      section_ids: [...notice.section_ids],
      table_ids: [...notice.table_ids],
    })),
    exclusions: document.exclusions.map((exclusion) => ({
      id: exclusion.id,
      ...(exclusion.capability_cell !== undefined
        ? { capability_cell: exclusion.capability_cell === null ? null : copyCapability(exclusion.capability_cell) }
        : {}),
      title: exclusion.title,
      reason: exclusion.reason,
    })),
    footnotes: document.footnotes.map((footnote) => ({
      id: footnote.id,
      text: footnote.text,
      ...(footnote.reference !== undefined ? { reference: footnote.reference } : {}),
    })),
  };
}

function orderingFor(document: CanonicalResultDocumentV2): CanonicalSemanticExportOrderingV2 {
  return {
    sections: document.sections.map((section) => ({
      section_id: section.id,
      table_ids: [...section.table_ids],
      chart_ids: [...section.chart_ids],
    })),
    tables: document.tables.map((table) => ({
      table_id: table.id,
      column_ids: table.columns.map((column) => column.id),
      row_ids: table.rows.map((row) => row.id),
    })),
    charts: document.charts.map((chart) => ({
      chart_id: chart.id,
      series: chart.series.map((series) => ({
        series_id: series.id,
        point_count: series.points.length,
      })),
    })),
    notice_ids: document.notices.map((notice) => notice.id),
    exclusion_ids: document.exclusions.map((exclusion) => exclusion.id),
    footnote_ids: document.footnotes.map((footnote) => footnote.id),
  };
}

function projectValidatedDocument(document: CanonicalResultDocumentV2): CanonicalResultSemanticExportV2 {
  const collections = copyDocumentCollections(document);
  return {
    export_schema_version: CANONICAL_RESULT_SEMANTIC_EXPORT_V2_SCHEMA_VERSION,
    format: CANONICAL_RESULT_SEMANTIC_EXPORT_V2_FORMAT,
    source: {
      document_schema_version: document.schema_version,
      document_id: document.document_id,
    },
    title: document.title,
    provenance: {
      run_id: document.provenance.run_id,
      project_id: document.provenance.project_id,
      model_id: document.provenance.model_id,
      model_digest: document.provenance.model_digest,
      dataset_id: document.provenance.dataset_id,
      dataset_fingerprint: document.provenance.dataset_fingerprint,
      recipe_id: document.provenance.recipe_id,
      recipe_digest: document.provenance.recipe_digest,
      capability_cell: copyCapability(document.provenance.capability_cell),
      method_version: document.provenance.method_version,
      engine_version: document.provenance.engine_version,
      seed: document.provenance.seed,
      workers: document.provenance.workers,
      started_at: document.provenance.started_at,
      completed_at: document.provenance.completed_at,
    },
    ...(document.capability_cells !== undefined
      ? { capability_cells: document.capability_cells.map(copyCapability) }
      : {}),
    ordering: orderingFor(document),
    ...collections,
    presentation: {
      default_section_id: document.presentation.default_section_id,
      default_table_id: document.presentation.default_table_id,
      precision: document.presentation.precision,
      missing_value_label: document.presentation.missing_value_label,
      chart_defaults: copyDisplay(document.presentation.chart_defaults),
    },
  };
}

function documentFromProjection(projection: CanonicalResultSemanticExportV2): CanonicalResultDocumentV2 {
  const projectedDocument: CanonicalResultDocumentV2 = {
    schema_version: projection.source.document_schema_version,
    document_id: projection.source.document_id,
    title: projection.title,
    provenance: projection.provenance,
    ...(projection.capability_cells !== undefined ? { capability_cells: projection.capability_cells } : {}),
    sections: projection.sections,
    tables: projection.tables,
    charts: projection.charts,
    notices: projection.notices,
    exclusions: projection.exclusions,
    footnotes: projection.footnotes,
    presentation: projection.presentation,
  };
  const collections = copyDocumentCollections(projectedDocument);
  return {
    ...projectedDocument,
    provenance: {
      ...projectedDocument.provenance,
      capability_cell: copyCapability(projectedDocument.provenance.capability_cell),
    },
    ...(projectedDocument.capability_cells !== undefined
      ? { capability_cells: projectedDocument.capability_cells.map(copyCapability) }
      : {}),
    ...collections,
    presentation: {
      ...projectedDocument.presentation,
      chart_defaults: copyDisplay(projectedDocument.presentation.chart_defaults),
    },
  };
}

function safeSourceValidation(document: CanonicalResultDocumentV2): string[] {
  try {
    return validateCanonicalResultDocumentV2(document).errors;
  } catch {
    return ["The source is not a structurally valid CanonicalResultDocumentV2."];
  }
}

export function buildCanonicalResultSemanticExportV2(
  document: CanonicalResultDocumentV2,
): CanonicalResultSemanticExportBuildV2 {
  const errors = safeSourceValidation(document);
  if (errors.length > 0) return { ok: false, code: "invalid_source_document", errors };
  return { ok: true, projection: projectValidatedDocument(document) };
}

export function canonicalResultSemanticExportJsonV2(
  projection: CanonicalResultSemanticExportV2,
): string {
  return JSON.stringify(stableValue(projection));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

/**
 * Parse, validate, and normalize a semantic export. The deterministic rebuild
 * comparison rejects missing, unexpected, or reordered contract fields.
 */
export function parseCanonicalResultSemanticExportJsonV2(
  json: string,
): CanonicalResultSemanticExportParseV2 {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch {
    return { ok: false, code: "invalid_json", errors: ["The export is not valid JSON."] };
  }
  if (!isRecord(parsed)) {
    return { ok: false, code: "invalid_export", errors: ["The export root must be an object."] };
  }
  if (parsed.export_schema_version !== CANONICAL_RESULT_SEMANTIC_EXPORT_V2_SCHEMA_VERSION) {
    return {
      ok: false,
      code: "invalid_export",
      errors: [`export_schema_version must equal ${CANONICAL_RESULT_SEMANTIC_EXPORT_V2_SCHEMA_VERSION}.`],
    };
  }
  if (parsed.format !== CANONICAL_RESULT_SEMANTIC_EXPORT_V2_FORMAT) {
    return { ok: false, code: "invalid_export", errors: ["The export format identifier is not supported."] };
  }

  try {
    const candidate = parsed as unknown as CanonicalResultSemanticExportV2;
    const document = documentFromProjection(candidate);
    const documentErrors = safeSourceValidation(document);
    if (documentErrors.length > 0) {
      return { ok: false, code: "invalid_export", errors: documentErrors };
    }
    const normalized = projectValidatedDocument(document);
    if (canonicalResultSemanticExportJsonV2(candidate) !== canonicalResultSemanticExportJsonV2(normalized)) {
      return {
        ok: false,
        code: "invalid_export",
        errors: ["The export fields or ordering do not match the canonical semantic projection."],
      };
    }
    return { ok: true, projection: normalized, document };
  } catch {
    return {
      ok: false,
      code: "invalid_export",
      errors: ["The export does not contain a complete canonical result payload."],
    };
  }
}

/** Verify both exact-document and scientific-payload readback equivalence. */
export function verifyCanonicalResultSemanticExportReadbackV2(
  source: CanonicalResultDocumentV2,
  json: string,
): CanonicalResultSemanticExportReadbackV2 {
  const sourceErrors = safeSourceValidation(source);
  if (sourceErrors.length > 0) {
    return {
      passed: false,
      exact_document_match: false,
      analytical_match: false,
      errors: sourceErrors.map((error) => `Source: ${error}`),
    };
  }
  const parsed = parseCanonicalResultSemanticExportJsonV2(json);
  if (!parsed.ok) {
    return {
      passed: false,
      exact_document_match: false,
      analytical_match: false,
      errors: parsed.errors,
    };
  }
  const exactDocumentMatch = canonicalResultDocumentJson(source) === canonicalResultDocumentJson(parsed.document);
  const analyticalMatch = canonicalAnalyticalResultJson(source) === canonicalAnalyticalResultJson(parsed.document);
  const errors: string[] = [];
  if (!exactDocumentMatch) errors.push("The readback document differs from the source document.");
  if (!analyticalMatch) errors.push("The readback analytical payload differs from the source analytical payload.");
  return {
    passed: exactDocumentMatch && analyticalMatch,
    exact_document_match: exactDocumentMatch,
    analytical_match: analyticalMatch,
    errors,
    projection: parsed.projection,
  };
}
