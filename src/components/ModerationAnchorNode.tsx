import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import type { ModerationAnchorProjectionV1 } from "../domain/moderationDiagramProjectionV1";

export function ModerationAnchorNode({ data, selected }: NodeProps<Node<ModerationAnchorProjectionV1>>) {
  const editable = data.editable === true;
  return <div
    className={`moderation-anchor${selected ? " selected" : ""}`}
    role={editable ? "button" : "img"}
    tabIndex={editable ? 0 : undefined}
    aria-label={data.label}
    aria-keyshortcuts={editable ? "Enter Delete" : undefined}
    title={editable ? `${data.label}. Enter edits; Delete removes the moderating effect.` : data.label}
    onFocus={() => {
      if (editable) window.dispatchEvent(new CustomEvent("quickpls:moderation-focus", { detail: { interactionTermId: data.interactionTermId } }));
    }}
  >
    <Handle className="moderation-anchor-handle" id="target-left" type="target" position={Position.Left} />
    <Handle className="moderation-anchor-handle" id="target-right" type="target" position={Position.Right} />
    <Handle className="moderation-anchor-handle" id="target-top" type="target" position={Position.Top} />
    <Handle className="moderation-anchor-handle" id="target-bottom" type="target" position={Position.Bottom} />
    <span aria-hidden="true">{data.order === 3 ? "3×" : "×"}</span>
  </div>;
}
