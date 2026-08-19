import {
  canonicalAnalyticalResultJson,
  capabilityCellReferenceIdentityV2,
  type CanonicalResultCell,
  type CanonicalResultDocumentV2,
  type CanonicalResultTable,
  type CapabilityCellReferenceV2,
  validateCanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";

export const CANONICAL_RESULT_COMPARISON_V2_SCHEMA_VERSION = 2 as const;

export type CanonicalComparisonIssueCodeV2 =
  | "first_result_invalid"
  | "second_result_invalid"
  | "first_result_historical_text"
  | "second_result_historical_text"
  | "first_result_attribution_missing"
  | "second_result_attribution_missing"
  | "analysis_version_mismatch"
  | "analysis_components_mismatch"
  | "dataset_mismatch"
  | "model_mismatch"
  | "settings_mismatch"
  | "general_sem_results_mismatch"
  | "table_set_mismatch"
  | "column_set_mismatch"
  | "column_type_mismatch"
  | "table_analysis_components_mismatch"
  | "row_set_mismatch"
  | "comparison_build_failed";

export interface CanonicalComparisonIssueV2 {
  id: string;
  code: CanonicalComparisonIssueCodeV2;
  title: string;
  message: string;
  related_ids: string[];
  /** Internal validation details; normal customer surfaces need not show them. */
  technical_details: string[];
}

export interface CanonicalResultCompatibilityV2 {
  compatible: boolean;
  issues: CanonicalComparisonIssueV2[];
}

export interface CanonicalComparisonNumberDeltaV2 {
  id: string;
  column_id: string;
  kind: "number";
  left: number;
  right: number;
  change: number;
  absolute_change: number;
  changed: boolean;
}

export interface CanonicalComparisonTextDeltaV2 {
  id: string;
  column_id: string;
  kind: "text";
  left: string;
  right: string;
  changed: boolean;
}

export interface CanonicalComparisonBooleanDeltaV2 {
  id: string;
  column_id: string;
  kind: "boolean";
  left: boolean;
  right: boolean;
  changed: boolean;
}

export type CanonicalComparisonCellSnapshotV2 =
  | { kind: "number"; value: number }
  | { kind: "text"; value: string }
  | { kind: "boolean"; value: boolean }
  | { kind: "missing"; reason: "not_applicable" | "not_estimated" | "undefined" | "withheld" };

export interface CanonicalComparisonMissingDeltaV2 {
  id: string;
  column_id: string;
  kind: "missing";
  left: CanonicalComparisonCellSnapshotV2;
  right: CanonicalComparisonCellSnapshotV2;
  transition:
    | "unchanged_missing"
    | "missing_reason_changed"
    | "became_missing"
    | "became_available";
  changed: boolean;
}

export type CanonicalComparisonCellDeltaV2 =
  | CanonicalComparisonNumberDeltaV2
  | CanonicalComparisonTextDeltaV2
  | CanonicalComparisonBooleanDeltaV2
  | CanonicalComparisonMissingDeltaV2;

export interface CanonicalComparisonColumnV2 {
  id: string;
  label: string;
  data_type: "number" | "text" | "boolean";
  description: string;
}

export interface CanonicalComparisonRowV2 {
  id: string;
  source_row_id: string;
  cells: CanonicalComparisonCellDeltaV2[];
  changed_cell_count: number;
}

export interface CanonicalComparisonTableV2 {
  id: string;
  source_table_id: string;
  title: string;
  capability_cells: CapabilityCellReferenceV2[];
  columns: CanonicalComparisonColumnV2[];
  rows: CanonicalComparisonRowV2[];
  changed_cell_count: number;
}

export interface CanonicalResultComparisonDocumentV2 {
  schema_version: typeof CANONICAL_RESULT_COMPARISON_V2_SCHEMA_VERSION;
  comparison_id: string;
  title: string;
  sources: {
    left_document_id: string;
    right_document_id: string;
  };
  analytical_identity: {
    capability_cell: CapabilityCellReferenceV2;
    capability_cells: CapabilityCellReferenceV2[];
    dataset_fingerprint: string;
    model_digest: string;
    recipe_digest: string;
  };
  tables: CanonicalComparisonTableV2[];
  summary: {
    table_count: number;
    row_count: number;
    cell_count: number;
    changed_cell_count: number;
  };
}

export type CanonicalResultComparisonBuildV2 =
  | { compatible: false; issues: CanonicalComparisonIssueV2[] }
  | { compatible: true; issues: []; comparison: CanonicalResultComparisonDocumentV2 };

export interface CanonicalComparisonValidationV2 {
  passed: boolean;
  errors: string[];
}

const STABLE_ID = /^[a-z0-9][a-z0-9_.:-]*$/;
const HEX_64 = /^[a-f0-9]{64}$/;

function issue(
  id: string,
  code: CanonicalComparisonIssueCodeV2,
  title: string,
  message: string,
  relatedIds: readonly string[] = [],
  technicalDetails: readonly string[] = [],
): CanonicalComparisonIssueV2 {
  return {
    id,
    code,
    title,
    message,
    related_ids: [...new Set(relatedIds)].sort(),
    technical_details: [...technicalDetails],
  };
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return Object.fromEntries(Object.keys(record).sort().map((key) => [key, stableValue(record[key])]));
  }
  return value;
}

