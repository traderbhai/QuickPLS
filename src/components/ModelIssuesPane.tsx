import { AlertTriangle, CheckCircle2, Focus, GitBranch, MousePointer2, Network, PanelBottom, ShieldCheck } from "lucide-react";
import { useMemo } from "react";
import { type Edge } from "@xyflow/react";
import { analysisReadiness } from "../domain/analysisReadiness";
import { isNativeDesktop } from "../services/projectService";
import { useWorkspace } from "../store";

function pathLabel(edge: Edge, labels: Map<string, string>) {
  const source = labels.get(edge.source) ?? edge.source;
  const target = labels.get(edge.target) ?? edge.target;
  return `${source} ${edge.data?.role === "covariance" ? "<->" : "->"} ${target}`;
}

export function ModelIssuesPane() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const settings = useWorkspace((state) => state.analysisSettings);
  const selectedNodeId = useWorkspace((state) => state.selectedNodeId);
  const selectedEdgeId = useWorkspace((state) => state.selectedEdgeId);
  const selectedResultRunId = useWorkspace((state) => state.selectedResultRunId);
  const runs = useWorkspace((state) => state.runs);
  const setView = useWorkspace((state) => state.setView);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const setSelectedNode = useWorkspace((state) => state.setSelectedNode);
  const setSelectedEdge = useWorkspace((state) => state.setSelectedEdge);
  const readiness = useMemo(() => analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() }), [dataset, edges, nodes, settings]);
  const labelById = useMemo(() => new Map(nodes.map((node) => [node.id, node.data.shortName || node.data.label])), [nodes]);
  const selectedNode = selectedNodeId ? nodes.find((node) => node.id === selectedNodeId) : null;
  const selectedEdge = selectedEdgeId ? edges.find((edge) => edge.id === selectedEdgeId) : null;
  const structuralEdges = edges.filter((edge) => edge.data?.role !== "covariance" && !edge.id.startsWith("measurement::"));
  const covarianceEdges = edges.filter((edge) => edge.data?.role === "covariance");
  const selectedRun = selectedResultRunId ? runs.find((run) => run.id === selectedResultRunId) : null;
  const publicationIssues = useMemo(() => {
    const issues: Array<{ tone: "ok" | "warn" | "issue"; label: string; detail: string }> = [];
    const offCanvas = nodes.filter((node) => node.position.x < -30 || node.position.y < -30);
    if (offCanvas.length) issues.push({ tone: "issue", label: "Off-canvas objects", detail: `${offCanvas.length} construct(s) are outside the normal publication area.` });
    const missingIndicators = nodes.filter((node) => node.data.indicators.length === 0);
    if (missingIndicators.length) issues.push({ tone: "issue", label: "Missing indicators", detail: `${missingIndicators.length} construct(s) need observed indicators before analysis.` });
    if (structuralEdges.length > 12) issues.push({ tone: "warn", label: "Dense structure", detail: "Many structural paths may need Focus Diagram or a tidy publication layout." });
    if (!selectedRun && selectedResultRunId) issues.push({ tone: "warn", label: "Stale overlay", detail: "The selected run is not available, so result labels are suppressed." });
    if (!issues.length) issues.push({ tone: "ok", label: "Publication check", detail: "No obvious off-canvas, missing-indicator, or stale-overlay issue is detected." });
    return issues;
  }, [nodes, selectedResultRunId, selectedRun, structuralEdges.length]);

  const primaryIssue = readiness.blockers[0] ?? readiness.warnings[0] ?? null;

  return <section className="model-v225-bottom-pane" data-v225-model-workbench="issues-output-pane" aria-label="Model issues and output pane">
    <div className="model-v225-pane-section">
      <header><PanelBottom size={14} /><strong>Model issues</strong><span>{readiness.canRun ? "ready" : `${readiness.blockers.length} blocker(s)`}</span></header>
      {primaryIssue ? <button type="button" className="model-v225-issue-row" onClick={() => primaryIssue.actionView ? setView(primaryIssue.actionView) : undefined}>
        <AlertTriangle size={14} />
        <span><b>{primaryIssue.label}</b><small>{primaryIssue.detail}</small></span>
      </button> : <div className="model-v225-ok-row"><CheckCircle2 size={14} /><span>Diagram is structurally ready for the selected method.</span></div>}
    </div>
    <div className="model-v225-pane-section">
      <header><MousePointer2 size={14} /><strong>Selection</strong><span>{selectedNode ? "construct" : selectedEdge ? "path" : "none"}</span></header>
      {selectedNode ? <div className="model-v225-selection">
        <b>{selectedNode.data.label}</b>
        <small>{selectedNode.data.shortName} | {selectedNode.data.mode} | {selectedNode.data.indicators.length} indicator(s)</small>
      </div> : selectedEdge ? <div className="model-v225-selection">
        <b>{selectedEdge.data?.role === "covariance" ? "Covariance" : "Structural path"}</b>
        <small>{pathLabel(selectedEdge, labelById)}</small>
      </div> : <div className="model-v225-selection muted">Select a construct, indicator, or path to see object-specific actions.</div>}
    </div>
    <div className="model-v225-pane-section">
      <header><ShieldCheck size={14} /><strong>Publication check</strong><span>{publicationIssues[0]?.tone === "ok" ? "clear" : "review"}</span></header>
      <div className="model-v225-publication-list">
        {publicationIssues.slice(0, 2).map((item) => <span key={item.label} className={item.tone}><b>{item.label}</b><small>{item.detail}</small></span>)}
      </div>
    </div>
    <div className="model-v225-pane-section actions">
      <header><Network size={14} /><strong>Workbench actions</strong><span>{structuralEdges.length} path(s) | {covarianceEdges.length} covariance</span></header>
      <div>
        <button type="button" onClick={() => autoLayout("smartpls")}><GitBranch size={13} />Arrange model</button>
        <button type="button" onClick={() => setView("analyses")}>Setup calculation</button>
        <button type="button" onClick={() => {
          const firstIssueNode = nodes.find((node) => node.data.indicators.length === 0) ?? nodes[0];
          if (firstIssueNode) {
            setSelectedNode(firstIssueNode.id);
            window.dispatchEvent(new CustomEvent("quickpls:focus-construct", { detail: { id: firstIssueNode.id } }));
          } else if (edges[0]) {
            setSelectedEdge(edges[0].id);
            window.dispatchEvent(new CustomEvent("quickpls:focus-edge", { detail: { id: edges[0].id } }));
          }
        }}><Focus size={13} />Focus issue</button>
      </div>
    </div>
  </section>;
}
