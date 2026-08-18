import {
  CanonicalGeneralSemResultsV1ParseError,
  parseCanonicalGeneralSemResultsV1,
  type CanonicalGeneralSemResultsV1,
} from "./canonicalGeneralSemResultsV1";

/**
 * Canonical, method-neutral result document used by UI, comparison, archive,
 * accessibility, and export adapters.
 *
 * Arrays are ordered analytical/display contracts. Object keys are
 * canonicalized before hashing or byte comparison. Presentation settings are
 * deliberately removable from the analytical projection.
 */

export const CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION = 2 as const;

export interface CapabilityCellReferenceV2 {
  registry_schema_version: 2;
  capability_id: string;
  cell_id: string;
  capability_version: string;
}

export function capabilityCellReferenceIdentityV2(reference: CapabilityCellReferenceV2): string {
  return `${reference.registry_schema_version}:${reference.capability_id}:${reference.cell_id}:${reference.capability_version}`;
}

export type CanonicalCellKind = "number" | "text" | "boolean" | "missing";
export type CanonicalColumnType = Exclude<CanonicalCellKind, "missing">;

export interface CanonicalNumberCell {
  kind: "number";
  value: number;
  /** Optional display cache. It is excluded from the analytical projection. */
  display?: string;
}

export interface CanonicalTextCell {
  kind: "text";
  value: string;
}

export interface CanonicalBooleanCell {
  kind: "boolean";
  value: boolean;
}

export interface CanonicalMissingCell {
  kind: "missing";
  reason: "not_applicable" | "not_estimated" | "undefined" | "withheld";
  display?: string;
}

export type CanonicalResultCell =
  | CanonicalNumberCell
  | CanonicalTextCell
  | CanonicalBooleanCell
  | CanonicalMissingCell;

export interface CanonicalResultColumn {
  id: string;
  label: string;
  data_type: CanonicalColumnType;
  description: string;
  role?: "label" | "estimate" | "uncertainty" | "decision" | "diagnostic" | "provenance";
  unit?: string | null;
  default_precision?: number | null;
}

export interface CanonicalResultRow {
  id: string;
  cells: CanonicalResultCell[];
}

export interface CanonicalResultTable {
  id: string;
  title: string;
  description?: string | null;
  columns: CanonicalResultColumn[];
  rows: CanonicalResultRow[];
  footnote_ids: string[];
  /** Explicit option cells that produced this table. Required for new comparable documents. */
  capability_cells?: CapabilityCellReferenceV2[];
}

export interface CanonicalChartPoint {
  x: number | string;
  y: number;
  lower?: number | null;
  upper?: number | null;
  label?: string | null;
}

export interface CanonicalChartSeries {
  id: string;
  label: string;
  group?: string | null;
  points: CanonicalChartPoint[];
}

export interface CanonicalChartDisplayOptions {
  palette?: string;
  show_legend?: boolean;
  show_values?: boolean;
  x_axis_label?: string | null;
  y_axis_label?: string | null;
}

export interface CanonicalResultChart {
  id: string;
  title: string;
  description: string;
  kind: "line" | "bar" | "scatter" | "interval" | "heatmap";
  series: CanonicalChartSeries[];
  source_table_id?: string | null;
  display: CanonicalChartDisplayOptions;
}

export interface CanonicalResultSection {
  id: string;
  title: string;
  description?: string | null;
  table_ids: string[];
  chart_ids: string[];
  /** Explicit union of the option cells represented by this section. */
  capability_cells?: CapabilityCellReferenceV2[];
}

export interface CanonicalResultNotice {
  id: string;
  code: string;
  severity: "information" | "warning" | "error";
  message: string;
  section_ids: string[];
  table_ids: string[];
}

export interface CanonicalResultExclusion {
  id: string;
  capability_cell?: CapabilityCellReferenceV2 | null;
  title: string;
  reason: string;
}

export interface CanonicalResultFootnote {
  id: string;
  text: string;
  reference?: string | null;
}

export interface CanonicalResultProvenanceV2 {
  run_id: string;
  project_id: string;
  model_id: string;
  model_digest: string;
  dataset_id: string;
  dataset_fingerprint: string;
  recipe_id: string;
  recipe_digest: string;
  capability_cell: CapabilityCellReferenceV2;
  method_version: string;
  engine_version: string;
  seed: number | null;
  workers: number;
  started_at: string;
  completed_at: string;
}

export interface CanonicalResultPresentationV2 {
  default_section_id: string | null;
  default_table_id: string | null;
  precision: number;
  missing_value_label: string;
  chart_defaults: CanonicalChartDisplayOptions;
}

