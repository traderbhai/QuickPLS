import { AlertTriangle, CheckCircle2, Database, WifiOff } from "lucide-react";
import { useWorkspace } from "../store";
import { validateModel } from "../domain/modelValidation";
import { analysisReadiness } from "../domain/analysisReadiness";
import { isNativeDesktop } from "../services/projectService";

export function StatusBar() {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const dataset = useWorkspace((state) => state.dataset);
  const settings = useWorkspace((state) => state.analysisSettings);
  const projectPath = useWorkspace((state) => state.projectPath);
  const setShortcutOverlayOpen = useWorkspace((state) => state.setShortcutOverlayOpen);
  const issues = validateModel(nodes, edges);
  const readiness = analysisReadiness({ dataset, nodes, edges, settings, nativeDesktop: isNativeDesktop() });
  const structuralPathCount = edges.filter((edge) => edge.data?.role !== "covariance").length;
  return <footer className="status-bar">
    <span className={readiness.canRun ? "status valid" : issues.length ? "status warning" : "status warning"}>{readiness.canRun ? <CheckCircle2 size={15} /> : <AlertTriangle size={15} />}{readiness.summary}</span>
    <span className="status-readiness-strip" aria-label="Persistent analysis readiness checklist">
      {readiness.items.map((item) => <span key={item.id} className={`status-readiness-pill ${item.status}`} title={item.detail} aria-label={`${item.label}: ${item.detail}`}>
        {item.label}
      </span>)}
    </span>
    <span><Database size={14} />{dataset.rows.length} rows</span><span>{nodes.length} constructs</span><span>{structuralPathCount} paths</span>
    <span>{projectPath ? "Autosave active" : "Save project to enable autosave"}</span>
    <span className="status-spacer" /><button className="status-link" onClick={() => setShortcutOverlayOpen(true)}>Shortcuts ?</button><span><WifiOff size={14} />Offline mode</span><span>Engine 1.0.0 stable scope</span>
  </footer>;
}
