import {
  BarChart3,
  ChevronDown,
  ChevronRight,
  FileSpreadsheet,
  FolderOpen,
  Maximize2,
} from "lucide-react";
import { useId, useMemo, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { publicationDiagramSvg } from "../domain/publicationDiagram";
import type { ResultTable } from "../domain/resultTables";
import { useWorkspace } from "../store";
import type { AnalysisRun, ProcessConditionalPlot, ProcessJohnsonNeymanAnalysis } from "../types";
import type {
  NativeResultNavigation,
  NativeResultNavigationGroup,
  NativeResultNavigationItem,
} from "./nativeResults";
import {
  CURRENT_CVPAT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION,
} from "./nativeCalculationMode";
import {
  nativeCbsemDiagramRun,
  nativeCbsemResultProjection,
  nativeGscaResultProjection,
  nativeIpmaPlot,
  nativeModerationPlot,
  nativeNcaCeilingLabel,
  nativeNcaPlot,
  nativeNcaResultProjection,
  nativeProcessResultProjection,
  type NativeIpmaPlot,
  type NativeModerationPlot,
  type NativeNcaPlot,
} from "./nativeResults";
import { nativeRunSettingApplicability } from "./nativeExportTables";
import { resolveAnalysisModel } from "./nativeRunModelSnapshot";
import { nativeGridClipboardText, useNativeScientificGrid } from "./nativeScientificGrid";

export interface NativeResultTreeEntry {
  id: string;
  kind: "group" | "item";
  parentId?: string;
  expanded?: boolean;
}

export interface NativeResultTreeKeyboardAction {
  focusId?: string;
  toggleGroupId?: string;
  activateItemId?: string;
}

export function nativeVisibleResultTreeEntries(
  navigation: NativeResultNavigation,
  collapsedGroupIds: ReadonlySet<string>,
): NativeResultTreeEntry[] {
  return navigation.groups.flatMap((group) => {
    const expanded = !collapsedGroupIds.has(group.id);
    return [
      { id: group.id, kind: "group" as const, expanded },
      ...(expanded
        ? group.items.map((item) => ({
          id: item.id,
          kind: "item" as const,
          parentId: group.id,
        }))
        : []),
    ];
  });
}

export function nativeResultTreeKeyboardAction(
  entries: readonly NativeResultTreeEntry[],
  currentId: string,
  key: string,
): NativeResultTreeKeyboardAction | null {
  if (!entries.length) return null;
  const currentIndex = entries.findIndex((entry) => entry.id === currentId);
  const index = currentIndex >= 0 ? currentIndex : 0;
  const current = entries[index];
  if (key === "ArrowDown") {
    return { focusId: entries[Math.min(index + 1, entries.length - 1)].id };
  }
  if (key === "ArrowUp") {
    return { focusId: entries[Math.max(index - 1, 0)].id };
  }
  if (key === "Home") return { focusId: entries[0].id };
  if (key === "End") return { focusId: entries[entries.length - 1].id };
  if (key === "ArrowRight" && current.kind === "group") {
    if (!current.expanded) return { toggleGroupId: current.id };
    const firstChild = entries[index + 1];
    return firstChild?.parentId === current.id ? { focusId: firstChild.id } : null;
  }
  if (key === "ArrowLeft") {
    if (current.kind === "group" && current.expanded) {
      return { toggleGroupId: current.id };
    }
    if (current.kind === "item" && current.parentId) {
      return { focusId: current.parentId };
    }
    return null;
  }
  if (key === "Enter" || key === " " || key === "Spacebar") {
    return current.kind === "group"
      ? { toggleGroupId: current.id }
      : { activateItemId: current.id };
  }
  return null;
}

export interface NativeResultsSurfaceProps {
  runs: AnalysisRun[];
  selectedRun?: AnalysisRun;
  selectedRunId: string;
  setSelectedRunId: (id: string) => void;
  navigation: NativeResultNavigation;
  selectedItem?: NativeResultNavigationItem;
  selectedTable?: ResultTable;
  setSelectedTableId: (id: string) => void;
  propertiesOpen: boolean;
}

export default function NativeResultsSurface({
  runs,
  selectedRun,
  selectedRunId,
  setSelectedRunId,
  navigation,
  selectedItem,
  selectedTable,
  setSelectedTableId,
  propertiesOpen,
}: NativeResultsSurfaceProps) {
  const [collapsedGroupIds, setCollapsedGroupIds] = useState<Set<string>>(() => new Set());
  const [focusedTreeItemId, setFocusedTreeItemId] = useState(
    () => selectedItem?.id ?? navigation.groups[0]?.id ?? "",
  );
  const visibleTreeItems = useMemo(
    () => nativeVisibleResultTreeEntries(navigation, collapsedGroupIds),
    [collapsedGroupIds, navigation],
  );
  const activeTreeItemId = visibleTreeItems.some((item) => item.id === focusedTreeItemId)
    ? focusedTreeItemId
    : visibleTreeItems.some((item) => item.id === selectedItem?.id)
      ? selectedItem?.id ?? ""
      : visibleTreeItems[0]?.id ?? "";
  const settingApplicability = selectedRun ? nativeRunSettingApplicability(selectedRun) : null;
  const ncaResult = selectedRun ? nativeNcaResultProjection(selectedRun) : null;
  const cbsemResult = selectedRun ? nativeCbsemResultProjection(selectedRun) : null;
  const gscaResult = selectedRun ? nativeGscaResultProjection(selectedRun) : null;
  const predictionV2 = selectedRun?.result?.predict?.method_version === CURRENT_PLS_PREDICT_METHOD_VERSION
    && selectedRun.result.predict.repeated_kfold?.method_version === CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION
    && /^sha256:[0-9a-f]{64}$/.test(selectedRun.result.predict.repeated_kfold.assignment_digest ?? "")
    && selectedRun.result.predict.repeated_kfold.cvpat_benchmark_assessments?.length === 2
    && new Set(selectedRun.result.predict.repeated_kfold.cvpat_benchmark_assessments.map((row) => row.benchmark)).size === 2
    && selectedRun.result.predict.repeated_kfold.cvpat_benchmark_assessments.every((row) => row.method_version === CURRENT_CVPAT_METHOD_VERSION)
    ? selectedRun.result.predict.repeated_kfold
    : null;
  const processResult = selectedRun ? nativeProcessResultProjection(selectedRun) : null;

  const toggleGroup = (groupId: string) => {
    setCollapsedGroupIds((current) => {
      const next = new Set(current);
      if (next.has(groupId)) next.delete(groupId);
      else next.add(groupId);
      return next;
    });
  };
  const focusTreeItem = (tree: HTMLDivElement, itemId: string) => {
    setFocusedTreeItemId(itemId);
    const item = Array.from(tree.querySelectorAll<HTMLElement>('[role="treeitem"]'))
      .find((candidate) => candidate.dataset.resultTreeItemId === itemId);
    item?.focus();
  };
  const handleTreeKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const target = (event.target as HTMLElement).closest?.<HTMLElement>('[role="treeitem"]');
    if (!target || !event.currentTarget.contains(target)) return;
    const currentId = target.dataset.resultTreeItemId;
    if (!currentId) return;
    const action = nativeResultTreeKeyboardAction(visibleTreeItems, currentId, event.key);
    if (!action) return;
    event.preventDefault();
    if (action.toggleGroupId) toggleGroup(action.toggleGroupId);
    if (action.activateItemId) setSelectedTableId(action.activateItemId);
    if (action.focusId) focusTreeItem(event.currentTarget, action.focusId);
  };

  return <div className={`nd-three-pane nd-results-workspace${propertiesOpen ? "" : " no-properties"}`}>
    <aside className="nd-navigator nd-results-nav" aria-label="Results navigation">
      <PaneTitle icon={<BarChart3 size={14} />} title="Results" />
      {runs.length ? <label className="nd-run-select">Run<select value={selectedRun?.id ?? selectedRunId} onChange={(event) => setSelectedRunId(event.target.value)}>{runs.map((run) => <option value={run.id} key={run.id}>{run.name}</option>)}</select></label> : null}
      {selectedRun ? <div className="nd-result-tree" role="tree" aria-label="Available result sections" onKeyDown={handleTreeKeyDown}>
        {navigation.groups.map((group) => <TreeGroup
          group={group}
          key={group.id}
          open={!collapsedGroupIds.has(group.id)}
          focusedItemId={activeTreeItemId}
          selectedItemId={selectedItem?.id}
          onFocusItem={setFocusedTreeItemId}
          onToggle={toggleGroup}
          onActivate={setSelectedTableId}
        />)}
      </div> : null}
    </aside>
    <section className="nd-document nd-results-document">
      <div className="nd-document-tab"><BarChart3 size={14} /><span>{selectedRun?.name ?? "Results"}</span></div>
      {!selectedRun ? <div className="nd-empty"><BarChart3 size={28} /><strong>No completed calculation</strong><span>Choose a method from Calculate to create results.</span></div> : selectedItem?.kind === "diagram" ? <ResultDiagramView run={selectedRun} /> : selectedTable ? <ResultTableView table={selectedTable} run={selectedRun} /> : <div className="nd-empty"><FileSpreadsheet size={28} /><strong>No available output</strong><span>The selected calculation did not produce this result.</span></div>}
    </section>
    {propertiesOpen ? <aside className="nd-properties" aria-label="Result properties">
      <PaneTitle title="Run information" />
      {processResult ? <dl className="nd-property-list">
        <div><dt>Method</dt><dd>Graph-defined Path Analysis / PROCESS</dd></div>
        <div><dt>Status</dt><dd>Completed</dd></div>
        <div><dt>Outcome</dt><dd>{processResult.outcome}</dd></div>
        <div><dt>Complete cases</dt><dd>{processResult.observations}</dd></div>
        <div><dt>Omitted cases</dt><dd>{processResult.omittedObservations}</dd></div>
        <div><dt>Equations</dt><dd>{processResult.graph.equations.length}</dd></div>
        <div><dt>Bootstrap</dt><dd>{processResult.bootstrap ? `${processResult.bootstrap.usable_replicates} / ${processResult.bootstrap.requested_replicates} usable` : "Off"}</dd></div>
        <div><dt>Recorded seed</dt><dd>{selectedRun!.provenance?.seed ?? selectedRun!.seed}</dd></div>
        <div><dt>Completed</dt><dd>{new Date(selectedRun!.createdAt).toLocaleString()}</dd></div>
      </dl> : ncaResult ? <dl className="nd-property-list">
        <div><dt>Method</dt><dd>Necessary Condition Analysis</dd></div>
        <div><dt>Status</dt><dd>Completed</dd></div>
        <div><dt>Condition (X)</dt><dd>{ncaResult.x}</dd></div>
        <div><dt>Outcome (Y)</dt><dd>{ncaResult.y}</dd></div>
        <div><dt>Observations</dt><dd>{ncaResult.observations}</dd></div>
        <div><dt>Ceiling lines</dt><dd>{nativeNcaCeilingLabel(ncaResult.ceiling)}</dd></div>
        <div><dt>Requested permutations</dt><dd>{ncaResult.permutationSamples}</dd></div>
        <div><dt>Usable permutations</dt><dd>{ncaResult.usablePermutations}</dd></div>
        <div><dt>Recorded seed</dt><dd>{selectedRun?.provenance?.seed ?? selectedRun?.seed}</dd></div>
        <div><dt>Completed</dt><dd>{new Date(selectedRun!.createdAt).toLocaleString()}</dd></div>
      </dl> : gscaResult ? <dl className="nd-property-list">
        <div><dt>Method</dt><dd>GSCA</dd></div>
        <div><dt>Status</dt><dd>Completed</dd></div>
        <div><dt>Estimator</dt><dd>Joint global least-squares ALS</dd></div>
        <div><dt>Complete cases</dt><dd>{gscaResult.usedObservations}</dd></div>
        <div><dt>Omitted cases</dt><dd>{gscaResult.omittedObservations}</dd></div>
        <div><dt>Converged</dt><dd>{gscaResult.analysis.converged ? "Yes" : "No"}</dd></div>
        <div><dt>ALS iterations</dt><dd>{gscaResult.analysis.iterations}</dd></div>
        <div><dt>Global FIT</dt><dd>{gscaResult.analysis.fit.toFixed(6)}</dd></div>
        <div><dt>Adjusted FIT</dt><dd>{gscaResult.analysis.adjusted_fit.toFixed(6)}</dd></div>
        <div><dt>GFI</dt><dd>{gscaResult.analysis.gfi.toFixed(6)}</dd></div>
        <div><dt>SRMR</dt><dd>{gscaResult.analysis.srmr.toFixed(6)}</dd></div>
        <div><dt>Completed</dt><dd>{new Date(selectedRun!.createdAt).toLocaleString()}</dd></div>
      </dl> : cbsemResult ? <dl className="nd-property-list">
        <div><dt>Method</dt><dd>CB-SEM / CFA</dd></div>
        <div><dt>Status</dt><dd>Completed</dd></div>
        <div><dt>Model type</dt><dd>{cbsemResult.modelType === "cfa" ? "Confirmatory factor analysis" : "Recursive structural equation model"}</dd></div>
        <div><dt>Estimator</dt><dd>Maximum likelihood</dd></div>
        <div><dt>Complete cases</dt><dd>{cbsemResult.analysis.sample_size}</dd></div>
        <div><dt>Converged</dt><dd>{cbsemResult.analysis.converged ? "Yes" : "No"}</dd></div>
        <div><dt>Optimizer iterations</dt><dd>{cbsemResult.analysis.iterations}</dd></div>
        <div><dt>Objective</dt><dd>{cbsemResult.analysis.objective.toFixed(6)}</dd></div>
        <div><dt>Gradient norm</dt><dd>{cbsemResult.analysis.gradient_norm.toExponential(3)}</dd></div>
        <div><dt>Completed</dt><dd>{new Date(selectedRun!.createdAt).toLocaleString()}</dd></div>
      </dl> : predictionV2 && selectedRun?.result ? <dl className="nd-property-list">
        <div><dt>Method</dt><dd>{selectedRun.method}</dd></div>
        <div><dt>Status</dt><dd>Completed</dd></div>
        <div><dt>Complete cases</dt><dd>{selectedRun.result.used_observations}</dd></div>
        <div><dt>Folds</dt><dd>{predictionV2.folds}</dd></div>
        <div><dt>Repeats</dt><dd>{predictionV2.repeats}</dd></div>
        <div><dt>Recorded seed</dt><dd>{predictionV2.seed ?? selectedRun.provenance?.seed ?? selectedRun.seed}</dd></div>
        <div><dt>CVPAT</dt><dd>One-sided, 95% confidence</dd></div>
        <div><dt>Completed</dt><dd>{new Date(selectedRun.createdAt).toLocaleString()}</dd></div>
      </dl> : selectedRun?.result ? <dl className="nd-property-list">
        <div><dt>Method</dt><dd>{selectedRun.method}</dd></div>
        <div><dt>Status</dt><dd>Completed</dd></div>
        <div><dt>Observations</dt><dd>{selectedRun.result.used_observations}</dd></div>
        <div><dt>Iterations</dt><dd>{selectedRun.result.iterations}</dd></div>
        {settingApplicability?.usesSeed ? <div><dt>Recorded seed</dt><dd>{selectedRun.provenance?.seed ?? selectedRun.seed}</dd></div> : null}
        <div><dt>Completed</dt><dd>{new Date(selectedRun.createdAt).toLocaleString()}</dd></div>
      </dl> : <div className="nd-pane-empty">No run selected.</div>}
    </aside> : null}
  </div>;
}