export function canonicalResultComparisonJson(comparison: CanonicalResultComparisonDocumentV2): string {
  return JSON.stringify(stableValue(comparison));
}

function sameCapabilityCell(left: CapabilityCellReferenceV2, right: CapabilityCellReferenceV2): boolean {
  return left.registry_schema_version === right.registry_schema_version
    && left.capability_id === right.capability_id
    && left.cell_id === right.cell_id
    && left.capability_version === right.capability_version;
}

function capabilityIdentities(references: readonly CapabilityCellReferenceV2[]): string[] {
  return references.map(capabilityCellReferenceIdentityV2).sort();
}

function sortedIds(values: ReadonlyArray<{ id: string }>): string[] {
  return values.map((value) => value.id).sort();
}

function differentIds(left: readonly string[], right: readonly string[]): string[] {
  const leftSet = new Set(left);
  const rightSet = new Set(right);
  return [...new Set([
    ...left.filter((id) => !rightSet.has(id)),
    ...right.filter((id) => !leftSet.has(id)),
  ])].sort();
}

function sameIds(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((id, index) => id === right[index]);
}

function isHistoricalTextFallback(document: CanonicalResultDocumentV2): boolean {
  return document.provenance.engine_version === "historical_unrecorded"
    || document.sections.some((section) => section.id === "historical_results")
    || (
      document.tables.length > 0
      && document.tables.every((table) => table.description === "Historical string-table result preserved without numeric reinterpretation.")
    );
}

function canonicalGeneralSemAnalyticalJson(document: CanonicalResultDocumentV2): string | undefined {
  const analytical = JSON.parse(canonicalAnalyticalResultJson(document)) as Record<string, unknown>;
  if (!Object.prototype.hasOwnProperty.call(analytical, "general_sem_results")) return undefined;
  return JSON.stringify(analytical.general_sem_results);
}

function invalidDocumentIssues(
  left: CanonicalResultDocumentV2,
  right: CanonicalResultDocumentV2,
): CanonicalComparisonIssueV2[] {
  const issues: CanonicalComparisonIssueV2[] = [];
  const leftValidation = validateCanonicalResultDocumentV2(left);
  const rightValidation = validateCanonicalResultDocumentV2(right);
  if (!leftValidation.passed) {
    issues.push(issue(
      "first_result_invalid",
      "first_result_invalid",
      "First result cannot be compared",
      "The first saved result is incomplete or has been altered. Reopen the original result or calculate it again.",
      [left.document_id],
      leftValidation.errors,
    ));
  }
  if (!rightValidation.passed) {
    issues.push(issue(
      "second_result_invalid",
      "second_result_invalid",
      "Second result cannot be compared",
      "The second saved result is incomplete or has been altered. Reopen the original result or calculate it again.",
      [right.document_id],
      rightValidation.errors,
    ));
  }
  return issues;
}

