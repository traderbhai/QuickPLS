import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type {
  CanonicalResultChart,
  CanonicalResultColumn,
  CanonicalResultDocumentV2,
  CanonicalResultTable,
  CapabilityCellReferenceV2,
} from "./canonicalResultDocumentV2";
import {
  CANONICAL_RESULT_DERIVED_AGGREGATE_EFFECT_CHART_ID_V2,
  CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2,
  canonicalResultExportChartsV2,
  prepareCanonicalResultExportV2,
  readPreparedCanonicalResultExportSemanticV2,
  verifyPreparedCanonicalResultExportV2,
  type CanonicalResultExportRequestV2,
  type PreparedCanonicalResultExportV2,
} from "./canonicalResultCrossFormatExportV2";

type Rank0CellKind =
  | "mediation_point"
  | "mediation_bootstrap"
  | "moderation_point"
  | "moderation_bootstrap";

const BASE_CELL: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.pls_algorithm",
  cell_id: "qpls3.pls.algorithm",
  capability_version: "pls_pm_v1",
};
const MEDIATION_POINT_CELL: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.mediation",
  capability_version: "pls_mediation_v1",
};
const MEDIATION_BOOTSTRAP_CELL: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.mediation",
  cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
  capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
};
const MODERATION_POINT_CELL: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
  capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
};
const MODERATION_BOOTSTRAP_CELL: CapabilityCellReferenceV2 = {
  registry_schema_version: 2,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
  capability_version: "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
};

const textColumn = (id: string, label: string): CanonicalResultColumn => ({
  id,
  label,
  data_type: "text",
  description: `Native canonical ${label} field.`,
  role: "label",
});
const numberColumn = (id: string, label: string): CanonicalResultColumn => ({
  id,
  label,
  data_type: "number",
  description: `Native canonical ${label} field.`,
  role: "estimate",
  default_precision: 4,
});
const booleanColumn = (id: string, label: string): CanonicalResultColumn => ({
  id,
  label,
  data_type: "boolean",
  description: `Native canonical ${label} field.`,
  role: "diagnostic",
});
const text = (value: string) => ({ kind: "text" as const, value });
const number = (value: number) => ({ kind: "number" as const, value });
const boolean = (value: boolean) => ({ kind: "boolean" as const, value });
const missing = () => ({ kind: "missing" as const, reason: "not_estimated" as const });

function cellIdentity(cell: CapabilityCellReferenceV2): string {
  return `${cell.registry_schema_version}:${cell.capability_id}:${cell.cell_id}:${cell.capability_version}`;
}

function sortedCells(...cells: CapabilityCellReferenceV2[]): CapabilityCellReferenceV2[] {
  return [...cells].sort((left, right) => cellIdentity(left).localeCompare(cellIdentity(right)));
}

const effectColumns = (): CanonicalResultColumn[] => [
  textColumn("effect_id", "Effect ID"),
  textColumn("estimand_id", "Estimand ID"),
  textColumn("kind", "Kind"),
  textColumn("source", "Source"),
  textColumn("target", "Target"),
  textColumn("path_identities", "Paths"),
  numberColumn("estimate", "Estimate"),
  numberColumn("bootstrap_mean", "Bootstrap mean"),
  numberColumn("bootstrap_bias", "Bias"),
  numberColumn("standard_error", "Standard error"),
  numberColumn("lower", "Lower"),
  numberColumn("upper", "Upper"),
  numberColumn("p_value", "P value"),
  numberColumn("usable_replicates", "Usable replicates"),
];

