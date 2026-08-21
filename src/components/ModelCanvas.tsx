import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  ReactFlow,
  applyNodeChanges,
  type Edge,
  type EdgeChange,
  type EdgeTypes,
  type Node,
  type NodeChange,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { DragEvent, KeyboardEvent as ReactKeyboardEvent } from "react";
import { buildDiagramGraph, isIndicatorNodeId, parseIndicatorNodeId, type DiagramGraph } from "../domain/diagramGraph";
import {
  dispatchModerationCanvasRequest,
  isModerationAnchorData,
  isModerationConnectorData,
  MODERATION_FOCUS_EVENT,
  type ResultOverlaySelectionV1,
} from "../domain/moderationDiagramProjectionV1";
import { SEM_SIZES, boxCenter, semNodeBox } from "../domain/semGeometry";
import { nearestNativeModerationDropTarget, type NativeModerationDropTarget } from "../native/nativeModeration";
import {
  nativeCanvasSemanticZoomLevelV1,
  planNativeCanvasConnectionV1,
  projectNativeCanvasSemanticZoomV1,
} from "../native/nativeCanvasBehaviorV1";
import { useWorkspace } from "../store";
import type { ConstructData, DiagramToolMode, DiagramViewport, ModelEditCommandV1, StandardSemPresentationLayoutV1 } from "../types";
import { ConstructNode } from "./ConstructNode";
import { IndicatorNode } from "./IndicatorNode";
import { LatentNode } from "./LatentNode";
import { ModerationAnchorNode } from "./ModerationAnchorNode";
import { planModelCanvasNodeChanges } from "./modelCanvasNodeChangePlan";
import { SemEdge } from "./SemEdge";
import { StandardSemPresentationLayer } from "./StandardSemPresentationLayer";

const nodeTypes: NodeTypes = { construct: memo(ConstructNode), latent: memo(LatentNode), indicator: memo(IndicatorNode), moderationAnchor: memo(ModerationAnchorNode) };
const edgeTypes: EdgeTypes = { semEdge: SemEdge };
const SNAP_SIZE = 10;
const ALIGN_THRESHOLD = 8;
const LARGE_GRAPH_VISIBLE_RENDER_THRESHOLD = 100;
const animationDuration = (milliseconds: number) =>
  window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? 0 : milliseconds;

const smartplsNodeSize = { width: SEM_SIZES.smartplsLatent.width, height: SEM_SIZES.smartplsLatent.height };
const compactNodeSize = { width: 170, height: 118 };

export type ModelCanvasContextMenuTarget =
  | { kind: "canvas" }
  | { kind: "construct"; id: string }
  | { kind: "path"; id: string }
  | { kind: "moderation"; interactionTermId: string };

export interface ModelCanvasContextMenuRequest {
  clientX: number;
  clientY: number;
  returnFocus: HTMLElement | null;
  target: ModelCanvasContextMenuTarget;
}

export interface ModelCanvasProps {
  onContextMenuRequest?: (request: ModelCanvasContextMenuRequest) => void;
  presentation?: "editor" | "results_readonly";
  resultOverlay?: ResultOverlaySelectionV1 | null;
  showGeneratedInteractionTerms?: boolean;
}

/**
 * React Flow also reports changes for presentation-only measurement and HOC
 * membership edges. Keep those changes outside the scientific store for every
 * change shape, including defensive add events.
 */
export function persistentModelEdgeChanges(
  changes: readonly EdgeChange[],
  visualEdges: readonly Edge[],
): EdgeChange[] {
  const visualOnlyIds = new Set(visualEdges
    .filter((edge) => edge.data?.visualOnly === true)
    .map((edge) => edge.id));
  return changes.filter((change) => {
    if (change.type === "add") {
      return !change.item.id.startsWith("measurement::")
        && change.item.data?.visualOnly !== true;
    }
    if (!("id" in change)) return true;
    return !change.id.startsWith("measurement::")
      && !visualOnlyIds.has(change.id);
  });
}

export function persistentModelNodeChanges(
  changes: readonly NodeChange[],
  visualNodes: readonly Node[],
): NodeChange[] {
  const visualOnlyIds = new Set(visualNodes
    .filter((node) => node.data?.visualOnly === true)
    .map((node) => node.id));
  return changes.filter((change) => {
    if (change.type === "add") return change.item.data?.visualOnly !== true;
    if (!("id" in change)) return true;
    return !visualOnlyIds.has(change.id);
  });
}

/**
 * Navigator focus is a plain selection gesture. Mirror it into React Flow so
 * the next Ctrl-click extends from the focused construct instead of toggling a
 * stale Canvas selection.
 */
export function focusedConstructSelectionChanges(
  nodes: readonly Node<ConstructData>[],
  focusedId: string,
): Array<NodeChange<Node<ConstructData>>> {
  if (!nodes.some((node) => node.id === focusedId)) return [];
  const changes: Array<NodeChange<Node<ConstructData>>> = [];
  for (const node of nodes) {
    const selected = node.id === focusedId;
    if (Boolean(node.selected) !== selected) changes.push({ type: "select", id: node.id, selected });
  }
  return changes;
}

export function modelCanvasInitialViewportPlan(persistedViewport: DiagramViewport | undefined): {
  defaultViewport: DiagramViewport | undefined;
  fitOnInit: boolean;
} {
  return { defaultViewport: persistedViewport, fitOnInit: persistedViewport === undefined };
}

export function shouldAutoFitModelCanvasAfterNodeGrowth(input: {
  strictAuthority: boolean;
  persistedViewport: DiagramViewport | undefined;
  preserveViewportForDrop: boolean;
}): boolean {
  return !input.preserveViewportForDrop && !(input.strictAuthority && input.persistedViewport);
}

