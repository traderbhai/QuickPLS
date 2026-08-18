import { AlertTriangle, Boxes, CheckCircle2, Database, FlaskConical, Info, Save, Search, Upload } from "lucide-react";
import Papa from "papaparse";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import validationFixture from "../../validation/fixtures/corporate_reputation.csv?raw";
import { columnProfile, dataQualitySummary, detectPrefixGroups, filteredColumns, type DataColumnFilter, type DataQualitySummary } from "../domain/dataWorkspace";
import { dataGuidance } from "../domain/methodApplicability";
import { applyNativeDatasetTransformation, importNativeDataset, importNativeValidationFixture, isNativeDesktop, recodeNativeDatasetColumn, updateNativeColumnMetadata } from "../services/projectService";
import { useWorkspace } from "../store";
import type { ColumnMetadata, Dataset, WorkspaceView } from "../types";
import { executeDataWorkspaceVersionedAction, sortDataWorkspaceViewRows, type DataWorkspaceVersionedAction, type DataWorkspaceViewSort } from "./dataWorkspaceVersionedActions";
import { InlineNotice, MetricCard, PageHeader, Panel, WorkspacePage } from "./Ui";

type ImportKind = "raw" | "covariance" | "correlation";
type DataWorkbenchTab = "data" | "variables" | "import" | "quality" | "notes";
type DerivedVariableOperation = "reverse" | "add" | "subtract" | "multiply" | "divide" | "sum" | "mean" | "dummy" | "group";

const dataWorkbenchTabs: Array<{ id: DataWorkbenchTab; label: string; detail: string }> = [
  { id: "data", label: "Data View", detail: "Case-by-variable data grid" },
  { id: "variables", label: "Variable View", detail: "Metadata and measurement roles" },
  { id: "import", label: "Import History", detail: "Source, file type, and import options" },
  { id: "quality", label: "Data Quality", detail: "Readiness and variable issues" },
  { id: "notes", label: "Notes", detail: "Research notes and model handoff" },
];

const validationFixtureSource = "Bundled sample: corporate_reputation.csv";
const validationFixtureDevelopmentPath = "D:\\QuickPLS\\validation\\fixtures\\corporate_reputation.csv";

const defaultMetadata = (name: string): ColumnMetadata => ({
  name,
  label: null,
  column_type: "numeric",
  scale_type: "continuous",
  missing_markers: ["", "NA", "N/A", "."],
  theoretical_min: null,
  theoretical_max: null,
  value_labels: {},
});

const importKindLabel = (kind: ImportKind) => kind === "raw" ? "Raw data" : kind === "covariance" ? "Covariance matrix" : "Correlation matrix";

const status = (message: string, tone: "info" | "success" | "warning" | "error" = "info") => {
  window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message, tone } }));
};