function mediationTables(bootstrap: boolean): CanonicalResultTable[] {
  const tableCells = bootstrap
    ? sortedCells(MEDIATION_BOOTSTRAP_CELL, MEDIATION_POINT_CELL)
    : [MEDIATION_POINT_CELL];
  const inference = bootstrap
    ? [number(0.183), number(0.003), number(0.041), number(0.101), number(0.262), number(0.002), number(500)]
    : [missing(), missing(), missing(), missing(), missing(), missing(), missing()];
  const tables: CanonicalResultTable[] = [
    {
      id: "general_sem_specific_indirect_effects",
      title: "Specific indirect effects",
      description: "Requested ordered mediation paths with stable relation identities and optional full-model bootstrap inference.",
      columns: effectColumns(),
      rows: [{
        id: "specific_indirect_0000",
        cells: [
          text("specific_indirect:estimand:x_m1_y"),
          text("estimand:x_m1_y"),
          text("specific_indirect"),
          text("construct:x"),
          text("construct:y"),
          text("relation:x_m1 -> relation:m1_y"),
          number(0.18),
          ...inference,
        ],
      }],
      footnote_ids: [],
      capability_cells: tableCells,
    },
    {
      id: "general_sem_aggregate_effects",
      title: "Aggregate effects",
      description: "Requested total-indirect and total effects with their contributing stable path identities.",
      columns: effectColumns(),
      rows: [{
        id: "aggregate_effect_0000",
        cells: [
          text("total_indirect:construct:x:construct:y"),
          text("estimand:total_x_y"),
          text("total_indirect"),
          text("construct:x"),
          text("construct:y"),
          text("relation:x_m1 -> relation:m1_y; relation:x_m2 -> relation:m2_y"),
          number(0.29),
          ...inference,
        ],
      }],
      footnote_ids: [],
      capability_cells: tableCells,
    },
  ];
  if (bootstrap) {
    tables.push({
      id: "general_sem_bootstrap_receipt",
      title: "Full-model bootstrap receipt",
      description: "Exact case-resampling, usable-replicate, worker, and failure-ledger identity for inferred mediation effects.",
      columns: [
        textColumn("method_version", "Method"),
        numberColumn("resamples_requested", "Requested"),
        numberColumn("resamples_usable", "Usable"),
        numberColumn("minimum_usable", "Minimum usable"),
        textColumn("seed", "Seed"),
        numberColumn("workers", "Workers"),
        booleanColumn("complete_model_refit", "Complete refit"),
        numberColumn("failed_replicates", "Failed"),
      ],
      rows: [{
        id: "bootstrap_receipt",
        cells: [
          text("general_sem_pls_full_model_case_bootstrap_v1"),
          number(500),
          number(500),
          number(450),
          text("42"),
          number(2),
          boolean(true),
          number(0),
        ],
      }],
      footnote_ids: [],
      capability_cells: tableCells,
    });
  }
  return tables;
}

