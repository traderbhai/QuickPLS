import { describe, expect, it, vi } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import {
  CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID,
  CANONICAL_RESULT_DERIVED_HIGHER_ORDER_TARGET_CHART_ID_V2,
  canonicalResultExportChartsV2,
  dispatchCanonicalResultExportV2,
  prepareCanonicalResultExportV2,
  readPreparedCanonicalResultExportSemanticV2,
  verifyPreparedCanonicalResultExportV2,
  type CanonicalResultExportRequestV2,
  type PreparedCanonicalResultExportV2,
} from "./canonicalResultCrossFormatExportV2";
import { sha256HexUtf8V1 } from "./sha256V1";

const digest = "a".repeat(64);

function fixture(): CanonicalResultDocumentV2 {
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.moderation",
    cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    capability_version: "general_sem_pls_multiple_two_way_point_v1",
  };
  return {
    schema_version: 2,
    document_id: "result.general-sem:export-v2",
    title: "General SEM moderation",
    provenance: {
      run_id: "run:general-sem-export-v2",
      project_id: "project:general-sem-export-v2",
      model_id: "model:general-sem-export-v2",
      model_digest: digest,
      dataset_id: "dataset:general-sem-export-v2",
      dataset_fingerprint: digest,
      recipe_id: "recipe:general-sem-export-v2",
      recipe_digest: "b".repeat(64),
      capability_cell: capability,
      method_version: "qpls.general-sem-pls.multiple-two-way.point.v1",
      engine_version: "qpls-estimation-test",
      seed: 42,
      workers: 2,
      started_at: "2026-08-19T10:00:00Z",
      completed_at: "2026-08-19T10:00:01Z",
    },
    capability_cells: [capability],
    sections: [{
      id: "moderation",
      title: "Moderation",
      table_ids: ["effects", "scope"],
      chart_ids: ["interaction_plot"],
      capability_cells: [capability],
    }],
    tables: [
      {
        id: "effects",
        title: "Interaction effects",
        description: "Persisted interaction estimates.",
        columns: [
          { id: "effect", label: "Effect", data_type: "text", description: "Stable effect label.", role: "label" },
          { id: "estimate", label: "Estimate", data_type: "number", description: "Scientific gamma.", role: "estimate", default_precision: 4 },
          { id: "reported", label: "Reported", data_type: "boolean", description: "Whether the effect is reported.", role: "decision" },
        ],
        rows: [
          { id: "gamma_x_w", cells: [{ kind: "text", value: "X × W" }, { kind: "number", value: 0.375 }, { kind: "boolean", value: true }] },
          { id: "formula_guard", cells: [{ kind: "text", value: "=SUM(A1:A2)" }, { kind: "missing", reason: "not_estimated" }, { kind: "boolean", value: false }] },
        ],
        footnote_ids: ["point_only"],
        capability_cells: [capability],
      },
      {
        id: "scope",
        title: "Method scope",
        columns: [{ id: "boundary", label: "Boundary", data_type: "text", description: "Method boundary.", role: "diagnostic" }],
        rows: [{ id: "point", cells: [{ kind: "text", value: "Point estimates only" }] }],
        footnote_ids: [],
        capability_cells: [capability],
      },
    ],
    charts: [{
      id: "interaction_plot",
      title: "Interaction plot",
      description: "Persisted conditional relationship at two moderator levels.",
      kind: "line",
      series: [
        { id: "w_minus_1", label: "W = -1", points: [{ x: -1, y: 0.1 }, { x: 1, y: 0.4 }] },
        { id: "w_plus_1", label: "W = +1", points: [{ x: -1, y: -0.2, lower: -0.3, upper: -0.1 }, { x: 1, y: 0.7, lower: 0.6, upper: 0.8 }] },
      ],
      source_table_id: "effects",
      display: { show_legend: true, x_axis_label: "X", y_axis_label: "Y" },
    }],
    notices: [{
      id: "point_only_notice",
      code: "point_only",
      severity: "information",
      message: "Plots remain point-only.",
      section_ids: ["moderation"],
      table_ids: ["effects"],
    }],
    exclusions: [{ id: "no_bands", title: "Chart bands excluded", reason: "The point method does not estimate chart bands." }],
    footnotes: [{ id: "point_only", text: "Scientific gamma is a point estimate." }],
    presentation: {
      default_section_id: "moderation",
      default_table_id: "effects",
      precision: 4,
      missing_value_label: "Not estimated",
      chart_defaults: { show_legend: true },
    },
  };
}