function historicalIssues(
  left: CanonicalResultDocumentV2,
  right: CanonicalResultDocumentV2,
): CanonicalComparisonIssueV2[] {
  const issues: CanonicalComparisonIssueV2[] = [];
  if (isHistoricalTextFallback(left)) {
    issues.push(issue(
      "first_result_historical_text",
      "first_result_historical_text",
      "First result is available for reference only",
      "This historical result contains formatted text rather than comparable typed values. Recalculate the model to include it in a side-by-side comparison.",
      [left.document_id],
    ));
  }
  if (isHistoricalTextFallback(right)) {
    issues.push(issue(
      "second_result_historical_text",
      "second_result_historical_text",
      "Second result is available for reference only",
      "This historical result contains formatted text rather than comparable typed values. Recalculate the model to include it in a side-by-side comparison.",
      [right.document_id],
    ));
  }
  return issues;
}

function attributionIssues(
  left: CanonicalResultDocumentV2,
  right: CanonicalResultDocumentV2,
): CanonicalComparisonIssueV2[] {
  const issues: CanonicalComparisonIssueV2[] = [];
  if (!left.capability_cells) {
    issues.push(issue(
      "first_result_attribution_missing",
      "first_result_attribution_missing",
      "First result needs updated analysis details",
      "The first result was saved without complete analysis-component details. Recalculate it before creating a side-by-side comparison.",
      [left.document_id],
    ));
  }
  if (!right.capability_cells) {
    issues.push(issue(
      "second_result_attribution_missing",
      "second_result_attribution_missing",
      "Second result needs updated analysis details",
      "The second result was saved without complete analysis-component details. Recalculate it before creating a side-by-side comparison.",
      [right.document_id],
    ));
  }
  return issues;
}