function moderationTables(bootstrap: boolean): CanonicalResultTable[] {
  const pointCells = [MODERATION_POINT_CELL];
  const tables: CanonicalResultTable[] = [
    {
      id: "general_sem_interaction_effects",
      title: "Interaction effects and product scaling",
      description: "Final joint stage-two interaction coefficients with their exact product-scaling receipts.",
      columns: [
        textColumn("effect_id", "Effect ID"),
        textColumn("interaction_id", "Interaction"),
        textColumn("focal_predictor_id", "Focal predictor"),
        textColumn("moderator_id", "Moderator"),
        textColumn("outcome_id", "Outcome"),
        numberColumn("standardized_product_coefficient", "Standardized product"),
        numberColumn("scientific_rescaled_gamma", "Rescaled gamma"),
      ],
      rows: [{
        id: "interaction_effect_0000",
        cells: [
          text("interaction_effect:interaction:x_by_m1"),
          text("interaction:x_by_m1"),
          text("construct:x"),
          text("construct:m1"),
          text("construct:y"),
          number(0.21),
          number(0.19),
        ],
      }],
      footnote_ids: [],
      capability_cells: pointCells,
    },
    {
      id: "general_sem_conditional_slopes",
      title: "Conditional focal slopes",
      description: "Frozen minus-one, zero, and plus-one standardized moderator probes from the final joint stage-two coefficients.",
      columns: [
        textColumn("effect_id", "Effect ID"),
        textColumn("interaction_id", "Interaction"),
        textColumn("moderator_id", "Moderator"),
        numberColumn("moderator_value", "Moderator value"),
        numberColumn("estimate", "Conditional slope"),
      ],
      rows: [-1, 0, 1].map((probe, index) => ({
        id: `conditional_slope_000${index}`,
        cells: [
          text(`conditional_effect:interaction:x_by_m1:${index}`),
          text("interaction:x_by_m1"),
          text("construct:m1"),
          number(probe),
          number(0.35 + 0.19 * probe),
        ],
      })),
      footnote_ids: [],
      capability_cells: pointCells,
    },
    {
      id: "general_sem_interaction_plots",
      title: "Interaction plot points",
      description: "Every point from the typed canonical interaction plots, materialized for viewer and tabular export parity.",
      columns: [
        textColumn("plot_id", "Plot ID"),
        textColumn("interaction_id", "Interaction"),
        textColumn("series_id", "Series ID"),
        numberColumn("moderator_value", "Moderator value"),
        numberColumn("focal_value", "Focal value"),
        numberColumn("predicted_value", "Predicted outcome"),
        numberColumn("lower", "Lower"),
        numberColumn("upper", "Upper"),
      ],
      rows: [{
        id: "interaction_plot_point_0000",
        cells: [
          text("interaction_plot:interaction:x_by_m1"),
          text("interaction:x_by_m1"),
          text("probe_series:minus_one"),
          number(-1),
          number(-1),
          number(-0.16),
          missing(),
          missing(),
        ],
      }],
      footnote_ids: [],
      capability_cells: pointCells,
    },
  ];
  if (bootstrap) {
    tables.push(
      {
        id: "general_sem_moderation_gamma_inference",
        title: "Scientific gamma bootstrap inference",
        description: "Percentile full-pipeline case-bootstrap inference for every compiled scientific rescaled interaction gamma.",
        columns: [
          textColumn("effect_id", "Effect ID"),
          textColumn("interaction_id", "Interaction"),
          numberColumn("estimate", "Estimate"),
          numberColumn("bootstrap_mean", "Bootstrap mean"),
          numberColumn("standard_error", "Standard error"),
          numberColumn("lower", "Lower"),
          numberColumn("upper", "Upper"),
          numberColumn("p_value", "P value"),
          numberColumn("bootstrap_usable_replicates", "Usable"),
        ],
        rows: [{
          id: "moderation_gamma_inference_0000",
          cells: [
            text("interaction_effect:interaction:x_by_m1"),
            text("interaction:x_by_m1"),
            number(0.19),
            number(0.192),
            number(0.05),
            number(0.09),
            number(0.29),
            number(0.002),
            number(500),
          ],
        }],
        footnote_ids: [],
        capability_cells: [MODERATION_BOOTSTRAP_CELL],
      },
      {
        id: "general_sem_moderation_bootstrap_receipt",
        title: "Moderation bootstrap pipeline receipt",
        description: "Exact supplemental cell, algorithm, provenance, full-pipeline, and failed-replicate receipt.",
        columns: [
          textColumn("cell_id", "Cell"),
          textColumn("method_version", "Method"),
          numberColumn("resamples_requested", "Requested"),
          numberColumn("resamples_usable", "Usable"),
          booleanColumn("complete_model_reestimated_per_replicate", "Complete model"),
        ],
        rows: [{
          id: "moderation_bootstrap_receipt",
          cells: [
            text(MODERATION_BOOTSTRAP_CELL.cell_id),
            text("qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1"),
            number(500),
            number(500),
            boolean(true),
          ],
        }],
        footnote_ids: [],
        capability_cells: [MODERATION_BOOTSTRAP_CELL],
      },
    );
  }
  return tables;
}

