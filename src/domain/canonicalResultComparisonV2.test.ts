import { describe, expect, it } from "vitest";
import {
  canonicalResultDocumentFromLegacyTables,
  type CanonicalResultDocumentV2,
  validateCanonicalResultDocumentV2,
} from "./canonicalResultDocumentV2";
import {
  canonicalResultComparisonJson,
  canonicalResultCompatibilityV2,
  compareCanonicalResultDocumentsV2,
  validateCanonicalResultComparisonDocumentV2,
} from "./canonicalResultComparisonV2";

const digest = (character: string) => character.repeat(64);

function resultDocument(documentId: string, right = false): CanonicalResultDocumentV2 {
  const capabilityCell = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_pm_v1",
  };
  return {
    schema_version: 2,
    document_id: documentId,
    title: right ? "Comparison result" : "Baseline result",
    provenance: {
      run_id: right ? "run-right" : "run-left",
      project_id: "project-one",
      model_id: "model-one",
      model_digest: digest("a"),
      dataset_id: "dataset-one",
      dataset_fingerprint: digest("b"),
      recipe_id: "recipe-one",
      recipe_digest: digest("c"),
      capability_cell: { ...capabilityCell },
      method_version: "pls_pm_v1",
      engine_version: right ? "qpls-estimation-next" : "qpls-estimation-current",
      seed: 42,
      workers: right ? 1 : 4,
      started_at: right ? "2026-08-14T00:01:00Z" : "2026-08-14T00:00:00Z",
      completed_at: right ? "2026-08-14T00:01:02Z" : "2026-08-14T00:00:01Z",
    },
    capability_cells: [{ ...capabilityCell }],
    sections: [{
      id: "results",
      title: "Results",
      table_ids: ["metadata", "effects"],
      chart_ids: ["effect_chart"],
      capability_cells: [{ ...capabilityCell }],
    }],
    tables: [
      {
        id: "metadata",
        title: "Model information",
        columns: [{ id: "metric", label: "Metric", data_type: "text", description: "Model information item" }],
        rows: [{ id: "method", cells: [{ kind: "text", value: "PLS-SEM" }] }],
        footnote_ids: [],
        capability_cells: [{ ...capabilityCell }],
      },
      {
        id: "effects",
        title: "Effects",
        columns: [
          { id: "label", label: "Effect", data_type: "text", description: "Effect label" },
          { id: "estimate", label: "Estimate", data_type: "number", description: "Effect estimate" },
          { id: "selected", label: "Selected", data_type: "boolean", description: "Selection flag" },
          { id: "optional", label: "Optional estimate", data_type: "number", description: "Optional result" },
        ],
        rows: [
          {
            id: "row_b",
            cells: [
              { kind: "text", value: "B" },
              { kind: "number", value: 0.2, display: right ? "0.200000" : "0.20" },
              { kind: "boolean", value: false },
              { kind: "missing", reason: right ? "withheld" : "not_estimated", display: right ? "Hidden" : "—" },
            ],
          },
          {
            id: "row_a",
            cells: [
              { kind: "text", value: right ? "Beta" : "Alpha" },
              { kind: "number", value: right ? 0.5 : 0.4, display: right ? "0.500000" : "0.40" },
              { kind: "boolean", value: !right },
              right
                ? { kind: "number", value: 8, display: "8.0000" }
                : { kind: "missing", reason: "undefined", display: "N/A" },
            ],
          },
        ],
        footnote_ids: [],
        capability_cells: [{ ...capabilityCell }],
      },
    ],
    charts: [{
      id: "effect_chart",
      title: "Effect chart",
      description: "Comparison effect shown as one point.",
      kind: "bar",
      series: [{ id: "effect", label: "Effect", points: [{ x: "A", y: right ? 0.5 : 0.4 }] }],
      source_table_id: "effects",
      display: { palette: right ? "journal_mono" : "institutional_navy", show_values: right },
    }],
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: "results",
      default_table_id: "effects",
      precision: right ? 6 : 2,
      missing_value_label: right ? "N/A" : "—",
      chart_defaults: { palette: right ? "journal_mono" : "institutional_navy" },
    },
  };
}

function withGeneralSemFit(document: CanonicalResultDocumentV2, chiSquare = 12.5): CanonicalResultDocumentV2 {
  document.general_sem_results = {
    schema_version: 1,
    cbsem_fit: [{
      fit_id: "fit_model",
      trace: {
        model_id: document.provenance.model_id,
        capability_cell: { ...document.provenance.capability_cell },
      },
      chi_square: chiSquare,
      degrees_of_freedom: 8,
      chi_square_p_value: 0.13,
      rmsea: 0.04,
      cfi: 0.97,
      srmr: 0.03,
    }],
  };
  return document;
}

