import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  ReactFlow,
  type EdgeChange,
  type EdgeTypes,
  type Node,
  type NodeChange,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { DragEvent } from "react";
import { buildDiagramGraph, isIndicatorNodeId, parseIndicatorNodeId } from "../domain/diagramGraph";
import { SEM_SIZES } from "../domain/semGeometry";
import { useWorkspace } from "../store";
import type { ConstructData, DiagramToolMode } from "../types";
import { ConstructNode } from "./ConstructNode";
import { IndicatorNode } from "./IndicatorNode";
import { LatentNode } from "./LatentNode";
import { SemEdge } from "./SemEdge";

const nodeTypes: NodeTypes = { construct: memo(ConstructNode), latent: memo(LatentNode), indicator: memo(IndicatorNode) };
const edgeTypes: EdgeTypes = { semEdge: SemEdge };
const SNAP_SIZE = 10;
const ALIGN_THRESHOLD = 8;
const animationDuration = (milliseconds: number) =>
  window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ? 0 : milliseconds;

const smartplsNodeSize = { width: SEM_SIZES.smartplsLatent.width, height: SEM_SIZES.smartplsLatent.height };
const compactNodeSize = { width: 170, height: 118 };

export type ModelCanvasContextMenuTarget =
  | { kind: "canvas" }
  | { kind: "construct"; id: string }
  | { kind: "path"; id: string };

export interface ModelCanvasContextMenuRequest {
  clientX: number;
  clientY: number;
  returnFocus: HTMLElement | null;
  target: ModelCanvasContextMenuTarget;
}

export interface ModelCanvasProps {
  onContextMenuRequest?: (request: ModelCanvasContextMenuRequest) => void;
}