export interface CanonicalResultDocumentV2 {
  schema_version: typeof CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION;
  document_id: string;
  title: string;
  provenance: CanonicalResultProvenanceV2;
  /** Sorted, distinct option-cell set; provenance.capability_cell remains primary. */
  capability_cells?: CapabilityCellReferenceV2[];
  /** Additive typed General SEM result extension; legacy documents omit it. */
  general_sem_results?: CanonicalGeneralSemResultsV1;
  sections: CanonicalResultSection[];
  tables: CanonicalResultTable[];
  charts: CanonicalResultChart[];
  notices: CanonicalResultNotice[];
  exclusions: CanonicalResultExclusion[];
  footnotes: CanonicalResultFootnote[];
  presentation: CanonicalResultPresentationV2;
}

export interface CanonicalResultValidation {
  passed: boolean;
  errors: string[];
}

const STABLE_ID = /^[a-z0-9][a-z0-9_.:-]*$/;
const HEX_64 = /^[a-f0-9]{64}$/;
const DATASET_FINGERPRINT_V1 = /^(?:v2:)?[a-f0-9]{64}$/;

function duplicateIds(items: ReadonlyArray<{ id: string }>): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const item of items) {
    if (seen.has(item.id)) duplicates.add(item.id);
    seen.add(item.id);
  }
  return [...duplicates].sort();
}

function requireStableId(errors: string[], value: string, context: string) {
  if (!STABLE_ID.test(value)) errors.push(`${context} must be a stable lowercase identifier`);
}

function requireUniqueIds(errors: string[], items: ReadonlyArray<{ id: string }>, context: string) {
  const duplicates = duplicateIds(items);
  if (duplicates.length > 0) errors.push(`${context} contains duplicate IDs: ${duplicates.join(", ")}`);
  for (const item of items) requireStableId(errors, item.id, `${context} ID ${JSON.stringify(item.id)}`);
}

function validateCapabilityReference(errors: string[], reference: CapabilityCellReferenceV2, context: string) {
  if (reference.registry_schema_version !== 2) errors.push(`${context}.registry_schema_version must equal 2`);
  requireStableId(errors, reference.capability_id, `${context}.capability_id`);
  requireStableId(errors, reference.cell_id, `${context}.cell_id`);
  requireStableId(errors, reference.capability_version, `${context}.capability_version`);
}

function validateCapabilitySet(
  errors: string[],
  references: readonly CapabilityCellReferenceV2[],
  context: string,
): string[] {
  if (references.length === 0) errors.push(`${context} must not be empty`);
  const identities = references.map((reference, index) => {
    validateCapabilityReference(errors, reference, `${context}[${index}]`);
    return capabilityCellReferenceIdentityV2(reference);
  });
  const duplicates = duplicateIds(identities.map((id) => ({ id })));
  if (duplicates.length > 0) errors.push(`${context} contains duplicate references: ${duplicates.join(", ")}`);
  const sorted = [...identities].sort();
  if (!identities.every((identity, index) => identity === sorted[index])) errors.push(`${context} must be ordered by exact option-cell identity`);
  return identities;
}

function validTimestamp(value: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(value)) return null;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

