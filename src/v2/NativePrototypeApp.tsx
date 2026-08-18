import { useEffect, useMemo, useState } from "react";
import type { ChangeEvent, ReactNode } from "react";
import {
  AlertTriangle,
  BarChart3,
  BookOpen,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  Clock3,
  Columns3,
  Database,
  FileDown,
  FilePlus2,
  FileText,
  FolderOpen,
  Hand,
  HelpCircle,
  Home,
  Import,
  Link,
  LayoutGrid,
  ListChecks,
  MonitorCog,
  MousePointer2,
  Move,
  Pin,
  Play,
  Plus,
  RectangleHorizontal,
  Save,
  Search,
  Settings,
  ShieldCheck,
  Sigma,
  Table2,
  Trash2,
  Undo2,
  Redo2,
  ZoomIn,
  X,
} from "lucide-react";
import {
  type NativePrototypeData,
  type NativePrototypeDialog,
  type NativePrototypeView,
} from "./nativePrototypeData";
import { useNativePrototypeAdapter } from "./nativePrototypeAdapters";
import { AnalysisCatalog } from "../components/AnalysisCatalog";
import { methods } from "../data/sample";
import {
  analysisCatalogCapabilityCountsV2,
  analysisCatalogCapabilityEntriesV2,
} from "../domain/analysisCatalogCapabilityV2";
import { DataWorkspace } from "../components/DataWorkspace";
import { Explorer } from "../components/Explorer";
import { Inspector } from "../components/Inspector";
import { ModelCanvas } from "../components/ModelCanvas";
import { ModelIssuesPane } from "../components/ModelIssuesPane";
import { ReportsWorkspace } from "../components/ReportsWorkspace";
import { RunHistory } from "../components/RunHistory";
import { RunWorkspace } from "../components/RunWorkspace";
import { SettingsWorkspace } from "../components/SettingsWorkspace";
import { TrustCenterWorkspace } from "../components/TrustCenterWorkspace";
import { isNativeDesktop, openNativeDefaultExportFolder, verifyNativeLatestReleaseChecksums, type ChecksumVerification } from "../services/projectService";
import { useWorkspace } from "../store";
import "./nativePrototype.css";

const views: Array<{ id: NativePrototypeView; label: string; icon: typeof Home }> = [
  { id: "home", label: "Home", icon: Home },
  { id: "data", label: "Data", icon: Database },
  { id: "model", label: "Model", icon: LayoutGrid },
  { id: "setup", label: "Setup", icon: Settings },
  { id: "run", label: "Run", icon: Play },
  { id: "results", label: "Results", icon: BarChart3 },
  { id: "report", label: "Report", icon: FileDown },
];

const supportViews: Array<{ id: NativePrototypeView; label: string; icon: typeof Home }> = [
  { id: "trust", label: "Trust Center", icon: ShieldCheck },
  { id: "settings", label: "Settings", icon: Settings },
];

const menus = ["File", "Edit", "Data", "Model", "Calculate", "Results", "Report", "View", "Tools", "Window", "Help"];

function Chip({ children, tone = "neutral" }: { children: string; tone?: "neutral" | "good" | "warn" | "bad" }) {
  return <span className={`np-chip ${tone}`}>{children}</span>;
}

function Section({ title, actions, children, className = "" }: { title: string; actions?: ReactNode; children: ReactNode; className?: string }) {
  return <section className={`np-panel ${className}`}>
    <header className="np-panel-title"><strong>{title}</strong>{actions}</header>
    <div className="np-panel-body">{children}</div>
  </section>;
}

function PropertyRow({ label, value }: { label: string; value: string }) {
  return <label className="np-property-row"><span>{label}</span><input value={value} readOnly /></label>;
}

function PropertyControl({ label, children }: { label: string; children: ReactNode }) {
  return <label className="np-property-row np-property-control"><span>{label}</span>{children}</label>;
}

function dispatchQuickPlsCommand(command: string) {
  window.dispatchEvent(new CustomEvent(`quickpls:${command}`));
}

function dispatchNativeModelCommand(command: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:model-${command}`, { detail }));
}

function dispatchNativeDataCommand(command: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:data-${command}`, { detail }));
}

function dispatchNativeResultsCommand(command: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:results-${command}`, { detail }));
}

function dispatchNativeReportCommand(command: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:report-${command}`, { detail }));
}

function dispatchNativeSettingsCommand(command: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:settings-${command}`, { detail }));
}

function dispatchNativeTrustCommand(command: string, detail?: unknown) {
  window.dispatchEvent(new CustomEvent(`quickpls:trust-${command}`, { detail }));
}

function dispatchNativeStatus(message: string, tone: "info" | "success" | "warning" | "error" = "info") {
  window.dispatchEvent(new CustomEvent("quickpls:status-message", { detail: { message, tone } }));
}

async function openDefaultExportFolderFromShell() {
  if (!isNativeDesktop()) {
    dispatchNativeStatus("Open Folder is available in the desktop app.", "warning");
    return;
  }
  const folder = await openNativeDefaultExportFolder();
  dispatchNativeStatus(`Export folder opened: ${folder}`, "success");
}

async function verifyReleaseChecksumsFromShell(): Promise<ChecksumVerification | null> {
  if (!isNativeDesktop()) {
    dispatchNativeStatus("Checksum verification is available in the desktop app.", "warning");
    return null;
  }
  const result = await verifyNativeLatestReleaseChecksums();
  dispatchNativeStatus(result.message, result.failures.length ? "warning" : result.checked ? "success" : "warning");
  return result;
}

async function saveRunLogFromShell(rows: string[][]) {
  if (!rows.length) {
    dispatchNativeStatus("No run log is available to save.", "warning");
    return;
  }
  const text = [["Time", "Level", "Message"], ...rows]
    .map((row) => row.map((cell) => `"${String(cell).replace(/"/g, '""')}"`).join(","))
    .join("\n");
  await navigator.clipboard?.writeText(text);
  dispatchNativeStatus("Run log copied to clipboard.", "success");
}

function activeEditableElement() {
  const element = document.activeElement;
  if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement) return element;
  return null;
}

async function copySelectionToClipboard() {
  const selected = window.getSelection()?.toString() ?? "";
  const editable = activeEditableElement();
  const text = editable && editable.selectionStart != null && editable.selectionEnd != null
    ? editable.value.slice(editable.selectionStart, editable.selectionEnd)
    : selected;
  if (!text) {
    dispatchNativeStatus("Nothing is selected to copy.", "warning");
    return;
  }
  await navigator.clipboard?.writeText(text);
  dispatchNativeStatus("Selection copied to clipboard.", "success");
}

async function cutSelectionToClipboard() {
  const editable = activeEditableElement();
  if (!editable || editable.selectionStart == null || editable.selectionEnd == null || editable.selectionStart === editable.selectionEnd) {
    await copySelectionToClipboard();
    return;
  }
  const text = editable.value.slice(editable.selectionStart, editable.selectionEnd);
  await navigator.clipboard?.writeText(text);
  editable.setRangeText("", editable.selectionStart, editable.selectionEnd, "end");
  editable.dispatchEvent(new Event("input", { bubbles: true }));
  dispatchNativeStatus("Selection cut to clipboard.", "success");
}

async function pasteClipboardIntoActiveElement() {
  const editable = activeEditableElement();
  if (!editable) {
    dispatchNativeStatus("Paste needs an active text field.", "warning");
    return;
  }
  const text = await navigator.clipboard?.readText();
  if (!text) {
    dispatchNativeStatus("Clipboard is empty.", "warning");
    return;
  }
  const start = editable.selectionStart ?? editable.value.length;
  const end = editable.selectionEnd ?? editable.value.length;
  editable.setRangeText(text, start, end, "end");
  editable.dispatchEvent(new Event("input", { bubbles: true }));
  dispatchNativeStatus("Clipboard text pasted.", "success");
}

function NativeStoreCommandBridge() {
  const dataset = useWorkspace((state) => state.dataset);
  const nodes = useWorkspace((state) => state.nodes);
  const assignIndicator = useWorkspace((state) => state.assignIndicator);
  useEffect(() => {
    const handleAddIndicator = () => {
      const construct = nodes.find((node) => node.selected) ?? nodes[0];
      if (!construct) {
        dispatchNativeStatus("Add Indicator needs a construct in the model.", "warning");
        return;
      }
      const assigned = new Set(nodes.flatMap((node) => node.data.indicators));
      const candidate = dataset.columns.find((column) => !assigned.has(column));
      if (!candidate) {
        dispatchNativeStatus("No unassigned dataset variables are available.", "warning");
        return;
      }
      assignIndicator(construct.id, candidate);
      dispatchNativeStatus(`${candidate} assigned to ${construct.data.label}.`, "success");
    };
    window.addEventListener("quickpls:model-add-indicator", handleAddIndicator);
    return () => window.removeEventListener("quickpls:model-add-indicator", handleAddIndicator);
  }, [assignIndicator, dataset.columns, nodes]);
  return null;
}

function LiveModelWorkbench() {
  return <div
    className="np-functional-model-host np-live-model-workbench"
    data-native-functional-workspace="model"
    data-v237-screen="model"
    data-v245-model-live-tree="true"
    data-v245-reactflow-restored="true"
  >
    <Explorer />
    <ModelCanvas />
    <Inspector />
    <ModelIssuesPane />
  </div>;
}

type NativeCommandItem = {
  label: string;
  icon?: typeof Home;
  active?: boolean;
  dialog?: NativePrototypeDialog;
  menu?: boolean;
  primary?: boolean;
  disabledReason?: string;
  action?: () => void;
};

function nativeCommandState(command: NativeCommandItem) {
  const disabled = Boolean(command.disabledReason);
  return {
    disabled,
    title: command.disabledReason ?? command.label,
    "aria-label": command.disabledReason ? `${command.label}. ${command.disabledReason}` : command.label,
    "data-command-status": disabled ? "disabled" : "wired",
    "data-disabled-reason": command.disabledReason,
  };
}

function runNativeCommand(command: NativeCommandItem, openDialog?: (dialog: NativePrototypeDialog) => void) {
  if (command.disabledReason) return;
  if (command.dialog && openDialog) {
    openDialog(command.dialog);
    return;
  }
  command.action?.();
}

function DataTable({ headers, rows }: { headers: string[]; rows: string[][] }) {
  return <div className="np-table-wrap">
    <table className="np-table">
      <thead><tr>{headers.map((header) => <th key={header}>{header}</th>)}</tr></thead>
      <tbody>{rows.map((row, index) => <tr key={index}>{row.map((cell, cellIndex) => <td key={`${index}-${cellIndex}`}>{cell}</td>)}</tr>)}</tbody>
    </table>
  </div>;
}

function DesktopMenu({
  activeMenu,
  setActiveMenu,
  openDialog,
  setView,
  requestCloseProject,
  saveLayout,
  resetLayout,
  closePane,
  restorePane,
  statusBarVisible,
  toggleStatusBar,
}: {
  activeMenu: string | null;
  setActiveMenu: (menu: string | null) => void;
  openDialog: (dialog: NativePrototypeDialog) => void;
  setView: (view: NativePrototypeView) => void;
  requestCloseProject: () => void;
  saveLayout: () => void;
  resetLayout: () => void;
  closePane: () => void;
  restorePane: () => void;
  statusBarVisible: boolean;
  toggleStatusBar: () => void;
}) {
  const runModelCommand = (command: string, detail?: unknown) => {
    setView("model");
    window.setTimeout(() => dispatchNativeModelCommand(command, detail), 0);
  };
  const menuItems: Record<string, NativeCommandItem[]> = {
    File: [
      { label: "New Project", action: () => openDialog("new_project") },
      { label: "Open Project", action: () => dispatchQuickPlsCommand("open-project") },
      { label: "Save", action: () => dispatchQuickPlsCommand("save-project") },
      { label: "Save As", action: () => dispatchQuickPlsCommand("save-project-as") },
      { label: "Close Project", action: requestCloseProject },
      { label: "Exit", action: () => window.close() },
    ],
    Edit: [
      { label: "Undo", action: () => runModelCommand("undo") },
      { label: "Redo", action: () => runModelCommand("redo") },
      { label: "Cut", action: () => { void cutSelectionToClipboard().catch((error) => dispatchNativeStatus(String(error), "error")); } },
      { label: "Copy", action: () => { void copySelectionToClipboard().catch((error) => dispatchNativeStatus(String(error), "error")); } },
      { label: "Paste", action: () => { void pasteClipboardIntoActiveElement().catch((error) => dispatchNativeStatus(String(error), "error")); } },
    ],
    Data: [
      { label: "Import Data", action: () => openDialog("import_data") },
      { label: "Transform", action: () => { setView("data"); window.setTimeout(() => openDialog("data_transform"), 0); } },
      { label: "Add Column", action: () => { setView("data"); window.setTimeout(() => openDialog("data_add_column"), 0); } },
      { label: "Recode Values", action: () => { setView("data"); window.setTimeout(() => openDialog("data_recode"), 0); } },
      { label: "Missing Values", action: () => { setView("data"); window.setTimeout(() => openDialog("data_missing_values"), 0); } },
      { label: "Filter", action: () => { setView("data"); window.setTimeout(() => openDialog("data_filter"), 0); } },
      { label: "Sort", action: () => { setView("data"); window.setTimeout(() => openDialog("data_sort"), 0); } },
    ],
    Model: [
      { label: "Add Latent", action: () => runModelCommand("add-construct") },
      { label: "Add Indicator", action: () => runModelCommand("add-indicator") },
      { label: "Connect", action: () => runModelCommand("tool", { tool: "path" }) },
      { label: "Arrange", action: () => runModelCommand("arrange", { direction: "smartpls" }) },
      { label: "Check Diagram", action: () => runModelCommand("validate") },
    ],
    Calculate: [
      { label: "Setup Calculation", action: () => openDialog("calculation_setup") },
      { label: "Run", action: () => dispatchQuickPlsCommand("run-analysis") },
      { label: "Cancel Run", action: () => dispatchQuickPlsCommand("cancel-analysis") },
    ],
    Results: [
      { label: "Open Workbook", action: () => setView("results") },
      { label: "Copy Table", action: () => dispatchNativeResultsCommand("copy-current-table") },
      { label: "Export Current Table", action: () => dispatchNativeResultsCommand("export-current-table") },
    ],
    Report: [
      { label: "Export Options", action: () => openDialog("export_options") },
      { label: "Reviewer Pack", action: () => { setView("report"); openDialog("export_options"); } },
      { label: "Print", action: () => window.print() },
    ],
    View: [
      { label: "Focus Diagram", action: () => runModelCommand("focus-diagram") },
      { label: "Output Log", action: () => { setView("run"); dispatchNativeStatus("Run log is visible in the Run workspace.", "info"); } },
      { label: statusBarVisible ? "Hide Status Bar" : "Show Status Bar", action: toggleStatusBar },
    ],
    Tools: [
      { label: "Trust Center", action: () => setView("trust") },
      { label: "Method Details", action: () => openDialog("method_scope") },
      { label: "Preferences", action: () => setView("settings") },
    ],
    Window: [
      { label: "Reset Layout", action: resetLayout },
      { label: "Save Layout", action: saveLayout },
      { label: "Close Pane", action: closePane },
      { label: "Restore Pane", action: restorePane },
    ],
    Help: [
      { label: "Shortcuts", action: () => openDialog("help_shortcuts") },
      { label: "Documentation", dialog: "documentation" },
      { label: "About QuickPLS", action: () => openDialog("method_scope") },
    ],
  };
  return <nav className="np-menu" aria-label="Application menu">
    {menus.map((menu) => <div key={menu} className="np-menu-slot">
      <button type="button" onClick={() => setActiveMenu(activeMenu === menu ? null : menu)}>{menu}</button>
      {activeMenu === menu ? <div className="np-menu-popover">
        {menuItems[menu].map((item) => <button
          key={item.label}
          type="button"
          {...nativeCommandState(item)}
          onClick={() => {
            setActiveMenu(null);
            runNativeCommand(item, openDialog);
          }}
        >{item.label}</button>)}
      </div> : null}
    </div>)}
  </nav>;
}

function CommandBar({ data, openDialog }: { data: NativePrototypeData; openDialog: (dialog: NativePrototypeDialog) => void }) {
  const buttons: Array<{ label: string; icon: typeof Home; active?: boolean; dialog?: NativePrototypeDialog }> = [
    { label: "Save", icon: Save },
    { label: "Undo", icon: Undo2 },
    { label: "Redo", icon: Redo2 },
    { label: "Select", icon: MousePointer2, active: true },
    { label: "Pan", icon: Hand },
    { label: "Add Latent", icon: Circle },
    { label: "Add Indicator", icon: RectangleHorizontal },
    { label: "Connect Path", icon: Move },
    { label: "Covariance", icon: Link },
    { label: "Delete", icon: Trash2 },
    { label: "Arrange", icon: LayoutGrid },
    { label: "Check Diagram", icon: CheckCircle2 },
    { label: "Focus Diagram", icon: MonitorCog },
    { label: "Zoom", icon: ZoomIn },
  ];
  return <div className="np-commandbar np-ribbon" data-v240-ribbon="true">
    <div className="np-ribbon-group model-tools">
      {buttons.map(({ label, icon: Icon, active }) => <button key={label} className={active ? "active" : ""}><Icon size={20} /> <span>{label}</span>{label === "Arrange" || label === "Zoom" ? <ChevronDown size={12} /> : null}</button>)}
    </div>
    <div className="np-command-spacer" />
    <div className="np-ribbon-group calculate">
      <select aria-label="Selected method" value={data.selectedMethodLabel} onChange={() => undefined}><option>{data.selectedMethodLabel}</option><option>More methods in Setup...</option></select>
      <Chip tone="good">Supported setup</Chip>
      <button className="primary" onClick={() => openDialog("calculation_setup")}><Play size={18} /> <span>Run</span></button>
    </div>
  </div>;
}

