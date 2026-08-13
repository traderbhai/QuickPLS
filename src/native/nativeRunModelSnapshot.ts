import type { Edge, Node } from "@xyflow/react";
import type {
  AnalysisModelSnapshot,
  AnalysisRun,
  ConstructData,
  DiagramLayoutState,
} from "../types";

function serializableClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function createAnalysisModelSnapshot(
  nodes: Array<Node<ConstructData>>,
  edges: Edge[],
  diagramLayout: DiagramLayoutState,
): AnalysisModelSnapshot {
  const snapshotNodes = nodes.map((node) => {
    const modelNode = { ...node };
    delete modelNode.selected;
    delete modelNode.dragging;
    delete modelNode.measured;
    return modelNode;
  });
  const snapshotEdges = edges.map((edge) => {
    const modelEdge = { ...edge };
    delete modelEdge.selected;
    return modelEdge;
  });
  return serializableClone({
    nodes: snapshotNodes,
    edges: snapshotEdges,
    diagramLayout,
  });
}

export function resolveAnalysisModel(
  run: AnalysisRun,
  liveNodes: Array<Node<ConstructData>>,
  liveEdges: Edge[],
  liveDiagramLayout: DiagramLayoutState,
): AnalysisModelSnapshot {
  return {
    nodes: run.modelSnapshot?.nodes ?? liveNodes,
    edges: run.modelSnapshot?.edges ?? liveEdges,
    diagramLayout: run.modelSnapshot?.diagramLayout ?? liveDiagramLayout,
  };
}
