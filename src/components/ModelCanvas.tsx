import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  type EdgeChange,
  type EdgeTypes,
  type Node,
  type NodeChange,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import { AlignCenterHorizontal, AlignCenterVertical, AlignHorizontalSpaceBetween, AlignStartHorizontal, AlignStartVertical, AlignVerticalSpaceBetween, ArrowLeftRight, CircleHelp, Columns3, Copy, Focus, GitBranch, Hand, Link2, MousePointer2, Plus, Redo2, Trash2, Undo2 } from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { DragEvent } from "react";
import { analysisReadiness } from "../domain/analysisReadiness";
import { buildDiagramGraph, isIndicatorNodeId, parseIndicatorNodeId } from "../domain/diagramGraph";
import { SEM_SIZES } from "../domain/semGeometry";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";
import type { ConstructData, DiagramToolMode, IndicatorSide } from "../types";
import { ConstructNode } from "./ConstructNode";
import { IndicatorNode } from "./IndicatorNode";
import { LatentNode } from "./LatentNode";
import { SemEdge } from "./SemEdge";

const nodeTypes: NodeTypes = { construct: memo(ConstructNode), latent: memo(LatentNode), indicator: memo(IndicatorNode) };
const edgeTypes: EdgeTypes = { semEdge: SemEdge };
const SNAP_SIZE = 10;
const ALIGN_THRESHOLD = 8;
const smartplsNodeSize = { width: SEM_SIZES.smartplsLatent.width, height: SEM_SIZES.smartplsLatent.height };
const compactNodeSize = { width: 170, height: 118 };

const isEditingText = (target: EventTarget | null) => {
  const element = target as HTMLElement | null;
  return element?.matches("input, textarea, select, [contenteditable='true']") ?? false;
};

