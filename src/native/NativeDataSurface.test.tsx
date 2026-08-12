import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "../store";
import { NativeDataSurface } from "./NativeDataSurface";

const baselineAnalysisSettings = { ...useWorkspace.getState().analysisSettings };

afterEach(() => {
  vi.unstubAllGlobals();
  useWorkspace.setState({ analysisSettings: { ...baselineAnalysisSettings } });
});

describe("native Data scientific grid", () => {
  it("renders one roving data-cell stop with virtualized grid semantics", () => {
    vi.stubGlobal("window", {});
    const dataset = useWorkspace.getState().dataset;
    const selectedColumn = dataset.columns[0];
    const markup = renderToStaticMarkup(<NativeDataSurface
      selectedColumn={selectedColumn}
      setSelectedColumn={vi.fn()}
      groupColumn={null}
      propertiesOpen={false}
      mutationsLocked={false}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain('class="nd-data-table nd-scientific-grid"');
    expect(markup).toContain('role="grid"');
    expect(markup).toContain(`aria-rowcount="${(dataset.rowCount ?? dataset.rows.length) + 1}"`);
    expect(markup).toContain(`aria-colcount="${dataset.columns.length + 1}"`);
    expect(markup).toContain('aria-keyshortcuts="Control+C"');
    const gridCells = markup.match(/<td[^>]*role="gridcell"[^>]*>/g) ?? [];
    expect(gridCells).toHaveLength(dataset.rows.length * dataset.columns.length);
    expect(gridCells.filter((cell) => cell.includes('aria-selected="true"'))).toHaveLength(1);
    expect(gridCells.filter((cell) => cell.includes('tabindex="0"'))).toHaveLength(1);
    expect(gridCells.filter((cell) => cell.includes('tabindex="-1"'))).toHaveLength(gridCells.length - 1);
  });

  it("marks the configured grouping variable in navigation, headers, and properties", () => {
    vi.stubGlobal("window", {});
    const dataset = useWorkspace.getState().dataset;
    const selectedColumn = dataset.columns[0];
    const markup = renderToStaticMarkup(<NativeDataSurface
      selectedColumn={selectedColumn}
      setSelectedColumn={vi.fn()}
      groupColumn={selectedColumn}
      propertiesOpen
      mutationsLocked={false}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain("configured grouping variable");
    expect(markup).toContain(">Groups</small>");
    expect(markup).toContain("Grouping variable</span>");
  });
});
