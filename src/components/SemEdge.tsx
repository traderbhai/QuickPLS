import { BaseEdge, EdgeLabelRenderer, getBezierPath, getSmoothStepPath, getStraightPath, useStore, type EdgeProps } from "@xyflow/react";
import type { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent } from "react";
import { useWorkspace } from "../store";

type LabelOffset = { x?: number; y?: number };
type VisualPoint = { x: number; y: number };

function polylinePath(
  source: VisualPoint,
  bends: readonly VisualPoint[],
  target: VisualPoint,
): [string, number, number] {
  const points = [source, ...bends, target];
  const lengths = points.slice(1).map((point, index) => Math.hypot(
    point.x - points[index]!.x,
    point.y - points[index]!.y,
  ));
  const halfway = lengths.reduce((sum, length) => sum + length, 0) / 2;
  let consumed = 0;
  let label = { x: (source.x + target.x) / 2, y: (source.y + target.y) / 2 };
  for (let index = 0; index < lengths.length; index += 1) {
    const length = lengths[index]!;
    if (consumed + length >= halfway && length > 0) {
      const start = points[index]!;
      const end = points[index + 1]!;
      const fraction = (halfway - consumed) / length;
      label = {
        x: start.x + (end.x - start.x) * fraction,
        y: start.y + (end.y - start.y) * fraction,
      };
      break;
    }
    consumed += length;
  }
  return [
    points.map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`).join(" "),
    label.x,
    label.y,
  ];
}

export function SemEdge({ id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, markerEnd, markerStart, label, selected, data, style, interactionWidth }: EdgeProps) {
  const checkpoint = useWorkspace((state) => state.checkpoint);
  const nudgeEdgeLabel = useWorkspace((state) => state.nudgeEdgeLabel);
  const resetEdgeLabel = useWorkspace((state) => state.resetEdgeLabel);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setEdgeLabelOffset = useWorkspace((state) => state.setEdgeLabelOffset);
  const zoom = useStore((state) => state.transform[2]);
  const routing = String(data?.routing ?? "straight");
  const bendPoints = Array.isArray(data?.bendPoints)
    ? data.bendPoints.flatMap((candidate): VisualPoint[] => {
      if (!candidate || typeof candidate !== "object") return [];
      const x = Number((candidate as VisualPoint).x);
      const y = Number((candidate as VisualPoint).y);
      return Number.isFinite(x) && Number.isFinite(y) ? [{ x, y }] : [];
    })
    : [];
  const [path, labelX, labelY] = routing === "polyline" && bendPoints.length
    ? polylinePath({ x: sourceX, y: sourceY }, bendPoints, { x: targetX, y: targetY })
    : routing === "smoothstep"
    ? getSmoothStepPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, borderRadius: 8 })
    : routing === "default"
      ? getBezierPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition })
      : getStraightPath({ sourceX, sourceY, targetX, targetY });
  const offset = (data?.labelOffset ?? {}) as LabelOffset;
  const edgeClassName = String(data?.edgeClassName ?? "");
  const visualOnly = data?.visualOnly === true;
  const x = labelX + Number(offset.x ?? 0);
  const y = labelY + Number(offset.y ?? 0);
  const text = typeof label === "string" ? label : "";
  const isGenericPathLabel = text.trim().toLowerCase() === "path";
  const shouldShowLabel = Boolean(text && (!isGenericPathLabel || selected));
  const startDrag = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (visualOnly) return;
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
  const handleKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (visualOnly) return;
    const step = event.shiftKey ? 12 : 4;
    if (event.key === "ArrowUp") {
      event.preventDefault();
      nudgeEdgeLabel(id, { x: 0, y: -step });
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      nudgeEdgeLabel(id, { x: 0, y: step });
    } else if (event.key === "ArrowLeft") {
      event.preventDefault();
      nudgeEdgeLabel(id, { x: -step, y: 0 });
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      nudgeEdgeLabel(id, { x: step, y: 0 });
    } else if (event.key === "Home") {
      event.preventDefault();
      resetEdgeLabel(id);
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      setSelectedEdge(id);
      removeSelection();
    }
  };

  return <>
    <BaseEdge id={id} path={path} markerEnd={markerEnd} markerStart={markerStart} style={style} interactionWidth={interactionWidth} className={`${edgeClassName}${selected ? " selected" : ""}`} />
    {shouldShowLabel ? <EdgeLabelRenderer>
      <div className={`sem-edge-label${isGenericPathLabel ? " generic-path-label" : ""}${selected ? " selected" : ""}`} role="button" tabIndex={0} aria-label={`Move label for ${text || "selected path"}`} title="Drag to move label. Arrow keys nudge; Home resets." style={{ transform: `translate(-50%, -50%) translate(${x}px, ${y}px)` }} onPointerDown={startDrag} onKeyDown={handleKeyDown}>
        {text}
      </div>
    </EdgeLabelRenderer> : null}
  </>;
}
