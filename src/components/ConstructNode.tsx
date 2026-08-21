import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { useState } from "react";
import { useWorkspace } from "../store";
import type { ConstructData, InteractionData } from "../types";

function interactionNote(interaction: InteractionData): string {
  const method = interaction.canonicalMethod === "product_indicator"
    ? "Product indicator"
    : interaction.canonicalMethod === "orthogonalizing"
      ? "Orthogonalizing"
      : "Two-stage product score";
  return interaction.kind === "interaction_v2"
    ? `${interaction.operands.length}-way · ${method} · ${interaction.hierarchyPolicy} hierarchy`
    : method;
}

export function ConstructNode({ id, data, selected }: NodeProps<Node<ConstructData>>) {
  const assignIndicators = useWorkspace((state) => state.assignIndicators);
  const [dropTarget, setDropTarget] = useState(false);
  const higherOrder = data.semantic === "higher_order";
  const acceptsIndicators = data.semantic !== "interaction" && data.semantic !== "higher_order";
  const shownIndicators = data.indicators.slice(0, 4);
  const hasResults = Boolean(data.resultR2 !== undefined || data.resultLoadings);
  return <div
    className={`construct-node${selected ? " selected" : ""}${dropTarget ? " drop-target" : ""}`}
    onDragEnter={(event) => { if (acceptsIndicators) { event.preventDefault(); setDropTarget(true); } }}
    onDragOver={(event) => { if (acceptsIndicators) { event.preventDefault(); event.dataTransfer.dropEffect = "move"; } }}
    onDragLeave={() => setDropTarget(false)}
    onDrop={(event) => {
      event.preventDefault();
      event.stopPropagation();
      setDropTarget(false);
      if (!acceptsIndicators) return;
      const encoded = event.dataTransfer.getData("application/qpls-indicators");
      const indicator = event.dataTransfer.getData("application/qpls-indicator");
      let indicators = indicator ? [indicator] : [];
      if (encoded) {
        try {
          const parsed: unknown = JSON.parse(encoded);
          if (!Array.isArray(parsed)) return;
          indicators = parsed.filter((value): value is string => typeof value === "string");
        } catch { return; }
      }
      if (indicators.length > 0) assignIndicators(id, indicators);
    }}
  >
    <Handle id="target-left" type="target" position={Position.Left} />
    <Handle id="target-top" type="target" position={Position.Top} />
    <Handle id="source-right" type="source" position={Position.Right} />
    <Handle id="source-bottom" type="source" position={Position.Bottom} />
    <div className="construct-kind">{data.semantic === "interaction" ? "INT" : data.semantic === "higher_order" ? "HOC" : data.mode === "reflective" ? "A" : "B"}</div>
    {!higherOrder ? <div className={`construct-score${hasResults ? " has-results" : ""}`}>{data.resultR2 !== undefined ? `R2 ${data.resultR2.toFixed(3)}` : "Model"}</div> : null}
    <strong title={data.label}>{data.label}</strong>{!higherOrder ? <span>[{data.shortName}]</span> : null}
    {data.semantic === "interaction" && data.interaction ? <small className="interaction-note">{interactionNote(data.interaction)}</small> : null}
    {acceptsIndicators ? <div className="indicator-strip">
      {shownIndicators.map((item) => <small key={item}>{item}{data.resultLoadings?.[item] !== undefined ? <b>{data.resultLoadings[item].toFixed(3)}</b> : null}</small>)}
      {data.indicators.length > shownIndicators.length ? <small>+{data.indicators.length - shownIndicators.length}</small> : null}
      {data.indicators.length === 0 ? <small className="empty-indicators">No indicators</small> : null}
    </div> : null}
  </div>;
}
