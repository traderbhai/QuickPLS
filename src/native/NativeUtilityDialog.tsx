import { useState } from "react";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import { NATIVE_PREDICTION_METHOD_LABEL } from "./nativeCalculationMode";
import { NATIVE_NCA_SCOPE_NOTE } from "./nativeNca";
import { NATIVE_PCA_SCOPE_NOTE } from "./nativePca";
import { NATIVE_GSCA_SCOPE_NOTE } from "./nativeGsca";
import { completedResultRuns } from "./nativeResults";

export interface NativeUtilityDialogProps {
  kind: "trust" | "settings";
  close: () => void;
}

export default function NativeUtilityDialog({ kind, close }: NativeUtilityDialogProps) {
  if (kind === "trust") return <TrustDialog />;
  return <PreferencesDialog close={close} />;
}

function TrustDialog() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const runs = useWorkspace((state) => state.runs);

  return <div className="nd-utility-dialog">
    <p>QuickPLS executes calculations locally and validates supported methods against independent reference fixtures.</p>
    <dl className="nd-property-list">
      <div><dt>PLS-SEM Algorithm</dt><dd>Validated core scope</dd></div>
      <div><dt>Consistent PLS</dt><dd>Validated reflective bounded scope</dd></div>
      <div><dt>Weighted PLS</dt><dd>Validated positive case-weight bounded scope</dd></div>
      <div><dt>GSCA</dt><dd>{NATIVE_GSCA_SCOPE_NOTE}</dd></div>
      <div><dt>CCA composite residual diagnostics</dt><dd>Validated standardized reflective composite path-model scope; descriptive residuals only, without fit thresholds or inference</dd></div>
      <div><dt>Importance-Performance Map Analysis</dt><dd>Validated single-target predecessor map with 0-100 observed-range performance from listwise-standardized scores; no theoretical-range correction</dd></div>
      <div><dt>Bootstrapping</dt><dd>Validated inference add-on</dd></div>
      <div><dt>Structural Path Randomization</dt><dd>Candidate single-model Freedman-Lane fixed-score inference under exchangeable reduced-model residuals, with unadjusted pathwise p values; current calibration covers homoscedastic Gaussian errors only and this is not a group comparison</dd></div>
      <div><dt>MICOM and Two-Group Permutation MGA</dt><dd>Validated bounded v2 scope with explicit ordered groups, 5,000–10,000 usable permutations, path/loading/weight comparisons, and MICOM Steps 1–3</dd></div>
      <div><dt>{NATIVE_PREDICTION_METHOD_LABEL}</dt><dd>Validated bounded indicator-level scope with seeded 10-fold × 10-repeat cross-validation, IA/LM benchmarks, and one-sided 95% CVPAT benchmark assessment; construct scores and the deterministic holdout are supplementary</dd></div>
      <div><dt>Necessary Condition Analysis</dt><dd>{NATIVE_NCA_SCOPE_NOTE}</dd></div>
      <div><dt>Principal Component Analysis</dt><dd>{NATIVE_PCA_SCOPE_NOTE}</dd></div>
      <div><dt>Dataset fingerprint</dt><dd>{dataset.fingerprint || "Not available"}</dd></div>
      <div><dt>Model</dt><dd>{nodes.length} constructs</dd></div>
      <div><dt>Completed runs</dt><dd>{completedResultRuns(runs).length}</dd></div>
      <div><dt>Runtime</dt><dd>{isNativeDesktop() ? "Offline desktop" : "Web preview"}</dd></div>
    </dl>
  </div>;
}

function PreferencesDialog({ close }: { close: () => void }) {
  const preferences = useWorkspace((state) => state.uiPreferences);
  const setPreferences = useWorkspace((state) => state.setUiPreferences);
  const [draft, setDraft] = useState(preferences);

  return <form className="nd-preferences" onSubmit={(event) => { event.preventDefault(); setPreferences(draft); close(); }}>
    <label>Interface density<select value={draft.density} onChange={(event) => setDraft((current) => ({ ...current, density: event.target.value as typeof current.density }))}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></label>
    <label>Table density<select value={draft.tableDensity} onChange={(event) => setDraft((current) => ({ ...current, tableDensity: event.target.value as typeof current.tableDensity }))}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></label>
    <label>Default precision<input type="number" min={2} max={6} value={draft.defaultPrecision} onChange={(event) => setDraft((current) => ({ ...current, defaultPrecision: Number(event.target.value) }))} /></label>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">OK</button></footer>
  </form>;
}
