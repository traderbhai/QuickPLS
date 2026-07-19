import { Boxes, ChevronDown, Database, GripVertical, MoreVertical, Network, Plus, Search, X } from "lucide-react";
import { useMemo, useState } from "react";
import { useWorkspace } from "../store";

export function Explorer() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const addConstruct = useWorkspace((state) => state.addConstruct);
  const addConstructsFromIndicators = useWorkspace((state) => state.addConstructsFromIndicators);
  const addConstructsFromIndicatorGroups = useWorkspace((state) => state.addConstructsFromIndicatorGroups);
  const assignIndicators = useWorkspace((state) => state.assignIndicators);
  const [query, setQuery] = useState("");
  const [selectedVariables, setSelectedVariables] = useState<string[]>([]);
  const ownerByIndicator = useMemo(() => new Map(nodes.flatMap((node) => node.data.indicators.map((indicator) => [indicator, node.data.shortName]))), [nodes]);
  const variables = dataset.columns.filter((column) => column.toLowerCase().includes(query.toLowerCase()));

  return <aside className="explorer">
    <div className="pane-tabs"><button>Data</button><button className="active">Model</button></div>
    <div className="tree-block">
      <div className="tree-heading"><ChevronDown size={14} /><Network size={15} /><strong>Corporate Reputation</strong><MoreVertical size={15} /></div>
      <div className="tree-row dataset-row"><Database size={14} />{dataset.name}<span>{dataset.rowCount ?? dataset.rows.length}</span></div>
      <div className="section-label">Constructs <button title="Add construct" onClick={() => addConstruct()}><Plus size={14} /></button></div>
      {nodes.map((node) => <button className={selectedNodeId === node.id ? "construct-row selected" : "construct-row"} key={node.id} onClick={() => setSelectedNode(node.id)}>
        <span className={`mode-dot ${node.data.mode}`} />
        <span>{node.data.label}</span><small>{node.data.shortName}</small>
      </button>)}
    </div>
    <div className="variables-block">
      <div className="section-label"><span>Dataset variables</span><small>{dataset.columns.length}</small></div>
      <label className="variable-search"><Search size={13} /><input aria-label="Search dataset variables" placeholder="Find variable" value={query} onChange={(event) => setQuery(event.target.value)} /></label>
      {selectedVariables.length > 0 && <div className="variable-bulk-actions">
        <span>{selectedVariables.length} selected</span>
        <button title="Create construct from selected variables" onClick={() => { addConstruct(undefined, selectedVariables); setSelectedVariables([]); }}><Plus size={13} />Construct</button>
        <button title="Create constructs grouped by variable prefix" onClick={() => { addConstructsFromIndicatorGroups(selectedVariables); setSelectedVariables([]); }}><Boxes size={13} />Group</button>
        <button title="Create one construct per selected variable" onClick={() => { addConstructsFromIndicators(selectedVariables); setSelectedVariables([]); }}><Boxes size={13} />Separate</button>
        <button title="Assign selected variables to the selected construct" disabled={!selectedNodeId} onClick={() => { if (selectedNodeId) assignIndicators(selectedNodeId, selectedVariables); setSelectedVariables([]); }}><Network size={13} />Assign</button>
        <button title="Clear variable selection" onClick={() => setSelectedVariables([])}><X size={13} /></button>
      </div>}
      <div className="variable-list">
        {variables.map((variable) => {
          const owner = ownerByIndicator.get(variable);
          const checked = selectedVariables.includes(variable);
          return <div
            key={variable}
            className={`${owner ? "variable-row assigned" : "variable-row"}${checked ? " selected" : ""}`}
            draggable
            title={`Drag ${checked ? "selected variables" : variable} onto a construct or empty canvas`}
            onDragStart={(event) => {
              const dragged = checked ? selectedVariables : [variable];
              event.dataTransfer.setData("application/qpls-indicator", variable);
              event.dataTransfer.setData("application/qpls-indicators", JSON.stringify(dragged));
              event.dataTransfer.effectAllowed = "move";
            }}
          >
            <GripVertical size={12} />
            <input type="checkbox" aria-label={`Select ${variable}`} checked={checked} onChange={() => setSelectedVariables((current) => current.includes(variable) ? current.filter((item) => item !== variable) : [...current, variable])} />
            <button onClick={() => setSelectedVariables((current) => current.includes(variable) ? current.filter((item) => item !== variable) : [...current, variable])}><span>{variable}</span><small>{owner ?? "Unassigned"}</small></button>
          </div>;
        })}
      </div>
    </div>
    <div className="model-summary">
      <h3>Model overview</h3>
      <dl><dt>Constructs</dt><dd>{nodes.length}</dd><dt>Indicators</dt><dd>{nodes.reduce((sum, node) => sum + node.data.indicators.length, 0)}</dd><dt>Paths</dt><dd>{edges.length}</dd><dt>Sample rows</dt><dd>{dataset.rowCount ?? dataset.rows.length}</dd><dt>Missing cells</dt><dd>{dataset.missing}</dd><dt>Weighting</dt><dd>Path</dd></dl>
    </div>
  </aside>;
}