export function DataWorkspace() {
  const inputRef = useRef<HTMLInputElement>(null);
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const setDataset = useWorkspace((state) => state.setDataset);
  const commitDatasetVersion = useWorkspace((state) => state.commitDatasetVersion);
  const setView = useWorkspace((state) => state.setView);
  const addConstructsFromIndicatorGroups = useWorkspace((state) => state.addConstructsFromIndicatorGroups);
  const [importKind, setImportKind] = useState<ImportKind>("raw");
  const [sampleSize, setSampleSize] = useState("");
  const [missingMarkers, setMissingMarkers] = useState("NA, N/A, .");
  const [selectedColumn, setSelectedColumn] = useState(dataset.columns[0] ?? "");
  const [columnQuery, setColumnQuery] = useState("");
  const [columnFilter, setColumnFilter] = useState<DataColumnFilter>("all");
  const [showValidationDetails, setShowValidationDetails] = useState(false);
  const [activeTab, setActiveTab] = useState<DataWorkbenchTab>("data");
  const [transformationNotice, setTransformationNotice] = useState<{ message: string; danger: boolean } | null>(null);
  const [rowSort, setRowSort] = useState<DataWorkspaceViewSort | null>(null);
  const [derivedOperation, setDerivedOperation] = useState<DerivedVariableOperation>("reverse");
  const [derivedOutputName, setDerivedOutputName] = useState("");
  const [rightColumn, setRightColumn] = useState(dataset.columns[1] ?? dataset.columns[0] ?? "");
  const [aggregateColumns, setAggregateColumns] = useState(dataset.columns.slice(0, 2).join(", "));
  const [reverseMinimum, setReverseMinimum] = useState("1");
  const [reverseMaximum, setReverseMaximum] = useState("5");
  const [aggregateMissingPolicy, setAggregateMissingPolicy] = useState<"propagate" | "available">("propagate");
  const [minimumNonMissing, setMinimumNonMissing] = useState("");
  const [dummyMatchValue, setDummyMatchValue] = useState("");
  const [dummyMissingPolicy, setDummyMissingPolicy] = useState<"missing" | "zero">("missing");
  const [groupRules, setGroupRules] = useState("");
  const [groupUnmatched, setGroupUnmatched] = useState<"missing" | "error">("missing");
  const selectedMetadata = useMemo(() => dataset.columnMetadata?.find((column) => column.name === selectedColumn) ?? defaultMetadata(selectedColumn), [dataset.columnMetadata, selectedColumn]);
  const selectedProfile = useMemo(() => selectedColumn ? columnProfile(dataset, selectedColumn) : null, [dataset, selectedColumn]);
  const [draft, setDraft] = useState<ColumnMetadata>(selectedMetadata);
  const quality = useMemo(() => dataQualitySummary(dataset), [dataset]);
  const prefixGroups = useMemo(() => detectPrefixGroups(dataset.columns), [dataset.columns]);
  const visibleColumns = useMemo(() => filteredColumns(dataset, columnQuery, columnFilter), [dataset, columnFilter, columnQuery]);
  const guidance = useMemo(() => dataGuidance({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() }), [dataset, edges, nodes, settings]);
  const topGuidance = guidance[0];
  const matrixSampleSize = sampleSize === "" ? undefined : Number(sampleSize);
  const matrixReady = importKind === "raw" || Boolean(matrixSampleSize && matrixSampleSize >= 2);
  const desktopOnlyMatrix = importKind !== "raw" && !isNativeDesktop();
  const filteredOutCount = Math.max(0, dataset.columns.length - visibleColumns.length);
  const previewRows = useMemo(() => sortDataWorkspaceViewRows(dataset.rows, rowSort).slice(0, 100), [dataset.rows, rowSort]);

  const setParsedDataset = (csv: string, name: string) => {
    Papa.parse<Record<string, string | number | null>>(csv, {
      header: true,
      dynamicTyping: true,
      skipEmptyLines: true,
      complete: ({ data, meta }) => {
        const columns = meta.fields ?? [];
        setDataset({
          id: crypto.randomUUID(),
          name,
          rows: data,
          columns,
          missing: data.reduce((sum, row) => sum + Object.values(row).filter((value) => value == null || value === "").length, 0),
          rowCount: data.length,
          kind: "raw",
          columnMetadata: columns.map(defaultMetadata),
        });
        setSelectedColumn(columns[0] ?? "");
      },
    });
  };

  useEffect(() => { setDraft(selectedMetadata); }, [selectedMetadata]);
  useEffect(() => { if (!dataset.columns.includes(selectedColumn)) setSelectedColumn(dataset.columns[0] ?? ""); }, [dataset.columns, selectedColumn]);
  useEffect(() => { setRowSort(null); }, [dataset.id]);
  useEffect(() => {
    if (!dataset.columns.includes(rightColumn)) setRightColumn(dataset.columns.find((column) => column !== selectedColumn) ?? dataset.columns[0] ?? "");
  }, [dataset.columns, rightColumn, selectedColumn]);

  const importData = async () => {
    if (importKind !== "raw" && !matrixReady) {
      window.alert("Enter the study sample size before importing a covariance or correlation matrix."); return;
    }
    if (!isNativeDesktop()) {
      if (importKind !== "raw") { window.alert("Matrix imports require the native QuickPLS desktop application."); return; }
      inputRef.current?.click(); return;
    }
    const imported = await importNativeDataset(importKind, matrixSampleSize, ["", ...missingMarkers.split(",").map((value) => value.trim()).filter(Boolean)]);
    if (imported) setDataset(imported);
  };

  const saveMetadata = async () => {
    if (!selectedColumn) return;
    if (isNativeDesktop()) {
      setDataset(await updateNativeColumnMetadata(dataset.id, selectedColumn, draft));
      return;
    }
    const columnMetadata = (dataset.columnMetadata ?? dataset.columns.map(defaultMetadata)).map((column) => column.name === selectedColumn ? draft : column);
    setDataset({ ...dataset, columnMetadata });
  };

  const loadSampleDataset = async () => {
    if (isNativeDesktop()) {
      const imported = await importNativeValidationFixture();
      setDataset(imported);
      setSelectedColumn(imported.columns[0] ?? "");
    } else {
      setParsedDataset(validationFixture, "corporate_reputation.csv");
    }
    setImportKind("raw");
    setSampleSize("");
  };

  const createConstructsFromPrefixes = () => {
    addConstructsFromIndicatorGroups(prefixGroups.flatMap((group) => group.indicators));
    setView("models");
  };

  const runVersionedAction = useCallback((action: DataWorkspaceVersionedAction) => {
    void executeDataWorkspaceVersionedAction({
      dataset,
      nativeDesktop: isNativeDesktop(),
      createRecodeVersion: recodeNativeDatasetColumn,
      createTransformationVersion: applyNativeDatasetTransformation,
      commitVersion: commitDatasetVersion,
    }, action).then((result) => {
      if (result.kind === "blocked") {
        setTransformationNotice({ message: result.message, danger: false });
        status(result.message, "warning");
        return;
      }
      setTransformationNotice(null);
      setSelectedColumn(result.selectedColumn);
      setActiveTab("data");
      if (result.kind === "view-only") {
        setRowSort(result.sort);
        status(result.message, "info");
        return;
      }
      status(`Immutable dataset version created. ${result.message}`, "success");
    }).catch((error) => {
      const message = error instanceof Error ? error.message : String(error);
      setTransformationNotice({ message, danger: true });
      status(message, "error");
    });
  }, [commitDatasetVersion, dataset]);

  const createDerivedVariable = () => {
    if (derivedOperation === "reverse") {
      runVersionedAction({ kind: "reverse-scale", column: selectedColumn || null, minimum: reverseMinimum, maximum: reverseMaximum, outputName: derivedOutputName });
      return;
    }
    if (["add", "subtract", "multiply", "divide"].includes(derivedOperation)) {
      runVersionedAction({
        kind: "arithmetic",
        leftColumn: selectedColumn || null,
        right: { kind: "column", column: rightColumn || null },
        operator: derivedOperation as "add" | "subtract" | "multiply" | "divide",
        outputName: derivedOutputName,
      });
      return;
    }
    if (derivedOperation === "sum" || derivedOperation === "mean") {
      runVersionedAction({
        kind: "row-aggregate",
        columns: aggregateColumns.split(",").map((column) => column.trim()).filter(Boolean),
        operation: derivedOperation,
        missingPolicy: aggregateMissingPolicy,
        minimumNonMissing,
        outputName: derivedOutputName,
      });
      return;
    }
    if (derivedOperation === "dummy") {
      runVersionedAction({ kind: "dummy", column: selectedColumn || null, matchValue: dummyMatchValue, missingPolicy: dummyMissingPolicy, outputName: derivedOutputName });
      return;
    }
    runVersionedAction({ kind: "group-values", column: selectedColumn || null, rules: groupRules, unmatched: groupUnmatched, outputName: derivedOutputName });
  };

  useEffect(() => {
    const handleFilter = (event: Event) => {
      setActiveTab("data");
      const detail = (event as CustomEvent<{ query?: string }>).detail;
      const query = detail?.query ?? window.prompt("Filter visible variables by name", columnQuery);
      if (query == null) return;
      setColumnQuery(query);
      status(query.trim() ? `Data view filtered to variables matching "${query.trim()}".` : "Data view variable filter cleared.", "info");
    };
    const handleSort = (event: Event) => {
      const detail = (event as CustomEvent<{ column?: string; direction?: "asc" | "desc" }>).detail;
      const direction = detail?.direction === "desc" ? "desc" : "asc";
      runVersionedAction({ kind: "sort", column: detail?.column ?? (selectedColumn || null), direction });
    };
    const handleAddColumn = (event: Event) => {
      const detail = (event as CustomEvent<{ name?: string; value?: string }>).detail;
      if (!isNativeDesktop()) {
        runVersionedAction({ kind: "add-column", name: detail?.name ?? "", value: detail?.value ?? "" });
        return;
      }
      const name = detail?.name ?? window.prompt("Name for the new immutable column", "new_column");
      if (name == null) return;
      const value = detail?.value ?? window.prompt("Constant value for every row. Leave blank for a missing-value column.", "");
      if (value == null) return;
      runVersionedAction({ kind: "add-column", name, value });
    };
    const handleRecode = (event: Event) => {
      const detail = (event as CustomEvent<{ column?: string; from?: string; to?: string }>).detail;
      const recodeColumn = detail?.column ?? selectedColumn;
      if (!recodeColumn) {
        status("Recode needs a selected data column.", "warning");
        return;
      }
      if (!isNativeDesktop()) {
        runVersionedAction({ kind: "recode", column: recodeColumn, from: "", to: "" });
        return;
      }
      const from = detail?.from ?? window.prompt(`Recode values in ${recodeColumn}: value to replace`, "");
      if (from == null) return;
      const to = detail?.to ?? window.prompt(`Replacement value for ${recodeColumn}. Leave blank to recode as missing.`, "");
      if (to == null) return;
      runVersionedAction({ kind: "recode", column: recodeColumn, from, to });
    };
    const handleMissingValues = (event: Event) => {
      const detail = (event as CustomEvent<{ column?: string | null; markers?: string }>).detail;
      runVersionedAction({ kind: "missing-values", column: detail?.column ?? null, markers: detail?.markers ?? "" });
    };
    const handleTransform = (event: Event) => {
      const detail = (event as CustomEvent<{ column?: string; outputName?: string; transform?: string }>).detail;
      runVersionedAction({ kind: "z-score", column: detail?.column ?? (selectedColumn || null), outputName: detail?.outputName ?? "" });
    };
    const handleCreateConstructs = () => {
      if (!prefixGroups.length) {
        window.alert("Prefix grouping needs at least two variables sharing the same alphabetic prefix.");
        return;
      }
      createConstructsFromPrefixes();
    };
    const handleShowQuality = () => setActiveTab("quality");
    window.addEventListener("quickpls:data-create-constructs-from-prefixes", handleCreateConstructs);
    window.addEventListener("quickpls:data-show-quality", handleShowQuality);
    window.addEventListener("quickpls:data-filter", handleFilter);
    window.addEventListener("quickpls:data-sort", handleSort);
    window.addEventListener("quickpls:data-add-column", handleAddColumn);
    window.addEventListener("quickpls:data-recode", handleRecode);
    window.addEventListener("quickpls:data-missing-values", handleMissingValues);
    window.addEventListener("quickpls:data-transform", handleTransform);
    return () => {
      window.removeEventListener("quickpls:data-create-constructs-from-prefixes", handleCreateConstructs);
      window.removeEventListener("quickpls:data-show-quality", handleShowQuality);
      window.removeEventListener("quickpls:data-filter", handleFilter);
      window.removeEventListener("quickpls:data-sort", handleSort);
      window.removeEventListener("quickpls:data-add-column", handleAddColumn);
      window.removeEventListener("quickpls:data-recode", handleRecode);
      window.removeEventListener("quickpls:data-missing-values", handleMissingValues);
      window.removeEventListener("quickpls:data-transform", handleTransform);
    };
  }, [prefixGroups, addConstructsFromIndicatorGroups, setView, columnQuery, runVersionedAction, selectedColumn]);

  return <WorkspacePage className="data-page data-v2-workspace data-v211-workspace data-v215-workspace data-v217-workspace data-v224-workbench" data-method-applicability-polish="v2.11.0" data-workflow-method-guidance-triage="v2.15.0" data-v217-mockup-screen="data" data-v224-data-workbench="spss-like-tabs">
    <PageHeader
      kicker="Data workspace"
      title={dataset.name}
      description={`${quality.rows} rows, ${quality.variables} variables, ${quality.missingCells} missing cells${dataset.kind && dataset.kind !== "raw" ? `, ${dataset.kind} matrix (n=${dataset.sampleSize ?? "unknown"})` : ""}`}
      actions={<button className="qpls2-primary-action" disabled={!dataset.columns.length} title={dataset.columns.length ? "Continue to the SEM diagram designer" : "Import a dataset before building the model"} onClick={() => setView("models")}>Open Model Designer</button>}
    />
    {quality.sampleWarning ? <InlineNotice tone="warning" title="Sample-size caution">{quality.sampleWarning}</InlineNotice> : null}
    {transformationNotice ? <div role="status" aria-live="polite"><InlineNotice tone={transformationNotice.danger ? "danger" : "warning"} title="Versioned transformation required">{transformationNotice.message}</InlineNotice></div> : null}

    <nav className="data-v224-tabs" aria-label="Data workbench tabs">
      {dataWorkbenchTabs.map((tab) => <button key={tab.id} type="button" className={activeTab === tab.id ? "active" : ""} onClick={() => setActiveTab(tab.id)}>
        <strong>{tab.label}</strong>
        <span>{tab.detail}</span>
      </button>)}
    </nav>

    {activeTab === "import" ? <div className="data-v2-top-grid data-v217-overview data-v224-import-history">
      <Panel title="Import source" description="Choose raw data or matrix input before opening the native import dialog." className="data-import-panel data-v2-import" actions={<select aria-label="Data type" value={importKind} onChange={(event) => setImportKind(event.target.value as ImportKind)}><option value="raw">Raw data</option><option value="covariance">Covariance matrix</option><option value="correlation">Correlation matrix</option></select>}>
        <div className="data-import-mode">
          <div><strong>{importKindLabel(importKind)}</strong><span>Current project data stays visible until a new file is imported.</span></div>
        </div>
        <div className="data-import-body">
          {importKind === "raw" ? <>
            <label>Missing value markers<input className="missing-markers-input" aria-label="Missing value markers" title="Comma-separated missing value markers for the next import" placeholder="NA, N/A, ." value={missingMarkers} onChange={(event) => setMissingMarkers(event.target.value)} /></label>
            <p><Info size={13} />Raw CSV/TSV can be inspected in browser preview. Desktop builds also use the native import dialog for supported research files.</p>
          </> : <>
            <label>Sample size<input aria-label="Sample size" type="number" min="2" placeholder="Required" value={sampleSize} onChange={(event) => setSampleSize(event.target.value)} /></label>
            <p className={matrixReady ? "" : "import-warning"}><Info size={13} />{importKindLabel(importKind)} imports require a square matrix with matching row/column labels and a study sample size of at least 2.</p>
            {desktopOnlyMatrix ? <p className="import-warning"><AlertTriangle size={13} />Matrix imports require the native QuickPLS desktop application; browser preview can inspect raw CSV/sample data only.</p> : null}
          </>}
          <div className="data-import-actions">
            <button className="qpls2-secondary-action" onClick={() => { void loadSampleDataset().catch((error) => window.alert(error)); }}><FlaskConical size={16} />Load Sample Dataset</button>
            <button className="qpls2-primary-action" disabled={!matrixReady || desktopOnlyMatrix} title={!matrixReady ? "Enter sample size before importing this matrix" : desktopOnlyMatrix ? "Matrix imports require the desktop app" : `Import ${importKindLabel(importKind).toLowerCase()}`} onClick={() => { void importData().catch((error) => window.alert(error)); }}><Upload size={16} />Import Data</button>
          </div>
        </div>
        <details className="validation-details" open={showValidationDetails} onToggle={(event) => setShowValidationDetails(event.currentTarget.open)}>
          <summary>Sample dataset details</summary>
          <span>{validationFixtureSource}</span>
          <code>{validationFixtureDevelopmentPath}</code>
        </details>
      </Panel>

      <Panel title="Data Quality" description="Fast checks before model building." className="data-quality-section data-v2-quality">
        <div className="data-quality-grid">
          <QualityCard label="Rows" value={quality.rows} tone={quality.sampleReady ? "ok" : "warning"} detail={quality.sampleReady ? "Sample-size screen passed" : "Small-sample warning"} />
          <QualityCard label="Variables" value={quality.variables} detail={`${quality.numericVariables} numeric`} />
          <QualityCard label="Missing cells" value={quality.missingCells} tone={quality.missingCells ? "warning" : "ok"} detail={quality.missingCells ? "Review missing policy" : "No missing cells"} />
          <QualityCard label="Nonnumeric" value={quality.nonnumericVariables} tone={quality.nonnumericVariables ? "warning" : "ok"} detail="Check scale metadata" />
          <QualityCard label="Constant columns" value={quality.constantColumns.length} tone={quality.constantColumns.length ? "warning" : "ok"} detail={quality.constantColumns.slice(0, 2).join(", ") || "None detected"} />
          <QualityCard label="Header issues" value={quality.duplicateHeaders.length + quality.invalidHeaders.length} tone={quality.duplicateHeaders.length || quality.invalidHeaders.length ? "warning" : "ok"} detail="Duplicate or spaced names" />
        </div>
      </Panel>
    </div> : null}

    {activeTab === "quality" ? <DataQualityWorkbench quality={quality} dataset={dataset} guidance={guidance} onNavigate={setView} /> : null}

    {activeTab === "quality" && topGuidance ? <section
      className={`data-v215-next-move ${topGuidance.tone}`}
      aria-label="Recommended next move from data"
      data-data-guidance-next-action={topGuidance.actionLabel}
      data-data-guidance-target={topGuidance.actionView}
    >
      <div>
        <span className="qpls2-eyebrow">Recommended next move</span>
        <strong>{topGuidance.actionLabel}</strong>
        <p>{topGuidance.title}: {topGuidance.detail}</p>
      </div>
      <button className="qpls2-secondary-action" onClick={() => setView(topGuidance.actionView)}>{topGuidance.actionLabel}</button>
    </section> : null}

    {activeTab === "notes" ? <div className="data-model-bridge data-v224-notes">
      <div>
        <strong>Create constructs from prefixes</strong>
        <span>{prefixGroups.length ? "Detected grouped indicators that can become reflective constructs." : "No repeated variable prefixes detected. You can still build constructs manually in Model."}</span>
        {prefixGroups.length ? <div className="prefix-preview" aria-label="Detected prefix groups">{prefixGroups.map((group) => <span key={group.prefix}>{group.prefix} {"->"} {group.indicators.length} indicators</span>)}</div> : null}
      </div>
      <button className="qpls2-secondary-action" disabled={!prefixGroups.length} title={prefixGroups.length ? "Create one construct per detected prefix and open Model" : "No repeated variable prefixes were detected"} onClick={createConstructsFromPrefixes}><Boxes size={16} />Create Constructs From Prefixes</button>
      {!prefixGroups.length ? <p className="disabled-reason inline-disabled-reason">Prefix grouping needs at least two variables sharing the same alphabetic prefix.</p> : null}
    </div> : null}

    {activeTab === "variables" ? <VariableView dataset={dataset} selectedColumn={selectedColumn} onSelectColumn={setSelectedColumn} /> : null}

    {activeTab === "data" ? <div className={`data-preview-panel data-v217-preview data-v224-data-view ${importKind !== "raw" ? "matrix-context" : ""}`}>
      <div className="data-preview-header">
        <div><strong>{importKind === "raw" ? "Data preview and metadata" : "Current loaded dataset preview"}</strong><span>{importKind === "raw" ? "Select a column header to edit metadata." : "Matrix import settings are above; this preview remains from the currently loaded dataset."}</span></div>
        <div className="data-table-tools">
          <label><Search size={13} /><input aria-label="Search variables in data preview" placeholder="Find variable" value={columnQuery} onChange={(event) => setColumnQuery(event.target.value)} /></label>
          <select aria-label="Filter variables by metadata" value={columnFilter} onChange={(event) => setColumnFilter(event.target.value as DataColumnFilter)}>
            <option value="all">All columns</option><option value="continuous">Continuous</option><option value="ordinal">Ordinal</option><option value="nominal">Nominal</option><option value="binary">Binary</option><option value="identifier">Identifier</option><option value="nonnumeric">Nonnumeric</option><option value="missing_heavy">Missing-heavy</option>
          </select>
          <div role="group" aria-label="Preview row order">
            <label>Sort preview by<select aria-label="Preview sort column" value={selectedColumn} onChange={(event) => setSelectedColumn(event.target.value)}>{dataset.columns.map((column) => <option key={column} value={column}>{column}</option>)}</select></label>
            <button type="button" aria-pressed={rowSort?.column === selectedColumn && rowSort.direction === "asc"} disabled={!selectedColumn} onClick={() => runVersionedAction({ kind: "sort", column: selectedColumn || null, direction: "asc" })}>Ascending</button>
            <button type="button" aria-pressed={rowSort?.column === selectedColumn && rowSort.direction === "desc"} disabled={!selectedColumn} onClick={() => runVersionedAction({ kind: "sort", column: selectedColumn || null, direction: "desc" })}>Descending</button>
            <button type="button" disabled={!rowSort} onClick={() => { setRowSort(null); status("Preview rows restored to source order. The scientific dataset was not changed.", "info"); }}>Source order</button>
          </div>
        </div>
      </div>
      <details className="validation-details" open>
        <summary>Create an immutable derived variable</summary>
        <div className="data-table-tools" role="group" aria-label="Immutable derived variable controls">
          <label>Operation<select aria-label="Derived variable operation" value={derivedOperation} onChange={(event) => setDerivedOperation(event.target.value as DerivedVariableOperation)}>
            <option value="reverse">Reverse scale</option>
            <option value="add">Add two variables</option>
            <option value="subtract">Subtract variables</option>
            <option value="multiply">Multiply variables</option>
            <option value="divide">Divide variables</option>
            <option value="sum">Row-wise sum</option>
            <option value="mean">Row-wise average</option>
            <option value="dummy">Dummy variable</option>
            <option value="group">Group values</option>
          </select></label>
          <label>Source variable<select aria-label="Derived source variable" value={selectedColumn} onChange={(event) => setSelectedColumn(event.target.value)}>{dataset.columns.map((column) => <option key={column} value={column}>{column}</option>)}</select></label>
          {["add", "subtract", "multiply", "divide"].includes(derivedOperation) ? <label>Right-hand variable<select aria-label="Arithmetic right-hand variable" value={rightColumn} onChange={(event) => setRightColumn(event.target.value)}>{dataset.columns.map((column) => <option key={column} value={column}>{column}</option>)}</select></label> : null}
          {derivedOperation === "reverse" ? <div className="metadata-range">
            <label>Scale minimum<input aria-label="Reverse scale minimum" type="number" value={reverseMinimum} onChange={(event) => setReverseMinimum(event.target.value)} /></label>
            <label>Scale maximum<input aria-label="Reverse scale maximum" type="number" value={reverseMaximum} onChange={(event) => setReverseMaximum(event.target.value)} /></label>
          </div> : null}
          {derivedOperation === "sum" || derivedOperation === "mean" ? <>
            <label>Input variables<input aria-label="Aggregate input variables" value={aggregateColumns} placeholder="item1, item2, item3" onChange={(event) => setAggregateColumns(event.target.value)} /></label>
            <label>Missing values<select aria-label="Aggregate missing policy" value={aggregateMissingPolicy} onChange={(event) => setAggregateMissingPolicy(event.target.value as "propagate" | "available")}><option value="propagate">Require every value</option><option value="available">Use available values</option></select></label>
            {aggregateMissingPolicy === "available" ? <label>Minimum observed<input aria-label="Minimum non-missing values" type="number" min="1" value={minimumNonMissing} onChange={(event) => setMinimumNonMissing(event.target.value)} /></label> : null}
          </> : null}
          {derivedOperation === "dummy" ? <>
            <label>Value mapped to 1<input aria-label="Dummy match value" value={dummyMatchValue} onChange={(event) => setDummyMatchValue(event.target.value)} /></label>
            <label>Missing source<select aria-label="Dummy missing policy" value={dummyMissingPolicy} onChange={(event) => setDummyMissingPolicy(event.target.value as "missing" | "zero")}><option value="missing">Keep missing</option><option value="zero">Map to 0</option></select></label>
          </> : null}
          {derivedOperation === "group" ? <>
            <label>Value groups<textarea aria-label="Value group rules" placeholder="A, B = Treatment; C = Control" value={groupRules} onChange={(event) => setGroupRules(event.target.value)} /></label>
            <label>Unmatched values<select aria-label="Unmatched group policy" value={groupUnmatched} onChange={(event) => setGroupUnmatched(event.target.value as "missing" | "error")}><option value="missing">Set missing</option><option value="error">Reject transformation</option></select></label>
          </> : null}
          <label>Output variable<input aria-label="Derived output variable" placeholder="Automatic unique name" value={derivedOutputName} onChange={(event) => setDerivedOutputName(event.target.value)} /></label>
          <button type="button" className="qpls2-secondary-action" onClick={createDerivedVariable}>Create dataset version</button>
        </div>
        <p><Info size={13} />Every operation creates one immutable child dataset with native lineage. Browser preview remains read-only.</p>
      </details>
      <div className="data-scroll-hint" role="status" aria-live="polite">Showing {visibleColumns.length} of {dataset.columns.length} columns{filteredOutCount ? `, ${filteredOutCount} hidden by filter` : ""}. {rowSort ? `Preview rows are ${rowSort.direction === "desc" ? "descending" : "ascending"} by ${rowSort.column}; source order is unchanged.` : "Preview rows use source order."} Scroll horizontally to inspect all visible variables.</div>
      <div className="data-workbench">
        <div className="data-grid" tabIndex={0} role="region" aria-label={`Data preview table for ${dataset.name}`}><table><caption>Data preview: {Math.min(100, dataset.rows.length)} displayed rows of {dataset.name}; row numbers retain source order</caption><thead><tr><th>#</th>{visibleColumns.map((column) => { const metadata = dataset.columnMetadata?.find((item) => item.name === column); const activeSort = rowSort?.column === column ? rowSort.direction === "asc" ? "ascending" : "descending" : "none"; return <th aria-sort={activeSort} className={selectedColumn === column ? "selected-column" : ""} key={column} onClick={() => setSelectedColumn(column)}><button type="button">{column}</button><small>{metadata?.scale_type ?? metadata?.column_type ?? "Numeric"}</small></th>; })}</tr></thead><tbody>{previewRows.map(({ row, sourceIndex }) => <tr key={sourceIndex}><td>{sourceIndex + 1}</td>{visibleColumns.map((column) => <td key={column}>{row[column] ?? <span className="missing-value">missing</span>}</td>)}</tr>)}</tbody></table></div>
        <aside className="metadata-editor" aria-label="Column metadata">
          <div className="metadata-heading"><strong>{selectedColumn || "No column selected"}</strong><span>Selected column metadata</span></div>
          <p className="metadata-help">Select a column header to edit metadata. Import missing markers are applied during import and do not recode already-loaded values.</p>
          {selectedProfile ? <div className="column-profile-wrap"><strong>Column profile</strong><dl className="column-profile" aria-label="Selected column profile">
            <div><dt>Complete</dt><dd>{selectedProfile.complete}</dd></div>
            <div><dt>Missing</dt><dd>{selectedProfile.missing}</dd></div>
            <div><dt>Unique</dt><dd>{selectedProfile.unique}</dd></div>
            <div><dt>Min</dt><dd>{formatProfileValue(selectedProfile.min)}</dd></div>
            <div><dt>Max</dt><dd>{formatProfileValue(selectedProfile.max)}</dd></div>
            <div><dt>Mean</dt><dd>{formatProfileValue(selectedProfile.mean)}</dd></div>
            <div><dt>Standard deviation</dt><dd>{formatProfileValue(selectedProfile.standardDeviation)}</dd></div>
          </dl></div> : null}
          <details open><summary>Essentials</summary>
            <label>Label<input value={draft.label ?? ""} onChange={(event) => setDraft({ ...draft, label: event.target.value || null })} /></label>
            <label>Scale<select value={draft.scale_type} onChange={(event) => setDraft({ ...draft, scale_type: event.target.value as ColumnMetadata["scale_type"] })}><option value="continuous">Continuous</option><option value="ordinal">Ordinal</option><option value="nominal">Nominal</option><option value="binary">Binary</option><option value="identifier">Identifier</option></select></label>
            <label>Import missing markers<input value={draft.missing_markers.join(", ")} readOnly title="Missing markers are applied when the dataset is imported" /></label>
          </details>
          <details><summary>Bounds</summary>
            <div className="metadata-range"><label>Minimum<input type="number" value={draft.theoretical_min ?? ""} onChange={(event) => setDraft({ ...draft, theoretical_min: event.target.value === "" ? null : Number(event.target.value) })} /></label><label>Maximum<input type="number" value={draft.theoretical_max ?? ""} onChange={(event) => setDraft({ ...draft, theoretical_max: event.target.value === "" ? null : Number(event.target.value) })} /></label></div>
          </details>
          <div className="metadata-actions"><button className="qpls2-secondary-action" disabled={!selectedColumn} onClick={() => { void saveMetadata().catch((error) => window.alert(error)); }}><Save size={15} />Apply metadata</button><button className="qpls2-secondary-action" disabled={!selectedColumn} onClick={() => setDraft(selectedMetadata)}>Reset draft</button></div>
          {!selectedColumn ? <p className="disabled-reason inline-disabled-reason">Select a column in the data preview to edit and apply metadata.</p> : null}
        </aside>
      </div>
    </div> : null}
    <input ref={inputRef} className="file-input" type="file" accept=".csv,.tsv" onChange={(event) => {
      const file = event.target.files?.[0]; if (!file) return;
      void file.text().then((csv) => setParsedDataset(csv, file.name));
    }} />
  </WorkspacePage>;
}

