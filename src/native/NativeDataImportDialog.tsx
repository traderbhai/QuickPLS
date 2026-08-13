import { useState } from "react";
import {
  DEFAULT_NATIVE_DATA_IMPORT_DRAFT,
  validateNativeDataImportDraft,
  type NativeDataImportDraft,
  type NativeDataImportRequest,
  type NativeDataKind,
} from "./nativeDataImport";

interface NativeDataImportDialogProps {
  close: () => void;
  importData: (request: NativeDataImportRequest) => void;
}

const DATA_KINDS: ReadonlyArray<{ id: NativeDataKind; label: string; detail: string }> = [
  { id: "raw", label: "Raw data", detail: "Cases in rows and variables in columns" },
  { id: "covariance", label: "Covariance matrix", detail: "Square labeled matrix with a declared sample size" },
  { id: "correlation", label: "Correlation matrix", detail: "Square labeled matrix with a declared sample size" },
];

export function NativeDataImportDialog({ close, importData }: NativeDataImportDialogProps) {
  const [draft, setDraft] = useState<NativeDataImportDraft>({ ...DEFAULT_NATIVE_DATA_IMPORT_DRAFT });
  const [error, setError] = useState<string | null>(null);
  const matrix = draft.dataKind !== "raw";
  const update = <K extends keyof NativeDataImportDraft>(key: K, value: NativeDataImportDraft[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setError(null);
  };
  const submit = () => {
    const validation = validateNativeDataImportDraft(draft);
    if (!validation.request) {
      setError(validation.error);
      return;
    }
    importData(validation.request);
  };

  return <form className="nd-import-data-dialog" onSubmit={(event) => { event.preventDefault(); submit(); }}>
    <div className="nd-import-data-content">
      <fieldset>
        <legend>Data format</legend>
        <div className="nd-import-kind-list">
          {DATA_KINDS.map((kind, index) => <label key={kind.id} className={draft.dataKind === kind.id ? "active" : ""}>
            <input autoFocus={index === 0} type="radio" name="data-kind" value={kind.id} checked={draft.dataKind === kind.id} onChange={() => update("dataKind", kind.id)} />
            <span><strong>{kind.label}</strong><small>{kind.detail}</small></span>
          </label>)}
        </div>
      </fieldset>
      {matrix ? <label>Study sample size
        <input type="number" min={2} step={1} value={draft.sampleSize} onChange={(event) => update("sampleSize", event.target.value)} aria-describedby="nd-import-sample-help" />
        <small id="nd-import-sample-help">Required because a matrix file does not contain individual cases.</small>
      </label> : null}
      <label>Missing-value markers
        <input type="text" value={draft.missingMarkers} onChange={(event) => update("missingMarkers", event.target.value)} aria-describedby="nd-import-missing-help" />
        <small id="nd-import-missing-help">Comma-separated markers. Blank cells are always treated as missing.</small>
      </label>
      <p className="nd-import-file-types">Supported files: CSV, TSV, TXT, Excel, SPSS, ODS, covariance matrices, and correlation matrices.</p>
      {error ? <p className="nd-form-error" role="alert">{error}</p> : null}
    </div>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">Choose File…</button></footer>
  </form>;
}
