import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { useState } from "react";
import { useWorkspace } from "../store";
import type { LatentNodeData } from "../domain/diagramGraph";

export function LatentNode({ id, data, selected }: NodeProps<Node<LatentNodeData>>) {
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const assignIndicators = useWorkspace((state) => state.assignIndicators);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(data.label);
  const [dropTarget, setDropTarget] = useState(false);
  const commit = () => {
    const label = draft.trim();
    if (label && label !== data.label) updateConstruct(id, { label });
    setEditing(false);
  };
  const modeLabel = data.semantic === "interaction" ? "INT" : data.mode === "reflective" ? "Mode A" : "Mode B";

  const paperStyle = data.displayMode === "sem" || data.displayMode === "publication" || data.displayMode === "smartpls_result";
  const editablePaperMode = data.displayMode === "sem";
  const lockedResultMode = data.displayMode === "smartpls_result" || data.displayMode === "publication";
  const setCanvasDropTarget = (active: boolean) => {
    window.dispatchEvent(new CustomEvent("quickpls:diagram-drop-target", { detail: { constructId: active ? id : null } }));
  };

  if (paperStyle) {
    return <div
      className={`smartpls-latent-node ${data.mode}${selected ? " selected" : ""}${dropTarget ? " drop-target" : ""}`}
      onDragEnter={(event) => { if (lockedResultMode) return; event.preventDefault(); setDropTarget(true); setCanvasDropTarget(true); }}
      onDragOver={(event) => { if (lockedResultMode) return; event.preventDefault(); event.dataTransfer.dropEffect = "move"; setCanvasDropTarget(true); }}
      onDragLeave={() => { setDropTarget(false); setCanvasDropTarget(false); }}
      onDrop={(event) => {
        if (lockedResultMode) return;
        event.preventDefault();
        event.stopPropagation();
        setDropTarget(false);
        setCanvasDropTarget(false);
        const encoded = event.dataTransfer.getData("application/qpls-indicators");
        const indicator = event.dataTransfer.getData("application/qpls-indicator");
        let indicators = indicator ? [indicator] : [];
        if (encoded) {
          try {
            const parsed: unknown = JSON.parse(encoded);
            if (Array.isArray(parsed)) indicators = parsed.filter((value): value is string => typeof value === "string");
          } catch { return; }
        }
        if (indicators.length) assignIndicators(id, indicators);
      }}
    >
      <Handle className={editablePaperMode ? "smartpls-edit-handle" : "smartpls-hidden-handle"} id="target-left" type="target" position={Position.Left} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle" : "smartpls-hidden-handle"} id="target-right" type="target" position={Position.Right} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle" : "smartpls-hidden-handle"} id="target-top" type="target" position={Position.Top} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle" : "smartpls-hidden-handle"} id="target-bottom" type="target" position={Position.Bottom} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle source" : "smartpls-hidden-handle"} id="source-left" type="source" position={Position.Left} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle source" : "smartpls-hidden-handle"} id="source-right" type="source" position={Position.Right} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle source" : "smartpls-hidden-handle"} id="source-top" type="source" position={Position.Top} />
      <Handle className={editablePaperMode ? "smartpls-edit-handle source" : "smartpls-hidden-handle"} id="source-bottom" type="source" position={Position.Bottom} />
      <div className="smartpls-latent-ellipse">
        {data.resultR2 !== undefined && data.overlayMode !== "model" ? <span className="smartpls-r2">R² {data.resultR2.toFixed(3)}</span> : null}
      </div>
      {editing && !lockedResultMode ? <input
        className="smartpls-latent-edit"
        value={draft}
        autoFocus
        onChange={(event) => setDraft(event.target.value)}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") commit();
          if (event.key === "Escape") { setDraft(data.label); setEditing(false); }
        }}
      /> : <div className="smartpls-latent-label" role="button" tabIndex={0} title="Double-click to rename; drag the oval to move" onDoubleClick={() => { if (!lockedResultMode) { setDraft(data.label); setEditing(true); } }}>{data.label}</div>}
    </div>;
  }

  return <div
    className={`latent-node ${data.mode}${selected ? " selected" : ""}${dropTarget ? " drop-target" : ""}${data.semantic ? ` ${data.semantic}` : ""}`}
    onDragEnter={(event) => { event.preventDefault(); setDropTarget(true); setCanvasDropTarget(true); }}
    onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; setCanvasDropTarget(true); }}
    onDragLeave={() => { setDropTarget(false); setCanvasDropTarget(false); }}
    onDrop={(event) => {
      event.preventDefault();
      event.stopPropagation();
      setDropTarget(false);
      setCanvasDropTarget(false);
      const encoded = event.dataTransfer.getData("application/qpls-indicators");
      const indicator = event.dataTransfer.getData("application/qpls-indicator");
      let indicators = indicator ? [indicator] : [];
      if (encoded) {
        try {
          const parsed: unknown = JSON.parse(encoded);
          if (Array.isArray(parsed)) indicators = parsed.filter((value): value is string => typeof value === "string");
        } catch { return; }
      }
      if (indicators.length) assignIndicators(id, indicators);
    }}
  >
    <Handle id="target-left" type="target" position={Position.Left} />
    <Handle id="target-top" type="target" position={Position.Top} />
    <Handle id="source-right" type="source" position={Position.Right} />
    <Handle id="source-bottom" type="source" position={Position.Bottom} />
    <div className="latent-badge">{modeLabel}</div>
    {data.resultR2 !== undefined && data.overlayMode !== "model" ? <div className="latent-r2">R² {data.resultR2.toFixed(3)}</div> : null}
    {editing ? <input
      className="latent-edit"
      value={draft}
      autoFocus
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => {
        if (event.key === "Enter") commit();
        if (event.key === "Escape") { setDraft(data.label); setEditing(false); }
      }}
    /> : <button className="latent-title" title="Double-click to rename" onDoubleClick={() => { setDraft(data.label); setEditing(true); }}>{data.label}</button>}
    <span className="latent-short">[{data.shortName}]</span>
    <span className="latent-meta">{data.indicators.length} indicators | {data.pathCount} paths</span>
  </div>;
}
