import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  buildCanonicalResultSemanticExportV2,
  canonicalResultSemanticExportJsonV2,
  parseCanonicalResultSemanticExportJsonV2,
  verifyCanonicalResultSemanticExportReadbackV2,
} from "./canonicalResultSemanticExportV2";

const digest = "a".repeat(64);

function resultDocument(): CanonicalResultDocumentV2 {
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.pls_algorithm",
    cell_id: "qpls3.pls.algorithm",
    capability_version: "pls_algorithm_v2",
  };
  return {
    schema_version: 2,
    document_id: "result.document:semantic-export",
    title: "PLS result",
    provenance: {
      run_id: "run-semantic-export",
      project_id: "project-semantic-export",
      model_id: "model-semantic-export",
      model_digest: digest,
      dataset_id: "dataset-semantic-export",
      dataset_fingerprint: digest,
      recipe_id: "recipe-semantic-export",
      recipe_digest: digest,
      capability_cell: capability,
      method_version: "pls_algorithm_v2",
      engine_version: "qpls-estimation-test",
      seed: 42,
      workers: 2,
      started_at: "2026-08-14T10:00:00Z",
      completed_at: "2026-08-14T10:00:01Z",
    },
    capability_cells: [capability],
    sections: [
      {
        id: "structural",
        title: "Structural model",
        table_ids: ["paths", "decisions"],
        chart_ids: ["path_plot"],
        capability_cells: [capability],
      },
    ],
    tables: [
      {
        id: "paths",
        title: "Path coefficients",
        columns: [
          { id: "path", label: "Path", data_type: "text", description: "Directed path", role: "label" },
          { id: "estimate", label: "Estimate", data_type: "number", description: "Path estimate", role: "estimate", default_precision: 4 },
          { id: "p_value", label: "p", data_type: "number", description: "Two-sided p-value", role: "uncertainty", default_precision: 4 },
        ],
        rows: [
          {
            id: "x_to_y",
            cells: [
              { kind: "text", value: "X to Y" },
              { kind: "number", value: 0.42, display: "0.4200" },
              { kind: "missing", reason: "not_estimated", display: "Not estimated" },
            ],
          },
          {
            id: "z_to_y",
            cells: [
              { kind: "text", value: "Z to Y" },
              { kind: "number", value: -0.15 },
              { kind: "number", value: 0.032 },
            ],
          },
        ],
        footnote_ids: ["standardized"],
        capability_cells: [capability],
      },
      {
        id: "decisions",
        title: "Decisions",
        columns: [
          { id: "test", label: "Test", data_type: "text", description: "Decision name", role: "label" },
          { id: "supported", label: "Supported", data_type: "boolean", description: "Decision outcome", role: "decision" },
        ],
        rows: [{ id: "x_to_y_supported", cells: [{ kind: "text", value: "X to Y" }, { kind: "boolean", value: true }] }],
        footnote_ids: [],
        capability_cells: [capability],
      },
    ],
    charts: [
      {
        id: "path_plot",
        title: "Path estimates",
        description: "Two bars showing the path estimates.",
        kind: "bar",
        series: [{
          id: "estimate",
          label: "Estimate",
          points: [
            { x: "X to Y", y: 0.42, label: "X to Y" },
            { x: "Z to Y", y: -0.15, lower: -0.28, upper: -0.02 },
          ],
        }],
        source_table_id: "paths",
        display: { palette: "journal_mono", show_legend: true, show_values: true },
      },
    ],
    notices: [{
      id: "small_sample",
      code: "small_sample",
      severity: "warning",
      message: "Interpret uncertainty carefully for this sample size.",
      section_ids: ["structural"],
      table_ids: ["paths"],
    }],
    exclusions: [{
      id: "not_requested",
      title: "Additional inference not requested",
      reason: "The saved recipe did not request it.",
    }],
    footnotes: [{ id: "standardized", text: "Standardized estimates." }],
    presentation: {
      default_section_id: "structural",
      default_table_id: "paths",
      precision: 4,
      missing_value_label: "Not available",
      chart_defaults: { show_legend: true },
    },
  };
}