/** Explain whether two well-formed result documents can be compared cell by cell. */
export function canonicalResultCompatibilityV2(
  left: CanonicalResultDocumentV2,
  right: CanonicalResultDocumentV2,
): CanonicalResultCompatibilityV2 {
  const invalid = invalidDocumentIssues(left, right);
  if (invalid.length > 0) return { compatible: false, issues: invalid };

  const historical = historicalIssues(left, right);
  if (historical.length > 0) return { compatible: false, issues: historical };

  const attribution = attributionIssues(left, right);
  if (attribution.length > 0) return { compatible: false, issues: attribution };

  const issues: CanonicalComparisonIssueV2[] = [];
  if (!sameCapabilityCell(left.provenance.capability_cell, right.provenance.capability_cell)) {
    issues.push(issue(
      "analysis_version_mismatch",
      "analysis_version_mismatch",
      "Analysis or version differs",
      "These results use different analyses or analysis versions. Choose runs created with the same method and version.",
      [
        left.provenance.capability_cell.capability_id,
        left.provenance.capability_cell.cell_id,
        right.provenance.capability_cell.capability_id,
        right.provenance.capability_cell.cell_id,
      ],
    ));
  }
  const leftCapabilityIds = capabilityIdentities(left.capability_cells!);
  const rightCapabilityIds = capabilityIdentities(right.capability_cells!);
  if (!sameIds(leftCapabilityIds, rightCapabilityIds)) {
    issues.push(issue(
      "analysis_components_mismatch",
      "analysis_components_mismatch",
      "Included analysis components differ",
      "These results contain different analysis components. Choose runs created with the same calculation and result options.",
      differentIds(leftCapabilityIds, rightCapabilityIds),
    ));
  }
  if (left.provenance.dataset_fingerprint !== right.provenance.dataset_fingerprint) {
    issues.push(issue(
      "dataset_mismatch",
      "dataset_mismatch",
      "Data differs",
      "These results use different data. Choose runs calculated from the same dataset.",
    ));
  }
  if (left.provenance.model_digest !== right.provenance.model_digest) {
    issues.push(issue(
      "model_mismatch",
      "model_mismatch",
      "Model differs",
      "These results use different models. Choose runs calculated from the same model.",
    ));
  }
  if (left.provenance.recipe_digest !== right.provenance.recipe_digest) {
    issues.push(issue(
      "settings_mismatch",
      "settings_mismatch",
      "Analysis settings differ",
      "These results use different analysis settings. Choose runs calculated with the same settings.",
    ));
  }
  if (canonicalGeneralSemAnalyticalJson(left) !== canonicalGeneralSemAnalyticalJson(right)) {
    issues.push(issue(
      "general_sem_results_mismatch",
      "general_sem_results_mismatch",
      "General SEM analytical results differ",
      "These runs contain different General SEM analytical results. This comparison version cannot safely reduce every nested General SEM result to typed table deltas, so it will not report an incomplete zero-change comparison.",
      [left.document_id, right.document_id],
    ));
  }

  const leftTableIds = sortedIds(left.tables);
  const rightTableIds = sortedIds(right.tables);
  if (!sameIds(leftTableIds, rightTableIds)) {
    issues.push(issue(
      "table_set_mismatch",
      "table_set_mismatch",
      "Result tables differ",
      "The reports do not contain the same result tables. Choose runs with the same result options.",
      differentIds(leftTableIds, rightTableIds),
    ));
  }

  const leftTables = new Map(left.tables.map((table) => [table.id, table]));
  const rightTables = new Map(right.tables.map((table) => [table.id, table]));
  for (const tableId of leftTableIds.filter((id) => rightTables.has(id))) {
    const leftTable = leftTables.get(tableId)!;
    const rightTable = rightTables.get(tableId)!;
    const leftTableCapabilityIds = capabilityIdentities(leftTable.capability_cells!);
    const rightTableCapabilityIds = capabilityIdentities(rightTable.capability_cells!);
    if (!sameIds(leftTableCapabilityIds, rightTableCapabilityIds)) {
      issues.push(issue(
        `table_analysis_components_mismatch:${tableId}`,
        "table_analysis_components_mismatch",
        `Analysis components differ in ${leftTable.title}`,
        `The ${leftTable.title} tables were produced by different analysis components. Choose runs created with the same calculation and result options.`,
        [tableId, ...differentIds(leftTableCapabilityIds, rightTableCapabilityIds)],
      ));
    }
    const leftColumnIds = sortedIds(leftTable.columns);
    const rightColumnIds = sortedIds(rightTable.columns);
    if (!sameIds(leftColumnIds, rightColumnIds)) {
      issues.push(issue(
        `column_set_mismatch:${tableId}`,
        "column_set_mismatch",
        `Columns differ in ${leftTable.title}`,
        `The ${leftTable.title} tables do not contain the same columns. Choose runs with the same result options.`,
        [tableId, ...differentIds(leftColumnIds, rightColumnIds)],
      ));
    }
    const rightColumns = new Map(rightTable.columns.map((column) => [column.id, column]));
    for (const columnId of leftColumnIds.filter((id) => rightColumns.has(id))) {
      const leftColumn = leftTable.columns.find((column) => column.id === columnId)!;
      const rightColumn = rightColumns.get(columnId)!;
      if (leftColumn.data_type !== rightColumn.data_type) {
        issues.push(issue(
          `column_type_mismatch:${tableId}:${columnId}`,
          "column_type_mismatch",
          `Value type differs in ${leftTable.title}`,
          `The ${leftColumn.label} column in ${leftTable.title} was stored with a different value type. Recalculate both runs with the same analysis version.`,
          [tableId, columnId],
        ));
      }
    }

    const leftRowIds = sortedIds(leftTable.rows);
    const rightRowIds = sortedIds(rightTable.rows);
    if (!sameIds(leftRowIds, rightRowIds)) {
      issues.push(issue(
        `row_set_mismatch:${tableId}`,
        "row_set_mismatch",
        `Result rows differ in ${leftTable.title}`,
        `The ${leftTable.title} tables do not contain the same result rows. Check that both runs use the same model and result options.`,
        [tableId, ...differentIds(leftRowIds, rightRowIds)],
      ));
    }
  }

  return { compatible: issues.length === 0, issues };
}

