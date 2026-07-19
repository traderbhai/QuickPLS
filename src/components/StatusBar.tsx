import { AlertTriangle, CheckCircle2, Database, WifiOff } from "lucide-react";
import { useWorkspace } from "../store";
import { validateModel } from "../domain/modelValidation";

export function StatusBar() {
  const nodes = useWorkspace((state) => state.nodes);
  const edges = useWorkspace((state) => state.edges);
  const dataset = useWorkspace((state) => state.dataset);
  const issues = validateModel(nodes, edges);
  return <footer className="status-bar">
    <span className={issues.length ? "status warning" : "status valid"}>{issues.length ? <AlertTriangle size={15} /> : <CheckCircle2 size={15} />}{issues.length ? `${issues.length} model issues` : "Model structure valid"}</span>
    <span><Database size={14} />{dataset.rows.length} rows</span><span>{nodes.length} constructs</span><span>{edges.length} paths</span>
    <span className="status-spacer" /><span><WifiOff size={14} />Offline mode</span><span>Engine 0.1.0-alpha</span>
  </footer>;
}
