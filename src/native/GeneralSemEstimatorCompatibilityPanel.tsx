import { useId, useMemo } from "react";
import type { GeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
  preflightGeneralSemCbsemV1,
  preflightGeneralSemPlsV1,
} from "../domain/generalSemCapabilityPreflightV1";
import type {
  SemCapabilityDecisionV1,
  SemCapabilityDiagnosticV1,
} from "../domain/semCapabilityDecisionV1";
import type { SemModelV4 } from "../domain/semModelV4";

export type GeneralSemEstimatorIdV1 =
  | typeof GENERAL_SEM_PLS_ESTIMATOR_ID_V1
  | typeof GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1;

export interface GeneralSemEstimatorCompatibilityPanelProps {
  model: SemModelV4;
  config: GeneralSemConfigV1;
  onSelectEstimator: (estimatorId: GeneralSemEstimatorIdV1) => void;
  selectedEstimatorId?: GeneralSemEstimatorIdV1 | null;
}

interface EstimatorOption {
  readonly estimatorId: GeneralSemEstimatorIdV1;
  readonly label: string;
  readonly decision: SemCapabilityDecisionV1;
}

export interface GeneralSemEstimatorSelectionButtonProps {
  estimatorId: GeneralSemEstimatorIdV1;
  estimatorLabel: string;
  decision: SemCapabilityDecisionV1;
  selected: boolean;
  descriptionId: string;
  blockedReason: string;
  onSelectEstimator: (estimatorId: GeneralSemEstimatorIdV1) => void;
}

export function isRunnableGeneralSemDecisionV1(decision: SemCapabilityDecisionV1): boolean {
  return decision.status === "supported" || decision.status === "experimental";
}

/** A native button preserves Enter/Space behavior and exposes no blocked callback. */
export function GeneralSemEstimatorSelectionButton({
  estimatorId,
  estimatorLabel,
  decision,
  selected,
  descriptionId,
  blockedReason,
  onSelectEstimator,
}: GeneralSemEstimatorSelectionButtonProps) {
  const runnable = isRunnableGeneralSemDecisionV1(decision);
  const isSelected = runnable && selected;
  return <button
    type="button"
    className={isSelected ? "primary" : undefined}
    disabled={!runnable}
    aria-disabled={!runnable}
    aria-describedby={descriptionId}
    aria-pressed={runnable ? isSelected : undefined}
    title={runnable
      ? isSelected ? `${estimatorLabel} is selected.` : `Select ${estimatorLabel}.`
      : blockedReason}
    data-general-sem-estimator-select={estimatorId}
    onClick={runnable ? () => onSelectEstimator(estimatorId) : undefined}
  >
    {isSelected ? `Selected ${estimatorLabel}` : runnable ? `Select ${estimatorLabel}` : `${estimatorLabel} unavailable`}
  </button>;
}

function severityLabel(severity: SemCapabilityDiagnosticV1["severity"]): string {
  if (severity === "error") return "Blocking issue";
  if (severity === "warning") return "Warning";
  return "Information";
}

function statusClass(decision: SemCapabilityDecisionV1): string {
  if (decision.status === "supported") return "is-supported";
  if (decision.status === "experimental") return "is-experimental";
  return "is-hidden";
}

function firstBlockingReason(decision: SemCapabilityDecisionV1): string {
  return decision.diagnostics.find((diagnostic) => diagnostic.severity === "error")?.message
    ?? decision.summary;
}

