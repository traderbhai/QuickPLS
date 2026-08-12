import { Info, Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { Dataset, RecodeColumnSpec } from "../types";
import {
  defaultNativeRecodeDraft,
  validateNativeRecodeDraft,
  type NativeRecodeDraft,
  type NativeRecodeTargetType,
} from "./nativeRecode";
import { runNativeScopedSubmission } from "./nativeScopedSubmission";

interface NativeRecodeDialogProps {
  dataset: Dataset;
  sourceColumn: string;
  nativeDesktop: boolean;
  projectWritable: boolean;
  dialogScope: number;
  close: () => void;
  complete: (dialogScope: number) => void;
  onBusyChange: (dialogScope: number, busy: boolean) => void;
  recode: (spec: RecodeColumnSpec) => Promise<void>;
}

function scaleForType(type: NativeRecodeTargetType, current: NativeRecodeDraft["targetScale"]): NativeRecodeDraft["targetScale"] {
  if (type === "boolean") return "binary";
  if (type === "text" && current === "continuous") return "nominal";
  return current;
}

export function nativeRecodeIssueFieldId(fieldPrefix: string, path: string): string | null {
  if (path === "sourceColumn") return `${fieldPrefix}-source`;
  if (path === "targetColumn") return `${fieldPrefix}-target`;
  if (path === "targetType") return `${fieldPrefix}-type`;
  if (path === "targetScale") return `${fieldPrefix}-scale`;
  if (path === "unmapped") return `${fieldPrefix}-unmapped`;
  if (path === "mappings") return `${fieldPrefix}-source-0`;
  const mapping = /^mappings\.(\d+)\.(source|target)$/.exec(path);
  return mapping ? `${fieldPrefix}-${mapping[2]}-${mapping[1]}` : null;
}

export function NativeRecodeDialog({ dataset, sourceColumn, nativeDesktop, projectWritable, dialogScope, close, complete, onBusyChange, recode }: NativeRecodeDialogProps) {
  const initialDraft = useMemo(() => defaultNativeRecodeDraft(dataset, sourceColumn), [dataset, sourceColumn]);
  const [draft, setDraft] = useState<NativeRecodeDraft>(initialDraft);
  const [status, setStatus] = useState<"idle" | "saving" | "error">("idle");
  const [error, setError] = useState<string | null>(null);
  const [invalidPath, setInvalidPath] = useState<string | null>(null);
  const mountedRef = useRef(true);
  const submissionRef = useRef(0);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      submissionRef.current += 1;
      onBusyChange(dialogScope, false);
    };
  }, [dialogScope, onBusyChange]);

  useEffect(() => {
    if (status === "saving") return;
    setDraft(initialDraft);
    setStatus("idle");
    setError(null);
    setInvalidPath(null);
    // The active mutation owns the draft until its scoped completion closes the dialog.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialDraft]);

  const update = <K extends keyof NativeRecodeDraft>(key: K, value: NativeRecodeDraft[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setStatus("idle");
    setError(null);
    setInvalidPath(null);
  };
  const updateMapping = (index: number, key: "source" | "target", value: string) => {
    update("mappings", draft.mappings.map((mapping, mappingIndex) => mappingIndex === index ? { ...mapping, [key]: value } : mapping));
  };
  const setTargetType = (targetType: NativeRecodeTargetType) => {
    setDraft((current) => ({ ...current, targetType, targetScale: scaleForType(targetType, current.targetScale) }));
    setStatus("idle");
    setError(null);
    setInvalidPath(null);
  };

  const submit = async () => {
    if (!nativeDesktop || !projectWritable || status === "saving") return;
    const validation = validateNativeRecodeDraft(dataset, draft);
    if (!validation.spec) {
      setStatus("error");
      setError(validation.error);
      const firstPath = validation.issues[0]?.path ?? null;
      setInvalidPath(firstPath);
      const fieldId = firstPath ? nativeRecodeIssueFieldId(fieldPrefix, firstPath) : null;
      if (fieldId) window.setTimeout(() => document.getElementById(fieldId)?.focus(), 0);
      return;
    }
    const submission = submissionRef.current + 1;
    submissionRef.current = submission;
    setStatus("saving");
    setError(null);
    setInvalidPath(null);
    await runNativeScopedSubmission({
      perform: () => recode(validation.spec as RecodeColumnSpec),
      isCurrent: () => mountedRef.current && submissionRef.current === submission,
      setBusy: (busy) => onBusyChange(dialogScope, busy),
      complete: () => complete(dialogScope),
      fail: (reason) => {
        setStatus("error");
        setError(reason instanceof Error ? reason.message : String(reason));
      },
    });
  };

  const disabledReason = !nativeDesktop
    ? "Recode creates an immutable dataset version and is available only in the installed Windows app. Browser preview cannot write dataset versions."
    : !projectWritable
      ? "This project is read-only. Save a writable copy before creating a recoded variable."
      : null;
  const controlsDisabled = Boolean(disabledReason) || status === "saving";
  const fieldPrefix = `nd-recode-${dataset.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  const errorId = `${fieldPrefix}-error`;
  const invalid = (path: string) => invalidPath === path;

  return <form className="nd-recode-dialog" aria-busy={status === "saving"} onSubmit={(event) => { event.preventDefault(); void submit(); }}>
    <div className="nd-recode-content">
      {disabledReason ? <p className="nd-recode-disabled" role="status"><Info size={14} aria-hidden="true" />{disabledReason}</p> : null}
      <div className="nd-recode-fields">
        <label htmlFor={`${fieldPrefix}-source`}>Source indicator<input id={`${fieldPrefix}-source`} type="text" value={sourceColumn} readOnly disabled={controlsDisabled} aria-readonly="true" aria-invalid={invalid("sourceColumn")} aria-describedby={invalid("sourceColumn") ? errorId : undefined} /></label>
        <label htmlFor={`${fieldPrefix}-target`}>New indicator
          <input id={`${fieldPrefix}-target`} autoFocus={!controlsDisabled} type="text" disabled={controlsDisabled} value={draft.targetColumn} aria-invalid={invalid("targetColumn")} aria-describedby={invalid("targetColumn") ? errorId : undefined} onChange={(event) => update("targetColumn", event.target.value)} />
        </label>
        <label htmlFor={`${fieldPrefix}-label`}>Label
          <input id={`${fieldPrefix}-label`} type="text" disabled={controlsDisabled} value={draft.targetLabel} onChange={(event) => update("targetLabel", event.target.value)} />
        </label>
        <div className="nd-recode-type-fields">
          <label htmlFor={`${fieldPrefix}-type`}>Type
            <select id={`${fieldPrefix}-type`} aria-label="Type" disabled={controlsDisabled} value={draft.targetType} aria-invalid={invalid("targetType")} aria-describedby={invalid("targetType") ? errorId : undefined} onChange={(event) => setTargetType(event.target.value as NativeRecodeTargetType)}>
              <option value="numeric">Numeric</option><option value="text">Text</option><option value="boolean">Boolean</option>
            </select>
          </label>
          <label htmlFor={`${fieldPrefix}-scale`}>Scale
            <select id={`${fieldPrefix}-scale`} aria-label="Scale" disabled={controlsDisabled} value={draft.targetScale} aria-invalid={invalid("targetScale")} aria-describedby={invalid("targetScale") ? errorId : undefined} onChange={(event) => update("targetScale", event.target.value as NativeRecodeDraft["targetScale"])}>
              <option value="continuous">Continuous</option><option value="ordinal">Ordinal</option><option value="nominal">Nominal</option><option value="binary">Binary</option><option value="identifier">Identifier</option>
            </select>
          </label>
        </div>
      </div>

      <fieldset className="nd-recode-mappings">
        <legend>Value mappings</legend>
        <div className="nd-recode-mapping-head" aria-hidden="true"><span>Source value</span><span>New value</span><span /></div>
        {draft.mappings.map((mapping, index) => <div className="nd-recode-mapping-row" key={index}>
          <label className="nd-sr-only" htmlFor={`${fieldPrefix}-source-${index}`}>Mapping {index + 1} source value</label>
          <input id={`${fieldPrefix}-source-${index}`} type="text" disabled={controlsDisabled} value={mapping.source} aria-invalid={invalid(`mappings.${index}.source`) || (invalidPath === "mappings" && index === 0)} aria-describedby={invalid(`mappings.${index}.source`) || (invalidPath === "mappings" && index === 0) ? errorId : undefined} onChange={(event) => updateMapping(index, "source", event.target.value)} />
          <label className="nd-sr-only" htmlFor={`${fieldPrefix}-target-${index}`}>Mapping {index + 1} new value; leave blank for missing</label>
          <input id={`${fieldPrefix}-target-${index}`} type="text" disabled={controlsDisabled} value={mapping.target} placeholder="Missing if blank" aria-invalid={invalid(`mappings.${index}.target`)} aria-describedby={invalid(`mappings.${index}.target`) ? errorId : undefined} onChange={(event) => updateMapping(index, "target", event.target.value)} />
          <button type="button" aria-label={`Remove mapping ${index + 1}`} title="Remove mapping" disabled={controlsDisabled || draft.mappings.length === 1} onClick={() => update("mappings", draft.mappings.filter((_, mappingIndex) => mappingIndex !== index))}><Trash2 size={13} /></button>
        </div>)}
        <button className="nd-recode-add" type="button" disabled={controlsDisabled} onClick={() => update("mappings", [...draft.mappings, { source: "", target: "" }])}><Plus size={13} />Add mapping</button>
        <small>Leave a new value blank to set that mapped source value to missing.</small>
      </fieldset>

      <label className="nd-recode-unmapped" htmlFor={`${fieldPrefix}-unmapped`}>All other values
        <select id={`${fieldPrefix}-unmapped`} aria-label="All other values" disabled={controlsDisabled} value={draft.unmapped} aria-invalid={invalid("unmapped")} aria-describedby={invalid("unmapped") ? errorId : undefined} onChange={(event) => update("unmapped", event.target.value as NativeRecodeDraft["unmapped"])}>
          <option value="keep_original">Keep original value</option>
          <option value="set_missing">Set to missing</option>
          <option value="error">Stop with an error</option>
        </select>
      </label>
      {error ? <p id={errorId} className="nd-form-error" role="alert">{error}</p> : null}
    </div>
    <footer>
      <button type="button" disabled={status === "saving"} onClick={close}>Cancel</button>
      <button className="primary" type="submit" disabled={Boolean(disabledReason) || status === "saving"}>{status === "saving" ? "Creating version..." : "Create Recode"}</button>
    </footer>
  </form>;
}