function candidateMultimodFixture(): CanonicalResultDocumentV2 {
  const document = fixture();
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.mediation",
    cell_id: "qpls.multimod.general_sem_conditional_process_v2",
    capability_version: "general_sem_conditional_process_v2",
  };
  const receipt = {
    schema_version: 1 as const,
    authority_binding_sha256: "1".repeat(64),
    candidate_commit_sha: "2".repeat(40),
    candidate_version: "2.56.0",
    qualification_plan_sha256: "3".repeat(64),
    gate_binding_sha256: "4".repeat(64),
    capability_index_sha256: "5".repeat(64),
    prepackage_manifest_set_sha256: "6".repeat(64),
    required_profile_cells: [
      "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
    ],
  };
  document.provenance.capability_cell = capability;
  document.provenance.method_version = "general_sem_conditional_process_v2";
  document.capability_cells = [capability];
  for (const section of document.sections) section.capability_cells = [capability];
  for (const table of document.tables) table.capability_cells = [capability];
  document.sections[0]!.table_ids.push("multimod_run_provenance");
  document.tables.push({
    id: "multimod_run_provenance",
    title: "MultiMod run provenance",
    columns: [
      {
        id: "qualification",
        label: "Qualification",
        data_type: "text",
        description: "Typed qualification state.",
        role: "decision",
      },
      {
        id: "candidate_qualification_receipt_json",
        label: "Candidate authority receipt",
        data_type: "text",
        description: "Exact build-embedded authority receipt.",
        role: "provenance",
      },
    ],
    rows: [{
      id: "run",
      cells: [
        { kind: "text", value: "release_qualified_candidate" },
        { kind: "text", value: JSON.stringify(receipt) },
      ],
    }],
    footnote_ids: [],
    capability_cells: [capability],
  });
  return document;
}

function higherOrderFixture(): CanonicalResultDocumentV2 {
  const document = fixture();
  const capability = {
    registry_schema_version: 2 as const,
    capability_id: "smartpls.higher_order_models",
    cell_id: "qpls3.pls.general_sem_higher_order_point",
    capability_version: "general_sem_pls_higher_order_point_v1",
  };
  document.document_id = "result.general-sem:higher-order-export-v2";
  document.title = "General SEM higher-order point estimates";
  document.provenance.capability_cell = capability;
  document.provenance.method_version = "general_sem_pls_higher_order_point_v1";
  document.capability_cells = [capability];
  document.sections = [{
    id: "general_sem_higher_order",
    title: "Higher-order construct targets",
    table_ids: ["general_sem_higher_order_targets"],
    chart_ids: [],
    capability_cells: [capability],
  }];
  document.tables = [{
    id: "general_sem_higher_order_targets",
    title: "Higher-order component and structural targets",
    columns: [
      { id: "relation_id", label: "Relation ID", data_type: "text", description: "Stable relation identity.", role: "label" },
      { id: "estimate", label: "Estimate", data_type: "number", description: "Point estimate.", role: "estimate", default_precision: 4 },
    ],
    rows: [
      { id: "higher_order_target_0000", cells: [{ kind: "text", value: "hoc_component:quality" }, { kind: "number", value: 0.82 }] },
      { id: "higher_order_target_0001", cells: [{ kind: "text", value: "hoc_path:quality_to_loyalty" }, { kind: "number", value: 0.44 }] },
    ],
    footnote_ids: [],
    capability_cells: [capability],
  }];
  document.charts = [];
  document.notices = [];
  document.exclusions = [];
  document.footnotes = [];
  document.presentation.default_section_id = "general_sem_higher_order";
  document.presentation.default_table_id = "general_sem_higher_order_targets";
  return document;
}