function GuidancePanel({ title, items, onNavigate }: { title: string; items: Array<{ title: string; detail: string; tone: "validated" | "warning" | "neutral"; actionLabel: string; actionView: WorkspaceView }>; onNavigate: (view: WorkspaceView) => void }) {
  return <section className="method-guidance-panel" aria-label={title}>
    <header><strong>{title}</strong><span>QuickPLS filters methods by file type, metadata, sample rows, and current model state.</span></header>
    <div className="method-guidance-panel-grid">
      {items.map((item) => <article key={`${item.title}-${item.actionLabel}`} className={`method-guidance-mini ${item.tone}`}>
        <strong>{item.title}</strong>
        <p>{item.detail}</p>
        <button type="button" className="secondary-button" onClick={() => onNavigate(item.actionView)}>{item.actionLabel}</button>
      </article>)}
    </div>
  </section>;
}

function VariableView({ dataset, selectedColumn, onSelectColumn }: { dataset: Dataset; selectedColumn: string; onSelectColumn: (column: string) => void }) {
  const metadata = dataset.columnMetadata ?? dataset.columns.map(defaultMetadata);
  return <section className="data-v224-variable-view" aria-label="Variable View">
    <header className="data-v224-section-head">
      <div>
        <strong>Variable View</strong>
        <span>one row per variable, with editable metadata available from the property inspector in Data View.</span>
      </div>
      <small>{metadata.length} variables</small>
    </header>
    <div className="data-v224-variable-grid" role="region" aria-label="Variable metadata table">
      <table>
        <thead><tr><th>Name</th><th>Label</th><th>Type</th><th>Scale</th><th>Missing</th><th>Complete</th><th>Unique</th><th>Min</th><th>Max</th></tr></thead>
        <tbody>{metadata.map((column) => {
          const profile = columnProfile(dataset, column.name);
          return <tr key={column.name} className={selectedColumn === column.name ? "selected" : ""} onClick={() => onSelectColumn(column.name)}>
            <th><button type="button">{column.name}</button></th>
            <td>{column.label || "Not labeled"}</td>
            <td>{column.column_type}</td>
            <td>{column.scale_type}</td>
            <td>{profile.missing}</td>
            <td>{profile.complete}</td>
            <td>{profile.unique}</td>
            <td>{formatProfileValue(profile.min)}</td>
            <td>{formatProfileValue(profile.max)}</td>
          </tr>;
        })}</tbody>
      </table>
    </div>
  </section>;
}