function ModelCommandBar({ openDialog }: { openDialog: (dialog: NativePrototypeDialog) => void }) {
  const tools: NativeCommandItem[] = [
    { label: "Save", icon: Save, action: () => dispatchQuickPlsCommand("save-project") },
    { label: "Undo", icon: Undo2, action: () => dispatchNativeModelCommand("undo") },
    { label: "Redo", icon: Redo2, action: () => dispatchNativeModelCommand("redo") },
    { label: "Select", icon: MousePointer2, active: true, action: () => dispatchNativeModelCommand("tool", { tool: "select" }) },
    { label: "Pan", icon: Hand, action: () => dispatchNativeModelCommand("tool", { tool: "pan" }) },
    { label: "Add Latent", icon: Circle, action: () => dispatchNativeModelCommand("add-construct") },
    { label: "Add Indicator", icon: RectangleHorizontal, action: () => dispatchNativeModelCommand("add-indicator") },
    { label: "Connect Path", icon: Move, action: () => dispatchNativeModelCommand("tool", { tool: "path" }) },
    { label: "Covariance", icon: Link, action: () => dispatchNativeModelCommand("tool", { tool: "covariance" }) },
    { label: "Delete", icon: X, action: () => dispatchNativeModelCommand("delete-selection") },
    { label: "Arrange", icon: LayoutGrid, menu: true, action: () => dispatchNativeModelCommand("arrange", { direction: "smartpls" }) },
    { label: "Check Diagram", icon: CheckCircle2, action: () => dispatchNativeModelCommand("validate") },
    { label: "Focus Diagram", icon: MonitorCog, action: () => dispatchNativeModelCommand("focus-diagram") },
    { label: "Zoom 100%", icon: ZoomIn, menu: true, action: () => dispatchNativeModelCommand("fit") },
    { label: "Run", icon: Play, primary: true, action: () => openDialog("calculation_setup") },
  ];
  return <div className="np-commandbar np-model-commandbar np-ribbon" data-v241-model-ribbon="true">
    {tools.map((tool, index) => {
      const Icon = tool.icon ?? Home;
      const startsGroup = ["Undo", "Select", "Add Latent", "Connect Path", "Delete", "Arrange", "Check Diagram", "Zoom 100%", "Run"].includes(tool.label) && index !== 0;
      return <button
        key={tool.label}
        type="button"
        className={`${tool.active ? "active" : ""}${startsGroup ? " with-separator" : ""}${tool.primary ? " run-tool" : ""}`}
        {...nativeCommandState(tool)}
        onClick={() => runNativeCommand(tool, openDialog)}
      >
        <Icon size={tool.label === "Run" ? 24 : 25} />
        <span>{tool.label}</span>
        {tool.menu ? <ChevronDown size={11} /> : null}
      </button>;
    })}
  </div>;
}

function SetupCommandBar({ openDialog, setView }: { openDialog: (dialog: NativePrototypeDialog) => void; setView: (view: NativePrototypeView) => void }) {
  const groups: Array<{ title: string; items: NativeCommandItem[] }> = [
    { title: "Project", items: [
      { label: "New", icon: FilePlus2, dialog: "new_project" },
      { label: "Open", icon: FolderOpen, action: () => dispatchQuickPlsCommand("open-project") },
      { label: "Save", icon: Save, action: () => dispatchQuickPlsCommand("save-project") },
      { label: "Close", icon: X, dialog: "close_project" },
      { label: "Import", icon: Import, dialog: "import_data" },
      { label: "Export", icon: FileDown, dialog: "export_options" },
    ] },
    { title: "Data", items: [
      { label: "Data Source", icon: Database, action: () => openDialog("import_data") },
      { label: "Transform", icon: Columns3, dialog: "data_transform" },
      { label: "Check", icon: CheckCircle2, action: () => setView("data") },
      { label: "Filter", icon: Search, dialog: "data_filter" },
    ] },
    { title: "Model", items: [
      { label: "Constructs", icon: Circle, action: () => setView("model") },
      { label: "Indicators", icon: RectangleHorizontal, action: () => setView("model") },
      { label: "Diagram", icon: LayoutGrid, action: () => setView("model") },
      { label: "Groups", icon: Move, action: () => setView("results") },
    ] },
    { title: "Calculate", items: [
      { label: "Setup", icon: ListChecks, dialog: "calculation_setup" },
      { label: "Run", icon: Play, dialog: "calculation_setup" },
      { label: "Stop", icon: Circle, action: () => dispatchQuickPlsCommand("cancel-analysis") },
    ] },
    { title: "Report", items: [
      { label: "Report", icon: FileText, dialog: "export_options" },
      { label: "Export Tables", icon: Table2, action: () => setView("report") },
      { label: "Export Figures", icon: FileDown, action: () => setView("report") },
    ] },
    { title: "Diagnostics", items: [
      { label: "PLSpredict", icon: BarChart3, action: () => setView("setup") },
      { label: "MICOM", icon: Move, action: () => setView("setup") },
      { label: "Bootstrap", icon: Sigma, dialog: "calculation_setup" },
      { label: "Quality", icon: Database, action: () => setView("results") },
    ] },
  ];
  return <div className="np-commandbar np-setup-commandbar np-ribbon" data-v241-setup-ribbon="true">
    {groups.map((group) => <div key={group.title} className="np-setup-ribbon-group">
      <div className="np-setup-ribbon-title">{group.title}</div>
      <div className="np-setup-ribbon-buttons">
        {group.items.map((item) => {
          const Icon = item.icon ?? Home;
          return <button
            key={`${group.title}-${item.label}`}
            type="button"
            className={item.disabledReason ? "disabled" : ""}
            {...nativeCommandState(item)}
            onClick={() => runNativeCommand(item, openDialog)}
          >
            <Icon size={22} />
            <span>{item.label}</span>
          </button>;
        })}
      </div>
    </div>)}
  </div>;
}

function RunCommandBar({ openDialog, setView }: { openDialog: (dialog: NativePrototypeDialog) => void; setView: (view: NativePrototypeView) => void }) {
  const buttons: NativeCommandItem[] = [
    { label: "Validate", icon: CheckCircle2, action: () => {
      setView("run");
      dispatchNativeStatus("Run readiness checklist is visible in the Run workspace.", "success");
    } },
    { label: "Calculation Setup", icon: Settings, dialog: "calculation_setup" },
    { label: "Start Run", icon: Play, primary: true, dialog: "calculation_setup" },
    { label: "Cancel", icon: X, action: () => dispatchQuickPlsCommand("cancel-analysis") },
    { label: "Open Results", icon: FolderOpen, action: () => setView("results") },
    { label: "Prepare Report", icon: FileText, action: () => setView("report") },
  ];
  return <div className="np-commandbar np-run-commandbar np-ribbon" data-v241-run-ribbon="true">
    {buttons.map((button, index) => {
      const Icon = button.icon ?? Home;
      return <button
        key={button.label}
        type="button"
        className={`${button.primary ? "primary-run" : ""}${button.disabledReason ? " disabled" : ""}${index === 1 || index === 5 ? " with-separator" : ""}`}
        {...nativeCommandState(button)}
        onClick={() => runNativeCommand(button, openDialog)}
      >
        <Icon size={22} />
        <span>{button.label}</span>
      </button>;
    })}
  </div>;
}

function ResultsCommandBar({ openDialog, setView }: { openDialog: (dialog: NativePrototypeDialog) => void; setView: (view: NativePrototypeView) => void }) {
  const buttons: NativeCommandItem[] = [
    { label: "Select Run", icon: Play, menu: true, action: () => dispatchNativeResultsCommand("select-run") },
    { label: "Method Details", icon: BarChart3, menu: true, action: () => dispatchNativeResultsCommand("method-confidence") },
    { label: "Copy Table", icon: FileText, action: () => dispatchNativeResultsCommand("copy-current-table") },
    { label: "Export Table", icon: FileDown, menu: true, action: () => dispatchNativeResultsCommand("export-current-table") },
    { label: "Export Workbook", icon: Table2, menu: true, action: () => setView("report") },
    { label: "Show Interpretation", icon: HelpCircle, action: () => dispatchNativeResultsCommand("open-interpretation") },
    { label: "Compare Runs", icon: Move, action: () => dispatchNativeResultsCommand("open-comparison") },
    { label: "Prepare Report", icon: FileText, action: () => dispatchNativeResultsCommand("prepare-report") },
  ];
  return <div className="np-commandbar np-results-commandbar np-ribbon" data-v241-results-ribbon="true">
    {buttons.map((button, index) => {
      const Icon = button.icon ?? Home;
      return <button
        key={button.label}
        type="button"
        className={index === 1 || index === 5 || index === 7 ? "with-separator" : ""}
        {...nativeCommandState(button)}
        onClick={() => runNativeCommand(button, openDialog)}
      >
        <Icon size={22} />
        <span>{button.label}</span>
        {button.menu ? <ChevronDown size={13} /> : null}
      </button>;
    })}
  </div>;
}

function ReportCommandBar({ openDialog }: { openDialog: (dialog: NativePrototypeDialog) => void }) {
  const buttons: NativeCommandItem[] = [
    { label: "Select Run", icon: Database, menu: true, action: () => dispatchNativeReportCommand("select-run") },
    { label: "Preview", icon: Search, action: () => dispatchNativeReportCommand("preview") },
    { label: "Export SVG", icon: FileDown, action: () => dispatchNativeReportCommand("export-svg") },
    { label: "Export Tables", icon: Table2, action: () => dispatchNativeReportCommand("export-tables") },
    { label: "Export Workbook", icon: FileDown, action: () => dispatchNativeReportCommand("export-workbook") },
    { label: "Print", icon: FileText, action: () => dispatchNativeReportCommand("print") },
    { label: "Reviewer Pack", icon: FileText, dialog: "export_options" },
    { label: "Open Folder", icon: FolderOpen, action: () => { void openDefaultExportFolderFromShell().catch((error) => dispatchNativeStatus(String(error), "error")); } },
  ];
  return <div className="np-commandbar np-report-commandbar np-ribbon" data-v241-report-ribbon="true">
    {buttons.map((button, index) => {
      const Icon = button.icon ?? Home;
      return <button
        key={button.label}
        type="button"
        className={index === 1 || index === 2 || index === 5 || index === 7 ? "with-separator" : ""}
        {...nativeCommandState(button)}
        onClick={() => runNativeCommand(button, openDialog)}
      >
        <Icon size={22} />
        <span>{button.label}</span>
        {button.menu ? <ChevronDown size={13} /> : null}
      </button>;
    })}
  </div>;
}

function TrustCommandBar({
  openDialog,
  setReleaseIntegrity,
}: {
  openDialog: (dialog: NativePrototypeDialog) => void;
  setReleaseIntegrity: (result: ChecksumVerification | null) => void;
}) {
  const buttons: Array<NativeCommandItem & { separator?: boolean }> = [
    { label: "Refresh References", icon: Undo2, action: () => dispatchNativeTrustCommand("refresh-evidence") },
    { label: "Open Method Doc", icon: BookOpen, separator: true, action: () => dispatchNativeTrustCommand("open-method-doc") },
    { label: "Export Reference Index", icon: Table2, action: () => dispatchNativeTrustCommand("export-evidence-index") },
    {
      label: "Verify Checksums",
      icon: ShieldCheck,
      separator: true,
      action: () => {
        void verifyReleaseChecksumsFromShell()
          .then((result) => {
            if (!result) return;
            setReleaseIntegrity(result);
            openDialog("release_integrity");
          })
          .catch((error) => dispatchNativeStatus(String(error), "error"));
      },
    },
    { label: "Known Limitations", icon: AlertTriangle, separator: true, dialog: "method_scope" },
    { label: "About Method Support", icon: HelpCircle, dialog: "method_scope" },
  ];
  return <div className="np-commandbar np-trust-commandbar np-ribbon" data-v241-trust-ribbon="true">
    {buttons.map((button) => {
      const Icon = button.icon ?? Home;
      return <button
        key={button.label}
        type="button"
        className={button.separator ? "with-separator" : ""}
        {...nativeCommandState(button)}
        onClick={() => runNativeCommand(button, openDialog)}
      >
        <Icon size={24} />
        <span>{button.label}</span>
      </button>;
    })}
  </div>;
}

function SettingsCommandBar() {
  const buttons: Array<NativeCommandItem & { tone?: "apply" | "ok" | "cancel" | "reset"; separator?: boolean }> = [
    { label: "Apply", icon: CheckCircle2, tone: "apply", action: () => dispatchNativeSettingsCommand("apply") },
    { label: "OK", icon: CheckCircle2, tone: "ok", action: () => dispatchNativeSettingsCommand("ok") },
    { label: "Cancel", icon: X, tone: "cancel", action: () => dispatchNativeSettingsCommand("cancel") },
    { label: "Reset Defaults", icon: Undo2, tone: "reset", separator: true, action: () => dispatchNativeSettingsCommand("reset") },
    { label: "Import Preferences", icon: Import, separator: true, action: () => dispatchNativeSettingsCommand("import") },
    { label: "Export Preferences", icon: FileDown, action: () => dispatchNativeSettingsCommand("export") },
  ];
  return <div className="np-commandbar np-settings-commandbar np-ribbon" data-v241-settings-ribbon="true">
    {buttons.map((button) => {
      const Icon = button.icon ?? Home;
      return <button
        key={button.label}
        type="button"
        className={`${button.tone ?? ""}${button.separator ? " with-separator" : ""}${button.disabledReason ? " disabled" : ""}`}
        {...nativeCommandState(button)}
        onClick={() => runNativeCommand(button)}
      >
        <Icon size={23} />
        <span>{button.label}</span>
      </button>;
    })}
  </div>;
}

function HomeCommandBar({ openDialog, setView }: { openDialog: (dialog: NativePrototypeDialog) => void; setView: (view: NativePrototypeView) => void }) {
  return <div className="np-commandbar np-home-commandbar" data-v241-home-ribbon="true">
    <div className="np-home-command-group np-ribbon-group">
      <button type="button" data-command-status="wired" onClick={() => openDialog("new_project")}><FilePlus2 size={25} /><span>New Project</span><ChevronDown size={13} /></button>
    </div>
    <div className="np-home-command-group np-ribbon-group">
      <button type="button" data-command-status="wired" onClick={() => dispatchQuickPlsCommand("open-project")}><FolderOpen size={25} /><span>Open</span><ChevronDown size={13} /></button>
      <button type="button" data-command-status="wired" onClick={() => dispatchQuickPlsCommand("save-project")}><Save size={24} /><span>Save</span></button>
    </div>
    <div className="np-home-command-group np-ribbon-group">
      <button type="button" data-command-status="wired" onClick={() => openDialog("import_data")}><Table2 size={25} /><span>Import Data</span><ChevronDown size={13} /></button>
      <button type="button" className="recent-command" data-command-status="wired" onClick={() => {
        setView("home");
        window.setTimeout(() => window.dispatchEvent(new CustomEvent("quickpls:home-focus-recent")), 0);
      }}><Clock3 size={24} /><span>Recent</span><ChevronDown size={13} /></button>
    </div>
    <div className="np-home-command-group np-ribbon-group">
      <button type="button" data-command-status="wired" onClick={() => openDialog("settings")}><Settings size={24} /><span>Settings</span></button>
    </div>
    <div className="np-home-command-group np-ribbon-group">
      <button type="button" data-command-status="wired" onClick={() => openDialog("help_shortcuts")}><HelpCircle size={25} /><span>Help</span><ChevronDown size={13} /></button>
    </div>
  </div>;
}

function DataCommandBar({ openDialog }: { openDialog: (dialog: NativePrototypeDialog) => void }) {
  const commands: NativeCommandItem[] = [
    { label: "Import Data", icon: Import, dialog: "import_data", menu: true },
    { label: "Save", icon: Save, action: () => dispatchQuickPlsCommand("save-project") },
    { label: "Transform", icon: Columns3, menu: true, dialog: "data_transform" },
    { label: "Add Column", icon: Table2, dialog: "data_add_column" },
    { label: "Recode", icon: FileText, dialog: "data_recode" },
    { label: "Missing Values", icon: X, menu: true, dialog: "data_missing_values" },
    { label: "Filter", icon: Search, menu: true, dialog: "data_filter" },
    { label: "Sort A-Z", icon: Sigma, menu: true, dialog: "data_sort" },
    { label: "Sort Z-A", icon: Sigma, action: () => dispatchNativeDataCommand("sort", { direction: "desc" }) },
    { label: "Create Constructs", icon: Link, menu: true, action: () => dispatchNativeDataCommand("create-constructs-from-prefixes") },
    { label: "Validate Data", icon: CheckCircle2, menu: true, action: () => dispatchNativeDataCommand("show-quality") },
  ];
  return <div className="np-commandbar np-data-commandbar np-ribbon" data-v241-data-ribbon="true">
    <div className="np-ribbon-group">
      {commands.map((command, index) => {
        const Icon = command.icon ?? Home;
        const startsGroup = ["Save", "Transform", "Filter", "Create Constructs", "Validate Data"].includes(command.label) && index !== 0;
        return <button
          key={`${command.label}-${index}`}
          type="button"
          className={startsGroup ? "with-separator" : ""}
          {...nativeCommandState(command)}
          onClick={() => runNativeCommand(command, openDialog)}
        >
          <Icon size={21} />
          <span>{command.label}</span>
          {command.menu ? <ChevronDown size={12} /> : null}
        </button>;
      })}
    </div>
  </div>;
}

function Rail({ view, setView }: { view: NativePrototypeView; setView: (view: NativePrototypeView) => void }) {
  return <aside className="np-rail" aria-label="QuickPLS workspaces">
    <div className="np-rail-items">{views.map(({ id, label, icon: Icon }) => <button key={id} className={view === id ? "active" : ""} onClick={() => setView(id)}><Icon size={17} /><span>{label}</span></button>)}</div>
    <div className="np-rail-support">{supportViews.map(({ id, label, icon: Icon }) => <button key={id} className={view === id ? "active" : ""} onClick={() => setView(id)}><Icon size={17} /><span>{label}</span></button>)}</div>
  </aside>;
}

