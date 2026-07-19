import { BaseEdge, EdgeLabelRenderer, getStraightPath, useStore, type EdgeProps } from "@xyflow/react";
import type { PointerEvent as ReactPointerEvent } from "react";
import { useWorkspace } from "../store";

type LabelOffset = { x?: number; y?: number };

export function SemEdge({ id, sourceX, sourceY, targetX, targetY, markerEnd, markerStart, label, selected, data }: EdgeProps) {
  const checkpoint = useWorkspace((state) => state.checkpoint);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setEdgeLabelOffset = useWorkspace((state) => state.setEdgeLabelOffset);
  const zoom = useStore((state) => state.transform[2]);
  const [path, labelX, labelY] = getStraightPath({ sourceX, sourceY, targetX, targetY });
  const offset = (data?.labelOffset ?? {}) as LabelOffset;
  const edgeClassName = String(data?.edgeClassName ?? "");
  const x = labelX + Number(offset.x ?? 0);
  const y = labelY + Number(offset.y ?? 0);
  const text = typeof label === "string" ? label : "";
  const startDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.stopPropagation();
    checkpoint();
    setSelectedEdge(id);
    const start = { pointerX: event.clientX, pointerY: event.clientY, offsetX: Number(offset.x ?? 0), offsetY: Number(offset.y ?? 0) };
    const move = (moveEvent: PointerEvent) => {
      setEdgeLabelOffset(id, {
        x: Math.round(start.offsetX + (moveEvent.clientX - start.pointerX) / zoom),
        y: Math.round(start.offsetY + (moveEvent.clientY - start.pointerY) / zoom),
      });
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up, { once: true });
  };

  return <>
    <BaseEdge id={id} path={path} markerEnd={markerEnd} markerStart={markerStart} className={`${edgeClassName}${selected ? " selected" : ""}`} />
    {text ? <EdgeLabelRenderer>
      <div className={`sem-edge-label${selected ? " selected" : ""}`} role="button" tabIndex={0} aria-label={`Move label for ${text || "selected path"}`} title="Drag to move label" style={{ transform: `translate(-50%, -50%) translate(${x}px, ${y}px)` }} onPointerDown={startDrag}>
        {text}
      </div>
    </EdgeLabelRenderer> : null}
  </>;
}