export function validateCanonicalResultDocumentV2(document: CanonicalResultDocumentV2): CanonicalResultValidation {
  const errors: string[] = [];
  if (document.schema_version !== CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION) {
    errors.push(`schema_version must equal ${CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION}`);
  }
  requireStableId(errors, document.document_id, "document_id");
  if (!document.title.trim()) errors.push("title must be nonempty");

  requireUniqueIds(errors, document.sections, "sections");
  requireUniqueIds(errors, document.tables, "tables");
  requireUniqueIds(errors, document.charts, "charts");
  requireUniqueIds(errors, document.notices, "notices");
  requireUniqueIds(errors, document.exclusions, "exclusions");
  requireUniqueIds(errors, document.footnotes, "footnotes");

  const sectionIds = new Set(document.sections.map((section) => section.id));
  const tableIds = new Set(document.tables.map((table) => table.id));
  const chartIds = new Set(document.charts.map((chart) => chart.id));
  const footnoteIds = new Set(document.footnotes.map((footnote) => footnote.id));
  let documentCapabilityIds: Set<string> | null = null;
  if (document.capability_cells) {
    const identities = validateCapabilitySet(errors, document.capability_cells, "capability_cells");
    documentCapabilityIds = new Set(identities);
    const primaryIdentity = capabilityCellReferenceIdentityV2(document.provenance.capability_cell);
    if (!documentCapabilityIds.has(primaryIdentity)) errors.push("capability_cells must include provenance.capability_cell");
  }

  for (const section of document.sections) {
    if (!section.title.trim()) errors.push(`section ${section.id} title must be nonempty`);
    for (const tableId of section.table_ids) {
      if (!tableIds.has(tableId)) errors.push(`section ${section.id} references missing table ${tableId}`);
    }
    for (const chartId of section.chart_ids) {
      if (!chartIds.has(chartId)) errors.push(`section ${section.id} references missing chart ${chartId}`);
    }
    if (documentCapabilityIds) {
      if (!section.capability_cells) {
        errors.push(`section ${section.id} must declare capability_cells`);
      } else {
        const identities = validateCapabilitySet(errors, section.capability_cells, `section ${section.id}.capability_cells`);
        for (const identity of identities) {
          if (!documentCapabilityIds.has(identity)) errors.push(`section ${section.id} references an undeclared option cell ${identity}`);
        }
      }
    } else if (section.capability_cells) {
      errors.push(`section ${section.id} cannot declare capability_cells without a document capability_cells set`);
    }
  }

  for (const table of document.tables) {
    if (!table.title.trim()) errors.push(`table ${table.id} title must be nonempty`);
    requireUniqueIds(errors, table.columns, `table ${table.id} columns`);
    requireUniqueIds(errors, table.rows, `table ${table.id} rows`);
    for (const column of table.columns) {
      if (!column.label.trim()) errors.push(`table ${table.id} column ${column.id} label must be nonempty`);
      if (!column.description.trim()) errors.push(`table ${table.id} column ${column.id} description must be nonempty`);
      if (column.default_precision != null && (!Number.isInteger(column.default_precision) || column.default_precision < 0 || column.default_precision > 12)) {
        errors.push(`table ${table.id} column ${column.id} default_precision must be an integer from 0 to 12`);
      }
    }
    for (const row of table.rows) {
      if (row.cells.length !== table.columns.length) {
        errors.push(`table ${table.id} row ${row.id} has ${row.cells.length} cells; expected ${table.columns.length}`);
        continue;
      }
      row.cells.forEach((cell, index) => {
        const column = table.columns[index];
        if (cell.kind !== "missing" && cell.kind !== column.data_type) {
          errors.push(`table ${table.id} row ${row.id} cell ${column.id} is ${cell.kind}; expected ${column.data_type} or missing`);
        }
        if (cell.kind === "number" && !Number.isFinite(cell.value)) {
          errors.push(`table ${table.id} row ${row.id} cell ${column.id} must be finite`);
        }
      });
    }
    for (const footnoteId of table.footnote_ids) {
      if (!footnoteIds.has(footnoteId)) errors.push(`table ${table.id} references missing footnote ${footnoteId}`);
    }
    if (documentCapabilityIds) {
      if (!table.capability_cells) {
        errors.push(`table ${table.id} must declare capability_cells`);
      } else {
        const identities = validateCapabilitySet(errors, table.capability_cells, `table ${table.id}.capability_cells`);
        for (const identity of identities) {
          if (!documentCapabilityIds.has(identity)) errors.push(`table ${table.id} references an undeclared option cell ${identity}`);
        }
      }
    } else if (table.capability_cells) {
      errors.push(`table ${table.id} cannot declare capability_cells without a document capability_cells set`);
    }
  }

  if (documentCapabilityIds) {
    const tableById = new Map(document.tables.map((table) => [table.id, table]));
    for (const section of document.sections) {
      if (!section.capability_cells) continue;
      const sectionCapabilities = new Set(section.capability_cells.map(capabilityCellReferenceIdentityV2));
      const requiredByTables = new Set(section.table_ids.flatMap((tableId) => (
        tableById.get(tableId)?.capability_cells?.map(capabilityCellReferenceIdentityV2) ?? []
      )));
      for (const identity of requiredByTables) {
        if (!sectionCapabilities.has(identity)) errors.push(`section ${section.id} is missing table option cell ${identity}`);
      }
    }
  }

  for (const chart of document.charts) {
    if (!chart.title.trim() || !chart.description.trim()) errors.push(`chart ${chart.id} needs a title and accessible description`);
    if (chart.source_table_id != null && !tableIds.has(chart.source_table_id)) {
      errors.push(`chart ${chart.id} references missing table ${chart.source_table_id}`);
    }
    requireUniqueIds(errors, chart.series, `chart ${chart.id} series`);
    for (const series of chart.series) {
      for (const [pointIndex, point] of series.points.entries()) {
        if (typeof point.x === "number" && !Number.isFinite(point.x)) {
          errors.push(`chart ${chart.id} series ${series.id} point ${pointIndex} x must be finite`);
        }
        for (const [name, value] of [["y", point.y], ["lower", point.lower], ["upper", point.upper]] as const) {
          if (value != null && !Number.isFinite(value)) errors.push(`chart ${chart.id} series ${series.id} point ${pointIndex} ${name} must be finite`);
        }
        if (point.lower != null && point.upper != null && point.lower > point.upper) {
          errors.push(`chart ${chart.id} series ${series.id} point ${pointIndex} lower exceeds upper`);
        }
      }
    }
  }

  for (const notice of document.notices) {
    if (!notice.code.trim() || !notice.message.trim()) errors.push(`notice ${notice.id} code and message must be nonempty`);
    for (const sectionId of notice.section_ids) if (!sectionIds.has(sectionId)) errors.push(`notice ${notice.id} references missing section ${sectionId}`);
    for (const tableId of notice.table_ids) if (!tableIds.has(tableId)) errors.push(`notice ${notice.id} references missing table ${tableId}`);
  }

  for (const exclusion of document.exclusions) {
    if (!exclusion.title.trim() || !exclusion.reason.trim()) errors.push(`exclusion ${exclusion.id} title and reason must be nonempty`);
    if (exclusion.capability_cell) validateCapabilityReference(errors, exclusion.capability_cell, `exclusion ${exclusion.id}.capability_cell`);
  }

  const provenance = document.provenance;
  for (const [name, value] of Object.entries({
    run_id: provenance.run_id,
    project_id: provenance.project_id,
    model_id: provenance.model_id,
    dataset_id: provenance.dataset_id,
    recipe_id: provenance.recipe_id,
    method_version: provenance.method_version,
    engine_version: provenance.engine_version,
  })) {
    if (!value.trim()) errors.push(`provenance.${name} must be nonempty`);
  }
  if (!HEX_64.test(provenance.model_digest)) errors.push("provenance.model_digest must be lowercase SHA-256");
  if (!DATASET_FINGERPRINT_V1.test(provenance.dataset_fingerprint)) {
    errors.push("provenance.dataset_fingerprint must be bare lowercase SHA-256 or v2:<lowercase SHA-256>");
  }
  if (!HEX_64.test(provenance.recipe_digest)) errors.push("provenance.recipe_digest must be lowercase SHA-256");
  if (provenance.seed != null && (!Number.isSafeInteger(provenance.seed) || provenance.seed < 0)) errors.push("provenance.seed must be a nonnegative safe integer or null");
  if (!Number.isInteger(provenance.workers) || provenance.workers < 1) errors.push("provenance.workers must be a positive integer");
  validateCapabilityReference(errors, provenance.capability_cell, "provenance.capability_cell");
  const startedAt = validTimestamp(provenance.started_at);
  const completedAt = validTimestamp(provenance.completed_at);
  if (startedAt == null) errors.push("provenance.started_at must be an ISO timestamp");
  if (completedAt == null) errors.push("provenance.completed_at must be an ISO timestamp");
  if (startedAt != null && completedAt != null && completedAt < startedAt) errors.push("provenance.completed_at precedes started_at");

  if (document.general_sem_results !== undefined) {
    try {
      parseCanonicalGeneralSemResultsV1(document.general_sem_results, {
        modelId: provenance.model_id,
        modelDigest: provenance.model_digest,
        datasetFingerprint: provenance.dataset_fingerprint,
        recipeDigest: provenance.recipe_digest,
        seed: provenance.seed,
        workers: provenance.workers,
        capabilityCells: document.capability_cells ?? [],
      });
    } catch (error) {
      if (error instanceof CanonicalGeneralSemResultsV1ParseError) {
        errors.push(`${error.path}: ${error.message}`);
      } else {
        throw error;
      }
    }
  }

  const presentation = document.presentation;
  if (presentation.default_section_id != null && !sectionIds.has(presentation.default_section_id)) errors.push("presentation.default_section_id is missing");
  if (presentation.default_table_id != null && !tableIds.has(presentation.default_table_id)) errors.push("presentation.default_table_id is missing");
  if (!Number.isInteger(presentation.precision) || presentation.precision < 0 || presentation.precision > 12) errors.push("presentation.precision must be an integer from 0 to 12");
  if (!presentation.missing_value_label.trim()) errors.push("presentation.missing_value_label must be nonempty");

  return { passed: errors.length === 0, errors };
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    const object = value as Record<string, unknown>;
    return Object.fromEntries(Object.keys(object).sort().map((key) => [key, stableValue(object[key])]));
  }
  return value;
}