function cellByColumn(table: CanonicalResultTable, rowId: string): Map<string, CanonicalResultCell> {
  const row = table.rows.find((candidate) => candidate.id === rowId)!;
  return new Map(table.columns.map((column, index) => [column.id, row.cells[index]]));
}

function snapshot(cell: CanonicalResultCell): CanonicalComparisonCellSnapshotV2 {
  if (cell.kind === "number") return { kind: "number", value: cell.value };
  if (cell.kind === "text") return { kind: "text", value: cell.value };
  if (cell.kind === "boolean") return { kind: "boolean", value: cell.value };
  return { kind: "missing", reason: cell.reason };
}

function normalizedNumber(value: number): number {
  return Object.is(value, -0) ? 0 : value;
}

function compareId(left: { id: string }, right: { id: string }): number {
  return left.id === right.id ? 0 : left.id < right.id ? -1 : 1;
}

function compareCell(
  rowId: string,
  columnId: string,
  left: CanonicalResultCell,
  right: CanonicalResultCell,
): CanonicalComparisonCellDeltaV2 {
  const id = `${rowId}:${columnId}`;
  if (left.kind === "missing" || right.kind === "missing") {
    let transition: CanonicalComparisonMissingDeltaV2["transition"];
    let changed = true;
    if (left.kind === "missing" && right.kind === "missing") {
      changed = left.reason !== right.reason;
      transition = changed ? "missing_reason_changed" : "unchanged_missing";
    } else {
      transition = left.kind === "missing" ? "became_available" : "became_missing";
    }
    return {
      id,
      column_id: columnId,
      kind: "missing",
      left: snapshot(left),
      right: snapshot(right),
      transition,
      changed,
    };
  }
  if (left.kind === "number" && right.kind === "number") {
    const change = normalizedNumber(right.value - left.value);
    return {
      id,
      column_id: columnId,
      kind: "number",
      left: left.value,
      right: right.value,
      change,
      absolute_change: Math.abs(change),
      changed: left.value !== right.value,
    };
  }
  if (left.kind === "text" && right.kind === "text") {
    return { id, column_id: columnId, kind: "text", left: left.value, right: right.value, changed: left.value !== right.value };
  }
  if (left.kind === "boolean" && right.kind === "boolean") {
    return { id, column_id: columnId, kind: "boolean", left: left.value, right: right.value, changed: left.value !== right.value };
  }
  throw new Error(`Compatible comparison cell types differ at ${id}`);
}

function comparisonTable(left: CanonicalResultTable, right: CanonicalResultTable): CanonicalComparisonTableV2 {
  const columns = [...left.columns]
    .sort(compareId)
    .map((column): CanonicalComparisonColumnV2 => ({
      id: column.id,
      label: column.label,
      data_type: column.data_type,
      description: column.description,
    }));
  const rightRows = new Map(right.rows.map((row) => [row.id, row]));
  const rows = [...left.rows]
    .sort(compareId)
    .map((leftRow): CanonicalComparisonRowV2 => {
      const rightRow = rightRows.get(leftRow.id)!;
      const leftCells = cellByColumn(left, leftRow.id);
      const rightCells = cellByColumn(right, rightRow.id);
      const cells = columns.map((column) => compareCell(
        leftRow.id,
        column.id,
        leftCells.get(column.id)!,
        rightCells.get(column.id)!,
      ));
      return {
        id: leftRow.id,
        source_row_id: leftRow.id,
        cells,
        changed_cell_count: cells.filter((cell) => cell.changed).length,
      };
    });
  return {
    id: `comparison:${left.id}`,
    source_table_id: left.id,
    title: left.title,
    capability_cells: left.capability_cells!.map((reference) => ({ ...reference })),
    columns,
    rows,
    changed_cell_count: rows.reduce((total, row) => total + row.changed_cell_count, 0),
  };
}

