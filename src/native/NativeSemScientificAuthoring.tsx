import type { Edge, Node } from "@xyflow/react";
import { useEffect, useId, useMemo, useState } from "react";
import {
  confirmNativeSemConstructAuthoringV4,
  confirmNativeSemCovarianceAuthoringV4,
  nativeSemCovarianceChoiceSignatureV4,
  projectNativeSemConstructAuthoringV4,
  projectNativeSemCovarianceAuthoringV4,
  validateNativeSemCovarianceChoiceV4,
  type NativeSemCovarianceChoiceV4,
  type NativeSemCovarianceDraftV4,
  type NativeSemScientificAuthoringDiagnosticV4,
} from "../domain/semModelV4ScientificAuthoring";
import type { ConstructData, SemModelV4ConstructAuthoring } from "../types";

export interface NativeSemConstructAuthoringFieldsProps {
  node: Node<ConstructData>;
  onCommit: (specification: SemModelV4ConstructAuthoring) => void;
}

export function NativeSemConstructAuthoringFields({ node, onCommit }: NativeSemConstructAuthoringFieldsProps) {
  const controlId = useId();
  const projection = useMemo(() => projectNativeSemConstructAuthoringV4(node), [node]);
  const [localDiagnostics, setLocalDiagnostics] = useState<readonly NativeSemScientificAuthoringDiagnosticV4[]>([]);
  useEffect(() => setLocalDiagnostics([]), [node.id, projection.choice, projection.marker_indicator]);

  const confirm = (choice: "composite" | "common_factor", markerIndicator: string | null = null) => {
    const result = confirmNativeSemConstructAuthoringV4(node, choice, markerIndicator);
    if (!result.ok) {
      setLocalDiagnostics(result.diagnostics);
      return;
    }
    setLocalDiagnostics([]);
    onCommit(result.node.data.semModelV4!.construct);
  };
  const diagnostics = localDiagnostics.length ? localDiagnostics : projection.diagnostics;
  const markerValue = projection.marker_indicator ?? projection.marker_candidates[0] ?? "";

  return <section className="nd-sem-authoring" aria-labelledby={`${controlId}-heading`}>
    <header><strong id={`${controlId}-heading`}>Scientific representation</strong><span>Experimental</span></header>
    <p id={`${controlId}-help`}>Confirm whether this construct is an explicitly weighted composite or a common factor reflected by its indicators.</p>
    <label htmlFor={`${controlId}-representation`}>Representation</label>
    <select
      id={`${controlId}-representation`}
      value={projection.choice === "composite" || projection.choice === "common_factor" ? projection.choice : "decision_required"}
      aria-describedby={`${controlId}-help ${controlId}-status`}
      onChange={(event) => {
        const choice = event.target.value;
        if (choice === "composite") confirm("composite");
        else if (choice === "common_factor") confirm("common_factor", projection.marker_candidates[0] ?? null);
      }}
    >
      <option value="decision_required" disabled>Choose representation</option>
      <option value="composite">Composite</option>
      <option value="common_factor">Common factor</option>
    </select>
    {projection.choice === "common_factor" ? <>
      <label htmlFor={`${controlId}-marker`}>Marker indicator</label>
      <select
        id={`${controlId}-marker`}
        value={markerValue}
        aria-describedby={`${controlId}-status`}
        disabled={projection.marker_candidates.length === 0}
        onChange={(event) => confirm("common_factor", event.target.value)}
      >
        {projection.marker_candidates.length === 0 ? <option value="">No indicator available</option> : null}
        {projection.marker_candidates.map((indicator) => <option key={indicator} value={indicator}>{indicator}</option>)}
      </select>
    </> : null}
    <AuthoringStatus
      id={`${controlId}-status`}
      diagnostics={diagnostics}
      readyMessage={projection.choice === "composite"
        ? "Composite confirmed. Existing measurement mode determines Mode A or Mode B in the SemModelV4 plan."
        : projection.choice === "common_factor"
          ? `Common factor confirmed${markerValue ? ` with ${markerValue} as marker` : ""}.`
          : "Choose a representation before the scientific parameter table can be complete."}
    />
    {projection.choice === "composite" || projection.choice === "common_factor" ? <>
      <button type="button" className="nd-sem-authoring-clear" onClick={() => onCommit({ kind: "legacy_estimand_unspecified" })}>Clear representation decision</button>
      <p className="nd-sem-authoring-boundary">This choice is serialized with the model. Current calculation recipes stop before executing it; clearing the decision restores legacy recipe eligibility without guessing its meaning.</p>
    </> : null}
  </section>;
}