export function ModelCanvas({
  onContextMenuRequest,
  presentation = "editor",
  resultOverlay = null,
  showGeneratedInteractionTerms = false,
}: ModelCanvasProps) {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramTool = useWorkspace((state) => state.diagramTool);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const generalSemPublicationPending = useWorkspace((state) => state.generalSemPublicationPending);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const executeModelEditCommand = useWorkspace((state) => state.executeModelEditCommand);
  const onNodesChange = useWorkspace((state) => state.onNodesChange);
  const onEdgesChange = useWorkspace((state) => state.onEdgesChange);
  const reconnectPath = useWorkspace((state) => state.reconnectPath);
  const addCovariance = useWorkspace((state) => state.addCovariance);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setDiagramTool = useWorkspace((state) => state.setDiagramTool);
  const setDiagramViewport = useWorkspace((state) => state.setDiagramViewport);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const undo = useWorkspace((state) => state.undo);
  const redo = useWorkspace((state) => state.redo);
  const readOnlyResultsPresentation = presentation === "results_readonly";
  const [flow, setFlow] = useState<ReactFlowInstance<DiagramGraph["nodes"][number], Edge> | null>(null);
  const previousNodeCount = useRef(nodes.length);
  const preserveViewportForDrop = useRef(false);
  const [pathSource, setPathSource] = useState<string | null>(null);
  const [dropHint, setDropHint] = useState<null | { count: number; x: number; y: number; targetConstructId?: string | null }>(null);
  const [dragGuide, setDragGuide] = useState<null | { vertical?: number; horizontal?: number; x: number; y: number; label: string }>(null);
  const [actionFeedback, setActionFeedback] = useState<null | { message: string; x?: number; y?: number }>(null);
  const [selectedInteractionTermId, setSelectedInteractionTermId] = useState<string | null>(null);
  const [moderationDropTarget, setModerationDropTarget] = useState<null | (NativeModerationDropTarget & { clientX: number; clientY: number })>(null);
  const moderationDropTargetRef = useRef<typeof moderationDropTarget>(null);
  const connectSourceRef = useRef<string | null>(null);
  const connectCompletedRef = useRef(false);
  const [semanticZoom, setSemanticZoom] = useState<"far" | "medium" | "near">("near");
  const [isolatedNodeIds, setIsolatedNodeIds] = useState<Set<string> | null>(null);
  const [draggingVariableCount, setDraggingVariableCount] = useState(0);
  const [hoverDropTargetId, setHoverDropTargetId] = useState<string | null>(null);
  const strictIdCounter = useRef(0);
  const nextStableId = (kind: string) => `model:${kind}:${Date.now()}:${++strictIdCounter.current}`;
  const addCanvasConstruct = (position?: { x: number; y: number }, indicators: string[] = []) => {
    runModelEditCommand({
      kind: "add_construct",
      constructId: nextStableId("construct"),
      label: `Construct ${nodes.length + 1}`,
      columns: indicators,
      ...(position ? { position } : {}),
    });
  };
  const assignCanvasIndicators = (constructId: string, indicators: string[]) => {
    void executeModelEditCommand({ kind: "assign_indicators", constructId, columns: indicators }).then((result) => {
      setActionFeedback({
        message: result.status === "applied"
          ? `${indicators.length} indicator${indicators.length === 1 ? "" : "s"} assigned.`
          : `${result.message} ${result.correctiveAction}`,
      });
    });
  };
  const resultRuns = useMemo(() => runs.filter((run) => run.status === "completed" && run.result), [runs]);
  const selectedResultRun = useMemo(() => resultRuns.find((run) => run.id === selectedResultRunId), [resultRuns, selectedResultRunId]);
  const canvasDiagramMode = readOnlyResultsPresentation ? "smartpls_result" : diagramMode;
  const graph = useMemo(() => buildDiagramGraph(
    nodes,
    edges,
    canvasDiagramMode,
    diagramOverlaySettings.mode,
    readOnlyResultsPresentation ? undefined : selectedResultRun,
    {
      layout: diagramLayout,
      layoutSource: readOnlyResultsPresentation || diagramMode === "publication" ? "current_canvas" : undefined,
      selectedHigherOrderId: selectedNodeId,
      selectedInteractionTermId,
      resultOverlay,
      showGeneratedInteractionTerms,
      moderationAnchorFractions: diagramLayout.moderationAnchorFractions,
      moderationConnectorBendPoints: diagramLayout.moderationConnectorBendPoints,
    },
  ), [canvasDiagramMode, diagramLayout, diagramMode, diagramOverlaySettings.mode, edges, nodes, readOnlyResultsPresentation, resultOverlay, selectedInteractionTermId, selectedNodeId, selectedResultRun, showGeneratedInteractionTerms]);
  const [canvasNodes, setCanvasNodes] = useState(graph.nodes);
  const draggingNodeId = useRef<string | null>(null);
  const dragGuideFrame = useRef<number | null>(null);
  const pendingDragGuideNode = useRef<Node | null>(null);
  const resultDiagramMode = canvasDiagramMode === "smartpls_result" || canvasDiagramMode === "publication";
  const paperStyleCanvas = canvasDiagramMode === "sem" || canvasDiagramMode === "publication" || canvasDiagramMode === "smartpls_result";
  const layoutLocked = diagramLayout.layoutLocked && !resultDiagramMode;
  const canEditLayout = !resultDiagramMode && !layoutLocked && !generalSemPublicationPending;
  const initialViewportPlan = modelCanvasInitialViewportPlan(diagramLayout.diagramViewport);
  const standardPresentation = diagramLayout.standardSemPresentation ?? { schemaVersion: 1, objects: [] };
  const updateStandardPresentation = (presentation: StandardSemPresentationLayoutV1) => {
    if (!strictAuthority || !canEditLayout) return;
    runModelEditCommand({ kind: "set_standard_sem_presentation", presentation });
  };
  const semanticGraph = useMemo(() => ({
    ...graph,
    ...projectNativeCanvasSemanticZoomV1(graph.nodes, graph.edges, semanticZoom, isolatedNodeIds),
  }), [graph, isolatedNodeIds, semanticZoom]);
  const visibleGraph = useMemo(() => {
    const hoveredRelationshipId = moderationDropTarget?.relationship.edgeId;
    if (!hoveredRelationshipId) return semanticGraph;
    return {
      ...semanticGraph,
      edges: semanticGraph.edges.map((edge) => {
        if (edge.id !== hoveredRelationshipId) return edge;
        const className = [edge.className, "moderation-drop-target-edge"].filter(Boolean).join(" ");
        return { ...edge, className, data: { ...edge.data, edgeClassName: className } };
      }),
    };
  }, [moderationDropTarget?.relationship.edgeId, semanticGraph]);
  const updateModerationDropTarget = (target: typeof moderationDropTarget) => {
    moderationDropTargetRef.current = target;
    setModerationDropTarget(target);
  };
  const visualNodeCenter = (nodeId: string, dragged?: Node) => {
    const node = dragged?.id === nodeId ? dragged : canvasNodes.find((candidate) => candidate.id === nodeId);
    return node ? boxCenter(semNodeBox(node)) : undefined;
  };
  const moderationTargetForPointer = (moderatorId: string, clientX: number, clientY: number) => {
    if (!flow) return null;
    const modelNode = nodes.find((candidate) => candidate.id === moderatorId);
    if (!modelNode || modelNode.data.semantic || modelNode.data.indicators.length === 0) return null;
    return nearestNativeModerationDropTarget(
      nodes,
      edges,
      moderatorId,
      flow.screenToFlowPosition({ x: clientX, y: clientY }),
      (id) => visualNodeCenter(id),
    );
  };
  useEffect(() => {
    const handleModerationFocus = (event: Event) => {
      const termId = (event as CustomEvent<{ interactionTermId?: string }>).detail?.interactionTermId;
      if (termId) setSelectedInteractionTermId(termId);
    };
    window.addEventListener(MODERATION_FOCUS_EVENT, handleModerationFocus);
    return () => window.removeEventListener(MODERATION_FOCUS_EVENT, handleModerationFocus);
  }, []);
  useEffect(() => {
    const handleModelEditResult = (event: Event) => {
      const result = (event as CustomEvent<{ status?: string; message?: string; correctiveAction?: string }>).detail;
      if (!result) return;
      setActionFeedback({
        message: result.status === "applied"
          ? "Model updated."
          : `${result.message ?? "The edit could not be applied."} ${result.correctiveAction ?? "Review the selection and retry."}`,
      });
    };
    window.addEventListener("quickpls:model-edit-result", handleModelEditResult);
    return () => window.removeEventListener("quickpls:model-edit-result", handleModelEditResult);
  }, []);

  useEffect(() => {
    if (draggingNodeId.current === null) setCanvasNodes(semanticGraph.nodes);
  }, [semanticGraph.nodes]);
  useEffect(() => {
    if (!selectedInteractionTermId) return;
    if (!graph.nodes.some((node) => isModerationAnchorData(node.data)
      && node.data.interactionTermId === selectedInteractionTermId)) {
      setSelectedInteractionTermId(null);
    }
  }, [graph.nodes, selectedInteractionTermId]);
  const runModelEditCommand = (command: ModelEditCommandV1) => {
    void executeModelEditCommand(command).then((result) => {
      setActionFeedback({
        message: result.status === "applied"
          ? "Diagram updated."
          : `${result.message} ${result.correctiveAction}`,
      });
    });
  };
  const selectedConstructIds = () => [...new Set([
    ...nodes.filter((node) => node.selected && node.data.semantic !== "interaction").map((node) => node.id),
    ...(selectedNodeId && nodes.some((node) => node.id === selectedNodeId && node.data.semantic !== "interaction") ? [selectedNodeId] : []),
  ])];
  const arrangeModel = (direction: "horizontal" | "vertical" | "smartpls") => {
    runModelEditCommand({ kind: "arrange_model", direction });
    window.setTimeout(() => { void fitCanvas("structure"); }, 0);
  };
  const selectionScopeNodeIds = () => {
    const selected = new Set(selectedConstructIds());
    const selectedEdge = selectedEdgeId ? edges.find((edge) => edge.id === selectedEdgeId) : undefined;
    if (selectedEdge) {
      selected.add(selectedEdge.source);
      selected.add(selectedEdge.target);
    }
    if (selectedInteractionTermId) {
      const anchor = graph.nodes.find((node) => isModerationAnchorData(node.data)
        && node.data.interactionTermId === selectedInteractionTermId);
      if (anchor && isModerationAnchorData(anchor.data)) {
        selected.add(anchor.id);
        selected.add(anchor.data.predictorId);
        selected.add(anchor.data.outcomeId);
        anchor.data.moderatorIds.forEach((id) => selected.add(id));
      }
    }
    return selected;
  };
  const fitCanvas = (scope: "structure" | "all" | "selection") => {
    if (!flow) return;
    const candidateIds = scope === "all"
      ? new Set(graph.nodes.map((node) => node.id))
      : scope === "selection"
        ? selectionScopeNodeIds()
        : new Set(graph.nodes.filter((node) => !isIndicatorNodeId(node.id)).map((node) => node.id));
    const fitNodes = graph.nodes.filter((node) => candidateIds.has(node.id));
    if (!fitNodes.length) {
      setActionFeedback({ message: scope === "selection" ? "Select a construct, path, or moderating effect to fit." : "The model has no visible objects to fit." });
      return;
    }
    void flow.fitView({
      nodes: fitNodes,
      padding: scope === "selection" ? 0.32 : 0.18,
      minZoom: scope === "all" ? 0.25 : 0.55,
      maxZoom: scope === "selection" ? 1.25 : 1,
      duration: animationDuration(220),
    });
  };
  const toggleSelectionIsolation = () => {
    if (isolatedNodeIds) {
      setIsolatedNodeIds(null);
      setActionFeedback({ message: "Showing the complete model." });
      window.setTimeout(() => fitCanvas("structure"), 0);
      return;
    }
    const scope = selectionScopeNodeIds();
    if (!scope.size) {
      setActionFeedback({ message: "Select a construct, path, or moderating effect before using Focus selection." });
      return;
    }
    const expanded = new Set(scope);
    for (const edge of graph.edges) {
      if (edge.id.startsWith("measurement::") && (scope.has(edge.source) || scope.has(edge.target))) {
        expanded.add(edge.source);
        expanded.add(edge.target);
      }
    }
    setIsolatedNodeIds(expanded);
    setActionFeedback({ message: "Focused on the selected model region. Use Focus selection again to show all." });
    window.setTimeout(() => {
      void flow?.fitView({
        nodes: graph.nodes.filter((node) => expanded.has(node.id)),
        padding: 0.28,
        minZoom: 0.65,
        maxZoom: 1.25,
        duration: animationDuration(220),
      });
    }, 0);
  };
  useEffect(() => {
    if (nodes.length > previousNodeCount.current) {
      const preserveViewportForDropValue = preserveViewportForDrop.current;
      if (preserveViewportForDropValue) preserveViewportForDrop.current = false;
      else if (shouldAutoFitModelCanvasAfterNodeGrowth({
        strictAuthority: Boolean(strictAuthority),
        persistedViewport: diagramLayout.diagramViewport,
        preserveViewportForDrop: preserveViewportForDropValue,
      })) window.setTimeout(() => { fitCanvas("structure"); }, 0);
    }
    previousNodeCount.current = nodes.length;
  }, [diagramLayout.diagramViewport, flow, nodes.length, strictAuthority]);

  useEffect(() => {
    const centerNode = (id: string) => {
      const node = graph.nodes.find((candidate) => candidate.id === id);
      if (!node || !flow) return;
      const selectionChanges = focusedConstructSelectionChanges(nodes, id);
      if (selectionChanges.length) {
        setCanvasNodes((current) => applyNodeChanges(
          selectionChanges as Array<NodeChange<(typeof current)[number]>>,
          current,
        ));
        onNodesChange(selectionChanges);
      }
      const size = node.type === "latent" ? smartplsNodeSize : compactNodeSize;
      void flow.setCenter(node.position.x + size.width / 2, node.position.y + size.height / 2, { zoom: Math.max(0.75, flow.getZoom()), duration: animationDuration(240) });
    };
    const centerEdge = (id: string) => {
      const edge = graph.edges.find((candidate) => candidate.id === id);
      const source = edge ? graph.nodes.find((node) => node.id === edge.source) : null;
      const target = edge ? graph.nodes.find((node) => node.id === edge.target) : null;
      if (!source || !target || !flow) return;
      void flow.setCenter((source.position.x + target.position.x) / 2 + smartplsNodeSize.width / 2, (source.position.y + target.position.y) / 2 + smartplsNodeSize.height / 2, { zoom: Math.max(0.7, flow.getZoom()), duration: animationDuration(240) });
    };
    const centerModeration = (termId: string) => {
      const anchor = graph.nodes.find((node) => isModerationAnchorData(node.data)
        && node.data.interactionTermId === termId);
      if (!anchor || !flow) return;
      setSelectedInteractionTermId(termId);
      void flow.setCenter(anchor.position.x + 11, anchor.position.y + 11, { zoom: Math.max(0.8, flow.getZoom()), duration: animationDuration(240) });
    };
    const handleConstruct = (event: Event) => centerNode((event as CustomEvent<{ id: string }>).detail.id);
    const handleEdge = (event: Event) => centerEdge((event as CustomEvent<{ id: string }>).detail.id);
    const handleModeration = (event: Event) => centerModeration((event as CustomEvent<{ interactionTermId: string }>).detail.interactionTermId);
    window.addEventListener("quickpls:focus-construct", handleConstruct);
    window.addEventListener("quickpls:focus-edge", handleEdge);
    window.addEventListener("quickpls:focus-moderation", handleModeration);
    return () => {
      window.removeEventListener("quickpls:focus-construct", handleConstruct);
      window.removeEventListener("quickpls:focus-edge", handleEdge);
      window.removeEventListener("quickpls:focus-moderation", handleModeration);
    };
  }, [flow, graph.edges, graph.nodes, nodes, onNodesChange]);

  useEffect(() => {
    const handleVariablesDragging = (event: Event) => {
      const detail = (event as CustomEvent<{ count?: number }>).detail;
      setDraggingVariableCount(Math.max(0, Number(detail?.count ?? 0)));
      if (!detail?.count) {
        setDropHint(null);
        setHoverDropTargetId(null);
      }
    };
    window.addEventListener("quickpls:variables-dragging", handleVariablesDragging);
    return () => window.removeEventListener("quickpls:variables-dragging", handleVariablesDragging);
  }, []);

  useEffect(() => {
    const handleDiagramDropTarget = (event: Event) => {
      const detail = (event as CustomEvent<{ constructId?: string | null }>).detail;
      const constructId = typeof detail?.constructId === "string" ? detail.constructId : null;
      setHoverDropTargetId(constructId);
      if (constructId) setDropHint((current) => current ? { ...current, targetConstructId: constructId } : current);
    };
    window.addEventListener("quickpls:diagram-drop-target", handleDiagramDropTarget);
    return () => window.removeEventListener("quickpls:diagram-drop-target", handleDiagramDropTarget);
  }, []);

  const selectTool = (tool: DiagramToolMode) => {
    setDiagramTool(tool);
    setPathSource(null);
    setActionFeedback(null);
  };
  const covarianceExists = (source: string, target: string) => {
    const [left, right] = [source, target].sort();
    return edges.some((edge) => edge.data?.role === "covariance" && [edge.source, edge.target].sort().join("\u0000") === `${left}\u0000${right}`);
  };
  const structuralPathExists = (source: string, target: string) =>
    edges.some((edge) => edge.data?.role !== "covariance" && edge.source === source && edge.target === target);
  const createPathOrCovariance = (source: string, target: string, point?: { x: number; y: number }) => {
    if (source === target) {
      setActionFeedback({ message: "Self-paths and self-covariances are not valid SEM diagram actions.", ...point });
      return false;
    }
    if (diagramTool === "path") {
      if (structuralPathExists(source, target)) {
        setActionFeedback({ message: "That structural path already exists. Select the path to edit, reverse, or delete it.", ...point });
        return false;
      }
      runModelEditCommand({
        kind: "add_path",
        relationId: nextStableId("relationship"),
        sourceId: source,
        targetId: target,
        label: "Path",
      });
      setActionFeedback(null);
      return true;
    }
    if (covarianceExists(source, target)) {
      setActionFeedback({ message: "That covariance already exists between these constructs.", ...point });
      return false;
    }
    if (strictAuthority) {
      setActionFeedback({ message: "Covariance creation requires the calculation-ready revision workflow. Use Advanced Parameter Table to add it safely.", ...point });
      return false;
    }
    addCovariance(source, target);
    setActionFeedback(null);
    return true;
  };
  const onVisualNodesChange = (changes: NodeChange[]) => {
    setCanvasNodes((current) => applyNodeChanges(changes as NodeChange<(typeof current)[number]>[], current));
    const persistentChanges = persistentModelNodeChanges(changes, graph.nodes);
    const permittedChanges = strictAuthority ? persistentChanges.filter((change) => change.type !== "remove") : persistentChanges;
    const plan = planModelCanvasNodeChanges(
      permittedChanges as Array<NodeChange<Node>>,
      draggingNodeId.current !== null,
    );
    for (const change of plan.constructKeyboardPositions) {
      runModelEditCommand({ kind: "move_construct", constructId: change.constructId, position: change.position });
    }
    for (const change of plan.indicatorKeyboardPositions) {
      runModelEditCommand({ kind: "move_indicator", constructId: change.constructId, column: change.indicator, position: change.position });
    }
    if (plan.modelChanges.length) {
      onNodesChange(plan.modelChanges as Array<NodeChange<Node<ConstructData>>>);
    }
  };
  const onVisualEdgesChange = (changes: EdgeChange[]) => {
    if (strictAuthority) return;
    const modelChanges = persistentModelEdgeChanges(changes, graph.edges);
    if (modelChanges.length) onEdgesChange(modelChanges);
  };
  const chooseConstruct = (id: string, point?: { x: number; y: number }) => {
    setSelectedInteractionTermId(null);
    if (diagramTool === "path" || diagramTool === "covariance") {
      if (!pathSource) {
        setPathSource(id);
        setSelectedNode(id);
      } else if (pathSource !== id) {
        if (createPathOrCovariance(pathSource, id, point)) setPathSource(null);
      }
      return;
    }
    setSelectedNode(id);
  };
  const nearestConstructForIndicator = (indicatorNode: Node, sourceConstructId: string) => {
    const center = { x: indicatorNode.position.x + 39, y: indicatorNode.position.y + 12 };
    return graph.nodes.find((node) => node.type === "latent" && node.id !== sourceConstructId
      && Math.abs(center.x - (node.position.x + 44)) < 90
      && Math.abs(center.y - (node.position.y + 29)) < 70);
  };
  const updateDragGuide = (dragged: Node) => {
    if (!flow || isIndicatorNodeId(dragged.id)) {
      setDragGuide(null);
      return;
    }
    const size = paperStyleCanvas ? smartplsNodeSize : compactNodeSize;
    const draggedAnchors = [
      { axis: "x" as const, kind: "left", value: dragged.position.x },
      { axis: "x" as const, kind: "center", value: dragged.position.x + size.width / 2 },
      { axis: "x" as const, kind: "right", value: dragged.position.x + size.width },
      { axis: "y" as const, kind: "top", value: dragged.position.y },
      { axis: "y" as const, kind: "middle", value: dragged.position.y + size.height / 2 },
      { axis: "y" as const, kind: "bottom", value: dragged.position.y + size.height },
    ];
    const candidates = graph.nodes
      .filter((node) => node.id !== dragged.id && (node.type === "latent" || node.type === "construct"))
      .flatMap((node) => [
        { axis: "x" as const, kind: "left", value: node.position.x, label: String(node.data?.label ?? node.id) },
        { axis: "x" as const, kind: "center", value: node.position.x + size.width / 2, label: String(node.data?.label ?? node.id) },
        { axis: "x" as const, kind: "right", value: node.position.x + size.width, label: String(node.data?.label ?? node.id) },
        { axis: "y" as const, kind: "top", value: node.position.y, label: String(node.data?.label ?? node.id) },
        { axis: "y" as const, kind: "middle", value: node.position.y + size.height / 2, label: String(node.data?.label ?? node.id) },
        { axis: "y" as const, kind: "bottom", value: node.position.y + size.height, label: String(node.data?.label ?? node.id) },
      ]);
    const matched = draggedAnchors
      .flatMap((anchor) => candidates
        .filter((candidate) => candidate.axis === anchor.axis)
        .map((candidate) => ({ ...candidate, distance: Math.abs(candidate.value - anchor.value), anchorKind: anchor.kind })))
      .filter((candidate) => candidate.distance <= ALIGN_THRESHOLD)
      .sort((left, right) => left.distance - right.distance)[0];
    const snapped = {
      x: Math.round(dragged.position.x / SNAP_SIZE) * SNAP_SIZE,
      y: Math.round(dragged.position.y / SNAP_SIZE) * SNAP_SIZE,
    };
    if (!matched) {
      const screen = flow.flowToScreenPosition(snapped);
      setDragGuide({ x: screen.x, y: screen.y, label: `Snap ${snapped.x}, ${snapped.y}` });
      return;
    }
    const linePoint = matched.axis === "x"
      ? flow.flowToScreenPosition({ x: matched.value, y: dragged.position.y })
      : flow.flowToScreenPosition({ x: dragged.position.x, y: matched.value });
    const anchorLabel = matched.anchorKind === matched.kind ? matched.kind : `${matched.anchorKind} to ${matched.kind}`;
    setDragGuide({
      x: linePoint.x,
      y: linePoint.y,
      vertical: matched.axis === "x" ? linePoint.x : undefined,
      horizontal: matched.axis === "y" ? linePoint.y : undefined,
      label: `Align ${anchorLabel} with ${matched.label}`,
    });
  };
  const scheduleDragGuide = (dragged: Node) => {
    pendingDragGuideNode.current = dragged;
    if (dragGuideFrame.current !== null) return;
    dragGuideFrame.current = window.requestAnimationFrame(() => {
      dragGuideFrame.current = null;
      const pending = pendingDragGuideNode.current;
      pendingDragGuideNode.current = null;
      if (pending) updateDragGuide(pending);
    });
  };
  const cancelPendingDragGuide = () => {
    if (dragGuideFrame.current !== null) window.cancelAnimationFrame(dragGuideFrame.current);
    dragGuideFrame.current = null;
    pendingDragGuideNode.current = null;
  };
  useEffect(() => () => {
    if (dragGuideFrame.current !== null) window.cancelAnimationFrame(dragGuideFrame.current);
  }, []);
  const draggedIndicators = (event: DragEvent) => {
    const encoded = event.dataTransfer.getData("application/qpls-indicators");
    const indicator = event.dataTransfer.getData("application/qpls-indicator");
    if (encoded) {
      try {
        const parsed: unknown = JSON.parse(encoded);
        if (Array.isArray(parsed)) return parsed.filter((value): value is string => typeof value === "string");
      } catch { return []; }
    }
    return indicator ? [indicator] : [];
  };
  const dropTargetConstructId = (event: DragEvent) => {
    const element = (document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null) ?? (event.target as HTMLElement | null);
    const flowNode = element?.closest(".react-flow__node") as HTMLElement | null;
    const nodeId = flowNode?.dataset.id ?? flowNode?.getAttribute("data-id") ?? null;
    if (!nodeId) {
      if (hoverDropTargetId && nodes.some((node) => node.id === hoverDropTargetId)) return hoverDropTargetId;
      const highlighted = document.querySelector(".smartpls-latent-node.drop-target")?.closest(".react-flow__node") as HTMLElement | null;
      const highlightedId = highlighted?.dataset.id ?? highlighted?.getAttribute("data-id") ?? null;
      return highlightedId && nodes.some((node) => node.id === highlightedId) ? highlightedId : null;
    }
    const indicator = parseIndicatorNodeId(nodeId);
    if (indicator) return indicator.constructId;
    return nodes.some((node) => node.id === nodeId) ? nodeId : null;
  };
  const dropTargetLabel = dropHint?.targetConstructId
    ? nodes.find((node) => node.id === dropHint.targetConstructId)?.data.label
    : null;

  const showDropCue = draggingVariableCount > 0 && canEditLayout;
  useEffect(() => {
    if (readOnlyResultsPresentation) return;
    const handleTool = (event: Event) => {
      const tool = (event as CustomEvent<{ tool?: DiagramToolMode }>).detail?.tool;
      if (tool === "select" || tool === "pan" || tool === "path" || tool === "covariance") selectTool(tool);
    };
    const handleAddConstruct = () => {
      if (!canEditLayout) {
        setActionFeedback({ message: layoutLocked ? "Unlock layout before adding a construct." : "Switch to Edit model before adding a construct." });
        return;
      }
      addCanvasConstruct();
    };
    const handleArrange = (event: Event) => {
      const strategy = (event as CustomEvent<{ direction?: string }>).detail?.direction ?? "model-horizontal";
      if (!canEditLayout) {
        setActionFeedback({ message: layoutLocked ? "Unlock layout before arranging the diagram." : "Switch to Edit model before arranging the diagram." });
        return;
      }
      const constructIds = selectedConstructIds();
      if (strategy === "model-horizontal" || strategy === "model-vertical") {
        arrangeModel(strategy === "model-horizontal" ? "horizontal" : "vertical");
        return;
      }
      if (strategy === "tidy-selection") {
        runModelEditCommand({ kind: "tidy_constructs", constructIds });
        return;
      }
      if (strategy === "distribute-horizontal" || strategy === "distribute-vertical") {
        runModelEditCommand({
          kind: "distribute_constructs",
          constructIds,
          axis: strategy === "distribute-horizontal" ? "horizontal" : "vertical",
        });
        return;
      }
      const alignTargets = {
        "align-left": "left",
        "align-center": "centerX",
        "align-right": "right",
        "align-top": "top",
        "align-middle": "centerY",
        "align-bottom": "bottom",
      } as const;
      const target = alignTargets[strategy as keyof typeof alignTargets];
      if (target) runModelEditCommand({ kind: "align_constructs", constructIds, target });
    };
    const handleFit = (event: Event) => {
      const scope = (event as CustomEvent<{ scope?: "structure" | "all" | "selection" }>).detail?.scope ?? "structure";
      fitCanvas(scope);
    };
    const handleTogglePin = () => {
      const id = selectedNodeId;
      if (!id) {
        setActionFeedback({ message: "Select one construct before changing its pin state." });
        return;
      }
      const current = diagramLayout.constructLayouts[id]?.pinned ?? false;
      runModelEditCommand({ kind: "set_construct_pinned", constructId: id, pinned: !current });
    };
    const handleFocusSelection = () => toggleSelectionIsolation();
    const handleDeleteSelection = () => {
      if (!canEditLayout) {
        setActionFeedback({ message: generalSemPublicationPending
          ? "Wait for the calculation-ready project file to finish publishing before deleting diagram objects."
          : "Result and publication views are locked. Switch to Edit model before deleting diagram objects." });
        return;
      }
      if (selectedInteractionTermId) {
        dispatchModerationCanvasRequest({ action: "remove", interactionTermId: selectedInteractionTermId, origin: "menu" });
        return;
      }
      const state = useWorkspace.getState();
      if (state.selectedEdgeId) {
        runModelEditCommand({ kind: "remove_path", relationId: state.selectedEdgeId });
      } else if (!strictAuthority) {
        removeSelection();
      } else {
        setActionFeedback({ message: "Construct deletion needs a calculation-ready revision. Use Model > Create Calculation-Ready Revision…" });
      }
    };
    const handleUndo = () => { if (canEditLayout) undo(); };
    const handleRedo = () => { if (canEditLayout) redo(); };

    window.addEventListener("quickpls:model-tool", handleTool);
    window.addEventListener("quickpls:model-add-construct", handleAddConstruct);
    window.addEventListener("quickpls:model-arrange", handleArrange);
    window.addEventListener("quickpls:model-fit", handleFit);
    window.addEventListener("quickpls:model-toggle-pin", handleTogglePin);
    window.addEventListener("quickpls:model-focus-selection", handleFocusSelection);
    window.addEventListener("quickpls:model-delete-selection", handleDeleteSelection);
    window.addEventListener("quickpls:model-undo", handleUndo);
    window.addEventListener("quickpls:model-redo", handleRedo);
    return () => {
      window.removeEventListener("quickpls:model-tool", handleTool);
      window.removeEventListener("quickpls:model-add-construct", handleAddConstruct);
      window.removeEventListener("quickpls:model-arrange", handleArrange);
      window.removeEventListener("quickpls:model-fit", handleFit);
      window.removeEventListener("quickpls:model-toggle-pin", handleTogglePin);
      window.removeEventListener("quickpls:model-focus-selection", handleFocusSelection);
      window.removeEventListener("quickpls:model-delete-selection", handleDeleteSelection);
      window.removeEventListener("quickpls:model-undo", handleUndo);
      window.removeEventListener("quickpls:model-redo", handleRedo);
    };
  }, [arrangeModel, canEditLayout, diagramLayout.constructLayouts, flow, generalSemPublicationPending, layoutLocked, readOnlyResultsPresentation, redo, removeSelection, selectTool, selectedInteractionTermId, selectedNodeId, strictAuthority, undo]);
  const selectIndicatorForToolbar = (constructId: string, _indicator: string) => {
    setSelectedInteractionTermId(null);
    setSelectedNode(constructId);
  };
  const selectModeratingEffect = (interactionTermId: string) => {
    setSelectedNode(null);
    setSelectedEdge(null);
    setSelectedInteractionTermId(interactionTermId);
  };
  const clearSelectionForCanvas = () => {
    setSelectedInteractionTermId(null);
    setSelectedNode(null);
  };
  const requestCreateModeratingEffect = (relationId: string, moderatorId: string | undefined, origin: "drag" | "keyboard" | "menu") => {
    dispatchModerationCanvasRequest({
      action: "create",
      target: { kind: "focal_relation", relationId },
      moderatorId,
      origin,
    });
  };
  const handleCanvasKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest("input, textarea, select, [contenteditable='true']")) return;
    if (canEditLayout && selectedInteractionTermId && (event.key === "Enter" || event.key === "F2")) {
      event.preventDefault();
      dispatchModerationCanvasRequest({ action: "edit", interactionTermId: selectedInteractionTermId, origin: "keyboard" });
      return;
    }
    if (canEditLayout && selectedInteractionTermId && (event.key === "Delete" || event.key === "Backspace")) {
      event.preventDefault();
      dispatchModerationCanvasRequest({ action: "remove", interactionTermId: selectedInteractionTermId, origin: "keyboard" });
      return;
    }
    if (canEditLayout && selectedInteractionTermId && event.key.toLowerCase() === "m") {
      const selectedAnchor = graph.nodes.find((node) => isModerationAnchorData(node.data)
        && node.data.interactionTermId === selectedInteractionTermId);
      if (selectedAnchor && isModerationAnchorData(selectedAnchor.data) && selectedAnchor.data.order === 2) {
        event.preventDefault();
        dispatchModerationCanvasRequest({
          action: "create",
          target: { kind: "parent_interaction", interactionTermId: selectedInteractionTermId },
          origin: "keyboard",
        });
        return;
      }
    }
    if (event.key === "Escape") {
      if (connectSourceRef.current || moderationDropTargetRef.current) {
        event.preventDefault();
        connectSourceRef.current = null;
        connectCompletedRef.current = false;
        updateModerationDropTarget(null);
        setActionFeedback({ message: "Connection cancelled." });
        return;
      }
      if (selectedInteractionTermId) {
        event.preventDefault();
        setSelectedInteractionTermId(null);
        return;
      }
    }
    if (!canEditLayout || event.key.toLowerCase() !== "m" || !selectedEdgeId) return;
    const selectedEdge = edges.find((edge) => edge.id === selectedEdgeId);
    if (!selectedEdge || selectedEdge.data?.role === "control" || selectedEdge.data?.role === "covariance") return;
    event.preventDefault();
    requestCreateModeratingEffect(selectedEdge.id, undefined, "keyboard");
  };
  const requestNativeContextMenu = (event: { clientX: number; clientY: number; target: EventTarget | null; stopPropagation: () => void }, target: ModelCanvasContextMenuTarget) => {
    if (!onContextMenuRequest) return;
    event.stopPropagation();
    const eventTarget = event.target instanceof Element ? event.target : null;
    const returnFocus = eventTarget?.closest<HTMLElement>("[tabindex], button, input, select, textarea, [href]")
      ?? document.getElementById("nd-main");
    onContextMenuRequest({ clientX: event.clientX, clientY: event.clientY, returnFocus, target });
  };
  return <div
    className={`model-canvas theme-${diagramLayout.diagramTheme}${paperStyleCanvas ? " smartpls-result-canvas" : ""}${resultDiagramMode ? " locked-result-canvas" : ""}${layoutLocked ? " layout-locked-canvas" : ""}${showDropCue ? " can-drop-variables" : ""}${isolatedNodeIds ? " isolating-neighborhood" : ""} semantic-zoom-${semanticZoom}`}
    data-model-canvas-presentation={presentation}
    onKeyDownCapture={handleCanvasKeyDown}
    onPointerMove={(event) => {
      const moderatorId = connectSourceRef.current;
      if (!moderatorId || connectCompletedRef.current || !canEditLayout) return;
      const target = moderationTargetForPointer(moderatorId, event.clientX, event.clientY);
      updateModerationDropTarget(target ? { ...target, clientX: event.clientX, clientY: event.clientY } : null);
    }}
  >
    {resultDiagramMode && !readOnlyResultsPresentation ? <div className="canvas-tool-status warning">Result view is locked. Switch to Edit model to change diagram objects.</div> : null}
    {generalSemPublicationPending && !readOnlyResultsPresentation ? <div className="canvas-tool-status warning" role="status">Calculation-ready project publication is in progress. Canvas editing is temporarily locked.</div> : null}
    {!resultDiagramMode && (diagramTool === "path" || diagramTool === "covariance") ? <span className="sr-only" role="status" aria-live="polite">{pathSource ? `Choose ${diagramTool === "path" ? "outcome construct" : "second construct"}` : `Choose ${diagramTool === "path" ? "predictor construct" : "first construct"}`}</span> : null}
    {actionFeedback && !readOnlyResultsPresentation ? <div
      className={`canvas-action-feedback${actionFeedback.x !== undefined && actionFeedback.y !== undefined ? " local" : ""}`}
      style={actionFeedback.x !== undefined && actionFeedback.y !== undefined ? { left: actionFeedback.x + 12, top: actionFeedback.y + 12 } : undefined}
      role="status"
      aria-live="polite"
    >{actionFeedback.message}</div> : null}
    {showDropCue && !dropHint ? <div className="canvas-drop-guide" aria-live="polite">
      <strong>Drop on canvas</strong>
      <span>Create a construct, or drop onto an oval to assign indicators.</span>
    </div> : null}
    {dropHint ? <div className="canvas-drop-hint" style={{ left: dropHint.x + 14, top: dropHint.y + 14 }} aria-live="polite">
      <strong>{dropTargetLabel ? `Drop on ${dropTargetLabel}` : "Drop to create construct"}</strong>
      <span>{dropHint.count} variable{dropHint.count === 1 ? "" : "s"} will {dropTargetLabel ? "be assigned as indicator" : "become indicator"}{dropHint.count === 1 ? "" : "s"}</span>
    </div> : null}
    {dragGuide?.vertical !== undefined ? <div className="canvas-alignment-guide vertical" style={{ left: dragGuide.vertical }} /> : null}
    {dragGuide?.horizontal !== undefined ? <div className="canvas-alignment-guide horizontal" style={{ top: dragGuide.horizontal }} /> : null}
    {dragGuide ? <div className="canvas-snap-hint" style={{ left: dragGuide.x + 12, top: dragGuide.y + 12 }}>{dragGuide.label}</div> : null}
    {moderationDropTarget ? <div
      className="canvas-moderation-drop-hint"
      style={{ left: moderationDropTarget.clientX + 14, top: moderationDropTarget.clientY + 14 }}
      role="status"
      aria-live="polite"
    >
      <strong>Release to moderate</strong>
      <span>{moderationDropTarget.relationship.label}</span>
    </div> : null}
    <ReactFlow
      nodes={canvasNodes}
      edges={visibleGraph.edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      defaultViewport={initialViewportPlan.defaultViewport}
      onInit={(instance) => {
        setFlow(instance);
        if (!initialViewportPlan.fitOnInit) return;
        window.setTimeout(() => {
          void instance.fitView({
            nodes: graph.nodes.filter((node) => !isIndicatorNodeId(node.id)),
            padding: 0.18,
            minZoom: 0.55,
            maxZoom: 1,
            duration: animationDuration(180),
          });
        }, 0);
      }}
      defaultEdgeOptions={{ type: "smoothstep", markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 } }}
      onNodesChange={!canEditLayout ? undefined : onVisualNodesChange}
      onEdgesChange={!canEditLayout ? undefined : onVisualEdgesChange}
      onConnectStart={(_, params) => {
        connectCompletedRef.current = false;
        updateModerationDropTarget(null);
        const source = params.nodeId ? nodes.find((node) => node.id === params.nodeId) : null;
        connectSourceRef.current = source && !source.data.semantic && source.data.indicators.length > 0
          ? source.id
          : null;
      }}
      onConnect={(connection) => {
        if (!canEditLayout) return;
        connectCompletedRef.current = true;
        if (!connection.source || !connection.target || isIndicatorNodeId(connection.source) || isIndicatorNodeId(connection.target)) return;
        const anchor = graph.nodes.find((node) => node.id === connection.target && isModerationAnchorData(node.data));
        if (diagramTool === "covariance") {
          createPathOrCovariance(connection.source, connection.target);
          return;
        }
        const plan = planNativeCanvasConnectionV1({
          sourceConstructId: connection.source,
          target: anchor && isModerationAnchorData(anchor.data)
            ? {
              kind: "moderation_anchor",
              visualNodeId: anchor.id,
              interactionTermId: anchor.data.interactionTermId,
              order: anchor.data.order,
            }
            : { kind: "construct", constructId: connection.target },
          relationId: nextStableId("relationship"),
          structuralPathExists: anchor ? false : structuralPathExists(connection.source, connection.target),
          origin: "drag",
        });
        if (plan.status === "blocked") {
          setActionFeedback({ message: plan.message });
          return;
        }
        setActionFeedback(null);
        if (plan.operation === "moderating_effect") {
          dispatchModerationCanvasRequest(plan.request);
          setActionFeedback({ message: `Three-way moderating effect setup opened for ${anchor && isModerationAnchorData(anchor.data) ? anchor.data.label : "the selected path"}.` });
          return;
        }
        runModelEditCommand(plan.command);
      }}
      onConnectEnd={(event) => {
        const source = connectSourceRef.current;
        const completed = connectCompletedRef.current;
        const point = "changedTouches" in event
          ? event.changedTouches.item(0)
          : event;
        const clientX = Number(point?.clientX ?? 0);
        const clientY = Number(point?.clientY ?? 0);
        const target = source && !completed
          ? moderationDropTargetRef.current ?? moderationTargetForPointer(source, clientX, clientY)
          : null;
        if (source && target && !completed) {
          const plan = planNativeCanvasConnectionV1({
            sourceConstructId: source,
            target: { kind: "focal_relation", relationId: target.relationship.edgeId },
            origin: "drag",
          });
          if (plan.status === "ready" && plan.operation === "moderating_effect") {
            dispatchModerationCanvasRequest(plan.request);
            setActionFeedback({ message: `Moderating effect setup opened for ${target.relationship.label}.` });
          } else if (plan.status === "blocked") {
            setActionFeedback({ message: plan.message });
          }
        }
        connectSourceRef.current = null;
        connectCompletedRef.current = false;
        updateModerationDropTarget(null);
      }}
      onReconnect={!canEditLayout ? undefined : (edge, connection) => {
        if (!strictAuthority) {
          reconnectPath(edge, connection);
          return;
        }
        setActionFeedback({
          message: "Retargeting a calculation-ready relationship needs a versioned revision. Reverse or remove the path here, or create a calculation-ready revision first.",
        });
      }}
      onNodeDragStart={!canEditLayout ? undefined : (_, node) => {
        draggingNodeId.current = node.id;
        updateModerationDropTarget(null);
      }}
      onNodeDrag={!canEditLayout ? undefined : (_event, node) => {
        scheduleDragGuide(node);
      }}
      onNodeDragStop={!canEditLayout ? undefined : (_, node) => {
        draggingNodeId.current = null;
        cancelPendingDragGuide();
        setDragGuide(null);
        updateModerationDropTarget(null);
        const indicator = parseIndicatorNodeId(node.id);
        if (!indicator) {
          if (!isModerationAnchorData(node.data)) {
            runModelEditCommand({ kind: "move_construct", constructId: node.id, position: node.position });
          }
          return;
        }
        const target = nearestConstructForIndicator(node, indicator.constructId);
        if (target) {
          assignCanvasIndicators(target.id, [indicator.indicator]);
        }
        else runModelEditCommand({ kind: "move_indicator", constructId: indicator.constructId, column: indicator.indicator, position: node.position });
      }}
      onNodeClick={(event, node) => {
        if (isModerationAnchorData(node.data)) {
          selectModeratingEffect(node.data.interactionTermId);
          return;
        }
        const indicator = parseIndicatorNodeId(node.id);
        if (indicator) {
          selectIndicatorForToolbar(indicator.constructId, indicator.indicator);
          return;
        }
        if (!canEditLayout) {
          setSelectedNode(node.id);
          return;
        }
        chooseConstruct(node.id, { x: event.clientX, y: event.clientY });
      }}
      onNodeDoubleClick={(event, node) => {
        if (!canEditLayout) return;
        if (node.data.semantic === "higher_order") {
          event.preventDefault();
          setSelectedNode(node.id);
          window.dispatchEvent(new CustomEvent("quickpls:edit-higher-order", { detail: { constructId: node.id } }));
          return;
        }
        if (!isModerationAnchorData(node.data)) return;
        event.preventDefault();
        selectModeratingEffect(node.data.interactionTermId);
        dispatchModerationCanvasRequest({
          action: "edit",
          interactionTermId: node.data.interactionTermId,
          origin: "anchor",
        });
      }}
      onEdgeClick={(_, edge) => {
        if (isModerationConnectorData(edge.data)) {
          selectModeratingEffect(edge.data.interactionTermId);
          return;
        }
        setSelectedInteractionTermId(null);
        setSelectedEdge(edge.id);
      }}
      onNodeContextMenu={(event, node) => {
        event.preventDefault();
        if (!canEditLayout) return;
        if (isModerationAnchorData(node.data)) {
          selectModeratingEffect(node.data.interactionTermId);
          requestNativeContextMenu(event, { kind: "moderation", interactionTermId: node.data.interactionTermId });
          return;
        }
        const indicator = parseIndicatorNodeId(node.id);
        if (indicator) {
          selectIndicatorForToolbar(indicator.constructId, indicator.indicator);
          requestNativeContextMenu(event, { kind: "construct", id: indicator.constructId });
        } else {
          setSelectedNode(node.id);
          requestNativeContextMenu(event, { kind: "construct", id: node.id });
        }
      }}
      onEdgeContextMenu={(event, edge) => {
        event.preventDefault();
        if (!canEditLayout) return;
        if (isModerationConnectorData(edge.data)) {
          selectModeratingEffect(edge.data.interactionTermId);
          requestNativeContextMenu(event, { kind: "moderation", interactionTermId: edge.data.interactionTermId });
          return;
        }
        if (edge.id.startsWith("measurement::")) {
          clearSelectionForCanvas();
          requestNativeContextMenu(event, { kind: "canvas" });
          return;
        }
        setSelectedEdge(edge.id);
        requestNativeContextMenu(event, { kind: "path", id: edge.id });
      }}
      onPaneContextMenu={(event) => {
        event.preventDefault();
        if (!canEditLayout) return;
        clearSelectionForCanvas();
        requestNativeContextMenu(event, { kind: "canvas" });
      }}
      onPaneClick={(event) => {
        clearSelectionForCanvas();
        if (diagramTool === "path" || diagramTool === "covariance") { setPathSource(null); return; }
        if (!canEditLayout) return;
        if (!flow) return;
        if (diagramTool === "construct") {
          addCanvasConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
          return;
        }
        if (diagramTool === "indicator" || diagramTool === "residual" || diagramTool === "caption") return;
        if (event.detail !== 2) return;
        addCanvasConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
      }}
      onDragOver={(event) => {
        if (!canEditLayout) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
        const indicators = draggedIndicators(event);
        const count = indicators.length || draggingVariableCount;
        if (count > 0) setDropHint({ count, x: event.clientX, y: event.clientY, targetConstructId: dropTargetConstructId(event) ?? hoverDropTargetId });
      }}
      onDragLeave={(event) => {
        const related = event.relatedTarget;
        if (!(related instanceof globalThis.Node) || !event.currentTarget.contains(related)) setDropHint(null);
      }}
      onDrop={(event) => {
        event.preventDefault();
        setDropHint(null);
        setDraggingVariableCount(0);
        setHoverDropTargetId(null);
        if (!canEditLayout) return;
        if (!flow) return;
        const indicators = draggedIndicators(event);
        if (indicators.length > 0) {
          const targetConstructId = dropTargetConstructId(event);
          if (targetConstructId) {
            assignCanvasIndicators(targetConstructId, indicators);
            return;
          }
          preserveViewportForDrop.current = true;
          addCanvasConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }), indicators);
        }
      }}
      onMove={(_, viewport) => {
        const level = nativeCanvasSemanticZoomLevelV1(viewport.zoom);
        setSemanticZoom((current) => current === level ? current : level);
      }}
      onMoveEnd={(_, viewport) => setDiagramViewport({ x: viewport.x, y: viewport.y, zoom: viewport.zoom })}
      minZoom={0.25}
      maxZoom={2.2}
      selectionOnDrag
      panOnDrag={resultDiagramMode || diagramTool === "pan"}
      multiSelectionKeyCode="Control"
      snapToGrid
      snapGrid={[10, 10]}
      onlyRenderVisibleElements={visibleGraph.nodes.length + visibleGraph.edges.length >= LARGE_GRAPH_VISIBLE_RENDER_THRESHOLD}
      nodesDraggable={canEditLayout && diagramTool !== "pan"}
      nodesConnectable={canEditLayout}
      edgesReconnectable={canEditLayout}
      deleteKeyCode={null}
    >
      {strictAuthority ? <StandardSemPresentationLayer
        presentation={standardPresentation}
        editable={canEditLayout}
        onChange={updateStandardPresentation}
      /> : null}
      {diagramLayout.showGrid && !resultDiagramMode ? <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#dbe1e4" /> : null}
      <Controls showInteractive={false} />
    </ReactFlow>
  </div>;
}
