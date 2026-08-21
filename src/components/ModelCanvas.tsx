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
import { buildDiagramGraph, isIndicatorNodeId, parseIndicatorNodeId } from "../domain/diagramGraph";
import {
  dispatchModerationCanvasRequest,
  isModerationAnchorData,
  isModerationConnectorData,
  MODERATION_FOCUS_EVENT,
  type ResultOverlaySelectionV1,
} from "../domain/moderationDiagramProjectionV1";
import type { StandardSemModelV4AuthorityRecordV1, StandardSemModelV4EditorIntentV1 } from "../domain/standardSemModelV4Authority";
import { compareUtf8StringsV1, type SemVariableV4 } from "../domain/semModelV4";
import { SEM_SIZES, boxCenter, semNodeBox } from "../domain/semGeometry";
import { nearestNativeModerationDropTarget, type NativeModerationDropTarget } from "../native/nativeModeration";
import { useWorkspace } from "../store";
import type { ConstructData, Dataset, DiagramToolMode, StandardSemPresentationLayoutV1 } from "../types";
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
  const dataset = useWorkspace((state) => state.dataset);
  const generalSemPublicationPending = useWorkspace((state) => state.generalSemPublicationPending);
  const strictAuthority = useWorkspace((state) => state.activeModelId
    ? state.standardSemModelV4Authorities[state.activeModelId] ?? null
    : null);
  const commitStandardIntent = useWorkspace((state) => state.commitStandardSemModelV4Intent);
  const onNodesChange = useWorkspace((state) => state.onNodesChange);
  const onEdgesChange = useWorkspace((state) => state.onEdgesChange);
  const onConnect = useWorkspace((state) => state.onConnect);
  const reconnectPath = useWorkspace((state) => state.reconnectPath);
  const addPath = useWorkspace((state) => state.addPath);
  const addCovariance = useWorkspace((state) => state.addCovariance);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setDiagramTool = useWorkspace((state) => state.setDiagramTool);
  const checkpoint = useWorkspace((state) => state.checkpoint);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const moveIndicator = useWorkspace((state) => state.moveIndicator);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const assignIndicators = useWorkspace((state) => state.assignIndicators);
  const undo = useWorkspace((state) => state.undo);
  const redo = useWorkspace((state) => state.redo);
  const readOnlyResultsPresentation = presentation === "results_readonly";
  const [flow, setFlow] = useState<ReactFlowInstance | null>(null);
  const previousNodeCount = useRef(nodes.length);
  const preserveViewportForDrop = useRef(false);
  const [pathSource, setPathSource] = useState<string | null>(null);
  const [dropHint, setDropHint] = useState<null | { count: number; x: number; y: number; targetConstructId?: string | null }>(null);
  const [dragGuide, setDragGuide] = useState<null | { vertical?: number; horizontal?: number; x: number; y: number; label: string }>(null);
  const [actionFeedback, setActionFeedback] = useState<null | { message: string; x?: number; y?: number }>(null);
  const [selectedInteractionTermId, setSelectedInteractionTermId] = useState<string | null>(null);
  const [moderationDropTarget, setModerationDropTarget] = useState<null | (NativeModerationDropTarget & { clientX: number; clientY: number })>(null);
  const moderationDropTargetRef = useRef<typeof moderationDropTarget>(null);
  const moderationDragOrigin = useRef<null | { id: string; position: { x: number; y: number } }>(null);
  const [draggingVariableCount, setDraggingVariableCount] = useState(0);
  const [hoverDropTargetId, setHoverDropTargetId] = useState<string | null>(null);
  const strictIdCounter = useRef(0);
  const nextStrictId = (kind: string) => `standard:editor:${kind}:${Date.now()}:${++strictIdCounter.current}`;
  const commitStrict = (intent: StandardSemModelV4EditorIntentV1) => {
    if (generalSemPublicationPending) {
      setActionFeedback({ message: "Wait for the calculation-ready project file to finish publishing before editing the model." });
      return;
    }
    setActionFeedback({ message: "Committing strict Standard model edit…" });
    void commitStandardIntent(intent).then((result) => {
      if (result.status === "committed") setActionFeedback({ message: "Committed to the strict Standard model authority." });
      else if (result.status === "blocked") setActionFeedback({ message: `Blocked: ${result.diagnostic.message} ${result.diagnostic.correctiveAction}` });
      else if (result.status === "stale") setActionFeedback({ message: "Stale edit ignored because the active authority changed." });
      else setActionFeedback({ message: `Rejected: ${result.error instanceof Error ? result.error.message : String(result.error)}` });
    });
  };
  const addStrictConstruct = (indicators: string[] = []) => commitStrict({
    kind: "add_construct",
    variable_id: nextStrictId("construct"),
    label: `Construct ${nodes.length + 1}`,
    representation: { kind: "composite", weighting: { kind: "mode_a" } },
    indicators: indicators.map((column) => observedForStrictCanvas(strictAuthority!, dataset, column)),
  });
  const assignStrictIndicators = (constructId: string, indicators: string[]) => commitStrict({
    kind: "assign_indicators",
    construct_id: constructId,
    indicators: indicators.map((column) => observedForStrictCanvas(strictAuthority!, dataset, column)),
  });
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
  const standardPresentation = diagramLayout.standardSemPresentation ?? { schemaVersion: 1, objects: [] };
  const updateStandardPresentation = (presentation: StandardSemPresentationLayoutV1) => {
    if (!strictAuthority || !canEditLayout) return;
    checkpoint();
    useWorkspace.setState((state) => ({
      diagramLayout: { ...state.diagramLayout, standardSemPresentation: presentation },
    }));
  };
  const visibleGraph = useMemo(() => {
    const hoveredRelationshipId = moderationDropTarget?.relationship.edgeId;
    if (!hoveredRelationshipId) return graph;
    return {
      ...graph,
      edges: graph.edges.map((edge) => {
        if (edge.id !== hoveredRelationshipId) return edge;
        const className = [edge.className, "moderation-drop-target-edge"].filter(Boolean).join(" ");
        return { ...edge, className, data: { ...edge.data, edgeClassName: className } };
      }),
    };
  }, [graph, moderationDropTarget?.relationship.edgeId]);
  const updateModerationDropTarget = (target: typeof moderationDropTarget) => {
    moderationDropTargetRef.current = target;
    setModerationDropTarget(target);
  };
  const visualNodeCenter = (nodeId: string, dragged?: Node) => {
    const node = dragged?.id === nodeId ? dragged : canvasNodes.find((candidate) => candidate.id === nodeId);
    return node ? boxCenter(semNodeBox(node)) : undefined;
  };
  const moderationTargetForDraggedNode = (node: Node) => {
    const modelNode = nodes.find((candidate) => candidate.id === node.id);
    if (!modelNode || modelNode.data.semantic || modelNode.data.indicators.length === 0) return null;
    return nearestNativeModerationDropTarget(
      nodes,
      edges,
      node.id,
      boxCenter(semNodeBox(node)),
      (id) => visualNodeCenter(id, node),
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
    if (draggingNodeId.current === null) setCanvasNodes(graph.nodes);
  }, [graph.nodes]);
  useEffect(() => {
    if (!selectedInteractionTermId) return;
    if (!graph.nodes.some((node) => isModerationAnchorData(node.data)
      && node.data.interactionTermId === selectedInteractionTermId)) {
      setSelectedInteractionTermId(null);
    }
  }, [graph.nodes, selectedInteractionTermId]);
  const arrangeModel = (direction: "horizontal" | "vertical" | "smartpls") => {
    autoLayout(direction);
    window.setTimeout(() => { void flow?.fitView({ padding: 0.2, duration: animationDuration(220) }); }, 0);
  };
  useEffect(() => {
    if (nodes.length > previousNodeCount.current) {
      if (preserveViewportForDrop.current) preserveViewportForDrop.current = false;
      else window.setTimeout(() => { void flow?.fitView({ padding: 0.16, duration: animationDuration(220) }); }, 0);
    }
    previousNodeCount.current = nodes.length;
  }, [flow, nodes.length]);

  useEffect(() => {
    const centerNode = (id: string) => {
      const node = graph.nodes.find((candidate) => candidate.id === id);
      if (!node || !flow) return;
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
  }, [flow, graph.edges, graph.nodes]);

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
      if (strictAuthority) commitStrict({
        kind: "add_relationship",
        relationship_id: nextStrictId("relationship"),
        definition: { kind: "structural", source, target, label: "Path" },
      });
      else addPath(source, target);
      setActionFeedback(null);
      return true;
    }
    if (covarianceExists(source, target)) {
      setActionFeedback({ message: "That covariance already exists between these constructs.", ...point });
      return false;
    }
    if (strictAuthority) commitStrict({
      kind: "add_relationship",
      relationship_id: nextStrictId("covariance"),
      definition: { kind: "covariance", left: { kind: "variable", id: source }, right: { kind: "variable", id: target }, label: "Covariance" },
    });
    else addCovariance(source, target);
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
    if (plan.checkpointBeforePersisting) checkpoint();
    for (const change of plan.indicatorKeyboardPositions) {
      moveIndicator(change.constructId, change.indicator, change.position);
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
      if (strictAuthority) addStrictConstruct();
      else addConstruct();
    };
    const handleArrange = (event: Event) => {
      const direction = (event as CustomEvent<{ direction?: "horizontal" | "vertical" | "smartpls" }>).detail?.direction ?? "smartpls";
      if (!canEditLayout) {
        setActionFeedback({ message: layoutLocked ? "Unlock layout before arranging the diagram." : "Switch to Edit model before arranging the diagram." });
        return;
      }
      arrangeModel(direction);
    };
    const handleFit = () => { void flow?.fitView({ padding: 0.22, duration: animationDuration(220) }); };
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
      if (strictAuthority) {
        const state = useWorkspace.getState();
        if (state.selectedNodeId) commitStrict({ kind: "delete_construct", variable_id: state.selectedNodeId });
        else if (state.selectedEdgeId) commitStrict({ kind: "delete_relationship", relationship_id: state.selectedEdgeId });
      } else removeSelection();
    };
    const handleUndo = () => { if (canEditLayout) undo(); };
    const handleRedo = () => { if (canEditLayout) redo(); };

    window.addEventListener("quickpls:model-tool", handleTool);
    window.addEventListener("quickpls:model-add-construct", handleAddConstruct);
    window.addEventListener("quickpls:model-arrange", handleArrange);
    window.addEventListener("quickpls:model-fit", handleFit);
    window.addEventListener("quickpls:model-delete-selection", handleDeleteSelection);
    window.addEventListener("quickpls:model-undo", handleUndo);
    window.addEventListener("quickpls:model-redo", handleRedo);
    return () => {
      window.removeEventListener("quickpls:model-tool", handleTool);
      window.removeEventListener("quickpls:model-add-construct", handleAddConstruct);
      window.removeEventListener("quickpls:model-arrange", handleArrange);
      window.removeEventListener("quickpls:model-fit", handleFit);
      window.removeEventListener("quickpls:model-delete-selection", handleDeleteSelection);
      window.removeEventListener("quickpls:model-undo", handleUndo);
      window.removeEventListener("quickpls:model-redo", handleRedo);
    };
  }, [addConstruct, arrangeModel, canEditLayout, flow, generalSemPublicationPending, layoutLocked, readOnlyResultsPresentation, redo, removeSelection, selectTool, selectedInteractionTermId, strictAuthority, undo]);
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
    if (event.key === "Escape" && selectedInteractionTermId) {
      event.preventDefault();
      setSelectedInteractionTermId(null);
      return;
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
    className={`model-canvas theme-${diagramLayout.diagramTheme}${paperStyleCanvas ? " smartpls-result-canvas" : ""}${resultDiagramMode ? " locked-result-canvas" : ""}${layoutLocked ? " layout-locked-canvas" : ""}${showDropCue ? " can-drop-variables" : ""}`}
    data-model-canvas-presentation={presentation}
    onKeyDownCapture={handleCanvasKeyDown}
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
      onInit={setFlow}
      defaultEdgeOptions={{ type: "smoothstep", markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 } }}
      onNodesChange={!canEditLayout ? undefined : onVisualNodesChange}
      onEdgesChange={!canEditLayout ? undefined : onVisualEdgesChange}
      onConnect={(connection) => {
        if (!canEditLayout) return;
        if (!connection.source || !connection.target || isIndicatorNodeId(connection.source) || isIndicatorNodeId(connection.target)) return;
        if (diagramTool === "covariance") {
          createPathOrCovariance(connection.source, connection.target);
          return;
        }
        if (connection.source === connection.target) {
          setActionFeedback({ message: "Self-paths are not valid. Connect two different constructs." });
          return;
        }
        if (structuralPathExists(connection.source, connection.target)) {
          setActionFeedback({ message: "That structural path already exists. Select the path to edit, reverse, or delete it." });
          return;
        }
        setActionFeedback(null);
        if (strictAuthority) commitStrict({
          kind: "add_relationship",
          relationship_id: nextStrictId("relationship"),
          definition: { kind: "structural", source: connection.source, target: connection.target, label: "Path" },
        });
        else onConnect(connection);
      }}
      onReconnect={!canEditLayout ? undefined : (edge, connection) => {
        if (!strictAuthority) {
          reconnectPath(edge, connection);
          return;
        }
        if (!connection.source || !connection.target) return;
        const relation = strictAuthority.model.relations.find((candidate) => candidate.id === edge.id);
        const label = relation ? strictAuthority.model.parameters.find((parameter) => parameter.id === relation.parameter)?.label ?? "Relationship" : "Relationship";
        commitStrict({
          kind: "replace_relationship",
          relationship_id: edge.id,
          definition: relation?.kind === "covariance"
            ? { kind: "covariance", left: { kind: "variable", id: connection.source }, right: { kind: "variable", id: connection.target }, label }
            : relation?.kind === "structural" && relation.role === "control"
              ? { kind: "control", source: connection.source, target: connection.target, label }
              : { kind: "structural", source: connection.source, target: connection.target, label },
        });
      }}
      onNodeDragStart={!canEditLayout ? undefined : (_, node) => {
        draggingNodeId.current = node.id;
        moderationDragOrigin.current = { id: node.id, position: { ...node.position } };
        updateModerationDropTarget(null);
        checkpoint();
      }}
      onNodeDrag={!canEditLayout ? undefined : (event, node) => {
        scheduleDragGuide(node);
        const target = moderationTargetForDraggedNode(node);
        updateModerationDropTarget(target ? {
          ...target,
          clientX: "clientX" in event ? Number(event.clientX) : 0,
          clientY: "clientY" in event ? Number(event.clientY) : 0,
        } : null);
      }}
      onNodeDragStop={!canEditLayout ? undefined : (_, node) => {
        draggingNodeId.current = null;
        cancelPendingDragGuide();
        setDragGuide(null);
        const moderationTarget = moderationDropTargetRef.current;
        const dragOrigin = moderationDragOrigin.current;
        updateModerationDropTarget(null);
        moderationDragOrigin.current = null;
        if (moderationTarget && dragOrigin?.id === node.id) {
          const restored = { id: node.id, type: "position" as const, position: dragOrigin.position, dragging: false };
          setCanvasNodes((current) => applyNodeChanges([restored] as NodeChange<(typeof current)[number]>[], current));
          onNodesChange([restored] as Array<NodeChange<Node<ConstructData>>>);
          setSelectedNode(node.id);
          requestCreateModeratingEffect(moderationTarget.relationship.edgeId, node.id, "drag");
          setActionFeedback({ message: `Moderating effect setup opened for ${moderationTarget.relationship.label}.` });
          return;
        }
        const indicator = parseIndicatorNodeId(node.id);
        if (!indicator) {
          onNodesChange([{ id: node.id, type: "position", position: node.position, dragging: false }]);
          return;
        }
        const target = nearestConstructForIndicator(node, indicator.constructId);
        if (target) {
          if (strictAuthority) assignStrictIndicators(target.id, [indicator.indicator]);
          else assignIndicator(target.id, indicator.indicator);
        }
        else moveIndicator(indicator.constructId, indicator.indicator, node.position);
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
        if (!canEditLayout || !isModerationAnchorData(node.data)) return;
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
          if (strictAuthority) addStrictConstruct();
          else addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
          return;
        }
        if (diagramTool === "indicator" || diagramTool === "residual" || diagramTool === "caption") return;
        if (event.detail !== 2) return;
        if (strictAuthority) addStrictConstruct();
        else addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
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
            if (strictAuthority) assignStrictIndicators(targetConstructId, indicators);
            else assignIndicators(targetConstructId, indicators);
            return;
          }
          preserveViewportForDrop.current = true;
          if (strictAuthority) addStrictConstruct(indicators);
          else addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }), indicators);
        }
      }}
      fitView
      fitViewOptions={{ padding: 0.2 }}
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

function observedForStrictCanvas(
  authority: StandardSemModelV4AuthorityRecordV1,
  dataset: Dataset,
  column: string,
): Extract<SemVariableV4, { kind: "observed" }> {
  const existing = authority.model.variables.find((variable): variable is Extract<SemVariableV4, { kind: "observed" }> =>
    variable.kind === "observed" && variable.source_column === column);
  if (existing) return structuredClone(existing);
  const metadata = dataset.columnMetadata?.find((item) => item.name === column);
  return {
    kind: "observed",
    id: `observed:${column}`,
    label: metadata?.label?.trim() || column,
    source_column: column,
    scale: metadata?.scale_type ?? "continuous",
    role: "indicator",
    categories: Object.keys(metadata?.value_labels ?? {}).sort(compareUtf8StringsV1),
    value_labels: { ...(metadata?.value_labels ?? {}) },
    missing_markers: [...new Set((metadata?.missing_markers ?? []).map((value) => value.trim()).filter(Boolean))].sort(compareUtf8StringsV1),
    transformation_lineage: [],
  };
}
