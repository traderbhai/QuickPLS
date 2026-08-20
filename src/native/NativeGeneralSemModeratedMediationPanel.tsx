import { useMemo, useState } from "react";
import type { GeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import {
  buildGeneralSemModeratedMediationSelectionV1,
  type GeneralSemModeratedMediationSelectionReadyV1,
} from "../domain/generalSemModeratedMediationAuthoringV1";
import type { SemModelV4 } from "../domain/semModelV4";

export interface NativeGeneralSemModeratedMediationPanelProps {
  readonly connected: boolean;
  readonly model: SemModelV4;
  readonly config: GeneralSemConfigV1;
  readonly revisionPending?: boolean;
  readonly revisionBlocked?: boolean;
  readonly revisionBlockedReason?: string | undefined;
  readonly revisionStatusMessage?: string | null;
  readonly revisionFailure?: {
    readonly code: string;
    readonly message: string;
    readonly correctiveAction: string;
  } | null;
  readonly onSaveAsRevision?: (
    selection: GeneralSemModeratedMediationSelectionReadyV1,
  ) => void | Promise<void>;
}

function pathLabel(selection: GeneralSemModeratedMediationSelectionReadyV1["selectedPath"]): string {
  return `${selection.xLabel} → ${selection.mediatorLabel} → ${selection.yLabel}`;
}

export function NativeGeneralSemModeratedMediationPanel({
  connected,
  model,
  config,
  revisionPending = false,
  revisionBlocked = false,
  revisionBlockedReason,
  revisionStatusMessage = null,
  revisionFailure = null,
  onSaveAsRevision,
}: NativeGeneralSemModeratedMediationPanelProps) {
  const [selectedPathId, setSelectedPathId] = useState<string | null>(null);
  const selection = useMemo(() => buildGeneralSemModeratedMediationSelectionV1({
    model,
    config,
    selectedPathId,
  }), [config, model, selectedPathId]);

  if (!connected) {
    return <section hidden aria-hidden="true" data-testid="moderated-mediation-unavailable">
      Two-way moderated-mediation authoring is unavailable until its model-and-Recipe revision is connected.
    </section>;
  }

  const selected = selection.selectedPath;
  return <section className="nd-cbsem-v4-card" aria-labelledby="nd-general-sem-moderated-mediation-heading">
    <h3 id="nd-general-sem-moderated-mediation-heading">Two-way moderated mediation</h3>
    <p>Select one exact two-relation path. QuickPLS preserves the authored diagram and saves the choice only as a new versioned model + Recipe project.</p>

    {selection.candidates.length > 1 ? <label htmlFor="nd-general-sem-moderated-mediation-path">
      Conditional-process path
      <select
        id="nd-general-sem-moderated-mediation-path"
        value={selection.selectedPathId ?? ""}
        disabled={revisionPending || revisionBlocked}
        onChange={(event) => setSelectedPathId(event.target.value || null)}
      >
        <option value="">Select one path…</option>
        {selection.candidates.map((candidate) => <option key={candidate.pathId} value={candidate.pathId}>
          {candidate.xLabel} → {candidate.mediatorLabel} → {candidate.yLabel}
        </option>)}
      </select>
    </label> : null}

    {selection.autoSelected && selected ? <p role="status">
      QuickPLS auto-selected {selected.xLabel} → {selected.mediatorLabel} → {selected.yLabel} because it is the only eligible path.
    </p> : null}

    {selected ? <dl className="nd-property-list">
      <div><dt>Selected path</dt><dd>{selected.xLabel} → {selected.mediatorLabel} → {selected.yLabel}</dd></div>
      <div><dt>Moderation stage</dt><dd>{selected.moderatedStage === "first_stage" ? "First stage (X × W → M)" : "Second stage (M × W → Y)"}</dd></div>
      <div><dt>Moderator</dt><dd>{selected.moderatorLabel}</dd></div>
      <div><dt>Stable relations</dt><dd><code>{selected.orderedRelationIds.join(" → ")}</code></dd></div>
    </dl> : null}

    <div aria-labelledby="nd-general-sem-moderated-mediation-targets">
      <h4 id="nd-general-sem-moderated-mediation-targets">Exact five-target inventory</h4>
      <ol>
        {selection.targetInventory.map((target) => <li key={target.id}>{target.label}</li>)}
      </ol>
    </div>
    <p className="nd-dialog-note">Moderator probes are locked to standardized W = -1, 0, and +1. Bootstrap is mandatory; arbitrary probes, both-stage moderation, and causal claims remain excluded.</p>

    {selection.status === "blocked" && selection.issues.length > 0 ? <div className="nd-form-error" role="alert">
      {selection.issues.map((item) => <p key={`${item.code}:${item.subject}`}>
        <strong>{item.message}</strong> {item.correctiveAction} <code>{item.code}</code>
      </p>)}
    </div> : null}

    {revisionFailure ? <div className="nd-form-error" role="alert">
      <p><strong>{revisionFailure.message}</strong> {revisionFailure.correctiveAction} <code>{revisionFailure.code}</code></p>
    </div> : null}

    <button
      type="button"
      className="primary"
      disabled={selection.status !== "ready" || revisionPending || revisionBlocked || !onSaveAsRevision}
      title={revisionBlocked ? revisionBlockedReason : undefined}
      onClick={() => {
        if (selection.status === "ready") void onSaveAsRevision?.(selection);
      }}
    >
      {revisionPending ? "Saving revision…" : "Save path as new model + Recipe revision…"}
    </button>
    {revisionBlocked && revisionBlockedReason ? <p className="nd-dialog-note" role="note">{revisionBlockedReason}</p> : null}
    {selection.status === "ready" ? <p role="status">
      Ready to save {pathLabel(selection.selectedPath)} as a new source-preserving revision. The current archive remains unchanged.
    </p> : null}
    {revisionStatusMessage ? <p role="status" aria-live="polite" aria-atomic="true">{revisionStatusMessage}</p> : null}
  </section>;
}

export default NativeGeneralSemModeratedMediationPanel;
