import { FlaskConical, Save, Upload } from "lucide-react";
import Papa from "papaparse";
import { useEffect, useMemo, useRef, useState } from "react";
import validationFixture from "../../validation/fixtures/corporate_reputation.csv?raw";
import { importNativeDataset, importNativeValidationFixture, isNativeDesktop, updateNativeColumnMetadata } from "../services/projectService";
import { useWorkspace } from "../store";
import type { ColumnMetadata } from "../types";

type ImportKind = "raw" | "covariance" | "correlation";
const validationFixtureSource = "Bundled fixture: validation/fixtures/corporate_reputation.csv";
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

export function DataWorkspace() {
  const inputRef = useRef<HTMLInputElement>(null);
  const dataset = useWorkspace((state) => state.dataset);
  const setDataset = useWorkspace((state) => state.setDataset);
  const setView = useWorkspace((state) => state.setView);
  const [importKind, setImportKind] = useState<ImportKind>("raw");
  const [sampleSize, setSampleSize] = useState("");
  const [missingMarkers, setMissingMarkers] = useState("NA, N/A, .");
  const [selectedColumn, setSelectedColumn] = useState(dataset.columns[0] ?? "");
  const selectedMetadata = useMemo(() => dataset.columnMetadata?.find((column) => column.name === selectedColumn) ?? defaultMetadata(selectedColumn), [dataset.columnMetadata, selectedColumn]);
  const [draft, setDraft] = useState<ColumnMetadata>(selectedMetadata);

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
    const parsedSampleSize = sampleSize === "" ? undefined : Number(sampleSize);
    if (importKind !== "raw" && (!parsedSampleSize || parsedSampleSize < 2)) {
      window.alert("Enter the study sample size before importing a covariance or correlation matrix."); return;
    }
    if (!isNativeDesktop()) {
      if (importKind !== "raw") { window.alert("Matrix imports require the native QuickPLS desktop application."); return; }
      inputRef.current?.click(); return;
    }
    const imported = await importNativeDataset(importKind, parsedSampleSize, ["", ...missingMarkers.split(",").map((value) => value.trim()).filter(Boolean)]);
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

  const loadValidationFixture = async () => {
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

  return <section className="workspace-page data-page">
    <div className="page-heading">
      <div><h1>{dataset.name}</h1><p>{dataset.rowCount ?? dataset.rows.length} rows, {dataset.columns.length} variables, {dataset.missing} missing cells{dataset.kind && dataset.kind !== "raw" ? `, ${dataset.kind} matrix (n=${dataset.sampleSize ?? "unknown"})` : ""}</p></div>
      <div className="data-import-controls">
        <select aria-label="Data type" value={importKind} onChange={(event) => setImportKind(event.target.value as ImportKind)}><option value="raw">Raw data</option><option value="covariance">Covariance matrix</option><option value="correlation">Correlation matrix</option></select>
        {importKind !== "raw" && <input aria-label="Sample size" type="number" min="2" placeholder="Sample size" value={sampleSize} onChange={(event) => setSampleSize(event.target.value)} />}
        {importKind === "raw" && <input className="missing-markers-input" aria-label="Missing value markers" title="Comma-separated missing value markers" placeholder="Missing: NA, ." value={missingMarkers} onChange={(event) => setMissingMarkers(event.target.value)} />}
        <button className="secondary-button" title={validationFixtureSource} onClick={() => { void loadValidationFixture().catch((error) => window.alert(error)); }}><FlaskConical size={16} />Validation fixture</button>
        <button className="secondary-button" onClick={() => { void importData().catch((error) => window.alert(error)); }}><Upload size={16} />Import data</button>
        <button className="secondary-button" disabled={!dataset.columns.length} title={dataset.columns.length ? "Continue to SEM diagram designer" : "Import a dataset before building the model"} onClick={() => setView("models")}>Build model</button>
      </div>
    </div>
    <div className="data-next-step-card">
      <div><strong>Next step after data import</strong><span>Review column metadata, then create constructs from prefixes or continue to the SEM designer.</span></div>
      <button className="secondary-button" disabled={!dataset.columns.length} onClick={() => setView("models")}>Open Model</button>
    </div>
    <div className="fixture-source-card">
      <FlaskConical size={16} />
      <div><strong>Validation fixture</strong><span>{validationFixtureSource}</span><code>{validationFixtureDevelopmentPath}</code></div>
      <button className="secondary-button" onClick={() => { void loadValidationFixture().catch((error) => window.alert(error)); }}>Load fixture</button>
    </div>
    <input ref={inputRef} className="file-input" type="file" accept=".csv,.tsv" onChange={(event) => {
      const file = event.target.files?.[0]; if (!file) return;
      void file.text().then((csv) => setParsedDataset(csv, file.name));
    }} />
    <div className="data-workbench">
      <div className="data-grid" tabIndex={0} role="region" aria-label={`Data preview table for ${dataset.name}`}><table><caption>Data preview: first {Math.min(100, dataset.rows.length)} rows of {dataset.name}</caption><thead><tr><th>#</th>{dataset.columns.map((column) => { const metadata = dataset.columnMetadata?.find((item) => item.name === column); return <th className={selectedColumn === column ? "selected-column" : ""} key={column} onClick={() => setSelectedColumn(column)}>{column}<small>{metadata?.scale_type ?? metadata?.column_type ?? "Numeric"}</small></th>; })}</tr></thead><tbody>{dataset.rows.slice(0, 100).map((row, index) => <tr key={index}><td>{index + 1}</td>{dataset.columns.map((column) => <td key={column}>{row[column] ?? <span className="missing-value">missing</span>}</td>)}</tr>)}</tbody></table></div>
      <aside className="metadata-editor" aria-label="Column metadata">
        <div className="metadata-heading"><strong>{selectedColumn || "No column"}</strong><span>Column metadata</span></div>
        <label>Label<input value={draft.label ?? ""} onChange={(event) => setDraft({ ...draft, label: event.target.value || null })} /></label>
        <label>Scale<select value={draft.scale_type} onChange={(event) => setDraft({ ...draft, scale_type: event.target.value as ColumnMetadata["scale_type"] })}><option value="continuous">Continuous</option><option value="ordinal">Ordinal</option><option value="nominal">Nominal</option><option value="binary">Binary</option><option value="identifier">Identifier</option></select></label>
        <label>Import missing markers<input value={draft.missing_markers.join(", ")} readOnly title="Missing markers are applied when the dataset is imported" /></label>
        <div className="metadata-range"><label>Minimum<input type="number" value={draft.theoretical_min ?? ""} onChange={(event) => setDraft({ ...draft, theoretical_min: event.target.value === "" ? null : Number(event.target.value) })} /></label><label>Maximum<input type="number" value={draft.theoretical_max ?? ""} onChange={(event) => setDraft({ ...draft, theoretical_max: event.target.value === "" ? null : Number(event.target.value) })} /></label></div>
        <button className="secondary-button" disabled={!selectedColumn} onClick={() => { void saveMetadata().catch((error) => window.alert(error)); }}><Save size={15} />Apply metadata</button>
        {!selectedColumn ? <p className="disabled-reason inline-disabled-reason">Select a column in the data preview to edit and apply metadata.</p> : null}
      </aside>
    </div>
  </section>;
}
