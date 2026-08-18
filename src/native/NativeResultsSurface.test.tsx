import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
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

describe("native Results tree accessibility", () => {
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