function moderationChart(): CanonicalResultChart {
  return {
    id: "general_sem_interaction_chart_0000",
    title: "Interaction interaction:x_by_m1",
    description: "Final joint stage-two predicted construct:y across standardized construct:x values at frozen standardized construct:m1 probes.",
    kind: "line",
    series: [
      {
        id: "probe_series:minus_one",
        label: "construct:m1 = -1.0000",
        group: "interaction:x_by_m1",
        points: [{ x: -1, y: -0.16 }, { x: 1, y: 0.16 }],
      },
      {
        id: "probe_series:zero",
        label: "construct:m1 = 0.0000",
        group: "interaction:x_by_m1",
        points: [{ x: -1, y: -0.35 }, { x: 1, y: 0.35 }],
      },
      {
        id: "probe_series:plus_one",
        label: "construct:m1 = 1.0000",
        group: "interaction:x_by_m1",
        points: [{ x: -1, y: -0.54 }, { x: 1, y: 0.54 }],
      },
    ],
    source_table_id: "general_sem_interaction_plots",
    display: {
      show_legend: true,
      show_values: false,
      x_axis_label: "construct:x (standardized)",
      y_axis_label: "construct:y (predicted standardized)",
    },
  };
}

function rank0Document(kind: Rank0CellKind): CanonicalResultDocumentV2 {
  const moderation = kind.startsWith("moderation");
  const bootstrap = kind.endsWith("bootstrap");
  const primary = moderation ? MODERATION_POINT_CELL : MEDIATION_POINT_CELL;
  const supplemental = moderation ? MODERATION_BOOTSTRAP_CELL : MEDIATION_BOOTSTRAP_CELL;
  const cells = bootstrap
    ? sortedCells(BASE_CELL, primary, supplemental)
    : sortedCells(BASE_CELL, primary);
  const tables = moderation ? moderationTables(bootstrap) : mediationTables(bootstrap);
  const charts = moderation ? [moderationChart()] : [];
  const pointTableIds = moderation
    ? ["general_sem_interaction_effects", "general_sem_conditional_slopes", "general_sem_interaction_plots"]
    : tables.map((table) => table.id);
  const pointSectionCells = moderation
    ? [MODERATION_POINT_CELL]
    : bootstrap
      ? sortedCells(MEDIATION_BOOTSTRAP_CELL, MEDIATION_POINT_CELL)
      : [MEDIATION_POINT_CELL];
  const sections: CanonicalResultDocumentV2["sections"] = [{
    id: moderation ? "general_sem_moderation" : "general_sem_effects",
    title: moderation ? "Moderation effects" : "Mediation effects",
    description: moderation
      ? "Exact simultaneous two-way interaction coefficients, product scaling receipts, conditional slopes, and typed interaction plots from the final joint stage-two solve."
      : "Stable specific-indirect, total-indirect, and total-effect estimands from the authored recursive SEM topology.",
    table_ids: pointTableIds,
    chart_ids: charts.map((chart) => chart.id),
    capability_cells: pointSectionCells,
  }];
  if (moderation && bootstrap) {
    sections.push({
      id: "general_sem_moderation_bootstrap",
      title: "Moderation bootstrap inference",
      description: "Gamma-only percentile case-bootstrap inference for every simultaneous two-way interaction.",
      table_ids: ["general_sem_moderation_gamma_inference", "general_sem_moderation_bootstrap_receipt"],
      chart_ids: [],
      capability_cells: [MODERATION_BOOTSTRAP_CELL],
    });
  }
  const title = moderation
    ? bootstrap
      ? "General SEM simultaneous two-way PLS moderation bootstrap inference"
      : "General SEM simultaneous two-way PLS moderation point estimates"
    : bootstrap
      ? "PLS-SEM multiple mediation with full-model bootstrap"
      : "PLS-SEM mediation effects";
  const methodVersion = moderation
    ? bootstrap
      ? "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1"
      : "qpls.general-sem-pls.multiple-two-way.point.v1"
    : bootstrap
      ? "general_sem_pls_full_model_case_bootstrap_v1"
      : "general_sem_effects_v1";
  const engineVersion = moderation
    ? bootstrap
      ? "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_percentile_bootstrap_execution_v1"
      : "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1"
    : bootstrap
      ? "compiled_general_sem_pls_recipe_v1_percentile_bootstrap_execution_v1"
      : "compiled_general_sem_pls_recipe_v1_point_execution_v1";
  const defaultSection = moderation && bootstrap
    ? "general_sem_moderation_bootstrap"
    : moderation
      ? "general_sem_moderation"
      : "general_sem_effects";
  const defaultTable = moderation && bootstrap
    ? "general_sem_moderation_gamma_inference"
    : tables[0]!.id;
  return {
    schema_version: 2,
    document_id: `result_rank0_${kind}`,
    title,
    provenance: {
      run_id: `run:rank0:${kind}`,
      project_id: "00000000-0000-4000-8000-000000007400",
      model_id: "model:native-general-sem",
      model_digest: "a".repeat(64),
      dataset_id: "00000000-0000-4000-8000-000000007401",
      dataset_fingerprint: "b".repeat(64),
      recipe_id: "00000000-0000-4000-8000-000000007402",
      recipe_digest: "c".repeat(64),
      capability_cell: primary,
      method_version: methodVersion,
      engine_version: engineVersion,
      seed: 42,
      workers: 2,
      started_at: "2026-08-19T10:00:00Z",
      completed_at: "2026-08-19T10:00:01Z",
    },
    capability_cells: cells,
    sections,
    tables,
    charts,
    notices: [],
    exclusions: [],
    footnotes: [],
    presentation: {
      default_section_id: defaultSection,
      default_table_id: defaultTable,
      precision: 4,
      // This is the routine non-ASCII production value. PDF/PNG keep it in
      // the semantic envelope while rendering exact ASCII analytical tokens.
      missing_value_label: "—",
      chart_defaults: { show_legend: true },
    },
  };
}

