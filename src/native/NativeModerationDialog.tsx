import { useMemo, useState } from "react";
import type { Edge, Node } from "@xyflow/react";
import type { AddTwoStageInteractionBlockReason, AddTwoStageInteractionResult } from "../store";
import type { ConstructData } from "../types";
import type { ModeratingEffectTargetV1 } from "../domain/moderationDiagramProjectionV1";
import {
  nativeModeratingEffect,
  nativeModeratingEffects,
  nativeModerationCreationError,
  nativeModerationRelationships,
  nativeModeratorCandidates,
} from "./nativeModeration";

export type NativeModerationDialogRequest =
  | { kind: "create"; target?: ModeratingEffectTargetV1; moderatorId?: string }
  | { kind: "edit"; interactionTermId: string };

export interface NativeModerationDialogSubmissionV1 {
  mode: "create" | "edit";
  target: ModeratingEffectTargetV1;
  relationshipId: string;
  predictorId: string;
  moderatorId: string;
  outcomeId: string;
  interactionTermId?: string;
  order: 2 | 3;
}

export type NativeModerationDialogCommitResult =
  | AddTwoStageInteractionResult
  | { status: "updated"; interactionTermId: string }
  | { status: "blocked"; reason: AddTwoStageInteractionBlockReason | string };

export interface NativeModerationDialogProps {
  nodes: readonly Node<ConstructData>[];
  edges: readonly Edge[];
  /** Backward-compatible focal relation seam used by the current desktop command. */
  selectedEdgeId?: string | null;
  request?: NativeModerationDialogRequest;
  create?: (predictor: string, moderator: string, outcome: string) => AddTwoStageInteractionResult;
  /** Unified 2.53 create/edit seam; preferred when supplied. */
  commit?: (submission: NativeModerationDialogSubmissionV1) => NativeModerationDialogCommitResult;
  close: () => void;
}

function creationMessage(reason: AddTwoStageInteractionBlockReason | string): string {
  switch (reason) {
    case "constructs_not_distinct":
    case "duplicate_interaction":
    case "construct_missing":
    case "unsupported_construct":
    case "focal_path_missing":
    case "control_paths_unsupported":
      return nativeModerationCreationError(reason);
    default:
      return String(reason);
  }
}