function clone<T>(value: T): T {
  return structuredClone(value);
}

function replaceCapabilityVersion(document: CanonicalResultDocumentV2, capabilityVersion: string): void {
  document.provenance.capability_cell.capability_version = capabilityVersion;
  document.capability_cells!.forEach((reference) => { reference.capability_version = capabilityVersion; });
  document.sections.forEach((section) => section.capability_cells!.forEach((reference) => {
    reference.capability_version = capabilityVersion;
  }));
  document.tables.forEach((table) => table.capability_cells!.forEach((reference) => {
    reference.capability_version = capabilityVersion;
  }));
}

function reorderColumns(document: CanonicalResultDocumentV2): void {
  const table = document.tables.find((candidate) => candidate.id === "effects")!;
  const originalColumns = [...table.columns];
  const indexes = originalColumns.map((_column, index) => index).reverse();
  table.columns = indexes.map((index) => originalColumns[index]);
  table.rows = [...table.rows].reverse().map((row) => ({
    ...row,
    cells: indexes.map((index) => row.cells[index]),
  }));
  document.tables.reverse();
  document.sections[0].table_ids.reverse();
}

describe("CanonicalResultDocumentV2 semantic comparison", () => {
  it("produces typed, deterministic deltas for compatible result documents", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    expect(validateCanonicalResultDocumentV2(left)).toEqual({ passed: true, errors: [] });
    expect(validateCanonicalResultDocumentV2(right)).toEqual({ passed: true, errors: [] });

    const built = compareCanonicalResultDocumentsV2(left, right);

    expect(built.compatible).toBe(true);
    if (!built.compatible) return;
    expect(validateCanonicalResultComparisonDocumentV2(built.comparison)).toEqual({ passed: true, errors: [] });
    expect(built.comparison.tables.map((table) => table.source_table_id)).toEqual(["effects", "metadata"]);
    const effects = built.comparison.tables[0];
    expect(effects.columns.map((column) => column.id)).toEqual(["estimate", "label", "optional", "selected"]);
    expect(effects.rows.map((row) => row.id)).toEqual(["row_a", "row_b"]);
    expect(effects.rows[0].cells.find((cell) => cell.column_id === "estimate")).toMatchObject({
      kind: "number",
      left: 0.4,
      right: 0.5,
      change: expect.closeTo(0.1),
      absolute_change: expect.closeTo(0.1),
      changed: true,
    });
    expect(effects.rows[0].cells.find((cell) => cell.column_id === "label")).toMatchObject({
      kind: "text",
      left: "Alpha",
      right: "Beta",
      changed: true,
    });
    expect(effects.rows[0].cells.find((cell) => cell.column_id === "selected")).toMatchObject({
      kind: "boolean",
      left: true,
      right: false,
      changed: true,
    });
    expect(effects.rows[0].cells.find((cell) => cell.column_id === "optional")).toMatchObject({
      kind: "missing",
      transition: "became_available",
      changed: true,
    });
    expect(effects.rows[1].cells.find((cell) => cell.column_id === "optional")).toMatchObject({
      kind: "missing",
      transition: "missing_reason_changed",
      changed: true,
    });
    expect(built.comparison.summary).toEqual({ table_count: 2, row_count: 3, cell_count: 9, changed_cell_count: 5 });
  });

  it("normalizes table, row, and column reorder by stable IDs", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    const expected = compareCanonicalResultDocumentsV2(left, right);
    const reorderedLeft = clone(left);
    const reorderedRight = clone(right);
    reorderColumns(reorderedLeft);
    reorderColumns(reorderedRight);

    const actual = compareCanonicalResultDocumentsV2(reorderedLeft, reorderedRight);

    expect(expected.compatible && actual.compatible).toBe(true);
    if (!expected.compatible || !actual.compatible) return;
    expect(canonicalResultComparisonJson(actual.comparison)).toBe(canonicalResultComparisonJson(expected.comparison));
  });

  it("accepts identical valid General SEM analytical extensions", () => {
    const left = withGeneralSemFit(resultDocument("result.left"));
    const right = clone(left);
    right.document_id = "result.right";
    right.provenance.run_id = "run-right";
    expect(validateCanonicalResultDocumentV2(left)).toEqual({ passed: true, errors: [] });
    expect(validateCanonicalResultDocumentV2(right)).toEqual({ passed: true, errors: [] });

    const built = compareCanonicalResultDocumentsV2(left, right);

    expect(built.compatible).toBe(true);
    if (!built.compatible) return;
    expect(built.comparison.summary.changed_cell_count).toBe(0);
  });

  it("fails closed instead of reporting zero changes when only General SEM results differ", () => {
    const left = withGeneralSemFit(resultDocument("result.left"), 12.5);
    const right = clone(left);
    right.document_id = "result.right";
    right.provenance.run_id = "run-right";
    right.general_sem_results!.cbsem_fit![0].chi_square = 14.25;
    expect(validateCanonicalResultDocumentV2(left)).toEqual({ passed: true, errors: [] });
    expect(validateCanonicalResultDocumentV2(right)).toEqual({ passed: true, errors: [] });

    expect(compareCanonicalResultDocumentsV2(left, right)).toMatchObject({
      compatible: false,
      issues: [{
        code: "general_sem_results_mismatch",
        title: "General SEM analytical results differ",
      }],
    });
  });

  it("rejects tampered General SEM results before compatibility analysis", () => {
    const left = withGeneralSemFit(resultDocument("result.left"));
    const right = clone(left);
    right.document_id = "result.right";
    const tampered = right.general_sem_results!.cbsem_fit![0] as unknown as Record<string, unknown>;
    tampered.unexpected = true;

    expect(canonicalResultCompatibilityV2(left, right)).toMatchObject({
      compatible: false,
      issues: [{
        code: "second_result_invalid",
        technical_details: [expect.stringContaining("general_sem_results.cbsem_fit[0].unexpected")],
      }],
    });
  });

  it("excludes display caches, chart styling, precision, workers, and timing", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    const expected = compareCanonicalResultDocumentsV2(left, right);
    const displayLeft = clone(left);
    const displayRight = clone(right);
    displayLeft.presentation.precision = 11;
    displayRight.presentation.precision = 0;
    displayLeft.presentation.missing_value_label = "Missing";
    displayRight.presentation.chart_defaults = { palette: "high_contrast", show_legend: false };
    displayLeft.charts[0].display = { palette: "high_contrast", show_values: false };
    displayRight.charts[0].display = { palette: "quickpls_color", show_values: true };
    (displayLeft.tables[1].rows[1].cells[1] as { display?: string }).display = "0.40000000000";
    (displayRight.tables[1].rows[0].cells[3] as { display?: string }).display = "Withheld";
    displayLeft.provenance.workers = 16;
    displayRight.provenance.workers = 2;
    displayLeft.provenance.started_at = "2026-08-14T00:10:00Z";
    displayLeft.provenance.completed_at = "2026-08-14T00:10:05Z";

    const actual = compareCanonicalResultDocumentsV2(displayLeft, displayRight);

    expect(expected.compatible && actual.compatible).toBe(true);
    if (!expected.compatible || !actual.compatible) return;
    expect(canonicalResultComparisonJson(actual.comparison)).toBe(canonicalResultComparisonJson(expected.comparison));
  });

  it("reports all analytical identity and schema incompatibilities in customer language", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    replaceCapabilityVersion(right, "pls_pm_v2");
    right.provenance.dataset_fingerprint = digest("d");
    right.provenance.model_digest = digest("e");
    right.provenance.recipe_digest = digest("f");
    right.tables = right.tables.filter((table) => table.id !== "metadata");
    right.sections[0].table_ids = right.sections[0].table_ids.filter((id) => id !== "metadata");
    const effects = right.tables[0];
    const optionalIndex = effects.columns.findIndex((column) => column.id === "optional");
    effects.columns.splice(optionalIndex, 1);
    effects.rows.forEach((row) => row.cells.splice(optionalIndex, 1));
    effects.rows = effects.rows.filter((row) => row.id !== "row_b");

    const compatibility = canonicalResultCompatibilityV2(left, right);

    expect(compatibility.compatible).toBe(false);
    expect(compatibility.issues.map((item) => item.code)).toEqual([
      "analysis_version_mismatch",
      "analysis_components_mismatch",
      "dataset_mismatch",
      "model_mismatch",
      "settings_mismatch",
      "table_set_mismatch",
      "table_analysis_components_mismatch",
      "column_set_mismatch",
      "row_set_mismatch",
    ]);
    expect(compatibility.issues.every((item) => item.title.trim() && item.message.trim())).toBe(true);
  });

  it("compares the full option-cell set and each table attribution, not only the primary cell", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    const htmt = {
      registry_schema_version: 2 as const,
      capability_id: "smartpls.htmt",
      cell_id: "qpls3.assessment.htmt",
      capability_version: "ringle_et_al_htmt_plus_v1",
    };
    right.capability_cells!.unshift({ ...htmt });
    right.sections[0].capability_cells!.unshift({ ...htmt });
    right.tables.find((table) => table.id === "effects")!.capability_cells!.unshift({ ...htmt });
    expect(validateCanonicalResultDocumentV2(right)).toEqual({ passed: true, errors: [] });

    const compatibility = canonicalResultCompatibilityV2(left, right);

    expect(compatibility.issues.map((item) => item.code)).toEqual([
      "analysis_components_mismatch",
      "table_analysis_components_mismatch",
    ]);
    expect(compatibility.issues[0].related_ids.join(" ")).toContain("smartpls.htmt");
  });

  it("does not infer missing option-cell attribution from the primary cell", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    delete left.capability_cells;
    left.sections.forEach((section) => { delete section.capability_cells; });
    left.tables.forEach((table) => { delete table.capability_cells; });
    expect(validateCanonicalResultDocumentV2(left)).toEqual({ passed: true, errors: [] });

    expect(canonicalResultCompatibilityV2(left, right)).toMatchObject({
      compatible: false,
      issues: [{ code: "first_result_attribution_missing" }],
    });
  });

  it("rejects column type drift even when both source documents are individually valid", () => {
    const left = resultDocument("result.left");
    const right = resultDocument("result.right", true);
    const effects = right.tables.find((table) => table.id === "effects")!;
    const estimateIndex = effects.columns.findIndex((column) => column.id === "estimate");
    effects.columns[estimateIndex].data_type = "text";
    effects.rows.forEach((row) => {
      const value = row.cells[estimateIndex];
      row.cells[estimateIndex] = { kind: "text", value: value.kind === "number" ? String(value.value) : "" };
    });
    expect(validateCanonicalResultDocumentV2(right).passed).toBe(true);

    expect(canonicalResultCompatibilityV2(left, right).issues).toEqual(expect.arrayContaining([
      expect.objectContaining({ code: "column_type_mismatch", related_ids: ["effects", "estimate"] }),
    ]));
  });

  it("fails closed on duplicate or non-finite source content", () => {
    const duplicateLeft = resultDocument("result.left");
    duplicateLeft.tables[1].rows.push(clone(duplicateLeft.tables[1].rows[0]));
    const nonFiniteRight = resultDocument("result.right", true);
    const numberCell = nonFiniteRight.tables[1].rows[1].cells[1];
    if (numberCell.kind === "number") numberCell.value = Number.POSITIVE_INFINITY;

    const compatibility = canonicalResultCompatibilityV2(duplicateLeft, nonFiniteRight);

    expect(compatibility.compatible).toBe(false);
    expect(compatibility.issues.map((item) => item.code)).toEqual(["first_result_invalid", "second_result_invalid"]);
    expect(compatibility.issues[0].technical_details.join(" ")).toContain("duplicate IDs");
    expect(compatibility.issues[1].technical_details.join(" ")).toContain("must be finite");
  });

  it("keeps historical text-only results readable but incompatible with typed comparison", () => {
    const current = resultDocument("result.current");
    const historical = canonicalResultDocumentFromLegacyTables({
      document_id: "result.historical",
      title: "Historical result",
      provenance: { ...current.provenance, engine_version: "historical_unrecorded" },
    }, [{
      id: "effects",
      title: "Effects",
      columns: ["Effect", "Estimate"],
      rows: [["A", "0.4000"]],
    }]);

    const compatibility = canonicalResultCompatibilityV2(current, historical);

    expect(compatibility).toMatchObject({
      compatible: false,
      issues: [{
        code: "second_result_historical_text",
        title: "Second result is available for reference only",
      }],
    });
  });

  it("validates generated comparison counts, IDs, and finite deltas", () => {
    const built = compareCanonicalResultDocumentsV2(resultDocument("result.left"), resultDocument("result.right", true));
    expect(built.compatible).toBe(true);
    if (!built.compatible) return;
    const tampered = clone(built.comparison);
    tampered.tables.push(clone(tampered.tables[0]));
    const numeric = tampered.tables[0].rows[0].cells.find((cell) => cell.kind === "number");
    if (numeric?.kind === "number") numeric.change = Number.NaN;

    const validation = validateCanonicalResultComparisonDocumentV2(tampered);

    expect(validation.passed).toBe(false);
    expect(validation.errors).toEqual(expect.arrayContaining([
      expect.stringContaining("tables contains duplicate IDs"),
      expect.stringContaining("contains a non-finite number"),
      expect.stringContaining("summary.table_count is inconsistent"),
    ]));
  });
});