export function canonicalResultDocumentJson(document: CanonicalResultDocumentV2): string {
  return JSON.stringify(stableValue(document));
}

function analyticalGeneralSemResults(document: CanonicalResultDocumentV2): unknown | undefined {
  const generalSemResults = (document as CanonicalResultDocumentV2 & {
    general_sem_results?: unknown;
  }).general_sem_results;
  if (
    generalSemResults == null
    || typeof generalSemResults !== "object"
    || Array.isArray(generalSemResults)
  ) {
    return generalSemResults;
  }

  const results = generalSemResults as unknown as Record<string, unknown>;
  const inferenceReceipt = results.inference_receipt;
  if (
    inferenceReceipt == null
    || typeof inferenceReceipt !== "object"
    || Array.isArray(inferenceReceipt)
  ) {
    return generalSemResults;
  }

  const { workers: _workers, ...analyticalReceipt } = inferenceReceipt as Record<string, unknown>;
  return {
    ...results,
    inference_receipt: analyticalReceipt,
  };
}

/**
 * Return the scientific projection used for semantic equality. Execution-only
 * timing/workers and every display cache/default are excluded; model, data,
 * recipe, capability, engine, ordered tables, chart data, notices, and
 * exclusions remain bound.
 */
export function canonicalAnalyticalResultJson(document: CanonicalResultDocumentV2): string {
  const generalSemResults = analyticalGeneralSemResults(document);
  const analytical = {
    schema_version: document.schema_version,
    document_id: document.document_id,
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
      capability_cell: document.provenance.capability_cell,
      method_version: document.provenance.method_version,
      engine_version: document.provenance.engine_version,
      seed: document.provenance.seed,
    },
    ...(document.capability_cells ? { capability_cells: document.capability_cells } : {}),
    sections: document.sections,
    tables: document.tables.map((table) => ({
      ...table,
      rows: table.rows.map((row) => ({
        ...row,
        cells: row.cells.map((cell) => {
          if (cell.kind === "number") return { kind: cell.kind, value: cell.value };
          if (cell.kind === "missing") return { kind: cell.kind, reason: cell.reason };
          return cell;
        }),
      })),
    })),
    charts: document.charts.map(({ display: _display, ...chart }) => chart),
    notices: document.notices,
    exclusions: document.exclusions,
    footnotes: document.footnotes,
    ...(generalSemResults === undefined ? {} : { general_sem_results: generalSemResults }),
  };
  return JSON.stringify(stableValue(analytical));
}

