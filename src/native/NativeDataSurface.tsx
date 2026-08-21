import {
  Check,
  ChevronDown,
  ChevronsLeft,
  ChevronsRight,
  ChevronLeft,
  ChevronRight,
  CircleGauge,
  Database,
  FileSpreadsheet,
  FileText,
  History,
  LoaderCircle,
  Plus,
  RefreshCw,
  Search,
  Square,
  Table2,
  UsersRound,
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactNode,
} from "react";
import { activateNativeDataset, getNativeDatasetRows, isNativeDesktop, updateNativeColumnMetadata } from "../services/projectService";
import { readInternalProjectArchiveV6DatasetRows } from "../services/internalProjectArchiveV6DatasetRowsService";
import { useInternalProjectArchiveV6Session } from "../internalProjectArchiveV6SessionStore";
import { useWorkspace } from "../store";
import type { ColumnMetadata, Dataset } from "../types";
import {
  DEFAULT_NATIVE_DATA_PAGE_SIZE,
  NATIVE_DATA_PAGE_SIZES,
  nativeDataPage,
  nativeDataPageRows,
  nativeDataRangeLabel,
  nativeMissingCounts,
} from "./nativeDataGrid";
import { nativeDatasetOperationLabel, nativeDatasetVersionItems } from "./nativeDatasetVersions";
import type { NativeDataContextMenuRequest, NativeDataContextTarget } from "./nativeDataContext";
import { isContextMenuKeyboardGesture } from "./nativeMenuNavigation";
import { nativeGridClipboardText, useNativeScientificGrid } from "./nativeScientificGrid";
import { readNativeDatasetPageV1 } from "./nativeDatasetRowPaging";
import {
  defaultNativeColumnMetadata,
  nativeVariableMetadataDraft,
  validateNativeVariableMetadata,
  type NativeVariableMetadataDraft,
} from "./nativeVariableMetadata";

interface NativeDataSurfaceProps {
  selectedColumn: string;
  setSelectedColumn: (column: string) => void;
  groupColumn: string | null;
  propertiesOpen: boolean;
  hasEditableModel: boolean;
  projectWritable: boolean;
  mutationsLocked: boolean;
  onNewModel: () => void;
  onAnalyze: () => void;
  onDerive: () => void;
  onContextMenuRequest: (request: NativeDataContextMenuRequest) => boolean;
}
type NativeDataMode = "data" | "variables" | "quality" | "import";

interface NativePageState {
  key: string;
  status: "loading" | "ready" | "error";
  rows: Dataset["rows"];
  error?: string;
}


function dataContextTarget(target: EventTarget | null, root: HTMLElement): NativeDataContextTarget {
  if (!(target instanceof Element)) return { kind: "none" };
  const variable = target.closest<HTMLElement>("[data-native-variable]");
  if (variable && root.contains(variable)) {
    const column = variable.dataset.nativeVariable;
    if (column) return { kind: "variable", column };
  }
  const datasetTarget = target.closest<HTMLElement>("[data-native-dataset]");
  return datasetTarget && root.contains(datasetTarget) ? { kind: "dataset" } : { kind: "none" };
}