function DataQualityWorkbench({ quality, dataset, guidance, onNavigate }: { quality: DataQualitySummary; dataset: Dataset; guidance: Array<{ title: string; detail: string; tone: "validated" | "warning" | "neutral"; actionLabel: string; actionView: WorkspaceView }>; onNavigate: (view: WorkspaceView) => void }) {
  const issueRows = [
    ...quality.constantColumns.map((column) => ({ severity: "Review", variable: column, issue: "Constant column", action: "Remove, recode, or exclude before modeling if not theoretically required." })),
    ...quality.missingHeavyColumns.map((column) => ({ severity: "Review", variable: column, issue: "Missing-heavy column", action: "Inspect missing policy and complete cases before running." })),
    ...quality.invalidHeaders.map((column) => ({ severity: "Review", variable: column || "(blank)", issue: "Invalid header", action: "Rename before export or reproducible handoff." })),
    ...quality.duplicateHeaders.map((column) => ({ severity: "Issue", variable: column, issue: "Duplicate header", action: "Use unique variable names before analysis." })),
  ];
  return <section className="data-v224-quality-view" aria-label="Data Quality">
    <Panel title="Data Quality" description="Readiness checks with direct variable-level follow-up." className="data-quality-section data-v2-quality">
      <div className="data-quality-grid">
        <QualityCard label="Rows" value={quality.rows} tone={quality.sampleReady ? "ok" : "warning"} detail={quality.sampleReady ? "Sample-size screen passed" : "Small-sample warning"} />
        <QualityCard label="Variables" value={quality.variables} detail={`${quality.numericVariables} numeric`} />
        <QualityCard label="Missing cells" value={quality.missingCells} tone={quality.missingCells ? "warning" : "ok"} detail={quality.missingCells ? "Review missing policy" : "No missing cells"} />
        <QualityCard label="Nonnumeric" value={quality.nonnumericVariables} tone={quality.nonnumericVariables ? "warning" : "ok"} detail="Check scale metadata" />
        <QualityCard label="Constant columns" value={quality.constantColumns.length} tone={quality.constantColumns.length ? "warning" : "ok"} detail={quality.constantColumns.slice(0, 2).join(", ") || "None detected"} />
        <QualityCard label="Header issues" value={quality.duplicateHeaders.length + quality.invalidHeaders.length} tone={quality.duplicateHeaders.length || quality.invalidHeaders.length ? "warning" : "ok"} detail="Duplicate or spaced names" />
      </div>
    </Panel>
    <div className="data-v224-quality-grid">
      <div className="data-v224-issue-table">
        <header><strong>Variable issues</strong><span>{issueRows.length ? `${issueRows.length} item${issueRows.length === 1 ? "" : "s"} to review` : "No variable-level issues detected"}</span></header>
        <table>
          <thead><tr><th>Severity</th><th>Variable</th><th>Issue</th><th>Recommended action</th></tr></thead>
          <tbody>{issueRows.length ? issueRows.map((row) => <tr key={`${row.issue}-${row.variable}`}><td>{row.severity}</td><th>{row.variable}</th><td>{row.issue}</td><td>{row.action}</td></tr>) : <tr><td colSpan={4}>No duplicate headers, constant columns, or missing-heavy variables were detected in {dataset.name}.</td></tr>}</tbody>
        </table>
      </div>
      <GuidancePanel title="What can I do with this data?" items={guidance} onNavigate={onNavigate} />
    </div>
  </section>;
}

function QualityCard({ label, value, detail, tone = "neutral" }: { label: string; value: string | number; detail: string; tone?: "neutral" | "ok" | "warning" }) {
  const metricTone = tone === "ok" ? "success" : tone === "warning" ? "warning" : "info";
  return <MetricCard
    label={label}
    value={value}
    tone={metricTone}
    detail={<>{tone === "ok" ? <CheckCircle2 size={12} /> : tone === "warning" ? <AlertTriangle size={12} /> : <Database size={12} />}{detail}</>}
  />;
}

function formatProfileValue(value: number | null) {
  return value == null || !Number.isFinite(value) ? "N/A" : Math.abs(value) >= 100 ? value.toFixed(1) : value.toFixed(3);
}
