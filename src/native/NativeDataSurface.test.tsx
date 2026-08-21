import { renderToStaticMarkup } from "react-dom/server";
import { readFileSync } from "node:fs";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "../store";
import { NativeDataSurface } from "./NativeDataSurface";

const baselineAnalysisSettings = { ...useWorkspace.getState().analysisSettings };

afterEach(() => {
  vi.unstubAllGlobals();
  useWorkspace.setState({ analysisSettings: { ...baselineAnalysisSettings }, datasetDescriptorOnly: false });
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
      hasEditableModel
      projectWritable
      mutationsLocked={false}
      onNewModel={vi.fn()}
      onAnalyze={vi.fn()}
      onDerive={vi.fn()}
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
      hasEditableModel
      projectWritable
      mutationsLocked={false}
      onNewModel={vi.fn()}
      onAnalyze={vi.fn()}
      onDerive={vi.fn()}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain("configured grouping variable");
    expect(markup).toContain(">Groups</small>");
    expect(markup).toContain("Grouping variable</span>");
  });

  it("presents explicit model and model-free next actions after importing into an empty project", () => {
    vi.stubGlobal("window", {});
    const dataset = useWorkspace.getState().dataset;
    const markup = renderToStaticMarkup(<NativeDataSurface
      selectedColumn={dataset.columns[0]}
      setSelectedColumn={vi.fn()}
      groupColumn={null}
      propertiesOpen={false}
      hasEditableModel={false}
      projectWritable
      mutationsLocked={false}
      onNewModel={vi.fn()}
      onAnalyze={vi.fn()}
      onDerive={vi.fn()}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain('aria-labelledby="nd-data-next-actions-title"');
    expect(markup).toContain("Choose what to do next");
    expect(markup).toContain(">New Model…</button>");
    expect(markup).toContain(">Analyze…</button>");
    expect(markup).toContain('aria-label="Next actions for imported data"');
  });

  it("keeps next actions visible but disabled with an actionable read-only explanation", () => {
    vi.stubGlobal("window", {});
    const dataset = useWorkspace.getState().dataset;
    const markup = renderToStaticMarkup(<NativeDataSurface
      selectedColumn={dataset.columns[0]}
      setSelectedColumn={vi.fn()}
      groupColumn={null}
      propertiesOpen={false}
      hasEditableModel={false}
      projectWritable={false}
      mutationsLocked={false}
      onNewModel={vi.fn()}
      onAnalyze={vi.fn()}
      onDerive={vi.fn()}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain("This project is read-only. Save a writable copy before starting new work.");
    expect(markup.match(/<button[^>]*disabled=""[^>]*>/g)).toHaveLength(3);
  });

  it("offers the non-destructive derive workflow from every data view", () => {
    vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
    const dataset = useWorkspace.getState().dataset;
    const markup = renderToStaticMarkup(<NativeDataSurface
      selectedColumn={dataset.columns[0]}
      setSelectedColumn={vi.fn()}
      groupColumn={null}
      propertiesOpen={false}
      hasEditableModel
      projectWritable
      mutationsLocked={false}
      onNewModel={vi.fn()}
      onAnalyze={vi.fn()}
      onDerive={vi.fn()}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain("Derive variable…");
    expect(markup).toContain("Create a non-destructive derived variable");
  });

  it("renders archive-bound metadata as read-only before any save handler can run", () => {
    vi.stubGlobal("window", {});
    const dataset = useWorkspace.getState().dataset;
    const markup = renderToStaticMarkup(<NativeDataSurface
      selectedColumn={dataset.columns[0]}
      setSelectedColumn={vi.fn()}
      groupColumn={null}
      propertiesOpen
      hasEditableModel
      projectWritable={false}
      mutationsLocked
      onNewModel={vi.fn()}
      onAnalyze={vi.fn()}
      onDerive={vi.fn()}
      onContextMenuRequest={() => false}
    />);

    expect(markup).toContain("This General SEM archive is read-only");
    expect(markup).toMatch(/<input[^>]*disabled=""/);
    expect(markup).toMatch(/<select[^>]*disabled=""/);
    expect(markup).toMatch(/<button[^>]*class="primary"[^>]*disabled=""/);
  });

  it("never invents missing-value counts for a descriptor-only General SEM dataset", () => {
    const source = readFileSync("src/native/NativeDataSurface.tsx", "utf8");
    expect(source).toContain("const dataQualityAvailable = !datasetDescriptorOnly && Number.isFinite(dataset.missing)");
    expect(source).toContain('missingCount={dataQualityAvailable ? missingCounts.get(selectedColumn) ?? 0 : null}');
    expect(source).toContain('missingCount == null ? "Not stored" : missingCount.toLocaleString()');
  });
});
