import { AlertTriangle, CheckCircle2, Info, Plus, Trash2 } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { canonicalDatasetTransformationJsonV2 } from "../domain/datasetTransformationsV2";
import type {
  DatasetTransformationIssueV2,
  DatasetTransformationPreviewV2,
  DatasetTransformationSpecV2,
} from "../domain/datasetTransformationsV2";
import type { Dataset, DatasetVersionMutation } from "../types";
import {
  NATIVE_DATASET_TRANSFORM_KINDS_V2,
  buildNativeDatasetTransformationSpecV2,
  changeNativeDatasetTransformKindV2,
  defaultNativeDatasetTransformDraftV2,
  nativeDatasetTransformAvailabilityReasonV2,
  nativeDatasetTransformTargetV2,
  nativeDatasetTransformationIssuesFromErrorV2,
  nativeDatasetTransformationScaleLabelV2,
  type NativeDatasetTransformDraftV2,
  type NativeDatasetTransformKindV2,
  type NativeGroupRuleDraftV2,
} from "./nativeDatasetTransform";

interface NativeDatasetTransformDialogProps {
  dataset: Dataset;
  selectedColumn: string;
  nativeDesktop: boolean;
  projectWritable: boolean;
  mutationsLocked: boolean;
  datasetResident: boolean;
  dialogScope: number;
  close: () => void;
  complete: (dialogScope: number) => void;
  onBusyChange: (dialogScope: number, busy: boolean) => void;
  previewTransformation: (spec: DatasetTransformationSpecV2) => Promise<DatasetTransformationPreviewV2>;
  applyTransformation: (spec: DatasetTransformationSpecV2, outputDatasetName: string) => Promise<DatasetVersionMutation>;
}

type TransformDialogStatus = "idle" | "previewing" | "ready" | "committing" | "committed" | "error";

const emptyGroupRule = (index: number): NativeGroupRuleDraftV2 => ({
  kind: "values",
  output: `Group ${index + 1}`,
  label: `Group ${index + 1}`,
  values: "",
  minimum: "",
  maximum: "",
  includeMinimum: true,
  includeMaximum: true,
});

function previewPlanKey(spec: DatasetTransformationSpecV2, outputDatasetName: string): string {
  return `${canonicalDatasetTransformationJsonV2(spec)}\n${outputDatasetName.trim()}`;
}

function displayCell(value: string | number | null): string {
  return value === null ? "Missing" : String(value);
}

function shortHash(value: string): string {
  if (!value) return "Not available";
  return value.length > 24 ? `${value.slice(0, 12)}…${value.slice(-8)}` : value;
}

function TransformIssues({ issues }: { issues: readonly DatasetTransformationIssueV2[] }) {
  if (!issues.length) return null;
  return <section className="nd-transform-issues" role="alert" aria-labelledby="nd-transform-issues-title">
    <strong id="nd-transform-issues-title"><AlertTriangle size={14} aria-hidden="true" />Check this setup</strong>
    <ul>{issues.map((item, index) => <li key={`${item.code}-${item.field}-${item.row_index ?? "setup"}-${index}`}>
      <span>{item.message}</span>
      <small>{item.field}{item.row_index === null ? "" : ` · case ${item.row_index + 1}`}</small>
    </li>)}</ul>
  </section>;
}