export default function NativeModerationDialog({
  nodes,
  edges,
  selectedEdgeId,
  request,
  create,
  commit,
  close,
}: NativeModerationDialogProps) {
  const relationships = useMemo(() => nativeModerationRelationships(nodes, edges), [edges, nodes]);
  const resolvedRequest: NativeModerationDialogRequest = request ?? {
    kind: "create",
    target: selectedEdgeId ? { kind: "focal_relation", relationId: selectedEdgeId } : undefined,
  };
  const editing = resolvedRequest.kind === "edit"
    ? nativeModeratingEffect(nodes, edges, resolvedRequest.interactionTermId)
    : undefined;
  const parent = resolvedRequest.kind === "create" && resolvedRequest.target?.kind === "parent_interaction"
    ? nativeModeratingEffect(nodes, edges, resolvedRequest.target.interactionTermId)
    : undefined;
  const fixedRelationshipId = editing?.focalRelationId
    ?? parent?.focalRelationId
    ?? (resolvedRequest.kind === "create" && resolvedRequest.target?.kind === "focal_relation"
      ? resolvedRequest.target.relationId
      : selectedEdgeId)
    ?? null;
  // A two-way effect can be retargeted to another eligible focal path while
  // preserving its stable term/output identity. Three-way effects remain
  // bound to their resident parent interaction.
  const fixedRelationship = Boolean(parent || editing?.order === 3
    || resolvedRequest.kind === "create" && resolvedRequest.target?.kind === "focal_relation");
  const initialRelationship = fixedRelationshipId
    ? relationships.find((relationship) => relationship.edgeId === fixedRelationshipId)
    : relationships[0];
  const [relationshipId, setRelationshipId] = useState(initialRelationship?.edgeId ?? "");
  const relationship = relationships.find((candidate) => candidate.edgeId === relationshipId) ?? initialRelationship;
  const moderators = useMemo(() => {
    const candidates = nativeModeratorCandidates(
      nodes,
      relationship,
      editing?.interactionTermId,
      Boolean(parent || editing?.order === 3),
    );
    const alreadyUsed = new Set(parent
      ? [parent.predictor, ...parent.moderatorIds, parent.outcome]
      : editing
        ? [editing.predictor, ...editing.moderatorIds.slice(0, -1), editing.outcome]
        : []);
    return candidates.filter((candidate) => !alreadyUsed.has(candidate.id));
  }, [editing?.interactionTermId, editing?.order, nodes, parent, relationship]);
  const requestedModeratorId = resolvedRequest.kind === "create" ? resolvedRequest.moderatorId : undefined;
  const initialModeratorId = editing?.moderatorIds.at(-1) ?? requestedModeratorId ?? moderators[0]?.id ?? "";
  const [moderatorId, setModeratorId] = useState(initialModeratorId);
  const [creationError, setCreationError] = useState<string | null>(null);
  const selectedModerator = moderators.some((candidate) => candidate.id === moderatorId)
    ? moderatorId
    : moderators[0]?.id ?? "";
  const order: 2 | 3 = editing?.order ?? (parent ? 3 : 2);
  const target: ModeratingEffectTargetV1 = resolvedRequest.kind === "create" && resolvedRequest.target
    ? resolvedRequest.target
    : editing?.parentInteractionTermId
      ? { kind: "parent_interaction", interactionTermId: editing.parentInteractionTermId }
      : { kind: "focal_relation", relationId: relationship?.edgeId ?? fixedRelationshipId ?? "" };
  const hasOtherThreeWayEffect = nativeModeratingEffects(nodes, edges)
    .some((effect) => effect.order === 3 && effect.interactionTermId !== editing?.interactionTermId);
  const globalBlocker = resolvedRequest.kind === "edit" && !editing
    ? "This moderating effect is no longer available. Close the dialog and select it again."
    : parent && hasOtherThreeWayEffect
      ? "This model already has its supported three-way moderating effect. Remove that effect before adding another."
      : fixedRelationshipId && !relationship
        ? "The selected item is not an eligible measured structural relationship. Select a standard predictor-to-outcome path."
        : edges.some((edge) => (edge.data as { role?: string } | undefined)?.role === "control")
          ? "Remove or convert control paths before creating a moderating effect; this workflow does not accept control paths."
          : relationships.length === 0
            ? "Create and select a structural path before adding a moderating effect."
            : !commit && !create
              ? "Moderating-effect editing is not connected in this workspace."
              : null;
  const blocker = globalBlocker ?? (moderators.length === 0
    ? editing
      ? "No eligible replacement moderator is available for this relationship."
      : "Add another measured construct to use as the moderator."
    : null);

  return <form
    className="nd-dialog-form nd-moderation-dialog"
    onSubmit={(event) => {
      event.preventDefault();
      if (!relationship || !selectedModerator || blocker) return;
      const submission: NativeModerationDialogSubmissionV1 = {
        mode: editing ? "edit" : "create",
        target,
        relationshipId: relationship.edgeId,
        predictorId: relationship.predictor,
        moderatorId: selectedModerator,
        outcomeId: relationship.outcome,
        interactionTermId: editing?.interactionTermId,
        order,
      };
      const result = commit
        ? commit(submission)
        : create!(relationship.predictor, selectedModerator, relationship.outcome);
      if (result.status === "created" || result.status === "updated") close();
      else setCreationError(creationMessage(result.reason));
    }}
  >
    <label htmlFor="nd-moderation-relationship">Relationship
      <select
        id="nd-moderation-relationship"
        autoFocus
        value={relationship?.edgeId ?? ""}
        disabled={Boolean(globalBlocker) || fixedRelationship}
        onChange={(event) => {
          const nextId = event.target.value;
          setRelationshipId(nextId);
          const nextRelationship = relationships.find((candidate) => candidate.edgeId === nextId);
          setModeratorId(nativeModeratorCandidates(
            nodes,
            nextRelationship,
            editing?.interactionTermId,
            Boolean(parent || editing?.order === 3),
          )[0]?.id ?? "");
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
      <div><dt>Effect</dt><dd>{order === 3 ? "Three-way moderation" : "Two-way moderation"}</dd></div>
      <div><dt>Relationship</dt><dd>{relationship.label}</dd></div>
    </dl> : null}
    <details className="nd-moderation-advanced">
      <summary>Advanced</summary>
      <dl className="nd-property-list">
        <div><dt>Construction</dt><dd>Two-stage</dd></div>
        <div><dt>Hierarchy</dt><dd>Strong</dd></div>
      </dl>
    </details>
    {blocker || creationError ? <div className="nd-form-error" role="alert">{blocker ?? creationError}</div> : null}
    <footer>
      <button type="button" onClick={close}>Cancel</button>
      <button className="primary" type="submit" disabled={Boolean(blocker) || !relationship || !selectedModerator}>
        {editing ? "Save changes" : order === 3 ? "Add three-way effect" : "Add moderating effect"}
      </button>
    </footer>
  </form>;
}
