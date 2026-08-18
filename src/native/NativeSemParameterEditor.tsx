import { useEffect, useId, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import type { SemModelV4FactorIdentificationAuthoring, SemModelV4ParameterAuthoringSpecification } from "../types";
import type { SemParameterV4, SemVariableV4 } from "../domain/semModelV4";
import { validateNativeSemParameterSpecificationV4 } from "../domain/semModelV4ParameterAuthoring";

export interface NativeSemParameterEditPolicy {
  managedMessage?: string | null;
  freeOnly?: boolean;
}

export interface NativeSemParameterEditorProps {
  parameter: Exclude<SemParameterV4, { kind: "derived" }>;
  canRestore: boolean;
  policy?: NativeSemParameterEditPolicy;
  onApply: (specification: SemModelV4ParameterAuthoringSpecification) => void;
  onRestore: () => void;
  onClose: () => void;
}

export function NativeSemParameterEditor({
  parameter,
  canRestore,
  policy = {},
  onApply,
  onRestore,
  onClose,
}: NativeSemParameterEditorProps) {
  const id = useId();
  const form = useRef<HTMLFormElement>(null);
  const [kind, setKind] = useState<"free" | "fixed">(parameter.kind);
  const [start, setStart] = useState(parameter.kind === "free" ? numberText(parameter.start) : "");
  const [lower, setLower] = useState(parameter.kind === "free" ? numberText(parameter.lower) : "");
  const [upper, setUpper] = useState(parameter.kind === "free" ? numberText(parameter.upper) : "");
  const [equalityLabel, setEqualityLabel] = useState(parameter.kind === "free" ? parameter.equality_label ?? "" : "");
  const [fixedValue, setFixedValue] = useState(parameter.kind === "fixed" ? String(parameter.value) : "0");
  const [errors, setErrors] = useState<readonly string[]>([]);

  useEffect(() => {
    const container = form.current;
    (container?.querySelector<HTMLElement>("select:not(:disabled), input:not(:disabled)")
      ?? container?.querySelector<HTMLElement>("button:not(:disabled)"))?.focus();
  }, [parameter.id]);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (policy.managedMessage) return;
    let specification: SemModelV4ParameterAuthoringSpecification;
    const effectiveKind = policy.freeOnly ? "free" : kind;
    if (effectiveKind === "fixed") {
      const parsed = requiredNumber(fixedValue, "Enter a finite fixed value.");
      if (!parsed.ok) {
        setErrors([parsed.error]);
        return;
      }
      specification = { kind: "fixed", value: parsed.value };
    } else {
      const parsedStart = optionalNumber(start);
      const parsedLower = optionalNumber(lower);
      const parsedUpper = optionalNumber(upper);
      if (!parsedStart.ok || !parsedLower.ok || !parsedUpper.ok) {
        const problem = !parsedStart.ok ? parsedStart : !parsedLower.ok ? parsedLower : parsedUpper;
        setErrors([problem.ok ? "Enter finite numeric values or leave the field blank." : problem.error]);
        return;
      }
      specification = {
        kind: "free",
        start: parsedStart.value,
        lower: parsedLower.value,
        upper: parsedUpper.value,
        equality_label: equalityLabel.trim() || null,
      };
    }
    const diagnostics = validateNativeSemParameterSpecificationV4(parameter.id, specification);
    if (diagnostics.length) {
      setErrors(diagnostics.map((diagnostic) => `${diagnostic.message} ${diagnostic.corrective_action}`));
      return;
    }
    onApply(specification);
  };

  return <form
    ref={form}
    id="nd-sem-parameter-editor"
    className="nd-sem-editor"
    aria-labelledby={`${id}-title`}
    onSubmit={submit}
    onKeyDown={(event) => closeOnEscape(event, onClose)}
  >
    <div className="nd-sem-editor-heading">
      <div><h4 id={`${id}-title`}>Edit {parameter.label}</h4><code>{parameter.id}</code></div>
      <button type="button" onClick={onClose} aria-label="Close parameter editor">Close</button>
    </div>
    {policy.managedMessage ? <p className="nd-sem-editor-note" role="note">{policy.managedMessage}</p> : <>
      <label htmlFor={`${id}-kind`}>Specification</label>
      <select
        id={`${id}-kind`}
        value={policy.freeOnly ? "free" : kind}
        onChange={(event) => setKind(event.target.value as "free" | "fixed")}
        disabled={policy.freeOnly}
      >
        <option value="free">Free</option>
        {!policy.freeOnly ? <option value="fixed">Fixed</option> : null}
      </select>
      {kind === "fixed" && !policy.freeOnly ? <label htmlFor={`${id}-fixed`}>Fixed value
        <input id={`${id}-fixed`} type="number" step="any" value={fixedValue} onChange={(event) => setFixedValue(event.target.value)} />
      </label> : <div className="nd-sem-editor-grid">
        <label htmlFor={`${id}-start`}>Start value<input id={`${id}-start`} type="number" step="any" value={start} onChange={(event) => setStart(event.target.value)} placeholder="Automatic" /></label>
        <label htmlFor={`${id}-lower`}>Lower bound<input id={`${id}-lower`} type="number" step="any" value={lower} onChange={(event) => setLower(event.target.value)} placeholder="None" /></label>
        <label htmlFor={`${id}-upper`}>Upper bound<input id={`${id}-upper`} type="number" step="any" value={upper} onChange={(event) => setUpper(event.target.value)} placeholder="None" /></label>
        <label htmlFor={`${id}-equality`}>Equality label<input id={`${id}-equality`} value={equalityLabel} maxLength={64} onChange={(event) => setEqualityLabel(event.target.value)} placeholder="Optional" /></label>
      </div>}
    </>}
    {errors.length ? <ul className="nd-sem-editor-errors" role="alert">{errors.map((error) => <li key={error}>{error}</li>)}</ul> : null}
    <div className="nd-sem-editor-actions">
      {!policy.managedMessage ? <button type="submit">Apply</button> : null}
      <button type="button" disabled={!canRestore} onClick={onRestore}>Restore generated setting</button>
      <button type="button" onClick={onClose}>Cancel</button>
    </div>
  </form>;
}