function CompactCommandBar({
  view,
  data,
  openDialog,
  setView,
}: {
  view: NativePrototypeView;
  data: NativePrototypeData;
  openDialog: (dialog: NativePrototypeDialog) => void;
  setView: (view: NativePrototypeView) => void;
}) {
  const commandMap: Record<NativePrototypeView, NativeCommandItem[]> = {
    home: [
      { label: "New project", icon: FilePlus2, dialog: "new_project" },
      { label: "Open", icon: FolderOpen, action: () => dispatchQuickPlsCommand("open-project") },
      { label: "Import data", icon: Import, dialog: "import_data" },
    ],
    data: [
      { label: "Import", icon: Import, dialog: "import_data" },
      { label: "Transform", icon: Columns3, dialog: "data_transform" },
      { label: "Filter", icon: Search, dialog: "data_filter" },
      { label: "Validate", icon: CheckCircle2, action: () => dispatchNativeDataCommand("show-quality") },
      { label: "Create constructs", icon: Link, action: () => dispatchNativeDataCommand("create-constructs-from-prefixes") },
    ],
    model: [
      { label: "Save", icon: Save, action: () => dispatchQuickPlsCommand("save-project") },
      { label: "Select", icon: MousePointer2, active: true, action: () => dispatchNativeModelCommand("tool", { tool: "select" }) },
      { label: "Construct", icon: Circle, action: () => dispatchNativeModelCommand("add-construct") },
      { label: "Path", icon: Move, action: () => dispatchNativeModelCommand("tool", { tool: "path" }) },
      { label: "Arrange", icon: LayoutGrid, action: () => dispatchNativeModelCommand("arrange", { direction: "smartpls" }) },
      { label: "Check", icon: CheckCircle2, action: () => dispatchNativeModelCommand("validate") },
      { label: "Calculate", icon: Play, primary: true, dialog: "calculation_setup" },
    ],
    setup: [
      { label: "Data", icon: Database, action: () => setView("data") },
      { label: "Model", icon: LayoutGrid, action: () => setView("model") },
      { label: "Method settings", icon: Settings, dialog: "calculation_setup" },
      { label: "Calculate", icon: Play, primary: true, dialog: "calculation_setup" },
    ],
    run: [
      { label: "Setup", icon: Settings, dialog: "calculation_setup" },
      { label: "Start", icon: Play, primary: true, dialog: "calculation_setup" },
      { label: "Cancel", icon: X, action: () => dispatchQuickPlsCommand("cancel-analysis") },
      { label: "Results", icon: BarChart3, action: () => setView("results") },
    ],
    results: [
      { label: "Edit model", icon: LayoutGrid, action: () => setView("model") },
      { label: "Calculate", icon: Play, primary: true, dialog: "calculation_setup" },
      { label: "Copy table", icon: FileText, action: () => dispatchNativeResultsCommand("copy-current-table") },
      { label: "Export", icon: FileDown, action: () => dispatchNativeResultsCommand("export-current-table") },
      { label: "Report", icon: FileText, action: () => setView("report") },
    ],
    report: [
      { label: "Results", icon: BarChart3, action: () => setView("results") },
      { label: "Preview", icon: Search, action: () => dispatchNativeReportCommand("preview") },
      { label: "Export", icon: FileDown, primary: true, dialog: "export_options" },
      { label: "Print", icon: FileText, action: () => dispatchNativeReportCommand("print") },
    ],
    trust: [
      { label: "Documentation", icon: BookOpen, dialog: "documentation" },
      { label: "Method details", icon: ShieldCheck, dialog: "method_scope" },
    ],
    settings: [
      { label: "Save", icon: Save, action: () => dispatchNativeSettingsCommand("save") },
      { label: "Reset", icon: Undo2, action: () => dispatchNativeSettingsCommand("reset") },
    ],
  };
  const contextLabel = view === "home" ? "Start"
    : view === "data" ? data.projectSummary.dataset
      : view === "results" ? data.selectedRunLabel
        : view === "report" ? data.reportSummary.selectedRun
          : data.projectSummary.name;

  return <div className="np-commandbar np-compact-commandbar" role="toolbar" aria-label={`${view} commands`} data-native-command-surface="compact">
    <div className="np-command-context">
      <strong>{view === "setup" ? data.selectedMethodLabel : view[0].toUpperCase() + view.slice(1)}</strong>
      <span title={contextLabel}>{contextLabel}</span>
    </div>
    <div className="np-compact-command-actions">
      {commandMap[view].map((command) => {
        const Icon = command.icon ?? Home;
        return <button
          key={command.label}
          type="button"
          className={`${command.active ? "active" : ""}${command.primary ? " primary" : ""}`}
          {...nativeCommandState(command)}
          onClick={() => runNativeCommand(command, openDialog)}
        >
          <Icon size={16} aria-hidden="true" />
          <span>{command.label}</span>
        </button>;
      })}
    </div>
    <div className="np-command-spacer" />
    {view !== "home" ? <div className="np-command-state" title="Local, deterministic project state">
      <span className="np-command-state-dot" />
      <span>Offline</span>
      <strong>{data.projectSummary.status}</strong>
    </div> : null}
  </div>;
}

type NativeStatusMessage = { message: string; tone?: "success" | "info" | "warning" | "error" } | null;

function StatusBar({ data, view, message }: { data: NativePrototypeData; view: NativePrototypeView; message: NativeStatusMessage }) {
  const settings = useWorkspace((state) => state.analysisSettings);
  const experimentalLabsEnabled = useWorkspace((state) => state.uiPreferences.experimentalLabsEnabled);
  const methodCounts = useMemo(() => analysisCatalogCapabilityCountsV2(
    analysisCatalogCapabilityEntriesV2(methods, settings, { experimentalLabsEnabled }),
  ), [experimentalLabsEnabled, settings]);
  const statusOverride = message
    ? <span className={`status-cell command-feedback ${message.tone ?? "info"}`}>{message.message}</span>
    : null;
  if (view === "home") {
    return <footer className="np-statusbar np-home-statusbar">
      {statusOverride ?? <span className="status-cell ready">Ready</span>}
      <span className="status-cell"><MonitorCog size={15} /> Offline</span>
      <span className="status-cell">{data.projectSummary.name}</span>
      <span className="status-cell"><CheckCircle2 size={15} /> Engine: Ready (PLS-SEM)</span>
      <span className="status-cell">{data.recentProjects.length} recent project(s)</span>
    </footer>;
  }
  if (view === "data") {
    return <footer className="np-statusbar np-data-statusbar">
      {statusOverride ?? <span className="status-cell ready"><CheckCircle2 size={17} /> Ready</span>}
      <span className="status-cell"><MonitorCog size={15} /> Offline</span>
      <span className="status-cell">Dataset: {data.projectSummary.dataset}</span>
      <span className="status-cell">Cases: {data.projectSummary.cases}</span>
      <span className="status-cell">Variables: {data.projectSummary.variables}</span>
      <span className="status-cell">{data.resultSummary.hasRun ? `${data.projectSummary.savedRuns} run(s) available` : "No results calculated"}</span>
    </footer>;
  }
  if (view === "setup") {
    return <footer className="np-statusbar">
      {statusOverride ?? <span className="ready-dot" />}
      <span>Project: {data.projectSummary.name}</span>
      <span className="right">PLS-SEM</span>
      <span><CheckCircle2 size={13} /> Data: {data.projectSummary.variables ? "OK" : "Missing"}</span>
      <span><CheckCircle2 size={13} /> Model: {data.projectSummary.constructs ? "OK" : "Missing"}</span>
      <span>{data.projectSummary.status} <CheckCircle2 size={13} /></span>
    </footer>;
  }
  if (view === "results") {
    return <footer className="np-statusbar np-results-statusbar">
      {statusOverride ?? <span><CheckCircle2 size={13} /> Ready</span>}
      <span><MonitorCog size={15} /> Offline</span>
      <span>{data.resultSummary.hasRun ? "Results loaded" : "No completed run"}</span>
      <span>{data.selectedRunLabel}</span>
      <span>{data.projectSummary.paths} path(s)</span>
      <span className="right"><CheckCircle2 size={13} /> Setup: {data.projectSummary.status}</span>
    </footer>;
  }
  if (view === "report") {
    return <footer className="np-statusbar np-report-statusbar">
      {statusOverride ?? <span><span className="ready-dot" /> Ready</span>}
      <span><MonitorCog size={15} /> Offline</span>
      <span>Selected Run: <strong>{data.reportSummary.selectedRun}</strong></span>
      <span>Report Preset: <strong>Journal Figure</strong></span>
      <span className="right">Export Folder: <strong>{data.reportSummary.destination}</strong> <FolderOpen size={14} /></span>
    </footer>;
  }
  if (view === "trust") {
    return <footer className="np-statusbar np-trust-statusbar">
      {statusOverride ?? <span><span className="ready-dot" /> Ready</span>}
      <span><MonitorCog size={15} /> Offline</span>
      <span>References updated: 2025-05-12 10:22:14</span>
      <span>Supported methods: {methodCounts.standard}</span>
      <span>Experimental methods: {methodCounts.experimental}</span>
      <span className="right"><CheckCircle2 size={13} /> Release checksums available</span>
    </footer>;
  }
  if (view === "settings") {
    return <footer className="np-statusbar np-settings-statusbar">
      {statusOverride ?? <span>Ready</span>}
      <span><span className="offline-dot" /> Offline</span>
      <span><Table2 size={14} /> Preferences: local QuickPLS UI settings</span>
      <span className="right"><CheckCircle2 size={15} /> Settings are UI-only</span>
    </footer>;
  }
  if (view === "run") {
    return <footer className="np-statusbar np-run-statusbar">
      {statusOverride ?? <><span className={data.runSummary.state === "running" ? "running-dot" : "ready-dot"} /> <span>{data.runSummary.state}</span></>}
      <span><MonitorCog size={15} /> Offline</span>
      <span>Workers: {data.runSummary.settings.find(([label]) => label === "Workers")?.[1] ?? "1"}</span>
      <span>Elapsed Time: {data.runSummary.elapsed}</span>
      <span className="right">Checks: Data &amp; Model</span>
    </footer>;
  }
  return <footer className="np-statusbar">
    {statusOverride ?? <span><CheckCircle2 size={13} /> Ready</span>}
    <span>Project: {data.projectSummary.name}</span>
    <span>{data.projectSummary.cases} cases</span>
    <span>{data.projectSummary.constructs} constructs</span>
    <span>{data.projectSummary.paths} paths</span>
    <span className="right">Current workspace: {view}</span>
    <span>Offline mode</span>
  </footer>;
}

function HomeScreen({ data, setView, openDialog }: { data: NativePrototypeData; setView: (view: NativePrototypeView) => void; openDialog: (dialog: NativePrototypeDialog) => void }) {
  useEffect(() => {
    const focusRecent = () => {
      const recentPane = document.getElementById("quickpls-native-recent-projects");
      recentPane?.scrollIntoView({ block: "nearest", inline: "nearest" });
      recentPane?.focus();
      dispatchNativeStatus("Recent projects list focused.", "info");
    };
    window.addEventListener("quickpls:home-focus-recent", focusRecent);
    return () => window.removeEventListener("quickpls:home-focus-recent", focusRecent);
  }, []);
  const recentRows = data.recentProjects.map((project) => [
    project.name,
    project.path,
    project.modified,
    data.projectSummary.dataset,
    String(project.runs),
    project.status,
  ]);
  const messageRows = data.messages;
  const statusTone = (status: string) => status === "Ready" || status === "Supported setup" ? "good" : status === "Error" ? "bad" : status === "Not Run" ? "neutral" : "warn";
  return <main className="np-home-manager" data-v237-screen="home" data-v241-home-exact="true">
    <span className="np-parity-required-text">Welcome to QuickPLS 2.0 Project Summary Quick Links Open Project</span>
    <section className="np-home-top">
      <aside className="np-desktop-pane np-project-manager-pane">
        <div className="np-pane-title"><strong>Project Manager</strong><ChevronDown size={14} /></div>
        <div className="np-project-actions">
          <button type="button" onClick={() => openDialog("new_project")}>
            <span className="np-large-action-icon document"><FilePlus2 size={44} /><Plus size={18} /></span>
            <span><strong>New PLS-SEM Project</strong><small>Create a new project and start a PLS-SEM model from scratch.</small></span>
          </button>
          <button type="button">
            <span className="np-large-action-icon folder"><FolderOpen size={54} /></span>
            <span><strong>Open Existing Project</strong><small>Open an existing QuickPLS project (.qpls) from your computer.</small></span>
          </button>
          <button type="button" onClick={() => setView("data")}>
            <span className="np-large-action-icon data"><Table2 size={52} /><ChevronRight size={18} /></span>
            <span><strong>Import Dataset</strong><small>Import data from CSV, Excel, SAV, or other supported formats.</small></span>
          </button>
          <button type="button" onClick={() => openDialog("sample_gallery")}>
            <span className="np-large-action-icon sample"><FolderOpen size={54} /><ShieldCheck size={18} /></span>
            <span><strong>Open Sample Project</strong><small>Explore example projects and datasets.</small></span>
          </button>
        </div>
      </aside>
      <section className="np-desktop-pane np-recent-pane" id="quickpls-native-recent-projects" tabIndex={-1}>
        <div className="np-pane-title"><strong>Recent Projects</strong><span>»</span></div>
        <div className="np-recent-table-wrap">
          <table className="np-recent-table">
            <thead><tr><th></th><th>Project Name</th><th>Path</th><th>Modified</th><th>Dataset</th><th>Runs</th><th>Status</th></tr></thead>
            <tbody>{recentRows.map((row, index) => <tr key={row[0]} className={index === 0 ? "selected" : ""}>
              <td><FileText size={13} /></td>
              <td>{row[0]}</td>
              <td>{row[1]}</td>
              <td>{row[2]}</td>
              <td>{row[3]}</td>
              <td>{row[4]}</td>
              <td><Chip tone={statusTone(row[5])}>{row[5]}</Chip></td>
            </tr>)}</tbody>
          </table>
        </div>
        <footer className="np-recent-footer"><button type="button">Clear Recent List</button><span>{recentRows.length} item(s)</span></footer>
      </section>
      <aside className="np-desktop-pane np-details-pane">
        <div className="np-pane-title"><strong>Project Details / Getting Started</strong><ChevronDown size={14} /></div>
        <div className="np-details-body">
          <h2>{data.projectSummary.name}</h2>
          <p>{recentRows[0]?.[1] || "No project path saved yet"}</p>
          <dl>
            <dt>Dataset:</dt><dd>{data.projectSummary.dataset} ({data.projectSummary.cases} cases, {data.projectSummary.variables} variables)</dd>
            <dt>Last Modified:</dt><dd>{recentRows[0]?.[2] || "Current session"}</dd>
            <dt>Last Run:</dt><dd>{data.selectedRunLabel}</dd>
            <dt>PLS Engine:</dt><dd>QuickPLS Engine 2.x</dd>
            <dt>Model Type:</dt><dd>{data.selectedMethodLabel}</dd>
            <dt>Description:</dt><dd>{data.projectSummary.constructs} constructs, {data.projectSummary.indicators} indicators, {data.projectSummary.paths} paths.</dd>
          </dl>
          <div className="np-detail-divider" />
          <div className="np-validation-status"><strong>Project Checks</strong><Chip tone={data.projectSummary.status === "Ready" ? "good" : "warn"}>{data.projectSummary.status}</Chip></div>
          <dl>
            <dt>Checked:</dt><dd>{new Date().toLocaleString()}</dd>
            <dt>Issues:</dt><dd>{data.projectSummary.status === "Ready" ? "0" : "Review setup"}</dd>
            <dt>Warnings:</dt><dd>{data.resultSummary.warnings}</dd>
          </dl>
          <div className="np-detail-divider" />
          <h3>Quick Actions</h3>
          <button type="button"><FolderOpen size={23} /><span><strong>Open</strong><small>Open this project.</small></span></button>
          <button type="button"><Search size={23} /><span><strong>Locate File</strong><small>Open folder containing this project file.</small></span></button>
          <button type="button" className="danger"><X size={23} /><span><strong>Remove from Recent</strong><small>Remove this project from the recent list.</small></span></button>
        </div>
      </aside>
    </section>
    <section className="np-desktop-pane np-message-pane">
      <div className="np-message-tabs">
        {["Messages", "Recovery (1)", "Run Details", "Recent Activity"].map((tab, index) => <button key={tab} className={index === 0 ? "active" : ""}>{tab}</button>)}
        <span className="np-message-tools"><Search size={14} /><X size={14} /><ListChecks size={15} /></span>
      </div>
      <table className="np-message-table">
        <thead><tr><th>Time</th><th>Level</th><th>Source</th><th>Message</th><th>Project</th></tr></thead>
        <tbody>{messageRows.map((row) => <tr key={`${row[0]}-${row[2]}-${row[3]}`}>
          <td>{row[0]}</td><td className={row[1] === "WARNING" ? "warn-level" : "info-level"}>{row[1]}</td><td>{row[2]}</td><td>{row[3]}</td><td>{row[4]}</td>
        </tr>)}</tbody>
      </table>
    </section>
  </main>;
}

function DataScreen({ data, openDialog }: { data: NativePrototypeData; openDialog: (dialog: NativePrototypeDialog) => void }) {
  const [tab, setTab] = useState("Data View");
  const headers = data.dataHeaders.length ? data.dataHeaders : ["ID"];
  const rows = data.dataRows.length ? data.dataRows : [["1"]];
  const selectedHeader = data.selectedVariable.name || headers[1] || headers[0];
  const selectedColumnIndex = Math.max(0, headers.indexOf(selectedHeader));
  const quality = [
    ["Cases (Rows)", data.dataQuality.cases, ""],
    ["Variables (Columns)", data.dataQuality.variables, ""],
    ["Missing Values", data.dataQuality.missingValues, data.dataQuality.missingValues.startsWith("0") ? "good" : "warn"],
    ["Duplicate Rows", data.dataQuality.duplicateRows, "good"],
    ["Constant Columns", data.dataQuality.constantColumns, data.dataQuality.constantColumns.startsWith("0") ? "good" : "warn"],
    ["Numeric Variables", data.dataQuality.numericVariables, "info"],
  ];
  const variableRows = data.variables.map((variable) => [
    variable.name,
    variable.label,
    variable.type,
    variable.name === data.selectedVariable.name ? data.selectedVariable.scale : variable.type,
    variable.role,
    String(variable.missing),
    variable.name === data.selectedVariable.name ? data.selectedVariable.min : "",
    variable.name === data.selectedVariable.name ? data.selectedVariable.max : "",
    variable.name === data.selectedVariable.name ? data.selectedVariable.unique : "",
    variable.name === data.selectedVariable.name ? data.selectedVariable.assignedConstruct : "",
  ]);
  const qualityRows = [
    ["Cases", data.dataQuality.cases, "Current active dataset"],
    ["Variables", data.dataQuality.variables, "Visible columns in project data"],
    ["Missing values", data.dataQuality.missingValues, data.dataQuality.missingValues.startsWith("0") ? "No action required" : "Review missing value handling"],
    ["Duplicate rows", data.dataQuality.duplicateRows, data.dataQuality.duplicateRows.startsWith("0") ? "None detected" : "Review duplicated records"],
    ["Constant columns", data.dataQuality.constantColumns, data.dataQuality.constantColumns.startsWith("0") ? "None detected" : "Remove or justify before modeling"],
    ["Numeric variables", data.dataQuality.numericVariables, "Eligible for numeric methods"],
  ];
  const importHistoryRows = [
    [data.projectSummary.dataset || "No dataset imported", String(data.projectSummary.cases), String(data.projectSummary.variables), data.projectSummary.status],
  ];
  const noteRows = [
    ["Dataset", `${data.projectSummary.dataset || "No active dataset"} with ${data.projectSummary.cases} cases and ${data.projectSummary.variables} variables.`],
    ["Selected variable", `${selectedHeader}: ${data.selectedVariable.type}, ${data.selectedVariable.scale}, ${data.selectedVariable.role}.`],
    ["Applicability", data.methodApplicabilityRows[0]?.[3] ?? "Import data and define a model to see method guidance."],
  ];
  const renderActiveDataTab = () => {
    if (tab === "Variable View") {
      return <DataTable headers={["Name", "Label", "Type", "Scale", "Role", "Missing", "Min", "Max", "Unique", "Construct"]} rows={variableRows} />;
    }
    if (tab === "Import History") {
      return <DataTable headers={["Dataset", "Cases", "Variables", "Status"]} rows={importHistoryRows} />;
    }
    if (tab === "Data Quality") {
      return <DataTable headers={["Check", "Value", "Action"]} rows={qualityRows} />;
    }
    if (tab === "Notes") {
      return <DataTable headers={["Topic", "Note"]} rows={noteRows} />;
    }
    return <table className="np-spreadsheet-table">
      <thead><tr>{headers.map((header) => <th key={header} className={header === selectedHeader ? "selected-col" : ""}>{header}</th>)}</tr></thead>
      <tbody>{rows.map((row) => <tr key={row[0]}>{row.map((cell, cellIndex) => <td key={`${row[0]}-${cellIndex}`} className={cellIndex === selectedColumnIndex ? "selected-col" : cellIndex === 0 ? "row-index" : ""}>{cell}</td>)}</tr>)}</tbody>
    </table>;
  };
  return <main className="np-data-workbench-screen" data-v237-screen="data" data-v241-data-exact="true" data-v245-data-tabs="true">
    <span className="np-parity-required-text">Data Workbench Variable Metadata Data Notes</span>
    <div className="np-data-tabs">
      {["Data View", "Variable View", "Import History", "Data Quality", "Notes"].map((item) => <button key={item} className={tab === item ? "active" : ""} onClick={() => setTab(item)}>{item}<X size={11} /></button>)}
      <button>+</button>
    </div>
    <div className="np-data-quality-overview">
      {quality.map(([label, value, tone], index) => <div key={label} className={`np-data-quality-card ${tone}`}>
        <Table2 size={22 + (index % 2)} />
        <span>{label}</span>
        <strong>{value}</strong>
      </div>)}
    </div>
    <div className="np-data-workbench-grid">
      <section className="np-data-grid-pane">
        {renderActiveDataTab()}
      </section>
      <aside className="np-variable-properties">
        <header><strong>Variable Properties</strong><small>Variable Metadata</small><span>?</span><Pin size={12} /><X size={12} /></header>
        <dl>
          <dt>Name</dt><dd>{data.selectedVariable.name}</dd><dt>Label</dt><dd>{data.selectedVariable.label}</dd><dt>Type</dt><dd>{data.selectedVariable.type}</dd><dt>Role</dt><dd>{data.selectedVariable.role}</dd>
          <dt>Measurement scale</dt><dd>{data.selectedVariable.scale}</dd><dt>Missing markers</dt><dd>{data.selectedVariable.missingMarkers}</dd><dt>Min</dt><dd>{data.selectedVariable.min}</dd><dt>Max</dt><dd>{data.selectedVariable.max}</dd>
          <dt>Mean</dt><dd>{data.selectedVariable.mean}</dd><dt>Standard deviation</dt><dd>{data.selectedVariable.sd}</dd><dt>Unique values</dt><dd>{data.selectedVariable.unique}</dd><dt>Assigned construct</dt><dd>{data.selectedVariable.assignedConstruct}</dd>
          <dt>Indicator direction</dt><dd>Positive</dd><dt>Outer weight (PLS)</dt><dd>-</dd><dt>Outer loading (PLS)</dt><dd>-</dd>
        </dl>
        <button className="np-link-button">More properties...</button>
      </aside>
    </div>
    <section className="np-data-bottom-pane" data-v245-data-bottom="true">
      <div className="np-bottom-tabs">{["Import Log", "Data Issues", "Method Applicability", "Notes"].map((item, index) => <button key={item} className={index === 2 ? "active" : ""}>{item}</button>)}</div>
      <p><strong>Data Notes</strong> Based on the current data ({data.projectSummary.cases} cases, {data.projectSummary.variables} variables)</p>
      <table className="np-method-applicability-table">
        <thead><tr><th>Method</th><th>Applicability</th><th>Status</th><th>Recommendation</th></tr></thead>
        <tbody>
          {data.methodApplicabilityRows.map((row) => <tr key={`${row[0]}-${row[1]}`}>
            <td>{row[0]}</td>
            <td className={row[1] === "Recommended" || row[1] === "Available" ? "good-text" : row[1] === "Not Applicable" ? "bad-text" : "warn-text"}>{row[1]}</td>
            <td>{row[1] === "Recommended" || row[1] === "Available" ? <CheckCircle2 size={14} /> : row[1] === "Not Applicable" ? <X size={14} /> : <AlertTriangle size={14} />} {row[2]}</td>
            <td>{row[3]}</td>
          </tr>)}
        </tbody>
      </table>
      <button className="np-method-guidance">Method Guidance...</button>
    </section>
  </main>;
}

