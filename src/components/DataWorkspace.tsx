import { AlertTriangle, Boxes, CheckCircle2, Database, FlaskConical, Info, Save, Search, Upload } from "lucide-react";
import Papa from "papaparse";
import { useEffect, useMemo, useRef, useState } from "react";
import validationFixture from "../../validation/fixtures/corporate_reputation.csv?raw";
import { columnProfile, dataQualitySummary, detectPrefixGroups, filteredColumns, type DataColumnFilter } from "../domain/dataWorkspace";
import { importNativeDataset, importNativeValidationFixture, isNativeDesktop, updateNativeColumnMetadata } from "../services/projectService";
import { useWorkspace } from "../store";
import type { ColumnMetadata } from "../types";

type ImportKind = "raw" | "covariance" | "correlation";

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

export function DataWorkspace() {
  const inputRef = useRef<HTMLInputElement>(null);
  const dataset = useWorkspace((state) => state.dataset);
  const setDataset = useWorkspace((state) => state.setDataset);
  const setView = useWorkspace((state) => state.setView);
  const addConstructsFromIndicatorGroups = useWorkspace((state) => state.addConstructsFromIndicatorGroups);
  const [importKind, setImportKind] = useState<ImportKind>("raw");
  const [sampleSize, setSampleSize] = useState("");
  const [missingMarkers, setMissingMarkers] = useState("NA, N/A, .");
  const [selectedColumn, setSelectedColumn] = useState(dataset.columns[0] ?? "");
  const [columnQuery, setColumnQuery] = useState("");
  const [columnFilter, setColumnFilter] = useState<DataColumnFilter>("all");
  const [showValidationDetails, setShowValidationDetails] = useState(false);
  const selectedMetadata = useMemo(() => dataset.columnMetadata?.find((column) => column.name === selectedColumn) ?? defaultMetadata(selectedColumn), [dataset.columnMetadata, selectedColumn]);
  const selectedProfile = useMemo(() => selectedColumn ? columnProfile(dataset, selectedColumn) : null, [dataset, selectedColumn]);
  const [draft, setDraft] = useState<ColumnMetadata>(selectedMetadata);
  const quality = useMemo(() => dataQualitySummary(dataset), [dataset]);
  const prefixGroups = useMemo(() => detectPrefixGroups(dataset.columns), [dataset.columns]);
  const visibleColumns = useMemo(() => filteredColumns(dataset, columnQuery, columnFilter), [dataset, columnFilter, columnQuery]);
  const matrixSampleSize = sampleSize === "" ? undefined : Number(sampleSize);
  const matrixReady = importKind === "raw" || Boolean(matrixSampleSize && matrixSampleSize >= 2);
  const desktopOnlyMatrix = importKind !== "raw" && !isNativeDesktop();
  const filteredOutCount = Math.max(0, dataset.columns.length - visibleColumns.length);

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

  return <section className="workspace-page data-page">
    <div className="page-heading-pro">
      <div>
        <div className="page-kicker">Data workspace</div>
        <h1>{dataset.name}</h1>
        <p>{quality.rows} rows, {quality.variables} variables, {quality.missingCells} missing cells{dataset.kind && dataset.kind !== "raw" ? `, ${dataset.kind} matrix (n=${dataset.sampleSize ?? "unknown"})` : ""}</p>
        {quality.sampleWarning ? <p className="data-title-warning"><AlertTriangle size={13} />{quality.sampleWarning}</p> : null}
      </div>
      <button className="run-button" disabled={!dataset.columns.length} title={dataset.columns.length ? "Continue to the SEM diagram designer" : "Import a dataset before building the model"} onClick={() => setView("models")}>Open Model Designer</button>
    </div>

    <div className="data-import-panel" aria-label="Import source">
      <div className="data-import-mode">
        <div><strong>Import source</strong><span>Choose the file shape before importing. Current project data stays visible until a new file is imported.</span></div>
        <select aria-label="Data type" value={importKind} onChange={(event) => setImportKind(event.target.value as ImportKind)}><option value="raw">Raw data</option><option value="covariance">Covariance matrix</option><option value="correlation">Correlation matrix</option></select>
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
          <button className="secondary-button" onClick={() => { void loadSampleDataset().catch((error) => window.alert(error)); }}><FlaskConical size={16} />Load Sample Dataset</button>
          <button className="run-button" disabled={!matrixReady || desktopOnlyMatrix} title={!matrixReady ? "Enter sample size before importing this matrix" : desktopOnlyMatrix ? "Matrix imports require the desktop app" : `Import ${importKindLabel(importKind).toLowerCase()}`} onClick={() => { void importData().catch((error) => window.alert(error)); }}><Upload size={16} />Import Data</button>
        </div>
      </div>
      <details className="validation-details" open={showValidationDetails} onToggle={(event) => setShowValidationDetails(event.currentTarget.open)}>
        <summary>Sample dataset details</summary>
        <span>{validationFixtureSource}</span>
        <code>{validationFixtureDevelopmentPath}</code>
      </details>
    </div>

    <div className="data-quality-section" aria-label="Data quality summary">
      <div className="data-section-heading"><strong>Data Quality</strong><span>Fast checks before model building</span></div>
      <div className="data-quality-grid">
        <QualityCard label="Rows" value={quality.rows} tone={quality.sampleReady ? "ok" : "warning"} detail={quality.sampleReady ? "Sample-size screen passed" : "Small-sample warning"} />
        <QualityCard label="Variables" value={quality.variables} detail={`${quality.numericVariables} numeric`} />
        <QualityCard label="Missing cells" value={quality.missingCells} tone={quality.missingCells ? "warning" : "ok"} detail={quality.missingCells ? "Review missing policy" : "No missing cells"} />
        <QualityCard label="Nonnumeric" value={quality.nonnumericVariables} tone={quality.nonnumericVariables ? "warning" : "ok"} detail="Check scale metadata" />
        <QualityCard label="Constant columns" value={quality.constantColumns.length} tone={quality.constantColumns.length ? "warning" : "ok"} detail={quality.constantColumns.slice(0, 2).join(", ") || "None detected"} />
        <QualityCard label="Header issues" value={quality.duplicateHeaders.length + quality.invalidHeaders.length} tone={quality.duplicateHeaders.length || quality.invalidHeaders.length ? "warning" : "ok"} detail="Duplicate or spaced names" />
      </div>
    </div>

    <div className="data-model-bridge">
      <div>
        <strong>Create constructs from prefixes</strong>
        <span>{prefixGroups.length ? "Detected grouped indicators that can become reflective constructs." : "No repeated variable prefixes detected. You can still build constructs manually in Model."}</span>
        {prefixGroups.length ? <div className="prefix-preview" aria-label="Detected prefix groups">{prefixGroups.map((group) => <span key={group.prefix}>{group.prefix} {"->"} {group.indicators.length} indicators</span>)}</div> : null}
      </div>
      <button className="secondary-button" disabled={!prefixGroups.length} title={prefixGroups.length ? "Create one construct per detected prefix and open Model" : "No repeated variable prefixes were detected"} onClick={createConstructsFromPrefixes}><Boxes size={16} />Create Constructs From Prefixes</button>
      {!prefixGroups.length ? <p className="disabled-reason inline-disabled-reason">Prefix grouping needs at least two variables sharing the same alphabetic prefix.</p> : null}
    </div>

    <div className={`data-preview-panel ${importKind !== "raw" ? "matrix-context" : ""}`}>
      <div className="data-preview-header">
        <div><strong>{importKind === "raw" ? "Data preview and metadata" : "Current loaded dataset preview"}</strong><span>{importKind === "raw" ? "Select a column header to edit metadata." : "Matrix import settings are above; this preview remains from the currently loaded dataset."}</span></div>
        <div className="data-table-tools">
          <label><Search size={13} /><input aria-label="Search variables in data preview" placeholder="Find variable" value={columnQuery} onChange={(event) => setColumnQuery(event.target.value)} /></label>
          <select aria-label="Filter variables by metadata" value={columnFilter} onChange={(event) => setColumnFilter(event.target.value as DataColumnFilter)}>
            <option value="all">All columns</option><option value="continuous">Continuous</option><option value="ordinal">Ordinal</option><option value="nominal">Nominal</option><option value="binary">Binary</option><option value="identifier">Identifier</option><option value="nonnumeric">Nonnumeric</option><option value="missing_heavy">Missing-heavy</option>
          </select>
        </div>
      </div>
      <div className="data-scroll-hint">Showing {visibleColumns.length} of {dataset.columns.length} columns{filteredOutCount ? `, ${filteredOutCount} hidden by filter` : ""}. Scroll horizontally to inspect all visible variables.</div>
      <div className="data-workbench">
        <div className="data-grid" tabIndex={0} role="region" aria-label={`Data preview table for ${dataset.name}`}><table><caption>Data preview: first {Math.min(100, dataset.rows.length)} rows of {dataset.name}</caption><thead><tr><th>#</th>{visibleColumns.map((column) => { const metadata = dataset.columnMetadata?.find((item) => item.name === column); return <th className={selectedColumn === column ? "selected-column" : ""} key={column} onClick={() => setSelectedColumn(column)}><button type="button">{column}</button><small>{metadata?.scale_type ?? metadata?.column_type ?? "Numeric"}</small></th>; })}</tr></thead><tbody>{dataset.rows.slice(0, 100).map((row, index) => <tr key={index}><td>{index + 1}</td>{visibleColumns.map((column) => <td key={column}>{row[column] ?? <span className="missing-value">missing</span>}</td>)}</tr>)}</tbody></table></div>
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
          <div className="metadata-actions"><button className="secondary-button" disabled={!selectedColumn} onClick={() => { void saveMetadata().catch((error) => window.alert(error)); }}><Save size={15} />Apply metadata</button><button className="secondary-button" disabled={!selectedColumn} onClick={() => setDraft(selectedMetadata)}>Reset draft</button></div>
          {!selectedColumn ? <p className="disabled-reason inline-disabled-reason">Select a column in the data preview to edit and apply metadata.</p> : null}
        </aside>
      </div>
    </div>
    <input ref={inputRef} className="file-input" type="file" accept=".csv,.tsv" onChange={(event) => {
      const file = event.target.files?.[0]; if (!file) return;
      void file.text().then((csv) => setParsedDataset(csv, file.name));
    }} />
  </section>;
}

function QualityCard({ label, value, detail, tone = "neutral" }: { label: string; value: string | number; detail: string; tone?: "neutral" | "ok" | "warning" }) {
  return <article className={`data-quality-card ${tone}`}><span>{label}</span><strong>{value}</strong><small>{tone === "ok" ? <CheckCircle2 size={12} /> : tone === "warning" ? <AlertTriangle size={12} /> : <Database size={12} />}{detail}</small></article>;
}

function formatProfileValue(value: number | null) {
  return value == null || !Number.isFinite(value) ? "N/A" : Math.abs(value) >= 100 ? value.toFixed(1) : value.toFixed(3);
}
