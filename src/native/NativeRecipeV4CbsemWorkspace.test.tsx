import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Node } from "@xyflow/react";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import { useWorkspace } from "../store";
import type { ConstructData, Dataset } from "../types";
import {
  canonicalResultDocumentV2ExportTables,
  CanonicalResultDocumentV2View,
  NativeRecipeV4CbsemWorkspace,
  type NativeRecipeV4CbsemWorkspaceServices,
} from "./NativeRecipeV4CbsemWorkspace";

const originalState = useWorkspace.getState();

afterEach(() => {
  useWorkspace.setState(originalState, true);
});

function dataset(): Dataset {
  return {
    id: "dataset-v4",
    name: "Resident observations",
    kind: "raw",
    columns: ["x1", "x2", "x3"],
    rows: Array.from({ length: 12 }, (_, index) => ({ x1: index, x2: index + 1, x3: index + 2 })),
    rowCount: 12,
    missing: 0,
    fingerprint: "resident-fingerprint-v4",
  };
}

function factorNode(): Node<ConstructData> {
  return {
    id: "factor",
    type: "construct",
    position: { x: 100, y: 100 },
    data: {
      label: "Factor",
      shortName: "F",
      mode: "reflective",
      indicators: ["x1", "x2", "x3"],
      semModelV4: {
        version: 1,
        construct: { kind: "common_factor", marker_indicator: "x1" },
        identification: { kind: "marker_loading", indicator: "x1" },
      },
    },
  };
}

const services = {
  scientificDigest: vi.fn(),
  start: vi.fn(),
  status: vi.fn(),
  cancel: vi.fn(),
  dismiss: vi.fn(),
  result: vi.fn(),
  append: vi.fn(),
  read: vi.fn(),
  exportXlsx: vi.fn(),
  inspect: vi.fn(),
  selectArchive: vi.fn(),
} as unknown as NativeRecipeV4CbsemWorkspaceServices;