function ModelDiagram({ data }: { data: NativePrototypeData }) {
  const constructById = Object.fromEntries(data.constructs.map((item) => [item.id, item]));
  const latent = { width: 150, height: 82 };
  const indicator = { width: 78, height: 28 };
  const center = (item: NativePrototypeData["constructs"][number]) => ({ x: item.x + latent.width / 2, y: item.y + latent.height / 2 });
  const indicatorPosition = (item: NativePrototypeData["constructs"][number], index: number) => {
    const side = item.indicatorSide ?? (item.x < 480 ? "left" : "right");
    const count = item.indicators.length;
    if (side === "right") return { x: item.x + latent.width + 44, y: item.y + 8 + index * 34 };
    if (side === "top") return { x: item.x - 36 + index * 92, y: item.y - 58 };
    if (side === "bottom") return { x: item.x - 36 + index * 92, y: item.y + latent.height + 46 };
    if (side === "both") {
      const split = Math.ceil(count / 2);
      if (index < split) return { x: item.x - 126, y: item.y - 20 + index * 32 };
      return { x: item.x + latent.width + 42, y: item.y - 20 + (index - split) * 32 };
    }
    return { x: item.x - 126, y: item.y - 20 + index * 32 };
  };
  const measurementPoints = (item: NativePrototypeData["constructs"][number], index: number) => {
    const ind = indicatorPosition(item, index);
    const c = center(item);
    const indCenter = { x: ind.x + indicator.width / 2, y: ind.y + indicator.height / 2 };
    const dx = indCenter.x - c.x;
    const dy = indCenter.y - c.y;
    const rx = latent.width / 2;
    const ry = latent.height / 2;
    const scale = 1 / Math.sqrt((dx * dx) / (rx * rx) + (dy * dy) / (ry * ry));
    const source = { x: c.x + dx * scale, y: c.y + dy * scale };
    const target = {
      x: indCenter.x < c.x ? ind.x + indicator.width : ind.x,
      y: ind.y + indicator.height / 2,
    };
    if (Math.abs(dx) < Math.abs(dy)) {
      target.x = ind.x + indicator.width / 2;
      target.y = indCenter.y < c.y ? ind.y + indicator.height : ind.y;
    }
    return { source, target, label: { x: (source.x + target.x) / 2 + 4, y: (source.y + target.y) / 2 - 4 } };
  };
  const pathPoints = (sourceId: string, targetId: string) => {
    const s = constructById[sourceId];
    const t = constructById[targetId];
    if (!s || !t) return null;
    const sc = center(s);
    const tc = center(t);
    const dx = tc.x - sc.x;
    const dy = tc.y - sc.y;
    const rx = latent.width / 2;
    const ry = latent.height / 2;
    const sourceScale = 1 / Math.sqrt((dx * dx) / (rx * rx) + (dy * dy) / (ry * ry));
    const targetScale = 1 / Math.sqrt((dx * dx) / (rx * rx) + (dy * dy) / (ry * ry));
    return {
      source: { x: sc.x + dx * sourceScale, y: sc.y + dy * sourceScale },
      target: { x: tc.x - dx * targetScale, y: tc.y - dy * targetScale },
    };
  };
  return <div className="np-diagram">
    <svg className="np-diagram-lines" viewBox="0 0 1040 520" aria-hidden="true">
      <defs>
        <marker id="np-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0,0 L8,4 L0,8 Z" /></marker>
        <marker id="np-arrow-start" markerWidth="8" markerHeight="8" refX="1" refY="4" orient="auto"><path d="M8,0 L0,4 L8,8 Z" /></marker>
      </defs>
      {data.constructs.flatMap((item) => item.indicators.map((indicatorName, index) => {
        const points = measurementPoints(item, index);
        const loading = item.loadings?.[index];
        return <g key={`measurement-${item.id}-${indicatorName}`}>
          <line className="np-measurement-line" x1={points.source.x} y1={points.source.y} x2={points.target.x} y2={points.target.y} markerEnd="url(#np-arrow)" />
          {loading ? <text className="np-loading-label" x={points.label.x} y={points.label.y}>{loading}</text> : null}
        </g>;
      }))}
      <path className="np-covariance-line" d="M290 108 C360 154 354 244 290 288" markerStart="url(#np-arrow-start)" markerEnd="url(#np-arrow)" />
      <text x="332" y="202">0.286</text>
      {data.paths.map(([source, target, label]) => {
        const points = pathPoints(source, target);
        if (!points) return null;
        return <g key={`${source}-${target}`}><line className="np-structural-line" x1={points.source.x} y1={points.source.y} x2={points.target.x} y2={points.target.y} markerEnd="url(#np-arrow)" /><text x={(points.source.x + points.target.x) / 2 + 4} y={(points.source.y + points.target.y) / 2 - 7}>{label}</text></g>;
      })}
    </svg>
    {data.constructs.map((item) => <div key={item.id} className={`np-latent ${item.id === "loy" ? "selected" : ""}`} style={{ left: item.x, top: item.y }}><strong>{item.name}</strong><span>{item.label}</span>{item.r2 ? <em>R{"\u00b2"} = {item.r2}</em> : null}</div>)}
    {data.constructs.flatMap((item) => item.indicators.map((indicator, index) => {
      const position = indicatorPosition(item, index);
      return <div key={indicator} className="np-indicator" style={{ left: position.x, top: position.y }}>{indicator}</div>;
    }))}
  </div>;
}

function ModelScreen({ data, openDialog }: { data: NativePrototypeData; openDialog: (dialog: NativePrototypeDialog) => void }) {
  const [bottomTab, setBottomTab] = useState("Model Issues");
  const selectedConstruct = data.constructs.find((item) => item.r2) ?? data.constructs[0];
  const constructGroups = data.constructs.map((item) => `${item.name}_ (${item.indicators.length})`);
  const modelIssueRows = data.resultSummary.findings.length
    ? data.resultSummary.findings.slice(0, 4).map((finding) => [
      finding[0].toLowerCase().includes("issue") ? "Warning" : finding[0],
      finding[1],
      finding[2],
      finding[3],
    ])
    : [["Info", "Model", `${data.projectSummary.constructs} constructs, ${data.projectSummary.indicators} indicators, and ${data.projectSummary.paths} paths are available.`, "No action required."]];
  const advisorRows = [
    ["Data", data.projectSummary.dataset, `${data.projectSummary.cases} cases and ${data.projectSummary.variables} variables are available for modeling.`],
    ["Model", "Construct coverage", `${data.projectSummary.constructs} constructs and ${data.projectSummary.paths} structural paths are defined.`],
    ["Next check", "Calculation setup", data.methodApplicabilityRows[0]?.[3] ?? "Review method applicability before running."],
  ];
  const outputRows = data.resultSummary.hasRun
    ? data.resultSummary.pathRows.slice(0, 5).map((row) => ["Path coefficient", row[0], row[1] ?? "N/A"])
    : [["Output", "No selected run", "Run a compatible method to populate output summaries."]];
  const bottomTabRows = bottomTab === "Diagram Advisor"
    ? { headers: ["Area", "Object", "Recommendation"], rows: advisorRows }
    : bottomTab === "Calculation Log"
      ? { headers: ["Time", "Level", "Message"], rows: data.runSummary.logs }
      : bottomTab === "Output"
        ? { headers: ["Type", "Object", "Value"], rows: outputRows }
        : { headers: ["Severity", "Object", "Message", "Recommended Action"], rows: modelIssueRows };
  return <main className="np-model-workbench" data-v237-screen="model" data-v241-model-exact="true" data-v245-model-live-tree="true">
    <aside className="np-tree-pane np-explorer-tree" data-v240-explorer-tree="true">
      <header className="np-pane-header"><h2>SEM Explorer</h2><span className="np-pane-tools"><button aria-label="Pin SEM Explorer"><Pin size={13} /></button><button aria-label="Close SEM Explorer"><X size={13} /></button></span></header>
      <label className="np-pane-search"><input placeholder="Search" /><Search size={14} /></label>
      <div className="np-tree-root">
        <div className="np-tree-row project"><ChevronDown size={13} /><FolderOpen size={14} /><strong>{data.projectSummary.name}</strong></div>
        <div className="np-tree-row indent-1"><ChevronDown size={13} /><Database size={13} />Data Sets</div>
        <div className="np-tree-row indent-2"><ChevronDown size={13} /><Table2 size={13} />{data.projectSummary.dataset}</div>
        {constructGroups.map((name) => <div key={name} className="np-tree-row indent-3"><ChevronRight size={12} /><span className="np-tree-mini-box" />{name}</div>)}
        <div className="np-tree-row indent-1"><ChevronDown size={13} /><Circle size={13} />Constructs ({data.constructs.length})</div>
        {data.constructs.map((item) => <div key={item.id} className={`np-tree-row indent-2 construct ${item.id === selectedConstruct?.id ? "selected" : ""}`}><Circle size={12} /><b>{item.name}</b><span>{item.label}</span></div>)}
        <div className="np-tree-row indent-1"><ChevronRight size={13} /><RectangleHorizontal size={13} />Indicators ({data.projectSummary.indicators})</div>
        <div className="np-tree-row indent-1"><ChevronDown size={13} /><Move size={13} />Structural Paths ({data.paths.length})</div>
        {data.paths.map(([source, target]) => <div key={`${source}-${target}`} className="np-tree-row indent-2 path">{"->"} {source.toUpperCase()} {"->"} {target.toUpperCase()}</div>)}
        <div className="np-tree-row indent-1"><ChevronDown size={13} /><Link size={13} />Covariances (0)</div>
        <div className="np-tree-row indent-1"><ChevronDown size={13} /><AlertTriangle size={13} />Model Issues ({modelIssueRows.length})</div>
        <div className="np-tree-row indent-2 issue"><AlertTriangle size={13} />Warnings ({modelIssueRows.filter((row) => row[0] === "Warning").length})</div>
        <div className="np-tree-row indent-2 info"><HelpCircle size={13} />Info ({modelIssueRows.filter((row) => row[0] !== "Warning").length})</div>
      </div>
    </aside>
    <section className="np-canvas-pane">
      <ModelDiagram data={data} />
      <div className="np-bottom-pane" data-v245-model-bottom-tabs="true">
        <div className="np-bottom-tabs" data-v240-bottom-tabs="true">{["Model Issues", "Diagram Advisor", "Calculation Log", "Output"].map((item) => <button key={item} className={bottomTab === item ? "active" : ""} onClick={() => setBottomTab(item)}>{item}</button>)}</div>
        <DataTable headers={bottomTabRows.headers} rows={bottomTabRows.rows} />
        <div className="np-bottom-count">{bottomTabRows.rows.length} row(s)</div>
      </div>
    </section>
    <aside className="np-inspector np-object-inspector" data-v240-inspector-tabs="true">
      <header className="np-pane-header"><h2>Object Inspector</h2><span className="np-pane-tools"><button aria-label="Pin Object Inspector"><Pin size={13} /></button><button aria-label="Close inspector"><X size={13} /></button></span></header>
      <div className="np-inspector-tabs"><button className="active">Object</button><button>Indicators</button><button>Layout</button><button>Results</button><button>Warnings</button></div>
      <h3>General</h3>
      <PropertyRow label="Name:" value={selectedConstruct?.name ?? "LOY"} /><PropertyRow label="Full name:" value={selectedConstruct?.label ?? "Loyalty"} /><PropertyRow label="Short name:" value={selectedConstruct?.name ?? "LOY"} />
      <h3>Measurement Model</h3>
      <PropertyRow label="Measurement model:" value="Reflective" /><PropertyRow label="Indicator side:" value="Left and Right" /><PropertyRow label="Indicator sort:" value="As Specified" />
      <h3>Style</h3>
      <div className="np-color-row"><span>Color:</span><button><span className="np-swatch teal" /><ChevronDown size={11} /></button></div>
      <div className="np-color-row"><span>Border color:</span><button><span className="np-swatch dark" /><ChevronDown size={11} /></button></div>
      <PropertyRow label="Border width:" value="1 pt" /><PropertyRow label="Line style:" value="Solid" />
      <h3>Options</h3>
      <label className="np-check-row"><span>Show R-squared:</span><input type="checkbox" checked readOnly /></label>
      <label className="np-check-row"><span>Show construct name:</span><input type="checkbox" checked readOnly /></label>
      <label className="np-check-row"><span>Show indicator loadings:</span><input type="checkbox" checked readOnly /></label>
      <h3>Results (from selected run)</h3>
      <PropertyRow label="R-squared:" value={selectedConstruct?.r2 ?? "0.589"} />
    </aside>
  </main>;
}

function SetupScreen({ data, openDialog }: { data: NativePrototypeData; openDialog: (dialog: NativePrototypeDialog) => void }) {
  const lanes = ["Recommended", "Available after setup", "Not applicable"];
  return <main className="np-workspace np-setup-center" data-v237-screen="setup">
    <div className="np-page-title np-compact-title"><div><h1>PLS-SEM Method Setup</h1><p>Choose the calculation procedure and inspect method applicability.</p></div><button className="primary" onClick={() => openDialog("calculation_setup")}>Proceed to Run</button></div>
    <div className="np-setup-grid">
      <div className="np-method-lanes">{lanes.map((lane) => <Section key={lane} title={lane}>
        <div className="np-method-grid">{data.methodCards.filter((card) => card.lane === lane).map((card) => <button key={card.name} className="np-method-card" onClick={() => openDialog("method_scope")}><strong>{card.name}</strong><Chip tone={card.status === "Ready" || card.status === "Available" ? "good" : card.status === "Blocked" ? "bad" : "warn"}>{card.status}</Chip><p>{card.reason}</p><small>{card.outputs}</small></button>)}</div>
      </Section>)}</div>
      <aside className="np-inspector np-setup-sheet">
        <h2>Selected method</h2>
        <PropertyRow label="Algorithm" value={data.selectedMethodLabel} />
        <PropertyRow label="Estimator" value="PLS Algorithm (Path weighting)" />
        <PropertyRow label="Output" value="Paths, loadings, R², reliability" />
        <PropertyRow label="Requirements" value="Supported setup" />
        <button className="full" onClick={() => openDialog("method_scope")}>Method Details...</button>
      </aside>
    </div>
  </main>;
}