export function compareCanonicalResultDocumentsV2(
  left: CanonicalResultDocumentV2,
  right: CanonicalResultDocumentV2,
): CanonicalResultComparisonBuildV2 {
  const compatibility = canonicalResultCompatibilityV2(left, right);
  if (!compatibility.compatible) return { compatible: false, issues: compatibility.issues };

  const rightTables = new Map(right.tables.map((table) => [table.id, table]));
  const tables = [...left.tables]
    .sort(compareId)
    .map((table) => comparisonTable(table, rightTables.get(table.id)!));
  const rowCount = tables.reduce((total, table) => total + table.rows.length, 0);
  const cellCount = tables.reduce((total, table) => (
    total + table.rows.reduce((rowTotal, row) => rowTotal + row.cells.length, 0)
  ), 0);
  const changedCellCount = tables.reduce((total, table) => total + table.changed_cell_count, 0);
  const comparison: CanonicalResultComparisonDocumentV2 = {
    schema_version: CANONICAL_RESULT_COMPARISON_V2_SCHEMA_VERSION,
    comparison_id: `comparison:${left.document_id}:to:${right.document_id}`,
    title: `${left.title} compared with ${right.title}`,
    sources: {
      left_document_id: left.document_id,
      right_document_id: right.document_id,
    },
    analytical_identity: {
      capability_cell: { ...left.provenance.capability_cell },
      capability_cells: left.capability_cells!.map((reference) => ({ ...reference })),
      dataset_fingerprint: left.provenance.dataset_fingerprint,
      model_digest: left.provenance.model_digest,
      recipe_digest: left.provenance.recipe_digest,
    },
    tables,
    summary: {
      table_count: tables.length,
      row_count: rowCount,
      cell_count: cellCount,
      changed_cell_count: changedCellCount,
    },
  };
  const validation = validateCanonicalResultComparisonDocumentV2(comparison);
  if (!validation.passed) {
    return {
      compatible: false,
      issues: [issue(
        "comparison_document_invalid",
        "comparison_build_failed",
        "Comparison could not be prepared",
        "QuickPLS could not prepare a complete side-by-side comparison. Reopen both results and try again.",
        [],
        validation.errors,
      )],
    };
  }
  return { compatible: true, issues: [], comparison };
}

function duplicates(values: readonly string[]): string[] {
  const seen = new Set<string>();
  const repeated = new Set<string>();
  for (const value of values) {
    if (seen.has(value)) repeated.add(value);
    seen.add(value);
  }
  return [...repeated].sort();
}

function validateIds(errors: string[], values: readonly string[], context: string): void {
  const repeated = duplicates(values);
  if (repeated.length > 0) errors.push(`${context} contains duplicate IDs: ${repeated.join(", ")}`);
  for (const value of values) if (!STABLE_ID.test(value)) errors.push(`${context} contains an invalid ID: ${value}`);
}

function requireCanonicalOrder(errors: string[], values: readonly string[], context: string): void {
  const expected = [...values].sort();
  if (!values.every((value, index) => value === expected[index])) errors.push(`${context} must be ordered by stable ID`);
}

function finiteSnapshot(snapshot: CanonicalComparisonCellSnapshotV2): boolean {
  return snapshot.kind !== "number" || Number.isFinite(snapshot.value);
}