function EstimatorCard({
  option,
  selectedEstimatorId,
  idPrefix,
  onSelectEstimator,
}: {
  option: EstimatorOption;
  selectedEstimatorId?: GeneralSemEstimatorIdV1 | null;
  idPrefix: string;
  onSelectEstimator: (estimatorId: GeneralSemEstimatorIdV1) => void;
}) {
  const { decision, estimatorId, label } = option;
  const runnable = isRunnableGeneralSemDecisionV1(decision);
  const headingId = `${idPrefix}-heading`;
  const explanationId = `${idPrefix}-explanation`;
  const blockedReasonId = `${idPrefix}-blocked-reason`;
  const selectionDescriptionId = runnable ? explanationId : blockedReasonId;
  const blockingReason = firstBlockingReason(decision);
  const capabilitySummary = decision.capability_cells
    .map((capability) => `${capability.cell_id} (${capability.capability_version})`)
    .join("; ");

  return <article
    className="nd-method-details-card"
    aria-labelledby={headingId}
    data-general-sem-estimator-card={estimatorId}
    data-compatibility-status={decision.status}
  >
    <header>
      <div>
        <span aria-label={`${label} exact capability cells`}>Exact capability cells: {capabilitySummary}</span>
        <h3 id={headingId}>{label}</h3>
      </div>
      <span
        className={`nd-method-availability ${statusClass(decision)}`}
        aria-label={`Compatibility status: ${decision.status_label}`}
      >
        {decision.status_label}
      </span>
    </header>

    <p className="nd-method-availability-message"><strong>{decision.summary}</strong></p>
    <div className="nd-method-details-grid">
      <section>
        <h4>Compatibility explanation</h4>
        <p id={explanationId}>{decision.explanation}</p>
        {decision.status === "experimental" ? <p className="nd-inline-warning" role="note">
          <strong>Experimental Labs.</strong> This request passes the exact compiler-qualification cells listed above. Selecting it records an estimator preference only; it does not start native execution.
        </p> : null}
      </section>
      <section>
        <h4>Diagnostics and next actions</h4>
        <ul className="nd-cbsem-v4-preflight-list" aria-label={`${label} diagnostics`}>
          {decision.diagnostics.map((diagnostic) => <li
            key={`${diagnostic.code}:${diagnostic.subject ?? ""}`}
            className={diagnostic.severity === "error" ? "blocked" : "ready"}
          >
            <span aria-hidden="true">{diagnostic.severity === "error" ? "×" : "✓"}</span>
            <div>
              <strong>{severityLabel(diagnostic.severity)}: {diagnostic.message}</strong>
              {diagnostic.corrections.length > 0 ? <>
                <small>How to proceed</small>
                <div role="list">{diagnostic.corrections.map((correction) => <p key={correction} role="listitem">{correction}</p>)}</div>
              </> : null}
            </div>
          </li>)}
        </ul>
      </section>
    </div>

    <footer className="nd-cbsem-v4-actions">
      <GeneralSemEstimatorSelectionButton
        estimatorId={estimatorId}
        estimatorLabel={label}
        decision={decision}
        selected={selectedEstimatorId === estimatorId}
        descriptionId={selectionDescriptionId}
        blockedReason={blockingReason}
        onSelectEstimator={onSelectEstimator}
      />
      {!runnable ? <p id={blockedReasonId} className="nd-method-availability-message" role="note">
        <strong>Cannot select:</strong> {blockingReason}
      </p> : null}
    </footer>
  </article>;
}

/**
 * Provides a deterministic client-side compatibility preview for both
 * estimator boundaries. This component does not invoke native preflight or
 * execute a General SEM calculation.
 */
export function GeneralSemEstimatorCompatibilityPanel({
  model,
  config,
  onSelectEstimator,
  selectedEstimatorId = null,
}: GeneralSemEstimatorCompatibilityPanelProps) {
  const instanceId = useId().replaceAll(":", "");
  const headingId = `${instanceId}-general-sem-estimator-heading`;
  const options: readonly EstimatorOption[] = useMemo(() => [
    {
      estimatorId: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
      label: "PLS-SEM General v3",
      decision: preflightGeneralSemPlsV1(model, config),
    },
    {
      estimatorId: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
      label: "CB-SEM General v3",
      decision: preflightGeneralSemCbsemV1(model, config),
    },
  ], [config, model]);
  const selected = options.find((option) => (
    option.estimatorId === selectedEstimatorId
    && isRunnableGeneralSemDecisionV1(option.decision)
  ));

  return <section
    className="nd-utility-dialog"
    aria-labelledby={headingId}
    data-general-sem-estimator-compatibility="v1"
  >
    <header>
      <h2 id={headingId}>Estimator compatibility preview</h2>
      <p>Review which estimator passes the current client-side compiler qualification for the authored General SEM request.</p>
      <p role="note">Compile-qualification preview only: this panel does not run a calculation or invoke native General SEM execution.</p>
    </header>
    <p className="nd-form-status" role="status" aria-live="polite" aria-atomic="true">
      Estimator compatibility preview: {options.map((option) => `${option.label}: ${option.decision.status_label}`).join("; ")}.
      {selected ? ` Selected: ${selected.label}.` : " No compile-qualified estimator selected."}
    </p>
    <div className="nd-method-details-list">
      {options.map((option, index) => <EstimatorCard
        key={option.estimatorId}
        option={option}
        selectedEstimatorId={selectedEstimatorId}
        idPrefix={`${instanceId}-general-sem-estimator-${index + 1}`}
        onSelectEstimator={onSelectEstimator}
      />)}
    </div>
  </section>;
}

export default GeneralSemEstimatorCompatibilityPanel;