function ResultDiagramView({ run }: { run: AnalysisRun }) {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.publicationDiagramSettings);
  const layout = useWorkspace((state) => state.diagramLayout);
  const diagramRun = nativeCbsemDiagramRun(run);
  const svg = useMemo(() => {
    const model = resolveAnalysisModel(run, nodes, edges, layout);
    return publicationDiagramSvg(model.nodes, model.edges, diagramRun, {
      ...settings,
      showValidationWatermark: false,
      showUnsupportedWarning: false,
      showRunProvenance: false,
    }, model.diagramLayout);
  }, [diagramRun, edges, layout, nodes, run, settings]);
  const source = useMemo(() => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`, [svg]);
  const [zoom, setZoom] = useState(1);
  const diagramTitle = diagramRun === run ? "Model estimates" : "Standardized model estimates";
  return <section className="nd-result-diagram-view">
    <header><h1>{diagramTitle}</h1><div role="toolbar" aria-label={`${diagramTitle} view`}><button aria-label="Zoom out" onClick={() => setZoom((value) => Math.max(0.5, value - 0.1))}>-</button><button onClick={() => setZoom(1)}><Maximize2 size={13} />Fit</button><button aria-label="Zoom in" onClick={() => setZoom((value) => Math.min(2.5, value + 0.1))}>+</button><span aria-live="polite">{Math.round(zoom * 100)}%</span></div></header>
    <div className="nd-result-diagram-canvas" tabIndex={0} role="region" aria-label={`${diagramTitle} diagram`}>
      <img src={source} alt={`${diagramTitle} for ${run.name}`} style={{ width: `${zoom * 100}%` }} />
    </div>
  </section>;
}

function ResultTableView({ table, run }: { table: ResultTable; run: AnalysisRun }) {
  const moderationPlot = table.id === "moderation_simple_slopes" ? nativeModerationPlot(run) : null;
  const ipmaPlot = table.id === "ipma_constructs" ? nativeIpmaPlot(run) : null;
  const ncaPlot = table.id === "nca_ceiling_effects" ? nativeNcaPlot(run) : null;
  const process = nativeProcessResultProjection(run);
  const processConditionalPlots = table.id === "process_simple_slopes" ? process?.graph.plots ?? [] : [];
  const processJohnsonNeyman = table.id === "process_johnson_neyman"
    ? process?.graph.johnson_neyman.filter((row) => row.status === "available") ?? []
    : [];
  const grid = useNativeScientificGrid({
    gridKey: `${run.id}:${table.id}`,
    rowCount: table.rows.length,
    columnCount: table.columns.length,
    getClipboardText: ({ rowIndex, columnIndex }) => nativeGridClipboardText(table.rows[rowIndex]?.[columnIndex]),
  });
  const headingId = `nd-result-heading-${table.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  const instructionsId = `${headingId}-grid-instructions`;
  return <section className="nd-result-table-view" data-result-table-id={table.id}>
    <header><h1 id={headingId}>{table.title}</h1><span>{table.rows.length} row{table.rows.length === 1 ? "" : "s"}</span></header>
    {table.warning ? <div className="nd-inline-warning" role="status">{table.warning}</div> : null}
    {moderationPlot ? <ModerationSlopePlot plot={moderationPlot} /> : null}
    {ipmaPlot ? <IpmaScatterPlot plot={ipmaPlot} /> : null}
    {ncaPlot ? <NcaCeilingPlot plot={ncaPlot} /> : null}
    {processConditionalPlots.map((plot) => <ProcessConditionalPlotView key={plot.plot_id} plot={plot} outcome={process?.outcome ?? "Outcome"} />)}
    {processJohnsonNeyman.map((plot) => <ProcessJohnsonNeymanPlot key={`${plot.moderation_id}:${plot.solved_moderator}:${plot.conditioning_values.map((value) => value.raw_value).join(":")}`} plot={plot} />)}
    <div className="nd-table-scroll" role="region" aria-labelledby={headingId}>
      <table
        ref={grid.tableRef}
        className="nd-result-table nd-scientific-grid"
        role="grid"
        aria-labelledby={headingId}
        aria-describedby={instructionsId}
        aria-rowcount={table.rows.length + 1}
        aria-colcount={table.columns.length}
        aria-multiselectable="false"
        aria-keyshortcuts="Control+C"
        tabIndex={table.rows.length && table.columns.length ? undefined : 0}
        onKeyDown={grid.handleKeyDown}
      >
        <thead><tr role="row" aria-rowindex={1}>{table.columns.map((column, columnIndex) => <th key={column} role="columnheader" scope="col" aria-colindex={columnIndex + 1}>{column}</th>)}</tr></thead>
        <tbody>{table.rows.map((row, rowIndex) => <tr role="row" aria-rowindex={rowIndex + 2} key={rowIndex}>{row.map((cell, columnIndex) => <td
          {...grid.cellProps(rowIndex, columnIndex)}
          aria-colindex={columnIndex + 1}
          key={columnIndex}
        >{cell}</td>)}</tr>)}</tbody>
      </table>
    </div>
    <span className="nd-sr-only" id={instructionsId}>Use the arrow keys to move between cells. Press Control+C to copy the selected cell.</span>
    <span className="nd-sr-only" role="status" aria-live="polite">{grid.announcement}</span>
  </section>;
}

function processProbeLabel(values: readonly { variable: string; raw_value: number }[]): string {
  return values.length
    ? values.map((value) => `${value.variable} = ${formatPlotNumber(value.raw_value)}`).join(", ")
    : "Reference probe";
}

export function ProcessConditionalPlotView({
  plot,
  outcome,
}: {
  plot: ProcessConditionalPlot;
  outcome: string;
}) {
  const width = 680;
  const height = 290;
  const left = 66;
  const right = 24;
  const top = 35;
  const bottom = 52;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const points = plot.series.flatMap((series) => series.points);
  const minimumX = Math.min(...points.map((point) => point.predictor_raw));
  const maximumX = Math.max(...points.map((point) => point.predictor_raw));
  const minimumY = Math.min(...points.flatMap((point) => [point.predicted_raw, point.confidence_interval_lower]));
  const maximumY = Math.max(...points.flatMap((point) => [point.predicted_raw, point.confidence_interval_upper]));
  const yPadding = Math.max(1e-9, (maximumY - minimumY) * 0.08);
  const x = (value: number) => left + ((value - minimumX) / Math.max(Number.EPSILON, maximumX - minimumX)) * plotWidth;
  const y = (value: number) => top + ((maximumY + yPadding - value) / Math.max(Number.EPSILON, maximumY - minimumY + 2 * yPadding)) * plotHeight;
  const palette = ["#1f62b7", "#bf4b42", "#2b8a5a", "#8455a8", "#bb751c", "#2b7c85"];
  const seriesStyles = [
    { dash: undefined, marker: "circle", width: 2.4 },
    { dash: "8 3", marker: "square", width: 2.4 },
    { dash: "2 3", marker: "triangle", width: 2.4 },
    { dash: "10 3 2 3", marker: "circle", width: 2.8 },
    { dash: "6 2 1 2", marker: "square", width: 2.8 },
    { dash: "1 2", marker: "triangle", width: 2.8 },
    { dash: "12 3", marker: "circle", width: 3.2 },
    { dash: "4 2 1 2", marker: "square", width: 3.2 },
    { dash: "2 2 8 2", marker: "triangle", width: 3.2 },
  ] as const;
  const instanceId = useId();
  const titleId = `nd-process-conditional-title-${instanceId}`;
  const descriptionId = `nd-process-conditional-description-${instanceId}`;
  const description = `Engine-persisted conditional outcome data for ${plot.moderation_id}. ${plot.series.map((series) => {
    const first = series.points[0];
    const last = series.points.at(-1)!;
    return `${processProbeLabel(series.moderator_values)}: ${series.points.length} points from predictor ${formatPlotNumber(first.predictor_raw)}, predicted ${formatPlotNumber(first.predicted_raw)}, to predictor ${formatPlotNumber(last.predictor_raw)}, predicted ${formatPlotNumber(last.predicted_raw)}.`;
  }).join(" ")} Exact predicted values and confidence intervals are available in the adjacent conditional outcome plot data table.`;
  return <figure className="nd-process-result-plot" data-process-plot-id={plot.plot_id}>
    <figcaption><strong>Persisted conditional outcome plot</strong><span>{plot.moderation_id}</span></figcaption>
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-labelledby={`${titleId} ${descriptionId}`}>
      <title id={titleId}>{`Conditional outcome plot for ${plot.moderation_id}`}</title>
      <desc id={descriptionId}>{description}</desc>
      <line className="axis" x1={left} y1={height - bottom} x2={width - right} y2={height - bottom} />
      <line className="axis" x1={left} y1={top} x2={left} y2={height - bottom} />
      {[minimumX, (minimumX + maximumX) / 2, maximumX].map((tick) => <g key={`x-${tick}`}><line className="tick" x1={x(tick)} y1={height - bottom} x2={x(tick)} y2={height - bottom + 5} /><text x={x(tick)} y={height - bottom + 18} textAnchor="middle">{formatPlotNumber(tick)}</text></g>)}
      {[minimumY, (minimumY + maximumY) / 2, maximumY].map((tick) => <g key={`y-${tick}`}><line className="tick" x1={left - 5} y1={y(tick)} x2={left} y2={y(tick)} /><text x={left - 9} y={y(tick) + 4} textAnchor="end">{formatPlotNumber(tick)}</text></g>)}
      {plot.series.map((series, index) => {
        const color = palette[index % palette.length];
        const seriesStyle = seriesStyles[index % seriesStyles.length];
        const styleSignature = `${seriesStyle.dash ?? "solid"}|${seriesStyle.marker}|${seriesStyle.width}`;
        const mean = series.points.map((point) => `${x(point.predictor_raw)},${y(point.predicted_raw)}`).join(" ");
        const lower = series.points.map((point) => `${x(point.predictor_raw)},${y(point.confidence_interval_lower)}`).join(" ");
        const upper = series.points.map((point) => `${x(point.predictor_raw)},${y(point.confidence_interval_upper)}`).join(" ");
        return <g key={series.series_id} style={{ color }} data-process-series-style={styleSignature}>
          <polyline className="process-ci" points={lower} />
          <polyline className="process-ci" points={upper} />
          <polyline className="process-estimate" strokeDasharray={seriesStyle.dash} strokeWidth={seriesStyle.width} points={mean} />
          {series.points.map((point, pointIndex) => seriesStyle.marker === "square"
            ? <rect key={pointIndex} x={x(point.predictor_raw) - 2} y={y(point.predicted_raw) - 2} width={4} height={4}><title>{`${processProbeLabel(series.moderator_values)}; predictor ${formatPlotNumber(point.predictor_raw)}; predicted ${outcome} ${formatPlotNumber(point.predicted_raw)}; 95% CI ${formatPlotNumber(point.confidence_interval_lower)} to ${formatPlotNumber(point.confidence_interval_upper)}`}</title></rect>
            : seriesStyle.marker === "triangle"
              ? <path key={pointIndex} d={`M ${x(point.predictor_raw)} ${y(point.predicted_raw) - 3} l 3 6 h -6 z`}><title>{`${processProbeLabel(series.moderator_values)}; predictor ${formatPlotNumber(point.predictor_raw)}; predicted ${outcome} ${formatPlotNumber(point.predicted_raw)}; 95% CI ${formatPlotNumber(point.confidence_interval_lower)} to ${formatPlotNumber(point.confidence_interval_upper)}`}</title></path>
              : <circle key={pointIndex} cx={x(point.predictor_raw)} cy={y(point.predicted_raw)} r={2.2}><title>{`${processProbeLabel(series.moderator_values)}; predictor ${formatPlotNumber(point.predictor_raw)}; predicted ${outcome} ${formatPlotNumber(point.predicted_raw)}; 95% CI ${formatPlotNumber(point.confidence_interval_lower)} to ${formatPlotNumber(point.confidence_interval_upper)}`}</title></circle>)}
        </g>;
      })}
      <text className="axis-label" x={left + plotWidth / 2} y={height - 8} textAnchor="middle">Focal predictor (raw value)</text>
      <text className="axis-label" transform={`translate(14 ${top + plotHeight / 2}) rotate(-90)`} textAnchor="middle">Predicted {outcome} (raw value)</text>
    </svg>
    <ul className="nd-process-plot-legend" aria-label={`Series legend for ${plot.moderation_id}`}>
      {plot.series.map((series, index) => {
        const seriesStyle = seriesStyles[index % seriesStyles.length];
        const signature = `${seriesStyle.dash ?? "solid"}|${seriesStyle.marker}|${seriesStyle.width}`;
        return <li key={series.series_id} data-process-legend-style={signature}>
          <svg viewBox="0 0 28 8" aria-hidden="true"><line className="process-estimate" style={{ color: palette[index % palette.length] }} strokeDasharray={seriesStyle.dash} strokeWidth={seriesStyle.width} x1={1} y1={4} x2={27} y2={4} /></svg>
          <span>{processProbeLabel(series.moderator_values)}; {seriesStyle.marker} markers; {seriesStyle.dash ?? "solid"} line; width {seriesStyle.width}</span>
        </li>;
      })}
    </ul>
    <p>All 25 points per series and their intervals were produced by the engine at original-sample raw moderator probes.</p>
  </figure>;
}

export function ProcessJohnsonNeymanPlot({
  plot,
}: {
  plot: Extract<ProcessJohnsonNeymanAnalysis, { status: "available" }>;
}) {
  const width = 680;
  const height = 280;
  const left = 66;
  const right = 24;
  const top = 30;
  const bottom = 50;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const minimumY = Math.min(0, ...plot.curve_points.map((point) => point.confidence_interval_lower));
  const maximumY = Math.max(0, ...plot.curve_points.map((point) => point.confidence_interval_upper));
  const padding = Math.max(1e-9, (maximumY - minimumY) * 0.08);
  const x = (value: number) => left + ((value - plot.raw_min) / Math.max(Number.EPSILON, plot.raw_max - plot.raw_min)) * plotWidth;
  const y = (value: number) => top + ((maximumY + padding - value) / Math.max(Number.EPSILON, maximumY - minimumY + 2 * padding)) * plotHeight;
  const estimate = plot.curve_points.map((point) => `${x(point.moderator_raw)},${y(point.effect)}`).join(" ");
  const lower = plot.curve_points.map((point) => `${x(point.moderator_raw)},${y(point.confidence_interval_lower)}`).join(" ");
  const upper = plot.curve_points.map((point) => `${x(point.moderator_raw)},${y(point.confidence_interval_upper)}`).join(" ");
  const instanceId = useId();
  const titleId = `nd-process-jn-title-${instanceId}`;
  const descriptionId = `nd-process-jn-description-${instanceId}`;
  const description = `Engine-persisted Johnson-Neyman curve for ${plot.moderation_id}, solved moderator ${plot.solved_moderator}${plot.conditioning_values.length ? `, conditioned at ${processProbeLabel(plot.conditioning_values)}` : ""}. ${plot.curve_points.length} curve points span raw ${formatPlotNumber(plot.raw_min)} to ${formatPlotNumber(plot.raw_max)}. Roots: ${plot.roots.length ? plot.roots.map(formatPlotNumber).join(", ") : "none"}. Regions: ${plot.regions.map((region) => `${formatPlotNumber(region.lower)} to ${formatPlotNumber(region.upper)} ${region.status.replaceAll("_", " ")}`).join("; ")}. Exact effect, SE, and confidence bounds are available in the adjacent Johnson-Neyman curve data table.`;
  return <figure className="nd-process-result-plot" data-process-jn-moderation={plot.moderation_id}>
    <figcaption><strong>Persisted Johnson-Neyman curve</strong><span>{plot.solved_moderator}{plot.conditioning_values.length ? ` at ${processProbeLabel(plot.conditioning_values)}` : ""}</span></figcaption>
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-labelledby={`${titleId} ${descriptionId}`}>
      <title id={titleId}>{`Johnson-Neyman curve for ${plot.moderation_id}`}</title>
      <desc id={descriptionId}>{description}</desc>
      <line className="axis" x1={left} y1={y(0)} x2={width - right} y2={y(0)} />
      <line className="axis" x1={left} y1={top} x2={left} y2={height - bottom} />
      {[plot.raw_min, (plot.raw_min + plot.raw_max) / 2, plot.raw_max].map((tick) => <g key={`x-${tick}`}><line className="tick" x1={x(tick)} y1={height - bottom} x2={x(tick)} y2={height - bottom + 5} /><text x={x(tick)} y={height - bottom + 18} textAnchor="middle">{formatPlotNumber(tick)}</text></g>)}
      {plot.roots.map((root) => <line key={root} className="process-root" x1={x(root)} y1={top} x2={x(root)} y2={height - bottom}><title>{`Johnson-Neyman root ${formatPlotNumber(root)}`}</title></line>)}
      <polyline className="process-ci" points={lower} />
      <polyline className="process-ci" points={upper} />
      <polyline className="process-estimate" points={estimate} />
      {plot.curve_points.map((point, index) => <circle key={index} cx={x(point.moderator_raw)} cy={y(point.effect)} r={1.8}><title>{`${plot.solved_moderator} ${formatPlotNumber(point.moderator_raw)}; effect ${formatPlotNumber(point.effect)}; SE ${formatPlotNumber(point.standard_error)}; 95% CI ${formatPlotNumber(point.confidence_interval_lower)} to ${formatPlotNumber(point.confidence_interval_upper)}`}</title></circle>)}
      <text className="axis-label" x={left + plotWidth / 2} y={height - 7} textAnchor="middle">{plot.solved_moderator} (original raw range)</text>
      <text className="axis-label" transform={`translate(14 ${top + plotHeight / 2}) rotate(-90)`} textAnchor="middle">Conditional effect</text>
    </svg>
    <p>All 101 curve points, intervals, roots, and regions were persisted by the engine; the UI only scales them for display.</p>
  </figure>;
}

export function NcaCeilingPlot({ plot }: { plot: NativeNcaPlot }) {
  const width = 680;
  const height = 300;
  const left = 72;
  const right = 24;
  const top = 42;
  const bottom = 50;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const { minimumX, maximumX, minimumY, maximumY } = plot.scope;
  const x = (value: number) => left + ((value - minimumX) / (maximumX - minimumX)) * plotWidth;
  const y = (value: number) => top + ((maximumY - value) / (maximumY - minimumY)) * plotHeight;
  const ceilingPath = plot.ceFdhPeers.reduce((path, peer, index) => {
    if (index === 0) return `M ${x(peer.x)} ${y(peer.y)}`;
    return `${path} H ${x(peer.x)} V ${y(peer.y)}`;
  }, "") + ` H ${x(maximumX)}`;
  const xTicks = [minimumX, (minimumX + maximumX) / 2, maximumX];
  const yTicks = [minimumY, (minimumY + maximumY) / 2, maximumY];
  const showCeFdh = plot.ceiling === "ce_fdh" || plot.ceiling === "both";
  const titleId = "nd-nca-plot-title";
  const descriptionId = "nd-nca-plot-description";
  const crDescription = plot.crFdh
    ? ` CR-FDH slope ${formatPlotNumber(plot.crFdh.slope)} and intercept ${formatPlotNumber(plot.crFdh.intercept)}.`
    : "";
  const description = `Observed-range necessary condition ceiling plot for ${plot.xLabel} as X and ${plot.yLabel} as Y. ${plot.ceFdhPeers.map((peer) => `CE-FDH peer ${formatPlotNumber(peer.x)}, ${formatPlotNumber(peer.y)}.`).join(" ")}${crDescription} Exact effect sizes and permutation p-values are listed in the table.`;
  return <figure className="nd-nca-plot">
    <figcaption><strong>Necessary condition ceiling plot</strong><span>{plot.xLabel} -&gt; {plot.yLabel}</span></figcaption>
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-labelledby={`${titleId} ${descriptionId}`}>
      <title id={titleId}>{`Necessary condition ceiling plot for ${plot.xLabel} and ${plot.yLabel}`}</title>
      <desc id={descriptionId}>{description}</desc>
      <defs><clipPath id="nd-nca-plot-clip"><rect x={left} y={top} width={plotWidth} height={plotHeight} /></clipPath></defs>
      <line className="axis" x1={left} y1={height - bottom} x2={width - right} y2={height - bottom} />
      <line className="axis" x1={left} y1={top} x2={left} y2={height - bottom} />
      {xTicks.map((tick) => <g key={`x-${tick}`}><line className="tick" x1={x(tick)} y1={height - bottom} x2={x(tick)} y2={height - bottom + 5} /><text x={x(tick)} y={height - bottom + 18} textAnchor="middle">{formatPlotNumber(tick)}</text></g>)}
      {yTicks.map((tick) => <g key={`y-${tick}`}><line className="tick" x1={left - 5} y1={y(tick)} x2={left} y2={y(tick)} /><text x={left - 9} y={y(tick) + 4} textAnchor="end">{formatPlotNumber(tick)}</text></g>)}
      <g clipPath="url(#nd-nca-plot-clip)">
        {showCeFdh ? <path className="ceiling ce-fdh" d={ceilingPath} /> : null}
        {showCeFdh ? plot.ceFdhPeers.map((peer) => <circle className="ce-peer" key={`${peer.x}-${peer.y}`} cx={x(peer.x)} cy={y(peer.y)} r={3.5}><title>{`CE-FDH peer: ${plot.xLabel} ${formatPlotNumber(peer.x)}, ${plot.yLabel} ${formatPlotNumber(peer.y)}`}</title></circle>) : null}
        {plot.crFdh ? <line className="ceiling cr-fdh" x1={x(minimumX)} y1={y(plot.crFdh.slope * minimumX + plot.crFdh.intercept)} x2={x(maximumX)} y2={y(plot.crFdh.slope * maximumX + plot.crFdh.intercept)} /> : null}
      </g>
      <g className="legend" aria-hidden="true">
        {showCeFdh ? <><line className="ceiling ce-fdh" x1={left} y1={17} x2={left + 22} y2={17} /><text x={left + 28} y={20}>CE-FDH</text></> : null}
        {plot.crFdh ? <><line className="ceiling cr-fdh" x1={left + 92} y1={17} x2={left + 114} y2={17} /><text x={left + 120} y={20}>CR-FDH</text></> : null}
      </g>
      <text className="axis-label" x={left + plotWidth / 2} y={height - 7} textAnchor="middle">Condition {plot.xLabel} (observed values)</text>
      <text className="axis-label" transform={`translate(15 ${top + plotHeight / 2}) rotate(-90)`} textAnchor="middle">Outcome {plot.yLabel} (observed values)</text>
    </svg>
  </figure>;
}

function formatPlotNumber(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(4).replace(/0+$/, "").replace(/\.$/, "");
}

function IpmaScatterPlot({ plot }: { plot: NativeIpmaPlot }) {
  const width = 680;
  const height = 280;
  const left = 66;
  const right = 28;
  const top = 25;
  const bottom = 48;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const importanceValues = plot.points.map((point) => point.importance);
  const rawMinimumImportance = Math.min(0, ...importanceValues);
  const rawMaximumImportance = Math.max(0, ...importanceValues);
  const importancePadding = Math.max(0.05, (rawMaximumImportance - rawMinimumImportance) * 0.08);
  const minimumImportance = rawMinimumImportance - importancePadding;
  const maximumImportance = rawMaximumImportance + importancePadding;
  const x = (value: number) => left + ((value - minimumImportance) / Math.max(Number.EPSILON, maximumImportance - minimumImportance)) * plotWidth;
  const y = (value: number) => top + ((100 - value) / 100) * plotHeight;
  const meanImportance = plot.points.reduce((sum, point) => sum + point.importance, 0) / plot.points.length;
  const meanPerformance = plot.points.reduce((sum, point) => sum + point.performance, 0) / plot.points.length;
  const importanceTicks = [rawMinimumImportance, 0, rawMaximumImportance]
    .filter((tick, index, all) => all.findIndex((candidate) => Math.abs(candidate - tick) < 1e-12) === index);
  const titleId = "nd-ipma-plot-title";
  const descriptionId = "nd-ipma-plot-description";
  const description = `${plot.scopeNote} ${plot.points.map((point) => `${point.constructLabel}: importance ${point.importance.toFixed(6)}, performance ${point.performance.toFixed(4)}.`).join(" ")}`;
  return <figure className="nd-ipma-plot">
    <figcaption><strong>Importance-performance map</strong><span>Target: {plot.targetLabel}</span></figcaption>
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-labelledby={`${titleId} ${descriptionId}`}>
      <title id={titleId}>{`Importance-performance map for ${plot.targetLabel}`}</title>
      <desc id={descriptionId}>{description}</desc>
      <line className="axis" x1={left} y1={height - bottom} x2={width - right} y2={height - bottom} />
      <line className="axis" x1={left} y1={top} x2={left} y2={height - bottom} />
      <line className="reference" x1={x(meanImportance)} y1={top} x2={x(meanImportance)} y2={height - bottom} />
      <line className="reference" x1={left} y1={y(meanPerformance)} x2={width - right} y2={y(meanPerformance)} />
      {importanceTicks.map((tick) => <g key={`x-${tick}`}><line className="tick" x1={x(tick)} y1={height - bottom} x2={x(tick)} y2={height - bottom + 5} /><text x={x(tick)} y={height - bottom + 18} textAnchor="middle">{tick.toFixed(2)}</text></g>)}
      {[0, 25, 50, 75, 100].map((tick) => <g key={`y-${tick}`}><line className="tick" x1={left - 5} y1={y(tick)} x2={left} y2={y(tick)} /><text x={left - 9} y={y(tick) + 4} textAnchor="end">{tick}</text></g>)}
      {plot.points.map((point) => <g key={point.constructId}>
        <circle cx={x(point.importance)} cy={y(point.performance)} r={4}><title>{`${point.constructLabel}: importance ${point.importance.toFixed(6)}, performance ${point.performance.toFixed(4)}`}</title></circle>
        <text className="point-label" x={x(point.importance) + 7} y={y(point.performance) - 6}>{point.constructLabel}</text>
      </g>)}
      <text className="axis-label" x={left + plotWidth / 2} y={height - 7} textAnchor="middle">Total importance for {plot.targetLabel}</text>
      <text className="axis-label" transform={`translate(14 ${top + plotHeight / 2}) rotate(-90)`} textAnchor="middle">Performance (0-100)</text>
    </svg>
    <p>{plot.scopeNote}</p>
  </figure>;
}

function ModerationSlopePlot({ plot }: { plot: NativeModerationPlot }) {
  const width = 680;
  const height = 250;
  const left = 62;
  const right = 22;
  const top = 28;
  const bottom = 48;
  const plotWidth = width - left - right;
  const plotHeight = height - top - bottom;
  const scores = plot.slopes.map((slope) => slope.moderatorScore);
  const effects = plot.slopes.map((slope) => slope.effect);
  const minimumScore = Math.min(...scores);
  const maximumScore = Math.max(...scores);
  const rawMinimumEffect = Math.min(0, ...effects);
  const rawMaximumEffect = Math.max(0, ...effects);
  const effectPadding = Math.max(0.05, (rawMaximumEffect - rawMinimumEffect) * 0.12);
  const minimumEffect = rawMinimumEffect - effectPadding;
  const maximumEffect = rawMaximumEffect + effectPadding;
  const x = (value: number) => left + ((value - minimumScore) / Math.max(Number.EPSILON, maximumScore - minimumScore)) * plotWidth;
  const y = (value: number) => top + ((maximumEffect - value) / Math.max(Number.EPSILON, maximumEffect - minimumEffect)) * plotHeight;
  const points = plot.slopes.map((slope) => `${x(slope.moderatorScore)},${y(slope.effect)}`).join(" ");
  const titleId = "nd-moderation-plot-title";
  const descriptionId = "nd-moderation-plot-description";
  return <figure className="nd-moderation-plot">
    <figcaption><strong>Conditional effect plot</strong><span>{plot.title}</span></figcaption>
    <svg viewBox={`0 0 ${width} ${height}`} role="img" aria-labelledby={`${titleId} ${descriptionId}`}>
      <title id={titleId}>Conditional effect plot for {plot.title}</title>
      <desc id={descriptionId}>The exact conditional effect of the predictor on the outcome at each reported moderator score. The same values are listed in the table.</desc>
      <line className="axis" x1={left} y1={y(0)} x2={width - right} y2={y(0)} />
      <line className="axis" x1={left} y1={top} x2={left} y2={height - bottom} />
      {plot.slopes.map((slope) => <g key={`x-${slope.moderatorScore}`}><line className="tick" x1={x(slope.moderatorScore)} y1={height - bottom} x2={x(slope.moderatorScore)} y2={height - bottom + 5} /><text x={x(slope.moderatorScore)} y={height - bottom + 18} textAnchor="middle">{slope.moderatorScore > 0 ? `+${slope.moderatorScore}` : String(slope.moderatorScore)}</text></g>)}
      {[rawMinimumEffect, 0, rawMaximumEffect].filter((tick, index, all) => all.findIndex((candidate) => Math.abs(candidate - tick) < 1e-12) === index).map((tick) => <g key={`y-${tick}`}><line className="tick" x1={left - 5} y1={y(tick)} x2={left} y2={y(tick)} /><text x={left - 9} y={y(tick) + 4} textAnchor="end">{tick.toFixed(2)}</text></g>)}
      <polyline className="slope" points={points} />
      {plot.slopes.map((slope) => <circle key={`${slope.moderatorScore}-${slope.effect}`} cx={x(slope.moderatorScore)} cy={y(slope.effect)} r={3.5}><title>{`${slope.label}: conditional effect ${slope.effect.toFixed(6)}`}</title></circle>)}
      <text className="axis-label" x={left + plotWidth / 2} y={height - 7} textAnchor="middle">{plot.moderatorLabel} (standardized score)</text>
      <text className="axis-label" transform={`translate(14 ${top + plotHeight / 2}) rotate(-90)`} textAnchor="middle">Effect of {plot.predictorLabel} on {plot.outcomeLabel}</text>
    </svg>
  </figure>;
}

function TreeGroup({
  group,
  open,
  focusedItemId,
  selectedItemId,
  onFocusItem,
  onToggle,
  onActivate,
}: {
  group: NativeResultNavigationGroup;
  open: boolean;
  focusedItemId: string;
  selectedItemId?: string;
  onFocusItem: (id: string) => void;
  onToggle: (id: string) => void;
  onActivate: (id: string) => void;
}) {
  return <div className="nd-tree-group" role="none">
    <button
      type="button"
      role="treeitem"
      aria-level={1}
      aria-expanded={open}
      tabIndex={focusedItemId === group.id ? 0 : -1}
      data-result-tree-item-id={group.id}
      onFocus={() => onFocusItem(group.id)}
      onClick={() => {
        onFocusItem(group.id);
        onToggle(group.id);
      }}
    >{open ? <ChevronDown size={13} /> : <ChevronRight size={13} />}<FolderOpen size={13} /><strong>{group.title}</strong></button>
    {open ? <div role="group">{group.items.map((item) => {
      const selected = selectedItemId === item.id;
      return <button
        type="button"
        role="treeitem"
        aria-level={2}
        aria-selected={selected}
        aria-current={selected ? "page" : undefined}
        tabIndex={focusedItemId === item.id ? 0 : -1}
        data-result-tree-item-id={item.id}
        key={item.id}
        className={selected ? "active" : ""}
        onFocus={() => onFocusItem(item.id)}
        onClick={() => {
          onFocusItem(item.id);
          onActivate(item.id);
        }}
      >{item.title}</button>;
    })}</div> : null}
  </div>;
}

function PaneTitle({ title, icon }: { title: string; icon?: ReactNode }) {
  return <header className="nd-pane-title">{icon}<strong>{title}</strong></header>;
}
