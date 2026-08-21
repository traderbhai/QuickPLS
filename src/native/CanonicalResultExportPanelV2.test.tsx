import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2 } from "../domain/canonicalResultCrossFormatExportV2";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import { CanonicalResultExportPanelV2 } from "./CanonicalResultExportPanelV2";

function documentWithChart(): CanonicalResultDocumentV2 {
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.moderation",
    cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    capability_version: "general_sem_pls_multiple_two_way_point_v1",
  };
  return {
    schema_version: 2,
    document_id: "result.general-sem:export-panel",
    title: "General SEM result",
    provenance: {
      run_id: "run:export-panel",
      project_id: "project:export-panel",
      model_id: "model:export-panel",
      model_digest: "a".repeat(64),
      dataset_id: "dataset:export-panel",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "recipe:export-panel",
      recipe_digest: "c".repeat(64),
      capability_cell: capability,
      method_version: "qpls.general-sem-pls.multiple-two-way.point.v1",
      engine_version: "qpls-estimation-test",
      seed: 7,
      workers: 1,
      started_at: "2026-08-19T10:00:00Z",
      completed_at: "2026-08-19T10:00:01Z",
    },
    capability_cells: [capability],
    sections: [{ id: "results", title: "Results", table_ids: ["effects", "scope"], chart_ids: ["plot"], capability_cells: [capability] }],
    tables: [
      {
        id: "effects",
        title: "Effects",
        columns: [{ id: "estimate", label: "Estimate", data_type: "number", description: "Estimate." }],
        rows: [{ id: "gamma", cells: [{ kind: "number", value: 0.2 }] }],
        footnote_ids: [],
        capability_cells: [capability],
      },
      {
        id: "scope",
        title: "Scope",
        columns: [{ id: "boundary", label: "Boundary", data_type: "text", description: "Boundary." }],
        rows: [{ id: "point", cells: [{ kind: "text", value: "Point only" }] }],
        footnote_ids: [],
        capability_cells: [capability],
      },
    ],
    charts: [{ id: "plot", title: "Interaction plot", description: "Persisted interaction plot.", kind: "line", series: [{ id: "low", label: "Low", points: [{ x: -1, y: 0.1 }, { x: 1, y: 0.3 }] }], source_table_id: "effects", display: {} }],
    notices: [], exclusions: [], footnotes: [],
    presentation: { default_section_id: "results", default_table_id: "effects", precision: 4, missing_value_label: "—", chart_defaults: {} },
  };
}

