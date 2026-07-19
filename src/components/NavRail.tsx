import { BarChart3, Database, FileText, Network, PlayCircle, Users } from "lucide-react";
import type { WorkspaceView } from "../types";
import { useWorkspace } from "../store";

const items: Array<{ view: WorkspaceView; label: string; Icon: typeof Database }> = [
  { view: "data", label: "Data", Icon: Database },
  { view: "models", label: "Models", Icon: Network },
  { view: "analyses", label: "Analyses", Icon: BarChart3 },
  { view: "runs", label: "Runs", Icon: PlayCircle },
  { view: "groups", label: "Groups", Icon: Users },
  { view: "reports", label: "Reports", Icon: FileText },
];

export function NavRail() {
  const view = useWorkspace((state) => state.view);
  const setView = useWorkspace((state) => state.setView);
  return <nav className="nav-rail" aria-label="Workspace">
    {items.map(({ view: itemView, label, Icon }) => (
      <button key={itemView} className={view === itemView ? "nav-item active" : "nav-item"} onClick={() => setView(itemView)} title={label}>
        <Icon size={21} strokeWidth={1.8} />
        <span>{label}</span>
      </button>
    ))}
  </nav>;
}
