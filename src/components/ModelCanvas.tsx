import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  type Edge,
  type Node,
  ReactFlow,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import { AlignCenterHorizontal, AlignCenterVertical, AlignHorizontalSpaceBetween, AlignStartHorizontal, AlignStartVertical, AlignVerticalSpaceBetween, ArrowLeftRight, Columns3, Copy, Eye, EyeOff, Focus, GitBranch, MousePointer2, Plus, Redo2, Rows3, Trash2, Undo2 } from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import { useWorkspace } from "../store";
import type { ConstructData } from "../types";
import { ConstructNode } from "./ConstructNode";

const nodeTypes: NodeTypes = { construct: memo(ConstructNode) };

const isEditingText = (target: EventTarget | null) => {
  const element = target as HTMLElement | null;
  return element?.matches("input, textarea, select, [contenteditable='true']") ?? false;
};

export function ModelCanvas() {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const runs = useWorkspace((state) => state.runs);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const pastCount = useWorkspace((state) => state.past.length);
  const futureCount = useWorkspace((state) => state.future.length);
  const onNodesChange = useWorkspace((state) => state.onNodesChange);
  const onEdgesChange = useWorkspace((state) => state.onEdgesChange);
  const onConnect = useWorkspace((state) => state.onConnect);
  const reconnectPath = useWorkspace((state) => state.reconnectPath);
  const addPath = useWorkspace((state) => state.addPath);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const setSelectedResultRun = useWorkspace((state) => state.setSelectedResultRun);
  const checkpoint = useWorkspace((state) => state.checkpoint);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const duplicateSelected = useWorkspace((state) => state.duplicateSelected);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const reverseSelectedPath = useWorkspace((state) => state.reverseSelectedPath);
  const setSelectedPathRouting = useWorkspace((state) => state.setSelectedPathRouting);
  const alignSelectedConstructs = useWorkspace((state) => state.alignSelectedConstructs);
  const distributeSelectedConstructs = useWorkspace((state) => state.distributeSelectedConstructs);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const undo = useWorkspace((state) => state.undo);
  const redo = useWorkspace((state) => state.redo);
  const [flow, setFlow] = useState<ReactFlowInstance<Node<ConstructData>> | null>(null);
  const previousNodeCount = useRef(nodes.length);
  const preserveViewportForDrop = useRef(false);
  const [tool, setTool] = useState<"select" | "path">("select");
  const [pathSource, setPathSource] = useState<string | null>(null);
  const resultRuns = useMemo(() => runs.filter((run) => run.status === "completed" && run.result), [runs]);
  const selectedResultRun = useMemo(() => resultRuns.find((run) => run.id === selectedResultRunId), [resultRuns, selectedResultRunId]);
  const resultIsCompatible = useMemo(() => Boolean(selectedResultRun?.result && resultMatchesModel(nodes, edges, selectedResultRun.result)), [edges, nodes, selectedResultRun?.result]);
  const compatibleResult = resultIsCompatible ? selectedResultRun?.result : undefined;
  const resultNodes = useMemo(() => decorateNodesWithResults(nodes, compatibleResult), [compatibleResult, nodes]);
  const resultEdges = useMemo(() => decorateEdgesWithResults(edges, compatibleResult), [compatibleResult, edges]);
  const selectedConstructCount = useMemo(() => new Set([...nodes.filter((node) => node.selected).map((node) => node.id), ...(selectedNodeId ? [selectedNodeId] : [])]).size, [nodes, selectedNodeId]);
  const selectedEdge = useMemo(() => edges.find((edge) => edge.id === selectedEdgeId), [edges, selectedEdgeId]);
  const arrangeModel = (direction: "horizontal" | "vertical") => {
    autoLayout(direction);
    window.setTimeout(() => { void flow?.fitView({ padding: 0.2, duration: 220 }); }, 0);
  };

  useEffect(() => {
    if (nodes.length > previousNodeCount.current) {
      if (preserveViewportForDrop.current) preserveViewportForDrop.current = false;
      else window.setTimeout(() => { void flow?.fitView({ padding: 0.16, duration: 220 }); }, 0);
    }
    previousNodeCount.current = nodes.length;
  }, [flow, nodes.length]);

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
        duplicateSelected();
      } else if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        removeSelection();
      } else if (event.key === "Escape") {
        setTool("select");
        setPathSource(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [duplicateSelected, redo, removeSelection, undo]);

  return <main className="model-canvas">
    <div className="canvas-toolbar" role="toolbar" aria-label="Model editing tools">
      <div className="canvas-tool-group">
        <button title="Undo (Ctrl+Z)" disabled={pastCount === 0} onClick={undo}><Undo2 size={15} /></button>
        <button title="Redo (Ctrl+Y)" disabled={futureCount === 0} onClick={redo}><Redo2 size={15} /></button>
      </div>
      <div className="canvas-tool-group">
        <button className={tool === "select" ? "active" : ""} title="Select and move constructs" onClick={() => { setTool("select"); setPathSource(null); }}><MousePointer2 size={15} /></button>
        <button className={tool === "path" ? "active" : ""} title="Draw structural path" onClick={() => { setTool("path"); setPathSource(null); }}><GitBranch size={15} /><span>Path</span></button>
      </div>
      <div className="canvas-tool-group">
        <button title="Add construct" onClick={() => addConstruct()}><Plus size={15} /><span>Construct</span></button>
        <button title="Duplicate selected construct (Ctrl+D)" disabled={!selectedNodeId} onClick={duplicateSelected}><Copy size={15} /></button>
        <button title="Delete selection" disabled={!selectedNodeId && !selectedEdgeId} onClick={removeSelection}><Trash2 size={15} /></button>
      </div>
      <div className="canvas-tool-group path-tools">
        <button title="Reverse selected path" disabled={!selectedEdgeId} onClick={reverseSelectedPath}><ArrowLeftRight size={15} /></button>
        <select
          aria-label="Selected path routing"
          value={String(selectedEdge?.type ?? "smoothstep")}
          disabled={!selectedEdge}
          onChange={(event) => setSelectedPathRouting(event.target.value as "smoothstep" | "default" | "straight")}
        >
          <option value="smoothstep">Orthogonal</option>
          <option value="default">Curved</option>
          <option value="straight">Straight</option>
        </select>
      </div>
      <div className="canvas-tool-group">
        <button title="Arrange model left to right" onClick={() => arrangeModel("horizontal")}><Columns3 size={15} /><span>Horizontal</span></button>
        <button title="Arrange model top to bottom" onClick={() => arrangeModel("vertical")}><Rows3 size={15} /></button>
        <button title="Fit model to view" onClick={() => { void flow?.fitView({ padding: 0.22, duration: 220 }); }}><Focus size={15} /></button>
      </div>
      <div className="canvas-tool-group">
        <button title="Align selected constructs to left edge" disabled={selectedConstructCount < 2} onClick={() => alignSelectedConstructs("left")}><AlignStartVertical size={15} /></button>
        <button title="Align selected constructs by horizontal center" disabled={selectedConstructCount < 2} onClick={() => alignSelectedConstructs("centerX")}><AlignCenterVertical size={15} /></button>
        <button title="Align selected constructs to top edge" disabled={selectedConstructCount < 2} onClick={() => alignSelectedConstructs("top")}><AlignStartHorizontal size={15} /></button>
        <button title="Align selected constructs by vertical center" disabled={selectedConstructCount < 2} onClick={() => alignSelectedConstructs("centerY")}><AlignCenterHorizontal size={15} /></button>
        <button title="Distribute selected constructs horizontally" disabled={selectedConstructCount < 3} onClick={() => distributeSelectedConstructs("horizontal")}><AlignHorizontalSpaceBetween size={15} /></button>
        <button title="Distribute selected constructs vertically" disabled={selectedConstructCount < 3} onClick={() => distributeSelectedConstructs("vertical")}><AlignVerticalSpaceBetween size={15} /></button>
      </div>
      <div className="canvas-tool-group result-tools">
        <button
          className={selectedResultRunId ? "active" : ""}
          title={selectedResultRunId ? "Hide run estimates on diagram" : "Show latest completed run estimates on diagram"}
          disabled={resultRuns.length === 0}
          onClick={() => setSelectedResultRun(selectedResultRunId ? null : resultRuns[0]?.id ?? null)}
        >
          {selectedResultRunId ? <Eye size={15} /> : <EyeOff size={15} />}
          <span>{selectedResultRunId ? "Estimates" : "Model only"}</span>
        </button>
        <select
          aria-label="Diagram result run"
          value={selectedResultRunId ?? ""}
          disabled={resultRuns.length === 0}
          onChange={(event) => setSelectedResultRun(event.target.value || null)}
        >
          <option value="">No diagram estimates</option>
          {resultRuns.map((run) => <option key={run.id} value={run.id}>{run.name} | {new Date(run.createdAt).toLocaleString()}</option>)}
        </select>
      </div>
      {tool === "path" && <div className="canvas-tool-status">{pathSource ? "Choose outcome construct" : "Choose predictor construct"}</div>}
      {selectedResultRunId && !resultIsCompatible ? <div className="canvas-tool-status warning">Selected run does not match this model</div> : null}
    </div>
    <ReactFlow<Node<ConstructData>>
      nodes={resultNodes}
      edges={resultEdges}
      nodeTypes={nodeTypes}
      onInit={setFlow}
      defaultEdgeOptions={{
        type: "smoothstep",
        markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 },
      }}
      onNodesChange={onNodesChange}
      onEdgesChange={onEdgesChange}
      onConnect={onConnect}
      onReconnect={reconnectPath}
      onNodeDragStart={checkpoint}
      onNodeClick={(_, node) => {
        if (tool === "path") {
          if (!pathSource) {
            setPathSource(node.id);
            setSelectedNode(node.id);
          } else if (pathSource !== node.id) {
            addPath(pathSource, node.id);
            setPathSource(null);
          }
          return;
        }
        setSelectedNode(node.id);
      }}
      onEdgeClick={(_, edge) => setSelectedEdge(edge.id)}
      onPaneClick={(event) => {
        setSelectedNode(null);
        if (tool === "path") { setPathSource(null); return; }
        if (event.detail !== 2 || !flow) return;
        addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
      }}
      onDragOver={(event) => { event.preventDefault(); event.dataTransfer.dropEffect = "move"; }}
      onDrop={(event) => {
        event.preventDefault();
        if (!flow) return;
        const encoded = event.dataTransfer.getData("application/qpls-indicators");
        const indicator = event.dataTransfer.getData("application/qpls-indicator");
        let indicators: string[] = indicator ? [indicator] : [];
        if (encoded) {
          try {
            const parsed: unknown = JSON.parse(encoded);
            if (!Array.isArray(parsed)) return;
            indicators = parsed.filter((value): value is string => typeof value === "string");
          } catch { return; }
        }
        if (indicators.length > 0) {
          preserveViewportForDrop.current = true;
          addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }), indicators);
        }
      }}
      fitView
      fitViewOptions={{ padding: 0.2 }}
      minZoom={0.35}
      maxZoom={1.8}
      selectionOnDrag
      multiSelectionKeyCode="Control"
      snapToGrid
      snapGrid={[10, 10]}
      edgesReconnectable
      deleteKeyCode={null}
    >
      <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#dbe1e4" />
      <Controls showInteractive={false} />
      <MiniMap pannable zoomable nodeColor="#c6eef0" maskColor="rgba(246,248,249,.7)" />
    </ReactFlow>
  </main>;
}

