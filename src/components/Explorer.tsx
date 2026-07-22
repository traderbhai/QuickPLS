import { AlertTriangle, ArrowLeftRight, Boxes, ChevronLeft, ChevronRight, Database, Eye, Filter, Focus, GitBranch, GripVertical, Link2, MoreVertical, Network, Pin, Plus, Search, Trash2, X } from "lucide-react";
import { useMemo, useState, type PointerEvent as ReactPointerEvent } from "react";
import type { Edge } from "@xyflow/react";
import { useWorkspace } from "../store";
import type { ConstructData, ExplorerTab } from "../types";

type VariableFilter = "all" | "unassigned" | "assigned" | "selected";

const tabLabels: Array<{ id: ExplorerTab; label: string }> = [
  { id: "constructs", label: "Constructs" },
  { id: "variables", label: "Variables" },
  { id: "structure", label: "Structure" },
  { id: "issues", label: "Issues" },
];

function directStoreAction(action: () => void) {
  action();
}

function edgeLabel(edge: Edge, constructs: Map<string, string>) {
  const source = constructs.get(edge.source) ?? edge.source;
  const target = constructs.get(edge.target) ?? edge.target;
  return `${source} ${edge.data?.role === "covariance" ? "<->" : "->"} ${target}`;
}

export function Explorer() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const projectName = useWorkspace((state) => state.projectName);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const explorerTab = useWorkspace((state) => state.explorerTab);
  const explorerCollapsed = useWorkspace((state) => state.explorerCollapsed);
  const setExplorerTab = useWorkspace((state) => state.setExplorerTab);
  const setExplorerCollapsed = useWorkspace((state) => state.setExplorerCollapsed);
  const setExplorerWidth = useWorkspace((state) => state.setExplorerWidth);
  const setView = useWorkspace((state) => state.setView);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const addPath = useWorkspace((state) => state.addPath);
  const updateConstruct = useWorkspace((state) => state.updateConstruct);
  const duplicateSelected = useWorkspace((state) => state.duplicateSelected);
  const removeSelection = useWorkspace((state) => state.removeSelection);
  const reverseSelectedPath = useWorkspace((state) => state.reverseSelectedPath);
  const setPathRouting = useWorkspace((state) => state.setPathRouting);
  const resetEdgeLabel = useWorkspace((state) => state.resetEdgeLabel);
  const setConstructIndicatorSide = useWorkspace((state) => state.setConstructIndicatorSide);
  const toggleConstructPinned = useWorkspace((state) => state.toggleConstructPinned);
  const resetIndicatorLayout = useWorkspace((state) => state.resetIndicatorLayout);
  const assignIndicators = useWorkspace((state) => state.assignIndicators);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  const unassignIndicator = useWorkspace((state) => state.unassignIndicator);
  const addConstructsFromIndicators = useWorkspace((state) => state.addConstructsFromIndicators);
  const addConstructsFromIndicatorGroups = useWorkspace((state) => state.addConstructsFromIndicatorGroups);
  const [query, setQuery] = useState("");
  const [variableFilter, setVariableFilter] = useState<VariableFilter>("all");
  const [selectedVariables, setSelectedVariables] = useState<string[]>([]);
  const [draggingVariables, setDraggingVariables] = useState<string[]>([]);

  const ownerByIndicator = useMemo(() => new Map(nodes.flatMap((node) => node.data.indicators.map((indicator) => [indicator, node.id]))), [nodes]);
  const constructLabelById = useMemo(() => new Map(nodes.map((node) => [node.id, node.data.shortName || node.data.label])), [nodes]);
  const selectedConstruct = selectedNodeId ? nodes.find((node) => node.id === selectedNodeId) ?? null : null;
  const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance" && !edge.id.startsWith("measurement::"));
  const covarianceEdges = edges.filter((edge) => edge.data?.role === "covariance");
  const assignedVariableCount = ownerByIndicator.size;
  const variables = dataset.columns
    .filter((column) => column.toLowerCase().includes(query.toLowerCase()))
    .filter((column) => {
      const assigned = ownerByIndicator.has(column);
      if (variableFilter === "assigned") return assigned;
      if (variableFilter === "unassigned") return !assigned;
      if (variableFilter === "selected") return selectedVariables.includes(column);
      return true;
    });
  const issues = useMemo(() => {
    const list: Array<{ tone: "error" | "warning" | "info"; title: string; detail: string; action?: () => void }> = [];
    if (dataset.columns.length === 0) list.push({ tone: "error", title: "No dataset columns", detail: "Import a raw dataset before building a model.", action: () => setView("data") });
    if ((dataset.rowCount ?? dataset.rows.length) === 0) list.push({ tone: "warning", title: "No sample rows", detail: "The model can be drawn, but analyses need observed rows.", action: () => setView("data") });
    if (nodes.length === 0) list.push({ tone: "error", title: "No constructs", detail: "Add at least one latent construct to start the SEM model.", action: () => addConstruct() });
    nodes.forEach((node) => {
      if (node.data.indicators.length === 0) list.push({ tone: "error", title: `${node.data.label} has no indicators`, detail: "Assign observed variables from the Variables tab.", action: () => focusConstruct(node.id) });
      else if (node.data.indicators.length === 1) list.push({ tone: "info", title: `${node.data.label} is single-item`, detail: "Single-item constructs are supported but should be deliberate.", action: () => focusConstruct(node.id) });
    });
    if (nodes.length > 1 && structuralEdges.length === 0) list.push({ tone: "warning", title: "No structural paths", detail: "Use the Path tool or Structure tab to connect constructs.", action: () => setExplorerTab("structure") });
    if (dataset.missing > 0) list.push({ tone: "info", title: "Missing values detected", detail: `${dataset.missing} missing cells will follow the selected missing-data policy.`, action: () => setView("data") });
    return list;
  }, [addConstruct, dataset, nodes, setExplorerTab, setView, structuralEdges.length]);

  function focusConstruct(id: string) {
    setSelectedNode(id);
    window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id } }));
  }

  function focusEdge(id: string) {
    setSelectedEdge(id);
    window.dispatchEvent(new CustomEvent("quickpls:focus-edge", { detail: { id } }));
  }

  function renameConstruct(node: { id: string; data: ConstructData }) {
    const value = window.prompt("Construct name", node.data.label);
    if (value?.trim()) updateConstruct(node.id, { label: value.trim() });
  }

  function duplicateConstruct(id: string) {
    directStoreAction(() => {
      useWorkspace.getState().setSelectedNode(id);
      useWorkspace.getState().duplicateSelected();
    });
  }

  function deleteConstruct(id: string) {
    if (!window.confirm("Delete this construct and all connected paths?")) return;
    directStoreAction(() => {
      useWorkspace.getState().setSelectedNode(id);
      useWorkspace.getState().removeSelection();
    });
  }

  function createPathFrom(sourceId: string) {
    const targetText = window.prompt("Target construct short name or label");
    if (!targetText?.trim()) return;
    const normalized = targetText.trim().toLowerCase();
    const target = nodes.find((node) => node.id !== sourceId && [node.data.shortName, node.data.label, node.id].some((value) => value.toLowerCase() === normalized));
    if (target) addPath(sourceId, target.id);
  }

  function reversePath(id: string) {
    directStoreAction(() => {
      useWorkspace.getState().setSelectedEdge(id);
      useWorkspace.getState().reverseSelectedPath();
    });
  }

  function deleteEdge(id: string) {
    directStoreAction(() => {
      useWorkspace.getState().setSelectedEdge(id);
      useWorkspace.getState().removeSelection();
    });
  }

  function startResize(event: ReactPointerEvent<HTMLDivElement>) {
    event.preventDefault();
    const handleMove = (move: PointerEvent) => setExplorerWidth(move.clientX - 76);
    const handleUp = () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", handleUp);
    };
    window.addEventListener("pointermove", handleMove);
    window.addEventListener("pointerup", handleUp);
  }

  if (explorerCollapsed) {
    return <aside className="explorer collapsed" aria-label="Collapsed SEM explorer">
      <button className="explorer-expand" title="Expand SEM explorer" onClick={() => setExplorerCollapsed(false)}><ChevronRight size={17} /></button>
      <span>SEM</span>
    </aside>;
  }

  return <aside className="explorer sem-explorer" aria-label="SEM explorer">
    <header className="explorer-header">
      <div>
        <span className="eyebrow">SEM explorer</span>
        <strong>{projectName}</strong>
      </div>
      <button title="Collapse SEM explorer" onClick={() => setExplorerCollapsed(true)}><ChevronLeft size={16} /></button>
    </header>

    <section className="explorer-status-card" aria-label="Project data status">
      <div><Database size={15} /><span>{dataset.name}</span><b>{dataset.rowCount ?? dataset.rows.length} rows</b></div>
      <div className="explorer-status-actions">
        <button onClick={() => setView("data")}>Data</button>
        <button onClick={() => setView("analyses")}>Validate</button>
      </div>
    </section>

    <nav className="explorer-tabs" aria-label="SEM explorer sections">
      {tabLabels.map((tab) => <button key={tab.id} className={explorerTab === tab.id ? "active" : ""} onClick={() => setExplorerTab(tab.id)}>{tab.label}</button>)}
    </nav>

    <div className="explorer-body">
      {explorerTab === "constructs" && <>
        <div className="explorer-section-heading">
          <div><Network size={15} /><strong>Constructs</strong><span>{nodes.length}</span></div>
          <button onClick={() => addConstruct()}><Plus size={14} />Add</button>
        </div>
        <label className="explorer-search"><Search size={13} /><input aria-label="Search constructs" placeholder="Find construct" value={query} onChange={(event) => setQuery(event.target.value)} /></label>
        <div className="explorer-list">
          {nodes.filter((node) => node.data.label.toLowerCase().includes(query.toLowerCase()) || node.data.shortName.toLowerCase().includes(query.toLowerCase())).map((node) => {
            const incoming = structuralEdges.filter((edge) => edge.target === node.id).length;
            const outgoing = structuralEdges.filter((edge) => edge.source === node.id).length;
            return <article key={node.id} className={selectedNodeId === node.id ? "explorer-card selected" : "explorer-card"}>
              <button className="explorer-card-main" onClick={() => focusConstruct(node.id)}>
                <span className={`mode-dot ${node.data.mode}`} />
                <span><strong>{node.data.label}</strong><small>{node.data.shortName} · {node.data.indicators.length} indicators · {incoming} in / {outgoing} out</small></span>
              </button>
              <div className="explorer-card-actions" aria-label={`${node.data.label} actions`}>
                <button title="Focus construct" onClick={() => focusConstruct(node.id)}><Focus size={13} /></button>
                <button title="Rename construct" onClick={() => renameConstruct(node)}><MoreVertical size={13} /></button>
                <button title="Duplicate construct" onClick={() => duplicateConstruct(node.id)}><Boxes size={13} /></button>
                <button title="Create path from this construct" onClick={() => createPathFrom(node.id)}><GitBranch size={13} /></button>
                <button title="Pin or unpin layout" onClick={() => toggleConstructPinned(node.id)}><Pin size={13} /></button>
                <button title="Delete construct" onClick={() => deleteConstruct(node.id)}><Trash2 size={13} /></button>
              </div>
              <div className="explorer-mini-actions">
                {(["left", "right", "top", "bottom"] as const).map((side) => <button key={side} onClick={() => setConstructIndicatorSide(node.id, side)}>{side}</button>)}
                <button onClick={() => resetIndicatorLayout(node.id)}>Reset indicators</button>
              </div>
            </article>;
          })}
        </div>
      </>}

      {explorerTab === "variables" && <>
        <div className="explorer-section-heading">
          <div><Database size={15} /><strong>Dataset variables</strong><span>{dataset.columns.length}</span></div>
          <button onClick={() => setVariableFilter(variableFilter === "all" ? "unassigned" : "all")}><Filter size={14} />{variableFilter}</button>
        </div>
        <label className="explorer-search"><Search size={13} /><input aria-label="Search dataset variables" placeholder="Find variable" value={query} onChange={(event) => setQuery(event.target.value)} /></label>
        <div className="explorer-filter-row">
          {(["all", "unassigned", "assigned", "selected"] as const).map((filter) => <button key={filter} className={variableFilter === filter ? "active" : ""} onClick={() => setVariableFilter(filter)}>{filter}</button>)}
        </div>
        {selectedVariables.length > 0 && <div className="variable-bulk-actions" aria-live="polite">
          <span>{selectedVariables.length} selected</span>
          <button title="Create construct from selected variables" onClick={() => { addConstruct(undefined, selectedVariables); setSelectedVariables([]); }}><Plus size={13} />Construct</button>
          <button title="Create constructs grouped by variable prefix" onClick={() => { addConstructsFromIndicatorGroups(selectedVariables); setSelectedVariables([]); }}><Boxes size={13} />Group</button>
          <button title="Create one construct per selected variable" onClick={() => { addConstructsFromIndicators(selectedVariables); setSelectedVariables([]); }}><Boxes size={13} />Separate</button>
          <button title={selectedConstruct ? `Assign to ${selectedConstruct.data.label}` : "Select a construct first"} disabled={!selectedConstruct} onClick={() => { if (selectedConstruct) assignIndicators(selectedConstruct.id, selectedVariables); setSelectedVariables([]); }}><Network size={13} />Assign</button>
          <button title="Clear variable selection" onClick={() => setSelectedVariables([])}><X size={13} /></button>
        </div>}
        <div className="variable-list">
          {variables.map((variable) => {
            const ownerId = ownerByIndicator.get(variable);
            const owner = ownerId ? constructLabelById.get(ownerId) : null;
            const checked = selectedVariables.includes(variable);
            return <div
              key={variable}
              className={`${owner ? "variable-row assigned" : "variable-row"}${checked ? " selected" : ""}${draggingVariables.includes(variable) ? " dragging" : ""}`}
              draggable
              title={`Drag ${checked ? "selected variables" : variable} onto a construct or empty canvas`}
              onDragStart={(event) => {
                const dragged = checked ? selectedVariables : [variable];
                setDraggingVariables(dragged);
                window.dispatchEvent(new CustomEvent("quickpls:variables-dragging", { detail: { count: dragged.length } }));
                event.dataTransfer.setData("application/qpls-indicator", variable);
                event.dataTransfer.setData("application/qpls-indicators", JSON.stringify(dragged));
                event.dataTransfer.effectAllowed = "move";
              }}
              onDragEnd={() => {
                setDraggingVariables([]);
                window.dispatchEvent(new CustomEvent("quickpls:variables-dragging", { detail: { count: 0 } }));
              }}
            >
              <GripVertical size={12} />
              <input type="checkbox" aria-label={`Select ${variable}`} checked={checked} onChange={() => setSelectedVariables((current) => current.includes(variable) ? current.filter((item) => item !== variable) : [...current, variable])} />
              <button type="button" aria-pressed={checked} onClick={() => setSelectedVariables((current) => current.includes(variable) ? current.filter((item) => item !== variable) : [...current, variable])}><span>{variable}</span><small>{owner ?? "Unassigned"}</small></button>
              {ownerId && <button className="row-icon" title={`Focus ${owner}`} onClick={() => focusConstruct(ownerId)}><Eye size={12} /></button>}
              {ownerId && <button className="row-icon" title="Unassign variable" onClick={() => unassignIndicator(ownerId, variable)}><X size={12} /></button>}
              {!ownerId && selectedConstruct && <button className="row-icon" title={`Assign to ${selectedConstruct.data.label}`} onClick={() => assignIndicator(selectedConstruct.id, variable)}><Plus size={12} /></button>}
            </div>;
          })}
        </div>
      </>}

      {explorerTab === "structure" && <>
        <div className="explorer-section-heading">
          <div><GitBranch size={15} /><strong>Structural model</strong><span>{structuralEdges.length} paths</span></div>
        </div>
        <div className="explorer-list compact">
          {structuralEdges.map((edge) => <article key={edge.id} className={selectedEdgeId === edge.id ? "structure-row selected" : "structure-row"}>
            <button className="structure-main" onClick={() => focusEdge(edge.id)}><Link2 size={13} /><span>{edgeLabel(edge, constructLabelById)}</span></button>
            <div className="structure-actions">
              <button title="Focus path" onClick={() => focusEdge(edge.id)}><Focus size={12} /></button>
              <button title="Reverse path" onClick={() => reversePath(edge.id)}><ArrowLeftRight size={12} /></button>
              <button title="Straight route" onClick={() => setPathRouting(edge.id, "straight")}>Straight</button>
              <button title="Reset label" onClick={() => resetEdgeLabel(edge.id)}>Label</button>
              <button title="Delete path" onClick={() => deleteEdge(edge.id)}><Trash2 size={12} /></button>
            </div>
          </article>)}
          {structuralEdges.length === 0 && <div className="empty-panel">No structural paths yet. Select Path in the canvas toolbar or create paths from construct actions.</div>}
        </div>
        <div className="explorer-section-heading sub">
          <div><ArrowLeftRight size={15} /><strong>Covariances</strong><span>{covarianceEdges.length}</span></div>
        </div>
        <div className="explorer-list compact">
          {covarianceEdges.map((edge) => <article key={edge.id} className={selectedEdgeId === edge.id ? "structure-row selected" : "structure-row"}>
            <button className="structure-main" onClick={() => focusEdge(edge.id)}><ArrowLeftRight size={13} /><span>{edgeLabel(edge, constructLabelById)}</span></button>
            <div className="structure-actions">
              <button title="Focus covariance" onClick={() => focusEdge(edge.id)}><Focus size={12} /></button>
              <button title="Curved route" onClick={() => setPathRouting(edge.id, "default")}>Curved</button>
              <button title="Reset label" onClick={() => resetEdgeLabel(edge.id)}>Label</button>
              <button title="Delete covariance" onClick={() => deleteEdge(edge.id)}><Trash2 size={12} /></button>
            </div>
          </article>)}
        </div>
      </>}

      {explorerTab === "issues" && <>
        <div className="explorer-section-heading">
          <div><AlertTriangle size={15} /><strong>Model issues</strong><span>{issues.length}</span></div>
        </div>
        <div className="explorer-list compact">
          {issues.length === 0 && <div className="empty-panel success">No obvious model-building issues in the current diagram.</div>}
          {issues.map((issue, index) => <article key={`${issue.title}-${index}`} className={`issue-row ${issue.tone}`}>
            <AlertTriangle size={14} />
            <div><strong>{issue.title}</strong><span>{issue.detail}</span></div>
            {issue.action && <button onClick={issue.action}>Fix</button>}
          </article>)}
        </div>
      </>}
    </div>

    <footer className="explorer-summary">
      <span><b>{nodes.length}</b> constructs</span>
      <span><b>{assignedVariableCount}</b> indicators</span>
      <span><b>{structuralEdges.length}</b> paths</span>
      <span><b>{dataset.missing}</b> missing</span>
    </footer>
    <div className="explorer-resize-handle" title="Resize SEM explorer" onPointerDown={startResize} />
  </aside>;
}