export interface NativeSemCovarianceAuthoringFieldsProps {
  edge: Edge;
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  onCommit: (edge: Edge) => void;
}

export function NativeSemCovarianceAuthoringFields({ edge, nodes, edges, onCommit }: NativeSemCovarianceAuthoringFieldsProps) {
  const controlId = useId();
  const projection = useMemo(
    () => projectNativeSemCovarianceAuthoringV4(edge, nodes, edges),
    [edge, edges, nodes],
  );
  const projectedDraft = useMemo(() => projection.choice.kind === "invalid"
    ? { kind: "decision_required" } as const
    : projection.choice, [projection.choice]);
  const currentSignature = nativeSemCovarianceChoiceSignatureV4(projection.choice);
  const [draft, setDraft] = useState<NativeSemCovarianceDraftV4>(projectedDraft);
  const [commitDiagnostics, setCommitDiagnostics] = useState<readonly NativeSemScientificAuthoringDiagnosticV4[]>([]);
  useEffect(() => {
    setDraft(projectedDraft);
    setCommitDiagnostics([]);
  }, [edge.id, currentSignature, projectedDraft]);

  const draftDiagnostics = useMemo(
    () => validateNativeSemCovarianceChoiceV4(edge, nodes, edges, draft),
    [draft, edge, edges, nodes],
  );
  const draftSignature = nativeSemCovarianceChoiceSignatureV4(draft);
  const dirty = draftSignature !== currentSignature || projection.diagnostics.length > 0;
  const diagnostics = commitDiagnostics.length ? commitDiagnostics
    : dirty || draft.kind !== "decision_required" ? draftDiagnostics : projection.diagnostics;

  const chooseKind = (kind: NativeSemCovarianceDraftV4["kind"]) => {
    setCommitDiagnostics([]);
    if (kind === "residual_covariance") setDraft({
      kind,
      source_indicator: projection.source_indicator_candidates[0] ?? "",
      target_indicator: projection.target_indicator_candidates[0] ?? "",
    });
    else if (kind === "decision_required") setDraft({ kind });
    else setDraft({ kind });
  };
  const apply = () => {
    if (draft.kind === "decision_required") return;
    const result = confirmNativeSemCovarianceAuthoringV4(edge, nodes, edges, draft as NativeSemCovarianceChoiceV4);
    if (!result.ok) {
      setCommitDiagnostics(result.diagnostics);
      return;
    }
    setCommitDiagnostics([]);
    onCommit(result.edge);
  };
  const sourceLabel = nodes.find((node) => node.id === edge.source)?.data.label || edge.source;
  const targetLabel = nodes.find((node) => node.id === edge.target)?.data.label || edge.target;
  const readyMessage = draft.kind === "model_covariance"
    ? `Model covariance joins ${sourceLabel} and ${targetLabel}.`
    : draft.kind === "residual_covariance"
      ? "Residual/error covariance joins the two selected observed-indicator residuals."
      : draft.kind === "disturbance_covariance"
        ? "Disturbance covariance joins the structural disturbances of two endogenous constructs."
        : draft.kind === "presentation_only"
          ? "Presentation-only arcs stay on the canvas and never become scientific parameters."
          : "Choose how this drawn covariance should be used.";

  return <section className="nd-sem-authoring" aria-labelledby={`${controlId}-heading`}>
    <header><strong id={`${controlId}-heading`}>Relationship use</strong><span>Experimental</span></header>
    <p id={`${controlId}-help`}>Classify this drawn covariance explicitly. The choice controls whether and where it appears in SemModelV4.</p>
    <label htmlFor={`${controlId}-kind`}>Use</label>
    <select
      id={`${controlId}-kind`}
      value={draft.kind}
      aria-describedby={`${controlId}-help ${controlId}-status`}
      onChange={(event) => chooseKind(event.target.value as NativeSemCovarianceDraftV4["kind"])}
    >
      <option value="decision_required" disabled>Choose relationship use</option>
      <option value="model_covariance">Model covariance</option>
      <option value="residual_covariance">Residual/error covariance</option>
      <option value="disturbance_covariance">Disturbance covariance</option>
      <option value="presentation_only">Presentation only</option>
    </select>
    {draft.kind === "residual_covariance" ? <div className="nd-sem-authoring-endpoints" role="group" aria-label="Residual covariance endpoints">
      <label htmlFor={`${controlId}-source-indicator`}>{sourceLabel} indicator</label>
      <select
        id={`${controlId}-source-indicator`}
        value={draft.source_indicator}
        aria-describedby={`${controlId}-status`}
        onChange={(event) => setDraft((current) => current.kind === "residual_covariance"
          ? { ...current, source_indicator: event.target.value }
          : current)}
      >
        {projection.source_indicator_candidates.length === 0 ? <option value="">No indicator available</option> : null}
        {projection.source_indicator_candidates.map((indicator) => <option key={indicator} value={indicator}>{indicator}</option>)}
      </select>
      <label htmlFor={`${controlId}-target-indicator`}>{targetLabel} indicator</label>
      <select
        id={`${controlId}-target-indicator`}
        value={draft.target_indicator}
        aria-describedby={`${controlId}-status`}
        onChange={(event) => setDraft((current) => current.kind === "residual_covariance"
          ? { ...current, target_indicator: event.target.value }
          : current)}
      >
        {projection.target_indicator_candidates.length === 0 ? <option value="">No indicator available</option> : null}
        {projection.target_indicator_candidates.map((indicator) => <option key={indicator} value={indicator}>{indicator}</option>)}
      </select>
    </div> : null}
    {draft.kind === "disturbance_covariance" ? <dl className="nd-sem-authoring-eligibility">
      <div><dt>{sourceLabel}</dt><dd>{projection.source_is_endogenous ? "Endogenous" : "No incoming path"}</dd></div>
      <div><dt>{targetLabel}</dt><dd>{projection.target_is_endogenous ? "Endogenous" : "No incoming path"}</dd></div>
    </dl> : null}
    <AuthoringStatus id={`${controlId}-status`} diagnostics={diagnostics} readyMessage={readyMessage} />
    <button
      type="button"
      className="nd-sem-authoring-apply"
      disabled={draft.kind === "decision_required" || !dirty || draftDiagnostics.length > 0}
      onClick={apply}
    >Apply relationship use</button>
    {draft.kind !== "presentation_only" && draft.kind !== "decision_required" ? <p className="nd-sem-authoring-boundary">Scientific covariance choices are serialized now. Current calculation recipes stop before executing them.</p> : null}
  </section>;
}

function AuthoringStatus({
  id,
  diagnostics,
  readyMessage,
}: {
  id: string;
  diagnostics: readonly NativeSemScientificAuthoringDiagnosticV4[];
  readyMessage: string;
}) {
  return <div id={id} className={`nd-sem-authoring-status ${diagnostics.length ? "attention" : "ready"}`} role="status" aria-live="polite">
    {diagnostics.length ? <ul>{diagnostics.map((item) => <li key={`${item.code}:${item.subject}`}><strong>{item.message}</strong> {item.corrective_action}</li>)}</ul> : <p>{readyMessage}</p>}
  </div>;
}
