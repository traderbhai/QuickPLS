import {
  Controls,
  ReactFlow,
  type EdgeTypes,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import { memo } from "react";
import type { DiagramGraph } from "../domain/diagramGraph";
import { ConstructNode } from "./ConstructNode";
import { IndicatorNode } from "./IndicatorNode";
import { LatentNode } from "./LatentNode";
import { ModerationAnchorNode } from "./ModerationAnchorNode";
import { SemEdge } from "./SemEdge";

const nodeTypes: NodeTypes = {
  construct: memo(ConstructNode),
  latent: memo(LatentNode),
  indicator: memo(IndicatorNode),
  moderationAnchor: memo(ModerationAnchorNode),
};
const edgeTypes: EdgeTypes = { semEdge: SemEdge };

export function ReadOnlyDiagramViewport({ graph, ariaLabel }: { graph: DiagramGraph; ariaLabel: string }) {
  const fit = (instance: ReactFlowInstance<DiagramGraph["nodes"][number], DiagramGraph["edges"][number]>) => window.setTimeout(() => {
    void instance.fitView({ nodes: graph.nodes, padding: 0.2, minZoom: 0.35, maxZoom: 1.15, duration: 0 });
  }, 0);
  return <div className="nd-canvas-host readonly-diagram-viewport" role="group" aria-label={ariaLabel}>
    <ReactFlow<DiagramGraph["nodes"][number], DiagramGraph["edges"][number]>
      nodes={graph.nodes}
      edges={graph.edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onInit={fit}
      minZoom={0.25}
      maxZoom={2.2}
      nodesDraggable={false}
      nodesConnectable={false}
      nodesFocusable={false}
      elementsSelectable={false}
      edgesFocusable={false}
      edgesReconnectable={false}
      panOnDrag
      zoomOnDoubleClick={false}
      deleteKeyCode={null}
    >
      <Controls showInteractive={false} />
    </ReactFlow>
  </div>;
}