export function NativeDatasetTransformDialog({
  dataset,
  selectedColumn,
  nativeDesktop,
  projectWritable,
  mutationsLocked,
  datasetResident,
  dialogScope,
  close,
  complete,
  onBusyChange,
  previewTransformation,
  applyTransformation,
}: NativeDatasetTransformDialogProps) {
  const initialDraft = useMemo(
    () => defaultNativeDatasetTransformDraftV2(dataset, selectedColumn),
    [dataset, selectedColumn],
  );
  const [draft, setDraft] = useState<NativeDatasetTransformDraftV2>(initialDraft);
  const [status, setStatus] = useState<TransformDialogStatus>("idle");
  const [preview, setPreview] = useState<DatasetTransformationPreviewV2 | null>(null);
  const [previewedSpec, setPreviewedSpec] = useState<DatasetTransformationSpecV2 | null>(null);
  const [previewedPlanKey, setPreviewedPlanKey] = useState<string | null>(null);
  const [issues, setIssues] = useState<DatasetTransformationIssueV2[]>([]);
  const [mutation, setMutation] = useState<DatasetVersionMutation | null>(null);
  const mountedRef = useRef(true);
  const requestRef = useRef(0);

  const availabilityReason = nativeDatasetTransformAvailabilityReasonV2({
    dataset,
    nativeDesktop,
    projectWritable,
    mutationsLocked,
    datasetResident,
  });
  const controlsDisabled = Boolean(availabilityReason) || status === "previewing" || status === "committing" || status === "committed";
  const fieldPrefix = `nd-transform-${dataset.id.replace(/[^a-zA-Z0-9_-]/g, "-")}`;
  const scaleLabel = nativeDatasetTransformationScaleLabelV2(draft);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      requestRef.current += 1;
      onBusyChange(dialogScope, false);
    };
  }, [dialogScope, onBusyChange]);

  useEffect(() => {
    if (status === "previewing" || status === "committing" || status === "committed") return;
    setDraft(initialDraft);
    setStatus("idle");
    setPreview(null);
    setPreviewedSpec(null);
    setPreviewedPlanKey(null);
    setIssues([]);
    setMutation(null);
    // A request owns its draft and a completed request keeps its exact lineage visible.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialDraft]);

  const invalidatePreview = () => {
    setStatus("idle");
    setPreview(null);
    setPreviewedSpec(null);
    setPreviewedPlanKey(null);
    setIssues([]);
  };

  const update = <K extends keyof NativeDatasetTransformDraftV2>(key: K, value: NativeDatasetTransformDraftV2[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
    invalidatePreview();
  };

  const changeKind = (kind: NativeDatasetTransformKindV2) => {
    setDraft((current) => changeNativeDatasetTransformKindV2(dataset, current, kind));
    invalidatePreview();
  };

  const changeSource = (sourceColumn: string) => {
    setDraft((current) => {
      const targetColumn = nativeDatasetTransformTargetV2(dataset, sourceColumn, current.kind);
      return {
        ...current,
        sourceColumn,
        targetColumn,
        outputDatasetName: `${dataset.name} - ${targetColumn}`,
      };
    });
    invalidatePreview();
  };

  const updateMapping = (index: number, field: "source" | "target", value: string) => {
    update("recodeMappings", draft.recodeMappings.map((mapping, mappingIndex) => mappingIndex === index ? { ...mapping, [field]: value } : mapping));
  };

  const updateGroupRule = <K extends keyof NativeGroupRuleDraftV2>(index: number, field: K, value: NativeGroupRuleDraftV2[K]) => {
    update("groupRules", draft.groupRules.map((rule, ruleIndex) => ruleIndex === index ? { ...rule, [field]: value } : rule));
  };

  const setRecodeTargetType = (value: NativeDatasetTransformDraftV2["recodeTargetType"]) => {
    setDraft((current) => ({
      ...current,
      recodeTargetType: value,
      recodeTargetScale: value === "boolean" ? "binary" : value === "text" && current.recodeTargetScale === "continuous" ? "nominal" : current.recodeTargetScale,
    }));
    invalidatePreview();
  };

  const runPreview = async () => {
    if (availabilityReason || status === "previewing" || status === "committing" || status === "committed") return;
    const built = buildNativeDatasetTransformationSpecV2(dataset, draft);
    if (!built.spec) {
      setIssues(built.issues);
      setStatus("error");
      return;
    }
    const spec = built.spec;
    const planKey = previewPlanKey(spec, draft.outputDatasetName);
    const request = requestRef.current + 1;
    requestRef.current = request;
    setStatus("previewing");
    setIssues([]);
    setPreview(null);
    setPreviewedSpec(null);
    setPreviewedPlanKey(null);
    onBusyChange(dialogScope, true);
    try {
      const result = await previewTransformation(spec);
      if (!mountedRef.current || requestRef.current !== request) return;
      setPreview(result);
      setIssues([...result.issues]);
      if (result.issues.length) {
        setStatus("error");
      } else {
        setPreviewedSpec(spec);
        setPreviewedPlanKey(planKey);
        setStatus("ready");
      }
    } catch (reason) {
      if (!mountedRef.current || requestRef.current !== request) return;
      setIssues(nativeDatasetTransformationIssuesFromErrorV2(reason));
      setStatus("error");
    } finally {
      if (mountedRef.current && requestRef.current === request) onBusyChange(dialogScope, false);
    }
  };

  const commit = async () => {
    if (availabilityReason || status !== "ready" || !previewedSpec || !previewedPlanKey) return;
    const current = buildNativeDatasetTransformationSpecV2(dataset, draft);
    if (!current.spec || previewPlanKey(current.spec, draft.outputDatasetName) !== previewedPlanKey) {
      setIssues(current.issues.length ? current.issues : [{
        code: "preview.outdated",
        field: "spec",
        message: "Preview this setup again before creating the dataset version.",
        row_index: null,
      }]);
      setStatus("error");
      setPreviewedSpec(null);
      setPreviewedPlanKey(null);
      return;
    }
    const request = requestRef.current + 1;
    requestRef.current = request;
    setStatus("committing");
    setIssues([]);
    onBusyChange(dialogScope, true);
    try {
      const result = await applyTransformation(previewedSpec, draft.outputDatasetName.trim());
      if (!mountedRef.current || requestRef.current !== request) return;
      setMutation(result);
      setStatus("committed");
    } catch (reason) {
      if (!mountedRef.current || requestRef.current !== request) return;
      setIssues(nativeDatasetTransformationIssuesFromErrorV2(reason));
      setStatus("error");
    } finally {
      if (mountedRef.current && requestRef.current === request) onBusyChange(dialogScope, false);
    }
  };

  const toggleAggregateColumn = (column: string, checked: boolean) => {
    update("aggregateSourceColumns", checked
      ? [...draft.aggregateSourceColumns, column]
      : draft.aggregateSourceColumns.filter((candidate) => candidate !== column));
  };

  const lineage = mutation?.version.transformation;
  if (status === "committed" && mutation && lineage) return <div className="nd-transform-dialog nd-transform-complete" role="status">
    <section className="nd-transform-success" aria-labelledby="nd-transform-complete-title">
      <CheckCircle2 size={28} aria-hidden="true" />
      <div><h3 id="nd-transform-complete-title">Derived dataset created</h3><p>{mutation.version.summary}. The source version remains unchanged.</p></div>
    </section>
    <dl className="nd-transform-lineage" aria-label="Transformation lineage">
      <div><dt>Source dataset</dt><dd>{lineage.source_dataset_id}</dd></div>
      <div><dt>New dataset</dt><dd>{mutation.dataset.name}</dd></div>
      <div><dt>Derived variable</dt><dd>{lineage.output_columns.join(", ")}</dd></div>
      <div><dt>Cases</dt><dd>{lineage.source_row_count.toLocaleString()}</dd></div>
      <div><dt>Missing outputs</dt><dd>{lineage.output_missing_count.toLocaleString()}</dd></div>
      <div><dt>Created</dt><dd>{new Date(lineage.created_at).toLocaleString()}</dd></div>
      <div><dt>Source fingerprint</dt><dd><code title={lineage.source_dataset_fingerprint}>{shortHash(lineage.source_dataset_fingerprint)}</code></dd></div>
      <div><dt>Output fingerprint</dt><dd><code title={lineage.output_dataset_fingerprint}>{shortHash(lineage.output_dataset_fingerprint)}</code></dd></div>
      <div><dt>Transformation fingerprint</dt><dd><code title={lineage.spec_sha256}>{shortHash(lineage.spec_sha256)}</code></dd></div>
      <div><dt>Operation</dt><dd><code>{lineage.operation_id}</code></dd></div>
    </dl>
    <footer><button className="primary" type="button" autoFocus onClick={() => complete(dialogScope)}>Done</button></footer>
  </div>;

  return <form className="nd-transform-dialog" aria-busy={status === "previewing" || status === "committing"} onSubmit={(event) => { event.preventDefault(); void runPreview(); }}>
    <div className="nd-transform-content">
      {availabilityReason ? <p className="nd-transform-disabled" role="status"><Info size={14} aria-hidden="true" />{availabilityReason}</p> : null}

      <fieldset className="nd-transform-kinds" disabled={controlsDisabled}>
        <legend>Transformation</legend>
        {NATIVE_DATASET_TRANSFORM_KINDS_V2.map((item) => <label key={item.id} className={draft.kind === item.id ? "selected" : ""}>
          <input type="radio" name="transform-kind" value={item.id} checked={draft.kind === item.id} onChange={() => changeKind(item.id)} />
          <span><strong>{item.label}</strong><small>{item.description}</small></span>
        </label>)}
      </fieldset>

      <div className="nd-transform-layout">
        <section className="nd-transform-setup" aria-labelledby="nd-transform-setup-title">
          <header><h3 id="nd-transform-setup-title">Setup</h3><span>{NATIVE_DATASET_TRANSFORM_KINDS_V2.find((item) => item.id === draft.kind)?.label}</span></header>
          {draft.kind !== "row_aggregate" ? <label htmlFor={`${fieldPrefix}-source`}>Source variable
            <select id={`${fieldPrefix}-source`} autoFocus={!controlsDisabled} disabled={controlsDisabled} value={draft.sourceColumn} onChange={(event) => changeSource(event.target.value)}>
              {dataset.columns.map((column) => <option key={column} value={column}>{column}</option>)}
            </select>
          </label> : null}

          {draft.kind === "reverse_scale" ? <div className="nd-transform-inline-fields">
            <label htmlFor={`${fieldPrefix}-minimum`}>Scale minimum<input id={`${fieldPrefix}-minimum`} type="number" step="any" disabled={controlsDisabled} value={draft.scaleMinimum} onChange={(event) => update("scaleMinimum", event.target.value)} /></label>
            <label htmlFor={`${fieldPrefix}-maximum`}>Scale maximum<input id={`${fieldPrefix}-maximum`} type="number" step="any" disabled={controlsDisabled} value={draft.scaleMaximum} onChange={(event) => update("scaleMaximum", event.target.value)} /></label>
          </div> : null}

          {draft.kind === "recode" ? <>
            <div className="nd-transform-inline-fields">
              <label htmlFor={`${fieldPrefix}-recode-type`}>Output type<select id={`${fieldPrefix}-recode-type`} disabled={controlsDisabled} value={draft.recodeTargetType} onChange={(event) => setRecodeTargetType(event.target.value as NativeDatasetTransformDraftV2["recodeTargetType"])}><option value="numeric">Numeric</option><option value="text">Text</option><option value="boolean">Boolean</option></select></label>
              <label htmlFor={`${fieldPrefix}-recode-scale`}>Output scale<select id={`${fieldPrefix}-recode-scale`} disabled={controlsDisabled} value={draft.recodeTargetScale} onChange={(event) => update("recodeTargetScale", event.target.value as NativeDatasetTransformDraftV2["recodeTargetScale"])}><option value="continuous">Continuous</option><option value="ordinal">Ordinal</option><option value="nominal">Nominal</option><option value="binary">Binary</option><option value="identifier">Identifier</option></select></label>
            </div>
            <fieldset className="nd-transform-repeat-list"><legend>Value mappings</legend>
              {draft.recodeMappings.map((mapping, index) => <div className="nd-transform-mapping-row" key={index}>
                <label htmlFor={`${fieldPrefix}-map-source-${index}`}>Source {index + 1}<input id={`${fieldPrefix}-map-source-${index}`} disabled={controlsDisabled} value={mapping.source} onChange={(event) => updateMapping(index, "source", event.target.value)} /></label>
                <label htmlFor={`${fieldPrefix}-map-target-${index}`}>New value<input id={`${fieldPrefix}-map-target-${index}`} disabled={controlsDisabled} value={mapping.target} placeholder="Missing if blank" onChange={(event) => updateMapping(index, "target", event.target.value)} /></label>
                <button type="button" aria-label={`Remove mapping ${index + 1}`} disabled={controlsDisabled || draft.recodeMappings.length === 1} onClick={() => update("recodeMappings", draft.recodeMappings.filter((_, candidate) => candidate !== index))}><Trash2 size={13} /></button>
              </div>)}
              <button className="nd-transform-add" type="button" disabled={controlsDisabled} onClick={() => update("recodeMappings", [...draft.recodeMappings, { source: "", target: "" }])}><Plus size={13} />Add mapping</button>
            </fieldset>
            <label htmlFor={`${fieldPrefix}-recode-unmapped`}>All other values<select id={`${fieldPrefix}-recode-unmapped`} disabled={controlsDisabled} value={draft.recodeUnmapped} onChange={(event) => update("recodeUnmapped", event.target.value as NativeDatasetTransformDraftV2["recodeUnmapped"])}><option value="keep">Keep original value</option><option value="missing">Set to missing</option><option value="error">Stop with an error</option></select></label>
          </> : null}

          {draft.kind === "arithmetic" ? <>
            <div className="nd-transform-inline-fields">
              <label htmlFor={`${fieldPrefix}-operator`}>Operation<select id={`${fieldPrefix}-operator`} disabled={controlsDisabled} value={draft.arithmeticOperator} onChange={(event) => update("arithmeticOperator", event.target.value as NativeDatasetTransformDraftV2["arithmeticOperator"])}><option value="add">Add</option><option value="subtract">Subtract</option><option value="multiply">Multiply</option><option value="divide">Divide</option></select></label>
              <label htmlFor={`${fieldPrefix}-right-kind`}>Use<select id={`${fieldPrefix}-right-kind`} disabled={controlsDisabled} value={draft.arithmeticRightKind} onChange={(event) => update("arithmeticRightKind", event.target.value as NativeDatasetTransformDraftV2["arithmeticRightKind"])}><option value="column">Another variable</option><option value="constant">A constant</option></select></label>
            </div>
            {draft.arithmeticRightKind === "column" ? <label htmlFor={`${fieldPrefix}-right-column`}>Right-hand variable<select id={`${fieldPrefix}-right-column`} disabled={controlsDisabled} value={draft.arithmeticRightColumn} onChange={(event) => update("arithmeticRightColumn", event.target.value)}>{dataset.columns.map((column) => <option key={column} value={column}>{column}</option>)}</select></label> : <label htmlFor={`${fieldPrefix}-constant`}>Constant<input id={`${fieldPrefix}-constant`} type="number" step="any" disabled={controlsDisabled} value={draft.arithmeticConstant} onChange={(event) => update("arithmeticConstant", event.target.value)} /></label>}
          </> : null}

          {draft.kind === "row_aggregate" ? <>
            <fieldset className="nd-transform-column-list"><legend>Variables to combine</legend>{dataset.columns.map((column) => <label key={column}><input type="checkbox" disabled={controlsDisabled} checked={draft.aggregateSourceColumns.includes(column)} onChange={(event) => toggleAggregateColumn(column, event.target.checked)} />{column}</label>)}</fieldset>
            <div className="nd-transform-inline-fields">
              <label htmlFor={`${fieldPrefix}-aggregate-operation`}>Calculation<select id={`${fieldPrefix}-aggregate-operation`} disabled={controlsDisabled} value={draft.aggregateOperation} onChange={(event) => update("aggregateOperation", event.target.value as NativeDatasetTransformDraftV2["aggregateOperation"])}><option value="sum">Sum</option><option value="mean">Mean</option></select></label>
              <label htmlFor={`${fieldPrefix}-aggregate-missing`}>When a value is missing<select id={`${fieldPrefix}-aggregate-missing`} disabled={controlsDisabled} value={draft.aggregateMissingPolicy} onChange={(event) => update("aggregateMissingPolicy", event.target.value as NativeDatasetTransformDraftV2["aggregateMissingPolicy"])}><option value="propagate">Set output to missing</option><option value="available">Use available values</option></select></label>
            </div>
            <label htmlFor={`${fieldPrefix}-aggregate-minimum`}>Minimum complete variables<input id={`${fieldPrefix}-aggregate-minimum`} type="number" min="1" step="1" disabled={controlsDisabled} value={draft.aggregateMinimumNonMissing} onChange={(event) => update("aggregateMinimumNonMissing", event.target.value)} /></label>
          </> : null}

          {draft.kind === "dummy" ? <div className="nd-transform-inline-fields">
            <label htmlFor={`${fieldPrefix}-dummy-value`}>Value coded as 1<input id={`${fieldPrefix}-dummy-value`} disabled={controlsDisabled} value={draft.dummyMatchValue} onChange={(event) => update("dummyMatchValue", event.target.value)} /></label>
            <label htmlFor={`${fieldPrefix}-dummy-missing`}>Missing source values<select id={`${fieldPrefix}-dummy-missing`} disabled={controlsDisabled} value={draft.dummyMissingPolicy} onChange={(event) => update("dummyMissingPolicy", event.target.value as NativeDatasetTransformDraftV2["dummyMissingPolicy"])}><option value="missing">Keep missing</option><option value="zero">Code as 0</option></select></label>
          </div> : null}

          {draft.kind === "group" ? <>
            <div className="nd-transform-inline-fields">
              <label htmlFor={`${fieldPrefix}-group-output-type`}>Group code type<select id={`${fieldPrefix}-group-output-type`} disabled={controlsDisabled} value={draft.groupOutputType} onChange={(event) => update("groupOutputType", event.target.value as NativeDatasetTransformDraftV2["groupOutputType"])}><option value="text">Text</option><option value="numeric">Numeric</option></select></label>
              <label htmlFor={`${fieldPrefix}-group-unmatched`}>Unmatched values<select id={`${fieldPrefix}-group-unmatched`} disabled={controlsDisabled} value={draft.groupUnmatched} onChange={(event) => update("groupUnmatched", event.target.value as NativeDatasetTransformDraftV2["groupUnmatched"])}><option value="missing">Set to missing</option><option value="error">Stop with an error</option></select></label>
            </div>
            <fieldset className="nd-transform-repeat-list"><legend>Group rules</legend>
              {draft.groupRules.map((rule, index) => <article className="nd-transform-group-rule" key={index}>
                <header><strong>Group {index + 1}</strong><button type="button" aria-label={`Remove group rule ${index + 1}`} disabled={controlsDisabled || draft.groupRules.length === 1} onClick={() => update("groupRules", draft.groupRules.filter((_, candidate) => candidate !== index))}><Trash2 size={13} /></button></header>
                <div className="nd-transform-inline-fields">
                  <label htmlFor={`${fieldPrefix}-group-kind-${index}`}>Rule type<select id={`${fieldPrefix}-group-kind-${index}`} disabled={controlsDisabled} value={rule.kind} onChange={(event) => updateGroupRule(index, "kind", event.target.value as NativeGroupRuleDraftV2["kind"])}><option value="values">Exact values</option><option value="numeric_range">Numeric range</option></select></label>
                  <label htmlFor={`${fieldPrefix}-group-output-${index}`}>Output code<input id={`${fieldPrefix}-group-output-${index}`} disabled={controlsDisabled} value={rule.output} onChange={(event) => updateGroupRule(index, "output", event.target.value)} /></label>
                  <label htmlFor={`${fieldPrefix}-group-label-${index}`}>Display label<input id={`${fieldPrefix}-group-label-${index}`} disabled={controlsDisabled} value={rule.label} onChange={(event) => updateGroupRule(index, "label", event.target.value)} /></label>
                </div>
                {rule.kind === "values" ? <label htmlFor={`${fieldPrefix}-group-values-${index}`}>Source values, one per line<textarea id={`${fieldPrefix}-group-values-${index}`} rows={3} disabled={controlsDisabled} value={rule.values} onChange={(event) => updateGroupRule(index, "values", event.target.value)} /></label> : <>
                  <div className="nd-transform-inline-fields"><label htmlFor={`${fieldPrefix}-group-min-${index}`}>Minimum<input id={`${fieldPrefix}-group-min-${index}`} type="number" step="any" disabled={controlsDisabled} value={rule.minimum} placeholder="No lower limit" onChange={(event) => updateGroupRule(index, "minimum", event.target.value)} /></label><label htmlFor={`${fieldPrefix}-group-max-${index}`}>Maximum<input id={`${fieldPrefix}-group-max-${index}`} type="number" step="any" disabled={controlsDisabled} value={rule.maximum} placeholder="No upper limit" onChange={(event) => updateGroupRule(index, "maximum", event.target.value)} /></label></div>
                  <div className="nd-transform-check-row" role="group" aria-label={`Range boundary rules for group ${index + 1}`}><label><input type="checkbox" disabled={controlsDisabled} checked={rule.includeMinimum} onChange={(event) => updateGroupRule(index, "includeMinimum", event.target.checked)} />Include minimum</label><label><input type="checkbox" disabled={controlsDisabled} checked={rule.includeMaximum} onChange={(event) => updateGroupRule(index, "includeMaximum", event.target.checked)} />Include maximum</label></div>
                </>}
              </article>)}
              <button className="nd-transform-add" type="button" disabled={controlsDisabled} onClick={() => update("groupRules", [...draft.groupRules, emptyGroupRule(draft.groupRules.length)])}><Plus size={13} />Add group</button>
            </fieldset>
          </> : null}

          <fieldset className="nd-transform-output"><legend>Output</legend>
            <label htmlFor={`${fieldPrefix}-target`}>New variable name<input id={`${fieldPrefix}-target`} disabled={controlsDisabled} value={draft.targetColumn} onChange={(event) => update("targetColumn", event.target.value)} /></label>
            <label htmlFor={`${fieldPrefix}-label`}>Variable label<input id={`${fieldPrefix}-label`} disabled={controlsDisabled} value={draft.targetLabel} onChange={(event) => update("targetLabel", event.target.value)} /></label>
            <label htmlFor={`${fieldPrefix}-dataset-name`}>New dataset version name<input id={`${fieldPrefix}-dataset-name`} disabled={controlsDisabled} value={draft.outputDatasetName} onChange={(event) => update("outputDatasetName", event.target.value)} /></label>
            <dl><div><dt>Output scale</dt><dd>{scaleLabel}</dd></div><div><dt>Source version</dt><dd>{dataset.name}</dd></div></dl>
          </fieldset>
        </section>

        <section className="nd-transform-preview" aria-labelledby="nd-transform-preview-title">
          <header><h3 id="nd-transform-preview-title">Preview</h3><span>{preview ? `${preview.inspected_rows} of ${preview.total_rows.toLocaleString()} cases` : "Not run"}</span></header>
          <p className="nd-transform-lineage-note"><Info size={13} aria-hidden="true" />Preview does not change the dataset. Creating the version records the source fingerprint, exact transformation, inputs, output, and missing-value count.</p>
          <TransformIssues issues={issues} />
          {status === "previewing" ? <p className="nd-transform-wait" role="status">Checking the complete dataset…</p> : null}
          {!preview && status !== "previewing" && !issues.length ? <div className="nd-transform-preview-empty"><strong>Preview required</strong><span>Run Preview to check values and missing outputs before creating a new dataset version.</span></div> : null}
          {preview?.rows.length ? <div className="nd-transform-preview-table" role="region" tabIndex={0} aria-label="Transformation preview rows"><table><caption>First {preview.rows.length} inspected cases</caption><thead><tr><th>Case</th>{preview.input_columns.map((column) => <th key={column}>{column}</th>)}<th>{preview.target_column}</th></tr></thead><tbody>{preview.rows.map((row) => <tr key={row.row_index}><th scope="row">{row.row_index + 1}</th>{preview.input_columns.map((column) => <td key={column}>{displayCell(row.inputs[column] ?? null)}</td>)}<td>{displayCell(row.output)}</td></tr>)}</tbody></table></div> : null}
          {preview ? <dl className="nd-transform-preview-summary"><div><dt>Output variable</dt><dd>{preview.target_column}</dd></div><div><dt>Output scale</dt><dd>{scaleLabel}</dd></div><div><dt>Missing outputs</dt><dd>{preview.output_missing_count.toLocaleString()}</dd></div><div><dt>Input variables</dt><dd>{preview.input_columns.join(", ")}</dd></div></dl> : null}
          {status === "ready" ? <p className="nd-transform-ready" role="status"><CheckCircle2 size={14} aria-hidden="true" />Preview passed. Create Version will use this exact setup.</p> : null}
        </section>
      </div>
    </div>
    <footer>
      <button type="button" disabled={status === "previewing" || status === "committing"} onClick={close}>Cancel</button>
      <span className="spacer" />
      <button type="submit" disabled={controlsDisabled}>{status === "previewing" ? "Previewing…" : "Preview"}</button>
      <button className="primary" type="button" disabled={status !== "ready" || Boolean(availabilityReason)} onClick={() => { void commit(); }}>{status === "committing" ? "Creating version…" : "Create Version"}</button>
    </footer>
  </form>;
}