export function NativeDataSurface({
  selectedColumn,
  setSelectedColumn,
  groupColumn: configuredGroupColumn,
  propertiesOpen,
  hasEditableModel,
  projectWritable,
  mutationsLocked,
  onNewModel,
  onAnalyze,
  onDerive,
  onContextMenuRequest,
}: NativeDataSurfaceProps) {
  const dataset = useWorkspace((state) => state.dataset);
  const datasetDescriptorOnly = useWorkspace((state) => state.datasetDescriptorOnly);
  const datasetCatalog = useWorkspace((state) => state.datasetCatalog);
  const datasetVersions = useWorkspace((state) => state.datasetVersions);
  const groupColumn = configuredGroupColumn?.trim() ?? "";
  const setDataset = useWorkspace((state) => state.setDataset);
  const pushToast = useWorkspace((state) => state.pushToast);
  const schema6Session = useInternalProjectArchiveV6Session((state) => state.session);
  const [mode, setMode] = useState<NativeDataMode>("data");
  const [variableSearch, setVariableSearch] = useState("");
  const [activatingDatasetId, setActivatingDatasetId] = useState<string | null>(null);
  const [pageSize, setPageSize] = useState(DEFAULT_NATIVE_DATA_PAGE_SIZE);
  const [requestedPage, setRequestedPage] = useState(0);
  const [nativePageState, setNativePageState] = useState<NativePageState | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const tableRegionRef = useRef<HTMLDivElement>(null);
  const nativeRuntime = isNativeDesktop();
  const rowCount = dataset.rowCount ?? dataset.rows.length;

  const page = useMemo(
    () => nativeDataPage(rowCount, pageSize, requestedPage),
    [rowCount, pageSize, requestedPage],
  );
  const pageAuthorityKey = datasetDescriptorOnly
    ? schema6Session?.snapshot.archiveSha256 ?? "unbound"
    : "legacy";
  const pageKey = `${dataset.id}:${dataset.fingerprint ?? "unfingerprinted"}:${pageAuthorityKey}:${page.start}:${pageSize}`;
  const currentNativePage = nativePageState?.key === pageKey ? nativePageState : null;
  const visibleRows = useMemo(
    () => nativeRuntime
      ? currentNativePage?.status === "ready" ? currentNativePage.rows : []
      : nativeDataPageRows(dataset.rows, page),
    [currentNativePage, dataset.rows, nativeRuntime, page.start, page.end],
  );
  const missingCounts = useMemo(
    () => dataset.missingByColumn
      ? new Map(dataset.columns.map((column) => [column, dataset.missingByColumn?.[column] ?? 0]))
      : nativeMissingCounts(dataset.columns, dataset.rows),
    [dataset.columns, dataset.missingByColumn, dataset.rows],
  );
  const metadataByColumn = useMemo(
    () => new Map(dataset.columnMetadata?.map((item) => [item.name, item]) ?? []),
    [dataset.columnMetadata],
  );
  const pagerLabelId = `nd-data-range-${dataset.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  const gridInstructionsId = `${pagerLabelId}-grid-instructions`;
  const dataQualityAvailable = !datasetDescriptorOnly && Number.isFinite(dataset.missing);
  const totalCells = rowCount * dataset.columns.length;
  const completePercent = dataQualityAvailable && totalCells > 0
    ? Math.max(0, Math.min(100, ((totalCells - dataset.missing) / totalCells) * 100))
    : null;
  const dataKindLabel = dataset.kind === "covariance" ? "Covariance matrix" : dataset.kind === "correlation" ? "Correlation matrix" : "Raw data";
  const deriveDisabledReason = !nativeRuntime
    ? "Derived variables are available in the installed Windows app."
    : !projectWritable
      ? "Save a writable copy before deriving a variable."
      : mutationsLocked
        ? "Finish or cancel the active calculation before deriving a variable."
        : (dataset.kind ?? "raw") !== "raw"
          ? "Choose a raw-observation dataset before deriving a variable."
          : !dataset.columns.length
            ? "Import data before deriving a variable."
            : null;
  const pageReady = !nativeRuntime || currentNativePage?.status === "ready";
  const pageError = currentNativePage?.status === "error" ? currentNativePage.error : null;
  const versionItems = useMemo(
    () => nativeDatasetVersionItems(datasetCatalog, datasetVersions),
    [datasetCatalog, datasetVersions],
  );
  const visibleColumns = useMemo(() => {
    const query = variableSearch.trim().toLocaleLowerCase();
    if (!query) return dataset.columns;
    return dataset.columns.filter((column) => {
      const metadata = metadataByColumn.get(column);
      return column.toLocaleLowerCase().includes(query)
        || metadata?.label?.toLocaleLowerCase().includes(query);
    });
  }, [dataset.columns, metadataByColumn, variableSearch]);
  const selectedColumnIndex = Math.max(0, dataset.columns.indexOf(selectedColumn));
  const dataGrid = useNativeScientificGrid({
    gridKey: `${dataset.id}:${page.start}:${pageSize}`,
    rowCount: visibleRows.length,
    columnCount: dataset.columns.length,
    initialColumnIndex: selectedColumnIndex,
    controlledColumnIndex: selectedColumnIndex,
    getClipboardText: ({ rowIndex, columnIndex }) => nativeGridClipboardText(
      visibleRows[rowIndex]?.[dataset.columns[columnIndex]],
    ),
    onActiveCellChange: ({ columnIndex }) => {
      const column = dataset.columns[columnIndex];
      if (column && column !== selectedColumn) setSelectedColumn(column);
    },
  });

  const activateVersion = async (datasetId: string) => {
    if (datasetId === dataset.id || activatingDatasetId || mutationsLocked) return;
    setActivatingDatasetId(datasetId);
    try {
      const local = datasetCatalog.find((candidate) => candidate.id === datasetId);
      const activated = nativeRuntime
        ? await activateNativeDataset(datasetId)
        : local;
      if (!activated) throw new Error("This dataset version is not available.");
      setDataset(activated);
      pushToast({ tone: "success", title: "Dataset version activated", detail: activated.name });
    } catch (reason) {
      pushToast({ tone: "error", title: "Could not activate dataset version", detail: reason instanceof Error ? reason.message : String(reason) });
    } finally {
      setActivatingDatasetId(null);
    }
  };

  useEffect(() => {
    setRequestedPage(0);
  }, [dataset.id]);
  useEffect(() => {
    if (!nativeRuntime || mode !== "data" || dataset.columns.length === 0 || rowCount === 0) {
      setNativePageState(null);
      return;
    }

    let active = true;
    setNativePageState({ key: pageKey, status: "loading", rows: [] });
    void readNativeDatasetPageV1({
      dataset,
      datasetDescriptorOnly,
      session: schema6Session,
      offset: page.start,
      limit: pageSize,
    }, readInternalProjectArchiveV6DatasetRows, getNativeDatasetRows)
      .then((response) => {
        if (!active) return;
        if (response.datasetId !== dataset.id || response.offset !== page.start) {
          throw new Error("The dataset changed while this page was loading.");
        }
        setNativePageState({ key: pageKey, status: "ready", rows: response.rows });
      })
      .catch((error: unknown) => {
        if (!active) return;
        const message = error instanceof Error ? error.message : String(error);
        setNativePageState({ key: pageKey, status: "error", rows: [], error: message });
      });

    return () => { active = false; };
  }, [dataset, datasetDescriptorOnly, mode, nativeRuntime, page.start, pageKey, pageSize, reloadToken, rowCount, schema6Session]);


  useEffect(() => {
    if (requestedPage !== page.pageIndex) setRequestedPage(page.pageIndex);
  }, [page.pageIndex, requestedPage]);

  useEffect(() => {
    tableRegionRef.current?.scrollTo({ top: 0 });
  }, [page.pageIndex, pageSize]);

  const changePageSize = (nextPageSize: number) => {
    setPageSize(nextPageSize);
    setRequestedPage(0);
  };

  const requestPointerContextMenu = (event: ReactMouseEvent<HTMLDivElement>) => {
    const target = dataContextTarget(event.target, event.currentTarget);
    if (target.kind === "variable") setSelectedColumn(target.column);
    const returnFocus = event.target instanceof Element
      ? event.target.closest<HTMLElement>("button, [href], input, select, textarea, [tabindex]")
      : null;
    const opened = onContextMenuRequest({
      clientX: event.clientX,
      clientY: event.clientY,
      returnFocus: returnFocus ?? event.currentTarget,
      target,
    });
    event.stopPropagation();
    if (opened) event.preventDefault();
  };

  const requestKeyboardContextMenu = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.defaultPrevented || !isContextMenuKeyboardGesture(event.key, event.shiftKey)) return;
    const targetElement = event.target instanceof Element ? event.target : event.currentTarget;
    if (targetElement.closest("input, textarea, select, [contenteditable='true']")) return;
    const target = dataContextTarget(targetElement, event.currentTarget);
    if (target.kind === "variable") setSelectedColumn(target.column);
    const returnFocus = targetElement.closest<HTMLElement>("button, [href], [tabindex]")
      ?? (targetElement instanceof HTMLElement ? targetElement : event.currentTarget);
    const bounds = returnFocus.getBoundingClientRect();
    const opened = onContextMenuRequest({
      clientX: bounds.left + Math.min(24, Math.max(8, bounds.width / 2)),
      clientY: bounds.bottom || bounds.top + 24,
      returnFocus,
      target,
    });
    event.stopPropagation();
    if (opened) event.preventDefault();
  };

  return <div className={`nd-three-pane${propertiesOpen ? "" : " no-properties"}`} onContextMenu={requestPointerContextMenu} onKeyDown={requestKeyboardContextMenu}>
    <aside className="nd-navigator" aria-label="Project data navigator">
      <DataPaneTitle icon={<Database size={14} />} title="Dataset" />
      <div className="nd-tree-row active" data-native-dataset="active"><ChevronDown size={13} /><Table2 size={14} /><span>{dataset.name}</span></div>
      <div className="nd-tree-children">
        <button type="button" className={mode === "data" ? "active" : ""} onClick={() => setMode("data")}><Table2 size={13} />Data view</button>
        <button type="button" className={mode === "variables" ? "active" : ""} onClick={() => setMode("variables")}><FileSpreadsheet size={13} />Variable view</button>
        <button type="button" className={mode === "quality" ? "active" : ""} onClick={() => setMode("quality")}><CircleGauge size={13} />Data Quality</button>
        <button type="button" className={mode === "import" ? "active" : ""} onClick={() => setMode("import")}><FileText size={13} />Import Details</button>
      </div>
      <DataPaneTitle icon={<History size={13} />} title={`Versions (${versionItems.length})`} />
      <div className="nd-version-list" role="list" aria-label="Dataset versions" aria-busy={Boolean(activatingDatasetId)}>
        {versionItems.map((item) => {
          const active = item.dataset.id === dataset.id;
          const loading = item.dataset.id === activatingDatasetId;
          const operation = nativeDatasetOperationLabel(item.record?.operation ?? null);
          return <div className="nd-version-item" role="listitem" key={item.dataset.id}><button
              type="button"
              data-native-dataset={item.dataset.id}
              className={active ? "active" : ""}
              aria-current={active ? "true" : undefined}
              disabled={Boolean(activatingDatasetId) || mutationsLocked}
              onClick={() => { void activateVersion(item.dataset.id); }}
              style={{ paddingLeft: 7 + Math.min(item.depth, 4) * 11 }}
              title={mutationsLocked ? "Dataset versions cannot change while a calculation is active." : item.record?.summary ?? item.dataset.name}
            >
              {loading ? <LoaderCircle className="nd-spin" size={12} aria-hidden="true" /> : active ? <Check size={12} aria-hidden="true" /> : <History size={12} aria-hidden="true" />}
              <span><strong>V{item.versionNumber}</strong> {item.dataset.name}<small>{operation}</small></span>
            </button></div>;
        })}
        {!versionItems.length ? <p className="nd-nav-empty">No dataset versions</p> : null}
      </div>
      <DataPaneTitle title="Variables" />
      <label className="nd-nav-search">
        <Search size={12} aria-hidden="true" />
        <span className="nd-sr-only">Search variables</span>
        <input type="search" value={variableSearch} onChange={(event) => setVariableSearch(event.target.value)} placeholder="Search variables" aria-label="Search variables" />
      </label>
      <div className="nd-variable-list">
        {visibleColumns.map((column) => {
          const grouping = column === groupColumn;
          return <button type="button" key={column} data-native-variable={column} className={`${selectedColumn === column ? "active" : ""}${grouping ? " grouping" : ""}`} title={grouping ? `${column} is the configured grouping variable` : undefined} onClick={() => setSelectedColumn(column)}>{grouping ? <UsersRound size={12} aria-hidden="true" /> : <Square size={9} fill="currentColor" />}{column}{grouping ? <small>Groups</small> : null}</button>;
        })}
        {!visibleColumns.length && dataset.columns.length ? <p className="nd-nav-empty">No matching variables</p> : null}
      </div>
    </aside>

    <section className="nd-document nd-data-document">
      <div className="nd-document-tab nd-data-document-tab"><Table2 size={14} /><span>{dataset.name}</span><button type="button" disabled={Boolean(deriveDisabledReason)} title={deriveDisabledReason ?? "Create a non-destructive derived variable"} onClick={onDerive}><Plus size={13} aria-hidden="true" />Derive variable…</button></div>
      {dataset.columns.length > 0 && !hasEditableModel ? <section className="nd-data-next-actions" aria-labelledby="nd-data-next-actions-title">
        <div>
          <strong id="nd-data-next-actions-title">Choose what to do next</strong>
          <span>Build a path model, or analyze observed variables without creating a model.</span>
          {!projectWritable ? <small role="status">This project is read-only. Save a writable copy before starting new work.</small> : mutationsLocked ? <small role="status">Finish or cancel the active calculation before starting another workflow.</small> : null}
        </div>
        <div role="group" aria-label="Next actions for imported data">
          <button type="button" className="primary" disabled={!projectWritable || mutationsLocked} onClick={onNewModel}>New Model…</button>
          <button type="button" disabled={!projectWritable || mutationsLocked} onClick={onAnalyze}>Analyze…</button>
        </div>
      </section> : null}
      {dataset.columns.length && mode === "data" ? <div className="nd-data-grid">
        <div
          ref={tableRegionRef}
          className="nd-table-scroll"
          tabIndex={pageReady && visibleRows.length > 0 ? undefined : 0}
          role="region"
          aria-label={`Data table viewport for ${dataset.name}`}
          aria-describedby={pagerLabelId}
        >
          <table
            ref={dataGrid.tableRef}
            className="nd-data-table nd-scientific-grid"
            role="grid"
            aria-label={`Data table for ${dataset.name}`}
            aria-describedby={`${pagerLabelId} ${gridInstructionsId}`}
            aria-rowcount={rowCount + 1}
            aria-colcount={dataset.columns.length + 1}
            aria-multiselectable="false"
            aria-keyshortcuts="Control+C"
            aria-busy={!pageReady}
            onKeyDown={dataGrid.handleKeyDown}
          >
            <caption className="nd-sr-only">{dataset.name}. {nativeDataRangeLabel(rowCount, page)}.</caption>
            <thead><tr role="row" aria-rowindex={1}><th className="row-index" role="columnheader" scope="col" aria-colindex={1}>#</th>{dataset.columns.map((column, columnIndex) => { const grouping = column === groupColumn; return <th key={column} data-native-variable={column} className={`${selectedColumn === column ? "selected" : ""}${grouping ? " grouping" : ""}`} role="columnheader" scope="col" aria-colindex={columnIndex + 2}><button type="button" tabIndex={-1} onClick={() => setSelectedColumn(column)} title={grouping ? `Select ${column}; configured grouping variable` : `Select ${column}`}>{grouping ? <UsersRound size={11} aria-hidden="true" /> : null}{column}</button></th>; })}</tr></thead>
            <tbody>
              {!pageReady && !pageError ? <tr><td className="nd-data-message" colSpan={dataset.columns.length + 1}><span role="status">Loading cases...</span></td></tr> : null}
              {pageError ? <tr><td className="nd-data-message error" colSpan={dataset.columns.length + 1}><span role="alert">Could not load this data page. {pageError}</span><button type="button" onClick={() => setReloadToken((value) => value + 1)}><RefreshCw size={13} />Retry</button></td></tr> : null}
              {pageReady ? visibleRows.map((row, localIndex) => {
                const rowIndex = page.start + localIndex;
                return <tr role="row" key={rowIndex} aria-rowindex={rowIndex + 2}>
                  <th className="row-index" role="rowheader" scope="row" aria-colindex={1}>{rowIndex + 1}</th>
                  {dataset.columns.map((column, columnIndex) => <td
                    {...dataGrid.cellProps(localIndex, columnIndex)}
                    aria-colindex={columnIndex + 2}
                    key={column}
                    data-native-variable={column}
                    className={selectedColumn === column ? "selected" : ""}
                  >{row[column] ?? <span className="nd-missing" aria-label="Missing value">.</span>}</td>)}
                </tr>;
              }) : null}
            </tbody>
          </table>
          <span className="nd-sr-only" id={gridInstructionsId}>Use the arrow keys to move between cells. Press Control+C to copy the selected cell.</span>
          <span className="nd-sr-only" role="status" aria-live="polite">{dataGrid.announcement}</span>
        </div>
        <footer className="nd-data-pager" aria-label="Data table pagination">
          <span id={pagerLabelId} className="nd-data-range">{nativeDataRangeLabel(rowCount, page)}</span>
          {page.pageCount > 1 ? <>
            <label className="nd-page-size">Rows per page
              <select value={pageSize} onChange={(event) => changePageSize(Number(event.target.value))}>
                {NATIVE_DATA_PAGE_SIZES.map((size) => <option key={size} value={size}>{size}</option>)}
              </select>
            </label>
            <span className="nd-page-number">Page {page.pageIndex + 1} of {page.pageCount}</span>
            <div className="nd-page-buttons" role="group" aria-label="Choose data page">
              <button type="button" disabled={!page.hasPrevious} onClick={() => setRequestedPage(0)} aria-label="First page" title="First page"><ChevronsLeft size={14} /></button>
              <button type="button" disabled={!page.hasPrevious} onClick={() => setRequestedPage(page.pageIndex - 1)} aria-label="Previous page" title="Previous page"><ChevronLeft size={14} /></button>
              <button type="button" disabled={!page.hasNext} onClick={() => setRequestedPage(page.pageIndex + 1)} aria-label="Next page" title="Next page"><ChevronRight size={14} /></button>
              <button type="button" disabled={!page.hasNext} onClick={() => setRequestedPage(page.pageCount - 1)} aria-label="Last page" title="Last page"><ChevronsRight size={14} /></button>
            </div>
          </> : null}
        </footer>
      </div> : dataset.columns.length && mode === "variables" ? <div className="nd-table-scroll" tabIndex={0} role="region" aria-label="Variable view">
        <table className="nd-result-table">
          <thead><tr><th>Name</th><th>Type</th><th>Scale</th><th>Role</th><th>Missing</th></tr></thead>
          <tbody>{dataset.columns.map((column) => {
            const item = metadataByColumn.get(column);
            return <tr key={column} data-native-variable={column} tabIndex={0} aria-selected={selectedColumn === column} onClick={() => setSelectedColumn(column)} onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); setSelectedColumn(column); } }}>
               <td>{column}</td><td>{item?.column_type ?? "Not declared"}</td><td>{item?.scale_type ?? "Not declared"}</td><td>{column === groupColumn ? <span className="nd-group-role"><UsersRound size={12} aria-hidden="true" />Grouping</span> : "Analysis"}</td><td>{dataQualityAvailable ? missingCounts.get(column) ?? 0 : "Not stored"}</td>
             </tr>;
           })}</tbody>
         </table>
       </div> : dataset.columns.length && mode === "quality" && !dataQualityAvailable ? <div className="nd-data-detail" role="region" aria-label="Data Quality unavailable">
         <header className="nd-data-detail-heading"><h2>Data Quality</h2><span>Summary not stored</span></header>
         <p role="status">This verified General SEM archive exposes its exact data rows on demand, but it does not store per-variable missing-value summaries. QuickPLS will not infer quality statistics from the descriptor-only placeholder.</p>
       </div> : dataset.columns.length && mode === "quality" ? <div className="nd-data-detail" role="region" aria-label="Data Quality">
        <header className="nd-data-detail-heading"><h2>Data Quality</h2><span>Current dataset</span></header>
        <dl className="nd-data-summary">
          <div><dt>Cases</dt><dd>{rowCount.toLocaleString()}</dd></div>
          <div><dt>Variables</dt><dd>{dataset.columns.length.toLocaleString()}</dd></div>
          <div><dt>Missing values</dt><dd>{dataset.missing.toLocaleString()}</dd></div>
           <div><dt>Complete cells</dt><dd>{completePercent == null ? "Not stored" : `${completePercent.toFixed(1)}%`}</dd></div>
        </dl>
        <div className="nd-table-scroll" tabIndex={0} role="region" aria-label="Missing values by variable">
          <table className="nd-result-table">
            <thead><tr><th>Variable</th><th>Type</th><th>Scale</th><th>Missing</th><th>Missing %</th></tr></thead>
            <tbody>{dataset.columns.map((column) => {
              const item = metadataByColumn.get(column);
              const missing = missingCounts.get(column) ?? 0;
              return <tr key={column} data-native-variable={column} tabIndex={0} aria-selected={selectedColumn === column} onClick={() => setSelectedColumn(column)}>
                <td>{column}</td>
                <td>{item?.column_type ?? "Not declared"}</td>
                <td>{item?.scale_type ?? "Not declared"}</td>
                <td>{missing.toLocaleString()}</td>
                <td>{rowCount > 0 ? ((missing / rowCount) * 100).toFixed(1) : "0.0"}%</td>
              </tr>;
            })}</tbody>
          </table>
        </div>
      </div> : dataset.columns.length && mode === "import" ? <div className="nd-data-detail" role="region" aria-label="Import Details">
        <header className="nd-data-detail-heading"><h2>Import Details</h2><span>Current dataset</span></header>
        <dl className="nd-import-details">
          <div><dt>File</dt><dd>{dataset.name}</dd></div>
          <div><dt>Data kind</dt><dd>{dataKindLabel}</dd></div>
          <div><dt>Cases</dt><dd>{rowCount.toLocaleString()}</dd></div>
          <div><dt>Variables</dt><dd>{dataset.columns.length.toLocaleString()}</dd></div>
           <div><dt>Missing values</dt><dd>{dataQualityAvailable ? dataset.missing.toLocaleString() : "Not stored"}</dd></div>
          {dataset.sampleSize != null ? <div><dt>Declared sample size</dt><dd>{dataset.sampleSize.toLocaleString()}</dd></div> : null}
          <div><dt>Fingerprint</dt><dd><code title={dataset.fingerprint}>{dataset.fingerprint?.trim() || "Not available"}</code></dd></div>
        </dl>
      </div> : <div className="nd-empty"><Database size={28} /><strong>No dataset imported</strong><span>Use Import data to add a CSV or supported data file.</span></div>}
    </section>

    {propertiesOpen ? <aside className="nd-properties" aria-label="Variable properties">
      <DataPaneTitle title="Variable properties" />
      {selectedColumn ? <NativeVariableProperties
        dataset={dataset}
        selectedColumn={selectedColumn}
         missingCount={dataQualityAvailable ? missingCounts.get(selectedColumn) ?? 0 : null}
        rowCount={rowCount}
        isGroupingVariable={selectedColumn === groupColumn}
        mutationDisabledReason={!projectWritable
          ? "This General SEM archive is read-only. Dataset metadata revisions require a future versioned authority workflow."
          : mutationsLocked
            ? "Dataset changes are locked while General SEM publication or another protected operation is active."
            : null}
      /> : <div className="nd-pane-empty">Select a variable.</div>}
    </aside> : null}
  </div>;
}

function NativeVariableProperties({
  dataset,
  selectedColumn,
  missingCount,
  rowCount,
  isGroupingVariable,
  mutationDisabledReason,
}: {
  dataset: Dataset;
  selectedColumn: string;
  missingCount: number | null;
  rowCount: number;
  isGroupingVariable: boolean;
  mutationDisabledReason: string | null;
}) {
  const setDataset = useWorkspace((state) => state.setDataset);
  const pushToast = useWorkspace((state) => state.pushToast);
  const currentMetadata = useMemo(
    () => dataset.columnMetadata?.find((item) => item.name === selectedColumn)
      ?? defaultNativeColumnMetadata(dataset, selectedColumn),
    [dataset, selectedColumn],
  );
  const [draft, setDraft] = useState<NativeVariableMetadataDraft>(() => nativeVariableMetadataDraft(currentMetadata));
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setDraft(nativeVariableMetadataDraft(currentMetadata));
    setStatus("idle");
    setError(null);
    // A returned metadata snapshot should not erase the visible Saved state.
    // Switching datasets or variables always resets the editor.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dataset.id, selectedColumn]);

  const update = <K extends keyof NativeVariableMetadataDraft>(key: K, value: NativeVariableMetadataDraft[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setStatus("idle");
    setError(null);
  };

  const saveMetadata = async () => {
    if (mutationDisabledReason) {
      setStatus("error");
      setError(mutationDisabledReason);
      return;
    }
    const validation = validateNativeVariableMetadata(currentMetadata, draft);
    if (!validation.metadata) {
      setStatus("error");
      setError(validation.error);
      return;
    }
    setStatus("saving");
    setError(null);
    try {
      const updated = isNativeDesktop()
        ? await updateNativeColumnMetadata(dataset.id, selectedColumn, validation.metadata)
        : replaceColumnMetadata(dataset, validation.metadata);
      setDataset(updated);
      setStatus("saved");
      pushToast({ tone: "success", title: "Variable updated", detail: `${selectedColumn} metadata was saved.` });
    } catch (reason) {
      const message = reason instanceof Error ? reason.message : String(reason);
      setStatus("error");
      setError(message);
    }
  };

  const markerLabel = currentMetadata.missing_markers.filter(Boolean).join(", ") || "Blank cells only";
  const fieldPrefix = `nd-variable-${selectedColumn.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  return <form className="nd-property-form nd-variable-property-form" onSubmit={(event) => { event.preventDefault(); void saveMetadata(); }}>
    <dl className="nd-property-list">
      <div><dt>Name</dt><dd>{selectedColumn}</dd></div>
      <div><dt>Physical type</dt><dd>{currentMetadata.column_type}</dd></div>
      <div><dt>Role</dt><dd>{isGroupingVariable ? <span className="nd-group-role"><UsersRound size={12} aria-hidden="true" />Grouping variable</span> : "Analysis variable"}</dd></div>
       <div><dt>Missing values</dt><dd>{missingCount == null ? "Not stored" : missingCount.toLocaleString()}</dd></div>
      <div><dt>Cases</dt><dd>{rowCount.toLocaleString()}</dd></div>
      <div><dt>Import markers</dt><dd title={markerLabel}>{markerLabel}</dd></div>
    </dl>
    <p className="nd-property-note">Missing markers are applied when the file is imported.</p>
    {mutationDisabledReason ? <p className="nd-property-note" role="status">{mutationDisabledReason}</p> : null}
    <label htmlFor={`${fieldPrefix}-label`}>Label
      <input id={`${fieldPrefix}-label`} type="text" value={draft.label} disabled={Boolean(mutationDisabledReason)} onChange={(event) => update("label", event.target.value)} />
    </label>
    <label htmlFor={`${fieldPrefix}-scale`}>Scale
      <select id={`${fieldPrefix}-scale`} value={draft.scaleType} disabled={Boolean(mutationDisabledReason)} onChange={(event) => update("scaleType", event.target.value as ColumnMetadata["scale_type"])}>
        <option value="continuous">Continuous</option>
        <option value="ordinal">Ordinal</option>
        <option value="nominal">Nominal</option>
        <option value="binary">Binary</option>
        <option value="identifier">Identifier</option>
      </select>
    </label>
    <div className="nd-property-bounds" role="group" aria-label="Theoretical range">
      <label htmlFor={`${fieldPrefix}-minimum`}>Minimum
        <input id={`${fieldPrefix}-minimum`} type="number" step="any" value={draft.theoreticalMin} disabled={Boolean(mutationDisabledReason)} onChange={(event) => update("theoreticalMin", event.target.value)} />
      </label>
      <label htmlFor={`${fieldPrefix}-maximum`}>Maximum
        <input id={`${fieldPrefix}-maximum`} type="number" step="any" value={draft.theoreticalMax} disabled={Boolean(mutationDisabledReason)} onChange={(event) => update("theoreticalMax", event.target.value)} />
      </label>
    </div>
    {error ? <p className="nd-form-error" role="alert">{error}</p> : null}
    <div className="nd-property-actions">
      <span className="nd-form-status" role="status">{status === "saving" ? "Saving..." : status === "saved" ? "Saved" : ""}</span>
      <button className="primary" type="submit" disabled={status === "saving" || Boolean(mutationDisabledReason)}>{status === "saving" ? "Applying..." : "Apply"}</button>
    </div>
  </form>;
}

function replaceColumnMetadata(dataset: Dataset, metadata: ColumnMetadata): Dataset {
  const items = dataset.columnMetadata ?? [];
  const exists = items.some((item) => item.name === metadata.name);
  return {
    ...dataset,
    columnMetadata: exists
      ? items.map((item) => item.name === metadata.name ? metadata : item)
      : [...items, metadata],
  };
}

function DataPaneTitle({ title, icon }: { title: string; icon?: ReactNode }) {
  return <header className="nd-pane-title">{icon}<strong>{title}</strong></header>;
}