describe("CanonicalResultExportPanelV2 accessibility", () => {
  it("exposes keyboard-native table selection, chart selection, six formats, and live feedback semantics", () => {
    const html = renderToStaticMarkup(<CanonicalResultExportPanelV2
      document={documentWithChart()}
      nativeDesktop
      writers={{
        text: vi.fn(),
        workbook: vi.fn(),
        binary: vi.fn(),
      }}
    />);

    expect(html).toContain('aria-labelledby="nd-canonical-export-v2-heading"');
    expect(html).toContain('<fieldset');
    expect(html).toContain('<legend>Tables to include</legend>');
    expect(html).toMatch(/<label[^>]+for="nd-canonical-export-table-[^"]+"/u);
    expect(html).toContain('type="checkbox"');
    expect(html).toContain('id="nd-canonical-export-v2-chart"');
    expect(html).toContain('for="nd-canonical-export-v2-chart"');
    expect(html).toContain('aria-label="Canonical result export formats"');
    for (const format of ["CSV", "XLSX", "HTML", "PDF", "SVG", "PNG"]) {
      expect(html).toContain(`Export ${format}`);
    }
    expect(html).toContain('id="nd-canonical-export-v2-selection-summary"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toContain("2 of 2 canonical tables selected");
    expect(html).not.toContain('role="button"');
  });

  it("explains unavailable chart and browser-only XLSX actions instead of exposing dead controls", () => {
    const document = documentWithChart();
    document.charts = [];
    document.sections[0]!.chart_ids = [];
    const html = renderToStaticMarkup(<CanonicalResultExportPanelV2 document={document} nativeDesktop={false} />);

    expect(html).toContain("contains no persisted chart");
    expect(html).toMatch(/<button(?=[^>]*disabled="")(?=[^>]*title="XLSX workbook publication requires the QuickPLS desktop runtime\.")[^>]*>[^<]*(?:<svg[\s\S]*?<\/svg>)?Export XLSX<\/button>/u);
    expect(html).toMatch(/<button(?=[^>]*disabled="")(?=[^>]*title="Select a canonical export chart first\.")[^>]*>[\s\S]*?Export SVG<\/button>/u);
    expect(html).toMatch(/<button(?=[^>]*disabled="")(?=[^>]*title="Select a canonical export chart first\.")[^>]*>[\s\S]*?Export PNG<\/button>/u);
  });

  it("hides stable table, chart and fallback point IDs from the researcher-facing panel", () => {
    const html = renderToStaticMarkup(<CanonicalResultExportPanelV2
      document={documentWithChart()}
      nativeDesktop
      researcherFacing
    />);

    expect(html).toContain("Export verified result");
    expect(html).toContain("2 of 2 result tables selected");
    expect(html).not.toContain("Stable table ID:");
    expect(html).not.toContain("Interaction plot · saved · plot");
    expect(html).toContain("Point 1");
    expect(html).toContain("<summary>Export identity</summary>");
    expect(html).toContain("Selected table IDs</dt><dd>effects, scope");
    expect(html).toContain("Selected chart ID</dt><dd>plot");
  });

  it("exposes a keyboard-reachable accessible preview and exact table fallback for mediation-derived charts", () => {
    const document = documentWithChart();
    const capability = {
      registry_schema_version: 2 as const,
      capability_id: "smartpls.mediation",
      cell_id: "qpls3.pls.mediation",
      capability_version: "pls_mediation_v1",
    };
    document.title = "PLS-SEM mediation effects";
    document.provenance.capability_cell = capability;
    document.provenance.method_version = "general_sem_effects_v1";
    document.capability_cells = [capability];
    document.charts = [];
    document.tables = [{
      id: "general_sem_specific_indirect_effects",
      title: "Specific indirect effects",
      columns: [
        { id: "effect_id", label: "Effect ID", data_type: "text", description: "Stable effect identity." },
        { id: "estimate", label: "Estimate", data_type: "number", description: "PLS point estimate." },
      ],
      rows: [{ id: "specific_indirect_0000", cells: [{ kind: "text", value: "effect:x_m_y" }, { kind: "number", value: 0.18 }] }],
      footnote_ids: [],
      capability_cells: [capability],
    }];
    document.sections = [{
      id: "general_sem_effects",
      title: "Mediation effects",
      table_ids: ["general_sem_specific_indirect_effects"],
      chart_ids: [],
      capability_cells: [capability],
    }];
    document.presentation.default_section_id = "general_sem_effects";
    document.presentation.default_table_id = "general_sem_specific_indirect_effects";

    const html = renderToStaticMarkup(<CanonicalResultExportPanelV2 document={document} nativeDesktop />);
    expect(html).toContain(`value="${CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2}"`);
    expect(html).toContain("derived from canonical table");
    expect(html).toMatch(new RegExp(`<figure(?=[^>]*class="nd-canonical-chart")(?=[^>]*role="img")(?=[^>]*tabindex="0")(?=[^>]*data-canonical-chart-id="${CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2}")[^>]*>`, "u"));
    expect(html).toContain("Derived visual only; the resident canonical result and its scientific identities are unchanged.");
    expect(html).toContain(`data-canonical-chart-table-fallback="${CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2}"`);
    expect(html).toContain("Exact table fallback in stable chart order.");
    expect(html).toContain("effect:x_m_y");
    expect(html).toMatch(/<button(?=[^>]*(?!disabled=""))[^>]*>[\s\S]*?Export SVG<\/button>/u);
    expect(html).toMatch(/<button(?=[^>]*(?!disabled=""))[^>]*>[\s\S]*?Export PNG<\/button>/u);
  });

  it("aligns repeated line-series coordinates on the persisted x domain instead of ordinal point indexes", () => {
    const document = documentWithChart();
    document.charts[0]!.display.x_axis_label = "W";
    document.charts[0]!.series = [-1, 0, 1].map((z, seriesIndex) => ({
      id: `z-${seriesIndex}`,
      label: `Z = ${z}`,
      points: [-1, 0, 1].map((w) => ({ x: w, y: 0.2 + (seriesIndex + 1) * (w + 2) * 0.1 })),
    }));

    const html = renderToStaticMarkup(<CanonicalResultExportPanelV2 document={document} nativeDesktop />);
    const ticks = [...html.matchAll(/<text class="nd-canonical-chart__tick"[^>]*>([^<]+)<\/text>/gu)]
      .map((match) => match[1]);
    const pointCoordinates = [...html.matchAll(/<circle class="nd-canonical-chart__point" cx="([^"]+)"/gu)]
      .map((match) => match[1]);

    expect(ticks).toEqual(["-1", "0", "1"]);
    expect(pointCoordinates).toHaveLength(9);
    expect(pointCoordinates.slice(0, 3)).toEqual(pointCoordinates.slice(3, 6));
    expect(pointCoordinates.slice(0, 3)).toEqual(pointCoordinates.slice(6, 9));
    expect(html).toContain("<td>-1</td>");
    expect(html).toContain("<td>0</td>");
    expect(html).toContain("<td>1</td>");

    document.charts[0]!.kind = "bar";
    const barHtml = renderToStaticMarkup(<CanonicalResultExportPanelV2 document={document} nativeDesktop />);
    const barTicks = [...barHtml.matchAll(/<text class="nd-canonical-chart__tick"[^>]*>([^<]+)<\/text>/gu)]
      .map((match) => match[1]);
    expect(barTicks).toEqual(["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
  });

  it("is the only General SEM export UI seam and replaces the method-specific XLSX handler", () => {
    const source = readFileSync("src/native/NativeRecipeV4GeneralSemWorkspace.tsx", "utf8");
    const panel = readFileSync("src/native/CanonicalResultExportPanelV2.tsx", "utf8");
    expect(source).toContain("<CanonicalResultExportPanelV2 document={displayedDocument}");
    expect(source).not.toContain("writers={{");
    expect(source).not.toContain("const exportDisplayed = async");
    expect(source).not.toContain("canonicalResultDocumentV2ExportTables(displayedDocument)");
    expect(source).not.toContain(">Export XLSX</button>");
    expect(panel).toContain("publishNativeCanonicalResultExportV2(artifact, signal)");
    expect(panel).toContain("dispatchCanonicalResultExportV2(document");
    expect(panel).not.toContain("exportNativeTextFile");
    expect(panel).not.toContain("exportNativeXlsxTables");
  });
});
