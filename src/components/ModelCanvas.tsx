import {
  Background,
  BackgroundVariant,
  Controls,
  MarkerType,
  MiniMap,
  ReactFlow,
  type EdgeChange,
  type Node,
  type NodeChange,
  type NodeTypes,
  type ReactFlowInstance,
} from "@xyflow/react";
import { AlignCenterHorizontal, AlignCenterVertical, AlignHorizontalSpaceBetween, AlignStartHorizontal, AlignStartVertical, AlignVerticalSpaceBetween, ArrowLeftRight, Box, Circle, CircleHelp, Columns3, Copy, Focus, GitBranch, Hand, Link2, MousePointer2, Plus, Redo2, Rows3, Square, Trash2, Type, Undo2 } from "lucide-react";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import { buildDiagramGraph, isIndicatorNodeId, parseIndicatorNodeId } from "../domain/diagramGraph";
import { useWorkspace } from "../store";
import type { ConstructData, DiagramToolMode, IndicatorSide } from "../types";
import { ConstructNode } from "./ConstructNode";
import { IndicatorNode } from "./IndicatorNode";
import { LatentNode } from "./LatentNode";

const nodeTypes: NodeTypes = { construct: memo(ConstructNode), latent: memo(LatentNode), indicator: memo(IndicatorNode) };

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
  const diagramMode = useWorkspace((state) => state.diagramMode);
  const diagramTool = useWorkspace((state) => state.diagramTool);
  const diagramOverlaySettings = useWorkspace((state) => state.diagramOverlaySettings);
  const diagramLayout = useWorkspace((state) => state.diagramLayout);
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
  const setDiagramMode = useWorkspace((state) => state.setDiagramMode);
  const setDiagramTool = useWorkspace((state) => state.setDiagramTool);
  const setDiagramOverlaySettings = useWorkspace((state) => state.setDiagramOverlaySettings);
  const checkpoint = useWorkspace((state) => state.checkpoint);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const duplicateSelected = useWorkspace((state) => state.duplicateSelected);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const reverseSelectedPath = useWorkspace((state) => state.reverseSelectedPath);
  const setSelectedPathRouting = useWorkspace((state) => state.setSelectedPathRouting);
  const alignSelectedConstructs = useWorkspace((state) => state.alignSelectedConstructs);
  const distributeSelectedConstructs = useWorkspace((state) => state.distributeSelectedConstructs);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const moveIndicator = useWorkspace((state) => state.moveIndicator);
  const setIndicatorSide = useWorkspace((state) => state.setIndicatorSide);
  const resetIndicatorLayout = useWorkspace((state) => state.resetIndicatorLayout);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const unassignIndicator = useWorkspace((state) => state.unassignIndicator);
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const updateEdge = useWorkspace((state) => state.updateEdge);
  const undo = useWorkspace((state) => state.undo);
  const redo = useWorkspace((state) => state.redo);
  const [flow, setFlow] = useState<ReactFlowInstance | null>(null);
  const previousNodeCount = useRef(nodes.length);
  const preserveViewportForDrop = useRef(false);
  const [pathSource, setPathSource] = useState<string | null>(null);
  const [showHelp, setShowHelp] = useState(false);
  const [contextMenu, setContextMenu] = useState<null | { kind: "canvas"; x: number; y: number } | { kind: "construct"; id: string; x: number; y: number } | { kind: "indicator"; constructId: string; indicator: string; x: number; y: number } | { kind: "edge"; id: string; x: number; y: number }>(null);
  const resultRuns = useMemo(() => runs.filter((run) => run.status === "completed" && run.result), [runs]);
  const selectedResultRun = useMemo(() => resultRuns.find((run) => run.id === selectedResultRunId), [resultRuns, selectedResultRunId]);
  const graph = useMemo(() => buildDiagramGraph(nodes, edges, diagramMode, diagramOverlaySettings.mode, selectedResultRun, { layout: diagramLayout, layoutSource: diagramMode === "publication" ? "current_canvas" : undefined }), [diagramLayout, diagramMode, diagramOverlaySettings.mode, edges, nodes, selectedResultRun]);
  const selectedConstructCount = useMemo(() => new Set([...nodes.filter((node) => node.selected).map((node) => node.id), ...(selectedNodeId ? [selectedNodeId] : [])]).size, [nodes, selectedNodeId]);
  const selectedEdge = useMemo(() => edges.find((edge) => edge.id === selectedEdgeId), [edges, selectedEdgeId]);
  const resultDiagramMode = diagramMode === "smartpls_result" || diagramMode === "publication";
  const paperStyleCanvas = diagramMode === "sem" || diagramMode === "publication" || diagramMode === "smartpls_result";
  const arrangeModel = (direction: "horizontal" | "vertical" | "smartpls") => {
    autoLayout(direction);
    window.setTimeout(() => { void flow?.fitView({ padding: 0.2, duration: 220 }); }, 0);
  };
  const setMode = (mode: typeof diagramMode) => {
    setDiagramMode(mode);
    if (mode === "smartpls_result" || mode === "publication") {
      setDiagramOverlaySettings({ mode: selectedResultRunId ? "paths_r2" : "model" });
      window.setTimeout(() => { void flow?.fitView({ padding: 0.16, duration: 220 }); }, 0);
    }
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
        if (resultDiagramMode) return;
        duplicateSelected();
      } else if (event.key === "Delete" || event.key === "Backspace") {
        event.preventDefault();
        if (resultDiagramMode) return;
        removeSelection();
      } else if (event.key === "Escape") {
        setDiagramTool("select");
        setPathSource(null);
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
  }, [duplicateSelected, flow, redo, removeSelection, resultDiagramMode, setDiagramTool, undo]);

  const selectTool = (tool: DiagramToolMode) => {
    setDiagramTool(tool);
    setPathSource(null);
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
  const chooseConstruct = (id: string) => {
    if (diagramTool === "path" || diagramTool === "covariance") {
      if (!pathSource) {
        setPathSource(id);
        setSelectedNode(id);
      } else if (pathSource !== id) {
        diagramTool === "path" ? addPath(pathSource, id) : addCovariance(pathSource, id);
        setPathSource(null);
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

  return <main className={`model-canvas${paperStyleCanvas ? " smartpls-result-canvas" : ""}${resultDiagramMode ? " locked-result-canvas" : ""}`}>
    <div className="canvas-toolbar" role="toolbar" aria-label="Model editing tools">
      <div className="canvas-tool-group">
        <button title="Undo (Ctrl+Z)" disabled={pastCount === 0} onClick={undo}><Undo2 size={15} /></button>
        <button title="Redo (Ctrl+Y)" disabled={futureCount === 0} onClick={redo}><Redo2 size={15} /></button>
      </div>
      <div className="canvas-tool-group">
        <button className={diagramTool === "select" ? "active" : ""} title="Select and move diagram items (V)" disabled={resultDiagramMode} onClick={() => selectTool("select")}><MousePointer2 size={15} /></button>
        <button className={diagramTool === "pan" ? "active" : ""} title="Pan canvas" onClick={() => selectTool("pan")}><Hand size={15} /></button>
        <button className={diagramTool === "construct" ? "active" : ""} title="Latent construct tool" disabled={resultDiagramMode} onClick={() => selectTool("construct")}><Circle size={15} /></button>
        <button className={diagramTool === "indicator" ? "active" : ""} title="Observed indicator tool" disabled={resultDiagramMode} onClick={() => selectTool("indicator")}><Square size={15} /></button>
        <button className={diagramTool === "path" ? "active" : ""} title="Draw structural path (P)" disabled={resultDiagramMode} onClick={() => selectTool("path")}><GitBranch size={15} /><span>Path</span></button>
        <button className={diagramTool === "covariance" ? "active" : ""} title="Draw covariance display arc (C)" disabled={resultDiagramMode} onClick={() => selectTool("covariance")}><Link2 size={15} /><span>Cov</span></button>
        <button className={diagramTool === "residual" ? "active" : ""} title="Residual/error node placeholder" disabled={resultDiagramMode} onClick={() => selectTool("residual")}><Box size={15} /></button>
        <button className={diagramTool === "caption" ? "active" : ""} title="Caption placeholder" disabled={resultDiagramMode} onClick={() => selectTool("caption")}><Type size={15} /></button>
      </div>
      <div className="canvas-tool-group">
        <button title="Add construct" disabled={resultDiagramMode} onClick={() => addConstruct()}><Plus size={15} /><span>Construct</span></button>
        <button title="Duplicate selected construct (Ctrl+D)" disabled={resultDiagramMode || !selectedNodeId} onClick={duplicateSelected}><Copy size={15} /></button>
        <button title="Delete selection" disabled={resultDiagramMode || (!selectedNodeId && !selectedEdgeId)} onClick={removeSelection}><Trash2 size={15} /></button>
      </div>
      <div className="canvas-tool-group path-tools">
        <button title="Reverse selected path" disabled={!selectedEdgeId || selectedEdge?.data?.role === "covariance"} onClick={reverseSelectedPath}><ArrowLeftRight size={15} /></button>
        <select aria-label="Selected path routing" value={String(selectedEdge?.type ?? "smoothstep")} disabled={!selectedEdge || selectedEdge.data?.role === "covariance"} onChange={(event) => setSelectedPathRouting(event.target.value as "smoothstep" | "default" | "straight")}>
          <option value="smoothstep">Orthogonal</option>
          <option value="default">Curved</option>
          <option value="straight">Straight</option>
        </select>
      </div>
      <div className="canvas-tool-group">
        <button title="Arrange model left to right" onClick={() => arrangeModel("horizontal")}><Columns3 size={15} /><span>Horizontal</span></button>
        <button title="Arrange model top to bottom" onClick={() => arrangeModel("vertical")}><Rows3 size={15} /></button>
        <button title="Arrange like SmartPLS" disabled={resultDiagramMode} onClick={() => arrangeModel("smartpls")}><Focus size={15} /><span>SmartPLS</span></button>
        <button title="Fit model to view (F)" onClick={() => { void flow?.fitView({ padding: 0.22, duration: 220 }); }}><Focus size={15} /></button>
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
        <select aria-label="Diagram mode" value={diagramMode} onChange={(event) => setMode(event.target.value as typeof diagramMode)}>
          <option value="sem">Edit model</option>
          <option value="smartpls_result">Result diagram</option>
          <option value="compact">Compact</option>
          <option value="publication">Publication preview</option>
        </select>
        <select aria-label="Diagram result run" value={selectedResultRunId ?? ""} disabled={resultRuns.length === 0} onChange={(event) => setSelectedResultRun(event.target.value || null)}>
          <option value="">No diagram estimates</option>
          {resultRuns.map((run) => <option key={run.id} value={run.id}>{run.name} | {new Date(run.createdAt).toLocaleString()}</option>)}
        </select>
        <select aria-label="Diagram result overlay" value={diagramOverlaySettings.mode} disabled={!selectedResultRunId} onChange={(event) => setDiagramOverlaySettings({ mode: event.target.value as typeof diagramOverlaySettings.mode })}>
          <option value="model">Model only</option>
          <option value="loadings">Loadings / weights</option>
          <option value="paths_r2">Paths + R²</option>
          <option value="significance">Significance</option>
          <option value="quality">Reliability warnings</option>
          <option value="cbsem_standardized">CB-SEM standardized</option>
          <option value="cbsem_residuals">CB-SEM residuals</option>
          <option value="modification_indices">Modification indices</option>
        </select>
        <button title="Diagram legend" onClick={() => setShowHelp((value) => !value)}><CircleHelp size={15} /></button>
      </div>
      {(diagramTool === "path" || diagramTool === "covariance") && <div className="canvas-tool-status">{pathSource ? `Choose ${diagramTool === "path" ? "outcome construct" : "second construct"}` : `Choose ${diagramTool === "path" ? "predictor construct" : "first construct"}`}</div>}
      {graph.diagnostic ? <div className="canvas-tool-status warning">{graph.diagnostic}</div> : null}
    </div>
    {showHelp && <div className="diagram-help" role="dialog" aria-label="Diagram legend">
      <strong>Diagram legend</strong>
      <span><i className="legend-latent" />Latent construct</span>
      <span><i className="legend-indicator" />Observed indicator</span>
      <span><i className="legend-path" />Structural path</span>
      <span><i className="legend-covariance" />Covariance display</span>
      <span>Shortcuts: P path, C covariance, V select, F fit view, Esc cancel. Right-click opens object actions.</span>
    </div>}
    {contextMenu ? <div className="diagram-context-menu" style={{ left: contextMenu.x, top: contextMenu.y }}>
      {contextMenu.kind === "canvas" ? <>
        <button onClick={() => { if (flow) addConstruct(flow.screenToFlowPosition({ x: contextMenu.x, y: contextMenu.y })); setContextMenu(null); }}>Add latent construct</button>
        <button onClick={() => { arrangeModel("smartpls"); setContextMenu(null); }}>Arrange like SmartPLS</button>
        <button onClick={() => { void flow?.fitView({ padding: 0.22, duration: 220 }); setContextMenu(null); }}>Fit view</button>
      </> : contextMenu.kind === "construct" ? <>
        <button onClick={() => { renameConstruct(contextMenu.id); setContextMenu(null); }}>Rename construct</button>
        <button onClick={() => { updateConstruct(contextMenu.id, { mode: nodes.find((node) => node.id === contextMenu.id)?.data.mode === "reflective" ? "formative" : "reflective" }); setContextMenu(null); }}>Invert reflective/formative</button>
        <button onClick={() => { resetIndicatorLayout(contextMenu.id); setContextMenu(null); }}>Align indicators</button>
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
        <button onClick={() => { updateEdge(contextMenu.id, { type: "straight" }); setContextMenu(null); }}>Straight route</button>
        <button onClick={() => { updateEdge(contextMenu.id, { type: "smoothstep" }); setContextMenu(null); }}>Orthogonal route</button>
        <button onClick={() => { updateEdge(contextMenu.id, { type: "default" }); setContextMenu(null); }}>Curved route</button>
        <button onClick={() => { updateEdge(contextMenu.id, { label: "Control", data: { role: "control" } }); setContextMenu(null); }}>Mark control</button>
        <button className="danger" onClick={() => { setSelectedEdge(contextMenu.id); removeSelection(); setContextMenu(null); }}>Delete</button>
      </>}
    </div> : null}
    <ReactFlow
      nodes={graph.nodes}
      edges={graph.edges}
      nodeTypes={nodeTypes}
      onInit={setFlow}
      defaultEdgeOptions={{ type: "smoothstep", markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16 } }}
      onNodesChange={resultDiagramMode ? undefined : onVisualNodesChange}
      onEdgesChange={resultDiagramMode ? undefined : onVisualEdgesChange}
      onConnect={(connection) => {
        if (resultDiagramMode) return;
        if (!connection.source || !connection.target || isIndicatorNodeId(connection.source) || isIndicatorNodeId(connection.target)) return;
        diagramTool === "covariance" ? addCovariance(connection.source, connection.target) : onConnect(connection);
      }}
      onReconnect={resultDiagramMode ? undefined : reconnectPath}
      onNodeDragStart={resultDiagramMode ? undefined : checkpoint}
      onNodeDragStop={resultDiagramMode ? undefined : (_, node) => {
        const indicator = parseIndicatorNodeId(node.id);
        if (!indicator) return;
        const target = nearestConstructForIndicator(node, indicator.constructId);
        if (target) assignIndicator(target.id, indicator.indicator);
        else moveIndicator(indicator.constructId, indicator.indicator, node.position);
      }}
      onNodeClick={(_, node) => {
        const indicator = parseIndicatorNodeId(node.id);
        if (indicator) setSelectedNode(indicator.constructId);
        else chooseConstruct(node.id);
      }}
      onEdgeClick={(_, edge) => setSelectedEdge(edge.id)}
      onNodeContextMenu={(event, node) => {
        event.preventDefault();
        if (resultDiagramMode) return;
        const indicator = parseIndicatorNodeId(node.id);
        setContextMenu(indicator ? { kind: "indicator", ...indicator, x: event.clientX, y: event.clientY } : { kind: "construct", id: node.id, x: event.clientX, y: event.clientY });
      }}
      onEdgeContextMenu={(event, edge) => {
        event.preventDefault();
        if (resultDiagramMode || edge.id.startsWith("measurement::")) return;
        setContextMenu({ kind: "edge", id: edge.id, x: event.clientX, y: event.clientY });
      }}
      onPaneContextMenu={(event) => {
        event.preventDefault();
        if (resultDiagramMode) return;
        setContextMenu({ kind: "canvas", x: event.clientX, y: event.clientY });
      }}
      onPaneClick={(event) => {
        setContextMenu(null);
        setSelectedNode(null);
        if (diagramTool === "path" || diagramTool === "covariance") { setPathSource(null); return; }
        if (resultDiagramMode) return;
        if (!flow) return;
        if (diagramTool === "construct") {
          addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
          return;
        }
        if (diagramTool === "indicator" || diagramTool === "residual" || diagramTool === "caption") return;
        if (event.detail !== 2) return;
        addConstruct(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY }));
      }}
      onDragOver={(event) => { if (resultDiagramMode) return; event.preventDefault(); event.dataTransfer.dropEffect = "move"; }}
      onDrop={(event) => {
        event.preventDefault();
        if (resultDiagramMode) return;
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
      minZoom={0.25}
      maxZoom={2.2}
      selectionOnDrag
      panOnDrag={resultDiagramMode || diagramTool === "pan"}
      multiSelectionKeyCode="Control"
      snapToGrid
      snapGrid={[10, 10]}
      nodesDraggable={!resultDiagramMode && diagramTool !== "pan"}
      nodesConnectable={!resultDiagramMode}
      edgesReconnectable={!resultDiagramMode}
      deleteKeyCode={null}
    >
      {!paperStyleCanvas ? <Background variant={BackgroundVariant.Dots} gap={20} size={1} color="#dbe1e4" /> : null}
      <Controls showInteractive={false} />
      {!paperStyleCanvas ? <MiniMap pannable zoomable nodeColor={(node) => node.type === "indicator" ? "#f8dd8a" : "#c6eef0"} maskColor="rgba(246,248,249,.7)" /> : null}
    </ReactFlow>
  </main>;
}