export type NativeSemVariableAuthoringDraft =
  | {
    kind: "common_factor";
    identification: SemModelV4FactorIdentificationAuthoring;
    estimate_latent_mean: boolean;
  }
  | {
    kind: "observed";
    estimate_intercept: boolean;
    estimate_thresholds: boolean;
  };

export interface NativeSemVariableEditorProps {
  variable: Extract<SemVariableV4, { kind: "common_factor" | "observed" }>;
  indicators: readonly string[];
  hasLatentMean: boolean;
  hasIntercept: boolean;
  hasThresholds: boolean;
  onApply: (draft: NativeSemVariableAuthoringDraft) => void;
  onClose: () => void;
}

export function NativeSemVariableEditor({
  variable,
  indicators,
  hasLatentMean,
  hasIntercept,
  hasThresholds,
  onApply,
  onClose,
}: NativeSemVariableEditorProps) {
  const id = useId();
  const form = useRef<HTMLFormElement>(null);
  const initialIdentification = variable.kind === "common_factor" ? variable.identification : null;
  const [identificationKind, setIdentificationKind] = useState(initialIdentification?.kind ?? "marker_loading");
  const [marker, setMarker] = useState(initialIdentification?.kind === "marker_loading"
    ? observedSource(initialIdentification.indicator)
    : [...indicators].sort()[0] ?? "");
  const [estimateLatentMean, setEstimateLatentMean] = useState(hasLatentMean);
  const [estimateIntercept, setEstimateIntercept] = useState(hasIntercept);
  const [estimateThresholds, setEstimateThresholds] = useState(hasThresholds);

  useEffect(() => {
    const container = form.current;
    (container?.querySelector<HTMLElement>("select:not(:disabled), input:not(:disabled)")
      ?? container?.querySelector<HTMLElement>("button:not(:disabled)"))?.focus();
  }, [variable.id]);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (variable.kind === "common_factor") {
      const identification: SemModelV4FactorIdentificationAuthoring = identificationKind === "marker_loading"
        ? { kind: "marker_loading", indicator: marker }
        : identificationKind === "fixed_variance" ? { kind: "fixed_variance" } : { kind: "effects_coding" };
      onApply({ kind: "common_factor", identification, estimate_latent_mean: estimateLatentMean });
    } else onApply({ kind: "observed", estimate_intercept: estimateIntercept, estimate_thresholds: estimateThresholds });
  };

  const interceptAvailable = variable.kind === "observed" && !["ordinal", "nominal", "identifier"].includes(variable.scale);
  const thresholdAvailable = variable.kind === "observed" && variable.scale === "ordinal" && variable.categories.length >= 2;

  return <form
    ref={form}
    id="nd-sem-parameter-editor"
    className="nd-sem-editor"
    aria-labelledby={`${id}-title`}
    onSubmit={submit}
    onKeyDown={(event) => closeOnEscape(event, onClose)}
  >
    <div className="nd-sem-editor-heading">
      <div><h4 id={`${id}-title`}>Edit {variable.label}</h4><code>{variable.id}</code></div>
      <button type="button" onClick={onClose} aria-label="Close variable editor">Close</button>
    </div>
    {variable.kind === "common_factor" ? <>
      <label htmlFor={`${id}-identification`}>Factor identification
        <select
          id={`${id}-identification`}
          value={identificationKind}
          onChange={(event) => setIdentificationKind(event.target.value as SemModelV4FactorIdentificationAuthoring["kind"])}
        >
          <option value="marker_loading">Marker loading</option>
          <option value="fixed_variance">Fixed variance</option>
          <option value="effects_coding" disabled={indicators.length < 3}>Effects coding</option>
        </select>
      </label>
      {identificationKind === "marker_loading" ? <label htmlFor={`${id}-marker`}>Marker indicator
        <select id={`${id}-marker`} value={marker} onChange={(event) => setMarker(event.target.value)}>
          {[...indicators].sort().map((indicator) => <option key={indicator} value={indicator}>{indicator}</option>)}
        </select>
      </label> : null}
      <label className="nd-sem-editor-check"><input type="checkbox" checked={estimateLatentMean} onChange={(event) => setEstimateLatentMean(event.target.checked)} />Estimate latent mean</label>
    </> : <>
      <p className="nd-sem-editor-note">Scale: {variable.scale}. Location parameters are stored with this indicator and are not sent to the current estimator.</p>
      <label className="nd-sem-editor-check"><input type="checkbox" checked={estimateIntercept} disabled={!interceptAvailable} onChange={(event) => setEstimateIntercept(event.target.checked)} />Estimate observed intercept</label>
      <label className="nd-sem-editor-check"><input type="checkbox" checked={estimateThresholds} disabled={!thresholdAvailable} onChange={(event) => setEstimateThresholds(event.target.checked)} />Estimate ordinal thresholds</label>
      {!interceptAvailable && !thresholdAvailable ? <p className="nd-sem-editor-note" role="note">No location parameter is available for this scale in the current model contract.</p> : null}
    </>}
    <div className="nd-sem-editor-actions"><button type="submit">Apply</button><button type="button" onClick={onClose}>Cancel</button></div>
  </form>;
}

function optionalNumber(value: string): { ok: true; value: number | null } | { ok: false; error: string } {
  if (!value.trim()) return { ok: true, value: null };
  const parsed = Number(value);
  return Number.isFinite(parsed) ? { ok: true, value: parsed } : { ok: false, error: "Enter finite numeric values or leave the field blank." };
}

function requiredNumber(value: string, error: string): { ok: true; value: number } | { ok: false; error: string } {
  const parsed = Number(value);
  return value.trim() && Number.isFinite(parsed) ? { ok: true, value: parsed } : { ok: false, error };
}

function numberText(value: number | null | undefined): string {
  return value == null ? "" : String(value);
}

function observedSource(id: string): string {
  return id.startsWith("observed:") ? id.slice("observed:".length) : id;
}

function closeOnEscape(event: KeyboardEvent<HTMLFormElement>, onClose: () => void) {
  if (event.key !== "Escape") return;
  event.preventDefault();
  onClose();
}
