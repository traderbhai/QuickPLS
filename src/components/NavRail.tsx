import { ClipboardCheck, Database, FileText, Home, Network, Play, PlayCircle } from "lucide-react";
import type { WorkspaceView } from "../types";
import { useWorkspace } from "../store";

const items: Array<{ view: WorkspaceView; label: string; tooltip: string; Icon: typeof Database }> = [
  { view: "welcome", label: "Home", tooltip: "Project start, recent projects, demo, and recovery", Icon: Home },
  { view: "data", label: "Data", tooltip: "Import, inspect, metadata, and missing values", Icon: Database },
  { view: "models", label: "Model", tooltip: "SEM diagram designer", Icon: Network },
  { view: "analyses", label: "Setup", tooltip: "Method selection, validation, and readiness", Icon: ClipboardCheck },
  { view: "run", label: "Run", tooltip: "Execute analysis and monitor jobs", Icon: Play },
  { view: "runs", label: "Results", tooltip: "Review saved runs and tables", Icon: PlayCircle },
  { view: "reports", label: "Report", tooltip: "Export figures, tables, and reproducibility report", Icon: FileText },
];

export function NavRail() {
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.setView);
  return <nav className="nav-rail" aria-label="Workspace">
    {items.map(({ view: itemView, label, tooltip, Icon }) => (
      <button key={itemView} className={view === itemView ? "nav-item active" : "nav-item"} aria-current={view === itemView ? "page" : undefined} onClick={() => setView(itemView)} title={`${label}: ${tooltip}`} aria-label={`${label}: ${tooltip}`}>
        <Icon size={21} strokeWidth={1.8} />
        <span>{label}</span>
      </button>
    ))}
  </nav>;
}