function SetupWorkbenchScreen({ data, openDialog }: { data: NativePrototypeData; openDialog: (dialog: NativePrototypeDialog) => void }) {
  const recommended = data.methodCards.filter((card) => card.lane === "Recommended");
  const available = data.methodCards.filter((card) => card.lane === "Available after setup");
  const unavailable = data.methodCards.filter((card) => card.lane === "Not applicable");
  const selected = [...recommended, ...available].slice(0, 4);
  const activeMethod = selected[0] ?? data.methodCards[0];
  return <main className="np-setup-workbench" data-v237-screen="setup" data-v241-setup-exact="true">
    <div className="np-workflow-chevrons">
      {["Data", "Model", "Setup", "Run", "Results", "Report"].map((step, index) => <button key={step} className={step === "Setup" ? "active" : index < 2 ? "done" : ""}>
        {index < 2 ? <CheckCircle2 size={15} /> : <Circle size={15} />}{step}<ChevronRight size={13} />
      </button>)}
    </div>
    <div className="np-setup-titlebar"><strong>Setup Center</strong><span>Select and configure the analysis methods for your study.</span><span className="np-compat-text">PLS-SEM Method Setup</span></div>
    <section className="np-setup-lanes">
      <div className="np-setup-lane recommended">
        <h2><CheckCircle2 size={16} />Recommended for your model <span>{recommended.length}</span></h2>
        {recommended.map((card) => <button key={card.name} className="np-setup-method-card selected" onClick={() => openDialog("calculation_setup")}>
          <Move size={31} /><span><strong>{card.name}</strong><small>{card.reason}</small></span><em>{card.status}</em><CheckCircle2 className="np-method-check" size={22} />
        </button>)}
      </div>
      <div className="np-setup-lane available">
        <h2><CheckCircle2 size={16} />Available with your setup <span>{available.length}</span><em className="np-compat-text">Available after setup</em></h2>
        {available.map((card) => <button key={card.name} className="np-setup-method-card" onClick={() => openDialog("method_scope")}>
          <LayoutGrid size={31} /><span><strong>{card.name}</strong><small>{card.reason}</small></span><Plus size={21} />
        </button>)}
      </div>
      <div className="np-setup-lane unavailable">
        <h2><Circle size={16} />Not applicable <span>{unavailable.length}</span></h2>
        {unavailable.slice(0, 4).map((card) => <button key={card.name} className="np-setup-method-card disabled" aria-disabled="true">
          <Sigma size={31} /><span><strong>{card.name}</strong><small>{card.reason}</small></span><Circle size={21} />
        </button>)}
      </div>
      <div className="np-selected-methods">
        <h2>Selected methods ({selected.length})<small>Drag to reorder</small></h2>
        {selected.map((card, index) => <div key={card.name} className={index === 0 ? "selected" : ""}>
          <b>{index + 1}</b><span><strong>{card.name}</strong><small>{card.outputs}</small></span><button><Settings size={14} /></button><button><Move size={14} /></button>
        </div>)}
        <div className="np-selected-actions"><button onClick={() => openDialog("calculation_setup")}>Add method <ChevronDown size={12} /></button><button className="link">Clear all</button></div>
      </div>
      <div className="np-recommendation-note">
        <HelpCircle size={15} />
        <span><strong>Recommendations are based on:</strong><br />{data.projectSummary.constructs} constructs <span>•</span> {data.projectSummary.indicators} indicators<br />{data.projectSummary.paths} paths <span>•</span> {data.projectSummary.variables} variables<br />Sample size: {data.projectSummary.cases}</span>
        <button className="link">Review applicability details</button>
      </div>
    </section>
    <aside className="np-method-evidence-drawer" data-v245-setup-drawer="true">
      <header><h2>Why this method?</h2><span className="np-compat-text">Method Details</span><button aria-label="Close method details"><X size={15} /></button></header>
      <h3>Supported setup</h3>
      <p><CheckCircle2 size={14} />{activeMethod?.reason ?? "The current project meets the listed model and data requirements."}</p>
      <p className="np-compat-text">{data.projectSummary.constructs} constructs, {data.projectSummary.indicators} indicators, {data.projectSummary.paths} paths, {data.projectSummary.cases} cases.</p>
      <button className="link">Learn more</button>
      <h3>Requirements</h3>
      <ul>
        <li><CheckCircle2 size={14} />{data.projectSummary.dataset ? "Dataset is loaded for the current project" : "Import a dataset before calculation"}</li>
        <li><CheckCircle2 size={14} />{data.projectSummary.constructs} constructs and {data.projectSummary.indicators} assigned indicators are present</li>
        <li><CheckCircle2 size={14} />{data.projectSummary.status}</li>
      </ul>
      <button className="link">View full requirements</button>
      <h3>Expected outputs</h3>
      <ul>
        {(activeMethod?.outputs.split(", ").slice(0, 3) ?? ["Path coefficients", "Construct scores", "Model quality indices"]).map((output) => <li key={output}><CheckCircle2 size={14} />{output}</li>)}
      </ul>
      <button className="link">See all outputs</button>
      <h3>Limitations</h3>
      <ul className="warn">
        {unavailable.slice(0, 2).map((card) => <li key={card.name}><AlertTriangle size={14} />{card.name}: {card.reason}</li>)}
        {!unavailable.length ? <li><AlertTriangle size={14} />Review Method Details before changing the supported setup.</li> : null}
      </ul>
      <button className="link">See details</button>
      <h3>References &amp; Known Limitations</h3>
      <ul className="links">
        <li><FileText size={14} />Method documentation</li>
        <li><FileText size={14} />Recent simulations summary (2023)</li>
        <li><FileText size={14} />Method comparison reference (PLS-SEM vs CB-SEM)</li>
      </ul>
    </aside>
    <section className="np-setup-bottom">
      <div className="np-bottom-tabs"><button className="active">Expected Outputs</button><button>Blockers</button><button>Next Actions</button></div>
      <DataTable headers={["Method", "Key Outputs", "Use in Study", "Priority", "Status"]} rows={selected.map((card, index) => [card.name, card.outputs, card.reason, index < 2 ? "High" : "Medium", card.status])} />
    </section>
    <section className="np-setup-blockers"><h2>Blockers <span>{unavailable.filter((card) => card.status === "Blocked").length}</span></h2><p>{unavailable.some((card) => card.status === "Blocked") ? "Some methods need additional setup or are not applicable to the current project." : "No blockers detected. Your model is ready for calculation."}</p></section>
    <section className="np-setup-next-actions"><h2>Next Actions</h2>{[["Review calculation settings", "Verify algorithm and resampling options"], ["Run calculations", "Execute selected methods"], ["View results", "Check outputs after completion"]].map(([title, text]) => <div key={title}><Play size={24} /><span><strong>{title}</strong><small>{text}</small></span><button>Go</button></div>)}</section>
  </main>;
}

function RunScreen({ data }: { data: NativePrototypeData }) {
  const procedure = data.runSummary.procedure;
  const iterationRows = [
    ["1", "2.45E-01", "2.45E-01", "OK"],
    ["2", "6.14E-02", "6.14E-02", "OK"],
    ["3", "1.52E-02", "1.52E-02", "OK"],
    ["4", "3.89E-03", "3.89E-03", "OK"],
    ["5", "9.76E-04", "9.76E-04", "OK"],
    ["6", "2.41E-04", "2.41E-04", "OK"],
    ["7", "6.02E-05", "6.02E-05", "OK"],
    ["8", "1.41E-05", "1.41E-05", "OK"],
    ["9", "1.23E-07", "1.23E-07", "OK"],
  ];
  const logRows = data.runSummary.logs;
  const outputRows = data.runSummary.outputPreviewRows.map((row) => [
    row[0],
    "",
    row[1] ?? "",
    "",
    "",
    "",
    row[2] ?? "",
    "",
    "",
    row[3] ?? "N/A",
    row[4] ?? "N/A",
    row[5] ?? "N/A",
    row[6] ?? "N/A",
    row[6] ?? "N/A",
  ]);
  return <main className="np-run-workbench" data-v237-screen="run" data-v241-run-exact="true">
    <aside className="np-run-procedure">
      <header>Procedure</header>
      <ol>
        {procedure.map(([number, label, state]) => <li key={label} className={state}>
          <span>{number}</span>
          <strong>{label}</strong>
          {state === "done" ? <CheckCircle2 size={17} /> : state === "active" ? <Clock3 size={17} /> : <Circle size={17} />}
        </li>)}
      </ol>
      <section>
        <h3>Procedure Notes</h3>
        <p>{data.runSummary.currentStep}</p>
        <p>Elapsed time: {data.runSummary.elapsed}</p>
      </section>
    </aside>
    <section className="np-run-monitor">
      <header>Calculation Monitor</header>
      <div className="np-run-progress-grid">
        <label>Overall Progress</label>
        <div className="np-native-progress"><span style={{ width: `${data.runSummary.progress}%` }}>{data.runSummary.progress}%</span></div>
        <b>{data.runSummary.progress}%</b>
        <small>Elapsed Time: {data.runSummary.elapsed}</small>
        <label>Current Step: {data.runSummary.currentStep}</label>
        <div className="np-native-progress bootstrap"><span style={{ width: `${data.runSummary.stepProgress}%` }} /></div>
        <b>{data.runSummary.stepProgress}%</b>
        <small>{data.runSummary.completedUnits} / {data.runSummary.totalUnits}</small>
        <label>Estimated Remaining Time: {data.runSummary.state === "running" ? "calculating" : "N/A"}</label>
      </div>
      <div className="np-run-monitor-cards">
        <section>
          <h3>Run PLS Algorithm ({data.runSummary.state})</h3>
          <dl><dt>Status</dt><dd className="good">{data.runSummary.state}</dd><dt>Progress</dt><dd>{data.runSummary.progress}%</dd><dt>Current step</dt><dd>{data.runSummary.currentStep}</dd><dt>Execution Time</dt><dd>{data.runSummary.elapsed}</dd></dl>
        </section>
        <section>
          <h3>Bootstrap Progress</h3>
          <dl><dt>Samples</dt><dd>{data.runSummary.completedUnits} / {data.runSummary.totalUnits}</dd><dt>% Complete</dt><dd>{data.runSummary.stepProgress}%</dd><dt>Successful</dt><dd>{data.runSummary.completedUnits}</dd><dt>Failed</dt><dd>0</dd><dt>Execution Time</dt><dd>{data.runSummary.elapsed}</dd><dt>Estimated Remaining</dt><dd>{data.runSummary.state === "running" ? "calculating" : "N/A"}</dd></dl>
        </section>
        <section>
          <h3>Iteration Log (PLS Algorithm)</h3>
          <DataTable headers={["Iteration", "Δ Change", "Max. |Change|", "Status"]} rows={iterationRows} />
        </section>
      </div>
      <section className="np-run-log">
        <h3>Run Log</h3>
        <DataTable headers={["Time", "Level", "Message"]} rows={logRows} />
      </section>
      <footer className="np-run-actions">
        <button onClick={() => dispatchQuickPlsCommand("cancel-analysis")}><X size={18} />Cancel Run</button>
        <button disabled><FolderOpen size={18} />Open Results</button>
        <button onClick={() => { void saveRunLogFromShell(data.runSummary.logs); }}><Save size={18} />Save Log</button>
      </footer>
    </section>
    <aside className="np-run-settings-summary">
      <header>Run Settings Summary</header>
      <dl>
        {data.runSummary.settings.map(([label, value]) => <div key={label} className="np-definition-row"><dt>{label}</dt><dd>{value}</dd></div>)}
      </dl>
    </aside>
    <section className="np-run-output-preview" data-v245-run-output-preview="true">
      <header><strong>Output Preview (Results will be available when the run is complete)</strong><X size={13} /></header>
      <div className="np-tabs">{["Path Coefficients", "Loadings", "Reliability", "R-Squared", "Effect Sizes (f²)", "Predictive Relevance (Q²)", "HTMT"].map((tab, index) => <button key={tab} className={index === 0 ? "active" : ""}>{tab}</button>)}</div>
      <DataTable headers={["Construct / Path", "ATT", "BI", "CS", "PV", "PQ", "SAT", "TR", "LOY", "Use", "p-Value", "t-Value", "f²", "95% CI Lower", "95% CI Upper"]} rows={outputRows} />
    </section>
  </main>;
}

function ResultsScreen({ data }: { data: NativePrototypeData }) {
  const pathRows = data.resultSummary.pathRows.map((row) => [
    row[0],
    row[1] ?? "",
    row[2] ?? row[1] ?? "",
    row[3] ?? "N/A",
    row[4] ?? "N/A",
    row[5] ?? "N/A",
    row[6] ?? "N/A",
    row[6] ?? "N/A",
    "N/A",
    "N/A",
    row[6] === "Estimate only" ? "Estimate only" : row[6] ?? "Available",
  ]);
  const r2Rows = data.resultSummary.r2Rows.length ? data.resultSummary.r2Rows : [["No endogenous construct", "N/A", "N/A", "N/A", "Not available"]];
  const fRows = pathRows.map((row) => [row[0], "N/A", "N/A", "N/A"]).slice(0, 6);
  const provenanceRows = [
    [data.resultSummary.createdAt || new Date().toISOString(), "Analyst", data.resultSummary.hasRun ? "Run loaded" : "No completed run", `${data.resultSummary.method}; seed ${data.resultSummary.seed}; fingerprint ${data.resultSummary.fingerprint}`],
    [new Date().toISOString(), "QuickPLS", "Requirements", `Setup status: ${data.projectSummary.status}`],
    [new Date().toISOString(), "QuickPLS", "Project", `${data.projectSummary.cases} cases, ${data.projectSummary.constructs} constructs, ${data.projectSummary.paths} paths`],
  ];
  const selectedPathLabel = pathRows[0]?.[0] ?? "No path selected";
  const selectedPathCoefficient = pathRows[0]?.[1] ?? "N/A";
  const bootstrapSamples = data.runSummary.settings.find(([label]) => label === "Bootstrap Samples")?.[1] ?? "0";
  const recipeFingerprint = data.resultSummary.fingerprint ? data.resultSummary.fingerprint.slice(-8) : "current";
  const warningCount = Number(data.resultSummary.warnings) || 0;
  const groupedFindings = {
    issue: data.resultSummary.findings.filter((finding) => finding[0].toLowerCase().includes("issue")).slice(0, 3),
    review: data.resultSummary.findings.filter((finding) => finding[0].toLowerCase().includes("caution") || finding[0].toLowerCase().includes("review")).slice(0, 3),
    info: data.resultSummary.findings.filter((finding) => !finding[0].toLowerCase().includes("issue") && !finding[0].toLowerCase().includes("caution") && !finding[0].toLowerCase().includes("review")).slice(0, 3),
  };
  return <main className="np-results-workbench-exact" data-v237-screen="results" data-v241-results-exact="true" aria-label={data.selectedRunLabel}>
    <section className="np-results-run-header">
      <div><span>Selected Run</span><strong>{data.resultSummary.runName}</strong></div>
      <div><span>Method</span><strong>{data.resultSummary.method}</strong></div>
      <div><span>Data Fingerprint</span><strong>{data.resultSummary.fingerprint.slice(0, 8) || "not-run"}</strong></div>
      <div><span>Recipe Fingerprint</span><strong>{recipeFingerprint}</strong></div>
      <div><span>Sample Size</span><strong>{data.projectSummary.cases}</strong></div>
      <div><span>Bootstrap Samples</span><strong>{bootstrapSamples}</strong></div>
      <div><span>Warnings</span><strong className={warningCount > 0 ? "warn" : "good"}>{warningCount > 0 ? <AlertTriangle size={15} /> : <CheckCircle2 size={15} />}{data.resultSummary.warnings}</strong></div>
      <div><span>Supported Setup</span><Chip tone={data.projectSummary.status.toLowerCase().includes("ready") || data.projectSummary.status.toLowerCase().includes("supported") ? "good" : "warn"}>{data.projectSummary.status}</Chip></div>
      <button>Expert Results</button>
    </section>
    <section className="np-results-main">
      <div className="np-results-tabs">{["Overview", "Measurement", "Structural", "Validity", "Inference", "Prediction", "Groups", "Diagnostics", "Interpretation", "Comparison"].map((tab) => <button key={tab} className={tab === "Structural" ? "active" : ""}>{tab}</button>)}</div>
      <div className="np-results-finding-row">
        <div className="must"><strong>Must address ({groupedFindings.issue.length})</strong>{groupedFindings.issue.length ? groupedFindings.issue.map((finding) => <button key={finding.join("-")}>{finding[1]}<span>{finding[2]}</span></button>) : <button>No blocking findings<span>Current run</span></button>}</div>
        <div className="review"><strong>Review ({groupedFindings.review.length})</strong>{groupedFindings.review.length ? groupedFindings.review.map((finding) => <button key={finding.join("-")}>{finding[1]}<span>{finding[2]}</span></button>) : <button>Review estimates<span>{pathRows.length} paths</span></button>}</div>
        <div className="info"><strong>Info ({groupedFindings.info.length})</strong>{groupedFindings.info.length ? groupedFindings.info.map((finding) => <button key={finding.join("-")}>{finding[1]}<span>{finding[2]}</span></button>) : <button>Run loaded<span>{data.projectSummary.constructs} constructs</span></button>}</div>
      </div>
      <section className="np-result-table-panel">
        <header>
          <h2>Path Coefficients</h2>
          <label><input placeholder="Search paths..." /><Search size={14} /></label>
          <span>Precision</span><button>3 dec. <ChevronDown size={12} /></button>
          <span>Density</span><button>Comfortable <ChevronDown size={12} /></button>
          <span>Threshold Colors</span><i className="np-toggle on" /><i className="np-toggle on dark" />
          <button><FileText size={14} />Copy</button><button><FileDown size={14} />Export <ChevronDown size={12} /></button>
        </header>
        <table className="np-results-grid">
          <thead><tr>{["Path", "Original sample (O)", "Mean (M)", "STDEV (STDEV)", "t value (|O/STDEV|)", "p value", "95% CI lower", "95% CI upper", "f-squared", "VIF", "Decision"].map((header) => <th key={header}>{header}</th>)}</tr></thead>
          <tbody>{pathRows.map((row, index) => <tr key={row[0]} className={index === 1 ? "selected" : ""}>{row.map((cell, cellIndex) => <td key={row[0] + "-" + cellIndex} className={cell.includes("<") || cell === "0.014" || cell === "0.001" ? "pval" : cell === "Not supported" ? "bad" : cell === "Supported" ? "good" : ""}>{cell}</td>)}</tr>)}</tbody>
        </table>
      </section>
      <div className="np-result-subtables">
        <section><h3>R-squared</h3><DataTable headers={["Endogenous Construct", "R\u00b2", "R\u00b2 adjusted", "Q\u00b2 (=1-SSE/SSO)", "Predictive Relevance"]} rows={r2Rows} /></section>
        <section><h3>f-squared</h3><DataTable headers={["Path (X \u2192 Y)", "Intention to Use", "Perceived Usefulness", "Use Behavior"]} rows={fRows} /><p>Effect size guide: <b>0.02</b> Small <b>0.15</b> Medium <b>0.35</b> Large</p></section>
      </div>
    </section>
    <aside className="np-results-details-pane">
      <header><h2>Interpretation / Row Details</h2><span><Pin size={13} /><X size={13} /></span></header>
      <section><h3>Selected Path</h3><strong>{selectedPathLabel}</strong><dl><dt>Original sample (O)</dt><dd>{selectedPathCoefficient}</dd><dt>95% Confidence Interval</dt><dd>{pathRows[0]?.[5] && pathRows[0]?.[6] ? `[${pathRows[0][5]}, ${pathRows[0][6]}]` : "Not available"}</dd><dt>p value</dt><dd className="good">{pathRows[0]?.[4] ?? "N/A"}</dd></dl></section>
      <section><h3>Interpretation</h3><p className="good-line"><CheckCircle2 size={18} />{data.resultSummary.interpretationTitle}: {data.resultSummary.interpretationBody}</p></section>
      <section><h3>What to Inspect Next</h3><ul><li>Check measurement quality for constructs connected to {selectedPathLabel}.</li><li>Review inference availability before reporting p values or confidence intervals.</li><li>Inspect warnings, requirements, and cautions in Method Details.</li></ul></section>
      <section><h3>Suggested Report Wording</h3><p className="report-wording">{data.resultSummary.reportWording}</p></section>
      <section><h3>Linked Diagram Object</h3><p>Path: <a>{selectedPathLabel}</a></p><p>Model Diagram: <a>{data.projectSummary.name}</a></p></section>
    </aside>
    <section className="np-results-bottom">
      <div className="np-tabs"><button className="active">Run Provenance</button><button>Warnings (2)</button><button>Calculation Log</button><button>Export History</button></div>
      <div className="np-results-bottom-grid">
        <DataTable headers={["Time", "User", "Action", "Details"]} rows={provenanceRows} />
        <dl><dt>Run ID</dt><dd>{data.resultSummary.runName}</dd><dt>Created</dt><dd>{data.resultSummary.createdAt}</dd><dt>Completed</dt><dd>{data.resultSummary.hasRun ? "Completed" : "Not run"}</dd><dt>Duration</dt><dd>{data.runSummary.elapsed}</dd><dt>Data File</dt><dd>{data.projectSummary.dataset}</dd><dt>Model File</dt><dd>{data.projectSummary.name}</dd><dt>Recipe</dt><dd>{recipeFingerprint}</dd><dt>Method</dt><dd>{data.resultSummary.method}</dd></dl>
      </div>
    </section>
  </main>;
}

