import { useId } from "react";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
} from "../domain/generalSemCapabilityPreflightV1";
import type { GeneralSemEstimatorParameterTableAuthorityV2 } from "../domain/internalRecipeV4GeneralSemWorkspace";
import type {
  SemCapabilityDecisionV1,
  SemCapabilityDiagnosticV1,
} from "../domain/semCapabilityDecisionV1";

export type GeneralSemEstimatorIdV1 =
  | typeof GENERAL_SEM_PLS_ESTIMATOR_ID_V1
  | typeof GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1;

export interface GeneralSemEstimatorCompatibilityPanelProps {
  decisions: {
    readonly pls: SemCapabilityDecisionV1;
    readonly cbsem: SemCapabilityDecisionV1;
  };
  authority: GeneralSemEstimatorParameterTableAuthorityV2;
  onSelectEstimator: (estimatorId: GeneralSemEstimatorIdV1) => void;
  selectedEstimatorId?: GeneralSemEstimatorIdV1 | null;
  /** A marked project executes only its resident RecipeV4 method. */
  selectionLocked?: boolean;
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
  selectionLocked?: boolean;
  onSelectEstimator: (estimatorId: GeneralSemEstimatorIdV1) => void;
}

export function isRunnableGeneralSemDecisionV1(
  decision: SemCapabilityDecisionV1,
  _estimatorId: GeneralSemEstimatorIdV1,
): boolean {
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
  selectionLocked = false,
  onSelectEstimator,
}: GeneralSemEstimatorSelectionButtonProps) {
  const runnable = isRunnableGeneralSemDecisionV1(decision, estimatorId);
  const isSelected = selected && runnable;
  const selectionDisabled = !runnable || selectionLocked;
  return <button
    type="button"
    className={isSelected && runnable ? "primary" : undefined}
    disabled={selectionDisabled}
    aria-disabled={selectionDisabled}
    aria-describedby={descriptionId}
    aria-pressed={runnable ? isSelected : undefined}
    title={runnable
      ? selectionLocked ? `${estimatorLabel} is fixed by the resident RecipeV4.` : isSelected ? `${estimatorLabel} is selected.` : `Select ${estimatorLabel}.`
      : blockedReason}
    data-general-sem-estimator-select={estimatorId}
    onClick={runnable && !selectionLocked ? () => onSelectEstimator(estimatorId) : undefined}
  >
    {isSelected && selectionLocked
      ? runnable ? `Resident ${estimatorLabel}` : `Resident ${estimatorLabel} blocked`
      : isSelected
        ? `Selected ${estimatorLabel}`
        : runnable && !selectionLocked
          ? `Select ${estimatorLabel}`
          : `${estimatorLabel} unavailable`}
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
  selectionLocked,
}: {
  option: EstimatorOption;
  selectedEstimatorId?: GeneralSemEstimatorIdV1 | null;
  idPrefix: string;
  onSelectEstimator: (estimatorId: GeneralSemEstimatorIdV1) => void;
  selectionLocked: boolean;
}) {
  const { decision, estimatorId, label } = option;
  const runnable = isRunnableGeneralSemDecisionV1(decision, estimatorId);
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
        </p> : decision.status === "supported" ? <p className="nd-method-availability-message" role="note">
          <strong>Supported.</strong> This request passes the exact Standard cells listed above. Review the settings, then calculate through the resident RecipeV4.
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
        selectionLocked={selectionLocked}
        onSelectEstimator={onSelectEstimator}
      />
      {!runnable ? <p id={blockedReasonId} className="nd-method-availability-message" role="note">
        <strong>Cannot select:</strong> {blockingReason}
      </p> : null}
      {runnable && selectionLocked ? <p className="nd-method-availability-message" role="note">
        The estimator is fixed by the saved calculation recipe. Choose another method in Calculate to create a source-preserving revision.
      </p> : null}
    </footer>
  </article>;
}

/**
 * Renders only the native decision bound to the resident schema-6 SemModelV4
 * parameter table. This component neither reconstructs a model from canvas
 * nodes/edges. Selection remains preference-only until the workspace verifies
 * the same resident recipe, exact cell, and archive identity at execution.
 */
export function GeneralSemEstimatorCompatibilityPanel({
  decisions,
  authority,
  onSelectEstimator,
  selectedEstimatorId = null,
  selectionLocked = false,
}: GeneralSemEstimatorCompatibilityPanelProps) {
  const instanceId = useId().replaceAll(":", "");
  const headingId = `${instanceId}-general-sem-estimator-heading`;
  const options: readonly EstimatorOption[] = [
    {
      estimatorId: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
      label: "PLS-SEM General v3",
      decision: decisions.pls,
    },
    {
      estimatorId: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
      label: "CB-SEM General v3",
      decision: decisions.cbsem,
    },
  ];
  const selected = options.find((option) => (
    option.estimatorId === selectedEstimatorId
    && isRunnableGeneralSemDecisionV1(option.decision, option.estimatorId)
  ));

  return <section
    className="nd-utility-dialog"
    aria-labelledby={headingId}
    data-general-sem-estimator-compatibility="v1"
  >
    <header>
      <h2 id={headingId}>Estimator compatibility</h2>
      <p>Native preflight from the active resident schema-6 SemModelV4 parameter table.</p>
      <p role="note">Authority: {authority.parameterCount} parameters ({authority.freeParameterCount} free, {authority.fixedParameterCount} fixed, {authority.derivedParameterCount} derived); table SHA-256 {authority.parameterTableSha256}.</p>
      <p role="note">Compatibility inspection only: a blocked or unpublished candidate has no calculation action.</p>
    </header>
    <p className="nd-form-status" role="status" aria-live="polite" aria-atomic="true">
      Native estimator compatibility: {options.map((option) => `${option.label}: ${option.decision.status_label}`).join("; ")}.
      {selected ? ` Selected: ${selected.label}.` : " No compile-qualified estimator selected."}
    </p>
    <div className="nd-method-details-list">
      {options.map((option, index) => <EstimatorCard
        key={option.estimatorId}
        option={option}
        selectedEstimatorId={selectedEstimatorId}
        idPrefix={`${instanceId}-general-sem-estimator-${index + 1}`}
        onSelectEstimator={onSelectEstimator}
        selectionLocked={selectionLocked}
      />)}
    </div>
  </section>;
}

export default GeneralSemEstimatorCompatibilityPanel;
