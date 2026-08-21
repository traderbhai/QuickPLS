import { useState } from "react";
import { DiagnosticBundlePanel } from "../components/SettingsWorkspace";
import {
  methodDetailsForRequirementsV2,
  methodDetailsForSettingsV2,
  type CapabilityMethodDetailsV2,
} from "../domain/methodDetailsV2";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { AnalysisRun, AnalysisUiSettings } from "../types";
import { nativeCapabilityRequirementsForRunV2 } from "./nativeCanonicalResultDocumentV2";
import { completedResultRuns } from "./nativeResults";

export interface NativeUtilityDialogProps {
  kind: "trust" | "settings";
  close: () => void;
  /** When opened from Results, bind Method Details to this immutable completed run. */
  run?: AnalysisRun | null;
  /** Explicit context seam for callers rendering Method Details outside the live workspace. */
  methodDetailsSettings?: Readonly<AnalysisUiSettings>;
  experimentalLabsEnabledOverride?: boolean;
}

export default function NativeUtilityDialog({
  kind,
  close,
  run,
  methodDetailsSettings,
  experimentalLabsEnabledOverride,
}: NativeUtilityDialogProps) {
  if (kind === "trust") return <TrustDialog
    run={run ?? null}
    settingsOverride={methodDetailsSettings}
    experimentalLabsEnabledOverride={experimentalLabsEnabledOverride}
  />;
  return <PreferencesDialog close={close} />;
}

function TrustDialog({
  run,
  settingsOverride,
  experimentalLabsEnabledOverride,
}: {
  run: AnalysisRun | null;
  settingsOverride?: Readonly<AnalysisUiSettings>;
  experimentalLabsEnabledOverride?: boolean;
}) {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const runs = useWorkspace((state) => state.runs);
  const workspaceSettings = useWorkspace((state) => state.analysisSettings);
  const workspaceExperimentalLabsEnabled = useWorkspace((state) => state.uiPreferences.experimentalLabsEnabled);
  const settings = settingsOverride ?? workspaceSettings;
  const experimentalLabsEnabled = experimentalLabsEnabledOverride ?? workspaceExperimentalLabsEnabled;
  const resolution = run
    ? methodDetailsForRequirementsV2(
        run.method,
        nativeCapabilityRequirementsForRunV2(run),
        experimentalLabsEnabled,
      )
    : methodDetailsForSettingsV2(settings, experimentalLabsEnabled);
  const datasetFingerprint = run?.provenance?.dataset_fingerprint ?? dataset.fingerprint;

  return <div className="nd-utility-dialog" data-method-details-context={run ? "completed-run" : "workspace-settings"} data-method-guidance-home="true">
    <p>Purpose, requirements, settings, outputs, assumptions, limitations, interpretation, and references for the selected method.</p>
    {resolution.issues.length > 0 ? <div className="nd-inline-error" role="alert">
      <strong>Method information is unavailable.</strong>
      <ul>{resolution.issues.map((issue) => <li key={issue}>{issue}</li>)}</ul>
    </div> : null}
    <div className="nd-method-details-list">
      {resolution.items.map((item) => <MethodDetailsCard key={`${item.capability_cell.capability_id}::${item.capability_cell.cell_id}`} item={item} />)}
    </div>
    <section className="nd-method-run-context" aria-labelledby="nd-method-run-context-title">
      <h3 id="nd-method-run-context-title">{run ? "Selected completed run" : "Current workspace"}</h3>
      <dl className="nd-property-list">
        <div><dt>Dataset fingerprint</dt><dd>{datasetFingerprint || "Not available"}</dd></div>
        {run ? <div><dt>Run</dt><dd>{run.name}</dd></div> : <div><dt>Model</dt><dd>{nodes.length} constructs</dd></div>}
        {run ? <div><dt>Completed</dt><dd>{run.createdAt}</dd></div> : <div><dt>Completed runs</dt><dd>{completedResultRuns(runs).length}</dd></div>}
        <div><dt>Runtime</dt><dd>{isNativeDesktop() ? "Offline desktop" : "Web preview"}</dd></div>
      </dl>
    </section>
  </div>;
}

function MethodDetailsCard({ item }: { item: CapabilityMethodDetailsV2 }) {
  const status = item.availability.customer_label ?? "Unavailable";
  const details = item.details;
  return <article className="nd-method-details-card" data-method-details-v2="true">
    <header>
      <div><span>{item.family}</span><h3>{item.option_name}</h3></div>
      <span className={`nd-method-availability is-${item.availability.visibility}`}>{status}</span>
    </header>
    {item.availability.visibility === "hidden" ? <p className="nd-method-availability-message">{item.availability_message}</p> : null}
    <div className="nd-method-details-grid">
      <section><h4>What this method answers</h4><p>{details.what_it_answers}</p></section>
      <section><h4>When to use it</h4><p>{details.when_to_use}</p></section>
      <section><h4>Required model and data</h4><p>{details.required_model_and_data}</p></section>
      <section><h4>Main settings and defaults</h4><p>{details.settings_and_defaults}</p></section>
      <section><h4>Outputs</h4><p>{details.outputs}</p></section>
      <section><h4>Assumptions and limitations</h4><p>{details.assumptions_and_cautions}</p></section>
      <section><h4>Interpretation guidance</h4><p>{details.interpretation_guidance}</p></section>
      <section><h4>Advanced technical details</h4><p>{details.advanced_technical_details}</p></section>
      <section className="nd-method-references"><h4>Method references</h4><ol>{details.method_references.map((reference) => <li key={reference}><a href={reference} target="_blank" rel="noreferrer">{reference}</a></li>)}</ol></section>
    </div>
  </article>;
}

function PreferencesDialog({ close }: { close: () => void }) {
  const preferences = useWorkspace((state) => state.uiPreferences);
  const setPreferences = useWorkspace((state) => state.setUiPreferences);
  const [draft, setDraft] = useState(preferences);

  return <div className="nd-utility-dialog" data-live-preferences-dialog="true">
    <form className="nd-preferences" onSubmit={(event) => { event.preventDefault(); setPreferences(draft); close(); }}>
      <label>Interface density<select value={draft.density} onChange={(event) => setDraft((current) => ({ ...current, density: event.target.value as typeof current.density }))}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></label>
      <label>Table density<select value={draft.tableDensity} onChange={(event) => setDraft((current) => ({ ...current, tableDensity: event.target.value as typeof current.tableDensity }))}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></label>
      <label>Default precision<input type="number" min={2} max={6} value={draft.defaultPrecision} onChange={(event) => setDraft((current) => ({ ...current, defaultPrecision: Number(event.target.value) }))} /></label>
      <label className="checkbox-row">Show generated interaction terms<input type="checkbox" checked={draft.showGeneratedInteractionTerms} onChange={(event) => setDraft((current) => ({ ...current, showGeneratedInteractionTerms: event.target.checked }))} /></label>
      <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">OK</button></footer>
    </form>
    <DiagnosticBundlePanel />
  </div>;
}