export function validateCanonicalResultComparisonDocumentV2(
  comparison: CanonicalResultComparisonDocumentV2,
): CanonicalComparisonValidationV2 {
  const errors: string[] = [];
  if (comparison.schema_version !== CANONICAL_RESULT_COMPARISON_V2_SCHEMA_VERSION) errors.push("schema_version must equal 2");
  if (!STABLE_ID.test(comparison.comparison_id)) errors.push("comparison_id must be a stable lowercase identifier");
  if (!comparison.title.trim()) errors.push("title must be nonempty");
  if (!comparison.sources.left_document_id.trim() || !comparison.sources.right_document_id.trim()) errors.push("source document IDs must be nonempty");
  const identity = comparison.analytical_identity;
  for (const [name, value] of Object.entries({
    dataset_fingerprint: identity.dataset_fingerprint,
    model_digest: identity.model_digest,
    recipe_digest: identity.recipe_digest,
  })) {
    if (!HEX_64.test(value)) errors.push(`${name} must be lowercase SHA-256`);
  }
  if (identity.capability_cell.registry_schema_version !== 2
    || !STABLE_ID.test(identity.capability_cell.capability_id)
    || !STABLE_ID.test(identity.capability_cell.cell_id)
    || !STABLE_ID.test(identity.capability_cell.capability_version)) {
    errors.push("capability_cell must be a complete V2 reference");
  }
  const identityCapabilityIds = capabilityIdentities(identity.capability_cells);
  const rawIdentityCapabilityIds = identity.capability_cells.map(capabilityCellReferenceIdentityV2);
  if (identityCapabilityIds.length === 0) errors.push("capability_cells must not be empty");
  if (new Set(identityCapabilityIds).size !== identityCapabilityIds.length) errors.push("capability_cells contains duplicate references");
  if (!rawIdentityCapabilityIds.every((value, index) => value === identityCapabilityIds[index])) errors.push("capability_cells must be ordered by exact option-cell identity");
  if (!identityCapabilityIds.includes(capabilityCellReferenceIdentityV2(identity.capability_cell))) errors.push("capability_cells must include capability_cell");
  identity.capability_cells.forEach((reference, index) => {
    if (reference.registry_schema_version !== 2
      || !STABLE_ID.test(reference.capability_id)
      || !STABLE_ID.test(reference.cell_id)
      || !STABLE_ID.test(reference.capability_version)) {
      errors.push(`capability_cells[${index}] must be a complete V2 reference`);
    }
  });

  validateIds(errors, comparison.tables.map((table) => table.id), "tables");
  validateIds(errors, comparison.tables.map((table) => table.source_table_id), "source tables");
  requireCanonicalOrder(errors, comparison.tables.map((table) => table.source_table_id), "source tables");
  let rowCount = 0;
  let cellCount = 0;
  let changedCellCount = 0;
  for (const table of comparison.tables) {
    if (!table.title.trim()) errors.push(`table ${table.id} title must be nonempty`);
    if (table.id !== `comparison:${table.source_table_id}`) errors.push(`table ${table.id} does not match its source table ID`);
    const tableCapabilityIds = capabilityIdentities(table.capability_cells);
    const rawTableCapabilityIds = table.capability_cells.map(capabilityCellReferenceIdentityV2);
    if (tableCapabilityIds.length === 0) errors.push(`table ${table.id} capability_cells must not be empty`);
    if (new Set(tableCapabilityIds).size !== tableCapabilityIds.length) errors.push(`table ${table.id} capability_cells contains duplicate references`);
    if (!rawTableCapabilityIds.every((value, index) => value === tableCapabilityIds[index])) errors.push(`table ${table.id} capability_cells must be ordered by exact option-cell identity`);
    table.capability_cells.forEach((reference, index) => {
      if (reference.registry_schema_version !== 2
        || !STABLE_ID.test(reference.capability_id)
        || !STABLE_ID.test(reference.cell_id)
        || !STABLE_ID.test(reference.capability_version)) {
        errors.push(`table ${table.id} capability_cells[${index}] must be a complete V2 reference`);
      }
    });
    for (const identityValue of tableCapabilityIds) {
      if (!identityCapabilityIds.includes(identityValue)) errors.push(`table ${table.id} references an undeclared option cell ${identityValue}`);
    }
    validateIds(errors, table.columns.map((column) => column.id), `table ${table.id} columns`);
    validateIds(errors, table.rows.map((row) => row.id), `table ${table.id} rows`);
    requireCanonicalOrder(errors, table.columns.map((column) => column.id), `table ${table.id} columns`);
    requireCanonicalOrder(errors, table.rows.map((row) => row.id), `table ${table.id} rows`);
    for (const column of table.columns) {
      if (!column.label.trim() || !column.description.trim()) errors.push(`table ${table.id} column ${column.id} needs a label and description`);
    }
    const columnIds = table.columns.map((column) => column.id);
    let tableChanged = 0;
    for (const row of table.rows) {
      rowCount += 1;
      cellCount += row.cells.length;
      if (row.source_row_id !== row.id) errors.push(`table ${table.id} row ${row.id} source_row_id must match`);
      if (row.cells.length !== table.columns.length) errors.push(`table ${table.id} row ${row.id} has the wrong cell count`);
      validateIds(errors, row.cells.map((cell) => cell.id), `table ${table.id} row ${row.id} cells`);
      row.cells.forEach((cell, index) => {
        const columnId = columnIds[index];
        const column = table.columns[index];
        if (cell.column_id !== columnId || cell.id !== `${row.id}:${columnId}`) {
          errors.push(`table ${table.id} row ${row.id} cell ${index + 1} does not match its column`);
        }
        if (cell.kind === "number") {
          if (![cell.left, cell.right, cell.change, cell.absolute_change].every(Number.isFinite)) {
            errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} contains a non-finite number`);
          }
          const expectedChange = normalizedNumber(cell.right - cell.left);
          if (cell.change !== expectedChange || cell.absolute_change !== Math.abs(expectedChange) || cell.changed !== (cell.left !== cell.right)) {
            errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} has an inconsistent numeric delta`);
          }
        }
        if (cell.kind === "text" && cell.changed !== (cell.left !== cell.right)) {
          errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} has an inconsistent text delta`);
        }
        if (cell.kind === "boolean" && cell.changed !== (cell.left !== cell.right)) {
          errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} has an inconsistent boolean delta`);
        }
        if (cell.kind === "missing") {
          if (!finiteSnapshot(cell.left) || !finiteSnapshot(cell.right)) {
            errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} contains a non-finite snapshot`);
          }
          let expectedTransition: CanonicalComparisonMissingDeltaV2["transition"];
          if (cell.left.kind === "missing" && cell.right.kind === "missing") {
            expectedTransition = cell.left.reason === cell.right.reason
              ? "unchanged_missing"
              : "missing_reason_changed";
          } else {
            expectedTransition = cell.left.kind === "missing" ? "became_available" : "became_missing";
          }
          const expectedChanged = expectedTransition !== "unchanged_missing";
          if (cell.transition !== expectedTransition || cell.changed !== expectedChanged) {
            errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} has an inconsistent missing-value transition`);
          }
          for (const value of [cell.left, cell.right]) {
            if (value.kind !== "missing" && value.kind !== column?.data_type) {
              errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} snapshot does not match its column type`);
            }
          }
        } else if (cell.kind !== column?.data_type) {
          errors.push(`table ${table.id} row ${row.id} column ${cell.column_id} delta does not match its column type`);
        }
        if (cell.changed) tableChanged += 1;
      });
      const actualChanged = row.cells.filter((cell) => cell.changed).length;
      if (row.changed_cell_count !== actualChanged) errors.push(`table ${table.id} row ${row.id} changed-cell count is inconsistent`);
    }
    if (table.changed_cell_count !== tableChanged) errors.push(`table ${table.id} changed-cell count is inconsistent`);
    changedCellCount += tableChanged;
  }
  for (const [name, value] of Object.entries(comparison.summary)) {
    if (!Number.isSafeInteger(value) || value < 0) errors.push(`summary.${name} must be a nonnegative safe integer`);
  }
  if (comparison.summary.table_count !== comparison.tables.length) errors.push("summary.table_count is inconsistent");
  if (comparison.summary.row_count !== rowCount) errors.push("summary.row_count is inconsistent");
  if (comparison.summary.cell_count !== cellCount) errors.push("summary.cell_count is inconsistent");
  if (comparison.summary.changed_cell_count !== changedCellCount) errors.push("summary.changed_cell_count is inconsistent");
  return { passed: errors.length === 0, errors };
}