export interface LegacyStringResultTable {
  id: string;
  title: string;
  columns: string[];
  rows: string[][];
  warning?: string | null;
}

export interface LegacyResultMigrationContext {
  document_id: string;
  title: string;
  provenance: CanonicalResultProvenanceV2;
}

/**
 * Lossless text migration for historical string-only tables. It never infers
 * numeric meaning from formatted strings; future method adapters must create
 * typed cells directly from analytical payloads.
 */
export function canonicalResultDocumentFromLegacyTables(
  context: LegacyResultMigrationContext,
  legacyTables: LegacyStringResultTable[],
): CanonicalResultDocumentV2 {
  const tables: CanonicalResultTable[] = legacyTables.map((table) => ({
    id: table.id,
    title: table.title,
    description: "Historical string-table result preserved without numeric reinterpretation.",
    columns: table.columns.map((label, index) => ({
      id: `column_${index + 1}`,
      label,
      data_type: "text",
      description: `Historical column ${label}`,
    })),
    rows: table.rows.map((cells, index) => ({
      id: `row_${index + 1}`,
      cells: cells.map((value) => ({ kind: "text" as const, value })),
    })),
    footnote_ids: [],
  }));
  const notices: CanonicalResultNotice[] = legacyTables.flatMap((table) => table.warning ? [{
    id: `historical_${table.id}`,
    code: "historical_string_table",
    severity: "information" as const,
    message: table.warning,
    section_ids: ["historical_results"],
    table_ids: [table.id],
  }] : []);
  return {
    schema_version: CANONICAL_RESULT_DOCUMENT_V2_SCHEMA_VERSION,
    document_id: context.document_id,
    title: context.title,
    provenance: context.provenance,
    sections: [{
      id: "historical_results",
      title: "Historical results",
      table_ids: tables.map((table) => table.id),
      chart_ids: [],
    }],
    tables,
    charts: [],
    notices,
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: "historical_results",
      default_table_id: tables[0]?.id ?? null,
      precision: 4,
      missing_value_label: "—",
      chart_defaults: {},
    },
  };
}