function decorateNodesWithResults(nodes: Array<Node<ConstructData>>, result: ReturnType<typeof useWorkspace.getState>["runs"][number]["result"] | undefined): Array<Node<ConstructData>> {
  if (!result) return nodes.map((node) => ({ ...node, data: { ...node.data, resultLoadings: undefined, resultR2: undefined } }));
  const loadingsByConstruct = new Map<string, Record<string, number>>();
  for (const estimate of result.outer_estimates) {
    const loadings = loadingsByConstruct.get(estimate.construct) ?? {};
    loadings[estimate.indicator] = estimate.loading;
    loadingsByConstruct.set(estimate.construct, loadings);
  }
  return nodes.map((node) => ({
    ...node,
    data: {
      ...node.data,
      resultLoadings: loadingsByConstruct.get(node.id),
      resultR2: result.r_squared[node.id],
    },
  }));
}

function resultMatchesModel(nodes: Array<Node<ConstructData>>, edges: Edge[], result: NonNullable<ReturnType<typeof useWorkspace.getState>["runs"][number]["result"]>) {
  const nodeIds = new Set(nodes.map((node) => node.id));
  const currentPaths = new Set(edges.map((edge) => `${edge.source}\u0000${edge.target}`));
  const resultPaths = new Set(result.paths.map((path) => `${path.source}\u0000${path.target}`));
  if (currentPaths.size !== resultPaths.size || [...currentPaths].some((path) => !resultPaths.has(path))) return false;
  const resultIndicators = new Map<string, Set<string>>();
  for (const estimate of result.outer_estimates) {
    if (!nodeIds.has(estimate.construct)) return false;
    const indicators = resultIndicators.get(estimate.construct) ?? new Set<string>();
    indicators.add(estimate.indicator);
    resultIndicators.set(estimate.construct, indicators);
  }
  return nodes.every((node) => {
    const indicators = resultIndicators.get(node.id);
    return indicators !== undefined
      && indicators.size === node.data.indicators.length
      && node.data.indicators.every((indicator) => indicators.has(indicator));
  });
}

function decorateEdgesWithResults(edges: Edge[], result: ReturnType<typeof useWorkspace.getState>["runs"][number]["result"] | undefined): Edge[] {
  if (!result) return edges.map((edge) => ({ ...edge, label: edge.data?.role === "control" ? "Control" : edge.label && edge.label !== "Path" ? edge.label : "Path" }));
  const coefficients = new Map(result.paths.map((path) => [`${path.source}\u0000${path.target}`, path.coefficient]));
  return edges.map((edge) => {
    const coefficient = coefficients.get(`${edge.source}\u0000${edge.target}`);
    return {
      ...edge,
      label: coefficient === undefined ? edge.data?.role === "control" ? "Control" : "Path" : `${edge.data?.role === "control" ? "C " : ""}${coefficient.toFixed(3)}`,
    };
  });
}