export function ModelCanvas() {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const dataset = useWorkspace((state) => state.dataset);
  const runs = useWorkspace((state) => state.runs);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramTool = useWorkspace((state) => state.diagramTool);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
  const largeModelViewState = useWorkspace((state) => state.largeModelViewState);
  const pastCount = useWorkspace((state) => state.past.length);
  const futureCount = useWorkspace((state) => state.future.length);
  const onNodesChange = useWorkspace((state) => state.onNodesChange);
  const onEdgesChange = useWorkspace((state) => state.onEdgesChange);
  const onConnect = useWorkspace((state) => state.onConnect);
  const reconnectPath = useWorkspace((state) => state.reconnectPath);
  const addPath = useWorkspace((state) => state.addPath);
  const addCovariance = useWorkspace((state) => state.addCovariance);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setSelectedResultRun = useWorkspace((state) => state.setSelectedResultRun);
  const setView = useWorkspace((state) => state.setView);
  const setDiagramMode = useWorkspace((state) => state.setDiagramMode);
  const setDiagramTool = useWorkspace((state) => state.setDiagramTool);
  const setDiagramOverlaySettings = useWorkspace((state) => state.setDiagramOverlaySettings);
  const setDiagramTheme = useWorkspace((state) => state.setDiagramTheme);
  const setDiagramGridVisible = useWorkspace((state) => state.setDiagramGridVisible);
  const setDiagramLayoutLocked = useWorkspace((state) => state.setDiagramLayoutLocked);
  const setLargeModelViewState = useWorkspace((state) => state.setLargeModelViewState);
  const checkpoint = useWorkspace((state) => state.checkpoint);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const duplicateSelected = useWorkspace((state) => state.duplicateSelected);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const reverseSelectedPath = useWorkspace((state) => state.reverseSelectedPath);
  const setSelectedPathRouting = useWorkspace((state) => state.setSelectedPathRouting);
  const setPathRouting = useWorkspace((state) => state.setPathRouting);
  const alignSelectedConstructs = useWorkspace((state) => state.alignSelectedConstructs);
  const distributeSelectedConstructs = useWorkspace((state) => state.distributeSelectedConstructs);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const moveIndicator = useWorkspace((state) => state.moveIndicator);
  const setIndicatorSide = useWorkspace((state) => state.setIndicatorSide);
  const setConstructIndicatorSide = useWorkspace((state) => state.setConstructIndicatorSide);
  const toggleConstructPinned = useWorkspace((state) => state.toggleConstructPinned);
  const resetIndicatorLayout = useWorkspace((state) => state.resetIndicatorLayout);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const assignIndicators = useWorkspace((state) => state.assignIndicators);
  const unassignIndicator = useWorkspace((state) => state.unassignIndicator);
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const updateEdge = useWorkspace((state) => state.updateEdge);
  const nudgeEdgeLabel = useWorkspace((state) => state.nudgeEdgeLabel);
  const resetEdgeLabel = useWorkspace((state) => state.resetEdgeLabel);
  const resetAllEdgeLabels = useWorkspace((state) => state.resetAllEdgeLabels);
  const analysisSettings = useWorkspace((state) => state.analysisSettings);
  const undo = useWorkspace((state) => state.undo);
  const redo = useWorkspace((state) => state.redo);
  const [flow, setFlow] = useState<ReactFlowInstance | null>(null);
  const previousNodeCount = useRef(nodes.length);
  const preserveViewportForDrop = useRef(false);
  const [pathSource, setPathSource] = useState<string | null>(null);
  const [showHelp, setShowHelp] = useState(false);
  const [dropHint, setDropHint] = useState<null | { count: number; x: number; y: number; targetConstructId?: string | null }>(null);
  const [dragGuide, setDragGuide] = useState<null | { vertical?: number; horizontal?: number; x: number; y: number; label: string }>(null);
  const [actionFeedback, setActionFeedback] = useState<null | { message: string; x?: number; y?: number }>(null);
  const [draggingVariableCount, setDraggingVariableCount] = useState(0);
  const [hoverDropTargetId, setHoverDropTargetId] = useState<string | null>(null);
  const [selectedIndicator, setSelectedIndicator] = useState<null | { constructId: string; indicator: string }>(null);
  const [openToolbarMenu, setOpenToolbarMenu] = useState<null | "arrange" | "view" | "results">(null);
  const [contextMenu, setContextMenu] = useState<null | { kind: "canvas"; x: number; y: number } | { kind: "construct"; id: string; x: number; y: number } | { kind: "indicator"; constructId: string; indicator: string; x: number; y: number } | { kind: "edge"; id: string; x: number; y: number }>(null);
  useEffect(() => {
    if (selectedNodeId || selectedEdgeId) setSelectedIndicator(null);
  }, [selectedEdgeId, selectedNodeId]);
  const resultRuns = useMemo(() => runs.filter((run) => run.status === "completed" && run.result), [runs]);
  const selectedResultRun = useMemo(() => resultRuns.find((run) => run.id === selectedResultRunId), [resultRuns, selectedResultRunId]);
  const graph = useMemo(() => buildDiagramGraph(nodes, edges, diagramMode, diagramOverlaySettings.mode, selectedResultRun, { layout: diagramLayout, layoutSource: diagramMode === "publication" ? "current_canvas" : undefined }), [diagramLayout, diagramMode, diagramOverlaySettings.mode, edges, nodes, selectedResultRun]);
  const selectedConstructCount = useMemo(() => new Set([...nodes.filter((node) => node.selected).map((node) => node.id), ...(selectedNodeId ? [selectedNodeId] : [])]).size, [nodes, selectedNodeId]);
  const selectedEdge = useMemo(() => edges.find((edge) => edge.id === selectedEdgeId), [edges, selectedEdgeId]);
  const selectedConstruct = useMemo(() => selectedNodeId ? nodes.find((node) => node.id === selectedNodeId) ?? null : null, [nodes, selectedNodeId]);
  const selectedIndicatorOwner = useMemo(() => selectedIndicator ? nodes.find((node) => node.id === selectedIndicator.constructId) ?? null : null, [nodes, selectedIndicator]);
  const validSelectedIndicator = selectedIndicator && selectedIndicatorOwner?.data.indicators.includes(selectedIndicator.indicator) ? selectedIndicator : null;
  const selectedRouteValue = useMemo(() => {
    const layout = selectedEdgeId ? diagramLayout.edgeLayouts[selectedEdgeId] : null;
    if (!layout?.pinned) return "straight";
    return layout.routing === "orthogonal" ? "smoothstep" : layout.routing === "curved" ? "default" : "straight";
  }, [diagramLayout.edgeLayouts, selectedEdgeId]);
  const resultDiagramMode = diagramMode === "smartpls_result" || diagramMode === "publication";
  const paperStyleCanvas = diagramMode === "sem" || diagramMode === "publication" || diagramMode === "smartpls_result";
  const layoutLocked = diagramLayout.layoutLocked && !resultDiagramMode;
  const canEditLayout = !resultDiagramMode && !layoutLocked;
  const readiness = useMemo(() => analysisReadiness({ dataset, nodes, edges, settings: analysisSettings, nativeDesktop: isNativeDesktop() }), [analysisSettings, dataset, edges, nodes]);
  const visibleGraph = useMemo(() => {
    if (!largeModelViewState.isolatedConstructId || largeModelViewState.neighborhoodMode === "off") return graph;
    const focusId = largeModelViewState.isolatedConstructId;
    const adjacent = new Set<string>([focusId]);
    graph.edges.forEach((edge) => {
      if (edge.source === focusId) adjacent.add(edge.target);
      if (edge.target === focusId) adjacent.add(edge.source);
    });
    const visibleNodeIds = new Set<string>();
    graph.nodes.forEach((node) => {
      const indicator = parseIndicatorNodeId(node.id);
      if (adjacent.has(node.id) || (indicator && adjacent.has(indicator.constructId))) visibleNodeIds.add(node.id);
    });
    return {
      ...graph,
      nodes: graph.nodes.filter((node) => visibleNodeIds.has(node.id)),
      edges: graph.edges.filter((edge) => visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target)),
    };
  }, [graph, largeModelViewState.isolatedConstructId, largeModelViewState.neighborhoodMode]);
  const overlayStatus = graph.diagnostic
    ? { tone: "warning", label: "Overlay blocked", detail: graph.diagnostic }
    : selectedResultRun
      ? { tone: "ready", label: "Result overlay active", detail: `${selectedResultRun.name} supplies loadings, paths, and R² where available.` }
      : { tone: "idle", label: "Model-only diagram", detail: "Run or select a compatible result to show loadings, path coefficients, and R²." };
  const nextAction = readiness.blockers.find((item) => item.actionView && item.actionView !== "models")
    ?? (readiness.canRun && !selectedResultRun ? { actionLabel: "Open run checklist", actionView: "run" as const, detail: "Model is structurally ready. Review settings and launch the selected method." } : null)
    ?? (!readiness.canRun ? { actionLabel: "Open run checklist", actionView: "run" as const, detail: readiness.blockers[0]?.detail ?? readiness.summary } : null);
  const arrangeModel = (direction: "horizontal" | "vertical" | "smartpls") => {
    autoLayout(direction);
    window.setTimeout(() => { void flow?.fitView({ padding: 0.2, duration: 220 }); }, 0);
  };
  const disabledActionReason = resultDiagramMode
    ? "Result and publication views are locked to protect saved results. Switch to Edit model to move, delete, reconnect, or assign diagram objects."
    : layoutLocked
      ? "Layout lock is on. Unlock layout in View to move constructs, reroute paths, or reposition indicators."
    : selectedEdgeId && selectedEdge?.data?.role === "covariance"
      ? "Covariance display arcs cannot be reversed as structural paths. Use route, label, reset, or delete actions from the edge context menu."
      : selectedEdgeId && !selectedEdge
        ? "The selected edge is not available in the current model."
        : selectedNodeId
          ? null
          : "Select a construct, indicator, or path to enable object-specific editing actions.";
  const setMode = (mode: typeof diagramMode) => {
    setDiagramMode(mode);
    setOpenToolbarMenu(null);
    if (mode === "smartpls_result" || mode === "publication") {
      setDiagramOverlaySettings({ mode: selectedResultRunId ? "paths_r2" : "model" });
      window.setTimeout(() => { void flow?.fitView({ padding: 0.16, duration: 220 }); }, 0);
    }
  };
  const fitSelectedObject = () => {
    if (!flow) return;
    const selectedGraphNode = selectedNodeId ? graph.nodes.find((node) => node.id === selectedNodeId) : null;
    if (selectedGraphNode) {
      const size = selectedGraphNode.type === "latent" ? smartplsNodeSize : compactNodeSize;
      void flow.setCenter(selectedGraphNode.position.x + size.width / 2, selectedGraphNode.position.y + size.height / 2, { zoom: Math.max(0.85, flow.getZoom()), duration: 220 });
      return;
    }
    if (selectedEdgeId) {
      window.dispatchEvent(new CustomEvent("quickpls:focus-edge", { detail: { id: selectedEdgeId } }));
      return;
    }
    void flow.fitView({ padding: 0.22, duration: 220 });
  };
  const isolateSelectedObject = () => {
    if (!selectedNodeId) {
      setActionFeedback({ message: "Select a construct first to isolate its local neighborhood." });
      return;
    }
    const constructId = parseIndicatorNodeId(selectedNodeId)?.constructId ?? selectedNodeId;
    setLargeModelViewState({ isolatedConstructId: constructId, neighborhoodMode: "selected" });
    window.setTimeout(() => { void flow?.fitView({ padding: 0.24, duration: 220 }); }, 0);
  };

  useEffect(() => {
    if (nodes.length > previousNodeCount.current) {
      if (preserveViewportForDrop.current) preserveViewportForDrop.current = false;
      else window.setTimeout(() => { void flow?.fitView({ padding: 0.16, duration: 220 }); }, 0);
    }
    previousNodeCount.current = nodes.length;
  }, [flow, nodes.length]);

  useEffect(() => {
    const centerNode = (id: string) => {
      const node = graph.nodes.find((candidate) => candidate.id === id);
      if (!node || !flow) return;
      const size = node.type === "latent" ? smartplsNodeSize : compactNodeSize;
      void flow.setCenter(node.position.x + size.width / 2, node.position.y + size.height / 2, { zoom: Math.max(0.75, flow.getZoom()), duration: 240 });
    };
    const centerEdge = (id: string) => {
      const edge = graph.edges.find((candidate) => candidate.id === id);
      const source = edge ? graph.nodes.find((node) => node.id === edge.source) : null;
      const target = edge ? graph.nodes.find((node) => node.id === edge.target) : null;
      if (!source || !target || !flow) return;
      void flow.setCenter((source.position.x + target.position.x) / 2 + smartplsNodeSize.width / 2, (source.position.y + target.position.y) / 2 + smartplsNodeSize.height / 2, { zoom: Math.max(0.7, flow.getZoom()), duration: 240 });
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
    const handleKeyDown = (event: KeyboardEvent) => {
      if (isEditingText(event.target)) return;
      const command = event.ctrlKey || event.metaKey;
      if (command && event.key.toLowerCase() === "z") {
        event.preventDefault();
        event.shiftKey ? redo() : undo();
      } else if (command && event.key.toLowerCase() === "y") {
        event.preventDefault();
        redo();
      } else if (command && event.key.toLowerCase() === "d") {
        event.preventDefault();
        if (resultDiagramMode) return;
        duplicateSelected();
      } else if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        if (resultDiagramMode) return;
        removeSelection();
      } else if (event.key === "Escape") {
        setDiagramTool("select");
        setOpenToolbarMenu(null);
        setPathSource(null);
      } else if (event.key === "Enter") {
        if (resultDiagramMode) return;
        const selectedNode = nodes.find((node) => node.id === selectedNodeId);
        if (selectedNode) {
          event.preventDefault();
          const value = window.prompt("Construct name", selectedNode.data.label);
          if (value?.trim()) updateConstruct(selectedNode.id, { label: value.trim() });
          return;
        }
        const edge = edges.find((candidate) => candidate.id === selectedEdgeId);
        if (edge && !edge.id.startsWith("measurement::")) {
          event.preventDefault();
          const current = typeof edge.label === "string" ? edge.label : "";
          const value = window.prompt("Path label", current);
          if (value?.trim()) updateEdge(edge.id, { label: value.trim() });
        }
      } else if (resultDiagramMode) {
        return;
      } else if (event.key.toLowerCase() === "p") {
        setDiagramTool("path");
        setPathSource(null);
      } else if (event.key.toLowerCase() === "c") {
        setDiagramTool("covariance");
        setPathSource(null);
      } else if (event.key.toLowerCase() === "v") {
        setDiagramTool("select");
        setPathSource(null);
      } else if (event.key.toLowerCase() === "f") {
        void flow?.fitView({ padding: 0.22, duration: 220 });
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [duplicateSelected, edges, flow, nodes, redo, removeSelection, resultDiagramMode, selectedEdgeId, selectedNodeId, setDiagramTool, undo, updateConstruct, updateEdge]);

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
    setOpenToolbarMenu(null);
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
  const renameConstruct = (id: string) => {
    const node = nodes.find((candidate) => candidate.id === id);
    if (!node) return;
    const value = window.prompt("Construct name", node.data.label);
    if (value?.trim()) updateConstruct(id, { label: value.trim() });
  };
  const renameIndicator = (constructId: string, indicator: string) => {
    const node = nodes.find((candidate) => candidate.id === constructId);
    const value = window.prompt("Indicator label", indicator);
    if (!node || !value?.trim() || value.trim() === indicator) return;
    updateConstruct(constructId, { indicators: node.data.indicators.map((item) => item === indicator ? value.trim() : item) });
  };
  const setIndicatorSideFromMenu = (side: IndicatorSide) => {
    if (contextMenu?.kind !== "indicator") return;
    setIndicatorSide(contextMenu.constructId, contextMenu.indicator, side);
    setContextMenu(null);
  };
  const setConstructIndicatorSideFromMenu = (side: Exclude<IndicatorSide, "free">) => {
    if (contextMenu?.kind !== "construct") return;
    setConstructIndicatorSide(contextMenu.id, side);
    setContextMenu(null);
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
  const toggleToolbarMenu = (menu: "arrange" | "view" | "results") => setOpenToolbarMenu((current) => current === menu ? null : menu);
  const validateDiagram = () => {
    setOpenToolbarMenu(null);
    setActionFeedback({ message: readiness.canRun ? "Diagram validation passed for the selected method settings." : readiness.blockers[0]?.detail ?? readiness.summary });
  };
  const selectIndicatorForToolbar = (constructId: string, indicator: string) => {
    setSelectedIndicator({ constructId, indicator });
    setSelectedNode(constructId);
  };
  const clearSelectionForCanvas = () => {
    setContextMenu(null);
    setSelectedIndicator(null);
    setSelectedNode(null);
  };
  const renameSelectedPath = () => {
    if (!selectedEdge) return;
    const current = typeof selectedEdge.label === "string" ? selectedEdge.label : "";
    const value = window.prompt("Path label", current);
    if (value?.trim()) updateEdge(selectedEdge.id, { label: value.trim() });
  };
  const reassignSelectedIndicator = () => {
    if (!validSelectedIndicator) return;
    const candidates = nodes.filter((node) => node.id !== validSelectedIndicator.constructId);
    if (candidates.length === 0) {
      setActionFeedback({ message: "Create another construct before reassigning this indicator." });
      return;
    }
    const options = candidates.map((node) => `${node.data.shortName}: ${node.data.label}`).join("\n");
    const value = window.prompt(`Reassign ${validSelectedIndicator.indicator} to construct short name:\n${options}`, candidates[0]?.data.shortName ?? "");
    const target = candidates.find((node) => node.data.shortName.toLowerCase() === value?.trim().toLowerCase() || node.data.label.toLowerCase() === value?.trim().toLowerCase());
    if (!target) {
      if (value?.trim()) setActionFeedback({ message: "No matching construct short name or label was found for reassignment." });
      return;
    }
    assignIndicator(target.id, validSelectedIndicator.indicator);
    setSelectedIndicator({ constructId: target.id, indicator: validSelectedIndicator.indicator });
  };
  const selectedConstructPinned = selectedConstruct ? Boolean(diagramLayout.constructLayouts[selectedConstruct.id]?.pinned) : false;
  const contextToolbar = resultDiagramMode ? null : validSelectedIndicator ? <div className="canvas-context-toolbar" role="toolbar" aria-label="Selected indicator actions">
    <strong>Indicator: {validSelectedIndicator.indicator}</strong>
    <button onClick={() => renameIndicator(validSelectedIndicator.constructId, validSelectedIndicator.indicator)}>Rename</button>
    <button onClick={reassignSelectedIndicator}>Reassign</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicator to the left side."} onClick={() => setIndicatorSide(validSelectedIndicator.constructId, validSelectedIndicator.indicator, "left")}>Left</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicator to the right side."} onClick={() => setIndicatorSide(validSelectedIndicator.constructId, validSelectedIndicator.indicator, "right")}>Right</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicator to the top side."} onClick={() => setIndicatorSide(validSelectedIndicator.constructId, validSelectedIndicator.indicator, "top")}>Top</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicator to the bottom side."} onClick={() => setIndicatorSide(validSelectedIndicator.constructId, validSelectedIndicator.indicator, "bottom")}>Bottom</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to reset indicator position." : "Reset manually positioned indicator."} onClick={() => resetIndicatorLayout(validSelectedIndicator.constructId, validSelectedIndicator.indicator)}>Reset position</button>
    <button className="danger" onClick={() => { unassignIndicator(validSelectedIndicator.constructId, validSelectedIndicator.indicator); setSelectedIndicator(null); }}>Unassign</button>
  </div> : selectedEdge ? <div className="canvas-context-toolbar" role="toolbar" aria-label={selectedEdge.data?.role === "covariance" ? "Selected covariance actions" : "Selected path actions"}>
    <strong>{selectedEdge.data?.role === "covariance" ? "Covariance" : "Path"}: {selectedEdge.source} → {selectedEdge.target}</strong>
    {selectedEdge.data?.role !== "covariance" ? <button onClick={reverseSelectedPath}><ArrowLeftRight size={13} /> Reverse</button> : <button disabled title="Covariance arcs have no structural direction to reverse.">Reverse</button>}
    {selectedEdge.data?.role !== "covariance" ? <>
      <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to change path routing." : "Use a direct shortest route."} className={selectedRouteValue === "straight" ? "active" : ""} onClick={() => setSelectedPathRouting("straight")}>Straight</button>
      <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to change path routing." : "Use a curved route."} className={selectedRouteValue === "default" ? "active" : ""} onClick={() => setSelectedPathRouting("default")}>Curved</button>
      <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to change path routing." : "Use an orthogonal route."} className={selectedRouteValue === "smoothstep" ? "active" : ""} onClick={() => setSelectedPathRouting("smoothstep")}>Orthogonal</button>
    </> : <button onClick={() => resetEdgeLabel(selectedEdge.id)}>Reset arc label</button>}
    <button onClick={renameSelectedPath}>Rename label</button>
    <button onClick={() => resetEdgeLabel(selectedEdge.id)}>Reset label</button>
    {selectedEdge.data?.role !== "covariance" ? <button onClick={() => updateEdge(selectedEdge.id, { label: "Control", data: { ...selectedEdge.data, role: "control" } })}>Mark control</button> : null}
    <button className="danger" onClick={removeSelection}>Delete</button>
  </div> : selectedConstructCount >= 2 ? <div className="canvas-context-toolbar" role="toolbar" aria-label="Selected constructs alignment actions">
    <strong>{selectedConstructCount} constructs selected</strong>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to align constructs." : "Align selected constructs left."} onClick={() => alignSelectedConstructs("left")}><AlignStartVertical size={13} /> Left</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to align constructs." : "Align selected construct centers on X."} onClick={() => alignSelectedConstructs("centerX")}><AlignCenterVertical size={13} /> Center X</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to align constructs." : "Align selected constructs top."} onClick={() => alignSelectedConstructs("top")}><AlignStartHorizontal size={13} /> Top</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to align constructs." : "Align selected construct centers on Y."} onClick={() => alignSelectedConstructs("centerY")}><AlignCenterHorizontal size={13} /> Center Y</button>
    <button disabled={selectedConstructCount < 3 || layoutLocked} title={selectedConstructCount < 3 ? "Select at least three constructs to distribute." : "Unlock layout in View to distribute constructs."} onClick={() => distributeSelectedConstructs("horizontal")}><AlignHorizontalSpaceBetween size={13} /> Distribute H</button>
    <button disabled={selectedConstructCount < 3 || layoutLocked} title={selectedConstructCount < 3 ? "Select at least three constructs to distribute." : "Unlock layout in View to distribute constructs."} onClick={() => distributeSelectedConstructs("vertical")}><AlignVerticalSpaceBetween size={13} /> Distribute V</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to tidy selected constructs." : "Tidy selected constructs."} onClick={() => arrangeModel("smartpls")}>Tidy selection</button>
  </div> : selectedConstruct ? <div className="canvas-context-toolbar" role="toolbar" aria-label="Selected construct actions">
    <strong>Construct: {selectedConstruct.data.label}</strong>
    <button onClick={() => renameConstruct(selectedConstruct.id)}>Rename</button>
    <button onClick={duplicateSelected}><Copy size={13} /> Duplicate</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to auto-place indicators." : "Auto-place this construct's indicators."} onClick={() => resetIndicatorLayout(selectedConstruct.id)}>Auto indicators</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicators left."} onClick={() => setConstructIndicatorSide(selectedConstruct.id, "left")}>Indicators left</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicators right."} onClick={() => setConstructIndicatorSide(selectedConstruct.id, "right")}>Right</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicators top."} onClick={() => setConstructIndicatorSide(selectedConstruct.id, "top")}>Top</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to move indicators." : "Move indicators bottom."} onClick={() => setConstructIndicatorSide(selectedConstruct.id, "bottom")}>Bottom</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to reset indicator layout." : "Reset this construct's indicator layout."} onClick={() => resetIndicatorLayout(selectedConstruct.id)}>Reset indicator layout</button>
    <button disabled={layoutLocked} title={layoutLocked ? "Unlock layout in View to tidy selected construct." : "Tidy selected construct."} onClick={() => arrangeModel("smartpls")}>Tidy selected</button>
    <button onClick={() => toggleConstructPinned(selectedConstruct.id)}>{selectedConstructPinned ? "Unpin" : "Pin"}</button>
    <button className="danger" onClick={removeSelection}><Trash2 size={13} /> Delete</button>
  </div> : null;

  return <main className={`model-canvas theme-${diagramLayout.diagramTheme}${paperStyleCanvas ? " smartpls-result-canvas" : ""}${resultDiagramMode ? " locked-result-canvas" : ""}${layoutLocked ? " layout-locked-canvas" : ""}${showDropCue ? " can-drop-variables" : ""}${largeModelViewState.isolatedConstructId ? " isolating-neighborhood" : ""}`}>
    <div className="canvas-toolbar redesigned" role="toolbar" aria-label="Model editing tools">
      <div className="canvas-toolbar-primary">
        <div className="canvas-tool-group">
          <button aria-label="Undo" title="Undo (Ctrl+Z)" disabled={pastCount === 0} onClick={undo}><Undo2 size={15} /></button>
          <button aria-label="Redo" title="Redo (Ctrl+Y)" disabled={futureCount === 0} onClick={redo}><Redo2 size={15} /></button>
        </div>
        <div className="canvas-tool-group segmented">
          <button aria-label="Select and move diagram items" className={diagramTool === "select" ? "active" : ""} title={layoutLocked ? "Layout is locked. Select is available for inspection; unlock layout to move items." : "Select and move diagram items (V)"} disabled={resultDiagramMode} onClick={() => selectTool("select")}><MousePointer2 size={15} /><span>Select</span></button>
          <button aria-label="Pan canvas" className={diagramTool === "pan" ? "active" : ""} title="Pan canvas" onClick={() => selectTool("pan")}><Hand size={15} /><span>Pan</span></button>
        </div>
        <div className="canvas-tool-group">
          <button title={layoutLocked ? "Unlock layout in View to add constructs." : "Add construct"} disabled={!canEditLayout} onClick={() => addConstruct()}><Plus size={15} /><span>Construct</span></button>
          <button className={diagramTool === "path" ? "active" : ""} title={layoutLocked ? "Unlock layout in View to draw paths." : "Draw structural path (P)"} disabled={!canEditLayout} onClick={() => selectTool("path")}><GitBranch size={15} /><span>Path</span></button>
          <button className={diagramTool === "covariance" ? "active" : ""} title={layoutLocked ? "Unlock layout in View to draw covariances." : "Draw covariance display arc (C)"} disabled={!canEditLayout} onClick={() => selectTool("covariance")}><Link2 size={15} /><span>Cov</span></button>
        </div>
        <div className="canvas-tool-group menu-group">
          <button aria-haspopup="menu" aria-expanded={openToolbarMenu === "arrange"} title="Arrange model" onClick={() => toggleToolbarMenu("arrange")}><Columns3 size={15} /><span>Arrange</span></button>
          {openToolbarMenu === "arrange" ? <div className="canvas-dropdown-menu" role="menu" aria-label="Arrange model options">
            <button role="menuitem" disabled={!canEditLayout} title={layoutLocked ? "Unlock layout in View to arrange the model." : undefined} onClick={() => { arrangeModel("smartpls"); setOpenToolbarMenu(null); }}>Arrange like SmartPLS</button>
            <button role="menuitem" disabled={!canEditLayout} title={layoutLocked ? "Unlock layout in View to arrange the model." : undefined} onClick={() => { arrangeModel("horizontal"); setOpenToolbarMenu(null); }}>Left to right</button>
            <button role="menuitem" disabled={!canEditLayout} title={layoutLocked ? "Unlock layout in View to arrange the model." : undefined} onClick={() => { arrangeModel("vertical"); setOpenToolbarMenu(null); }}>Top to bottom</button>
            <button role="menuitem" disabled title="Dedicated CFA preset will use the existing SmartPLS layout engine in a later pass.">CFA measurement preset</button>
            <button role="menuitem" disabled title="Dedicated mediation preset will use the existing SmartPLS layout engine in a later pass.">Mediation preset</button>
            <button role="menuitem" disabled title="Large-model compaction is available through Compact view.">Large model preset</button>
          </div> : null}
          <button aria-label="Fit model to view" title="Fit model to view (F)" onClick={() => { void flow?.fitView({ padding: 0.22, duration: 220 }); }}><Focus size={15} /><span>Fit</span></button>
          <button aria-label="Validate diagram" title="Validate diagram readiness" onClick={validateDiagram}><span>Validate</span></button>
        </div>
        <div className="canvas-tool-group result-tools menu-group">
          <button aria-haspopup="menu" aria-expanded={openToolbarMenu === "view"} title="View and diagram mode" onClick={() => toggleToolbarMenu("view")}><span>View</span></button>
          {openToolbarMenu === "view" ? <div className="canvas-dropdown-menu view-menu" role="menu" aria-label="View options">
            <label>Diagram mode<select aria-label="Diagram mode" value={diagramMode} onChange={(event) => setMode(event.target.value as typeof diagramMode)}>
              <option value="sem">Edit model</option>
              <option value="smartpls_result">Result diagram</option>
              <option value="compact">Compact</option>
              <option value="publication">Publication preview</option>
            </select></label>
            <button role="menuitem" className={diagramLayout.diagramTheme === "smartpls_like" ? "active" : ""} onClick={() => setDiagramTheme("smartpls_like")}>SmartPLS-like theme</button>
            <button role="menuitem" className={diagramLayout.diagramTheme === "academic_grayscale" ? "active" : ""} onClick={() => setDiagramTheme("academic_grayscale")}>Academic grayscale</button>
            <button role="menuitem" className={diagramLayout.diagramTheme === "quickpls_color" ? "active" : ""} onClick={() => setDiagramTheme("quickpls_color")}>QuickPLS color</button>
            <button role="menuitem" className={diagramLayout.diagramTheme === "journal_mono" ? "active" : ""} onClick={() => setDiagramTheme("journal_mono")}>Journal mono</button>
            <button role="menuitem" className={diagramLayout.diagramTheme === "high_contrast" ? "active" : ""} onClick={() => setDiagramTheme("high_contrast")}>High contrast</button>
            <button role="menuitem" className={largeModelViewState.indicatorsCollapsed ? "active" : ""} onClick={() => {
              const nextCollapsed = !largeModelViewState.indicatorsCollapsed;
              setLargeModelViewState({ indicatorsCollapsed: nextCollapsed });
              setMode(nextCollapsed ? "compact" : "sem");
            }}>{largeModelViewState.indicatorsCollapsed ? "Show measurement indicators" : "Collapse measurement indicators"}</button>
            <button role="menuitem" disabled={!selectedNodeId} title={!selectedNodeId ? "Select a construct to isolate its neighborhood." : "Show the selected construct and directly connected constructs."} className={largeModelViewState.neighborhoodMode === "selected" ? "active" : ""} onClick={isolateSelectedObject}>Isolate selected neighborhood</button>
            <button role="menuitem" disabled={!largeModelViewState.isolatedConstructId} title={!largeModelViewState.isolatedConstructId ? "No isolated neighborhood is active." : "Return to the full diagram."} onClick={() => {
              setLargeModelViewState({ isolatedConstructId: null, neighborhoodMode: "off" });
              window.setTimeout(() => { void flow?.fitView({ padding: 0.22, duration: 220 }); }, 0);
            }}>Clear isolation</button>
            <button role="menuitem" title={selectedNodeId || selectedEdgeId ? "Fit the selected diagram object." : "Select a construct or path to fit only that object."} onClick={fitSelectedObject}>Fit selected</button>
            <button role="menuitem" className={diagramLayout.layoutLocked ? "active" : ""} onClick={() => setDiagramLayoutLocked(!diagramLayout.layoutLocked)}>{diagramLayout.layoutLocked ? "Unlock layout" : "Lock layout"}</button>
            <button role="menuitem" className={diagramLayout.showGrid ? "active" : ""} onClick={() => setDiagramGridVisible(!diagramLayout.showGrid)}>{diagramLayout.showGrid ? "Hide grid" : "Show grid"}</button>
          </div> : null}
          <button aria-haspopup="menu" aria-expanded={openToolbarMenu === "results"} title={selectedResultRunId ? "Result overlay controls" : "no compatible result selected. Run PLS or select a completed compatible result to enable overlays."} onClick={() => toggleToolbarMenu("results")}><span>Results</span></button>
          {openToolbarMenu === "results" ? <div className="canvas-dropdown-menu results-menu" role="menu" aria-label="Result overlay options">
            <label>Run<select aria-label="Diagram result run" value={selectedResultRunId ?? ""} disabled={resultRuns.length === 0} onChange={(event) => setSelectedResultRun(event.target.value || null)}>
              <option value="">No diagram estimates</option>
              {resultRuns.map((run) => <option key={run.id} value={run.id}>{run.name} | {new Date(run.createdAt).toLocaleString()}</option>)}
            </select></label>
            <label>Overlay<select aria-label="Diagram result overlay" value={diagramOverlaySettings.mode} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ mode: event.target.value as typeof diagramOverlaySettings.mode })}>
              <option value="model">Model only</option>
              <option value="loadings">Loadings / weights</option>
              <option value="paths_r2">Paths + R²</option>
              <option value="significance">Significance</option>
              <option value="quality">Reliability warnings</option>
              <option value="cbsem_standardized">CB-SEM standardized</option>
              <option value="cbsem_residuals">CB-SEM residuals</option>
              <option value="modification_indices">Modification indices</option>
            </select></label>
            <label>Precision<select aria-label="Diagram overlay precision" value={diagramOverlaySettings.precision} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ precision: Number(event.target.value) })}>
              {[2, 3, 4, 5, 6].map((value) => <option key={value} value={value}>{value} decimals</option>)}
            </select></label>
            <label><input type="checkbox" checked={diagramOverlaySettings.showLoadings} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ showLoadings: event.target.checked })} /> Loadings</label>
            <label><input type="checkbox" checked={diagramOverlaySettings.showPathCoefficients} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ showPathCoefficients: event.target.checked })} /> Path coefficients</label>
            <label><input type="checkbox" checked={diagramOverlaySettings.showRSquared} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ showRSquared: event.target.checked })} /> R²</label>
            <label><input type="checkbox" checked={diagramOverlaySettings.showPValues || diagramOverlaySettings.showTValues} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ showPValues: event.target.checked, showTValues: event.target.checked })} /> Significance</label>
          </div> : null}
          <button aria-label="Show diagram legend" title="Diagram legend" onClick={() => setShowHelp((value) => !value)}><CircleHelp size={15} /></button>
        </div>
      </div>
      {contextToolbar}
      {resultDiagramMode ? <div className="canvas-tool-status warning">Result view is locked. Switch to Edit model to move, delete, or reconnect diagram objects.</div> : null}
      {(diagramTool === "path" || diagramTool === "covariance") && <div className="canvas-tool-status">{pathSource ? `Choose ${diagramTool === "path" ? "outcome construct" : "second construct"}` : `Choose ${diagramTool === "path" ? "predictor construct" : "first construct"}`}</div>}
    </div>
    {disabledActionReason ? <div className="canvas-disabled-action-reason" role="status">{disabledActionReason}</div> : null}
    {actionFeedback ? <div
      className={`canvas-action-feedback${actionFeedback.x !== undefined && actionFeedback.y !== undefined ? " local" : ""}`}
      style={actionFeedback.x !== undefined && actionFeedback.y !== undefined ? { left: actionFeedback.x + 12, top: actionFeedback.y + 12 } : undefined}
      role="status"
      aria-live="polite"
    >{actionFeedback.message}</div> : null}
    <div className={`canvas-overlay-status ${overlayStatus.tone}`} role="status" aria-live="polite">
      <strong>{overlayStatus.label}</strong>
      <span>{overlayStatus.detail}</span>
    </div>
    {nextAction ? <div className="canvas-next-action" aria-label="Recommended next workflow action">
      <strong title={nextAction.detail}>Next step</strong>
      <button type="button" onClick={() => setView(nextAction.actionView!)}>{nextAction.actionLabel}</button>
    </div> : null}
    {showDropCue && !dropHint ? <div className="canvas-drop-guide" aria-live="polite">
      <strong>Drop on canvas</strong>
      <span>Create a construct, or drop onto an oval to assign indicators.</span>
    </div> : null}
    {dropHint ? <div className="canvas-drop-hint" style={{ left: dropHint.x + 14, top: dropHint.y + 14 }} aria-live="polite">
      <strong>{dropTargetLabel ? `Drop on ${dropTargetLabel}` : "Drop to create construct"}</strong>
      <span>{dropHint.count} variable{dropHint.count === 1 ? "" : "s"} will {dropTargetLabel ? "be assigned as indicator" : "become indicator"}{dropHint.count === 1 ? "" : "s"}</span>
    </div> : null}
    {showHelp && <div className="diagram-help" role="dialog" aria-label="Diagram legend">
      <strong>Diagram legend</strong>
      <span><i className="legend-latent" />Latent construct</span>
      <span><i className="legend-indicator" />Observed indicator</span>
      <span><i className="legend-path" />Structural path</span>
      <span><i className="legend-covariance" />Covariance display</span>
      <span>Shortcuts: P path, C covariance, V select, F fit view, Esc cancel. Right-click opens object actions.</span>
    </div>}
    {dragGuide?.vertical !== undefined ? <div className="canvas-alignment-guide vertical" style={{ left: dragGuide.vertical }} /> : null}
    {dragGuide?.horizontal !== undefined ? <div className="canvas-alignment-guide horizontal" style={{ top: dragGuide.horizontal }} /> : null}
    {dragGuide ? <div className="canvas-snap-hint" style={{ left: dragGuide.x + 12, top: dragGuide.y + 12 }}>{dragGuide.label}</div> : null}
    {contextMenu ? <div className="diagram-context-menu" style={{ left: contextMenu.x, top: contextMenu.y }}>
      {contextMenu.kind === "canvas" ? <>
        <button onClick={() => { if (flow) addConstruct(flow.screenToFlowPosition({ x: contextMenu.x, y: contextMenu.y })); setContextMenu(null); }}>Add latent construct</button>
        <button onClick={() => { arrangeModel("smartpls"); setContextMenu(null); }}>Arrange like SmartPLS</button>
        <button onClick={() => { resetAllEdgeLabels(); setContextMenu(null); }}>Tidy labels</button>
        <button onClick={() => { void flow?.fitView({ padding: 0.22, duration: 220 }); setContextMenu(null); }}>Fit view</button>
      </> : contextMenu.kind === "construct" ? <>
        <button onClick={() => { renameConstruct(contextMenu.id); setContextMenu(null); }}>Rename construct</button>
        <button onClick={() => { updateConstruct(contextMenu.id, { mode: nodes.find((node) => node.id === contextMenu.id)?.data.mode === "reflective" ? "formative" : "reflective" }); setContextMenu(null); }}>Invert reflective/formative</button>
        <button onClick={() => setConstructIndicatorSideFromMenu("left")}>Place all indicators left</button>
        <button onClick={() => setConstructIndicatorSideFromMenu("right")}>Place all indicators right</button>
        <button onClick={() => setConstructIndicatorSideFromMenu("top")}>Place all indicators top</button>
        <button onClick={() => setConstructIndicatorSideFromMenu("bottom")}>Place all indicators bottom</button>
        <button onClick={() => { resetIndicatorLayout(contextMenu.id); setContextMenu(null); }}>Auto-place indicators</button>
        <button onClick={() => { resetIndicatorLayout(contextMenu.id); resetAllEdgeLabels(); setContextMenu(null); }}>Tidy selected construct</button>
        <button onClick={() => { resetIndicatorLayout(contextMenu.id); setContextMenu(null); }}>Reset indicator layout</button>
        <button onClick={() => { setSelectedNode(contextMenu.id); duplicateSelected(); setContextMenu(null); }}>Duplicate</button>
        <button className="danger" onClick={() => { setSelectedNode(contextMenu.id); removeSelection(); setContextMenu(null); }}>Delete</button>
      </> : contextMenu.kind === "indicator" ? <>
        <button onClick={() => { renameIndicator(contextMenu.constructId, contextMenu.indicator); setContextMenu(null); }}>Rename indicator label</button>
        <button onClick={() => setIndicatorSideFromMenu("left")}>Move left</button>
        <button onClick={() => setIndicatorSideFromMenu("right")}>Move right</button>
        <button onClick={() => setIndicatorSideFromMenu("top")}>Move top</button>
        <button onClick={() => setIndicatorSideFromMenu("bottom")}>Move bottom</button>
        <button onClick={() => { resetIndicatorLayout(contextMenu.constructId, contextMenu.indicator); setContextMenu(null); }}>Reset position</button>
        <button className="danger" onClick={() => { unassignIndicator(contextMenu.constructId, contextMenu.indicator); setContextMenu(null); }}>Unassign</button>
      </> : <>
        <button onClick={() => { setSelectedEdge(contextMenu.id); reverseSelectedPath(); setContextMenu(null); }}>Reverse path</button>
        <button onClick={() => { setPathRouting(contextMenu.id, "straight"); setContextMenu(null); }}>Straight route</button>
        <button onClick={() => { setPathRouting(contextMenu.id, "smoothstep"); setContextMenu(null); }}>Orthogonal route</button>
        <button onClick={() => { setPathRouting(contextMenu.id, "default"); setContextMenu(null); }}>Curved route</button>
        <button onClick={() => { updateEdge(contextMenu.id, { label: "Control", data: { role: "control" } }); setContextMenu(null); }}>Mark control</button>
        <button onClick={() => { const edge = edges.find((item) => item.id === contextMenu.id); if (edge) updateEdge(contextMenu.id, { label: "Covariance", data: { ...edge.data, role: "covariance" } }); setContextMenu(null); }}>Convert to covariance display</button>
        <button onClick={() => nudgeEdgeLabel(contextMenu.id, { x: 0, y: -16 })}>Move label up</button>
        <button onClick={() => nudgeEdgeLabel(contextMenu.id, { x: 0, y: 16 })}>Move label down</button>
        <button onClick={() => nudgeEdgeLabel(contextMenu.id, { x: -18, y: 0 })}>Move label left</button>
        <button onClick={() => nudgeEdgeLabel(contextMenu.id, { x: 18, y: 0 })}>Move label right</button>
        <button onClick={() => { resetEdgeLabel(contextMenu.id); setContextMenu(null); }}>Reset label</button>
        <button className="danger" onClick={() => { setSelectedEdge(contextMenu.id); removeSelection(); setContextMenu(null); }}>Delete</button>
      </>}
    </div> : null}
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
          setSelectedIndicator(null);
          chooseConstruct(node.id, { x: event.clientX, y: event.clientY });
        }
      }}
      onEdgeClick={(_, edge) => { setSelectedIndicator(null); setSelectedEdge(edge.id); }}
      onNodeContextMenu={(event, node) => {
        event.preventDefault();
        if (!canEditLayout) return;
        const indicator = parseIndicatorNodeId(node.id);
        if (indicator) selectIndicatorForToolbar(indicator.constructId, indicator.indicator);
        else {
          setSelectedIndicator(null);
          setSelectedNode(node.id);
        }
        setContextMenu(indicator ? { kind: "indicator", ...indicator, x: event.clientX, y: event.clientY } : { kind: "construct", id: node.id, x: event.clientX, y: event.clientY });
      }}
      onEdgeContextMenu={(event, edge) => {
        event.preventDefault();
        if (!canEditLayout || edge.id.startsWith("measurement::")) return;
        setSelectedIndicator(null);
        setSelectedEdge(edge.id);
        setContextMenu({ kind: "edge", id: edge.id, x: event.clientX, y: event.clientY });
      }}
      onPaneContextMenu={(event) => {
        event.preventDefault();
        if (!canEditLayout) return;
        setContextMenu({ kind: "canvas", x: event.clientX, y: event.clientY });
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
      {diagramLayout.showGrid && !resultDiagramMode ? <MiniMap pannable zoomable nodeColor={(node) => node.type === "indicator" ? "#f8dd8a" : "#c6eef0"} maskColor="rgba(246,248,249,.7)" /> : null}
    </ReactFlow>
  </main>;
}