describe("Exact CB-SEM Recipe-v4 workspace accessibility", () => {
  it("renders the Standard exact workspace, labelled inputs, bootstrap control, layered preflight, and native job action", () => {
    const resident = dataset();
    useWorkspace.setState({
      projectName: "Study",
      projectPath: "D:\\Study.qpls",
      activeModelId: "model-v4",
      nodes: [factorNode()],
      edges: [],
      dataset: resident,
      datasetCatalog: [resident],
    });
    const html = renderToStaticMarkup(<NativeRecipeV4CbsemWorkspace modelName="One-factor CFA" experimentalLabsEnabled services={services} />);
    expect(html).toContain('role="tabpanel"');
    expect(html).toContain('aria-labelledby="nd-model-cbsem-labs-tab"');
    expect(html).toContain("Exact CB-SEM workspace");
    expect(html).toContain('id="nd-cbsem-v4-bootstrap-enabled"');
    expect(html).toContain("Full-ML indexed case bootstrap");
    expect(html).not.toContain("Experimental CB-SEM");
    expect(html).toContain("Scientific model");
    expect(html).toContain("Resident dataset");
    expect(html).toContain('id="nd-cbsem-v4-missing-data"');
    expect(html).toContain('aria-describedby="nd-cbsem-v4-missing-data-help"');
    expect(html).toContain("Listwise deletion");
    expect(html).toContain("Mean replacement (continuous variables only)");
    expect(html).toContain("Variable warnings begin at 5%");
    expect(html).toContain("above 15% for a variable or case");
    expect(html).toContain("Rows missing every modeled value are retained and fillable");
    expect(html).toContain("Layered preflight");
    expect(html).toContain("Start native job");
    expect(html).toContain("Schema-6 attachment");
    expect(html).not.toContain("select a previously stored exact result");
    expect(html).toContain("Reopen and verify completed run");
    expect(html).toMatch(/Append exact native document<\/button>/);
    expect(html).toMatch(/<button[^>]*disabled=""[^>]*>Append exact native document<\/button>/);
    expect(html).not.toContain("contenteditable");
  });

  it("remains available when Experimental Labs is disabled", () => {
    const html = renderToStaticMarkup(<NativeRecipeV4CbsemWorkspace modelName="CFA" experimentalLabsEnabled={false} services={services} />);
    expect(html).toContain("Exact CB-SEM workspace");
    expect(html).toContain('id="nd-cbsem-v4-bootstrap-enabled"');
  });

  it("renders canonical cells directly with accessible table headings", () => {
    const document: CanonicalResultDocumentV2 = {
      schema_version: 2,
      document_id: "document-v4",
      title: "Native CB-SEM result",
      provenance: {
        run_id: "run-v4",
        project_id: "project-v4",
        model_id: "model-v4",
        model_digest: "a".repeat(64),
        dataset_id: "dataset-v4",
        dataset_fingerprint: "resident-fingerprint-v4",
        recipe_id: "recipe-v4",
        recipe_digest: "b".repeat(64),
        capability_cell: { registry_schema_version: 2, capability_id: "smartpls.cbsem", cell_id: "qpls3.cbsem.ml", capability_version: "cbsem_ml_v1" },
        method_version: "cbsem_ml_exact_parameter_table_v3",
        engine_version: "test",
        seed: 42,
        workers: 1,
        started_at: "2026-08-15T00:00:00Z",
        completed_at: "2026-08-15T00:00:01Z",
      },
      sections: [{ id: "estimates", title: "Estimates", table_ids: ["parameters"], chart_ids: [] }],
      tables: [{
        id: "parameters",
        title: "Parameters",
        columns: [{ id: "estimate", label: "Estimate", data_type: "number", description: "Unstandardized estimate", default_precision: 2 }],
        rows: [{ id: "loading-x1", cells: [{ kind: "number", value: 1.2345 }] }],
        footnote_ids: [],
      }],
      charts: [],
      notices: [],
      exclusions: [],
      footnotes: [],
      presentation: { default_section_id: "estimates", default_table_id: "parameters", precision: 3, missing_value_label: "—", chart_defaults: {} },
    };
    const html = renderToStaticMarkup(<CanonicalResultDocumentV2View document={document} reopened />);
    expect(html).toContain("Reopened immutable schema-6 document");
    expect(html).toContain('<th scope="col"');
    expect(html).toContain("1.23");
    expect(html).toContain('data-canonical-table-id="parameters"');
    expect(html).toContain('data-result-horizontal-scroll="true"');
    expect(html).toContain('data-result-column-kind="number"');
    expect(html).toContain('data-result-responsive-columns="false"');
    const exported = canonicalResultDocumentV2ExportTables(document);
    expect(exported.map((table) => table.id)).toEqual([
      "parameters",
      "canonical_run_provenance",
      "canonical_result_notes",
    ]);
    expect(exported[0]?.rows).toEqual([["1.23"]]);
    expect(exported[1]?.rows).toContainEqual(["Run ID", "run-v4"]);
    expect(exported[1]?.rows).toContainEqual(["Recipe digest", "b".repeat(64)]);
  });

  it("renders persisted line charts with a textual legend and exact table fallback", () => {
    const primary = { registry_schema_version: 2 as const, capability_id: "smartpls.moderation", cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point", capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1" };
    const document: CanonicalResultDocumentV2 = {
      schema_version: 2,
      document_id: "moderation-chart-document",
      title: "Multiple moderation result",
      provenance: {
        run_id: "run-moderation",
        project_id: "project-moderation",
        model_id: "model-moderation",
        model_digest: "a".repeat(64),
        dataset_id: "dataset-moderation",
        dataset_fingerprint: "b".repeat(64),
        recipe_id: "recipe-moderation",
        recipe_digest: "c".repeat(64),
        capability_cell: primary,
        method_version: "qpls.general-sem-pls.multiple-two-way.point.v1",
        engine_version: "compiled_general_sem_pls_recipe_v1_multiple_two_way_moderation_point_execution_v1",
        seed: 42,
        workers: 1,
        started_at: "2026-08-19T00:00:00Z",
        completed_at: "2026-08-19T00:00:01Z",
      },
      capability_cells: [primary],
      sections: [{ id: "conditional_effects", title: "Conditional effects", table_ids: ["general_sem_interaction_plots"], chart_ids: ["plot:x-by-w"], capability_cells: [primary] }],
      tables: [{
        id: "general_sem_interaction_plots",
        title: "Persisted interaction plot points",
        columns: [
          { id: "moderator", label: "Moderator", data_type: "number", description: "Standardized moderator" },
          { id: "outcome", label: "Predicted outcome", data_type: "number", description: "Predicted outcome" },
        ],
        rows: [
          { id: "low", cells: [{ kind: "number", value: -1 }, { kind: "number", value: -0.75 }] },
          { id: "mean", cells: [{ kind: "number", value: 0 }, { kind: "number", value: 0.25 }] },
          { id: "high", cells: [{ kind: "number", value: 1 }, { kind: "number", value: 1.5 }] },
        ],
        footnote_ids: [],
        capability_cells: [primary],
      }],
      charts: [{
        id: "plot:x-by-w",
        title: "Conditional X to Y relationship",
        description: "Predicted Y across fixed W probes.",
        kind: "line",
        series: [
          { id: "w-low", label: "W = -1", points: [{ x: -1, y: -0.75, label: "low X" }, { x: 1, y: 0.75, label: "high X" }] },
          { id: "w-high", label: "W = +1", points: [{ x: -1, y: -0.25, lower: -0.5, upper: 0 }, { x: 1, y: 1.5, lower: 1.2, upper: 1.8 }] },
        ],
        source_table_id: "general_sem_interaction_plots",
        display: { show_legend: true, show_values: true, x_axis_label: "Standardized X", y_axis_label: "Predicted Y" },
      }],
      notices: [], exclusions: [], footnotes: [],
      presentation: { default_section_id: "conditional_effects", default_table_id: "general_sem_interaction_plots", precision: 3, missing_value_label: "—", chart_defaults: {} },
    };

    const html = renderToStaticMarkup(<CanonicalResultDocumentV2View document={document} reopened />);
    expect(html).toContain('data-canonical-chart-id="plot:x-by-w"');
    expect(html).toContain('<svg class="nd-canonical-chart__plot"');
    expect(html).toContain('role="img"');
    expect(html).toContain("Line chart with 2 series and 4 persisted points");
    expect(html).toContain("W = -1 (solid)");
    expect(html).toContain("W = +1 (long dashed)");
    expect(html).toContain("W = +1: x -1, y -0.25, interval -0.5 to 0");
    expect(html).toMatch(/href="#nd-canonical-table-[0-9a-f]+"/u);
    expect(html).toContain("Exact plot data: Persisted interaction plot points");
    expect(html).toContain('data-canonical-table-id="general_sem_interaction_plots"');
  });

  it("renders the generic nonlinear section and all three v7 tables accessibly", () => {
    const primary = { registry_schema_version: 2 as const, capability_id: "smartpls.nonlinear_relationships", cell_id: "qpls3.pls.nonlinear_quadratic", capability_version: "pls_quadratic_nonlinear_effects_v1" };
    const textColumn = (id: string) => ({ id, label: id, data_type: "text" as const, description: id });
    const numberColumn = (id: string) => ({ id, label: id, data_type: "number" as const, description: id });
    const document: CanonicalResultDocumentV2 = {
      schema_version: 2,
      document_id: "nonlinear-v7",
      title: "PLS nonlinear quadratic diagnostics",
      provenance: {
        run_id: "run-v7", project_id: "project-v7", model_id: "model-v7",
        model_digest: "a".repeat(64), dataset_id: "dataset-v7", dataset_fingerprint: "b".repeat(64),
        recipe_id: "recipe-v7", recipe_digest: "c".repeat(64), capability_cell: primary,
        method_version: "pls_quadratic_nonlinear_effects_v1",
        engine_version: "compiled_recipe_v4_pls_plan_v2_execution_v7", seed: 42, workers: 1,
        started_at: "2026-08-16T00:00:00Z", completed_at: "2026-08-16T00:00:01Z",
      },
      capability_cells: [primary],
      sections: [{
        id: "nonlinear_relationships", title: "Nonlinear relationships",
        table_ids: ["nonlinear_quadratic_diagnostics", "nonlinear_equation_fit", "nonlinear_method_scope"],
        chart_ids: [], capability_cells: [primary],
      }],
      tables: [
        {
          id: "nonlinear_quadratic_diagnostics", title: "Quadratic diagnostics",
          columns: [textColumn("source"), textColumn("target"), numberColumn("linear_coefficient"), numberColumn("quadratic_coefficient"), numberColumn("standard_error"), numberColumn("t_statistic"), numberColumn("p_value_two_sided"), textColumn("warning")],
          rows: [{ id: "nonlinear_quadratic_diagnostic_0000", cells: [
            { kind: "text", value: "x" }, { kind: "text", value: "y" },
            { kind: "number", value: 0.25 }, { kind: "number", value: 0.1 },
            { kind: "number", value: 0.05 }, { kind: "number", value: 2 },
            { kind: "number", value: 0.0455 }, { kind: "missing", reason: "not_estimated" },
          ] }],
          footnote_ids: [], capability_cells: [primary],
        },
        {
          id: "nonlinear_equation_fit", title: "Nonlinear equation fit",
          columns: [textColumn("target"), numberColumn("linear_r_squared"), numberColumn("augmented_r_squared"), numberColumn("delta_r_squared")],
          rows: [{ id: "nonlinear_equation_fit_0000", cells: [
            { kind: "text", value: "y" }, { kind: "number", value: 0.4 },
            { kind: "number", value: 0.45 }, { kind: "number", value: 0.04999999999999999 },
          ] }],
          footnote_ids: [], capability_cells: [primary],
        },
        {
          id: "nonlinear_method_scope", title: "Nonlinear method scope",
          columns: [textColumn("method_version"), textColumn("term"), textColumn("warning")],
          rows: [{ id: "nonlinear_method_scope", cells: [
            { kind: "text", value: "pls_quadratic_nonlinear_effects_v1" },
            { kind: "text", value: "centered_squared_construct_score_v1" },
            { kind: "text", value: "Fixed-score diagnostic scope" },
          ] }],
          footnote_ids: [], capability_cells: [primary],
        },
      ],
      charts: [], notices: [], exclusions: [], footnotes: [],
      presentation: { default_section_id: "nonlinear_relationships", default_table_id: "nonlinear_quadratic_diagnostics", precision: 4, missing_value_label: "—", chart_defaults: {} },
    };
    const html = renderToStaticMarkup(<CanonicalResultDocumentV2View document={document} reopened />);
    expect(html).toContain("PLS nonlinear quadratic diagnostics");
    expect(html).toContain("Quadratic diagnostics");
    expect(html).toContain("Nonlinear equation fit");
    expect(html).toContain("Nonlinear method scope");
    expect(html).toContain("centered_squared_construct_score_v1");
    expect(html.match(/<th scope="col"/g)?.length).toBe(15);
  });
});