function prepared(
  document: CanonicalResultDocumentV2,
  request: CanonicalResultExportRequestV2,
): PreparedCanonicalResultExportV2 {
  const result = prepareCanonicalResultExportV2(document, request);
  expect(result, `${document.provenance.capability_cell.cell_id} ${request.format}`).toMatchObject({ ok: true });
  if (!result.ok) throw new Error(result.errors.join("\n"));
  expect(verifyPreparedCanonicalResultExportV2(document, result.artifact)).toMatchObject({
    passed: true,
    exact_semantic_match: true,
    digest_match: true,
    rendered_surface_match: true,
    errors: [],
  });
  return result.artifact;
}

describe("Rank 0 native-shaped canonical cross-format export", () => {
  it("keeps the four fixtures pinned to the native Rust projection identities and ordinary Unicode boundary", () => {
    const source = readFileSync("src-tauri/src/recipe_v4_general_sem_canonical_result.rs", "utf8");
    const baseSource = readFileSync("src-tauri/src/recipe_v4_canonical_result.rs", "utf8");
    for (const literal of [
      "PLS-SEM mediation effects",
      "PLS-SEM multiple mediation with full-model bootstrap",
      "General SEM simultaneous two-way PLS moderation point estimates",
      "General SEM simultaneous two-way PLS moderation bootstrap inference",
      "general_sem_specific_indirect_effects",
      "general_sem_interaction_effects",
      "general_sem_moderation_gamma_inference",
      "Interaction effects and product scaling",
      "Scientific gamma bootstrap inference",
    ]) expect(source).toContain(literal);
    expect(baseSource).toContain('missing_value_label: "—".into()');
  });

  it.each([
    "mediation_point",
    "mediation_bootstrap",
    "moderation_point",
    "moderation_bootstrap",
  ] as const)("reconciles CSV, XLSX, HTML, and PDF from one table selection for %s", (kind) => {
    const document = rank0Document(kind);
    const tableIds = document.tables.map((table) => table.id);
    const artifacts = (["csv", "xlsx", "html", "pdf"] as const).map((format) => prepared(document, {
      format,
      tableIds,
      chartIds: [],
    }));
    expect(new Set(artifacts.map((artifact) => artifact.semantic.semantic_sha256)).size).toBe(1);
    expect(artifacts.every((artifact) => (
      readPreparedCanonicalResultExportSemanticV2(artifact)?.presentation.missing_value_label === "—"
    ))).toBe(true);

    const pdf = artifacts.find((artifact) => artifact.format === "pdf");
    if (!pdf || pdf.format !== "pdf") throw new Error("Expected PDF artifact");
    const visiblePdf = new TextDecoder().decode(pdf.bytes);
    const hasMissing = document.tables.some((table) => table.rows.some((row) => row.cells.some((cell) => cell.kind === "missing")));
    expect(visiblePdf.includes("missing:not_estimated")).toBe(hasMissing);
    expect(visiblePdf).not.toContain("—");
  });

  it.each([
    "moderation_point",
    "moderation_bootstrap",
  ] as const)("reconciles native interaction SVG/PNG and complete HTML/PDF reports for %s", (kind) => {
    const document = rank0Document(kind);
    const chartId = document.charts[0]!.id;
    const chartArtifacts = (["svg", "png"] as const).map((format) => prepared(document, {
      format,
      tableIds: [],
      chartIds: [chartId],
    }));
    expect(new Set(chartArtifacts.map((artifact) => artifact.semantic.semantic_sha256)).size).toBe(1);
    expect(chartArtifacts.every((artifact) => (
      readPreparedCanonicalResultExportSemanticV2(artifact)?.selection.chart_ids[0] === chartId
    ))).toBe(true);

    const completeReportArtifacts = (["html", "pdf"] as const).map((format) => prepared(document, {
      format,
      tableIds: document.tables.map((table) => table.id),
      chartIds: [chartId],
    }));
    expect(new Set(completeReportArtifacts.map((artifact) => artifact.semantic.semantic_sha256)).size).toBe(1);
  });

  it.each([
    "mediation_point",
    "mediation_bootstrap",
  ] as const)("reconciles truthful table-derived SVG/PNG and complete HTML/PDF reports for %s", (kind) => {
    const document = rank0Document(kind);
    expect(document.charts).toEqual([]);
    const exportCharts = canonicalResultExportChartsV2(document);
    expect(exportCharts.map(({ chart, origin }) => ({ id: chart.id, origin, source: chart.source_table_id }))).toEqual([
      {
        id: CANONICAL_RESULT_DERIVED_SPECIFIC_INDIRECT_CHART_ID_V2,
        origin: "derived_from_canonical_table",
        source: "general_sem_specific_indirect_effects",
      },
      {
        id: CANONICAL_RESULT_DERIVED_AGGREGATE_EFFECT_CHART_ID_V2,
        origin: "derived_from_canonical_table",
        source: "general_sem_aggregate_effects",
      },
    ]);
    const chartId = exportCharts[0]!.chart.id;
    const chartArtifacts = (["svg", "png"] as const).map((format) => prepared(document, {
      format,
      tableIds: [],
      chartIds: [chartId],
    }));
    expect(new Set(chartArtifacts.map((artifact) => artifact.semantic.semantic_sha256)).size).toBe(1);
    for (const artifact of chartArtifacts) {
      const chart = readPreparedCanonicalResultExportSemanticV2(artifact)?.charts[0];
      expect(chart).toMatchObject({
        id: chartId,
        source_table_id: "general_sem_specific_indirect_effects",
        kind: "bar",
        series: [{ id: "estimate", points: [{ x: 1, y: 0.18, label: "specific_indirect:estimand:x_m1_y" }] }],
      });
    }

    const completeReportArtifacts = (["html", "pdf"] as const).map((format) => prepared(document, {
      format,
      tableIds: document.tables.map((table) => table.id),
      chartIds: [chartId],
    }));
    expect(new Set(completeReportArtifacts.map((artifact) => artifact.semantic.semantic_sha256)).size).toBe(1);
  });
});