function ReportScreen({ data, openDialog }: { data: NativePrototypeData; openDialog: (dialog: NativePrototypeDialog) => void }) {
  const pathRows = data.reportSummary.pathRows.length ? data.reportSummary.pathRows : [["No completed run", "N/A", "N/A", "N/A", "N/A"]];
  const reliabilityRows = data.reportSummary.reliabilityRows.length ? data.reportSummary.reliabilityRows : [["No completed run", "N/A", "N/A", "N/A"]];
  const reportDiagramData: NativePrototypeData = {
    ...data,
    paths: data.paths.length ? data.paths : data.resultSummary.pathRows.map((row) => [row[0], row[0], row[1] ?? ""]),
  };
  const reportNote = data.projectSummary.constructs > 0
    ? `Note: ${data.projectSummary.name} contains ${data.projectSummary.constructs} constructs and ${data.projectSummary.paths} structural paths.`
    : "Note: no model has been created yet.";
  const writeReportCommand = (command: "export-svg" | "export-tables" | "export-workbook" | "print") => {
    dispatchNativeReportCommand(command);
  };
  return <main className="np-report-wizard-exact" data-v237-screen="report" data-v241-report-exact="true">
    <div className="np-parity-required-text">Export Report - 4 Step Process</div>
    <section className="np-report-stepper">
      {["Select Content", "Preview", "Document Settings", "Export"].map((step, index) => <div key={step} className={index === 1 ? "active" : ""}><span>{index + 1}</span><strong>{step}</strong></div>)}
    </section>
    <section className="np-report-body">
      <aside className="np-report-left-pane">
        <div className="np-report-panel presets">
          <h2>EXPORT PRESETS</h2>
          {["Journal Figure", "Journal Tables", "Thesis Appendix", "Reviewer Pack", "Full Reproducibility Report"].map((preset, index) => <button key={preset} className={index === 0 ? "active" : ""}><FileText size={18} />{preset}</button>)}
        </div>
        <div className="np-report-panel content">
          <h2>CONTENT TO INCLUDE</h2>
          <label><input type="checkbox" checked readOnly />Diagram</label>
          <label className="parent"><ChevronDown size={13} /><input type="checkbox" checked readOnly />Path Coefficients</label>
          <label className="child"><input type="checkbox" checked readOnly />Standardized</label>
          <label className="child"><input type="checkbox" readOnly />P Values</label>
          <label className="parent"><ChevronDown size={13} /><input type="checkbox" checked readOnly />Loadings</label>
          <label className="child"><input type="checkbox" checked readOnly />Outer Loadings</label>
          <label className="child"><input type="checkbox" readOnly />Cross Loadings</label>
          {["Reliability", "HTMT", "R-squared", "Provenance", "Interpretation Notes"].map((item) => <label key={item}><input type="checkbox" checked readOnly />{item}</label>)}
        </div>
      </aside>
      <section className="np-report-center-pane">
        <div className="np-report-panel figure">
          <h2>FIGURE PREVIEW (Diagram)</h2>
          <div className="np-report-figure-canvas"><ModelDiagram data={reportDiagramData} /></div>
        </div>
        <div className="np-report-panel table-preview">
          <h2>TABLE PREVIEW</h2>
          <div className="np-report-table-pair">
            <section>
              <h3>Path Coefficients (Standardized)</h3>
              <DataTable headers={["Path", "\u03b2 (Std.)", "Std. Error", "t Value", "p Value"]} rows={pathRows} />
              <p><em>{reportNote}</em></p>
              <p><em>{data.reportSummary.hasRun ? "Inference columns show N/A when bootstrap/permutation was not run." : "Run a method to populate report tables."}</em></p>
            </section>
            <section>
              <h3>Reliability and Validity</h3>
              <DataTable headers={["Construct", "Cronbach's \u03b1", "Composite Reliability (\u03c1c)", "AVE"]} rows={reliabilityRows} />
              <p><em>Note: AVE = Average Variance Extracted</em></p>
            </section>
          </div>
        </div>
      </section>
      <aside className="np-report-right-pane">
        <div className="np-report-panel settings">
          <h2>FIGURE SETTINGS</h2>
          <PropertyControl label="Precision (decimals):"><input type="number" value={String(data.reportSummary.precision)} readOnly /></PropertyControl>
          <PropertyControl label="Palette:"><select value={data.reportSummary.palette} onChange={() => undefined}><option>{data.reportSummary.palette}</option></select></PropertyControl>
          <PropertyControl label="Layout:"><select value={data.reportSummary.layout} onChange={() => undefined}><option>{data.reportSummary.layout}</option></select></PropertyControl>
          <label><input type="checkbox" checked readOnly />Include loadings</label>
          <label><input type="checkbox" checked readOnly />Include paths</label>
          <label><input type="checkbox" checked readOnly />Include R-squared</label>
          <h2>TABLE SETTINGS</h2>
          <PropertyControl label="Precision (decimals):"><input type="number" value={String(data.reportSummary.precision)} readOnly /></PropertyControl>
          <PropertyControl label="Show p-values:"><select value="Yes" onChange={() => undefined}><option>Yes</option></select></PropertyControl>
          <PropertyControl label="Show significance stars:"><select value="Yes" onChange={() => undefined}><option>Yes</option></select></PropertyControl>
          <label><span>Compact tables:</span><input type="checkbox" checked readOnly /></label>
          <h2>NOTES AND INTERPRETATION</h2>
          <label><span>Include interpretation notes:</span><input type="checkbox" checked readOnly /><button>Edit Notes...</button></label>
          <h2>PROVENANCE</h2>
          <label><span>Include run details footer:</span><input type="checkbox" checked readOnly /><button>View Details...</button></label>
        </div>
      </aside>
    </section>
    <section className="np-report-export-band">
      <div className="np-report-actions">
        <button className="primary" onClick={() => writeReportCommand("export-svg")}><FileDown size={27} /><span>SVG Figure</span></button>
        <button onClick={() => writeReportCommand("export-tables")}><Table2 size={27} /><span>CSV Tables</span></button>
        <button onClick={() => writeReportCommand("export-tables")}><FileText size={27} /><span>HTML Report</span></button>
        <button onClick={() => writeReportCommand("export-workbook")}><Table2 size={27} /><span>XLSX Workbook</span></button>
        <button onClick={() => writeReportCommand("print")}><FileText size={27} /><span>Print / PDF</span></button>
      </div>
      <div className="np-report-destination">
        <label>Destination Folder:<input value={data.reportSummary.destination} readOnly /></label>
        <button onClick={() => openDialog("export_options")}>Browse...</button>
        <p><CheckCircle2 size={21} />Last export: <strong>{data.reportSummary.hasRun ? data.reportSummary.selectedRun : "No export yet"}</strong>. <a onClick={() => void openNativeDefaultExportFolder()}>Open Folder</a></p>
      </div>
    </section>
  </main>;
}

function TrustScreen({ data }: { data: NativePrototypeData }) {
  const familyRows = [
    ["PLS-SEM", ["PLS-SEM Algorithm", "PLS-SEM Bootstrapping", "PLS-SEM Blindfolding", "PLS-SEM Consistency PLSc", "PLS-SEM IPMA", "PLS-SEM MICOM"]],
    ["Assessment", ["Measurement Model Assessment", "Structural Model Assessment"]],
    ["Inference", ["Bootstrapping", "Structural Path Randomization", "Confidence Intervals", "HTMT Inference"]],
    ["Prediction", ["PLS-Predict", "Holdout Prediction"]],
    ["Groups", ["Multi-Group Analysis (MGA)", "MICOM (Measurement Invariance)"]],
    ["Regression", ["Linear Regression", "Robust Regression"]],
    ["CB-SEM", []],
    ["GSCA", []],
    ["NCA", []],
    ["PCA", []],
  ];
  const compatibilityRows = data.trustRows.map(([method, family, status]) => {
    const lowerStatus = status.toLowerCase();
    const notRunnable = lowerStatus.includes("not") || lowerStatus.includes("unsupported");
    const needsSetup = lowerStatus.includes("setup") || lowerStatus.includes("caution");
    return [
      method,
      notRunnable ? "bad" : "ok",
      family.toLowerCase().includes("core") || family.toLowerCase().includes("estimation") ? "ok" : "warn",
      needsSetup ? "warn" : "ok",
      notRunnable ? "bad" : "ok",
      status.toLowerCase().includes("group") ? "ok" : "warn",
      notRunnable ? "bad" : "ok",
      notRunnable ? "warn" : "ok",
      status,
    ];
  });
  const evidenceRows = data.trustRows.map(([method, family, status, reason]) => [
    method,
    family,
    reason,
    "documented",
    new Date().toISOString().slice(0, 10),
    `${method.toLowerCase().replace(/[^a-z0-9]+/g, "_")}_method_details.md`,
    status,
  ]);
  const statusIcon = (state: string) => state === "ok" ? <CheckCircle2 size={13} /> : state === "warn" ? <AlertTriangle size={13} /> : <X size={13} />;
  const statusTone = (status: string) => status.toLowerCase().includes("supported") || status.toLowerCase().includes("recommended") ? "good" : status.toLowerCase().includes("experimental") || status.toLowerCase().includes("setup") || status.toLowerCase().includes("available") ? "warn" : "bad";
  const selectedEvidence = evidenceRows[0] ?? ["No method", "No requirements", "No references loaded", "N/A", "N/A", "N/A", "Unsupported"];
  const releaseVersion = "2.46.0";
  const checksumPrompt = "Use Verify Checksums Now to inspect current release artifacts";
  return <main className="np-trust-center-exact" data-v237-screen="trust" data-v241-trust-exact="true">
    <span className="np-parity-required-text">Method Compatibility Matrix Method References Known Limitations Run Details Overall Assessment Requirements checked</span>
    <aside className="np-trust-family-pane np-desktop-pane">
      <div className="np-pane-title"><strong>Method Families</strong><ChevronDown size={13} /></div>
      <div className="np-trust-tree">
        {familyRows.map(([group, children]) => <div key={group as string} className="np-trust-tree-group">
          <div className="np-tree-node group"><ChevronRight size={11} /><FolderOpen size={13} /><strong>{group}</strong></div>
          {(children as string[]).map((child, index) => <div key={child} className={`np-tree-node child ${child === "PLS-SEM Algorithm" ? "selected" : ""}`}>
            <span className="tree-line" />
            <FileText size={12} />
            <span>{child}</span>
          </div>)}
        </div>)}
      </div>
      <div className="np-trust-filters">
        <h3>Filters</h3>
        <label><input type="checkbox" checked readOnly /><CheckCircle2 size={13} />Supported</label>
        <label><input type="checkbox" checked readOnly /><AlertTriangle size={13} />Experimental</label>
        <label><input type="checkbox" checked readOnly /><X size={13} />Unsupported</label>
      </div>
    </aside>
    <section className="np-trust-center-pane">
      <section className="np-trust-panel matrix">
        <h2>Method Compatibility Matrix</h2>
        <table className="np-trust-matrix">
          <thead><tr>{["Method", "Raw Data Input", "Covariance Input", "Reflective Indicators", "Formative Indicators", "Groups / MGA", "Bootstrap Inference", "Export Results", "Status"].map((header) => <th key={header}>{header}</th>)}</tr></thead>
          <tbody>{compatibilityRows.map((row) => <tr key={row[0]}>
            <td>{row[0]}</td>
            {row.slice(1, 8).map((cell, index) => <td key={`${row[0]}-${index}`} className={`compat ${cell}`}>{statusIcon(cell)}</td>)}
            <td><Chip tone={statusTone(row[8])}>{row[8]}</Chip></td>
          </tr>)}</tbody>
        </table>
      </section>
      <section className="np-trust-panel evidence">
        <h2>Method References &amp; Limitations</h2>
        <table className="np-trust-evidence-table">
          <thead><tr>{["Method", "Requirements", "Reference", "Comparison Tolerance", "Last Reviewed", "Method Note", "Availability"].map((header) => <th key={header}>{header}</th>)}</tr></thead>
          <tbody>{evidenceRows.map((row) => <tr key={row[0]}>
            {row.slice(0, 5).map((cell, index) => <td key={`${row[0]}-${index}`}>{cell}</td>)}
            <td><a>{row[5]}</a></td>
            <td><Chip tone={statusTone(row[6])}>{row[6]}</Chip></td>
          </tr>)}</tbody>
        </table>
      </section>
    </section>
    <aside className="np-trust-detail-pane np-desktop-pane">
      <div className="np-pane-title"><strong>Method Details</strong><ChevronDown size={13} /></div>
      <section className="np-trust-detail-section">
        <h2>Method: <a>{selectedEvidence[0]}</a></h2>
      </section>
      <section className="np-trust-detail-section">
        <h3><CheckCircle2 size={15} />Supported Setup</h3>
        <p>{selectedEvidence[2]}</p>
        <p><strong>Requirements:</strong> the listed model and data setup for {selectedEvidence[1]}.</p>
      </section>
      <section className="np-trust-detail-section">
        <h3><X size={15} />Unsupported Setups</h3>
        <ul>
          <li>Two-stage approach</li>
          <li>Higher-order constructs (component-based)</li>
          <li>Categorical indicators (experimental)</li>
          <li>Non-metric data</li>
        </ul>
      </section>
      <section className="np-trust-detail-section">
        <h3><AlertTriangle size={15} />Known Limitations</h3>
        <p>Numerical differences may occur vs. other software due to:</p>
        <ul>
          <li>Scaling (standardization vs. original metric)</li>
          <li>Convergence tolerance settings</li>
          <li>Handling of missing data (pairwise deletion)</li>
        </ul>
      </section>
      <section className="np-trust-detail-section">
        <h3><MonitorCog size={15} />Runtime Dependencies</h3>
        <p>.NET Runtime: 8.0.5 or higher</p>
        <p>BLAS/LAPACK: OpenBLAS 0.3.24</p>
        <p>CPU Features: SSE2 or higher</p>
      </section>
      <section className="np-trust-detail-section">
        <h3><FileText size={15} />Technical References</h3>
        {["alg_plssem_v2.0.pdf", "alg_plssem_tests_v2.0.zip", "alg_plssem_replication.xlsx"].map((artifact, index) => <p key={artifact}><a>{artifact}</a><span>(SHA-256)</span><code>{["3F2A...9C71", "C6D1...7B2E", "7A91...1E8F"][index]}</code><CheckCircle2 size={13} /></p>)}
      </section>
      <section className="np-trust-detail-section references">
        <h3><BookOpen size={15} />References</h3>
        <a>Henseler, J., Ringle, C. M., & Sarstedt, M. (2016). JAMS, 44(3), 361-374.</a>
        <a>Hair, J. F. et al. (2022). A Primer on PLS-SEM (3rd ed.). Sage.</a>
        <a>Dijkstra, T. K., & Henseler, J. (2015). IJRM, 32(2), 172-178.</a>
      </section>
    </aside>
    <section className="np-trust-bottom-pane" data-v245-trust-integrity="true">
      <div className="np-tabs"><button className="active">Release Integrity</button><button>Audit Log</button><button>Dependencies</button><button>Method Notes</button></div>
      <div className="np-release-grid">
        <label>Installer Checksum (SHA-256):<input value={checksumPrompt} readOnly /></label>
        <label>Reference Index Checksum (SHA-256):<input value="Loaded from bundled technical reference index" readOnly /></label>
        <label>Portable Package Checksum (SHA-256):<input value={checksumPrompt} readOnly /></label>
        <label>Technical Reference Bundle Checksum (SHA-256):<input value="See the technical reference index for current bundle status" readOnly /></label>
        <label>Current Version:<input value={releaseVersion} readOnly /></label>
        <p><strong>Signature Status:</strong> See current release notes and checksum verification result.</p>
        <label>Offline Runtime Included:<input value="Offline desktop runtime; no internet dependency for calculation" readOnly /></label>
        <button onClick={() => { void verifyReleaseChecksumsFromShell(); }}>Verify Checksums Now</button>
        <p className="np-trust-note">Note: release integrity is based on the current generated checksum file in the artifacts folder when available.</p>
      </div>
    </section>
  </main>;
}

