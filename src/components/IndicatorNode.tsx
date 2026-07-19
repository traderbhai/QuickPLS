import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { Trash2 } from "lucide-react";
import { useWorkspace } from "../store";
import type { IndicatorNodeData } from "../domain/diagramGraph";

export function IndicatorNode({ data, selected }: NodeProps<Node<IndicatorNodeData>>) {
  const unassignIndicator = useWorkspace((state) => state.unassignIndicator);
  const statistic = data.mode === "reflective" ? data.loading : data.weight;
  const paperStyle = data.displayMode === "sem" || data.displayMode === "publication" || data.displayMode === "smartpls_result";
  if (paperStyle) {
    return <div
      className={`smartpls-indicator-node ${data.mode}${selected ? " selected" : ""}`}
    >
      <Handle className="smartpls-hidden-handle" id="target-left" type="target" position={Position.Left} />
      <Handle className="smartpls-hidden-handle" id="target-right" type="target" position={Position.Right} />
      <Handle className="smartpls-hidden-handle" id="target-top" type="target" position={Position.Top} />
      <Handle className="smartpls-hidden-handle" id="target-bottom" type="target" position={Position.Bottom} />
      <Handle className="smartpls-hidden-handle" id="source-left" type="source" position={Position.Left} />
      <Handle className="smartpls-hidden-handle" id="source-right" type="source" position={Position.Right} />
      <Handle className="smartpls-hidden-handle" id="source-top" type="source" position={Position.Top} />
      <Handle className="smartpls-hidden-handle" id="source-bottom" type="source" position={Position.Bottom} />
      <span title={data.indicator}>{data.indicator}</span>
    </div>;
  }
  return <div
    className={`indicator-node ${data.mode}${selected ? " selected" : ""}`}
  >
    <Handle id="target" type="target" position={Position.Left} />
    <Handle id="source" type="source" position={Position.Right} />
    <span title={data.indicator}>{data.indicator}</span>
    {statistic !== undefined ? <b>{statistic.toFixed(3)}</b> : null}
    <button title={`Remove ${data.indicator}`} onClick={(event) => { event.stopPropagation(); unassignIndicator(data.constructId, data.indicator); }}><Trash2 size={12} /></button>
  </div>;
}