export function ModelCanvas({ onContextMenuRequest }: ModelCanvasProps) {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramTool = useWorkspace((state) => state.diagramTool);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const onNodesChange = useWorkspace((state) => state.onNodesChange);
  const onEdgesChange = useWorkspace((state) => state.onEdgesChange);
  const onConnect = useWorkspace((state) => state.onConnect);
  const reconnectPath = useWorkspace((state) => state.reconnectPath);
  const addPath = useWorkspace((state) => state.addPath);
  const addCovariance = useWorkspace((state) => state.addCovariance);
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
  const [flow, setFlow] = useState<ReactFlowInstance | null>(null);
  const previousNodeCount = useRef(nodes.length);
  const preserveViewportForDrop = useRef(false);
  const [pathSource, setPathSource] = useState<string | null>(null);
  const [dropHint, setDropHint] = useState<null | { count: number; x: number; y: number; targetConstructId?: string | null }>(null);
  const [dragGuide, setDragGuide] = useState<null | { vertical?: number; horizontal?: number; x: number; y: number; label: string }>(null);
  const [actionFeedback, setActionFeedback] = useState<null | { message: string; x?: number; y?: number }>(null);
  const [draggingVariableCount, setDraggingVariableCount] = useState(0);
  const [hoverDropTargetId, setHoverDropTargetId] = useState<string | null>(null);
  const resultRuns = useMemo(() => runs.filter((run) => run.status === "completed" && run.result), [runs]);
  const selectedResultRun = useMemo(() => resultRuns.find((run) => run.id === selectedResultRunId), [resultRuns, selectedResultRunId]);
  const graph = useMemo(() => buildDiagramGraph(nodes, edges, diagramMode, diagramOverlaySettings.mode, selectedResultRun, { layout: diagramLayout, layoutSource: diagramMode === "publication" ? "current_canvas" : undefined }), [diagramLayout, diagramMode, diagramOverlaySettings.mode, edges, nodes, selectedResultRun]);
  const resultDiagramMode = diagramMode === "smartpls_result" || diagramMode === "publication";
  const paperStyleCanvas = diagramMode === "sem" || diagramMode === "publication" || diagramMode === "smartpls_result";
  const layoutLocked = diagramLayout.layoutLocked && !resultDiagramMode;
  const canEditLayout = !resultDiagramMode && !layoutLocked;
  const visibleGraph = graph;
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
    const handleConstruct = (event: Event) => centerNode((event as CustomEvent<{ id: string }>).detail.id);
    const handleEdge = (event: Event) => centerEdge((event as CustomEvent<{ id: string }>).detail.id);
    window.addEventListener("quickpls:focus-construct", handleConstruct);
    window.addEventListener("quickpls:focus-edge", handleEdge);
    return () => {
      window.removeEventListener("quickpls:focus-construct", handleConstruct);
      window.removeEventListener("quickpls:focus-edge", handleEdge);
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
      addPath(source, target);
      setActionFeedback(null);
      return true;
    }
    if (covarianceExists(source, target)) {
      setActionFeedback({ message: "That covariance display arc already exists.", ...point });
      return false;
    }
    addCovariance(source, target);
    setActionFeedback(null);
    return true;
  };
  const onVisualNodesChange = (changes: NodeChange[]) => {
    const modelChanges = changes.filter((change) => !("id" in change) || !isIndicatorNodeId(change.id)) as Array<NodeChange<Node<ConstructData>>>;
    for (const change of changes) {
      if (!("id" in change) || !isIndicatorNodeId(change.id) || change.type !== "position" || !change.position) continue;
      const indicator = parseIndicatorNodeId(change.id);
      if (indicator) moveIndicator(indicator.constructId, indicator.indicator, change.position);
    }
    if (modelChanges.length) onNodesChange(modelChanges);
  };
  const onVisualEdgesChange = (changes: EdgeChange[]) => {
    const modelChanges = changes.filter((change) => !("id" in change) || !change.id.startsWith("measurement::"));
    if (modelChanges.length) onEdgesChange(modelChanges);
  };
  const chooseConstruct = (id: string, point?: { x: number; y: number }) => {
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
    const handleTool = (event: Event) => {
      const tool = (event as CustomEvent<{ tool?: DiagramToolMode }>).detail?.tool;
      if (tool === "select" || tool === "pan" || tool === "path" || tool === "covariance") selectTool(tool);
    };
    const handleAddConstruct = () => {
      if (!canEditLayout) {
        setActionFeedback({ message: layoutLocked ? "Unlock layout before adding a construct." : "Switch to Edit model before adding a construct." });
        return;
      }
      addConstruct();
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
      if (resultDiagramMode) {
        setActionFeedback({ message: "Result and publication views are locked. Switch to Edit model before deleting diagram objects." });
        return;
      }
      removeSelection();
    };
    const handleUndo = () => undo();
    const handleRedo = () => redo();

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
  }, [addConstruct, arrangeModel, canEditLayout, flow, layoutLocked, redo, removeSelection, resultDiagramMode, selectTool, undo]);
  const selectIndicatorForToolbar = (constructId: string, _indicator: string) => {
    setSelectedNode(constructId);
  };
  const clearSelectionForCanvas = () => {
    setSelectedNode(null);
  };
  const requestNativeContextMenu = (event: { clientX: number; clientY: number; target: EventTarget | null; stopPropagation: () => void }, target: ModelCanvasContextMenuTarget) => {
    if (!onContextMenuRequest) return;
    event.stopPropagation();
    const eventTarget = event.target instanceof Element ? event.target : null;
    const returnFocus = eventTarget?.closest<HTMLElement>("[tabindex], button, input, select, textarea, [href]")
      ?? document.getElementById("nd-main");
    onContextMenuRequest({ clientX: event.clientX, clientY: event.clientY, returnFocus, target });
  };
  return <div className={`model-canvas theme-${diagramLayout.diagramTheme}${paperStyleCanvas ? " smartpls-result-canvas" : ""}${resultDiagramMode ? " locked-result-canvas" : ""}${layoutLocked ? " layout-locked-canvas" : ""}${showDropCue ? " can-drop-variables" : ""}`}>
    {resultDiagramMode ? <div className="canvas-tool-status warning">Result view is locked. Switch to Edit model to change diagram objects.</div> : null}
    {!resultDiagramMode && (diagramTool === "path" || diagramTool === "covariance") ? <span className="sr-only" role="status" aria-live="polite">{pathSource ? `Choose ${diagramTool === "path" ? "outcome construct" : "second construct"}` : `Choose ${diagramTool === "path" ? "predictor construct" : "first construct"}`}</span> : null}
    {actionFeedback ? <div
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
    <ReactFlow
      nodes={visibleGraph.nodes}
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
        onConnect(connection);
      }}
      onReconnect={!canEditLayout ? undefined : reconnectPath}
      onNodeDragStart={!canEditLayout ? undefined : (_, node) => { checkpoint(); updateDragGuide(node); }}
      onNodeDrag={!canEditLayout ? undefined : (_, node) => updateDragGuide(node)}
      onNodeDragStop={!canEditLayout ? undefined : (_, node) => {
        setDragGuide(null);
        const indicator = parseIndicatorNodeId(node.id);
        if (!indicator) return;
        const target = nearestConstructForIndicator(node, indicator.constructId);
        if (target) assignIndicator(target.id, indicator.indicator);
        else moveIndicator(indicator.constructId, indicator.indicator, node.position);
      }}
      onNodeClick={(event, node) => {
        const indicator = parseIndicatorNodeId(node.id);
        if (indicator) selectIndicatorForToolbar(indicator.constructId, indicator.indicator);
        else {
          chooseConstruct(node.id, { x: event.clientX, y: event.clientY });
        }
      }}
      onEdgeClick={(_, edge) => setSelectedEdge(edge.id)}
      onNodeContextMenu={(event, node) => {
        event.preventDefault();
        if (!canEditLayout) return;
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
          addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
          return;
        }
        if (diagramTool === "indicator" || diagramTool === "residual" || diagramTool === "caption") return;
        if (event.detail !== 2) return;
        addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
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
            assignIndicators(targetConstructId, indicators);
            return;
          }
          preserveViewportForDrop.current = true;
          addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }), indicators);
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
      nodesDraggable={canEditLayout && diagramTool !== "pan"}
      nodesConnectable={canEditLayout}
      edgesReconnectable={canEditLayout}
      deleteKeyCode={null}
    >
      {diagramLayout.showGrid && !resultDiagramMode ? <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#dbe1e4" /> : null}
      <Controls showInteractive={false} />
    </ReactFlow>
  </div>;
}
