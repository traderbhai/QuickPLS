import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { completedSamplePlsRun } from "../data/smokeRun";
import type { CanonicalResultDocumentV2 } from "../domain/canonicalResultDocumentV2";
import type { ResultTable } from "../domain/resultTables";
import type { AnalysisRun } from "../types";
import type { NativeResultNavigation } from "./nativeResults";
import NativeResultsSurface, {
  NcaCeilingPlot,
  nativeResultTreeKeyboardAction,
  nativeVisibleResultTreeEntries,
} from "./NativeResultsSurface";
import { completedCbsemRun } from "./nativeCbsem.testFixture";
import { completedGscaRun } from "./nativeGsca.testFixture";
import { buildNativeResultNavigation, resultTableForItem } from "./nativeResults";

const navigation: NativeResultNavigation = {
  runId: "run-1",
  defaultItemId: "path_coefficients",
  groups: [
    {
      id: "graphical",
      title: "Graphical output",
      items: [{
        id: "model_estimates",
        kind: "diagram",
        title: "Model estimates",
        diagram: "model_estimates",
      }],
    },
    {
      id: "final_results",
      title: "Final results",
      items: [
        { id: "path_coefficients", kind: "table", title: "Path coefficients", tableId: "path_coefficients" },
        { id: "r_squared", kind: "table", title: "R squared", tableId: "r_squared" },
      ],
    },
  ],
  tables: [],
};

function completedHigherOrderRun(): AnalysisRun {
  const run = completedSamplePlsRun();
  run.id = "results-surface-hoc";
  run.name = "Higher-order construct run";
  run.modelSnapshot = {
    nodes: [
      { id: "competence", position: { x: 0, y: 0 }, data: { label: "Competence", shortName: "COMP", mode: "reflective", indicators: ["COMP1", "COMP2", "COMP3"] } },
      { id: "likeability", position: { x: 0, y: 160 }, data: { label: "Likeability", shortName: "LIKE", mode: "reflective", indicators: ["LIKE1", "LIKE2"] } },
      {
        id: "hoc",
        position: { x: 240, y: 80 },
        data: {
          label: "Corporate standing",
          shortName: "HOC",
          mode: "reflective",
          indicators: [],
          semantic: "higher_order",
          higherOrder: { id: "hoc", components: ["competence", "likeability"], method: "two_stage", stage_one_recipe: null },
        },
      },
    ],
    edges: [],
  };
  run.result!.outer_estimates.push(
    { construct: "hoc", indicator: "__qpls_hoc_hoc_competence", loading: 0.91, weight: 0.58 },
    { construct: "hoc", indicator: "__qpls_hoc_hoc_likeability", loading: 0.89, weight: 0.55 },
  );
  return run;
}