function largePreparationFixture(): CanonicalResultDocumentV2 {
  const document = fixture();
  const table = document.tables[0]!;
  table.rows = Array.from({ length: 6_000 }, (_, index) => ({
    id: `effect_${index}`,
    cells: [
      { kind: "text" as const, value: `X × W ${index}` },
      { kind: "number" as const, value: index / 10_000 },
      { kind: "boolean" as const, value: index % 2 === 0 },
    ],
  }));
  document.charts[0]!.series[0]!.points = Array.from({ length: 6_000 }, (_, index) => ({
    x: index,
    y: index / 10_000,
  }));
  return document;
}

function prepared(request: CanonicalResultExportRequestV2): PreparedCanonicalResultExportV2 {
  const result = prepareCanonicalResultExportV2(fixture(), request);
  expect(result.ok).toBe(true);
  if (!result.ok) throw new Error(result.errors.join("\n"));
  return result.artifact;
}

describe("CanonicalResultDocumentV2 cross-format dispatcher", () => {
  it("preserves the frozen pre-MultiMod V2 envelope key order and hashes for legacy results", () => {
    const artifact = prepared({ format: "csv", tableIds: ["effects"], chartIds: [] });
    if (artifact.format !== "csv") throw new Error("Expected CSV artifact");

    expect(Object.keys(artifact.semantic)).toEqual([
      "schema_version",
      "format",
      "source",
      "title",
      "provenance",
      "capability_cells",
      "selection",
      "sections",
      "tables",
      "charts",
      "notices",
      "exclusions",
      "footnotes",
      "presentation",
      "semantic_sha256",
    ]);
    expect(artifact.semantic).not.toHaveProperty("publication_qualification");
    expect(artifact.semantic).not.toHaveProperty("candidate_qualification_receipt");
    // Frozen from the identical fixture under main commit 6ed46cc422917fc9fc9c463302ca4ff1e9ea01a4.
    expect(artifact.semantic.semantic_sha256).toBe(
      "ec56a5b3d17a2cea1b857780adfde39ec5e779082a9de7d500e4d7487b6dc90a",
    );
    expect(sha256HexUtf8V1(artifact.contents)).toBe(
      "749ce66ad38c9b38adafe05b8f5da48f28f4e857e2f4ab05fcee5214c78084f0",
    );
  });

  it("retains candidate authority outside a table selection for native export verification", () => {
    const result = prepareCanonicalResultExportV2(candidateMultimodFixture(), {
      format: "json",
      tableIds: ["effects"],
      chartIds: [],
    });
    expect(result.ok).toBe(true);
    if (!result.ok) throw new Error(result.errors.join("\n"));
    expect(result.artifact.semantic.tables.map((table) => table.id)).toEqual(["effects"]);
    expect(result.artifact.semantic.publication_qualification).toBe("release_qualified_candidate");
    expect(result.artifact.semantic.candidate_qualification_receipt).toMatchObject({
      candidate_commit_sha: "2".repeat(40),
      required_profile_cells: [
        "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
      ],
    });
  });

  it("derives the HOC SVG and PNG chart from exact canonical relation IDs and estimates", () => {
    const document = higherOrderFixture();
    const charts = canonicalResultExportChartsV2(document);
    expect(charts).toEqual([{
      origin: "derived_from_canonical_table",
      chart: expect.objectContaining({
        id: CANONICAL_RESULT_DERIVED_HIGHER_ORDER_TARGET_CHART_ID_V2,
        source_table_id: "general_sem_higher_order_targets",
        series: [{
          id: "estimate",
          label: "Estimate",
          points: [
            { x: 1, y: 0.82, label: "hoc_component:quality" },
            { x: 2, y: 0.44, label: "hoc_path:quality_to_loyalty" },
          ],
        }],
      }),
    }]);
    for (const format of ["svg", "png"] as const) {
      const prepared = prepareCanonicalResultExportV2(document, {
        format,
        tableIds: [],
        chartIds: [CANONICAL_RESULT_DERIVED_HIGHER_ORDER_TARGET_CHART_ID_V2],
      });
      expect(prepared).toMatchObject({ ok: true });
      if (prepared.ok) expect(verifyPreparedCanonicalResultExportV2(document, prepared.artifact).passed).toBe(true);
    }
  });

  it("round-trips one selected table through deterministic CSV with formula protection and stable IDs", () => {
    const artifact = prepared({ format: "csv", tableIds: ["effects"] });
    if (artifact.format !== "csv") throw new Error("Expected CSV artifact");

    expect(artifact.contents).toContain(`${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID},`);
    expect(artifact.contents).toContain("table_id,effects");
    expect(artifact.contents).toContain("column_ids,row_id,effect,estimate,reported");
    expect(artifact.contents).toContain("row,gamma_x_w,X × W,0.375,true");
    expect(artifact.contents).toContain("'=SUM(A1:A2)");
    expect(artifact.contents).not.toContain("table_id,scope");
    expect(verifyPreparedCanonicalResultExportV2(fixture(), artifact)).toMatchObject({
      passed: true,
      exact_semantic_match: true,
      digest_match: true,
    });
  });

  it("round-trips selected tables and charts through provenance-bound semantic JSON", () => {
    const artifact = prepared({
      format: "json",
      tableIds: ["effects"],
      chartIds: ["interaction_plot"],
    });
    if (artifact.format !== "json") throw new Error("Expected JSON artifact");

    const decoded = JSON.parse(artifact.contents) as Record<string, unknown>;
    expect(decoded.semantic_sha256).toMatch(/^[a-f0-9]{64}$/u);
    expect(readPreparedCanonicalResultExportSemanticV2(artifact)?.selection).toEqual({
      table_ids: ["effects"],
      chart_ids: ["interaction_plot"],
    });
    expect(verifyPreparedCanonicalResultExportV2(fixture(), artifact).passed).toBe(true);
  });

  it("builds a multisheet workbook model with an exact manifest, stable sheet IDs, and provenance", () => {
    const artifact = prepared({ format: "xlsx", tableIds: ["scope", "effects"] });
    if (artifact.format !== "xlsx") throw new Error("Expected workbook artifact");

    expect(artifact.workbookTables.map((table) => table.id)).toEqual([
      "quickpls_export_manifest_v2",
      "effects",
      "scope",
      "quickpls_export_provenance_v2",
    ]);
    expect(artifact.workbookTables[1]?.title).toBe("effects");
    expect(artifact.workbookTables[1]?.columns).toEqual(["Row ID", "Effect [effect]", "Estimate [estimate]", "Reported [reported]"]);
    expect(artifact.workbookTables.at(-1)?.rows).toContainEqual(["dataset_fingerprint", digest]);
    expect(artifact.workbookTables.flatMap((table) => table.rows.flat()).every((cell) => cell.length <= 30_000)).toBe(true);
    expect(verifyPreparedCanonicalResultExportV2(fixture(), artifact).passed).toBe(true);
  });

  it("creates one self-contained accessible HTML report with tables, inline chart, and semantic readback", () => {
    const artifact = prepared({ format: "html", tableIds: ["effects"], chartIds: ["interaction_plot"] });
    if (artifact.format !== "html") throw new Error("Expected HTML artifact");

    expect(artifact.contents).toContain("<!doctype html>");
    expect(artifact.contents).toContain("Content-Security-Policy");
    expect(artifact.contents).toContain('data-canonical-table-id="effects"');
    expect(artifact.contents).toContain('data-canonical-row-id="gamma_x_w"');
    expect(artifact.contents).toContain('data-canonical-chart-id="interaction_plot"');
    expect(artifact.contents).toContain('role="img"');
    expect(artifact.contents).not.toMatch(/<(?:link|script)[^>]+src=/u);
    expect(verifyPreparedCanonicalResultExportV2(fixture(), artifact).passed).toBe(true);
  });

  it("creates standalone accessible SVG and valid PNG chart artifacts with the same chart-only semantics", () => {
    const svg = prepared({ format: "svg", tableIds: [], chartIds: ["interaction_plot"] });
    const png = prepared({ format: "png", tableIds: [], chartIds: ["interaction_plot"] });
    if (svg.format !== "svg" || png.format !== "png") throw new Error("Expected chart artifacts");

    expect(svg.contents).toContain('<svg xmlns="http://www.w3.org/2000/svg"');
    expect(svg.contents).toContain('role="img"');
    expect(svg.contents).toContain('<title id="chart-title">Interaction plot</title>');
    expect(svg.contents).toContain('data-canonical-series-id="w_minus_1"');
    expect(svg.contents).toContain('data-canonical-legend-series-id="w_minus_1"');
    expect(svg.contents).toContain('>W = -1</text>');
    expect(svg.contents).toContain('>Y: Y</text>');
    expect(svg.semantic.tables).toEqual([]);
    expect([...png.bytes.slice(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
    expect(new TextDecoder().decode(png.bytes.slice(-12, -4))).toContain("IEND");
    expect(png.bytes.length).toBeGreaterThan(100_000);
    expect(readPreparedCanonicalResultExportSemanticV2(svg)?.selection).toEqual({ table_ids: [], chart_ids: ["interaction_plot"] });
    expect(readPreparedCanonicalResultExportSemanticV2(png)?.selection).toEqual({ table_ids: [], chart_ids: ["interaction_plot"] });
    expect(verifyPreparedCanonicalResultExportV2(fixture(), svg).passed).toBe(true);
    expect(verifyPreparedCanonicalResultExportV2(fixture(), png).passed).toBe(true);
  });

  it("creates a paginated PDF report with provenance, stable IDs, and exact embedded semantic readback", () => {
    const artifact = prepared({ format: "pdf", tableIds: ["scope"], chartIds: ["interaction_plot"] });
    if (artifact.format !== "pdf") throw new Error("Expected PDF artifact");
    const text = new TextDecoder().decode(artifact.bytes);

    expect(text.startsWith("%PDF-1.4")).toBe(true);
    expect(text).toContain("%QPLS-CANONICAL-SEMANTIC-V2-BEGIN");
    expect(text).toContain("[table:scope] Method scope");
    expect(text).toContain("[chart:interaction_plot] Interaction plot");
    expect(text).toContain("series:w_minus_1 W = -1");
    expect(text).not.toContain("?) Tj");
    expect(text.endsWith("%%EOF\n")).toBe(true);
    expect(verifyPreparedCanonicalResultExportV2(fixture(), artifact).passed).toBe(true);
  });

  it("fails closed before publication when PDF or PNG cannot render an exact canonical glyph", async () => {
    const pdfWriter = vi.fn(async () => "D:/exports/result.pdf");
    const pdf = await dispatchCanonicalResultExportV2(
      fixture(),
      { format: "pdf", tableIds: ["effects"], chartIds: [] },
      { binary: pdfWriter },
    );
    expect(pdf).toMatchObject({
      status: "failed",
      format: "pdf",
      message: expect.stringContaining('"×" (U+00D7)'),
    });
    expect(pdfWriter).not.toHaveBeenCalled();

    const unicodeChart = fixture();
    unicodeChart.charts[0]!.title = "Interacción plot";
    const pngWriter = vi.fn(async () => "D:/exports/result.png");
    const png = await dispatchCanonicalResultExportV2(
      unicodeChart,
      { format: "png", tableIds: [], chartIds: ["interaction_plot"] },
      { binary: pngWriter },
    );
    expect(png).toMatchObject({
      status: "failed",
      format: "png",
      message: expect.stringContaining('"ó" (U+00F3)'),
    });
    expect(pngWriter).not.toHaveBeenCalled();
  });

  it("preserves UTF-8 SVG labels and renders lowercase PNG labels without uppercasing substitution", () => {
    const unicodeChart = fixture();
    unicodeChart.charts[0]!.title = "Interacción plot";
    unicodeChart.charts[0]!.series[0]!.label = "Nivel bajo";
    const svg = prepareCanonicalResultExportV2(unicodeChart, {
      format: "svg",
      tableIds: [],
      chartIds: ["interaction_plot"],
    });
    expect(svg.ok).toBe(true);
    if (!svg.ok || svg.artifact.format !== "svg") throw new Error("Expected SVG artifact");
    expect(svg.artifact.contents).toContain("Interacción plot");
    expect(svg.artifact.contents).toContain(">Nivel bajo</text>");

    const lowercase = prepared({ format: "png", tableIds: [], chartIds: ["interaction_plot"] });
    const uppercaseDocument = fixture();
    uppercaseDocument.charts[0]!.title = "INTERACTION PLOT";
    const uppercase = prepareCanonicalResultExportV2(uppercaseDocument, {
      format: "png",
      tableIds: [],
      chartIds: ["interaction_plot"],
    });
    expect(uppercase.ok).toBe(true);
    if (lowercase.format !== "png" || !uppercase.ok || uppercase.artifact.format !== "png") throw new Error("Expected PNG artifacts");
    const idat = (bytes: Uint8Array) => {
      let offset = 8;
      while (offset + 12 <= bytes.length) {
        const length = (((bytes[offset] ?? 0) << 24) | ((bytes[offset + 1] ?? 0) << 16) | ((bytes[offset + 2] ?? 0) << 8) | (bytes[offset + 3] ?? 0)) >>> 0;
        const type = new TextDecoder().decode(bytes.slice(offset + 4, offset + 8));
        if (type === "IDAT") return bytes.slice(offset + 8, offset + 8 + length);
        offset += 12 + length;
      }
      throw new Error("Missing IDAT");
    };
    expect(idat(lowercase.bytes)).not.toEqual(idat(uppercase.artifact.bytes));
  });

  it("detects an altered embedded payload instead of trusting rendered values", () => {
    const artifact = prepared({ format: "csv", tableIds: ["effects"] });
    if (artifact.format !== "csv") throw new Error("Expected CSV artifact");
    const lines = artifact.contents.split("\r\n");
    const index = lines.findIndex((line) => line.startsWith(`${CANONICAL_RESULT_CROSS_FORMAT_EXPORT_V2_METADATA_ID},`));
    expect(index).toBeGreaterThan(0);
    const original = lines[index]!;
    lines[index] = `${original.slice(0, -1)}${original.endsWith("A") ? "B" : "A"}`;
    const tampered = { ...artifact, contents: lines.join("\r\n") };

    expect(verifyPreparedCanonicalResultExportV2(fixture(), tampered).passed).toBe(false);
  });

  it("detects a changed rendered cell even when the embedded semantic payload is untouched", () => {
    const artifact = prepared({ format: "csv", tableIds: ["effects"] });
    if (artifact.format !== "csv") throw new Error("Expected CSV artifact");
    const tampered = { ...artifact, contents: artifact.contents.replace("0.375,true", "0.376,true") };

    expect(verifyPreparedCanonicalResultExportV2(fixture(), tampered)).toMatchObject({
      passed: false,
      exact_semantic_match: true,
      digest_match: true,
      rendered_surface_match: false,
      errors: [expect.stringContaining("rendered export surface")],
    });
  });

  it("rejects unknown, duplicate, empty, and multi-chart selections before publication", () => {
    expect(prepareCanonicalResultExportV2(fixture(), { format: "csv", tableIds: [] })).toMatchObject({ ok: false, code: "invalid_selection" });
    expect(prepareCanonicalResultExportV2(fixture(), { format: "xlsx", tableIds: ["effects", "effects"] })).toMatchObject({ ok: false, code: "invalid_selection" });
    expect(prepareCanonicalResultExportV2(fixture(), { format: "html", tableIds: ["unknown"] })).toMatchObject({ ok: false, code: "invalid_selection" });
    expect(prepareCanonicalResultExportV2(fixture(), { format: "png", tableIds: [], chartIds: [] })).toMatchObject({ ok: false, code: "invalid_selection" });
  });

  it("keeps the writer untouched when cancelled or invalid and treats Save-dialog cancellation neutrally", async () => {
    const writer = vi.fn(async () => "D:/exports/result.csv");
    const aborted = new AbortController();
    aborted.abort();
    await expect(dispatchCanonicalResultExportV2(fixture(), { format: "csv", tableIds: ["effects"] }, { text: writer }, aborted.signal))
      .resolves.toEqual({ status: "cancelled" });
    expect(writer).not.toHaveBeenCalled();

    const midFlight = new AbortController();
    const pending = dispatchCanonicalResultExportV2(fixture(), { format: "csv", tableIds: ["effects"] }, { text: writer }, midFlight.signal);
    midFlight.abort();
    await expect(pending).resolves.toMatchObject({ status: "cancelled" });
    expect(writer).not.toHaveBeenCalled();

    await expect(dispatchCanonicalResultExportV2(fixture(), { format: "csv", tableIds: [] }, { text: writer }))
      .resolves.toMatchObject({ status: "failed", format: "csv" });
    expect(writer).not.toHaveBeenCalled();

    const cancelledWriter = vi.fn(async () => null);
    await expect(dispatchCanonicalResultExportV2(fixture(), { format: "csv", tableIds: ["effects"] }, { text: cancelledWriter }))
      .resolves.toMatchObject({ status: "cancelled" });
    expect(cancelledWriter).toHaveBeenCalledOnce();
  });

  it.each([
    ["csv", "text", { format: "csv", tableIds: ["effects"] }],
    ["xlsx", "workbook", { format: "xlsx", tableIds: ["effects"] }],
    ["png", "binary", { format: "png", tableIds: [], chartIds: ["interaction_plot"] }],
  ] as const)("cooperatively cancels large %s preparation within one second before any writer", async (_format, writerKind, request) => {
    const controller = new AbortController();
    const writer = vi.fn(async () => "D:/exports/should-not-exist");
    const writers = { [writerKind]: writer };
    const startedAt = performance.now();
    const pending = dispatchCanonicalResultExportV2(largePreparationFixture(), request, writers, controller.signal);
    // Interleave enough browser tasks to finish the chunked semantic envelope
    // and enter the actual CSV/workbook/raster builder before aborting.
    let preparationTasksToPass = 28;
    const cancellationTick = () => {
      preparationTasksToPass -= 1;
      if (preparationTasksToPass > 0) {
        globalThis.setTimeout(cancellationTick, 0);
        return;
      }
      controller.abort();
    };
    globalThis.setTimeout(cancellationTick, 0);

    await expect(pending).resolves.toMatchObject({ status: "cancelled" });
    expect(performance.now() - startedAt).toBeLessThan(1_000);
    expect(writer).not.toHaveBeenCalled();
  });

  it.each([
    ["csv", { format: "csv", tableIds: ["effects"] }],
    ["xlsx", { format: "xlsx", tableIds: ["effects"] }],
    ["html", { format: "html", tableIds: ["scope"], chartIds: ["interaction_plot"] }],
    ["pdf", { format: "pdf", tableIds: ["scope"], chartIds: ["interaction_plot"] }],
    ["svg", { format: "svg", tableIds: [], chartIds: ["interaction_plot"] }],
    ["png", { format: "png", tableIds: [], chartIds: ["interaction_plot"] }],
  ] as const)("keeps async-dispatch %s output byte/model-identical to the synchronous pure API", async (format, request) => {
    const expected = prepared(request);
    const writer = vi.fn(async () => `D:/exports/result.${format}`);
    const writers = format === "xlsx"
      ? { workbook: writer }
      : format === "pdf" || format === "png"
        ? { binary: writer }
        : { text: writer };

    const outcome = await dispatchCanonicalResultExportV2(fixture(), request, writers);
    expect(outcome).toMatchObject({ status: "saved", path: `D:/exports/result.${format}` });
    if (outcome.status !== "saved") throw new Error(`Expected saved ${format}`);
    if ("bytes" in outcome.artifact && "bytes" in expected) {
      const { bytes: actualBytes, ...actualMetadata } = outcome.artifact;
      const { bytes: expectedBytes, ...expectedMetadata } = expected;
      expect(actualMetadata).toEqual(expectedMetadata);
      expect(actualBytes.length).toBe(expectedBytes.length);
      let mismatch = -1;
      for (let index = 0; index < actualBytes.length; index += 1) {
        if (actualBytes[index] !== expectedBytes[index]) {
          mismatch = index;
          break;
        }
      }
      expect(mismatch).toBe(-1);
    } else {
      expect(outcome.artifact).toEqual(expected);
    }
    expect(writer).toHaveBeenCalledOnce();
  });

  it("reports writer failure without returning a saved path", async () => {
    const writer = vi.fn(async () => {
      throw new Error("[CANONICAL_EXPORT_WRITE_FAILED] temporary write failed");
    });
    await expect(dispatchCanonicalResultExportV2(
      fixture(),
      { format: "html", tableIds: ["scope"], chartIds: [] },
      { text: writer },
    )).resolves.toMatchObject({
      status: "failed",
      format: "html",
      message: expect.stringContaining("CANONICAL_EXPORT_WRITE_FAILED"),
    });
    expect(writer).toHaveBeenCalledOnce();
  });
});