describe("CanonicalResultDocumentV2 semantic export", () => {
  it("projects ordered typed tables, notices, provenance, and chart data without method payload access", () => {
    const source = resultDocument();
    const built = buildCanonicalResultSemanticExportV2(source);

    expect(built.ok).toBe(true);
    if (!built.ok) return;
    expect(built.projection.ordering).toEqual({
      sections: [{ section_id: "structural", table_ids: ["paths", "decisions"], chart_ids: ["path_plot"] }],
      tables: [
        { table_id: "paths", column_ids: ["path", "estimate", "p_value"], row_ids: ["x_to_y", "z_to_y"] },
        { table_id: "decisions", column_ids: ["test", "supported"], row_ids: ["x_to_y_supported"] },
      ],
      charts: [{ chart_id: "path_plot", series: [{ series_id: "estimate", point_count: 2 }] }],
      notice_ids: ["small_sample"],
      exclusion_ids: ["not_requested"],
      footnote_ids: ["standardized"],
    });
    expect(built.projection.tables[0].rows[0].cells[2]).toEqual({
      kind: "missing",
      reason: "not_estimated",
      display: "Not estimated",
    });
    expect(built.projection.tables[1].rows[0].cells[1]).toEqual({ kind: "boolean", value: true });
    expect(built.projection.notices[0]).toMatchObject({ severity: "warning", table_ids: ["paths"] });
    expect(built.projection.provenance).toEqual(source.provenance);
    expect(built.projection.charts[0].series[0].points).toEqual(source.charts[0].series[0].points);
  });

  it("serializes deterministically and verifies exact semantic readback", () => {
    const source = resultDocument();
    const first = buildCanonicalResultSemanticExportV2(source);
    const reordered = structuredClone(source);
    reordered.presentation = {
      chart_defaults: { show_legend: true },
      missing_value_label: "Not available",
      precision: 4,
      default_table_id: "paths",
      default_section_id: "structural",
    };
    const second = buildCanonicalResultSemanticExportV2(reordered);
    expect(first.ok && second.ok).toBe(true);
    if (!first.ok || !second.ok) return;

    const json = canonicalResultSemanticExportJsonV2(first.projection);
    expect(canonicalResultSemanticExportJsonV2(second.projection)).toBe(json);
    expect(parseCanonicalResultSemanticExportJsonV2(json)).toMatchObject({
      ok: true,
      document: { document_id: source.document_id },
    });
    expect(verifyCanonicalResultSemanticExportReadbackV2(source, json)).toMatchObject({
      passed: true,
      exact_document_match: true,
      analytical_match: true,
      errors: [],
    });
  });

  it("rejects reordered indexes, changed cell types, and unexpected fields", () => {
    const source = resultDocument();
    const built = buildCanonicalResultSemanticExportV2(source);
    expect(built.ok).toBe(true);
    if (!built.ok) return;

    const reordered = structuredClone(built.projection);
    reordered.ordering.tables[0].row_ids.reverse();
    expect(parseCanonicalResultSemanticExportJsonV2(JSON.stringify(reordered))).toMatchObject({
      ok: false,
      code: "invalid_export",
    });

    const changedType = structuredClone(built.projection);
    changedType.tables[0].rows[0].cells[1] = { kind: "text", value: "0.42" };
    expect(parseCanonicalResultSemanticExportJsonV2(JSON.stringify(changedType))).toMatchObject({
      ok: false,
      code: "invalid_export",
      errors: [expect.stringContaining("expected number or missing")],
    });

    const unexpected = JSON.parse(canonicalResultSemanticExportJsonV2(built.projection)) as Record<string, unknown>;
    unexpected.unexpected = true;
    expect(parseCanonicalResultSemanticExportJsonV2(JSON.stringify(unexpected))).toMatchObject({
      ok: false,
      code: "invalid_export",
      errors: [expect.stringContaining("fields or ordering")],
    });
  });

  it("fails closed for an invalid canonical source or malformed JSON", () => {
    const invalid = resultDocument();
    invalid.tables[0].rows[0].cells[1] = { kind: "number", value: Number.NaN };
    expect(buildCanonicalResultSemanticExportV2(invalid)).toMatchObject({
      ok: false,
      code: "invalid_source_document",
      errors: [expect.stringContaining("must be finite")],
    });
    expect(parseCanonicalResultSemanticExportJsonV2("{")) .toEqual({
      ok: false,
      code: "invalid_json",
      errors: ["The export is not valid JSON."],
    });
  });
});