describe("native Results tree accessibility", () => {
  it("renders a searchable categorized tree for a selected schema-6 canonical output", () => {
    const canonicalDocument: CanonicalResultDocumentV2 = {
      schema_version: 2,
      document_id: "result:canonical",
      title: "Unified SEM result",
      provenance: {
        run_id: "run:canonical",
        project_id: "project:canonical",
        model_id: "model:canonical",
        model_digest: "a".repeat(64),
        dataset_id: "dataset:canonical",
        dataset_fingerprint: "b".repeat(64),
        recipe_id: "recipe:canonical",
        recipe_digest: "c".repeat(64),
        capability_cell: {
          registry_schema_version: 2,
          capability_id: "smartpls.cbsem",
          cell_id: "qpls3.cbsem.general_sem_ml",
          capability_version: "cbsem_general_sem_ml_v1",
        },
        method_version: "cbsem_general_sem_ml_v1",
        engine_version: "native_cbsem_v1",
        seed: null,
        workers: 1,
        started_at: "2026-08-21T00:00:00Z",
        completed_at: "2026-08-21T00:00:01Z",
      },
      sections: [{
        id: "cbsem_general_sem_point",
        title: "CB-SEM ML estimates",
        table_ids: ["cbsem_general_sem_parameters", "cbsem_general_sem_fit"],
        chart_ids: [],
      }],
      tables: [
        {
          id: "cbsem_general_sem_parameters",
          title: "CB-SEM parameters",
          columns: [{ id: "estimate", label: "Estimate", data_type: "number", description: "ML estimate." }],
          rows: [{ id: "loading", cells: [{ kind: "number", value: 0.8 }] }],
          footnote_ids: [],
        },
        {
          id: "cbsem_general_sem_fit",
          title: "Model fit",
          columns: [{ id: "cfi", label: "CFI", data_type: "number", description: "Comparative fit index." }],
          rows: [{ id: "fit", cells: [{ kind: "number", value: 0.96 }] }],
          footnote_ids: [],
        },
      ],
      charts: [],
      notices: [],
      exclusions: [{ id: "bounded", title: "Bounded model", reason: "Raw continuous data only." }],
      footnotes: [],
      presentation: {
        default_section_id: "cbsem_general_sem_point",
        default_table_id: "cbsem_general_sem_parameters",
        precision: 4,
        missing_value_label: "—",
        chart_defaults: {},
      },
    };
    const selectedNavigationId = "canonical:table:cbsem_general_sem_fit";
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[]}
      selectedRunId=""
      setSelectedRunId={vi.fn()}
      navigation={{ runId: null, defaultItemId: null, groups: [], tables: [] }}
      setSelectedTableId={vi.fn()}
      canonicalDocument={canonicalDocument}
      canonicalSelected
      canonicalNavigationItemId={selectedNavigationId}
      onCanonicalNavigationItemChange={vi.fn()}
      selectCanonicalDocument={vi.fn()}
      propertiesOpen
    />);

    expect(markup).toContain('aria-label="Search result sections"');
    expect(markup).toContain("CB-SEM Parameters");
    expect(markup).toContain("CB-SEM Fit and Identification");
    expect(markup).toContain("Diagnostics and Run Details");
    expect(markup).toContain(`data-result-tree-item-id="${selectedNavigationId}"`);
    expect(markup.match(new RegExp(`<button[^>]*data-result-tree-item-id="${selectedNavigationId}"[^>]*>`))?.[0])
      .toContain('aria-selected="true"');
    expect(markup).toContain('data-canonical-table-id="cbsem_general_sem_fit"');
    expect(markup).toContain("Selected output</dt><dd>Model fit");
    expect(markup).toContain(">Model diagram</h2>");
    expect(markup).toContain("Model diagram unavailable");
    expect(markup).toContain("Open the matching Canvas model revision");
  });

  it("renders GSCA-specific fit and convergence properties without generic PLS settings", () => {
    const run = completedGscaRun();
    const gscaNavigation = buildNativeResultNavigation(run);
    const selectedItem = gscaNavigation.groups.find((group) => group.id === "gsca_component_model")!.items[0];
    const selectedTable = resultTableForItem(gscaNavigation, selectedItem.id);
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={vi.fn()}
      navigation={gscaNavigation}
      selectedItem={selectedItem}
      selectedTable={selectedTable}
      setSelectedTableId={vi.fn()}
      propertiesOpen
      openMethodDetails={vi.fn()}
    />);

    expect(markup).toContain("GSCA component model");
    expect(markup).toContain("Joint global least-squares ALS");
    expect(markup).toContain("Complete cases</dt><dd>140");
    expect(markup).toContain("Omitted cases</dt><dd>0");
    expect(markup).toContain("ALS iterations</dt><dd>4");
    expect(markup).toContain("Global FIT</dt><dd>0.369258");
    expect(markup).toContain("GFI</dt><dd>0.647993");
    expect(markup).not.toContain("Recorded seed");
    expect(markup).not.toContain("<dt>Iterations</dt>");
    expect(markup).toContain(">Method Details</button>");
  });

  it("renders CB-SEM/CFA run information from the ML payload rather than generic PLS iterations", () => {
    const run = completedCbsemRun("sem");
    const cbsemNavigation = buildNativeResultNavigation(run);
    const selectedItem = cbsemNavigation.groups.find((group) => group.id === "covariance_sem")!.items[0];
    const selectedTable = resultTableForItem(cbsemNavigation, selectedItem.id);
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={vi.fn()}
      navigation={cbsemNavigation}
      selectedItem={selectedItem}
      selectedTable={selectedTable}
      setSelectedTableId={vi.fn()}
      propertiesOpen
    />);

    expect(markup).toContain("CB-SEM / CFA");
    expect(markup).toContain("Recursive structural equation model");
    expect(markup).toContain("Maximum likelihood");
    expect(markup).toContain("Complete cases</dt><dd>120");
    expect(markup).toContain("Optimizer iterations</dt><dd>24");
    expect(markup).toContain("Gradient norm");
    expect(markup).not.toContain("<dt>Iterations</dt>");
  });

  it("renders an accessible observed-range NCA ceiling plot backed by the result table", () => {
    const markup = renderToStaticMarkup(<NcaCeilingPlot plot={{
      xLabel: "condition",
      yLabel: "outcome",
      ceiling: "both",
      scope: { minimumX: 1, maximumX: 8, minimumY: 1, maximumY: 9 },
      ceFdhPeers: [{ x: 1, y: 1 }, { x: 3, y: 2.5 }, { x: 6, y: 7 }, { x: 8, y: 9 }],
      crFdh: { slope: 1, intercept: 0 },
    }} />);

    expect(markup).toContain('class="nd-nca-plot"');
    expect(markup).toContain("<strong>Necessary condition ceiling plot</strong>");
    expect(markup).toContain("condition -&gt; outcome");
    expect(markup).toContain('role="img"');
    expect(markup).toContain('aria-labelledby="nd-nca-plot-title nd-nca-plot-description"');
    expect(markup).toContain("<title id=\"nd-nca-plot-title\">Necessary condition ceiling plot for condition and outcome</title>");
    expect(markup).toContain("Exact effect sizes and permutation p-values are listed in the table.");
    expect(markup).toMatch(/class="ceiling ce-fdh" d="M [^"]+ H [^"]+ V [^"]+/);
    expect(markup).toContain('class="ceiling cr-fdh"');
    expect(markup).toContain("Condition condition (observed values)");
    expect(markup).toContain("Outcome outcome (observed values)");
  });

  it("renders one roving tab stop with hierarchical and current-item semantics", () => {
    const run: AnalysisRun = {
      id: "run-1",
      name: "PLS Algorithm",
      method: "PLS Algorithm",
      createdAt: "2026-08-11T00:00:00Z",
      seed: 42,
      status: "completed",
      warnings: [],
      fingerprint: "fixture",
    };
    const table: ResultTable = {
      id: "path_coefficients",
      title: "Path coefficients",
      status: "validated",
      warning: null,
      columns: ["Path", "Coefficient"],
      rows: [["x → y", "0.5"]],
    };
    const selectedItem = navigation.groups[1].items[0];
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={vi.fn()}
      navigation={navigation}
      selectedItem={selectedItem}
      selectedTable={table}
      setSelectedTableId={vi.fn()}
      propertiesOpen={false}
    />);

    const treeItems = markup.match(/<button[^>]*role="treeitem"[^>]*>/g) ?? [];
    expect(treeItems.filter((item) => item.includes('tabindex="0"'))).toHaveLength(1);
    expect(treeItems.filter((item) => item.includes('tabindex="-1"'))).toHaveLength(4);
    const currentItem = markup.match(/<button[^>]*data-result-tree-item-id="path_coefficients"[^>]*>/)?.[0];
    expect(currentItem).toContain('role="treeitem"');
    expect(currentItem).toContain('aria-level="2"');
    expect(currentItem).toContain('aria-selected="true"');
    expect(currentItem).toContain('aria-current="page"');
    expect(currentItem).toContain('tabindex="0"');
    const group = markup.match(/<button[^>]*data-result-tree-item-id="final_results"[^>]*>/)?.[0];
    expect(group).toContain('aria-level="1"');
    expect(group).toContain('aria-expanded="true"');
    expect(group).toContain('tabindex="-1"');
    expect(markup).toContain('class="nd-result-table nd-scientific-grid"');
    expect(markup).toContain('data-result-table-id="path_coefficients"');
    expect(markup).toContain('role="grid"');
    expect(markup).toContain('aria-rowcount="2"');
    expect(markup).toContain('aria-colcount="2"');
    expect(markup).toContain('aria-keyshortcuts="Control+C"');
    const gridCells = markup.match(/<td[^>]*role="gridcell"[^>]*>/g) ?? [];
    expect(gridCells).toHaveLength(2);
    expect(gridCells.filter((cell) => cell.includes('aria-selected="true"'))).toHaveLength(1);
    expect(gridCells.filter((cell) => cell.includes('tabindex="0"'))).toHaveLength(1);
    expect(gridCells.filter((cell) => cell.includes('tabindex="-1"'))).toHaveLength(1);
  });

  it("renders presentation advisories as compact keyboard-accessible details and hides the legacy warning ribbon", () => {
    const run: AnalysisRun = {
      id: "run-advisory",
      name: "PLS Algorithm",
      method: "PLS Algorithm",
      createdAt: "2026-08-21T00:00:00Z",
      seed: 42,
      status: "completed",
      warnings: [],
      fingerprint: "fixture",
    };
    const table: ResultTable = {
      id: "path_coefficients",
      title: "Model fit — descriptive",
      status: "validated",
      warning: "Legacy export warning remains authoritative.",
      advisory: {
        tone: "info",
        title: "About these measures",
        message: "Full interpretation is available on demand.",
      },
      columns: ["Model", "SRMR"],
      rows: [["Estimated", "0.0700"]],
    };
    const selectedItem = navigation.groups[1].items[0];
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={vi.fn()}
      navigation={navigation}
      selectedItem={selectedItem}
      selectedTable={table}
      setSelectedTableId={vi.fn()}
      propertiesOpen={false}
    />);

    expect(markup).toContain('class="nd-result-advisory"');
    expect(markup).toContain('data-result-advisory-tone="info"');
    expect(markup).toContain('aria-label="About these measures. Show explanation"');
    expect(markup).toContain("Full interpretation is available on demand.");
    expect(markup).not.toContain("Legacy export warning remains authoritative.");
    expect(markup).not.toContain('class="nd-inline-warning"');
  });

  it("shows the bounded HOC model-fit note in researcher-facing run details", () => {
    const run = completedHigherOrderRun();
    const hocNavigation = buildNativeResultNavigation(run);
    const group = hocNavigation.groups.find((candidate) => candidate.id === "higher_order")!;
    const selectedItem = group.items.find((item) => item.id === "hoc_scope")!;
    const selectedTable = resultTableForItem(hocNavigation, selectedItem.id)!;
    const markup = renderToStaticMarkup(<NativeResultsSurface
      runs={[run]}
      selectedRun={run}
      selectedRunId={run.id}
      setSelectedRunId={vi.fn()}
      navigation={hocNavigation}
      selectedItem={selectedItem}
      selectedTable={selectedTable}
      setSelectedTableId={vi.fn()}
      propertiesOpen
    />);

    expect(markup).toContain("Higher-order constructs");
    expect(markup).toContain("Higher-order method and run details");
    expect(markup).toContain('data-result-advisory-tone="neutral"');
    expect(markup).toContain("Model fit not reported");
    expect(markup).toContain("Model fit</dt><dd>Not reported for this higher-order workflow");
  });

  it("moves among only visible entries with arrows and Home/End", () => {
    const entries = nativeVisibleResultTreeEntries(navigation, new Set());
    expect(entries.map((entry) => entry.id)).toEqual([
      "graphical",
      "model_estimates",
      "final_results",
      "path_coefficients",
      "r_squared",
    ]);
    expect(nativeResultTreeKeyboardAction(entries, "graphical", "ArrowDown")).toEqual({ focusId: "model_estimates" });
    expect(nativeResultTreeKeyboardAction(entries, "graphical", "ArrowUp")).toEqual({ focusId: "graphical" });
    expect(nativeResultTreeKeyboardAction(entries, "path_coefficients", "Home")).toEqual({ focusId: "graphical" });
    expect(nativeResultTreeKeyboardAction(entries, "path_coefficients", "End")).toEqual({ focusId: "r_squared" });
    expect(nativeResultTreeKeyboardAction(entries, "r_squared", "ArrowDown")).toEqual({ focusId: "r_squared" });

    const collapsed = nativeVisibleResultTreeEntries(navigation, new Set(["graphical"]));
    expect(collapsed.map((entry) => entry.id)).toEqual([
      "graphical",
      "final_results",
      "path_coefficients",
      "r_squared",
    ]);
    expect(nativeResultTreeKeyboardAction(collapsed, "graphical", "ArrowDown")).toEqual({ focusId: "final_results" });
  });

  it("expands, enters, collapses, returns to parents, and activates consistently", () => {
    const expanded = nativeVisibleResultTreeEntries(navigation, new Set());
    const collapsed = nativeVisibleResultTreeEntries(navigation, new Set(["final_results"]));

    expect(nativeResultTreeKeyboardAction(collapsed, "final_results", "ArrowRight")).toEqual({ toggleGroupId: "final_results" });
    expect(nativeResultTreeKeyboardAction(expanded, "final_results", "ArrowRight")).toEqual({ focusId: "path_coefficients" });
    expect(nativeResultTreeKeyboardAction(expanded, "final_results", "ArrowLeft")).toEqual({ toggleGroupId: "final_results" });
    expect(nativeResultTreeKeyboardAction(expanded, "path_coefficients", "ArrowLeft")).toEqual({ focusId: "final_results" });
    expect(nativeResultTreeKeyboardAction(expanded, "final_results", "Enter")).toEqual({ toggleGroupId: "final_results" });
    expect(nativeResultTreeKeyboardAction(expanded, "final_results", " ")).toEqual({ toggleGroupId: "final_results" });
    expect(nativeResultTreeKeyboardAction(expanded, "path_coefficients", "Enter")).toEqual({ activateItemId: "path_coefficients" });
    expect(nativeResultTreeKeyboardAction(expanded, "path_coefficients", " ")).toEqual({ activateItemId: "path_coefficients" });
  });
});
