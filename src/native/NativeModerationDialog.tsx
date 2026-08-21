import { useMemo, useState } from "react";
import type { Edge, Node } from "@xyflow/react";
import type { AddTwoStageInteractionResult } from "../store";
import type { ConstructData } from "../types";
import {
  nativeModerationCreationError,
  nativeModerationRelationships,
  nativeModeratorCandidates,
} from "./nativeModeration";

export interface NativeModerationDialogProps {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  selectedEdgeId?: string | null;
  create: (predictor: string, moderator: string, outcome: string) => AddTwoStageInteractionResult;
  close: () => void;
}

export default function NativeModerationDialog({
  nodes,
  edges,
  selectedEdgeId,
  create,
  close,
}: NativeModerationDialogProps) {
  const relationships = useMemo(() => nativeModerationRelationships(nodes, edges), [edges, nodes]);
  const interactionCount = nodes.filter((node) => node.data.semantic === "interaction").length;
  const invokedForSelectedRelationship = Boolean(selectedEdgeId);
  const initialRelationship = invokedForSelectedRelationship
    ? relationships.find((relationship) => relationship.edgeId === selectedEdgeId)
    : relationships[0];
  const [relationshipId, setRelationshipId] = useState(initialRelationship?.edgeId ?? "");
  const relationship = relationships.find((candidate) => candidate.edgeId === relationshipId)
    ?? (invokedForSelectedRelationship ? undefined : relationships[0]);
  const moderators = useMemo(() => nativeModeratorCandidates(nodes, relationship), [nodes, relationship]);
  const [moderatorId, setModeratorId] = useState(moderators[0]?.id ?? "");
  const [creationError, setCreationError] = useState<string | null>(null);
  const selectedModerator = moderators.some((candidate) => candidate.id === moderatorId)
    ? moderatorId
    : moderators[0]?.id ?? "";
  const globalBlocker = invokedForSelectedRelationship && !relationship
      ? "The selected item is not an eligible measured structural relationship. Select a standard predictor-to-outcome path."
      : edges.some((edge) => (edge.data as { role?: string } | undefined)?.role === "control")
        ? "Remove or convert control paths before creating a moderating effect; this workflow does not accept control paths."
      : relationships.length === 0
      ? "Create and select a structural path before adding a moderating effect."
      : null;
  const blocker = globalBlocker ?? (moderators.length === 0
    ? interactionCount > 0
      ? "Every eligible moderator for this relationship already has a two-stage interaction. Choose another relationship or remove an existing moderating effect."
      : "Add another measured construct to use as the moderator."
    : null);

  return <form
    className="nd-dialog-form nd-moderation-dialog"
    onSubmit={(event) => {
      event.preventDefault();
      if (!relationship || !selectedModerator || blocker) return;
      const result = create(relationship.predictor, selectedModerator, relationship.outcome);
      if (result.status === "created") close();
      else setCreationError(nativeModerationCreationError(result.reason));
    }}
  >
    <p className="nd-dialog-intro">Create a two-stage product-score interaction for an existing structural relationship.</p>
    <label htmlFor="nd-moderation-relationship">Relationship
      <select
        id="nd-moderation-relationship"
        autoFocus
        value={relationship?.edgeId ?? ""}
        disabled={Boolean(globalBlocker)}
        onChange={(event) => {
          const nextId = event.target.value;
          setRelationshipId(nextId);
          const nextRelationship = relationships.find((candidate) => candidate.edgeId === nextId);
          setModeratorId(nativeModeratorCandidates(nodes, nextRelationship)[0]?.id ?? "");
          setCreationError(null);
        }}
      >
        {relationships.map((candidate) => <option key={candidate.edgeId} value={candidate.edgeId}>{candidate.label}</option>)}
      </select>
    </label>
    <label htmlFor="nd-moderation-moderator">Moderator
      <select
        id="nd-moderation-moderator"
        value={selectedModerator}
        disabled={Boolean(globalBlocker) || moderators.length === 0}
        onChange={(event) => {
          setModeratorId(event.target.value);
          setCreationError(null);
        }}
      >
        {moderators.map((candidate) => <option key={candidate.id} value={candidate.id}>{candidate.label}</option>)}
      </select>
    </label>
    {relationship && selectedModerator && !blocker ? <dl className="nd-property-list nd-moderation-summary">
      <div><dt>Predictor</dt><dd>{relationship.predictorLabel}</dd></div>
      <div><dt>Outcome</dt><dd>{relationship.outcomeLabel}</dd></div>
      <div><dt>Method</dt><dd>Two-stage product score</dd></div>
    </dl> : null}
    <p className="nd-dialog-note">QuickPLS creates the interaction term and adds the moderator’s main-effect path to the outcome when it is missing.</p>
    {interactionCount > 0 ? <p className="nd-dialog-note" role="status" aria-live="polite">
      This model already contains {interactionCount} moderating effect{interactionCount === 1 ? "" : "s"}. You can author another distinct two-stage interaction. Authoring does not mean every estimator can calculate it; Calculation readiness is checked separately and may still block models with multiple interactions.
    </p> : null}
    {blocker || creationError ? <div className="nd-form-error" role="alert">{blocker ?? creationError}</div> : null}
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit" disabled={Boolean(blocker) || !relationship || !selectedModerator}>Create moderating effect</button></footer>
  </form>;
}
