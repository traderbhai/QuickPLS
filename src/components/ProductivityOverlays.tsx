import { BookOpen, CheckCircle2, Command, Database, Download, FileText, FlaskConical, FolderOpen, Keyboard, Network, Play, Search, Settings, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useWorkspace } from "../store";
import type { WorkspaceView } from "../types";

const shortcuts = [
  ["Ctrl+K", "Open command palette"],
  ["?", "Show keyboard shortcuts"],
  ["V", "Select tool"],
  ["P", "Path tool"],
  ["C", "Covariance tool"],
  ["F", "Fit diagram to view"],
  ["Delete", "Delete selected object"],
  ["Ctrl+Z", "Undo diagram change"],
  ["Ctrl+Y", "Redo diagram change"],
  ["Esc", "Close menus or cancel active tool"],
] as const;

export function ProductivityOverlays() {
  const commandPaletteOpen = useWorkspace((state) => state.commandPaletteOpen);
  const shortcutOverlayOpen = useWorkspace((state) => state.shortcutOverlayOpen);
  const toasts = useWorkspace((state) => state.toasts);
  const setCommandPaletteOpen = useWorkspace((state) => state.setCommandPaletteOpen);
  const setShortcutOverlayOpen = useWorkspace((state) => state.setShortcutOverlayOpen);
  const dismissToast = useWorkspace((state) => state.dismissToast);
  const setView = useWorkspace((state) => state.setView);
  const applyMethodPreset = useWorkspace((state) => state.applyMethodPreset);
  const setDiagramTool = useWorkspace((state) => state.setDiagramTool);
  const setDiagramMode = useWorkspace((state) => state.setDiagramMode);
  const autoLayout = useWorkspace((state) => state.autoLayout);
  const datasetColumnCount = useWorkspace((state) => state.dataset.columns.length);
  const constructCount = useWorkspace((state) => state.nodes.length);
  const runCount = useWorkspace((state) => state.runs.length);
  const [query, setQuery] = useState("");

  useEffect(() => {
    const handleKeys = (event: KeyboardEvent) => {
      const target = event.target;
      const editing = target instanceof HTMLElement ? target.matches("input, textarea, select, [contenteditable='true']") : false;
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandPaletteOpen(true);
        return;
      }
      if (!editing && event.key === "?") {
        event.preventDefault();
        setShortcutOverlayOpen(true);
      }
      if (event.key === "Escape") {
        setCommandPaletteOpen(false);
        setShortcutOverlayOpen(false);
      }
    };
    window.addEventListener("keydown", handleKeys);
    return () => window.removeEventListener("keydown", handleKeys);
  }, [setCommandPaletteOpen, setShortcutOverlayOpen]);

  useEffect(() => {
    if (!toasts.length) return;
    const timer = window.setTimeout(() => dismissToast(toasts[toasts.length - 1].id), 5200);
    return () => window.clearTimeout(timer);
  }, [dismissToast, toasts]);

  const commands = useMemo(() => [
    { label: "Start workspace", detail: "Open the desktop-first start page", icon: BookOpen, action: () => setView("welcome" as WorkspaceView) },
    { label: "Import dataset", detail: `${datasetColumnCount} columns currently loaded`, icon: Database, action: () => setView("data" as WorkspaceView) },
    { label: "Open SEM designer", detail: `${constructCount} constructs in the current model`, icon: Network, action: () => setView("models" as WorkspaceView) },
    { label: "Use Path tool", detail: "Draw structural paths between constructs", icon: Network, action: () => { setView("models" as WorkspaceView); setDiagramTool("path"); } },
    { label: "Arrange like SmartPLS", detail: "Tidy the SEM diagram left-to-right", icon: Network, action: () => { setView("models" as WorkspaceView); autoLayout("smartpls"); } },
    { label: "Setup PLS + Bootstrap", detail: "Apply recommended bootstrap preset", icon: FlaskConical, action: () => { applyMethodPreset("pls_bootstrap"); setView("analyses" as WorkspaceView); } },
    { label: "Setup MICOM + MGA", detail: "Open group-analysis preset", icon: Settings, action: () => { applyMethodPreset("micom_mga"); setView("analyses" as WorkspaceView); } },
    { label: "Open Run checklist", detail: "Review readiness and launch analysis", icon: Play, action: () => setView("run" as WorkspaceView) },
    { label: "Open Results", detail: `${runCount} saved runs`, icon: FileText, action: () => setView("runs" as WorkspaceView) },
    { label: "Open Publication export", detail: "Preview diagram and table exports", icon: Download, action: () => setView("reports" as WorkspaceView) },
    { label: "Publication preview mode", detail: "Lock canvas into publication figure view", icon: FileText, action: () => { setView("models" as WorkspaceView); setDiagramMode("publication"); } },
    { label: "Show keyboard shortcuts", detail: "Open shortcut overlay", icon: Keyboard, action: () => setShortcutOverlayOpen(true) },
  ], [applyMethodPreset, autoLayout, constructCount, datasetColumnCount, runCount, setDiagramMode, setDiagramTool, setShortcutOverlayOpen, setView]);
  const filteredCommands = commands.filter((command) => `${command.label} ${command.detail}`.toLowerCase().includes(query.toLowerCase()));
  const runCommand = (action: () => void) => {
    action();
    setCommandPaletteOpen(false);
    setQuery("");
  };

  return <>
    {commandPaletteOpen ? <div className="overlay-backdrop command-backdrop" role="dialog" aria-modal="true" aria-label="Command palette">
      <section className="command-palette">
        <header><Command size={18} /><strong>Quick actions</strong><button aria-label="Close command palette" onClick={() => setCommandPaletteOpen(false)}><X size={16} /></button></header>
        <label className="command-search"><Search size={15} /><input autoFocus placeholder="Search commands, views, presets, exports" value={query} onChange={(event) => setQuery(event.target.value)} /></label>
        <div className="command-list">
          {filteredCommands.map((command) => {
            const Icon = command.icon;
            return <button key={command.label} onClick={() => runCommand(command.action)}>
              <Icon size={16} />
              <span><strong>{command.label}</strong><small>{command.detail}</small></span>
            </button>;
          })}
          {filteredCommands.length === 0 ? <p>No matching command.</p> : null}
        </div>
      </section>
    </div> : null}

    {shortcutOverlayOpen ? <div className="overlay-backdrop" role="dialog" aria-modal="true" aria-label="Keyboard shortcuts">
      <section className="shortcut-panel">
        <header><Keyboard size={18} /><strong>Keyboard shortcuts</strong><button aria-label="Close keyboard shortcuts" onClick={() => setShortcutOverlayOpen(false)}><X size={16} /></button></header>
        <div>{shortcuts.map(([key, label]) => <p key={key}><kbd>{key}</kbd><span>{label}</span></p>)}</div>
      </section>
    </div> : null}

    <aside className="toast-stack" aria-live="polite" aria-label="Application notifications">
      {toasts.map((toast) => <article key={toast.id} className={`toast ${toast.tone}`}>
        <CheckCircle2 size={15} />
        <span><strong>{toast.title}</strong>{toast.detail ? <small>{toast.detail}</small> : null}</span>
        <button aria-label={`Dismiss ${toast.title}`} onClick={() => dismissToast(toast.id)}><X size={13} /></button>
      </article>)}
    </aside>
  </>;
}
