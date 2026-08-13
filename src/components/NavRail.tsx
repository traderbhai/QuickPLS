import { ClipboardCheck, Database, FileText, Home, Network, Play, PlayCircle, Settings, ShieldCheck } from "lucide-react";
import type { WorkspaceView } from "../types";
import { useWorkspace } from "../store";

type RailItem = { view: WorkspaceView; label: string; tooltip: string; Icon: typeof Database };

const workflowItems: RailItem[] = [
  { view: "welcome", label: "Home", tooltip: "Project start, recent projects, demo, and recovery", Icon: Home },
  { view: "data", label: "Data", tooltip: "Import, inspect, metadata, and missing values", Icon: Database },
  { view: "models", label: "Model", tooltip: "SEM diagram designer", Icon: Network },
  { view: "analyses", label: "Setup", tooltip: "Method selection, validation, and readiness", Icon: ClipboardCheck },
  { view: "run", label: "Run", tooltip: "Execute analysis and monitor jobs", Icon: Play },
  { view: "runs", label: "Results", tooltip: "Review saved runs and tables", Icon: PlayCircle },
  { view: "reports", label: "Report", tooltip: "Export figures, tables, and reproducibility report", Icon: FileText },
];

const utilityItems: RailItem[] = [
  { view: "trust", label: "Trust", tooltip: "Validation evidence, method scope, and known limitations", Icon: ShieldCheck },
  { view: "settings", label: "Settings", tooltip: "Desktop preferences, density, precision, and offline behavior", Icon: Settings },
];

export function NavRail() {
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.setView);
  return <nav className="nav-rail" aria-label="Workspace">
    <RailGroup label="Research workflow" items={workflowItems} currentView={view} setView={setView} />
    <RailGroup label="Support" items={utilityItems} currentView={view} setView={setView} utility />
  </nav>;
}

function RailGroup({ label, items, currentView, setView, utility = false }: { label: string; items: RailItem[]; currentView: WorkspaceView; setView: (view: WorkspaceView) => void; utility?: boolean }) {
  return <div className={utility ? "nav-section utility" : "nav-section"} aria-label={label} data-nav-section={label}>
    <span className="nav-section-label">{label}</span>
    {items.map(({ view: itemView, label: itemLabel, tooltip, Icon }) => (
      <button key={itemView} className={currentView === itemView ? "nav-item active" : "nav-item"} aria-current={currentView === itemView ? "page" : undefined} onClick={() => setView(itemView)} title={`${itemLabel}: ${tooltip}`} aria-label={`${itemLabel}: ${tooltip}`} data-nav-view={itemView}>
        <Icon size={21} strokeWidth={1.8} />
        <span>{itemLabel}</span>
      </button>
    ))}
  </div>;
}