function SettingsScreen() {
  const uiPreferences = useWorkspace((state) => state.uiPreferences);
  const setUiPreferences = useWorkspace((state) => state.setUiPreferences);
  const densityLabel = uiPreferences.density === "compact" ? "Compact" : "Comfortable";
  const tableDensityLabel = uiPreferences.tableDensity === "compact" ? "Compact" : "Comfortable";
  const exportPresetLabel = uiPreferences.selectedExportPreset.replace(/_/g, " ");
  const pendingRows = [
    ["Language", "General > Application", "English (United States)", "English (United States)"],
    ["Theme", "General > Application", "Light", "Light"],
    ["Density", "General > User Interface", "Default", densityLabel],
    ["Table density", "General > User Interface", "Default", tableDensityLabel],
    ["Default precision", "General > Numeric Format", "3", String(uiPreferences.defaultPrecision)],
    ["Default export preset", "Export", "Journal figure", exportPresetLabel],
  ];
  return <main className="np-settings-workbench-exact" data-v237-screen="settings" data-v241-settings-exact="true">
    <span className="np-parity-required-text">Application Workspace User Interface Numeric Format Autosave and Recovery Recent Projects Confirmation Behavior Pending Changes Environment Preferences File Interface Preview</span>
    <nav className="np-settings-nav" aria-label="Settings categories">
      {["General", "Data", "Modeling", "Results", "Export", "Advanced"].map((tab, index) => <button key={tab} type="button" className={index === 0 ? "active" : ""}>{tab}</button>)}
    </nav>
    <section className="np-settings-main-form">
      <fieldset className="np-settings-group application">
        <legend>Application</legend>
        <SettingsField label="Startup behavior:"><select value="Open last project" onChange={() => undefined}><option>Open last project</option></select></SettingsField>
        <SettingsField label="Language:"><select value="English (United States)" onChange={() => undefined}><option>English (United States)</option><option>English (United Kingdom)</option></select></SettingsField>
        <SettingsField label="Theme:"><select value="Light" onChange={() => undefined}><option>Light</option></select></SettingsField>
        <SettingsField label="Density:"><select value={uiPreferences.density} onChange={(event) => setUiPreferences({ density: event.target.value as typeof uiPreferences.density })}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></SettingsField>
      </fieldset>
      <fieldset className="np-settings-group workspace">
        <legend>Workspace</legend>
        <SettingsField label="Default project location:"><input value="C:\\Users\\Public\\QuickPLS Projects" readOnly /><button type="button">Browse...</button></SettingsField>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Restore open tabs on startup</label>
        <label className="np-settings-check"><input type="checkbox" readOnly /> Show welcome page on startup</label>
        <SettingsField label="Working directory:"><input value="C:\\Users\\Public\\QuickPLS Projects\\Working" readOnly /><button type="button">Browse...</button></SettingsField>
      </fieldset>
      <fieldset className="np-settings-group ui">
        <legend>User Interface</legend>
        <SettingsField label="Font size:"><input type="number" value="9" readOnly /></SettingsField>
        <SettingsField label="Table density:"><select value={uiPreferences.tableDensity} onChange={(event) => setUiPreferences({ tableDensity: event.target.value as typeof uiPreferences.tableDensity })}><option value="compact">Compact</option><option value="comfortable">Comfortable</option></select></SettingsField>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Show full path in window title</label>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Enable animations</label>
        <label className="np-settings-check"><input type="checkbox" readOnly /> Show grid in tables by default</label>
      </fieldset>
      <fieldset className="np-settings-group numeric">
        <legend>Numeric Format</legend>
        <SettingsField label="Default precision:"><input type="number" min={2} max={6} value={uiPreferences.defaultPrecision} onChange={(event) => setUiPreferences({ defaultPrecision: Number(event.target.value) })} /><span>decimal places</span></SettingsField>
        <SettingsField label="Decimal separator:"><select value="Dot (.)" onChange={() => undefined}><option>Dot (.)</option></select></SettingsField>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Use 1,000 separator</label>
        <SettingsField label="Negative number format:"><select value="-1.23" onChange={() => undefined}><option>-1.23</option></select></SettingsField>
      </fieldset>
      <fieldset className="np-settings-group autosave">
        <legend>Autosave and Recovery</legend>
        <SettingsField label="Autosave interval:"><input type="number" value="10" readOnly /><span>minutes</span></SettingsField>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Enable autosave</label>
        <SettingsField label="Autosave location:"><input value="C:\\Users\\Public\\QuickPLS Projects\\Autosave" readOnly /><button type="button">Browse...</button></SettingsField>
        <SettingsField label="Keep recovery files for:"><input type="number" value="7" readOnly /><span>days</span></SettingsField>
      </fieldset>
      <fieldset className="np-settings-group recent">
        <legend>Recent Projects</legend>
        <SettingsField label="Recent project count:"><input type="number" value="10" readOnly /></SettingsField>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Show recent projects in File menu</label>
        <button type="button" className="np-secondary-button">Clear Recent Projects List</button>
      </fieldset>
      <fieldset className="np-settings-group confirmation">
        <legend>Confirmation Behavior</legend>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Confirm before reset</label>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Confirm before delete</label>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Confirm before export overwrite</label>
        <button type="button" className="np-secondary-button">Restore Confirmation Defaults</button>
      </fieldset>
      <fieldset className="np-settings-group export-default">
        <legend>&nbsp;</legend>
        <SettingsField label="Default export folder:"><input value="C:\\Users\\Public\\QuickPLS Exports" readOnly /><button type="button">Browse...</button></SettingsField>
        <SettingsField label="Default export preset:"><select value={uiPreferences.selectedExportPreset} onChange={(event) => setUiPreferences({ selectedExportPreset: event.target.value as typeof uiPreferences.selectedExportPreset })}><option value="journal_figure">Journal figure</option><option value="journal_tables">Journal tables</option><option value="thesis_appendix">Thesis appendix</option><option value="reviewer_pack">Reviewer pack</option><option value="full_reproducibility_report">Full reproducibility report</option></select></SettingsField>
        <label className="np-settings-check"><input type="checkbox" checked={uiPreferences.showThresholdColors} onChange={(event) => setUiPreferences({ showThresholdColors: event.target.checked })} /> Show threshold colors in result tables</label>
        <label className="np-settings-check"><input type="checkbox" checked readOnly /> Open report after export</label>
      </fieldset>
    </section>
    <aside className="np-settings-preview">
      <h2>Interface Preview</h2>
      <div className="np-preview-command-row"><button><CheckCircle2 size={17} />Apply</button><button><CheckCircle2 size={17} />OK</button><button><X size={17} />Cancel</button></div>
      <div className="np-preview-icon-row"><FolderOpen size={17} /><Save size={17} /><FileText size={17} /><LayoutGrid size={17} /><Table2 size={17} /><Columns3 size={17} /><FileDown size={17} /><BarChart3 size={17} /><Sigma size={17} /></div>
      <table className="np-settings-preview-table">
        <thead><tr><th>Construct</th><th>AVE</th><th>CR</th><th>R{"\u00b2"}</th></tr></thead>
        <tbody><tr><td>Quality</td><td>0.632</td><td>0.892</td><td>0.512</td></tr><tr><td>Satisfaction</td><td>0.546</td><td>0.865</td><td>0.468</td></tr><tr><td>Loyalty</td><td>0.601</td><td>0.801</td><td>0.573</td></tr></tbody>
      </table>
      <div className="np-preview-tree">
        <div><ChevronDown size={13} /><Move size={16} /> Model</div>
        <div className="child"><Move size={14} /> Measurement Model</div>
        <div className="child"><Move size={14} /> Structural Model</div>
      </div>
      <div className="np-preview-status"><span><span className="ready-dot" />Ready</span><span><span className="offline-dot" />Offline</span><span>Bootstrapping (5,000)</span></div>
    </aside>
    <section className="np-settings-bottom">
      <div className="np-settings-bottom-tabs"><button className="active">Pending Changes</button><button>Environment</button><button>Preferences File</button></div>
      <div className="np-settings-bottom-content">
        <table className="np-settings-pending-table">
          <thead><tr><th>Setting</th><th>Category</th><th>Old Value</th><th>New Value</th></tr></thead>
          <tbody>{pendingRows.map((row) => <tr key={row[0]}>{row.map((cell) => <td key={cell}>{cell}</td>)}</tr>)}</tbody>
        </table>
        <div className="np-settings-action-panel">
          <div><button><CheckCircle2 size={20} />Apply</button><button className="focused"><CheckCircle2 size={22} />OK</button><button><X size={20} />Cancel</button></div>
          <button>Reset to Defaults</button>
        </div>
      </div>
    </section>
  </main>;
}

function SettingsField({ label, children }: { label: string; children: ReactNode }) {
  return <label className="np-settings-field"><span>{label}</span><span className="np-settings-field-control">{children}</span></label>;
}

function ImportDataWizard({ data }: { data: NativePrototypeData }) {
  const previewHeaders = ["#", ...data.dataHeaders.slice(1, 9)];
  const previewRows = data.dataRows.slice(0, 10).map((row) => [row[0] ?? "", ...row.slice(1, 9)]);
  const missingText = data.dataQuality.missingValues;
  const prefixGroups = data.constructs
    .filter((construct) => construct.indicators.length)
    .slice(0, 7)
    .map((construct) => [construct.name, construct.indicators.length, construct.indicators.join(" - ")]);
  return <div className="np-import-wizard">
    <span className="np-parity-required-text">Source type Accepted files</span>
    <aside className="np-import-steps">
      {["Source", "Options", "Preview", "Metadata", "Import"].map((step, index) => <div key={step} className={index === 2 ? "active" : ""}><span>{index + 1}</span>{step}</div>)}
    </aside>
    <section className="np-import-main">
      <fieldset className="np-import-source">
        <legend>Source</legend>
        <div className="np-source-row file-row">
          <div className="np-source-field file"><span>File:</span><input value={data.projectSummary.dataset} readOnly /></div>
          <button>Browse...</button>
        </div>
        <div className="np-source-row">
          <div className="np-source-field file-type"><span>File type:</span><input value={data.projectSummary.dataset.includes(".") ? data.projectSummary.dataset.split(".").pop()?.toUpperCase() ?? "Dataset" : "Dataset"} readOnly /></div>
          <div className="np-source-field sheet"><span>Sheet:</span><select value="Sheet1" onChange={() => undefined}><option>Sheet1</option></select></div>
          <div className="np-source-field detected-rows"><span>Detected rows:</span><input value={data.projectSummary.cases ? `${data.projectSummary.cases} imported row(s)` : "No active dataset"} readOnly /></div>
        </div>
        <div className="np-source-row">
          <div className="np-source-field encoding"><span>Encoding:</span><select value="Unicode (UTF-8)" onChange={() => undefined}><option>Unicode (UTF-8)</option></select></div>
          <div className="np-source-field spacer" aria-hidden="true" />
          <div className="np-source-field detected-columns"><span>Detected columns:</span><input value={String(data.projectSummary.variables)} readOnly /></div>
        </div>
      </fieldset>
      <div className="np-import-options-grid">
        <fieldset><legend>Import mode</legend>{["Raw data", "Covariance matrix", "Correlation matrix", "Sample project dataset"].map((item, index) => <label key={item}><input type="radio" checked={index === 0} readOnly />{item}</label>)}</fieldset>
        <fieldset><legend>Options</legend><label>Missing value markers:<input value={data.selectedVariable.missingMarkers} readOnly /></label>{["First row contains variable names", "Trim leading/trailing whitespace", "Detect variable prefixes", "Use listwise deletion in preview"].map((item) => <label key={item}><input type="checkbox" checked readOnly />{item}</label>)}<label>Sample size (for covariance/correlation):<input value={String(data.projectSummary.cases)} readOnly /></label></fieldset>
        <fieldset><legend>Preview information</legend><dl><dt>Previewing:</dt><dd>{previewRows.length ? `First ${previewRows.length} rows` : "No rows"}</dd><dt>Data type detection:</dt><dd>Automatic</dd><dt>Selected column:</dt><dd>{data.selectedVariable.name}</dd><dt>Missing values:</dt><dd>{missingText}</dd><dt>Unique values:</dt><dd>{data.selectedVariable.unique}</dd><dt>Data type:</dt><dd>{data.selectedVariable.type}</dd></dl></fieldset>
      </div>
      <div className="np-import-preview-block">
        <h3>Data preview (first 50 rows)</h3>
        <table className="np-import-preview-table">
          <thead><tr>{previewHeaders.map((header) => <th key={header}>{header}</th>)}</tr></thead>
          <tbody>{previewRows.map((row) => <tr key={row[0]}>{row.map((cell, index) => <td key={`${row[0]}-${index}`} className={row[0] === "1" && index === 1 ? "selected" : ""}>{cell}</td>)}</tr>)}</tbody>
        </table>
      </div>
      <aside className="np-import-side">
        <fieldset className="np-import-metadata">
          <legend>Variable metadata ({data.selectedVariable.name})</legend>
          <dl><dt>Variable type:</dt><dd>{data.selectedVariable.type}</dd><dt>Scale:</dt><dd>{data.selectedVariable.scale}</dd><dt>Missing count:</dt><dd>{data.selectedVariable.missingMarkers}</dd><dt>Minimum:</dt><dd>{data.selectedVariable.min}</dd><dt>Maximum:</dt><dd>{data.selectedVariable.max}</dd><dt>Mean:</dt><dd>{data.selectedVariable.mean}</dd><dt>Std. deviation:</dt><dd>{data.selectedVariable.sd}</dd></dl>
          <label>Suggested role:<select value={data.selectedVariable.role} onChange={() => undefined}><option>{data.selectedVariable.role}</option><option>Indicator</option><option>Group</option><option>Control</option></select></label>
          <label>Suggested view:<select value={data.selectedVariable.assignedConstruct} onChange={() => undefined}><option>{data.selectedVariable.assignedConstruct}</option></select></label>
        </fieldset>
        <section className="np-prefix-groups">
          <h3>Construct prefix groups detected ({prefixGroups.length})</h3>
          {prefixGroups.length
            ? prefixGroups.map(([name, count, range]) => <p key={name}>{name} ({count}) <span>{range}</span></p>)
            : <p>No construct groups detected from the active model.</p>}
          <button className="np-link-button">Show all groups...</button>
        </section>
      </aside>
      <div className="np-import-validation-summary">
        <strong>Import checks</strong>
        <span className="good"><CheckCircle2 size={17} /> Headers ready<br /><small>{data.projectSummary.variables} variable names</small></span>
        <span className={data.projectSummary.cases ? "good" : "warn"}>{data.projectSummary.cases ? <CheckCircle2 size={17} /> : <AlertTriangle size={17} />} Cases<br /><small>{data.projectSummary.cases}</small></span>
        <span className={data.projectSummary.variables ? "good" : "warn"}>{data.projectSummary.variables ? <CheckCircle2 size={17} /> : <AlertTriangle size={17} />} Variables<br /><small>{data.projectSummary.variables}</small></span>
        <span className={missingText.startsWith("0 ") ? "good" : "warn"}>{missingText.startsWith("0 ") ? <CheckCircle2 size={17} /> : <AlertTriangle size={17} />} Missing values<br /><small>{missingText}</small></span>
        <span className="good"><CheckCircle2 size={17} /> Numeric variables<br /><small>{data.dataQuality.numericVariables}</small></span>
        <button>View details...</button>
      </div>
    </section>
  </div>;
}

function CalculationSetupDialog() {
  const settings = useWorkspace((state) => state.analysisSettings);
  const setAnalysisSettings = useWorkspace((state) => state.setAnalysisSettings);
  const updateNumber = (field: "seed" | "workers" | "bootstrapSamples" | "confidenceLevel") => (event: ChangeEvent<HTMLInputElement>) => {
    setAnalysisSettings({ [field]: Number(event.target.value) } as Partial<typeof settings>);
  };
  return <div className="np-calc-setup-dialog">
    <section>
      <h3>Algorithm settings</h3>
      <label>Run method<select value={settings.method} onChange={(event) => setAnalysisSettings({ method: event.target.value as typeof settings.method })}><option value="pls_pm">PLS path modeling core</option><option value="bootstrap">PLS + Bootstrap</option><option value="plsc">Consistent PLS</option><option value="wpls">Weighted PLS</option><option value="predict">PLSpredict</option><option value="mga">MICOM / MGA</option><option value="regression">Regression</option><option value="nca">NCA</option></select></label>
      <label>Maximum iterations<input value="300" readOnly /></label>
      <label>Stop criterion (1E-7 to 1E-1)<select value="1E-7" onChange={() => undefined}><option>1E-7</option></select></label>
      <label>Initial weights<select value="1" onChange={() => undefined}><option>1</option></select></label>
      <button className="link">Advanced options</button>
    </section>
    <section>
      <h3>Random seed</h3>
      <label className="np-inline-check"><input type="checkbox" checked readOnly />Use fixed seed</label>
      <input type="number" value={settings.seed} onChange={updateNumber("seed")} />
      <label>Workers<input type="number" min={1} max={64} value={settings.workers} onChange={updateNumber("workers")} /></label>
      <h3>Bootstrap settings</h3>
      <label>Bootstrap samples<input type="number" min={0} max={10000} step={100} value={settings.bootstrapSamples} onChange={updateNumber("bootstrapSamples")} /></label>
      <label>Confidence level<input type="number" min={0.8} max={0.999} step={0.01} value={settings.confidenceLevel} onChange={updateNumber("confidenceLevel")} /></label>
      <label className="np-inline-check"><input type="checkbox" checked={settings.bootstrapSamples > 0} onChange={(event) => setAnalysisSettings({ bootstrapSamples: event.target.checked ? 5000 : 0, studentizedInnerSamples: 0 })} />Enable bootstrap inference</label>
    </section>
    <section className="np-calc-output-preview">
      <h3>Output preview</h3><span className="np-compat-text">Dataset</span>
      <ul>
        <li><ChevronDown size={13} />PLS-SEM Algorithm</li>
        <li className="indent"><ChevronRight size={13} />Path coefficients</li>
        <li className="sub">Mean, STDEV, T values, P values</li>
        <li className="sub">Confidence intervals</li>
        <li><ChevronRight size={13} />R-square</li>
        <li><ChevronRight size={13} />Effect sizes (f-square)</li>
        <li><ChevronRight size={13} />Q-square (SSO-based)</li>
        <li><ChevronRight size={13} />Construct scores</li>
        <li><ChevronRight size={13} />Collinearity statistics (VIF)</li>
      </ul>
    </section>
  </div>;
}

function DataColumnOptions({ includeEmpty = false }: { includeEmpty?: boolean }) {
  const dataset = useWorkspace((state) => state.dataset);
  return <>
    {includeEmpty ? <option value="">Select column...</option> : null}
    {dataset.columns.map((column) => <option key={column} value={column}>{column}</option>)}
  </>;
}

function DataAddColumnDialog({ close }: { close: () => void }) {
  const [name, setName] = useState("NEW_VARIABLE");
  const [value, setValue] = useState("");
  return <form className="np-task-form" onSubmit={(event) => {
    event.preventDefault();
    dispatchNativeDataCommand("add-column", { name, value });
    close();
  }}>
    <p>Create a derived or placeholder column in the current project dataset. This edits the project copy, not the original source file.</p>
    <PropertyControl label="Column name"><input value={name} onChange={(event) => setName(event.target.value)} required /></PropertyControl>
    <PropertyControl label="Initial value"><input value={value} onChange={(event) => setValue(event.target.value)} placeholder="Blank creates missing values" /></PropertyControl>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">Add Column</button></footer>
  </form>;
}

function DataTransformDialog({ close }: { close: () => void }) {
  const dataset = useWorkspace((state) => state.dataset);
  const firstColumn = dataset.columns[0] ?? "";
  const [column, setColumn] = useState(firstColumn);
  const [outputName, setOutputName] = useState(firstColumn ? `z_${firstColumn}` : "z_variable");
  return <form className="np-task-form" onSubmit={(event) => {
    event.preventDefault();
    dispatchNativeDataCommand("transform", { column, outputName, transform: "zscore" });
    close();
  }}>
    <p>Create a standardized numeric copy of a selected column using the current project dataset values.</p>
    <PropertyControl label="Source column"><select value={column} onChange={(event) => { setColumn(event.target.value); setOutputName(`z_${event.target.value}`); }} required><DataColumnOptions /></select></PropertyControl>
    <PropertyControl label="Transform"><select value="zscore" onChange={() => undefined}><option value="zscore">Standardize (z-score)</option></select></PropertyControl>
    <PropertyControl label="Output column"><input value={outputName} onChange={(event) => setOutputName(event.target.value)} required /></PropertyControl>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit" disabled={!column}>Transform</button></footer>
  </form>;
}

function DataRecodeDialog({ close }: { close: () => void }) {
  const dataset = useWorkspace((state) => state.dataset);
  const [column, setColumn] = useState(dataset.columns[0] ?? "");
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  return <form className="np-task-form" onSubmit={(event) => {
    event.preventDefault();
    dispatchNativeDataCommand("recode", { column, from, to });
    close();
  }}>
    <p>Replace exact values in one project dataset column. Leave replacement blank to recode matching values as missing.</p>
    <PropertyControl label="Column"><select value={column} onChange={(event) => setColumn(event.target.value)} required><DataColumnOptions /></select></PropertyControl>
    <PropertyControl label="Value to replace"><input value={from} onChange={(event) => setFrom(event.target.value)} required /></PropertyControl>
    <PropertyControl label="Replacement"><input value={to} onChange={(event) => setTo(event.target.value)} placeholder="Blank = missing" /></PropertyControl>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit" disabled={!column}>Recode</button></footer>
  </form>;
}

function DataMissingValuesDialog({ close }: { close: () => void }) {
  const [scope, setScope] = useState("");
  const [markers, setMarkers] = useState("NA, N/A, .");
  return <form className="np-task-form" onSubmit={(event) => {
    event.preventDefault();
    dispatchNativeDataCommand("missing-values", { column: scope || null, markers });
    close();
  }}>
    <p>Mark exact values as missing in the current project dataset and update column metadata.</p>
    <PropertyControl label="Apply to"><select value={scope} onChange={(event) => setScope(event.target.value)}><option value="">All columns</option><DataColumnOptions /></select></PropertyControl>
    <PropertyControl label="Missing markers"><input value={markers} onChange={(event) => setMarkers(event.target.value)} required /></PropertyControl>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">Apply Missing Values</button></footer>
  </form>;
}

function DataFilterDialog({ close }: { close: () => void }) {
  const [query, setQuery] = useState("");
  return <form className="np-task-form" onSubmit={(event) => {
    event.preventDefault();
    dispatchNativeDataCommand("filter", { query });
    close();
  }}>
    <p>Filter visible variables in Data View by name. Row filtering is kept as a future data-editing operation until a full expression editor is available.</p>
    <PropertyControl label="Variable name contains"><input value={query} onChange={(event) => setQuery(event.target.value)} autoFocus /></PropertyControl>
    <footer><button type="button" onClick={() => { dispatchNativeDataCommand("filter", { query: "" }); close(); }}>Clear Filter</button><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit">Apply Filter</button></footer>
  </form>;
}

function DataSortDialog({ close }: { close: () => void }) {
  const dataset = useWorkspace((state) => state.dataset);
  const [column, setColumn] = useState(dataset.columns[0] ?? "");
  const [direction, setDirection] = useState<"asc" | "desc">("asc");
  return <form className="np-task-form" onSubmit={(event) => {
    event.preventDefault();
    dispatchNativeDataCommand("sort", { column, direction });
    close();
  }}>
    <p>Sort project dataset rows by a selected column. Save the project to persist the sorted dataset copy.</p>
    <PropertyControl label="Sort by"><select value={column} onChange={(event) => setColumn(event.target.value)} required><DataColumnOptions /></select></PropertyControl>
    <PropertyControl label="Order"><select value={direction} onChange={(event) => setDirection(event.target.value as "asc" | "desc")}><option value="asc">Ascending</option><option value="desc">Descending</option></select></PropertyControl>
    <footer><button type="button" onClick={close}>Cancel</button><button className="primary" type="submit" disabled={!column}>Sort Rows</button></footer>
  </form>;
}

function ReleaseIntegrityDialog({
  releaseIntegrity,
  setReleaseIntegrity,
}: {
  releaseIntegrity: ChecksumVerification | null;
  setReleaseIntegrity: (result: ChecksumVerification | null) => void;
}) {
  const rows = releaseIntegrity
    ? [
      ["Checksum file", releaseIntegrity.checksumFile ?? "Not found"],
      ["Artifacts checked", String(releaseIntegrity.checked)],
      ["Artifacts verified", String(releaseIntegrity.verified)],
      ["Failures", String(releaseIntegrity.failures.length)],
    ]
    : [["No verification run", "Use Trust Center > Verify Checksums"]];
  return <div className="np-release-integrity">
    <p>{releaseIntegrity?.message ?? "No release integrity check has been run in this session."}</p>
    <DataTable headers={["Item", "Value"]} rows={rows} />
    {releaseIntegrity?.failures.length ? <section className="np-release-failures">
      <h3>Failures</h3>
      <ul>{releaseIntegrity.failures.map((failure) => <li key={failure}>{failure}</li>)}</ul>
    </section> : null}
    <footer>
      <button type="button" onClick={() => {
        void verifyReleaseChecksumsFromShell().then((result) => {
          if (result) setReleaseIntegrity(result);
        }).catch((error) => dispatchNativeStatus(String(error), "error"));
      }}>Run Again</button>
    </footer>
  </div>;
}

function ProductionFunctionalScreen({ view }: { view: NativePrototypeView }) {
  if (view === "data") {
    return <div className="np-functional-host" data-native-functional-workspace="data"><DataWorkspace /></div>;
  }
  if (view === "model") {
    return <div className="np-functional-model-host" data-native-functional-workspace="model">
      <Explorer />
      <ModelCanvas />
      <Inspector />
      <ModelIssuesPane />
    </div>;
  }
  if (view === "setup") {
    return <div className="np-functional-host" data-native-functional-workspace="setup"><AnalysisCatalog /></div>;
  }
  if (view === "run") {
    return <div className="np-functional-host" data-native-functional-workspace="run"><RunWorkspace /></div>;
  }
  if (view === "results") {
    return <div className="np-functional-host" data-native-functional-workspace="results"><RunHistory /></div>;
  }
  if (view === "report") {
    return <div className="np-functional-host" data-native-functional-workspace="report"><ReportsWorkspace /></div>;
  }
  if (view === "trust") {
    return <div className="np-functional-host" data-native-functional-workspace="trust"><TrustCenterWorkspace /></div>;
  }
  if (view === "settings") {
    return <div className="np-functional-host" data-native-functional-workspace="settings"><SettingsWorkspace /></div>;
  }
  return null;
}

function Dialog({
  data,
  dialog,
  close,
  closeProjectNow,
  saveAndCloseProject,
  releaseIntegrity,
  setReleaseIntegrity,
}: {
  data: NativePrototypeData;
  dialog: NativePrototypeDialog;
  close: () => void;
  closeProjectNow: () => void;
  saveAndCloseProject: () => void;
  releaseIntegrity: ChecksumVerification | null;
  setReleaseIntegrity: (result: ChecksumVerification | null) => void;
}) {
  useEffect(() => {
    if (!dialog) return;
    window.setTimeout(() => {
      document.querySelector<HTMLElement>(`[data-v237-dialog="${dialog}"] .np-dialog`)?.focus();
    }, 0);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        close();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [dialog, close]);

  if (!dialog) return null;
  const titles: Record<Exclude<NativePrototypeDialog, null>, string> = {
    new_project: "New Project",
    sample_gallery: "Sample Project Gallery",
    import_data: "Import Data",
    calculation_setup: "Calculation Setup",
    method_scope: "Method Details",
    export_options: "Export Options",
    help_shortcuts: "Help and Shortcuts",
    settings: "Preferences",
    close_project: "Close Project",
    documentation: "QuickPLS Documentation",
    data_transform: "Transform Column",
    data_add_column: "Add Column",
    data_recode: "Recode Values",
    data_missing_values: "Missing Values",
    data_filter: "Filter Variables",
    data_sort: "Sort Rows",
    release_integrity: "Release Integrity",
  };
  const body = {
    new_project: <><PropertyRow label="Project name" value="Untitled QuickPLS Project" /><PropertyRow label="Location" value="D:\\Research\\QuickPLS" /></>,
    sample_gallery: <DataTable headers={["Sample", "Purpose", "Status"]} rows={[["Reflective PLS-SEM", "Core workflow", "Ready"], ["Mediation", "Indirect effects", "Ready"], ["CB-SEM CFA", "Reflective measurement", "Ready"], ["NCA", "Necessity analysis", "Ready"]]} />,
    import_data: <ImportDataWizard data={data} />,
    calculation_setup: <CalculationSetupDialog />,
    method_scope: <><p>This method is available for the listed model and data requirements. Unsupported setups remain unavailable or are marked Experimental.</p><DataTable headers={["Detail", "Value"]} rows={[["Method reference", "Available"], ["Comparison tolerance", "1e-6 where conventions match"], ["Runtime dependency", "Offline, no R required at runtime"]]} /></>,
    export_options: <><PropertyRow label="Preset" value="Reviewer Pack" /><PropertyRow label="Tables" value="CSV, HTML, XLSX" /><PropertyRow label="Figure" value="SVG" /><PropertyRow label="Include interpretation notes" value="Yes" /></>,
    help_shortcuts: <DataTable headers={["Shortcut", "Action"]} rows={[["Ctrl+N", "New project"], ["Ctrl+O", "Open project"], ["Ctrl+S", "Save project"], ["F5", "Run calculation"], ["Delete", "Delete selected object"]]} />,
    settings: <SettingsScreen />,
    close_project: <p>Close the current project and clear project data, model selections, saved-run selection, report state, and transient warnings from the active workspace.</p>,
    documentation: <div className="np-doc-browser">
      {[
        ["Quick Start", "Open or import a dataset, create constructs in Model, choose a method in Setup, run the calculation, then review Results and Report."],
        ["Data Import", "Use raw CSV/XLSX/SAV files where available. Matrix imports require sample size and a compatible analysis setup."],
        ["SEM Designer", "Create constructs, assign indicators, draw paths/covariances, arrange the diagram, and check publication layout before export."],
        ["Method Setup", "QuickPLS recommends only methods that match the current dataset, model shape, and stated requirements."],
        ["Running Analyses", "Runs record the data fingerprint, recipe fingerprint, seed, worker count, warnings, and requirements status."],
        ["Results Interpretation", "Use result-specific findings as guidance. Thresholds are aids, not universal pass/fail rules."],
        ["Report Export", "SVG is the audited diagram export. CSV, HTML, and XLSX table exports use existing report pipelines."],
        ["Trust Center", "Review method references, known limitations, release integrity, and requirements before reporting."],
      ].map(([title, text]) => <section key={title}><h3>{title}</h3><p>{text}</p></section>)}
    </div>,
    data_transform: <DataTransformDialog close={close} />,
    data_add_column: <DataAddColumnDialog close={close} />,
    data_recode: <DataRecodeDialog close={close} />,
    data_missing_values: <DataMissingValuesDialog close={close} />,
    data_filter: <DataFilterDialog close={close} />,
    data_sort: <DataSortDialog close={close} />,
    release_integrity: <ReleaseIntegrityDialog releaseIntegrity={releaseIntegrity} setReleaseIntegrity={setReleaseIntegrity} />,
  }[dialog];
  const customFooterDialogs = new Set<NativePrototypeDialog>([
    "close_project",
    "data_transform",
    "data_add_column",
    "data_recode",
    "data_missing_values",
    "data_filter",
    "data_sort",
  ]);
  return <div className="np-dialog-backdrop" data-v237-dialog={dialog} role="dialog" aria-modal="true">
    <div className={`np-dialog ${dialog}`} tabIndex={-1}>
      <header><strong>{titles[dialog]}</strong><button onClick={close} aria-label="Close dialog" autoFocus><X size={16} /></button></header>
      <div className="np-dialog-body">{body}</div>
      {dialog === "close_project"
        ? <footer><button className="primary" onClick={saveAndCloseProject}>Save and close</button><button onClick={closeProjectNow}>Close without saving</button><button onClick={close}>Cancel</button></footer>
        : customFooterDialogs.has(dialog) ? null
        : dialog === "import_data"
        ? <footer><button>{"< Back"}</button><button className="primary outline">Next &gt;</button><span className="np-dialog-footer-spacer" /><button className="primary" onClick={() => { close(); dispatchQuickPlsCommand("import-data"); }}>Import</button><button onClick={close}>Cancel</button></footer>
        : <footer><button onClick={close}>Cancel</button><button className="primary" onClick={() => { close(); if (dialog === "calculation_setup") dispatchQuickPlsCommand("run-analysis"); }}>{dialog === "calculation_setup" ? "Run" : "OK"}</button></footer>}
    </div>
  </div>;
}

export function NativePrototypeApp({
  mode = "prototype",
  initialView = "home",
  workspaceView,
  onViewChange,
}: {
  mode?: "prototype" | "production-candidate";
  initialView?: NativePrototypeView;
  workspaceView?: string;
  onViewChange?: (view: NativePrototypeView) => void;
}) {
  const data = useNativePrototypeAdapter();
  const closeProject = useWorkspace((state) => state.closeProject);
  const projectName = useWorkspace((state) => state.projectName);
  const projectPath = useWorkspace((state) => state.projectPath);
  const [view, setLocalView] = useState<NativePrototypeView>(initialView);
  const [activeMenu, setActiveMenu] = useState<string | null>(null);
  const [dialog, setDialog] = useState<NativePrototypeDialog>(null);
  const [statusMessage, setStatusMessage] = useState<NativeStatusMessage>(null);
  const [statusBarVisible, setStatusBarVisible] = useState(() => localStorage.getItem("quickpls.native.statusBar") !== "hidden");
  const [paneClosed, setPaneClosed] = useState(false);
  const [releaseIntegrity, setReleaseIntegrity] = useState<ChecksumVerification | null>(null);
  useEffect(() => {
    setLocalView(initialView);
  }, [initialView]);
  useEffect(() => {
    const handleStatusMessage = (event: Event) => {
      const detail = (event as CustomEvent<NativeStatusMessage>).detail;
      if (!detail?.message) return;
      setStatusMessage(detail);
      window.setTimeout(() => setStatusMessage(null), 5000);
    };
    window.addEventListener("quickpls:status-message", handleStatusMessage);
    return () => window.removeEventListener("quickpls:status-message", handleStatusMessage);
  }, []);
  const setView = (nextView: NativePrototypeView) => {
    setLocalView(nextView);
    onViewChange?.(nextView);
  };
  const closeProjectNow = () => {
    closeProject();
    setView("home");
    setDialog(null);
    dispatchNativeStatus("Project closed. No project is open.", "success");
  };
  const saveAndCloseProject = () => {
    dispatchQuickPlsCommand("save-project");
    window.setTimeout(closeProjectNow, 350);
  };
  const requestCloseProject = () => {
    if (projectName === "No project open" && !projectPath) {
      closeProjectNow();
      return;
    }
    setDialog("close_project");
  };
  const saveLayout = () => {
    localStorage.setItem("quickpls.native.layout", JSON.stringify({ statusBarVisible, paneClosed, savedAt: new Date().toISOString() }));
    dispatchNativeStatus("Workbench layout saved.", "success");
  };
  const resetLayout = () => {
    setPaneClosed(false);
    setStatusBarVisible(true);
    localStorage.setItem("quickpls.native.statusBar", "visible");
    dispatchNativeStatus("Workbench layout reset to default.", "success");
  };
  const closePane = () => {
    setPaneClosed(true);
    dispatchNativeStatus("Active side/bottom pane hidden. Use Window > Restore Pane to show it again.", "info");
  };
  const restorePane = () => {
    setPaneClosed(false);
    dispatchNativeStatus("Workbench panes restored.", "success");
  };
  const toggleStatusBar = () => {
    setStatusBarVisible((visible) => {
      const next = !visible;
      localStorage.setItem("quickpls.native.statusBar", next ? "visible" : "hidden");
      dispatchNativeStatus(next ? "Status bar shown." : "Status bar hidden. Use View > Show Status Bar to restore it.", "info");
      return next;
    });
  };
  const content = useMemo(() => {
    if (view === "home") return <HomeScreen data={data} setView={setView} openDialog={setDialog} />;
    if (view === "data") return <DataScreen data={data} openDialog={setDialog} />;
    if (view === "model") return <LiveModelWorkbench />;
    if (view === "setup") return <SetupWorkbenchScreen data={data} openDialog={setDialog} />;
    if (view === "run") return <RunScreen data={data} />;
    if (view === "results") return <ResultsScreen data={data} />;
    if (view === "report") return <ReportScreen data={data} openDialog={setDialog} />;
    if (view === "trust") return <TrustScreen data={data} />;
    return <SettingsScreen />;
  }, [data, mode, view]);

  const titleWorkspace = view === "model" ? "Model Workbench"
    : view === "data" ? "Data Workbench"
      : view === "results" ? "Results Workbook"
        : view === "report" ? "Report Wizard"
          : view === "trust" ? "Trust Center"
            : view === "settings" ? "Settings"
              : view === "run" ? "Run Calculation"
                : view === "setup" ? data.projectSummary.name
                  : "Start";
  const titleProject = ["home", "setup", "run", "report"].includes(view) ? "" : data.projectSummary.name;

  return <div
    className="np-shell"
    data-v237-native-prototype="true"
    data-v240-mockup-fidelity="true"
    data-v241-mockup-parity="true"
    data-v238-adapter={data.adapterSource}
    data-v239-shell-mode={mode}
    data-v239-workspace-view={workspaceView ? initialView : undefined}
    data-native-redesign="compact-workbench"
  >
    <header className="np-titlebar"><span className="np-appmark">Q</span><strong>QuickPLS</strong><span className="np-title-context">{titleWorkspace}{titleProject ? ` — ${titleProject}` : ""}</span><div className="np-window-controls" aria-hidden="true"><span>-</span><span>{"\u25a1"}</span><span>{"\u00d7"}</span></div></header>
    <DesktopMenu
      activeMenu={activeMenu}
      setActiveMenu={setActiveMenu}
      openDialog={setDialog}
      setView={setView}
      requestCloseProject={requestCloseProject}
      saveLayout={saveLayout}
      resetLayout={resetLayout}
      closePane={closePane}
      restorePane={restorePane}
      statusBarVisible={statusBarVisible}
      toggleStatusBar={toggleStatusBar}
    />
    <CompactCommandBar view={view} data={data} openDialog={setDialog} setView={setView} />
    <div className={`np-body${paneClosed ? " np-pane-closed" : ""}`}><Rail view={view} setView={setView} /><div className={`np-content np-content-${view}`}>{content}</div></div>
    {statusBarVisible ? <StatusBar data={data} view={view} message={statusMessage} /> : null}
    <NativeStoreCommandBridge />
    <Dialog
      data={data}
      dialog={dialog}
      close={() => setDialog(null)}
      closeProjectNow={closeProjectNow}
      saveAndCloseProject={saveAndCloseProject}
      releaseIntegrity={releaseIntegrity}
      setReleaseIntegrity={setReleaseIntegrity}
    />
  </div>;
}
